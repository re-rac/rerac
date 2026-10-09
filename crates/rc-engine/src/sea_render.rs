//! The seas and the other liquid surfaces the level code draws from a draw callback (docs/plan/world_animation.md §3.3;
//! the class updates, their data and state: `rc_game::water::sea`). Every tick, each registered `Callback::Sea(port)`
//! becomes one or more draw groups of the shared draw-callback material (crate::fx_draw::FxPrimMaterial, its
//! `PrimBuf`): textured, vertex-coloured, fogged GS strips, in callback order. `RC_SEA=0` turns the drawing off.
//!
//! **The liquid grid** (`0x2ce830(state)`: `0x2ce130`, `0x2775c8`, `0x277900`, `0x2ce720`). Per block (7×7 cells,
//! row-major) with a non-zero flag whose centre (`origin + (7b + 3.5)·cell`, z = the record's z) is nearer the camera
//! than √(record w) (400 on every level): one strip of the module's 124 vertices, grid point `order[k]` at
//! `origin + (7b + col, 7r + row)·cell` and the record's z, ST = `st[k]·scale`, RGBA = the block's colour k. The texture
//! is the module's 64×64 image (`0x277b20`): frames `a = tex + (c/p mod n)` and `b = tex + ((c/p + 1) mod n)` of the FX
//! textures (c = the tick counter `0x15f5cc`, p = ticks per frame), `t = c/p − ⌊c/p⌋` (`f32`), each texel
//! `trunc(A·(1 − t) + B·t)` per byte with A, B = the **stored** CLUT entry of the texel's index (the EE indexes the CLUT
//! as stored, not in the GS's CSM1 order), sent as PSMCT24. GS: `ALPHA = FIX << 32 | 0x64`, TEST 0x5360a (FIX ≥ 0x61:
//! no alpha test, Z written) or 0x5370b (vertex A = 0 fails AREF 0x70 with RGB_ONLY: colour only), MODULATE, fog on with
//! the record's own fog (`+0x20..+0x32`: near 10000 / far 260080 integer units on every level, FOGCOL per level); the
//! restore puts the level fog back. The port draws FIX 0x80 as an opaque world surface (`(Cs − Cd)·1 + Cd = Cs`, Z
//! written) and FIX < 0x61 as a display-blend effect with `As = FIX` (the texture's alpha set to 0x80 and the vertex
//! alpha to FIX: `At·Av >> 7 = FIX`) and no Z.
//!
//! **The ocean 1111** (`0x30a908`, list 1). F, L = the camera's forward and left rows (0x1679d0) flattened (z = 0) and
//! normalised; N0/N1 = cam − 75F ∓ 87L, F0/F1 = cam + reach·F ∓ 750L, all at the sea z (`0x161e18`):
//! 1. the far band: one 12-vertex strip `L(t), R(t)` for t = 0, 0.2 … 1 (`N0 + t(F0 − N0)`, `N1 + t(F1 − N1)`), colour
//!    `FastTweenColor(t, pvar 0, pvar 1)`, ST 0, FX 0x16, ALPHA 0x3200000044 (the vertex alpha mixes), TEST 0x513f1;
//! 2. while `pvar 5 < cam z − pvar 9`: the wall `FastDrawQuadReal` F0, F1 from the sea up to `cam z − pvar 9`, colours
//!    pvar 1 (bottom) / pvar 2 (top), FX 0x16, ALPHA 0x44 (the hero-state 0x1d / `0x16d180` exceptions: not modelled);
//! 3. with `f = min(1 − (cam z − sea z)/pvar 11, 1) ≥ 0`: the surface, per layer (FX `pvar 7 + layer + 0x28`, ALPHA
//!    0x8000000048 additive): four rows of four S×S quads around the camera (x, y from −2S by S, at `cam + (x, y) + 1`),
//!    ST `x/(2S)·scale + scroll`, colour `FastTweenColor(min(d/(2S), 1), tween(f, 0, pvar 3), tween(f, 0, pvar 4))`, d =
//!    the vertex's distance to the camera. The callback's fog-intensity override (`0x16d668/c`) is restored before any
//!    `UpdateViewContext` reads it, so the level fog applies [L].
//!
//! **The Hoven liquid 1901** (`0x30be68`, after-ties list): FIX 0x7f `0x64` blend with TEST 0x5360a (Z written) for group
//! 1 (FX 0x2c, while the camera is in the moby's cuboid; ST + its scroll) and group 2 layer 0 (FX 0x2f, ST + scroll 0),
//! then group 2 layer 1 with `ALPHA = fix2 << 32 | 0x68` (`Cs·FIX + Cd`, FX 0x30, ST + scroll 1). The port draws the
//! FIX 0x7f passes as opaque world surfaces [L: 127/128 of the source, at most one level off] and the additive pass as a
//! display-blend effect with `As = FIX`.
//!
//! **The liquid meshes (G-REN-026)** (`rc_game::water::sea::MeshSet`; [`mesh_prims`] = the generic strip emitters level01
//! `0x21fda8` / `0x21fa98`): every mesh whose sphere passes `FastBSphereCheck(256, sphere)` (the particles' view test of
//! the camera as last drawn), one strip per stored strip, per pass: the ST rule (stored · scale, the lava scroll, the
//! sphere map `0x2667fc`, 1848's reflection map `0x30ef18` + scroll), the colour rule (one RGBA, the lava pattern, the
//! stored colours), the texture (an FX, an animated FX blend made like the grid's image, or the grid's own image and
//! fog). ALPHA `FIX << 32 | 0x64` with TEST 0x50000 (Z GEQUAL): FIX 0x80 = an opaque world surface, a lower FIX a
//! display-blend effect with `As = FIX` (the texture's alpha forced to 0x80, the vertex alpha = FIX) over it. Users:
//! level 9's lava flows (two passes, FIX 0x80 / 0x60) before its grid and its ten grid-textured meshes after it; 854's
//! seven gated grids (one shared image and fog, [`grid_prims_into`]); 293 (12) / 1418 (14) (FX a stored, FX b sphere
//! map at FIX 0x40); 1848 (FX 40 reflection at FIX 0x20, list 1).
//!
//! **Order.** Opaque surfaces go to the main opaque pass (their Z makes the order irrelevant). The effect draws of the
//! after-ties list go first among the effects (`TIES_BIAS`), the list-1 draws just before the other list-1 callbacks
//! (`SEA_LIST1_BIAS`). The display-blend pass runs after the world's own translucent draws, so a translucent moby part in
//! front of an after-ties effect draw is drawn before it instead of after (crate::display_blend).

