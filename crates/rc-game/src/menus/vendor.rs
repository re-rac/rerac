//! The Gadgetron vendor, game mode 5: `OpenVendorMenu` 0x2ae1a0, `VendorBuildItemList` 0x2adef0,
//! `VendorModeUpdate` 0x2b03b8, the buy flow 0x2af7e8 (Lombyte `DrawSpriteHelper_C`), `VendorExit` 0x2ae660, the
//! render `DrawWorld_Mode5` 0x2b4020 and the screens `VendorDrawScreens` 0x2b3130 (level01). The whole experience is
//! documented in docs/plan/interaction.md §9 ("Vendor, as built"); docs/plan/menus.md §4 has the flow.
//!
//! [`Vendor::frame`] is one main-loop frame of mode 5 (the game runs `VendorModeUpdate` once per frame). Tables come
//! from the level ([`VendorTables::load`]): the price records 0x1c4530 (`interact::ShopTable`), the item
//! definitions 0x179f40 (+0 name, +0x10 class, +0x38 icon), the description ids (hologram table 0x1c2588, +0x10), the
//! 24 ticker lines 0x1ca538, the ticker's LED font (glyph cells 0x1ca598, advances 0x1ca698, HUD icon 0xe935) and the
//! placement tables ([`layout::VendorLayout`]).
//!
//! **Substates** (0x1ca940, timer 0x1ca944):
//! * **Open** (`OpenVendorMenu`): the list, the selection in the middle, sound 3, `FadeToBlack(4)` (4 blocking
//!   frames: [`Vendor::pre_fade`]), the vendor cut to **seq 2** (the unfold) at half speed, Ratchet `SetState(100)`
//!   and hidden, the camera cut to the vendor's front ([`vendor_camera`]), `music_Pause(0)`.
//! * **0** (40 frames): the **world runs** ([`Vendor::world_runs`]: moby loop, particles, tick counter; not the
//!   hero or the camera), the full-screen fade 1.0 → 0 at −0.34 per frame. At 40: **seq 3** (blend 8, speed 1),
//!   sound 4, the screens power on (8 frames).
//! * **1** (the menu; no world update): the vendor's `MobyAnimAdvance`, the salesman ([`salesman`]; created the
//!   frame after the first menu render started reading `vendor.bin`), the spinning hologram (φ += 0.05), the pad:
//!   Left / Right (0x13cb04 edges not seen last frame) move the selection (≤ 7 items: no wrap; ≥ 8: a wrapping
//!   carousel whose offset 0x1ca98c moves ±56 and eases back 4 px per frame), each with sound 1, the entry's
//!   description on the ticker, the HUD's ammo slot and a possible remark of the salesman; ✕ on an unlocked entry →
//!   the buy flow (sound 0); △ (not while the salesman's data loads) → sound 5, the screens power off over 8 frames,
//!   then **seq 4** from frame 9 backwards at half speed, substate 2.
//! * **Buy flow** (0x1ca99c), timed by the popup moby (class 0x471, seq 0 = 4 frames): 1 creates it (hard cut to
//!   seq 0) and asks "How many?" for an ammo entry that is not full and whose unit price the bolts cover (quantity
//!   1.., Left / Right with auto-repeat after 16 held frames every 8th, after 48 every frame, capped at min(max −
//!   ammo, bolts / unit)), "Purchase?" for an affordable weapon, else why not (sound 2); 2: the popup opens (seq 0,
//!   then seq 1), ✕ confirms (blend back to seq 0's last frame over 8 ticks) and △ cancels (1 tick); 3: when it
//!   lands, speed −1 (it folds up); 4: when that ends, the purchase — ammo: `AddAmmo` (overflow returned), bolts −=
//!   unit·(qty − overflow), stats 0x13dd70 += bought; a weapon: `GiveItem(item, 1)` (owned, quick-select slot, vendor
//!   stock byte |= 0x40), bolts −= price; either → ticker 20319 "THANK YOU", sound 7, list rebuilt, the salesman's
//!   idle timer reset; short of bolts → ticker 20320, sound 0. A cancel plays sound 0.
//! * **2** leave (40 frames, the world runs), then seq 1, `VendorExit`: mode 0, the HUD slots released, Ratchet shown
//!   and teleported 3.5 in front of the vendor, `CameraScript2(2)` (the follow camera blends back), the vendor's
//!   state 1, sound 6, music resumed. A weapon bought with a demo scene (0x1ca4a0) plays it first (substate 3: the
//!   space-scene player `crate::travel::space::SUB_DEMO` in the vendor's frame, `rc-engine` `travel_render`), then
//!   `VendorExit(1)` (the follow camera reset behind Ratchet).
//!
//! **Screens** ([`screens`]): six 512×128 targets placed over the vendor's monitor joints; their content is
//! [`Vendor::screen_content`] (target pixels), their static [`Vendor::statics`]; the engine places and composes them.
//! The weapon demo: the decision and the request here ([`VendorOut::weapon_demo`]), the playback by the space-scene
//! player (`crate::travel::space::ShipMode::demo_start`). The PDA's remote vendor: the list and prices here, its moby and view in the engine
//! (`interact_render::remote_vendor`), no hologram ([`Vendor::scene`]) and no cone.

pub mod layout;
pub mod salesman;
pub mod screens;

use crate::game_state::{GameState, SessionState};
use crate::menus::{sprite, text, MenuAssets, MenuDraw, MenuInput, Overlay};
use crate::moby_update::interact::ShopTable;
use crate::pad::button;
use crate::rng::Rng;
use layout::VendorLayout;
use rc_formats::font::Font;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, MobyFrame};
use rc_formats::save_game::ItemTables;
use salesman::Salesman;
use std::sync::Arc;

/// Items with an entry in the tables.
pub const ITEMS: usize = 37;
/// Text ids.
pub mod msg {
    pub const AMMO: i32 = 20317;
    pub const THANK_YOU: i32 = 20319;
    pub const CANT_AFFORD_TICKER: i32 = 20320;
    pub const EXIT: i32 = 20192;
    pub const NO: i32 = 21067;
    pub const YES: i32 = 21070;
    pub const BUY: i32 = 21044;
    pub const BACK: i32 = 21043;
    pub const MAXED: i32 = 21046;
    pub const HOW_MANY: i32 = 21047;
    pub const CANT_AFFORD: i32 = 21048;
    pub const PURCHASE: i32 = 21049;
}
/// The ticker's LED font texture (`GetIconFrame(0xe935, 0)`).
pub const LED_ICON: u16 = 0xe935;
/// The LED glyphs' RGBA (`fun_00238310`, level01 0x2b19b4..0x2b19bc: `0x8040 << 16 | 0x4040`).
pub const LED_RGBA: u32 = 0x8040_4040;
/// The ticker format 0x20a510: 18 spaces, then the message.
const TICKER_PAD: usize = 18;
/// The vendor class, its popup (0x471) and the class-13 backdrop of the item panel.
pub const VENDOR_CLASS: i16 = 11;
pub const POPUP_CLASS: i16 = 0x471;
pub const BACKDROP_CLASS: i16 = 13;
/// `ticks(0x28)` / `ticks(0x24)`: the fly-in and leave, the glass quads' window.
pub const FLY_TICKS: i32 = 40;
pub const GLASS_TICKS: i32 = 36;
/// The screens' power-on / off (0x16229c / 0x162294).
pub const POWER_TICKS: i32 = 8;

/// Vendor class sound indices (`PlayClassSound(n, 0, vendor)`).
pub mod sound {
    pub const SELECT: u8 = 0;
    pub const CURSOR: u8 = 1;
    pub const DENIED: u8 = 2;
    pub const OPEN: u8 = 3;
    pub const SCREENS_ON: u8 = 4;
    pub const EXIT: u8 = 5;
    pub const CLOSED: u8 = 6;
    pub const PURCHASE: u8 = 7;
}

