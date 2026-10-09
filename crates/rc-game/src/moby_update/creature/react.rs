//! **How weapons act on creatures besides damage** (level01; docs/plan/hero_gameplay.md §10, creatures.md §8). What
//! the game shares, and what it does not:
//!
//! * **Damage, knockback, burn** go through the hit records every weapon writes (`coll_sphere_mobys` / `CollLine_Fix`
//!   with a template, `0x26eaa8`) and the resolver `0x26f378` every creature runs ([`super::damage`], ported before).
//! * **The class reaction table** (the third word of each `lvl.vtbl` entry, copied to class header +0x2c by the level
//!   loader: six function pointers; level01's default 0x20c3ac = six `return 0`s, 0x3040b8..0x3040f8, and a
//!   `DeleteMoby` wrapper): the only per-class dispatch a weapon uses. **Only the Suck Cannon calls it** (0x3028c8,
//!   0x302a88, the fire-out in 0x303000, the carried update's release). Slots: +0x00 suck start, +0x04 swallow,
//!   +0x08 fire out, +0x0c let go, +0x10 the suck record, +0x14 delete. Each class's slots are three-line wrappers
//!   ([`Wrappers`]: 577 0x2f1c78.., the amoeboids 572 / 866 0x2efa88.., the chicken 270 0x2e0a28..) around the shared
//!   handlers ported here: [`approach`] 0x304168, [`swallow`] 0x304390, [`fire_out`] 0x3044a0, [`let_go`] 0x304690,
//!   the carried update [`carried`] 0x305260 (the classes call it from their own state: 577 state 7, 572 / 866
//!   state 0xe, 270 state 5), [`land`] 0x3051a8 and the burst [`burst`] 0x304798.
//!   Classes with a table (all 19 overlays' `lvl.vtbl`): 270 everywhere; 577 (01); 572 / 866 (01, 05, 11); 749
//!   (00, 18: [`VELDIN_749`], reversed on level 00, found by [`tables_from_overlays`]); 580 (02), 340 (04: [`EUDORA_340`]), 827 (06,
//!   10), 252 (08, 14), 193 (09, 15), 1246 (11), 238 (12), 63 (13), 1445 (16), 1382 (17: [`FLEET_1382`]), 568 (18: [`ROLLING_MINE_568`]),
//!   1906 (18: [`VELDIN_1906`]) — the classes without a table row here are not ported. 865 and 459 keep the default: never sucked.
//! * **The damage record** (the creature header's +0x00, `FUN_002711f8`) carries two more weapon inputs: **+0x18 the
//!   lure** (the Taunter's `0x2cc830` writes its moby; 577 turns it into its 240-tick alert, 572 into its alert,
//!   459 into its 600-tick alert — the classes already read it, as `ALERT`), +0x04 (s16) the Morph-o-Ray's full meter
//!   and +0x0e the morph's keep byte (1 → 2 instead of deleted; [`morph_target`], the chicken 270's module).
//! * **Decoys**: the enemies' own target search (`0x274b78`, [`super::target`]) prefers the Decoy Glove's 0xcb / 0x76c
//!   and a gold Morph-o-Ray chicken (0x10e with +0xbc ≠ 0).
//!
//! There is no stun and no freeze in this game; knockback is the resolver's reaction byte (classes act on it).
//!
//! **The suck record** (the creature header's +0x14 "extra" record: 577 at pvar +0x60, the amoeboids at +0xc0; the
//! chicken's lives in a global table the port keeps in its pvars): [`rec`] offsets. Values +0x78..+0x9e come from the
//! instance data (0, 0.05, 0.863, 4, 3, 0.625, sounds −1 on every Novalis / Rilgar creature and in `0x2def40`).
//!
//! **The Suck Cannon's globals** ([`Globals`]): the slots 0x1413a0 (5, or 10 with the gold cannon 0x13e529), the
//! counts 0x1413c8 (held) / 0x1413cc (coming), the swap lock 0x1403fc the carried update sets. The hand item is not a
//! table moby in the port: the cannon's position, mouth (joint list 0) and rows as its last update left them are
//! [`Cannon`], which is what the game's moby loop reads too (the hand item's matrix is built in the hero update).
//!
//! Native `f32`; the rand draws are the game's, at the game's points.

use super::V;
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::services::{pf, HitTemplate, World};
use crate::ps2v::Pf;
use std::f32::consts::PI;

/// The suck record's fields (offsets from its start).
pub mod rec {
    /// +0x00 vec: the cannon's mouth when it last pulled (w = 1).
    pub const POINT: usize = 0x00;
    /// +0x0c f32: the let-go countdown (1 on every pull; the carried update lets go at ≤ 0).
    pub const HOLD: usize = 0x0c;
    /// +0x10 vec: the flight velocity.
    pub const VEL: usize = 0x10;
    /// +0x20 quat: the orientation when the pull began.
    pub const QUAT: usize = 0x20;
    /// +0x30 vec: the moby's position (copied every carried tick).
    pub const POS: usize = 0x30;
    /// +0x40 vec: the homing target's point last tick.
    pub const TARGET_AT: usize = 0x40;
    /// +0x50 vec: the last contact (hit point − position; the roll axis's up).
    pub const CONTACT: usize = 0x50;
    /// +0x60: the cannon (the port: Ratchet's moby + 1).
    pub const CANNON: usize = 0x60;
    /// +0x64 f32: the distance to the mouth when the pull began.
    pub const DIST0: usize = 0x64;
    /// +0x68 s16 state, +0x6a s16 slot.
    pub const STATE: usize = 0x68;
    pub const SLOT: usize = 0x6a;
    /// +0x6c f32: progress.
    pub const T: usize = 0x6c;
    /// +0x70: the class's sequence table (the port: which [`Wrappers::seqs`]).
    pub const SEQS: usize = 0x70;
    /// +0x74: the homing target (the port: moby + 1).
    pub const TARGET: usize = 0x74;
    /// +0x78 f32: the animation speed after a let-go landing; +0x7c the scale regrowth a tick; +0x80 the flight
    /// sphere; +0x84 / +0x88 (s32) the collision bits (moby +0x98) after a landing / a bounce; +0x8c the fall
    /// sphere; +0x90 the target's height offset; +0x94 (s32) the flight timer; +0x98 the pull speed; +0x9c / +0x9e
    /// (s16) the fire-out / bounce sounds.
    pub const SPEED_AFTER: usize = 0x78;
    pub const REGROW: usize = 0x7c;
    pub const FLY_R: usize = 0x80;
    pub const COLL_LAND: usize = 0x84;
    pub const COLL_BOUNCE: usize = 0x88;
    pub const FALL_R: usize = 0x8c;
    pub const TARGET_DZ: usize = 0x90;
    pub const TIMER: usize = 0x94;
    pub const PULL: usize = 0x98;
    pub const SOUND_FIRE: usize = 0x9c;
    pub const SOUND_BOUNCE: usize = 0x9e;
    /// The record's size (the chicken's global records are 0xb0 apart).
    pub const SIZE: usize = 0xb0;
}

/// `vel.w` of a fire-out that starts in the bounce state (`0x3044a0`'s 1.23456 marker).
pub const BOUNCE_MARK: f32 = 1.23456;

/// A class's reaction-table wrappers (the six slots; module doc). Each game wrapper is `r = handler(); if r == 0 {
/// if state != held { return 0 } state = 1 } else { state = held } return r` with these per-class differences.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrappers {
    /// The class's state while the Suck Cannon has it (577: 7, amoeboids: 0xe, chicken: 5, 749: 9).
    pub held: u8,
    /// The state a refused slot puts a held moby back in (`state = 1`; 749's level00 wrappers: 5).
    pub release: u8,
    /// Slot +0x00's extra gate (the amoeboids' 0x2efa88: only class 866 is taken, and not in its state 8).
    pub start_only: Option<(i16, u8)>,
    /// Slot +0x08: a class sound when the fire-out starts bouncing (577: 2, chicken: 1).
    pub bounce_sound: Option<i32>,
    /// The class's sequence table (+0x70 → gp−0x5190 / −0x51b0 / −0x51c0 / −0x5390, level01 0x161a70 / 0x161a50 /
    /// 0x161a40 / 0x161870): entries 1 approach, 2 rise, 3 pulled, 4 held, 5 fired, 6 bounce, 7 let go.
    pub seqs: [u8; 9],
    /// The suck record's pvar offset (slot +0x10: 577 `pvars + 0x60` 0x2f1df8, amoeboids `+0xc0` 0x2efbc8; the
    /// chicken's is its pvar +0x14 pointer, [`crate::moby_update::classes::chicken`]).
    pub record: usize,
    /// The wrappers that keep the state they took (568's level18 0x2d6108..): slot +0x00 takes only a moby in state
    /// `held − 1` or `held` (the state saved in moby +0xbc, returns 2; any other state: record state 0, refused), a
    /// refusal returns the moby to its saved state (instead of [`Wrappers::release`]), slot +0x0c leaves the state.
    pub saved: bool,
    /// Slot +0x14 returns the moby to its class's pool in this state instead of `DeleteMoby` ([`pool_park`]: 568's
    /// level18 0x2d6280 parks in 5, 1906's 0x2fcd00 in 8).
    pub pool: Option<u8>,
    /// Slot +0x0c after the let-go handler.
    pub let_go: LetGo,
    /// 1246's wrappers (level11 0x316160..): slot +0x00 keeps a taken moby only in these states (else returns 0, the
    /// record untouched), and every slot saves the state it replaces in moby +0xbc (empty: the other tables).
    pub take_states: &'static [u8],
    /// Slots +0x04 / +0x14 of 580's wrappers (level02 0x2d6210 / 0x2d6340): the nest count and the respawn.
    pub hooks: Hooks,
    /// A refusal's blend after [`Wrappers::release`]: `(seq, n)` → `fun_00212f90(m, seq, 0, ticks(n))` unless already
    /// on it (827's level06 wrappers: seq 0 over `ticks(10)`).
    pub release_seq: Option<(u8, i32)>,
}

