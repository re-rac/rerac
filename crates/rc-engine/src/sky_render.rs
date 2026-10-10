//! The level sky, drawn as the game draws it (docs/plan/sky_render_notes.md).
//!
//! The game builds the sky's GS packets on the EE (no VU1 program): every shell in index order, every
//! cluster and face in stored order, as a plain triangle list (`rc_formats::sky::sky_gs_vertices`).
//! Per shell kind (A+D blocks 0x13d0f0 / 0x13d160, sky_render_notes.md §3):
//! * gouraud (`flags != 0`): RGBAQ = vertex colour, no texture, ZBUF ZMSK 0 (writes Z), ZTST ALWAYS;
//! * textured (`flags == 0`): MODULATE with RGBAQ = (0x80, 0x80, 0x80, vertex alpha), Q = 1, ZMSK 1, ZTST ALWAYS;
//! * both: ALPHA_1 = (Cs − Cd)·As + Cd, FGE 0 (no fog), GS Z = 0 on every vertex (the builders 0x22c0e0 /
//!   0x22c208 store the XYZ2 high word as zero), no back-face culling.
//!
//! Projection: `p = v·SkyM·C` with C = boot 0x187040 = the rotation-only view 0x186f40 (translation row
//! zero) times the world projection with rows 0/1 ×4, which the screen scale 0x18ce90·¼ cancels. So the
//! sky uses the world camera's exact projection and a view without translation; SkyM is the per-shell
//! rotation of the level's dispatch (identity or Euler about Z-up, [`level_rotations`]).
//!
//! Bevy mapping: a second `Camera3d` on [`SKY_LAYER`], ordered before the main camera, with the same
//! projection and transform; the shader drops the view translation (`w = 0`), SkyM is the entity's
//! rotation. Every draw is `AlphaMode::Blend` (SrcAlpha / OneMinusSrcAlpha = the GS equation for
//! As ≤ 0x80) in the Transparent3d phase, whose back-to-front sort is overridden by a per-draw
//! `depth_bias` so the draws run exactly in the game's order; `specialize` sets depth compare Always and
//! depth writes for gouraud shells only, and the vertex shader puts every vertex at depth 0 (= GS Z 0, the
//! far end under the game's reverse-Z mapping, game_camera.rs). The main camera then loads colour and depth
//! instead of clearing, so it draws over the sky like the rest of the GS chain.
//!
//! Output: both cameras share the view's main texture, so after the world pass it holds the whole frame. The main
//! camera writes it to the target as is (`BlendState::REPLACE`): Bevy would otherwise composite a second camera on
//! the same target with alpha blending over the first one's earlier output (the sky pass alone), and every world
//! pixel whose frame alpha is below 1 (e.g. a tie fat vertex whose VU-blended alpha lane truncates 0x80 to 0x7f,
//! or a translucent draw's `OVER` alpha) would let that stale image through: the see-through, flickering surfaces
//! of 2026-09-27. The GS frame buffer's alpha is never used for display either.
//! The sky camera writes nothing (`CameraOutputMode::Skip`): the main camera's write replaces the whole target, so
//! the sky's own copy of the main texture to it was a full-frame blit thrown away every frame.
//!
//! Frame clear: the game clears (colour = level background, Z = 0) before the sky only while sky header
//! +4 is non-zero. The loader sets it to 1, and the per-level dispatch zeroes it every frame on all levels
//! except 05, 07, 10, 13, 14, 15, so those levels clear every frame and the others only on the first
//! frame, after which the gouraud dome is the clear ([`level_zeroes_clear_flag`]).
//!
//! Star sprites (levels 00, 02, 05, 06, 07, 13, 15, 17): `crate::sky_stars` draws them in the order slot
//! [`SkyStarOrder`] that `spawn_sky` reserves between the shells the level's dispatch names
//! (`rc_game::sky_stars::level_stars`). Not ported: the per-cluster
//! bounding-sphere and per-face clip-flag rejection (GPU clipping replaces them).

use crate::game_camera;
use crate::gs_state::GsPass;
use crate::level_load::LoadedLevel;
use crate::tfrag_render::game_to_bevy;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::camera::{Camera3dDepthLoadOp, CameraOutputMode, ClearColorConfig};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendState, Extent3d, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension,
    TextureFormat,
};
use bevy::shader::ShaderRef;
use bevy::transform::TransformSystems;
use rc_formats::level::LevelCore;
use rc_formats::sky::{self, Sky, SkyTexture};
use std::f32::consts::PI;

