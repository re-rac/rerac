//! **The zapper bots, class 28** (level14 `0x2b17d8`; census U490, 21 placed on Oltanis and 37 on Quartu), and the
//! pieces they leave, **325** (`0x2de670`) and **403** (`0x2dfd10`). Read from the level14 decomp and disassembly;
//! native `f32`.
//!
//! A bot that hovers in its area (the path +0x244), turns to its target (`0x274df8` within 40, 46 for a while after a
//! hit) and sweeps an arc across it (the shared arc slots, [`super::oltanis_arcs`]). With +0x24a set it patrols: it
//! waits hidden in its hatch (the moby +0x264, class 250) and rises out of it along its path +0x240 when its target is
//! near and no other bot of its group is rising; with a path but no patrol it follows the path back and forth.
//!
//! * **Every tick**: the big-head cheat (list 2, 2.5). A Clank-only bot (+0x288) away from Clank (0x1413f4 ≠ 2):
//!   hidden without collision (shown again for him, update distance 0x40); hidden for good once its sequence B is 5
//!   (its arc let go, its death bits). From state 1 on: its hits (`0x2b2f60`), its target (`0x2b3268`), its range
//!   (`0x2b2ed8`). In a scene (game mode 2) hidden, else (not in state 1) shown.
//! * **Hits** (`0x2b2f60`, mask 0x330000, the resolver column 4; a hit by its own class or 0x370 counts as none):
//!   health −= damage; dead (reaction 1 / 2): state 0xc for a 0x800000 hit (else 0xb), not targetable, the
//!   knockback (radius = the old height ·1024, height 0.3, speed and up 10·dt, keys 9 / 15, sequence 8 in
//!   `ticks(5)`, along the hit), red 0xfa, its death bits (0x200 for Clank), no collision; stunned (3..10): its arc let
//!   go, state 10, +0x26 `ticks(60)`, the knock start (keys 11 / 29, speed 8·dt, up 3·dt, sequence 7), red 0xfa; the
//!   flash.
//! * **States**: 0 the start (paths checked, the damage record: health 2, its path's chords, the manipulators on
//!   lists 2 and 3, the ground; → 1 hidden (patrol), 4 or 0xd); 1 in the hatch (a target, `ticks(60)` waited, the
//!   hatch open (`0x2d9870`), no group bot rising → shown, 2, sequence 1); 2 rising (the path past key time 3) → 3;
//!   3 along the path → 4; 4 ready: drifting (×0.92 a tick), turning to the target; no target → 8 back along the
//!   path; else after its cool-down, with fewer than two of its group zapping, an arc slot, one in four, facing it
//!   within 1° and its head within 1°, no wall between → the group's cool-downs `ticks(45)`, 5, sequence 4, the
//!   sweep `ticks(120)`; 5 wind-up (head 30° left) → 6; 6 the sweep: the head from −30° to +30° (and down at a
//!   target below it with +0x24b), the arc from joint 0 along its row 1 for 25 (cut at what it hits; that gets
//!   damage 1, flags 0x10001), → 7 at the end (the arc let go); 7 → 4; 8 back along the path → 0xd (or, patrolling,
//!   into the hatch: 9 → 1); 10 stunned → 4; 0xb flying dead: on landing the piece 325, the explosion, its death
//!   bits; 0xc: the pieces 325 and 403, gone; 0xd idle → 3.
//! * **Every tick after** (not in 4..7 or 1: the head and body straight), `0x2b2600`: the eye glow (joint list 1,
//!   0.5 toward the camera) with its sparks (type 69), the coil points (lists 6, 7, 2) and the draws: a ground glow
//!   (`0x2b3a20`) and five camera-facing glows (`0x2b2928`).
//! * **325** (`0x2de670`): a piece thrown off the bot (0.075..0.2 a tick away from Ratchet, turned 20..60°, up
//!   0.2..0.3, tumbling) that falls (0.003 a tick², ×0.97 drag), smoking (type 21 and two type 23 a tick) for
//!   `ticks(200)` or until it touches something: then a small explosion (sound 0) and gone. A red glow on it every
//!   other tick (`0x2dec40`).
//! * **403** (`0x2dfd10`): mode 1 (the bot's) the same flight (33..80° turn, 0.2 up, sparks `0x2e03b8`), its owner's
//!   sound 0 and an explosion at its end; mode 0 (the rail bots 211's) rides its owner's grind rail at the speed given
//!   for `ticks(90)`, then its owner's sound 1 and the explosion 0.25 above it.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2b17d8` | 28 | [`update`] |
//! | `0x2b2f60` / `0x2b3268` / `0x2b2ed8` | hits, target, range | [`hits`], [`target`], [`range`] |
//! | `0x2b3358` / `0x2b3418` / `0x2b3578` | path chords, follow, drift | [`chords`], [`follow`], [`drift`] |
//! | `0x2b35d8` / `0x2b37b0` | head, body | [`head`], [`body`] |
//! | `0x2b3890` / `0x2b3920` / `0x2b39a0` | the group's cool-downs, zapping count, rising | [`update`] |
//! | `0x2b2600` / `0x2b2928` / `0x2b3a20` | points, glows | [`points`], [`frame`], [`fx_quad_groups`] |
//! | `0x2dea98` / `0x2e0170` | make 325 / 403 | [`make_325`], [`make_403`] |
//! | `0x2de670` / `0x2dec40` | 325 | [`piece_update`], [`piece_quads`] |
//! | `0x2dfd10` / `0x2e03b8` | 403 | [`spark_piece_update`] |
//!
//! [L] The draws are built from the camera the frame part saw (kept in +0x290).

use super::oltanis_arcs::{self as arcs, billboard};
use super::{FxQuad, FxQuads};
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, damage, flash, fx, knock, region, target, turn, DT, DT2, SPEED};
use crate::moby_update::services::{self as sv, pvar as p, HitTemplate, Services, World};
use crate::moby_update::{manip, scheduler, story};
use crate::particles::rec;
use crate::ps2v::Pf;
use crate::spline::{self, Cursor};

pub const REFERENCE_LEVEL: u32 = 14;
pub const UPDATE_FN: u32 = 0x2b_17d8;
pub const DRAW_FN: u32 = 0x2b_2928;
pub const CLASSES: [i16; 1] = [28];
pub const PIECE_FN: u32 = 0x2d_e670;
pub const PIECE_DRAW_FN: u32 = 0x2d_ec40;
pub const PIECE_CLASSES: [i16; 1] = [325];
pub const SPARK_PIECE_FN: u32 = 0x2d_fd10;
pub const SPARK_PIECE_CLASSES: [i16; 1] = [403];

