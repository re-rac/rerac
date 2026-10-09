//! Orxon's small classes (level 10), read from the level10 decomp and disassembly. Native `f32`.
//!
//! * **1100, the cracked walls** (`0x2dd650`, census U359; 5 placed): already broken on an earlier visit (the
//!   collected byte or the death bit) → deleted. Else draw distance 0x80, and every hit record aimed at it with flags
//!   0x20000 from a class 0xac or 0x99 attacker: from the wall's front (the attacker's offset in the wall's frame, x
//!   > 0.25) both death bits and deleted; else the record's target cleared.
//! * **1033, the lift** (`0x295a38`, U343; 1 placed): the platform block's shared words (4 / 5 / 0 / 0), its rest
//!   point; up 1.5 over `scale(45)` ticks, a wait of `ticks(120)`, sound 0, down again, a wait, sound 0, and so on.
//!   (No riders carried: the game calls no `CarryRiders` here.)
//! * **1031, the pressure plates** (`0x2d92b8`, U355; 4 placed): pvar 0 the door (a moby index, −1 none: dim 8),
//!   pvar 1 a partner plate, +0x0c pressed by the partner, +0x10 a sound timer. Pressed when a moby other than a 0x3ef
//!   stands on it (listed within 1 and its ground probe lands on the plate within 0.1) or its mission is done: the
//!   partner's +0x0c set (none: its own cleared); pressed or pressed by the partner: a door in state 1 / 2 → 3
//!   (opening; with the timer out sound 0 on the door and the plate, timer `ticks(30)`), bright 0x7f. Released: a door
//!   in state 4 → 1 (sound 1 on the door, 0 on the plate), dim 8.
//! * **353, the sliding gate** (`0x2befe8`, U348; 1 placed): its rest point (+0x00) kept; shut until its spawn's
//!   collected byte or death bit is set; then it slides 3.75 along −y at dt a tick, and at the end its collected byte
//!   = mission + 2 and (on level 10) the story flag 0x13d3d8 is set; open: held 3.75 off.
//! * **1117, the sinking lava platforms** (`0x295c20`, U344; 2 placed): the platform block's shared words, the rest
//!   height +0xa0, random phases; idle they bob (0.1 at π/2·dt + `randf_sym(8°·dt)`) and rock (2.76°, 20° / 30° a
//!   second). Ratchet on one (`0x295bc8`, the ledge-aware test) → 2: it sinks, speeding up by 2·dt² to 2·dt, the sizzle
//!   (sound 0) every `rand_range(40, 60)` ticks; off it → 3: it rises back (braking so it stops at its rest height:
//!   `v −= v²/(2d)` once `v²/(4·dt²)` reaches the distance), → 1 at the top. Riders carried (`CarryRiders`, +0x60).
//! * **1421, the bridge** (`0x2e9648`, U370) and **1424, the sliding block** (`0x2e9990`, U371): shut while Ratchet
//!   is on foot (0x1413f4 = 0) and their spawn's latch is clear; when he stands in the cuboid (pvar 3) (1421) or once
//!   planet 12 is unlocked (0x13dd4c), with pvars 0 / 4 set (1421) or always (1424): both death bits, the fly-by camera
//!   record pvar 4 / 2 armed (`0x2f5a50`: +0x38 = 1, +0x39 = 0; class 19, logged), then `scale(90)` ticks of motion:
//!   1421 lowers (rot.y −80° over them), 1424 slides 1.5 back along its yaw (+π) from its rest point (+0x00); open (3).
//!   1424 also freezes its linked moby (pvar +0x1c, mode bit 2) until it opens. Already open: placed open at once. A
//!   reset latch puts them back (3 → 0).
//! * **1555, the scene thrusters** (`0x2eacd8`, U373): in a scene (game mode 2) 3 / 5 / 6 / 7 the infobot thrusters
//!   of actor 1 / 2 / 2 / 3 (`0x278450`).
//! * **1346, the guard turret** (`0x2e8ab0`, U368; 1 placed): pvar 0 a moby told to act (+0xbc = 1) once the turret
//!   acts, pvars 1–3 its targets, +0x10 the turn speed, pvar 5 a moby put in state 1 when the targets are gone. Its
//!   latch set → 3; else waiting (1) until another class sets 2. Firing (2): the first target still alive (not
//!   0xfe / 0xfd): it turns its back (yaw + π) toward it (`SpringTurn` 90°·dt², 180°·dt², 45°·dt), and within 2.5°
//!   fires Gaspar's cannon shell (`0x2e68b8` = the same code, `gaspar_cannon::shell`) from 1.5 out at 40·dt, life
//!   `ticks(30)`, that target done; none left → 3 and pvar 5's moby woken. 3: the targets deleted, the death bits.
//! * **1047, the generator core** (`0x2d9eb8`, U356; 1 placed, a target of 1346): already latched → its rubble
//!   (classes 1122 and 1921 at its point, z 59: `0x2dd848` / `0x2eb8c0`, both with empty updates) and deleted. A hit
//!   (0x40000): a beam explosion (4, 2, 100000, scale 3, light 15; 5 / 3 / 4, 1 debris, shake), the sound 0 of class
//!   0x99, 30 burning bits (`SpawnDebrisMoby`: class 0x70e, from (−1, ±3, ±3) in its frame out at `randf(5, 10)·dt`),
//!   a second explosion at its rows·(2.86, 1.57, 1.83) (the rows only: no position added, as the game does [L]), the
//!   rubble, both death bits, deleted.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2dd650` | 1100 | [`wall_update`] |
//! | `0x295a38` | 1033 | [`lift_update`] |
//! | `0x2d92b8` | 1031 | [`plate_update`] |
//! | `0x2befe8` | 353 | [`gate_update`] |
//! | `0x295c20` | 1117 | [`sink_update`] |
//! | `0x2e9648` / `0x2e9990` | 1421 / 1424 | [`bridge_update`] / [`block_update`] |
//! | `0x2eacd8` | 1555 | [`thrusters_update`] |
//! | `0x2e8ab0` | 1346 | [`turret_update`] |
//! | `0x2d9eb8` | 1047 | [`core_update`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, V, DT};
use crate::moby_update::services::{pf, pv, sphere_mobys_in, World};
use crate::moby_update::story;