use crate::display_blend::DisplayEffect;
use crate::fx_draw::{FxPrimMaterial, FxPrimParams, PrimBuf, LIST1_BIAS};
use crate::game_camera::{game_eye, GameFog, LevelFog, TfragFog};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rc_formats::sea::{GRID_BLOCK_CELLS, GRID_BLOCK_POINTS, GRID_STRIP_LEN, GRID_TEXTURE_SIDE};
use rc_game::moby_update::classes::draw_callbacks::Callback;
use rc_game::water::sea::{self as gs, GridAnim, SeaData, SeaKind};
use std::collections::HashMap;

/// `RC_SEA=0` turns the sea drawing off.
pub fn enabled() -> bool { !std::env::var("RC_SEA").is_ok_and(|v| v.trim() == "0") }

/// The effect draws of the after-ties list: before every other effect.
const TIES_BIAS: f32 = -2.0e6;
/// The list-1 sea draws: just before the other list-1 callbacks (crate::fx_draw).
const SEA_LIST1_BIAS: f32 = LIST1_BIAS - 1.0e3;

/// FX textures the ocean and the Hoven liquid use (`GetEffectTex` numbers in their callbacks).
const OCEAN_BAND_FX: usize = 0x16;
const HOVEN_G1_FX: usize = 0x2c;
const HOVEN_FX: [usize; 2] = [0x2f, 0x30];

/// A grid's animation frames as the module reads them: the stored 8-bit indices and the stored CLUT (1024 bytes).
#[derive(Clone, Debug, Default)]
pub struct GridFrames {
    pub frames: Vec<(Vec<u8>, Vec<u8>)>,
}

/// The level data the drawing needs beyond `rc_game::water::sea`: the raw frames of every animated liquid image (the
/// grids', 854's shared one, the lava flows'), by animation.
#[derive(Clone, Debug, Default)]
pub struct LevelSea {
    pub anim_frames: HashMap<GridAnim, GridFrames>,
}

/// The animations the level's sea ports draw with.
fn anims(d: &rc_game::water::world::LevelWaterData) -> Vec<(usize, GridAnim)> {
    let mut out = Vec::new();
    let sets = |port: usize, sets: &[gs::MeshSet], out: &mut Vec<(usize, GridAnim)>| {
        for p in sets.iter().flat_map(|s| &s.passes) {
            if let gs::MeshTex::Anim(a) = p.tex { out.push((port, a)); }
        }
    };
    for p in &d.sea {
        match &p.data {
            SeaData::Grid(g) => {
                out.push((p.port, gs::grid_anim(&g.grid)));
                sets(p.port, &g.extras, &mut out);
            }
            SeaData::GridSet(g) => out.push((p.port, g.anim)),
            SeaData::Meshes(m) => sets(p.port, std::slice::from_ref(&**m), &mut out),
            _ => {}
        }
    }
    out
}

