//! The pause pages' widgets beyond the lists, labels, grids and models (docs/plan/menus.md §10, §12): the widget
//! types several pages share and the per-page ones, each ported as the game has it (addresses level01.elf, all
//! boot-hash-matched engine code).
//!
//! **Shared widget types**
//! * **Streamed image** (update 0x2937d0, draw 0x293d50, enter 0x2936e8, leave 0x293780): a picture from a global TOC
//!   lump (`+0x30` = the TOC field's address, 0x137b80 + offset: `item_images`, `help_ss`, `help_moves`,
//!   `help_weapons`, `help_gadgets`, `goodies_images`, `skill_images`, `options_ss`), `+0x34` flags, `+0x38 / +0x3c` the
//!   picture's size in texels. The index by the flags: 4 the focused grid's cursor, 8 the focused grid's cursor cell's
//!   +8, 0x100 the memory-card slot preview (not ported: the Save / Load pages), else the focused list's cursor (0x4000:
//!   an entry with action 2 → 9); then 0x2000: a skill point not earned → 0x1e. (Flags 1 / 2 / 0x400 / 0x1000 serve
//!   the front end's pages only.) Two buffers (+0x48 / +0x4c) streamed in turn (+0x44: 0 load A, 1 reading, 2 A
//!   shown, 3 reading B, 4 B shown, 5 reading A, 6 → 2); `+0x50 / +0x54` the index in each. The draw (state ≥ 2)
//!   maps the shown buffer's `+0x38 × +0x3c` texels over the whole panel target (return 0x10); flag 4 draws only
//!   while the cell's item is owned (kind 0) or its global flag set (kind 1). Port: the disc read completes on the
//!   next update [L]; the buffers are the engine's image cache (`MenuDraw::Image`).
//! * **Icon list** (update 0x28d818, draw 0x28d9a8): a vertical list of 32×32 item icons (38 px apart, 1/16-pixel
//!   positions), Up / Down without wrap (class sound 1), scrolled to keep the cursor in view, arrows (59806 frame 6)
//!   when entries are above / below. Its entries are built by the Help / Weapons and Help / Gadgets title enters
//!   (0x295000 / 0x2950c8) from the owned items of the Weapons page grid and the Gadgets page grids, with the labels'
//!   text tables (item definition +0x40 / +0x42 gold / +0x44).
//!
//! **Per-page widgets**
//! * Quick Select: the slot ring (update 0x2901a8, draw 0x294528, enter 0x2903d0, leave 0x290468): a copy of the
//!   quick-select slots 0x141ea0 (+0x30..+0x4c) and a slot cursor +0x50 (the first empty slot at the enter); R1 / L1
//!   step it (mod 8, class sound 1); ✕ on the focused grid's owned item bumps move record 21 (0x1418f0), moves the item
//!   out of any other slot into this one and steps the cursor. **The leave writes the slots back to 0x141ea0** (the
//!   NTSC writer the quick-select ring report looked for).
//! * Weapons: the ammo text (update 0x28f7a0, draw 0x292b60): "ammo/max" (thousands as `%d,%03d`) or "(no ammo)" of
//!   the focused cell's owned item, regular font centred in the panel.
//! * Items: the gold-bolt panel (enter 0x292450, draw 0x292528, leave 0x2924f8): the gold bolt moby (class 0x46e at
//!   the menu camera + (8, 0.5, −0.1), turned −1.9 about x) and "Gold Bolts" with Found / Used / Remain (the gold
//!   bolts collected 0x14bec0, 4 per gold weapon 0x13e520, the rest), return 8 (centre crop).
//! * Help / Controls (update 0x294050, draw 0x294198): the two pictures `help_controls[lang]` and `[6 + lang]` side by
//!   side (256×256 each), return 8.
//! * Help / Help Log: the list's enter 0x290b70 (the log newest first, each entry's title from 0x1798d0) and the text
//!   widget (enter 0x290c00, update 0x290d40, leave 0x290cd0) that streams `all_text` and swaps the message table to
//!   it while the page is up (hidden until loaded); its label (source 0x1000) shows the entry's help message.
//! * Help / Moves: the label enter 0x2904a0 (the Heli-Pack owned → table 0x1b4c50, else 0x1b4c88).
//! * Goodies / Skill Points: the level text 0x295af8 (the entry's level: its location and planet names, or "All
//!   Levels").
//! * Goodies / In-Level Movies: the list enter 0x295730 (the current level's list, table 0x1b8aa8[level % 19]).
//!
//! **Centre crop** (draw return 8): the panel shows the middle `w × h` of a `2^u × 2^v` target (≥ 128 each, u + v ≤ 17,
//! v reduced); the 2D draws here are shifted by that crop ([`crop`]).

use super::super::{sprite, text, ImageSrc, MenuAssets, MenuDraw, MenuInput, MenuSound};
use super::{gadgets, Data, Item, List, MenuOut, PageMenu, LIGHT_BLUE, SHADOW};
use crate::game_state::GameState;
use crate::hud::{text as wtext, Draw, Rot};
use crate::pad::button;
use rc_formats::font::Font;

