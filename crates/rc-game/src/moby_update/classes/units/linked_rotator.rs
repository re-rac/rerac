//! Rotators driven by a linked moby's state, classes 127, 128, 159, 169: level13 0x2c7f38 (census U411; 24 created
//! instances). The turning counterpart of the sliding movers (`linked_mover`, U247: the same pvar layout for the
//! link, its two states and the two sounds, separate code): the rotator turns by its Euler step (degrees) over T
//! seconds from its placed rotation while the link is away or not in the "close" state B, and back while the link is
//! there and not in the "open" state A. A done mission unlinks it. Read from the level13 decomp (0x2c7f38). Native
//! `f32`.
//!
//! **Pvar block**: +0x00 the placed rotation (x, y, z), +0x0c the progress 0..1, +0x10 the link (runtime index, −1
//! none), +0x14 state A, +0x18 state B, +0x1c..+0x24 the Euler step (degrees), +0x2c T (seconds), +0x30 / +0x34 the
//! sounds (−1 none), +0x38 a scale factor, +0x3c start turned (≠ 0).
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x2c7f38 | mission byte (`0x14c050 + level·16`) done → link = −1 | [`update`] |
//! | state 0 | +0x00 = rotation, progress 0, → 1 (+0x3c = 0) or 2; 0 < +0x38 → scale = class scale (class +0x24) · +0x38 | [`update`] |
//! | state 1 | link there and not in A → wait; else sound +0x30, progress 0, → 2 | [`update`] |
//! | state 2 | link there and in B → sound +0x30, → 4; else rate = 1/(60·T): progress += rate, rotation += step·π/180·rate (`fast_add_rotations`); at 1: rotation = placed + step, progress 1, → 3, sound +0x30 when +0x34 ≠ −1 (sic) | [`update`] |
//! | state 3 | link there and in B → progress 1, sound +0x30, → 4 | [`update`] |
//! | state 4 | link away or in A → sound +0x30, → 2; else progress −= rate, rotation −= step·rate (`fast_subtract_rotations`); at 0: rotation = placed, progress 0, sound +0x34 when +0x30 ≠ −1 (sic), → 1 | [`update`] |
//! | | no particle, light, save flag, other moby written | n/a |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{add_rot, pf, pi32, pv4, set_pf, set_pi32, set_pv4, sub_rot};
use crate::moby_update::services::World;

use super::linked_mover::link_state;

/// The update in the level13 class table.
pub const UPDATE_FN: u32 = 0x2c_7f38;
pub const REFERENCE_LEVEL: u32 = 13;
pub const CLASSES: [i16; 4] = [127, 128, 159, 169];

const DEG: f32 = 0.017453292;

fn sound(w: &mut World, id: MobyId, o: usize) {
    let s = pi32(w, id, o);
    if s != -1 { w.play_sound(s, 0, id); }
}

/// The Euler step in degrees.
fn step(w: &World, id: MobyId) -> [f32; 3] { let s = pv4(w, id, 0x1c); [s[0], s[1], s[2]] }

/// Level13 0x2c7f38 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x40 { return; }
    let (mission, level) = (w.m(id).mission, w.svc.level);
    if mission != 0xff && w.mission_done(level, mission) == 0xff { set_pi32(w, id, 0x10, -1); }
    let (a, b) = (pi32(w, id, 0x14) as u32, pi32(w, id, 0x18) as u32);
    let link = link_state(w, id).map(u32::from);
    let rate = 1.0 / (pf(w, id, 0x2c) * 60.0);
    let next = match w.m(id).state {
        0 => {
            let r = w.m(id).rotation;
            set_pv4(w, id, 0, [r[0], r[1], r[2], 0.0]);
            w.mm(id).state = if pi32(w, id, 0x3c) == 0 { 1 } else { 2 };
            let k = pf(w, id, 0x38);
            if 0.0 < k {
                let oc = w.m(id).o_class;
                w.mm(id).scale = super::class_scale(w, oc) * k;
            }
            return;
        }
        1 => {
            if link.is_some_and(|s| s != a) { return; }
            sound(w, id, 0x30);
            set_pf(w, id, 0xc, 0.0);
            2
        }
        2 => {
            if link == Some(b) {
                sound(w, id, 0x30);
                4
            } else {
                let t = pf(w, id, 0xc) + rate;
                set_pf(w, id, 0xc, t);
                let s = step(w, id);
                let m = w.mm(id);
                for (r, s) in m.rotation.iter_mut().zip(s) { *r = add_rot(*r, s * DEG * rate); }
                if t < 1.0 { return; }
                let home = pv4(w, id, 0);
                let m = w.mm(id);
                for k in 0..3 { m.rotation[k] = add_rot(home[k], s[k] * DEG); }
                set_pf(w, id, 0xc, 1.0);
                w.mm(id).state = 3;
                if pi32(w, id, 0x34) != -1 { let s = pi32(w, id, 0x30); w.play_sound(s, 0, id); }
                return;
            }
        }
        3 => {
            if link != Some(b) { return; }
            set_pf(w, id, 0xc, 1.0);
            sound(w, id, 0x30);
            4
        }
        4 => {
            if link.is_none() || link == Some(a) {
                sound(w, id, 0x30);
                2
            } else {
                let t = pf(w, id, 0xc) - rate;
                set_pf(w, id, 0xc, t);
                let s = step(w, id);
                let m = w.mm(id);
                for (r, s) in m.rotation.iter_mut().zip(s) { *r = sub_rot(*r, s * DEG * rate); }
                if 0.0 < t { return; }
                let home = pv4(w, id, 0);
                let m = w.mm(id);
                m.rotation[..3].copy_from_slice(&home[..3]);
                set_pf(w, id, 0xc, 0.0);
                if pi32(w, id, 0x30) != -1 { let s = pi32(w, id, 0x34); w.play_sound(s, 0, id); }
                1
            }
        }
        _ => return,
    };
    w.mm(id).state = next;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::services::pvar as p;

    #[test]
    fn turns_out_while_the_link_is_away_and_back_in_state_a() {
        let mut m = Moby { o_class: 127, mission: 0xff, pvars: vec![0; 0x40], ..Moby::default() };
        p::set_i32(&mut m.pvars, 0x10, 1);
        p::set_i32(&mut m.pvars, 0x14, 1);
        p::set_i32(&mut m.pvars, 0x18, 2);
        p::set_ff(&mut m.pvars, 0x24, 90.0);
        p::set_ff(&mut m.pvars, 0x2c, 0.5);
        p::set_i32(&mut m.pvars, 0x30, 3);
        p::set_i32(&mut m.pvars, 0x34, 4);
        let link = Moby { o_class: 1, state: 1, ..Moby::default() };
        let mut t = MobyTable::new(vec![m, link], 4);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        update(&mut w, 0);
        update(&mut w, 0);
        assert_eq!(w.m(0).state, 2, "link in A: sound 3, turning");
        for _ in 0..30 { update(&mut w, 0); }
        assert_eq!(w.m(0).state, 3);
        assert!((w.m(0).rotation[2] - 90f32.to_radians()).abs() < 1e-5);
        w.mm(1).state = 2;
        update(&mut w, 0);
        assert_eq!(w.m(0).state, 4, "link in B: back");
        for _ in 0..31 { update(&mut w, 0); }
        assert_eq!((w.m(0).state, w.m(0).rotation[2]), (1, 0.0));
        let s: Vec<i32> = w.svc.sounds.iter().map(|s| s.index).collect();
        assert_eq!(s, vec![3, 3, 3, 4], "start, the end of the turn (sic: +0x30), B, home (+0x34)");
    }
}
