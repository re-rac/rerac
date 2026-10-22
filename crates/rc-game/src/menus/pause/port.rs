//! **Port-only** (not in the game): the "Port Options" page, for settings the PS2 never had (the graphics options and
//! their preset, anti-aliasing, switching the moby shadows off, the aspect, the render resolution, fullscreen, and the
//! controller's vibration, its strength, R2 as the fire button and the trigger effects).
//! Spec: docs/plan/menus.md "Port-only settings"; the graphics rows: docs/plan/graphics_options.md (every row's first
//! value is the game's own look).
//!
//! It is built entirely from the game's own machinery so it looks and behaves like the Options sub-pages:
//! a page record whose frame-moby seqs, filler widgets and "✕ Toggle / △ Exit" hint list are those of a
//! game sub-page ([`MODEL_FEW`] Subtitles for ≤ 2 rows, [`MODEL_MANY`] Camera for more), a title label with
//! the sub-pages' flags (0xf), and a list drawn and driven exactly like the camera list (update 0x294cc0 /
//! draw 0x294e68: label left at 12 in yellow / light blue, value right-aligned at w − 12, rows h/(n+1),
//! Up/Down without wrap with sound 1, ✕ cycles the value with sound 0, generic Start/Select/R3 and △ keys).
//! Transitions, panels, fonts, colours and sounds come from [`PageMenu`] unchanged.
//!
//! Entry: one item "Port Options" (action 3 → [`PAGE`]) inserted into the Options list 0x1b4f78 right before
//! Quit Game, so Quit Game stays the last (and, with the list's wrap, the Up-from-top) entry as on the disc;
//! the description label W1 (flags 0x90, table 0x1b4fc8) gets a patched copy of its id table with the port
//! description at the same index ([`Label::table`]). [`PageMenu::install_port_page`] does all of it; without
//! the call every record is exactly as read from the overlay.
//!
//! **The main menu** (the front end's Options page 0x1b8d08, its list filled by `0x28dbb8` from
//! `saves::data::FRONT_OPTIONS_*`): a second page [`PAGE_FRONT`] with the same title and list widgets, its frames those
//! of the front end's Sound page (so they play from, and △ goes back to, that Options page), and a "Port Options"
//! entry appended to both of the front end's Options lists.
//!
//! **Scrolling** (port-only, beyond the camera list's look): the list shows [`VISIBLE`] rows; the game's arrow (icon
//! 0xe99e frame 6, the Epilogue's page arrows) marks rows hidden above and below.
//!
//! Records live at addresses no EE pointer can hold (0x7f00_xxxx), and the text ids are negative (the disc's
//! ids are 0..=21500), so nothing here can collide with or be mistaken for game data.
//!
//! **Adding an option**: one [`Entry`] in [`ENTRIES`] (a [`Setting`] variant, a label id and its value ids in
//! [`text`]); the engine maps the [`Setting`] to its resource and persistence. The engine can restrict a row to
//! some of its values ([`PageMenu::set_port_choices`], e.g. the sample counts the GPU supports): ✕ then skips
//! the others.
//!
//! **The Gadgetron Movement System** (GMS, the controllers in the game's own lore): the "Controller" row ([`Setting::
//! Controller`], value "GMS") does not cycle; ✕ opens a page of its own ([`GMS_PAGE`], from the main menu
//! [`GMS_PAGE_FRONT`]) with the same frames, its title "Gadgetron Movement System" and a list of the connected
//! controllers ([`GmsList`]): each row a device's name and its status (Active, Available, Locked, Off). The engine fills
//! the rows before every menu tick ([`PageMenu::set_gms_rows`]) and reads the row ✕ was pressed on after it
//! ([`PageMenu::take_gms_pick`]); △ goes back to Port Options.

use super::super::{text as print, MenuAssets, MenuDraw, MenuInput, MenuSound, Overlay};
use super::options::{keys, right};
use super::{Data, Item, MenuOut, Page, PageMenu, Widget, LIGHT_BLUE, YELLOW};
use crate::pad::button;
use rc_formats::font::Font;

/// The page and its two own widgets (title label, settings list).
pub const PAGE: u32 = 0x7f00_0000;
/// The main menu's copy of the page (module docs).
pub const PAGE_FRONT: u32 = 0x7f00_0010;
pub const TITLE_W: u32 = 0x7f00_0100;
pub const LIST_W: u32 = 0x7f00_0200;
/// The list's callbacks (dispatched like the game's by address).
pub const UPDATE: u32 = 0x7f00_1000;
pub const DRAW: u32 = 0x7f00_1004;
/// The page kind (no game code keys on it).
pub const KIND: i32 = 0x7f;
/// The Gadgetron Movement System page (from the pause menu's Port Options), its main-menu copy, its title and list
/// widgets and the list's callbacks.
pub const GMS_PAGE: u32 = 0x7f00_0020;
pub const GMS_PAGE_FRONT: u32 = 0x7f00_0030;
pub const GMS_TITLE_W: u32 = 0x7f00_0300;
pub const GMS_LIST_W: u32 = 0x7f00_0400;
pub const GMS_UPDATE: u32 = 0x7f00_1008;
pub const GMS_DRAW: u32 = 0x7f00_100c;