/// Widget callbacks (level01 labels).
pub mod func {
    pub const IMAGE_UPDATE: u32 = 0x2937d0;
    pub const IMAGE_DRAW: u32 = 0x293d50;
    pub const IMAGE_ENTER: u32 = 0x2936e8;
    pub const IMAGE_LEAVE: u32 = 0x293780;
    pub const CONTROLS_UPDATE: u32 = 0x294050;
    pub const CONTROLS_DRAW: u32 = 0x294198;
    pub const SLOTS_UPDATE: u32 = 0x2901a8;
    pub const SLOTS_DRAW: u32 = 0x294528;
    pub const SLOTS_ENTER: u32 = 0x2903d0;
    pub const SLOTS_LEAVE: u32 = 0x290468;
    pub const AMMO_UPDATE: u32 = 0x28f7a0;
    pub const AMMO_DRAW: u32 = 0x292b60;
    pub const AMMO_MODEL_UPDATE: u32 = 0x2919a0;
    pub const AMMO_MODEL_DRAW: u32 = 0x291b18;
    pub const GOLD_DRAW: u32 = 0x292528;
    pub const GOLD_ENTER: u32 = 0x292450;
    pub const GOLD_LEAVE: u32 = 0x2924f8;
    pub const LOG_LIST_ENTER: u32 = 0x290b70;
    pub const LOG_LIST_LEAVE: u32 = 0x290bf8;
    pub const TEXT_UPDATE: u32 = 0x290d40;
    pub const TEXT_ENTER: u32 = 0x290c00;
    pub const TEXT_LEAVE: u32 = 0x290cd0;
    pub const ICONS_UPDATE: u32 = 0x28d818;
    pub const ICONS_DRAW: u32 = 0x28d9a8;
    pub const HELP_WEAPONS_ENTER: u32 = 0x295000;
    pub const HELP_GADGETS_ENTER: u32 = 0x2950c8;
    pub const MOVES_LABEL_ENTER: u32 = 0x2904a0;
    /// `fun_00225ac0(1)`: the stream buffers' layout (memory only: nothing to port).
    pub const STREAM_LAYOUT_ENTER: u32 = 0x2904e8;
    pub const SKILL_DRAW: u32 = 0x295af8;
    pub const MOVIES_ENTER: u32 = 0x295730;
}

/// The TOC's load address (`field` = widget +0x30 − this).
pub const TOC_BASE: u32 = 0x137b80;
/// `help_controls`' TOC field offset (0x137e48).
pub const HELP_CONTROLS: u32 = 0x2c8;
/// Menu icon 59806 (0xe99e): frame 0 the Quick Select page's shoulder tabs, frame 6 the icon list's arrow.
pub const MENU_ICON: u16 = 0xe99e;

/// Level-01 data the per-page enters read (resolved per level in `Addrs`).
pub mod data {
    /// The Weapons page grid's cells (+6 = the item of cell 0).
    pub const WEAPON_CELLS: u32 = 0x1b36a0;
    /// The Gadgets page grids' cells (their item fields: 6 + 3 + 3 + 2 cells).
    pub const GADGET_ITEMS: [u32; 4] = [0x1b31a6, 0x1b31aa, 0x1b31ac, 0x1b31ae];
    /// The Help / Weapons label table, the Help / Gadgets title and text tables.
    pub const WEAPON_TEXTS: u32 = 0x1ba6f8;
    pub const GADGET_TEXTS_A: u32 = 0x1ba7c8;
    pub const GADGET_TEXTS_B: u32 = 0x1ba800;
    /// The Moves label's two id tables.
    pub const MOVES_HELI: u32 = 0x1b4c50;
    pub const MOVES_NO_HELI: u32 = 0x1b4c88;
    /// The In-Level Movies lists by level.
    pub const MOVIE_LISTS: u32 = 0x1b8aa8;
}

// ---------------------------------------------------------------------------------------------------
// Shared helpers

/// The render-target size of a `w × h` panel: `2^u × 2^v`, each ≥ 128, `u + v ≤ 17` (v reduced).
pub fn target_size(w: i32, h: i32) -> (i32, i32) {
    let lg = |n: i32| (n.max(128) as u32).next_power_of_two().trailing_zeros();
    let (u, mut v) = (lg(w), lg(h));
    while u + v > 17 && v > 7 { v -= 1; }
    (1 << u, 1 << v)
}

/// The shift of a centre-cropped (return 8) panel's target-pixel draws: `(w − tw) / 2, (h − th) / 2`. While such a
/// panel draws, `SetRenderToTextureView` (0x219ba0) has set the frame size 0x13e500 / 0x13e504 to the target's
/// ([`target_size`]): a draw that centres on "the screen" centres on the target.
pub fn crop(w: i32, h: i32) -> (i32, i32) {
    let (tw, th) = target_size(w, h);
    ((w - tw) / 2, (h - th) / 2)
}

/// `fun_00200e08(x0, y0, x1, y1, rgba, 1)`: a rect in 1/16 pixels.
fn rect16(out: &mut Vec<MenuDraw>, x0: i32, y0: i32, x1: i32, y1: i32, rgba: u32) { out.push(MenuDraw::Rect { x0: x0 >> 4, y0: y0 >> 4, x1: x1 >> 4, y1: y1 >> 4, rgba }); }

/// The selected cell's pulsing frame colour: `(|(vsync & 0x3f) − 0x20| + 0x40) · 0x10202 | 0x80000000`.
pub fn pulse(vsync: u32) -> u32 { ((((vsync & 0x3f) as i32 - 0x20).abs() + 0x40) as u32).wrapping_mul(0x10202) | 0x8000_0000 }

/// The page's focused widget.
fn focus(m: &PageMenu) -> u32 { m.pages.get(&m.current).map_or(0, |p| p.focus) }

/// The focused list's cursor (`+0x40`) and its items.
fn focused_list(m: &PageMenu) -> Option<&List> {
    match &m.widgets.get(&focus(m))?.data {
        Data::List(l) => Some(l),
        _ => None,
    }
}

/// The focused grid-like widget's `(cursor, count, cells)` (the grid, or the icon list).
fn focused_cells(m: &PageMenu) -> Option<(i32, i32, Vec<gadgets::Cell>)> {
    match &m.widgets.get(&focus(m))?.data {
        Data::Grid(g) => Some((g.cursor, g.rows, g.cells.clone())),
        Data::IconList(l) => Some((l.cursor, l.cells.len() as i32, l.cells.clone())),
        _ => None,
    }
}

fn owned(gs: &GameState, item: i32) -> bool { usize::try_from(item).ok().and_then(|i| gs.global.owned.get(i)).is_some_and(|&b| b != 0) }

// ---------------------------------------------------------------------------------------------------
// The streamed image

