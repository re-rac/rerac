//! Moby (object/character) instances: class meshes placed at their gameplay-file position and rotation,
//! skinned with the animated joint palette (crate::moby_anim) and lit per vertex on the GPU with the
//! game's moby lighting (crate::moby_light). Notes: docs/plan/moby_render_notes.md,
//! docs/plan/moby_animation.md ("In the port").
//!
//! Placement. The game draws a moby with the VU1 matrix `V · [s·r0; s·r1; s·r2; 1024·(p − cam)]`
//! (MobyProc `fun_00211808`, docs/plan/moby_skinning_lighting.md §2): packed model integers (the skinned
//! vertex position) go to world units ×1024 through the rotation rows r_i (moby+0xc0.., built from the
//! instance's Euler angles by `fun_0020def8` / VU0 28259, R = Rz·Ry·Rx), the scale s = class scale ×
//! instance scale (moby+0x2c), and the position p (moby+0x10). So world = s/1024 · R · packed + p. The
//! mesh holds the packed positions in game axes; the per-instance storage record holds that affine map
//! followed by `game_to_bevy`, and the shader applies it to the skinned, truncated position.
//!
//! Per frame (`update_moby_occlusion`), MobyProc's decisions for every placed instance, in its order:
//! the occlusion word (moby +0x36/+0x37, `LevelOcclusion::moby`, level01 0x26ab2c..0x26ab58; instances whose
//! gameplay `occlusion` word is not 0 carry bit 1023 and always pass), then crate::moby_lod: the
//! draw-distance / near / frustum sphere culls, the LOD, the distance alpha fade and the metal (shine) gate.
//! Each instance has four entity groups — high LOD, high LOD fading (TEST_1 AREF 0x08), low LOD, low LOD
//! fading — plus its metal entities; exactly one group (or none) is `Visibility::Inherited`, switched only
//! when the pick changes, so the draw count stays that of one LOD. A moby MobyProc does not draw gets no
//! animation job, so its pose is not evaluated (`AnimInstance::visible` = drawn, the game's +0x31);
//! `MobyAnimAdvance` still ticks it.
//!
//! GPU layout (see moby.wgsl, moby_metal.wgsl):
//! * one mesh per (class, LOD, texture): position (packed i16 as f32), UV, a `Uint32x4` skin word
//!   (azimuth | elevation << 8 | joint count << 16; joints; weights 10 bits each; RGBA multiplier) and a `u32`
//!   class vertex id (for the CPU-colour mode). Texture −1 packets use the reserved 8×8 0x80808080 texture
//!   (GS block 0x3ffb, docs/plan/moby_untextured.md): a 1×1 grey image, so the pixel is the vertex colour.
//!   A low LOD drawn with job joint count 0 (class byte 9 = 0) gets joint count 0 = the identity;
//! * one mesh per (class, metal texture −2 chrome / −3 glass) from the metal packets: position and skin word;
//! * one entity per (instance, part, GS pass) with `MeshTag(instance index)`, so every instance of a class
//!   part is one instanced draw; `NoFrustumCulling` because animated poses leave the bind-pose AABB;
//! * storage: `MobyInst` per gameplay instance (model matrix, light block, palette base, colour mode),
//!   the palette (`mat4x4` per joint slot, rewritten after each 60 Hz tick), the boot-ELF normal table
//!   (256 × (cos, sin)), the CPU colour table (only filled with `RC_MOBY_CPU_LIGHT=1`) and `MobyLod` per
//!   instance (vertex alpha, tint flag, shine alpha, sphere-map basis; rewritten per frame).
//!
//! Winding is not normalised and the GS does not cull, so both faces are drawn. GS state per moby is
//! ALPHA_1 0x8000000044 and TEST_1 0x5360b (MobyProc 0x1dedc0), 0x5308b while the distance fade is below
//! 0x80: a part whose As can differ from 0x80 is drawn twice with the AREF split (crate::gs_state); textures
//! repeat (CLAMP_1 = 0 on every Novalis moby ad-gif), bilinear, no mips. The metal pass: crate::moby_lod,
//! moby_metal.wgsl.
//!
//! Blend per moby ([`MobyBlend`], docs/plan/moby_render_notes.md §8): the vertex alpha is `(fade · moby+0x23) >> 7` for
//! every moby (statics, dynamic slots); a moby whose +0x23 is below 0x80 (the explosion flashes) is drawn alpha-blended
//! without depth write, one with mode bit 0x200 additive; their entity groups are spawned on first use, and entities
//! carry the moby's translation (the blended phase's sort key). The point lights in range of a moby become its third
//! light ([`point_light_merge`], from [`PointLightFrame`]).
//!
//! Glow list (docs/plan/moby_skinning_lighting.md §10): the packets from class byte 0xa (high) / 0xb (low) on are glow
//! packets ([`SKIN_GLOW`], their own parts); a moby with mode bit 0x10 draws them in its +0x90 RGB at its vertex alpha
//! (`MobyLod.misc.w`, [`MobyLook`], `SlotLook::glow`), their soft edge on display bytes (`GsPass::EffectLowAlpha`).
//! Dynamic slots get the metal pass too, gated by their own +0x73 ([`ExtraMobys::show_slot`]).

use crate::gs_state::{self, AlphaRange, GsPass};
use crate::level_load::LoadedLevel;
use crate::moby_anim::{self, AnimInstance, MobyAnim};
use crate::moby_light::{self, GpuLights, MobyLighting};
use crate::moby_lod::{self, ProcInput, ProcPick};
use crate::tfrag_render::game_to_bevy;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, VisibilitySystems};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshTag, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::gameplay::{self, MobyInstance};
use rc_formats::level::LevelCore;
use rc_formats::moby::{self, LevelMobyClass, MobySubmesh};
use rc_formats::moby_anim::{AnimState, MobyAnimClass};
use rc_formats::moby_light::MobyLights;
use rc_formats::occlusion::OcclBits;
use rc_formats::texture::{Texture, TextureTable};
use rc_formats::tfrag_light::ps2;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

const SHADER_PATH: &str = "shaders/moby.wgsl";
const METAL_SHADER_PATH: &str = "shaders/moby_metal.wgsl";

/// Per vertex: azimuth | elevation << 8 | skin count << 16 | [`SKIN_GLOW`]; joints j0 | j1 << 8 | j2 << 16;
/// weights w0 | w1 << 10 | w2 << 20 (/256); RGBA multiplier bytes (0x80 = 1.0).
pub const ATTRIBUTE_MOBY_SKIN: MeshVertexAttribute = MeshVertexAttribute::new("MobySkin", 0x4d4f_4231, VertexFormat::Uint32x4);
/// Skin word x bit: the vertex is in a glow packet (index ≥ class byte 0xa for the high LOD, 0xb for the low one), whose
/// colour MobyProc's glow list replaces with the moby's glow word (crate::moby_lod module doc, moby.wgsl).
pub const SKIN_GLOW: u32 = 1 << 24;
/// Per vertex: index of the (packet, vertex) in the class's high-LOD list (CPU colour table index).
pub const ATTRIBUTE_MOBY_VID: MeshVertexAttribute = MeshVertexAttribute::new("MobyVid", 0x4d4f_4232, VertexFormat::Uint32);

/// Colour modes in `MobyInst.misc.y`.
const MODE_GPU_LIGHT: u32 = 0;
const MODE_CPU_TABLE: u32 = 1;
const MODE_UNLIT: u32 = 2;

/// Texture key of texture −1 packets: the reserved 8×8 texture at GS block 0x3ffb that `init_once` fills with
/// 0x80808080 (docs/plan/moby_untextured.md).
const GREY: usize = usize::MAX;
/// Metal packet textures (TEX0 data_lo): the level's chrome and glass maps.
const TEX_CHROME: i32 = -2;
const TEX_GLASS: i32 = -3;

/// Level mobys after load: classes, animation data, instances and their light blocks.
pub struct LevelMobys {
    pub classes: Vec<LevelMobyClass>,
    /// Per class: sequences, skeleton, rest pose (crate::moby_anim).
    pub anim: Vec<MobyAnimClass>,
    pub instances: Vec<MobyInstance>,
    /// Per instance: placement, or None when the class has no geometry in this level.
    pub placed: Vec<Option<Placed>>,
    pub lighting: Option<MobyLighting>,
    /// `RC_MOBY_CPU_LIGHT=1` (or `RC_MOBY_LIGHT_CHECK=1`): distinct bit-exact bind-pose colour sets
    /// (class, per packet per vertex RGBA); `Placed::colors` indexes it.
    pub colors: Vec<(usize, Vec<Vec<[u8; 4]>>)>,
    pub cpu_light: bool,
    /// The metal pass's environment maps: chrome (128×128) and glass (64×64) PSMT8 from gs_ram at the core
    /// header's `chrome_map_*` / `glass_map_*` byte offsets (the TEX0 words 0x19e6c0 / 0x19e6d8 the level
    /// loader builds, L01 0x258128).
    pub env_maps: [Option<Texture>; 2],
    pub parse_time: Duration,
    pub light_time: Duration,
}

#[derive(Clone, Copy)]
pub struct Placed {
    pub class: usize,
    /// moby+0xc0..0xe0 as f32 (row i = image of model axis i, game space).
    pub rows: [[f32; 3]; 3],
    /// moby+0x2c = class scale × instance scale.
    pub scale: f32,
    /// The MobyProc light block (None with `RC_NO_LIGHT`).
    pub lights: Option<MobyLights>,
    /// CPU colour set (index into `LevelMobys::colors`), when computed.
    pub colors: Option<usize>,
}

fn env_on(name: &str) -> bool { std::env::var(name).is_ok_and(|v| v.trim() == "1") }

/// Chrome (128×128) and glass (64×64) maps: TEX0 TBP0 = (gs base + `*_texture`) >> 8, CBP = (gs base +
/// `*_palette`) >> 8, both byte offsets into the gs_ram lump (0x1d308000 / 0x19304000 give the sizes).
fn decode_env_maps(core: &LevelCore, gs_ram: &[u8]) -> [Option<Texture>; 2] {
    let h = &core.header;
    let one = |tex: i32, pal: i32, side: u32| -> Option<Texture> {
        let (t, p) = (usize::try_from(tex).ok()?, usize::try_from(pal).ok()?);
        let px = gs_ram.get(t..t + (side * side) as usize)?;
        let clut = gs_ram.get(p..p + 1024)?;
        rc_formats::texture::decode_indexed8(px, side, side, clut).ok()
    };
    [one(h.chrome_map_texture, h.chrome_map_palette, 128), one(h.glass_map_texture, h.glass_map_palette, 64)]
}

/// Parses the classes, their sequences and the instances, and builds every instance's light block.
pub fn load_mobys(root: &Path, core: &LevelCore, core_data: &[u8], gameplay_file: &[u8]) -> Result<LevelMobys> {
    let gs_ram = crate::disc_source::level_file(root, crate::level_load::level_index(), "gs_ram.bin").context("reading gs_ram for the moby env maps")?;
    load_mobys_with_gs(root, core, core_data, gameplay_file, &gs_ram)
}

/// [`load_mobys`] with the GS image given (the level's gs_ram, or the title world's GS upload, crate::title_world).
pub fn load_mobys_with_gs(root: &Path, core: &LevelCore, core_data: &[u8], gameplay_file: &[u8], gs_ram: &[u8]) -> Result<LevelMobys> {
    let t0 = Instant::now();
    let classes = moby::parse_level_mobys(core, core_data).context("parsing moby classes")?;
    let anim = moby_anim::load_anim_classes(core, core_data, &classes);
    let instances = gameplay::parse_moby_instances(gameplay_file).context("parsing moby instances")?;
    let env_maps = decode_env_maps(core, gs_ram);
    let parse_time = t0.elapsed();

    let t0 = Instant::now();
    let lighting: Option<MobyLighting> = moby_light::load(root, gameplay_file)?;
    let cpu_light = env_on("RC_MOBY_CPU_LIGHT");
    let check = env_on("RC_MOBY_LIGHT_CHECK");
    let by_class: HashMap<i32, usize> = classes.iter().enumerate().map(|(i, c)| (c.o_class, i)).collect();
    let mut colors = Vec::new();
    let mut shared: HashMap<(usize, [u32; 9], u32, [u8; 3]), usize> = HashMap::new();
    let mut check_sum = (0usize, 0i32, 0usize, 0usize);
    let placed = instances
        .iter()
        .map(|m| {
            let &ci = by_class.get(&m.o_class)?;
            let class = &classes[ci];
            let rows = moby_light::instance_rows(m);
            let lights = lighting.as_ref().map(|l| moby_light::instance_lights(l, &rows, m));
            let set = (cpu_light || check).then(|| {
                let key_rows: [u32; 9] = std::array::from_fn(|i| rows[i / 3][i % 3]);
                let key = (ci, key_rows, m.light_word(), m.ambient_rgb());
                *shared.entry(key).or_insert_with(|| {
                    let exact = moby_light::light_instance(lighting.as_ref(), class, lights.as_ref());
                    if let (true, Some(l), Some(k)) = (check, lighting.as_ref(), lights.as_ref()) {
                        let (n, max, over1, any) = moby_light::light_check(l, class, k, &exact);
                        check_sum = (check_sum.0 + n, check_sum.1.max(max), check_sum.2 + over1, check_sum.3 + any);
                    }
                    colors.push((ci, exact));
                    colors.len() - 1
                })
            });
            let scale = f32::from_bits(ps2::mul(class.class.header.scale.to_bits(), m.scale.to_bits()));
            Some(Placed { class: ci, rows: rows.map(|r| [0, 1, 2].map(|k| f32::from_bits(r[k]))), scale, lights, colors: set })
        })
        .collect();
    if check {
        println!(
            "moby light check (identity palette, {} distinct colour sets): {} vertices, max per-channel |GPU f32 replica − CPU exact| = {}, \
             {} vertices differ by > 1, {} differ at all",
            colors.len(), check_sum.0, check_sum.1, check_sum.2, check_sum.3
        );
    }
    Ok(LevelMobys { classes, anim, instances, placed, lighting, colors, cpu_light, env_maps, parse_time, light_time: t0.elapsed() })
}

/// Moby material: texture × vertex colour (GS MODULATE), tfrag fog; skinning and lighting in the vertex
/// shader from the storage buffers (shared by every moby material).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GsPass)]
pub struct MobyMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    /// The moby VU1 program writes F with the same fog lanes as the tfrag one (docs/plan/moby_render_notes.md).
    #[storage(2, read_only)]
    /// The shared fog buffer (crate::game_camera::fog_buffer).
    pub fog: Handle<bevy::render::storage::ShaderBuffer>,
    /// `MobyInst` per gameplay instance, indexed by `MeshTag`.
    #[storage(3, read_only, visibility(vertex))]
    pub instances: Handle<ShaderBuffer>,
    /// Joint palette, `mat4x4` per slot (crate::moby_anim).
    #[storage(4, read_only, visibility(vertex))]
    pub palette: Handle<ShaderBuffer>,
    /// Boot-ELF normal table: 256 × vec2 (cos, sin).
    #[storage(5, read_only, visibility(vertex))]
    pub normal_table: Handle<ShaderBuffer>,
    /// CPU bit-exact colours (`RC_MOBY_CPU_LIGHT=1`), packed RGBA8.
    #[storage(6, read_only, visibility(vertex))]
    pub cpu_colors: Handle<ShaderBuffer>,
    /// `MobyLod` per instance (crate::moby_lod), indexed by `MeshTag`.
    #[storage(7, read_only, visibility(vertex))]
    pub lods: Handle<ShaderBuffer>,
    /// This draw's GS state (crate::gs_state; same TEST_1 as tfrags, AREF 0x08 while fading).
    pub pass: GsPass,
    /// Drawn in moby order ([`ExtraMobys::set_ordered`]): the moby's place in `DrawMobys`' list.
    pub order: Option<u32>,
}

