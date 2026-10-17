//! **Weapons: the throw gloves and the Bomb Glove** (level01; docs/plan/hero_states.md "Weapons + first person").
//!
//! * **The weapon check's throw case** (`HeroPdaGadget` 0x240ed8, items 10 / 0x11 / 0x14 / 0x18 / 0x19: the Bomb
//!   Glove, Mine Glove, Glove of Doom, Drone Device, Decoy Glove; [`fire`], the row's `fire` in
//!   `super::gadgets::HAND_ITEMS`): ○ within 7 ticks (at most the ticks the item has been ready), or ○ held while the
//!   item became ready 9..16 ticks ago, in the standing / walking / crouching / gliding / grinding groups (not with the
//!   weapon already out 0x1413f8, not in 0x1e, 0x23 only past frame 25): with ammo (`0x249530`) → the throw state
//!   **0x23** when crouched, standing, stopping or walking slowly (stick < 0.7), else the weapon arm while moving
//!   (`0x22ee08`, [`draw_weapon`]); without ammo the glove's class sound 0 (the empty click).
//! * **State 0x23 "glove throw"** (group 6; entry [`throw_entry`] after `super::melee`'s group-6 entry:
//!   `SetAnim(9, 0x2c, 7)`; physics [`physics`]: aim at the stick (`0x2351d0(11, 50°, −1)`) or turn to the aim at
//!   15 rad/s, `SpeedStep(30, 35)·dt²`, gravity 54 (25 in the air, with the steep-wall stop); transitions
//!   [`transitions`]: ✕ within 11 ticks after key time 19 jumps (crouched: the crouch jumps), idle from 21).
//! * **The weapon arm** (0x1413f8, `0x22ee08`, [`draw_weapon`]): out for the moving throw; its timer 0x13f50c counts
//!   in the post-move (`super::physics`). The arm is a **pose layer** on Ratchet's moby ([`super::anim::AnimLayer`],
//!   `FUN_00263e08` on his joint list 12 (gp−0x7560; a second one on list 13 when 0x1413fb = 2), kept in 0x140058 /
//!   0x14005c: [`Weapons::layers`]): it plays the item's upper-body sequence (`def +0x28`, `+0x2c` crouched) from
//!   Ratchet's current key over `ticks(10)` (item +0x18 ≠ 0) or `ticks(11)` at full weight (`FUN_002641c0`), while
//!   the main animation (the run, the walk, the jump) keeps playing on the other joints. Upkeep `0x22f068`
//!   ([`arm_upkeep`]): re-blend over 8 ticks when the item's layer sequence changed (crouching), advance
//!   (`FUN_00263f70`), weight → 1 by 0.2 a tick; when the layer sequence wraps with 0x1413fa clear the arm is down
//!   (0x1413f8 = 0) and the layer fades (0x140064 = 1: speed 0, weight → 0 by 0.07 a tick; 0.25 after a holster,
//!   0x140064 = 2) and is freed at 0. The renderer blends it over those joints
//!   (`rc_formats::moby_anim::evaluate_layered`). The holster check `0x2405f8` ([`holster_check`]: stopped within 6
//!   ticks of the draw → put away, `0x140064 = 2`, state 0x23). The Comet-Strike (0x15) is a full-body state: the
//!   wrench's code never calls `0x22ee08`, so it has no layer.
//! * **The gloves' item update** (the Bomb Glove's `0x2d8330` and its three copies): `super::gloves` (one shared
//!   update, a row per glove). The Bomb Glove's row: 3 ticks of warm-up; the hand point in the glove (joint list 0 +
//!   (0, −0.09, −0.02) in its frame); the look stance 1 becomes 0x1e (`SetState(0x1e, 1)`, made right after the slot
//!   loop: [`after_items`]); when the 20-tick fire timer is out: **the throw** — the arm out 17 ticks, ○ in the
//!   first-person stance 0x1e with ammo (the first-person throw: original behaviour), or 0x23 at tick 16 — voice
//!   0x1a, one ammo used (`0x249450`, the ammo-used stat 0x13dea0), the glove's state 3; the aim of the held bomb
//!   every tick (a point 0.86 ahead-left of Ratchet, 0.49 up ([`launch_point`]); 8.5 ahead, or in first person along
//!   the camera, clamped −80°..34°, to where the camera's line hits; `0x2d80f0` ([`launch_velocity`]): 8.5 u/s level,
//!   the vertical speed of the arc with gravity 11 (at most 5.5 u/s), aimed ahead when the target is more than 50°
//!   off Ratchet's facing or closer than 1.5); state 3 releases the bomb (`0x2c27a8`,
//!   `crate::moby_update::classes::bomb::release`) — a new one is created in the glove when there is ammo
//!   (`0x2c2640`, `CreateMoby(0x79)` through the hit sink).
//!
//! **Ammo** ([`Weapons::ammo`]): the game state's table 0x13d428 + id, mirrored like the owned items (the engine
//! copies it in before the tick and back after; `uses_ammo` from the item records, +8).
//!
//! **The aim at a target** (`crate::targeting`): without L1 / L2 held (or with 0x1413fc) the glove's update searches
//! the target list 0x1abe80 ([`crate::targeting::aim_search`] with [`crate::targeting::BOMB_GLOVE`]) and aims the
//! held bomb's arc at the chosen target's aim point (its record's height above it) instead of the point 8.5 ahead;
//! the target is 0x13fda0 ([`Weapons::aim`]), and the throw state 0x23 turns Ratchet to it (SetState 0x23: 0x13fda8 =
//! 1, 0x13fdac = its yaw). The bomb then previews the landing and registers the reticle
//! (`crate::moby_update::classes::bomb`).
//!
//! Not ported: the melee aim-assist `0x22e238` over the same list, the gold gloves (0x13e52a..: 0), the gloves' throw
//! stats records 0x1416d0 / 0x141708 / 0x141720 / 0x141748 (counted in [`Weapons::throws`]), 0x141618.
//!
//! **Weapons that keep the arm raised** (item def +0x30 → 0x1413fa; the Pyrocitor, the Blaster, …): [`gun_stance`]
//! (`0x242858`: the stop 3 (its SetState and its physics) or the walk's slow stop becomes the idle state in the weapon's
//! standing sequence; the walk's SetState does not call it, so Ratchet runs off from the stance while firing),
//! [`arm_on_state_change`] (`0x22eca0`: the arm layers when leaving idle), [`stance_kept`] / [`idle_stance`] (the idle
//! transitions keep the stance), [`put_away`]'s return to the idle sequence (made by [`apply_pending`] where the item
//! update asked for it).
//!
//! **The glove-holding layers** (0x140050 / 0x140054, [`hold_update`] = `0x22e660`; docs/plan/hero_gameplay.md §11):
//! with a hand item whose def +0x18 (0x1413fb) is set, Ratchet's arm(s) replay his current key from the holding
//! classes 1 / 2, so the weapon stays in his hands and level while he runs; the firing arm layer takes over from them.

