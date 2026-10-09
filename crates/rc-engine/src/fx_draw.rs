//! Draw-callback effects: the GS primitives the game sends straight from a moby's draw callback
//! (`RegisterDrawCallback` 0x21afe0, list 1, drawn after the mobys and before the particles; list 2 after the
//! particles; the registrations are `rc_game::moby_update::classes::draw_callbacks`). One material for all of them
//! ([`FxPrimMaterial`], `fx_prim.wgsl`: `FastDrawQuadReal` quads and `DrawEnvOverlayMesh` strips with an FX texture,
//! MODULATE, fog, ALPHA 0x44 / 0x48 on the frame's display bytes through crate::display_blend) and one vertex builder
//! ([`PrimBuf`]); the fire / smoke fields 760 (crate::water_render, list 2) and the callbacks here use them.
//!
//! **The nanotech glow** (class 806, `0x301c00`, list 1; [`nanotech_prims`]). The cluster registers it every tick it
//! is in view (`rc_game::moby_update::classes::pickup`). With p = the cluster's position + (0, 0, 0.5 + bob) and the
//! camera-facing basis F = unit(camera − p), R = unit(U_crate × F), U = F × R (U_crate = the crate's third rotation
//! row), it draws, TEST_1 0x53001 (Z tested, never written), bilinear, CLAMP repeat:
//! 1. **the sphere**: a 290-vertex hemisphere of radius 0.3 (table `verts`, x ≥ 0) turned to face the camera
//!    (`F·x + R·y + U·z + p`), three tri-strips (vertices 0..148, 146..255, 254..290), per-vertex ST and RGBA
//!    0x50804040 (a violet tint at alpha 0x50), FX 21 (a cloudy blue map), ALPHA from the gp words (0x44): the dark
//!    translucent ball;
//! 2. **the halo**: 32 quads of a ring (inner corners at 0.3 × the unit circle, outer at 0.3·1.125 × 1.3), FX 11 (a
//!    radial glow) along s = 0.5 from t = 0.5 inside to t = 1 outside, colour 0x802020 with alpha
//!    `trunc(bob/0.06·64 + 128)` (64..192, the bob's pulse) on the inner corners and 0 outside, ALPHA 0x48: the
//!    blue-violet glow around the ball;
//! 3. **on the crate only** (state 1): the crate glass's sheen, four quads of the crate box (±0.5, z 0.124..0.891) in
//!    the crate's frame, FX 21 at `0x107f7f7f` (alpha 0x10), ALPHA 0x44, sphere-mapped: per corner the view vector
//!    scaled to length 2, `s = x/2 + 0.5`, `t = z/2 + 0.5` (each `fmod 8`).
//!
//! The tables are the overlay's (read per level through the function's relocations, [`NanotechTables::parse`]); the
//! glowing dotted rings around the ball are not this callback: they are the orbs' type-62 trail particles.
//!
//! **The ship glass** (`0x2a70a8`, the boot's `0x2327a0`; [`ship_glass_prims`]): the canopy of the ships 530..533,
//! registered by the cutscene FX driver on list 1; the callback takes `MobyAttachToJoint(ship, 0, M)` **when it draws**,
//! i.e. on the pose the ship is drawn with this frame (a scene actor's pose is written after the moby loop, so the
//! registering tick's matrix is one pose behind: the glass then trailed the approaching ship and fell behind its
//! cockpit and crew; [`crate::scene_render::drawn_actor`]). M is the world matrix of the
//! last joint of the class's **joint list 0** (the ship's cockpit joint: list `[0, 1, 2, 3, 6]` on 530; the second
//! argument is a joint-list index, `MobyMarkJointChain` 0x268d80 reads class header +0x1c). Per class `class −
//! 0x212` the gp arrays (level01 gp −0x6620 colour, −0x6610 point count, −0x6600 quad count, −0x65f0 normals,
//! −0x65e0 points, −0x65d0 quads; [`ShipGlassTables::parse`]) give points and normals `(x, y, z, 1)` in that joint's
//! frame and quads of four point indices (16-byte records, a short every 4 bytes). Each point goes through the full
//! matrix (`M·(x, y, z, w)`, `0x221608`); so does its normal, **w = 1 included** (the joint's translation is added
//! to it: a quirk kept), then scaled to length 0.1; with `e` = unit(point − camera) the reflection `r = unit(e −
//! 2(n·e)n)` gives the sphere map `s = r.x/m + ½`, `t = r.y/m + ½`, `m = 2·√(2(r.z + 1))`. Drawn as `FastDrawQuadReal`
//! quads, FX 0x15 (the nanotech crate glass's texture), the class colour on all four corners (530: 0x50807060),
//! TEX1 bilinear, ALPHA 0x44. The near / far state ([`GlassFade`], 2026-10-02): outside scenes (game mode 0) the ST
//! follows the camera only while it is within 16 of the ship in x and y; farther, it stays as it was, and coming back
//! it fades from that frozen ST to the live one over `ticks(60)` (level 00's ship 530 draws through its own copy
//! `0x2d19b0`, the same tables and draw, which tests the distance in every game mode). The mode-6 space scenes with
//! `0x13e050 == 4` (`crate::flight_render`) use FX 1 instead of 0x15 and never draw live: the first draw's ST stays.
//!
//! **The glow quad** (level01 `0x2781d0`, [`glow_quad`]): the engine's shared soft-glow billboard, called by class draw
//! callbacks and the hero's glow drawer: the vendor's four glow points (`0x2ba9c0`), the mouse 1818's glow sprites
//! (`0x30de68`) and `0x229440` (six calls). Arguments: size (f12), a pull toward the camera (f13), the point (a0) and
//! one RGBA for the four corners (a1). With F = unit(camera 0x167240 − point), R = F × (0, 0, 1) normalised and
//! U = R × F, the point moves pull·F toward the camera and the corners are `point + size·(y·R + z·U)` for the table
//! at 0x1b08f0, (y, z) = (−1, 1), (−1, −1), (1, 1), (1, −1), ST (0, 0), (0, 1), (1, 0), (1, 1); one
//! `FastDrawQuadReal` with FX 0xb (the radial glow), CLAMP_1 5, TEX1 bilinear, **ALPHA 0x48** (additive, on display
//! bytes like every callback draw here).
//!
//! **`DrawSpriteHelper_A` 0x21e340** (the boot's `fun_001f76a0`), which the sprite-drawing callbacks call first: the
//! set-up of the VU1 billboard program 7 (view·projection 0x167140, the guard band 0x1671c0 with −camera·1024, the
//! fog words, the GIF tags); it keeps no game state. Here the material does the same for every prim (the camera, the
//! fog), so a callback's port only lists its prims (G-REN-020, 2026-09-29).
//!
//! **The census units' quad callbacks** (`Callback::UnitQuads`): the `FastDrawQuadReal` quads
//! `rc_game::moby_update::classes::units::fx_quads` lists (the laser fences 838's `0x30de90`: ten bars per lit fence
//! of the group, FX 0x13, additive).
//!
//! **The census units' glow callbacks** (`Callback::UnitGlow`): the glow quads `rc_game::moby_update::classes::units::
//! glow_quads` lists (the lamps 1060's `0x2df3d8`: one quad of size 1.2 / pull 0.5 on each visible lamp of the group).
//!
//! **The vendor's glow points** (class 11's callback `0x2ba9c0`, list 2; [`vendor_glow_points`]): after its beam
//! (drawn by crate::vendor_render), four glow quads of size 0.1333 (0x3e087fcc), no pull, at the antenna tips
//! `rows · (1.1·cos a, 1.1·sin a, 0.59) + position` for a = k·π/2 − π (k = 0..3), coloured `moby+0x90 & 0xffff0000`:
//! the blue byte and the alpha of the vendor's pulsing glow word (0x2bb058..0x2bb0d4).

