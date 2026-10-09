//! The game's "use" system: the context prompt, the NPC talk tables and the hand-off to a mode, a menu or a scene.
//! Spec: `docs/plan/interaction.md`; addresses are level01.elf (every function here is engine code present in all
//! 19 overlays).
//!
//! **Two shared layers**, used by every "press △ to …" in the game:
//!
//! 1. **The context prompt** (HUD slot 12): one owner-keyed lease. A class that wants the prompt calls
//!    `try_set_help_message(owner, msg)` 0x278f58 every tick it qualifies ([`Prompt::try_set`]); the lease lasts
//!    two ticks (`0x15f590 = 2`) and is counted down before the moby loop by `PromptTick` 0x278eb8 (Lombyte
//!    `RaceTimerShow`, [`Prompt::tick`]), which also keeps HUD slot 12 up while an owner holds it. The class then
//!    acts on △ itself: `if (pressed & △) && acquired { … }`. `force_help_message` 0x279000 takes the lease from
//!    anyone ([`Prompt::force`]); `PromptRelease` 0x279070 (Lombyte `OpenShipMenu`) drops it ([`Prompt::release`]).
//!    Owner ids are small constants per kind of user ([`owner`]), not moby pointers. The text goes into the
//!    0x50-byte buffer 0x17e9b0 (`PromptSetText` 0x24cdb0) that the slot-12 draw 0x24c898 (Lombyte
//!    `HudRaceTimerDraw`) prints: `crate::hud`'s `Element::Prompt`.
//! 2. **The NPC talk system** (`NpcTalkUpdate` 0x27b028, `NpcTalkRegister` 0x27b480, `NpcTalkRefresh` 0x27b550):
//!    a data-driven dialogue graph per NPC. The level's instance field +0x74 (the loader's `MobyUnknown74Hook`
//!    0x25e7b0) gives an NPC its slot k; the node table is `0x1b1af8[base(level) + k]` (base = `0x1c4938[level]`).
//!    Nodes (0x1c bytes, [`TalkNode`]) carry the prompt text, a scene or movie to play, a condition (bolts ≥ price,
//!    item owned, …) with true / false successors, and flags (1 auto: no △ needed, 4 chain the next scene, 8
//!    purchase: bolts −= price). Proximity: distance ≤ 2·r (r = talk +0x0c), the NPC facing the hero within r
//!    (radians), the hero facing the NPC within 1.57 ([`talk_rule`]). The prompt text goes to the same slot 12
//!    through its own HUD handle 0x160130. △ → the node's scene (`DialogStreamStart` 0x2ac330) or movie
//!    (`0x2acf50`); when it ends, `0x2ac608` / `MovieExitToGameplay` 0x2ad2b8 call `NpcTalkRefresh(npc, talk, 1)`:
//!    the dialogue advances. Salesmen (Infobot, Heli-Pack, R.Y.N.O., Grindboots, gold weapons …), race starters
//!    (Rilgar's "Enter race") and plain talkers ("△ Talk") are all nodes of this system.
//!
//! What is **per class** is only the proximity rule of the prompt users (vendor 11: XY ≤ 4, |Δz| ≤ 2, facing
//! ≤ π/2; ship: 4 of the hatch; teleporter: standing on it; …) and what the class does on △ (vendor → mode 5,
//! ship → mode 6, talkers → their scene). The classes here call [`vendor_rule`] / [`talk_rule`] and hand off
//! through [`Handoff`], which the engine consumes after the tick.
//!
//! Native `f32` throughout: `FastArcTan` / `FastDiffRots` (octant-table approximations) become `atan2` and an
//! exact angle difference; at the 90° / r thresholds the difference is far below a degree.

use crate::game_state::GameState;
use crate::hero::Hero;
use crate::menus::Overlay;
use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pvar as p, World};
use rc_formats::strings::{self, Message};
use std::collections::HashMap;
use std::sync::Arc;

/// Owner ids of the prompt lease (`try_set_help_message`'s first argument), read from every call site of the 19
/// overlays (docs/plan/interaction.md §2).
pub mod owner {
    /// Gadgetron vendor 11; also Orxon's Nanotech seller 1326 and the O2-mask warning 22.
    pub const VENDOR: i32 = 1;
    /// The ship (`ShipUpdate` 0x2a1c40, "△ Enter ship" 21476).
    pub const SHIP: i32 = 2;
    /// Teleporter pads 1135; Nebula G34's big red button 1118.
    pub const TELEPORTER: i32 = 4;
    /// `Help_Update` 0x225bd0 (forced, text 0: clears the prompt while a help box opens).
    pub const HELP: i32 = 5;
    /// Forced by the hand gadgets 168 / 172 / 615 (text 0: no prompt while they run).
    pub const GADGET: i32 = 6;
    /// Vehicles: Pokitaru's jet plane 1242, Gemlik's 69.
    pub const VEHICLE: i32 = 7;
    /// Kerwan 997 / 806, Rilgar 998: the transport pads ("△ Go to …").
    pub const TRANSPORT: i32 = 8;
}

/// `PromptSetText` 0x24cdb0: the buffer is 0x50 bytes.
pub const PROMPT_TEXT_MAX: usize = 0x50;
/// The text `PromptSetText` stores instead of one that does not fit.
pub const TOO_LONG: &[u8] = b"Message too long";

/// The context prompt lease and its text (0x15f590 timer, 0x15f594 owner, 0x162288 message, 0x17e9b0 text).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prompt {
    /// 0x15f590: ticks left (2 on every set).
    pub timer: i32,
    /// 0x15f594: the owner id (0 = free).
    pub owner: i32,
    /// 0x162288: the message id (0 = no text: the slot is not shown).
    pub msg: i32,
    /// 0x17e9b0.
    pub text: Vec<u8>,
    /// Bumped by every [`Prompt::release`] that freed slot 12: the HUD drops the element at once (the engine's feed).
    pub released: u32,
}

/// `PromptSetText` 0x24cdb0 into `buf`.
fn set_text(buf: &mut Vec<u8>, text: &[u8]) {
    buf.clear();
    buf.extend_from_slice(if text.len() >= PROMPT_TEXT_MAX { TOO_LONG } else { text });
}

impl Prompt {
    /// `try_set_help_message(owner, msg)` 0x278f58: 2 when `owner` already holds the lease (text and timer
    /// renewed), 1 when it was free and is now `owner`'s, 0 when someone else holds it. `text` = `msg_string(msg)`.
    pub fn try_set(&mut self, owner: i32, msg: i32, text: &[u8]) -> i32 {
        if self.owner == owner {
            if msg != 0 { set_text(&mut self.text, text); }
            self.timer = 2;
            self.msg = msg;
            2
        } else if self.owner == 0 {
            if msg != 0 { set_text(&mut self.text, text); }
            self.timer = 2;
            self.owner = owner;
            self.msg = msg;
            1
        } else {
            0
        }
    }

