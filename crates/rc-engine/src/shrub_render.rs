//! Shrub (small instanced prop: grass, plants, rocks) renderer: every gameplay shrub instance, drawn with
//! its class mesh, its own 4×4 matrix, the class textures and the 24-entry palette the game's
//! `LightShrubs` pass computes for it (crate::shrub_light, docs/plan/shrub_lighting.md).
//!
//! Geometry: per class, one mesh per (shrub texture, CLAMP wrap state) from all packets
//! (`shrub::shrub_triangles`), in class space: `position × scale / 1024` (header 0x20). That is the whole
//! placement: `ShrubProc` (level01 0x29cdf0) uploads the VU1 matrix `V · [c0·s, c1·s, c2·s, (t − eye)·1024]`
//! (instance columns times the class `scale` it keeps in col3.w, translation relative to the camera in
//! 1/1024 units) and VU1 program 56467 multiplies the raw s16 positions (`itof0`) by it; the 2²³ bias in
//! VU1's `vf30` / `vf17` is address arithmetic for the GS-packet slots, not part of the transform. Each
//! vertex carries its UV (s16 / 4096; every retail `h` = 1.0) and one u32: the palette entry VU1 reads its
//! colour from. That is the vertex's normal index, except for vertex 3 of the 7 retail packets with 6
//! written vertices (stop flag on vertex 2), which VU1 reads from `n + 2·vi13`
//! (`rc_formats::shrub_light::vu1_colour_address`). The contents there are run-time state; the port models
//! the instance as slot 0 of the batch in buffer 0xee, where the address falls in the other buffer's slot
//! 3 / 4, and takes that slot's palette entry from the instance's own palette (a neighbour of the same class
//! in the game); when it falls on a matrix column the vertex keeps its own entry.
//!
//! Instances: one entity per (instance, class part) with `MeshTag(instance index)`, so Bevy batches every
//! instance of a part into one instanced draw; the storage record holds the class → Bevy-world matrix
//! (can hold scale, shear and mirroring, like ties), the bounding-sphere centre and draw distance, the
//! origin and billboard fade distance, and the palette. The entity `Transform` only feeds Bevy's frustum
//! culling (sheared instances get `NoFrustumCulling`); a CPU pass (`cull_system`) additionally hides
//! entities that the distance / fade rules cannot draw this frame, with a 2-unit margin.
//!
//! Per-instance rules (level-load `FUN_00255958` and `ShrubProc`, docs/plan/shrub_lighting.md §5, §6):
//! * draw distance D = `max(draw_distance, 16)` (f32 at instance +0x04, loaded with `lwc1`), raised to
//!   `F + 24` for billboard classes (F = `trunc(fade_distance) & 0xff`, the byte at record +0x17), then
//!   `pminw`'d with the global cap at `gp-0x675c` (500.0, set by the loader); the instance is skipped when
//!   the view depth z of its bounding-sphere centre exceeds D;
//! * vertex alpha = palette entry 0's alpha (0x80) for opaque instances; non-billboard classes fade over the
//!   last 8 units (`min(16·(D − z), 128)`); billboard classes draw the mesh opaque below F and fade it out
//!   over F..F + 8 while the billboard fades in; beyond that only the billboard sprite
//!   (crate::shrub_billboard) is drawn (F = 0: never a mesh);
//! * fog: one F per instance from the origin depth (VU1 qw 4.z), with the tfrag slope / offset / clamps
//!   (the VU form is `z · q_num + 254.99998` on the 0x167180 matrix's z lane; the equivalence is inferred);
//! * classes with `mode_bits & 1` are skipped; `(mode_bits & 6) >> 1` selects wind sway: `ShrubProc` shears
//!   the instance columns (`x += sx·z, y += sy·z`, world Z up, about the origin) with `rc_formats::shrub::wind_sway`,
//!   recomputed here every frame (`update_sway`, driven by the 60 Hz fixed clock) into a per-instance
//!   `(sx, sy)` storage buffer the vertex shader applies; `RC_SWAY=0` disables it;
//! * GS: every shrub GIF tag has PRIM 0x7c (strip, Gouraud, textured, fog, ABE = 1), MODULATE. `ShrubProc`
//!   draws two lists: instances at alpha 0x80 (the opaque list) after TEST_1 = 0x5320b (AREF 0x20), then the
//!   fading ones (alpha < 0x80) after TEST_1 = 0x530cb (AREF 0x0c); both AFAIL = RGB_ONLY, so fragments at or
//!   above AREF write Z (crate::gs_state has the addresses). Per class part, one entity per list and GS draw
//!   (`gs_state::draws`): the opaque list is one `Opaque` draw for a texture with every At = 0x80, else the
//!   AREF 0x20 split; the fading list is always the AREF 0x0c split. The vertex shader keeps each list's
//!   entities to the instances of that list this frame.
//! * textures: the class init (boot `fun_00203b08`) converts shrub ad-gifs bit for bit like the tfrag init
//!   (`ShrubAdGifs::gs_registers`): MXL = ty − 1, the mip chain of `texture::decode_tfrag_mip_levels`, the
//!   GS mip rule `round(log2(z / 32) + K)` per pixel with the block's TEX1 K, and its CLAMP wrap (one ad-gif
//!   state per class texture slot on every retail class).

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
use rc_formats::shrub::{self, LevelShrubClass, ShrubInstance};
use rc_formats::shrub_light::{instance_centre, vu1_colour_address, vu1_palette_base, vu1_shrub_data, Vu1ShrubData, PALETTE};
use rc_formats::texture::{self, LevelTexture, Texture, TextureTable};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::{Duration, Instant};

