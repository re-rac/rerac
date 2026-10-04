//! **Port-only** (not in the game): the Port Options "Strafe", a Going Commando–style strafe. Off by default; the
//! original game is untouched while it is off.
//!
//! **What it does.** With the option on, holding **L2 or R2** on foot makes Ratchet face the way the camera looks
//! and keep facing it while the left stick moves him in any direction (Going Commando's strafe). ✕ with the stick
//! to a side or back is a strafe flip; flips chain while the button is held. While the option is on, L2 and R2 are
//! the strafe's alone for Ratchet: L1 keeps the look stance and R1 the crouch (Going Commando's layout).
//!
//! **Built from the game's own pieces**, no new movement model:
//! * the turn toward the camera is the one RaC1's own strafe uses, the Thruster-Pack hover's L2 / R2 turn
//!   (`0x2370b8` case 0x81: `TurnTo(0.017, 0.3, 4.712389·dt)` toward the camera yaw; [`super::packs`]);
//! * the movement is the walk's (state 2: the stick's table speed, walk 0.9 u/s or run 5.7 u/s, the walk's
//!   acceleration and deceleration) along the stick's direction (the walk's own `set_planar_vel(target)` path),
//!   with the walk / run sequences chosen by speed as usual;
//! * the flip is the side / back flip 0xb (`TryJump`'s flip, which RaC1 reaches from the crouch), its direction from
//!   the stick against the facing as there.
//!
//! [L] RaC1's Ratchet has no side-step or back-step sequences (Going Commando added them), so the walk / run
//! cycle plays whichever way he moves.
//!
//! | piece | where |
//! |---|---|
//! | L2 / R2 kept from the hero's pad while the option applies; [`Hero::strafe`] set | the tick (`crate::tick`), [`owns_buttons`] |
//! | facing in idle 0 / stop 3 | [`Hero::dispatch_physics`](super::registry) → [`face_camera`] |
//! | the walk 2: face the camera, move along the stick | `walk.rs` (`phys_walk`), [`move_yaw`] |
//! | no flick turn while strafing | `walk.rs` (`tr_walk`) |
//! | the strafe flip | `common.rs` (`try_jump`), [`flip`]; the chain: `jump.rs` (`tr_jump`) |
//! | the Thruster hover's own L2 / R2 turn keeps working | `packs.rs` (`hover_physics`) |

use super::physics::*;
use super::states::Ctx;
use super::Hero;
use crate::pad::{button, PadState};
use crate::ps2v::Pf;

/// The strafe buttons.
pub const BUTTONS: u32 = button::L2 | button::R2;

/// Whether the strafe owns L2 / R2 for the hero this tick: the option is on, Ratchet himself (body 0) is on foot,
/// not on the Hoverboard (its tricks read L2 / R2).
pub fn owns_buttons(h: &Hero) -> bool { h.strafe_mode && h.mode == 0 && !(0x6b..=0x6f).contains(&h.state) }

/// Sets [`Hero::strafe`] from this tick's pad and returns the pad the hero should see (L2 / R2 removed while the
/// strafe owns them; None: the pad as it is).
pub fn prepare(h: &mut Hero, pad: &PadState) -> Option<PadState> {
    let own = owns_buttons(h);
    h.strafe = own && pad.held & BUTTONS != 0;
    own.then(|| pad.without(BUTTONS))
}

/// The Thruster hover's strafe turn (`TurnTo(0.017, 0.3, 4.712389·dt)` toward the camera yaw), the target yaw kept.
pub(super) fn face_camera(h: &mut Hero, env: &Env) {
    let keep = h.target_yaw;
    h.target_yaw = env.cam_yaw;
    h.turn_to(SCALE64 * Pf::f(0.017), SCALE64 * Pf::f(0.3), DT * Pf::f(4.712_389));
    h.target_yaw = keep;
}

/// The direction the walk moves Ratchet while strafing: the stick's (after `StickTarget`), or the last one while the
/// stick is let go (the speed then runs down along it).
pub(super) fn move_yaw(h: &mut Hero) -> Pf {
    if Pf::ZERO < h.target_speed { h.strafe_yaw = h.target_yaw; }
    h.strafe_yaw
}

/// The strafe flip: strafing with the stick to a side or back (`stick_sector` 0 / 1 / 3, the stick past 0.2) →
/// the flip 0xb. False: the caller's own jump.
///
/// The flip's horizontal motion (`0x234b40` for 0xb) carries the entry speed (above 1.5 u/s) along the *facing*,
/// right for RaC1's flips, whose run-up is along the facing; a strafe's run-up is along the stick, so here the
/// same entry speed starts the flip's own speed along the stick instead.
pub(super) fn flip(h: &mut Hero, c: &mut Ctx) -> bool {
    if !h.strafe || h.stick_mag < Pf::f(0.2) || h.stick_sector(c) == 2 { return false; }
    h.set_state(c, 0xb, true);
    if h.state == 0xb { carry_along_stick(h); }
    true
}

/// The strafe's run-up into a flip 0xb (module doc of [`flip`]); also for a chained flip (`jump.rs`).
pub(super) fn carry_along_stick(h: &mut Hero) {
    h.jump.fwd_speed = h.jump.side_speed;
    h.jump.side_speed = Pf::ZERO;
}
