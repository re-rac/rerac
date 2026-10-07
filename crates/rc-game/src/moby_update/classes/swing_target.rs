//! Swingshot targets, classes **758** (0x2f6, pull targets) and **803** (0x323, swing targets): one update function
//! for both, identical in every overlay that has them (clusters.tsv 30323bfd…: level03 0x2d0bd8 and 12 more; the
//! swing-only levels 6 and 9 compile the same code at another size, 0x2e6270 / 0x2f8c70). No level01 instance:
//! the registry keys it by the level03 address. Spec: docs/plan/hero_states.md "P6" (the pvar record there and in
//! [`crate::hero::swingshot::Record`]). Native `f32`.
//!
//! **Update** (the state byte +0x20):
//! * **0** (placed): with no mission gate (record +0x30 = −1) and no gating moby (+0x3c = −1, or that moby deleted:
//!   state 0xfe / 0xfd) the target lights up (ambient 0xc0, 0xc0, 0xc0) and becomes state 1; otherwise it waits in
//!   state 2, hidden and frozen (mode |= 0x41) and always updated (+0x30 = 0xff).
//! * **2**: the mission done (`0x14c050[level·16 + m] ≠ 0`) or the gating moby gone → shown again (mode &= ~0x41),
//!   the gate cleared, state 0.
//! * **1** (active): a swing target (803) turns to face the hero (`0x24a848`: accel / decel 2π·dt², at most
//!   4π·dt, velocity in +0x38) and, every other tick while it was drawn last frame, spawns a glint (type 60:
//!   1.0 along its row 1, alternating side every 4 ticks, drifting back across over 30 ticks, size 1.25, colour
//!   0x60408080, a `randi(0xff)` rotation byte). While the hero is not swinging (0x2c) it offers itself to the
//!   follow camera's look-up hint `0x2eb4c0` (`crate::follow_camera::swing::LookHint`; state 0 → 1 resets the hint,
//!   `0x2eb3d0`).
//!
//! The glint is a type-60 particle (`crate::particles::type60`, through `World::part60`).

use crate::moby_runtime::{mode, state, MobyId};
use crate::moby_update::services::{pvar as p, World};

/// The update address (level03's class table; the port's registry key).
pub const UPDATE_FN: u32 = 0x2d0bd8;

/// Classes that run [`update`].
pub const CLASSES: [i16; 2] = [758, 803];

/// The overlays that compile the function (level, address).
pub const LEVEL_FNS: [(u32, u32); 16] = [
    (2, 0x2e0450), (3, 0x2d0bd8), (4, 0x2d9a58), (5, 0x3098d8), (6, 0x2e6270), (7, 0x302238), (9, 0x2f8c70), (10, 0x2c9590),
    (11, 0x300010), (12, 0x2f7b28), (13, 0x2fa708), (14, 0x2f2cb8), (15, 0x2db100), (16, 0x2d6378), (17, 0x2d95a8), (18, 0x2e19d8),
];

const DT: f32 = 1.0 / 60.0;
const DT2: f32 = 1.0 / 3600.0;
const PI: f32 = std::f32::consts::PI;
const TAU: f32 = std::f32::consts::TAU;

fn wrap(a: f32) -> f32 {
    let mut x = a;
    while x >= PI { x -= TAU; }
    while x < -PI { x += TAU; }
    x
}

/// `0x270ac0(target, step, &x)`: x approaches the target by at most `step` (as an angle).
fn approach_rot(target: f32, step: f32, x: &mut f32) {
    let d = wrap(target - *x).clamp(-step, step);
    *x = wrap(*x + d);
}

/// `0x270cc0(target, accel, decel, max, &angle, &vel)` (level03 `0x24a848`): turn with a bounded angular
/// acceleration, braking so as to stop on the target.
pub fn turn_accel(target: f32, accel: f32, decel: f32, max: f32, angle: &mut f32, vel: &mut f32) {
    let diff = wrap(target - *angle);
    if *vel * diff < 0.0 || diff == 0.0 {
        approach_rot(0.0, decel, vel);
    } else {
        let stop = (*vel * *vel / decel) * 0.5;
        if diff.abs() < stop {
            let step = if stop < diff.abs() + vel.abs() { decel } else { decel * 1.1 };
            *vel += (0.0 - *vel).clamp(-step, step);
        } else {
            let v = ((decel + decel) * diff).abs().sqrt().min(max);
            approach_rot(if diff < 0.0 { -v } else { v }, accel, vel);
        }
        if diff.abs() <= vel.abs() {
            *angle = target;
            return;
        }
    }
    *angle = wrap(*vel + *angle);
}

/// The gating moby (+0x3c) is gone (or none).
fn link_gone(w: &World, link: i32) -> bool {
    if link == -1 { return true; }
    match usize::try_from(link).ok().and_then(|i| w.table.mobys.get(i)) {
        None => true,
        Some(m) => m.state == state::DELETED || m.state == state::DELETED_STATIC,
    }
}