use super::anim::{AnimCtl, AnimLayer};
use super::physics::*;
use super::states::Ctx;
use super::Hero;
use crate::moby_runtime::{MobyId, MobyTable};
use crate::pad::button;
use crate::ps2v::Pf;

/// The Bomb Glove (item 10).
pub const BOMB_GLOVE: i32 = super::items::item::BOMB_GLOVE;
/// Ratchet's voice of a throw.
pub const THROW_VOICE: i32 = 0x1a;
/// The glove's class sound of an empty throw.
pub const EMPTY_SOUND: i32 = 0;
/// The throw state.
pub const THROW: i32 = 0x23;
const DT: f32 = 1.0 / 60.0;
const N: usize = rc_formats::save_game::ITEM_COUNT;

/// The hand glove's pvars (moby +0x78 of the glove; one record for whichever throw glove is in hand, the shared
/// update `super::gloves`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Glove {
    /// +0x58 (the Mine Glove's +0x5c): warm-up ticks (3; the Bomb and Mine Gloves).
    pub warmup: i32,
    /// +0x50: the object in the glove (the bomb, mine, Doom canister or decoy).
    pub held: Option<MobyId>,
    /// +0x54 (the Mine Glove's +0x58): the fire lockout (20 ticks after a throw).
    pub fire_timer: i32,
    /// +0x40: the hand point in the glove.
    pub hand: [f32; 3],
    /// The glove's joint list 0 world matrix (`MobyAttachToJoint(glove, 0)`): rows 0..2 the rotation, row 3 the
    /// joint point. Written by `gloves::update` step 2; the held mine's frame is this turned by π about x
    /// (`0x2bfe40`'s held branch).
    pub hand_mtx: [[f32; 4]; 4],
    /// The Mine Glove's +0x54: no new mine until it runs out (10 ticks after a throw).
    pub respawn: i32,
}

/// The weapons' fields: the ammo mirror, the Bomb Glove's pvars, the weapon arm.
#[derive(Clone, Debug, PartialEq)]
pub struct Weapons {
    /// 0x13d428 + 4·id: ammo (the game state's, mirrored by the engine before the tick and copied back after).
    pub ammo: [i32; N],
    /// The item records' "uses ammo" (+8 ≠ 0; `0x249530` / `0x249450` read it at 0x1c453e + 0x18·id).
    pub uses_ammo: [bool; N],
    /// 0x13dea0 + 4·id: ammo used (stat), added up here and added to the game state by the engine.
    pub used: [i32; N],
    /// 0x13de08 + 4·id: ammo picked up (stat; the ammo pickups `0x2db028`), added to the game state by the engine.
    pub picked: [i32; N],
    pub glove: Glove,
    /// Ratchet's pose-layer nodes ([`super::anim::PoseNodes`]): 0 / 1 = 0x140058 / 0x14005c, the weapon arm's layers
    /// (joint lists 12 / 13); 2 / 3 = 0x140050 / 0x140054, the glove-holding layers ([`hold_update`]).
    pub layers: super::anim::PoseNodes,
    /// Pose-layer nodes made so far (the creation stamp of [`AnimLayer::born`]: the +0x60 list's order).
    pub nodes_made: u32,
    /// 0x140064: the arm layers' phase (0 in / playing, 1 fading out, 2 fading fast after a holster).
    pub layer_fade: u8,
    /// 0x1415e4: the weapon sequence last set (the layer's, or the standing full-body one).
    pub layer_seq: i32,
    /// 0x1415e8: the item whose arm is out.
    pub arm_item: i32,
    /// A SetState the item update asked for (`SetState(0x1e, 1)` of the glove update, the Visibomb's launch
    /// `SetState(0x1d, 1)`), made right after the slot loop by [`after_items`].
    pub deferred: Option<i32>,
    /// Throws (the stats record 0x1416d0 is not applied).
    pub throws: u32,
    /// The item definitions' weapon fields (`0x22ee08`: the arm's sequences), by item id, from the level's item
    /// table (the engine sets them; none: no arm sequences).
    pub defs: Vec<super::items::WeaponDef>,
    /// 0x13e520 + id: the gold weapons (the Pyrocitor reads its own, 0x13e530), mirrored from the saved game every
    /// tick by the engine.
    pub gold: [u8; N],
    /// The Pyrocitor's pvars ([`super::pyrocitor`]).
    pub pyro: super::pyrocitor::Pyro,
    /// `0x22ee08` / `0x22efd8`'s standing `SetAnim` asked for by an item update (which has no mutable animation in
    /// the port), made right after the slot loop by [`after_items`].
    pub pending_draw: bool,
    pub pending_idle: bool,
    /// 0x13fda0: the glove's target (the last search's; kept by the first-person aim), and its position (moby +0x10)
    /// as the moby loop left it ([`refresh_aim`]).
    pub aim: Option<MobyId>,
    pub aim_pos: [f32; 3],
    /// The Blaster's pvars ([`super::blaster`]).
    pub blaster: super::blaster::Blaster,
    /// The Devastator's pvars ([`super::devastator`]).
    pub devastator: super::devastator::Devastator,
    /// Ratchet's `SetAnim(blend, seq, frame)` (`0x247a90`) an item update asked for (the Devastator's standing shot 54),
    /// made right after the slot loop by [`after_items`].
    pub pending_anim: Option<(i32, u8, i32)>,
    /// The Tesla Claw's pvars and chain ([`super::tesla`]).
    pub tesla: super::tesla::Tesla,
    /// The R.Y.N.O.'s pvars ([`super::ryno`]).
    pub ryno: super::ryno::Ryno,
    /// 0x1694c0: the screen markers the guns register this tick (`FUN_0020fb60`; drawn by the engine).
    pub markers: crate::targeting::Markers,
    /// The Suck Cannon's, the Taunter's and the Morph-o-Ray's pvars ([`super::reactive`]).
    pub reactive: super::reactive::Reactive,
    /// The Visibomb's pvars ([`super::visibomb`]).
    pub visibomb: super::visibomb::Visibomb,
}

