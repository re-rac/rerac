//! Item slots 1 (feet: the Magneboots 28, the Grindboots 29) and 2 (head: the Sonic Summoner 5, the O2 Mask 6,
//! the Pilot's Helmet 7) — docs/plan/gadgets.md §2, §4. The same slot bookkeeping as the back
//! ([`super::idle::ItemSlot`]): creation in `HeroItemsCreate` 0x22f3c0, the slot loop 0x231088, the per-slot
//! rules of `UpdateWrenchSelected` 0x2307e0 and the restore requests `SetState` 0x23cf98 raises. Addresses are
//! level01.elf; native `f32`.
//!
//! * **Creation** (0x22f3c0): the feet slot, empty, takes the saved feet item `0x141664` when it is not 0 (its
//!   target is never read: every feet request is saved); the head slot, empty, takes the target `0x14142c`, else
//!   the saved head item `0x141668`, when either is set. The feet slot has two mobys (item definition `+0x10` and
//!   `+0x14`: left and right boot), the head slot one.
//! * **Automatic items** (0x2307e0):
//!   - feet, in the water (`FUN_0022dea8`) or state 0x12 with an item on: that item is kept to restore and the
//!     request is 0x26 (take them off, saved item cleared);
//!   - feet, grinding (group 0xf) with the Grindboots owned (`0x13d4dd`) and the saved feet item not them (or no
//!     feet moby): request 29 (saved), the current item (or 0x26) kept to restore;
//!   - feet, on a magnetic floor (0x140637 = 1, the surface reaction's surface 2) with the Magneboots owned
//!     (`0x13d4dc`) and the saved feet item not them (or no feet moby): request 28, likewise;
//!   - head, under water (group 0x11, states 0x76 / 0x6a / 0x82, or 0x14161b) with the O2 Mask owned (`0x13d4c6`),
//!     unless it is already on, saved or targeted: request 6 **not saved**, the current head item (or 0x26) kept
//!     to restore; out of it, with the O2 Mask on and a restore item: restore request 0x141464.
//! * **Restores** (`SetState` tail): feet 0x141460 when off the magnetic floor (0x13f658 = 0) with the Magneboots
//!   on, out of the grind group with the Grindboots on, or out of the water with no feet item — each only with a
//!   restore item.
//! * **Slot loop** (0x231088): ready (2): +1 tick, the rules; being put away (3): the feet mobys are deleted at
//!   once (slot 1), the head moby when its put-away animation wraps; the empty slot runs the rules too.
//! * **Swap** (0x2307e0, `bVar3`): state 3, the effects every slot's swap has
//!   ([`Hero::slot_swap_effects`]: look cleared, fidget re-armed with one `rand_range`, a raised weapon put away),
//!   the slot mobys blend to their put-away sequence 2 (the second feet moby too).
//!
//! The feet mobys have no animation of their own in the port (deleted on the swap tick; in the game their pose is
//! Ratchet's feet, `HeroItemPoseFromRatchet` 0x22a9c8, while ready): only the head item's put-away plays
//! ([`Worn::head_anim`], with the classes given by [`Hero::set_head_classes`]; without them the put-away ends on
//! the next tick).

use super::idle::ItemSlot;
use super::Hero;
use crate::inventory::{item, NOTHING};
use crate::rng::Rng;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, MobyFrame};
use std::sync::Arc;

/// The head items' classes: `(item id, o_class, class)` (item definition `+0x10`).
pub type HeadClasses = Vec<(i32, i16, MobyAnimClass)>;

/// The head item moby's own animation (slot 2's moby `0x140480`: only its put-away plays, module docs).
#[derive(Clone, Debug)]
pub struct HeadMoby {
    pub o_class: i16,
    pub anim: AnimState,
    pub snapshot: Option<MobyFrame>,
}

/// The worn items' state besides the slot bookkeeping.
#[derive(Clone, Debug, Default)]
pub struct Worn {
    pub head: Option<HeadMoby>,
    pub head_classes: Option<Arc<HeadClasses>>,
    /// 0x14161b: no air here (written by the levels' Clank-section classes: Orxon's 22, Blarg's 1061): the O2 Mask goes
    /// on as under water.
    pub airless: u8,
}

impl Hero {
    /// The head items' classes (their put-away animation); see [`Worn`].
    pub fn set_head_classes(&mut self, classes: HeadClasses) { self.worn.head_classes = Some(Arc::new(classes)); }