const SHADER_PATH: &str = "shaders/shrub.wgsl";

/// Global shrub draw-distance cap: the level loader stores 0x43fa0000 (500.0) at `gp-0x675c` (0x1604a4),
/// which `ShrubProc` `pminw`s every instance distance with (other writers: 40.0 / 500.0 in level01
/// `FUN_00216198`, 144.0 in `FUN_002cb338`; not modelled).
pub const DRAW_DISTANCE_CAP: f32 = 500.0;
/// The loader's minimum draw distance (`0x41800000`).
pub const DRAW_DISTANCE_MIN: f32 = 16.0;

/// Per vertex: the palette entry VU1 reads the colour from | TEX1 K (s12, 1/16 units) << 20.
pub const ATTRIBUTE_SHRUB_INFO: MeshVertexAttribute = MeshVertexAttribute::new("ShrubInfo", 0x5348_5242, VertexFormat::Uint32);

/// Shrub classes, instances and their lit palettes after load.
pub struct LevelShrubs {
    pub classes: Vec<LevelShrubClass>,
    pub instances: Vec<ShrubInstance>,
    /// Per instance: its class (index into `classes`).
    pub class_of: Vec<Option<usize>>,
    /// Per instance: `LightShrubs` output, indexed by normal.
    pub palettes: Vec<[[u8; 4]; PALETTE]>,
    /// Shrub texture index → mip chain (level 0 first; ty levels).
    pub mips: HashMap<usize, Vec<Texture>>,
    /// Textures whose mip chain did not decode (level 0 only).
    pub mip_fallbacks: usize,
    /// Class index → billboard texture mip chain (`texture::decode_billboard_mip_levels`), for classes with one.
    pub billboard_mips: HashMap<usize, Vec<Texture>>,
    pub parse_time: Duration,
    pub light_time: Duration,
    pub texture_time: Duration,
}