pub fn update(w: &mut World, id: MobyId) {
    let st = w.m(id).state;
    match st {
        0 => {
            let pv = &w.m(id).pvars;
            let (gate, link) = (p::i32(pv, 0x30), p::i32(pv, 0x3c));
            if gate == -1 && link_gone(w, link) {
                let m = w.mm(id);
                m.ambient[0] = 0xc0;
                m.ambient[1] = 0xc0;
                m.ambient[2] = 0xc0;
                // FUN_002eb3d0: the camera hint's reset.
                crate::cinematic::look_hint(w, crate::follow_camera::swing::HintCall::Reset);
                w.mm(id).state = 1;
                return;
            }
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.state = 2;
            m.mode |= mode::HIDDEN | mode::NO_ANIM;
        }
        1 => {
            if w.m(id).o_class == 803 {
                let hero = crate::hero::physics::to_f32x3(w.hero.pos);
                let m = w.m(id);
                let target = (hero[1] - m.position[1]).atan2(hero[0] - m.position[0]);
                let (mut a, mut v) = (m.rotation[2], p::ff(&m.pvars, 0x38));
                turn_accel(target, DT2 * TAU, DT2 * TAU, DT * 2.0 * TAU, &mut a, &mut v);
                let m = w.mm(id);
                m.rotation[2] = a;
                p::set_ff(&mut m.pvars, 0x38, v);
                if w.counter & 1 != 0 && w.m(id).visible != 0 {
                    // PartType60Spawn(1.25, pos + setlen(row 1, ±1), setlen(that, −2/30), 0x60408080, 30, randi(0xff),
                    // 0): the side flips every 4 ticks and the glint drifts back across the target over its life.
                    let m = w.m(id);
                    let side = if w.counter & 4 != 0 { -1.0 } else { 1.0 };
                    let r1 = [m.rows[1][0], m.rows[1][1], m.rows[1][2]];
                    let l = (r1[0] * r1[0] + r1[1] * r1[1] + r1[2] * r1[2]).sqrt();
                    let off = if l == 0.0 { [0.0; 3] } else { r1.map(|x| x * (side / l)) };
                    let life = w.ticks(30);
                    let lo = (off[0] * off[0] + off[1] * off[1] + off[2] * off[2]).sqrt();
                    let k = if lo == 0.0 { 0.0 } else { (-2.0 / life as f32) / lo };
                    let vel = [off[0] * k, off[1] * k, off[2] * k, 0.0];
                    let pos = [m.position[0] + off[0], m.position[1] + off[1], m.position[2] + off[2], 0.0];
                    let rot = w.rng.randi(0xff) as u8;
                    w.part60(1.25, pos, vel, 0x6040_8080, life as u16, rot, 0);
                }
            }
            // FUN_002eb4c0: the camera's look-up hint while the hero is not swinging.
            if w.hero.state != 0x2c {
                let pos = { let p = w.m(id).position; [p[0], p[1], p[2]] };
                let hero = crate::hero::physics::to_f32x3(w.hero.pos);
                crate::cinematic::look_hint(w, crate::follow_camera::swing::HintCall::Offer { target: id, pos, hero });
            }
        }
        2 => {
            let (gate, link) = { let pv = &w.m(id).pvars; (p::i32(pv, 0x30), p::i32(pv, 0x3c)) };
            let done = gate >= 0 && w.mission_done(w.svc.level, gate as u8) != 0;
            if !done {
                if link == -1 { return; }
                if !link_gone(w, link) { return; }
            }
            let m = w.mm(id);
            m.mode &= !(mode::HIDDEN | mode::NO_ANIM);
            p::set_i32(&mut m.pvars, 0x30, -1);
            m.state = 0;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_accel_reaches_the_target_and_stops() {
        let (mut a, mut v) = (0.0f32, 0.0f32);
        let mut n = 0;
        while a != 1.0 && n < 600 {
            turn_accel(1.0, DT2 * TAU, DT2 * TAU, DT * 2.0 * TAU, &mut a, &mut v);
            assert!(v.abs() <= DT * 2.0 * TAU + 1e-6, "speed capped");
            n += 1;
        }
        assert_eq!(a, 1.0, "lands exactly on the target");
        assert!(n > 20 && n < 200, "accelerates and brakes: {n} ticks");
    }

    #[test]
    fn registry_maps_both_target_classes() {
        // Reversed on level 03 and not on level 01: the class-number (level-01) registry has no port for them; level 03's
        // table runs this one (`LevelPorts`, tests/hero/hero_swingshot_levels.rs).
        for oc in CLASSES { assert_eq!(crate::moby_update::scheduler::port_update_fn(oc), None, "class {oc}"); }
        assert!(crate::moby_update::classes::ClassUpdate::SwingTarget.classes() == CLASSES && crate::moby_update::classes::ClassUpdate::SwingTarget.reference_level() == 3);
    }
}
