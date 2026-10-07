//! Movers driven by a linked moby's state, classes 129, 130, 182, 183, 360, 1063, 1078, 1079, 1131: level07 0x310df0,
//! the same code on 13 (census U247; 28 created instances: doors, gates and lifts). The mover slides along a vector
//! (in its own frame) from its home to the far end and back, according to the state of the moby it is linked to:
//! out while the link is gone or not in the "close" state B, back while it is not in the "open" state A. It plays
//! its sounds at the turns and keeps an in-game map zone flag set while it is away from home. A done mission
//! unlinks it (it then opens for good). Read from the level07 decomp and disassembly (0x310df0). Native `f32`.
//!
//! **Pvar block**: +0x00 home (the init's position; +0x0c then holds the distance travelled), +0x10 the linked moby
//! (a pvar moby link: runtime index, −1 none), +0x14 state A, +0x18 state B, +0x1c..+0x24 the travel vector (moby
//! frame), +0x28 / +0x2c speed out / back (units per second), +0x30 / +0x34 sounds (−1 none), +0x38 the map zone
//! flag index (−1 none).
//!
//! The link is **away** when it is −1 or its moby is deleted (state 0xfe / 0xfd).
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x310df0 | mission byte (`0x14c050 + level·16`) done → link = −1 | [`update`] |
//! | state 0 | home = position, map flag = 0, → 1, travelled = 0 | [`update`] |
//! | state 1 | map flag = 0; wait while the link is there and not in A; else travelled = \|T\| (0x2212e8), sound +0x30, → 2 | [`update`] |
//! | state 2 | map flag = 1; link there and in B → sound +0x30, → 4; else position += `rows·T` (0x2215e0) at length speed-out·dt (0x221410, 0x221188), travelled −= speed-out·dt; below 0: position = home + rows·T, travelled = 0, → 3, sound +0x30 when +0x34 ≠ −1 (sic) | [`update`] |
//! | state 3 | map flag = 1; link there and in B → travelled = 0, sound +0x30, → 4 | [`update`] |
//! | state 4 | map flag = 0; link away or in A → sound +0x30, → 2; else position += rows·(T at length −speed-back·dt), travelled += speed-back·dt; past \|T\|: position = home, sound +0x34 when +0x30 ≠ −1 (sic), map flag = 0, → 1 | [`update`] |
//! | | the map zone flags `0x184528[i]` (read by the in-game map's zone table, flag 0x80) | NOT ported: G-UI-001 (the map) |
//! | | no particle, light, save flag, other moby written | n/a |
//!
//! **The platform variant**, classes 104, 106, 1129: level07 0x31aee0, the same code on 13 (census U263; 5 created
//! instances): the same state machine and quirks with the block at pvar +0xa0 (home +0xa0, travelled +0xac, link
//! +0xb0, A / B +0xb4 / +0xb8, travel +0xbc, speeds +0xc8 / +0xcc, sounds +0xd0 / +0xd4, map flag +0xd8), the map
//! flag written only at the init (0), the end of the way out (1) and home (0), and a platform block at +0x60:
//! `CarryRiders(+0x60, position − old position, rotation, rotation)` every tick (0x288d88 = L01 0x2755f8)
//! ([`platform_update`], [`run`] with base 0xa0; read from the level07 decomp of 0x31aee0).
//!
//! **The held variant**, class 1128: level07 0x31a9e8 (census U276; 1 placed): the same machine on the block at +0x00,
//! but when its mission's loaded byte (`0x15fc88[+0xb0]`, `MissionState::mission_slot`) is −1 it no longer follows the
//! link: with pvar +0x3c = 0 it stays home (or goes back), else it opens and stays open ([`held_update`]).

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{add, len3, pf, pi32, pv4, set_len3, set_pf, set_pi32, set_pv4, DT};
use crate::moby_update::services::World;

