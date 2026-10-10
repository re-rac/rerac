//! Tfrag (terrain) meshes and their material.
//!
//! LOD selection, morphing and collapse as the game does them (docs/formats/tfrag_rac1.md §3b): every
//! triangle of all three strip lists (LOD 0, 1, 2) is in the meshes, batched per GS texture state
//! (texture index, CLAMP_1 wrap flags, TEX1 K). A vertex carries only its own position (for bounds) and a
//! reference (global vertex-info index, tfrag index | list << 16). The vertex shader reads that tfrag's draw
//! mode for this frame (`tfrag_lod::update_tfrag_modes`, a storage buffer of one u32 per tfrag), drops the
//! vertex if the mode does not draw its list, and otherwise replays the VU1 morph / collapse passes of that
//! mode from two static storage buffers:
//! * `slots`: one entry per (tfrag, position): world position, lit RGBA, and for positions owned by a
//!   morphing primary vertex-info entry its tier and the two parent positions;
//! * `vinfos`: one entry per (tfrag, vertex-info entry): UV, position, tier, and the collapse links
//!   (parent-1 entry = `parent_indices` / `unk_indices_2`, parent-2 position = `parent / 2`).
//!
//! No directional lighting here (lighting: tfrag_light.rs; fog and projection: game_camera.rs): each vertex gets its
//! position slot's RGBA, modulated with its texture exactly as the GS does with TFX = MODULATE. The point lights of
//! `LightTfrags` are added to a slot's RGBA in the vertex shader when the tfrag's nibble list names any
//! (crate::world_lights): the slot's normal is packed in its `pad` word, the lists and the bank follow the draw
//! modes in the `modes` buffer (crate::tfrag_lod). Texture
//! sampling uses the full mip chain and the GS mip rule (LOD = log2(1/Q) + K, nearest level, bilinear).

use crate::gs_state::{self, AlphaRange, GsPass};
use crate::level_load::LoadedLevel;
use crate::tfrag_lod::{TfragLodState, TfragLodUniform};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::tfrag::{tfrag_triangles, TfragAdGifs, TfragMorphTier};
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

/// Game → Bevy coordinates, used by every renderer in this crate.
///
/// The game is right-handed Z-up (docs/formats/tfrag_rac1.md §6); Bevy is right-handed Y-up.
/// `(x, y, z)_game → (x, z, -y)_bevy` is a -90° rotation about X (determinant +1, not a mirror),
/// so triangle winding is preserved and game "north" (+Y) becomes Bevy forward (-Z).
/// Units stay game units (positions already divided by 1024).
pub fn game_to_bevy(p: [f32; 3]) -> Vec3 { Vec3::new(p[0], p[2], -p[1]) }

const SHADER_PATH: &str = "shaders/tfrag.wgsl";

/// Per vertex: (global vertex-info index, tfrag index | strip list << 16 | TEX1 K (s12, 1/16) << 18).
pub const ATTRIBUTE_TFRAG_REF: MeshVertexAttribute = MeshVertexAttribute::new("TfragRef", 0x5446_5246, VertexFormat::Uint32x2);

/// Tfrag material: texture × vertex colour, GS MODULATE semantics, no lighting; LOD/morph data in storage buffers.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GsPass)]
pub struct TfragMaterial {
    /// The texture with its full mip chain (`TextureEntry::ty` levels).
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    /// Level fog as the VU/GS apply it (crate::game_camera::TfragFog).
    #[storage(2, read_only)]
    /// The shared fog buffer (crate::game_camera::fog_buffer).
    pub fog: Handle<bevy::render::storage::ShaderBuffer>,
    /// VU qw666..669, the w slope, and this batch's TEX1 K / MXL.
    #[uniform(3)]
    pub lod: TfragLodUniform,
    /// Static per-position data (see the module doc).
    #[storage(4, read_only, visibility(vertex))]
    pub slots: Handle<ShaderBuffer>,
    /// Static per-vertex-info data.
    #[storage(5, read_only, visibility(vertex))]
    pub vinfos: Handle<ShaderBuffer>,
    /// Per-frame draw mode per tfrag (`TfragLodState::modes`).
    #[storage(6, read_only, visibility(vertex))]
    pub modes: Handle<ShaderBuffer>,
    /// This draw's GS state (`TfragMaterial::alpha_mode`).
    pub pass: GsPass,
}