const SHADER_PATH: &str = "shaders/sky.wgsl";

/// Render layer of the sky meshes and the sky camera.
pub const SKY_LAYER: usize = 1;

/// Transparent3d sort spacing between consecutive sky draws. The sort key is the view depth of the mesh
/// centre plus this bias; mesh centres lie within ±32768 raw units of the camera, far below the spacing.
pub const ORDER_SPACING: f32 = 1.0e6;

/// The parsed sky and its decoded textures.
pub struct LevelSky {
    pub level: u32,
    pub sky: Sky,
    pub textures: Vec<SkyTexture>,
    /// The star records' memory as loaded: `maximum_sprite_count` × 0x20 bytes at header +0x1c (empty when
    /// the block has none). Generators leave some bytes untouched (e.g. a moving star's rotation).
    pub sprite_scratch: Vec<u8>,
}

/// Parses the level's sky block (None when the core index has none; every retail level has one).
pub fn load(core: &LevelCore, core_data: &[u8], level: u32) -> Result<Option<LevelSky>> {
    let Some(block) = sky::sky_block(core, core_data).context("locating sky block")? else { return Ok(None) };
    let sky = sky::parse_sky_block(block).context("parsing sky")?;
    let textures = sky::parse_sky_textures(block, &sky).context("decoding sky textures")?;
    let h = &sky.header;
    let scratch_len = h.maximum_sprite_count.max(0) as usize * rc_game::sky_stars::RECORD;
    let sprite_scratch = match usize::try_from(h.sprites) {
        Ok(o) if o > 0 => block.get(o..o + scratch_len).map(<[u8]>::to_vec).unwrap_or_default(),
        _ => Vec::new(),
    };
    Ok(Some(LevelSky { level, sky, textures, sprite_scratch }))
}

/// Whether the level's sky dispatch zeroes header +4 (`clear_screen`) every frame, i.e. the frame is cleared
/// only on the first frame after load. Levels 05, 07, 10, 13, 14, 15 never zero it and clear every frame.
pub fn level_zeroes_clear_flag(level: u32) -> bool { !matches!(level, 5 | 7 | 10 | 13 | 14 | 15) }

/// One animated shell of a level's sky dispatch: SkyM = Rz(θ)·Ry(`ry`) (row-vector, Z applied first) with
/// θ = ((`mul`·c) mod `period`)·2π/`period` − π, c = the 60 Hz tick counter 0x15f5cc.
#[derive(Clone, Copy, Debug)]
pub struct ShellRotation {
    pub shell: usize,
    pub mul: u64,
    pub period: u64,
    pub ry: f32,
}

impl ShellRotation {
    const fn z(shell: usize, period: u64) -> Self { ShellRotation { shell, mul: 1, period, ry: 0.0 } }

    pub fn theta(&self, counter: u64) -> f32 {
        let c = self.mul.wrapping_mul(counter) % self.period;
        (c as f64 * (std::f64::consts::TAU / self.period as f64)) as f32 - PI
    }

    /// SkyM as a Bevy rotation. The Euler builder 0x1fa070 gives v' = v·Rz·Ry·Rx with counter-clockwise
    /// rotations about the game's +Z (up) and +Y; conjugating by `game_to_bevy` maps a game axis `a` to
    /// the Bevy axis `game_to_bevy(a)` with the same angle.
    pub fn rotation(&self, counter: u64) -> Quat {
        let axis = |a: [f32; 3]| game_to_bevy(a);
        Quat::from_axis_angle(axis([0.0, 1.0, 0.0]), self.ry) * Quat::from_axis_angle(axis([0.0, 0.0, 1.0]), self.theta(counter))
    }
}