/// Parses the shrub classes and instances, lights every instance and decodes the mip chains of the shrub and
/// billboard textures. Levels 2 / 3, the palettes and the billboards live in `gs_ram`, the GS image of the world
/// being loaded (a level's `gs_ram.bin`, or the title world's). Textures that do not decode fall back to the
/// world's decoded base level.
pub fn load_shrubs(core: &rc_formats::level::LevelCore, core_data: &[u8], gs_ram: &[u8], gameplay: &[u8], textures: &[LevelTexture]) -> Result<LevelShrubs> {
    let t0 = Instant::now();
    let classes = shrub::parse_level_shrubs(core, core_data).context("parsing shrub classes")?;
    let instances = shrub::parse_shrub_instances(gameplay).context("parsing shrub instances")?;
    let by_class: HashMap<i32, usize> = classes.iter().enumerate().map(|(i, c)| (c.o_class, i)).collect();
    let class_of: Vec<Option<usize>> = instances.iter().map(|i| by_class.get(&i.o_class).copied()).collect();
    let used: HashSet<usize> = classes
        .iter()
        .flat_map(|c| c.class.packets.iter().flat_map(|p| p.draws.iter().filter_map(|d| c.texture_table_index(d.texture as u16))))
        .collect();
    let parse_time = t0.elapsed();
    let t0 = Instant::now();
    let palettes = crate::shrub_light::light_instances(gameplay, &classes, &instances, &class_of)?;
    let light_time = t0.elapsed();

    let t0 = Instant::now();
    // The GS image of the world being loaded (the level's gs_ram, or the title world's): the mip levels 2 / 3, the
    // palettes and the billboards live there. (It was read from the *current* level's file, which gave the title world
    // and a runtime level change still on the old level the wrong palettes and mips.)
    let gs_ram = Some(gs_ram);
    let base = |table: TextureTable, index: usize| textures.iter().find(|t| t.table == table && t.index == index).map(|t| vec![t.texture.clone()]);
    let mut mips = HashMap::new();
    let mut mip_fallbacks = 0;
    for idx in used {
        let chain = gs_ram.and_then(|gs| core.shrub_textures.get(idx).and_then(|e| texture::decode_tfrag_mip_levels(core, core_data, gs, e).ok()));
        let levels = match chain {
            Some(l) => Some(l),
            None => base(TextureTable::Shrub, idx).inspect(|_| mip_fallbacks += 1),
        };
        if let Some(l) = levels { mips.insert(idx, l); }
    }
    let mut billboard_mips = HashMap::new();
    if let Some(gs) = gs_ram {
        for (ci, c) in classes.iter().enumerate() {
            let Some(info) = c.billboard_texture() else { continue };
            match texture::decode_billboard_mip_levels(gs, info) {
                Ok(levels) => { billboard_mips.insert(ci, levels); }
                Err(e) => warn!("shrub class {}: billboard texture not decoded ({e})", c.o_class),
            }
        }
    }
    Ok(LevelShrubs { classes, instances, class_of, palettes, mips, mip_fallbacks, billboard_mips, parse_time, light_time, texture_time: t0.elapsed() })
}

/// Material uniform: x = list (0 always, 1 only the opaque list, 2 only the fading list; see shrub.wgsl), y = MXL,
/// z = near (32; GS Q = n / z).
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct ShrubParams {
    pub misc: Vec4,
}

/// Shrub material: texture × palette colour (GS MODULATE), per-instance fog and alpha, per-instance data in storage.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GsPass)]
pub struct ShrubMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[storage(2, read_only)]
    /// The shared fog buffer (crate::game_camera::fog_buffer).
    pub fog: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(3)]
    pub params: ShrubParams,
    /// One `ShrubInst` per gameplay instance (see shrub.wgsl), indexed by `MeshTag`.
    #[storage(4, read_only, visibility(vertex))]
    pub instances: Handle<ShaderBuffer>,
    /// The per-frame buffer ([`ShrubSway`]): one `vec2<f32>` wind shear `(sx, sy)` per gameplay instance (rewritten
    /// every frame by `update_sway`), then one `vec2` per instance whose x holds its point-light nibble list (u32 bits),
    /// then the point-light bank (8 × 4 `vec2`; crate::world_lights).
    #[storage(5, read_only, visibility(vertex))]
    pub sway: Handle<ShaderBuffer>,
    /// This draw's GS state (crate::gs_state).
    pub pass: GsPass,
}

impl From<&ShrubMaterial> for GsPass {
    fn from(m: &ShrubMaterial) -> Self { m.pass }
}

impl Material for ShrubMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { self.pass.alpha_mode() }
    /// The GS does not cull, strips have no consistent winding, and matrices may mirror.
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
            ATTRIBUTE_SHRUB_INFO.at_shader_location(2),
        ])?];
        Ok(())
    }
}

pub struct ShrubRenderPlugin;

impl Plugin for ShrubRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ShrubMaterial>::default())
            .add_plugins(crate::shrub_billboard::ShrubBillboardPlugin)
            .add_systems(crate::level_switch::LevelStartup, spawn_system)
            .add_systems(crate::level_switch::LevelUnload, crate::level_switch::remove::<ShrubSway>)
            .add_systems(Update, cull_system)
            .add_systems(PostUpdate, update_sway);
    }
}

