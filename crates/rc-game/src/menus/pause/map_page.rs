//! The map page (Select / R3 → `EnterMenuMode(10)` → page 0x1b3998, kind 0xb; docs/plan/menus.md §13): the map widget
//! (update 0x28f868 after its keys [`super::planet_select::map_update`], draw 0x292d38 → `UNK_NoMapAvailable`
//! 0x25b1c0, leave 0x28f7a8), the legend 0x292d70, and the per-open setup `FUN_0025a8d0` (0x28c128's first tick).
//! The map system itself (files, mask, fog writer) is [`crate::map`]; its live state is moved into [`MapPage::state`]
//! while the menu is open (the engine's `menu_render`) and back when it closes.
//!
//! Coordinates: the game draws the map in 1/16 pixels of the widget's panel (0x800 = 128 px down, 0x1000 = 256 px
//! across the view point); the port converts to panel pixels when it emits the draws.
//!
//! **The missions page** (0x1b3bf8, ✕ on the map; its missions widget also sits on the ship's confirm page 0x1b6878)
//! and **the streamed pictures** of the missions page and the ship's planet select (G-UI-002):
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x20bc00 `fun_0020bc00(names, mask, pictures, descs)` | the destination's mission list (0x1a2c20): every record without flag 2 and with status ≠ 0 (flag 1: not when done) is listed: its name, or with `descs` "Completed" 0x523e when done else its description by the callback's value (+0x14 + 2·value); bit k of the mask when done; its picture +0x12 + value; bit 31 when every record without flag 2 is done; returns the count | [`mission_list`] |
//! | 0x28fe28 missions enter | `FUN_00262760` (the destination's status); +0x7c = the count; the destination's cursor (+0x30 + 4·dest) ≠ −1: the description label 0x1b3dc0's id (0x1b3df4) and the picture widget 0x1b3c80's index (0x1b3cd8) | [`missions_enter`] |
//! | 0x28fec8 missions update | unfocused: the destination's cursor = −1; focused: the close keys, R1 / L1 the next / previous known planet (sound 1, `FUN_00262760`), Up / Down the cursor mod the count (sound 1 on a change), on Up / Down or a new planet the count, the label and the picture again; △ back | `planet_select::missions_update` |
//! | 0x293090 `DrawMissionsMenu2` | "Missions" 0x4f59 (large, (4, 4)); per listed mission from y 0x18: its name wrapped in the small font at x 0x10 within w − 0x11 (`fun_001f6fd0`; the cursor's line yellow 0x8020ffff with the colour codes off, else 0x80ffa888), its check box (`draw_menu_selection_marker`) at (9, y + 10) or (9, y + 0x10) for two lines, ticked when done; y += 16 per line; all done: 0x523d in the small font, window y 0..h, x 0..w, anchor h / 2, from y + 8, centred lines, 0x8020ffff; return 2 | [`missions_draw`] (the wrap: [`wrap_small`]) |
//! | 0x293398 picture update | the index by flags: 1 the explicit +0x58 (−1: nothing), 2 the destination, 4 the focused widget's +0x3c, 0x100 the card slots', else the focused widget's +0x40 (≥ 0); +0x44 0 / 2 (shown): a new index whose TOC entry is not empty → the read (0x1516d8 free, the buffer, +0x50 = the index, +0x44 + 1; refused: −1); 1 / 3: the read done → WAD-decompressed with flag 0x20, the PIF converted (`fun_00204e30`) → 2 | [`picture_update`] (the read is the engine's and never pends: done the next frame [L]; flag 0x100: no widget of this callback has it, n/a) |
//! | 0x293670 picture draw | +0x44 ≥ 2: `DrawTexturedQuad(0, 0, panel w, panel h, 0, 0, +0x38, +0x3c)` of the converted picture, return 0x10; else 0 | [`picture_draw`] |
//! | 0x2936e8 / 0x293780 | the image widgets' enter / leave (+0x44 = 0 / −1, the indices −1) | [`picture_enter`] / [`picture_leave`] |
//! | 0x292980 `DrawGBsShipMenu` | the confirm page's gold bolt (the Items page's moby, enter 0x292450 / leave 0x2924f8) placed at the menu camera + (8, *gp−0x689c = 1.3, −0.1) and drawn; "%s %d %s %d" of 0x4f4f, the destination's gold bolts found (0x14bec0 + 4·dest, 4 bytes), 0x4f53 and its total (0x1c4e08 + 4·dest) right-aligned at (w − 0x10, H / 2 − 8) in black and (w − 0x11, H / 2 − 9) in 0x80ffa888; return 8 | [`gold_panel_draw`] |

use super::super::{MenuAssets, MenuDraw, MenuInput, QuadTex};
use super::{Data, MenuOut, PageMenu, LIGHT_BLUE, YELLOW};
use crate::game_state::GameState;
use crate::hud::{text as wtext, Draw};
use crate::map::{self, MapFile, Mask};
use rc_formats::font::Font;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Loads map file `index` (the engine's `disc_source` read of `global/unknown_0820/NNN.bin`, decompressed).
#[derive(Clone, Default)]
pub struct Loader(pub Option<Arc<dyn Fn(usize) -> Option<MapFile> + Send + Sync>>);

impl std::fmt::Debug for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Loader({})", self.0.is_some()) }
}

/// The hero as the draw reads it (0x13f3d0 position, 0x13f3e8 yaw, 0x1413dc group).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeroMark {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub group: i32,
}

/// The page's state (0x184694.. and the loaded map system).
#[derive(Clone, Debug, Default)]
pub struct MapPage {
    /// The live map system (moved in for the menu's lifetime).
    pub state: map::MapState,
    pub loader: Loader,
    cache: BTreeMap<usize, Option<MapFile>>,
    /// 0x184694: the current level has a map (the setup), 0x184898: the level shown (−2: compose on the next
    /// update, −1: none).
    pub available: bool,
    pub shown: i32,
    /// The composed picture of `shown` (the stream buffer the game uploads as the 512 × 512 texture).
    pub picture: Option<Arc<rc_formats::texture::Texture>>,
    /// Pictures composed so far (the image cache's key).
    pub composed: u64,
    pub hero: HeroMark,
    /// 0x15edb4: the mirror cheat.
    pub mirror: bool,
    /// The globe's three 128 × 128 layers of the shown level (the map file's PIFs [4..6]: `DAT_001848c8 / d0 / d8`).
    pub layers: [Option<Arc<rc_formats::texture::Texture>>; 3],
    /// 0x1c24e8: the globe's margin per level (1/16 px; 0 on the stations 6 / 13 / 17).
    pub radii: Vec<i32>,
    /// The mission and marker lists (level01 overlay), the destination's missions (0x1871a0) and markers (0x184690).
    pub tables: MapTables,
    pub missions: Vec<Mission>,
    pub markers: Vec<Marker>,
    /// The current level's hook mobys (0x179638: slot → x, y, angle), given by the engine when the menu opens.
    pub hooks: std::collections::HashMap<i32, (f32, f32, f32)>,
    /// Per level, the CLUT of the grid icon's frame (0xe999, frame = the level): the draw's TEX0 takes its CBP from
    /// `GetFrameTex(GetIconFrame(0xe999, shown))`, so the picture is drawn with that palette (the CLUT the compose
    /// uploads to 0x3ff0 is never used). Set by the engine from the level's HUD; empty → the file's first PIF palette
    /// [L].
    pub palettes: Vec<Option<Vec<u8>>>,
}

/// The globe margins' table (level01 address).
pub const GLOBE_RADII: u32 = 0x1c24e8;

/// The map widget's draw panel: a 512 × 256 view at zoom 1 (the draw's 0x2000 × 0x1000 in 1/16 px).
pub const VIEW_CENTRE: (i32, i32) = (0x1000, 0x800);

/// Icons: the background grid cell (frame = the level) and the hero arrow (frame 5).
pub const GRID_ICON: u16 = 0xe999;
pub const ARROW_ICON: u16 = 0xe99a;

impl MapPage {
    /// The page's load-time data: the globe margins (0x1c24e8, 19 words).
    pub fn new(ov: &super::super::Overlay) -> MapPage {
        let t = ov.at(GLOBE_RADII);
        MapPage { radii: (0..map::LEVELS as u32).map(|l| ov.i32(t + 4 * l).unwrap_or(0)).collect(), shown: -1, tables: MapTables::read(ov), ..MapPage::default() }
    }

