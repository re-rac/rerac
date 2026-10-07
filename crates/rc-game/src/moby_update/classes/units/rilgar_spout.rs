//! Rilgar's water spouts, class 855 (level 05, 7 created instances): level05 0x3150f0 (census U191; was U186 / U189,
//! an override until the voice handoff was ported, G-AUD-010). A spout of its group starts when its linked moby's
//! command byte is set (the whole group goes to state 2, `0x26e0e0(group, 2)`); a spout whose mission is done stays
//! off. Running, it throws three type-50 droplets a tick along its x row (30 in 100 small bright ones, the rest
//! large dark), falling 10 units/s² to the water of its ripple patch, and, once the stream has had time to reach the
//! water, a type-46 ring where it lands (one in two ticks per droplet); the group shares one loop voice (class sound 0,
//! flags 0xd), held by the running spout nearest the camera. A spout whose mouth is under the water stops. Read
//! from the level05 decomp and disassembly of 0x3150f0 (the droplet velocity runs along row 0; the fall time is a
//! VU square root) and the overlay's data (gp−0x4d68 .. −0x4d44). Native `f32`; the `rand` draws in the game's order.
//!
//! **Pvar block**: +0x04 s32 the ripple patch (its record +0x08: the water height), +0x08 s32 the linked moby, +0x0c
//! the group's voice-slot word (an offset into the pvar shared data), +0x10 s32 the ticks running.
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x3150f0 | no pvar block → nothing | [`update`] |
//! | state 0 | the mission byte (`0x14c050 + level·16 + +0xb0`) ≠ −1 → 1; else → 3 (off) | [`update`] (`MissionState::mission_done`; +0xb0 = 0xff reads another byte in the game: [L] not done) |
//! | state 1 | link ≥ 0 and its +0xbc ≠ 0 → `0x283498(group, 2)` (L01 0x26e0e0) | [`update`] (`scheduler::group_state`) |
//! | state 2 | ticks += 1; `vec_distance`(position, camera 0x1671c0) < 64: slot < 0, its owner (`0x13e5d8 + slot·0x70`) null or not class 0x357 → slot = `PlayClassSound(0, 0xd, m)`, slot ≥ 0 → its position = position + unit(row 0)·2.5; owner = m → nothing; owner in state 2 and not farther than m → nothing; else the slot's position (as above) and owner = m | [`update`] (`World::play_sound`, `World::hand_over_sound`) |
//! | | water = patch +0x08 (`0x1612d0 + i·0x1190`); z < water → state 3 (the droplets of this tick still fly) | [`update`] (`water::RippleSim::patches`) |
//! | | ×3 (gp−0x4d48): small = `randf(0, 100)` ≤ 30 (gp−0x4d44); p = position + unit(row 1)·`randf(−0.2, 0.2)` (gp−0x4d4c); speed = the largest of three `randf(small ? 1.5 : 2.5, 4)` (gp−0x4d54 / −0x4d50 / −0x4d58); v = unit(row 0)·speed·dt | [`update`] |
//! | | small: k = 0.15, colour 0x60f0a0a0, p.z += (0.2 − 0.15) + `randf(−0.4, 0.4)`; large: k = 0.5, colour 0x20404030, p.z += (0.2 − 0.5) + 0.4 (gp−0x4d60 / −0x4d5c / −0x4d68 / −0x4d64) | [`update`] |
//! | | `0x29be70(k·randf(0.8, 1.2), water, 10·dt², p, v, colour, small)`: a type-50 droplet, texture frame `small` | [`update`] (`particles::type48::spawn50`) |
//! | | `randi(2)` ≠ 0 and t = √(2(z − water)/(10·dt²)) < ticks running: q = position + row 0·(t·4·dt) + (cos a, sin a)·`randf(0, 1.25)` (a = `rand_angle`), q.z = water + 0.05; `PartType46Spawn(randf(1, 1.5), randi(2) ? 2 : −2, q, 0, &water)` (0x29b830 = L01 0x286a68) | [`update`] (`particles::type46::spawn`; the water pointer as its value now [L]) |
//! | | no hit, light, flag | n/a |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, DT, DT2};
use crate::moby_update::scheduler::group_state;
use crate::moby_update::services::World;
use crate::particles::{type46, type48};

/// The update in the level05 class table.
pub const UPDATE_FN: u32 = 0x31_50f0;
pub const REFERENCE_LEVEL: u32 = 5;
pub const CLASSES: [i16; 1] = [855];
pub const DROPS: i32 = 3;
pub const SMALL_ODDS: f32 = 30.0;
pub const COLOURS: [u32; 2] = [0x60f0_a0a0, 0x2040_4030];
pub const SIZES: [f32; 2] = [0.15, 0.5];
pub const SPEED_LO: [f32; 2] = [1.5, 2.5];
pub const SPEED_HI: f32 = 4.0;
pub const SCATTER: f32 = 0.2;

