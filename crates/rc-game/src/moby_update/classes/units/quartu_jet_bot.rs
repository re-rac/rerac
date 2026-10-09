//! **Quartu's jet robots, class 491** (level15 `0x2cf3a8`; census U545, 15 placed). Giant Clank's enemies (hidden
//! unless Giant Clank plays, hero mode 2): a robot that flies in on its jets along a path (+0x170), then fights from the points of two jump
//! paths (+0x180 / +0x184): volleys of shots 819 ([`super::orxon_drone`]'s), a lunge and a swipe up close, a jump to
//! the other path when pushed or cornered. Destroyed it breaks apart and comes back when Giant Clank returns. Read from the
//! level15 decomp and disassembly; native `f32`.
//!
//! * **Every tick** (`0x2d0fa8`): scale = class · 1; the hit records aimed at it: one of the heavy kinds (0xa0000)
//!   but Ratchet's that are not 0x80000 (those are dropped) is taken (health +0x20; class 0x100's only flash; in the
//!   intro all flash; dead → 10 with the blast at joint 7 and its death bits (0x200); else the knock (4 far, 1 up,
//!   12 / 16 keys, seq 8) → 9, red 0x78; 9 / 10 red 0xfa); scale = class · 1.5; the target (range +0x174 = 90 asleep,
//!   +0x178 awake, inside its area +0x1d0 when it has one; beyond it none). Not Giant Clank playing: hidden, → 1.
//! * **0** health 12, mirrored at random, hidden, at its intro path's start → 1. **1** woken by its group +0x1cc
//!   gone, or Ratchet in its cuboid +0x1dc (shown as Giant Clank), with a target and an intro path: in its area → 2 (seq 4).
//! * **2** along the intro path at 12·dt (the jets on, seq 5 near the end); its end on the ground → 4. **3** a jump
//!   (lob to the point, 12·dt across, 10·dt² gravity; seq 5 before landing) → 4 on the ground.
//! * **4** standing (seq 0), turned to the target; after `ticks(15)`: within 20 (xy²)... → 7 (lunge), else → 5.
//! * **5** a volley (seq 6 then 7): a shot at each key-0 pass (30·dt, flat, `ticks(ticks(180))`; the muzzle flash 3
//!   ahead), six → 6; up close → 7 / 8; hit while far (10) → a jump to the other path. **6** waits `scale(+0x188·60)`
//!   then shoots again; cornered within 8 → a jump to the next point of its path.
//! * **7** the lunge (6.5·dt toward a place by the target, apart from the others; the ground move); within 12 → 8.
//! * **8** the swipe (keys 17..24: damage 4, flags 0x10007, a sphere of 2.5 at joint 1 and the swept lines).
//! * **9** knocked; down → a jump to the other path's point (or 4). **10** broken: three pieces 0x64a..0x64c, the
//!   shards (`0x2533c8`) → 0x40 hidden, its save bytes cleared, back to 0 when Giant Clank plays.
//! * **Tail**: flying: the jet hum (sound 7, loop) and 40 type-22 jet puffs from joints 2..5; else the hum off, and
//!   from state 2 on gravity (30·dt²) onto the ground and a touch hit (2.5, damage 50, flags 0x10000).
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2cf3a8` | 491 | [`update`] |
//! | `0x2d0fa8` | hits and the target | [`senses`] |
//! | `0x2cf110` | the lunge's place | [`lunge_point`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::breakables::burst_pieces;
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::creature::{self as c, attack, damage, flash, fx, knock, target, turn, walker, DT, DT2, SPEED};
use crate::moby_update::services::{pf, pv, HitTemplate, World};
use crate::moby_update::{manip, story};
use crate::ps2v::Pf;
use std::f32::consts::{FRAC_PI_3, PI};

pub const REFERENCE_LEVEL: u32 = 15;
pub const UPDATE_FN: u32 = 0x2c_f3a8;
pub const CLASSES: [i16; 1] = [491];

mod o {
    pub const D: usize = 0x20;
    pub const VEL: usize = 0x40;
    pub const F: usize = 0x60;
    pub const K: usize = 0x70;
    pub const FALL: usize = 0xe8;
    pub const TGT: usize = 0x120;
    pub const TMOBY: usize = 0x160;
    pub const KIND: usize = 0x164;
    pub const INTRO: usize = 0x170;
    pub const RANGE_ASLEEP: usize = 0x174;
    pub const RANGE: usize = 0x178;
    pub const PATHS: usize = 0x180;
    pub const WAIT_SECS: usize = 0x188;
    pub const LAST_JOINT: usize = 0x1a0;
    pub const SIDE: usize = 0x1b0;
    pub const POINT: usize = 0x1b4;
    pub const RANGE_NOW: usize = 0x1b8;
    pub const YAW_V: usize = 0x1bc;
    pub const TIMER: usize = 0x1c0;
    pub const SHOTS: usize = 0x1c4;
    pub const HIT: usize = 0x1c8;
    pub const GROUP: usize = 0x1cc;
    pub const AREA: usize = 0x1d0;
    pub const STILL: usize = 0x1d4;
    pub const NO_MELEE: usize = 0x1d8;
    pub const CUBOID: usize = 0x1dc;
    pub const VOICE: usize = 0x1e0;
    pub const HEAD: usize = 0x200;
    pub const SIZE: usize = 0x2a0;
}

/// Level15 gp −0x5100.. (af00..af3c).
mod k {
    pub const KNOCK_G: f32 = 50.0;
    pub const KNOCK_UP: f32 = 4.0;
    pub const KNOCK_SPEED: f32 = 4.0;
    pub const JUMP: f32 = 12.0;
    pub const GRAVITY: f32 = 10.0;
    pub const SHOT_LIFE: i32 = 0xb4;
    pub const SHOT: f32 = 30.0;
    pub const LUNGE: f32 = 20.0;
    pub const SWIPE: f32 = 12.0;
    pub const SCALE_A: f32 = 1.0;
    pub const SCALE_B: f32 = 1.5;
    pub const FIRE_SPEED: f32 = 1.0;
    pub const PIECE_OUT: f32 = -0.045;
    pub const PIECE_UP: f32 = 0.18;
}

const CLANK: u8 = 2;
const FLAME: i16 = 0x350;
const QUIET_CLASS: u16 = 0x100;
const DEATH: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 16.0, flash2: 8.0, flash_dist: 9.0, scale: 4.0, light: 32.0, streaks: 20, sparks: 6, puffs: 32, debris: 1, sound: -1, shake: true };

fn ptr(w: &World, id: MobyId, at: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, at) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn path_pts(w: &World, i: i32) -> Vec<[f32; 4]> {
    usize::try_from(i).ok().and_then(|i| w.svc.splines.get(i)).map(|s| s.iter().map(|q| q.map(f32::from_bits)).collect()).unwrap_or_default()
}
fn tpos(w: &World, id: MobyId) -> c::V { ptr(w, id, o::TMOBY).map_or(c::pv4(w, id, o::TGT), |m| c::pos(w, m)) }
fn kind(w: &World, id: MobyId) -> i32 { c::pi32(w, id, o::KIND) }
fn seq(w: &mut World, id: MobyId, s: u8, t: i32) {
    if w.m(id).anim.seq_b != s {
        let n = w.ticks(t);
        w.anim_blend(id, s, 0, n);
    }
}
fn wrapped(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn in_poly(w: &World, path: i32, p: c::V) -> bool { usize::try_from(path).ok().is_some_and(|a| c::region::point_in_polygon(w, a, p)) }
fn ground(w: &World, p: c::V) -> f32 { w.ground_height(Pf::f(0.5), pv(p), 0).to_f32() }

/// The turn to the target (`0x270cc0`: 500°·dt², 4π·dt).
fn face_target(w: &mut World, id: MobyId) {
    let (p, t) = (c::pos(w, id), tpos(w, id));
    let a = c::atan(t[0] - p[0], t[1] - p[1]);
    let (mut yaw, mut v) = (w.m(id).rotation[2], c::pf(w, id, o::YAW_V));
    turn::turn_toward(a, DT2 * 8.726_646, DT2 * 8.726_646, DT * 4.0 * PI, &mut yaw, &mut v);
    w.mm(id).rotation[2] = yaw;
    c::set_pf(w, id, o::YAW_V, v);
}

/// A jump (state 3, seq 3) to `point`: 12·dt across, the lob under 10·dt².
fn jump_to(w: &mut World, id: MobyId, point: c::V) {
    w.mm(id).state = 3;
    c::set_pi32(w, id, o::TIMER, 0);
    seq(w, id, 3, 0x14);
    let p = c::pos(w, id);
    let v = c::set_len2(c::sub(point, p), k::JUMP * DT);
    let mut t = 0.0;
    let up = knock::lob_up(k::JUMP * DT, -(k::GRAVITY * DT2), p, point, &mut t);
    c::set_pv4(w, id, o::VEL, [v[0], v[1], up, 0.0]);
}

/// The point `k` of jump path `side`.
fn jump_point(w: &World, id: MobyId, side: i32, k: i32) -> c::V {
    let pts = path_pts(w, c::pi32(w, id, o::PATHS + 4 * side.clamp(0, 1) as usize));
    pts.get(k.max(0) as usize).copied().unwrap_or(c::pos(w, id))
}

/// To the next point of its jump path when one is left (states 6 / 8).
fn retreat(w: &mut World, id: MobyId) -> bool {
    let side = c::pi32(w, id, o::SIDE);
    let n = path_pts(w, c::pi32(w, id, o::PATHS + 4 * side.clamp(0, 1) as usize)).len() as i32;
    if c::pi32(w, id, o::STILL) != 0 || n - 1 <= c::pi32(w, id, o::POINT) { return false; }
    let k = c::pi32(w, id, o::POINT) + 1;
    c::set_pi32(w, id, o::POINT, k);
    let pt = jump_point(w, id, side, k);
    jump_to(w, id, pt);
    true
}

/// To the other path's point (states 5 / 9).
fn switch_side(w: &mut World, id: MobyId, on_ground: bool) {
    let side = (c::pi32(w, id, o::SIDE) + 1) & 1;
    c::set_pi32(w, id, o::SIDE, side);
    let mut pt = jump_point(w, id, side, c::pi32(w, id, o::POINT));
    if on_ground { pt[2] = ground(w, pt); }
    jump_to(w, id, pt);
}

/// `0x2d0fa8` (module doc).
#[allow(clippy::too_many_lines)]
pub fn senses(w: &mut World, id: MobyId) {
    let cs = super::class_scale(w, w.m(id).o_class);
    w.mm(id).scale = cs * k::SCALE_A;
    c::set_pi32(w, id, o::HIT, 0);
    let hero = w.hero_moby;
    let mut found = None;
    for i in 0..w.svc.hits.records.len().min(0x40) {
        let r = w.svc.hits.records[i];
        if r.target != id { continue; }
        if r.flags & 0xa_0000 == 0 { continue; }
        if r.attacker != hero || r.flags & 0x8_0000 != 0 {
            found = Some(i);
            break;
        }
        w.svc.hits.records[i].target = usize::MAX;
    }
    let rec = found.map(|i| w.svc.hits.records[i]);
    let res = damage::resolve(w, id, rec, o::D, 0, 4);
    if let Some(r) = rec {
        let s = w.m(id).state;
        let from_flame = r.attacker.filter(|&a| a < w.table.mobys.len()).is_some_and(|a| w.m(a).o_class == FLAME);
        if res.out5 != 1 && s != 10 && s != 0x40 && !from_flame {
            let hp = c::pf(w, id, o::D) - res.damage;
            c::set_pf(w, id, o::D, hp);
            let mut reaction = if r.h2a == QUIET_CLASS { 9 } else { res.reaction };
            if s == 2 { reaction = 9; }
            if hp <= 0.0 { reaction = 1; }
            let kr = o::K;
            c::set_pi32(w, id, kr + knock::k::RADIUS, 2560);
            c::set_pf(w, id, kr + knock::k::ZOFF, 2.5);
            c::set_pf(w, id, kr + knock::k::GRAVITY, k::KNOCK_G * DT2);
            c::set_pf(w, id, kr + knock::k::DRAG, f32::from_bits(0x3a03_126f));
            c::set_pi32(w, id, kr + knock::k::FLAGS, 9);
            c::set_pu8(w, id, kr + 0x3d, 0);
            match reaction {
                1 | 2 => {
                    w.mm(id).mode &= !mode::TARGETABLE;
                    let j = w.joint_point(id, 7);
                    fx::beam_explosion(w, &DEATH, Some(id), j);
                    w.play_sound_as(0, 0, id, 0x79);
                    w.mm(id).state = 10;
                    c::set_pu8(w, id, o::F + 7, 0xf0);
                    set_death_bits(w, id, 0x200, -1);
                }
                3..=8 => {
                    c::set_pi32(w, id, o::HIT, 1);
                    c::set_pf(w, id, kr + knock::k::SPEED, k::KNOCK_SPEED * DT);
                    c::set_pf(w, id, kr + knock::k::UP, k::KNOCK_UP * DT);
                    knock::ballistic(w, id, 4.0, 1.0, kr);
                    c::set_pf(w, id, kr + knock::k::KEY_APEX, 12.0);
                    c::set_pf(w, id, kr + knock::k::KEY_LAND, 16.0);
                    let dir = r.dir.map(|x| x.to_f32());
                    let (mut sp, mut up) = (c::pf(w, id, kr + knock::k::SPEED), c::pf(w, id, kr + knock::k::UP));
                    let a = knock::aim(dir, &mut sp, &mut up);
                    c::set_pf(w, id, kr + knock::k::SPEED, sp);
                    c::set_pf(w, id, kr + knock::k::UP, up);
                    knock::start(w, id, kr, a, 8, 1, 0);
                    w.play_sound_as(0, 0, id, 0x79);
                    w.mm(id).state = 9;
                    c::set_pu8(w, id, o::F + 7, 0x78);
                }
                9 | 10 => {
                    c::set_pi32(w, id, o::HIT, 1);
                    c::set_pu8(w, id, o::F + 7, 0xfa);
                }
                _ => {}
            }
            flash::start(w, id, o::F);
        }
        if let Some(i) = found { w.svc.hits.records[i].target = usize::MAX; }
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, o::F);
    w.mm(id).scale = cs * k::SCALE_B;
    c::set_pi32(w, id, o::KIND, 0);
    let range = if w.m(id).state == 1 { c::pf(w, id, o::RANGE_ASLEEP) } else { c::pf(w, id, o::RANGE) };
    c::set_pf(w, id, o::RANGE_NOW, range);
    let area = if w.m(id).state != 1 { usize::try_from(c::pi32(w, id, o::AREA)).ok() } else { None };
    let mut t = target::acquire_in(w, id, range, area);
    if range < c::dist3(c::pos(w, id), t.pos) { t.kind = 2; }
    c::set_pv4(w, id, o::TGT, t.pos);
    c::set_pi32(w, id, o::TMOBY, t.moby.or(w.hero_moby).map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, o::KIND, t.kind as i32);
}

/// `0x2cf110(range, m, &goal)`: the target's place (the nearest robot), or a place 10 from it apart from the
/// robots within 3; itself within 1. Returns the heading to it.
pub fn lunge_point(w: &World, id: MobyId) -> (f32, c::V) {
    let me = c::pos(w, id);
    let tgt = c::pv4(w, id, o::TGT);
    let d = c::dist2(me, tgt);
    let mut goal = [0.0f32; 4];
    let mut n = 0.0f32;
    let mut nearest = true;
    for (k, m) in w.table.mobys.iter().enumerate() {
        if k == id || m.o_class != w.m(id).o_class || m.state >= 0xfd { continue; }
        if c::dist2(m.position, me) < 3.0 {
            n += 6.0;
            goal = c::add(goal, c::sub(me, m.position).map(|x| x * 6.0));
        }
        if c::dist2(m.position, tgt) < d { nearest = false; }
    }
    if nearest {
        goal = tgt;
    } else {
        let r = if d < 9.5 { 10.0 } else if 10.5 < d { -10.0 } else { 0.0 };
        n += 10.0;
        goal = c::add(goal, c::set_len3(c::sub(me, tgt), r * 10.0));
        goal = c::add(goal.map(|x| x / n), me);
    }
    if c::dist2(me, goal) < 1.0 {
        goal = me;
        return (c::atan(tgt[0] - me[0], tgt[1] - me[1]), goal);
    }
    (c::atan(goal[0] - me[0], goal[1] - me[1]), goal)
}

/// Level15 `0x2cf3a8` (module doc).
#[allow(clippy::too_many_lines)]
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    let mut flying = false;
    senses(w, id);
    if w.m(id).state >= 0xfd { return; }
    let s = w.m(id).state;
    if s != 0 && s != 0x40 && w.body() != CLANK {
        let m = w.mm(id);
        m.has_collision = false;
        m.mode |= 0x41;
        let v = c::pi32(w, id, o::VOICE);
        if v != -1 && w.sound_alive(v, id) { w.release_sound(v, id); }
        c::set_pi32(w, id, o::VOICE, -1);
        let m = w.mm(id);
        m.mode &= !mode::TARGETABLE;
        m.state = 1;
        c::set_pu8(w, id, 0x58, 0);
        return;
    }
    manip::big_head(w, 2.5, id, 6, id, o::HEAD);
    match s {
        0 => {
            c::set_pf(w, id, o::D, 12.0);
            c::set_pi16(w, id, o::D + 4, 12);
            c::set_pu8(w, id, o::D + 8, 3);
            let f = c::pi16(w, id, 0x3e) | 2;
            c::set_pi16(w, id, 0x3e, f);
            c::set_pf(w, id, 0x30, 6.0);
            c::set_pu8(w, id, o::D + 9, 0);
            w.mm(id).update_dist = 0xff;
            w.mm(id).draw_dist = 0xff;
            c::set_pi32(w, id, o::VOICE, -1);
            c::set_pf(w, id, o::YAW_V, 0.0);
            c::set_pf(w, id, o::RANGE_ASLEEP, 90.0);
            if w.rng.rand() & 1 != 0 { w.mm(id).mode |= 0x8000; }
            let m = w.mm(id);
            m.has_collision = false;
            m.mode = m.mode & 0xefff | 0x41;
            if c::pi32(w, id, o::PATHS) == -1 || c::pi32(w, id, o::PATHS + 4) == -1 { c::set_pi32(w, id, o::STILL, 1); }
            if let Some(p0) = path_pts(w, c::pi32(w, id, o::INTRO)).first() { w.mm(id).position = *p0; }
            w.mm(id).state = 1;
            if w.m(id).anim.seq_b != 0 { w.anim_blend(id, 0, 0, 5); }
        }
        1 => {
            let g = c::pi32(w, id, o::GROUP);
            let (proceed, show) = if g == -1 {
                (true, w.in_cuboid(w.hero_point(), c::pi32(w, id, o::CUBOID)))
            } else if crate::moby_update::scheduler::group_count(w, g, 0x40) != 0 {
                (false, false)
            } else {
                (true, true)
            };
            if show && w.body() == CLANK {
                let coll = super::class_collision(w, w.m(id).o_class);
                let m = w.mm(id);
                m.mode = m.mode & 0xffbe | mode::TARGETABLE;
                m.has_collision = coll;
            }
            if proceed && kind(w, id) != 2 && c::pi32(w, id, o::INTRO) != -1 {
                if w.body() != CLANK || w.m(id).mode & mode::TARGETABLE == 0 { return tail(w, id, flying); }
                let area = c::pi32(w, id, o::AREA);
                if area == -1 || in_poly(w, area, super::hero_pos(w)) {
                    w.mm(id).state = 2;
                    seq(w, id, 4, 0x14);
                    let pi = c::pi32(w, id, o::INTRO);
                    let pts = path_pts(w, pi);
                    for k in 0..pts.len().saturating_sub(2) {
                        let d = c::dist3(pts[k], pts[k + 1]);
                        w.svc.splines[pi as usize][k][3] = d.to_bits();
                    }
                    if let (Some(p0), Some(pl)) = (pts.first().copied(), pts.last().copied()) {
                        w.mm(id).position = p0;
                        let v = c::set_len2(c::sub(pl, p0), k::JUMP * DT);
                        let mut t = 0.0;
                        let up = knock::lob_up(k::JUMP * DT, -(k::GRAVITY * DT2), p0, pl, &mut t);
                        c::set_pv4(w, id, o::VEL, [v[0], v[1], up, 0.0]);
                    }
                } else {
                    let m = w.mm(id);
                    m.has_collision = false;
                    m.mode = m.mode & 0xefff | 0x41;
                }
            }
        }
        2 => {
            let pts = path_pts(w, c::pi32(w, id, o::INTRO));
            let n = pts.len();
            if n >= 2 {
                let p = c::pos(w, id);
                let speed = DT * 12.0;
                let cur = crate::spline::nearest(&pts, false, 1000.0, 5.0, 0.0, [p[0], p[1], p[2]]).map(|(_, k)| k).unwrap_or_default();
                let l0 = pts[0][3];
                let along = cur.seg as f32 * l0 + cur.t + 2.0;
                let seg = (along / l0) as i32;
                let f = (along - seg as f32 * l0) / l0;
                if n as i32 - 1 <= seg {
                    let gz = ground(w, p);
                    if w.m(id).position[2] < gz { w.mm(id).position[2] = gz; }
                    w.mm(id).state = 4;
                    let t = w.ticks(0xf);
                    c::set_pi32(w, id, o::TIMER, t);
                } else {
                    let near = speed / w.svc.timing.scale(Pf::f(26.0)).to_f32();
                    if c::dist3(p, pts[n - 1]) < near { seq(w, id, 5, 6); }
                    let (a, b) = (pts[seg as usize], pts[seg as usize + 1]);
                    let goal: c::V = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * f);
                    let next = seg as usize + 1;
                    if next < n.saturating_sub(8) {
                        let q = pts[next];
                        let mut v = c::pf(w, id, o::YAW_V);
                        let y = turn::spring_turn(w.m(id).rotation[2], c::atan(q[0] - p[0], q[1] - p[1]), 0.01, 0.3, 0.1, &mut v);
                        w.mm(id).rotation[2] = y;
                        c::set_pf(w, id, o::YAW_V, v);
                    }
                    let d = c::sub(goal, p);
                    let step = c::set_len3(d, c::len3(d).min(speed));
                    w.mm(id).position = c::add(p, step);
                    flying = true;
                }
            }
        }
        3 => {
            if w.m(id).anim.seq_b == 3 && wrapped(w, id) { seq(w, id, 4, 6); }
            let v = c::pv4(w, id, o::VEL);
            let p = c::add(c::pos(w, id), [v[0], v[1], v[2], 0.0]);
            w.mm(id).position = p;
            let vz = v[2] - k::GRAVITY * DT2;
            c::set_pf(w, id, o::VEL + 8, vz);
            if vz < 0.0 {
                let gz = ground(w, [p[0], p[1], p[2] + 1.0, p[3]]);
                let s26 = w.svc.timing.scale(Pf::f(26.0)).to_f32();
                if p[2] - gz < vz * s26 + k::GRAVITY * DT2 * 0.5 * s26 * s26 { seq(w, id, 5, 6); }
                if w.m(id).position[2] < gz {
                    w.mm(id).position[2] = gz;
                    w.mm(id).state = 4;
                    let t = w.ticks(0xf);
                    c::set_pi32(w, id, o::TIMER, t);
                }
            }
            flying = true;
        }
        4 => {
            if w.m(id).anim.seq_b != 0 && wrapped(w, id) { seq(w, id, 0, 0x14); }
            if kind(w, id) != 2 {
                face_target(w, id);
                if c::dec_timer_pvar_i32(w, id, o::TIMER) != 0 { close_or_shoot(w, id, true); }
            }
        }
        5 => volley(w, id),
        6 => {
            if kind(w, id) == 2 {
                to_stand(w, id);
            } else {
                face_target(w, id);
                let close = c::pi32(w, id, o::NO_MELEE) == 0 && c::dist2(c::pos(w, id), tpos(w, id)) < 8.0;
                if !(close && retreat(w, id)) && c::dec_timer_pvar_i32(w, id, o::TIMER) != 0 { start_volley(w, id); }
            }
        }
        7 => lunge(w, id),
        8 => swipe(w, id),
        9 => {
            let r = if c::pi32(w, id, o::NO_MELEE) == 0 { knock::update(w, id, o::K) } else {
                w.mm(id).anim.speed = 1.0;
                0
            };
            let done = r & 0x40 != 0 || (c::pi32(w, id, o::NO_MELEE) != 0 && wrapped(w, id));
            if done {
                if c::pi32(w, id, o::STILL) != 0 { to_stand(w, id); } else { switch_side(w, id, true); }
            }
        }
        10 => {
            let r = w.m(id).rows;
            let mut v = [r[0][0], r[0][1], r[0][2], 0.0].map(|x| x * k::PIECE_OUT * SPEED);
            v[2] += k::PIECE_UP * SPEED;
            let (p, rot) = (w.m(id).position, w.m(id).rotation);
            let t = w.ticks(0x5a);
            fx::break_piece_with(w, id, 0x64a, p, rot, t, 0, v, [0.0; 4], [0.0; 4]);
            fx::break_piece_with(w, id, 0x64b, p, rot, t, 0, v, [0.0; 4], [0.0; 4]);
            fx::break_piece(w, id, 0x64c, p, rot, 0, 0);
            burst_pieces(w, id, 0x7a9, 1, 0x7a9, 2, 4, 2);
            w.mm(id).state = 0x40;
            return;
        }
        0x40 => {
            let m = w.mm(id);
            m.has_collision = false;
            m.mode = m.mode & 0xefff | 0x41;
            c::set_pu8(w, id, 0x58, 0);
            let (lvl, sid) = (w.svc.level, w.m(id).spawn_id);
            w.svc.save.killed.remove(&sid);
            w.svc.save.collected.remove(&sid);
            w.svc.save.death.remove(&(lvl, sid));
            w.svc.save.death_level.remove(&sid);
            if w.body() != CLANK { w.mm(id).state = 0; }
        }
        _ => {}
    }
    tail(w, id, flying);
}

fn to_stand(w: &mut World, id: MobyId) {
    w.mm(id).state = 4;
    seq(w, id, 0, 0x14);
}

fn start_volley(w: &mut World, id: MobyId) {
    w.mm(id).state = 5;
    seq(w, id, 6, 0x14);
    c::set_pi32(w, id, o::SHOTS, 0);
}

/// Up close: the lunge (within 20) or the swipe (within 12); else the volley (`from_stand`) or nothing.
fn close_or_shoot(w: &mut World, id: MobyId, from_stand: bool) -> bool {
    if c::pi32(w, id, o::NO_MELEE) == 0 {
        let d = c::dist2(c::pos(w, id), tpos(w, id));
        if d < k::LUNGE {
            w.mm(id).state = 7;
            seq(w, id, 1, 10);
            return true;
        }
        if d < k::SWIPE {
            w.mm(id).state = 8;
            seq(w, id, 2, 10);
            return true;
        }
    }
    if from_stand { start_volley(w, id); }
    false
}

/// State 5 (module doc).
fn volley(w: &mut World, id: MobyId) {
    if w.m(id).anim.seq_b == 6 && wrapped(w, id) {
        seq(w, id, 7, 5);
        w.mm(id).anim.speed = k::FIRE_SPEED;
    }
    if kind(w, id) == 2 {
        to_stand(w, id);
        return;
    }
    face_target(w, id);
    if w.m(id).anim.seq_b == 7 && c::ground::passed_frame(w, id, 0.0) {
        let mz = w.joint_point(id, 0);
        let t = tpos(w, id);
        let mut v = c::sub(t, mz);
        v[2] += 0.5;
        v = c::set_len3(v, k::SHOT * DT);
        v[2] = 0.0;
        let life = w.ticks(k::SHOT_LIFE);
        let tm = ptr(w, id, o::TMOBY);
        super::orxon_drone::shot(w, 3.0, v, mz, id, tm, life);
        let off = c::set_len3(v, 3.0);
        let at = c::add(mz, off);
        super::orxon_drone::muzzle_flash(w, id, at);
        let n = c::pi32(w, id, o::SHOTS) + 1;
        c::set_pi32(w, id, o::SHOTS, n);
        return;
    }
    if 6 <= c::pi32(w, id, o::SHOTS) && wrapped(w, id) {
        w.mm(id).state = 6;
        seq(w, id, 0, 10);
        let t = w.svc.timing.scale(Pf::f(c::pf(w, id, o::WAIT_SECS) * 60.0)).to_i32();
        c::set_pi32(w, id, o::TIMER, t);
        return;
    }
    if close_or_shoot(w, id, false) { return; }
    let far = c::pi32(w, id, o::NO_MELEE) == 0 && 10.0 < c::dist2(c::pos(w, id), tpos(w, id));
    if far && c::pi32(w, id, o::HIT) != 0 && c::pi32(w, id, o::STILL) == 0 { switch_side(w, id, false); }
}

/// State 7 (module doc).
fn lunge(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    let t = tpos(w, id);
    let (mut a, mut goal) = (c::atan(t[0] - p[0], t[1] - p[1]), t);
    let g = DT2;
    if kind(w, id) == 2 {
        w.mm(id).state = 4;
        seq(w, id, 0, 0x14);
    } else {
        (a, goal) = lunge_point(w, id);
        if c::dist2(p, t) < k::SWIPE {
            w.mm(id).state = 8;
            seq(w, id, 2, 10);
        }
    }
    let mut v = c::pf(w, id, o::YAW_V);
    turn::spring_turn2(w, id, a, g * FRAC_PI_3, g * PI, DT * PI, &mut v);
    c::set_pf(w, id, o::YAW_V, v);
    let mut from = c::pos(w, id);
    let mut to = c::add(from, c::set_len3(c::sub(goal, from), DT * 6.5));
    let r = walker::move_ground(w, id, 2.25, 1.75, 0.5, std::f32::consts::FRAC_PI_6, &mut from, &mut to, 1);
    w.mm(id).position = from;
    if r == 0 || in_poly(w, c::pi32(w, id, o::SIDE), from) {
        w.mm(id).state = 4;
        seq(w, id, 0, 0xc);
    }
}

/// State 8 (module doc).
fn swipe(w: &mut World, id: MobyId) {
    face_target(w, id);
    let p = c::pos(w, id);
    let d = c::dist2(p, tpos(w, id));
    if wrapped(w, id) {
        if d < k::SWIPE && retreat(w, id) { return; }
        if k::SWIPE < d {
            w.mm(id).state = 4;
            seq(w, id, 0, 0x14);
        }
        return;
    }
    let kt = c::ground::key_time(w, id);
    let jp = w.joint_point(id, 1);
    if (17.0..=24.0).contains(&kt) {
        let base = [p[0], p[1], p[2] + 0.5, p[3]];
        let y = w.m(id).rotation[2];
        let tmpl = HitTemplate { dir: [pf(y.cos()), pf(y.sin()), Pf::ONE, Pf::ZERO], attacker: Some(id), flags: 0x1_0007, h1a: w.m(id).o_class as u16, damage: Pf::f(4.0), w20: 1, ..Default::default() };
        w.sphere_mobys(Pf::f(2.5), pv(jp), 9, Some(id), Some(&tmpl));
        crate::moby_update::services::line_hit_in(w.table, w.svc, w.classes, w.coll, pv(base), pv(jp), 0, Some(id), &tmpl);
        let last = c::pv4(w, id, o::LAST_JOINT);
        attack::swept_lines(w, base, jp, base, last, id, &tmpl, 5);
        c::set_pv4(w, id, o::LAST_JOINT, jp);
    }
}

/// The tail (module doc).
fn tail(w: &mut World, id: MobyId, flying: bool) {
    let v = c::pi32(w, id, o::VOICE);
    if !flying {
        if v != -1 && w.sound_alive(v, id) { w.release_sound(v, id); }
        c::set_pi32(w, id, o::VOICE, -1);
        let s = w.m(id).state;
        if s < 2 || s == 0x40 { return; }
        let f = c::pf(w, id, o::FALL) + DT2 * 30.0;
        c::set_pf(w, id, o::FALL, f);
        w.mm(id).position[2] -= f - 2.0;
        let gz = ground(w, c::pos(w, id));
        let z = w.m(id).position[2] - 2.0;
        w.mm(id).position[2] = z;
        if z < gz {
            w.mm(id).position[2] = gz;
            c::set_pf(w, id, o::FALL, 0.0);
        }
        let p = c::pos(w, id);
        attack::sphere_hit(w, 2.5, 50.0, 1.0, id, p, 0x1_0000, 0, 1, 0);
        return;
    }
    if !w.sound_alive(v, id) {
        let nv = w.play_sound(7, 4, id);
        c::set_pi32(w, id, o::VOICE, nv);
    }
    let r = w.m(id).rows;
    for _ in 1..=10 {
        for j in 2..6 {
            let x = w.rng.randf(1.0, -1.0) * DT * 20.0;
            let y = w.rng.randf(1.0, -1.0) * DT * 20.0;
            let z = w.rng.randf(f32::from_bits(0x3f4c_cccd), f32::from_bits(0x3f99_999a)) * DT * -20.0;
            let pick = w.rng.randi(100);
            let at = w.joint_point(id, j);
            let vel: c::V = std::array::from_fn(|k| if k == 3 { 0.0 } else { r[0][k] * x + r[1][k] * y + r[2][k] * z });
            let rgba = if pick & 1 != 0 { 0x400f_0f7f } else { 0x407f_7f7f };
            let size = w.rng.randf(1.5, 3.0);
            let n = if pick & 1 == 0 { w.rng.rand_range(0x14, 0x1e) } else { w.rng.rand_range(10, 0xf) };
            let life = w.ticks(n);
            let s = crate::particles::type22::Spawn { size: size * 210_000.0, pos: at, vel, c1: rgba, c2: 0x27_2727, life };
            crate::moby_update::creature::projectile::part22(w, &s);
        }
    }
}