    fn file(&mut self, index: usize) -> Option<MapFile> {
        if let Some(f) = self.cache.get(&index) { return f.clone(); }
        let f = self.loader.0.as_ref().and_then(|l| l(index));
        self.cache.insert(index, f.clone());
        f
    }
}

/// `FUN_0025a8d0` (the page menu's first tick, every open): the map is available when the current level has one; a
/// Map-o-Matic picked up during the level switches the current level's picture to its set (the game streams that
/// file into the stash now; 0x1848a0 = 1); the shown level is reset so the next update composes.
pub fn setup(m: &mut PageMenu, gs: &GameState) {
    let p = &mut m.map;
    let owned = gs.global.owned.get(map::MAP_O_MATIC).is_some_and(|&b| b != 0);
    p.available = p.state.file.is_some();
    if p.available && !p.state.map_o_matic && owned { p.state.map_o_matic = true; }
    p.shown = -2;
    // FUN_00262da0(0x184894, 1): the markers, and the view centred on the hero.
    let dest = m.dest;
    setup_markers(m, gs, dest, true);
}

/// `EnterMenuMode`'s `FUN_00262760` (0x28c0ec): the destination's (= the current level's) mission status.
pub fn enter(m: &mut PageMenu, gs: &GameState) {
    let dest = m.dest;
    m.map.missions = m.map.tables.missions.get(dest.max(0) as usize).cloned().unwrap_or_default();
    mission_status(&mut m.map.missions, gs, dest);
}

/// The widget's key half on a destination change (0x28fa34): sound 1 and `FUN_00262760` for the new destination.
pub fn dest_changed(m: &mut PageMenu, gs: &GameState) { enter(m, gs); }

/// `fun_00205440` (0x25afc0, every update of the widget): unless Select / R3 was pressed, with a map shown, the right
/// stick's y zooms (×(1 − 0.02·ry), 0.65..4) and the left stick pans (3·10⁶ / zoom per unit), clamped to the
/// picture (x 0..0x10000000, y (1280 / zoom)·0x8000 .. 0x10000000 − that).
pub fn pan_zoom(m: &mut PageMenu, inp: &MenuInput) -> bool {
    let p = &mut m.map;
    if inp.pressed & 0x500 != 0 { return true; }
    if !p.available || p.shown < 0 { return false; }
    let Some(v) = p.state.view.get_mut(p.shown as usize) else { return false };
    let [ry, lx, ly] = [inp.sticks[1].to_f32(), inp.sticks[2].to_f32(), inp.sticks[3].to_f32()];
    v.zoom *= 1.0 - ry * 0.02;
    v.zoom = v.zoom.clamp(0.65, 4.0);
    let (dx, dy) = (lx * (3e6 / v.zoom), ly * (3e6 / v.zoom));
    v.pan[0] = v.pan[0].wrapping_add(dx as i32);
    v.pan[1] = v.pan[1].wrapping_add(dy as i32);
    let ey = (1280.0 / v.zoom) as i32 * 0x8000;
    let ex = (0.0 / v.zoom) as i32 * 0x8000;
    v.pan[0] = v.pan[0].clamp(ex, 0x1000_0000 - ex);
    v.pan[1] = v.pan[1].clamp(ey, 0x1000_0000 - ey);
    false
}

/// The second half of 0x28f868: when the destination 0x184894 is not the shown level, its map file (the Map-o-Matic
/// set while owned[33]; the current level's from the stash: its entry's set) with its mask — the live one for the
/// current level, else the level's saved mask when it was visited (0x13dd58), else its zone tiles — composed into the
/// picture; the markers of the new level (`FUN_00262da0(dest, 0)`).
pub fn compose(m: &mut PageMenu, gs: &GameState) {
    let dest = m.dest;
    if dest == m.map.shown || !(0..map::LEVELS as i32).contains(&dest) { return; }
    let owned = gs.global.owned.get(map::MAP_O_MATIC).is_some_and(|&b| b != 0);
    let cur = m.map.state.level;
    let index = if dest == cur { map::file_index(cur as usize, m.map.state.map_o_matic) } else { map::file_index(dest as usize, owned) };
    let Some(file) = m.map.file(index) else { return };
    let picture = if dest == cur && m.map.state.file.is_some() {
        map::compose(&file, &m.map.state.mask)
    } else {
        let d = dest as usize;
        let mask = match gs.levels.get(d) {
            Some(l) if l.visited != 0 => Mask::unpack(&l.map_mask, file.runs()),
            _ => Mask::initial(&file),
        };
        map::compose(&file, &mask)
    };
    let clut = m.map.palettes.get(dest as usize).cloned().flatten().unwrap_or_else(|| file.palette().to_vec());
    m.map.picture = rc_formats::hud::decode_indexed8_raw(&picture, map::SIZE as u32, map::SIZE as u32, &clut).ok().map(Arc::new);
    // The globe's layers (`fun_00204e30(7, 7, …)` ×3: every layer with the first PIF's palette).
    let pal = file.palette().to_vec();
    m.map.layers = std::array::from_fn(|k| file.pif(k).and_then(|p| rc_formats::hud::decode_indexed8_raw(p.pixels, p.width, p.height, &pal).ok()).map(Arc::new));
    m.map.composed += 1;
    m.map.shown = dest;
    setup_markers(m, gs, dest, false);
}

/// The view rectangle of the shown level in 1/16 panel pixels: (left, top, right, bottom) of the 512 × 512 picture
/// (`UNK_NoMapAvailable`: `0x1000 − z·(pan_x >> 15)` · mirror, `0x800 − z·(pan_y >> 15)`, width and height z·8192).
pub fn view_rect(p: &MapPage) -> Option<(i32, i32, i32, i32)> {
    let v = p.state.view.get(usize::try_from(p.shown).ok()?)?;
    let z = v.zoom;
    let s = if p.mirror { -1 } else { 1 };
    let top = VIEW_CENTRE.1 - (z * (v.pan[1] >> 15) as f32) as i32;
    let bottom = top + (z * 8192.0) as i32;
    let left = VIEW_CENTRE.0 - (z * (v.pan[0] >> 15) as f32) as i32 * s;
    let right = left + (z * 8192.0) as i32 * s;
    Some((left, top, right, bottom))
}