/// Transparent3d sort bands (`Material::depth_bias`, added to the view depth, which grows towards the camera): a
/// shadow caster's own draws ([`caster_pass`]) come first in the phase (right after the shadow pass, as the game
/// draws its deferred mobys), then the casters' metal passes; every other blended draw keeps its view-depth order
/// after them. The bands are further apart than any view depth (the far plane is 728 units).
pub const CASTER_BAND: f32 = -30000.0;
pub const CASTER_METAL_BAND: f32 = -20000.0;
/// Ordered draws ([`MobyMaterial::order`]): moby k's band starts at `ORDER_BAND + k·ORDER_STEP`, its Z-writing draws
/// first, its colour-only half `ORDER_STEP / 3` later, its shine `2·ORDER_STEP / 3` later; between the casters' metal
/// and the view-depth-sorted draws (15 mobys fit; the actors of a scene stand well within a third of a step apart).
pub const ORDER_BAND: f32 = -19000.0;
pub const ORDER_STEP: f32 = 1200.0;

/// The band of ordered moby `order`'s draws of `sub` (0 Z-writing, 1 colour-only / blended, 2 shine).
fn order_bias(order: u32, sub: u32) -> f32 { ORDER_BAND + order.min(14) as f32 * ORDER_STEP + sub as f32 * (ORDER_STEP / 3.0) }

/// A shadow caster's draw (docs/plan/shadows.md §6.3: a class with a shadow block is deferred by `MobyProc` and
/// drawn after the shadow pass, so it is never darkened): its Z-writing draws move from Opaque3d / AlphaMask3d to
/// the start of Transparent3d ([`CASTER_BAND`]); its colour-only half and blended modes stay as they are.
pub fn caster_pass(p: GsPass) -> GsPass {
    match p {
        GsPass::Opaque => GsPass::LateOpaque,
        GsPass::OpaqueTested { aref } => GsPass::LateTested { aref },
        p => p,
    }
}

/// The draw of `part` for GS pass `p`: a caster's Z-writing draws move late ([`caster_pass`]); a glow part's
/// colour-only half (its soft edge, As < AREF) blends on display bytes like the other glows (crate::display_blend).
fn part_pass(part: &Part, p: GsPass) -> GsPass {
    match p {
        GsPass::ColorOnlyLowAlpha { aref } if part.glow => GsPass::EffectLowAlpha { aref },
        p if part.caster => caster_pass(p),
        p => p,
    }
}

/// A shadow caster's Z-writing draw in both orders: `late` (the deferred moby, drawn after the shadow pass) and
/// `early` (`MobyProc` drew the moby with the others: mode 0x400 off or no live shadow this frame, so other casters'
/// shadows fall on it). Which one is used follows the moby ([`set_late`]).
#[derive(Component, Clone)]
pub struct CasterTwin {
    pub early: Handle<MobyMaterial>,
    pub late: Handle<MobyMaterial>,
}

/// Points the caster draws among `entities` at their late or early material (a queued command: groups spawned this
/// frame included).
pub fn queue_late(commands: &mut Commands, entities: Vec<Entity>, late: bool) {
    commands.queue(move |world: &mut World| {
        for e in entities {
            let Some(t) = world.get::<CasterTwin>(e).cloned() else { continue };
            let want = if late { t.late } else { t.early };
            if world.get::<MeshMaterial3d<MobyMaterial>>(e).is_some_and(|m| m.0 != want) { world.entity_mut(e).insert(MeshMaterial3d(want)); }
        }
    });
}

impl From<&MobyMaterial> for GsPass {
    fn from(m: &MobyMaterial) -> Self { m.pass }
}

impl Material for MobyMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { self.pass.alpha_mode() }
    /// A caster's late draws first in Transparent3d ([`CASTER_BAND`]); `LateTested` is only a caster's here.
    fn depth_bias(&self) -> f32 {
        let z_writing = matches!(self.pass, GsPass::LateOpaque | GsPass::LateTested { .. });
        match self.order {
            Some(k) => order_bias(k, if z_writing { 0 } else { 1 }),
            None if z_writing => CASTER_BAND,
            None => 0.0,
        }
    }
    /// No culling: the GS draws both faces and the index stream's winding is inconsistent.
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
            ATTRIBUTE_MOBY_SKIN.at_shader_location(2),
            ATTRIBUTE_MOBY_VID.at_shader_location(3),
        ])?];
        Ok(())
    }
}

/// Metal (shine) pass material: the chrome or glass map × the metal vertex colour, sphere-mapped in the
/// vertex shader (moby_metal.wgsl), with the moby's records, palette and `MobyLod` buffers.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GsPass)]
pub struct MobyMetalMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[storage(2, read_only)]
    /// The shared fog buffer (crate::game_camera::fog_buffer).
    pub fog: Handle<bevy::render::storage::ShaderBuffer>,
    #[storage(3, read_only, visibility(vertex))]
    pub instances: Handle<ShaderBuffer>,
    #[storage(4, read_only, visibility(vertex))]
    pub palette: Handle<ShaderBuffer>,
    #[storage(5, read_only, visibility(vertex))]
    pub normal_table: Handle<ShaderBuffer>,
    #[storage(6, read_only, visibility(vertex))]
    pub lods: Handle<ShaderBuffer>,
    pub pass: GsPass,
    /// The metal of a shadow caster: after the caster's own late draws ([`CASTER_METAL_BAND`]).
    pub caster: bool,
    /// The metal of an ordered moby ([`MobyMaterial::order`]): after its own draws.
    pub order: Option<u32>,
}

impl From<&MobyMetalMaterial> for GsPass {
    fn from(m: &MobyMetalMaterial) -> Self { m.pass }
}

impl Material for MobyMetalMaterial {
    fn vertex_shader() -> ShaderRef { METAL_SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { METAL_SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { self.pass.alpha_mode() }
    fn depth_bias(&self) -> f32 {
        match self.order {
            Some(k) => order_bias(k, 2),
            None if self.caster => CASTER_METAL_BAND,
            None => 0.0,
        }
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        key.bind_group_data.specialize(descriptor);
        descriptor.vertex.buffers =
            vec![layout.0.get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0), ATTRIBUTE_MOBY_SKIN.at_shader_location(2)])?];
        Ok(())
    }
}

/// The metal pass's GS passes: its As = At·shine >> 7 spans 0..=0x80, so both halves of the TEST_1 0x5360b
/// split; the Z-writing half runs in Transparent3d (`LateTested`) so it follows the moby's own draws.
fn metal_passes(texel: AlphaRange) -> Vec<GsPass> {
    gs_state::draws(gs_state::AREF_WORLD, texel, AlphaRange { min: 0, max: 0x80 })
        .into_iter()
        .map(|p| match p { GsPass::OpaqueTested { aref } => GsPass::LateTested { aref }, p => p })
        .collect()
}

/// How a moby is blended, chosen per moby from the game's own data, the way MobyProc (level01 0x26a7a0) decides it
/// (docs/plan/moby_skinning_lighting.md §9): the moby's alpha byte +0x23 times the distance fade is its vertex
/// alpha, and its mode bits +0x34 carry the effect flags (0x200 additive, 8 "fading"). The port draws each with a
/// native render mode:
/// * [`Self::Plain`]: vertex alpha 0x80: the regular moby draw (opaque, texture cut-outs as before);
/// * [`Self::Fading`]: the distance fade (or mode bit 8): the existing fade draw, unchanged;
/// * [`Self::Translucent`]: +0x23 below 0x80 (the explosion flashes 0x70 / 1192 are spawned with 0x20..0x40 and
///   fade to 0, a fading body piece counts it down): **alpha blend on display bytes (crate::display_blend), no depth
///   write, sorted back to front**; in the game such a moby's pixels fail the Z-writing alpha test (below 0x60), so
///   they blend over the scene without occluding what is behind;
/// * [`Self::Additive`]: mode bit 0x200 (the game switches the moby to `Cs·As + Cd`): **additive on display bytes,
///   no depth write, sorted**.
///
/// Each value is one entity group per (instance, LOD) ([`Self::passes`]). No class is special-cased.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MobyBlend {
    /// Vertex alpha 0x80, TEST_1 0x5360b, ALPHA_1 0x44.
    Plain,
    /// Vertex alpha ≠ 0x80 (moby+0x23), TEST_1 0x5360b, ALPHA_1 0x44.
    Translucent,
    /// TEST_1 0x5308b (distance fade or mode bit 8), ALPHA_1 0x44.
    Fading,
    /// Mode bit 0x200: ALPHA_1 0x48, TEST_1 0x5360b.
    Additive,
}

impl MobyBlend {
    /// The group for mode bits `mode` (moby+0x34), MobyProc's `fading` (fade < 0x80) and the vertex alpha.
    pub fn pick(mode: u16, fading: bool, vertex_alpha: u8) -> MobyBlend {
        let t6 = (mode & 0x208) | ((fading as u16) << 3);
        if t6 & 0x200 != 0 {
            MobyBlend::Additive
        } else if t6 & 8 != 0 {
            MobyBlend::Fading
        } else if vertex_alpha != 0x80 {
            MobyBlend::Translucent
        } else {
            MobyBlend::Plain
        }
    }

    /// The draws of a part with texel alpha range `texel` and per-vertex multiplier alphas `mult`.
    pub fn passes(self, texel: AlphaRange, mult: AlphaRange) -> Vec<GsPass> {
        match self {
            MobyBlend::Plain => gs_state::draws(gs_state::AREF_WORLD, texel, mult),
            MobyBlend::Fading => {
                let fade = AlphaRange { min: 0, max: ((0x7f * mult.max as u32) >> 7) as u8 };
                gs_state::draws(moby_lod::AREF_FADE, texel, fade)
            }
            MobyBlend::Translucent => vec![GsPass::EffectMix],
            MobyBlend::Additive => vec![GsPass::AdditiveNoZ],
        }
    }
}

/// The buffers every material of one record set shares (moby.wgsl bindings 2..7).
#[derive(Clone)]
struct MatProto {
    instances: Handle<ShaderBuffer>,
    palette: Handle<ShaderBuffer>,
    normal_table: Handle<ShaderBuffer>,
    cpu_colors: Handle<ShaderBuffer>,
    lods: Handle<ShaderBuffer>,
}

/// Images and materials of one record set, by texture and GS pass; spawns part entities.
struct MatCache {
    proto: MatProto,
    images: HashMap<usize, (Handle<Image>, AlphaRange)>,
    mats: HashMap<(usize, GsPass, Option<u32>), Handle<MobyMaterial>>,
    /// Draw the tags (slots) in their order ([`ExtraMobys::set_ordered`]).
    ordered: bool,
    /// Distinct (mesh, material) pairs spawned (the stats line's instanced draws).
    batches: HashSet<(AssetId<Mesh>, AssetId<MobyMaterial>)>,
}

impl MatCache {
    fn new(proto: MatProto) -> Self { MatCache { proto, images: HashMap::new(), mats: HashMap::new(), batches: HashSet::new(), ordered: false } }

    /// One entity per (part, GS pass of `blend`), tagged `tag`, at `transform` (only its translation matters: the
    /// Transparent3d sort key; the shader places the vertices from the record). A caster's Z-writing draws get both
    /// orders ([`CasterTwin`]) and start `late` or early.
    #[allow(clippy::too_many_arguments)]
    fn spawn(
        &mut self,
        commands: &mut Commands,
        level: &LoadedLevel,
        parts: &[Part],
        blend: MobyBlend,
        late: bool,
        transform: Transform,
        tag: u32,
        visibility: Visibility,
        name: &str,
        images: &mut Assets<Image>,
        materials: &mut Assets<MobyMaterial>,
    ) -> Vec<Entity> {
        let mut out = Vec::new();
        for part in parts {
            let (image, texel) = self.images.entry(part.texture).or_insert_with(|| moby_image(level, part.texture, images)).clone();
            for base in blend.passes(texel, part.mult_alpha) {
                if self.ordered {
                    // In moby order: every Z-writing draw late (as a caster's), in the moby's band; a glow part's
                    // colour-only half stays in the band too (not the display-blend effect pass, which draws after
                    // everything: G-REN-028), blended in linear light.
                    let pass = caster_pass(base);
                    let mat = self.material_in(part.texture, pass, Some(tag), &image, materials);
                    self.batches.insert((part.mesh.id(), mat.id()));
                    let mut e = commands.spawn((
                        Mesh3d(part.mesh.clone()),
                        MeshMaterial3d(mat),
                        transform,
                        MeshTag(tag),
                        NoFrustumCulling,
                        visibility,
                        Name::new(format!("{name} tex {} {pass:?} order {tag}", part.texture as isize)),
                    ));
                    if pass.state().display { e.insert(crate::display_blend::DisplayEffect); }
                    out.push(e.id());
                    continue;
                }
                let pass = part_pass(part, base);
                // The same draw not deferred: only a caster's Z-writing passes differ (caster_pass).
                let early = if part.caster && caster_pass(base) != base { base } else { pass };
                let twin = (early != pass).then(|| CasterTwin { early: self.material(part.texture, early, &image, materials), late: self.material(part.texture, pass, &image, materials) });
                let mat = match &twin {
                    Some(t) => if late { t.late.clone() } else { t.early.clone() },
                    None => self.material(part.texture, pass, &image, materials),
                };
                self.batches.insert((part.mesh.id(), mat.id()));
                let mut e = commands.spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(mat),
                    transform,
                    MeshTag(tag),
                    NoFrustumCulling,
                    visibility,
                    Name::new(format!("{name} tex {} {pass:?}", part.texture as isize)),
                ));
                // Effect draws (display-byte blending) are drawn by crate::display_blend's effect pass.
                if pass.state().display { e.insert(crate::display_blend::DisplayEffect); }
                if let Some(t) = twin { e.insert(t); }
                out.push(e.id());
            }
        }
        out
    }

    /// The material of (texture, GS pass), made once.
    fn material(&mut self, texture: usize, pass: GsPass, image: &Handle<Image>, materials: &mut Assets<MobyMaterial>) -> Handle<MobyMaterial> {
        self.material_in(texture, pass, None, image, materials)
    }

    /// [`Self::material`] for draw order `order` ([`MobyMaterial::order`]).
    fn material_in(&mut self, texture: usize, pass: GsPass, order: Option<u32>, image: &Handle<Image>, materials: &mut Assets<MobyMaterial>) -> Handle<MobyMaterial> {
        let proto = &self.proto;
        self.mats
            .entry((texture, pass, order))
            .or_insert_with(|| {
                materials.add(MobyMaterial {
                    texture: image.clone(),
                    fog: crate::game_camera::fog_buffer(),
                    instances: proto.instances.clone(),
                    palette: proto.palette.clone(),
                    normal_table: proto.normal_table.clone(),
                    cpu_colors: proto.cpu_colors.clone(),
                    lods: proto.lods.clone(),
                    pass,
                    order,
                })
            })
            .clone()
    }
}

