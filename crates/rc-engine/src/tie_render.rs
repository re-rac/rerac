//! Tie (instanced static prop) renderer: every gameplay tie instance, drawn with the class LOD `TieProc`
//! picks for it this frame (crate::tie_lod), its own 4×4 matrix, the class textures (full mip chain, GS mip rule) and the 64-slot colour
//! table the game's `LightTies` pass computes for it (crate::tie_light, docs/plan/tie_lighting.md).
//!
//! Geometry: per class, one mesh per (tie texture, CLAMP wrap state) holding the triangles of all three
//! LODs (`tie::tie_triangles`), in class space (`position × scale / 1024`, game axes). Each vertex carries
//! its UV, its morph delta (same scale; 0 for dinky vertices), one u32 with the light slot (bits 0..5)
//! and the ad-gif's TEX1 K (s12 in 1/16, bits 20..31, as the tfrag renderer does), and one u32 with the
//! fat vertex's two morph colour slots, a fat flag and its LOD (`ATTRIBUTE_TIE_MORPH`). The vertex shader
//! drops the vertices of the LODs the instance does not use this frame, so the draw count is the same as
//! with one LOD.
//!
//! Instances: one entity per (instance, class part) with `MeshTag(instance index)`, so Bevy batches
//! every instance of a part into one instanced draw. The vertex shader reads the instance's record
//! from one storage buffer: the class→Bevy-world matrix (`game_to_bevy` applied after the game's
//! column-major instance matrix, `[3][3]` = 1; it can hold scale, shear and mirroring, so the shader
//! uses it instead of the entity transform), the bounding-sphere centre and radius, the draw distance,
//! and the 64 lit colours (the baked `LightTies` result; the shader adds the point lights of the instance's
//! nibble list, crate::world_lights, from the class normals the record also carries; the lists and the bank follow
//! the per-frame LOD words, crate::tie_lod). The entity `Transform` is only for Bevy's frustum culling
//! (`Transform::from_matrix`; instances it cannot represent exactly, i.e. sheared ones, get
//! `NoFrustumCulling`).
//!
//! Per-instance rules from `TieProc` (boot 0x235be8, level01 0x2a9a90), evaluated on the CPU each frame
//! by `tie_lod::update_tie_lods` into a second storage buffer (binding 5), which also hides the part
//! entities of instances it does not draw (see tie_lod.rs, "CPU culling"):
//! * cull: the view depth of the bounding-sphere centre exceeds `min(draw_distance, 720.0)` (instance
//!   +0x04 is an **s32**, converted to float at load; 720.0 = 0x44340000 is the global cap set by the
//!   level loader; 0 = never drawn), the sphere is wholly in front of the near plane or outside a side plane;
//! * LOD and morph factor k from the centre depth and the class `near/mid/far_dist` (tie_lod.rs);
//! * fog: one F per instance (qw 5.w) from the **centre** depth (clamped at 0) with the level fog line;
//! * before all of these, the occlusion word (+0x18/+0x19) against this frame's mask (crate::occlusion).
//!
//! GS state and MODULATE / alpha as for tfrags (`TfragMaterial::alpha_mode`): TEST_1 = 0x5360b is inherited
//! by the main tie list, so a part whose As can differ from 0x80 is drawn twice with the AREF 0x60 split
//! (crate::gs_state). The second (clipping) list's TEST_1 = 0x5340b (AREF 0x40) is not modelled.

use crate::gs_state::{self, AlphaRange, GsPass};
use crate::level_load::LoadedLevel;
use crate::pre_shadow::WorldDrawOrder;
use crate::tfrag_render::game_to_bevy;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshTag, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::level::LevelCore;
use rc_formats::texture::{self, LevelTexture, Texture, TextureTable};
use rc_formats::tie::{self, LevelTieClass, TieAdGifs, TieInstance};
use rc_formats::tie_light::{instance_centre, SLOTS};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::{Duration, Instant};

const SHADER_PATH: &str = "shaders/tie.wgsl";

/// Global tie draw-distance cap: boot `fun_001e9b10` stores 0x44340000 (720.0) in the word `TieProc`
/// `pminw`s every instance distance with (boot gp−0x5c90 = 0x160f70).
pub const DRAW_DISTANCE_CAP: f32 = 720.0;