/// Game records the page is attached to / modelled on (level01).
pub const OPTIONS: u32 = 0x1b4dd0;
pub const OPTIONS_LIST: u32 = 0x1b4f78;
pub const OPTIONS_LABEL: u32 = 0x1b4fe8;
pub const QUIT_PAGE: u32 = 0x1b6060;
/// The front end's Options page (kind 0x2f), and the label of its Sound entry, whose page is [`PAGE_FRONT`]'s model.
pub const FRONT_OPTIONS: u32 = 0x1b8d08;
const FRONT_SOUND_LABEL: i16 = 20258;
/// Subtitles 0x1b5788 (seqs 73..77: the one/two-row sub-page frames) and Camera 0x1b5ee0 (118..122, three rows). Only an
/// Options sub-page's frames will do: a page's frame animations start from its parent's layout, and △ plays them
/// backwards to it (another page's, e.g. Cheats', end on their own parent's layout under the Options page).
pub const MODEL_FEW: u32 = 0x1b5788;
pub const MODEL_MANY: u32 = 0x1b5ee0;
/// The rows a port list shows at once (in the pause menu the list panel is stretched for them: [`panel_layout`]); the
/// list scrolls ([`PortList::top`]).
pub const VISIBLE: usize = 7;

/// **Port-only** frame layout (`frame::PanelLayout`) of the pause menu's port pages: the Camera page's frames put their
/// title at the top left and their list in a short panel, so the title is centred and the list stretched from under it
/// to the hints. The main menu's copies keep the front end's Sound page frames as they are (a centred title, a tall
/// list); the game's own pages are never changed.
pub fn panel_layout(m: &PageMenu, page: u32) -> Option<super::frame::PanelLayout> {
    if page != PAGE && page != GMS_PAGE { return None; }
    let p = m.pages.get(&page)?;
    let title = p.widgets.iter().position(|&w| w == TITLE_W || w == GMS_TITLE_W)?;
    Some(super::frame::PanelLayout { seqs: p.seqs, title, list: 3, hint: 4 })
}

/// The port's own strings (not in the game's text table). Ids are negative; [`MenuAssets::msg`] resolves them.
pub mod text {
    pub const ENTRY: i32 = -0x100;
    pub const TITLE: i32 = -0x101;
    pub const DESCRIPTION: i32 = -0x102;
    pub const ANTI_ALIASING: i32 = -0x110;
    pub const X2: i32 = -0x111;
    pub const X4: i32 = -0x112;
    pub const X8: i32 = -0x113;
    pub const SHADOWS: i32 = -0x120;
    pub const RESOLUTION: i32 = -0x130;
    pub const RES_WINDOW: i32 = -0x131;
    pub const RES_416: i32 = -0x132;
    pub const RES_720: i32 = -0x133;
    pub const RES_1080: i32 = -0x134;
    pub const RES_1440: i32 = -0x135;
    pub const RES_2160: i32 = -0x136;
    pub const FULLSCREEN: i32 = -0x140;
    pub const ASPECT: i32 = -0x150;
    pub const ASPECT_4_3: i32 = -0x151;
    pub const ASPECT_16_9: i32 = -0x152;
    pub const ASPECT_16_10: i32 = -0x153;
    pub const PRESET: i32 = -0x160;
    pub const ORIGINAL: i32 = -0x161;
    pub const ENHANCED: i32 = -0x162;
    pub const CUSTOM: i32 = -0x163;
    pub const HUD: i32 = -0x170;
    pub const SHARP_PIXELS: i32 = -0x171;
    pub const TEXTURES: i32 = -0x180;
    pub const SMOOTH: i32 = -0x181;
    pub const SHARP: i32 = -0x182;
    pub const DETAIL: i32 = -0x190;
    pub const FAR: i32 = -0x191;
    pub const FARTHER: i32 = -0x192;
    pub const MAXIMUM: i32 = -0x193;
    pub const VIBRATION: i32 = -0x1a0;
    pub const VIBRATION_STRENGTH: i32 = -0x1a1;
    pub const LOW: i32 = -0x1a2;
    pub const MEDIUM: i32 = -0x1a3;
    pub const HIGH: i32 = -0x1a4;
    pub const FIRE_BUTTON: i32 = -0x1b0;
    pub const CIRCLE: i32 = -0x1b1;
    pub const R2: i32 = -0x1b2;
    pub const TRIGGER_EFFECTS: i32 = -0x1b3;
    pub const CONTROLLER: i32 = -0x1c0;
    pub const GMS: i32 = -0x1c1;
    pub const GMS_TITLE: i32 = -0x1c2;
    pub const ACTIVE: i32 = -0x1c3;
    pub const AVAILABLE: i32 = -0x1c4;
    pub const LOCKED: i32 = -0x1c5;
    pub const GMS_OFF: i32 = -0x1c6;
    /// The game's own "on" / "off" (20314 / 20315, the Subtitles / HelpDesk toggle values).
    pub const ON: i32 = 20314;
    pub const OFF: i32 = 20315;

