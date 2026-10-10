//! Mode 3, the page menu: `EnterMenuMode` 0x28bf50, the first tick `FUN_0028c128`, the update
//! `SceneController` 0x28c990 (transitions, widget updates, focus changes), `PageMenuClose` 0x28c6c8 and the
//! render `PageMenuDraw` 0x28d080(0); the list widget (update 0x28e600, draw 0x28ebd0), the label widget
//! (update 0x28d788, draw 0x28dd30) and the Start root page. Spec: docs/plan/menus.md §3.
//!
//! Page, widget and item records are read from the overlay ([`PageMenu::load`] follows the page tree from the
//! roots); their run-time fields (focus, cursor, highlight timers, label cross-fade, patched wiring) live in
//! the loaded copies, shared between pages exactly as the game shares the structs. Widgets are recognised by
//! their callbacks' level-01 addresses: on another level the records sit elsewhere and [`Overlay::relocated`]
//! pairs them with level 01's (the pages the code names are in [`PageMenu::addrs`], the callbacks read as labels).
//!
//! The 14 class-0x472 frame mobys ([`frame`]) are spawned, animated and freed here; their corner joints give
//! the panel rects exactly as `PageMenuDraw` projects them (the engine draws the mobys themselves).
//!
//! **The Gadgets page** (and the grids, name label and 3D model of the Weapons page it shares): [`gadgets`].
//!
//! **The Goodies unlocks, the Cheats page, the code entry `MenuInput` 0x298f80 and the end-of-game page:** [`media`].
//!
//! **Every widget callback of the page tree** (the 41 pages and 145 widgets reached from the roots on level 01) is
//! dispatched; [`PageMenu::stub_calls`] counts a callback this code does not know (none on the disc's levels). The
//! no-ops: the enters 0x28dd10 / 0x295200, the leaves 0x28dd18, 0x28f7a8 (the map file stream's break: no stream here),
//! 0x290508 (the Sound page's group-2 volume back to sfx·8/10: the port's mixer derives it every frame, G-AUD-013),
//! 0x290bf8, the stream-buffer enters / leaves (memory only). Not ported: the Gadgets page's 3D Ratchet's streamed
//! per-item animations and drones (G-UI-002).
//!
//! **Port-only:** [`port`] adds a "Port Options" page (not in the game) built from the same machinery, reached
//! from an extra entry of the Options list ([`PageMenu::install_port_page`]; the engine skips it with
//! `RC_SETTINGS_PAGE=0`, leaving every record as on the disc).

pub mod frame;
pub mod gadgets;
pub mod map_page;
pub mod media;
pub mod model_anim;
mod options;
pub mod pages;
pub mod planet_select;
pub mod port;
pub mod saves;

use super::{scale_ticks, screen_static, text, text_plain, tween, MenuAssets, MenuDraw, MenuInput, MenuSound, Overlay};
use crate::game_state::GameState;
use crate::hud::{text as wtext, Draw};
use crate::pad::button;
use rc_formats::font::Font;
use std::collections::BTreeMap;

/// Pages the code refers to by address (level01 immediates in 0x28bf50 / 0x28c128 / 0x28c990 / updates).
pub mod page {
    pub const ROOT: u32 = 0x1b2a08;
    pub const MAP: u32 = 0x1b3998;
    pub const MAP_MISSIONS: u32 = 0x1b3bf8;
    pub const PLANET_SELECT: u32 = 0x1b6508;
    pub const PLANET_CONFIRM: u32 = 0x1b6878;
    pub const KIND22: u32 = 0x1b7670;
    pub const KIND23: u32 = 0x1b6fb8;
    pub const KIND2D: u32 = 0x1b8b48;
    /// Self-transitions to these play forward (0x28c990).
    pub const FORWARD_ON_SELF: [u32; 6] = [0x1b7670, 0x1b8250, 0x1b8560, 0x1b6878, 0x1b7d70, 0x1b6ca0];
    /// The pages the code forms (the page walk's starting points on another level, `menus::Overlay::relocated`).
    pub const ROOTS: [u32; 20] = [
        ROOT, MAP, MAP_MISSIONS, PLANET_SELECT, PLANET_CONFIRM, KIND22, KIND23, KIND2D, 0x1b8250, 0x1b8560, 0x1b7d70, 0x1b6ca0,
        // The memory-card pages (`saves::page::ROOTS`: the confirm pages are named by code only).
        0x1b5b48, 0x1b5bd0, 0x1b6af8, 0x1b9208, 0x1b93b8, 0x1b9180, 0x1b9440, 0x1b9518,
    ];
}

/// `0x1c23a8`: the galaxy-map point table the planet map forms (16 bytes per level; the map reads level 1.. at +0x10).
pub const PLANET_POINTS_BASE: u32 = 0x1c23a8;

/// This level's addresses of the pages and widgets the code names by address ([`page`], [`wiring`] and the
/// records [`port`] attaches to; level-01 labels resolved through [`Overlay::at`]). `Default`: level 01.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Addrs {
    pub root: u32,
    pub map: u32,
    pub map_missions: u32,
    pub planet_select: u32,
    pub planet_confirm: u32,
    pub kind22: u32,
    pub kind23: u32,
    pub kind2d: u32,
    pub forward_on_self: [u32; 6],
    pub weapons: u32,
    pub options: u32,
    pub goodies: u32,
    pub port_options: u32,
    pub port_options_list: u32,
    pub port_options_label: u32,
    pub port_quit: u32,
    pub port_model_few: u32,
    pub port_model_many: u32,
    /// The Help / Weapons and Help / Gadgets label tables (`pages::data::WEAPON_TEXTS`, `GADGET_TEXTS_A` / `_B`).
    pub help_texts: [u32; 3],
    /// The Moves label's tables (Heli-Pack owned / not: `pages::data::MOVES_HELI`, `MOVES_NO_HELI`).
    pub moves_tables: [u32; 2],
    /// The memory-card pages the code names (`saves::page::ROOTS`).
    pub saves_pages: [u32; 8],
    /// The missions page's description label 0x1b3dc0 and picture widget 0x1b3c80 (the missions widget writes their
    /// +0x34 id 0x1b3df4 and +0x58 index 0x1b3cd8).
    pub missions_label: u32,
    pub missions_image: u32,
}

impl Addrs {
    pub fn resolve(ov: &Overlay) -> Addrs {
        let a = |l: u32| ov.at(l);
        Addrs {
            root: a(page::ROOT),
            map: a(page::MAP),
            map_missions: a(page::MAP_MISSIONS),
            planet_select: a(page::PLANET_SELECT),
            planet_confirm: a(page::PLANET_CONFIRM),
            kind22: a(page::KIND22),
            kind23: a(page::KIND23),
            kind2d: a(page::KIND2D),
            forward_on_self: page::FORWARD_ON_SELF.map(a),
            weapons: a(wiring::WEAPONS),
            options: a(wiring::OPTIONS),
            goodies: a(wiring::GOODIES),
            port_options: a(port::OPTIONS),
            port_options_list: a(port::OPTIONS_LIST),
            port_options_label: a(port::OPTIONS_LABEL),
            port_quit: a(port::QUIT_PAGE),
            port_model_few: a(port::MODEL_FEW),
            port_model_many: a(port::MODEL_MANY),
            help_texts: [a(pages::data::WEAPON_TEXTS), a(pages::data::GADGET_TEXTS_A), a(pages::data::GADGET_TEXTS_B)],
            moves_tables: [a(pages::data::MOVES_HELI), a(pages::data::MOVES_NO_HELI)],
            saves_pages: saves::page::ROOTS.map(a),
            missions_label: a(wiring::MISSIONS_LABEL),
            missions_image: a(wiring::MISSIONS_IMAGE),
        }
    }
}

impl Default for Addrs {
    fn default() -> Addrs { Addrs::resolve(&Overlay::default()) }
}

/// The level-01 menu tree walked in step with another level's from `roots` (label, this level's address): the
/// same records in the same shape on every level. Returns (label → this level's address) of every page, widget
/// and item list reached, and (this level's callback → its label) of every widget callback. A pair whose page
/// kinds or widget counts differ is not followed (the trees are the same on the 19 levels).
pub fn correlate(reference: &Overlay, target: &Overlay, roots: &[(u32, u32)]) -> (std::collections::HashMap<u32, u32>, std::collections::HashMap<u32, u32>) {
    let mut at = std::collections::HashMap::new();
    let mut label = std::collections::HashMap::new();
    let mut pages: Vec<(u32, u32)> = roots.to_vec();
    let mut widgets: Vec<(u32, u32)> = Vec::new();
    loop {
        if let Some((r, t)) = pages.pop() {
            if r == 0 || t == 0 || at.contains_key(&r) { continue; }
            let (Some(rp), Some(tp)) = (read_page(reference, r), read_page(target, t)) else { continue };
            if rp.kind != tp.kind || rp.widgets.map(|w| w != 0) != tp.widgets.map(|w| w != 0) { continue; }
            at.insert(r, t);
            pages.push((rp.parent, tp.parent));
            widgets.extend(rp.widgets.iter().zip(&tp.widgets).map(|(&a, &b)| (a, b)));
            continue;
        }
        let Some((r, t)) = widgets.pop() else { break };
        if r == 0 || t == 0 || at.contains_key(&r) { continue; }
        let u = |ov: &Overlay, a: u32, o: u32| ov.u32(a + o).unwrap_or(0);
        at.insert(r, t);
        for o in [0, 4, 8, 0xc] {
            let (cr, ct) = (u(reference, r, o), u(target, t, o));
            if cr != 0 && ct != 0 { label.insert(ct, cr); }
        }
        let (upd, draw) = (u(reference, r, 0), u(reference, r, 4));
        if upd == func::LIST_UPDATE || (upd == 0 && draw == func::LIST_DRAW) {
            let (ri, ti) = (read_items(reference, u(reference, r, 0x34)), read_items(target, u(target, t, 0x34)));
            if ri.len() == ti.len() {
                at.insert(u(reference, r, 0x34), u(target, t, 0x34));
                // Action 3 pages and the locked Goodies entries' (−1 until `0x29aa60`).
                pages.extend(ri.iter().zip(&ti).filter(|(a, b)| a.action == b.action && (a.action == 3 || a.action == -1)).map(|(a, b)| (a.arg, b.arg)));
                // Save / Load / New Game / Load Game (actions 4 / 5) open their pages too.
                pages.extend(ri.iter().zip(&ti).filter(|(a, b)| a.action == b.action && (a.action == 4 || a.action == 5)).map(|(a, b)| (a.arg, b.arg)));
            }
            widgets.push((u(reference, r, 0x38), u(target, t, 0x38)));
            widgets.push((u(reference, r, 0x3c), u(target, t, 0x3c)));
        }
    }
    (at, label)
}