/// CPU copy of the shader's distance rules, so entities that cannot draw this frame skip the render
/// pipeline (hundreds of instances sit beyond their 32-unit draw distance from any view). Conservative by
/// `CULL_MARGIN`; the vertex shader applies the exact rules to whatever stays visible.
#[derive(Component, Clone, Copy)]
struct ShrubCull {
    centre: Vec3,
    d: f32,
    f: f32,
    variant: u8,
}

/// World units of slack for the CPU test (camera `Transform` of the current `Update`, not the rendered one).
const CULL_MARGIN: f32 = 2.0;

fn cull_system(
    cams: Query<&Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>,
    mut q: Query<(&ShrubCull, &mut Visibility)>,
) {
    let Some(cam) = cams.iter().next() else { return };
    let (eye, fwd) = (cam.translation, *cam.forward());
    for (c, mut vis) in &mut q {
        let z = (c.centre - eye).dot(fwd);
        let mut show = z <= c.d + CULL_MARGIN;
        if c.f >= 0.0 {
            show &= z < c.f + 8.0 + CULL_MARGIN;
            if c.variant == 2 { show &= z > c.f - CULL_MARGIN; }
        } else if c.variant == 2 {
            show &= z > c.d - 8.0 - CULL_MARGIN;
        }
        vis.set_if_neq(if show { Visibility::Inherited } else { Visibility::Hidden });
    }
}

/// CLAMP_1 wrap state of a shrub ad-gif (`data_lo` = WMS, `data_hi` = WMT; 0 repeat / 1 clamp).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct SamplerKey {
    clamp_s: bool,
    clamp_t: bool,
}

impl SamplerKey {
    fn descriptor(self) -> ImageSamplerDescriptor {
        let wrap = |clamp: bool| if clamp { ImageAddressMode::ClampToEdge } else { ImageAddressMode::Repeat };
        ImageSamplerDescriptor {
            address_mode_u: wrap(self.clamp_s),
            address_mode_v: wrap(self.clamp_t),
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            // TEX1 MMIN = 4 (LINEAR_MIPMAP_NEAREST) on every retail shrub ad-gif; shrub.wgsl picks the level (an integer one
            // is read alone). Linear between levels and anisotropic for the texture option (crate::graphics).
            mipmap_filter: ImageFilterMode::Linear,
            anisotropy_clamp: 16,
            ..default()
        }
    }
}

/// One class mesh: (shrub texture index, wrap), the mesh and its triangle count.
struct Part {
    texture: usize,
    skey: SamplerKey,
    mesh: Handle<Mesh>,
    triangles: usize,
}

#[derive(Default)]
struct Stats {
    placed: usize,
    no_class: usize,
    skipped_class: usize,
    billboard_mesh: usize,
    billboard_only: usize,
    sway: usize,
    sway_sheared: usize,
    mirrored: usize,
    sheared: usize,
    classes_used: usize,
    meshes: usize,
    entities: usize,
    fade_twins: usize,
    draw_batches: usize,
    triangles: usize,
    untextured: usize,
    quirk_vertices: usize,
    quirk_remapped: usize,
    images: usize,
    blended_materials: usize,
    build: Duration,
}

