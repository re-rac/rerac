//! **The other hero bodies (G-HERO-005).** The game's hero is a body that can change: the body word `0x1413f4` (0
//! Ratchet, 1 Clank, 2 Giant Clank, 3 the Hologuise disguise) picks which moby the hero code drives (`0x1413d0`: Ratchet's
//! own moby `0x13fdd8`, or the body moby `0x13fdd4`), which states it runs (Clank 0x43..0x52 / 0x7d, Giant Clank
//! 0x5a..0x62, the disguise 0x53..0x59), the capsule, the probes' heights, the HUD and the classes' branches. This module
//! is that system, ported once for every level and class that uses it:
//!
//! * **The switch** `SwitchCharacter(mode, state, moby)` ([`switch_character`]; level01 `0x231348` = levels 02, 03, 05,
//!   06, 08, 11, 12, 14, 16, 17; level00 `0x210a08` = 04 `0x203fe8`, 07 `0x238870`, 09 `0x2306e8`, 10 `0x204070`, 13
//!   `0x21c4f0`, 15 `0x208b80`, 18 `0x218800`, which add Giant Clank's energy 200): the superset, as every copy runs on
//!   the paths a level reaches (a level-01-family copy never switches to body 2).
//! * **Leaving the body** ([`leave_body`]; level01 `0x231450`, level00 `0x210b30` = 04 `0x204110`, 07 `0x238998`, 09
//!   `0x230810`, 10 `0x204198`, 13 `0x21c618`; 06 `0x227cf8` / 17 `0x20da68` without the body-2 branch; 15 `0x208ca8`
//!   / 18 `0x218928` without the body-1 branch): the superset (a copy's missing branch is for a body that level never
//!   has).
//! * **The per-body idle** `0x227638` ([`body_idle`]; L04 `0x21ac18`, L06 `0x239528`, L07 `0x24f4a0`, L09
//!   `0x247318`, L10 `0x21aca0`, L13 `0x233120`, L15 `0x21ab60`, L17 `0x223380`, L18 `0x22c418`): what `SetState(0)`
//!   becomes in a body.
//! * **The body's hero update** `HeroUpdateAlt` (L00 `0x2062b0`; level01 `0x228000`): [`body_update`] (the hero part) and
//!   [`after_update`] (the mobys it creates and places: Clank's antenna glow 0x4b4 and rotor 0x47a, Giant Clank's pilot).
//! * The states: [`clank`] (body 1), [`giant`] (body 2), [`disguise`] (body 3, 0x53..0x59: the Hologuise's, with the item
//!   and the way in / out in `crate::hero::hologuise`, 2026-10-01 G-WPN-006).
//!
//! **Plug-and-play.** A class that hands the hero a body calls [`queue_switch`] (`SwitchCharacter`) and
//! [`queue_leave`] (the level's leave copy) through its `World`; the per-body idle is `HeroCall::BodyIdle`. A class
//! that branches on the body reads `World::hero.mode` (0x1413f4) and, for Clank's kick, `World::hero.state == 0x51`.
//! The engine binds the hero's animation to the body moby through [`crate::hero::anim::HeroAnimCtl`] (the tick calls
//! `AnimCtl::bind_body`), draws the body moby from the table, and gives the hero the body classes' joint lists
//! ([`BodyJoints`], `fx::JointData::bodies`).
//!
//! **Coverage** (`address | what | ported (fn) / NOT ported (gap) / n/a (why)`):
//!
//! | address | what | status |
//! |---|---|---|
//! | 0x231348 / 0x210a08 +0 | `0x22efd8`: a raised weapon put away | ported ([`switch_character`] → `weapons::put_away`) |
//! | +1 | 0x1413f6 = 1 | ported (`items.f13f6`) |
//! | +2 | `0x231088`: one pass of the item slots' updates (with 0x1413f6 set the hand item starts its put-away: the wrench requested, the held item kept to restore, slot state 3, its put-away blend, the swap's look / fidget / weapon effects) | ported ([`switch_character_with`]'s pass = `items::slot_pass`, made by the tick with the item environment) |
//! | +3 | 0x1413d0 = Ratchet; `0x277740(0x1409c0)`: his after-images end | ported (`fx.trails.hero.kill`) |
//! | +4 | Ratchet +0x98 = −1 (every collision primitive of his moby off) | ported ([`BodyCmd::RatchetCollision`]) |
//! | +5 | `0x2486c0`: Ratchet and his items hidden (+0x34 \|= 1) | ported ([`BodyCmd::RatchetHidden`]; the engine hides his items with him) |
//! | +6 | `0x2283a8`: the hero's looping sounds released | ported (`packs::stop_loops`) |
//! | +7 | `0x227420`: every joint record detached, angles / targets zeroed, scale 1 | ported (`Idle::detach_all`, [`Bodies::detach_all`]) |
//! | +8 | 0x1413f4 = mode, 0x13fdd4 = 0x1413d0 = the body moby | ported ([`Bodies::moby`]) |
//! | +9 | `HeroSizeCapsule`, capsule top / bottom = their targets | ported (`size_capsule`, snap) |
//! | +10 | body +0x34 \|= 6 (no update, matrix kept) | ported ([`BodyCmd::Mode`]) |
//! | +11 | 0x13fde0 = 0x13fde4 = 1 (playback speed, rate) | ported (`anim_speed`; the rate by `bind_body`) |
//! | +12 | `0x247d00`: the anim loop cleared | ported (`clear_loop` after `bind_body`) |
//! | +13 | `0x26be04(body)`: the hero lighting on the body moby | n/a here (the engine's hero lighting reads the hero moby, [`Hero::hero_moby`]) |
//! | +14 | `0x229158`: the squash table 0x14162c | ported (`hologuise::squash`, 2026-10-01) |
//! | +15 | 0x14161c = state | ported ([`Bodies::state_param`]) |
//! | +16 | mode 1: 0x141600 = (s16)health, health = Clank's 0x1415fc | ported |
//! | +17 | (L00 family) mode 2: 0x140980 = 200 | ported ([`Bodies::energy`]) |
//! | +18 | `SetState(state, 1)` | ported |
//! | 0x231450 / 0x210b30 +0 | mode 1: 0x1415fc = health, health = 0x141600; the rotor 0x140970 and the glow 0x140974 deleted | ported ([`leave_body`], [`BodyCmd::Delete`]) |
//! | +1 | mode 2 and 0x140984 ≠ −1: `0x24b090(0x140984, 0)` (Giant Clank's energy HUD released), 0x140984 = −1 | ported ([`Bodies::energy_hud`]; the HUD element itself: `crate::hud`) |
//! | +2 | `0x227420` | ported |
//! | +3 | 0x1413f4 = 0; body +0x34 &= ~6; 0x13fdd4 = 0 | ported |
//! | +4 | the disguise moby 0x13fddc deleted | ported (made by the Hologuise's timer: `hologuise`, `fx::MobySpawn::Disguise`) |
//! | +5 | `0x2283a8` | ported |
//! | +6 | 0x1413d0 = Ratchet; his position = the hero's; +0x98 = 0; 0x13fde4 = 1; `0x247d00` | ported ([`BodyCmd`], `bind_body(None)`, `clear_loop`) |
//! | +7 | state ≠ 100 or game mode 0x15f5c4 ∉ {2, 6}: `SetState(0, 1)` | ported |
//! | 0x227638 | body 0 → `SetState(0, 1)`, 1 → 0x43, 2 → 0x5a, 3 → 0x53 | ported ([`body_idle`]) |
//! | SetState 0 (L00 case 0) | in a body: group 0, 0x1415d4 = 0, the look timer draw, `0x22b8e8`, then `0x227638` and SetState returns 1 | ported (`ground.rs`) |
//! | SetState prologue | mode ≠ 0: 0x1413fd / 0x1413f7 / 0x1413ff / Ratchet +0x98 / after-images untouched | ported (the existing `mode == 0` gate) |
//! | SetState prologue | mode 3 and the state not 0x53..0x59 / 0x65..0x67 / 0 / 1: `0x22cd18` → leave the disguise | ported (`states::set_state`, 2026-10-01) |
//! | HeroUpdateAlt +0 | `RatchetAnimAdvance` on the body moby | ported ([`body_update`]; the class sound of its triggers through `HeroSounds::anim_advanced`) |
//! | +1 | `HeroMotionUpdate` | ported (`input_physics_move`) |
//! | +2 | `HeroSurfaceReaction` | ported (`surface_reaction`; Clank's burn 0x7d: `surface.rs`) |
//! | +3 | mode 1: `HeroWallLedgeCheckB`, Clank's lean `0x215b68` | ported (`ledge::wall_ledge_probe_b` with Clank's heights, [`clank::lean`]) |
//! | +4 | the transitions | ported |
//! | +5 | mode 0 after them: return | ported |
//! | +6 | `0x236860`: the delayed voices | ported (`fx::voice_queue`) |
//! | +7 | `0x2273d0`: the joint springs (kinds 2 / 3 on the body moby, kind 1 on Clank's back moby) | ported ([`springs`]) |
//! | +8 | mode 0: return | ported |
//! | +9 | `0x22a110`: the body's write-back (first-person hide / show, Ratchet hidden, hit slot 0xff, position / Euler / rows, `MobyBuildMatrix`) | ported (`write_back`, the tick) |
//! | +10 | `0x229348`: the global fade 0x15f3fc | n/a (not ported on foot either; no reader in the body states) |
//! | +11 | `0x229158` | ported (`hologuise::squash` on the body moby) |
//! | +12 | mode 1: record 25's scale = 0x15ee18; `0x2278c0` (0x15ee18 → 1.0, the glow pulse on the body moby) | ported ([`Bodies::scale18`], [`glow`]) |
//! | +13 | mode 1: the antenna glow moby 0x4b4 created (scale ×1.7, glow 0x801432d7, 0x141650 = 0x301432d7) and placed at joint list 7 (0.0257 down its rows), its scale approaching 1.7 (×1.4 while flashing) | ported ([`after_update`]; 3.1 with the cheat 0x15edb3) |
//! | +14 | mode 1, body shown: joint list 7's point → 0x1410d0; `0x229400`: the hero's draw callback `0x229440` | ported (`super::glow::body_dot`, `Callback::HeroGlow`) |
//! | +15 | mode 1: the glow phase 0x140978 (170°/s, 700°/s while the flash 0x14164c runs) and its colours by 0x14164e, tweened into the glow moby's +0x90 and 0x141650 | ported ([`after_update`]) |
//! | +16 | mode 1: the rotor moby 0x47a created / placed at joint list 5 (rows: the joint's × Rz(−yaw)), its light = the body's; in 0x4f its sequence 1 (blend 10), else 0 (blend 17) | ported ([`after_update`]) |
//! | +17 | mode 1: a command 0x141610 (the command menu): `PlayClassSound(cmd + 16, 0, hero)`, 0x14164e = cmd, 0x14164c = 57 | ported ([`Bodies::command`]; the menu that sets it: `menus::quick_select`'s command page) |
//! | +18 | `0x25c4f8`: the map fog writer | ported by the engine (it reads the hero position) |
//! | +19 | mode 2: cheat 0x15edb3 → record 28's scale = 0x15ee18 ·1.4 | ported (`update` tail, `crate::cheats`) |
//! | +20 | mode 2: `0x2278c0`, `0x2061f0` (the pilot: Ratchet shown when the body is, sequence 0x81 over 10 ticks, his generic advance) | ported ([`glow`], [`after_update`]) |
//! | +21 | mode 2, body shown: joint list 6's point → 0x1410d0, `0x229400` | ported (`super::glow::body_dot`) |
//! | +22 | mode 3 (the disguise's tint and glow points, 0x14162e; the cheat 0x15edb1's head record 30 (L01 0x17c04c) = 1.9) | ported (`hologuise::body_update`, record [`rec::R30`]) |
//! | +23 | `0x22a260`: the hero shadow; `0x26be04`: the hero lighting | ported by the engine on the hero moby ([`Hero::hero_moby`]) |
//! | HeroTickStateTimer | mode 2: 0x140986 (the beam's lockout) counted down | ported (`post_move`) |
//! | HeroTickStateTimer | the body point 0x13f420 (0.4 up for Clank, 4.0 for Giant Clank) and the bolt radii 0x1415d8 / 0x1415dc (2.125 / 1.25, 15 / 3) | ported (`post_move`; `World::bolt_radii`) |
//! | HeroTickStateTimer | the disguise's 18-tick timer 0x14162e (enter / leave body 3, moby 0x27a) | ported (`hologuise::tick_timer` / `enter` / `leave`) |
//! | `HeroSizeCapsule` | Clank: bottom 0.45, radius 0.3, top 0.6; Giant Clank 4.45 / 3.75 / 5.25 | ported (`size_capsule`) |
//! | `0x232978` wall check | Clank: lines at 0.35 (×0.3), 0.27 long; Giant Clank 1.5, 5.0 | ported (`wall_check`) |
//! | `HeroWallLedgeCheckB` / `C` | Clank's heights (top line 0.875..0.7, 0.725 max climb, 0.7 above the ground, wall from −0.4, hang −0.71, 0.32 out, steps 0.0625) and states | ported (`ledge.rs`) |
//! | `HeroSurfaceReaction` | body 1 on surface 1 grounded → 0x7d | ported (`surface.rs`) |
//! | hit intake `0x231580` | body 2: flag bit 2 accepted, the damage taken from the energy; flag 4 or no energy → 0x5d + `0x2a9be0` + knockback (7, 3.5)·dt on the ground | ported (`damage::hit_intake`, [`giant::beam_end`]) |
//! | HUD `0x24e418` | body 2: no health; body 1: the orbs' count = 0x1415fc | ported (`crate::hud`) |
//! | HUD `0x24f9c0` | body 2: Giant Clank's energy bar (slot 0x10, data 0x140980, max 200; draw `0x24f248`) | ported by `crate::hud` (derived each tick from `Hero::mode` and [`Bodies::energy`]: `update_weapon_request`, `draw_giant`) |
//! | transitions prologue | △ in body 1: the command menu `0x238b18` / `0x238b80` / `0x238f88` instead of the quick select | ported (`menus::quick_select` [`Commands`](crate::menus::quick_select::Commands); the gadgetbots that obey it: `units::blarg_gadgetbot`) |
//! | checkpoint `0x29ac10` / `0x29adc8` | the body and 0x14161c saved; the reload switches back into the body (the first moby of class 0x57 / 0x1a3) | ported (`checkpoint::record`, the engine's respawn) |
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod clank;
pub mod disguise;
pub mod giant;

