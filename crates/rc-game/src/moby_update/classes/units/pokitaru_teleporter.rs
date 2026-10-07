//! U359: Pokitaru's teleporter pads 318 (level11 `0x2f2518`, 5 placed), with the show `0x2f3040`, the prompt
//! `0x2f2cd0` and the beam callback `0x2f2d58`. Their own code, a sibling of the teleporter pads 1135
//! (`classes::teleporter`): the same arms (class 315, the arm placement `0x2f2b70` is level01 `0x3092d0` by masked
//! identity), the same spring, beam and hero walk / teleport; it differs in the pvar layout, a mission-gated
//! visibility and second partner, its own prompt (owner 0xb) that needs the lease for △, no activation cuboid, no
//! challenge gate, no group and no state 6 / 7 / 8. Every constant equals the 1135 copy's (read from the level11
//! overlay: spring gp−0x52f0.. = 12, 12, 32; beam FX 0x13, ALPHA 0, 2, 0, 1, RGBA 0x60408080, base z 1.0, ST
//! 0x1d3530; arm offsets −0.4, −20°).
//!
//! Pvars (s32 words P[k]): P[0] the partner, P[1] the second partner (−1 none), P[2] the mission that shows the pad
//! (−1 none), P[3] the mission that switches to the second partner, P[4] the prompt kind, P[5..8] the arms (port:
//! moby index + 1), P[8] / P[9] spread / tilt, P[10] / P[11] their spring velocities, P[12] / P[13] the arrival's music
//! track and stinger, P[15] +0x3c the beam radius.
//!
//! | address | what | here |
//! |---|---|---|
//! | state 0 | state 1; three arms (as 1135's state 0, `teleporter::create_arms`); mission P[2] ≥ 0 not done → pad +0x31 = 0, +0x94 = 0, mode \|= 1, and each arm mode \|= 1, +0x31 = 0, +0x94 = 0; arm placement | [`update`] |
//! | state 1 | partner P[0] ≠ −1: hidden (mode & 1), P[2] ≥ 0 and its mission done → `0x2f3040`; Ratchet's ground moby 0x13f64c = the pad and 0x13f65e = 0 → the prompt `0x2f2cd0`; △ and the prompt owner `0x15f594` = 0xb → `PlayClassSound(0, 0, pad)`, state 2, P[10] = P[11] = 0, `0x25a918(yaw, pos, 0)` (= L01 `0x249580`: the walk to the pad), `0x27f830(1.5, 20, 1, pad, pos, 0x10000, 0, 1, 0)` (= `0x26e830`: the sphere hit) | [`update`] |
//! | 0x2f2cd0 | `try_set_help_message(0xb, msg)`: P[4] = 0 → 0x2b0d, 1 / 2 / 4 → 0x2b0c, 3 → 0x2b0b, else 0 | [`prompt`] |
//! | 0x2f3040 | pad +0x31 = 1, mode &= ~1, +0x94 = class collision; each arm ≠ 0: mode &= ~1, +0x31 = 1, +0x94 = its class collision | [`show`] |
//! | state 2 | Ratchet's walk (`0x25a968` = `0x2495d0`); spread < 1 → spring it (P[10]) to 1; else tilt < 1 → spring it (P[11]) to 1; else, unless the owner is 0xb and spread > 0.25: standing → P[15] = 1.95, state 3; not walking → state 4; arm placement | [`update`] |
//! | state 3 | P[15] ≠ 0: `RegisterDrawCallback(0x2f2d58)`, `Approach(0, 4·dt, P[15])`, return | [`update`] (`Callback::UnitQuads`) |
//! | state 3 | P[15] = 0: the partner P[0], or P[1] when P[1] ≠ −1 and mission P[3] is done; `HeroTeleport(partner + 0.2 z, its Euler, 0, 1)` (`0x246398`), 0x1413f5 = 1, P[12] ≠ −1 and P[13] ≠ −1 → `MusicRequestTrack(P[12], P[13])`; `0x2f3040(partner)`; partner state 4, its +0x3c = +0x28 = +0x2c = 0, +0x24 = +0x20 = 1; `PlayClassSound(1, 0, partner)`; state 1, P[8..12] = 0, P[15] = 1.95; arm placement | [`update`] |
//! | state 4 | Ratchet's z (0x13f3d8) < 138 → global flag 0x13d3dd (0x13d388 + 0x55) = 1; P[15] < 1.95: the beam, `Approach(1.95, 4·dt, P[15])`; else tilt > 0 → spring it (P[10]) to 0; else spread ≤ 0 → state 1; else spring spread (P[11]) to 0; arm placement | [`update`] (`interact::set_global_flag`) |
//! | 0x2f2d58 | the 1135 beam on P[15]; below 1.15: 0x1413f5 = (r < 0.65), in every state | [`update`], `teleporter::beam_quads_at` |

