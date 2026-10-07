//! U139 (census 2026-10-02): class 631, Kerwan's Blarg hover ship with a gun trooper 574 riding it (level03 0x2cca00,
//! the only copy: 1 placed), its hit handler 0x2ce238, and the flame stream 848 it fires (level03 0x2d47c0, spawned
//! by 0x2d5208; Quartu's class 233 fires it too, so the stream also serves that owner). Read from the level03 decomp
//! and disassembly. Native `f32`; the `rand` draws in the game's order.
//!
//! The ship waits until Ratchet enters its arrival cuboid, flies in along its arrival path, then patrols path A (or
//! B): it always faces Ratchet, aims its turret (two manipulators on joint lists 0 / 1) at him or at its live flame,
//! keeps its distance along the path's nodes, and inside its range cuboid sprays a flame stream at him (every 360
//! ticks, sooner while alerted with no flame alive). When Ratchet (the target) enters the path's exit cuboid it pauses
//! and flies the exit path, switching between A and B at its end. It banks into its moves and bobs. Shot down (health
//! 3), it knocks its rider off with a hit, explodes, breaks into the pieces 1591–1593 and drops its bolts.
//!
//! **Ship pvars** (0x260): +0x20 the damage record (health 3, +0x24 the meter, +0x28 2, +0x29 0, +0x2e 1 alive), +0x38
//! the lure, +0x58 / +0x59 / +0x5a 0x19 / 1 / 6, +0x60 the flash record, +0x120 / +0x150 the patrol records A / B
//! (cursor; +0x10 the spline: the game's pointer, the port's index; +0x14 the path id), +0x180 / +0x1c0 the turret
//! manipulators, +0x200 the yaw velocity, +0x204 s16 the fire timer, +0x206 s16 the pause, +0x208 the range cuboid,
//! +0x20c / +0x244 the exit cuboids A / B, +0x210 the bob phase, +0x214 the speed, +0x218 / +0x21c the roll / pitch
//! velocities, +0x220 / +0x228 the turret yaw / pitch (+0x224 / +0x22c their velocities), +0x230 / +0x234 the exit
//! paths A / B, +0x238 the rider (moby index, −1 none), +0x23c the path (0 A, 1 B), +0x240 the live flame (the port:
//! index + 1), +0x248 the arrival cuboid, +0x24c the arrival path, +0x250 the flame's voice, +0x254 s32 the alert.
//! **Flame pvars** (0x80): +0x00 the velocity (w the target height), +0x10 the owner (index + 1), +0x14 s32 the life,
//! +0x18 the side.
//!
//! **The ship** `0x2cca00`:
//!
//! | address | what | port |
//! |---|---|---|
//! | prologue | the target (`0x24e830` = L01 0x274b78, range 80 while alerted, else 40) | [`update`] (`target::acquire`) |
//! | | this visit's death bit (0x1ba5d0), no path A, or (state ≠ 0 and +0x2e ≠ 1) → the rider and the ship `DeleteMoby` | [`update`] |
//! | | the lure +0x38 → 0, the alert `ticks(240)`; `FastDecTimer` (int) on the alert | [`update`] |
//! | | the rider (class 574, state < 0x80): `0x251d70(2.5, rider +0x150)` (big head), `0x250db8(0.03, 0.3, rider, +0x150, 2)` (the look; ×0x15ed64 = 1) | [`update`] (`manip`) |
//! | | the hits 0x2ce238 | [`hits`] |
//! | 0 init | +0x7f 0x20; health 3, meter 3, +0x28 2, +0x58..5a, +0x2e 1, +0x29 0; the splines of A / B; at A's point 0; speed, roll / pitch velocities, path, 0; cursor A = 1; manipulators lists 0 / 1; voice −1, flame none; turret yaw = yaw, pitch = Euler y; draw distance ≥ 0x80; the rider: `MobyAnimBlend(8, 0, 0)`, update distance 0, no collision, mode &= ~0x41 | [`init`] |
//! | | the mission done, no arrival path or no arrival cuboid → 2; else the arrival path's w = the chord to the next (but the last two), at its point 0, → 1 | [`init`] |
//! | 1 arrival | until Ratchet is in the arrival cuboid (then −1): nothing | [`update`] |
//! | | `SplineSample(1000, 5, 0, path, pos)` (0x24c9b0 = L01 0x272e28): along = seg·w₀ + t + 2; braking distance speed² / (2·10dt²) ≥ the distance to the last point → speed − 10dt² (at least 0.1·dt), else + 10dt² (at most 20·dt) | [`follow`] |
//! | | the point at along (w₀ steps); past the end: stopped → A's point 0, cursor A 1, `cmd` 2, 4 for `ticks(15)`; moving → the last segment's end | [`follow`] |
//! | | yaw `SpringTurn(0.01, 0.3, 0.1)` (0x246c80 = L01 0x26cef0) toward the next point (the last 8: toward the target); move toward the point by at most the speed; [`bank`] (K 20) | [`follow`] |
//! | 2 patrol | yaw toward the target; the turret aims at the live flame (848, alive) or at the target (then the flame cleared): v = aim − joint 0 in the ship's frame (the rows' transpose); turret yaw `SpringTurn(atan v, 2π·dt², 4π·dt², 2π·dt)`, pitch `SpringTurn(−atan(\|v.xy\|, v.z), π·dt², 2π·dt², π/2·dt)`, both on axis 2 | [`patrol`], [`aim`] |
//! | | the node n (1 ≤ n < count − 1): within 20 of the target, 10 in height, and nearer it than node n + 1 → the cursor + 1 (next tick; n < count − 2) | [`patrol`] |
//! | | v = node n − pos; \|v\| under the braking distance (4·dt²), or the node nearer the target than the ship → speed − 4dt² (≥ 0); else + 4dt² (≤ 10·dt); move along v by the speed (away from the node when it is nearer the target); [`bank`] (K 10) | [`patrol`] |
//! | | the target in the path's exit cuboid → (the exit path's w written) `cmd` 3, 4 for `ticks(15)` | [`patrol`] |
//! | | the target in the range cuboid: the fire timer out, or a flame-less alert → timer `ticks(360)`; v = (target − joint 0) set to 6·dt in xy, z 0; flame = 0x2d5208(target z, v, joint 0, ship, `ticks(240)`) | [`patrol`] ([`spawn_flame`]) |
//! | 3 exit | no exit path: z `Approach`es 200 by 8·dt; pitch and roll spring to 0 | [`update`] |
//! | | else as 1 on the exit path of the current side, facing the target; at its end stopped: the side flips, at that side's point 0, its cursor 1, `cmd` 2, 4 for `ticks(15)` | [`follow`] |
//! | 4 pause | the pause running: speed − 4dt² (≥ 0), move back along the yaw by it, [`bank`] (K 10); out → `cmd` | [`update`] |
//! | 5 destroyed | `rand_angle` (four points about joint 4 that nothing reads); the rider: +0x262 = 2, update distance 0xff, mode &= ~6, the hit `0x2484f0` (dir unit xy (rider − ship), z 1, w 5627.925; flags 0x830000, type 3 / 3, damage 20, +0x20 1); `SpawnBeamExplosion(0, 0, 4, 2, 99840, 3, 15, …, 20, 3, 4, sound 2, shake, 1)`; `SetDeathBits(0, −1)`; `BreakFxB` 1591..1593; the manipulators detached; the voice released; `DeleteMoby` | [`destroyed`] |
//! | tail | drawn and within 40 of the camera: the shadow probe (0x248ba8 = L01 0x26f020) | [`update`] |
//! | | the bob: z −= 0.17·cos φ; φ += 2π / (2·60·scale); z += 0.17·cos φ | [`update`] |
//! | | the rider: the ship's position and Euler, `MobyAnimAdvance`, `MobyBuildMatrix`, mode \|= 6 | [`update`] |
//!
//! **[`bank`]** (`0x2cdd0c`): position += the move; a = atan(move) − yaw; pitch `SpringTurn(speed·20°·cos a / (dt·K),
//! π/6·dt², π/3·dt², π/4·dt)`, roll `SpringTurn(−speed·20°·sin a / (dt·K), π/3·dt², 2π/3·dt², π/2·dt)`.
//!
//! **The hits** `0x2ce238`: `MobyGetHitMessage(0x330000)`, the resolver (column 4); out ≠ 1 and not 5: health −=
//! damage; reaction 1 or health ≤ 0 → 5; else not 0xb: red 0xb4, the flash; a push with w 5627.925 outside state 4:
//! a = atan(position − hit point) − yaw; roll −20°·sin a and pitch 5°·cos a when larger than the current (their
//! velocities 0); speed 4·dt; pause `ticks(60)`; `cmd` = the state, 4; class sound 1. Then +0xa4 = 0xff, the flash.
//!
//! **The trap**: at the end of the arrival / exit path, stopped, with no path to go on (no path A; no exit path B or
//! path B), the game executes `teq zero, zero` (`0x1f8970`). Kerwan's ship has every path; the port does nothing there.
//!
//! **The flame** `0x2d47c0`:
//!
//! | address | what | port |
//! |---|---|---|
//! | | the target (range 20); the owner's voice word (631: +0x250; 233: +0x1b8, and 233 in state 0x40 or 1 → 2) | [`flame_update`] |
//! | 2 | the voice released (when the owner holds it), −1; `DeleteMoby` | [`flame_update`] |
//! | 1 | the template: dir unit xy (velocity), z 1, w 5627.925; attacker the flame; flags 0x10003; type 1 / 1; damage 2 (`0x2484c0` = L01 0x26e808) | [`flame_update`] |
//! | | vz = −(z − target z) / 10, within ±8·dt; heading within 90° of the target: `0x270ac0(to target, 90°·dt)` (gp−0x50d0) and the xy velocity turned to it | [`flame_update`] |
//! | | position += velocity; d = its distance to the owner; d ≤ owner z − target z → the voice released; else not alive → class sound 0 (flags 4) on the owner | [`flame_update`] |
//! | | d > owner z − target z, every `ticks(6)` of the life: the side flips; o = (cos, sin)(heading ± 90°)·0.25; j = the owner's joint 3 − side; v = (position + o) − j; end = j + 2v | [`flame_update`] |
//! | | five type-27 flames (49920, `0x5f2f4f6f`, `rand_range(ticks(5), ticks(10))`): r = `randf(−1, 1)`³ set to 0.65·\|v\|, + v, set to `randf(5, 15)·dt`; one (399360, `0x2f4f7f7f`, `rand_range(ticks(4), ticks(7))`) along v at 0.5·dt | [`flame_update`] (`fx::part27`) |
//! | | the tracer (type 49, 0x260130): head h = (position + velocity·6·scale − o); its tail 2 behind j toward h, the head j; velocity toward h at 100·dt; `ticks(45)` | [`flame_update`] (`particles::type49`) |
//! | | `CollLine_Fix(j, end, 0, owner)`: a hit → j = the point; ten sparks (29952, `0x7f2f4f6f`, `rand_range(ticks(15), ticks(20))`): v at 10·dt reflected off the normal + r set to half its length, set to `randf(3, 6)·dt`; then the line j → j − unit v, the line j → j + 2 z (flags 0x10, the template) and the sphere 0.5 at the flame (0x214468): Ratchet or a 479 hit → 2 | [`flame_update`] |
//! | | the life (int) out, no owner, or the owner deleted → 2 | [`flame_update`] |

