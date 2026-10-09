//! **Quartu's guards, class 44** (level15 `0x2979d8`; census U528, 21 placed) and **their shot 934** (spawner
//! `0x2e4580`, update `0x2e4380`; created by code only). Robots that stand at their posts or walk a patrol path
//! (+0x124), watching an area (+0x120, a polygon path) for Ratchet (`0x274df8`); the Hologuise (hero body 3, state 0x65)
//! fools them. A guard that sees Ratchet raises the alarm in its group (`0x29a0d0`), runs to its alarm 408
//! ([`super::quartu_alarm`]) and sets it off, or shoots at him from both arms; one that is told of him (+0x156) looks
//! around. Read from the level15 decomp and disassembly; native `f32`.
//!
//! * **Every tick**: game mode 2 → hidden. Not culled by its camera cuboids (+0x2d0..+0x2dc) → shown with collision,
//!   else hidden (but in its flights 0x15 / 0x17). The hits and the senses (`0x29a570`, module part [`senses`]); the
//!   big-head cheat on its head (list 1, 2.5); the shadow (within 28 of the camera, +0x7f 0x16); the timers +0x154,
//!   +0x156 (0 while Ratchet is disguised), +0x19c; told of Ratchet with an alarm not running: the alarm set off for
//!   `ticks(180)`.
//! * **The senses** (`0x29a570`): hits (mask 0x330000, health +0x20; a hit from class 71 counts as 3; dead → the knock
//!   flight 0x17 (8·dt, 8·dt up, 30·dt² gravity), else → 0x15 (6·dt, 4·dt up); red 0xf0 / 0x78 / 0xfa; on Ratchet's
//!   body 0 its group told); its idle sound timer; the line of sight to Ratchet's body point (within 64); the target
//!   in its area (`0x274df8`, 128): in front (90°, at most 4 apart when wall-walking) and seen → it keeps its state,
//!   else calm (+0x114 = 2) — the Hologuise (body 3) calms a guard not yet suspicious.
//! * **States** (module table): 0 set up; 1 waiting for its links (+0x180..) to reach state 2; 2 / 3 at a post or a
//!   patrol point (waiting `ticks(+0x148)`, taking turns with its group, `0x299ff8`); 4 walking (2·dt) to the next
//!   point; 5 switching its barriers off (78, [`super::barrier_field`]) on Ratchet's state 0x59; 6 / 7 / 8 turning
//!   (sequences 10 / 11 / 12 with their angle curves 0x1cc060 / 0x1cbfe0 / 0x1cc0e0); 9 back to its post; 10 looking;
//!   0xb the alert (its alarm poked, the group told), → 0xe to the alarm, or back to what it did; 0xc fooled
//!   (disguised Ratchet: back to its post, the alarm stopped); 0xd walking (3.5·dt) on alert; 0xe to the alarm (4·dt),
//!   0xf pressing it; 0x10 / 0x12 aiming, 0x11 / 0x13 firing (lists 3 / 2: at Ratchet within 45°, else ahead); 0x15 /
//!   0x17 the knock flights: landed → the explosion, its barriers off, sound 13, gone.
//! * **The glow** (+0x90): a pulse every `ticks(50)` red when alerted (or told), every `ticks(90)` green-yellow when
//!   suspicious (+0x158), every `ticks(170)` else, tweened 10 % a tick; the level's alert word 0x141630 keeps the
//!   highest of the tick. Three glows (0.2, pull 0.08, alpha 0x30) on joint lists 4..6 (`0x29aba8`).
//! * **934** (`0x2e4380`): spins, flies its velocity (5 times the aim of length 8·dt); a hit (the line, flags 0, the
//!   template damage 2; or its sphere 0.3; or its `ticks(240)` out) on a moby other than its guard: a sphere hit (0.5,
//!   damage 2) unless a barrier 78, the death explosion (0.5, 13), sound 0, gone; a type-61 glow every third tick.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2979d8` | 44 | [`update`] |
//! | `0x29a570` / `0x29a0d0` / `0x29a248` | senses and hits, tell the group, tell one | [`senses`], [`tell_group`] |
//! | `0x299880` / `0x299a68` / `0x299ae0` | turn to an angle / a point, walk to a point | [`turn_to`], [`face`], [`walk_to`] |
//! | `0x299ca0` / `0x299dd8` / `0x299ff8` | to the alarm?, a bait 635, the patrol turn | [`go_alarm`], [`find_bait`], [`my_turn`] |
//! | `0x29a2a8` / `0x29aba8` | the head and body look, the glows | [`look`], [`glow_quads`] |
//! | `0x2e4580` / `0x2e4380` | 934 | [`fire`], [`shot_update`] |
//!
//! [L] A shot whose timer runs out with no hit flies on (the game reads the last collision's moby there). The last
//! frame read on a turn's end is that of the sequence playing (the call's sequence argument is not set up).

use super::quartu_alarm as alarm;
use super::GlowQuad;
use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, damage, flash, fx, knock, target, turn, walker, DT, DT2, SPEED};
use crate::moby_update::services::{pf, pv, pvar as p, HitTemplate, Services, World};
use crate::moby_update::{manip, story};
use crate::ps2v::Pf;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

pub const REFERENCE_LEVEL: u32 = 15;
pub const UPDATE_FN: u32 = 0x29_79d8;
pub const DRAW_FN: u32 = 0x29_aba8;
pub const CLASSES: [i16; 1] = [44];
pub const SHOT_FN: u32 = 0x2e_4380;
pub const SHOT_CLASSES: [i16; 1] = [934];

mod o {
    pub const D: usize = 0x20;
    pub const IDLE: usize = 0x38;
    pub const F: usize = 0x60;
    pub const K: usize = 0x70;
    pub const TGT: usize = 0xd0;
    /// The target record (+0xd0 position, +0xe0, +0xf0, +0x100 its body point: the aim).
    pub const AIM: usize = 0x100;
    /// The target record's moby (index + 1; Ratchet when none was ever set).
    pub const HERO: usize = 0x110;
    /// The target record's kind (0 Ratchet, 1 a decoy, 2 none: calm).
    pub const ALERT: usize = 0x114;
    pub const AREA: usize = 0x120;
    pub const PATROL: usize = 0x124;
    pub const ALARM: usize = 0x128;
    pub const BARRIER_A: usize = 0x12c;
    pub const IDLE_T: usize = 0x130;
    pub const SPEED: usize = 0x134;
    pub const YAW_V: usize = 0x138;
    pub const POINT: usize = 0x13c;
    pub const ADVANCE: usize = 0x144;
    pub const WAIT_SECS: usize = 0x148;
    pub const WAIT_T: usize = 0x14c;
    pub const BARRIER_B: usize = 0x150;
    pub const T154: usize = 0x154;
    pub const TOLD: usize = 0x156;
    pub const WARY: usize = 0x158;
    pub const SEES: usize = 0x15a;
    pub const LOST: usize = 0x15b;
    pub const RET_STATE: usize = 0x15c;
    pub const RET_SEQ: usize = 0x15d;
    pub const DOUBT: usize = 0x15e;
    pub const ALERTED: usize = 0x15f;
    pub const HOME: usize = 0x160;
    pub const HOME_YAW: usize = 0x16c;
    pub const TURN_FROM: usize = 0x170;
    pub const TURN_TO: usize = 0x174;
    pub const TURN_D: usize = 0x178;
    pub const BAITS: usize = 0x17c;
    pub const LINKS: usize = 0x180;
    pub const BAIT: usize = 0x190;
    pub const BAIT_SEEN: usize = 0x194;
    pub const BAIT_T: usize = 0x196;
    pub const CALLER: usize = 0x198;
    pub const CALL_T: usize = 0x19c;
    pub const SHOT: usize = 0x19e;
    pub const LOOKS: usize = 0x19f;
    pub const LOOK_A: usize = 0x1a0;
    pub const LOOK_B: usize = 0x220;
    pub const EYES: usize = 0x2a0;
    pub const CUB: usize = 0x2d0;
    pub const SIZE: usize = 0x2e0;
}

