//! U113 (census 2026-10-02): class 792, Aridia's platform carried by a big animated arm (level02 0x2e2228, the only copy:
//! 1 placed; the arm is the moby its pvar +0xa0 names). Read from the level02 decomp. Native `f32`.
//!
//! The platform sits on the arm's joint 0 every tick (and faces along the joint) and carries its riders. Once its
//! mission is done, the arm swings it between its two ends: it starts when Ratchet steps on, or when he is within 16
//! and 5 below (at the low end: above) it; it pauses while he stands right under it. The arm plays its hum while it
//! moves and a sound at each end.
//!
//! **Pvars**: +0x60 the platform block (`CarryRiders`), +0xa0 the arm (moby index after the loader's link fixups), +0xa4
//! the hum's voice. `cmd` (+0xbc): 1 after an end until Ratchet steps off.
//!
//! | address | what | port |
//! |---|---|---|
//! | every tick | old = position, rotation; position = the arm's joint 0 (0x251e38 = L01 0x2645a8) | [`update`] |
//! | 0 | yaw = atan(row 0) of the joint's matrix (`moby_attach_to_joint` 0x20cca8); 1; the arm blends to 3 | [`update`] (`World::joint_matrix`) |
//! | 1 | the mission done → 5 | [`update`] |
//! | 2, 4 | yaw from the joint; Ratchet within 1.5 and 1.5 under it → the arm stops (speed 0), the hum released; a stopped arm: Ratchet beyond 2 or not under it (z > pos − 1.25) → speed 1, the hum (class sound 2 in 2, 0 in 4, flags 4) | [`update`] |
//! | | the arm's clip wrapped: the hum released; 2 → 3, the arm blends to 1, class sound 3; 4 → 5, to 3, sound 1; `cmd` 1 | [`update`] |
//! | 3, 5 | go = Ratchet on it (ground moby 0x13f64c, air ticks 0) with `cmd` 0, or within 16 and (5: 5 below it; 3: 5 above it) | [`update`] |
//! | | go: 5 (or 1) → 2, the arm cut to 0, the hum 2; else → 4, cut to 2, hum 0; not: Ratchet's ground moby not it → `cmd` 0 | [`update`] |
//! | tail | `CarryRiders(+0x60, position − old, old rotation, rotation)` | [`update`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature as c;
use crate::moby_update::services::World;

pub const REFERENCE_LEVEL: u32 = 2;
pub const UPDATE_FN: u32 = 0x2e_2228;
pub const CLASSES: [i16; 1] = [792];

const BLOCK: usize = 0x60;
const ARM: usize = 0xa0;
const VOICE: usize = 0xa4;
const SIZE: usize = 0xa8;

fn arm(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, ARM)).ok().filter(|&m| m < w.table.mobys.len()) }

fn release(w: &mut World, id: MobyId, arm: MobyId) {
    let v = c::pi32(w, id, VOICE);
    if v != -1 { w.release_sound(v, arm); }
    c::set_pi32(w, id, VOICE, -1);
}

fn hum(w: &mut World, id: MobyId, arm: MobyId, index: i32) {
    let s = w.play_sound(index, 4, arm);
    c::set_pi32(w, id, VOICE, s);
}

fn face_joint(w: &mut World, id: MobyId, arm: MobyId) {
    let m = w.joint_matrix(arm, 0);
    w.mm(id).rotation[2] = c::atan(m[0][0], m[0][1]);
}

/// Level02 0x2e2228 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < SIZE { return; }
    let Some(a) = arm(w, id) else { return };
    let (old, old_rot) = (w.m(id).position, w.m(id).rotation);
    let j = w.joint_point(a, 0);
    w.mm(id).position = j;
    let hero = crate::moby_update::classes::units::hero_pos(w);
    match w.m(id).state {
        0 => {
            face_joint(w, id, a);
            w.mm(id).state = 1;
            w.anim_blend(a, 3, 0, 0);
        }
        1 => {
            let level = w.svc.level;
            if w.mission_done(level, w.m(id).mission) == 0xff { w.mm(id).state = 5; }
        }
        st @ (2 | 4) => {
            face_joint(w, id, a);
            let pos = w.m(id).position;
            if c::dist3(pos, hero) < 1.5 && hero[2] < pos[2] - 1.5 {
                w.mm(a).anim.speed = 0.0;
                release(w, id, a);
            }
            if w.m(a).anim.speed == 0.0 && (2.0 < c::dist3(pos, hero) || pos[2] - 1.25 < hero[2]) {
                w.mm(a).anim.speed = 1.0;
                hum(w, id, a, if st == 2 { 2 } else { 0 });
            }
            if w.m(a).anim.flags & 2 != 0 {
                release(w, id, a);
                if st == 2 {
                    w.mm(id).state = 3;
                    w.anim_blend(a, 1, 0, 0);
                    w.play_sound(3, 0, a);
                } else {
                    w.mm(id).state = 5;
                    w.anim_blend(a, 3, 0, 0);
                    w.play_sound(1, 0, a);
                }
                w.mm(id).cmd = 1;
            }
        }
        st @ (3 | 5) => {
            let pos = w.m(id).position;
            let on = w.hero.ground_moby == Some(id) && w.hero.air_ticks == 0;
            let go = (on && w.m(id).cmd == 0)
                || (c::dist3(pos, hero) < 16.0 && ((st == 5 && hero[2] < pos[2] - 5.0) || (st == 3 && pos[2] + 5.0 < hero[2])));
            if go {
                if st == 5 {
                    w.mm(id).state = 2;
                    c::hard_cut(w, a, 0, 0);
                    hum(w, id, a, 2);
                } else {
                    w.mm(id).state = 4;
                    c::hard_cut(w, a, 2, 0);
                    hum(w, id, a, 0);
                }
            } else if w.hero.ground_moby != Some(id) {
                w.mm(id).cmd = 0;
            }
        }
        _ => {}
    }
    let pos = w.m(id).position;
    let rot = w.m(id).rotation;
    let delta = [pos[0] - old[0], pos[1] - old[1], pos[2] - old[2], pos[3] - old[3]];
    crate::moby_update::triggers::carry_riders(&mut w.mm(id).pvars, BLOCK, delta, old_rot, rot);
}