    /// `GetClankModule(1)`: the feet item while slot 1 is ready, else −1.
    pub fn feet_module(&self) -> i32 { if self.feet_slot.state == 2 { self.feet_slot.id } else { -1 } }
    /// `GetClankModule(2)`: the head item while slot 2 is ready, else −1.
    pub fn head_module(&self) -> i32 { if self.head_slot.state == 2 { self.head_slot.id } else { -1 } }
    /// The feet moby is the Magneboots' (the surface reaction's test of 0x140430's class 0xad, and the footsteps').
    pub fn magneboots_on(&self) -> bool { self.feet_slot.state != 0 && self.feet_slot.id == item::MAGNEBOOTS }

    /// Slots 1 and 2 of `HeroItemsCreate` / `HeroItemsAttach` / the slot loop, in the game's slot order (the hand
    /// slot 0 before them is `super::items`', the back slot 3 after them [`Hero::back_items_update`]).
    pub(super) fn worn_items_update(&mut self, rng: &mut Rng) {
        // HeroItemsCreate.
        let f = &mut self.feet_slot;
        if f.state == 0 && f.saved != 0 {
            (f.id, f.state, f.ticks_ready) = (f.saved, 2, 0);
        }
        let h = &mut self.head_slot;
        if h.state == 0 && (h.saved != 0 || h.target != 0) {
            let id = if h.target != 0 { h.target } else { h.saved };
            (h.id, h.state, h.ticks_ready) = (id, 2, 0);
            let c = self.worn.head_classes.clone();
            self.worn.head = c.as_ref().and_then(|c| c.iter().find(|e| e.0 == id)).map(|e| HeadMoby { o_class: e.1, anim: AnimState::spawn(&e.2), snapshot: None });
        }
        // HeroItemsAttach: the head moby advances its own animation only while not ready (its put-away).
        if self.head_slot.state == 3 {
            if let (Some(m), Some(c)) = (self.worn.head.as_mut(), self.worn.head_classes.clone()) {
                if let Some(e) = c.iter().find(|e| e.1 == m.o_class) { moby_anim::advance(&mut m.anim, &e.2); }
            }
        }
        self.worn_slot_loop(rng);
    }

    /// The slot loop 0x231088's slots 1 and 2 (also run alone by the body switch's pass, `items::slot_pass`).
    pub(super) fn worn_slot_loop(&mut self, rng: &mut Rng) {
        // The slot loop: slot 1, then slot 2.
        match self.feet_slot.state {
            2 => {
                self.feet_slot.ticks_ready += 1;
                self.feet_swap(rng);
            }
            3 => {
                // Slot 1 is deleted on the put-away tick (0x2305e8).
                self.feet_slot.putaway_ticks = self.feet_slot.putaway_ticks.wrapping_add(1);
                (self.feet_slot.state, self.feet_slot.id) = (0, 0);
            }
            _ => self.feet_swap(rng),
        }
        match self.head_slot.state {
            2 => {
                self.head_slot.ticks_ready += 1;
                self.head_swap(rng);
            }
            3 => {
                self.head_slot.putaway_ticks = self.head_slot.putaway_ticks.wrapping_add(1);
                let wrapped = self.worn.head.as_ref().is_none_or(|m| m.anim.flags & 2 != 0);
                if wrapped {
                    (self.head_slot.state, self.head_slot.id) = (0, 0);
                    self.worn.head = None;
                }
            }
            _ => self.head_swap(rng),
        }
    }

    /// `UpdateWrenchSelected(1)`'s rules, then the common swap.
    fn feet_swap(&mut self, rng: &mut Rng) {
        let water = self.in_water_groups() || self.state == 0x12;
        let (grind_owned, magnet_owned) = (self.owned.has(item::GRINDBOOTS as usize), self.owned.has(item::MAGNEBOOTS as usize));
        let (group, f0637) = (self.group, self.f0637);
        let s = &mut self.feet_slot;
        let cur = if s.state != 0 { s.id } else { 0 };
        if water && cur != 0 {
            s.restore = cur;
            s.request = NOTHING;
        }
        let keep = |s: &mut ItemSlot, want: i32| {
            s.request = want;
            s.restore = if cur == 0 { NOTHING } else { cur };
        };
        if group == 0xf && grind_owned && (s.saved != item::GRINDBOOTS || cur == 0) { keep(s, item::GRINDBOOTS); }
        if f0637 != 0 && magnet_owned && (s.saved != item::MAGNEBOOTS || cur == 0) { keep(s, item::MAGNEBOOTS); }
        if s.swap_requests(None) { self.worn_swap(1, rng); }
    }

