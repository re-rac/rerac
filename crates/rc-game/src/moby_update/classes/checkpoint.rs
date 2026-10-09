//! Checkpoint triggers, class 805: `CheckpointTriggerUpdate` level01 0x300220 and the checkpoint record
//! `FUN_0029ac10` (the respawn point the death reload `0x29adc8` puts the hero on). Spec: docs/plan/triggers.md
//! §4 ("rising edge, re-armed on exit"), docs/plan/hero_states.md P2. Native `f32`.
//!
//! Pvar block (P, s32 words): P[0] the cuboid (`PointInCuboid`), P[2] flags (8 off; 2 Clank only; 4 Ratchet or
//! the disguise only; 0x10 only in the groups 0 / 1 / 9), P[3] taken, P[4] may be taken again, P[5] inside last
//! tick, P[6] the mission that must be done first (−1: none). `moby+0xbc`: initialised.
//!
//! Per tick: the first update only clears P[5]. Then, gated by the flags and P[6], the hero's feet 0x13f3d0 are
//! tested against the cuboid; entering it (or the checkpoint's spawn id already collected / dead, e.g. reached
//! another way) takes it — unless taken and not repeatable: P[3] = 1, `SetMissionDone(+0xb0)`, the record, and
//! its own spawn id's killed / collected bytes and death bits cleared. Otherwise P[3] = 0. Last, state 0 → 1 once
//! its mission is done.
//!
//! **The record `0x29ac10(point, euler)`**: the point is the checkpoint's position with z on the ground below
//! it (`GroundHeight(0.5)`) unless that is more than 2 away; the Euler angles are the checkpoint's (+0x40).
//! Every spawn id this visit killed (`0x1baea4[id] = mission + 2`) whose mission is done (or 1: no mission)
//! becomes collected (`0x1bbb04`) with both death bits set; the record goes to [`SaveBits::checkpoint`]
//! (0x1bb6b0.., [`Record`]).
//!
//! `SetMissionDone(+0xb0)` is [`crate::cinematic::set_mission_done`] (the engine applies it after the tick, so the
//! state 0 → 1 step below sees it on the next tick; nothing reads that state).
//!
//! The record also keeps (`0x29ac10`'s tail): the visit state 0x1baaa0 → 0x1bb700, Ratchet's moby's light word and
//! ambient (+0x38 / +0x3c; +0x80 is not modelled), the body (0x1413f4, 0x14161c), the reverb request and the music track
//! (0x1bc304: the pending track 0x1516f2, else the current 0x151708); the death reload's `0x29adc8` puts them back.
//!
//! [`SaveBits::checkpoint`]: crate::moby_update::services::SaveBits::checkpoint

use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pvar as p, World};
use crate::moby_update::triggers::point_in_cuboid;
use crate::ps2v::Pf;

/// The checkpoint update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x300220;

/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [805];

/// The checkpoint record (0x1bb6b0 = 1, position 0x1bb6c0, Euler 0x1bb6d0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    pub pos: [f32; 3],
    pub rot: [f32; 3],
}

