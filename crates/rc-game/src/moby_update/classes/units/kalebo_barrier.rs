//! Kalebo's energy barriers: the switches 546, the barrier posts 552 and the invisible walls 1387, level16 0x2cf198 /
//! 0x2cf4a8 / 0x2e36e8 (census U502 (6), U503 (20), U514 (4); one family: the posts read the switches of their moby
//! group, the switches turn a wall's collision). Each group's posts pair up by height (|Δz| < 0.1); one post per
//! group runs (the others get mode 2), and while no switch of the group is thrown it keeps the group's loop (class
//! sound 0) playing and registers the beams: four stacked additive strips (FX 0x13) from each post to its partner. A
//! weapon hit throws a switch (its glow from a pale pulse to green, sound 0, command byte 1, its wall's collision
//! off); a switch with a re-arm time (+0x64 > 0) flips back after it, blinking faster in its last 90 ticks. A switch
//! whose mission is done (or with no re-arm, −1, and its mission byte done) starts thrown. Read from the level16
//! decomp (0x2cf198, 0x2e3770, 0x2cf4a8, 0x2cf5c8, 0x2cf678, 0x2cf738, 0x2cf7a8, 0x2cf850, 0x2e36e8). Native `f32`.
//!
//! **Switch pvar**: +0x30 f32 (0.001), +0x60 the wall (moby index, −1 none), +0x64 the re-arm time (ticks; −1 none),
//! +0x68 the glow phase, +0x6c the re-arm timer. **Post pvar**: +0x00 the partner (the port: moby index + 1, 0 none),
//! +0x04 the tick its beam was last drawn, +0x08 on, +0x0c the loop's voice slot. **Wall pvar**: +0x00 / +0x04 the
//! y / z sizes (×0.25).
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x2cf198 state 0 | phase = `random_angle_radians`, +0x30 = 0.001; mission byte (`0x14c050 + level·16 + +0xb0`) not done or +0x64 ≠ −1 → state 1; else +0xbc = 1, state 2, the wall's collision off | [`switch_update`] |
//! | state 1 | phase += τ·dt; c = clamp(trunc((4 sin − 3)·128), 0x20, 0x80); glow +0x90 = 0x80cccccc (c in r, g, b) | [`switch_update`] |
//! | | `MobyGetHitMessage(m, 0x330000, 0)` → state 2, glow 0x80208020, +0xbc = 1, phase π/2, `PlayClassSound(0, 0, m)`, the wall's collision off (0x2e3770), +0x64 > 0 → +0x6c = ticks(+0x64) | [`switch_update`] |
//! | state 2 | glow = trunc((sin/2 + ½)·128) clamped 0x20..0x80 in green, 0x80200020; +0x64 ≥ 1: `FastDecTimer(+0x6c)`: running → below ticks(90) phase += 8π·dt; done → +0xbc = 0, state 1, phase π/2, the wall's collision on | [`switch_update`] |
//! | every state | +0xa4 = 0xff (the hit message dropped) | [`switch_update`] |
//! | 0x2e3770 | a 1387: +0x94 = on ? class +0x10 : 0 | [`set_wall`] |
//! | 0x2cf4a8 | mode & 2 → nothing (the posts paired off) | [`post_update`] (the scheduler skips mode 2) |
//! | state 0 | partner 0 → every 552 of the group: mode \| 2 unless it is this post, partner = the first other 552 of the group with \|Δz\| < 0.1 (0x2cf678); state 1 | [`post_update`] |
//! | state 1 | a 546 of the group with +0xbc ≠ 0 (0x2cf738) → on = 0, the loop released when this post owns it, slot −1 | [`post_update`] |
//! | | else on = 1, `RegisterDrawCallback(0x2cf7a8)` (list 1), `SoundIsAlive(m, slot)` no → slot = `PlayClassSound(0, 4, m)` | [`post_update`] (`Callback::UnitQuads`) |
//! | 0x2cf7a8 / 0x2cf850 | every 552 of the group with a partner, not drawn this tick: both marked, rows = (d, (0, 0, 1) × d, (0, 0, 1)), d = partner − post, point = post + 0.25 z (the corners have y = 0: row 1 adds nothing); quads (0, 0, ∓0.05) → (1, 0, ∓0.05) (0x1d3170), ST 0x1d31b0, RGBA 0x60d02020, FX 0x13, ALPHA 0x48, four of them 0.666 apart | [`fx_quads`] + `rc-engine` fx_draw |
//! | 0x2e36e8 state 0 | rows 1 / 2 = unit · +0x00·¼ / +0x04·¼, state 1, mode \| 0x101, `MobyBuildMatrix` | [`wall_update`] |
//! | | no particle, light, save flag, bolt | n/a |