pub const REFERENCE_LEVEL: u32 = 10;
pub const WALL_FN: u32 = 0x2d_d650;
pub const LIFT_FN: u32 = 0x29_5a38;
pub const PLATE_FN: u32 = 0x2d_92b8;
pub const GATE_FN: u32 = 0x2b_efe8;
pub const WALL_CLASSES: [i16; 1] = [1100];
pub const LIFT_CLASSES: [i16; 1] = [1033];
pub const PLATE_CLASSES: [i16; 1] = [1031];
pub const GATE_CLASSES: [i16; 1] = [353];
pub const SINK_FN: u32 = 0x29_5c20;
pub const BRIDGE_FN: u32 = 0x2e_9648;
pub const BLOCK_FN: u32 = 0x2e_9990;
pub const THRUSTERS_FN: u32 = 0x2e_acd8;
pub const SINK_CLASSES: [i16; 1] = [1117];
pub const BRIDGE_CLASSES: [i16; 1] = [1421];
pub const BLOCK_CLASSES: [i16; 1] = [1424];
pub const THRUSTERS_CLASSES: [i16; 1] = [1555];
pub const TURRET_FN: u32 = 0x2e_8ab0;
pub const TURRET_CLASSES: [i16; 1] = [1346];
pub const CORE_FN: u32 = 0x2d_9eb8;
pub const CORE_CLASSES: [i16; 1] = [1047];
pub const RUBBLE_CLASSES: [i16; 2] = [1122, 1921];

/// The spawn's collected byte (`0x1bbb04[uid]`) or its persistent death bit.
fn latched(w: &World, id: MobyId) -> bool {
    let sid = w.m(id).spawn_id;
    w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(w.svc.level, sid))
}

