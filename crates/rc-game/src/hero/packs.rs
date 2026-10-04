//! **Package P4 — Clank's packs: the Heli-Pack and the Thruster-Pack.** The states 8 (glide / hover), 10 / 0xf
//! (Heli-Pack long / high jump), 0x10 / 0xd (Thruster-Pack long / high jump), 0x22 (Thruster stomp), 0x7a (pack jump
//! wall rebound) and 0x81 (Thruster-Pack hover): their SetState entries (0x23cf98), physics (0x2370b8) and
//! transitions (0x242930), and the pack tests of the other states (crouch, fall, jumps, the weapon check's end).
//! The four pack jumps are jump-group ids: their parameters, vertical curves, horizontal control and landing are
//! the shared jump system in [`super::jump`], which calls back here at the game's branch points. Spec:
//! docs/plan/hero_states.md §1.2 (P4 rows). Addresses level01; level00 (the superset) where noted.
//!
//! **Which pack.** The pack on Clank is the ready item of item slot 3, `GetClankModule(3)` 0x22ddd8
//! ([`Hero::back_module`]: 2 Heli-Pack, 3 Thruster-Pack, 4 Hydro-Pack; −1 while the slot is empty or swapping;
//! [`super::idle::BackSlot`]). Ownership is the item table `0x13d4c0 + id` ([`Hero::owned`]): the Heli-Pack moves
//! also test 0x13d4c2, the Thruster-Pack moves only the module. `0x141628` (Clank hidden) disables every pack move.
//! The glide 8 needs the Heli-Pack owned and a module other than the Hydro-Pack: with the Thruster-Pack on the back
//! it is the faster Thruster glide (5·dt instead of 3·dt, the Thruster's loop). The double jump 0xe's boost is
//! larger with the Heli-Pack owned and the Thruster-Pack on the back ([`super::jump`]).
//!
//! **Hits.** The stomp's descent hits the mobys under the feet (`coll_sphere_mobys(0.8, feet − 0.5, 0x10, Ratchet,
//! tmpl)` with the template `FUN_0026e808(1.0, Ratchet, 0x30000, facing)`), the Thruster long jump hits the crates
//! (classes 500..=540) in a 0.9 sphere 0.8 ahead (`FUN_0026eaa8(1.0, crate, Ratchet, 0x10000, pos, disp·7)`); both
//! are queued in [`Packs::hits`] by the physics and delivered through the hit sink right after the hero update
//! ([`deliver_hits`], `crate::tick`): the mobys read the records on their next update either way.
//!
//! **Sounds.** The packs' loops play in the hero's looping-sound slots 0x141568 + 4·n (`0x236798(n, Ratchet,
//! sound)`: 3 the Heli-Pack's class sound 2, 4 the Thruster-Pack's 0x12, flags 4 = loop); a group change stops them
//! all (`0x2283a8`, SetState's epilogue and the end of 0x242930). The physics queues them ([`Packs::sounds`]); the
//! hero update plays them right after the physics ([`flush_sounds`]) and stops them after the transitions
//! ([`after_transitions`]), through [`super::HeroSounds`].
//!
//! **Camera shake.** The stomp's landing shakes the camera (0x167260 = 0.2 for 40 ticks, level01 0x2390a0) through
//! the general request ([`super::fx::shake`], `crate::follow_camera::Shake`).
//!
//! **After-images and flames.** The Thruster jumps' after-images (`0x277400` / `0x277428` at SetState 0x10 / 0xd,
//! `0x277508` every physics tick: [`thruster_trail`], `crate::afterimage`) and the Thruster-Pack's two flame mobys
//! (class 0xa7, created with the pack: [`flames_on_create`], `crate::moby_update::classes::thruster_flame`).
//!
//! `0x248920` (not the pad vibration: the foot motes' parameters) at the glide's and the hover's landings: `super::pose`.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)]

use super::anim::AnimCtl;
use super::common::*;
use super::idle::dec_timer;
use super::physics::*;
use super::states::Ctx;
use super::Hero;
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::services::HitTemplate;
use crate::pad::{button, fast_diff_rots};
use crate::ps2v::Pf;
use crate::rng::Rng;

/// The pack states.
pub mod id {
    pub const GLIDE: i32 = 8;
    pub const HELI_LONG_JUMP: i32 = 10;
    pub const THRUSTER_HIGH_JUMP: i32 = 0xd;
    pub const HELI_HIGH_JUMP: i32 = 0xf;
    pub const THRUSTER_LONG_JUMP: i32 = 0x10;
    pub const STOMP: i32 = 0x22;
    pub const REBOUND: i32 = 0x7a;
    pub const HOVER: i32 = 0x81;
}

/// Back items (`GetClankModule(3)`) and item ids (the owned table).
pub mod item {
    pub const HELI_PACK: i32 = 2;
    pub const THRUSTER_PACK: i32 = 3;
    pub const HYDRO_PACK: i32 = 4;
    pub const MAGNEBOOTS: usize = 28;
}

/// 0x17c3a0: the Thruster-Pack high jump 0xd's vertical curve `(acc·dt², increment·dt² per tick, ticks)` (the
/// jump block's 0x13f734 table; −999999 keeps the acceleration), over ticks 9..44.
pub const THRUSTER_HIGH_JUMP_CURVE: [(f32, f32, i32); 5] =
    [(151.0, 0.0, 5), (0.0, 0.0, 27), (22.0, 2.0, 6), (-999_999.0, -3.0, 6), (0.0, 2.0, 99)];
/// gp−0x7518 (0x15f6e8): the Heli-Pack high jump 0xf's: 48·dt² over ticks 35..71 (34.. on PAL), the rotor's lift.
pub const HELI_HIGH_JUMP_CURVE: [(f32, f32, i32); 1] = [(48.0, 0.0, 1000)];