use super::anim::AnimCtl;
use super::idle::JointRec;
use super::physics::Env;
use super::states::Ctx;
use super::{Hero, HeroSounds};
use crate::moby_runtime::{Moby, MobyId};
use crate::ps2v::Pf;
use crate::rng::Rng;

/// The body word `0x1413f4`.
pub mod body {
    pub const RATCHET: u8 = 0;
    pub const CLANK: u8 = 1;
    pub const GIANT: u8 = 2;
    /// The Hologuise disguise (G-WPN-006).
    pub const DISGUISE: u8 = 3;
}

/// Clank's body class (the checkpoint reload's search, `0x29adc8`).
pub const CLANK_CLASS: i16 = 0x57;
/// Giant Clank's body class.
pub const GIANT_CLASS: i16 = 0x1a3;
/// The disguise moby (`CreateMoby(0x27a)` in `HeroTickStateTimer`; G-WPN-006).
pub const DISGUISE_CLASS: i16 = 0x27a;
/// Clank's antenna glow (0x140974).
pub const GLOW_CLASS: i16 = 0x4b4;
/// Clank's rotor (0x140970).
pub const ROTOR_CLASS: i16 = 0x47a;

/// A command to the moby table the body code makes (applied by the tick right after the call or the update that made
/// it: [`apply_cmds`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyCmd {
    /// A moby's +0x34: `mode = (mode | set) & !clear`.
    Mode { id: MobyId, set: u16, clear: u16 },
    /// Ratchet's +0x34 bit 0 (`0x2486c0` / `0x2487a8`).
    RatchetHidden(bool),
    /// Ratchet's +0x98 (every collision primitive off: −1; on: 0).
    RatchetCollision(u32),
    /// Ratchet's position +0x10..+0x1c = the hero's (leaving the body).
    RatchetPlace([f32; 4]),
    /// `DeleteMoby`.
    Delete(MobyId),
}

