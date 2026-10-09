//! **Quartu's flame drones, class 233** (level15 `0x2c5630`; census U543, 28 placed). Hovering drones with a rider 1371
//! ([`super::drone_rider`]) that spray flame streams 848 ([`super::hover_ship::spawn_flame`]) at their target. Some
//! are Giant Clank's (+0x1a4: hidden unless Giant Clank is playing, hero mode 2); a destroyed one is set up again for
//! the next time once Giant Clank is gone. Read from the level15 decomp and disassembly; native `f32`.
//!
//! * **Every tick**: no rider → one made; a Clank drone while Ratchet plays: hidden in state 1. The hits (`0x2c6900`:
//!   mask 0x330000, health +0x20; dead → 7; else red 0xb4 and, pushed exactly (w 5627.925), tipped away and stunned
//!   (6) for `ticks(60)`); the target (`0x274b78` in its waking range +0x198 in state 1, else +0x19c = 60, inside its
//!   area +0x194 when it has one; beyond the range none); the record's +0xe not 1 → gone with its rider.
//! * **0**: 7.5 up, health 3, collision off, its flight path's chords (+0x1ac), half size (its rider too), the head's
//!   manipulators (lists 0 / 1), hidden → 1.
//! * **1**: wakes (shown, collision) when its group +0x190 has no one left, or Ratchet is in its cuboid +0x1b0 (or it
//!   has none) with the target in range → 2 along its path, or 3.
//! * **2**: along its path toward its end, speeding up to 10·dt (10·dt² a tick, braking to stop there), turned to the
//!   path ahead (the target over the last 8 points); at the end or inside its area → 5 (the target) or 4.
//! * **3**: to the last point of its area (at the target's height + 7.5) → 5 / 4 as above. **4**: holds, facing the
//!   target, → 5 inside its area, else 3.
//! * **5**: hovers round the target (`0x2c52b8`: 10 away for the nearest drone, 15 for the rest, apart from the
//!   others), and every `ticks(60)` with no flame out fires one (6·dt flat at the target, `ticks(240)`).
//! * **6**: stunned: drifts back (−4·dt² a tick) tipped, then back to what it did. **7**: destroyed: the rider knocked
//!   off, the explosion (20 streaks, a shake, sound 2), its death bits (0x200) → 0x40.
//! * **0x40**: hidden; its save bytes cleared; when Giant Clank is not playing it comes back (→ 0, at its place).
//! * **Tail**: the shadow (within 40 of the camera); the height change at most 10·dt (but in 1); a bob (`ae0c`,
//!   period `ae08` seconds); the rider posed on it; the head turned to the live flame, else the target.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2c5630` | 233 | [`update`] |
//! | `0x2c6900` | hits | [`hits`] |
//! | `0x2c4f88` / `0x2c52b8` | the hover move, the ring round the target | [`hover_to`], [`ring`] |
//! | `0x247e90` | the move with the walls pushed out | [`slide`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::creature::{self as c, damage, flash, fx, target, turn, DT, DT2};
use crate::moby_update::services::{pv, World};
use crate::moby_update::{manip, story};
use crate::ps2v::Pf;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, FRAC_PI_6, PI, TAU};

pub const REFERENCE_LEVEL: u32 = 15;
pub const UPDATE_FN: u32 = 0x2c_5630;
pub const CLASSES: [i16; 1] = [233];

mod o {
    pub const D: usize = 0x20;
    pub const KILL: usize = 0x2e;
    pub const F: usize = 0x60;
    pub const YAW_NODE: usize = 0xd0;
    pub const PITCH_NODE: usize = 0x110;
    pub const VEL: usize = 0x150;
    pub const RIDER: usize = 0x160;
    pub const FLAME: usize = 0x164;
    pub const FIRE_T: usize = 0x168;
    pub const STUN_T: usize = 0x16a;
    pub const BOB: usize = 0x16c;
    pub const SPEED: usize = 0x170;
    pub const TILT_X_V: usize = 0x174;
    pub const TILT_Y_V: usize = 0x178;
    pub const YAW_V: usize = 0x17c;
    pub const HEAD_YAW: usize = 0x180;
    pub const HEAD_YAW_V: usize = 0x184;
    pub const HEAD_PITCH: usize = 0x188;
    pub const HEAD_PITCH_V: usize = 0x18c;
    pub const GROUP: usize = 0x190;
    pub const AREA: usize = 0x194;
    pub const WAKE_R: usize = 0x198;
    pub const RANGE: usize = 0x19c;
    pub const RANGE_NOW: usize = 0x1a0;
    pub const CLANK: usize = 0x1a4;
    pub const HOME_Z: usize = 0x1a8;
    pub const PATH: usize = 0x1ac;
    pub const CUBOID: usize = 0x1b0;
    pub const SIZE: usize = 0x1b4;
}

