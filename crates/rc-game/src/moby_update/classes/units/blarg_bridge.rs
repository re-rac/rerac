//! **Blarg's bridge, class 1028** (level06 `0x2f55a0` with its placement `0x2f5d78`; 1 placed; census U227; the name
//! is descriptive [L]). A retracted bridge of 20 deck pairs (class 0x405) and 10 rail pairs (0x406) it makes at
//! load. Once Clank has done his part (the Hydrodisplacer owned, 0x13d4d6, as Clank, in gameplay) the screen fades to a cutaway: the deck
//! slides out piece by piece to 20, the rails flip up (two stages of a quarter turn each, a sound as each piece
//! starts), the camera gliding between cuboids +0x60 → +0x64 → +0x68; its mission is done, then a fade back, a hint
//! (0x1776, record 0x2d), a banner (0x177c) and a save. With the mission done at load it starts extended. The
//! extension words are level globals (gp−0x4e88.., 0x161d78). Read from the level06 decomp and its words
//! gp−0x4eb0..−0x4e4c. Native `f32`.
//!
//! **Pvars**: +0x60 / +0x64 / +0x68 the camera's cuboids, +0x6c the fade's ride (−1 → 1), +0x70 / +0xc0 the 20
//! deck pieces of each side, +0x110 / +0x138 the 10 rails of each side (the last one, +0x134, carries the hum).
//!
//! | state | what | port |
//! |---|---|---|
//! | 0 | mission done → 6, extended (20, 1000°, 1000°); else 1; the 40 deck pieces and 20 rails made at it (draw distance 0x40, drawn, its mode; the first side lit like Ratchet, the second like it); placed | [`update`] |
//! | 1 | Clank (0x1413f4 = 1), the Hydrodisplacer owned (0x13d4d6) and mode 0 → the fade 1, 3, the ride −0.01 | [`update`] |
//! | 3 / 5 | the ride `Approach`ed to 1 by 1/`ticks(20)`, the fade 1 − \|ride\|; crossing 0: (3) `HeroTeleport(here, 0x72, 1)`, `CameraScript(cuboid +0x60, its Euler, 1, 0, 0)`; (5) the body's idle, `CameraScript2(3)`; at 1: (3) → 4, the hum (`PlayClassSound(0, 4)` on rail 9 into +0xbc); (5) → 6, `Help_Request(0x1776, 0x2d)`, `ShowBanner(0x177c, −1)`, save | [`update`] |
//! | 4 | `SetMissionDone`; the deck `0x26a780(20, 15·dt², 15·dt², 10·dt)` (= `0x270830`) below 20; the rails' first flip `(1000, 200·dt², …, 300·dt)` once the deck passes 5, the second once the first passes 300°; at 1000° → 5, the ride −1; the deck out and the hum alive → released; placed; the camera lerped +0x60 → +0x64 by deck/20, then +0x64 → +0x68 by second/1000 | [`update`] |
//! | `0x2f5d78` | deck i: the offset `max(deck − i, 0)` along row 0; A = rows·(−0.49, 2, 0) + pos + offset, B = rows·(−0.49, −2, 0) + …; rot.x ∓ clamp(first° − 45°·i, 0, 90°); rail j: A at `max(−0.6 + 0.25·(4 − j), deck − 2j)`, B at `max(deck − 2j − 1, …)`; rot.x −a / b and rot.y −c / −d from the two flips (each piece's half 45° behind the other's), a sound (1, then 2) when a piece leaves 0 | [`place`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, turn, DT, DT2};
use crate::moby_update::services::{HeroCall, World};
use crate::moby_update::story;

pub const REFERENCE_LEVEL: u32 = 6;
pub const UPDATE_FN: u32 = 0x2f_55a0;
pub const CLASSES: [i16; 1] = [1028];
pub const DECK: i16 = 0x405;
pub const RAIL: i16 = 0x406;
/// The extension globals gp−0x4e88 / −0x4e84 / −0x4e80 (deck, first flip °, second flip °) and their velocities
/// gp−0x4e64 / −0x4e60 / −0x4e5c.
const DECK_KEY: u32 = 0x16_1d78;
const FIRST_KEY: u32 = 0x16_1d7c;
const SECOND_KEY: u32 = 0x16_1d80;
const VEL_KEY: [u32; 3] = [0x16_1d9c, 0x16_1da0, 0x16_1da4];
/// gp−0x4eb0 / −0x4ea0: the sides' offsets; −0x4e90 / −0x4e8c: the rails' base and step.
const SIDE_A: [f32; 3] = [f32::from_bits(0xbefa_e148), 2.0, 0.0];
const SIDE_B: [f32; 3] = [f32::from_bits(0xbefa_e148), -2.0, 0.0];
const RAIL_BASE: f32 = f32::from_bits(0xbf19_999a);
const RAIL_STEP: f32 = 0.25;
const DEG: f32 = 0.017_453_292;
/// 0x13d4d6 = the owned-items table 0x13d4c0 + 22: the Hydrodisplacer, Clank's prize at the end of his part (not a
/// story flag of 0x13d388: the same byte `blarg_clank_lift` reads).
const HYDRODISPLACER: usize = 22;