/// Level15 gp words −0x5818.. (a7e8..a814).
mod k {
    pub const SCALE: f32 = 1.0;
    pub const G_HURT: f32 = 20.0;
    pub const G_DEAD: f32 = 30.0;
    pub const UP_HURT: f32 = 4.0;
    pub const SPEED_HURT: f32 = 6.0;
    pub const DRAG_HURT: f32 = 0.0;
    pub const UP_DEAD: f32 = 8.0;
    pub const SPEED_DEAD: f32 = 8.0;
    pub const DRAG_DEAD: f32 = 0.03;
    pub const SHOT: f32 = 8.0;
}

const HOLOGUISE_STATE: i32 = 0x65;
const TERMINAL_STATE: i32 = 0x59;
const DISGUISED: u8 = 3;
const BAIT_CLASS: i16 = 0x27b;
const BARRIER_CLASS: i16 = 0x4e;
const GUARD_WORDS: (u32, u32) = (0x14_1630, 0x16_13e8);

/// The turn sequences' angle curves (degrees by key frame): 10 0x1cc060, 11 0x1cbfe0, 12 0x1cc0e0.
const TURN_RIGHT: [u32; 32] = [0x00000000, 0x4030c49c, 0x40ab74bc, 0x41049375, 0x4140b852, 0x4184ae14, 0x41abdf3b, 0x41d56c8b, 0x42006b85, 0x4216cccd, 0x422d9aa0, 0x42465d2f, 0x42615b23, 0x427c71aa, 0x428abd71, 0x42974189, 0x42a3e7f0, 0x42ae49ba, 0x42b40000, 0x42b62a7f, 0x42b794fe, 0x42b85a1d, 0x42b89581, 0x42b86354, 0x42b7de35, 0x42b726e9, 0x42b64ac1, 0x42b57333, 0x42b4b74c, 0x42b4322d, 0x42b40000, 0x42b40000];
const TURN_LEFT: [u32; 32] = [0x80000000, 0xc030c49c, 0xc0ab74bc, 0xc1049375, 0xc140b852, 0xc184ae14, 0xc1abdf3b, 0xc1d56c8b, 0xc2006b85, 0xc216cccd, 0xc22d9aa0, 0xc2465d2f, 0xc2615b23, 0xc27c71aa, 0xc28abd71, 0xc2974189, 0xc2a3e7f0, 0xc2ae49ba, 0xc2b40000, 0xc2b62a7f, 0xc2b794fe, 0xc2b85a1d, 0xc2b89581, 0xc2b86354, 0xc2b7de35, 0xc2b726e9, 0xc2b64ac1, 0xc2b57333, 0xc2b4b74c, 0xc2b4322d, 0xc2b40000, 0xc2b40000];
const TURN_BACK: [u32; 37] = [0x80000000, 0xbfcd9168, 0xc014bc6a, 0xc0704189, 0xc0efc6a8, 0xc16fa5e3, 0xc1c8a7f0, 0xc213374c, 0xc246851f, 0xc27c2b02, 0xc2990dd3, 0xc2b322d1, 0xc2cb851f, 0xc2e3f646, 0xc2fc35c3, 0xc3099168, 0xc313ced9, 0xc31ca7f0, 0xc324d2b0, 0xc32bd333, 0xc3311439, 0xc3340000, 0xc3355893, 0xc3364b02, 0xc336e419, 0xc3372f5c, 0xc337399a, 0xc3370ed9, 0xc336bb23, 0xc3364b02, 0xc335cac1, 0xc3354625, 0xc334c9ba, 0xc33461cb, 0xc3341aa0, 0xc3340000, 0xc3340000];

