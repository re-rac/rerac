//! Ratchet's animation interface: the calls the hero code makes (`SetAnim` 0x247a90, Ratchet's own
//! advance 0x247d48) and the moby fields it reads back, as a trait the engine binds to
//! `rc_formats::moby_anim` (`docs/plan/player_controller.md` §6, `docs/plan/moby_animation.md` §4).
//!
//! **`SetAnim(blend, seq, frame)` (0x247a90), what the binding must do:**
//! * if `0x13fde8 & 2`: t (+0x54) = 0 first (not used by the on-foot states);
//! * always snapshot the current pose into key A (`fun_0020ede8(moby, slot | 0x300)`, seq A = 0xff) —
//!   unlike the generic `fun_00212f90`, there is no `t > 0.025` test;
//! * frame B (+0x51) = `frame` (not clamped here), seq B (+0x53) = `seq` (0x31 ↔ 0x32 swapped with the
//!   mirror option 0x15edb5), `update_moby_animation_state`;
//! * playback speed `0x13fde0` = 1; t = 0; `blend > 0`: rate `0x13fde4` = 1 / blend (a blend over `blend`
//!   ticks), curve `0x13fdf0` = −1; `blend ≤ 0`: rate 1, eased curve `(int)(−blend) − 1` (t table at
//!   `0x17c270 + 100·curve`, length from the gp table −0x7520), curve position `0x13fdf4` = 0;
//! * loop-sound byte (+0x7c) from the target sequence header; `0x13fdec` = 1 ("anim set this tick").
//!
//! **Ratchet's advance (0x247d48)** differs from `MobyAnimAdvance`: during a blend t += `0x13fde4` (or the
//! next value of the eased curve table), on the same sequence t += `0x13fde0 · 0x13fde4`; t in (0.99, 1.01)
//! snaps to 1 and |t| < 0.01 to 0; each key step sets `0x13fde8 |= 1`, a wrap `|= 2` (bits cleared at the
//! start of every advance), and the new rate `0x13fde4` is the new key A frame's own rate word.
//!
//! Its tail also refreshes the **sequence loop sound** (+0x7c / +0x7d), exactly as `MobyAnimAdvance`'s tail does:
//! a playing slot whose voice is gone is cleared, one that plays another class sound is released, and a wanted
//! loop sound (+0x7c ≠ 0xff, from the `SetAnim` sequence header) with no slot plays `PlayClassSound(+0x7c, 4,
//! moby)`. There is no phase gate (unlike the moby advance's every-4th-tick refresh). This is the helicopter hum
//! Clank's glide carries (class 0x57 sequence 0xe, loop sound 0x10), played while he holds ✕ in the air.
//!
//! The hero owns `0x13fde0` (playback speed, [`crate::hero::Hero::anim_speed`]) and passes it to
//! [`AnimCtl::advance`] once per tick (step 1 of the hero update); the rate `0x13fde4`, the curve state
//! `0x13fdf0/f4` and the flags `0x13fde8` live in the binding.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use crate::ps2v::Pf;

/// `+0x7c` / `+0x7d`: no loop sound, no slot.
const NO_LOOP: u8 = 0xff;

/// The animation fields of Ratchet's moby that the hero code reads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimView {
    /// +0x52 / +0x53: key A / key B sequence (A = 0xff while blending from a snapshot).
    pub seq_a: u8,
    pub seq_b: u8,
    /// +0x50 / +0x51: key A / key B frame index.
    pub frame_a: u8,
    pub frame_b: u8,
    /// +0x54: blend A → B.
    pub t: f32,
    /// `0x13fde8` (Ratchet's own, not moby+0x70): bit 0 crossed a key this tick, bit 1 wrapped.
    pub flags: u8,
    /// `0x13fde4`: t increment per tick (1/blend during a blend, then the key's rate).
    pub rate: f32,
    /// `0x13fdf8`: Ratchet's current key time (`0x263920`, frame index + t), written by the advance.
    pub frame: f32,
    /// Frame count of sequence B (sequence header +0x10).
    pub frame_count_b: u8,
    /// Rate word of frame B (`*(moby+0x6c)`).
    pub frame_b_rate: f32,
    /// `0x13fdfc`: how far the readout moved this tick (`new − old` when it did not go back, else the t
    /// increment of the advance); `0x231f18(f)` "passed frame f" tests `f < frame && frame − f ≤ this`.
    pub frame_step: f32,
}

impl AnimView {
    /// `0x13fdec`: set by `SetAnim`, then `seq_a != seq_b` after every advance (a blend is running).
    pub fn blending(&self) -> bool { self.seq_a != self.seq_b }
}

