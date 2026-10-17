//! Debug fly camera: WASD + mouse look, Shift for speed. It only moves the view; the projection,
//! depth mapping and fog are the game's (`game_camera.rs`).

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::game_camera::CameraSource;

pub struct FlyCamPlugin;

impl Plugin for FlyCamPlugin {
    fn build(&self, app: &mut App) { app.add_systems(Update, (grab_cursor, fly_cam).chain()); }
}

#[derive(Component)]
pub struct FlyCam {
    /// Game units per second.
    pub speed: f32,
    /// Multiplier while Shift is held.
    pub fast: f32,
    /// Radians per pixel of mouse motion.
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl FlyCam {
    /// Takes yaw/pitch from an existing orientation so a `looking_at` transform is kept.
    pub fn from_transform(t: &Transform, speed: f32) -> Self {
        let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
        FlyCam { speed, fast: 5.0, sensitivity: 0.003, yaw, pitch }
    }
}

pub const CONTROLS: &str = "\
controls: hold right mouse (or press Tab to toggle) to look | WASD move | Space/E up, Ctrl/Q down
          Shift fast | mouse wheel changes speed | Esc releases the cursor | P prints the game camera (RC_CAM + VU matrix)";

/// Right mouse held or Tab toggles a locked, hidden cursor; Esc releases it. With gameplay on
/// (`CameraSource` present, crate::gameplay) Tab switches cameras instead and only the right button grabs. The cursor
/// is also hidden whenever the window has focus, and shows again when it loses it.
fn grab_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    source: Option<Res<CameraSource>>,
    window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut toggled: Local<bool>,
) {
    let (window, mut cursor) = window.into_inner();
    if keys.just_pressed(KeyCode::Tab) && source.is_none() { *toggled = !*toggled; }
    if keys.just_pressed(KeyCode::Escape) { *toggled = false; }
    let grab = *toggled || mouse.pressed(MouseButton::Right);
    let mode = if grab { CursorGrabMode::Locked } else { CursorGrabMode::None };
    if cursor.grab_mode != mode { cursor.grab_mode = mode; }
    let visible = !grab && !window.focused;
    if cursor.visible != visible { cursor.visible = visible; }
}

fn fly_cam(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    source: Option<Res<CameraSource>>,
    mut cams: Query<(&mut Transform, &mut FlyCam)>,
) {
    // Frozen while the game camera drives the view (crate::play_camera).
    if source.is_some_and(|s| *s == CameraSource::Play) { return; }
    let looking = cursor.grab_mode != CursorGrabMode::None;
    for (mut tf, mut cam) in &mut cams {
        if scroll.delta.y != 0.0 { cam.speed = (cam.speed * 1.2f32.powf(scroll.delta.y.signum())).clamp(0.5, 5000.0); }
        if looking && motion.delta != Vec2::ZERO {
            cam.yaw -= motion.delta.x * cam.sensitivity;
            cam.pitch = (cam.pitch - motion.delta.y * cam.sensitivity).clamp(-1.54, 1.54);
            tf.rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
        }
        let mut dir = Vec3::ZERO;
        let (fwd, right) = (tf.forward().as_vec3(), tf.right().as_vec3());
        if keys.pressed(KeyCode::KeyW) { dir += fwd; }
        if keys.pressed(KeyCode::KeyS) { dir -= fwd; }
        if keys.pressed(KeyCode::KeyD) { dir += right; }
        if keys.pressed(KeyCode::KeyA) { dir -= right; }
        if keys.any_pressed([KeyCode::Space, KeyCode::KeyE]) { dir += Vec3::Y; }
        if keys.any_pressed([KeyCode::ControlLeft, KeyCode::KeyQ]) { dir -= Vec3::Y; }
        if dir != Vec3::ZERO {
            let fast = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) { cam.fast } else { 1.0 };
            tf.translation += dir.normalize() * cam.speed * fast * time.delta_secs();
        }
    }
}
