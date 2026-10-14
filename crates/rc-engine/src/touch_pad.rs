//! **Port-only: on-screen touch controls** (iOS; `RC_TOUCH=1` on other platforms, for a touch screen).
//!
//! The controls are drawn by the present camera (crate::display) over the game frame, so they sit in the bars beside
//! the 4:3 picture on a phone held sideways and never touch the game's own frame. Every touch is classified once, when
//! it starts: on a button (nearest within its hit radius) it holds that button until it lifts; otherwise its side of
//! the screen decides: left = the left stick, right = the right stick (camera), both floating (the stick's centre is
//! where the finger landed). Each frame the held controls are merged into [`PadFrame`] after the keyboard / gamepad
//! sample (crate::input_map), so the game reads the same 18 libpad2 bytes it reads from a pad.
//!
//! The overlay hides while a game controller is connected (crate::gc_pad on iOS, Bevy's gamepads elsewhere).

use bevy::camera::visibility::RenderLayers;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::sprite_render::AlphaMode2d;
use bevy::window::PrimaryWindow;
use rc_game::pad::button;
use std::collections::HashMap;

use crate::display::PRESENT_LAYER;
use crate::input_map::{raw_axis, PadFrame, STICK_SCALE};

/// Window logical pixels for a full stick deflection.
const STICK_RANGE: f32 = 56.0;
/// A button's hit radius as a multiple of its drawn radius (fingers are wider than the circles).
const HIT_SCALE: f32 = 1.35;
/// The overlay's resting and pressed opacity.
const ALPHA_IDLE: f32 = 0.30;
const ALPHA_HELD: f32 = 0.65;
/// Above the game frame's sprite (z 0) on the present camera.
const Z: f32 = 50.0;

/// Touch controls on: always on iOS, `RC_TOUCH=1` elsewhere.
pub fn enabled() -> bool { cfg!(target_os = "ios") || std::env::var("RC_TOUCH").is_ok_and(|v| v.trim() == "1") }

/// One on-screen button: its pad bit, label, colour and drawn radius.
#[derive(Clone, Copy, Debug)]
struct ButtonSpec {
    mask: u32,
    label: &'static str,
    color: [u8; 3],
    radius: f32,
}

const FACE: f32 = 27.0;
const SHOULDER: f32 = 26.0;
const DPAD: f32 = 19.0;
const SMALL: f32 = 18.0;

/// Every button, in drawing order; [`button_center`] places them.
const BUTTONS: [ButtonSpec; 14] = [
    ButtonSpec { mask: button::CROSS, label: "X", color: [120, 160, 255], radius: FACE },
    ButtonSpec { mask: button::CIRCLE, label: "O", color: [255, 110, 110], radius: FACE },
    ButtonSpec { mask: button::SQUARE, label: "[]", color: [255, 140, 220], radius: FACE },
    ButtonSpec { mask: button::TRIANGLE, label: "/\\", color: [110, 230, 170], radius: FACE },
    ButtonSpec { mask: button::L1, label: "L1", color: [220, 220, 220], radius: SHOULDER },
    ButtonSpec { mask: button::L2, label: "L2", color: [220, 220, 220], radius: SHOULDER },
    ButtonSpec { mask: button::R1, label: "R1", color: [220, 220, 220], radius: SHOULDER },
    ButtonSpec { mask: button::R2, label: "R2", color: [220, 220, 220], radius: SHOULDER },
    ButtonSpec { mask: button::UP, label: "^", color: [200, 200, 200], radius: DPAD },
    ButtonSpec { mask: button::DOWN, label: "v", color: [200, 200, 200], radius: DPAD },
    ButtonSpec { mask: button::LEFT, label: "<", color: [200, 200, 200], radius: DPAD },
    ButtonSpec { mask: button::RIGHT, label: ">", color: [200, 200, 200], radius: DPAD },
    ButtonSpec { mask: button::SELECT, label: "SEL", color: [200, 200, 200], radius: SMALL },
    ButtonSpec { mask: button::START, label: "START", color: [200, 200, 200], radius: SMALL },
];

