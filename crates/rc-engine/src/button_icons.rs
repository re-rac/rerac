//! **Port-only**: the button icons in the game's text drawn as the PS2's (the disc's), a PS5 pad's or an Xbox One pad's
//! (Port Options → Controller → "Button icons"; `Auto` follows the controller last used, crate::gms).
//!
//! Every button icon is a font cell: the codes 0x10..0x19 of the three fonts' glyph tables (✕ ○ △ □, L1 R1 L2 R2, the
//! left and right stick; docs/plan/hud_text.md), 24×16 texels at the glyph's (u, v) of the font's FX texture, which the
//! HUD atlas holds; the menus, the vendor, the help box and the HUD prompts all print through them. crate::hud_render
//! rewrites those cells in the atlas ([`cell`]) whenever the wanted style ([`Icons`]) changes and on every new atlas
//! (each level load builds one from the disc's textures).
//!
//! The art is drawn here, supersampled 4×4 like the originals' soft edges, on their footprints: the face buttons a 16×16
//! dark disc at x 2..17, the shoulder buttons a 21×13 rounded box at x 3..23, the sticks a 15×14 knob at x 1..15.
//! Colours are raw GS values (alpha 0..0x80): the text draw multiplies them by the caller colour's grey, as the disc's.
//! * **PS5**: the DualSense's monochrome symbols, white outlines on the dark disc; L1 R1 L2 R2.
//! * **Xbox One**: the letters in their colours on the dark disc, by position (✕ the bottom button: A green, ○ B red,
//!   △ Y yellow, □ X blue); LB RB LT RT.
//!
//! `RC_DUMP_BUTTONS=<dir>`: writes each font's original cells 0x10..0x1f (24×16 RGBA, raw GS alpha) as
//! `f<font>_<code>.rgba` and prints their glyph table entries.

use bevy::prelude::*;

/// The pad-icon cells' size in the font textures (`text_render::font_print`).
pub const CELL_W: u32 = 24;
pub const CELL_H: u32 = 16;
/// The pad-icon codes with a glyph on the disc.
pub const CODES: std::ops::Range<u8> = 0x10..0x1a;

/// The Controller page's "Button icons" values, in the row's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconStyle {
    /// The disc's PS2 icons.
    #[default]
    Original,
    Ps5,
    Xbox,
    /// PS5 for a Sony pad, Xbox One for any other pad, as the controller last used ([`resolve`]).
    Auto,
}

