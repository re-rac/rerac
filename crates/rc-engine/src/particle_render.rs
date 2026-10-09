//! Particles: the game's simulation (`rc_game::particles`, run on the 60 Hz fixed clock) and `PartProc`'s
//! sprite renderer (docs/plan/particles.md §7, level01 0x27c878, VU1 program 221571).
//!
//! **Simulation.** With the game tick (crate::gameplay, the default): the class-27 emitters (0x2bd100) are
//! moby updates run by the moby scheduler at their place in the moby order (its load pass, then every tick),
//! on the game's one `rand` stream, spawning into [`ParticleSim::sys`]; the tick's particle hook then runs
//! only `UpdateParts` ([`update_parts`]). With `RC_PLAY=0` this plugin runs the old stand-alone loop on a
//! stream of its own: at level start `srand(1234)` (level init 0x255958), pool reset, then the load-time
//! moby pass (`FUN_002792d0`: every moby's update runs once before the first frame, with no distance gate)
//! runs the class-27 emitters once; every `FixedUpdate` tick: the emitters in instance order (the moby loop;
//! the activity gate is update distance 0xff or `|pos − cam|² ≤ ud²`, all ten Novalis emitters have 0xff),
//! then `UpdateParts`. The emitters' `FastBSphereCheck` reads the camera of the previous frame's render (the game
//! builds 0x16d140 during the render), which in `FixedUpdate` is the camera `Transform` as last drawn.
//! Only Novalis (level 1) has class 27 with this update (0x2bd100 is a Novalis-only function), so no other
//! level spawns anything yet. `RC_PART_STATS=1` prints the pool once per second of ticks.
//!
//! **Pass 1 (cull, per frame on the CPU, as the EE does it).** For each live record 0..=hw: camera space
//! `c = R·(pos − cam)` (x right, y down, z forward, game units), `z12 = ftoi12(c.z)`; culled when
//! `z12 ≤ near12` (byte9 low nibble n: n << 10 = n/4 units) or `z12 ≥ min(far12, 0x1f4000)` (high nibble m:
//! m << 17 = 32·m units; global far gp−0x6a84 = 500 units, 64 underwater, from `GameFog::particle_far12`);
//! frustum: `|c.x| − r·√(1 + tx²) ≤ tx·c.z` and the same in y, r = size/420000, (tx, ty) the view's (0x16cf70:
//! the camera's `GameProjection`, which scenes and first person narrow; [`view_tans`]). Survivors go to bucket
//! `min(z12 >> 10, 1023)` (0.25 units), head-inserted, and get flag 0x20 (cleared on culled records that had
//! it). **Order**: buckets 1023 → 0 (back to front), inside a bucket the higher index first.
//! Fade: `m = min((far12 − z12) >> 4, z12 − near12, 0x1000)`, vertex alpha = `A·m >> 12` (1 unit past near,
//! the last 16 units before far).
//!
//! **Pass 2 (kind 0, camera-facing sprite).** `Q = size·[0x1607ec]/(c.z + 0.5)`, `[0x1607ec]` = 256 /
//! (tx·210000), recomputed from the view's tx by `UpdateViewContext` 0x219580 (0.63 by default): the half-diagonal in 512×416 frame-buffer pixels. θ = byte8·2π/256, a = (cos θ, 1.0625·sin θ)·Q,
//! b = (−sin θ, 1.0625·cos θ)·Q; corners S+a (ST 0,0), S+b (1,0), S−b (0,1), S−a (1,1) around the projected
//! centre S, all at the centre's Z (the vertex shader adds the pixel offsets in clip space). **Kind 1** (flat quad
//! in the world XY plane, the amoeboids' goo drips): corners `p + (c, s)`, `p + (−s, c)`, `p + (s, −c)`, `p − (c, s)`
//! with `(c, s) = (cos θ, sin θ)·size/420000`, each projected (a real 3D quad), same STs; also culled when
//! `z12 − ftoi12(r) < 0x100`. **Kinds 2 / 3** (the line and the ribbon, L3 / L4; types 19 and 55 are ribbons): pass
//! 1 takes both ends through the guard-band planes (the view ×0.9375·4) and culls when both are outside the same
//! one, near / far and the fade from the midpoint; the line is one frame-buffer pixel wide, untextured, Gouraud from
//! the two raw colours, ALPHA 0x44; the ribbon is the quad `p1 ± n̂·w1`, `p2 ± n̂·w1·k` with n̂ = unit(cross(p2 − p1,
//! midpoint − camera)) (the VU's `vopmula` / `vopmsub`; its sign only mirrors S), ST (0,0) (1,0) (0,1) (1,1), colours
//! 1, 1, 2, 2 with the faded alphas, its record's blend.
//!
//! **GS state.** TEX1 bilinear, no mips, CLAMP; MODULATE (C = Ct·Cv >> 7, A = At·Av >> 7, colours above 0x80
//! brighten up to 2×); no fog; TEST_1 = 0x5380b (A ≥ 0x80 writes Z, else RGB only; ZTST GEQUAL); ALPHA_1 per
//! record: 0x44 `(Cs − Cd)·As + Cd`, 0x48 `Cs·As + Cd` (FIX 0), both on the frame's display bytes like the GS
//! (crate::display_blend: the shader's `gs_mix` / `gs_add` against the frame snapshot, one premultiplied blend).
//! The Z rule is the two-draw pattern of `shrub_billboard.rs` / `gs_state.rs`: draw A keeps A ≥ 0x80 with Z write,
//! draw B the rest without, each over the whole sorted list of one blend group. **Order per group**: in the GS a B
//! pixel is only rejected by the A pixels of particles drawn *before* it, i.e. (back to front) by a
//! farther-bucket-or-same-bucket particle that is actually nearer; drawn A-then-B, every B pixel would also be
//! rejected by the A pixels of all nearer particles (a sprite's opaque core would hide the soft edges of every
//! particle behind it). So the **additive** group (0x48, whose result does not depend on draw order) draws B first,
//! then A: the A draw is then exactly the GS's (same order, same Z writes), and B differs only for same-bucket
//! inversions. The **normal** group (0x44) keeps A-then-B like the billboards (colour order matters there). Normal
//! group first, then additive (the GS interleaves them by depth).
//! **Display bytes.** Before 2026-09-27 only the additive group blended in display bytes (the Blarg flyers' exhaust
//! trails, Cs ≈ 0x1f: +0.12..0.2 of full scale per puff on the PS2, a few percent in linear light, had all but
//! vanished) with its own snapshot trigger; the mechanism is now crate::display_blend, shared with every effect, and
//! the 0x44 group uses it too. Its limits (overlapping effects each blend against the same snapshot) are listed
//! there.
//! **Frame position.** After mobys: the draws are Transparent3d items with a depth bias far above any scene
//! distance, so they sort after every other translucent item (and Opaque3d / AlphaMask3d run first).
//! Textures: the level's part textures as one 2D array, uploaded once (no paging emulation).
//!
//! **Buffers.** Each blend group has one persistent mesh of [`POOL_RECORDS`] quads (a vertex carries only its
//! sprite index and corner, [`quad_mesh`]) and one storage buffer of sprites ([`SpriteBuffer`]: the count, then
//! per sprite the centre, the corner vectors a and b, RGBA and the tag), rewritten each frame in draw order.
//! Quads past the count are dropped by the vertex shader. Nothing is re-inserted into `Assets<Mesh>`, so the
//! mesh allocator and material specialisation never see a change; the triangles, their order and every
//! vertex value are the ones a per-frame mesh of the same sprites would have.

