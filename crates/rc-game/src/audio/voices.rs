//! The EE sound manager (docs/plan/audio.md §3): the 30 logical sound slots and their per-frame update
//! `sound_update` (level01 0x2a0638), the sound-instance emitters (§3.3), and the 989snd voice manager that
//! turns tone volumes into SPU voice registers and hands out SPU voices.
//!
//! Sources, all level01 unless noted: slot allocation `fun_0022d7f0` 0x2a13a0, release 0x2a1348, level-def
//! play at a sound instance 0x2a1808, distance volume `fun_0022c6f8` 0x2a02e0, pan `fun_0022c830` 0x2a0418,
//! occlusion origin `fun_0022c5a8` 0x2a0190 and test `fun_0022c658` 0x2a0240, doppler scale boot 0x12f1a0,
//! the play / params callbacks 0x2a1bc8 / 0x2a1c10 (disassembled), master groups `fun_0022c8d0` 0x2a04b8,
//! sphere emitter 0x3197a0. The 989snd side mirrors OpenGOAL `989snd/vagvoice.cpp` (`MakeVolume`,
//! `AdjustVolToGroup`) with RAC1's pan table.

use super::{Spu, SPU_VOICES};
use crate::collision_query::{coll_line, QueryFlags};
use crate::ps2v::Pf;
use crate::rng::Rng;
use rc_formats::collision::Collision;
use rc_formats::moby_light::vu0_sin_cos;
use rc_formats::sound_bank::{LevelSounds, SoundDef, SoundInstance, SoundOwner};

/// Logical sound slots at 0x13e5c0 + i·0x70.
pub const SLOTS: usize = 30;
/// Slots open to ordinary owners (first-free search bound); the hero, 0x1403e0 and class 0x472 get all 30.
pub const SLOTS_OTHERS: usize = 26;
/// Occlusion ring length (slot +0x44).
pub const RING: usize = 36;
/// The CollLine_Fix mask the sound code passes: skip moby primitives | exclude surfaces with the high bit.
pub const OCCLUSION_QUERY: QueryFlags = QueryFlags(0x82);

/// Play flags (slot +5).
pub mod flags {
    /// 2-D: no pan, no doppler.
    pub const TWO_D: u8 = 0x01;
    pub const LOOP: u8 = 0x04;
    /// Do not follow the owner.
    pub const NO_FOLLOW: u8 = 0x08;
    /// Fixed volume (no distance law, no occlusion).
    pub const FIXED_VOLUME: u8 = 0x10;
    pub const NO_DOPPLER: u8 = 0x20;
    /// Follow the owner at a rotated local offset (moby +0xc0 matrix).
    pub const ROTATED_OFFSET: u8 = 0x40;
}

/// Slot states (+4).
pub mod state {
    pub const FREE: u8 = 0;
    pub const PLAYING: u8 = 1;
    /// Set by the play callback once 989snd returned a handle.
    pub const CONFIRMED: u8 = 2;
    pub const RELEASE: u8 = 4;
    pub const STOPPING: u8 = 6;
    pub const START_PENDING: u8 = 7;
}

/// The listener: the render camera (position 0x167240, rows 0x167450/60/70 = forward, left, up in game
/// coordinates) plus the underwater flag 0x167494 and water height 0x13f640.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Listener {
    pub pos: [f32; 3],
    pub rows: [[f32; 3]; 3],
    pub underwater: bool,
    pub water_height: f32,
}

/// Who owns a playing sound, for the position update. The port has no moby table yet: an owner is an id the
/// caller resolves through [`UpdateCtx::owner_pos`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Owner {
    pub id: u32,
    /// The hero, 0x1403e0 or a class 0x472 moby: may use slots 26..29 too.
    pub privileged: bool,
}

/// One logical sound (0x70 bytes in the game).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    /// +0x00: 989snd handle, −1 while a reply is pending, 0 = none.
    pub handle: i32,
    /// +0x04.
    pub state: u8,
    /// +0x05 play flags.
    pub flags: u8,
    /// +0x08: the def (copied; the game keeps a pointer).
    pub def: SoundDef,
    /// +0x0c: bank sound id (def +0x1a).
    pub bank_id: u16,
    /// +0x0e: level def index for sound-instance plays, 0xffff otherwise.
    pub class_index: u16,
    /// +0x10: volume scale, 0x400 = 1.
    pub vol: i32,
    /// +0x14: pitch bend, drawn once per play.
    pub pb: i32,
    /// +0x18: owner moby (None = static).
    pub owner: Option<Owner>,
    /// +0x1c: owning sound instance, −1 = none.
    pub sndinst: i32,
    /// +0x20: position.
    pub pos: [f32; 3],
    /// +0x30: local offset (flag 0x40).
    pub offset: [f32; 3],
    /// +0x40: occlusion ring index.
    pub occl_index: usize,
    /// +0x44: occlusion ring (non-zero = occluded).
    pub occl: [u8; RING],
}

impl Default for Slot {
    fn default() -> Self {
        Slot {
            handle: 0,
            state: state::FREE,
            flags: 0,
            def: SoundDef::default(),
            bank_id: 0,
            class_index: 0xffff,
            vol: 0,
            pb: 0,
            owner: None,
            sndinst: -1,
            pos: [0.0; 3],
            offset: [0.0; 3],
            occl_index: 0,
            occl: [0; RING],
        }
    }
}