/// Class code a table's wrappers run besides the shared handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hooks {
    None,
    /// 580: a swallow takes one off its nest's count; the delete slot respawns it
    /// ([`crate::moby_update::classes::units::aridia_sandshark`]).
    Aridia580,
    /// 238's wrappers (level12 0x2e16b8, 0x2e17f8): slot +0x00 refuses a burrower in its states 9, 0xb, 0xd before the
    /// pull and wakes its group (command 1) when it takes it; slot +0x0c leaves states 9 / 0xb as they are.
    Hoven238,
}

/// What a table's slot +0x0c does after the let-go handler [`let_go`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LetGo {
    /// `state = held` (the level-01 wrappers).
    Held,
    /// Nothing more (568's 0x2d6230).
    Keep,
    /// 252's 0x2d4050: back to its hover home ([`crate::moby_update::classes::units::hover_zapper::let_go`]).
    Hover252,
}

/// 577's table 0x20c3f4 (0x2f1c78, 0x2f1cc8, 0x2f1d18, 0x2f1dc8, 0x2f1df8).
pub const CRITTER: Wrappers = Wrappers { held: 7, release: 1, start_only: None, bounce_sound: Some(2), seqs: [2, 2, 2, 9, 4, 10, 10, 6, 4], record: 0x60, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::None, release_seq: None };
/// 866's table 0x20c40c and 572's 0x20c3dc (0x2efa88, 0x2efaf8, 0x2efb48, 0x2efb98, 0x2efbc8); the sequence table
/// by class (866 gp−0x51b0, else gp−0x51c0).
pub const AMOEBOID_866: Wrappers = Wrappers { held: 0xe, release: 1, start_only: Some((866, 8)), bounce_sound: None, seqs: [1, 1, 1, 6, 6, 6, 6, 6, 6], record: 0xc0, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::None, release_seq: None };
pub const AMOEBOID_572: Wrappers = Wrappers { seqs: [1; 9], ..AMOEBOID_866 };
/// 270's table (0x2e0a28, 0x2e0a78, 0x2e0ac8, 0x2e0b78, 0x2e0ba8); record: see the chicken module.
pub const CHICKEN: Wrappers = Wrappers { held: 5, release: 1, start_only: None, bounce_sound: Some(1), seqs: [2, 2, 2, 0, 0, 0, 0, 0, 0], record: 0xe0, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::None, release_seq: None };

/// 749's table (level00 0x1ea7ec: 0x2d56e0, 0x2d5730, 0x2d5780, 0x2d57d0, 0x2d5800; level18 0x1f34b0 the same code):
/// held 9, a refused slot → 5, no bounce sound, the sequence table gp−0x52a0 (level00 0x161960: 4, 4, 4, then 0),
/// the record at pvar +0xd0 (0x2d5800).
pub const VELDIN_749: Wrappers = Wrappers { held: 9, release: 5, start_only: None, bounce_sound: None, seqs: [4, 4, 4, 0, 0, 0, 0, 0, 0], record: 0xd0, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::None, release_seq: None };

/// `0x2defb0(target)`: the Morph-o-Ray's morph (a spawn and a delete: `crate::moby_update::classes::chicken::morph`).
pub fn morph_target(w: &mut World, target: MobyId) -> Option<MobyId> {
    let gold = w.svc.creatures.react.gold_morph;
    crate::moby_update::classes::chicken::morph(w, target, gold)
}

/// The level-01 reference tables: their slots +0x00, +0x08 and +0x0c (the wrappers that differ between the classes'
/// tables: the held state, the bounce sound, the let-go); [`tables_from_overlay`] matches a level's table by all three.
pub const REF_CRITTER: [u32; 3] = [0x2f1c78, 0x2f1d18, 0x2f1dc8];
pub const REF_AMOEBOID: [u32; 3] = [0x2efa88, 0x2efb48, 0x2efb98];
pub const REF_CHICKEN: [u32; 3] = [0x2e0a28, 0x2e0ac8, 0x2e0b78];
/// 568's table (level18 0x1f3498: 0x2d6108, 0x2d6190, 0x2d61e0, 0x2d6230, 0x2d6250, 0x2d6280): held 4, the saved
/// state (moby +0xbc) on a refusal, no bounce sound, the sequence table gp−0x5278 (level18 0x161988: nine 2s), the
/// record at pvar +0x60 (0x2d6250), slot +0x14 parks it ([`Wrappers::pool`]).
pub const ROLLING_MINE_568: Wrappers = Wrappers { held: 4, release: 4, start_only: None, bounce_sound: None, seqs: [2; 9], record: 0x60, saved: true, pool: Some(5), let_go: LetGo::Keep, take_states: &[], hooks: Hooks::None, release_seq: None };
/// The level-01 wrapper shape (held 7, a refusal → 1) without the bounce sound: 193's table (level09 0x209b00:
/// 0x2e2680, 0x2e26d0, 0x2e2720, 0x2e2770, 0x2e27a0 → pvar +0x60, `DeleteMoby`; level15 0x1e3e9c the same code) and
/// 1445's (level16 0x1e9050). The sequence table is the class's own (its init stores it in the record's +0x70:
/// [`SEQ_TABLES`]); these are the defaults.
pub const HELD7: Wrappers = Wrappers { held: 7, release: 1, start_only: None, bounce_sound: None, seqs: [0; 9], record: 0x60, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::None, release_seq: None };

/// 252's table (level08 0x1fd200: 0x2d3f60, 0x2d3fb0, 0x2d4000 (held 5, refused → 1), 0x2d4050 (the let-go:
/// [`LetGo::Hover252`]), 0x2d40d8 (+0x60), `DeleteMoby`; level14 0x1fd9d0 the same code); its sequence table gp−0x5378
/// (level08 0x161888: nine 2s).
pub const HOVER_252: Wrappers = Wrappers { held: 5, release: 1, start_only: None, bounce_sound: None, seqs: [2; 9], record: 0x60, saved: false, pool: None, let_go: LetGo::Hover252, take_states: &[], hooks: Hooks::None, release_seq: None };

/// 1246's table (level11 0x21c038: 0x316160, 0x316210, 0x316270, 0x3162d0, 0x3162f0 → pvar +0x60, `DeleteMoby`):
/// held 0x14; slot +0x00 keeps it only while it walks (states 2, 4, 5, 6, 0xe, 0xf, 0x10, 0x14), each slot saves the
/// state it replaces in moby +0xbc and a refusal returns to it; slot +0x0c only lets go; the sequence table gp−0x4940
/// (level11 0x1622c0: nine 0s) ([`crate::moby_update::classes::units::pokitaru_biter`]).
pub const POKITARU_1246: Wrappers = Wrappers { held: 0x14, release: 0x14, start_only: None, bounce_sound: None, seqs: [0; 9], record: 0x60, saved: true, pool: None, let_go: LetGo::Keep, take_states: &[2, 4, 5, 6, 0xe, 0xf, 0x10, 0x14], hooks: Hooks::None, release_seq: None };

/// 580's table (level02 0x1fb6dc: 0x2d61c0, 0x2d6210, 0x2d6290, 0x2d62e0, 0x2d6310 → pvar +0x60, 0x2d6340): the
/// level-01 shape with held 9 (refused → 1), the sequence table gp−0x5250 (level02 0x1619b0: 0, 0, 0, 10, 2, 11, 11,
/// 7, 2); the swallow takes the shark off its counter moby, the delete slot respawns it through a nest
/// ([`crate::moby_update::classes::units::aridia_sandshark`]).
pub const ARIDIA_580: Wrappers = Wrappers { held: 9, release: 1, start_only: None, bounce_sound: None, seqs: [0, 0, 0, 10, 2, 11, 11, 7, 2], record: 0x60, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::Aridia580, release_seq: None };

/// 827's table (level06 0x2023b4: 0x2ea5e0, 0x2ea6a8, 0x2ea738, 0x2ea7c8, 0x2ea800 → pvar +0x60, 0x2ec4d0
/// `DeleteMoby`): held 0x10; slot +0x00 takes it only in states 8, 9, 0xc, 0xd, 0x10; every taking slot saves the state
/// it replaces in moby +0xbc; a refusal → 8 with seq 0 over `ticks(10)`; the sequence table gp−0x5010 (level06
/// 0x161bf0: 4, 4, 4, 2, 2, 2, 2, 2, 2) ([`crate::moby_update::classes::units::blarg_crawler`]).
pub const BLARG_827: Wrappers = Wrappers { held: 0x10, release: 8, start_only: None, bounce_sound: None, seqs: [4, 4, 4, 2, 2, 2, 2, 2, 2], record: 0x60, saved: false, pool: None, let_go: LetGo::Held, take_states: &[8, 9, 0xc, 0xd, 0x10], hooks: Hooks::None, release_seq: Some((0, 10)) };

/// 238's table (level12 0x20d228: 0x2e16b8, 0x2e1758, 0x2e17a8, 0x2e17f8, 0x2e1848 → pvar +0x60, 0x2fbcc8
/// `DeleteMoby`): the level-01 shape with held 8 (refused → 1) and [`Hooks::Hoven238`]; its sequence table gp−0x5318
/// (level12 0x1618e8: 3, 3, 3, 10, 11, 11, 11, 11, 11) ([`crate::moby_update::classes::units::hoven_burrower`]).
pub const HOVEN_238: Wrappers = Wrappers { held: 8, release: 1, start_only: None, bounce_sound: None, seqs: [3, 3, 3, 10, 11, 11, 11, 11, 11], record: 0x60, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::Hoven238, release_seq: None };

