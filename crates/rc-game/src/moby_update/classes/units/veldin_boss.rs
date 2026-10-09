//! U564: class 1422, the boss of Veldin's last arena (level18 `0x2f2bf0`, 1 placed, #875; 50 private functions in the
//! census: the update, its prologue `0x2f5e18` / `0x2f5e80` / `0x2f2b18`, its helpers `0x2f6aa0`, `0x2f6878`,
//! `0x2f6960`, `0x2f6e10`, `0x2f6fa8`, `0x2f7028`, `0x2f70b8`, `0x2f7220`, `0x2f7288`, `0x2f76a0`, its draw callback
//! `0x2f7880`, and the calls it makes into its partner classes, ported with them). The name is descriptive [L]. A
//! flying mech carrying a seat 1423 and its pilot 1249 (created by it and placed at its joints every tick), fought in
//! four arenas (the cuboids +0x250..+0x25c, each with a flight path +0x260..): it waits until Ratchet reaches an arena,
//! flies its path, then circles the arena centre picking attacks (lobbed shells 564, ground rings 624, a lightning
//! beam 983, the rolling mines 568, the hoppers 1906, a charge with the damage aura 628, the divers 1355), losing a
//! seventh of its 350 health per phase; each lost seventh plays a cutaway through the director 644 and moves the
//! fight on; from phase 2 it flies to one of the four pads 583 to recharge under a countdown (586) Ratchet must stop;
//! at 0 health it explodes, Ratchet presses a pad, scenes 3, 4 and 5 run, then the slideshow and the ending movie, and
//! it deletes itself and its riders.
//!
//! **Pvars** (0x440; mode 0x20: +0x00 → the damage record +0x20, +0x0c → the flash record +0x60): +0x70 the target
//! record (`0x274df8`: +0xb0 the moby, +0xb4 the kind), +0xc0 / +0x140 the look-at records (joint lists 7 / 0x13),
//! +0x1c0 the big-head cheat's record. Data: +0x200 the pads' group, +0x204 / +0x208 / +0x20c the floater groups,
//! +0x210 the hoppers' group, +0x214 the divers' group, +0x218 the target region path, +0x21c the countdown 586,
//! +0x220 the director 644, +0x224 the focus camera record, +0x228 / +0x22c the cutaways' aim cuboids, +0x230.. /
//! +0x240.. the falling platforms 1381 (×4 each), +0x250.. the arena centres, +0x260.. the arena paths, +0x270 the
//! arena polygon, +0x274 Giant Clank's cuboid, +0x280.. the beam's three cuboids, +0x294 the checkpoint cuboid, +0x298
//! the finale camera cuboid, +0x29c the mines' group, +0x2a0 Giant Clank's exit cuboid, +0x2b0 the mission. Run time:
//! +0x300 the cutaway's camera point, +0x310 its Euler, +0x320 the beam's point, +0x330 the pad, +0x334 the director,
//! +0x338 the aura, +0x33c the beam, +0x340 the path, +0x344 the arena cuboid, +0x348 the path node, +0x34c the phase,
//! +0x350 the last attack, +0x354 / +0x358 / +0x35c / +0x360 timers, +0x364 a pad was reached, +0x368 the HUD meter
//! handle, +0x36c the meter's value, +0x370 the yaw spring, +0x374 the speed, +0x378 / +0x37c the camera springs,
//! +0x38c the health, +0x390 / +0x394 the damage taken (Ratchet in group 0xf / not), +0x398 the shown group, +0x39c
//! the state after a scene, +0x3a0 / +0x3a4 the seat / pilot, +0x3a8 the beam's cuboid, +0x3ac the pad-press count,
//! +0x3b0 the voice, +0x3b4 / +0x3b8 the glow phase / colour, +0x3bc the lean, +0x3c0 the base point, +0x3d0..+0x3e0
//! the hover bob, +0x3e4 / +0x3e8 the death latch / last ground moby, +0x3ec Ratchet stood on a floater. Port only
//! (past the game's block): +0x440.. the glow points of the draw callback ([`DRAW`]).
//!
//! The level18 words (gp = 0x166c00; gp−0x48d0 = 0x162330 ..): 0x162330 1.0 (the scale factor), 0x162338 24 (the
//! floater search radius), 0x16233c 240 (their arm time), 0x16234c 12 / 0x162350 12 / 0x162354 4 / 0x162358 16 /
//! 0x16235c 8 (the circling's speed ramp), 0x162370 15 / 0x162374 26 (the circling radii), 0x162378 30 / 0x16237c 24
//! (the strafe radii), 0x162384 180 / 0x162388 60 (the cutaway's length / delay), 0x162394 420 / 0x162398 10 (the
//! finale camera's), 0x16239c 85 (the beam's charge), 0x1623a4 2.5 (the big-head scale), 0x1623b0..0x1623e4 (the
//! camera tweak, [`camera_and_meter`]), 0x162408..0x16244c (the sparks, [`sparks`]), 0x162450..0x162474 (the draw).
//! The arena words it shares: 0x162360 (Ratchet entered the arena in Giant Clank's state 4 path), 0x16238c (a reset:
//! no writer on level 18), 0x162390 (the countdown ran out; written by 586), 0x1623a0 (the checkpoint taken; read by
//! 1434 / 1435), 0x1623a8 (the difficulty: Ratchet's deaths off a floater; read by 587).
//!
//! ## Coverage
//!
//! The tables below walk every function line by line; `[L]` marks an inference.
//!
//! **The update** `0x2f2bf0`:
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | `0x2f5e18` | the death latch: +0x3e4 = 0: Ratchet's grounded ticks 0x13f650 ≠ 0 → +0x3e8 = his ground moby (0x13f64c); Ratchet in state 0x77 → +0x3e4 = 1 and, +0x3e8 a floater 587, 0x1623a8 += 1 | [`death_watch`] |
//! | `0x2f5e80` | the hits, the riders, the target, the countdown, the phase change (below) | [`intake`] |
//! | `0x2f2b18` | game mode 2 and the cheat 0x15edb0: the scene actors of class 0x4e1 get `AttachManipulator(·, 0, 0x17ce40)` at 2.5 | [`update`] (`manip::scene_big_head`) |
//! | the pilot exists and its state < 0x80: `0x266098(2.5, pilot, 0, +0x1c0)` | the big-head cheat's manipulator on the pilot | [`update`] (`manip::big_head`) |
//! | drawn and within 32 of the camera (0x1677c0): `0x25c460` (= `0x26f020`), +0x7f = 0x1a | the shadow probe | [`update`] (`shadows::probe_down`) |
//! | states 0..0x1b | (the state table below) | [`update`] |
//! | `0x2f7288` | the camera tweak and the HUD meter | [`camera_and_meter`] |
//! | drawn: joints 8 / 9, dir = unit(8 − 9)·0.1; `0x266140(600000, 0, j8, j8, dir)` (= L01 `0x278810`) | the jet glow (type-23 puffs) | [`update`] (`creature::fx::jet_puffs`) |
//! | phase > 1 → 0x13f928 = the boss | the grind look-at moby | NOT ported (G-HERO-038: no hero reader of 0x13f928; recorded in the arena words) |
//! | position = +0x3c0; +0x3d4 += π·dt; `0x25dcb0(0.75·sin(+0x3d4) + +0x3d8, 4·dt², 4·dt², 4·dt, &+0x3dc, &+0x3e0)`; z += +0x3dc; +0x3d8 = 0 | the hover bob | [`update`] (`turn::spring`) |
//! | `0x264ee0(0.03, 0.3, m, +0xc0, 7)`, `(…, +0x140, 0x13)` (×[0x15ed64] = 1) | the two look-at records | [`update`] (`manip::look`) |
//! | +0x3b4 += 200°·dt; +0x90 = `FastTweenColor(sin·½ + ½, 0x30c8c8c8, 0x1c393939)`; +0x3b8 = it with r, b / 3 | the glow colours | [`update`] |
//! | drawn → `RegisterDrawCallback(0x2f7880)` | the glows | [`update`] (`Callback::UnitGlow`, [`glow_quads`]) |
//!
//! (The state table, the helpers and the partner calls follow in the docs of [`update`] and the helper functions.)

use crate::cinematic;
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, damage, flash, ground, target, turn, DT, DT2};
use crate::moby_update::manip;
use crate::moby_update::scheduler::group_ids;
use crate::moby_update::services::World;

use super::{falling_platform, rolling_mine, veldin_cutaway as director, veldin_diver, veldin_floater as floater, veldin_hopper as hopper, veldin_pads as pads, veldin_shots as shots, GlowQuad};

/// The update in the level18 class table.
pub const UPDATE_FN: u32 = 0x2f_2bf0;
/// The glow draw callback (the port's second row).
pub const DRAW_FN: u32 = 0x2f_7880;
pub const REFERENCE_LEVEL: u32 = 18;
pub const CLASSES: [i16; 1] = [1422];
/// The seat and the pilot it carries.
pub const SEAT: i16 = 0x58f;
pub const PILOT: i16 = 0x4e1;
/// The joint lists the port reads (`FUN_002645a8`).
pub const JOINTS: [i16; 2] = [1422, SEAT];
/// The full health (0x43af0000) and the phase count.
pub const HEALTH: f32 = 350.0;
pub const PHASES: f32 = 7.0;
/// Ratchet's hold, his death fall and the state he presses a pad with.
pub const HELD: i32 = 0x72;
pub const DYING: i32 = 0x77;
pub const PRESS: i32 = 0x22;

/// The arena words (module doc).
pub mod words {
    pub const GIANT_DONE: u32 = 0x16_2360;
    pub const RESET: u32 = 0x16_238c;
    pub const COUNTDOWN_OUT: u32 = 0x16_2390;
    pub const CHECKPOINT: u32 = 0x16_23a0;
    pub const DEATHS: u32 = 0x16_23a8;
    /// 0x13f928 (the hero's grind look-at moby), id + 1.
    pub const LOOK_MOBY: u32 = 0x13_f928;
}

/// Pvar offsets (module doc).
pub mod pv {
    pub const D: usize = 0x20;
    pub const F: usize = 0x60;
    pub const TARGET: usize = 0x70;
    pub const TARGET_MOBY: usize = 0xb0;
    pub const TARGET_KIND: usize = 0xb4;
    pub const LOOK_A: usize = 0xc0;
    pub const LOOK_B: usize = 0x140;
    pub const PADS: usize = 0x200;
    pub const GROUPS: usize = 0x204;
    pub const HOPPERS: usize = 0x210;
    pub const DIVERS: usize = 0x214;
    pub const AREA: usize = 0x218;
    pub const COUNTDOWN: usize = 0x21c;
    pub const DIRECTOR: usize = 0x220;
    pub const FOCUS: usize = 0x224;
    pub const AIM: usize = 0x228;
    pub const PLATS_A: usize = 0x230;
    pub const PLATS_B: usize = 0x240;
    pub const CENTRES: usize = 0x250;
    pub const PATHS: usize = 0x260;
    pub const ARENA: usize = 0x270;
    pub const GIANT: usize = 0x274;
    pub const ZAP: usize = 0x280;
    pub const CHECKPOINT: usize = 0x294;
    pub const FINALE: usize = 0x298;
    pub const MINES: usize = 0x29c;
    pub const CLANK_OUT: usize = 0x2a0;
    pub const MISSION: usize = 0x2b0;
    pub const CUT_FROM: usize = 0x300;
    pub const CUT_EULER: usize = 0x310;
    pub const ZAP_POINT: usize = 0x320;
    pub const PAD: usize = 0x330;
    pub const DIRECTOR_M: usize = 0x334;
    pub const AURA: usize = 0x338;
    pub const BEAM: usize = 0x33c;
    pub const PATH: usize = 0x340;
    pub const CENTRE: usize = 0x344;
    pub const NODE: usize = 0x348;
    pub const PHASE: usize = 0x34c;
    pub const LAST: usize = 0x350;
    pub const T354: usize = 0x354;
    pub const T358: usize = 0x358;
    pub const T35C: usize = 0x35c;
    pub const T360: usize = 0x360;
    pub const PAD_DONE: usize = 0x364;
    pub const METER: usize = 0x368;
    pub const METER_VALUE: usize = 0x36c;
    pub const YAW_V: usize = 0x370;
    pub const SPEED: usize = 0x374;
    pub const CAM_V1: usize = 0x378;
    pub const CAM_V2: usize = 0x37c;
    pub const HEALTH: usize = 0x38c;
    pub const DMG_RIDE: usize = 0x390;
    pub const DMG: usize = 0x394;
    pub const SHOW: usize = 0x398;
    pub const AFTER: usize = 0x39c;
    pub const SEAT: usize = 0x3a0;
    pub const PILOT: usize = 0x3a4;
    pub const ZAP_CUBOID: usize = 0x3a8;
    pub const PRESSES: usize = 0x3ac;
    pub const VOICE: usize = 0x3b0;
    pub const GLOW_PHASE: usize = 0x3b4;
    pub const GLOW: usize = 0x3b8;
    pub const LEAN: usize = 0x3bc;
    pub const BASE: usize = 0x3c0;
    pub const BOB_AMP: usize = 0x3d0;
    pub const BOB_PHASE: usize = 0x3d4;
    pub const BOB_ADD: usize = 0x3d8;
    pub const BOB_Z: usize = 0x3dc;
    pub const BOB_V: usize = 0x3e0;
    pub const DIED: usize = 0x3e4;
    pub const LAST_GROUND: usize = 0x3e8;
    pub const ON_FLOATER: usize = 0x3ec;
    /// The game's block.
    pub const GAME_SIZE: usize = 0x440;
    /// Port only: 12 glow points ([`super::glow_quads`]) and their count.
    pub const GLOWS: usize = 0x440;
    pub const GLOW_N: usize = 0x500;
    pub const SIZE: usize = 0x504;
}

