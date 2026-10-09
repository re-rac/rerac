//! Teleporter pads, class 1135: `TeleporterPadUpdate` level01 0x308bd8 with the activation `FUN_00308b28`, the show
//! `FUN_00309838`, the hide `FUN_003097e8`, the prompt `FUN_00309430`, the arm placement `FUN_003092d0` and the beam
//! draw callback `FUN_003094f0`. The same code (masked identity, `rc-trace overlay-diff`) runs on levels 1, 7, 8, 12,
//! 13, 16, 17 and 18 (24 placed pads), every constant it reads is equal on all eight overlays (read from each). Spec:
//! `docs/plan/moby_update_catalogue.md` "In the port: teleporter pads and item offers", docs/plan/interaction.md §8.
//! Pokitaru's pads 318 (level11 0x2f2518) are their own code (another partner rule, another beam word): not this port.
//! Native `f32`.
//!
//! Pvar block (P, s32 words): P[0] +0x00 the partner pad (moby link), P[1] +0x04 activation cuboid (−1: none),
//! P[2] +0x08 mission (−1: none), P[4] +0x10 the prompt's message index, P[5..8] +0x14 the three arm mobys (class 315;
//! port: `moby index + 1`, 0 = none), P[8] +0x20 arm spread 0..1, P[9] +0x24 arm tilt 0..1, P[10] / P[11] +0x28 /
//! +0x2c their spring velocities, P[12] +0x30 (−1 at the teleport's start; nothing reads it), P[13] +0x34 the beam
//! radius (0..1.95), P[14] +0x38 starts active, P[15] +0x3c challenge-mode only, P[16] / P[17] +0x40 / +0x44 the music
//! track and stinger of the arrival.
//!
//! | address | what | here |
//! |---|---|---|
//! | 0x308bd8 | `done` = the spawn id's per-id flag `0x1bbb04[id]` or its persistent death bit (`0x14c190 + L·0x100`) | [`update`] |
//! | 0x308bd8 | state ≠ 0, cuboid P[1] ≠ −1, not done, Ratchet (0x13f3d0) in the cuboid (`PointInCuboid` 0x274820): done; `0x308b28`; `0x309838`; state 5 → 1 | [`update`] (`World::in_cuboid`) |
//! | 0x308b28 | the death bits of the spawn id (persistent `0x14c190 + L·0x100` and this visit's `0x1ba950`); group −1 → state 1, else the group command `0x26e0e0(group, 1)` | [`activate`] (`scheduler::group_state`) |
//! | 0x309838 | pad +0x94 = class +0x10 (collision), +0x31 = 1, mode &= ~1; each arm ≠ 0: mode &= ~1, +0x31 = 1 | [`show`] |
//! | 0x308bd8 | the challenge gate: P[15] ≠ 0 and the times-completed count `0x15ee20` = 0 → +0x94 = 0, mode \|= 0x41, return (before the state switch); P[15] ≠ 0 with the count set: class collision, mode &= ~0x41 | [`update`] |
//! | state 0 | update distance 0xff; three `CreateMoby(315)` arms: draw distance 0x40, +0x31 = 1, +0x38 (light word, ambient) = Ratchet's moby's, mode = the pad's, position and Euler = the pad's, yaw + k·2π/3; P[14] ≠ 0 or done → 6, else 5 and `0x3097e8`; arm placement | [`update`], [`hide`], [`place_arms`] |
//! | 0x3097e8 | pad +0x31 = 0, mode \|= 1, +0x94 = 0; each arm ≠ 0: mode \|= 1, +0x31 = 0 | [`hide`] |
//! | state 6 | group ≠ −1 → `0x26e0e0(group, 1)`; state 1; on into 1 | [`update`] (`scheduler::group_state`) |
//! | state 1 | mission P[2] ≥ 0 not done (`0x14c050 + L·16` ≠ 0xff) → return; partner P[0] ≠ −1, body 0x1413f4 ≠ 2, Ratchet's ground moby 0x13f64c = the pad, 0x13f65e = 0 → the prompt `0x309430`; △ (0x13cae4 & 0x10, the lease not tested) → `PlayClassSound(0, 0, pad)`, state 2, P[12] = −1, P[10] = P[11] = 0, `0x249580(yaw, pos, 0)` (Ratchet walks to the pad's centre and faces its yaw), `0x26e830(1.5, 20, 1, pad, pos, 0x10000, 0, 1, 0)` (a 20-damage sphere hit on every moby on the pad) | [`update`] (`interact::try_prompt`, `HeroCall::WalkTo`, `attack::sphere_hit`) |
//! | 0x309430 | `try_set_help_message(4, msg)`: msg = table[P[4]] by level: 1 / 7 / 13 → 0x201600, 8 → gp−0x4c68, 12 → gp−0x4c60, 16 → gp−0x4c58, 17 → 0x201618, 18 → gp−0x4c50 (table words: [`PROMPT_ABS`], [`PROMPT_GP`]); other levels: nothing | [`prompt`] |
//! | state 2 | `0x2495d0` = Ratchet's walk (1: 0x65 / 0x66 walking, 2: 0x67 standing at the point, 0: other); spread P[8] < 1 → spring P[8] (velocity P[10]) to 1; else tilt P[9] < 1 → spring P[9] (P[11]) to 1; else, unless the prompt owner `0x15f594` is 4 and P[8] > 0.25: standing → P[13] = 1.95, state 3; not walking → state 4; arm placement | [`update`] (`turn::spring`, 12·dt², 12·dt², 32·dt: gp−0x4c78 / −0x4c74 / −0x4c70) |
//! | state 3 | P[13] ≠ 0: `RegisterDrawCallback(0x3094f0)` (list 1), `Approach(0, 4·dt, P[13])`, return (no arm placement) | [`update`] (`Callback::UnitQuads`, [`beam_quads`]) |
//! | state 3 | P[13] = 0: `HeroTeleport(partner pos + 0.2 z, partner Euler, 0, 1)`; 0x1413f5 = 1 (Ratchet hidden); P[16] ≠ −1 and P[17] ≠ −1 → `MusicRequestTrack(P[16], P[17])`; partner state 4, its P[13] = P[10] = P[11] = 0, P[9] = P[8] = 1; `PlayClassSound(1, 0, partner)`; state 1; P[8..12] = 0; P[13] = 1.95; arm placement | [`update`] (`cinematic::hero_teleport`, `HeroFields::hero_hidden`, `SoundSink::music_request`) |
//! | states 4, 8 | P[13] < 1.95: the beam callback, `Approach(1.95, 4·dt, P[13])`; else tilt P[9] > 0 → spring P[9] (velocity P[10]) to 0; else spread P[8] ≤ 0 → state 1; else spring P[8] (P[11]) to 0; arm placement | [`update`] |
//! | state 7 | spread P[8] < 1 → spring P[8] (P[10]) to 1; else tilt P[9] < 1 → spring P[9] (P[11]) to 1; else `PlayClassSound(1, 0, pad)`, state 8, P[13] = 0; arm placement | [`update`] |
//! | state 5, others | waits for the cuboid (above) / nothing | [`update`] |
//! | 0x3092d0 | per arm: position = pad + (0, 0, −0.4) (gp−0x4c88), rotation y = −20° (gp−0x4c80, ·π/180), position += row 0 of that rotation scaled to `P[8]·2.4 − 1.2`, rotation y = `P[9]·(−π/3) − 20°`, `MobyBuildMatrix` | [`place_arms`] |
//! | 0x3094f0 | the beam: 32 quads of a cylinder of radius P[13] around the pad (corners `(cos a·r, sin a·r, 1 ∓ h)`, a = (i + {0, 1})·2π/32 − π, in the pad's frame: rows and position), h = 0.1 + clamp(2·(1.15 − r), 0, 1) below r = 1.15, else 0.1; FX texture 0x13 (gp−0x4c38), ALPHA 0x48 (additive, gp−0x4c48..−0x4c3c = 0, 2, 0, 1; FIX 0x80), RGBA 0x60408080 (gp−0x4c34) on every corner, ST (0.5, 0) / (0.5, 1) (0x201630); z base 1.0 (gp−0x4c30) | [`beam_quads`] (drawn by `rc-engine` fx_draw) |
//! | 0x3094f0 | below r = 1.15 and the pad not in state 7 / 8: 0x1413f5 = (r < 0.65) (Ratchet hidden while the beam is narrow) | [`update`] (taken where the update registers the callback: the draw reads the radius and state the update leaves, and the hero reads 0x1413f5 on his next update either way) |
//!
//! Who enters state 7 (the arms opening with the class sound 1, then the beam): Kalebo's chicken pad 1923
//! (`units::kalebo_chicken_pad`, level16 `0x2e0de0`), which sends chickens through pad #1192; no other class does.