/// The classes mode 5 animates itself (not in the moby table): the popup and the salesman (with the sequences from
/// `vendor.bin`).
#[derive(Clone, Debug, Default)]
pub struct VendorClasses {
    pub popup: Option<MobyAnimClass>,
    pub salesman: Option<MobyAnimClass>,
}

/// What the vendor reads from the level.
#[derive(Clone, Debug, Default)]
pub struct VendorTables {
    pub shop: ShopTable,
    /// Item definition +0x00: name id; +0x10: moby class; +0x38: icon id.
    pub name: Vec<i32>,
    pub class: Vec<i16>,
    pub icon: Vec<u16>,
    /// Hologram table +0x10: the description the ticker shows.
    pub desc: Vec<i32>,
    /// 0x1ca538: the idle ticker lines.
    pub ticker: Vec<i32>,
    /// 0x1ca598 / 0x1ca698: LED font cell (−1: none; `(v & 0xffff) >> 4` = u, `v >> 20` = v) and advance of the
    /// characters 0x20..0x5f.
    pub led_cell: Vec<i32>,
    pub led_adv: Vec<i32>,
    /// The placement tables (None: the overlay does not have them; the screens are then not placed).
    pub layout: Option<VendorLayout>,
    pub classes: Arc<VendorClasses>,
}

fn find(ov: &Overlay, pat: &[u8]) -> Option<u32> {
    ov.sections().iter().filter(|s| s.kind != 8).find_map(|s| s.data.windows(pat.len()).position(|w| w == pat).map(|i| s.dest + i as u32))
}

fn i32s(v: &[i32]) -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() }

impl VendorTables {
    /// From the level overlay: `item_defs` = the item definition table (`ItemTables::item_defs_addr`); the other
    /// tables by their bytes (the description ids of the Bomb Glove / Devastator records, the first ticker line
    /// ids, the LED advances of ' ' '!' '"'), the placement tables by their labels ([`VendorLayout::load`]).
    pub fn load(ov: &Overlay, item_defs: u32, shop: ShopTable) -> Option<VendorTables> {
        let name = (0..ITEMS as u32).map(|i| ov.i16(item_defs + 0x4c * i).map(i32::from)).collect::<Option<Vec<_>>>()?;
        let class = (0..ITEMS as u32).map(|i| ov.i16(item_defs + 0x4c * i + 0x10)).collect::<Option<Vec<_>>>()?;
        let icon = (0..ITEMS as u32).map(|i| ov.u16(item_defs + 0x4c * i + 0x38)).collect::<Option<Vec<_>>>()?;
        // Records 10 / 11 of the hologram table: desc 21083 at +0x10, 21084 at +0x24.
        let holo = ov
            .sections()
            .iter()
            .filter(|s| s.kind != 8)
            .flat_map(|s| s.data.windows(0x18).enumerate().filter(|(_, w)| w[..4] == 21083i32.to_le_bytes() && w[0x14..] == 21084i32.to_le_bytes()).map(move |(i, _)| s.dest + i as u32))
            .next()?
            .checked_sub(10 * 0x14 + 0x10)?;
        let desc = (0..ITEMS as u32).map(|i| ov.i32(holo + 0x14 * i + 0x10)).collect::<Option<Vec<_>>>()?;
        let t = find(ov, &i32s(&[20321, 20322, 20322, 20322, 20322, 20323]))?;
        let ticker = (0..24).map(|i| ov.i32(t + 4 * i)).collect::<Option<Vec<_>>>()?;
        let adv = find(ov, &i32s(&[9, 5, 10, 10, 10, 10, 10, 5]))?;
        let led_adv = (0..64).map(|i| ov.i32(adv + 4 * i)).collect::<Option<Vec<_>>>()?;
        let led_cell = (0..64).map(|i| ov.i32(adv - 0x100 + 4 * i)).collect::<Option<Vec<_>>>()?;
        let layout = VendorLayout::load(ov);
        Some(VendorTables { shop, name, class, icon, desc, ticker, led_cell, led_adv, layout, classes: Arc::default() })
    }
}

/// One entry of the list 0x1caa10 (`{item, is_ammo, locked, 0, hologram}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub item: usize,
    pub ammo: bool,
    pub locked: bool,
}

/// `VendorBuildItemList(remote)` 0x2adef0: the stock 0x15edd0 in order (not owned → the weapon, unless remote; owned
/// (bit 0x40) → its ammo when it has an ammo price), then the ammo of every owned item with an ammo price that is
/// not listed yet.
pub fn build_list(gs: &GameState, shop: &ShopTable, remote: bool) -> Vec<Entry> {
    let g = &gs.global;
    let n = g.vendor.iter().position(|&b| b == 0xff).unwrap_or(g.vendor.len());
    let mut out: Vec<Entry> = Vec::new();
    for &b in &g.vendor[..n] {
        let item = (b & 0x3f) as usize;
        if b & 0x40 == 0 {
            if !remote { out.push(Entry { item, ammo: false, locked: false }); }
        } else if shop.ammo_price(item) != 0 {
            out.push(Entry { item, ammo: true, locked: false });
        }
    }
    for item in 0..ITEMS {
        if g.owned[item] != 0 && shop.ammo_price(item) != 0 && !out.iter().any(|e| e.item == item) {
            out.push(Entry { item, ammo: true, locked: false });
        }
    }
    out
}

/// The vendor camera (`OpenVendorMenu`: 0x1ca9a0 = the vendor's rows · `offset` (gp−0x5ba0: (3.8, 0, 1.5)) + its
/// position, yaw = the vendor's + π, pitch 0): (eye, forward, left, up).
pub fn vendor_camera_at(offset: [f32; 3], pos: [f32; 3], rows: [[f32; 4]; 4], yaw: f32) -> ([f32; 3], [[f32; 3]; 3]) {
    let eye = std::array::from_fn(|k| pos[k] + offset[0] * rows[0][k] + offset[1] * rows[1][k] + offset[2] * rows[2][k]);
    let cy = crate::moby_update::interact::add_rot(yaw, std::f32::consts::PI);
    let (s, c) = cy.sin_cos();
    (eye, [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]])
}

/// [`vendor_camera_at`] with the Novalis constant (3.8, 0, 1.5).
pub fn vendor_camera(pos: [f32; 3], rows: [[f32; 4]; 4], yaw: f32) -> ([f32; 3], [[f32; 3]; 3]) { vendor_camera_at([3.8, 0.0, 1.5], pos, rows, yaw) }

/// A change of the vendor moby's animation the frame asks for (applied by the engine to the moby table).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VendorAnim {
    /// `hard_cut(vendor, seq, frame)` 0x26c5a8.
    HardCut { seq: u8, frame: i32 },
    /// `MobyAnimBlend(vendor, seq, frame, ticks)` 0x26c660.
    Blend { seq: u8, frame: i32, ticks: i32 },
    /// +0x58.
    Speed(f32),
}