/// What the hero needs from the animation system. The engine implements it on Ratchet's moby with
/// `rc_formats::moby_anim`; [`RecordingAnim`] is a data-free stand-in for tests.
pub trait AnimCtl {
    /// `SetAnim(blend, seq, frame)` (0x247a90), see the module doc. `blend` > 0: blend ticks (the game
    /// passes `(float)ticks(n)`); ≤ 0: eased curve `−blend`.
    fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32);
    /// Ratchet's advance 0x247d48 for one tick at playback speed `speed` (`0x13fde0`). The generic
    /// `MobyAnimAdvance` is not run on Ratchet (mode 2).
    fn advance(&mut self, speed: Pf);
    /// The advance's loop-sound tail (`0x247d48`'s +0x7c / +0x7d refresh, see the module doc): the wanted loop
    /// sound `+0x7c` (set by [`AnimCtl::set_anim`]) played through `sounds` with flags 4 while no slot holds it.
    /// Called right after [`AnimCtl::advance`] and its trigger; the caller passes the hero moby `0x1413d0` (the
    /// sound's owner and class). Default: nothing.
    fn refresh_loop_sound(&mut self, _moby: &crate::moby_runtime::Moby, _rng: &mut crate::rng::Rng, _sounds: &mut dyn super::HeroSounds) {}
    /// The moby's current animation fields.
    fn view(&self) -> AnimView;
    /// Frame count of sequence `seq` of Ratchet's class (0 when absent).
    fn frame_count(&self, seq: u8) -> u8;
    /// `0x247cb8(start, end)`: loop key B between `start` and `end` (re-entry at rate 1/3) until cleared;
    /// ignored unless `start ≥ 0` and `end` < the frame count of sequence B.
    fn set_loop(&mut self, start: i32, end: i32);
    /// `0x247d00()`: clear the loop range (also done by every `set_anim`).
    fn clear_loop(&mut self);
    /// `0x247d18(n)` followed by the caller's `0x13fe04 = to` (the comet strike's catch, `0x2be1c0` / `0x242930`):
    /// the next key step of the advance leaves the loop and continues at frame `to`, and the advance after it
    /// steps onto that key at once (see [`RatchetAnim`]'s `jump`). No-op without animation data.
    fn exit_loop(&mut self, _to: i32) {}
    /// `(0x13fe00 ≠ −1, 0x13fe04)`: a loop range is set, and the loop end (which outlives the range: a
    /// `clear_loop` only clears the start).
    fn loop_state(&self) -> (bool, i32) { (false, 0) }
    /// `0x13fe00`: the loop range's start (−1: none).
    fn loop_start(&self) -> i32 { -1 }
    /// One step of the eased blend curve right away (`moby+0x54 = curve[0x13fdf4++]`, what the hurt entries
    /// 0x16 / 0x75 / 0x76 do after their `SetAnim(−3, …)`); nothing outside a curve blend.
    fn curve_step(&mut self) {}
    /// The pose matrices `P_j` of the last joints of `chains` (root-to-joint byte lists of Ratchet's joint
    /// lists) in Ratchet's current pose (`fun_00210850` / `fun_002109b8`,
    /// `rc_formats::moby_anim::evaluate_chains`). Empty when there is no class data.
    fn eval_chains(&self, _chains: &[&[u8]]) -> Vec<rc_formats::moby_anim::Rows> { Vec::new() }
    /// [`AnimCtl::eval_chains`] with Ratchet's weapon-arm pose layers blended in (`MobyAnimEvalChain` 0x268ee8 walks
    /// the moby's +0x60 list): the hand under the arm. Without layer joints the same as `eval_chains`.
    /// `mods` = the moby's joint-modifier list (+0x64, the chain evaluator applies it after the layers).
    fn eval_chains_with(&self, chains: &[&[u8]], _layers: &PoseNodes, _mods: &[rc_formats::moby_anim::JointModifier]) -> Vec<rc_formats::moby_anim::Rows> { self.eval_chains(chains) }
    /// Ratchet's current local pose as one keyframe (the decode `MobyAnimDecodeLocalPose` 0x269938 does, in
    /// the frame encoding of the pose snapshot, `rc_formats::moby_anim::snapshot`). None without class data.
    fn pose_frame(&self) -> Option<rc_formats::moby_anim::MobyFrame> { None }
    /// The rate word of key `frame` of sequence `seq` (`*(seq +0x1c)[frame]`, frame header +0; what a key step of
    /// an advance continues with). 1 without class data.
    fn key_rate(&self, _seq: u8, _frame: u8) -> f32 { 1.0 }
    /// The hero moby `0x1413d0` changed (`SwitchCharacter` 0x231348 / leaving the body 0x231450,
    /// `crate::hero::bodies`): from now on every call above is the body moby's (`Some((moby, o_class, its anim
    /// fields))`: the hero code drives it with Ratchet's own advance 0x247d48 and `SetAnim` 0x247a90, as the game
    /// does through 0x1413d0) or Ratchet's again (None). Default: nothing (no animation data; [`HeroAnimCtl`] binds).
    fn bind_body(&mut self, _body: Option<(crate::moby_runtime::MobyId, i16, rc_formats::moby_anim::AnimState)>) {}
    /// The moby the calls drive now (None: Ratchet's).
    fn bound_body(&self) -> Option<crate::moby_runtime::MobyId> { None }
    /// The bound body's pose snapshot (what a blend from a snapshot key reads: the body moby's snapshot slot, for its
    /// joint points and its draw). None: none or no body bound.
    fn body_snapshot(&self) -> Option<rc_formats::moby_anim::MobyFrame> { None }
    /// Giant Clank's `0x2061f0` on Ratchet's own moby (the pilot in the cockpit; the generic moby animation, not the
    /// hero's): `moby_set_anim_snapshot(Ratchet, seq, 0, blend)` when his key B is not `seq`, then
    /// `MobyAnimAdvance(Ratchet)` (`fun_0020d580`). Default: nothing.
    fn ratchet_generic(&mut self, _seq: u8, _blend: i32) {}
}

// ------------------------------------------------------------------------------------------------
// Pose layers (the moby +0x60 list): Ratchet's weapon arm.

/// A pose layer node of Ratchet's moby (the six 0x40-byte nodes at 0x18efc0, `FUN_00263e08` links one into the
/// moby's +0x60 list for class joint list `list`): its own key A / key B of Ratchet's sequences, `t`, speed and
/// rate, and the weight `+8` with which the evaluator (`rc_formats::moby_anim::evaluate_layered`) blends it over
/// the joints of that list. The main animation (the legs' run, the walk, the jump) keeps playing underneath.
///
/// | off | field |
/// |---|---|
/// | +0x00 | joint list (Ratchet's lists 12 / 13: gp−0x7560) |
/// | +0x05 | flags (1 key step, 2 the sequence wrapped) |
/// | +0x08 | weight |
/// | +0x20 / +0x21 / +0x22 / +0x23 | frame A / frame B / seq A / seq B |
/// | +0x18 | the retarget class (the glove-holding layers: classes 1 / 2, [`HoldClass`]) |
/// | +0x24 / +0x28 / +0x2c | t / speed (1) / rate |
/// | +0x30..+0x34 | the glove-holding layers' own blend, special-pose and fade flags (`crate::hero::weapons::hold_update`) |
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimLayer {
    pub list: u8,
    pub flags: u8,
    pub weight: f32,
    pub frame_a: u8,
    pub frame_b: u8,
    pub seq_a: u8,
    pub seq_b: u8,
    pub t: f32,
    pub speed: f32,
    pub rate: f32,
    /// +0x18: the retarget class's slot in [`RatchetAnim::hold`] + 1 (0: the keys are Ratchet's own).
    pub alt: u8,
    /// +0x30 (u16): the layer runs its own blend (0.15 a tick) before it follows Ratchet's key again.
    pub own_blend: bool,
    /// +0x32 / +0x33: in (or leaving) the special poses of `0x22df10`; the way back has started.
    pub special: bool,
    pub back: bool,
    /// +0x34: fading out for good (the hand item hidden, 0x1413ff).
    pub kill: bool,
    /// The node's place in the moby's +0x60 list: `FUN_00263e08` links a new node in front, so a later one is
    /// evaluated first (a creation stamp; [`pose_layers_with`] orders by it).
    pub born: u32,
}