impl From<&TfragMaterial> for GsPass {
    fn from(m: &TfragMaterial) -> Self { m.pass }
}

impl Material for TfragMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }

    /// GS state of the tfrag pass (boot ELF): `reset_gs_registers` (0x1f3868) sends the 19-register A+D
    /// table at 0x13cfc0, which holds ALPHA_1 = 0x8000000044, TEST_1 = 0x5360b, PABE = 0, FBA_1 = 0,
    /// COLCLAMP = 1, DTHE = 0 (no writer of those table entries exists in the boot decomp). The sky drawn next
    /// (`transition_draw_sky`, 0x1e9ab8) sets ALPHA_1 = 0x8000000044 again (`update_sky_effects`) and restores
    /// TEST_1 = 0x5360b and an unmasked ZBUF; the tfrag chain itself only sends PRIM 0x7c (ABE = 1) and the
    /// TEX0/TEX1/CLAMP/MIPTBP1/MIPTBP2 ad-gifs. So every tfrag pixel is `((Cs − Cd)·As >> 7) + Cd` with
    /// As = (At·Af) >> 7, then the alpha test As >= 0x60 decides only the Z write (AFAIL = RGB_ONLY).
    ///
    /// On Novalis every drawn vertex has Af = 0x80 (lighting keeps the base alpha, tfrag_lighting.md §4) and 85
    /// of the 86 tfrag textures are all At = 0x80, so As = 0x80 and the blend is exactly Cs: one
    /// `GsPass::Opaque` draw. Texture 35 (the 128×128 grass overhang) has 7166 texels At = 0, 443 texels
    /// 0 < At < 0x60, 176 texels 0x60 <= At < 0x80: its batch is drawn twice (`gs_state::draws`), the
    /// fragments with As >= 0x60 blended with Z write, the rest blended without.
    ///
    /// Known differences (only the non-0x80 fragments are affected; gs_state.rs has the draw-order ones):
    /// - the GPU blends in linear light (sRGB target), the GS blends the display-encoded bytes, so fractional
    ///   As gives slightly different mixes (As = 0 and 0x80 are exact);
    /// - As > 0x80 would over-weight Cs on the GS; the unorm target clamps the blend factor to 1 (no texel
    ///   or vertex of Novalis exceeds 0x80).
    fn alpha_mode(&self) -> AlphaMode { self.pass.alpha_mode() }

    /// The GS has no back-face culling and the tfrag VU1 program emits every strip triangle
    /// regardless of facing (only the guard-band ADC drop, vu1_tfrag_analysis.md §2), so draw both faces.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        key.bind_group_data.specialize(descriptor);
        // Own position (bounds only) and the (vertex-info, tfrag | list) reference; everything else is in storage.
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_TFRAG_REF.at_shader_location(1),
        ])?];
        Ok(())
    }
}

/// The texture wrap state a triangle needs, from the ad-gif's CLAMP_1 quadword.
///
/// As the level-load init assembles it (tfrag_rac1.md §2.5.1, `TfragAdGifs::{wrap_s, wrap_t}`): on-disc
/// `clamp.data_lo` is WMS, `clamp.data_hi` is WMT, 0 = REPEAT, 1 = CLAMP (the only values on the disc).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct SamplerKey {
    clamp_s: bool,
    clamp_t: bool,
}