use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::classes::teleporter::{self as tp, place_arms, spring_arm};
use crate::moby_update::creature::{attack, turn, DT};
use crate::moby_update::interact::set_global_flag;
use crate::moby_update::services::{pvar as p, HeroCall, Services, World};

pub const REFERENCE_LEVEL: u32 = 11;
pub const UPDATE_FN: u32 = 0x2f_2518;
/// The beam draw callback.
pub const BEAM_FN: u32 = 0x2f_2d58;
pub const CLASSES: [i16; 1] = [318];

/// The prompt lease owner of these pads (0xb).
pub const OWNER: i32 = 0xb;
/// The beam radius word.
const BEAM: usize = 0x3c;
const BEAM_MAX: f32 = 1.95;
/// `0x13d3dd − 0x13d388`.
const FLAG: usize = 0x55;

fn pf(w: &World, id: MobyId, o: usize) -> f32 { p::ff(&w.m(id).pvars, o) }
fn set_pf(w: &mut World, id: MobyId, o: usize, v: f32) { p::set_ff(&mut w.mm(id).pvars, o, v) }
fn pi(w: &World, id: MobyId, o: usize) -> i32 { p::i32(&w.m(id).pvars, o) }
fn set_pi(w: &mut World, id: MobyId, o: usize, v: i32) { p::set_i32(&mut w.mm(id).pvars, o, v) }

fn mission_done(w: &World, m: i32) -> bool { w.mission_done(w.svc.level, m as u8) == 0xff }

/// The beam's registration and its 0x1413f5 store (no state test in this copy).
fn beam(w: &mut World, id: MobyId) {
    if let Some(i) = super::row(REFERENCE_LEVEL, BEAM_FN) { w.svc.draw_callbacks.register(Callback::UnitQuads(i), id); }
}
fn beam_hides_hero(w: &mut World, id: MobyId) {
    let r = pf(w, id, BEAM);
    if r < 1.15 { w.hero_fields_mut().hero_hidden = Some((r < 0.65) as u8); }
}

/// Level11 `0x2f2518`.
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x40 { return; }
    match w.m(id).state {
        0 => {
            w.mm(id).state = 1;
            tp::create_arms(w, id);
            let m2 = pi(w, id, 8);
            if m2 >= 0 && !mission_done(w, m2) {
                let pv = w.m(id).pvars.clone();
                let m = w.mm(id);
                m.visible = 0;
                m.has_collision = false;
                m.mode |= mode::HIDDEN;
                for k in 0..3 {
                    if let Some(a) = tp::arm(&pv, k).filter(|&a| a < w.table.mobys.len()) {
                        let am = w.mm(a);
                        am.mode |= mode::HIDDEN;
                        am.visible = 0;
                        am.has_collision = false;
                    }
                }
            }
            place_arms(w, id);
        }
        1 => {
            if pi(w, id, 0) == -1 { return; }
            let m2 = pi(w, id, 8);
            if w.m(id).mode & mode::HIDDEN != 0 && m2 >= 0 && mission_done(w, m2) { show(w, id); }
            if w.hero.ground_moby != Some(id) || w.hero.air_ticks != 0 { return; }
            prompt(w, id);
            if !w.svc.interact.triangle() || w.svc.interact.prompt.owner != OWNER { return; }
            w.play_sound(0, 0, id);
            w.mm(id).state = 2;
            set_pi(w, id, 0x2c, 0);
            set_pi(w, id, 0x28, 0);
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
            } else if w.svc.interact.prompt.owner != OWNER || pf(w, id, 0x20) <= 0.25 {
                if walk == 2 {
                    set_pf(w, id, BEAM, BEAM_MAX);
                    w.mm(id).state = 3;
                } else if walk == 0 {
                    w.mm(id).state = 4;
                }
            }
            place_arms(w, id);
        }
        3 => {
            if pf(w, id, BEAM) != 0.0 {
                beam(w, id);
                let mut r = pf(w, id, BEAM);
                turn::approach(0.0, DT * 4.0, &mut r);
                set_pf(w, id, BEAM, r);
                beam_hides_hero(w, id);
                return;
            }
            teleport(w, id);
        }
        4 => {
            if crate::moby_update::classes::units::hero_pos(w)[2] < 138.0 { set_global_flag(w, FLAG, 1); }
            if pf(w, id, BEAM) < BEAM_MAX {
                beam(w, id);
                let mut r = pf(w, id, BEAM);
                turn::approach(BEAM_MAX, DT * 4.0, &mut r);
                set_pf(w, id, BEAM, r);
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
        _ => {}
    }
}

