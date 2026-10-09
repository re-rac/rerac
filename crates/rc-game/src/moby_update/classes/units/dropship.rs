//! **The dropships, class 1356** (level18 `0x2f0920` with `0x2f11a8`, `0x2f14d0`, `0x2f15e8`, `0x2f1698`; census U511;
//! ten placed on 18, five on 16: level16 `0x2e22d8` is the same code) **and their homing shots, class 50**
//! (`0x2a7b90`, its spawner `0x2a7eb8` and trail `0x2a8010`). A dropship waits hidden until Ratchet enters its gate
//! region (pvar +0xd4), then flies its approach path (+0xd8) carrying up to four hover troopers 638
//! (`units::drop_trooper`), lowering its ramp (the look record at +0xe0 on joint list 2) over the last 16 units; at the
//! end it drops them one every half second, turns to its exit path (+0xdc), raising the ramp, and flies it. With a
//! target region (+0xd0) it then hovers, bobbing, turning to face its target and lobbing a homing shot every three
//! seconds; without one it leaves (deleted at the end of the exit path). Twelve damage (or a 2-damage hit by Giant
//! Clank) brings it down: its riders vanish, it sinks and rolls for 1.5 seconds, then explodes into three pieces.
//! Read from the level18 decomp and data (gp−0x4990.. 1.0, 25, 30, 15, 90, 0.6; gp−0x5820 / −0x581c the shot's 30°
//! / 20°, gp−0x5818.. its trail). Native `f32`.
//!
//! **Pvars** (0x198): +0x20 the damage record (health 12.0, +0x24 12, +0x28 3), +0x60 the flash, +0x70 the target record
//! (+0xb0 the moby, +0xb4 the kind), +0xc0..+0xcc the riders (moby indices, −1 none), +0xd0 the target region path,
//! +0xd4 the gate path, +0xd8 / +0xdc the approach / exit paths (−1 none), +0xe0 the ramp's look record, +0x160 the
//! fall timer, +0x164 the shot timer, +0x168 / +0x16c the path position (point, fraction), +0x170 the speed, +0x174
//! the first segment's length, +0x178 the yaw velocity, +0x17c / +0x180 the ramp and its velocity, +0x184 the drop
//! timer, +0x188 the roll velocity, +0x18c the bob phase, +0x190 the hum's voice, +0x194 the base height.
//!
//! ## Coverage
//! | address | what | port |
//! |---|---|---|
//! | `0x2f11a8` | scale = class scale ·1.0; states 2..5: the hum (class sound 0, looped), else released; the hit (`0x26f378(+0x20, col 4)`), not in 8: health −= damage, ≤ 0 or Giant Clank with ≥ 2 → reaction 1; reactions 1 / 2: sounds 3 and (not Giant Clank) 2, the riders deleted, → 8, fall `ticks(90)`, flash 0x78; 3..8: sound 3, flash 0x78, roll velocity `randf_sym(30°, 50°)·dt`; `0x272318`; +0xa4 = 0xff, `0x2723f8` | [`prologue`] |
//! | | no target region or Ratchet in group 0x16: kind 2; else `0x274df8(128, region +0xd0)` into +0x70; no moby → Ratchet at 0x13f3d0 | [`prologue`] |
//! | state 0 | distances 0xff; the record; both paths' headings (`0x2f15e8`: each point's w = the yaw to the next, the last the one before); path 0 / 0, speed 30·dt, base z; an approach path: +0x174 = its first segment, at its first point, → 1, hidden, collision off, not targetable; else → 6 | [`update`], [`headings`] |
//! | 1 | game mode 0 and (no gate or Ratchet's feet in the gate polygon): → 2, shown, targetable, the class's collision; each rider seated (`0x2dd9e8`: offset (1, 0, −3) (0x1da310), yaw offset π) | [`update`] |
//! | 2 | riders aboard and (count − point)·segment < 16: the ramp `0x270830(π/2, 4π·dt², 4π·dt², 2π·dt)`; the approach (`0x2f14d0`); at its end: riders → 3, an exit path → 4, else → 6 | [`update`], [`follow`] |
//! | 3 | the drop timer out: the first rider dropped (`0x2dda08(rider, ticks(90))`), its slot −1, timer `ticks(30)`; none left → 4 (exit path) or 6 | [`update`] |
//! | 4 | yaw `0x270830` to the exit path's first heading (2π·dt², 2π·dt); within 0.01 → 5, path position 0 | [`update`] |
//! | 5 | the ramp back to 0; the exit path; at its end: no target region → deleted, else → 6 | [`update`] |
//! | 6 | the ramp to 0; yaw to the target (π·dt², 2π·dt); roll to 0 (π/2·dt², 2π·dt, +0x188); the shot timer out and a target → 7 | [`update`] |
//! | 7 | a shot (`0x2a7eb8`) from joint list 1 at 25·dt toward the target, its blend `ticks(45)`, class sound 1; → 6, timer `ticks(180)` | [`update`] |
//! | 8 | the base −2·dt, roll += 15°·dt; the fall timer out or Giant Clank: `SpawnBeamExplosion(0, 0, 4, 2.5, 9, 2, 0, m, 0, position, 20, 12, 8, −1, 0)`, pieces 0x634..0x636, `SetDeathBits`, deleted | [`update`] |
//! | tail | bob phase += 90°·dt, z = base + 0.6·sin; the ramp (`0x2f1698`: +0x144 = the ramp, `0x2777d8(0.03, 0.3, +0xe0, list 2)`) | [`update`] (`manip::look`) |
//! | `0x2f14d0(m, path)` | t = point + fraction + speed / segment; point / fraction kept; before the last point: position / rotation at t (`0x277d40`, open), base z, the speed `0x270830` toward the distance to the last point (15·dt², 30·dt); returns "at the end" | [`follow`] (`path::pose`) |
//! | `0x2a7eb8(m, v, p, at, n)` | `CreateMoby(0x32)`: distances 0xff, state 0, scale ·3, the owner's light; +0 v, +0x10 at, +0x20 = +0x30 = n, +0x24 \|v\|, +0x28 `randf(−30°, 30°)` [L: the decompile shows one bound], +0x2c `randf(0, 20°)`, +0x34 0; yaw / pitch from v; hidden | [`spawn_shot`] |
//! | `0x2a7b90` | within 15 of the camera once: `PlayClassSoundByClass(5, 0, m, 0x54c)`; state 0: the trail; d = at − position; aim = polar(\|v\|, atan d + yaw off, pitch(d) + pitch off clamped ±70°); the blend running: v' = lerp(t / n, aim, v), else aim; `CollLine_Fix(position, position + v', 0, m)` or \|d\| < \|v\|: (Giant Clank: `0x26e830(2, 2, 1, m, position, 3, 0, 1, 0)`), sound 4, `SpawnBeamExplosion(4, 4, 4, 2, 9, 1, 15, m, v, position, 10, 3, 16, −1, shake)`, deleted; else moves on | [`shot_update`] |
//! | `0x2a8010` | as the trooper shot's trail: `rand_vec(0.25·dt)` / `rand_vec(2·dt)`, sizes 0.5 / 0.25, phases 2 / 10..20 / 10..20, colours 0x80207080 / 0x401040ff | [`shot_trail`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::{self as c, attack, damage, flash, fx, target, turn, DT, DT2};
use crate::moby_update::manip;
use crate::moby_update::services::{self as sv, pvar as p, World};
use crate::ps2v::Pf;