use crate::game_camera::{game_eye, GameFog, TfragFog};
use anyhow::Result;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;
use rc_formats::water as wf;
use rc_game::moby_update::classes::draw_callbacks::Callback;

const SHADER_PATH: &str = "shaders/fx_prim.wgsl";
/// Transparent3d sort bias of the list-1 callback draws: after the mobys, before the particles (1e6).
pub const LIST1_BIAS: f32 = 9.0e5;
/// The list-2 callback draws here (after the particles and the fire fields' band 2e6).
pub const LIST2_BIAS: f32 = 3.0e6;

/// Static uniform of one draw group.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct FxPrimParams {
    /// x = 1: ALPHA 0x48 (additive `Cs·As + Cd`), 0: ALPHA 0x44 (`(Cs − Cd)·As + Cd`). y = 1: the subtractive
    /// `Cd − Cs·As` (ALPHA 0x62 with As = FIX: [`FxPrimParams::subtract`]; a pipeline key). w = 1: an opaque world
    /// surface ([`FxPrimParams::opaque`]; the pipeline key).
    pub misc: Vec4,
}

impl FxPrimParams {
    pub fn blend(additive: bool) -> Self { FxPrimParams { misc: Vec4::new(additive as u32 as f32, 0.0, 0.0, 0.0) } }

    /// A strip the GS writes with Z and a FIX of 0x80 (`(Cs − Cd)·0x80 + Cd` = Cs, e.g. TEST 0x5360a without the alpha
    /// test): drawn in the main pass as an opaque world surface (no blend, Z written, display bytes converted to linear
    /// like the world shaders), not as an effect (crate::sea_render).
    pub fn opaque() -> Self { FxPrimParams { misc: Vec4::new(0.0, 0.0, 0.0, 1.0) } }

    /// ALPHA 0x62 (`(0 − Cs)·FIX + Cd`) with As = FIX through the vertex alpha: the colour taken off the frame.
    pub fn subtract() -> Self { FxPrimParams { misc: Vec4::new(0.0, 1.0, 0.0, 0.0) } }

    fn is_opaque(&self) -> bool { self.misc.w > 0.5 }
    fn is_subtract(&self) -> bool { self.misc.y > 0.5 }
}

/// The pipeline key: an effect (display blend, no Z) or an opaque world surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FxPrimKey {
    opaque: bool,
    subtract: bool,
}

impl From<&FxPrimMaterial> for FxPrimKey {
    fn from(m: &FxPrimMaterial) -> Self { FxPrimKey { opaque: m.params.is_opaque(), subtract: m.params.is_subtract() } }
}

/// One draw group of a callback: an FX texture, the fog, one of the two ALPHA equations, its Transparent3d order.
/// Drawn by crate::display_blend's effect pass (the entity carries `DisplayEffect`).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(FxPrimKey)]
pub struct FxPrimMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub fog: TfragFog,
    #[uniform(3)]
    pub params: FxPrimParams,
    /// Transparent3d sort bias (the callback list's band + the draw's position in it).
    pub order: f32,
}

impl Material for FxPrimMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { if self.params.is_opaque() { AlphaMode::Opaque } else { AlphaMode::Blend } }
    fn depth_bias(&self) -> f32 { self.order }
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if key.bind_group_data.opaque {
            crate::gs_state::GsPass::Opaque.specialize(descriptor);
            if let Some(f) = descriptor.fragment.as_mut() { f.shader_defs.push("FX_OPAQUE".into()); }
        } else {
            // TEST_1 0x53001 / 0x51001 (ATST NEVER, AFAIL FB_ONLY: colour, never Z; ZTST GEQUAL).
            crate::gs_state::GsPass::BlendNoZ.specialize(descriptor);
            crate::display_blend::specialize(descriptor);
            if key.bind_group_data.subtract { crate::display_blend::specialize_subtract(descriptor); }
        }
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?];
        Ok(())
    }
}

/// An FX texture (`GetEffectTex`) as an image of raw GS bytes; TEX1 0xff9000000260 (bilinear, no mips), CLAMP 0
/// (repeat). The decoded texture's alpha is scaled to 0..0xff (`rc_formats::texture::scale_alpha`); it goes back to
/// GS units (0x80 = 1.0) here, as for the particle textures (`particle_render`), since `fx_prim.wgsl` multiplies the
/// texel alpha by the vertex alpha like the GS (`At·Av >> 7`).
pub fn fx_image(images: &mut Assets<Image>, t: &rc_formats::texture::Texture) -> Handle<Image> {
    let mut img = Image::new_uninit(
        Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    let mut rgba = t.rgba.clone();
    for a in rgba.iter_mut().skip(3).step_by(4) { *a = if *a == 0xff { 0x80 } else { *a / 2 }; }
    img.data = Some(rgba);
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    images.add(img)
}

/// Mesh data of one draw group being built: game-space points, (s, t), GS RGBA bytes.
#[derive(Default)]
pub struct PrimBuf {
    pos: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    color: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

impl PrimBuf {
    fn vertex(&mut self, p: [f32; 3], st: [f32; 2], c: u32) -> u32 {
        let b = self.pos.len() as u32;
        self.pos.push(crate::tfrag_render::game_to_bevy(p).to_array());
        self.uv.push(st);
        self.color.push([(c & 0xff) as f32, (c >> 8 & 0xff) as f32, (c >> 16 & 0xff) as f32, (c >> 24) as f32].map(|x| x / 128.0));
        b
    }

    /// One `FastDrawQuadReal` quad: corners in GS strip order (triangles 0 1 2, 1 2 3). A quad whose four alphas are
    /// 0 adds nothing under either equation and is skipped.
    pub fn quad(&mut self, p: [[f32; 3]; 4], st: [[f32; 2]; 4], rgba: [u32; 4]) {
        if rgba.iter().all(|c| c >> 24 == 0) { return; }
        let b = self.pos.len() as u32;
        for k in 0..4 { self.vertex(p[k], st[k], rgba[k]); }
        self.idx.extend([b, b + 1, b + 2, b + 1, b + 2, b + 3]);
    }

    /// One tri-strip (`DrawEnvOverlayMesh`): triangle k = vertices k, k + 1, k + 2.
    pub fn strip(&mut self, v: impl IntoIterator<Item = ([f32; 3], [f32; 2], u32)>) {
        let b = self.pos.len() as u32;
        for (p, st, c) in v { self.vertex(p, st, c); }
        let n = self.pos.len() as u32 - b;
        for k in 0..n.saturating_sub(2) { self.idx.extend([b + k, b + k + 1, b + k + 2]); }
    }

    pub fn is_empty(&self) -> bool { self.idx.is_empty() }

    /// Whether `mesh` already holds these primitives (crate::asset_write: a rewrite is an upload).
    fn same_as(&self, mesh: &Mesh) -> bool {
        use bevy::mesh::VertexAttributeValues as V;
        matches!(mesh.try_attribute_option(Mesh::ATTRIBUTE_POSITION), Ok(Some(V::Float32x3(v))) if *v == self.pos)
            && matches!(mesh.try_attribute_option(Mesh::ATTRIBUTE_UV_0), Ok(Some(V::Float32x2(v))) if *v == self.uv)
            && matches!(mesh.try_attribute_option(Mesh::ATTRIBUTE_COLOR), Ok(Some(V::Float32x4(v))) if *v == self.color)
            && matches!(mesh.try_indices_option(), Ok(Some(Indices::U32(v))) if *v == self.idx)
    }

    /// Writes into mesh `h` unless it already holds these primitives.
    pub fn update(self, meshes: &mut Assets<Mesh>, h: &Handle<Mesh>) {
        if meshes.get(h).is_some_and(|m| self.same_as(m)) { return; }
        if let Some(mut m) = meshes.get_mut(h) { self.write(&mut m); }
    }

    pub fn write(self, mesh: &mut Mesh) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.color);
        mesh.insert_indices(Indices::U32(self.idx));
    }
}