/// Ratchet's pose-layer nodes (the moby's +0x60 list): 0x140058 / 0x14005c (the weapon arm, lists 12 / 13),
/// 0x140050 / 0x140054 (the glove-holding layers, lists 12 / 13) and 0x140060 (the face, list 0x18:
/// `crate::hero::weapons::face_update`).
pub type PoseNodes = [Option<AnimLayer>; 5];

/// The joint lists the pose layers run on, in the order of the `joints` argument of [`pose_layers_with`]: the arms
/// (12, 13) and the face (0x18).
pub const LAYER_LISTS: [u8; 3] = [12, 13, 0x18];

/// An animation-only class a pose layer reads its keys from (`FUN_00264220`): Ratchet's classes 1 and 2, the
/// glove-holding versions of his first 0x17 sequences (the same keys and key times) for his right arm (joints 52..71,
/// list 12) and his left arm (72..91, list 13). `map` = `rc_formats::moby_anim::retarget_map`.
#[derive(Clone, Debug)]
pub struct HoldClass {
    pub anim: rc_formats::moby_anim::MobyAnimClass,
    pub map: Vec<u8>,
}

impl HoldClass {
    /// A holding class from its blob (the level's `moby_class/0001` / `0002` core blocks): its sequences and its
    /// retarget table. None when it does not parse.
    pub fn parse(blob: &[u8]) -> Option<HoldClass> {
        let c = rc_formats::moby::parse_moby_class(blob).ok()?;
        let map = rc_formats::moby_anim::retarget_map(blob, &c.header)?;
        let seqs = rc_formats::moby_anim::parse_sequences(blob, &c).ok()?;
        Some(HoldClass { anim: rc_formats::moby_anim::MobyAnimClass::new(&c, seqs), map })
    }
}

/// The holding classes' o_class numbers (the lists 12 / 13 layers' `0x198040[1]` / `0x198040[2]`).
pub const HOLD_CLASSES: [i16; 2] = [1, 2];

/// The first sequences the holding classes carry (`0x22e660`: a key on a sequence below 0x17 is read from them).
pub const HOLD_SEQS: u8 = 0x17;

/// The evaluator's view of the weapon-arm layers (`rc_formats::moby_anim::PoseLayer`) over their joints (without
/// the retarget classes: the glove-holding layers are left out; [`pose_layers_with`] has them).
pub fn pose_layers<'a>(layers: &PoseNodes, joints: &'a [Vec<u8>; 3]) -> Vec<rc_formats::moby_anim::PoseLayer<'a>> { pose_layers_with(layers, joints, &[None, None]) }

/// The evaluator's view of all of Ratchet's pose layers, in the +0x60 list's order (the newest node first): each over
/// its list's joints (`joints[k]` for list [`LAYER_LISTS`]`[k]`); a glove-holding layer's keys below [`HOLD_SEQS`] from
/// its retarget class (a layer whose class is missing is left out).
pub fn pose_layers_with<'a>(layers: &PoseNodes, joints: &'a [Vec<u8>; 3], hold: &'a [Option<std::sync::Arc<HoldClass>>; 2]) -> Vec<rc_formats::moby_anim::PoseLayer<'a>> {
    let mut v: Vec<(u32, rc_formats::moby_anim::PoseLayer<'a>)> = layers
        .iter()
        .filter_map(|l| {
            let l = l.as_ref()?;
            let j = &joints[LAYER_LISTS.iter().position(|&k| k == l.list)?];
            if j.is_empty() { return None; }
            let alt = match l.alt {
                0 => None,
                k => {
                    let c = hold.get(k as usize - 1)?.as_deref()?;
                    Some(rc_formats::moby_anim::AltKeys { class: &c.anim, map: &c.map, a: l.seq_a < HOLD_SEQS, b: l.seq_b < HOLD_SEQS })
                }
            };
            Some((l.born, rc_formats::moby_anim::PoseLayer { joints: j, seq_a: l.seq_a, frame_a: l.frame_a, seq_b: l.seq_b, frame_b: l.frame_b, t: l.t, weight: l.weight, alt }))
        })
        .collect();
    v.sort_by_key(|a| std::cmp::Reverse(a.0));
    v.into_iter().map(|(_, l)| l).collect()
}

impl AnimLayer {
    /// `FUN_00263e08(moby, list)`: a fresh node (weight 0, nothing set).
    pub fn new(list: u8) -> AnimLayer { AnimLayer { list, ..Default::default() } }

    /// `FUN_002641c0(moby, layer, seq, frame, blend, advance)`: key A = the moby's current key B (seq +0x53, frame
    /// +0x51), key B = (`seq`, `frame`), t = 0, rate = 1 / `blend`, speed 1; `advance`: one step at once
    /// ([`AnimLayer::advance`]).
    pub fn start(&mut self, anim: &dyn AnimCtl, seq: u8, frame: u8, blend: i32, advance: bool) {
        let v = anim.view();
        self.seq_a = v.seq_b;
        self.frame_a = v.frame_b;
        self.seq_b = seq;
        self.frame_b = frame;
        self.rate = 1.0 / blend as f32;
        self.t = 0.0;
        self.speed = 1.0;
        if advance { self.advance(anim); }
    }

    /// `FUN_00263f70(moby, layer)`: t += speed · rate; each key step (t ≥ 1, with the 0.99..1.01 snap) moves key
    /// B to key A and on to the next key (frame B at the sequence's end wraps to 0: flag 2), continuing with the
    /// new key A's rate (flag 1). The key decode that follows (`FUN_00264220` / `FUN_00267770`) is the
    /// evaluator's.
    pub fn advance(&mut self, anim: &dyn AnimCtl) {
        self.flags = 0;
        if self.speed == 0.0 || self.rate == 0.0 { return; }
        let mut t = self.t + self.speed * self.rate;
        if !(0.0 < self.speed) {
            self.t = t;
            return;
        }
        if 0.99 < t && t < 1.01 { t = 1.0; }
        let mut guard = 0;
        while 1.0 <= t && guard < 256 {
            guard += 1;
            self.flags |= 1;
            self.frame_a = self.frame_b;
            self.seq_a = self.seq_b;
            self.frame_b = self.frame_b.wrapping_add(1);
            if self.frame_b >= anim.frame_count(self.seq_b) {
                self.frame_b = 0;
                self.flags |= 2;
            }
            t = (t - 1.0) / self.rate;
            let r = anim.key_rate(self.seq_a, self.frame_a);
            self.rate = r;
            t *= r;
        }
        self.t = t;
    }
}

/// A data-free [`AnimCtl`]: records every `set_anim` and fakes the fields the hero reads (the blend lands
/// on key B after `blend` ticks; sequences are treated as endless, so `flags` never reports a wrap).
#[derive(Clone, Debug, Default)]
pub struct RecordingAnim {
    pub calls: Vec<(Pf, u8, i32)>,
    pub v: AnimView,
    blend_left: f32,
}

impl AnimCtl for RecordingAnim {
    fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32) {
        self.calls.push((blend, seq, frame));
        self.v.seq_a = 0xff;
        self.v.seq_b = seq;
        self.v.frame_b = frame as u8;
        self.v.t = 0.0;
        if self.v.frame_count_b == 0 { self.v.frame_count_b = 30; }
        if self.v.frame_b_rate == 0.0 { self.v.frame_b_rate = 1.0; }
        self.blend_left = if Pf::ZERO < blend { blend.to_f32() } else { 1.0 };
        self.v.rate = if Pf::ZERO < blend { 1.0 / blend.to_f32() } else { 1.0 };
    }
    fn advance(&mut self, speed: Pf) {
        self.v.flags = 0;
        if self.v.seq_a != self.v.seq_b {
            self.blend_left -= 1.0;
            self.v.t += self.v.rate;
            if self.blend_left <= 0.0 {
                self.v.seq_a = self.v.seq_b;
                self.v.frame_a = self.v.frame_b;
                self.v.t = 0.0;
                self.v.flags = 1;
            }
        } else {
            self.v.frame += speed.to_f32();
        }
    }
    fn view(&self) -> AnimView { self.v }
    fn frame_count(&self, _seq: u8) -> u8 { 30 }
    fn set_loop(&mut self, _start: i32, _end: i32) {}
    fn clear_loop(&mut self) {}
}