fn link(w: &World, v: i32) -> Option<MobyId> { usize::try_from(v).ok().filter(|&m| m < w.table.mobys.len()) }
fn ptr(w: &World, id: MobyId, at: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, at) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn st(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_st(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn wrapped(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn hero_state(w: &World) -> i32 { w.hero.state }
fn body(w: &World) -> c::V { let b = w.hero.body_point; [b[0].to_f32(), b[1].to_f32(), b[2].to_f32(), 0.0] }
fn hero_pos_of(w: &World, id: MobyId) -> c::V { ptr(w, id, o::HERO).map_or_else(|| super::hero_pos(w), |h| c::pos(w, h)) }
fn path_pts(w: &World, i: i32) -> Vec<[f32; 4]> {
    usize::try_from(i).ok().and_then(|i| w.svc.splines.get(i)).map(|s| s.iter().map(|q| q.map(f32::from_bits)).collect()).unwrap_or_default()
}
fn patrol_point(w: &World, id: MobyId) -> c::V {
    let pts = path_pts(w, c::pi32(w, id, o::PATROL));
    let k = c::pi32(w, id, o::POINT).max(0) as usize;
    pts.get(k).copied().unwrap_or(c::pos(w, id))
}
fn alarm_of(w: &World, id: MobyId) -> Option<MobyId> { link(w, c::pi32(w, id, o::ALARM)) }
fn alarm_on(w: &World, a: MobyId) -> bool { w.m(a).pvars.len() >= 0x1c && c::pi32(w, a, 8) != 0 }

/// The blend to `seq` over `ticks(t)` unless key B already is it.
fn seq(w: &mut World, id: MobyId, s: u8, t: i32) {
    if w.m(id).anim.seq_b != s {
        let n = w.ticks(t);
        w.anim_blend(id, s, 0, n);
    }
}

/// The patrol's next point (+0x144 set: one on, modulo the point count).
fn advance(w: &mut World, id: MobyId) {
    if c::pi32(w, id, o::ADVANCE) != 0 {
        let n = path_pts(w, c::pi32(w, id, o::PATROL)).len().max(1) as i32;
        let k = (c::pi32(w, id, o::POINT) + 1) % n;
        c::set_pi32(w, id, o::POINT, k);
    }
    c::set_pi32(w, id, o::ADVANCE, 0);
}

/// The alert's start: `ret` / `ret_seq` to go back to; the alert sequence 1 (state 0xb) the first time.
fn alert_to(w: &mut World, id: MobyId, ret: u8, ret_seq: u8) {
    c::set_pu8(w, id, o::ALERTED, 1);
    set_st(w, id, 0xb);
    c::set_pu8(w, id, o::RET_STATE, ret);
    c::set_pu8(w, id, o::RET_SEQ, ret_seq);
    seq(w, id, 1, 10);
}

/// Ratchet spotted: the alert (first time) or aiming (0x10).
fn spotted(w: &mut World, id: MobyId) {
    w.mm(id).cmd = 1;
    if c::pu8(w, id, o::ALERTED) == 0 {
        alert_to(w, id, 0x10, 3);
    } else {
        set_st(w, id, 0x10);
        seq(w, id, 3, 10);
    }
}

/// Told of Ratchet on the way: to the next point on alert (0xd).
fn told(w: &mut World, id: MobyId) {
    w.mm(id).cmd = 1;
    if c::pu8(w, id, o::ALERTED) == 0 {
        alert_to(w, id, 0xd, 0xe);
    } else {
        set_st(w, id, 0xd);
        seq(w, id, 0xe, 10);
    }
}

/// `0x299880(angle, m, P, state, seq)`: a turn to `angle`: within 45° straight to `state` / `seq` (false); else the
/// turn sequence (10 left, 11 right, 12 about) in state 6 / 7 / 8 (true).
pub fn turn_to(w: &mut World, id: MobyId, angle: f32, state: u8, s: u8) -> bool {
    let yaw = w.m(id).rotation[2];
    let d = -c::sub_rot(yaw, angle);
    c::set_pi32(w, id, o::ADVANCE, 0);
    c::set_pf(w, id, o::TURN_TO, angle);
    c::set_pf(w, id, o::TURN_FROM, yaw);
    c::set_pf(w, id, o::TURN_D, d);
    c::set_pu8(w, id, o::RET_STATE, state);
    c::set_pu8(w, id, o::RET_SEQ, s);
    let (ns, nq) = if !(-2.094_395_2..=2.094_395_2).contains(&d) {
        if 0.0 < d { c::set_pf(w, id, o::TURN_D, d - TAU); }
        (8, 12)
    } else if -FRAC_PI_4 <= d {
        if d <= FRAC_PI_4 {
            set_st(w, id, state);
            seq(w, id, s, 10);
            return false;
        }
        (7, 11)
    } else {
        (6, 10)
    };
    set_st(w, id, ns);
    seq(w, id, nq, 10);
    true
}

/// `0x299a68(m, P, point, state, seq)`: [`turn_to`] the heading to `point`.
pub fn face(w: &mut World, id: MobyId, point: c::V, state: u8, s: u8) -> bool {
    let p = c::pos(w, id);
    turn_to(w, id, c::atan(point[0] - p[0], point[1] - p[1]), state, s)
}

/// `0x299ae0(speed, m, point)`: turn toward `point` (4π·dt², 400°·dt), speed up to `speed` while facing it within 45°
/// (9·dt²), the move with collision (0.5, 0.5), sinking 0.5·dt; whether within 0.2 of it (xy).
pub fn walk_to(w: &mut World, id: MobyId, speed: f32, point: c::V) -> bool {
    let p = c::pos(w, id);
    let a = c::atan(point[0] - p[0], point[1] - p[1]);
    let (mut yaw, mut v) = (w.m(id).rotation[2], c::pf(w, id, o::YAW_V));
    turn::turn_toward(a, DT2 * 4.0 * PI, DT2 * 4.0 * PI, DT * 6.981_317, &mut yaw, &mut v);
    w.mm(id).rotation[2] = yaw;
    c::set_pf(w, id, o::YAW_V, v);
    let a = c::atan(point[0] - p[0], point[1] - p[1]);
    let goal = if c::diff_rots(yaw, a) < FRAC_PI_4 { speed } else { 0.0 };
    let mut s = c::pf(w, id, o::SPEED);
    turn::approach(goal, DT2 * 9.0, &mut s);
    c::set_pf(w, id, o::SPEED, s);
    let mut mv = [yaw.cos() * s, yaw.sin() * s, -(DT * 0.5), 0.0];
    walker::move_collide(w, id, 0.5, 0.5, 0.0, &mut mv, 0);
    c::dist2(c::pos(w, id), point) < 0.2
}

/// `0x299ca0`: an alarm not running whose group has no guard already at it (state 0xe) or nearer to its own.
pub fn go_alarm(w: &World, id: MobyId) -> bool {
    let Some(a) = alarm_of(w, id) else { return false };
    if alarm_on(w, a) { return false; }
    let g = w.m(id).group;
    if g == -1 { return true; }
    let d = c::dist2(c::pos(w, id), c::pos(w, a));
    for m in crate::moby_update::scheduler::group_ids(w, g) {
        if w.m(m).state == 0xe { return false; }
        if w.m(m).cmd == 1 && w.m(m).pvars.len() >= o::SIZE {
            if let Some(ma) = alarm_of(w, m) {
                if c::dist2(c::pos(w, m), c::pos(w, ma)) < d { return false; }
            }
        }
    }
    true
}

/// `0x299dd8`: the nearest bait (class 635, states 6..15) of group +0x17c in view (within 70° of the yaw, or once one
/// has been met in state 7 / 15) with a clear line; within 8 unless met.
pub fn find_bait(w: &mut World, id: MobyId) -> Option<MobyId> {
    let g = c::pi32(w, id, o::BAITS);
    if g == -1 { return None; }
    let p = c::pos(w, id);
    let from = [p[0], p[1], p[2] + 1.0, p[3]];
    let (mut best, mut bd) = (None, 0.0f32);
    for m in crate::moby_update::scheduler::group_ids(w, g as i8) {
        if w.m(m).o_class != BAIT_CLASS || !(6..0x10).contains(&w.m(m).state) { continue; }
        let q = c::pos(w, m);
        let off = c::diff_rots(w.m(id).rotation[2], c::atan(q[0] - p[0], q[1] - p[1]));
        if matches!(w.m(m).state, 7 | 0xf) { c::set_pi16(w, id, o::BAIT_SEEN, 1); }
        if c::pi16(w, id, o::BAIT_SEEN) == 0 && 1.221_730_5 <= off { continue; }
        let to = [q[0], q[1], q[2] + 1.0, q[3]];
        if w.coll_line(pv(from), pv(to), 2, None).is_some() { continue; }
        let d = c::dist3(from, to);
        if best.is_none() || d < bd {
            best = Some(m);
            bd = d;
        }
    }
    if c::pi16(w, id, o::BAIT_SEEN) != 0 || bd <= 8.0 { best } else { None }
}

/// `0x299ff8`: no other live guard of the group idle at a lower patrol point (its turn).
pub fn my_turn(w: &World, id: MobyId) -> bool {
    let g = w.m(id).group;
    if g == -1 { return false; }
    let mine = c::pi32(w, id, o::POINT);
    for m in crate::moby_update::scheduler::group_ids(w, g) {
        let mo = w.m(m);
        if mo.o_class != CLASSES[0] || mo.state >= 0xfd || mo.cmd != 0 || mo.pvars.len() < o::SIZE { continue; }
        if c::pi32(w, m, o::ADVANCE) == 0 && c::pi32(w, m, o::PATROL) != -1 && mine <= c::pi32(w, m, o::POINT) { return false; }
    }
    true
}

/// `0x29a0d0`: Ratchet disguised: +0x156 cleared; no group: told for `ticks(180)`; else every live guard of the group
/// in sight (a clear line 1 up) told for `ticks(300)` and sent to look at this one (`0x29a248`).
pub fn tell_group(w: &mut World, id: MobyId) {
    if hero_state(w) == HOLOGUISE_STATE {
        c::set_pi16(w, id, o::TOLD, 0);
        return;
    }
    let g = w.m(id).group;
    if g == -1 {
        let t = w.ticks(0xb4);
        c::set_pi16(w, id, o::TOLD, t as i16);
        return;
    }
    let p = c::pos(w, id);
    let from = [p[0], p[1], p[2] + 1.0, p[3]];
    for m in crate::moby_update::scheduler::group_ids(w, g) {
        if (w.m(m).state as i8) < 0 || w.m(m).o_class != CLASSES[0] || w.m(m).pvars.len() < o::SIZE { continue; }
        let q = c::pos(w, m);
        if w.coll_line(pv([q[0], q[1], q[2] + 1.0, q[3]]), pv(from), 2, None).is_some() { continue; }
        let t = w.ticks(300);
        c::set_pu8(w, m, o::LOST, 0);
        c::set_pi16(w, m, o::TOLD, t as i16);
        if m != id {
            if c::pi16(w, m, o::CALL_T) != 0 && w.rng.randi(0x10) != 0 { continue; }
            c::set_pi32(w, m, o::CALLER, id as i32 + 1);
            let t = w.ticks(0xb4);
            c::set_pi16(w, m, o::CALL_T, t as i16);
        }
    }
}

/// `0x29a570(m, culled)` (module doc).
pub fn senses(w: &mut World, id: MobyId, culled: bool) {
    if st(w, id) == 0 { return; }
    let cs = super::class_scale(w, w.m(id).o_class);
    w.mm(id).scale = cs * k::SCALE;
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, o::D, 0, 4);
    if res.out5 != 1 && st(w, id) != 0x17 {
        if w.body() == 0 { tell_group(w, id); }
        let hp = c::pf(w, id, o::D) - res.damage;
        c::set_pf(w, id, o::D, hp);
        let mut reaction = res.reaction;
        if hit.and_then(|h| h.attacker).filter(|&a| a < w.table.mobys.len()).is_some_and(|a| w.m(a).o_class == 0x47) { reaction = 3; }
        if hp <= 0.0 { reaction = 1; }
        let kr = o::K;
        c::set_pi32(w, id, kr + knock::k::RADIUS, 512);
        c::set_pf(w, id, kr + knock::k::ZOFF, 0.5);
        c::set_pi32(w, id, kr + knock::k::FLAGS, 9);
        c::set_pf(w, id, kr + knock::k::GRAVITY, k::G_HURT * DT2);
        c::set_pu8(w, id, kr + 0x3d, 0);
        let dir = hit.map_or([0.0; 4], |h| h.dir.map(|x| f32::from_bits(x.0)));
        let fly = |w: &mut World, g: f32, sp: f32, up: f32, drag: f32, apex: f32, land: f32, s: u8, next: u8| {
            c::set_pf(w, id, kr + knock::k::GRAVITY, g * DT2);
            c::set_pf(w, id, kr + knock::k::SPEED, sp * DT);
            c::set_pf(w, id, kr + knock::k::UP, up * DT);
            c::set_pf(w, id, kr + knock::k::DRAG, drag * DT);
            c::set_pf(w, id, kr + knock::k::KEY_APEX, apex);
            c::set_pf(w, id, kr + knock::k::KEY_LAND, land);
            let (mut a, mut b) = (c::pf(w, id, kr + knock::k::SPEED), c::pf(w, id, kr + knock::k::UP));
            let h = knock::aim(dir, &mut a, &mut b);
            c::set_pf(w, id, kr + knock::k::SPEED, a);
            c::set_pf(w, id, kr + knock::k::UP, b);
            knock::start(w, id, kr, h, s, 1, 0);
            set_st(w, id, next);
            w.mm(id).cmd = 2;
        };
        match reaction {
            1 | 2 => {
                w.mm(id).mode &= !mode::TARGETABLE;
                fly(w, k::G_DEAD, k::SPEED_DEAD, k::UP_DEAD, k::DRAG_DEAD, 8.0, 16.0, 8, 0x17);
                c::set_pu8(w, id, o::F + 7, 0xf0);
            }
            3..=8 => {
                fly(w, k::G_HURT, k::SPEED_HURT, k::UP_HURT, k::DRAG_HURT, 5.0, 10.0, 7, 0x15);
                c::set_pu8(w, id, o::F + 7, 0x78);
            }
            9 | 10 => c::set_pu8(w, id, o::F + 7, 0xfa),
            _ => {}
        }
        flash::start(w, id, o::F);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, o::F);
    if culled { return; }
    if c::pi32(w, id, o::IDLE) != 0 {
        let r = w.rng.randf(180.0, 240.0);
        let t = w.svc.timing.scale(Pf::f(r)).to_i32();
        c::set_pi32(w, id, o::IDLE, 0);
        c::set_pf(w, id, o::IDLE_T, t as f32);
    }
    let it = (c::pf(w, id, o::IDLE_T) - 1.0).max(0.0);
    c::set_pf(w, id, o::IDLE_T, it);
    let p = c::pos(w, id);
    let b = body(w);
    let sees = if c::dist3(p, b) < 64.0 {
        let barrier = link(w, c::pi32(w, id, o::BARRIER_A));
        w.coll_line(pv([p[0], p[1], p[2] + 1.0, p[3]]), pv(b), 2, barrier).is_none()
    } else {
        false
    };
    c::set_pu8(w, id, o::SEES, sees as u8);
    let area = usize::try_from(c::pi32(w, id, o::AREA)).ok();
    let t = target::acquire_in(w, id, 128.0, area);
    c::set_pv4(w, id, o::TGT, t.pos);
    c::set_pv4(w, id, o::TGT + 0x10, t.rot);
    c::set_pv4(w, id, o::TGT + 0x20, t.aim);
    c::set_pv4(w, id, o::AIM, t.body);
    c::set_pi32(w, id, o::HERO, t.moby.map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, o::ALERT, t.kind as i32);
    if t.kind == 2 {
        c::set_pu8(w, id, o::LOOKS, 0);
        c::set_pu8(w, id, o::LOST, 1);
        c::set_pi16(w, id, o::WARY, 0);
    } else {
        let tp = t.pos;
        let ahead = c::diff_rots(w.m(id).rotation[2], c::atan(tp[0] - p[0], tp[1] - p[1])) < FRAC_PI_2
            && (w.hero.f658 == 0 || (p[2] - tp[2]).abs() < 4.0);
        let seen = if ahead || matches!(st(w, id), 0x10 | 0x11) { sees } else { false };
        let alert = c::pi32(w, id, o::ALERT);
        if alert == 1 {
            let v = c::pu8(w, id, o::LOST) + 1;
            c::set_pu8(w, id, o::DOUBT, v);
        } else if w.body() == DISGUISED {
            if 1 < c::pu8(w, id, o::DOUBT) { c::set_pu8(w, id, o::LOST, 1); }
        } else {
            c::set_pu8(w, id, o::DOUBT, 0);
        }
        c::set_pu8(w, id, o::LOOKS, 1);
        if !seen {
            if !sees { c::set_pu8(w, id, o::LOST, 1); }
            c::set_pu8(w, id, o::LOOKS, 0);
            c::set_pi32(w, id, o::ALERT, 2);
            c::set_pi16(w, id, o::WARY, 0);
        } else if ((c::pi16(w, id, o::TOLD) == 0 || c::pu8(w, id, o::LOST) != 0) && w.body() == DISGUISED)
            || (c::pu8(w, id, o::DOUBT) != 0 && c::pi32(w, id, o::ALERT) != 1)
        {
            c::set_pi32(w, id, o::ALERT, 2);
            c::set_pi16(w, id, o::WARY, 1);
        } else if w.body() != DISGUISED && c::pi32(w, id, o::ALERT) != 2 {
            c::set_pu8(w, id, o::LOST, 0);
        }
        if w.svc.game_mode == 2 { c::set_pi32(w, id, o::ALERT, 2); }
    }
    if c::pi32(w, id, o::HERO) == 0 {
        if let Some(h) = w.hero_moby { c::set_pi32(w, id, o::HERO, h as i32 + 1); }
    }
}

/// `0x29a2a8`: the head (list 1) and body (list 0) look-at toward what it watches (the bait, the caller, Ratchet),
/// the head ±60° (±90° aiming), up / down 30°.
pub fn look(w: &mut World, id: MobyId) {
    let at = if c::pi16(w, id, o::BAIT_SEEN) != 0 && ptr(w, id, o::BAIT).is_some() {
        ptr(w, id, o::BAIT).map(|m| c::pos(w, m))
    } else if c::pi16(w, id, o::CALL_T) != 0 && ptr(w, id, o::CALLER).is_some() {
        ptr(w, id, o::CALLER).map(|m| c::pos(w, m))
    } else if c::pu8(w, id, o::LOOKS) != 0 {
        Some(c::pv4(w, id, o::AIM))
    } else {
        None
    };
    let aiming = matches!(st(w, id), 0x10 | 0x11);
    let (mut yaw, mut pitch) = (0.0f32, 0.0f32);
    if let Some(q) = at {
        let p = c::pos(w, id);
        let d = [q[0] - p[0], q[1] - p[1], q[2] - (p[2] + 1.0), 0.0];
        let rel = c::sub_rot(c::atan(d[0], d[1]), w.m(id).rotation[2]);
        let lim = if aiming { FRAC_PI_2 } else { std::f32::consts::FRAC_PI_3 };
        yaw = rel.clamp(-lim, lim);
        pitch = (-c::atan(c::len2(d), d[2])).clamp(-std::f32::consts::FRAC_PI_6, std::f32::consts::FRAC_PI_6);
    }
    let (head, body_yaw) = if aiming {
        let b = yaw.clamp(-std::f32::consts::FRAC_PI_3, std::f32::consts::FRAC_PI_3);
        (yaw - b, b)
    } else {
        (yaw * 0.7, yaw * 0.3)
    };
    c::set_pf(w, id, o::LOOK_A + 0x68, head);
    c::set_pf(w, id, o::LOOK_A + 0x64, pitch);
    c::set_pf(w, id, o::LOOK_B + 0x68, body_yaw);
    let s2 = SPEED * SPEED;
    manip::look(w, id, id, o::LOOK_A, 1, s2 * 0.02, 0.3 * s2);
    manip::look(w, id, id, o::LOOK_B, 0, s2 * 0.02, 0.3 * s2);
}

fn table_of(s: u8) -> Option<&'static [u32]> {
    match s {
        10 => Some(&TURN_LEFT),
        11 => Some(&TURN_RIGHT),
        12 => Some(&TURN_BACK),
        _ => None,
    }
}

/// The last key frame's time of sequence `s` (÷ 16).
fn last_key(w: &World, id: MobyId, s: u8) -> f32 {
    let Some(class) = w.classes.anim(w.m(id).o_class) else { return 0.0 };
    let Some(sq) = class.sequences.get(s as usize).and_then(|x| x.as_ref()) else { return 0.0 };
    class.frame(s, sq.header.frame_count.wrapping_sub(1)).map_or(0.0, |fr| fr.header.time as i32 as f32 * 0.0625)
}

/// The shots of 0x11 / 0x13 (`list` 3 / 2, the arm's row 1 times `side`·8·dt).
fn shoot(w: &mut World, id: MobyId, list: usize, side: f32) {
    if c::pu8(w, id, o::SHOT) != 0 { return; }
    let kt = c::ground::key_time(w, id);
    if !(0.85..3.0).contains(&kt) { return; }
    let m = w.joint_matrix(id, list);
    let jp = [m[3][0], m[3][1], m[3][2], 1.0];
    let a = c::set_len3([m[1][0], m[1][1], m[1][2], 0.0], side * k::SHOT * DT);
    let aim = c::pv4(w, id, o::AIM);
    let b = c::set_len3([aim[0] - jp[0], aim[1] - jp[1], aim[2] - jp[2], 0.0], k::SHOT * DT);
    let ok = c::diff_rots(c::atan(a[0], a[1]), c::atan(b[0], b[1])) <= FRAC_PI_4
        && c::diff_rots(c::atan(c::len2(a), a[2]), c::atan(c::len2(b), b[2])) <= FRAC_PI_4;
    fire(w, id, if ok { b } else { a }, jp);
    c::set_pu8(w, id, o::SHOT, 1);
}

/// Level15 `0x2e4580(m, dir, pos, aim)`: a shot 934 (module doc).
pub fn fire(w: &mut World, owner: MobyId, dir: c::V, at: c::V) -> Option<MobyId> {
    let id = w.create_moby(SHOT_CLASSES[0])?;
    story::pvars(w, id, 0x40);
    let (light, ambient) = (w.m(owner).light, w.m(owner).ambient);
    let t = w.ticks(0xf0);
    {
        let m = w.mm(id);
        m.update_dist = 0xff;
        m.draw_dist = 0xff;
        m.visible = 1;
        m.state = 0;
        m.scale *= 20.0;
        m.light = light;
        m.ambient = ambient;
        m.position = at;
        m.alpha = 0x30;
        m.mode |= 0x200;
        m.rotation[2] = c::atan(dir[0], dir[1]);
        m.rotation[1] = -c::atan(c::len2(dir), dir[2]);
    }
    c::set_pv4(w, id, 0, dir.map(|x| x * 5.0));
    c::set_pi32(w, id, 0x38, t);
    c::set_pi32(w, id, 0x3c, owner as i32 + 1);
    w.build_matrix(id);
    crate::moby_update::creature::projectile::part26_joint(w, 42_000.0, id, 0x6000_40ff, 0x80, -1);
    Some(id)
}

/// Level15 `0x2e4380` (module doc).
pub fn shot_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x40 {
        w.delete_moby(id);
        return;
    }
    if st(w, id) != 0 { return; }
    let r = c::add_rot(w.m(id).rotation[0], DT * TAU);
    w.mm(id).rotation[0] = r;
    let p = c::pos(w, id);
    let v = c::pv4(w, id, 0);
    let np = [p[0] + v[0], p[1] + v[1], p[2] + v[2], p[3]];
    let tmpl = HitTemplate { dir: [pf(v[0]), pf(v[1]), Pf::ONE, pf(v[3])], attacker: Some(id), flags: 1, h1a: w.m(id).o_class as u16, damage: Pf::f(2.0), w20: 1, ..Default::default() };
    let owner = ptr(w, id, 0x3c);
    let line = crate::moby_update::services::line_hit_in(w.table, w.svc, w.classes, w.coll, pv(p), pv(np), 0, Some(id), &tmpl);
    let (hit, hm) = match line {
        Some(h) => (true, h.moby),
        None => match w.coll_sphere(pv(p), Pf::f(0.3), 0, Some(id)) {
            Some(h) => (true, h.moby),
            None => (c::dec_timer_pvar_i32(w, id, 0x38) != 0, None),
        },
    };
    if !hit || hm.is_none() || hm == owner {
        w.mm(id).position = np;
        if w.counter.is_multiple_of(3) { super::umbris_beast_fx::part61(w, 30_000.0, 0.1, id, 0); }
        return;
    }
    if hm.is_some_and(|m| w.m(m).o_class != BARRIER_CLASS) {
        crate::moby_update::creature::attack::sphere_hit(w, 0.5, 2.0, 0.0, id, p, 1, 0, 1, 0);
    }
    fx::death_explosion(w, 0.5, 13.0, Some(id), p, -1);
    w.play_sound(0, 0, id);
    w.delete_moby(id);
}