impl SamplerKey {
    fn of(ad: &TfragAdGifs) -> Self { SamplerKey { clamp_s: ad.clamp.data_lo & 1 != 0, clamp_t: ad.clamp.data_hi & 1 != 0 } }

    /// TEX1_1: MMAG = LINEAR, MMIN = LINEAR_MIPMAP_NEAREST (disc value 4 everywhere): bilinear inside one mip
    /// level. The level itself is chosen in the fragment shader with the GS rule (`textureSampleLevel`), so the
    /// hardware's derivative-based LOD is never used; `mipmap_filter: Nearest` keeps a whole-number level exact.
    fn descriptor(self) -> ImageSamplerDescriptor {
        let wrap = |clamp: bool| if clamp { ImageAddressMode::ClampToEdge } else { ImageAddressMode::Repeat };
        ImageSamplerDescriptor {
            address_mode_u: wrap(self.clamp_s),
            address_mode_v: wrap(self.clamp_t),
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            // Linear between levels and anisotropic: the shader picks the level (an integer one is read alone, the GS's
            // LINEAR_MIPMAP_NEAREST); the texture option (crate::graphics) blends or lets the GPU choose.
            mipmap_filter: ImageFilterMode::Linear,
            anisotropy_clamp: 16,
            ..default()
        }
    }
}


/// Everything accumulated for one draw batch.
#[derive(Default)]
struct Batch {
    positions: Vec<[f32; 3]>,
    refs: Vec<[u32; 2]>,
    indices: Vec<u32>,
    /// Range of the slot-colour alphas (Af) of the tfrags drawn here (None before the first triangle).
    vertex_alpha: Option<AlphaRange>,
    /// (tfrag, list, vertex-info index, K) → vertex, so shared strip vertices stay shared.
    remap: HashMap<(u32, u8, u16, u32), u32>,
}