// ------------------------------------------------------------------------------------------------
// Ratchet's own animation path on `rc_formats::moby_anim` data.

use rc_formats::moby_anim::{snapshot, AnimState, MobyAnimClass, MobyFrame, SNAPSHOT_SEQ};

/// Eased blend curves (t per tick) at 0x17c270 + 100·(n − 1): −1 = 11 ticks, −2 = 16, −3 = 3.
pub const CURVES: [&[u32]; 3] = [
    &[0x3e19_999a, 0x3e94_7ae1, 0x3ed7_0a3d, 0x3f07_ae14, 0x3f21_47ae, 0x3f38_51ec, 0x3f4c_cccd, 0x3f5e_b852, 0x3f6e_147b, 0x3f78_51ec, 0x3f80_0000],
    &[
        0x3dcc_cccd, 0x3e4c_cccd, 0x3e94_7ae1, 0x3ec2_8f5c, 0x3eeb_851f, 0x3f0a_3d71, 0x3f1c_28f6, 0x3f2e_147b, 0x3f3d_70a4, 0x3f4c_cccd,
        0x3f59_999a, 0x3f66_6666, 0x3f70_a3d7, 0x3f78_51ec, 0x3f7d_70a4, 0x3f80_0000,
    ],
    &[0x3f00_0000, 0x3f40_0000, 0x3f80_0000],
];

/// Ratchet's animation state: the moby's anim fields (+0x50..+0x70 as [`AnimState`]; its `rate`/`speed`
/// are not used by this path) plus Ratchet's globals `0x13fde4` (rate), `0x13fde8` (flags), `0x13fdf0/f4`
/// (curve, position), `0x13fdf8` (frame readout), `0x13fe00/04` (loop range) and the snapshot frame.
#[derive(Clone, Debug)]
pub struct RatchetAnim {
    pub state: AnimState,
    pub snapshot: Option<MobyFrame>,
    pub rate: Pf,
    pub flags: u8,
    pub curve: i32,
    pub curve_pos: usize,
    pub frame: Pf,
    /// `0x13fdfc`: see [`AnimView::frame_step`].
    pub frame_step: Pf,
    pub loop_range: Option<(i32, i32)>,
    /// Option 0x15edb5: swap sequences 0x31 ↔ 0x32.
    pub mirror: bool,
    /// `0x13fe04`: the loop end as last written (kept after the range is cleared; `0x247d18`'s exit target).
    pub loop_end: i32,
    /// `0x13fe08 ≠ 0`: leave the loop at the next key step (`0x247d18`).
    pub exit: bool,
    /// The loop exit's rate `0x15f708` (= `0x7f800000`, which the PS2 FPU treats as the largest finite value, not
    /// as infinity): the next advance completes the key step at once and continues with `t = speed · rate` of the
    /// new key. Reproduced as that result (docs/plan/hardware_fidelity_layers.md), not as an infinite rate.
    pub jump: bool,
    /// The joints of the pose layers' lists (the second byte lists of Ratchet's class joint lists [`LAYER_LISTS`]: the
    /// arms 12 / 13, the face 0x18), set by the loader; empty: those layers are not evaluated.
    pub layer_joints: [Vec<u8>; 3],
    /// The glove-holding layers' retarget classes 1 / 2 ([`HoldClass`]), set by the loader; None: those layers are
    /// not evaluated.
    pub hold: [Option<std::sync::Arc<HoldClass>>; 2],
    /// `+0x7c`: the loop sound the current sequence wants (0xff none), from its header ([`AnimCtl::set_anim`]).
    pub loop_sound: u8,
    /// `+0x7d`: the sound slot playing [`RatchetAnim::loop_sound`] (0xff none).
    pub loop_slot: u8,
    /// The class sound [`RatchetAnim::loop_slot`] was started with (the game reads it from the voice entry): the
    /// slot is released when the wanted loop sound changed under it.
    pub loop_index: u8,
}