/// State 3 with the beam closed (module table).
fn teleport(w: &mut World, id: MobyId) {
    let (p0, p1, m3) = (pi(w, id, 0), pi(w, id, 4), pi(w, id, 0xc));
    let target = if p1 != -1 && mission_done(w, m3) { p1 } else { p0 };
    let Some(partner) = usize::try_from(target).ok().filter(|&q| q < w.table.mobys.len()) else { return };
    let (pp, rot) = { let m = w.m(partner); (m.position, m.rotation) };
    crate::cinematic::hero_teleport(w, [pp[0], pp[1], pp[2] + 0.2], [rot[0], rot[1], rot[2]], 0, true);
    w.hero_fields_mut().hero_hidden = Some(1);
    let (track, stinger) = (pi(w, id, 0x30), pi(w, id, 0x34));
    if track != -1 && stinger != -1 {
        if let Some(s) = w.sound.as_deref_mut() { s.music_request(track as i16, stinger as i16); }
    }
    show(w, partner);
    w.mm(partner).state = 4;
    if w.m(partner).pvars.len() >= 0x40 {
        set_pi(w, partner, BEAM, 0);
        set_pi(w, partner, 0x28, 0);
        set_pi(w, partner, 0x2c, 0);
        set_pf(w, partner, 0x24, 1.0);
        set_pf(w, partner, 0x20, 1.0);
    }
    w.play_sound(1, 0, partner);
    w.mm(id).state = 1;
    for o in [0x20, 0x24, 0x28, 0x2c] { set_pi(w, id, o, 0); }
    set_pf(w, id, BEAM, BEAM_MAX);
    place_arms(w, id);
}

/// `0x2f3040`: the pad and its arms shown, with their classes' collision.
fn show(w: &mut World, id: MobyId) {
    let pv = w.m(id).pvars.clone();
    let coll = |w: &World, i: MobyId| w.classes.info(w.m(i).o_class).is_some_and(|c| c.has_collision);
    let c = coll(w, id);
    let m = w.mm(id);
    m.visible = 1;
    m.mode &= !mode::HIDDEN;
    m.has_collision = c;
    for k in 0..3 {
        if let Some(a) = tp::arm(&pv, k).filter(|&a| a < w.table.mobys.len()) {
            let c = coll(w, a);
            let am = w.mm(a);
            am.mode &= !mode::HIDDEN;
            am.visible = 1;
            am.has_collision = c;
        }
    }
}

/// `0x2f2cd0`: the prompt (owner 0xb) by P[4].
fn prompt(w: &mut World, id: MobyId) {
    let msg = match pi(w, id, 0x10) { 0 => 0x2b0d, 1 | 2 | 4 => 0x2b0c, 3 => 0x2b0b, _ => 0 };
    w.svc.interact.try_prompt(OWNER, msg);
}

/// The beam quads of `0x2f2d58` (the 1135 beam on +0x3c).
pub fn beam_quads(table: &MobyTable, _svc: &Services, id: MobyId) -> Option<crate::moby_update::classes::units::FxQuads> { tp::beam_quads_at(table, id, BEAM) }