/// An EE → IOP command (the RPC batch of one frame), with the slot whose callback receives the reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SndCommand {
    /// `SetMasterVolume(group, vol)` (0x09).
    MasterVolume { group: u8, vol: i32 },
    /// `PlaySoundVolPanPMPB(bank, sound, vol, pan, pm, pb)` (0x11), play callback 0x2a1bc8.
    Play { slot: usize, sound: u16, vol: i32, pan: i32, pm: i32, pb: i32 },
    /// `SetSoundParams_CB(handle, mask, vol, pan, pm, pb)` (0x21), liveness callback 0x2a1c10.
    SetParams { slot: usize, handle: i32, mask: u32, vol: i32, pan: i32, pm: i32, pb: i32 },
    /// `StopSound(handle)` (0x15).
    Stop { handle: i32 },
    /// `SoundIsStillPlaying_CB(handle)` (0x19), liveness callback 0x2a1c10.
    Poll { slot: usize, handle: i32 },
    /// `SetReverbEx(2, type, depth, delay, feedback)` (0x50; `super::reverb`).
    SetReverb { kind: u8, depth: i32, delay: u8, feedback: u8 },
    /// `snd_AutoReverb(2, depth, delta, channels)` (0x10): the depth glides to `depth`.
    AutoReverb { depth: i32, delta: u16, channels: u8 },
}

/// A reply: the handle 989snd returned (0 = not playing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SndReply {
    pub slot: usize,
    pub handle: i32,
    /// The play callback (0x2a1bc8) rather than the liveness one (0x2a1c10).
    pub play: bool,
}

/// Per-frame context of [`SoundSlots::update`].
pub struct UpdateCtx<'a> {
    pub listener: &'a Listener,
    pub collision: Option<&'a Collision>,
    pub rng: &'a mut Rng,
    /// Frame counter 0x15f5cc (phases the occlusion re-tests).
    pub frame: u32,
    /// Resolves an owner to its current position (moby +0x10); None = deleted (state −2/−3).
    pub owner_pos: &'a dyn Fn(u32) -> Option<[f32; 3]>,
    /// Master volume options 0x15edf0 (sfx) and 0x15edec (music); 0x400 = full.
    pub sfx_option: i32,
    pub music_option: i32,
    /// The Sound options page is open: group 2 at the music volume (`AudioSystem::sound_page`).
    pub sound_page: bool,
    /// 0x15f5c4 == 2.
    pub cutscene: bool,
    /// The game mode 0x15f5c4: outside 0 (play), 2 (scene) and 6 (space) the running sounds are only polled; the
    /// occlusion runs in 0 and 2 only.
    pub game_mode: i32,
}

