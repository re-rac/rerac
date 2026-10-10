//! **Port-only** (not in the game): the "Port Options" page, for settings the PS2 never had (the graphics options and
//! their preset, anti-aliasing, switching the moby shadows off, the aspect, the render resolution and fullscreen).
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
//! Records live at addresses no EE pointer can hold (0x7f00_xxxx), and the text ids are negative (the disc's
//! ids are 0..=21500), so nothing here can collide with or be mistaken for game data.
//!
//! **Adding an option**: one [`Entry`] in [`ENTRIES`] (a [`Setting`] variant, a label id and its value ids in
//! [`text`]); the engine maps the [`Setting`] to its resource and persistence. The engine can restrict a row to
//! some of its values ([`PageMenu::set_port_choices`], e.g. the sample counts the GPU supports): ✕ then skips
//! the others.

use super::super::{text as print, MenuAssets, MenuDraw, MenuInput, MenuSound, Overlay};
use super::options::{keys, right};
use super::{Data, Item, MenuOut, Page, PageMenu, Widget, LIGHT_BLUE, YELLOW};
use crate::pad::button;
use rc_formats::font::Font;

/// The page and its two own widgets (title label, settings list).
pub const PAGE: u32 = 0x7f00_0000;
pub const TITLE_W: u32 = 0x7f00_0100;
pub const LIST_W: u32 = 0x7f00_0200;
/// The list's callbacks (dispatched like the game's by address).
pub const UPDATE: u32 = 0x7f00_1000;
pub const DRAW: u32 = 0x7f00_1004;
/// The page kind (no game code keys on it).
pub const KIND: i32 = 0x7f;

/// Game records the page is attached to / modelled on (level01).
pub const OPTIONS: u32 = 0x1b4dd0;
pub const OPTIONS_LIST: u32 = 0x1b4f78;
pub const OPTIONS_LABEL: u32 = 0x1b4fe8;
pub const QUIT_PAGE: u32 = 0x1b6060;
/// Subtitles 0x1b5788 (seqs 73..77: the one/two-row sub-page frames) and Camera 0x1b5ee0 (118..122, three rows).
pub const MODEL_FEW: u32 = 0x1b5788;
pub const MODEL_MANY: u32 = 0x1b5ee0;

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
    /// World-camera multisampling: value 0 off, 1 2x, 2 4x, 3 8x.
    Msaa,
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
    Entry { setting: Setting::Msaa, label: text::ANTI_ALIASING, values: &[text::OFF, text::X2, text::X4, text::X8] },
    Entry { setting: Setting::Hud, label: text::HUD, values: &[text::ORIGINAL, text::SHARP_PIXELS] },
    Entry { setting: Setting::Shadows, label: text::SHADOWS, values: &[text::ON, text::OFF] },
    Entry { setting: Setting::Aspect, label: text::ASPECT, values: &[text::ASPECT_4_3, text::ASPECT_16_10, text::ASPECT_16_9] },
    Entry { setting: Setting::Resolution, label: text::RESOLUTION, values: &[text::RES_WINDOW, text::RES_416, text::RES_720, text::RES_1080, text::RES_1440, text::RES_2160] },
    Entry { setting: Setting::Fullscreen, label: text::FULLSCREEN, values: &[text::OFF, text::ON] },
];

/// The list's run-time state: cursor, each row's value index and each row's selectable values (bit k = value k;
/// a missing entry = all).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PortList {
    pub cursor: i32,
    pub values: Vec<u8>,
    pub choices: Vec<u32>,
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

/// The row of a setting in [`ENTRIES`].
pub fn row_of(s: Setting) -> Option<usize> { ENTRIES.iter().position(|e| e.setting == s) }

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
        let settings = Widget {
            addr: LIST_W,
            update: UPDATE,
            draw: DRAW,
            enter: 0,
            leave: 0,
            dflags: 0,
            moby: 0,
            rect: [0; 4],
            raw: [0; 8],
            data: Data::Port(PortList { cursor: 0, values: vec![0; ENTRIES.len()], choices: vec![u32::MAX; ENTRIES.len()] }),
        };
        let mut widgets = model.widgets;
        widgets[0] = TITLE_W;
        widgets[3] = LIST_W;
        let page = Page { addr: PAGE, seqs: model.seqs, parent: ad.port_options, kind: KIND, focus: LIST_W, widgets, pending: 0 };
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
        self.pages.insert(PAGE, page);
        true
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
}

/// The list update, as the camera list 0x294cc0.
pub fn update(m: &mut PageMenu, w: u32, inp: &MenuInput, out: &mut MenuOut) -> i32 {
    if !m.is_focus(w) { return 0; }
    if let Some(r) = keys(m, inp, 1) { return r; }
    let Some(p) = list_mut(m, w) else { return 0 };
    let start = p.cursor;
    if inp.pressed_u & button::UP != 0 && p.cursor != 0 { p.cursor -= 1; }
    if inp.pressed_u & button::DOWN != 0 && (p.cursor as usize + 1) < ENTRIES.len() { p.cursor += 1; }
    if p.cursor != start { out.sounds.push(MenuSound::Cursor); }
    let r = p.cursor as usize;
    if inp.pressed_u & button::CROSS != 0 {
        let choices = p.choices.get(r).copied().unwrap_or(u32::MAX);
        if let (Some(e), Some(v)) = (ENTRIES.get(r), p.values.get_mut(r)) {
            *v = next_choice(*v, e.values.len(), choices);
            out.sounds.push(MenuSound::Confirm);
        }
    }
    0
}

/// The list draw, as the camera list 0x294e68 (panel-local).
pub fn draw(m: &mut PageMenu, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 1 };
    let [_, _, ww, wh] = wd.rect;
    let Data::Port(p) = &wd.data else { return 1 };
    let step = wh / (ENTRIES.len() as i32 + 1);
    let mut y = step - 8;
    for (i, e) in ENTRIES.iter().enumerate() {
        let col = if i as i32 == p.cursor { YELLOW } else { LIGHT_BLUE };
        print(out, Font::Regular, 0xc, y, col, a.msg(e.label));
        let v = p.values.get(i).copied().unwrap_or(0) as usize;
        right(out, a, ww - 0xc, y, LIGHT_BLUE, a.msg(e.values.get(v).copied().unwrap_or(0)));
        y += step;
    }
    2
}

#[cfg(test)]
mod tests {
    use super::next_choice;

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
