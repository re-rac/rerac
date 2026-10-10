//! The 2D pass: HUD sprites, text quads and rectangles as the game sends them to the GS (PATH2 DIRECT packets,
//! no VU1 program), drawn after all 3D. Spec: docs/plan/hud_text.md §2; the calls come from
//! [`rc_game::hud::HudState`] through [`crate::text_render`].
//!
//! **Primitives** ([`Hud2d`]): `HudSprite` 0x2500e0 (SPRITE, PRIM 0x156, UV (0,0)..(tw,th) stretched over w×h,
//! RGBAQ = `alpha << 24 | 0x7f7f7f`), its 180° / 90° TRISTRIP variants 0x250468 / 0x2506c8 (PRIM 0x154),
//! `DrawTexturedQuad` 0x21be90 (TRISTRIP, glyphs and FX textures, UV = (u, v)..(u + tw, v + th)) and
//! `DrawRectOverlay` 0x21bce0 (untextured TRISTRIP 0x144). All are flat-shaded, blended, at Z 0xfffff0.
//!
//! **Mapping.** A corner at game pixel (x, y) is sent as `X = x·16 + OFX − 8` with `OFX = (2048 − 256)·16`, i.e.
//! at window pixel `x − 0.5`. The GS samples pixel *X* at its integer position, the GPU at `X + 0.5`, so the
//! corner lands exactly on GPU coordinate `x` of a 512×416 target and a w-wide primitive covers pixels
//! `x .. x + w − 1`, as on the GS. UVs are texel·16 (FST): the vertex UV here is in texels; at pixel `px` the
//! interpolated U is `u0 + (px + 0.5 − x)·Δu/Δx`, the GS's value.
//!
//! **Pixel pipeline** (`assets/shaders/hud.wgsl`). Texture: TEX1 bilinear + CLAMP inherited from the AA blit's
//! A+D block (the HUD packets set neither), done by hand on an atlas of every HUD frame and FX texture (raw GS
//! bytes, alpha 0x80 = 1.0): sample position quantised to 1/16 texel (UV is 12.4 fixed point), four texels
//! clamped to the texture's own rectangle, weights `k/16`, result truncated [M]; a primitive drawn after
//! `CLAMP_1 = 0` ([`Prim::repeat`]) wraps them at the texture's size instead (REPEAT); one drawn under TEX1_1 = 1
//! ([`Prim::nearest`]) takes the single texel `floor(U, V)` (point sampling). MODULATE: `C = Ct·Cf >> 7`,
//! `A = At·Af >> 7` (clamped to 0xff), so RGB 0x7f gives `tex × 127/128`. Blend ALPHA_1 0x44
//! `(Cs − Cd)·As/128 + Cd` ([`GsPass::Hud`]: TEST_1 0x5380b passes RGB either way and Z is constant, so one
//! blended draw is exact). Untextured rectangles use the vertex RGBA. Scissor (`FontPrintWindow`) per
//! primitive.
//!
//! **Where it draws.** The primitives render in submission (painter's) order into an offscreen 512×416
//! `Rgba16Float` target on a dedicated `Camera2d` (order −10, [`HUD_LAYER`]), at the game's own resolution:
//! the GS's bilinear filter works on 512×416 pixels, so filtering at the window's 2× resolution would be a
//! different (smoother) image. Blending there runs in display bytes / 255 without clamping, premultiplied
//! (the target keeps `Σ Cs·As` in RGB and the coverage in A), which is exact for HUD-on-HUD overlaps and
//! keeps As > 0x80 (orb glow) as the GS computes it over opaque HUD pixels. The result is composited onto the
//! main camera by a Bevy UI node ([`HudComposite`], `UiMaterial`) filling the camera's letterboxed 512×416
//! viewport (`game_camera::letterbox`), sampled as the HUD option says (crate::graphics: Original bilinear half a pixel
//! to the left, as the game's display copy and the TV showed the 512×416 picture, crate::aa_blit; Sharp pixels
//! nearest), in the UI
//! pass: after every 3D pass and before the underwater tint (`fog_state::UnderwaterTint`, scheduled after
//! `ui_pass`), which is the game's order (the tint is in the `0x15f3f4 & 0x40` pass after the HUD's `& 0x80`).
//! The composite mixes in linear light on the sRGB target (exact where the HUD coverage is 0 or 1; the GS
//! mixes display bytes), the difference the world passes also accept (gs_state.rs).
//!
//! **Static layer** ([`Hud2dHook::statics`]): the screens' and menu panels' noise, scan lines, vignette and glass
//! (rc_game::menus::screen_static) go to a second 512×416 target (camera order −9, [`STATIC_LAYER`]) in three
//! meshes drawn in order: pass 0 and 2 with the GS blend, pass 1 (the noise) with ALPHA_1 0x68 (`Cd + Cs·FIX`,
//! [`HudMaterial::additive`]). Its node ([`HudStaticComposite`]) is a child of the HUD composite over the canvases
//! composed over the HUD, because the game draws the effect after the page's 3D views and inside the vendor's
//! monitor targets; it adds the layer's display-space premultiplied colour in linear light and keeps
//! 1 − coverage of what is under it, exact over black (the monitors, the navy panels).
//!
//! Environment: `RC_HUD=0` disables the HUD; `RC_HUD_DEMO=1` sets bolts to 1234 at tick 60 and drops HP to
//! 3 at tick 180; `RC_HUD_TEXT="…"` shows it as a banner (`ShowBanner` path, 180 ticks) from tick 1, or with
//! `RC_HUD_TEXT_WINDOW=1` as `FontPrintWindow` text (regular font) centred in a `DrawUIFrame` at y = 100;
//! `RC_HUD_HELP=<id>` opens the help box with that level message at the first gameplay tick (crate::gameplay, the
//! help system's first-input gate skipped); `RC_LANG` = game language
//! (0 En, 2 Fr, 3 De, 4 Es, 5 It).

use crate::gs_state::GsPass;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::camera::{ClearColorConfig, Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Extent3d, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::render::view::Msaa;
use bevy::shader::{ShaderDefVal, ShaderRef};
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin, MeshMaterial2d};
use bevy::ui::UiTargetCamera;
use bevy::ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialKey, UiMaterialPlugin};
use rc_formats::font::GlyphTable;
use rc_formats::hud::Hud;
use rc_formats::strings::Message;
use rc_formats::texture::Texture;
use rc_game::hud::{Draw, HudAssets, HudState, Inputs, Rot};
use std::path::Path;

const SHADER_PATH: &str = "shaders/hud.wgsl";
/// Render layer of the offscreen HUD camera and its mesh.
pub const HUD_LAYER: usize = 29;
/// Render layer of the static layer's camera and meshes ([`Hud2dHook::statics`]).
pub const STATIC_LAYER: usize = 30;
/// The GS draw buffer (NTSC).
pub const W: i32 = 512;
pub const H: i32 = 416;
/// Atlas width in texels.
const ATLAS_W: u32 = 1024;

/// Per vertex: texel UV.
pub const ATTRIBUTE_UV: MeshVertexAttribute = MeshVertexAttribute::new("HudUv", 0x4855_4401, VertexFormat::Float32x2);
/// Per vertex: RGBA bytes (R low).
pub const ATTRIBUTE_RGBA: MeshVertexAttribute = MeshVertexAttribute::new("HudRgba", 0x4855_4402, VertexFormat::Uint32);
/// Per vertex: atlas x | y << 16, texture w | h << 16, flags (1 = textured, 2 = REPEAT, 4 = NEAREST), 0.
pub const ATTRIBUTE_TEX: MeshVertexAttribute = MeshVertexAttribute::new("HudTex", 0x4855_4403, VertexFormat::Uint32x4);
/// Per vertex: scissor x0, x1, y0, y1 (inclusive pixels).
pub const ATTRIBUTE_SCISSOR: MeshVertexAttribute = MeshVertexAttribute::new("HudScissor", 0x4855_4404, VertexFormat::Uint32x4);

/// A texture a primitive samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tex {
    None,
    /// HUD frame (`GetFrameTex`).
    Frame(usize),
    /// FX texture n (`GetEffectTex(n)`: 1..3 fonts, 4 the Gadgetron logo).
    Fx(usize),
    /// A streamed picture's atlas slot (crate::hud_images).
    Dyn(usize),
}