/// Reads the raw FX frames of every liquid grid on the level (`water`: the level's water data).
pub fn load(core: &rc_formats::level::LevelCore, index: &[u8], core_data: &[u8], water: &crate::water_render::LevelWater) -> LevelSea {
    let mut out = LevelSea::default();
    let Some(d) = water.data.as_ref() else { return out };
    let grids = anims(d);
    if grids.is_empty() { return out; }
    let h = &core.header;
    let entries: Vec<rc_formats::particle_tex::FxTextureEntry> = match (h.fx_textures.count, h.fx_textures.offset) {
        (n, o) if n > 0 && o > 0 => rc_formats::buf::Buf(index).pod_slice(o as usize, n as usize, "fx_textures").unwrap_or_default(),
        _ => Vec::new(),
    };
    let Ok(bank) = rc_formats::particle_tex::core_bank(core, core_data, "fx_bank") else {
        eprintln!("sea: no FX bank: the liquid grids are not drawn");
        return out;
    };
    let side = GRID_TEXTURE_SIDE as i32;
    for (port, g) in grids {
        if out.anim_frames.contains_key(&g) { continue; }
        let frames: Option<Vec<(Vec<u8>, Vec<u8>)>> = (0..g.frames as usize)
            .map(|k| {
                let e = entries.get(g.tex as usize + k)?;
                if (e.width, e.height) != (side, side) { return None; }
                let img = rc_formats::particle_tex::bank_texture_image(bank, e.palette, e.texture, side, side).ok()?;
                Some((img.indices.to_vec(), img.clut.to_vec()))
            })
            .collect();
        match frames {
            Some(frames) => {
                out.anim_frames.insert(g, GridFrames { frames });
            }
            None => eprintln!("sea: {}: FX frames {}..+{} are not all 64×64: not drawn", gs::PORTS[port].name, g.tex, g.frames),
        }
    }
    out
}

/// The module's 64×64 image for tick `counter` (module doc), RGBA with the alpha at 0x80 (1.0; PSMCT24).
pub fn grid_image(frames: &GridFrames, period: u8, counter: u64, out: &mut Vec<u8>) {
    let n = frames.frames.len() as u64;
    out.clear();
    if n == 0 || period == 0 { return; }
    let p = period as u64;
    let q = counter / p;
    let t = counter as f32 / period as f32 - q as f32;
    let (a, b) = (&frames.frames[(q % n) as usize], &frames.frames[((q + 1) % n) as usize]);
    let s = 1.0 - t;
    for (&ia, &ib) in a.0.iter().zip(&b.0) {
        let (ca, cb) = (&a.1[4 * ia as usize..4 * ia as usize + 4], &b.1[4 * ib as usize..4 * ib as usize + 4]);
        for c in 0..3 { out.push((ca[c] as f32 * s + cb[c] as f32 * t) as u8); }
        out.push(0x80);
    }
}

/// `FastTweenColor(t, a, b)` 0x2221a8: per byte `trunc(a·(1 − t) + b·t)`.
pub fn tween(t: f32, a: u32, b: u32) -> u32 {
    let s = 1.0 - t;
    (0..4).fold(0u32, |acc, k| {
        let (x, y) = ((a >> (8 * k)) & 0xff, (b >> (8 * k)) & 0xff);
        acc | (((x as f32 * s + y as f32 * t) as u32 & 0xff) << (8 * k))
    })
}

/// Which texture a group samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Tex {
    /// FX texture n; `true`: its alpha forced to 0x80 (a FIX blend reads `As = FIX` through the vertex alpha).
    Fx(usize, bool),
    /// An animated liquid image (a grid's, 854's, the lava flows').
    Anim(GridAnim),
}

/// A display-blend effect's GS equation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Blend {
    /// ALPHA 0x44: `(Cs − Cd)·As + Cd`.
    Mix,
    /// ALPHA 0x48 (or 0x68 with As = FIX): `Cs·As + Cd`.
    Add,
    /// ALPHA 0x62 with As = FIX: `Cd − Cs·FIX`.
    Sub,
}

impl Blend {
    fn of(additive: bool) -> Blend { if additive { Blend::Add } else { Blend::Mix } }
}

/// One draw of this tick.
struct Group {
    tex: Tex,
    /// `None`: an opaque world surface; `Some(blend)`: a display-blend effect.
    effect: Option<Blend>,
    fog: TfragFog,
    bias: f32,
    prims: PrimBuf,
}

