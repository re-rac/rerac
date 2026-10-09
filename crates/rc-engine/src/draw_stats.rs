//! `RC_DRAW_STATS=1` (dev): every 300 frames, per material type, the mesh entities, how many are visible this frame
//! (`ViewVisibility`), and how many distinct materials and meshes the visible ones use (a draw is made per visible
//! entity and view; a distinct material is a distinct bind group); then the active cameras with their order and
//! render layers. To see what the renderer is asked to draw.

use bevy::camera::visibility::{RenderLayers, ViewVisibility};
use bevy::prelude::*;
use std::collections::HashSet;

pub struct DrawStatsPlugin;

impl Plugin for DrawStatsPlugin {
    fn build(&self, app: &mut App) {
        if !std::env::var("RC_DRAW_STATS").is_ok_and(|v| v.trim() == "1") { return; }
        app.add_systems(
            Last,
            (
                count::<crate::tfrag_render::TfragMaterial>,
                count::<crate::tie_render::TieMaterial>,
                count::<crate::shrub_render::ShrubMaterial>,
                count::<crate::shrub_billboard::BillboardMaterial>,
                count::<crate::moby_render::MobyMaterial>,
                count::<crate::moby_render::MobyMetalMaterial>,
                count::<crate::fx_draw::FxPrimMaterial>,
                count::<crate::particle_render::ParticleMaterial>,
                count::<crate::sky_render::SkyMaterial>,
                count::<crate::sky_stars::SkyStarMaterial>,
                count::<crate::water_render::WaterMaterial>,
                count::<StandardMaterial>,
                cameras,
            )
                .chain(),
        );
    }
}

fn due(frame: u64) -> bool { frame > 0 && frame.is_multiple_of(300) }

fn count<T: Material>(q: Query<(&MeshMaterial3d<T>, &Mesh3d, &ViewVisibility)>, mut frame: Local<u64>) {
    *frame += 1;
    if !due(*frame) { return; }
    let (mut n, mut vis) = (0usize, 0usize);
    let (mut mats, mut meshes) = (HashSet::new(), HashSet::new());
    for (m, mesh, v) in &q {
        n += 1;
        if !v.get() { continue; }
        vis += 1;
        mats.insert(m.0.id());
        meshes.insert(mesh.0.id());
    }
    let name = std::any::type_name::<T>().rsplit("::").next().unwrap_or("?");
    println!("draws: {name:<20} entities {n:6}  visible {vis:6}  materials {:5}  meshes {:5}", mats.len(), meshes.len());
}

fn cameras(q: Query<(Entity, &Camera, Option<&Name>, Option<&RenderLayers>)>, mut frame: Local<u64>) {
    *frame += 1;
    if !due(*frame) { return; }
    let mut v: Vec<_> = q.iter().filter(|c| c.1.is_active).collect();
    v.sort_by_key(|c| c.1.order);
    println!("draws: {} active cameras", v.len());
    for (e, c, name, layers) in v {
        println!("draws:   order {:3} {:?} {} layers {:?}", c.order, e, name.map_or("-", |n| n.as_str()), layers.map(|l| l.iter().collect::<Vec<_>>()));
    }
}
