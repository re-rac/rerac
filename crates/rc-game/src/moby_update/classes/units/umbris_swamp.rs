//! **Umbris' swamp beasts, class 1059** (level07 `0x30f5f0` with its init `0x30ed30`, depth `0x30ebc8`, hits
//! `0x30e998`, knocks `0x30e858` / `0x30e720`, heading `0x30eee8`, swim `0x30f0f8`, speed `0x30f1e0`, bite `0x30f308`,
//! mouth test `0x30f4d0`, busy test `0x30f540` and the state by depth `0x30ec58`; 14 placed; census U267; the name is
//! descriptive [L]). Creatures of the swamp water that cruise its region (the path +0x110 as a polygon), keeping to
//! the middle of the water's depth (the moby +0x118's height is the surface), turning away from walls and the
//! region's edge. When Ratchet swims in their region within 30 one of them (only one of the group at a time) turns
//! and comes for him; within 3 and facing him it lunges and swallows him (he is hidden, held and dies: state 0x3d)
//! and swims off. In shallow water they wade and, with him in the region, bite (keys 17–20, one sphere at the mouth)
//! and back off; their depth class decides the moves between deep (0), shallow (1) and land (2). Deep in the water
//! hits do nothing to them; out of it a hit knocks them (light: a short knock, sequence 6; the reactions 9 / 10:
//! sequence 4) or, the health gone, throws them (sequence 12) and they blow up. Their mouths disturb the water and
//! make splash rings. The heads grow with the enemies' big-head cheat. Read from the level07 decomp and its words
//! gp−0x50c0 / −0x50b0 (the depth transitions). Native `f32`.
//!
//! **Pvars**: +0x20 the damage record, +0x60 the flash record, +0x70 the knockback record `K`, +0xd0 the velocity
//! (+0xd8 its z), +0xe0 home, +0xf0 the area cuboid's centre, +0x100 the lunge's start, +0x110 the swim region path,
//! +0x114 the area cuboid, +0x118 the surface moby, +0x120 / +0x124 the yaw / pitch springs, +0x128 the region (the
//! port: the path index again), +0x12c the surface (the moby), +0x130 the ground under it, +0x134 a timer, +0x138 the
//! height it keeps, +0x13c the heading, +0x140 the search step (±20°), +0x144 / +0x148 Ratchet's region, +0x14c the
//! swallow's progress, +0x150 the big head record.
//!
//! | address | what | port |
//! |---|---|---|
//! | top | the big head (2.2, list 0); state 0: the init, failing → deleted | [`update`], [`init`] |
//! | top | the target (`0x2886d8(30)`); the hits; the flash; the timer +0x134; in states < 7: Ratchet in his region (+0x148) and his record as the target within 30 (xy), else none; from 7 on: the target in the region; the depth class (+0xbc) and its transition (state / sequence from gp−0x50c0 / −0x50b0 by old·3 + new) | [`update`], [`depth`] |
//! | top | joint list 0 in a 0.8 sphere (flags 1) on water (surface 0): every 45th tick (by its spawn id) at random a ripple (`RippleDisturb`), every `(rand() & 7) + 20` ticks a splash ring (type 45) on the water there; the swallow done (+0x14c ≥ 1): Ratchet at its position | [`water_fx`] |
//! | 1 | idle: Ratchet in the region and the target → (him, and no member of the group in 4..6) 4; his body at the mouth (1.5) → 5; else the timer out → 2 (a random heading, `rand() % 200 + 300`); slows 5 %, swims (15, 50, 8) | [`update`] |
//! | 2 | cruise: the target Ratchet in the region → (none busy) 4; else the springs (100°, 260°, 100°) toward the heading, level; the timer out inside the area → 1; the heading search, the speed (12°, 6, 30°, 4, 7, 3) facing → 1 when the timer is out; the mouth on Ratchet → 5; swims (15, 50, 8) | [`update`], [`search`], [`speed`] |
//! | 4 | the chase: the springs at him (720°, 0, 720°), pitched; within 3 slows 10 %; the speed (12°, 20, 30°, 3, 20, 5): facing and within 3 → sequence 2 (frame 4), no collision, → 5, the start; depth at his feet − 0.25; swims (38, 0, 38); he left the region or is off the ground → 2 | [`update`] |
//! | 5 | the swallow: turning to him, pitch ·0.7, progress + 0.028: the mouth slides from the start onto him; at 1: his health 0, `SetState(0x3d, 1)`, his moby's collision off, hidden, → 6; he left → 2 | [`update`] |
//! | 6 | swims off (the search, 12° / 6 / 30° / 4 / 7 / 3, swims 12 / 50 / 7, springs 90° / 180° / 180°) | [`update`] |
//! | 7, 8 | shallow: Ratchet in the region → 9 (sequence 4); 7 waits for the timer → 8 (a random heading); 8 walks (springs 45° / 90° / 45°, the search, the speed 20° / 3 / 60° / 9 / 1 / 1), back to 7 inside the area when the timer is out; gravity 9.8·dt, the move (`0x26d8b0`: 1 up, radius 1, flags 0x10) halved when it hit | [`update`] |
//! | 9, 10 | the bite: turn at him, the speed (30°, 4, 90°, 9, 1.5, 1); facing within 1.9 → sequence 5, 10; the bite (`0x30f308`: state 10, sequence 5, keys 17–20: a 1-damage sphere of 1 at the mouth hitting him → bounced back, sequence 4 frame 4, → 0xb); out of the region → 8; wrapped → 8 | [`update`], [`bite`] |
//! | 0xb / 0xc / 0xd | the bite's recoil → 7; the knock (`0x2850b8`) landed → by depth; sequence 4 wrapped → by depth | [`update`], [`by_depth`] |
//! | 0xe | thrown: landed and wrapped → the explosion (2, 1, 4, 1, 7; 3, 3, 5, no sound), deleted | [`update`] |
//! | 0xf / 0x10 / 0x11 | on land: slowing, falling, the move; 0x10 / 0x11 wrapped → by depth | [`update`] |
//! | end | out of the world → deleted | [`update`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::bomb_water;
use crate::moby_update::creature::{self as c, damage, flash, fx, knock, region, target, turn, walker, DT, DT2, SPEED, V};
use crate::moby_update::services::{pf, pv as v4, HeroCall, HeroPose, HitTemplate, World};
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 7;
pub const UPDATE_FN: u32 = 0x30_f5f0;
pub const CLASSES: [i16; 1] = [1059];
const PVARS: usize = 0x190;
const D: usize = 0x20;
const K: usize = 0x70;
const PI: f32 = std::f32::consts::PI;
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
const QUARTER_PI: f32 = std::f32::consts::FRAC_PI_4;
const DEG12: f32 = f32::from_bits(0x3e56_7750);
const DEG20: f32 = f32::from_bits(0x3eb2_b8c2);
const DEG30: f32 = f32::from_bits(0x3f06_0a92);
const DEG60: f32 = f32::from_bits(0x3f86_0a92);
const DEG100: f32 = 1.745_329_3;
const DEG260: f32 = 4.537_856;
/// gp−0x50c0 / gp−0x50b0: the state and the sequence a depth change (old·3 + new) sets (0xff: none).
const DEPTH_STATE: [u8; 9] = [0xff, 0x07, 0x10, 0x01, 0xff, 0x10, 0x11, 0x11, 0xff];
const DEPTH_SEQ: [u8; 9] = [0xff, 0x03, 0x08, 0x00, 0xff, 0x08, 0x0a, 0x0a, 0xff];
const DEATH_BEAM: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 4.0, scale: 1.0, light: 7.0, streaks: 3, sparks: 3, puffs: 5, debris: 1, sound: -1, shake: false };