use crate::game_camera::{game_eye, game_rows, TAN_HALF_FOV_X, NTSC_Y_RATIO};
use crate::tfrag_render::game_to_bevy;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor,
    SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::particle_tex::ParticleTextures;
use rc_game::particles::{rec, type06, BSphereView, Owner, Particles, FLAG_DRAWN, PART_TYPES, POOL_RECORDS};
use rc_game::rng::{Rng, LEVEL_SEED};

const SHADER_PATH: &str = "shaders/particle.wgsl";

/// Per vertex of a [`quad_mesh`]: sprite index << 2 | corner (0..3: S+a, S+b, S−b, S−a).
pub const ATTRIBUTE_SPRITE_CORNER: MeshVertexAttribute = MeshVertexAttribute::new("SpriteCorner", 0x5052_5445, VertexFormat::Uint32);

/// Bytes of the [`SpriteBuffer`] header (count, 3 × 0) and of one sprite record.
const SPRITE_HEADER: usize = 16;
const SPRITE_RECORD: usize = 48;

/// A storage buffer of sprites as `particle.wgsl` / `sky_stars.wgsl` read it: `count: vec4<u32>` (x), then
/// per sprite `p = (centre xyz, a.x)`, `q = (a.y, b.x, b.y, 0)` (f32) and `c = (rgba, tag, 0, 0)` (u32).
/// The corners are S + a (ST 0,0), S + b (1,0), S − b (0,1), S − a (1,1).
pub(crate) struct SpriteBuffer {
    bytes: Vec<u8>,
    count: u32,
    capacity: u32,
}

