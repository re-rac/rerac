//! **Quartu's story pickup and director** (level 15; read from the level15 decomp):
//!
//! * **1388 the Bolt Grabber** (`0x2ea748`, census U503, one instance): a spinning, glowing item; within 3 (XY) it
//!   hides and plays scene 5; after it: banner 15011 (countdown `ticks(180)`), the Bolt Grabber (item 34), a save;
//!   gone (also when acquired).
//! * **1419 the Drek broadcast director** (`0x2eb4c0`, U506, one instance): with its mission open and the gp word
//!   0x161ac0 clear, Ratchet in cuboid P[0] while the moby P[2] is in state 3: scene 3, its mission, Ratchet hidden;
//!   then movie 17, scene 4 (Ratchet shown, put on cuboid P[1]), and Veldin Orbit (planet 17) unlocked with the
//!   checkpoint at cuboid P[1], its banner and a save.
//!
//! * **1469 the help director** (`0x2ed398`, U511, one instance): the Hologuise / stealth hints around the sentry
//!   moby +0x10 (class 0x4e), the Giant Clank hint, the Hydro-Pack hints, the swim hint; flags 0x6e / 0x6f.
//!
//! **System or not.** Per-class code; the glow is the infobot's (`gold_bolt`); the director's inline patterns are
//! `units::hints`'.
//!
//! ## 1388 coverage (level15 `0x2ea748`)
//!
//! | address | what | status |
//! |---|---|---|
//! | head | yaw += dt·π/2; scale = class scale × gp−0x4b88 (1.0) | [`grabber_update`] |
//! | state 0 | the glow init (`0x2ea8d0`); Bolt Grabber acquired (0x13d50a) → `DeleteMoby`; else → 1, z += 1 | [`grabber_update`] |
//! | state 1 | the glow (`0x2ea9b0`: z + 0.5); within 3 (XY): mode \|= 0x41, `DialogStreamStart(5)`, → 2 | [`grabber_update`] |
//! | state 2 | game mode ≠ 2: `ShowBanner(15011, −1)`, countdown `ticks(180)`, `GiveItem(34, 1)`, `memcard_Save`, → 3 | [`grabber_update`] |
//! | state 3 | `DeleteMoby` | [`grabber_update`] |
//!
//! ## 1419 coverage (level15 `0x2eb4c0`)
//!
//! | address | what | status |
//! |---|---|---|
//! | head | update distance 0x80 | [`broadcast_update`] |
//! | state 0 | mission open, gp−0x5140 (0x161ac0) = 0, Ratchet in cuboid P[0], moby P[2] in state 3: `DialogStreamStart(3)`, → 2, `SetMissionDone(+0xb0)`, Ratchet hidden (`0x223c20` = level01 `0x2486c0`) | [`broadcast_update`] (`HeroFields::hero_hidden` [L: the port hides him through 0x1413f5]) |
//! | state 2 | game mode ≠ 2: `DialogStreamUpdate(17)`, → 1 | [`broadcast_update`] |
//! | state 1 | game mode ≠ 2: `DialogStreamStart(4)`, → 3, Ratchet shown (`0x223d08` = `0x2487a8`), `HeroTeleport(cuboid P[1], 0, 1)` | [`broadcast_update`] |
//! | state 3 | game mode ≠ 2: `UnlockPlanet(17)`, the checkpoint at cuboid P[1], `ShowPlanetBanner(17)`, `memcard_Save`, → 4 | [`broadcast_update`] |
//! | tail | in game mode 2: `RegisterDrawCallback(0x2eb6b8)`: the typed captions of scene 3 (21098 at scene ticks 181..260, 21099 at 450..640, 21100 at 750..930) and scene 4 (21102 at 2300..2400), one character per 4 ticks, large font with a shadow | NOT ported (G-UI-025: a class's text over a scene) |