/// Level15 `0x2979d8` (module doc).
#[allow(clippy::too_many_lines)]
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    if w.svc.game_mode == 2 {
        w.mm(id).mode |= 0x41;
        return;
    }
    let cam = w.camera_point();
    let cam = [cam[0], cam[1], cam[2]];
    let cam_in = |w: &World, cub: i32| w.in_cuboid(cam, cub);
    let culled = if c::pi32(w, id, o::CUB) == -1 {
        let mut u = false;
        if c::pi32(w, id, o::CUB + 8) != -1 {
            u = cam_in(w, c::pi32(w, id, o::CUB + 8));
            if c::pi32(w, id, o::CUB + 0xc) != -1 { u |= cam_in(w, c::pi32(w, id, o::CUB + 0xc)); }
        }
        u
    } else {
        let mut u = !cam_in(w, c::pi32(w, id, o::CUB));
        if c::pi32(w, id, o::CUB + 4) != -1 && cam_in(w, c::pi32(w, id, o::CUB + 4)) { u = false; }
        u
    };
    senses(w, id, culled);
    if w.m(id).state >= 0xfd { return; }
    if culled && !matches!(st(w, id), 0x15 | 0x17) {
        let m = w.mm(id);
        m.has_collision = false;
        m.mode = m.mode & 0xefff | 0x41;
        return;
    }
    let coll = super::class_collision(w, w.m(id).o_class);
    {
        let m = w.mm(id);
        m.mode = m.mode & 0xffbe | 0x1000;
        m.has_collision = coll;
    }
    manip::big_head_scale(w, 2.5, id, o::LOOK_A);
    if w.m(id).visible != 0 && c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 28.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0x16;
    }
    if hero_state(w) == HOLOGUISE_STATE { c::set_pi16(w, id, o::TOLD, 0); }
    for t in [o::T154, o::TOLD, o::CALL_T] { c::dec_timer_pvar_s16(w, id, t); }
    if c::pi16(w, id, o::TOLD) != 0 {
        if let Some(a) = alarm_of(w, id).filter(|&a| alarm_on(w, a)) { alarm::set_off(w, a, 0xb4); }
    }
    if w.m(id).cmd == 0 { c::set_pu8(w, id, o::ALERTED, 0); }
    let hp = hero_pos_of(w, id);
    let s = st(w, id);
    match s {
        0 => {
            let area = path_pts(w, c::pi32(w, id, o::AREA));
            if area.is_empty() {
                w.delete_moby(id);
                return;
            }
            c::set_pi16(w, id, o::D + 4, 3);
            c::set_pf(w, id, o::D, 3.0);
            c::set_pu8(w, id, o::D + 8, 2);
            c::set_pu8(w, id, o::D + 9, 0);
            let (pos, yaw) = (c::pos(w, id), w.m(id).rotation[2]);
            c::set_pv4(w, id, o::HOME, [pos[0], pos[1], pos[2], yaw]);
            if c::pi32(w, id, o::WAIT_SECS) < 5 { c::set_pi32(w, id, o::WAIT_SECS, 5); }
            c::set_pi32(w, id, o::ADVANCE, 1);
            set_st(w, id, 2);
            w.mm(id).mode |= 0x1000;
            if (0..4).any(|k| c::pi32(w, id, o::LINKS + 4 * k) != -1) { set_st(w, id, 1); }
            w.mm(id).cmd = 0;
        }
        1 => {
            let up = (0..4).any(|k| link(w, c::pi32(w, id, o::LINKS + 4 * k)).is_some_and(|m| w.m(m).state >= 2));
            if up { set_st(w, id, 2); }
        }
        2 | 3 => patrol(w, id, hp),
        4 => {
            let pt = patrol_point(w, id);
            if walk_to(w, id, DT + DT, pt) {
                set_st(w, id, 2);
                c::set_pi32(w, id, o::ADVANCE, 1);
                seq(w, id, 0, 10);
            } else if c::pi32(w, id, o::ALERT) == 2 {
                on_way(w, id, hp, 4);
            } else {
                spotted(w, id);
            }
        }
        5 => {
            if wrapped(w, id) {
                let t = w.ticks(0xf0);
                c::set_pi16(w, id, o::T154, t as i16);
                for b in [o::BARRIER_B, o::BARRIER_A] {
                    if let Some(m) = link(w, c::pi32(w, id, b)) { super::barrier_field::shut_down(w, m); }
                    c::set_pi32(w, id, b, -1);
                }
                let r = c::pu8(w, id, o::RET_STATE);
                set_st(w, id, r);
                match r {
                    2 => seq(w, id, 0, 10),
                    4 => seq(w, id, 0xd, 10),
                    _ => {}
                }
            }
        }
        6..=8 => {
            if c::pi32(w, id, o::ALERT) != 2 {
                let t = c::pv4(w, id, o::TGT);
                let p = c::pos(w, id);
                if c::diff_rots(w.m(id).rotation[2], c::atan(t[0] - p[0], t[1] - p[1])) < std::f32::consts::FRAC_PI_6 {
                    spotted(w, id);
                    return tail(w, id);
                }
            }
            let a = w.m(id).anim;
            let tab = table_of(a.seq_a);
            let from = c::pf(w, id, o::TURN_FROM);
            if !wrapped(w, id) {
                if a.seq_a == a.seq_b {
                    if let Some(tb) = tab {
                        let kt = c::ground::key_time(w, id);
                        let i = (kt as i32).clamp(0, tb.len() as i32 - 2) as usize;
                        let (t0, t1) = (f32::from_bits(tb[i]), f32::from_bits(tb[i + 1]));
                        let d = (t0 + (t1 - t0) * (kt - i as f32)) * 0.017_453_292;
                        w.mm(id).rotation[2] = c::add_rot(from, d);
                    }
                }
            } else {
                let i = last_key(w, id, a.seq_a) as i32;
                if let Some(tb) = tab {
                    let i = i.clamp(0, tb.len() as i32 - 1) as usize;
                    w.mm(id).rotation[2] = c::add_rot(from, f32::from_bits(tb[i]) * 0.017_453_292);
                }
                let r = c::pu8(w, id, o::RET_STATE);
                set_st(w, id, r);
                let q = c::pu8(w, id, o::RET_SEQ);
                seq(w, id, q, 10);
            }
        }
        9 => {
            let home = c::pv4(w, id, o::HOME);
            if 0.2 < c::dist3(c::pos(w, id), [home[0], home[1], home[2], 0.0]) {
                walk_to(w, id, DT + DT, home);
            } else if c::pi32(w, id, o::PATROL) == -1 {
                turn_to(w, id, c::pf(w, id, o::HOME_YAW), 2, 0);
            } else {
                w.mm(id).cmd = 0;
                c::set_pi32(w, id, o::ADVANCE, 0);
                let pt = patrol_point(w, id);
                face(w, id, pt, 2, 0);
            }
        }
        10 => {
            if c::pi32(w, id, o::ALERT) == 2 {
                if wrapped(w, id) {
                    set_st(w, id, 2);
                    seq(w, id, 0, 10);
                    let t = w.ticks(0x168);
                    c::set_pi16(w, id, o::BAIT_T, t as i16);
                    c::set_pi16(w, id, o::BAIT_SEEN, 0);
                }
            } else {
                spotted(w, id);
            }
        }
        0xb => {
            if let Some(a) = alarm_of(w, id) { alarm::poke(w, a); }
            if wrapped(w, id) {
                if hero_state(w) == HOLOGUISE_STATE {
                    set_st(w, id, 0xc);
                    seq(w, id, 0, 10);
                } else {
                    tell_group(w, id);
                    let to_alarm = alarm_of(w, id).is_some_and(|a| !alarm_on(w, a)) && go_alarm(w, id);
                    if to_alarm {
                        set_st(w, id, 0xe);
                        seq(w, id, 0xe, 0x14);
                    } else {
                        let r = c::pu8(w, id, o::RET_STATE);
                        if r == 0x10 {
                            face(w, id, hp, 0x10, 3);
                        } else {
                            if r != 0xff { set_st(w, id, r); }
                            let q = c::pu8(w, id, o::RET_SEQ);
                            if q != 0xff { seq(w, id, q, 10); }
                        }
                    }
                }
            }
        }
        0xc => {
            if hero_state(w) == HOLOGUISE_STATE {
                set_st(w, id, 9);
                seq(w, id, 0xd, 10);
                if let Some(a) = alarm_of(w, id).filter(|&a| alarm_on(w, a)) { alarm::set_off(w, a, -1); }
            } else if c::pi32(w, id, o::ALERT) == 2 || c::pu8(w, id, o::LOST) != 0 {
                calm_or_told(w, id, hp);
            } else {
                let t = c::pv4(w, id, o::TGT);
                face(w, id, t, 0x10, 3);
            }
        }
        0xd => {
            let pt = patrol_point(w, id);
            if walk_to(w, id, DT * 3.5, pt) {
                set_st(w, id, 0xc);
                c::set_pi32(w, id, o::ADVANCE, 1);
                seq(w, id, 0, 10);
            } else if c::pi32(w, id, o::ALERT) != 2 && c::pu8(w, id, o::LOST) == 0 {
                spotted(w, id);
            } else if c::pi16(w, id, o::TOLD) == 0 {
                stand_down(w, id);
            } else {
                let t = w.ticks(0x3c);
                c::set_pi32(w, id, o::WAIT_T, t);
            }
        }
        0xe => {
            let Some(a) = alarm_of(w, id) else { return tail(w, id) };
            alarm::poke(w, a);
            let p = c::pos(w, id);
            let d = c::sub(c::pos(w, a), p);
            let l = c::len3(d);
            let to = c::add(c::set_len3(d, l - 1.5), p);
            if walk_to(w, id, DT * 4.0, to) {
                set_st(w, id, 0xf);
                seq(w, id, 9, 10);
            }
        }
        0xf => {
            let a = alarm_of(w, id);
            let an = w.m(id).anim;
            if an.seq_a == an.seq_b && 12.0 <= c::ground::key_time(w, id) {
                if let Some(a) = a { alarm::set_off(w, a, 0xb4); }
            }
            if wrapped(w, id) {
                if let Some(a) = a { alarm::set_off(w, a, 0xb4); }
                face(w, id, hp, 0x14, 0xe);
            }
        }
        0x10 | 0x12 => {
            c::set_pi16(w, id, o::CALL_T, 0);
            c::set_pi16(w, id, o::BAIT_SEEN, 0);
            tell_group(w, id);
            if wrapped(w, id) {
                if hero_state(w) == HOLOGUISE_STATE {
                    set_st(w, id, 0xc);
                    seq(w, id, 0, 10);
                } else {
                    set_st(w, id, s + 1);
                    c::set_pu8(w, id, o::SHOT, 0);
                    seq(w, id, if s == 0x10 { 4 } else { 6 }, 10);
                }
            }
        }
        0x11 | 0x13 => {
            c::set_pi16(w, id, o::CALL_T, 0);
            c::set_pi16(w, id, o::BAIT_SEEN, 0);
            tell_group(w, id);
            if s == 0x11 { shoot(w, id, 3, -1.0) } else { shoot(w, id, 2, 1.0) }
            if wrapped(w, id) { after_shot(w, id, s); }
        }
        0x14 => {
            let home = c::pv4(w, id, o::HOME);
            if 0.2 < c::dist3(c::pos(w, id), [home[0], home[1], home[2], 0.0]) {
                walk_to(w, id, DT * 3.0, home);
            } else if c::pi16(w, id, o::TOLD) == 0 {
                w.mm(id).cmd = 0;
                c::set_pi32(w, id, o::ADVANCE, 0);
                if c::pi32(w, id, o::PATROL) == -1 {
                    turn_to(w, id, c::pf(w, id, o::HOME_YAW), 2, 0);
                } else {
                    let pt = patrol_point(w, id);
                    face(w, id, pt, 2, 0);
                }
            } else {
                let t = c::pv4(w, id, o::TGT);
                let p = c::pos(w, id);
                if FRAC_PI_4 < c::diff_rots(w.m(id).rotation[2], c::atan(t[0] - p[0], t[1] - p[1])) && c::pu8(w, id, o::LOST) == 0 {
                    face(w, id, t, 0x10, 3);
                } else {
                    set_st(w, id, 0xc);
                    seq(w, id, 0, 10);
                }
            }
        }
        0x15 | 0x17 => {
            let r = knock::update(w, id, o::K);
            let landed = if s == 0x15 { r & 0x40 != 0 } else { r & 0x160 != 0 };
            if s == 0x15 && r & 0x40 == 0 && r & 0x120 != 0 {
                w.delete_moby(id);
                return;
            }
            if !landed { return tail(w, id); }
            if s == 0x15 {
                let p = c::pos(w, id);
                let g = w.coll_line(pv([p[0], p[1], p[2] + 0.5, p[3]]), pv([p[0], p[1], 0.01, p[3]]), 2, None);
                let plain = g.is_none_or(|o| o.kind < 0 || o.kind & 0x1f == 0x1f);
                if plain && c::pf(w, id, o::HOME + 8) - 5.0 < p[2] {
                    if hero_state(w) == HOLOGUISE_STATE {
                        set_st(w, id, 0xc);
                        seq(w, id, 0, 10);
                    } else {
                        spotted(w, id);
                    }
                    return;
                }
            }
            for b in [o::BARRIER_A, o::BARRIER_B] {
                if let Some(m) = link(w, c::pi32(w, id, b)) { super::barrier_field::shut_down(w, m); }
                c::set_pi32(w, id, b, -1);
            }
            w.play_sound(0xd, 0, id);
            let p = c::pos(w, id);
            fx::death_explosion(w, 0.5, 13.0, Some(id), p, -1);
            set_death_bits(w, id, 0, -1);
            w.delete_moby(id);
            return;
        }
        _ => {}
    }
    tail(w, id);
}

