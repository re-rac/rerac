//! **Blarg's Clank station** (class 1061, level06 0x2fc640; census U220, one placed instance): the lift pad where Ratchet
//! and Clank trade places on the Blarg station. △ on the pad (its cuboid) fades to black and: as Clank, gives the hero
//! back to Ratchet (the level's leave copy 0x227cf8) at one cuboid; without the O2 Mask, hands the hero to Clank
//! (`SwitchCharacter(1, 0x43, clank)` = level06 0x227bf0, `crate::hero::bodies`) at another; with the mask, Ratchet
//! himself goes through the airlock (0x14161b, the mask on). Its three linked mobys (the airlock's fields, +0x60 /
//! +0x64 / +0x68, each with seven parts) are told to open and close (0x2f53e8 / 0x2f54a0 / 0x2f5360), and the pad
//! itself rides between its cuboid's floor and ceiling with whoever stands on it. Read from the level06 decomp (the
//! only copy).
//!
//! **Pvar block** (s32 unless noted): +0x60 / +0x64 / +0x68 the linked mobys, +0x6c Clank's moby (class 0x57), +0x70
//! the pad cuboid, +0x78 the shaft cuboid (+0x38 its floor z, +0x28 its height), +0x7c the cuboid Ratchet comes back at,
//! +0x80 the one Clank / Ratchet goes out at, +0x84 the side (1: out), +0x88 = −1, +0x8c (f32) the ride's velocity,
//! +0x90 the lockout timer, +0x94 no air (→ 0x14161b), +0x9c (f32) the glow phase.
//!
//! | address | what | port |
//! |---|---|---|
//! | every tick | scale +0x2c = class scale × 1.1 (gp−0x4ca0) | [`update`] |
//! | state 0 | gp−0x4c88 = −1 → = (Hydrodisplacer owned, `0x13d4d6`); +0x94 = 0; +0x88 = −1; → 1, or 3 with the global flag 0x13d3a8; z = the shaft's floor + height; on foot: Clank hidden / frozen / no collision; as Clank: +0x60 told 0x2f53e8; +0x64 and +0x68 told 0x2f53e8 | [`update`] |
//! | state 1 | Ratchet in the pad cuboid → flag 0x13d3a8 = 1, → 3, timer 0; else the hero moby of class 0x57 → 3, +0x84 = 1 | [`update`] |
//! | state 2 | game mode 0 → 3 | [`update`] |
//! | states 3 / 4 | the timer out and Ratchet in the pad cuboid: `try_set_help_message(9, 0x1781 (Clank) / 0x177e (no mask) / 0x1789 (+0x84 = 0) / 0x1785)`; △ with the prompt held, not in state 100: the jump lockout 0x13f542 = 60, timer 120, `FadeToBlack(16)`, then by the body / mask / side: (Clank) the fields 0x2f54a0 / 0x2f54a0 / 0x2f5360, +0x84 = 0, leave, `HeroTeleport(+0x7c)`, `MusicRequestTrack(0, 10)`, Clank hidden; the Hydrodisplacer owned and gp−0x4c88 = 0 → it = 1, the scene 3 (0x2a5938), → 2; (no mask) 0x2f5360 / 0x2f53e8 / 0x2f54a0, +0x84 = 1, Clank's collision on and shown, `HeroTeleport(+0x80)`, `SwitchCharacter(1, 0x43, clank)`, `MusicRequestTrack(2, 7)`; (side 0) the same fields, +0x94 = 1, `HeroTeleport(+0x80)`, +0x84 = 1; (side 1) 0x2f54a0 / 0x2f53e8 / 0x2f5360, +0x84 = 0, `HeroTeleport(+0x7c)`, +0x94 = 0 | [`update`] |
//! | | the ride: on the pad (0x13f64c = self), grounded, the timer out (a second count-down), 0.29 above it and within 1 (xy): its class sound 0, → 5 (from 3, +0x64 told 0x2f53e8) or 6 (+0x68), timer 120; not on it: within 24 (xy) in state 3 with the pad 5 above → +0x94 = +0x84 = 1, → 5, +0x64 and +0x60 told 0x2f53e8, timer 120; in state 4 with the pad 5 below → 6 | [`update`] |
//! | states 5 / 6 | the spring (gp−0x4c98 = 1·dt², 1·dt², 2·dt) to the shaft's floor (5) or floor + height (6); there: 6 → 7, +0x64 told 0x2f5360; 5 → 8, +0x68 told 0x2f5360 | [`update`] (`creature::turn::spring`) |
//! | states 7 / 8 | Ratchet off the pad (≥ 1 xy) → 3 / 4 | [`update`] |
//! | tail | 0x14161b = (+0x94 ≠ 0); the glow +0x90: states 5..8 the colour gp−0x4c8c (phase 0), else the phase +300°/s and `FastTweenColor(clamp(4·sin − 3, 0, 1), gp−0x4c94, gp−0x4c90)` | [`update`] |
//! | 0x2f53e8 / 0x2f54a0 / 0x2f5360 | the linked mobys' +0xbc = 1 and, from their states 6 / 6 / 2: → 8 (class sound 0, shown, drawn, their seven parts → 9), → 2 (shown, drawn, +0x14 = 0, rotation x = +0x10, matrix; the parts → 3 likewise), → 4 (class sound 0, +0x18 = +0x14 = 0, the parts → 5) | [`field_close`], [`field_open`], [`field_lower`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::{self as c, turn};
use crate::moby_update::services::World;

