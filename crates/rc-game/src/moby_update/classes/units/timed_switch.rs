//! Timed switches, class 886: level07 0x30bf90, the same code on 12 (census U241; 30 created instances). A switch
//! group: pressing a switch (standing on it, or for a wall switch — tilted more than 45° — wall-jumping off it within
//! 7 animation keys) turns it green, starts the group's countdown with a ticking sound that speeds up as time runs
//! out, and moves the whole group on (states 1 → 2 → 4 → 6 through the group command `0x26e0e0`); the third press
//! wins: the switch blinks and marks its mission done. If the countdown runs out first, sound 2 and the whole group
//! resets (state 8). A switch whose mission is already done starts won. Read from the level07 decomp (0x30bf90).
//! Native `f32`.
//!
//! **Pvar block** (s32): +0x00 the countdown, +0x04 the next tick sound, +0x08 pressed, +0x0c / +0x0e (s16) the win
//! blink timer / phase, +0x10 the countdown in seconds (≤ 0: 15).
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x30bf90 | state < 6 and a mission whose byte `0x14c050 + level·16` is done → group state 6 | [`update`] (`scheduler::group_state`) |
//! | | press test (not while +0x08): Ratchet on it (`0x13f64c` = m, air `0x13f65e` = 0); a wall switch (`|rot.y|` > π/4, 0x221128): within 2 (xy, 0x221398) and in state 0x11 (wall jump) before key 7 (`0x13fdf8`): the local offset (rows +0xc0 transposed, 0x221c08 / 0x221608) within 0.6 / 0.6 / 0.957 | [`pressed`] |
//! | | states 2..5 and +0xbc = 1: countdown running → when +0x04 runs out, tick sound 0 and the next tick in `ticks(45)` if `ticks(45) < ticks(5) + t / ticks(40)`, else `ticks(5) + t / ticks(40)` (faster as t falls); run out → sound 2, group state 8; then the state switch below | [`update`] |
//! | state 0 | ambient 0x80 grey (0x2650d0), group +0xbc = 0 (0x26e090), group state 1 | [`update`] |
//! | state 1 | +0x08 = 0, grey; pressed → countdown `ticks(+0x10 · 60)` (900), green, +0x08 = 1, group +0xbc = 0, own +0xbc = 1, sound 3, group state 2 | [`update`] |
//! | 2 / 4 | → 3 / 5 | [`update`] |
//! | 3 | pressed → +0x08, green, sound 3, group state 4 | [`update`] |
//! | 5 | pressed → +0x08, green, group state 6, sound 1 | [`update`] |
//! | 6 | blink timer `ticks(20)`, phase 0, ambient (0x80, 0xff, 0x80), → 7 | [`update`] |
//! | 7 | `SetMissionDone(mission)` (0x265080) while not done; every `ticks(20)`: phase 0 → green, 1 → white | [`update`] (`cinematic::set_mission_done`) |
//! | 8 | grey, → 1, +0xbc = 0 | [`update`] |
//! | | no particle, light, save flag other than the mission byte | n/a |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{dec_timer_pvar_i32, dec_timer_pvar_s16, dist2, pi16, pi32, set_pi16, set_pi32};
use crate::moby_update::scheduler::{group_cmd, group_state};
use crate::moby_update::services::World;

/// The update in the level07 class table.
pub const UPDATE_FN: u32 = 0x30_bf90;
pub const REFERENCE_LEVEL: u32 = 7;
pub const CLASSES: [i16; 1] = [886];

/// Ratchet's wall jump (`0x1413d4`).
const WALL_JUMP: i32 = 0x11;

fn ambient(w: &mut World, id: MobyId, r: u8, g: u8, b: u8) {
    let a = &mut w.mm(id).ambient;
    a[0] = r;
    a[1] = g;
    a[2] = b;
}

/// The press test (module doc).
pub fn pressed(w: &World, id: MobyId) -> bool {
    let h = w.hero;
    let on = h.air_ticks == 0 && h.ground_moby == Some(id);
    let m = w.m(id);
    if m.rotation[1].abs() <= std::f32::consts::FRAC_PI_4 { return on; }
    let hp = h.pos.map(|x| f32::from_bits(x.0));
    if dist2(m.position, hp) >= 2.0 || h.state != WALL_JUMP || h.loop_in.anim.frame >= 7.0 { return on; }
    let d: [f32; 4] = std::array::from_fn(|k| m.position[k] - hp[k]);
    let l: [f32; 3] = std::array::from_fn(|k| (0..4).map(|i| m.rows[k][i] * d[i]).sum());
    if l[0].abs() < 0.6 && l[1].abs() < 0.6 { l[2].abs() < 0.957 } else { on }
}