/// The streamed image widget's fields (+0x30..+0x5c).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Image {
    /// +0x30: the TOC field's offset (`+0x30` − 0x137b80).
    pub field: u32,
    pub flags: u32,
    /// +0x38 / +0x3c: the picture's texels.
    pub tw: i32,
    pub th: i32,
    /// +0x40: flag 0x1000's index pointer (the Epilogue pager's +0x54); +0x58: flag 1's explicit index.
    pub ptr: u32,
    pub explicit: i32,
    /// +0x44 state, +0x50 / +0x54 buffer A's / B's index, +0x5c ticks.
    pub state: i32,
    pub a: i32,
    pub b: i32,
    pub t: i32,
}

impl Image {
    pub fn read(raw: &[u32; 8]) -> Image {
        Image { field: raw[0].wrapping_sub(TOC_BASE), flags: raw[1], tw: raw[2] as i32, th: raw[3] as i32, ptr: raw[4], explicit: -1, state: -1, a: -1, b: -1, t: 0 }
    }
}

/// The number of entries of the TOC field at `field` (`rc_formats::disc::RAC1_GLOBAL_FIELDS`); 0 unknown.
/// The widget reads entry `index` at TOC `field + 8·index` whatever the field's own count, so the fields right after it
/// count too (the Epilogue's languages: `epilogue_english` 0xbc8 then `_french`, `_italian`, `_german`, `_spanish`).
pub fn field_entries(field: u32) -> u32 {
    let fields = rc_formats::disc::RAC1_GLOBAL_FIELDS;
    let Some(mut i) = fields.iter().position(|f| f.offset as u32 == field) else { return 0 };
    let mut n = fields[i].count as u32;
    while let Some(next) = fields.get(i + 1).filter(|g| g.offset as u32 == fields[i].offset as u32 + 8 * fields[i].count as u32) {
        n += next.count as u32;
        i += 1;
    }
    n
}

/// `fun_0021fce0` (0x2936e8): state (+0x44) 0, both indices −1 (the buffers are the engine's). The Controls widget
/// uses the same enter / leave: its update reads the same +0x44.
pub fn image_enter(m: &mut PageMenu, w: u32) {
    match m.widgets.get_mut(&w).map(|x| &mut x.data) {
        Some(Data::Image(i)) => {
            i.t = 0;
            i.state = 0;
            i.a = -1;
            i.b = -1;
        }
        Some(Data::Controls { state }) => *state = 0,
        _ => {}
    }
}

/// `fun_0021fd78` (0x293780): state −1.
pub fn image_leave(m: &mut PageMenu, w: u32) {
    match m.widgets.get_mut(&w).map(|x| &mut x.data) {
        Some(Data::Image(i)) => {
            i.state = -1;
            i.a = -1;
            i.b = -1;
        }
        Some(Data::Controls { state }) => *state = -1,
        _ => {}
    }
}

/// `fun_0021fdc8` (0x2937d0).
pub fn image_update(m: &mut PageMenu, w: u32, gs: &GameState) -> i32 {
    let Some(Data::Image(img)) = m.widgets.get(&w).map(|x| &x.data) else { return 0 };
    let mut img = *img;
    img.t += 1;
    let f = img.flags;
    // Flags 1, 2, 0x400 and 0x1000 (an explicit index, the planet, a slideshow, a language table) are the front
    // end's; no in-level page uses them.
    let mut idx: i32;
    if f & 1 != 0 {
        // An explicit index (+0x58; −1: no picture, nothing done).
        idx = img.explicit;
        if idx == -1 {
            store(m, w, img);
            return 0;
        }
    } else if f & 2 != 0 {
        // The planet 0x184894.
        idx = m.dest;
    } else if f & 4 != 0 {
        idx = focused_cells(m).map_or(0, |c| c.0);
    } else if f & 0x100 != 0 {
        // The memory-card slot pages: the focused slot list's cursor (+0x40, 0..4) → its preview's level, the planet
        // picture of that level; nothing while the card works, for an empty slot or without a PS2 card.
        let slot = m.widgets.get(&focus(m)).map_or(0, |x| x.raw[4] as i32).clamp(0, 4);
        let c = &m.saves.card;
        if !c.idle() {
            img.state = -1;
            store(m, w, img);
            return 0;
        }
        if img.state == -1 { img.state = 0; }
        idx = c.card.previews[slot as usize].level;
        if idx == -1 { img.state = -1; }
        if c.card.ty != crate::memcard::TYPE_PS2 {
            img.state = -1;
            store(m, w, img);
            return 0;
        }
    } else if f & 8 != 0 {
        let Some((cursor, count, cells)) = focused_cells(m) else { return 0 };
        idx = cells.get(cursor.max(0) as usize).map_or(0, |c| c.image as i32);
        if count == 0 {
            img.state = -1;
            store(m, w, img);
            return 0;
        }
    } else if f & 0x400 != 0 {
        // A slideshow: picture (ticks / ticks(300)) % 19 (+0x5c, counted above).
        idx = (img.t / crate::menus::scale_ticks(300).max(1)) % 0x13;
    } else if f & 0x1000 != 0 {
        // A language table (the Epilogue): *(+0x40) (the pager's page, `media::epilogue_pager`) + the language's first
        // entry (0x20a2d0[0x15ed88]: English 0, French 12, German 36, Spanish 48, Italian 24) in the fields that follow.
        idx = super::media::pager_page(m, img.ptr) + m.media.lang_offsets.get(m.lang as usize).copied().unwrap_or(0);
    } else {
        let l = focused_list(m);
        idx = l.map_or(0, |l| l.cursor).max(0);
        if f & 0x4000 != 0 && l.and_then(|l| l.items.get(idx as usize)).is_some_and(|it| it.action == 2) { idx = 9; }
    }
    if f & 0x2000 != 0 && gs.global.skill_points.get(idx.max(0) as usize).is_none_or(|&b| b == 0) { idx = 0x1e; }
    let available = |i: i32| 0 <= i && (i as u32) < field_entries(img.field);
    match img.state {
        0 => {
            if available(idx) {
                img.a = idx;
                img.state = 1;
            }
        }
        1 | 3 | 5 => img.state += 1,
        4 => {
            if idx != img.b {
                if idx == img.a {
                    img.state = 2;
                } else if available(idx) {
                    img.a = idx;
                    img.state = 5;
                }
            }
        }
        6 | 2 => {
            if img.state == 6 { img.state = 2; }
            if idx != img.a {
                if idx == img.b {
                    img.state = 4;
                } else if available(idx) {
                    img.b = idx;
                    img.state = 3;
                }
            }
        }
        _ => {}
    }
    store(m, w, img);
    0
}