    pub fn get(id: i32) -> Option<&'static [u8]> {
        Some(match id {
            ENTRY | TITLE => b"Port Options",
            DESCRIPTION => b"Settings of this port that the original game does not have",
            ANTI_ALIASING => b"Anti-aliasing",
            X2 => b"2x",
            X4 => b"4x",
            X8 => b"8x",
            SHADOWS => b"Shadows",
            RESOLUTION => b"Resolution",
            RES_WINDOW => b"Window",
            RES_416 => b"416p",
            RES_720 => b"720p",
            RES_1080 => b"1080p",
            RES_1440 => b"1440p",
            RES_2160 => b"2160p",
            FULLSCREEN => b"Fullscreen",
            ASPECT => b"Aspect ratio",
            ASPECT_4_3 => b"4:3",
            ASPECT_16_9 => b"16:9",
            ASPECT_16_10 => b"16:10",
            PRESET => b"Preset",
            ORIGINAL => b"Original",
            ENHANCED => b"Enhanced",
            CUSTOM => b"Custom",
            HUD => b"HUD",
            SHARP_PIXELS => b"Sharp pixels",
            TEXTURES => b"Texture filtering",
            SMOOTH => b"Smooth",
            SHARP => b"Sharp",
            DETAIL => b"Detail distance",
            FAR => b"Far",
            FARTHER => b"Farther",
            MAXIMUM => b"Maximum",
            VIBRATION => b"Vibration",
            VIBRATION_STRENGTH => b"Vibration strength",
            LOW => b"Low",
            MEDIUM => b"Medium",
            HIGH => b"High",
            FIRE_BUTTON => b"Fire button",
            CIRCLE => b"Circle",
            R2 => b"R2",
            TRIGGER_EFFECTS => b"Trigger effects",
            CONTROLLER => b"Controller",
            GMS => b"GMS",
            GMS_TITLE => b"Gadgetron Movement System",
            ACTIVE => b"Active",
            AVAILABLE => b"Available",
            LOCKED => b"Locked",
            GMS_OFF => b"Off",
            _ => return None,
        })
    }
}

/// A port setting (the engine owns the value's meaning and its persistence).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Setting {
    /// The graphics preset: value 0 Original, 1 Enhanced, 2 Custom (shown when the rows match no preset; ✕ skips it,
    /// the engine restricts the row).
    Preset,
    /// Anti-aliasing: value 0 Original (the game's own softening passes), 1 off, 2 2x, 3 4x, 4 8x multisampling.
    AntiAliasing,
    /// The moby shadows (docs/plan/shadows.md): value 0 on (the game's look, the default), 1 off.
    Shadows,
    /// The render resolution (the engine's game frame): value 0 fits the window, 1..5 a height of 416 (the PS2's lines), 720, 1080, 1440 or 2160 (the width
    /// follows the Aspect row).
    Resolution,
    /// The window: value 0 windowed, 1 fullscreen.
    Fullscreen,
    /// The frame's aspect: value 0 the TV's 4:3, 1 16:10, 2 16:9 (the 3D view widens; the 2D screen stays 4:3, centred).
    Aspect,
    /// How the game's 2D screen reaches the frame: value 0 Original (scaled smoothly, as on a TV), 1 Sharp pixels.
    Hud,
    /// How world textures pick their detail level: value 0 Original (the PS2's rule), 1 Smooth, 2 Sharp.
    Textures,
    /// How far away models keep their full detail: value 0 Original (the game's distances), 1 Far (×2), 2 Farther (×4),
    /// 3 Maximum (always).
    Detail,
    /// Controller rumble (the engine's haptics; the game has none): value 0 on, 1 off.
    Vibration,
    /// The rumble's strength: value 0 Low, 1 Medium, 2 High.
    VibrationStrength,
    /// The pad's fire button: value 0 ○ (the game's), 1 R2 (read as ○; R1 still crouches).
    FireButton,
    /// R2's adaptive-trigger feel per weapon (DualSense, with R2 as the fire button): value 0 on, 1 off.
    TriggerEffects,
    /// Not a value: ✕ opens the Gadgetron Movement System page (module docs).
    Controller,
}