/// The ocean's pieces for one callback (module doc), `rows` = the camera's forward and left rows (game space).
pub fn ocean_groups(pv: &[u8], run: &gs::SeaRun, t: &rc_formats::sea::OceanTables, cam: [f32; 3], rows: [[f32; 3]; 2]) -> Vec<(usize, bool, PrimBuf)> {
    use gs::ocean_pvar as o;
    let w = |k: usize| u32::from_le_bytes(pv[4 * k..4 * k + 4].try_into().unwrap());
    let f = |off: usize| f32::from_le_bytes(pv[off..off + 4].try_into().unwrap());
    let mut out = Vec::new();
    let flat = |v: [f32; 3]| -> [f32; 3] {
        let l = (v[0] * v[0] + v[1] * v[1]).sqrt();
        if l == 0.0 { [0.0; 3] } else { [v[0] / l, v[1] / l, 0.0] }
    };
    let (fw, lf) = (flat(rows[0]), flat(rows[1]));
    let z = run.sea_z;
    let at = |a: f32, b: f32| [cam[0] + fw[0] * a + lf[0] * b, cam[1] + fw[1] * a + lf[1] * b, z];
    let reach = f(o::REACH);
    let (n0, n1, f0, f1) = (at(-75.0, -87.0), at(-75.0, 87.0), at(reach, -750.0), at(reach, 750.0));
    let lerp = |a: [f32; 3], b: [f32; 3], t: f32| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]), a[2] + t * (b[2] - a[2])];
    // 1. The far band.
    let mut band = PrimBuf::default();
    band.strip((0..6).flat_map(|i| {
        let t = (2 * i) as f32 / 10.0;
        let c = tween(t, w(0), w(1));
        [(lerp(n0, f0, t), [0.0, 0.0], c), (lerp(n1, f1, t), [0.0, 0.0], c)]
    }));
    out.push((OCEAN_BAND_FX, false, band));
    // 2. The wall.
    let top = cam[2] - f(o::WALL_DROP);
    if f(o::SEA_Z) < top {
        let mut wall = PrimBuf::default();
        wall.quad([f0, [f0[0], f0[1], top], f1, [f1[0], f1[1], top]], [[0.0; 2]; 4], [w(1), w(2), w(1), w(2)]);
        out.push((OCEAN_BAND_FX, false, wall));
    }
    // 3. The surface.
    let fade = (1.0 - (cam[2] - z) / f(o::FADE)).min(1.0);
    if fade >= 0.0 {
        let (c3, c4) = (tween(fade, 0, w(3)), tween(fade, 0, w(4)));
        let s = f(o::TILE);
        let base = pvar_i32(pv, o::FX);
        for layer in 0..2 {
            let mut b = PrimBuf::default();
            let (k, [su, sv]) = (t.scale[layer], run.scroll[layer]);
            let vert = |x: f32, y: f32| {
                let p = [cam[0] + x + 1.0, cam[1] + y + 1.0, z];
                let d = ((p[0] - cam[0]).powi(2) + (p[1] - cam[1]).powi(2) + (p[2] - cam[2]).powi(2)).sqrt() / (s + s);
                (p, [x / (s + s) * k + su, y / (s + s) * k + sv], tween(d.min(1.0), c3, c4))
            };
            let mut y = -2.0 * s;
            while y < s + s {
                let mut row = Vec::new();
                let mut x = -2.0 * s;
                while x < s + s {
                    row.extend([vert(x, y), vert(x, y + s), vert(x + s, y), vert(x + s, y + s)]);
                    x += s;
                }
                b.strip(row);
                y += s;
            }
            if let Ok(fx) = usize::try_from(base + layer as i32 + 0x28) { out.push((fx, true, b)); }
        }
    }
    out
}

fn pvar_i32(pv: &[u8], o: usize) -> i32 { i32::from_le_bytes(pv[o..o + 4].try_into().unwrap()) }

/// The grid's strips this tick (module doc): None when no block is near enough.
pub fn grid_prims(r: &rc_formats::sea::LiquidGrid, module: &rc_formats::sea::LiquidGridModule, scale: f32, z: f32, alpha: u8, cam: [f32; 3]) -> PrimBuf {
    let mut b = PrimBuf::default();
    grid_prims_into(&mut b, r, module, scale, z, alpha, cam);
    b
}

/// [`grid_prims`] appended to `b` (854 draws its seven records with one image and fog).
pub fn grid_prims_into(b: &mut PrimBuf, r: &rc_formats::sea::LiquidGrid, module: &rc_formats::sea::LiquidGridModule, scale: f32, z: f32, alpha: u8, cam: [f32; 3]) {
    let [bx, by] = r.blocks();
    let n = GRID_BLOCK_CELLS as f32;
    for j in 0..by {
        for i in 0..bx {
            let k = j * bx + i;
            if r.flags.get(k).copied().unwrap_or(0) == 0 { continue; }
            let c = [r.origin[0] + (i as f32 * n + n * 0.5) * r.cell[0], r.origin[1] + (j as f32 * n + n * 0.5) * r.cell[1], z];
            let d2 = (c[0] - cam[0]).powi(2) + (c[1] - cam[1]).powi(2) + (c[2] - cam[2]).powi(2);
            // The MAC sign of `d² − w`: drawn only when negative.
            if (d2 - r.cull_dist2).partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) { continue; }
            let cols = &r.colours[k];
            b.strip((0..GRID_STRIP_LEN).map(|v| {
                let o = module.order[v] as usize;
                let (col, row) = (o % GRID_BLOCK_POINTS, o / GRID_BLOCK_POINTS);
                let p = [r.origin[0] + (i * GRID_BLOCK_CELLS + col) as f32 * r.cell[0], r.origin[1] + (j * GRID_BLOCK_CELLS + row) as f32 * r.cell[1], z];
                let st = module.st[v];
                (p, [st[0] * scale, st[1] * scale], (cols[v] & 0x00ff_ffff) | (alpha as u32) << 24)
            }));
        }
    }
}