use super::drop_trooper;

pub const REFERENCE_LEVEL: u32 = 18;
pub const UPDATE_FN: u32 = 0x2f_0920;
pub const CLASSES: [i16; 1] = [1356];
pub const SHOT_FN: u32 = 0x2a_7b90;
pub const SHOT_CLASSES: [i16; 1] = [SHOT];

const SHOT: i16 = 0x32;
const PIECES: [i16; 3] = [0x634, 0x635, 0x636];
/// 0x1da310: where a rider sits.
const SEAT: [f32; 3] = [1.0, 0.0, -3.0];
const SPEED: f32 = 30.0;
const ACCEL: f32 = 15.0;
const SHOT_SPEED: f32 = 25.0;

mod pv {
    pub const LEN: usize = 0x198;
    pub const D: usize = 0x20;
    pub const F: usize = 0x60;
    pub const T: usize = 0x70;
    pub const T_MOBY: usize = 0xb0;
    pub const T_KIND: usize = 0xb4;
    pub const RIDERS: usize = 0xc0;
    pub const REGION: usize = 0xd0;
    pub const GATE: usize = 0xd4;
    pub const PATH_IN: usize = 0xd8;
    pub const PATH_OUT: usize = 0xdc;
    pub const RAMP_REC: usize = 0xe0;
    pub const FALL: usize = 0x160;
    pub const FIRE: usize = 0x164;
    pub const POINT: usize = 0x168;
    pub const FRAC: usize = 0x16c;
    pub const SPEED: usize = 0x170;
    pub const SEG: usize = 0x174;
    pub const YAW_V: usize = 0x178;
    pub const RAMP: usize = 0x17c;
    pub const RAMP_V: usize = 0x180;
    pub const DROP: usize = 0x184;
    pub const ROLL_V: usize = 0x188;
    pub const BOB: usize = 0x18c;
    pub const HUM: usize = 0x190;
    pub const BASE: usize = 0x194;
}