#![allow(clippy::needless_range_loop)] // the game's per-lane VU writes, spelled out.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::creature::{self as c, damage, flash, fx, target, turn, DT, DT2};
use crate::moby_update::manip;
use crate::moby_update::services::{fast_dec_timer_s16, pv, HitTemplate, World};
use crate::ps2v::Pf;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, FRAC_PI_6, PI, TAU};

pub const REFERENCE_LEVEL: u32 = 3;
/// The ship's update and class (0x277).
pub const UPDATE_FN: u32 = 0x2c_ca00;
pub const SHIP: i16 = 631;
pub const CLASSES: [i16; 1] = [SHIP];
/// The flame's update and class (0x350).
pub const FLAME_FN: u32 = 0x2d_47c0;
pub const FLAME: i16 = 848;
pub const FLAME_CLASSES: [i16; 1] = [FLAME];
/// Its other owner (level 15).
pub const QUARTU_OWNER: i16 = 233;
/// The rider's class.
pub const RIDER: i16 = 574;
/// The pieces the ship breaks into (0x637..0x639).
pub const PIECES: [i16; 3] = [1591, 1592, 1593];
/// The class Ratchet's flame hits count with.
const FLAME_TARGET: i16 = 479;
/// The exact-push marker of a hit's direction w.
const PUSH_W: u32 = 0x45af_df66;
/// gp−0x5264 / −0x5268 / −0x526c (level03): the slowest speed (·dt), the bob height and its period (s).
const MIN_SPEED: f32 = 0.1;
const BOB: f32 = 0.17;
const BOB_PERIOD: f32 = 2.0;
const DEG20: f32 = 0.349_065_84;
const DEG5: f32 = 0.087_266_46;
const DEG: f32 = 0.017_453_292;
/// gp−0x50d0 (level03): the flame's turn rate toward its target (° a second).
const FLAME_TURN: f32 = 90.0;

