//! Shrub billboards: the far LOD of the 83 retail shrub classes with a `ShrubBillboard` record, drawn as the
//! game draws them (docs/plan/shrub_lighting.md §6). `ShrubProc` (level01 0x29cdf0 = boot `fun_00228be8`)
//! builds the sprites on the EE, not with a VU1 program:
//!
//! * **Which instances** (level01 0x29d274.., boot 0x22906c..): an instance of a billboard class whose
//!   bounding-sphere centre is at view depth z ≥ F (F = `trunc(fade_distance) & 0xff`) and within its draw
//!   distance D is listed with a billboard alpha: `min(16·(z − F), 128)` during the cross-fade (F ≤ z < F + 8,
//!   the mesh fades out meanwhile), then `min(8·(D − z), 128)` (fades over the last 16 units before D). F = 0:
//!   always billboard (`rc_formats::shrub::shrub_fade`).
//! * **Two passes** over the list, per class in class order (0x29dae4.., boot 0x2298dc..): pass 1 draws the
//!   entries with alpha 0x80 after a GS setup (0x1c3090) of TEST_1 = 0x5360b (ATE, alpha ≥ 0x60, AFAIL =
//!   RGB_ONLY, ZTST GEQUAL) and CLAMP_1 = 5 (clamp S and T); pass 2 (0x1c3060) sets TEST_1 = 0x53001 (alpha
//!   test NEVER, AFAIL RGB_ONLY: no Z write) and draws the entries with alpha 1..0x7f. Alpha 0 entries are
//!   never drawn.
//! * **Texture**: the class record's TEX1 / TEX0 / MIPTBP1 A+D words, converted at load by the class init
//!   (boot `fun_00203b08`, `ShrubBillboard::gs_registers`) from the class entry's `ShrubBillboardInfo`: 8-bit
//!   indexed, all levels resident in gs_ram, MXL = `max_mip − 1`, MMIN = 4 (LINEAR_MIPMAP_NEAREST), K from the
//!   record (retail −207..−167 / 16). Sent once per class and pass, before its first sprite.
//! * **Quad** (0x29dcd4..0x29de2c, boot 0x229acc..0x229c24): one GIF tag, PRIM 0x7c (triangle strip, Gouraud,
//!   textured, fog, ABE), 4 × (ST, RGBAQ, XYZF2). Corners `t + y·W·(d.y, −d.x, 0) + (z·H + Z)·ẑ` with
//!   (y, z) = (−½, 1), (½, 1), (−½, 0), (½, 0) and ST (0, 0), (1, 0), (0, 1), (1, 1) (table 0x1c3130), where
//!   t = the instance origin, d = (t − eye)/|t − eye| (so the width shrinks with cos(elevation): a Z-up
//!   cylindrical billboard), and (W, H, Z) = (width·|c0,c1|, height·|c2|, z_ofs·|c2|)·scale/1024 from the
//!   loader's packed column lengths (`rc_formats::shrub::billboard_extent`). Projected with the 0x186f80 view ×
//!   projection, Q = cf10 / w (so the GS mip rule is the tfrag one, `round(log2(z / 32) + K)`), fog F per
//!   corner = clamp(w + cf14) (the tfrag fog line, per vertex).
//! * **Colour**: RGBAQ = (average of the instance's 24 lit palette colours (matrix block col1.w, written by the
//!   loader after `LightShrubs`), billboard alpha), MODULATE.
//!
//! The port: one mesh per billboard class with 4 vertices per instance (corner and instance index in a u32
//! attribute), and the vertex shader (`shrub_billboard.wgsl`) evaluates the list rule, alpha and corners every
//! frame from a static storage record. Three draws per class (crate::gs_state): pass 1 texels with As ≥ 0x60
//! (`OpaqueTested`, Z write, AlphaMask3d phase), pass 1 texels below (`ColorOnlyLowAlpha`) and pass 2
//! (`BlendNoZ`), the last two drawn before the shadows in the game's order (crate::pre_shadow; `depth_bias` orders
//! them where they stay in Transparent3d); all SrcAlpha blended with
//! As = At·alpha >> 7, as the GS ALPHA_1 (Cs − Cd)·As + Cd. `RC_GS_ALPHA=0` keeps all three in Transparent3d.
//! Not modelled: the guard-band test (an instance crossing the 4× guard band goes to the clip program 912339
//! as a mesh and is never a billboard), the 0x300-entry list limit, and ordering against other translucent draws.

use crate::gs_state::{self, GsPass};
use crate::shrub_render::{billboard_f, mip_image, LevelShrubs};
use crate::tfrag_render::game_to_bevy;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::shrub::{billboard_extent, packed_column_lengths};
use rc_formats::shrub_light::{instance_centre, PALETTE};
use std::time::Instant;