use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{add_rot, dec_timer_pvar_i32, pf, pi32, set_pf, set_pi32, DT};
use crate::moby_update::services::{Services, World};

use super::{FxQuad, FxQuads};

pub const REFERENCE_LEVEL: u32 = 16;
/// The switches 546.
pub const SWITCH_FN: u32 = 0x2c_f198;
pub const SWITCH: i16 = 546;
pub const SWITCH_CLASSES: [i16; 1] = [SWITCH];
/// The posts 552 (the unit row whose `Callback::UnitQuads` draws the beams).
pub const UPDATE_FN: u32 = 0x2c_f4a8;
pub const POST: i16 = 552;
pub const CLASSES: [i16; 1] = [POST];
/// The walls 1387.
pub const WALL_FN: u32 = 0x2e_36e8;
pub const WALL: i16 = 1387;
pub const WALL_CLASSES: [i16; 1] = [WALL];

/// gp−0x5208..: the beam's height, step, FX texture, colour.
pub const BEAM_Z: f32 = 0.25;
pub const BEAM_STEP: f32 = f32::from_bits(0x3f2a_7efa);
pub const FX: usize = 0x13;
pub const BEAM_RGBA: u32 = 0x60d0_2020;
/// 0x1d3170 (x from post to partner, z the strip's half height) and 0x1d31b0.
pub const HALF: f32 = 0.05;
pub const ST: [[f32; 2]; 4] = [[0.5, 0.0], [0.5, 1.0], [0.5, 0.0], [0.5, 1.0]];

const HALF_PI: u32 = 0x3fc9_0fdb;

fn members(w: &World, id: MobyId) -> Vec<MobyId> {
    w.svc.groups.lists.get(w.m(id).group as u8 as usize).and_then(|l| l.as_ref()).map(|l| l.iter().map(|&e| (e & 0x7fff) as usize).collect()).unwrap_or_default()
}

/// Level16 0x2e3770(moby, on): a wall 1387's collision on (the class's blob) or off.
pub fn set_wall(w: &mut World, wall: i32, on: bool) {
    let Ok(k) = usize::try_from(wall) else { return };
    if w.table.mobys.get(k).is_none_or(|m| m.o_class != WALL) { return; }
    let c = on && super::class_collision(w, WALL);
    w.mm(k).has_collision = c;
}

fn clamp_c(x: f32) -> u32 { (x as i32).clamp(0x20, 0x80) as u32 }

/// Level16 0x2cf198: the switch (module doc).
pub fn switch_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x70 { return; }
    let wall = pi32(w, id, 0x60);
    match w.m(id).state {
        0 => {
            let a = w.rng.rand_angle();
            set_pf(w, id, 0x68, a);
            set_pi32(w, id, 0x30, 0x3a83_126f);
            let (lvl, b0) = (w.svc.level, w.m(id).mission);
            if w.mission_done(lvl, b0) != 0xff || pi32(w, id, 0x64) != -1 {
                w.mm(id).state = 1;
            } else {
                let m = w.mm(id);
                m.cmd = 1;
                m.state = 2;
                set_wall(w, wall, false);
            }
        }
        1 => {
            let ph = add_rot(pf(w, id, 0x68), DT * f32::from_bits(0x40c9_0fdb));
            set_pf(w, id, 0x68, ph);
            let c = clamp_c((ph.sin() * 4.0 - 3.0) * 128.0);
            w.mm(id).glow = c << 16 | c << 8 | 0x8000_0000 | c;
            if w.get_hit(id, 0x33_0000, false).is_some() {
                let m = w.mm(id);
                m.state = 2;
                m.glow = 0x8020_8020;
                m.cmd = 1;
                set_pi32(w, id, 0x68, HALF_PI as i32);
                w.play_sound(0, 0, id);
                set_wall(w, wall, false);
                let n = pi32(w, id, 0x64);
                if 0 < n { let t = w.ticks(n); set_pi32(w, id, 0x6c, t); }
            }
        }
        2 => {
            let ph = pf(w, id, 0x68);
            let c = clamp_c((ph.sin() * 0.5 + 0.5) * 128.0);
            w.mm(id).glow = c << 8 | 0x8020_0020;
            if pi32(w, id, 0x64) >= 1 {
                if dec_timer_pvar_i32(w, id, 0x6c) == 0 {
                    if pi32(w, id, 0x6c) < w.ticks(0x5a) {
                        let ph = add_rot(ph, DT * f32::from_bits(0x41c9_0fdb));
                        set_pf(w, id, 0x68, ph);
                    }
                } else {
                    let m = w.mm(id);
                    m.cmd = 0;
                    m.state = 1;
                    set_pi32(w, id, 0x68, HALF_PI as i32);
                    set_wall(w, wall, true);
                }
            }
        }
        _ => {}
    }
    w.mm(id).hit_slot = 0xff;
}