impl Default for Weapons {
    fn default() -> Self {
        Weapons {
            ammo: [0; N], uses_ammo: [false; N], used: [0; N], picked: [0; N], glove: Glove::default(), layers: [None; 5], nodes_made: 0, layer_fade: 0, layer_seq: 0,
            arm_item: 0, deferred: None, throws: 0, defs: Vec::new(), gold: [0; N], pyro: Default::default(), pending_draw: false, pending_idle: false,
            aim: None, aim_pos: [0.0; 3], blaster: Default::default(), ryno: Default::default(), devastator: Default::default(), tesla: Default::default(), pending_anim: None, markers: Default::default(), reactive: Default::default(), visibomb: Default::default(),
        }
    }
}

impl Weapons {
    /// `0x249530(id)`: the ammo, or 1 for an item without ammo.
    pub fn has_ammo(&self, id: i32) -> i32 {
        let i = id.max(0) as usize;
        if !self.uses_ammo.get(i).copied().unwrap_or(false) { return 1; }
        self.ammo.get(i).copied().unwrap_or(0)
    }
    /// `0x249450(id, n)`: use `n` (false when there are fewer).
    pub fn use_ammo(&mut self, id: i32, n: i32) -> bool {
        let i = id.max(0) as usize;
        if !self.uses_ammo.get(i).copied().unwrap_or(false) { return true; }
        let Some(a) = self.ammo.get_mut(i) else { return false };
        if n <= *a {
            *a -= n;
            self.used[i] += n;
            return true;
        }
        false
    }
}

// ------------------------------------------------------------------------------------------------
// The weapon check's throw case

/// `HeroPdaGadget` 0x240ed8, cases 10 / 0x11 / 0x14 / 0x18 / 0x19.
pub fn fire(h: &mut Hero, c: &mut Ctx) {
    let g = h.group;
    let st = h.state;
    let frame = c.anim.view().frame;
    let blocked = (g != 0 || st == 0x1e)
        && g != 1 && g != 0xc && g != 5 && g != 0xf
        && (g != 4 || h.jump.f7fc == 0)
        && (st != THROW || frame <= 25.0);
    if blocked || h.f13f8 != 0 { return; }
    let pad = c.env.pad;
    let ready = h.items.slot.ticks_ready;
    let mask = h.items.slot.fire_mask;
    let n = ticks(7).min(ready);
    let pressed = pad.pressed_within(mask, n).is_some();
    let held = pad.held & button::CIRCLE != 0 && ticks(8) < ready && ready < ticks(0x11);
    if !pressed && !held { return; }
    let id = h.items.slot.id;
    if h.weapons.has_ammo(id) != 0 {
        if g == 0xc || g == 0 || st == 3 || (g == 1 && h.stick_mag < Pf::b(0x3f33_3333)) {
            h.set_state(c, THROW, true);
        } else {
            draw_weapon(h, c);
        }
    } else if h.items.slot.item.is_some() {
        h.fx.item_sounds.push(EMPTY_SOUND);
    }
}

/// gp−0x7560: Ratchet's joint lists of the two arm layers (and of the two glove-holding layers).
pub const ARM_LISTS: [u8; 2] = [12, 13];

/// `FUN_00263e08(ratchet, list)`: a fresh pose-layer node, linked in front of the +0x60 list (its stamp).
fn new_node(h: &mut Hero, list: u8) -> AnimLayer {
    h.weapons.nodes_made += 1;
    AnimLayer { born: h.weapons.nodes_made, ..AnimLayer::new(list) }
}

/// The glove-holding layers' slots in [`Weapons::layers`] (0x140050 / 0x140054).
pub const HOLD: usize = 2;
/// The face layer's slot in [`Weapons::layers`] (0x140060, [`face_update`]).
pub const FACE: usize = 4;
/// Ratchet's joint list of the face layer.
pub const FACE_LIST: u8 = 0x18;

/// `0x22e350(kind)`: a face byte (the look target's record +0x3c, 0x14161e) as Ratchet's face sequence: 1 → 0x79,
/// 2 → 0x77, 3 → 0x75, any other → 0.
fn face_seq(kind: i32) -> u8 {
    match kind {
        1 => 0x79,
        2 => 0x77,
        3 => 0x75,
        _ => 0,
    }
}

/// **The face layer** `0x22e3a8` (last in the items' upkeep `0x22f390`, after `0x22f068`): a pose layer on Ratchet's
/// joint list 0x18 (his face) plays one of his face sequences while one of these holds, the later winning:
/// * the head has a look target (0x1415c4, [`super::pose`]'s `HeroScanTargets`: an enemy in range ahead): sequence
///   0x79 (the glare), or the one of the target record's byte +0x3c when it is set ([`face_seq`]; weight 0.7 then in
///   group 1);
/// * the swap timer 0x13f52c runs (set with 0x14161e = 3 for 70 ticks when the Suck Cannon, Devastator, Visibomb,
///   Pyrocitor or R.Y.N.O. comes into the hand, `0x2306c0`): the sequence of 0x14161e;
/// * one hit point left (0x1415f8 = 1): 0x7b at 0.8, but not wanted on his sequences 0x54 / 0x82 / 0x5a unless he
///   is blending;
/// * under water (group 0x11) without the O2 Mask: 0x78, weight `min(1, 1.25 − air / 10000)`;
/// * the Blaster, the Pyrocitor or the Tesla Claw ready (15 / 16 / 19) with the weapon out (0x1413f8): 0x76 at 0.8.
///
/// Wanted: the node is made (weight 0) if missing, its weight approaches the target by 0.05 a tick (`0x270728`), and
/// it plays the sequence (a new one from frame 0 over `ticks(8)` with one step at once, `FUN_002641c0`; the same one
/// advanced, `FUN_00263f70`). Not wanted: the weight goes to 0 by 0.05 a tick, the node freed at 0 (no advance).
pub fn face_update(h: &mut Hero, anim: &dyn AnimCtl, table: &MobyTable) {
    let mut seq = 0u8;
    let mut weight = 1.0f32;
    let mut want = false;
    if h.idle.look_target != 0 {
        want = true;
        seq = 0x79;
        let kind = table.mobys.get(h.idle.look_target as usize - 1).and_then(|m| Some(*m.pvars.get(crate::targeting::record(m)? + 0x3c)?));
        if let Some(k) = kind.filter(|&k| k != 0) {
            seq = face_seq(k as i8 as i32);
            if h.group == 1 { weight = 0.7; }
        }
    }
    if h.items.f52c != 0 {
        want = true;
        seq = face_seq(h.items.f161e as i32);
    }
    if h.health == 1 {
        want = true;
        let v = anim.view();
        if matches!(v.seq_b, 0x54 | 0x82 | 0x5a) && !v.blending() { want = false; }
        weight = 0.8;
        seq = 0x7b;
    }
    if h.group == 0x11 && !h.owned.has(super::swim::ITEM_O2_MASK) {
        want = true;
        seq = 0x78;
        weight = ((1.0 - h.swim.oxygen as f32 / 10000.0) + 0.25).min(1.0);
    }
    let ready = h.items.ready_item().map_or(-1, |_| h.items.slot.id);
    if matches!(ready, 15 | 16 | 19) && h.f13f8 != 0 {
        weight = 0.8;
        want = true;
        seq = 0x76;
    }
    if want {
        if h.weapons.layers[FACE].is_none() {
            let l = new_node(h, FACE_LIST);
            h.weapons.layers[FACE] = Some(AnimLayer { weight: 0.0, ..l });
        }
    } else {
        let Some(l) = h.weapons.layers[FACE].as_mut() else { return };
        approach_f(&mut l.weight, 0.0, 0.05);
        if l.weight == 0.0 { h.weapons.layers[FACE] = None; }
        return;
    }
    let Some(l) = h.weapons.layers[FACE].as_mut() else { return };
    approach_f(&mut l.weight, weight, 0.05);
    if l.seq_b == seq { l.advance(anim); } else { l.start(anim, seq, 0, ticks(8), true); }
}