mod o {
    pub const D: usize = 0x20;
    pub const K: usize = 0x60;
    pub const F: usize = 0xc0;
    pub const T: usize = 0xd0;
    pub const TM: usize = 0x110;
    pub const KIND: usize = 0x114;
    pub const NA: usize = 0x120;
    pub const NB: usize = 0x160;
    pub const EYE: usize = 0x1e0;
    pub const P2: usize = 0x1f0;
    pub const P3: usize = 0x200;
    pub const P4: usize = 0x210;
    pub const P5: usize = 0x220;
    pub const RANGE: usize = 0x234;
    pub const RANGE_T: usize = 0x238;
    pub const YAW_V: usize = 0x23c;
    pub const PATH: usize = 0x240;
    pub const AREA: usize = 0x244;
    pub const WAIT: usize = 0x248;
    pub const PATROL: usize = 0x24a;
    pub const AIM_DOWN: usize = 0x24b;
    pub const SEG: usize = 0x24c;
    pub const DIST: usize = 0x250;
    pub const VX: usize = 0x254;
    pub const VY: usize = 0x258;
    pub const COOL: usize = 0x260;
    pub const SWEEP: usize = 0x262;
    pub const HATCH: usize = 0x264;
    pub const NB_YAW: usize = 0x268;
    pub const NB_YAW_V: usize = 0x26c;
    pub const NA_YAW: usize = 0x270;
    pub const NA_YAW_V: usize = 0x274;
    pub const AIM: usize = 0x278;
    pub const DIR: usize = 0x27c;
    pub const NB_PITCH: usize = 0x280;
    pub const NB_PITCH_V: usize = 0x284;
    pub const CLANK: usize = 0x288;
    pub const CAM: usize = 0x290;
    pub const SIZE: usize = 0x2a0;
}

/// Level14 gp words (0x1614a8..0x161504).
const RANGE_BASE: f32 = 40.0;
const PATH_SPEED: f32 = 6.0;
const DRIFT: f32 = 0.92;
const SWING: f32 = std::f32::consts::FRAC_PI_6;
const ARC_LEN: f32 = 25.0;
const LIMIT: f32 = 1.221_730_5;

fn path_pts(w: &World, i: i32) -> Option<Vec<[f32; 4]>> {
    let s = w.svc.splines.get(usize::try_from(i).ok()?)?;
    Some(s.iter().map(|q| q.map(f32::from_bits)).collect())
}
fn path_ok(w: &World, i: i32) -> bool { i != -1 && path_pts(w, i).is_some_and(|p| !p.is_empty()) }
fn link(w: &World, v: i32) -> Option<MobyId> { usize::try_from(v - 1).ok().filter(|&m| v > 0 && m < w.table.mobys.len()) }
fn tmoby(w: &World, id: MobyId) -> Option<MobyId> { link(w, c::pi32(w, id, o::TM)) }
fn heading_to(w: &World, id: MobyId, t: Option<MobyId>) -> f32 {
    let (p, q) = (c::pos(w, id), t.map_or([0.0; 4], |t| c::pos(w, t)));
    c::atan(q[0] - p[0], q[1] - p[1])
}
fn turn4(target: f32, angle: &mut f32, vel: &mut f32) {
    turn::turn_toward(target, DT2 * 12.566_371, DT2 * 12.566_371, DT * 6.283_185_5, angle, vel);
}
fn turn_yaw(w: &mut World, id: MobyId, target: f32) {
    let (mut a, mut v) = (c::yaw(w, id), c::pf(w, id, o::YAW_V));
    turn4(target, &mut a, &mut v);
    c::set_yaw(w, id, a);
    c::set_pf(w, id, o::YAW_V, v);
}
fn turn_pv(w: &mut World, id: MobyId, target: f32, a_off: usize, v_off: usize) {
    let (mut a, mut v) = (c::pf(w, id, a_off), c::pf(w, id, v_off));
    turn4(target, &mut a, &mut v);
    c::set_pf(w, id, a_off, a);
    c::set_pf(w, id, v_off, v);
}
fn wrapped_same(w: &World, id: MobyId) -> bool {
    let a = &w.m(id).anim;
    a.flags & 2 != 0 && a.seq_a == a.seq_b
}
fn blend(w: &mut World, id: MobyId, seq: u8, t: i32) { w.anim_blend(id, seq, 0, t); }
/// `0x2b3318`: hidden, not targetable, no collision.
fn hide(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.has_collision = false;
    m.mode = m.mode & 0xefff | 0x41;
}
/// `0x2b3330`: shown, targetable, its collision.
fn show(w: &mut World, id: MobyId) {
    let coll = super::class_collision(w, w.m(id).o_class);
    let m = w.mm(id);
    m.mode = m.mode & 0xffbe | 0x1000;
    m.has_collision = coll;
}
fn death_bits(w: &mut World, id: MobyId) {
    let fl = if w.body() == 2 { 0x200 } else { 0 };
    set_death_bits(w, id, fl, -1);
}