/// Level16 0x2cf678: the first other post of `id`'s group level with it (|Δz| < 0.1).
fn partner(w: &World, id: MobyId) -> Option<MobyId> {
    let z = w.m(id).position[2];
    members(w, id).into_iter().find(|&k| k != id && w.table.mobys.get(k).is_some_and(|m| m.o_class == POST && (m.position[2] - z).abs() < 0.1))
}

/// Level16 0x2cf4a8: the post (module doc).
pub fn post_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x10 || w.m(id).mode & mode::NO_UPDATE != 0 { return; }
    match w.m(id).state {
        0 => {
            if pi32(w, id, 0) != 0 { return; }
            for k in members(w, id) {
                if w.table.mobys.get(k).is_none_or(|m| m.o_class != POST || m.pvars.len() < 0x10) { continue; }
                if k != id { w.mm(k).mode |= mode::NO_UPDATE; }
                let p = partner(w, k).map_or(0, |p| p as i32 + 1);
                set_pi32(w, k, 0, p);
            }
            w.mm(id).state = 1;
        }
        1 => {
            let thrown = members(w, id).into_iter().any(|k| w.table.mobys.get(k).is_some_and(|m| m.o_class == SWITCH && m.cmd != 0));
            if thrown {
                if pi32(w, id, 8) != 0 {
                    set_pi32(w, id, 8, 0);
                    let slot = pi32(w, id, 0xc);
                    if slot != -1 && w.sound_owner(slot) == Some(id) { w.release_sound(slot, id); }
                    set_pi32(w, id, 0xc, -1);
                }
            } else {
                set_pi32(w, id, 8, 1);
                if let Some(i) = super::row(REFERENCE_LEVEL, UPDATE_FN) { w.svc.draw_callbacks.register(Callback::UnitQuads(i), id); }
                let slot = pi32(w, id, 0xc);
                if !w.sound_alive(slot, id) {
                    let s = w.play_sound(0, 4, id);
                    set_pi32(w, id, 0xc, s);
                }
            }
        }
        _ => {}
    }
}

/// Level16 0x2cf7a8 for the post `id` (module doc): the beams of its group, one per pair. The "drawn this tick"
/// marks (+0x04) are the draw's own bookkeeping; the port recomputes the pairs per call (draw only).
pub fn fx_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Option<FxQuads> {
    let me = table.mobys.get(id)?;
    let list = svc.groups.lists.get(me.group as u8 as usize)?.as_ref()?;
    let mut drawn = std::collections::HashSet::new();
    let mut quads = Vec::new();
    for &e in list {
        let a = (e & 0x7fff) as usize;
        let Some(m) = table.mobys.get(a) else { continue };
        if m.o_class != POST || m.pvars.len() < 0x10 || drawn.contains(&a) { continue; }
        let p = crate::moby_update::services::pvar::i32(&m.pvars, 0);
        if p == 0 { continue; }
        let b = (p - 1) as usize;
        let Some(q) = table.mobys.get(b) else { continue };
        drawn.insert(a);
        drawn.insert(b);
        let d = [q.position[0] - m.position[0], q.position[1] - m.position[1], q.position[2] - m.position[2]];
        for k in 0..4 {
            let o = [m.position[0], m.position[1], m.position[2] + BEAM_Z + BEAM_STEP * k as f32];
            let at = |x: f32, z: f32| -> [f32; 3] { std::array::from_fn(|i| o[i] + d[i] * x + if i == 2 { z } else { 0.0 }) };
            quads.push(FxQuad { corners: [at(0.0, -HALF), at(0.0, HALF), at(1.0, -HALF), at(1.0, HALF)], st: ST, rgba: [BEAM_RGBA; 4] });
        }
    }
    Some(FxQuads { fx: FX, additive: true, subtract: false, quads })
}