fn build_parts(c: &LevelShrubClass, meshes: &mut Assets<Mesh>, st: &mut Stats) -> Vec<Part> {
    #[derive(Default)]
    struct B { pos: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, info: Vec<u32>, idx: Vec<u32>, remap: HashMap<(usize, u16, u32), u32>, tris: usize }
    // Texture slot → (wrap, TEX1 K as s12) of the ad-gif block that selects it. The GS state carries across
    // packets, but on every retail class all blocks naming one slot agree on TEX1 and CLAMP, so the slot is the key.
    let mut state: HashMap<i32, (SamplerKey, u32)> = HashMap::new();
    for a in c.class.packets.iter().flat_map(|p| &p.ad_gifs) {
        let (clamp_s, clamp_t) = a.clamp_st();
        state.entry(a.tex0.data_lo).or_insert((SamplerKey { clamp_s, clamp_t }, a.lod_k_raw() as u32 & 0xfff));
    }
    let mut by_key: BTreeMap<(usize, SamplerKey), B> = BTreeMap::new();
    let scale = c.class.header.scale;
    // The quirk model: slot 0 of the batch in buffer 0xee.
    let base = vu1_palette_base(0xee, 0);
    for (pi, pk) in c.class.packets.iter().enumerate() {
        for tri in shrub::shrub_triangles(pk) {
            let Some(tex) = c.texture_table_index(tri.texture) else { st.untextured += 1; continue };
            let (skey, k12) = state.get(&(tri.texture as i32)).copied().unwrap_or((SamplerKey { clamp_s: false, clamp_t: false }, 0));
            let b = by_key.entry((tex, skey)).or_default();
            b.tris += 1;
            for vi in [tri.a, tri.b, tri.c] {
                let v = *b.remap.entry((pi, vi, k12)).or_insert_with(|| {
                    let vx = &pk.vertices[vi as usize];
                    let n = vx.normal;
                    let addr = vu1_colour_address(pk.vertices.len(), vi as usize, n, base);
                    let entry = if addr == n as u16 + base {
                        n
                    } else {
                        st.quirk_vertices += 1;
                        match vu1_shrub_data(addr) {
                            Vu1ShrubData::Palette { entry, .. } => { st.quirk_remapped += 1; entry }
                            _ => n,
                        }
                    };
                    b.pos.push(vx.class_position(scale));
                    b.uv.push(vx.uv());
                    b.info.push(entry as u32 | k12 << 20);
                    (b.pos.len() - 1) as u32
                });
                b.idx.push(v);
            }
        }
    }
    by_key
        .into_iter()
        .map(|((texture, skey), b)| {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, b.uv)
                .with_inserted_attribute(ATTRIBUTE_SHRUB_INFO, VertexAttributeValues::Uint32(b.info))
                .with_inserted_indices(Indices::U32(b.idx));
            Part { texture, skey, mesh: meshes.add(mesh), triangles: b.tris }
        })
        .collect()
}

/// `game_to_bevy` as a 4×4 (columns = images of game x, y, z).
fn axes() -> Mat4 { Mat4::from_mat3(Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y)) }

/// Billboard fade distance F as the loader keeps it (`trunc(fade_distance)` stored as a byte at +0x17).
pub(crate) fn billboard_f(c: &LevelShrubClass) -> Option<f32> { c.class.billboard.map(|b| ((b.fade_distance as i32) & 0xff) as f32) }

/// The run-time draw distance D (record +0x10 after the loader, then `ShrubProc`'s cap).
pub fn runtime_draw_distance(c: &LevelShrubClass, inst: &ShrubInstance) -> f32 {
    let mut d = if inst.draw_distance < DRAW_DISTANCE_MIN { DRAW_DISTANCE_MIN } else { inst.draw_distance };
    if let Some(f) = billboard_f(c) { if d < f + 24.0 { d = f + 24.0; } }
    d.min(DRAW_DISTANCE_CAP)
}

