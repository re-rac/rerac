//! **Blarg's mini-boss, class 1051** (level06 `0x2f9a28` with its hits `0x2fa9c0`, its cutaways `0x2fb148` /
//! `0x2fb2a8` / `0x2fb4b0` (and the copies for its later phases), its circling `0x2fbc88` and the crawler drop
//! `0x2fb090`; 1 placed; census U233; the name is descriptive [L]). It waits 17.5 above its arena; when Ratchet comes
//! (a fade) it drops in on a cutaway, the petal doors +0xfc / +0x100 shut and the sliders +0x104 rise, and the fight
//! starts under a boss meter (30 health, ×16). It runs at him, rears up and slams (seven joint spheres) and rests,
//! circling, every 1200 ticks; it keeps Ratchet inside its arena (the spline wall +0x108). At 20 health a fade and a
//! cutaway: the crawlers 827 of group +0xf0 rain down (again whenever one hides) and it circles until they are all
//! gone; at 10, the same with the troopers 1048 of group +0xf4. Killed: an explosion, pieces, the doors and sliders
//! open, the checkpoint, and a banner (0x53d6) when Ratchet never fired anything but item 8; deleted once the
//! following scene runs. Some weapons hurt it more (classes 0xb0, 0x79, 0x33b ×1.5; 0x131 ×0.666). Read from the
//! level06 decomp; its words gp−0x4d10..−0x4cec. Native `f32`.
//!
//! **Pvars** (0x1b0; the header: damage +0x20, flash +0x60): +0x70 the target record (+0xb0 its moby, here the runtime
//! index + 1; +0xb4 its kind), +0xc0 home, +0xd0 the arena's centre, +0xdc its radius, +0xe0 the turn velocity, +0xe8
//! the speed, +0xec the arena path, +0xf0 / +0xf4 the crawler / trooper groups, +0xf8 Ratchet's cuboid for the
//! cutaways, +0xfc / +0x100 the doors, +0x104 the slider group, +0x108 the spline wall, +0x10c the meter's slot, +0x110
//! the meter's value, +0x114 the arena path's count, +0x118 / +0x11c the cutaway's ride, +0x120 the timer, +0x128 the
//! glow phase, +0x12c the fade timer, +0x130 the big head, +0x170 a fade running, +0x174 the phase's health floor,
//! +0x178 Ratchet fired something.
//!
//! | state | what | port |
//! |---|---|---|
//! | top | the hits; the big head (1.9, list 5); within 45 of the camera the shadow probe (+0x7f 0x24); in 3..13 the boss meter (`queue_animation_update(0x16, …, &+0x110, 495)`) | [`update`] (`hud::Calls::boss_meter`) |
//! | 0 | health 30, column 3, +0x2e 1, +0x29 1, meter 30; +0x58 14, +0x5a 18; timer `ticks(1200)`; home; the arena's count, centre (the midpoint of its points 0 and n/2) and radius; glow phase; fade timer `ticks(30)`; no collision, hidden, not targetable; 17.5 up; the troopers hidden (0x12); flag 0x13d3ab clear or 0x13d505 set → 1, else deleted | [`init`] |
//! | 1 | a target and it is Ratchet: a fade (`ticks(10)`); faded → the doors shut (`petal_door::close`), the sliders up (`0x2d9e08`), 2, collision, shown, targetable, seq 8 (1 tick), the cutaway (`0x2fb148`) | |
//! | 2 | falling (30·dt²) to home; seq 8 → 9 within 6 ticks of landing; landed: seq 9 → 6, else 1; the cutaway's ride (3·dt², 2·dt) done → 7, its end (`0x2fb4b0`) | [`ride`] |
//! | 3 / 5 | seq 6 done → seq 1 and (3) the crawlers drop / (5) the troopers shown (`0x2f99b0`); the ride (0.5·dt², 0.5·dt) done → 4 / 6, the end | |
//! | 4 / 6 | (4) the drop again; the circling (`0x2fbc88`); 4: every crawler gone or held → 8; 6: the trooper group gone → 8; timer `ticks(1200)`, seq 3 | [`circle`] |
//! | 7 | a target → 8 (seq 3) | |
//! | 8 | turned to the target (2π·dt², π·dt); speed: facing within 45° and beyond 7 → 4·dt; within 4 → backing off at −4·dt; else to 0 (8·dt² / 8·dt²); a step toward it; 1 off the walls; no target → 0xd; the timer out → 0xc, timer `ticks(360)`; within 7 facing within 10° → 9, +0xbc 7, seq 4 | [`chase`] |
//! | 9 / 10 / 0xb | the rear-up's end → 10 (seq 5); keys 1–8: spheres at joints 0..4 (0.5) and 5, 6 (1.1), damage 1, flags 1; the end → +0xbc (seq 1) | |
//! | 0xc | circling; the timer out → `ticks(1200)`, 0xb (+0xbc 7, seq 6) | |
//! | 0xd | home (facing within 45°, 4·dt); a target → 7; there → 7 (seq 1) | |
//! | 0xe | flag 0x13d3ab; meter 0; update 0xff; seq done → `0x26e1f8(2, 20)` 1 up, the burst (0x6a2 ×1, 0x6a3 ×3, 6, 2), door +0x100 open, sliders down, 0xf (seq 1), hidden; no shots fired and flag 0x13d412 clear → it set, banner 0x53d6; morphed (+0x2e = 2): straight to 0xf | |
//! | 0xf | the first camera slot of class 0x12 (the boss camera) let go (`0x316330`: its record +0x50 = 1); in a scene (mode 2) → door +0xfc opened, deleted | [`update`] |
//! | `0x2fa9c0` | the spline wall in 4..13; meter = health·16; a hit (0x330000) through the resolver (column 4) outside 0xe / 3 / 5: the weapon factor; health −; crossing 20 or 10 with no fade running → held there, a fade (`ticks(30)`); a fade running → not below its floor; ≤ 0 → reaction 1, else 1 → 5; 1 / 2: the meter released, 0xe, `SetDeathBits`, the checkpoint at cuboid +0xf8, not targetable, seq 10, flash 0xfa; 3..10: flash 200; the flash; morphed → the meter released, 0xe; from state 2 on a held fire button with a hand item other than 8 marks +0x178; the fade: out over its last `ticks(10)` → at 0 the next phase (health 19.95: 3, the cutaway; else 9.9: 5, Ratchet teleported to +0xf8, the cutaway), home, facing Ratchet, seq 6; in back over `ticks(10)`; glow (120°/s, 0x80808080 → 0x80804080); the target in its arena (range 255) | [`hits`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::creature::{self as c, damage, flash, fx, target, turn, V};
use crate::moby_update::manip;
use crate::moby_update::scheduler;
use crate::moby_update::services::{pf, pv as v4, HitTemplate, World};
use crate::moby_update::story;
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 6;
pub const UPDATE_FN: u32 = 0x2f_9a28;
pub const CLASSES: [i16; 1] = [1051];
const CRAWLER: i16 = 0x33b;
const SLIDER: i16 = 0x16f;
const HEALTH: f32 = 30.0;
const METER_MAX: i32 = 0x1ef;
/// gp−0x4d0c (4) run, −0x4d08 (5) circle, −0x4d04 (8) the speed's spring, −0x4d00 (7) the reach, −0x4cfc (4) too
/// near, −0x4cf8 (10°) the slam's cone, −0x4cf4 (120°/s) the glow, −0x4cf0 / −0x4cec its colours.
const RUN: f32 = 4.0;
const CIRCLE: f32 = 5.0;
const ACCEL: f32 = 8.0;
const REACH: f32 = 7.0;
const TOO_NEAR: f32 = 4.0;
const CONE: f32 = 10.0;
const GLOW_RATE: f32 = 120.0;
const GLOW: (u32, u32) = (0x8080_8080, 0x8080_4080);
const DEG: f32 = 0.017_453_292;