/// The update in the level07 class table.
pub const UPDATE_FN: u32 = 0x31_0df0;
pub const REFERENCE_LEVEL: u32 = 7;
pub const CLASSES: [i16; 4] = [1063, 1078, 1079, 1131];
/// The platform variant (level07 0x31aee0): its classes and its block's pvar offset.
pub const PLATFORM_FN: u32 = 0x31_aee0;
pub const PLATFORM_CLASSES: [i16; 3] = [104, 106, 1129];
pub const PLATFORM_BASE: usize = 0xa0;
/// The held variant (level07 0x31a9e8, class 1128; census U276).
pub const HELD_FN: u32 = 0x31_a9e8;
pub const HELD_CLASSES: [i16; 1] = [1128];

type V = [f32; 4];

/// `rows · v` (x, y, z; w 0).
fn rows_mul(r: &[[f32; 4]; 4], v: V) -> V { std::array::from_fn(|l| if l == 3 { 0.0 } else { r[0][l] * v[0] + r[1][l] * v[1] + r[2][l] * v[2] }) }

/// The link's state (pvar +0x10), None when it is away (module doc); also the rotators' (`linked_rotator`, the same pvar layout).
pub fn link_state(w: &World, id: MobyId) -> Option<u8> { link_state_at(w, id, 0) }

/// [`link_state`] for the block at pvar `base` (the link at `base + 0x10`).
pub fn link_state_at(w: &World, id: MobyId, base: usize) -> Option<u8> {
    let l = usize::try_from(pi32(w, id, base + 0x10)).ok()?;
    let s = w.table.mobys.get(l)?.state;
    (s != 0xfe && s != 0xfd).then_some(s)
}

fn sound(w: &mut World, id: MobyId, o: usize) {
    let s = pi32(w, id, o);
    if s != -1 { w.play_sound(s, 0, id); }
}

fn travel(w: &World, id: MobyId, base: usize) -> V { let t = pv4(w, id, base + 0x1c); [t[0], t[1], t[2], 0.0] }

/// Level07 0x310df0 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x3c { return; }
    run(w, id, 0);
}

/// Level07 0x31aee0 (classes 104 / 106 / 1129, census U263; the same code on 13): the mover with its block at pvar
/// +0xa0 and a platform block at +0x60, carrying its riders every tick by its displacement (`CarryRiders(+0x60,
/// position − old, rot, rot)`, 0x288d88 = L01 0x2755f8). Its map zone flag is written only at the init and the two
/// turns (+0xd8; G-UI-001 either way) (module doc).
pub fn platform_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < PLATFORM_BASE + 0x3c { return; }
    let old = w.m(id).position;
    run(w, id, PLATFORM_BASE);
    let m = w.mm(id);
    let (d, r) = (crate::moby_update::creature::sub(m.position, old), m.rotation);
    crate::moby_update::triggers::carry_riders(&mut m.pvars, 0x60, d, r, r);
}

/// Level07 0x31a9e8 (class 1128, census U276): [`run`] with the mission's loaded byte (`0x15fc88[+0xb0]`) at −1
/// holding it: pvar +0x3c 0 keeps it shut (it closes), else open (module doc).
pub fn held_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x40 { return; }
    let mission = w.m(id).mission;
    let held = (w.missions.mission_slot(mission) == 0xff).then(|| pi32(w, id, 0x3c) != 0);
    run_with(w, id, 0, held);
}

/// The state machine on the block at pvar `base` (module doc; offsets relative to `base`).
pub fn run(w: &mut World, id: MobyId, base: usize) { run_with(w, id, base, None) }