#![allow(clippy::neg_cmp_op_on_partial_ord)] // the angle wrap tests `!(s < π)` as the game's `c.lt.s` does.

use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::classes::units::{FxQuad, FxQuads};
use crate::moby_update::creature::{attack, turn, DT, DT2};
use crate::moby_update::interact::owner;
use crate::moby_update::scheduler::group_state;
use crate::moby_update::services::{pvar as p, HeroCall, Services, World};

/// The pad update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x308bd8;
/// The beam draw callback (level01).
pub const BEAM_FN: u32 = 0x3094f0;

/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [1135];

/// The arm class (`CreateMoby(0x13b)`).
pub const ARM_CLASS: i16 = 315;

/// The prompt tables at 0x201600 (levels 1 / 7 / 13 from +0, level 17 from +0x18), words in memory order.
pub const PROMPT_ABS: [i32; 12] = [13002, 13003, 1036, 1035, 7001, 7002, 17006, 17007, 17008, 17005, 17003, 0];
/// The prompt tables at gp−0x4c68 (level 8 from +0, 12 from +8, 16 from +0x10, 18 from +0x18), words in memory order
/// (the beam's ALPHA words follow at gp−0x4c48).
pub const PROMPT_GP: [i32; 12] = [8015, 8013, 12010, 12011, 21487, 21488, 18004, 18003, 0, 2, 0, 1];

