//! PS2-style blending of the effects: additive and translucent effects blend on the frame's **display bytes**, as
//! the GS does, not in linear light (user decision 2026-09-27; docs/plan/hardware_fidelity_layers.md "Result-level
//! reproductions"). One mechanism for every effect: the particles (crate::particle_render), the effect mobys
//! (`MobyBlend::Translucent` / `Additive` → `GsPass::EffectTested` + `EffectLowAlpha` / `GsPass::AdditiveNoZ`) and the draw-callback
//! effects (crate::fx_draw; the fire fields of crate::water_render). Each effect entity carries [`DisplayEffect`].
//!
//! **Why.** The GS equations are `Cd + (Cs·As >> 7)` (ALPHA 0x48) and `Cd + ((Cs − Cd)·As >> 7)` (0x44) on the
//! frame buffer's display-encoded bytes, clamped to 0..255. The port's frame is an sRGB target whose bytes are those
//! display bytes (every world shader outputs `srgb_to_linear(GS byte)`), but the GPU blends it in linear light: a dim
//! additive glow adds next to nothing over a lit scene (+25 display levels of a type-23 puff become +3 over grass),
//! stacked glows never reach the white the GS clamps them to, and a translucent shell looks lighter over dark ground.
//!
//! **How (a render target choice).** After Bevy's transparent pass (the world, water and every other blended draw,
//! in linear light as before), the effects are drawn into a display-encoded copy of the frame and copied back:
//! 1. the effect items ([`DisplayEffect`] entities) are taken out of the view's `Transparent3d` phase before the
//!    transparent pass ([`split_effects`]; they keep their sorted order: back to front within the bands the materials
//!    give them, effect mobys, then the list-1 callbacks, the particles, the list-2 callbacks);
//! 2. [`effect_pass`]: the frame is converted into an `Rgba8Unorm` texture that stores the display bytes themselves
//!    (`effect_blit.wgsl` `to_display`, per pixel: the sRGB decode and re-encode round-trips every byte), the effect
//!    items are drawn into it with ordinary hardware blending (`One, OneMinusSrcAlpha` on the premultiplied GS terms,
//!    [`PREMULTIPLIED`]) against the view's depth buffer (their Z tests and writes as before), and the result is
//!    converted back into the view's target (`to_linear`, exact per byte);
//! 3. the items are put back ([`restore_effects`]) so Bevy's retained phase is unchanged for the next frame.
//!
//! Blending on a non-sRGB target is the GS's display-byte arithmetic, so overlapping effects stack and saturate as
//! on the PS2, in draw order, and nothing is read back per fragment. The effect materials' pipelines target
//! `Rgba8Unorm` ([`specialize`]); their shaders output display-encoded colour (`display_blend.wgsl`). Remaining
//! differences: the hardware rounds `Cs·As/128` where the GS truncates (±1 level), As > 0x80 clamps to 1 in the 0x44
//! destination factor, and the effects are drawn after all the world's translucent draws (in the GS the effect mobys
//! come before the list-1 water strips).
//!
//! **Cost.** When a view has effect items: two full-frame conversion passes (and the MSAA resolve of the effect
//! target); none otherwise. Measured in docs/plan/particles.md "Display-space blending".

use bevy::core_pipeline::core_3d::{main_transparent_pass_3d, Transparent3d};
use bevy::core_pipeline::{Core3d, Core3dSystems, FullscreenShader};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::render_phase::{SortedRenderPhase, ViewSortedRenderPhases};
use bevy::render::render_resource::binding_types::texture_2d;
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendComponent, BlendFactor, BlendOperation, BlendState,
    CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FragmentState, LoadOp, MultisampleState, Operations,
    PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages, StoreOp, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, VertexState,
};
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::sync_world::MainEntity;
use bevy::render::texture::{CachedTexture, TextureCache};
use bevy::render::view::{ExtractedView, ViewDepthTexture, ViewTarget};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;
use std::sync::Mutex;

