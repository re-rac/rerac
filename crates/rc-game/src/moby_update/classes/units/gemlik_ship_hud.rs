//! **Gemlik's ship 69's HUD** (the draw callback level13 `0x2b97f8`, registered by the flight every tick; run by the
//! frame's callbacks, [`crate::moby_update::classes::draw_callbacks::run_frame`]). It is game logic as much as a draw:
//! its target search (`0x2b8be0`) picks the missile lock and **counts the targets left** (+0xe8: the mobys of classes
//! 1003, 388 (Qwark's ship) and 1284 on the run list); the ship lands when the count is 0 ([`super::gemlik_ship`]).
//! The draws: the radar blips, the target circles and the off-screen arrows, the lock marker, the missile pips, the
//! crosshair, the low-health warning and the health gauge (an FX texture whose CLUT entries above the health are
//! made black). Read from the level13 decomp and disassembly (the stack arguments the decompiler dropped: the blips'
//! and the gauge's colours); the tables from the overlay. Screen pixels of the 512 × 416 frame. Native `f32`.
//!
//! ## Coverage
//! | address | what | port |
//! |---|---|---|
//! | 0x2b97f8 | the lock (`0x2b95b8`) | [`lock_update`] |
//! | 0x2b97f8 | a lock: its screen point (`0x2b84e8`); locking (+0x8c > `ticks(500)`): f = +0x8c − `ticks(500)` of `ticks(75)`, the marker at f·5 + 1, turned by the frame / 30 (wrapped, `0x2731d0`), green with alpha `trunc(min(2·(1 − f), 1)·96)`; +0xec = 0. Locked: the marker at 1, red, green blinking every `ticks(20)`, alpha 0x60; +0xec = the lock | [`hud_frame`], [`marker`] |
//! | 0x2b97f8 | the missile pips (0x140947 of them): the strip 0x1cbc60 (25 corners, ×0.5) from (24, 64), 18 apart, a second column (+30, from 64) after the half; 0x50008f00 for each missile left, else 0x20004f00 (`0x2b7b30`) | [`hud_frame`] |
//! | 0x2b97f8 | the crosshair at +0xe0 / +0xe4: FX 0x11 and 0x12 40 × 40, FX 8 10 × 10 (`fun_001f5ab0`, anchor ½, 0xff20ff20) | [`hud_frame`] (`targeting::marker_corners`' corner order) |
//! | 0x2b97f8 | health below a tenth of 0x140950 and (0x15f5cc / 90) odd → `font_print_center_small(256, 390, 0x80000080, msg 0x5269, 100)` | [`hud_frame`] (`DrawCallbacks::screen_texts`) |
//! | 0x2b97f8 | the gauge: +0xf0 eases to the health (a fifth of the gap, at most 1 a frame); cut = clamp(`trunc(+0xf0·251 / 0x140950)` + 2, 2, 0xfd); the CLUT entries (CSM1 order) between +0xf4 and the cut restored below the cut from the copy 0x1cbe70 and black (0x80000000) from it; +0xf4 = cut; `DrawTexturedQuad(368, h − 144, 128, 128, 0, 0, 128, 128, 0x70808080)` with FX 0x28 + +0x104, then FX 0x29 + +0x104 | [`hud_frame`] (`ScreenTex::FxCut`: the edited CLUT is a function of the cut, the engine decodes it) |
//! | 0x2b95b8 | game mode not 3 / 4 → `FastDecTimer(+0x8c)` | [`lock_update`] |
//! | 0x2b95b8 | no lock, the lock timer out, the lock no longer its class (+0x124), deleted or not drawn (+0x31): the lock dropped (+0xec, +0x124 = −1, +0x88); the pad's raw 0x2000 pressed → the dropped one is skipped by the search | [`lock_update`] |
//! | 0x2b95b8 | a live lock: raw 0x2000 pressed → dropped (and skipped); no missiles → dropped | [`lock_update`] |
//! | 0x2b95b8 | the search (11.25°, 11.25°, 255; the camera 0x1670c0 and its Euler 0x1670d8..); another target found while +0x8c > 500, or no missiles → dropped | [`lock_update`] |
//! | 0x2b95b8 | a target found and no lock (or the lock not of class 0x3eb and the find of 0x3eb), missiles left → the lock, +0x8c = +0x138, +0x124 = its class | [`lock_update`] |
//! | 0x2b95b8 | the find ≠ the lock: +0x118 + 1, past `ticks(60)` → dropped; else +0x118 = 0; the missile cooldown 0 and the lock changed class or deleted → +0x88 = 0, +0x124 = −1 | [`lock_update`] |
//! | 0x2b8be0 | +0xe8 = 0; the run list (0x15ffe4): mode 0x1000 without 0x3, drawn, a class of type 5: its screen point within (radius px + 28) of the crosshair (radius: the bounding sphere / 1280 along the camera's left row, projected): with no lock the nearest (squared pixels) but the skipped one, with a lock only the lock (distance 0) | [`search`] |
//! | 0x2b8be0 | classes 0x52 / 0x53 (the fighters' shots): blip 1 (0x75004080 / 0x75007080) | [`search`], [`blip`] |
//! | 0x2b8be0 | 0x6f / 0x4be not hidden: drawn → the green ring (1.0, −frame/20 wrapped); blip 1 (0x75808080). 0x4bf not hidden and past state 1: the same | [`search`], [`ring`] |
//! | 0x2b8be0 | 0xe0 / 0x4c2 / 0xe4 / 0x4c4: blip 2 | [`search`] |
//! | 0x2b8be0 | 0x3eb / 0x191 / 0x184 / 0x504: drawn and not 0x184 → the red rings (1.0 and 1.3, ±frame/20); 0x191: nothing more | [`search`] |
//! | 0x2b8be0 | 0x3eb / 0x184 / 0x504 not drawn: the camera-space direction from Ratchet (0x13f3d0, the transposed camera rows 0x1672d0) at 160 px from the centre, turned to it; +0x114 += 0.1 / (r / π) (r = the off-angle from the ship's yaw / pitch); beyond 45° the arrow (the strip 0x1613c8, 240 / distance + 1.5, alpha `trunc((sin + 1.4)·53)`, 0x8000 green), else the red rings at the projection | [`search`] |
//! | 0x2b8be0 | 0x3eb / 0x184 / 0x504: blip 0 (0x75808080); +0xe8 + 1 | [`search`] |
//! | 0x2b89f8 | the blip: (position − camera) with z 0, turned by π/2 − the camera yaw, / 14; within 46 px: alpha × (46 − d)/8 beyond 38 (`trunc`); `DrawTexturedQuad` of the table 0x1cbdb8[kind] (FX +0x104 + tex + 0x28, its UV rectangle) at (432 + x − cx, h − 80 − y − cy) | [`blip`] |
//! | 0x2b8320 | the lock marker: four `0x2b7fd0` quarter sprites (FX 0x37, 20·s, at angle, +90°, +180°, +270°) | [`marker`] |
//! | 0x2b7fd0 | a sprite quarter: corners P + A − B, P + A, P − B, P (A = h·(sin, cos), B = w·(cos, −sin)); UVs (1, 1), (63, 0), (0, 63), (63, 63) (the packet's register list: UV then XYZ2 per corner; the first UV is the 0x100010 word); RGBA r g b a | [`marker`] |
//! | 0x2b7d50 / 0x2b7b30 | a flat strip of the s16 corner table, turned (`0x2b7d50`) and scaled, at (x, y) | [`ring`], [`strip`] |
//! | 0x218838 | the projection of a world point (with the off-screen ones' circles) | [`super::gemlik_ship::screen_point`] |

