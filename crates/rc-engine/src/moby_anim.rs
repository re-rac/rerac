//! Moby animation at run time: every placed instance plays its class's sequence 0 in a loop, as the
//! engine does for mobys without an update function (docs/plan/moby_animation.md §2).
//!
//! * Load: `load_anim_classes` parses each class's sequences (`rc_formats::moby_anim`). Ratchet
//!   (class 0) has 134 empty slots on the disc; his sequences are the level's `ratchet_seq` table
//!   (256 slots, slot = sequence id), which the port puts into his class slots (inferred: the player
//!   code fills them at run time; slot 0 is taken as his idle).
//! * Tick: `MobyAnimAdvance` (`rc_formats::moby_anim::advance`) once per 60 Hz tick in `FixedUpdate`
//!   (`Time<Fixed>` = 60 Hz; the game advances once per game frame, 60 Hz on NTSC — inferred).
//! * Palette: after a tick, `MobyAnimEval` (`evaluate`) per distinct (class, state) — instances of a
//!   class that spawned together share the pose, so this is one evaluation per class, not per instance
//!   — copied into every instance's slot range of one storage buffer (`mat4x4` per joint, column i =
//!   F row i), which `moby.wgsl` skins with.
//!
//! Occlusion: `MobyProc` only queues an evaluation job for a moby it draws, so an occluded instance
//! (`AnimInstance::visible == false`, set by `moby_render::update_moby_occlusion`) keeps ticking but is not
//! evaluated; it is evaluated again as soon as it becomes visible.
//!
//! Spawn state (crate::moby_spawn): each instance starts from its state after the level's load pass
//! (`rc_formats::moby_spawn::SpawnState::anim_after_load`: advanced once, class init sequence changes),
//! with its snapshot frame in [`MobyAnim::snapshots`] when key A is one (`fun_00212f90`). A change the game
//! makes in the moby's first update after load ([`MobyAnim::pending`], e.g. the big amoeboids' blend to
//! sequence 1) is applied after the advance of the first tick on which the moby is active by the port's
//! reading of `fun_0020d868`: drawn last frame (passed occlusion), update distance 0xff, or within its
//! update distance of the camera (group activation is not modelled). Instances hidden at spawn
//! ([`MobyAnim::hidden`]) are never evaluated.
//!
//! Scheduler-driven instances ([`MobyAnim::drive`]): the classes with a ported update (bolts, crates, grass)
//! take their state from the moby table after every game tick (crate::gameplay) and are not advanced here.
//!
//! `RC_ANIM=0` freezes every instance at the bind pose (identity palette, no evaluation).

use crate::level_load::LoadedLevel;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::level::LevelCore;
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, MobyFrame, MobySequence, Rows, IDENTITY};
use rc_formats::moby_spawn::SpawnState;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Per class (same order as `classes`): its animation data. Classes whose sequences fail to parse get
/// none (bind pose) and a warning.
pub fn load_anim_classes(core: &LevelCore, core_data: &[u8], classes: &[LevelMobyClass]) -> Vec<MobyAnimClass> {
    let block = |name: &str| core.blocks.iter().find(|b| b.name == name).and_then(|b| core_data.get(b.offset..b.offset + b.size));
    classes
        .iter()
        .map(|c| {
            let seqs = if c.o_class == 0 && !core.ratchet_seqs.is_empty() && c.class.sequence_pointers.iter().all(|&p| p == 0) {
                ratchet_sequences(core, &block)
            } else {
                match block(&format!("moby_class/{:04}", c.o_class)).map(|blob| moby_anim::parse_sequences(blob, &c.class)) {
                    Some(Ok(s)) => s,
                    Some(Err(e)) => { warn!("moby class {}: sequences not parsed ({e}); drawn in the bind pose", c.o_class); Vec::new() }
                    None => Vec::new(),
                }
            };
            MobyAnimClass::new(&c.class, seqs)
        })
        .collect()
}

