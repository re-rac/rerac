//! Floor switches, class 830: level05 0x30c5b0, the same code on levels 5, 11, 15, 17, 18 (30 placed instances, 25 on
//! Rilgar). A plate Ratchet steps on once: its glow pulses until he stands on it, then it turns green, plays its
//! sound, records itself as done (its spawn id's killed / collected bytes, so it stays pressed on a revisit) and
//! **opens a path**: every point of its path whose w equals the switch's key has its w cleared (the path table
//! `0x1b0930`, which the level's path followers read). Read from the level05 decomp
//! (level05 0x30c5b0). Native `f32`.
//!
//! **Pvar block** (P, s32 words): P[0] the path (−1: none), P[1] f32 the key, P[2] ≠ 0: it starts pressed once its
//! mission (+0xb0) is done, P[3] the glow phase.
//!
//! **0** init: P[3] = `rand_angle()`, z −= 0.35; unless its spawn id is collected or its level death bit set, or P[2]
//! and its mission byte is not done: → **1** armed; else → **2** pressed (glow 0x80208020, +0xbc = 2, the path opened).
//! **1** armed: P[3] += 2π·dt, the glow grey `v = clamp(trunc((4·sin P[3] − 3)·128), 0x20, 0x80)` (0x80vvvvvv); when
//! Ratchet stands on it (ground moby 0x13f64c, air ticks 0x13f65e = 0): killed[spawn id] = mission + 2 (and
//! collected when its mission is none or done), +0xbc = 1, → **2**, glow 0x80208020, `PlayClassSound(0, 0)`, the
//! path opened.

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{add_rot, DT};
use crate::moby_update::services::{pvar as p, World};
use std::f32::consts::TAU;

/// The update address in the level05 class table (the reference overlay: [`REFERENCE_LEVEL`]).
pub const UPDATE_FN: u32 = 0x30c5b0;
pub const REFERENCE_LEVEL: u32 = 5;
pub const CLASSES: [i16; 1] = [830];

/// The pressed glow (green).
pub const PRESSED_GLOW: u32 = 0x8020_8020;
/// The init's drop onto the floor.
pub const DROP: f32 = 0.35;

/// Clears the w of every point of path `path` whose w equals `key` (both of the port's views of the table: the
/// splines the path followers read and the trigger volumes' paths).
pub fn open_path(w: &mut World, path: i32, key: f32) {
    let Ok(i) = usize::try_from(path) else { return };
    if let Some(pts) = w.svc.splines.get_mut(i) {
        for q in pts.iter_mut() { if f32::from_bits(q[3]) == key { q[3] = 0; } }
    }
    if w.svc.volumes.paths.get(i).is_some_and(|pts| pts.iter().any(|q| q[3] == key)) {
        let v = std::sync::Arc::make_mut(&mut w.svc.volumes);
        for q in v.paths[i].iter_mut() { if q[3] == key { q[3] = 0.0; } }
    }
}

/// Level05 0x30c5b0 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x10 { return; }
    let level = w.svc.level;
    match w.m(id).state {
        0 => {
            let a = w.rng.rand_angle();
            p::set_ff(&mut w.mm(id).pvars, 0xc, a);
            w.mm(id).position[2] -= DROP;
            let (sid, ms) = (w.m(id).spawn_id, w.m(id).mission);
            let gated = p::i32(&w.m(id).pvars, 8) != 0;
            let done = w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(level, sid));
            if !done && (!gated || w.mission_done(level, ms) != 0xff) {
                w.mm(id).state = 1;
            } else {
                let m = w.mm(id);
                m.state = 2;
                m.glow = PRESSED_GLOW;
                m.cmd = 2;
                let (path, key) = (p::i32(&m.pvars, 0), p::ff(&m.pvars, 4));
                open_path(w, path, key);
            }
        }
        1 => {
            let ph = add_rot(p::ff(&w.m(id).pvars, 0xc), DT * TAU);
            p::set_ff(&mut w.mm(id).pvars, 0xc, ph);
            let v = (((ph.sin() * 4.0 - 3.0) * 128.0) as i32).clamp(0x20, 0x80) as u32;
            w.mm(id).glow = v << 16 | v << 8 | 0x8000_0000 | v;
            let h = w.hero;
            if h.ground_moby == Some(id) && h.air_ticks == 0 { press(w, id); }
        }
        _ => {}
    }
}

/// Level05 `0x30c928(switch)` (the same code as level00 `0x2d5830` and level18 `0x2e1880`, cluster 5f30809d27b0): the
/// press, when the switch is armed (state 1): killed[spawn id] = mission + 2 (and collected when its mission is none
/// or done), +0xbc = 1, → 2, the green glow, `PlayClassSound(0, 0)`, the path opened. Ratchet standing on it and
/// (level 00 / 18) a knocked Veldin creature 749 landing within 1 of it press it
/// ([`crate::moby_update::classes::units::horny_toad`]).
pub fn press(w: &mut World, id: MobyId) {
    if w.m(id).state != 1 || w.m(id).pvars.len() < 8 { return; }
    let level = w.svc.level;
    let (sid, ms) = (w.m(id).spawn_id, w.m(id).mission);
    w.svc.save.killed.insert(sid, ms.wrapping_add(2));
    if ms == 0xff || (w.missions.mission_slot(ms) != 0xff && w.mission_done(level, ms) == 0xff) {
        w.svc.save.collected.insert(sid, ms.wrapping_add(2));
    }
    let m = w.mm(id);
    m.cmd = 1;
    m.state = 2;
    m.glow = PRESSED_GLOW;
    w.play_sound(0, 0, id);
    let (path, key) = (p::i32(&w.m(id).pvars, 0), p::ff(&w.m(id).pvars, 4));
    open_path(w, path, key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};

    #[test]
    fn press_opens_the_path() {
        let mut m = Moby { o_class: 830, mission: 0xff, spawn_id: 7, position: [0.0, 0.0, 1.0, 1.0], pvars: vec![0; 16], ..Moby::default() };
        p::set_i32(&mut m.pvars, 0, 0);
        p::set_ff(&mut m.pvars, 4, 3.0);
        let mut t = MobyTable::new(vec![m], 4);
        let mut hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.set_splines(&[vec![[0.0, 0.0, 0.0, 3.0], [1.0, 0.0, 0.0, 2.0], [2.0, 0.0, 0.0, 3.0]]]);
        {
            let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
            update(&mut w, 0);
            assert_eq!((w.m(0).state, w.m(0).position[2]), (1, 1.0 - DROP));
            update(&mut w, 0);
            assert_eq!(w.m(0).glow >> 24, 0x80);
        }
        hero.ground_moby = Some(0);
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 1);
        update(&mut w, 0);
        assert_eq!((w.m(0).state, w.m(0).cmd, w.m(0).glow), (2, 1, PRESSED_GLOW));
        assert_eq!(w.svc.save.collected.get(&7), Some(&1));
        let ws: Vec<f32> = w.svc.splines[0].iter().map(|q| f32::from_bits(q[3])).collect();
        assert_eq!(ws, [0.0, 2.0, 0.0]);
    }
}