/// Per-level rotating shells (all others are identity), from the level overlays' sky dispatch
/// (level01: 0x252570 → 0x29ee10 when the current level 0x15ed84 == 1; survey of levels 00–18).
pub fn level_rotations(level: u32) -> Vec<ShellRotation> {
    use ShellRotation as R;
    match level {
        0 => vec![R::z(3, 0x8000)],
        1 | 11 | 12 | 16 => {
            let s = if level == 1 { 2 } else { 1 };
            vec![R::z(s, 0x40000), R::z(s + 1, 0x20000), R::z(s + 2, 0x10000)]
        }
        3 | 4 => vec![R::z(1, 0x10000), R::z(2, 0x20000)],
        5 => vec![R::z(1, 50000)],
        7 => vec![R::z(2, 50000)],
        8 => vec![R::z(1, 40000)],
        9 => vec![ShellRotation { shell: 3, mul: 1, period: 25000, ry: 0.3 }],
        10 => vec![
            ShellRotation { shell: 0, mul: 2, period: 0x20000, ry: 0.0 },
            ShellRotation { shell: 1, mul: 5, period: 0x20000, ry: 0.0 },
            ShellRotation { shell: 2, mul: 10, period: 0x40000, ry: 0.0 },
            ShellRotation { shell: 3, mul: 10, period: 0x20000, ry: 0.0 },
        ],
        14 => vec![R::z(1, 50000), R::z(2, 100000)],
        18 => vec![R::z(4, 40000), R::z(5, 60000)],
        _ => Vec::new(),
    }
}

/// Pipeline key: gouraud shells write depth (ZMSK 0), textured ones do not (ZMSK 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SkyMaterialKey {
    textured: bool,
}

impl From<&SkyMaterial> for SkyMaterialKey {
    fn from(m: &SkyMaterial) -> Self { SkyMaterialKey { textured: m.textured } }
}

/// One sky draw: a run of consecutive faces of one shell with one texture state.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(SkyMaterialKey)]
pub struct SkyMaterial {
    /// Raw GS texel alpha (0..0x80) in the alpha channel; a 1×1 white for gouraud shells (unused there).
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    pub textured: bool,
    /// Position in the game's draw order.
    pub order: u32,
}

impl Material for SkyMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { AlphaMode::Blend }
    /// Sort key bias: Transparent3d draws ascending, so a larger bias draws later.
    fn depth_bias(&self) -> f32 { self.order as f32 * ORDER_SPACING }
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let textured = key.bind_group_data.textured;
        descriptor.primitive.cull_mode = None;
        // Depth compare Always, depth write for gouraud shells only, the GS blend (crate::gs_state).
        let pass = if textured { GsPass::SkyTextured } else { GsPass::SkyDome };
        pass.specialize(descriptor);
        if textured {
            descriptor.vertex.shader_defs.push("SKY_TEXTURED".into());
            if let Some(f) = descriptor.fragment.as_mut() { f.shader_defs.push("SKY_TEXTURED".into()); }
        }
        Ok(())
    }
}

/// The sky draw-order slot of the star step (`crate::sky_stars`), reserved by `spawn_sky` between the
/// shells its level's dispatch draws before and after it; the star draws use `depth_bias` = slot ×
/// [`ORDER_SPACING`] (+ a sub-slot per texture group).
#[derive(Resource, Clone, Copy, Debug)]
pub struct SkyStarOrder(pub u32);

/// Marks the sky camera.
#[derive(Component)]
pub struct SkyCamera;

/// A sky mesh entity of shell `shell`.
#[derive(Component)]
struct SkyShellEntity {
    shell: usize,
}

/// Rotating shells of this level and whether the rotation runs (`RC_SKY_ROT`).
#[derive(Resource)]
struct SkyAnim {
    rotations: Vec<ShellRotation>,
    /// Ticks per 60 Hz tick (`RC_SKY_ROT`: 0 freezes at the first tick, other values speed up).
    speed: f64,
    /// The level's sky dispatch zeroes the clear flag: clear on the first frame only.
    clear_first_frame_only: bool,
}

pub struct SkyRenderPlugin;

impl Plugin for SkyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<SkyMaterial>::default())
            .add_systems(crate::level_switch::LevelStartup, spawn_sky)
            .add_systems(crate::level_switch::LevelPostStartup, main_camera_loads)
            .add_systems(crate::level_switch::LevelUnload, (crate::level_switch::remove::<SkyAnim>, crate::level_switch::remove::<SkyStarOrder>))
            .add_systems(Update, (animate_shells, stop_clearing))
            .add_systems(PostUpdate, follow_main_camera.before(TransformSystems::Propagate));
    }
}

/// The counter 0x15f5cc after `ticks` 60 Hz logic ticks ([`crate::determinism::GameTicks`], never wall
/// time): reset to 0 at level init, +1 at level start, then +1 per logic tick (the main loop runs one tick
/// per frame at 60 fps and catch-up ticks at 30 fps). `speed` scales the tick rate (`RC_SKY_ROT`).
fn tick_counter(ticks: u64, speed: f64) -> u64 {
    if speed == 1.0 { 1 + ticks } else { 1 + (ticks as f64 * speed) as u64 }
}

