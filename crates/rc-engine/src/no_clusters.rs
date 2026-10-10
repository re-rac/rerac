//! **Port-only: Bevy's light clustering off.** The port spawns no Bevy light (the world's lighting is the game's own,
//! computed in our shaders from crate::world_lights), yet Bevy clusters lights for every 3D camera: on the GPU that is
//! a dozen compute and render passes a camera per frame, plus fresh index-list buffers uploaded each frame. With no
//! light to assign it computes nothing. This turns GPU clustering off (CPU clustering with nothing to assign is a
//! few empty buffers) and gives every 3D camera `ClusterConfig::None` (no clusters at all).

use bevy::light::cluster::{ClusterConfig, GlobalClusterSettings};
use bevy::prelude::*;

pub struct NoClustersPlugin;

impl Plugin for NoClustersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, cpu_clustering).add_observer(no_clusters);
    }
}

/// Inserted by `PbrPlugin::finish`, read every frame (the render world follows it).
fn cpu_clustering(settings: Option<ResMut<GlobalClusterSettings>>) {
    if let Some(mut s) = settings { s.gpu_clustering = None; }
}

fn no_clusters(add: On<Add, Camera3d>, mut commands: Commands) {
    commands.entity(add.entity).insert(ClusterConfig::None);
}