/// What the hand swap `UpdateWrenchSelected` 0x2307e0 does to the weapons when it commits a change (before the item's
/// blend to its sequence 2 and slot state 3): a weapon out is put away (0x1413f8 → `0x22efd8`), and both glove-holding
/// nodes are told to fade out for good (0x140050 / 0x140054 +0x34 = 1): [`hold_update`] fades them by 0.1 a tick and
/// frees them, and only then makes the new item's layers (weight 0, fading in). Without it a layer whose item no longer
/// wants it would hold its weight: `0x22e660` fades an unwanted node and then, in its common tail, brings it back to 1.
pub fn swap_commit(h: &mut Hero) {
    if h.f13f8 != 0 { put_away(h); }
    for l in h.weapons.layers[HOLD..FACE].iter_mut().flatten() { l.kill = true; }
}

/// `0x22df10(seq)`: Ratchet's key-B sequences in which the glove-holding layers leave his arms to a fixed pose (the
/// holding class's sequence 0): 0x1c / 0x1d / 0x1f, 0x31 / 0x32, 0x37, 0x4a / 0x4b, 0x50, 0x60, 0x6d / 0x6e. (The
/// function also returns an arm angle for 0x1c..0x1f past key time 25 (−75°) and 0x37 (−55°), which `0x22e660` writes
/// to the joint records 8 / 9: not ported, gaps.md G-WPN-011.)
///
/// The board levels' copies (level05 `0x23b378`, level16 `0x20afa0`) first answer yes on the board (group 0x16) with
/// board weapons held (0x13fc1e) and Ratchet's key B in 0x69..0x6c (Kalebo III's board weapon; 0 elsewhere).
pub fn hold_special(h: &Hero, seq: u8) -> bool {
    if h.group == super::hoverboard::GROUP && h.board.weapons != 0 && (0x69..=0x6c).contains(&seq) { return true; }
    matches!(seq, 0x1c | 0x1d | 0x1f | 0x31 | 0x32 | 0x37 | 0x4a | 0x4b | 0x50 | 0x60 | 0x6d | 0x6e)
}

/// **The glove-holding layers** `0x22e660` (first in the items' upkeep `0x22f390`, before `0x22f068`): while the hand
/// item's def byte +0x18 (0x1413fb) is set, a pose layer on Ratchet's joint list 12 (his right arm, the hand item's)
/// and, with 0x1413fb = 2, one on list 13 (his left arm) play **his current key from the holding classes 1 / 2**
/// (node +0x18 = `0x197780[0x198040[i + 1]]`; the keys of his first 0x17 sequences are read from them: the walk, the
/// run, the idle, the jumps … with the arms holding the weapon), so the gun stays in both hands and level while his
/// legs and body run. Per layer:
/// * made (weight 0) when wanted, faded out by 0.1 a tick and freed when not (0x1413fb clear, list 13 with 0x1413fb =
///   1), or for good once the hand item is hidden (0x1413ff: +0x34);
/// * weight → 1 by 0.1 a tick, but → 0 while the weapon arm's layer ([`draw_weapon`]) is out and not fading
///   (0x140058, 0x140064 = 0): the firing arm replaces the holding one;
/// * its keys follow Ratchet's (`+0x52`/`+0x53` sequences, `+0x50`/`+0x51` frames, t `+0x54`; key A only when he is
///   not blending): a new sequence while he blends is taken as key B over his blend's t from the layer's last key;
///   when the layer is itself between two sequences it first finishes that blend at 0.15 a tick (+0x30);
/// * in the special poses of [`hold_special`] it blends to the holding class's sequence 0 over `ticks(15)` and plays
///   it (speed 1), and on the way out blends back to his sequence at 0.1 a tick (+0x32 / +0x33);
/// * its advance (`FUN_00263f70`, speed 0 outside the special poses: no key steps of its own).
///
/// `FUN_0022def8`'s 0xff → Ratchet +0xa5 mapping never applies: the layer copies key A only when Ratchet is not
/// blending, so never his snapshot key (0xff).
pub fn hold_update(h: &mut Hero, anim: &dyn AnimCtl) {
    let v = anim.view();
    for (i, &list) in ARM_LISTS.iter().enumerate() {
        let slot = HOLD + i;
        let mut node = h.weapons.layers[slot];
        if node.is_none_or(|l| !l.kill) {
            let mut want = h.items.f13fb != 0 && (i == 0 || h.items.f13fb == 2);
            if h.f13ff != 0 {
                want = false;
                if let Some(l) = node.as_mut() { l.kill = true; }
            }
            if want {
                if node.is_none() {
                    let mut l = new_node(h, list);
                    l.weight = 0.0;
                    l.alt = i as u8 + 1;
                    node = Some(l);
                }
            } else if let Some(l) = node.as_mut() {
                approach_f(&mut l.weight, 0.0, 0.1);
                if l.weight == 0.0 {
                    h.weapons.layers[slot] = None;
                    continue;
                }
            }
        } else if let Some(l) = node.as_mut() {
            approach_f(&mut l.weight, 0.0, 0.1);
            if l.weight == 0.0 {
                h.weapons.layers[slot] = None;
                continue;
            }
        }
        let Some(mut l) = node else { continue };
        let steady = v.seq_a == v.seq_b;
        if h.weapons.layers[0].is_none() || h.weapons.layer_fade != 0 {
            if !l.kill { approach_f(&mut l.weight, 1.0, 0.1); }
        } else {
            approach_f(&mut l.weight, 0.0, 0.1);
        }
        // +0x30: Ratchet blending (0x13fdec) into another sequence than the layer's key B while the layer is itself
        // between two sequences.
        l.own_blend = !steady && v.blending() && l.seq_b != v.seq_b && l.seq_a != l.seq_b;
        let special = hold_special(h, v.seq_b);
        if !special && !l.special {
            if !l.own_blend {
                if steady { l.seq_a = v.seq_a; }
                l.seq_b = v.seq_b;
                if steady { l.frame_a = v.frame_a; }
                l.frame_b = v.frame_b;
                l.t = v.t;
            } else {
                l.t += 0.15;
                if 1.0 <= l.t {
                    l.seq_a = l.seq_b;
                    l.frame_a = l.frame_b;
                    l.own_blend = false;
                }
            }
        } else if !special {
            // Out of the special poses: back to his key B from frame 0 over 10 ticks.
            if !l.back {
                l.seq_b = v.seq_b;
                l.frame_b = 0;
                l.back = true;
                l.t = 0.0;
                l.speed = 0.0;
            } else {
                l.t = (l.t + 0.1).min(1.0);
                if l.t == 1.0 { l.special = false; }
            }
        } else {
            if !l.special || l.seq_b != 0 {
                if l.seq_a == l.seq_b || 0.5 < l.t {
                    l.seq_a = l.seq_b;
                    l.frame_a = l.frame_b;
                }
                l.t = 0.0;
                l.seq_b = 0;
                l.frame_b = 0;
                l.rate = 1.0 / ticks(15) as f32;
                l.special = true;
                l.back = false;
            }
            l.speed = 1.0;
        }
        l.advance(anim);
        h.weapons.layers[slot] = Some(l);
    }
}