pub fn update(w: &mut World, id: MobyId) {
    let flags = p::u32(&w.m(id).pvars, 8);
    if flags & 8 != 0 { return; }
    if w.m(id).cmd == 0 {
        let m = w.mm(id);
        m.cmd = 1;
        p::set_i32(&mut m.pvars, 0x14, 0);
        return;
    }
    let level = w.svc.level;
    let gate = p::i32(&w.m(id).pvars, 0x18);
    if gate != -1 && w.mission_done(level, gate as u8) != 0xff { return; }
    let (mode, group) = (w.body(), w.hero.group);
    if flags & 2 != 0 && mode != 1 { return; }
    if flags & 4 != 0 && mode != 3 && mode != 0 { return; }
    if flags & 0x10 != 0 && 1 < group as u32 && group != 9 { return; }
    let feet = crate::hero::physics::to_f32x3(w.hero.pos);
    let inside = point_in_cuboid(&w.svc.volumes, feet, p::i32(&w.m(id).pvars, 0));
    let mut take = false;
    {
        let pv = &mut w.mm(id).pvars;
        if inside {
            take = p::i32(pv, 0x14) == 0;
            p::set_i32(pv, 0x14, 1);
        } else {
            p::set_i32(pv, 0x14, 0);
        }
    }
    let sid = w.m(id).spawn_id;
    if !take {
        let collected = w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0);
        if collected || w.svc.save.death.contains(&(level, sid)) {
            take = true;
        } else {
            p::set_i32(&mut w.mm(id).pvars, 0xc, 0);
        }
    }
    if take {
        let pv = &w.m(id).pvars;
        if !(p::i32(pv, 0xc) != 0 && p::i32(pv, 0x10) == 0) {
            p::set_i32(&mut w.mm(id).pvars, 0xc, 1);
            let mission = w.m(id).mission;
            crate::cinematic::set_mission_done(w, mission);
            let m = w.m(id);
            let (pos, rot) = ([m.position[0], m.position[1], m.position[2]], [m.rotation[0], m.rotation[1], m.rotation[2]]);
            let g = w.ground_height(Pf::b(0x3f00_0000), [Pf::f(pos[0]), Pf::f(pos[1]), Pf::f(pos[2]), Pf::f(m.position[3])], 0).to_f32();
            let z = if 2.0 < (pos[2] - g).abs() { pos[2] } else { g };
            record(w, Record { pos: [pos[0], pos[1], z], rot });
            let s = &mut w.svc.save;
            s.killed.remove(&sid);
            s.collected.remove(&sid);
            s.death.remove(&(level, sid));
            s.death_level.remove(&sid);
        }
    }
    if w.m(id).state == 0 && w.mission_done(level, w.m(id).mission) == 0xff { w.mm(id).state = 1; }
}