/// `UNK_NoMapAvailable` (0x25b1c0) through the widget draw 0x292d38 (return 8): the background grid (icon 0xe999,
/// frame = the shown level, one 32-texel cell per z·32 px, repeated over the panel from the picture's edges), the
/// picture, the markers, and on the current level the hero arrow (icon 0xe99a frame 5, at the hero's map point, turned
/// by his yaw (+π/2 in movement group 0xf and on level 6's alternative map), (((4z + 10)·0.75) / 13)·16 px square).
/// Without a map for the current level the game prints "No Map Available" in the debug font: every level 0..18 has
/// one, so it is not drawn here.
pub fn draw(m: &mut PageMenu, w: u32, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) -> u32 {
    let p = &m.map;
    if !p.available { return 8; }
    let Some((left, top, right, bottom)) = view_rect(p) else { return 8 };
    let Some(wd) = m.widgets.get(&w) else { return 8 };
    let (ww, wh) = (wd.rect[2], wd.rect[3]);
    let (dx, dy) = super::pages::crop(ww, wh);
    let px = |v: i32| v >> 4;
    let z = p.state.view[p.shown as usize].zoom;
    // The grid: tile = z·512 in 1/16 px; n tiles each side to cover 0..0x2000 × 0..0x1000 (+ one).
    let tile = (z * 512.0) as i32;
    if tile > 0 && !p.mirror {
        let n_l = (left + tile - 1) / tile;
        let n_t = (top + tile - 1) / tile;
        let n_r = ((tile + 0x2000) - right - 1) / tile;
        let n_b = ((tile + 0x2000) - bottom - 1) / tile;
        let (x0, y0) = (left - tile * n_l, top - tile * n_t);
        let (x1, y1) = (right + tile * n_r, bottom + tile * n_b);
        let frame = a.frame(GRID_ICON, p.shown);
        let (tw, th) = a.frame_size(frame);
        let (u1, v1) = ((n_l + n_r + 16) * tw, (n_t + n_b + 16) * th);
        out.push(MenuDraw::Quad {
            tex: QuadTex::Frame(frame),
            pos: [[px(x0 - 8) + dx, px(y0 - 8) + dy], [px(x1 - 8) + dx, px(y0 - 8) + dy], [px(x0 - 8) + dx, px(y1 - 8) + dy], [px(x1 - 8) + dx, px(y1 - 8) + dy]],
            uv: [[0, 0], [u1, 0], [0, v1], [u1, v1]],
            rgba: 0x8080_8080,
            repeat: true,
            uv16: false,
        });
    }
    if let Some(tex) = p.picture.clone() {
        let src = super::super::ImageSrc::Pixels { key: 0x6d61_7000_0000_0000 | p.composed, tex };
        let (xa, xb) = (px(left - 8) + dx, px(right - 8) + dx);
        let (ya, yb) = (px(top - 8) + dy, px(bottom - 8) + dy);
        out.push(MenuDraw::Quad {
            tex: QuadTex::Image(src),
            pos: [[xa, ya], [xb, ya], [xa, yb], [xb, yb]],
            uv: [[0, 0], [512, 0], [0, 512], [512, 512]],
            rgba: 0x8080_8080,
            repeat: false,
            uv16: false,
        });
    }
    draw_markers(p, a, gs, (left, top, right, bottom), dx, dy, out);
    if p.shown == p.state.level {
        let h = p.hero;
        let mut yaw = h.yaw;
        if h.group == 0xf { yaw += std::f32::consts::FRAC_PI_2; }
        if p.state.alt { yaw += std::f32::consts::FRAC_PI_2; }
        let (u, v) = map::world_to_map(&p.state.transforms, p.state.level, p.state.alt, h.pos[0], h.pos[1]);
        let cx = left as f32 + u * (right - left) as f32;
        let cy = top as f32 + v * (bottom - top) as f32;
        if p.mirror { yaw = -(yaw + std::f32::consts::FRAC_PI_2) - std::f32::consts::FRAC_PI_2; }
        let size = (((z * 4.0 + 10.0) * 0.75) / 13.0) * 256.0;
        let frame = a.frame(ARROW_ICON, 5);
        let (tw, th) = a.frame_size(frame);
        out.push(rotated(frame, cx / 16.0 + dx as f32, cy / 16.0 + dy as f32, size / 16.0, size / 16.0, yaw, tw, th));
    }
    8
}

/// `fun_00200600(cx, cy, w, h, angle, …)`: a frame drawn centred at (cx, cy), `w × h`, turned by `angle`.
#[allow(clippy::too_many_arguments)]
pub fn rotated(frame: usize, cx: f32, cy: f32, w: f32, h: f32, angle: f32, tw: i32, th: i32) -> MenuDraw {
    let (s, c) = angle.sin_cos();
    let corner = |x: f32, y: f32| [(cx + x * c - y * s).round() as i32, (cy + x * s + y * c).round() as i32];
    let (hw, hh) = (w * 0.5, h * 0.5);
    MenuDraw::Quad {
        tex: QuadTex::Frame(frame),
        pos: [corner(-hw, -hh), corner(hw, -hh), corner(-hw, hh), corner(hw, hh)],
        uv: [[0, 0], [tw, 0], [0, th], [tw, th]],
        rgba: 0x8080_8080,
        repeat: false,
        uv16: false,
    }
}

/// The legend (`DrawMissionsMenu` 0x292d70 on the map page, return 2): "View Missions", "Play Infobot" (not on level
/// 0), "Previous Map", "Next Map", "Track/Zoom", "Exit" in the small font with the colour codes off, one row per
/// h / n (n = the rows + 1: 6, or 7 with the Infobot row) from y = h / n − 6, at x = (w − the widest) / 2 (≥ 2).
pub fn legend_draw(m: &PageMenu, w: u32, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 2 };
    let (ww, wh) = (wd.rect[2], wd.rect[3]);
    let with_infobot = gs.global.level != 0;
    let ids: Vec<i32> = if with_infobot { vec![0x4eee, 0x4eef, 0x4efa, 0x4efb, 0x4efc, 0x4ee0] } else { vec![0x4eee, 0x4efa, 0x4efb, 0x4efc, 0x4ee0] };
    let widest = ids.iter().map(|&id| a.width(Font::Small, a.msg(id))).max().unwrap_or(0);
    let x = ((ww - widest) >> 1).max(2);
    let step = wh / (ids.len() as i32 + 1);
    for (k, &id) in ids.iter().enumerate() {
        super::super::text_plain(out, Font::Small, x, step - 6 + step * k as i32, 0x80ff_a888, a.msg(id));
    }
    2
}

/// The globe widget's enter (0x2904d0): 0x184898 = −1 (the map composes again on the next update).
pub fn globe_enter(m: &mut PageMenu) { m.map.shown = -1; }

/// The globe (0x294258, return 4): on a station (levels 6, 13, 17) the three layers over the whole target, the second
/// while `vsync % 60 < 40`, the third while `vsync % 150 < 90`; elsewhere the planet: the first layer inset by the
/// level's margin with its u scrolling ((vsync + level·0x2ab) mod 0x800 in 1/16 texels, REPEAT), the other two over
/// it. Drawn in the render target and copied with the aspect crop [L: the copy keeps the target's centre at the panel's
/// aspect, scaled to the panel].
pub fn globe_draw(m: &PageMenu, w: u32, vsync: u32, out: &mut Vec<MenuDraw>) -> u32 {
    let p = &m.map;
    if p.shown < 0 { return 0; }
    let Some(wd) = m.widgets.get(&w) else { return 0 };
    let (ww, wh) = (wd.rect[2], wd.rect[3]);
    let (tw, th) = super::pages::target_size(ww, wh);
    // Aspect crop: the largest centred region of the target with the panel's aspect, scaled onto the panel.
    let (cw, ch) = if tw * wh > th * ww { (th * ww / wh.max(1), th) } else { (tw, tw * wh / ww.max(1)) };
    let (ox, oy) = ((tw - cw) / 2, (th - ch) / 2);
    let sx = ww as f32 / cw.max(1) as f32;
    let sy = wh as f32 / ch.max(1) as f32;
    let to_panel = |x16: i32, y16: i32| [((x16 as f32 / 16.0 - ox as f32) * sx).round() as i32, ((y16 as f32 / 16.0 - oy as f32) * sy).round() as i32];
    let key = 0x676c_6f62_0000_0000u64 | (p.composed << 2);
    let layer = |k: usize, x: i32, y: i32, w16: i32, h16: i32, u16: i32, repeat: bool, out: &mut Vec<MenuDraw>| {
        let Some(tex) = p.layers[k].clone() else { return };
        let (a, b) = (to_panel(x, y), to_panel(x + w16, y + h16));
        let (u0, u1) = (u16, u16 + 128 * 16);
        out.push(MenuDraw::Quad {
            tex: QuadTex::Image(super::super::ImageSrc::Pixels { key: key | k as u64, tex }),
            pos: [[a[0], a[1]], [b[0], a[1]], [a[0], b[1]], [b[0], b[1]]],
            uv: [[u0, 0], [u1, 0], [u0, 128 * 16], [u1, 128 * 16]],
            rgba: 0x8080_8080,
            repeat,
            uv16: true,
        });
    };
    let (t16w, t16h) = (tw * 16, th * 16);
    if matches!(p.shown, 6 | 13 | 17) {
        layer(0, 0, 0, t16w, t16h, 0, false, out);
        if vsync % 60 < 40 { layer(1, 0, 0, t16w, t16h, 0, false, out); }
        if vsync % 150 < 90 { layer(2, 0, 0, t16w, t16h, 0, false, out); }
    } else {
        let r = p.radii.get(p.shown as usize).copied().unwrap_or(0);
        let t = vsync as i32 + p.shown * 0x2ab;
        let u = t.rem_euclid(0x800);
        layer(0, r, r, t16w - 2 * r, t16h - 2 * r, u, true, out);
        layer(1, r, r, t16w - 2 * r, t16h - 2 * r, 0, false, out);
        layer(2, r, r, t16w - 2 * r, t16h - 2 * r, 0, false, out);
    }
    4
}