/// The root wiring `EnterMenuMode` patches (0x1b2b70 = Weapons.above, 0x1b2d04 = Options.below).
pub mod wiring {
    pub const WEAPONS: u32 = 0x1b2b38;
    pub const OPTIONS: u32 = 0x1b2cc8;
    pub const GOODIES: u32 = 0x1b2d18;
    /// The missions page's description label and picture widget (`map_page::missions_enter`).
    pub const MISSIONS_LABEL: u32 = 0x1b3dc0;
    pub const MISSIONS_IMAGE: u32 = 0x1b3c80;
}

/// Widget callbacks by address (level01).
pub mod func {
    pub const LIST_UPDATE: u32 = 0x28e600;
    pub const LIST_DRAW: u32 = 0x28ebd0;
    pub const LABEL_UPDATE: u32 = 0x28d788;
    pub const LABEL_DRAW: u32 = 0x28dd30;
    pub const LABEL_ENTER: u32 = 0x28dd20;
    pub const ROOT_ENTER: u32 = 0x2917d8;
    pub const TOGGLE_UPDATE: u32 = 0x294830;
    pub const TOGGLE_DRAW: u32 = 0x294a38;
    pub const CAMERA_UPDATE: u32 = 0x294cc0;
    pub const CAMERA_DRAW: u32 = 0x294e68;
    pub const SOUND_UPDATE: u32 = 0x290538;
    pub const SOUND_DRAW: u32 = 0x290808;
    pub const SOUND_LEAVE: u32 = 0x290508;
    pub const QUIT_UPDATE: u32 = 0x2921d0;
    pub const QUIT_DRAW: u32 = 0x292298;
    pub const PLANET_LIST_DRAW: u32 = 0x290eb0;
    pub const PLANET_LIST_ENTER: u32 = 0x295208;
    pub const GALAXY_UPDATE: u32 = 0x295310;
    pub const GALAXY_DRAW: u32 = 0x295338;
    pub const MAP_UPDATE: u32 = 0x28f868;
    /// The map widget's draw 0x292d38 (→ `UNK_NoMapAvailable` 0x25b1c0) and the map page's legend 0x292d70.
    pub const MAP_DRAW: u32 = 0x292d38;
    pub const MAP_LEGEND_DRAW: u32 = 0x292d70;
    /// The globe: draw 0x294258, enter 0x2904d0.
    pub const GLOBE_DRAW: u32 = 0x294258;
    pub const GLOBE_ENTER: u32 = 0x2904d0;
    pub const CONFIRM_UPDATE: u32 = 0x295370;
    pub const MISSIONS_UPDATE: u32 = 0x28fec8;
    /// The missions widget's draw `DrawMissionsMenu2` and enter `fun_0021c420`.
    pub const MISSIONS_DRAW: u32 = 0x293090;
    pub const MISSIONS_ENTER: u32 = 0x28fe28;
    /// The streamed picture of the missions page / the planet select (`fun_0021f990`, `draw_transition_overlay`).
    pub const PICTURE_UPDATE: u32 = 0x293398;
    pub const PICTURE_DRAW: u32 = 0x293670;
    /// The map widgets' leave (`fun_0021bda0`: the map file stream's break).
    pub const MAP_LEAVE: u32 = 0x28f7a8;
    /// The confirm page's gold-bolt panel `DrawGBsShipMenu`.
    pub const SHIP_GOLD_DRAW: u32 = 0x292980;
}

/// Colours (`*0x160270` navy; the list tween ends from 0x28f0e0; disabled colours of 0x28ebd0).
pub const NAVY: u32 = 0x8010_0808;
pub const LIGHT_BLUE: u32 = 0x80ff_a888;
pub const YELLOW: u32 = 0x8020_ffff;
pub const DISABLED: u32 = 0x8030_3030;
pub const DISABLED_SELECTED: u32 = 0x8000_6060;
pub const SHADOW: u32 = 0x8000_0000;
/// Darkening over the snapshot (menu), and under a freeze dialog.
pub const DARKEN: i32 = 0x30;
pub const DARKEN_FREEZE: i32 = 0x40;
/// `*0x160274`: highlight tween length (ScaleTicks); `*0x160278/7c`: shadow offset; `*0x160318`: label margin;
/// `*0x160328`: label line height. Read from the overlay by [`MenuConsts::load`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuConsts {
    pub hl_ticks: i32,
    pub shadow: (i32, i32),
    pub margin: i32,
    pub line_height: i32,
    pub navy: u32,
}

impl MenuConsts {
    pub fn load(ov: &Overlay) -> Option<MenuConsts> {
        Some(MenuConsts {
            navy: ov.u32(ov.at(0x160270))?,
            hl_ticks: ov.i32(ov.at(0x160274))?,
            shadow: (ov.i32(ov.at(0x160278))?, ov.i32(ov.at(0x16027c))?),
            margin: ov.i32(ov.at(0x160318))?,
            line_height: ov.i32(ov.at(0x160328))?,
        })
    }
}

impl Default for MenuConsts {
    /// Level-01 values (tests); the engine uses [`MenuConsts::load`].
    fn default() -> Self { MenuConsts { hl_ticks: 10, shadow: (2, 2), margin: 4, line_height: 16, navy: NAVY } }
}

/// A page record (0x84 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub addr: u32,
    pub seqs: [i32; 14],
    pub parent: u32,
    pub kind: i32,
    /// +0x40 (run time).
    pub focus: u32,
    pub widgets: [u32; 14],
    /// +0x80 (run time): the focus change requested by a list's Up/Down.
    pub pending: u32,
}

/// A list item (12 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Item {
    pub label: i16,
    pub action: i16,
    pub arg: u32,
    pub sublabel: i16,
    /// +10: highlight timer.
    pub hl: i16,
}

/// List flags (+0x30).
pub mod lf {
    pub const BUTTONS_ONLY: u32 = 1;
    pub const FIXED_COLOUR: u32 = 2;
    pub const LARGE: u32 = 4;
    pub const SMALL: u32 = 8;
    pub const STEP_FROM_FONT: u32 = 0x10;
    pub const PLANET_CURSOR: u32 = 0x20;
    pub const LEFT: u32 = 0x40;
    pub const NO_CODES: u32 = 0x80;
    pub const SHOULDERS: u32 = 0x100;
    pub const WRAP: u32 = 0x1000;
    pub const COMMON_CENTRE: u32 = 0x4000;
    pub const JUMP_SCROLL: u32 = 0x8000;
    pub const AUTO_SMALL: u32 = 0x20000;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    pub flags: u32,
    pub items_addr: u32,
    pub items: Vec<Item>,
    pub above: u32,
    pub below: u32,
    pub cursor: i32,
    /// +0x44: the planet list's scroll.
    pub scroll: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub flags: u32,
    /// +0x34: fixed id (flag 4) or id table; +0x38: table stride.
    pub id: u32,
    pub stride: u32,
    /// +0x3c: auto-scroll (1/16 px), +0x44: cross-fade timer, +0x48 / +0x4c: shown content and variant.
    pub scroll: i32,
    pub timer: i32,
    pub content: i32,
    pub variant: i32,
    /// Port-only: a patched copy of the id table (indexed by content; variant 0) used instead of the overlay's
    /// when set ([`port`] inserts the description of its Options entry). None for every disc label.
    pub table: Option<Vec<u32>>,
}

/// Type data of a widget (by its callbacks).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data {
    List(List),
    Label(Label),
    Toggle(options::Toggle),
    Camera(options::Camera),
    Sound { cursor: i32 },
    Quit,
    Galaxy,
    /// 0x28f868 with its passive flag (+0x34 & 0x40).
    Map { passive: bool },
    Confirm,
    /// The missions widget (`map_page::Missions`) and the streamed picture (`map_page::Picture`).
    Missions(map_page::Missions),
    Picture(map_page::Picture),
    /// The kind-0x23 page's scrolling text (`media::Scroller`).
    Scroller(media::Scroller),
    /// Port-only: the "Port Options" list ([`port`]).
    Port(port::PortList),
    /// The icon grids, the item preview and the 3D Ratchet of the Gadgets / Weapons pages ([`gadgets`]).
    Grid(gadgets::Grid),
    Preview(gadgets::Preview),
    Model,
    /// The pause pages' other widgets ([`pages`]).
    Image(pages::Image),
    Controls { state: i32 },
    Slots { slots: [i32; 8], cursor: i32 },
    Ammo,
    AmmoModel(pages::AmmoModel),
    GoldBolt,
    IconList(pages::IconList),
    Skill,
    Other,
}

/// A widget record (+0x00..+0x2c) and its type data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Widget {
    pub addr: u32,
    pub update: u32,
    pub draw: u32,
    pub enter: u32,
    pub leave: u32,
    /// +0x10: 1 draw direct, 2 3D pass, 4 hidden.
    pub dflags: u32,
    /// +0x14 moby slot (the widget index on the page last transitioned to).
    pub moby: i32,
    /// +0x18..+0x24: x, y, w, h (written by the render from the moby rect).
    pub rect: [i32; 4],
    /// +0x30..+0x4c as on disc.
    pub raw: [u32; 8],
    pub data: Data,
}

