//! The game's follow camera (`rc_game::follow_camera`, published by crate::gameplay each tick) driving the
//! main view exactly as the fly camera does: position + rows → the Bevy `Transform` through `game_to_bevy`,
//! the same `GameProjection` (tan(hfov/2) = 0.63 at 0x16cf70 unless the game changes it), letterbox and fog
//! (crate::game_camera). The game's rows are (forward, left, up) in game space; the view looks along
//! forward with up as the up vector (left = up × forward follows).
//!
//! Runs in `RunFixedMainLoop` after the fixed loop, so every `Update` / `PostUpdate` reader of the camera
//! transform (occlusion, LODs, shrub culling, fog, the sky camera) sees this frame's game camera.

use crate::fly_cam::FlyCam;
use crate::game_camera::{CameraSource, TAN_HALF_FOV_X};
use crate::tfrag_render::game_to_bevy;
use bevy::prelude::*;
use rc_game::follow_camera::CameraView;

/// The camera the last gameplay tick produced, and the projection tangent it asks for.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlayView {
    pub view: CameraView,
    /// tan(hfov/2): 0.63 (`InitViewContext`); the type-0 camera never writes it (player_controller.md §7).
    pub tan_half_fov: f32,
    /// The view's far plane in integer units (`Services::view_far`): 745 472 unless a class moves it.
    pub far: f32,
}

/// The Bevy transform of a game camera: eye = pos, looking along row 0 (forward) with row 2 (up) up.
pub fn view_transform(v: &CameraView) -> Transform {
    let [fwd, _left, up] = v.rows_f32();
    Transform::from_translation(game_to_bevy(v.pos_f32())).looking_to(game_to_bevy(fwd), game_to_bevy(up))
}

/// Writes the game camera into the main camera while it is the source.
pub fn apply(
    view: Option<Res<PlayView>>,
    source: Option<Res<CameraSource>>,
    frame: Res<crate::display::GameFrame>,
    mut cams: Query<(&mut Transform, &mut Projection), With<FlyCam>>,
) {
    let ratio = frame.ratio();
    let (Some(view), Some(source)) = (view, source) else { return };
    if *source != CameraSource::Play { return; }
    let t = view_transform(&view.view);
    for (mut tf, mut proj) in &mut cams {
        if *tf != t { *tf = t; }
        // The FOV hook: the game's tangent (Hor+ in 16:9, crate::display); only touched when it changes.
        crate::display::set_view_tans(&mut proj, view.tan_half_fov, ratio);
        crate::game_camera::set_view_far(&mut proj, view.far);
    }
}

/// The default tangent (`0x16cf70`).
pub const GAME_TAN_HALF_FOV: f32 = TAN_HALF_FOV_X;

#[cfg(test)]
mod tests {
    use super::*;
    use rc_game::ps2v::Pf;

    #[test]
    fn transform_round_trips_through_game_rows() {
        // A camera 4.64 behind a hero at the origin facing +x, looking slightly down.
        let (c, s) = (0.2f32.cos(), 0.2f32.sin());
        let f = |v: [f32; 3]| [Pf::f(v[0]), Pf::f(v[1]), Pf::f(v[2]), Pf::ZERO];
        let v = CameraView {
            pos: f([-4.64, 0.0, 2.0]),
            rows: [f([c, 0.0, -s]), f([0.0, 1.0, 0.0]), f([s, 0.0, c])],
            euler: [Pf::ZERO; 4],
            class: 0,
            flyby_phase: 0,
        };
        let t = view_transform(&v);
        assert!((crate::game_camera::game_eye(&t) - Vec3::new(-4.64, 0.0, 2.0)).length() < 1e-5);
        let rows = crate::game_camera::game_rows(&t);
        for (r, e) in rows.iter().zip(v.rows_f32()) { assert!((*r - Vec3::from(e)).length() < 1e-5, "{r:?} vs {e:?}"); }
    }
}