fn ratchet_sequences<'a>(core: &LevelCore, block: &dyn Fn(&str) -> Option<&'a [u8]>) -> Vec<Option<MobySequence>> {
    (0..core.ratchet_seqs.len())
        .map(|i| {
            let blob = block(&format!("ratchet_seq/{i:03}"))?;
            moby_anim::parse_sequence(blob, 0).map_err(|e| warn!("ratchet_seq {i}: {e}")).ok()
        })
        .collect()
}

/// One animated instance: its class (index into `LevelMobys::classes`), its palette slots and state.
pub struct AnimInstance {
    pub class: usize,
    /// First palette matrix and slot count (≥ joint count, ≥ 1, ≥ highest skinned joint + 1).
    pub base: u32,
    pub slots: u32,
    pub state: AnimState,
    /// Passed the occlusion test this frame (crate::moby_render::update_moby_occlusion).
    pub visible: bool,
}

#[derive(Resource)]
pub struct MobyAnim {
    pub enabled: bool,
    pub instances: Vec<AnimInstance>,
    pub palette: Handle<ShaderBuffer>,
    pub palette_len: u32,
    /// An instance became visible since the last evaluation: evaluate even without a new tick.
    pub pose_dirty: bool,
    /// Per instance: the snapshot frame key A reads when `seq_a` is `moby_anim::SNAPSHOT_SEQ`.
    pub snapshots: Vec<Option<MobyFrame>>,
    /// Per instance: a sequence change the first active update after load makes (crate::moby_spawn).
    pub pending: Vec<Option<PendingChange>>,
    /// Per instance: hidden (mode bit 1) since the load pass; never evaluated.
    pub hidden: Vec<bool>,
    /// Per instance: the runtime joint-modifier list of a driven moby (`Moby::joint_mods`, its class manipulators:
    /// `rc_game::moby_update::manip`); applied by the evaluation (`evaluate_posed`), which makes the pose private.
    pub mods: Vec<Vec<rc_formats::moby_anim::JointModifier>>,
    ticks: u64,
    evaluated: Option<u64>,
    eval_time: Duration,
    evals: u64,
    poses: usize,
    last_report: f32,
    /// The palette as last uploaded (`palette_len` matrices; identity outside the evaluated instances).
    bytes: Vec<u8>,
    /// One identity matrix as bytes.
    identity: [u8; MATRIX_BYTES],
    /// Per instance: its slot range holds a pose written by the last evaluation (else identity).
    written: Vec<bool>,
}

/// Bytes of one palette matrix.
const MATRIX_BYTES: usize = 64;

impl MobyAnim {
    pub fn new(enabled: bool, instances: Vec<AnimInstance>, palette: Handle<ShaderBuffer>, palette_len: u32) -> Self {
        let n = instances.len();
        MobyAnim {
            enabled, instances, palette, palette_len, pose_dirty: false,
            snapshots: vec![None; n], pending: (0..n).map(|_| None).collect(), hidden: vec![false; n], mods: vec![Vec::new(); n],
            ticks: 0, evaluated: None, eval_time: Duration::ZERO, evals: 0, poses: 0, last_report: 0.0,
            bytes: Vec::new(), identity: matrix_bytes(&IDENTITY), written: vec![false; n],
        }
    }
}

impl MobyAnim {
    /// Instance `k` belongs to a class the moby scheduler runs (crate::gameplay): its animation state and
    /// snapshot are the table's (`MobyAnimAdvance` ran in the scheduler), so this module never advances it
    /// (`skip_advance`) and drops any spawn-rule change for it.
    pub fn drive(&mut self, k: usize, mut state: AnimState, snapshot: Option<&MobyFrame>, mods: &[rc_formats::moby_anim::JointModifier]) {
        let Some(i) = self.instances.get_mut(k) else { return };
        if self.mods[k] != mods { self.mods[k] = mods.to_vec(); }
        state.skip_advance = true;
        i.state = state;
        self.snapshots[k] = if state.seq_a == moby_anim::SNAPSHOT_SEQ { snapshot.cloned() } else { None };
        self.pending[k] = None;
        self.hidden[k] = false;
    }