pub const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.0, flash_dist: 99840.0, scale: 3.0, light: 15.0, streaks: 20, sparks: 3, puffs: 4, debris: 1, sound: 2, shake: true };

/// The ship's pvar offsets (module doc).
pub mod sp {
    pub const DAMAGE: usize = 0x20;
    pub const ALIVE: usize = 0x2e;
    pub const LURE: usize = 0x38;
    pub const FLASH: usize = 0x60;
    pub const REC_A: usize = 0x120;
    pub const REC_B: usize = 0x150;
    pub const YAW_MANIP: usize = 0x180;
    pub const PITCH_MANIP: usize = 0x1c0;
    pub const YAW_VEL: usize = 0x200;
    pub const FIRE: usize = 0x204;
    pub const PAUSE: usize = 0x206;
    pub const RANGE_CUBOID: usize = 0x208;
    pub const EXIT_CUBOID_A: usize = 0x20c;
    pub const BOB: usize = 0x210;
    pub const SPEED: usize = 0x214;
    pub const ROLL_VEL: usize = 0x218;
    pub const PITCH_VEL: usize = 0x21c;
    pub const TURRET_YAW: usize = 0x220;
    pub const TURRET_PITCH: usize = 0x228;
    pub const EXIT_A: usize = 0x230;
    pub const EXIT_B: usize = 0x234;
    pub const RIDER: usize = 0x238;
    pub const SIDE: usize = 0x23c;
    pub const SHOT: usize = 0x240;
    pub const EXIT_CUBOID_B: usize = 0x244;
    pub const ARRIVE_CUBOID: usize = 0x248;
    pub const ARRIVE: usize = 0x24c;
    pub const VOICE: usize = 0x250;
    pub const ALERT: usize = 0x254;
    pub const SIZE: usize = 0x258;
    /// A patrol record: the cursor, the spline (+0x10), the path id (+0x14).
    pub const CURSOR: usize = 0;
    pub const SPLINE: usize = 0x10;
    pub const PATH: usize = 0x14;
}

/// The flame's pvar offsets (module doc).
pub mod fl {
    pub const OWNER: usize = 0x10;
    pub const LIFE: usize = 0x14;
    pub const SIDE: usize = 0x18;
    pub const SIZE: usize = 0x1c;
}

fn gone(w: &World, id: MobyId) -> bool { matches!(w.m(id).state, 0xfe | 0xfd) }

/// A moby pointer field (index + 1, 0 none).
fn moby_ref(w: &World, id: MobyId, o: usize) -> Option<MobyId> {
    let v = c::pi32(w, id, o);
    usize::try_from(v - 1).ok().filter(|&m| v > 0 && m < w.table.mobys.len())
}
fn set_ref(w: &mut World, id: MobyId, o: usize, m: Option<MobyId>) { c::set_pi32(w, id, o, m.map_or(0, |m| m as i32 + 1)); }

/// The rider (a moby index after the loader's link fixups; −1 none).
fn rider(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, sp::RIDER)).ok().filter(|&m| m < w.table.mobys.len()) }

fn fcos(a: f32) -> f32 { crate::hero::physics::fast_cos(Pf::f(a)).to_f32() }
fn fsin(a: f32) -> f32 { crate::hero::physics::fast_sin(Pf::f(a)).to_f32() }

/// `0x1f8ee8` (L03, = L01 0x221460): the xy scaled to length `l`, z and w kept ([`c::set_len2`]).
fn set_len_xy(v: c::V, l: f32) -> c::V { c::set_len2(v, l) }

fn spline(w: &World, p: i32) -> Option<usize> { usize::try_from(p).ok().filter(|&i| i < w.svc.splines.len()) }
fn points(w: &World, p: usize) -> Vec<[f32; 4]> { w.svc.splines[p].iter().map(|q| q.map(f32::from_bits)).collect() }

/// The arrival / exit paths' w: each point's chord to the next, but the last two (the game's loop).
fn write_lengths(w: &mut World, p: usize) {
    let pts = points(w, p);
    for k in 0..pts.len().saturating_sub(2) {
        w.svc.splines[p][k][3] = c::dist3(pts[k], pts[k + 1]).to_bits();
    }
}

/// The target record's position (0x274b78's out).
fn target_pos(t: &target::Target) -> c::V { t.pos }

