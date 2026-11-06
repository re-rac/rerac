//! The mirror cheat 0x15edb4 ("Levels are mirrored", `rc_game::cheats`) on the screen: the game mirrors its camera's left
//! row (`CameraUpdate` 0x20eca8 for the follow camera, `0x2ac8d8` for the scene camera, `0x2a4080` for the ship), so the
//! world is drawn as its mirror image while the 2D layer (the HUD, the menus, the subtitles, the movies) is not. The
//! port's cameras ignore the left row (a Bevy transform has no reflection), so the world's frame is mirrored left to
//! right by a full-screen pass after the 3D passes and before the UI pass ([`MirrorFlip`],
//! `assets/shaders/mirror.wgsl`). The pad's left / right swap (`ProcessPadInput`), the follow camera's rows
//! (`rc_game::follow_camera`, the sound's listener) and the move classification of the cheat entry read the byte in the
//! game code; this module only draws.
//!
//! The pass mirrors the whole main target about its centre: the game frame's letterboxed viewport is centred, so the
//! picture inside it is mirrored about its own centre [L: an off-centre viewport would shift].

use crate::fly_cam::FlyCam;
use crate::gameplay::Persistent;
use bevy::core_pipeline::fullscreen_material::FullscreenMaterial;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

/// The mirror pass on the main camera (present only while the cheat is on).
#[derive(Component, ExtractComponent, Clone, Copy, Default, PartialEq, ShaderType)]
pub struct MirrorFlip {
    pub on: u32,
}

impl FullscreenMaterial for MirrorFlip {
    fn fragment_shader() -> ShaderRef { "shaders/mirror.wgsl".into() }

    /// After the 3D passes and the screen tint, before the UI pass (the HUD and the menus stay as they are). Both full-
    /// screen passes swap the post-process target: left unordered (the tint's pass is always on the camera), the two
    /// could reach the GPU in the other order than their swaps and the frame came out black now and then.
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system
            .after(bevy::core_pipeline::Core3dSystems::PostProcess)
            .after(crate::gs_post::pass::<crate::screen_tint::ScreenTint>)
            .before(bevy::ui_render::ui_pass)
    }
}

pub struct MirrorRenderPlugin;

impl Plugin for MirrorRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::gs_post::GsPostPlugin::<MirrorFlip>::default()).add_systems(PostUpdate, follow_cheat);
    }
}

/// The pass on the main camera while 0x15edb4 is set.
fn follow_cheat(mut commands: Commands, state: Option<Res<Persistent>>, cams: Query<(Entity, Option<&MirrorFlip>), With<FlyCam>>) {
    let on = state.is_some_and(|s| s.0.global.cheats_active[rc_game::cheats::slot::MIRROR] != 0);
    for (e, have) in &cams {
        match (on, have.is_some()) {
            (true, false) => { commands.entity(e).insert(MirrorFlip { on: 1 }); }
            (false, true) => { commands.entity(e).remove::<MirrorFlip>(); }
            _ => {}
        }
    }
}