    /// Instance `k`'s moby changed its class at run time (`MobyOcclusion::set_class`): its pose is evaluated with
    /// class `class` (index into `LevelMobys::classes`) from now on; its palette range was made to fit it.
    pub fn set_class(&mut self, k: usize, class: usize) {
        let Some(i) = self.instances.get_mut(k) else { return };
        i.class = class;
        self.pose_dirty = true;
    }
}

/// A sequence change waiting for the moby's first active update, with what the activity gate reads.
pub struct PendingChange {
    pub spawn: SpawnState,
    /// moby+0x10 (game units).
    pub position: Vec3,
    /// moby+0x30.
    pub update_distance: u8,
}

impl PendingChange {
    /// `fun_0020d868`'s test, with "drawn last frame" (+0x31) read as the port's occlusion result.
    fn active(&self, drawn: bool, camera: Option<Vec3>) -> bool {
        let d = self.update_distance as f32;
        drawn || self.update_distance == 0xff || camera.is_some_and(|c| (self.position - c).length_squared() <= d * d)
    }
}

/// Identity palette bytes for `n` matrices.
pub fn identity_palette(n: u32) -> Vec<u8> { matrix_bytes(&IDENTITY).repeat(n.max(1) as usize) }

/// One matrix as the shader reads it: 16 little-endian f32, row by row of `Rows`.
fn matrix_bytes(r: &Rows) -> [u8; MATRIX_BYTES] {
    let mut out = [0u8; MATRIX_BYTES];
    for (dst, v) in out.as_chunks_mut::<4>().0.iter_mut().zip(r.iter().flatten()) { *dst = v.to_le_bytes(); }
    out
}

/// `RC_ANIM` (default on).
pub fn anim_enabled() -> bool { !std::env::var("RC_ANIM").is_ok_and(|v| v.trim() == "0") }

pub struct MobyAnimPlugin;

impl Plugin for MobyAnimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(60.0))
            .add_systems(FixedUpdate, tick)
            .add_systems(PostUpdate, upload_palette.after(crate::moby_render::update_moby_occlusion));
    }
}

/// `MobyAnimAdvance` for every instance (the update loops skip mode-0x40 mobys), then the pending
/// first-update sequence changes of the instances that are active this tick.
fn tick(anim: Option<ResMut<MobyAnim>>, level: Res<crate::Level>, cams: Query<&Transform, With<Camera3d>>) {
    let Some(mut anim) = anim else { return };
    if !anim.enabled { return; }
    let classes = &level.0.mobys.anim;
    let camera = cams.iter().next().map(crate::game_camera::game_eye);
    let a = &mut *anim;
    for (k, i) in a.instances.iter_mut().enumerate() {
        if !i.state.skip_advance { moby_anim::advance(&mut i.state, &classes[i.class]); }
        // +0x31 (drawn last frame) is 0 before the first frame: InitMobyInstance zeroes it.
        if a.pending[k].as_ref().is_some_and(|p| p.active(a.ticks > 0 && i.visible, camera)) {
            let p = a.pending[k].take().unwrap();
            let snap_before = a.snapshots[k].is_some();
            p.spawn.apply_change(&mut i.state, &classes[i.class], &mut a.snapshots[k]);
            println!(
                "moby anim: first update after load of animation instance {k} at tick {}: seq B {} over {} ticks{}",
                a.ticks + 1, i.state.seq_b, p.spawn.blend_ticks.unwrap_or(0),
                if a.snapshots[k].is_some() && !snap_before { " (from a snapshot of the current pose)" } else { "" }
            );
        }
    }
    anim.ticks += 1;
}