pub mod pv {
    pub const D: usize = 0x20;
    pub const F: usize = 0x60;
    pub const TGT_REC: usize = 0x70;
    pub const TGT: usize = 0xb0;
    pub const KIND: usize = 0xb4;
    pub const HOME: usize = 0xc0;
    pub const CENTRE: usize = 0xd0;
    pub const RADIUS: usize = 0xdc;
    pub const TURN_V: usize = 0xe0;
    pub const SPEED: usize = 0xe8;
    pub const ARENA: usize = 0xec;
    pub const CRAWLERS: usize = 0xf0;
    pub const TROOPERS: usize = 0xf4;
    pub const CUBOID: usize = 0xf8;
    pub const DOOR_A: usize = 0xfc;
    pub const DOOR_B: usize = 0x100;
    pub const SLIDERS: usize = 0x104;
    pub const WALL: usize = 0x108;
    pub const METER_SLOT: usize = 0x10c;
    pub const METER: usize = 0x110;
    pub const COUNT: usize = 0x114;
    pub const RIDE: usize = 0x118;
    pub const RIDE_V: usize = 0x11c;
    pub const TIMER: usize = 0x120;
    pub const GLOW: usize = 0x128;
    pub const FADE: usize = 0x12c;
    pub const BIG_HEAD: usize = 0x130;
    pub const FADING: usize = 0x170;
    pub const FLOOR: usize = 0x174;
    pub const FIRED: usize = 0x178;
    pub const SIZE: usize = 0x1b0;
}