/// One draw of a callback this tick: FX texture, equation and primitives.
pub struct FxGroup {
    pub fx: usize,
    pub additive: bool,
    /// [`FxPrimParams::subtract`] (over `additive`).
    pub subtract: bool,
    pub prims: PrimBuf,
}

/// One slot's entity, mesh, material and whether it is shown.
type FxSlot = (Entity, Handle<Mesh>, Handle<FxPrimMaterial>, bool);

/// The persistent entities of the callback draws: slot k = the k-th draw of the list this tick (Bevy's mesh
/// allocator keeps no slab for an empty mesh, so an entity is made when its slot first has primitives).
#[derive(Default)]
pub struct FxSlots {
    slots: Vec<Option<FxSlot>>,
    fx: Vec<Option<Handle<Image>>>,
    /// The render layer of the slots' entities (None: the default layer; the flight's draws are on the transition's
    /// layer, crate::flight_render).
    pub layer: Option<bevy::camera::visibility::RenderLayers>,
}

/// The assets the slots need.
pub struct FxAssets<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub images: &'a mut Assets<Image>,
    pub materials: &'a mut Assets<FxPrimMaterial>,
    pub fx: Option<&'a [Option<rc_formats::texture::Texture>]>,
    pub fog: TfragFog,
}

impl FxSlots {
    /// FX image `i`, made on first use.
    fn image(&mut self, a: &mut FxAssets, i: usize) -> Option<Handle<Image>> {
        let t = a.fx?.get(i)?.as_ref()?;
        if self.fx.len() <= i { self.fx.resize(i + 1, None); }
        if self.fx[i].is_none() { self.fx[i] = Some(fx_image(a.images, t)); }
        self.fx[i].clone()
    }

    /// Shows `groups` in slots 0.. (bias `band + slot`) and hides the rest.
    pub fn show(&mut self, commands: &mut Commands, vis: &mut Query<&mut Visibility>, a: &mut FxAssets, groups: Vec<FxGroup>, band: f32, name: &str) {
        let n = groups.len();
        for (k, g) in groups.into_iter().enumerate() {
            let img = self.image(a, g.fx);
            if self.slots.len() <= k { self.slots.push(None); }
            let show = !g.prims.is_empty() && img.is_some();
            if show {
                let img = img.expect("checked");
                let params = if g.subtract { FxPrimParams::subtract() } else { FxPrimParams::blend(g.additive) };
                match &mut self.slots[k] {
                    Some((_, mesh, mat, _)) => {
                        g.prims.update(a.meshes, mesh);
                        let need = a.materials.get(&*mat).is_none_or(|m| m.texture != img || m.params.misc != params.misc || m.fog != a.fog);
                        if need {
                            if let Some(mut m) = a.materials.get_mut(&*mat) {
                                m.texture = img;
                                m.params = params;
                                m.fog = a.fog;
                            }
                        }
                    }
                    None => {
                        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
                        g.prims.write(&mut m);
                        let mesh = a.meshes.add(m);
                        let mat = a.materials.add(FxPrimMaterial { texture: img, fog: a.fog, params, order: band + k as f32 });
                        let e = commands
                            .spawn((
                                Mesh3d(mesh.clone()),
                                MeshMaterial3d(mat.clone()),
                                Transform::IDENTITY,
                                NoFrustumCulling,
                                Visibility::Inherited,
                                crate::display_blend::DisplayEffect,
                                Name::new(format!("{name} draw {k}")),
                            ))
                            .id();
                        if let Some(l) = &self.layer { commands.entity(e).insert(l.clone()); }
                        self.slots[k] = Some((e, mesh, mat, true));
                        continue;
                    }
                }
            }
            set_visible(vis, &mut self.slots[k], show);
        }
        for s in self.slots.iter_mut().skip(n) { set_visible(vis, s, false); }
    }
}

fn set_visible(vis: &mut Query<&mut Visibility>, s: &mut Option<FxSlot>, show: bool) {
    let Some((e, _, _, shown)) = s else { return };
    if *shown == show { return; }
    *shown = show;
    if let Ok(mut v) = vis.get_mut(*e) { *v = if show { Visibility::Inherited } else { Visibility::Hidden }; }
}

// ---------------------------------------------------------------------------------------------------
// Level tables

/// The draw-callback tables of the level's overlay.
#[derive(Clone, Debug, Default)]
pub struct LevelFx {
    pub nanotech: Option<NanotechTables>,
    pub ship_glass: Option<ShipGlassTables>,
    /// Level 18's pool meshes (`rc_game` `units::veldin_pool`; handed to the moby loop at the level load).
    pub veldin_pools: Option<std::sync::Arc<rc_game::moby_update::classes::units::veldin_pool::Meshes>>,
    /// Level 6's glass meshes (`rc_game` `units::blarg_glass`).
    pub blarg_glass: Option<std::sync::Arc<rc_game::moby_update::classes::units::blarg_glass::Meshes>>,
    /// The gadgetbot bubble mesh of levels 6 and 10 (`rc_game` `units::blarg_gadgetbot`).
    pub blarg_bubble: Option<std::sync::Arc<rc_game::moby_update::classes::units::blarg_gadgetbot::Bubble>>,
    /// Quartu's and the Fleet's water strips (`rc_game` `units::wave_mesh`).
    pub wave_meshes: Option<std::sync::Arc<rc_game::moby_update::classes::units::wave_mesh::Meshes>>,
}

impl LevelFx {
    pub fn parse(ov: &wf::Overlay, level: u32) -> LevelFx {
        let nanotech = NanotechTables::parse(ov, level).unwrap_or_else(|e| {
            eprintln!("fx: nanotech glow tables: {e}");
            None
        });
        let ship_glass = ShipGlassTables::parse(ov, level).unwrap_or_else(|e| {
            eprintln!("fx: ship glass tables: {e}");
            None
        });
        let veldin_pools = rc_game::moby_update::classes::units::veldin_pool::Meshes::parse(ov, level);
        if level == rc_game::moby_update::classes::units::veldin_pool::REFERENCE_LEVEL && veldin_pools.is_none() { eprintln!("fx: the pool meshes of level 18 did not read"); }
        let blarg_glass = rc_game::moby_update::classes::units::blarg_glass::Meshes::parse(ov, level);
        if level == rc_game::moby_update::classes::units::blarg_glass::REFERENCE_LEVEL && blarg_glass.is_none() { eprintln!("fx: the glass meshes of level 6 did not read"); }
        let blarg_bubble = rc_game::moby_update::classes::units::blarg_gadgetbot::Bubble::parse(ov, level);
        if rc_game::moby_update::classes::units::blarg_gadgetbot::Bubble::on_level(level) && blarg_bubble.is_none() { eprintln!("fx: the gadgetbot bubble mesh of level {level} did not read"); }
        let wave_meshes = rc_game::moby_update::classes::units::wave_mesh::Meshes::parse(ov, level);
        if rc_game::moby_update::classes::units::wave_mesh::DEFS.iter().any(|d| d.level == level) && wave_meshes.is_none() { eprintln!("fx: the water strips of level {level} did not read"); }
        LevelFx { nanotech, ship_glass, veldin_pools, blarg_glass, blarg_bubble, wave_meshes }
    }
}