/// One pass of a liquid mesh set for this tick (`rc_game::water::sea::MeshSet`; the generic strip emitters level01
/// `0x21fda8` / `0x21fa98`): every mesh whose sphere passes `FastBSphereCheck(256, sphere)` (`view`; None: all), each
/// strip with the pass's ST and the set's colours, alpha `alpha` (0x80 for an opaque pass, the FIX for a blend).
pub fn mesh_prims(set: &gs::MeshSet, pass: &gs::MeshPass, counter: u64, cam: [f32; 3], view: Option<&rc_game::particles::BSphereView>, alpha: u8, scroll: [f32; 2]) -> PrimBuf {
    let mut b = PrimBuf::default();
    let f = gs::flow_offset(counter);
    for m in &set.meshes {
        if let (Some(v), Some(sp)) = (view, m.sphere) {
            if v.culled(set.far, sp) { continue; }
        }
        for (s, normals) in m.strips.iter().zip(&m.normals) {
            b.strip(s.pos.iter().zip(&s.st).enumerate().map(|(i, (p, st))| {
                let uv = match pass.st {
                    gs::MeshSt::Stored { scale } => [st[0] * scale, st[1] * scale],
                    gs::MeshSt::Flow { k, add } => [st[0], st[1] + (k * f + add)],
                    gs::MeshSt::SphereMap => gs::sphere_map_st(*p, normals.get(i).copied().unwrap_or_default(), cam),
                    gs::MeshSt::EnvMap => gs::env_map_st(*p, normals.get(i).copied().unwrap_or_default(), cam, scroll),
                };
                let c = match set.colour {
                    gs::MeshColour::Const(c) => c,
                    gs::MeshColour::Flow => gs::flow_colour(i),
                    gs::MeshColour::Stored => s.rgba.get(i).copied().unwrap_or(0x8080_8080),
                };
                (*p, uv, (c & 0x00ff_ffff) | (alpha as u32) << 24)
            }));
        }
    }
    b
}

/// The Hoven liquid's strips of one group with an ST scroll.
fn strip_prims(strips: &[rc_formats::sea::StripMesh], scroll: [f32; 2], alpha: Option<u8>) -> PrimBuf {
    let mut b = PrimBuf::default();
    for s in strips {
        b.strip(s.pos.iter().zip(&s.st).zip(&s.rgba).map(|((p, st), c)| {
            let c = match alpha { Some(a) => (c & 0x00ff_ffff) | (a as u32) << 24, None => *c };
            (*p, [st[0] + scroll[0], st[1] + scroll[1]], c)
        }));
    }
    b
}

/// The draw groups of one liquid mesh set (`rc_game::water::sea::MeshSet`), one per pass in order: a FIX 0x80 pass
/// (`(Cs − Cd)·1 + Cd = Cs`, Z written) as an opaque world surface, a lower FIX as a display-blend effect with
/// `As = FIX` (the texture's alpha forced to 0x80, the vertex alpha = FIX) on top of it (Z test GEQUAL: the same
/// strip passes). `grid` = the port's grid image, FIX and fog for a [`gs::MeshTex::Grid`] pass; `anim` makes an
/// animated image for this tick (false: its frames are missing, the pass is skipped).
#[allow(clippy::too_many_arguments)]
fn mesh_set_groups(groups: &mut Vec<Group>, set: &gs::MeshSet, counter: u64, cam: [f32; 3], view: &rc_game::particles::BSphereView, bias: f32, level_fog: TfragFog, grid: Option<(GridAnim, u8, TfragFog)>, scroll: [f32; 2], anim: &mut dyn FnMut(GridAnim) -> bool) {
    for (j, pass) in set.passes.iter().enumerate() {
        let (tex, fix, fog) = match pass.tex {
            gs::MeshTex::Fx(n) => (Tex::Fx(n as usize, pass.fix < 0x80), pass.fix, level_fog),
            gs::MeshTex::Anim(a) => {
                if !anim(a) { continue; }
                (Tex::Anim(a), pass.fix, level_fog)
            }
            gs::MeshTex::Grid => {
                let Some((a, fix, fog)) = grid else { continue };
                (Tex::Anim(a), fix, fog)
            }
        };
        let opaque = fix >= 0x80 || (matches!(pass.tex, gs::MeshTex::Grid) && fix >= 0x61);
        let prims = mesh_prims(set, pass, counter, cam, Some(view), if opaque { 0x80 } else { fix }, scroll);
        groups.push(Group { tex, effect: (!opaque).then_some(Blend::Mix), fog, bias: bias + j as f32 * 0.25, prims });
    }
}

