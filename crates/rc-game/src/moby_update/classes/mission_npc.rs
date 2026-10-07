//! Mission NPCs, classes 730 / 790: `MissionNpcUpdate` level01 0x2fad68 (Novalis only: clusters.tsv 2c0aee80). The
//! director of the Novalis story beats: the first-arrival scene, the drop-in cutaway, the wait for three kills, scene 3
//! → in-level movie 3 → scene 4, Kerwan unlocked and the hinged-bridge cutaway. Spec: docs/plan/cutscenes.md §4.
//! Native `f32`.
//!
//! Pvar block (P): +0x00 f32 drop height (30), +0x04 the drop-in camera trigger 737 (moby link, −1 none), +0x08 s16
//! timer (`FastDecTimer` every tick), +0x0a s16 the saved draw distance, +0x0c f32 the landing z (spawn z − 0.75),
//! +0x10 the drop-in cuboid, +0x14/+0x18/+0x1c the three mobys to kill (links), +0x30 the checkpoint cuboid, +0x34 /
//! +0x38 the hinged-bridge halves 746 (links), +0x3c the bridge's camera trigger 737 (link).
//!
//! | state | what |
//! |---|---|
//! | 0 (load pass) | update distance 0xff, landing z, collision off, hidden (mode |= 0x41), z += drop height |
//! | 1 | mission done → shown on its spot (anim 2) → 10. Else: game mode 0 and global flag 0x13d397 clear → `DialogStreamStart(5)` (the arrival scene), flag set; the ship hidden (`FUN_002a2450`) every tick; Ratchet's feet in the cuboid → shown, the drop-in trigger commanded for `ticks(420)` with its camera cuboid aimed at the NPC → 2 |
//! | 2 | drops at 5 u/s, the trigger's target Euler (+0x14 / +0x18) re-aimed every tick; lands (anim 1 hard cut) → 3 |
//! | 3 | anim 1 wrapped → anim 2 over 20 ticks; 730 → 10, 790 → 4 |
//! | 4 | the three are dead and Ratchet not in group 7 / 0x14 → timer 45 → 5 |
//! | 5 | timer out: hidden, `DialogStreamStart(3)` → 6 → (mode ≠ 2) `DialogStreamUpdate(3)`: movie `mpegs[5]` → 7 → `DialogStreamStart(4)` → 8 |
//! | 8 | `UnlockPlanet(3)` + banner, shown, the drop-in trigger's spawn id spent, the bridge trigger commanded for `ticks(180)` and the bridge halves +0xbc = 2 (they lower), `SetMissionDone`, checkpoint at +0x30, save, the ship shown → 10 |
//! | 10 | z = landing z |
//!
//! `FUN_002fac80` at the top: the big-head cheat on the scene actors of class 0x32b at 2.75
//! (`manip::scene_big_head`). The bridge halves' +0xbc are visit records (`FUN_0029b0a0`, `crate::moby_update::visit`):
//! restored after a death reload. `UnlockPlanet` / `ShowPlanetBanner` go through
//! `crate::cinematic` (the saved game's planet bits, the HUD banner); the save is logged.

use super::checkpoint::{self, Record};
use crate::cinematic::{self, EngineRequest};
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::services::{pvar as p, World};

/// The update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x2fad68;
/// Classes that run [`update`].
pub const CLASSES: [i16; 2] = [730, 790];
/// 0x2da: the NPC without a mission of its own (stops at state 10 after landing).
pub const NO_MISSION_CLASS: i16 = 0x2da;
/// The arrival scene and the mission's scenes / movie.
pub const ARRIVAL_SCENE: usize = 5;
pub const MISSION_SCENES: [usize; 2] = [3, 4];
pub const MISSION_MOVIE: i32 = 3;
/// The camera trigger holds (ticks before `ticks()`).
pub const DROP_IN_HOLD: i32 = 0x1a4;
pub const BRIDGE_HOLD: i32 = 0xb4;
/// `UnlockPlanet(3)`: Kerwan.
pub const PLANET: i32 = 3;

fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> {
    usize::try_from(p::i32(&w.m(id).pvars, o)).ok().filter(|&m| m < w.table.mobys.len())
}