pub struct TfragSceneStats {
    pub tfrags: usize,
    /// LOD-0 triangles (the LOD-1 / LOD-2 lists are in the meshes too).
    pub triangles: usize,
    pub vertices: usize,
    pub meshes: usize,
    /// Meshes some of whose fragments can have As != 0x80 (drawn with the GS alpha-test split, gs_state.rs).
    pub blended_meshes: usize,
    pub images: usize,
    pub mesh_build: Duration,
    pub upload: Duration,
    /// Game-space bounding box of LOD-0 vertices.
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Little-endian byte writer for the storage buffers (layouts mirror the WGSL structs `Slot` and `VInfo`).
#[derive(Default)]
struct Bytes(Vec<u8>);

impl Bytes {
    fn f(&mut self, v: f32) -> &mut Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    fn u(&mut self, v: u32) -> &mut Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
}

const TIER_NONE: u32 = 0;
const TIER_LOD01: u32 = 1;
const TIER_LOD0: u32 = 2;

fn tier_code(t: TfragMorphTier) -> u32 { match t { TfragMorphTier::Lod01 => TIER_LOD01, TfragMorphTier::Lod0 => TIER_LOD0 } }

/// Builds the static storage buffers `slots` (32 bytes per position) and `vinfos` (24 bytes per vertex-info entry)
/// over all tfrags, and returns them with each tfrag's first slot / vertex-info index.
///
/// `Slot { pos: vec3<f32>, color: u32, p1: u32, p2: u32, tier: u32, normal: u32 }` (`normal` = the `LightTfrags`
/// record's azimuth | elevation << 8 | 1 << 16, 0 when the position has no record; for the point lights): a position written by a
/// morphing primary entry (VU1 L17/L18/L25/L26 store the morphed value into the primary's own slot) gets that
/// entry's tier and parent positions. On the disc every primary owns a distinct position of its own tier
/// (a permutation of the tier's positions in 6 LOD-01 cases).
/// `VInfo { uv: vec2<f32>, slot: u32, tier: u32, p1: u32, p2: u32 }`: `tier` marks LOD-01 / LOD-0 entries (primary
/// or extra), which collapse onto `p1` (a vertex-info index) when both parents are past the threshold.
fn build_storage(level: &LoadedLevel) -> (Vec<u8>, Vec<u8>, Vec<u32>, Vec<u32>, usize) {
    let (mut slots, mut vinfos) = (Bytes::default(), Bytes::default());
    let (mut slot_base, mut vinfo_base) = (Vec::new(), Vec::new());
    let (mut ns, mut nv) = (0u32, 0u32);
    let mut missing_rgba = 0usize;
    for t in &level.tfrags {
        slot_base.push(ns);
        vinfo_base.push(nv);
        let lit = (t.header.vert_count as usize).min(t.lights.len());
        let mut owner = vec![(TIER_NONE, 0u32, 0u32); t.positions.len()];
        for v in 0..t.vertex_info.len() {
            if let Some(l) = t.lod_link(v).filter(|l| l.morphs) {
                owner[l.own_position] = (tier_code(l.tier), ns + l.parent1_position as u32, ns + l.parent2_position as u32);
            }
        }
        for (p, pos) in t.positions.iter().enumerate() {
            let w = [
                t.origin[0].wrapping_add(pos.x as i32) as f32 / 1024.0,
                t.origin[1].wrapping_add(pos.y as i32) as f32 / 1024.0,
                t.origin[2].wrapping_add(pos.z as i32) as f32 / 1024.0,
            ];
            let b = game_to_bevy(w);
            // GS MODULATE: Cv = Ct * Cf >> 7, so 0x80 = 1.0; the shader divides by 128.
            let c = t.rgba.get(p).map(|c| [c.r, c.g, c.b, c.a]).unwrap_or_else(|| { missing_rgba += 1; [0x80; 4] });
            let (tier, p1, p2) = owner[p];
            // `LightTfrags` pairs light record i with position i (tfrag_lighting.md §4).
            let normal = if p < lit {
                let l = rc_formats::tfrag_light::VertexLight::from_record(&t.lights[p]);
                l.azimuth as u32 | (l.elevation as u32) << 8 | 1 << 16
            } else { 0 };
            slots.f(b.x).f(b.y).f(b.z).u(u32::from_le_bytes(c)).u(p1).u(p2).u(tier).u(normal);
        }
        for (v, e) in t.vertex_info.iter().enumerate() {
            // GS ST with Q = 1: S and T are normalised texture coordinates. The VU emits (s_float - 2048,
            // t_float - 2048) where s_float = 2048 + s/4096 (STROW 0x45000000, tfrag_rac1.md §2.2).
            vinfos.f(e.s as f32 / 4096.0).f(e.t as f32 / 4096.0).u(ns + t.position_index(v) as u32);
            match t.lod_link(v) {
                Some(l) => vinfos.u(tier_code(l.tier)).u(nv + l.parent1_vinfo as u32).u(ns + l.parent2_position as u32),
                None => vinfos.u(TIER_NONE).u(0).u(0),
            };
        }
        ns += t.positions.len() as u32;
        nv += t.vertex_info.len() as u32;
    }
    (slots.0, vinfos.0, slot_base, vinfo_base, missing_rgba)
}

/// Builds the batched tfrag meshes (all three strip lists), the storage buffers and the LOD state, and spawns them.
pub fn spawn_tfrags(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<TfragMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> TfragSceneStats {
    spawn_tfrags_on(commands, level, meshes, images, materials, buffers, None)
}

/// [`spawn_tfrags`] with the entities on render layer `layer` (None: the default layer; crate::title_world draws the
/// title world on its own layer).
pub fn spawn_tfrags_on(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<TfragMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
    layer: Option<&bevy::camera::visibility::RenderLayers>,
) -> TfragSceneStats {
    let t0 = Instant::now();
    let (slot_bytes, vinfo_bytes, _slot_base, vinfo_base, missing_rgba) = build_storage(level);
    if missing_rgba > 0 { warn!("{missing_rgba} tfrag positions had no RGBA entry; used 0x80 grey"); }
    // Batch key: texture index and wrap state. TEX1 K differs between ad-gifs of one texture (49 values on the
    // disc), so it travels per vertex (bits 18..29 of the reference) instead of splitting batches.
    let mut batches: BTreeMap<(i32, SamplerKey), Batch> = BTreeMap::new();
    let mut n_tris = [0usize; 3];
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for (ti, t) in level.tfrags.iter().enumerate() {
        let alpha = AlphaRange::of(t.rgba.iter().take(t.positions.len()).map(|c| c.a));
        for (list, n_list) in n_tris.iter_mut().enumerate() {
            let tris = tfrag_triangles(t, list).expect("tfrag strips were validated by the golden test");
            *n_list += tris.len();
            for tri in &tris {
                let ad = &t.ad_gifs[tri.ad_gif as usize];
                let key = (t.texture_index(tri.ad_gif as usize), SamplerKey::of(ad));
                let k12 = ad.lod_k_raw() as u32 & 0xfff;
                let b = batches.entry(key).or_default();
                b.vertex_alpha = Some(b.vertex_alpha.map_or(alpha, |r| r.union(alpha)));
                for vi in [tri.a, tri.b, tri.c] {
                    let v = if let Some(&v) = b.remap.get(&(ti as u32, list as u8, vi, k12)) { v } else {
                        let p = t.world_position(vi as usize);
                        if list == 0 { for k in 0..3 { min[k] = min[k].min(p[k]); max[k] = max[k].max(p[k]); } }
                        b.positions.push(game_to_bevy(p).to_array());
                        b.refs.push([vinfo_base[ti] + vi as u32, ti as u32 | (list as u32) << 16 | k12 << 18]);
                        let v = (b.positions.len() - 1) as u32;
                        b.remap.insert((ti as u32, list as u8, vi, k12), v);
                        v
                    };
                    b.indices.push(v);
                }
            }
        }
    }
    info!("tfrag triangles per strip list (LOD 0 / 1 / 2): {n_tris:?}");
    let mesh_build = t0.elapsed();

    let t0 = Instant::now();
    let slots = buffers.add(ShaderBuffer::new(&slot_bytes, RenderAssetUsages::RENDER_WORLD));
    let vinfos = buffers.add(ShaderBuffer::new(&vinfo_bytes, RenderAssetUsages::RENDER_WORLD));
    let state = TfragLodState::new(level, buffers);
    let modes = state.modes.clone();
    commands.insert_resource(state);
    let lod_base = TfragLodUniform::new(&level.tfrag_lod.block, &level.fog);
    info!(
        "tfrag LOD: L = {}, D = {:?}, VU qw666 {:?} qw667 {:?} qw668 {:?} qw669 {:?}",
        level.tfrag_lod.block.unknown_8, level.tfrag_lod.block.lod_distances(), lod_base.k666, lod_base.k667, lod_base.k668, lod_base.k669
    );

    let fallback = images.add(Image::new_fill(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255, 0, 255, 0x80], // alpha is the raw GS At (0x80 = opaque)
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    ));
    // Image, texel alpha range (all levels) and MXL per (texture, sampler state).
    let mut image_cache: HashMap<(i32, SamplerKey), (Handle<Image>, AlphaRange, u32)> = HashMap::new();
    let mut n_blend = 0usize;
    let mut n_vertices = 0;
    let n_meshes = batches.len();
    for (bi, ((tex_index, skey), b)) in batches.into_iter().enumerate() {
        let (image, texel_alpha, mxl) = image_cache
            .entry((tex_index, skey))
            .or_insert_with(|| match usize::try_from(tex_index).ok().and_then(|i| level.tfrag_lod.mips.get(i)).and_then(|m| m.as_ref()) {
                Some(levels) => {
                    // Rgba8Unorm, not *Srgb: the GS blends and filters the raw 8-bit values, which are
                    // display-encoded. The shader modulates in that space and converts to linear once at
                    // the end, so the sRGB view target stores exactly the value the GS would.
                    // Alpha back to the raw GS texel alpha (0..0x80) for the As = (At·Af) >> 7 in the shader:
                    // `texture::scale_alpha` maps a < 0x80 to 2a and a >= 0x80 to 0xff, so this is exact
                    // for a <= 0x80 (every Novalis tfrag texel; nothing above 0x80 is recoverable).
                    // All levels share the entry's CLUT (texture::decode_tfrag_mip_levels), level 0 first.
                    let mut rgba = Vec::with_capacity(levels.iter().map(|l| l.rgba.len()).sum());
                    for l in levels { rgba.extend_from_slice(&l.rgba); }
                    for a in rgba.iter_mut().skip(3).step_by(4) {
                        *a = if *a == 0xff { 0x80 } else { *a / 2 };
                    }
                    let range = AlphaRange::of(rgba.iter().skip(3).step_by(4).copied());
                    let mut img = Image::new_uninit(
                        Extent3d { width: levels[0].width, height: levels[0].height, depth_or_array_layers: 1 },
                        TextureDimension::D2,
                        TextureFormat::Rgba8Unorm,
                        RenderAssetUsages::RENDER_WORLD,
                    );
                    img.texture_descriptor.mip_level_count = levels.len() as u32;
                    img.data = Some(rgba);
                    img.sampler = ImageSampler::Descriptor(skey.descriptor());
                    (images.add(img), range, levels.len() as u32 - 1)
                }
                None => {
                    warn!("tfrag texture index {tex_index} has no decoded texture; drawing magenta");
                    (fallback.clone(), AlphaRange::OPAQUE, 0)
                }
            })
            .clone();
        n_vertices += b.positions.len();
        let vertex_alpha = b.vertex_alpha.unwrap_or(AlphaRange::OPAQUE);
        let passes = gs_state::draws(gs_state::AREF_WORLD, texel_alpha, vertex_alpha);
        if passes != [GsPass::Opaque] {
            n_blend += 1;
            info!("tfrag tex {tex_index} {skey:?}: texel alpha {texel_alpha:?}, vertex alpha {vertex_alpha:?}: draws {passes:?}");
        }
        let mut lod = lod_base;
        // MXL = ty - 1: GS LOD = log2(1/Q) + K, level = nearest, clamped to 0..MXL (K comes per vertex).
        lod.tex = Vec4::new(0.0, mxl as f32, 0.0, 0.0);
        let mesh = meshes.add(
            Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.positions)
                .with_inserted_attribute(ATTRIBUTE_TFRAG_REF, VertexAttributeValues::Uint32x2(b.refs))
                .with_inserted_indices(Indices::U32(b.indices)),
        );
        for pass in passes {
            let mut e = commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(materials.add(TfragMaterial {
                    texture: image.clone(),
                    fog: crate::game_camera::fog_buffer(),
                    lod,
                    slots: slots.clone(),
                    vinfos: vinfos.clone(),
                    modes: modes.clone(),
                    pass,
                })),
                // Morphing moves vertices toward parents that can lie outside this batch's bounds.
                NoFrustumCulling,
                crate::pre_shadow::WorldDrawOrder([crate::pre_shadow::WorldDrawOrder::TFRAG, bi as u32, 0, 0, 0]),
                Name::new(format!("tfrag tex {tex_index} {pass:?}")),
            ));
            if let Some(l) = layer { e.insert(l.clone()); }
        }
    }

    TfragSceneStats {
        tfrags: level.tfrags.len(),
        triangles: n_tris[0],
        vertices: n_vertices,
        meshes: n_meshes,
        blended_meshes: n_blend,
        images: image_cache.len(),
        mesh_build,
        upload: t0.elapsed(),
        min,
        max,
    }
}