mod pv {
    pub const VEL: usize = 0xd0;
    pub const VZ: usize = 0xd8;
    pub const HOME: usize = 0xe0;
    pub const CENTRE: usize = 0xf0;
    pub const START: usize = 0x100;
    pub const REGION: usize = 0x110;
    pub const AREA: usize = 0x114;
    pub const SURFACE: usize = 0x118;
    pub const YAW_V: usize = 0x120;
    pub const PITCH_V: usize = 0x124;
    pub const GROUND: usize = 0x130;
    pub const TIMER: usize = 0x134;
    pub const KEEP_Z: usize = 0x138;
    pub const HEADING: usize = 0x13c;
    pub const SEARCH: usize = 0x140;
    pub const HERO_REGION: usize = 0x144;
    pub const SWALLOW: usize = 0x14c;
    pub const HEAD: usize = 0x150;
}

fn state(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_state(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn blend(w: &mut World, id: MobyId, s: u8, frame: i32, t: i32) { c::blend_to(w, id, s, frame, t); }
fn wrapped(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn path(w: &World, id: MobyId, o: usize) -> Option<usize> { usize::try_from(c::pi32(w, id, o)).ok().filter(|&p| p < w.svc.splines.len()) }
fn in_region(w: &World, id: MobyId, o: usize, p: V) -> bool { path(w, id, o).is_some_and(|r| region::point_in_polygon(w, r, p)) }
fn surface_z(w: &World, id: MobyId) -> f32 {
    usize::try_from(c::pi32(w, id, pv::SURFACE)).ok().and_then(|m| w.table.mobys.get(m)).map_or(0.0, |m| m.position[2])
}
fn rand_timer(w: &mut World, m: i32, base: i32) -> i32 { w.rng.rand() % m + base }
fn scale_vel(w: &mut World, id: MobyId, k: f32) {
    let v = c::pv4(w, id, pv::VEL);
    c::set_pv4(w, id, pv::VEL, c::scale(v, k));
}

/// `0x30eb38(path)`: each point's w = the length of its segment (to the next, the last to the first).
fn segment_lengths(w: &mut World, p: usize) {
    let pts = &mut w.svc.splines[p];
    let n = pts.len();
    for i in 0..n {
        let (a, b) = (pts[i].map(f32::from_bits), pts[(i + 1) % n].map(f32::from_bits));
        pts[i][3] = c::dist3(a, b).to_bits();
    }
}

/// `0x30ed30` (module doc): false when a link is missing.
fn init(w: &mut World, id: MobyId) -> bool {
    let (Some(r), Some(h)) = (path(w, id, pv::REGION), path(w, id, pv::HERO_REGION)) else { return false };
    segment_lengths(w, r);
    segment_lengths(w, h);
    if c::pi32(w, id, pv::SURFACE) == -1 { return false; }
    let d = depth(w, id);
    w.mm(id).cmd = d;
    let area = c::pi32(w, id, pv::AREA);
    if area == -1 { return false; }
    let centre = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, area).map_or([0.0; 3], |s| s.centre());
    c::set_pv4(w, id, pv::CENTRE, [centre[0], centre[1], centre[2], 1.0]);
    let p = c::pos(w, id);
    c::set_pv4(w, id, pv::HOME, p);
    by_depth(w, id);
    let t = w.rng.rand() % 0x78;
    c::set_pi32(w, id, pv::TIMER, t);
    let s = if w.rng.rand() & 1 == 0 { -DEG20 } else { DEG20 };
    c::set_pf(w, id, pv::SEARCH, s);
    c::set_pf(w, id, pv::SWALLOW, 0.0);
    c::set_pu8(w, id, D + 9, 1);
    true
}

/// `0x30ebc8`: the ground under it (`0x282210`: `GroundHeight(0.5, position, 0x20)`) to +0x130, the height to keep
/// (half way up the water) to +0x138; the class: deep (0) from 0.68 of water, shallow (1) from 0.41, else land (2).
fn depth(w: &mut World, id: MobyId) -> u8 {
    let g = c::ground::ground(w, c::pos(w, id), 0.5, 0x20).z;
    c::set_pf(w, id, pv::GROUND, g);
    let d = surface_z(w, id) - g;
    c::set_pf(w, id, pv::KEEP_Z, g + d * 0.5);
    if d < 0.41 {
        2
    } else if d <= 0.68 {
        1
    } else {
        0
    }
}

/// `0x30ec58`: the state by the depth class (1 sequence 0; 7 sequence 3; 0xf sequence 9).
fn by_depth(w: &mut World, id: MobyId) {
    match w.m(id).cmd {
        0 => {
            blend(w, id, 0, 0, 10);
            set_state(w, id, 1);
        }
        1 => {
            blend(w, id, 3, 0, 10);
            set_state(w, id, 7);
        }
        2 => {
            blend(w, id, 9, 0, 10);
            set_state(w, id, 0xf);
        }
        _ => {}
    }
}

/// `0x30e858` / `0x30e720(m, K, dir)`: the knock's record and its start (sequence 6 / 12).
fn knock_start(w: &mut World, id: MobyId, dir: V, throw: bool) {
    let k = K;
    c::set_pf(w, id, k + knock::k::GRAVITY, DT2 * 29.400_002);
    c::set_pf(w, id, k + knock::k::DRAG, f32::from_bits(0x3a03_126f));
    c::set_pf(w, id, k + knock::k::SPEED, DT * 9.0);
    c::set_pf(w, id, k + knock::k::UP, DT * 7.0);
    c::set_pi32(w, id, k + knock::k::RADIUS, if throw { 0x400 } else { 0x599 });
    c::set_pf(w, id, k + knock::k::ZOFF, if throw { 1.0 } else { f32::from_bits(0x3fb3_3333) });
    c::set_pf(w, id, k + knock::k::BOUNCE, f32::from_bits(0x3e75_c28f));
    c::set_pf(w, id, k + 0x34, 0.5);
    c::set_pf(w, id, k + 0x38, f32::from_bits(0x3f26_6666));
    c::set_pf(w, id, k + knock::k::YAW_MAX, f32::from_bits(0x3e4c_cccd));
    c::set_pf(w, id, k + knock::k::AIR_SPEED, f32::from_bits(0x3c23_d70a));
    c::set_pf(w, id, k + knock::k::KEY_APEX, if throw { 3.0 } else { 8.0 });
    c::set_pf(w, id, k + knock::k::KEY_LAND, if throw { 9.0 } else { 19.0 });
    c::set_pi32(w, id, k + knock::k::FLAGS, 0xd);
    c::set_pu8(w, id, k + 0x3d, 0);
    let (mut sp, mut up) = (c::pf(w, id, k + knock::k::SPEED), c::pf(w, id, k + knock::k::UP));
    let a = knock::aim(dir, &mut sp, &mut up);
    c::set_pf(w, id, k + knock::k::SPEED, sp);
    c::set_pf(w, id, k + knock::k::UP, up);
    knock::start(w, id, k, a, if throw { 0xc } else { 6 }, 1, 0);
}

/// `0x30e998` (module doc).
fn hits(w: &mut World, id: MobyId) {
    if state(w, id) == 0xe { return; }
    let h = w.get_hit(id, 0x33_0000, false);
    let r = damage::resolve(w, id, h, D, 0, 4);
    if w.m(id).cmd != 0 {
        let dir = h.map_or([0.0; 4], |h| h.dir.map(|x| f32::from_bits(x.0)));
        match r.reaction {
            1 | 2 => c::set_pf(w, id, D, 0.0),
            3..=8 => {
                knock_start(w, id, dir, false);
                set_state(w, id, 0xc);
            }
            9 | 10 => {
                blend(w, id, 4, 4, 4);
                set_state(w, id, 0xd);
            }
            _ => {}
        }
        if 1 < r.out5 {
            let dmg = r.damage;
            if c::pf(w, id, D) <= dmg {
                c::set_pf(w, id, D, 0.0);
                w.mm(id).mode &= !mode::TARGETABLE;
                knock_start(w, id, dir, true);
                crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
                c::set_pu8(w, id, 0x67, 0x78);
                flash::start(w, id, 0x60);
                set_state(w, id, 0xe);
            } else {
                c::set_pf(w, id, D, c::pf(w, id, D) - dmg);
                c::set_pu8(w, id, 0x67, 0xfa);
                let t = w.ticks(60);
                c::set_pi16(w, id, D + 6, t as i16);
                flash::start(w, id, 0x60);
            }
        }
    }
    w.mm(id).hit_slot = 0xff;
}

/// `0x30eee8`: the heading: with the timer running, keep it when 5.2 ahead is inside the region and clear (a line,
/// flags 0x24, and a 1.5 sphere); else turn by +0x140 until it is, a whole turn at most; else (or the timer out)
/// toward the area's centre.
fn search(w: &mut World, id: MobyId) {
    let pos = c::pos(w, id);
    let clear = |w: &World, a: f32| -> bool {
        let q = [pos[0] + a.cos() * 5.2, pos[1] + a.sin() * 5.2, pos[2], pos[3]];
        in_region(w, id, pv::REGION, q) && w.coll_line(v4(pos), v4(q), 0x24, Some(id)).is_none() && w.coll_sphere(v4(q), pf(1.5), 0x24, Some(id)).is_none()
    };
    if c::pi32(w, id, pv::TIMER) != 0 {
        let yaw = c::yaw(w, id);
        if clear(w, yaw) { return; }
        let step = c::pf(w, id, pv::SEARCH);
        let mut a = yaw;
        let mut turned = 0.0;
        loop {
            a = c::add_rot(a, step);
            if clear(w, a) {
                c::set_pf(w, id, pv::HEADING, a);
                return;
            }
            turned += step.abs();
            if std::f32::consts::TAU <= turned { break; }
        }
    }
    let ctr = c::pv4(w, id, pv::CENTRE);
    c::set_pf(w, id, pv::HEADING, c::atan(ctr[0] - pos[0], ctr[1] - pos[1]));
}

/// `0x30f0f8(acc, dec, max)`: falling 9.8·dt above the water's − 0.3, the vertical speed sprung toward +0x138, then
/// the move (`0x26d8b0(1, 0, m, +0xd0, 0x400, 0x10)`), halved when it hit something.
fn swim(w: &mut World, id: MobyId, acc: f32, dec: f32, max: f32) {
    if surface_z(w, id) - 0.3 < w.m(id).position[2] {
        c::set_pf(w, id, pv::VZ, c::pf(w, id, pv::VZ) - DT * 9.8);
    }
    let mut z = w.m(id).position[2];
    let mut vz = c::pf(w, id, pv::VZ);
    turn::spring(c::pf(w, id, pv::KEEP_Z), acc, dec, max, &mut z, &mut vz);
    c::set_pf(w, id, pv::VZ, vz);
    step(w, id, true);
}

/// The move with collision (`0x26d8b0(1, 0, m, +0xd0, 0x400, 0x10)`); `halve`: the velocity halved when it hit.
fn step(w: &mut World, id: MobyId, halve: bool) {
    let mut v = c::pv4(w, id, pv::VEL);
    let r = walker::move_collide(w, id, 1.0, 1.0, 0.0, &mut v, 0x10);
    // The move writes the displacement it made back into the velocity.
    c::set_pv4(w, id, pv::VEL, if r != 0 && halve { c::scale(v, 0.5) } else { v });
}

/// `0x30f1e0(heading, fast, acc, slow, dec, vmax, vmin)`: the flat speed + acc when facing within `fast` (true), −
/// dec beyond `slow`, kept in [vmin, vmax], along the yaw.
#[allow(clippy::too_many_arguments)]
fn speed(w: &mut World, id: MobyId, heading: f32, fast: f32, acc: f32, slow: f32, dec: f32, vmax: f32, vmin: f32) -> bool {
    let v = c::pv4(w, id, pv::VEL);
    let mut s = c::len2(v);
    let d = c::diff_rots(c::yaw(w, id), heading);
    let facing = d < fast;
    if facing {
        s += acc;
    } else if slow < d {
        s -= dec;
    }
    let s = if s <= vmax { if s < vmin { vmin } else { s } } else { vmax };
    let (cs, sn) = c::cs(c::yaw(w, id));
    c::set_pv4(w, id, pv::VEL, [cs * s, sn * s, v[2], v[3]]);
    facing
}

/// `0x30f540(m, lo, hi)`: another member of its group in a state in [lo, hi].
fn group_busy(w: &World, id: MobyId, lo: u8, hi: u8) -> bool {
    crate::moby_update::scheduler::group_walk(w, w.m(id).group as i32, crate::moby_update::scheduler::GroupWalk::Alive)
        .into_iter()
        .any(|m| m != id && (lo..=hi).contains(&w.m(m).state))
}

/// `0x30f4d0(r, m)`: a sphere of `r` at joint list 0 (flags 0x21) touching Ratchet's moby.
fn mouth_on(w: &World, id: MobyId, r: f32) -> bool {
    let jp = w.joint_point(id, 0);
    w.coll_sphere(v4(jp), pf(r), 0x21, Some(id)).is_some_and(|o| o.moby.is_some() && o.moby == w.hero_moby)
}

/// `0x30f308(17, 20, 1, 1, 1, m, P, 10, 5, 4, 4, 0xb, target)` (module doc).
fn bite(w: &mut World, id: MobyId, target: Option<MobyId>) {
    let k = c::ground::key_time(w, id);
    if state(w, id) != 10 || w.m(id).anim.seq_a != 5 || !(17.0 < k && k < 20.0) { return; }
    let v = c::pv4(w, id, pv::VEL);
    let d = c::set_len3(v, 1.0);
    let dir = [d[0], d[1], 1.0, d[3]];
    let tmpl = HitTemplate { dir: v4(dir), attacker: Some(id), flags: 1, damage: Pf::ONE, w20: 1, ..Default::default() };
    let jp = w.joint_point(id, 0);
    let hit = w.sphere_mobys_list(pf(1.0), v4(jp), 0x21, Some(id), Some(&tmpl));
    if hit.last().is_some() && hit.last().copied() == target {
        c::set_pv4(w, id, pv::VEL, c::scale(v, -1.0));
        c::set_pf(w, id, pv::VZ, -(SPEED * 0.05));
        blend(w, id, 4, 4, 5);
        set_state(w, id, 0xb);
    }
}

/// The mouth's water effects and the carried Ratchet (module doc).
fn water_fx(w: &mut World, id: MobyId) {
    let jp = w.joint_point(id, 0);
    let Some(o) = w.coll_sphere(v4(jp), pf(0.8), 1, Some(id)) else { return };
    if o.surface_id() != 0 { return; }
    let pos = c::pos(w, id);
    let ground = c::pf(w, id, pv::GROUND);
    let surf = surface_z(w, id);
    let dz = (pos[2] - jp[2]).abs();
    let mut amp = if (surf - ground) * 0.5 < dz { (surf - ground) * 0.5 } else { dz };
    if c::pf(w, id, pv::VZ) < 0.0 { amp = -amp; }
    if matches!(state(w, id), 0xe | 0xc) { amp += amp; }
    let hit = [o.point[0], o.point[1], o.point[2], 0.0];
    let d2 = c::dist2(pos, hit);
    let r = if 0.5 < d2 { d2 + d2 } else { 1.0 };
    if (w.counter % 45) as i16 == w.m(id).spawn_id && w.rng.randi(2) == 0 { bomb_water::ripple(w, pos[0], pos[1], r, amp, true); }
    let n = (w.rng.rand() & 7) + 20;
    if w.counter.is_multiple_of(n as u64) {
        let size = w.rng.randf(1.5, f32::from_bits(0x4019_999a));
        let x = jp[0] + w.rng.randf(-f32::from_bits(0x3e19_999a), f32::from_bits(0x3e19_999a));
        let y = jp[1] + w.rng.randf(-f32::from_bits(0x3e19_999a), f32::from_bits(0x3e19_999a));
        let z = bomb_water::water_level(w, hit);
        *w.svc.fx.part_spawns.entry(45).or_default() += 1;
        if let Some(sys) = w.particles.as_deref_mut() {
            if crate::particles::type45::spawn45(sys, w.rng, size, 5250.0, [x, y, z, jp[3]], u32::MAX).is_none() { w.svc.fx.part_failed += 1; }
        }
    }
}

/// Level07 `0x30f5f0` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < PVARS { w.mm(id).pvars.resize(PVARS, 0); }
    crate::moby_update::manip::big_head(w, f32::from_bits(0x400c_cccd), id, 0, id, pv::HEAD);
    if state(w, id) == 0 {
        if !init(w, id) {
            w.delete_moby(id);
            return;
        }
        world_box(w, id);
        return;
    }
    let mut t = target::acquire(w, id, 30.0);
    hits(w, id);
    flash::update(w, id, 0x60);
    let timer_out = c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0;
    let hero = super::hero_pos(w);
    let in_hero_region;
    if c::pi32(w, id, pv::REGION) == 0 || state(w, id) < 7 {
        in_hero_region = in_region(w, id, pv::HERO_REGION, hero);
        if c::dist2(c::pos(w, id), hero) < 30.0 {
            t = target::Target { pos: hero, rot: w.hero.rot.map(|x| x.to_f32()), aim: w.hero.shadow_point.map(|x| x.to_f32()), body: w.hero.body_point.map(|x| x.to_f32()), moby: w.hero_moby, kind: 0 };
        } else {
            t = target::Target { kind: 2, ..Default::default() };
        }
    } else {
        in_hero_region = in_region(w, id, pv::HERO_REGION, t.pos);
    }
    let old = w.m(id).cmd;
    let new = depth(w, id);
    w.mm(id).cmd = new;
    let (mut to_state, mut to_seq) = (0xffu8, 0xffu8);
    if old != new {
        let i = (new + old * 3) as usize;
        to_state = DEPTH_STATE[i];
        to_seq = DEPTH_SEQ[i];
    }
    water_fx(w, id);
    if 1.0 <= c::pf(w, id, pv::SWALLOW) {
        let p = c::pos(w, id);
        let rot = w.hero.rot.map(|x| x.to_f32());
        w.hero_fields_mut().pose = Some(HeroPose { pos: [p[0], p[1], p[2]], yaw: rot[2], target_yaw: w.hero.target_yaw.to_f32() });
    }
    let is_hero = t.moby.is_some() && t.moby == w.hero_moby;
    let apply = match state(w, id) {
        1 => {
            if wrapped(w, id) { blend(w, id, 0, 0, 10); }
            let idle_out = |w: &mut World| {
                if timer_out {
                    blend(w, id, 1, 0, 10);
                    let r = rand_timer(w, 200, 300);
                    c::set_pi32(w, id, pv::TIMER, r);
                    let a = w.rng.rand_angle();
                    c::set_pf(w, id, pv::HEADING, a);
                    set_state(w, id, 2);
                }
            };
            if in_hero_region && t.moby.is_none() {
                idle_out(w);
            } else {
                if in_hero_region && is_hero && !group_busy(w, id, 4, 6) {
                    blend(w, id, 1, 0, 10);
                    set_state(w, id, 4);
                }
                if !is_hero || !mouth_on(w, id, 1.5) {
                    idle_out(w);
                } else {
                    blend(w, id, 2, 0, 2);
                    set_state(w, id, 5);
                }
            }
            scale_vel(w, id, SPEED * -0.050_000_012 + 1.0);
            swim(w, id, DT2 * 15.0, DT2 * 50.0, DT * 8.0);
            true
        }
        2 => {
            if wrapped(w, id) { blend(w, id, 1, 0, 10); }
            if in_hero_region && is_hero {
                if !group_busy(w, id, 4, 6) {
                    blend(w, id, 1, 0, 10);
                    set_state(w, id, 4);
                }
            } else {
                let h = c::pf(w, id, pv::HEADING);
                turn::spring_turn2_pvar(w, id, h, DT2 * DEG100, DT2 * DEG260, DT * DEG100, pv::YAW_V);
                pitch_to(w, id, 0.0, DT2 * DEG100, DT2 * DEG260, DT * DEG100);
                if timer_out && in_area(w, id) {
                    blend(w, id, 0, 0, 10);
                    let r = rand_timer(w, 0x78, 0x3c);
                    c::set_pi32(w, id, pv::TIMER, r);
                    set_state(w, id, 1);
                }
            }
            search(w, id);
            let h = c::pf(w, id, pv::HEADING);
            if speed(w, id, h, DEG12, DT2 * 6.0, DEG30, DT2 * 4.0, DT * 7.0, DT * 3.0) && timer_out {
                blend(w, id, 0, 0, 10);
                let r = rand_timer(w, 0x78, 0x3c);
                c::set_pi32(w, id, pv::TIMER, r);
                set_state(w, id, 1);
            }
            if is_hero && mouth_on(w, id, 1.5) {
                blend(w, id, 2, 0, 2);
                set_state(w, id, 5);
            }
            swim(w, id, DT2 * 15.0, DT2 * 50.0, DT * 8.0);
            true
        }
        4 => {
            let p = c::pos(w, id);
            let h = c::atan(t.pos[0] - p[0], t.pos[1] - p[1]);
            turn::spring_turn2_pvar(w, id, h, DT2 * 12.566_371, 0.0, DT * 12.566_371, pv::YAW_V);
            let up = c::atan(c::dist2(p, t.pos), t.pos[2] - p[2]);
            pitch_to(w, id, -up, DT2 * 12.566_371, 0.0, DT * 12.566_371);
            let near = c::dist2(p, hero) < 3.0;
            if near { scale_vel(w, id, SPEED * -0.100_000_024 + 1.0); }
            if speed(w, id, h, DEG12, DT2 * 20.0, DEG30, DT2 * 3.0, DT * 20.0, DT * 5.0) && near {
                blend(w, id, 2, 4, 2);
                w.mm(id).has_collision = false;
                set_state(w, id, 5);
                c::set_pf(w, id, pv::SWALLOW, 0.0);
                let p = c::pos(w, id);
                c::set_pv4(w, id, pv::START, p);
            }
            c::set_pf(w, id, pv::KEEP_Z, t.pos[2] - 0.25);
            swim(w, id, DT2 * 38.0, 0.0, DT * 38.0);
            if !in_hero_region || w.hero.f65c == 0 {
                blend(w, id, 1, 0, 10);
                set_state(w, id, 2);
            }
            true
        }
        5 => {
            swallow(w, id, t.pos, in_hero_region);
            false
        }
        6 => {
            search(w, id);
            let h = c::pf(w, id, pv::HEADING);
            speed(w, id, h, DEG12, DT2 * 6.0, DEG30, DT2 * 4.0, DT * 7.0, DT * 3.0);
            swim(w, id, DT2 * 12.0, DT2 * 50.0, DT * 7.0);
            turn::spring_turn2_pvar(w, id, h, DT2 * HALF_PI, DT2 * PI, DT * PI, pv::YAW_V);
            pitch_to(w, id, 0.0, DT2 * HALF_PI, DT2 * PI, DT * PI);
            if wrapped(w, id) { blend(w, id, 1, 0, 10); }
            true
        }
        7 => {
            if in_hero_region {
                blend(w, id, 4, 0, 10);
                set_state(w, id, 9);
            } else if timer_out {
                blend(w, id, 4, 0, 10);
                let r = rand_timer(w, 200, 300);
                c::set_pi32(w, id, pv::TIMER, r);
                let a = w.rng.rand_angle();
                c::set_pf(w, id, pv::HEADING, a);
                set_state(w, id, 8);
            }
            scale_vel(w, id, SPEED * -0.050_000_012 + 1.0);
            c::set_pf(w, id, pv::VZ, c::pf(w, id, pv::VZ) - DT * 9.8);
            step(w, id, true);
            true
        }
        8 => {
            if in_hero_region {
                blend(w, id, 4, 0, 10);
                set_state(w, id, 9);
            } else {
                let h = c::pf(w, id, pv::HEADING);
                turn::spring_turn2_pvar(w, id, h, DT2 * QUARTER_PI, DT2 * HALF_PI, DT * QUARTER_PI, pv::YAW_V);
                pitch_to(w, id, 0.0, DT2 * QUARTER_PI, DT2 * HALF_PI, DT * QUARTER_PI);
                if timer_out && in_area(w, id) {
                    blend(w, id, 3, 0, 10);
                    let r = rand_timer(w, 0x78, 0x3c);
                    c::set_pi32(w, id, pv::TIMER, r);
                    set_state(w, id, 7);
                }
            }
            search(w, id);
            let h = c::pf(w, id, pv::HEADING);
            if speed(w, id, h, DEG20, DT2 * 3.0, DEG60, DT2 * 9.0, DT, DT) && timer_out {
                blend(w, id, 3, 0, 10);
                let r = rand_timer(w, 0x78, 0x3c);
                c::set_pi32(w, id, pv::TIMER, r);
                set_state(w, id, 7);
            }
            c::set_pf(w, id, pv::VZ, c::pf(w, id, pv::VZ) - DT * 9.8);
            step(w, id, true);
            true
        }
        9 | 10 => {
            let p = c::pos(w, id);
            let h = c::atan(t.pos[0] - p[0], t.pos[1] - p[1]);
            turn::spring_turn2_pvar(w, id, h, DT2 * QUARTER_PI, DT2 * HALF_PI, DT * QUARTER_PI, pv::YAW_V);
            if speed(w, id, h, DEG30, DT2 * 4.0, HALF_PI, DT2 * 9.0, DT * 1.5, DT) && c::dist2(c::pos(w, id), t.pos) < 1.9 && state(w, id) != 10 {
                blend(w, id, 5, 0, 2);
                set_state(w, id, 10);
            }
            c::set_pf(w, id, pv::VZ, c::pf(w, id, pv::VZ) - DT * 9.8);
            step(w, id, true);
            c::set_pu8(w, id, 0x58, 8);
            c::set_pu8(w, id, 0x5a, 9);
            if !in_hero_region {
                blend(w, id, 4, 0, 10);
                set_state(w, id, 8);
            }
            bite(w, id, t.moby);
            if wrapped(w, id) {
                blend(w, id, 4, 0, 10);
                let r = rand_timer(w, 200, 300);
                c::set_pi32(w, id, pv::TIMER, r);
                set_state(w, id, 8);
            }
            true
        }
        0xb => {
            if wrapped(w, id) { set_state(w, id, 7); }
            false
        }
        0xc => {
            if knock::update(w, id, K) & 1 != 0 { by_depth(w, id); }
            false
        }
        0xd => {
            if wrapped(w, id) { by_depth(w, id); }
            false
        }
        0xe => {
            let r = knock::update(w, id, K);
            if r & w.m(id).anim.flags as u32 & 2 != 0 {
                let p = c::pos(w, id);
                fx::beam_explosion(w, &DEATH_BEAM, Some(id), p);
                w.delete_moby(id);
                return;
            }
            false
        }
        0xf => {
            scale_vel(w, id, SPEED * -0.050_000_012 + 1.0);
            c::set_pf(w, id, pv::VZ, c::pf(w, id, pv::VZ) - DT * 9.8);
            step(w, id, false);
            true
        }
        0x10 => {
            scale_vel(w, id, SPEED * -0.050_000_012 + 1.0);
            c::set_pf(w, id, pv::VZ, c::pf(w, id, pv::VZ) - DT * 9.8);
            step(w, id, false);
            if wrapped(w, id) { by_depth(w, id); }
            false
        }
        0x11 => {
            scale_vel(w, id, SPEED * -0.050_000_012 + 1.0);
            if wrapped(w, id) { by_depth(w, id); }
            false
        }
        _ => false,
    };
    if apply && to_state != 0xff {
        blend(w, id, to_seq, 0, 6);
        set_state(w, id, to_state);
    }
    world_box(w, id);
}

fn world_box(w: &mut World, id: MobyId) {
    if !crate::moby_update::creature::projectile::in_world(c::pos(w, id)) { w.delete_moby(id); }
}

/// `PointInCuboid(position, +0x114)`.
fn in_area(w: &World, id: MobyId) -> bool {
    let p = c::pos(w, id);
    w.in_cuboid([p[0], p[1], p[2]], c::pi32(w, id, pv::AREA))
}

/// The pitch (+0x44) spring (`0x280b38` = L01 `SpringTurn`) toward `target`.
fn pitch_to(w: &mut World, id: MobyId, target: f32, acc: f32, damp: f32, max: f32) {
    let mut v = c::pf(w, id, pv::PITCH_V);
    let p = turn::spring_turn(w.m(id).rotation[0], target, acc, damp, max, &mut v);
    c::set_pf(w, id, pv::PITCH_V, v);
    w.mm(id).rotation[0] = p;
}

/// State 5, the swallow (module doc).
fn swallow(w: &mut World, id: MobyId, tp: V, in_hero_region: bool) {
    let pos = c::pos(w, id);
    let jp = w.joint_point(id, 0);
    let mouth = c::sub(jp, pos);
    let h = c::atan(tp[0] - jp[0], tp[1] - jp[1]);
    turn::spring_turn2_pvar(w, id, h, DT2 * 12.566_371, 0.0, DT * 12.566_371, pv::YAW_V);
    w.mm(id).rotation[0] *= SPEED * 0.7;
    let s = c::pf(w, id, pv::SWALLOW) + 0.028;
    c::set_pf(w, id, pv::SWALLOW, s);
    let start = c::pv4(w, id, pv::START);
    let body = c::sub(tp, mouth);
    let e = c::scale(c::sub(body, start), s.min(1.0));
    let p = c::add(start, e);
    c::set_pos(w, id, p);
    c::set_pv4(w, id, pv::VEL, c::sub(p, pos));
    if 1.0 <= s {
        let f = w.hero_fields_mut();
        f.health = 0;
        f.call(HeroCall::SetState { id: 0x3d, play: true });
        f.hero_hidden = Some(1);
        if let Some(h) = w.hero_moby { w.mm(h).coll_disable = u32::MAX; }
        set_state(w, id, 6);
    } else {
        c::set_pf(w, id, pv::HEADING, h);
    }
    if in_hero_region || w.hero.f65c != 0 { return; }
    c::set_pf(w, id, pv::SWALLOW, 0.0);
    blend(w, id, 1, 0, 10);
    set_state(w, id, 2);
}