/// gp−0x51f4 / −0x51f8: the bob's height and period (seconds).
const BOB_H: f32 = f32::from_bits(0x3e2e_147b);
const BOB_PERIOD: f32 = 2.0;
const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.0, flash_dist: 100_000.0, scale: 3.0, light: 15.0, streaks: 20, sparks: 3, puffs: 4, debris: 1, sound: 2, shake: true };

fn ptr(w: &World, id: MobyId, at: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, at) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn set_ptr(w: &mut World, id: MobyId, at: usize, m: Option<MobyId>) { c::set_pi32(w, id, at, m.map_or(0, |m| m as i32 + 1)); }
fn path_pts(w: &World, i: i32) -> Vec<[f32; 4]> {
    usize::try_from(i).ok().and_then(|i| w.svc.splines.get(i)).map(|s| s.iter().map(|q| q.map(f32::from_bits)).collect()).unwrap_or_default()
}
fn rider(w: &World, id: MobyId) -> Option<MobyId> { ptr(w, id, o::RIDER) }
fn hide(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.has_collision = false;
    m.mode = m.mode & 0xefff | 0x41;
    c::set_pu8(w, id, 0x58, 0);
    if let Some(r) = rider(w, id) { w.mm(r).mode |= 0x41; }
}
fn in_area(w: &World, id: MobyId, p: c::V) -> bool {
    usize::try_from(c::pi32(w, id, o::AREA)).ok().is_some_and(|a| c::region::point_in_polygon(w, a, p))
}

/// `0x247e90(up, r, m, &vel, flags)`: position += velocity; six times the sphere `r` at `up` above it pushes it out of
/// what it touches steeper than 30°; the velocity becomes the move made.
pub fn slide(w: &mut World, id: MobyId, up: f32, r: f32, vel: &mut c::V, flags: u32) {
    let old = c::pos(w, id);
    let mut p = c::add(old, [vel[0], vel[1], vel[2], 0.0]);
    w.mm(id).position = p;
    for _ in 0..6 {
        let ctr = [p[0], p[1], p[2] + up, p[3]];
        let Some(h) = w.coll_sphere(pv(ctr), Pf::f(r), (flags & 0x10) << 1 | 4, Some(id)) else { break };
        let n = h.normal;
        if FRAC_PI_6 < c::atan(n[2], (n[0] * n[0] + n[1] * n[1]).sqrt()) {
            if let Some(pc) = h.pushed_centre {
                p = [pc[0], pc[1], pc[2] - up, p[3]];
                w.mm(id).position = p;
            }
        }
    }
    *vel = c::sub(p, old);
}

/// The speed toward a stop at `d` (10·dt² a tick, at most 10·dt).
fn speed_for(w: &mut World, id: MobyId, d: f32) {
    let v = c::pf(w, id, o::SPEED);
    let a = DT2 * 10.0;
    let n = if d <= v * v / (a + a) { (v - a).max(0.0) } else { (v + a).min(DT * 10.0) };
    c::set_pf(w, id, o::SPEED, n);
}

/// The tilt from the move `step` across the heading (`LAB_002c6184`).
fn tilt(w: &mut World, id: MobyId, step: c::V) {
    let rel = c::sub_rot(c::atan(step[0], step[1]), w.m(id).rotation[2]);
    let (cs, sn) = (rel.cos(), rel.sin());
    let v = c::pf(w, id, o::SPEED);
    let k = DT * 10.0;
    let ty = v * 0.349_065_84 * cs / k;
    let tx = v * -0.349_065_84 * sn / k;
    let (mut vy, mut vx) = (c::pf(w, id, o::TILT_Y_V), c::pf(w, id, o::TILT_X_V));
    let ry = turn::spring_turn(w.m(id).rotation[1], ty, DT2 * FRAC_PI_6, DT2 * FRAC_PI_3, DT * FRAC_PI_4, &mut vy);
    let rx = turn::spring_turn(w.m(id).rotation[0], tx, DT2 * FRAC_PI_3, DT2 * 2.094_395_2, DT * FRAC_PI_2, &mut vx);
    let m = w.mm(id);
    m.rotation[1] = ry;
    m.rotation[0] = rx;
    c::set_pf(w, id, o::TILT_Y_V, vy);
    c::set_pf(w, id, o::TILT_X_V, vx);
}