/// 340's table (level04 0x1dd11c: 0x2c4030, 0x2c4080, 0x2c40d0 (bounce: class sound 1), 0x2c4180, 0x2c41b0 →
/// pvar +0x180, 0x2ddbf8 `DeleteMoby`): the level-01 shape with held 9 (refused → 1); the sequence table is the
/// class's own ([`SEQS_340`], stored by its init) ([`crate::moby_update::classes::units::eudora_brawler`]).
pub const EUDORA_340: Wrappers = Wrappers { held: 9, release: 1, start_only: None, bounce_sound: Some(1), seqs: SEQ_TABLES[SEQS_340], record: 0x180, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::None, release_seq: None };

/// 1382's table (level17 0x1e7854: 0x2efea8, 0x2eff30, 0x2effb8, 0x2f0040, 0x2f0070 → pvar +0x60, 0x2de0b0
/// `DeleteMoby`): held 0xc; a refusal while held → 6 with seq 7 over `ticks(10)`; the sequence table gp−0x4a00
/// (level17 0x162200: 9, 9, 9, 11, 12, 11, 11, 11, 2) ([`crate::moby_update::classes::units::fleet_crew`]).
pub const FLEET_1382: Wrappers = Wrappers { held: 0xc, release: 6, start_only: None, bounce_sound: None, seqs: [9, 9, 9, 11, 12, 11, 11, 11, 2], record: 0x60, saved: false, pool: None, let_go: LetGo::Held, take_states: &[], hooks: Hooks::None, release_seq: Some((7, 10)) };

/// 1906's table (level18 0x1f34c8: 0x2fcbb0, 0x2fcc00, 0x2fcc50, 0x2fcca0, 0x2fccd0 → pvar +0xd0, 0x2fcd00 parks it
/// in 8): the level-01 shape with held 7 (refused → 2); the sequence table is the walker's gp−0x4610 its init stores
/// (level18 0x1625f0: 1, 1, 1, 4, 4, 4, 4, 4, 4) ([`crate::moby_update::classes::units::veldin_hopper`]).
pub const VELDIN_1906: Wrappers = Wrappers { held: 7, release: 2, start_only: None, bounce_sound: None, seqs: [1, 1, 1, 4, 4, 4, 4, 4, 4], record: 0xd0, saved: false, pool: Some(8), let_go: LetGo::Held, take_states: &[], hooks: Hooks::None, release_seq: None };

/// The reaction tables reversed on other levels: (level, its slots +0x00 / +0x08 / +0x0c there, the table).
pub const OTHER_REFS: [(u32, [u32; 3], Table); 11] = [
    (0, [0x2d56e0, 0x2d5780, 0x2d57d0], Table::Veldin749),
    (18, [0x2d6108, 0x2d61e0, 0x2d6230], Table::RollingMine568),
    (9, [0x2e2680, 0x2e2720, 0x2e2770], Table::Held7),
    (8, [0x2d3f60, 0x2d4000, 0x2d4050], Table::Hover252),
    (11, [0x316160, 0x316270, 0x3162d0], Table::Pokitaru1246),
    (2, [0x2d61c0, 0x2d6290, 0x2d62e0], Table::Aridia580),
    (6, [0x2ea5e0, 0x2ea738, 0x2ea7c8], Table::Blarg827),
    (12, [0x2e16b8, 0x2e17a8, 0x2e17f8], Table::Hoven238),
    (4, [0x2c4030, 0x2c40d0, 0x2c4180], Table::Eudora340),
    (17, [0x2efea8, 0x2effb8, 0x2f0040], Table::Fleet1382),
    (18, [0x2fcbb0, 0x2fcc50, 0x2fcca0], Table::Veldin1906),
];

/// The classes' sequence tables as their inits store them in the suck record's +0x70 (the game: a pointer to the
/// class's gp table; the port: [`seq_table_id`] of the entry here). Entries 1 approach, 2 rise, 3 pulled, 4 held,
/// 5 fired, 6 bounce, 7 let go. 193: level09 gp−0x5328 (0x1618d8).
/// 63: level13 gp−0x5860 (0x1613a0). 340: level04 gp−0x5388 (0x161878).
pub const SEQ_TABLES: [[u8; 9]; 3] = [[0, 0, 0, 7, 2, 8, 8, 6, 2], [5, 5, 5, 7, 3, 8, 8, 2, 3], [1, 1, 1, 9, 0, 11, 11, 10, 0]];
pub const SEQS_193: usize = 0;
pub const SEQS_63: usize = 1;
pub const SEQS_340: usize = 2;
/// 1445's table (level16 gp−0x4db8, 0x161e48) holds the same bytes as 193's.
pub const SEQS_1445: usize = SEQS_193;

/// The record +0x70 word for [`SEQ_TABLES`] entry `i` (a tag the instance data never holds, so a record whose class
/// init did not store one keeps its table's default [`Wrappers::seqs`]).
pub fn seq_table_id(i: usize) -> i32 { 0x5e05_0000 | i as i32 }

/// A ported reaction table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Table {
    Critter,
    Amoeboid,
    Chicken,
    /// 749 (levels 00, 18: [`VELDIN_749`]).
    Veldin749,
    /// 568 (level 18: [`ROLLING_MINE_568`]).
    RollingMine568,
    /// The shape [`HELD7`] (193 on 09 / 15, 1445 on 16).
    Held7,
    /// 252 (levels 08, 14: [`HOVER_252`]).
    Hover252,
    /// 1246 (level 11: [`POKITARU_1246`]).
    Pokitaru1246,
    /// 580 (level 02: [`ARIDIA_580`]).
    Aridia580,
    /// 827 (levels 06, 10: [`BLARG_827`]).
    Blarg827,
    /// 238 (level 12: [`HOVEN_238`]).
    Hoven238,
    /// 340 (level 04: [`EUDORA_340`]).
    Eudora340,
    /// 1382 (level 17: [`FLEET_1382`]).
    Fleet1382,
    /// 1906 (level 18: [`VELDIN_1906`]).
    Veldin1906,
}

/// The Suck Cannon as its last update left it (module doc).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cannon {
    pub pos: [f32; 3],
    /// Joint list 0 (`FUN_002645a8(cannon, 0)`).
    pub mouth: [f32; 3],
    /// The item's rows (+0xc0).
    pub rows: [[f32; 3]; 3],
}

/// The reaction layer's globals (in `creature::Globals`).
#[derive(Clone, Debug, Default)]
pub struct Globals {
    /// The level's ported reaction tables by class (from its `lvl.vtbl`; empty: by class number, [`table`]).
    pub tables: std::collections::HashMap<i16, Table>,
    /// 0x1413a0: the Suck Cannon's slots (moby + 1; 0 empty).
    pub slots: [u32; 10],
    /// 0x1413c8 / 0x1413cc: held, coming.
    pub held: i32,
    pub coming: i32,
    /// 0x13e529: the gold Suck Cannon (mirrored by the engine every tick).
    pub gold: bool,
    /// A write of the swap lock 0x1403fc by the moby side (the cannon's next update copies it to the hand slot [L]).
    pub swap_lock: Option<u8>,
    /// The cannon ([`Cannon`]); None: no Suck Cannon in the hand.
    pub cannon: Option<Cannon>,
    /// The cannon's `fun_00212f90(cannon, 4, 0, ticks(2))` from a carried update (made by its next update).
    pub cannon_seq4: bool,
    /// Ratchet's moby (the port's owner of the hand item; the records' +0x60).
    pub hero: Option<MobyId>,
    /// 0x1dd580[20] and gp−0x537c: the Morph-o-Ray's chickens (moby + 1) and the next slot
    /// (`crate::moby_update::classes::chicken`).
    pub chickens: [u32; 20],
    pub chicken_next: usize,
    /// 0x13e535: the gold Morph-o-Ray (mirrored by the engine every tick; the chickens' and feathers' gold size read it).
    pub gold_morph: u8,
    /// The cannon's class sound 5 asked for by a swallow (played by its next update).
    pub cannon_sound5: u32,
    /// Suck bursts made (stats).
    pub bursts: u32,
}

impl Globals {
    /// The slot count (5, 10 gold).
    pub fn limit(&self) -> usize { if self.gold { 10 } else { 5 } }
}

/// Class-number fallback of [`table`] (the level-01 users; the level's own table when the engine filled it).
pub fn table_by_class(o_class: i16) -> Option<Table> {
    match o_class {
        577 => Some(Table::Critter),
        572 | 866 => Some(Table::Amoeboid),
        270 => Some(Table::Chicken),
        _ => None,
    }
}

/// The level's class table's reaction tables: for each class whose table's slots +0x00, +0x08 and +0x0c are the same
/// code as a ported reference's (level 01's [`REF_CRITTER`] / [`REF_AMOEBOID`] / [`REF_CHICKEN`]), that table.
pub fn tables_from_overlay(target: &rc_formats::level_overlay::LevelOverlay, level01: &rc_formats::level_overlay::LevelOverlay) -> std::collections::HashMap<i16, Table> {
    let rel = rc_formats::level_overlay::Relocation::new(level01, target);
    let mut out = std::collections::HashMap::new();
    for e in target.vtbl() {
        for (r, t) in [(REF_CRITTER, Table::Critter), (REF_AMOEBOID, Table::Amoeboid), (REF_CHICKEN, Table::Chicken)] {
            if same_table(&rel, level01, target, r, e.w8) { out.insert(e.o_class as i16, t); }
        }
    }
    out
}

