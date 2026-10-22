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
//! **Sharp icons**: the PS5 and Xbox styles are drawn at the window's resolution, over the 512×416 layer:
//! `text_render::font_print` leaves those codes out of the HUD's and the page menus' primitives and records where they
//! go ([`crate::hud_render::IconMark`]); [`shown`] takes the frame's list (dimmed by the fades drawn over them) and
//! [`place`] lays one UI image per icon on the HUD's 512×416 box. The art is Zacksly's "PS5 Button Icons and
//! Controls" and "Xbox Series Button Icons and Controls" (Buttons Outline / White / 128w; CC BY 3.0,
//! art/button_icons/CREDITS.md), fitted to the cells' footprints, square on screen. The cells above stay for the
//! monitors' canvases and the scene subtitles, which keep their own 2D lists.
//!
//! `RC_DUMP_BUTTONS=<dir>`: writes each font's original cells 0x10..0x1f (24×16 RGBA, raw GS alpha) as
//! `f<font>_<code>.rgba` and prints their glyph table entries.

use crate::hud_render::{IconMark, Prim, Tex, H, W};
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use std::sync::atomic::{AtomicBool, Ordering};

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
    SHARP.store(matches!(style, IconStyle::Ps5 | IconStyle::Xbox), Ordering::Relaxed);
}

/// A PS5 or Xbox style is shown: `text_render::font_print` hands the pad icons to the sharp overlay.
static SHARP: AtomicBool = AtomicBool::new(false);

pub fn sharp() -> bool { SHARP.load(Ordering::Relaxed) }

macro_rules! art {
    ($dir:literal: $($name:literal),*) => { [$(include_bytes!(concat!("../art/button_icons/", $dir, "/", $name, ".png")) as &[u8]),*] };
}

/// The sharp art by code (0x10..0x19).
const PS5_ART: [&[u8]; 10] = art!("ps5": "cross", "circle", "triangle", "square", "l1", "r1", "l2", "r2", "left_stick", "right_stick");
const XBOX_ART: [&[u8]; 10] = art!("xbox": "a", "b", "y", "x", "lb", "rb", "lt", "rt", "left_stick", "right_stick");

/// A PNG as 8-bit sRGB RGBA: (width, height, texels).
fn decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let img = Image::from_buffer(bytes, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::linear(), RenderAssetUsages::all()).ok()?;
    let img = if img.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb { img } else { img.convert(TextureFormat::Rgba8UnormSrgb)? };
    Some((img.width(), img.height(), img.data?))
}

/// Half the size, 2×2 boxes (colour weighted by coverage): the copy drawn below ~96 pixels, where the 128-texel art
/// would alias under bilinear sampling.
fn half(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let (hw, hh) = (w / 2, h / 2);
    let mut out = vec![0u8; (hw * hh * 4) as usize];
    for y in 0..hh {
        for x in 0..hw {
            let (mut c, mut a) = ([0u32; 3], 0u32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = (((2 * y + dy) * w + 2 * x + dx) * 4) as usize;
                let pa = rgba[i + 3] as u32;
                for (k, ck) in c.iter_mut().enumerate() { *ck += rgba[i + k] as u32 * pa; }
                a += pa;
            }
            let o = ((y * hw + x) * 4) as usize;
            for (k, ck) in c.iter().enumerate() { out[o + k] = ck.checked_div(a).unwrap_or(0) as u8; }
            out[o + 3] = (a / 4) as u8;
        }
    }
    out
}

/// The drawn part of a picture: x0, y0, x1, y1 (ends exclusive), in texels.
fn bounds(w: u32, h: u32, rgba: &[u8]) -> [f32; 4] {
    let lit = |x: u32, y: u32| rgba[((y * w + x) * 4 + 3) as usize] > 0;
    let (mut b, mut any) = ([w, h, 0, 0], false);
    for y in 0..h {
        for x in 0..w {
            if lit(x, y) {
                any = true;
                b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
            }
        }
    }
    if any { b.map(|v| v as f32) } else { [0.0, 0.0, w as f32, h as f32] }
}

/// The sharp art, `[PS5 0x10..0x19, Xbox 0x10..0x19]`: each picture at full and [`half`] size, its [`bounds`] and
/// its side, in full-size texels.
#[derive(Resource)]
struct Art {
    images: Vec<[Handle<Image>; 2]>,
    bounds: Vec<[f32; 4]>,
    side: Vec<f32>,
}