fn spawn_sky(
    mut commands: Commands,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
) {
    let level: &LoadedLevel = &level.0;
    let Some(ls) = &level.sky else {
        println!("sky: none in this level; the frame is cleared to the level background");
        return;
    };
    let rotations = level_rotations(ls.level);
    let speed = match std::env::var("RC_SKY_ROT") { Ok(v) => v.trim().parse().unwrap_or(1.0), Err(_) => 1.0 };
    let counter = tick_counter(0, speed);
    // The star step runs after shells 0..before_shell (SkyDrawShell skips shells ≥ the header count, so a
    // step "before" a missing shell still runs after the last one).
    let star_before = rc_game::sky_stars::level_stars(ls.level).map(|s| s.before_shell);
    let s = spawn_shells(&mut commands, ls, star_before, RenderLayers::layer(SKY_LAYER), &mut meshes, &mut images, &mut materials, |e, si| {
        let rot = rotations.iter().find(|r| r.shell == si).map_or(Quat::IDENTITY, |r| r.rotation(counter));
        e.insert((Transform::from_rotation(rot), SkyShellEntity { shell: si }));
    });
    let (order, n_tris, star_order) = (s.order, s.triangles, s.star_order);
    let has_dome = ls.sky.shells.iter().any(|s| !s.textured());
    let clear_first_frame_only = level_zeroes_clear_flag(ls.level);
    commands.spawn((
        Camera3d { depth_load_op: Camera3dDepthLoadOp::Clear(0.0), ..default() },
        // First frame: the loader's clear_screen = 1 clears to the level background (the ClearColor resource).
        Camera { order: -1, clear_color: ClearColorConfig::Default, output_mode: CameraOutputMode::Skip, ..default() },
        game_camera::game_projection(),
        Tonemapping::None,
        DebandDither::Disabled,
        RenderLayers::layer(SKY_LAYER),
        SkyCamera,
        // Same MSAA as the main camera (crate::render_settings): the main pass loads this pass's depth.
        crate::render_settings::WorldCamera,
    ));
    let desc = |r: &ShellRotation| format!("shell {} period {} ×{}{}", r.shell, r.period, r.mul, if r.ry != 0.0 { format!(" Ry {}", r.ry) } else { String::new() });
    println!(
        "sky: {} shells ({} gouraud), {} draws, {} triangles, {} textures; rotating: [{}]{}; frame clear: {}",
        ls.sky.shells.len(), ls.sky.shells.iter().filter(|s| !s.textured()).count(), order, n_tris, ls.textures.len(),
        rotations.iter().map(desc).collect::<Vec<_>>().join(", "),
        if speed == 1.0 { String::new() } else { format!(" (RC_SKY_ROT speed {speed})") },
        match (clear_first_frame_only, has_dome) {
            (true, true) => "first frame only (the gouraud dome is the clear)",
            (true, false) => "first frame only (no gouraud shell: previous frames persist where the sky is transparent)",
            (false, _) => "every frame",
        }
    );
    if let Some(o) = star_order { commands.insert_resource(SkyStarOrder(o)); }
    commands.insert_resource(SkyAnim { rotations, speed, clear_first_frame_only });
}

/// What [`spawn_shells`] made.
pub(crate) struct SpawnedShells {
    /// The next draw-order slot after the shells (and the star slot).
    pub order: u32,
    pub triangles: usize,
    /// The star step's slot (`star_before`'s), if the shells reached it.
    pub star_order: Option<u32>,
    /// Every shell draw entity with its shell index.
    pub entities: Vec<(Entity, usize)>,
}

