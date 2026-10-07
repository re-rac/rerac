//! Hinged bridge halves, class 746: `HingedBridgeUpdate` level01 0x2fb8a8 (read from the disassembly). The "door"
//! of the Novalis mission cutaway: the mission NPC 790 commands both halves (+0xbc = 2) while its camera trigger 863
//! shows them, and a finished mission lowers them on any later load. Spec: docs/plan/cutscenes.md §4. Native `f32`.
//!
//! Pvar block (P): +0x60 the hinge (the instance position, w kept), +0x70 f32 raise 1 → 0, +0x74 the loop-sound voice.
//! **0** init: hinge = position, raise 1, voice −1 → **1** up: +0xbc = 2 or the mission byte done → +0xbc = 0,
//! **2** lowering: raise −= 0.5·dt (clamped at 0; 2 s), loop sound 0 (flags 4) kept alive; at 0 the voice is
//! released → 1. Every tick: pitch (+0x44) = raise·π/5 (36°) and position = hinge + (0, 0, −11) +
//! 11·(cos yaw·sin pitch, sin yaw·sin pitch, cos pitch): the half turns about its hinge 11 units below its origin.

use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pvar as p, World};

/// The update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x2fb8a8;
/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [746];

/// 0x3f20d97c: the raised pitch (π/5).
pub const RAISED_PITCH: f32 = f32::from_bits(0x3f20_d97c);
/// The arm from the hinge to the origin.
pub const ARM: f32 = 11.0;

/// `HingedBridgeUpdate` 0x2fb8a8 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x78 { return; }
    let level = w.svc.level;
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            let pos = m.position;
            p::set_v4f(&mut m.pvars, 0x60, pos);
            p::set_ff(&mut m.pvars, 0x70, 1.0);
            p::set_i32(&mut m.pvars, 0x74, -1);
            m.state = 1;
        }
        1 => {
            if w.m(id).cmd == 2 || w.mission_done(level, w.m(id).mission) == 0xff {
                let m = w.mm(id);
                m.state = 2;
                m.cmd = 0;
            }
        }
        2 => {
            let dt = crate::moby_update::creature::DT;
            let pv = &mut w.mm(id).pvars;
            let r = (p::ff(pv, 0x70) - dt * 0.5).max(0.0);
            p::set_ff(pv, 0x70, r);
            let voice = p::i32(pv, 0x74);
            if r == 0.0 {
                if voice != -1 { w.release_sound(voice, id); }
                p::set_i32(&mut w.mm(id).pvars, 0x74, -1);
                w.mm(id).state = 1;
            } else if !w.sound_alive(voice, id) {
                let v = w.play_sound(0, 4, id);
                p::set_i32(&mut w.mm(id).pvars, 0x74, v);
            }
        }
        _ => {}
    }
    let m = w.mm(id);
    let pitch = p::ff(&m.pvars, 0x70) * RAISED_PITCH;
    m.rotation[1] = pitch;
    let hinge = p::v4f(&m.pvars, 0x60);
    let yaw = m.rotation[2];
    let (sp, cp) = pitch.sin_cos();
    let (sy, cy) = yaw.sin_cos();
    m.position[0] = hinge[0] + cy * ARM * sp;
    m.position[1] = hinge[1] + sy * ARM * sp;
    m.position[2] = (hinge[2] - ARM) + cp * ARM;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raised_pitch_is_36_degrees() { assert!((RAISED_PITCH - std::f32::consts::PI / 5.0).abs() < 1e-6); }
}