/// What a frame did besides the state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VendorOut {
    /// Vendor class sounds, in order.
    pub sounds: Vec<u8>,
    /// Vendor animation requests, in order.
    pub anim: Vec<VendorAnim>,
    /// The vendor's `MobyAnimAdvance` of substate 1 (the moby loop advances it in substates 0 / 2).
    pub advance_vendor: bool,
    /// `VendorExit` ran: back to mode 0.
    pub exit: bool,
    /// A purchase: (item, ammo entry, bolts spent, units bought).
    pub purchase: Option<(usize, bool, i32, i32)>,
    /// The salesman starts this voice line (stream id; `vendor_audio` entry = id − 10000).
    pub voice: Option<i32>,
    /// Substate 2's end: the four arm manipulators 0x166300 detached from the vendor (`DetachManipulator`).
    pub detach_arms: bool,
    /// Substate 2's end with a bought weapon that has a demo scene: `VendorStartWeaponDemo` 0x2ae7f8 instead of the
    /// exit (substate 3).
    pub weapon_demo: Option<WeaponDemo>,
    /// `VendorExit`: the dialogue line stopped (0x15172a ∉ {6, 7} → 5).
    pub stop_voice: bool,
    /// The selection changed (`FUN_002af3f0`: the item model and the hologram are rebuilt).
    pub selection_changed: bool,
}

/// `VendorStartWeaponDemo` 0x2ae7f8 (substate 2's end, the vendor not the PDA's, a weapon bought (0x1ca988 ≥ 0) whose
/// demo scene 0x1ca4a0[item] ≥ 0): `CameraScript2(2)`; the dialogue line stopped (0x15172a ∉ {6, 7} → 5); 0x15f5d8 = 1;
/// the vendor's +0x20 = 1; substate 3, timer 0; the scene's frame: the origin 0x1caa00 = vendor rows · gp−0x5b10 +
/// vendor position, z + 2 then `GroundHeight(0.5, origin)`; the rotation 0x1ca9f0 = (0, 0, vendor yaw + π) → the matrix
/// 0x1ca9c0; the scene state cleared (0x16cce0 0x1c0 bytes, 0x17c7c0 0x40); the scene buffer 0x1611cc − 0x40000; fade
/// 0x15f3fc = 0, gp−0x7800 = 0; the hand item: the held item ≠ the bought one → `FUN_002305e8(0, 0)`, 0x141408 = item (0
/// for the Drone Device 0x18); `select_world_object_resource_tables(item class)`; `FUN_002594e0(scene)` (the space-scene
/// lump `unknown_1530[scene]` read, `FadeToBlack(4)`, its chunk table); 0x1516ec = 10036 (`vendor_audio[36]`); chunk 0
/// parsed; the stream waited for and started. Substate 3 then plays the scene (the space-scene player of mode 6: the
/// camera and actors in the vendor's frame) and ends with the actors deleted, `VendorExit(1)` (or `FUN_002aecf0` for the
/// demo-only entry `FUN_002aea70`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaponDemo {
    pub item: usize,
    /// The scene: `unknown_1530[scene]` (NTSC; PAL + 14).
    pub scene: i32,
    /// `vendor_audio` stream id of the demo's sound (0x2734).
    pub stream: i32,
}

/// The weapon demo's stream (0x1516ec = 0x2734).
pub const DEMO_STREAM: i32 = 0x2734;

/// `OpenVendorMenu` 0x2ae1a0's four manipulators 0x166300 (0x40 bytes each, .bss) on the vendor's joint lists
/// 0x14..0x17: (list, translation); quaternion identity, scale 1 (`AttachManipulator`); with fewer than 8 entries record
/// 2's translation y (0x1663b4) = +0x681·(7 − n) and record 3's (0x1663f4) = −0x681·(7 − n), else 0, then copied into
/// records 0 (0x166334) and 1 (0x166374): the strip's ends pulled in for a short list.
pub fn arm_manipulators(entries: usize) -> [(u8, [f32; 3]); 4] {
    let d = if entries < 8 { ((7 - entries as i32) * 0x681) as f32 } else { 0.0 };
    [(0x14, [0.0, d, 0.0]), (0x15, [0.0, -d, 0.0]), (0x16, [0.0, d, 0.0]), (0x17, [0.0, -d, 0.0])]
}

/// The popup moby (0x1ca960 + 0x400, class 0x471).
#[derive(Clone, Debug, PartialEq)]
pub struct Popup {
    pub anim: AnimState,
    pub snapshot: Option<MobyFrame>,
}

/// The vendor's state (0x1ca940.., the popup, the ticker, the screens).
#[derive(Clone, Debug)]
pub struct Vendor {
    pub tables: VendorTables,
    /// 0x1ca940 / 0x1ca944.
    pub sub: u8,
    pub t: i32,
    /// 0x1ca980: the PDA's remote vendor (ammo only, PDA prices).
    pub remote: bool,
    /// 0x1caa10.. / 0x1cab50.
    pub items: Vec<Entry>,
    /// 0x1ca998: selected entry.
    pub sel: usize,
    /// 0x1ca98c: carousel offset (≥ 8 entries), eased back 4 a frame ([`Vendor::ease_strip`]).
    pub scroll: i32,
    /// The carousel's selection frame is drawn this frame (the offset was 0 before its ease).
    pub strip_settled: bool,
    /// 0x1ca99c: buy flow 0..4.
    pub buy: u8,
    /// 0x1622b4: quantity; 0x162290: the popup asks a quantity; 0x16228c: confirmed.
    pub qty: i32,
    pub qty_mode: bool,
    pub confirm: bool,
    /// The popup moby (while buying).
    pub popup: Option<Popup>,
    /// 0x162298 / 0x162294: △ pressed, frames until the leave (the screens power off meanwhile).
    pub exit_req: bool,
    pub exit_t: i32,
    /// 0x1622a0 / 0x16229c: the screens power on, frames left.
    pub power_on: bool,
    pub power_t: i32,
    /// 0x1622a8 / 0x1622ac: Left / Right held frames (auto-repeat).
    pub held_l: i32,
    pub held_r: i32,
    /// gp−0x4950: 0x13cb04 of the last frame.
    pub prev_pressed: u32,
    /// 0x1ca988: the weapon bought last (−1).
    pub bought: i32,
    /// 0x1ca96c: ticker text; 0x1ca984: its scroll (2 px per frame).
    pub ticker: Vec<u8>,
    pub ticker_scroll: i32,
    /// 0x15f3fc: fade-to-black factor.
    pub fade: f32,
    /// Frames of the open's `FadeToBlack(4)` still to show (the camera cuts after them).
    pub pre_fade: i32,
    /// 0x1ca948: the menu's first render happened (the world snapshot; the salesman's data is being read).
    pub rendered: bool,
    /// 0x16118c: the salesman's data is loading (△ is refused meanwhile); 0x161188: he exists.
    pub salesman_loading: bool,
    pub salesman: Option<Salesman>,
    /// 0x1622a4: the hologram's angle (+0.05 per frame).
    pub spin: f32,
    /// The class-13 mobys' Euler angles (+0 / +0x500: x −0.05, z +0.05 per frame, y −0.36).
    pub backdrop: [[f32; 3]; 2],
    /// The screens' static.
    pub statics: screens::Statics,
    /// The language 0x15ed88 (the salesman's lines).
    pub lang: i32,
    /// 0x15f3f8: frames since the vendor opened (the salesman's remark spacing).
    pub vsync: i64,
}