use super::gemlik_ship::{pvo, screen_point};
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::draw_callbacks::{ScreenPrim, ScreenTex, ScreenText};
use crate::moby_update::creature::{self as c, add_rot, atan, dist2, dist3, sub_rot};
use crate::moby_update::services::World;

pub const HUD_FN: u32 = 0x2b_97f8;

/// 0x1cbc60: the missile pip (25 corners).
pub(super) const PIP: [[i16; 2]; 25] = [
    [-32, -14], [-16, -12], [-24, -2], [-12, -6], [-24, 2], [-4, -8], [-32, 14], [-12, 6], [-16, 12], [-16, 12], [-16, 12], [-12, 6], [-12, 6], [-4, -8],
    [-4, 8], [4, -9], [4, 9], [12, -8], [12, 8], [18, -6], [18, 6], [22, -3], [22, 4], [24, -1], [24, 2],
];
/// 0x1cbd40: the target ring (30 corners).
const RING: [[i16; 2]; 30] = [
    [-12, -3], [-9, -3], [-9, -9], [-7, -7], [-3, -12], [-3, -9], [-3, -9], [3, -12], [3, -12], [3, -9], [9, -9], [7, -7], [12, -3], [9, -3], [9, -3],
    [12, 2], [12, 2], [9, 2], [9, 8], [7, 6], [3, 11], [3, 8], [3, 8], [-3, 11], [-3, 11], [-3, 8], [-9, 8], [-7, 6], [-12, 2], [-9, 2],
];
/// gp−0x5838 (0x1613c8): the off-screen arrow.
pub(super) const ARROW: [[i16; 2]; 3] = [[2, 8], [-2, 8], [0, -8]];
/// 0x1cbdb8: the blips (tex, u, v, w, h, cx, cy).
const BLIPS: [[i32; 7]; 3] = [[2, 0, 8, 7, 7, 3, 3], [2, 9, 13, 2, 2, 0, 0], [2, 13, 13, 2, 2, 0, 0]];