/// [`run`] with the held variant's override: `Some(false)` shut, `Some(true)` open, whatever the link does.
fn run_with(w: &mut World, id: MobyId, base: usize, held: Option<bool>) {
    let (shut, open) = (held == Some(false), held == Some(true));
    let (mission, level) = (w.m(id).mission, w.svc.level);
    if mission != 0xff && w.mission_done(level, mission) == 0xff { set_pi32(w, id, base + 0x10, -1); }
    let (a, b) = (pi32(w, id, base + 0x14) as u32, pi32(w, id, base + 0x18) as u32);
    let link = link_state_at(w, id, base).map(u32::from);
    let next = match w.m(id).state {
        0 => {
            let p = w.m(id).position;
            set_pv4(w, id, base, p);
            set_pf(w, id, base + 0xc, 0.0);
            1
        }
        1 => {
            if shut || (!open && link.is_some_and(|s| s != a)) { return; }
            let l = len3(travel(w, id, base));
            set_pf(w, id, base + 0xc, l);
            sound(w, id, base + 0x30);
            2
        }
        2 => {
            if shut || (!open && link == Some(b)) {
                sound(w, id, base + 0x30);
                4
            } else {
                let d = rows_mul(&w.m(id).rows, travel(w, id, base));
                let step = pf(w, id, base + 0x28) * DT;
                let p = add(w.m(id).position, set_len3(d, step));
                w.mm(id).position = p;
                let left = pf(w, id, base + 0xc) - step;
                set_pf(w, id, base + 0xc, left);
                if 0.0 <= left { return; }
                let home = pv4(w, id, base);
                w.mm(id).position = add(home, d);
                set_pf(w, id, base + 0xc, 0.0);
                w.mm(id).state = 3;
                if pi32(w, id, base + 0x34) != -1 { sound(w, id, base + 0x30); }
                return;
            }
        }
        3 => {
            if !shut && (open || link != Some(b)) { return; }
            set_pf(w, id, base + 0xc, 0.0);
            sound(w, id, base + 0x30);
            4
        }
        4 => {
            if !shut && (open || link.is_none_or(|s| s == a)) {
                sound(w, id, base + 0x30);
                2
            } else {
                let t = travel(w, id, base);
                let l = len3(t);
                let step = pf(w, id, base + 0x2c) * DT;
                let d = rows_mul(&w.m(id).rows, set_len3(t, -step));
                let p = add(w.m(id).position, d);
                w.mm(id).position = p;
                let done = pf(w, id, base + 0xc) + step;
                set_pf(w, id, base + 0xc, done);
                if done <= l { return; }
                let home = pv4(w, id, base);
                w.mm(id).position = home;
                if pi32(w, id, base + 0x30) != -1 { sound(w, id, base + 0x34); }
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
    fn opens_while_the_link_is_away_and_closes_on_state_b() {
        let mut m = Moby { o_class: 129, mission: 0xff, position: [1.0, 2.0, 3.0, 1.0], pvars: vec![0; 0x40], ..Moby::default() };
        m.rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        p::set_i32(&mut m.pvars, 0x10, 1);
        p::set_i32(&mut m.pvars, 0x14, 5);
        p::set_i32(&mut m.pvars, 0x18, 6);
        p::set_v4f(&mut m.pvars, 0x1c, [0.0, 0.0, 1.0, 0.0]);
        p::set_ff(&mut m.pvars, 0x28, 30.0);
        p::set_ff(&mut m.pvars, 0x2c, 30.0);
        p::set_i32(&mut m.pvars, 0x30, 4);
        p::set_i32(&mut m.pvars, 0x34, 7);
        p::set_i32(&mut m.pvars, 0x38, -1);
        let link = Moby { state: 1, ..Moby::default() };
        let mut t = MobyTable::new(vec![m, link], 4);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        update(&mut w, 0);
        update(&mut w, 0);
        assert_eq!(w.m(0).state, 1, "waits for the link's state A");
        w.mm(1).state = 5;
        update(&mut w, 0);
        assert_eq!((w.m(0).state, pf(&w, 0, 0xc)), (2, 1.0));
        for _ in 0..3 { update(&mut w, 0); }
        assert_eq!((w.m(0).state, w.m(0).position[2]), (3, 4.0), "the far end after 1 / (30·dt) = 2 ticks + the snap");
        w.mm(1).state = 6;
        update(&mut w, 0);
        assert_eq!(w.m(0).state, 4);
        w.mm(1).state = 2;
        for _ in 0..3 { update(&mut w, 0); }
        assert_eq!((w.m(0).state, w.m(0).position[2]), (1, 3.0), "home again");
        let s: Vec<i32> = w.svc.sounds.iter().map(|s| s.index).collect();
        assert_eq!(s, [4, 4, 4, 7]);
    }
}
