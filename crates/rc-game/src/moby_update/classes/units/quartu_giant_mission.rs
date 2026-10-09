//! **Quartu's Giant Clank mission NPC** (class 1446, level15 `0x2ec760`, census U499; one instance): the talker who
//! sends Clank into Giant Clank. Talk to him (the shared talk system, inside his cuboid) and his mission is set; once
//! Giant Clank has cleared the two enemy groups, two seconds later Clank climbs out (the level's leave copy
//! `0x208ca8`: `crate::hero::bodies::queue_leave`), Ratchet is put on a spot, planet 16 (Kalebo III) is unlocked, the
//! scenes and the movie play and the game is saved. Between the scenes his head follows Ratchet (the NPC look-at shared
//! with the talkers 774: `talking_npc::look_at_layout`). Read from the level15 decomp and disassembly.
//!
//! **Pvar block**: +0x00..+0x40 the talk block (`interact::talk`), +0x140 (s32) 1 after the finale, +0x144 / +0x148
//! the two enemy groups, +0x14c Giant Clank's body moby (class 0x1a3), +0x150 the checkpoint cuboid, +0x154 the
//! teleport / checkpoint cuboid of the finale, +0x158 a moby deleted once Giant Clank is in, +0x15c the finale timer,
//! +0x160 / +0x1e0 the look records (lists 1 / 0), +0x260 the glance point, +0x274 / +0x278 the seen / glance timers,
//! +0x27c the talk cuboid, +0x280 the talk mission, +0x284 the finale mission.
//!
//! **System or not.** Per-class code on the shared systems: the talk system (`interact`), the bodies
//! (`hero::bodies::queue_leave`), the cinematic calls (`cinematic`), the checkpoint record (`checkpoint::record`), the
//! group counts (`scheduler::group_count`), the NPC look-at (`talking_npc::look_at_layout`, now a layout row).
//!
//! **Coverage** (level15 addresses):
//!
//! | address | what | status |
//! |---|---|---|
//! | 0x2ec760 +0 | `0x2ec678`: in a cutscene with the big-head cheat 0x15edb0, the cheat's manipulator (list 1, 2.75) on this class's scene actors | ported (`manip::scene_big_head`) |
//! | +1 | +0x14c = −1 → return | ported ([`update`]) |
//! | +2 | body ≠ 2: Giant Clank's moby: collision off (+0x94 = 0), hidden (+0x34 \|= 0x41) | ported |
//! | +3 | state 9: +0x30 = 0, collision off, hidden; return | ported |
//! | +4 | game mode 2: collision off, hidden; else collision = the class's, shown (+0x34 &= ~0x41), the shadow slab z ± 0.2 (`0x2499d8`) | ported |
//! | state 0 | +0x30 = 0xff; → 1; talk radius 20; `NpcTalkRegister` (`0x255e88`); the talk mission (+0x280) done: planet 16 not yet unlocked (0x13dd50 = 0) → 3 and every other live moby of the run list with mission byte (+0xb0) = +0x280 deleted; unlocked → 9 | ported |
//! | state 1 | Ratchet (0x13f3d0) in cuboid +0x27c (`0x24f208`), his group (0x1413dc) < 2 or 9, `NpcTalkUpdate` (`0x255a30`) started: `SetMissionDone(+0x280)`, anim speed 0 (+0x58), → 2, the mission's mobys deleted | ported (`interact::talk_update`) |
//! | state 2 | game mode ≠ 2: anim speed 1; the node that played last (talk +0x04) ≠ 0 → 9; else the checkpoint of cuboid +0x150 (`0x274af8`), → 3 | ported (the scene still pending in the port counts as mode 2 [L]) |
//! | state 3 | body 2: the moby +0x158 (not 0xfe / 0xfd) deleted, +0x158 = −1; groups +0x144 and +0x148 empty (`MobyGroupCount(g, 0x40)` = 0) → 4, timer `ticks(120)` | ported (`scheduler::group_count`) |
//! | state 4 | timer out: leave the body (`0x208ca8`); Giant Clank's anim cut to 0 (`fun_00212ed8`); talk node 1, auto 1; +0x140 = 1; `UnlockPlanet(16)`; `SetMissionDone(+0x284)`; `MusicRequestTrack(0, 5)`; cuboid +0x154: `HeroTeleport(centre, Euler, 0, 1)` (`0x20e370`) and the checkpoint there; → 5 | ported (`bodies::queue_leave`, `cinematic::hero_teleport`) |
//! | state 5 | `DialogStreamStart(1)` (`0x286788`) → 6 | ported (`cinematic::start_scene`) |
//! | state 6 | game mode ≠ 2: `DialogStreamUpdate(16)` (the movie, `0x2873a8`) → 7 | ported (`cinematic::start_movie`) |
//! | state 7 | game mode ≠ 2: `ShowPlanetBanner(16)`, `DialogStreamStart(2)`, → 8, then state 8's save at once; game mode 2: the save and → 9 at once (no banner, no scene) | ported |
//! | state 8 | `memcard_Save(0, −1)` → 9 | ported (`cinematic::save`) |
//! | tail | the head look-at (layout: pitch record +0x160 list 1, yaw record +0x1e0 list 0, glance +0x260, timers +0x274 / +0x278; no sequence gate) | ported (`talking_npc::look_at_layout`) |
//! | tail | the big-head cheat: +0x1d0 = 2.75 | ported (`talking_npc::look_springs`) |
//! | | no particle, light, sound of its own (the scenes carry theirs) | n/a |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::talking_npc::{look_at_layout, LookLayout};
use crate::moby_update::interact::{self, talk};
use crate::moby_update::services::{pvar as p, World};