/// Post-actions (`0x1ba17c`) the close runs after its 2 ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostAction {
    /// 1: back to gameplay.
    Resume,
    /// 2: `ShipTravelTo(0x184894)`.
    ShipTravel(i32),
    /// 3 / 4 / 6: fade 16 + movie `0x2acf50` / `0x2ad050` / `0x2acfe8` (arg `0x1ba254`).
    Movie { kind: i32, arg: u32 },
    /// 5: fade 16 + in-engine scene `0x2ac330(arg)`.
    Scene(u32),
    /// 7: fade 16 + slideshow (mode 7).
    Slideshow,
}

/// What the frame did outside the menu state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuOut {
    /// Class-0x472 sounds requested this frame, in order.
    pub sounds: Vec<MenuSound>,
    /// The menu finished closing: run this and leave mode 3.
    pub exit: Option<PostAction>,
    /// Quit Game ○: `0x13d384 = 0, 0x15f5c0 = −1, 0x15f5d8 = 1, 0x15f570 = 1`.
    pub quit: bool,
    /// `mode_freezeInit(3, page)` requested (Save / Load actions 4 / 5); not ported.
    pub freeze: Option<u32>,
    /// `PageMenuClose`: the item requests `0x141408 + 4·slot` of the slots whose menu copy changed (hand, feet,
    /// head, back; `crate::inventory::menu_close_requests`), for the session.
    pub equip: Option<[Option<i32>; 4]>,
    /// ✕ on the Sound page (stereo / mono and the mixer, 0x12e240).
    pub sound_settings: bool,
    /// The transition started / finished this frame.
    pub transition: Option<(u32, u32)>,
    pub entered: Option<u32>,
    /// `MenuInput` 0x298f80 recognised a code this frame: its effect (applied to the game state) and what is left for
    /// the engine (the banner, the skill point jingle).
    pub code: Option<(crate::cheats::CodeEffect, crate::cheats::CodeOut)>,
    /// The end-of-game page's choice ([`media::EndChoice`]).
    pub end_choice: Option<media::EndChoice>,
    /// `0x15f5c0 = dest, 0x15f570 = 1`: leave the level for `dest` (−1: Quit Game, back to the title; 0: the challenge
    /// mode's restart on Veldin), `DoSpaceTransition` after the frame (the `travel` lane's level change).
    pub level_exit: Option<i32>,
    /// `0x13e05a`: the level exit runs its story trip (1: New Game's opening) or not (0: a loaded game).
    pub story: Option<bool>,
    /// Action 9 (the front end's Language list): the language 0x15ed88 = arg.
    pub language: Option<u32>,
    /// 0x1516ec = id: a dialogue line requested (the end page's Helpdesk girl, `media::girl_update`).
    pub voice: Option<i32>,
    /// `continue_audio_stream_if_ready` 0x279e78: the loaded line starts (0x151720 ≠ 0 and 0x15172a = 3 → 4).
    pub voice_continue: bool,
    /// 0x15172a ∉ {6, 7} → 5: the line stops (the girl's leave).
    pub voice_stop: bool,
}

/// The per-frame values the menus read from outside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuEnv {
    /// `0x15f3f8`, the VSync / frame counter (it keeps running in mode 3).
    pub vsync: u32,
    /// `0x1413f4` (hero player-state byte; 1 disables Weapons / Gadgets / Quick Select).
    pub b13f4: u8,
    /// `0x15ed80` PAL.
    pub pal: bool,
}

/// The page-menu globals (`0x1ba170..`) and the loaded records.
#[derive(Clone, Debug)]
pub struct PageMenu {
    pub consts: MenuConsts,
    /// This level's addresses of the pages and widgets the code names.
    pub addrs: Addrs,
    pub pages: BTreeMap<u32, Page>,
    pub widgets: BTreeMap<u32, Widget>,
    /// 0x1ba170 kind, 0x1ba174 current page, 0x1ba178 target page, 0x1ba17c post-action, 0x1ba184
    /// transition / close ticks, 0x1ba280 menu ticks.
    pub kind: i32,
    pub current: u32,
    pub target: u32,
    pub post: i32,
    pub trans: i32,
    pub ticks: i32,
    /// 0x1ba240: the page before the running transition.
    pub prev: u32,
    /// 0x1ba248 Goodies unlocked, 0x1ba294 close keys off (kind 0x21).
    pub goodies: bool,
    pub no_close: bool,
    /// 0x1ba254 post-action argument, 0x1ba260 / 0x1ba264 return page and kind.
    pub arg: u32,
    pub return_page: u32,
    pub return_kind: i32,
    /// 0x184894: the destination / selected planet.
    pub dest: i32,
    /// 0x1ba310[14]: each frame moby's (seq, playing backwards) as last set by the menu.
    pub mobys: [(i32, bool); 14],
    /// The frame mobys themselves (None when class 0x472 is not available: the game then spawns none, and
    /// no panel is placed or drawn).
    pub frames: Option<frame::FrameMobys>,
    /// Calls into unported widget code, by name.
    pub stub_calls: BTreeMap<&'static str, u64>,
    /// The menu is active (mode 3).
    pub active: bool,
    /// `0x1413f4` as the root enter 0x2917d8 reads it (set from [`MenuEnv`] each tick).
    pub enter_b13f4: u8,
    /// Per level (0..20): location and planet name ids (`0x1c22c0 + 12·lvl`, `+4`).
    pub level_names: Vec<(u32, u32)>,
    /// Galaxy-map points `{x, y, dx, dy}` of levels 1..=19 (`0x1c23b8 + 16·(lvl−1)`) and the name offset
    /// `*0x15f650`.
    pub planet_points: Vec<[i32; 4]>,
    pub name_dy: i32,
    /// `0x1ba1a0[4]`: the saved items (hand, feet, head, back) copied at the first tick; the Gadgets / Weapons grids
    /// equip into it and the close requests the changes.
    pub equip: [i32; 4],
    /// The item definitions (the grids' slot types and classes; `crate::inventory`), given by the engine.
    pub items: Option<std::sync::Arc<crate::inventory::ItemInfos>>,
    /// `0x1ba2a4` (level 0xd or 0x14161b: the head items unusable) and `0x1ba2a8` (level 0 or 0xe: the packs).
    pub unusable_head: bool,
    pub unusable_back: bool,
    /// The 3D widgets of the last draw (the engine renders them) and the 3D Ratchet widget while it exists.
    pub view: gadgets::GadgetsView,
    pub view_model: Option<u32>,
    /// `MenuTextLoad`'s state (+0x50 of the Help Log's text widget): 0 the level's table, 1 `all_text` streaming, 2 the
    /// message table swapped to `all_text` ([`pages::text_update`]).
    pub text_swap: i32,
    /// Label id tables the page enters build (0x1ba6f8, 0x1ba7c8, 0x1ba800: [`pages::help_weapons_enter`]), by address.
    pub label_tables: BTreeMap<u32, Vec<u32>>,
    /// The items the Help / Weapons and Help / Gadgets enters walk (the Weapons page grid's 15 cells, the Gadgets page
    /// grids' 14), read at load.
    pub help_items: pages::HelpItems,
    /// The In-Level Movies lists by level (`0x1b8aa8[level]`), read at load.
    pub movie_lists: Vec<Vec<Item>>,
    /// The help log's id table 0x1798d0 (`crate::help`), read at load.
    pub log_ids: Vec<(i16, i16)>,
    /// 0x15ed88: the language (the Controls page's pictures, the Items page's German hyphen).
    pub lang: u32,
    /// The price records' `{u16 has ammo (+8), u16 max ammo (+0xe)}` by item (0x1c4530 + 0x18·id), from the engine.
    pub ammo_records: Vec<(u16, u16)>,
    /// The Items page's gold bolt moby while it exists: its turn about x (`pages::gold_enter`).
    pub gold_spin: Option<f32>,
    /// The map page (`map_page`; the live map system moved in while the menu is open).
    pub map: map_page::MapPage,
    /// The Goodies unlocks, the Cheats page, the code entry and the end-of-game page ([`media`]).
    pub media: media::MediaMenu,
    /// The memory-card pages and the card ([`saves`]).
    pub saves: saves::SaveMenu,
}

fn stub(m: &mut BTreeMap<&'static str, u64>, name: &'static str) { *m.entry(name).or_default() += 1; }

impl PageMenu {
    /// Parses every page reachable from the roots and their widgets / items.
    pub fn load(ov: &Overlay) -> Option<PageMenu> {
        let addrs = Addrs::resolve(ov);
        let mut m = PageMenu {
            consts: MenuConsts::load(ov)?,
            pages: BTreeMap::new(),
            widgets: BTreeMap::new(),
            kind: 0,
            current: 0,
            target: 0,
            post: 0,
            trans: 0,
            ticks: 0,
            prev: 0,
            goodies: false,
            no_close: false,
            arg: 0,
            return_page: 0,
            return_kind: 0,
            dest: 0,
            mobys: [(0, false); 14],
            stub_calls: BTreeMap::new(),
            active: false,
            frames: None,
            enter_b13f4: 0,
            level_names: (0..20u32).map(|l| (ov.u32(ov.at(0x1c22c0) + 12 * l).unwrap_or(0), ov.u32(ov.at(0x1c22c0) + 4 + 12 * l).unwrap_or(0))).collect(),
            planet_points: (0..19u32).map(|k| std::array::from_fn(|j| ov.i32(ov.at(PLANET_POINTS_BASE) + 0x10 + 16 * k + 4 * j as u32).unwrap_or(0))).collect(),
            name_dy: ov.i32(ov.at(0x15f650))?,
            addrs,
            equip: [0; 4],
            items: None,
            unusable_head: false,
            unusable_back: false,
            view: gadgets::GadgetsView::default(),
            view_model: None,
            text_swap: 0,
            label_tables: BTreeMap::new(),
            help_items: pages::HelpItems::read(ov),
            movie_lists: pages::read_movie_lists(ov),
            log_ids: crate::help::read_log_ids(ov),
            lang: 0,
            ammo_records: Vec::new(),
            gold_spin: None,
            map: map_page::MapPage::new(ov),
            media: media::MediaMenu::default(),
            saves: saves::SaveMenu::read(ov),
        };
        let a = &m.addrs;
        // The kinds 0x21 / 0x23 / 0x2d roots (the end-of-game page, the front end) and the end page's save page.
        let mut todo = vec![a.root, a.map, a.map_missions, a.planet_select, a.planet_confirm, a.kind22, a.kind23, a.kind2d, ov.at(media::label::SAVE_PAGE)];
        // The memory-card pages and the front end's Options pages (the list the enter 0x28dbb8 installs).
        todo.extend(a.saves_pages);
        todo.extend(m.saves.front_options.iter().flatten().filter(|it| it.action == 3).map(|it| it.arg));
        while let Some(p) = todo.pop() {
            if p == 0 || m.pages.contains_key(&p) { continue; }
            let Some(pg) = read_page(ov, p) else { continue };
            if pg.parent != 0 { todo.push(pg.parent); }
            for &w in pg.widgets.iter().filter(|&&w| w != 0) {
                if m.widgets.contains_key(&w) { continue; }
                let Some(wd) = read_widget(ov, w) else { continue };
                if let Data::List(l) = &wd.data {
                    // Action 3 pages, and the locked Goodies entries' pages (action −1 until `0x29aa60` unlocks them).
                    for it in &l.items {
                        if it.action == 3 || it.action == -1 { todo.push(it.arg); }
                        // Save / Load (actions 4 / 5).
                        if it.action == 4 || it.action == 5 { todo.push(it.arg); }
                    }
                }
                m.widgets.insert(w, wd);
            }
            m.pages.insert(p, pg);
        }
        m.media = media::MediaMenu::read(ov, &m.pages);
        m.pages.contains_key(&m.addrs.root).then_some(m)
    }