/// Level03 0x2cca00, the ship (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < sp::SIZE { return; }
    let range = if c::pi32(w, id, sp::ALERT) != 0 { 80.0 } else { 40.0 };
    let tg = target::acquire(w, id, range);
    let t = target_pos(&tg);
    let sid = w.m(id).spawn_id;
    let dead = w.svc.save.death_level.contains(&sid);
    if dead || c::pi32(w, id, sp::REC_A + sp::PATH) == -1 || (w.m(id).state != 0 && c::pu8(w, id, sp::ALIVE) != 1) {
        if let Some(r) = rider(w, id) { w.delete_moby(r); }
        w.delete_moby(id);
        return;
    }
    if c::pi32(w, id, sp::LURE) != 0 {
        c::set_pi32(w, id, sp::LURE, 0);
        let a = w.ticks(240);
        c::set_pi32(w, id, sp::ALERT, a);
    }
    let a = c::pi32(w, id, sp::ALERT);
    if a != 0 { c::set_pi32(w, id, sp::ALERT, a.max(1) - 1); }
    if let Some(r) = rider(w, id).filter(|&r| w.m(r).o_class == RIDER && w.m(r).state < 0x80) {
        manip::big_head_scale(w, 2.5, r, 0x150);
        manip::look(w, r, r, 0x150, 2, 0.03, 0.3);
    }
    hits(w, id);
    match w.m(id).state {
        0 => init(w, id),
        1 => {
            let cub = c::pi32(w, id, sp::ARRIVE_CUBOID);
            if cub != -1 {
                let h = crate::moby_update::classes::units::hero_pos(w);
                if !crate::moby_update::triggers::point_in_cuboid(&w.svc.volumes, [h[0], h[1], h[2]], cub) {
                    tail(w, id);
                    return;
                }
            }
            c::set_pi32(w, id, sp::ARRIVE_CUBOID, -1);
            let p = c::pi32(w, id, sp::ARRIVE);
            follow(w, id, p, t, false);
        }
        2 => {
            if !patrol(w, id, t) { return; }
        }
        3 => {
            let side = c::pi32(w, id, sp::SIDE);
            let p = c::pi32(w, id, sp::EXIT_A);
            if p == -1 {
                let mut z = w.m(id).position[2];
                turn::approach(200.0, DT * 8.0, &mut z);
                w.mm(id).position[2] = z;
                spring_axis(w, id, 1, 0.0, DT2 * FRAC_PI_6, DT2 * FRAC_PI_3, DT * FRAC_PI_4, sp::PITCH_VEL);
                spring_axis(w, id, 0, 0.0, DT2 * FRAC_PI_6, DT2 * FRAC_PI_3, DT * FRAC_PI_4, sp::ROLL_VEL);
            } else {
                let p = if side != 0 { c::pi32(w, id, sp::EXIT_B) } else { p };
                follow(w, id, p, t, true);
            }
        }
        4 => {
            let mut timer = c::pi16(w, id, sp::PAUSE);
            let r = fast_dec_timer_s16(&mut timer);
            c::set_pi16(w, id, sp::PAUSE, timer);
            if r == 0 {
                let s = c::pf(w, id, sp::SPEED) - DT2 * 4.0;
                let s = if s < 0.0 { 0.0 } else { s };
                c::set_pf(w, id, sp::SPEED, s);
                let yaw = c::yaw(w, id);
                let mv = [fcos(yaw) * -s, fsin(yaw) * -s, 0.0, 0.0];
                bank(w, id, mv, 10.0);
            } else {
                w.mm(id).state = w.m(id).cmd;
            }
        }
        5 => {
            destroyed(w, id);
            return;
        }
        _ => {}
    }
    tail(w, id);
}

/// `SpringTurn` on Euler lane `k` (0 roll, 1 pitch) with its velocity pvar.
#[allow(clippy::too_many_arguments)]
fn spring_axis(w: &mut World, id: MobyId, k: usize, target: f32, acc: f32, damp: f32, max: f32, vel: usize) {
    let mut v = c::pf(w, id, vel);
    let cur = w.m(id).rotation[k];
    let a = turn::spring_turn(cur, target, acc, damp, max, &mut v);
    w.mm(id).rotation[k] = a;
    c::set_pf(w, id, vel, v);
}

/// The tail of 0x2cca00 (module table): the shadow probe, the bob, the rider.
fn tail(w: &mut World, id: MobyId) {
    if w.m(id).visible != 0 {
        let cam = w.camera.map(|x| f32::from_bits(x.0));
        if c::dist3(w.m(id).position, cam) < 40.0 { crate::shadows::probe_down(w, id); }
    }
    let ph = c::pf(w, id, sp::BOB);
    w.mm(id).position[2] -= BOB * fcos(ph);
    let period = crate::moby_update::services::fl(w.svc.timing.scale(Pf::f(BOB_PERIOD * 60.0)));
    let ph = c::add_rot(ph, TAU / period);
    c::set_pf(w, id, sp::BOB, ph);
    w.mm(id).position[2] += BOB * fcos(ph);
    if let Some(r) = rider(w, id) {
        let (p, e) = (w.m(id).position, w.m(id).rotation);
        {
            let m = w.mm(r);
            m.position = p;
            m.rotation = e;
        }
        if w.m(r).mode & mode::NO_ANIM == 0 { crate::moby_update::anim_sound::advance(w, r); }
        w.build_matrix(r);
        w.mm(r).mode |= mode::NO_UPDATE | mode::KEEP_MATRIX;
    }
}