/// The update in the level15 class table.
pub const UPDATE_FN: u32 = 0x2e_c760;
pub const REFERENCE_LEVEL: u32 = 15;
pub const CLASSES: [i16; 1] = [1446];
/// Kalebo III.
const PLANET: i32 = 16;
/// The finale's movie (`DialogStreamUpdate(0x10)`).
const MOVIE: i32 = 16;

pub mod pv {
    pub const DONE: usize = 0x140;
    pub const GROUP_A: usize = 0x144;
    pub const GROUP_B: usize = 0x148;
    pub const GIANT: usize = 0x14c;
    pub const CHECKPOINT: usize = 0x150;
    pub const FINALE: usize = 0x154;
    pub const GONE: usize = 0x158;
    pub const TIMER: usize = 0x15c;
    pub const TALK_CUBOID: usize = 0x27c;
    pub const TALK_MISSION: usize = 0x280;
    pub const FINALE_MISSION: usize = 0x284;
    pub const SIZE: usize = 0x288;
}

/// The head look-at layout of this class.
const LOOK: LookLayout = LookLayout { pitch: (0x160, 1), yaw: (0x1e0, 0), glance: 0x260, seen: 0x274, glance_timer: 0x278, gate_seq_b: false, eye: 1.0, pitch_k: 1.25, yaw_a: 0.5, yaw_b: 0.5, short_timers: false, gate_main: 0, gate_alt: 0xff, k_seen: 0.04 };

fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(p::i32(&w.m(id).pvars, o)).ok().filter(|&m| m < w.table.mobys.len()) }

/// Every live moby of the run list (the 0x15ffe4 chain) but this one whose mission byte (+0xb0) is the talk mission
/// (not 0xfe / 0xfd) is deleted.
fn delete_mission_mobys(w: &mut World, id: MobyId) {
    let mission = p::i32(&w.m(id).pvars, pv::TALK_MISSION);
    for m in crate::moby_update::classes::units::hints::run_list(w) {
        if m == id || w.m(m).mission as i32 != mission { continue; }
        let s = w.m(m).state;
        if s == 0xfe || s == 0xfd { continue; }
        w.delete_moby(m);
    }
}

fn hide(w: &mut World, m: MobyId) {
    let mo = w.mm(m);
    mo.has_collision = false;
    mo.mode |= mode::HIDDEN | mode::NO_ANIM;
}

fn cuboid(w: &World, c: i32) -> Option<([f32; 3], [f32; 3])> {
    if c == -1 { return None; }
    w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c).map(|s| (s.centre(), s.euler))
}