mod shot {
    pub const LEN: usize = 0x38;
    pub const VEL: usize = 0x00;
    pub const AT: usize = 0x10;
    pub const TIMER: usize = 0x20;
    pub const SPEED: usize = 0x24;
    pub const YAW: usize = 0x28;
    pub const PITCH: usize = 0x2c;
    pub const TOTAL: usize = 0x30;
    pub const HEARD: usize = 0x34;
}

fn gscale(w: &World, x: f32) -> i32 { w.svc.timing.scale(Pf::f(x)).to_f32() as i32 }

fn rider(w: &World, id: MobyId, k: usize) -> Option<MobyId> {
    usize::try_from(c::pi32(w, id, pv::RIDERS + 4 * k)).ok().filter(|&m| m < w.table.mobys.len())
}

fn path_of(w: &World, id: MobyId, o: usize) -> Option<usize> {
    usize::try_from(c::pi32(w, id, o)).ok().filter(|&i| i < w.svc.splines.len())
}

/// `0x270830(t, a, a, vmax, &x, &v)` on two pvar floats.
fn spring_pv(w: &mut World, id: MobyId, t: f32, a: f32, vmax: f32, x: usize, v: usize) {
    let (mut xv, mut vv) = (c::pf(w, id, x), c::pf(w, id, v));
    turn::spring(t, a, a, vmax, &mut xv, &mut vv);
    c::set_pf(w, id, x, xv);
    c::set_pf(w, id, v, vv);
}

/// [`spring_pv`] on one of the moby's Euler angles.
fn spring_rot(w: &mut World, id: MobyId, t: f32, a: f32, vmax: f32, axis: usize, v: usize) {
    let (mut xv, mut vv) = (w.m(id).rotation[axis], c::pf(w, id, v));
    turn::spring(t, a, a, vmax, &mut xv, &mut vv);
    w.mm(id).rotation[axis] = xv;
    c::set_pf(w, id, v, vv);
}