/// State 0 of 0x2cca00 (module table).
fn init(w: &mut World, id: MobyId) {
    w.mm(id).b7f = 0x20;
    c::set_pf(w, id, sp::DAMAGE, 3.0);
    c::set_pi16(w, id, sp::DAMAGE + 4, 3);
    c::set_pu8(w, id, sp::DAMAGE + 8, 2);
    c::set_pu8(w, id, 0x58, 0x19);
    c::set_pu8(w, id, 0x59, 1);
    c::set_pu8(w, id, 0x5a, 6);
    c::set_pu8(w, id, sp::ALIVE, 1);
    c::set_pu8(w, id, sp::DAMAGE + 9, 0);
    let pa = c::pi32(w, id, sp::REC_A + sp::PATH);
    c::set_pi32(w, id, sp::REC_A + sp::SPLINE, pa);
    let pb = c::pi32(w, id, sp::REC_B + sp::PATH);
    if pb != -1 { c::set_pi32(w, id, sp::REC_B + sp::SPLINE, pb); }
    if let Some(p) = spline(w, pa).and_then(|p| points(w, p).first().copied()) { w.mm(id).position = p; }
    for o in [sp::SPEED, sp::ROLL_VEL, sp::PITCH_VEL, sp::SIDE] { c::set_pi32(w, id, o, 0); }
    c::set_pi32(w, id, sp::REC_A + sp::CURSOR, 1);
    manip::attach(w, id, 0, id, sp::YAW_MANIP);
    manip::attach(w, id, 1, id, sp::PITCH_MANIP);
    c::set_pi32(w, id, sp::VOICE, -1);
    c::set_pi32(w, id, sp::SHOT, 0);
    let e = w.m(id).rotation;
    c::set_pf(w, id, sp::TURRET_YAW, e[2]);
    c::set_pf(w, id, sp::TURRET_YAW + 4, 0.0);
    c::set_pf(w, id, sp::TURRET_PITCH, e[1]);
    c::set_pf(w, id, sp::TURRET_PITCH + 4, 0.0);
    if w.m(id).draw_dist < 0x80 { w.mm(id).draw_dist = 0x80; }
    if let Some(r) = rider(w, id) {
        w.anim_blend(r, 8, 0, 0);
        let m = w.mm(r);
        m.update_dist = 0;
        m.has_collision = false;
        m.mode &= !0x41;
    }
    let level = w.svc.level;
    let done = w.mission_done(level, w.m(id).mission) == 0xff;
    let arrive = spline(w, c::pi32(w, id, sp::ARRIVE));
    match arrive {
        Some(p) if !done && c::pi32(w, id, sp::ARRIVE_CUBOID) != -1 => {
            write_lengths(w, p);
            w.mm(id).state = 1;
            if let Some(q) = points(w, p).first().copied() { w.mm(id).position = q; }
        }
        _ => w.mm(id).state = 2,
    }
}

/// States 1 / 3 of 0x2cca00 on path `p` (module table). `exit`: the exit flight (faces the target, switches sides at
/// the end).
fn follow(w: &mut World, id: MobyId, p: i32, t: c::V, exit: bool) {
    let Some(p) = spline(w, p) else { return };
    let pts = points(w, p);
    let n = pts.len() as i32;
    if n < 2 { return; }
    let pos = w.m(id).position;
    let (seg, tt) = crate::spline::nearest(&pts, false, 1000.0, 5.0, 0.0, [pos[0], pos[1], pos[2]]).map_or((0, 0.0), |(_, c)| (c.seg, c.t));
    let w0 = pts[0][3];
    let along = seg as f32 * w0 + tt + 2.0;
    let to_end = c::dist3(pos, pts[n as usize - 1]);
    let s = c::pf(w, id, sp::SPEED);
    let a = DT2 * 10.0;
    let (s, over, cap) = if to_end <= (s * s) / (a + a) { (s - a, s - a < MIN_SPEED * DT, MIN_SPEED * DT) } else { (s + a, DT * 20.0 < s + a, DT * 20.0) };
    c::set_pf(w, id, sp::SPEED, if over { cap } else { s });
    let mut k = (along / w0) as i32;
    let mut f = (along - k as f32 * w0) / w0;
    if n - 1 <= k {
        let stopped = c::pf(w, id, sp::SPEED) <= MIN_SPEED * DT;
        if stopped {
            if !exit {
                c::set_pi32(w, id, sp::SIDE, 0);
                if let Some(q) = spline(w, c::pi32(w, id, sp::REC_A + sp::SPLINE)).and_then(|a| points(w, a).first().copied()) { w.mm(id).position = q; }
                c::set_pi32(w, id, sp::REC_A + sp::CURSOR, 1);
                pause(w, id, 2);
                return;
            }
            if c::pi32(w, id, sp::EXIT_B) == -1 || c::pi32(w, id, sp::REC_B + sp::PATH) == -1 { return; }
            let side = c::pi32(w, id, sp::SIDE) ^ 1;
            c::set_pi32(w, id, sp::SIDE, side);
            let rec = if side != 0 { sp::REC_B } else { sp::REC_A };
            if let Some(q) = spline(w, c::pi32(w, id, rec + sp::SPLINE)).and_then(|a| points(w, a).first().copied()) { w.mm(id).position = q; }
            c::set_pi32(w, id, rec + sp::CURSOR, 1);
            pause(w, id, 2);
            return;
        }
        k = n - 2;
        f = 1.0;
    }
    let (a0, a1) = (pts[k as usize], pts[k as usize + 1]);
    let q = c::add(c::scale(c::sub(a1, a0), f), a0);
    let dir = if exit || k + 1 >= n - 8 { [t[0] - pos[0], t[1] - pos[1]] } else { [pts[k as usize + 1][0] - pos[0], pts[k as usize + 1][1] - pos[1]] };
    c::turn::spring_turn2_pvar(w, id, c::atan(dir[0], dir[1]), 0.01, 0.3, 0.1, sp::YAW_VEL);
    let mv = c::sub(q, pos);
    let l = if c::pf(w, id, sp::SPEED) < c::len3(mv) { c::pf(w, id, sp::SPEED) } else { c::len3(mv) };
    bank(w, id, c::set_len3(mv, l), 20.0);
}

/// `cmd` = `next`, the pause (state 4) for `ticks(15)`.
fn pause(w: &mut World, id: MobyId, next: u8) {
    w.mm(id).cmd = next;
    w.mm(id).state = 4;
    let t = w.ticks(15);
    c::set_pi16(w, id, sp::PAUSE, t as i16);
}

/// `0x2cdd0c` (module doc): the move, then the pitch and roll into it.
fn bank(w: &mut World, id: MobyId, mv: c::V, k: f32) {
    {
        let m = w.mm(id);
        for j in 0..3 { m.position[j] += mv[j]; }
    }
    let a = c::sub_rot(c::atan(mv[0], mv[1]), c::yaw(w, id));
    let s = c::pf(w, id, sp::SPEED);
    let pitch = (s * DEG20 * fcos(a)) / (DT * k);
    let roll = (s * -DEG20 * fsin(a)) / (DT * k);
    spring_axis(w, id, 1, pitch, DT2 * FRAC_PI_6, DT2 * FRAC_PI_3, DT * FRAC_PI_4, sp::PITCH_VEL);
    spring_axis(w, id, 0, roll, DT2 * FRAC_PI_3, DT2 * 2.094_395_2, DT * FRAC_PI_2, sp::ROLL_VEL);
}