/// A button's centre in window logical pixels (origin top-left, y down) for a window of size `w`. The layout keeps
/// clear of a landscape iPhone's notch / Dynamic Island (about 60 points on either long side).
fn button_center(mask: u32, w: Vec2) -> Vec2 {
    let face = Vec2::new(w.x - 165.0, w.y - 130.0);
    let dpad = Vec2::new(125.0, 150.0);
    let (f, d) = (56.0, 38.0);
    match mask {
        button::CROSS => face + Vec2::new(0.0, f),
        button::CIRCLE => face + Vec2::new(f, 0.0),
        button::SQUARE => face + Vec2::new(-f, 0.0),
        button::TRIANGLE => face + Vec2::new(0.0, -f),
        button::L2 => Vec2::new(105.0, 42.0),
        button::L1 => Vec2::new(180.0, 42.0),
        button::R1 => Vec2::new(w.x - 180.0, 42.0),
        button::R2 => Vec2::new(w.x - 105.0, 42.0),
        button::UP => dpad + Vec2::new(0.0, -d),
        button::DOWN => dpad + Vec2::new(0.0, d),
        button::LEFT => dpad + Vec2::new(-d, 0.0),
        button::RIGHT => dpad + Vec2::new(d, 0.0),
        button::SELECT => Vec2::new(w.x * 0.5 - 60.0, 26.0),
        button::START => Vec2::new(w.x * 0.5 + 60.0, 26.0),
        _ => Vec2::ZERO,
    }
}

/// The left stick's drawn resting place (where it shows while no finger holds it).
fn left_stick_rest(w: Vec2) -> Vec2 { Vec2::new(170.0, w.y - 110.0) }

/// What a touch controls, decided when it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    LeftStick,
    RightStick,
    Button(u32),
}

/// The control a touch starting at `p` (window logical pixels) takes.
fn classify(p: Vec2, w: Vec2) -> Control {
    let hit = BUTTONS
        .iter()
        .map(|b| (b.mask, p.distance(button_center(b.mask, w)), b.radius * HIT_SCALE))
        .filter(|&(_, d, r)| d <= r)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    match hit {
        Some((mask, _, _)) => Control::Button(mask),
        None if p.x < w.x * 0.5 => Control::LeftStick,
        None => Control::RightStick,
    }
}

/// A floating stick's value in [−1, 1] per axis (y down, as the pad's bytes), from its centre and the finger.
fn stick_value(origin: Vec2, p: Vec2) -> Vec2 {
    let v = (p - origin) / STICK_RANGE;
    if v.length() > 1.0 { v.normalize() } else { v }
}

/// The touches held now: each touch's control and where it started.
#[derive(Resource, Default)]
struct TouchState {
    owners: HashMap<u64, (Control, Vec2)>,
    /// This frame's result, for the overlay.
    held: u32,
    left: Option<(Vec2, Vec2)>,
    right: Option<(Vec2, Vec2)>,
}

/// The overlay's parts.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Part {
    Button(u32),
    LeftBase,
    LeftKnob,
    RightBase,
    RightKnob,
}

/// The overlay's root visibility (hidden while a controller is connected).
#[derive(Component)]
struct Overlay;

pub struct TouchPadPlugin;

impl Plugin for TouchPadPlugin {
    fn build(&self, app: &mut App) {
        if !enabled() { return; }
        println!("touch controls: on");
        app.init_resource::<TouchState>()
            .add_systems(Startup, spawn_overlay)
            .add_systems(PreUpdate, apply_touches.after(crate::input_map::sample_system))
            .add_systems(Update, draw_overlay);
    }
}