fn store(m: &mut PageMenu, w: u32, img: Image) {
    if let Some(Data::Image(i)) = m.widgets.get_mut(&w).map(|x| &mut x.data) { *i = img; }
}

/// `DrawCheckingMemoryCardDataMenu` (0x293d50, Lombyte's name): the shown buffer over the panel (return 0x10).
pub fn image_draw(m: &mut PageMenu, w: u32, gs: &GameState, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 1 };
    let [_, _, ww, wh] = wd.rect;
    let Data::Image(img) = wd.data else { return 1 };
    if img.state < 2 { return 1; }
    let shown = if img.state < 4 { img.a } else { img.b };
    if img.flags & 4 != 0 {
        let Some((_, _, cells)) = focused_cells(m) else { return 1 };
        let Some(c) = cells.get(shown.max(0) as usize) else { return 1 };
        let set = if c.kind == 0 { owned(gs, c.item as i32) } else { gs.global.flags.get(c.item.max(0) as usize).is_some_and(|&b| b != 0) };
        if !set { return 1; }
    }
    out.push(MenuDraw::Image { src: ImageSrc::Lump { field: img.field, index: shown as u32 }, x: 0, y: 0, w: ww, h: wh, u: 0, v: 0, tw: img.tw, th: img.th, rgba: 0x8080_8080 });
    0x10
}

// ---------------------------------------------------------------------------------------------------
// Help / Controls

/// `advance_audio_stream_state` (0x294050, Lombyte's name): read `help_controls[lang]` (state 0 → 1 → 2), then
/// `[6 + lang]` (2 → 3 → 4).
pub fn controls_update(m: &mut PageMenu, w: u32, lang: u32) -> i32 {
    let Some(Data::Controls { state }) = m.widgets.get_mut(&w).map(|x| &mut x.data) else { return 0 };
    let n = field_entries(HELP_CONTROLS);
    *state = match *state {
        0 if lang < n => 1,
        0 => -1,
        1 => 2,
        2 if 6 + lang < n => 3,
        2 => -1,
        3 => 4,
        s => s,
    };
    0
}

/// `draw_two_texture_panels` (0x294198): state 4 → the two pictures at (0, 0) and (256, 0), 256×256 each; return 8.
pub fn controls_draw(m: &mut PageMenu, w: u32, lang: u32, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 0 };
    let Data::Controls { state } = wd.data else { return 0 };
    if state < 4 { return 0; }
    let (dx, dy) = crop(wd.rect[2], wd.rect[3]);
    for (k, x) in [(lang, 0), (6 + lang, 0x100)] {
        out.push(MenuDraw::Image { src: ImageSrc::Lump { field: HELP_CONTROLS, index: k }, x: x + dx, y: dy, w: 0x100, h: 0x100, u: 0, v: 0, tw: 0x100, th: 0x100, rgba: 0x8080_8080 });
    }
    8
}

// ---------------------------------------------------------------------------------------------------
// Quick Select

/// `0x2903d0`: the slots copied from 0x141ea0; the cursor on the first empty slot (0 when all are full).
pub fn slots_enter(m: &mut PageMenu, w: u32, gs: &GameState) {
    let Some(Data::Slots { slots, cursor }) = m.widgets.get_mut(&w).map(|x| &mut x.data) else { return };
    *slots = gs.global.quick_select;
    let mut c = 0i32;
    if slots[0] != 0 {
        loop {
            c += 1;
            if c >= 8 || slots[c as usize] == 0 { break; }
        }
    }
    *cursor = c.rem_euclid(8);
}

/// `0x290468`: the slots back to 0x141ea0.
pub fn slots_leave(m: &mut PageMenu, w: u32, gs: &mut GameState) {
    if let Some(Data::Slots { slots, .. }) = m.widgets.get(&w).map(|x| &x.data) { gs.global.quick_select = *slots; }
}

/// `fun_0021c7a0` (0x2901a8).
pub fn slots_update(m: &mut PageMenu, w: u32, inp: &MenuInput, gs: &mut GameState, out: &mut MenuOut) -> i32 {
    let item = gadgets::focused_item(m).unwrap_or(0);
    let Some(Data::Slots { slots, cursor }) = m.widgets.get_mut(&w).map(|x| &mut x.data) else { return 0 };
    let old = *cursor;
    if inp.pressed_u & button::R1 != 0 { *cursor = (*cursor + 1).rem_euclid(8); }
    if inp.pressed_u & button::L1 != 0 { *cursor = (*cursor + 7).rem_euclid(8); }
    if *cursor != old { out.sounds.push(MenuSound::Cursor); }
    if item != 0 && owned(gs, item) && inp.pressed_u & button::CROSS != 0 {
        let (level, t) = (gs.global.level, gs.global.play_time);
        crate::help::bump(&mut gs.global.move_help[21], level, t);
        if let Some(k) = slots.iter().position(|&s| s == item) { slots[k] = 0; }
        slots[*cursor as usize] = item;
        *cursor = (*cursor + 1).rem_euclid(8);
    }
    0
}