type V3 = [f32; 3];
fn vsub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn vadd(a: V3, b: V3) -> V3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn vscale(a: V3, s: f32) -> V3 { [a[0] * s, a[1] * s, a[2] * s] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
/// VU0 length `sqrt((x² + y²) + z²)` (fun_001f9b48 / FastVecLength order) on PS2 floats.
fn length(v: V3) -> f32 {
    let [x, y, z] = v.map(Pf::f);
    (((x * x) + (y * y)) + (z * z)).sqrt().to_f32()
}

/// Distance volume `fun_0022c6f8` 0x2a02e0: `vol_near` inside `near`, `vol_far` beyond `far`, else
/// `vol_far + trunc((far − d)·f32(vol_near − vol_far) / (far − near))`, squared terms with def flag bit 0.
pub fn distance_volume(d: f32, def: &SoundDef) -> i32 { distance_volume_between(d, def.near, def.far, def) }

/// `SoundDistanceVolume(d, near, far, def)` (0x2a02e0) with the caller's `near` / `far` (the box emitter passes its
/// own); the def gives the volumes and the falloff bit.
pub fn distance_volume_between(d: f32, near: f32, far: f32, def: &SoundDef) -> i32 {
    if d <= near { return def.vol_near; }
    if far <= d { return def.vol_far; }
    let (d, near, far) = (Pf::f(d), Pf::f(near), Pf::f(far));
    let range = Pf::from_i32(def.vol_near - def.vol_far);
    let (a, b) = if def.flags & 1 == 0 { (far - d, far - near) } else { ((far - d) * (far - d), (far - near) * (far - near)) };
    def.vol_far + ((a * range) / b).to_i32()
}

/// Pan `fun_0022c830` 0x2a0418: `v = (fwd·p, left·p)` for `p = pos − cam`, `k = clamp(|v| − 1, 0, 1)`,
/// `trunc(−FastArcTan(v.x, v.y)·180·k·0.31830987)` degrees (0 ahead, −90 left, +90 right; fades to centre
/// inside one unit).
pub fn pan_degrees(pos: V3, l: &Listener) -> i32 {
    let p = vsub(pos, l.pos).map(Pf::f);
    let row = |r: usize| l.rows[r].map(Pf::f);
    let d = |r: [Pf; 3]| ((r[0] * p[0]) + (r[1] * p[1])) + (r[2] * p[2]);
    let (vx, vy) = (d(row(0)), d(row(1)));
    let len = ((vx * vx) + (vy * vy)).sqrt();
    let k = (len - Pf::ONE).max(Pf::ZERO).min(Pf::ONE);
    let a = crate::pad::fast_arctan(vx, vy);
    ((((-a) * Pf::b(0x4334_0000)) * k) * Pf::b(0x3ea2_f983)).to_i32()
}

/// Doppler pitch modulation (boot 0x12f1a0): `(trunc(dot·300)·0x5f4) / 0x2e5`, 1/128 semitone units.
pub fn doppler_pm(dot: f32) -> i32 {
    let x = (Pf::f(dot) * Pf::b(0x4396_0000)).to_i32();
    x.wrapping_mul(0x5f4) / 0x2e5
}

/// `fun_00213358(0.5, 6.0, out)`: `a = rand_angle, b = rand_angle, l = randf(lo, hi)`,
/// `out = ((cos a·l)·cos b, (sin a·l)·cos b, sin b·l)` with the VU0 sine/cosine.
fn random_offset(rng: &mut Rng, lo: f32, hi: f32) -> V3 {
    let a = rng.rand_angle_bits();
    let b = rng.rand_angle_bits();
    let l = Pf::f(rng.randf(lo, hi));
    let (sa, ca) = vu0_sin_cos(a);
    let (sb, cb) = vu0_sin_cos(b);
    let (sa, ca, sb, cb) = (Pf(sa), Pf(ca), Pf(sb), Pf(cb));
    [((ca * l) * cb).to_f32(), ((sa * l) * cb).to_f32(), (sb * l).to_f32()]
}

/// Occlusion origin `fun_0022c5a8`: a point 0.5–6 units around the camera, pulled to 0.75 of the way to any
/// wall between the camera and it.
fn occlusion_origin(l: &Listener, coll: Option<&Collision>, rng: &mut Rng) -> V3 {
    let p = vadd(random_offset(rng, 0.5, 6.0), l.pos);
    match coll.and_then(|c| coll_line(c, l.pos, p, OCCLUSION_QUERY)) {
        Some(hit) => vadd(vscale(vsub(hit.point, l.pos), 0.75), l.pos),
        None => p,
    }
}

/// Occlusion test `fun_0022c658`: from `origin` to `cam + v`, `v = 0.75·(pos − cam)` clamped to 64 units.
fn occluded(origin: V3, pos: V3, l: &Listener, coll: Option<&Collision>) -> u8 {
    let Some(c) = coll else { return 0 };
    let mut v = vscale(vsub(pos, l.pos), 0.75);
    let q = length(v);
    if q != 0.0 && q >= 64.0 { v = vscale(v, 64.0 / q); }
    coll_line(c, origin, vadd(l.pos, v), OCCLUSION_QUERY).is_some() as u8
}

/// The master volume of each 989snd group as `fun_0022c8d0` sets it up and `sound_update` resends it every
/// frame: 0 = sfx·8/10, 1 = music, 2 = sfx·8/10, 3 = sfx·7/10, 4 = sfx·7/10, 5 = sfx. Underwater: music
/// ×3/5. Cutscene: music 0, groups 0 and 3 halved.
pub fn master_volumes(sfx: i32, music: i32, underwater: bool, cutscene: bool) -> [i32; 6] {
    let g0 = sfx * 8 / 10;
    let g3 = sfx * 7 / 10;
    let mut v = [g0, music, g0, g3, g3, sfx];
    if cutscene {
        v[1] = 0;
        v[0] = g0 / 2;
        v[3] = g3 / 2;
    } else if underwater {
        v[1] = music * 3 / 5;
    }
    v
}

/// The 30 slots and the state `sound_update` keeps between frames.
#[derive(Clone, Debug)]
pub struct SoundSlots {
    pub slots: [Slot; SLOTS],
    /// 0x13e550: the last four camera positions, 0x13e590 the newest index.
    cam_ring: [V3; 4],
    ring_idx: usize,
    ring_primed: bool,
    /// 0x13f2f0: six occlusion origins for new sounds, regenerated once per frame (0x13f2e8 = frame).
    origins: [V3; 6],
    origins_frame: Option<u32>,
}

impl Default for SoundSlots {
    fn default() -> Self {
        SoundSlots { slots: [Slot::default(); SLOTS], cam_ring: [[0.0; 3]; 4], ring_idx: 0, ring_primed: false, origins: [[0.0; 3]; 6], origins_frame: None }
    }
}

impl SoundSlots {
    pub fn new() -> Self { Self::default() }

    /// `fun_0022d7f0` (0x2a13a0): claim a slot and queue a start. Returns the slot or −1 (loop flag differs
    /// from the def, no free slot, or the initial distance volume below 0x20). `pos` None with no owner: the
    /// sound sits at the listener (flags |= 0x11).
    #[allow(clippy::too_many_arguments)]
    pub fn play(&mut self, def: &SoundDef, mut fl: u8, owner: Option<Owner>, owner_pos: Option<V3>, pos: Option<V3>, vol: i32, listener: &Listener, rng: &mut Rng) -> i32 {
        if (fl & flags::LOOP != 0) != (def.looped != 0) { return -1; }
        let limit = if owner.is_some_and(|o| o.privileged) { SLOTS } else { SLOTS_OTHERS };
        let Some(i) = (0..limit).find(|&i| self.slots[i].state == state::FREE) else { return -1 };
        let s = &mut self.slots[i];
        s.def = *def;
        s.class_index = 0xffff;
        s.bank_id = def.index;
        s.sndinst = -1;
        s.vol = vol;
        s.owner = None;
        s.offset = [0.0; 3];
        match (owner, pos) {
            (None, None) => {
                fl |= flags::TWO_D | flags::FIXED_VOLUME;
                s.pos = listener.pos;
            }
            (None, Some(p)) => s.pos = p,
            (Some(o), _) => {
                let p = owner_pos.unwrap_or(listener.pos);
                s.pos = [p[0], p[1], p[2] + 1.0];
                s.owner = Some(o);
            }
        }
        let start_vol = if fl & flags::FIXED_VOLUME == 0 { distance_volume(length(vsub(s.pos, listener.pos)), def) } else { vol };
        if start_vol < 0x20 { return -1; }
        s.flags = fl;
        s.state = state::START_PENDING;
        s.occl_index = 0;
        s.pb = if def.pb_hi != def.pb_lo { rng.randi(def.pb_hi - def.pb_lo) + def.pb_lo } else { def.pb_hi };
        s.handle = -1;
        i as i32
    }

    /// `release_voice_slot` (0x2a1348).
    pub fn release(&mut self, slot: i32) {
        let Ok(i) = usize::try_from(slot) else { return };
        let Some(s) = self.slots.get_mut(i) else { return };
        match s.state {
            state::START_PENDING => {
                s.state = state::FREE;
                s.owner = None;
                s.sndinst = -1;
            }
            state::FREE | state::STOPPING => {}
            _ => s.state = state::RELEASE,
        }
    }

    /// The play (0x2a1bc8) and liveness (0x2a1c10) callbacks.
    pub fn apply_reply(&mut self, r: SndReply) {
        let s = &mut self.slots[r.slot];
        s.handle = r.handle;
        if r.handle == 0 {
            s.state = state::FREE;
            s.owner = None;
            s.sndinst = -1;
        } else if r.play && s.state == state::PLAYING {
            s.state = state::CONFIRMED;
        }
    }

    /// `sound_update` (0x2a0638) minus the reverb and music parts: listener velocity, master volumes, the
    /// per-slot volume / pan / doppler / occlusion, and the commands to 989snd.
    pub fn update(&mut self, ctx: &mut UpdateCtx) -> Vec<SndCommand> {
        let l = ctx.listener;
        let mut cmds = Vec::new();
        // The origin of this frame's single occlusion re-test (game mode 0/2).
        let occlusion = matches!(ctx.game_mode, 0 | 2);
        let origin = if occlusion { occlusion_origin(l, ctx.collision, ctx.rng) } else { l.pos };

        // Listener velocity from the 4-entry camera ring: average of the last ≤3 steps shorter than
        // 60·(1/60) = 1 unit. The game zeroes the ring at level init and keeps updating it through the load
        // and intro frames; the port starts at the first gameplay frame, so it primes the ring with that
        // camera position (inferred) instead of letting 0 → camera read as one huge step.
        if !self.ring_primed {
            self.cam_ring = [l.pos; 4];
            self.ring_primed = true;
        }
        self.ring_idx = (self.ring_idx + 1) % 4;
        self.cam_ring[self.ring_idx] = l.pos;
        let (mut vel, mut n) = ([0f32; 3], 0);
        let mut step;
        let (mut newer, mut older) = (self.ring_idx, (self.ring_idx + 3) % 4);
        loop {
            step = vsub(self.cam_ring[newer], self.cam_ring[older]);
            if length(step) >= 60.0 * (1.0 / 60.0) { break; }
            n += 1;
            vel = vadd(vel, step);
            newer = older;
            older = (older + 3) % 4;
            if older == self.ring_idx { break; }
        }
        if n > 1 { vel = vscale(vel, 1.0 / n as f32); }

        let mut groups = master_volumes(ctx.sfx_option, ctx.music_option, l.underwater, ctx.cutscene);
        if ctx.sound_page { groups[2] = ctx.music_option; }
        for (group, vol) in groups.into_iter().enumerate() {
            cmds.push(SndCommand::MasterVolume { group: group as u8, vol });
        }

        let mut mask = [0u32; SLOTS];
        let mut vols = [0i32; SLOTS];
        let mut dops = [0f32; SLOTS];
        for i in 0..SLOTS {
            let s = &mut self.slots[i];
            if !(s.state == state::START_PENDING || (s.handle != 0 && s.handle != -1)) { continue; }
            let mut deleted = false;
            let owner_now = s.owner.map(|o| (ctx.owner_pos)(o.id));
            if matches!(owner_now, Some(None)) {
                deleted = true;
                s.owner = None;
            }
            if s.state == state::RELEASE || (s.state != state::STOPPING && deleted && s.def.looped != 0) {
                mask[i] = 0x20;
                continue;
            }
            if !matches!(ctx.game_mode, 0 | 2 | 6) && s.state != state::START_PENDING {
                mask[i] = 0x10;
                continue;
            }
            // Game state 0 (playing): always computed. The source velocity is the owner's step, or zero without
            // an owner or with flag 8 (`sq zero` at 0x2a0b5c).
            let mut rel = [0f32; 3];
            if let (Some(Some(p)), true) = (owner_now, s.flags & flags::NO_FOLLOW == 0) {
                // Flag 0x40 (rotated local offset) needs the moby matrix; not ported: plain follow.
                let old = [s.pos[0], s.pos[1], s.pos[2] - 1.0];
                rel = vsub(p, old);
                s.pos = [p[0], p[1], p[2] + 1.0];
            }
            let rel = vsub(rel, vel);
            let to_cam = vsub(l.pos, s.pos);
            let len = length(to_cam);
            let dir = if len > 0.0 { vscale(to_cam, 1.0 / len) } else { [0.0; 3] };
            dops[i] = dot(dir, rel);
            let dist_vol;
            if s.flags & flags::FIXED_VOLUME == 0 {
                dist_vol = distance_volume(length(vsub(s.pos, l.pos)), &s.def);
                vols[i] = (dist_vol * s.vol) / 1024;
                if s.def.flags & 2 == 0 { mask[i] |= 8; }
            } else {
                dist_vol = 0x400;
                vols[i] = s.vol;
            }
            if l.underwater && s.def.flags & 4 == 0 && l.water_height < s.pos[2] { vols[i] /= 2; }
            mask[i] |= 1;
            if dist_vol < 0x20 && vols[i] < 0x20 && s.flags & flags::LOOP != 0 {
                mask[i] = 0x20;
                continue;
            }
            if s.flags & flags::TWO_D == 0 {
                mask[i] |= 2;
                if s.flags & flags::NO_DOPPLER == 0 { mask[i] |= 6; }
            }
        }

        // Occlusion: a new sound fills the ring from six origins (regenerated once per frame); a running one
        // re-tests one entry every 2nd (loop) or 4th (one-shot) frame, phased by the slot index.
        for i in 0..SLOTS {
            if mask[i] & 8 == 0 { continue; }
            let s = &mut self.slots[i];
            let count = if s.state == state::START_PENDING {
                if !occlusion { continue; }
                if self.origins_frame != Some(ctx.frame) {
                    for o in &mut self.origins { *o = occlusion_origin(l, ctx.collision, ctx.rng); }
                    self.origins_frame = Some(ctx.frame);
                }
                let mut c = 0;
                for (k, o) in self.origins.iter().enumerate() {
                    let r = occluded(*o, s.pos, l, ctx.collision);
                    s.occl[6 * k..6 * k + 6].fill(r);
                    if r != 0 { c += 6; }
                }
                c
            } else {
                if occlusion {
                    let idx = (s.occl_index + 1) % RING;
                    s.occl_index = idx;
                    let test = if s.flags & flags::LOOP != 0 { (i as u32 ^ ctx.frame) & 1 != 0 } else { (ctx.frame ^ i as u32) & 3 == 0 };
                    s.occl[idx] = if test { occluded(origin, s.pos, l, ctx.collision) } else { s.occl[(idx + RING - 1) % RING] };
                }
                s.occl.iter().filter(|&&b| b != 0).count() as i32
            };
            if count >= RING as i32 {
                vols[i] = 0;
            } else if count > 18 {
                vols[i] = (36 - count) * vols[i] / 18;
            }
        }

        // Send.
        for i in 0..SLOTS {
            if mask[i] == 0 { continue; }
            let s = &mut self.slots[i];
            let handle = s.handle;
            s.handle = -1;
            if mask[i] & 0x20 != 0 {
                if s.state == state::START_PENDING {
                    s.state = state::FREE;
                    s.owner = None;
                    s.sndinst = -1;
                } else {
                    cmds.push(SndCommand::Stop { handle });
                    s.state = state::STOPPING;
                    cmds.push(SndCommand::Poll { slot: i, handle });
                }
                continue;
            }
            if mask[i] & 0x10 != 0 {
                cmds.push(SndCommand::Poll { slot: i, handle });
                continue;
            }
            let pb = s.pb;
            let mut m: u32 = if pb != 0 { 0x11 } else { 1 };
            let (mut pan, mut pm) = (0, 0);
            if mask[i] & 2 != 0 {
                pan = pan_degrees(s.pos, l);
                m |= 6;
            }
            if mask[i] & 4 != 0 {
                m |= 8;
                pm = doppler_pm(dops[i]);
            }
            if l.underwater && s.def.flags & 8 == 0 {
                m |= 8;
                pm -= 0x5f4;
            }
            if s.state == state::START_PENDING {
                s.state = state::PLAYING;
                cmds.push(SndCommand::Play { slot: i, sound: s.bank_id, vol: vols[i], pan, pm, pb });
            } else {
                cmds.push(SndCommand::SetParams { slot: i, handle, mask: m, vol: vols[i], pan, pm, pb });
            }
        }
        cmds
    }
}

// ---------------------------------------------------------------------------------------------------
// Sound instances (gameplay section 0x0c), docs/plan/audio.md §3.3.

/// Pvars of the emitter classes 0/1/2/5: `{def, min_s, max_s, timer, slot}` (s32 each).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EmitterPvars {
    pub def: i32,
    pub min_s: i32,
    pub max_s: i32,
    pub timer: i32,
    pub slot: i32,
}