    fn page(&self, p: u32) -> Option<&Page> { self.pages.get(&p) }

    /// This level's address of a memory-card page `label` (`saves::page`).
    pub fn addrs_saves(&self, label: u32) -> u32 {
        saves::page::ROOTS.iter().position(|&l| l == label).map_or(label, |i| self.addrs.saves_pages[i])
    }
    fn w(&self, a: u32) -> Option<&Widget> { self.widgets.get(&a) }
    fn wm(&mut self, a: u32) -> Option<&mut Widget> { self.widgets.get_mut(&a) }

    /// `EnterMenuMode` 0x28bf50 (the caller sets mode 3). `hand_item` = 0x141660[0] (0x24 → 0 not modelled).
    pub fn enter(&mut self, kind: i32, gs: &GameState) {
        let g = &gs.global;
        self.goodies = g.game_beaten != 0 || g.completes != 0 || self.media.goodies_extra;
        let ad = &self.addrs;
        let (a, b) = if self.goodies { (ad.goodies, ad.goodies) } else { (ad.options, ad.weapons) };
        let (weapons, options) = (ad.weapons, ad.options);
        if let Some(Data::List(l)) = self.wm(weapons).map(|w| &mut w.data) { l.above = a; }
        if let Some(Data::List(l)) = self.wm(options).map(|w| &mut w.data) { l.below = b; }
        self.kind = kind;
        // 0x1ba24c = (kind == 0x23): the kind-0x23 page's scroller closes the menu at its end.
        self.media.kind23 = kind == 0x23;
        self.ticks = 0;
        self.post = 0;
        self.dest = if g.level < 0x13 { g.level } else { 0 };
        // FUN_00262760 (0x28c0ec): the destination's missions.
        map_page::enter(self, gs);
        // 0x1ba2a4 / 0x1ba2a8 (0x14161b is not modelled: 0).
        self.unusable_head = g.level == 0xd;
        self.unusable_back = g.level == 0 || g.level == 0xe;
        self.active = true;
        // FUN_0029aa60: the Goodies entries the skill points and gold weapons unlock.
        media::goodies_unlocks(self, gs);
    }

    /// `FUN_0028c128`, the first tick (kind → first page, the saved items copied, the moby spawn).
    fn first_tick(&mut self, gs: &GameState) {
        self.equip.copy_from_slice(&gs.global.equipped[..4]);
        self.no_close = false;
        self.prev = 0;
        self.target = match self.kind {
            10 => {
                self.kind = 0xb;
                self.addrs.map
            }
            0xe => {
                self.kind = 0xf;
                self.addrs.planet_select
            }
            0x21 => {
                self.no_close = true;
                self.kind = 0x22;
                self.addrs.kind22
            }
            0x23 => self.addrs.kind23,
            k => {
                self.kind = 2;
                if k == 0x2d { self.addrs.kind2d } else { self.addrs.root }
            }
        };
        if self.return_page != 0 {
            self.target = self.return_page;
            self.kind = self.return_kind;
            if self.return_kind == 0xb || self.return_kind == 0xf { self.dest = self.arg as i32; }
            self.return_kind = 0;
            self.return_page = 0;
        }
        self.current = self.target;
        // FUN_0025a8d0 for the map and the planet select (kinds 0xb / 0xf).
        if matches!(self.kind, 0xb | 0xf) { map_page::setup(self, gs); }
        // The 14 mobys start on the page's seqs at their last frame.
        if let Some(p) = self.page(self.current) {
            let seqs = p.seqs;
            for (m, s) in self.mobys.iter_mut().zip(seqs) { *m = (s, false); }
            if let Some(f) = self.frames.as_mut() { f.spawn(&seqs); }
        }
    }

    /// One frame of mode 3 (`SceneController` 0x28c990).
    pub fn tick(&mut self, inp: &MenuInput, gs: &mut GameState, env: &MenuEnv) -> MenuOut {
        let mut out = MenuOut::default();
        self.enter_b13f4 = env.b13f4;
        self.ticks += 1;
        if self.kind == 0x14 {
            let before = self.trans;
            self.trans -= 1;
            if before == 0 || self.trans == 0 {
                self.trans = 0;
                self.active = false;
                out.exit = Some(match self.post {
                    2 => PostAction::ShipTravel(self.dest),
                    3 | 4 | 6 => PostAction::Movie { kind: self.post, arg: self.arg },
                    5 => PostAction::Scene(self.arg),
                    7 => PostAction::Slideshow,
                    _ => PostAction::Resume,
                });
            }
            return out;
        }
        if matches!(self.kind, 0 | 10 | 0xe | 0x11 | 0x21 | 0x2d | 0x23) { self.first_tick(gs); }
        // A changed memory card (0x15eeb4 & 1) holds the menu in the card dialog (`mode_freezeInit(3, page)`).
        if self.saves.card.flags & 1 != 0 {
            out.freeze = Some(self.current);
            return out;
        }
        // MenuInput 0x298f80: the debug code entry (R2 + L1 held, 20 presses).
        let codes = std::mem::take(&mut self.media.cheats.codes);
        if let Some(e) = self.media.code.step(inp.held_u, inp.pressed_u, &codes) { out.code = Some((e, crate::cheats::apply_code(e, gs))); }
        self.media.cheats.codes = codes;
        if self.kind == 1 {
            let done = self.trans < 1;
            self.trans -= 1;
            if done { self.trans = 0; }
            if self.trans == 0 {
                let t = self.target;
                self.target = 0;
                self.current = t;
                self.kind = self.page(t).map_or(2, |p| p.kind);
                let ws = self.page(t).map(|p| p.widgets).unwrap_or([0; 14]);
                for w in ws.iter().filter(|&&w| w != 0) { self.call_enter(*w, false, gs); }
                out.entered = Some(t);
            }
        } else if self.target != 0 {
            out.sounds.push(if self.current == self.target { MenuSound::Open } else { MenuSound::PageChange });
            let cur_ws = self.page(self.current).map(|p| p.widgets).unwrap_or([0; 14]);
            for w in cur_ws.iter().filter(|&&w| w != 0) { self.call_leave(*w, &mut out, gs); }
            let parent = self.page(self.current).map_or(0, |p| p.parent);
            let mut back = self.target == parent;
            if self.current == self.target && !self.addrs.forward_on_self.contains(&self.current) && self.kind != 0x23 { back = !back; }
            let (cur_seqs, tgt) = (self.page(self.current).map(|p| p.seqs), self.page(self.target).cloned());
            if let Some(tp) = &tgt {
                for (i, &w) in tp.widgets.iter().enumerate() {
                    if let Some(wd) = self.wm(w) { wd.moby = i as i32; }
                }
            }
            for i in 0..14 {
                self.mobys[i] = if back {
                    (cur_seqs.map_or(0, |s| s[i]), true)
                } else {
                    (tgt.as_ref().map_or(0, |p| p.seqs[i]), false)
                };
                if let Some(f) = self.frames.as_mut() { f.start(i, self.mobys[i].0, back); }
            }
            out.transition = Some((self.current, self.target));
            self.kind = 1;
            self.trans = 0xc;
            self.prev = self.current;
            self.current = 0;
        }
        if self.current != 0 {
            let ws = self.page(self.current).map(|p| p.widgets).unwrap_or([0; 14]);
            for w in ws {
                if w == 0 || self.w(w).is_none_or(|x| x.update == 0) { continue; }
                if self.current == 0 { break; }
                let r = self.call_update(w, inp, gs, env, &mut out);
                if r != 0 {
                    self.post = 1;
                    if (self.kind - 0xf) as u32 >= 2 { continue; }
                    self.post = 2;
                }
            }
            let cur = self.current;
            if let Some(pending) = self.page(cur).map(|p| p.pending).filter(|&p| p != 0) {
                let focus = self.page(cur).map_or(0, |p| p.focus);
                self.call_leave(focus, &mut out, gs);
                if let Some(p) = self.pages.get_mut(&cur) {
                    p.focus = pending;
                    p.pending = 0;
                }
                self.call_enter(pending, true, gs);
            }
        }
        // MobyUpdateLoop 0x2793d8: the frame mobys (advance, MenuMobyUpdate, corners) and the pages' own mobys (the
        // Items page's gold bolt turns).
        if let Some(f) = self.frames.as_mut() { f.update(); }
        pages::gold_tick(self);
        media::girl_tick(self);
        if self.post != 0 && self.ticks >= 10 {
            // PageMenuClose 0x28c6c8.
            let ws = self.page(self.current).map(|p| p.widgets).unwrap_or([0; 14]);
            for w in ws.iter().filter(|&&w| w != 0) { self.call_leave(*w, &mut out, gs); }
            if let Some(f) = self.frames.as_mut() { f.free(); }
            // The hand / feet / head / back requests of the slots the pages changed.
            out.equip = Some(crate::inventory::menu_close_requests(&gs.global.equipped, &self.equip));
            self.current = 0;
            self.kind = 0x14;
            self.trans = 2;
        }
        out
    }