/// The body moby a class hands to `SwitchCharacter` (what the hero code needs of it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyMoby {
    pub id: MobyId,
    pub o_class: i16,
    /// Its animation fields (+0x50..+0x70): where the hero's animation of the body starts.
    pub anim: rc_formats::moby_anim::AnimState,
}

/// The body fields of the hero block.
#[derive(Clone, Debug)]
pub struct Bodies {
    /// 0x13fdd4: the body moby (0x1413d0 while a body is in, `Hero::mode` ≠ 0).
    pub moby: Option<MobyId>,
    /// Its class (+0xa6): the body's joint lists and sounds.
    pub class: i16,
    /// 0x141600 (s16): Ratchet's health while Clank is the hero.
    pub saved_health: i16,
    /// 0x1415fc: Clank's health (4 at the hero init 0x226b70).
    pub clank_health: i32,
    /// 0x140980: Giant Clank's energy (200 at the switch).
    pub energy: i32,
    /// 0x140984 (s16): the HUD request handle of Giant Clank's energy bar (`HudWeaponShow` 0x24f9c0; −1 none).
    pub energy_hud: i32,
    /// 0x140986 (s16): Giant Clank's beam lockout (ticks(301) after the beam 0x61; counted down in body 2).
    pub beam_lock: i16,
    /// 0x140970 / 0x140974 / 0x13fddc: Clank's rotor 0x47a, his antenna glow 0x4b4, the disguise moby.
    pub rotor: Option<MobyId>,
    pub glow: Option<MobyId>,
    pub disguise: Option<MobyId>,
    /// 0x140978: the glow's phase (rad).
    pub glow_phase: f32,
    /// 0x14164c (s16) / 0x14164e (s16): the command flash timer (ticks(57)) and its kind (the command 1..4).
    pub flash: i16,
    pub flash_kind: i16,
    /// 0x141610: the command the menu chose this tick (0: none; read and cleared by the body update).
    pub command: i32,
    /// 0x141650: the glow colour word the hero's draw callback reads.
    pub glow_colour: u32,
    /// 0x14161c (s16): the state the switch entered (the checkpoint saves it).
    pub state_param: i32,
    /// 0x15ee18: the body's head scale (approaches 1.0 by 0.05 a tick in `0x2278c0`; 1.8 with the cheat 0x15edb3).
    pub scale18: f32,
    /// The glow word `0x2278c0` wrote this tick into the hero moby's +0x90 (bodies 1 / 2; applied by the tick).
    pub body_glow: Option<u32>,
    /// The body's joint records (L00 0x17a680 + 0xb0·rec: 24..26 kind 2 on Clank's lists 4..6; 27..29 kind 3 on Giant
    /// Clank's lists 2, 1, 0; spring 0.02 / 0.2), and those attached to the body moby (its +0x64 list, head first).
    /// Record 30 (L01 0x17bfa0: kind 4 on the disguise's list 3, spring 0.02 / 0.2): the disguise's head (the cheat
    /// 0x15edb1 scales it 1.9 in body 3, `super::hologuise::body_update`).
    pub joints: [JointRec; 7],
    pub manips: Vec<u8>,
    /// Commands to the moby table ([`apply_cmds`]).
    pub cmds: Vec<BodyCmd>,
    /// Ratchet's class sounds the body states play on Ratchet's moby at the hero's position (`0x236738` in body 3,
    /// Clank's burn 0x7d's `PlayClassSound(9, 0, Ratchet)`): played by the tick.
    pub ratchet_sounds: Vec<(i32, u32)>,
    /// The hits Clank's kick and Giant Clank's attacks deal this tick (delivered by the tick through the moby world).
    pub hits: Vec<BodyHit>,
    /// The mobys Giant Clank's states fire this tick (created by the tick, [`after_update`]).
    pub spawns: Vec<giant::Spawn>,
    /// 0x141050 / 0x141060: the hero's position and Euler where he got into Giant Clank (the pads 1451 / 1899 store them;
    /// the mode freeze's exit puts him back there: G-LVL-002).
    pub entry_pose: Option<([f32; 3], [f32; 3])>,
    /// The death reload's switch back into the body the checkpoint saved (`0x29adc8`: `SwitchCharacter(0x1bb6f4,
    /// 0x1bb6fc, the first moby of class 0x57 / 0x1a3)`), made by the next tick before the classes' calls.
    pub restore: Option<(u8, i32, BodyMoby)>,
}