/// One row: the setting, its label and the ids of its values (✕ cycles through them).
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub setting: Setting,
    pub label: i32,
    pub values: &'static [i32],
}

/// The rows, in order.
pub const ENTRIES: &[Entry] = &[
    Entry { setting: Setting::Preset, label: text::PRESET, values: &[text::ORIGINAL, text::ENHANCED, text::CUSTOM] },
    Entry { setting: Setting::AntiAliasing, label: text::ANTI_ALIASING, values: &[text::ORIGINAL, text::OFF, text::X2, text::X4, text::X8] },
    Entry { setting: Setting::Detail, label: text::DETAIL, values: &[text::ORIGINAL, text::FAR, text::FARTHER, text::MAXIMUM] },
    Entry { setting: Setting::Textures, label: text::TEXTURES, values: &[text::ORIGINAL, text::SMOOTH, text::SHARP] },
    Entry { setting: Setting::Hud, label: text::HUD, values: &[text::ORIGINAL, text::SHARP_PIXELS] },
    Entry { setting: Setting::Shadows, label: text::SHADOWS, values: &[text::ON, text::OFF] },
    Entry { setting: Setting::Aspect, label: text::ASPECT, values: &[text::ASPECT_4_3, text::ASPECT_16_10, text::ASPECT_16_9] },
    Entry { setting: Setting::Resolution, label: text::RESOLUTION, values: &[text::RES_WINDOW, text::RES_416, text::RES_720, text::RES_1080, text::RES_1440, text::RES_2160] },
    Entry { setting: Setting::Fullscreen, label: text::FULLSCREEN, values: &[text::OFF, text::ON] },
    Entry { setting: Setting::Controller, label: text::CONTROLLER, values: &[] },
    // The Controller page's rows ([`PORT_ROWS`] on): listed there above the connected controllers.
    Entry { setting: Setting::Vibration, label: text::VIBRATION, values: &[text::ON, text::OFF] },
    Entry { setting: Setting::VibrationStrength, label: text::VIBRATION_STRENGTH, values: &[text::LOW, text::MEDIUM, text::HIGH] },
    Entry { setting: Setting::FireButton, label: text::FIRE_BUTTON, values: &[text::CIRCLE, text::R2] },
    Entry { setting: Setting::TriggerEffects, label: text::TRIGGER_EFFECTS, values: &[text::ON, text::OFF] },
];

/// The Port Options page shows [`ENTRIES`]`[..PORT_ROWS]` (the last one the Controller row); the Controller page the
/// rest, then the connected controllers. Every row's value lives in the Port Options list ([`PortList::values`], by
/// [`ENTRIES`] index), whichever page shows it.
pub const PORT_ROWS: usize = 10;

/// The list's run-time state: cursor, each row's value index and each row's selectable values (bit k = value k;
/// a missing entry = all).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PortList {
    pub cursor: i32,
    /// The first row shown ([`VISIBLE`] at a time; the cursor stays among them).
    pub top: i32,
    pub values: Vec<u8>,
    pub choices: Vec<u32>,
}

/// A GMS row's status ([`GmsRow::status`]).
pub mod gms_status {
    /// The controller last used, input from all of them counting.
    pub const ACTIVE: u8 = 0;
    pub const AVAILABLE: u8 = 1;
    /// The only controller read (the player picked it).
    pub const LOCKED: u8 = 2;
    /// Ignored while another one is locked.
    pub const OFF: u8 = 3;
}

/// One connected controller on the GMS page: its name (the device's own, or the port's) and its status.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GmsRow {
    pub name: Vec<u8>,
    pub status: u8,
}

/// The GMS list's run-time state: cursor, first row shown, the rows the engine filled, and the row ✕ was pressed on
/// this tick (taken by the engine).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GmsList {
    pub cursor: i32,
    pub top: i32,
    pub rows: Vec<GmsRow>,
    pub pick: Option<usize>,
}

/// The value after `v` among `n` values that `choices` allows (wrapping); `v` when no other is allowed.
fn next_choice(v: u8, n: usize, choices: u32) -> u8 {
    (1..=n).map(|k| (v as usize + k) % n.max(1)).find(|&i| choices & (1 << i) != 0).unwrap_or(v as usize) as u8
}