/// The frame height 0x13e504 (NTSC).
pub(super) const H: f32 = 416.0;
const GREEN_RING: u32 = 0xff00_ff00;
const RED_RING: u32 = 0x5000_00ff;
const BLIP_WHITE: u32 = 0x7580_8080;
/// The classes the search counts (and marks, and tracks off screen); 0x191 is ringed only.
const TARGETS: [i16; 3] = [0x3eb, 0x184, 0x504];
const RINGED: i16 = 0x191;
/// The low-health warning.
const WARNING: i32 = 0x5269;

fn pi(w: &World, id: MobyId, o: usize) -> i32 { c::pi32(w, id, o) }
fn seti(w: &mut World, id: MobyId, o: usize, v: i32) { c::set_pi32(w, id, o, v) }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(pi(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn set_link(w: &mut World, id: MobyId, o: usize, m: Option<MobyId>) { seti(w, id, o, m.map_or(0, |m| m as i32 + 1)) }
pub(super) fn wrap(x: f32) -> f32 { crate::moby_update::classes::flyer::wrap_frac(x) }
fn alive(w: &World, m: MobyId) -> bool { let s = w.m(m).state; s != 0xfe && s != 0xfd }

fn drop_lock(w: &mut World, id: MobyId) {
    set_link(w, id, pvo::MISSILE_TARGET, None);
    seti(w, id, pvo::LOCK_CLASS, -1);
    set_link(w, id, pvo::LOCK, None);
}

/// A flat strip of `table` scaled by `s` and turned by `a`, at (x, y) (`0x2b7d50`; `0x2b7b30` with `a` = 0).
pub(super) fn strip(out: &mut Vec<ScreenPrim>, x: f32, y: f32, s: f32, a: f32, table: &[[i16; 2]], rgba: u32) {
    let (sn, cs) = a.sin_cos();
    let pts: Vec<[f32; 2]> = table.iter().map(|v| {
        let (vx, vy) = (v[0] as f32, v[1] as f32);
        [(vx * cs - vy * sn) * s + x, (vx * sn + vy * cs) * s + y]
    }).collect();
    ScreenPrim::strip(out, &pts, rgba);
}

/// `0x2b7d50(x, y, s, a, 0x1cbd40, 30, ·, rgba)`: a target ring.
pub(super) fn ring(out: &mut Vec<ScreenPrim>, x: f32, y: f32, s: f32, a: f32, rgba: u32) { strip(out, x, y, s, a, &RING, rgba) }

/// `0x2b8320(x, y, s, a, r, g, b, alpha)`: the lock marker's four quarters of FX `fx` (module doc; 0x37 here, Pokitaru's
/// jet `0x311048` the same with FX 0x42).
pub(super) fn marker(out: &mut Vec<ScreenPrim>, fx: usize, x: f32, y: f32, s: f32, a: f32, rgba: [u8; 4]) {
    let side = s * 20.0;
    let colour = u32::from_le_bytes(rgba);
    let mut ang = a;
    for _ in 0..4 {
        let (sn, cs) = ang.sin_cos();
        let av = [side * sn, side * cs];
        let bv = [side * cs, -side * sn];
        let pos = [[x + av[0] - bv[0], y + av[1] - bv[1]], [x + av[0], y + av[1]], [x - bv[0], y - bv[1]], [x, y]];
        out.push(ScreenPrim { tex: ScreenTex::Fx(fx), pos, uv: [[1.0, 1.0], [63.0, 0.0], [0.0, 63.0], [63.0, 63.0]], rgba: colour, add: false });
        ang = add_rot(ang, std::f32::consts::FRAC_PI_2);
    }
}

/// A `fun_001f5ab0(x, y, w, w, 0, ½, ½, uv, uv, tex, …)` sprite (the crosshair).
pub(super) fn sprite(out: &mut Vec<ScreenPrim>, x: f32, y: f32, side: f32, uv: f32, fx: usize, rgba: u32) {
    let h = side * 0.5;
    let pos = [[x - h, y + h], [x + h, y + h], [x - h, y - h], [x + h, y - h]];
    out.push(ScreenPrim { tex: ScreenTex::Fx(fx), pos, uv: [[1.0, 1.0], [uv, 1.0], [1.0, uv], [uv, uv]], rgba, add: false });
}

/// `0x2b89f8(euler, camera, m, kind, rgba, set)`: a radar blip (module doc).
fn blip(w: &World, out: &mut Vec<ScreenPrim>, m: MobyId, kind: usize, rgba: u32, set: i32) {
    let cam = w.camera_point();
    blip_from(w, out, [cam[0], cam[1], cam[2]], m, kind, rgba, set, false);
}

/// The blip relative to `from` (the camera, or what a caller passes in its place); `mirror`: x negated under the mirror
/// cheat 0x15edb4 (the fleet's copy `0x2eaae0`).
#[allow(clippy::too_many_arguments)]
pub(super) fn blip_from(w: &World, out: &mut Vec<ScreenPrim>, from: [f32; 3], m: MobyId, kind: usize, rgba: u32, set: i32, mirror: bool) {
    let th = sub_rot(std::f32::consts::FRAC_PI_2, w.camera_yaw);
    let p = w.m(m).position;
    let b = [p[0] - from[0], p[1] - from[1]];
    let (sn, cs) = th.sin_cos();
    let k = f32::from_bits(0x3d92_4925);
    let v = [(b[0] * cs - b[1] * sn) * k, (b[0] * sn + b[1] * cs) * k];
    let d = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if 46.0 <= d { return; }
    let mut colour = rgba;
    if 38.0 < d {
        let a = ((rgba >> 24) & 0xff) as f32;
        colour = (rgba & 0xff_ffff) | (((a * (46.0 - d) * 0.125) as i32 as u32) << 24);
    }
    let [tex, u, vv, bw, bh, cx, cy] = BLIPS[kind];
    let sign = if mirror && w.svc.cheats.on(4) { -1.0 } else { 1.0 };
    let x = ((sign * v[0]) as i32 + 432 - cx) as f32;
    let y = ((-v[1]) as i32 - 80 + H as i32 - cy) as f32;
    let (bw, bh, u, vv) = (bw as f32, bh as f32, u as f32, vv as f32);
    let pos = [[x, y], [x + bw, y], [x, y + bh], [x + bw, y + bh]];
    let uv = [[u, vv], [u + bw, vv], [u, vv + bh], [u + bw, vv + bh]];
    out.push(ScreenPrim { tex: ScreenTex::Fx((set + tex + 0x28) as usize), pos, uv, rgba: colour, add: false });
}

fn class_type(w: &World, m: MobyId) -> Option<u8> { w.classes.info(w.m(m).o_class).map(|i| i.ty) }

/// `0x2b8be0(11.25°, 11.25°, 255, m, pvars, camera, euler, lock, skip)` (module doc): the target under the crosshair,
/// the HUD's markers, and +0xe8.
fn search(w: &mut World, id: MobyId, out: &mut Vec<ScreenPrim>, lock: Option<MobyId>, skip: Option<MobyId>) -> Option<MobyId> {
    seti(w, id, pvo::TARGETS, 0);
    let set = pi(w, id, pvo::HUD_SET);
    let cross = [pi(w, id, pvo::CROSS_X), pi(w, id, pvo::CROSS_Y)];
    let left = {
        let e = w.hero.loop_in.cam_euler;
        let r = crate::follow_camera::script::euler_rows(e);
        r[1]
    };
    let frame = w.counter as f32;
    let mut best = None;
    let mut best_d = i32::MAX;
    for m in super::hints::run_list(w) {
        let mo = w.m(m);
        let class = mo.o_class;
        if mo.mode & (mode::TARGETABLE | 3) == mode::TARGETABLE && mo.visible != 0 && mo.has_class && class_type(w, m) == Some(5) {
            let p = [mo.position[0], mo.position[1], mo.position[2]];
            if let Some([sx, sy]) = screen_point(w, p, 0) {
                // 32-bit integer arithmetic as the game's (it wraps: a target near the camera projects far off screen).
                let (dx, dy) = (sx.wrapping_sub(cross[0]).wrapping_abs(), sy.wrapping_sub(cross[1]).wrapping_abs());
                let r = mo.bsphere[3] / 1280.0;
                let q = [p[0] + left[0] * r, p[1] + left[1] * r, p[2] + left[2] * r];
                let rpix = screen_point(w, q, 0).map_or(0, |[qx, qy]| {
                    let (ex, ey) = (qx.wrapping_sub(sx) as f32, qy.wrapping_sub(sy) as f32);
                    (ex * ex + ey * ey).abs().sqrt() as i32
                });
                if dx < rpix.wrapping_add(0x1c) && dy < rpix.wrapping_add(0x1c) {
                    match lock {
                        None => {
                            let d = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
                            if Some(m) != skip && d < best_d {
                                best = Some(m);
                                best_d = d;
                            }
                        }
                        Some(l) if l == m => {
                            best = Some(m);
                            best_d = 0;
                        }
                        Some(_) => {}
                    }
                }
            }
        }
        let mo = w.m(m);
        let pos = [mo.position[0], mo.position[1], mo.position[2]];
        let drawn = mo.visible != 0;
        let hidden = mo.mode & 1 != 0;
        match class {
            0x52 => blip(w, out, m, 1, 0x7500_4080, set),
            0x53 => blip(w, out, m, 1, 0x7500_7080, set),
            0x6f | 0x4be if !hidden => {
                if drawn { if let Some([x, y]) = screen_point(w, pos, 0) { ring(out, x as f32, y as f32, 1.0, -wrap(frame / 20.0), GREEN_RING); } }
                blip(w, out, m, 1, BLIP_WHITE, set);
            }
            0x4bf if !hidden && 1 < mo.state => {
                if drawn { if let Some([x, y]) = screen_point(w, pos, 0) { ring(out, x as f32, y as f32, 1.0, -wrap(frame / 20.0), GREEN_RING); } }
                blip(w, out, m, 1, BLIP_WHITE, set);
            }
            0xe0 | 0x4c2 | 0xe4 | 0x4c4 => blip(w, out, m, 2, BLIP_WHITE, set),
            k if k == RINGED || TARGETS.contains(&k) => {
                let proj = screen_point(w, pos, 0);
                if k != 0x184 && drawn {
                    if let Some([x, y]) = proj {
                        ring(out, x as f32, y as f32, 1.0, wrap(frame / 20.0), RED_RING);
                        ring(out, x as f32, y as f32, f32::from_bits(0x3fa6_6666), -wrap(frame / 20.0), RED_RING);
                    }
                }
                if k == RINGED { continue; }
                if !drawn { off_screen(w, id, out, m, proj, frame); }
                blip(w, out, m, 0, BLIP_WHITE, set);
                let n = pi(w, id, pvo::TARGETS) + 1;
                seti(w, id, pvo::TARGETS, n);
            }
            _ => {}
        }
    }
    best
}

/// The off-screen target's arrow (or rings) of `0x2b8be0` (module doc).
fn off_screen(w: &mut World, id: MobyId, out: &mut Vec<ScreenPrim>, m: MobyId, proj: Option<[i32; 2]>, frame: f32) {
    let p = w.m(m).position;
    let hero = w.hero_point();
    let d = [p[0] - hero[0], p[1] - hero[1], p[2] - hero[2]];
    let rows = crate::follow_camera::script::euler_rows(w.hero.loop_in.cam_euler);
    // The transposed camera matrix applied: the direction in camera space (forward, left, up).
    let v = [rows[0][0] * d[0] + rows[0][1] * d[1] + rows[0][2] * d[2], rows[1][0] * d[0] + rows[1][1] * d[1] + rows[1][2] * d[2], rows[2][0] * d[0] + rows[2][1] * d[1] + rows[2][2] * d[2]];
    let (mut x, mut y) = (-v[1], -v[2]);
    let l = (x * x + y * y).sqrt();
    if l != 0.0 {
        x *= 160.0 / l;
        y *= 160.0 / l;
    }
    let ang = add_rot(atan(x, y), std::f32::consts::FRAC_PI_2);
    let (x, y) = (x + 256.0, y + 208.0);
    let ship = w.m(id).position;
    let dist = dist3(ship, w.m(m).position);
    let dy = sub_rot(atan(p[0] - ship[0], p[1] - ship[1]), c::pf(w, id, pvo::YAW)).abs();
    let dp = sub_rot(-atan(dist2(ship, w.m(m).position), p[2] - ship[2]), c::pf(w, id, pvo::PITCH)).abs();
    let r = (dy * dy + dp * dp).abs().sqrt();
    let ph = add_rot(c::pf(w, id, pvo::ARROW_PHASE), 0.1 / (r / std::f32::consts::PI));
    c::set_pf(w, id, pvo::ARROW_PHASE, ph);
    let pulse = ((ph.sin() + 1.4) * 53.0) as i32;
    if std::f32::consts::FRAC_PI_4 <= dy || std::f32::consts::FRAC_PI_4 <= dp {
        strip(out, x, y, 240.0 / dist + 1.5, ang, &ARROW, ((pulse as u32) << 24) | 0x8000);
    } else if let Some([px, py]) = proj {
        ring(out, px as f32, py as f32, 1.0, wrap(frame / 20.0), RED_RING);
        ring(out, px as f32, py as f32, f32::from_bits(0x3fa6_6666), -wrap(frame / 20.0), RED_RING);
    }
}

/// `0x2b95b8(m, pvars)` (module doc).
fn lock_update(w: &mut World, id: MobyId, out: &mut Vec<ScreenPrim>) {
    let gm = w.svc.game_mode;
    if !(gm == 3 || gm == 4) { c::dec_timer_pvar_i32(w, id, pvo::LOCK_T); }
    let pressed = w.hero.loop_in.pad.raw_pressed & 0x2000 != 0;
    let lock = link(w, id, pvo::LOCK);
    let stale = pi(w, id, pvo::LOCK_T) == 0
        || lock.is_none_or(|l| w.m(l).o_class as i32 != pi(w, id, pvo::LOCK_CLASS) || !alive(w, l) || w.m(l).visible == 0);
    let mut skip = None;
    if stale || pressed || w.svc.vehicle.missiles == 0 {
        if pressed { skip = lock; }
        drop_lock(w, id);
    }
    let cur = link(w, id, pvo::LOCK);
    let found = search(w, id, out, cur, skip);
    if (found != cur && 500 < pi(w, id, pvo::LOCK_T)) || w.svc.vehicle.missiles == 0 { drop_lock(w, id); }
    let mut cur = link(w, id, pvo::LOCK);
    if let Some(f) = found {
        let keep = cur.is_some_and(|l| w.m(l).o_class == 0x3eb || w.m(f).o_class != 0x3eb);
        if !keep && w.svc.vehicle.missiles != 0 {
            set_link(w, id, pvo::LOCK, Some(f));
            let t = pi(w, id, pvo::LOCK_TIME);
            seti(w, id, pvo::LOCK_T, t);
            seti(w, id, pvo::LOCK_CLASS, w.m(f).o_class as i32);
        }
        cur = link(w, id, pvo::LOCK);
    }
    if found == cur {
        seti(w, id, pvo::LOCK_LOST, 0);
    } else {
        let n = pi(w, id, pvo::LOCK_LOST) + 1;
        seti(w, id, pvo::LOCK_LOST, n);
        if w.ticks(0x3c) < n { drop_lock(w, id); }
    }
    if let Some(l) = link(w, id, pvo::LOCK) {
        if pi(w, id, pvo::MISSILE_T) == 0 && (w.m(l).o_class as i32 != pi(w, id, pvo::LOCK_CLASS) || !alive(w, l)) {
            set_link(w, id, pvo::LOCK, None);
            seti(w, id, pvo::LOCK_CLASS, -1);
        }
    }
}

/// Level13 `0x2b97f8` (list 1), run by the frame's callbacks (module doc).
pub fn hud_frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pvo::LEN { return; }
    let mut out = Vec::new();
    lock_update(w, id, &mut out);
    if let Some(l) = link(w, id, pvo::LOCK) {
        let p = w.m(l).position;
        let [x, y] = screen_point(w, [p[0], p[1], p[2]], 0).unwrap_or([0, 0]);
        let (x, y) = (x as f32, y as f32);
        let t = pi(w, id, pvo::LOCK_T);
        let t500 = w.ticks(500);
        if t500 < t {
            let f = (t - t500) as f32;
            let total = w.ticks(0x4b) as f32;
            let a = ((1.0 - f / total) * 2.0).min(1.0);
            let alpha = (a * 96.0) as i32 as u8;
            let ang = wrap(w.counter as f32 / 30.0);
            marker(&mut out, 0x37, x, y, (f / total) * 5.0 + 1.0, ang, [0, 0xff, 0, alpha]);
            set_link(w, id, pvo::MISSILE_TARGET, None);
        } else {
            let n = w.ticks(0x14).max(1);
            let g = if (t / n) & 1 != 0 { 0 } else { 0xff };
            marker(&mut out, 0x37, x, y, 1.0, 0.0, [0xff, g, 0, 0x60]);
            set_link(w, id, pvo::MISSILE_TARGET, Some(l));
        }
    }
    let (left, max) = (w.svc.vehicle.missiles as usize, w.svc.vehicle.missiles_max as usize);
    let (mut px, mut py) = (24.0f32, 64.0f32);
    for i in 0..max {
        let rgba = if i < left { 0x5000_8f00 } else { 0x2000_4f00 };
        strip(&mut out, px, py, 0.5, 0.0, &PIP, rgba);
        if i == max >> 1 {
            py = 46.0;
            px += 30.0;
        }
        py += 18.0;
    }
    let (cx, cy) = (pi(w, id, pvo::CROSS_X) as f32, pi(w, id, pvo::CROSS_Y) as f32);
    sprite(&mut out, cx, cy, 40.0, 63.0, 0x11, 0xff20_ff20);
    sprite(&mut out, cx, cy, 40.0, 63.0, 0x12, 0xff20_ff20);
    sprite(&mut out, cx, cy, 10.0, 31.0, 8, 0xff20_ff20);
    let v = &w.svc.vehicle;
    if v.health < v.health_max / 10.0 && (w.counter / 90) & 1 != 0 {
        let text = w.svc.interact.msg(WARNING);
        w.svc.draw_callbacks.screen_texts.push(ScreenText { x: 256, y: 0x186, rgba: 0x8000_0080, text, len: 100, font: rc_formats::font::Font::Small });
    }
    let (health, full) = (w.svc.vehicle.health, w.svc.vehicle.health_max);
    let g = c::pf(w, id, pvo::GAUGE);
    let mut step = (health - g) * 0.2;
    if 1.0 < step.abs() { step /= step.abs(); }
    let g = g + step;
    c::set_pf(w, id, pvo::GAUGE, g);
    let cut = (((g * 251.0) / full) as i32 + 2).clamp(2, 0xfd);
    seti(w, id, pvo::GAUGE_CUT, cut);
    let set = pi(w, id, pvo::HUD_SET);
    let (x, y) = (368.0, H - 144.0);
    let pos = [[x, y], [x + 128.0, y], [x, y + 128.0], [x + 128.0, y + 128.0]];
    let uv = [[0.0, 0.0], [128.0, 0.0], [0.0, 128.0], [128.0, 128.0]];
    out.push(ScreenPrim { tex: ScreenTex::FxCut { fx: (set + 0x28) as usize, cut: cut as u8 }, pos, uv, rgba: 0x7080_8080, add: false });
    out.push(ScreenPrim { tex: ScreenTex::Fx((set + 0x29) as usize), pos, uv, rgba: 0x7080_8080, add: false });
    w.svc.draw_callbacks.screen.extend(out);
}
