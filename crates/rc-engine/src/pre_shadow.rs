//! The world's colour-only draws before the shadow pass (docs/plan/shadows.md §7 risk 5): the halves of the
//! world's alpha tests that write colour only (`GsPass::ColorOnlyLowAlpha`: tfrags, ties, shrubs) and the shrub
//! billboards' pass 2 (`GsPass::BlendNoZ`). The game draws them with the world, before the shadow volumes darken the
//! frame, so a shadow darkens them too; Bevy would draw them in the transparent pass, after crate::shadow_render's
//! darkening. Their items are taken out of the view's `Transparent3d` phase (in sorted order) and drawn right after
//! the opaque pass, against the scene depth, then put back for Bevy's retained phase (as crate::display_blend does
//! for the effects).
//!
//! Which entities: [`BeforeShadows`], added when a world entity first gets a material whose GS pass is one of the
//! two ([`tag`], one system per world material type).
//!
//! Order: the game's, not Bevy's back-to-front sort ([`order`], between Bevy's sort and its batching). The GS frame
//! runs sky → tfrag → tie → shrub → billboards (docs/plan/shadows.md §3.1), and inside each:
//! * tfrags: the port's texture batches in their build order (the game draws per tfrag; a batch merges many);
//! * ties: `TieProc` (boot 0x235be8) per class, then per packet across the class's visible instances in list
//!   order: the port's (class, part, instance);
//! * shrubs: `ShrubProc` (boot 0x228be8) draws the opaque list class by class, then sends the fading test block
//!   and draws the fading list class by class: (list, class, part, instance); the game sends all of an instance's
//!   packets together, the port each part across the class's instances (as in the opaque phase);
//! * billboards: pass 1 per class in class order, then pass 2 per class.
//!
//! Every renderer puts a [`WorldDrawOrder`] on its entities. Besides following the game, the order puts each
//! class part's instances next to each other, so Bevy draws them as one instanced draw instead of one draw per
//! instance interleaved by distance (and resets the pipeline only between groups).

use crate::gs_state::GsPass;
use bevy::core_pipeline::core_3d::{main_opaque_pass_3d, main_transparent_pass_3d, Transparent3d};
use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::render_phase::{sort_phase_system, SortedRenderPhase, ViewSortedRenderPhases};
use bevy::render::render_resource::{RenderPassDescriptor, StoreOp};
use bevy::render::renderer::{RenderContext, ViewQuery};
use bevy::render::sync_world::MainEntity;
use bevy::render::view::{ExtractedView, ViewDepthTexture, ViewTarget};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};
use std::sync::Mutex;

/// A world entity drawn before the shadow pass (module doc).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BeforeShadows;

/// A world entity's place in the game's draw order (module doc, "Order"), compared lexicographically:
/// `[renderer, …]` with the renderer ranks below.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldDrawOrder(pub [u32; 5]);

impl WorldDrawOrder {
    pub const TFRAG: u32 = 0;
    pub const TIE: u32 = 1;
    pub const SHRUB: u32 = 2;
    pub const BILLBOARD: u32 = 3;
    /// An entity without an order: after every ordered one.
    const NONE: Self = Self([u32::MAX; 5]);
}

/// Tags the new entities of material `M` whose GS pass writes colour only.
pub fn tag<M: Material>(mut commands: Commands, q: Query<(Entity, &MeshMaterial3d<M>), Added<MeshMaterial3d<M>>>, mats: Res<Assets<M>>)
where
    for<'a> GsPass: From<&'a M>,
{
    for (e, m) in &q {
        let Some(mat) = mats.get(&m.0) else { continue };
        if matches!(GsPass::from(mat), GsPass::ColorOnlyLowAlpha { .. } | GsPass::BlendNoZ) { commands.entity(e).insert(BeforeShadows); }
    }
}

pub struct PreShadowPlugin;