/// `0x22ee08`: the weapon out (0x1413f8 = 1, 0x1413fa = def +0x30, 0x1415e8 = the item): standing (group 0 and
/// no 0x141618) `SetAnim(10 or 11, def +0x24, 0)`; else the arm layer(s) with `def +0x28` (`+0x2c` crouched) over the
/// same blend at full weight (see the module doc).
pub fn draw_weapon(h: &mut Hero, c: &mut Ctx) {
    let id = h.items.slot.id;
    if !(1 < id + 1) { return; }
    let def = h.weapons_def(id);
    h.f13f8 = 1;
    h.f13fa = def.w30 as u8;
    h.weapons.arm_item = id;
    let blend = if def.w18 != 0 { ticks(10) } else { ticks(11) };
    if h.group == 0 && !h.arm_at_wall {
        h.weapons.layer_seq = def.anims[0];
        if def.anims[0] >= 0 { h.set_anim(c.anim, c.rng, Pf::from_i32(blend), def.anims[0] as u8, 0); }
        return;
    }
    let seq = if h.group == 0xc { def.anims[2] } else { def.anims[1] };
    if seq == -1 { return; }
    for (i, &list) in ARM_LISTS.iter().enumerate() {
        let mut l = new_node(h, list);
        h.weapons.layer_fade = 0;
        h.weapons.layer_seq = seq;
        l.weight = 1.0;
        l.start(&*c.anim, seq.max(0) as u8, 0, blend, true);
        h.weapons.layers[i] = Some(l);
        if i == 0 && h.items.f13fb != 2 { return; }
    }
}

/// `0x22efd8`: the weapon put away (0x1413fa = 0, 0x1413f8 = 0); an arm layer out fades (0x140064 = 1); without one,
/// standing in the weapon's stance (state 0, key B on 0x1415e4), Ratchet blends back to his idle sequence
/// (`SetAnim(−2, idle)`: made by [`after_items`] / [`apply_pending`], where the animation can be changed).
pub fn put_away(h: &mut Hero) {
    // A draw the same item update asked for earlier (made after the slot loop in the port) came first in the game.
    h.weapons.pending_draw = false;
    if h.f13f8 == 0 { return; }
    if h.weapons.layers[0].is_some() { h.weapons.layer_fade = 1; } else if h.state == 0 { h.weapons.pending_idle = true; }
    h.f13fa = 0;
    h.f13f8 = 0;
}

/// The standing `SetAnim`s an item update asked for ([`Weapons::pending_draw`], [`Weapons::pending_idle`]).
pub(super) fn apply_pending(h: &mut Hero, c: &mut Ctx) {
    if std::mem::take(&mut h.weapons.pending_idle) && h.state == 0 && c.anim.view().seq_b as i32 == h.weapons.layer_seq {
        let seq = h.idle_seq();
        h.set_anim(c.anim, c.rng, Pf::b(0xc000_0000), seq, 0);
    }
    if std::mem::take(&mut h.weapons.pending_draw) { draw_weapon(h, c); }
    if let Some((blend, seq, frame)) = h.weapons.pending_anim.take() { h.set_anim(c.anim, c.rng, Pf::from_i32(blend), seq, frame); }
}

/// `0x242858`: a weapon that keeps the arm raised (0x1413fa, item def +0x30) is out on foot (0x141618 clear):
/// the item in hand is the one drawn → `SetState(0, 0)` and, when it took, the weapon's standing sequence
/// (`def +0x24`) over 11 ticks (an arm layer out fades), true: the caller (SetState 3, the stop's physics, the walk's
/// slow stop in its transitions) gives up its own state; another item in hand → the weapon put away, false.
pub(super) fn gun_stance(h: &mut Hero, c: &mut Ctx) -> bool {
    if h.f13f8 == 0 || h.f13fa == 0 || h.arm_at_wall { return false; }
    let id = h.held_item();
    if id != h.weapons.arm_item {
        put_away(h);
        return false;
    }
    let s = h.weapons_def(id).anims[0];
    if !h.set_state(c, 0, false) { return false; }
    if s >= 0 { h.set_anim(c.anim, c.rng, Pf::from_i32(ticks(11)), s as u8, 0); }
    if h.weapons.layers[0].is_some() { h.weapons.layer_fade = 1; }
    h.weapons.layer_seq = s;
    true
}

/// `0x22eca0(prev, new)` (every SetState, before its switch): leaving the idle state (0) for another with a weapon
/// that keeps the arm raised out, the arm layers over the weapon's moving sequence (`def +0x28`; none: −1) are made
/// (weight 0, from Ratchet's key over 15 ticks, one step at once), layer 1 only with 0x1413fb = 2.
pub(super) fn arm_on_state_change(h: &mut Hero, c: &mut Ctx, prev: i32, new: i32) {
    if h.mode != 0 || h.f13f8 == 0 || h.f13fa == 0 || prev != 0 || new == 0 { return; }
    let seq = h.weapons_def(h.held_item()).anims[1];
    if seq == -1 { return; }
    for (i, &list) in ARM_LISTS.iter().enumerate() {
        if i == 1 && h.items.f13fb != 2 { break; }
        if h.weapons.layers[i].is_some() { continue; }
        let mut l = new_node(h, list);
        h.weapons.layer_fade = 0;
        l.weight = 0.0;
        l.start(&*c.anim, seq.max(0) as u8, 0, ticks(15), true);
        l.speed = 1.0;
        h.weapons.layers[i] = Some(l);
    }
}