/// One GS primitive: four corners in strip order (v0 v1 v2 v3: triangles 012, 123), game pixels, with texel UVs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prim {
    pub tex: Tex,
    pub pos: [[i32; 2]; 4],
    pub uv: [[i32; 2]; 4],
    pub rgba: u32,
    /// x0, x1, y0, y1, inclusive.
    pub scissor: [i32; 4],
    /// CLAMP_1 WMS = WMT = REPEAT (`CLAMP_1 = 0`): texel coordinates wrap at the texture's size instead of clamping to
    /// its edge (the CLAMP the 2D pass inherits from the AA blit). The screens' static sets it
    /// (rc_game::menus::screen_static; [`Hud2dHook::statics`]).
    pub repeat: bool,
    /// TEX1_1 MMAG = MMIN = NEAREST (point sampling) instead of the bilinear the 2D pass inherits: the vendor's ticker,
    /// drawn right after the hologram cone's `FastDrawQuadReal` wrote TEX1_1 = 1 (crate::vendor_render).
    pub nearest: bool,    /// Port-only: clipped to the 512×416 screen even in a 16:9 frame (crate::display): the page menus, whose unused
    /// panels lie off the screen. Other primitives with the full screen's scissor reach the frame's edges there.
    pub boxed: bool,
    /// The texel coordinates are in 1/16 texels (the GS's UV precision): sub-texel scrolls (the planet globe).
    pub uv16: bool,
}

impl Prim {
    /// Bounding rectangle `[x, y, w, h]` (tests).
    #[allow(dead_code)]
    pub fn rect(&self) -> [i32; 4] {
        let (x0, x1) = (self.pos.iter().map(|p| p[0]).min().unwrap(), self.pos.iter().map(|p| p[0]).max().unwrap());
        let (y0, y1) = (self.pos.iter().map(|p| p[1]).min().unwrap(), self.pos.iter().map(|p| p[1]).max().unwrap());
        [x0, y0, x1 - x0, y1 - y0]
    }
}

const FULL_SCISSOR: [i32; 4] = [0, W - 1, 0, H - 1];

/// The frame's 2D primitive list, in draw order.
#[derive(Clone, Debug)]
pub struct Hud2d {
    pub prims: Vec<Prim>,
    scissor: [i32; 4],
    /// Texture size of every HUD frame.
    pub frame_sizes: Vec<(i32, i32)>,
    /// The primitives sent with 1/16-pixel corners (`fun_00200080`, `fun_00200e08(…, 1)`): their index in
    /// [`Hud2d::prims`] and their corners in 1/16 pixels (the prim's own `pos` holds them rounded down).
    pub fine: Vec<(usize, [[i32; 2]; 4])>,
    /// The primitives (indices into `prims`, ascending) drawn under ALPHA_1 0x48 (`Cs·As + Cd`) instead of 0x44: the
    /// vehicle HUDs' screen primitives after their gauge (`ScreenPrim::add`).
    pub add: Vec<usize>,
}

impl Default for Hud2d {
    fn default() -> Self { Hud2d { prims: Vec::new(), scissor: FULL_SCISSOR, frame_sizes: Vec::new(), fine: Vec::new(), add: Vec::new() } }
}

impl Hud2d {
    pub fn clear(&mut self) {
        self.prims.clear();
        self.fine.clear();
        self.add.clear();
        self.scissor = FULL_SCISSOR;
    }

    /// A primitive with 1/16-pixel corners (strip order).
    fn push_fine(&mut self, tex: Tex, pos16: [[i32; 2]; 4], uv: [[i32; 2]; 4], rgba: u32) {
        self.fine.push((self.prims.len(), pos16));
        self.push(tex, pos16.map(|p| [p[0] >> 4, p[1] >> 4]), uv, rgba);
    }

    /// `fun_00200080(frame, x, y, w, h, alpha)` (0x250928): the whole frame over a `w×h` rectangle at (x, y), all in
    /// 1/16 pixels (the same `v + OF − 8` convention as `HudSprite`).
    pub fn sprite_fine(&mut self, frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32) {
        let (tw, th) = self.frame_sizes.get(frame).copied().unwrap_or((0, 0));
        let rgba = ((alpha as u32) & 0xff) << 24 | 0x007f_7f7f;
        let (x1, y1) = (x + w, y + h);
        self.push_fine(Tex::Frame(frame), [[x, y], [x1, y], [x, y1], [x1, y1]], [[0, 0], [tw, 0], [0, th], [tw, th]], rgba);
    }

    /// `fun_00200e08(x0, y0, x1, y1, rgba, 1)` (0x2516b0): an untextured blended rectangle between two corners in 1/16
    /// pixels.
    pub fn rect_fine(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, rgba: u32) {
        self.push_fine(Tex::None, [[x0, y0], [x1, y0], [x0, y1], [x1, y1]], [[0, 0]; 4], rgba);
    }

    /// `HudSpriteSubRect(frame, x, y, w, h, alpha)` (0x2502c8): the first `w×h` texels of the frame at their own size.
    pub fn sprite_sub(&mut self, frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32) {
        let rgba = ((alpha as u32) & 0xff) << 24 | 0x007f_7f7f;
        let (x1, y1) = (x + w, y + h);
        self.push(Tex::Frame(frame), [[x, y], [x1, y], [x, y1], [x1, y1]], [[0, 0], [w, 0], [0, h], [w, h]], rgba);
    }

    /// A frame callback's screen primitive (`rc_game`'s `ScreenPrim`: pixels, its corners kept to 1/16 pixel).
    pub fn screen_prim(&mut self, tex: Tex, pos: [[f32; 2]; 4], uv: [[f32; 2]; 4], rgba: u32, add: bool) {
        if add { self.add.push(self.prims.len()); }
        let pos16 = pos.map(|p| [(p[0] * 16.0) as i32, (p[1] * 16.0) as i32]);
        self.push_fine(tex, pos16, uv.map(|u| [u[0] as i32, u[1] as i32]), rgba);
    }

    /// `VU1_setScissor(x0, x1, y0, y1)`.
    pub fn set_scissor(&mut self, x0: i32, x1: i32, y0: i32, y1: i32) { self.scissor = [x0, x1, y0, y1]; }
    pub fn reset_scissor(&mut self) { self.scissor = FULL_SCISSOR; }

    fn push(&mut self, tex: Tex, pos: [[i32; 2]; 4], uv: [[i32; 2]; 4], rgba: u32) {
        self.prims.push(Prim { tex, pos, uv, rgba, scissor: self.scissor, repeat: false, nearest: false, boxed: false, uv16: false });
    }

    /// `HudSprite(frame, x, y, w, h, alpha)` and its rotated variants.
    #[allow(clippy::too_many_arguments)]
    pub fn sprite(&mut self, frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32, rot: Rot) {
        let (tw, th) = self.frame_sizes.get(frame).copied().unwrap_or((0, 0));
        let rgba = ((alpha as u32) & 0xff) << 24 | 0x007f_7f7f;
        let (x1, y1) = (x + w, y + h);
        let (pos, uv) = match rot {
            Rot::None => ([[x, y], [x1, y], [x, y1], [x1, y1]], [[0, 0], [tw, 0], [0, th], [tw, th]]),
            Rot::R180 => ([[x, y1], [x, y], [x1, y1], [x1, y]], [[tw, 0], [tw, th], [0, 0], [0, th]]),
            Rot::R90 => ([[x, y], [x1, y], [x, y1], [x1, y1]], [[tw, 0], [tw, th], [0, 0], [0, th]]),
        };
        self.push(Tex::Frame(frame), pos, uv, rgba);
    }

    /// `DrawTexturedQuad(x, y, w, h, u, v, tw, th, rgba, GetEffectTex(fx))`.
    #[allow(clippy::too_many_arguments)]
    pub fn strip_glyph(&mut self, fx: usize, x: i32, y: i32, w: i32, h: i32, u: i32, v: i32, tw: i32, th: i32, rgba: u32) {
        let (x1, y1, u1, v1) = (x + w, y + h, u + tw, v + th);
        self.push(Tex::Fx(fx), [[x, y], [x1, y], [x, y1], [x1, y1]], [[u, v], [u1, v], [u, v1], [u1, v1]], rgba);
    }

    /// `DrawRectOverlay(top, bottom, left, right, rgba)`: pixels `left..right−1` × `top..bottom−1`.
    pub fn rect(&mut self, top: i32, bottom: i32, left: i32, right: i32, rgba: u32) {
        self.push(Tex::None, [[left, top], [right, top], [left, bottom], [right, bottom]], [[0, 0]; 4], rgba);
    }
}