// ---------------------------------------------------------------------------------------------------
// Missions and markers (`FUN_00262760`, `FUN_00262da0`, the marker part of `UNK_NoMapAvailable`)

/// The per-level mission lists (0x1870f0: 19 pointers to 0x28-byte records ending at a zero name) and marker lists
/// (0x187140: 19 pointers to 0x28-byte records ending at flags & 4), the rotated markers' sizes (gp−0x6de8..−0x6ddc)
/// and the price table 0x1c4530 (0x18-byte records, +0 the price) the labels' `%b` prints.
pub const MISSION_LISTS: u32 = 0x1870f0;
pub const MARKER_LISTS: u32 = 0x187140;
pub const MARKER_SIZES: u32 = 0x15fe18;
pub const PRICES: u32 = 0x1c4530;
/// 0x1c4e08: the gold bolts per level (`DrawGBsShipMenu` 0x292980).
pub const GOLD_TOTALS: u32 = 0x1c4e08;

/// One mission record (0x1870f0 lists).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mission {
    /// +0 name, +0x14 description (message ids).
    pub name: i16,
    pub desc: i16,
    /// +2 / +4 and +8 / +0xc: the two conditions (`fun_0020baf0` type, value).
    pub req: [(i16, u32); 2],
    /// +0x10: 2 not counted in "all done", 4 only once the level was visited.
    pub flags: u16,
    /// +0x1c / +0x20: the list's callback (level01 address) and its argument.
    pub cb: u32,
    pub arg: u32,
    /// +0x24: 0 unavailable, 1 open, 2 done; +0x26: the callback's value.
    pub status: i16,
    pub cb_value: i16,
    /// +0x12: the first of its pictures (`mission_ss`, + the callback's value); +0x14..+0x1a: its descriptions by the
    /// callback's value (`desc` = the first).
    pub picture: i16,
    pub descs: [i16; 4],
}

/// One map marker record (0x187140 lists).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Marker {
    /// +0: the hook slot (0x179638 on the current level, the landmark 0x13d5b0 elsewhere); −1..−9 fixed points.
    pub idx: i16,
    /// +2: the mission whose status 1 shows it (−1 always).
    pub link: i16,
    /// +4: 1 hidden, 2 not pushed apart, 4 end, 0x10 label box (set at the setup), 0x40 black backing, 0x80 ×1.5,
    /// 0x100 turned sprite, 0x200 sized by the zoom, 0x400 / 0x800 / 0x1000 the turned sizes (+π/2).
    pub flags: u16,
    /// +6 / +8: icon and frame; +0xa label message, +0xc price index; +0xe / +0x10 the label box, +0x12 / +0x14 its
    /// offsets.
    pub icon: i16,
    pub frame: i16,
    pub label: i16,
    pub price: i16,
    pub box_w: i16,
    pub box_h: i16,
    pub box_dx: i16,
    pub box_dy: i16,
    /// +0x18 / +0x1c: the map point (0..1), +0x20 the angle, +0x24 shown.
    pub u: f32,
    pub v: f32,
    pub rot: f32,
    pub shown: bool,
}

/// The mission and marker lists of the 19 levels (read from the level01 overlay: the same records in every overlay,
/// `tests/ui/map_levels.rs`), the turned markers' sizes and the prices.
#[derive(Clone, Debug, Default)]
pub struct MapTables {
    pub missions: Vec<Vec<Mission>>,
    pub markers: Vec<Vec<Marker>>,
    pub sizes: [f32; 6],
    pub prices: Vec<i32>,
    /// 0x1c4e08: the gold bolts of each level (20 words; the confirm page's gold-bolt panel 0x292980).
    pub gold_totals: Vec<i32>,
}

impl MapTables {
    pub fn read(ov: &super::super::Overlay) -> MapTables {
        let (ml, kl) = (ov.at(MISSION_LISTS), ov.at(MARKER_LISTS));
        let mut t = MapTables::default();
        for l in 0..map::LEVELS as u32 {
            let mut ms = Vec::new();
            if let Some(mut a) = ov.u32(ml + 4 * l).filter(|&p| p != 0) {
                while ov.i16(a).is_some_and(|n| n != 0) {
                    let (s, u) = (|o: u32| ov.i16(a + o).unwrap_or(0), |o: u32| ov.u32(a + o).unwrap_or(0));
                    ms.push(Mission {
                        name: s(0), desc: s(0x14), req: [(s(2), u(4)), (s(8), u(0xc))], flags: u(0x10) as u16, cb: u(0x1c), arg: u(0x20), status: 0, cb_value: 0,
                        picture: s(0x12), descs: [s(0x14), s(0x16), s(0x18), s(0x1a)],
                    });
                    a += 0x28;
                }
            }
            t.missions.push(ms);
            let mut ks = Vec::new();
            if let Some(mut a) = ov.u32(kl + 4 * l).filter(|&p| p != 0) {
                loop {
                    let s = |o: u32| ov.i16(a + o).unwrap_or(0);
                    let flags = s(4) as u16;
                    if flags & 4 != 0 || ks.len() > 64 { break; }
                    let f = |o: u32| f32::from_bits(ov.u32(a + o).unwrap_or(0));
                    ks.push(Marker {
                        idx: s(0), link: s(2), flags, icon: s(6), frame: s(8), label: s(0xa), price: s(0xc),
                        box_w: s(0xe), box_h: s(0x10), box_dx: s(0x12), box_dy: s(0x14), u: f(0x18), v: f(0x1c), rot: f(0x20), shown: false,
                    });
                    a += 0x28;
                }
            }
            t.markers.push(ks);
        }
        let sz = ov.at(MARKER_SIZES);
        t.sizes = std::array::from_fn(|k| ov.u32(sz + 4 * k as u32).map_or(0.0, f32::from_bits));
        let pr = ov.at(PRICES);
        t.prices = (0..64u32).map(|k| ov.i32(pr + 0x18 * k).unwrap_or(0)).collect();
        let gt = ov.at(GOLD_TOTALS);
        t.gold_totals = (0..20u32).map(|l| ov.i32(gt + 4 * l).unwrap_or(0)).collect();
        t
    }
}

/// `fun_0020baf0` (0x262900): 0 true; 1 planet unlocked; 2 owned; 3 acquired; 4 / 5 landmark flags ≠ 0 / > 1 (≤ 0x78);
/// 6 a global flag; 7 a callback; 8 a gold bolt (level `v >> 16`, index `v & 0xffff`); else false.
pub fn condition(gs: &GameState, t: i16, v: u32) -> bool {
    let g = &gs.global;
    let b = |s: &[u8], i: u32| s.get(i as usize).is_some_and(|&x| x != 0);
    match t {
        0 => true,
        1 => b(&g.planet_unlocked, v),
        2 => b(&g.owned, v),
        3 => b(&g.acquired, v),
        4 => v <= 0x78 && g.landmarks.get(v as usize).is_some_and(|l| l.flags != 0),
        5 => v <= 0x78 && g.landmarks.get(v as usize).is_some_and(|l| l.flags as i32 > 1),
        6 => b(&g.flags, v),
        7 => callback(gs, v, 0) != 0,
        8 => gs.levels.get((v >> 16) as usize).and_then(|l| l.gold_bolts.get((v & 0xffff) as usize)).is_some_and(|&x| x != 0),
        _ => false,
    }
}

/// The mission lists' level-specific callbacks (level01 addresses 0x262b40..0x262d78), by their code.
pub fn callback(gs: &GameState, a: u32, arg: u32) -> i16 {
    let g = &gs.global;
    let lm = |k: usize| g.landmarks.get(k).map_or(0, |l| l.flags);
    let f = |k: usize| g.flags.get(k).is_some_and(|&x| x != 0);
    let owned = |k: usize| g.owned.get(k).is_some_and(|&x| x != 0);
    let r = match a {
        0x262b40 => lm(24) != 0 && f(24),
        0x262b68 => lm(47) != 0 && f(32) && f(33),
        0x262ba0 => lm(64) != 0 && lm(63) != 0,
        0x262bd0 => lm(66) != 0 && f(85),
        0x262bf8 => lm(70) != 0 && f(97),
        0x262c20 => lm(87) != 0 && g.acquired.get(21).is_some_and(|&x| x != 0),
        0x262c48 => owned(33) && owned(31),
        0x262c78 => owned(arg as usize),
        0x262c90 => {
            return if lm(20) != 0 && lm(24) == 0 { 1 } else if lm(24) != 0 && owned(2) && lm(22) == 0 { 2 } else { 0 };
        }
        0x262ce8 => return if lm(47) == 0 { 0 } else if f(32) { 2 } else { 1 },
        0x262d18 => f(97) && g.planet_unlocked.get(13).is_some_and(|&x| x != 0),
        0x262d40 => return if owned(21) { 2 } else { (lm(87) != 0) as i16 },
        0x262d68 => owned(31),
        0x262d78 => return if owned(31) { 2 } else { owned(33) as i16 },
        _ => false,
    };
    r as i16
}