fn shown(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.mode &= !(mode::HIDDEN | mode::NO_ANIM);
    m.has_collision = true;
}

/// FastArcTan(dist2, dz) / FastArcTan(dx, dy) from the camera cuboid of trigger `t` to moby `id`: (−pitch, yaw).
fn aim(w: &World, t: MobyId, id: MobyId) -> Option<(usize, f32, f32)> {
    let c = usize::try_from(p::i32(&w.m(t).pvars, 0x24)).ok()?;
    let cub = w.svc.volumes.cuboids.get(c)?;
    let (cc, m) = (cub.centre(), w.m(id).position);
    let d2 = ((cc[0] - m[0]).powi(2) + (cc[1] - m[1]).powi(2)).sqrt();
    let pitch = (m[2] - cc[2]).atan2(d2);
    let yaw = (m[1] - cc[1]).atan2(m[0] - cc[0]);
    Some((c, -pitch, yaw))
}

/// `MissionNpcUpdate` 0x2fad68 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x40 { return; }
    // FUN_002fac80: the actors' big-head cheat on the scene actors of class 0x32b at 2.75.
    crate::moby_update::manip::scene_big_head(w, &[0x32b], 0, f32::from_bits(0x4030_0000));
    // FastDecTimer__FRs(+0x08).
    {
        let pv = &mut w.mm(id).pvars;
        let t = p::i16(pv, 8);
        if t != 0 { p::set_i16(pv, 8, (t.max(1) - 1).max(0)); }
    }
    let level = w.svc.level;
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            let z0 = m.position[2];
            m.update_dist = 0xff;
            m.state = 1;
            p::set_i16(&mut m.pvars, 8, 0);
            p::set_ff(&mut m.pvars, 0xc, z0 - 0.75);
            m.has_collision = false;
            m.mode |= mode::HIDDEN | mode::NO_ANIM;
            m.position[2] = z0 + p::ff(&m.pvars, 0);
        }
        1 => {
            if w.mission_done(level, w.m(id).mission) == 0xff {
                if w.m(id).anim.seq_b != 2 {
                    let t = w.ticks(0x14);
                    w.anim_blend(id, 2, 0, t);
                }
                let m = w.mm(id);
                m.state = 10;
                m.draw_dist = 0xff;
                shown(w, id);
                return;
            }
            if w.svc.game_mode == 0 && !w.svc.cinematic.arrival_seen {
                cinematic::start_scene(w, ARRIVAL_SCENE, true);
                w.svc.cinematic.arrival_seen = true;
            }
            w.svc.cinematic.requests.push(EngineRequest::ShipHidden(true));
            let feet = crate::hero::physics::to_f32x3(w.hero.pos);
            if !w.in_cuboid(feet, p::i32(&w.m(id).pvars, 0x10)) { return; }
            shown(w, id);
            if let Some(t) = link(w, id, 4) {
                let hold = w.ticks(DROP_IN_HOLD);
                let tm = w.mm(t);
                if tm.pvars.len() >= 0x38 {
                    p::set_i32(&mut tm.pvars, 0x2c, hold);
                    tm.cmd = 1;
                    p::set_v4f(&mut tm.pvars, 0x10, [0.0; 4]);
                    if let Some((c, pitch, yaw)) = aim(w, t, id) {
                        let v = std::sync::Arc::make_mut(&mut w.svc.volumes);
                        v.cuboids[c].euler = [0.0, pitch, yaw];
                    }
                }
            }
            w.mm(id).state = 2;
        }
        2 => {
            let dz = crate::moby_update::creature::DT * 5.0;
            w.mm(id).position[2] -= dz;
            if let Some(t) = link(w, id, 4) {
                if let Some((_, pitch, yaw)) = aim(w, t, id) {
                    let tp = &mut w.mm(t).pvars;
                    if tp.len() >= 0x38 {
                        p::set_ff(tp, 0x14, pitch);
                        p::set_ff(tp, 0x18, yaw);
                    }
                }
            }
            let land = p::ff(&w.m(id).pvars, 0xc);
            if w.m(id).anim.seq_b == 0 && w.m(id).position[2] - land <= 0.0 {
                let o = w.m(id).o_class;
                if let Some(class) = w.classes.anim(o) {
                    let class = class.clone();
                    if rc_formats::moby_anim::hard_cut(&mut w.mm(id).anim, &class, 1, 0) { crate::moby_update::anim_sound::after_sequence_change(w.mm(id), &class); }
                }
            }
            if w.m(id).position[2] <= land {
                let m = w.mm(id);
                m.position[2] = land;
                m.state = 3;
            }
        }
        3 => {
            let a = w.m(id).anim;
            if a.seq_a != 1 || a.flags & 2 == 0 { return; }
            if a.seq_b != 2 {
                let t = w.ticks(0x14);
                w.anim_blend(id, 2, 0, t);
            }
            w.mm(id).state = if w.m(id).o_class != NO_MISSION_CLASS { 4 } else { 10 };
        }
        4 => {
            let alive = [0x14, 0x18, 0x1c].into_iter().filter_map(|o| link(w, id, o)).any(|k| !w.m(k).is_deleted());
            if alive || w.hero.group == 7 || w.hero.group == 0x14 { return; }
            let t = w.ticks(0x2d) as i16;
            let m = w.mm(id);
            p::set_i16(&mut m.pvars, 8, t);
            m.state = 5;
        }
        5 => {
            if p::i16(&w.m(id).pvars, 8) != 0 { return; }
            let m = w.mm(id);
            let dd = m.draw_dist;
            p::set_i16(&mut m.pvars, 0xa, dd);
            m.mode |= mode::HIDDEN | mode::NO_ANIM;
            m.state = 6;
            cinematic::start_scene(w, MISSION_SCENES[0], false);
        }
        6 => {
            if w.svc.game_mode == 2 { return; }
            cinematic::start_movie(w, MISSION_MOVIE);
            w.mm(id).state = 7;
        }
        7 => {
            if w.svc.game_mode == 2 { return; }
            cinematic::start_scene(w, MISSION_SCENES[1], false);
            w.mm(id).state = 8;
        }
        8 => {
            if w.svc.game_mode == 2 { return; }
            mission_done(w, id);
            w.mm(id).state = 10;
        }
        10 => {
            let land = p::ff(&w.m(id).pvars, 0xc);
            w.mm(id).position[2] = land;
        }
        _ => {}
    }
}