/// The update in the level06 class table.
pub const UPDATE_FN: u32 = 0x2f_c640;
pub const REFERENCE_LEVEL: u32 = 6;
pub const CLASSES: [i16; 1] = [1061];

/// The level06 gp data (gp−0x4ca0 .. −0x4c88).
const SCALE: f32 = 1.1;
const RIDE_MAX: f32 = 2.0;
const RIDE_K: f32 = 1.0;
const GLOW_A: u32 = 0x8020_2020;
const GLOW_B: u32 = 0x8080_8080;
const GLOW_RIDE: u32 = 0x8020_8020;
const O2_MASK: usize = 6;
const HYDRODISPLACER: usize = 22;
/// The global flag 0x13d3a8 (0x13d388[0x20]): the pad was reached once.
const FLAG: usize = 0x20;

/// gp−0x4c88 (0x161f78): −1 at load, then whether the Hydrodisplacer was owned the first time the pad ran (a level word).
fn hydro_word(w: &World) -> u32 { w.svc.level_words.get(&0x161f78).copied().unwrap_or(0xffff_ffff) }
fn set_hydro_word(w: &mut World, v: u32) { w.svc.level_words.insert(0x161f78, v); }

fn shaft(w: &World, id: MobyId) -> Option<rc_formats::volumes::Shape> {
    w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c::pi32(w, id, 0x78)).copied()
}

fn cuboid_pose(w: &World, i: i32) -> Option<([f32; 3], [f32; 3])> {
    let s = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, i)?;
    Some((s.centre(), s.euler))
}

fn teleport(w: &mut World, i: i32) {
    if let Some((pos, euler)) = cuboid_pose(w, i) { crate::cinematic::hero_teleport(w, pos, euler, 0, true); }
}

fn music(w: &mut World, track: i16, stinger: i16) {
    if let Some(s) = w.sound.as_deref_mut() { s.music_request(track, stinger); }
}

fn linked(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o)).ok().filter(|&m| m < w.table.mobys.len()) }

/// The seven parts a linked moby lists (its pvar +0x24 .. +0x3c, table indices).
fn parts(w: &World, m: MobyId) -> Vec<MobyId> { (0..7).filter_map(|k| usize::try_from(c::pi32(w, m, 0x24 + 4 * k)).ok().filter(|&p| p < w.table.mobys.len())).collect() }