/// The turret's aim at `at` (state 2 of 0x2cca00): yaw and pitch springs in the ship's frame, on axis 2.
fn aim(w: &mut World, id: MobyId, at: c::V) -> c::V {
    let j = w.joint_point(id, 0);
    let v = c::sub(at, j);
    let r = w.m(id).rows;
    let l = [c::dot3(r[0], v), c::dot3(r[1], v), c::dot3(r[2], v), v[3]];
    let mut vel = c::pf(w, id, sp::TURRET_YAW + 4);
    let y = turn::spring_turn(c::pf(w, id, sp::TURRET_YAW), c::atan(l[0], l[1]), DT2 * TAU, DT2 * 4.0 * PI, DT * TAU, &mut vel);
    c::set_pf(w, id, sp::TURRET_YAW, y);
    c::set_pf(w, id, sp::TURRET_YAW + 4, vel);
    let mut vel = c::pf(w, id, sp::TURRET_PITCH + 4);
    let p = turn::spring_turn(c::pf(w, id, sp::TURRET_PITCH), -c::atan(c::len2(l), l[2]), DT2 * PI, DT2 * TAU, DT * FRAC_PI_2, &mut vel);
    c::set_pf(w, id, sp::TURRET_PITCH, p);
    c::set_pf(w, id, sp::TURRET_PITCH + 4, vel);
    manip::set_axis(w, id, id, sp::YAW_MANIP, y, 2);
    manip::set_axis(w, id, id, sp::PITCH_MANIP, p, 2);
    j
}

/// State 2 of 0x2cca00 (module table). False: the ship is gone (never here).
fn patrol(w: &mut World, id: MobyId, t: c::V) -> bool {
    let side = c::pi32(w, id, sp::SIDE);
    let rec = if side != 0 { sp::REC_B } else { sp::REC_A };
    let node = c::pi32(w, id, rec + sp::CURSOR);
    let pos = w.m(id).position;
    c::turn::spring_turn2_pvar(w, id, c::atan(t[0] - pos[0], t[1] - pos[1]), 0.01, 0.3, 0.1, sp::YAW_VEL);
    let shot = moby_ref(w, id, sp::SHOT).filter(|&s| w.m(s).o_class == FLAME && !gone(w, s));
    let j = match shot {
        Some(s) => {
            let at = w.m(s).position;
            aim(w, id, at)
        }
        None => {
            let j = aim(w, id, t);
            c::set_pi32(w, id, sp::SHOT, 0);
            j
        }
    };
    let Some(p) = spline(w, c::pi32(w, id, rec + sp::SPLINE)) else { return true };
    let pts = points(w, p);
    let n = pts.len() as i32;
    if node < 0 || n <= node { return true; }
    let pt = |i: i32| pts.get(i as usize).copied().unwrap_or([0.0; 4]);
    if 1 <= node && node < n - 1 {
        let d0 = c::dist3(pt(node), t);
        if d0 < 20.0 && (t[2] - pos[2]).abs() < 10.0 && d0 < c::dist3(pt(node + 1), t) && node < n - 2 {
            c::set_pi32(w, id, rec + sp::CURSOR, node + 1);
        }
    }
    let v = c::sub(pt(node), pos);
    let len = c::len3(v);
    let s = c::pf(w, id, sp::SPEED);
    let nearer = |w: &World| c::dist3(pt(node), t) < c::dist3(w.m(id).position, t);
    let brake = len < (s * s) / (DT2 * 4.0 + DT2 * 4.0) || nearer(w);
    let s = if brake {
        let s = s - DT2 * 4.0;
        if s < 0.0 { 0.0 } else { s }
    } else {
        let s = s + DT2 * 4.0;
        if DT * 10.0 < s { DT * 10.0 } else { s }
    };
    c::set_pf(w, id, sp::SPEED, s);
    let mv = c::set_len3(v, if nearer(w) { -s } else { s });
    bank(w, id, mv, 10.0);
    let exit_cub = if side == 0 { c::pi32(w, id, sp::EXIT_CUBOID_A) } else { c::pi32(w, id, sp::EXIT_CUBOID_B) };
    let inside = |w: &World, cub: i32| crate::moby_update::triggers::point_in_cuboid(&w.svc.volumes, [t[0], t[1], t[2]], cub);
    if inside(w, exit_cub) {
        let ex = if side == 0 { c::pi32(w, id, sp::EXIT_A) } else { c::pi32(w, id, sp::EXIT_B) };
        if let Some(e) = spline(w, ex) { write_lengths(w, e); }
        pause(w, id, 3);
        return true;
    }
    if !inside(w, c::pi32(w, id, sp::RANGE_CUBOID)) { return true; }
    let mut timer = c::pi16(w, id, sp::FIRE);
    let ready = fast_dec_timer_s16(&mut timer) != 0;
    c::set_pi16(w, id, sp::FIRE, timer);
    if !ready && (c::pi32(w, id, sp::SHOT) != 0 || c::pi32(w, id, sp::ALERT) == 0) { return true; }
    let n = w.ticks(360);
    c::set_pi16(w, id, sp::FIRE, n as i16);
    let mut v = set_len_xy(c::sub(t, j), DT * 6.0);
    v[2] = 0.0;
    let life = w.ticks(240);
    let f = spawn_flame(w, t[2], v, j, id, life);
    set_ref(w, id, sp::SHOT, f);
    true
}

/// Level03 0x2d5208(target z, velocity, at, owner, life): a flame (state 1). None when the table is full.
pub fn spawn_flame(w: &mut World, tz: f32, vel: c::V, at: c::V, owner: MobyId, life: i32) -> Option<MobyId> {
    let e = w.create_moby(FLAME)?;
    {
        let m = w.mm(e);
        m.draw_dist = 0x7e;
        m.update_dist = 0xff;
        m.state = 1;
        m.visible = 1;
        m.position = at;
    }
    if w.m(e).pvars.len() < fl::SIZE { return Some(e); }
    c::set_pv4(w, e, 0, [vel[0], vel[1], vel[2], tz]);
    c::set_pi32(w, e, fl::LIFE, life);
    set_ref(w, e, fl::OWNER, Some(owner));
    c::set_pi32(w, e, fl::SIDE, 0);
    Some(e)
}