/// Per vertex: light slot | TEX1 K (s12, 1/16 units) << 20.
pub const ATTRIBUTE_TIE_INFO: MeshVertexAttribute = MeshVertexAttribute::new("TieInfo", 0x5449_4549, VertexFormat::Uint32);
/// Per vertex: morph colour slot 1 | slot 2 << 6 | fat << 12 | LOD << 13.
pub const ATTRIBUTE_TIE_MORPH: MeshVertexAttribute = MeshVertexAttribute::new("TieMorph", 0x5449_454d, VertexFormat::Uint32);
/// Per vertex: class-space morph delta (world units; fat vertices are drawn at position + k·delta).
pub const ATTRIBUTE_TIE_DELTA: MeshVertexAttribute = MeshVertexAttribute::new("TieDelta", 0x5449_4544, VertexFormat::Float32x3);

/// Tie classes, instances and their lit colour tables after load.
pub struct LevelTies {
    pub classes: Vec<LevelTieClass>,
    pub instances: Vec<TieInstance>,
    /// Per instance: its class (index into `classes`).
    pub class_of: Vec<Option<usize>>,
    /// Per instance: `LightTies` output, indexed by light slot.
    pub colors: Vec<[[u8; 4]; SLOTS]>,
    /// Tie texture index → mip chain (level 0 first).
    pub mips: HashMap<usize, Vec<Texture>>,
    /// Textures whose mip chain did not decode (drawn with level 0 only).
    pub mip_fallbacks: usize,
    pub parse_time: Duration,
    pub light_time: Duration,
    pub texture_time: Duration,
}

/// Parses the tie classes and instances, lights every instance and decodes the tie textures.
pub fn load_ties(core: &LevelCore, core_data: &[u8], gs_ram: &[u8], gameplay: &[u8], textures: &[LevelTexture]) -> Result<LevelTies> {
    let t0 = Instant::now();
    let classes = tie::parse_level_ties(core, core_data).context("parsing tie classes")?;
    let instances = tie::parse_tie_instances(gameplay).context("parsing tie instances")?;
    let by_class: HashMap<i32, usize> = classes.iter().enumerate().map(|(i, c)| (c.o_class, i)).collect();
    let class_of: Vec<Option<usize>> = instances.iter().map(|i| by_class.get(&i.o_class).copied()).collect();
    let parse_time = t0.elapsed();

    let t0 = Instant::now();
    let colors = crate::tie_light::light_instances(gameplay, &classes, &instances, &class_of)?;
    let light_time = t0.elapsed();

    // Tie textures get the same load-time ad-gif conversion as tfrags (boot fun_00203730: TEX0 from the
    // texture entry, MXL = ty − 1, MIPTBP1 from mipmap / pad) and the same paging layout (fun_002370c0:
    // square base level, mip 1 right after it), so the tfrag mip decoder applies.
    let t0 = Instant::now();
    let used: HashSet<usize> = classes.iter().flat_map(|c| (0..c.class.ad_gifs.len()).filter_map(|a| c.texture_table_index(a as u16))).collect();
    let mut mips = HashMap::new();
    let mut mip_fallbacks = 0;
    for idx in used {
        let Some(e) = core.tie_textures.get(idx) else { continue };
        match texture::decode_tfrag_mip_levels(core, core_data, gs_ram, e) {
            Ok(levels) => { mips.insert(idx, levels); }
            Err(_) => {
                if let Some(t) = textures.iter().find(|t| t.table == TextureTable::Tie && t.index == idx) {
                    mip_fallbacks += 1;
                    mips.insert(idx, vec![t.texture.clone()]);
                }
            }
        }
    }
    Ok(LevelTies { classes, instances, class_of, colors, mips, mip_fallbacks, parse_time, light_time, texture_time: t0.elapsed() })
}

/// Material uniform: x = near (32; GS Q = n / z), y = MXL, z = 1 to tint by LOD (`RC_TIE_LOD_TINT=1`).
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct TieParams {
    pub misc: Vec4,
}