    fn call_enter(&mut self, w: u32, refocus: bool, gs: &GameState) {
        let Some(enter) = self.w(w).map(|x| x.enter) else { return };
        match enter {
            0 | 0x28dd10 | 0x28dd18 | 0x295200 => {}
            func::ROOT_ENTER => {
                // 0x2917d8: the first item's action = 0 (disabled) when 0x1413f4 == 1, else 3. The byte is
                // passed through `enter_b13f4` (set by the caller before the tick).
                let b = self.enter_b13f4;
                if let Some(Data::List(l)) = self.wm(w).map(|x| &mut x.data) {
                    if let Some(it) = l.items.first_mut() { it.action = if b == 1 { 0 } else { 3 }; }
                }
            }
            func::LABEL_ENTER => {
                if let Some(Data::Label(l)) = self.wm(w).map(|x| &mut x.data) { l.timer = -1; }
            }
            gadgets::func::GRID_ENTER => gadgets::grid_enter(self, w, refocus),
            gadgets::func::MODEL_ENTER => gadgets::model_enter(self, w),
            gadgets::func::PREVIEW_ENTER => gadgets::preview_enter(self, w),
            func::PLANET_LIST_ENTER => {
                let dest = self.dest;
                let names = std::mem::take(&mut self.level_names);
                if let Some(Data::List(l)) = self.wm(w).map(|x| &mut x.data) { planet_select::build_list(l, gs, dest, &names); }
                self.level_names = names;
            }
            // 0x2936e8, shared by the image widgets and the streamed pictures.
            pages::func::IMAGE_ENTER if matches!(self.w(w).map(|x| &x.data), Some(Data::Picture(_))) => map_page::picture_enter(self, w),
            pages::func::IMAGE_ENTER => pages::image_enter(self, w),
            func::MISSIONS_ENTER => map_page::missions_enter(self, w, gs),
            func::GLOBE_ENTER => map_page::globe_enter(self),
            pages::func::SLOTS_ENTER => pages::slots_enter(self, w, gs),
            pages::func::GOLD_ENTER => pages::gold_enter(self, w),
            pages::func::LOG_LIST_ENTER => {
                let ids = std::mem::take(&mut self.log_ids);
                pages::log_list_enter(self, w, gs, &ids);
                self.log_ids = ids;
            }
            pages::func::TEXT_ENTER => pages::text_enter(self, w),
            pages::func::HELP_WEAPONS_ENTER => pages::help_weapons_enter(self, gs),
            pages::func::HELP_GADGETS_ENTER => pages::help_gadgets_enter(self, gs),
            pages::func::MOVES_LABEL_ENTER => pages::moves_label_enter(self, w, gs),
            pages::func::MOVIES_ENTER => pages::movies_enter(self, w, gs),
            // fun_00225ac0(1): the stream buffers' layout (memory only).
            pages::func::STREAM_LAYOUT_ENTER => {}
            saves::func::SLOTS_ENTER => saves::slots_enter(self, w),
            saves::func::CONFIRM_ENTER => saves::confirm_enter(self),
            saves::func::FRONT_OPTIONS_ENTER => {
                let pal = self.saves.card.pal;
                saves::front_options_enter(self, w, pal);
            }
            media::func::CHEATS_ENTER => media::cheats_enter(self, w, gs),
            media::func::SCROLL_ENTER => media::scroller_enter(self, w),
            media::func::GIRL_ENTER => media::girl_enter(self),
            _ => stub(&mut self.stub_calls, "widget enter"),
        }
    }

    fn call_leave(&mut self, w: u32, out: &mut MenuOut, gs: &mut GameState) {
        let Some(leave) = self.w(w).map(|x| x.leave) else { return };
        match leave {
            0 | 0x28dd10 | 0x28dd18 => {}
            // 0x290508: 0x13e5a0 (group 2's master volume) = sfx·8/10: the port's `sound_update` derives group 2 from the
            // sfx option every frame (`audio::voices::master_volumes`), which is this value; the Sound page's preview
            // of group 2 at the music volume and the options reaching the mixer: G-AUD-013.
            func::SOUND_LEAVE => {}
            // 0x28f7a8: the map file stream's break (the port reads the files at once: no stream).
            func::MAP_LEAVE => {}
            gadgets::func::MODEL_LEAVE => gadgets::model_leave(self, w),
            gadgets::func::PREVIEW_LEAVE => gadgets::preview_leave(self, w),
            pages::func::IMAGE_LEAVE if matches!(self.w(w).map(|x| &x.data), Some(Data::Picture(_))) => map_page::picture_leave(self, w),
            pages::func::IMAGE_LEAVE => pages::image_leave(self, w),
            pages::func::SLOTS_LEAVE => pages::slots_leave(self, w, gs),
            pages::func::GOLD_LEAVE => pages::gold_leave(self, w),
            pages::func::LOG_LIST_LEAVE => {}
            // The stream buffers of the slot and confirm pages (memory only).
            saves::func::SLOTS_LEAVE | saves::func::CONFIRM_LEAVE => {}
            pages::func::TEXT_LEAVE => pages::text_leave(self),
            media::func::GIRL_LEAVE => media::girl_leave(self, out),
            _ => stub(&mut self.stub_calls, "widget leave"),
        }
    }

    /// The generic keys every focused widget's update runs (Start/Select/R3 close, △ parent).
    /// Returns `Some(ret)` when the update ends here.
    fn generic_keys(&mut self, inp: &MenuInput, planet_cursor: bool, level: i32) -> Option<i32> {
        if inp.pressed_u & 0xd00 != 0 && !self.no_close {
            if planet_cursor { self.dest = level; }
            return Some(-1);
        }
        if inp.pressed_u & button::TRIANGLE != 0 {
            if planet_cursor { self.dest = level; }
            let parent = self.page(self.current).map_or(0, |p| p.parent);
            if parent != 0 {
                self.target = parent;
            } else if !self.no_close {
                return Some(-1);
            }
        }
        None
    }

    fn is_focus(&self, w: u32) -> bool { self.page(self.current).is_some_and(|p| p.focus == w) }

    fn call_update(&mut self, w: u32, inp: &MenuInput, gs: &mut GameState, env: &MenuEnv, out: &mut MenuOut) -> i32 {
        let upd = self.w(w).map_or(0, |x| x.update);
        match upd {
            func::LIST_UPDATE => self.list_update(w, inp, gs, out),
            func::LABEL_UPDATE => {
                if !self.is_focus(w) { return 0; }
                self.generic_keys(inp, false, gs.global.level).unwrap_or(0)
            }
            func::TOGGLE_UPDATE => options::toggle_update(self, w, inp, gs, out),
            func::CAMERA_UPDATE => options::camera_update(self, w, inp, gs, out),
            func::SOUND_UPDATE => options::sound_update(self, w, inp, gs, out),
            func::QUIT_UPDATE => options::quit_update(self, w, inp, gs, out),
            func::GALAXY_UPDATE => {
                if inp.pressed_u & button::CROSS != 0 { self.target = self.addrs.planet_confirm; }
                0
            }
            func::MAP_UPDATE => {
                // 0x28f868: fun_00205440 (pan / zoom), the keys, then the compose.
                map_page::pan_zoom(self, inp);
                let r = planet_select::map_update(self, w, inp, gs, out);
                map_page::compose(self, gs);
                r
            }
            func::CONFIRM_UPDATE => planet_select::confirm_update(self, inp, gs, out),
            func::MISSIONS_UPDATE => planet_select::missions_update(self, w, inp, gs, out),
            func::PICTURE_UPDATE => map_page::picture_update(self, w),
            port::UPDATE => port::update(self, w, inp, out),
            gadgets::func::GRID_UPDATE => gadgets::grid_update(self, w, inp, gs, out),
            // `LoadHandGadget` 0x297d70: the models follow the page's copy of the saved items (engine side).
            gadgets::func::MODEL_UPDATE => 0,
            gadgets::func::PREVIEW_UPDATE => gadgets::preview_update(self, w),
            pages::func::IMAGE_UPDATE => pages::image_update(self, w, gs),
            pages::func::CONTROLS_UPDATE => {
                let lang = self.lang;
                pages::controls_update(self, w, lang)
            }
            pages::func::SLOTS_UPDATE => pages::slots_update(self, w, inp, gs, out),
            pages::func::AMMO_UPDATE => 0,
            pages::func::AMMO_MODEL_UPDATE => pages::ammo_model_update(self, w),
            pages::func::TEXT_UPDATE => {
                pages::text_update(self, w);
                if !self.is_focus(w) { return 0; }
                self.generic_keys(inp, false, gs.global.level).unwrap_or(0)
            }
            pages::func::ICONS_UPDATE => pages::icons_update(self, w, inp, gs, out),
            saves::func::SAVE_UPDATE => saves::save_update(self, w, inp, gs, out),
            saves::func::LOAD_UPDATE => saves::load_update(self, w, inp, gs, out),
            saves::func::NEW_GAME_UPDATE => saves::new_game_update(self, w, inp, gs, out),
            saves::func::CONFIRM_UPDATE => saves::confirm_update(self, inp),
            media::func::STATS_UPDATE => media::stats_update(self, inp, out),
            media::func::SKETCH_PAGER => media::sketch_pager(self, w, inp, out),
            media::func::EPILOGUE_PAGER => media::epilogue_pager(self, w, inp, out),
            media::func::GIRL_UPDATE => media::girl_update(self, gs, out),
            media::func::SCROLL_UPDATE => media::scroller_update(self, w, inp),
            _ => {
                let _ = env;
                stub(&mut self.stub_calls, "widget update");
                0
            }
        }
    }