/// Level18 `0x2f0920` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    crate::moby_update::story::pvars(w, id, pv::LEN);
    if !prologue(w, id) { return; }
    let st = w.m(id).state;
    let tau = std::f32::consts::TAU;
    let pi = std::f32::consts::PI;
    match st {
        0 => {
            {
                let m = w.mm(id);
                m.update_dist = 0xff;
                m.draw_dist = 0xff;
                let pvs = &mut m.pvars;
                p::set_ff(pvs, pv::D, 12.0);
                p::set_i16(pvs, pv::D + 4, 12);
                p::set_u8(pvs, pv::D + 8, 3);
            }
            for o in [pv::PATH_IN, pv::PATH_OUT] {
                if let Some(path) = path_of(w, id, o) { headings(w, path); }
            }
            c::set_pi32(w, id, pv::FRAC, 0);
            c::set_pi32(w, id, pv::POINT, 0);
            c::set_pf(w, id, pv::SPEED, SPEED * DT);
            let z = w.m(id).position[2];
            c::set_pf(w, id, pv::BASE, z);
            let Some(path) = path_of(w, id, pv::PATH_IN) else {
                w.mm(id).state = 6;
                return tail(w, id);
            };
            let pts = &w.svc.splines[path];
            let at = |k: usize| pts.get(k).map_or([0.0; 4], |q| q.map(f32::from_bits));
            let (p0, p1) = (at(0), at(1));
            c::set_pf(w, id, pv::SEG, c::dist3(p0, p1));
            let m = w.mm(id);
            m.position = p0;
            m.state = 1;
            m.visible = 0;
            m.has_collision = false;
            m.mode = (m.mode & 0xefff) | 1;
        }
        1 => {
            let gate = c::pi32(w, id, pv::GATE);
            let inside = gate == -1 || w.in_path(w.hero_point(), gate);
            if w.svc.game_mode == 0 && inside {
                let has = w.classes.info(w.m(id).o_class).is_some_and(|i| i.has_collision);
                {
                    let m = w.mm(id);
                    m.state = 2;
                    m.mode &= 0xfffe;
                    m.visible = 1;
                    m.mode |= mode::TARGETABLE;
                    m.has_collision = has;
                }
                for k in 0..4 {
                    if let Some(r) = rider(w, id, k) { drop_trooper::seat(w, r, id, SEAT, f32::from_bits(0x4049_0fd0)); }
                }
            }
        }
        2 => {
            let aboard = (0..4).any(|k| c::pi32(w, id, pv::RIDERS + 4 * k) >= 0);
            let Some(path) = path_of(w, id, pv::PATH_IN) else { return tail(w, id) };
            let n = w.svc.splines[path].len() as i32;
            if aboard && ((n - c::pi32(w, id, pv::POINT)) as f32) * c::pf(w, id, pv::SEG) < 16.0 {
                spring_pv(w, id, std::f32::consts::FRAC_PI_2, DT2 * 12.566371, DT * tau, pv::RAMP, pv::RAMP_V);
            }
            if follow(w, id, path) {
                if aboard {
                    w.mm(id).state = 3;
                } else if path_of(w, id, pv::PATH_OUT).is_some() {
                    w.mm(id).state = 4;
                } else {
                    w.mm(id).state = 6;
                }
            }
        }
        3 => {
            let mut t = c::pi32(w, id, pv::DROP);
            if c::dec_timer_i32(&mut t) != 0 {
                c::set_pi32(w, id, pv::DROP, t);
                for k in 0..4 {
                    if let Some(r) = rider(w, id, k) {
                        let wait = w.ticks(0x5a);
                        drop_trooper::drop(w, r, wait);
                        c::set_pi32(w, id, pv::RIDERS + 4 * k, -1);
                        let t = w.ticks(0x1e);
                        c::set_pi32(w, id, pv::DROP, t);
                        break;
                    }
                    if k == 3 {
                        w.mm(id).state = if path_of(w, id, pv::PATH_OUT).is_some() { 4 } else { 6 };
                    }
                }
            } else {
                c::set_pi32(w, id, pv::DROP, t);
            }
        }
        4 => {
            let Some(path) = path_of(w, id, pv::PATH_OUT) else { return tail(w, id) };
            let yaw = w.svc.splines[path].first().map_or(0.0, |q| f32::from_bits(q[3]));
            spring_rot(w, id, yaw, DT2 * tau, DT * tau, 2, pv::YAW_V);
            if c::diff_rots(w.m(id).rotation[2], yaw) < 0.01 {
                w.mm(id).state = 5;
                c::set_pi32(w, id, pv::POINT, 0);
                c::set_pf(w, id, pv::FRAC, 0.0);
            }
        }
        5 => {
            spring_pv(w, id, 0.0, DT2 * 12.566371, DT * tau, pv::RAMP, pv::RAMP_V);
            let Some(path) = path_of(w, id, pv::PATH_OUT) else { return tail(w, id) };
            if follow(w, id, path) {
                if c::pi32(w, id, pv::REGION) == -1 {
                    w.delete_moby(id);
                    return;
                }
                w.mm(id).state = 6;
            }
        }
        6 => {
            spring_pv(w, id, 0.0, DT2 * 12.566371, DT * tau, pv::RAMP, pv::RAMP_V);
            let t = c::pv4(w, id, pv::T);
            let me = c::pos(w, id);
            let yaw = c::atan(t[0] - me[0], t[1] - me[1]);
            spring_rot(w, id, yaw, DT2 * pi, DT * tau, 2, pv::YAW_V);
            spring_rot(w, id, 0.0, DT2 * std::f32::consts::FRAC_PI_2, DT * tau, 0, pv::ROLL_V);
            if c::dec_timer_pvar_i32(w, id, pv::FIRE) != 0 && c::pi32(w, id, pv::T_KIND) != 2 { w.mm(id).state = 7; }
        }
        7 => {
            let m = w.joint_point(id, 1);
            let at = c::pv4(w, id, pv::T);
            let v = c::set_len3(c::sub(at, m), SHOT_SPEED * DT);
            w.play_sound(1, 0, id);
            let n = w.ticks(0x2d);
            spawn_shot(w, id, v, m, at, n);
            w.mm(id).state = 6;
            let t = w.ticks(0xb4);
            c::set_pi32(w, id, pv::FIRE, t);
        }
        8 => {
            let b = c::pf(w, id, pv::BASE) - (DT + DT);
            c::set_pf(w, id, pv::BASE, b);
            let r = w.m(id).rotation[0] + DT * 0.2617994;
            w.mm(id).rotation[0] = r;
            if c::dec_timer_pvar_i32(w, id, pv::FALL) != 0 || w.body() == 2 {
                let me = c::pos(w, id);
                let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.5, flash_dist: 9.0, scale: 2.0, light: 0.0, streaks: 20, sparks: 12, puffs: 8, debris: 0, sound: -1, shake: false };
                fx::beam_explosion(w, &b, Some(id), me);
                let rot = w.m(id).rotation;
                for class in PIECES { fx::break_piece(w, id, class, me, rot, 0, 0); }
                crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
                w.delete_moby(id);
                return;
            }
        }
        _ => {}
    }
    tail(w, id);
}