// ---------------------------------------------------------------------------------------------------
// Bevy

type Slot = (Entity, Handle<Mesh>, Handle<FxPrimMaterial>, bool);

#[derive(Resource, Default)]
struct SeaDraw {
    slots: Vec<Option<Slot>>,
    fx: HashMap<(usize, bool), Handle<Image>>,
    anim_images: HashMap<GridAnim, Handle<Image>>,
    drawn: Option<u64>,
    pixels: Vec<u8>,
}

pub struct SeaRenderPlugin;

impl Plugin for SeaRenderPlugin {
    fn build(&self, app: &mut App) {
        if !enabled() { return; }
        app.init_resource::<SeaDraw>().add_systems(crate::level_switch::LevelUnload, crate::level_switch::reset::<SeaDraw>).add_systems(PostUpdate, draw.before(bevy::asset::AssetEventSystems));
    }
}

type MainCamera<'w, 's> = Query<'w, 's, &'static Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>;

fn repeat_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    })
}

fn fx_handle(st: &mut SeaDraw, images: &mut Assets<Image>, fx: Option<&[Option<rc_formats::texture::Texture>]>, i: usize, opaque_alpha: bool) -> Option<Handle<Image>> {
    if let Some(h) = st.fx.get(&(i, opaque_alpha)) { return Some(h.clone()); }
    let t = fx?.get(i)?.as_ref()?;
    let h = crate::fx_draw::fx_image(images, t);
    let h = if opaque_alpha {
        let mut img = images.get(&h)?.clone();
        if let Some(d) = img.data.as_mut() { for a in d.iter_mut().skip(3).step_by(4) { *a = 0x80; } }
        images.add(img)
    } else {
        h
    };
    st.fx.insert((i, opaque_alpha), h.clone());
    Some(h)
}