const DTF: f32 = f32::from_bits(0x3c88_8889);
const DT2F: f32 = f32::from_bits(0x3991_a2b4);

fn f(x: Pf) -> f32 { x.to_f32() }
fn p(x: f32) -> Pf { Pf::f(x) }

/// Sound work of the hero code for the sound layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SoundCmd {
    /// `0x236798(n, Ratchet, sound)`: class sound `sound` of Ratchet looping (flags 4) in hero slot `n`.
    Loop { n: usize, sound: i32 },
    /// `0x236738(index, flags)`: a voice.
    Voice { index: i32, flags: u32 },
    /// `release_voice_slot(slot)` when the slot still plays Ratchet's sound.
    Release { slot: i32 },
    /// `0x236798(n, moby, sound)` with another owner (the Hoverboard's hum and boost on the board, slots 6 / 7):
    /// class sound `sound` of `moby` looping (flags 4) in hero slot `n` (`pos`: where it starts).
    MobyLoop { n: usize, moby: crate::moby_runtime::MobyId, o_class: i16, pos: [f32; 3], sound: i32 },
    /// `release_voice_slot(slot)` when the slot still plays `moby`'s sound (`0x2283a8` with the slot's owner
    /// 0x141588 + 4·n set).
    ReleaseOf { slot: i32, moby: crate::moby_runtime::MobyId },
    /// `if !SoundIsAlive(item, loop n) { loop n = PlayClassSound(index, flags, item) }` on the hand item (the
    /// Pyrocitor's flame loop); the slot is kept in `super::fx::HeroFx::item_loops[n]`.
    ItemLoop { n: usize, index: i32, flags: u32 },
    /// The hand item's loop `n` released (`release_voice_slot` when still its sound) and forgotten.
    ItemRelease { n: usize },
    /// `SoundSetPitchBend(loop n, pb)` 0x2a1988 on the hand item's loop `n` (the Metal Detector's beep).
    ItemPitch { n: usize, pb: i32 },
    /// `PlayClassSound(index, flags, moby)` on a moby the item's update created (`HeroSounds::moby_sound`).
    MobySound { moby: crate::moby_runtime::MobyId, o_class: i16, pos: [f32; 3], index: i32, flags: u32 },
}

/// A hit for the moby hit path ([`deliver_hits`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PackHit {
    /// `coll_sphere_mobys(r, centre, flags, Ratchet, tmpl)`.
    Sphere { r: Pf, centre: V4, flags: u32, tmpl: HitTemplate },
    /// A hit record for one moby (`FUN_0026eaa8`).
    Moby { id: MobyId, tmpl: HitTemplate },
}

/// The pack fields of the hero block and the hero's looping-sound slots.
#[derive(Clone, Debug, PartialEq)]
pub struct Packs {
    /// 0x13f800: the stomp's fall gravity (100·dt²); 0x13f808: coming down (vz < 0 after tick 33); 0x13f80c: ticks
    /// to the ground (`LandEta`); 0x13f810: the anim frame the landing aims at (39); 0x13f814: ticks on the ground
    /// since coming down.
    pub stomp_g: f32,
    pub stomp_down: bool,
    pub stomp_eta: i32,
    pub stomp_frame: f32,
    pub stomp_ground: i32,
    /// 0x13fdb8: the rebound's direction (the facing + π at entry; shared with the wrench rebound 0x21).
    pub rebound_yaw: f32,
    /// 0x14161a: the Thruster hover latch (idle re-enters 0x81 while it is set).
    pub hover_latch: u8,
    /// gp−0x758c (0x15f674) / gp−0x7588 (0x15f678): the R1 double-tap windows into / out of the hover (30 ticks
    /// from an R1 press).
    pub tap_on: i32,
    pub tap_off: i32,
    /// 0x141568 + 4·n: the hero's looping-sound slots (n = 3 the Heli-Pack, 4 the Thruster-Pack, 6 / 7 the Hoverboard's
    /// hum and boost; −1 none) and 0x141588 + 4·n their owners (None: Ratchet).
    pub loops: [i32; 8],
    pub loop_owner: [Option<crate::moby_runtime::MobyId>; 8],
    /// Sounds queued by the hero code, played by [`flush_sounds`].
    pub sounds: Vec<SoundCmd>,
    /// Hits queued by the physics, delivered by [`deliver_hits`].
    pub hits: Vec<PackHit>,
}

impl Default for Packs {
    fn default() -> Self {
        Packs {
            stomp_g: 0.0, stomp_down: false, stomp_eta: 0, stomp_frame: 0.0, stomp_ground: 0, rebound_yaw: 0.0,
            hover_latch: 0, tap_on: 0, tap_off: 0, loops: [-1; 8], loop_owner: [None; 8], sounds: Vec::new(), hits: Vec::new(),
        }
    }
}

/// Clank shown (`0x141628 == 0`).
fn shown(h: &Hero) -> bool { h.back_slot.clank_hidden == 0 }
fn heli_owned(h: &Hero) -> bool { h.owned.has(item::HELI_PACK as usize) }

// ------------------------------------------------------------------------------------------------
// Sounds.

/// `0x236798(n, Ratchet, sound)`: start the loop in slot `n` unless it runs.
pub(super) fn loop_sound(h: &mut Hero, n: usize, sound: i32) {
    h.packs.loop_owner[n] = None;
    if h.packs.loops[n] == -1 && !h.packs.sounds.iter().any(|c| matches!(c, SoundCmd::Loop { n: m, .. } if *m == n)) {
        h.packs.sounds.push(SoundCmd::Loop { n, sound });
    }
}