/// One FX texture as stored: 8-bit indices and its 256-entry CLUT (CSM1 order).
pub struct IndexedFx {
    pub width: u32,
    pub height: u32,
    pub indices: Vec<u8>,
    pub clut: Vec<u8>,
}

/// The FX texture decoded with the entries of the logical indices from `cut` on opaque black (0x80000000): the CLUT a
/// vehicle gauge leaves (`rc_game::moby_update::classes::draw_callbacks::ScreenTex::FxCut`). Raw GS alpha, as the atlas.
pub fn decode_cut(t: &IndexedFx, cut: u8) -> Texture {
    let n = t.width as usize * t.height as usize;
    let mut rgba = Vec::with_capacity(n * 4);
    for &ix in t.indices.iter().take(n) {
        if ix >= cut {
            rgba.extend_from_slice(&[0, 0, 0, 0x80]);
        } else {
            let e = rc_formats::texture::clut_index(ix as u32) as usize * 4;
            rgba.extend_from_slice(t.clut.get(e..e + 4).unwrap_or(&[0; 4]));
        }
    }
    Texture { width: t.width, height: t.height, rgba }
}

/// The level's HUD data.
pub struct LevelHud {
    pub hud: Hud,
    /// Every frame with raw GS alpha.
    pub frames: Vec<Texture>,
    /// FX textures (raw alpha), index = `GetEffectTex` argument.
    pub fx: Vec<Option<Texture>>,
    /// The FX textures' stored indices and CLUTs (the vehicle gauges redraw theirs with a cut CLUT: [`apply_fx_cuts`]).
    pub fx_indexed: Vec<Option<IndexedFx>>,
    pub glyphs: [GlyphTable; 3],
    pub glyph_addrs: [u32; 3],
    pub messages: Vec<Message>,
    pub lang: u32,
}

/// 0x15ed88, the game's language: `RC_LANG` (default English) until the front end's Language list changes it
/// ([`set_language`]); every later load (the level's text, the help voice bank, the movies' and scenes' language, the
/// space plates) reads it.
pub fn language() -> u32 {
    let v = LANGUAGE.load(std::sync::atomic::Ordering::Relaxed);
    if v != u32::MAX { return v; }
    std::env::var("RC_LANG").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(rc_formats::strings::lang::ENGLISH)
}

/// The runtime language (u32::MAX: not set, `RC_LANG` applies).
static LANGUAGE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);

/// 0x15ed88 = `lang` (`MenuSetPostAction` action 9).
pub fn set_language(lang: u32) { LANGUAGE.store(lang, std::sync::atomic::Ordering::Relaxed); }

/// Reads `hud_header`, the banks, the overlay's glyph tables, the FX textures and the level text.
pub fn load(root: &Path, index: u32, core: &rc_formats::level::LevelCore, core_index: &[u8], core_data: &[u8], gameplay: &[u8]) -> Result<LevelHud> {
    use rc_formats::{font, hud, particle_tex, strings};
    let read = |name: &str| crate::disc_source::level_file(root, index, name);
    let header = read("hud_header.bin")?;
    let h = hud::parse_header(&header)?;
    let mut banks: [Vec<u8>; hud::BANKS] = Default::default();
    for (b, bank) in banks.iter_mut().enumerate() {
        if h.bank_size[b] != 0 { *bank = rc_data::hud_bank(root, index, b).with_context(|| format!("hud bank {b}"))?.to_vec(); }
    }
    let hud = hud::parse_hud(&header, std::array::from_fn(|b| banks[b].as_slice())).context("parsing hud")?;
    let frames = (0..hud.frames.len()).map(|i| hud.decode_frame_raw(i)).collect::<rc_formats::buf::Result<Vec<_>>>()?;
    let (glyphs, glyph_addrs) = font::parse_glyph_tables(&read("overlay.bin")?).context("glyph tables")?;
    let fx_entries = particle_tex::parse_particle_textures(core, core_index, core_data).context("fx textures")?.fx_entries;
    let fx_bank = core.blocks.iter().find(|b| b.name == "fx_bank").map(|b| &core_data[b.offset..b.offset + b.size]).unwrap_or(&[]);
    let fx = fx_entries
        .iter()
        .map(|e| {
            if !e.present() { return Ok(None); }
            let px = fx_bank.get(e.texture as usize..).context("fx pixels")?;
            let clut = fx_bank.get(e.palette as usize..).context("fx palette")?;
            Ok(Some(hud::decode_indexed8_raw(px, e.width as u32, e.height as u32, clut)?))
        })
        .collect::<Result<Vec<_>>>()?;
    let fx_indexed = fx_entries
        .iter()
        .map(|e| {
            if !e.present() { return None; }
            let n = e.width as usize * e.height as usize;
            let indices = fx_bank.get(e.texture as usize..e.texture as usize + n)?.to_vec();
            let clut = fx_bank.get(e.palette as usize..e.palette as usize + 1024)?.to_vec();
            Some(IndexedFx { width: e.width as u32, height: e.height as u32, indices, clut })
        })
        .collect();
    let lang = language();
    let messages = strings::parse_strings(gameplay, lang).context("level text")?;
    Ok(LevelHud { hud, frames, fx, fx_indexed, glyphs, glyph_addrs, messages, lang })
}

/// Rectangles of every texture in the atlas.
struct Atlas {
    rgba: Vec<u8>,
    height: u32,
    frames: Vec<[u32; 4]>,
    fx: Vec<Option<[u32; 4]>>,
}

/// Shelf-packs the frames and FX textures (tallest first) into one raw-byte image.
fn build_atlas(frames: &[Texture], fx: &[Option<Texture>]) -> Atlas {
    let mut items: Vec<(usize, &Texture)> = frames.iter().enumerate().chain(fx.iter().enumerate().filter_map(|(i, t)| t.as_ref().map(|t| (frames.len() + i, t)))).collect();
    items.sort_by_key(|(i, t)| (std::cmp::Reverse(t.height), std::cmp::Reverse(t.width), *i));
    let mut rects = vec![[0u32; 4]; frames.len() + fx.len()];
    let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
    for (i, t) in &items {
        if x + t.width > ATLAS_W {
            x = 0;
            y += shelf;
            shelf = 0;
        }
        rects[*i] = [x, y, t.width, t.height];
        x += t.width;
        shelf = shelf.max(t.height);
    }
    let height = (y + shelf).max(1);
    let mut rgba = vec![0u8; (ATLAS_W * height * 4) as usize];
    for (i, t) in &items {
        let [rx, ry, w, _] = rects[*i];
        for row in 0..t.height {
            let src = &t.rgba[(row * w * 4) as usize..((row + 1) * w * 4) as usize];
            let dst = (((ry + row) * ATLAS_W + rx) * 4) as usize;
            rgba[dst..dst + src.len()].copy_from_slice(src);
        }
    }
    let fx_rects = (0..fx.len()).map(|i| fx[i].as_ref().map(|_| rects[frames.len() + i])).collect();
    rects.truncate(frames.len());
    Atlas { rgba, height, frames: rects, fx: fx_rects }
}

/// Pipeline key of [`HudMaterial`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HudMaterialKey {
    additive: bool,
}

impl From<&HudMaterial> for HudMaterialKey {
    fn from(m: &HudMaterial) -> Self { HudMaterialKey { additive: m.additive } }
}

/// ALPHA_1 0x44 and 0x48 on a premultiplied fragment (`Cs·As`, coverage As, or 0 under 0x48).
const PREMULT_BLEND: BlendState = BlendState {
    color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::OneMinusSrcAlpha, operation: BlendOperation::Add },
    alpha: BlendComponent::OVER,
};

/// ALPHA_1 0x68 with FIX: `Cd + Cs·FIX/128` (the fragment outputs `Cs·FIX/128` and coverage 0, so the target's
/// premultiplied colour grows and its coverage stays).
const ADD_FIX_BLEND: BlendState = BlendState {
    color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
    alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
};

/// The 2D primitives' material: the atlas; `additive` = ALPHA_1 0x68 (FIX = the vertex alpha) instead of 0x44.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(HudMaterialKey)]
pub struct HudMaterial {
    #[texture(0)]
    pub atlas: Handle<Image>,
    pub additive: bool,
}