impl SpriteBuffer {
    pub(crate) fn new(capacity: u32) -> Self {
        SpriteBuffer { bytes: vec![0; SPRITE_HEADER + capacity as usize * SPRITE_RECORD], count: 0, capacity }
    }

    /// Starts a new frame's list.
    pub(crate) fn clear(&mut self) { self.count = 0; }

    pub(crate) fn len(&self) -> u32 { self.count }

    /// Appends one sprite (ignored past the capacity).
    pub(crate) fn push(&mut self, centre: [f32; 3], a: [f32; 2], b: [f32; 2], rgba: u32, tag: u32) {
        self.push_raw([centre[0], centre[1], centre[2], a[0]], [a[1], b[0], b[1], 0.0], [rgba, tag, 0, 0]);
    }

    /// Appends one record as the shader reads it: `p`, `q` (f32) and `c` (u32; x = RGBA, y = tag).
    pub(crate) fn push_raw(&mut self, p: [f32; 4], q: [f32; 4], c: [u32; 4]) {
        if self.count >= self.capacity { return; }
        let at = SPRITE_HEADER + self.count as usize * SPRITE_RECORD;
        for (dst, v) in self.bytes[at..at + 32].as_chunks_mut::<4>().0.iter_mut().zip(p.into_iter().chain(q)) { *dst = v.to_le_bytes(); }
        for (dst, v) in self.bytes[at + 32..at + 48].as_chunks_mut::<4>().0.iter_mut().zip(c) { *dst = v.to_le_bytes(); }
        self.count += 1;
    }

    /// The buffer contents with this frame's count.
    pub(crate) fn contents(&mut self) -> Vec<u8> {
        self.bytes[0..4].copy_from_slice(&self.count.to_le_bytes());
        self.bytes.clone()
    }

    /// A GPU buffer of this capacity (kept in the main world too: it is rewritten every frame).
    pub(crate) fn asset(&mut self) -> ShaderBuffer { ShaderBuffer::new(&self.contents(), RenderAssetUsages::default()) }
}

/// A persistent mesh of `capacity` sprite quads: vertex `4q + k` carries `q << 2 | k`, and quad q is the
/// GS strip v0 v1 v2 v3 as the triangles (0, 1, 2), (1, 3, 2).
pub(crate) fn quad_mesh(capacity: u32) -> Mesh {
    let tags: Vec<u32> = (0..capacity * 4).collect();
    let idx: Vec<u32> = (0..capacity).flat_map(|q| { let b = q * 4; [b, b + 1, b + 2, b + 1, b + 3, b + 2] }).collect();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(ATTRIBUTE_SPRITE_CORNER, VertexAttributeValues::Uint32(tags))
        .with_inserted_indices(Indices::U32(idx))
}

/// 0x1607ec = W/2 / (tan_x · 210000), with the view's tan_x (`UpdateViewContext` 0x219580 recomputes it from
/// 0x16cf70 whenever the field of view changes: scenes, first person). Game pixels: the game's own tangent: the
/// projection's tan_x without the 16:9 widening (crate::display; the shader converts game pixels across).
fn size_to_pixels(tan_x: f32) -> f32 { 256.0 / (tan_x * 210000.0) }

/// The view's (tan_x, tan_y) = 0x16cf70 / 0x16cf74: the main camera's game projection (the default field of view
/// when it has none).
pub(crate) fn view_tans(p: Option<&Projection>) -> (f32, f32) {
    match p {
        Some(Projection::Custom(c)) => c.get::<crate::game_camera::GameProjection>().map_or((TAN_HALF_FOV_X, TAN_HALF_FOV_X * NTSC_Y_RATIO), |g| (g.tan_x, g.tan_y)),
        _ => (TAN_HALF_FOV_X, TAN_HALF_FOV_X * NTSC_Y_RATIO),
    }
}
/// gp−0x6a84: global particle far, 500 units in 20.12 (before the first `GameFog`).
const GLOBAL_FAR12: i32 = rc_game::fog_zones::PARTICLE_FAR12;
/// Transparent3d sort bias: after every scene item.
const DRAW_BIAS: f32 = 1.0e6;

/// Level data for the particle system.
pub struct LevelParticles {
    pub textures: Option<ParticleTextures>,
    /// Class-27 emitters in instance order, with their update distance byte (moby+0x30).
    pub owners: Vec<(Owner, u8)>,
    pub level: u32,
    /// The level's height grid (core +0xa4; Batalia, Orxon, Oltanis): the weather particles read it.
    pub grid: Option<rc_formats::level::HeightGrid>,
}