#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    state: ResMut<SeaDraw>,
    play: Option<Res<crate::gameplay::Play>>,
    cams: MainCamera,
    fog: Option<Res<GameFog>>,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FxPrimMaterial>>,
    mut vis: Query<&mut Visibility>,
    particles: Option<Res<crate::particle_render::ParticleSim>>,
) {
    let st = state.into_inner();
    let Some(p) = play.as_deref() else { return };
    let Some(cam_t) = cams.iter().next() else { return };
    let counter = p.game.counter;
    if st.drawn == Some(counter) { return; }
    st.drawn = Some(counter);
    let level_fog = fog.map(|f| f.uniform).unwrap_or_else(|| TfragFog::new(&level.0.fog));
    let missile = crate::visibomb_view::missile_view(play.as_deref());
    let cam = game_eye(cam_t).to_array();
    let to_game = |v: Vec3| [v.x, -v.z, v.y];
    let rows = [to_game(*cam_t.forward()), to_game(*cam_t.left())];
    let fx = level.0.particles.textures.as_ref().map(|t| t.fx_textures.as_slice());
    let water = &p.svc.water;
    let cbs = &p.svc.draw_callbacks;
    let view = crate::particle_render::bsphere_view(cam_t, particles.map_or_else(|| crate::particle_render::view_tans(None), |s| s.view_tan));
    let all_visible = crate::visibomb_view::all_visible(Some(p));
    let registered = cbs.ties.iter().map(|e| (true, e)).chain(cbs.list1.iter().map(|e| (false, e)));
    let mut groups: Vec<Group> = Vec::new();
    // The animated images of this tick, made once each.
    let mut made: Vec<GridAnim> = Vec::new();
    let mut anim = |a: GridAnim, st: &mut SeaDraw, images: &mut Assets<Image>| -> bool {
        if made.contains(&a) { return true; }
        let Some(frames) = level.0.sea.anim_frames.get(&a) else { return false };
        grid_image(frames, a.period, counter, &mut st.pixels);
        let h = st.anim_images.entry(a).or_insert_with(|| {
            let mut img = Image::new_uninit(
                Extent3d { width: GRID_TEXTURE_SIDE, height: GRID_TEXTURE_SIDE, depth_or_array_layers: 1 },
                TextureDimension::D2,
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            );
            img.sampler = repeat_sampler();
            images.add(img)
        });
        if let Some(mut img) = images.get_mut(&*h) { img.data = Some(st.pixels.clone()); }
        made.push(a);
        true
    };
    for (k, (ties, &(cb, id))) in registered.enumerate() {
        let Callback::Sea(port) = cb else { continue };
        let port = port as usize;
        let Some(m) = p.game.mobys.mobys.get(id) else { continue };
        let Some(data) = gs::data(water, port) else { continue };
        let run = &water.sea.run[port];
        let bias = if ties { TIES_BIAS } else { SEA_LIST1_BIAS } + (k * 4) as f32;
        match (gs::PORTS[port].kind, data) {
            (SeaKind::Grid(gp), SeaData::Grid(g)) => {
                if !gs::grid_draws(port, &m.pvars, &p.svc.volumes, cam) { continue; }
                // The animated image of this tick.
                let ga = gs::grid_anim(&g.grid);
                if !anim(ga, st, &mut images) { continue; }
                let r = &g.grid;
                let lf = LevelFog { color: r.fog_rgb, near_dist: r.fog[0], far_dist: r.fog[1], near_intensity: r.fog[2], far_intensity: r.fog[3] };
                // With 0x15f458 = 1 (the Visibomb's view) the set-up keeps the level fog and FOGCOL (`0x2c1978`).
                let grid_fog = if missile { level_fog } else { TfragFog::new(&lf) };
                let opaque = run.fix >= 0x61;
                let prims = grid_prims(&g.grid, &g.module, gp.scale, run.z, if opaque { 0x80 } else { run.fix }, cam);
                groups.push(Group { tex: Tex::Anim(ga), effect: (!opaque).then_some(Blend::Mix), fog: grid_fog, bias, prims });
                // Level 9's lava meshes (317's callback `0x2ef750`): the flows before the grid, the grid-textured ones
                // after it with its image, fog and FIX.
                for (j, set) in g.extras.iter().enumerate() {
                    let b = if set.before_grid { bias - 1.0 - j as f32 } else { bias + 1.0 + j as f32 };
                    mesh_set_groups(&mut groups, set, counter, cam, &view, b, level_fog, Some((ga, run.fix, grid_fog)), [0.0; 2], &mut |a| anim(a, st, &mut images));
                }
            }
            (SeaKind::Ocean, SeaData::Ocean(t)) => {
                if m.pvars.len() < 0x30 { continue; }
                for (j, (fxi, additive, prims)) in ocean_groups(&m.pvars, run, t, cam, rows).into_iter().enumerate() {
                    groups.push(Group { tex: Tex::Fx(fxi, false), effect: Some(Blend::of(additive)), fog: level_fog, bias: bias + j as f32 * 0.25, prims });
                }
            }
            (SeaKind::Hoven, SeaData::Hoven(h)) => {
                if gs::hoven_group1_drawn(&m.pvars, &p.svc.volumes, cam) {
                    groups.push(Group { tex: Tex::Fx(HOVEN_G1_FX, false), effect: None, fog: level_fog, bias, prims: strip_prims(&h.groups[0], run.g1_scroll, None) });
                }
                groups.push(Group { tex: Tex::Fx(HOVEN_FX[0], false), effect: None, fog: level_fog, bias: bias + 1.0, prims: strip_prims(&h.groups[1], run.scroll[0], None) });
                groups.push(Group { tex: Tex::Fx(HOVEN_FX[1], true), effect: Some(Blend::Add), fog: level_fog, bias: bias + 2.0, prims: strip_prims(&h.groups[1], run.scroll[1], Some(h.fix2)) });
            }
            (SeaKind::GridSet, SeaData::GridSet(g)) => {
                // 854's callback `0x2ea048`: the shared image and fog (`0x2a4818`), the gated records, the fog restore.
                if !anim(g.anim, st, &mut images) { continue; }
                let opaque = g.fix >= 0x61;
                let mut prims = PrimBuf::default();
                for (r, gate) in &g.grids {
                    if !gs::grid_set_drawn(*gate, &m.pvars, &p.svc.volumes, cam, all_visible) { continue; }
                    grid_prims_into(&mut prims, r, &g.module, gs::aridia_ref::SCALE, r.origin[2], if opaque { 0x80 } else { g.fix }, cam);
                }
                let lf = LevelFog { color: g.fog_rgb, near_dist: g.fog[0], far_dist: g.fog[1], near_intensity: g.fog[2], far_intensity: g.fog[3] };
                // (`0x2a4818` has no Visibomb-view branch: the record's fog always.)
                groups.push(Group { tex: Tex::Anim(g.anim), effect: (!opaque).then_some(Blend::Mix), fog: TfragFog::new(&lf), bias, prims });
            }
            (SeaKind::Pool(pm), SeaData::Pool(d)) => {
                // 1903 / 1919's callbacks `0x31db50` / `0x31e930` (rc_game::water::sea::pool_ref): L0 (with the shimmer's
                // two FIX passes while its alpha is up) and L2 behind B's camera gate, L1's two scrolls always.
                let (pr, tex) = (&run.pool, pm.fx.map(|t| t as usize));
                let full = gs::pool_drawn(pm, &m.pvars, &p.svc.volumes, cam);
                let mut passes: Vec<(Tex, Blend, PrimBuf)> = Vec::new();
                if full {
                    // L0: `ALPHA 0x44`, or (1649) `FIX << 32 | 0x64` drawn as the mix with As = FIX.
                    passes.push((Tex::Fx(tex[0], d.l0_fix.is_some()), Blend::Mix, strip_prims(&d.layers[0], pr.l0[0], d.l0_fix)));
                    if pr.alpha != 0 {
                        for k in 0..2 {
                            let fix = gs::pool_fix(d.fix[k], pr.alpha);
                            let blend = if pm.shimmer[k].0 { Blend::Add } else { Blend::Sub };
                            passes.push((Tex::Fx(tex[1], true), blend, strip_prims(&d.layers[0], pr.l0[1 + k], Some(fix))));
                        }
                    }
                }
                for k in 0..2 { passes.push((Tex::Fx(tex[2], false), Blend::Add, strip_prims(&d.layers[1], pr.l1[k], None))); }
                if full { passes.push((Tex::Fx(tex[3], false), Blend::Add, strip_prims(&d.layers[2], pr.l2, None))); }
                for (j, (t, b, prims)) in passes.into_iter().enumerate() {
                    groups.push(Group { tex: t, effect: Some(b), fog: level_fog, bias: bias + j as f32 * 0.25, prims });
                }
            }
            (SeaKind::TwoTex(_) | SeaKind::EnvOverlay | SeaKind::Reflect(_), SeaData::Meshes(set)) => {
                mesh_set_groups(&mut groups, set, counter, cam, &view, bias, level_fog, None, run.scroll[0], &mut |a| anim(a, st, &mut images));
            }
            _ => {}
        }
    }
    // Show the groups in slots 0.., hide the rest.
    let n = groups.len();
    for (k, g) in groups.into_iter().enumerate() {
        let img = match g.tex {
            Tex::Fx(i, a) => fx_handle(st, &mut images, fx, i, a),
            Tex::Anim(a) => st.anim_images.get(&a).cloned(),
        };
        if st.slots.len() <= k { st.slots.push(None); }
        let (Some(img), false) = (img, g.prims.is_empty()) else {
            hide(&mut vis, &mut st.slots[k]);
            continue;
        };
        let params = match g.effect { None => FxPrimParams::opaque(), Some(Blend::Sub) => FxPrimParams::subtract(), Some(b) => FxPrimParams::blend(b == Blend::Add) };
        let effect = g.effect.is_some();
        if let Some((e, mesh, mat, shown)) = &mut st.slots[k] {
            g.prims.update(&mut meshes, mesh);
            let need = materials.get(&*mat).is_none_or(|m| m.texture != img || m.params.misc != params.misc || m.fog != g.fog || m.order != g.bias);
            if need {
                if let Some(mut m) = materials.get_mut(&*mat) {
                    m.texture = img;
                    m.params = params;
                    m.fog = g.fog;
                    m.order = g.bias;
                }
            }
            let mut ec = commands.entity(*e);
            if effect { ec.insert(DisplayEffect); } else { ec.remove::<DisplayEffect>(); }
            if !*shown {
                *shown = true;
                if let Ok(mut v) = vis.get_mut(*e) { *v = Visibility::Inherited; }
            }
            continue;
        }
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        g.prims.write(&mut mesh);
        let mesh = meshes.add(mesh);
        let mat = materials.add(FxPrimMaterial { texture: img, fog: g.fog, params, order: g.bias });
        let mut ec = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), Transform::IDENTITY, NoFrustumCulling, Visibility::Inherited, Name::new(format!("sea draw {k}"))));
        if effect { ec.insert(DisplayEffect); }
        st.slots[k] = Some((ec.id(), mesh, mat, true));
    }
    for s in st.slots.iter_mut().skip(n) { hide(&mut vis, s); }
}