impl Vendor {
    /// `OpenVendorMenu(vendor)` 0x2ae1a0 without the moby / camera / HUD parts (the engine's): the list, the
    /// selection in the middle, the carousel offset, sound 3, the vendor's cut to seq 2 at half speed.
    pub fn open(tables: VendorTables, gs: &GameState, remote: bool, out: &mut VendorOut) -> Vendor {
        let items = build_list(gs, &tables.shop, remote);
        let n = items.len();
        let sel = n / 2;
        let base = if n < 3 { 1 } else { sel as i32 };
        out.sounds.push(sound::OPEN);
        out.anim.push(VendorAnim::HardCut { seq: if remote { 3 } else { 2 }, frame: 0 });
        // +0x58 = 0x15ed60 (the global rate, 1.0) · 0.5.
        out.anim.push(VendorAnim::Speed(0.5));
        out.selection_changed = true;
        Vendor {
            tables,
            sub: 0,
            t: 0,
            remote,
            items,
            sel,
            scroll: (base - sel as i32) * 0x28,
            strip_settled: false,
            buy: 0,
            qty: 0,
            qty_mode: false,
            confirm: false,
            popup: None,
            exit_req: false,
            exit_t: 0,
            power_on: false,
            power_t: 0,
            held_l: 0,
            held_r: 0,
            prev_pressed: 0,
            bought: -1,
            ticker: Vec::new(),
            ticker_scroll: 0,
            fade: 1.0,
            pre_fade: if remote { 0 } else { 4 },
            rendered: false,
            salesman_loading: false,
            salesman: None,
            spin: 0.0,
            backdrop: [[0.0, -0.36, 0.0]; 2],
            statics: screens::Statics::default(),
            lang: 0,
            vsync: 0,
        }
    }

    /// The selected entry.
    pub fn current(&self) -> Option<Entry> { self.items.get(self.sel).copied() }

    /// The world update runs this frame (substates 0 and 2 after the open's fade; `VendorModeUpdate`'s
    /// `MobyUpdateLoop` … `IncrementTickCounter` block).
    pub fn world_runs(&self) -> bool { self.pre_fade == 0 && (self.sub == 0 || self.sub == 2) }

    /// The screens are drawn this frame (the menu render after the snapshot: substate 1).
    pub fn screens_shown(&self) -> bool { self.pre_fade == 0 && self.sub == 1 }

    /// The glass quads' draw callback is registered (`RegisterDrawCallback2(0x2b3700)` while the timer ≤ 36 in
    /// substates 0 and 2).
    pub fn glass_only(&self) -> bool { self.world_runs() && self.t <= GLASS_TICKS }

    /// The screens' power factor (1: at rest): power-on (8 − 0x16229c)/8, power-off 0x162294/8.
    pub fn power(&self) -> f32 {
        let mut f = 1.0;
        if self.power_on { f = (POWER_TICKS - self.power_t) as f32 * 0.125; }
        if self.exit_req { f = self.exit_t as f32 * 0.125; }
        f
    }

    fn ammo_unit(&self, item: usize) -> i32 {
        if self.remote { self.tables.shop.pda_ammo_price(item) as i32 } else { self.tables.shop.ammo_price(item) as i32 }
    }

    /// The weapon price (the discounted one while the flag 0x13d4e3 is set).
    fn price(&self, gs: &GameState, item: usize) -> i32 {
        if discount(gs) { self.tables.shop.discounted(item) } else { self.tables.shop.price(item) }
    }

    fn full(&self, gs: &GameState, item: usize) -> bool { self.tables.shop.max_ammo(item) as i32 <= gs.global.ammo[item] }

    /// 0x1ca4a0[item]: the item's weapon demo scene (None: −1, or no table).
    fn layout_demo(&self, item: usize) -> Option<i32> { self.tables.layout.as_ref()?.demo.get(item).copied().filter(|&s| s >= 0) }

    /// `set_scrolling_status_message(msg)` 0x2aede0: 18 spaces and the text, scroll 0.
    pub fn set_ticker(&mut self, text: &[u8]) {
        self.ticker = vec![b' '; TICKER_PAD];
        self.ticker.extend_from_slice(text);
        self.ticker_scroll = 0;
    }

    /// The HUD's ammo slot after a move: the selected ammo entry (`queue_animation_update(0x30, 60000 + item, …)`)
    /// or none.
    pub fn hud_ammo(&self, gs: &GameState) -> Option<(u16, i32, i32)> {
        let e = self.current().filter(|e| e.ammo)?;
        Some((e.item as u16, gs.global.ammo[e.item], self.tables.shop.max_ammo(e.item) as i32))
    }

    /// One frame of `VendorModeUpdate` (0x2b03b8). `rng` is the game's stream (the ticker picks its idle line with
    /// `randi(24)` and the screens' static draws in the render; the salesman's line choices).
    pub fn frame(&mut self, inp: &MenuInput, gs: &mut GameState, items: &ItemTables, session: &mut SessionState, assets: &MenuAssets, rng: &mut Rng) -> VendorOut {
        let mut out = VendorOut::default();
        if self.pre_fade > 0 {
            self.pre_fade -= 1;
            return out;
        }
        self.vsync += 1;
        match self.sub {
            0 => {
                self.fade = (self.fade - 0.34).max(0.0);
                self.t += 1;
                if self.t >= FLY_TICKS || self.remote {
                    self.fade = 0.0;
                    self.sub = 1;
                    self.t = 0;
                    out.anim.push(VendorAnim::Blend { seq: 3, frame: 0, ticks: 8 });
                    out.anim.push(VendorAnim::Speed(1.0));
                    self.power_on = true;
                    self.power_t = POWER_TICKS;
                    out.sounds.push(sound::SCREENS_ON);
                }
            }
            1 => self.menu(inp, gs, items, session, assets, rng, &mut out),
            2 => {
                // The demo test before the timer (0x1ca980 = 0, 0x1ca988 ≥ 0, 0x1ca4a0[0x1ca988] ≥ 0).
                let demo = (!self.remote && self.bought >= 0).then(|| self.layout_demo(self.bought as usize)).flatten();
                self.t += 1;
                if self.t >= FLY_TICKS || self.remote {
                    out.anim.push(VendorAnim::Blend { seq: 1, frame: 0, ticks: 8 });
                    // DetachManipulator(vendor, 0x166300 + 0x40·k) for the attached records.
                    out.detach_arms = true;
                    match demo {
                        Some(scene) => {
                            // VendorStartWeaponDemo: substate 3 (the engine plays the scene and exits: VendorExit(1)).
                            self.sub = 3;
                            self.t = 0;
                            out.weapon_demo = Some(WeaponDemo { item: self.bought as usize, scene, stream: DEMO_STREAM });
                            out.stop_voice = true;
                        }
                        None => {
                            out.exit = true;
                            out.stop_voice = true;
                            out.sounds.push(sound::CLOSED);
                        }
                    }
                }
            }
            // Substate 3: the demo scene runs (the engine's; its end is `VendorExit(1)`).
            3 => {}
            _ => {
                out.exit = true;
                out.stop_voice = true;
            }
        }
        out
    }