/// gp−0x4c78 / −0x4c74 / −0x4c70: the arms' spring (accel and decel ·dt², top speed ·dt).
const SPRING: (f32, f32, f32) = (12.0, 12.0, 32.0);
/// The beam's full radius (0x3ff9999a).
const BEAM_MAX: f32 = 1.95;
/// The beam: FX texture (gp−0x4c38), corner RGBA (gp−0x4c34), base height (gp−0x4c30), ST (0x201630).
const BEAM_FX: usize = 0x13;
const BEAM_RGBA: u32 = 0x6040_8080;
const BEAM_Z: f32 = 1.0;
const BEAM_ST: [[f32; 2]; 4] = [[0.5, 0.0], [0.5, 1.0], [0.5, 0.0], [0.5, 1.0]];

const PI: f32 = std::f32::consts::PI;
/// π/180 (0x3c8efa35).
const DEG: f32 = 0.017_453_292;

/// `fast_add_rotations` 0x221ff8.
fn add_rot(a: f32, b: f32) -> f32 {
    let s = a + b;
    let below = s < -PI;
    if !(s < PI) { return (s - PI) - PI; }
    if below { return (s + PI) + PI; }
    s
}

pub(crate) fn arm(pv: &[u8], k: usize) -> Option<MobyId> { let v = p::i32(pv, 0x14 + 4 * k); (v > 0).then(|| (v - 1) as usize) }

fn pf(w: &World, id: MobyId, o: usize) -> f32 { p::ff(&w.m(id).pvars, o) }
fn set_pf(w: &mut World, id: MobyId, o: usize, v: f32) { p::set_ff(&mut w.mm(id).pvars, o, v) }
fn pi(w: &World, id: MobyId, o: usize) -> i32 { p::i32(&w.m(id).pvars, o) }
fn set_pi(w: &mut World, id: MobyId, o: usize, v: i32) { p::set_i32(&mut w.mm(id).pvars, o, v) }

/// `FUN_00270830(target, 12·dt², 12·dt², 32·dt, &P[x], &P[v])`.
pub(crate) fn spring_arm(w: &mut World, id: MobyId, target: f32, x: usize, v: usize) {
    let (mut xx, mut vv) = (pf(w, id, x), pf(w, id, v));
    turn::spring(target, SPRING.0 * DT2, SPRING.1 * DT2, SPRING.2 * DT, &mut xx, &mut vv);
    set_pf(w, id, x, xx);
    set_pf(w, id, v, vv);
}

/// The beam callback's registration and its store of 0x1413f5 (module table, the last rows).
fn beam(w: &mut World, id: MobyId) {
    if let Some(i) = crate::moby_update::classes::units::row(1, BEAM_FN) { w.svc.draw_callbacks.register(Callback::UnitQuads(i), id); }
}

/// The beam draw's 0x1413f5 store, from the radius and state the update leaves.
fn beam_hides_hero(w: &mut World, id: MobyId) {
    let r = pf(w, id, 0x34);
    if r < 1.15 && !matches!(w.m(id).state, 7 | 8) { w.hero_fields_mut().hero_hidden = Some((r < 0.65) as u8); }
}