/// Writes bytes `range` of `MobyLod` record `slot` in `buf` (the whole record: `0..LOD_RECORD_SIZE`), keeping the other
/// fields; true when it changed.
fn set_lod_bytes(buffers: &mut Assets<ShaderBuffer>, buf: &Handle<ShaderBuffer>, slot: usize, from: usize, want: &[u8]) -> bool {
    let at = slot * moby_lod::LOD_RECORD_SIZE + from;
    let range = at..at + want.len();
    if buffers.get(buf).and_then(|b| b.data.as_ref()).and_then(|d| d.get(range.clone())).is_none_or(|d| d == want) { return false; }
    let Some(mut b) = buffers.get_mut(buf) else { return false };
    if let Some(d) = b.data.as_mut().and_then(|d| d.get_mut(range)) { d.copy_from_slice(want); }
    true
}

/// The point-light bank as the game tick left it (`rc_game::point_lights`, written by crate::gameplay after every
/// tick): what MobyProc merges into the mobys' third light.
#[derive(Resource, Default, Clone)]
pub struct PointLightFrame(pub Vec<rc_game::point_lights::PointLight>);

/// MobyProc's point-light merge (0x26aa.., docs/plan/moby_skinning_lighting.md "Point lights"): for every light j
/// whose distance δ_j from the moby's sphere centre `c` (world units) is below its radius R_j, `a_j = 1 − δ_j/R_j`
/// and the model-space direction to it `ℓ_j = Rᵀ·(x_j − c)/δ_j` (`rows` = moby+0xc0.., row i = image of model axis i);
/// the third light is `C_2 = Σ a_j·colour_j`, `L_2 = normalize(Σ a_j·ℓ_j)` (one light: its own ℓ), K_2 = 0 (no
/// back light). None: no light in range (C_2 = 0).
pub fn point_light_merge(lights: &[rc_game::point_lights::PointLight], c: [f32; 3], rows: &[[f32; 3]; 3]) -> Option<([f32; 3], [f32; 3])> {
    let (mut col, mut dir, mut n) = ([0.0f32; 3], [0.0f32; 3], 0);
    for l in lights {
        let d = [l.pos[0] - c[0], l.pos[1] - c[1], l.pos[2] - c[2]];
        let delta = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if delta >= l.radius || delta.is_nan() || delta == 0.0 { continue; }
        let a = 1.0 - delta / l.radius;
        let ell: [f32; 3] = std::array::from_fn(|i| (rows[i][0] * d[0] + rows[i][1] * d[1] + rows[i][2] * d[2]) / delta);
        for k in 0..3 {
            col[k] += a * l.color[k];
            dir[k] += a * ell[k];
        }
        n += 1;
    }
    if n == 0 { return None; }
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len > 0.0 { dir = dir.map(|x| x / len); }
    Some((dir, col))
}

/// Writes the merged point light (or none) into a `MobyInst` record: lane z of the three light rows, colour 2 and
/// −|K_2| = 0. True when the record changed.
pub fn write_point_light(rec: &mut [u8], merged: Option<([f32; 3], [f32; 3])>) -> bool {
    let (dir, col) = merged.unwrap_or(([0.0; 3], [0.0; 3]));
    let mut changed = false;
    let mut put = |o: usize, v: f32| {
        if let Some(d) = rec.get_mut(o..o + 4) {
            let b = v.to_le_bytes();
            if d != b { d.copy_from_slice(&b); changed = true; }
        }
    };
    for (j, &x) in dir.iter().enumerate() { put(64 + 16 * j + 8, x); }
    for (k, &x) in col.iter().enumerate() { put(64 + 48 + 32 + 4 * k, x); }
    put(64 + 96 + 8, 0.0);
    changed
}

pub struct MobyRenderPlugin;

impl Plugin for MobyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MaterialPlugin::<MobyMaterial>::default(), MaterialPlugin::<MobyMetalMaterial>::default(), moby_anim::MobyAnimPlugin))
            .init_resource::<PointLightFrame>()
            // A runtime level change (crate::level_switch): `spawn_system` builds them again.
            .add_systems(crate::level_switch::LevelUnload, (crate::level_switch::reset::<PointLightFrame>, crate::level_switch::remove::<MobyOcclusion>, crate::level_switch::remove::<crate::moby_anim::MobyAnim>))
            .add_systems(crate::level_switch::LevelStartup, spawn_system)
            .add_systems(
                PostUpdate,
                update_moby_occlusion
                    .after(crate::occlusion::OcclusionSet)
                    .before(VisibilitySystems::VisibilityPropagate)
                    .before(bevy::asset::AssetEventSystems),
            )

            // After moby_attach's PostUpdate upload of the extra records (read here).
            .add_systems(Last, update_extra_metal);
    }
}

fn spawn_system(
    mut commands: Commands,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut metal_materials: ResMut<Assets<MobyMetalMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    // The classes a placed moby may become at run time (rc_game::moby_update::class_swap, G-CLS-031): only where the
    // moby loop runs the level's class code.
    let swaps = |o: i16| -> Vec<i16> {
        if !crate::gameplay::enabled() { return Vec::new(); }
        rc_game::moby_update::class_swap::targets(crate::gameplay::level_ports(), o)
    };
    let s = spawn_mobys(&mut commands, &level.0, &swaps, &mut meshes, &mut images, &mut materials, &mut metal_materials, &mut buffers);
    let m = &level.0.mobys;
    let mode = if m.cpu_light { "CPU bit-exact colours, bind pose (RC_MOBY_CPU_LIGHT=1)" } else if m.lighting.is_none() { "unlit (RC_NO_LIGHT)" } else { "GPU skinning + lighting" };
    println!(
        "mobys: {} instances in the gameplay file, {} placed ({} without class geometry), {} classes parsed, {} used; \
         {} high-LOD triangles ({} of them texture -1, drawn with the grey 0x80 texture), {} low-LOD triangles, {} metal triangles \
         ({} chrome + {} glass), skipped {} on an unused class slot; {} meshes, {} entities ({} per LOD group high/high fading/low/low fading, \
         {} metal), <= {} instanced draws, {} images; {mode}",
        m.instances.len(), s.placed, s.no_class, m.classes.len(), s.classes_used,
        s.triangles, s.grey, s.low_triangles, s.chrome + s.glass, s.chrome, s.glass, s.untextured, s.meshes, s.entities,
        format_args!("{}/{}/{}/{}", s.group_entities[0], s.group_entities[1], s.group_entities[2], s.group_entities[3]),
        s.metal_entities, s.batches, s.images
    );
    println!(
        "mobys: parse {:.1} ms, light blocks {:.1} ms, mesh build {:.1} ms; {} animated instances ({} static: 1 sequence of <= 1 frame, {} classes without sequence 0), \
         palette {} matrices; {} instance positions outside the tfrag sphere bounds; {} vertices skinned to a joint past the class joint count; \
         env maps: chrome {}, glass {}; LOD {} (RC_MOBY_LOD), metal pass {} (RC_MOBY_METAL)",
        m.parse_time.as_secs_f64() * 1e3, m.light_time.as_secs_f64() * 1e3, s.build.as_secs_f64() * 1e3,
        s.animated, s.static_, s.no_seq0, s.palette, s.outside, s.joint_past_count,
        m.env_maps[0].is_some(), m.env_maps[1].is_some(),
        if moby_lod::lod_enabled() && !m.cpu_light { "on" } else { "off" }, if moby_lod::metal_enabled() { "on" } else { "off" }
    );
}

#[derive(Default)]
struct Stats {
    placed: usize,
    no_class: usize,
    classes_used: usize,
    triangles: usize,
    grey: usize,
    low_triangles: usize,
    chrome: usize,
    glass: usize,
    untextured: usize,
    meshes: usize,
    entities: usize,
    group_entities: [usize; 4],
    metal_entities: usize,
    batches: usize,
    images: usize,
    outside: usize,
    animated: usize,
    static_: usize,
    no_seq0: usize,
    palette: u32,
    joint_past_count: usize,
    build: Duration,
}

/// One mesh of a class: texture key (moby texture table index or [`GREY`]), mesh, range of its vertices'
/// multiplier alphas.
#[derive(Clone)]
struct Part {
    texture: usize,
    /// The mesh holds glow-packet triangles: its colour-only half blends on display bytes (`GsPass::EffectLowAlpha`).
    glow: bool,
    mesh: Handle<Mesh>,
    /// The vertex alpha Af is ambient alpha (the `MobyLod` alpha) × multiplier alpha >> 7.
    mult_alpha: AlphaRange,
    triangles: usize,
    /// The class casts a shadow (class byte 0x0f ≠ 0): drawn after the shadow pass ([`caster_pass`]).
    caster: bool,
}

/// A metal mesh: its texture (−2 chrome / −3 glass) and triangle count.
#[derive(Clone)]
struct MetalPart {
    kind: i32,
    mesh: Handle<Mesh>,
    triangles: usize,
    caster: bool,
}

/// Texture key of a regular triangle: the moby texture table index, [`GREY`] for −1, None for an unused class
/// slot (0xff) or −2/−3 (metal packets only).
fn part_texture(class: &LevelMobyClass, texture: i32) -> Option<usize> {
    match texture {
        -1 => Some(GREY),
        t if t >= 0 => class.texture_table_index(t),
        _ => None,
    }
}

/// The first glow packet of a class's high or low packet list: class byte 0xa / 0xb (`lbu v1, 0xa(class + lod)`,
/// level01 0x26b8b0); a value at or past the list's end means no glow packets (0xff on most classes).
fn glow_from(class: &LevelMobyClass, low: bool) -> usize {
    let h = &class.class.header;
    (if low { h.rac1_byte_b } else { h.rac1_byte_a }) as usize
}

/// Class meshes of one packet list (high or low LOD), one per (texture, glow packet or not), plus the highest joint
/// index any vertex skins to. Packets from index `glow_from` on are the list's glow packets ([`SKIN_GLOW`]). `identity`: the list is drawn with job joint count 0 (the low LOD of a class with class byte 9 =
/// 0): `MobyAnimEval` then only writes the identity at palette slot 0, so every vertex gets joint count 0
/// (moby.wgsl: the identity). The vertex id is the (packet, vertex) index in `lod` (the CPU colour table only
/// covers the high LOD, and `RC_MOBY_CPU_LIGHT=1` keeps every instance on it).
fn build_parts(class: &LevelMobyClass, lod: &[MobySubmesh], identity: bool, glow_from: usize, meshes: &mut Assets<Mesh>) -> (Vec<Part>, u8) {
    #[derive(Default)]
    struct B { pos: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, skin: Vec<[u32; 4]>, vid: Vec<u32>, idx: Vec<u32>, remap: HashMap<(usize, u32), u32>, alpha: Option<AlphaRange>, tris: usize }
    let mults = moby_light::vertex_multipliers(lod);
    let mut base = 0u32;
    let vid_base: Vec<u32> = lod.iter().map(|s| { let b = base; base += s.vertices.len() as u32; b }).collect();
    let mut max_joint = 0u8;
    let mut by_tex: BTreeMap<(usize, bool), B> = BTreeMap::new();
    for (si, sub) in lod.iter().enumerate() {
        let glow = si >= glow_from;
        for t in &sub.triangles {
            let Some(tex) = part_texture(class, t.texture) else { continue };
            let b = by_tex.entry((tex, glow)).or_default();
            b.tris += 1;
            for vi in [t.a, t.b, t.c] {
                let v = *b.remap.entry((si, vi)).or_insert_with(|| {
                    let vx = &sub.vertices[vi as usize];
                    b.pos.push(vx.packed_position().map(|c| c as f32));
                    // ST 4.12 (the VU converts with itof12; q = 1.0 from STCOL), GS-normalised coordinates.
                    b.uv.push([vx.st[0] as f32 / 4096.0, vx.st[1] as f32 / 4096.0]);
                    let sk = vx.skin;
                    let n = sk.count.max(1) as usize;
                    if !identity { max_joint = max_joint.max(*sk.joints[..n].iter().max().unwrap()); }
                    let m = mults[si][vi as usize];
                    b.alpha = Some(b.alpha.map_or(AlphaRange::of([m[3]]), |r| r.union(AlphaRange::of([m[3]]))));
                    let count = if identity { 0 } else { sk.count as u32 };
                    b.skin.push([
                        vx.normal_azimuth as u32 | (vx.normal_elevation as u32) << 8 | count << 16 | if glow { SKIN_GLOW } else { 0 },
                        sk.joints[0] as u32 | (sk.joints[1] as u32) << 8 | (sk.joints[2] as u32) << 16,
                        sk.weights[0] as u32 | (sk.weights[1] as u32) << 10 | (sk.weights[2] as u32) << 20,
                        u32::from_le_bytes(m),
                    ]);
                    b.vid.push(vid_base[si] + vi);
                    (b.pos.len() - 1) as u32
                });
                b.idx.push(v);
            }
        }
    }
    let parts = by_tex
        .into_iter()
        .map(|((texture, glow), b)| {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, b.uv)
                .with_inserted_attribute(ATTRIBUTE_MOBY_SKIN, VertexAttributeValues::Uint32x4(b.skin))
                .with_inserted_attribute(ATTRIBUTE_MOBY_VID, VertexAttributeValues::Uint32(b.vid))
                .with_inserted_indices(Indices::U32(b.idx));
            Part { texture, glow, mesh: meshes.add(mesh), mult_alpha: b.alpha.unwrap_or(AlphaRange::OPAQUE), triangles: b.tris, caster: class.class.header.shadow != 0 }
        })
        .collect();
    (parts, max_joint)
}