impl Plugin for PreShadowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                tag::<crate::tfrag_render::TfragMaterial>,
                tag::<crate::tie_render::TieMaterial>,
                tag::<crate::shrub_render::ShrubMaterial>,
                tag::<crate::shrub_billboard::BillboardMaterial>,
            ),
        );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .init_resource::<Tagged>()
            .init_resource::<Stash>()
            .add_systems(ExtractSchedule, extract)
            .add_systems(Render, order.in_set(RenderSystems::PhaseSort).after(sort_phase_system::<Transparent3d>))
            .add_systems(
            Core3d,
            (
                (split, draw).chain().after(main_opaque_pass_3d).before(crate::shadow_render::ShadowPassSet),
                restore.after(main_transparent_pass_3d),
            )
                .in_set(Core3dSystems::MainPass),
        );
    }
}

/// The main-world entities tagged this frame, with their draw order.
#[derive(Resource, Default)]
struct Tagged(HashMap<MainEntity, WorldDrawOrder>);

fn extract(mut set: ResMut<Tagged>, q: Extract<Query<(Entity, Option<&WorldDrawOrder>), With<BeforeShadows>>>) {
    set.0.clear();
    set.0.extend(q.iter().map(|(e, o)| (MainEntity::from(e), o.copied().unwrap_or(WorldDrawOrder::NONE))));
}

/// Puts the tagged items first, in the game's order (module doc, "Order"); the others keep Bevy's order (a stable
/// sort). Before batching, so consecutive items of one mesh and material become one instanced draw.
fn order(mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>, set: Res<Tagged>) {
    if set.0.is_empty() { return; }
    let key = |k: &(Entity, MainEntity)| set.0.get(&k.1).map_or((1, WorldDrawOrder::NONE), |o| (0, *o));
    for phase in phases.values_mut() {
        if !phase.items.keys().any(|k| set.0.contains_key(&k.1)) { continue; }
        phase.items.sort_by(|a, _, b, _| key(a).cmp(&key(b)));
    }
}

/// Per view: the items taken out of its `Transparent3d` phase this frame.
#[derive(Resource, Default)]
struct Stash(Mutex<HashMap<Entity, SortedRenderPhase<Transparent3d>>>);

fn split(view: ViewQuery<&ExtractedView>, mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>, set: Res<Tagged>, stash: Res<Stash>) {
    let view_entity = view.entity();
    let ev = view.into_inner();
    if set.0.is_empty() { return; }
    let Some(phase) = phases.get_mut(&ev.retained_view_entity) else { return };
    let keys: Vec<_> = phase.items.keys().filter(|k| set.0.contains_key(&k.1)).copied().collect();
    if keys.is_empty() { return; }
    let mut out = SortedRenderPhase::<Transparent3d>::default();
    for k in keys {
        if let Some(item) = phase.items.shift_remove(&k) { out.items.insert(k, item); }
    }
    stash.0.lock().unwrap_or_else(|e| e.into_inner()).insert(view_entity, out);
}

fn draw(world: &World, view: ViewQuery<(&ExtractedCamera, &ViewTarget, &ViewDepthTexture)>, mut ctx: RenderContext) {
    let view_entity = view.entity();
    let (camera, target, depth) = view.into_inner();
    let stash = world.resource::<Stash>();
    let guard = stash.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(phase) = guard.get(&view_entity).filter(|p| !p.items.is_empty()) else { return };
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("world colour-only draws (before the shadows)"),
        color_attachments: &[Some(target.get_color_attachment())],
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some(v) = camera.viewport.as_ref() { pass.set_camera_viewport(v); }
    if let Err(err) = phase.render(&mut pass, world, view_entity) { error!("pre-shadow draws: {err:?}"); }
}

fn restore(view: ViewQuery<&ExtractedView>, mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>, stash: Res<Stash>) {
    let view_entity = view.entity();
    let ev = view.into_inner();
    let Some(out) = stash.0.lock().unwrap_or_else(|e| e.into_inner()).remove(&view_entity) else { return };
    let Some(phase) = phases.get_mut(&ev.retained_view_entity) else { return };
    for (k, item) in out.items { phase.items.insert(k, item); }
}