impl Material2d for HudMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode2d { AlphaMode2d::Blend }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, layout: &MeshVertexBufferLayoutRef, key: Material2dKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_UV.at_shader_location(1),
            ATTRIBUTE_RGBA.at_shader_location(2),
            ATTRIBUTE_TEX.at_shader_location(3),
            ATTRIBUTE_SCISSOR.at_shader_location(4),
        ])?];
        descriptor.vertex.shader_defs.push(ShaderDefVal::Bool("HUD_PRIMS".into(), true));
        if let Some(f) = descriptor.fragment.as_mut() { f.shader_defs.push(ShaderDefVal::Bool("HUD_PRIMS".into(), true)); }
        descriptor.primitive.cull_mode = None;
        GsPass::Hud.specialize(descriptor);
        // The GS blends 0x44 / 0x48 as `Cs·As + Cd·(1 − As)` / `Cs·As + Cd`: the fragment outputs `Cs·As` and its coverage
        // (0 for a 0x48 primitive), blended premultiplied.
        if let Some(f) = descriptor.fragment.as_mut() {
            for t in f.targets.iter_mut().flatten() { t.blend = Some(PREMULT_BLEND); }
        }
        if key.bind_group_data.additive {
            if let Some(f) = descriptor.fragment.as_mut() {
                f.shader_defs.push("HUD_ADD".into());
                for t in f.targets.iter_mut().flatten() { t.blend = Some(ADD_FIX_BLEND); }
            }
        }
        Ok(())
    }
}

/// The UI node that puts the 512×416 HUD image on the main camera (premultiplied → straight alpha), scaled as the
/// HUD option says ([`scaling`]).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HudComposite {
    #[texture(0)]
    pub image: Handle<Image>,
    #[uniform(1)]
    pub scaling: Vec4,
}

impl UiMaterial for HudComposite {
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {
        if let Some(f) = descriptor.fragment.as_mut() {
            f.entry_point = Some("composite".into());
            f.shader_defs.push(ShaderDefVal::Bool("HUD_COMPOSITE".into(), true));
        }
        descriptor.vertex.shader_defs.push(ShaderDefVal::Bool("HUD_COMPOSITE".into(), true));
    }
}

/// The static layer's UI node (a child of the HUD composite, over the canvases composed over the HUD): its image holds
/// display-space premultiplied colour and coverage; the node adds the colour in linear light over what is under it,
/// scaled by 1 − coverage (`(Cs − Cd)·As + Cd` exactly where the effect lies on black — the monitors, the navy
/// panels — and `Cd + Cs·FIX` for the noise wherever its coverage is 0).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HudStaticComposite {
    #[texture(0)]
    pub image: Handle<Image>,
    #[uniform(1)]
    pub scaling: Vec4,
}

/// The composites' scaling uniform for the HUD option (crate::graphics): x = 1 bilinear (Original: the TV showed the
/// 512×416 image smoothed), 0 nearest (Sharp pixels).
pub fn scaling(h: crate::graphics::Hud) -> Vec4 { Vec4::new(if h == crate::graphics::Hud::Original { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0) }

/// Applies a changed HUD option to both composites.
fn apply_scaling(gfx: Res<crate::graphics::GraphicsSettings>, mut a: ResMut<Assets<HudComposite>>, mut b: ResMut<Assets<HudStaticComposite>>) {
    if !gfx.is_changed() { return; }
    let v = scaling(gfx.hud);
    for (_, m) in a.iter_mut() { if m.scaling != v { m.scaling = v; } }
    for (_, m) in b.iter_mut() { if m.scaling != v { m.scaling = v; } }
}

impl UiMaterial for HudStaticComposite {
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {
        if let Some(f) = descriptor.fragment.as_mut() {
            f.entry_point = Some("composite_static".into());
            f.shader_defs.push(ShaderDefVal::Bool("HUD_COMPOSITE".into(), true));
            for t in f.targets.iter_mut().flatten() { t.blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING); }
        }
        descriptor.vertex.shader_defs.push(ShaderDefVal::Bool("HUD_COMPOSITE".into(), true));
    }
}

/// Demo / debug requests from the environment.
#[derive(Clone, Debug, Default)]
struct HudEnv {
    demo: bool,
    text: Option<Vec<u8>>,
    text_window: bool,
    help: Option<i32>,
}

impl HudEnv {
    fn read() -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        HudEnv {
            demo: var("RC_HUD_DEMO").as_deref() == Some("1"),
            text: var("RC_HUD_TEXT").map(|s| s.into_bytes()),
            text_window: var("RC_HUD_TEXT_WINDOW").as_deref() == Some("1"),
            help: var("RC_HUD_HELP").and_then(|v| v.trim().parse().ok()),
        }
    }
}

#[derive(Resource)]
struct HudRuntime {
    state: HudState,
    /// The 2D layer's targets (resized with [`Self::extra`]) and the side margin in game pixels (crate::display: the
    /// layer covers the whole frame; the 512-wide screen sits in its middle).
    target: Handle<Image>,
    static_target: Handle<Image>,
    extra: i32,
    env: HudEnv,
    glyphs: [GlyphTable; 3],
    atlas_frames: Vec<[u32; 4]>,
    atlas_fx: Vec<Option<[u32; 4]>>,
    mesh: Handle<Mesh>,
    /// The static layer's three passes ([`Hud2dHook::statics`]).
    static_meshes: [Handle<Mesh>; 3],
    /// What [`Self::mesh`] and the three [`Self::static_meshes`] were last built from ([`Built`]).
    built: [Built; 4],
    ticks_done: u64,
    draws: Vec<Draw>,
    hud2d: Hud2d,
    game: Inputs,
    /// The last pickup banner shown (`rc_game::moby_update::classes::pickup::Banner::seq`).
    banner_seq: u32,
    /// The last `rc_game::cinematic::Cinematic::banner` call shown.
    cine_banner_seq: u32,
    /// The last [`HudFeed::reset`] applied.
    reset: u32,
    /// The last [`HudFeed::prompt_released`] applied.
    prompt_released: u32,
    /// The last game-side HUD call applied (`rc_game::hud::Calls::since`, `Services::hud`).
    calls_cursor: Option<u64>,
    /// The calls not applied yet (kept while the HUD loop is frozen: the page menus, scenes).
    pending: Vec<rc_game::hud::Call>,
    /// The CLUT cut each FX texture's atlas region holds (none: the stored CLUT) ([`apply_fx_cuts`]).
    fx_cuts: std::collections::HashMap<usize, u8>,
}

/// The inputs a HUD mesh was last built from. [`tick_and_build`] re-inserts a mesh only when they change: every
/// inserted `Mesh` makes Bevy re-check all the mesh entities of every material for specialization (about 2.5 ms per
/// frame over a level's ~25k entities, docs/plan/performance.md F5), and most frames draw the same HUD.
#[derive(Default)]
struct Built {
    prims: Vec<Prim>,
    fine: Vec<(usize, [[i32; 2]; 4])>,
    add: Vec<usize>,
    dyns: Vec<Option<[u32; 4]>>,
    extra: i32,
}

impl Built {
    /// True (and the new inputs kept) when they differ from the last build's.
    fn update(&mut self, prims: &[Prim], fine: &[(usize, [[i32; 2]; 4])], add: &[usize], dyns: &[Option<[u32; 4]>], extra: i32) -> bool {
        if self.prims == prims && self.fine == fine && self.add == add && self.dyns == dyns && self.extra == extra { return false; }
        fn copy<T: Copy>(v: &mut Vec<T>, new: &[T]) { v.clear(); v.extend_from_slice(new); }
        copy(&mut self.prims, prims);
        copy(&mut self.fine, fine);
        copy(&mut self.add, add);
        copy(&mut self.dyns, dyns);
        self.extra = extra;
        true
    }
}

#[derive(Component)]
struct HudMesh;

/// The HUD composite's UI node (crate::screen_canvas puts the canvases composed over the HUD under it as its children).
#[derive(Component)]
pub(crate) struct HudCompositeNode;

/// The 512×416 screen's box inside the HUD composite (crate::display's 4:3 box).
#[derive(Component)]
pub(crate) struct HudBoxNode;

pub struct HudPlugin;