fn face_yaw(w: &mut World, id: MobyId, at: c::V) {
    let p = c::pos(w, id);
    let mut v = c::pf(w, id, o::YAW_V);
    let y = turn::spring_turn(w.m(id).rotation[2], c::atan(at[0] - p[0], at[1] - p[1]), 0.01, 0.3, 0.1, &mut v);
    w.mm(id).rotation[2] = y;
    c::set_pf(w, id, o::YAW_V, v);
}

/// `0x2c4f88(m, goal, face)`: toward `goal` (the speed braking to stop there; the drift ×0.9 plus 0.1 of the speed
/// toward it; walls pushed out), held at its height, turned to `face`, tipped by the move.
pub fn hover_to(w: &mut World, id: MobyId, goal: c::V, at: c::V) {
    let old = c::pos(w, id);
    speed_for(w, id, c::dist3(old, goal));
    let s = c::set_len3(c::sub(goal, old), c::pf(w, id, o::SPEED) * 0.1);
    let mut v = c::pv4(w, id, o::VEL).map(|x| x * 0.9);
    v = c::add(v, s);
    slide(w, id, 1.0, 2.0, &mut v, 0);
    let hz = c::pf(w, id, o::HOME_Z);
    w.mm(id).position[2] = hz;
    face_yaw(w, id, at);
    let d = c::sub(c::pos(w, id), old);
    c::set_pv4(w, id, o::VEL, d);
    tilt(w, id, d);
}

/// `0x2c52b8(m, &goal, target)`: the place round the target: away from the drones within 10, at 10 from the target
/// for the drone nearest it (else 15), averaged; itself when within 1.
pub fn ring(w: &World, id: MobyId, tgt: c::V) -> c::V {
    let me = c::pos(w, id);
    let d = c::dist2(me, tgt);
    let mut goal = [0.0f32; 4];
    let mut n = 0.0f32;
    let mut nearest = true;
    for (k, m) in w.table.mobys.iter().enumerate() {
        if k == id || m.o_class != w.m(id).o_class || m.state >= 0xfd || matches!(m.state, 1 | 0x40) { continue; }
        let q = m.position;
        if c::dist2(q, me) < 10.0 {
            n += 6.0;
            goal = c::add(goal, c::sub(me, q).map(|x| x * 6.0));
        }
        if c::dist2(q, tgt) < d { nearest = false; }
    }
    let (r, lo, hi) = if nearest { (10.0, 9.5, 10.5) } else { (15.0, 14.5, 15.5) };
    let k = if d < lo { r } else if hi < d { -r } else { 0.0 };
    n += 10.0;
    goal = c::add(goal, c::set_len3(c::sub(me, tgt), k * 10.0));
    goal = c::add(goal.map(|x| x / n), me);
    if c::dist2(me, goal) < 1.0 { me } else { goal }
}

/// `0x2c6900` (module doc).
pub fn hits(w: &mut World, id: MobyId) {
    let hit = w.get_hit(id, 0x33_0000, false);
    let r = damage::resolve(w, id, hit, o::D, 0, 4);
    let s = w.m(id).state;
    if r.out5 != 1 && s != 7 && s != 0x40 {
        let hp = c::pf(w, id, o::D) - r.damage;
        c::set_pf(w, id, o::D, hp);
        if r.reaction == 1 || hp <= 0.0 {
            w.mm(id).state = 7;
        } else if r.reaction != 0xb {
            c::set_pu8(w, id, o::F + 7, 0xb4);
            flash::start(w, id, o::F);
            if let Some(h) = hit.filter(|h| h.dir[3].to_f32() == 5627.925 && s != 6) {
                let p = c::pos(w, id);
                let hp_ = h.pos.map(|x| x.to_f32());
                let rel = c::sub_rot(c::atan(p[0] - hp_[0], p[1] - hp_[1]), w.m(id).rotation[2]);
                let (cs, sn) = (rel.cos(), rel.sin() * -0.349_065_84);
                if w.m(id).rotation[0].abs() < sn.abs() {
                    w.mm(id).rotation[0] = sn;
                    c::set_pf(w, id, o::TILT_X_V, 0.0);
                }
                if w.m(id).rotation[1].abs() < (cs * 0.087_266_46).abs() {
                    w.mm(id).rotation[1] = cs * 0.087_266_46;
                    c::set_pf(w, id, o::TILT_Y_V, 0.0);
                }
                c::set_pf(w, id, o::SPEED, DT * 4.0);
                let t = w.ticks(0x3c);
                c::set_pi16(w, id, o::STUN_T, t as i16);
                let m = w.mm(id);
                m.cmd = m.state;
                m.state = 6;
            }
        }
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, o::F);
}

