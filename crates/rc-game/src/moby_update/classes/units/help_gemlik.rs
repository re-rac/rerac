//! The Gemlik help director, class 558 (census U419; level13 `0x2f3778`, one instance). Read from the level13 decomp.
//!
//! **Pvars**: +0x00 the hint's cuboid, +0x04 a moby (class 170; −1 none), +0x08 the station's air cuboid.
//!
//! | address | what | port |
//! |---|---|---|
//! | entry | update distance 0xff every tick | [`update`] |
//! | | box idle, group ok, game mode 0x15f5c4 = 0: the moby +0x04 gone (−1, not class 170, or deleted) and item 11 or 13 owned → `H[0x6f]` bumped (every such tick) | [`update`] |
//! | | else in cuboid +0x00 and `H[0x6f]` never shown (mask ≥ 0) → `Help_Request(13000, 0x6f)` | [`update`] |
//! | exit | 0x14161b := Ratchet **not** in cuboid +0x08 (the airless flag the O2 Mask's head-item rule reads, `hero/worn.rs`): the mask goes on outside the station's air, also at the level's start | [`update`] (`HeroFields::airless`) |
//! | no sound, particle, flag, save, other moby written | | n/a |

use super::hints::{bump, class_state, group_ok, idle, in_cuboid, owned, pvi, request};
use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

pub const UPDATE_FN: u32 = 0x2f_3778;
pub const REFERENCE_LEVEL: u32 = 13;
pub const CLASSES: [i16; 1] = [558];

/// Level13 0x2f3778 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    w.mm(id).update_dist = 0xff;
    hints(w, id);
    let air = in_cuboid(w, id, 8);
    w.hero_fields_mut().airless = Some(!air as u8);
}

/// The help part (module doc).
fn hints(w: &mut World, id: MobyId) {
    if !(idle(w) && group_ok(w) && w.svc.game_mode == 0) { return; }
    let gone = match class_state(w, pvi(w, id, 4)) {
        Some((c, s)) => c != 0xaa || s == 0xfe || s == 0xfd,
        None => true,
    };
    if gone && (owned(w, 11) || owned(w, 13)) {
        bump(w, 0x6f);
    } else if in_cuboid(w, id, 0) && (w.svc.help.records.help[0x6f].mask as i32) >= 0 {
        request(w, 13000, 0x6f);
    }
}