/// `TeleporterPadUpdate` (0x308bd8).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x48 { return; }
    let spawn_id = w.m(id).spawn_id;
    let level = w.svc.level;
    let mut done = w.svc.save.collected.get(&spawn_id).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(level, spawn_id));
    if w.m(id).state != 0 && pi(w, id, 4) != -1 && !done && w.in_cuboid(w.hero_point(), pi(w, id, 4)) {
        done = true;
        activate(w, id);
        show(w, id);
        if w.m(id).state == 5 { w.mm(id).state = 1; }
    }
    if pi(w, id, 0x3c) != 0 {
        if w.svc.counters.times_completed == 0 {
            let m = w.mm(id);
            m.has_collision = false;
            m.mode |= 0x41;
            return;
        }
        let coll = w.classes.info(w.m(id).o_class).is_some_and(|c| c.has_collision);
        let m = w.mm(id);
        m.has_collision = coll;
        m.mode &= !0x41;
    }
    let st = w.m(id).state;
    match st {
        0 => {
            w.mm(id).update_dist = 0xff;
            create_arms(w, id);
            if pi(w, id, 0x38) != 0 || done {
                w.mm(id).state = 6;
            } else {
                w.mm(id).state = 5;
                hide(w, id);
            }
            place_arms(w, id);
        }
        6 | 1 => {
            if st == 6 {
                let g = w.m(id).group;
                if g != -1 { group_state(w, g, 1); }
                w.mm(id).state = 1;
            }
            let mission = pi(w, id, 8);
            if mission >= 0 && w.mission_done(level, mission as u8) != 0xff { return; }
            if pi(w, id, 0) == -1 || w.body() == 2 || w.hero.ground_moby != Some(id) || w.hero.air_ticks != 0 { return; }
            prompt(w, id);
            if !w.svc.interact.triangle() { return; }
            w.play_sound(0, 0, id);
            w.mm(id).state = 2;
            set_pi(w, id, 0x30, -1);
            set_pi(w, id, 0x28, 0);
            set_pi(w, id, 0x2c, 0);
            let (pos, yaw) = { let m = w.m(id); ([m.position[0], m.position[1], m.position[2]], m.rotation[2]) };
            w.hero_fields_mut().call(HeroCall::WalkTo { point: pos, yaw, release: 0 });
            let centre = w.m(id).position;
            attack::sphere_hit(w, 1.5, 20.0, 1.0, id, centre, 0x10000, 0, 1, 0);
        }
        2 => {
            let walk = match w.hero.state { 0x65 | 0x66 => 1, 0x67 => 2, _ => 0 };
            if pf(w, id, 0x20) < 1.0 {
                spring_arm(w, id, 1.0, 0x20, 0x28);
            } else if pf(w, id, 0x24) < 1.0 {
                spring_arm(w, id, 1.0, 0x24, 0x2c);
            } else if w.svc.interact.prompt.owner != owner::TELEPORTER || pf(w, id, 0x20) <= 0.25 {
                if walk == 2 {
                    set_pf(w, id, 0x34, BEAM_MAX);
                    w.mm(id).state = 3;
                } else if walk == 0 {
                    w.mm(id).state = 4;
                }
            }
            place_arms(w, id);
        }
        3 => {
            if pf(w, id, 0x34) != 0.0 {
                beam(w, id);
                let mut r = pf(w, id, 0x34);
                turn::approach(0.0, DT * 4.0, &mut r);
                set_pf(w, id, 0x34, r);
                beam_hides_hero(w, id);
                return;
            }
            teleport(w, id);
        }
        4 | 8 => {
            if pf(w, id, 0x34) < BEAM_MAX {
                beam(w, id);
                let mut r = pf(w, id, 0x34);
                turn::approach(BEAM_MAX, DT * 4.0, &mut r);
                set_pf(w, id, 0x34, r);
                beam_hides_hero(w, id);
            } else if 0.0 < pf(w, id, 0x24) {
                spring_arm(w, id, 0.0, 0x24, 0x28);
            } else if pf(w, id, 0x20) <= 0.0 {
                w.mm(id).state = 1;
            } else {
                spring_arm(w, id, 0.0, 0x20, 0x2c);
            }
            place_arms(w, id);
        }
        7 => {
            if pf(w, id, 0x20) < 1.0 {
                spring_arm(w, id, 1.0, 0x20, 0x28);
            } else if pf(w, id, 0x24) < 1.0 {
                spring_arm(w, id, 1.0, 0x24, 0x2c);
            } else {
                w.play_sound(1, 0, id);
                w.mm(id).state = 8;
                set_pf(w, id, 0x34, 0.0);
            }
            place_arms(w, id);
        }
        _ => {}
    }
}