/// `0x236798(n, moby, sound)` with another owner: start the loop in slot `n` unless it runs; `moby` is the slot's owner.
pub(super) fn loop_sound_on(h: &mut Hero, n: usize, moby: crate::moby_runtime::MobyId, o_class: i16, sound: i32) {
    h.packs.loop_owner[n] = Some(moby);
    let queued = h.packs.sounds.iter().any(|c| matches!(c, SoundCmd::MobyLoop { n: m, .. } if *m == n));
    if h.packs.loops[n] == -1 && !queued {
        let pos = to_f32x3(h.pos);
        h.packs.sounds.push(SoundCmd::MobyLoop { n, moby, o_class, pos, sound });
    }
}

/// Release slot `n`'s loop (the game's inline `release_voice_slot` + slot = −1), for its owner.
pub(super) fn release_loop(h: &mut Hero, n: usize) {
    let s = std::mem::replace(&mut h.packs.loops[n], -1);
    if s == -1 { return; }
    match h.packs.loop_owner[n] {
        Some(moby) => h.packs.sounds.push(SoundCmd::ReleaseOf { slot: s, moby }),
        None => h.packs.sounds.push(SoundCmd::Release { slot: s }),
    }
}

/// `0x2283a8`: every looping slot released.
pub fn stop_loops(h: &mut Hero) {
    for n in 0..h.packs.loops.len() { release_loop(h, n); }
}

/// The queued sounds, in order (a loop's slot is remembered; −1 without a sound layer, so it retries next tick
/// as the game's does when no slot is free).
pub(super) fn flush_sounds(h: &mut Hero, moby: &crate::moby_runtime::Moby, sounds: &mut dyn super::HeroSounds, rng: &mut Rng) {
    for cmd in std::mem::take(&mut h.packs.sounds) {
        match cmd {
            SoundCmd::Loop { n, sound } => h.packs.loops[n] = sounds.voice(moby, sound, 4, rng),
            SoundCmd::Voice { index, flags } => { sounds.voice(moby, index, flags, rng); }
            SoundCmd::Release { slot } => sounds.release(moby, slot),
            SoundCmd::MobyLoop { n, moby: m, o_class, pos, sound } => h.packs.loops[n] = sounds.moby_sound(m, o_class, pos, sound, 4, rng),
            SoundCmd::ReleaseOf { slot, moby: m } => sounds.release_of(m, slot),
            // The hand item's loops are flushed with the item sounds (super::gadgets::flush_item_sounds).
            SoundCmd::ItemLoop { .. } | SoundCmd::ItemRelease { .. } | SoundCmd::ItemPitch { .. } | SoundCmd::MobySound { .. } => {}
        }
    }
}

/// After the transitions: a group change (from `group`) stops the loops, then the queue is played.
pub(super) fn after_transitions(h: &mut Hero, moby: &crate::moby_runtime::Moby, sounds: &mut dyn super::HeroSounds, rng: &mut Rng, group: i32) {
    if h.group != group { stop_loops(h); }
    flush_sounds(h, moby, sounds, rng);
}

/// The queued pack hits through the hit sink (after the hero update; `hero_moby` = Ratchet).
pub fn deliver_hits(h: &mut Hero, table: &mut MobyTable, hero_moby: MobyId, sink: &mut dyn super::items::HitSink) {
    for hit in std::mem::take(&mut h.packs.hits) {
        match hit {
            PackHit::Sphere { r, centre, flags, tmpl } => { sink.sphere(table, r, centre, flags, Some(hero_moby), &tmpl); }
            PackHit::Moby { id, tmpl } => sink.deliver(table, id, &tmpl),
        }
    }
}

/// `FUN_0026e808(damage, tmpl, Ratchet, flags, dir)`: the hero's hit template.
pub(super) fn template(env: &Env, damage: f32, flags: u32, dir: V4) -> HitTemplate {
    HitTemplate { dir, attacker: env.hero_moby, flags, damage: p(damage), w20: 1, ..Default::default() }
}

/// `0x248cf8(fwd, side, up)`: a point in the hero's frame (rows 0x13f350) from the feet.
pub(super) fn local(h: &Hero, v: [f32; 3]) -> [f32; 3] {
    let r = h.rows.map(to_f32x3);
    let pos = to_f32x3(h.pos);
    std::array::from_fn(|k| ((r[0][k] * v[0] + r[1][k] * v[1]) + r[2][k] * v[2]) + pos[k])
}

// ------------------------------------------------------------------------------------------------
// The registry's three entry points.

/// SetState entry of 8, 10, 0xd, 0xf, 0x10, 0x22, 0x7a, 0x81. `None`: continue with SetState's epilogue.
pub(super) fn entry(h: &mut Hero, c: &mut Ctx, id: i32, play: bool, old_sub: i32) -> Option<bool> {
    match id {
        10 | 0xd | 0xf | 0x10 => return h.jump_group_entry(c, id, play, old_sub),
        id::GLIDE | id::HOVER => {
            // 8: group 5, vel = the displacement 0x13f450; 0x81: group 1, the latch, vel = eff 0x13f460. Both cap
            // the entry speed at 4.5 u/s (`0x2342d8`).
            if id == id::HOVER { h.packs.hover_latch = 1; }
            h.idle.blink_period = 0x68;
            h.group = if id == id::GLIDE { 5 } else { 1 };
            h.f15d4 = 0;
            let cap = DT * p(4.5);
            if cap < h.eff_len { h.scale_motion(cap / h.eff_len); }
            h.vel = if id == id::GLIDE { h.disp } else { h.eff };
            if play { h.set_anim(c.anim, c.rng, p(-2.0), 0x13, 0); }
        }
        id::STOMP => {
            let s = &mut h.packs;
            s.stomp_g = DT2F * 100.0;
            h.group = 0xb;
            s.stomp_frame = 39.0;
            h.f15d4 = 0x50;
            s.stomp_down = false;
            s.stomp_ground = 0;
            if play { h.set_anim(c.anim, c.rng, blend(6), 0x2a, 0); }
        }
        id::REBOUND => {
            h.group = 10;
            h.f15d4 = 0;
            h.packs.rebound_yaw = f(fast_add_rotations(h.rot[2], PI));
            h.speed = DT * p(9.0);
            if play { h.set_anim(c.anim, c.rng, blend(8), 0x29, 5); }
        }
        _ => {}
    }
    None
}

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(h: &mut Hero, env: &Env, anim: &mut dyn AnimCtl, _rng: &mut Rng) -> bool {
    match h.state {
        10 | 0xd | 0xf | 0x10 => {
            h.phys_jump_anim(env, anim.view().frame);
            thruster_trail(h, &anim.view());
        }
        id::GLIDE => glide_physics(h, env),
        id::STOMP => stomp_physics(h, env),
        id::REBOUND => rebound_physics(h),
        id::HOVER => hover_physics(h, env),
        _ => return false,
    }
    true
}

