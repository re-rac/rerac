//! Camera triggers, class 737: `CameraTriggerUpdate` level01 0x2fb5b0 (the same code on levels 1, 2, 3, 7, 10, 13:
//! clusters.tsv 5c26590d). The generic **cutaway**: a cuboid (or another moby's command) puts Ratchet on hold, cuts
//! to a fixed camera for a while and returns. Spec: docs/plan/cutscenes.md §3.2. Native `f32`.
//!
//! Pvar block (P, s32 words): P[0..3] the camera's target position and P[4..7] its target Euler (the camera cuboid's,
//! copied at the start; another class may overwrite them every tick: the mission NPC aims P[5] / P[6] at itself),
//! P[8] the trigger cuboid (−1: fired only by command), P[9] the camera cuboid (centre + Euler; −1: the moby deletes
//! itself), P[10] the hero cuboid (−1: Ratchet stays where he is), P[11] the hold time (ticks; `ticks()` applied at
//! init), P[12] 0 = cut to the camera cuboid, else move there from the current view over P[13] ticks (mode 2).
//! `moby+0xbc` = 1: fire / keep holding (a commanding class sets it with P[11]; the trigger sets it when it fires).
//!
//! States: **0** init (P[11] scaled, update and draw distance 0xff) → **1** armed: unless the spawn id is collected or
//! its level death bit set, Ratchet's feet in P[8] or +0xbc = 1 fire it: `HeroTeleport(P[10] or himself, 0x72, reset
//! camera)`, `CameraScript(P[9], 0, 0, 0)` (or `CameraScript(current view, 2, P[13], 0)`), `0x15f404 = 1`, +0xbc = 1,
//! P[0..7] = P[9]'s centre and Euler → **2** holding: each tick `FastDecTimer(P[11])`; while it runs and +0xbc = 1
//! the camera's targets are set from P[0..7]; when it runs out (or +0xbc is cleared) `CameraScript2(0)` (mode 2:
//! `CameraScript2(2)`), Ratchet `SetState(0, 1)` (`FUN_002405a0`; 0x53 in his mode 3), `0x15f404 = 0`, +0xbc = 0 →
//! **3** spent: only a new command (+0xbc = 1) re-arms it. No skip.
//!
//! Novalis has five: 860 (trigger cuboid 42, camera 43, Ratchet to 42; also commanded by the mission NPC 790 for 7 s
//! while it drops in), 863 (camera 51, commanded by 790 for 3 s when the hinged bridge 746 lowers), 859 / 861 / 862
//! (no cuboid and no commander on the disc: never fire). `FUN_002fb4b8` at the top: the big-head cheat 0x15edb0 on
//! the scene actors 920 / 447 at 2.1 (`manip::scene_big_head`).

use crate::cinematic;
use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pvar as p, World};

/// The update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x2fb5b0;
/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [737];

/// Hero state of the hold (`HeroTeleport(…, 0x72, 1)`).
pub const HOLD: i32 = 0x72;

fn cuboid(w: &World, i: i32) -> Option<([f32; 3], [f32; 3])> {
    let s = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, i)?;
    Some((s.centre(), s.euler))
}

/// `CameraTriggerUpdate` 0x2fb5b0 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x38 { return; }
    // FUN_002fb4b8: the actors' big-head cheat on the scene actors of class 0x398 / 0x1bf at 2.1.
    crate::moby_update::manip::scene_big_head(w, &[0x398, 0x1bf], 0, f32::from_bits(0x4006_6666));
    let pi = |w: &World, o: usize| p::i32(&w.m(id).pvars, o);
    match w.m(id).state {
        0 => {
            if pi(w, 0x24) == -1 {
                w.delete_moby(id);
                return;
            }
            let t = w.ticks(pi(w, 0x2c));
            let m = w.mm(id);
            p::set_i32(&mut m.pvars, 0x2c, t);
            m.state = 1;
            m.update_dist = 0xff;
            m.draw_dist = 0xff;
        }
        1 => {
            let (sid, level) = (w.m(id).spawn_id, w.svc.level);
            let spent = w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(level, sid));
            if spent { return; }
            let feet = crate::hero::physics::to_f32x3(w.hero.pos);
            if !(w.in_cuboid(feet, pi(w, 0x20)) || w.m(id).cmd == 1) { return; }
            fire(w, id);
        }
        2 => {
            let pv = &mut w.mm(id).pvars;
            let mut t = p::i32(pv, 0x2c);
            // FastDecTimer: 1 when it was already 0, 2 when it just ran out, 0 while it runs.
            let r = if t == 0 {
                1
            } else {
                t = t.max(1) - 1;
                if t < 1 { 2 } else { 0 }
            };
            p::set_i32(pv, 0x2c, t);
            if r == 0 && w.m(id).cmd == 1 {
                let pv = &w.m(id).pvars;
                let pos = [p::ff(pv, 0), p::ff(pv, 4), p::ff(pv, 8)];
                let euler = [p::ff(pv, 0x10), p::ff(pv, 0x14), p::ff(pv, 0x18)];
                cinematic::camera_targets(w, Some(pos), Some(euler));
                return;
            }
            let kind = if pi(w, 0x30) == 0 { 0 } else { 2 };
            cinematic::camera_script2(w, kind);
            // FUN_002405a0: back to idle (0x53 when he is in mode 3).
            match w.body() {
                0 => cinematic::hero_state(w, 0, true),
                3 => cinematic::hero_state(w, 0x53, true),
                _ => {}
            }
            cinematic::letterbox(w, false);
            let m = w.mm(id);
            m.state = 3;
            m.cmd = 0;
        }
        3 if w.m(id).cmd == 1 => w.mm(id).state = 1,
        _ => {}
    }
}

/// The firing branch of state 1.
fn fire(w: &mut World, id: MobyId) {
    let pi = |w: &World, o: usize| p::i32(&w.m(id).pvars, o);
    // The view before the switch (0x167240 / 0x167250), for the mode-2 move.
    let cam_pos = crate::hero::physics::to_f32x3(w.camera);
    let cam_euler = w.hero.loop_in.cam_euler;
    let hero_to = cuboid(w, pi(w, 0x28)).unwrap_or_else(|| {
        let h = w.hero;
        (crate::hero::physics::to_f32x3(h.pos), [h.rot[0].to_f32(), h.rot[1].to_f32(), h.rot[2].to_f32()])
    });
    cinematic::hero_teleport(w, hero_to.0, hero_to.1, HOLD, true);
    let Some((cc, ce)) = cuboid(w, pi(w, 0x24)) else { return };
    if pi(w, 0x30) == 0 {
        cinematic::camera_script(w, cc, ce, 0, 0, false);
    } else {
        let t = pi(w, 0x34);
        cinematic::camera_script(w, cam_pos, cam_euler, 2, t, false);
    }
    cinematic::letterbox(w, true);
    let m = w.mm(id);
    m.state = 2;
    m.cmd = 1;
    p::set_v4f(&mut m.pvars, 0, [cc[0], cc[1], cc[2], 0.0]);
    p::set_v4f(&mut m.pvars, 0x10, [ce[0], ce[1], ce[2], 0.0]);
}