/// A hit of the body's states (`0x26e830` and the shockwave lines), made by the tick on the moby world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyHit {
    /// `0x26e830(r, damage, push, hero, centre, flags, b18, b19, sphere_flags)`.
    Sphere { r: f32, damage: f32, push: f32, centre: [f32; 3], flags: u32, b18: u8, b19: u8, sphere_flags: u32 },
    /// `0x26e808(r, tmpl, hero, flags, dir)` + `coll_sphere_mobys(r, centre, 0x10, hero, tmpl)` (Giant Clank's punch).
    Punch { r: f32, damage: f32, centre: [f32; 3], dir: [f32; 3], flags: u32 },
}

impl Default for Bodies {
    fn default() -> Self {
        let k = 0x3ca3_d70a;
        let d = 0x3e4c_cccd;
        Bodies {
            moby: None,
            class: 0,
            saved_health: 4,
            clank_health: 4,
            energy: 0,
            energy_hud: 0,
            beam_lock: 0,
            rotor: None,
            glow: None,
            disguise: None,
            glow_phase: 0.0,
            flash: 0,
            flash_kind: 0,
            command: 0,
            glow_colour: 0,
            state_param: 0,
            scale18: 1.0,
            body_glow: None,
            joints: [
                JointRec::of_kind(24, 4, 2, k, d),
                JointRec::of_kind(25, 5, 2, k, d),
                JointRec::of_kind(26, 6, 2, k, d),
                JointRec::of_kind(27, 2, 3, k, d),
                JointRec::of_kind(28, 1, 3, k, d),
                JointRec::of_kind(29, 0, 3, k, d),
                JointRec::of_kind(30, 3, 4, k, d),
            ],
            manips: Vec::new(),
            cmds: Vec::new(),
            ratchet_sounds: Vec::new(),
            hits: Vec::new(),
            spawns: Vec::new(),
            restore: None,
            entry_pose: None,
        }
    }
}