    /// The list widget update 0x28e600 (Lombyte `MenuSetPostAction`).
    fn list_update(&mut self, w: u32, inp: &MenuInput, gs: &mut GameState, out: &mut MenuOut) -> i32 {
        let focused = self.is_focus(w);
        let s = scale_ticks(self.consts.hl_ticks) as i16;
        let (flags, cursor) = match self.w(w).map(|x| &x.data) {
            Some(Data::List(l)) => (l.flags, l.cursor),
            _ => return 0,
        };
        if let Some(Data::List(l)) = self.wm(w).map(|x| &mut x.data) {
            for (i, it) in l.items.iter_mut().enumerate() {
                it.hl = if focused && cursor == i as i32 {
                    it.hl.wrapping_add(1)
                } else {
                    let t = if s < it.hl { s } else { it.hl };
                    if t < 1 { 0 } else { t - 1 }
                };
            }
        }
        if !focused { return 0; }
        let level = gs.global.level;
        let planet = flags & lf::PLANET_CURSOR != 0;
        if let Some(r) = self.generic_keys(inp, planet, level) { return r; }
        let items = match self.w(w).map(|x| &x.data) {
            Some(Data::List(l)) => l.items.clone(),
            _ => return 0,
        };
        if inp.pressed_u & button::CROSS != 0 {
            if let Some(it) = items.get(cursor as usize) {
                match it.action {
                    2 => out.sounds.push(MenuSound::Denied),
                    3 => self.target = it.arg,
                    4 | 5 => {
                        out.sounds.push(MenuSound::Confirm);
                        // Save / Load: straight to the page when the card is ready (0x15eeb0 ∈ {1, 0x10}), else flag
                        // 2 (save) / 4 (load) and the card dialog `mode_freezeInit(3, page)`.
                        if self.saves.card.ready() {
                            self.target = it.arg;
                        } else {
                            self.saves.card.flags |= if it.action == 4 { 2 } else { 4 };
                            out.freeze = Some(it.arg);
                        }
                    }
                    6 | 7 | 8 | 10 | 11 => {
                        self.post = match it.action {
                            6 => 5,
                            7 => 3,
                            8 => 4,
                            10 => 6,
                            _ => 7,
                        };
                        if it.action != 11 {
                            self.return_kind = if it.action == 6 { 0 } else { 2 };
                            self.arg = if it.action == 6 { it.arg & 0xffff } else { it.arg };
                        } else {
                            self.return_kind = 2;
                        }
                        self.return_page = self.current;
                        out.sounds.push(MenuSound::Confirm);
                        return 0;
                    }
                    9 => {
                        // Language 0x15ed88 = arg (session state; the front end's Language list), no sound.
                        self.lang = it.arg;
                        out.language = Some(it.arg);
                        return 0;
                    }
                    _ => {}
                }
            }
        }
        let n = items.len() as i32;
        let keys = if flags & lf::BUTTONS_ONLY != 0 { inp.raw_pressed } else { inp.pressed_u };
        let mut new = cursor;
        let mut pending = 0u32;
        let (above, below) = match self.w(w).map(|x| &x.data) {
            Some(Data::List(l)) => (l.above, l.below),
            _ => (0, 0),
        };
        if keys & button::UP != 0 || (flags & lf::SHOULDERS != 0 && keys & button::L1 != 0) {
            if new == 0 {
                if flags & lf::WRAP == 0 { pending = above } else { new = n - 1 }
            } else {
                new -= 1;
            }
        }
        if keys & button::DOWN != 0 || (flags & lf::SHOULDERS != 0 && keys & button::R1 != 0) {
            let next = items.get((new + 1) as usize);
            if next.is_none_or(|it| it.label == 0 || it.action == 0) {
                if flags & lf::WRAP == 0 { pending = below } else { new = 0 }
            } else {
                new += 1;
            }
        }
        if pending != 0 {
            if let Some(p) = self.pages.get_mut(&self.current) { p.pending = pending; }
        }
        let has_pending = self.page(self.current).is_some_and(|p| p.pending != 0);
        if new == cursor && !has_pending { return 0; }
        if let Some(Data::List(l)) = self.wm(w).map(|x| &mut x.data) { l.cursor = new; }
        out.sounds.push(MenuSound::Cursor);
        if planet {
            let order = &gs.global.map_order;
            self.dest = order.get(new as usize).copied().unwrap_or(0);
        }
        0
    }

    /// `PageMenuDraw` 0x28d080(0). Draw callbacks can change widget state (label timers, list flags).
    ///
    /// After the snapshot and the black (and, in the engine, the frame mobys `DrawMobyList(m, 1)` under
    /// TEST_1 0x5360b), every live frame moby slot (enable array 0x1b2840 all 1; slot 6 only with Goodies)
    /// gets its rect from `MobyScreenRect(corner 0, corner 3)`: (x + 1, y + 1, w, h) into pvar +0x50.. and,
    /// while a page is current, into that page's widget of the slot (+0x18..+0x24); then the navy rect
    /// `fun_00200e08(x + 2, y + 2, x + w, y + h)`. The widgets draw into their slot's rect (pvar +0x50..) in
    /// two passes (draw flags & 2 first). Last, with or without a current page, every live slot's panel effect
    /// `fun_00223e28(0x1ba310[i])` (vignette, noise bursts, scan lines, glass: crate::menus::screen_static), which
    /// draws from the game's `rand` stream `rng`.
    pub fn draw(&mut self, a: &MenuAssets, gs: &GameState, env: &MenuEnv, rng: &mut crate::rng::Rng, out: &mut Vec<MenuDraw>) {
        out.push(MenuDraw::Snapshot);
        out.push(MenuDraw::Darken { alpha: DARKEN });
        self.view = gadgets::GadgetsView::default();
        self.media.girl_view = None;
        if self.kind == 0x14 { return; }
        let cur = self.page(self.current).cloned();
        let mut rects: [Option<[i32; 4]>; 14] = [None; 14];
        for (i, r) in rects.iter_mut().enumerate() {
            if i == 6 && !self.goodies { continue; }
            let Some([x, y, w, h]) = self.frames.as_mut().and_then(|f| f.place(i)) else { continue };
            *r = Some([x, y, w, h]);
            if let Some(p) = &cur {
                if let Some(wd) = self.wm(p.widgets[i]) { wd.rect = [x, y, w, h]; }
            }
            out.push(MenuDraw::Rect { x0: x + 1, y0: y + 1, x1: x + w - 1, y1: y + h - 1, rgba: self.consts.navy });
        }
        if let Some(cp) = cur {
            for pass in 0..2 {
                for (i, (&wa, rect)) in cp.widgets.iter().zip(rects).enumerate() {
                    let Some(wd) = self.w(wa) else { continue };
                    if wd.dflags & 4 != 0 || wd.draw == 0 || (i == 6 && !self.goodies) { continue; }
                    if (pass == 0) != (wd.dflags & 2 != 0) { continue; }
                    let Some([x, y, w, h]) = rect else { continue };
                    let mut buf = Vec::new();
                    let ret = self.call_draw(wa, a, gs, env, &mut buf);
                    if wd_direct(self.w(wa)) {
                        out.extend(buf);
                        continue;
                    }
                    if ret & 1 != 0 || ret & 0x1e == 0 { continue; }
                    out.push(MenuDraw::PanelBegin { x, y, w, h, clear: self.consts.navy });
                    out.extend(buf);
                    out.push(MenuDraw::PanelEnd);
                }
            }
        }
        // The panels' effect, slot by slot (the same slots as the rects).
        let frame = a.frame(screen_static::VIGNETTE_ICON, screen_static::VIGNETTE_FRAME);
        let vignette = (frame, a.frame_size(frame));
        let mut fx = Vec::new();
        for (i, r) in rects.iter().enumerate() {
            if r.is_none() { continue; }
            if let Some(f) = self.frames.as_mut() { f.panel_static(i, env.pal, vignette, rng, &mut fx); }
        }
        out.extend(fx.into_iter().map(MenuDraw::Static));
    }

    fn call_draw(&mut self, w: u32, a: &MenuAssets, gs: &GameState, env: &MenuEnv, out: &mut Vec<MenuDraw>) -> u32 {
        let d = self.w(w).map_or(0, |x| x.draw);
        match d {
            func::LIST_DRAW => self.list_draw(w, a, out),
            func::LABEL_DRAW => self.label_draw(w, a, gs, out),
            func::TOGGLE_DRAW => options::toggle_draw(self, w, a, gs, out),
            func::CAMERA_DRAW => options::camera_draw(self, w, a, gs, out),
            func::SOUND_DRAW => options::sound_draw(self, w, a, gs, out),
            func::QUIT_DRAW => options::quit_draw(self, w, a, out),
            func::PLANET_LIST_DRAW => planet_select::list_draw(self, w, a, out),
            func::GALAXY_DRAW => planet_select::galaxy_draw(self, w, a, gs, env, out),
            port::DRAW => port::draw(self, w, a, out),
            gadgets::func::GRID_DRAW => gadgets::grid_draw(self, w, a, gs, env.vsync, out),
            gadgets::func::MODEL_DRAW => gadgets::model_draw(self, w, gs),
            gadgets::func::PREVIEW_DRAW => gadgets::preview_draw(self, w, a, gs),
            // 0x293d50 with flag 0x100 and no picture yet: the card's busy text (`saves::busy_draw`).
            pages::func::IMAGE_DRAW if matches!(self.w(w).map(|x| &x.data), Some(Data::Image(i)) if i.state < 2 && i.flags & 0x100 != 0) => saves::busy_draw(self, w, a, out),
            pages::func::IMAGE_DRAW => pages::image_draw(self, w, gs, out),
            pages::func::CONTROLS_DRAW => {
                let lang = self.lang;
                pages::controls_draw(self, w, lang, out)
            }
            pages::func::SLOTS_DRAW => pages::slots_draw(self, w, a, gs, env.vsync, out),
            pages::func::AMMO_DRAW => {
                let rec = std::mem::take(&mut self.ammo_records);
                let r = pages::ammo_draw(self, w, a, gs, &rec, out);
                self.ammo_records = rec;
                r
            }
            pages::func::AMMO_MODEL_DRAW => pages::ammo_model_draw(self, w, a, gs, out),
            pages::func::GOLD_DRAW => {
                let lang = self.lang;
                pages::gold_draw(self, w, a, gs, lang, out)
            }
            pages::func::ICONS_DRAW => pages::icons_draw(self, w, a, env.vsync, out),
            pages::func::SKILL_DRAW => pages::skill_draw(self, w, a, out),
            func::MAP_DRAW => map_page::draw(self, w, a, gs, out),
            func::MAP_LEGEND_DRAW => map_page::legend_draw(self, w, a, gs, out),
            func::GLOBE_DRAW => map_page::globe_draw(self, w, env.vsync, out),
            func::MISSIONS_DRAW => map_page::missions_draw(self, w, a, out),
            func::PICTURE_DRAW => map_page::picture_draw(self, w, out),
            func::SHIP_GOLD_DRAW => map_page::gold_panel_draw(self, w, a, gs, out),
            saves::func::SLOTS_DRAW => saves::slots_draw(self, w, a, env.pal, out),
            saves::func::SLOT_INFO_DRAW => saves::slot_info_draw(self, w, a, out),
            saves::func::CONFIRM_DRAW => saves::confirm_draw(self, w, a, out),
            media::func::STATS_DRAW => media::stats_draw(self, w, a, gs, out),
            media::func::EPILOGUE_ARROWS => media::epilogue_arrows(self, w, a, out),
            media::func::GIRL_DRAW => media::girl_draw(self, w),
            media::func::SCROLL_DRAW => media::scroller_draw(self, w, a, out),
            _ => {
                stub(&mut self.stub_calls, "widget draw");
                out.push(MenuDraw::Stub("widget draw"));
                1
            }
        }
    }