/// Classifies new touches, forgets lifted ones and merges the held controls into this frame's pad read.
fn apply_touches(touches: Res<Touches>, window: Option<Single<&Window, With<PrimaryWindow>>>, mut st: ResMut<TouchState>, mut frame: ResMut<PadFrame>) {
    let Some(window) = window else { return };
    let w = Vec2::new(window.width(), window.height());
    for t in touches.iter_just_pressed() {
        let c = classify(t.start_position(), w);
        st.owners.insert(t.id(), (c, t.start_position()));
    }
    for t in touches.iter_just_released().chain(touches.iter_just_canceled()) {
        st.owners.remove(&t.id());
    }
    // A touch the window lost track of (backgrounding) must not stay held.
    st.owners.retain(|id, _| touches.get_pressed(*id).is_some());

    let (mut held, mut left, mut right) = (0u32, None, None);
    for t in touches.iter() {
        let Some(&(c, origin)) = st.owners.get(&t.id()) else { continue };
        match c {
            Control::Button(m) => held |= m,
            Control::LeftStick => left = Some((origin, t.position())),
            Control::RightStick => right = Some((origin, t.position())),
        }
    }
    (st.held, st.left, st.right) = (held, left, right);

    let mut p = frame.0;
    if let Some((o, at)) = left {
        let v = stick_value(o, at);
        if v != Vec2::ZERO { (p.lx, p.ly) = (raw_axis(v.x * STICK_SCALE), raw_axis(v.y * STICK_SCALE)); }
    }
    if let Some((o, at)) = right {
        let v = stick_value(o, at);
        if v != Vec2::ZERO { (p.rx, p.ry) = (raw_axis(v.x * STICK_SCALE), raw_axis(v.y * STICK_SCALE)); }
    }
    frame.0 = p.press(held);
}