/// States 2 / 3 (module doc).
fn patrol(w: &mut World, id: MobyId, hp: c::V) {
    if c::pi32(w, id, o::ALERT) != 2 {
        spotted(w, id);
        return;
    }
    if c::pi16(w, id, o::TOLD) != 0 && c::pu8(w, id, o::LOST) == 0 {
        if c::pu8(w, id, o::SEES) != 0 || c::pi32(w, id, o::PATROL) == -1 {
            face(w, id, hp, 0x10, 3);
            return;
        }
        advance(w, id);
        told(w, id);
        return;
    }
    if c::pi16(w, id, o::CALL_T) != 0 {
        if let Some(m) = ptr(w, id, o::CALLER) { let q = c::pos(w, m); face(w, id, q, 10, 0xf); }
        return;
    }
    if c::dec_timer_pvar_s16(w, id, o::BAIT_T) != 0 {
        let b = find_bait(w, id);
        c::set_pi32(w, id, o::BAIT, b.map_or(0, |m| m as i32 + 1));
        if let Some(m) = b {
            let go = c::pi16(w, id, o::BAIT_SEEN) != 0 || { let t = w.ticks(0x96); w.rng.randi(t) == 0 };
            if go {
                let q = c::pos(w, m);
                face(w, id, q, 10, 0xf);
                return;
            }
        }
    }
    let s = st(w, id);
    match s {
        2 => {
            if c::pi32(w, id, o::PATROL) == -1 {
                if c::pi16(w, id, o::WARY) == 0 || hero_state(w) != TERMINAL_STATE {
                    let y = c::pf(w, id, o::HOME_YAW);
                    turn_to(w, id, y, 2, 0);
                    return;
                }
                let barriers = c::pi32(w, id, o::BARRIER_B) != -1 || c::pi32(w, id, o::BARRIER_A) != -1;
                if !barriers && (c::pi16(w, id, o::T154) != 0 || w.rng.randi(0x78) != 0) { return; }
                c::set_pu8(w, id, o::RET_SEQ, 0);
                c::set_pu8(w, id, o::RET_STATE, 2);
                set_st(w, id, 5);
                seq(w, id, 2, 10);
            } else if c::pi32(w, id, o::ADVANCE) == 0 || my_turn(w, id) {
                let t = w.ticks(c::pi32(w, id, o::WAIT_SECS));
                c::set_pi32(w, id, o::WAIT_T, t);
                set_st(w, id, 3);
            }
        }
        3 if c::dec_timer_pvar_i32(w, id, o::WAIT_T) != 0 => {
            advance(w, id);
            let pt = patrol_point(w, id);
            face(w, id, pt, 4, 0xd);
        }
        _ => {}
    }
}

