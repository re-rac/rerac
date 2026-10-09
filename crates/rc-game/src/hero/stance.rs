//! **Package P2 (with damage) — stances and scripted walking.** Addresses level01 (0x40's SetState and 0x41's
//! physics from level00, which has the same code).
//!
//! * **1 look stance** (group 0, 0x1415d4 = 4; L1 / L2 held in idle, walk, stop or crouch; the camera side is
//!   follow_camera's). Entry: the stance sequence `0x226f10(1)` over 18 ticks unless it already plays (or a
//!   weapon is out, 0x1413fa). Physics: the ground case 0x23713c ([`Hero::phys_ground`]). Transitions: R1 / R2
//!   held → the crouch sequence 0xd, else the stance sequence (blend 8); L1 / L2 held, or the wrench (class 0x47)
//!   in a non-zero state in the hand, keeps the stance; released: in a body its idle (`0x227638`: Clank 0x43, Giant
//!   Clank 0x5a, the disguise 0x53), else R1 / R2 → crouch 4, else idle 0 without an anim
//!   change and the stance sequence blended in over 8 when it is not already playing.
//! * **0x1e** — the same stance set by mobys (four classes; 0x1415d4 = 0x51, blend 5, no `play` test); releases
//!   to idle the same way (`0x16cc00 = 0`, a camera word, is not modelled).
//! * **0x40 fidget state** (fidget record 2 of [`super::idle`], chance 0: never picked on the disc): no SetState
//!   case (the group stays), physics: velocity 0; transitions: L1 / L2 → 1; airborne, ✕ or a move → the end
//!   0x41; R1 / R2 → 0x41; no stick and the sequence wrapped → idle. **0x41** its end: the idle sequence over 9
//!   (the fidget moby 0x140964 of `0x241d98` is not modelled: 0 in the port), no physics; after 8 ticks → fall 6
//!   in the air, a jump (`TryJump(7)`), crouch 4, walk 2 or idle 0.
//! * **0x65 / 0x66 / 0x67 walk to a point** (vendors, the ship, cutscene marks: the scripts set the point
//!   0x140990 and the facing 0x14099c = [`WalkTo`], then SetState(0x65)). 0x65 walks (the walk speed table: 0.9
//!   u/s, 5.7 beyond 0.82 units; stick magnitude forced to 1) with the run turn, SpeedStep(7.5, 8.5)·dt², the
//!   walk / run anim; within 0.2 it turns to the facing and at < 2.7 u/s and 2° off → 0x67 (stand: velocity 0,
//!   idle anim; back to 0x65 when pushed off the point, to idle on the wrap or 0x1409a0). 0x66 (the stop) decays
//!   to 0x65 on its wrap.
//!
//! **First person** (0x1413f5, `Hero::f13f5`): the first-person camera (camera type 4, `crate::follow_camera`) sets
//! it once its blend-in is over; the ground physics then turns Ratchet to the camera ([`first_person_turn`]).
//! Not modelled: the first-person play-time stat, the weapon put-away `0x22efd8` (0x1413f8 is 0 on foot). The walk
//! lean `HeroLean` (0x235638) is `Hero::lean` (`super::idle`), called from the walk / air / jump / pack physics.

use super::anim::AnimCtl;
use super::common::blend;
use super::physics::*;
use super::states::Ctx;
use super::Hero;
use crate::pad::{button, fast_arctan, fast_diff_rots};
use crate::ps2v::Pf;
use crate::rng::Rng;

/// The walk-to-point target the scripts set (0x140990 point, 0x14099c facing, 0x1409a0 release).
#[derive(Clone, Copy, Debug, Default)]
pub struct WalkTo {
    pub point: V4,
    pub yaw: Pf,
    pub release: i32,
}

/// The frame 0x65 starts the walk sequence on, by substate (gp−0x7528: 8, 0, 11, 16).
const WALK_FRAME: [i32; 4] = [8, 0, 11, 16];