/// `FUN_00262760` (0x262760): the destination's missions' status (a list flagged 4 only once the level was visited:
/// the first condition → open, both → done), then their callbacks' values; true when none is open (flag 2 aside).
pub fn mission_status(missions: &mut [Mission], gs: &GameState, dest: i32) -> bool {
    let visited = gs.levels.get(dest.max(0) as usize).is_some_and(|l| l.visited != 0);
    for m in missions.iter_mut() {
        m.status = if m.flags & 4 == 0 || visited {
            if !condition(gs, m.req[0].0, m.req[0].1) { 0 } else if condition(gs, m.req[1].0, m.req[1].1) { 2 } else { 1 }
        } else {
            0
        };
    }
    for m in missions.iter_mut() {
        if m.cb != 0 { m.cb_value = callback(gs, m.cb, m.arg); }
    }
    !missions.iter().any(|m| m.status == 1 && m.flags & 2 == 0)
}

/// The fixed marker points −1..−9 (`FUN_00262da0`: u, v).
fn fixed_point(idx: i16) -> Option<(f32, f32)> {
    let p = |u: u32, v: u32| Some((f32::from_bits(u), f32::from_bits(v)));
    match idx {
        -1 => p(0x3e70_0000, 0x3e9a_0000),
        -2 => p(0x3f0a_0000, 0x3ebb_0000),
        -3 => p(0x3f15_0000, 0x3f30_0000),
        -4 => p(0x3f38_8000, 0x3f3a_8000),
        -5 => p(0x3ec5_0000, 0x3ecb_0000),
        -7 => p(0x3f5e_0000, 0x3e6c_0000),
        -8 => p(0x3ef6_0000, 0x3f12_0000),
        -9 => p(0x3f20_0000, 0x3f39_0000),
        _ => None,
    }
}

/// `FUN_00262da0(dest, centre)` (0x262da0): the destination's marker list when it was visited (else none); with
/// `centre` on the current level (and the hero's z ≠ 0) the view pans to the hero; each marker's map point (the fixed
/// ones, the hook moby's position and angle on the current level, the landmark's elsewhere) and whether it shows (its
/// mission open, or no mission). The label boxes are sized by the draw ([`draw_markers`]).
pub fn setup_markers(m: &mut PageMenu, gs: &GameState, dest: i32, centre: bool) {
    let p = &mut m.map;
    let cur = p.state.level;
    let visited = (0..map::LEVELS as i32).contains(&dest) && gs.levels.get(dest as usize).is_some_and(|l| l.visited != 0);
    p.markers = if visited { p.tables.markers.get(dest as usize).cloned().unwrap_or_default() } else { Vec::new() };
    if centre && p.hero.pos[2] != 0.0 && dest == cur {
        let (u, v) = map::world_to_map(&p.state.transforms, dest, p.state.alt, p.hero.pos[0], p.hero.pos[1]);
        if let Some(view) = p.state.view.get_mut(dest.max(0) as usize) {
            view.pan = [((u * 4096.0) as i32) << 16, ((v * 4096.0) as i32) << 16];
        }
    }
    let missions = p.missions.clone();
    for k in p.markers.iter_mut() {
        if let Some((u, v)) = fixed_point(k.idx) {
            (k.u, k.v) = (u, v);
        } else if dest == cur {
            if let Some(&(x, y, rot)) = p.hooks.get(&(k.idx as i32)) {
                (k.u, k.v) = map::world_to_map(&p.state.transforms, dest, false, x, y);
                k.rot = rot;
            }
        } else if let Some(l) = gs.global.landmarks.get(k.idx.max(0) as usize) {
            (k.u, k.v) = map::world_to_map(&p.state.transforms, dest, false, l.x, l.y);
            k.rot = l.rot;
        }
        k.flags &= !0x10;
        k.shown = k.link == -1 || missions.get(k.link.max(0) as usize).is_some_and(|m| m.status == 1);
        if k.flags & 0x1000 != 0 && gs.global.landmarks.get(k.idx.max(0) as usize).is_none_or(|l| l.flags & 1 == 0) { k.shown = false; }
        if k.label != 0 { k.flags |= 0x10; }
    }
}

/// `FUN_0025e630`: the label's message with `%b` → the price (0x1c4530 + 0x18·price, `%d`), another `%x` → "error".
fn label_text(a: &MenuAssets, t: &MapTables, k: &Marker) -> Vec<u8> {
    let s = a.msg(k.label as i32);
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'%' {
            if s.get(i + 1) == Some(&b'b') {
                out.extend(format!("{}", t.prices.get(k.price.max(0) as usize).copied().unwrap_or(0)).bytes());
            } else {
                out.extend(b"error");
            }
            out.extend(&s[(i + 2).min(s.len())..]);
            return out;
        }
        out.push(s[i]);
        i += 1;
    }
    out
}