/// The shells of a sky block as draws (one per run of consecutive faces with the same texture, in shell, cluster and
/// face order) on render layer `layer`, each given its components by `extra(entity, shell)` (the shell's SkyM
/// `Transform`, a marker). `star_before`: the shell the star step's draw slot comes before. Used by the level sky
/// ([`spawn_sky`]) and the space skies the camera of [`SKY_LAYER`] is switched to (the flight between planets,
/// crate::flight_render; the title world, crate::title_world).
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_shells(
    commands: &mut Commands,
    ls: &LevelSky,
    star_before: Option<usize>,
    layer: RenderLayers,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<SkyMaterial>,
    mut extra: impl FnMut(&mut bevy::ecs::system::EntityCommands, usize),
) -> SpawnedShells {
    let white = images.add(Image::new_fill(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255, 255, 255, 0x80],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let tex_images: Vec<Handle<Image>> = ls.textures.iter().map(|t| images.add(sky_image(t))).collect();
    let (mut order, mut n_tris) = (0u32, 0usize);
    let mut star_order = None;
    let mut entities = Vec::new();
    for (si, shell) in ls.sky.shells.iter().enumerate() {
        if star_before == Some(si) {
            star_order = Some(order);
            order += 1;
        }
        // Runs of consecutive faces with the same texture, in cluster and face order: one draw each.
        let mut runs: Vec<(u8, Vec<sky::SkyGsVertex>)> = Vec::new();
        for c in &shell.clusters {
            let verts = sky::sky_gs_vertices(shell, c);
            for tri in verts.as_chunks::<3>().0 {
                match runs.last_mut() {
                    Some((t, v)) if *t == tri[0].texture => v.extend_from_slice(tri),
                    _ => runs.push((tri[0].texture, tri.to_vec())),
                }
            }
        }
        for (tex, verts) in runs {
            n_tris += verts.len() / 3;
            let textured = shell.textured();
            let image = if textured {
                match ls.textures.iter().position(|t| t.index == tex as usize) {
                    Some(i) => tex_images[i].clone(),
                    None => { warn!("sky shell {si}: texture {tex} not decoded; drawing white"); white.clone() }
                }
            } else { white.clone() };
            let positions: Vec<[f32; 3]> =
                verts.iter().map(|v| game_to_bevy(v.position.map(|x| x as f32)).to_array()).collect();
            let uvs: Vec<[f32; 2]> = verts.iter().map(|v| v.st).collect();
            // Raw GS RGBA bytes; the shader applies the GS rules per shell kind.
            let colors: Vec<[f32; 4]> = verts.iter().map(|v| v.rgba.map(|c| c as f32)).collect();
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            let mut e = commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(SkyMaterial { texture: image, textured, order })),
                layer.clone(),
                NoFrustumCulling,
                Name::new(format!("sky shell {si} tex {tex}")),
            ));
            extra(&mut e, si);
            entities.push((e.id(), si));
            order += 1;
        }
    }
    if star_before.is_some() && star_order.is_none() { star_order = Some(order); }
    SpawnedShells { order, triangles: n_tris, star_order, entities }
}

/// A space sky shell's SkyM (`DrawSkyShells` 0x29f260): the Euler rotation (x, y, z) (the builder 0x1fa070:
/// `v·Rz·Ry·Rx`), the rows scaled by `scale`, the translation row `t` (sky units, game axes). The shader applies the
/// whole matrix to `(x, y, z, 1)` as the game's 0x22bf94 does.
pub(crate) fn shell_transform(euler: [f32; 3], scale: f32, t: [f32; 3]) -> Transform {
    let axis = |a: [f32; 3]| game_to_bevy(a);
    let rot = Quat::from_axis_angle(axis([1.0, 0.0, 0.0]), euler[0])
        * Quat::from_axis_angle(axis([0.0, 1.0, 0.0]), euler[1])
        * Quat::from_axis_angle(axis([0.0, 0.0, 1.0]), euler[2]);
    Transform { translation: game_to_bevy(t), rotation: rot, scale: Vec3::splat(scale) }
}

/// Decoded sky texture as the GS sees it: display-encoded RGB, raw texel alpha 0..0x80.
pub(crate) fn sky_image(t: &SkyTexture) -> Image {
    let mut rgba = t.texture.rgba.clone();
    // Undo `texture::scale_alpha` (a < 0x80 → 2a, a ≥ 0x80 → 0xff): exact for a ≤ 0x80, which holds for
    // every texel the Novalis sky uses.
    for a in rgba.iter_mut().skip(3).step_by(4) { *a = if *a == 0xff { 0x80 } else { *a / 2 }; }
    let mut img = Image::new(
        Extent3d { width: t.texture.width, height: t.texture.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    // TEX1: bilinear mag/min, no mips; CLAMP_1: clamp on both axes.
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    });
    img
}

/// The world pass continues the GS chain after the sky: no colour clear, depth loaded from the sky pass, and the
/// finished frame (sky + world, one shared main texture) replaces the target instead of being alpha-blended over the
/// sky camera's output (module doc, "Output").
fn main_camera_loads(
    sky_cam: Query<(), With<SkyCamera>>,
    mut cams: Query<(&mut Camera, &mut Camera3d), Without<SkyCamera>>,
) {
    if sky_cam.is_empty() { return; }
    for (mut cam, mut c3d) in &mut cams {
        cam.clear_color = ClearColorConfig::None;
        cam.output_mode = CameraOutputMode::Write { blend_state: Some(BlendState::REPLACE), clear_color: ClearColorConfig::None };
        c3d.depth_load_op = Camera3dDepthLoadOp::Load;
    }
}