    /// Substate 1 (and the buy flow).
    #[allow(clippy::too_many_arguments)]
    fn menu(&mut self, inp: &MenuInput, gs: &mut GameState, items: &ItemTables, session: &mut SessionState, assets: &MenuAssets, rng: &mut Rng, out: &mut VendorOut) {
        // The salesman: created on the first frame after the render that started reading his data (0x16118c).
        if self.salesman.is_none() && self.salesman_loading {
            self.salesman_loading = false;
            if let Some(c) = self.tables.classes.salesman.as_ref() { self.salesman = Some(Salesman::new(c, self.lang, rng)); }
        }
        out.advance_vendor = true;
        let keys = self.tables.layout.as_ref().map_or([[100, 100], [200, 100], [44, 100]], |l| l.talk_keys);
        if let (Some(s), Some(c)) = (self.salesman.as_mut(), self.tables.classes.salesman.as_ref()) {
            let o = s.frame(c, &keys);
            if o.start_voice.is_some() { out.voice = o.start_voice; }
        }
        // The class-13 mobys and the hologram (`multiply_global_scale(±0.05)`).
        let add = crate::moby_update::interact::add_rot;
        for b in &mut self.backdrop {
            b[0] = add(b[0], -0.05);
            b[1] = -0.36;
            b[2] = add(b[2], 0.05);
        }
        self.spin = add(self.spin, 0.05);
        self.t += 1;
        let prev = self.prev_pressed;
        let edge = |b: u32| inp.pressed_u & b != 0 && prev & b == 0;
        let n = self.items.len();
        if self.buy == 0 && !self.exit_req && n > 0 {
            let mut moved = false;
            if n < 8 {
                if edge(button::RIGHT) && self.sel + 1 < n {
                    self.sel += 1;
                    moved = true;
                }
                if edge(button::LEFT) && self.sel > 0 {
                    self.sel -= 1;
                    moved = true;
                }
            } else {
                if edge(button::RIGHT) && self.scroll < 0x39 {
                    self.scroll += 0x38;
                    self.sel = if self.sel + 1 >= n { 0 } else { self.sel + 1 };
                    moved = true;
                }
                if edge(button::LEFT) && self.scroll > -0x39 {
                    self.scroll -= 0x38;
                    self.sel = if self.sel == 0 { n - 1 } else { self.sel - 1 };
                    moved = true;
                }
            }
            if moved {
                // FUN_002af248(0): maybe a remark (before the sound, as the ≤ 7 branch; the carousel plays the sound
                // first: the two orders give the same stream).
                if let (Some(s), Some(c)) = (self.salesman.as_mut(), self.tables.classes.salesman.as_ref()) { s.request_at(c, 0, rng, self.vsync); }
                out.sounds.push(sound::CURSOR);
                let d = self.current().map_or(0, |e| self.tables.desc.get(e.item).copied().unwrap_or(0));
                let t = assets.msg(d).to_vec();
                self.set_ticker(&t);
                out.selection_changed = true;
            }
            if edge(button::CROSS) && self.current().is_some_and(|e| !e.locked) {
                self.buy = 1;
                out.sounds.push(sound::SELECT);
            }
            if inp.pressed_u & button::TRIANGLE != 0 && !self.salesman_loading {
                self.exit_req = true;
                self.exit_t = POWER_TICKS;
                out.sounds.push(sound::EXIT);
            }
        }
        self.buy_flow(inp, gs, items, session, assets, rng, out);
        self.ease_strip();
        self.prev_pressed = inp.pressed_u;
        if self.power_on {
            if self.power_t == 0 { self.power_on = false; } else { self.power_t -= 1; }
        }
        if self.exit_req {
            self.exit_t -= 1;
            if self.exit_t == 0 {
                out.anim.push(VendorAnim::Blend { seq: 4, frame: 9, ticks: 8 });
                out.anim.push(VendorAnim::Speed(-0.5));
                self.sub = 2;
                self.t = 0;
                self.rendered = false;
            }
        }
    }

    fn popup_class(&self) -> Option<&MobyAnimClass> { self.tables.classes.popup.as_ref() }

    /// The popup's `MobyAnimAdvance`; returns the wrap flag (+0x70 & 2) and whether key A is seq 0.
    fn popup_advance(&mut self) -> (bool, bool) {
        let class = self.tables.classes.popup.clone();
        match (self.popup.as_mut(), class.as_ref()) {
            (Some(p), Some(c)) => {
                moby_anim::advance(&mut p.anim, c);
                (p.anim.flags & 2 != 0, p.anim.seq_a == 0)
            }
            // Without the class the popup's animation is 4 frames per phase (seq 0 = 4 keys at rate 1/2 is 8 ticks).
            _ => (true, true),
        }
    }

    fn popup_blend(&mut self, seq: u8, frame: i32, ticks: i32) {
        let class = self.tables.classes.popup.clone();
        if let (Some(p), Some(c)) = (self.popup.as_mut(), class.as_ref()) { moby_anim::set_sequence(&mut p.anim, c, seq, frame, ticks, &mut p.snapshot); }
    }

    fn popup_last_frame(&self) -> i32 { self.popup_class().and_then(|c| c.sequence(0)).map_or(0, |q| q.header.frame_count as i32 - 1) }

    /// The buy flow 0x2af7e8.
    #[allow(clippy::too_many_arguments)]
    fn buy_flow(&mut self, inp: &MenuInput, gs: &mut GameState, items: &ItemTables, session: &mut SessionState, assets: &MenuAssets, rng: &mut Rng, out: &mut VendorOut) {
        let Some(e) = self.current() else {
            self.buy = 0;
            return;
        };
        let bolts = gs.global.bolts;
        let prev = self.prev_pressed;
        let edge = |b: u32| inp.pressed_u & b != 0 && prev & b == 0;
        match self.buy {
            1 => {
                self.qty = 1;
                self.qty_mode = false;
                // InitMobyInstance(popup, 0x471) at the vendor, hard cut to seq 0 (a one-frame seq 0 would freeze).
                let still = AnimState { seq_a: 0, frame_a: 0, seq_b: 0, frame_b: 0, t: 0.0, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false };
                let mut p = Popup { anim: still, snapshot: None };
                if let Some(c) = self.popup_class() {
                    p.anim = AnimState::spawn(c);
                    moby_anim::hard_cut(&mut p.anim, c, 0, 0);
                    if c.sequence(0).is_some_and(|q| q.header.frame_count < 2) {
                        p.anim.speed = 0.0;
                        p.anim.frame_b = 0;
                    }
                }
                self.popup = Some(p);
                let full = e.ammo && self.full(gs, e.item);
                let ok = if e.ammo { !full && self.ammo_unit(e.item) <= bolts } else { self.price(gs, e.item) <= bolts };
                if e.ammo && ok { self.qty_mode = true; }
                if !ok { out.sounds.push(sound::DENIED); }
                self.buy = 2;
            }
            2 => {
                if self.qty_mode {
                    self.held_l = if inp.held_u & button::LEFT != 0 { self.held_l + 1 } else { 0 };
                    self.held_r = if inp.held_u & button::RIGHT != 0 { self.held_r + 1 } else { 0 };
                    let rep = |h: i32| (h > 15 && h & 7 == 0) || h > 47;
                    if (inp.pressed_u & button::LEFT != 0 || rep(self.held_l)) && self.qty > 1 {
                        self.qty -= 1;
                        if self.held_l & 3 == 0 || self.held_l < 16 { out.sounds.push(sound::CURSOR); }
                    }
                    if inp.pressed_u & button::RIGHT != 0 || rep(self.held_r) {
                        let unit = self.ammo_unit(e.item);
                        if unit != 0 {
                            let room = self.tables.shop.max_ammo(e.item) as i32 - gs.global.ammo[e.item];
                            let max = room.min(bolts / unit);
                            if self.qty < max {
                                self.qty += 1;
                                if self.held_r & 3 == 0 || self.held_r < 16 { out.sounds.push(sound::CURSOR); }
                            }
                        }
                    }
                }
                let last = self.popup_last_frame();
                if edge(button::TRIANGLE) {
                    self.buy = 3;
                    self.confirm = false;
                    self.popup_blend(0, last, 1);
                } else if edge(button::CROSS) {
                    self.buy = 3;
                    self.confirm = true;
                    self.popup_blend(0, last, 8);
                } else {
                    let (wrapped, on0) = self.popup_advance();
                    if wrapped && on0 && self.popup_class().is_some() { self.popup_blend(1, 0, 1); }
                }
            }
            3 => {
                let (wrapped, on0) = self.popup_advance();
                if wrapped && on0 {
                    if let Some(p) = self.popup.as_mut() { p.anim.speed = -1.0; }
                    self.buy = 4;
                }
            }
            4 => {
                let (wrapped, _) = self.popup_advance();
                if !wrapped { return; }
                let s = if self.confirm { self.purchase(e, gs, items, session, assets, rng, out) } else { sound::SELECT };
                out.sounds.push(s);
                self.buy = 0;
                self.popup = None;
            }
            _ => {}
        }
    }