/// The markers of `UNK_NoMapAvailable`: each shown marker's rectangle about its map point (the icon's texture size ×
/// the scale: ×1.5 with flag 0x80, × the zoom with 0x200 else (2z + 5) / 13), pushed apart pairwise (markers without
/// flags 1 / 2: the smaller overlap axis split half each), then drawn: the black backing (0x40), the icon or the turned
/// sprite (0x100; 0x400 / 0x800 / 0x1000 add π/2 and take their sizes, the landmark's flag 2 the next frame), and the
/// label box (0x10: a UI frame and the text in the small font, sized to the fewest columns that keep its lines).
fn draw_markers(p: &MapPage, a: &MenuAssets, gs: &GameState, rect: (i32, i32, i32, i32), dx: i32, dy: i32, out: &mut Vec<MenuDraw>) {
    let (left, top, right, bottom) = rect;
    let z = p.state.view[p.shown.max(0) as usize].zoom;
    let fz = (z + z + 5.0) / 13.0;
    let n = p.markers.len();
    let mut r: Vec<[i32; 4]> = vec![[0; 4]; n];
    for (i, k) in p.markers.iter().enumerate() {
        if !k.shown || k.icon == 0 || k.flags & 1 != 0 { continue; }
        let s = if k.flags & 0x80 != 0 { 1.5 } else { 1.0 } * if k.flags & 0x200 != 0 { z } else { fz };
        let f = a.frame(k.icon as u16, k.frame as i32);
        let (tw, th) = a.frame_size(f);
        let (tw, th) = ((tw.max(1) as u32).next_power_of_two() as f32, (th.max(1) as u32).next_power_of_two() as f32);
        let x0 = (left as f32 + k.u * (right - left) as f32 - s * tw * 8.0) as i32;
        let y0 = (top as f32 + k.v * (bottom - top) as f32 - s * th * 8.0) as i32;
        r[i] = [x0, y0, (x0 as f32 + s * tw * 16.0) as i32, (y0 as f32 + s * th * 16.0) as i32];
    }
    let pushable = |k: &Marker| k.shown && k.icon != 0 && k.flags & 3 == 0;
    for i in 0..n {
        if !pushable(&p.markers[i]) || i + 1 >= n { continue; }
        for j in i + 1..n {
            if !pushable(&p.markers[j]) { continue; }
            let (a0, b0) = (r[i], r[j]);
            let ox = b0[2] - a0[0];
            let (oy1, oy2, oy3) = (a0[2] - b0[0], b0[3] - a0[1], a0[3] - b0[1]);
            if ox <= 0 || oy1 <= 0 || oy2 <= 0 || oy3 <= 0 { continue; }
            let (mut ax, mut ay, mut bx, mut by) = (0, 0, 0, 0);
            if oy1 < ox || oy2 < ox || oy3 < ox {
                if oy2 < oy1 || oy3 < oy1 {
                    if oy3 < oy2 {
                        by = oy3 >> 1;
                        ay = by - oy3;
                    } else {
                        ay = oy2 >> 1;
                        by = ay - oy2;
                    }
                } else {
                    bx = oy1 >> 1;
                    ax = bx - oy1;
                }
            } else {
                ax = ox >> 1;
                bx = ax - ox;
            }
            r[i] = [a0[0] + ax, a0[1] + ay, a0[2] + ax, a0[3] + ay];
            r[j] = [b0[0] + bx, b0[1] + by, b0[2] + bx, b0[3] + by];
        }
    }
    let px = |v: i32| v >> 4;
    for (i, k) in p.markers.iter().enumerate() {
        if !k.shown || k.flags & 1 != 0 || k.icon == 0 { continue; }
        let [x0, y0, x1, y1] = r[i];
        if k.flags & 0x40 != 0 {
            out.push(MenuDraw::Rect { x0: px(x0 - 0x20) + dx + 1, y0: px(y0 - 0x20) + dy + 1, x1: px(x1 + 0x20) + dx + 1, y1: px(y1 + 0x20) + dy + 1, rgba: 0x8000_0000 });
        }
        let zs = if k.flags & 0x200 != 0 { z } else { fz };
        if k.flags & 0x100 == 0 {
            let f = a.frame(k.icon as u16, k.frame as i32);
            let (tw, th) = a.frame_size(f);
            out.push(MenuDraw::Quad {
                tex: QuadTex::Frame(f),
                pos: [[px(x0) + dx, px(y0) + dy], [px(x1) + dx, px(y0) + dy], [px(x0) + dx, px(y1) + dy], [px(x1) + dx, px(y1) + dy]],
                uv: [[0, 0], [tw, 0], [0, th], [tw, th]],
                rgba: 0x8080_8080,
                repeat: false,
                uv16: false,
            });
        } else {
            let (mut ang, mut w, mut h) = (k.rot, zs * 256.0, zs * 256.0);
            let mut frame = k.frame as i32;
            let lm2 = gs.global.landmarks.get(k.idx.max(0) as usize).is_some_and(|l| l.flags & 2 != 0);
            for (bit, (sw, sh)) in [(0x400u16, (p.tables.sizes[2], p.tables.sizes[3])), (0x800, (p.tables.sizes[0], p.tables.sizes[1]))] {
                if k.flags & bit != 0 {
                    ang += std::f32::consts::FRAC_PI_2;
                    (w, h) = (zs * sw, zs * sh);
                    if lm2 { frame += 1; }
                }
            }
            if k.flags & 0x1000 != 0 {
                ang += std::f32::consts::FRAC_PI_2;
                (w, h) = (zs * p.tables.sizes[4], zs * p.tables.sizes[5]);
            }
            let f = a.frame(k.icon as u16, frame);
            let (tw, th) = a.frame_size(f);
            let (cx, cy) = ((x0 + x1) as f32 * 0.5 / 16.0 + dx as f32, (y0 + y1) as f32 * 0.5 / 16.0 + dy as f32);
            out.push(rotated(f, cx, cy, w / 16.0, h / 16.0, ang, tw, th));
        }
        if k.flags & 0x10 != 0 {
            let text = label_text(a, &p.tables, k);
            let glyphs = &a.hud.glyphs[Font::Small as usize];
            // The box size (0x262da0): the measured height + 8, then the narrowest width (in steps of 4) with that
            // height, + 4.
            let measure = |w: i16| {
                let mut win = crate::hud::text::Window::new(0, k.box_h, 0, w, 4, 4, 0xf, crate::hud::text::MEASURE_ONLY);
                crate::hud::text::layout(&mut win, &text, -1, glyphs, true);
                win.height
            };
            let h0 = measure(k.box_w);
            let mut bw = k.box_w;
            while bw > 8 && measure(bw - 4) == h0 { bw -= 4; }
            let (bw, bh) = (bw as i32, h0 as i32 + 8);
            let x = if k.box_dx == 0 { ((x0 + x1) >> 5) - (bw >> 1) } else { ((if k.box_dx < 1 { x0 } else { x1 }) >> 4) - (bw >> 1) + k.box_dx as i32 };
            let y = if k.box_dy == 0 { ((y0 + y1) >> 5) - (bh >> 1) } else { ((if k.box_dy < 1 { y0 } else { y1 }) >> 4) - (bh >> 1) + k.box_dy as i32 };
            let (x, y) = (x + dx, y + dy);
            out.push(MenuDraw::Hud(crate::hud::Draw::UiFrame { top: y, bottom: y + bh, left: x, right: x + bw, alpha: 0x40 }));
            let win = crate::hud::text::Window::new(y as i16, (y + bh) as i16, x as i16, (x + bw) as i16, (x + bw / 2) as i16, (y + 4) as i16, 0xf, crate::hud::text::CENTRE_LINES);
            out.push(MenuDraw::Hud(crate::hud::Draw::TextWindow { font: Font::Small, window: win, rgba: 0x80ff_a888, text }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::save_game::ChunkTables;

    fn gs() -> GameState { GameState::zeroed(ChunkTables { global: vec![], level: vec![] }) }

    /// `fun_0020baf0`'s condition types on a bare state, each flipped by its own field.
    #[test]
    fn conditions() {
        let mut g = gs();
        assert!(condition(&g, 0, 0));
        for t in [1, 2, 3, 4, 5, 6, 8, 9, -1] { assert!(!condition(&g, t, 3), "type {t}"); }
        g.global.planet_unlocked[3] = 1;
        g.global.owned[3] = 1;
        g.global.acquired[3] = 1;
        g.global.flags[3] = 1;
        g.global.landmarks[3].flags = 1;
        g.levels[2].gold_bolts[1] = 1;
        for t in [1, 2, 3, 4, 6] { assert!(condition(&g, t, 3), "type {t}"); }
        assert!(!condition(&g, 5, 3), "type 5 needs flags > 1");
        g.global.landmarks[3].flags = 2;
        assert!(condition(&g, 5, 3));
        assert!(!condition(&g, 4, 0x79), "landmarks ≤ 0x78");
        assert!(condition(&g, 8, 2 << 16 | 1));
        assert!(condition(&g, 7, 0x262c68) == (callback(&g, 0x262c68, 0) != 0));
    }

    /// The 14 mission callbacks (0x262b40..0x262d78), each against its fields.
    #[test]
    fn callbacks() {
        let mut g = gs();
        for a in [0x262b40, 0x262b68, 0x262ba0, 0x262bd0, 0x262bf8, 0x262c20, 0x262c48, 0x262c90, 0x262ce8, 0x262d18, 0x262d40, 0x262d68, 0x262d78] {
            assert_eq!(callback(&g, a, 0), 0, "{a:#x} on a bare state");
        }
        g.global.landmarks[24].flags = 1;
        assert_eq!(callback(&g, 0x262b40, 0), 0);
        g.global.flags[24] = 1;
        assert_eq!(callback(&g, 0x262b40, 0), 1);
        g.global.landmarks[47].flags = 1;
        g.global.flags[32] = 1;
        assert_eq!((callback(&g, 0x262b68, 0), callback(&g, 0x262ce8, 0)), (0, 2));
        g.global.flags[33] = 1;
        assert_eq!(callback(&g, 0x262b68, 0), 1);
        g.global.flags[32] = 0;
        assert_eq!(callback(&g, 0x262ce8, 0), 1);
        g.global.landmarks[64].flags = 1;
        g.global.landmarks[63].flags = 1;
        assert_eq!(callback(&g, 0x262ba0, 0), 1);
        g.global.landmarks[66].flags = 1;
        g.global.flags[85] = 1;
        assert_eq!(callback(&g, 0x262bd0, 0), 1);
        g.global.landmarks[70].flags = 1;
        g.global.flags[97] = 1;
        assert_eq!((callback(&g, 0x262bf8, 0), callback(&g, 0x262d18, 0)), (1, 0));
        g.global.planet_unlocked[13] = 1;
        assert_eq!(callback(&g, 0x262d18, 0), 1);
        g.global.landmarks[87].flags = 1;
        assert_eq!((callback(&g, 0x262c20, 0), callback(&g, 0x262d40, 0)), (0, 1));
        g.global.acquired[21] = 1;
        g.global.owned[21] = 1;
        assert_eq!((callback(&g, 0x262c20, 0), callback(&g, 0x262d40, 0)), (1, 2));
        g.global.owned[33] = 1;
        assert_eq!((callback(&g, 0x262c48, 0), callback(&g, 0x262d78, 0), callback(&g, 0x262c78, 33)), (0, 1, 1));
        g.global.owned[31] = 1;
        assert_eq!((callback(&g, 0x262c48, 0), callback(&g, 0x262d78, 0), callback(&g, 0x262d68, 0)), (1, 2, 1));
        // 0x262c90: 1 while landmark 20 is set and 24 not; 2 with 24, the Heli-Pack and landmark 22 clear.
        let mut h = gs();
        h.global.landmarks[20].flags = 1;
        assert_eq!(callback(&h, 0x262c90, 0), 1);
        h.global.landmarks[24].flags = 1;
        assert_eq!(callback(&h, 0x262c90, 0), 0);
        h.global.owned[2] = 1;
        assert_eq!(callback(&h, 0x262c90, 0), 2);
        h.global.landmarks[22].flags = 1;
        assert_eq!(callback(&h, 0x262c90, 0), 0);
    }

    /// `FUN_00262760`: open with the first condition, done with both; flag 4 lists wait for the visit; the "none open"
    /// answer ignores flag-2 missions; the callbacks' values.
    #[test]
    fn mission_status_rules() {
        let mut g = gs();
        let mut ms = vec![
            Mission { req: [(0, 0), (2, 5)], ..Default::default() },
            Mission { req: [(0, 0), (0, 0)], flags: 4, ..Default::default() },
            Mission { req: [(2, 6), (0, 0)], flags: 2, cb: 0x262c78, arg: 6, ..Default::default() },
        ];
        assert!(!mission_status(&mut ms, &g, 3));
        assert_eq!(ms.iter().map(|m| m.status).collect::<Vec<_>>(), vec![1, 0, 0]);
        g.levels[3].visited = 1;
        g.global.owned[5] = 1;
        g.global.owned[6] = 1;
        assert!(mission_status(&mut ms, &g, 3));
        assert_eq!(ms.iter().map(|m| (m.status, m.cb_value)).collect::<Vec<_>>(), vec![(2, 0), (2, 0), (2, 1)]);
    }
}

// ---------------------------------------------------------------------------------------------------
// The missions page and the streamed pictures (G-UI-002)

/// One listed mission of `fun_0020bc00`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Listed {
    /// The name (`descs` false) or the description / "Completed" 0x523e (`descs` true).
    pub text: i32,
    /// Its picture (`mission_ss`).
    pub picture: i32,
    pub done: bool,
}

/// `fun_0020bc00` over the destination's missions (their status already computed, [`mission_status`]): the listed
/// missions and the "all done" bit (31 of the mask).
pub fn mission_list(missions: &[Mission], descs: bool) -> (Vec<Listed>, bool) {
    let mut all = true;
    let mut out = Vec::new();
    for m in missions {
        if m.status != 2 && m.flags & 2 == 0 { all = false; }
        if m.flags & 2 != 0 || m.status == 0 || (m.flags & 1 != 0 && m.status == 2) { continue; }
        let text = if !descs {
            m.name as i32
        } else if m.status == 2 {
            0x523e
        } else {
            m.descs.get(m.cb_value.max(0) as usize).copied().unwrap_or(0) as i32
        };
        out.push(Listed { text, picture: (m.picture as i32) + (m.cb_value as i32), done: m.status == 2 });
    }
    (out, all)
}

/// The missions widget's state (+0x30: a cursor per destination, −1 none; +0x7c: the count).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Missions {
    pub cursors: [i32; 20],
    pub count: i32,
}

/// The label 0x1b3df4 and the picture index 0x1b3cd8 for the destination's cursor (0x28fe28 / 0x28fec8): the label
/// widget 0x1b3dc0 shows the description, the picture widget 0x1b3c80 the mission's picture.
fn missions_show(m: &mut PageMenu, cursor: i32) {
    let (list, _) = mission_list(&m.map.missions, true);
    let Some(it) = usize::try_from(cursor).ok().and_then(|c| list.get(c)).copied() else { return };
    let (label, image) = (m.addrs.missions_label, m.addrs.missions_image);
    if let Some(Data::Label(l)) = m.widgets.get_mut(&label).map(|x| &mut x.data) { l.id = it.text as u32; }
    if let Some(Data::Picture(p)) = m.widgets.get_mut(&image).map(|x| &mut x.data) { p.explicit = it.picture; }
}

/// `fun_0021c420` 0x28fe28, the missions widget's enter.
pub fn missions_enter(m: &mut PageMenu, w: u32, gs: &GameState) {
    let dest = m.dest;
    enter(m, gs);
    let count = mission_list(&m.map.missions, true).0.len() as i32;
    let cursor = match m.widgets.get_mut(&w).map(|x| &mut x.data) {
        Some(Data::Missions(ms)) => {
            ms.count = count;
            ms.cursors.get(dest.max(0) as usize).copied().unwrap_or(-1)
        }
        _ => return,
    };
    if cursor != -1 { missions_show(m, cursor); }
}

/// The Up / Down half of 0x28fec8 (after the planet keys; `changed` = the destination changed): the cursor of the
/// destination moves mod the count, sound 1 on a change, the count, label and picture again on Up / Down or a new
/// destination.
pub fn missions_keys(m: &mut PageMenu, w: u32, keys: u32, changed: bool, out: &mut MenuOut) {
    use crate::pad::button;
    let dest = m.dest.clamp(0, 19) as usize;
    let Some(Data::Missions(ms)) = m.widgets.get(&w).map(|x| &x.data) else { return };
    let mut ms = *ms;
    if ms.count == 0 { return; }
    let before = ms.cursors[dest];
    if keys & button::UP != 0 { ms.cursors[dest] = (before + ms.count - 1) % ms.count; }
    if keys & button::DOWN != 0 { ms.cursors[dest] = (ms.cursors[dest] + 1) % ms.count; }
    if ms.cursors[dest] != before { out.sounds.push(super::super::MenuSound::Cursor); }
    if keys & (button::UP | button::DOWN) != 0 || changed {
        ms.count = mission_list(&m.map.missions, true).0.len() as i32;
        let c = ms.cursors[dest];
        if let Some(Data::Missions(x)) = m.widgets.get_mut(&w).map(|x| &mut x.data) { *x = ms; }
        missions_show(m, c);
        return;
    }
    if let Some(Data::Missions(x)) = m.widgets.get_mut(&w).map(|x| &mut x.data) { *x = ms; }
}

/// The unfocused half of 0x28fec8: the destination's cursor = −1 (dest < 0x14).
pub fn missions_unfocused(m: &mut PageMenu, w: u32) {
    let dest = m.dest;
    if let Some(Data::Missions(ms)) = m.widgets.get_mut(&w).map(|x| &mut x.data) {
        if (0..0x14).contains(&dest) { ms.cursors[dest as usize] = -1; }
    }
}

/// `fun_001f6fd0(x, y, w, max_h, rgba, text, −1)` → `fun_001f6cb8` with the small font: the greedy word wrap (a word
/// that does not fit starts a new line, a space at the break is dropped). Returns the lines (y, bytes) and the height
/// (`last y − y + 16`).
pub fn wrap_small(a: &MenuAssets, x: i32, y: i32, w: i32, max_h: i32, t: &[u8]) -> (Vec<(i32, Vec<u8>)>, i32) {
    let adv = |c: u8| a.width(Font::Small, &[c]);
    let mut lines: Vec<(i32, Vec<u8>)> = vec![(y, Vec::new())];
    let (mut i, mut ly, mut lx) = (0usize, y, x);
    if t.first().is_some_and(|&c| c != 0) {
        while ly + 0x10 <= y + max_h {
            let c = t[i];
            let mut wx = lx as f32;
            if c != b' ' && c > 0xf {
                let mut j = i;
                let mut sum = 0.0f32;
                while j < t.len() {
                    sum += adv(t[j]) as f32;
                    j += 1;
                    if j >= t.len() || t[j] == b' ' || t[j] < 0x10 { break; }
                }
                wx += sum;
            }
            if ((x + w) as f32) < wx {
                if c != b' ' && c > 0xf { i = i.wrapping_sub(1); }
                ly += 0x10;
                lx = x;
                lines.push((ly, Vec::new()));
            } else if !(8..=15).contains(&c) && adv(c) != 0 {
                if let Some(l) = lines.last_mut() { l.1.push(c); }
                lx += adv(c);
            }
            i = i.wrapping_add(1);
            if i >= t.len() || t[i] == 0 { break; }
        }
    }
    (lines, ly - y + 0x10)
}

/// `DrawMissionsMenu2` 0x293090 (panel-local; return 2).
pub fn missions_draw(m: &PageMenu, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 2 };
    let [_, _, ww, wh] = wd.rect;
    let cursor = match &wd.data {
        Data::Missions(ms) => ms.cursors.get(m.dest.max(0) as usize).copied().unwrap_or(-1),
        _ => -1,
    };
    let (list, all) = mission_list(&m.map.missions, false);
    out.push(MenuDraw::Hud(Draw::Text { font: Font::Large, x: 4, y: 4, rgba: LIGHT_BLUE, text: a.msg(0x4f59).to_vec() }));
    let mut y = 0x18;
    for (k, it) in list.iter().enumerate() {
        let sel = cursor == k as i32;
        let col = if sel { YELLOW } else { LIGHT_BLUE };
        let (lines, h) = wrap_small(a, 0x10, y, ww - 0x11, 1000, a.msg(it.text));
        for (ly, t) in lines {
            if sel { super::super::text_plain(out, Font::Small, 0x10, ly, col, &t) } else { super::super::text(out, Font::Small, 0x10, ly, col, &t) }
        }
        let n = if h > 0x10 { 2 } else { 1 };
        super::media::check_box(a, m.consts.navy, 9, if n == 2 { y + 0x10 } else { y + 10 }, it.done, out);
        y += n * 0x10;
    }
    if all {
        let win = wtext::Window::new(0, wh as i16, 0, ww as i16, (wh >> 1) as i16, (y + 8) as i16, 0x10, wtext::CENTRE_LINES);
        out.push(MenuDraw::Hud(Draw::TextWindow { font: Font::Small, window: win, rgba: YELLOW, text: a.msg(0x523d).to_vec() }));
    }
    2
}

/// The picture widget 0x293398 / 0x293670's state (+0x30 TOC field, +0x34 flags, +0x38 / +0x3c texels, +0x44 state,
/// +0x50 the index read, +0x58 the explicit index; `shown`: the index whose picture was converted last).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Picture {
    pub field: u32,
    pub flags: u32,
    pub tw: i32,
    pub th: i32,
    pub state: i32,
    pub index: i32,
    pub explicit: i32,
    pub shown: i32,
}