/// The idle transitions' weapon rules (0x242930 state 0): whether a sequence's wrap keeps the weapon's stance (the
/// weapon is out with the arm raised and Ratchet is in its sequence or an arm layer is out).
pub(super) fn stance_kept(h: &Hero, seq_b: u8) -> bool {
    h.f13f8 != 0 && h.f13fa != 0 && (seq_b as i32 == h.weapons.layer_seq || h.weapons.layers[0].is_some())
}

/// The idle transitions' weapon stance (0x242930 state 0, after Clank's fidget): with the arm raised, out of a blend
/// (0x13fdec clear), not raised at a wall (0x141618), the item drawn in hand → its standing sequence (`def +0x24`) over 11 ticks when Ratchet is not
/// in it (an arm layer out fades); another item → put away.
pub(super) fn idle_stance(h: &mut Hero, c: &mut Ctx) {
    if h.f13f8 == 0 || c.anim.view().blending() || h.f13fa == 0 || h.arm_at_wall { return; }
    let id = h.held_item();
    if id != h.weapons.arm_item {
        put_away(h);
        return;
    }
    let s = h.weapons_def(id).anims[0];
    if c.anim.view().seq_b as i32 != s {
        if h.weapons.layers[0].is_some() { h.weapons.layer_fade = 1; }
        h.weapons.layer_seq = s;
        if s >= 0 { h.set_anim(c.anim, c.rng, Pf::from_i32(ticks(11)), s as u8, 0); }
    }
}

/// `0x22f068`'s upkeep of the arm (from the hero's item update): out of ammo with 0x1413fa set → put away; each
/// arm layer re-blends to the item's layer sequence when it changed, advances, and fades in (0.2 a tick); its
/// sequence's wrap (with 0x1413fa clear) takes the arm down and starts the fade (0.07 a tick, 0.25 after a
/// holster); a layer faded to 0 is freed.
pub fn arm_upkeep(h: &mut Hero, anim: &dyn AnimCtl) {
    if h.f13f8 != 0 && h.f13fa != 0 && h.weapons.has_ammo(h.items.slot.id) == 0 { put_away(h); }
    for i in 0..2 {
        let Some(mut l) = h.weapons.layers[i] else { continue };
        let id = h.items.slot.id;
        if id == h.weapons.arm_item {
            let def = h.weapons_def(id);
            let seq = if h.group == 0xc { def.anims[2] } else { def.anims[1] };
            if seq != -1 && l.seq_b as i32 != seq {
                h.weapons.layer_seq = seq;
                l.start(anim, seq.max(0) as u8, 0, ticks(8), true);
            }
        }
        l.advance(anim);
        if h.weapons.layer_fade == 0 {
            approach_f(&mut l.weight, 1.0, 0.2);
            if l.flags & 2 != 0 && h.f13fa == 0 {
                h.weapons.layer_fade = 1;
                l.speed = 0.0;
                h.f13f8 = 0;
                h.weapons.layers[i] = Some(l);
                return;
            }
        } else {
            let step = if h.weapons.layer_fade == 2 { 0.25 } else { 0.07 };
            approach_f(&mut l.weight, 0.0, step);
            if l.weight == 0.0 {
                h.weapons.layers[i] = None;
                return;
            }
        }
        h.weapons.layers[i] = Some(l);
    }
}

/// `0x270728(target, step, &x)` on native floats.
fn approach_f(x: &mut f32, target: f32, step: f32) { *x += (target - *x).clamp(-step, step); }

/// The holster check `0x2405f8`: stopped (idle, walk, stop) within 6 ticks of the draw with the stick below 0.7 and
/// no 0x1413fb: put away, `0x140064 = 2`, the throw state 0x23.
pub(super) fn holster_check(h: &mut Hero, c: &mut Ctx) -> bool {
    if !((h.state - 2) as u32 <= 1 || h.state == 0) || h.f13f8 == 0 { return false; }
    if !(h.f50c < ticks(6) && h.stick_mag < Pf::b(0x3f33_3333)) { return false; }
    if h.items.f13fb != 0 { return false; }
    put_away(h);
    h.weapons.layer_fade = 2;
    h.set_state(c, THROW, true);
    true
}

impl Hero {
    fn weapons_def(&self, id: i32) -> super::items::WeaponDef { self.weapons.defs.get(id.max(0) as usize).copied().unwrap_or_default() }
}

// ------------------------------------------------------------------------------------------------
// State 0x23

/// SetState 0x23 (after the group-6 part of `super::melee`'s entry).
pub(super) fn throw_entry(h: &mut Hero, c: &mut Ctx, play: bool) {
    if play { h.set_anim(c.anim, c.rng, Pf::from_i32(ticks(9)), 0x2c, 7); }
    // Item 10 with a target (0x13fda0): the aim at it (0x13fda8 = 1, 0x13fdac = the yaw from Ratchet to it).
    if h.items.slot.id == BOMB_GLOVE && h.weapons.aim.is_some() {
        let t = h.weapons.aim_pos;
        h.melee.aimed = 1;
        h.melee.aim_yaw = crate::pad::fast_arctan(Pf::f(t[0]) - h.pos[0], Pf::f(t[1]) - h.pos[1]);
    }
}

/// The glove's target position (0x13fda0 +0x10) as the moby loop left it: SetState reads the moby when it runs.
pub fn refresh_aim(h: &mut Hero, table: &MobyTable) {
    if let Some(m) = h.weapons.aim.and_then(|id| table.mobys.get(id)) { h.weapons.aim_pos = [m.position[0], m.position[1], m.position[2]]; }
}

/// The glove's launch point (`0x2d8330` / `0x2c2be0`): 0.859 from Ratchet at his facing − 20.8°, 0.491 up. `fwd` is
/// his facing row (0x13f9c0).
pub fn launch_point(pos: [f32; 3], fwd: [f32; 3]) -> [f32; 3] {
    let a = fwd[1].atan2(fwd[0]) + f32::from_bits(0xbeb9_53df);
    [pos[0] + a.cos() * 0.859_039, pos[1] + a.sin() * 0.859_039, pos[2] + 0.490_82]
}