    /// `force_help_message(owner, msg)` 0x279000: [`try_set`](Self::try_set), else take the lease (3).
    pub fn force(&mut self, owner: i32, msg: i32, text: &[u8]) -> i32 {
        let r = self.try_set(owner, msg, text);
        if r != 0 { return r; }
        if msg != 0 { set_text(&mut self.text, text); }
        self.timer = 2;
        self.msg = msg;
        self.owner = owner;
        3
    }

    /// `PromptRelease(owner)` 0x279070 (Lombyte `OpenShipMenu`): when `owner` holds the lease and slot 12 is up
    /// (`0x162284 ≠ −1`), free it and the slot at once. `slot_up` = the HUD holds the prompt element.
    pub fn release(&mut self, owner: i32, slot_up: bool) -> bool {
        if self.owner != owner || !slot_up { return false; }
        self.timer = 0;
        self.owner = 0;
        self.released = self.released.wrapping_add(1);
        true
    }

    /// The lease part of `PromptTick` 0x278eb8 (run before the moby loop when the hero is not in state 0x1d /
    /// 0x32): the timer counts down and the owner is dropped when it reaches 0.
    pub fn tick(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
            if self.timer == 0 { self.owner = 0; }
        }
    }

    /// `PromptTick`'s HUD part: slot 12 is kept up (`FUN_0024b4b0(handle, 10)`) while an owner holds a message.
    pub fn shown(&self) -> bool { self.owner != 0 && self.msg != 0 }
}

/// What a class asked of the engine this tick (the game calls these directly; the port runs them after the tick).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Handoff {
    /// `OpenVendorMenu(vendor)` 0x2ae1a0: game mode 5 with this vendor moby (None: the PDA's remote vendor).
    OpenVendor { vendor: Option<MobyId> },
    /// `DialogStreamStart(scene)` 0x2ac330 for the talker `npc` (`0x179588`): its node's scene.
    Scene { scene: i32, npc: Option<MobyId> },
    /// The PSS movie `0x2acf50(movie)` (node scene id with bit 0x4000).
    Movie { movie: i32, npc: Option<MobyId> },
    /// The ship's △ (`0x15f630 = 1` → take-off, mode 6).
    ShipMenu,
    /// `FUN_002aea70(offer)`: the gold-weapon upgrade effect of an item offer (not ported).
    GoldUpgrade { offer: MobyId, item: i32 },
}

/// Writes of the talk system into the saved game, applied by the engine after the tick
/// ([`Interact::apply_writes`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameWrite {
    /// `0x13d388[i] = 0` (a purchase with condition kind 4, e.g. Pokitaru's Raritanium trade).
    ClearFlag(usize),
    /// `0x13d388[i] = v` written by a class update ([`set_global_flag`]).
    Flag(usize, u8),
    /// `0x13e520[item] = 1` (a gold weapon bought, `ItemOfferUpdate`).
    GoldWeapon(usize),
    /// `UnlockPlanet(p)` 0x2756d0 (Novalis' Water Pump Worker: the Infobot to Aridia).
    UnlockPlanet(usize),
    /// `0x13d5bc + 16·i` (the landmark record's +0xc: "talked to" flag of talk slot i, chunk 15).
    Talked(usize, u32),
    /// `*(0x14bec0 + level·4 + index) = 1`: gold bolt `index` of `level` collected (class 1134).
    GoldBolt { level: usize, index: usize },
    /// `GiveItem(item, equip)` 0x275760 called by a class (Pokitaru's commando 114: the O2 Mask, item 6): applied by the
    /// engine with the item tables (`GameState::give_item`); the banner is the class side's ([`give_item`]).
    GiveItem { item: usize, equip: bool },
    /// `0x13d408[k] = 1`: skill point k earned (`crate::moby_update::story::award_skill_point`).
    SkillPoint(usize),
    /// `0x13d4c0[item] = v`: a class's direct item store (`story::set_owned`; not `GiveItem`).
    Owned(usize, u8),
    /// `0x13d4e8[item] = v` (`story::set_acquired`).
    Acquired(usize, u8),
    /// `0x15eda0 = n`: max health (the nanotech upgrades, `story::set_max_hp`).
    MaxHp(i32),
    /// `0x2f21c0`: buried cache `cache` (1..) of `level` dug once more: its nibble of `0x14bf10 + level·16` + 1, at most
    /// 15 (the Metal Detector, `crate::hero::metal_detector`).
    MetalDetectorDig { level: usize, cache: u8 },
}

// ---------------------------------------------------------------------------------------------------------------
// Rules

/// What the rules read of the hero block.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeroView {
    /// 0x13f3d0: the feet.
    pub pos: [f32; 3],
    /// 0x13f3e8: yaw.
    pub yaw: f32,
    /// 0x1413d4 / 0x1413dc.
    pub state: i32,
    pub group: i32,
    /// 0x1413f4: control mode (0 = Ratchet on foot).
    pub mode: u8,
    /// 0x1415f8.
    pub hp: i32,
}

impl HeroView {
    pub fn of(h: &Hero) -> HeroView {
        HeroView { pos: h.position(), yaw: h.yaw().to_f32(), state: h.state, group: h.group, mode: h.mode, hp: h.health }
    }
}

/// `FastArcTan(dx, dy)`: the angle of (dx, dy) from +x (native `atan2`).
pub fn heading(dx: f32, dy: f32) -> f32 { dy.atan2(dx) }

/// `fast_add_rotations(a, b)`: a + b wrapped to −π..π.
pub fn add_rot(a: f32, b: f32) -> f32 {
    let pi = std::f32::consts::PI;
    let s = a + b;
    if s >= pi { s - 2.0 * pi } else if s < -pi { s + 2.0 * pi } else { s }
}

/// `FastDiffRots(a, b)`: |a − b| wrapped to 0..π.
pub fn diff_rots(a: f32, b: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let d = (a - b).rem_euclid(tau);
    if d > std::f32::consts::PI { tau - d } else { d }
}

/// The vendor's prompt rule (class 11 state 2, level01 0x2bb5a0..0x2bb6a8) → (show the prompt, △ may open).
/// * both: hero group 0 or 1 or state 3; not in state 0x1d / 0x32; control mode 0; XY distance ≤ 4 (0x40800000)
///   and |Δz| ≤ 2; the hero's yaw within π/2 of the direction to the vendor;
/// * opening also needs the vendor's animation settled on its open sequence (key A +0x52 = 1: the seq-1 blend
///   done).
pub fn vendor_rule(h: &HeroView, vendor: [f32; 3], settled: bool) -> (bool, bool) {
    let mut ok = h.group == 0 || h.group == 1 || h.state == 3;
    if h.state == 0x1d || h.state == 0x32 || h.mode != 0 { ok = false; }
    let (dx, dy) = (vendor[0] - h.pos[0], vendor[1] - h.pos[1]);
    let xy = (dx * dx + dy * dy).sqrt();
    let dz = (vendor[2] - h.pos[2]).abs();
    if xy > 4.0 || dz > 2.0 { ok = false; }
    if diff_rots(h.yaw, heading(dx, dy)) > std::f32::consts::FRAC_PI_2 { ok = false; }
    (ok, ok && settled)
}