/// State 4 calm, not there yet (module doc).
fn on_way(w: &mut World, id: MobyId, hp: c::V, _from: u8) {
    if c::pi16(w, id, o::TOLD) != 0 && c::pu8(w, id, o::LOST) == 0 {
        if c::pu8(w, id, o::SEES) != 0 {
            face(w, id, hp, 0x10, 3);
            return;
        }
        let t = w.ticks(0x3c);
        c::set_pi32(w, id, o::WAIT_T, t);
        told(w, id);
        return;
    }
    if c::pi16(w, id, o::CALL_T) != 0 {
        if let Some(m) = ptr(w, id, o::CALLER) { let q = c::pos(w, m); face(w, id, q, 10, 0xf); }
        return;
    }
    if c::pi16(w, id, o::WARY) != 0 && hero_state(w) == TERMINAL_STATE {
        let barriers = c::pi32(w, id, o::BARRIER_B) != -1 || c::pi32(w, id, o::BARRIER_A) != -1;
        if !barriers && c::pi16(w, id, o::T154) != 0 { return; }
        c::set_pu8(w, id, o::RET_SEQ, 0xd);
        c::set_pu8(w, id, o::RET_STATE, 4);
        set_st(w, id, 5);
        seq(w, id, 2, 10);
        return;
    }
    if c::dec_timer_pvar_s16(w, id, o::BAIT_T) == 0 { return; }
    let b = find_bait(w, id);
    c::set_pi32(w, id, o::BAIT, b.map_or(0, |m| m as i32 + 1));
    let Some(m) = b else { return };
    if c::pi16(w, id, o::BAIT_SEEN) == 0 {
        let t = w.ticks(0x96);
        if w.rng.randi(t) != 0 { return; }
    }
    let q = c::pos(w, m);
    face(w, id, q, 10, 0xf);
}