/// 0x23's physics (`0x2370b8` case 0x23).
pub(super) fn physics(h: &mut Hero, env: &Env) -> bool {
    h.target_speed = Pf::ZERO;
    if h.melee.aimed == 0 {
        h.melee_aim_pub(env);
    } else {
        h.target_yaw = h.melee.aim_yaw;
        h.turn_to(SCALE64 * Pf::b(0x3d23_d70a), SCALE64 * Pf::b(0x3e4c_cccd), DT_PF * Pf::b(0x4170_2845));
    }
    h.speed_step(DT2 * Pf::b(0x41f0_0000), DT2 * Pf::b(0x420c_0000));
    h.set_planar_vel(Pf::b(0x47c3_4f80));
    if h.air_ticks != 0 {
        h.vel[2] = h.eff_v[2] - DT2 * Pf::b(0x41c8_0000);
        h.climb_check(env);
    } else {
        h.vel[2] = h.vel[2] - DT2 * Pf::b(0x4258_0000);
    }
    true
}

const DT_PF: Pf = super::physics::DT;

/// 0x23's transitions (`0x242930` case 0x23).
pub(super) fn transitions(h: &mut Hero, c: &mut Ctx) {
    let frame = c.anim.view().frame;
    if 19.0 < frame && c.env.pad.pressed_within(button::CROSS, ticks(0xb)).is_some() {
        if c.env.pad.held & button::CROUCH != 0 {
            h.crouch_jump(c);
        } else {
            h.set_state(c, 7, true);
        }
        return;
    }
    if 21.0 <= frame { h.set_state(c, 0, false); }
}

