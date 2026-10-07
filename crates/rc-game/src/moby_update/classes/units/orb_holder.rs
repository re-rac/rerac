//! The orb holders, class 1038, and their orbs, class 1040: level06 0x2f7288 / 0x2f7ab8, the same code on 10 and 17
//! (census U215; 21 created holders, each making one orb). A holder whose spawn id is not dead (on Orxon: also while
//! the mission of its mission byte is not done) creates a spinning orb 0.85 above itself and clears its death bits;
//! it passes the weapon hits it takes on to the orb, keeps a four-sprite glow behind the orb (the gold bolts' item
//! glow in the holder's own colours: `gold_bolt::item_glow_with`) and puts out a green spark every fourth tick while
//! drawn. A hit orb bursts into 40 type-60 sparks, plays its break sound and is deleted; the holder then sets its
//! death bits (the orb stays gone) and raises its command byte. The orb keeps its class loop (sound 0) playing. Read
//! from the level06 decomp (0x2f7288, 0x2f7548, 0x2f7628, 0x2f7ab8, 0x2f7c68); the data (gp−0x4da8.., level06
//! 0x161e58) are the same bytes on 10 (0x161c14) and 17 (0x161de4). Native `f32`; the `rand` draws in the game's
//! order.
//!
//! **Holder pvar block**: +0x00..+0x3c the glow block (angles, spins, timers, sizes: `gold_bolt::glow`), +0x40 the
//! orb (moby index). **Orb pvar block**: +0x00 its damage record (+0x20), +0x60 its loop's voice slot.
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x2f7288 state 0 | death bit (`0x14c190 + level·0x100`, spawn id +0xb2) clear, or level 10 with the mission byte (`0x14c0f0 + +0xb0`) not done: `CreateMoby(1040)` (+0x32 = 0x40, +0x30 = 0x40, +0x31 = 1, the holder's light / ambient, mode \| 0x5000, position + 0.85 z, three `random_angle_radians`), +0x40 = orb, state 1, the glow init, both death bits cleared (persistent and this visit's, 0x1bac50 = level01's 0x1ba950) | [`update`] (`gold_bolt::glow_init_with`, `Services::save`) |
//! | | else +0xbc = 1, state 2 | [`update`] |
//! | state 1 | `MobyGetHitMessage(m, 0x330000, 0)` → `0x26eaa8(1, orb, the hit's attacker, 0x10000, its point, its push)`, +0xa4 = 0xff | [`update`] (`attack::hit_moby`) |
//! | | orb deleted (state < 0) → +0xbc = 1, state 2, both death bits set | [`update`] |
//! | 0x2f7628 | the glow: four soft type-59 sprites (tex 0x35, life 2) from 0.3 behind the orb (the camera record, level06 0x167540), 0.1 apart; colours tween 0x40ff4040 → 0x10208080 | [`update`] (`gold_bolt::item_glow_with`) |
//! | | tick & 3 = 3 and drawn (+0x31): v = (0, 0, ±0.9) (− on tick & 4), velocity = unit(v)·(−2 / ticks(20)), p = holder + v + 0.85 z; `PartType60Spawn(0.5, p, vel, 0x30ff3040, trunc(ticks(20)), randi(0xff), 0)` | [`spark`] |
//! | 0x2f7ab8 | a weapon hit (0x330000): the burst 0x2f7c68, `PlayClassSound(1, 0, m)`, the loop's voice released when the orb owns it, slot −1, `DeleteMoby` | [`orb_update`] |
//! | state 0 | `0x2650d0(m, 0x80, 0x80, 0x80)` (ambient), scale = class scale × 3, +0x00 → the record at +0x20, state 1, mode \| 0x20, slot −1 | [`orb_update`] |
//! | state 1 | spin 580 / 180 / 140 °/s about x / y / z | [`orb_update`] |
//! | every state | `SoundIsAlive(m, slot)` no → slot = `PlayClassSound(0, 4, m)` (the loop) | [`orb_update`] |
//! | 0x2f7c68 | ×40: v = `rand_vec(dt, 2·dt)`, size `randf(0.5, 0.3)`, `randi(2)` picks the pair (0x50804040 → 0x40c02020 or 0x5040c0c0 → 0x40008080) tweened by `randf(0, 1)`, `PartType60Spawn(size, pos, v, colour, 20, randi(0xff), 0)` | [`burst`] |
//! | | no bolt (no `BoltBurst`), light, other moby | n/a |

use crate::moby_runtime::MobyId;
use crate::moby_update::classes::gold_bolt::{glow_init_with, item_glow_with};
use crate::moby_update::creature::{attack, pi32, set_pi32, DT};
use crate::moby_update::services::World;

/// The updates in the level06 class table.
pub const UPDATE_FN: u32 = 0x2f_7288;
pub const ORB_FN: u32 = 0x2f_7ab8;
pub const REFERENCE_LEVEL: u32 = 6;
pub const CLASSES: [i16; 1] = [1038];
pub const ORB: i16 = 1040;
pub const ORB_CLASSES: [i16; 1] = [ORB];

/// The glow's spins, sizes and colours (0x2f7548; level06 0x161e58 / 0x161e5c).
pub const SPINS: [f32; 4] = [-2.0, -4.25, 2.25, 4.5];
pub const SIZES: [f32; 4] = [1.75, 2.25, 2.25, 1.75];
pub const GLOW: (u32, u32) = (0x40ff_4040, 0x1020_8080);
/// The spark (0x161e68..0x161e70): colour, size, life (ticks).
pub const SPARK_RGBA: u32 = 0x30ff_3040;
pub const SPARK_SIZE: f32 = 0.5;
pub const SPARK_LIFE: i32 = 20;
/// The orb's spin (°/s, 0x161e74..) and scale (0x161e80).
pub const SPIN: [f32; 3] = [580.0, 180.0, 140.0];
pub const ORB_SCALE: f32 = 3.0;
/// The burst (0x161e84..0x161eac): velocity range (·dt), life, the two colour pairs, sizes.
pub const BURST_V: (f32, f32) = (1.0, 2.0);
pub const BURST_LIFE: u16 = 20;
pub const BURST_A: (u32, u32) = (0x5040_c0c0, 0x4000_8080);
pub const BURST_B: (u32, u32) = (0x5080_4040, 0x40c0_2020);
pub const BURST_SIZE: (f32, f32) = (0.5, 0.3);

/// Pvar offsets.
pub mod pv {
    pub const ORB: usize = 0x40;
    pub const SLOT: usize = 0x60;
}

fn dead(w: &World, id: MobyId) -> bool {
    let b2 = w.m(id).spawn_id;
    b2 >= 0 && w.svc.save.death.contains(&(w.svc.level, b2))
}

fn set_dead(w: &mut World, id: MobyId, on: bool) {
    let (b2, lvl) = (w.m(id).spawn_id, w.svc.level);
    if b2 < 0 { return; }
    if on {
        w.svc.save.death.insert((lvl, b2));
        w.svc.save.death_level.insert(b2);
    } else {
        w.svc.save.death.remove(&(lvl, b2));
        w.svc.save.death_level.remove(&b2);
    }
}

/// Level06 0x2f7288 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x44 { return; }
    match w.m(id).state {
        0 => {
            let orxon = w.svc.level == 10 && w.mission_done(10, w.m(id).mission) != 0xff;
            if dead(w, id) && !orxon {
                let m = w.mm(id);
                m.cmd = 1;
                m.state = 2;
                return;
            }
            let Some(o) = w.create_moby(ORB) else { return };
            let (light, ambient, md, pos) = { let m = w.m(id); (m.light, m.ambient, m.mode, m.position) };
            let r: [f32; 3] = std::array::from_fn(|_| w.rng.rand_angle());
            let m = w.mm(o);
            m.draw_dist = 0x40;
            m.update_dist = 0x40;
            m.visible = 1;
            m.light = light;
            m.ambient = ambient;
            m.mode = md | 0x5000;
            m.position = pos;
            m.rotation = [r[0], r[1], r[2], m.rotation[3]];
            m.position[2] += 0.85;
            set_pi32(w, id, pv::ORB, o as i32);
            w.mm(id).state = 1;
            glow_init_with(w, id, 0, SPINS, SIZES);
            set_dead(w, id, false);
        }
        1 => {
            let orb = pi32(w, id, pv::ORB) as usize;
            if let Some(h) = w.get_hit(id, 0x33_0000, false) {
                let f = |v: [crate::ps2v::Pf; 4]| v.map(|x| f32::from_bits(x.0));
                attack::hit_moby(w, orb, h.attacker.unwrap_or(id), 1.0, 0x1_0000, f(h.pos), f(h.dir));
                w.mm(id).hit_slot = 0xff;
            }
            if w.table.mobys.get(orb).is_none_or(|m| (m.state as i8) < 0) {
                let m = w.mm(id);
                m.cmd = 1;
                m.state = 2;
                set_dead(w, id, true);
            } else {
                let c = w.m(orb).position;
                item_glow_with(w, id, 0, [c[0], c[1], c[2]], 1.0, true, GLOW);
                spark(w, id);
            }
        }
        _ => {}
    }
}