/// State 0xc calm or told (module doc).
fn calm_or_told(w: &mut World, id: MobyId, hp: c::V) {
    if c::pi16(w, id, o::TOLD) == 0 {
        stand_down(w, id);
        return;
    }
    if c::pu8(w, id, o::SEES) != 0 {
        if c::pi32(w, id, o::ALERT) != 2 && c::pu8(w, id, o::LOST) == 0 {
            face(w, id, hp, 0x10, 3);
        } else {
            face(w, id, hp, 0xc, 0);
        }
        return;
    }
    if c::pi32(w, id, o::PATROL) == -1 { return; }
    advance(w, id);
    let t = w.ticks(0x3c);
    c::set_pi32(w, id, o::WAIT_T, t);
    told(w, id);
}

/// Back to its post (state 9, sequence 13) with its alarm stopped after `ticks(30)`.
fn stand_down(w: &mut World, id: MobyId) {
    set_st(w, id, 9);
    seq(w, id, 0xd, 10);
    if let Some(a) = alarm_of(w, id).filter(|&a| alarm_on(w, a)) {
        let t = w.ticks(0x1e);
        alarm::set_off(w, a, t);
    }
}

/// The end of a shot (0x11 / 0x13 wrapped).
fn after_shot(w: &mut World, id: MobyId, s: u8) {
    if hero_state(w) == HOLOGUISE_STATE {
        set_st(w, id, 0xc);
        if w.m(id).anim.seq_b != 0 {
            seq(w, id, 0, 10);
            c::set_pu8(w, id, o::SHOT, 0);
        }
        return;
    }
    let (next, nseq) = if s == 0x11 { (0x12, 5) } else { (0x10, 3) };
    let at_alarm = alarm_of(w, id).is_some_and(|a| !alarm_on(w, a)) && go_alarm(w, id);
    if at_alarm {
        set_st(w, id, 0xe);
        seq(w, id, 0xe, 0x14);
        c::set_pu8(w, id, o::SHOT, 0);
        return;
    }
    let t = c::pv4(w, id, o::TGT);
    let p = c::pos(w, id);
    if FRAC_PI_4 < c::diff_rots(w.m(id).rotation[2], c::atan(t[0] - p[0], t[1] - p[1])) {
        face(w, id, t, next, nseq);
        c::set_pu8(w, id, o::SHOT, 0);
        return;
    }
    if c::pi32(w, id, o::ALERT) == 2 {
        set_st(w, id, 0xc);
        if w.m(id).anim.seq_b != 0 {
            seq(w, id, 0, 10);
            c::set_pu8(w, id, o::SHOT, 0);
        }
        return;
    }
    set_st(w, id, next);
    seq(w, id, nseq, 10);
    c::set_pu8(w, id, o::SHOT, 0);
}