/// `0x2a70a8` per level (the one function on all 19 levels; `tools/ghidra/names/clusters.tsv`, the boot's `0x2327a0`).
const SHIP_GLASS_FN: [(u32, u32); 19] = [
    (0, 0x29_35b8), (1, 0x2a_70a8), (2, 0x29_2d28), (3, 0x27_ff00), (4, 0x28_44b0), (5, 0x2b_bd08), (6, 0x2a_06b0),
    (7, 0x2b_a260), (8, 0x29_b940), (9, 0x2a_ff08), (10, 0x28_40b8), (11, 0x2b_64b0), (12, 0x2a_9e40), (13, 0x29_d9c8),
    (14, 0x29_9960), (15, 0x28_1500), (16, 0x28_c490), (17, 0x28_b818), (18, 0x29_3160),
];
/// Its size in bytes.
const SHIP_GLASS_SIZE: u32 = 1344;
/// The first ship class (`class − 0x212` indexes the tables) and the number of ships.
const SHIP_GLASS_CLASS0: i16 = 0x212;
const SHIP_GLASS_SHIPS: usize = 4;
/// `GetEffectTex(0x15)`.
const SHIP_GLASS_FX: usize = 0x15;

/// One ship's canopy (module doc).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShipGlass {
    pub rgba: u32,
    pub pts: Vec<[f32; 4]>,
    pub normals: Vec<[f32; 4]>,
    pub quads: Vec<[u16; 4]>,
}

/// The ship glass tables of a level, by `class − 0x212`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShipGlassTables {
    pub ships: Vec<Option<ShipGlass>>,
}

impl ShipGlassTables {
    /// Reads the six gp arrays through the function's `addiu r, gp, imm` references, in code order (quads,
    /// points, normals, point counts, quad counts, colours on level 01).
    pub fn parse(ov: &wf::Overlay, level: u32) -> Result<Option<ShipGlassTables>> {
        let Some(&(_, f)) = SHIP_GLASS_FN.iter().find(|t| t.0 == level) else { return Ok(None) };
        let mut gp_refs = Vec::new();
        for a in (f..f + SHIP_GLASS_SIZE).step_by(4) {
            let w = ov.u32(a)?;
            if w >> 26 == 0x09 && (w >> 21 & 31) == 28 { gp_refs.push(GP.wrapping_add_signed((w & 0xffff) as u16 as i16 as i32)); }
        }
        if gp_refs.len() < 6 { anyhow::bail!("0x{f:x}: {} gp address references, expected 6", gp_refs.len()); }
        let (quads_a, pts_a, nrm_a, npts_a, nq_a, col_a) = (gp_refs[0], gp_refs[1], gp_refs[2], gp_refs[3], gp_refs[4], gp_refs[5]);
        let v4 = |a: u32| -> Result<[f32; 4]> { Ok([ov.f32(a)?, ov.f32(a + 4)?, ov.f32(a + 8)?, ov.f32(a + 12)?]) };
        let one = |i: u32| -> Result<ShipGlass> {
            let (np, nq) = (ov.i32(npts_a + 4 * i)?, ov.i32(nq_a + 4 * i)?);
            if !(0..=256).contains(&np) || !(0..=256).contains(&nq) { anyhow::bail!("ship {i}: {np} points, {nq} quads"); }
            let (pa, na, qa) = (ov.u32(pts_a + 4 * i)?, ov.u32(nrm_a + 4 * i)?, ov.u32(quads_a + 4 * i)?);
            let pts = (0..np as u32).map(|k| v4(pa + 16 * k)).collect::<Result<Vec<_>>>()?;
            let normals = (0..np as u32).map(|k| v4(na + 16 * k)).collect::<Result<Vec<_>>>()?;
            let quads = (0..nq as u32)
                .map(|q| -> Result<[u16; 4]> {
                    let b = ov.read(qa + 16 * q, 16)?;
                    Ok(std::array::from_fn(|k| u16::from_le_bytes([b[4 * k], b[4 * k + 1]])))
                })
                .collect::<Result<Vec<_>>>()?;
            if quads.iter().flatten().any(|&k| k as i32 >= np) { anyhow::bail!("ship {i}: a quad index past {np} points"); }
            Ok(ShipGlass { rgba: ov.u32(col_a + 4 * i)?, pts, normals, quads })
        };
        Ok(Some(ShipGlassTables { ships: (0..SHIP_GLASS_SHIPS as u32).map(|i| one(i).ok()).collect() }))
    }

    /// The glass of `o_class` (None: not a ship with glass tables on this level).
    pub fn of(&self, o_class: i16) -> Option<&ShipGlass> { usize::try_from(o_class - SHIP_GLASS_CLASS0).ok().and_then(|i| self.ships.get(i)?.as_ref()) }
}

/// `0x301c00` per level (the one function, hash-identical in the 19 overlays; `tools/ghidra/names/clusters.tsv`).
const NANOTECH_GLOW_FN: [(u32, u32); 19] = [
    (0, 0x2d_9810), (1, 0x30_1c00), (2, 0x2e_4280), (3, 0x2d_2b20), (4, 0x2d_b9a0), (5, 0x30_b820), (6, 0x2e_80c0),
    (7, 0x30_4180), (8, 0x2f_b390), (9, 0x2f_aac0), (10, 0x2c_bc00), (11, 0x30_1f58), (12, 0x2f_9a70), (13, 0x2f_c650),
    (14, 0x2f_5b70), (15, 0x2d_d048), (16, 0x2d_83c0), (17, 0x2d_b4f0), (18, 0x2e_39a0),
];
/// Its size in bytes.
const NANOTECH_GLOW_SIZE: u32 = 1464;
/// The level overlays' gp.
const GP: u32 = 0x16_6c00;

/// A data reference of a function, in instruction order: a `lui`/`addiu` address, a gp-relative load, a
/// `lui`-based load. Identical code in every overlay gives the same list with each level's own addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reloc {
    Addr(u32),
    Gp(u32),
    Mem(u32),
}