/// Level14 `0x2b17d8` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    manip::big_head(w, 2.5, id, 2, id, o::NA);
    if c::pi32(w, id, o::CLANK) != 0 {
        if w.body() != 2 {
            let m = w.mm(id);
            m.has_collision = false;
            m.mode |= 0x41;
            if w.m(id).anim.seq_b != 5 {
                let m = w.mm(id);
                m.mode = m.mode & 0xefff | 0x41;
                m.update_dist = 0xff;
                return;
            }
            arcs::release(w, id);
            set_death_bits(w, id, 0, -1);
            w.delete_moby(id);
            return;
        }
        let coll = super::class_collision(w, w.m(id).o_class);
        let st = w.m(id).state;
        let m = w.mm(id);
        m.mode &= 0xffbe;
        if st != 0xb { m.has_collision = coll; }
        m.update_dist = 0x40;
        m.mode |= 0x1000;
    }
    let mut tgt = None;
    if w.m(id).state != 0 {
        hits(w, id);
        if !w.table.mobys.get(id).is_some_and(|m| m.state < 0xfd) { return; }
        target(w, id);
        range(w, id);
        tgt = tmoby(w, id);
    }
    if w.svc.game_mode == 2 {
        let m = w.mm(id);
        m.visible = 0;
        m.mode |= 1;
    } else if w.m(id).state != 1 {
        let m = w.mm(id);
        m.visible = 1;
        m.mode &= 0xfffe;
    }
    let st = w.m(id).state;
    match st {
        0 => {
            start(w, id);
            return;
        }
        1 => {
            rise(w, id);
            return;
        }
        2 => {
            if 3.0 <= c::ground::key_time(w, id) { follow(w, id); }
            if wrapped_same(w, id) {
                w.mm(id).state = 3;
                let t = w.ticks(10);
                blend(w, id, 3, t);
                w.mm(id).anim.speed = 1.0;
            }
        }
        3 => {
            if follow(w, id) {
                let t = w.ticks(10);
                blend(w, id, 0, t);
                w.mm(id).state = 4;
            }
        }
        4 => ready(w, id, tgt),
        5 => {
            let h = heading_to(w, id, tgt);
            head(w, id, c::sub_rot(h, SWING), false);
            body(w, id, h);
            if wrapped_same(w, id) {
                let t = w.ticks(10);
                blend(w, id, 5, t);
                w.mm(id).state = 6;
            }
        }
        6 => zap(w, id, tgt),
        7 => {
            if wrapped_same(w, id) {
                let t = w.ticks(10);
                blend(w, id, 0, t);
                w.mm(id).state = 4;
            }
        }
        8 => {
            let patrol = c::pu8(w, id, o::PATROL) != 0;
            if patrol { hatch_open(w, c::pi32(w, id, o::HATCH)); }
            if follow(w, id) {
                if !patrol {
                    let t = w.ticks(60);
                    c::set_pi16(w, id, o::WAIT, t as i16);
                    w.mm(id).state = 0xd;
                } else {
                    let t = w.ticks(10);
                    blend(w, id, 2, t);
                    w.mm(id).state = 9;
                }
            }
        }
        9 => {
            if let Some(p0) = path_pts(w, c::pi32(w, id, o::PATH)).and_then(|p| p.first().copied()) {
                for (a, v) in [(0, o::VX), (1, o::VY)] {
                    let (mut x, mut vv) = (Pf::f(w.m(id).position[a]), Pf::f(c::pf(w, id, v)));
                    crate::hero::physics::spring(Pf::f(p0[a]), Pf::f(0.01), Pf::f(0.2), Pf::ZERO, &mut x, &mut vv);
                    w.mm(id).position[a] = x.to_f32();
                    c::set_pf(w, id, v, vv.to_f32());
                }
            }
            if wrapped_same(w, id) {
                let t = w.ticks(10);
                blend(w, id, 0, t);
                w.mm(id).state = 1;
                w.mm(id).anim.speed = 1.0;
                hide(w, id);
                return;
            }
        }
        10 => {
            let (a, b) = (w.m(id).anim.seq_a, w.m(id).anim.seq_b);
            if wrapped_same(w, id) || (a != 7 && b != 7) {
                blend(w, id, 0, 10);
                w.mm(id).state = 4;
            }
        }
        0xb => {
            let r = knock::update(w, id, o::K);
            if r & 0x160 == 0 { return; }
            arcs::release(w, id);
            let jp = w.joint_point(id, 2);
            make_325(w, id, jp);
            let pos = c::pos(w, id);
            let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 5, sparks: 2, puffs: 4, debris: 1, sound: -1, shake: true };
            fx::beam_explosion(w, &b, Some(id), pos);
            death_bits(w, id);
            w.delete_moby(id);
            return;
        }
        0xc => {
            let jp = w.joint_point(id, 2);
            make_325(w, id, jp);
            make_403(w, id, 0.0, 1);
            arcs::release(w, id);
            w.delete_moby(id);
            return;
        }
        0xd => {
            let h = heading_to(w, id, tgt);
            turn_yaw(w, id, h);
            if c::pi32(w, id, o::KIND) != 2 && c::dec_timer_pvar_s16(w, id, o::WAIT) != 0 {
                w.mm(id).state = 3;
                let t = w.ticks(10);
                blend(w, id, 3, t);
                w.mm(id).anim.speed = 1.0;
                for k in [o::DIST, o::SEG, o::VX, 0x25c, o::VY] { c::set_pi32(w, id, k, 0); }
                c::set_pf(w, id, o::DIR, 1.0);
            }
        }
        _ => {}
    }
    let s = w.m(id).state;
    if !(4..=7).contains(&s) && s != 1 {
        let y = c::yaw(w, id);
        head(w, id, y, false);
        body(w, id, y);
    }
    points(w, id);
}

/// State 0 (module doc).
fn start(w: &mut World, id: MobyId) {
    let patrol = c::pu8(w, id, o::PATROL) != 0;
    if patrol && !path_ok(w, c::pi32(w, id, o::PATH)) {
        w.delete_moby(id);
        return;
    }
    if !path_ok(w, c::pi32(w, id, o::AREA)) || (patrol && c::pi32(w, id, o::HATCH) < 0) {
        w.delete_moby(id);
        return;
    }
    if w.rng.randi(2) != 0 { w.mm(id).mode |= 0x8000; }
    chords(w, id);
    if let Some(p0) = path_pts(w, c::pi32(w, id, o::PATH)).and_then(|p| p.first().copied()) { w.mm(id).position = p0; }
    c::set_pu8(w, id, o::D + 8, 1);
    c::set_pu8(w, id, o::D + 9, 0);
    c::set_pf(w, id, o::D, 2.0);
    c::set_pf(w, id, o::D + 0x10, 1.25);
    c::set_pi16(w, id, o::D + 4, 2);
    c::set_pu8(w, id, 0x58, 15);
    c::set_pu8(w, id, 0x5a, 16);
    c::set_pf(w, id, o::DIR, 1.0);
    c::set_pi16(w, id, o::WAIT, 0);
    c::set_pf(w, id, o::YAW_V, 0.0);
    c::set_pi16(w, id, o::COOL, 0);
    for k in [o::NB_YAW, o::NB_YAW_V, o::NB_PITCH, o::NB_PITCH_V, o::NA_YAW, o::NA_YAW_V] { c::set_pf(w, id, k, 0.0); }
    manip::attach(w, id, 2, id, o::NA);
    manip::attach(w, id, 3, id, o::NB);
    if patrol {
        hide(w, id);
        w.mm(id).state = 1;
        return;
    }
    let z = c::ground::ground(w, c::pos(w, id), 0.5, 0).z;
    w.mm(id).position[2] = z;
    w.mm(id).state = if c::pi32(w, id, o::PATH) == -1 { 4 } else { 0xd };
}

/// `0x2d9870(hatch)`: the hatch 250 opens (state 0 → 5, its hard cut 1), its timer `ticks(60)`; whether it is open.
fn hatch_open(w: &mut World, h: i32) -> bool {
    let Some(h) = usize::try_from(h).ok().filter(|&m| m < w.table.mobys.len()) else { return false };
    let st = w.m(h).state;
    if st == 0 {
        w.mm(h).state = 5;
        c::hard_cut(w, h, 1, 0);
    }
    let t = w.ticks(0x3c);
    story::pvars(w, h, 0xc);
    c::set_pi32(w, h, 8, t);
    st == 6
}

/// The bots of the group (`0x1abf40[+0x21]`).
fn group(w: &World, id: MobyId) -> Vec<MobyId> {
    let g = w.m(id).group;
    if g == -1 { return Vec::new(); }
    scheduler::group_ids(w, g).into_iter().filter(|&m| w.table.mobys.get(m).is_some_and(|b| b.o_class == CLASSES[0])).collect()
}