/// Tie material: texture × slot colour (GS MODULATE), per-instance fog, per-instance data in storage.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GsPass)]
pub struct TieMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[storage(2, read_only)]
    /// The shared fog buffer (crate::game_camera::fog_buffer).
    pub fog: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(3)]
    pub params: TieParams,
    /// One `TieInst` per gameplay instance (see tie.wgsl), indexed by `MeshTag`.
    #[storage(4, read_only, visibility(vertex))]
    pub instances: Handle<ShaderBuffer>,
    /// One `vec4<u32>` per instance, rewritten every frame (`tie_lod::TieLodState`).
    #[storage(5, read_only, visibility(vertex))]
    pub lods: Handle<ShaderBuffer>,
    /// This draw's GS state (crate::gs_state).
    pub pass: GsPass,
}

impl From<&TieMaterial> for GsPass {
    fn from(m: &TieMaterial) -> Self { m.pass }
}

impl Material for TieMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { self.pass.alpha_mode() }
    /// The GS does not cull and instance matrices can mirror (12 Novalis instances have det < 0).
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        key.bind_group_data.specialize(descriptor);
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            ATTRIBUTE_TIE_INFO.at_shader_location(2),
            ATTRIBUTE_TIE_MORPH.at_shader_location(3),
            ATTRIBUTE_TIE_DELTA.at_shader_location(4),
        ])?];
        Ok(())
    }
}

pub struct TieRenderPlugin;

impl Plugin for TieRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TieMaterial>::default())
            .add_systems(crate::level_switch::LevelStartup, spawn_system)
            .add_systems(crate::level_switch::LevelUnload, crate::level_switch::remove::<crate::tie_lod::TieLodState>)
            .add_systems(
                PostUpdate,
                // Before visibility propagation: the Visibility it writes applies to this frame (tie_lod.rs).
                crate::tie_lod::update_tie_lods
                    .after(crate::occlusion::OcclusionSet)
                    .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            );
    }
}

/// CLAMP_1 wrap state of a tie ad-gif (qword 3: `data_lo` = WMS, `data_hi` = WMT, 0 repeat / 1 clamp,
/// copied into the register by the load-time conversion like the tfrag one).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct SamplerKey {
    clamp_s: bool,
    clamp_t: bool,
}

impl SamplerKey {
    fn of(ad: &TieAdGifs) -> Self { SamplerKey { clamp_s: ad.clamp.data_lo & 1 != 0, clamp_t: ad.clamp.data_hi & 1 != 0 } }
    /// TEX1: MMAG = LINEAR, MMIN = TEX1.hi = 4 (LINEAR_MIPMAP_NEAREST) on every retail tie ad-gif.
    fn descriptor(self) -> ImageSamplerDescriptor {
        let wrap = |clamp: bool| if clamp { ImageAddressMode::ClampToEdge } else { ImageAddressMode::Repeat };
        ImageSamplerDescriptor {
            address_mode_u: wrap(self.clamp_s),
            address_mode_v: wrap(self.clamp_t),
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Nearest,
            ..default()
        }
    }
}

/// One class mesh: (tie texture index, wrap), the mesh, triangles per LOD, light slots used.
struct Part {
    texture: usize,
    skey: SamplerKey,
    mesh: Handle<Mesh>,
    triangles: [usize; 3],
    slots: u64,
}

#[derive(Default)]
struct Stats {
    placed: usize,
    no_class: usize,
    zero_distance: usize,
    classes_used: usize,
    meshes: usize,
    entities: usize,
    draw_batches: usize,
    /// Per LOD, summed over the placed instances.
    triangles: [usize; 3],
    vertices: usize,
    untextured: usize,
    sheared: usize,
    mirrored: usize,
    images: usize,
    blended_materials: usize,
    clamped_slots: usize,
    build: Duration,
}