/// Level15 `0x2c5630` (module doc).
#[allow(clippy::too_many_lines)]
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    let old_z = w.m(id).position[2];
    if rider(w, id).is_none() {
        let r = super::drone_rider::spawn(w, id);
        set_ptr(w, id, o::RIDER, r);
    }
    let s0 = w.m(id).state;
    if s0 != 0 && s0 != 0x40 && c::pi32(w, id, o::CLANK) != 0 && w.body() != 2 {
        hide(w, id);
        w.mm(id).state = 1;
        return;
    }
    hits(w, id);
    let range = if w.m(id).state == 1 { c::pf(w, id, o::WAKE_R) } else { c::pf(w, id, o::RANGE) };
    c::set_pf(w, id, o::RANGE_NOW, range);
    let area = usize::try_from(c::pi32(w, id, o::AREA)).ok();
    let mut t = if w.m(id).state != 1 && area.is_some() { target::acquire_in(w, id, range, area) } else { target::acquire(w, id, range) };
    if range < c::dist3(c::pos(w, id), t.pos) { t.kind = 2; }
    if t.moby.is_none() { t.moby = w.hero_moby; }
    let tm = t.moby.map_or(t.pos, |m| c::pos(w, m));
    let st = w.m(id).state;
    if st != 0 && c::pu8(w, id, o::KILL) != 1 {
        if let Some(r) = rider(w, id) { w.delete_moby(r); }
        w.delete_moby(id);
        return;
    }
    match st {
        0 => {
            let z = w.m(id).position[2] + 7.5;
            w.mm(id).position[2] = z;
            w.mm(id).b7f = 0x20;
            c::set_pf(w, id, o::HOME_Z, z);
            c::set_pf(w, id, o::D, 3.0);
            c::set_pi16(w, id, o::D + 4, 3);
            c::set_pu8(w, id, o::D + 8, 2);
            c::set_pu8(w, id, 0x58, 0x19);
            c::set_pf(w, id, 0x30, 1.0);
            c::set_pu8(w, id, 0x59, 1);
            c::set_pu8(w, id, 0x5a, 6);
            c::set_pu8(w, id, o::KILL, 1);
            w.mm(id).update_dist = 0xff;
            w.mm(id).draw_dist = 0x80;
            let pi = c::pi32(w, id, o::PATH);
            if pi != -1 {
                let pts = path_pts(w, pi);
                let n = pts.len();
                for k in 0..n.saturating_sub(2) {
                    let d = c::dist3(pts[k], pts[k + 1]);
                    w.svc.splines[pi as usize][k][3] = d.to_bits();
                }
                if let Some(p0) = pts.first() { w.mm(id).position = *p0; }
            }
            w.mm(id).state = 1;
            for k in [o::SPEED, o::TILT_X_V, o::TILT_Y_V, o::YAW_V] { c::set_pf(w, id, k, 0.0); }
            c::set_pf(w, id, o::RANGE, 60.0);
            manip::attach(w, id, 0, id, o::YAW_NODE);
            manip::attach(w, id, 1, id, o::PITCH_NODE);
            w.mm(id).scale *= 0.5;
            if let Some(r) = rider(w, id) {
                let m = w.mm(r);
                m.update_dist = 0;
                m.mode &= 0xffbe;
                m.scale *= 0.5;
            }
            hide(w, id);
            if rider(w, id).is_some() { return; }
        }
        1 => {
            let g = c::pi32(w, id, o::GROUP);
            let wake = if g != -1 {
                crate::moby_update::scheduler::group_count(w, g, 0x40) == 0
            } else {
                let cub = c::pi32(w, id, o::CUBOID);
                let inside = w.in_cuboid(w.hero_point(), cub);
                (cub == -1 || inside) && t.kind != 2
            };
            if wake {
                let coll = super::class_collision(w, w.m(id).o_class);
                let m = w.mm(id);
                m.mode = m.mode & 0xffbe | mode::TARGETABLE;
                m.has_collision = coll;
                c::set_pu8(w, id, 0x58, 0x19);
                w.mm(id).state = if c::pi32(w, id, o::PATH) == -1 { 3 } else { 2 };
                if let Some(r) = rider(w, id) { w.mm(r).mode &= 0xffbe; }
                return tail(w, id, old_z, tm);
            }
        }
        2 => {
            let pts = path_pts(w, c::pi32(w, id, o::PATH));
            let n = pts.len();
            if n < 2 { return tail(w, id, old_z, tm); }
            let p = c::pos(w, id);
            let cur = crate::spline::nearest(&pts, false, 1000.0, 5.0, 0.0, [p[0], p[1], p[2]]).map(|(_, k)| k).unwrap_or_default();
            let seg0 = pts[0][3];
            let along = cur.seg as f32 * seg0 + cur.t + 2.0;
            speed_for(w, id, c::dist3(p, pts[n - 1]));
            let mut seg = (along / seg0) as i32;
            let mut f = (along - seg as f32 * seg0) / seg0;
            if n as i32 - 1 <= seg {
                seg = n as i32 - 2;
                f = 1.0;
            }
            let (a, b) = (pts[seg as usize], pts[seg as usize + 1]);
            let goal: c::V = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * f);
            if c::dist2(pts[n - 1], p) < 1.0 || in_area(w, id, p) {
                w.mm(id).state = if t.kind == 2 { 4 } else { 5 };
                c::set_pi16(w, id, o::FIRE_T, 0);
            }
            let next = seg + 1;
            let look = if next < n as i32 - 8 { pts[next as usize] } else { t.pos };
            face_yaw(w, id, look);
            let d = c::sub(goal, p);
            let l = c::len3(d).min(c::pf(w, id, o::SPEED));
            let step = c::set_len3(d, l);
            w.mm(id).position = c::add(p, step);
            tilt(w, id, step);
        }
        3 => {
            let mut goal = tm;
            let pts = path_pts(w, c::pi32(w, id, o::AREA));
            let p = c::pos(w, id);
            for q in &pts {
                if c::dist2(*q, p) < 9999.999 { goal = *q; }
            }
            goal[2] = tm[2] + 7.5;
            hover_to(w, id, goal, tm);
            if c::dist2(goal, c::pos(w, id)) < 1.0 || in_area(w, id, c::pos(w, id)) {
                if t.kind == 2 {
                    w.mm(id).state = 4;
                } else {
                    w.mm(id).state = 5;
                    c::set_pi16(w, id, o::FIRE_T, 0);
                }
            }
        }
        4 => {
            let p = c::pos(w, id);
            hover_to(w, id, p, tm);
            if t.kind != 2 {
                if in_area(w, id, c::pos(w, id)) {
                    w.mm(id).state = 5;
                    c::set_pi16(w, id, o::FIRE_T, 0);
                } else {
                    w.mm(id).state = 3;
                }
            }
        }
        5 => {
            let muzzle = w.joint_point(id, 0);
            if t.kind == 2 {
                let p = c::pos(w, id);
                hover_to(w, id, p, t.pos);
                w.mm(id).state = 4;
            } else {
                let goal = ring(w, id, t.pos);
                hover_to(w, id, goal, tm);
                if c::dec_timer_pvar_s16(w, id, o::FIRE_T) != 0 && c::pi32(w, id, o::FLAME) == 0 {
                    let n = w.ticks(0x3c);
                    c::set_pi16(w, id, o::FIRE_T, n as i16);
                    let mut v = c::set_len2(c::sub(t.pos, muzzle), DT * 6.0);
                    v[2] = 0.0;
                    let life = w.ticks(0xf0);
                    let f = super::hover_ship::spawn_flame(w, t.pos[2], v, muzzle, id, life);
                    set_ptr(w, id, o::FLAME, f);
                }
            }
        }
        6 => {
            if c::dec_timer_pvar_s16(w, id, o::STUN_T) == 0 {
                let v = (c::pf(w, id, o::SPEED) - DT2 * 4.0).max(0.0);
                c::set_pf(w, id, o::SPEED, v);
                let y = w.m(id).rotation[2];
                let step = [y.cos() * -v, y.sin() * -v, 0.0, 0.0];
                let p = c::pos(w, id);
                w.mm(id).position = c::add(p, step);
                tilt(w, id, step);
            } else {
                let m = w.mm(id);
                m.state = m.cmd;
            }
        }
        7 => {
            let _ = w.rng.rand_angle();
            if let Some(r) = rider(w, id) {
                super::drone_rider::knock_off(w, id, r);
                c::set_pi32(w, id, o::RIDER, 0);
            }
            let p = c::pos(w, id);
            fx::beam_explosion(w, &BLAST, Some(id), p);
            set_death_bits(w, id, 0x200, -1);
            manip::detach(w, id, id, o::YAW_NODE);
            manip::detach(w, id, id, o::PITCH_NODE);
            w.mm(id).state = 0x40;
            return;
        }
        0x40 => {
            hide(w, id);
            let (lvl, sid) = (w.svc.level, w.m(id).spawn_id);
            w.svc.save.killed.remove(&sid);
            w.svc.save.collected.remove(&sid);
            w.svc.save.death.remove(&(lvl, sid));
            w.svc.save.death_level.remove(&sid);
            if w.body() == 2 { return; }
            let hz = c::pf(w, id, o::HOME_Z);
            let m = w.mm(id);
            m.state = 0;
            m.scale += m.scale;
            m.position[2] = hz - 7.5;
            return;
        }
        _ => {}
    }
    tail(w, id, old_z, tm);
}