/// State 1: in the hatch (module doc).
fn rise(w: &mut World, id: MobyId) {
    if c::pi32(w, id, o::KIND) == 2 { return; }
    if c::dec_timer_pvar_s16(w, id, o::WAIT) == 0 { return; }
    if !hatch_open(w, c::pi32(w, id, o::HATCH)) { return; }
    // 0x2b39a0: no bot of the group rising.
    if group(w, id).into_iter().any(|m| w.m(m).state == 2) { return; }
    let t = w.ticks(60);
    c::set_pi16(w, id, o::WAIT, t as i16);
    show(w, id);
    w.mm(id).state = 2;
    c::hard_cut(w, id, 1, 0);
    if let Some(p) = path_pts(w, c::pi32(w, id, o::PATH)) {
        let (p0, p1) = (p[0], p.get(1).copied().unwrap_or(p[0]));
        c::set_yaw(w, id, c::atan(p1[0] - p0[0], p1[1] - p0[1]));
    }
    c::set_pf(w, id, o::NB_YAW, 0.0);
    c::set_pf(w, id, o::DIR, 1.0);
    for k in [o::VX, o::NB_YAW_V, o::NA_YAW, o::NA_YAW_V, o::SEG, o::DIST, 0x25c, o::VY] { c::set_pi32(w, id, k, 0); }
}

/// State 4 (module doc).
fn ready(w: &mut World, id: MobyId, tgt: Option<MobyId>) {
    drift(w, id);
    let h = heading_to(w, id, tgt);
    turn_yaw(w, id, h);
    let y = c::yaw(w, id);
    let f = head(w, id, y, false);
    body(w, id, h);
    if c::pi32(w, id, o::KIND) == 2 {
        if c::pi32(w, id, o::PATH) == -1 { return; }
        w.mm(id).state = 8;
        if w.m(id).anim.seq_b != 3 {
            let t = w.ticks(0x14);
            blend(w, id, 3, t);
        }
        let seg = c::pi32(w, id, o::SEG) - 1;
        c::set_pi32(w, id, o::SEG, seg);
        let d = path_pts(w, c::pi32(w, id, o::PATH)).and_then(|p| p.get(seg.max(0) as usize).map(|q| q[3])).unwrap_or(0.0);
        c::set_pf(w, id, o::VX, 0.0);
        c::set_pf(w, id, o::DIST, d);
        c::set_pf(w, id, o::DIR, -1.0);
        c::set_pi32(w, id, 0x25c, 0);
        c::set_pf(w, id, o::VY, 0.0);
        arcs::release(w, id);
        return;
    }
    if c::dec_timer_pvar_s16(w, id, o::COOL) == 0 { return; }
    let zapping = group(w, id).into_iter().filter(|&m| (5..=6).contains(&w.m(m).state)).count();
    if 1 < zapping { return; }
    if !arcs::register(w, id, 1, 0) { return; }
    if w.rng.randi(4) != 0 { return; }
    if 0.017_453_292 <= c::diff_rots(c::yaw(w, id), h) { return; }
    if f >= 0.017_453_292 { return; }
    let Some(t) = tgt else { return };
    let (a, b) = (c::pos(w, id), c::pos(w, t));
    if let Ok(area) = usize::try_from(c::pi32(w, id, o::AREA)) {
        if region::crosses(w, area, a, b) { return; }
    }
    for m in group(w, id) {
        let t = w.ticks(45);
        story::pvars(w, m, o::SIZE);
        c::set_pi16(w, m, o::COOL, t as i16);
    }
    c::set_pf(w, id, o::AIM, h);
    w.mm(id).state = 5;
    let t = w.ticks(5);
    blend(w, id, 4, t);
    let t = w.ticks(120);
    c::set_pi16(w, id, o::SWEEP, t as i16);
}

/// State 6, the sweep (module doc).
fn zap(w: &mut World, id: MobyId, tgt: Option<MobyId>) {
    let h = heading_to(w, id, tgt);
    let k = c::pi16(w, id, o::SWEEP) as f32 / w.ticks(120) as f32;
    let a = c::sub_rot(c::pf(w, id, o::AIM), -SWING + (SWING + SWING) * k);
    let down = c::pu8(w, id, o::AIM_DOWN) != 0 && tgt.is_some_and(|t| (w.m(t).position[2] + 0.1) - w.m(id).position[2] < 0.0);
    head(w, id, a, down);
    body(w, id, h);
    let j = w.joint_matrix(id, 0);
    let mut v = [j[1][0], j[1][1], j[1][2], 0.0];
    if !down { v[2] = 0.0; }
    let from0 = j[3];
    let mut end = c::add(from0, c::set_len3(v, ARC_LEN));
    let from = c::add(from0, c::set_len3(v, 0.25));
    let y = c::yaw(w, id);
    let tmpl = HitTemplate { dir: sv::pv([y.cos(), y.sin(), 1.0, f32::from_bits(0x45af_df66)]), attacker: Some(id), flags: 0x1_0001, b18: 0, b19: 1, h1a: w.m(id).o_class as u16, damage: Pf::ONE, w20: 1 };
    if let Some(hit) = sv::line_hit_in(w.table, w.svc, w.classes, w.coll, sv::pv(from), sv::pv(end), 0, Some(id), &tmpl) {
        end = [hit.point[0], hit.point[1], hit.point[2], end[3]];
    }
    arcs::aim(w, id, j, end);
    if c::dec_timer_pvar_s16(w, id, o::SWEEP) != 0 {
        let t = w.ticks(10);
        blend(w, id, 6, t);
        w.mm(id).state = 7;
        arcs::release(w, id);
    }
}