fn list(m: &PageMenu) -> Option<&PortList> {
    match m.widgets.get(&LIST_W).map(|w| &w.data) {
        Some(Data::Port(p)) => Some(p),
        _ => None,
    }
}

fn list_mut(m: &mut PageMenu, w: u32) -> Option<&mut PortList> {
    match m.widgets.get_mut(&w).map(|w| &mut w.data) {
        Some(Data::Port(p)) => Some(p),
        _ => None,
    }
}

fn gms_mut(m: &mut PageMenu, w: u32) -> Option<&mut GmsList> {
    match m.widgets.get_mut(&w).map(|w| &mut w.data) {
        Some(Data::Gms(g)) => Some(g),
        _ => None,
    }
}

/// The row of a setting in [`ENTRIES`].
pub fn row_of(s: Setting) -> Option<usize> { ENTRIES.iter().position(|e| e.setting == s) }

/// A list widget of the port pages (the callbacks dispatch on `update` / `draw`).
fn list_widget(addr: u32, update: u32, draw: u32, data: Data) -> Widget {
    Widget { addr, update, draw, enter: 0, leave: 0, dflags: 0, moby: 0, rect: [0; 4], raw: [0; 8], data }
}

impl PageMenu {
    /// Adds the page, its widgets and the Options entry (module docs). `ov` is read for the description
    /// label's id table. False (nothing changed) when the Options page, its list or a model page is missing.
    pub fn install_port_page(&mut self, ov: &Overlay) -> bool {
        if self.pages.contains_key(&PAGE) { return true; }
        let ad = self.addrs.clone();
        let model = if ENTRIES.len() <= 2 { ad.port_model_few } else { ad.port_model_many };
        let (Some(model), true) = (self.pages.get(&model).cloned(), self.pages.contains_key(&ad.port_options)) else { return false };
        let Some(Data::List(opts)) = self.widgets.get(&ad.port_options_list).map(|w| &w.data) else { return false };
        let n = opts.items.len();
        let at = opts.items.iter().position(|it| it.action == 3 && it.arg == ad.port_quit).unwrap_or(n);
        // The title: the model's title label (flags 0xf, large, centred) with the port title.
        let Some(mut title) = self.widgets.get(&model.widgets[0]).cloned() else { return false };
        let Data::Label(l) = &mut title.data else { return false };
        l.id = text::TITLE as u32;
        title.addr = TITLE_W;
        // The GMS page's title: the same label with its own text, in the regular font (flag 8 off): "Gadgetron Movement
        // System" in the large one wraps to two lines the box cannot hold, which turns the label into a scroller; 0x2000
        // keeps it still.
        let mut gms_title = title.clone();
        if let Data::Label(l) = &mut gms_title.data {
            l.id = text::GMS_TITLE as u32;
            l.flags = (l.flags & !8) | 0x2000;
        }
        gms_title.addr = GMS_TITLE_W;
        let settings = list_widget(
            LIST_W,
            UPDATE,
            DRAW,
            Data::Port(PortList { cursor: 0, top: 0, values: vec![0; ENTRIES.len()], choices: vec![u32::MAX; ENTRIES.len()] }),
        );
        let gms = list_widget(GMS_LIST_W, GMS_UPDATE, GMS_DRAW, Data::Gms(GmsList::default()));
        let page = self.port_page(PAGE, &model, ad.port_options);
        let gms_page = self.sub_page(GMS_PAGE, &model, PAGE, GMS_TITLE_W, GMS_LIST_W);
        // The Options description label: the disc table (one id per item, variant 0) with ours inserted.
        if let Some(Data::Label(l)) = self.widgets.get_mut(&ad.port_options_label).map(|w| &mut w.data) {
            let mut t: Vec<u32> = (0..n as u32).map(|k| ov.u32(l.id.wrapping_add(k.wrapping_mul(l.stride) & !3)).unwrap_or(0)).collect();
            t.insert(at, text::DESCRIPTION as u32);
            l.table = Some(t);
        }
        if let Some(Data::List(opts)) = self.widgets.get_mut(&ad.port_options_list).map(|w| &mut w.data) {
            opts.items.insert(at, Item { label: text::ENTRY as i16, action: 3, arg: PAGE, sublabel: 0, hl: 0 });
        }
        self.widgets.insert(TITLE_W, title);
        self.widgets.insert(LIST_W, settings);
        self.widgets.insert(GMS_TITLE_W, gms_title);
        self.widgets.insert(GMS_LIST_W, gms);
        self.pages.insert(PAGE, page);
        self.pages.insert(GMS_PAGE, gms_page);
        self.install_front_port_page(ad.port_front_options);
        true
    }