const SHADER_PATH: &str = "shaders/shrub_billboard.wgsl";

/// Per vertex: billboard record index | corner (0..3) << 24.
pub const ATTRIBUTE_BILLBOARD_INFO: MeshVertexAttribute = MeshVertexAttribute::new("ShrubBillboardInfo", 0x5348_4242, VertexFormat::Uint32);

/// Transparent3d sort bias between the three draws of a class (pass 1 Z-writing texels, the rest, pass 2).
const VARIANT_BIAS: f32 = 1e-3;

/// x = variant (0 = pass 1, As ≥ 0x60, Z write; 1 = pass 1, As < 0x60; 2 = pass 2), y = MXL, z = K, w = near (32).
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct BillboardParams {
    pub misc: Vec4,
}

/// GS draw of each variant (x of `BillboardParams`): pass 1 split at AREF 0x60 (TEST_1 = 0x5360b), pass 2
/// TEST_1 = 0x53001.
const VARIANT_PASS: [GsPass; 3] = [
    GsPass::OpaqueTested { aref: gs_state::AREF_WORLD },
    GsPass::ColorOnlyLowAlpha { aref: gs_state::AREF_WORLD },
    GsPass::BlendNoZ,
];

impl From<&BillboardMaterial> for GsPass {
    fn from(m: &BillboardMaterial) -> Self { VARIANT_PASS[m.variant as usize] }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GsPass)]
pub struct BillboardMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[storage(2, read_only)]
    /// The shared fog buffer (crate::game_camera::fog_buffer).
    pub fog: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(3)]
    pub params: BillboardParams,
    /// One `BillboardInst` per billboard instance (see shrub_billboard.wgsl).
    #[storage(4, read_only, visibility(vertex))]
    pub instances: Handle<ShaderBuffer>,
    pub variant: u8,
}

impl Material for BillboardMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode {
        if gs_state::gs_alpha_enabled() { VARIANT_PASS[self.variant as usize].alpha_mode() } else { AlphaMode::Blend }
    }
    /// Transparent3d draws ascending: pass 1 (Z-writing texels, there only with `RC_GS_ALPHA=0`), pass 1
    /// (the rest), pass 2.
    fn depth_bias(&self) -> f32 { self.variant as f32 * VARIANT_BIAS }
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        key.bind_group_data.specialize(descriptor);
        descriptor.vertex.buffers =
            vec![layout.0.get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0), ATTRIBUTE_BILLBOARD_INFO.at_shader_location(1)])?];
        Ok(())
    }
}

pub struct ShrubBillboardPlugin;

impl Plugin for ShrubBillboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<BillboardMaterial>::default()).add_systems(crate::level_switch::LevelStartup, spawn_system);
    }
}

/// Matrix-block col1.w: the loader's average of the 24 lit palette colours (`sum / 24` per channel, alpha 0).
pub fn average_colour(palette: &[[u8; 4]; PALETTE]) -> [u8; 3] {
    let sum = |k: usize| palette.iter().map(|c| c[k] as u32).sum::<u32>() / PALETTE as u32;
    [sum(0) as u8, sum(1) as u8, sum(2) as u8]
}

/// `BillboardInst` (64 bytes): game-space origin + F, Bevy-space bounding-sphere centre + D, world extent
/// (W, H, Z), packed average colour.
fn write_record(out: &mut Vec<u8>, origin: [f32; 3], f: f32, centre: Vec3, d: f32, extent: [f32; 3], colour: [u8; 3]) {
    for v in [origin[0], origin[1], origin[2], f, centre.x, centre.y, centre.z, d, extent[0], extent[1], extent[2], 0.0] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    let c = colour[0] as u32 | (colour[1] as u32) << 8 | (colour[2] as u32) << 16;
    for v in [c, 0, 0, 0] { out.extend_from_slice(&v.to_le_bytes()); }
}

fn spawn_system(
    mut commands: Commands,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<BillboardMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    spawn_billboards(&mut commands, &level.0, &mut meshes, &mut images, &mut materials, &mut buffers, None);
}