    /// `UpdateWrenchSelected(2)`'s rules, then the common swap (the O2 Mask is not saved).
    fn head_swap(&mut self, rng: &mut Rng) {
        let under = self.group == 0x11 || matches!(self.state, 0x76 | 0x6a | 0x82) || self.worn.airless != 0;
        let o2 = self.owned.has(item::O2_MASK as usize);
        let s = &mut self.head_slot;
        let cur = if s.state != 0 { s.id } else { 0 };
        let mut keep_unsaved = None;
        if under {
            if (s.saved != item::O2_MASK || cur == 0) && cur != item::O2_MASK && s.target != item::O2_MASK && o2 {
                s.request = item::O2_MASK;
                keep_unsaved = Some(item::O2_MASK);
                s.restore = if cur == 0 { NOTHING } else { cur };
            }
        } else if cur == item::O2_MASK && s.restore != 0 {
            s.restore_pending = 1;
        }
        if s.swap_requests(keep_unsaved) { self.worn_swap(2, rng); }
    }

    /// The swap of slot 1 / 2 once `swap_requests` started one: the shared effects, then the slot put away
    /// (state 3, its mobys on sequence 2; an empty slot has nothing to put away and is created next tick).
    fn worn_swap(&mut self, slot: u8, rng: &mut Rng) {
        self.slot_swap_effects(rng);
        let s = if slot == 1 { &mut self.feet_slot } else { &mut self.head_slot };
        if s.state == 0 { return; }
        s.state = 3;
        s.putaway_ticks = 0;
        if slot == 2 {
            if let (Some(m), Some(c)) = (self.worn.head.as_mut(), self.worn.head_classes.clone()) {
                if let Some(e) = c.iter().find(|e| e.1 == m.o_class) { moby_anim::set_sequence(&mut m.anim, &e.2, 2, 0, 2, &mut m.snapshot); }
            }
        }
    }

    /// The feet restore requests of the `SetState` tail (0x23cf98, module docs).
    pub(super) fn worn_on_set_state(&mut self) {
        let water = self.in_water_groups();
        let f = &mut self.feet_slot;
        let cur = if f.state != 0 { f.id } else { 0 };
        if self.f658 == 0 && cur == item::MAGNEBOOTS && f.restore != 0 { f.restore_pending = 1; }
        if self.group != 0xf && cur == item::GRINDBOOTS && f.restore != 0 { f.restore_pending = 1; }
        if !water && cur == 0 && f.restore != 0 { f.restore_pending = 1; }
    }
}

// ------------------------------------------------------------------------------------------------
// The worn items' pose (`HeroItemPoseFromRatchet` 0x22a9c8).

/// Ratchet's joints a worn item copies (the tables `HeroItemsAttach` 0x22fec0 passes, level01 data; the word before
/// each list is the attach joint the item's root sits on): the left boot 0x17aad0 (joint 99: 100..103, attach word
/// 2), the right boot 0x17aae8 (93: 94..97, attach word 3), the head items `g_head_item_joint_table` 0x15f6a8 (7:
/// 8, 10, attach word 4) and, for the Sonic Summoner's class 0x1b1, `g_head_item_joint_table_b` 0x17aa88 (7: 8, 9,
/// 10, 16, 17, 25, 26, 29, 30, 33, 34, 35, 36, 41, 44) with 6 identity joints after them.
pub const LEFT_BOOT_JOINTS: [u8; 4] = [100, 101, 102, 103];
pub const RIGHT_BOOT_JOINTS: [u8; 4] = [94, 95, 96, 97];
pub const HEAD_JOINTS: [u8; 2] = [8, 10];
pub const SONIC_SUMMONER_JOINTS: [u8; 15] = [8, 9, 10, 16, 17, 25, 26, 29, 30, 33, 34, 35, 36, 41, 44];
/// Attach words (the index of the attach matrix 0x13fe10 + 0x40·w the item's root is placed at).
pub const LEFT_BOOT_ATTACH: usize = 2;
pub const RIGHT_BOOT_ATTACH: usize = 3;
pub const HEAD_ATTACH: usize = 4;