/// The storage record (`ShrubInst` in shrub.wgsl): model (16 f32), centre xyz + D, origin xyz + F, 24 palette words,
/// then the class's 24 normals `LightShrubs` lights with, packed as the raw s16 (x | y << 16, z) for the point lights
/// (crate::world_lights; the centre is their origin). The normals travel in the record rather than in a buffer of
/// their own: a new binding changes the material layout, which reorders the blended shrub draws and so the pixels of
/// frames without any light.
#[allow(clippy::too_many_arguments)]
fn write_record(out: &mut Vec<u8>, model: &Mat4, centre: Vec3, d: f32, origin: Vec3, f: f32, palette: &[[u8; 4]; PALETTE], normals: &[[i16; 4]]) {
    for v in model.to_cols_array() { out.extend_from_slice(&v.to_le_bytes()); }
    for v in [centre.x, centre.y, centre.z, d, origin.x, origin.y, origin.z, f] { out.extend_from_slice(&v.to_le_bytes()); }
    for c in palette { out.extend_from_slice(c); }
    for j in 0..PALETTE {
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
    mut materials: ResMut<Assets<ShrubMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    // `RC_NO_SHRUBS=1`: draw no shrubs (for comparisons).
    if std::env::var("RC_NO_SHRUBS").is_ok_and(|v| v.trim() == "1") { return; }
    let (s, sway) = spawn_shrubs(&mut commands, &level.0, &mut meshes, &mut images, &mut materials, &mut buffers);
    let t = &level.0.shrubs;
    println!(
        "shrubs: {} instances ({} placed, {} without class, {} in skipped classes; {} of billboard classes drawn as mesh up to F + 8, \
         {} billboard only (no mesh); {} with wind sway ({}; {} not frustum-culled); {} mirrored, {} sheared), {} classes ({} used); \
         {} triangles per full draw ({} untextured skipped per class), {} meshes, {} entities ({} fading-list draws), <= {} instanced draws, \
         {} images ({} mip fallbacks), {} blended materials; 6-vertex colour quirk on {} mesh vertices ({} remapped to a slot palette entry)",
        t.instances.len(), s.placed, s.no_class, s.skipped_class, s.billboard_mesh, s.billboard_only, s.sway,
        if sway.enabled { "on" } else { "RC_SWAY=0: static" }, s.sway_sheared, s.mirrored, s.sheared,
        t.classes.len(), s.classes_used, s.triangles, s.untextured, s.meshes, s.entities, s.fade_twins, s.draw_batches, s.images,
        t.mip_fallbacks, s.blended_materials, s.quirk_vertices, s.quirk_remapped
    );
    println!(
        "shrubs: parse {:.1} ms, LightShrubs {:.1} ms, mip chains {:.1} ms, mesh build + spawn {:.1} ms",
        t.parse_time.as_secs_f64() * 1e3, t.light_time.as_secs_f64() * 1e3, t.texture_time.as_secs_f64() * 1e3, s.build.as_secs_f64() * 1e3
    );
    commands.insert_resource(sway);
}

/// `RC_SWAY` (default on).
fn sway_enabled() -> bool { !std::env::var("RC_SWAY").is_ok_and(|v| v.trim() == "0") }

/// Low bits of the EE address of the matrix-block array (`gp-0x6764`, 64-byte aligned, allocated by the level
/// loader after the tie blocks). Only `address mod 512` reaches the sway phase (`address · 67`, `· 123`,
/// `>> 1`, `& 0xff`), i.e. which of 8 phase groups instance 0 gets; the heap layout is run-time state, taken as 0.
const SWAY_BLOCK_BASE: u32 = 0;

/// Per-frame wind sway state: the storage buffer and, per sway instance, (instance index, mode, origin).
#[derive(Resource)]
pub struct ShrubSway {
    enabled: bool,
    buffer: Handle<ShaderBuffer>,
    len: usize,
    instances: Vec<(u32, u16, [f32; 3])>,
    last_tick: Option<(u32, [f32; 3])>,
    /// The whole buffer: `len` shears, `len` point-light lists, the bank (see `ShrubMaterial::sway`).
    data: Vec<u8>,
}

impl ShrubSway {
    /// Zero shear, every nibble list empty (0xffff), an empty bank.
    fn initial_data(n: usize) -> Vec<u8> {
        let mut d = vec![0u8; n * 16 + crate::world_lights::BANK_BYTES];
        for i in 0..n { d[n * 8 + i * 8..n * 8 + i * 8 + 4].copy_from_slice(&0xffffu32.to_le_bytes()); }
        d
    }

    /// crate::world_lights: the shrubs' nibble lists (instance order) and the bank bytes.
    pub fn set_point_lights(&mut self, lists: &[u16], bank: &[u8], buffers: &mut Assets<ShaderBuffer>) {
        let n = self.len;
        for (i, &l) in lists.iter().take(n).enumerate() { self.data[n * 8 + i * 8..n * 8 + i * 8 + 4].copy_from_slice(&(l as u32).to_le_bytes()); }
        self.data[n * 16..n * 16 + bank.len()].copy_from_slice(bank);
        crate::asset_write::set_buffer(&mut *buffers, &self.buffer, &self.data);
    }
}

/// `ShrubProc`'s sway for every sway-class instance, once per frame, from the 60 Hz fixed clock (the game's
/// frame counter advances once per 60 Hz game frame; this is the same `Time<Fixed>` that ticks moby animation,
/// read as `elapsed / timestep`) and the camera position.
fn update_sway(
    sway: Option<ResMut<ShrubSway>>,
    fixed: Res<Time<Fixed>>,
    cams: Query<&Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(mut sway) = sway else { return };
    if !sway.enabled || sway.instances.is_empty() { return; }
    let Some(cam) = cams.iter().next() else { return };
    let tick = (fixed.elapsed().as_nanos() / fixed.timestep().as_nanos().max(1)) as u32;
    let e = cam.translation;
    let eye = [e.x, -e.z, e.y]; // Bevy → game axes
    if sway.last_tick == Some((tick, eye)) { return; }
    sway.last_tick = Some((tick, eye));
    let sway = &mut *sway;
    let len = sway.len;
    let bytes = &mut sway.data;
    bytes[..len * 8].fill(0);
    for &(ii, mode, t) in &sway.instances {
        let rel = [t[0] - eye[0], t[1] - eye[1], t[2] - eye[2]];
        if let Some([sx, sy]) = shrub::wind_sway(mode, SWAY_BLOCK_BASE.wrapping_add(ii.wrapping_mul(0x40)), tick, rel) {
            let at = ii as usize * 8;
            bytes[at..at + 4].copy_from_slice(&sx.to_le_bytes());
            bytes[at + 4..at + 8].copy_from_slice(&sy.to_le_bytes());
        }
    }
    crate::asset_write::set_buffer(&mut buffers, &sway.buffer, bytes);
}

fn spawn_shrubs(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<ShrubMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> (Stats, ShrubSway) {
    spawn_shrubs_on(commands, level, meshes, images, materials, buffers, None)
}

/// The shrubs of `level` on render layer `layer` (crate::title_world draws the title world on its own layer); their
/// wind sway state is inserted as the resource the per-frame sway system drives. Returns the entity count.
pub(crate) fn spawn_shrubs_layered(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<ShrubMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
    layer: &bevy::camera::visibility::RenderLayers,
) -> usize {
    let (s, sway) = spawn_shrubs_on(commands, level, meshes, images, materials, buffers, Some(layer));
    commands.insert_resource(sway);
    s.entities
}

fn spawn_shrubs_on(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<ShrubMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
    layer: Option<&bevy::camera::visibility::RenderLayers>,
) -> (Stats, ShrubSway) {
    let t0 = Instant::now();
    let shrubs = &level.shrubs;
    let mut st = Stats::default();

    // In class order (asset ids follow it).
    let used: BTreeSet<usize> = shrubs.class_of.iter().flatten().copied().filter(|&ci| shrubs.classes[ci].class.header.mode_bits & 1 == 0).collect();
    st.classes_used = used.len();
    let mut parts: HashMap<usize, Vec<Part>> = HashMap::new();
    for &ci in &used {
        let p = build_parts(&shrubs.classes[ci], meshes, &mut st);
        st.meshes += p.len();
        parts.insert(ci, p);
    }

    // Per-instance storage records (index = instance index = MeshTag).
    let a = axes();
    let mut bytes = Vec::with_capacity(shrubs.instances.len() * 384);
    let mut models = Vec::with_capacity(shrubs.instances.len());
    let mut culls = Vec::with_capacity(shrubs.instances.len());
    for (ii, inst) in shrubs.instances.iter().enumerate() {
        let model = a * Mat4::from_cols_array_2d(&inst.world_matrix());
        let origin = game_to_bevy(inst.matrix[3][..3].try_into().unwrap());
        let (centre, d, f) = match shrubs.class_of[ii] {
            Some(ci) => {
                let c = &shrubs.classes[ci];
                let s = instance_centre(&c.class, inst);
                (game_to_bevy([s[0], s[1], s[2]]), runtime_draw_distance(c, inst), billboard_f(c).unwrap_or(-1.0))
            }
            None => (origin, 0.0, -1.0),
        };
        let normals = shrubs.class_of[ii].map_or(&[][..], |ci| &shrubs.classes[ci].class.normals[..]);
        write_record(&mut bytes, &model, centre, d, origin, f, &shrubs.palettes[ii], normals);
        models.push(model);
        culls.push(ShrubCull { centre, d, f, variant: 0 });
    }
    let inst_buffer = buffers.add(ShaderBuffer::new(&bytes, RenderAssetUsages::RENDER_WORLD));
    // Wind sway: zero shear for every instance until `update_sway` runs (and for good with `RC_SWAY=0`); no point
    // light on any instance until crate::world_lights writes one.
    let n = shrubs.instances.len().max(1);
    let data = ShrubSway::initial_data(n);
    let sway_buffer = buffers.add(ShaderBuffer::new(&data, RenderAssetUsages::default()));
    let mut sway = ShrubSway { enabled: sway_enabled(), buffer: sway_buffer.clone(), len: n, instances: Vec::new(), last_tick: None, data };

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
            .or_insert_with(|| match shrubs.mips.get(&tex) {
                Some(levels) => {
                    let (img, range) = mip_image(levels, skey.descriptor());
                    (images.add(img), range, levels.len() as u32 - 1)
                }
                None => {
                    warn!("shrub texture {tex} has no decoded texture; drawing magenta");
                    (fallback.clone(), AlphaRange::OPAQUE, 0)
                }
            })
            .clone()
    };
    // Materials per (texture, wrap, list, GS draw); list 1 = opaque list, 2 = fading list.
    let mut mats: HashMap<(usize, SamplerKey, u8, GsPass), Handle<ShrubMaterial>> = HashMap::new();
    // Fading-list vertex alpha: `(0x8000 − iz) >> 8` or `min(16·(D − z), 128)` below 0x80.
    let fading = AlphaRange { min: 0, max: 0x7f };
    let mut batches: HashSet<(AssetId<Mesh>, AssetId<ShrubMaterial>)> = HashSet::new();

    for (ii, inst) in shrubs.instances.iter().enumerate() {
        let Some(ci) = shrubs.class_of[ii] else { st.no_class += 1; continue };
        let c = &shrubs.classes[ci];
        if c.class.header.mode_bits & 1 != 0 { st.skipped_class += 1; continue; }
        if let Some(f) = billboard_f(c) {
            if f == 0.0 { st.billboard_only += 1; continue; }
            st.billboard_mesh += 1;
        }
        let mode = (c.class.header.mode_bits & 6) >> 1;
        if mode != 0 {
            st.sway += 1;
            let t = inst.matrix[3];
            sway.instances.push((ii as u32, mode, [t[0], t[1], t[2]]));
        }
        st.placed += 1;
        let model = models[ii];
        let transform = Transform::from_matrix(model);
        let exact = transform.to_matrix().abs_diff_eq(model, 1e-3 * model.x_axis.length().max(1.0));
        if model.determinant() < 0.0 { st.mirrored += 1; }
        if !exact { st.sheared += 1; }
        // The lean moves the top by up to ~0.1 of the height outside the mesh AABB: no Bevy frustum test.
        let swayed = mode != 0 && sway.enabled;
        if swayed && exact { st.sway_sheared += 1; }
        for (pi, part) in parts[&ci].iter().enumerate() {
            st.triangles += part.triangles;
            let (image, texel_alpha, mxl) = image_for(part.texture, part.skey, images);
            let lists = [
                (1u8, gs_state::draws(gs_state::AREF_SHRUB_OPAQUE, texel_alpha, AlphaRange::OPAQUE)),
                (2u8, gs_state::draws(gs_state::AREF_SHRUB_FADING, texel_alpha, fading)),
            ];
            for (list, passes) in lists {
                for pass in passes {
                    let mat = mats
                        .entry((part.texture, part.skey, list, pass))
                        .or_insert_with(|| {
                            materials.add(ShrubMaterial {
                                texture: image.clone(),
                                fog: crate::game_camera::fog_buffer(),
                                params: ShrubParams { misc: Vec4::new(list as f32, mxl as f32, crate::game_camera::NEAR, 0.0) },
                                instances: inst_buffer.clone(),
                                sway: sway_buffer.clone(),
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
                        ShrubCull { variant: list, ..culls[ii] },
                        WorldDrawOrder([WorldDrawOrder::SHRUB, list as u32, ci as u32, pi as u32, ii as u32]),
                        Visibility::Hidden,
                        Name::new(format!("shrub {ii} class {} tex {} list {list} {pass:?}", inst.o_class, part.texture)),
                    ));
                    if !exact || swayed { e.insert(NoFrustumCulling); }
                    if let Some(l) = layer { e.insert(l.clone()); }
                    st.entities += 1;
                    if list == 2 { st.fade_twins += 1; }
                }
            }
        }
    }
    st.draw_batches = batches.len();
    st.images = image_cache.len();
    st.blended_materials = mats.keys().filter(|k| k.3 != GsPass::Opaque).count();
    st.build = t0.elapsed();
    (st, sway)
}

/// A mip chain as one `Rgba8Unorm` image in the tfrag / tie conventions (raw display-encoded bytes, texel alpha
/// back to the GS 0..0x80); also returns the range of its texel alphas.
pub(crate) fn mip_image(levels: &[Texture], sampler: ImageSamplerDescriptor) -> (Image, AlphaRange) {
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
    img.sampler = ImageSampler::Descriptor(sampler);
    (img, range)
}