/// What the "use" system feeds the HUD (crate::interact_render, set before [`HudBuild`]): the context prompt of
/// slot 12 (`PromptTick` / `NpcTalkUpdate` requests and the text buffer 0x17e9b0), and while the vendor is open
/// its bolt counter (slot 2 | 0x10) and ammo slot (slot 0: the selected ammo entry, `Some(None)` = none).
#[derive(Resource, Default)]
pub struct HudFeed {
    pub prompt: bool,
    pub prompt_text: Vec<u8>,
    pub bolts_pinned: bool,
    pub weapon: Option<Option<(u16, i32, i32)>>,
    /// Bumped by `FUN_0024fb00` callers (the vendor's open): every HUD slot is emptied at once.
    pub reset: u32,
    /// `rc_game::moby_update::interact::Prompt::released`: the prompt's slot emptied at once on a change.
    pub prompt_released: u32,
    /// The slot calls the engine-side callers made this frame (the quick select's opening and `PageMenuClose`: health and
    /// bolts kept up, `rc_game::hud::Call::ShowHealthBolts`); applied and cleared before the HUD's next tick.
    pub calls: Vec<rc_game::hud::Call>,
}

/// Other 2D layers drawn through this pass (crate::menu_render: the quick-select ring, which is HUD slot 3,
/// and the mode-3 page menus). Set before [`HudBuild`] each frame.
#[derive(Resource, Default)]
pub struct Hud2dHook {
    /// Appended after the HUD's own primitives.
    pub prims: Vec<Prim>,
    /// The screens' and panels' static (rc_game::menus::screen_static): its own layer over everything 2D and the
    /// canvases (the game draws it after the page's 3D views and inside the monitors' targets), in three passes:
    /// 0 under the noise, 1 the noise (ALPHA_1 0x68, added), 2 over it. Set by crate::menu_render and
    /// crate::vendor_render each frame.
    pub statics: [Vec<Prim>; 3],
    /// The HUD's own calls are not drawn (mode 3 draws no HUD).
    pub replace_hud: bool,
    /// The HUD update loop does not run (it is part of the mode-0 render only).
    pub freeze: bool,
}

/// The mode-2 scene layer (crate::scene_render), separate from [`Hud2dHook`] (which the menus rewrite every
/// frame): while a scene runs the draw mask is 0x7f, so the HUD neither updates nor draws, and the subtitle
/// box primitives (`fun_001f4be0`) are appended last. Set before [`HudBuild`] each frame.
#[derive(Resource, Default)]
pub struct SceneLayer {
    pub hide_hud: bool,
    pub prims: Vec<Prim>,
}

/// The system set that ticks the HUD and builds the primitive mesh.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HudBuild;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hud2dHook>().init_resource::<SceneLayer>().init_resource::<HudFeed>();
        // A runtime level change (crate::level_switch): the level's HUD is built again by `setup`.
        app.add_systems(crate::level_switch::LevelUnload, (crate::level_switch::reset::<Hud2dHook>, crate::level_switch::reset::<SceneLayer>, crate::level_switch::reset::<HudFeed>, crate::level_switch::remove::<HudRuntime>, crate::level_switch::remove::<crate::hud_images::HudImages>));
        if std::env::var("RC_HUD").is_ok_and(|v| v.trim() == "0") { return; }
        app.add_plugins((Material2dPlugin::<HudMaterial>::default(), UiMaterialPlugin::<HudComposite>::default(), UiMaterialPlugin::<HudStaticComposite>::default()))
            .add_systems(crate::level_switch::LevelStartup, setup)
            .add_systems(Update, (target_main_camera, tick_and_build, apply_fx_cuts).chain().in_set(HudBuild))
            .add_systems(PostUpdate, apply_scaling);
    }
}