/// Level10 `0x2dd650`: the cracked walls (module doc).
pub fn wall_update(w: &mut World, id: MobyId) {
    if latched(w, id) {
        w.delete_moby(id);
        return;
    }
    w.mm(id).draw_dist = 0x80;
    let p = c::pos(w, id);
    let r = w.m(id).rows;
    for i in 0..w.svc.hits.records.len() {
        let rec = w.svc.hits.records[i];
        if rec.target != id || rec.flags & 0x2_0000 == 0 { continue; }
        let Some(a) = rec.attacker.filter(|&a| a < w.table.mobys.len()) else { continue };
        let class = w.m(a).o_class;
        if class == 0xac || class == 0x99 {
            let d = c::sub(p, w.m(a).position);
            // The rows transposed: the offset in the wall's frame.
            let x = r[0][0] * d[0] + r[0][1] * d[1] + r[0][2] * d[2];
            if 0.25 < x {
                story::death_bits(w, id);
                w.delete_moby(id);
                return;
            }
        }
        w.svc.hits.records[i].target = usize::MAX;
    }
}

/// Level10 `0x295a38`: the lift (module doc).
pub fn lift_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0xb4);
    let step = 1.5 / w.ticks(0x2d) as f32;
    match w.m(id).state {
        0 => {
            let p = c::pos(w, id);
            let pvars = &mut w.mm(id).pvars;
            crate::moby_update::services::pvar::set_u8(pvars, 0x28, 4);
            crate::moby_update::services::pvar::set_i16(pvars, 0x3e, 5);
            crate::moby_update::services::pvar::set_i32(pvars, 0x20, 0);
            crate::moby_update::services::pvar::set_i16(pvars, 0x24, 0);
            c::set_pv4(w, id, 0xa0, p);
            w.mm(id).state = 1;
        }
        1 => {
            let z = w.m(id).position[2] + step;
            w.mm(id).position[2] = z;
            let top = c::pf(w, id, 0xa8) + 1.5;
            if top <= z {
                w.mm(id).position[2] = top;
                w.mm(id).state = 2;
                let t = w.ticks(0x78);
                c::set_pi32(w, id, 0xb0, t);
            }
        }
        2 | 4 => {
            if c::dec_timer_pvar_i32(w, id, 0xb0) == 0 { return; }
            w.play_sound(0, 0, id);
            let s = w.m(id).state;
            w.mm(id).state = if s == 2 { 3 } else { 1 };
        }
        3 => {
            let z = w.m(id).position[2] - step;
            w.mm(id).position[2] = z;
            let base = c::pf(w, id, 0xa8);
            if z <= base {
                w.mm(id).position[2] = base;
                w.mm(id).state = 4;
                let t = w.ticks(0x78);
                c::set_pi32(w, id, 0xb0, t);
            }
        }
        _ => {}
    }
}

fn ambient(w: &mut World, id: MobyId, k: u8) {
    let a = &mut w.mm(id).ambient;
    a[0] = k;
    a[1] = k;
    a[2] = k;
}

fn link(w: &World, i: i32) -> Option<MobyId> { usize::try_from(i).ok().filter(|&m| m < w.table.mobys.len()) }

/// Level10 `0x2d92b8`: the pressure plates (module doc).
pub fn plate_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x14);
    let Some(door) = link(w, c::pi32(w, id, 0)) else {
        ambient(w, id, 8);
        return;
    };
    let p = c::pos(w, id);
    let list = sphere_mobys_in(w.table, w.svc, w.classes, pf(1.0), pv(p), 0, Some(id), None);
    let mut pressed = false;
    for q in list {
        let qp = w.m(q).position;
        let a: V = [qp[0], qp[1], qp[2] + 0.5, qp[3]];
        let b: V = [qp[0], qp[1], f32::from_bits(0x3c23_d70a), qp[3]];
        let hit = w.coll_line(pv(a), pv(b), 2, None);
        let g = hit.map_or(0.0, |h| h.point[2]);
        if hit.and_then(|h| h.moby) == Some(id) && (g - qp[2]).abs() < 0.1 && w.m(q).o_class != 0x3ef { pressed = true; }
    }
    c::dec_timer_pvar_i32(w, id, 0x10);
    let mission = w.m(id).mission;
    if story::mission_done(w, mission as i32) { pressed = true; }
    if pressed {
        match link(w, c::pi32(w, id, 4)) {
            None => c::set_pi32(w, id, 0xc, 0),
            Some(o) => {
                story::pvars(w, o, 0x14);
                c::set_pi32(w, o, 0xc, 1);
            }
        }
    } else if c::pi32(w, id, 0xc) == 0 {
        if w.m(door).state == 4 {
            w.mm(door).state = 1;
            if c::pi32(w, id, 0x10) == 0 {
                w.play_sound(1, 0, door);
                w.play_sound(0, 0, id);
                let t = w.ticks(0x1e);
                c::set_pi32(w, id, 0x10, t);
            }
        }
        ambient(w, id, 8);
        return;
    } else {
        c::set_pi32(w, id, 0xc, 0);
    }
    if (1..=2).contains(&w.m(door).state) {
        w.mm(door).state = 3;
        if c::pi32(w, id, 0x10) == 0 {
            w.play_sound(0, 0, door);
            w.play_sound(0, 0, id);
            let t = w.ticks(0x1e);
            c::set_pi32(w, id, 0x10, t);
        }
    }
    ambient(w, id, 0x7f);
}