fn build_parts(c: &LevelTieClass, meshes: &mut Assets<Mesh>, untextured: &mut usize) -> Vec<Part> {
    #[derive(Default)]
    struct B {
        pos: Vec<[f32; 3]>,
        delta: Vec<[f32; 3]>,
        uv: Vec<[f32; 2]>,
        info: Vec<u32>,
        morph: Vec<u32>,
        idx: Vec<u32>,
        remap: HashMap<(usize, usize, u16, u32), u32>,
        tris: [usize; 3],
        slots: u64,
    }
    let mut by_key: BTreeMap<(usize, SamplerKey), B> = BTreeMap::new();
    let scale = c.class.header.scale;
    for (lod, packets) in c.class.lods.iter().enumerate() {
        for (pi, pk) in packets.iter().enumerate() {
            for tri in tie::tie_triangles(pk) {
                let Some(tex) = c.texture_table_index(tri.ad_gif) else { *untextured += 1; continue };
                let Some(ad) = c.class.ad_gifs.get(tri.ad_gif as usize) else { *untextured += 1; continue };
                let k12 = ad.tex1.data_lo as u32 & 0xfff;
                let b = by_key.entry((tex, SamplerKey::of(ad))).or_default();
                b.tris[lod] += 1;
                for vi in [tri.a, tri.b, tri.c] {
                    let v = *b.remap.entry((lod, pi, vi, k12)).or_insert_with(|| {
                        let vx = &pk.vertices[vi as usize];
                        b.pos.push(vx.class_position(scale));
                        b.delta.push(vx.class_morph_delta(scale));
                        b.uv.push(vx.uv());
                        b.info.push(vx.color as u32 & 63 | k12 << 20);
                        let [m1, m2] = vx.morph_colors.map(|s| s as u32 & 63);
                        b.morph.push(m1 | m2 << 6 | (vx.is_fat() as u32) << 12 | (lod as u32) << 13);
                        b.slots |= 1 << (vx.color & 63);
                        if vx.is_fat() { b.slots |= 1 << m1 | 1 << m2; }
                        (b.pos.len() - 1) as u32
                    });
                    b.idx.push(v);
                }
            }
        }
    }
    by_key
        .into_iter()
        .map(|((texture, skey), b)| {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, b.uv)
                .with_inserted_attribute(ATTRIBUTE_TIE_INFO, VertexAttributeValues::Uint32(b.info))
                .with_inserted_attribute(ATTRIBUTE_TIE_MORPH, VertexAttributeValues::Uint32(b.morph))
                .with_inserted_attribute(ATTRIBUTE_TIE_DELTA, b.delta)
                .with_inserted_indices(Indices::U32(b.idx));
            Part { texture, skey, mesh: meshes.add(mesh), triangles: b.tris, slots: b.slots }
        })
        .collect()
}

/// `game_to_bevy` as a 4×4 (columns = images of game x, y, z).
fn axes() -> Mat4 { Mat4::from_mat3(Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y)) }

/// The static storage record (`TieInst` in tie.wgsl): model (16 f32), centre xyz + draw distance, misc
/// (x = bounding radius), then the 64 colours as packed RGBA8, then the class's 64 normals `LightTies` lights with,
/// packed as the raw s16 (x | y << 16, z), for the point lights (crate::world_lights; the centre is their origin).
/// (Culling, LOD and fog come from the per-frame `tie_lod` buffer.) The normals travel in the record, not in a
/// buffer of their own: a new binding changes the material layout, which reorders draws and so the pixels of
/// frames without any light.
fn write_record(out: &mut Vec<u8>, model: &Mat4, centre: Vec3, dist: f32, radius: f32, colors: &[[u8; 4]; SLOTS], normals: &[[i16; 4]]) {
    for v in model.to_cols_array() { out.extend_from_slice(&v.to_le_bytes()); }
    for v in [centre.x, centre.y, centre.z, dist, radius, 0.0, 0.0, 0.0] { out.extend_from_slice(&v.to_le_bytes()); }
    for c in colors { out.extend_from_slice(c); }
    for j in 0..SLOTS {
        let n = normals.get(j).copied().unwrap_or([0; 4]);
        out.extend_from_slice(&(n[0] as u16 as u32 | (n[1] as u16 as u32) << 16).to_le_bytes());
        out.extend_from_slice(&(n[2] as u16 as u32).to_le_bytes());
    }
}