/// State 5 of 0x2cca00 (module table).
fn destroyed(w: &mut World, id: MobyId) {
    let _ = w.rng.rand_angle();
    let pos = w.m(id).position;
    if let Some(r) = rider(w, id) {
        if w.m(r).pvars.len() > 0x262 { c::set_pu8(w, r, 0x262, 2); }
        let m = w.mm(r);
        m.update_dist = 0xff;
        m.mode &= !(mode::NO_UPDATE | mode::KEEP_MATRIX);
        let d = set_len_xy(c::sub(w.m(r).position, pos), 1.0);
        let tmpl = HitTemplate {
            dir: pv([d[0], d[1], 1.0, f32::from_bits(PUSH_W)]),
            attacker: Some(id),
            flags: 0x83_0000,
            b18: 3,
            b19: 3,
            h1a: w.m(id).o_class as u16,
            damage: Pf::f(20.0),
            w20: 1,
        };
        w.deliver_hit(r, &tmpl);
    }
    fx::beam_explosion(w, &BLAST, Some(id), pos);
    set_death_bits(w, id, 0, -1);
    let rot = w.m(id).rotation;
    for cl in PIECES { fx::break_piece(w, id, cl, pos, rot, 0, 0); }
    manip::detach(w, id, id, sp::YAW_MANIP);
    manip::detach(w, id, id, sp::PITCH_MANIP);
    let v = c::pi32(w, id, sp::VOICE);
    if v != -1 { w.release_sound(v, id); }
    c::set_pi32(w, id, sp::VOICE, -1);
    w.delete_moby(id);
}

/// Level03 0x2ce238, the ship's hits (module doc).
fn hits(w: &mut World, id: MobyId) {
    let h = w.get_hit(id, 0x33_0000, false);
    let r = damage::resolve(w, id, h, sp::DAMAGE, 0, 4);
    if r.out5 != 1 && w.m(id).state != 5 {
        let hp = c::pf(w, id, sp::DAMAGE) - r.damage;
        c::set_pf(w, id, sp::DAMAGE, hp);
        if r.reaction == 1 || hp <= 0.0 {
            w.mm(id).state = 5;
        } else if r.reaction != 0xb {
            c::set_pu8(w, id, sp::FLASH + 7, 0xb4);
            flash::start(w, id, sp::FLASH);
            if let Some(rec) = r.hit.or(h).filter(|rec| rec.dir[3].0 == PUSH_W && w.m(id).state != 4) {
                let pos = w.m(id).position;
                let p = rec.pos.map(|x| x.to_f32());
                let a = c::sub_rot(c::atan(pos[0] - p[0], pos[1] - p[1]), c::yaw(w, id));
                let (co, si) = (fcos(a), fsin(a) * -DEG20);
                if w.m(id).rotation[0].abs() < si.abs() {
                    w.mm(id).rotation[0] = si;
                    c::set_pf(w, id, sp::ROLL_VEL, 0.0);
                }
                if w.m(id).rotation[1].abs() < (co * DEG5).abs() {
                    w.mm(id).rotation[1] = co * DEG5;
                    c::set_pf(w, id, sp::PITCH_VEL, 0.0);
                }
                c::set_pf(w, id, sp::SPEED, DT * 4.0);
                let t = w.ticks(60);
                c::set_pi16(w, id, sp::PAUSE, t as i16);
                let st = w.m(id).state;
                w.mm(id).cmd = st;
                w.mm(id).state = 4;
                w.play_sound(1, 0, id);
            }
        }
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, sp::FLASH);
}

/// The owner's voice word (631: +0x250; 233: +0x1b8).
fn voice_slot(w: &World, owner: MobyId) -> Option<usize> {
    match w.m(owner).o_class {
        SHIP => Some(sp::VOICE),
        QUARTU_OWNER => Some(0x1b8),
        _ => None,
    }
}

fn release_voice(w: &mut World, owner: MobyId, slot: Option<usize>) {
    let Some(o) = slot.filter(|&o| w.m(owner).pvars.len() >= o + 4) else { return };
    let v = c::pi32(w, owner, o);
    if v != -1 { w.release_sound(v, owner); }
    c::set_pi32(w, owner, o, -1);
}

/// Ratchet's moby or a 479 (a flame line / sphere that hits it ends the flame).
fn flame_victim(w: &World, m: Option<MobyId>) -> bool {
    m.is_some_and(|m| Some(m) == w.hero_moby || w.m(m).o_class == FLAME_TARGET)
}