/// [`tables_from_overlay`] plus the tables reversed on other levels ([`OTHER_REFS`]; `reference(level)` gives their
/// overlays): what the engine loads for a level.
pub fn tables_from_overlays(target: &rc_formats::level_overlay::LevelOverlay, level01: &rc_formats::level_overlay::LevelOverlay, reference: &dyn Fn(u32) -> Option<std::sync::Arc<rc_formats::level_overlay::LevelOverlay>>) -> std::collections::HashMap<i16, Table> {
    let mut out = tables_from_overlay(target, level01);
    for (level, r, t) in OTHER_REFS {
        let Some(ov) = reference(level) else { continue };
        let rel = rc_formats::level_overlay::Relocation::new(&ov, target);
        for e in target.vtbl() {
            if same_table(&rel, &ov, target, r, e.w8) { out.insert(e.o_class as i16, t); }
        }
    }
    out
}

/// Whether the target table at `w8` has slots +0x00, +0x08, +0x0c of the same code as the reference's `r`.
fn same_table(rel: &rc_formats::level_overlay::Relocation, a: &rc_formats::level_overlay::LevelOverlay, b: &rc_formats::level_overlay::LevelOverlay, r: [u32; 3], w8: u32) -> bool {
    [0u32, 2, 3].iter().zip(r).all(|(&k, ra)| b.u32(w8 + 4 * k).is_some_and(|tb| same_small(rel, a, b, ra, tb)))
}

/// The slot functions are reached only through the tables, so a function extent may be unknown: compare 16 masked
/// words when [`rc_formats::level_overlay::Relocation::same_code`] cannot tell.
fn same_small(rel: &rc_formats::level_overlay::Relocation, a: &rc_formats::level_overlay::LevelOverlay, b: &rc_formats::level_overlay::LevelOverlay, ra: u32, tb: u32) -> bool {
    if rel.same_code(ra, tb) { return true; }
    match (a.code(ra, 16), b.code(tb, 16)) {
        (Some(x), Some(y)) => rc_formats::level_overlay::mask(x) == rc_formats::level_overlay::mask(y),
        _ => false,
    }
}

/// The reaction table of moby `id`'s class (None: the default table, every slot 0).
pub fn table(w: &World, id: MobyId) -> Option<Table> {
    let o = w.m(id).o_class;
    let g = &w.svc.creatures.react;
    if g.tables.is_empty() { table_by_class(o) } else { g.tables.get(&o).copied() }
}

/// The wrappers of moby `id` (its table and class).
pub fn wrappers(w: &World, id: MobyId) -> Option<Wrappers> {
    Some(match table(w, id)? {
        Table::Critter => CRITTER,
        Table::Amoeboid => if w.m(id).o_class == 866 { AMOEBOID_866 } else { AMOEBOID_572 },
        Table::Chicken => CHICKEN,
        Table::Veldin749 => VELDIN_749,
        Table::RollingMine568 => ROLLING_MINE_568,
        Table::Held7 => HELD7,
        Table::Hover252 => HOVER_252,
        Table::Pokitaru1246 => POKITARU_1246,
        Table::Aridia580 => ARIDIA_580,
        Table::Blarg827 => BLARG_827,
        Table::Hoven238 => HOVEN_238,
        Table::Eudora340 => EUDORA_340,
        Table::Fleet1382 => FLEET_1382,
        Table::Veldin1906 => VELDIN_1906,
    })
}

/// Slot +0x10 through `0x304100`: the suck record's pvar offset (None: deleted, no pvars, the default table, or the
/// record does not fit).
pub fn record(w: &World, id: MobyId) -> Option<usize> {
    let m = w.m(id);
    if m.state == crate::moby_runtime::state::DELETED || m.state == crate::moby_runtime::state::DELETED_STATIC || m.pvars.is_empty() || !m.has_class { return None; }
    let r = wrappers(w, id)?.record;
    (r + rec::SIZE <= m.pvars.len()).then_some(r)
}

// --- record field access
fn rf(w: &World, r: usize, id: MobyId, o: usize) -> f32 { super::pf(w, id, r + o) }
fn set_rf(w: &mut World, r: usize, id: MobyId, o: usize, x: f32) { super::set_pf(w, id, r + o, x) }
fn rs(w: &World, r: usize, id: MobyId, o: usize) -> i16 { super::pi16(w, id, r + o) }
fn set_rs(w: &mut World, r: usize, id: MobyId, o: usize, x: i16) { super::set_pi16(w, id, r + o, x) }
fn ri(w: &World, r: usize, id: MobyId, o: usize) -> i32 { super::pi32(w, id, r + o) }
fn set_ri(w: &mut World, r: usize, id: MobyId, o: usize, x: i32) { super::set_pi32(w, id, r + o, x) }
fn rv(w: &World, r: usize, id: MobyId, o: usize) -> V { super::pv4(w, id, r + o) }
fn set_rv(w: &mut World, r: usize, id: MobyId, o: usize, x: V) { super::set_pv4(w, id, r + o, x) }

/// The record's state (+0x68).
pub fn rec_state(w: &World, id: MobyId) -> Option<i16> { record(w, id).map(|r| rs(w, r, id, rec::STATE)) }

fn moby_ref(w: &World, r: usize, id: MobyId, o: usize) -> Option<MobyId> {
    let v = ri(w, r, id, o);
    (v > 0).then(|| (v - 1) as MobyId).filter(|&m| m < w.table.mobys.len())
}

/// The handlers' "gone" test (`state == −2 || state == −3`: deleted, dynamic or static; not every state ≥ 0x80).
fn dead(w: &World, id: MobyId) -> bool {
    let s = w.m(id).state;
    s == crate::moby_runtime::state::DELETED || s == crate::moby_runtime::state::DELETED_STATIC
}

/// `gp−0x4d60` (level01 0x161ea0): the swallow's Euler (−π/2, 0, −π/2); `gp−0x4d50` (0x161eb0): the fire's (π/2, 0, π/2).
pub const SWALLOW_EULER: [f32; 3] = [-PI / 2.0, 0.0, -PI / 2.0];
pub const FIRE_EULER: [f32; 3] = [PI / 2.0, 0.0, PI / 2.0];

/// `MobyAnimSphereLerp` 0x265d78 (boot `fun_0020e098`, what every handler here calls after moving or turning a moby):
/// `MobyBuildMatrix` without the rows from the Euler angles — the stored rows stay (row 1 negated with mode 0x8000,
/// as the build does on kept rows), the bounding sphere, the change counter, the grid.
pub fn sphere_lerp(w: &mut World, id: MobyId) {
    let m = w.m(id).mode;
    w.mm(id).mode |= mode::KEEP_ROWS;
    w.build_matrix(id);
    w.mm(id).mode = m;
}

/// `FUN_00302840(cannon, moby, euler)`: the moby's rows = `Euler(euler)` in the cannon's rows (the product order as
/// the pull's orientation target [L]), mode |= 4 (kept matrix), its matrix built.
pub fn cannon_frame(w: &mut World, id: MobyId, euler: [f32; 3]) {
    if let Some(c) = w.svc.creatures.react.cannon {
        set_rows3(w, id, mul_rows(euler_rows(euler), c.rows));
    }
    w.mm(id).mode |= mode::KEEP_MATRIX;
    sphere_lerp(w, id);
}

/// Entry `k` of the class's sequence table: the one its init stored in the record's +0x70 ([`SEQ_TABLES`]), else its
/// table's default.
fn seq(w: &World, id: MobyId, k: usize) -> u8 {
    let own = record(w, id).map(|r| ri(w, r, id, rec::SEQS)).and_then(|v| SEQ_TABLES.iter().enumerate().find(|(i, _)| seq_table_id(*i) == v).map(|(_, t)| t[k]));
    own.unwrap_or_else(|| wrappers(w, id).map_or(0, |x| x.seqs[k]))
}

/// `fun_00212f90(moby, seq, 0, ticks(n))` behind the game's `if (m+0x53 != seq)`.
fn blend(w: &mut World, id: MobyId, s: u8, n: i32) {
    let t = w.ticks(n);
    super::blend_to(w, id, s, 0, t);
}

fn class_scale(w: &World, id: MobyId) -> f32 { w.class_scale(w.m(id).o_class).to_f32() }

/// The pool tables' slot +0x14 (level18 0x2d6280 / 0x2fcd00, one body): `state`, blend to 0 (`ticks(10)`) when not on
/// it, +0x31 = 0, +0x94 = 0, mode & ~0x1000 | 1.
pub fn pool_park(w: &mut World, id: MobyId, state: u8) {
    w.mm(id).state = state;
    if w.m(id).anim.seq_b != 0 {
        let t = w.ticks(10);
        w.anim_blend(id, 0, 0, t);
    }
    let m = w.mm(id);
    m.visible = 0;
    m.has_collision = false;
    m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN;
}

// ------------------------------------------------------------------------------------------------------------------
// The slots (module doc), dispatched on the moby's table

/// Slot +0x00 `(moby, mouth, cannon)`: the suck start (0 no, 1 coming, 2 taken, 3 held).
pub fn slot_start(w: &mut World, id: MobyId, mouth: V) -> i32 {
    let Some(x) = wrappers(w, id) else { return 0 };
    if x.hooks == Hooks::Hoven238 && [9, 0xb, 0xd].contains(&w.m(id).state) { return 0; }
    let r = approach(w, id, mouth);
    if r == 0 {
        refuse(w, id, &x);
        return 0;
    }
    if !x.take_states.is_empty() {
        let s = w.m(id).state;
        if !x.take_states.contains(&s) { return 0; }
        if s != x.held {
            w.mm(id).state = x.held;
            w.mm(id).cmd = s;
        }
        return r;
    }
    if x.saved {
        let s = w.m(id).state;
        if s.wrapping_sub(x.held.wrapping_sub(1)) < 2 {
            if s != x.held {
                w.mm(id).state = x.held;
                w.mm(id).cmd = s;
            }
            return 2;
        }
        if let Some(rr) = record(w, id) { set_rs(w, rr, id, rec::STATE, 0); }
        return 0;
    }
    if let Some((only, not_in)) = x.start_only {
        if w.m(id).o_class != only || w.m(id).state == not_in { return 0; }
    }
    if x.hooks == Hooks::Hoven238 {
        let g = w.m(id).group;
        if g != -1 { crate::moby_update::scheduler::group_cmd(w, g, 1); }
    }
    w.mm(id).state = x.held;
    r
}