/// Reads the particle textures and the level's emitters (class 27 on Novalis) with their pvars.
pub fn load(core: &rc_formats::level::LevelCore, index: &[u8], core_data: &[u8], gameplay: &[u8], level: u32) -> anyhow::Result<LevelParticles> {
    let textures = match rc_formats::particle_tex::parse_particle_textures(core, index, core_data) {
        Ok(t) => Some(t),
        Err(e) => {
            eprintln!("particles: no textures ({e})");
            None
        }
    };
    let mut owners = Vec::new();
    // The emitters: the instances whose class runs the emitter update 0x2bd100 in the level's class table
    // (crate::gameplay::level_ports; class 27 on Novalis, no other level has the function).
    let emitter = |oc: i32| crate::gameplay::level_ports().external(oc as i16) == Some(crate::gameplay::EMITTER_UPDATE);
    {
        let instances = rc_formats::gameplay::parse_moby_instances(gameplay)?;
        let pvars = rc_formats::gameplay::parse_pvars(gameplay)?;
        for (i, m) in instances.iter().enumerate().filter(|(_, m)| emitter(m.o_class)) {
            let Some(p) = m.pvar(&pvars).filter(|p| p.len() >= 0xe0) else { continue };
            let pos = m.position;
            owners.push((Owner { instance: i, pos: [pos[0], pos[1], pos[2], 0.0], rot: m.rotation, pvars: p.to_vec() }, m.update_distance as u8));
        }
    }
    let grid = rc_formats::level::HeightGrid::parse(&core.header, core_data);
    Ok(LevelParticles { textures, owners, level, grid })
}

/// The live simulation.
#[derive(Resource)]
pub struct ParticleSim {
    pub sys: Particles,
    /// The `RC_PLAY=0` stream (`srand(1234)`); with the game tick the game's one stream is used instead.
    pub rng: Rng,
    level: u32,
    update_distance: Vec<u8>,
    loaded: bool,
    tick: u64,
    stats: bool,
    /// The (tan_x, tan_y) of the frame last drawn (0x16cf70 / 0x16cf74): the particles' size and cull, and the
    /// emitters' `FastBSphereCheck` view of the next ticks.
    pub view_tan: (f32, f32),
    /// Last frame's pass-1 result.
    drawn: usize,
    culled: usize,
    other_kinds: u64,
    alive_sum: u64,
    /// The tick is driven by the gameplay tick (crate::gameplay: pad → mobys (the emitters) → hero →
    /// particles ([`update_parts`]) → camera); this plugin's own `FixedUpdate` system then does nothing.
    pub external: bool,
}

/// A particle draw entity of blend group `additive`.
#[derive(Component)]
struct ParticleDraw {
    additive: bool,
}

/// The sprite buffers of the two blend groups [normal (0x44), additive (0x48)] and their GPU copies.
#[derive(Resource)]
struct ParticleMesh {
    sprites: [SpriteBuffer; 2],
    buffers: [Handle<ShaderBuffer>; 2],
}

/// Pipeline key: only draw A writes Z.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParticleKey {
    z_write: bool,
}

impl From<&ParticleMaterial> for ParticleKey {
    fn from(m: &ParticleMaterial) -> Self { ParticleKey { z_write: m.draw == DRAW_A } }
}

/// [`ParticleMaterial::draw`]: A (modulated A ≥ 0x80, Z write), B (the rest).
const DRAW_A: u8 = 0;
const DRAW_B: u8 = 1;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(ParticleKey)]
pub struct ParticleMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub textures: Handle<Image>,
    /// x = draw (0 = A: modulated A ≥ 0x80, Z write; 1 = B: the rest), y = AREF (0x80).
    #[uniform(2)]
    pub params: Vec4,
    /// This group's sprites ([`SpriteBuffer`]).
    #[storage(3, read_only, visibility(vertex))]
    pub sprites: Handle<ShaderBuffer>,
    pub draw: u8,
    /// Transparent3d position among the four particle draws (normal A, normal B, additive B, additive A).
    pub slot: u8,
}