/// Level10 `0x2befe8`: the sliding gate (module doc).
pub fn gate_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x10);
    const SLIDE: f32 = -3.75;
    match w.m(id).state {
        0 => {
            let p = c::pos(w, id);
            c::set_pv4(w, id, 0, p);
            if latched(w, id) { w.mm(id).state = 1; }
        }
        1 => {
            let y = w.m(id).position[1] - DT;
            w.mm(id).position[1] = y;
            if y <= c::pf(w, id, 4) + SLIDE {
                let sid = w.m(id).spawn_id;
                let m = w.m(id).mission;
                w.svc.save.collected.insert(sid, m.wrapping_add(2));
                w.mm(id).state = 2;
                if w.svc.level == 10 { story::set_flag(w, story::flag_index(0x13_d3d8), 1); }
            }
        }
        2 => {
            let y = c::pf(w, id, 4) + SLIDE;
            w.mm(id).position[1] = y;
        }
        _ => {}
    }
}

/// Level10 `0x295c20`: the sinking lava platforms (module doc).
pub fn sink_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0xc0);
    let old = c::pos(w, id);
    let rot = w.m(id).rotation;
    if w.hero_on_moby(id) { w.mm(id).state = 2; }
    c::dec_timer_pvar_i32(w, id, 0xbc);
    let rest = c::pf(w, id, 0xa0);
    let mut idle = false;
    match w.m(id).state {
        0 => {
            let pvars = &mut w.mm(id).pvars;
            crate::moby_update::services::pvar::set_u8(pvars, 0x28, 4);
            crate::moby_update::services::pvar::set_i16(pvars, 0x3e, 5);
            crate::moby_update::services::pvar::set_i32(pvars, 0x20, 0);
            crate::moby_update::services::pvar::set_i16(pvars, 0x24, 0);
            let z = w.m(id).position[2];
            c::set_pf(w, id, 0xa0, z);
            for o in [0xa8, 0xb4, 0xb8] {
                let a = w.rng.rand_angle();
                c::set_pf(w, id, o, a);
            }
            let k = DT * f32::from_bits(0x3e0e_fa35);
            let r = w.rng.randf_sym(0.0, k);
            c::set_pf(w, id, 0xb0, DT * std::f32::consts::FRAC_PI_2 + r);
            w.mm(id).state = 1;
            idle = true;
        }
        1 => idle = true,
        2 => {
            let max = DT + DT;
            let v = (c::pf(w, id, 0xa4) - (DT * DT + DT * DT)).max(-max);
            c::set_pf(w, id, 0xa4, v);
            w.mm(id).position[2] += v;
            if !w.hero_on_moby(id) { w.mm(id).state = 3; }
            if c::pi32(w, id, 0xbc) < 1 {
                w.play_sound(0, 0, id);
                let t = w.rng.rand_range(0x28, 0x3c);
                c::set_pi32(w, id, 0xbc, t);
            }
        }
        3 => {
            let dt2 = DT * DT;
            let mut v = c::pf(w, id, 0xa4);
            let d = rest - w.m(id).position[2];
            let accel = v < 0.0 || (v * v) / (dt2 + dt2 + dt2 + dt2) < d;
            if accel {
                v = (v + dt2).min(DT + DT);
            } else if 0.0 < d {
                v = (v - (v * v) / (d + d)).max(0.0);
            }
            c::set_pf(w, id, 0xa4, v);
            let z = w.m(id).position[2] + v;
            w.mm(id).position[2] = z;
            if rest <= z {
                w.mm(id).position[2] = rest;
                w.mm(id).state = 1;
            }
        }
        _ => {}
    }
    if idle {
        c::set_pf(w, id, 0xa4, 0.0);
        let rate = c::pf(w, id, 0xb0);
        super::bob(w, id, f32::from_bits(0x3dcc_cccd), rate, 0xa8, 0xac);
        super::wobble(w, id, f32::from_bits(0x3d41_04fb), DT * f32::from_bits(0x3eb2_b8c2), DT * std::f32::consts::FRAC_PI_6, 0xb4, 0xb8);
    }
    let p = c::pos(w, id);
    let delta = [p[0] - old[0], p[1] - old[1], p[2] - old[2], p[3]];
    let rot_new = w.m(id).rotation;
    crate::moby_update::triggers::carry_riders(&mut w.mm(id).pvars, 0x60, delta, rot, rot_new);
}