    /// A port page modelled on `model`: its frames and fillers, the model's title label replaced by [`TITLE_W`] and
    /// its list (widget 3) by [`LIST_W`].
    fn port_page(&self, addr: u32, model: &Page, parent: u32) -> Page { self.sub_page(addr, model, parent, TITLE_W, LIST_W) }

    /// A page modelled on `model` (its frames and fillers) with the title label `title` and the list `list` (widget 3).
    /// The GMS page uses the Port Options page's model too, so its frames play from that page's layout and back.
    fn sub_page(&self, addr: u32, model: &Page, parent: u32, title: u32, list: u32) -> Page {
        let mut widgets = model.widgets;
        if let Some(t) = widgets.iter_mut().find(|w| matches!(self.widgets.get(w).map(|x| &x.data), Some(Data::Label(_)))) { *t = title; }
        widgets[3] = list;
        Page { addr, seqs: model.seqs, parent, kind: KIND, focus: list, widgets, pending: 0 }
    }

    /// [`PAGE_FRONT`] and its entry in the front end's Options lists (nothing when the front end's pages are missing).
    fn install_front_port_page(&mut self, front_options: u32) {
        if !self.pages.contains_key(&front_options) { return; }
        let sound = self.saves.front_options.iter().flatten().find(|it| it.action == 3 && it.label == FRONT_SOUND_LABEL).map(|it| it.arg);
        let Some(model) = sound.and_then(|a| self.pages.get(&a)).cloned() else { return };
        let page = self.port_page(PAGE_FRONT, &model, front_options);
        let gms_page = self.sub_page(GMS_PAGE_FRONT, &model, PAGE_FRONT, GMS_TITLE_W, GMS_LIST_W);
        self.pages.insert(PAGE_FRONT, page);
        self.pages.insert(GMS_PAGE_FRONT, gms_page);
        for list in self.saves.front_options.iter_mut() {
            if !list.iter().any(|it| it.arg == PAGE_FRONT) { list.push(Item { label: text::ENTRY as i16, action: 3, arg: PAGE_FRONT, sublabel: 0, hl: 0 }); }
        }
    }

    /// The value index of a port setting (None without the page).
    pub fn port_value(&self, s: Setting) -> Option<u8> { list(self)?.values.get(row_of(s)?).copied() }

    /// Sets a port setting's value index (from the engine's current value), clamped to the row's values.
    pub fn set_port_value(&mut self, s: Setting, v: u8) {
        let Some(r) = row_of(s) else { return };
        let max = ENTRIES[r].values.len().saturating_sub(1) as u8;
        if let Some(slot) = list_mut(self, LIST_W).and_then(|p| p.values.get_mut(r)) { *slot = v.min(max); }
    }

    /// Restricts a port setting to some of its values (bit k = value index k; ✕ skips the others). Value 0 stays
    /// selectable whatever the mask, so a row always has a choice.
    pub fn set_port_choices(&mut self, s: Setting, mask: u32) {
        let Some(r) = row_of(s) else { return };
        if let Some(p) = list_mut(self, LIST_W) {
            if p.choices.len() < ENTRIES.len() { p.choices.resize(ENTRIES.len(), u32::MAX); }
            p.choices[r] = mask | 1;
        }
    }

    /// The GMS page's rows (the connected controllers, from the engine), the cursor kept on the list.
    pub fn set_gms_rows(&mut self, rows: Vec<GmsRow>) {
        if let Some(g) = gms_mut(self, GMS_LIST_W) {
            let n = (GMS_SETTINGS + rows.len()) as i32;
            g.cursor = g.cursor.clamp(0, n - 1);
            g.top = g.top.clamp(0, (n - VISIBLE as i32).max(0));
            g.rows = rows;
        }
    }

    /// The GMS row ✕ was pressed on since the last call.
    pub fn take_gms_pick(&mut self) -> Option<usize> { gms_mut(self, GMS_LIST_W)?.pick.take() }
}