/// `HeroItemPoseFromRatchet(table, dst, item class, extra, 0)`: a keyframe for the item whose joint 0 is the identity
/// and whose joints 1.. take Ratchet's current local pose (`MobyAnimDecodeLocalPose` 0x269938 with his pose layers
/// `layers` and joint modifiers `mods`: `rc_formats::moby_anim::snapshot_posed`) of the listed joints, then `extra`
/// identity joints; a joint's scale is written (inherited, flag 0x80) when it is not 1, its translation when it differs
/// from the item's rest. The item is evaluated with it as key A (t = 0), placed on its attach matrix without column
/// normalisation.
#[allow(clippy::too_many_arguments)]
pub fn pose_from_host(
    host: &MobyAnimClass,
    host_state: &AnimState,
    host_snap: Option<&MobyFrame>,
    layers: &[moby_anim::PoseLayer],
    mods: &[moby_anim::JointModifier],
    item: &MobyAnimClass,
    joints: &[u8],
    extra: usize,
) -> Option<MobyFrame> {
    use rc_formats::moby_anim::{MobyFrameHeader, ScaleRec, TransRec};
    let local = moby_anim::snapshot_posed(host, host_state, host_snap, layers, mods)?;
    let mut quats: Vec<[i16; 4]> = vec![[0, 0, 0, 0x7fff]];
    let (mut scales, mut trans) = (Vec::new(), Vec::new());
    for (k, &j) in joints.iter().enumerate() {
        let d = k + 1;
        quats.push(local.quat_at(j as usize));
        if let Some(r) = local.scales.iter().find(|r| r.joint == j) {
            if r.scale != [0x1000; 3] { scales.push(ScaleRec { scale: r.scale, joint: d as u8, flags: 0x80 }); }
        }
        let tv = match local.trans.iter().find(|r| r.joint == j) {
            Some(r) => r.trans,
            None => host.rest.get(j as usize).map_or([0; 3], |t| t.map(|c| c as i32 as i16)),
        };
        let rest = item.rest.get(d).map_or([0; 3], |t| t.map(|c| c as i32 as i16));
        if tv != rest { trans.push(TransRec { trans: tv, joint: d as u8, pad: 0 }); }
    }
    quats.extend(std::iter::repeat_n([0, 0, 0, 0x7fff], extra));
    let mut payload: Vec<u8> = quats.iter().flat_map(|q| q.iter().flat_map(|c| c.to_le_bytes())).collect();
    let quat_bytes = payload.len();
    for r in &scales {
        for c in r.scale { payload.extend_from_slice(&c.to_le_bytes()); }
        payload.extend_from_slice(&[r.joint, r.flags]);
    }
    let trans_offset = payload.len();
    for r in &trans {
        for c in r.trans { payload.extend_from_slice(&c.to_le_bytes()); }
        payload.extend_from_slice(&[r.joint, r.pad]);
    }
    while !payload.len().is_multiple_of(16) { payload.push(0); }
    let header = MobyFrameHeader {
        rate: 0.0,
        time: 0,
        qwc: (payload.len() / 16) as u16,
        quat_bytes: quat_bytes as u16,
        scale_count: scales.len() as u16,
        trans_offset: trans_offset as u16,
        trans_count: trans.len() as u16,
    };
    Some(MobyFrame { header, quats, scales, trans, payload })
}