pub fn update(w: &mut World, id: MobyId) {
    use crate::hero::bodies::body;
    if w.m(id).pvars.len() < pv::SIZE { w.mm(id).pvars.resize(pv::SIZE, 0); }
    interact::poll_scene_end(w, id);
    // 0x2ec678: the actors' big-head cheat on this class's scene actor (list 1) at 2.75.
    let own = w.m(id).o_class;
    crate::moby_update::manip::scene_big_head(w, &[own], 1, f32::from_bits(0x4030_0000));
    let Some(giant) = link(w, id, pv::GIANT) else { return };
    if w.body() != body::GIANT { hide(w, giant); }
    if w.m(id).state == 9 {
        w.mm(id).update_dist = 0;
        hide(w, id);
        return;
    }
    if w.svc.game_mode == 2 {
        hide(w, id);
    } else {
        let col = w.classes.info(w.m(id).o_class).is_some_and(|i| i.has_collision);
        let m = w.mm(id);
        m.has_collision = col;
        m.mode &= !(mode::HIDDEN | mode::NO_ANIM);
        // 0x2499d8: the shadow slab z ± 0.2.
        m.shadow_hi = m.position[2] + 0.2;
        m.shadow_lo = m.position[2] - 0.2;
    }
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.state = 1;
            p::set_ff(&mut m.pvars, talk::RADIUS, 20.0);
            interact::talk_register(w, id);
            let mission = p::i32(&w.m(id).pvars, pv::TALK_MISSION);
            if crate::moby_update::classes::units::hints::mission_done(w, mission) {
                if w.svc.interact.game.planet_unlocked.get(PLANET as usize).copied().unwrap_or(0) == 0 {
                    w.mm(id).state = 3;
                    delete_mission_mobys(w, id);
                } else {
                    w.mm(id).state = 9;
                }
            }
        }
        1 => {
            let hp = crate::hero::physics::to_f32x3(w.hero.pos);
            let c = p::i32(&w.m(id).pvars, pv::TALK_CUBOID);
            let g = w.hero.group;
            if crate::moby_update::triggers::point_in_cuboid(&w.svc.volumes, hp, c) && (g < 2 || g == 9) && interact::talk_update(w, id) {
                let mission = p::i32(&w.m(id).pvars, pv::TALK_MISSION);
                crate::cinematic::set_mission_done(w, mission as u8);
                let m = w.mm(id);
                m.anim.speed = 0.0;
                m.state = 2;
                delete_mission_mobys(w, id);
            }
        }
        2 => {
            // The port's scene starts after the tick: a talk scene still pending counts as game mode 2 [L].
            if w.svc.game_mode != 2 && w.svc.interact.talker != Some(id) {
                w.mm(id).anim.speed = 1.0;
                if p::i16(&w.m(id).pvars, talk::LAST) != 0 {
                    w.mm(id).state = 9;
                } else {
                    if let Some((pos, rot)) = cuboid(w, p::i32(&w.m(id).pvars, pv::CHECKPOINT)) {
                        crate::moby_update::classes::checkpoint::record(w, crate::moby_update::classes::checkpoint::Record { pos, rot });
                    }
                    w.mm(id).state = 3;
                }
            }
        }
        3 => {
            if w.body() == body::GIANT {
                if let Some(g) = link(w, id, pv::GONE) {
                    let s = w.m(g).state;
                    if s != 0xfe && s != 0xfd {
                        w.delete_moby(g);
                        p::set_i32(&mut w.mm(id).pvars, pv::GONE, -1);
                    }
                }
                let (a, b) = (p::i32(&w.m(id).pvars, pv::GROUP_A), p::i32(&w.m(id).pvars, pv::GROUP_B));
                if crate::moby_update::scheduler::group_count(w, a, 0x40) == 0 && crate::moby_update::scheduler::group_count(w, b, 0x40) == 0 {
                    let t = w.ticks(120);
                    let m = w.mm(id);
                    m.state = 4;
                    p::set_i32(&mut m.pvars, pv::TIMER, t);
                }
            }
        }
        4 => {
            if crate::moby_update::creature::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 {
                crate::hero::bodies::queue_leave(w);
                crate::moby_update::creature::hard_cut(w, giant, 0, 0);
                let m = w.mm(id);
                p::set_i16(&mut m.pvars, talk::NODE, 1);
                p::set_u8(&mut m.pvars, talk::AUTO, 1);
                p::set_i32(&mut m.pvars, pv::DONE, 1);
                crate::cinematic::unlock_planet(w, PLANET);
                let mission = p::i32(&w.m(id).pvars, pv::FINALE_MISSION);
                crate::cinematic::set_mission_done(w, mission as u8);
                if let Some(s) = w.sound.as_deref_mut() { s.music_request(0, 5); }
                if let Some((pos, rot)) = cuboid(w, p::i32(&w.m(id).pvars, pv::FINALE)) {
                    crate::cinematic::hero_teleport(w, pos, rot, 0, true);
                    crate::moby_update::classes::checkpoint::record(w, crate::moby_update::classes::checkpoint::Record { pos, rot });
                }
                w.mm(id).state = 5;
            }
        }
        5 => {
            crate::cinematic::start_scene(w, 1, false);
            w.mm(id).state = 6;
        }
        6 => {
            if w.svc.game_mode != 2 {
                crate::cinematic::start_movie(w, MOVIE);
                w.mm(id).state = 7;
            }
        }
        7 => {
            if w.svc.game_mode != 2 {
                crate::cinematic::show_planet_banner(w, PLANET);
                crate::cinematic::start_scene(w, 2, false);
            }
            // State 8 at once (game mode 2: straight to it, the disassembly's 0x2eccbc).
            crate::cinematic::save(w);
            w.mm(id).state = 9;
        }
        8 => {
            crate::cinematic::save(w);
            w.mm(id).state = 9;
        }
        _ => {}
    }
    // The big-head cheat's +0x1d0 = 2.75: in `look_at_layout` (`talking_npc::look_springs`).
    look_at_layout(w, id, &LOOK);
}