/// The fly-by camera record `k` armed (`0x2f5a50`: its pvar +0x38 = 1, +0x39 = 0; camera class 19,
/// `crate::follow_camera::flyby`).
fn arm_flyby(w: &mut World, k: i32) { crate::cinematic::flyby_arm(w, k); }

const SWING: f32 = f32::from_bits(0x3fb2_b8c2);

/// Level10 `0x2e9648`: the bridge (module doc).
pub fn bridge_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x14);
    let on_foot = w.body() == 0;
    let latch = latched(w, id);
    let lower = |w: &mut World| {
        let y = c::add_rot(w.m(id).rotation[1], -SWING);
        w.mm(id).rotation[1] = y;
        w.mm(id).state = 3;
    };
    match w.m(id).state {
        0 => {
            w.mm(id).update_dist = 0xff;
            if on_foot && !latch {
                let y = w.m(id).rotation[1];
                c::set_pf(w, id, 4, y);
                w.mm(id).state = 1;
            } else {
                lower(w);
            }
        }
        1 => {
            if !(on_foot && !latch) {
                lower(w);
                return;
            }
            let inside = w.in_cuboid(w.hero_point(), c::pi32(w, id, 0xc));
            if !inside && !story::planet_unlocked(w, 12) { return; }
            if c::pi32(w, id, 0) < 1 || c::pi32(w, id, 0x10) < 1 { return; }
            story::death_bits(w, id);
            if w.in_cuboid(w.hero_point(), c::pi32(w, id, 0xc)) { arm_flyby(w, c::pi32(w, id, 0x10)); }
            let t = w.ticks(0x5a);
            c::set_pi32(w, id, 8, t);
            w.mm(id).state = 2;
        }
        2 => {
            let k = w.ticks(0x5a) as f32;
            let y = c::add_rot(w.m(id).rotation[1], -f32::from_bits(0x3fb2_b8c2) / k);
            w.mm(id).rotation[1] = y;
            if c::dec_timer_pvar_i32(w, id, 8) != 0 { w.mm(id).state = 3; }
        }
        3 => {
            if !on_foot || w.svc.save.collected.get(&w.m(id).spawn_id).is_some_and(|&b| b != 0) { return; }
            if w.svc.save.death.contains(&(w.svc.level, w.m(id).spawn_id)) { return; }
            let y = c::add_rot(w.m(id).rotation[1], SWING);
            w.mm(id).rotation[1] = y;
            w.mm(id).state = 0;
        }
        _ => {}
    }
}

fn freeze(w: &mut World, id: MobyId, on: bool) {
    let k = c::pi32(w, id, 0x1c);
    if k < 1 { return; }
    let Some(m) = link(w, k) else { return };
    if on { w.mm(m).mode |= 2 } else { w.mm(m).mode &= !2 }
}