/// The class's metal packets as one mesh per texture (−2 / −3), plus the highest joint they skin to. Metal
/// vertices carry their skin inline (up to 3 palette joints, weights /256) and no ST.
fn build_metal_parts(class: &LevelMobyClass, meshes: &mut Assets<Mesh>) -> (Vec<MetalPart>, u8) {
    #[derive(Default)]
    struct B { pos: Vec<[f32; 3]>, skin: Vec<[u32; 4]>, idx: Vec<u32>, remap: HashMap<(usize, u32), u32>, tris: usize }
    let mut by_kind: BTreeMap<i32, B> = BTreeMap::new();
    let mut max_joint = 0u8;
    for (si, sub) in class.class.metal.iter().enumerate() {
        for t in &sub.triangles {
            if t.texture != TEX_CHROME && t.texture != TEX_GLASS { continue; }
            let b = by_kind.entry(t.texture).or_default();
            b.tris += 1;
            for vi in [t.a, t.b, t.c] {
                let v = *b.remap.entry((si, vi)).or_insert_with(|| {
                    let vx = &sub.vertices[vi as usize];
                    b.pos.push(vx.packed_position().map(|c| c as f32));
                    let sk = vx.skin;
                    max_joint = max_joint.max(*sk.joints[..sk.count.max(1) as usize].iter().max().unwrap());
                    b.skin.push([
                        vx.normal_azimuth as u32 | (vx.normal_elevation as u32) << 8 | (sk.count as u32) << 16,
                        sk.joints[0] as u32 | (sk.joints[1] as u32) << 8 | (sk.joints[2] as u32) << 16,
                        sk.weights[0] as u32 | (sk.weights[1] as u32) << 10 | (sk.weights[2] as u32) << 20,
                        0,
                    ]);
                    (b.pos.len() - 1) as u32
                });
                b.idx.push(v);
            }
        }
    }
    let parts = by_kind
        .into_iter()
        .map(|(kind, b)| {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.pos)
                .with_inserted_attribute(ATTRIBUTE_MOBY_SKIN, VertexAttributeValues::Uint32x4(b.skin))
                .with_inserted_indices(Indices::U32(b.idx));
            MetalPart { kind, mesh: meshes.add(mesh), triangles: b.tris, caster: class.class.header.shadow != 0 }
        })
        .collect();
    (parts, max_joint)
}

/// `game_to_bevy` as a matrix (columns = images of game x, y, z).
fn axes() -> Mat3 { Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y) }

/// `MobyInst` (moby.wgsl): model (16 f32), light rows (3 vec4), light colours (3 vec4), −|K|, ambient,
/// misc (u32: palette base, colour mode, CPU colour base, 0). 208 bytes.
fn write_record(out: &mut Vec<u8>, model: &Mat4, g: &GpuLights, misc: [u32; 4]) {
    for v in model.to_cols_array() { out.extend_from_slice(&v.to_le_bytes()); }
    for r in g.rows.iter().chain(&g.colors).chain([&g.neg_k, &g.ambient]) {
        for v in r { out.extend_from_slice(&v.to_le_bytes()); }
    }
    for v in misc { out.extend_from_slice(&v.to_le_bytes()); }
}
const RECORD_SIZE: usize = 208;

/// A class texture scroll on a record (`misc.w`: s | t << 16, s16 each in 1/4096 of the texture: the offset
/// `FUN_00263d90(class, s, t)` has added to every vertex ST of the class's packets; the Pyrocitor's pilot flame 179).
pub fn set_uv_scroll(rec: &mut [u8], s: i16, t: i16) {
    let w = (s as u16 as u32) | (t as u16 as u32) << 16;
    rec[RECORD_SIZE - 4..RECORD_SIZE].copy_from_slice(&w.to_le_bytes());
}

/// A moby texture as an image (repeat, bilinear), with the raw GS alpha like tfrag_render; [`GREY`] is the
/// reserved 0x80808080 texture (clamp, as CLAMP_1 = 5 for −1 blocks; the texture is uniform anyway).
fn moby_image(level: &LoadedLevel, tex: usize, images: &mut Assets<Image>) -> (Handle<Image>, AlphaRange) {
    let (w, h, mut rgba, mode) = if tex == GREY {
        (1, 1, vec![0x80, 0x80, 0x80, 0xff], ImageAddressMode::ClampToEdge)
    } else {
        match level.textures.iter().find(|t| t.table == TextureTable::Moby && t.index == tex).map(|t| &t.texture) {
            Some(t) => (t.width, t.height, t.rgba.clone(), ImageAddressMode::Repeat),
            None => {
                warn!("moby texture {tex} has no decoded texture; drawing magenta");
                (1, 1, vec![255, 0, 255, 0xff], ImageAddressMode::Repeat)
            }
        }
    };
    (images.add(gs_image(w, h, &mut rgba, mode)), AlphaRange::of(rgba.iter().skip(3).step_by(4).copied()))
}

/// `texture::scale_alpha` maps a < 0x80 to 2a and a >= 0x80 to 0xff; undo it (exact for a <= 0x80) and make
/// a bilinear, unmipped `Rgba8Unorm` image.
fn gs_image(w: u32, h: u32, rgba: &mut [u8], mode: ImageAddressMode) -> Image {
    for a in rgba.iter_mut().skip(3).step_by(4) {
        *a = if *a == 0xff { 0x80 } else { *a / 2 };
    }
    let mut img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, rgba.to_vec(), TextureFormat::Rgba8Unorm, RenderAssetUsages::RENDER_WORLD);
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    img
}

/// The chrome (−2) or glass (−3) map: TEX1 MXL 0 / MMAG linear, CLAMP_1 = 5 (`fun_00202d78`). A level whose
/// map does not decode gets a magenta 1×1.
fn metal_image(level: &LoadedLevel, kind: i32, images: &mut Assets<Image>) -> (Handle<Image>, AlphaRange) {
    let (w, h, mut rgba) = match &level.mobys.env_maps[if kind == TEX_CHROME { 0 } else { 1 }] {
        Some(t) => (t.width, t.height, t.rgba.clone()),
        None => {
            warn!("moby metal texture {kind}: no env map decoded; drawing magenta");
            (1, 1, vec![255, 0, 255, 0xff])
        }
    };
    let img = gs_image(w, h, &mut rgba, ImageAddressMode::ClampToEdge);
    (images.add(img), AlphaRange::of(rgba.iter().skip(3).step_by(4).copied()))
}

/// Per class: (high parts, highest high joint), low parts, (metal parts, highest metal joint).
type ClassDraws = ((Vec<Part>, u8), Vec<Part>, (Vec<MetalPart>, u8));

/// Builds the class meshes and storage buffers and spawns the entities of every instance (module doc).
#[allow(clippy::too_many_arguments)]
fn spawn_mobys(
    commands: &mut Commands,
    level: &LoadedLevel,
    swaps: &dyn Fn(i16) -> Vec<i16>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    metal_materials: &mut Assets<MobyMetalMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> Stats {
    let t0 = Instant::now();
    let m = &level.mobys;
    let mut st = Stats::default();
    let lod_on = moby_lod::lod_enabled() && !m.cpu_light;

    // Per placed instance, the other classes it may take at run time (`swaps`: a class swap of its class code): their
    // meshes, palette slots and metal entities are made with the level (MobyOcclusion::set_class).
    let class_ix: HashMap<i32, usize> = m.classes.iter().enumerate().map(|(i, c)| (c.o_class, i)).collect();
    let swap_to: Vec<Vec<usize>> = m.instances.iter().zip(&m.placed)
        .map(|(inst, p)| if p.is_none() { Vec::new() } else { swaps(inst.o_class as i16).into_iter().filter_map(|o| class_ix.get(&(o as i32)).copied()).collect() })
        .collect();
    // Class meshes, only for classes something instances (or may become), in class order (asset ids follow it).
    let used: BTreeSet<usize> = m.placed.iter().flatten().map(|p| p.class).chain(swap_to.iter().flatten().copied()).collect();
    st.classes_used = used.len();
    let mut parts: HashMap<usize, ClassDraws> = HashMap::new();
    for &ci in &used {
        let c = &m.classes[ci];
        let high = build_parts(c, &c.class.high_lod, false, glow_from(c, false), meshes);
        let low = if lod_on && !c.class.low_lod.is_empty() { build_parts(c, &c.class.low_lod, c.class.header.low_lod_joint_count == 0, glow_from(c, true), meshes).0 } else { Vec::new() };
        let metal = build_metal_parts(c, meshes);
        st.meshes += high.0.len() + low.len() + metal.0.len();
        parts.insert(ci, (high, low, metal));
    }

    // CPU colour table: every colour set flattened (packet by packet), when computed.
    let mut cpu_bytes = Vec::new();
    let set_base: Vec<u32> = m
        .colors
        .iter()
        .map(|(_, c)| {
            let b = (cpu_bytes.len() / 4) as u32;
            for rgba in c.iter().flatten() { cpu_bytes.extend_from_slice(rgba); }
            b
        })
        .collect();
    if cpu_bytes.is_empty() { cpu_bytes = vec![0; 16]; }

    // Instance records (index = gameplay instance index = MeshTag), palette slots and animation states.
    let animate = moby_anim::anim_enabled() && !m.cpu_light;
    let a = axes();
    let mut rec = Vec::with_capacity(m.instances.len() * RECORD_SIZE);
    let mut anims = Vec::new();
    let mut anim_index = vec![None; m.instances.len()];
    let mut palette_len = 0u32;
    for (ii, (inst, p)) in m.instances.iter().zip(&m.placed).enumerate() {
        let Some(p) = p else { write_record(&mut rec, &Mat4::ZERO, &GpuLights::default(), [0; 4]); continue };
        let r = Mat3::from_cols(Vec3::from(p.rows[0]), Vec3::from(p.rows[1]), Vec3::from(p.rows[2]));
        let model = Mat4::from_mat3_translation(a * r * (p.scale / 1024.0), game_to_bevy(inst.position));
        let ac = &m.anim[p.class];
        let slots_of = |ci: usize| {
            let (_, _, (_, metal_joint)) = &parts[&ci];
            let max_joint = parts[&ci].0 .1.max(*metal_joint);
            ((m.anim[ci].joint_count as u32).max(max_joint as u32 + 1).max(1), max_joint)
        };
        let (own, max_joint) = slots_of(p.class);
        // The palette range fits every class the instance may take.
        let slots = swap_to[ii].iter().map(|&ci| slots_of(ci).0).fold(own, u32::max);
        if (max_joint as usize) >= ac.joint_count.max(1) { st.joint_past_count += 1; }
        let (mode, cpu_base) = match (m.cpu_light, p.colors, &p.lights) {
            (true, Some(c), _) => (MODE_CPU_TABLE, set_base[c]),
            (_, _, None) => (MODE_UNLIT, 0),
            _ => (MODE_GPU_LIGHT, 0),
        };
        let g = p.lights.as_ref().map(GpuLights::new).unwrap_or_default();
        write_record(&mut rec, &model, &g, [palette_len, mode, cpu_base, 0]);
        let state = AnimState::spawn(ac);
        if ac.sequence(0).is_none() && ac.joint_count > 0 { st.no_seq0 += 1; }
        if state.speed == 0.0 { st.static_ += 1; } else { st.animated += 1; }
        anim_index[ii] = Some(anims.len());
        anims.push(AnimInstance { class: p.class, base: palette_len, slots, state, visible: true });
        palette_len += slots;
    }
    st.palette = palette_len;
    // MAIN_WORLD too: the scheduler-driven instances' records are rewritten per tick (MobyOcclusion::drive).
    let inst_buffer = buffers.add(ShaderBuffer::new(&rec, RenderAssetUsages::default()));
    let palette = buffers.add(ShaderBuffer::new(&moby_anim::identity_palette(palette_len), RenderAssetUsages::default()));
    let table_bytes: Vec<u8> = match &m.lighting {
        Some(l) => l.table.0.iter().flatten().flat_map(|w| w.to_le_bytes()).collect(),
        None => vec![0; 256 * 8],
    };
    let normal_table = buffers.add(ShaderBuffer::new(&table_bytes, RenderAssetUsages::RENDER_WORLD));
    let cpu_colors = buffers.add(ShaderBuffer::new(&cpu_bytes, RenderAssetUsages::RENDER_WORLD));
    // MAIN_WORLD too: rewritten per frame at the same size (update_moby_occlusion).
    let lod_bytes = moby_lod::default_lod_records(m.instances.len());
    let lods = buffers.add(ShaderBuffer::new(&lod_bytes, RenderAssetUsages::default()));
    if m.cpu_light && moby_anim::anim_enabled() { println!("mobys: RC_MOBY_CPU_LIGHT=1 draws the bind pose (the CPU colours are for the identity palette)"); }
    commands.insert_resource(MobyAnim::new(animate, anims, palette.clone(), palette_len));

    let mut cache = MatCache::new(MatProto {
        instances: inst_buffer.clone(),
        palette: palette.clone(),
        normal_table: normal_table.clone(),
        cpu_colors: cpu_colors.clone(),
        lods: lods.clone(),
    });
    let mut metal_mats: HashMap<(i32, GsPass, bool), Handle<MobyMetalMaterial>> = HashMap::new();
    let mut metal_imgs: HashMap<i32, (Handle<Image>, AlphaRange)> = HashMap::new();

    // Sanity bounds: the tfrag bounding spheres (integer units, /1024), padded by 50 units.
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for t in &level.tfrags {
        let s = t.header.bsphere.map(|v| v / 1024.0);
        lo = lo.min(Vec3::new(s[0], s[1], s[2]) - s[3]);
        hi = hi.max(Vec3::new(s[0], s[1], s[2]) + s[3]);
    }
    let (lo, hi) = (lo - 50.0, hi + 50.0);

    let mut metal_batches = 0usize;
    let mut draws: Vec<InstanceDraws> = Vec::with_capacity(m.instances.len());
    for (ii, (inst, p)) in m.instances.iter().zip(&m.placed).enumerate() {
        let Some(p) = p else { st.no_class += 1; draws.push(InstanceDraws::default()); continue };
        st.placed += 1;
        let class = &m.classes[p.class];
        let pos = Vec3::from(inst.position);
        if pos.cmplt(lo).any() || pos.cmpgt(hi).any() { st.outside += 1; }
        let r = Mat3::from_cols(Vec3::from(p.rows[0]), Vec3::from(p.rows[1]), Vec3::from(p.rows[2]));
        let transform = Transform::from_matrix(Mat4::from_mat3_translation(a * r * (p.scale / 1024.0), game_to_bevy(inst.position)));
        let ((high, _), low, _) = &parts[&p.class];
        let mut d = InstanceDraws {
            input: Some(ProcInput {
                position: inst.position,
                rows: p.rows,
                scale: p.scale,
                draw_distance: inst.draw_distance as i16 as i32,
                lod_trans: class.class.header.lod_trans,
                shine_distance: if class.class.header.metal_count > 0 { moby_lod::SHINE_DISTANCE } else { 0 },
                alpha: 0x80,
            }),
            class: p.class,
            class_sphere: class.class.header.bsphere,
            translation: transform.translation,
            ..default()
        };
        // The eager groups: high and low LOD, each plain and fading (the other MobyBlend groups are spawned when an
        // instance first needs them, update_moby_occlusion).
        for (low, list) in [(false, high), (true, low)] {
            if low && !lod_on { break; }
            if low { st.low_triangles += list.iter().map(|p| p.triangles).sum::<usize>(); } else {
                st.triangles += list.iter().map(|p| p.triangles).sum::<usize>();
                st.grey += list.iter().filter(|p| p.texture == GREY).map(|p| p.triangles).sum::<usize>();
            }
            for blend in [MobyBlend::Plain, MobyBlend::Fading] {
                if blend == MobyBlend::Fading && !lod_on { continue; }
                let vis = if !low && blend == MobyBlend::Plain { Visibility::Inherited } else { Visibility::Hidden };
                let name = format!("moby {ii} class {} {} {blend:?}", inst.o_class, if low { "low" } else { "high" });
                let ents = cache.spawn(commands, level, list, blend, true, transform, ii as u32, vis, &name, images, materials);
                st.group_entities[(low as usize) * 2 + (blend == MobyBlend::Fading) as usize] += ents.len();
                st.entities += ents.len();
                d.groups.insert(GroupKey { low, blend }, ents);
            }
        }
        // The metal entities of the placed class, then of each class it may become (kept hidden in `other`).
        for (n, &ci) in std::iter::once(&p.class).chain(&swap_to[ii]).enumerate() {
            let cls = &m.classes[ci];
            let (_, _, (metal, _)) = &parts[&ci];
            let mut ents = Vec::new();
            for part in metal.iter() {
                if part.kind == TEX_CHROME { st.chrome += part.triangles } else { st.glass += part.triangles }
                let (image, texel_alpha) = metal_imgs.entry(part.kind).or_insert_with(|| metal_image(level, part.kind, images)).clone();
                for pass in metal_passes(texel_alpha) {
                    let mat = metal_mats
                        .entry((part.kind, pass, part.caster))
                        .or_insert_with(|| {
                            metal_materials.add(MobyMetalMaterial {
                                texture: image.clone(),
                                fog: crate::game_camera::fog_buffer(),
                                instances: inst_buffer.clone(),
                                palette: palette.clone(),
                                normal_table: normal_table.clone(),
                                lods: lods.clone(),
                                pass,
                                caster: part.caster,
                                order: None,
                            })
                        })
                        .clone();
                    metal_batches += 1;
                    let e = commands.spawn((
                        Mesh3d(part.mesh.clone()),
                        MeshMaterial3d(mat),
                        transform,
                        MeshTag(ii as u32),
                        NoFrustumCulling,
                        Visibility::Hidden,
                        Name::new(format!("moby {ii} class {} metal {} {pass:?}", cls.o_class, part.kind)),
                    ));
                    ents.push(e.id());
                    st.metal_entities += 1;
                    st.entities += 1;
                }
            }
            if n == 0 {
                d.metal = ents;
            } else {
                let h = &cls.class.header;
                d.other.push(OtherClass { class: ci, class_sphere: h.bsphere, lod_trans: h.lod_trans, groups: BTreeMap::new(), metal: ents });
            }
        }
        // Triangles on an unused class slot (0xff): not drawn (none on the disc, docs/plan/moby_untextured.md).
        for sub in &class.class.high_lod {
            st.untextured += sub.triangles.iter().filter(|t| t.texture >= 0 && class.texture_table_index(t.texture).is_none()).count();
        }
        draws.push(d);
    }
    let shown = draws.iter().map(|d| d.input.map(|_| Shown { group: GroupKey { low: false, blend: MobyBlend::Plain }, metal: false, late: true })).collect();
    let n_inst = m.instances.len();
    st.batches = cache.batches.len() + metal_batches;
    st.images = cache.images.len() + metal_imgs.len();
    commands.insert_resource(MobyOcclusion {
        records: rec,
        instances: inst_buffer.clone(),
        driven_hidden: vec![false; n_inst],
        records_dirty: false,
        look: m.placed.iter().map(|p| p.map_or(MobyLook { alpha: 0x80, mode: 0, glow: 0, shine_distance: 0, draw_dist: None, late: None }, |p| MobyLook::of_class(&m.classes[p.class]))).collect(),
        moved: vec![false; n_inst],
        lit_by_point: vec![false; n_inst],
        class_parts: parts.into_iter().map(|(ci, (high, low, _))| (ci, [high.0, low])).collect(),
        cache,
        bits: level.occlusion.objects.moby.clone(),
        loader_bits: level.occlusion.objects.moby.clone(),
        anim_index,
        draws,
        shown,
        lods,
        last_lod: lod_bytes,
        lod_on,
        tint: moby_lod::tint_enabled(),
        metal_on: moby_lod::metal_enabled(),
        last_hist: None,
        last_print: f32::MIN,
        pending_hide: Vec::new(),
    });
    st.build = t0.elapsed();
    st
}

/// One placed instance's entities and MobyProc inputs.
#[derive(Default)]
struct InstanceDraws {
    /// None: the class has no geometry (nothing spawned).
    input: Option<ProcInput>,
    class: usize,
    /// Class header 0x30 (packed units): the sphere when a sequence is missing.
    class_sphere: [f32; 4],
    /// The entity groups spawned so far, by LOD and GS state.
    groups: BTreeMap<GroupKey, Vec<Entity>>,
    metal: Vec<Entity>,
    /// The translation the entities carry (the Transparent3d sort key), updated when a driven instance moves.
    translation: Vec3,
    /// The other classes the instance may take at run time (a class swap, [`MobyOcclusion::set_class`]): their
    /// entities, kept hidden, swapped with the fields above when the moby's class changes.
    other: Vec<OtherClass>,
}

/// A class a placed instance is not showing now ([`InstanceDraws::other`]): what [`InstanceDraws`] holds per class.
struct OtherClass {
    class: usize,
    class_sphere: [f32; 4],
    /// Class +0x0e (MobyProc's LOD switch).
    lod_trans: u8,
    groups: BTreeMap<GroupKey, Vec<Entity>>,
    metal: Vec<Entity>,
}

/// One entity group of an instance: LOD and GS state ([`MobyBlend`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct GroupKey {
    low: bool,
    blend: MobyBlend,
}