/// Level07 0x30bf90 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x14 { return; }
    let (group, mission, level) = (w.m(id).group, w.m(id).mission, w.svc.level);
    if w.m(id).state < 6 && mission != 0xff && w.mission_done(level, mission) == 0xff { group_state(w, group, 6); }
    let press = pi32(w, id, 8) == 0 && pressed(w, id);
    if (2..6).contains(&w.m(id).state) && w.m(id).cmd == 1 {
        if dec_timer_pvar_i32(w, id, 0) == 0 {
            if dec_timer_pvar_i32(w, id, 4) != 0 {
                w.play_sound(0, 0, id);
                let (a, b, c) = (w.ticks(45), w.ticks(5), w.ticks(40));
                let t = pi32(w, id, 0);
                let next = if a < b + t / c { w.ticks(45) } else { w.ticks(5) + t / w.ticks(40) };
                set_pi32(w, id, 4, next);
            }
        } else {
            w.play_sound(2, 0, id);
            group_state(w, group, 8);
        }
    }
    let green = |w: &mut World| ambient(w, id, 0, 0xff, 0);
    match w.m(id).state {
        0 => {
            ambient(w, id, 0x80, 0x80, 0x80);
            group_cmd(w, group, 0);
            group_state(w, group, 1);
        }
        1 => {
            set_pi32(w, id, 8, 0);
            ambient(w, id, 0x80, 0x80, 0x80);
            if press {
                let secs = pi32(w, id, 0x10);
                let t = w.ticks(if secs < 1 { 900 } else { secs * 60 });
                set_pi32(w, id, 0, t);
                green(w);
                set_pi32(w, id, 8, 1);
                group_cmd(w, group, 0);
                w.mm(id).cmd = 1;
                w.play_sound(3, 0, id);
                group_state(w, group, 2);
            }
        }
        2 => w.mm(id).state = 3,
        3 => {
            if press {
                set_pi32(w, id, 8, 1);
                green(w);
                w.play_sound(3, 0, id);
                group_state(w, group, 4);
            }
        }
        4 => w.mm(id).state = 5,
        5 => {
            if press {
                set_pi32(w, id, 8, 1);
                green(w);
                group_state(w, group, 6);
                w.play_sound(1, 0, id);
            }
        }
        6 => {
            let t = w.ticks(20);
            set_pi16(w, id, 0xe, 0);
            set_pi16(w, id, 0xc, t as i16);
            ambient(w, id, 0x80, 0xff, 0x80);
            w.mm(id).state = 7;
        }
        7 => {
            if mission != 0xff && w.mission_done(level, mission) != 0xff { crate::cinematic::set_mission_done(w, mission); }
            if dec_timer_pvar_s16(w, id, 0xc) == 0 { return; }
            let t = w.ticks(20);
            set_pi16(w, id, 0xc, t as i16);
            match pi16(w, id, 0xe) {
                1 => {
                    ambient(w, id, 0xff, 0xff, 0xff);
                    set_pi16(w, id, 0xe, 0);
                }
                0 => {
                    green(w);
                    set_pi16(w, id, 0xe, 1);
                }
                2..=5 => set_pi16(w, id, 0xe, 0),
                _ => {}
            }
        }
        8 => {
            ambient(w, id, 0x80, 0x80, 0x80);
            let m = w.mm(id);
            m.state = 1;
            m.cmd = 0;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::scheduler::Groups;
    use crate::moby_update::services::pvar as p;

    #[test]
    fn three_presses_win_and_a_timeout_resets() {
        let sw = |x: f32| Moby { o_class: 886, group: 0, mission: 0xff, position: [x, 0.0, 0.0, 1.0], pvars: vec![0; 0x20], ..Moby::default() };
        let mut t = MobyTable::new(vec![sw(0.0), sw(10.0), sw(20.0)], 4);
        let mut hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.groups = Groups { lists: vec![Some(vec![0, 1, 2])] };
        p::set_i32(&mut t.mobys[0].pvars, 0x10, 2);
        let mut tick = |t: &mut MobyTable, hero: &crate::hero::Hero, svc: &mut crate::moby_update::Services| {
            let mut w = World::new(t, hero, &mut rng, &classes, svc, 1);
            for id in 0..3 { update(&mut w, id); }
        };
        tick(&mut t, &hero, &mut svc);
        assert!(t.mobys.iter().take(3).all(|m| m.state == 1 && m.ambient[..3] == [0x80; 3]));
        hero.ground_moby = Some(0);
        tick(&mut t, &hero, &mut svc);
        assert_eq!((t.mobys[0].cmd, t.mobys[0].ambient[1], p::i32(&t.mobys[0].pvars, 0)), (1, 0xff, 120));
        assert_eq!(svc.sounds.iter().map(|s| s.index).collect::<Vec<_>>(), [3]);
        // The group moved on: 2 → 3; the others wait for their press.
        hero.ground_moby = None;
        tick(&mut t, &hero, &mut svc);
        assert!(t.mobys.iter().take(3).all(|m| m.state == 3));
        hero.ground_moby = Some(1);
        tick(&mut t, &hero, &mut svc);
        hero.ground_moby = None;
        tick(&mut t, &hero, &mut svc);
        assert!(t.mobys.iter().take(3).all(|m| m.state == 5));
        hero.ground_moby = Some(2);
        tick(&mut t, &hero, &mut svc);
        assert!(t.mobys.iter().take(3).all(|m| m.state == 6));
        tick(&mut t, &hero, &mut svc);
        assert_eq!((t.mobys[0].state, t.mobys[0].ambient[..3].to_vec()), (7, vec![0x80, 0xff, 0x80]));
        // A timeout instead: back to 1 through 8 with sound 2.
        for m in t.mobys.iter_mut().take(3) { m.state = 3; m.cmd = 0; }
        t.mobys[0].cmd = 1;
        p::set_i32(&mut t.mobys[0].pvars, 0, 1);
        hero.ground_moby = None;
        tick(&mut t, &hero, &mut svc);
        assert!(svc.sounds.iter().any(|s| s.index == 2));
        assert!(t.mobys.iter().take(3).all(|m| m.state == 1), "{:?}", t.mobys.iter().take(3).map(|m| m.state).collect::<Vec<_>>());
    }
}