/// `fun_00220b20` (0x294528): the ring of 8 slots (the cursor's in a pulsing frame), the shoulder tabs, return 2.
pub fn slots_draw(m: &mut PageMenu, w: u32, a: &MenuAssets, gs: &GameState, vsync: u32, out: &mut Vec<MenuDraw>) -> u32 {
    let navy = m.consts.navy;
    let icons: Vec<u16> = (0..0x25).map(|i| m.items.as_ref().and_then(|d| d.get(i)).map_or(0, |d| d.icon)).collect();
    let Some(wd) = m.widgets.get(&w) else { return 1 };
    let [_, _, ww, wh] = wd.rect;
    let Data::Slots { slots, cursor } = wd.data else { return 1 };
    let (cx, cy) = (ww as f32 * 0.5, wh as f32 * 0.5);
    let r = ww.min(wh) as f32 * 0.5 - 40.0;
    for (i, &item) in slots.iter().enumerate() {
        // FastNormalizeAngle(i·π/4 − π/2): already in [−π, π].
        let ang = i as f32 * std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_2;
        let (x, y) = ((cx + ang.cos() * r) as i32, (cy + ang.sin() * r) as i32);
        if i as i32 == cursor {
            out.push(MenuDraw::Rect { x0: x - 0x13, y0: y - 0x13, x1: x + 0x13, y1: y + 0x13, rgba: pulse(vsync) });
            out.push(MenuDraw::Rect { x0: x - 0x12, y0: y - 0x12, x1: x + 0x12, y1: y + 0x12, rgba: navy });
        }
        if item == 0 {
            out.push(MenuDraw::Rect { x0: x - 0xf, y0: y - 0xf, x1: x + 0xf, y1: y + 0xf, rgba: 0x4040_4040 });
        } else {
            let gold = gs.global.gold_weapons.get(item as usize).is_some_and(|&b| b != 0);
            let f = a.frame(icons.get(item as usize).copied().unwrap_or(0), if gold { 4 } else { 0 });
            sprite(out, f, x - 0x11, y - 0x11, 0x20, 0x20, 0x80);
        }
    }
    let tab = a.frame(MENU_ICON, 0);
    sprite(out, tab, 8, 0x27, 0x20, -0x20, 0x80);
    sprite(out, tab, ww - 10, 0x27, -0x20, -0x20, 0x80);
    text(out, Font::Regular, 0x28, 0xf, LIGHT_BLUE, b"\x14");
    text(out, Font::Regular, ww - 0x3c, 0xf, LIGHT_BLUE, b"\x15");
    2
}

// ---------------------------------------------------------------------------------------------------
// Weapons: the ammo text

/// `fun_0021f158` (0x292b60).
pub fn ammo_draw(m: &mut PageMenu, w: u32, a: &MenuAssets, gs: &GameState, records: &[(u16, u16)], out: &mut Vec<MenuDraw>) -> u32 {
    let item = gadgets::focused_item(m).unwrap_or(0);
    if !owned(gs, item) { return 0; }
    let Some(wd) = m.widgets.get(&w) else { return 0 };
    let [_, _, ww, wh] = wd.rect;
    let (has_ammo, max) = records.get(item as usize).copied().unwrap_or((0, 0));
    let ammo = gs.global.ammo.get(item as usize).copied().unwrap_or(0);
    let s: Vec<u8> = if has_ammo == 0 {
        a.msg(0x4f52).to_vec()
    } else {
        let n = |v: i32| if v < 1000 { format!("{v}") } else { format!("{},{:03}", v / 1000, v % 1000) };
        format!("{}/{}", n(ammo), n(max as i32)).into_bytes()
    };
    let win = wtext::Window::new(0, wh as i16, 0, ww as i16, (ww >> 1) as i16, (wh >> 1) as i16, 0x10, wtext::CENTRE_LINES | wtext::CENTRE_BLOCK);
    out.push(MenuDraw::Hud(Draw::TextWindow { font: Font::Regular, window: win, rgba: LIGHT_BLUE, text: s }));
    2
}

// ---------------------------------------------------------------------------------------------------
// Items: the gold-bolt panel

/// `count_nonzero_entries_up_to_40` / `_up_to_10` / `compute_clamped_count_difference`.
pub fn gold_counts(gs: &GameState) -> (i32, i32, i32) {
    let found = (gs.levels.iter().take(20).map(|l| l.gold_bolts.iter().filter(|&&b| b != 0).count()).sum::<usize>() as i32).clamp(0, 0x28);
    let weapons = (gs.global.gold_weapons.iter().take(0x25).filter(|&&b| b != 0).count() as i32).clamp(0, 10);
    (found, weapons * 4, (found - weapons * 4).clamp(0, 0x28))
}