/// What an instance shows: its entity group and whether the metal pass is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shown {
    group: GroupKey,
    metal: bool,
    /// Its caster draws late ([`MobyLook::late`]).
    late: bool,
}

/// What MobyProc reads from a moby besides its placement: +0x23 (alpha), +0x34 (mode bits: the blend, [`MobyBlend`], and
/// the glow list, bit 0x10), +0x90 (glow colour) and +0x73 (the shine distance, 0 = no metal pass).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobyLook {
    pub alpha: u8,
    pub mode: u16,
    pub glow: u32,
    pub shine_distance: u8,
    /// +0x32, the draw distance MobyProc reads (`InitMobyInstance` sets it from the instance's; updates rewrite it:
    /// the convoys' 0x200). None: the instance's own (an undriven static).
    pub draw_dist: Option<i16>,
    /// Deferred by `MobyProc` (mode 0x400 with a live shadow, or 0x800): a caster's draws after the shadow pass
    /// ([`CasterTwin`]). None: as its class (deferred when it has a shadow block; an undriven static).
    pub late: Option<bool>,
}

impl MobyLook {
    /// A fresh moby of `class` as `InitMobyInstance` (level01 0x263488) leaves it: alpha 0x80; mode 0x10 and +0x90 =
    /// class +0x40 when that is non-zero; +0x73 = 0x18 when the class has metal packets. The other mode bits are not
    /// taken (the undriven statics keep the blend they had).
    pub fn of_class(class: &LevelMobyClass) -> Self {
        let h = &class.class.header;
        let glow = h.glow_rgba != 0;
        MobyLook {
            alpha: 0x80,
            mode: if glow { 0x10 } else { 0 },
            glow: h.glow_rgba as u32,
            shine_distance: if h.metal_count > 0 { moby_lod::SHINE_DISTANCE } else { 0 },
            draw_dist: None,
            late: None,
        }
    }
}

/// Per static gameplay instance: its occlusion word, its entities, its animation slot and what it shows; the
/// `MobyLod` buffer and the frame's pick statistics.
#[derive(Resource)]
pub struct MobyOcclusion {
    /// The `MobyInst` records (CPU copy of `instances`) and a pending rewrite (scheduler-driven instances).
    records: Vec<u8>,
    instances: Handle<ShaderBuffer>,
    /// Per instance: hidden by the moby loop (deleted, or mode & 1): MobyProc skips it.
    driven_hidden: Vec<bool>,
    /// Per instance: what MobyProc reads from the moby's runtime struct besides its placement, set by
    /// [`MobyOcclusion::look`].
    look: Vec<MobyLook>,
    /// Per instance: the driven position moved since the entities' translation was last written.
    moved: Vec<bool>,
    /// Per instance: its record holds a merged point light (cleared when no light reaches it any more).
    lit_by_point: Vec<bool>,
    /// Per class: high and low LOD parts (for the groups spawned on first use).
    class_parts: HashMap<usize, [Vec<Part>; 2]>,
    cache: MatCache,
    records_dirty: bool,
    bits: Vec<OcclBits>,
    /// The loader's words (`bits` as resolved at load), what a moby +0x36 of 0 stands for.
    loader_bits: Vec<OcclBits>,
    anim_index: Vec<Option<usize>>,
    draws: Vec<InstanceDraws>,
    shown: Vec<Option<Shown>>,
    lods: Handle<ShaderBuffer>,
    last_lod: Vec<u8>,
    lod_on: bool,
    tint: bool,
    metal_on: bool,
    last_hist: Option<[usize; 9]>,
    last_print: f32,
    /// Entities of a class an instance stopped showing ([`MobyOcclusion::set_class`]), hidden by the next
    /// `update_moby_occlusion`.
    pending_hide: Vec<Entity>,
}