/// `0x2f53e8(m)`: +0xbc = 1; from state 6: class sound 0, drawn, → 8, shown, +0x18 = 0, the parts → 9, shown, drawn.
pub fn field_close(w: &mut World, m: Option<MobyId>) {
    let Some(m) = m else { return };
    w.mm(m).cmd = 1;
    if w.m(m).state != 6 { return; }
    w.play_sound(0, 0, m);
    let x = w.mm(m);
    x.visible = 1;
    x.state = 8;
    x.mode &= !mode::HIDDEN;
    c::set_pi32(w, m, 0x18, 0);
    for p in parts(w, m) {
        let x = w.mm(p);
        x.state = 9;
        x.mode &= !mode::HIDDEN;
        x.visible = 1;
    }
}

/// `0x2f54a0(m)`: +0xbc = 1; from state 6: drawn, → 2, shown, +0x14 = 0, rotation x = sub_rot(+0x10, 0), matrix; each
/// part likewise → 3.
pub fn field_open(w: &mut World, m: Option<MobyId>) {
    let Some(m) = m else { return };
    w.mm(m).cmd = 1;
    if w.m(m).state != 6 { return; }
    let reset = |w: &mut World, x: MobyId, state: u8| {
        let mm = w.mm(x);
        mm.visible = 1;
        mm.state = state;
        mm.mode &= !mode::HIDDEN;
        c::set_pi32(w, x, 0x14, 0);
        let a = f32::from_bits(c::pi32(w, x, 0x10) as u32);
        w.mm(x).rotation[0] = c::sub_rot(a, 0.0);
        w.build_matrix(x);
    };
    reset(w, m, 2);
    for p in parts(w, m) { reset(w, p, 3); }
}

/// `0x2f5360(m)`: +0xbc = 1; from state 2: class sound 0, → 4, +0x18 = +0x14 = 0, the parts → 5.
pub fn field_lower(w: &mut World, m: Option<MobyId>) {
    let Some(m) = m else { return };
    w.mm(m).cmd = 1;
    if w.m(m).state != 2 { return; }
    w.play_sound(0, 0, m);
    w.mm(m).state = 4;
    c::set_pi32(w, m, 0x18, 0);
    c::set_pi32(w, m, 0x14, 0);
    for p in parts(w, m) { w.mm(p).state = 5; }
}

/// `FastDecTimer` on the s32 at pvar `o` (1 when it was already 0, 2 when it reaches 0 now, else 0).
fn dec(w: &mut World, id: MobyId, o: usize) -> i32 {
    let mut t = c::pi32(w, id, o);
    let r = crate::hero::idle::dec_timer(&mut t);
    c::set_pi32(w, id, o, t);
    r
}