fn hide(vis: &mut Query<&mut Visibility>, s: &mut Option<Slot>) {
    let Some((e, _, _, shown)) = s else { return };
    if !*shown { return; }
    *shown = false;
    if let Ok(mut v) = vis.get_mut(*e) { *v = Visibility::Hidden; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_is_per_byte_and_truncates() {
        assert_eq!(tween(0.0, 0x1020_3040, 0xffff_ffff), 0x1020_3040);
        assert_eq!(tween(1.0, 0x1020_3040, 0xffff_ffff), 0xffff_ffff);
        // 0x40·0.5 + 0x41·0.5 = 64.5 → 64.
        assert_eq!(tween(0.5, 0x40, 0x41) & 0xff, 0x40);
    }

    #[test]
    fn grid_image_blends_two_frames_by_the_tick() {
        let clut = |c: [u8; 4]| { let mut v = vec![0u8; 1024]; v[..4].copy_from_slice(&c); v };
        let f = GridFrames { frames: vec![(vec![0; 4], clut([0, 0, 0, 0])), (vec![0; 4], clut([100, 200, 40, 0]))] };
        let mut px = Vec::new();
        // period 10, tick 5: frames 0 → 1 at t = 0.5.
        grid_image(&f, 10, 5, &mut px);
        assert_eq!(&px[..4], &[50, 100, 20, 0x80]);
        // tick 15: frames 1 → 0 at 0.5.
        grid_image(&f, 10, 15, &mut px);
        assert_eq!(&px[..4], &[50, 100, 20, 0x80]);
    }
}