/// `0x2b2f60` (module doc).
pub fn hits(w: &mut World, id: MobyId) {
    let st = w.m(id).state;
    if st == 0 { return; }
    if st == 0xb {
        w.mm(id).hit_slot = 0xff;
        flash::update(w, id, o::F);
        return;
    }
    let mut hit = w.get_hit(id, 0x33_0000, false);
    if let Some(h) = hit {
        let own = w.m(id).o_class;
        if h.attacker.is_some_and(|a| w.table.mobys.get(a).is_some_and(|m| m.o_class == own || m.o_class == 0x370)) { hit = None; }
    }
    let r = damage::resolve(w, id, hit, o::D, 0, 4);
    if r.out5 != 1 && w.m(id).state != 0xb {
        let health = c::pf(w, id, o::D) - r.damage;
        c::set_pf(w, id, o::D, health);
        let reaction = if health <= 0.0 { 1 } else { r.reaction };
        let (flags, dir) = r.hit.or(hit).map_or((0, [0.0; 4]), |h| (h.flags, h.dir.map(|x| x.to_f32())));
        let ang = c::atan(dir[0], dir[1]);
        match reaction {
            1 | 2 => {
                let m = w.mm(id);
                m.state = if flags & 0x80_0000 == 0 { 0xb } else { 0xc };
                m.mode &= 0xefff;
                let zoff = c::pf(w, id, o::K + knock::k::ZOFF);
                c::set_pi32(w, id, o::K + knock::k::RADIUS, (zoff * 1024.0) as i32);
                c::set_pf(w, id, o::K + knock::k::ZOFF, 0.3);
                c::set_pf(w, id, o::K + knock::k::UP, DT * 10.0);
                c::set_pf(w, id, o::K + knock::k::KEY_APEX, 9.0);
                c::set_pf(w, id, o::K + knock::k::SPEED, DT * 10.0);
                c::set_pf(w, id, o::K + knock::k::KEY_LAND, 15.0);
                let t = w.ticks(5);
                knock::start(w, id, o::K, ang, 8, t, 0);
                c::set_pu8(w, id, o::F + 7, 0xfa);
                death_bits(w, id);
                w.mm(id).has_collision = false;
            }
            3..=10 => {
                arcs::release(w, id);
                w.mm(id).state = 10;
                let t = w.ticks(0x3c);
                c::set_pi16(w, id, o::D + 6, t as i16);
                c::set_pf(w, id, o::K + knock::k::KEY_APEX, 11.0);
                c::set_pf(w, id, o::K + knock::k::SPEED, DT * 8.0);
                c::set_pf(w, id, o::K + knock::k::KEY_LAND, 29.0);
                c::set_pf(w, id, o::K + knock::k::UP, 3.0 * DT);
                knock::start(w, id, o::K, ang, 7, 1, 0);
                c::set_pu8(w, id, o::F + 7, 0xfa);
            }
            _ => {}
        }
        flash::start(w, id, o::F);
    }
    if !w.table.mobys.get(id).is_some_and(|m| m.state < 0xfd) { return; }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, o::F);
}

/// `0x2b3268` (module doc).
pub fn target(w: &mut World, id: MobyId) {
    let area = usize::try_from(c::pi32(w, id, o::AREA)).ok();
    let t = target::acquire_in(w, id, c::pf(w, id, o::RANGE), area);
    c::set_pv4(w, id, o::T, t.pos);
    c::set_pv4(w, id, o::T + 0x10, t.rot);
    c::set_pv4(w, id, o::T + 0x20, t.aim);
    c::set_pv4(w, id, o::T + 0x30, t.body);
    c::set_pi32(w, id, o::TM, t.moby.map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, o::KIND, t.kind as i32);
    if t.kind != 2 && c::pf(w, id, o::RANGE) < c::dist2(c::pos(w, id), t.pos) { c::set_pi32(w, id, o::KIND, 2); }
    if c::pi32(w, id, o::TM) == 0 { c::set_pi32(w, id, o::TM, w.hero_moby.map_or(0, |m| m as i32 + 1)); }
}

/// `0x2b2ed8` (module doc).
pub fn range(w: &mut World, id: MobyId) {
    if c::pi32(w, id, o::D + 0x18) != 0 {
        let r = w.rng.randf(180.0, 240.0);
        let t = w.svc.timing.scale(Pf::f(r)).to_i32();
        c::set_pi32(w, id, o::RANGE_T, t);
        c::set_pi32(w, id, o::D + 0x18, 0);
    }
    c::dec_timer_pvar_i32(w, id, o::RANGE_T);
    let r = if c::pi32(w, id, o::RANGE_T) != 0 { RANGE_BASE + 6.0 } else { RANGE_BASE };
    c::set_pf(w, id, o::RANGE, r);
}

/// `0x2b3358`: the cursor and drift 0; the path's chords as a loop (point i to point (i + 1) mod n).
pub fn chords(w: &mut World, id: MobyId) {
    for k in [o::DIST, o::SEG, o::VX, 0x25c, o::VY] { c::set_pi32(w, id, k, 0); }
    let i = c::pi32(w, id, o::PATH);
    let Some(pts) = path_pts(w, i) else { return };
    let n = pts.len();
    for k in 0..n {
        let d = c::dist3(pts[k], pts[(k + 1) % n]);
        w.svc.splines[i as usize][k][3] = d.to_bits();
    }
}

/// `0x2b3418`: along the path at 6·+0x27c a tick (module doc); whether an end was reached.
pub fn follow(w: &mut World, id: MobyId) -> bool {
    let pts = path_pts(w, c::pi32(w, id, o::PATH)).unwrap_or_default();
    let mut cur = Cursor { seg: c::pi32(w, id, o::SEG), t: c::pf(w, id, o::DIST) };
    let (q, ended) = spline::advance(&pts, false, PATH_SPEED * DT * c::pf(w, id, o::DIR), &mut cur);
    c::set_pi32(w, id, o::SEG, cur.seg);
    c::set_pf(w, id, o::DIST, cur.t);
    let old = c::pos(w, id);
    for (a, v) in [(0, o::VX), (1, o::VY)] {
        let (mut x, mut vv) = (Pf::f(w.m(id).position[a]), Pf::f(c::pf(w, id, v)));
        crate::hero::physics::spring(Pf::f(q[a]), Pf::f(0.01), Pf::f(0.2), Pf::ZERO, &mut x, &mut vv);
        w.mm(id).position[a] = x.to_f32();
        c::set_pf(w, id, v, vv.to_f32());
    }
    turn_yaw(w, id, c::atan(q[0] - old[0], q[1] - old[1]));
    ended
}

/// `0x2b3578`: the drift (×0.92 a tick).
pub fn drift(w: &mut World, id: MobyId) {
    let f = (DRIFT - 1.0) * SPEED + 1.0;
    let (vx, vy) = (c::pf(w, id, o::VX) * f, c::pf(w, id, o::VY) * f);
    c::set_pf(w, id, o::VY, vy);
    c::set_pf(w, id, o::VX, vx);
    let m = w.mm(id);
    m.position[0] += vx;
    m.position[1] += vy;
}

/// `0x2b35d8(angle, m, down)`: the head (list 3) toward `angle` (within ±70° of the body), pitched at the target 1.5
/// up when `down`; returns how far it still is.
pub fn head(w: &mut World, id: MobyId, angle: f32, down: bool) -> f32 {
    let f = c::sub_rot(angle, c::yaw(w, id)).clamp(-LIMIT, LIMIT);
    turn_pv(w, id, f, o::NB_YAW, o::NB_YAW_V);
    let yaw_q = sv::axis_quat(Pf::f(c::pf(w, id, o::NB_YAW)), 2);
    let mut q = yaw_q;
    if down {
        let t = tmoby(w, id);
        let mut tp = t.map_or([0.0; 4], |t| c::pos(w, t));
        if t.is_some() && t == w.hero_moby { tp[2] -= w.hero.height.to_f32(); }
        let mut d = c::sub(tp, c::pos(w, id));
        d[2] += 1.5;
        let pitch = c::atan(c::len2(d), d[2]);
        turn_pv(w, id, -pitch, o::NB_PITCH, o::NB_PITCH_V);
        q = sv::quat_mul(yaw_q, sv::axis_quat(Pf::f(c::pf(w, id, o::NB_PITCH)), 1));
    }
    manip::set_quat(w, id, id, o::NB, sv::fv(q));
    c::diff_rots(f, c::pf(w, id, o::NB_YAW))
}