/// `DrawItemsMenu` (0x292528): "Gold Bolts" (0x4f4e, a space after the '-' in German) in a window over the left third,
/// Found / Used / Remain right-aligned at x 200 and their counts (large) right-aligned at x 240, black shadows at
/// (+2, +2), the rule under the second count; return 8 (the gold bolt moby is the engine's: `GadgetsView::gold_bolt`).
pub fn gold_draw(m: &mut PageMenu, w: u32, a: &MenuAssets, gs: &GameState, lang: u32, out: &mut Vec<MenuDraw>) -> u32 {
    let sh = m.consts.shadow;
    let Some(wd) = m.widgets.get(&w) else { return 8 };
    let [_, _, ww, wh] = wd.rect;
    let (dx, dy) = crop(ww, wh);
    let mut t = a.msg(0x4f4e).to_vec();
    if lang == 3 {
        if let Some(i) = t.iter().position(|&c| c == b'-') { t.insert(i + 1, b' '); }
    }
    // Window {y 0..h, x 8..w/3, anchor (8 + w/3)/2, line 16, flags 5 (measure)}, then drawn at y = (H − height)/2.
    let mut win = wtext::Window::new(0, wh as i16, 8, (ww / 3) as i16, ((8 + ww / 3) / 2) as i16, 0, 0x10, wtext::CENTRE_LINES | wtext::MEASURE_ONLY);
    wtext::layout(&mut win, &t, -1, &a.hud.glyphs[Font::Regular as usize], true);
    // H = 0x13e504, the target's height while the panel draws (`SetRenderToTextureView`).
    let th = target_size(ww, wh).1;
    win.y_start = ((th - win.height as i32) >> 1) as i16;
    win.flags ^= wtext::MEASURE_ONLY;
    let mv = |w0: wtext::Window| {
        let mut w1 = w0;
        w1.x_min += dx as i16;
        w1.x_max += dx as i16;
        w1.x_anchor += dx as i16;
        w1.y_min += dy as i16;
        w1.y_max += dy as i16;
        w1.y_start += dy as i16;
        w1
    };
    out.push(MenuDraw::Hud(Draw::TextWindow { font: Font::Regular, window: mv(win), rgba: 0x8000_c0c0, text: t }));
    let right = |out: &mut Vec<MenuDraw>, font: Font, x: i32, y: i32, rgba: u32, s: &[u8]| text(out, font, x - a.width(font, s) + dx, y + dy, rgba, s);
    let rows = [(0x4f4f, 0x1d), (0x4f50, 0x36), (0x4f51, 0x54)];
    for &(id, y) in &rows { right(out, Font::Regular, sh.0 + 200, sh.1 + y, SHADOW, a.msg(id)); }
    for &(id, y) in &rows { right(out, Font::Regular, 200, y, LIGHT_BLUE, a.msg(id)); }
    let (found, used, remain) = gold_counts(gs);
    for (v, y) in [(found, 0x1d), (used, 0x36), (remain, 0x54)] {
        let s = format!("{v}").into_bytes();
        right(out, Font::Large, sh.0 + 0xf0, sh.1 + y, SHADOW, &s);
        right(out, Font::Large, 0xf0, y, LIGHT_BLUE, &s);
    }
    out.push(MenuDraw::Rect { x0: sh.0 + 0xd0 + dx, y0: sh.1 + 0x4d + dy, x1: sh.0 + 0xf2 + dx, y1: sh.1 + 0x50 + dy, rgba: SHADOW });
    out.push(MenuDraw::Rect { x0: 0xd0 + dx, y0: 0x4d + dy, x1: 0xf2 + dx, y1: 0x50 + dy, rgba: LIGHT_BLUE });
    if let Some(spin) = m.gold_spin {
        m.view.gold_bolt = Some(gadgets::PreviewView { rect: [0; 4], item: 0, o_class: GOLD_BOLT_CLASS, clank: false, seq: 0, offset: GOLD_BOLT_OFFSET, rot: [spin, GOLD_BOLT_PITCH, 0.0] });
        if let (Some(v), Some(wd)) = (m.view.gold_bolt.as_mut(), m.widgets.get(&w)) { v.rect = wd.rect; }
    }
    8
}

// ---------------------------------------------------------------------------------------------------
// Help / Help Log

/// `0x290b70`: the list items from the help log, newest first: `{title of 0x1798d0[log[n − 1 − k]], action 1}`.
pub fn log_list_enter(m: &mut PageMenu, w: u32, gs: &GameState, log_ids: &[(i16, i16)]) {
    let n = gs.global.help_log_pos.clamp(0, gs.global.help_log.len() as i32) as usize;
    let Some(Data::List(l)) = m.widgets.get_mut(&w).map(|x| &mut x.data) else { return };
    l.items = (0..n).map(|k| Item { label: log_ids.get(gs.global.help_log[n - 1 - k] as usize).map_or(0, |e| e.1), action: 1, ..Item::default() }).collect();
    l.items.retain(|it| it.label != 0);
    // Port-only: the Gadgetron Movement System note, last (the oldest), never in the saved log.
    if m.gms_in_log { l.items.push(Item { label: super::port::text::GMS_TITLE as i16, action: 1, ..Item::default() }); }
}

/// `fun_0021d1f8` (0x290c00): hidden (draw flags |= 4) while `all_text` streams (+0x50 = 1).
pub fn text_enter(m: &mut PageMenu, w: u32) {
    m.text_swap = 1;
    if let Some(wd) = m.widgets.get_mut(&w) { wd.dflags |= 4; }
}

/// `MenuTextLoad` (0x290d40): the read done (the next update [L]) → the message table swapped to `all_text` (+0x50 = 2)
/// and the widget shown.
pub fn text_update(m: &mut PageMenu, w: u32) -> i32 {
    if m.text_swap == 1 {
        m.text_swap = 2;
        if let Some(wd) = m.widgets.get_mut(&w) { wd.dflags &= !4; }
    }
    0
}

/// `MenuTextSwap` (0x290cd0): the level's table back.
pub fn text_leave(m: &mut PageMenu) { m.text_swap = 0; }

// ---------------------------------------------------------------------------------------------------
// The icon list (Help / Weapons, Help / Gadgets)

/// The icon list's fields: +0x3c cursor, +0x40 count, +0x48 cells, +0x5c the first entry's y (1/16 px), +0x60 the top
/// entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IconList {
    pub cursor: i32,
    pub cells: Vec<gadgets::Cell>,
    pub y: i32,
    pub top: i32,
}

/// `fun_00219e10` (0x28d818).
pub fn icons_update(m: &mut PageMenu, w: u32, inp: &MenuInput, gs: &GameState, out: &mut MenuOut) -> i32 {
    if let Some(r) = m.generic_keys(inp, false, gs.global.level) { return r; }
    let Some(wd) = m.widgets.get_mut(&w) else { return 0 };
    let h = wd.rect[3];
    let Data::IconList(l) = &mut wd.data else { return 0 };
    let old = l.cursor;
    let n = l.cells.len() as i32;
    if inp.pressed_u & button::UP != 0 && l.cursor != 0 { l.cursor -= 1; }
    if inp.pressed_u & button::DOWN != 0 && l.cursor + 1 < n { l.cursor += 1; }
    if l.cursor != old { out.sounds.push(MenuSound::Cursor); }
    let h16 = h * 16;
    l.y = if h16 / 0x260 < n {
        if l.cursor <= l.top { l.top = (l.cursor - 1).max(0); }
        let lo = l.cursor - ((h16 - 0x28) / 0x260 - 2);
        if l.top < lo { l.top = lo; }
        l.top * -0x260 + 400
    } else {
        0x60
    };
    0
}