/// Per-state transitions.
pub(super) fn transitions(h: &mut Hero, c: &mut Ctx) {
    match h.state {
        10 | 0xd | 0xf | 0x10 => h.tr_jump(c),
        id::GLIDE => tr_glide(h, c),
        id::STOMP => tr_stomp(h, c),
        // 0x7a (with the wrench rebound 0x21): idle once the anim wraps.
        id::REBOUND => {
            if c.anim.view().flags & 2 != 0 { h.set_state(c, 0, true); }
        }
        id::HOVER => tr_hover(h, c),
        _ => {}
    }
}

// ------------------------------------------------------------------------------------------------
// Physics.

/// 8 (0x2370b8 case 8): the pack's loop (the Heli-Pack's with module 2, else the Thruster-Pack's), stick target
/// 3·dt (5·dt with the Thruster-Pack on the back), TurnTo(0.025, 0.3, 720°/s), SpeedStep(15, 7)·dt², the planar
/// velocity, vz = −0.72·target (a steady sink: 2.16 u/s, 3.6 with the Thruster-Pack), the reference height follows
/// the feet down (4 u/s), then the wall / ledge probe A.
fn glide_physics(h: &mut Hero, env: &Env) {
    if h.back_module() == item::HELI_PACK {
        release_loop(h, 4);
        loop_sound(h, 3, 2);
    } else {
        release_loop(h, 3);
        loop_sound(h, 4, 0x12);
    }
    let k = if h.back_slot.slot.id == item::THRUSTER_PACK { 5.0 } else { 3.0 };
    let t = DT * p(k);
    h.stick_target(env, t);
    h.turn_to(SCALE64 * p(0.025), SCALE64 * p(0.3), DT * p(12.566_371));
    h.lean(); // HeroLean 0x235638: super::idle.
    h.speed_step(DT2 * p(15.0), DT2 * p(7.0));
    h.set_planar_vel(Pf::b(0x47c3_4f80));
    h.vel[2] = Pf::ZERO;
    h.gravity_from(Pf::ZERO, t * p(0.72));
    if h.pos[2] < h.jump.ref_z {
        let mut r = h.jump.ref_z;
        approach(h.pos[2], DT * p(4.0), &mut r);
        h.jump.ref_z = r;
    }
    super::ledge::wall_ledge_probe_a(h, env);
}

/// 0x22: the horizontal speed brakes to 0 (30·dt²); up to 5.7 u/s (70·dt²) for 12 ticks, then from tick 34 down
/// at 100·dt²; coming down it hits the mobys under the feet every airborne tick; on the ground the shake.
fn stomp_physics(h: &mut Hero, env: &Env) {
    h.target_speed = Pf::ZERO;
    h.speed_step(DT2 * p(30.0), DT2 * p(30.0));
    h.vel = set_len2(h.vel, h.speed);
    if h.timer < ticks(12) {
        let mut vz = h.vel[2];
        approach(DT * p(5.7), DT2 * p(70.0), &mut vz);
        h.vel[2] = vz;
    } else if ticks(33) < h.timer {
        let z = h.vel[2];
        h.gravity_from(z, p(h.packs.stomp_g));
        if h.vel[2] < Pf::ZERO { h.packs.stomp_down = true; }
    }
    if h.packs.stomp_down && h.air_ticks != 0 {
        let mut centre = h.pos;
        centre[2] = centre[2] - p(0.5);
        let yaw = h.rot[2];
        let dir = [fast_cos(yaw), fast_sin(yaw), Pf::ZERO, Pf::ZERO];
        let tmpl = template(env, 1.0, 0x3_0000, dir);
        h.packs.hits.push(PackHit::Sphere { r: p(0.8), centre, flags: 0x10, tmpl });
    }
    if !h.packs.stomp_down || h.air_ticks != 0 { return; }
    // 0x2390a0: the shake record 0x167260 (along up) = 0.2 for 40 ticks.
    if h.packs.stomp_ground == 0 { super::fx::shake(h, crate::follow_camera::ShakeAxis::Up, 0.2, ticks(40)); }
    h.packs.stomp_ground += 1;
}

/// 0x7a (with 0x21): the speed brakes to 0 (24·dt²) along the rebound direction, gravity 25·dt² from the effective
/// vz while above the ground, the facing held toward the wall.
pub(super) fn rebound_physics(h: &mut Hero) {
    let mut s = h.speed;
    approach(Pf::ZERO, DT2 * p(24.0), &mut s);
    h.speed = s;
    let y = p(h.packs.rebound_yaw);
    h.vel[0] = fast_cos(y) * h.speed;
    h.vel[1] = fast_sin(y) * h.speed;
    h.vel[2] = if Pf::ZERO < h.height { h.eff[2] - DT2 * p(25.0) } else { Pf::ZERO };
    h.target_yaw = fast_add_rotations(y, PI);
    h.turn_to(SCALE64 * p(0.05), SCALE64 * p(0.2), DT * p(15.009_831));
}