/// State 0's three `CreateMoby(315)` arms (module table; the same lines in Pokitaru's pads 318). A failed create
/// leaves the arm word 0 (the game would write through the null pointer).
pub(crate) fn create_arms(w: &mut World, id: MobyId) {
    let hero = w.hero_moby.map(|h| (w.m(h).light, w.m(h).ambient));
    for k in 0..3 {
        let Some(a) = w.create_moby(ARM_CLASS) else {
            set_pi(w, id, 0x14 + 4 * k, 0);
            continue;
        };
        let (pad_mode, pos, rot) = { let m = w.m(id); (m.mode, m.position, m.rotation) };
        let am = w.mm(a);
        am.draw_dist = 0x40;
        am.visible = 1;
        if let Some((light, ambient)) = hero {
            am.light = light;
            am.ambient = ambient;
        }
        am.mode = pad_mode;
        am.position = pos;
        am.rotation = rot;
        am.rotation[2] = add_rot(am.rotation[2], k as f32 * 2.094_395_2);
        set_pi(w, id, 0x14 + 4 * k, a as i32 + 1);
    }
}

/// State 3 with the beam closed: Ratchet sent to the partner pad (module table).
fn teleport(w: &mut World, id: MobyId) {
    let Some(partner) = usize::try_from(pi(w, id, 0)).ok().filter(|&q| q < w.table.mobys.len()) else { return };
    let (pp, rot) = { let m = w.m(partner); (m.position, m.rotation) };
    crate::cinematic::hero_teleport(w, [pp[0], pp[1], pp[2] + 0.2], [rot[0], rot[1], rot[2]], 0, true);
    w.hero_fields_mut().hero_hidden = Some(1);
    let (track, stinger) = (pi(w, id, 0x40), pi(w, id, 0x44));
    if track != -1 && stinger != -1 {
        if let Some(s) = w.sound.as_deref_mut() { s.music_request(track as i16, stinger as i16); }
    }
    w.mm(partner).state = 4;
    if w.m(partner).pvars.len() >= 0x38 {
        set_pi(w, partner, 0x34, 0);
        set_pi(w, partner, 0x28, 0);
        set_pi(w, partner, 0x2c, 0);
        set_pf(w, partner, 0x24, 1.0);
        set_pf(w, partner, 0x20, 1.0);
    }
    w.play_sound(1, 0, partner);
    w.mm(id).state = 1;
    for o in [0x20, 0x24, 0x28, 0x2c] { set_pi(w, id, o, 0); }
    set_pf(w, id, 0x34, BEAM_MAX);
    place_arms(w, id);
}

/// `FUN_00308b28`: the spawn id's death bits (persistent and this visit's), then the pad's group to state 1 (or the
/// pad itself without a group). A negative spawn id would index before the bit arrays in the game; no pad has one.
fn activate(w: &mut World, id: MobyId) {
    let sid = w.m(id).spawn_id;
    if sid >= 0 {
        let level = w.svc.level;
        w.svc.save.death.insert((level, sid));
        w.svc.save.death_level.insert(sid);
    }
    let g = w.m(id).group;
    if g == -1 { w.mm(id).state = 1; } else { group_state(w, g, 1); }
}

/// `FUN_00309838`: the pad and its arms shown, the pad's collision on.
fn show(w: &mut World, id: MobyId) {
    let pv = w.m(id).pvars.clone();
    let coll = w.classes.info(w.m(id).o_class).is_some_and(|c| c.has_collision);
    let m = w.mm(id);
    m.has_collision = coll;
    m.visible = 1;
    m.mode &= !mode::HIDDEN;
    for k in 0..3 {
        if let Some(a) = arm(&pv, k).filter(|&a| a < w.table.mobys.len()) {
            let am = w.mm(a);
            am.mode &= !mode::HIDDEN;
            am.visible = 1;
        }
    }
}