fn load_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    use bevy::render::render_resource::{Extent3d, TextureDimension};
    let image = |w: u32, h: u32, data: Vec<u8>| {
        let mut i = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
        i.sampler = ImageSampler::linear();
        i
    };
    let mut art = Art { images: Vec::new(), bounds: Vec::new(), side: Vec::new() };
    for bytes in PS5_ART.iter().chain(&XBOX_ART) {
        let (w, h, rgba) = decode(bytes).filter(|(w, h, _)| *w >= 2 && *h >= 2).unwrap_or((2, 2, vec![0; 16]));
        let small = image(w / 2, h / 2, half(w, h, &rgba));
        art.bounds.push(bounds(w, h, &rgba));
        art.side.push(w.max(h) as f32);
        art.images.push([images.add(image(w, h, rgba)), images.add(small)]);
    }
    commands.insert_resource(art);
}

/// The box an icon's drawn part fits in, in its 24×16 cell: centre x, y and width, height (the disc's footprints: the
/// face buttons' 16×16 disc at x 2..17, y 0..15; the shoulders' 21×13 box at x 3..23, y 2..14; the sticks' 15×14 knob
/// at x 1..15, y 1..14).
fn footprint(code: u8) -> [f32; 4] {
    match code {
        0x10..=0x13 => [10.0, 8.0, 16.0, 16.0],
        0x14..=0x17 => [13.5, 8.5, 21.0, 13.0],
        _ => [8.5, 8.0, 15.0, 14.0],
    }
}

/// An icon to draw this frame: its cell's corner in game pixels and its tint (sRGB and alpha, 0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub code: u8,
    pub x: i32,
    pub y: i32,
    pub colour: [f32; 4],
}

/// The frame's sharp icons (crate::hud_render sets them once the layer's primitives are final).
#[derive(Resource, Default)]
pub struct Marks(pub Vec<Shown>);

/// The marks to draw: those inside their scissor (a page menu's only on the 512×416 screen), in the text colour's
/// grey (`FontPrint`: texel × grey / 0x80) and alpha, faded toward the untextured rectangles drawn over them after
/// (the fades: a black or white rectangle over the screen).
pub fn shown(prims: &[Prim], add: &[usize], marks: &[IconMark]) -> Vec<Shown> {
    let full = [0, W - 1, 0, H - 1];
    let ch = |v: u32, k: u32| ((v >> k) & 0xff) as f32;
    marks
        .iter()
        .filter_map(|m| {
            let (cx, cy) = (m.x + 12, m.y + 8);
            let s = m.scissor;
            if s != full && !(s[0] <= cx && cx <= s[1] && s[2] <= cy && cy <= s[3]) { return None; }
            if m.boxed && !(0..W).contains(&cx) { return None; }
            let k = (ch(m.rgba, 0) / 128.0).min(1.0);
            let mut colour = [k, k, k, (ch(m.rgba, 24) / 128.0).min(1.0)];
            for (i, p) in prims.iter().enumerate().skip(m.at) {
                if p.tex != Tex::None || add.binary_search(&i).is_ok() { continue; }
                let [x, y, w, h] = rect(p);
                if !(x <= cx && cx < x + w && y <= cy && cy < y + h) { continue; }
                let a = (ch(p.rgba, 24) / 128.0).min(1.0);
                for (c, v) in colour.iter_mut().take(3).enumerate() { *v += (ch(p.rgba, 8 * c as u32) / 255.0 - *v) * a; }
            }
            (colour[3] > 0.0).then_some(Shown { code: m.code, x: m.x, y: m.y, colour })
        })
        .collect()
}

/// A primitive's bounding rectangle `[x, y, w, h]`.
fn rect(p: &Prim) -> [i32; 4] {
    let (x0, x1) = (p.pos.iter().map(|v| v[0]).min().unwrap_or(0), p.pos.iter().map(|v| v[0]).max().unwrap_or(0));
    let (y0, y1) = (p.pos.iter().map(|v| v[1]).min().unwrap_or(0), p.pos.iter().map(|v| v[1]).max().unwrap_or(0));
    [x0, y0, x1 - x0, y1 - y0]
}

/// One sharp icon's UI image, a child of the HUD's 512×416 box (pooled: hidden while unused).
#[derive(Component)]
struct IconNode;