/// The vendor's approach distances (`VecDistance2`, XY): state 1 → 2 within 16 (and |Δz| ≤ 8), state 2 → 1
/// beyond 18 (or |Δz| > 10), both tested every 8th tick.
pub const VENDOR_NEAR: (f32, f32) = (16.0, 8.0);
pub const VENDOR_FAR: (f32, f32) = (18.0, 10.0);

/// `NpcTalkUpdate`'s proximity and facing rule (0x27b0b4..0x27b16c): distance (3-D) ≤ 2·r, and for a
/// non-auto node the NPC's yaw within r (radians) of the direction to the hero and the hero's within 1.57 of
/// the direction to the NPC.
pub fn talk_rule(h: &HeroView, npc: [f32; 3], npc_yaw: f32, r: f32, auto: bool) -> bool {
    let d = [h.pos[0] - npc[0], h.pos[1] - npc[1], h.pos[2] - npc[2]];
    if (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() > r + r { return false; }
    if auto { return true; }
    if diff_rots(heading(d[0], d[1]), npc_yaw) > r { return false; }
    diff_rots(heading(-d[0], -d[1]), h.yaw) <= 1.57
}

// ---------------------------------------------------------------------------------------------------------------
// Talk tables

/// One dialogue node (0x1c bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TalkNode {
    /// +0x00: prompt text id (0: none).
    pub msg: i32,
    /// +0x04: scene to play on △ (−1 none; bit 0x4000: a movie).
    pub scene: i16,
    /// +0x06: the node after the scene.
    pub next: i16,
    /// +0x08: condition kind (0 none, 1 bolts ≥ price, 2 owned, 3 owned and not 0x13d4e8, 4 flag 0x13d388,
    /// 5 spendable gold bolts ≥ arg, 6 bolts ≥ gold price and > 3 gold bolts).
    pub kind: i16,
    /// +0x0a: the condition's item / argument.
    pub item: i16,
    /// +0x0c / +0x0e: the node when the condition holds / fails.
    pub yes: i16,
    pub no: i16,
    /// +0x10: 1 auto (no △, no facing), 4 chain the next node's scene when this one ends, 8 purchase.
    pub flags: u16,
}

impl TalkNode {
    pub fn parse(b: &[u8]) -> TalkNode {
        let h = |o: usize| i16::from_le_bytes([b[o], b[o + 1]]);
        TalkNode {
            msg: i32::from_le_bytes(b[0..4].try_into().unwrap()),
            scene: h(4),
            next: h(6),
            kind: h(8),
            item: h(10),
            yes: h(12),
            no: h(14),
            flags: h(16) as u16,
        }
    }
}

/// The level's dialogue tables: `0x1b1af8[i]` (node lists) and the level ranges `0x1c4938[level]`, plus the
/// price records the conditions and the vendor read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TalkTables {
    /// Global slot → nodes (empty: no table).
    pub tables: Vec<Vec<TalkNode>>,
    /// `0x1c4938`: first global slot of each level (20 entries: levels 0..18 and the end).
    pub base: Vec<i32>,
    pub shop: ShopTable,
    /// `GiveItem`'s banner table (level11 0x1b0d40; found through its code): the message of item i (owned: i + 0x25), −1
    /// none ([`give_item`]).
    pub give_banners: Vec<i32>,
}

/// The 0x18-byte price records `0x1c4530[43]` (level01; the level range table follows them directly): items
/// 0..36 and the talk system's pseudo items 37..42 (the Infobots 37 / 39 / 42, the bouncer's bribe 38, the Premium
/// / Ultra Nanotech 40 / 41).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShopTable {
    pub records: Vec<[u8; 0x18]>,
}

/// Records in [`ShopTable`].
pub const SHOP_RECORDS: usize = 43;

impl ShopTable {
    fn u16(&self, i: usize, o: usize) -> u16 { self.records.get(i).map_or(0, |r| u16::from_le_bytes([r[o], r[o + 1]])) }
    /// +0x00: price (`g_weapon_prices`).
    pub fn price(&self, i: usize) -> i32 { self.records.get(i).map_or(0, |r| i32::from_le_bytes(r[0..4].try_into().unwrap())) }
    /// +0x04: the discounted price (flag `0x13d4e3`).
    pub fn discounted(&self, i: usize) -> i32 { self.records.get(i).map_or(0, |r| i32::from_le_bytes(r[4..8].try_into().unwrap())) }
    /// +0x08: ammo unit price at a vendor (0: no ammo), +0x0a: through the PDA.
    pub fn ammo_price(&self, i: usize) -> u16 { self.u16(i, 8) }
    pub fn pda_ammo_price(&self, i: usize) -> u16 { self.u16(i, 0xa) }
    /// +0x0e: max ammo.
    pub fn max_ammo(&self, i: usize) -> u16 { self.u16(i, 0xe) }
    /// +0x14: the gold version's bolt price (`0x1c4544`).
    pub fn gold_price(&self, i: usize) -> u16 { self.u16(i, 0x14) }
}

/// The first eight entries of the level ranges `0x1c4938` (the same bytes in every overlay).
const RANGE_PREFIX: [i32; 8] = [0, 2, 15, 19, 34, 37, 46, 50];

impl TalkTables {
    /// From the level overlay: the range table found by its bytes, the node-list pointer array by the code of
    /// `NpcTalkRegister` (`lui v0,H` · `sll a0,a2,2` · `addiu v0,v0,L` · `addu a1,a0,v0` · `lw v1,0(a1)`). A list
    /// runs to the next list's start (they are consecutive in .data) and at most 16 nodes; it stops at a node whose
    /// fields are out of range.
    pub fn load(ov: &Overlay) -> Option<TalkTables> {
        let pat: Vec<u8> = RANGE_PREFIX.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut ranges_at = None;
        let mut tables_at = None;
        for s in ov.sections().iter().filter(|s| s.kind != 8) {
            if ranges_at.is_none() {
                if let Some(i) = s.data.windows(pat.len()).position(|w| w == pat.as_slice()) { ranges_at = Some(s.dest + i as u32); }
            }
            let w: Vec<u32> = s.data.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
            for i in 0..w.len().saturating_sub(4) {
                if w[i] >> 16 == 0x3c02 && w[i + 1] == 0x0006_2080 && w[i + 2] >> 16 == 0x2442 && w[i + 3] == 0x0082_2821 && w[i + 4] == 0x8ca3_0000 {
                    let a = ((w[i] & 0xffff) << 16).wrapping_add((w[i + 2] & 0xffff) as i16 as i32 as u32);
                    tables_at.get_or_insert(a);
                }
            }
        }
        // Levels without NPC talk (Umbris, Gaspar) have no `NpcTalkRegister`: no node lists, but the records and
        // ranges are there all the same (the vendor's ammo list reads the prices, the pickups the max ammo).
        let ranges_at = ranges_at?;
        let give_banners = give_banner_table(ov);
        let base: Vec<i32> = (0..20).map(|i| ov.i32(ranges_at + 4 * i)).collect::<Option<_>>()?;
        let rec_at = ranges_at.checked_sub((SHOP_RECORDS * 0x18) as u32)?;
        let records: Vec<[u8; 0x18]> = (0..SHOP_RECORDS).map(|i| ov.bytes(rec_at + (i * 0x18) as u32, 0x18).map(|b| b.try_into().unwrap())).collect::<Option<_>>()?;
        let shop = ShopTable { records };
        // Sanity: the Blaster (15) costs 2500, its ammo 1 bolt.
        if shop.price(15) != 2500 || shop.ammo_price(15) != 1 { return None; }
        let n = *base.last()? as usize + 1;
        let ptrs: Vec<u32> = (0..n).map(|i| tables_at.and_then(|t| ov.u32(t + 4 * i as u32)).unwrap_or(0)).collect();
        let mut sorted: Vec<u32> = ptrs.iter().copied().filter(|&a| a != 0).collect();
        sorted.sort_unstable();
        sorted.dedup();
        let tables = ptrs
            .iter()
            .map(|&a| {
                if a == 0 { return Vec::new(); }
                let end = sorted.iter().copied().find(|&b| b > a).unwrap_or(a + 16 * 0x1c);
                let count = ((end - a) / 0x1c).min(16);
                let mut out = Vec::new();
                for k in 0..count {
                    let Some(b) = ov.bytes(a + k * 0x1c, 0x1c) else { break };
                    let nd = TalkNode::parse(b);
                    if !(0..=10).contains(&nd.kind) || nd.msg.unsigned_abs() > 100_000 { break; }
                    out.push(nd);
                }
                out
            })
            .collect();
        Some(TalkTables { tables, base, shop, give_banners })
    }