fn relocs(ov: &wf::Overlay, start: u32, size: u32) -> Result<Vec<Reloc>> {
    let mut hi = [None::<u32>; 32];
    let mut out = Vec::new();
    for a in (start..start + size).step_by(4) {
        let w = ov.u32(a)?;
        let (op, rs, rt) = (w >> 26, (w >> 21 & 31) as usize, (w >> 16 & 31) as usize);
        let simm = (w & 0xffff) as u16 as i16 as i32;
        match op {
            0x0f => hi[rt] = Some((w & 0xffff) << 16),
            0x09 => {
                if let Some(h) = hi[rs] { out.push(Reloc::Addr(h.wrapping_add_signed(simm))); }
            }
            // Loads and stores (lb .. lw, sb .. sw, lwc1, swc1, ld, sd).
            0x20 | 0x21 | 0x23 | 0x24 | 0x25 | 0x2b | 0x31 | 0x37 | 0x39 | 0x3f => {
                if rs == 28 {
                    out.push(Reloc::Gp(GP.wrapping_add_signed(simm)));
                } else if let Some(h) = hi[rs] {
                    out.push(Reloc::Mem(h.wrapping_add_signed(simm)));
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// The nanotech glow's tables and constants (module doc).
#[derive(Clone, Debug, PartialEq)]
pub struct NanotechTables {
    /// The hemisphere: 290 × (x, y, z) (radius 0.3, x ≥ 0 toward the camera), ST and RGBA per vertex.
    pub verts: Vec<[f32; 3]>,
    pub st: Vec<[f32; 2]>,
    pub rgba: Vec<u32>,
    /// The sphere's FX texture (gp word) and its ALPHA (A, B, C, D, FIX gp words; 0x44 on every level).
    pub sphere_fx: u32,
    pub sphere_additive: bool,
    /// The halo: 96 points (32 inner of length 1, 64 outer of length 1.3), 32 quads of 4 point indices, the scale
    /// (0.3) and the outer corners' extra factor (1.125), the colour (RGB), the alpha offset `0x161e80`.
    pub halo_pts: Vec<[f32; 3]>,
    pub halo_quads: Vec<[u16; 4]>,
    pub halo_scale: f32,
    pub halo_outer: f32,
    pub halo_rgb: u32,
    pub halo_alpha_add: i32,
    /// The bob's amplitude (0.06): the pulse is bob / this.
    pub bob_amp: f32,
    /// The crate glass: 16 points, 4 quads, its FX texture, the view vector's length for the sphere map (2).
    pub glass_pts: Vec<[f32; 3]>,
    pub glass_quads: Vec<[u16; 4]>,
    pub glass_fx: u32,
    pub glass_env: f32,
    /// A vector added to the centre (`0x161e60`, zero on Novalis).
    pub offset: [f32; 3],
}

/// The halo's FX texture (`GetEffectTex(0xb)`, a literal), the glass colour, the sphere map's wrap.
const HALO_FX: usize = 0xb;
const GLASS_RGBA: u32 = 0x107f_7f7f;
const GLASS_WRAP: f32 = 8.0;

impl NanotechTables {
    /// Reads the tables through the function's references (their order is the code's, the same on every level).
    pub fn parse(ov: &wf::Overlay, level: u32) -> Result<Option<NanotechTables>> {
        let Some(&(_, f)) = NANOTECH_GLOW_FN.iter().find(|t| t.0 == level) else { return Ok(None) };
        let r = relocs(ov, f, NANOTECH_GLOW_SIZE)?;
        if r.len() != 47 { anyhow::bail!("0x{f:x}: {} data references, expected 47", r.len()); }
        let addr = |i: usize| -> Result<u32> {
            match r[i] {
                Reloc::Addr(a) | Reloc::Gp(a) | Reloc::Mem(a) => Ok(a),
            }
        };
        let (fv, uv) = (|a: u32| -> Result<f32> { Ok(ov.f32(a)?) }, |a: u32| -> Result<u32> { Ok(ov.u32(a)?) });
        let v3 = |a: u32| -> Result<[f32; 3]> { Ok([fv(a)?, fv(a + 4)?, fv(a + 8)?]) };
        let shorts = |a: u32, n: usize| -> Result<Vec<[u16; 4]>> {
            (0..n).map(|q| -> Result<[u16; 4]> { let b = ov.read(a + 8 * q as u32, 8)?; Ok(std::array::from_fn(|k| u16::from_le_bytes([b[2 * k], b[2 * k + 1]]))) }).collect()
        };
        // Reference ordinals (level01 0x301c00): 0 gp bob amplitude, 2 gp sphere FX, 3..7 gp ALPHA B, A, C, D, FIX,
        // 8 the centre offset, 13 the vertices, 15 RGBA, 16 ST, 21 the alpha offset, 22 gp halo RGB, 23 halo quads,
        // 24 halo points, 26 / 27 gp halo scale / outer factor, 31 gp glass FX, 36 glass points, 38 glass quads, 43 gp
        // the sphere map's length.
        let (verts_a, rgba_a, st_a) = (addr(13)?, addr(15)?, addr(16)?);
        let n = 290u32;
        let verts = (0..n).map(|i| v3(verts_a + 16 * i)).collect::<Result<Vec<_>>>()?;
        let rgba = (0..n).map(|i| uv(rgba_a + 4 * i)).collect::<Result<Vec<_>>>()?;
        let st = (0..n).map(|i| -> Result<[f32; 2]> { Ok([fv(st_a + 8 * i)?, fv(st_a + 8 * i + 4)?]) }).collect::<Result<Vec<_>>>()?;
        let (a_b, a_a, a_c, a_d) = (uv(addr(3)?)?, uv(addr(4)?)?, uv(addr(5)?)?, uv(addr(6)?)?);
        let sphere_additive = match (a_a, a_b, a_c, a_d) {
            (0, 1, 0, 1) => false,
            (0, 2, 0, 1) => true,
            other => anyhow::bail!("sphere ALPHA {other:?} is neither 0x44 nor 0x48"),
        };
        let halo_quads = shorts(addr(23)?, 32)?;
        let glass_quads = shorts(addr(38)?, 4)?;
        let (hp, gp_) = (addr(24)?, addr(36)?);
        let halo_n = halo_quads.iter().flatten().map(|&i| i as u32 + 1).max().unwrap_or(0);
        let glass_n = glass_quads.iter().flatten().map(|&i| i as u32 + 1).max().unwrap_or(0);
        Ok(Some(NanotechTables {
            verts,
            st,
            rgba,
            sphere_fx: uv(addr(2)?)?,
            sphere_additive,
            halo_pts: (0..halo_n).map(|i| v3(hp + 16 * i)).collect::<Result<Vec<_>>>()?,
            halo_quads,
            halo_scale: fv(addr(26)?)?,
            halo_outer: fv(addr(27)?)?,
            halo_rgb: uv(addr(22)?)? & 0xff_ffff,
            halo_alpha_add: uv(addr(21)?)? as i32,
            bob_amp: fv(addr(0)?)?,
            glass_pts: (0..glass_n).map(|i| v3(gp_ + 16 * i)).collect::<Result<Vec<_>>>()?,
            glass_quads,
            glass_fx: uv(addr(31)?)?,
            glass_env: fv(addr(43)?)?,
            offset: v3(addr(8)?)?,
        }))
    }
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn scale3(a: [f32; 3], s: f32) -> [f32; 3] { [a[0] * s, a[1] * s, a[2] * s] }
/// `FastVecCross(out, a, b)` 0x2212d0: b × a.
fn cross_ba(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [b[1] * a[2] - b[2] * a[1], b[2] * a[0] - b[0] * a[2], b[0] * a[1] - b[1] * a[0]] }
fn unit3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l == 0.0 { v } else { scale3(v, 1.0 / l) }
}
/// `M·(x, y, z, 1)` with rows `m[0..3]` and translation `t`.
fn xform(m: &[[f32; 3]; 3], t: [f32; 3], v: [f32; 3]) -> [f32; 3] { add3(add3(add3(scale3(m[0], v[0]), scale3(m[1], v[1])), scale3(m[2], v[2])), t) }
/// `FUN_00222170(v, m)`: v − m·trunc(v/m) (VU0 `vdiv`, `ftoi0`, `itof0`).
fn wrap(v: f32, m: f32) -> f32 { v - m * (v / m).trunc() }

/// The three draws of callback `0x301c00` for one cluster (module doc).
pub fn nanotech_prims(t: &NanotechTables, g: &rc_game::moby_update::classes::pickup::NanotechGlow, cam: [f32; 3]) -> Vec<FxGroup> {
    let p = add3([g.pos[0], g.pos[1], g.pos[2] + 0.5 + g.bob], t.offset);
    let f = unit3(sub3(cam, p));
    let r = unit3(cross_ba(f, g.crate_rows[2]));
    let u = cross_ba(r, f);
    let basis = [f, r, u];
    let mut out = Vec::with_capacity(3);
    // 1. The sphere: three strips over the turned hemisphere.
    let mut sphere = PrimBuf::default();
    for (a, b) in [(0usize, 0x94usize), (146, 146 + 0x6d), (254, 254 + 0x24)] {
        sphere.strip((a..b.min(t.verts.len())).map(|i| (xform(&basis, p, t.verts[i]), t.st[i], t.rgba[i])));
    }
    out.push(FxGroup { fx: t.sphere_fx as usize, additive: t.sphere_additive, subtract: false, prims: sphere });
    // 2. The halo ring, pulsing with the bob.
    let pulse = g.bob / t.bob_amp;
    let a = ((pulse * 64.0 + 128.0) as i32).wrapping_add(t.halo_alpha_add) as u32;
    let c = a << 24 | t.halo_rgb;
    let mut halo = PrimBuf::default();
    for q in &t.halo_quads {
        let pts: [[f32; 3]; 4] = std::array::from_fn(|k| {
            let s = if k < 2 { t.halo_scale } else { t.halo_scale * t.halo_outer };
            xform(&basis, p, scale3(t.halo_pts.get(q[k] as usize).copied().unwrap_or_default(), s))
        });
        halo.quad(pts, [[0.5, 0.5], [0.5, 0.5], [0.5, 1.0], [0.5, 1.0]], [c, c, 0, 0]);
    }
    out.push(FxGroup { fx: HALO_FX, additive: true, subtract: false, prims: halo });
    // 3. The crate glass's sheen, while on the crate.
    let mut glass = PrimBuf::default();
    if g.on_crate {
        let at = [p[0], p[1], g.pos[2]];
        for q in &t.glass_quads {
            let pts: [[f32; 3]; 4] = std::array::from_fn(|k| xform(&g.crate_rows, at, t.glass_pts.get(q[k] as usize).copied().unwrap_or_default()));
            let st = pts.map(|w| {
                let e = sub3(w, cam);
                let l = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt();
                let e = if l == 0.0 { e } else { scale3(e, t.glass_env / l) };
                [wrap(e[0] * 0.5 + 0.5, GLASS_WRAP), wrap(e[2] * 0.5 + 0.5, GLASS_WRAP)]
            });
            glass.quad(pts, st, [GLASS_RGBA; 4]);
        }
    }
    out.push(FxGroup { fx: t.glass_fx as usize, additive: false, subtract: false, prims: glass });
    out
}

/// The glow quad's corner table at 0x1b08f0 ((y, z); x = 0) and its ST.
const GLOW_CORNERS: [[f32; 2]; 4] = [[-1.0, 1.0], [-1.0, -1.0], [1.0, 1.0], [1.0, -1.0]];
const GLOW_ST: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]];
/// The glow quad's texture: FX 0xb (`GetEffectTex(0xb)`), the radial glow the nanotech halo uses too.
pub const GLOW_FX: usize = HALO_FX;

/// One glow quad (`0x2781d0`, module doc) into `b`: `size`, `pull` toward the camera `cam`, at `point`, RGBA `rgba`
/// (GS bytes, R low). Draw `b` as an additive FX [`GLOW_FX`] group.
pub fn glow_quad(b: &mut PrimBuf, size: f32, pull: f32, point: [f32; 3], rgba: u32, cam: [f32; 3]) {
    let f = unit3(sub3(cam, point));
    let r = unit3(cross_ba([0.0, 0.0, 1.0], f));
    let u = cross_ba(f, r);
    let p = add3(point, scale3(f, pull));
    let pts = GLOW_CORNERS.map(|[y, z]| add3(p, add3(scale3(r, y * size), scale3(u, z * size))));
    b.quad(pts, GLOW_ST, [rgba; 4]);
}

/// The vendor callback's four glow points (module doc) for the vendor moby's rotation rows, position and glow word.
pub fn vendor_glow_points(rows: &[[f32; 3]; 3], pos: [f32; 3], glow: u32, cam: [f32; 3]) -> FxGroup {
    let mut b = PrimBuf::default();
    let rgba = glow & 0xffff_0000;
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::PI * 0.5 - std::f32::consts::PI;
        let p = xform(rows, pos, [a.cos() * 1.1, a.sin() * 1.1, 0.59]);
        glow_quad(&mut b, f32::from_bits(0x3e08_7fcc), 0.0, p, rgba, cam);
    }
    FxGroup { fx: GLOW_FX, additive: true, subtract: false, prims: b }
}

/// `M·(x, y, z, w)`: rows 0..2 the axes, row 3 the point (`0x221608`).
fn apply4(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 3] { std::array::from_fn(|k| m[0][k] * v[0] + m[1][k] * v[1] + m[2][k] * v[2] + m[3][k] * v[3]) }

/// The sphere-map ST of the ship glass at world point `w` with the transformed normal `n` (module doc).
fn ship_glass_st(w: [f32; 3], n: [f32; 3], cam: [f32; 3]) -> [f32; 2] {
    let e = unit3(sub3(w, cam));
    let n = scale3(unit3(n), 0.1);
    let d = n[0] * e[0] + n[1] * e[1] + n[2] * e[2];
    let r = unit3(sub3(e, scale3(n, d + d)));
    let m = ((r[2] + 1.0) * 2.0).sqrt() * 2.0;
    [r[0] / m + 0.5, r[1] / m + 0.5]
}

/// The ship glass's near / far state (the callback's globals: level01 gp −0x669c the first-draw state, −0x6698 the
/// "near" byte (level00's 530: moby +0xbc), −0x6694 the timer (530: pvar +0x40), the current ST 0x1c0c40 and the
/// frozen ST 0x1c0f70), kept per ship moby.
#[derive(Default)]
pub struct GlassFade {
    started: bool,
    near: bool,
    timer: i32,
    st: Vec<[f32; 2]>,
    frozen: Vec<[f32; 2]>,
}

/// The draw of callback `0x2a70a8` (level00's 530: `0x2d19b0`, the same draw) for one ship (module doc): `m` = the
/// matrix of its joint list 0. `live`: the ST follows the camera this draw (a scene, or the camera within 16 of the
/// ship in x and y); otherwise the ST stays as it was, and coming back it fades from that frozen ST to the live one
/// over `ticks(60)` (`fade`, the callback's state).
pub fn ship_glass_prims(g: &ShipGlass, m: &[[f32; 4]; 4], cam: [f32; 3], live: bool, fade: &mut GlassFade) -> FxGroup {
    let world: Vec<[f32; 3]> = g.pts.iter().map(|&p| apply4(m, p)).collect();
    let n = g.pts.len();
    if fade.st.len() != n { fade.st = vec![[0.0; 2]; n]; }
    if fade.frozen.len() != n { fade.frozen = vec![[0.0; 2]; n]; }
    let t60 = rc_game::hero::physics::ticks(60);
    if live || !fade.started {
        fade.near = true;
        if fade.timer > 0 { fade.timer -= 1; }
        let k = fade.timer as f32 / t60 as f32;
        for (i, (&w, &nm)) in world.iter().zip(&g.normals).enumerate() {
            let l = ship_glass_st(w, apply4(m, nm), cam);
            fade.st[i] = if !fade.started || fade.timer == 0 { l } else { [l[0] + (fade.frozen[i][0] - l[0]) * k, l[1] + (fade.frozen[i][1] - l[1]) * k] };
        }
        fade.started = true;
    } else {
        if fade.near {
            fade.near = false;
            fade.frozen.clone_from(&fade.st);
        }
        fade.timer = t60;
    }
    let st = &fade.st;
    let mut prims = PrimBuf::default();
    for q in &g.quads {
        let k = q.map(|i| i as usize);
        prims.quad(k.map(|i| world[i]), k.map(|i| st[i]), [g.rgba; 4]);
    }
    FxGroup { fx: SHIP_GLASS_FX, additive: false, subtract: false, prims }
}

// ---------------------------------------------------------------------------------------------------
// Plugin

pub struct FxDrawPlugin;

impl Plugin for FxDrawPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FxPrimMaterial>::default())
            .init_resource::<FxDraw>()
            // A runtime level change (crate::level_switch): the slots' entities are gone.
            .add_systems(crate::level_switch::LevelUnload, crate::level_switch::reset::<FxDraw>)
            .add_systems(PostUpdate, draw_list1.before(bevy::asset::AssetEventSystems));
    }
}