fn setup(
    mut commands: Commands,
    level: Res<crate::Level>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HudMaterial>>,
    (mut composites, mut static_composites): (ResMut<Assets<HudComposite>>, ResMut<Assets<HudStaticComposite>>),
    gfx: Res<crate::graphics::GraphicsSettings>,
) {
    let sc = scaling(gfx.hud);
    let Some(lh) = level.0.hud.as_ref() else {
        eprintln!("hud: no HUD data for this level");
        return;
    };
    let mut atlas = build_atlas(&lh.frames, &lh.fx);
    // The streamed pictures' slots below the frames (crate::hud_images); the image stays in the main world so the slots
    // can be rewritten.
    let dyn_y = atlas.height;
    atlas.height += crate::hud_images::ROWS;
    atlas.rgba.resize((ATLAS_W * atlas.height * 4) as usize, 0);
    let mut img = Image::new(
        Extent3d { width: ATLAS_W, height: atlas.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        atlas.rgba,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::all(),
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    let atlas_handle = images.add(img);
    commands.insert_resource(crate::hud_images::HudImages::new(atlas_handle.clone(), dyn_y));
    let target = images.add(Image::new_target_texture(W as u32, H as u32, TextureFormat::Rgba16Float, None));

    commands.spawn((
        Camera2d,
        Camera { order: -10, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::Image(target.clone().into()),
        Hdr,
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        RenderLayers::layer(HUD_LAYER),
        Name::new("hud camera (512x416 offscreen)"),
    ));
    let mesh = meshes.add(empty_mesh());
    commands.spawn((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(HudMaterial { atlas: atlas_handle.clone(), additive: false })),
        Transform::IDENTITY,
        NoFrustumCulling,
        RenderLayers::layer(HUD_LAYER),
        HudMesh,
        Name::new("hud primitives"),
    ));
    let full = || Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    // On the frame's 4:3 box (crate::display: in 16:9 the 512×416 screen is centred, the world widens around it).
    // The whole frame (crate::display): the 512×416 screen in its middle, wider in 16:9 ([`HudRuntime::extra`]).
    let hud_node = commands.spawn((full(), MaterialNode(composites.add(HudComposite { image: target.clone(), scaling: sc })), GlobalZIndex(i32::MAX), HudCompositeNode, Name::new("hud composite"))).id();
    // The 512×416 screen's box inside it: the parent of the canvases composed with the HUD (crate::screen_canvas).
    commands.spawn((full(), crate::display::UiBoxed, HudBoxNode, ChildOf(hud_node), Name::new("hud box")));
    // The static layer: its own 512×416 target, three meshes in pass order (Transparent2d sorts by z), a node over
    // the HUD composite's other children (the canvases composed over the HUD).
    let static_target = images.add(Image::new_target_texture(W as u32, H as u32, TextureFormat::Rgba16Float, None));
    commands.spawn((
        Camera2d,
        Camera { order: -9, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::Image(static_target.clone().into()),
        Hdr,
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        RenderLayers::layer(STATIC_LAYER),
        Name::new("hud static camera (512x416 offscreen)"),
    ));
    let (mix, add) = (materials.add(HudMaterial { atlas: atlas_handle.clone(), additive: false }), materials.add(HudMaterial { atlas: atlas_handle, additive: true }));
    let static_meshes = [0usize, 1, 2].map(|k| {
        let m = meshes.add(empty_mesh());
        commands.spawn((
            Mesh2d(m.clone()),
            MeshMaterial2d(if k == 1 { add.clone() } else { mix.clone() }),
            Transform::from_xyz(0.0, 0.0, k as f32),
            NoFrustumCulling,
            RenderLayers::layer(STATIC_LAYER),
            Name::new(format!("hud static pass {k}")),
        ));
        m
    });
    commands.spawn((full(), MaterialNode(static_composites.add(HudStaticComposite { image: static_target.clone(), scaling: sc })), ZIndex(1), ChildOf(hud_node), Name::new("hud static composite")));

    let assets = HudAssets::new(&lh.hud, lh.glyphs, lh.messages.clone());
    let frame_sizes = assets.frame_sizes.clone();
    let env = HudEnv::read();
    println!(
        "hud: {} icons, {} frames, {} fx textures, glyph tables {:#x}/{:#x}/{:#x}, {} messages (language {}), atlas {}x{}{}{}{}",
        lh.hud.icons.len() - 1, lh.frames.len(), lh.fx.iter().flatten().count(), lh.glyph_addrs[0], lh.glyph_addrs[1], lh.glyph_addrs[2],
        lh.messages.len(), lh.lang, ATLAS_W, atlas.height,
        if env.demo { "; RC_HUD_DEMO: bolts 1234 at tick 60, HP 3 at tick 180" } else { "" },
        env.text.as_ref().map_or(String::new(), |t| format!("; RC_HUD_TEXT {:?}{}", String::from_utf8_lossy(t), if env.text_window { " (window)" } else { "" })),
        env.help.map_or(String::new(), |id| format!("; RC_HUD_HELP {id}: {}", rc_formats::strings::display(rc_formats::strings::lookup(&lh.messages, id)))),
    );
    commands.insert_resource(HudRuntime {
        state: HudState::new(assets),
        target,
        static_target,
        extra: 0,
        env,
        glyphs: lh.glyphs,
        atlas_frames: atlas.frames,
        atlas_fx: atlas.fx,
        mesh,
        static_meshes,
        built: Default::default(),
        ticks_done: 0,
        draws: Vec::new(),
        hud2d: Hud2d { frame_sizes, ..default() },
        // Until the first frame reads the game state (Persistent / Session / HeldWeapon): 4/4, no bolts, no slot.
        game: Inputs { lang: lh.lang, ..Default::default() },
        banner_seq: 0,
        cine_banner_seq: 0,
        reset: 0,
        prompt_released: 0,
        calls_cursor: None,
        pending: Vec::new(),
        fx_cuts: Default::default(),
    });
}

/// The composite node goes to the main (fly) camera's UI pass.
fn target_main_camera(mut commands: Commands, nodes: Query<Entity, (With<HudCompositeNode>, Without<UiTargetCamera>)>, cams: Query<Entity, With<crate::fly_cam::FlyCam>>) {
    let Some(cam) = cams.iter().next() else { return };
    for n in &nodes { commands.entity(n).insert(UiTargetCamera(cam)); }
}

/// Runs the HUD for every 60 Hz tick since the last frame, then rebuilds the primitive mesh.
/// The game state the HUD shows (crate::gameplay's resources).
type GameInputs<'w> = (Option<Res<'w, crate::gameplay::Persistent>>, Option<Res<'w, crate::gameplay::Session>>, Option<Res<'w, crate::gameplay::HeldWeapon>>);

#[allow(clippy::too_many_arguments)]
fn tick_and_build(
    rt: Option<ResMut<HudRuntime>>,
    ticks: Res<crate::determinism::GameTicks>,
    hook: Res<Hud2dHook>,
    scene: Res<SceneLayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    (state, session, held): GameInputs,
    mut feed: ResMut<HudFeed>,
    (mut play, mut audio): (Option<ResMut<crate::gameplay::Play>>, Option<ResMut<crate::audio_out::AudioOut>>),
    dyn_images: Option<Res<crate::hud_images::HudImages>>,
    (display, mut images): (Option<Res<crate::display::DisplaySettings>>, ResMut<Assets<Image>>),
    (mut composites, mut static_composites, mut targets): (ResMut<Assets<HudComposite>>, ResMut<Assets<HudStaticComposite>>, Query<&mut RenderTarget>),
) {
    let Some(mut rt) = rt else { return };
    let rt = &mut *rt;
    // The side margin of the frame's aspect (crate::display): the layer's targets and the HUD's left / right slots.
    let extra = display.map_or(0, |d| crate::display::side_extra(d.aspect));
    if extra != rt.extra {
        rt.extra = extra;
        rt.state.side_extra = extra;
        // Fresh targets of the new width (crate::display::fresh_target); the two cameras and composites move to them.
        let size = UVec2::new((W + 2 * extra) as u32, H as u32);
        for old in [rt.target.clone(), rt.static_target.clone()] {
            let Some(new) = crate::display::fresh_target(&mut images, &old, size) else { continue };
            for mut t in &mut targets {
                if matches!(&*t, RenderTarget::Image(i) if i.handle.id() == old.id()) { *t = RenderTarget::Image(new.clone().into()); }
            }
            for (_, m) in composites.iter_mut() { if m.image.id() == old.id() { m.image = new.clone(); } }
            for (_, m) in static_composites.iter_mut() { if m.image.id() == old.id() { m.image = new.clone(); } }
            if old.id() == rt.target.id() { rt.target = new; } else { rt.static_target = new; }
        }
    }
    // The slot calls since the last frame: the game's (the classes', `Services::hud`) then the engine's (the menus').
    if let Some(p) = play.as_deref() {
        let (c, cursor) = p.svc.hud.since(rt.calls_cursor);
        rt.pending.extend(c);
        rt.calls_cursor = cursor;
    }
    rt.pending.append(&mut feed.calls);
    // The pickups' banner (`ShowBannerf` in the ammo pickup 0x2db028: "+n" of the item's ammo text).
    if let Some(b) = play.as_ref().map(|p| p.svc.pickups_banner).filter(|b| b.seq != rt.banner_seq) {
        rt.banner_seq = b.seq;
        rt.state.show_bannerf(b.text, b.arg);
    }
    // `ShowBanner(msg, ticks)` of the moby loop (the gold bolt, the planet banners: rc_game::cinematic::show_banner).
    if let Some(b) = play.as_ref().map(|p| p.svc.cinematic.banner).filter(|b| b.seq != rt.cine_banner_seq) {
        rt.cine_banner_seq = b.seq;
        rt.state.show_banner_msg(b.msg, b.ticks);
    }
    // The game's values (bolts 0x15ed98, max HP 0x15eda0, HP 0x1415f8, the held item's ammo slot), unless the
    // RC_HUD_DEMO values drive it.
    if !rt.env.demo {
        if let Some(gs) = &state {
            rt.game.bolts = gs.0.global.bolts;
            rt.game.max_hp = gs.0.global.max_hp;
            rt.game.ammo = gs.0.global.ammo;
        }
        if let Some(s) = &session { rt.game.hp = s.0.hp; }
        rt.game.weapon = held.as_ref().and_then(|h| h.0);
        // The vendor's own slot-0 requests (`rc_game::hud::HudState`'s vendor part), not the held weapon's.
        rt.game.vendor = feed.weapon.map(|w| w.map(|(item, _, max)| (item, max)));
        // Ratchet's state 0x1413d4 (mounted, 0x32: the health and bolt draws skip).
        if let Some(p) = play.as_deref() {
            rt.game.hero_state = p.game.hero.state;
            // The body (rc_game::hero::bodies): Clank's orbs, Giant Clank's no health.
            (rt.game.body, rt.game.clank_max) = (p.game.hero.mode, p.game.hero.bodies.clank_health);
            hero_inputs(&mut rt.game, p);
        }
    }
    if feed.reset != rt.reset {
        rt.reset = feed.reset;
        rt.state.reset_slots();
    }
    rt.state.set_prompt(feed.prompt, &feed.prompt_text);
    if feed.prompt_released != rt.prompt_released {
        rt.prompt_released = feed.prompt_released;
        if rt.state.release_prompt() { rt.draws = rt.state.draws_now(); }
    }
    rt.state.bolts_pinned = feed.bolts_pinned;
    let target = ticks.0;
    if hook.freeze || scene.hide_hud { rt.ticks_done = target; }
    // Catch up at most a second of ticks per frame.
    if target > rt.ticks_done + 60 { rt.ticks_done = target - 60; }
    while rt.ticks_done < target {
        rt.ticks_done += 1;
        let t = rt.ticks_done;
        if t == 1 {
            if let Some(text) = rt.env.text.clone().filter(|_| !rt.env.text_window) { rt.state.show_banner(&text, None); }
        }
        if rt.env.demo {
            if t == 60 { rt.game.bolts = 1234; }
            if t == 180 { rt.game.hp = 3; }
        }
        // The help box of the game tick (rc_game::help runs in crate::gameplay's tick; the HUD draws it).
        if let Some(p) = play.as_deref() { rt.state.set_help(&p.svc.help.bx); }
        if !rt.pending.is_empty() { rt.state.apply_calls(&std::mem::take(&mut rt.pending)); }
        rt.game.tick = play.as_deref().map_or(t, |p| p.game.counter.saturating_sub(target - t));
        rt.draws = rt.state.tick(rt.game);
        // Ratchet's class sounds of the HUD (`0x236738(11, 0)`: the low-air beep of the oxygen meter's blink).
        if let (Some(p), Some(a)) = (play.as_deref_mut(), audio.as_deref_mut()) {
            for &index in &rt.state.sounds { play_hero_sound(p, a, index); }
        }
        if let Some(text) = rt.env.text.clone().filter(|_| rt.env.text_window) { window_text_demo(&rt.state, &rt.glyphs, &text, &mut rt.draws); }
    }
    rt.hud2d.clear();
    // The guns' screen markers (`DrawWorld`'s 2D overlay before the HUD: crate::marker_render).
    if let Some(p) = play.as_deref().filter(|_| !hook.replace_hud && !scene.hide_hud) { rt.hud2d.prims.extend(crate::marker_render::prims(p)); }
    let mut st = crate::text_render::TextState::default();
    // The moby draw callbacks' 2-D layer (the vehicles' HUDs: rc_game draw_callbacks::screen / screen_texts), drawn by
    // `DrawWorld` with the mobys' callbacks, before the HUD.
    if let Some(p) = play.as_deref().filter(|_| !hook.replace_hud && !scene.hide_hud) {
        use rc_game::moby_update::classes::draw_callbacks::ScreenTex;
        let cb = &p.svc.draw_callbacks;
        for q in &cb.screen {
            let tex = match q.tex {
                ScreenTex::None => Tex::None,
                ScreenTex::Fx(n) | ScreenTex::FxCut { fx: n, .. } => Tex::Fx(n),
            };
            rt.hud2d.screen_prim(tex, q.pos, q.uv, q.rgba, q.add);
        }
        for t in &cb.screen_texts {
            let g = rt.glyphs[t.font as usize];
            let width = rc_formats::font::measure_text_width(&t.text, t.len, &g);
            crate::text_render::font_print(&mut rt.hud2d, &mut st, &g, t.font, t.x - (width >> 1), t.y, t.rgba, &t.text, t.len);
        }
    }
    // HudDraw 0x24fb50 skips the HUD in a frame whose tick set 0x17e988 (the Visibomb's flight: crate::visibomb_view).
    let hud_off = crate::visibomb_view::hud_off(play.as_deref());
    if !hook.replace_hud && !scene.hide_hud && !hud_off { crate::text_render::execute(&mut rt.hud2d, &mut st, &rt.glyphs, &rt.draws); }
    rt.hud2d.prims.extend(hook.prims.iter().copied());
    rt.hud2d.prims.extend(scene.prims.iter().copied());
    let dyns = dyn_images.as_deref().map_or([None; crate::hud_images::SLOTS], |d| d.rects());
    let e = rt.extra;
    // Only the meshes whose inputs changed ([`Built`]).
    if rt.built[0].update(&rt.hud2d.prims, &rt.hud2d.fine, &rt.hud2d.add, &dyns, e) {
        let _ = meshes.insert(&rt.mesh, build_mesh(&rt.hud2d.prims, &rt.hud2d.fine, &rt.hud2d.add, &rt.atlas_frames, &rt.atlas_fx, &dyns, e));
    }
    for ((m, prims), b) in rt.static_meshes.iter().zip(&hook.statics).zip(&mut rt.built[1..]) {
        if b.update(prims, &[], &[], &dyns, e) { let _ = meshes.insert(m, build_mesh(prims, &[], &[], &rt.atlas_frames, &rt.atlas_fx, &dyns, e)); }
    }
}

/// The HUD inputs the hero and the classes hold (rc_game::hud::Inputs: the callers the HUD sees through the state).
fn hero_inputs(g: &mut Inputs, p: &crate::gameplay::Play) {
    let h = &p.game.hero;
    let react = &p.svc.creatures.react;
    let morph = &h.weapons.reactive.morph;
    g.level = p.svc.level as i32;
    g.mode = p.svc.game_mode;
    g.pressed = p.game.pad.pressed;
    g.hero_pos = [h.pos[0].to_f32(), h.pos[1].to_f32(), h.pos[2].to_f32()];
    g.group = h.group;
    g.o2_mask = h.owned.has(rc_game::hero::swim::ITEM_O2_MASK);
    g.oxygen = h.swim.oxygen;
    g.held_item = h.items.slot.id;
    // `0x303000` runs from the slot loop for a Suck Cannon in the hand (not being put away), on foot.
    g.suck_active = h.mode == 0 && h.items.slot.item.is_some() && h.items.slot.id == 9 && h.items.slot.state != 3;
    (g.suck_held, g.suck_gold) = (react.held, react.gold);
    (g.morph_hud, g.morph_target, g.morph_value) = (morph.hud, morph.target.map_or(-1, |t| t as i32), morph.hud_value);
    (g.energy, g.beam_lock) = (h.bodies.energy, h.bodies.beam_lock);
    g.bolt_alert = p.svc.buried.alert;
    // The race records the Hoverboard writes (0x15ee58.. / 0x15ee68..: rc_game::moby_update::classes::units::hoverboard).
    let r = &p.svc.board.records;
    (g.race_best_time, g.race_best_score) = (r.best_time, r.best_score);
    let b = &h.board;
    (g.race_lap, g.race_place, g.race_ticks, g.race_score) = (b.lap as i32, b.place, b.race_ticks, b.score);
    g.race_won = p.svc.interact.game.flags.first().is_some_and(|&f| f != 0);
    g.race_weapons = b.weapons;
    g.race_meter = rc_game::hero::hoverboard::MeterView { meter: b.meter, wrong_way: b.wrong_way, fuel: b.fuel };
}

/// `0x236738(index, 0)`: Ratchet's class sound (`PlayClassSound` on the hero moby), as `HeroClassSounds::voice`.
fn play_hero_sound(p: &mut crate::gameplay::Play, audio: &mut crate::audio_out::AudioOut, index: i32) {
    let hero = p.game.hero.hero_moby(p.game.hero_moby);
    let Some(m) = p.game.mobys.mobys.get(hero) else { return };
    let ev = rc_game::moby_update::services::SoundEvent {
        index,
        flags: 0,
        moby: hero,
        o_class: m.o_class,
        sound_class: m.o_class,
        pos: [m.position[0], m.position[1], m.position[2]],
        tick: p.game.counter,
    };
    let listener = rc_game::audio::class_sounds::listener_of(&p.game.camera.out);
    audio.system().play_class_sound(&ev, Some(hero), &listener, &mut p.game.rng);
}

/// `RC_HUD_TEXT_WINDOW=1`: the text in a `DrawUIFrame` sized from a `FontPrintWindow` measure (regular font,
/// window x 44..468, anchor 256, centred block on y = 100, line height 16), as the help box sizes itself.
fn window_text_demo(state: &HudState, glyphs: &[GlyphTable; 3], text: &[u8], out: &mut Vec<Draw>) {
    use rc_formats::font::Font;
    use rc_game::hud::text;
    let font = Font::Regular;
    let mut win = text::Window::new(0, 416, 0x2c, 0x1d4, 0x100, 100, 0x10, text::CENTRE_LINES | text::CENTRE_BLOCK | text::MEASURE_ONLY);
    text::layout(&mut win, text, -1, &glyphs[font as usize], true);
    let (hw, hh) = ((win.max_width >> 1) as i32 + 10, (win.height >> 1) as i32 + 5);
    let _ = state;
    out.push(Draw::UiFrame { top: 100 - hh, bottom: 100 + hh, left: 256 - hw, right: 256 + hw, alpha: 0x60 });
    win.flags = text::CENTRE_LINES | text::CENTRE_BLOCK;
    out.push(Draw::TextWindow { font, window: win, rgba: 0x80f0_f0f0, text: text.to_vec() });
}

fn empty_mesh() -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 3])
        .with_inserted_attribute(ATTRIBUTE_UV, vec![[0.0f32; 2]; 3])
        .with_inserted_attribute(ATTRIBUTE_RGBA, VertexAttributeValues::Uint32(vec![0; 3]))
        .with_inserted_attribute(ATTRIBUTE_TEX, VertexAttributeValues::Uint32x4(vec![[0; 4]; 3]))
        .with_inserted_attribute(ATTRIBUTE_SCISSOR, VertexAttributeValues::Uint32x4(vec![[1, 0, 1, 0]; 3]))
        .with_inserted_indices(Indices::U32(vec![0, 1, 2]))
}