/// The display-encoded effect target's format (its bytes are the GS frame buffer's).
pub const EFFECT_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

/// `One, OneMinusSrcAlpha`: the blend of every effect draw; the shaders output `(Cs·As, As)` for 0x44 and
/// `(Cs·As, 0)` for 0x48, in display units (`display_blend.wgsl` `gs_mix` / `gs_add`).
pub const PREMULTIPLIED: BlendState = BlendState {
    color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::OneMinusSrcAlpha, operation: BlendOperation::Add },
    alpha: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::OneMinusSrcAlpha, operation: BlendOperation::Add },
};

/// Makes a material pipeline an effect pipeline: the display-encoded target and [`PREMULTIPLIED`]. The entity drawn
/// with it must carry [`DisplayEffect`] (it is drawn only by [`effect_pass`]).
pub fn specialize(descriptor: &mut RenderPipelineDescriptor) {
    if let Some(f) = descriptor.fragment.as_mut() {
        for t in f.targets.iter_mut().flatten() {
            t.format = EFFECT_FORMAT;
            t.blend = Some(PREMULTIPLIED);
        }
    }
}

/// An effect pipeline's blend made subtractive (after [`specialize`]): `Cd − Cs·As` on the display bytes (the shader
/// outputs `(Cs·As, 0)` as for 0x48), alpha kept; the GS's `(0 − Cs)·FIX + Cd` (ALPHA 0x62), clamped at 0.
pub fn specialize_subtract(descriptor: &mut RenderPipelineDescriptor) {
    if let Some(f) = descriptor.fragment.as_mut() {
        for t in f.targets.iter_mut().flatten() {
            t.blend = Some(BlendState {
                color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::ReverseSubtract },
                alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
            });
        }
    }
}

/// Marks an entity whose draws are effect draws (their pipelines went through [`specialize`]).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct DisplayEffect;

const IMPORT_PATH: &str = "shaders/display_blend.wgsl";
const BLIT_SHADER: &str = "shaders/effect_blit.wgsl";

/// Keeps the `rerac::display_blend` WGSL module loaded (the effect shaders import it by its path).
#[derive(Resource)]
struct DisplayBlendShader(#[allow(dead_code)] Handle<Shader>);

pub struct DisplayBlendPlugin;

impl Plugin for DisplayBlendPlugin {
    fn build(&self, app: &mut App) {
        let import = app.world().resource::<AssetServer>().load(IMPORT_PATH);
        app.insert_resource(DisplayBlendShader(import));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .init_resource::<EffectEntities>()
            .init_resource::<EffectStash>()
            .add_systems(RenderStartup, init_pipelines)
            .add_systems(ExtractSchedule, extract_effects)
            .add_systems(Render, prepare_targets.in_set(RenderSystems::PrepareResources))
            .add_systems(
                Core3d,
                (
                    split_effects.before(main_transparent_pass_3d),
                    (effect_pass, restore_effects).chain().after(main_transparent_pass_3d),
                )
                    .in_set(Core3dSystems::MainPass),
            );
    }
}

// ---------------------------------------------------------------------------------------------------
// Render world

/// The main-world entities that are effects (this frame).
#[derive(Resource, Default)]
struct EffectEntities(HashSet<MainEntity>);

fn extract_effects(mut set: ResMut<EffectEntities>, q: Extract<Query<Entity, With<DisplayEffect>>>) {
    set.0.clear();
    set.0.extend(q.iter().map(MainEntity::from));
}

/// Per view: the effect items taken out of its `Transparent3d` phase for this frame (render entity of the view →
/// phase).
#[derive(Resource, Default)]
struct EffectStash(Mutex<HashMap<Entity, SortedRenderPhase<Transparent3d>>>);

#[derive(Resource)]
struct EffectPipelines {
    layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
    fullscreen: VertexState,
    ids: Mutex<HashMap<(bool, u32, TextureFormat), CachedRenderPipelineId>>,
}

impl EffectPipelines {
    /// The conversion into the display target (`to_display`) or back into the view target (`to_linear`).
    fn id(&self, cache: &PipelineCache, to_display: bool, samples: u32, format: TextureFormat) -> CachedRenderPipelineId {
        let mut ids = self.ids.lock().unwrap_or_else(|e| e.into_inner());
        *ids.entry((to_display, samples, format)).or_insert_with(|| {
            cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(if to_display { "effects: frame to display bytes" } else { "effects: display bytes to frame" }.into()),
                layout: vec![self.layout.clone()],
                vertex: self.fullscreen.clone(),
                fragment: Some(FragmentState {
                    shader: self.shader.clone(),
                    shader_defs: vec![],
                    entry_point: Some(if to_display { "to_display" } else { "to_linear" }.into()),
                    targets: vec![Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })],
                }),
                multisample: MultisampleState { count: samples, ..default() },
                ..default()
            })
        })
    }
}