    /// The global slot of talk slot `k` (instance +0x74) on `level`.
    pub fn global(&self, level: u32, k: i32) -> Option<usize> {
        let lo = *self.base.get(level as usize)?;
        let hi = *self.base.get(level as usize + 1)?;
        let g = lo + k;
        (k >= 0 && g < hi).then_some(g as usize)
    }

    pub fn nodes(&self, g: usize) -> &[TalkNode] { self.tables.get(g).map_or(&[], |t| t.as_slice()) }
}

/// The entries of `GiveItem`'s banner table (`2·37`: the item's message, then the owned items' at +0x25).
pub const GIVE_BANNERS: usize = 0x4a;

/// `GiveItem` 0x275760's banner table, found by its code (`addiu v1, s0, 0x25` · `sq` · `lui a0, H` · `sq` · `addiu a0,
/// a0, L`: the table at `H << 16 + L`); empty when the overlay has no such code.
fn give_banner_table(ov: &Overlay) -> Vec<i32> {
    for s in ov.sections().iter().filter(|s| s.kind != 8) {
        let w: Vec<u32> = s.data.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
        for i in 0..w.len().saturating_sub(5) {
            if w[i] != 0x2603_0025 { continue; }
            let Some(h) = (1..=3).map(|k| w[i + k]).find(|x| x >> 16 == 0x3c04) else { continue };
            let Some(l) = (2..=5).filter_map(|k| w.get(i + k)).find(|x| *x >> 16 == 0x2484) else { continue };
            let a = ((h & 0xffff) << 16).wrapping_add((l & 0xffff) as i16 as i32 as u32);
            let t: Option<Vec<i32>> = (0..GIVE_BANNERS as u32).map(|k| ov.i32(a + 4 * k)).collect();
            if let Some(t) = t { return t; }
        }
    }
    Vec::new()
}

// ---------------------------------------------------------------------------------------------------------------
// Talk block (the NPC's pvar header)

/// Offsets of the talk block, which is the first 0x40 bytes of an NPC's pvars (`moby+0x78`).
pub mod talk {
    /// s16: the node whose scene ended / that was advanced last (−1).
    pub const LAST: usize = 0x04;
    /// u8: the current node's flag 1 (auto; 0xff: talk disabled).
    pub const AUTO: usize = 0x08;
    /// u8: registered.
    pub const REGISTERED: usize = 0x09;
    /// f32: the radius r (range 2·r, NPC facing within r).
    pub const RADIUS: usize = 0x0c;
    /// s16: current node (−1: none).
    pub const NODE: usize = 0x36;
    /// u32: tick of the last node change.
    pub const SINCE: usize = 0x38;
    /// The table (the game: a pointer; the port: global slot + 1, 0 = none).
    pub const TABLE: usize = 0x3c;
    /// Pvar bytes the block needs.
    pub const SIZE: usize = 0x40;
}

/// State of the talk system and the prompt (the game's globals the classes share).
#[derive(Clone, Debug, Default)]
pub struct Interact {
    pub prompt: Prompt,
    /// `PromptTick`'s slot-12 request this tick (an owner held a message after the countdown).
    pub prompt_hud: bool,
    /// 0x15f594 as the hero update saw it (before this tick's moby loop): the quick-select ring's open test reads it
    /// there, so a class that takes △ and releases its prompt in the same tick (Blarg's button) still blocks the ring.
    pub owner_at_hero: i32,
    /// The talk system's own slot-12 request this tick (`NpcTalkUpdate` bumps handle 0x160130 while the hero is in
    /// range and △ is not pressed).
    pub talk_shown: bool,
    /// A class's own slot-12 request this tick (Kerwan's transporters 1012 create the prompt element themselves and
    /// keep it up for 10 ticks through their handle).
    pub class_shown: bool,
    /// The level's text (`msg_string`), set by the loader.
    pub messages: Arc<Vec<Message>>,
    /// 0x15ed88: the language (price separators).
    pub lang: u32,
    pub tables: Arc<TalkTables>,
    /// `0x179638` via the loader's `MobyUnknown74Hook`: NPC moby → talk slot k (instance +0x74).
    pub talk_slots: HashMap<MobyId, i32>,
    /// `0x179590`: talking is refused until this tick (written only at level load: 0).
    pub cooldown: u64,
    /// `0x179588`: the NPC whose scene or movie runs (the refresh at its end goes to it).
    pub talker: Option<MobyId>,
    /// The scene / movie of [`talker`](Self::talker) ended: its refresh is due ([`poll_scene_end`]).
    pub scene_ended: bool,
    /// Pad bits of this tick (0x13cae4 pressed after the lock).
    pub pressed: u32,
    /// The game values the talk conditions read, synced before every tick ([`Interact::sync_game`]).
    pub game: TalkGame,
    /// Hand-offs requested this tick, in order.
    pub handoffs: Vec<Handoff>,
    /// `FUN_002783a8(d, npc)` (0x16cd26 / 0x16ccf0 / 0x16cd00): where the scene end puts Ratchet (in front of the
    /// talker, facing it); the engine applies it when the scene ends.
    pub scene_end_place: Option<([f32; 3], f32)>,
    /// The vendor class's globals (glow and hologram phases).
    pub vendor: crate::moby_update::classes::vendor::Globals,
    /// Saved-game writes, in order.
    pub writes: Vec<GameWrite>,
}

