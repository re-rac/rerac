//! The game's text and UI-frame drawing on top of [`crate::hud_render::Hud2d`]: `FontPrint` (level01 0x21ccf0),
//! `FontPrintWindow` (0x21db48), `DrawUIFrame` (0x21c958), and the executor for the draw calls
//! [`rc_game::hud::HudState`] returns. Spec: docs/plan/hud_text.md §3.
//!
//! `FontPrint` per byte `c` (stops at the NUL or after `len` bytes; `len ≤ 0`… `len < 0` runs to the NUL,
//! `len == 0` prints nothing):
//! * 0x08..0x0f: colour code. With the colour-code switch 0x15f45c on (set by the level loader), the colour
//!   becomes `(colour & 0xff000000) | (table[c − 8] & 0xffffff)` (table 0x16ccb8, slot 0 = the caller's colour);
//!   no glyph, no advance either way;
//! * `advance == 0`: skipped;
//! * 0x80..0xa7: the accent glyph `g[c + 0x40]` first, 16×16 at `(x + accent.advance, y + accent.y_off)`;
//! * `c < 0x20` (pad icons): 24×16 at `(x, y + y_off)` in grey `avg(R, G, B)` with the colour's alpha;
//!   `c > 0x20`: 16×16 at `(x, y + y_off)`; space: advance only;
//! * `x += advance`.
//!
//! Every quad is `DrawTexturedQuad` with the font's FX texture, UV = the atlas cell at 1:1.
//! `FontPrintWindow` sets the scissor to the window, locks colour slot 0 to its caller's colour (0x15f460),
//! lays the text out with [`rc_game::hud::text::layout`] and prints each line with `FontPrint`, starting in the
//! colour slot the layout remembered for it. The float-position variant (flag 8, `0x21d0f0`) is drawn like the
//! integer path (no caller here uses it). Drop shadows are drawn by the callers (a black copy at +1, +1).

use crate::hud_render::Hud2d;
use rc_formats::font::{Font, GlyphTable, COLOUR_TABLE};
use rc_game::hud::{text, Draw};

/// The global text state `FontPrint` reads.
#[derive(Clone, Copy, Debug)]
pub struct TextState {
    /// 0x15f45c: colour codes honoured (1 during play; `FUN_0021cc38` / `FUN_0021cc28` toggle it).
    pub colour_codes: bool,
    /// 0x15f460: slot 0 of the colour table is not overwritten by `FontPrint` (inside `FontPrintWindow`).
    pub window_lock: bool,
    /// 0x16ccb8.
    pub table: [u32; 8],
}

impl Default for TextState {
    fn default() -> Self { TextState { colour_codes: true, window_lock: false, table: COLOUR_TABLE } }
}

/// `FontPrint(x, y, rgba, text, len)` with the glyph table and FX texture of `font`.
#[allow(clippy::too_many_arguments)]
pub fn font_print(out: &mut Hud2d, st: &mut TextState, glyphs: &GlyphTable, font: Font, x: i32, y: i32, rgba: u32, text: &[u8], len: i32) {
    if !st.window_lock { st.table[0] = rgba; }
    let at = |i: usize| text.get(i).copied().unwrap_or(0);
    if len == 0 || at(0) == 0 { return; }
    let fx = font.fx_texture();
    let (mut x, mut colour) = (x, rgba);
    let mut i = 0usize;
    loop {
        let c = at(i);
        if c.wrapping_sub(8) < 8 {
            if st.colour_codes { colour = (colour & 0xff00_0000) | (st.table[(c - 8) as usize] & 0x00ff_ffff); }
        } else if let Some(g) = glyphs.get(c as usize).filter(|g| g.advance != 0) {
            if c.wrapping_add(0x80) < 0x28 {
                let e = glyphs[c as usize + 0x40];
                out.strip_glyph(fx, x + e.advance as i32, y + e.y_off as i32, 16, 16, e.u as i32, e.v as i32, 16, 16, colour);
            }
            if c < 0x20 {
                let avg = ((colour & 0xff) + ((colour >> 8) & 0xff) + ((colour >> 16) & 0xff)) / 3;
                let grey = (colour & 0xff00_0000).wrapping_add(avg << 16).wrapping_add(avg << 8).wrapping_add(avg);
                if out.sharp_icons && crate::button_icons::sharp() && crate::button_icons::CODES.contains(&c) {
                    out.icon(c, x, y + g.y_off as i32, grey);
                } else {
                    out.strip_glyph(fx, x, y + g.y_off as i32, 24, 16, g.u as i32, g.v as i32, 24, 16, grey);
                }
            } else if c > 0x20 {
                out.strip_glyph(fx, x, y + g.y_off as i32, 16, 16, g.u as i32, g.v as i32, 16, 16, colour);
            }
            x += g.advance as i32;
        }
        i += 1;
        if i as i32 == len || at(i) == 0 { break; }
    }
}

/// `FontPrintWindow(window, rgba, text, len)`; fills the window's measured width and height.
#[allow(clippy::too_many_arguments)]
pub fn font_print_window(out: &mut Hud2d, st: &mut TextState, glyphs: &GlyphTable, font: Font, win: &mut text::Window, rgba: u32, text: &[u8], len: i32) {
    out.set_scissor(win.x_min as i32, win.x_max as i32 - 1, win.y_min as i32, win.y_max as i32 - 1);
    st.window_lock = true;
    let lines = text::layout(win, text, len, glyphs, st.colour_codes);
    if win.flags & text::MEASURE_ONLY == 0 {
        for l in lines {
            st.table[0] = rgba;
            let colour = st.table[l.colour as usize & 7];
            font_print(out, st, glyphs, font, l.x, l.y, colour, text.get(l.start..).unwrap_or(&[]), l.count);
        }
    }
    st.window_lock = false;
    out.reset_scissor();
}