/// After the first frame, levels whose dispatch zeroes the clear flag stop clearing.
fn stop_clearing(anim: Option<Res<SkyAnim>>, mut cams: Query<&mut Camera, With<SkyCamera>>, mut frames: Local<u32>, generation: Res<crate::level_switch::LevelGeneration>) {
    // A runtime level change (crate::level_switch): the new level's first frame clears again.
    if generation.is_changed() { *frames = 0; }
    let Some(anim) = anim else { return };
    *frames += 1;
    if *frames != 2 || !anim.clear_first_frame_only { return; }
    for mut cam in &mut cams { cam.clear_color = ClearColorConfig::None; }
}

/// SkyM per shell from the 60 Hz tick counter.
fn animate_shells(
    anim: Option<Res<SkyAnim>>,
    ticks: Res<crate::determinism::GameTicks>,
    mut shells: Query<(&SkyShellEntity, &mut Transform)>,
) {
    let Some(anim) = anim else { return };
    if anim.rotations.is_empty() || anim.speed == 0.0 { return; }
    let counter = tick_counter(ticks.0, anim.speed);
    for (s, mut t) in &mut shells {
        if let Some(r) = anim.rotations.iter().find(|r| r.shell == s.shell) { t.rotation = r.rotation(counter); }
    }
}

type MainView<'w, 's> = Query<'w, 's, (&'static Transform, &'static Projection), (With<crate::fly_cam::FlyCam>, Without<SkyCamera>)>;
type SkyView<'w, 's> = Query<'w, 's, (&'static mut Transform, &'static mut Projection), (With<SkyCamera>, Without<crate::fly_cam::FlyCam>)>;

/// The sky camera shares the main camera's transform and **projection** (the game draws the sky with the world
/// camera's own projection, 0x187040 = the rotation-only view times the world projection; the shader removes the
/// translation). The projection follows every change of tan(hfov/2) 0x16cf70: a scene camera record's (0.414 or 0.554
/// on Novalis, gameplay 0.63), so the sky turns exactly with the world instead of sliding against it.
pub(crate) fn follow_main_camera(main: MainView, mut sky: SkyView) {
    let Some((src, proj)) = main.iter().next() else { return };
    let tan = |p: &Projection| match p {
        Projection::Custom(c) => c.get::<game_camera::GameProjection>().map(|g| (g.tan_x, g.tan_y)),
        _ => None,
    };
    let want = tan(proj);
    for (mut t, mut p) in &mut sky {
        if *t != *src { *t = *src; }
        if want.is_some() && tan(&p) != want {
            if let (Some((x, y)), Projection::Custom(c)) = (want, &mut *p) {
                if let Some(g) = c.get_mut::<game_camera::GameProjection>() {
                    g.tan_x = x;
                    g.tan_y = y;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// θ = (c & mask)·2π/(mask + 1) − π, as level01 0x29ee10 computes it for shell 4 (mask 0xffff).
    #[test]
    fn theta_matches_dispatch() {
        let r = ShellRotation::z(4, 0x10000);
        assert!((r.theta(0) + PI).abs() < 1e-6);
        assert!(r.theta(0x8000).abs() < 1e-6);
        assert!((r.theta(0x10000 + 0x4000) - (-PI / 2.0)).abs() < 1e-6);
        // level01's constant for shell 4 is 9.58738e-05 = 2π/65536 per tick.
        assert!(((r.theta(1000) - r.theta(0)) - 1000.0 * 9.58738e-5).abs() < 1e-5);
    }

    /// Rz(θ) row-vector form: rows (c, s, 0), (−s, c, 0): game +X turns toward +Y (counter-clockwise about up).
    #[test]
    fn rotation_is_ccw_about_game_up() {
        let r = ShellRotation::z(0, 4);
        let q = r.rotation(3); // θ = 3·π/2 − π = π/2
        let x = q * game_to_bevy([1.0, 0.0, 0.0]);
        assert!((x - game_to_bevy([0.0, 1.0, 0.0])).length() < 1e-6, "{x}");
    }
}