/// Indices into [`Bodies::joints`].
pub mod rec {
    pub const R24: usize = 0;
    pub const R25: usize = 1;
    pub const R26: usize = 2;
    pub const R27: usize = 3;
    pub const R28: usize = 4;
    pub const R29: usize = 5;
    pub const R30: usize = 6;
}

impl Bodies {
    /// `0x227420`'s part for the body records (see `Idle::detach_all`).
    fn detach_all(&mut self) {
        for r in self.joints.iter_mut() {
            r.attached = false;
            r.cur = [0.0; 3];
            r.target = [0.0; 3];
            r.trans_target = [0.0; 3];
            r.scale = 1.0;
        }
        self.manips.clear();
    }

    /// The body moby's joint-modifier list (+0x64) as the evaluator reads it, head first; `targets` = the body class's
    /// joint lists' targets ([`BodyJoints`]).
    pub fn modifiers(&self, targets: &[u8]) -> Vec<rc_formats::moby_anim::JointModifier> {
        self.manips
            .iter()
            .filter_map(|&i| {
                let r = self.joints.get(i as usize)?;
                let t = targets.get(r.joint as usize).copied().filter(|&t| t != 0xff)?;
                Some(r.modifier(t))
            })
            .collect()
    }
}

/// A body class's joint lists as the engine loaded them: the first byte lists (root-to-joint chains, `0x2645a8`'s
/// joint points) and each list's target joint (the joint modifiers).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BodyJoints {
    pub o_class: i16,
    pub chains: Vec<Vec<u8>>,
    pub targets: Vec<u8>,
}

impl Hero {
    /// `0x1413d0`: the hero moby — the body moby while a body is in, else Ratchet's (`ratchet`, 0x13fdd8).
    pub fn hero_moby(&self, ratchet: MobyId) -> MobyId {
        match (self.mode, self.bodies.moby) {
            (0, _) | (_, None) => ratchet,
            (_, Some(b)) => b,
        }
    }

    /// The loader's joint lists of the body classes ([`BodyJoints`]).
    pub fn set_body_joints(&mut self, j: Vec<BodyJoints>) { self.fx.joints.bodies = std::sync::Arc::new(j); }

    /// The joint lists of the body moby's class now (None: Ratchet, or not loaded).
    pub fn body_joints(&self) -> Option<&BodyJoints> {
        if self.mode == 0 { return None; }
        self.fx.joints.bodies.iter().find(|b| b.o_class == self.bodies.class)
    }
}

// ------------------------------------------------------------------------------------------------
// The switch, leaving, the per-body idle.

/// The item slots' pass `0x231088` the switch makes (`items::slot_pass` with the tick's item environment).
pub type SlotPass<'a> = &'a mut dyn FnMut(&mut Hero, &mut dyn AnimCtl, &mut Rng);

/// `SwitchCharacter(mode, state, moby)` (0x231348; L00 0x210a08) without the item slots' pass (no item environment:
/// unit tests).
pub fn switch_character(h: &mut Hero, c: &mut Ctx, mode: u8, state: i32, b: BodyMoby) { switch_character_with(h, c, mode, state, b, None) }