/// The list-1 callback draws' state.
#[derive(Resource, Default)]
struct FxDraw {
    slots: FxSlots,
    slots2: FxSlots,
    /// The tick counter the last draw used (redrawn once per tick, like the other callbacks).
    drawn: Option<u64>,
    /// The ship glass's near / far state per ship moby ([`GlassFade`]).
    glass: std::collections::HashMap<usize, GlassFade>,
}

type MainCamera<'w, 's> = Query<'w, 's, &'static Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>;

/// The callbacks of this tick that draw here (the nanotech glow, the ship glass, the vendor's glow points), per list in
/// registration order.
#[allow(clippy::too_many_arguments)]
fn draw_list1(
    mut commands: Commands,
    mut state: ResMut<FxDraw>,
    play: Option<Res<crate::gameplay::Play>>,
    cams: MainCamera,
    fog: Option<Res<GameFog>>,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FxPrimMaterial>>,
    mut vis: Query<&mut Visibility>,
    scene: Option<Res<crate::scene_render::SceneRuntime>>,
    attach: Option<Res<crate::moby_attach::MobyAttach>>,
) {
    let Some(cam_t) = cams.iter().next() else { return };
    let counter = play.as_deref().map(|p| p.game.counter);
    let fog = fog.map(|f| f.uniform).unwrap_or_else(|| TfragFog::new(&level.0.fog));
    if counter == state.drawn && counter.is_some() { return; }
    state.drawn = counter;
    let cam = game_eye(cam_t).to_array();
    let (mut g1, mut g2) = (Vec::new(), Vec::new());
    if let Some(p) = play.as_deref() {
        let cbs = &p.svc.draw_callbacks;
        for (list, out) in [(&cbs.list1, &mut g1), (&cbs.list2, &mut g2)] {
            for &(cb, id) in list {
                match cb {
                    Callback::NanotechGlow => {
                        let (Some(t), Some(g)) = (level.0.water.fx.nanotech.as_ref(), rc_game::moby_update::classes::pickup::nanotech_glow(&p.game.mobys, id)) else { continue };
                        out.extend(nanotech_prims(t, &g, cam));
                    }
                    Callback::ShipGlass => {
                        let (Some(t), Some(mo)) = (level.0.water.fx.ship_glass.as_ref(), p.game.mobys.mobys.get(id)) else { continue };
                        let Some(g) = t.of(mo.o_class) else { continue };
                        // `MobyAttachToJoint(ship, 0, M)` at draw time: the pose the ship is drawn with this frame (a scene
                        // actor's, set after the tick's moby loop), else the matrix the registration took.
                        let drawn = scene.as_deref().and_then(|rt| crate::scene_render::drawn_actor(rt, id)).map(|a| a.joint_matrix(0));
                        let Some(m) = drawn.or_else(|| cbs.matrices.get(&id).copied()) else { continue };
                        // Live: a scene's actor; the camera (0x167240) within 16 of the ship in x and y; level 01's code
                        // (not level 00's 530 copy) also in any game mode but 0.
                        let near = (cam[0] - mo.position[0]).abs() < 16.0 && (cam[1] - mo.position[1]).abs() < 16.0;
                        let live = drawn.is_some() || near || (p.svc.level != 0 && p.svc.game_mode != 0);
                        out.push(ship_glass_prims(g, &m, cam, live, state.glass.entry(id).or_default()));
                    }
                    // The glow points (the beam: crate::vendor_render).
                    Callback::VendorBeam => {
                        let Some(mo) = p.game.mobys.mobys.get(id) else { continue };
                        let rows = [0, 1, 2].map(|i| [mo.rows[i][0], mo.rows[i][1], mo.rows[i][2]]);
                        out.push(vendor_glow_points(&rows, [mo.position[0], mo.position[1], mo.position[2]], mo.glow, cam));
                    }
                    // The Thruster-Pack's flames (crate::thruster_render).
                    Callback::ThrusterFlame => out.extend(crate::thruster_render::flame_groups(&p.game, id, cam)),
                    // A census unit port's glow quads (the lamps 1060, …).
                    Callback::UnitGlow(i) => {
                        let quads = rc_game::moby_update::classes::units::glow_quads(&p.game.mobys, &p.svc, i, id);
                        if quads.is_empty() { continue; }
                        let mut b = PrimBuf::default();
                        for q in quads { glow_quad(&mut b, q.size, q.pull, q.point, q.rgba, cam); }
                        out.push(FxGroup { fx: GLOW_FX, additive: true, subtract: false, prims: b });
                    }
                    // A census unit port's quads (the laser fences 838, …).
                    Callback::UnitQuads(i) => {
                        for g in rc_game::moby_update::classes::units::fx_quad_groups(&p.game.mobys, &p.svc, i, id) {
                            if g.quads.is_empty() { continue; }
                            let mut b = PrimBuf::default();
                            for q in g.quads { b.quad(q.corners, q.st, q.rgba); }
                            out.push(FxGroup { fx: g.fx, additive: g.additive, subtract: g.subtract, prims: b });
                        }
                    }
                    // The ship's shadow (`0x2a2130`), flames (`0x2a2ab8`) and trail (`0x2a2d28`): rc_game::travel::ship's quads.
                    Callback::ShipShadow | Callback::ShipFlames | Callback::ShipTrail => {
                        let g = match cb {
                            Callback::ShipShadow => cbs.ship.shadow.get(&id).cloned(),
                            Callback::ShipFlames => cbs.ship.flames.get(&id).cloned(),
                            _ => {
                                let tr = &p.svc.travel;
                                let flight = p.svc.game_mode == 6 && tr.sub == rc_game::travel::space::SUB_FLIGHT;
                                Some(rc_game::travel::ship::trail_quads(&tr.trail, rc_game::travel::ship_index(tr.ship), flight))
                            }
                        };
                        let Some(g) = g.filter(|g| !g.quads.is_empty()) else { continue };
                        let mut b = PrimBuf::default();
                        for q in g.quads { b.quad(q.corners, q.st, q.rgba); }
                        out.push(FxGroup { fx: g.fx, additive: g.additive, subtract: g.subtract, prims: b });
                    }
                    // Drawn by crate::water_render / crate::sea_render; the Walloper's arcs by crate::walloper_render; the
                    // range static by crate::visibomb_view.
                    // The Metal Detector's scan (`0x2f1e28`): rc_game's buried_bolts::Scan squares, FX 8, additive.
                    Callback::DetectorScan => {
                        let s = &p.svc.buried.scan;
                        if s.quads.is_empty() { continue; }
                        let mut b = PrimBuf::default();
                        for (q, rgba) in &s.quads { b.quad(*q, rc_game::moby_update::classes::buried_bolts::SCAN_ST, [*rgba; 4]); }
                        out.push(FxGroup { fx: rc_game::moby_update::classes::buried_bolts::SCAN_FX, additive: true, subtract: false, prims: b });
                    }
                    // The disguise's glow quads (`0x229440` body 3: `0x2781d0(0.2, 0.08, point, 0x141634)`).
                    Callback::DisguiseGlow => {
                        let Some((pts, rgba)) = cbs.disguise else { continue };
                        let mut b = PrimBuf::default();
                        for pt in pts { glow_quad(&mut b, 0.2, 0.08, pt, rgba, cam); }
                        out.push(FxGroup { fx: GLOW_FX, additive: true, subtract: false, prims: b });
                    }
                    // The hero's draw callback `0x229440` (rc_game::hero::glow): in the scene / space modes 2 / 6 (0x15f5c4)
                    // the glow list only, whatever the body (the dot's point is the hidden back Clank's: it floated over a
                    // cutscene's set); else on foot the antenna dot then the glow list, in bodies 1 / 2 their dot; each
                    // sprite where its item sits as last placed (crate::moby_attach).
                    Callback::HeroGlow => {
                        let g = &p.game.hero.glow;
                        let at = |a: rc_game::hero::glow::At| match a {
                            rc_game::hero::glow::At::Point(pt) => Some(pt),
                            other => attach.as_deref().and_then(|m| m.glow_point(other)),
                        };
                        let mut b = PrimBuf::default();
                        let scene = matches!(p.svc.game_mode, 2 | 6);
                        let list: &[rc_game::hero::glow::Sprite] = if scene || p.game.hero.mode == 0 { &g.list } else { &[] };
                        let dot = if scene { None } else { g.dot.as_ref() };
                        for s in dot.into_iter().chain(list) {
                            if let Some(pt) = at(s.at) { glow_quad(&mut b, s.size, s.pull, pt, s.rgba, cam); }
                        }
                        out.push(FxGroup { fx: GLOW_FX, additive: true, subtract: false, prims: b });
                    }
                    // The Trespasser lock's minigame is 2-D: crate::scene_render draws it.
                    Callback::TrespasserRings => {}
                    Callback::FireField760 | Callback::RipplePatches | Callback::Sea(_) | Callback::Walloper | Callback::RangeStatic | Callback::UnitFrame(_) => {}
                }
            }
        }
    }
    let tex = level.0.particles.textures.as_ref().map(|t| t.fx_textures.as_slice());
    let mut a = FxAssets { meshes: &mut meshes, images: &mut images, materials: &mut materials, fx: tex, fog };
    let st = &mut *state;
    st.slots.show(&mut commands, &mut vis, &mut a, g1, LIST1_BIAS, "list-1 callback");
    st.slots2.show(&mut commands, &mut vis, &mut a, g2, LIST2_BIAS, "list-2 callback");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `0x2781d0`: a square of side 2·size centred on the point pulled toward the camera, facing it, upright (z up),
    /// corners in the table's order with its ST, one colour.
    #[test]
    fn glow_quad_faces_the_camera() {
        let mut b = PrimBuf::default();
        glow_quad(&mut b, 0.5, 0.25, [10.0, 0.0, 2.0], 0x80bb_0000, [0.0, 0.0, 2.0]);
        assert_eq!(b.idx, [0, 1, 2, 1, 2, 3]);
        assert_eq!(b.uv, GLOW_ST.to_vec());
        // Camera along −x: F = (−1, 0, 0), R = F × z = (0, 1, 0), U = R × F = (0, 0, 1); the centre is 0.25 nearer.
        let game: Vec<[f32; 3]> = b.pos.iter().map(|p| [p[0], -p[2], p[1]]).collect();
        let want = [[9.75, -0.5, 2.5], [9.75, -0.5, 1.5], [9.75, 0.5, 2.5], [9.75, 0.5, 1.5]];
        for (g, w) in game.iter().zip(want) { assert!(g.iter().zip(w).all(|(a, b)| (a - b).abs() < 1e-5), "{game:?}"); }
        assert!(b.color.iter().all(|c| *c == [0.0, 0.0, 0xbb as f32 / 128.0, 1.0]));
    }

    /// The vendor's glow points: four quads at the antenna tips (1.1 out along ±x / ±y of the vendor, 0.59 up), coloured
    /// by the blue byte and the alpha of its glow word.
    #[test]
    fn vendor_glow_points_sit_on_the_antennas() {
        let id = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let g = vendor_glow_points(&id, [100.0, 50.0, 10.0], 0x8060_6060, [100.0, 40.0, 10.59]);
        assert_eq!((g.fx, g.additive), (0xb, true));
        let game: Vec<[f32; 3]> = g.prims.pos.iter().map(|p| [p[0], -p[2], p[1]]).collect();
        for (k, c) in game.chunks(4).enumerate() {
            let m = [0, 1, 2].map(|i| c.iter().map(|p| p[i]).sum::<f32>() / 4.0);
            let a = k as f32 * std::f32::consts::FRAC_PI_2 - std::f32::consts::PI;
            let want = [100.0 + 1.1 * a.cos(), 50.0 + 1.1 * a.sin(), 10.59];
            assert!(m.iter().zip(want).all(|(x, y)| (x - y).abs() < 1e-4), "point {k}: {m:?} vs {want:?}");
        }
        assert!(g.prims.color.iter().all(|c| *c == [0.0, 0.0, 0x60 as f32 / 128.0, 1.0]));
    }

    #[test]
    fn strips_and_quads_make_the_game_triangles() {
        let mut b = PrimBuf::default();
        b.strip((0..5).map(|i| ([i as f32, 0.0, 0.0], [0.0, 0.0], 0x8080_8080)));
        assert_eq!(b.idx, [0, 1, 2, 1, 2, 3, 2, 3, 4]);
        b.quad([[0.0; 3]; 4], [[0.0; 2]; 4], [0; 4]);
        assert_eq!(b.pos.len(), 5, "an all-transparent quad is skipped");
        b.quad([[0.0; 3]; 4], [[0.0; 2]; 4], [0x1000_0000, 0, 0, 0]);
        assert_eq!(&b.idx[9..], [5, 6, 7, 6, 7, 8]);
        assert_eq!(b.color[5], [0.0, 0.0, 0.0, 0.125]);
    }

    #[test]
    fn wrap_is_the_vu_remainder() {
        assert_eq!(wrap(9.5, 8.0), 1.5);
        assert_eq!(wrap(-0.25, 8.0), -0.25);
        assert_eq!(wrap(0.75, 8.0), 0.75);
    }
}