impl Material for ParticleMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { AlphaMode::Blend }
    fn depth_bias(&self) -> f32 { DRAW_BIAS + self.slot as f32 }
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(ds) = descriptor.depth_stencil.as_mut() { ds.depth_write_enabled = Some(key.bind_group_data.z_write); }
        crate::display_blend::specialize(descriptor);
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[ATTRIBUTE_SPRITE_CORNER.at_shader_location(0)])?];
        Ok(())
    }
}

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::determinism::DeterminismPlugin>() { app.add_plugins(crate::determinism::DeterminismPlugin); }
        // The game ticks at 60 Hz; the determinism plugin sets this clock only in frame-exact mode.
        if !crate::determinism::deterministic() { app.insert_resource(Time::<Fixed>::from_hz(crate::determinism::TICK_HZ)); }
        app.add_plugins(MaterialPlugin::<ParticleMaterial>::default())
            .add_systems(crate::level_switch::LevelStartup, setup)
            .add_systems(crate::level_switch::LevelUnload, (crate::level_switch::remove::<ParticleSim>, crate::level_switch::remove::<ParticleMesh>))
            .add_systems(FixedUpdate, tick)
            .add_systems(PostUpdate, build_sprites.after(crate::fog_state::FogSet));
    }
}

fn setup(
    mut commands: Commands,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
    mut shader_buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let lp = &level.0.particles;
    let defs = lp.textures.as_ref().map(|t| t.defs.clone());
    let owners: Vec<Owner> = lp.owners.iter().map(|(o, _)| o.clone()).collect();
    let mut rng = Rng::new();
    rng.srand(LEVEL_SEED);
    let mut sys = Particles::new(defs, owners);
    sys.pool.level_init();
    sys.level = lp.level;
    // The world mesh the sparks' lines test (type 25).
    sys.coll = level.0.collision.clone().map(std::sync::Arc::new);
    // The height grid the weather particles read (types 0, 73, `SpawnImpactSparks`).
    sys.grid = lp.grid.clone().map(std::sync::Arc::new);
    let stats = std::env::var("RC_PART_STATS").is_ok_and(|v| v.trim() == "1");
    println!(
        "particles: {} part textures, {} fx textures, {} class-27 emitters (instances {:?})",
        lp.textures.as_ref().map_or(0, |t| t.textures.len()),
        lp.textures.as_ref().map_or(0, |t| t.fx_textures.iter().flatten().count()),
        lp.owners.len(),
        lp.owners.iter().map(|(o, _)| o.instance).collect::<Vec<_>>()
    );
    commands.insert_resource(ParticleSim {
        sys,
        rng,
        level: lp.level,
        update_distance: lp.owners.iter().map(|(_, ud)| *ud).collect(),
        loaded: false,
        tick: 0,
        stats,
        view_tan: (TAN_HALF_FOV_X, TAN_HALF_FOV_X * NTSC_Y_RATIO),
        drawn: 0,
        culled: 0,
        other_kinds: 0,
        alive_sum: 0,
        external: false,
    });

    // Textures: one 32×32 layer per part texture, alpha back in GS units (0x80 = 1.0) like shrub_render::mip_image.
    let Some(tex) = lp.textures.as_ref().filter(|t| !t.textures.is_empty()) else { return };
    let (w, h) = (tex.textures[0].width, tex.textures[0].height);
    let mut rgba = Vec::with_capacity(tex.textures.len() * (w * h * 4) as usize);
    for t in &tex.textures {
        if (t.width, t.height) == (w, h) { rgba.extend_from_slice(&t.rgba) } else { rgba.extend(std::iter::repeat_n(0u8, (w * h * 4) as usize)) }
    }
    for a in rgba.iter_mut().skip(3).step_by(4) { *a = if *a == 0xff { 0x80 } else { *a / 2 }; }
    let mut img = Image::new_uninit(
        Extent3d { width: w, height: h, depth_or_array_layers: tex.textures.len() as u32 },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.data = Some(rgba);
    // TEX1_1 = 0xff9000000120 (bilinear, no mips), CLAMP_1 = 5 (clamp S and T).
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    let textures = images.add(img);
    // One quad per pool record: every record can be drawn, and the two groups split them.
    let mesh = meshes.add(quad_mesh(POOL_RECORDS as u32));
    let mut sprites = [SpriteBuffer::new(POOL_RECORDS as u32), SpriteBuffer::new(POOL_RECORDS as u32)];
    let buffers = [shader_buffers.add(sprites[0].asset()), shader_buffers.add(sprites[1].asset())];
    // (group, draw, slot): normal A, normal B, additive B, additive A (see the module doc).
    for (additive, draw, slot) in [(false, DRAW_A, 0u8), (false, DRAW_B, 1), (true, DRAW_B, 2), (true, DRAW_A, 3)] {
        let mat = materials.add(ParticleMaterial {
            textures: textures.clone(),
            params: Vec4::new(draw as f32, 128.0, 0.0, 0.0),
            sprites: buffers[additive as usize].clone(),
            draw,
            slot,
        });
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat),
            Transform::IDENTITY,
            NoFrustumCulling,
            Visibility::Hidden,
            ParticleDraw { additive },
            crate::display_blend::DisplayEffect,
            Name::new(format!("particles {} draw {}", if additive { "0x48" } else { "0x44" }, if draw == DRAW_A { "A (Z)" } else { "B" })),
        ));
    }
    commands.insert_resource(ParticleMesh { sprites, buffers });
}