    /// The list draw 0x28ebd0 (panel-local).
    fn list_draw(&mut self, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
        let focus = self.page(self.current).map_or(0, |p| p.focus);
        let s = scale_ticks(self.consts.hl_ticks);
        let sh = self.consts.shadow;
        let swapped = self.text_swap == 2;
        let Some(wd) = self.wm(w) else { return 1 };
        let [_, _, ww, wh] = wd.rect;
        let Data::List(l) = &mut wd.data else { return 1 };
        let (font, size) = list_font(l.flags);
        let n = l.items.len() as i32;
        let step = if l.flags & lf::STEP_FROM_FONT != 0 { size + 3 } else { wh / (n + 1) };
        let mut y = step - size / 2 - 1;
        let msg = |id: i16| a.msg_in(id as i32, swapped);
        let mut maxw = 0;
        if l.flags & lf::COMMON_CENTRE != 0 {
            for it in &l.items { maxw = maxw.max(a.width(font, msg(it.label))); }
        }
        if l.flags & lf::AUTO_SMALL != 0 && maxw + 6 > ww && l.flags & lf::SMALL == 0 {
            l.flags |= lf::SMALL;
            return 1;
        }
        let maxw = maxw.min(ww);
        let cx = ww >> 1;
        for (i, it) in l.items.iter().enumerate() {
            let sel = focus == w && l.cursor == i as i32;
            let disabled = it.action == 0;
            let col = if l.flags & lf::FIXED_COLOUR != 0 {
                LIGHT_BLUE
            } else if disabled {
                if sel { DISABLED_SELECTED } else { DISABLED }
            } else {
                hl_colour(it.hl as i32, s, LIGHT_BLUE, YELLOW)
            };
            let t = if it.action == 2 { a.msg_in(20308, swapped) } else { msg(it.label) };
            let tw = a.width(font, t);
            let x = if l.flags & lf::LEFT != 0 {
                4
            } else if l.flags & lf::COMMON_CENTRE != 0 {
                cx - (maxw >> 1)
            } else {
                cx - (tw >> 1)
            };
            text_plain(out, font, x + sh.0, y + sh.1, SHADOW, t);
            if l.flags & lf::NO_CODES != 0 { text_plain(out, font, x, y, col, t) } else { text(out, font, x, y, col, t) }
            let y2 = y + step;
            if it.sublabel != 0 {
                let t2 = msg(it.sublabel);
                text_plain(out, font, x + sh.0, y2 + sh.1, SHADOW, t2);
                if l.flags & lf::NO_CODES != 0 { text_plain(out, font, x, y2, col, t2) } else { text(out, font, x, y2, col, t2) }
                y = y2 + step;
            } else {
                y = y2;
            }
        }
        2
    }

    /// The label draw 0x28dd30 (panel-local; the content logic and the cross-fade live here).
    fn label_draw(&mut self, w: u32, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) -> u32 {
        let s = scale_ticks(self.consts.hl_ticks);
        let (margin, lh, sh, navy) = (self.consts.margin, self.consts.line_height, self.consts.shadow, self.consts.navy);
        let focus = self.page(self.current).map_or(0, |p| p.focus);
        let focus_cursor = match self.w(focus).map(|x| &x.data) {
            Some(Data::List(l)) => l.cursor,
            _ => 0,
        };
        let dest = self.dest;
        let level = gs.global.level;
        let focus_item = gadgets::focused_item(self);
        let swapped = self.text_swap == 2;
        // Source 0x100: the focused grid / icon list's cursor cell (shown only while its item is owned or its flag set).
        let focus_cell = match self.w(focus).map(|x| &x.data) {
            Some(Data::Grid(g)) => Some((g.cursor, g.cells.get(g.cursor.max(0) as usize).copied())),
            Some(Data::IconList(l)) => Some((l.cursor, l.cells.get(l.cursor.max(0) as usize).copied())),
            _ => None,
        };
        // Source 0x1000: the focused list's cursor item's label, looked up in the help log's table (`fun_001fecc8(label,
        // 1, &+0x34)`): the help message of a Help Log entry.
        let focus_log = match self.w(focus).map(|x| &x.data) {
            Some(Data::List(l)) => Some((l.cursor, l.items.get(l.cursor.max(0) as usize).map_or(0, |it| it.label))),
            _ => None,
        };
        let log_msg = focus_log.and_then(|(_, t)| self.log_ids.iter().find(|e| e.1 == t).map(|e| e.0 as u16 as u32));
        let tables = self.label_tables.clone();
        let Some(wd) = self.wm(w) else { return 1 };
        let [_, _, ww, wh] = wd.rect;
        let Data::Label(l) = &mut wd.data else { return 1 };
        let font = if l.flags & 0x10 != 0 { Font::Small } else if l.flags & 8 != 0 { Font::Large } else { Font::Regular };
        let mut variant = 0i32;
        let content: i32 = if l.flags & 0x20 != 0 {
            let c = level - 1;
            if c as u32 > 0x11 { -1 } else { c }
        } else if l.flags & 0x40 != 0 {
            dest - 1
        } else if l.flags & 4 != 0 {
            if s < l.timer { l.timer = s; }
            l.content = 0;
            l.variant = 0;
            0
        } else if l.flags & 0x80 != 0 {
            if l.flags & 0x8000 != 0 { variant = 0; }
            focus_cursor
        } else if l.flags & 0x1100 == 0 {
            // Default: the focused grid's cursor cell's item (+6), variant 1 for its gold version (0x13e520[item]).
            let Some(item) = focus_item else { return 1 };
            variant = gs.global.gold_weapons.get(item.max(0) as usize).is_some_and(|&b| b != 0) as i32;
            item
        } else if l.flags & 0x100 == 0 {
            // 0x1000: the entry's help message id into +0x34 (0xffff: none).
            let Some((cursor, _)) = focus_log else { return 1 };
            l.id = log_msg.unwrap_or(0xffff);
            cursor
        } else {
            // 0x100: the cell's item owned (kind 0) or its global flag set (kind 1), else −1.
            let Some((cursor, cell)) = focus_cell else { return 1 };
            let set = cell.is_some_and(|c| {
                let t = if c.kind == 0 { &gs.global.owned[..] } else { &gs.global.flags[..] };
                t.get(c.item.max(0) as usize).is_some_and(|&b| b != 0)
            });
            if set { cursor } else { -1 }
        };
        if l.timer == -1 {
            l.timer = s;
            l.content = content;
            l.variant = variant;
        }
        let (mut content, mut variant) = (content, variant);
        if content == l.content {
            l.timer += 3;
        } else {
            if s < l.timer { l.timer = s; }
            for _ in 0..3 { l.timer = (l.timer - 1).max(0); }
            if l.timer == 0 {
                l.content = content;
                l.variant = variant;
                l.flags &= !0x400;
                l.scroll = 0;
            } else {
                content = l.content;
                variant = l.variant;
            }
        }
        let owned = |i: i32| usize::try_from(i).ok().and_then(|i| gs.global.owned.get(i)).is_some_and(|&b| b != 0);
        let mut id = 0u32;
        let mut t: Vec<u8> = if l.flags & 4 != 0 {
            if l.id == 0 { return 1; }
            id = l.id;
            a.msg_in(l.id as i32, swapped).to_vec()
        } else if l.flags & 0x1000 != 0 {
            if l.id == 0xffff { return 1; }
            id = l.id;
            a.msg_in(l.id as i32, swapped).to_vec()
        } else if l.flags & 0x100 != 0 && content < 0 {
            Vec::new()
        } else if l.id != 0 {
            // `*(table + (content·stride & ~3) + variant·4)`, the table in the overlay (`MenuAssets::overlay`).
            // `content` may be −1 (the flags 0x20 / 0x40 index from one past the table base: the map and confirm
            // pages' labels carry `0x1c22cc` / `0x1c22d0` = `0x1c22c0 + 12` / `+16` and `dest − 1`, so on Veldin
            // (dest 0) the address lands on level 0's entry one record before the base). The game's arithmetic
            // wraps; the patched tables below are only for flags whose content is never negative.
            let addr = l.id.wrapping_add((content as u32).wrapping_mul(l.stride) & !3).wrapping_add(variant as u32 * 4);
            id = match (&l.table, tables.get(&l.id)) {
                (Some(t), _) if content >= 0 => t.get(content as usize).copied().unwrap_or(0),
                // A table a page enter built (the Help / Weapons and Help / Gadgets descriptions).
                (None, Some(t)) if content >= 0 => t.get(content as usize).copied().unwrap_or(0),
                _ => a.overlay.u32(addr).unwrap_or(0),
            };
            a.msg_in(id as i32, swapped).to_vec()
        } else {
            b"default".to_vec()
        };
        if l.flags & 0x11e4 == 0 && !owned(content) { t.clear(); }
        if l.flags & 0x200 != 0 && ![0x4ed2, 0x4ed9, 0x4edd].contains(&id) {
            let mut s2 = a.msg(0x4ecc).to_vec();
            s2.push(b' ');
            s2.extend_from_slice(&t);
            t = s2;
        }
        let mut flags = l.flags;
        let (mut anchor, mut ystart) = (4i32, 4i32);
        if flags & 0x4004 == 0x4004 && l.id == 0x523e {
            flags |= 1;
            ystart = 12;
        }
        if flags & 0x800 != 0 && gs.global.skill_points.get(content.max(0) as usize).is_some_and(|&b| b == 0) {
            flags |= 3;
            t = a.msg(0x4f54).to_vec();
        }
        let mut wflags = wtext::FLOAT_POS;
        if flags & 1 != 0 {
            wflags |= wtext::CENTRE_LINES;
            anchor = ww / 2;
        }
        if flags & 2 != 0 {
            wflags |= wtext::CENTRE_BLOCK;
            ystart = wh / 2;
        }
        let y_max = if l.flags & 0x10000 != 0 { wh - 1 } else { wh - margin };
        let mut win = wtext::Window::new(margin as i16, y_max as i16, 1, (ww - 4) as i16, anchor as i16, (ystart - (l.scroll >> 4)) as i16, lh as i16, wflags);
        win.sub_y = -((l.scroll & 0xf) as i16);
        let col = hl_colour(l.timer, s, tween(0.5, navy, LIGHT_BLUE), LIGHT_BLUE);
        // Measure pass (flag 4).
        let mut m = win;
        m.flags |= wtext::MEASURE_ONLY;
        wtext::layout(&mut m, &t, -1, &a.hud.glyphs[font as usize], true);
        let fits = (m.height as i32) + 4 < (m.y_max as i32 - m.y_min as i32);
        if l.flags & 0x2000 != 0 || fits {
            if l.flags & 0x400 != 0 {
                l.scroll = 0;
                l.flags ^= 0x400;
            }
        } else if l.flags & 0x400 == 0 {
            l.flags |= 0x400;
            l.scroll = wh * -8;
        }
        let shifted = |w0: &wtext::Window, dx: i32, dy: i32| {
            let mut w1 = *w0;
            w1.y_min += dy as i16;
            w1.y_max += dy as i16;
            w1.x_min += dx as i16;
            w1.x_max += dx as i16;
            w1.x_anchor += dx as i16;
            w1.y_start += dy as i16;
            w1
        };
        win.y_start = (ystart - (l.scroll >> 4)) as i16;
        let plain: Vec<u8> = t.iter().copied().filter(|c| c.wrapping_sub(8) >= 8).collect();
        out.push(MenuDraw::Hud(Draw::TextWindow { font, window: shifted(&win, sh.0, sh.1), rgba: SHADOW, text: plain.clone() }));
        out.push(MenuDraw::Hud(Draw::TextWindow { font, window: win, rgba: col, text: t.clone() }));
        if l.flags & 0x400 != 0 {
            let second = shifted(&win, 0, m.height as i32 + lh * 3);
            out.push(MenuDraw::Hud(Draw::TextWindow { font, window: shifted(&second, sh.0, sh.1), rgba: SHADOW, text: plain }));
            out.push(MenuDraw::Hud(Draw::TextWindow { font, window: second, rgba: col, text: t }));
            let period = ((m.height as i32) + lh * 3) * 16;
            if period != 0 { l.scroll = (l.scroll + 3) % period; }
        }
        2
    }
}

