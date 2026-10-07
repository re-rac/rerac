//! Panels slid by a linked moby's state, classes 1013, 1014, 1064, 1065: level07 0x30cf90 (census U248; 6 created
//! instances on Pokitaru). A panel slides 0.01 a tick along its own y (1013 / 1065 toward +y, 1014 / 1064 toward −y)
//! from its home until it is further than its travel (xy) while the link is gone or not in state B, and back home
//! while the link is there and not in state A, with a sound at each start and stop. A done mission unlinks it. The
//! same idea as `linked_mover` (U247), separate code. Read from the level07 decomp (0x30cf90). Native `f32`.
//!
//! **Pvar block**: +0x00 home, +0x0c s16 a voice slot (never stored: always −1), +0x0e s8 / +0x0f s8 the start / stop
//! sounds (−1 none), +0x10 the link (moby index, −1 none), +0x14 s16 the map zone flag (−1 none), +0x16 u16 state A,
//! +0x18 state B, +0x1c the travel.
//!
//! The link is **away** when it is −1 or its moby is deleted (state 0xfe / 0xfd).
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x30cf90 | mission byte (+0xb0, `0x14c050 + level·16`) done → link = −1 | [`update`] |
//! | state 0 | home = position, voice −1, map flag = 0, → 1 | [`update`] |
//! | state 1 | wait while the link is there and not in A; else start sound, → 2 | [`update`] |
//! | state 2 | link there and in B → start sound (unchecked), → 4; else xy distance from home ≤ travel → position += rows·(0, ±0.01 (K = 0x15ed60), 0); else stop sound, map flag = 1, → 3 | [`update`] |
//! | state 3 | link there and in B → start sound, → 4 | [`update`] |
//! | state 4 | link away or in A → start sound (unchecked), → 2; else xy distance > 0.01 → position += rows·(0, ∓0.01, 0); else position = home (w 0), the voice released when it owns it, voice −1, stop sound, map flag = 0, → 1 | [`update`] |
//! | | the map zone flags `0x184528[i]` (read by the in-game map's zone table) | NOT ported: G-UI-001 (the map) |
//! | | no particle, light, save flag, other moby written | n/a |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, pf, pi16, pi32, pv4, set_pi16, set_pi32, set_pv4};
use crate::moby_update::services::World;

/// The update in the level07 class table.
pub const UPDATE_FN: u32 = 0x30_cf90;
pub const REFERENCE_LEVEL: u32 = 7;
pub const CLASSES: [i16; 4] = [1013, 1014, 1064, 1065];
/// The classes that slide out toward +y.
const PLUS: [i16; 2] = [0x3f5, 0x429];
/// The step per tick (K · 0.01).
const STEP: f32 = 0.01;

fn link(w: &World, id: MobyId) -> Option<u8> { super::linked_mover::link_state(w, id) }

fn sound(w: &mut World, id: MobyId, o: usize, checked: bool) {
    let s = w.m(id).pvars[o] as i8 as i32;
    if !checked || s != -1 { w.play_sound(s, 0, id); }
}

fn slide(w: &mut World, id: MobyId, out: bool) {
    let plus = PLUS.contains(&w.m(id).o_class);
    let y = if plus == out { STEP } else { -STEP };
    let r = w.m(id).rows;
    let d = [r[1][0] * y, r[1][1] * y, r[1][2] * y, r[1][3] * y];
    let p = c::add(w.m(id).position, d);
    w.mm(id).position = p;
}

/// Level07 0x30cf90 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x20 { return; }
    let (mission, level) = (w.m(id).mission, w.svc.level);
    if mission != 0xff && w.mission_done(level, mission) == 0xff { set_pi32(w, id, 0x10, -1); }
    let a = u16::from_le_bytes([w.m(id).pvars[0x16], w.m(id).pvars[0x17]]) as u32;
    let b = pi32(w, id, 0x18) as u32;
    let l = link(w, id).map(u32::from);
    let home = pv4(w, id, 0);
    let d = c::dist2(w.m(id).position, home);
    match w.m(id).state {
        0 => {
            let p = w.m(id).position;
            set_pv4(w, id, 0, [p[0], p[1], p[2], home[3]]);
            set_pi16(w, id, 0xc, -1);
            w.mm(id).state = 1;
        }
        1 => {
            if l.is_some_and(|s| s != a) { return; }
            sound(w, id, 0xe, true);
            w.mm(id).state = 2;
        }
        2 => {
            if l == Some(b) {
                sound(w, id, 0xe, false);
                w.mm(id).state = 4;
            } else if d <= pf(w, id, 0x1c) {
                slide(w, id, true);
            } else {
                sound(w, id, 0xf, true);
                w.mm(id).state = 3;
            }
        }
        3 => {
            if l.is_some() && l == Some(b) {
                sound(w, id, 0xe, true);
                w.mm(id).state = 4;
            }
        }
        4 => {
            if l.is_none() || l == Some(a) {
                sound(w, id, 0xe, false);
                w.mm(id).state = 2;
            } else if STEP < d {
                slide(w, id, false);
            } else {
                w.mm(id).position = [home[0], home[1], home[2], 0.0];
                let slot = pi16(w, id, 0xc) as i32;
                if slot != -1 && w.sound_owner(slot) == Some(id) { w.release_sound(slot, id); }
                set_pi16(w, id, 0xc, -1);
                sound(w, id, 0xf, true);
                w.mm(id).state = 1;
            }
        }
        _ => {}
    }
}