impl Picture {
    pub fn read(raw: &[u32; 8]) -> Picture {
        Picture { field: raw[0].wrapping_sub(super::pages::TOC_BASE), flags: raw[1], tw: raw[2] as i32, th: raw[3] as i32, state: -1, index: -1, explicit: -1, shown: -1 }
    }
}

/// 0x2936e8 (shared with the image widgets): +0x44 = 0, the indices −1.
pub fn picture_enter(m: &mut PageMenu, w: u32) {
    if let Some(Data::Picture(p)) = m.widgets.get_mut(&w).map(|x| &mut x.data) {
        p.state = 0;
        p.index = -1;
        p.shown = -1;
    }
}

/// 0x293780: +0x44 = −1, the indices −1.
pub fn picture_leave(m: &mut PageMenu, w: u32) {
    if let Some(Data::Picture(p)) = m.widgets.get_mut(&w).map(|x| &mut x.data) {
        p.state = -1;
        p.index = -1;
    }
}

/// `fun_0021f990` 0x293398.
pub fn picture_update(m: &mut PageMenu, w: u32) -> i32 {
    let focus = m.pages.get(&m.current).map_or(0, |p| p.focus);
    let Some(Data::Picture(p)) = m.widgets.get(&w).map(|x| &x.data) else { return 0 };
    let mut p = *p;
    let idx = if p.flags & 1 != 0 {
        if p.explicit == -1 { return 0; }
        p.explicit
    } else if p.flags & 2 != 0 {
        m.dest
    } else if p.flags & 4 != 0 {
        // The focused widget's +0x3c (a grid's cursor).
        match m.widgets.get(&focus).map(|x| &x.data) {
            Some(Data::Grid(g)) => g.cursor,
            _ => 0,
        }
    } else {
        // The focused widget's +0x40 (a list's cursor), ≥ 0.
        match m.widgets.get(&focus).map(|x| &x.data) {
            Some(Data::List(l)) => l.cursor.max(0),
            _ => 0,
        }
    };
    match p.state {
        1 | 3 => {
            // The read is done: decompressed (flag 0x20) and converted (`fun_00204e30`).
            p.shown = p.index;
            p.state = 2;
        }
        0 | 2 => {
            if idx == p.index { return 0; }
            // An entry past the TOC field (its sector count 0): nothing.
            let n = super::pages::field_entries(p.field);
            if n != 0 && idx as u32 >= n { return 0; }
            if idx < 0 { return 0; }
            p.index = idx;
            p.state += 1;
        }
        _ => {}
    }
    if let Some(Data::Picture(x)) = m.widgets.get_mut(&w).map(|x| &mut x.data) { *x = p; }
    0
}