/// The bob and the ramp (module table).
fn tail(w: &mut World, id: MobyId) {
    let ph = c::add_rot(c::pf(w, id, pv::BOB), 90.0 * 0.017453292 * DT);
    c::set_pf(w, id, pv::BOB, ph);
    let z = c::pf(w, id, pv::BASE) + 0.6 * ph.sin();
    w.mm(id).position[2] = z;
    let ramp = c::pf(w, id, pv::RAMP);
    c::set_pf(w, id, pv::RAMP_REC + 0x64, ramp);
    manip::look(w, id, id, pv::RAMP_REC, 2, 0.03, 0.3);
}

/// `0x2f15e8(path)`: each point's w = the yaw to the next point, the last point's the one before (the dropship's
/// paths only; the game writes its path table in place).
fn headings(w: &mut World, path: usize) {
    let pts = &mut w.svc.splines[path];
    let n = pts.len();
    for i in 0..n.saturating_sub(1) {
        let (a, b) = (pts[i].map(f32::from_bits), pts[i + 1].map(f32::from_bits));
        pts[i][3] = c::atan(b[0] - a[0], b[1] - a[1]).to_bits();
    }
    if 1 < n { pts[n - 1][3] = pts[n - 2][3]; }
}

/// `0x2f14d0(m, path)` (module table): true at the path's end.
fn follow(w: &mut World, id: MobyId, path: usize) -> bool {
    let t = c::pi32(w, id, pv::POINT) as f32 + c::pf(w, id, pv::FRAC) + c::pf(w, id, pv::SPEED) / c::pf(w, id, pv::SEG);
    let i = t as i32;
    c::set_pi32(w, id, pv::POINT, i);
    c::set_pf(w, id, pv::FRAC, t - i as f32);
    let n = w.svc.splines[path].len() as i32;
    let running = i < n - 1;
    if running {
        let (pos, rot) = crate::path::pose(&w.svc.splines[path], false, t, true);
        let last = w.svc.splines[path].last().map_or([0.0; 4], |q| q.map(f32::from_bits));
        {
            let m = w.mm(id);
            m.position = pos;
            m.rotation = rot;
        }
        c::set_pf(w, id, pv::BASE, pos[2]);
        let d = c::dist3(pos, last);
        let (mut x, mut v) = (0.0, c::pf(w, id, pv::SPEED));
        turn::spring(d, ACCEL * DT2, ACCEL * DT2, SPEED * DT, &mut x, &mut v);
        c::set_pf(w, id, pv::SPEED, v);
    }
    !running
}