/// `FUN_003097e8`: pad `+0x31 = 0`, mode |= 1, `+0x94 = 0`; each arm mode |= 1, `+0x31 = 0`.
fn hide(w: &mut World, id: MobyId) {
    let pv = w.m(id).pvars.clone();
    let m = w.mm(id);
    m.visible = 0;
    m.mode |= mode::HIDDEN;
    m.has_collision = false;
    for k in 0..3 {
        if let Some(a) = arm(&pv, k).filter(|&a| a < w.table.mobys.len()) {
            let am = w.mm(a);
            am.mode |= mode::HIDDEN;
            am.visible = 0;
        }
    }
}

/// `FUN_00309430`: the level's prompt text for the pad (owner 4), none on a level without a table.
fn prompt(w: &mut World, id: MobyId) {
    let k = pi(w, id, 0x10);
    let at = |t: &[i32], base: i32| usize::try_from(base + k).ok().and_then(|i| t.get(i)).copied();
    let msg = match w.svc.level {
        1 | 7 | 13 => at(&PROMPT_ABS, 0),
        17 => at(&PROMPT_ABS, 6),
        8 => at(&PROMPT_GP, 0),
        12 => at(&PROMPT_GP, 2),
        16 => at(&PROMPT_GP, 4),
        18 => at(&PROMPT_GP, 6),
        _ => return,
    };
    // An index past the tables reads the words after them in the game [L: no pad on the disc has one].
    if let Some(msg) = msg { w.svc.interact.try_prompt(owner::TELEPORTER, msg); }
}

/// `FUN_003092d0`: the three arms around the pad (module table).
pub(crate) fn place_arms(w: &mut World, id: MobyId) {
    let pv = w.m(id).pvars.clone();
    let (spread, tilt) = (p::ff(&pv, 0x20), p::ff(&pv, 0x24));
    let pad = w.m(id).position;
    for k in 0..3 {
        let Some(a) = arm(&pv, k).filter(|&a| a < w.table.mobys.len()) else { continue };
        let am = w.mm(a);
        am.position = pad;
        am.position[2] += f32::from_bits(0xbecc_cccd); // gp−0x4c88: −0.4
        am.rotation[1] = -20.0 * DEG; // gp−0x4c80: −20 (degrees)
        let d = spread * 2.4 - 1.2;
        let r0 = rc_formats::moby_light::rotation_rows([am.rotation[0], am.rotation[1], am.rotation[2]])[0].map(f32::from_bits);
        let l = (r0[0] * r0[0] + r0[1] * r0[1] + r0[2] * r0[2]).sqrt();
        if l != 0.0 {
            for (c, r) in am.position.iter_mut().zip(r0).take(3) { *c += r * (d / l); }
        }
        am.rotation[1] = tilt * -std::f32::consts::FRAC_PI_3 - 20.0 * DEG;
        w.build_matrix(a);
    }
}

/// `FUN_003094f0`'s quads for pad `id` (the beam of radius P[13] in the pad's frame; module table).
pub fn beam_quads(table: &MobyTable, _svc: &Services, id: MobyId) -> Option<FxQuads> { beam_quads_at(table, id, 0x34) }

/// The beam quads of the pad `id` with its radius at pvar `off` (1135: +0x34; Pokitaru's 318: +0x3c, the same draw
/// code and constants, level11 `0x2f2d58`).
#[allow(clippy::approx_constant)] // the code's own 6.28318 and 3.14159 (not 2π and π).
pub fn beam_quads_at(table: &MobyTable, id: MobyId, off: usize) -> Option<FxQuads> {
    let m = table.mobys.get(id)?;
    if m.pvars.len() < off + 4 { return None; }
    let r = p::ff(&m.pvars, off);
    let h = if r < 1.15 { (2.0 * (1.15 - r)).clamp(0.0, 1.0) + 0.1 } else { 0.1 };
    let rows = m.rows;
    let world = |l: [f32; 3]| -> [f32; 3] { std::array::from_fn(|k| rows[0][k] * l[0] + rows[1][k] * l[1] + rows[2][k] * l[2] + m.position[k]) };
    let quads = (0..32)
        .map(|i| {
            let corners = std::array::from_fn(|u| {
                let a = (i + u / 2) as f32 * 6.28318 * 0.03125 - 3.14159;
                let z = if u % 2 == 0 { BEAM_Z - h } else { BEAM_Z + h };
                world([a.cos() * r, a.sin() * r, z + 0.0])
            });
            FxQuad { corners, st: BEAM_ST, rgba: [BEAM_RGBA; 4] }
        })
        .collect();
    Some(FxQuads { fx: BEAM_FX, additive: true, subtract: false, quads })
}