impl Hero {
    /// `0x226f10(1)`: the stance sequence — the idle one (0x54 at health 1), a weapon's stance sequence with a
    /// weapon out (not reachable on foot here), 0 for the disguise.
    fn stance_seq(&self) -> u8 { if self.mode == 3 { 0 } else { self.idle_seq() } }

    /// `0x241d98`: the fidget moby 0x140964 deleted and the hidden back mobys shown again (neither is modelled:
    /// no fidget moby is ever created in the port).
    fn fidget_end(&mut self) {}
}

/// SetState entry of 1, 0x1e, 0x40, 0x41, 0x65..0x67. `None`: continue with SetState's epilogue.
pub(super) fn entry(h: &mut Hero, c: &mut Ctx, id: i32, play: bool, _old_sub: i32) -> Option<bool> {
    match id {
        1 => {
            h.f15d4 = 4;
            h.group = 0;
            // gp−0x7578 (0x15f688): the first-person look timer (crate::help::HeroHelp).
            h.help.look_timer = super::physics::ticks(0x3c);
            let s = h.stance_seq();
            if h.f13fa == 0 && c.anim.view().seq_b != s && play { h.set_anim(c.anim, c.rng, blend(18), s, 0); }
        }
        0x1e => {
            h.f15d4 = 0x51;
            h.group = 0;
            let s = h.stance_seq();
            if c.anim.view().seq_b != s { h.set_anim(c.anim, c.rng, blend(5), s, 0); }
        }
        0x41 => {
            let s = h.idle_seq();
            h.set_anim(c.anim, c.rng, blend(9), s, 0);
        }
        0x65 => {
            h.group = 1;
            h.f15d4 = 0;
            h.speed = len2(h.eff);
            h.items.f13fc = 1;
            if play && h.mode == 0 {
                let k = h.substate as usize & 3;
                h.set_anim(c.anim, c.rng, blend(8), h.substate as u8 + 3, WALK_FRAME[k]);
            }
        }
        0x66 => {
            h.group = 1;
            h.f15d4 = 0;
            h.speed = if h.gravity_mode == 0 { len2(h.eff) } else { Pf::ZERO };
            h.momentum = h.eff;
            if play {
                let v = c.anim.view();
                let seq = if v.seq_a == 4 && 3.0 < v.frame && v.frame < 16.0 { 5 } else { 6 };
                h.set_anim(c.anim, c.rng, blend(9), seq, 4);
            }
        }
        0x67 => {
            h.group = 0;
            h.f15d4 = 0;
            h.momentum = V0;
            h.idle.blink_period = 0x68;
            if play {
                if h.idle_seq() == 0x54 { h.set_anim(c.anim, c.rng, blend(18), 0x54, 0); } else { h.set_anim(c.anim, c.rng, Pf::b(0xbf80_0000), 0, 0); }
            }
        }
        // 0x40: no case (the group is left as it was).
        _ => {}
    }
    None
}

/// The look stances' turn under the first-person camera (the ground case of `0x2370b8` with state 1 / 0x1e and
/// 0x1413f5): the stick direction 0x13f4c0 = the camera's forward, the target yaw = the camera's 0x167258, and
/// `TurnTo(1, 0, 10000°/s)` (Ratchet faces the view at once). Then the look statistic: the timer `gp−0x7578` (0x15f688,
/// `ScaleTicks(60)` from SetState 1) runs out → move record 18 (0x1418d8) bumped, the timer −1 and the look count
/// `gp−0x7574` (0x15f68c) + 1 (the Novalis help director reads both).
pub(super) fn first_person_turn(h: &mut Hero, env: &Env) {
    h.stick_world = env.cam_rows[0];
    h.target_yaw = env.cam_yaw;
    h.turn_to(SCALE64, SCALE64 * Pf::ZERO, DT * Pf::b(0x432e_886e));
    if 0 < h.help.look_timer && crate::moby_update::creature::dec_timer_i32(&mut h.help.look_timer) != 0 {
        h.help.out.push(crate::help::HeroHelpOut::BumpMove(18));
        h.help.look_timer = -1;
        h.help.looks += 1;
    }
}

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(h: &mut Hero, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng) -> bool {
    match h.state {
        1 | 0x1e => h.phys_ground(env, anim, rng),
        0x40 | 0x67 => h.vel = V0,
        0x41 => {}
        0x65 => phys_walk_to(h, env),
        0x66 => {
            let d = vsub(h.walk_to.point, h.pos);
            if dot3(d, h.momentum) < Pf::ZERO { h.momentum = V0; }
            h.vel = V0;
            h.momentum_decay(DT2 * Pf::f(12.0));
            h.wall_check(env, 0);
            ground_gravity(h, env);
        }
        _ => return false,
    }
    true
}