    /// State 4 of the buy flow with ✕: the purchase; returns the class sound.
    #[allow(clippy::too_many_arguments)]
    fn purchase(&mut self, e: Entry, gs: &mut GameState, items: &ItemTables, session: &mut SessionState, assets: &MenuAssets, rng: &mut Rng, out: &mut VendorOut) -> u8 {
        let bolts = gs.global.bolts;
        let max = self.tables.shop.max_ammo(e.item) as i32;
        let cost = if e.ammo { self.ammo_unit(e.item) * self.qty } else { self.price(gs, e.item) };
        if bolts < cost && max <= gs.global.ammo[e.item] { return sound::SELECT; }
        let salesman_class = self.tables.classes.salesman.clone();
        if e.ammo {
            if gs.global.ammo[e.item] >= max { return sound::SELECT; }
            let unit = self.ammo_unit(e.item);
            if bolts < unit * self.qty {
                self.set_ticker(assets.msg(msg::CANT_AFFORD_TICKER));
                return sound::SELECT;
            }
            let a = &mut gs.global.ammo[e.item];
            if *a > max { *a = max; }
            // AddAmmo 0x2494d8: the overflow above the max comes back.
            *a += self.qty;
            let over = if max != 0 && *a > max { let o = *a - max; *a = max; o } else { 0 };
            let bought = self.qty - over;
            gs.global.ammo_bought[e.item] += bought;
            if let (Some(s), Some(c)) = (self.salesman.as_mut(), salesman_class.as_ref()) { s.request_at(c, 3, rng, self.vsync); }
            gs.global.bolts -= unit * bought;
            out.purchase = Some((e.item, true, unit * bought, bought));
        } else {
            let price = self.price(gs, e.item);
            if bolts < price {
                self.set_ticker(assets.msg(msg::CANT_AFFORD_TICKER));
                return sound::SELECT;
            }
            if let (Some(s), Some(c)) = (self.salesman.as_mut(), salesman_class.as_ref()) { s.request_at(c, 2, rng, self.vsync); }
            gs.give_item(e.item, true, items, session);
            // 0x2af7e8: buying the Drone Device launches its drones (0x141345 = 1).
            if e.item == 0x18 { session.drone = true; }
            self.bought = e.item as i32;
            gs.global.bolts -= price;
            out.purchase = Some((e.item, false, price, 1));
        }
        self.items = build_list(gs, &self.tables.shop, self.remote);
        if self.sel + 1 > self.items.len() { self.sel = self.items.len().saturating_sub(1); }
        out.selection_changed = true;
        self.set_ticker(assets.msg(msg::THANK_YOU));
        sound::PURCHASE
    }

    /// The ticker's scroll (0x2b1a48, in its draw): +2 per frame; when the text has scrolled out (`len − scroll/20 <
    /// 1`) a random idle line (`randi(24)` of 0x1ca538).
    fn ticker_step(&mut self, rng: &mut Rng, assets: &MenuAssets) {
        let len = self.ticker.len() as i32;
        if len - self.ticker_scroll / 0x14 < 1 {
            let k = rng.randi(24) as usize;
            let id = self.tables.ticker.get(k).copied().unwrap_or(0);
            let t = assets.msg(id).to_vec();
            self.set_ticker(&t);
        } else {
            self.ticker_scroll += 2;
        }
    }

    /// The menu render's screen pass for this frame, in the game's order (`VendorDrawScreens`, screens 0..5): the
    /// ticker's scroll, then per screen its static (`FUN_002b2cd8(w, h, s)` with `sizes[s]` = the drawn size), and
    /// the popup's (screen 6) while buying. Returns the static draws per screen (target pixels). Called once per
    /// frame on which the screens are drawn ([`Vendor::screens_shown`]); it draws from the game's stream.
    pub fn render_screens(&mut self, sizes: &[(f32, f32); 7], rng: &mut Rng, assets: &MenuAssets) -> [Vec<screens::FxDraw>; 7] {
        let mut out: [Vec<screens::FxDraw>; 7] = Default::default();
        if !self.rendered {
            // DrawWorld_Mode5's first menu frame: the snapshot, the screen mobys, the read of the salesman's data.
            self.rendered = true;
            if self.salesman.is_none() && self.tables.classes.salesman.is_some() { self.salesman_loading = true; }
        }
        for (s, o) in out.iter_mut().enumerate().take(6) {
            if s == screens::TICKER { self.ticker_step(rng, assets); }
            *o = self.statics.step(sizes[s].0, sizes[s].1, s, rng);
        }
        if self.buy != 0 { out[screens::POPUP] = self.statics.step(sizes[6].0, sizes[6].1, screens::POPUP, rng); }
        out
    }
}

/// Flag 0x13d4e3 (the discount; identity [L]): `GameState::global.owned[35]`'s neighbour byte in the owned table
/// 0x13d4c0 + 0x23.
fn discount(gs: &GameState) -> bool { gs.global.owned.get(0x23).is_some_and(|&b| b != 0) }

const WHITE: u32 = 0x80f0_f0f0;

fn small_width(a: &MenuAssets, t: &[u8]) -> i32 { a.width(Font::Small, t) }

/// `format_scaled_display_value` 0x2b1bb0: "%d" below 1000, else "%d,%03d".
pub fn price_text(v: i32) -> Vec<u8> {
    if v < 1000 { format!("{v}").into_bytes() } else { format!("{},{:03}", v / 1000, v % 1000).into_bytes() }
}

impl Vendor {
    /// The full-screen fades of this frame: `FadeToBlack(4)` then the 0x15f3fc fade.
    pub fn fades(&self, out: &mut Vec<MenuDraw>) {
        if self.pre_fade > 0 {
            // FadeToBlack(4): black 0x80 − i·0x80/4 over the still frame.
            let k = 4 - self.pre_fade;
            out.push(MenuDraw::Darken { alpha: 0x80 - (3 - k) * 0x80 / 4 });
            return;
        }
        if self.fade > 0.0 { out.push(MenuDraw::Darken { alpha: (self.fade * 128.0) as i32 }); }
    }

    /// Screen `s`'s content in target pixels (the draw functions of the jump table 0x20a570 without their clear:
    /// the target is black), `size` = the screen's (W, H) (the button window centres its text in it). The 3D parts
    /// (the item model, the salesman) are the engine's.
    pub fn screen_content(&self, s: usize, size: (i32, i32), a: &MenuAssets, gs: &GameState, vsync: u32) -> Vec<MenuDraw> {
        let mut out = Vec::new();
        match s {
            screens::TICKER => self.draw_ticker(a, vsync, &mut out),
            screens::ITEM => self.draw_item(a, gs, &mut out),
            screens::SALESMAN => {}
            screens::BUTTONS => self.draw_buttons(a, gs, size, &mut out),
            screens::PROMPT => self.draw_prompt(a, gs, &mut out),
            screens::STRIP => self.draw_strip(a, &mut out),
            screens::POPUP => self.draw_popup(a, gs, size, &mut out),
            _ => {}
        }
        out
    }