/// The sound instances the EE updates each frame.
#[derive(Clone, Debug, Default)]
pub struct Emitters {
    pub instances: Vec<SoundInstance>,
    pub pvars: Vec<EmitterPvars>,
    /// Classes met whose update is not ported, counted once (none since the reverb boxes, `super::reverb`).
    pub unported: Vec<(usize, i16)>,
}

/// `FastDecTimer` (boot 0x1f9740): 0 fires every call; otherwise decrement and fire on reaching 0.
fn fast_dec_timer(t: &mut i32) -> bool {
    if *t == 0 { return true; }
    *t = (*t).max(1) - 1;
    *t < 1
}

impl Emitters {
    /// `blocks[i]` = the pvar block of instance i (None for pvar −1).
    pub fn new(instances: Vec<SoundInstance>, blocks: &[Option<Vec<u8>>]) -> Self {
        let pvars = instances
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let w = |k: usize| blocks.get(i).and_then(|b| b.as_ref()).and_then(|b| b.get(4 * k..4 * k + 4)).map_or(0, |x| i32::from_le_bytes(x.try_into().unwrap()));
                EmitterPvars { def: w(0), min_s: w(1), max_s: w(2), timer: w(3), slot: w(4) }
            })
            .collect();
        Emitters { instances, pvars, unported: Vec::new() }
    }

    /// The per-frame sound-instance update (0x2a19a8 dispatch) for class 0 (sphere, `0x3197a0`), class 1 (box with
    /// the volume by depth, `0x319928`), class 2 (box one-shots at random points, `0x319cc8`) and class 5 (underwater
    /// loop at the camera, `0x31a078`: flags 0x15 while `underwater`). Class 3 (reverb box) has no sound of its own
    /// (`super::reverb::ReverbBox`); class 6 (music box) is `music::MusicBox`.
    pub fn update(&mut self, slots: &mut SoundSlots, sounds: &LevelSounds, l: &Listener, rng: &mut Rng) {
        for (i, inst) in self.instances.iter().enumerate() {
            let pv = &mut self.pvars[i];
            // `PlayLevelSoundAtInstance` / `release_voice_slot` guard: the slot still plays this instance's sound.
            let ours = |slots: &SoundSlots, k: i32| usize::try_from(k).ok().and_then(|k| slots.slots.get(k)).is_some_and(|s| s.sndinst == i as i32 && s.state != state::FREE);
            match inst.o_class {
                1 => {
                    // The camera in box space (the inverse rows +0x50) and the box's half size (range, range, range).
                    let p = inst.position();
                    let local = inst.to_local(vsub(l.pos, p));
                    let half = inst.to_local([inst.range; 3]);
                    let (outer, d) = (length(half), length(local));
                    if outer <= d {
                        if ours(slots, pv.slot) { slots.release(pv.slot); }
                        pv.slot = -1;
                        continue;
                    }
                    let Some(def) = usize::try_from(pv.def).ok().and_then(|k| sounds.def(SoundOwner::Level, k)) else { continue };
                    // Inside the unit cube: the def's near volume; outside it: the distance law from the cube's surface
                    // (|clamped|) to the box's corner (|half|).
                    let mut c = local;
                    let vol = if local.iter().any(|x| 1.0 < x.abs()) {
                        for x in &mut c { *x = x.clamp(-1.0, 1.0); }
                        distance_volume_between(d, length(c), outer, def)
                    } else {
                        def.vol_near
                    };
                    let fl = flags::FIXED_VOLUME | if def.looped != 0 { flags::LOOP } else { 0 };
                    if !ours(slots, pv.slot) {
                        if !fast_dec_timer(&mut pv.timer) {
                            pv.slot = -1;
                            continue;
                        }
                        let k = slots.play(def, fl, None, None, Some(p), vol, l, rng);
                        if k >= 0 {
                            let s = &mut slots.slots[k as usize];
                            s.class_index = pv.def as u16;
                            s.sndinst = i as i32;
                        }
                        pv.slot = k;
                        if pv.max_s > 0 {
                            let r = rng.randi(pv.max_s - pv.min_s);
                            pv.timer = (((pv.min_s + r) as f32 + 0.5) as i32 as f32 * 60.0) as i32;
                        }
                    }
                    if let Some(s) = usize::try_from(pv.slot).ok().and_then(|k| slots.slots.get_mut(k)) {
                        // SoundSetVolume, and the sound at the clamped point in world space (the rows +0x10).
                        s.vol = vol;
                        let m = &inst.matrix;
                        s.pos = std::array::from_fn(|k| c[0] * m[0][k] + c[1] * m[1][k] + c[2] * m[2][k] + p[k]);
                    }
                }
                2 => {
                    let p = inst.position();
                    let local = inst.to_local(vsub(l.pos, p));
                    let half = inst.to_local([inst.range; 3]);
                    let inside = (0..3).all(|k| local[k].abs() <= half[k].abs() + 1.0);
                    if !inside || ours(slots, pv.slot) { continue; }
                    if !fast_dec_timer(&mut pv.timer) {
                        pv.slot = -1;
                        continue;
                    }
                    // PlayLevelSoundAtInstance refuses a def index at or past the level's count (−1).
                    let def = usize::try_from(pv.def).ok().and_then(|k| sounds.def(SoundOwner::Level, k));
                    let k = def.map_or(-1, |def| slots.play(def, 0, None, None, Some(p), 0x400, l, rng));
                    pv.slot = k;
                    if k >= 0 {
                        let s = &mut slots.slots[k as usize];
                        s.class_index = pv.def as u16;
                        s.sndinst = i as i32;
                        // A random point of the box: randf(−1, 1) per axis through the rows +0x10.
                        let r = [rng.randf(-1.0, 1.0), rng.randf(-1.0, 1.0), rng.randf(-1.0, 1.0)];
                        let m = &inst.matrix;
                        s.pos = std::array::from_fn(|k| r[0] * m[0][k] + r[1] * m[1][k] + r[2] * m[2][k] + p[k]);
                    }
                    if pv.max_s > 0 {
                        let r = rng.randi(pv.max_s - pv.min_s);
                        pv.timer = (((pv.min_s + r) as f32 + 0.5) as i32 as f32 * 60.0) as i32;
                    }
                }
                0 => {
                    if pv.def < 0 { continue; }
                    let d = length(vsub(l.pos, inst.position()));
                    if d < inst.range {
                        let Some(def) = sounds.def(SoundOwner::Level, pv.def as usize) else { continue };
                        let fl = if def.looped != 0 { flags::LOOP } else { 0 };
                        let ours = usize::try_from(pv.slot).ok().and_then(|k| slots.slots.get(k)).is_some_and(|s| s.sndinst == i as i32 && s.state != state::FREE);
                        if !ours && fast_dec_timer(&mut pv.timer) {
                            let k = slots.play(def, fl, None, None, Some(inst.position()), 0x400, l, rng);
                            if k >= 0 {
                                let s = &mut slots.slots[k as usize];
                                s.class_index = pv.def as u16;
                                s.sndinst = i as i32;
                            }
                            pv.slot = k;
                            if pv.max_s > 0 {
                                let r = rng.randi(pv.max_s - pv.min_s);
                                // ScaleTicks(min + r) (rate factor 1.0 on NTSC), then · 60.0.
                                pv.timer = (((pv.min_s + r) as f32 + 0.5) as i32 as f32 * 60.0) as i32;
                            }
                        }
                    } else if inst.range + 1.0 < d {
                        if let Ok(k) = usize::try_from(pv.slot) {
                            if slots.slots[k].sndinst == i as i32 && slots.slots[k].state != state::FREE { slots.release(pv.slot); }
                        }
                        pv.slot = -1;
                    }
                }
                5 => {
                    let playing = usize::try_from(pv.slot).ok().is_some_and(|k| slots.slots[k].sndinst == i as i32 && slots.slots[k].state != state::FREE);
                    if l.underwater && !playing {
                        if let Some(def) = sounds.def(SoundOwner::Level, pv.def.max(0) as usize) {
                            let k = slots.play(def, flags::TWO_D | flags::LOOP | flags::FIXED_VOLUME, None, None, None, 0x400, l, rng);
                            if k >= 0 { slots.slots[k as usize].sndinst = i as i32; }
                            pv.slot = k;
                        }
                    } else if !l.underwater && playing {
                        slots.release(pv.slot);
                        pv.slot = -1;
                    }
                }
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// 989snd voice manager

/// The 989snd pan table (181 constant-power pairs, left/right, 0x3fff = full): `(trunc(0x3fff·cos(k/2°)),
/// trunc(0x3fff·sin(k/2°)))`, which reproduces the table in the RAC1 989snd v2.09 IRX (global `irx` entry
/// 22) except entry 1, whose right value there is 0xb6 (the formula and OpenGOAL's Jak table give 0x8e).
/// The disc value is used.
pub fn pan_table() -> [(i16, i16); 181] {
    let mut t = [(0i16, 0i16); 181];
    for (k, e) in t.iter_mut().enumerate() {
        let a = k as f64 * std::f64::consts::PI / 360.0;
        *e = ((16383.0 * a.cos()) as i16, (16383.0 * a.sin()) as i16);
    }
    t[1].1 = 0xb6;
    t
}

/// What owns an SPU voice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceUse {
    #[default]
    Free,
    /// A tone of the 989snd handler with this handle.
    Tone { handle: u32 },
    /// A VAG stream (never stolen).
    Stream { handle: u32 },
}

/// Per SPU voice bookkeeping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoiceSlot {
    pub owner: VoiceUse,
    pub priority: i8,
    pub group: u8,
    /// `MakeVolume` result before the group scale.
    pub basevol: (i16, i16),
    /// 989snd tick of the key-on (age for stealing).
    pub start_tick: u64,
    /// The SPU voice generation at key-on.
    pub generation: u32,
}

/// 989snd's `VoiceManager` (OpenGOAL `vagvoice.cpp`) plus the SPU voice allocator.
///
/// Allocation (the 989snd rule is not reversed, docs/plan/audio.md §7; this is the proposal of §6.6): a free
/// voice (stopped envelope) in the group's range, lowest index first; else the lowest-priority voice in the
/// range whose priority ≤ the new tone's, oldest first; else the tone is dropped. Streams have priority 127
/// and are never taken. Ranges: `SetGroupVoiceRange` puts groups 1, 2, 4 on voices 0x18–0x2f; the other
/// groups use all 48. OpenGOAL does not limit voices at all.
#[derive(Clone, Debug)]
pub struct VoiceManager {
    pub master: [i32; 32],
    pub duck: [i32; 32],
    /// `snd_SetPlaybackMode`: 1 = mono. RAC1 passes `0x15ede8 == 0` (option "stereo" on: 0).
    pub mono: bool,
    pub pan: [(i16, i16); 181],
    pub ranges: [(usize, usize); 32],
    pub voices: [VoiceSlot; SPU_VOICES],
    /// Tones dropped because no voice could be taken.
    pub dropped: u32,
}

impl Default for VoiceManager {
    fn default() -> Self {
        let mut ranges = [(0, SPU_VOICES - 1); 32];
        for g in [1, 2, 4] { ranges[g] = (0x18, 0x2f); }
        VoiceManager { master: [0x400; 32], duck: [0x10000; 32], mono: false, pan: pan_table(), ranges, voices: [VoiceSlot::default(); SPU_VOICES], dropped: 0 }
    }
}

impl VoiceManager {
    /// `MakeVolume(vol1, pan1, vol2, pan2, vol3, pan3)`: `127·258 · v2/127 · v3/127` panned by the table
    /// (0° = centre, +90° = right) with the wrap 270→0, else +90.
    pub fn make_volume(&self, vol1: i32, pan1: i32, vol2: i32, pan2: i32, vol3: i32, pan3: i32) -> (i16, i16) {
        let mut vol = vol1 * 258;
        vol = (vol * vol2) / 0x7f;
        vol = (vol * vol3) / 0x7f;
        if vol == 0 { return (0, 0); }
        if self.mono { return (vol as i16, vol as i16); }
        let mut p = (pan1 + pan3 + pan2).rem_euclid(360);
        if p >= 270 { p -= 270 } else { p += 90 }
        let (a, b) = if p < 180 { self.pan[p as usize] } else { let (x, y) = self.pan[(p - 180) as usize]; (y, x) };
        (((a as i32 * vol) / 0x3fff) as i16, ((b as i32 * vol) / 0x3fff) as i16)
    }

    /// `AdjustVolToGroup`: `v = v·(master·duck/0x10000)/0x400`, then squared: `v²/0x7ffe` with the sign kept.
    pub fn adjust_vol_to_group(&self, v: i16, group: usize) -> i16 {
        let mut volume = v as i32;
        if group >= 15 { return v; }
        if volume >= 0x7fff { volume = 0x7ffe; }
        let modifier = ((self.master[group] as i64 * self.duck[group] as i64) / 0x10000) as i32;
        volume = (volume * modifier) / 0x400;
        let sign = if volume < 0 { -1 } else { 1 };
        ((volume * volume) / 0x7ffe * sign) as i16
    }

    /// The VOLL/VOLR pair for a voice of `group` with base volume `v`.
    pub fn voice_registers(&self, v: (i16, i16), group: usize) -> [u16; 2] {
        [(self.adjust_vol_to_group(v.0, group) >> 1) as u16 & 0x7fff, (self.adjust_vol_to_group(v.1, group) >> 1) as u16 & 0x7fff]
    }

    /// Picks an SPU voice for a tone (or a stream with priority 127) of `group`.
    pub fn allocate(&mut self, spu: &mut Spu, group: usize, priority: i8, tick: u64) -> Option<usize> {
        let (lo, hi) = self.ranges[group.min(31)];
        let free = |i: usize, vm: &VoiceManager| {
            match vm.voices[i].owner {
                VoiceUse::Free => true,
                // A preseeked stream has no active envelope yet, but its voice is still reserved.
                VoiceUse::Stream { .. } => false,
                VoiceUse::Tone { .. } => !spu.voices[i].active() || spu.voices[i].generation != vm.voices[i].generation,
            }
        };
        if let Some(i) = (lo..=hi).find(|&i| free(i, self)) {
            self.voices[i] = VoiceSlot::default();
            return Some(i);
        }
        let victim = (lo..=hi)
            .filter(|&i| matches!(self.voices[i].owner, VoiceUse::Tone { .. }) && self.voices[i].priority <= priority)
            .min_by_key(|&i| (self.voices[i].priority, self.voices[i].start_tick, i))?;
        let _ = tick;
        spu.voices[victim].stop();
        self.voices[victim] = VoiceSlot::default();
        Some(victim)
    }

    /// Is voice `i` still the one keyed on for `owner`?
    pub fn owns(&self, spu: &Spu, i: usize, owner: VoiceUse) -> bool {
        self.voices[i].owner == owner && spu.voices[i].generation == self.voices[i].generation && spu.voices[i].active()
    }

    /// `SetMasterVol(group, vol)`: store and re-apply to that group's voices.
    pub fn set_master_volume(&mut self, group: usize, vol: i32, spu: &mut Spu) {
        if group >= 32 || self.master[group] == vol { return; }
        self.master[group] = vol;
        for i in 0..SPU_VOICES {
            let v = self.voices[i];
            if v.owner != VoiceUse::Free && v.group as usize == group && spu.voices[i].generation == v.generation {
                spu.voices[i].vol = self.voice_registers(v.basevol, group);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_law() {
        let def = SoundDef { near: 0.0, far: 24.0, vol_far: 0, vol_near: 2048, ..Default::default() };
        assert_eq!(distance_volume(0.0, &def), 2048);
        assert_eq!(distance_volume(30.0, &def), 0);
        assert_eq!(distance_volume(12.0, &def), 1024);
        // Truncated toward zero.
        assert_eq!(distance_volume(17.9, &def), 520);
        let sq = SoundDef { flags: 1, ..def };
        assert_eq!(distance_volume(12.0, &sq), 512);
    }

    #[test]
    fn pan_directions() {
        let l = Listener { pos: [0.0; 3], rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], ..Default::default() };
        assert_eq!(pan_degrees([10.0, 0.0, 0.0], &l), 0);
        let left = pan_degrees([0.0, 10.0, 0.0], &l);
        let right = pan_degrees([0.0, -10.0, 0.0], &l);
        assert!((-90..=-89).contains(&left), "{left}");
        assert!((89..=90).contains(&right), "{right}");
        // Inside one unit the pan fades to the centre.
        assert_eq!(pan_degrees([0.0, 0.5, 0.0], &l), 0);
    }

    #[test]
    fn volumes_and_pan_table() {
        let t = pan_table();
        assert_eq!(t[0], (0x3fff, 0));
        assert_eq!(t[1], (0x3ffe, 0xb6));
        assert_eq!(t[2], (0x3ffc, 0x11d));
        assert_eq!(t[90], (0x2d40, 0x2d40));
        let vm = VoiceManager::default();
        // Centre: 127·258 = 32766 at 0x2d40/0x3fff each side.
        assert_eq!(vm.make_volume(127, 0, 127, 0, 127, 0), (23168, 23168));
        // Hard left (pan 270) and right (90).
        assert_eq!(vm.make_volume(127, 0, 127, 270, 127, 0), (32766, 0));
        assert_eq!(vm.make_volume(127, 0, 127, 90, 127, 0), (0, 32766));
        // Group scale: master 0x400 squares; 0x2cc (the music option) ≈ 0.49.
        assert_eq!(vm.adjust_vol_to_group(32766, 0), 32766);
        let mut vm2 = VoiceManager::default();
        vm2.master[1] = 0x2cc;
        assert_eq!(vm2.adjust_vol_to_group(23168, 1), ((23168 * 0x2cc / 0x400) * (23168 * 0x2cc / 0x400) / 0x7ffe) as i16);
        assert_eq!(master_volumes(0x400, 0x2cc, false, false), [819, 716, 819, 716, 716, 1024]);
        assert_eq!(master_volumes(0x400, 0x2cc, true, false)[1], 429);
        assert_eq!(doppler_pm(0.0), 0);
        assert_eq!(doppler_pm(1.0), 300 * 0x5f4 / 0x2e5);
    }

    #[test]
    fn slot_rules() {
        let mut rng = Rng::new();
        let l = Listener::default();
        let mut s = SoundSlots::new();
        let def = SoundDef { near: 0.0, far: 24.0, vol_far: 0, vol_near: 2048, looped: 1, ..Default::default() };
        // Loop flag must match the def.
        assert_eq!(s.play(&def, 0, None, None, Some([1.0, 0.0, 0.0]), 0x400, &l, &mut rng), -1);
        assert_eq!(s.play(&def, flags::LOOP, None, None, Some([1.0, 0.0, 0.0]), 0x400, &l, &mut rng), 0);
        // Out of range: initial volume < 0x20.
        assert_eq!(s.play(&def, flags::LOOP, None, None, Some([100.0, 0.0, 0.0]), 0x400, &l, &mut rng), -1);
        // 26 for ordinary owners.
        for k in 1..SLOTS_OTHERS { assert_eq!(s.play(&def, flags::LOOP, None, None, Some([1.0, 0.0, 0.0]), 0x400, &l, &mut rng), k as i32); }
        assert_eq!(s.play(&def, flags::LOOP, None, None, Some([1.0, 0.0, 0.0]), 0x400, &l, &mut rng), -1);
        let hero = Owner { id: 1, privileged: true };
        assert_eq!(s.play(&def, flags::LOOP, Some(hero), Some([0.0; 3]), None, 0x400, &l, &mut rng), 26);
        // A pending start released is freed at once; a playing one is marked for release.
        s.release(0);
        assert_eq!(s.slots[0].state, state::FREE);
        s.slots[1].state = state::PLAYING;
        s.release(1);
        assert_eq!(s.slots[1].state, state::RELEASE);
        // Play callback: handle 0 frees, playing → confirmed.
        s.slots[2].state = state::PLAYING;
        s.apply_reply(SndReply { slot: 2, handle: 5, play: true });
        assert_eq!((s.slots[2].handle, s.slots[2].state), (5, state::CONFIRMED));
        s.apply_reply(SndReply { slot: 2, handle: 0, play: false });
        assert_eq!(s.slots[2].state, state::FREE);
    }

    #[test]
    fn timer() {
        let mut t = 0;
        assert!(fast_dec_timer(&mut t));
        let mut t = 2;
        assert!(!fast_dec_timer(&mut t));
        assert!(fast_dec_timer(&mut t));
        assert!(fast_dec_timer(&mut t));
    }
}