fn init_pipelines(mut commands: Commands, assets: Res<AssetServer>, fullscreen: Res<FullscreenShader>) {
    commands.insert_resource(EffectPipelines {
        layout: BindGroupLayoutDescriptor::new("effects: blit source", &BindGroupLayoutEntries::single(ShaderStages::FRAGMENT, texture_2d(TextureSampleType::Float { filterable: false }))),
        shader: assets.load(BLIT_SHADER),
        fullscreen: fullscreen.to_vertex_state(),
        ids: Default::default(),
    });
}

/// Per view with effect items: the display-encoded target (`ms` with MSAA, resolved into `res`; else `res` only).
#[derive(Component)]
struct EffectTargets {
    ms: Option<CachedTexture>,
    res: CachedTexture,
}

/// Queues the conversion pipelines and gives every view with effect items its display target.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn prepare_targets(
    mut commands: Commands,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    pipes: Option<Res<EffectPipelines>>,
    set: Res<EffectEntities>,
    phases: Res<ViewSortedRenderPhases<Transparent3d>>,
    mut textures: ResMut<TextureCache>,
    views: Query<(Entity, &ExtractedCamera, &ExtractedView, &ViewTarget)>,
) {
    let Some(pipes) = pipes else { return };
    for (e, cam, view, target) in &views {
        let has = phases.get(&view.retained_view_entity).is_some_and(|p| p.items.keys().any(|k| set.0.contains(&k.1)));
        let (Some(size), true) = (cam.physical_target_size, has) else {
            commands.entity(e).remove::<EffectTargets>();
            continue;
        };
        let samples = target.sampled_main_texture().map_or(1, |t| t.sample_count());
        let format = target.main_texture_format();
        pipes.id(&cache, true, samples, EFFECT_FORMAT);
        pipes.id(&cache, false, samples, format);
        let mut desc = |label: &'static str, samples: u32, usage: TextureUsages| {
            textures.get(
                &device,
                TextureDescriptor {
                    label: Some(label),
                    size: Extent3d { width: size.x.max(1), height: size.y.max(1), depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: TextureDimension::D2,
                    format: EFFECT_FORMAT,
                    usage,
                    view_formats: &[],
                },
            )
        };
        let res = desc("effects: display target", 1, TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING);
        let ms = (samples > 1).then(|| desc("effects: display target (MSAA)", samples, TextureUsages::RENDER_ATTACHMENT));
        commands.entity(e).insert(EffectTargets { ms, res });
    }
}

