//! Keyboard, mouse and gamepad → the 18 libpad2 bytes the game reads each tick (`sceScfPad2Read`: buttons
//! active-low in bytes 0/1, then rx, ry, lx, ly, then the 12 pressures), so `rc_game::pad` decodes exactly
//! what a PS2 pad would give it (docs/plan/player_controller.md §1, "Engine wiring").
//!
//! Encoding. Sticks: 127/128 = centre, 0 = full up/left, 255 = full down/right; the game's per-axis dead
//! zone (|b − 127| < 48) and scale (/76) are applied by `PadState`, not here. A gamepad stick is scaled per
//! axis by [`STICK_SCALE`] and clamped, then mapped linearly (`round(127.5 + 127.5·v)`, Bevy's y up → PS2 y
//! down): the DualShock 2's range, which reads full on both axes at a full diagonal; keys give full deflection
//! (0 / 255), or half deflection (`PadInput::axis_byte(±0.5)`) with Shift held. Buttons are digital: pressure
//! 0xff while held (`PadInput::press`).
//!
//! Scripted input (`RC_PLAY_SCRIPT`), for deterministic runs: see [`Script`]. With a script the devices are
//! ignored.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use rc_game::pad::{button, PadInput};

pub const CONTROLS: &str = "\
play controls (PS2 pad):  left stick = WASD / arrow keys (Shift: half stick = walk) | X = Space | [] = J | O = K | /\\ = L
                          L1 = Q | R1 = E or Ctrl (crouch) | L2 = Z | R2 = C | Start = Enter | Select = Backspace
                          d-pad = T/F/G/H | right stick = mouse with the right button held, or , and . | R3 = V or middle mouse
                          gamepad: sticks, South = X, East = O, West = [], North = /\\, bumpers L1/R1, triggers L2/R2,
                          Start/Select, stick clicks L3/R3, d-pad
engine keys:              Tab = fly camera <-> game camera (the fly camera takes the keys; the pad is neutral) |
                          R = respawn at the level's uid-0 moby | P = print the camera | Esc releases the cursor
                          F9 = dump the hero and the mobys near him | F10 = the respawn state (compare with work/scratch/respawn_probe.py) | N = noclip (dev builds)";

/// A gamepad stick's scale per axis before the byte, clamped (PCSX2's DualShock 2 default, 133 %). A DualShock 2
/// reads full on both axes at a full diagonal; a modern pad's round gate gives 0.71 on each, which the game's dead
/// zone and scale decode to a stick length of 0.80, under the walk / run table's 0.82 (`SPEED_TABLE`): held fully
/// toward a diagonal, Ratchet walked. Scaled, a full diagonal decodes to full on both axes, the right stick turns the
/// camera at full rate from about three quarters of its travel, and the game's dead zone (48 of 127) covers about
/// 28 % of the travel instead of 38 %.
pub const STICK_SCALE: f32 = 1.33;

/// Mouse pixels per frame for a full right-stick deflection.
const MOUSE_FULL: f32 = 12.0;

/// This frame's pad read (sampled in `PreUpdate`, used by every tick of the frame).
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PadFrame(pub PadInput);

/// A Bevy stick value in [−1, 1] (y up) → the raw libpad2 byte (y down).
fn raw_axis(v: f32) -> u8 { (127.5 + 127.5 * v.clamp(-1.0, 1.0)).round().clamp(0.0, 255.0) as u8 }