/// The green spark of every fourth tick (module doc).
pub fn spark(w: &mut World, id: MobyId) {
    if w.counter & 3 != 3 || w.m(id).visible == 0 { return; }
    let s = if w.counter & 4 != 0 { -1.0 } else { 1.0 };
    let v = [0.0, 0.0, s * 0.9, 0.0];
    let n = w.ticks(SPARK_LIFE);
    let vel = [0.0, 0.0, s * (-2.0 / n as f32), 0.0];
    let q = w.m(id).position;
    let p = [q[0] + v[0], q[1] + v[1], q[2] + v[2] + 0.85, q[3] + v[3]];
    let life = w.ticks(SPARK_LIFE);
    let rot = w.rng.randi(0xff) as u8;
    w.part60(SPARK_SIZE, p, vel, SPARK_RGBA, life as u16, rot, 0);
}

/// Level06 0x2f7ab8: the orb (module doc).
pub fn orb_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x64 { return; }
    if w.get_hit(id, 0x33_0000, false).is_some() {
        burst(w, id);
        w.play_sound(1, 0, id);
        let slot = pi32(w, id, pv::SLOT);
        if slot != -1 && w.sound_owner(slot) == Some(id) { w.release_sound(slot, id); }
        set_pi32(w, id, pv::SLOT, -1);
        w.delete_moby(id);
        return;
    }
    let st = w.m(id).state;
    if st == 0 {
        let oc = w.m(id).o_class;
        let s = super::class_scale(w, oc) * ORB_SCALE;
        let m = w.mm(id);
        m.ambient = [0x80, 0x80, 0x80, m.ambient[3]];
        m.scale = s;
        m.state = 1;
        m.mode |= 0x20;
        set_pi32(w, id, 0, 0x20);
        set_pi32(w, id, pv::SLOT, -1);
    }
    if st <= 1 {
        let m = w.mm(id);
        for (r, s) in m.rotation.iter_mut().zip(SPIN) { *r = crate::moby_update::creature::add_rot(*r, s * 0.017453292 * DT); }
    }
    let slot = pi32(w, id, pv::SLOT);
    if !w.sound_alive(slot, id) {
        let s = w.play_sound(0, 4, id);
        set_pi32(w, id, pv::SLOT, s);
    }
}