/// The game-space camera of a Bevy transform: eye and (forward, left, up) rows.
fn camera(t: &Transform) -> ([f32; 3], [Vec3; 3]) { (game_eye(t).to_array(), game_rows(t)) }

/// The `FastBSphereCheck` view (0x16d140) of the camera as last drawn, with that frame's (tan_x, tan_y)
/// ([`ParticleSim::view_tan`]).
pub(crate) fn bsphere_view(t: &Transform, (tx, ty): (f32, f32)) -> BSphereView {
    let (eye, [f, l, u]) = camera(t);
    BSphereView::from_camera(eye, f.to_array(), l.to_array(), u.to_array(), tx, ty)
}

type MainCamera<'w, 's> = Query<'w, 's, (&'static Transform, Option<&'static Projection>), (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>;

/// One 60 Hz game tick: the emitters (moby loop), then `UpdateParts`.
/// `pub(crate)` so the sky star step (`crate::sky_stars`) can run after it on the same tick (it shares `rng`).
pub(crate) fn tick(sim: Option<ResMut<ParticleSim>>, cams: MainCamera) {
    let Some(mut sim) = sim else { return };
    if sim.external { return; }
    let t = cams.iter().next().map(|(t, _)| *t);
    step(&mut sim, t.as_ref());
}

/// The body of [`tick`] for camera transform `cam_t` (the main camera as last drawn): the `RC_PLAY=0` loop.
fn step(sim: &mut ParticleSim, cam_t: Option<&Transform>) {
    let tans = sim.view_tan;
    let view = cam_t.map(|t| bsphere_view(t, tans));
    let eye = cam_t.map(|t| camera(t).0);
    if !sim.loaded {
        // Load-time pass (FUN_002792d0): every update once, no distance gate.
        for i in 0..sim.sys.owners.len() { type06::emitter_update(&mut sim.sys, &mut sim.rng, i, view.as_ref(), sim.level); }
        sim.loaded = true;
    }
    for i in 0..sim.sys.owners.len() {
        // Moby activity gate (fun_0020d868): update distance 0xff, or within it of the camera (0x167240).
        let ud = sim.update_distance[i];
        let p = sim.sys.owners[i].pos;
        let active = ud == 0xff || eye.is_none_or(|e| (0..3).map(|k| (p[k] - e[k]).powi(2)).sum::<f32>() <= (ud as f32).powi(2));
        if active { type06::emitter_update(&mut sim.sys, &mut sim.rng, i, view.as_ref(), sim.level); }
    }
    if let Some(e) = eye { sim.sys.camera = e.map(f32::to_bits); }
    update_parts(sim, None);
}

/// `UpdateParts` 0x27c7e8 and the stats line. `rng` is the game's one `rand` stream (the type-11 sparks draw
/// from it); `None` falls back to [`ParticleSim::rng`], the `RC_PLAY=0` stream. The camera 0x167240 the sparks
/// read is [`Particles::camera`]: the caller may set it from the tick's camera, else it is the camera as last
/// drawn (written by `build_sprites`).
pub(crate) fn update_parts(sim: &mut ParticleSim, rng: Option<&mut Rng>) {
    let state = match rng {
        Some(r) => {
            sim.sys.update_parts(r);
            r.state
        }
        None => {
            sim.sys.update_parts(&mut sim.rng);
            sim.rng.state
        }
    };
    sim.tick += 1;
    sim.alive_sum += sim.sys.pool.count as u64;
    if sim.stats && sim.tick.is_multiple_of(60) {
        let s = &sim.sys.stats;
        let by_type = sim.sys.live_by_type();
        let live: Vec<String> = (0..PART_TYPES).filter(|&t| by_type[t] > 0).map(|t| format!("{t}:{}", by_type[t])).collect();
        let unported: Vec<String> = (0..PART_TYPES).filter(|&t| s.unported_kills[t] > 0).map(|t| format!("{t}:{}", s.unported_kills[t])).collect();
        println!(
            "particles: tick {} alive {} (mean {:.0} over the last 60 ticks, by type [{}]), hw {}, created {}, killed {}, pool full {}, \
             drawn {} culled {} (last frame), kinds 2-3 drawn (total) {}, unported types killed [{}], collision branch skipped {}, rng state {:#010x}",
            sim.tick, sim.sys.pool.count, sim.alive_sum as f64 / 60.0, live.join(" "), sim.sys.pool.hw, s.created, s.killed,
            s.create_failed, sim.drawn, sim.culled, sim.other_kinds, unported.join(" "), s.unported_collision, state
        );
        sim.alive_sum = 0;
    }
}

/// Pass 1 + pass 2 for this frame's camera: rewrite the sprite buffers in back-to-front order.
fn build_sprites(
    sim: Option<ResMut<ParticleSim>>,
    mesh: Option<ResMut<ParticleMesh>>,
    cams: MainCamera,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut draws: Query<(&mut Visibility, &ParticleDraw)>,
    fog: Option<Res<crate::game_camera::GameFog>>,
    display: Option<Res<crate::display::DisplaySettings>>,
) {
    let (Some(mut sim), Some(mut mesh)) = (sim, mesh) else { return };
    let Some((t, proj)) = cams.iter().next() else { return };
    let (eye, [fwd, left, up]) = camera(t);
    let (tx, ty) = view_tans(proj);
    sim.view_tan = (tx, ty);
    let size_px = size_to_pixels(tx / display.map_or(1.0, |d| crate::display::hor_scale(d.aspect)));
    // 0x167240 for the next ticks' particle updates (type 11): the camera as last drawn.
    sim.sys.camera = eye.map(f32::to_bits);
    let eye = Vec3::from_array(eye);
    let (sec_x, sec_y) = ((1.0 + tx * tx).sqrt(), (1.0 + ty * ty).sqrt());
    let n_layers = sim.sys.defs.as_ref().map_or(0, |d| d.header[1].max(0) as u32);

    struct Item { bucket: u32, index: usize, c: Vec3, alpha: u32 }
    let mut items = Vec::new();
    let (mut culled, mut other) = (0usize, 0u64);
    let hw = (sim.sys.pool.hw + 1).max(0) as usize;
    let global_far12 = fog.map_or(GLOBAL_FAR12, |f| f.particle_far12);
    for i in 0..hw {
        let r = &mut sim.sys.pool.recs[i];
        if r[1] & 0x80 != 0 { continue; }
        let kind = r[1] & 3;
        let cam_space = |p: [f32; 3]| { let d = Vec3::from_array(p) - eye; Vec3::new(-left.dot(d), -up.dot(d), fwd.dot(d)) };
        if kind > 1 {
            // Kinds 2 / 3 (line, ribbon): both ends through the guard-band planes (×0.9375·4 of the view), culled when
            // both are outside the same one; near / far and the fade from the midpoint.
            let (c1, c2) = (cam_space(rec::pos(r)), cam_space(rec::v3(r, 0x20)));
            let (gx, gy) = (tx * 3.75, ty * 3.75);
            let out = |c: Vec3| [c.x > gx * c.z, -c.x > gx * c.z, c.y > gy * c.z, -c.y > gy * c.z, c.z <= 0.0];
            let (o1, o2) = (out(c1), out(c2));
            let mid = (c1 + c2) * 0.5;
            let z12 = (mid.z * 4096.0) as i32;
            let near12 = ((r[9] & 0xf) as i32) << 10;
            let far12 = (((r[9] >> 4) as i32) << 17).min(global_far12);
            if (0..5).any(|k| o1[k] && o2[k]) || z12 <= near12 || z12 >= far12 {
                r[1] &= !FLAG_DRAWN;
                culled += 1;
                continue;
            }
            r[1] |= FLAG_DRAWN;
            other += 1;
            let m = ((far12 - z12) >> 4).min(z12 - near12).min(0x1000) as u32;
            items.push(Item { bucket: ((z12 >> 10) as u32).min(1023), index: i, c: mid, alpha: m });
            continue;
        }
        let d = Vec3::from_array(rec::pos(r)) - eye;
        let c = Vec3::new(-left.dot(d), -up.dot(d), fwd.dot(d));
        let z12 = (c.z * 4096.0) as i32;
        let near12 = ((r[9] & 0xf) as i32) << 10;
        let far12 = (((r[9] >> 4) as i32) << 17).min(global_far12);
        let size = f32::from_bits(rec::u32(r, 0xc));
        let rad = size / 420_000.0;
        let mut visible = z12 > near12 && z12 < far12 && c.x.abs() - rad * sec_x <= tx * c.z && c.y.abs() - rad * sec_y <= ty * c.z;
        // A flat quad (kind 1) is also culled when its near edge reaches the camera plane.
        if kind == 1 && z12 - ((rad * 4096.0) as i32) < 0x100 { visible = false; }
        if !visible {
            r[1] &= !FLAG_DRAWN;
            culled += 1;
            continue;
        }
        r[1] |= FLAG_DRAWN;
        let m = ((far12 - z12) >> 4).min(z12 - near12).min(0x1000) as u32;
        let alpha = (r[7] as u32 * m) >> 12;
        items.push(Item { bucket: ((z12 >> 10) as u32).min(1023), index: i, c, alpha });
    }
    // Buckets 1023 → 0; head insertion puts the later (higher) index first inside a bucket.
    items.sort_by(|a, b| b.bucket.cmp(&a.bucket).then(b.index.cmp(&a.index)));
    sim.drawn = items.len();
    sim.culled = culled;
    sim.other_kinds += other;

    let ParticleMesh { sprites: groups, buffers: handles } = &mut *mesh;
    for g in groups.iter_mut() { g.clear(); }
    for it in &items {
        let r = &sim.sys.pool.recs[it.index];
        let kind = r[1] & 3;
        if kind > 1 {
            // Kind 2 (L3): a one-pixel Gouraud line, untextured, ALPHA 0x44, the raw colours. Kind 3 (L4): the
            // textured ribbon p1 ± n̂·w1, p2 ± n̂·w1·k (n̂ ⟂ the ribbon and the view), colours 1, 1, 2, 2 with the
            // alphas faded, its record's blend. `it.alpha` holds the fade m (0..0x1000).
            let (p1, p2) = (game_to_bevy(rec::pos(r)).to_array(), game_to_bevy(rec::v3(r, 0x20)).to_array());
            let (c1, c2) = (rec::u32(r, 4), rec::u32(r, 0xc));
            let fade = |c: u32| (c & 0x00ff_ffff) | (((c >> 24) * it.alpha) >> 12).min(0xff) << 24;
            let layer = if (r[2] as u32) < n_layers { r[2] as u32 } else { 0 };
            let (additive, rgba1, rgba2) = if kind == 2 { (false, c1, c2) } else { (r[3] == 0x48, fade(c1), fade(c2)) };
            let tag = layer | (additive as u32) << 16 | (kind as u32) << 18;
            let (w1, k) = (rec::ff(r, 0x1c), rec::ff(r, 0x2c));
            groups[additive as usize].push_raw([p1[0], p1[1], p1[2], w1], [p2[0], p2[1], p2[2], k], [rgba1, tag, rgba2, 0]);
            continue;
        }
        let size = f32::from_bits(rec::u32(r, 0xc));
        let flat = r[1] & 3 == 1;
        let theta = r[8] as f32 * (std::f32::consts::TAU / 256.0);
        let (a, b) = if flat {
            // Kind 1 (L2): world corners p ± (c, s), p ± (−s, c) in the XY plane, (c, s) = (cos θ, sin θ)·size/420000.
            let k = size / 420_000.0;
            let (s, c) = (theta.sin() * k, theta.cos() * k);
            ([c, s], [-s, c])
        } else {
            let q = size * size_px / (it.c.z + 0.5);
            let (s, c) = (theta.sin() * q, theta.cos() * q);
            ([c, 1.0625 * s], [-s, 1.0625 * c])
        };
        let centre = game_to_bevy(rec::pos(r)).to_array();
        let rgba = (rec::u32(r, 4) & 0x00ff_ffff) | it.alpha.min(0xff) << 24;
        let layer = if (r[2] as u32) < n_layers { r[2] as u32 } else { 0 };
        let additive = r[3] == 0x48;
        let tag = layer | (additive as u32) << 16 | (flat as u32) << 17;
        // GS strip v0 v1 v2 v3 = S+a, S+b, S−b, S−a (quad_mesh).
        groups[additive as usize].push(centre, a, b, rgba, tag);
    }
    for (mut v, d) in &mut draws {
        let vis = if groups[d.additive as usize].len() == 0 { Visibility::Hidden } else { Visibility::Inherited };
        if *v != vis { *v = vis; }
    }
    for (g, sprites) in groups.iter_mut().enumerate() {
        if sprites.len() == 0 { continue; }
        crate::asset_write::set_buffer(&mut buffers, &handles[g], &sprites.contents());
    }
}