/// `SwitchCharacter(mode, state, moby)` (0x231348; L00 0x210a08): see the module doc's coverage table. `pass` = the
/// item slots' pass `0x231088` at +2.
pub fn switch_character_with(h: &mut Hero, c: &mut Ctx, mode: u8, state: i32, b: BodyMoby, pass: Option<SlotPass>) {
    // 0x22efd8: a raised weapon put away.
    super::weapons::put_away(h);
    h.items.f13f6 = 1;
    // 0x231088: one pass of the item slots' updates.
    if let Some(pass) = pass { pass(h, &mut *c.anim, &mut *c.rng); }
    // 0x1413d0 = Ratchet; 0x277740(0x1409c0): his after-images end; his +0x98 = −1.
    h.fx.trails.hero.kill();
    h.bodies.cmds.push(BodyCmd::RatchetCollision(0xffff_ffff));
    // 0x2486c0: Ratchet and his items hidden.
    h.bodies.cmds.push(BodyCmd::RatchetHidden(true));
    // 0x2283a8: the looping sounds released.
    super::packs::stop_loops(h);
    // 0x227420: the joint records detached.
    h.idle.detach_all();
    h.bodies.detach_all();
    h.mode = mode;
    h.bodies.moby = Some(b.id);
    h.bodies.class = b.o_class;
    // HeroSizeCapsule, then the capsule's top / bottom snapped to their targets.
    h.size_capsule();
    h.cap_bottom = h.cap_bottom_target;
    h.cap_top = h.cap_top_target;
    // The body: no update of its own, matrix kept (+0x34 |= 6).
    h.bodies.cmds.push(BodyCmd::Mode { id: b.id, set: 6, clear: 0 });
    // 0x13fde0 = 0x13fde4 = 1; the animation now the body's; 0x247d00.
    h.anim_speed = Pf::ONE;
    c.anim.bind_body(Some((b.id, b.o_class, b.anim)));
    c.anim.clear_loop();
    h.bodies.state_param = state;
    match mode {
        body::CLANK => {
            h.bodies.saved_health = h.health as i16;
            h.health = h.bodies.clank_health;
        }
        body::GIANT => h.bodies.energy = 200,
        _ => {}
    }
    h.set_state(c, state, true);
}

/// Leaving the body (0x231450; L00 0x210b30): see the module doc's coverage table. `game_mode` = 0x15f5c4.
pub fn leave_body(h: &mut Hero, c: &mut Ctx, game_mode: i32) {
    if h.mode == body::CLANK {
        h.bodies.clank_health = h.health;
        h.health = h.bodies.saved_health as i32;
        if let Some(id) = h.bodies.rotor.take() { h.bodies.cmds.push(BodyCmd::Delete(id)); }
        if let Some(id) = h.bodies.glow.take() { h.bodies.cmds.push(BodyCmd::Delete(id)); }
    }
    if h.mode == body::GIANT && h.bodies.energy_hud != -1 {
        // 0x24b090(0x140984, 0): the energy bar released (crate::hud releases its element once the body is 0).
        h.bodies.energy_hud = -1;
    }
    h.idle.detach_all();
    h.bodies.detach_all();
    h.mode = body::RATCHET;
    if let Some(id) = h.bodies.moby.take() { h.bodies.cmds.push(BodyCmd::Mode { id, set: 0, clear: 6 }); }
    if let Some(id) = h.bodies.disguise.take() { h.bodies.cmds.push(BodyCmd::Delete(id)); }
    super::packs::stop_loops(h);
    // 0x1413d0 = Ratchet, placed at the hero; +0x98 = 0; 0x13fde4 = 1; 0x247d00.
    h.bodies.cmds.push(BodyCmd::RatchetPlace([h.pos[0].to_f32(), h.pos[1].to_f32(), h.pos[2].to_f32(), h.pos[3].to_f32()]));
    h.bodies.cmds.push(BodyCmd::RatchetCollision(0));
    c.anim.bind_body(None);
    c.anim.clear_loop();
    if h.state != 100 || (game_mode != 2 && game_mode != 6) { h.set_state(c, 0, true); }
}

/// The per-body idle `0x227638`.
pub fn body_idle(h: &mut Hero, c: &mut Ctx) {
    let s = match h.mode {
        body::RATCHET => 0,
        body::CLANK => clank::IDLE,
        body::GIANT => giant::IDLE,
        body::DISGUISE => 0x53,
        _ => return,
    };
    h.set_state(c, s, true);
}

/// A class's `SwitchCharacter(mode, state, moby)` (the level's copy): queued as a hero call (made right after the moby
/// loop, before the hero update; `services::HeroFields::run_calls`).
pub fn queue_switch(w: &mut crate::moby_update::services::World, mode: u8, state: i32, moby: MobyId) {
    let (o_class, anim) = { let m = w.m(moby); (m.o_class, m.anim) };
    w.hero_fields_mut().call(crate::moby_update::services::HeroCall::SwitchCharacter { mode, state, moby, o_class, anim });
}

/// The death reload's body restore (`0x29adc8`): with a checkpoint body 1 / 2, the first moby of the table of class 0x57
/// / 0x1a3 (none: no switch) becomes [`Bodies::restore`], made by the next tick. Body 0 and 3 restore nothing.
pub fn restore_from_checkpoint(h: &mut Hero, table: &crate::moby_runtime::MobyTable, body: u8, state: i32) {
    let class = match body {
        body::CLANK => CLANK_CLASS,
        body::GIANT => GIANT_CLASS,
        _ => return,
    };
    let Some(id) = table.mobys.iter().take_while(|m| m.state != crate::moby_runtime::state::END).position(|m| m.o_class == class) else { return };
    let m = &table.mobys[id];
    h.bodies.restore = Some((body, state, BodyMoby { id, o_class: m.o_class, anim: m.anim }));
}