fn gf(w: &World, k: u32) -> f32 { f32::from_bits(w.svc.units.word(k)) }
fn sf(w: &mut World, k: u32, x: f32) { w.svc.units.set_word(k, x.to_bits()); }
/// A piece (here the runtime index + 1; the game keeps the pointer).
fn piece(w: &World, id: MobyId, o: usize) -> Option<MobyId> { story::link(w, c::pi32(w, id, o) - 1) }

fn local(rows: &[[f32; 4]; 4], v: [f32; 3], at: c::V, off: c::V) -> c::V {
    std::array::from_fn(|i| if i == 3 { at[3] } else { rows[0][i] * v[0] + rows[1][i] * v[1] + rows[2][i] * v[2] + at[i] + off[i] })
}

/// `0x2f5d78(m)` (module doc).
fn place(w: &mut World, id: MobyId) {
    let (rows, at) = (w.m(id).rows, w.m(id).position);
    let r0 = rows[0];
    let deck = gf(w, DECK_KEY);
    let (first, second) = (gf(w, FIRST_KEY) * DEG, gf(w, SECOND_KEY) * DEG);
    let (v1, v2) = (gf(w, VEL_KEY[1]) * DEG, gf(w, VEL_KEY[2]) * DEG);
    let q = std::f32::consts::FRAC_PI_2;
    let clampq = |x: f32| x.clamp(0.0, q);
    for i in 0..20 {
        let off = c::set_len3(r0, (deck - i as f32).max(0.0));
        let a = clampq(first - (i as f32 * 45.0) * DEG);
        for (side, o, sign) in [(SIDE_A, 0x70, -1.0), (SIDE_B, 0xc0, 1.0)] {
            let Some(p) = piece(w, id, o + 4 * i) else { continue };
            w.mm(p).position = local(&rows, side, at, off);
            w.mm(p).rotation[0] = sign * a;
            w.build_matrix(p);
        }
    }
    let k45 = std::f32::consts::FRAC_PI_4;
    for j in 0..10 {
        let f14 = deck - 2.0 * j as f32;
        let f12 = RAIL_BASE + (4 - j as i32) as f32 * RAIL_STEP;
        let s1 = f12.max(f14);
        let s2 = (f14 - 1.0).max(f12);
        let (pa, pb) = (piece(w, id, 0x110 + 4 * j), piece(w, id, 0x138 + 4 * j));
        let k = 2.0 * j as f32;
        let a1 = clampq(first - k * k45);
        let a1v = clampq(first - k * k45 - v1);
        let a2 = clampq(first - k * k45 - k45);
        let a2v = clampq(first - k * k45 - k45 - v1);
        let c1 = clampq(second - k * k45);
        let c1v = clampq(second - k * k45 - v2);
        let c2 = clampq(second - k * k45 - k45);
        // The game tests the second rail's start with the first's angle (fVar11), as here.
        let c2v = clampq(second - k * k45 - v2);
        if let Some(p) = pa {
            w.mm(p).position = local(&rows, SIDE_A, at, c::set_len3(r0, s1));
            if a1 != 0.0 && a1v == 0.0 { w.play_sound(1, 0, p); }
            if c1 != 0.0 && c1v == 0.0 { w.play_sound(2, 0, p); }
            w.mm(p).rotation[0] = -a1;
            w.mm(p).rotation[1] = -c1;
            w.build_matrix(p);
        }
        if let Some(p) = pb {
            w.mm(p).position = local(&rows, SIDE_B, at, c::set_len3(r0, s2));
            if a2 != 0.0 && a2v == 0.0 { w.play_sound(1, 0, p); }
            if c2 != 0.0 && c2v == 0.0 { w.play_sound(2, 0, p); }
            w.mm(p).rotation[0] = a2;
            w.mm(p).rotation[1] = -c2;
            w.build_matrix(p);
        }
    }
}

fn make(w: &mut World, id: MobyId, class: i16, o: usize, hero_lit: bool) {
    let Some(p) = w.create_moby(class) else { return };
    c::set_pi32(w, id, o, p as i32 + 1);
    let (mode, pos, rot) = { let m = w.m(id); (m.mode, m.position, m.rotation) };
    let (light, ambient) = if hero_lit { w.hero_moby.map_or((0, [0x40; 4]), |h| (w.m(h).light, w.m(h).ambient)) } else { (w.m(id).light, w.m(id).ambient) };
    let m = w.mm(p);
    m.draw_dist = 0x40;
    m.visible = 1;
    m.light = light;
    m.ambient = ambient;
    m.mode = mode;
    m.position = pos;
    m.rotation = rot;
}

/// The fade's ride (states 3 / 5): returns (old, new).
fn ride(w: &mut World, id: MobyId) -> (f32, f32) {
    let old = c::pf(w, id, 0x6c);
    let mut t = old;
    let n = w.ticks(0x14) as f32;
    turn::approach(1.0, 1.0 / n, &mut t);
    c::set_pf(w, id, 0x6c, t);
    crate::cinematic::set_fade(w, 1.0 - t.abs());
    (old, t)
}