impl RatchetAnim {
    /// Spawned on sequence 0 frame 0 (what `InitMobyInstance` leaves).
    pub fn new(class: &MobyAnimClass) -> RatchetAnim {
        RatchetAnim { state: AnimState::spawn(class), snapshot: None, rate: Pf::ONE, flags: 0, curve: -1, curve_pos: 0, frame: Pf::ZERO, frame_step: Pf::ZERO, loop_range: None, mirror: false, loop_end: 0, exit: false, jump: false, layer_joints: Default::default(), hold: Default::default(), loop_sound: NO_LOOP, loop_slot: NO_LOOP, loop_index: NO_LOOP }
    }

    /// Bind to Ratchet's class for one tick of hero code.
    pub fn ctl<'a>(&'a mut self, class: &'a MobyAnimClass) -> RatchetAnimCtl<'a> { RatchetAnimCtl { a: self, class } }

    fn key_time(f: Option<&MobyFrame>) -> Pf { Pf::from_i32(f.map(|f| f.header.time as i32).unwrap_or(0)) }
}

/// [`AnimCtl`] on [`RatchetAnim`] with the class data (`SetAnim` 0x247a90, advance 0x247d48).
pub struct RatchetAnimCtl<'a> {
    pub a: &'a mut RatchetAnim,
    pub class: &'a MobyAnimClass,
}

impl RatchetAnimCtl<'_> {
    /// The read-only view of the same animation ([`RatchetAnimRef`]).
    fn r(&self) -> RatchetAnimRef<'_> { RatchetAnimRef { a: &*self.a, class: self.class } }
    fn frame_readout(&self) -> Pf { self.r().frame_readout() }
}

/// The read-only half of [`RatchetAnimCtl`] (the fields the hero code reads back), shared with [`HeroAnimCtl`].
#[derive(Clone, Copy)]
pub struct RatchetAnimRef<'a> {
    pub a: &'a RatchetAnim,
    pub class: &'a MobyAnimClass,
}

impl RatchetAnimRef<'_> {
    /// `0x263920`: the key time (frame index + t, in key-time units of 1/16).
    fn frame_readout(&self) -> Pf {
        let s = &self.a.state;
        let c = self.class;
        let fb = c.frame(s.seq_b, s.frame_b);
        let fa = if s.seq_a == SNAPSHOT_SEQ { fb } else { c.frame(s.seq_a, s.frame_a) };
        let t = Pf::f(s.t);
        let k = Pf::b(0x3d80_0000); // 0.0625
        let ta = RatchetAnim::key_time(fa);
        if t == Pf::ZERO { return ta * k; }
        if s.seq_a == s.seq_b && s.frame_a <= s.frame_b {
            let tb = RatchetAnim::key_time(fb);
            return (ta * (Pf::ONE - t) + tb * t) * k;
        }
        ta * k + t
    }

    pub fn view(&self) -> AnimView {
        let s = &self.a.state;
        let fb = self.class.frame(s.seq_b, s.frame_b);
        AnimView {
            seq_a: s.seq_a,
            seq_b: s.seq_b,
            frame_a: s.frame_a,
            frame_b: s.frame_b,
            t: s.t,
            flags: self.a.flags,
            rate: self.a.rate.to_f32(),
            frame: self.a.frame.to_f32(),
            frame_count_b: self.class.sequence(s.seq_b).map(|q| q.header.frame_count).unwrap_or(0),
            frame_b_rate: fb.map(|f| f.header.rate).unwrap_or(1.0),
            frame_step: self.a.frame_step.to_f32(),
        }
    }

    pub fn frame_count(&self, seq: u8) -> u8 { self.class.sequence(seq).map(|q| q.header.frame_count).unwrap_or(0) }

    pub fn eval_chains(&self, chains: &[&[u8]]) -> Vec<rc_formats::moby_anim::Rows> {
        rc_formats::moby_anim::evaluate_chains(self.class, &self.a.state, self.a.snapshot.as_ref(), chains)
    }

    pub fn eval_chains_with(&self, chains: &[&[u8]], layers: &PoseNodes, mods: &[rc_formats::moby_anim::JointModifier]) -> Vec<rc_formats::moby_anim::Rows> {
        let pl = pose_layers_with(layers, &self.a.layer_joints, &self.a.hold);
        rc_formats::moby_anim::evaluate_chains_posed(self.class, &self.a.state, self.a.snapshot.as_ref(), chains, &pl, mods)
    }

    pub fn pose_frame(&self) -> Option<rc_formats::moby_anim::MobyFrame> { snapshot(self.class, &self.a.state, self.a.snapshot.as_ref()) }

    pub fn key_rate(&self, seq: u8, frame: u8) -> f32 { self.class.frame(seq, frame).map_or(1.0, |f| f.header.rate) }

    pub fn loop_state(&self) -> (bool, i32) { (self.a.loop_range.is_some(), self.a.loop_end) }

    /// `0x13fe00` (−1 without a range).
    pub fn loop_start(&self) -> i32 { self.a.loop_range.map_or(-1, |r| r.0) }
}

/// The hero's animation while another body may be the hero moby (`crate::hero::bodies`): every [`AnimCtl`] call goes
/// to Ratchet's [`RatchetAnim`] or, after [`AnimCtl::bind_body`], to the body moby's own [`BodyAnim`] (the body
/// class's sequences, driven by the same advance 0x247d48 and `SetAnim` 0x247a90: the game's hero code reads and
/// writes the animation of 0x1413d0, whichever moby that is). `classes` finds a body class's animation by o_class (a
/// body whose class is missing stays unbound: the calls go to Ratchet, as with no data).
pub struct HeroAnimCtl<'a> {
    pub ratchet: RatchetAnimCtl<'a>,
    pub body: &'a mut Option<BodyAnim>,
    pub classes: &'a dyn Fn(i16) -> Option<&'a MobyAnimClass>,
}