    /// `VendorDrawTicker` 0x2b1a48 with the LED text `fun_00238310(2.0, text, −scroll, 8)`: characters 0x20..0x5a,
    /// cells 9×9 drawn 18×18 by `DrawTexturedQuad(x, 8, 18, 18, u, v, 9, 9, 0x80404040, GetFrameTex(icon 0xe935 / 0))`
    /// (RGBA from the disassembly, level01 0x2b19b4: half brightness, which leaves the font's dim rows and columns
    /// between the lit LEDs dark), 'b' makes the next glyph blink (hidden 10 of every 40 frames), glyphs drawn while
    /// −9 < x < 256; black bars at x 0..4 and 226..230.
    fn draw_ticker(&self, a: &MenuAssets, vsync: u32, out: &mut Vec<MenuDraw>) {
        let frame = a.frame(LED_ICON, 0);
        let mut x = -self.ticker_scroll - 4;
        let mut blink = false;
        for &c in &self.ticker {
            let k = c as i32 - 0x20;
            if k == 0x42 { blink = true; }
            if !(0..0x3b).contains(&k) { continue; }
            let cell = self.tables.led_cell.get(k as usize).copied().unwrap_or(-1);
            if cell == -1 && k != 0 { continue; }
            let hidden = std::mem::take(&mut blink) && vsync % 40 < 10;
            if k != 0 && !hidden && x > -9 && x < 0x100 {
                let (u, v) = (((cell & 0xffff) >> 4), ((cell as u32) >> 20) as i32);
                out.push(MenuDraw::FrameQuad { frame, x, y: 8, w: 18, h: 18, u, v, tw: 9, th: 9, rgba: LED_RGBA });
            }
            x += self.tables.led_adv.get(k as usize).copied().unwrap_or(10) * 2;
        }
        // FUN_00223470(0, 0, 4, 0x40) / (0xe2, 0, 0xe6, 0x40): opaque black fills (no blending).
        out.push(MenuDraw::Rect { x0: 1, y0: 1, x1: 5, y1: 0x41, rgba: 0x8000_0000 });
        out.push(MenuDraw::Rect { x0: 0xe3, y0: 1, x1: 0xe7, y1: 0x41, rgba: 0x8000_0000 });
    }

    /// `VendorDrawItemPanel` 0x2b1f08: the item model and the backdrop are the engine's (3D), then the text.
    fn draw_item(&self, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) {
        let Some(e) = self.current() else { return };
        let name = a.msg(self.tables.name.get(e.item).copied().unwrap_or(0)).to_vec();
        text(out, Font::Small, 6, 8, WHITE, &name);
        let strike = |out: &mut Vec<MenuDraw>, w: i32| {
            // fun_00200e08: seven 1-pixel lines, alpha 0x20..0x80 (colour 0x959544).
            for k in 0..7 {
                out.push(MenuDraw::Rect { x0: 0x75 + k - w, y0: 0x6d, x1: 0x7b - k, y1: 0x70, rgba: ((0x20 + 0x10 * k) as u32) << 24 | 0x95_9544 });
            }
        };
        if e.ammo {
            text(out, Font::Small, 0x18, 0x18, WHITE, a.msg(msg::AMMO));
            let t = price_text(self.tables.shop.ammo_price(e.item) as i32);
            text(out, Font::Small, 0x76 - small_width(a, &t), 0x65, WHITE, &t);
            if self.remote {
                strike(out, small_width(a, &t));
                let t = price_text(self.ammo_unit(e.item));
                text(out, Font::Small, 0x76 - small_width(a, &t), 0x55, WHITE, &t);
            }
        } else {
            let t = price_text(self.tables.shop.price(e.item));
            text(out, Font::Small, 0x76 - small_width(a, &t), 0x65, if discount(gs) { 0x8080_8080 } else { WHITE }, &t);
            if discount(gs) {
                strike(out, small_width(a, &t));
                let t = price_text(self.tables.shop.discounted(e.item));
                text(out, Font::Small, 0x76 - small_width(a, &t), 0x55, WHITE, &t);
            }
        }
    }

    /// The carousel's part of `VendorDrawIconStrip` 0x2b1c10 (≥ 8 entries; the game does it in the draw, once a frame):
    /// below 0 the offset + 4, above 0 − 4; at 0 the selection frame is drawn and the offset stays.
    pub(crate) fn ease_strip(&mut self) {
        if self.items.len() < 8 { return; }
        self.strip_settled = self.scroll == 0;
        if self.scroll < 0 { self.scroll += 4; } else if self.scroll > 0 { self.scroll -= 4; }
    }

    /// `VendorDrawIconStrip` 0x2b1c10.
    fn draw_strip(&self, a: &MenuAssets, out: &mut Vec<MenuDraw>) {
        let n = self.items.len() as i32;
        let pulse = ((self.t & 15) * 4 - 32).max(0) + 0x40;
        let sel_rgba = (pulse as u32).wrapping_mul(0x0001_0202) | 0x8000_0000;
        let icon = |e: &Entry| a.frame(self.tables.icon.get(e.item).copied().unwrap_or(0), if e.ammo { 2 } else { 0 });
        // FUN_00223470(x0, y0, x1, y1): an opaque fill of pixels x0..x1−1 (the Rect's corner offset is pixel − 1).
        let fill = |out: &mut Vec<MenuDraw>, x0: i32, y0: i32, x1: i32, y1: i32, rgba: u32| out.push(MenuDraw::Rect { x0: x0 + 1, y0: y0 + 1, x1: x1 + 1, y1: y1 + 1, rgba });
        if n < 8 {
            let x = self.sel as i32 * 0x38;
            fill(out, x + 8, 2, x + 0x40, 0x3a, sel_rgba);
            fill(out, x + 10, 4, x + 0x3e, 0x38, 0x8000_0000);
            for (i, e) in self.items.iter().enumerate() { sprite(out, icon(e), 12 + 0x38 * i as i32, 6, 0x30, 0x30, 0x80); }
        } else {
            if self.strip_settled {
                fill(out, 0xb0, 2, 0xe8, 0x3a, sel_rgba);
                fill(out, 0xb2, 4, 0xe6, 0x38, 0x8000_0000);
            }
            let mut x = self.scroll - 100;
            for k in -2..9i32 {
                let i = ((2 * n + k + self.sel as i32 - 3) % n) as usize;
                sprite(out, icon(&self.items[i]), x, 6, 0x30, 0x30, 0x80);
                x += 0x38;
            }
        }
    }

    /// `VendorDrawPrompt` 0x2b2688: small, centred at (40, 20).
    fn draw_prompt(&self, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) {
        let Some(e) = self.current() else { return };
        let id = if self.buy == 0 {
            Some(msg::EXIT)
        } else if e.ammo {
            (!self.full(gs, e.item) && self.ammo_unit(e.item) <= gs.global.bolts).then_some(msg::EXIT)
        } else {
            (self.price(gs, e.item) <= gs.global.bolts).then_some(msg::NO)
        };
        if let Some(id) = id {
            let t = a.msg(id);
            text(out, Font::Small, 0x28 - small_width(a, t) / 2, 0x14, WHITE, t);
        }
    }

    /// `VendorDrawButtonWindow(vendor, w, h)` 0x2b2430: centred in the screen [M: the window fields are lost in the
    /// decompile; the text is centred in (w, h)].
    fn draw_buttons(&self, a: &MenuAssets, gs: &GameState, size: (i32, i32), out: &mut Vec<MenuDraw>) {
        let Some(e) = self.current() else { return };
        let bolts = gs.global.bolts;
        let id = if self.buy == 0 {
            msg::BUY
        } else if e.ammo && self.full(gs, e.item) {
            msg::BACK
        } else if e.ammo {
            if self.ammo_unit(e.item) <= bolts { msg::BUY } else { msg::BACK }
        } else if self.price(gs, e.item) <= bolts {
            msg::YES
        } else {
            msg::BACK
        };
        let t = a.msg(id);
        text(out, Font::Small, (size.0 - small_width(a, t)) / 2, size.1 / 2 - 7, WHITE, t);
    }