impl IconStyle {
    pub const ALL: [IconStyle; 4] = [IconStyle::Original, IconStyle::Ps5, IconStyle::Xbox, IconStyle::Auto];
    pub const KEYS: [&'static str; 4] = ["original", "ps5", "xbox", "auto"];

    pub fn index(self) -> u8 { Self::ALL.iter().position(|&s| s == self).unwrap_or(0) as u8 }
    pub fn from_index(i: u8) -> IconStyle { Self::ALL.get(i as usize).copied().unwrap_or_default() }
    pub fn key(self) -> &'static str { Self::KEYS[self.index() as usize] }
    pub fn from_key(k: &str) -> Option<IconStyle> { Self::KEYS.iter().position(|&s| s == k).map(|i| Self::ALL[i]) }
}

/// The style the cells are drawn in now (never `Auto`: [`resolve`] picks one).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Icons(pub IconStyle);

/// GS-alpha RGBA (alpha 0..=0x80).
type Rgba = [u8; 4];

/// The cell of `code` in `style` (`Original`: `original(x, y)`, the disc's texel), row-major 24×16.
pub fn cell(style: IconStyle, code: u8, original: impl Fn(u32, u32) -> Rgba) -> Vec<Rgba> {
    let mut out = Vec::with_capacity((CELL_W * CELL_H) as usize);
    for y in 0..CELL_H {
        for x in 0..CELL_W {
            out.push(match style {
                IconStyle::Original | IconStyle::Auto => original(x, y),
                _ => pixel(style, code, x, y),
            });
        }
    }
    out
}

/// One texel: 4×4 samples of [`shade`], their coverage in the alpha.
fn pixel(style: IconStyle, code: u8, x: u32, y: u32) -> Rgba {
    let (mut rgb, mut cov) = ([0.0f32; 3], 0.0f32);
    for j in 0..4 {
        for i in 0..4 {
            let (px, py) = (x as f32 + (i as f32 + 0.5) / 4.0, y as f32 + (j as f32 + 0.5) / 4.0);
            if let Some(c) = shade(style, code, px, py) {
                for l in 0..3 { rgb[l] += c[l]; }
                cov += 1.0;
            }
        }
    }
    if cov == 0.0 { return [0; 4]; }
    [(rgb[0] / cov) as u8, (rgb[1] / cov) as u8, (rgb[2] / cov) as u8, (cov / 16.0 * 128.0).round() as u8]
}

/// 5×7 letters, rows top down, bit 4 the left column.
fn glyph(c: u8) -> [u8; 7] {
    match c {
        b'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        b'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        b'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        b'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        b'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        b'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        b'1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        _ => [0; 7],
    }
}

/// Whether the sample (px, py) is on letter `c` drawn bold (each column doubled to the right: 6×7) from texel (x0, y0).
fn on_letter(c: u8, x0: i32, y0: i32, px: f32, py: f32) -> bool {
    let (cx, cy) = (px.floor() as i32 - x0, py.floor() as i32 - y0);
    if !(0..7).contains(&cy) || !(0..6).contains(&cx) { return false; }
    let row = glyph(c)[cy as usize] as i32;
    let bit = |k: i32| (0..5).contains(&k) && row & (1 << (4 - k)) != 0;
    bit(cx) || bit(cx - 1)
}

/// Distance from (px, py) to the segment a–b.
fn seg(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((px - a.0) * dx + (py - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    ((px - a.0 - t * dx).powi(2) + (py - a.1 - t * dy).powi(2)).sqrt()
}

const WHITE: [f32; 3] = [0xe9 as f32, 0xe9 as f32, 0xec as f32];
/// Half the stroke of the PS5 symbols.
const STROKE: f32 = 0.72;

/// The dark button face, lighter toward the top left as the disc's bevel; `t` 0..1 across the shape.
fn face(t: f32) -> [f32; 3] {
    let v = 0x2a as f32 + 0x20 as f32 * (1.0 - t).clamp(0.0, 1.0);
    [v, v, v + 4.0]
}

/// The colour of sample (px, py) of `code`'s icon in `style`, None outside it.
fn shade(style: IconStyle, code: u8, px: f32, py: f32) -> Option<[f32; 3]> {
    match code {
        // Face buttons: a disc centred at (10, 8) (texels 2..17).
        0x10..=0x13 => {
            let (cx, cy, r) = (10.0, 8.0, 7.9);
            let (dx, dy) = (px - cx, py - cy);
            if dx * dx + dy * dy > r * r { return None; }
            let on = match style {
                IconStyle::Xbox => {
                    let (letter, colour) = match code {
                        0x10 => (b'A', [0x4c, 0xc2, 0x3a]),
                        0x11 => (b'B', [0xe8, 0x45, 0x3c]),
                        0x12 => (b'Y', [0xf5, 0xc8, 0x2a]),
                        _ => (b'X', [0x3d, 0x8b, 0xf5]),
                    };
                    on_letter(letter, 7, 4, px, py).then(|| colour.map(|c| c as f32))
                }
                _ => {
                    let hit = match code {
                        0x10 => seg(px, py, (6.6, 4.6), (13.4, 11.4)).min(seg(px, py, (6.6, 11.4), (13.4, 4.6))) < STROKE,
                        0x11 => ((dx * dx + dy * dy).sqrt() - 3.6).abs() < STROKE,
                        0x12 => {
                            let (a, b, c) = ((10.0, 3.9), (5.9, 11.0), (14.1, 11.0));
                            seg(px, py, a, b).min(seg(px, py, b, c)).min(seg(px, py, c, a)) < STROKE
                        }
                        _ => (dx.abs().max(dy.abs()) - 3.4).abs() < STROKE,
                    };
                    hit.then_some(WHITE)
                }
            };
            Some(on.unwrap_or_else(|| face(((dx + dy) / (2.0 * r) + 0.5).clamp(0.0, 1.0))))
        }
        // Shoulder buttons: a rounded box over texels 3..23 × 2..14 with a light rim and two letters.
        0x14..=0x17 => {
            let (x0, y0, x1, y1, rad) = (3.0, 2.0, 24.0, 15.0, 3.5);
            let qx = (px - (x0 + rad)).min(0.0).abs().max((px - (x1 - rad)).max(0.0));
            let qy = (py - (y0 + rad)).min(0.0).abs().max((py - (y1 - rad)).max(0.0));
            let outside = (qx * qx + qy * qy).sqrt() - rad;
            if outside > 0.0 { return None; }
            let text: &[u8; 2] = match (style, code) {
                (IconStyle::Xbox, 0x14) => b"LB",
                (IconStyle::Xbox, 0x15) => b"RB",
                (IconStyle::Xbox, 0x16) => b"LT",
                (IconStyle::Xbox, _) => b"RT",
                (_, 0x14) => b"L1",
                (_, 0x15) => b"R1",
                (_, 0x16) => b"L2",
                _ => b"R2",
            };
            if on_letter(text[0], 7, 5, px, py) || on_letter(text[1], 14, 5, px, py) { return Some([0xf0 as f32; 3]); }
            if outside > -1.0 { return Some([0x8a as f32; 3]); }
            Some(face((py - y0) / (y1 - y0)))
        }
        // Sticks: a knob centred at (8.5, 8) (texels 1..15) with a light ring and its side's letter.
        _ => {
            let (cx, cy, r) = (8.5, 8.0, 6.9);
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            if d > r { return None; }
            if on_letter(if code == 0x18 { b'L' } else { b'R' }, 6, 4, px, py) { return Some(WHITE); }
            if d > r - 1.1 { return Some([0x8a as f32; 3]); }
            Some(face(((px - cx + py - cy) / (2.0 * r) + 0.5).clamp(0.0, 1.0)))
        }
    }
}

/// The style to draw: the setting's, or for `Auto` the controller last used (PS5 for a Sony pad, Xbox One for any
/// other); before any pad is used, a connected one's; with only the keyboard, the disc's (or what was shown).
fn resolve(
    settings: Option<Res<crate::haptics::ControllerSettings>>,
    gms: Option<Res<crate::gms::Gms>>,
    gamepads: Query<&Gamepad>,
    mut icons: ResMut<Icons>,
) {
    use crate::gms::DeviceKey;
    let want = settings.map_or(IconStyle::Original, |s| s.icons);
    let style = match want {
        IconStyle::Auto => {
            let sony = |k: DeviceKey| match k {
                DeviceKey::DualSense => Some(true),
                DeviceKey::Gamepad(e) => gamepads.get(e).ok().map(|g| g.vendor_id() == Some(crate::dualsense::VENDOR)),
                DeviceKey::Keyboard => None,
            };
            let active = gms.as_ref().and_then(|g| g.active).and_then(sony);
            let connected = || gms.as_ref().and_then(|g| g.devices.iter().find_map(|(k, _)| sony(*k)));
            match active.or_else(connected) {
                Some(true) => IconStyle::Ps5,
                Some(false) => IconStyle::Xbox,
                None if icons.0 == IconStyle::Original => IconStyle::Original,
                None => icons.0,
            }
        }
        s => s,
    };
    if icons.0 != style { icons.0 = style; }
}

/// `RC_DUMP_BUTTONS`: the level's cells, once per level.
fn dump(level: Res<crate::Level>, mut done: Local<Option<u32>>) {
    let Ok(dir) = std::env::var("RC_DUMP_BUTTONS") else { return };
    let Some(lh) = level.0.hud.as_ref() else { return };
    let index = crate::level_load::level_index();
    if *done == Some(index) { return; }
    *done = Some(index);
    let _ = std::fs::create_dir_all(&dir);
    for (f, table) in lh.glyphs.iter().enumerate() {
        let Some(Some(tex)) = lh.fx.get(f + 1) else { continue };
        for code in 0x10u8..0x20 {
            let g = table[code as usize];
            println!("buttons: font {f} code {code:#04x}: u {} v {} y_off {} advance {}", g.u, g.v, g.y_off, g.advance);
            let texel = |x: u32, y: u32| {
                let i = (((g.v as u32 + y) * tex.width + g.u as u32 + x) * 4) as usize;
                tex.rgba.get(i..i + 4).map_or([0; 4], |p| [p[0], p[1], p[2], p[3]])
            };
            let bytes: Vec<u8> = cell(IconStyle::Original, code, texel).concat();
            let _ = std::fs::write(format!("{dir}/f{f}_{code:02x}.rgba"), bytes);
        }
    }
    // The port's art, once: `ps5_<code>.rgba`, `xbox_<code>.rgba`.
    for (style, name) in [(IconStyle::Ps5, "ps5"), (IconStyle::Xbox, "xbox")] {
        for code in CODES {
            let _ = std::fs::write(format!("{dir}/{name}_{code:02x}.rgba"), cell(style, code, |_, _| [0; 4]).concat());
        }
    }
}

pub struct ButtonIconsPlugin;

impl Plugin for ButtonIconsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Icons>().add_systems(PostUpdate, resolve).add_systems(Update, dump);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_fill_the_originals_footprints() {
        for style in [IconStyle::Ps5, IconStyle::Xbox] {
            for code in CODES {
                let c = cell(style, code, |_, _| [0; 4]);
                let lit: Vec<(u32, u32)> = (0..CELL_W * CELL_H).filter(|&i| c[i as usize][3] > 0).map(|i| (i % CELL_W, i / CELL_W)).collect();
                let (x0, x1) = (lit.iter().map(|p| p.0).min().unwrap(), lit.iter().map(|p| p.0).max().unwrap());
                let want = match code { 0x10..=0x13 => (2, 17), 0x14..=0x17 => (3, 23), _ => (1, 15) };
                assert_eq!((x0, x1), want, "{style:?} {code:#x}");
                // Never past the GS's opaque 0x80.
                assert!(c.iter().all(|p| p[3] <= 0x80));
            }
        }
    }

    #[test]
    fn original_copies_the_disc() {
        let c = cell(IconStyle::Original, 0x10, |x, y| [x as u8, y as u8, 7, 0x80]);
        assert_eq!((c[0], c[(5 * CELL_W + 9) as usize]), ([0, 0, 7, 0x80], [9, 5, 7, 0x80]));
    }

    #[test]
    fn styles_round_trip() {
        for s in IconStyle::ALL { assert_eq!((IconStyle::from_index(s.index()), IconStyle::from_key(s.key())), (s, Some(s))); }
    }
}