/// Level06 0x2f7c68: the orb's 40-spark burst.
pub fn burst(w: &mut World, id: MobyId) {
    let p = w.m(id).position;
    for _ in 0..40 {
        let v = w.rng.rand_vec(BURST_V.0 * DT, BURST_V.1 * DT);
        let size = w.rng.randf(BURST_SIZE.0, BURST_SIZE.1);
        let pair = if w.rng.randi(2) == 0 { BURST_B } else { BURST_A };
        let t = w.rng.randf(0.0, 1.0);
        let c = crate::hud::tween_color(t, pair.0, pair.1);
        let rot = w.rng.randi(0xff) as u8;
        w.part60(size, p, [v[0], v[1], v[2], 0.0], c, BURST_LIFE, rot, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::services::HitTemplate;

    #[test]
    fn holder_makes_an_orb_passes_hits_and_marks_its_death_bit() {
        let holder = Moby { o_class: 1038, spawn_id: 5, pvars: vec![0; 0x44], ..Moby::default() };
        let mut t = MobyTable::new(vec![holder], 8);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.level = 6;
        svc.save.death.insert((6, 5));
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 3);
        // Dead: no orb, command byte 1.
        update(&mut w, 0);
        assert_eq!((w.m(0).state, w.m(0).cmd), (2, 1));
        // Alive: the orb 0.85 above, the bits cleared.
        w.mm(0).state = 0;
        w.svc.save.death.clear();
        update(&mut w, 0);
        let o = pi32(&w, 0, pv::ORB) as usize;
        assert_eq!((w.m(0).state, w.m(o).o_class, w.m(o).position[2]), (1, ORB, 0.85));
        w.mm(o).pvars = vec![0; 0x80];
        orb_update(&mut w, o);
        assert_eq!(w.m(o).state, 1);
        assert!(w.svc.sounds.iter().any(|s| (s.moby, s.index, s.flags) == (o, 0, 4)), "the orb's loop");
        // A weapon hit on the holder reaches the orb, which bursts and goes; the holder then marks itself dead.
        w.deliver_hit(0, &HitTemplate { flags: 0x10_0000, ..Default::default() });
        update(&mut w, 0);
        assert_eq!(w.m(0).hit_slot, 0xff);
        let r = w.svc.hits.records[w.m(o).hit_slot as usize];
        assert_eq!((r.target, r.flags), (o, 0x1_0000));
        w.mm(o).hit_slot = 0xff;
        w.deliver_hit(o, &HitTemplate { flags: 0x10_0000, ..Default::default() });
        orb_update(&mut w, o);
        assert!(w.m(o).state >= 0x80, "deleted");
        assert!(w.svc.sounds.iter().any(|s| (s.moby, s.index) == (o, 1)), "the break sound");
        update(&mut w, 0);
        assert_eq!((w.m(0).state, w.m(0).cmd), (2, 1));
        assert!(w.svc.save.death.contains(&(6, 5)) && w.svc.save.death_level.contains(&5));
    }
}