fn wd_direct(w: Option<&Widget>) -> bool { w.is_some_and(|w| w.dflags & 1 != 0) }

/// List font and size by flags (0x28ebd0: 12 regular, flag 4 → 14 large, flag 8 → 10 small).
pub fn list_font(flags: u32) -> (Font, i32) {
    if flags & lf::SMALL != 0 {
        (Font::Small, 10)
    } else if flags & lf::LARGE != 0 {
        (Font::Large, 14)
    } else {
        (Font::Regular, 12)
    }
}

/// `FUN_0028f0e0(t, from, to)`: `FastTweenColor(t ≥ S ? 1 : 1 − (S − t)/S, from, to)`, t < 0 counts as 0.
pub fn hl_colour(t: i32, s: i32, from: u32, to: u32) -> u32 {
    let t = t.max(0);
    let f = if s < t { 1.0 } else { 1.0 - (s - t) as f32 / s as f32 };
    tween(f, from, to)
}

fn read_page(ov: &Overlay, p: u32) -> Option<Page> {
    let seqs: [i32; 14] = std::array::from_fn(|i| ov.i32(p + 4 * i as u32).unwrap_or(0));
    let widgets: [u32; 14] = std::array::from_fn(|i| ov.u32(p + 0x44 + 4 * i as u32).unwrap_or(0));
    Some(Page { addr: p, seqs, parent: ov.u32(p + 0x38)?, kind: ov.i32(p + 0x3c)?, focus: ov.u32(p + 0x40)?, widgets, pending: ov.u32(p + 0x80)? })
}

fn read_items(ov: &Overlay, a: u32) -> Vec<Item> {
    let mut v = Vec::new();
    for k in 0..64u32 {
        let b = a + 12 * k;
        let Some(label) = ov.i16(b) else { break };
        if label == 0 { break; }
        v.push(Item { label, action: ov.i16(b + 2).unwrap_or(0), arg: ov.u32(b + 4).unwrap_or(0), sublabel: ov.i16(b + 8).unwrap_or(0), hl: ov.i16(b + 10).unwrap_or(0) });
    }
    v
}

fn read_widget(ov: &Overlay, a: u32) -> Option<Widget> {
    let u = |o: u32| ov.u32(a + o);
    let raw: [u32; 8] = std::array::from_fn(|i| u(0x30 + 4 * i as u32).unwrap_or(0));
    // The callbacks as their level-01 labels (widgets dispatch on them).
    let (update, draw) = (ov.label(u(0)?), ov.label(u(4)?));
    let data = match (update, draw) {
        // The confirm page's keys (0x295370) on a list-drawn widget: its hints rows (△ Exit, ○ Play Infobot, ✕ Blast
        // Off) are the list the draw reads; the keys read no data.
        (func::LIST_UPDATE, _) | (0, func::LIST_DRAW) | (media::func::SKETCH_PAGER, func::LIST_DRAW) | (func::CONFIRM_UPDATE, func::LIST_DRAW) => Data::List(List {
            flags: raw[0],
            items_addr: raw[1],
            items: read_items(ov, raw[1]),
            above: raw[2],
            below: raw[3],
            cursor: raw[4] as i32,
            scroll: raw[5] as i32,
        }),
        (func::LABEL_UPDATE, func::LABEL_DRAW) | (0, func::LABEL_DRAW) | (pages::func::TEXT_UPDATE, func::LABEL_DRAW) => {
            Data::Label(Label { flags: raw[0], id: raw[1], stride: raw[2], scroll: raw[3] as i32, timer: raw[5] as i32, content: raw[6] as i32, variant: raw[7] as i32, table: None })
        }
        (func::TOGGLE_UPDATE, _) => Data::Toggle(options::Toggle::read(ov, &raw)),
        (func::CAMERA_UPDATE, _) => Data::Camera(options::Camera::read(ov, &raw)),
        (func::SOUND_UPDATE, _) => Data::Sound { cursor: raw[4] as i32 },
        (func::QUIT_UPDATE, _) => Data::Quit,
        (func::GALAXY_UPDATE, _) => Data::Galaxy,
        (func::MAP_UPDATE, _) => Data::Map { passive: raw[1] & 0x40 != 0 },
        (func::CONFIRM_UPDATE, _) => Data::Confirm,
        (func::MISSIONS_UPDATE, _) => Data::Missions(map_page::Missions::default()),
        (func::PICTURE_UPDATE, _) => Data::Picture(map_page::Picture::read(&raw)),
        (media::func::SCROLL_UPDATE, _) => Data::Scroller(media::Scroller::default()),
        (gadgets::func::GRID_UPDATE, _) => Data::Grid(gadgets::Grid::read(ov, a)?),
        (gadgets::func::PREVIEW_UPDATE, _) => Data::Preview(gadgets::Preview { flags: raw[0], angle: f32::from_bits(raw[2]), class: -1, ..Default::default() }),
        (gadgets::func::MODEL_UPDATE, _) => Data::Model,
        (pages::func::IMAGE_UPDATE, _) => Data::Image(pages::Image::read(&raw)),
        (pages::func::CONTROLS_UPDATE, _) => Data::Controls { state: -1 },
        (pages::func::SLOTS_UPDATE, _) => Data::Slots { slots: [0; 8], cursor: 0 },
        (pages::func::AMMO_UPDATE, _) => Data::Ammo,
        (pages::func::AMMO_MODEL_UPDATE, _) => Data::AmmoModel(pages::AmmoModel::default()),
        (0, pages::func::GOLD_DRAW) => Data::GoldBolt,
        (pages::func::ICONS_UPDATE, _) => Data::IconList(pages::IconList::default()),
        (0, pages::func::SKILL_DRAW) => Data::Skill,
        _ => Data::Other,
    };
    Some(Widget { addr: a, update, draw, enter: ov.label(u(8)?), leave: ov.label(u(0xc)?), dflags: u(0x10)?, moby: u(0x14)? as i32, rect: [0; 4], raw, data })
}

#[cfg(test)]
mod tests;