/// `DrawUIFrame(top, bottom, left, right, alpha)`: an RGB (4, 4, 4) rectangle with three stepped columns on each
/// side that round its ends.
pub fn draw_ui_frame(out: &mut Hud2d, top: i32, bottom: i32, left: i32, right: i32, alpha: i32) {
    let c = (alpha as u32) << 24 | 0x0004_0404;
    out.rect(top, bottom, left, right, c);
    out.rect(top + 1, bottom - 1, left - 2, left, c);
    out.rect(top + 2, bottom - 2, left - 3, left - 2, c);
    out.rect(top + 4, bottom - 4, left - 4, left - 3, c);
    out.rect(top + 1, bottom - 1, right, right + 2, c);
    out.rect(top + 2, bottom - 2, right + 2, right + 3, c);
    out.rect(top + 4, bottom - 4, right + 3, right + 4, c);
}

/// Draws the HUD's calls in order.
pub fn execute(out: &mut Hud2d, st: &mut TextState, glyphs: &[GlyphTable; 3], draws: &[Draw]) {
    for d in draws {
        match d {
            Draw::Sprite { frame, x, y, w, h, alpha, rot } => out.sprite(*frame, *x, *y, *w, *h, *alpha, *rot),
            Draw::Text { font, x, y, rgba, text } => font_print(out, st, &glyphs[*font as usize], *font, *x, *y, *rgba, text, -1),
            Draw::TextWindow { font, window, rgba, text } => {
                let mut w = *window;
                font_print_window(out, st, &glyphs[*font as usize], *font, &mut w, *rgba, text, -1);
            }
            Draw::UiFrame { top, bottom, left, right, alpha } => draw_ui_frame(out, *top, *bottom, *left, *right, *alpha),
            Draw::FxQuad { fx, x, y, w, h, u, v, tw, th, rgba } => out.strip_glyph(*fx, *x, *y, *w, *h, *u, *v, *tw, *th, *rgba),
            Draw::Sprite16 { frame, x, y, w, h, alpha } => out.sprite_fine(*frame, *x, *y, *w, *h, *alpha),
            Draw::Rect16 { x0, y0, x1, y1, rgba } => out.rect_fine(*x0, *y0, *x1, *y1, *rgba),
            Draw::SpriteSub { frame, x, y, w, h, alpha } => out.sprite_sub(*frame, *x, *y, *w, *h, *alpha),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud_render::Tex;
    use rc_formats::font::{Glyph, GLYPHS};

    fn glyphs() -> GlyphTable {
        let mut t = [Glyph::default(); GLYPHS];
        t[b'A' as usize] = Glyph { u: 0, v: 0, y_off: 0, advance: 14 };
        t[b' ' as usize] = Glyph { u: 0, v: 0, y_off: 0, advance: 5 };
        t[0x11] = Glyph { u: 32, v: 96, y_off: -2, advance: 20 };
        t[0x8c] = Glyph { u: 64, v: 32, y_off: 0, advance: 11 };
        t[0x8c + 0x40] = Glyph { u: 200, v: 96, y_off: -4, advance: 2 };
        t
    }

    #[test]
    fn font_print_rules() {
        let g = glyphs();
        let mut out = Hud2d::default();
        let mut st = TextState::default();
        font_print(&mut out, &mut st, &g, Font::Small, 10, 20, 0x8010_2030, b"A \x0cA\x11\x8c\x08A", -1);
        let q: Vec<_> = out.prims.iter().map(|p| (p.tex, p.rect(), p.rgba)).collect();
        assert_eq!(q.len(), 6);
        assert_eq!(q[0], (Tex::Fx(2), [10, 20, 16, 16], 0x8010_2030));
        // Orange after 0x0c, keeps alpha.
        assert_eq!(q[1], (Tex::Fx(2), [29, 20, 16, 16], 0x8040_80c0));
        // Pad icon: 24×16, grey = avg(0xc0, 0x80, 0x40) = 0x80.
        assert_eq!(q[2], (Tex::Fx(2), [43, 18, 24, 16], 0x8080_8080));
        // Accent (at x + its advance, y + its y_off) before the letter.
        assert_eq!(q[3], (Tex::Fx(2), [65, 16, 16, 16], 0x8040_80c0));
        assert_eq!(q[4], (Tex::Fx(2), [63, 20, 16, 16], 0x8040_80c0));
        // 0x08 back to the caller's colour.
        assert_eq!(q[5], (Tex::Fx(2), [74, 20, 16, 16], 0x8010_2030));
        // len limits the bytes.
        let mut out = Hud2d::default();
        font_print(&mut out, &mut st, &g, Font::Small, 0, 0, 0x8080_8080, b"AAAA", 2);
        assert_eq!(out.prims.len(), 2);
    }

    #[test]
    fn ui_frame_rects() {
        let mut out = Hud2d::default();
        draw_ui_frame(&mut out, 100, 140, 50, 150, 0x60);
        assert_eq!(out.prims.len(), 7);
        assert_eq!(out.prims[0].rect(), [50, 100, 100, 40]);
        assert_eq!(out.prims[3].rect(), [46, 104, 1, 32]);
        assert!(out.prims.iter().all(|p| p.rgba == 0x6004_0404 && p.tex == Tex::None));
    }
}