/// Level03 0x2d47c0, the flame (module doc).
pub fn flame_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < fl::SIZE { return; }
    let tg = target::acquire(w, id, 20.0);
    let t = target_pos(&tg);
    let owner = moby_ref(w, id, fl::OWNER);
    let slot = owner.and_then(|o| voice_slot(w, o));
    if let Some(o) = owner.filter(|&o| w.m(o).o_class == QUARTU_OWNER) {
        if matches!(w.m(o).state, 0x40 | 1) { w.mm(id).state = 2; }
    }
    match w.m(id).state {
        1 => {}
        2 => {
            if let Some(o) = owner { release_voice(w, o, slot); }
            w.delete_moby(id);
            return;
        }
        _ => return,
    }
    let mut vel = c::pv4(w, id, 0);
    let d0 = set_len_xy(vel, 1.0);
    vel[2] = 1.0;
    let st = w.m(id).state;
    let dir = set_len_xy(d0, 1.0);
    let tmpl = HitTemplate {
        dir: pv([dir[0], dir[1], 1.0, f32::from_bits(PUSH_W)]),
        attacker: Some(id),
        flags: 0x1_0003,
        b18: st,
        b19: st,
        h1a: w.m(id).o_class as u16,
        damage: Pf::f(2.0),
        w20: 0,
    };
    let pos = w.m(id).position;
    let lim = DT * 8.0;
    let vz = -(pos[2] - vel[3]) / 10.0;
    vel[2] = if lim < vz { lim } else if vz < -lim { -lim } else { vz };
    let mut heading = c::atan(vel[0], vel[1]);
    let to = c::atan(t[0] - pos[0], t[1] - pos[1]);
    if c::diff_rots(heading, to) < FRAC_PI_2 {
        let rate = FLAME_TURN * DEG * DT;
        turn::approach_rot(to, rate, &mut heading);
        vel[0] = fcos(heading) * c::len2(vel);
        vel[1] = fsin(heading) * c::len2(vel);
    }
    c::set_pv4(w, id, 0, vel);
    {
        let m = w.mm(id);
        for k in 0..3 { m.position[k] += vel[k]; }
    }
    let pos = w.m(id).position;
    let Some(o) = owner else {
        w.mm(id).state = 2;
        return;
    };
    let d = c::dist3(pos, w.m(o).position);
    let reach = w.m(o).position[2] - t[2];
    if let Some(so) = slot.filter(|&so| w.m(o).pvars.len() >= so + 4) {
        let v = c::pi32(w, o, so);
        if d <= reach {
            if v != -1 { w.release_sound(v, o); }
            c::set_pi32(w, o, so, -1);
        } else if !w.sound_alive(v, o) {
            let s = w.play_sound(0, 4, o);
            c::set_pi32(w, o, so, s);
        }
    }
    if reach < d {
        let k6 = w.ticks(6);
        if k6 != 0 && c::pi32(w, id, fl::LIFE) % k6 == 0 { spray(w, id, o, heading, &tmpl); }
    }
    let life = c::pi32(w, id, fl::LIFE);
    let out = if life == 0 { true } else {
        let l = life.max(1) - 1;
        c::set_pi32(w, id, fl::LIFE, l);
        l < 1
    };
    if out || gone(w, o) { w.mm(id).state = 2; }
}

/// The spray of 0x2d47c0 every `ticks(6)` (module table): the flames, the tracer, the wall sparks and the hit lines.
fn spray(w: &mut World, id: MobyId, owner: MobyId, heading: f32, tmpl: &HitTemplate) {
    let side = c::pi32(w, id, fl::SIDE) ^ 1;
    c::set_pi32(w, id, fl::SIDE, side);
    let turn90 = if side != 0 { FRAC_PI_2 } else { -FRAC_PI_2 };
    let a = c::add_rot(heading, turn90);
    let off = [fcos(a) * 0.25, fsin(a) * 0.25, 0.0, 0.0];
    let pos = w.m(id).position;
    let p = c::add(pos, off);
    let mut j = w.joint_point(owner, (3 - side) as usize);
    let d = c::sub(p, j);
    let end = c::add(c::scale(d, 2.0), j);
    let rv = |w: &mut World| {
        let x = w.rng.randf(-1.0, 1.0);
        let y = w.rng.randf(-1.0, 1.0);
        let z = w.rng.randf(-1.0, 1.0);
        [x, y, z, 0.0]
    };
    for _ in 0..5 {
        let r = rv(w);
        let r = c::set_len3(r, c::len3(d) * 0.65);
        let r = c::add(d, r);
        let sp = w.rng.randf(DT * 5.0, DT * 15.0);
        let r = c::set_len3(r, sp);
        let (a, b) = (w.ticks(5), w.ticks(10));
        let life = w.rng.rand_range(a, b);
        fx::part27(w, f32::from_bits(0x4743_5000), j, r, 0x5f2f_4f6f, life);
    }
    let d = c::set_len3(d, DT * 0.5);
    let (a, b) = (w.ticks(4), w.ticks(7));
    let life = w.rng.rand_range(a, b);
    fx::part27(w, f32::from_bits(0x48c3_5000), j, d, 0x2f4f_7f7f, life);
    // The tracer: a 2-unit line ending at the joint, moving toward the flame's next point.
    let six = crate::moby_update::services::fl(w.svc.timing.scale(Pf::f(6.0)));
    let vel = c::pv4(w, id, 0);
    let h = c::sub(c::add(c::scale(vel, six), pos), off);
    let to = c::sub(h, j);
    let s2 = c::set_len3(to, 2.0);
    let tail = c::sub(j, s2);
    let head = c::add(s2, tail);
    let tv = c::set_len3(to, DT * 100.0);
    let timer = w.ticks(45);
    if let Some(ps) = w.particles.as_deref_mut() {
        crate::particles::type49::spawn(ps, owner as u32, tail, head, [tv[0], tv[1], tv[2]], timer as i16);
    }
    let Some(hit) = w.coll_line(pv(j), pv(end), 0, Some(owner)) else { return };
    j = [hit.point[0], hit.point[1], hit.point[2], 0.0];
    let n = [hit.normal[0], hit.normal[1], hit.normal[2], 0.0];
    let d = c::set_len3(d, DT * 10.0);
    for _ in 0..10 {
        let r = rv(w);
        let s = crate::moby_update::services::reflect(pv(d), pv(n)).map(|x| x.to_f32());
        let r = c::set_len3(r, c::len3(s) * 0.5);
        let s = c::add(s, r);
        let sp = w.rng.randf(DT * 3.0, DT * 6.0);
        let s = c::set_len3(s, sp);
        let (a, b) = (w.ticks(15), w.ticks(20));
        let life = w.rng.rand_range(a, b);
        fx::part27(w, f32::from_bits(0x46ea_6000), j, s, 0x7f2f_4f6f, life);
    }
    let back = c::add(j, c::set_len3(d, -1.0));
    let mut ended = false;
    let line = |w: &mut World, a: c::V, b: c::V| crate::moby_update::services::line_hit_in(w.table, w.svc, w.classes, w.coll, pv(a), pv(b), 0x10, Some(owner), tmpl).and_then(|h| h.moby);
    let m = line(w, j, back);
    if flame_victim(w, m) { ended = true; }
    let up = [j[0], j[1], j[2] + 2.0, j[3]];
    let m = line(w, j, up);
    if flame_victim(w, m) { ended = true; }
    let at = w.m(id).position;
    let hit = crate::moby_update::services::sphere_mobys_in(w.table, w.svc, w.classes, Pf::f(0.5), pv(at), 0x10, Some(id), Some(tmpl)).first().copied();
    if flame_victim(w, hit) { ended = true; }
    if ended { w.mm(id).state = 2; }
}