/// `0x2f11a8` (module table); false when the dropship is gone.
fn prologue(w: &mut World, id: MobyId) -> bool {
    let cs = w.classes.info(w.m(id).o_class).map_or(1.0, |i| i.scale);
    w.mm(id).scale = cs;
    let st = w.m(id).state;
    let slot = c::pi32(w, id, pv::HUM);
    if (2..6).contains(&st) {
        if !w.sound_alive(slot, id) {
            let s = w.play_sound(0, 4, id);
            c::set_pi32(w, id, pv::HUM, s);
        }
    } else if w.sound_alive(slot, id) {
        if slot != -1 && w.sound_owner(slot) == Some(id) { w.release_sound(slot, id); }
        c::set_pi32(w, id, pv::HUM, -1);
    }
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    if res.out5 != 1 && st != 8 {
        let h = c::pf(w, id, pv::D) - res.damage;
        c::set_pf(w, id, pv::D, h);
        let mut reaction = res.reaction as i32;
        if h <= 0.0 || (w.body() == 2 && 2.0 <= res.damage) { reaction = 1; }
        if (1..3).contains(&reaction) {
            w.play_sound(3, 0, id);
            if w.body() != 2 { w.play_sound(2, 0, id); }
            for k in 0..4 {
                if let Some(r) = rider(w, id, k) { w.delete_moby(r); }
            }
            w.mm(id).state = 8;
            let t = w.ticks(0x5a);
            c::set_pi32(w, id, pv::FALL, t);
            p::set_u8(&mut w.mm(id).pvars, pv::F + 7, 0x78);
        } else if (3..9).contains(&reaction) {
            w.play_sound(3, 0, id);
            p::set_u8(&mut w.mm(id).pvars, pv::F + 7, 0x78);
            let r = w.rng.randf_sym(30.0, 50.0);
            c::set_pf(w, id, pv::ROLL_V, r * 0.017453292 * DT);
        }
        flash::start(w, id, pv::F);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pv::F);
    let region = c::pi32(w, id, pv::REGION);
    if region == -1 || w.hero.group == 0x16 {
        c::set_pi32(w, id, pv::T_KIND, 2);
    } else {
        let t = target::acquire_in(w, id, 128.0, Some(region as usize));
        c::set_pv4(w, id, pv::T, t.pos);
        c::set_pi32(w, id, pv::T_MOBY, t.moby.map_or(0, |m| m as i32 + 1));
        c::set_pi32(w, id, pv::T_KIND, t.kind as i32);
    }
    if c::pi32(w, id, pv::T_MOBY) == 0 {
        let h = w.hero_moby.map_or(0, |h| h as i32 + 1);
        c::set_pi32(w, id, pv::T_MOBY, h);
        let hp = w.hero_point();
        c::set_pv4(w, id, pv::T, [hp[0], hp[1], hp[2], 0.0]);
    }
    true
}

/// `0x2a7eb8(m, v, p, at, n)` (module table).
pub fn spawn_shot(w: &mut World, owner: MobyId, v: c::V, pos: c::V, at: c::V, n: i32) -> Option<MobyId> {
    let s = w.create_moby(SHOT)?;
    let (light, ambient) = (w.m(owner).light, w.m(owner).ambient);
    let yaw_off = w.rng.randf(-30.0, 30.0) * 0.017453292;
    let pitch_off = w.rng.randf(0.0, 20.0) * 0.017453292;
    let m = w.mm(s);
    m.update_dist = 0xff;
    m.draw_dist = 0xff;
    m.state = 0;
    m.scale *= 3.0;
    m.light = light;
    m.ambient = ambient;
    m.position = pos;
    if m.pvars.len() < shot::LEN { m.pvars.resize(shot::LEN, 0); }
    let pvs = &mut m.pvars;
    p::set_v4f(pvs, shot::VEL, v);
    p::set_v4f(pvs, shot::AT, at);
    p::set_i32(pvs, shot::TIMER, n);
    p::set_i32(pvs, shot::TOTAL, n);
    p::set_ff(pvs, shot::SPEED, c::len3(v));
    p::set_ff(pvs, shot::YAW, yaw_off);
    p::set_i32(pvs, shot::HEARD, 0);
    p::set_ff(pvs, shot::PITCH, pitch_off);
    m.rotation[2] = c::atan(v[0], v[1]);
    m.visible = 0;
    m.rotation[1] = -c::atan((v[0] * v[0] + v[1] * v[1]).sqrt(), v[2]);
    m.mode |= 1;
    Some(s)
}