/// Level06 `0x2f55a0` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x160 { w.mm(id).pvars.resize(0x160, 0); }
    match w.m(id).state {
        0 => {
            let mission = w.m(id).mission as i32;
            if super::hints::mission_done(w, mission) {
                w.mm(id).state = 6;
                sf(w, DECK_KEY, 20.0);
                sf(w, SECOND_KEY, 1000.0);
                sf(w, FIRST_KEY, 1000.0);
            } else {
                w.mm(id).state = 1;
            }
            for i in 0..20 {
                make(w, id, DECK, 0x70 + 4 * i, true);
                make(w, id, DECK, 0xc0 + 4 * i, false);
            }
            for j in 0..10 {
                make(w, id, RAIL, 0x110 + 4 * j, true);
                make(w, id, RAIL, 0x138 + 4 * j, false);
            }
            place(w, id);
        }
        1 => {
            if w.body() != 1 || !w.hero.owned.has(HYDRODISPLACER) || w.svc.game_mode != 0 { return; }
            crate::cinematic::set_fade(w, 1.0);
            w.mm(id).state = 3;
            c::set_pf(w, id, 0x6c, f32::from_bits(0xbc23_d70a));
        }
        3 => {
            let (old, t) = ride(w, id);
            if old < 0.0 && 0.0 <= t {
                let (p, r) = (super::hero_pos(w), w.hero.rot.map(|x| f32::from_bits(x.0)));
                crate::cinematic::hero_teleport(w, [p[0], p[1], p[2]], [r[0], r[1], r[2]], 0x72, true);
                if let Some((cp, ce)) = story::cuboid(w, c::pi32(w, id, 0x60)) { crate::cinematic::camera_script(w, cp, ce, 1, 0, false); }
                return;
            }
            if 1.0 <= t {
                w.mm(id).state = 4;
                if let Some(r9) = piece(w, id, 0x134) {
                    let v = w.play_sound(0, 4, r9);
                    w.mm(id).cmd = v as u8;
                }
            }
        }
        4 => extend(w, id),
        5 => {
            let (old, t) = ride(w, id);
            if old < 0.0 && 0.0 <= t {
                w.hero_fields_mut().call(HeroCall::BodyIdle);
                crate::cinematic::camera_script2(w, 3);
                return;
            }
            if 1.0 <= t {
                w.mm(id).state = 6;
                w.svc.help.request(0x1776, 0x2d);
                crate::cinematic::show_banner(w, 0x177c, -1);
                crate::cinematic::save(w);
            }
        }
        _ => {}
    }
}

fn spring_global(w: &mut World, key: u32, vel: u32, target: f32, k: f32, vmax: f32) {
    let (mut x, mut v) = (gf(w, key), gf(w, vel));
    turn::spring(target, k * DT2, k * DT2, vmax * DT, &mut x, &mut v);
    sf(w, key, x);
    sf(w, vel, v);
}

/// State 4 (module doc).
fn extend(w: &mut World, id: MobyId) {
    crate::cinematic::set_mission_done(w, w.m(id).mission);
    if gf(w, DECK_KEY) < 20.0 { spring_global(w, DECK_KEY, VEL_KEY[0], 20.0, 15.0, 10.0); }
    if 5.0 < gf(w, DECK_KEY) && gf(w, FIRST_KEY) < 1000.0 { spring_global(w, FIRST_KEY, VEL_KEY[1], 1000.0, 200.0, 300.0); }
    if 300.0 < gf(w, FIRST_KEY) && gf(w, SECOND_KEY) < 1000.0 { spring_global(w, SECOND_KEY, VEL_KEY[2], 1000.0, 200.0, 300.0); }
    if 1000.0 <= gf(w, SECOND_KEY) {
        w.mm(id).state = 5;
        c::set_pf(w, id, 0x6c, -1.0);
    }
    let v = w.m(id).cmd as i32;
    if 20.0 <= gf(w, DECK_KEY) {
        if let Some(r9) = piece(w, id, 0x134) {
            if w.sound_alive(v, r9) {
                if v != 0xff { w.release_sound(v, r9); }
                w.mm(id).cmd = 0xff;
            }
        }
    }
    place(w, id);
    let deck = gf(w, DECK_KEY);
    let (f, a, b) = if deck < 20.0 { (deck / 20.0, 0x60, 0x64) } else { (gf(w, SECOND_KEY) / 1000.0, 0x64, 0x68) };
    let (Some((pa, ea)), Some((pb, eb))) = (story::cuboid(w, c::pi32(w, id, a)), story::cuboid(w, c::pi32(w, id, b))) else { return };
    let p = std::array::from_fn(|i| pa[i] + (pb[i] - pa[i]) * f);
    let e = std::array::from_fn(|i| c::add_rot(ea[i], c::sub_rot(eb[i], ea[i]) * f));
    crate::cinematic::camera_targets(w, Some(p), Some(e));
}