/// The list update, as the camera list 0x294cc0.
pub fn update(m: &mut PageMenu, w: u32, inp: &MenuInput, out: &mut MenuOut) -> i32 {
    if !m.is_focus(w) { return 0; }
    if let Some(r) = keys(m, inp, 1) { return r; }
    let Some(p) = list_mut(m, w) else { return 0 };
    let start = p.cursor;
    if inp.pressed_u & button::UP != 0 && p.cursor != 0 { p.cursor -= 1; }
    if inp.pressed_u & button::DOWN != 0 && (p.cursor as usize + 1) < PORT_ROWS { p.cursor += 1; }
    if p.cursor != start { out.sounds.push(MenuSound::Cursor); }
    p.top = p.top.clamp(p.cursor - VISIBLE as i32 + 1, p.cursor).max(0);
    let r = p.cursor as usize;
    let opens_gms = ENTRIES.get(r).is_some_and(|e| e.setting == Setting::Controller);
    if inp.pressed_u & button::CROSS != 0 && !opens_gms {
        let choices = p.choices.get(r).copied().unwrap_or(u32::MAX);
        if let (Some(e), Some(v)) = (ENTRIES.get(r), p.values.get_mut(r)) {
            *v = next_choice(*v, e.values.len(), choices);
            out.sounds.push(MenuSound::Confirm);
        }
    }
    if inp.pressed_u & button::CROSS != 0 && opens_gms {
        // The Controller row opens the GMS page (the main menu's copy from the main menu's Port Options).
        m.target = if m.current == PAGE_FRONT { GMS_PAGE_FRONT } else { GMS_PAGE };
        out.sounds.push(MenuSound::Confirm);
    }
    0
}

/// The list draw, as the camera list 0x294e68 (panel-local).
pub fn draw(m: &mut PageMenu, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 1 };
    let [_, _, ww, wh] = wd.rect;
    let Data::Port(p) = &wd.data else { return 1 };
    // The camera list's spacing for the rows shown (h/(n+1), n = VISIBLE).
    let shown = PORT_ROWS.min(VISIBLE);
    let step = wh / (shown as i32 + 1);
    let mut y = step - 8;
    let top = (p.top.max(0) as usize).min(PORT_ROWS - shown);
    for (i, e) in ENTRIES.iter().enumerate().take(PORT_ROWS).skip(top).take(shown) {
        let col = if i as i32 == p.cursor { YELLOW } else { LIGHT_BLUE };
        print(out, Font::Regular, 0xc, y, col, a.msg(e.label));
        value(out, a, ww, y, e, p.values.get(i).copied().unwrap_or(0));
        y += step;
    }
    arrows(out, a, ww, wh, step, scroll_marks(top, shown));
    2
}

/// A row's value, right-aligned; nothing for a row without values (the Controller row opens a page).
fn value(out: &mut Vec<MenuDraw>, a: &MenuAssets, ww: i32, y: i32, e: &Entry, v: u8) {
    if let Some(&id) = e.values.get(v as usize) { right(out, a, ww - 0xc, y, LIGHT_BLUE, a.msg(id)); }
}

/// Rows hidden above / below: the Epilogue's arrow (`media::epilogue_arrows`: icon 0xe99e frame 6, 0x20 × 0x10
/// texels), half its size, centred between the panel's edge and the first / last row shown.
fn arrows(out: &mut Vec<MenuDraw>, a: &MenuAssets, ww: i32, wh: i32, step: i32, (more_above, more_below): (bool, bool)) {
    let frame = a.frame(0xe99e, 6);
    let (cx, gap) = (ww as f32 / 2.0, (step - 8) as f32 / 2.0);
    // At angle 0 the arrow points right (`rotated`: a quarter clockwise); ±π/2 turn it to point up / down.
    let half = std::f32::consts::FRAC_PI_2;
    if more_above { out.push(super::map_page::rotated(frame, cx, gap.max(ARROW_W), ARROW_W, ARROW_H, half, 0x20, 0x10)); }
    if more_below { out.push(super::map_page::rotated(frame, cx, (wh as f32 - gap).min(wh as f32 - ARROW_W), ARROW_W, ARROW_H, -half, 0x20, 0x10)); }
}

/// The Controller page's settings rows ([`ENTRIES`]`[PORT_ROWS..]`), before its controllers.
const GMS_SETTINGS: usize = ENTRIES.len() - PORT_ROWS;

/// The Controller page's list update: Up / Down over its settings and then its controllers; ✕ on a setting cycles
/// its value (kept in the Port Options list, where the engine reads it), on a controller reports it to the engine
/// ([`GmsList::pick`]), which locks or frees it.
pub fn gms_update(m: &mut PageMenu, w: u32, inp: &MenuInput, out: &mut MenuOut) -> i32 {
    if !m.is_focus(w) { return 0; }
    if let Some(r) = keys(m, inp, 1) { return r; }
    let Some(g) = gms_mut(m, w) else { return 0 };
    let total = GMS_SETTINGS + g.rows.len();
    let start = g.cursor;
    if inp.pressed_u & button::UP != 0 && g.cursor != 0 { g.cursor -= 1; }
    if inp.pressed_u & button::DOWN != 0 && (g.cursor as usize + 1) < total { g.cursor += 1; }
    if g.cursor != start { out.sounds.push(MenuSound::Cursor); }
    g.top = g.top.clamp(g.cursor - VISIBLE as i32 + 1, g.cursor).max(0);
    if inp.pressed_u & button::CROSS == 0 { return 0; }
    let row = g.cursor as usize;
    if row >= GMS_SETTINGS {
        g.pick = Some(row - GMS_SETTINGS);
        out.sounds.push(MenuSound::Confirm);
        return 0;
    }
    let r = PORT_ROWS + row;
    if let Some(p) = list_mut(m, LIST_W) {
        let choices = p.choices.get(r).copied().unwrap_or(u32::MAX);
        if let Some(v) = p.values.get_mut(r) {
            *v = next_choice(*v, ENTRIES[r].values.len(), choices);
            out.sounds.push(MenuSound::Confirm);
        }
    }
    0
}