/// `draw_transition_overlay` 0x293670 (panel-local): the converted picture over the panel; return 0x10 (or 0).
pub fn picture_draw(m: &PageMenu, w: u32, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 0 };
    let Data::Picture(p) = wd.data else { return 0 };
    if p.state < 2 || p.shown < 0 { return 0; }
    let [_, _, ww, wh] = wd.rect;
    out.push(MenuDraw::Image { src: super::super::ImageSrc::Lump { field: p.field, index: p.shown as u32 }, x: 0, y: 0, w: ww, h: wh, u: 0, v: 0, tw: p.tw, th: p.th, rgba: 0x8080_8080 });
    0x10
}

/// The confirm page's gold bolt: `*gp−0x689c` (y offset 1.3).
pub const SHIP_GOLD_BOLT_OFFSET: [f32; 3] = [8.0, 1.3, -0.1];

/// `DrawGBsShipMenu` 0x292980 (panel-local; return 8, centre crop).
pub fn gold_panel_draw(m: &mut PageMenu, w: u32, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 8 };
    let rect = wd.rect;
    let [_, _, ww, wh] = rect;
    if let Some(spin) = m.gold_spin {
        let class = super::pages::GOLD_BOLT_CLASS;
        m.view.gold_bolt = Some(super::gadgets::PreviewView { rect, item: 0, o_class: class, clank: false, seq: 0, offset: SHIP_GOLD_BOLT_OFFSET, rot: [spin, super::pages::GOLD_BOLT_PITCH, 0.0] });
    }
    let dest = m.dest.clamp(0, 19) as usize;
    let found = gs.levels.get(dest).map_or(0, |l| l.gold_bolts.iter().filter(|&&b| b != 0).count());
    let total = m.map.tables.gold_totals.get(dest).copied().unwrap_or(0);
    let mut t = a.msg(0x4f4f).to_vec();
    t.extend_from_slice(format!(" {found} ").as_bytes());
    t.extend_from_slice(a.msg(0x4f53));
    t.extend_from_slice(format!(" {total}").as_bytes());
    let (dx, dy) = super::pages::crop(ww, wh);
    let tw = a.width(Font::Regular, &t);
    // H = 0x13e504, the target's height while the panel draws (`SetRenderToTextureView`), not the screen's: with 416
    // the count sat far below the panel's target and was clipped away.
    let y = (super::pages::target_size(ww, wh).1 >> 1) - 8;
    super::super::text(out, Font::Regular, ww - 0x10 - tw + dx, y + dy, super::SHADOW, &t);
    super::super::text(out, Font::Regular, ww - 0x11 - tw + dx, y - 1 + dy, LIGHT_BLUE, &t);
    8
}