/// The walk's gravity tail (0x23c2e4 / 0x23c3b4): in the air 25·dt² from the effective vz and the steep-wall
/// stop, on the ground 54·dt².
fn ground_gravity(h: &mut Hero, env: &Env) {
    if h.air_ticks != 0 {
        h.gravity_from(h.eff_v[2], DT2 * Pf::f(25.0));
        h.climb_check(env);
    } else {
        let z = h.vel[2];
        h.gravity_from(z, DT2 * Pf::f(54.0));
    }
}

/// 0x65 (0x23bd00): steer to the point (module doc).
fn phys_walk_to(h: &mut Hero, env: &Env) {
    let d = vsub(h.walk_to.point, h.pos);
    h.target_yaw = fast_arctan(d[0], d[1]);
    let l = len2(d);
    if Pf::f(0.2) < l {
        h.stick_mag = Pf::ONE;
        let row = if SPEED_TABLE[0][3] <= l { SPEED_TABLE[1][2] } else { SPEED_TABLE[0][2] };
        h.target_speed = row * DT;
        match h.mode {
            3 => h.target_speed = DT * Pf::f(1.5),
            1 => h.target_speed = DT * Pf::f(0.9),
            _ => {}
        }
    } else {
        h.stick_mag = Pf::b(0x3f00_0000);
        h.target_yaw = h.walk_to.yaw;
        h.target_speed = Pf::ZERO;
    }
    match h.mode {
        3 => h.turn_to(SCALE64 * Pf::f(0.03), SCALE64 * Pf::f(0.3), DT * Pf::f(5.585_053_4)),
        1 => h.turn_to(SCALE64 * Pf::f(0.03), SCALE64 * Pf::f(0.3), DT * Pf::f(7.853_981_5)),
        0 if h.substate == 1 => h.turn_to(SCALE64 * Pf::f(0.016), SCALE64 * Pf::f(0.15), DT * Pf::f(9.948_377)),
        0 => h.run_turn(),
        _ => {}
    }
    h.speed_step((DT2 * Pf::f(7.5)) * Pf::ONE, DT2 * Pf::f(8.5));
    h.set_planar_vel(Pf::b(0x47c3_4f80));
    if h.mode == 0 { h.wall_check(env, 1); }
    ground_gravity(h, env);
}