    /// The popup's text `FUN_002b2848(popup, w, h)` (centred in the popup's rectangle `size`).
    fn draw_popup(&self, a: &MenuAssets, gs: &GameState, size: (i32, i32), out: &mut Vec<MenuDraw>) {
        let Some(e) = self.current() else { return };
        let w = size.0;
        let centre = |out: &mut Vec<MenuDraw>, id: i32, y: i32| {
            let t = a.msg(id);
            text(out, Font::Small, (w - small_width(a, t)) / 2, y, WHITE, t);
        };
        let bolts = gs.global.bolts;
        if e.ammo && self.full(gs, e.item) {
            centre(out, msg::MAXED, 24);
        } else if !e.ammo {
            if self.price(gs, e.item) <= bolts {
                text(out, Font::Small, 6, 6, WHITE, a.msg(msg::PURCHASE));
                text(out, Font::Small, 2, 0x20, WHITE, a.msg(msg::YES));
                text(out, Font::Small, 2, 0x3e, WHITE, a.msg(msg::NO));
            } else {
                centre(out, msg::CANT_AFFORD, 24);
            }
        } else if bolts < self.ammo_unit(e.item) {
            centre(out, msg::CANT_AFFORD, 24);
        } else {
            centre(out, msg::HOW_MANY, 6);
            let q = format!("{}", self.qty).into_bytes();
            let qx = if self.qty >= 100 { 23 } else if self.qty >= 10 { 30 } else { 37 };
            text(out, Font::Small, qx, 50, WHITE, &q);
            text(out, Font::Small, 12, 50, WHITE, b"<");
            text(out, Font::Small, 64, 50, WHITE, b">");
            let total = format!("{}", self.qty * self.ammo_unit(e.item)).into_bytes();
            text(out, Font::Small, 64 - small_width(a, &total), 76, WHITE, &total);
        }
    }
}

/// A moby of the vendor screen, placed in the world: its class, position and rotation rows (the engine applies the
/// class scale), its animation (None: the class's first frames, frozen) and light set 14 with this ambient.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenMoby {
    pub class: i16,
    pub position: [f32; 3],
    pub rows: [[f32; 3]; 3],
    pub anim: Option<(AnimState, Option<MobyFrame>)>,
    pub ambient: u32,
}

/// Where the screens' mobys are this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VendorScene {
    /// The item panel's target: the item model (+0x100) and the backdrop (+0).
    pub item_panel: Vec<ScreenMoby>,
    /// The salesman's target.
    pub salesman: Option<ScreenMoby>,
    /// Drawn in the world before the vendor: the hologram above the pad (+0x200 ammo / +0x300 weapon).
    pub hologram: Option<ScreenMoby>,
    /// The popup (+0x400) while buying.
    pub popup: Option<ScreenMoby>,
    /// The popup's target: the spinning class-13 moby +0x500 behind "How many?" (`FUN_002b2848`'s `DrawMobyList`, only
    /// for an ammo entry not full whose unit price the bolts cover).
    pub popup_panel: Option<ScreenMoby>,
}

/// `EulerToMatrix` (x, y, z): rows of R = Rz·Ry·Rx (row i = image of axis i).
pub fn euler_rows(e: [f32; 3]) -> [[f32; 3]; 3] {
    let (sx, cx) = e[0].sin_cos();
    let (sy, cy) = e[1].sin_cos();
    let (sz, cz) = e[2].sin_cos();
    [
        [cy * cz, cy * sz, -sy],
        [sx * sy * cz - cx * sz, sx * sy * sz + cx * cz, sx * cy],
        [cx * sy * cz + sx * sz, cx * sy * sz - sx * cz, cx * cy],
    ]
}

/// `rows · v + p` (MatrixMulVec3 + VecAdd).
pub fn to_world(rows: &[[f32; 4]; 4], pos: [f32; 3], v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|k| pos[k] + v[0] * rows[0][k] + v[1] * rows[1][k] + v[2] * rows[2][k])
}

impl Vendor {
    /// The screen mobys for the vendor at `pos` with rows `rows` and Euler `euler` (`FUN_002af3f0` and the
    /// substate-1 update). `hologram_class` = the ammo hologram class of the entry (0x1c94a0).
    pub fn scene(&self, pos: [f32; 3], rows: &[[f32; 4]; 4], euler: [f32; 3], gs: &GameState) -> VendorScene {
        let mut sc = VendorScene::default();
        let Some(l) = self.tables.layout.as_ref() else { return sc };
        let add3 = |a: [f32; 3], b: [f32; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
        let lit = 0x0020_2020;
        if let Some(s) = self.salesman.as_ref() {
            sc.salesman = Some(ScreenMoby {
                class: salesman::CLASS,
                position: to_world(rows, pos, l.salesman),
                rows: euler_rows([0.0, 0.0, euler[2]]),
                anim: Some((s.anim, s.snapshot.clone())),
                ambient: lit,
            });
        }
        if let Some(e) = self.current() {
            let item = e.item;
            // +0x100: the item's class (0x1df for the Drone Device 24), frozen on its first frames.
            let class = if item == 0x18 { 0x1df } else { self.tables.class.get(item).copied().unwrap_or(-1) };
            if let Some(p) = l.panel.get(item) {
                let mut off = add3(l.item_base, p.offset);
                if !e.ammo {
                    off[2] += p.weapon_dz;
                    if discount(gs) { off[2] += p.weapon_dz; }
                }
                if class > 0 {
                    sc.item_panel.push(ScreenMoby {
                        class,
                        position: to_world(rows, pos, off),
                        rows: euler_rows([p.rot[0], p.rot[1], p.rot[2] + euler[2]]),
                        anim: None,
                        ambient: lit,
                    });
                }
            }
            sc.item_panel.push(ScreenMoby {
                class: BACKDROP_CLASS,
                position: to_world(rows, pos, l.spin[0]),
                rows: euler_rows(self.backdrop[0]),
                anim: None,
                ambient: lit,
            });
            // The hologram: bob sin(φ)/20, yaw φ (not the vendor's); none for the PDA's remote vendor (`DrawWorld_Mode5`).
            let bob = [0.0, 0.0, self.spin.sin() / 20.0];
            let base = add3(l.holo_base, bob);
            if self.remote {
            } else if e.ammo {
                if let Some(h) = l.ammo_holo.get(item).filter(|h| h.class != 0) {
                    sc.hologram = Some(ScreenMoby {
                        class: h.class as i16,
                        position: to_world(rows, pos, add3(base, h.offset)),
                        rows: euler_rows([h.rot[0], h.rot[1], self.spin]),
                        anim: None,
                        ambient: lit,
                    });
                }
            } else if let Some(h) = l.weapon_holo.get(item) {
                if class > 0 {
                    let r = euler_rows([h.rot[0], h.rot[1], self.spin]);
                    let p0 = to_world(rows, pos, add3(base, h.offset));
                    // The post offset turned by the hologram's own rows.
                    let q: [f32; 3] = std::array::from_fn(|k| h.post[0] * r[0][k] + h.post[1] * r[1][k] + h.post[2] * r[2][k]);
                    sc.hologram = Some(ScreenMoby { class, position: add3(p0, q), rows: r, anim: None, ambient: lit });
                }
            }
        }
        if let Some(p) = self.popup.as_ref() {
            sc.popup = Some(ScreenMoby { class: POPUP_CLASS, position: pos, rows: euler_rows(euler), anim: Some((p.anim, p.snapshot.clone())), ambient: 0x0010_1010 });
            // FUN_002b2848: the quantity case draws +0x500 (placed and turned by `VendorModeUpdate` every frame) first.
            if let Some(e) = self.current() {
                if e.ammo && !self.full(gs, e.item) && gs.global.bolts >= self.ammo_unit(e.item) {
                    sc.popup_panel = Some(ScreenMoby { class: BACKDROP_CLASS, position: to_world(rows, pos, l.spin[1]), rows: euler_rows(self.backdrop[1]), anim: None, ambient: lit });
                }
            }
        }
        sc
    }
}

#[cfg(test)]
mod tests;