pub fn update(w: &mut World, id: MobyId) {
    use crate::hero::bodies::body;
    let cls = w.classes.info(w.m(id).o_class).map_or(1.0, |i| i.scale);
    w.mm(id).scale = cls * SCALE;
    let (l60, l64, l68) = (linked(w, id, 0x60), linked(w, id, 0x64), linked(w, id, 0x68));
    let clank = linked(w, id, 0x6c);
    let hero = crate::moby_update::classes::units::hero_pos(w);
    let flag = w.svc.interact.game.flags.get(FLAG).copied().unwrap_or(0);
    let o2 = w.hero.owned.has(O2_MASK);
    let ticks = |w: &World, n: i32| w.ticks(n);
    let mut glow_state = true;
    match w.m(id).state {
        0 => {
            if hydro_word(w) == 0xffff_ffff { set_hydro_word(w, w.hero.owned.has(HYDRODISPLACER) as u32); }
            c::set_pi32(w, id, 0x94, 0);
            c::set_pi32(w, id, 0x88, -1);
            w.mm(id).state = if flag == 0 { 1 } else { 3 };
            if let Some(s) = shaft(w, id) { w.mm(id).position[2] = s.centre()[2] + shaft_height(&s); }
            match w.body() {
                body::RATCHET => {
                    if let Some(k) = clank {
                        let m = w.mm(k);
                        m.mode |= mode::HIDDEN | mode::NO_UPDATE;
                        m.has_collision = false;
                    }
                }
                body::CLANK => field_close(w, l60),
                _ => {}
            }
            field_close(w, l64);
            field_close(w, l68);
        }
        1 => {
            if !w.in_cuboid([hero[0], hero[1], hero[2]], c::pi32(w, id, 0x70)) {
                if w.body() == body::CLANK && w.hero.bodies.class == crate::hero::bodies::CLANK_CLASS {
                    w.mm(id).state = 3;
                    c::set_pi32(w, id, 0x84, 1);
                }
            } else {
                if flag == 0 { crate::moby_update::interact::set_global_flag(w, FLAG, 1); }
                w.mm(id).state = 3;
                c::set_pi32(w, id, 0x90, 0);
            }
        }
        2 => {
            if w.svc.game_mode == 0 { w.mm(id).state = 3; }
        }
        3 | 4 => {
            if dec(w, id, 0x90) != 0 && w.in_cuboid([hero[0], hero[1], hero[2]], c::pi32(w, id, 0x70)) {
                let side = c::pi32(w, id, 0x84);
                let msg = if w.body() == body::CLANK { 0x1781 } else if !o2 { 0x177e } else if side == 0 { 0x1789 } else { 0x1785 };
                let lease = w.svc.interact.try_prompt(9, msg);
                if w.hero.loop_in.pad.pressed & crate::pad::button::TRIANGLE != 0 && w.hero.state != 100 && lease != 0 {
                    w.hero_fields_mut().jump_lockout = ticks(w, 60) as i16;
                    let t = ticks(w, 120);
                    c::set_pi32(w, id, 0x90, t);
                    let fade = ticks(w, 16);
                    crate::cinematic::fade_to_black(w, fade);
                    if w.body() == body::CLANK {
                        field_open(w, l64);
                        field_open(w, l68);
                        field_lower(w, l60);
                        c::set_pi32(w, id, 0x84, 0);
                        if let Some(k) = clank { w.mm(k).has_collision = false; }
                        crate::hero::bodies::queue_leave(w);
                        teleport(w, c::pi32(w, id, 0x7c));
                        music(w, 0, 10);
                        if let Some(k) = clank { w.mm(k).mode |= mode::HIDDEN | mode::NO_UPDATE; }
                        if w.hero.owned.has(HYDRODISPLACER) && hydro_word(w) == 0 {
                            set_hydro_word(w, 1);
                            crate::cinematic::start_scene(w, 3, false);
                            w.mm(id).state = 2;
                        }
                    } else if !o2 {
                        field_lower(w, l64);
                        field_close(w, l68);
                        field_open(w, l60);
                        c::set_pi32(w, id, 0x84, 1);
                        if let Some(k) = clank {
                            let col = w.classes.info(w.m(k).o_class).is_some_and(|i| i.has_collision);
                            let m = w.mm(k);
                            m.has_collision = col;
                            m.mode &= !mode::HIDDEN;
                        }
                        teleport(w, c::pi32(w, id, 0x80));
                        if let Some(k) = clank { crate::hero::bodies::queue_switch(w, body::CLANK, crate::hero::bodies::clank::IDLE, k); }
                        music(w, 2, 7);
                    } else if side == 0 {
                        field_lower(w, l64);
                        field_close(w, l68);
                        field_open(w, l60);
                        c::set_pi32(w, id, 0x94, 1);
                        c::set_pi32(w, id, 0x84, 1);
                        teleport(w, c::pi32(w, id, 0x80));
                    } else {
                        field_open(w, l64);
                        field_close(w, l68);
                        field_lower(w, l60);
                        c::set_pi32(w, id, 0x84, 0);
                        teleport(w, c::pi32(w, id, 0x7c));
                        c::set_pi32(w, id, 0x94, 0);
                    }
                }
            }
            if w.m(id).state == 2 {
                // (The scene branch left the case: the game's `goto LAB_002fcf6c` → the tail.)
            } else {
                ride(w, id, l60, l64, l68);
            }
        }
        5 | 6 => {
            glow_state = false;
            let six = w.m(id).state == 6;
            let Some(s) = shaft(w, id) else { tail(w, id, glow_state); return };
            let mut t = s.centre()[2];
            if six { t += shaft_height(&s); }
            let dt = c::DT;
            let (mut z, mut v) = (w.m(id).position[2], f32::from_bits(c::pi32(w, id, 0x8c) as u32));
            turn::spring(t, RIDE_K * dt * dt, RIDE_K * dt * dt, RIDE_MAX * dt, &mut z, &mut v);
            w.mm(id).position[2] = z;
            c::set_pi32(w, id, 0x8c, v.to_bits() as i32);
            if 0.01 <= (z - t).abs() { tail(w, id, glow_state); return; }
            if six {
                w.mm(id).state = 7;
                field_lower(w, l64);
            } else {
                field_lower(w, l68);
                w.mm(id).state = 8;
            }
        }
        7 | 8 => {
            glow_state = false;
            if 1.0 <= c::dist2([hero[0], hero[1], hero[2], 0.0], w.m(id).position) {
                let s = w.m(id).state;
                w.mm(id).state = if s == 7 { 3 } else { 4 };
            }
        }
        _ => {}
    }
    tail(w, id, glow_state);
}