/// The tail (module doc).
fn tail(w: &mut World, id: MobyId, old_z: f32, tm: c::V) {
    if w.m(id).visible != 0 {
        let cam = w.camera_point();
        if c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 40.0 { crate::shadows::probe_down(w, id); }
    }
    if w.m(id).state != 1 {
        let z = w.m(id).position[2];
        let k = DT * 10.0;
        if k < (z - old_z).abs() { w.mm(id).position[2] = if old_z < z { old_z + k } else { old_z - k }; }
    }
    let b = c::pf(w, id, o::BOB);
    w.mm(id).position[2] -= BOB_H * b.cos();
    let per = w.svc.timing.scale(Pf::f(BOB_PERIOD * 60.0)).to_f32();
    let nb = c::add_rot(b, TAU / per);
    c::set_pf(w, id, o::BOB, nb);
    w.mm(id).position[2] += BOB_H * nb.cos();
    if let Some(r) = rider(w, id) {
        let (p, rot) = (w.m(id).position, w.m(id).rotation);
        let m = w.mm(r);
        m.position = p;
        m.rotation = rot;
        m.mode |= 6;
        w.build_matrix(r);
    }
    let flame = ptr(w, id, o::FLAME).filter(|&f| w.m(f).o_class == super::hover_ship::FLAME && w.m(f).state < 0xfd);
    let at = flame.map_or(tm, |f| c::pos(w, f));
    if flame.is_none() { c::set_pi32(w, id, o::FLAME, 0); }
    let j = w.joint_point(id, 0);
    let d = c::sub(at, j);
    let r = w.m(id).rows;
    let l: [f32; 3] = std::array::from_fn(|k| r[k][0] * d[0] + r[k][1] * d[1] + r[k][2] * d[2]);
    let (mut yv, mut pv_) = (c::pf(w, id, o::HEAD_YAW_V), c::pf(w, id, o::HEAD_PITCH_V));
    let y = turn::spring_turn(c::pf(w, id, o::HEAD_YAW), c::atan(l[0], l[1]), DT2 * TAU, DT2 * 4.0 * PI, DT * TAU, &mut yv);
    let pt = turn::spring_turn(c::pf(w, id, o::HEAD_PITCH), -c::atan((l[0] * l[0] + l[1] * l[1]).sqrt(), l[2]), DT2 * PI, DT2 * TAU, DT * FRAC_PI_2, &mut pv_);
    c::set_pf(w, id, o::HEAD_YAW, y);
    c::set_pf(w, id, o::HEAD_YAW_V, yv);
    c::set_pf(w, id, o::HEAD_PITCH, pt);
    c::set_pf(w, id, o::HEAD_PITCH_V, pv_);
    manip::set_axis(w, id, id, o::YAW_NODE, y, 2);
    manip::set_axis(w, id, id, o::PITCH_NODE, pt, 2);
}