/// The main camera with its projection (the view tangent MobyProc's frustum cull takes).
type MainCameraView<'w, 's> = Query<'w, 's, (&'static Transform, Option<&'static Projection>), (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>;

/// `MobyProc` for every placed instance (module doc): the occlusion test (after the "dead" check, which no
/// static instance fails in the port), then crate::moby_lod, then the GS state ([`MobyBlend`], from moby+0x23 /
/// +0x34 and the fade). Changes `Visibility` only when an instance's pick changes (spawning a group the first time
/// it is needed), moves a driven instance's entities with it (the blended phase sorts by their translation), and
/// rewrites the `MobyLod` buffer when it changes.
#[allow(clippy::too_many_arguments)]
pub fn update_moby_occlusion(
    mut commands: Commands,
    state: Option<ResMut<MobyOcclusion>>,
    occl: Option<ResMut<crate::occlusion::OcclusionFrame>>,
    mut anim: Option<ResMut<MobyAnim>>,
    level: Res<crate::Level>,
    cams: MainCameraView,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut ents_q: Query<(&mut Visibility, &mut Transform), Without<Camera3d>>,
    time: Res<Time<Real>>,
    point_lights: Res<PointLightFrame>,
) {
    let Some(mut state) = state else { return };
    let mask = occl.as_ref().map(|o| o.mask);
    let tans = crate::game_camera::projection_tans(cams.iter().next().and_then(|(_, p)| p));
    let cam = cams.iter().next().map(|(t, _)| {
        let [fwd, left, up] = crate::game_camera::game_rows(t);
        (crate::game_camera::game_eye(t), moby_lod::camera_rows(fwd, left, up))
    });
    let classes = &level.0.mobys.anim;
    let s = &mut *state;
    // The entities of the classes instances stopped showing (a class swap, MobyOcclusion::set_class).
    for e in std::mem::take(&mut s.pending_hide) {
        if let Ok((mut v, _)) = ents_q.get_mut(e) { *v = Visibility::Hidden; }
    }
    let mut lod_bytes = vec![0u8; s.last_lod.len()];
    // hist: occluded, culled (distance, near, frustum), high, high fading, low, low fading, metal.
    let mut hist = [0usize; 9];
    for (ii, d) in s.draws.iter_mut().enumerate() {
        let rec = &mut lod_bytes[ii * moby_lod::LOD_RECORD_SIZE..(ii + 1) * moby_lod::LOD_RECORD_SIZE];
        let Some(mut inp) = d.input else { moby_lod::write_lod_record(rec, 0x80, 0, 0, 0, &[[0.0; 3]; 3]); continue };
        let look = s.look[ii];
        let mode = look.mode;
        inp.alpha = look.alpha;
        inp.shine_distance = look.shine_distance;
        if let Some(dd) = look.draw_dist { inp.draw_distance = dd as i32; }
        let k = s.anim_index[ii];
        let late = look.late.unwrap_or(true);
        let (want, pick, sphere) = if s.driven_hidden[ii] {
            (None, None, None)
        } else if mask.as_ref().is_some_and(|m| !s.bits[ii].visible(m)) {
            hist[0] += 1;
            (None, None, None)
        } else if let Some((eye, rows)) = cam {
            let seq = match k.zip(anim.as_ref()) {
                Some((k, a)) => moby_lod::seq_sphere(&classes[d.class], &a.instances[k].state, d.class_sphere),
                None => d.class_sphere,
            };
            let sphere = moby_lod::world_sphere(&inp, seq);
            let v = moby_lod::view_centre(sphere, eye, &rows);
            let result = if s.lod_on {
                moby_lod::moby_proc_view(v, sphere[3], &inp, tans)
            } else {
                Ok(ProcPick { low_lod: false, alpha: inp.alpha, fading: false, shine: moby_lod::shine_alpha(moby_lod::sphere_depth(v, sphere[3]), inp.shine_distance) })
            };
            match result {
                Err(c) => {
                    hist[match c { moby_lod::Cull::DrawDistance => 1, moby_lod::Cull::Near => 2, moby_lod::Cull::Frustum => 3 }] += 1;
                    (None, None, None)
                }
                Ok(p) => {
                    let group = GroupKey { low: p.low_lod, blend: MobyBlend::pick(mode, p.fading, p.alpha) };
                    let hidden = k.zip(anim.as_ref()).is_some_and(|(k, a)| a.hidden[k]);
                    let metal = s.metal_on && p.shine > 0 && !d.metal.is_empty() && !hidden;
                    hist[4 + (p.low_lod as usize) * 2 + (group.blend != MobyBlend::Plain) as usize] += 1;
                    if metal { hist[8] += 1; }
                    (Some(Shown { group, metal, late }), Some(p), Some((sphere, eye, rows)))
                }
            }
        } else {
            let blend = MobyBlend::pick(mode, false, inp.alpha);
            (Some(Shown { group: GroupKey { low: false, blend }, metal: false, late }), None, None)
        };
        let (alpha, flags, shine, e) = match (pick, want) {
            (Some(p), Some(w)) => {
                let e = match (w.metal, sphere) {
                    (true, Some((sphere, eye, rows))) => moby_lod::shine_basis(sphere, eye, &rows, &inp.rows),
                    _ => [[0.0; 3]; 3],
                };
                (p.alpha, if s.tint && p.low_lod { moby_lod::FLAG_TINT } else { 0 }, if w.metal { p.shine } else { 0 }, e)
            }
            (None, Some(_)) => (inp.alpha, 0, 0, [[0.0; 3]; 3]),
            _ => (0x80, 0, 0, [[0.0; 3]; 3]),
        };
        moby_lod::write_lod_record(rec, alpha, flags, shine, moby_lod::glow_word(mode, look.glow), &e);
        // The point lights in range of the sphere centre become the third light (none: cleared).
        if !point_lights.0.is_empty() || s.lit_by_point[ii] {
            let merged = sphere.and_then(|(sp, _, _)| point_light_merge(&point_lights.0, [sp[0] / crate::game_camera::UNITS, sp[1] / crate::game_camera::UNITS, sp[2] / crate::game_camera::UNITS], &inp.rows));
            s.lit_by_point[ii] = merged.is_some();
            let at = ii * RECORD_SIZE;
            if let Some(r) = s.records.get_mut(at..at + RECORD_SIZE) {
                if write_point_light(r, merged) { s.records_dirty = true; }
            }
        }

        // The group this pick shows, spawned on first use.
        if let Some(w) = want {
            if !d.groups.contains_key(&w.group) {
                let parts = s.class_parts.get(&d.class).map(|p| &p[w.group.low as usize][..]).unwrap_or(&[]);
                let name = format!("moby {ii} class {} {} {:?}", d.class, if w.group.low { "low" } else { "high" }, w.group.blend);
                let t = Transform::from_translation(d.translation);
                let ents = s.cache.spawn(&mut commands, &level.0, parts, w.group.blend, w.late, t, ii as u32, Visibility::Hidden, &name, &mut images, &mut materials);
                d.groups.insert(w.group, ents);
            }
        }
        let was = s.shown[ii];
        let moved = std::mem::take(&mut s.moved[ii]);
        if moved {
            if let Some(ents) = want.and_then(|w| d.groups.get(&w.group)) {
                for &e in ents {
                    if let Ok((_, mut t)) = ents_q.get_mut(e) { t.translation = d.translation; }
                }
            }
        }
        if want == was { continue; }
        s.shown[ii] = want;
        for (g, ents) in d.groups.iter() {
            let (on, before) = (want.is_some_and(|w| w.group == *g), was.is_some_and(|w| w.group == *g));
            if on == before { continue; }
            for &e in ents {
                // A group spawned this frame is not in the world yet: its commands follow.
                match ents_q.get_mut(e) {
                    Ok((mut v, mut t)) => {
                        *v = if on { Visibility::Inherited } else { Visibility::Hidden };
                        if on { t.translation = d.translation; }
                    }
                    Err(_) => {
                        if on { commands.entity(e).insert(Visibility::Inherited); }
                    }
                }
            }
        }
        // The caster draws' order of the group shown (a group spawned this frame starts in it).
        if let Some(w) = want.filter(|w| was.is_none_or(|v| v.group != w.group || v.late != w.late)) {
            if let Some(ents) = d.groups.get(&w.group) { queue_late(&mut commands, ents.clone(), w.late); }
        }
        let (on, before) = (want.is_some_and(|w| w.metal), was.is_some_and(|w| w.metal));
        if on != before {
            for &e in &d.metal {
                if let Ok((mut v, mut t)) = ents_q.get_mut(e) {
                    *v = if on { Visibility::Inherited } else { Visibility::Hidden };
                    if on { t.translation = d.translation; }
                }
            }
        }
        if want.is_some() != was.is_some() {
            if let (Some(a), Some(k)) = (anim.as_mut(), k) {
                a.instances[k].visible = want.is_some();
                if want.is_some() { a.pose_dirty = true; }
            }
        }
    }
    if lod_bytes != s.last_lod {
        if let Some(mut buf) = buffers.get_mut(&s.lods) { buf.data = Some(lod_bytes.clone()); }
        s.last_lod = lod_bytes;
    }
    if std::mem::take(&mut s.records_dirty) {
        if let Some(mut buf) = buffers.get_mut(&s.instances) { buf.data = Some(s.records.clone()); }
    }
    if let Some(mut o) = occl {
        o.mobys = crate::occlusion::CullCounts { occluded: hist[0], culled: hist[1] + hist[2] + hist[3], drawn: hist[4..8].iter().sum() };
    }
    let now = time.elapsed_secs();
    if s.last_hist != Some(hist) && now - s.last_print >= 1.0 && cam.is_some() {
        s.last_print = now;
        s.last_hist = Some(hist);
        println!(
            "moby LODs: occluded {}, culled {} (draw distance {}, near {}, frustum {}), high {} (+{} fading/translucent), low {} (+{} fading/translucent), metal pass {}",
            hist[0], hist[1] + hist[2] + hist[3], hist[1], hist[2], hist[3], hist[4], hist[5], hist[6], hist[7], hist[8]
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type P = [f64; 3];
    fn sub(a: P, b: P) -> P { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
    fn cross(a: P, b: P) -> P { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
    fn dot(a: P, b: P) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }

    /// Two 2D triangles share interior area (touching edges or corners do not count): no separating axis
    /// among the six edge normals.
    fn overlap_2d(a: &[[f64; 2]; 3], b: &[[f64; 2]; 3]) -> bool {
        for t in [a, b] {
            for i in 0..3 {
                let (p, q) = (t[i], t[(i + 1) % 3]);
                let n = [q[1] - p[1], p[0] - q[0]];
                let proj = |s: &[[f64; 2]; 3]| {
                    let v = s.map(|x| x[0] * n[0] + x[1] * n[1]);
                    (v[0].min(v[1]).min(v[2]), v[0].max(v[1]).max(v[2]))
                };
                let ((a0, a1), (b0, b1)) = (proj(a), proj(b));
                if a1 <= b0 || b1 <= a0 { return false; }
            }
        }
        true
    }

    /// Bind-pose (packed position) triangle pairs of one class that lie in one plane (every vertex of each
    /// within `tol` packed units of the other's plane) and overlap with positive area, from different
    /// texture batches (texture −1 is the grey batch). Returns (pairs, distinct (earlier batch, later batch)
    /// edges in game draw order, whether those edges admit one batch order (no cycle)).
    fn cross_batch_overlaps(class: &LevelMobyClass, tol: f64) -> (usize, Vec<(usize, usize)>, bool) {
        let mut tris: Vec<(usize, [P; 3])> = Vec::new();
        for sub in &class.class.high_lod {
            for t in &sub.triangles {
                let Some(tex) = part_texture(class, t.texture) else { continue };
                let p = [t.a, t.b, t.c].map(|i| sub.vertices[i as usize].packed_position().map(|c| c as f64));
                tris.push((tex, p));
            }
        }
        let on_plane = |t: &[P; 3], s: &[P; 3]| {
            let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            let len = dot(n, n).sqrt();
            len > 0.0 && s.iter().all(|&v| (dot(n, sub(v, t[0])) / len).abs() <= tol).then_some(n).is_some()
        };
        let (mut pairs, mut edges) = (0, std::collections::BTreeSet::new());
        for i in 0..tris.len() {
            for j in i + 1..tris.len() {
                let ((ba, ta), (bb, tb)) = (&tris[i], &tris[j]);
                if ba == bb || !on_plane(ta, tb) || !on_plane(tb, ta) { continue; }
                let n = cross(sub(ta[1], ta[0]), sub(ta[2], ta[0]));
                let k = (0..3).max_by(|&x, &y| n[x].abs().total_cmp(&n[y].abs())).unwrap();
                let (u, v) = ((k + 1) % 3, (k + 2) % 3);
                let flat = |t: &[P; 3]| t.map(|p| [p[u], p[v]]);
                if overlap_2d(&flat(ta), &flat(tb)) {
                    pairs += 1;
                    edges.insert((*ba, *bb)); // j is later in the game's packet order: its batch wins under GEQUAL
                }
            }
        }
        let edges: Vec<(usize, usize)> = edges.into_iter().collect();
        let acyclic = !edges.iter().any(|&(a, b)| edges.contains(&(b, a))) && {
            // Kahn's algorithm over the batch graph.
            let nodes: std::collections::BTreeSet<usize> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
            let mut indeg: HashMap<usize, usize> = nodes.iter().map(|&n| (n, 0)).collect();
            for &(_, b) in &edges { *indeg.get_mut(&b).unwrap() += 1; }
            let mut ready: Vec<usize> = indeg.iter().filter(|e| *e.1 == 0).map(|e| *e.0).collect();
            let mut seen = 0;
            while let Some(n) = ready.pop() {
                seen += 1;
                for &(a, b) in &edges {
                    if a == n { let d = indeg.get_mut(&b).unwrap(); *d -= 1; if *d == 0 { ready.push(b); } }
                }
            }
            seen == nodes.len()
        };
        (pairs, edges, acyclic)
    }

    /// MobyProc's per-moby choice: mode 0x200 wins (additive), then the fade / mode bit 8, then the alpha byte.
    #[test]
    fn moby_blend_pick_and_draws() {
        assert_eq!(MobyBlend::pick(0, false, 0x80), MobyBlend::Plain);
        assert_eq!(MobyBlend::pick(0, false, 0x30), MobyBlend::Translucent);
        assert_eq!(MobyBlend::pick(0, true, 0x30), MobyBlend::Fading);
        assert_eq!(MobyBlend::pick(8, false, 0x80), MobyBlend::Fading);
        assert_eq!(MobyBlend::pick(0x200, true, 0x80), MobyBlend::Additive);
        let o = AlphaRange::OPAQUE;
        assert_eq!(MobyBlend::Plain.passes(o, o), vec![GsPass::Opaque]);
        assert_eq!(MobyBlend::Translucent.passes(o, o), vec![GsPass::EffectMix]);
        assert_eq!(MobyBlend::Additive.passes(o, o), vec![GsPass::AdditiveNoZ]);
        assert!(!GsPass::EffectMix.state().depth_write && !GsPass::AdditiveNoZ.state().depth_write);
        assert!(GsPass::EffectMix.state().display && GsPass::AdditiveNoZ.state().display && !GsPass::BlendNoZ.state().display);
        assert!(GsPass::AdditiveNoZ.state().additive);
    }

    /// The point-light merge: out of range → none; one light → its colour × (1 − δ/R) along the model-space
    /// direction to it; two → the weighted sum, direction renormalised.
    #[test]
    fn point_light_merge_weights() {
        use rc_game::point_lights::PointLight;
        let id = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let l = PointLight { color: [2.0, 1.0, 0.0], intensity: 0.0, pos: [5.0, 0.0, 0.0], radius: 10.0 };
        assert_eq!(point_light_merge(&[l], [20.0, 0.0, 0.0], &id), None);
        let (d, c) = point_light_merge(&[l], [0.0, 0.0, 0.0], &id).unwrap();
        assert_eq!(d, [1.0, 0.0, 0.0]);
        assert_eq!(c, [1.0, 0.5, 0.0]);
        // A moby turned 90° about z (model x = world y): the light at world +x is at model −y.
        let rot = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let (d, _) = point_light_merge(&[l], [0.0; 3], &rot).unwrap();
        assert!((d[0]).abs() < 1e-6 && (d[1] + 1.0).abs() < 1e-6, "{d:?}");
        let l2 = PointLight { pos: [0.0, 5.0, 0.0], ..l };
        let (d, c) = point_light_merge(&[l, l2], [0.0; 3], &id).unwrap();
        assert!((d[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6 && (d[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert_eq!(c, [2.0, 1.0, 0.0]);
        let mut rec = vec![0u8; RECORD_SIZE];
        assert!(write_point_light(&mut rec, Some(([1.0, 0.0, 0.0], [1.0, 0.5, 0.0]))));
        assert_eq!(f32::from_le_bytes(rec[72..76].try_into().unwrap()), 1.0);
        assert_eq!(f32::from_le_bytes(rec[148..152].try_into().unwrap()), 0.5);
        assert!(!write_point_light(&mut rec, Some(([1.0, 0.0, 0.0], [1.0, 0.5, 0.0]))));
    }

    /// A glow part's soft edge (the TEST_1 fail half) blends on display bytes; its Z-writing half and every draw of a
    /// plain part are unchanged; a caster's Z-writing draws still move late.
    #[test]
    fn glow_parts_blend_their_soft_edge_on_display_bytes() {
        let part = |glow, caster| Part { texture: 0, glow, mesh: Handle::default(), mult_alpha: AlphaRange::OPAQUE, triangles: 1, caster };
        let (lo, hi) = (GsPass::ColorOnlyLowAlpha { aref: 0x60 }, GsPass::OpaqueTested { aref: 0x60 });
        assert_eq!(part_pass(&part(true, false), lo), GsPass::EffectLowAlpha { aref: 0x60 });
        assert_eq!(part_pass(&part(true, false), hi), hi);
        assert_eq!(part_pass(&part(false, false), lo), lo);
        assert_eq!(part_pass(&part(true, true), hi), GsPass::LateTested { aref: 0x60 });
        let s = GsPass::EffectLowAlpha { aref: 0x60 }.state();
        assert!(s.display && s.blend && !s.additive && !s.depth_write && s.discard == gs_state::AlphaDiscard::AtOrAbove(0x60));
    }

    /// The record's placement comes back from its model, at the item's own scale (not its class's).
    #[test]
    fn record_placement_round_trips() {
        let (c, s) = (0.6f32.cos(), 0.6f32.sin());
        let rows = [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]];
        let rec = extra_record(&extra_model(rows, 0.034, [170.0, 140.4, 63.4]), None, 0);
        let (r, p, k) = record_placement(&rec[..64]).unwrap();
        assert!((k - 0.034).abs() < 1e-6, "{k}");
        assert!(p.iter().zip([170.0, 140.4, 63.4]).all(|(a, b)| (a - b).abs() < 1e-4), "{p:?}");
        assert!(r.iter().flatten().zip(rows.iter().flatten()).all(|(a, b)| (a - b).abs() < 1e-5), "{r:?}");
        assert!(record_placement(&[0u8; 64]).is_none());
    }

    /// The glow packets (class byte 0xa on) of the vendor 11 and the floor switch 830 are their own parts with every
    /// vertex flagged, the other parts none. Needs the disc data (levels 01 and 05); skipped without it.
    #[test]
    fn glow_packets_are_flagged() {
        let root = crate::level_load::extracted_root();
        for (level, o) in [(1u32, 11), (1, 0), (5, 830), (5, 79)] {
            let (Ok(index), Ok(data)) = (crate::disc_source::level_file(&root, level, "core_index.bin"), crate::disc_source::level_file(&root, level, "core_data.bin")) else {
                eprintln!("skipped: no level {level:02} data");
                return;
            };
            let data = rc_formats::wad::decompress(&data).unwrap();
            let core = rc_formats::level::parse_level_core(&index, data.len()).unwrap();
            let classes = moby::parse_level_mobys(&core, &data).unwrap();
            let c = classes.iter().find(|c| c.o_class == o).expect("class on the level");
            let from = glow_from(c, false);
            let want: usize = c.class.high_lod.iter().skip(from).map(|s| s.triangles.iter().filter(|t| part_texture(c, t.texture).is_some()).count()).sum();
            let mut meshes = Assets::<Mesh>::default();
            let (parts, _) = build_parts(c, &c.class.high_lod, false, from, &mut meshes);
            assert!(want > 0 && c.class.header.glow_rgba != 0, "class {o}: glow packets from {from}");
            assert_eq!(parts.iter().filter(|p| p.glow).map(|p| p.triangles).sum::<usize>(), want, "class {o}");
            for p in &parts {
                let Some(VertexAttributeValues::Uint32x4(v)) = meshes.get(&p.mesh).unwrap().attribute(ATTRIBUTE_MOBY_SKIN) else { panic!("skin") };
                assert!(v.iter().all(|w| (w[0] & SKIN_GLOW != 0) == p.glow), "class {o}");
            }
        }
    }

    /// Measurement for the within-class draw order question (gs_state.rs "Packet order"): how many
    /// coplanar, overlapping triangle pairs of Ratchet (class 0) and class 577 come from different texture
    /// batches. Needs the disc data; skipped without it.
    #[test]
    fn moby_cross_batch_coplanar_overlaps() {
        let root = crate::level_load::extracted_root();
        let (Ok(index), Ok(data)) = (crate::disc_source::level_file(&root, 1, "core_index.bin"), crate::disc_source::level_file(&root, 1, "core_data.bin")) else {
            eprintln!("skipped: no level 01 data");
            return;
        };
        let data = rc_formats::wad::decompress(&data).unwrap();
        let core = rc_formats::level::parse_level_core(&index, data.len()).unwrap();
        let classes = moby::parse_level_mobys(&core, &data).unwrap();
        for o in [0, 577] {
            let c = classes.iter().find(|c| c.o_class == o).expect("class on Novalis");
            for tol in [0.0, 0.5, 2.0] {
                let (pairs, edges, acyclic) = cross_batch_overlaps(c, tol);
                println!("moby class {o}: tol {tol}: {pairs} coplanar overlapping cross-batch pairs, batch edges (earlier -> later) {edges:?}, one batch order possible: {acyclic}");
                // Measured 2026-09-26: none, so the class's batch order cannot change a pixel (gs_state.rs).
                assert_eq!(pairs, 0);
            }
        }
        // Level-wide (tol 0): 3 of 169 classes (725: 6 pairs, 731: 4, 790: 4), one batch edge each.
        let others: Vec<(i32, usize)> =
            classes.iter().map(|c| (c.o_class, cross_batch_overlaps(c, 0.0).0)).filter(|&(_, n)| n > 0).collect();
        println!("classes with cross-batch coplanar overlaps: {others:?} of {}", classes.len());
    }
}

// ===================================================================================================
// Extra (runtime-created) moby instances — crate::moby_attach. Separate from the gameplay instances above:
// the caller owns an instance-record buffer and a palette buffer of its own (both kept in the main world so
// it can rewrite them), and every part entity carries `MeshTag(slot)` into that record buffer. Everything
// else (meshes, textures, GS passes, shaders) is the gameplay path's. They are always drawn (high LOD, α 0x80, the
// class's glow colour); a class with metal packets also gets its shine pass (update_extra_metal; the dynamic slots'
// from their own MobyProc pick in show_slot).

/// Size of one `MobyInst` record (moby.wgsl).
pub const EXTRA_RECORD_SIZE: usize = RECORD_SIZE;

/// The record of one extra instance: `model` = game_to_bevy · [s/1024 · R | p], the light block (None =
/// unlit), the palette base.
pub fn extra_record(model: &Mat4, lights: Option<&MobyLights>, palette_base: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(RECORD_SIZE);
    let mode = if lights.is_some() { MODE_GPU_LIGHT } else { MODE_UNLIT };
    write_record(&mut out, model, &lights.map(GpuLights::new).unwrap_or_default(), [palette_base, mode, 0, 0]);
    out
}

/// `game_to_bevy · [s/1024 · R | p]` for rotation rows `rows` (row i = image of model axis i, game space).
pub fn extra_model(rows: [[f32; 3]; 3], scale: f32, position: [f32; 3]) -> Mat4 {
    let r = Mat3::from_cols(Vec3::from(rows[0]), Vec3::from(rows[1]), Vec3::from(rows[2]));
    Mat4::from_mat3_translation(axes() * r * (scale / 1024.0), game_to_bevy(position))
}

/// Shared state for spawning extra instances: the storage buffers and the per-texture images / materials.
pub struct ExtraMobys {
    pub instances: Handle<ShaderBuffer>,
    pub palette: Handle<ShaderBuffer>,
    normal_table: Handle<ShaderBuffer>,
    /// `MobyLod` per slot (vertex alpha from [`ExtraMobys::show_slot`], else 0x80; shine alpha and E written by
    /// update_extra_metal).
    lods: Handle<ShaderBuffer>,
    cache: MatCache,
    metal_images: HashMap<i32, (Handle<Image>, AlphaRange)>,
    /// High-LOD parts per class (`o_class`), built on first use.
    parts: HashMap<i32, Vec<Part>>,
    /// Metal parts per class (`o_class`), built on first use.
    metal_parts: HashMap<i32, Vec<MetalPart>>,
    /// [`ExtraMobys::show_slot`]: entities per (slot, class, blend), spawned on first use, and what each slot shows.
    slot_ents: HashMap<(u32, i32, MobyBlend), Vec<Entity>>,
    slot_shown: HashMap<u32, ((i32, MobyBlend), Vec3)>,
    /// [`ExtraMobys::show_slot`]: metal entities per (slot, class), spawned on first use, and the class whose metal a
    /// slot shows.
    slot_metal: HashMap<(u32, i32), Vec<Entity>>,
    slot_metal_shown: HashMap<u32, i32>,
    /// The caster order each slot's shown group has ([`queue_late`]).
    slot_late: HashMap<u32, bool>,
}

/// What a slot of an extra record set shows ([`ExtraMobys::show_slot`]).
#[derive(Clone, Copy, Debug)]
pub struct SlotLook {
    /// The moby's placement (`extra_model`): its translation is the blended phase's sort key.
    pub model: Mat4,
    /// moby+0x23 × the distance fade (the vertex alpha), MobyProc's `fading` and moby+0x34.
    pub alpha: u8,
    pub fading: bool,
    pub mode: u16,
    /// moby+0x90, the glow colour (drawn when `mode` has the glow bit 0x10).
    pub glow: u32,
    /// MobyProc's shine alpha (0: no metal pass; crate::moby_lod, from moby+0x73) and the sphere-map basis E.
    pub shine: u8,
    pub e: [[f32; 3]; 3],
    /// Deferred by `MobyProc` ([`MobyLook::late`]): a caster's draws after the shadow pass.
    pub late: bool,
}

/// A metal entity of an extra instance: where update_extra_metal reads its placement and writes its shine.
#[derive(Component, Clone)]
pub struct ExtraMetal {
    slot: u32,
    records: Handle<ShaderBuffer>,
    lods: Handle<ShaderBuffer>,
    /// Class header 0x30 (packed units): the sphere, as the item's sequence state is not visible here.
    sphere: [f32; 4],
}

impl ExtraMobys {
    /// `records` / `palette` = the initial buffer contents (their sizes are fixed from then on).
    pub fn new(level: &LoadedLevel, records: Vec<u8>, palette: Vec<u8>, buffers: &mut Assets<ShaderBuffer>) -> Self {
        let table_bytes: Vec<u8> = match &level.mobys.lighting {
            Some(l) => l.table.0.iter().flatten().flat_map(|w| w.to_le_bytes()).collect(),
            None => vec![0; 256 * 8],
        };
        let slots = records.len() / RECORD_SIZE;
        let proto = MatProto {
            instances: buffers.add(ShaderBuffer::new(&records, RenderAssetUsages::default())),
            palette: buffers.add(ShaderBuffer::new(&palette, RenderAssetUsages::default())),
            normal_table: buffers.add(ShaderBuffer::new(&table_bytes, RenderAssetUsages::RENDER_WORLD)),
            cpu_colors: buffers.add(ShaderBuffer::new(&[0u8; 16], RenderAssetUsages::RENDER_WORLD)),
            lods: buffers.add(ShaderBuffer::new(&moby_lod::default_lod_records(slots), RenderAssetUsages::default())),
        };
        ExtraMobys {
            instances: proto.instances.clone(),
            palette: proto.palette.clone(),
            normal_table: proto.normal_table.clone(),
            lods: proto.lods.clone(),
            cache: MatCache::new(proto),
            metal_images: HashMap::new(),
            parts: HashMap::new(),
            metal_parts: HashMap::new(),
            slot_ents: HashMap::new(),
            slot_shown: HashMap::new(),
            slot_metal: HashMap::new(),
            slot_metal_shown: HashMap::new(),
            slot_late: HashMap::new(),
        }
    }

    /// The highest joint index any high-LOD or metal vertex of `class` skins to (palette slots needed: this + 1).
    pub fn max_skinned_joint(class: &LevelMobyClass) -> u8 {
        class.class.high_lod.iter().chain(&class.class.metal).flat_map(|s| &s.vertices).map(|v| *v.skin.joints[..v.skin.count.max(1) as usize].iter().max().unwrap()).max().unwrap_or(0)
    }

    /// Builds `class`'s meshes (as for a gameplay instance) and spawns one entity per (texture, GS pass),
    /// tagged `MeshTag(slot)`, at `transform`, plus its metal entities. Returns the entities.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &mut self,
        commands: &mut Commands,
        level: &LoadedLevel,
        class: &LevelMobyClass,
        slot: u32,
        transform: Transform,
        name: &str,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        materials: &mut Assets<MobyMaterial>,
    ) -> Vec<Entity> {
        let (mut out, metal) = self.spawn_split(commands, level, class, slot, transform, name, meshes, images, materials);
        out.extend(metal);
        out
    }

    /// Draw this set's mobys in their slot order, as `DrawMobys` walks its list ([`MobyMaterial::order`]): every slot's
    /// Z-writing draws late, then its colour-only half (TEST_1 0x5360b's As < AREF: RGB, no Z), then its shine, before
    /// the next slot's. A later moby behind an earlier one's colour-only pixels then draws over them, as on the GS
    /// (Rilgar's broadcast: Qwark 920 stands behind the TV 984's screen and shows untinted). For sets whose slots are
    /// the game's draw order (the scene actors, crate::scene_render); spawn after calling it.
    pub fn set_ordered(&mut self) { self.cache.ordered = true; }

    /// [`Self::spawn`] with the high-LOD entities and the metal entities apart.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_split(
        &mut self,
        commands: &mut Commands,
        level: &LoadedLevel,
        class: &LevelMobyClass,
        slot: u32,
        transform: Transform,
        name: &str,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        materials: &mut Assets<MobyMaterial>,
    ) -> (Vec<Entity>, Vec<Entity>) {
        let parts = self.parts.entry(class.o_class).or_insert_with(|| build_parts(class, &class.class.high_lod, false, glow_from(class, false), meshes).0).clone();
        let out = self.cache.spawn(commands, level, &parts, MobyBlend::Plain, true, transform, slot, Visibility::Inherited, &format!("{name} class {}", class.o_class), images, materials);
        // The shine pass, its gate and basis written each frame by update_extra_metal.
        let mut metal = Vec::new();
        if moby_lod::metal_enabled() {
            let tracked = ExtraMetal { slot, records: self.instances.clone(), lods: self.lods.clone(), sphere: class.class.header.bsphere };
            metal = self.spawn_metal(commands, level, class, slot, transform, Visibility::Inherited, name, Some(tracked), meshes, images);
        }
        // The glow list: a fresh moby of the class glows in the class colour (`InitMobyInstance`, crate::moby_lod).
        let glow = moby_lod::class_glow_word(class.class.header.glow_rgba);
        if glow != 0 {
            let h = self.lods.clone();
            commands.queue(move |world: &mut World| {
                set_lod_bytes(&mut world.resource_mut::<Assets<ShaderBuffer>>(), &h, slot as usize, 12, &glow.to_le_bytes());
            });
        }
        (out, metal)
    }

    /// The low-LOD entities of `class` for record `slot` (shown: the caller hides them; empty without a low LOD): MobyProc's
    /// LOD pick shows them instead of the high ones (crate::moby_lod).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_low(
        &mut self,
        commands: &mut Commands,
        level: &LoadedLevel,
        class: &LevelMobyClass,
        slot: u32,
        transform: Transform,
        name: &str,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        materials: &mut Assets<MobyMaterial>,
    ) -> Vec<Entity> {
        if class.class.low_lod.is_empty() { return Vec::new(); }
        let identity = class.class.header.low_lod_joint_count == 0;
        let parts = build_parts(class, &class.class.low_lod, identity, glow_from(class, true), meshes).0;
        self.cache.spawn(commands, level, &parts, MobyBlend::Plain, true, transform, slot, Visibility::Inherited, &format!("{name} class {} low", class.o_class), images, materials)
    }
}

impl ExtraMobys {
    /// Takes slot `slot` off the glow list (its glow packets drawn lit), for a moby the game's glow overwrite does not
    /// reach (docs/plan/moby_skinning_lighting.md §10). Two ways, per caller:
    /// * its mode bits lose 0x10 after `InitMobyInstance`: the page menus' widget mobys (+0x34 = 0 / 4, crate::menu_models);
    /// * it is drawn in a `DrawMobyList` batch whose last list has no glow records: each `MobyProc` call restarts the list
    ///   at SPR 0x3400 and rewrites 0x1ac680 and its length 0x15fff8 (level01 0x26a8ac, 0x26b4c8..0x26b4dc), and
    ///   `DrawMobysCleanUp` (0x264d68 → `fun_002116b8`) runs once after the batch, so only the **last** list's glow
    ///   packets are recoloured: the vendor's mode-5 screens (`DrawWorld_Mode5` 0x2b4020: the salesman or the popup
    ///   last, neither with glow packets; crate::vendor_render).
    pub fn clear_glow(&self, commands: &mut Commands, slot: u32) {
        let h = self.lods.clone();
        commands.queue(move |world: &mut World| {
            set_lod_bytes(&mut world.resource_mut::<Assets<ShaderBuffer>>(), &h, slot as usize, 12, &0u32.to_le_bytes());
        });
    }

    /// Writes slot `slot`'s glow word (`moby_lod::glow_word` of the moby's mode and +0x90), for an owner that rewrites
    /// +0x90 every frame (Clank's pulse, crate::moby_attach).
    pub fn set_glow(&self, commands: &mut Commands, slot: u32, word: u32) {
        let h = self.lods.clone();
        commands.queue(move |world: &mut World| {
            set_lod_bytes(&mut world.resource_mut::<Assets<ShaderBuffer>>(), &h, slot as usize, 12, &word.to_le_bytes());
        });
    }

    /// The metal entities of `class` for record `slot` (one per metal texture and GS pass), with `tracked` when
    /// update_extra_metal owns their shine. The caller only lends the regular material store, so the metal materials
    /// are added by a queued command.
    #[allow(clippy::too_many_arguments)]
    fn spawn_metal(
        &mut self,
        commands: &mut Commands,
        level: &LoadedLevel,
        class: &LevelMobyClass,
        slot: u32,
        transform: Transform,
        visibility: Visibility,
        name: &str,
        tracked: Option<ExtraMetal>,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> Vec<Entity> {
        let metal = self.metal_parts.entry(class.o_class).or_insert_with(|| build_metal_parts(class, meshes).0).clone();
        let mut out = Vec::new();
        for part in &metal {
            let (image, texel_alpha) = self.metal_images.entry(part.kind).or_insert_with(|| metal_image(level, part.kind, images)).clone();
            for pass in metal_passes(texel_alpha) {
                let mat = MobyMetalMaterial {
                    texture: image.clone(),
                    fog: crate::game_camera::fog_buffer(),
                    instances: self.instances.clone(),
                    palette: self.palette.clone(),
                    normal_table: self.normal_table.clone(),
                    lods: self.lods.clone(),
                    pass,
                    caster: part.caster,
                    order: self.cache.ordered.then_some(slot),
                };
                let mut ec = commands.spawn((
                    Mesh3d(part.mesh.clone()),
                    transform,
                    MeshTag(slot),
                    NoFrustumCulling,
                    visibility,
                    Name::new(format!("{name} class {} metal {} {pass:?}", class.o_class, part.kind)),
                ));
                if let Some(t) = &tracked { ec.insert(t.clone()); }
                let e = ec.id();
                commands.queue(move |world: &mut World| {
                    let h = world.resource_mut::<Assets<MobyMetalMaterial>>().add(mat);
                    if let Ok(mut ent) = world.get_entity_mut(e) { ent.insert(MeshMaterial3d(h)); }
                });
                out.push(e);
            }
        }
        out
    }

    /// Shows slot `slot` of this record set as `class` (None: nothing) with the blend its [`SlotLook`] asks for
    /// ([`MobyBlend`]): the entities of that (class, blend) are spawned on first use (high LOD) and shown, the slot's
    /// other entities hidden; the class's metal entities are shown while the look's shine alpha is above 0 (spawned on
    /// first use); the entities follow the model's translation (the blended phase's sort key) and the slot's `MobyLod`
    /// record (vertex alpha, shine alpha and basis, glow word) is written. The record and palette are the caller's.
    #[allow(clippy::too_many_arguments)]
    pub fn show_slot(
        &mut self,
        commands: &mut Commands,
        level: &LoadedLevel,
        slot: u32,
        view: Option<(&LevelMobyClass, SlotLook)>,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        materials: &mut Assets<MobyMaterial>,
        buffers: &mut Assets<ShaderBuffer>,
    ) {
        let want = view.map(|(c, l)| ((c.o_class, MobyBlend::pick(l.mode, l.fading, l.alpha)), l.model.w_axis.truncate()));
        let was = self.slot_shown.get(&slot).copied();
        let (want_key, was_key) = (want.map(|w| w.0), was.map(|w| w.0));
        if want_key != was_key {
            if let Some(k) = was_key {
                for &e in self.slot_ents.get(&(slot, k.0, k.1)).into_iter().flatten() { commands.entity(e).insert(Visibility::Hidden); }
            }
        }
        // The metal pass: MobyProc's shine gate (moby+0x73; crate::moby_lod).
        let want_metal = view.filter(|(c, l)| l.shine > 0 && c.class.header.metal_count > 0 && moby_lod::metal_enabled()).map(|(c, _)| c.o_class);
        let was_metal = self.slot_metal_shown.get(&slot).copied();
        if want_metal != was_metal {
            if let Some(k) = was_metal {
                for &e in self.slot_metal.get(&(slot, k)).into_iter().flatten() { commands.entity(e).insert(Visibility::Hidden); }
            }
        }
        let Some(((oc, blend), at)) = want else {
            self.slot_shown.remove(&slot);
            self.slot_metal_shown.remove(&slot);
            return;
        };
        let (class, look) = view.unwrap();
        let t = Transform::from_translation(at);
        let key = (slot, oc, blend);
        if let Some(ents) = self.slot_ents.get(&key) {
            if want_key != was_key || was.is_none_or(|w| w.1 != at) {
                for &e in ents { commands.entity(e).insert((Visibility::Inherited, t)); }
            }
            if want_key != was_key || self.slot_late.get(&slot) != Some(&look.late) { queue_late(commands, ents.clone(), look.late); }
        } else {
            let parts = self.parts.entry(oc).or_insert_with(|| build_parts(class, &class.class.high_lod, false, glow_from(class, false), meshes).0).clone();
            let name = format!("dynamic moby slot {slot} class {oc} {blend:?}");
            let ents = self.cache.spawn(commands, level, &parts, blend, look.late, t, slot, Visibility::Inherited, &name, images, materials);
            self.slot_ents.insert(key, ents);
        }
        self.slot_late.insert(slot, look.late);
        if want_metal.is_some() {
            if let Some(ents) = self.slot_metal.get(&(slot, oc)) {
                if want_metal != was_metal || was.is_none_or(|w| w.1 != at) {
                    for &e in ents { commands.entity(e).insert((Visibility::Inherited, t)); }
                }
            } else {
                let name = format!("dynamic moby slot {slot}");
                let ents = self.spawn_metal(commands, level, class, slot, t, Visibility::Inherited, &name, None, meshes, images);
                self.slot_metal.insert((slot, oc), ents);
            }
            self.slot_metal_shown.insert(slot, oc);
        } else {
            self.slot_metal_shown.remove(&slot);
        }
        self.slot_shown.insert(slot, ((oc, blend), at));
        let mut rec = [0u8; moby_lod::LOD_RECORD_SIZE];
        let shine = if want_metal.is_some() { look.shine } else { 0 };
        moby_lod::write_lod_record(&mut rec, look.alpha, 0, shine, moby_lod::glow_word(look.mode, look.glow), &look.e);
        set_lod_bytes(buffers, &self.lods, slot as usize, 0, &rec);
    }
}

/// The placement a `MobyInst` record's model (its first 64 bytes, `game_to_bevy · [s/1024 · R | p]`, [`extra_model`])
/// stands for: the unit rotation rows R (moby+0xc0..), the position p and the scale s (moby+0x2c); None for a zero
/// model. Taken from the record rather than the class, so an item drawn at another scale than its class's (the vendor's
/// logo grows to 2.5× its class scale) gets unit rows for the shine basis E and its own sphere.
fn record_placement(bytes: &[u8]) -> Option<([[f32; 3]; 3], [f32; 3], f32)> {
    let f: [f32; 16] = std::array::from_fn(|i| f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()));
    let to_game = |v: [f32; 3]| [v[0], -v[2], v[1]];
    let k = Vec3::new(f[0], f[1], f[2]).length();
    if k == 0.0 { return None; }
    let rows = [0, 1, 2].map(|c| to_game([f[c * 4], f[c * 4 + 1], f[c * 4 + 2]]).map(|v| v / k));
    Some((rows, to_game([f[12], f[13], f[14]]), k * 1024.0))
}

/// The shine gate and basis of every extra instance with metal entities, from its record's model matrix
/// (`game_to_bevy · [s/1024 · R | p]`, written by moby_attach) and the class sphere. The draw-distance, LOD
/// and fade tests are not applied to extras.
fn update_extra_metal(
    q: Query<&ExtraMetal>,
    cams: Query<&Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(t) = cams.iter().next() else { return };
    let [fwd, left, up] = crate::game_camera::game_rows(t);
    let (eye, cam) = (crate::game_camera::game_eye(t), moby_lod::camera_rows(fwd, left, up));
    let mut done = HashSet::new();
    let mut writes: Vec<(Handle<ShaderBuffer>, u32, [u8; moby_lod::LOD_RECORD_SIZE])> = Vec::new();
    for x in &q {
        if !done.insert((x.lods.id(), x.slot)) { continue; }
        let Some(data) = buffers.get(&x.records).and_then(|b| b.data.as_ref()) else { continue };
        let at = x.slot as usize * RECORD_SIZE;
        let Some((rows, position, scale)) = data.get(at..at + 64).and_then(record_placement) else { continue };
        let inp = ProcInput {
            position,
            rows,
            scale,
            draw_distance: moby_lod::DRAW_DISTANCE_CAP,
            lod_trans: 0xff,
            shine_distance: moby_lod::SHINE_DISTANCE,
            alpha: 0x80,
        };
        let sphere = moby_lod::world_sphere(&inp, x.sphere);
        let v = moby_lod::view_centre(sphere, eye, &cam);
        let shine = moby_lod::shine_alpha(moby_lod::sphere_depth(v, sphere[3]), inp.shine_distance);
        let e = moby_lod::shine_basis(sphere, eye, &cam, &rows);
        let mut rec = [0u8; moby_lod::LOD_RECORD_SIZE];
        // The vertex alpha and the glow word are the slot's own (show_slot, ExtraMobys::spawn); only the shine fields are
        // this system's.
        let at = x.slot as usize * moby_lod::LOD_RECORD_SIZE;
        let old = buffers.get(&x.lods).and_then(|b| b.data.as_ref()).and_then(|d| d.get(at..at + 16));
        let alpha = old.map_or(0x80, |d| d[0]);
        let glow = old.map_or(0, |d| u32::from_le_bytes(d[12..16].try_into().unwrap()));
        moby_lod::write_lod_record(&mut rec, alpha, 0, shine, glow, &e);
        writes.push((x.lods.clone(), x.slot, rec));
    }
    for (h, slot, rec) in writes {
        let at = slot as usize * moby_lod::LOD_RECORD_SIZE;
        let range = at..at + moby_lod::LOD_RECORD_SIZE;
        if buffers.get(&h).and_then(|b| b.data.as_ref()).and_then(|d| d.get(range.clone())).is_none_or(|d| d == rec) { continue; }
        let Some(mut buf) = buffers.get_mut(&h) else { continue };
        if let Some(data) = buf.data.as_mut().and_then(|d| d.get_mut(range)) { data.copy_from_slice(&rec); }
    }
}

impl MobyOcclusion {
    /// Moby +0x36 of static instance `ii` (`MobyProc` tests it): 0, the game's "unchanged", is the loader's word for
    /// the instance; any other value (0x7f80 for a creature revived or moved elsewhere, a thrown crate) is used as is.
    pub fn set_occlusion_word(&mut self, ii: usize, word: u16) {
        let w = if word == 0 { self.loader_bits.get(ii).copied() } else { Some(OcclBits(word)) };
        if let (Some(b), Some(w)) = (self.bits.get_mut(ii), w) { *b = w; }
    }

    /// What MobyProc reads from static instance `ii` this tick ([`MobyLook`]): its vertex alpha, blend mode, glow and
    /// shine distance.
    pub fn look(&mut self, ii: usize, look: MobyLook) {
        if let Some(l) = self.look.get_mut(ii) {
            let late = l.late;
            *l = look;
            if look.late.is_none() { l.late = late; }
        }
    }

    /// Whether instance `ii`'s caster draws come after the shadow pass this frame ([`MobyLook::late`]).
    pub fn set_late(&mut self, ii: usize, late: bool) {
        if let Some(l) = self.look.get_mut(ii) { l.late = Some(late); }
    }

    /// The `MobyAnim` instance of gameplay instance `ii` (None without geometry).
    pub fn anim_index(&self, ii: usize) -> Option<usize> { self.anim_index.get(ii).copied().flatten() }

    /// Static instance `ii` now shows class `ci` (index into `LevelMobys::classes`): its moby changed its class at
    /// run time (rc_game::moby_update::class_swap, G-CLS-031: +0xa6, +0x24 header, +0x72). The entities, sphere and
    /// LOD switch of `ci`, made with the level for the classes the instance may become, take the place of the shown
    /// class's (hidden next frame); MobyProc then picks the new class's group as on a first show. False (nothing
    /// changes) when `ci` is the shown class or one the instance was not made for. The caller points the instance's
    /// `MobyAnim` entry at `ci` ([`crate::moby_anim::MobyAnim::set_class`]).
    pub fn set_class(&mut self, ii: usize, ci: usize) -> bool {
        let Some(d) = self.draws.get_mut(ii) else { return false };
        if d.class == ci { return false; }
        let Some(o) = d.other.iter_mut().find(|o| o.class == ci) else { return false };
        std::mem::swap(&mut d.class, &mut o.class);
        std::mem::swap(&mut d.class_sphere, &mut o.class_sphere);
        std::mem::swap(&mut d.groups, &mut o.groups);
        std::mem::swap(&mut d.metal, &mut o.metal);
        if let Some(inp) = d.input.as_mut() { std::mem::swap(&mut inp.lod_trans, &mut o.lod_trans); }
        self.pending_hide.extend(o.groups.values().flatten().copied().chain(o.metal.iter().copied()));
        if let Some(s) = self.shown.get_mut(ii) { *s = None; }
        true
    }
}

// ---------------------------------------------------------------------------------------------------
// Scheduler-driven static instances (crate::gameplay): the moby loop moves / turns / hides the static
// instances of the ported classes (bolts, crates, grass) every tick, and reads back their +0x31.

impl MobyOcclusion {
    /// +0x31 of static instance `ii` for the moby loop: MobyProc drew it this frame (not hidden by the moby
    /// loop, occlusion and the draw-distance / near / frustum culls passed). Spawn-hidden instances
    /// (crate::moby_spawn) are the caller's to exclude.
    pub fn drawn(&self, ii: usize) -> bool { self.shown.get(ii).is_some_and(|s| s.is_some()) }

    /// Forgets the group instance `ii` shows, for a caller that made all of its entities hidden (a spawn-hidden
    /// instance revealed): the next [`update_moby_occlusion`] switches on exactly the group (and metal) it picks.
    /// Showing every entity instead would show all its LOD / blend groups at once, and a later hide would then only
    /// switch off the picked one.
    pub fn reshow(&mut self, ii: usize) { if let Some(s) = self.shown.get_mut(ii) { *s = None; } }

    /// The placement the moby loop gave static instance `ii` this tick: position (+0x10), rotation rows
    /// (+0xc0.., row i = image of model axis i), scale (+0x2c), the light block for those rows (None: keep
    /// the loaded one) and hidden (deleted or mode & 1). Rewrites its `MobyInst` record (uploaded by
    /// `update_moby_occlusion` when anything changed) and MobyProc's inputs.
    pub fn drive(&mut self, ii: usize, position: [f32; 3], rows: [[f32; 3]; 3], scale: f32, lights: Option<&MobyLights>, hidden: bool) {
        let Some(d) = self.draws.get_mut(ii) else { return };
        if let Some(inp) = d.input.as_mut() {
            inp.position = position;
            inp.rows = rows;
            inp.scale = scale;
        }
        let at = game_to_bevy(position);
        if at != d.translation {
            d.translation = at;
            self.moved[ii] = true;
        }
        self.driven_hidden[ii] = hidden;
        let mut rec = Vec::with_capacity(RECORD_SIZE);
        let model = extra_model(rows, scale, position);
        write_record(&mut rec, &model, &lights.map(GpuLights::new).unwrap_or_default(), [0; 4]);
        let at = ii * RECORD_SIZE;
        let n = if lights.is_some() { 192 } else { 64 };
        let Some(dst) = self.records.get_mut(at..at + n) else { return };
        if dst != &rec[..n] {
            dst.copy_from_slice(&rec[..n]);
            self.records_dirty = true;
        }
    }
}
