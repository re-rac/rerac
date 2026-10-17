//! The story title cards of the transitions ("Kyzil Plateau, Planet Veldin", "Meanwhile, in a factory on a nearby
//! planet…", …): `PlayStoryTransition(lang, a, b, ticks, load)` (level01 0x2a64e0 = boot `fun_00231bd8`), its texture
//! upload `fun_00231878` (0x2a6180) and its band quad `fun_002316e8` (0x2a5ff0). Spec: docs/plan/cutscenes_transitions.md
//! §4.3; data: TOC `space_plates[lang]` (TOC + 0x1388, one WAD per language: count 18, offsets, each entry a PIF:
//! entry 0 the 64×64 band texture, entry c + 1 the 512×64 card c).
//!
//! | address | what | here |
//! |---|---|---|
//! | 0x2a6180 | `space_plates[lang]` read (`0x138f08 + lang·8`) and decompressed; uploads: the band's palette and 64×64 image (entry 0), card a's palette and 512×64 image, card b's | [`Plates::parse`] (the pictures; the engine uploads them) |
//! | 0x2a64e0 | `load`: `read_file_entry_with_retry(level)`, 0x15ee48 = 0, 0x15ee4a = 0 | the engine (the background load) |
//! | | per frame i < ticks (while the memory card is idle): alpha = `i < 0x20 ? 4·i : 0x80`, `(ticks − i)·8` in the last 16 frames; the band u = (i % 600)/600; a = b: the band and card a at y = cy − 0x20; else card a at cy − 0x2e and, from frame 0x41, card b at cy with alpha `(i − 0x40)·4` until 0x60 | [`CardPlayer::frame`] |
//! | | the band (`fun_002316e8(0, 4.0, u, u + 0.4, 0, y, 0x200, 0x40, alpha << 24 \| 0x808080)`): s 0..4, t u..u + 0.4 of the band texture; the card quad `DrawTexturedQuad(0, y, 0x200, 0x40, 0, 0, 0x200, 0x40)` over it is a black stencil with the letters cut out, drawn opaque (with the band's alpha its black would let the band through the whole strip while fading) | [`CardLine`] |
//! | | `load`: when the loader is not done, the card lasts ≥ i + 0x14; once it is, no more polling | [`CardPlayer::frame`] |
//! | | after the loop `FadeToBlack(2)` | [`CardPlayer::frame`] (`CardFrame::black`) |
//!
//! cy = 0x13e50c, the screen's centre line (208 on NTSC) [L: the writer of 0x13e50c is not traced].

use crate::scene_player::fade_to_black_coverage;

/// 0x13e50c [L].
pub const CENTRE_Y: i32 = 208;
/// The card / band size.
pub const CARD_W: i32 = 0x200;
pub const CARD_H: i32 = 0x40;
/// `FadeToBlack(2)` after the card.
pub const END_FADE: u32 = 2;

/// 0x15ed88 → the `space_plates` entry (`lang − 1`, at least 0).
pub fn lang_index(lang: i32) -> usize { (lang - 1).max(0) as usize }

/// The decoded plates of one language: the band (entry 0) and the cards (entries 1..).
#[derive(Clone, Debug)]
pub struct Plates {
    pub band: rc_formats::texture::Texture,
    pub cards: Vec<rc_formats::texture::Texture>,
}

impl Plates {
    /// A `space_plates` file (WAD or decompressed): count, offsets, PIF entries (raw GS colours, like the HUD atlas).
    pub fn parse(file: &[u8]) -> Option<Plates> {
        let bytes = if rc_formats::wad::is_wad(file) { rc_formats::wad::decompress(file).ok()? } else { file.to_vec() };
        let w = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
        let n = w(0)?;
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let off = w(4 + 4 * k)?;
            let pif = rc_formats::pif::Pif::parse(bytes.get(off..)?).ok()?;
            out.push(pif.decode_raw().ok()?);
        }
        let mut it = out.into_iter();
        let band = it.next()?;
        Some(Plates { band, cards: it.collect() })
    }
}

/// One card line this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardLine {
    /// The card (entry `card + 1`).
    pub card: usize,
    /// Its top (pixels), x 0..0x200.
    pub y: i32,
    /// GS alpha 0..0x80.
    pub alpha: u32,
    /// The band's t range (v of the 64×64 band texture: `u .. u + 0.4`; s 0..4).
    pub band_t: [f32; 2],
}

/// One frame of a card step.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CardFrame {
    pub lines: Vec<CardLine>,
    /// The closing `FadeToBlack(2)`'s coverage over the last card image (0 while the card runs).
    pub black: f32,
    /// The step is over (after this frame).
    pub done: bool,
}

/// `PlayStoryTransition`'s frame loop.
#[derive(Clone, Debug)]
pub struct CardPlayer {
    pub a: usize,
    pub b: usize,
    pub ticks: i32,
    pub load: bool,
    i: i32,
    fade: Option<u32>,
    last: Vec<CardLine>,
}

impl CardPlayer {
    pub fn new(a: usize, b: usize, ticks: i32, load: bool) -> CardPlayer { CardPlayer { a, b, ticks, load, i: 0, fade: None, last: Vec::new() } }

    /// One frame. `loaded`: the level loader is done (`FUN_00257dc8` ≠ 0).
    pub fn frame(&mut self, loaded: bool) -> CardFrame {
        if let Some(k) = self.fade {
            let black = fade_to_black_coverage(END_FADE, k);
            self.fade = if k + 1 < END_FADE { Some(k + 1) } else { None };
            return CardFrame { lines: self.last.clone(), black, done: self.fade.is_none() };
        }
        if self.i >= self.ticks {
            self.fade = Some(0);
            return self.frame(loaded);
        }
        let i = self.i;
        let mut alpha = if i < 0x20 { (i << 2) as u32 } else { 0x80 };
        if self.ticks - 0x10 < i { alpha = ((self.ticks - i) * 8) as u32; }
        let u = (i % 600) as f32 * 0.001_666_666_7;
        let band_t = [u, u + 0.4];
        let mut lines = Vec::new();
        if self.a == self.b {
            lines.push(CardLine { card: self.a, y: CENTRE_Y - 0x20, alpha, band_t });
        } else {
            lines.push(CardLine { card: self.a, y: CENTRE_Y - 0x2e, alpha, band_t });
            if 0x40 < i {
                let a2 = if i < 0x60 { ((i - 0x40) * 4) as u32 } else { alpha };
                lines.push(CardLine { card: self.b, y: CENTRE_Y, alpha: a2, band_t });
            }
        }
        if self.load {
            if !loaded {
                if self.ticks < i + 0x14 { self.ticks = i + 0x14; }
            } else {
                self.load = false;
            }
        }
        self.i += 1;
        self.last = lines.clone();
        CardFrame { lines, black: 0.0, done: false }
    }
}