/// The primitives as one triangle list in submission order (the GPU blends triangles of one draw in order).
///
/// `extra` (port-only, crate::display): game pixels the layer extends past the 512-wide screen on each side. The
/// screen is centred in the `512 + 2·extra` wide target (x + extra, scaled into the shader's 512-wide clip mapping;
/// the scissors are target pixels); a primitive spanning the whole screen (x ≤ 0 to x ≥ 512: fades, letterbox
/// bars, the menu's black) reaches the frame's edges, scissor included when it is the full screen's.
fn build_mesh(prims: &[Prim], fine: &[(usize, [[i32; 2]; 4])], add: &[usize], frames: &[[u32; 4]], fx: &[Option<[u32; 4]>], dyns: &[Option<[u32; 4]>], extra: i32) -> Mesh {
    if prims.is_empty() { return empty_mesh(); }
    let mut fine = fine.iter().peekable();
    let mut add = add.iter().peekable();
    let n = prims.len() * 4;
    let (mut pos, mut uv, mut rgba, mut tex, mut sc, mut idx) =
        (Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n / 4 * 6));
    for (i, p) in prims.iter().enumerate() {
        let pos16 = fine.next_if(|(k, _)| *k == i).map(|(_, q)| *q);
        let additive = add.next_if(|&&k| k == i).is_some();
        let rect = match p.tex {
            Tex::None => None,
            Tex::Frame(i) => frames.get(i).copied(),
            Tex::Fx(i) => fx.get(i).copied().flatten(),
            Tex::Dyn(i) => dyns.get(i).copied().flatten(),
        };
        let blend = if additive { 8 } else { 0 };
        let t = match rect {
            Some([x, y, w, hh]) => [x | y << 16, w | hh << 16, 1 | if p.repeat { 2 } else { 0 } | if p.nearest { 4 } else { 0 } | blend, 0],
            None => [0, 0x0001_0001, blend, 0],
        };
        let (x_lo, x_hi) = (p.pos.iter().map(|q| q[0]).min().unwrap_or(0), p.pos.iter().map(|q| q[0]).max().unwrap_or(0));
        let spans = extra != 0 && x_lo <= 0 && x_hi >= W;
        let mut sc4 = p.scissor;
        if extra != 0 {
            // The full screen's scissor opens to the frame's edges (the HUD's side slots move out there), except on
            // the boxed menu pages, whose unused panel rectangles lie off the screen; a primitive spanning the screen
            // stretches to the edges either way.
            if (spans || !p.boxed) && sc4[0] <= 0 && sc4[1] >= W - 1 { (sc4[0], sc4[1]) = (-extra, W - 1 + extra); }
            sc4[0] += extra;
            sc4[1] += extra;
        }
        let s = sc4.map(|v| v.clamp(-1, 0xffff) as u32);
        let kx = W as f32 / (W + 2 * extra) as f32;
        let wide_x = |x: f32| -> f32 {
            let x = if spans && x <= 0.0 { x - extra as f32 } else if spans && x >= W as f32 { x + extra as f32 } else { x };
            if extra == 0 { x } else { (x + extra as f32) * kx }
        };
        let base = pos.len() as u32;
        for k in 0..4 {
            pos.push(match pos16 {
                Some(q) => [wide_x(q[k][0] as f32 / 16.0), q[k][1] as f32 / 16.0, 0.0],
                None => [wide_x(p.pos[k][0] as f32), p.pos[k][1] as f32, 0.0],
            });
            let k16 = if p.uv16 { 1.0 / 16.0 } else { 1.0 };
            uv.push([p.uv[k][0] as f32 * k16, p.uv[k][1] as f32 * k16]);
            rgba.push(p.rgba);
            tex.push(t);
            sc.push([s[0], s[1], s[2], s[3]]);
        }
        idx.extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(ATTRIBUTE_UV, uv)
        .with_inserted_attribute(ATTRIBUTE_RGBA, VertexAttributeValues::Uint32(rgba))
        .with_inserted_attribute(ATTRIBUTE_TEX, VertexAttributeValues::Uint32x4(tex))
        .with_inserted_attribute(ATTRIBUTE_SCISSOR, VertexAttributeValues::Uint32x4(sc))
        .with_inserted_indices(Indices::U32(idx))
}