/// The shaft cuboid's +0x28 (its matrix row 2's z: how far the pad rides up from the centre).
fn shaft_height(s: &rc_formats::volumes::Shape) -> f32 { s.matrix[2][2] }

/// The ride part of states 3 / 4 (module doc).
fn ride(w: &mut World, id: MobyId, l60: Option<MobyId>, l64: Option<MobyId>, l68: Option<MobyId>) {
    let hero = crate::moby_update::classes::units::hero_pos(w);
    let pos = w.m(id).position;
    let on = w.hero.ground_moby == Some(id) && w.hero.air_ticks == 0 && dec(w, id, 0x90) != 0 && pos[2] + 0.29 < hero[2] && c::dist2(hero, pos) < 1.0;
    let next = if on {
        w.play_sound(0, 0, id);
        if w.m(id).state == 3 { (5, l64) } else { (6, l68) }
    } else {
        let d = c::dist2(pos, hero);
        if d < 24.0 && w.m(id).state == 3 && hero[2] + 5.0 < pos[2] {
            c::set_pi32(w, id, 0x94, 1);
            c::set_pi32(w, id, 0x84, 1);
            w.mm(id).state = 5;
            field_close(w, l64);
            (5, l60)
        } else if d < 24.0 && w.m(id).state == 4 && pos[2] < hero[2] - 5.0 {
            (6, l68)
        } else {
            return;
        }
    };
    w.mm(id).state = next.0;
    field_close(w, next.1);
    let t = w.ticks(120);
    c::set_pi32(w, id, 0x90, t);
}

/// The tail (0x2fcf74): 0x14161b = (+0x94 ≠ 0), the glow.
fn tail(w: &mut World, id: MobyId, cycling: bool) {
    let air = c::pi32(w, id, 0x94) != 0;
    w.hero_fields_mut().airless = Some(air as u8);
    let s = w.m(id).state;
    let colour = if s.wrapping_sub(5) < 4 || !cycling {
        c::set_pi32(w, id, 0x9c, 0);
        GLOW_RIDE
    } else {
        let dt = c::DT;
        let a = c::add_rot(f32::from_bits(c::pi32(w, id, 0x9c) as u32), dt * 5.235_987_7);
        c::set_pi32(w, id, 0x9c, a.to_bits() as i32);
        let f = (crate::hero::physics::fast_sin(crate::ps2v::Pf::f(a)).to_f32() * 4.0 - 3.0).clamp(0.0, 1.0);
        crate::hud::tween_color(f, GLOW_A, GLOW_B)
    };
    w.mm(id).glow = colour;
}
