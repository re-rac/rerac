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
//! Every connected pad counts, not just the first ([`Pads`]): a remapper's virtual pad (DualSenseX, DS4Windows, Steam
//! Input) sits next to the real pad it reads, which may report nothing, and on Linux gilrs also lists non-pads with axes
//! (a wireless keyboard receiver), which can come first. Buttons combine; each stick is the one deflected furthest. DualSense pads are read from their own reports (crate::dualsense) and their Bevy gamepad is
//! then skipped.
//!
//! Scripted input (`RC_PLAY_SCRIPT`), for deterministic runs: see [`Script`]. With a script the devices are
//! ignored.

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use rc_game::pad::{button, PadInput};

pub const CONTROLS: &str = "\
play controls (PS2 pad):  left stick = WASD / arrow keys (Shift: half stick = walk) | X = Space | [] = J | O = K | /\\ = L
                          L1 = Q | R1 = E or Ctrl (crouch) | L2 = Z | R2 = C | Start = Enter | Select = Backspace
                          d-pad = T/F/G/H | right stick = mouse with the right button held, or , and . | R3 = V or middle mouse
                          gamepad: sticks, South = X, East = O, West = [], North = /\\, bumpers L1/R1, triggers L2/R2,
                          Start/Select, stick clicks L3/R3, d-pad; DualSense over USB or Bluetooth, touchpad click = Select
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

/// SDL mappings for pads missing from gilrs' bundled SDL_GameControllerDB. Without one, gilrs maps evdev codes by
/// name, and `xpad` reports the Xbox X button as `BTN_X` (= `BTN_NORTH`): X and Y swap.
const EXTRA_SDL_MAPPINGS: &str = "\
03000000c82d00000f20000000020000,8BitDo Ultimate 3-mode Controller for Xbox,a:b0,b:b1,back:b6,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,dpup:h0.1,guide:b8,leftshoulder:b4,leftstick:b9,lefttrigger:a2,leftx:a0,lefty:a1,misc1:b11,rightshoulder:b5,rightstick:b10,righttrigger:a5,rightx:a3,righty:a4,start:b7,x:b2,y:b3,platform:Linux,";

/// Hands [`EXTRA_SDL_MAPPINGS`] to gilrs through `SDL_GAMECONTROLLERCONFIG` (Bevy builds gilrs itself, with no
/// mapping hook), before any mappings already there (the user's or Steam's): gilrs keeps the last line per UUID, so
/// theirs win. Call before any thread starts.
pub fn install_sdl_mappings() {
    let existing = std::env::var("SDL_GAMECONTROLLERCONFIG").unwrap_or_default();
    std::env::set_var("SDL_GAMECONTROLLERCONFIG", format!("{EXTRA_SDL_MAPPINGS}\n{existing}"));
}

/// This frame's pad read (sampled in `PreUpdate`, used by every tick of the frame).
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PadFrame(pub PadInput);

/// A Bevy stick value in [−1, 1] (y up) → the raw libpad2 byte (y down).
fn raw_axis(v: f32) -> u8 { (127.5 + 127.5 * v.clamp(-1.0, 1.0)).round().clamp(0.0, 255.0) as u8 }

/// One pad's state: sticks in Bevy's convention ([−1, 1], y up), buttons as libpad2 bits (`rc_game::pad::button`), and
/// R2's travel (0 released .. 1 fully pulled; 0 on a pad that only reports R2 as a button).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PadDevice {
    pub left: Vec2,
    pub right: Vec2,
    pub buttons: u32,
    pub r2: f32,
}

impl PadDevice {
    pub fn from_gamepad(g: &Gamepad) -> PadDevice {
        let mut buttons = 0;
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
        let r2 = g.get(GamepadButton::RightTrigger2).unwrap_or(0.0);
        PadDevice { left: g.left_stick(), right: g.right_stick(), buttons, r2 }
    }

    /// R2 as the fire button: R2 reads as ○ (the game's fire), so it no longer crouches; R1 still does (the game reads
    /// R1 and R2 as one crouch button, `held & (R1 | R2)`). It fires once R2 is pulled to `point` (0..1: where the held
    /// weapon's trigger effect clicks or resists, crate::haptics::FirePoint), so the shot follows the feel; a pad
    /// without R2's travel fires on its button.
    pub fn fire_on_r2(self, point: f32) -> PadDevice {
        let pulled = if self.r2 > 0.0 { self.r2 >= point.max(MIN_FIRE) } else { self.buttons & button::R2 != 0 };
        let buttons = self.buttons & !button::R2 | if pulled { button::CIRCLE } else { 0 };
        PadDevice { buttons, ..self }
    }