/// Per-state transitions.
pub(super) fn transitions(h: &mut Hero, c: &mut Ctx) {
    let held = c.env.pad.held;
    let wrapped = c.anim.view().flags & 2 != 0;
    match h.state {
        1 | 0x1e => {
            if h.mode == 0 && h.f13fa == 0 {
                let s = if held & button::CROUCH == 0 { h.stance_seq() } else { 0xd };
                if c.anim.view().seq_b != s { h.set_anim(c.anim, c.rng, blend(8), s, 0); }
            }
            if h.state == 0x1e {
                if held & button::STRAFE != 0 { return; }
                release_to_idle(h, c);
                return;
            }
            // The wrench (class 0x47) mid-swing in the hand keeps the stance.
            let busy = h.items.slot.item.as_ref().is_some_and(|m| m.o_class == 0x47 && m.mstate != 0);
            if held & button::STRAFE != 0 || busy { return; }
            // In a body (0x1413f4 ≠ 0): the per-body idle `0x227638` (Clank 0x43, Giant Clank 0x5a, the disguise 0x53).
            if h.mode != 0 {
                super::bodies::body_idle(h, c);
                return;
            }
            if held & button::CROUCH != 0 {
                h.set_state(c, 4, true);
                return;
            }
            release_to_idle(h, c);
        }
        0x40 => {
            let to = if held & button::STRAFE != 0 {
                1
            } else if h.air_ticks != 0 || held & button::CROSS != 0 || held & button::CROUCH != 0 {
                0x41
            } else {
                h.stick_target_table(c.env);
                if Pf::ZERO < h.target_speed {
                    0x41
                } else {
                    if !wrapped { return; }
                    h.fidget_end();
                    0
                }
            };
            h.set_state(c, to, true);
        }
        0x41 => {
            if h.timer <= ticks(8) { return; }
            h.fidget_end();
            let to = if h.air_ticks != 0 {
                6
            } else {
                if h.try_jump(c, ticks(7), false) { return; }
                if held & button::CROUCH != 0 {
                    4
                } else {
                    h.stick_target_table(c.env);
                    if Pf::ZERO < h.target_speed { 2 } else { 0 }
                }
            };
            h.set_state(c, to, true);
        }
        0x65 => {
            if !(dist2(h.pos, h.walk_to.point) < Pf::f(0.2)) {
                h.walk_run_anim(c);
                return;
            }
            if h.eff_len < DT * Pf::f(2.7) && fast_diff_rots(h.rot[2], h.walk_to.yaw) < Pf::f(0.034_906_585) {
                h.set_state(c, 0x67, true);
                return;
            }
            if h.mode != 0 && !(h.eff_len < DT * Pf::f(2.7)) { h.set_state(c, 0x66, true); }
        }
        0x66 => {
            if wrapped { h.set_state(c, 0x65, true); }
        }
        0x67 => {
            if wrapped || h.walk_to.release != 0 { h.set_state(c, 0, true); }
            if Pf::f(0.2) < dist2(h.pos, h.walk_to.point) { h.set_state(c, 0x65, true); }
        }
        _ => {}
    }
    // The transitions' tail (0x242930): leaving 0x40 (to anything but 0x41) ends the fidget.
    if h.prev_state == 0x40 && h.state != 0x41 && h.state != 0x40 { h.fidget_end(); }
}

/// The stance's release: `SetState(0, 0)` and the stance sequence over 8 ticks unless it plays already.
fn release_to_idle(h: &mut Hero, c: &mut Ctx) {
    h.set_state(c, 0, false);
    let s = h.stance_seq();
    if c.anim.view().seq_b != s { h.set_anim(c.anim, c.rng, blend(8), s, 0); }
}

#[cfg(test)]
mod tests {
    use crate::hero::bodies::{body, clank, giant};
    use crate::hero::testkit::{floor, Runner};
    use crate::pad::{button, PadInput};

    /// L1 in a body's idle takes the look stance 1; released, the per-body idle `0x227638` (the old port left a body
    /// in 1 for good: Clank stuck in first person).
    #[test]
    fn a_body_leaves_the_look_stance_for_its_idle() {
        for (mode, idle) in [(body::CLANK, clank::IDLE), (body::GIANT, giant::IDLE)] {
            let coll = floor(100.0, 100, 106, 100, 106);
            let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
            r.hero.mode = mode;
            r.hero.state = idle;
            r.run(&coll, PadInput::neutral(), 3);
            assert_eq!(r.hero.state, idle, "body {mode}: idle");
            r.run(&coll, PadInput::neutral().press(button::L1), 5);
            assert_eq!(r.hero.state, 1, "body {mode}: the look stance");
            r.run(&coll, PadInput::neutral(), 3);
            assert_eq!(r.hero.state, idle, "body {mode}: back to its idle");
        }
    }
}