/// The animation state that evaluates a [`pose_from_host`] keyframe (key A = the snapshot slot, t = 0, no advance:
/// `HeroItemsAttach` points +0x68 / +0x6c at it and zeroes +0x50 / +0x54).
pub fn posed_state() -> AnimState {
    AnimState { seq_a: moby_anim::SNAPSHOT_SEQ, frame_a: 0, seq_b: moby_anim::SNAPSHOT_SEQ, frame_b: 0, t: 0.0, speed: 0.0, rate: 0.0, flags: 0, trigger_count: 0, skip_advance: true }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hero() -> Hero { Hero::spawn([100.0, 100.0, 100.0], 0.0) }

    /// Grinding with the Grindboots owned puts them on (saved), leaving the grind group restores what was worn
    /// before (nothing): created the tick after the request, deleted on the put-away tick.
    #[test]
    fn grindboots_follow_the_grind_group() {
        let mut h = hero();
        let mut rng = Rng::new();
        h.owned.set(item::GRINDBOOTS as usize, true);
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_slot.state, h.feet_slot.saved), (0, 0), "nothing worn on foot");
        h.group = 0xf;
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_slot.saved, h.feet_slot.restore), (item::GRINDBOOTS, NOTHING));
        h.worn_items_update(&mut rng);
        assert_eq!(h.feet_module(), item::GRINDBOOTS, "created from the saved item");
        for _ in 0..5 { h.worn_items_update(&mut rng); }
        assert_eq!(h.feet_module(), item::GRINDBOOTS, "stays on while grinding");
        h.group = 0;
        h.worn_on_set_state();
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_slot.state, h.feet_slot.saved), (3, 0), "restore → nothing: put away");
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_slot.state, h.feet_module()), (0, -1));
        // Not owned: nothing happens.
        let mut h = hero();
        h.group = 0xf;
        for _ in 0..3 { h.worn_items_update(&mut rng); }
        assert_eq!(h.feet_slot.state, 0);
    }

    /// Magnetic floor → Magneboots (the surface reaction's gravity test then sees them); water takes feet items
    /// off and gives them back after.
    #[test]
    fn magneboots_and_water() {
        let mut h = hero();
        let mut rng = Rng::new();
        h.owned.set(item::MAGNEBOOTS as usize, true);
        h.owned.set(item::GRINDBOOTS as usize, true);
        h.feet_slot.saved = item::GRINDBOOTS;
        h.worn_items_update(&mut rng);
        assert_eq!(h.feet_module(), item::GRINDBOOTS, "the saved feet item is worn from the start");
        h.f0637 = 1;
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_slot.saved, h.feet_slot.restore, h.feet_slot.state), (item::MAGNEBOOTS, item::GRINDBOOTS, 3));
        h.worn_items_update(&mut rng);
        h.worn_items_update(&mut rng);
        assert!(h.magneboots_on());
        h.f0637 = 0;
        h.f658 = 0;
        h.worn_on_set_state();
        h.worn_items_update(&mut rng);
        h.worn_items_update(&mut rng);
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_module(), h.feet_slot.saved), (item::GRINDBOOTS, item::GRINDBOOTS), "back to the Grindboots");
        // Into the water: off (saved cleared, kept to restore); out: back on.
        h.group = 0x12;
        h.worn_items_update(&mut rng);
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_slot.state, h.feet_slot.saved, h.feet_slot.restore), (0, 0, item::GRINDBOOTS));
        h.group = 0;
        h.worn_on_set_state();
        for _ in 0..3 { h.worn_items_update(&mut rng); }
        assert_eq!(h.feet_module(), item::GRINDBOOTS);
    }

    /// Under water the O2 Mask goes on without being saved; out of it the head item worn before comes back.
    #[test]
    fn o2_mask_under_water() {
        let mut h = hero();
        let mut rng = Rng::new();
        h.owned.set(item::O2_MASK as usize, true);
        h.head_slot.saved = item::PILOTS_HELMET;
        h.worn_items_update(&mut rng);
        assert_eq!(h.head_module(), item::PILOTS_HELMET);
        h.group = 0x11;
        h.worn_items_update(&mut rng);
        assert_eq!((h.head_slot.state, h.head_slot.saved, h.head_slot.target), (3, item::PILOTS_HELMET, item::O2_MASK), "not saved");
        for _ in 0..3 { h.worn_items_update(&mut rng); }
        assert_eq!(h.head_module(), item::O2_MASK);
        h.group = 0;
        for _ in 0..4 { h.worn_items_update(&mut rng); }
        assert_eq!(h.head_module(), item::PILOTS_HELMET);
        // Without the mask nothing changes under water.
        let mut h = hero();
        h.group = 0x11;
        for _ in 0..3 { h.worn_items_update(&mut rng); }
        assert_eq!(h.head_slot.state, 0);
    }

    /// A menu request (the Gadgets page's close) swaps the slot like any request; 0x26 empties it.
    #[test]
    fn requests_equip_and_unequip() {
        let mut h = hero();
        let mut rng = Rng::new();
        h.feet_slot.request = item::MAGNEBOOTS;
        h.worn_items_update(&mut rng);
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_module(), h.feet_slot.saved), (item::MAGNEBOOTS, item::MAGNEBOOTS));
        h.feet_slot.request = NOTHING;
        h.worn_items_update(&mut rng);
        h.worn_items_update(&mut rng);
        assert_eq!((h.feet_slot.state, h.feet_slot.saved), (0, 0));
        h.head_slot.request = item::SONIC_SUMMONER;
        h.worn_items_update(&mut rng);
        h.worn_items_update(&mut rng);
        assert_eq!((h.head_module(), h.head_slot.saved), (item::SONIC_SUMMONER, item::SONIC_SUMMONER));
    }
}