fn spawn_system(
    mut commands: Commands,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TieMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    // `RC_NO_TIES=1`: draw no ties (for comparisons).
    if std::env::var("RC_NO_TIES").is_ok_and(|v| v.trim() == "1") { return; }
    let s = spawn_ties(&mut commands, &level.0, &mut meshes, &mut images, &mut materials, &mut buffers);
    let t = &level.0.ties;
    println!(
        "ties: {} instances ({} placed, {} without class, {} with draw distance 0; {} mirrored, {} sheared), {} classes ({} used); \
         LOD 0/1/2 triangles over all placed instances {:?} ({} untextured skipped per class), {} meshes ({} vertices), {} entities, <= {} instanced draws, \
         {} images ({} mip fallbacks), {} blended materials; {} lit slots at the 243 clamp",
        t.instances.len(), s.placed, s.no_class, s.zero_distance, s.mirrored, s.sheared, t.classes.len(), s.classes_used,
        s.triangles, s.untextured, s.meshes, s.vertices, s.entities, s.draw_batches, s.images, t.mip_fallbacks, s.blended_materials, s.clamped_slots
    );
    println!(
        "ties: parse {:.1} ms, LightTies {:.1} ms, textures {:.1} ms, mesh build + spawn {:.1} ms",
        t.parse_time.as_secs_f64() * 1e3, t.light_time.as_secs_f64() * 1e3, t.texture_time.as_secs_f64() * 1e3, s.build.as_secs_f64() * 1e3
    );
}