/// Evaluates the palettes after a tick and rewrites the storage buffer's contents when they changed.
fn upload_palette(anim: Option<ResMut<MobyAnim>>, level: Res<crate::Level>, mut buffers: ResMut<Assets<ShaderBuffer>>, time: Res<Time<Real>>) {
    let Some(mut anim) = anim else { return };
    if anim.evaluated == Some(anim.ticks) && !anim.pose_dirty { return; }
    let first = anim.evaluated.is_none();
    anim.evaluated = Some(anim.ticks);
    anim.pose_dirty = false;
    let t0 = Instant::now();
    let a = &mut *anim;
    let len = a.palette_len.max(1) as usize * MATRIX_BYTES;
    if a.bytes.len() != len { a.bytes = identity_palette(a.palette_len); }
    let changed = if a.enabled {
        let (changed, poses) = evaluate_all(a, &level.0);
        a.eval_time += t0.elapsed();
        a.evals += 1;
        a.poses = poses;
        changed
    } else {
        false
    };
    if changed || first {
        crate::asset_write::set_buffer(&mut buffers, &a.palette, &a.bytes);
    }
    let now = time.elapsed_secs();
    if a.enabled && now - a.last_report >= 5.0 {
        a.last_report = now;
        println!(
            "moby anim: {} ticks, {} instances, {} distinct poses per tick, palette {} matrices, evaluation {:.3} ms per tick (avg of {})",
            a.ticks, a.instances.len(), a.poses, a.palette_len, a.eval_time.as_secs_f64() * 1e3 / a.evals.max(1) as f64, a.evals
        );
    }
}

/// (class, seq A, frame A, seq B, frame B, t bits, snapshot owner): instances with equal keys have equal
/// palettes. A snapshot key A is private to its instance.
type PoseKey = (usize, u8, u8, u8, u8, u32, Option<usize>);

/// Brings `anim.bytes` to this tick's palette: identity everywhere except the slot ranges of the visible,
/// not hidden instances, which hold their pose (in instance order, so a later instance wins an overlap).
/// Returns whether any byte changed and the number of distinct poses evaluated.
fn evaluate_all(anim: &mut MobyAnim, level: &LoadedLevel) -> (bool, usize) {
    let classes = &level.mobys.anim;
    let MobyAnim { instances, hidden, snapshots, mods, bytes, identity, written, .. } = anim;
    let mut changed = false;
    let mut put = |bytes: &mut Vec<u8>, at: usize, src: &[u8]| {
        let Some(dst) = bytes.get_mut(at..at + src.len()) else { return };
        if dst != src {
            dst.copy_from_slice(src);
            changed = true;
        }
    };
    let draws = |k: usize, i: &AnimInstance| i.visible && !hidden[k];
    // Slots written last time whose instance is not evaluated now go back to identity first.
    for (k, i) in instances.iter().enumerate() {
        if !written[k] || draws(k, i) { continue; }
        written[k] = false;
        for s in 0..i.slots as usize { put(bytes, (i.base as usize + s) * MATRIX_BYTES, identity); }
    }
    // Pose bytes per distinct key (all its matrices; an instance takes the first `slots`).
    let mut memo: HashMap<PoseKey, Vec<u8>> = HashMap::new();
    for (k, i) in instances.iter().enumerate() {
        if !draws(k, i) { continue; }
        let s = &i.state;
        // A snapshot or a modifier list makes the pose the instance's own.
        let snap = (s.seq_a == moby_anim::SNAPSHOT_SEQ || !mods[k].is_empty()).then_some(k);
        let key = (i.class, s.seq_a, s.frame_a, s.seq_b, s.frame_b, s.t.to_bits(), snap);
        let pose = memo.entry(key).or_insert_with(|| {
            let f = moby_anim::evaluate_posed(&classes[i.class], s, snapshots[k].as_ref(), &[], &mods[k]);
            f.iter().flat_map(matrix_bytes).collect()
        });
        let at = i.base as usize * MATRIX_BYTES;
        let n = pose.len().min(i.slots as usize * MATRIX_BYTES);
        // The rest of the range (slots past the joint count) is never written: it stays identity.
        put(bytes, at, &pose[..n]);
        written[k] = true;
    }
    (changed, memo.len())
}