/// Takes the view's effect items out of its `Transparent3d` phase (in order), before the transparent pass.
fn split_effects(
    view: ViewQuery<&ExtractedView>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    set: Res<EffectEntities>,
    stash: Res<EffectStash>,
) {
    let view_entity = view.entity();
    let ev = view.into_inner();
    if set.0.is_empty() { return; }
    let Some(phase) = phases.get_mut(&ev.retained_view_entity) else { return };
    let keys: Vec<_> = phase.items.keys().filter(|k| set.0.contains(&k.1)).copied().collect();
    if keys.is_empty() { return; }
    let mut out = SortedRenderPhase::<Transparent3d>::default();
    for k in keys {
        if let Some(item) = phase.items.shift_remove(&k) { out.items.insert(k, item); }
    }
    stash.0.lock().unwrap_or_else(|e| e.into_inner()).insert(view_entity, out);
}

/// Draws the view's stashed effect items on the frame's display bytes (module doc, step 2).
fn effect_pass(
    world: &World,
    view: ViewQuery<(&ExtractedCamera, &ViewTarget, &ViewDepthTexture, Option<&EffectTargets>)>,
    pipes: Option<Res<EffectPipelines>>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (camera, target, depth, targets) = view.into_inner();
    let stash = world.resource::<EffectStash>();
    let guard = stash.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(phase) = guard.get(&view_entity).filter(|p| !p.items.is_empty()) else { return };
    let (Some(pipes), Some(t)) = (pipes, targets) else { return };
    let samples = target.sampled_main_texture().map_or(1, |t| t.sample_count());
    let format = target.main_texture_format();
    let (Some(to_display), Some(to_linear)) = (
        cache.get_render_pipeline(pipes.id(&cache, true, samples, EFFECT_FORMAT)),
        cache.get_render_pipeline(pipes.id(&cache, false, samples, format)),
    ) else {
        return;
    };
    let device = ctx.render_device().clone();
    let layout = cache.get_bind_group_layout(&pipes.layout);
    let from_frame = device.create_bind_group("effects: frame", &layout, &BindGroupEntries::single(target.main_texture_view()));
    let from_effects = device.create_bind_group("effects: display target", &layout, &BindGroupEntries::single(&t.res.default_view));
    let (color, resolve) = match &t.ms {
        Some(ms) => (&ms.default_view, Some(&t.res.default_view)),
        None => (&t.res.default_view, None),
    };
    // 1. The frame (resolved) as display bytes.
    {
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("effects: frame to display bytes"),
            color_attachments: &[Some(RenderPassColorAttachment { view: color, depth_slice: None, resolve_target: None, ops: Operations { load: LoadOp::Load, store: StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_render_pipeline(to_display);
        pass.set_bind_group(0, &from_frame, &[]);
        pass.draw(0..3, 0..1);
    }
    // 2. The effects, blended by the hardware on those bytes, depth-tested against the scene.
    {
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("effects: display-space blend"),
            color_attachments: &[Some(RenderPassColorAttachment { view: color, depth_slice: None, resolve_target: resolve.map(|v| &**v), ops: Operations { load: LoadOp::Load, store: StoreOp::Store } })],
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some(v) = camera.viewport.as_ref() { pass.set_camera_viewport(v); }
        if let Err(err) = phase.render(&mut pass, world, view_entity) { error!("effects: {err:?}"); }
    }
    // 3. Back into the view target.
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("effects: display bytes to frame"),
        color_attachments: &[Some(target.get_color_attachment())],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_render_pipeline(to_linear);
    pass.set_bind_group(0, &from_effects, &[]);
    pass.draw(0..3, 0..1);
}

/// Puts the stashed items back into the view's phase (Bevy keeps the phase from frame to frame).
fn restore_effects(view: ViewQuery<&ExtractedView>, mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>, stash: Res<EffectStash>) {
    let view_entity = view.entity();
    let ev = view.into_inner();
    let Some(out) = stash.0.lock().unwrap_or_else(|e| e.into_inner()).remove(&view_entity) else { return };
    let Some(phase) = phases.get_mut(&ev.retained_view_entity) else { return };
    for (k, item) in out.items { phase.items.insert(k, item); }
}
