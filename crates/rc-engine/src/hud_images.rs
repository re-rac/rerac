//! The 2D pass's streamed pictures (`rc_game::menus::MenuDraw::Image`): the pause pages' pictures (the PIFs of the
//! global lumps `item_images`, `help_*`, `goodies_images`, `skill_images`, `options_ss`, …) and the in-game map's
//! composed picture, drawn by the HUD's primitives from a region of the HUD atlas kept for them.
//!
//! **How the game does it**: the streamed image widget reads a PIF into one of two stream buffers and uploads it as a
//! texture (`fun_00204cf0`: TEX0 PSMT8 + a CT32 palette through `DoGifPaging`'s per-frame upload list). **Here**: the
//! atlas (crate::hud_render) has [`SLOTS`] slots of 512×512 below the HUD frames; a picture is decoded once (raw GS
//! alpha, as every atlas texture: `rc_formats::pif::Pif::decode_raw`) and copied into a free slot (least recently
//! used), the atlas image re-uploaded. The draws sample the slot like any HUD texture (`Tex::Dyn`).

use crate::hud_render::Prim;
use bevy::prelude::*;
use rc_formats::texture::Texture;
use rc_game::menus::ImageSrc;
use std::collections::HashMap;
use std::sync::Arc;

/// Slots in the atlas, each [`SLOT`] texels square, two per row.
pub const SLOTS: usize = 4;
pub const SLOT: u32 = 512;
/// The rows the slots add to the atlas.
pub const ROWS: u32 = SLOT * (SLOTS as u32).div_ceil(2);

/// A filled slot: the picture's key, its size, the frame it was last drawn.
type Slot = ((u32, u64), (u32, u32), u64);

/// The atlas' picture slots.
#[derive(Resource)]
pub struct HudImages {
    /// The HUD atlas and the first row of the slots.
    pub atlas: Handle<Image>,
    pub base_y: u32,
    /// Per slot: the picture's key, its size, the frame it was last drawn.
    slots: [Option<Slot>; SLOTS],
    /// Decoded pictures (None: unreadable).
    cache: HashMap<(u32, u64), Option<Arc<Texture>>>,
    pub frame: u64,
    /// Pictures loaded (log).
    pub loads: u64,
}

impl HudImages {
    pub fn new(atlas: Handle<Image>, base_y: u32) -> HudImages {
        HudImages { atlas, base_y, slots: [None; SLOTS], cache: HashMap::new(), frame: 0, loads: 0 }
    }

    /// The atlas rectangle `[x, y, w, h]` of slot `i` (its picture's size).
    pub fn rect(&self, i: usize) -> Option<[u32; 4]> {
        let (_, (w, h), _) = self.slots.get(i).copied().flatten()?;
        Some([(i as u32 % 2) * SLOT, self.base_y + (i as u32 / 2) * SLOT, w, h])
    }

    /// Every slot's rectangle, for the mesh build.
    pub fn rects(&self) -> [Option<[u32; 4]>; SLOTS] { std::array::from_fn(|i| self.rect(i)) }

    /// The slot holding `src`'s picture, loading it into the least recently drawn slot when absent (None: no picture).
    pub fn resolve(&mut self, src: &ImageSrc, images: &mut Assets<Image>) -> Option<usize> {
        let key = src.key();
        if let Some(i) = self.slots.iter().position(|s| s.is_some_and(|s| s.0 == key)) {
            if let Some(s) = self.slots[i].as_mut() { s.2 = self.frame; }
            return Some(i);
        }
        let tex = self.load(src)?;
        let (w, h) = (tex.width.min(SLOT), tex.height.min(SLOT));
        let i = (0..SLOTS).min_by_key(|&i| self.slots[i].map_or(0, |s| s.2 + 1))?;
        if self.slots[i].is_some_and(|s| s.2 == self.frame) { return None; }
        let [x, y, _, _] = [(i as u32 % 2) * SLOT, self.base_y + (i as u32 / 2) * SLOT, w, h];
        let mut img = images.get_mut(&self.atlas)?;
        let aw = img.texture_descriptor.size.width;
        let data = img.data.as_mut()?;
        for row in 0..h {
            let src_row = &tex.rgba[(row * tex.width * 4) as usize..(row * tex.width * 4 + w * 4) as usize];
            let dst = (((y + row) * aw + x) * 4) as usize;
            data[dst..dst + src_row.len()].copy_from_slice(src_row);
        }
        self.slots[i] = Some((key, (w, h), self.frame));
        self.loads += 1;
        Some(i)
    }

    fn load(&mut self, src: &ImageSrc) -> Option<Arc<Texture>> {
        let key = src.key();
        if let Some(t) = self.cache.get(&key) { return t.clone(); }
        let t = match src {
            ImageSrc::Pixels { tex, .. } => Some(tex.clone()),
            ImageSrc::Lump { field, index } => read_lump_picture(*field, *index).map(Arc::new),
        };
        // Composed pictures change every time their key does: keep only the lumps' decodes.
        if matches!(src, ImageSrc::Lump { .. }) { self.cache.insert(key, t.clone()); }
        t
    }
}

/// Entry `index` of the global TOC field at offset `field`: `global/<name>/NNN.bin`, WAD-compressed, a PIF.
pub fn read_lump_picture(field: u32, index: u32) -> Option<Texture> {
    // The TOC entry at `field + 8·index`: past the field's own count it is an entry of a field that follows (the
    // Epilogue's languages, `rc_game::menus::pause::pages::field_entries`).
    let at = field + 8 * index;
    let f = rc_formats::disc::RAC1_GLOBAL_FIELDS.iter().find(|f| (f.offset as u32..f.offset as u32 + 8 * f.count as u32).contains(&at))?;
    let index = (at - f.offset as u32) / 8;
    let root = crate::level_load::extracted_root();
    let raw = crate::disc_source::read(&root, &format!("global/{}/{index:03}.bin", f.name)).map_err(|e| eprintln!("hud images: {} {index}: {e:#}", f.name)).ok()?;
    let bytes = if rc_formats::wad::is_wad(&raw) { rc_formats::wad::decompress(&raw).ok()? } else { raw.to_vec() };
    rc_formats::pif::Pif::parse(&bytes).and_then(|p| p.decode_raw()).map_err(|e| eprintln!("hud images: {} {index}: {e}", f.name)).ok()
}

/// A picture quad as a primitive of slot `slot` (the caller's panel offset and scissor applied later).
#[allow(clippy::too_many_arguments)]
pub fn prim(slot: usize, x: i32, y: i32, w: i32, h: i32, u: i32, v: i32, tw: i32, th: i32, rgba: u32) -> Prim {
    let (xb, yb, ub, vb) = (x + w, y + h, u + tw, v + th);
    Prim {
        tex: crate::hud_render::Tex::Dyn(slot),
        pos: [[x, y], [xb, y], [x, yb], [xb, yb]],
        uv: [[u, v], [ub, v], [u, vb], [ub, vb]],
        rgba,
        scissor: [0, crate::hud_render::W - 1, 0, crate::hud_render::H - 1],
        repeat: false,
        nearest: false,
        boxed: false,
        uv16: false,
    }
}