/// The body moby's animation state while it is the hero moby ([`HeroAnimCtl`]): its moby, class and the hero
/// animation's own fields (the same globals 0x13fde4.. Ratchet's use: one set, the game keeps them in the hero block).
#[derive(Clone, Debug)]
pub struct BodyAnim {
    pub moby: crate::moby_runtime::MobyId,
    pub o_class: i16,
    pub anim: RatchetAnim,
}

impl<'a> HeroAnimCtl<'a> {
    fn body_class(&self) -> Option<&'a MobyAnimClass> { self.body.as_ref().and_then(|b| (self.classes)(b.o_class)) }

    /// `f` on the controller of the hero moby now.
    fn with<R>(&mut self, f: impl FnOnce(&mut dyn AnimCtl) -> R) -> R {
        if let Some(class) = self.body_class() {
            if let Some(b) = self.body.as_mut() { return f(&mut RatchetAnimCtl { a: &mut b.anim, class }); }
        }
        f(&mut self.ratchet)
    }

    /// The read-only view of the hero moby's animation now.
    fn r(&self) -> RatchetAnimRef<'_> {
        if let (Some(class), Some(b)) = (self.body_class(), self.body.as_ref()) { return RatchetAnimRef { a: &b.anim, class }; }
        self.ratchet.r()
    }
}

impl AnimCtl for HeroAnimCtl<'_> {
    fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32) { self.with(|a| a.set_anim(blend, seq, frame)) }
    fn advance(&mut self, speed: Pf) { self.with(|a| a.advance(speed)) }
    fn refresh_loop_sound(&mut self, moby: &crate::moby_runtime::Moby, rng: &mut crate::rng::Rng, sounds: &mut dyn super::HeroSounds) { self.with(|a| a.refresh_loop_sound(moby, rng, sounds)) }
    fn view(&self) -> AnimView { self.r().view() }
    fn frame_count(&self, seq: u8) -> u8 { self.r().frame_count(seq) }
    fn set_loop(&mut self, start: i32, end: i32) { self.with(|a| a.set_loop(start, end)) }
    fn clear_loop(&mut self) { self.with(|a| a.clear_loop()) }
    fn exit_loop(&mut self, to: i32) { self.with(|a| a.exit_loop(to)) }
    fn loop_state(&self) -> (bool, i32) { self.r().loop_state() }
    fn loop_start(&self) -> i32 { self.r().loop_start() }
    fn curve_step(&mut self) { self.with(|a| a.curve_step()) }
    fn eval_chains(&self, chains: &[&[u8]]) -> Vec<rc_formats::moby_anim::Rows> { self.r().eval_chains(chains) }
    fn eval_chains_with(&self, chains: &[&[u8]], layers: &PoseNodes, mods: &[rc_formats::moby_anim::JointModifier]) -> Vec<rc_formats::moby_anim::Rows> {
        // The weapon-arm layers are Ratchet's (his +0x60 list); a body moby has none.
        if self.body_class().is_some() { return self.r().eval_chains_with(chains, &[None; 5], mods); }
        self.r().eval_chains_with(chains, layers, mods)
    }
    fn pose_frame(&self) -> Option<rc_formats::moby_anim::MobyFrame> { self.r().pose_frame() }
    fn key_rate(&self, seq: u8, frame: u8) -> f32 { self.r().key_rate(seq, frame) }

    /// `SwitchCharacter`'s rebinding: the body's animation starts from the moby's own fields (what its placed
    /// update left: +0x50..+0x70), rate `0x13fde4` = 1 (the switch writes it), no loop (`0x247d00`, the switch's
    /// call), no pending blend curve. Rebinding the moby already bound keeps its state.
    fn bind_body(&mut self, body: Option<(crate::moby_runtime::MobyId, i16, rc_formats::moby_anim::AnimState)>) {
        match body {
            None => *self.body = None,
            Some((moby, o_class, state)) => {
                if self.body.as_ref().is_some_and(|b| b.moby == moby && b.o_class == o_class) { return; }
                let Some(class) = (self.classes)(o_class) else {
                    *self.body = None;
                    return;
                };
                let mut anim = RatchetAnim::new(class);
                anim.state = state;
                anim.rate = Pf::ONE;
                *self.body = Some(BodyAnim { moby, o_class, anim });
            }
        }
    }

    fn bound_body(&self) -> Option<crate::moby_runtime::MobyId> { self.body.as_ref().filter(|_| self.body_class().is_some()).map(|b| b.moby) }

    fn body_snapshot(&self) -> Option<rc_formats::moby_anim::MobyFrame> { self.body.as_ref().and_then(|b| b.anim.snapshot.clone()) }

    fn ratchet_generic(&mut self, seq: u8, blend: i32) {
        let a = &mut *self.ratchet.a;
        let class = self.ratchet.class;
        if a.state.seq_b != seq {
            let mut snap = a.snapshot.take();
            rc_formats::moby_anim::set_sequence(&mut a.state, class, seq, 0, blend, &mut snap);
            a.snapshot = snap;
        }
        rc_formats::moby_anim::advance(&mut a.state, class);
    }
}