/// `0x2b37b0(angle, m)`: the body (list 2) turned against the head toward `angle` (within ±70°).
pub fn body(w: &mut World, id: MobyId, angle: f32) {
    let f = c::sub_rot(angle, c::yaw(w, id));
    let g = c::add_rot(-c::pf(w, id, o::NB_YAW), -f).clamp(-LIMIT, LIMIT);
    turn_pv(w, id, g, o::NA_YAW, o::NA_YAW_V);
    let a = c::pf(w, id, o::NA_YAW);
    manip::set_axis(w, id, id, o::NA, a, 2);
}

/// `0x2b2600` (module doc).
pub fn points(w: &mut World, id: MobyId) {
    let cam = w.camera.map(|x| x.to_f32());
    let p1 = w.joint_point(id, 1);
    c::set_pv4(w, id, o::EYE, c::add(p1, c::set_len3(c::sub(p1, cam), -0.5)));
    let v = fx::rand_vec_ab(w, f32::from_bits(0x3ba3_d70a), f32::from_bits(0x3cf5_c28f));
    if let Some(i) = fx::part69(w, p1, v, 0x7f, Some(id)) {
        let s = w.rng.randf(f32::from_bits(0x45bb_8000), 32000.0);
        if let Some(r) = fx::rec_mut(w, i) { rec::set_ff(r, 0xc, s); }
    }
    for k in 0..3 {
        let Some(i) = fx::part69(w, p1, [0.0; 4], 0x7f, Some(id)) else { continue };
        let s = if k == 2 && w.rng.randi(8) == 0 { f32::from_bits(0x482f_c800) } else { w.rng.randf(f32::from_bits(0x479c_4000), f32::from_bits(0x47ea_6000)) };
        let t = w.ticks(2);
        if let Some(r) = fx::rec_mut(w, i) {
            rec::set_ff(r, 0xc, s);
            rec::set_i16(r, 10, t as i16);
            rec::set_i16(r, 0x36, 3);
            rec::set_u32(r, 0x38, 0x7f7f7f);
            rec::set_ff(r, 0x30, 1.0 / t as f32);
        }
    }
    let p6 = w.joint_point(id, 6);
    c::set_pv4(w, id, o::P3, p6);
    let j7 = w.joint_matrix(id, 7);
    c::set_pv4(w, id, o::P2, c::add(j7[3], c::scale(j7[0], 0.3)));
    let j2 = w.joint_matrix(id, 2);
    let (a, b, cc) = (c::scale(j2[0], 0.4), c::scale(j2[1], 0.1), c::scale(j2[2], 0.125));
    let p4 = c::add(c::add(c::add(j2[3], a), b), cc);
    c::set_pv4(w, id, o::P4, p4);
    c::set_pv4(w, id, o::P5, c::add(p4, c::scale(b, -2.0)));
    if let Some(row) = super::row(REFERENCE_LEVEL, DRAW_FN) {
        w.svc.draw_callbacks.register(Callback::UnitFrame(row), id);
        w.svc.draw_callbacks.register(Callback::UnitQuads(row), id);
    }
}

/// The draws' frame part: the camera kept.
pub fn frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < o::SIZE { return; }
    let cam = crate::hero::physics::to_f32x3(w.camera);
    c::set_pv4(w, id, o::CAM, [cam[0], cam[1], cam[2], 1.0]);
}

