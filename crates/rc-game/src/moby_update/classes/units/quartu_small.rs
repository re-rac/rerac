//! Quartu's small classes (level 15): **the bomb droppers 1394** (`0x2eabd8`, census U551, 2 placed) with **their
//! bombs 1257** (spawner `0x2e7ed8`, update `0x2e7f68`; created by code only, so the census never saw it) and **the
//! scene thrusters 1560** (`0x2edb20`, U560, one placed). Read from the level15 decomp and disassembly; native `f32`.
//!
//! * **1394**: updated always; every `scale(+0x04·60)` ticks at first, then every `scale(3.5·60)` (gp−0x4b84), a bomb
//!   1257 at its position.
//! * **1257**: falls (z velocity −20·dt² a tick); its sphere (0.5) touching something that is not a carrier, a hit
//!   (mask 0x830000) from another class, its line landing on Ratchet or landing faster than 9.75·dt: it blows up.
//!   Landing on a carrier (`0x275290`) it rides it (`0x2752c0`, the velocity the carried move) until the carrier
//!   is gone (it blows up) or the ground under it is no carrier's (it falls again). The blast: the beam explosion
//!   (flashes 2 / 1, light 7, 3 streaks, 3 sparks, 5 puffs, a debris piece), class 0x79's sound 0, hits within 2
//!   (damage 1, push 1 / 1, flags 0x830001, type 2 / 1), gone.
//! * **1560**: updated always; in a scene (game mode 2) 1..4 or 6, the thrusters (`0x252ef0` = L01 `0x278450`) on
//!   actor 4 (scene 1), 2 (scenes 2..4) or 1 (scene 6).
//!
//! * **92 the gates** (`0x2a36d0`, U536, 4 placed): open 4.25 up (or, Giant Clank's (+0x20 a cuboid), five times
//!   as fast down, the level word 0x184aa8 = 1 while open) in 0.6 s when Ratchet (as Giant Clank for theirs) enters
//!   +0x10, or when their link (+0x24) is gone with no alarm running (and then stay open); they close again once
//!   neither Ratchet nor the camera is in +0x14 (or Ratchet has left +0x20); the hum (sound 0) while moving. Giant
//!   Clank's stay open once planet 16 is unlocked.
//! * **1430 the dispenser** (`0x2ec030`, U556, one placed): carried by its link (+0x0c); hides its 24 groups;
//!   with flag 0x13d38a set and Ratchet on foot within 3: the fly-by camera +0x00 armed (camera class 19, not run:
//!   G-HERO-027, logged), then a piece 1428 slides out of it beside Ratchet (put 3 away facing it, state 0x72), the
//!   first group shown; the piece slides back over a full turn of `scale(360)` ticks; the level word 0x184aac = 1;
//!   with ten of the group gone, flag 0x13d3f5 and 15000 bolts. **1428** (`0x2ebd78`) does nothing on level 15.
//!
//! * **67 the sliding doors** (`0x29aff0`, U530, 8 placed; read from the disassembly, no decomp): slide +0x24 to the
//!   side (against the yaw) in 0.6 s, the hum (sound 0) while moving, when Ratchet enters +0x10 or their link (+0x1c:
//!   a 1209 or a dispenser 1430) reaches state 4 (or, with +0x28, they were opened for good before); they close once
//!   neither Ratchet nor the camera is in +0x14, and reopen if one comes back into +0x10; a linked (or +0x28) door
//!   stays open (its death bits).
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x29aff0` | 67 | [`door_update`] |
//! | `0x2a36d0` | 92 | [`gate_update`] |
//! | `0x2ec030` / `0x2eb9e0` / `0x2ebf28` / `0x2ebfb0` | 1430, the piece, a group shown / hidden, gone in a group | [`dispenser_update`] |
//! | `0x2ebd78` | 1428 | [`piece_update`] |
//! | `0x2eabd8` / `0x2e7ed8` | 1394, the bomb's spawn | [`dropper_update`], [`spawn_bomb`] |
//! | `0x2e7f68` | 1257 | [`bomb_update`] |
//! | `0x2edb20` | 1560 | [`thrusters_update`] |
//!
//! [L] The blast's position argument is 0 in the call; the bomb's position is used.

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, attack, fx};
use crate::moby_update::services::{pv, World};
use crate::moby_update::{story, triggers};
use std::f32::consts::FRAC_PI_2;
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 15;
pub const DROPPER_FN: u32 = 0x2e_abd8;
pub const DROPPER_CLASSES: [i16; 1] = [1394];
pub const BOMB_FN: u32 = 0x2e_7f68;
pub const BOMB_CLASSES: [i16; 1] = [1257];
pub const THRUSTERS_FN: u32 = 0x2e_db20;
pub const THRUSTERS_CLASSES: [i16; 1] = [1560];
pub const DOOR_FN: u32 = 0x29_aff0;
pub const DOOR_CLASSES: [i16; 1] = [67];
pub const GATE_FN: u32 = 0x2a_36d0;
pub const GATE_CLASSES: [i16; 1] = [92];
pub const DISPENSER_FN: u32 = 0x2e_c030;
pub const DISPENSER_CLASSES: [i16; 1] = [1430];
pub const PIECE_FN: u32 = 0x2e_bd78;
pub const PIECE_CLASSES: [i16; 1] = [1428];

/// gp−0x5708: the gates' rise.
const GATE_RISE: f32 = 4.25;
/// The level words the gates and the dispenser write (0x184aa8 / 0x184aac) and the alarm's (0x161ac0).
const GATE_WORD: u32 = 0x18_4aa8;
const DISPENSER_WORD: u32 = 0x18_4aac;
const FLAG_DISPENSER: usize = story::flag_index(0x13_d38a);
const FLAG_BOLTS: usize = story::flag_index(0x13_d3f5);

/// gp−0x4b84: the drop interval (seconds).
const INTERVAL: f32 = 3.5;
/// gp−0x4c44: the bombs' gravity (· dt²).
const GRAVITY: f32 = 20.0;
const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 4.0, scale: 1.0, light: 7.0, streaks: 3, sparks: 3, puffs: 5, debris: 1, sound: -1, shake: false };
const SOUND_CLASS: i16 = 0x79;

fn seconds(w: &World, s: f32) -> i32 { w.svc.timing.scale(Pf::f(s * 60.0)).to_i32() }

/// Level15 `0x2eabd8` (module doc).
pub fn dropper_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 8);
    let st = w.m(id).state;
    match st {
        0 => {
            let t = seconds(w, c::pf(w, id, 4));
            c::set_pi32(w, id, 0, t);
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.state = 1;
        }
        1 if c::dec_timer_pvar_i32(w, id, 0) != 0 => {
            let t = seconds(w, INTERVAL);
            c::set_pi32(w, id, 0, t);
            let p = c::pos(w, id);
            spawn_bomb(w, p);
        }
        _ => {}
    }
}

/// Level15 `0x2e7ed8`: a bomb 1257 at `p` (draw distance 0x7f, always updated, drawn, state 0, its velocity and
/// carrier 0, collision on).
pub fn spawn_bomb(w: &mut World, p: c::V) -> Option<MobyId> {
    let id = w.create_moby(BOMB_CLASSES[0])?;
    story::pvars(w, id, 0x20);
    let coll = super::class_collision(w, BOMB_CLASSES[0]);
    let m = w.mm(id);
    m.draw_dist = 0x7f;
    m.update_dist = 0xff;
    m.visible = 1;
    m.state = 0;
    m.cmd = 0;
    m.pvars[..0x20].fill(0);
    m.position = p;
    m.has_collision = coll;
    w.build_matrix(id);
    Some(id)
}

fn carrier(w: &World, id: MobyId) -> Option<MobyId> {
    let v = c::pi32(w, id, 0x10);
    usize::try_from(v - 1).ok().filter(|&m| v > 0 && m < w.table.mobys.len())
}
fn is_carrier(w: &World, m: Option<MobyId>) -> bool { m.is_some_and(|m| triggers::carrier(w.m(m)).is_some()) }

/// Level15 `0x2e7f68` (module doc).
pub fn bomb_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x20);
    let old = c::pos(w, id);
    let p = pv(old);
    if let Some(h) = w.coll_sphere(p, Pf::f(0.5), 1, Some(id)) {
        if !is_carrier(w, h.moby) { w.mm(id).cmd = 1; }
    }
    if let Some(a) = w.get_hit(id, 0x83_0000, false).and_then(|h| h.attacker) {
        if w.m(a).o_class != w.m(id).o_class { w.mm(id).cmd = 1; }
    }
    w.mm(id).hit_slot = 0xff;
    match w.m(id).state {
        0 => {
            let dt2 = crate::moby_update::creature::DT2;
            let vz = c::pf(w, id, 8) - GRAVITY * dt2;
            c::set_pf(w, id, 8, vz);
            let v = c::pv4(w, id, 0);
            let np = [old[0] + v[0], old[1] + v[1], old[2] + v[2], old[3]];
            w.mm(id).position = np;
            if let Some(h) = w.coll_line(p, pv(np), 0x10, Some(id)) {
                let at = [h.point[0], h.point[1], h.point[2], np[3]];
                if is_carrier(w, h.moby) {
                    c::set_pi32(w, id, 0x10, h.moby.map_or(0, |m| m as i32 + 1));
                    w.mm(id).state = 1;
                    c::set_pf(w, id, 8, 0.0);
                    w.mm(id).position = at;
                } else if h.moby.is_some() && h.moby == w.hero_moby || vz < crate::moby_update::creature::DT * -9.75 {
                    w.mm(id).cmd = 1;
                    w.mm(id).position = at;
                }
            }
        }
        1 => {
            let g = w.coll_line(pv([old[0], old[1], old[2] + 0.5, old[3]]), pv([old[0], old[1], 0.01, old[3]]), 2, None);
            let mut riding = false;
            if let Some(cm) = carrier(w, id) {
                if w.m(cm).state >= 0xfd {
                    w.mm(id).cmd = 1;
                    c::set_pi32(w, id, 0x10, 0);
                } else if g.as_ref().is_some_and(|g| g.moby.is_some()) && is_carrier(w, g.and_then(|g| g.moby)) {
                    riding = true;
                    if let Some(cv) = triggers::carrier(w.m(cm)) {
                        let rot = w.m(id).rotation;
                        let (q, r) = triggers::carried(&cv, [old[0], old[1], old[2]], [rot[0], rot[1], rot[2]]);
                        let m = w.mm(id);
                        m.position = [q[0], q[1], q[2], old[3]];
                        m.rotation = [r[0], r[1], r[2], rot[3]];
                        c::set_pv4(w, id, 0, [q[0] - old[0], q[1] - old[1], q[2] - old[2], 0.0]);
                    }
                }
            }
            if !riding {
                w.mm(id).state = 0;
                c::set_pf(w, id, 8, 0.0);
                c::set_pi32(w, id, 0x10, 0);
            }
        }
        _ => {}
    }
    if w.m(id).cmd != 0 { blow(w, id); }
}

fn blow(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    fx::beam_explosion(w, &BLAST, Some(id), p);
    w.play_sound_as(0, 0, id, SOUND_CLASS);
    attack::area_hit(w, 2.0, p, id, 1.0, 1.0, 1.0, None, 0x83_0001, 2, 1);
    w.delete_moby(id);
}

/// Level15 `0x2edb20` (module doc).
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
                1 => 4,
                2..=4 => 2,
                6 => 1,
                _ => return,
            };
            if let Some(a) = scene.actors.get(k) { crate::moby_update::classes::cutscene_fx::infobot_thrusters(w, a); }
        }
        _ => {}
    }
}

fn unlocked16(w: &World) -> bool { w.svc.interact.game.planet_unlocked.get(16).copied().unwrap_or(0) != 0 }

/// Level15 `0x2a36d0` (module doc).
pub fn gate_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x2c);
    let giant = c::pi32(w, id, 0x20) != -1;
    let home_z = c::pf(w, id, 8);
    let hero = w.hero_point();
    let cam = w.camera_point();
    let cam = [cam[0], cam[1], cam[2]];
    let speed = || { let v = GATE_RISE / 0.6 * crate::moby_update::creature::DT; if giant { v * 5.0 } else { v } };
    let set_z = |w: &mut World| {
        let f = c::pf(w, id, 0x18);
        w.mm(id).position[2] = home_z + if giant { -f } else { f };
    };
    let hum = |w: &mut World| {
        if !w.sound_alive(c::pi32(w, id, 0x1c), id) {
            let v = w.play_sound(0, 4, id);
            c::set_pi32(w, id, 0x1c, v);
        }
    };
    let quiet = |w: &mut World| {
        let v = c::pi32(w, id, 0x1c);
        if v != -1 && w.sound_alive(v, id) { w.release_sound(v, id); }
        c::set_pi32(w, id, 0x1c, -1);
    };
    match w.m(id).state {
        0 => {
            let p = c::pos(w, id);
            c::set_pv4(w, id, 0, p);
            w.mm(id).state = 1;
            c::set_pi32(w, id, 0x28, 0);
            if giant { w.mm(id).update_dist = 0xff; }
            if unlocked16(w) && giant {
                w.mm(id).state = 5;
                w.mm(id).position[2] = p[2] - GATE_RISE;
            }
        }
        1 => {
            c::set_pf(w, id, 0x18, 0.0);
            if giant { w.svc.units.set_word(GATE_WORD, 0); }
            let link = c::pi32(w, id, 0x24);
            if link == -1 {
                if w.in_cuboid(hero, c::pi32(w, id, 0x10)) && (!giant || w.body() == 2) { w.mm(id).state = 2; }
            } else {
                let gone = usize::try_from(link).ok().filter(|&m| m < w.table.mobys.len()).is_none_or(|m| w.m(m).state >= 0xfd);
                if gone && w.svc.units.word(super::quartu_alarm::lw::PLAYING) == 0 {
                    w.mm(id).state = 2;
                    c::set_pi32(w, id, 0x28, 1);
                }
            }
            if unlocked16(w) && giant {
                w.mm(id).state = 5;
                w.mm(id).position[2] = home_z - GATE_RISE;
            }
        }
        2 => {
            hum(w);
            let f = c::pf(w, id, 0x18) + speed();
            c::set_pf(w, id, 0x18, f);
            if GATE_RISE < f {
                quiet(w);
                c::set_pf(w, id, 0x18, GATE_RISE);
                w.mm(id).state = 3;
            }
            set_z(w);
        }
        3 => {
            if giant { w.svc.units.set_word(GATE_WORD, 1); }
            let watched = w.in_cuboid(hero, c::pi32(w, id, 0x14)) || w.in_cuboid(cam, c::pi32(w, id, 0x14));
            if watched && !w.in_cuboid(hero, c::pi32(w, id, 0x20)) { return; }
            if c::pi32(w, id, 0x28) == 0 { w.mm(id).state = 4; }
        }
        4 => {
            hum(w);
            let f = c::pf(w, id, 0x18) - speed();
            c::set_pf(w, id, 0x18, f);
            if f < 0.0 {
                quiet(w);
                c::set_pf(w, id, 0x18, 0.0);
                w.mm(id).state = 1;
                if giant {
                    w.mm(id).state = 5;
                    w.svc.units.set_word(GATE_WORD, 0);
                }
            }
            set_z(w);
            let open = c::pi32(w, id, 0x10);
            if w.in_cuboid(hero, open) || w.in_cuboid(cam, open) { w.mm(id).state = 2; }
        }
        5 => {
            if !unlocked16(w) {
                if w.body() != 2 {
                    w.mm(id).state = 1;
                    w.mm(id).position[2] = home_z;
                    w.svc.units.set_word(GATE_WORD, 0);
                }
            } else {
                w.mm(id).position[2] = home_z - GATE_RISE;
                w.svc.units.set_word(GATE_WORD, 1);
            }
        }
        _ => {}
    }
}

/// `0x2ebf28(g, shown)`: group `g` shown (drawn, collision) or hidden.
fn show_group(w: &mut World, g: i32, shown: bool) {
    for m in crate::moby_update::scheduler::group_ids(w, g as i8) {
        let coll = super::class_collision(w, w.m(m).o_class);
        let mo = w.mm(m);
        if shown {
            mo.visible = 1;
            mo.mode &= 0xfffc;
            mo.has_collision = coll;
        } else {
            mo.has_collision = false;
            mo.visible = 0;
            mo.mode |= 3;
        }
    }
}

/// `0x2eb9e0(pos, rot)`: a piece 1428 (always updated and drawn).
fn make_piece(w: &mut World, p: c::V, rot: [f32; 4]) -> Option<MobyId> {
    let m = w.create_moby(PIECE_CLASSES[0])?;
    let mo = w.mm(m);
    mo.update_dist = 0xff;
    mo.draw_dist = 0xff;
    mo.visible = 1;
    mo.position = p;
    mo.rotation = rot;
    w.build_matrix(m);
    Some(m)
}

/// Level15 `0x2ec030` (module doc).
pub fn dispenser_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x80);
    let link = c::pi32(w, id, 0x0c);
    if let Some(l) = usize::try_from(link).ok().filter(|&m| m < w.table.mobys.len()) {
        let lp = c::pos(w, l);
        if w.m(id).state != 0 {
            let d = c::sub(lp, c::pv4(w, id, 0x10));
            let p = c::add(c::pos(w, id), [d[0], d[1], d[2], 0.0]);
            w.mm(id).position = p;
        }
        c::set_pv4(w, id, 0x10, lp);
    }
    let yaw = w.m(id).rotation[2];
    let side = c::add_rot(yaw, -FRAC_PI_2);
    let off = |k: f32| [side.cos() * k, side.sin() * k, 0.0, 0.0];
    let piece = |w: &World| usize::try_from(c::pi32(w, id, 4) - 1).ok().filter(|&m| m < w.table.mobys.len());
    match w.m(id).state {
        0 => {
            let (lvl, sid) = (w.svc.level, w.m(id).spawn_id);
            let gone = w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(lvl, sid));
            if gone {
                w.mm(id).state = 4;
            } else {
                w.svc.units.set_word(DISPENSER_WORD, 0);
                w.mm(id).state = 1;
                for k in 0..24 {
                    let g = c::pi32(w, id, 0x20 + 4 * k);
                    if g != -1 { show_group(w, g, false); }
                }
            }
        }
        1 => {
            if story::flag(w, FLAG_DISPENSER) != 0 && c::dist2(super::hero_pos(w), c::pos(w, id)) < 3.0 && w.body() == 0 {
                // 0x2f78b8: the fly-by camera record +0x00 armed (camera class 19).
                let rec = c::pi32(w, id, 0x00);
                crate::cinematic::flyby_arm(w, rec);
                w.mm(id).state = 2;
            }
        }
        2 => {
            if w.camera_class != 0x13 {
                let p = c::add(c::pos(w, id), off(2.0));
                let pc = make_piece(w, p, [0.0, FRAC_PI_2, c::add_rot(yaw, FRAC_PI_2), 0.0]);
                c::set_pi32(w, id, 4, pc.map_or(0, |m| m as i32 + 1));
                c::set_pf(w, id, 8, 0.0);
                let mut q = c::add(c::pos(w, id), off(3.0));
                q[2] = w.ground_height(Pf::f(0.5), pv(q), 0).to_f32();
                crate::cinematic::hero_teleport(w, [q[0], q[1], q[2]], [0.0, 0.0, c::add_rot(yaw, FRAC_PI_2)], 0x72, false);
                let g = c::pi32(w, id, 0x20);
                if g != -1 { show_group(w, g, true); }
                w.mm(id).state = 3;
            }
        }
        3 => {
            if let Some(pc) = piece(w) {
                let per = w.svc.timing.scale(Pf::f(360.0)).to_f32();
                let a = c::add_rot(c::pf(w, id, 8), 360.0 / per * 0.017_453_292);
                c::set_pf(w, id, 8, a);
                let f = a.cos() + 1.0;
                let p = c::add(c::pos(w, id), off(f));
                w.mm(pc).position = p;
                if f < 0.5 {
                    story::death_bits(w, id);
                    w.mm(id).state = 4;
                }
            }
        }
        4 => {
            w.svc.units.set_word(DISPENSER_WORD, 1);
            let p = c::add(c::pos(w, id), off(0.5));
            let rot = [0.0, FRAC_PI_2, c::add_rot(yaw, FRAC_PI_2), 0.0];
            match piece(w) {
                None => {
                    let pc = make_piece(w, p, rot);
                    c::set_pi32(w, id, 4, pc.map_or(0, |m| m as i32 + 1));
                }
                Some(pc) => {
                    let m = w.mm(pc);
                    m.position = p;
                    m.rotation = rot;
                }
            }
            if story::flag(w, FLAG_BOLTS) == 0 {
                let g = c::pi32(w, id, 0x20);
                let gone = crate::moby_update::scheduler::group_ids(w, g as i8).iter().filter(|&&m| w.m(m).state >= 0xfd).count();
                if 9 < gone {
                    story::set_flag(w, FLAG_BOLTS, 1);
                    story::add_bolts(w, 15000);
                }
            }
        }
        _ => {}
    }
}

/// Level15 `0x2ebd78`: nothing on level 15 (the code's own test); the rest of it has no caller here.
pub fn piece_update(w: &mut World, _id: MobyId) {
    if w.svc.level == 15 { return; }
    w.svc.unported("quartu 1428: the item pickup off level 15");
}

/// Level15 `0x29aff0` (module doc).
pub fn door_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x2c);
    let hero = w.hero_point();
    let cam = w.camera_point();
    let cam = [cam[0], cam[1], cam[2]];
    let slide = |w: &mut World| {
        let (y, f, home) = (w.m(id).rotation[2], c::pf(w, id, 0x18), c::pv4(w, id, 0));
        let p = [home[0] + y.cos() * -f, home[1] + y.sin() * -f, home[2], home[3]];
        w.mm(id).position = p;
    };
    let hum = |w: &mut World| {
        if !w.sound_alive(c::pi32(w, id, 0x20), id) {
            let v = w.play_sound(0, 4, id);
            c::set_pi32(w, id, 0x20, v);
        }
    };
    let quiet = |w: &mut World| {
        let v = c::pi32(w, id, 0x20);
        if v != -1 && w.sound_alive(v, id) { w.release_sound(v, id); }
        c::set_pi32(w, id, 0x20, -1);
    };
    let step = |w: &World| c::pf(w, id, 0x24) / f32::from_bits(0x3f19_999a) * crate::moby_update::creature::DT;
    match w.m(id).state {
        0 => {
            let p = c::pos(w, id);
            c::set_pv4(w, id, 0, p);
            w.mm(id).state = 1;
        }
        1 => {
            c::set_pf(w, id, 0x18, 0.0);
            let mut go = false;
            if let Some(l) = usize::try_from(c::pi32(w, id, 0x1c)).ok().filter(|&m| m < w.table.mobys.len()) {
                let lc = w.m(l).o_class;
                if (lc == 0x4b9 || lc == 0x596) && w.m(l).state == 4 { go = true; }
            }
            if c::pi32(w, id, 0x28) != 0 {
                let (lvl, sid) = (w.svc.level, w.m(id).spawn_id);
                if w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(lvl, sid)) { go = true; }
            }
            if w.in_cuboid(hero, c::pi32(w, id, 0x10)) || go { w.mm(id).state = 2; }
        }
        2 => {
            hum(w);
            let f = c::pf(w, id, 0x18) + step(w);
            c::set_pf(w, id, 0x18, f);
            let full = c::pf(w, id, 0x24);
            if full < f {
                quiet(w);
                c::set_pf(w, id, 0x18, full);
                if c::pi32(w, id, 0x1c) == -1 && c::pi32(w, id, 0x28) == 0 {
                    w.mm(id).state = 3;
                } else {
                    story::death_bits(w, id);
                    w.mm(id).state = 5;
                }
            }
            slide(w);
        }
        3 => {
            let keep = c::pi32(w, id, 0x14);
            if !w.in_cuboid(hero, keep) && !w.in_cuboid(cam, keep) { w.mm(id).state = 4; }
        }
        4 => {
            hum(w);
            let f = c::pf(w, id, 0x18) - step(w);
            c::set_pf(w, id, 0x18, f);
            if f < 0.0 {
                quiet(w);
                c::set_pf(w, id, 0x18, 0.0);
                w.mm(id).state = 1;
            }
            slide(w);
            let open = c::pi32(w, id, 0x10);
            if w.in_cuboid(hero, open) || w.in_cuboid(cam, open) { w.mm(id).state = 2; }
        }
        _ => {}
    }
}