fn spawn_ties(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<TieMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> Stats {
    spawn_ties_on(commands, level, meshes, images, materials, buffers, None)
}

/// The ties of `level` (their LOD state inserted as the resource the per-frame LOD system drives), on render layer
/// `layer` (None: the default layer; crate::title_world draws the title world on its own layer). Returns the entity
/// count.
pub(crate) fn spawn_ties_layered(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<TieMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
    layer: &bevy::camera::visibility::RenderLayers,
) -> usize {
    spawn_ties_on(commands, level, meshes, images, materials, buffers, Some(layer)).entities
}

fn spawn_ties_on(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<TieMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
    layer: Option<&bevy::camera::visibility::RenderLayers>,
) -> Stats {
    let t0 = Instant::now();
    let ties = &level.ties;
    let mut st = Stats::default();

    // Class meshes, only for classes something instances, in class order (asset ids follow it).
    let used: BTreeSet<usize> = ties.class_of.iter().flatten().copied().collect();
    st.classes_used = used.len();
    let mut parts: HashMap<usize, Vec<Part>> = HashMap::new();
    for &ci in &used {
        let p = build_parts(&ties.classes[ci], meshes, &mut st.untextured);
        st.meshes += p.len();
        st.vertices += p.iter().map(|p| meshes.get(&p.mesh).map_or(0, |m| m.count_vertices())).sum::<usize>();
        parts.insert(ci, p);
    }

    // Per-instance storage records (index = instance index = MeshTag).
    let a = axes();
    let mut bytes = Vec::with_capacity(ties.instances.len() * 864);
    let mut models = Vec::with_capacity(ties.instances.len());
    let mut lod_inputs = Vec::with_capacity(ties.instances.len());
    for (ii, inst) in ties.instances.iter().enumerate() {
        let model = a * Mat4::from_cols_array_2d(&inst.world_matrix());
        let (sphere, dists) = match ties.class_of[ii] {
            Some(ci) => {
                let h = &ties.classes[ci].class.header;
                (instance_centre(&ties.classes[ci].class, inst), [h.near_dist, h.mid_dist, h.far_dist])
            }
            None => ([inst.matrix[3][0], inst.matrix[3][1], inst.matrix[3][2], 0.0], [0.0; 3]),
        };
        let (centre, radius) = (game_to_bevy([sphere[0], sphere[1], sphere[2]]), sphere[3]);
        // Instance +0x04 is an s32 (720 on every Novalis tie); the loader converts it with cvt.s.w.
        let dist = (inst.draw_distance as f32).min(DRAW_DISTANCE_CAP);
        let lod_dist = if ties.class_of[ii].is_some() { dist } else { 0.0 };
        lod_inputs.push(crate::tie_lod::TieLodInput { sphere, dist: lod_dist, dists });
        let normals = ties.class_of[ii].map_or(&[][..], |ci| &ties.classes[ci].class.normals[..]);
        write_record(&mut bytes, &model, centre, dist, radius, &ties.colors[ii], normals);
        models.push(model);
    }
    let inst_buffer = buffers.add(ShaderBuffer::new(&bytes, RenderAssetUsages::RENDER_WORLD));
    let mut lod_state = crate::tie_lod::TieLodState::new(lod_inputs, level.occlusion.objects.tie.clone(), level.fog, buffers);
    let lod_buffer = lod_state.buffer.clone();
    let tint = if crate::tie_lod::tint_enabled() { 1.0 } else { 0.0 };

    // Images (mip chain, raw GS alpha) and materials per (texture, wrap, blend).
    let fallback = images.add(Image::new_fill(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255, 0, 255, 0x80],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let mut image_cache: HashMap<(usize, SamplerKey), (Handle<Image>, AlphaRange, u32)> = HashMap::new();
    let mut image_for = |tex: usize, skey: SamplerKey, images: &mut Assets<Image>| {
        image_cache
            .entry((tex, skey))
            .or_insert_with(|| match ties.mips.get(&tex) {
                Some(levels) => {
                    // Same conventions as tfrag_render: Rgba8Unorm raw bytes, alpha back to the GS 0..0x80.
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
                    warn!("tie texture {tex} has no decoded texture; drawing magenta");
                    (fallback.clone(), AlphaRange::OPAQUE, 0)
                }
            })
            .clone()
    };
    let mut mats: HashMap<(usize, SamplerKey, GsPass), Handle<TieMaterial>> = HashMap::new();
    let mut batches: HashSet<(AssetId<Mesh>, AssetId<TieMaterial>)> = HashSet::new();

    for (ii, inst) in ties.instances.iter().enumerate() {
        let Some(ci) = ties.class_of[ii] else { st.no_class += 1; continue };
        if inst.draw_distance == 0 { st.zero_distance += 1; continue; }
        st.placed += 1;
        let model = models[ii];
        let transform = Transform::from_matrix(model);
        let exact = transform.to_matrix().abs_diff_eq(model, 1e-3 * model.x_axis.length().max(1.0));
        if model.determinant() < 0.0 { st.mirrored += 1; }
        if !exact { st.sheared += 1; }
        let colors = &ties.colors[ii];
        let mut spawned = Vec::new();
        for (pi, part) in parts[&ci].iter().enumerate() {
            for l in 0..3 { st.triangles[l] += part.triangles[l]; }
            // Af of every slot the part reads (fat-vertex morph colours are blends of two of them).
            let vertex_alpha = AlphaRange::of((0..SLOTS).filter(|&s| part.slots & (1 << s) != 0).map(|s| colors[s][3]));
            let (image, texel_alpha, mxl) = image_for(part.texture, part.skey, images);
            for pass in gs_state::draws(gs_state::AREF_WORLD, texel_alpha, vertex_alpha) {
                let mat = mats
                    .entry((part.texture, part.skey, pass))
                    .or_insert_with(|| {
                        materials.add(TieMaterial {
                            texture: image.clone(),
                            fog: crate::game_camera::fog_buffer(),
                            params: TieParams { misc: Vec4::new(crate::game_camera::NEAR, mxl as f32, tint, 0.0) },
                            instances: inst_buffer.clone(),
                            lods: lod_buffer.clone(),
                            pass,
                        })
                    })
                    .clone();
                batches.insert((part.mesh.id(), mat.id()));
                let mut e = commands.spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(mat),
                    transform,
                    MeshTag(ii as u32),
                    WorldDrawOrder([WorldDrawOrder::TIE, ci as u32, pi as u32, ii as u32, 0]),
                    Name::new(format!("tie {ii} class {} tex {} {pass:?}", inst.o_class, part.texture)),
                ));
                if !exact { e.insert(NoFrustumCulling); }
                if let Some(l) = layer { e.insert(l.clone()); }
                spawned.push(e.id());
                st.entities += 1;
            }
        }
        lod_state.set_entities(ii, spawned);
    }
    commands.insert_resource(lod_state);
    for (ii, c) in ties.colors.iter().enumerate() {
        let Some(ci) = ties.class_of[ii] else { continue };
        let slots = parts[&ci].iter().fold(0u64, |m, p| m | p.slots);
        st.clamped_slots += (0..SLOTS).filter(|&s| slots & (1 << s) != 0 && c[s][..3].contains(&243)).count();
    }
    st.draw_batches = batches.len();
    st.images = image_cache.len();
    st.blended_materials = mats.keys().filter(|k| k.2 != GsPass::Opaque).count();
    st.build = t0.elapsed();
    st
}