/// Keyboard + mouse + the first gamepad → one pad read.
pub fn sample(keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>, motion: &AccumulatedMouseMotion, gamepad: Option<&Gamepad>) -> PadInput {
    let mut p = PadInput::neutral();
    let mut buttons = 0u32;

    // Keyboard left stick (per axis full or half deflection).
    let k = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let x = k(KeyCode::KeyD, KeyCode::ArrowRight) as i32 - k(KeyCode::KeyA, KeyCode::ArrowLeft) as i32;
    let y = k(KeyCode::KeyS, KeyCode::ArrowDown) as i32 - k(KeyCode::KeyW, KeyCode::ArrowUp) as i32;
    if x != 0 || y != 0 {
        let walk = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let byte = |v: i32| if v == 0 { 0x7f } else if walk { PadInput::axis_byte(0.5 * v as f32) } else if v < 0 { 0 } else { 0xff };
        p.lx = byte(x);
        p.ly = byte(y);
    }
    // Right stick: mouse while the right button is held (the fly camera's grab locks the cursor), or , and .
    let rkx = keys.pressed(KeyCode::Period) as i32 - keys.pressed(KeyCode::Comma) as i32;
    if rkx != 0 { p.rx = if rkx < 0 { 0 } else { 0xff }; }
    if mouse.pressed(MouseButton::Right) && motion.delta != Vec2::ZERO {
        p.rx = raw_axis(motion.delta.x / MOUSE_FULL);
        p.ry = raw_axis(-motion.delta.y / MOUSE_FULL);
    }
    for (key, b) in [
        (KeyCode::Space, button::CROSS),
        (KeyCode::KeyJ, button::SQUARE),
        (KeyCode::KeyK, button::CIRCLE),
        (KeyCode::KeyL, button::TRIANGLE),
        (KeyCode::KeyQ, button::L1),
        (KeyCode::KeyE, button::R1),
        (KeyCode::ControlLeft, button::R1),
        (KeyCode::KeyZ, button::L2),
        (KeyCode::KeyC, button::R2),
        (KeyCode::Enter, button::START),
        (KeyCode::Backspace, button::SELECT),
        (KeyCode::KeyV, button::R3),
        (KeyCode::KeyT, button::UP),
        (KeyCode::KeyG, button::DOWN),
        (KeyCode::KeyF, button::LEFT),
        (KeyCode::KeyH, button::RIGHT),
    ] {
        if keys.pressed(key) { buttons |= b; }
    }
    if mouse.pressed(MouseButton::Middle) { buttons |= button::R3; }

    if let Some(g) = gamepad {
        let (l, r) = (g.left_stick(), g.right_stick());
        if l != Vec2::ZERO {
            p.lx = raw_axis(l.x * STICK_SCALE);
            p.ly = raw_axis(-l.y * STICK_SCALE);
        }
        if r != Vec2::ZERO {
            p.rx = raw_axis(r.x * STICK_SCALE);
            p.ry = raw_axis(-r.y * STICK_SCALE);
        }
        for (gb, b) in [
            (GamepadButton::South, button::CROSS),
            (GamepadButton::West, button::SQUARE),
            (GamepadButton::East, button::CIRCLE),
            (GamepadButton::North, button::TRIANGLE),
            (GamepadButton::LeftTrigger, button::L1),
            (GamepadButton::RightTrigger, button::R1),
            (GamepadButton::LeftTrigger2, button::L2),
            (GamepadButton::RightTrigger2, button::R2),
            (GamepadButton::Start, button::START),
            (GamepadButton::Select, button::SELECT),
            (GamepadButton::LeftThumb, button::L3),
            (GamepadButton::RightThumb, button::R3),
            (GamepadButton::DPadUp, button::UP),
            (GamepadButton::DPadDown, button::DOWN),
            (GamepadButton::DPadLeft, button::LEFT),
            (GamepadButton::DPadRight, button::RIGHT),
        ] {
            if g.pressed(gb) { buttons |= b; }
        }
    }
    p.press(buttons)
}

/// Samples the devices into [`PadFrame`] (after Bevy's input systems).
pub fn sample_system(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    gamepads: Query<&Gamepad>,
    mut frame: ResMut<PadFrame>,
) {
    frame.0 = sample(&keys, &mouse, &motion, gamepads.iter().next());
}

/// `RC_PLAY_SCRIPT`: pad input per gameplay tick, `item,item,...` with `item = range:action`.
///
/// * `range`: `a-b` (ticks a through b, inclusive) or `a`. Tick t is the (t+1)-th gameplay tick since the
///   level started (`Game::counter` before the tick); in frame-exact mode (`RC_SCREENSHOT_FRAME` /
///   `RC_DETERMINISTIC=1`) tick t runs in update t+1, so `RC_SCREENSHOT_FRAME=N` shows the state after
///   ticks 0..N−1.
/// * `action`: `stick x y` (left stick, x right, y down, each in [−1, 1]: `stick 0 -1` = full forward;
///   `PadInput::stick`, the byte that decodes to exactly that value), `rstick x y` (right stick), or
///   `press B[+B...]` (hold buttons: `X`/`CROSS`, `SQUARE`, `CIRCLE`/`O`, `TRIANGLE`, `L1`, `R1`, `L2`, `R2`,
///   `L3`, `R3`, `START`, `SELECT`, `UP`, `DOWN`, `LEFT`, `RIGHT`).
///
/// Items whose ranges overlap combine (the last stick item wins). Ticks no item covers are neutral.
/// Example: `0-179:stick 0 -1` runs forward for 180 ticks; `60-90:press X` holds jump for 31 ticks.
#[derive(Clone, Debug, Default)]
pub struct Script {
    items: Vec<(u64, u64, Action)>,
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Stick(f32, f32),
    RStick(f32, f32),
    Press(u32),
}