// ------------------------------------------------------------------------------------------------
// The throw's launch point and velocity (shared by the gloves' aims, super::gloves)

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn len3f(a: [f32; 3]) -> f32 { (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt() }
fn len2f(a: [f32; 3]) -> f32 { (a[0] * a[0] + a[1] * a[1]).sqrt() }
fn setlen(a: [f32; 3], l: f32) -> [f32; 3] {
    let n = len3f(a);
    if n == 0.0 { [0.0; 3] } else { [a[0] * l / n, a[1] * l / n, a[2] * l / n] }
}
fn diff_rot(a: f32, b: f32) -> f32 {
    let mut d = (a - b) % std::f32::consts::TAU;
    if d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
    if d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
    d.abs()
}

/// `0x2d80f0(g, ε, k, from, to)`: the bomb's velocity (u/tick) to land at `to` (see the module doc). The Glove of
/// Doom's `0x2dcf88` and the Decoy Glove's `0x2ece98` are the same code with gravity 9 and the reach `k` = 2.5 / 3.5.
pub fn launch_velocity(g: f32, k: f32, hero_yaw: f32, from: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    let mut t = to;
    if k < len3f(sub3(to, from)) { t = add3(from, setlen(sub3(to, from), k - 0.01)); }
    let mut yaw = (t[1] - from[1]).atan2(t[0] - from[0]);
    if f32::from_bits(0x3f5f_66f3) < diff_rot(hero_yaw, yaw) || len2f(sub3(from, t)) < 1.5 {
        yaw = hero_yaw;
        t = [from[0] + hero_yaw.cos() * 1.5, from[1] + hero_yaw.sin() * 1.5, from[2]];
    }
    let h = DT * 8.5;
    // 0x26faf0(h, −g, from, t): the vertical speed of the arc.
    let n = len2f(sub3(t, from)) / h;
    let mut vz = if n != 0.0 { -((from[2] - t[2]) + (-g) * n * n * 0.5) / n } else { 0.0 };
    let cap = g * (k / h) * 0.5;
    if cap < vz { vz = cap; }
    [yaw.cos() * h, yaw.sin() * h, vz]
}

/// Right after the slot loop (`super::gadgets::after_items`): the SetState the glove's update made, the draw / put-away
/// animations an item update asked for.
pub(super) fn after_items(h: &mut Hero, c: &mut Ctx) {
    if let Some(s) = h.weapons.deferred.take() {
        if h.state == 1 && s == 0x1e { h.set_state(c, 0x1e, true); }
        // The Visibomb's launch `0x2cb540`: `SetState(0x1d, 1)` (super::visibomb).
        if s == super::scripted::MISSILE { h.set_state(c, s, true); }
        // The wrench's rebound off a flag-2 target `SetState(0x21, 1)` (super::melee).
        if s == 0x21 { h.set_state(c, s, true); }
    }
    apply_pending(h, c);
}

#[cfg(test)]
mod tests {
    use super::super::items::{self, HandItem};
    use super::super::testkit::*;
    use super::*;
    use crate::pad::PadInput;
    use rc_formats::moby_anim::AnimState;

    /// `0x2d80f0` aimed 8.5 ahead at the same height: 8.5 u/s level, the arc's vertical speed ≈ 5.5 u/s (under the
    /// cap g·(k/h)/2 = 5.5 u/s); a target 90° off the facing is replaced by the point 1.5 ahead.
    #[test]
    fn launch_velocity_arcs() {
        let g = DT * DT * 11.0;
        let v = launch_velocity(g, 8.5, 0.0, [0.0; 3], [8.5, 0.0, 0.0]);
        assert!((v[0] - DT * 8.5).abs() < 1e-7 && v[1].abs() < 1e-7, "{v:?}");
        assert!((v[2] / DT - 5.49).abs() < 0.02, "vz {}", v[2] / DT);
        let v = launch_velocity(g, 8.5, 0.0, [0.0; 3], [0.0, 5.0, 0.0]);
        assert!((v[0] - DT * 8.5).abs() < 1e-7 && v[1].abs() < 1e-6, "{v:?}");
        // 1.5 ahead: n = 1.5 / (8.5·dt) ticks.
        let n = 1.5 / (8.5 * DT);
        assert!((v[2] - g * n * 0.5).abs() < 1e-6, "{v:?}");
    }

    /// `0x22e3a8`: one hit point left brings the face layer (sequence 0x7b) in by 0.05 a tick up to 0.8; healed, it
    /// fades out by 0.05 a tick and is freed; under water without the O2 Mask the weight follows the air left.
    #[test]
    fn face_layer() {
        let (mut r, _) = runner(3);
        let table = MobyTable::default();
        r.hero.health = 1;
        face_update(&mut r.hero, &r.anim, &table);
        let l = r.hero.weapons.layers[FACE].expect("made");
        assert_eq!((l.list, l.seq_b, l.weight), (FACE_LIST, 0x7b, 0.05));
        for _ in 0..30 { face_update(&mut r.hero, &r.anim, &table); }
        assert!((r.hero.weapons.layers[FACE].unwrap().weight - 0.8).abs() < 1e-5);
        r.hero.health = 4;
        for _ in 0..15 { face_update(&mut r.hero, &r.anim, &table); }
        assert!(r.hero.weapons.layers[FACE].is_some(), "still fading");
        for _ in 0..2 { face_update(&mut r.hero, &r.anim, &table); }
        assert!(r.hero.weapons.layers[FACE].is_none(), "freed at 0");
        (r.hero.group, r.hero.swim.oxygen) = (0x11, 10000);
        for _ in 0..30 { face_update(&mut r.hero, &r.anim, &table); }
        let l = r.hero.weapons.layers[FACE].unwrap();
        assert_eq!(l.seq_b, 0x78);
        assert!((l.weight - 0.25).abs() < 1e-5, "full air: 0.25");
    }

    #[test]
    fn ammo() {
        let mut w = Weapons::default();
        assert_eq!(w.has_ammo(10), 1, "no ammo counter: always 1");
        w.uses_ammo[10] = true;
        w.ammo[10] = 2;
        assert!(w.use_ammo(10, 1) && w.use_ammo(10, 1));
        assert!(!w.use_ammo(10, 1));
        assert_eq!((w.ammo[10], w.used[10], w.has_ammo(10)), (0, 2, 0));
    }

    fn glove() -> HandItem {
        let anim = AnimState { seq_a: 0, frame_a: 0, seq_b: 0, frame_b: 0, t: 0.0, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false };
        HandItem { o_class: 0xc0, mstate: 2, anim, snapshot: None, scale: 1.0, position: [0.0; 3], rows: [[0; 4]; 3], hit_timer: 0, flight: Default::default() }
    }

    fn runner(ammo: i32) -> (Runner, rc_formats::collision::Collision) {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.hero.items.slot = items::HandSlot { item: Some(glove()), fire_mask: button::CIRCLE, state: 2, id: 10, ticks_ready: 30, ..Default::default() };
        r.hero.weapons.uses_ammo[10] = true;
        r.hero.weapons.ammo[10] = ammo;
        r.run(&coll, PadInput::neutral(), 3);
        (r, coll)
    }

    /// ○ standing: the throw state 0x23 (anim 0x2c from frame 7), idle from key time 21; without ammo the glove's
    /// empty click and no state.
    #[test]
    fn circle_throws() {
        let (mut r, coll) = runner(3);
        r.tick(&coll, PadInput::neutral().press(button::CIRCLE));
        assert_eq!(r.hero.state, THROW);
        assert_eq!(r.hero.group, 6);
        assert_eq!(r.anim.calls.last().map(|c| (c.1, c.2)), Some((0x2c, 7)));
        let mut idle = None;
        for t in 0..60 {
            r.tick(&coll, PadInput::neutral());
            if r.hero.state == 0 { idle = Some(t); break; }
        }
        assert!(idle.is_some(), "never back to idle");
        let (mut r, coll) = runner(0);
        r.tick(&coll, PadInput::neutral().press(button::CIRCLE));
        assert_eq!(r.hero.state, 0);
        assert_eq!(r.hero.fx.item_sounds, vec![EMPTY_SOUND]);
    }

    /// `0x22e660`: the holding layers per 0x1413fb (2: lists 12 and 13, 1: list 12, 0: none), made at weight 0 and
    /// faded in by 0.1 a tick on Ratchet's key; out while the weapon arm's layer plays; into the holding class's
    /// sequence 0 (speed 1, over `ticks(15)`) in a special pose; faded out and freed for good with the hand hidden.
    #[test]
    fn holding_layers() {
        let mut r = Runner::new([0.0; 3], 0.0);
        r.anim.set_anim(Pf::ONE, 3, 0);
        r.anim.advance(Pf::ONE);
        for (b18, n) in [(2u8, 2), (1, 1), (0, 0)] {
            r.hero.weapons.layers = [None; 5];
            r.hero.items.f13fb = b18;
            hold_update(&mut r.hero, &r.anim);
            let made: Vec<_> = r.hero.weapons.layers[HOLD..FACE].iter().flatten().collect();
            assert_eq!(made.len(), n, "0x1413fb = {b18}");
            assert!(made.iter().all(|l| (l.weight - 0.1).abs() < 1e-6 && l.seq_b == 3 && l.alt as usize >= 1), "{made:?}");
        }
        r.hero.weapons.layers = [None; 5];
        r.hero.items.f13fb = 2;
        for _ in 0..12 { hold_update(&mut r.hero, &r.anim); }
        assert!(r.hero.weapons.layers[HOLD..FACE].iter().all(|l| l.is_some_and(|l| l.weight == 1.0 && l.list == ARM_LISTS[l.alt as usize - 1])));
        // The arm layer out and playing: the holding layers give way.
        r.hero.weapons.layers[0] = Some(AnimLayer::new(12));
        r.hero.weapons.layer_fade = 0;
        for _ in 0..12 { hold_update(&mut r.hero, &r.anim); }
        assert!(r.hero.weapons.layers[HOLD..FACE].iter().all(|l| l.is_some_and(|l| l.weight == 0.0)), "under the arm: weight 0");
        r.hero.weapons.layer_fade = 1;
        for _ in 0..12 { hold_update(&mut r.hero, &r.anim); }
        assert!(r.hero.weapons.layers[HOLD].is_some_and(|l| l.weight == 1.0), "the arm fading: back to 1");
        r.hero.weapons.layers[0] = None;
        // A special pose (the swing 0x31): the holding class's sequence 0, played at speed 1.
        r.anim.set_anim(Pf::ONE, 0x31, 0);
        r.anim.advance(Pf::ONE);
        hold_update(&mut r.hero, &r.anim);
        let l = r.hero.weapons.layers[HOLD].unwrap();
        assert!(l.special && l.seq_b == 0 && l.speed == 1.0 && (l.rate - 1.0 / 15.0).abs() < 1e-6, "{l:?}");
        // The hand hidden (0x1413ff): faded out and freed, not remade while it stays hidden.
        r.hero.f13ff = 1;
        for _ in 0..12 { hold_update(&mut r.hero, &r.anim); }
        assert!(r.hero.weapons.layers[HOLD..FACE].iter().all(Option::is_none));
    }

    /// ○ while running: the weapon arm (0x1413f8), not the state; its timer 0x13f50c counts.
    #[test]
    fn circle_while_running_draws_the_arm() {
        let (mut r, coll) = runner(3);
        r.run(&coll, PadInput::neutral().stick(0.0, -1.0), 40);
        assert_eq!(r.hero.state, 2);
        r.tick(&coll, PadInput::neutral().stick(0.0, -1.0).press(button::CIRCLE));
        assert_eq!(r.hero.state, 2);
        assert_eq!(r.hero.f13f8, 1);
        r.run(&coll, PadInput::neutral().stick(0.0, -1.0), 10);
        assert_eq!(r.hero.f50c, 10);
    }
}