/// Level18 `0x2a7b90` (module table).
pub fn shot_update(w: &mut World, id: MobyId) {
    crate::moby_update::story::pvars(w, id, shot::LEN);
    if c::pi32(w, id, shot::HEARD) == 0 {
        let cam = crate::hero::physics::to_f32x3(w.camera);
        if c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 15.0 {
            c::set_pi32(w, id, shot::HEARD, 1);
            w.play_sound_as(5, 0, id, 0x54c);
        }
    }
    if w.m(id).state != 0 { return; }
    shot_trail(w, id);
    let me = c::pos(w, id);
    let pvs = &w.m(id).pvars;
    let (v, at) = (p::v4f(pvs, shot::VEL), p::v4f(pvs, shot::AT));
    let (speed, yo, po) = (p::ff(pvs, shot::SPEED), p::ff(pvs, shot::YAW), p::ff(pvs, shot::PITCH));
    let d = c::sub(at, me);
    let yaw = c::add_rot(c::atan(d[0], d[1]), yo);
    let pitch = c::add_rot(c::atan((d[0] * d[0] + d[1] * d[1]).sqrt(), d[2]), po).clamp(-1.2217305, 1.2217305);
    let aim = crate::targeting::polar(speed, yaw, pitch);
    let aim = [aim[0], aim[1], aim[2], 0.0];
    let next = if c::dec_timer_pvar_i32(w, id, shot::TIMER) == 0 {
        let f = c::pi32(w, id, shot::TIMER) as f32 / c::pi32(w, id, shot::TOTAL) as f32;
        let u: c::V = std::array::from_fn(|k| aim[k] + (v[k] - aim[k]) * f);
        let m = w.mm(id);
        m.rotation[2] = c::atan(u[0], u[1]);
        m.rotation[1] = -c::atan((u[0] * u[0] + u[1] * u[1]).sqrt(), u[2]);
        c::add(me, u)
    } else {
        let m = w.mm(id);
        m.rotation[2] = yaw;
        m.rotation[1] = pitch;
        c::add(me, aim)
    };
    let hit = w.line(sv::pv([me[0], me[1], me[2], 1.0]), sv::pv([next[0], next[1], next[2], 1.0]), 0, Some(id));
    if hit.is_some() || c::len3(d) < speed {
        if w.body() == 2 { attack::sphere_hit(w, 2.0, 2.0, 1.0, id, me, 3, 0, 1, 0); }
        w.play_sound_as(4, 0, id, 0x54c);
        let b = fx::Beam { damage_r: 4.0, damage: 4.0, flash: 4.0, flash2: 2.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 10, sparks: 3, puffs: 16, debris: 0, sound: -1, shake: true };
        fx::beam_explosion(w, &b, Some(id), me);
        w.delete_moby(id);
    } else {
        w.mm(id).position = next;
    }
}

/// `0x2a8010` (module table).
fn shot_trail(w: &mut World, id: MobyId) {
    let v = p::v4f(&w.m(id).pvars, shot::VEL);
    let j = w.rng.rand_vec(0.25 * DT, 0.25 * DT);
    let mut v1 = c::add(c::scale(v, 0.0), [j[0], j[1], j[2], 0.0]);
    let j = w.rng.rand_vec(2.0 * DT, 2.0 * DT);
    let mut v2 = c::add(c::scale(v, 0.0), [j[0], j[1], j[2], 0.0]);
    v1[3] = 0.5;
    v2[3] = 0.25;
    let t: [i32; 3] = std::array::from_fn(|k| { let (a, b) = [(2.0, 2.0), (10.0, 20.0), (10.0, 20.0)][k]; let r = w.rng.randf(a, b); gscale(w, r) });
    let a = crate::particles::type02::Spawn { pos: c::pos(w, id), v1, v2, c1: 0x8020_7080, c2: 0x4010_40ff, t, def: -1 };
    fx::part02(w, &a);
}