fn button_named(s: &str) -> Option<u32> {
    Some(match s.to_ascii_uppercase().as_str() {
        "X" | "CROSS" => button::CROSS,
        "SQUARE" => button::SQUARE,
        "O" | "CIRCLE" => button::CIRCLE,
        "TRIANGLE" => button::TRIANGLE,
        "L1" => button::L1,
        "R1" => button::R1,
        "L2" => button::L2,
        "R2" => button::R2,
        "L3" => button::L3,
        "R3" => button::R3,
        "START" => button::START,
        "SELECT" => button::SELECT,
        "UP" => button::UP,
        "DOWN" => button::DOWN,
        "LEFT" => button::LEFT,
        "RIGHT" => button::RIGHT,
        _ => return None,
    })
}

impl Script {
    pub fn parse(s: &str) -> Result<Script, String> {
        let mut items = Vec::new();
        for item in s.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            let (range, action) = item.split_once(':').ok_or_else(|| format!("{item:?}: expected range:action"))?;
            let num = |v: &str| v.trim().parse::<u64>().map_err(|_| format!("{item:?}: bad tick {v:?}"));
            let (a, b) = match range.split_once('-') {
                Some((a, b)) => (num(a)?, num(b)?),
                None => (num(range)?, num(range)?),
            };
            if b < a { return Err(format!("{item:?}: empty range")); }
            let words: Vec<&str> = action.split_whitespace().collect();
            let f = |v: &str| v.parse::<f32>().map_err(|_| format!("{item:?}: bad number {v:?}"));
            let action = match words.as_slice() {
                ["stick", x, y] => Action::Stick(f(x)?, f(y)?),
                ["rstick", x, y] => Action::RStick(f(x)?, f(y)?),
                ["press", names] => Action::Press(
                    names.split('+').try_fold(0u32, |m, n| button_named(n).map(|b| m | b).ok_or_else(|| format!("{item:?}: unknown button {n:?}")))?,
                ),
                _ => return Err(format!("{item:?}: expected `stick x y`, `rstick x y` or `press B[+B]`")),
            };
            items.push((a, b, action));
        }
        Ok(Script { items })
    }

    /// The pad read of tick `t`.
    pub fn at(&self, t: u64) -> PadInput {
        let mut p = PadInput::neutral();
        for &(a, b, act) in &self.items {
            if t < a || t > b { continue; }
            p = match act {
                Action::Stick(x, y) => p.stick(x, y),
                Action::RStick(x, y) => p.rstick(x, y),
                Action::Press(m) => p.press(m),
            };
        }
        p
    }

    /// The last tick any item covers.
    pub fn end(&self) -> u64 { self.items.iter().map(|i| i.1).max().unwrap_or(0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_game::pad::{axis, PadState};

    #[test]
    fn script_ranges_and_actions() {
        let s = Script::parse("0-120:stick 0 -1, 60:press X+R1, 200-201:rstick 1 0").unwrap();
        assert_eq!(s.at(0), PadInput::neutral().stick(0.0, -1.0));
        assert_eq!(s.at(60), PadInput::neutral().stick(0.0, -1.0).press(button::CROSS | button::R1));
        assert_eq!(s.at(121), PadInput::neutral());
        assert_eq!(s.at(201).rx, PadInput::axis_byte(1.0));
        assert_eq!(s.end(), 201);
        assert!(Script::parse("5-1:press X").is_err());
        assert!(Script::parse("1:press Y").is_err());
        assert!(Script::parse("1:jump").is_err());
    }

    #[test]
    fn device_encoding_decodes_like_a_pad() {
        // Linear gamepad bytes: centre decodes to 0, full deflection to ±1, up/left negative.
        assert_eq!(axis(raw_axis(0.0)).to_f32(), 0.0);
        assert_eq!(axis(raw_axis(1.0)).to_f32(), 1.0);
        assert_eq!(axis(raw_axis(-1.0)).to_f32(), -1.0);
        assert_eq!(axis(raw_axis(0.3)).to_f32(), 0.0); // inside the game's dead zone (|b - 127| < 48)
        // Keyboard forward = stick up = ly byte 0.
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyW);
        keys.press(KeyCode::Space);
        let p = sample(&keys, &ButtonInput::default(), &AccumulatedMouseMotion::default(), None);
        let mut pad = PadState::default();
        pad.update(Some(&p.bytes()), false);
        assert_eq!((pad.lx.to_f32(), pad.ly.to_f32()), (0.0, -1.0));
        assert!(pad.pressed & button::CROSS != 0);
        // Shift: half stick.
        keys.press(KeyCode::ShiftLeft);
        let p = sample(&keys, &ButtonInput::default(), &AccumulatedMouseMotion::default(), None);
        pad.update(Some(&p.bytes()), false);
        assert!((pad.ly.to_f32() + 0.5).abs() < 0.01);
    }
}