/// The billboard sprites of `level`'s shrubs, on render layer `layer` (None: the default layer; crate::title_world
/// draws the title world on its own layer).
pub(crate) fn spawn_billboards(
    commands: &mut Commands,
    level: &crate::level_load::LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<BillboardMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
    layer: Option<&bevy::camera::visibility::RenderLayers>,
) {
    if std::env::var("RC_NO_SHRUBS").is_ok_and(|v| v.trim() == "1") { return; }
    // `RC_NO_BILLBOARDS=1`: no billboard sprites (the meshes still fade out at F).
    if std::env::var("RC_NO_BILLBOARDS").is_ok_and(|v| v.trim() == "1") { return; }
    let t0 = Instant::now();
    let shrubs: &LevelShrubs = &level.shrubs;
    let sampler = ImageSamplerDescriptor {
        // CLAMP_1 = 5 from the pass-1 setup packet.
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    };

    // Pass 1: records (one storage buffer for every class), meshes and textures.
    struct ClassDraw { class: usize, o_class: i32, mesh: Handle<Mesh>, image: Handle<Image>, mxl: u32, k: f32, centroid: Vec3 }
    let mut bytes = Vec::new();
    let mut n_records = 0u32;
    let mut draws = Vec::new();
    let mut no_texture = 0;
    for (ci, c) in shrubs.classes.iter().enumerate() {
        let (Some(bb), Some(f)) = (c.class.billboard, billboard_f(c)) else { continue };
        if c.class.header.mode_bits & 1 != 0 { continue; }
        let members: Vec<usize> = (0..shrubs.instances.len()).filter(|&ii| shrubs.class_of[ii] == Some(ci)).collect();
        if members.is_empty() { continue; }
        let (Some(levels), Some(info)) = (shrubs.billboard_mips.get(&ci), c.billboard_texture()) else { no_texture += 1; continue };
        let regs = bb.gs_registers(info, 0);

        let (mut pos, mut vinfo, mut idx) = (Vec::new(), Vec::new(), Vec::new());
        let mut centroid = Vec3::ZERO;
        for &ii in &members {
            let inst = &shrubs.instances[ii];
            let t: [f32; 3] = inst.matrix[3][..3].try_into().unwrap();
            let s = instance_centre(&c.class, inst);
            let d = crate::shrub_render::runtime_draw_distance(c, inst);
            let extent = billboard_extent(&bb, c.class.header.scale, packed_column_lengths(&inst.matrix));
            write_record(&mut bytes, t, f, game_to_bevy([s[0], s[1], s[2]]), d, extent, average_colour(&shrubs.palettes[ii]));
            let origin = game_to_bevy(t);
            centroid += origin;
            let base = pos.len() as u32;
            for corner in 0..4u32 {
                pos.push(origin.to_array());
                vinfo.push(n_records | corner << 24);
            }
            // The GS strip v0 v1 v2 v3 as two triangles (no culling, so winding is irrelevant).
            idx.extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
            n_records += 1;
        }
        let mesh = meshes.add(
            Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
                .with_inserted_attribute(ATTRIBUTE_BILLBOARD_INFO, VertexAttributeValues::Uint32(vinfo))
                .with_inserted_indices(Indices::U32(idx)),
        );
        let image = images.add(mip_image(levels, sampler.clone()).0);
        let mxl = regs.mxl().min(levels.len() as u32 - 1);
        draws.push(ClassDraw { class: ci, o_class: c.o_class, mesh, image, mxl, k: regs.lod_k(), centroid: centroid / members.len() as f32 });
    }
    if n_records == 0 {
        println!("shrub billboards: none on this level");
        return;
    }

    // Pass 2: three draws per class sharing the record buffer.
    let instances = buffers.add(ShaderBuffer::new(&bytes, RenderAssetUsages::RENDER_WORLD));
    for d in &draws {
        for variant in 0..3u8 {
            let params = BillboardParams { misc: Vec4::new(variant as f32, d.mxl as f32, d.k, crate::game_camera::NEAR) };
            let mat = materials.add(BillboardMaterial { texture: d.image.clone(), fog: crate::game_camera::fog_buffer(), params, instances: instances.clone(), variant });
            let mut e = commands.spawn((
                Mesh3d(d.mesh.clone()),
                MeshMaterial3d(mat),
                // Only the Transparent3d sort reads it (the shader places every corner itself).
                Transform::from_translation(d.centroid),
                NoFrustumCulling,
                // Pass 1 (variants 0 and 1) for every class, then pass 2 (crate::pre_shadow, "Order").
                crate::pre_shadow::WorldDrawOrder([crate::pre_shadow::WorldDrawOrder::BILLBOARD, if variant == 2 { 2 } else { 1 }, d.class as u32, 0, 0]),
                Name::new(format!("shrub billboard class {} pass {}", d.o_class, variant)),
            ));
            if let Some(l) = layer { e.insert(l.clone()); }
        }
    }
    println!(
        "shrub billboards: {n_records} instances in {} classes ({no_texture} without a decoded texture), {} draws; build {:.1} ms",
        draws.len(), draws.len() * 3, t0.elapsed().as_secs_f64() * 1e3
    );
}