//!
//! ## 1469 coverage (level15 `0x2ed398`)
//!
//! | address | what | status |
//! |---|---|---|
//! | state 0 | update distance 0xff; +0x34 = `ticks(1200)`; → 1 | [`help_update`] |
//! | state 1 | the sentry +0x10 (class 0x4e, alive, not in state 3), Ratchet in cuboid +0x0c, group ok, the help idle: Hologuise (item 31) not owned → the reminder on record 0x7b → `Help_Request(15000, 0x7b)`; owned → record 0x7c armed, then its reminder → `(15001, 0x7c)` | [`help_update`] (`hints::remind` / `arm_or_remind`) |
//! | | the level-15 death flag 0x15f5a4 set, the help idle, record 0x89 never closed → `(15007, 0x89)`, the flag cleared | [`help_update`] (the flag: `units.word(0x15f5a4)`; its writer, the death sequence on level 15, is not ported: `hero/damage.rs` module doc [L]) |
//! | | cuboid +0x54, the disguise (body 3), 0x141630 = 0, the word 0x161ac0 = 0, record 0x88 never closed → `(15006, 0x88)` | [`help_update`] (body 3 never occurs in the port: G-WPN-006; 0x141630 read as 0 [L]) |
//! | | cuboid +0x00: counter +0x3c + 1, else 0 and +0x40 = −1; the sentry valid, +0x34 < +0x3c, the cuboid ≠ +0x40, group ok, idle, Hologuise owned → +0x34 = `ticks(1800)`, record 0x7d never shown → `(15002, 0x7d)`; +0x40 = the cuboid, +0x3c = 0 | [`help_update`] |
//! | | the hero moby on sequence 0xc in Giant Clank (body 2) → flag 0x6f (0x13d3f7) | [`help_update`] |
//! | | cuboid +0x1c, body 2, flag 0x6f clear, idle, record 0x80 never shown → `(15004, 0x80)` | [`help_update`] |
//! | | cuboid +0x20, idle, the Hydro-Pack (item 4) not owned, record 0x82 without this level's bit → `(15005, 0x82)` | [`help_update`] |
//! | | cuboids +0x20 / +0x24 / +0x28: inside → counter +0x38 + 1 (a cuboid ≠ +0x40 is the candidate); outside or state 0x35 → +0x38 = 0, +0x40 = −1; past `ticks(3600)` with a candidate, in the water (`0x2056e0`), idle, Hydro-Pack owned → `(20014, 0x78)`, +0x40 = candidate, +0x38 = 0 | [`help_update`] (`help::hero_in_water`) |
//! | | flag 0x6e (0x13d3f6) clear and Ratchet in cuboid +0x2c → set | [`help_update`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::gold_bolt;
use crate::moby_update::creature::{self as c, DT};
use crate::moby_update::interact;
use crate::moby_update::services::{pvar as p, World};
use crate::moby_update::story;

pub const REFERENCE_LEVEL: u32 = 15;
pub const GRABBER_FN: u32 = 0x2e_a748;
pub const GRABBER_CLASSES: [i16; 1] = [1388];
pub const BROADCAST_FN: u32 = 0x2e_b4c0;
pub const BROADCAST_CLASSES: [i16; 1] = [1419];
pub const HELP_FN: u32 = 0x2e_d398;
pub const HELP_CLASSES: [i16; 1] = [1469];
/// The level-15 death flag 0x15f5a4 (gp−0x765c) the director reads.
pub const DEATH_WORD: u32 = 0x15_f5a4;
/// gp−0x5140 (0x161ac0): the level word that holds the broadcast back (written by level15 `0x2a36d0`'s class, not ported).
pub const HOLD_WORD: u32 = 0x16_1ac0;

/// Level15 `0x2ea748` (1388; module doc).
pub fn grabber_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x50);
    let y = c::add_rot(c::yaw(w, id), DT * std::f32::consts::FRAC_PI_2);
    c::set_yaw(w, id, y);
    let k = w.classes.info(w.m(id).o_class).map_or(1.0, |i| i.scale);
    w.mm(id).scale = k;
    match w.m(id).state {
        0 => {
            gold_bolt::glow_init(w, id, 0x10);
            if w.svc.interact.game.acquired.get(34).is_some_and(|&b| b != 0) {
                w.delete_moby(id);
                return;
            }
            w.mm(id).state = 1;
            w.mm(id).position[2] += 1.0;
        }
        1 => {
            let mut centre = w.m(id).position;
            centre[2] += 0.5;
            gold_bolt::item_glow(w, id, 0x10, [centre[0], centre[1], centre[2]], 1.0, true);
            if c::dist2(c::pos(w, id), story::hero4(w)) < 3.0 {
                w.mm(id).mode |= mode::HIDDEN | mode::NO_ANIM;
                crate::cinematic::start_scene(w, 5, false);
                w.mm(id).state = 2;
            }
        }
        2 => {
            if w.svc.game_mode == 2 { return; }
            let t = w.ticks(0xb4);
            crate::cinematic::show_banner(w, 0x3aa3, t);
            interact::give_item(w, 34, true);
            crate::cinematic::save(w);
            w.mm(id).state = 3;
        }
        3 => w.delete_moby(id),
        _ => {}
    }
}

/// Level15 `0x2eb4c0` (1419; module doc).
pub fn broadcast_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x10);
    w.mm(id).update_dist = 0x80;
    let mode2 = w.svc.game_mode == 2;
    let pv = |w: &World, k: usize| p::i32(&w.m(id).pvars, 4 * k);
    match w.m(id).state {
        0 => {
            if story::mission_done(w, w.m(id).mission as i32) { return; }
            if w.svc.units.word(HOLD_WORD) != 0 { return; }
            if !story::hero_in(w, pv(w, 0)) { return; }
            if story::link(w, pv(w, 2)).is_some_and(|m| w.m(m).state == 3) {
                crate::cinematic::start_scene(w, 3, false);
                w.mm(id).state = 2;
                let mission = w.m(id).mission;
                crate::cinematic::set_mission_done(w, mission);
                w.hero_fields_mut().hero_hidden = Some(1);
            }
        }
        1 => {
            if !mode2 {
                crate::cinematic::start_scene(w, 4, false);
                w.mm(id).state = 3;
                w.hero_fields_mut().hero_hidden = Some(0);
                story::teleport_to(w, pv(w, 1), 0, true);
            }
        }
        2 => {
            if !mode2 {
                crate::cinematic::start_movie(w, 0x11);
                w.mm(id).state = 1;
            }
        }
        3 if !mode2 => {
            crate::cinematic::unlock_planet(w, 0x11);
            story::checkpoint_at(w, pv(w, 1));
            crate::cinematic::show_planet_banner(w, 0x11);
            crate::cinematic::save(w);
            w.mm(id).state = 4;
        }
        _ => {}
    }
    if w.svc.game_mode == 2 { w.svc.unported("quartu 1419: scene captions 0x2eb6b8 (G-UI-025)"); }
}