/// Level16 0x2e36e8: the wall (module doc).
pub fn wall_update(w: &mut World, id: MobyId) {
    if w.m(id).state != 0 || w.m(id).pvars.len() < 8 { return; }
    let (sy, sz) = (pf(w, id, 0) * 0.25, pf(w, id, 4) * 0.25);
    let m = w.mm(id);
    for (k, s) in [(1, sy), (2, sz)] {
        let r = m.rows[k];
        let n = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        if n != 0.0 { for (x, y) in m.rows[k].iter_mut().zip(r).take(3) { *x = y * s / n; } }
    }
    m.state = 1;
    m.mode |= 0x101;
    w.build_matrix(id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::scheduler::Groups;
    use crate::moby_update::services::HitTemplate;

    #[test]
    fn a_thrown_switch_stops_the_beams_and_re_arms() {
        let mut sw = Moby { o_class: SWITCH, group: 0, mission: 0xff, pvars: vec![0; 0x70], ..Moby::default() };
        crate::moby_update::services::pvar::set_i32(&mut sw.pvars, 0x60, 3);
        crate::moby_update::services::pvar::set_i32(&mut sw.pvars, 0x64, 60);
        let post = |x: f32| { let mut m = Moby { o_class: POST, group: 0, pvars: vec![0; 0x10], ..Moby::default() }; m.position = [x, 0.0, 1.0, 1.0]; crate::moby_update::services::pvar::set_i32(&mut m.pvars, 0xc, -1); m };
        let wall = Moby { o_class: WALL, has_collision: true, ..Moby::default() };
        let mut t = MobyTable::new(vec![sw, post(0.0), post(4.0), wall], 8);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = Services::new();
        svc.groups = Groups { lists: vec![Some(vec![0, 1, 0x8002])] };
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 1);
        switch_update(&mut w, 0);
        assert_eq!(w.m(0).state, 1, "re-arming switch starts armed");
        post_update(&mut w, 1);
        assert_eq!((pi32(&w, 1, 0), pi32(&w, 2, 0), w.m(2).mode & mode::NO_UPDATE), (3, 2, mode::NO_UPDATE));
        post_update(&mut w, 1);
        assert_eq!(w.svc.draw_callbacks.list1.len(), 1);
        assert!(w.svc.sounds.iter().any(|s| (s.moby, s.index, s.flags) == (1, 0, 4)));
        let q = fx_quads(w.table, w.svc, 1).unwrap();
        assert_eq!(q.quads.len(), 4, "one pair, four strips");
        assert_eq!(q.quads[0].corners[2], [4.0, 0.0, 1.0 + BEAM_Z - HALF]);
        // Thrown: the wall's collision off, the beams stop; re-armed after 60 ticks.
        w.deliver_hit(0, &HitTemplate { flags: 0x10_0000, ..Default::default() });
        switch_update(&mut w, 0);
        assert_eq!((w.m(0).state, w.m(0).cmd, w.m(0).glow, w.m(3).has_collision), (2, 1, 0x8020_8020, false));
        w.svc.draw_callbacks.list1.clear();
        post_update(&mut w, 1);
        assert!(w.svc.draw_callbacks.list1.is_empty() && pi32(&w, 1, 8) == 0);
        for _ in 0..60 { switch_update(&mut w, 0); }
        assert_eq!((w.m(0).state, w.m(0).cmd), (1, 0));
    }
}