fn water(w: &World, id: MobyId) -> f32 {
    let i = c::pi32(w, id, 4);
    let z = usize::try_from(i).ok().and_then(|i| w.svc.water.sim.as_ref().and_then(|s| s.patches.get(i))).map(|p| p.centre[2].to_f32());
    z.unwrap_or(f32::NEG_INFINITY)
}

/// Level05 0x3150f0 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x14 { return; }
    match w.m(id).state {
        0 => {
            let (m, level) = (w.m(id).mission, w.svc.level);
            w.mm(id).state = if w.mission_done(level, m) == 0xff { 3 } else { 1 };
        }
        1 => {
            let link = c::pi32(w, id, 8);
            if usize::try_from(link).ok().and_then(|l| w.table.mobys.get(l)).is_some_and(|m| m.cmd != 0) {
                let g = w.m(id).group;
                group_state(w, g, 2);
            }
        }
        2 => run(w, id),
        _ => {}
    }
}

fn run(w: &mut World, id: MobyId) {
    let n = c::pi32(w, id, 0x10) + 1;
    c::set_pi32(w, id, 0x10, n);
    let cam = w.camera_point();
    let cam = [cam[0], cam[1], cam[2], 0.0];
    let pos = w.m(id).position;
    let mouth = |w: &World| {
        let m = w.m(id);
        let p = c::add(m.position, c::set_len3(m.rows[0], 2.5));
        [p[0], p[1], p[2]]
    };
    if c::dist3(pos, cam) < 64.0 {
        let word = c::pi32(w, id, 0xc);
        let slot = w.svc.shared_i32(word);
        let owner = w.sound_owner(slot).filter(|&o| w.table.mobys.get(o).is_some_and(|m| m.o_class == CLASSES[0]));
        match owner {
            None => {
                let s = w.play_sound(0, 0xd, id);
                w.svc.set_shared_i32(word, s);
                if s >= 0 {
                    let p = mouth(w);
                    w.hand_over_sound(s, id, p);
                }
            }
            Some(o) if o == id => {}
            Some(o) => {
                let keep = w.m(o).state == 2 && c::dist3(w.m(o).position, cam) <= c::dist3(pos, cam);
                if !keep {
                    let p = mouth(w);
                    w.hand_over_sound(slot, id, p);
                }
            }
        }
    }
    let wl = water(w, id);
    if pos[2] < wl { w.mm(id).state = 3; }
    for _ in 0..DROPS {
        let small = w.rng.randf(0.0, 100.0) <= SMALL_ODDS;
        let k = if small { 0 } else { 1 };
        let (r0, r1) = (w.m(id).rows[0], w.m(id).rows[1]);
        let s = w.rng.randf(-SCATTER, SCATTER);
        let mut p = c::add(pos, c::set_len3(r1, s));
        let mut sp = w.rng.randf(SPEED_LO[k], SPEED_HI);
        let b = w.rng.randf(SPEED_LO[k], SPEED_HI);
        if sp < b { sp = b; }
        let b = w.rng.randf(SPEED_LO[k], SPEED_HI);
        if sp < b { sp = b; }
        let v = c::set_len3(r0, sp * DT);
        if small {
            let j = w.rng.randf(-0.4, 0.4);
            p[2] += (SCATTER - SIZES[0]) + j;
        } else {
            p[2] += (SCATTER - SIZES[1]) + 0.4;
        }
        let f = w.rng.randf(0.8, 1.2);
        *w.svc.fx.part_spawns.entry(type48::TYPE50).or_default() += 1;
        if let Some(sys) = w.particles.as_deref_mut() {
            if type48::spawn50(sys, w.rng, SIZES[k] * f, wl, DT2 * 10.0, p, v, COLOURS[k], small as usize).is_none() { w.svc.fx.part_failed += 1; }
        }
        if w.rng.randi(2) == 0 { continue; }
        let t = (((pos[2] - wl) * 2.0) / (DT2 * 10.0)).sqrt();
        if t.partial_cmp(&(n as f32)) != Some(std::cmp::Ordering::Less) { continue; }
        let a = w.rng.rand_angle();
        let rr = w.rng.randf(0.0, 1.25);
        let mut q = c::add(c::scale(r0, t * (DT * 4.0)), pos);
        q[0] += a.cos() * rr;
        q[1] += a.sin() * rr;
        q[2] = wl + 0.05;
        let size = w.rng.randf(1.0, 1.5);
        let spin = if w.rng.randi(2) != 0 { 2.0 } else { -2.0 };
        *w.svc.fx.part_spawns.entry(type46::TYPE).or_default() += 1;
        if let Some(sys) = w.particles.as_deref_mut() {
            if type46::spawn(sys, w.rng, size, spin, q, [0.0; 4], wl).is_none() { w.svc.fx.part_failed += 1; }
        }
    }
}