/// Level15 `0x2ed398` (1469; module doc).
pub fn help_update(w: &mut World, id: MobyId) {
    use super::hints::{arm_or_remind, flag, group_ok, idle, in_cuboid, owned, pvi, remind, request, set_flag, set_pvi};
    story::pvars(w, id, 0x58);
    if w.m(id).state == 0 {
        w.mm(id).update_dist = 0xff;
        let t = w.ticks(0x4b0);
        set_pvi(w, id, 0x34, t);
        w.mm(id).state = 1;
        return;
    }
    if w.m(id).state != 1 { return; }
    let sentry_ok = |w: &World| story::link(w, pvi(w, id, 0x10)).is_some_and(|m| w.m(m).o_class == 0x4e && story::alive(w, m) && w.m(m).state != 3);
    if sentry_ok(w) && in_cuboid(w, id, 0xc) && group_ok(w) && idle(w) {
        if !owned(w, 31) { remind(w, 0x7b, 15000, 0x7b); } else { arm_or_remind(w, 0x7c, 0x3a99, 0x7c); }
    }
    if w.svc.units.word(DEATH_WORD) != 0 && idle(w) && w.svc.help.records.help[0x89].count == 0 {
        request(w, 0x3a9f, 0x89);
        w.svc.units.set_word(DEATH_WORD, 0);
    }
    if in_cuboid(w, id, 0x54) && w.body() == 3 && w.svc.units.word(HOLD_WORD) == 0 && w.svc.help.records.help[0x88].count == 0 {
        request(w, 0x3a9e, 0x88);
    }
    let c0 = pvi(w, id, 0);
    if in_cuboid(w, id, 0) {
        set_pvi(w, id, 0x3c, pvi(w, id, 0x3c) + 1);
    } else {
        set_pvi(w, id, 0x3c, 0);
        set_pvi(w, id, 0x40, -1);
    }
    let inside0 = in_cuboid(w, id, 0);
    if inside0 && sentry_ok(w) && pvi(w, id, 0x34) < pvi(w, id, 0x3c) && c0 != -1 && c0 != pvi(w, id, 0x40) && group_ok(w) && idle(w) && owned(w, 31) {
        let t = w.ticks(0x708);
        set_pvi(w, id, 0x34, t);
        if (w.svc.help.records.help[0x7d].mask as i32) >= 0 { request(w, 0x3a9a, 0x7d); }
        set_pvi(w, id, 0x40, c0);
        set_pvi(w, id, 0x3c, 0);
    }
    if w.body() == 2 && w.hero_moby.is_some_and(|h| w.m(h).anim.seq_a == 0xc) { set_flag(w, 0x6f); }
    if in_cuboid(w, id, 0x1c) && w.body() == 2 && !flag(w, 0x6f) && idle(w) && (w.svc.help.records.help[0x80].mask as i32) >= 0 {
        request(w, 0x3a9c, 0x80);
    }
    let bit = 1u32 << (w.svc.help.level as u32 & 31);
    if in_cuboid(w, id, 0x20) && idle(w) && !owned(w, 4) && w.svc.help.records.help[0x82].mask & bit == 0 {
        request(w, 0x3a9d, 0x82);
    }
    let found = [0x20, 0x24, 0x28].into_iter().find(|&o| in_cuboid(w, id, o)).map(|o| pvi(w, id, o));
    let candidate = found.filter(|&c| c != pvi(w, id, 0x40)).unwrap_or(-1);
    if found.is_some() { set_pvi(w, id, 0x38, pvi(w, id, 0x38) + 1); }
    if found.is_none() || w.hero.state == 0x35 {
        set_pvi(w, id, 0x38, 0);
        set_pvi(w, id, 0x40, -1);
    }
    if w.ticks(0xe10) < pvi(w, id, 0x38) && candidate != -1 && crate::help::hero_in_water(w.hero.state, w.hero.group) && idle(w) && owned(w, 4) {
        request(w, 0x4e2e, 0x78);
        set_pvi(w, id, 0x40, candidate);
        set_pvi(w, id, 0x38, 0);
    }
    if !flag(w, 0x6e) && in_cuboid(w, id, 0x2c) { set_flag(w, 0x6e); }
}