/// Level14 `0x2b3a20` (the ground glow) and `0x2b2928` (the five glows) (module doc).
pub fn fx_quad_groups(table: &MobyTable, _svc: &Services, id: MobyId) -> Vec<FxQuads> {
    let Some(m) = table.mobys.get(id).filter(|m| m.pvars.len() >= o::SIZE) else { return Vec::new() };
    let v = |off: usize| -> [f32; 3] { [p::ff(&m.pvars, off), p::ff(&m.pvars, off + 4), p::ff(&m.pvars, off + 8)] };
    let cam = v(o::CAM);
    // 0x2b3a20.
    let mut g = [m.position[0], m.position[1], m.position[2] + 0.01];
    let d = [cam[0] - g[0], cam[1] - g[1], cam[2] - g[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if 0.0 < l {
        let e = l - 2.0;
        let s = if e < 0.4 { e.max(0.0) } else { 0.4 };
        for k in 0..3 { g[k] += d[k] * s / l; }
    }
    let corners = [[-1.0f32, 1.0], [-1.0, -1.0], [1.0, 1.0], [1.0, -1.0]].map(|[x, y]| [x * 2.0 + g[0], y * 2.0 + g[1], g[2]]);
    let ground = FxQuad { corners, st: [[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]], rgba: [0x207f_6060; 4] };
    // 0x2b2928.
    let pull = |p: [f32; 3]| -> [f32; 3] {
        let d = [cam[0] - p[0], cam[1] - p[1], cam[2] - p[2]];
        let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if n == 0.0 { p } else { std::array::from_fn(|k| p[k] + d[k] * 0.3 / n) }
    };
    let glows = vec![
        billboard(cam, v(o::EYE), 0.4, [0.0; 3], 0x80ff_ffff),
        billboard(cam, v(o::P3), 0.2, [0.0; 3], 0x2020_20ff),
        billboard(cam, pull(v(o::P2)), 0.2, [0.0; 3], 0x4020_ff20),
        billboard(cam, pull(v(o::P4)), 0.15, [0.0; 3], 0x40ff_2020),
        billboard(cam, pull(v(o::P5)), 0.15, [0.0; 3], 0x40ff_2020),
    ];
    vec![FxQuads { fx: 0xb, additive: true, subtract: false, quads: vec![ground] }, FxQuads { fx: 0xb, additive: true, subtract: false, quads: glows }]
}

/// Away from Ratchet (xy) at `len`, turned by `ang` about the gravity (`FUN_00274ac8`).
fn away(w: &World, at: c::V, len: f32, ang: f32) -> [f32; 3] {
    let h = super::hero_pos(w);
    let d = c::set_len3([at[0] - h[0], at[1] - h[1], 0.0, 0.0], len);
    let (s, cs) = ang.sin_cos();
    // About (0, 0, −1): d·cos + (k × d)·sin (k·d = 0).
    let k = [0.0f32, 0.0, -1.0];
    let x = [k[1] * d[2] - k[2] * d[1], k[2] * d[0] - k[0] * d[2], k[0] * d[1] - k[1] * d[0]];
    std::array::from_fn(|i| d[i] * cs + x[i] * s)
}

/// `0x2dea98(owner, at)`: the piece 325 (module doc).
pub fn make_325(w: &mut World, owner: MobyId, at: c::V) -> Option<MobyId> {
    let m = w.create_moby(0x145)?;
    story::pvars(w, m, 0x28);
    c::set_pi32(w, m, 0x24, owner as i32 + 1);
    let yaw = c::yaw(w, owner);
    {
        let mm = w.mm(m);
        mm.position = at;
        mm.state = 0;
        mm.update_dist = 0xff;
        mm.draw_dist = 0xff;
        mm.visible = 1;
        mm.rotation[2] = yaw;
    }
    let len = w.rng.randf(0.075 * SPEED, 0.2 * SPEED);
    let ang = w.rng.randf_sym(f32::from_bits(0x3eb2_b8c2), f32::from_bits(0x3f86_0a92));
    let v = away(w, at, len, ang);
    let up = w.rng.randf(0.2 * SPEED, 0.3 * SPEED);
    c::set_pv4(w, m, 0, [v[0], v[1], up, 0.0]);
    for (o, lo, hi) in [(0x10, 0, 0x3c0e_fa35), (0x14, 0x3d56_7750, 0x3e0e_fa35), (0x18, 0, 0x3c8e_fa35)] {
        let r = w.rng.randf_sym(f32::from_bits(lo), f32::from_bits(hi));
        c::set_pf(w, m, o, r);
    }
    let t = w.ticks(200);
    c::set_pi32(w, m, 0x20, t);
    w.build_matrix(m);
    Some(m)
}

/// `0x2e0170(owner, &speed, mode)`: the piece 403 (module doc; mode 0 rides the owner's grind rail +0x60).
pub fn make_403(w: &mut World, owner: MobyId, speed: f32, mode: u8) -> Option<MobyId> {
    let m = w.create_moby(0x193)?;
    story::pvars(w, m, 0x40);
    let (pos, yaw) = (c::pos(w, owner), c::yaw(w, owner));
    {
        let mm = w.mm(m);
        mm.position = pos;
        mm.state = mode;
        mm.update_dist = 0xff;
        mm.rotation[2] = yaw;
        mm.draw_dist = 0xff;
        mm.visible = 1;
    }
    c::set_pi32(w, m, 0, owner as i32 + 1);
    if mode == 0 {
        let t = w.ticks(0x5a);
        c::set_pi32(w, m, 4, t);
        c::set_pf(w, m, 8, speed * DT);
        let rail = c::pi32(w, owner, 0x60);
        c::set_pi32(w, m, 0xc, rail);
        c::set_pi32(w, m, 0x10, 0);
        c::set_pi32(w, m, 0x14, 0);
        // SplineSample(20, 5, 0, rail, pos): the cursor at the rail point nearest it.
        if let Some(pts) = rail_pts(w, rail) {
            let k = spline::nearest(&pts, false, 20.0, 5.0, 0.0, [pos[0], pos[1], pos[2]]).map(|(_, k)| k).unwrap_or_default();
            c::set_pi32(w, m, 0x10, k.seg);
            c::set_pf(w, m, 0x14, k.t);
        }
    } else {
        let t = w.ticks(200);
        c::set_pi32(w, m, 4, t);
        let len = w.rng.randf(0.075 * SPEED, 0.2 * SPEED);
        let ang = w.rng.randf_sym(f32::from_bits(0x3f13_7207), f32::from_bits(0x3fb2_b8c2));
        let v = away(w, pos, len, ang);
        let up = w.rng.randf(0.2 * SPEED, 0.2 * SPEED);
        c::set_pv4(w, m, 0x30, [v[0], v[1], up, 0.0]);
        for (o, lo, hi) in [(0x20, 0, 0x3c0e_fa35), (0x24, 0x3d56_7750, 0x3e0e_fa35), (0x28, 0, 0x3c8e_fa35)] {
            let r = w.rng.randf_sym(f32::from_bits(lo), f32::from_bits(hi));
            c::set_pf(w, m, o, r);
        }
    }
    w.build_matrix(m);
    Some(m)
}

fn rail_pts(w: &World, i: i32) -> Option<Vec<[f32; 4]>> {
    let g = w.svc.volumes.grind_paths.get(usize::try_from(i).ok()?)?;
    (!g.points.is_empty()).then(|| g.points.clone())
}

pub(super) fn in_box(p: c::V) -> bool { p[..3].iter().all(|&x| (2.0..=1021.0).contains(&x)) }

/// The pieces' flight: position += velocity (at `v`), the drag (×0.97 above 0.05 a tick; z only rising), gravity
/// 0.003, the tumble (`spin`).
fn fly(w: &mut World, id: MobyId, v: usize, spin: usize) {
    let mut vel = c::pv4(w, id, v);
    let p = c::add(c::pos(w, id), vel);
    w.mm(id).position = [p[0], p[1], p[2], w.m(id).position[3]];
    let f = (0.97 - 1.0) * SPEED + 1.0;
    for v in vel.iter_mut().take(2) {
        if 0.05 * SPEED < v.abs() { *v *= f; }
    }
    if 0.0 < vel[2] { vel[2] *= f; }
    vel[2] -= 0.003;
    c::set_pv4(w, id, v, vel);
    for k in 0..3 {
        let r = c::add_rot(w.m(id).rotation[k], c::pf(w, id, spin + 4 * k));
        w.mm(id).rotation[k] = r;
    }
}

/// Two smoke puffs (type 23) at `at` moving `vel` (module doc).
pub(super) fn puffs(w: &mut World, at: c::V, vel: c::V, n: usize, lo: f32, hi: f32) {
    for _ in 0..n {
        let mut k = w.rng.randi(6);
        if w.rng.randi(2) != 0 { k = -k; }
        let size = w.rng.randf(lo, hi);
        let g = w.rng.rand_range(0x30, 0xff) as u32;
        let Some(i) = fx::part23_rec(w, [f32::from_bits(0x3ccc_cccd), 1.0, 1.0, size], at, k, vel, g | g << 16 | g << 8 | 0x6000_0000) else { continue };
        let t = w.ticks(0x3c);
        if let Some(r) = fx::rec_mut(w, i) {
            r[3] = 0x44;
            rec::set_i16(r, 10, t as i16);
            rec::set_u32(r, 0x24, 2);
            r[0x2a] = 0x60;
            r[0x2b] = r[10];
        }
    }
}

fn small_explosion(w: &mut World, id: MobyId, at: c::V, sound: i32) {
    let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 1.0, flash2: 0.5, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 5, sparks: 2, puffs: 4, debris: 5, sound, shake: true };
    fx::beam_explosion(w, &b, Some(id), at);
}

/// Level14 `0x2de670`: 325 (module doc).
pub fn piece_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x28);
    if w.rng.randi(2) != 0 {
        if let Some(row) = super::row(REFERENCE_LEVEL, PIECE_DRAW_FN) {
            w.svc.draw_callbacks.register(Callback::UnitFrame(row), id);
            w.svc.draw_callbacks.register(Callback::UnitQuads(row), id);
        }
    }
    fly(w, id, 0, 0x10);
    let p = c::pos(w, id);
    if !in_box(p) {
        w.delete_moby(id);
        return;
    }
    let owner = link(w, c::pi32(w, id, 0x24));
    if c::dec_timer_pvar_i32(w, id, 0x20) == 0 && w.coll_sphere(sv::pv(p), Pf::f(0.5), 0, owner).is_none() {
        let life = { let (a, b) = (w.ticks(10), w.ticks(0x14)); w.rng.rand_range(a, b) };
        fx::part21(w, 60000.0, p, [0.0; 4], 0x4f00_7fff, 0x1fff_ffff, life, 1);
        puffs(w, p, [0.0; 4], 2, 40000.0, 100_000.0);
        return;
    }
    small_explosion(w, id, p, 0);
    w.delete_moby(id);
}