fn spawn_overlay(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    let layer = RenderLayers::layer(PRESENT_LAYER);
    let mut material = |rgb: [u8; 3]| {
        materials.add(ColorMaterial { color: Color::srgba_u8(rgb[0], rgb[1], rgb[2], 255).with_alpha(ALPHA_IDLE), alpha_mode: AlphaMode2d::Blend, ..default() })
    };
    let root = commands
        .spawn((Overlay, Transform::default(), Visibility::default(), crate::level_switch::KeepAcrossLevels, Name::new("touch controls")))
        .id();
    let mut part = |commands: &mut Commands, p: Part, radius: f32, rgb: [u8; 3], label: &str, z: f32| {
        let mut e = commands.spawn((
            p,
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(material(rgb)),
            Transform::from_xyz(0.0, 0.0, Z + z),
            layer.clone(),
            ChildOf(root),
        ));
        if !label.is_empty() {
            e.with_child((
                Text2d::new(label),
                TextFont { font_size: FontSize::Px((radius * 0.75).clamp(10.0, 20.0)), ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                Transform::from_xyz(0.0, 0.0, 1.0),
                layer.clone(),
            ));
        }
    };
    for b in BUTTONS {
        part(&mut commands, Part::Button(b.mask), b.radius, b.color, b.label, 0.0);
    }
    part(&mut commands, Part::LeftBase, STICK_RANGE + 10.0, [200, 200, 200], "", 0.0);
    part(&mut commands, Part::LeftKnob, 26.0, [240, 240, 240], "", 2.0);
    part(&mut commands, Part::RightBase, STICK_RANGE + 10.0, [200, 200, 200], "", 0.0);
    part(&mut commands, Part::RightKnob, 26.0, [240, 240, 240], "", 2.0);
}

/// Places the overlay for the window's size and shows what is held; hides it while a controller is connected.
fn draw_overlay(
    st: Res<TouchState>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    gamepads: Query<(), With<Gamepad>>,
    mut root: Query<&mut Visibility, With<Overlay>>,
    mut parts: Query<(&Part, &mut Transform, &MeshMaterial2d<ColorMaterial>, &mut Visibility), Without<Overlay>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Some(window) = window else { return };
    let w = Vec2::new(window.width(), window.height());
    let controller = !gamepads.is_empty() || crate::gc_pad::connected();
    for mut v in &mut root {
        let want = if controller { Visibility::Hidden } else { Visibility::Inherited };
        if *v != want { *v = want; }
    }
    if controller { return; }
    // Window (top-left origin, y down) → the present camera's world (centre origin, y up).
    let world = |p: Vec2| Vec2::new(p.x - w.x * 0.5, w.y * 0.5 - p.y);
    for (part, mut tf, mat, mut vis) in &mut parts {
        let (at, shown, held) = match *part {
            Part::Button(m) => (button_center(m, w), true, st.held & m != 0),
            Part::LeftBase => (st.left.map_or(left_stick_rest(w), |(o, _)| o), true, st.left.is_some()),
            Part::LeftKnob => (st.left.map_or(left_stick_rest(w), |(o, p)| o + stick_value(o, p) * STICK_RANGE), true, st.left.is_some()),
            Part::RightBase => (st.right.map_or(Vec2::ZERO, |(o, _)| o), st.right.is_some(), true),
            Part::RightKnob => (st.right.map_or(Vec2::ZERO, |(o, p)| o + stick_value(o, p) * STICK_RANGE), st.right.is_some(), true),
        };
        let pos = world(at);
        if tf.translation.truncate() != pos { (tf.translation.x, tf.translation.y) = (pos.x, pos.y); }
        let want = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want { *vis = want; }
        let alpha = if held { ALPHA_HELD } else { ALPHA_IDLE };
        if let Some(m) = materials.get(&mat.0) {
            if (m.color.alpha() - alpha).abs() > f32::EPSILON {
                if let Some(mut m) = materials.get_mut(&mat.0) { m.color.set_alpha(alpha); }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHONE: Vec2 = Vec2::new(852.0, 393.0);
    const SMALL_PHONE: Vec2 = Vec2::new(667.0, 375.0);

    #[test]
    fn buttons_are_hit_at_their_centres() {
        for w in [PHONE, SMALL_PHONE] {
            for b in BUTTONS {
                assert_eq!(classify(button_center(b.mask, w), w), Control::Button(b.mask), "{} at {w}", b.label);
            }
        }
    }

    #[test]
    fn buttons_stay_on_screen_and_apart() {
        for w in [PHONE, SMALL_PHONE] {
            for (i, a) in BUTTONS.iter().enumerate() {
                let c = button_center(a.mask, w);
                assert!(c.x - a.radius >= 0.0 && c.x + a.radius <= w.x && c.y - a.radius >= 0.0 && c.y + a.radius <= w.y, "{} off screen at {w}", a.label);
                for b in &BUTTONS[i + 1..] {
                    let d = c.distance(button_center(b.mask, w));
                    assert!(d >= a.radius + b.radius, "{} and {} overlap at {w}", a.label, b.label);
                }
            }
        }
    }

    #[test]
    fn free_space_is_a_stick_by_side() {
        assert_eq!(classify(Vec2::new(250.0, 330.0), PHONE), Control::LeftStick);
        assert_eq!(classify(Vec2::new(500.0, 200.0), PHONE), Control::RightStick);
    }

    #[test]
    fn stick_values_clamp_to_the_unit_circle() {
        let o = Vec2::new(100.0, 100.0);
        assert_eq!(stick_value(o, o), Vec2::ZERO);
        assert_eq!(stick_value(o, o + Vec2::new(STICK_RANGE * 0.5, 0.0)), Vec2::new(0.5, 0.0));
        let v = stick_value(o, o + Vec2::new(500.0, 500.0));
        assert!((v.length() - 1.0).abs() < 1e-5);
        // Finger moved down: the pad's y grows (down), as the PS2 byte does.
        assert!(raw_axis(stick_value(o, o + Vec2::new(0.0, STICK_RANGE)).y) > 200);
    }
}