/// `fun_00219fa0` (0x28d9a8).
pub fn icons_draw(m: &mut PageMenu, w: u32, a: &MenuAssets, vsync: u32, out: &mut Vec<MenuDraw>) -> u32 {
    let navy = m.consts.navy;
    let focused = focus(m) == w;
    let Some(wd) = m.widgets.get(&w) else { return 1 };
    let [_, _, ww, wh] = wd.rect;
    let Data::IconList(l) = &wd.data else { return 1 };
    let x = (ww * 16 - 0x200) >> 1;
    let mut y = l.y;
    for (i, c) in l.cells.iter().enumerate() {
        if i as i32 == l.cursor {
            rect16(out, x - 0x30, y - 0x30, x + 0x230, y + 0x230, pulse(vsync));
            rect16(out, x - 0x10, y - 0x10, x + 0x210, y + 0x210, navy);
        }
        let f = a.frame(c.icon, c.variant as i32);
        let (tw, th) = a.frame_size(f);
        out.push(MenuDraw::SpriteUv { frame: f, x0: x, y0: y, x1: x + 0x200, y1: y + 0x200, u0: 0, v0: 0, u1: tw * 16, v1: th * 16, alpha: 0x80, repeat_u: false });
        y += 0x260;
    }
    let _ = focused;
    let ax = (ww * 16 - 0x200) >> 5;
    let arrow = a.frame(MENU_ICON, 6);
    if l.y < 0 {
        out.push(MenuDraw::Rect { x0: 0, y0: 0, x1: ww, y1: 0x14, rgba: navy });
        sprite(out, arrow, ax, 2, 0x20, 0x10, 0x80);
    }
    if wh << 4 < y {
        out.push(MenuDraw::Rect { x0: 0, y0: wh - 0x14, x1: ww, y1: wh, rgba: navy });
        out.push(MenuDraw::Hud(Draw::Sprite { frame: arrow, x: ax, y: wh - 0x12, w: 0x20, h: 0x10, alpha: 0x80, rot: Rot::R180 }));
    }
    2
}

/// The Help / Weapons title enter (0x295000): the Weapons page grid's owned items (15 cells) as icon-list entries
/// `{icon, gold · 4, 0, item, index}` and the label table (gold ? +0x42 : +0x40).
pub fn help_weapons_enter(m: &mut PageMenu, gs: &GameState) {
    let items = m.help_items.weapons.clone();
    let mut cells = Vec::new();
    let mut texts = Vec::new();
    for (i, &item) in items.iter().enumerate() {
        if !owned(gs, item as i32) { continue; }
        let gold = gs.global.gold_weapons.get(item as usize).is_some_and(|&b| b != 0);
        let d = m.items.as_ref().and_then(|d| d.get(item as i32)).copied().unwrap_or_default();
        cells.push(gadgets::Cell { icon: d.icon, variant: if gold { 4 } else { 0 }, kind: 0, item, image: i as i16 });
        texts.push(if gold { d.help[1] } else { d.help[0] } as u16 as u32);
    }
    set_icon_list(m, cells);
    let t = m.addrs.help_texts[0];
    m.label_tables.insert(t, texts);
}

/// The Help / Gadgets title enter (0x2950c8): the Gadgets page grids' owned items (14) with the title (+0x40) and text
/// (+0x44) tables.
pub fn help_gadgets_enter(m: &mut PageMenu, gs: &GameState) {
    let items = m.help_items.gadgets.clone();
    let mut cells = Vec::new();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for (i, &item) in items.iter().enumerate() {
        if !owned(gs, item as i32) { continue; }
        let d = m.items.as_ref().and_then(|d| d.get(item as i32)).copied().unwrap_or_default();
        cells.push(gadgets::Cell { icon: d.icon, variant: 0, kind: 0, item, image: i as i16 });
        a.push(d.help[0] as u16 as u32);
        b.push(d.help[2] as u16 as u32);
    }
    set_icon_list(m, cells);
    let (ta, tb) = (m.addrs.help_texts[1], m.addrs.help_texts[2]);
    m.label_tables.insert(ta, a);
    m.label_tables.insert(tb, b);
}