/// A class update's store into the global flags `0x13d388[i] = v`: the mirror the classes read this tick
/// ([`TalkGame::flags`]) and the saved-game write ([`GameWrite::Flag`], applied after the tick).
pub fn set_global_flag(w: &mut World, i: usize, v: u8) {
    if let Some(b) = w.svc.interact.game.flags.get_mut(i) { *b = v; }
    w.svc.interact.writes.push(GameWrite::Flag(i, v));
}

/// The saved-game values of the talk conditions.
#[derive(Clone, Debug, Default)]
pub struct TalkGame {
    /// 0x13d4e8.
    pub acquired: Vec<u8>,
    /// 0x13d388.
    pub flags: Vec<u8>,
    /// 0x13e520.
    pub gold_weapons: Vec<u8>,
    /// Gold bolts collected (non-zero bytes of 0x14bec0[20][4]).
    pub gold_bolts: i32,
    /// 0x13d5bc + 16·i: talked flags by global slot.
    pub talked: Vec<u32>,
    /// 0x13dd40: planets unlocked.
    pub planet_unlocked: Vec<u8>,
    /// 0x14bec0 + level·4 (save chunk 3003 of each level slot): the gold bolts collected, by level and index (the
    /// gold bolt class 1134 reads its level's at init and sets its byte at the pickup).
    pub gold_bolt_bits: Vec<[u8; 4]>,
    /// 0x14bf10 + level·16 (save chunk 3008): the Metal Detector's dug counts, two nibbles a byte, by level (the
    /// buried bolt caches 605 read their level's at init: `classes::buried_bolts`).
    pub metal_detector_bits: Vec<[u8; 16]>,
    /// 0x13d408: the skill points (chunk 8), read and set by the classes (`crate::moby_update::story`).
    pub skill_points: Vec<u8>,
}

impl TalkGame {
    /// `compute_clamped_count_difference` 0x2790d0: gold bolts collected (≤ 40) − 4 · gold weapons owned (≤ 10),
    /// clamped to 0..40.
    pub fn spendable_gold_bolts(&self) -> i32 {
        let owned = self.gold_weapons.iter().take(37).filter(|&&b| b != 0).count().min(10) as i32;
        (self.gold_bolts.clamp(0, 40) - 4 * owned).clamp(0, 40)
    }
}

impl Interact {
    /// The text of message `id` (`msg_string`).
    pub fn msg(&self, id: i32) -> Vec<u8> { strings::lookup(&self.messages, id).to_vec() }

    /// Called before the moby loop each tick: the pad and `PromptTick` 0x278eb8 (skipped by the game while the
    /// hero is in state 0x1d / 0x32).
    pub fn begin_tick(&mut self, pressed: u32, hero_state: i32) {
        self.pressed = pressed;
        self.talk_shown = false;
        self.class_shown = false;
        if hero_state != 0x1d && hero_state != 0x32 {
            self.prompt.tick();
            self.prompt_hud = self.prompt.shown();
        } else {
            self.prompt_hud = false;
        }
        self.owner_at_hero = self.prompt.owner;
    }

    /// `try_set_help_message(owner, msg)`.
    pub fn try_prompt(&mut self, owner: i32, msg: i32) -> i32 {
        let t = self.msg(msg);
        self.prompt.try_set(owner, msg, &t)
    }

    /// `force_help_message(owner, msg)`.
    pub fn force_prompt(&mut self, owner: i32, msg: i32) -> i32 {
        let t = self.msg(msg);
        self.prompt.force(owner, msg, &t)
    }

    /// △ pressed this tick (0x13cae4 & 0x10).
    pub fn triangle(&self) -> bool { self.pressed & crate::pad::button::TRIANGLE != 0 }

    /// The saved-game values the talk conditions read, from the game state (before the tick).
    pub fn sync_game(&mut self, gs: &GameState) {
        let g = &gs.global;
        self.game.acquired = g.acquired.to_vec();
        self.game.flags = g.flags.to_vec();
        self.game.gold_weapons = g.gold_weapons.to_vec();
        self.game.gold_bolts = gs.levels.iter().take(20).map(|l| l.gold_bolts.iter().filter(|&&b| b != 0).count() as i32).sum();
        self.game.gold_bolt_bits = gs.levels.iter().map(|l| l.gold_bolts).collect();
        self.game.metal_detector_bits = gs.levels.iter().map(|l| l.metal_detector).collect();
        self.game.talked = g.landmarks.iter().map(|l| l.flags).collect();
        self.game.planet_unlocked = g.planet_unlocked.to_vec();
        self.game.skill_points = g.skill_points.to_vec();
    }

    /// The saved-game writes of the tick into `gs` (and drains them).
    pub fn apply_writes(&mut self, gs: &mut GameState) -> Vec<GameWrite> {
        let w = std::mem::take(&mut self.writes);
        for x in &w {
            match *x {
                GameWrite::ClearFlag(i) => { if let Some(b) = gs.global.flags.get_mut(i) { *b = 0; } }
                GameWrite::Flag(i, v) => { if let Some(b) = gs.global.flags.get_mut(i) { *b = v; } }
                GameWrite::GoldWeapon(i) => { if let Some(b) = gs.global.gold_weapons.get_mut(i) { *b = 1; } }
                GameWrite::UnlockPlanet(pl) => gs.unlock_planet(pl),
                GameWrite::Talked(i, v) => { if let Some(l) = gs.global.landmarks.get_mut(i) { l.flags = v; } }
                GameWrite::GoldBolt { level, index } => {
                    if let Some(b) = gs.levels.get_mut(level).and_then(|l| l.gold_bolts.get_mut(index)) { *b = 1; }
                }
                // Needs the item tables: the engine applies it from the returned list.
                GameWrite::GiveItem { .. } => {}
                GameWrite::SkillPoint(k) => { if let Some(b) = gs.global.skill_points.get_mut(k) { *b = 1; } }
                GameWrite::Owned(i, v) => { if let Some(b) = gs.global.owned.get_mut(i) { *b = v; } }
                GameWrite::Acquired(i, v) => { if let Some(b) = gs.global.acquired.get_mut(i) { *b = v; } }
                GameWrite::MaxHp(n) => gs.global.max_hp = n,
                GameWrite::MetalDetectorDig { level, cache } => {
                    if let Some(l) = gs.levels.get_mut(level) {
                        let k = cache.wrapping_sub(1);
                        if let Some(b) = l.metal_detector.get_mut((k / 2) as usize) {
                            *b = if k & 1 != 0 { (*b & 0xf0) | ((*b & 0xf) + 1).min(15) } else { (*b & 0x0f) | (((*b >> 4) + 1).min(15) << 4) };
                        }
                    }
                }
            }
        }
        w
    }