/// The FX textures the frame's screen primitives draw with a cut CLUT (`ScreenTex::FxCut`, the vehicle gauges): their
/// atlas regions re-decoded when the cut changes. The game's CLUT edit persists, so the other draws of the texture see
/// it too, as here.
fn apply_fx_cuts(rt: Option<ResMut<HudRuntime>>, level: Res<crate::Level>, play: Option<Res<crate::gameplay::Play>>, dyn_images: Option<Res<crate::hud_images::HudImages>>, mut images: ResMut<Assets<Image>>) {
    use rc_game::moby_update::classes::draw_callbacks::ScreenTex;
    let (Some(mut rt), Some(p), Some(di), Some(lh)) = (rt, play, dyn_images, level.0.hud.as_ref()) else { return };
    for q in &p.svc.draw_callbacks.screen {
        let ScreenTex::FxCut { fx, cut } = q.tex else { continue };
        if rt.fx_cuts.get(&fx) == Some(&cut) { continue; }
        let (Some(Some(src)), Some(Some([rx, ry, w, h]))) = (lh.fx_indexed.get(fx), rt.atlas_fx.get(fx).copied()) else { continue };
        let t = decode_cut(src, cut);
        let Some(mut img) = images.get_mut(&di.atlas) else { return };
        let Some(data) = img.data.as_mut() else { return };
        for row in 0..h.min(t.height) {
            let src_row = &t.rgba[(row * t.width * 4) as usize..((row * t.width + w.min(t.width)) * 4) as usize];
            let dst = (((ry + row) * ATLAS_W + rx) * 4) as usize;
            if let Some(d) = data.get_mut(dst..dst + src_row.len()) { d.copy_from_slice(src_row); }
        }
        rt.fx_cuts.insert(fx, cut);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_variants_map_uv_like_the_gs_packets() {
        let mut h = Hud2d { frame_sizes: vec![(32, 32)], ..default() };
        h.sprite(0, 10, 20, 64, 32, 0x80, Rot::None);
        h.sprite(0, 10, 20, 32, 32, 0x40, Rot::R180);
        h.sprite(0, 10, 20, 32, 32, 0x1ff, Rot::R90);
        let p = &h.prims;
        assert_eq!((p[0].rect(), p[0].rgba), ([10, 20, 64, 32], 0x807f_7f7f));
        // 180°: the top-left corner shows texel (tw, th).
        assert_eq!((p[1].pos[1], p[1].uv[1]), ([10, 20], [32, 32]));
        assert_eq!((p[1].pos[2], p[1].uv[2]), ([42, 52], [0, 0]));
        // RGBAQ keeps the low alpha byte.
        assert_eq!(p[2].rgba, 0xff7f_7f7f);
        h.rect(5, 9, 1, 3, 0x6004_0404);
        assert_eq!(h.prims[3].rect(), [1, 5, 2, 4]);
    }

    #[test]
    fn repeat_and_nearest_reach_the_vertex_flags() {
        // CLAMP + bilinear (the 2D pass' default), a REPEAT noise quad (CLAMP_1 = 0), a point-sampled one (TEX1_1 = 1),
        // an untextured rect.
        let mut h = Hud2d::default();
        h.strip_glyph(26, 0, 0, 8, 8, 0, 0, 8, 8, 0x8080_8080);
        h.strip_glyph(26, 0, 0, 120, 40, 150, 90, 270, 130, 0x8080_8080);
        h.prims[1].repeat = true;
        h.strip_glyph(26, 0, 0, 120, 40, 150, 90, 270, 130, 0x8080_8080);
        h.prims[2].repeat = true;
        h.prims[2].nearest = true;
        h.rect(0, 4, 0, 4, 0x8000_0000);
        let fx = vec![None; 26].into_iter().chain([Some([64u32, 0, 32, 32])]).collect::<Vec<_>>();
        let m = build_mesh(&h.prims, &[], &[], &[], &fx, &[], 0);
        let Some(VertexAttributeValues::Uint32x4(t)) = m.attribute(ATTRIBUTE_TEX) else { panic!("no tex attribute") };
        assert_eq!([t[0][2], t[4][2], t[8][2], t[12][2]], [1, 3, 7, 0]);
    }

    #[test]
    fn atlas_packs_without_overlap() {
        let tex = |w: u32, h: u32, v: u8| Texture { width: w, height: h, rgba: vec![v; (w * h * 4) as usize] };
        let frames = vec![tex(32, 32, 1), tex(256, 256, 2), tex(1024, 8, 3), tex(64, 16, 4)];
        let fx = vec![None, Some(tex(256, 128, 5))];
        let a = build_atlas(&frames, &fx);
        let rects: Vec<[u32; 4]> = a.frames.iter().copied().chain(a.fx.iter().flatten().copied()).collect();
        for (i, r) in rects.iter().enumerate() {
            assert!(r[0] + r[2] <= ATLAS_W && r[1] + r[3] <= a.height);
            for s in &rects[i + 1..] {
                assert!(r[0] + r[2] <= s[0] || s[0] + s[2] <= r[0] || r[1] + r[3] <= s[1] || s[1] + s[3] <= r[1], "{r:?} overlaps {s:?}");
            }
        }
        let [x, y, ..] = a.fx[1].unwrap();
        assert_eq!(a.rgba[((y * ATLAS_W + x) * 4) as usize], 5);
    }
}