fn set_icon_list(m: &mut PageMenu, cells: Vec<gadgets::Cell>) {
    let ws = m.pages.get(&m.current).or_else(|| m.pages.get(&m.target)).map(|p| p.widgets).unwrap_or([0; 14]);
    for w in ws {
        if let Some(Data::IconList(l)) = m.widgets.get_mut(&w).map(|x| &mut x.data) {
            l.cells = cells.clone();
            l.cursor = l.cursor.min(cells.len() as i32 - 1).max(0);
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// Moves, Skill Points, In-Level Movies

/// `0x2904a0`: the Moves description label's table by the Heli-Pack (owned[2]).
pub fn moves_label_enter(m: &mut PageMenu, w: u32, gs: &GameState) {
    let id = if owned(gs, 2) { m.addrs.moves_tables[0] } else { m.addrs.moves_tables[1] };
    if let Some(Data::Label(l)) = m.widgets.get_mut(&w).map(|x| &mut x.data) {
        l.id = id;
        l.timer = -1;
    }
}

/// `fun_002220f0` (0x295af8): the focused list entry's level (item +4): −1 → "All Levels" (0x5019) centred at
/// (w/2, h/2 − 8); else its location name at (w/2, h/3 − 8) and planet name at (w/2, 2h/3 − 8); return 2.
pub fn skill_draw(m: &mut PageMenu, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
    let lvl = focused_list(m).and_then(|l| l.items.get(l.cursor.max(0) as usize)).map_or(-1, |it| it.arg as i32);
    let names = m.level_names.clone();
    let Some(wd) = m.widgets.get(&w) else { return 2 };
    let [_, _, ww, wh] = wd.rect;
    let centre = |out: &mut Vec<MenuDraw>, x: i32, y: i32, s: &[u8]| text(out, Font::Regular, x - (a.width(Font::Regular, s) >> 1), y, LIGHT_BLUE, s);
    if lvl == -1 {
        centre(out, ww / 2, wh / 2 - 8, a.msg(0x5019));
    } else {
        let (loc, planet) = names.get(lvl.max(0) as usize).copied().unwrap_or((0, 0));
        centre(out, ww / 2, wh / 3 - 8, a.msg(loc as i32));
        centre(out, ww / 2, (wh << 1) / 3 - 8, a.msg(planet as i32));
    }
    2
}

/// `0x295730`: the list's items = the current level's In-Level Movies list (`0x1b8aa8[level % 19]`).
pub fn movies_enter(m: &mut PageMenu, w: u32, gs: &GameState) {
    let items = m.movie_lists.get((gs.global.level.max(0) % 19) as usize).cloned().unwrap_or_default();
    if let Some(Data::List(l)) = m.widgets.get_mut(&w).map(|x| &mut x.data) { l.items = items; }
}

// ---------------------------------------------------------------------------------------------------
// The 3D widgets: the Weapons page's ammo model and the Items page's gold bolt

/// The ammo model widget (0x2919a0 / 0x291b18): the moby at +0x44 of class `item +0x3a` (the ammo pickup), placed at the
/// menu camera + (6, 0, −0.3) and turned about z from π by 0.01 rad a tick (its update `fun_0021e1f8`); replaced (same
/// place and angle) when the focused item's class changes, freed when it has none.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AmmoModel {
    /// The moby's class (`+0xa6`), −1 none.
    pub class: i32,
    pub angle: f32,
}

impl Eq for AmmoModel {}

/// The ammo model's and the gold bolt's offsets from the menu camera and rotations (`fun_0021df98`, `fun_0021ea48`).
pub const AMMO_OFFSET: [f32; 3] = [6.0, 0.0, -0.3];
pub const GOLD_BOLT_CLASS: i32 = 0x46e;
pub const GOLD_BOLT_OFFSET: [f32; 3] = [8.0, 0.5, -0.1];
/// +0x44 = 0xbff33333.
pub const GOLD_BOLT_PITCH: f32 = -1.9;

/// `fun_0021df98` (0x2919a0) and the moby's own update (the turn), in the menu's moby loop order [L: the update's
/// turn is applied here, one per menu tick].
pub fn ammo_model_update(m: &mut PageMenu, w: u32) -> i32 {
    let item = gadgets::focused_item(m).unwrap_or(0);
    let class = m.items.as_ref().and_then(|d| d.get(item)).map_or(-1, |d| d.ammo_class as i32);
    let Some(Data::AmmoModel(am)) = m.widgets.get_mut(&w).map(|x| &mut x.data) else { return 0 };
    if class == -1 {
        am.class = -1;
    } else if am.class == -1 {
        am.class = class;
        am.angle = std::f32::consts::PI;
    } else if am.class != class {
        am.class = class;
    }
    if am.class != -1 { am.angle = crate::moby_update::creature::add_rot(am.angle, 0.01); }
    0
}

/// `fun_0021e110` (0x291b18): nothing unless the focused item is owned; its ammo model (the engine: `GadgetsView::ammo`,
/// return 8), or "Uses no ammo" (0x4f4d) centred (return 2).
pub fn ammo_model_draw(m: &mut PageMenu, w: u32, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) -> u32 {
    let item = gadgets::focused_item(m).unwrap_or(0);
    if !owned(gs, item) { return 0; }
    let Some(wd) = m.widgets.get(&w) else { return 0 };
    let [_, _, ww, wh] = wd.rect;
    let Data::AmmoModel(am) = wd.data else { return 0 };
    if am.class == -1 {
        let win = wtext::Window::new(0, wh as i16, 0, ww as i16, (ww >> 1) as i16, (wh >> 1) as i16, 0x10, wtext::CENTRE_LINES | wtext::CENTRE_BLOCK);
        out.push(MenuDraw::Hud(Draw::TextWindow { font: Font::Regular, window: win, rgba: LIGHT_BLUE, text: a.msg(0x4f4d).to_vec() }));
        return 2;
    }
    let rect = wd.rect;
    m.view.ammo = Some(gadgets::PreviewView { rect, item, o_class: am.class, clank: false, seq: 0, offset: AMMO_OFFSET, rot: [0.0, 0.0, am.angle] });
    8
}

/// `fun_0021ea48` (0x292450): the gold bolt moby (spun about x by 0.02 rad a tick: `fun_0021f120`).
pub fn gold_enter(m: &mut PageMenu, _w: u32) { m.gold_spin = Some(0.0); }

/// `fun_0021eaf0` (0x2924f8): the moby freed.
pub fn gold_leave(m: &mut PageMenu, _w: u32) { m.gold_spin = None; }

/// The gold bolt's turn, one per menu tick (its update in the menu's moby loop).
pub fn gold_tick(m: &mut PageMenu) {
    if let Some(a) = m.gold_spin.as_mut() { *a = crate::moby_update::creature::add_rot(*a, 0.02); }
}

// ---------------------------------------------------------------------------------------------------
// Load-time tables

/// The items the Help / Weapons (the Weapons page grid 0x1b36a0, 15 cells) and Help / Gadgets (the Gadgets page grids'
/// item fields 0x1b31a6 / 0x1b31aa / 0x1b31ac / 0x1b31ae: 6 + 3 + 3 + 2 cells) enters walk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelpItems {
    pub weapons: Vec<i16>,
    pub gadgets: Vec<i16>,
}

impl HelpItems {
    pub fn read(ov: &super::super::Overlay) -> HelpItems {
        let wc = ov.at(data::WEAPON_CELLS);
        let weapons = (0..15u32).map(|k| ov.i16(wc + 10 * k + 6).unwrap_or(0)).collect();
        let g = data::GADGET_ITEMS.map(|a| ov.at(a));
        let gadgets = (0..14u32)
            .map(|k| {
                let base = match k {
                    0..=5 => g[0],
                    6..=8 => g[1],
                    9..=11 => g[2],
                    _ => g[3],
                };
                ov.i16(base + 10 * k).unwrap_or(0)
            })
            .collect();
        HelpItems { weapons, gadgets }
    }
}

/// The In-Level Movies lists (`0x1b8aa8[level]`: item-list pointers, 19 levels).
pub fn read_movie_lists(ov: &super::super::Overlay) -> Vec<Vec<Item>> {
    let t = ov.at(data::MOVIE_LISTS);
    (0..19u32).map(|l| ov.u32(t + 4 * l).map_or_else(Vec::new, |p| super::read_items(ov, p))).collect()
}