/// Slot +0x04 `(moby, point)`: swallowed.
pub fn slot_swallow(w: &mut World, id: MobyId, point: V) -> i32 {
    let Some(x) = wrappers(w, id) else { return 1 };
    let r = swallow(w, id, point);
    let r = wrap_state(w, id, &x, r);
    if r != 0 && x.hooks == Hooks::Aridia580 { crate::moby_update::classes::units::aridia_sandshark::swallowed(w, id); }
    r
}

/// Slot +0x08 `(height, moby, vel, target)`: fired out of the cannon.
pub fn slot_fire(w: &mut World, id: MobyId, height: f32, vel: V, target: Option<MobyId>) -> i32 {
    let Some(x) = wrappers(w, id) else { return 0 };
    let r = fire_out(w, id, height, vel, target);
    let r2 = wrap_state(w, id, &x, r);
    if let (Some(s), Some(rr)) = (x.bounce_sound, record(w, id)) {
        if rs(w, rr, id, rec::STATE) == 6 { w.play_sound(s, 0, id); }
    }
    r2
}

/// Slot +0x0c `(moby)`: let go (the class's held state stays).
pub fn slot_let_go(w: &mut World, id: MobyId) {
    let Some(x) = wrappers(w, id) else { return };
    let_go(w, id);
    if x.hooks == Hooks::Hoven238 && [9, 0xb].contains(&w.m(id).state) { return; }
    match x.let_go {
        LetGo::Held => w.mm(id).state = x.held,
        LetGo::Keep => {}
        LetGo::Hover252 => crate::moby_update::classes::units::hover_zapper::let_go(w, id),
    }
}

/// A refused slot: a held moby back to its saved state or [`Wrappers::release`] (and [`Wrappers::release_seq`]).
fn refuse(w: &mut World, id: MobyId, x: &Wrappers) {
    if w.m(id).state != x.held { return; }
    w.mm(id).state = if x.saved { w.m(id).cmd } else { x.release };
    if let Some((seq, n)) = x.release_seq {
        let t = w.ticks(n);
        super::blend_to(w, id, seq, 0, t);
    }
}

fn wrap_state(w: &mut World, id: MobyId, x: &Wrappers, r: i32) -> i32 {
    if r == 0 {
        refuse(w, id, x);
        return 0;
    }
    let s = w.m(id).state;
    if !x.take_states.is_empty() && s != x.held { w.mm(id).cmd = s; }
    w.mm(id).state = x.held;
    r
}

/// Slot +0x14: `DeleteMoby` (but the tables that park it: [`Wrappers::pool`]).
pub fn slot_delete(w: &mut World, id: MobyId) {
    if let Some(state) = wrappers(w, id).and_then(|x| x.pool) {
        pool_park(w, id, state);
    } else if wrappers(w, id).is_some_and(|x| x.hooks == Hooks::Aridia580) {
        crate::moby_update::classes::units::aridia_sandshark::respawn(w, id);
    } else {
        w.delete_moby(id);
    }
}

// ------------------------------------------------------------------------------------------------------------------
// The shared handlers

/// `0x304168(moby, mouth, cannon)`: the cannon pulls (module doc; returns 1 coming, 2 taken, 3 held).
pub fn approach(w: &mut World, id: MobyId, mouth: V) -> i32 {
    let Some(r) = record(w, id) else { return 0 };
    if rs(w, r, id, rec::STATE) == 8 { return 0; }
    set_rv(w, r, id, rec::POINT, [mouth[0], mouth[1], mouth[2], 1.0]);
    let hero = w.svc.creatures.react.hero.map_or(0, |h| h as i32 + 1);
    set_ri(w, r, id, rec::CANNON, hero);
    let t = rf(w, r, id, rec::T);
    match rs(w, r, id, rec::STATE) {
        1 => {
            if t < 1.0 {
                let s = seq(w, id, 1);
                blend(w, id, s, 10);
                return 1;
            }
            let s = seq(w, id, 2);
            blend(w, id, s, 10);
            set_rf(w, r, id, rec::T, 0.0);
            set_rs(w, r, id, rec::STATE, 2);
            1
        }
        2 => if t < 1.0 { 1 } else { 2 },
        3 => 2,
        4 => 3,
        5..=7 => {
            let s = seq(w, id, 3);
            blend(w, id, s, 10);
            set_rs(w, r, id, rec::STATE, 3);
            set_rf(w, r, id, rec::T, 0.0);
            w.mm(id).anim.speed = 1.0;
            let c = w.svc.creatures.react.cannon.map_or([0.0; 3], |c| c.pos);
            let d = super::dist3(super::pos(w, id), [c[0], c[1], c[2], 0.0]);
            set_rf(w, r, id, rec::DIST0, d);
            2
        }
        _ => {
            let s = seq(w, id, 1);
            blend(w, id, s, 10);
            set_rf(w, r, id, rec::T, 0.0);
            set_rs(w, r, id, rec::STATE, 1);
            1
        }
    }
}

/// `0x3028c8(cannon, pvars, moby, rec, mouth)`: slot +0x00, then a taken moby not yet pulled starts its pull (state 3,
/// the class's sequence 3, the distance to the mouth) in the first free slot, when fewer than the limit are held and
/// coming. Returns the slot's result.
pub fn take(w: &mut World, id: MobyId, mouth: V) -> i32 {
    let r0 = slot_start(w, id, mouth);
    let Some(r) = record(w, id) else { return r0 };
    if r0 != 2 || rs(w, r, id, rec::STATE) == 3 { return r0; }
    let g = &w.svc.creatures.react;
    if g.limit() as i32 - 1 < g.held + g.coming { return 2; }
    let s = seq(w, id, 3);
    blend(w, id, s, 10);
    set_rs(w, r, id, rec::STATE, 3);
    set_rf(w, r, id, rec::T, 0.0);
    w.mm(id).anim.speed = 1.0;
    let d = super::dist3(super::pos(w, id), mouth);
    set_rf(w, r, id, rec::DIST0, d);
    let limit = w.svc.creatures.react.limit();
    let Some(i) = (0..limit).find(|&i| w.svc.creatures.react.slots[i] == 0) else { return 2 };
    w.svc.creatures.react.slots[i] = id as u32 + 1;
    set_rs(w, r, id, rec::SLOT, i as i16);
    w.svc.creatures.react.coming += 1;
    r0
}

/// `0x304390(moby, point)`: swallowed at `point` (kept matrix, the cannon's sound 5 when the hand holds the Suck
/// Cannon, `SetDeathBits`, record state 4, the class's sequence 4, scale 0).
pub fn swallow(w: &mut World, id: MobyId, point: V) -> i32 {
    let Some(r) = record(w, id) else { return 0 };
    if rs(w, r, id, rec::STATE) == 8 { return 0; }
    super::set_pos(w, id, point);
    // FUN_00302840(record +0x60, moby, gp−0x4d60): the cannon's rows turned by (−π/2, 0, −π/2), then the matrix again.
    cannon_frame(w, id, SWALLOW_EULER);
    sphere_lerp(w, id);
    // `if (0x1403e0 && its class == 0x351) fun_0022da68(5, 0)`: the cannon's sound 5 (made by its next update).
    if w.svc.creatures.react.cannon.is_some() { w.svc.creatures.react.cannon_sound5 += 1; }
    crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
    set_rs(w, r, id, rec::STATE, 4);
    let s = seq(w, id, 4);
    blend(w, id, s, 10);
    w.mm(id).scale = 0.0;
    1
}

/// `0x3044a0(height, moby, vel, target)`: fired (record state 5, or 6 = bouncing with the [`BOUNCE_MARK`]); the
/// flight timer `ticks(300)`, the target (its damage record +0x1e |= 0x80, its point + `height` for the homing).
pub fn fire_out(w: &mut World, id: MobyId, height: f32, vel: V, target: Option<MobyId>) -> i32 {
    let Some(r) = record(w, id) else { return 0 };
    if rs(w, r, id, rec::STATE) == 8 { return 0; }
    w.mm(id).visible = 1;
    w.mm(id).mode &= !(mode::HIDDEN | mode::NO_UPDATE);
    if vel[3] == BOUNCE_MARK {
        let s = seq(w, id, 6);
        blend(w, id, s, 20);
        let c = ri(w, r, id, rec::COLL_BOUNCE);
        w.mm(id).coll_disable = c as u32;
        set_rs(w, r, id, rec::STATE, 6);
    } else {
        let s = seq(w, id, 5);
        blend(w, id, s, 0);
        set_rs(w, r, id, rec::STATE, 5);
    }
    set_rv(w, r, id, rec::VEL, vel);
    let t = w.ticks(300);
    set_ri(w, r, id, rec::TIMER, t);
    set_ri(w, r, id, rec::TARGET, target.map_or(0, |t| t as i32 + 1));
    if let Some(t) = target {
        if let Some(d) = super::header(w, t).damage {
            let f = super::pu8(w, t, d + 0x1e) | 0x80;
            super::set_pu8(w, t, d + 0x1e, f);
        }
        let mut p = super::pos(w, t);
        set_rf(w, r, id, rec::TARGET_DZ, height);
        p[2] += height;
        set_rv(w, r, id, rec::TARGET_AT, p);
    }
    let hp = w.hero.pos.map(|x| f32::from_bits(x.0));
    if super::dist3(super::pos(w, id), [hp[0], hp[1], hp[2], 0.0]) < 1.0 {
        let p = super::add(super::pos(w, id), [vel[0], vel[1], vel[2], 0.0]);
        super::set_pos(w, id, p);
    }
    let s = rs(w, r, id, rec::SOUND_FIRE);
    if s >= 0 { w.play_sound(s as i32, 0, id); }
    2
}