impl AnimCtl for RatchetAnimCtl<'_> {
    fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32) {
        let a = &mut *self.a;
        if a.flags & 2 != 0 { a.state.t = 0.0; }
        let seq = if a.mirror && seq == 0x31 { 0x32 } else if a.mirror && seq == 0x32 { 0x31 } else { seq };
        if let Some(f) = snapshot(self.class, &a.state, a.snapshot.as_ref()) {
            a.snapshot = Some(f);
            a.state.seq_a = SNAPSHOT_SEQ;
            a.state.frame_a = 0;
        }
        a.state.frame_b = frame as u8;
        a.state.seq_b = seq;
        // `update_moby_animation_state` on the target: +0x7e the trigger count, +0x7c the loop sound (0xff none).
        match self.class.sequence(seq) {
            Some(q) => { a.state.trigger_count = q.header.trigger_count; a.loop_sound = q.header.loop_sound; }
            None => a.loop_sound = NO_LOOP,
        }
        if Pf::ZERO < blend {
            a.rate = Pf::ONE / blend;
            a.state.t = 0.0;
            a.curve = -1;
        } else {
            a.rate = Pf::ONE;
            a.curve = (-blend).to_i32() - 1;
            a.curve_pos = 0;
            a.state.t = 0.0;
        }
        // 0x247d00: the loop range and a pending exit (0x13fe00 = −1, 0x13fe08 = 0).
        a.loop_range = None;
        a.exit = false;
    }

    fn refresh_loop_sound(&mut self, moby: &crate::moby_runtime::Moby, rng: &mut crate::rng::Rng, sounds: &mut dyn super::HeroSounds) {
        let a = &mut *self.a;
        if a.loop_slot != NO_LOOP {
            if !sounds.alive(a.loop_slot as i32) {
                // The voice was freed or another moby took the slot (the game's owner test).
                a.loop_slot = NO_LOOP;
            } else if a.loop_index != a.loop_sound {
                // The slot plays another class sound of this moby (the sequence changed its loop).
                sounds.release(moby, a.loop_slot as i32);
                a.loop_slot = NO_LOOP;
            }
        } else if a.loop_sound != NO_LOOP {
            // PlayClassSound((char)+0x7c, 4, moby); a refused play (a negative index) leaves 0xff, retried next tick.
            let k = sounds.voice(moby, a.loop_sound as i8 as i32, 4, rng);
            if k < 0 { a.loop_slot = NO_LOOP; } else { a.loop_slot = k as u8; a.loop_index = a.loop_sound; }
        }
    }

    fn advance(&mut self, speed: Pf) {
        let class = self.class;
        let a = &mut *self.a;
        a.flags &= !3;
        let t0 = Pf::f(a.state.t);
        // The advance after a loop exit: the huge rate takes t past 1 at once (see `RatchetAnim::jump`).
        let mut jump = std::mem::take(&mut a.jump);
        let mut t = if jump {
            Pf::ONE
        } else if a.state.seq_a == a.state.seq_b {
            t0 + speed * a.rate
        } else if a.curve >= 0 {
            let c = CURVES.get(a.curve as usize).copied().unwrap_or(&[0x3f80_0000]);
            let v = Pf(c[a.curve_pos.min(c.len() - 1)]);
            a.curve_pos += 1;
            v
        } else {
            t0 + a.rate
        };
        if Pf::b(0x3f7d_70a4) < t && t < Pf::b(0x3f81_47ae) { t = Pf::ONE; }
        if Pf::b(0xbc23_d70a) < t && t < Pf::b(0x3c23_d70a) { t = Pf::ZERO; }
        let dt = t - t0;
        let mut guard = 0;
        while Pf::ONE <= t && guard < 256 {
            guard += 1;
            let s = &mut a.state;
            if s.seq_a != s.seq_b {
                s.seq_a = s.seq_b;
                if let Some(q) = class.sequence(s.seq_b) { s.trigger_count = q.header.trigger_count; }
            }
            a.curve = -1;
            a.flags |= 1;
            s.frame_a = s.frame_b;
            s.frame_b = s.frame_b.wrapping_add(1);
            if a.exit {
                // 0x13fe08 ≠ 0: the loop exit (rate 0x15f708, t = 0, frame B = 0x13fe04, `0x247d00`).
                a.exit = false;
                a.jump = true;
                t = Pf::ZERO;
                s.frame_b = a.loop_end as u8;
                a.loop_range = None;
                continue;
            }
            if let Some((start, end)) = a.loop_range {
                if end < s.frame_b as i32 {
                    a.rate = Pf::b(0x3eaa_aaab);
                    t = Pf::ZERO;
                    s.frame_b = start as u8;
                    continue;
                }
            }
            let fc = class.sequence(s.seq_b).map(|q| q.header.frame_count).unwrap_or(1);
            if s.frame_b >= fc {
                s.frame_b = 0;
                a.flags |= 2;
            }
            if jump {
                // (speed · 2¹²⁸ − 1) / 2¹²⁸ on the PS2 FPU: the playback speed.
                jump = false;
                t = speed;
            } else {
                t = t - Pf::ONE;
                t = t / a.rate;
            }
            a.rate = class.frame(s.seq_a, s.frame_a).map(|f| Pf::f(f.header.rate)).unwrap_or(Pf::ONE);
            t = t * a.rate;
        }
        a.state.t = t.to_f32();
        a.state.flags = a.flags;
        a.state.rate = a.rate.to_f32();
        a.state.speed = speed.to_f32();
        let f = self.frame_readout();
        let old = self.a.frame;
        self.a.frame_step = if old <= f { f - old } else { dt };
        self.a.frame = f;
    }

    fn view(&self) -> AnimView { self.r().view() }

    fn frame_count(&self, seq: u8) -> u8 { self.r().frame_count(seq) }

    fn set_loop(&mut self, start: i32, end: i32) {
        let fc = self.frame_count(self.a.state.seq_b) as i32;
        if start >= 0 && end < fc {
            self.a.loop_range = Some((start, end));
            self.a.loop_end = end;
        }
    }

    fn clear_loop(&mut self) {
        self.a.loop_range = None;
        self.a.exit = false;
    }

    fn exit_loop(&mut self, to: i32) {
        self.a.exit = true;
        self.a.loop_end = to;
    }

    fn loop_state(&self) -> (bool, i32) { self.r().loop_state() }
    fn loop_start(&self) -> i32 { self.r().loop_start() }
    fn curve_step(&mut self) {
        let a = &mut *self.a;
        if a.curve < 0 || a.state.seq_a == a.state.seq_b { return; }
        let c = CURVES.get(a.curve as usize).copied().unwrap_or(&[0x3f80_0000]);
        a.state.t = Pf(c[a.curve_pos.min(c.len() - 1)]).to_f32();
        a.curve_pos += 1;
    }

    fn eval_chains(&self, chains: &[&[u8]]) -> Vec<rc_formats::moby_anim::Rows> { self.r().eval_chains(chains) }

    fn eval_chains_with(&self, chains: &[&[u8]], layers: &PoseNodes, mods: &[rc_formats::moby_anim::JointModifier]) -> Vec<rc_formats::moby_anim::Rows> {
        self.r().eval_chains_with(chains, layers, mods)
    }

    fn pose_frame(&self) -> Option<rc_formats::moby_anim::MobyFrame> { self.r().pose_frame() }

    fn key_rate(&self, seq: u8, frame: u8) -> f32 { self.r().key_rate(seq, frame) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::moby_anim::{MobyFrameHeader, MobySequence, MobySequenceHeader};

    /// One joint, 30 keys at rate 0.5 (2 ticks per key), key time 16·k.
    fn class() -> MobyAnimClass {
        let key = |k: i16| {
            let payload = vec![0u8, 0, 0, 0, 0, 0, 0xff, 0x7f, 0, 0, 0, 0, 0, 0, 0, 0];
            MobyFrame {
                header: MobyFrameHeader { rate: 0.5, time: 16 * k, qwc: 1, quat_bytes: 8, scale_count: 0, trans_offset: 8, trans_count: 0 },
                quats: vec![[0, 0, 0, 0x7fff]],
                scales: vec![],
                trans: vec![],
                payload,
            }
        };
        let seq = MobySequence { header: MobySequenceHeader { frame_count: 30, loop_sound: 0xff, ..Default::default() }, frames: (0..30).map(key).collect(), triggers: vec![] };
        MobyAnimClass { joint_count: 1, skeleton: vec![], rest: vec![[0.0; 3]], parent_word: vec![0], sequences: vec![Some(seq)] }
    }

    /// The loop 6..21 holds key B inside the range; `exit_loop(26)` (`0x247d18` + `0x13fe04 = 0x1a`) leaves it at the
    /// next key step onto frame 26, and the advance after that steps onto key 26 at once with t = speed · its rate
    /// (the PS2 result of the rate 0x7f800000).
    #[test]
    fn loop_exit_jumps_to_the_key() {
        let c = class();
        let mut a = RatchetAnim::new(&c);
        let mut ctl = a.ctl(&c);
        ctl.set_anim(Pf::ONE, 0, 0);
        ctl.set_loop(6, 21);
        for _ in 0..120 {
            ctl.advance(Pf::ONE);
            assert!(ctl.view().frame_b as i32 <= 21, "left the loop: {:?}", ctl.view());
        }
        assert_eq!(ctl.loop_state(), (true, 21));
        ctl.exit_loop(26);
        let mut steps = 0;
        while ctl.view().frame_b != 26 && steps < 10 {
            ctl.advance(Pf::ONE);
            steps += 1;
        }
        assert_eq!(ctl.view().frame_b, 26, "never left");
        assert!(steps <= 3, "the exit waits for a key step: {steps}");
        assert_eq!(ctl.loop_state(), (false, 26));
        ctl.advance(Pf::ONE);
        let v = ctl.view();
        assert_eq!((v.frame_a, v.frame_b), (26, 27), "one advance onto key 26");
        assert!((v.t - 0.5).abs() < 1e-6, "t = speed · rate: {}", v.t);
        ctl.advance(Pf::ONE);
        assert_eq!(ctl.view().frame_a, 27);
    }

    /// The advance's loop-sound tail: sequence 1's header wants class sound 0x10, so the first refresh plays it
    /// (flags 4), the next leaves it alone, and a `SetAnim` to a sequence without a loop sound releases it.
    #[test]
    fn the_loop_sound_plays_and_releases_with_the_sequence() {
        use crate::moby_runtime::Moby;

        struct Snd { log: Vec<(i32, u32)>, released: Vec<i32>, alive: bool, slot: i32 }
        impl super::super::HeroSounds for Snd {
            fn anim_advanced(&mut self, _: &Moby, _: &AnimView, _: &AnimView, _: &mut crate::rng::Rng) {}
            fn voice(&mut self, _: &Moby, index: i32, flags: u32, _: &mut crate::rng::Rng) -> i32 { self.log.push((index, flags)); self.slot }
            fn alive(&mut self, slot: i32) -> bool { self.alive && slot == self.slot }
            fn release(&mut self, _: &Moby, slot: i32) { self.released.push(slot); }
        }

        let mut c = class();
        let mut s1 = c.sequences[0].clone().unwrap();
        s1.header.loop_sound = 0x10;
        c.sequences.push(Some(s1));

        let moby = Moby::zeroed();
        let mut rng = crate::rng::Rng::new();
        let mut snd = Snd { log: vec![], released: vec![], alive: true, slot: 3 };
        let mut a = RatchetAnim::new(&c);

        a.ctl(&c).set_anim(Pf::ONE, 1, 0);
        a.ctl(&c).refresh_loop_sound(&moby, &mut rng, &mut snd);
        a.ctl(&c).refresh_loop_sound(&moby, &mut rng, &mut snd);
        assert_eq!(snd.log, vec![(0x10, 4)], "played once with flags 4");
        assert_eq!((a.loop_sound, a.loop_slot, a.loop_index), (0x10, 3, 0x10));

        // Sequence 0 wants none: the slot is released once and forgotten.
        a.ctl(&c).set_anim(Pf::ONE, 0, 0);
        a.ctl(&c).refresh_loop_sound(&moby, &mut rng, &mut snd);
        a.ctl(&c).refresh_loop_sound(&moby, &mut rng, &mut snd);
        assert_eq!(snd.released, vec![3], "released once");
        assert_eq!((a.loop_sound, a.loop_slot), (NO_LOOP, NO_LOOP));

        // A voice that is gone (freed or another owner) is forgotten without a release.
        a.ctl(&c).set_anim(Pf::ONE, 1, 0);
        a.ctl(&c).refresh_loop_sound(&moby, &mut rng, &mut snd);
        snd.alive = false;
        a.ctl(&c).refresh_loop_sound(&moby, &mut rng, &mut snd);
        assert_eq!((a.loop_slot, snd.released.len()), (NO_LOOP, 1));
    }
}