fn st(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn done(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn seq(w: &World, id: MobyId) -> u8 { w.m(id).anim.seq_b }
fn blend(w: &mut World, id: MobyId, s: u8, n: i32) {
    let t = w.ticks(n);
    c::blend_to(w, id, s, 0, t);
}
fn pi(w: &World, id: MobyId, o: usize) -> i32 { c::pi32(w, id, o) }
fn tgt(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(pi(w, id, pv::TGT) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn tgt_pos(w: &World, id: MobyId) -> V { tgt(w, id).map_or([0.0; 4], |m| w.m(m).position) }
fn heading(a: V, b: V) -> f32 { c::atan(b[0] - a[0], b[1] - a[1]) }
fn no_target(w: &World, id: MobyId) -> bool { pi(w, id, pv::KIND) == 2 }
fn meter_key(w: &World, id: MobyId) -> u32 { crate::hud::calls::pvar_key(w.m(id).spawn_id, id, pv::METER) }
fn group(w: &World, g: i32) -> Vec<MobyId> { scheduler::group_ids(w, i8::try_from(g).unwrap_or(-1)) }
fn turn_to(w: &mut World, id: MobyId, h: f32) {
    turn::turn_toward_pvar(w, id, h, c::DT2 * std::f32::consts::TAU, c::DT2 * std::f32::consts::TAU, c::DT * std::f32::consts::PI, pv::TURN_V);
}
fn speed_to(w: &mut World, id: MobyId, t: f32, step: f32) {
    let mut v = c::pf(w, id, pv::SPEED);
    turn::approach(t, step, &mut v);
    c::set_pf(w, id, pv::SPEED, v);
}
/// A step of the speed toward `p` (xy), then 1 off the arena's walls.
fn move_to(w: &mut World, id: MobyId, p: V, sp: f32) {
    let me = c::pos(w, id);
    let d = c::set_len3([p[0] - me[0], p[1] - me[1], 0.0, 0.0], sp);
    c::set_pos(w, id, c::add(me, d));
    let Some(pts) = usize::try_from(pi(w, id, pv::ARENA)).ok().and_then(|a| w.svc.splines.get(a)).cloned() else { return };
    if let Some(q) = crate::path::push_from_walls(&pts, 1.0, c::pos(w, id)) { c::set_pos(w, id, q); }
}

/// The arena path parked (its count 0) / put back.
fn park_arena(w: &mut World, id: MobyId) {
    let Ok(a) = usize::try_from(pi(w, id, pv::ARENA)) else { return };
    if let Some(s) = w.svc.splines.get_mut(a) {
        if !s.is_empty() { w.svc.units.parked_paths.insert(a, std::mem::take(s)); }
    }
}
fn unpark_arena(w: &mut World, id: MobyId) {
    let Ok(a) = usize::try_from(pi(w, id, pv::ARENA)) else { return };
    if let Some(p) = w.svc.units.parked_paths.remove(&a) {
        if let Some(s) = w.svc.splines.get_mut(a) { *s = p; }
    }
}

/// `0x2fb148` / `0x2fb508` / `0x2fb8c8`: the cutaway's start.
fn cut_start(w: &mut World, id: MobyId) {
    c::set_pf(w, id, pv::RIDE, 0.0);
    story::teleport_to(w, pi(w, id, pv::CUBOID), 0x72, false);
    let me = c::pos(w, id);
    let r0 = c::set_len3(w.m(id).rows[0], 8.0);
    let hz = super::hero_pos(w)[2];
    let cam = [me[0] + r0[0], me[1] + r0[1], hz + 2.0 + r0[2], 0.0];
    let e = [0.0, -c::atan(c::dist2(cam, me), me[2] - cam[2]) - 30.0 * DEG, heading(cam, me)];
    crate::cinematic::camera_script(w, [cam[0], cam[1], cam[2]], e, 1, 0, false);
    crate::cinematic::camera_targets(w, Some([cam[0], cam[1], cam[2]]), Some(e));
    park_arena(w, id);
}

/// `0x2fb2a8` / `0x2fb668` / `0x2fba28`: the cutaway's ride; true when over.
fn ride(w: &mut World, id: MobyId, hold_seq: u8, acc: f32, vmax: f32) -> bool {
    let cam_now = w.camera.map(|x| f32::from_bits(x.0));
    let me = c::pos(w, id);
    let t = if seq(w, id) == hold_seq {
        c::pf(w, id, pv::RIDE)
    } else {
        let (mut t, mut v) = (c::pf(w, id, pv::RIDE), c::pf(w, id, pv::RIDE_V));
        turn::spring(1.0, acc * c::DT2, acc * c::DT2, vmax * c::DT, &mut t, &mut v);
        c::set_pf(w, id, pv::RIDE, t);
        c::set_pf(w, id, pv::RIDE_V, v);
        t
    };
    let cam = if seq(w, id) == hold_seq {
        cam_now
    } else {
        let r0 = c::set_len3(w.m(id).rows[0], 8.0);
        let h = super::hero_pos(w);
        let a = [me[0] + r0[0], me[1] + r0[1], h[2] + 2.0, 0.0];
        let f = w.hero.rows[0].map(|x| f32::from_bits(x.0));
        let back = c::set_len3([f[0], f[1], f[2], 0.0], -7.0);
        let b = [h[0] + back[0], h[1] + back[1], h[2] + back[2] + 4.0, 0.0];
        std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
    };
    let e = [0.0, -c::atan(c::dist2(cam, me), me[2] - cam[2]) - (1.0 - t) * 30.0 * DEG, heading(cam, me)];
    crate::cinematic::camera_targets(w, Some([cam[0], cam[1], cam[2]]), Some(e));
    1.0 <= t && seq(w, id) == 1
}

/// `0x2fb4b0` / `0x2fb870` / `0x2fbc30`: the cutaway's end.
fn cut_end(w: &mut World, id: MobyId) {
    crate::cinematic::hero_state(w, 0, false);
    unpark_arena(w, id);
    crate::cinematic::camera_script2(w, 2);
}

/// `0x2fb090`: each hidden crawler of the group drops from 15..20 above where it is (`0x2e9d10`).
fn drop_crawlers(w: &mut World, id: MobyId) {
    for m in group(w, pi(w, id, pv::CRAWLERS)) {
        let mut p = w.m(m).position;
        p[2] += w.rng.randf(15.0, 20.0);
        if w.m(m).state != 0xf || w.m(m).pvars.len() < 0x160 { continue; }
        let col = w.m(m).has_class;
        let mm = w.mm(m);
        mm.position = p;
        mm.state = 6;
        mm.has_collision = col;
        mm.mode = (mm.mode & !0x41) | mode::TARGETABLE;
        mm.rotation = [0.0, 0.0, 0.0, mm.rotation[3]];
        let yaw = w.rng.rand_angle();
        w.mm(m).rotation[2] = yaw;
        w.build_matrix(m);
        let k = 0x120;
        c::set_pf(w, m, k + crate::moby_update::creature::knock::k::DRAG, 0.0);
        c::set_pi32(w, m, k + crate::moby_update::creature::knock::k::FLAGS, 5);
        c::set_pu8(w, m, k + 0x3d, 3);
        c::set_pf(w, m, k + crate::moby_update::creature::knock::k::UP, c::DT + c::DT);
        c::set_pf(w, m, k + crate::moby_update::creature::knock::k::GRAVITY, c::DT2 * 20.0);
        c::set_pf(w, m, k + crate::moby_update::creature::knock::k::SPEED, c::DT + c::DT);
        let a = w.rng.rand_angle();
        crate::moby_update::creature::knock::start(w, m, k, a, 2, 1, 0);
        let vz = c::pf(w, m, k + 8);
        c::set_pf(w, m, k + 8, -vz);
    }
}

/// `0x2e9ea0(group)`: every crawler of the group gone or held by the Suck Cannon.
fn crawlers_gone(w: &World, g: i32) -> bool {
    group(w, g).into_iter().all(|m| { let o = w.m(m); o.o_class != CRAWLER || o.state == 0xfe || o.state == 0xfd || o.state == 0x10 })
}

/// The sliders (0x16f) of a group to 2 with their target (`0x2d9e08` 1, `0x2d9e78` 0).
fn sliders(w: &mut World, id: MobyId, to: f32) {
    for m in group(w, pi(w, id, pv::SLIDERS)) {
        if w.m(m).o_class != SLIDER || w.m(m).pvars.len() < 0x2c { continue; }
        w.mm(m).state = 2;
        c::set_pf(w, m, 0x28, to);
    }
}

/// `0x2fbc88` (module doc).
fn circle(w: &mut World, id: MobyId) {
    let (ctr, me, h) = (c::pv4(w, id, pv::CENTRE), c::pos(w, id), super::hero_pos(w));
    let ah = c::add_rot(heading(ctr, h), f32::from_bits(0x4049_0fd0));
    let am = heading(ctr, me);
    let d = c::sub_rot(ah, am).clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
    let a = c::add_rot(d, am);
    let r = c::pf(w, id, pv::RADIUS);
    let goal = [ctr[0] + a.cos() * r, ctr[1] + a.sin() * r, ctr[2], ctr[3]];
    let t = tgt_pos(w, id);
    turn_to(w, id, heading(me, t));
    if c::dist3(me, goal) <= 1.0 || 16.0 <= c::dist2(me, h) {
        speed_to(w, id, 0.0, ACCEL * c::DT2);
    } else {
        speed_to(w, id, CIRCLE * c::DT, ACCEL * c::DT2);
    }
    if c::pf(w, id, pv::SPEED) == 0.0 { blend(w, id, 1, 20); } else { blend(w, id, 0, 20); }
    let sp = c::pf(w, id, pv::SPEED);
    move_to(w, id, goal, sp);
}

/// State 8 (module doc).
fn chase(w: &mut World, id: MobyId) {
    let (me, t) = (c::pos(w, id), tgt_pos(w, id));
    let d = c::dist2(me, t);
    let diff = c::diff_rots(c::yaw(w, id), heading(me, t));
    turn_to(w, id, heading(me, t));
    if std::f32::consts::FRAC_PI_4 <= diff || d <= REACH {
        if d < TOO_NEAR { speed_to(w, id, -(RUN * c::DT), ACCEL * c::DT2); } else { speed_to(w, id, 0.0, c::DT2 * 8.0); }
    } else {
        speed_to(w, id, RUN * c::DT, ACCEL * c::DT2);
    }
    let sp = c::pf(w, id, pv::SPEED);
    move_to(w, id, t, sp);
    if no_target(w, id) {
        set(w, id, 0xd);
        return;
    }
    if c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 {
        let n = w.ticks(0x168);
        c::set_pi32(w, id, pv::TIMER, n);
        set(w, id, 0xc);
        return;
    }
    if REACH <= d || CONE * DEG <= diff { return; }
    set(w, id, 9);
    w.mm(id).cmd = 7;
    blend(w, id, 4, 20);
}

/// State 0 (module doc).
fn init(w: &mut World, id: MobyId) {
    c::set_pf(w, id, pv::D, HEALTH);
    c::set_pu8(w, id, pv::D + 8, 3);
    c::set_pu8(w, id, pv::D + 0xe, 1);
    c::set_pu8(w, id, pv::D + 9, 1);
    c::set_pi16(w, id, pv::D + 4, 0x1e);
    c::set_pu8(w, id, 0x58, 14);
    c::set_pu8(w, id, 0x5a, f32::from_bits(0x4193_3333) as u8);
    let n = w.ticks(0x4b0);
    c::set_pi32(w, id, pv::TIMER, n);
    c::set_pi32(w, id, pv::FIRED, 0);
    let p = c::pos(w, id);
    c::set_pv4(w, id, pv::HOME, p);
    c::set_pi32(w, id, pv::METER_SLOT, -1);
    let pts: Vec<V> = usize::try_from(pi(w, id, pv::ARENA)).ok().and_then(|a| w.svc.splines.get(a)).map_or(Vec::new(), |s| s.iter().map(|q| q.map(f32::from_bits)).collect());
    c::set_pi32(w, id, pv::COUNT, pts.len() as i32);
    if let (Some(a), Some(b)) = (pts.first(), pts.get(pts.len() / 2)) {
        let ctr = std::array::from_fn(|i| (a[i] + b[i]) * 0.5);
        c::set_pv4(w, id, pv::CENTRE, ctr);
        let r = c::dist2(p, ctr);
        c::set_pf(w, id, pv::RADIUS, r);
    }
    let g = w.rng.rand_angle();
    c::set_pf(w, id, pv::GLOW, g);
    let n = w.ticks(0x1e);
    c::set_pi32(w, id, pv::FADE, n);
    let m = w.mm(id);
    m.has_collision = false;
    m.mode = (m.mode & !mode::TARGETABLE) | 0x4001;
    m.position[2] += 17.5;
    for t in group(w, pi(w, id, pv::TROOPERS)) {
        let mm = w.mm(t);
        mm.state = 0x12;
        mm.has_collision = false;
        mm.mode = (mm.mode & !mode::TARGETABLE) | 1;
    }
    let beaten = story::flag(w, story::flag_index(0x13_d3ab)) != 0;
    // 0x13d505 = the acquired items 0x13d4e8 + 29 (not a global flag).
    let replay = story::acquired(w, 29);
    if !beaten || replay { set(w, id, 1); } else { w.delete_moby(id); }
}

/// The next phase at the end of a fade (`0x2fa9c0`).
fn next_phase(w: &mut World, id: MobyId) {
    let high = 10.0 < c::pf(w, id, pv::D);
    c::set_pf(w, id, pv::D, if high { f32::from_bits(0x419f_3333) } else { f32::from_bits(0x411e_6666) });
    set(w, id, if high { 3 } else { 5 });
    c::set_pi32(w, id, pv::FADING, 0);
    if !high { story::teleport_to(w, pi(w, id, pv::CUBOID), 0x72, true); }
    let home = c::pv4(w, id, pv::HOME);
    c::set_pos(w, id, home);
    let h = super::hero_pos(w);
    let yaw = heading(home, h);
    c::set_yaw(w, id, yaw);
    w.build_matrix(id);
    blend(w, id, 6, 5);
    cut_start(w, id);
}

/// `0x2fa9c0` (module doc).
fn hits(w: &mut World, id: MobyId) {
    let s = st(w, id);
    if s == 0 { return; }
    if (4..14).contains(&s) && pi(w, id, pv::WALL) != -1 {
        let sp = pi(w, id, pv::WALL) as i16;
        w.hero_fields_mut().wall_spline = Some(sp);
    }
    let v = (c::pf(w, id, pv::D) * 16.0) as i32;
    c::set_pi32(w, id, pv::METER, v);
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    if res.out5 != 1 && s != 0xe && s != 3 && s != 5 {
        let mut dmg = res.damage;
        if let Some(a) = hit.and_then(|h| h.attacker) {
            match w.m(a).o_class {
                0xb0 | 0x79 | 0x33b => dmg *= 1.5,
                0x131 => dmg *= f32::from_bits(0x3f2a_7efa),
                _ => {}
            }
        }
        let hp = c::pf(w, id, pv::D) - dmg;
        c::set_pf(w, id, pv::D, hp);
        if pi(w, id, pv::FADING) == 0 {
            let t10 = w.ticks(10);
            let ready = t10 <= pi(w, id, pv::FADE);
            let floor = if ready && hp <= 20.0 && 20.0 < hp + dmg { Some(20.0) } else if ready && hp <= 10.0 && 10.0 < hp + dmg { Some(10.0) } else { None };
            if let Some(f) = floor {
                c::set_pf(w, id, pv::FLOOR, f);
                c::set_pi32(w, id, pv::FADING, 1);
                c::set_pf(w, id, pv::D, f);
                let n = w.ticks(0x1e);
                c::set_pi32(w, id, pv::FADE, n);
            }
        } else if hp < c::pf(w, id, pv::FLOOR) {
            let f = c::pf(w, id, pv::FLOOR);
            c::set_pf(w, id, pv::D, f);
        }
        let hp = c::pf(w, id, pv::D);
        let reaction = if hp <= 0.0 { 1 } else if res.reaction == 1 { 5 } else { res.reaction };
        match reaction {
            1 | 2 => {
                w.svc.hud.release(crate::hud::Request::boss(meter_key(w, id), METER_MAX));
                c::set_pi32(w, id, pv::METER_SLOT, -1);
                set(w, id, 0xe);
                set_death_bits(w, id, 0, -1);
                if let Some((p, e)) = story::cuboid(w, pi(w, id, pv::CUBOID)) {
                    crate::moby_update::classes::checkpoint::record(w, crate::moby_update::classes::checkpoint::Record { pos: p, rot: e });
                }
                w.mm(id).mode &= !mode::TARGETABLE;
                c::blend_to(w, id, 10, 0, 0);
                c::set_pu8(w, id, pv::F + 7, 0xfa);
            }
            3..=10 => c::set_pu8(w, id, pv::F + 7, 200),
            _ => {}
        }
        flash::start(w, id, pv::F);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pv::F);
    if c::pu8(w, id, pv::D + 0xe) & 2 != 0 && st(w, id) != 0xf {
        w.svc.hud.release(crate::hud::Request::boss(meter_key(w, id), METER_MAX));
        c::set_pi32(w, id, pv::METER_SLOT, -1);
        set(w, id, 0xe);
        return;
    }
    if 2 <= st(w, id) && w.hero.items.slot.id != 8 && w.hero.loop_in.pad.held & 0x20 != 0 { c::set_pi32(w, id, pv::FIRED, 1); }
    let n = pi(w, id, pv::FADE);
    let fading = pi(w, id, pv::FADING) != 0;
    if n != 0 && fading {
        let n = n - 1;
        c::set_pi32(w, id, pv::FADE, n);
        let t10 = w.ticks(10);
        if n < t10 {
            let k = w.svc.timing.scale(Pf::f(10.0)).to_f32();
            crate::cinematic::set_fade(w, 1.0 - n as f32 / k);
        }
        if n == 0 && st(w, id) != 1 { next_phase(w, id); }
    } else if !(n == 0 && fading) {
        let t10 = w.ticks(10);
        if n < t10 {
            c::set_pi32(w, id, pv::FADE, n + 1);
            crate::cinematic::set_fade(w, 1.0 - (n + 1) as f32 / 10.0);
        }
    }
    let g = c::add_rot(c::pf(w, id, pv::GLOW), GLOW_RATE * DEG * c::DT);
    c::set_pf(w, id, pv::GLOW, g);
    w.mm(id).glow = crate::particles::tween_color(((g.sin() + 1.0) * 0.5).to_bits(), GLOW.0, GLOW.1);
    let reg = usize::try_from(pi(w, id, pv::ARENA)).ok();
    let t = target::acquire_in(w, id, 255.0, reg);
    c::set_pv4(w, id, pv::TGT_REC, t.pos);
    c::set_pi32(w, id, pv::TGT, t.moby.map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, pv::KIND, t.kind as i32);
    if pi(w, id, pv::TGT) == 0 {
        let h = w.hero_moby.map_or(0, |m| m as i32 + 1);
        c::set_pi32(w, id, pv::TGT, h);
    }
}

/// Level06 `0x2f9a28` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { w.mm(id).pvars.resize(pv::SIZE, 0); }
    hits(w, id);
    if w.m(id).state == 0xfe { return; }
    manip::big_head(w, f32::from_bits(0x3ff3_3333), id, 5, id, pv::BIG_HEAD);
    if w.m(id).visible != 0 {
        let cam = w.camera.map(|x| f32::from_bits(x.0));
        if c::dist3(c::pos(w, id), cam) < 45.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0x24;
        }
    }
    if (3..14).contains(&st(w, id)) {
        let v = pi(w, id, pv::METER);
        w.svc.hud.boss_meter(meter_key(w, id), v, METER_MAX);
        c::set_pi32(w, id, pv::METER_SLOT, 1);
    }
    match st(w, id) {
        0 => init(w, id),
        1 => {
            if no_target(w, id) || tgt(w, id).is_none_or(|t| w.m(t).o_class != 0) { return; }
            if pi(w, id, pv::FADING) == 0 {
                c::set_pi32(w, id, pv::FADING, 1);
                let n = w.ticks(10);
                c::set_pi32(w, id, pv::FADE, n);
            } else if pi(w, id, pv::FADE) == 0 {
                for o in [pv::DOOR_A, pv::DOOR_B] {
                    if let Some(d) = story::link(w, pi(w, id, o)) { super::petal_door::close(w, d); }
                }
                sliders(w, id, 1.0);
                c::set_pi32(w, id, pv::FADING, 0);
                set(w, id, 2);
                let col = w.m(id).has_class;
                let m = w.mm(id);
                m.mode = (m.mode & !1) | mode::TARGETABLE;
                m.has_collision = col;
                blend(w, id, 8, 1);
                cut_start(w, id);
            }
        }
        2 => {
            let vz = c::pf(w, id, 0x48);
            w.mm(id).position[2] += vz;
            let vz = vz - c::DT2 * 30.0;
            c::set_pf(w, id, 0x48, vz);
            let home_z = c::pv4(w, id, pv::HOME)[2];
            if seq(w, id) == 8 {
                if ((w.m(id).position[2] - home_z) / vz).abs() <= 6.0 { blend(w, id, 9, 5); }
            } else {
                if w.m(id).position[2] < home_z { w.mm(id).position[2] = home_z; }
                if w.m(id).anim.t == 0.0 && done(w, id) {
                    if seq(w, id) == 9 { blend(w, id, 6, 30); } else { blend(w, id, 1, 30); }
                }
            }
            if ride(w, id, 8, 3.0, 2.0) {
                set(w, id, 7);
                cut_end(w, id);
            }
        }
        s @ (3 | 5) => {
            if seq(w, id) == 6 && w.m(id).anim.t == 0.0 && done(w, id) {
                blend(w, id, 1, 20);
                if s == 3 {
                    drop_crawlers(w, id);
                } else {
                    for t in group(w, pi(w, id, pv::TROOPERS)) {
                        if w.m(t).state != 0x12 { continue; }
                        let col = w.m(t).has_class;
                        let m = w.mm(t);
                        m.state = 0;
                        m.mode = (m.mode & !1) | mode::TARGETABLE;
                        m.has_collision = col;
                    }
                }
            }
            if ride(w, id, 6, 0.5, 0.5) {
                set(w, id, s + 1);
                cut_end(w, id);
            }
        }
        s @ (4 | 6) => {
            if s == 4 { drop_crawlers(w, id); }
            circle(w, id);
            let over = if s == 4 { crawlers_gone(w, pi(w, id, pv::CRAWLERS)) } else { scheduler::group_count(w, pi(w, id, pv::TROOPERS), -1) == 0 };
            if !over { return; }
            set(w, id, 8);
            let n = w.ticks(0x4b0);
            c::set_pi32(w, id, pv::TIMER, n);
            blend(w, id, 3, 20);
        }
        7 => {
            if no_target(w, id) { return; }
            set(w, id, 8);
            blend(w, id, 3, 20);
        }
        8 => chase(w, id),
        9 => {
            c::dec_timer_pvar_i32(w, id, pv::TIMER);
            if done(w, id) {
                set(w, id, 10);
                blend(w, id, 5, 20);
            }
        }
        10 => {
            c::dec_timer_pvar_i32(w, id, pv::TIMER);
            let key = c::ground::key_time(w, id);
            if 1.0 < key && key < 8.0 {
                let y = c::yaw(w, id);
                let t = HitTemplate { dir: [pf(y.cos()), pf(y.sin()), Pf::ZERO, Pf::ZERO], attacker: Some(id), flags: 1, b18: 0, b19: 0, h1a: 0, damage: Pf::ONE, w20: 1 };
                for j in 0..7 {
                    let at = w.joint_point(id, j);
                    let r = if j < 5 { 0.5 } else { f32::from_bits(0x3f8c_cccd) };
                    w.sphere_mobys(pf(r), v4(at), 0, Some(id), Some(&t));
                }
                return;
            }
            if done(w, id) {
                let b = w.m(id).cmd;
                set(w, id, b);
                blend(w, id, 1, 20);
            }
        }
        0xb => {
            if done(w, id) {
                let b = w.m(id).cmd;
                set(w, id, b);
                blend(w, id, 1, 20);
            }
        }
        0xc => {
            circle(w, id);
            if c::dec_timer_pvar_i32(w, id, pv::TIMER) == 0 { return; }
            let n = w.ticks(0x4b0);
            c::set_pi32(w, id, pv::TIMER, n);
            w.mm(id).cmd = 7;
            set(w, id, 0xb);
            blend(w, id, 6, 20);
        }
        0xd => {
            let (me, home) = (c::pos(w, id), c::pv4(w, id, pv::HOME));
            let d = c::dist2(me, home);
            let diff = c::diff_rots(c::yaw(w, id), heading(me, home));
            turn_to(w, id, heading(me, home));
            if diff < std::f32::consts::FRAC_PI_4 { move_to(w, id, home, RUN * c::DT); }
            if !no_target(w, id) {
                set(w, id, 7);
                return;
            }
            if 1.0 <= d { return; }
            set(w, id, 7);
            blend(w, id, 1, 20);
        }
        0xe => {
            story::set_flag(w, story::flag_index(0x13_d3ab), 1);
            c::set_pi32(w, id, pv::METER, 0);
            w.mm(id).update_dist = 0xff;
            if c::pu8(w, id, pv::D + 0xe) != 2 {
                if !done(w, id) { return; }
                let mut p = c::pos(w, id);
                p[2] += 1.0;
                fx::piece_explosion(w, 2.0, 20.0, Some(id), p);
                crate::moby_update::classes::breakables::burst_pieces(w, id, 0x6a2, 1, 0x6a3, 3, 6, 2);
                if let Some(d) = story::link(w, pi(w, id, pv::DOOR_B)) { super::petal_door::open(w, d); }
                sliders(w, id, 0.0);
                set(w, id, 0xf);
                blend(w, id, 1, 20);
                let m = w.mm(id);
                m.has_collision = false;
                m.visible = 0;
                m.mode |= 1;
                // Skill point 0x13d412 (the skill-point table 0x13d408, not the global flags).
                let k = story::skill_index(0x13_d412);
                if pi(w, id, pv::FIRED) == 0 && !story::skill_point(w, k) {
                    story::set_skill_point(w, k);
                    crate::cinematic::show_banner(w, 0x53d6, -1);
                }
                return;
            }
            if let Some(d) = story::link(w, pi(w, id, pv::DOOR_B)) { super::petal_door::open(w, d); }
            sliders(w, id, 0.0);
            set(w, id, 0xf);
            let m = w.mm(id);
            m.mode |= 1;
            m.visible = 0;
            m.has_collision = false;
            blend(w, id, 1, 20);
        }
        0xf => {
            // `0x316330` on the first slot of class 0x12 (the slots' active words 0x16a010: every loaded record).
            if let Some(k) = w.svc.camera_classes.iter().take(0x30).position(|&c| c == 0x12) { crate::cinematic::focus_suppress(w, k); }
            if w.svc.game_mode == 2 {
                if let Some(d) = story::link(w, pi(w, id, pv::DOOR_A)) { super::petal_door::open(w, d); }
                w.delete_moby(id);
            }
        }
        _ => {}
    }
}