/// `FUN_0029ac10(point, euler)`: this visit's kills whose mission is done become permanent, and the record.
pub(crate) fn record(w: &mut World, r: Record) {
    let level = w.svc.level;
    let kills: Vec<(i16, u8)> = w.svc.save.killed.iter().map(|(&k, &v)| (k, v)).collect();
    for (sid, b) in kills {
        if !(0..0x7ff).contains(&sid) || b == 0 { continue; }
        if b == 1 || w.mission_done(level, b.wrapping_sub(2)) == 0xff {
            let s = &mut w.svc.save;
            s.collected.insert(sid, b);
            s.death_level.insert(sid);
            s.death.insert((level, sid));
        }
    }
    w.svc.save.checkpoint = Some(r);
    // 0x1baaa0 → 0x1bb700: the visit records the reload writes back (crate::moby_update::visit).
    w.svc.save.checkpoint_visit = w.svc.save.visit.clone();
    // 0x1bb6f4 = the body 0x1413f4, 0x1bb6fc = 0x14161c (crate::hero::bodies; the reload switches back into the body).
    // The game's reload switches inside `0x29adc8`, before any checkpoint updates; the port makes that switch on the
    // next tick ([`crate::hero::bodies::Bodies::restore`]), after this moby pass: a checkpoint the hero respawns in
    // keeps the pending body, else a second death would come back as Ratchet.
    w.svc.save.checkpoint_body = match w.hero.bodies.restore {
        Some((body, state, _)) => (body, state),
        None => (w.body(), w.hero.bodies.state_param),
    };
    // 0x1bb6e0 / 0x1bb6e4: Ratchet's moby's light word and ambient.
    w.svc.save.checkpoint_light = w.hero_moby.and_then(|h| w.table.mobys.get(h)).map(|m| (m.light, m.ambient));
    // 0x1bb6ec..0x1bb6f2: the reverb request saved with the record (crate::audio::reverb), and 0x1bc304 the music
    // track (0x1516f2 pending, else 0x151708: `AudioSystem::checkpoint_saved`).
    if let Some(s) = w.sound.as_deref_mut() { s.checkpoint_saved(); }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::cinematic::EngineRequest;
    use crate::moby_runtime::{Moby, MobyTable};

    /// A cuboid of half-size 2 about `c`.
    pub(crate) fn cube(c: [f32; 3], euler: [f32; 3]) -> rc_formats::volumes::Shape {
        rc_formats::volumes::Shape {
            matrix: [[2.0, 0.0, 0.0, 0.0], [0.0, 2.0, 0.0, 0.0], [0.0, 0.0, 2.0, 0.0], [c[0], c[1], c[2], 1.0]],
            inverse: [[0.5, 0.0, 0.0, 0.0], [0.0, 0.5, 0.0, 0.0], [0.0, 0.0, 0.5, 0.0]],
            euler,
            ..Default::default()
        }
    }

    /// Entering the cuboid takes the checkpoint once: `SetMissionDone(+0xb0)` (the one mission writer,
    /// `crate::cinematic::set_mission_done`) and the record; staying inside does not take it again.
    #[test]
    fn taking_sets_its_mission_once() {
        let at = [10.0, 20.0, 5.0];
        let mut m = Moby { o_class: 805, mission: 3, spawn_id: 9, position: [at[0], at[1], at[2], 1.0], pvars: vec![0; 0x1c], ..Moby::default() };
        p::set_i32(&mut m.pvars, 0x18, -1);
        let mut t = MobyTable::new(vec![m], 4);
        let mut hero = crate::hero::Hero::new();
        hero.pos = [Pf::f(at[0]), Pf::f(at[1]), Pf::f(at[2]), Pf::ONE];
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.volumes = std::sync::Arc::new(rc_formats::volumes::Volumes { cuboids: vec![cube(at, [0.0; 3])], ..Default::default() });
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        update(&mut w, 0);
        assert!(w.svc.cinematic.requests.is_empty() && w.svc.save.checkpoint.is_none(), "the first update only arms it");
        for _ in 0..3 { update(&mut w, 0); }
        let done: Vec<_> = w.svc.cinematic.requests.iter().filter(|r| matches!(r, EngineRequest::MissionDone { .. })).collect();
        assert_eq!(done, [&EngineRequest::MissionDone { mission: 3 }]);
        assert!(w.svc.save.checkpoint.is_some());
        assert_eq!(w.svc.fx.unported.get("checkpoint 805 SetMissionDone"), None);
    }

    /// `SetMissionDone` writes 0x14c050 at once, so the record right after it promotes this visit's kills of the
    /// checkpoint's own mission ("never again", 0x1bbb04); a kill of a mission still open stays a visit kill.
    /// (Kerwan: a nanotech crate of mission 1 broken before the train station's checkpoint stays gone once it is taken.)
    #[test]
    fn taking_promotes_the_kills_of_its_own_mission() {
        let at = [10.0, 20.0, 5.0];
        let mut m = Moby { o_class: 805, mission: 1, spawn_id: 9, position: [at[0], at[1], at[2], 1.0], pvars: vec![0; 0x1c], ..Moby::default() };
        p::set_i32(&mut m.pvars, 0x18, -1);
        let mut t = MobyTable::new(vec![m], 4);
        let mut hero = crate::hero::Hero::new();
        hero.pos = [Pf::f(at[0]), Pf::f(at[1]), Pf::f(at[2]), Pf::ONE];
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.level = 3;
        svc.volumes = std::sync::Arc::new(rc_formats::volumes::Volumes { cuboids: vec![cube(at, [0.0; 3])], ..Default::default() });
        // A crate of mission 1 and an enemy of mission 2, both broken / killed this visit (0x1baea4 = mission + 2).
        svc.save.killed.insert(42, 1 + 2);
        svc.save.killed.insert(43, 2 + 2);
        let missions = crate::moby_update::services::LevelMissions::fresh_load(3, [0; 16]);
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        w.missions = &missions;
        for _ in 0..2 { update(&mut w, 0); }
        assert!(w.svc.save.checkpoint.is_some());
        assert_eq!(w.svc.save.collected.get(&42), Some(&3), "the kill of the checkpoint's own mission is promoted");
        assert_eq!(w.svc.save.collected.get(&43), None, "a kill of another, open mission is not");
    }
}
