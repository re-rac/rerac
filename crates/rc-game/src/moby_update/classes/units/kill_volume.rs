//! Kill cuboids, class 1039: level06 0x2f7930, the same code on 08 (0x302a78), 13 (0x3036f0) and 18 (0x2ec858)
//! (census U216; one placed instance per level). Two lists of up to 32 cuboids each: Ratchet's position inside one of
//! the first list starts the death fall (state 0x77) — or, in another body, fades to black and runs the death
//! sequence — and inside one of the second list always fades and dies. The cuboids are off while he is mounted in a
//! ship (hero state 0x32 with the vehicle moby `0x140940` alive and of class 0x45). Read from the level06 decomp and
//! the disassembly; the four copies are one census unit (the masked code is identical). Native `f32`.
//!
//! **Pvar block** (s32): +0x00..+0x7c the fall cuboids, +0x80..+0xfc the fade cuboids (−1: none).
//!
//! | address | what | port |
//! |---|---|---|
//! | state 0 | +0x30 (update distance) = 0xff, → 1 | [`update`] |
//! | state 1, gate | skipped while `0x1413d4` = 0x32 **and** `0x140940` ≠ 0 **and** that moby's state is not −2 / −3 **and** `0x140944` (its class) = 0x45 | [`update`]: the state test reads the hero ([`MOUNTED`]); the vehicle word `0x140940` is written only by the ships 1242 / 69 / 1379 (unported, G-LVL-009), so it is 0 in the port and the cuboids stay on (the game's own outcome whenever no ship is flown, e.g. Batalia's turret, which mounts him without it) |
//! | first list | per cuboid ≥ 0: `PointInCuboid(0x13f3d0, c)` (0x274820); body `0x1413f4` = 0: state ≠ 0x77 → `SetState(0x77, 1)` (the level's copy: the port's SetState, level_generalisation.md H2); other bodies: `FadeToBlack(ticks(10))`, the death sequence `0x2319b0` (L06 `0x228230`) | [`update`] (`HeroCall::SetState`, `cinematic::fade_to_black`, `HeroCall::Death`) |
//! | second list | per cuboid ≥ 0 inside: `FadeToBlack(ticks(10))`, the death sequence | [`update`] |
//! | | no sound, particle, light, HUD, save flag, other moby; the death sequence counts the deaths (`hero::damage::death_fade`) | n/a |

use crate::hero::scripted::MOUNTED;
use crate::moby_runtime::MobyId;
use crate::moby_update::creature::pi32;
use crate::moby_update::services::{HeroCall, World};

/// The update in the level06 class table.
pub const UPDATE_FN: u32 = 0x2f_7930;
pub const REFERENCE_LEVEL: u32 = 6;
pub const CLASSES: [i16; 1] = [1039];

/// The death fall the first list starts.
pub const DEATH_FALL: i32 = 0x77;
/// The ship class the gate tests (`0x140944`).
pub const SHIP_CLASS: i16 = 0x45;
/// The fade before the death sequence.
const FADE: i32 = 10;
/// Cuboids per list.
const PER_LIST: usize = 32;

/// `0x140940` / `0x140944`: the vehicle moby and its class. Written only by the ship classes (G-LVL-009), none of which
/// the port has: no vehicle.
fn vehicle(_w: &World) -> Option<(MobyId, i16)> { None }

/// The gate of state 1 (module doc): true while he flies a live ship of class 0x45.
fn in_ship(w: &World) -> bool {
    w.hero.state == MOUNTED
        && vehicle(w).is_some_and(|(v, class)| {
            let s = w.m(v).state as i8;
            s != -2 && s != -3 && class == SHIP_CLASS
        })
}

fn inside(w: &World, c: i32) -> bool {
    let p = super::hero_pos(w);
    -1 < c && w.in_cuboid([p[0], p[1], p[2]], c)
}

fn fade_and_die(w: &mut World) {
    let t = w.ticks(FADE);
    crate::cinematic::fade_to_black(w, t);
    w.hero_fields_mut().call(HeroCall::Death);
}

/// Level06 0x2f7930 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 8 * PER_LIST { return; }
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.state = 1;
        }
        1 if !in_ship(w) => {
            // The hero calls run after the loop; the game's SetState is immediate, so the next cuboid of the list
            // sees 0x77 already: kept here as the state the queued call sets.
            let mut state = w.hero.state;
            for k in 0..PER_LIST {
                let c = pi32(w, id, 4 * k);
                if !inside(w, c) { continue; }
                if w.body() == 0 {
                    if state != DEATH_FALL {
                        w.hero_fields_mut().call(HeroCall::SetState { id: DEATH_FALL, play: true });
                        state = DEATH_FALL;
                    }
                } else {
                    fade_and_die(w);
                }
            }
            for k in 0..PER_LIST {
                let c = pi32(w, id, 0x80 + 4 * k);
                if inside(w, c) { fade_and_die(w); }
            }
        }
        _ => {}
    }
}