/// `0x304690(moby)`: let go: a pulled one leaves its slot; approaching / rising / pulled → record state 7 (the
/// class's sequence 7). Returns 1 for a held one.
pub fn let_go(w: &mut World, id: MobyId) -> i32 {
    let Some(r) = record(w, id) else { return 0 };
    let st = rs(w, r, id, rec::STATE);
    match st {
        1..=3 => {
            if st == 3 {
                let slot = rs(w, r, id, rec::SLOT);
                if slot != -1 {
                    if let Some(s) = w.svc.creatures.react.slots.get_mut(slot as usize) { *s = 0; }
                    if 0 < w.svc.creatures.react.coming { w.svc.creatures.react.coming -= 1; }
                }
            }
            let s = seq(w, id, 7);
            blend(w, id, s, 10);
            set_rs(w, r, id, rec::STATE, 7);
            0
        }
        4 => 1,
        _ => 0,
    }
}

// --- small native math
fn dot(a: V, b: V) -> f32 { super::dot3(a, b) }
fn len2sq(v: V) -> f32 { dot(v, v) }
fn cross(a: V, b: V) -> V { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0], 0.0] }
fn rows3(w: &World, id: MobyId) -> [[f32; 3]; 3] {
    let m = w.m(id);
    std::array::from_fn(|i| [m.rows[i][0], m.rows[i][1], m.rows[i][2]])
}
fn set_rows3(w: &mut World, id: MobyId, r: [[f32; 3]; 3]) {
    let m = w.mm(id);
    for (i, row) in r.iter().enumerate() { m.rows[i] = [row[0], row[1], row[2], m.rows[i][3]]; }
}
/// The rows of a unit quaternion (the layout of 0x20eb2c's rows).
pub fn quat_rows(q: [f32; 4]) -> [[f32; 3]; 3] {
    let [x, y, z, w] = q;
    [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w)],
        [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w)],
        [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y)],
    ]
}
/// The inverse of [`quat_rows`] (`FUN_00272028` / `fun_00214260`).
pub fn rows_quat(m: [[f32; 3]; 3]) -> [f32; 4] {
    let tr = m[0][0] + m[1][1] + m[2][2];
    let q = if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        [(m[2][1] - m[1][2]) / s, (m[0][2] - m[2][0]) / s, (m[1][0] - m[0][1]) / s, 0.25 * s]
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        [0.25 * s, (m[0][1] + m[1][0]) / s, (m[0][2] + m[2][0]) / s, (m[2][1] - m[1][2]) / s]
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        [(m[0][1] + m[1][0]) / s, 0.25 * s, (m[1][2] + m[2][1]) / s, (m[0][2] - m[2][0]) / s]
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        [(m[0][2] + m[2][0]) / s, (m[1][2] + m[2][1]) / s, 0.25 * s, (m[1][0] - m[0][1]) / s]
    };
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if n == 0.0 { [0.0, 0.0, 0.0, 1.0] } else { q.map(|c| c / n) }
}
/// `fun_001fa400(t, out, a, b)`: the spherical blend of two orientations (shortest arc) [L: slerp].
pub fn slerp(t: f32, a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let mut d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let b = if d < 0.0 { d = -d; b.map(|c| -c) } else { b };
    let (ka, kb) = if d > 0.9995 { (1.0 - t, t) } else {
        let th = d.clamp(-1.0, 1.0).acos();
        let s = th.sin();
        (((1.0 - t) * th).sin() / s, (t * th).sin() / s)
    };
    let q: [f32; 4] = std::array::from_fn(|i| a[i] * ka + b[i] * kb);
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    q.map(|c| c / n)
}
fn mul_rows(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j]))
}
fn euler_rows(e: [f32; 3]) -> [[f32; 3]; 3] {
    let r = rc_formats::moby_light::rotation_rows(e);
    std::array::from_fn(|i| [f32::from_bits(r[i][0]), f32::from_bits(r[i][1]), f32::from_bits(r[i][2])])
}

/// `0x305260(moby, knock)`: the update of a moby the Suck Cannon has (its class calls it from its held state;
/// `knock` = the class's knockback record, whose +0x44 turn velocity and +0x84 collision bits it uses). Returns 1
/// when a let-go moby has landed (the class resumes).
pub fn carried(w: &mut World, id: MobyId, knock: usize) -> i32 {
    if dead(w, id) { return 0; }
    let Some(r) = record(w, id) else { return tail_none(w, id) };
    let p = super::pos(w, id);
    set_rv(w, r, id, rec::POS, p);
    let st = rs(w, r, id, rec::STATE);
    match st {
        1 => {
            // Turn away from the mouth, then rise.
            let pt = rv(w, r, id, rec::POINT);
            let a = super::add_rot(super::atan(pt[0] - p[0], pt[1] - p[1]), PI);
            // The game's literals (0.19634955, 0.3927: not π/16 / π/8 exactly).
            #[allow(clippy::approx_constant)]
            super::turn::spring_turn2_pvar(w, id, a, 0.196_349_55, 0.196_349_55, 0.3927, knock + 0x44);
            let yaw = super::yaw(w, id);
            if super::sub_rot(a, 0.01) < yaw && yaw < super::add_rot(a, 0.01) {
                set_rf(w, r, id, rec::T, 0.0);
                set_rs(w, r, id, rec::STATE, 2);
            }
            set_rv(w, r, id, rec::VEL, [0.0; 4]);
        }
        2 => {
            let t = rf(w, r, id, rec::T);
            w.mm(id).anim.speed = t + 1.0;
            let t = (t + 1.0 / w.ticks(1) as f32).min(1.0);
            set_rf(w, r, id, rec::T, t);
            let mut q = super::pos(w, id);
            q[2] += t / 100.0;
            super::set_pos(w, id, q);
            let quat = rows_quat(rows3(w, id));
            set_rv(w, r, id, rec::QUAT, quat);
            set_rv(w, r, id, rec::VEL, [0.0; 4]);
        }
        3 => pulled(w, id, r),
        4 => {
            w.mm(id).visible = 0;
            w.mm(id).mode = (w.m(id).mode & !mode::KEEP_MATRIX) | mode::HIDDEN | mode::NO_UPDATE;
            w.mm(id).has_collision = false;
            if w.m(id).anim.flags & 2 != 0 {
                let s = seq(w, id, 4);
                blend(w, id, s, 3);
                let sc = class_scale(w, id) * 0.3;
                w.mm(id).scale = sc;
                w.mm(id).rotation = [0.0; 4];
            }
        }
        5 | 6 => return flight(w, id, r),
        7 => {
            w.mm(id).visible = 1;
            w.mm(id).mode &= !mode::HIDDEN;
            let mut v = rv(w, r, id, rec::VEL);
            v[2] -= 0.013;
            set_rv(w, r, id, rec::VEL, v);
            if super::pos(w, id)[2] < 2.0 {
                land(w, id, r, true);
                return 0;
            }
            let fr = (rf(w, r, id, rec::FALL_R) * 1024.0).trunc() / 1024.0;
            let res = super::walker::move_collide(w, id, 0.8, fr, 0.0, &mut v, 0);
            set_rv(w, r, id, rec::VEL, v);
            let rot = w.m(id).rotation;
            w.mm(id).rotation[0] = rot[0] / 1.5;
            w.mm(id).rotation[1] = rot[1] / 1.5;
            if res != 0 {
                w.mm(id).rotation[1] = 0.0;
                w.mm(id).anim.speed = 1.0;
                w.mm(id).rotation[0] = 0.0;
                return 1;
            }
            if w.m(id).anim.flags & 2 != 0 {
                let s = rf(w, r, id, rec::SPEED_AFTER);
                w.mm(id).anim.speed = s;
            }
        }
        _ => {}
    }
    tail(w, id, r)
}

fn tail_none(w: &mut World, id: MobyId) -> i32 {
    if w.m(id).mode & mode::KEEP_MATRIX != 0 { sphere_lerp(w, id); }
    0
}

/// The carried update's end (LAB_003060a0): the matrix with mode 4; below state 4 the let-go countdown.
fn tail(w: &mut World, id: MobyId, r: usize) -> i32 {
    if w.m(id).mode & mode::KEEP_MATRIX != 0 { sphere_lerp(w, id); }
    if rs(w, r, id, rec::STATE) < 4 {
        let h = rf(w, r, id, rec::HOLD);
        if h <= 0.0 { slot_let_go(w, id); }
        let h = rf(w, r, id, rec::HOLD);
        set_rf(w, r, id, rec::HOLD, h - 1.0);
    }
    0
}