/// A class's call of its level's leave-the-body copy (as a hero call, [`queue_switch`]).
pub fn queue_leave(w: &mut crate::moby_update::services::World) {
    let game_mode = w.svc.game_mode;
    w.hero_fields_mut().call(crate::moby_update::services::HeroCall::LeaveBody { game_mode });
}

/// The commands to the moby table, in order (`ratchet` = Ratchet's moby 0x13fdd8).
pub fn apply_cmds(h: &mut Hero, table: &mut crate::moby_runtime::MobyTable, ratchet: MobyId, hits: &mut dyn super::items::HitSink, counter: u64) {
    for cmd in std::mem::take(&mut h.bodies.cmds) {
        match cmd {
            BodyCmd::Mode { id, set, clear } => {
                if let Some(m) = table.mobys.get_mut(id) { m.mode = (m.mode | set) & !clear; }
            }
            BodyCmd::RatchetHidden(on) => {
                if let Some(m) = table.mobys.get_mut(ratchet) {
                    if on { m.mode |= crate::moby_runtime::mode::HIDDEN } else { m.mode &= !crate::moby_runtime::mode::HIDDEN }
                }
            }
            BodyCmd::RatchetCollision(v) => {
                if let Some(m) = table.mobys.get_mut(ratchet) { m.coll_disable = v; }
            }
            BodyCmd::RatchetPlace(p) => {
                if let Some(m) = table.mobys.get_mut(ratchet) { m.position = p; }
            }
            BodyCmd::Delete(id) => {
                if table.mobys.get(id).is_some_and(|m| m.state < 0x80) { hits.delete_moby(table, id, counter); }
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------
// The body's hero update (HeroUpdateAlt, L00 0x2062b0): the hero part.

/// `HeroUpdateAlt` up to the write-back, on the body moby `moby` (see the module doc's table, rows +0..+12).
pub(super) fn body_update(h: &mut Hero, moby: &mut Moby, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng, sounds: &mut dyn HeroSounds, counter: i32) {
    // RatchetAnimAdvance on the body moby (its class's sound triggers and loop sound).
    let before = anim.view();
    anim.advance(h.anim_speed);
    sounds.anim_advanced(moby, &before, &anim.view(), rng);
    anim.refresh_loop_sound(moby, rng, sounds);
    h.input_physics_move(env, anim, rng);
    // `HeroTickStateTimer`'s end of the disguise's timer in body 3: leave the body (super::hologuise).
    if h.gadgets.disguise.due.is_some() {
        let mut c = Ctx { env, anim: &mut *anim, rng: &mut *rng, voice: None };
        super::hologuise::leave(h, &mut c);
    }
    super::packs::flush_sounds(h, moby, sounds, rng);
    super::surface::flush(h, moby, sounds, rng);
    h.surface_reaction(env, anim, rng);
    super::surface::flush(h, moby, sounds, rng);
    if h.mode == body::CLANK {
        super::ledge::wall_ledge_probe_b(h, env, &anim.view());
        clank::lean(h);
    }
    let group = h.group;
    {
        let m: &Moby = moby;
        let mut voice = |index: i32, flags: u32, r: &mut Rng| { sounds.voice(m, index, flags, r); };
        h.transitions_with_voice(env, anim, rng, Some(&mut voice));
    }
    super::packs::after_transitions(h, moby, sounds, rng, group);
    super::surface::after_transitions(h, moby, sounds, rng, group);
    super::damage::flush(h, moby, sounds, rng);
    if h.mode == body::RATCHET { return; }
    super::fx::flush(h, moby, sounds, rng);
    super::fx::voice_queue(h, moby, sounds, rng);
    springs(h);
    if h.mode == body::RATCHET { return; }
    h.write_back(moby);
    if !h.joint_targets.is_empty() || h.body_joints().is_some() {
        moby.joint_mods = h.body_joints().map(|j| h.bodies.modifiers(&j.targets)).unwrap_or_default();
    }
    // `0x229158`: the squash on the body moby (super::hologuise); then body 3's part of `HeroUpdateAlt`.
    super::hologuise::squash(h, moby);
    super::hologuise::body_update(h, &*anim, counter);
    // 0x2278c0 on the body (bodies 1 / 2).
    if matches!(h.mode, body::CLANK | body::GIANT) {
        if h.mode == body::CLANK { h.bodies.joints[rec::R25].scale = h.bodies.scale18; }
        // +19, body 2: the cheat 0x15edb3 → record 28's scale = 0x15ee18 · 1.4.
        if h.mode == body::GIANT && h.cheats.on(crate::cheats::slot::CLANK) { h.bodies.joints[rec::R28].scale = h.bodies.scale18 * 1.4; }
        glow(h, counter);
    }
}

/// `0x2273d0` in a body: every record in block order — Ratchet's (kind 0) skipped, Clank's back record (kind 1) while his
/// back moby exists, the body's (kinds 2 / 3) attached to the body moby.
fn springs(h: &mut Hero) {
    let clank_back = h.back.is_some();
    for r in h.idle.joints.iter_mut().filter(|r| r.kind == 1) {
        if clank_back { r.update(); }
    }
    let b = &mut h.bodies;
    for k in 0..b.joints.len() {
        b.joints[k].update();
        let i = k as u8;
        if b.joints[k].attached {
            if !b.manips.contains(&i) { b.manips.insert(0, i); }
        } else {
            b.manips.retain(|&x| x != i);
        }
    }
}

/// `0x2278c0`'s first call: `Approach(1.0, 0.05, &0x15ee18)`, 1.8 (0x3fe66666) with the cheat 0x15edb3 (in every
/// body: the hero update `0x228870` and `HeroUpdateAlt`).
pub fn approach_head_scale(h: &mut Hero) {
    let t = if h.cheats.on(crate::cheats::slot::CLANK) { f32::from_bits(0x3fe6_6666) } else { 1.0 };
    let s = &mut h.bodies.scale18;
    *s += (t - *s).clamp(-0.05, 0.05);
}

/// `0x2278c0` in bodies 1 / 2: 0x15ee18 → 1.0 (0.05 a tick), the glow pulse on the hero moby (+0x90, mode |= 0x10;
/// `idle::clank_glow_word`); the blink is Ratchet-mode only.
pub fn glow(h: &mut Hero, counter: i32) {
    approach_head_scale(h);
    h.bodies.body_glow = Some(super::idle::clank_glow_word(counter, h.health, h.f53e));
}

/// The rest of `HeroUpdateAlt` that needs the moby table, after the hero update and its write-back (the tick):
/// the glow word into the body moby, Clank's antenna glow moby 0x4b4 and rotor 0x47a, the command flash, Giant Clank's
/// pilot, the effect mobys Giant Clank's states fire and the hits of the body's attacks. `ratchet` = Ratchet's moby.
#[allow(clippy::too_many_arguments)]
pub fn after_update(
    h: &mut Hero,
    table: &mut crate::moby_runtime::MobyTable,
    ratchet: MobyId,
    anim: &mut dyn AnimCtl,
    hits: &mut dyn super::items::HitSink,
    sounds: &mut dyn HeroSounds,
    rng: &mut Rng,
    counter: u64,
) {
    apply_cmds(h, table, ratchet, hits, counter);
    let Some(body) = h.bodies.moby.filter(|_| h.mode != 0) else {
        h.bodies.hits.clear();
        h.bodies.spawns.clear();
        return;
    };
    // `0x22a110`: Ratchet stays hidden (`0x2486c0` every tick); the body's hit slot is consumed.
    if let Some(m) = table.mobys.get_mut(ratchet) { m.mode |= crate::moby_runtime::mode::HIDDEN; }
    // The body's pose snapshot into its slot (the joint points and the draw read it when its key A is a snapshot).
    let snap = anim.body_snapshot();
    if anim.bound_body() == Some(body) {
        let hero = h.clone();
        hits.world(table, &hero, rng, counter, &mut |w| {
            if w.svc.snapshots.len() <= body { w.svc.snapshots.resize(body + 1, None); }
            w.svc.snapshots[body] = snap.clone();
        });
    }
    if let Some(w) = h.bodies.body_glow.take() {
        if let Some(m) = table.mobys.get_mut(body) {
            m.mode |= crate::moby_runtime::mode::GLOW;
            m.glow = w;
        }
    }
    if h.mode == body::CLANK { clank::after_update(h, table, body, hits, sounds, rng, counter); }
    // Body 3: the hero's draw callback `0x229440` (registered once a frame by `0x229400`): the disguise's glow quads.
    if h.mode == body::DISGUISE {
        let hero = h.clone();
        hits.world(table, &hero, rng, counter, &mut |w| {
            w.svc.draw_callbacks.register(crate::moby_update::classes::draw_callbacks::Callback::DisguiseGlow, body);
            w.svc.draw_callbacks.disguise = Some((hero.gadgets.disguise.glow_points, hero.gadgets.disguise.tint));
        });
    }
    if h.mode == body::GIANT { giant::after_update(h, table, ratchet, body, anim, hits, rng, counter); }
    // HeroUpdateAlt +14 / +21: the body shown → joint list 7 (Clank) / 6 (Giant Clank)'s point into 0x1410d0 and the
    // hero's draw callback `0x229440` registered (`0x229400`): the antenna dot (super::glow).
    if h.mode == body::CLANK || h.mode == body::GIANT {
        h.glow.dot = None;
        let shown = table.mobys.get(body).is_some_and(|m| m.mode & crate::moby_runtime::mode::HIDDEN == 0);
        if shown {
            let list = if h.mode == body::CLANK { 7 } else { 6 };
            let hero = h.clone();
            let mut point = None;
            hits.world(table, &hero, rng, counter, &mut |w| {
                point = Some(w.joint_point(body, list));
                w.svc.draw_callbacks.register(crate::moby_update::classes::draw_callbacks::Callback::HeroGlow, body);
            });
            if let Some(p) = point { super::glow::body_dot(h, p); }
        }
    }
    // The hits the states queued, on the moby world (`0x26e830`, `coll_sphere_mobys`).
    let pending = std::mem::take(&mut h.bodies.hits);
    if !pending.is_empty() {
        let hero = h.clone();
        hits.world(table, &hero, rng, counter, &mut |w| {
            for &hit in &pending {
                match hit {
                    BodyHit::Sphere { r, damage, push, centre, flags, b18, b19, sphere_flags } => {
                        crate::moby_update::creature::attack::sphere_hit(w, r, damage, push, body, [centre[0], centre[1], centre[2], 0.0], flags, b18, b19, sphere_flags);
                    }
                    BodyHit::Punch { r, damage, centre, dir, flags } => giant::punch(w, body, r, damage, centre, dir, flags),
                }
            }
        });
    }
}