    /// The loader's `MobyUnknown74Hook` 0x25e7b0 for every created moby whose instance has +0x74 ≠ −1.
    pub fn register_talk_mobys(&mut self, slots: impl IntoIterator<Item = (MobyId, i32)>) {
        for (id, k) in slots { if k != -1 { self.talk_slots.insert(id, k); } }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// NpcTalk* (0x27b028 / 0x27b480 / 0x27b550)

fn node_of(w: &World, id: MobyId, base: usize, n: i16) -> Option<TalkNode> {
    let g = p::i32(&w.m(id).pvars, base + talk::TABLE) - 1;
    if g < 0 || n < 0 { return None; }
    w.svc.interact.tables.nodes(g as usize).get(n as usize).copied()
}

/// `FUN_0027b3b0(npc)`: the NPC's global talk slot (−1: none).
pub fn talk_slot(w: &World, id: MobyId) -> i32 {
    let Some(&k) = w.svc.interact.talk_slots.get(&id) else { return -1 };
    w.svc.interact.tables.global(w.svc.level, k).map_or(-1, |g| g as i32)
}

/// `NpcTalkRegister(npc, talk)` 0x27b480: −1 when the NPC has no slot or its slot no table; else the block is
/// set up at node 0 (node 0's successor when the NPC was talked to before and node 0 has no condition) and the
/// slot is returned.
pub fn talk_register(w: &mut World, id: MobyId) -> i32 { talk_register_at(w, id, 0) }

/// [`talk_register`] on a talk block at pvar offset `base` (`NpcTalkRegister(npc, pvars + base)`: the commando 114
/// keeps its block at +0x20).
pub fn talk_register_at(w: &mut World, id: MobyId, base: usize) -> i32 {
    if w.m(id).pvars.len() < base + talk::SIZE { return -1; }
    let g = talk_slot(w, id);
    if g < 0 || w.svc.interact.tables.nodes(g as usize).is_empty() { return -1; }
    let talked = w.svc.interact.game.talked.get(g as usize).copied().unwrap_or(0) != 0;
    let nodes = w.svc.interact.tables.nodes(g as usize).to_vec();
    let counter = w.counter as u32;
    let pv = &mut w.mm(id).pvars;
    p::set_u8(pv, base + talk::REGISTERED, 1);
    p::set_i16(pv, base + talk::LAST, -1);
    p::set_u32(pv, base + talk::SINCE, counter);
    p::set_i16(pv, base + talk::NODE, 0);
    p::set_i32(pv, base + talk::TABLE, g + 1);
    let mut n = 0i16;
    if talked && nodes[0].kind == 0 { n = nodes[0].next; }
    p::set_i16(pv, base + talk::NODE, n);
    let auto = nodes.get(n.max(0) as usize).map_or(0, |nd| (nd.flags & 1) as u8);
    p::set_u8(pv, base + talk::AUTO, auto);
    g
}

/// The condition of node `nd` (0x27b7e0..0x27b8c8).
fn condition(w: &World, nd: &TalkNode) -> bool {
    let g = &w.svc.interact.game;
    let it = nd.item.max(0) as usize;
    let bolts = w.svc.counters.bolts;
    let byte = |v: &Vec<u8>| v.get(it).copied().unwrap_or(0) != 0;
    match nd.kind {
        1 => w.svc.interact.tables.shop.price(it) <= bolts,
        2 => w.hero.owned.has(it),
        3 => w.hero.owned.has(it) && !byte(&g.acquired),
        4 => byte(&g.flags),
        5 => nd.item as i32 <= g.spendable_gold_bolts(),
        6 => (w.svc.interact.tables.shop.gold_price(it) as i32) <= bolts && g.spendable_gold_bolts() > 3,
        _ => false,
    }
}

/// The prompt text of node `nd`: kind 6 replaces the text's `%d` by the gold price as `"%d%s%03d"` (thousands,
/// the language's separator from 0x20a180 (`,` / `.` for German and Italian), the rest).
fn node_text(w: &World, nd: &TalkNode) -> Vec<u8> {
    let t = w.svc.interact.msg(nd.msg);
    if nd.kind != 6 { return t; }
    let price = w.svc.interact.tables.shop.gold_price(nd.item.max(0) as usize) as u32;
    let sep = if matches!(w.svc.interact.lang % 6, 3 | 5) { "." } else { "," };
    let num = format!("{}{}{:03}", price / 1000, sep, price % 1000);
    match t.windows(2).position(|x| x == b"%d") {
        Some(i) => [&t[..i], num.as_bytes(), &t[i + 2..]].concat(),
        None => t,
    }
}

/// `NpcTalkRefresh(npc, talk, advance)` 0x27b550: with `advance` the node that just played becomes
/// [`talk::LAST`] and the block moves to its `next` (playing that node's scene at once when the old node has flag
/// 4); then the current node's text goes into the prompt buffer (msg 0: the slot is dropped) and its condition
/// picks the true / false node (recursing when that changes the node).
pub fn talk_refresh(w: &mut World, id: MobyId, advance: bool) { talk_refresh_at(w, id, 0, advance) }

/// [`talk_refresh`] on a talk block at pvar offset `base`.
pub fn talk_refresh_at(w: &mut World, id: MobyId, base: usize, advance: bool) {
    let n = p::i16(&w.m(id).pvars, base + talk::NODE);
    let Some(mut nd) = node_of(w, id, base, n) else {
        if advance { p::set_i16(&mut w.mm(id).pvars, base + talk::LAST, n); }
        return;
    };
    if advance {
        let counter = w.counter as u32;
        let pv = &mut w.mm(id).pvars;
        p::set_i16(pv, base + talk::LAST, n);
        p::set_u32(pv, base + talk::SINCE, counter);
        if nd.next != n {
            let old_flags = nd.flags;
            p::set_i16(pv, base + talk::NODE, nd.next);
            let Some(nn) = node_of(w, id, base, nd.next) else { return };
            p::set_u8(&mut w.mm(id).pvars, base + talk::AUTO, (nn.flags & 1) as u8);
            nd = nn;
            if old_flags & 4 != 0 {
                w.svc.interact.talker = Some(id);
                start_scene(w, id, nd.scene);
            }
        }
    }
    if nd.msg != 0 {
        let t = node_text(w, &nd);
        set_text(&mut w.svc.interact.prompt.text, &t);
    } else {
        w.svc.interact.talk_shown = false;
    }
    if nd.kind == 0 { return; }
    let c = condition(w, &nd);
    let to = if c { nd.yes } else { nd.no };
    if to != p::i16(&w.m(id).pvars, base + talk::NODE) {
        let counter = w.counter as u32;
        let auto = node_of(w, id, base, to).map_or(0, |x| (x.flags & 1) as u8);
        let pv = &mut w.mm(id).pvars;
        p::set_i16(pv, base + talk::NODE, to);
        p::set_u32(pv, base + talk::SINCE, counter);
        p::set_u8(pv, base + talk::AUTO, auto);
        talk_refresh_at(w, id, base, false);
    }
}

/// The node's scene / movie hand-off (`DialogStreamStart` hides the talker: mode |= 1 while it runs).
fn start_scene(w: &mut World, id: MobyId, scene: i16) {
    if scene == -1 { return; }
    w.svc.interact.talk_shown = false;
    // DialogStreamStart 0x2ac330 / StartPssMovie 0x2ad0c0 close the help box (`FUN_002258b0`).
    w.svc.help.kill();
    if scene as u16 & 0x4000 == 0 {
        w.mm(id).mode |= 1;
        w.svc.interact.handoffs.push(Handoff::Scene { scene: scene as i32, npc: Some(id) });
    } else {
        w.svc.interact.handoffs.push(Handoff::Movie { movie: (scene as u16 ^ 0x4000) as i32, npc: Some(id) });
        // StartPssMovie's game mode 1 at once (`cinematic::start_movie`): without it the talker's own update in the
        // same tick ran `NpcTalkUpdate` on the auto movie node again and the movie played twice (Eudora's informant).
        w.svc.game_mode = 1;
    }
}

/// The end of the talker's scene or movie (`0x2ac608` / `MovieExitToGameplay`): the talker is shown again and
/// its dialogue advances (`NpcTalkRefresh(npc, talk, 1)`). The engine sets [`Interact::scene_ended`]; the talking
/// classes call this first in their update, so the refresh lands before their own logic as in the game.
pub fn poll_scene_end(w: &mut World, id: MobyId) { poll_scene_end_at(w, id, 0) }

/// [`poll_scene_end`] for a talk block at pvar offset `base`.
pub fn poll_scene_end_at(w: &mut World, id: MobyId, base: usize) {
    if !w.svc.interact.scene_ended || w.svc.interact.talker != Some(id) { return; }
    w.svc.interact.scene_ended = false;
    w.svc.interact.talker = None;
    w.mm(id).mode &= !1;
    talk_refresh_at(w, id, base, true);
}

/// `NpcTalkUpdate(npc, talk)` 0x27b028: true when △ (or an auto node) started the node's scene / movie or advanced
/// a scene-less node this tick.
pub fn talk_update(w: &mut World, id: MobyId) -> bool { talk_update_at(w, id, 0) }

/// [`talk_update`] on a talk block at pvar offset `base` (`NpcTalkUpdate(npc, pvars + base)`).
pub fn talk_update_at(w: &mut World, id: MobyId, base: usize) -> bool {
    let h = HeroView::of(w.hero);
    if h.state == 0x1d || h.hp == 0 { return false; }
    if w.m(id).pvars.len() < base + talk::SIZE { return false; }
    if p::u8(&w.m(id).pvars, base + talk::REGISTERED) == 0 { talk_register_at(w, id, base); }
    if w.svc.game_mode != 0 { return false; }
    let n = p::i16(&w.m(id).pvars, base + talk::NODE);
    if n == -1 || p::u8(&w.m(id).pvars, base + talk::AUTO) == 0xff { return false; }
    let m = w.m(id);
    let (pos, yaw) = ([m.position[0], m.position[1], m.position[2]], m.rotation[2]);
    let r = p::ff(&m.pvars, base + talk::RADIUS);
    // Range first (the game tests it before the refresh), then the node's text and condition, then facing.
    if !talk_rule(&h, pos, yaw, r, true) { return false; }
    talk_refresh_at(w, id, base, false);
    let auto = p::u8(&w.m(id).pvars, base + talk::AUTO) != 0;
    if !auto && !talk_rule(&h, pos, yaw, r, false) { return false; }
    if w.counter < w.svc.interact.cooldown { return false; }
    let Some(nd) = node_of(w, id, base, p::i16(&w.m(id).pvars, base + talk::NODE)) else { return false };
    if !auto && !w.svc.interact.triangle() {
        // The prompt (slot 12 through handle 0x160130) and, for a price condition (kinds 1 / 6), the bolt counter
        // for 60 ticks.
        w.svc.interact.talk_shown = true;
        if nd.kind == 1 || nd.kind == 6 {
            w.svc.counters.hud_bolt_refresh += 1;
            // `queue_animation_update(2, 0x754e, …)` + `FUN_0024b4b0(h, ScaleTicks(60))` (crate::hud).
            w.svc.hud.queue(crate::hud::Request::BOLTS);
            w.svc.hud.keep_up(crate::hud::Request::BOLTS, crate::hud::scale_ticks(60));
        }
        return false;
    }
    let g = talk_slot(w, id);
    if g >= 0 && w.svc.interact.game.talked.get(g as usize).copied().unwrap_or(0) == 0 {
        if let Some(t) = w.svc.interact.game.talked.get_mut(g as usize) { *t = 1; }
        w.svc.interact.writes.push(GameWrite::Talked(g as usize, 1));
    }
    w.svc.interact.talker = Some(id);
    if nd.flags & 8 != 0 {
        let it = nd.item.max(0) as usize;
        match nd.kind {
            1 => {
                let price = w.svc.interact.tables.shop.price(it);
                w.svc.counters.bolts -= price;
            }
            4 => {
                if let Some(b) = w.svc.interact.game.flags.get_mut(it) { *b = 0; }
                w.svc.interact.writes.push(GameWrite::ClearFlag(it));
            }
            _ => {}
        }
    }
    if nd.scene == -1 {
        w.svc.interact.talker = None;
        talk_refresh_at(w, id, base, true);
        return true;
    }
    start_scene(w, id, nd.scene);
    true
}

/// `FUN_0027b438(npc, v)`: the "talked" word of the NPC's global talk slot (`0x13d5bc + 16·slot`) = `v`; nothing for a
/// moby without a slot. (The Kerwan train 822 writes 1 on itself and 3 on the car Ratchet stands on; the commando 114
/// writes 2; the infobots 1.)
pub fn set_talked(w: &mut World, id: MobyId, v: u32) {
    let g = talk_slot(w, id);
    if g < 0 { return; }
    // Kerwan's transports 816 / 1012 store it every tick: a store of the same value writes nothing.
    if w.svc.interact.game.talked.get(g as usize) == Some(&v) { return; }
    if let Some(t) = w.svc.interact.game.talked.get_mut(g as usize) { *t = v; }
    w.svc.interact.writes.push(GameWrite::Talked(g as usize, v));
}

/// `FUN_002783a8(d, npc)` 0x2783a8: when the running scene ends Ratchet stands `d` in front of the NPC (its yaw), facing
/// it (Euler (0, 0, yaw + π)): 0x16cd26 = 1, 0x16ccf0 / 0x16cd00 ([`Interact::scene_end_place`]).
pub fn place_after_scene(w: &mut World, npc: MobyId, d: f32) {
    let m = w.m(npc);
    let yaw = m.rotation[2];
    let pos = [m.position[0] + yaw.cos() * d, m.position[1] + yaw.sin() * d, m.position[2]];
    w.svc.interact.scene_end_place = Some((pos, add_rot(yaw, std::f32::consts::PI)));
}

/// `GiveItem(item, equip)` 0x275760 from a class update: the banner (`ShowBanner(table[item], ticks(300))`, the table
/// entry `item + 0x25` once owned; −1: none), the acquired byte (the talk conditions' mirror), and the saved-game write
/// the engine applies with the item tables ([`GameWrite::GiveItem`]: owned, ammo, vendor stock, quick select, the hand
/// request).
pub fn give_item(w: &mut World, item: usize, equip: bool) {
    let owned = w.hero.owned.has(item);
    let k = if owned { item + 0x25 } else { item };
    let msg = w.svc.interact.tables.give_banners.get(k).copied().unwrap_or(-1);
    if msg != -1 {
        let t = w.ticks(300);
        crate::cinematic::show_banner(w, msg, t);
    }
    if let Some(b) = w.svc.interact.game.acquired.get_mut(item) { *b = 1; }
    w.svc.interact.writes.push(GameWrite::GiveItem { item, equip });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn hero_at(x: f32, y: f32, z: f32, yaw: f32) -> HeroView { HeroView { pos: [x, y, z], yaw, hp: 4, ..Default::default() } }

    #[test]
    fn lease_is_two_ticks_and_owner_keyed() {
        let mut p = Prompt::default();
        assert_eq!(p.try_set(owner::VENDOR, 21475, b"A"), 1);
        assert_eq!((p.owner, p.timer, p.shown()), (1, 2, true));
        assert_eq!(p.try_set(owner::SHIP, 21476, b"B"), 0, "someone else holds it");
        assert_eq!(p.text, b"A");
        assert_eq!(p.try_set(owner::VENDOR, 21475, b"A"), 2);
        // The owner stops asking: one more tick with the lease, then free.
        p.tick();
        assert_eq!((p.owner, p.timer), (1, 1));
        p.tick();
        assert_eq!((p.owner, p.timer), (0, 0));
        assert_eq!(p.try_set(owner::SHIP, 21476, b"B"), 1);
        assert_eq!(p.force(owner::HELP, 0, b""), 3, "force takes it");
        assert_eq!((p.owner, p.msg, p.shown()), (owner::HELP, 0, false));
        assert_eq!(p.text, b"B", "text 0 keeps the buffer");
        assert!(!p.release(owner::SHIP, true));
        assert!(!p.release(owner::HELP, false), "needs the slot up");
        assert!(p.release(owner::HELP, true));
        assert_eq!(p.owner, 0);
        let long = vec![b'x'; PROMPT_TEXT_MAX];
        p.try_set(1, 5, &long);
        assert_eq!(p.text, TOO_LONG);
    }

    #[test]
    fn vendor_rule_distance_height_facing_and_state() {
        let v = [10.0, 0.0, 5.0];
        // Facing the vendor (+x) from 3 units: prompt and use (settled).
        let h = hero_at(7.0, 0.0, 5.0, 0.0);
        assert_eq!(vendor_rule(&h, v, true), (true, true));
        assert_eq!(vendor_rule(&h, v, false), (true, false), "not settled: prompt only");
        // XY boundary 4 inclusive, 4.01 out.
        assert!(vendor_rule(&hero_at(6.0, 0.0, 5.0, 0.0), v, true).0);
        assert!(!vendor_rule(&hero_at(5.99, 0.0, 5.0, 0.0), v, true).0);
        // |Δz| ≤ 2.
        assert!(vendor_rule(&hero_at(7.0, 0.0, 3.0, 0.0), v, true).0);
        assert!(!vendor_rule(&hero_at(7.0, 0.0, 2.9, 0.0), v, true).0);
        // Facing within π/2 (inclusive), away no.
        assert!(vendor_rule(&hero_at(7.0, 0.0, 5.0, FRAC_PI_2 - 1e-3), v, true).0);
        assert!(!vendor_rule(&hero_at(7.0, 0.0, 5.0, FRAC_PI_2 + 1e-2), v, true).0);
        assert!(!vendor_rule(&hero_at(7.0, 0.0, 5.0, PI), v, true).0);
        // Groups and states.
        let mut h2 = h;
        h2.group = 2;
        assert!(!vendor_rule(&h2, v, true).0, "airborne group");
        h2.state = 3;
        assert!(vendor_rule(&h2, v, true).0, "state 3 passes in any group");
        for s in [0x1d, 0x32] {
            let mut h3 = h;
            h3.state = s;
            assert!(!vendor_rule(&h3, v, true).0);
        }
        let mut h4 = h;
        h4.mode = 1;
        assert!(!vendor_rule(&h4, v, true).0, "control mode ≠ 0");
    }

    #[test]
    fn talk_rule_range_and_both_facings() {
        // NPC at the origin facing +x, r = 1.9 (the gold-weapon offers): range 3.8.
        let r = 1.9;
        let h = hero_at(3.0, 0.0, 0.0, PI);
        assert!(talk_rule(&h, [0.0; 3], 0.0, r, false));
        assert!(!talk_rule(&hero_at(3.81, 0.0, 0.0, PI), [0.0; 3], 0.0, r, false), "range 2r");
        // Behind the NPC: its facing is π from the hero > r.
        assert!(!talk_rule(&hero_at(-3.0, 0.0, 0.0, 0.0), [0.0; 3], 0.0, r, false));
        // Auto nodes skip both facings.
        assert!(talk_rule(&hero_at(-3.0, 0.0, 0.0, 0.0), [0.0; 3], 0.0, r, true));
        // The hero must face the NPC within 1.57.
        assert!(!talk_rule(&hero_at(3.0, 0.0, 0.0, 0.0), [0.0; 3], 0.0, r, false));
        assert!(talk_rule(&hero_at(3.0, 0.0, 0.0, PI - 1.56), [0.0; 3], 0.0, r, false));
        assert!(!talk_rule(&hero_at(3.0, 0.0, 0.0, PI - 1.58), [0.0; 3], 0.0, r, false));
        // Height counts (3-D range).
        assert!(!talk_rule(&hero_at(0.0, 0.0, 4.0, 0.0), [0.0; 3], 0.0, r, true));
    }

    #[test]
    fn diff_rots_wraps() {
        assert!((diff_rots(3.0, -3.0) - (2.0 * PI - 6.0)).abs() < 1e-5);
        assert!((diff_rots(-3.0, 3.0) - (2.0 * PI - 6.0)).abs() < 1e-5);
        assert!((diff_rots(0.5, 0.25) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn spendable_gold_bolts() {
        let mut g = TalkGame { gold_bolts: 9, gold_weapons: vec![0; 40], ..Default::default() };
        assert_eq!(g.spendable_gold_bolts(), 9);
        g.gold_weapons[19] = 1;
        assert_eq!(g.spendable_gold_bolts(), 5);
        g.gold_weapons[10] = 1;
        g.gold_weapons[11] = 1;
        assert_eq!(g.spendable_gold_bolts(), 0);
    }

    #[test]
    fn node_parse() {
        let mut b = [0u8; 0x1c];
        b[0..4].copy_from_slice(&1011i32.to_le_bytes());
        b[4..6].copy_from_slice(&1i16.to_le_bytes());
        b[6..8].copy_from_slice(&3i16.to_le_bytes());
        b[8..10].copy_from_slice(&1i16.to_le_bytes());
        b[10..12].copy_from_slice(&37i16.to_le_bytes());
        b[12..14].copy_from_slice(&2i16.to_le_bytes());
        b[14..16].copy_from_slice(&1i16.to_le_bytes());
        b[16..18].copy_from_slice(&0xcu16.to_le_bytes());
        assert_eq!(TalkNode::parse(&b), TalkNode { msg: 1011, scene: 1, next: 3, kind: 1, item: 37, yes: 2, no: 1, flags: 0xc });
    }
}