/// Level14 `0x2dec40`: 325's red glow (row 0 −0.258, row 2 0.448 off its centre, size 0.4) (module doc).
pub fn piece_quads(table: &MobyTable, _svc: &Services, id: MobyId) -> Option<FxQuads> {
    let m = table.mobys.get(id).filter(|m| m.pvars.len() >= 0x38)?;
    let cam = [p::ff(&m.pvars, 0x28), p::ff(&m.pvars, 0x2c), p::ff(&m.pvars, 0x30)];
    let at: [f32; 3] = std::array::from_fn(|k| m.rows[0][k] * f32::from_bits(0xbe84_1893) + m.rows[2][k] * f32::from_bits(0x3ee5_6042) + m.position[k]);
    Some(FxQuads { fx: 0xb, additive: true, subtract: false, quads: vec![billboard(cam, at, 0.4, [0.0; 3], 0x2020_20ff)] })
}

/// 325's frame part: the camera kept (+0x28).
pub fn piece_frame(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x38);
    let cam = crate::hero::physics::to_f32x3(w.camera);
    c::set_pv4(w, id, 0x28, [cam[0], cam[1], cam[2], 1.0]);
}

/// `0x2e03b8`: 403's sparks and smoke (module doc).
fn spark_trail(w: &mut World, id: MobyId) {
    let st = w.m(id).state;
    let r2 = w.m(id).rows[2];
    let r2 = [r2[0], r2[1], r2[2], 0.0];
    let at = c::add(c::pos(w, id), c::set_len3(r2, 0.2));
    let v = fx::rand_vec_ab(w, f32::from_bits(0x3ba3_d70a), f32::from_bits(0x3cf5_c28f));
    if let Some(i) = fx::part69(w, at, v, 0x7f, Some(id)) {
        let s = w.rng.randf(f32::from_bits(0x45bb_8000), 32000.0);
        if let Some(r) = fx::rec_mut(w, i) { rec::set_ff(r, 0xc, s); }
    }
    for k in 0..3 {
        let Some(i) = fx::part69(w, at, [0.0; 4], 0x7f, Some(id)) else { continue };
        let s = if k == 2 && w.rng.randi(8) == 0 { f32::from_bits(0x482f_c800) } else { w.rng.randf(f32::from_bits(0x479c_4000), f32::from_bits(0x47ea_6000)) };
        let t = w.ticks(2);
        if let Some(r) = fx::rec_mut(w, i) {
            rec::set_ff(r, 0xc, s);
            rec::set_i16(r, 10, t as i16);
            rec::set_i16(r, 0x36, 4);
            rec::set_u32(r, 0x38, 0x7f7f7f);
            rec::set_ff(r, 0x30, 1.0 / t as f32);
        }
    }
    let mut vel = [c::pf(w, id, 0x18), c::pf(w, id, 0x1c), 0.0, 1.0];
    let up = if st == 0 { f32::from_bits(0x3f73_3333) } else {
        vel[2] = SPEED * 0.03;
        f32::from_bits(0x3f93_3333)
    };
    let y = c::yaw(w, id);
    let mut p = c::add(c::add(c::pos(w, id), c::set_len3(r2, up)), [y.cos() * 0.3, y.sin() * 0.3, 0.0, 0.0]);
    if st == 1 {
        p[0] += w.rng.randf_sym(0.0, f32::from_bits(0x3dcc_cccd));
        p[1] += w.rng.randf_sym(0.0, f32::from_bits(0x3dcc_cccd));
    }
    let life = { let (a, b) = (w.ticks(10), w.ticks(0x14)); w.rng.rand_range(a, b) };
    fx::part21(w, 60000.0, p, vel, 0x4f00_7fff, 0x1fff_ffff, life, 1);
    puffs(w, p, vel, 1, 60000.0, 140_000.0);
}

/// Level14 `0x2dfd10`: 403 (module doc).
pub fn spark_piece_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x40);
    let owner = link(w, c::pi32(w, id, 0));
    let owner_class = owner.map_or(-1, |o| w.m(o).o_class);
    if w.m(id).state == 0 {
        let old = c::pos(w, id);
        if let Some(pts) = rail_pts(w, c::pi32(w, id, 0xc)) {
            let mut cur = Cursor { seg: c::pi32(w, id, 0x10), t: c::pf(w, id, 0x14) };
            let (q, _) = spline::advance(&pts, false, c::pf(w, id, 8), &mut cur);
            c::set_pi32(w, id, 0x10, cur.seg);
            c::set_pf(w, id, 0x14, cur.t);
            let m = w.mm(id);
            m.position = [q[0], q[1], q[2], m.position[3]];
        }
        let d = c::sub(c::pos(w, id), old);
        c::set_pf(w, id, 0x18, d[0]);
        c::set_pf(w, id, 0x1c, d[1]);
        spark_trail(w, id);
        if c::dec_timer_pvar_i32(w, id, 4) == 0 { return; }
        let p = c::pos(w, id);
        w.play_sound_as(1, 0, id, owner_class);
        let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 1.0, flash2: 0.5, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 5, sparks: 2, puffs: 4, debris: 3, sound: -1, shake: false };
        fx::beam_explosion(w, &b, Some(id), [p[0], p[1], p[2] + 0.25, p[3]]);
        w.delete_moby(id);
        return;
    }
    fly(w, id, 0x30, 0x20);
    let p = c::pos(w, id);
    if !in_box(p) {
        w.delete_moby(id);
        return;
    }
    let r2 = w.m(id).rows[2];
    let tip = c::add(p, c::set_len3([r2[0], r2[1], r2[2], 0.0], f32::from_bits(0x3f05_1eb8)));
    if c::dec_timer_pvar_i32(w, id, 4) == 0 && w.coll_sphere(sv::pv(tip), Pf::f(0.5), 0, owner).is_none() {
        if w.rng.randi(2) == 0 { return; }
        spark_trail(w, id);
        return;
    }
    w.play_sound_as(0, 0, id, owner_class);
    let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 1.0, flash2: 0.5, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 5, sparks: 2, puffs: 4, debris: 3, sound: -1, shake: false };
    fx::beam_explosion(w, &b, Some(id), p);
    w.delete_moby(id);
}