/// The [`Marks`] as UI images on the HUD's box: square on screen (the game's pixels are not), the art's drawn part
/// fitted to the cell's [`footprint`].
fn place(
    marks: Res<Marks>,
    icons: Res<Icons>,
    art: Option<Res<Art>>,
    hud_box: Query<(Entity, &ComputedNode), With<crate::hud_render::HudBoxNode>>,
    mut nodes: Query<(&mut Node, &mut ImageNode, &mut Visibility), With<IconNode>>,
    mut commands: Commands,
) {
    let style = match icons.0 {
        IconStyle::Ps5 => Some(0),
        IconStyle::Xbox => Some(1),
        _ => None,
    };
    let (Some(art), Some(style), Ok((parent, computed))) = (art, style, hud_box.single()) else {
        for (_, _, mut v) in &mut nodes { v.set_if_neq(Visibility::Hidden); }
        return;
    };
    let size = computed.size();
    // Horizontal game pixels per vertical one's length on screen (0.92 in the 4:3 box).
    let ax = if size.x > 0.0 && size.y > 0.0 { (size.y / H as f32) / (size.x / W as f32) } else { 0.75 * W as f32 / H as f32 };
    let px_per_y = size.y / H as f32;
    let mut pool = nodes.iter_mut();
    for m in &marks.0 {
        let k = style * 10 + (m.code - 0x10) as usize;
        let [bx0, by0, bx1, by1] = art.bounds[k];
        let [cx, cy, tw, th] = footprint(m.code);
        // Vertical game pixels per art texel.
        let s = (th / (by1 - by0)).min(tw / ((bx1 - bx0) * ax));
        let side = art.side[k] * s;
        let left = m.x as f32 + cx - (bx0 + bx1) / 2.0 * s * ax;
        let top = m.y as f32 + cy - (by0 + by1) / 2.0 * s;
        let node = Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(left * 100.0 / W as f32),
            top: Val::Percent(top * 100.0 / H as f32),
            width: Val::Percent(side * ax * 100.0 / W as f32),
            height: Val::Percent(side * 100.0 / H as f32),
            ..default()
        };
        let image = art.images[k][usize::from(side * px_per_y < 96.0)].clone();
        let [r, g, b, a] = m.colour;
        let color = Color::srgba(r, g, b, a);
        match pool.next() {
            Some((mut n, mut img, mut v)) => {
                if *n != node { *n = node; }
                if img.image != image { img.image = image; }
                if img.color != color { img.color = color; }
                v.set_if_neq(Visibility::Inherited);
            }
            None => {
                commands.spawn((node, ImageNode { image, color, ..default() }, ZIndex(10), IconNode, ChildOf(parent), Name::new("sharp button icon")));
            }
        }
    }
    for (_, _, mut v) in pool { v.set_if_neq(Visibility::Hidden); }
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
        app.init_resource::<Icons>()
            .init_resource::<Marks>()
            .add_systems(Startup, load_art)
            .add_systems(PostUpdate, resolve)
            .add_systems(Update, (dump, place.after(crate::hud_render::HudBuild)));
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
    fn art_decodes() {
        for bytes in PS5_ART.iter().chain(&XBOX_ART) {
            let (w, h, rgba) = decode(bytes).expect("png");
            assert_eq!((w, h, rgba.len()), (128, 128, 128 * 128 * 4));
            let b = bounds(w, h, &rgba);
            assert!(b[2] - b[0] > 60.0 && b[3] - b[1] > 40.0, "{b:?}");
            assert_eq!(half(w, h, &rgba).len(), 64 * 64 * 4);
        }
    }

    #[test]
    fn fades_dim_the_icons() {
        let mark = IconMark { code: 0x10, x: 100, y: 100, rgba: 0x8080_8080, scissor: [0, W - 1, 0, H - 1], at: 1, boxed: true };
        let fade = |a: u32| Prim { tex: Tex::None, pos: [[0, 0], [W, 0], [0, H], [W, H]], uv: [[0, 0]; 4], rgba: a << 24, scissor: [0, W - 1, 0, H - 1], repeat: false, nearest: false, boxed: false, uv16: false };
        // Drawn before the icon: no effect; after it: halfway to black.
        assert_eq!(shown(&[fade(0x80), fade(0x40)], &[], &[mark])[0].colour, [0.5, 0.5, 0.5, 1.0]);
        // Off the screen (a page menu's unused panel) or outside its scissor: not drawn.
        assert!(shown(&[], &[], &[IconMark { x: 600, ..mark }]).is_empty());
        assert!(shown(&[], &[], &[IconMark { scissor: [0, 50, 0, 50], ..mark }]).is_empty());
    }

    #[test]
    fn styles_round_trip() {
        for s in IconStyle::ALL { assert_eq!((IconStyle::from_index(s.index()), IconStyle::from_key(s.key())), (s, Some(s))); }
    }
}