/// State 3: pulled toward the mouth (the tail runs after it either way).
fn pulled(w: &mut World, id: MobyId, r: usize) {
    w.mm(id).visible = 1;
    w.mm(id).mode &= !mode::HIDDEN;
    let Some(c) = w.svc.creatures.react.cannon else { return };
    let t = rf(w, r, id, rec::T);
    if t < 1.0 {
        let t = t + 1.0 / w.ticks(30) as f32;
        set_rf(w, r, id, rec::T, t);
        let e = euler_rows([-PI / 2.0, 0.0, -PI / 2.0]);
        let target = rows_quat(mul_rows(e, c.rows));
        let q0 = rv(w, r, id, rec::QUAT);
        let q = slerp(t, q0, target);
        set_rows3(w, id, quat_rows(q));
        w.mm(id).mode = (w.m(id).mode & !mode::MIRROR) | mode::KEEP_MATRIX;
    } else {
        set_rf(w, r, id, rec::T, 1.0);
    }
    let mouth = [c.mouth[0], c.mouth[1], c.mouth[2], 1.0];
    let p = super::pos(w, id);
    let d = super::dist3(p, mouth);
    let sc = class_scale(w, id) * (d / (rf(w, r, id, rec::DIST0) * 2.0) + 0.5);
    w.mm(id).scale = sc;
    let sp = (rf(w, r, id, rec::PULL) + 0.11).min(0.75);
    set_rf(w, r, id, rec::PULL, sp);
    if d < sp * 70.0 { w.svc.creatures.react.cannon_seq4 = true; }
    if d < 3.0 {
        w.svc.creatures.react.swap_lock = Some(2);
        w.mm(id).has_collision = false;
    }
    if sp <= d {
        // Drift sideways in Ratchet's aim frame (0x13f990) on the way in.
        let rows = w.hero.moby_rows.map(|v| [v[0].to_f32(), v[1].to_f32(), v[2].to_f32()]);
        let dv = super::sub(p, mouth);
        let l = [dv[0] * rows[0][0] + dv[1] * rows[0][1] + dv[2] * rows[0][2], dv[0] * rows[1][0] + dv[1] * rows[1][1] + dv[2] * rows[1][2]];
        let (ox, oy) = if l[0] < 0.0 {
            let oy = if 0.0 < l[1] { (-l[0]).min(2.0) } else if -2.0 < l[0] { l[0] } else { -2.0 };
            (-l[0] * 0.5, oy)
        } else {
            let a = l[1].abs();
            (if a < 2.0 { a } else { 2.0 }, l[1] * 0.5)
        };
        let off: V = [ox * rows[0][0] + oy * rows[1][0], ox * rows[0][1] + oy * rows[1][1], 0.0, 0.0];
        let tgt = super::add(mouth, off);
        let step = super::set_len3(super::sub(tgt, p), sp);
        let np = super::add(p, [step[0], step[1], step[2], 0.0]);
        super::set_pos(w, id, [np[0], np[1], np[2], p[3]]);
        sphere_lerp(w, id);
        return;
    }
    // Arrived: swallowed (slot +0x04), no longer a target.
    let g = &mut w.svc.creatures.react;
    g.coming -= 1;
    g.held += 1;
    slot_swallow(w, id, mouth);
    let slot = rs(w, r, id, rec::SLOT);
    if let Some(&s) = w.svc.creatures.react.slots.get(slot.max(0) as usize) {
        if s > 0 {
            let m = (s - 1) as usize;
            if m < w.table.mobys.len() { w.mm(m).mode &= !mode::TARGETABLE; }
        }
    }
}

/// States 5 / 6: the flight after the fire-out (homing, the hits along the way, the roll, the bounce off walls, the
/// burst on the floor, a creature (gold), the timer, 60 from Ratchet or too slow).
fn flight(w: &mut World, id: MobyId, r: usize) -> i32 {
    w.mm(id).visible = 1;
    w.mm(id).mode &= !mode::HIDDEN;
    if super::pos(w, id)[2] < 2.0 {
        land(w, id, r, false);
        return 0;
    }
    let vel0 = rv(w, r, id, rec::VEL);
    let regrow = class_scale(w, id) * rf(w, r, id, rec::REGROW);
    let sc = (w.m(id).scale + regrow).min(class_scale(w, id));
    w.mm(id).scale = sc;
    let mut vel = vel0;
    if let Some(t) = moby_ref(w, r, id, rec::TARGET).filter(|&t| !dead(w, t)) {
        if ri(w, r, id, rec::TIMER) < w.ticks(300) - w.ticks(5) {
            let mut tp = super::pos(w, t);
            tp[2] += rf(w, r, id, rec::TARGET_DZ);
            let a = super::sub(tp, rv(w, r, id, rec::TARGET_AT));
            let b = super::sub(tp, super::pos(w, id));
            set_rv(w, r, id, rec::TARGET_AT, tp);
            let s = super::len3(vel);
            let aa = s * s - len2sq(a);
            let bb = dot(a, b) * -2.0;
            let cc = len2sq(b);
            // `fun_001f9988` (0x2210f0) is the VU0 `vsqrt`, which roots the magnitude: a negative discriminant (a target
            // moving faster than the shot) gives a number in the game, not NaN — the same value here.
            let disc = (bb * bb - aa * 4.0 * -cc).abs().sqrt();
            let t1 = (-bb + disc) / (aa + aa);
            let t2 = (-bb - disc) / (aa + aa);
            let t = if t1 <= 0.0 || t2 <= 0.0 {
                if t1 <= 0.0 { if 0.0 < t2 { t2 } else { -1.0 } } else { t1 }
            } else if t1 <= t2 { t2 } else { t1 };
            let steer = if t <= 0.0 { super::set_len3(b, super::DT2 * 90.0) } else { super::set_len3(super::add(super::scale(a, t), b), super::DT2 * 90.0) };
            vel = super::set_len3(super::add(vel, [steer[0], steer[1], steer[2], 0.0]), s);
        }
    }
    let mut p = super::add(super::pos(w, id), [vel[0], vel[1], vel[2], 0.0]);
    super::set_pos(w, id, p);
    // The hits along the way (FUN_0026e808(2, tmpl, moby, 0x430000, vel), xy length 1, z 1, the exact push; type
    // bytes 1 / 1, class 0x351) with the flight sphere, then the world sphere.
    let d2 = super::set_len2([vel[0], vel[1], vel[2], 0.0], 1.0);
    let tmpl = HitTemplate { dir: [Pf::f(d2[0]), Pf::f(d2[1]), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x43_0000, b18: 1, b19: 1, h1a: 0x351, damage: Pf::f(2.0), w20: 1 };
    let fr = rf(w, r, id, rec::FLY_R);
    let hit_mobys = w.sphere_mobys(pf(fr), p.map(Pf::f), 1, Some(id), Some(&tmpl)) != 0;
    let world = w.coll_sphere(p.map(Pf::f), pf(fr), 0, Some(id));
    let mut hit = hit_mobys || world.is_some();
    if let Some(o) = &world {
        if let Some(c) = o.pushed_centre { p = [c[0], c[1], c[2], p[3]]; super::set_pos(w, id, p); }
        let contact = [o.point[0] - p[0], o.point[1] - p[1], o.point[2] - p[2], 0.0];
        set_rv(w, r, id, rec::CONTACT, contact);
        if o.moby.is_some_and(|m| w.m(m).o_class == 0) { hit = false; }
    }
    if rs(w, r, id, rec::STATE) == 6 {
        // Roll about the axis across the motion.
        w.mm(id).mode = (w.m(id).mode & !mode::MIRROR) | mode::KEEP_MATRIX;
        w.mm(id).has_collision = w.classes.info(w.m(id).o_class).is_some_and(|i| i.has_collision);
        set_rv(w, r, id, rec::CONTACT, [0.0, 0.0, 1.0, 0.01]);
        let sp = super::len3(vel0);
        let q0 = rows_quat(rows3(w, id));
        let back = super::set_len3(vel, -1.0);
        let axis = super::set_len3(cross(back, [0.0, 0.0, 1.0, 0.0]), 1.0);
        let ang = sp / fr;
        let (s, c) = (ang * 0.5).sin_cos();
        let qa = [axis[0] * s, axis[1] * s, axis[2] * s, c];
        let q = rc_formats::moby_anim::quat_product(q0, qa);
        let m = quat_rows(q);
        set_rows3(w, id, m.map(|row| { let n = (row[0] * row[0] + row[1] * row[1] + row[2] * row[2]).sqrt(); if n == 0.0 { row } else { row.map(|x| x / n) } }));
    }
    if hit {
        let n = world.as_ref().map_or([0.0, 0.0, 1.0], |o| {
            let l = (o.normal[0] * o.normal[0] + o.normal[1] * o.normal[1] + o.normal[2] * o.normal[2]).sqrt();
            if l == 0.0 { [0.0, 0.0, 1.0] } else { o.normal.map(|x| x / l) }
        });
        let ty = world.as_ref().and_then(|o| o.moby).and_then(|m| w.classes.info(w.m(m).o_class).map(|i| i.ty)).unwrap_or(0);
        let s = seq(w, id, 6);
        blend(w, id, s, 20);
        let cb = ri(w, r, id, rec::COLL_BOUNCE);
        w.mm(id).coll_disable = cb as u32;
        set_rs(w, r, id, rec::STATE, 6);
        // `FastArcTan(n.z, |n.xy|)`: the normal's angle from vertical (a wall is steep).
        let slope = super::atan(n[2], super::len2([n[0], n[1], 0.0, 0.0]));
        let bs = rs(w, r, id, rec::SOUND_BOUNCE);
        if 0.872_664_63 < slope {
            if bs >= 0 { w.play_sound(bs as i32, 0, id); }
            land(w, id, r, true);
            return 0;
        }
        if w.svc.creatures.react.gold && 4 < ty {
            if bs >= 0 { w.play_sound(bs as i32, 0, id); }
            land(w, id, r, true);
            return 0;
        }
        vel = reflect(vel, [n[0], n[1], n[2], 0.0]);
    }
    set_rv(w, r, id, rec::VEL, vel);
    let mut t = ri(w, r, id, rec::TIMER);
    let out = super::dec_timer_i32(&mut t);
    set_ri(w, r, id, rec::TIMER, t);
    let hp = w.hero.pos.map(|x| f32::from_bits(x.0));
    if out != 0 || 60.0 < super::dist3(super::pos(w, id), [hp[0], hp[1], hp[2], 0.0]) || super::len3(vel) < 0.01 {
        land(w, id, r, true);
        return 0;
    }
    vel[2] -= 0.00925;
    set_rv(w, r, id, rec::VEL, vel);
    // PartType18Spawn(pos, rows): the smoke trail (`particles::type18`; its 5 draws, also made without a particle system).
    let (at, m) = (w.m(id).position, w.m(id).rows);
    let rows = [0, 1, 2].map(|k| [m[k][0], m[k][1], m[k][2]]);
    *w.svc.fx.part_spawns.entry(crate::particles::type18::TYPE).or_default() += 1;
    match w.particles.as_deref_mut() {
        Some(ps) => {
            if crate::particles::type18::spawn(ps, w.rng, at, rows).is_none() { w.svc.fx.part_failed += 1; }
        }
        None => {
            for (lo, hi) in [(-0.1f32, 0.1f32), (-0.3, 0.3), (-0.3, 0.3)] { w.rng.randf(lo, hi); }
            w.rng.rand();
            w.rng.randf(f32::from_bits(0x46da_b201), f32::from_bits(0x4743_3c00));
        }
    }
    tail(w, id, r)
}

/// `FUN_00221570`: `v` reflected off the plane of normal `n` when it moves into it.
fn reflect(v: V, n: V) -> V {
    let u = super::set_len3(n, 1.0);
    let d = dot(v, u);
    if d <= 0.0 { super::sub(v, super::scale([u[0], u[1], u[2], 0.0], 2.0 * d)) } else { v }
}

/// `0x3051a8(moby, rec, burst)`: the end of a flight: the burst ([`burst`]) when asked, the landing collision bits,
/// no rotation, the class scale, record state 0, then slot +0x14 (every table: delete).
pub fn land(w: &mut World, id: MobyId, r: usize, with_burst: bool) {
    if with_burst { burst(w, id); }
    let c = ri(w, r, id, rec::COLL_LAND);
    w.mm(id).coll_disable = c as u32;
    w.mm(id).rotation = [0.0; 4];
    set_rows3(w, id, euler_rows([0.0; 3]));
    sphere_lerp(w, id);
    w.mm(id).mode &= !mode::KEEP_MATRIX;
    let sc = class_scale(w, id);
    w.mm(id).scale = sc;
    set_rs(w, r, id, rec::STATE, 0);
    slot_delete(w, id);
}

/// `0x304798(moby)`: the burst of a fired moby: the Bomb Glove's explosion code (`classes::bomb::blast` with
/// [`GLOVE_BLAST`](crate::moby_update::classes::bomb::GLOVE_BLAST): its tables 0x20b820 / 0x20b838 hold the bomb's
/// words) at size k = 1 (2 gold), the fireballs divided by n = 2 (1 gold), the colours shifted by the gold flag
/// 0x13e529 (`0x270fa8` / `0x270f48`); then the camera shake, the light (template 0x20b7d0 = the bomb's 0x20a930); area
/// hits (radius 3, `0x26f8f8` damage 2 flags 0x830000) only with the gold cannon. Everything carries the base
/// velocity `(0, 0, 2·dt·k)` (sp+0x60: the bomb's drift in its explosion). No sound.
pub fn burst(w: &mut World, id: MobyId) {
    w.svc.creatures.react.bursts += 1;
    let gold = w.svc.creatures.react.gold;
    let k = gold as i32 as f32 + 1.0;
    let n = if gold { 1 } else { 2 };
    let pos = super::pos(w, id);
    let dcam = super::len3(super::sub(w.camera.map(|x| x.to_f32()), pos));
    let up: V = [0.0, 0.0, 1.0, 0.0];
    let base: V = [0.0, 0.0, 2.0 * super::DT * k, 0.0];
    crate::moby_update::classes::bomb::blast(w, &crate::moby_update::classes::bomb::GLOVE_BLAST, id, pos, base, up, k, n, true, gold as u8);
    let amp = if dcam < 20.0 { 0.4 - dcam * 0.0175 } else { f32::from_bits(0x3d4c_ccd0) };
    let t = w.ticks(25);
    w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp, ticks: t });
    super::fx::light_spawn(w, &super::fx::LIGHT_BOMB, pos);
    if gold {
        let tmpl = HitTemplate { dir: [Pf::ZERO, Pf::ZERO, Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x83_0000, b18: 2, b19: 1, h1a: 0x351, damage: Pf::f(2.0), w20: 1 };
        w.sphere_mobys(Pf::f(3.0), pos.map(Pf::f), 0x10, Some(id), Some(&tmpl));
    }
}