/// Level10 `0x2e9990`: the sliding block (module doc).
pub fn block_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x20);
    let on_foot = w.body() == 0;
    let latch = latched(w, id);
    let back = |w: &World, k: f32| -> V {
        let a = c::add_rot(w.m(id).rotation[2], std::f32::consts::PI);
        let r = c::pv4(w, id, 0);
        [r[0] + a.cos() * k, r[1] + a.sin() * k, r[2], r[3]]
    };
    match w.m(id).state {
        0 => {
            w.mm(id).update_dist = 0xff;
            let p = c::pos(w, id);
            c::set_pv4(w, id, 0, p);
            if on_foot && !latch {
                freeze(w, id, true);
                w.mm(id).state = 1;
                return;
            }
            let at = back(w, 1.5);
            w.mm(id).state = 3;
            c::set_pos(w, id, at);
        }
        1 => {
            if on_foot && !latch {
                if !story::planet_unlocked(w, 12) { return; }
                freeze(w, id, false);
                story::death_bits(w, id);
                if 0 < c::pi32(w, id, 0x10) { arm_flyby(w, c::pi32(w, id, 0x10)); }
                let t = w.ticks(0x5a);
                c::set_pi32(w, id, 0x14, t);
                w.mm(id).state = 2;
                return;
            }
            let r = c::pv4(w, id, 0);
            w.mm(id).state = 3;
            c::set_pos(w, id, r);
            freeze(w, id, false);
        }
        2 => {
            let full = w.ticks(0x5a);
            let k = 1.5 * (full - c::pi32(w, id, 0x14)) as f32 / full as f32;
            let at = back(w, k);
            c::set_pos(w, id, at);
            if c::dec_timer_pvar_i32(w, id, 0x14) != 0 { w.mm(id).state = 3; }
        }
        3 => {
            if !on_foot || w.svc.save.collected.get(&w.m(id).spawn_id).is_some_and(|&b| b != 0) { return; }
            if w.svc.save.death.contains(&(w.svc.level, w.m(id).spawn_id)) { return; }
            let r = c::pv4(w, id, 0);
            w.mm(id).state = 0;
            c::set_pos(w, id, r);
        }
        _ => {}
    }
}

/// Level10 `0x2eacd8`: the scene thrusters (module doc).
pub fn thrusters_update(w: &mut World, id: MobyId) {
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.state = 1;
            m.update_dist = 0xff;
        }
        1 => {
            if w.svc.game_mode != 2 { return; }
            let Some(scene) = w.svc.cinematic.scene.clone() else { return };
            let k = match scene.id {
                3 => 1,
                5 | 6 => 2,
                7 => 3,
                _ => return,
            };
            if let Some(a) = scene.actors.get(k) { crate::moby_update::classes::cutscene_fx::infobot_thrusters(w, a); }
        }
        _ => {}
    }
}

/// A pvar's moby while it is alive (not deleted: state 0xfe / 0xfd).
fn alive_link(w: &World, id: MobyId, o: usize) -> Option<MobyId> {
    link(w, c::pi32(w, id, o)).filter(|&m| !matches!(w.m(m).state, 0xfe | 0xfd))
}

/// Level10 `0x2e8ab0`: the guard turret (module doc).
pub fn turret_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x1c);
    match w.m(id).state {
        0 => {
            if latched(w, id) {
                w.mm(id).state = 3;
                return;
            }
            if c::pi32(w, id, 4) == -1 {
                w.delete_moby(id);
                return;
            }
            c::set_pf(w, id, 0x10, 0.0);
            w.mm(id).state = 1;
        }
        2 => {
            if latched(w, id) {
                w.mm(id).state = 3;
                return;
            }
            let target = [4, 8, 0xc].into_iter().find_map(|o| alive_link(w, id, o).map(|m| (o, m)));
            match target {
                None => {
                    w.mm(id).state = 3;
                    if let Some(m) = link(w, c::pi32(w, id, 0x14)) {
                        w.mm(m).state = 1;
                        c::set_pi32(w, id, 0x14, -1);
                    }
                }
                Some((o, t)) => {
                    let pi = std::f32::consts::PI;
                    let yaw = c::add_rot(w.m(id).rotation[2], pi);
                    let p = c::pos(w, id);
                    let tp = w.m(t).position;
                    let bearing = c::atan(tp[0] - p[0], tp[1] - p[1]);
                    let mut v = c::pf(w, id, 0x10);
                    let yaw = crate::moby_update::classes::flyer::spring_turn(yaw, bearing, DT * DT * std::f32::consts::FRAC_PI_2, DT * DT * pi, DT * std::f32::consts::FRAC_PI_4, &mut v);
                    c::set_pf(w, id, 0x10, v);
                    w.mm(id).rotation[2] = yaw;
                    if c::diff_rots(yaw, bearing) < f32::from_bits(0x3d32_b8c2) {
                        let m = [p[0] + yaw.cos() * 1.5, p[1] + yaw.sin() * 1.5, p[2], p[3]];
                        let a = c::atan(tp[0] - m[0], tp[1] - m[1]);
                        let dxy = ((tp[0] - m[0]).powi(2) + (tp[1] - m[1]).powi(2)).sqrt();
                        let e = c::atan(dxy, tp[2] - m[2]);
                        let vel = crate::moby_update::creature::fx::polar(DT * 40.0, a, e);
                        let life = w.ticks(0x1e);
                        super::gaspar_cannon::shell(w, id, m, [vel[0], vel[1], vel[2], 0.0], life);
                        c::set_pi32(w, id, o, -1);
                    }
                    let z = c::add_rot(w.m(id).rotation[2], pi);
                    w.mm(id).rotation[2] = z;
                }
            }
            if let Some(m) = link(w, c::pi32(w, id, 0)) {
                w.mm(m).cmd = 1;
                c::set_pi32(w, id, 0, -1);
            }
        }
        3 => {
            if let Some(m) = alive_link(w, id, 4).or_else(|| alive_link(w, id, 8)) { w.delete_moby(m); }
            story::death_bits(w, id);
        }
        _ => {}
    }
}