/// The tail: the look, the glow, the eyes and their draw (module doc).
fn tail(w: &mut World, id: MobyId) {
    look(w, id);
    let told = c::pi16(w, id, o::TOLD) != 0;
    let calm = c::pi32(w, id, o::ALERT) == 2;
    let (level, stamp) = if told { (2u16, w.counter as u32) } else if calm { ((c::pi16(w, id, o::WARY) != 0) as u16, w.counter as u32) } else { (2, 0) };
    let (lw, sw) = GUARD_WORDS;
    if stamp != w.svc.units.word(sw) || (w.svc.units.word(lw) as i16) < level as i16 {
        w.svc.units.set_word(lw, level as u32);
        w.svc.units.set_word(sw, stamp);
    }
    let per = w.ticks(match level { 2 => 0x32, 1 => 0x5a, _ => 0xaa }).max(1);
    let f = (w.counter % per as u64) as f32 / per as f32;
    let s = ((f + f) * PI - PI).sin();
    let (g, r, b) = ((s * 20.0) as i32, (s * 70.0) as i32, (s * 10.0) as i32);
    let rr = (r + 0xb4).min(0xff) as u32;
    let gg = (g + 0x32).min(0xff) as u32;
    let bb = (b + 0x14) as u32;
    let col = if level == 2 { bb << 16 | 0x8000_0000 | (gg / 2) << 8 | rr } else { bb << 16 | 0x8000_0000 | rr << 8 | gg };
    let gcol = crate::hud::tween_color(0.1, w.m(id).glow, col);
    w.mm(id).glow = gcol;
    for k in 0..3 {
        let j = w.joint_point(id, 4 + k);
        c::set_pv4(w, id, o::EYES + 0x10 * k, j);
    }
    if let Some(row) = super::row(REFERENCE_LEVEL, DRAW_FN) { w.svc.draw_callbacks.register(Callback::UnitGlow(row), id); }
}

/// Level15 `0x29aba8`: three glows (0.2, pull 0.08) on the eyes, the glow colour at alpha 0x30.
pub fn glow_quads(table: &MobyTable, _svc: &Services, id: MobyId) -> Vec<GlowQuad> {
    let Some(m) = table.mobys.get(id).filter(|m| m.pvars.len() >= o::SIZE) else { return Vec::new() };
    (0..3).map(|k| {
        let q = p::v4f(&m.pvars, o::EYES + 0x10 * k);
        GlowQuad { size: 0.2, pull: 0.08, point: [q[0], q[1], q[2]], rgba: (m.glow & 0xff_ffff) | 0x3000_0000 }
    }).collect()
}
