//! **Port-only: Apple game controllers** (iOS). Bevy reads pads through gilrs, which has no iOS backend, so on iOS
//! the first connected extended gamepad (DualSense, DualShock 4, Xbox, MFi, Backbone, ...) is polled through Apple's
//! GameController framework each frame and merged into [`PadFrame`] like a Bevy gamepad (crate::input_map: same stick
//! scale, the face buttons by position: A = ✕, B = ○, X = □, Y = △). Elsewhere [`connected`] is always false and the
//! plugin adds nothing.

use bevy::prelude::*;

#[cfg(target_os = "ios")]
mod imp {
    use crate::input_map::{raw_axis, PadFrame, STICK_SCALE};
    use bevy::prelude::*;
    use objc2_game_controller::{GCController, GCControllerButtonInput, GCControllerDirectionPad, GCExtendedGamepad};
    use rc_game::pad::button;
    use std::sync::atomic::{AtomicBool, Ordering};

    static CONNECTED: AtomicBool = AtomicBool::new(false);

    pub fn connected() -> bool { CONNECTED.load(Ordering::Relaxed) }

    /// The GameController framework is used from the main thread only (this marker keeps the system there).
    #[derive(Default)]
    pub struct MainThread;

    fn pressed(b: &GCControllerButtonInput) -> bool { unsafe { b.isPressed() } }

    fn stick(d: &GCControllerDirectionPad) -> Vec2 { unsafe { Vec2::new(d.xAxis().value(), d.yAxis().value()) } }

    /// The first extended gamepad's state as (buttons, left stick, right stick), sticks y up.
    fn read(g: &GCExtendedGamepad) -> (u32, Vec2, Vec2) {
        let mut b = 0u32;
        unsafe {
            let dpad = g.dpad();
            for (input, mask) in [
                (g.buttonA(), button::CROSS),
                (g.buttonB(), button::CIRCLE),
                (g.buttonX(), button::SQUARE),
                (g.buttonY(), button::TRIANGLE),
                (g.leftShoulder(), button::L1),
                (g.rightShoulder(), button::R1),
                (g.leftTrigger(), button::L2),
                (g.rightTrigger(), button::R2),
                (g.buttonMenu(), button::START),
                (dpad.up(), button::UP),
                (dpad.down(), button::DOWN),
                (dpad.left(), button::LEFT),
                (dpad.right(), button::RIGHT),
            ] {
                if pressed(&input) { b |= mask; }
            }
            for (input, mask) in [(g.buttonOptions(), button::SELECT), (g.leftThumbstickButton(), button::L3), (g.rightThumbstickButton(), button::R3)] {
                if input.is_some_and(|i| pressed(&i)) { b |= mask; }
            }
            (b, stick(&g.leftThumbstick()), stick(&g.rightThumbstick()))
        }
    }

    pub fn poll(_main: NonSend<MainThread>, mut frame: ResMut<PadFrame>) {
        // SAFETY: GameController's class and instance getters, called on the main thread (NonSend above).
        let pad = unsafe { GCController::controllers() }.iter().find_map(|c| unsafe { c.extendedGamepad() });
        CONNECTED.store(pad.is_some(), Ordering::Relaxed);
        let Some(g) = pad else { return };
        let (buttons, l, r) = read(&g);
        let mut p = frame.0;
        if l != Vec2::ZERO { (p.lx, p.ly) = (raw_axis(l.x * STICK_SCALE), raw_axis(-l.y * STICK_SCALE)); }
        if r != Vec2::ZERO { (p.rx, p.ry) = (raw_axis(r.x * STICK_SCALE), raw_axis(-r.y * STICK_SCALE)); }
        frame.0 = p.press(buttons);
    }
}

/// A game controller is connected through the GameController framework (iOS only).
pub fn connected() -> bool {
    #[cfg(target_os = "ios")]
    return imp::connected();
    #[cfg(not(target_os = "ios"))]
    false
}

pub struct GcPadPlugin;

impl Plugin for GcPadPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(target_os = "ios")]
        _app.init_non_send::<imp::MainThread>()
            .add_systems(PreUpdate, imp::poll.after(crate::input_map::sample_system));
    }
}