/// The level18 words (module doc).
pub mod k {
    pub const SCALE: f32 = 1.0;
    pub const FLOATER_R: f32 = 24.0;
    pub const FLOATER_ARM: i32 = 240;
    pub const RAMP_FAR: f32 = 12.0;
    pub const RAMP_NEAR: f32 = 12.0;
    pub const SPEED_SLOW: f32 = 4.0;
    pub const RAMP_D_FAR: f32 = 16.0;
    pub const RAMP_D_NEAR: f32 = 8.0;
    pub const CIRCLE_FAST: f32 = 15.0;
    pub const CIRCLE: f32 = 26.0;
    pub const STRAFE: f32 = 30.0;
    pub const STRAFE_CLANK: f32 = 24.0;
    pub const CUT_TICKS: i32 = 180;
    pub const CUT_DELAY: i32 = 60;
    pub const FINALE_TICKS: i32 = 420;
    pub const FINALE_DELAY: i32 = 10;
    pub const BEAM_CHARGE: i32 = 85;
    /// The camera tweak (`0x2f7288`): the distance band, the near / far distances and pivots, the state-0x13 values.
    pub const CAM_RATE: f32 = 8.0;
    pub const CAM_ACCEL: f32 = 8.0;
    pub const CAM_NEAR: f32 = 6.0;
    pub const CAM_FAR: f32 = 16.0;
    pub const CAM_DIST_FAR: f32 = 9.0;
    pub const CAM_PIV_FAR: f32 = 3.0;
    pub const CAM_DIST_NEAR: f32 = 16.0;
    pub const CAM_PIV_NEAR: f32 = 5.0;
    pub const CAM_CHARGE_DIST: f32 = 10.0;
    pub const CAM_CHARGE_PIV: f32 = 1.0;
    pub const CAM_CENTRE_DIST: f32 = 14.0;
    pub const CAM_CENTRE_PIV: f32 = 4.0;
    pub const CAM_SHOT_DIST: f32 = 10.0;
    pub const CAM_SHOT_PIV: f32 = 5.0;
}

const DEG: f32 = 0.017_453_292;
const TAU: f32 = 6.283_185_5;

// ---------------------------------------------------------------------------------------------------------------
// Small reads

fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> {
    usize::try_from(c::pi32(w, id, o) - 1).ok().filter(|&m| w.table.mobys.get(m).is_some_and(|x| x.state < 0xfd))
}
fn set_link(w: &mut World, id: MobyId, o: usize, m: Option<MobyId>) { c::set_pi32(w, id, o, m.map_or(0, |m| m as i32 + 1)); }
/// A pvar moby index (the loader's fixed-up link: the table index).
fn moby_at(w: &World, id: MobyId, o: usize) -> Option<MobyId> {
    usize::try_from(c::pi32(w, id, o)).ok().filter(|&m| m < w.table.mobys.len())
}
fn hero(w: &World) -> c::V { super::hero_pos(w) }
fn base(w: &World, id: MobyId) -> c::V { c::pv4(w, id, pv::BASE) }
fn set_base(w: &mut World, id: MobyId, p: c::V) { c::set_pv4(w, id, pv::BASE, p) }
fn state(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_state(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn phase(w: &World, id: MobyId) -> i32 { c::pi32(w, id, pv::PHASE) }
fn seq_b(w: &World, id: MobyId) -> u8 { w.m(id).anim.seq_b }
fn anim_done(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }

/// A level cuboid's centre (+0x30, four lanes) and Euler (+0x70).
fn cuboid(w: &World, i: i32) -> Option<(c::V, [f32; 3])> {
    let s = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, i)?;
    Some((s.matrix[3], s.euler))
}
fn centre(w: &World, i: i32) -> c::V { cuboid(w, i).map_or([0.0; 4], |c| c.0) }

/// A path's points (`0x1b0eb0[i]` on level 18: count at +0, points from +0x10).
fn path<'a>(w: &'a World<'_>, i: i32) -> Option<&'a Vec<[u32; 4]>> { usize::try_from(i).ok().and_then(|i| w.svc.splines.get(i)) }
fn path_point(w: &World, i: i32, k: i32) -> Option<c::V> {
    let p = path(w, i)?;
    let q = p.get(usize::try_from(k).ok()?)?;
    Some(q.map(f32::from_bits))
}
fn path_len(w: &World, i: i32) -> i32 { path(w, i).map_or(0, |p| p.len() as i32) }
/// The path record's word at +0x10·k (the decomp's `*(path + k·16)`, with point k − 1 at it): `count`'s point is the
/// last.
fn path_last(w: &World, i: i32) -> Option<c::V> { path_point(w, i, path_len(w, i) - 1) }

fn heading(from: c::V, to: c::V) -> f32 { c::atan(to[0] - from[0], to[1] - from[1]) }

/// `0x25e140(target, 2π·dt², 2π·dt², 2π·dt, &yaw, &+0x370)`.
fn turn_to(w: &mut World, id: MobyId, target: f32) {
    turn::turn_toward_pvar(w, id, target, TAU * DT2, TAU * DT2, TAU * DT, pv::YAW_V);
}

/// `fun_00212f90(m, seq, frame, ticks)` guarded by `m+0x53 ≠ seq`.
fn blend(w: &mut World, id: MobyId, seq: u8, frame: i32, n: i32) {
    let t = w.ticks(n);
    c::blend_to(w, id, seq, frame, t);
}
fn blend_raw(w: &mut World, id: MobyId, seq: u8, frame: i32, ticks: i32) { c::blend_to(w, id, seq, frame, ticks); }

/// The collision on (class +0x10) and shown (mode & ~0x41).
fn appear(w: &mut World, id: MobyId) {
    let col = super::class_collision(w, w.m(id).o_class);
    let m = w.mm(id);
    m.mode &= !0x41;
    m.has_collision = col;
}
fn vanish(w: &mut World, id: MobyId) { w.mm(id).mode |= 0x41; }

fn release_voice(w: &mut World, id: MobyId) {
    let v = c::pi32(w, id, pv::VOICE);
    if w.sound_alive(v, id) {
        if v != -1 { w.release_sound(v, id); }
        c::set_pi32(w, id, pv::VOICE, -1);
    }
}

/// `0x238fd8(h, 0)`: the meter's flags cleared (it times out).
fn release_meter(w: &mut World, id: MobyId) {
    if c::pi32(w, id, pv::METER) == -1 { return; }
    let key = meter_key(w, id);
    w.svc.hud.set_flags(crate::hud::Request::boss(key, HEALTH as i32), 0);
    c::set_pi32(w, id, pv::METER, -1);
}
/// The meter's data key: the boss's +0x36c.
pub fn meter_key(w: &World, id: MobyId) -> u32 { crate::hud::calls::pvar_key(w.m(id).spawn_id, id, pv::METER_VALUE) }

fn release_voice_always(w: &mut World, id: MobyId) {
    let v = c::pi32(w, id, pv::VOICE);
    if v != -1 { w.release_sound(v, id); }
    c::set_pi32(w, id, pv::VOICE, -1);
}

// ---------------------------------------------------------------------------------------------------------------
// The prologue

/// `0x2f5e18` (module doc): the death latch and the difficulty word.
pub fn death_watch(w: &mut World, id: MobyId) {
    if c::pi32(w, id, pv::DIED) != 0 { return; }
    // 0x13f650 (Ratchet's grounded ticks) ≠ 0 → +0x3e8 = his ground moby 0x13f64c (none on the world).
    if w.hero.grounded_ticks != 0 {
        let g = w.hero.ground_moby.map_or(0, |g| g as i32 + 1);
        c::set_pi32(w, id, pv::LAST_GROUND, g);
    }
    if w.hero.state == DYING {
        c::set_pi32(w, id, pv::DIED, 1);
        let g = usize::try_from(c::pi32(w, id, pv::LAST_GROUND) - 1).ok().and_then(|g| w.table.mobys.get(g));
        if g.is_some_and(|m| m.o_class == 0x24b) {
            let n = w.svc.units.word(words::DEATHS).wrapping_add(1);
            w.svc.units.set_word(words::DEATHS, n);
        }
    }
}

/// The floor of the current phase: `((7 − phase − 2) / 7)·350`.
fn floor(w: &World, id: MobyId) -> f32 { ((PHASES - phase(w, id) as f32) - 2.0) / PHASES * HEALTH }

/// `0x2f5e80` (the hits, the riders, the target, the countdown and the phase change):
///
/// | address | what it does | ported / not |
/// |---|---|---|
/// | scale = class scale · 1.0; `0x265ff8(0.5, m)`: the damage record's byte +0xa = clamp(trunc(dist(Ratchet, m)·8·0.5) − 1, 0, 255) | the lock-on byte | [`intake`] |
/// | `MobyGetHitMessage(m, 0x330000, 0)`; a hit from its own shells 624 / 564, or from a mine 568 with +0xbc = 0, is dropped | its own shots | [`intake`] |
/// | `0x25c7b8(m, hit, +0x20, 0, &k, &dmg, 0, 4)` (= `0x26f378`); k = 1 → nothing more | the resolver | [`intake`] (`damage::resolve`) |
/// | states 0xc..0x13 or 2 only: attacker class 0xb1 → dmg ·3, 0x131 → dmg ·0.666; Ratchet in group 0xf → +0x390 += dmg, else +0x394 += dmg | | [`intake`] |
/// | dmg ≠ 0: state 2, the hit's flags & 0x80000 → blend 0xe (frame 5, 1 tick); health −= dmg, clamped to the phase floor ((7 − phase − 2)/7·350), crossing it latches the phase change; +0x67 = 0xf0, `0x25f798` (the flash) | | [`intake`] (`flash::start`) |
/// | health 0: the meter released (`0x238fd8(h, 0)`), state 0x1a, blend 0 (`ticks(20)`), the voice released, +0x358 = 0; no phase change | the death | [`intake`] |
/// | +0xa4 = 0xff; +0x36c = trunc(health); `0x25f878` | | [`intake`] (`flash::update`) |
/// | no seat: `CreateMoby(0x58f)`: update 0xff, drawn, blend 1 (0, 0); its draw distance and mode = the boss's \| 0x100; rows = joint list 4's (`0x251a10`, `0x208ac0`), position its point; matrix | the seat | [`intake`] |
/// | no pilot: `CreateMoby(0x4e1)` the same; on the seat's joint list 0 | the pilot | [`intake`] |
/// | `0x262278(512, m, +0x70, 0, 0, path +0x218)` (= `0x274df8`): the target in the arena region; +0xb0 = 0 → Ratchet's moby | | [`intake`] (`target::acquire_in`) |
/// | states 0xc..0x13: `0x2d8050(countdown)` 1 → 0x13, blend 1, +0x358 = `ticks(600)`, +0x364 = 0, +0x354 = `ticks(20)`, +0x338 = the aura (`0x2dc3e0(5.8, m, base)`); −1 → state 7, mode \| 0x41, scene 7, after it 0x19 | the countdown's end | [`intake`] |
/// | (the latch or 0x16238c) and Ratchet's health ≠ 0: 0x16238c → phase 0, the platforms +0x230.. reset (`0x2f1cc8`), 0x16238c = 0 | the reset (0x16238c has no writer: never) | [`intake`] |
/// | phase < 2: state 3, phase + 1, the new path, node 1, the shown group, speed 0; +0x300 = the path's first point pulled to 25 (xy) from the old arena's centre; the cutaway (`0x2dfaa0`, [`cutaway`]) | the next arena | [`intake`] |
/// | phase ≥ 2: state 0x15, the voice released, blend 0 (`ticks(20)`), +0x330 = the pad farthest round from Ratchet (`0x2d6b58`), +0x390 = +0x394 = 0 | to a pad | [`intake`] |
pub fn intake(w: &mut World, id: MobyId) {
    let s = super::class_scale(w, w.m(id).o_class) * k::SCALE;
    w.mm(id).scale = s;
    // 0x265ff8(0.5, m): the damage record's byte +0xa.
    let d = c::dist3(hero(w), c::pos(w, id));
    let b = ((d * 8.0 * 0.5) as i32 - 1).clamp(0, 255);
    c::set_pu8(w, id, pv::D + 0xa, b as u8);
    let floor = floor(w, id);
    let mut hit = w.get_hit(id, 0x33_0000, false);
    if let Some(a) = hit.and_then(|h| h.attacker) {
        let m = w.m(a);
        if m.o_class == 0x270 || m.o_class == 0x234 || (m.o_class == 0x238 && m.cmd == 0) { hit = None; }
    }
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    let mut crossed = false;
    'hit: {
        if res.out5 == 1 { break 'hit; }
        let st = state(w, id);
        if !(0xc..0x14).contains(&st) && st != 2 { break 'hit; }
        let mut dmg = res.damage;
        if let Some(a) = hit.and_then(|h| h.attacker) {
            match w.m(a).o_class {
                0xb1 => dmg *= 3.0,
                0x131 => dmg *= 0.666,
                _ => {}
            }
        }
        let o = if w.hero.group == 0xf { pv::DMG_RIDE } else { pv::DMG };
        let t = c::pf(w, id, o) + dmg;
        c::set_pf(w, id, o, t);
        if dmg == 0.0 { break 'hit; }
        if st == 2 && hit.is_some_and(|h| h.flags & 0x8_0000 != 0) { blend_raw(w, id, 0xe, 5, 1); }
        let hp = c::pf(w, id, pv::HEALTH);
        if floor <= hp && hp - dmg < floor { crossed = true; }
        let left = hp - dmg;
        c::set_pf(w, id, pv::HEALTH, if left < floor { floor } else { left });
        c::set_pu8(w, id, pv::F + 7, 0xf0);
        flash::start(w, id, pv::F);
        if c::pf(w, id, pv::HEALTH) == 0.0 {
            release_meter(w, id);
            set_state(w, id, 0x1a);
            if seq_b(w, id) != 0 { blend(w, id, 0, 0, 20); }
            release_voice(w, id);
            c::set_pi32(w, id, pv::T358, 0);
            crossed = false;
        }
    }
    w.mm(id).hit_slot = 0xff;
    let v = c::pf(w, id, pv::HEALTH) as i32;
    c::set_pi32(w, id, pv::METER_VALUE, v);
    flash::update(w, id, pv::F);
    riders(w, id);
    let area = c::pi32(w, id, pv::AREA);
    let t = target::acquire_in(w, id, 512.0, usize::try_from(area).ok());
    write_target(w, id, &t);
    if c::pi32(w, id, pv::TARGET_MOBY) == 0 {
        let h = w.hero_moby.map_or(0, |h| h as i32 + 1);
        c::set_pi32(w, id, pv::TARGET_MOBY, h);
    }
    if (0xc..0x14).contains(&state(w, id)) {
        if let Some(cd) = moby_at(w, id, pv::COUNTDOWN) {
            match pads::countdown_poll(w, cd) {
                1 => {
                    set_state(w, id, 0x13);
                    blend(w, id, 1, 0, 20);
                    let t = w.ticks(600);
                    c::set_pi32(w, id, pv::T358, t);
                    let t = w.ticks(0x14);
                    c::set_pi32(w, id, pv::PAD_DONE, 0);
                    c::set_pi32(w, id, pv::T354, t);
                    let b = base(w, id);
                    let a = shots::aura_new(w, f32::from_bits(0x40b9_999a), id, b);
                    set_link(w, id, pv::AURA, a);
                }
                -1 => {
                    set_state(w, id, 7);
                    vanish(w, id);
                    cinematic::start_scene(w, 7, false);
                    c::set_pi32(w, id, pv::AFTER, 0x19);
                }
                _ => {}
            }
        }
    }
    let reset = w.svc.units.word(words::RESET) != 0;
    if !(crossed || reset) || w.hero.health == 0 { return; }
    if reset {
        c::set_pi32(w, id, pv::PHASE, 0);
        for k in 0..4 {
            if let Some(p) = moby_at(w, id, pv::PLATS_A + 4 * k) { falling_platform::reset(w, p); }
        }
        w.svc.units.set_word(words::RESET, 0);
    }
    if phase(w, id) < 2 {
        set_state(w, id, 3);
        let old = phase(w, id);
        let ph = old + 1;
        c::set_pi32(w, id, pv::PHASE, ph);
        let p = c::pi32(w, id, pv::PATHS + 4 * ph as usize);
        c::set_pi32(w, id, pv::PATH, p);
        c::set_pi32(w, id, pv::NODE, 1);
        let g = c::pi32(w, id, pv::GROUPS + 4 * ph as usize);
        c::set_pi32(w, id, pv::SHOW, g);
        c::set_pf(w, id, pv::SPEED, 0.0);
        cutaway(w, id, old);
    } else {
        set_state(w, id, 0x15);
        release_voice(w, id);
        if seq_b(w, id) != 0 { blend(w, id, 0, 0, 20); }
        let (b, h) = (base(w, id), hero(w));
        let g = c::pi32(w, id, pv::PADS);
        let pad = pads::pad_farthest_turn(w, b, h, g);
        set_link(w, id, pv::PAD, pad);
        c::set_pf(w, id, pv::DMG, 0.0);
        c::set_pf(w, id, pv::DMG_RIDE, 0.0);
    }
}

/// The phase change's cutaway (`0x2f5e80`'s tail): from a point 20 behind the new path's start (pulled to 25 from the
/// old arena centre), 3 up, looking at it (pitch −10°), to a point 15 from the aim cuboid +0x228[phase − 1] toward the
/// old centre and 15 up, looking at the path's start (pitch 20°); Ratchet placed on the aim cuboid's ground, facing
/// away from the camera; `0x2dfaa0(director, from, to, e_from, e_to, place, 180, 60, 0)`.
fn cutaway(w: &mut World, id: MobyId, old: i32) {
    let path = c::pi32(w, id, pv::PATH);
    let mut p0 = path_point(w, path, 0).unwrap_or([0.0; 4]);
    let oc = centre(w, c::pi32(w, id, pv::CENTRES + 4 * old as usize));
    p0 = c::set_len2(c::sub(p0, oc), 25.0);
    p0[2] = 0.0;
    p0 = c::add(p0, oc);
    c::set_pv4(w, id, pv::CUT_FROM, p0);
    c::set_pv4(w, id, pv::CUT_EULER, [0.0; 4]);
    let yaw = c::atan(oc[0] - p0[0], oc[1] - p0[1]);
    c::set_pf(w, id, pv::CUT_EULER + 8, yaw);
    let mut from = p0;
    from[0] += yaw.cos() * 20.0;
    from[1] += yaw.sin() * 20.0;
    from[2] += 3.0;
    let ph = phase(w, id);
    let aim = c::pi32(w, id, pv::AIM + 4 * (ph - 1).max(0) as usize);
    let mut b = centre(w, aim);
    b[2] += 20.0;
    b[2] = ground::ground(w, b, 0.5, 0).z;
    let mut e = b;
    let mut d = c::sub(centre(w, c::pi32(w, id, pv::CENTRES + 4 * (ph - 1).max(0) as usize)), e);
    d[2] = 0.0;
    let d = c::set_len3(d, -15.0);
    e = c::add(e, d);
    e[2] += 15.0;
    b[3] = c::atan(b[0] - e[0], b[1] - e[1]);
    let e1 = [0.0, f32::from_bits(0xbe32_b8c2), c::atan(p0[0] - from[0], p0[1] - from[1]), 0.0];
    let p = path_point(w, path, 0).unwrap_or([0.0; 4]);
    let e2 = [0.0, f32::from_bits(0x3eb2_b8c2), c::atan(p[0] - e[0], p[1] - e[1]), 0.0];
    if let Some(dm) = link(w, id, pv::DIRECTOR_M) {
        director::start(w, dm, from, e, e1, e2, b, k::CUT_TICKS, k::CUT_DELAY, 0);
    }
}

/// The target record at +0x70 (`0x274df8`'s out: position, Euler, aim, body, moby, kind).
fn write_target(w: &mut World, id: MobyId, t: &target::Target) {
    c::set_pv4(w, id, pv::TARGET, t.pos);
    c::set_pv4(w, id, pv::TARGET + 0x10, t.rot);
    c::set_pv4(w, id, pv::TARGET + 0x20, t.aim);
    c::set_pv4(w, id, pv::TARGET + 0x30, t.body);
    c::set_pi32(w, id, pv::TARGET_MOBY, t.moby.map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, pv::TARGET_KIND, t.kind as i32);
}

/// The seat 0x58f on joint list 4, the pilot 0x4e1 on the seat's list 0 (created on the first call).
fn riders(w: &mut World, id: MobyId) {
    let (dd, md) = (w.m(id).draw_dist, w.m(id).mode);
    for (o, class, parent_is_seat, list) in [(pv::SEAT, SEAT, false, 4usize), (pv::PILOT, PILOT, true, 0usize)] {
        let r = match link(w, id, o) {
            Some(r) => r,
            None => {
                let Some(r) = w.create_moby(class) else { continue };
                set_link(w, id, o, Some(r));
                w.mm(r).update_dist = 0xff;
                w.mm(r).visible = 1;
                w.anim_blend(r, 1, 0, 0);
                r
            }
        };
        let parent = if parent_is_seat { match link(w, id, pv::SEAT) { Some(s) => s, None => continue } } else { id };
        w.mm(r).draw_dist = dd;
        w.mm(r).mode = md | mode::KEEP_ROWS;
        let mtx = w.joint_matrix(parent, list);
        let p = w.joint_point(parent, list);
        let m = w.mm(r);
        m.rows[..3].copy_from_slice(&mtx[..3]);
        m.position = p;
        w.build_matrix(r);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The helpers

/// `0x2f6fa8(m, group)`: every member of the group not deleted (state 0xfe / 0xfd) hidden: collision off, not drawn,
/// mode \| 1.
pub fn hide_group(w: &mut World, group: i32) {
    let Ok(g) = i8::try_from(group) else { return };
    for m in group_ids(w, g) {
        let o = w.mm(m);
        if o.state == 0xfe || o.state == 0xfd { continue; }
        o.has_collision = false;
        o.visible = 0;
        o.mode |= mode::HIDDEN;
    }
}

/// `0x2f7028(m, group)`: every member not deleted shown: drawn, mode & ~1, collision on (class +0x10).
pub fn show_group(w: &mut World, group: i32) {
    let Ok(g) = i8::try_from(group) else { return };
    for m in group_ids(w, g) {
        if w.m(m).state == 0xfe || w.m(m).state == 0xfd { continue; }
        let col = super::class_collision(w, w.m(m).o_class);
        let o = w.mm(m);
        o.visible = 1;
        o.mode &= !mode::HIDDEN;
        o.has_collision = col;
    }
}

/// `0x2f6878(rate, m, &p)`: the base point moves toward `p` (xy; z kept) by a spring on the distance
/// (`0x25dcb0(|d|, 10·dt², 10·dt², rate, &0, &+0x374)`): +0x374 is the step; true when within 0.1 and the step under
/// 0.1.
pub fn move_toward(w: &mut World, id: MobyId, rate: f32, p: c::V) -> bool {
    let b = base(w, id);
    let mut d = c::sub(p, b);
    d[2] = 0.0;
    let l = c::len3(d);
    let (mut x, mut v) = (0.0, c::pf(w, id, pv::SPEED));
    turn::spring(l, 10.0 * DT2, 10.0 * DT2, rate, &mut x, &mut v);
    c::set_pf(w, id, pv::SPEED, v);
    let d = c::set_len3(d, v);
    set_base(w, id, c::add(b, d));
    l < 0.1 && v < 0.1
}

/// `0x2f6aa0(m)`: circling the arena centre (+0x344) 135° round from Ratchet, at most 10° a step, at radius 15
/// (state 2) or 26; the speed 10·dt beyond 27 from the centre, 20·dt with Ratchet in group 0xf, else from Ratchet's
/// distance: 12 → 4 across 8..16 (state 0xd: 5.5 → 2 across 4..8; 0xf: 10 → 6 across 12..24); z eased to the
/// centre's (`Approach(4·dt)`).
pub fn steer(w: &mut World, id: MobyId) {
    let cub = c::pi32(w, id, pv::CENTRE);
    let cen = centre(w, cub);
    let a = heading(cen, hero(w));
    let b = heading(cen, base(w, id));
    let step = if c::sub_rot(b, a) < 0.0 { f32::from_bits(0xc016_cbe4) } else { f32::from_bits(0x4016_cbe4) };
    let t = c::add_rot(a, step);
    let lim = f32::from_bits(0x3e32_b8c2);
    let e = c::sub_rot(t, b).clamp(-lim, lim);
    let ang = c::add_rot(e, b);
    let r = if state(w, id) == 2 { k::CIRCLE_FAST } else { k::CIRCLE };
    let p = [ang.cos() * r + cen[0], ang.sin() * r + cen[1], cen[2], cen[3]];
    let speed = if k::CIRCLE + 1.0 < c::dist2(base(w, id), cen) {
        10.0
    } else if w.hero.group == 0xf {
        20.0
    } else {
        let (mut near, mut far, mut slow, mut fast) = (k::RAMP_D_NEAR, k::RAMP_D_FAR, k::SPEED_SLOW, k::RAMP_FAR);
        match state(w, id) {
            0xd => { far *= 0.5; near *= 0.5; slow = 2.0; fast = 5.5; }
            0xf => { far *= 1.5; near *= 1.5; slow = 6.0; fast = 10.0; }
            _ => {}
        }
        let d = c::dist2(c::pos(w, id), hero(w));
        let d = if far < d { far } else if d < near { near } else { d };
        (fast - slow) * (1.0 - (d - near) / (far - near)) + slow
    };
    move_toward(w, id, speed * DT, p);
    let mut z = c::pf(w, id, pv::BASE + 8);
    turn::approach(cen[2], 4.0 * DT, &mut z);
    c::set_pf(w, id, pv::BASE + 8, z);
}

/// `0x2f6960(m)`: the charge's strafe: toward Ratchet's offset from the arena centre clamped to 30 (24 in group 0xf),
/// at Ratchet's speed (0x13f4b0) + ((distance − 12)/12)·4.75·dt, clamped to [4.75·dt, 20·dt].
pub fn strafe(w: &mut World, id: MobyId) {
    let cen = centre(w, c::pi32(w, id, pv::CENTRE));
    let mut d = c::sub(hero(w), cen);
    d[3] = 0.0;
    d = c::clamp_len3(d, if w.hero.group == 0xf { k::STRAFE_CLANK } else { k::STRAFE });
    let p = c::add(d, cen);
    let dist = c::dist2(c::pos(w, id), p);
    let lo = 4.75 * DT;
    let hi = 20.0 * DT;
    let s = f32::from_bits(w.hero.eff_len.0) + ((dist - 12.0) / 12.0) * lo;
    let s = if hi < s { hi } else if s < lo { lo } else { s };
    move_toward(w, id, s, p);
}

/// `0x2f70b8(m)`: the next attack: `randi(6)` → 0xd, 0xe, 0xf, 0x10, 0x11, 0x13, allowed by the phase's row of the table
/// 0x1dfb40 and not the last one (+0x350); 0x13 only before a pad was reached and with Ratchet within 32 of the arena
/// cuboid +0x25c; 0x11 not after 0xe; up to 100 draws (the last one stands).
pub fn pick_attack(w: &mut World, id: MobyId) -> i32 {
    const TABLE: [[i16; 6]; 6] = [[1, 0, 1, 0, 0, 0], [1, 0, 1, 1, 0, 0], [1, 0, 1, 1, 1, 0], [1, 0, 1, 1, 1, 1], [1, 1, 1, 1, 1, 1], [1, 1, 1, 0, 1, 1]];
    const NEXT: [i32; 6] = [0xd, 0xe, 0xf, 0x10, 0x11, 0x13];
    let last = c::pi32(w, id, pv::LAST);
    let ph = phase(w, id);
    let mut s = last;
    for _ in 0..100 {
        let r = w.rng.randi(6) as usize;
        s = NEXT[r];
        let mut ok = TABLE.get(ph as usize).map_or(0, |t| t[r]);
        if s == last { ok = 0; }
        if s == 0x13 && (c::pi32(w, id, pv::PAD_DONE) != 0 || k::STRAFE + 2.0 < c::dist2(hero(w), centre(w, c::pi32(w, id, pv::CENTRES + 0xc)))) {
            ok = 0;
        }
        if s == 0x11 && last == 0xe { ok = 0; }
        if ok != 0 { break; }
    }
    s
}

/// `0x2f7220(m, rec, from, to)`: the look-at record's yaw target = `sub_rot(atan(to − from), yaw)`.
fn look_toward(w: &mut World, id: MobyId, rec: usize, from: c::V, to: c::V) {
    let a = c::sub_rot(heading(from, to), c::yaw(w, id));
    c::set_pf(w, id, rec + 0x68, a);
}

/// `0x2f6e10(m)`: the floaters 587 of the shown group (+0x398): each polled (`0x2d8858`); those within 24 (xy) of the
/// base marked to fall (`0x2d8888(·, ticks(60), ticks(240))`); of those ahead (`dot(d, row 0) > 0.333`), the nearest
/// (3-D, from 1024).
pub fn floater_ahead(w: &mut World, id: MobyId) -> Option<MobyId> {
    let g = c::pi32(w, id, pv::SHOW);
    let Ok(g) = i8::try_from(g) else { return None };
    let mut best = 1024.0f32;
    let mut out = None;
    let row0 = w.m(id).rows[0];
    for m in group_ids(w, g) {
        if w.m(m).o_class != 0x24b { continue; }
        floater::poll(w, m);
        let b = base(w, id);
        if k::FLOATER_R <= c::dist2(w.m(m).position, b) { continue; }
        let d = c::sub(w.m(m).position, b);
        let (t60, ta) = (w.ticks(0x3c), w.ticks(k::FLOATER_ARM));
        floater::arm(w, m, t60, ta);
        if c::dot3(d, row0) <= 0.333 { continue; }
        let l = c::len3(d);
        if l < best {
            best = l;
            out = Some(m);
        }
    }
    out
}

/// `0x2f76a0(m, p)`: the landing flash: +0xbc counted down (`FastDecTimer` u8); four smoke puffs (`PartType44Spawn`:
/// size 100000, growth 5000, damp 0.98, fall 0, w 0.85, along row 0 at `randf(0.06, 0.12)`, life
/// `ticks(rand_range(20, 40))`, alpha 0x7f, white, spin `trunc(randf_sym(0, 6))`) and one spark (`PartType21Spawn`:
/// size 100000, along row 0 at `randf(0.2, 0.3)`, 0x4f007fff / 0x1fffffff, `ticks(rand_range(20, 40))`, split 1).
pub fn sparks(w: &mut World, id: MobyId, p: c::V) {
    let mut cmd = w.m(id).cmd;
    crate::moby_update::services::fast_dec_timer_u8(&mut cmd);
    w.mm(id).cmd = cmd;
    let rows = w.m(id).rows;
    let dir: c::V = std::array::from_fn(|k| rows[0][k] + if k == 3 { 1.0 } else { 0.0 });
    for _ in 0..4 {
        let l = w.rng.randf(0.06, 0.12);
        let v = c::set_len3(dir, l);
        let spin = w.rng.randf_sym(0.0, 6.0) as i32;
        let n = w.rng.rand_range(20, 40);
        let life = w.ticks(n);
        let s = crate::particles::type44::Spawn { size: 100000.0, growth: 5000.0, damp: 0.98, fall: 0.0, w: 0.85, pos: [p[0], p[1], p[2]], vel: [v[0], v[1], v[2]], life, alpha: 0x7f, rgb: 0xff_ffff, spin };
        crate::moby_update::creature::fx::part44(w, &s);
    }
    let l = w.rng.randf(0.2, 0.3);
    let v = c::set_len3(dir, l);
    let n = w.rng.rand_range(20, 40);
    let life = w.ticks(n);
    crate::moby_update::creature::fx::part21(w, 100000.0, p, v, 0x4f00_7fff, 0x1fff_ffff, life, 1);
}

/// `0x2f7288(m)`: the camera tweak of the class-18 record +0x224 and the HUD meter.
///
/// | address | what it does | ported / not |
/// |---|---|---|
/// | `FastDecTimer(+0x360)`; d = clamp(xy distance Ratchet–base, 6, 16), f = (d − 6)/10: distance 16 + (9 − 16)·f, pivot 5 + (3 − 5)·f | | [`camera_and_meter`] |
/// | state 0x13: (Ratchet in group 0xf or state 0x42), +0x364 = 0, +0x390 ≤ 25 and +0x360 = 0 → 10 / 1; else Ratchet within 28 (xy) of the arena cuboid → 14 / 4 | the charge's views | [`camera_and_meter`] |
/// | states 9, 6 → 10 / 5 | | [`camera_and_meter`] |
/// | states 0, 0xa, 0xb, 1, 0x1b: every camera slot (48): +0x3ec = 0 and Ratchet on a floater 587 → +0x3ec = 1; the first active slot of class 18: `0x306338` (its record's +0x50 = 1), the meter released; state > 1 and (+0x3ec or 0x1623a0) and the follow camera current → `0x313628(10, 0.003, 0)`, `0x313690(5, 0.003)` | | [`camera_and_meter`] (`cinematic::focus_suppress`, `follow_distance`, `follow_pivot_height`; a slot is active when its record exists [L]) |
/// | other states, health > 0: `queue_animation_update(0x16, 0xffff, 0x239360, 0x239480, 0x23cd60, &+0x36c, 350)` → +0x368 | the boss meter (HUD slot 6, flags 0x10) | [`camera_and_meter`] (`hud::Calls::boss_meter`) |
/// | `0x25dcb0(distance, 8·dt², 8·dt², 8·dt, &record +0x34, &+0x378)`, the same for the pivot (+0x38, +0x37c) | the record's springs | [`camera_and_meter`] (`cinematic::focus_record`) |
pub fn camera_and_meter(w: &mut World, id: MobyId) {
    let rec = usize::try_from(c::pi32(w, id, pv::FOCUS)).ok();
    c::dec_timer_pvar_i32(w, id, pv::T360);
    let d = c::dist2(hero(w), base(w, id));
    let d = d.clamp(k::CAM_NEAR, k::CAM_FAR);
    let f = (d - k::CAM_NEAR) / (k::CAM_FAR - k::CAM_NEAR);
    let mut dist = (k::CAM_DIST_FAR - k::CAM_DIST_NEAR) * f + k::CAM_DIST_NEAR;
    let mut piv = (k::CAM_PIV_FAR - k::CAM_PIV_NEAR) * f + k::CAM_PIV_NEAR;
    match state(w, id) {
        0x13 => {
            let close = (w.hero.group == 0xf || w.hero.state == 0x42)
                && c::pi32(w, id, pv::PAD_DONE) == 0
                && c::pf(w, id, pv::DMG_RIDE) <= 25.0
                && c::pi32(w, id, pv::T360) == 0;
            if close {
                dist = k::CAM_CHARGE_DIST;
                piv = k::CAM_CHARGE_PIV;
            } else if c::dist2(hero(w), centre(w, c::pi32(w, id, pv::CENTRE))) < 28.0 {
                dist = k::CAM_CENTRE_DIST;
                piv = k::CAM_CENTRE_PIV;
            }
        }
        9 | 6 => {
            dist = k::CAM_SHOT_DIST;
            piv = k::CAM_SHOT_PIV;
        }
        0 | 0xa | 0xb | 1 | 0x1b => {
            let slots = w.svc.camera_classes.len().min(0x30);
            for i in 0..slots {
                if c::pi32(w, id, pv::ON_FLOATER) == 0 && w.hero.ground_moby.is_some_and(|g| w.m(g).o_class == 0x24b) {
                    c::set_pi32(w, id, pv::ON_FLOATER, 1);
                }
                if w.svc.camera_classes[i] != 0x12 { continue; }
                cinematic::focus_suppress(w, i);
                release_meter(w, id);
                if 1 < state(w, id) && (c::pi32(w, id, pv::ON_FLOATER) != 0 || w.svc.units.word(words::CHECKPOINT) != 0) {
                    let rate = f32::from_bits(0x3b44_9ba6);
                    cinematic::follow_distance(w, k::CAM_SHOT_DIST, rate, false);
                    cinematic::follow_pivot_height(w, k::CAM_SHOT_PIV, rate);
                }
                break;
            }
        }
        _ => {
            if 0.0 < c::pf(w, id, pv::HEALTH) {
                let v = c::pi32(w, id, pv::METER_VALUE);
                w.svc.hud.boss_meter(meter_key(w, id), v, HEALTH as i32);
                c::set_pi32(w, id, pv::METER, 1);
            }
        }
    }
    let Some(r) = rec else { return };
    let [mut x1, mut x2] = w.svc.camera_focus.get(r).copied().unwrap_or([0.0; 2]);
    let (mut v1, mut v2) = (c::pf(w, id, pv::CAM_V1), c::pf(w, id, pv::CAM_V2));
    let (a, vmax) = (k::CAM_ACCEL * DT2, k::CAM_RATE * DT);
    turn::spring(dist, a, a, vmax, &mut x1, &mut v1);
    turn::spring(piv, a, a, vmax, &mut x2, &mut v2);
    c::set_pf(w, id, pv::CAM_V1, v1);
    c::set_pf(w, id, pv::CAM_V2, v2);
    cinematic::focus_record(w, r, x1, x2);
}

// ---------------------------------------------------------------------------------------------------------------
// The update

/// The lobbed shell of states 2 and 0xf (`0x2d5348`): from joint 2 (key frame A < 15, state 2) / 3, along row 0 at
/// 20·dt, the yaw `randf_sym(5°, 30°)`, the pitch −`randf(5°, 10°)`, the flight 3·(xy distance to Ratchet) clamped to
/// [60, 120] ticks (scaled, truncated), at `at`.
fn lob_from(w: &mut World, id: MobyId, joint: usize, at: c::V, kind: i32) -> c::V {
    let p = w.joint_point(id, joint);
    let dir = c::set_len3(w.m(id).rows[0], 20.0 * DT);
    let yaw = w.rng.randf_sym(f32::from_bits(0x3db2_b8c2), f32::from_bits(0x3f06_0a92));
    let pitch = w.rng.randf(f32::from_bits(0x3db2_b8c2), f32::from_bits(0x3e32_b8c2));
    let d = c::dist2(c::pos(w, id), hero(w)) * 3.0;
    let d = d.clamp(60.0, 120.0);
    let n = (d * f32::from_bits(w.svc.timing.timer_scale.0)) as i32;
    shots::lob(w, yaw, -pitch, p, dir, at, n, kind);
    p
}

/// Level18 0x2f2bf0. The states (`switch(state)`; 6 and 0x16..0x19 have no case):
///
/// | state | what it does | ported / not |
/// |---|---|---|
/// | 0 | base = position; state 1; update 0xff, draw 0x200, mode \| 0x41, collision off; the damage record: +0x28 = 3, +0x20 = 500, +0x30 = 4, +0x24 (s16) = 500; +0x334 = the director (+0x220); +0x340 = path +0x26c; +0x398 = +0x204; node 1; meter −1; health 350; bob 0.75; speed 0; the three floater groups hidden (`0x2f6fa8`) | [`update`] |
/// | 1 | Ratchet in the cuboid +0x274 as Giant Clank (0x1413f4 = 2): 7, mode \| 0x41, scene 0 (`0x2983e8`), after it 2, +0x358 = `ticks(600)`, +0x344 = +0x25c, blend 10 (`ticks(10)`) | the Giant Clank round | [`update`] (`cinematic::start_scene`) |
/// | | Ratchet in the arena polygon (+0x270, `0x25ba80`): shown, collision on, 10, +0x340 = path +0x260, base = its first point, yaw at Ratchet, group +0x204 shown, health ·6/7 | the arena | [`update`] |
/// | | within 50 of cuboid +0x250: shown, 0xc, path +0x260, base = its last point, health ·6/7, +0x344 = +0x250; of +0x254: phase 1, health ·5/7, 0xc, path +0x264, last point, +0x344 = +0x254; of +0x258: 0x1623a0 → the pads armed (`0x2d6d50`), the voice released, 0x1b, collision off, mode \| 0x41, phase 7; else phase 2, health ·4/7, 0xc, path +0x268, last point, +0x344 = +0x258, +0x358 = `ticks(360)` | resuming at a later arena | [`update`] |
/// | 2 | turn at Ratchet; circle (`0x2f6aa0`); health ≤ 301: Giant Clank's moby hidden (collision off, mode \| 0x41) when in a body, `0x218928` (leave the body), `HeroTeleport(cuboid +0x2a0, 0, 1)`, `SetMissionDone(+0x2b0)`, the checkpoint (`0x286e08`), group +0x204 shown, scene 1, 7 (after it 10), blend 10, node 0, speed 0, path +0x260, base = its first point, yaw at Ratchet, meter −1 | the Giant Clank round won | [`update`] (`bodies::queue_leave`, `checkpoint::record`) |
/// | | sequence 10: key 1 or 16 passed and the aim point (0x13f410) within 5 (z) of the base: a shell (`0x2d5348`, kind 0) from joint 2 / 3 at it; the animation done and not sequence 10 → blend 10 | | [`update`] ([`lob_from`]) |
/// | 3 | the director gliding (`0x2dfba0` = 2): base = +0x300, rotation = +0x310, 9, blend 7, the shown group +0x398 shown, group +0x204[phase − 1] hidden, a beam 983 at joint 1 (`0x2ea0f0(m, p, 300)`) | | [`update`] |
/// | 4 | Ratchet grounded, not on a floater: 0x162360 = 1, 7 (after it 5), +0x330 = the pad nearest Ratchet (`0x2d6c80`), mode \| 0x41, scene 2, the three floater groups hidden | | [`update`] |
/// | 5 | 0x162390 = 0; the director: from countdown + (6, 0, 5), to countdown + (6·cos 45°, 6·sin 45°, 7), Euler (0, 10°, π) → (0, 30°, 225°), Ratchet at cuboid +0x298 facing its yaw, 420 / 10 ticks, help 1; 0x141414 = 3 (the Thruster-Pack); 8; the countdown started (`0x2d7f68(·, ticks(1800))`) | the countdown's cutaway | [`update`] (`HeroFields::back_request`) |
/// | 7 | game mode 2: scene 2 between ticks 2600 and 2870 → the pad +0x330 armed; scene 3 between ticks 950 and 960 → the pads armed; 0x1623a0 = 0 → the checkpoint at cuboid +0x294, 0x1623a0 = 1 | the scenes | [`update`] (`cinematic::scene`) |
/// | | else: shown, collision on, state = +0x39c; 5 → 0x15f3fc = 1; 0x19 → Ratchet's death (`0x218fc0`) | | [`update`] (`HeroCall::Death`) |
/// | 8 | mode \| 0x41; the director's t ≥ 1 → 0xc, shown, blend 0 (`ticks(10)`) | | [`update`] |
/// | 9 | turn at Ratchet; to the path's first point (`0x2f6878(20, …)`); sequence 7: done → blend 8; else the base 5·dt toward it; t ∈ (0.5, 0.7): blend 9, the beam fired from joint 1 at the arena centre's ground (`0x2ea168(1, …, 1)`); t ≥ 0.729: blend 0, arrived → 10, the four platforms of the phase started (`0x2f1c38`); else the beam aimed (`0x2ea168(min(2t, 1), …, 0)`) | | [`update`] |
/// | 10 | turn at Ratchet; `Approach(12·dt, 20·dt², &speed)`; along the path's nodes; a floater ahead (`0x2f6e10`): key 1 / 16 passed → a shell at it (from base + row 0, 6 up, along row 0 at 10·dt, yaw `randf_sym(30°, 45°)`, pitch −`randf(20°, 30°)`, `ticks(60)`, kind 5); node = count − 2 → blend 0, 0xb | | [`update`] ([`floater_ahead`]) |
/// | 0xb | not at the end: `Approach(10·dt, 20·dt²)` along the nodes; at the end, Ratchet within 50 (xy), grounded, not on a floater: speed 0, base = the last point, +0x344 = +0x250[phase]; phase 2: 0x162360 = 0 → 4, else 0x15 with the pad (`0x2d6b58`); else 0xc | | [`update`] |
/// | 0xc | turn at the target; circle; the animation done: [`pick_attack`]: 0xd (blend 2, +0x354 = 0, +0x358 = `ticks(90·randi(3) + 330)`, +0x35c = `ticks(30·(6 − phase))`, lean `randf(±0.333)`); 0xe → 0x12 (blend 0, the cuboid of +0x280.. farthest round from Ratchet, `ticks(900)`, `ticks(180)`); 0xf (blend 10, 0, `ticks(600)`, `ticks(360)`); 0x10 (blend 6, `ticks(15)`, `ticks(360)`); 0x11 (blend 0xb, `ticks(10)`, `ticks(600)`); 0x13 (blend 1, within 16 → `ticks(90)` else 0, `ticks(600)`, `ticks(600)`, the aura) | the attack choice | [`update`] |
/// | 0xd | turn at the target; circle; the lean while +0x354 < +0x358 + +0x35c; the ring shells' loop (sequences 2 → 4 → 3): key 1 passed → +0x354 = `ticks(90)`, +0x360 = `ticks(300)`, a ring shell 624 from joint 0 along row 0 + row 1·lean at (xy distance to the target)/2 ≤ 30 ·dt (`0x2db938(64, 18·dt, …)`), the sparks (`0x2f76a0`), lean = `randf_sym(0.133, 0.333)`; both timers out → +0x350 = 0xd, 0xc, blend 5 | | [`update`] |
/// | 0xe | turn at +0x320; the look record B at it from joint 0x13; bob −3; done → blend 8; a beam charging (`0x2ea0f0(m, joint 1, ticks(85))`); aimed with 1 − +0x354/90; tracking the target while +0x354 > 15; fired 180 along (xy) at 2 up (`0x2ea168(1, …, 1)`), blend 9, +0x354 = `ticks(120)`; +0x358 out → +0x350 = 0xe, 0xc | the zap | [`update`] |
/// | 0xf | turn at the target; circle; key 1 / 16 passed: a shell (kind 2) at a point 1..10 round the target (1 in 5: the target itself unless kind 2), held within 29 of the centre with Ratchet in group 0xf, on the ground, within 5 (z); the sparks; +0x358 out, or +0x390 > 25 below phase 4 → +0x350 = 0xf, 0xc | | [`update`] |
/// | 0x10 | turn at Ratchet; circle; every `ticks(15)`: a mine (`0x2d5cf8`) from row 0·3 + 3 up at a point 3..29 round the centre on the ground (within 5 z), speed (xy distance)/`ticks(60)`, `trunc(scale(60·randf(18, 20)))` ticks; +0x358 out → +0x350 = 0x10, 0xc | | [`update`] (`rolling_mine::throw`) |
/// | 0x11 | turn at Ratchet; circle; sequence 0xc; every loop (or +0x354 = `ticks(30)`): a hopper 1906 (`0x2fc668`) at row 1·(±4.3) beside the base on the ground (within 1 z), +0x354 = `ticks(60)` when it was 0; ≥ 11 hoppers out and +0x354 below `ticks(60)` → `ticks(1000)`; < 3 and +0x354 > `ticks(60)` → +0x358 = 0; +0x358 out → +0x350 = 0x11, 0xc, blend 0xd | | [`update`] (`veldin_hopper::launch` / `count`; the launch speed's stack point [L]) |
/// | 0x12 | turn at the target record; to cuboid +0x3a8 at 12·dt; arrived → 0xe, blend 7, +0x320 = the target's position | | [`update`] |
/// | 0x13 | turn at Ratchet; +0x354 out → the strafe (`0x2f6960`); the aura follows (`0x2dc458`); voice: `PlayClassSound(0xe, 4)` kept alive; end: +0x358 out (with Ratchet in group 0xf: and +0x35c), Ratchet in group 0xf within 3 of his rail's end, Ratchet in state 0x16 within 12, or Ratchet beyond 35.8 (xy) of cuboid +0x258 → the voice released, 0xc, blend 0, +0x350 = 0x13 | the charge | [`update`] |
/// | 0x14 | every `ticks(20)`: the divers (`0x2f0030(m, +0x214, base + 4 up)`); none left → 0xc | | [`update`] (`veldin_diver::activate`) |
/// | 0x15 | turn at the pad; the pad armed (`0x2d6dc0`); to it at 12·dt; arrived → the countdown (`ticks(ticks(3600 / 2700 / 1800 / 1200 / 900 / 600))` by phase), +0x354 = 0, 0x14 (phase 4) or 0xc, +0x364 = 1, phase + 1; 0x162390 set → 5 | to a pad | [`update`] |
/// | 0x1a | turn at Ratchet; circle; +0x358 + 1; every `ticks(20)`: `SpawnBeamExplosion` (`0x260790`: 0, 0, 1, 2.5, 9, 1, 0; 20 streaks, 6 sparks, 32 puffs) 6 out at the yaw ± `randf(±30°)`, `randf(3, 8)` up; at `ticks(120)`: blend 0xe (5, 1), a big one 3 ahead, 6 up (2.5, 5, 9, 2, 10; 40, 10, 32); sequence A = B = 0xe and key 30 passed → 0x1b, the countdown stopped (`0x2d7fd0`) | the death | [`update`] (`fx::beam_explosion`) |
/// | 0x1b | collision off, mode \| 0x41, & ~0x1000; phase < 7: scene 3, phase 7, the scene's end place = cuboid +0x294 (0x16d270 / 0x16d280, 0x16d2a6 = 1), 7 (after it 0x1b) | | [`update`] (`interact::scene_end_place`) |
/// | | phase 7: Ratchet on a pad 583 in state 0x22 (+0x3ac 0 → 1); else +0x3ac 0 → the pads armed; +0x3ac counting: past `ticks(40)` → scene 4, phase 8, 7 (after it 0x1b) | the last press | [`update`] |
/// | | phase 8: `CameraScript((660.6, 481.4, 112.6), (0, −0.12, −2.76), 1, 0, 0)`, scene 5, phase 9 | | [`update`] |
/// | | phase 9, game mode 0: `EnterSlideshowMode` (`0x299610`), 10; phase 10, game mode 0: `PlayMovieB(11)` (`0x299108`), 11 | the slideshow and the ending | [`update`] (`cinematic::enter_slideshow` / `play_movie_b`: the credits `crate::slideshow`, the movie `mpegs[51]`) |
/// | | phase 11, game mode 0: 0x162360 = 0x1623a0 = 0, `EnterMenuMode(0x21)` (Lombyte `PauseAllSounds`), the seat, the pilot and itself deleted | the end-of-game page | [`update`] (`cinematic::enter_menu_mode`) |
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::GAME_SIZE { return; }
    if w.m(id).pvars.len() < pv::SIZE { w.mm(id).pvars.resize(pv::SIZE, 0); }
    death_watch(w, id);
    intake(w, id);
    let tgt = usize::try_from(c::pi32(w, id, pv::TARGET_MOBY) - 1).ok().filter(|&m| m < w.table.mobys.len());
    // 0x2f2b18: the actors' big-head cheat on the scene actors of class 0x4e1 at 2.5 (gp−0x485c); 0x266098(2.5, pilot, 0,
    // +0x1c0): the enemies' big head on the pilot (+0x3a4) while its state is below 0x80.
    crate::moby_update::manip::scene_big_head(w, &[0x4e1], 0, 2.5);
    if let Some(p) = link(w, id, pv::PILOT).filter(|&p| w.m(p).state < 0x80) { crate::moby_update::manip::big_head(w, 2.5, p, 0, id, 0x1c0); }
    if w.m(id).visible != 0 && c::dist3(c::pos(w, id), w.camera.map(|x| f32::from_bits(x.0))) < 32.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0x1a;
    }
    if !states(w, id, tgt) { return; }
    epilogue(w, id);
}

/// The target moby's position (+0xb0 → +0x10; Ratchet without one).
fn tpos(w: &World, t: Option<MobyId>) -> c::V { t.map_or_else(|| hero(w), |t| w.m(t).position) }

/// The state table (module doc); false when the boss deleted itself.
fn states(w: &mut World, id: MobyId, tgt: Option<MobyId>) -> bool {
    match state(w, id) {
        0 => {
            let p = c::pos(w, id);
            set_base(w, id, p);
            set_state(w, id, 1);
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.draw_dist = 0x200;
            m.mode |= 0x41;
            m.has_collision = false;
            c::set_pu8(w, id, pv::D + 8, 3);
            c::set_pf(w, id, pv::D, 500.0);
            c::set_pf(w, id, pv::D + 0x10, 4.0);
            c::set_pi16(w, id, pv::D + 4, 500);
            let dm = moby_at(w, id, pv::DIRECTOR);
            set_link(w, id, pv::DIRECTOR_M, dm);
            let p = c::pi32(w, id, pv::PATHS + 0xc);
            c::set_pi32(w, id, pv::PATH, p);
            let g = c::pi32(w, id, pv::GROUPS);
            c::set_pi32(w, id, pv::SHOW, g);
            c::set_pi32(w, id, pv::NODE, 1);
            c::set_pi32(w, id, pv::METER, -1);
            c::set_pf(w, id, pv::HEALTH, HEALTH);
            c::set_pf(w, id, pv::BOB_AMP, 0.75);
            c::set_pf(w, id, pv::SPEED, 0.0);
            for k in 0..3 {
                let g = c::pi32(w, id, pv::GROUPS + 4 * k);
                hide_group(w, g);
            }
        }
        1 => wait(w, id),
        2 => clank_round(w, id),
        3 => {
            if link(w, id, pv::DIRECTOR_M).is_some_and(|d| director::phase(w, d) == 2) {
                let p = c::pv4(w, id, pv::CUT_FROM);
                set_base(w, id, p);
                w.mm(id).rotation = c::pv4(w, id, pv::CUT_EULER);
                set_state(w, id, 9);
                blend(w, id, 7, 0, 0x14);
                let g = c::pi32(w, id, pv::SHOW);
                show_group(w, g);
                let ph = phase(w, id);
                let g = c::pi32(w, id, pv::GROUPS + 4 * (ph - 1).max(0) as usize);
                hide_group(w, g);
                let p = w.joint_point(id, 1);
                let b = shots::beam_new(w, id, p, 300);
                set_link(w, id, pv::BEAM, b);
            }
        }
        4 => {
            if w.hero.air_ticks == 0 && !w.hero.ground_moby.is_some_and(|g| w.m(g).o_class == 0x24b) {
                w.svc.units.set_word(words::GIANT_DONE, 1);
                set_state(w, id, 7);
                c::set_pi32(w, id, pv::AFTER, 5);
                let g = c::pi32(w, id, pv::PADS);
                let h = hero(w);
                let pad = pads::pad_nearest(w, h, g);
                set_link(w, id, pv::PAD, pad);
                vanish(w, id);
                cinematic::start_scene(w, 2, false);
                for k in 0..3 {
                    let g = c::pi32(w, id, pv::GROUPS + 4 * k);
                    hide_group(w, g);
                }
            }
        }
        5 => countdown_cutaway(w, id),
        7 => scenes(w, id),
        8 => {
            vanish(w, id);
            let t = link(w, id, pv::DIRECTOR_M).map_or(1.0, |d| director::progress(w, d));
            if 1.0 <= t {
                set_state(w, id, 0xc);
                w.mm(id).mode &= !0x41;
                if seq_b(w, id) != 0 { blend(w, id, 0, 0, 10); }
            }
        }
        9 => to_path(w, id),
        0xa => fly_path(w, id),
        0xb => path_end(w, id),
        0xc => choose(w, id, tgt),
        0xd => rings(w, id, tgt),
        0xe => zap(w, id, tgt),
        0xf => lobs(w, id, tgt),
        0x10 => mines(w, id),
        0x11 => hoppers(w, id),
        0x12 => {
            let t = c::pv4(w, id, pv::TARGET);
            let h = heading(c::pos(w, id), t);
            turn_to(w, id, h);
            let cub = centre(w, c::pi32(w, id, pv::ZAP_CUBOID));
            if move_toward(w, id, 12.0 * DT, cub) {
                set_state(w, id, 0xe);
                blend(w, id, 7, 0, 0x14);
                let p = tpos(w, tgt);
                c::set_pv4(w, id, pv::ZAP_POINT, p);
            }
        }
        0x13 => charge(w, id),
        0x14 => {
            if c::dec_timer_pvar_i32(w, id, pv::T354) != 0 {
                let t = w.ticks(0x14);
                c::set_pi32(w, id, pv::T354, t);
                let mut p = base(w, id);
                p[2] += 4.0;
                let g = c::pi32(w, id, pv::DIVERS) as i8;
                if veldin_diver::activate(w, Some(id), g, p) != 0 { set_state(w, id, 0xc); }
            }
        }
        0x15 => to_pad(w, id),
        0x1a => dying(w, id),
        0x1b => return finale(w, id),
        _ => {}
    }
    true
}

/// State 1: waiting for Ratchet (module doc).
fn wait(w: &mut World, id: MobyId) {
    let h = hero(w);
    let giant = c::pi32(w, id, pv::GIANT);
    if crate::moby_update::triggers::point_in_cuboid(&w.svc.volumes, [h[0], h[1], h[2]], giant) && w.body() == 2 {
        set_state(w, id, 7);
        vanish(w, id);
        cinematic::start_scene(w, 0, false);
        c::set_pi32(w, id, pv::AFTER, 2);
        let t = w.ticks(600);
        c::set_pi32(w, id, pv::T358, t);
        let c25c = c::pi32(w, id, pv::CENTRES + 0xc);
        c::set_pi32(w, id, pv::CENTRE, c25c);
        blend(w, id, 10, 0, 10);
        return;
    }
    let arena = c::pi32(w, id, pv::ARENA);
    if usize::try_from(arena).is_ok_and(|a| crate::moby_update::creature::region::point_in_polygon(w, a, h)) {
        appear(w, id);
        set_state(w, id, 10);
        let p = c::pi32(w, id, pv::PATHS);
        c::set_pi32(w, id, pv::PATH, p);
        let p0 = path_point(w, p, 0).unwrap_or(base(w, id));
        set_base(w, id, p0);
        let y = heading(p0, h);
        c::set_yaw(w, id, y);
        let g = c::pi32(w, id, pv::GROUPS);
        show_group(w, g);
        let hp = c::pf(w, id, pv::HEALTH) * f32::from_bits(0x3f5b_6db7);
        c::set_pf(w, id, pv::HEALTH, hp);
        return;
    }
    let near = |w: &World, o: usize| c::dist3(h, centre(w, c::pi32(w, id, o))) < 50.0;
    let resume = |w: &mut World, ph: Option<i32>, k: f32, path: usize, cub: usize| {
        appear(w, id);
        if let Some(ph) = ph { c::set_pi32(w, id, pv::PHASE, ph); }
        let hp = c::pf(w, id, pv::HEALTH) * k;
        c::set_pf(w, id, pv::HEALTH, hp);
        set_state(w, id, 0xc);
        let p = c::pi32(w, id, path);
        c::set_pi32(w, id, pv::PATH, p);
        if let Some(q) = path_last(w, p) { set_base(w, id, q); }
        let cc = c::pi32(w, id, cub);
        c::set_pi32(w, id, pv::CENTRE, cc);
    };
    if near(w, pv::CENTRES) {
        resume(w, None, f32::from_bits(0x3f5b_6db7), pv::PATHS, pv::CENTRES);
    } else if near(w, pv::CENTRES + 4) {
        resume(w, Some(1), f32::from_bits(0x3f36_db6e), pv::PATHS + 4, pv::CENTRES + 4);
    } else if near(w, pv::CENTRES + 8) {
        if w.svc.units.word(words::CHECKPOINT) != 0 {
            let g = c::pi32(w, id, pv::PADS);
            pads::pads_arm(w, g);
            release_voice(w, id);
            set_state(w, id, 0x1b);
            w.mm(id).has_collision = false;
            vanish(w, id);
            c::set_pi32(w, id, pv::PHASE, 7);
        } else {
            resume(w, Some(2), f32::from_bits(0x3f12_4925), pv::PATHS + 8, pv::CENTRES + 8);
            let t = w.ticks(0x168);
            c::set_pi32(w, id, pv::T358, t);
        }
    }
}

/// State 2: the Giant Clank round (module doc).
fn clank_round(w: &mut World, id: MobyId) {
    let h = heading(c::pos(w, id), hero(w));
    turn_to(w, id, h);
    steer(w, id);
    if c::pf(w, id, pv::HEALTH) <= 301.0 {
        // 0x1413d0 is the body moby while in a body (0x13fdd4).
        if w.body() != 0 {
            if let Some(b) = w.hero.bodies.moby {
                let m = w.mm(b);
                m.has_collision = false;
                m.mode |= 0x41;
            }
        }
        crate::hero::bodies::queue_leave(w);
        let (p, e) = cuboid(w, c::pi32(w, id, pv::CLANK_OUT)).unwrap_or(([0.0; 4], [0.0; 3]));
        cinematic::hero_teleport(w, [p[0], p[1], p[2]], e, 0, true);
        if let Ok(m) = u8::try_from(c::pi32(w, id, pv::MISSION)) { cinematic::set_mission_done(w, m); }
        // 0x286e08(0x13f3d0, 0x13f3e0): the checkpoint at Ratchet's place, which the teleport above has just stored
        // (the port applies the teleport after the moby loop: its target stands for it).
        crate::moby_update::classes::checkpoint::record(w, crate::moby_update::classes::checkpoint::Record { pos: [p[0], p[1], p[2]], rot: e });
        let g = c::pi32(w, id, pv::GROUPS);
        show_group(w, g);
        cinematic::start_scene(w, 1, false);
        vanish(w, id);
        set_state(w, id, 7);
        blend(w, id, 10, 0, 0x14);
        c::set_pi32(w, id, pv::AFTER, 10);
        c::set_pi32(w, id, pv::NODE, 0);
        c::set_pf(w, id, pv::SPEED, 0.0);
        let path = c::pi32(w, id, pv::PATHS);
        c::set_pi32(w, id, pv::PATH, path);
        let p0 = path_point(w, path, 0).unwrap_or(base(w, id));
        set_base(w, id, p0);
        let y = heading(p0, hero(w));
        c::set_yaw(w, id, y);
        c::set_pi32(w, id, pv::METER, -1);
        return;
    }
    if seq_b(w, id) == 10 {
        if ground::passed_frame(w, id, 1.0) || ground::passed_frame(w, id, 16.0) {
            let t = f32::from_bits(w.hero.shadow_point[2].0);
            if (t - c::pf(w, id, pv::BASE + 8)).abs() < 5.0 {
                let at = w.hero.shadow_point.map(|x| f32::from_bits(x.0));
                let j = if (w.m(id).anim.frame_a as f32) < 15.0 { 2 } else { 3 };
                lob_from(w, id, j, at, 0);
            }
        }
    } else if anim_done(w, id) {
        blend(w, id, 10, 0, 0x14);
    }
}

/// State 5: the countdown's cutaway (module doc).
fn countdown_cutaway(w: &mut World, id: MobyId) {
    w.svc.units.set_word(words::COUNTDOWN_OUT, 0);
    let cd = moby_at(w, id, pv::COUNTDOWN);
    let cp = cd.map_or([0.0; 4], |m| w.m(m).position);
    let q = std::f32::consts::FRAC_PI_4;
    let mut from = c::add([0.0f32.cos() * 6.0, 0.0f32.sin() * 6.0, 0.0, 0.0], cp);
    let mut to = c::add([q.cos() * 6.0, q.sin() * 6.0, 0.0, 0.0], cp);
    from[2] += 5.0;
    to[2] += 7.0;
    let e1 = [0.0, 0.174_532_92, std::f32::consts::PI, 0.0];
    let e2 = [0.0, f32::from_bits(0x3f06_0a92), f32::from_bits(0x407b_53d1), 0.0];
    let (mut place, e) = cuboid(w, c::pi32(w, id, pv::FINALE)).unwrap_or(([0.0; 4], [0.0; 3]));
    place[3] = e[2];
    if let Some(d) = link(w, id, pv::DIRECTOR_M) {
        director::start(w, d, from, to, e1, e2, place, k::FINALE_TICKS, k::FINALE_DELAY, 1);
    }
    w.hero_fields_mut().back_request = Some(3);
    set_state(w, id, 8);
    if let Some(cd) = cd {
        let t = w.ticks(0x708);
        pads::countdown_start(w, cd, t);
    }
}

/// State 7: the scenes (module doc).
fn scenes(w: &mut World, id: MobyId) {
    if w.svc.game_mode == 2 {
        let sc = w.svc.cinematic.scene.as_ref().map(|s| (s.id, s.tick));
        match sc {
            Some((2, t)) => {
                if w.ticks(0xa28) <= t && t <= w.ticks(0xb36) {
                    if let Some(p) = link(w, id, pv::PAD) { pads::pad_arm(w, p); }
                }
            }
            Some((3, t)) => {
                if w.ticks(0x3b6) < t && t < w.ticks(0x3c0) {
                    let g = c::pi32(w, id, pv::PADS);
                    pads::pads_arm(w, g);
                }
                if w.svc.units.word(words::CHECKPOINT) == 0 {
                    if let Some((p, e)) = cuboid(w, c::pi32(w, id, pv::CHECKPOINT)) {
                        crate::moby_update::classes::checkpoint::record(w, crate::moby_update::classes::checkpoint::Record { pos: [p[0], p[1], p[2]], rot: e });
                    }
                    w.svc.units.set_word(words::CHECKPOINT, 1);
                }
            }
            _ => {}
        }
        return;
    }
    appear(w, id);
    let s = c::pi32(w, id, pv::AFTER) as u8;
    set_state(w, id, s);
    if s == 5 { cinematic::set_fade(w, 1.0); }
    if s == 0x19 { w.hero_fields_mut().call(crate::moby_update::services::HeroCall::Death); }
}

/// State 9: to the new path's start while the director glides (module doc).
fn to_path(w: &mut World, id: MobyId) {
    let h = heading(c::pos(w, id), hero(w));
    turn_to(w, id, h);
    let path = c::pi32(w, id, pv::PATH);
    let p0 = path_point(w, path, 0).unwrap_or(base(w, id));
    let arrived = move_toward(w, id, 20.0, p0);
    let t = link(w, id, pv::DIRECTOR_M).map_or(1.0, |d| director::progress(w, d));
    if seq_b(w, id) == 7 {
        if anim_done(w, id) { blend(w, id, 8, 0, 10); }
    } else {
        let b = base(w, id);
        let d = c::set_len3(c::sub(p0, b), 5.0 * DT);
        set_base(w, id, c::add(b, d));
    }
    let mut beam = link(w, id, pv::BEAM);
    if 0.5 < t && t < 0.7 {
        if seq_b(w, id) != 9 { w.anim_blend(id, 9, 0, 0); }
        let Some(bm) = beam else { return };
        let p = w.joint_point(id, 1);
        // The game also probes the ground 10 above the centre (`GroundHeight`) and drops the result: the beam goes to
        // the cuboid's centre itself.
        let at = centre(w, c::pi32(w, id, pv::CENTRE));
        shots::beam_aim(w, 1.0, bm, p, at, true);
        set_link(w, id, pv::BEAM, None);
        beam = None;
    } else if 0.729 <= t {
        if seq_b(w, id) != 0 { blend(w, id, 0, 0, 0x14); }
        if arrived { set_state(w, id, 10); }
        let base_o = if phase(w, id) == 1 { pv::PLATS_A } else { pv::PLATS_B };
        for k in 0..4 {
            if let Some(p) = moby_at(w, id, base_o + 4 * k) { falling_platform::start(w, p); }
        }
        beam = link(w, id, pv::BEAM);
    }
    if let Some(bm) = beam {
        let p = w.joint_point(id, 1);
        let tt = (t + t).min(1.0);
        shots::beam_aim(w, tt, bm, p, [0.0; 4], false);
    }
}

/// One step along the path's nodes at the speed +0x374 (states 10 and 0xb).
fn along(w: &mut World, id: MobyId) {
    let path = c::pi32(w, id, pv::PATH);
    let node = c::pi32(w, id, pv::NODE);
    let Some(q) = path_point(w, path, node) else { return };
    let b = base(w, id);
    let mut d = c::sub(q, b);
    let sp = c::pf(w, id, pv::SPEED);
    if c::len3(d) <= sp {
        c::set_pi32(w, id, pv::NODE, node + 1);
    } else {
        d = c::set_len3(d, sp);
    }
    set_base(w, id, c::add(b, d));
}

/// State 10: flying the path (module doc).
fn fly_path(w: &mut World, id: MobyId) {
    let h = heading(c::pos(w, id), hero(w));
    turn_to(w, id, h);
    let mut sp = c::pf(w, id, pv::SPEED);
    turn::approach(12.0 * DT, 20.0 * DT2, &mut sp);
    c::set_pf(w, id, pv::SPEED, sp);
    if sp != 0.0 { along(w, id); }
    if let Some(f) = floater_ahead(w, id) {
        if ground::passed_frame(w, id, 1.0) || ground::passed_frame(w, id, 16.0) {
            let at = w.m(f).position;
            let mut from = c::add(base(w, id), w.m(id).rows[0]);
            from[2] += 6.0;
            let dir = c::set_len3(w.m(id).rows[0], 10.0 * DT);
            let yaw = w.rng.randf_sym(f32::from_bits(0x3f06_0a92), f32::from_bits(0x3f49_0fdb));
            let pitch = w.rng.randf(f32::from_bits(0x3eb2_b8c2), f32::from_bits(0x3f06_0a92));
            let n = w.ticks(0x3c);
            shots::lob(w, yaw, -pitch, from, dir, at, n, 5);
        }
    }
    let path = c::pi32(w, id, pv::PATH);
    if c::pi32(w, id, pv::NODE) == path_len(w, path) - 2 {
        if seq_b(w, id) != 0 { blend(w, id, 0, 0, 0x14); }
        set_state(w, id, 0xb);
    }
}

/// State 0xb: the path's end (module doc).
fn path_end(w: &mut World, id: MobyId) {
    let path = c::pi32(w, id, pv::PATH);
    if c::pi32(w, id, pv::NODE) != path_len(w, path) {
        let mut sp = c::pf(w, id, pv::SPEED);
        turn::approach(10.0 * DT, 20.0 * DT2, &mut sp);
        c::set_pf(w, id, pv::SPEED, sp);
        along(w, id);
        return;
    }
    if 50.0 <= c::dist2(c::pos(w, id), hero(w)) || w.hero.air_ticks != 0 { return; }
    if w.hero.ground_moby.is_some_and(|g| w.m(g).o_class == 0x24b) { return; }
    c::set_pf(w, id, pv::SPEED, 0.0);
    if let Some(q) = path_last(w, path) { set_base(w, id, q); }
    let ph = phase(w, id);
    let cc = c::pi32(w, id, pv::CENTRES + 4 * ph.clamp(0, 3) as usize);
    c::set_pi32(w, id, pv::CENTRE, cc);
    if ph != 2 {
        set_state(w, id, 0xc);
        return;
    }
    if w.svc.units.word(words::GIANT_DONE) == 0 {
        set_state(w, id, 4);
    } else {
        set_state(w, id, 0x15);
        let (b, h) = (base(w, id), hero(w));
        let g = c::pi32(w, id, pv::PADS);
        let pad = pads::pad_farthest_turn(w, b, h, g);
        set_link(w, id, pv::PAD, pad);
    }
}

/// State 0xc: the attack choice (module doc).
fn choose(w: &mut World, id: MobyId, tgt: Option<MobyId>) {
    let h = heading(c::pos(w, id), tpos(w, tgt));
    turn_to(w, id, h);
    steer(w, id);
    if !anim_done(w, id) { return; }
    match pick_attack(w, id) {
        0xd => {
            set_state(w, id, 0xd);
            blend(w, id, 2, 0, 0x14);
            c::set_pi32(w, id, pv::T354, 0);
            let r = w.rng.randi(3);
            let t = w.ticks(r * 0x5a + 0x14a);
            c::set_pi32(w, id, pv::T358, t);
            let t = w.ticks((6 - phase(w, id)) * 0x1e);
            c::set_pi32(w, id, pv::T35C, t);
            let l = w.rng.randf(f32::from_bits(0xbeaa_7efa), f32::from_bits(0x3eaa_7efa));
            c::set_pf(w, id, pv::LEAN, l);
        }
        0xe => {
            set_state(w, id, 0x12);
            if seq_b(w, id) != 0 { blend(w, id, 0, 0, 0x14); }
            let mut best = 0.0f32;
            let (b, h) = (base(w, id), hero(w));
            for k in 0..3 {
                let ci = c::pi32(w, id, pv::ZAP + 4 * k);
                let cp = centre(w, ci);
                let a = c::atan(b[0] - h[0], b[1] - h[1]);
                let bb = c::atan(b[0] - cp[0], b[1] - cp[1]);
                let d = c::diff_rots(a, bb);
                if best < d {
                    c::set_pi32(w, id, pv::ZAP_CUBOID, ci);
                    best = d;
                }
            }
            let t = w.ticks(900);
            c::set_pi32(w, id, pv::T358, t);
            let t = w.ticks(0xb4);
            c::set_pi32(w, id, pv::T354, t);
        }
        0xf => {
            set_state(w, id, 0xf);
            blend(w, id, 10, 0, 0x14);
            c::set_pi32(w, id, pv::T354, 0);
            let t = w.ticks(600);
            c::set_pi32(w, id, pv::T358, t);
            let t = w.ticks(0x168);
            c::set_pi32(w, id, pv::T35C, t);
        }
        0x11 => {
            set_state(w, id, 0x11);
            blend(w, id, 0xb, 0, 0x14);
            let t = w.ticks(10);
            c::set_pi32(w, id, pv::T354, t);
            let t = w.ticks(600);
            c::set_pi32(w, id, pv::T358, t);
        }
        0x13 => {
            set_state(w, id, 0x13);
            blend(w, id, 1, 0, 0x14);
            let t = if c::dist2(c::pos(w, id), hero(w)) < 16.0 { w.ticks(0x5a) } else { 0 };
            c::set_pi32(w, id, pv::T354, t);
            let t = w.ticks(600);
            c::set_pi32(w, id, pv::T358, t);
            let t = w.ticks(600);
            c::set_pi32(w, id, pv::T35C, t);
            let b = base(w, id);
            let a = shots::aura_new(w, f32::from_bits(0x40b9_999a), id, b);
            set_link(w, id, pv::AURA, a);
        }
        _ => {
            set_state(w, id, 0x10);
            blend(w, id, 6, 0, 0x14);
            let t = w.ticks(0xf);
            c::set_pi32(w, id, pv::T354, t);
            let t = w.ticks(0x168);
            c::set_pi32(w, id, pv::T358, t);
        }
    }
}

/// State 0xd: the ring shells (module doc).
fn rings(w: &mut World, id: MobyId, tgt: Option<MobyId>) {
    let h = heading(c::pos(w, id), tpos(w, tgt));
    turn_to(w, id, h);
    steer(w, id);
    if c::pi32(w, id, pv::T354) < c::pi32(w, id, pv::T358) + c::pi32(w, id, pv::T35C) {
        let a = c::atan(1.0, c::pf(w, id, pv::LEAN));
        c::set_pf(w, id, pv::LOOK_A + 0x68, a);
    }
    let fire = if seq_b(w, id) == 2 {
        if anim_done(w, id) { blend(w, id, 4, 0, 0x14); }
        seq_b(w, id) == 4
    } else if c::dec_timer_pvar_i32(w, id, pv::T354) == 0 || c::pi32(w, id, pv::T358) == 0 {
        seq_b(w, id) == 4
    } else {
        if seq_b(w, id) != 4 { blend(w, id, 4, 0, 0x14); }
        seq_b(w, id) == 4
    };
    if fire {
        if ground::passed_frame(w, id, 1.0) {
            let t = w.ticks(0x5a);
            c::set_pi32(w, id, pv::T354, t);
            let t = w.ticks(0x12c);
            c::set_pi32(w, id, pv::T360, t);
            let p = w.joint_point(id, 0);
            let rows = w.m(id).rows;
            let d = c::add(rows[0], c::set_len3(rows[1], c::pf(w, id, pv::LEAN)));
            let dist = c::dist2(p, tpos(w, tgt)) * 0.5;
            let dist = dist.clamp(0.0, 30.0);
            let d = c::set_len3(d, dist * DT);
            shots::ring_new(w, 64.0, 18.0 * DT, p, d, id);
            sparks(w, id, p);
            let l = w.rng.randf_sym(f32::from_bits(0x3e08_3127), f32::from_bits(0x3eaa_7efa));
            c::set_pf(w, id, pv::LEAN, l);
        } else if anim_done(w, id) && seq_b(w, id) != 3 {
            blend(w, id, 3, 0, 0x14);
        }
    }
    if c::dec_timer_pvar_i32(w, id, pv::T358) == 0 || c::dec_timer_pvar_i32(w, id, pv::T35C) == 0 { return; }
    c::set_pi32(w, id, pv::LAST, 0xd);
    set_state(w, id, 0xc);
    blend(w, id, 5, 0, 0x14);
}

/// State 0xe: the beam (module doc).
fn zap(w: &mut World, id: MobyId, tgt: Option<MobyId>) {
    let zp = c::pv4(w, id, pv::ZAP_POINT);
    let h = heading(c::pos(w, id), zp);
    turn_to(w, id, h);
    let p13 = w.joint_point(id, 0x13);
    look_toward(w, id, pv::LOOK_B, p13, zp);
    c::set_pf(w, id, pv::BOB_ADD, -3.0);
    if anim_done(w, id) && seq_b(w, id) != 8 { blend(w, id, 8, 0, 0x14); }
    if c::pi32(w, id, pv::T354) != 0 {
        match link(w, id, pv::BEAM) {
            None => {
                let p = w.joint_point(id, 1);
                let n = w.ticks(k::BEAM_CHARGE);
                let b = shots::beam_new(w, id, p, n);
                set_link(w, id, pv::BEAM, b);
            }
            Some(bm) => {
                if c::dec_timer_pvar_i32(w, id, pv::T354) == 0 {
                    let n = c::pi32(w, id, pv::T354) as f32;
                    let f = 1.0 - n / (90.0 * f32::from_bits(w.svc.timing.timer_scale.0));
                    let f = f.clamp(0.0, 1.0);
                    let p = w.joint_point(id, 1);
                    let zp = c::pv4(w, id, pv::ZAP_POINT);
                    shots::beam_aim(w, f, bm, p, zp, false);
                    if w.ticks(0xf) < c::pi32(w, id, pv::T354) {
                        let tp = tpos(w, tgt);
                        c::set_pv4(w, id, pv::ZAP_POINT, tp);
                    }
                } else {
                    if seq_b(w, id) != 9 { w.anim_blend(id, 9, 0, 0); }
                    let t = w.ticks(0x78);
                    c::set_pi32(w, id, pv::T354, t);
                    let p = w.joint_point(id, 1);
                    let mut d = c::sub(c::pv4(w, id, pv::ZAP_POINT), p);
                    d[2] = 2.0;
                    let d = c::add(c::set_len2(d, 180.0), p);
                    shots::beam_aim(w, 1.0, bm, p, d, true);
                    set_link(w, id, pv::BEAM, None);
                    if c::pi32(w, id, pv::T358) == 0 {
                        c::set_pi32(w, id, pv::LAST, 0xe);
                        set_state(w, id, 0xc);
                    }
                }
            }
        }
    }
    c::dec_timer_pvar_i32(w, id, pv::T358);
}

/// State 0xf: the shells around the target (module doc).
fn lobs(w: &mut World, id: MobyId, tgt: Option<MobyId>) {
    let h = heading(c::pos(w, id), tpos(w, tgt));
    turn_to(w, id, h);
    steer(w, id);
    if ground::passed_frame(w, id, 1.0) || ground::passed_frame(w, id, 16.0) {
        let r = w.rng.randi(5);
        let kind = c::pi32(w, id, pv::TARGET_KIND);
        let mut e = if r == 0 && kind != 2 {
            c::pv4(w, id, pv::TARGET)
        } else {
            let base_pt = if kind == 2 { hero(w) } else { c::pv4(w, id, pv::TARGET) };
            let a = w.rng.rand_angle();
            let l = w.rng.randf(1.0, 10.0);
            c::add([a.cos() * l, a.sin() * l, 0.0, 0.0], base_pt)
        };
        if w.hero.group == 0xf {
            let cen = centre(w, c::pi32(w, id, pv::CENTRE));
            e = c::sub(e, cen);
            if 29.0 < c::len2(e) { e = c::set_len2(e, 29.0); }
            e = c::add(e, cen);
        }
        e[2] = ground::ground(w, e, 0.5, 0).z;
        if (e[2] - c::pf(w, id, pv::BASE + 8)).abs() < 5.0 {
            let j = if ground::passed_frame(w, id, 1.0) { 2 } else { 3 };
            let p = lob_from(w, id, j, e, 2);
            sparks(w, id, p);
        }
    }
    if c::dec_timer_pvar_i32(w, id, pv::T358) == 0 && (c::pf(w, id, pv::DMG_RIDE) <= 25.0 || 4.0 <= phase(w, id) as f32) { return; }
    c::set_pi32(w, id, pv::LAST, 0xf);
    set_state(w, id, 0xc);
}

/// State 0x10: the mines (module doc).
fn mines(w: &mut World, id: MobyId) {
    let h = heading(c::pos(w, id), hero(w));
    turn_to(w, id, h);
    steer(w, id);
    if c::dec_timer_pvar_i32(w, id, pv::T354) != 0 {
        let cen = centre(w, c::pi32(w, id, pv::CENTRE));
        let mut from = c::add(c::set_len3(w.m(id).rows[0], 3.0), c::pos(w, id));
        from[2] += 3.0;
        let a = w.rng.rand_angle();
        let l = w.rng.randf(3.0, 29.0);
        let mut to = c::add([a.cos() * l, a.sin() * l, 0.0, 0.0], cen);
        to[2] += 5.0;
        to[2] = ground::ground(w, to, 0.5, 0).z;
        if (to[2] - c::pf(w, id, pv::BASE + 8)).abs() < 5.0 {
            let d = c::dist2(to, from);
            let n60 = w.ticks(0x3c) as f32;
            let f = w.rng.randf(18.0, 20.0);
            let life = (f * 60.0 * f32::from_bits(w.svc.timing.timer_scale.0)) as i32;
            let g = c::pi32(w, id, pv::MINES);
            rolling_mine::throw(w, d / n60, id, g, from, to, life);
        }
        let t = w.ticks(0xf);
        c::set_pi32(w, id, pv::T354, t);
    }
    if c::dec_timer_pvar_i32(w, id, pv::T358) == 0 { return; }
    c::set_pi32(w, id, pv::LAST, 0x10);
    set_state(w, id, 0xc);
}

/// State 0x11: the hoppers (module doc).
fn hoppers(w: &mut World, id: MobyId) {
    let h = heading(c::pos(w, id), hero(w));
    turn_to(w, id, h);
    steer(w, id);
    let mut go = None;
    if anim_done(w, id) && seq_b(w, id) != 0xc { blend(w, id, 0xc, 0, 0x14); }
    if (seq_b(w, id) == 0xc && c::dec_timer_pvar_i32(w, id, pv::T354) != 0) || c::pi32(w, id, pv::T354) == w.ticks(0x1e) {
        go = Some(c::pi32(w, id, pv::T354));
    }
    if let Some(t) = go {
        let v = if t != 0 { f32::from_bits(0x4089_999a) } else { f32::from_bits(0xc089_999a) };
        let mut e = c::add(c::set_len3(w.m(id).rows[1], v), base(w, id));
        e[2] = ground::ground(w, e, 0.5, 0).z;
        if (e[2] - c::pf(w, id, pv::BASE + 8)).abs() < 1.0 {
            // The second point of this distance is a stack word this state never writes [L: the boss's base].
            let d = c::dist2(e, base(w, id));
            let n60 = w.ticks(0x3c) as f32;
            let g = c::pi32(w, id, pv::HOPPERS);
            if hopper::launch(w, d / n60, g, e, e) && c::pi32(w, id, pv::T354) == 0 {
                let t = w.ticks(0x3c);
                c::set_pi32(w, id, pv::T354, t);
            }
        }
    }
    let n = hopper::count(w, c::pi32(w, id, pv::HOPPERS));
    if 0xb <= n && c::pi32(w, id, pv::T354) < w.ticks(0x3c) {
        let t = w.ticks(1000);
        c::set_pi32(w, id, pv::T354, t);
    } else if n < 3 && w.ticks(0x3c) < c::pi32(w, id, pv::T354) {
        c::set_pi32(w, id, pv::T358, 0);
    }
    if c::dec_timer_pvar_i32(w, id, pv::T358) == 0 { return; }
    c::set_pi32(w, id, pv::LAST, 0x11);
    set_state(w, id, 0xc);
    blend(w, id, 0xd, 0, 0x14);
}

/// State 0x13: the charge with the aura (module doc).
fn charge(w: &mut World, id: MobyId) {
    let h = heading(c::pos(w, id), hero(w));
    turn_to(w, id, h);
    if c::dec_timer_pvar_i32(w, id, pv::T354) != 0 { strafe(w, id); }
    if let Some(a) = link(w, id, pv::AURA) {
        let b = base(w, id);
        shots::aura_follow(w, f32::from_bits(0x40b9_999a), a, b);
    }
    let v = c::pi32(w, id, pv::VOICE);
    if !w.sound_alive(v, id) {
        let s = w.play_sound(0xe, 4, id);
        c::set_pi32(w, id, pv::VOICE, s);
    }
    let mut end = false;
    if c::dec_timer_pvar_i32(w, id, pv::T358) != 0 {
        if w.hero.group == 0xf {
            if c::dec_timer_pvar_i32(w, id, pv::T35C) != 0 { end = true; }
        } else {
            end = true;
        }
    }
    if w.hero.group == 0xf {
        let rail_end = w.hero.boots.rail.and_then(|r| w.svc.volumes.grind_paths.get(r)).and_then(|g| g.points.last().copied());
        if rail_end.is_some_and(|p| c::dist2(hero(w), p) < 3.0) { end = true; }
    }
    if w.hero.state == 0x16 && c::dist2(c::pos(w, id), hero(w)) < 12.0 { end = true; }
    let cub = centre(w, c::pi32(w, id, pv::CENTRES + 8));
    if k::STRAFE + 5.8 < c::dist2(hero(w), cub) { end = true; }
    if end {
        release_voice_always(w, id);
        set_state(w, id, 0xc);
        if seq_b(w, id) != 0 { blend(w, id, 0, 0, 0x14); }
        c::set_pi32(w, id, pv::LAST, 0x13);
    }
}

/// State 0x15: to the pad (module doc).
fn to_pad(w: &mut World, id: MobyId) {
    let Some(pad) = link(w, id, pv::PAD) else {
        if w.svc.units.word(words::COUNTDOWN_OUT) != 0 { set_state(w, id, 5); }
        return;
    };
    let pp = w.m(pad).position;
    let h = heading(c::pos(w, id), pp);
    turn_to(w, id, h);
    pads::pad_arm(w, pad);
    if move_toward(w, id, 12.0 * DT, pp) {
        let n = match phase(w, id) {
            0 => 0xe10,
            1 => 0xa8c,
            2 => 0x708,
            3 => 0x4b0,
            4 => 900,
            _ => 600,
        };
        let t = w.ticks(n);
        let t = w.ticks(t);
        if let Some(cd) = moby_at(w, id, pv::COUNTDOWN) { pads::countdown_start(w, cd, t); }
        c::set_pi32(w, id, pv::T354, 0);
        let s = if phase(w, id) == 4 { 0x14 } else { 0xc };
        set_state(w, id, s);
        c::set_pi32(w, id, pv::PAD_DONE, 1);
        let ph = phase(w, id) + 1;
        c::set_pi32(w, id, pv::PHASE, ph);
    }
    if w.svc.units.word(words::COUNTDOWN_OUT) != 0 { set_state(w, id, 5); }
}

/// The death's explosions (`0x260790`): (flash, flash2, flash distance 9, scale, light; streaks, sparks, 32 puffs).
fn blast(w: &mut World, id: MobyId, p: c::V, big: bool) {
    let b = if big {
        crate::moby_update::creature::fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.5, flash2: 5.0, flash_dist: 9.0, scale: 2.0, light: 10.0, streaks: 0x28, sparks: 10, puffs: 0x20, debris: 0, sound: -1, shake: false }
    } else {
        crate::moby_update::creature::fx::Beam { damage_r: 0.0, damage: 0.0, flash: 1.0, flash2: 2.5, flash_dist: 9.0, scale: 1.0, light: 0.0, streaks: 0x14, sparks: 6, puffs: 0x20, debris: 0, sound: -1, shake: false }
    };
    crate::moby_update::creature::fx::beam_explosion(w, &b, Some(id), p);
}

/// State 0x1a: the death (module doc).
fn dying(w: &mut World, id: MobyId) {
    let h = heading(c::pos(w, id), hero(w));
    turn_to(w, id, h);
    steer(w, id);
    let n = c::pi32(w, id, pv::T358) + 1;
    c::set_pi32(w, id, pv::T358, n);
    let t20 = w.ticks(0x14);
    if t20 != 0 && n % t20 == 0 {
        let d = w.rng.randf(-30.0, 30.0);
        let a = c::add_rot(c::yaw(w, id), d * DEG);
        let z = w.rng.randf(3.0, 8.0);
        let p = c::add([a.cos() * 6.0, a.sin() * 6.0, z, 0.0], base(w, id));
        blast(w, id, p, false);
    }
    if n == w.ticks(0x78) {
        blend_raw(w, id, 0xe, 5, 1);
        let y = c::yaw(w, id);
        let p = c::add([y.cos() * 3.0, y.sin() * 3.0, 6.0, 0.0], base(w, id));
        blast(w, id, p, true);
    }
    let a = w.m(id).anim;
    if a.seq_a == a.seq_b && a.seq_a == 0xe && ground::passed_frame(w, id, 30.0) {
        set_state(w, id, 0x1b);
        if let Some(cd) = moby_at(w, id, pv::COUNTDOWN) { pads::countdown_stop(w, cd); }
    }
}

/// State 0x1b: the finale (module doc); false once the boss deleted itself.
fn finale(w: &mut World, id: MobyId) -> bool {
    {
        let m = w.mm(id);
        m.has_collision = false;
        m.mode = (m.mode & !mode::TARGETABLE) | 0x41;
    }
    match phase(w, id) {
        p if p < 7 => {
            cinematic::start_scene(w, 3, false);
            c::set_pi32(w, id, pv::PHASE, 7);
            if let Some((cp, e)) = cuboid(w, c::pi32(w, id, pv::CHECKPOINT)) {
                w.svc.interact.scene_end_place = Some(([cp[0], cp[1], cp[2]], e[2]));
            }
            set_state(w, id, 7);
            c::set_pi32(w, id, pv::AFTER, 0x1b);
        }
        7 => {
            let on_pad = w.hero.air_ticks == 0 && w.hero.ground_moby.is_some_and(|g| w.m(g).o_class == pads::PAD) && w.hero.state == PRESS;
            let n = c::pi32(w, id, pv::PRESSES);
            if n == 0 {
                if on_pad {
                    c::set_pi32(w, id, pv::PRESSES, 1);
                } else {
                    let g = c::pi32(w, id, pv::PADS);
                    pads::pads_arm(w, g);
                }
            }
            let n = c::pi32(w, id, pv::PRESSES);
            if n != 0 {
                c::set_pi32(w, id, pv::PRESSES, n + 1);
                if w.ticks(0x28) < n {
                    cinematic::start_scene(w, 4, false);
                    c::set_pi32(w, id, pv::PHASE, 8);
                    set_state(w, id, 7);
                    c::set_pi32(w, id, pv::AFTER, 0x1b);
                }
            }
        }
        8 => {
            let e = [0.0, f32::from_bits(0xbdf5_c28f), f32::from_bits(0xc030_a3d7)];
            let p = [f32::from_bits(0x4425_2666), f32::from_bits(0x43f0_b333), f32::from_bits(0x42e1_3333)];
            cinematic::camera_script(w, p, e, 1, 0, false);
            cinematic::start_scene(w, 5, false);
            c::set_pi32(w, id, pv::PHASE, 9);
        }
        9 if w.svc.game_mode == 0 => {
            cinematic::enter_slideshow(w);
            c::set_pi32(w, id, pv::PHASE, 10);
        }
        10 if w.svc.game_mode == 0 => {
            cinematic::play_movie_b(w, 0xb);
            c::set_pi32(w, id, pv::PHASE, 11);
        }
        11 if w.svc.game_mode == 0 => {
            w.svc.units.set_word(words::GIANT_DONE, 0);
            w.svc.units.set_word(words::CHECKPOINT, 0);
            cinematic::enter_menu_mode(w, 0x21);
            if let Some(s) = link(w, id, pv::SEAT) { w.delete_moby(s); }
            if let Some(p) = link(w, id, pv::PILOT) { w.delete_moby(p); }
            w.delete_moby(id);
            return false;
        }
        _ => {}
    }
    true
}

/// The update's tail after the state table (module doc).
fn epilogue(w: &mut World, id: MobyId) {
    camera_and_meter(w, id);
    // The meter's data word, every tick (the game's element reads +0x36c live).
    let v = c::pi32(w, id, pv::METER_VALUE);
    w.svc.hud.data(meter_key(w, id), v);
    if w.m(id).visible != 0 {
        let j8 = w.joint_point(id, 8);
        let j9 = w.joint_point(id, 9);
        let d = c::set_len3(c::sub(j8, j9), 0.1);
        crate::moby_update::creature::fx::jet_puffs(w, 600000.0, 0.0, j8, j8, d);
    }
    if 1 < phase(w, id) { w.svc.units.set_word(words::LOOK_MOBY, id as u32 + 1); }
    let b = base(w, id);
    c::set_pos(w, id, b);
    let ph = c::add_rot(c::pf(w, id, pv::BOB_PHASE), DT * std::f32::consts::PI);
    c::set_pf(w, id, pv::BOB_PHASE, ph);
    let target = c::pf(w, id, pv::BOB_AMP) * ph.sin() + c::pf(w, id, pv::BOB_ADD);
    let (mut z, mut v) = (c::pf(w, id, pv::BOB_Z), c::pf(w, id, pv::BOB_V));
    turn::spring(target, 4.0 * DT2, 4.0 * DT2, 4.0 * DT, &mut z, &mut v);
    c::set_pf(w, id, pv::BOB_Z, z);
    c::set_pf(w, id, pv::BOB_V, v);
    w.mm(id).position[2] += z;
    c::set_pf(w, id, pv::BOB_ADD, 0.0);
    manip::look(w, id, id, pv::LOOK_A, 7, 0.03, 0.3);
    manip::look(w, id, id, pv::LOOK_B, 0x13, 0.03, 0.3);
    let gp = c::add_rot(c::pf(w, id, pv::GLOW_PHASE), DT * f32::from_bits(0x405f_66f3));
    c::set_pf(w, id, pv::GLOW_PHASE, gp);
    let col = crate::hud::tween_color(gp.sin() * 0.5 + 0.5, 0x30c8_c8c8, 0x1c39_3939);
    w.mm(id).glow = col;
    let third = (col & 0xff00_0000) | ((((col >> 16) & 0xff) / 3) << 16) | (col & 0xff00) | ((col & 0xff) / 3);
    c::set_pi32(w, id, pv::GLOW, third as i32);
    if w.m(id).visible != 0 {
        glow_points(w, id);
        w.svc.units.set_word(super::veldin_diver::COUNTER_WORD, w.counter as u32);
        if let Some(r) = super::row(REFERENCE_LEVEL, DRAW_FN) { w.svc.draw_callbacks.register(Callback::UnitGlow(r), id); }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The draw callback `0x2f7880`

/// The draw's level words: gp−0x47b0 0.25 / −0x47ac −0.15 (the seat's glows' size / pull), gp−0x47a8 / −0x47a4
/// 0x80404080 / 0x20004080 (their colours), gp−0x47a0 60 (their pulse, ticks), gp−0x479c 1.0 / −0x4798 0 (the arm glows'
/// size / pull), gp−0x4794 / −0x4790 0x80006080 / 0x80004080, gp−0x478c 30.
pub const DRAW: [f32; 4] = [0.25, -0.15, 1.0, 0.0];
pub const DRAW_RGBA: [u32; 4] = [0x8040_4080, 0x2000_4080, 0x8000_6080, 0x8000_4080];
pub const DRAW_TICKS: [i32; 2] = [60, 30];

/// The draw's points, taken in the update (the port's draw callbacks have no joint poses): the seat's joints 1 and 2,
/// the boss's 5 and 6 (sequence B 0xb..0xd only), the boss's 0xb..0x12 (module doc; one tick behind the game's
/// draw-time pose [L]).
fn glow_points(w: &mut World, id: MobyId) {
    let mut pts: Vec<c::V> = Vec::with_capacity(12);
    if let Some(s) = link(w, id, pv::SEAT) {
        pts.push(w.joint_point(s, 1));
        pts.push(w.joint_point(s, 2));
    } else {
        pts.push([0.0; 4]);
        pts.push([0.0; 4]);
    }
    let arms = (0xb..0xe).contains(&seq_b(w, id));
    if arms {
        pts.push(w.joint_point(id, 5));
        pts.push(w.joint_point(id, 6));
    }
    for j in 0xb..0x13 { pts.push(w.joint_point(id, j)); }
    for (k, p) in pts.iter().enumerate() { c::set_pv4(w, id, pv::GLOWS + 0x10 * k, *p); }
    c::set_pi32(w, id, pv::GLOW_N, if arms { 1 } else { 0 });
}

/// `0x2f7880` (draw only): the seat's two glows (0.25, pull −0.15, the 60-tick pulse
/// `FastTweenColor(sin(2π·(frame % 60)/60 − π)·½ + ½, 0x80404080, 0x20004080)`), the arms' two (sequence B 0xb..0xd:
/// 1.0, 0, the 30-tick pulse of 0x80006080 / 0x80004080) and eight (0.7, 0.5, +0x3b8) at joints 0xb..0x12.
pub fn glow_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, id: MobyId) -> Vec<GlowQuad> {
    let Some(m) = table.mobys.get(id).filter(|m| m.o_class == CLASSES[0] && m.pvars.len() >= pv::SIZE) else { return Vec::new() };
    let p = &m.pvars;
    let at = |k: usize| { let v = crate::moby_update::services::pvar::v4f(p, pv::GLOWS + 0x10 * k); [v[0], v[1], v[2]] };
    let frame = svc.units.word(super::veldin_diver::COUNTER_WORD) as i64;
    let pulse = |n: i32, a: u32, b: u32| {
        let n = svc.ticks(n).max(1) as i64;
        let f = (frame.rem_euclid(n)) as f32 / n as f32;
        let s = (f * 6.18318 - f32::from_bits(0x4049_0fd0)).sin();
        crate::hud::tween_color(s * 0.5 + 0.5, a, b)
    };
    let mut out = Vec::new();
    let c1 = pulse(DRAW_TICKS[0], DRAW_RGBA[0], DRAW_RGBA[1]);
    for k in 0..2 { out.push(GlowQuad { size: DRAW[0], pull: DRAW[1], point: at(k), rgba: c1 }); }
    let arms = crate::moby_update::services::pvar::i32(p, pv::GLOW_N) != 0;
    let mut k = 2;
    if arms {
        let c2 = pulse(DRAW_TICKS[1], DRAW_RGBA[2], DRAW_RGBA[3]);
        for _ in 0..2 {
            out.push(GlowQuad { size: DRAW[2], pull: DRAW[3], point: at(k), rgba: c2 });
            k += 1;
        }
    }
    let rgba = crate::moby_update::services::pvar::u32(p, pv::GLOW);
    for _ in 0..8 {
        out.push(GlowQuad { size: f32::from_bits(0x3f33_3333), pull: 0.5, point: at(k), rgba });
        k += 1;
    }
    out
}