/// 0x81 (0x2370b8 case 0x23 / 0x81; level00's adds the slippery brake): the Thruster's loop; stick target
/// (5.75·dt, 4·dt above 0.7) × the stick; the turn (L2 / R2: toward the camera); AirAccel 7·dt² (3.5 with the
/// stick below 0.2; on a slippery floor 3.7 / 1.2); the wall check; below 0.7 the feet spring to 0.17 above the
/// ground (the position itself), above it vz = −2.88 u/s.
fn hover_physics(h: &mut Hero, env: &Env) {
    loop_sound(h, 4, 0x12);
    let high = p(0.7) < h.height;
    let low = DT * p(4.0);
    let t = if high { low } else { DT * p(5.75) };
    h.stick_target(env, h.stick_mag * t);
    // Port-only: the Port Options strafe keeps L2 / R2 from the hero's pad and reports them as `Hero::strafe`.
    let strafe = env.pad.held & (button::L2 | button::R2) != 0 || h.strafe;
    if p(0.2) < h.stick_mag && !strafe {
        h.turn_to(SCALE64 * p(0.027), SCALE64 * p(0.3), (DT * p(5.934_119)) * h.stick_mag);
    } else if strafe {
        let keep = h.target_yaw;
        h.target_yaw = env.cam_yaw;
        h.turn_to(SCALE64 * p(0.017), SCALE64 * p(0.3), DT * p(4.712_389));
        h.target_yaw = keep;
    }
    h.lean(); // HeroLean 0x235638: super::idle.
    let slow = h.stick_mag < p(0.2);
    let mut a = if slow { 3.5 } else { 7.0 };
    // Level00: on a slippery floor (0x140632) the hover brakes less (P1's open item).
    if h.f0632 != 0 { a = if slow { 1.2 } else { 3.7 }; }
    h.air_accel(DT2 * p(a));
    h.wall_check(env, 1);
    if !high {
        let (mut z, mut v) = (h.pos[2], h.f654);
        spring(h.ground_z + p(0.17), SCALE64 * p(0.02), SCALE64 * p(0.35), DT * p(2.5), &mut z, &mut v);
        (h.pos[2], h.f654) = (z, v);
        return;
    }
    h.gravity_from(Pf::ZERO, low * p(0.72));
    h.f654 = h.vel[2];
}

/// Ratchet's class (`CreateMoby(Ratchet+0xa6)` of the after-images).
const RATCHET_CLASS: i16 = 0;

/// SetState 0x10 / 0xd (0x23cf98): `0x277400(Ratchet, 0x1409c0)`, then `0x277428(0x1409c0, alpha, back)` per ghost
/// (crate::afterimage).
pub(super) fn thruster_trail_start(h: &mut Hero, ghosts: &[(u8, i32)]) {
    let t = &mut h.fx.trails.hero;
    t.start(RATCHET_CLASS);
    for &(a, b) in ghosts { t.add(a, b); }
}

/// `HeroStatePhysics` 0x2370b8 for 0x10 / 0xd (after `HeroLean`): `0x277508(0x1409c0, (ticks(12) < T) << 1)` with
/// Ratchet's moby as the last write-back left it and his anim keys now.
fn thruster_trail(h: &mut Hero, v: &super::AnimView) {
    if h.state != id::THRUSTER_LONG_JUMP && h.state != id::THRUSTER_HIGH_JUMP { return; }
    let fade = if ticks(12) < h.timer { 2 } else { 0 };
    hero_trail_update(h, v, fade);
}

/// `0x277508(0x1409c0, fade)` from Ratchet's state physics: his moby as the last write-back left it and his anim keys
/// now (the Thruster jumps here, the gadget lunge 0x20 in super::walloper).
pub(super) fn hero_trail_update(h: &mut Hero, v: &super::AnimView, fade: u8) {
    let f = &h.fx.frame;
    let place = crate::afterimage::Place { rows: f.rows, position: f.position };
    let pose = crate::afterimage::Pose { seq_a: v.seq_a, seq_b: v.seq_b, frame_a: v.frame_a, frame_b: v.frame_b, t: v.t };
    h.fx.trails.hero.update(Some((place, pose)), fade);
}

/// `HeroItemsCreate` 0x22f3c0, slot 3: a pack moby created for back item 3 with Clank shown (0x141628 = 0) creates the
/// Thruster-Pack's two flames (`FUN_002c9da0(0)`, `(1)`: class 0xa7, crate::moby_update::classes::thruster_flame),
/// made by the tick right after the hero update (super::fx::create_mobys). `created` = the slot was empty before
/// this update's creation.
pub(super) fn flames_on_create(h: &mut Hero, created: bool) {
    let s = &h.back_slot;
    if !created || s.slot.state != 2 || s.slot.id != item::THRUSTER_PACK || s.clank_hidden != 0 { return; }
    // The pack moby exists only with the back classes (`HeroItemsCreate` creates it from the item's class).
    if h.back.as_ref().is_none_or(|b| b.state == 0) { return; }
    for side in 0..2 { h.fx.mobys.push(super::fx::MobySpawn::ThrusterFlame { side }); }
}

/// The jump physics' horizontal control 0x234b40 for 10 and 0x10 (the others: [`Hero::air_control`]).
/// 10: no air control — the knockback push, momentum = push, speed = |eff.xy| (the windup's push carries it).
/// 0x10: after tick 54 the gravity eases to 25·dt² (7·dt² per tick); target speed 0x13f744 (3.5·dt from tick 55),
/// SpeedStep(44, 45)·dt², the velocity straight along the moby's yaw.
pub(super) fn jump_horizontal(h: &mut Hero) {
    if h.state == id::HELI_LONG_JUMP {
        h.drag(p(0.7), Pf::ZERO);
        h.momentum = h.push;
        h.speed = h.eff_len_xy;
        return;
    }
    if ticks(54) < h.timer {
        let mut g = h.jump.g;
        approach(DT2 * p(25.0), DT2 * p(7.0), &mut g);
        h.jump.g = g;
    }
    h.target_speed = if ticks(55) <= h.timer { DT * p(3.5) } else { h.jump.f744 };
    h.speed_step(DT2 * p(44.0), DT2 * p(45.0));
    h.vel = set_len2(h.vel, Pf::ZERO);
    let y = h.moby_rot[2];
    h.vel[0] = h.vel[0] + fast_cos(y) * h.speed;
    h.vel[1] = h.vel[1] + fast_sin(y) * h.speed;
}