    /// Two pads as one: the buttons of both, each stick from the pad that deflects it further, R2's furthest travel.
    pub fn merge(self, o: PadDevice) -> PadDevice {
        let far = |a: Vec2, b: Vec2| if b.length_squared() > a.length_squared() { b } else { a };
        PadDevice { left: far(self.left, o.left), right: far(self.right, o.right), buttons: self.buttons | o.buttons, r2: self.r2.max(o.r2) }
    }
}

/// The least R2 travel that fires (a resting trigger reads a little above 0).
const MIN_FIRE: f32 = 0.12;

/// Every connected pad: Bevy's gamepads and the DualSense read directly (crate::dualsense), whose Bevy gamepad is
/// skipped while it is read there. With Port Options' "Fire button: R2" (crate::haptics::ControllerSettings) a pad's R2
/// reads as ○.
#[derive(SystemParam)]
pub struct Pads<'w, 's> {
    gamepads: Query<'w, 's, &'static Gamepad>,
    dualsense: Option<Res<'w, crate::dualsense::DualSense>>,
    controller: Option<Res<'w, crate::haptics::ControllerSettings>>,
    fire_point: Option<Res<'w, crate::haptics::FirePoint>>,
}

impl Pads<'_, '_> {
    /// All pads as one, `None` without any.
    pub fn device(&self) -> Option<PadDevice> {
        let pad = self.merged()?;
        let point = self.fire_point.as_ref().map_or(0.0, |p| p.0);
        Some(if self.controller.as_ref().is_some_and(|c| c.fire_r2) { pad.fire_on_r2(point) } else { pad })
    }

    fn merged(&self) -> Option<PadDevice> {
        let ds = self.dualsense.as_ref().and_then(|d| d.get());
        let is_ds = |g: &Gamepad| {
            g.vendor_id() == Some(crate::dualsense::VENDOR) && g.product_id().is_some_and(|p| crate::dualsense::PRODUCTS.contains(&p))
        };
        self.gamepads
            .iter()
            .filter(|g| ds.is_none() || !is_ds(g))
            .map(PadDevice::from_gamepad)
            .chain(ds)
            .reduce(PadDevice::merge)
    }
}

/// Keyboard + mouse + the pads ([`Pads::device`]) → one pad read.
pub fn sample(keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>, motion: &AccumulatedMouseMotion, pad: Option<PadDevice>) -> PadInput {
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

    if let Some(g) = pad {
        let (l, r) = (g.left, g.right);
        if l != Vec2::ZERO {
            p.lx = raw_axis(l.x * STICK_SCALE);
            p.ly = raw_axis(-l.y * STICK_SCALE);
        }
        if r != Vec2::ZERO {
            p.rx = raw_axis(r.x * STICK_SCALE);
            p.ry = raw_axis(-r.y * STICK_SCALE);
        }
        buttons |= g.buttons;
    }
    p.press(buttons)
}

/// Samples the devices into [`PadFrame`] (after Bevy's input systems).
pub fn sample_system(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    pads: Pads,
    mut frame: ResMut<PadFrame>,
) {
    frame.0 = sample(&keys, &mouse, &motion, pads.device());
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
    fn r2_fires_r1_still_crouches() {
        let pad = |buttons, r2| PadDevice { buttons, r2, ..Default::default() };
        // A pad without R2's travel: its button fires.
        assert_eq!(pad(button::R2 | button::CROSS, 0.0).fire_on_r2(0.8).buttons, button::CIRCLE | button::CROSS);
        assert_eq!(pad(button::R1, 0.0).fire_on_r2(0.0).buttons, button::R1);
        assert_eq!(pad(button::CIRCLE, 0.0).fire_on_r2(0.0).buttons, button::CIRCLE);
        // With R2's travel: only once pulled past the weapon's point (the button bit alone never fires, nor crouches).
        assert_eq!(pad(button::R2, 0.5).fire_on_r2(0.8).buttons, 0);
        assert_eq!(pad(button::R2, 0.85).fire_on_r2(0.8).buttons, button::CIRCLE);
        assert_eq!(pad(button::R2, 0.1).fire_on_r2(0.0).buttons, 0);
        assert_eq!(pad(button::R2, 0.2).fire_on_r2(0.0).buttons, button::CIRCLE);
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