// ------------------------------------------------------------------------------------------------------------------
// The damage record's weapon inputs

/// The damage record's lure (+0x18: `0x2cc830` stores the Taunter; the classes read and clear it every tick).
pub fn lure(w: &World, id: MobyId) -> Option<usize> { super::header(w, id).damage.map(|d| d + 0x18) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quaternion_round_trip_and_slerp_ends() {
        let e = euler_rows([0.3, -0.2, 1.1]);
        let q = rows_quat(e);
        let back = quat_rows(q);
        for i in 0..3 { for j in 0..3 { assert!((back[i][j] - e[i][j]).abs() < 1e-5, "{i}{j}"); } }
        let a = rows_quat(euler_rows([0.0; 3]));
        assert!((slerp(0.0, a, q).iter().zip(a).map(|(x, y)| (x - y).abs()).sum::<f32>()) < 1e-5);
        assert!((slerp(1.0, a, q).iter().zip(q).map(|(x, y)| (x - y).abs()).sum::<f32>()) < 1e-5);
    }

    fn world_parts() -> (crate::hero::Hero, crate::rng::Rng, crate::moby_update::ClassTable, crate::moby_update::Services) {
        (crate::hero::Hero::new(), crate::rng::Rng::new(), crate::moby_update::ClassTable::default(), crate::moby_update::Services::new())
    }

    /// `0x302840` with `gp−0x4d60` (the swallow) and `gp−0x4d50` (the fire): the moby's rows are the Euler's rows in
    /// the cannon's, its matrix kept (mode 4); the swallow puts it at the mouth with those rows.
    #[test]
    fn swallow_and_fire_take_the_cannons_rows() {
        use crate::moby_runtime::{Moby, MobyTable};
        let critter = Moby { o_class: 577, has_class: true, state: 1, spawn_flag: 0xfe, pvars: vec![0; 0x60 + rec::SIZE], ..Moby::default() };
        let mut t = MobyTable::new(vec![critter], 4);
        let (hero, mut rng, classes, mut svc) = world_parts();
        let cannon = euler_rows([0.2, -0.4, 1.3]);
        svc.creatures.react.cannon = Some(Cannon { pos: [1.0, 2.0, 3.0], mouth: [1.5, 2.0, 3.5], rows: cannon });
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        let close = |a: [[f32; 3]; 3], b: [[f32; 3]; 3]| (0..3).all(|i| (0..3).all(|j| (a[i][j] - b[i][j]).abs() < 1e-5));
        assert_eq!(swallow(&mut w, 0, [1.5, 2.0, 3.5, 1.0]), 1);
        assert!(close(rows3(&w, 0), mul_rows(euler_rows(SWALLOW_EULER), cannon)), "swallowed: {:?}", rows3(&w, 0));
        assert_eq!(w.m(0).position[..3], [1.5, 2.0, 3.5]);
        assert!(w.m(0).mode & mode::KEEP_MATRIX != 0);
        assert_eq!(rs(&w, 0x60, 0, rec::STATE), 4);
        cannon_frame(&mut w, 0, FIRE_EULER);
        assert!(close(rows3(&w, 0), mul_rows(euler_rows(FIRE_EULER), cannon)), "fired: {:?}", rows3(&w, 0));
    }

    /// `0x304798`: the flashes (and the fireballs and rings) carry the base velocity (0, 0, 2·dt·k) (sp+0x60), k = 1.
    #[test]
    fn burst_carries_the_base_velocity() {
        use crate::moby_runtime::{Moby, MobyTable};
        let mut t = MobyTable::new(vec![Moby { o_class: 577, position: [0.0, 0.0, 10.0, 1.0], ..Moby::default() }], 40);
        let (hero, mut rng, classes, mut svc) = world_parts();
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        burst(&mut w, 0);
        let base = [0.0, 0.0, 2.0 * super::super::DT, 0.0];
        let vel = |m: &Moby| std::array::from_fn::<f32, 4, _>(|k| f32::from_le_bytes(m.pvars[4 * k..4 * k + 4].try_into().unwrap()));
        let flashes: Vec<[f32; 4]> = w.table.mobys.iter().filter(|m| m.o_class == crate::moby_update::classes::bomb::FLASH_CLASS && m.state < 0x80).map(vel).collect();
        assert_eq!(flashes.len(), 5, "the camera is 10 away: five flashes");
        assert!(flashes.iter().all(|v| *v == base), "{flashes:?}");
        let balls: Vec<[f32; 4]> = w.table.mobys.iter().filter(|m| m.o_class == crate::moby_update::classes::bomb::FIREBALL_CLASS && m.state < 0x80).map(vel).collect();
        assert_eq!(balls.len(), 5 + 2 + 1);
        // The low and high ones: a spread with a non-negative up part, plus the base.
        assert!(balls[..7].iter().all(|v| v[2] >= base[2] - 1e-6), "{balls:?}");
    }

    #[test]
    fn tables_by_class_number() {
        assert_eq!(table_by_class(577), Some(Table::Critter));
        assert_eq!(table_by_class(866), Some(Table::Amoeboid));
        assert_eq!(table_by_class(865), None, "865 keeps the default table: never sucked");
        assert_eq!(table_by_class(459), None);
        assert_eq!(table_by_class(270), Some(Table::Chicken));
    }
}