/// The Controller page's list draw: its settings (label left, value right), then each controller (name left, status
/// right), as the Port Options rows.
pub fn gms_draw(m: &mut PageMenu, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
    let values = list(m).map(|p| p.values.clone()).unwrap_or_default();
    let Some(wd) = m.widgets.get(&w) else { return 1 };
    let [_, _, ww, wh] = wd.rect;
    let Data::Gms(g) = &wd.data else { return 1 };
    let n = GMS_SETTINGS + g.rows.len();
    let shown = n.min(VISIBLE);
    let step = wh / (shown.max(1) as i32 + 1);
    let mut y = step - 8;
    let top = (g.top.max(0) as usize).min(n - shown);
    for i in (0..n).skip(top).take(shown) {
        let col = if i as i32 == g.cursor { YELLOW } else { LIGHT_BLUE };
        if let Some(e) = ENTRIES.get(PORT_ROWS + i).filter(|_| i < GMS_SETTINGS) {
            print(out, Font::Regular, 0xc, y, col, a.msg(e.label));
            value(out, a, ww, y, e, values.get(PORT_ROWS + i).copied().unwrap_or(0));
        } else if let Some(row) = g.rows.get(i - GMS_SETTINGS) {
            print(out, Font::Regular, 0xc, y, col, &row.name);
            let status = match row.status {
                gms_status::ACTIVE => text::ACTIVE,
                gms_status::LOCKED => text::LOCKED,
                gms_status::OFF => text::GMS_OFF,
                _ => text::AVAILABLE,
            };
            right(out, a, ww - 0xc, y, LIGHT_BLUE, a.msg(status));
        }
        y += step;
    }
    arrows(out, a, ww, wh, step, (top > 0, top + shown < n));
    2
}

/// The scroll arrows' half sizes (pixels; [`super::map_page::rotated`]'s `w` and `h`).
const ARROW_W: f32 = 6.0;
const ARROW_H: f32 = 10.0;

/// Whether Port Options rows are hidden above and below when `shown` rows from `top` are drawn.
pub fn scroll_marks(top: usize, shown: usize) -> (bool, bool) { (top > 0, top + shown < PORT_ROWS) }

#[cfg(test)]
mod tests {
    use super::{next_choice, scroll_marks, Setting, ENTRIES, PORT_ROWS, VISIBLE};

    #[test]
    fn scroll_marks_show_hidden_rows() {
        assert_eq!(scroll_marks(0, VISIBLE), (false, true));
        assert_eq!(scroll_marks(1, VISIBLE), (true, true));
        assert_eq!(scroll_marks(PORT_ROWS - VISIBLE, VISIBLE), (true, false));
    }

    #[test]
    fn controller_rows_are_on_the_controller_page() {
        // Port Options ends with the Controller row (no value: it opens a page); the controller settings follow.
        assert_eq!((ENTRIES[PORT_ROWS - 1].setting, ENTRIES[PORT_ROWS - 1].values.len()), (Setting::Controller, 0));
        let gms: Vec<Setting> = ENTRIES[PORT_ROWS..].iter().map(|e| e.setting).collect();
        assert_eq!(gms, vec![Setting::Vibration, Setting::VibrationStrength, Setting::FireButton, Setting::TriggerEffects]);
    }

    #[test]
    fn cycle_skips_unsupported_values() {
        // Off / 2x / 4x / 8x with 8x unsupported: 4x → Off.
        assert_eq!(next_choice(0, 4, 0b0111), 1);
        assert_eq!(next_choice(2, 4, 0b0111), 0);
        // WebGPU-only (off / 4x): off → 4x → off.
        assert_eq!(next_choice(0, 4, 0b0101), 2);
        assert_eq!(next_choice(2, 4, 0b0101), 0);
        // All allowed: plain wrap; nothing else allowed: stays.
        assert_eq!(next_choice(3, 4, u32::MAX), 0);
        assert_eq!(next_choice(0, 4, 0b0001), 0);
    }
}