/// The Thruster long jump's crate test (jump physics, 0x10 after tick 8 while the anim is before frame 28): the
/// mobys in a 0.9 sphere 0.8 ahead / 0.3 up (`coll_sphere_mobys`, flags 0); every crate (classes 500..=540) gets a
/// hit (damage 1, flags 0x10000, direction = the displacement × 7).
pub(super) fn thruster_crates(h: &mut Hero, env: &Env, frame: f32) {
    if h.state != id::THRUSTER_LONG_JUMP || !(ticks(8) < h.timer) || !(frame < 28.0) { return; }
    let Some(sc) = env.mobys else { return };
    let centre = local(h, [0.8, 0.0, 0.3]);
    let listed = crate::collision_query::coll_sphere_mobys(sc, centre, 0.9, crate::collision_query::QueryFlags(0), env.hero_moby);
    if listed.is_empty() { return; }
    let dir = vscale(h.disp, p(7.0));
    for id in listed {
        let crate_ = sc.mobys.moby(id).is_some_and(|m| (500..=540).contains(&m.o_class));
        if crate_ {
            let tmpl = template(env, 1.0, 0x1_0000, dir);
            h.packs.hits.push(PackHit::Moby { id, tmpl });
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Transitions.

/// 8 (0x242930 case 8).
fn tr_glide(h: &mut Hero, c: &mut Ctx) {
    let pad = c.env.pad;
    if h.ledge != 0 && h.timer < ticks(9) {
        // A wall within the first 9 ticks after a jump: the wall jump.
        if matches!(h.prev_state, 7 | 9 | 0xe | 0x11) { h.set_state(c, 0x11, true); }
        return;
    }
    if ticks(30) < h.timer && pad.held & button::CROSS == 0 && pad.held_unmirrored & button::CROSS == 0 {
        h.set_state(c, 6, true);
        return;
    }
    if h.f838 != 0 {
        h.set_state(c, 0x18, true);
        return;
    }
    if h.air_ticks != 0 && p(0.4) <= h.pos[2] - h.ground_z { return; }
    // Landing: 0x13f524 shortens the idle / walk / fall jump buffer for the first ticks.
    if h.timer < ticks(10) { h.f524 = ticks(12) - h.timer; }
    h.stick_target(c.env, Pf::ONE);
    if h.prev_state == id::THRUSTER_LONG_JUMP && h.timer < ticks(4) {
        if h.set_state(c, 3, false) {
            h.set_anim(c.anim, c.rng, p(-1.0), 5, 0);
            h.lockout = ticks(34);
        }
        return;
    }
    // The landings' foot motes `0x248920` (super::pose).
    if pad.held & button::CROUCH != 0 {
        h.set_state(c, 4, true);
        h.land_motes(super::pose::motes::LAND);
        return;
    }
    if h.target_speed <= SPEED_TABLE[0][3] {
        if h.set_state(c, 0, false) {
            if h.air_ticks != 0 {
                h.set_anim(c.anim, c.rng, blend(9), 7, 0x17);
            } else {
                let s = h.idle_seq();
                h.set_anim(c.anim, c.rng, p(-2.0), s, 0);
            }
            h.land_motes(super::pose::motes::LAND);
        }
        return;
    }
    let facing = fast_diff_rots(h.target_yaw, h.rot[2]) < HALF_PI;
    if h.f0632 != 0 || h.f13f9 != 0 {
        h.set_state(c, 2, true);
        return;
    }
    h.set_state(c, 2, false);
    if h.state == 2 {
        h.substate = 1;
        if facing {
            h.set_anim(c.anim, c.rng, blend(8), 4, 2);
            h.land_run_blend = 1;
        } else {
            // DAT_0015f6dc = 0.
            h.set_anim(c.anim, c.rng, blend(12), 4, 0);
        }
    }
    h.land_motes(super::pose::motes::LAND_ROWS);
}

/// 0x22: ✕ within the first 12 ticks (not after a pack jump / water / burn jump, nor the glide or fall of a
/// Thruster long jump) → the Thruster long jump 0x10 (the pass goes on); the landing anim aims at frame 39 by the
/// ground; idle when the anim wraps.
fn tr_stomp(h: &mut Hero, c: &mut Ctx) {
    let pp = h.prev_state;
    if h.timer < ticks(12)
        && !(h.prev_prev_state == 0x10 && (pp == 8 || pp == 6))
        && !matches!(pp, 0xf | 10 | 0x10 | 0x12 | 0x3c)
        && c.env.pad.pressed & button::CROSS != 0
    {
        h.set_state(c, id::THRUSTER_LONG_JUMP, true);
    }
    let eta = h.land_eta(c, p(60.0), p(h.packs.stomp_g), p(-1.0)).to_i32();
    h.packs.stomp_eta = eta;
    if !h.packs.stomp_down || h.f65c == 0 || eta == 0 {
        h.anim_speed = Pf::ONE;
    } else {
        let v = c.anim.view();
        h.anim_speed_for_ticks(p(h.packs.stomp_frame), Pf::from_i32(eta), p(0.5), p(-1.0), &v);
    }
    if c.anim.view().flags & 2 != 0 { h.set_state(c, 0, true); }
}

/// 0x81 (0x242930 case 0x23 / 0x81): the pit (surface 8 / 0xc below 0.5) → 0x79; after 20 ticks ✕ or an R1
/// double tap ends the hover: latch cleared, 25-tick lockout, idle (the idle anim on the ground, the jump's
/// landing frames in the air).
fn tr_hover(h: &mut Hero, c: &mut Ctx) {
    if h.f063a != 0 && h.height < p(0.5) {
        h.set_state(c, 0x79, true);
        return;
    }
    let pad = c.env.pad;
    dec_timer(&mut h.packs.tap_off);
    let tap = h.packs.tap_off != 0 && pad.held & button::R1 != 0 && pad.pressed & button::R1 != 0;
    if pad.pressed & button::R1 != 0 { h.packs.tap_off = ticks(30); }
    if h.timer <= ticks(20) || (pad.pressed & button::CROSS == 0 && !tap) { return; }
    h.packs.hover_latch = 0;
    h.lockout = ticks(25);
    if !h.set_state(c, 0, false) { return; }
    if h.air_ticks == 0 {
        let s = h.idle_seq();
        h.set_anim(c.anim, c.rng, p(-2.0), s, 0);
    } else {
        h.set_anim(c.anim, c.rng, blend(9), 7, 0x17);
    }
    h.land_motes(super::pose::motes::LAND);
}

// ------------------------------------------------------------------------------------------------
// The pack tests of the other states (seams).

/// `HeroCrouchJumps` 0x242420 after the ✕ test (crouch longer than 6 ticks, ✕ within 9): moving (stick > 0.7,
/// forward speed > 0.9 u/s) the long jumps — Heli-Pack 10 (owned; not with a floor-like face within 2.5 ahead) or
/// Thruster-Pack 0x10 (not with a non-crate face within 2.5 ahead; the lockout 0x13f508 swallows the press) —
/// standing the high jumps 0xd (Thruster) / 0xf (Heli, owned). True: handled; false → the plain jump 7.
pub(super) fn crouch_jump(h: &mut Hero, c: &mut Ctx) -> bool {
    let m = h.back_module();
    let (stick, fwd) = (f(h.stick_mag), f(h.fwd_speed));
    let (dist, elev) = (h.wall_ahead[0], h.wall_ahead[1]);
    let moving = 0.7 < stick && DTF * 0.9 < fwd;
    if m == item::HELI_PACK && heli_owned(h) && shown(h) && !(dist <= 2.5 && std::f32::consts::FRAC_PI_4 <= elev) && moving {
        h.set_state(c, id::HELI_LONG_JUMP, true);
        return true;
    }
    if m == item::THRUSTER_PACK && shown(h) && moving && !(dist <= 2.5 && h.f5a4 == 0) {
        if h.f508 == 0 { h.set_state(c, id::THRUSTER_LONG_JUMP, true); }
        return true;
    }
    if m == item::THRUSTER_PACK && shown(h) {
        h.set_state(c, id::THRUSTER_HIGH_JUMP, true);
        return true;
    }
    if m == item::HELI_PACK && shown(h) && heli_owned(h) {
        h.set_state(c, id::HELI_HIGH_JUMP, true);
        return true;
    }
    false
}

/// Idle with the hover latch 0x14161a set (and not on a magnetic floor with the Magneboots) → 0x81.
pub(super) fn hover_latched(h: &Hero) -> bool {
    h.packs.hover_latch != 0 && !(h.f658 != 0 && h.owned.has(item::MAGNEBOOTS))
}

/// The fall 6's glide test (✕ held, Heli-Pack owned, the back module not the Hydro-Pack, Clank shown, no
/// lockout, height > 0.6 → 8). True when it changed the state.
pub(super) fn fall_to_glide(h: &mut Hero, c: &mut Ctx) -> bool {
    if c.env.pad.held & button::CROSS == 0 || !heli_owned(h) || h.back_module() == item::HYDRO_PACK || !shown(h) { return false; }
    if h.lockout != 0 || !(p(0.6) < h.height) { return false; }
    h.set_state(c, id::GLIDE, true);
    true
}

/// The fall 6's Thruster test: ✕ and R1 / R2 pressed within 9 ticks and less than 8 apart, R1 / R2 held, ✕ this
/// tick, and (the last jump not descending, or above 1.5) → 0x10. True when it changed the state.
pub(super) fn fall_to_thruster(h: &mut Hero, c: &mut Ctx) -> bool {
    let pad = c.env.pad;
    if h.back_module() != item::THRUSTER_PACK || !shown(h) { return false; }
    if !pad.combo(button::CROSS, button::CROUCH, ticks(9), ticks(8)) { return false; }
    if pad.held & button::CROUCH == 0 || pad.pressed & button::CROSS == 0 { return false; }
    if !(h.jump.descending == 0 || p(1.5) < h.height) { return false; }
    h.set_state(c, id::THRUSTER_LONG_JUMP, true);
    true
}

/// The jump transitions' Thruster combo after the flip chain: R1 / R2 held and ✕ within 8 ticks, after tick 9
/// (at once in the double jump), not from a pack / wall / water / burn jump → 0x10. True: the pass ends.
pub(super) fn jump_thruster_combo(h: &mut Hero, c: &mut Ctx) -> bool {
    if h.back_module() != item::THRUSTER_PACK || !shown(h) { return false; }
    if matches!(h.state, 0x10 | 0x11 | 0x12 | 0x3c | 0xf | 10 | 0x69) { return false; }
    if !(h.state == 0xe || ticks(9) < h.timer) || c.env.pad.held & button::CROUCH == 0 { return false; }
    if c.env.pad.pressed_within(button::CROSS, ticks(8)).is_none() { return false; }
    h.set_state(c, id::THRUSTER_LONG_JUMP, true);
    true
}

/// The jump transitions' R1 / R2 tap (within 2 ticks, in the first 7 ticks of 7 / 9 / 0xb / 0xc / 0xd / 0xf /
/// 0x1c) with a forward speed above 0.9 u/s: the Heli long jump 10 (owned) or the Thruster long jump 0x10 (no
/// lockout 0x13f508). The pass goes on in the new state (the game does not break).
pub(super) fn jump_pack_tap(h: &mut Hero, c: &mut Ctx) {
    if !(h.timer < ticks(7)) || c.env.pad.pressed_within(button::CROUCH, 2).is_none() { return; }
    if matches!(h.state, 0x10 | 10 | 0x12 | 0x3c | 0x11 | 0xe | 0x69) { return; }
    let m = h.back_module();
    let fwd = DT * p(0.9) < h.fwd_speed;
    if m == item::HELI_PACK && heli_owned(h) && shown(h) && fwd {
        h.set_state(c, id::HELI_LONG_JUMP, true);
    } else if m == item::THRUSTER_PACK && shown(h) && fwd && h.f508 == 0 {
        h.set_state(c, id::THRUSTER_LONG_JUMP, true);
    }
}

/// R1 / R2 pressed within 9 ticks in the air (the pass ends after this either way): the Thruster stomp 0x22.
pub(super) fn jump_crouch_press(h: &mut Hero, c: &mut Ctx) {
    if h.back_module() == item::THRUSTER_PACK && shown(h) { h.set_state(c, id::STOMP, true); }
}

/// 10 / 0x10 hitting a wall (not landed, a wall within 0.9 ahead, capsule hit, |vel| > 7·dt, the move kept less
/// than half of it, no ledge, below 2.0) → 0x7a. True when it changed the state.
pub(super) fn pack_jump_wall_hit(h: &mut Hero, c: &mut Ctx) -> bool {
    if h.jump.landed != 0 || !matches!(h.state, 10 | 0x10) || !(h.wall_ahead[0] < 0.9) || h.cap_hit == 0 { return false; }
    let v = len3(h.vel);
    if !(DT * p(7.0) < v) { return false; }
    if !(len3(h.disp) < v * p(0.5)) || h.f838 != 0 || !(h.height < p(2.0)) { return false; }
    h.set_state(c, id::REBOUND, true);
    true
}

/// The Heli long jump 10 once landed: SetState(3, no anim) + skid anim 5 (curve −2), momentum × 0.8.
pub(super) fn heli_long_jump_landed(h: &mut Hero, c: &mut Ctx) {
    if h.set_state(c, 3, false) {
        h.set_anim(c.anim, c.rng, p(-2.0), 5, 0);
        h.momentum = vscale(h.momentum, p(0.8));
    }
}

/// 0x10 after 60 ticks above 1.5 → 6. True when it changed the state.
pub(super) fn thruster_fallover(h: &mut Hero, c: &mut Ctx) -> bool {
    if h.state != id::THRUSTER_LONG_JUMP || !(ticks(60) < h.timer) || !(p(1.5) < h.height) { return false; }
    h.set_state(c, 6, true);
    true
}

/// The long jumps' glide hold-off: 10 / 0x10 after 25 ticks with 0x13f65a set push 0x13f7f6 to 20.
pub(super) fn long_jump_glide_holdoff(h: &mut Hero) {
    if matches!(h.state, 10 | 0x10) && ticks(25) < h.timer && h.f65a != 0 { h.jump.f7f6 = ticks(20) as i16; }
}

/// The jumps' glide test when no double jump was taken: Heli-Pack owned, not the Hydro-Pack on the back, Clank
/// shown, ✕ held, past `min_timer` (20; 8 in 0x10; 17 in 0xe above 4) and 0x13f7f6, above 0.6 (1.0 in 7 / 9, 1.5
/// in 0xe) → 8.
pub(super) fn jump_to_glide(h: &mut Hero, c: &mut Ctx, min_timer: i32) {
    if !heli_owned(h) || h.back_module() == item::HYDRO_PACK || !shown(h) || c.env.pad.held & button::CROSS == 0 { return; }
    if !(min_timer < h.timer) || !((h.jump.f7f6 as i32) < h.timer) { return; }
    let above = match h.state {
        7 | 9 => 1.0,
        0xe => 1.5,
        _ => 0.6,
    };
    if p(above) < h.height { h.set_state(c, id::GLIDE, true); }
}

/// The end of `HeroPdaGadget` 0x240ed8 when it did not change the state: with the Thruster-Pack (not in the hover,
/// not on a magnetic floor with the Magneboots, Clank shown) an R1 double tap within 30 ticks on the ground
/// (groups 0 / 1 / 0xc, or 2 / 4 while grounded) → the hover 0x81; without it the latch 0x14161a is cleared (except
/// in 0x13 / 0x81 / 0x15 / 0x16).
pub(super) fn pda_epilogue(h: &mut Hero, c: &mut Ctx) {
    let pad = c.env.pad;
    let magnet = h.f658 != 0 && h.owned.has(item::MAGNEBOOTS);
    if h.back_module() == item::THRUSTER_PACK && h.state != id::HOVER && !magnet && shown(h) {
        dec_timer(&mut h.packs.tap_on);
        let g = h.group;
        let ground = (0..2).contains(&g) || g == 0xc || ((g == 2 || g == 4) && h.grounded_ticks != 0);
        if h.packs.tap_on != 0 && ground && pad.held & button::R1 != 0 && pad.pressed & button::R1 != 0 {
            h.set_state(c, id::HOVER, true);
        }
        if pad.pressed & button::R1 != 0 && h.state != id::HOVER { h.packs.tap_on = ticks(30); }
    } else if !matches!(h.state, 0x13 | 0x81 | 0x15 | 0x16) {
        h.packs.hover_latch = 0;
    }
}

#[cfg(test)]
mod tests;