/// `0x2dd848` / `0x2eb8c0`: a rubble piece at `m`'s point, z 59.
fn rubble(w: &mut World, id: MobyId, class: i16) {
    let Some(n) = w.create_moby(class) else { return };
    let (p, rz, l, a) = (w.m(id).position, w.m(id).rotation[2], w.m(id).light, w.m(id).ambient);
    let m = w.mm(n);
    m.update_dist = 0xff;
    m.draw_dist = 0xff;
    m.visible = 1;
    m.rotation = [0.0, 0.0, rz, m.rotation[3]];
    m.light = l;
    m.ambient = a;
    m.position = [p[0], p[1], 59.0, p[3]];
    w.build_matrix(n);
}

/// Level10 `0x2d9eb8`: the generator core (module doc).
pub fn core_update(w: &mut World, id: MobyId) {
    use crate::moby_update::creature::fx::{beam_explosion, Beam};
    const B: Beam = Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.0, flash_dist: 100_000.0, scale: 3.0, light: 15.0, streaks: 5, sparks: 3, puffs: 4, debris: 1, sound: -1, shake: true };
    if latched(w, id) {
        rubble(w, id, RUBBLE_CLASSES[0]);
        rubble(w, id, RUBBLE_CLASSES[1]);
        w.delete_moby(id);
        return;
    }
    if w.get_hit(id, 0x4_0000, false).is_none() { return; }
    let pos = c::pos(w, id);
    beam_explosion(w, &B, Some(id), pos);
    w.play_sound_as(0, 0, id, 0x99);
    let r = w.m(id).rows;
    let mul = |v: [f32; 3]| -> V { std::array::from_fn(|i| if i == 3 { 0.0 } else { r[0][i] * v[0] + r[1][i] * v[1] + r[2][i] * v[2] }) };
    for _ in 0..30 {
        let y = w.rng.randf(-3.0, 3.0);
        let z = w.rng.randf(-3.0, 3.0);
        let o = mul([-1.0, y, z]);
        let s = w.rng.randf(5.0, 10.0);
        let v = c::set_len3(o, s * DT);
        let at = c::add(o, pos);
        let _ = w.rng.randi(1);
        let scale = w.rng.randf(0.5, 1.0);
        let life = w.rng.rand_range(0x3c, 0x78);
        let g = w.rng.randf(2.0, 3.0);
        let keep = (w.rng.randi(4) == 0) as i32;
        crate::moby_update::classes::gunship::spawn_ember(w, scale, g, 1.0, 3.0, at, v, 0x70e, life, keep);
    }
    let o2 = mul([f32::from_bits(0x4037_0a3d), f32::from_bits(0x3fc8_f5c3), f32::from_bits(0x3fea_5e35)]);
    beam_explosion(w, &B, Some(id), o2);
    rubble(w, id, RUBBLE_CLASSES[0]);
    rubble(w, id, RUBBLE_CLASSES[1]);
    story::death_bits(w, id);
    w.delete_moby(id);
}
