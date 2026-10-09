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
        )
        .add_systems(
            Last,
            (
                changes::<Image>,
                changes::<bevy::render::storage::ShaderBuffer>,
                changes::<Mesh>,
                changes::<crate::tfrag_render::TfragMaterial>,
                changes::<crate::tie_render::TieMaterial>,
                changes::<crate::shrub_render::ShrubMaterial>,
                changes::<crate::moby_render::MobyMaterial>,
                changes::<crate::moby_render::MobyMetalMaterial>,
                changes::<crate::fx_draw::FxPrimMaterial>,
                changes::<crate::particle_render::ParticleMaterial>,
                changes::<crate::water_render::WaterMaterial>,
                changes::<crate::hud_render::HudMaterial>,
                buffer_detail,
                mesh_detail,
            ),
        );
    }
}

/// Asset changes per frame (each a re-upload, or a new GPU resource / bind group), averaged over 300 frames: added,
/// modified, removed, and the distinct assets modified.
fn changes<A: Asset>(mut ev: MessageReader<AssetEvent<A>>, mut acc: Local<(u64, [u64; 3], HashSet<AssetId<A>>)>) {
    acc.0 += 1;
    for e in ev.read() {
        match e {
            AssetEvent::Added { .. } => acc.1[0] += 1,
            AssetEvent::Modified { id } => {
                acc.1[1] += 1;
                acc.2.insert(*id);
            }
            AssetEvent::Removed { .. } | AssetEvent::Unused { .. } => acc.1[2] += 1,
            _ => {}
        }
    }
    if !due(acc.0) { return; }
    let n = 300.0;
    let name = std::any::type_name::<A>().rsplit("::").next().unwrap_or("?");
    println!(
        "draws: changes {name:<20} per frame: added {:.1}, modified {:.1} ({} distinct), removed {:.1}",
        acc.1[0] as f64 / n, acc.1[1] as f64 / n, acc.2.len(), acc.1[2] as f64 / n
    );
    *acc = (acc.0, [0; 3], HashSet::new());
}

/// The shader buffers modified in the last 300 frames: changes, byte size and label, most changed first.
fn buffer_detail(
    mut ev: MessageReader<AssetEvent<bevy::render::storage::ShaderBuffer>>,
    assets: Res<Assets<bevy::render::storage::ShaderBuffer>>,
    mut acc: Local<(u64, std::collections::HashMap<AssetId<bevy::render::storage::ShaderBuffer>, u64>)>,
) {
    acc.0 += 1;
    for e in ev.read() {
        if let AssetEvent::Modified { id } = e { *acc.1.entry(*id).or_default() += 1; }
    }
    if !due(acc.0) { return; }
    let mut v: Vec<_> = acc.1.drain().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (id, n) in v.into_iter().take(25) {
        let b = assets.get(id);
        let len = b.and_then(|b| b.data.as_ref().map(|d| d.len())).unwrap_or(0);
        let label = b.and_then(|b| b.buffer_description.label).unwrap_or("-");
        println!("draws: buffer {id:?} {n:4} changes / 300 frames, {len} bytes, label {label}");
    }
}

/// The meshes modified in the last 300 frames: changes, vertex count and attributes, most changed first.
fn mesh_detail(mut ev: MessageReader<AssetEvent<Mesh>>, assets: Res<Assets<Mesh>>, mut acc: Local<(u64, std::collections::HashMap<AssetId<Mesh>, u64>)>) {
    acc.0 += 1;
    for e in ev.read() {
        if let AssetEvent::Modified { id } = e { *acc.1.entry(*id).or_default() += 1; }
    }
    if !due(acc.0) { return; }
    let mut v: Vec<_> = acc.1.drain().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (id, n) in v.into_iter().take(25) {
        let m = assets.get(id);
        let verts = m.map_or(0, |m| m.count_vertices());
        let attrs: Vec<String> = m.map(|m| m.attributes().map(|(a, _)| a.name.to_string()).collect()).unwrap_or_default();
        println!("draws: mesh {id:?} {n:4} changes / 300 frames, {verts} vertices, attributes {attrs:?}");
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