/// State 8 (module doc).
fn mission_done(w: &mut World, id: MobyId) {
    let level = w.svc.level;
    crate::cinematic::unlock_planet(w, PLANET);
    crate::cinematic::show_planet_banner(w, PLANET);
    let m = w.mm(id);
    m.mode &= !(mode::HIDDEN | mode::NO_ANIM);
    // The drop-in trigger's spawn id: killed this visit (mission + 2); spent at once without a mission of its own.
    if let Some(t) = link(w, id, 4) {
        let (sid, ms) = (w.m(t).spawn_id, w.m(t).mission);
        w.svc.save.killed.insert(sid, ms.wrapping_add(2));
        if ms == 0xff || w.mission_done(level, ms) == 0xff { w.svc.save.collected.insert(sid, ms.wrapping_add(2)); }
    }
    if let Some(t) = link(w, id, 0x3c) {
        let hold = w.ticks(BRIDGE_HOLD);
        let tm = w.mm(t);
        if tm.pvars.len() >= 0x38 {
            p::set_i32(&mut tm.pvars, 0x2c, hold);
            tm.cmd = 1;
        }
        if let (Some(a), Some(b)) = (link(w, id, 0x34), link(w, id, 0x38)) {
            w.mm(a).cmd = 2;
            w.mm(b).cmd = 2;
            // `0x29b0a0(half + 0xbc, 1, half, 1, 0x1baaa0)` for each: the lowered command survives a death reload.
            crate::moby_update::visit::record_field(w, a, a, 0xbc);
            crate::moby_update::visit::record_field(w, b, b, 0xbc);
        }
    }
    let mission = w.m(id).mission;
    crate::cinematic::set_mission_done(w, mission);
    let c = p::i32(&w.m(id).pvars, 0x30);
    if let Some(s) = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c).copied() {
        checkpoint::record(w, Record { pos: s.centre(), rot: s.euler });
    }
    w.svc.cinematic.requests.push(EngineRequest::Save);
    w.svc.cinematic.requests.push(EngineRequest::ShipHidden(false));
}
