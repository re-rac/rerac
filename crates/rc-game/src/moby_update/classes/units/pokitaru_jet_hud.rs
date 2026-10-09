//! **Pokitaru's jet 1242's HUD** (the draw callback level11 `0x311d50`, registered by the flight every tick; run by the
//! frame's callbacks, [`crate::moby_update::classes::draw_callbacks::run_frame`]). Like Gemlik's ship's
//! ([`super::gemlik_ship_hud`]) it is game logic as much as a draw: its lock search (`0x311948`) **counts the convoys
//! left** (+0xe8: the live members of the convoys' group, +0xea the drawn ones; [`super::pokitaru_jet`] lands when
//! none is left and calls in ambushers while some are drawn) and locks the missiles on the cars nearest the crosshair.
//! The draws: the gauge (FX 0x3c with its CLUT cut above the health, FX 0x3d over it), the radar blips of the
//! convoys and their cars, the missile pips, the crosshair, the lock marker, "Mission complete" while landing. It also
//! asks for the jet's help lines (low health, few missiles, the fighters). Read from the level11 decomp and
//! disassembly (the stack arguments: the gauge's colour, the blips' alpha); the tables from the overlay. Screen pixels
//! of the 512 × 416 frame. Native `f32`.
//!
//! ## Coverage
//! | address | what | port |
//! |---|---|---|
//! | 0x311d50 | ALPHA 0x44; `DrawTexturedQuad(368, h − 144, 128, 128, 0, 0, 128, 128, 0x70808080)` with FX 0x3c (its CLUT cut, below) and FX 0x3d; ALPHA 0x48 | [`hud_frame`] (`ScreenTex::FxCut`; the rest under 0x48: `ScreenPrim::add`) |
//! | 0x311d50 | FX 0x3e; each live member of the convoys' group (+0x108): it and its cars (`0x318050`) a blip (`0x311710(camera Euler, camera, m, 0, 0x75808080)`) | [`hud_frame`], [`blip`] (`pokitaru_convoy::members`) |
//! | 0x311d50 | the missile pips (0x140947 of them): Gemlik's strip (0x1f0cf8 = 0x1cbc60) from (24, 64), 18 apart, the second column after the half; 0x50008f00 / 0x20004f00 | [`hud_frame`] (`gemlik_ship_hud::{strip, PIP}`) |
//! | 0x311d50 | the crosshair at +0xe0 / +0xe4: FX 0x11 and 0x12 40 × 40, FX 8 10 × 10 (0xff20ff20) | [`hud_frame`] (`gemlik_ship_hud::sprite`) |
//! | 0x311d50 | the lock (`0x311c80`); a lock: its screen point (`0x311210`); locking (+0x8c > 500): f = +0x8c − 500 of 100, the marker at f·5 + 1, turned by the frame / 30 (wrapped), green, alpha `trunc(min(2·(1 − f), 1)·96)`, +0xec = 0; locked: the marker at 1, red with green blinking every `ticks(20)`, alpha 0x60, +0xec = the lock | [`hud_frame`] (`gemlik_ship_hud::marker` with FX 0x42) |
//! | 0x311d50 | health ≥ a quarter: +0x11c ≠ 0 → help record 0x58 bumped, +0x11c = 0; below: record 0x58 unseen, or seen once and +0x11c + 1 past `ticks(1800)` → `Help_Request(0x2aff, 0x58)` | [`helps`] (`hints::{bump, request}`) |
//! | 0x311d50 | `FastDecTimer(+0x120)`; missiles below 5 and record 0x57 unseen, or none left, the record seen at most once and the timer out → `Help_Request(0x2afe, 0x57)`, +0x120 = `ticks(1200)` | [`helps`] |
//! | 0x311d50 | record 0x56 unseen (a fighter downed sets it to 0xffff): +0x128 + 1, past `ticks(3600)` → `Help_Request(0x2afd, 0x56)` | [`helps`] |
//! | 0x311d50 | state 8: `font_print_center_large(256, 200, 0x80005080, msg 0x523e, 0x11)` | [`hud_frame`] (`DrawCallbacks::screen_texts`, the Regular font) |
//! | 0x311d50 | the gauge: +0xf0 eases to the health (a fifth of the gap, at most 1 a frame); cut = clamp(`trunc(+0xf0·251 / 0x140950)` + 2, 2, 0xfd); the CLUT entries (CSM1 order) between +0xf4 and the cut restored from the copy below the cut and black from it; +0xf4 = cut | [`hud_frame`] (`ScreenTex::FxCut`: the edited CLUT is a function of the cut) |
//! | 0x311c80 | no missiles → no lock; a deleted lock dropped; the search (`0x311948(11.25°, 11.25°, 255, …, lock, 0)`): the same → `FastDecTimer(+0x8c)` when there is one; another → the lock, +0x8c = 600 | [`lock_update`] |
//! | 0x311948 | the count (`0x311be8`); no lock, or the lock's score ≥ 1000: the live convoys' cars (not the heads), the lowest score below 1000; else the lock | [`search`] |
//! | 0x311be8 | +0xe8 / +0xea (s16) = the convoys' group's live / live and drawn members | [`search`] |
//! | 0x311a88 | drawn: its screen point within (radius px + 28) of the crosshair in x and y (radius: the bounding sphere / 1280 along the camera's left row, projected) → dx² + dy²; else 10000 | [`score`] |
//! | 0x311710 | the camera yaw (negated with the pitch beyond ±90°); (position − camera) with z 0, turned by π/2 − it (`fun_001fa050`, `fun_001f9d20`), / 7; x negated with the mirror cheat (0x15edb4); within 46 px: alpha × (46 − d)/8 beyond 38 (`trunc`); `DrawTexturedQuad` of the table 0x1f1058[kind] (FX, UV rectangle) at (432 + x − cx, h − 80 − y − cy) | [`blip`] |
//! | 0x311048 / 0x310cf8 | the lock marker: four quarter sprites of FX 0x42 (Gemlik's `0x2b8320` / `0x2b7fd0`) | `gemlik_ship_hud::marker` |
//! | 0x310ad0 | a flat strip of the s16 corner table, scaled, at (x, y) (Gemlik's `0x2b7b30`) | `gemlik_ship_hud::strip` |
//! | 0x311210 | the projection of a world point | [`super::gemlik_ship::screen_point`] |

use super::gemlik_ship::screen_point;
use super::gemlik_ship_hud::{marker, sprite, strip, wrap, PIP};
use super::pokitaru_jet::pvo;
use crate::moby_runtime::MobyId;
use crate::moby_update::classes::draw_callbacks::{ScreenPrim, ScreenTex, ScreenText};
use crate::moby_update::creature::{self as c, sub_rot};
use crate::moby_update::scheduler;
use crate::moby_update::services::World;

pub const HUD_FN: u32 = 0x31_1d50;

/// 0x1f1058: the blips (FX, u, v, w, h, cx, cy).
const BLIPS: [[i32; 7]; 3] = [[0x3e, 0, 8, 7, 7, 3, 3], [0x3e, 9, 13, 2, 2, 0, 0], [0x3e, 13, 13, 2, 2, 0, 0]];
/// The frame height 0x13e504 (NTSC).
const H: f32 = 416.0;
const BLIP_WHITE: u32 = 0x7580_8080;
/// gp−0x4aa0 / −0x4a9c: the lock's wait and its grow-in (raw ticks).
const LOCK_WAIT: i32 = 500;
const LOCK_GROW: i32 = 100;
/// The gauge (FX 0x3c, its CLUT cut) and its frame (FX 0x3d).
const GAUGE_FX: usize = 0x3c;
const FRAME_FX: usize = 0x3d;
const MARKER_FX: usize = 0x42;
/// "Mission complete".
const DONE_MSG: i32 = 0x523e;
/// The help lines (message, record).
const HELP_HEALTH: (i32, usize) = (0x2aff, 0x58);
const HELP_MISSILES: (i32, usize) = (0x2afe, 0x57);
const HELP_FIGHTERS: (i32, usize) = (0x2afd, 0x56);

fn pi(w: &World, id: MobyId, o: usize) -> i32 { c::pi32(w, id, o) }
fn seti(w: &mut World, id: MobyId, o: usize, v: i32) { c::set_pi32(w, id, o, v) }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(pi(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn set_link(w: &mut World, id: MobyId, o: usize, m: Option<MobyId>) { seti(w, id, o, m.map_or(0, |m| m as i32 + 1)) }

/// The live convoys of the jet's group (+0x108).
fn convoys(w: &World, id: MobyId) -> Vec<MobyId> {
    let Ok(g) = i8::try_from(c::pi16(w, id, pvo::CONVOY_GROUP)) else { return Vec::new() };
    scheduler::group_ids(w, g).into_iter().filter(|&m| m < w.table.mobys.len() && (w.m(m).state as i8) >= 0).collect()
}

/// `0x311710(camera Euler, camera, m, kind, rgba)`: a radar blip (module doc).
fn blip(w: &World, out: &mut Vec<ScreenPrim>, m: MobyId, kind: usize, rgba: u32) {
    let e = w.hero.loop_in.cam_euler;
    let mut yaw = e[2];
    if std::f32::consts::FRAC_PI_2 < e[1] || e[1] < -std::f32::consts::FRAC_PI_2 { yaw = -yaw; }
    let th = sub_rot(std::f32::consts::FRAC_PI_2, yaw);
    let rows = super::ship_fighter::rows_of([0.0, 0.0, th]);
    let cam = w.camera_point();
    let p = w.m(m).position;
    let d = [p[0] - cam[0], p[1] - cam[1]];
    let k = f32::from_bits(0x3e12_4925);
    let v = [(rows[0][0] * d[0] + rows[1][0] * d[1]) * k, (rows[0][1] * d[0] + rows[1][1] * d[1]) * k];
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    let sign = if w.svc.cheats.on(4) { -1.0 } else { 1.0 };
    let x = (sign * v[0]) as i32;
    let y = (-v[1]) as i32;
    if 46.0 <= len { return; }
    let mut colour = rgba;
    if 38.0 < len {
        let a = ((rgba >> 24) & 0xff) as f32;
        colour = (rgba & 0xff_ffff) | (((a * (46.0 - len) * 0.125) as i32 as u32) << 24);
    }
    let [tex, u, vv, bw, bh, cx, cy] = BLIPS[kind];
    let sx = (x + 432 - cx) as f32;
    let sy = (y - 80 + H as i32 - cy) as f32;
    let (bw, bh, u, vv) = (bw as f32, bh as f32, u as f32, vv as f32);
    let pos = [[sx, sy], [sx + bw, sy], [sx, sy + bh], [sx + bw, sy + bh]];
    let uv = [[u, vv], [u + bw, vv], [u, vv + bh], [u + bw, vv + bh]];
    out.push(ScreenPrim { tex: ScreenTex::Fx(tex as usize), pos, uv, rgba: colour, add: false });
}

/// `0x311a88(m, target)`: the target's distance from the crosshair (module doc).
fn score(w: &World, id: MobyId, t: MobyId) -> f32 {
    let mo = w.m(t);
    if mo.visible == 0 { return 10000.0; }
    let p = [mo.position[0], mo.position[1], mo.position[2]];
    let [sx, sy] = screen_point(w, p, 0).unwrap_or([0, 0]);
    // 32-bit integer arithmetic as the game's (it wraps: a target near the camera projects far off screen).
    let (dx, dy) = (sx.wrapping_sub(pi(w, id, pvo::CROSS_X)).wrapping_abs(), sy.wrapping_sub(pi(w, id, pvo::CROSS_Y)).wrapping_abs());
    let left = crate::follow_camera::script::euler_rows(w.hero.loop_in.cam_euler)[1];
    let r = mo.bsphere[3] / 1280.0;
    let q = [p[0] + left[0] * r, p[1] + left[1] * r, p[2] + left[2] * r];
    let [qx, qy] = screen_point(w, q, 0).unwrap_or([0, 0]);
    let (ex, ey) = (qx.wrapping_sub(sx), qy.wrapping_sub(sy));
    // `fun_001f9988` (SQRT.S: of |x|).
    let rpix = crate::ps2v::Pf::f(ex.wrapping_mul(ex).wrapping_add(ey.wrapping_mul(ey)) as f32).sqrt().to_f32() as i32;
    let near = rpix.wrapping_add(0x1c);
    if dx < near && dy < near { dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy)) as f32 } else { 10000.0 }
}

/// `0x311948(…, lock, 0)` with the count `0x311be8` (module doc).
fn search(w: &mut World, id: MobyId, lock: Option<MobyId>) -> Option<MobyId> {
    let list = convoys(w, id);
    let drawn = list.iter().filter(|&&m| w.m(m).visible != 0).count();
    c::set_pi16(w, id, pvo::CONVOYS, list.len() as i16);
    c::set_pi16(w, id, pvo::CONVOYS_DRAWN, drawn as i16);
    if let Some(l) = lock {
        if score(w, id, l) < 1000.0 { return Some(l); }
    }
    let mut best = None;
    let mut bd = 1000.0f32;
    for cv in list {
        for car in super::pokitaru_convoy::members(w, cv).into_iter().skip(1) {
            let d = score(w, id, car);
            if d < bd {
                best = Some(car);
                bd = d;
            }
        }
    }
    best
}

/// `0x311c80(m, pvars)` (module doc).
fn lock_update(w: &mut World, id: MobyId) {
    if w.svc.vehicle.missiles == 0 {
        set_link(w, id, pvo::LOCK, None);
        return;
    }
    if link(w, id, pvo::LOCK).is_some_and(|l| matches!(w.m(l).state, 0xfe | 0xfd)) { set_link(w, id, pvo::LOCK, None); }
    let lock = link(w, id, pvo::LOCK);
    let found = search(w, id, lock);
    if found == lock {
        if found.is_some() { c::dec_timer_pvar_i32(w, id, pvo::LOCK_T); }
    } else {
        set_link(w, id, pvo::LOCK, found);
        seti(w, id, pvo::LOCK_T, LOCK_WAIT + LOCK_GROW);
    }
}

/// The help lines of `0x311d50` (module doc).
fn helps(w: &mut World, id: MobyId) {
    let v = &w.svc.vehicle;
    if v.health_max * 0.25 <= v.health {
        if pi(w, id, pvo::HELP_HEALTH_T) != 0 {
            super::hints::bump(w, HELP_HEALTH.1);
            seti(w, id, pvo::HELP_HEALTH_T, 0);
        }
    } else {
        let n = w.svc.help.records.help[HELP_HEALTH.1].count;
        let ask = if n == 0 {
            true
        } else if 1 < n {
            false
        } else {
            let t = pi(w, id, pvo::HELP_HEALTH_T) + 1;
            seti(w, id, pvo::HELP_HEALTH_T, t);
            w.ticks(0x708) < t
        };
        if ask { super::hints::request(w, HELP_HEALTH.0, HELP_HEALTH.1 as i32); }
    }
    c::dec_timer_pvar_i32(w, id, pvo::HELP_MISSILE_T);
    let (left, seen) = (w.svc.vehicle.missiles, w.svc.help.records.help[HELP_MISSILES.1].count);
    if (left < 5 && seen == 0) || (left == 0 && seen < 2 && pi(w, id, pvo::HELP_MISSILE_T) == 0) {
        super::hints::request(w, HELP_MISSILES.0, HELP_MISSILES.1 as i32);
        let t = w.ticks(0x4b0);
        seti(w, id, pvo::HELP_MISSILE_T, t);
    }
    if w.svc.help.records.help[HELP_FIGHTERS.1].count == 0 {
        let t = pi(w, id, pvo::HELP_FIGHTER_T);
        seti(w, id, pvo::HELP_FIGHTER_T, t + 1);
        if w.ticks(0xe10) < t { super::hints::request(w, HELP_FIGHTERS.0, HELP_FIGHTERS.1 as i32); }
    }
}

/// Level11 `0x311d50` (list 1), run by the frame's callbacks (module doc).
pub fn hud_frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pvo::LEN { return; }
    let mut out = Vec::new();
    // The gauge's cut (computed at the end of the callback in the game; the GS reads the CLUT when the frame draws).
    let (health, full) = (w.svc.vehicle.health, w.svc.vehicle.health_max);
    let g = c::pf(w, id, pvo::GAUGE);
    let mut step = (health - g) * 0.2;
    if 1.0 < step.abs() { step /= step.abs(); }
    let g = g + step;
    c::set_pf(w, id, pvo::GAUGE, g);
    let cut = (((g * 251.0) / full) as i32 + 2).clamp(2, 0xfd);
    seti(w, id, pvo::GAUGE_CUT, cut);
    let (x, y) = (368.0, H - 144.0);
    let pos = [[x, y], [x + 128.0, y], [x, y + 128.0], [x + 128.0, y + 128.0]];
    let uv = [[0.0, 0.0], [128.0, 0.0], [0.0, 128.0], [128.0, 128.0]];
    out.push(ScreenPrim { tex: ScreenTex::FxCut { fx: GAUGE_FX, cut: cut as u8 }, pos, uv, rgba: 0x7080_8080, add: false });
    out.push(ScreenPrim { tex: ScreenTex::Fx(FRAME_FX), pos, uv, rgba: 0x7080_8080, add: false });
    for cv in convoys(w, id) {
        for m in super::pokitaru_convoy::members(w, cv) { blip(w, &mut out, m, 0, BLIP_WHITE); }
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
    lock_update(w, id);
    if let Some(l) = link(w, id, pvo::LOCK) {
        let p = w.m(l).position;
        let [x, y] = screen_point(w, [p[0], p[1], p[2]], 0).unwrap_or([0, 0]);
        let (x, y) = (x as f32, y as f32);
        let t = pi(w, id, pvo::LOCK_T);
        if LOCK_WAIT < t {
            let f = (t - LOCK_WAIT) as f32;
            let total = LOCK_GROW as f32;
            let a = ((1.0 - f / total) * 2.0).min(1.0);
            let alpha = (a * 96.0) as i32 as u8;
            let ang = wrap(w.counter as f32 / 30.0);
            marker(&mut out, MARKER_FX, x, y, (f / total) * 5.0 + 1.0, ang, [0, 0xff, 0, alpha]);
            set_link(w, id, pvo::MISSILE_TARGET, None);
        } else {
            let n = w.ticks(0x14).max(1);
            let g = if (t / n) & 1 != 0 { 0 } else { 0xff };
            marker(&mut out, MARKER_FX, x, y, 1.0, 0.0, [0xff, g, 0, 0x60]);
            set_link(w, id, pvo::MISSILE_TARGET, Some(l));
        }
    }
    helps(w, id);
    if w.m(id).state == 8 {
        let text = w.svc.interact.msg(DONE_MSG);
        w.svc.draw_callbacks.screen_texts.push(ScreenText { x: 256, y: 200, rgba: 0x8000_5080, text, len: 0x11, font: rc_formats::font::Font::Regular });
    }
    // ALPHA_1 0x48 (`VU1_addGSregister(0x42, 0x8000000048)`) after the gauge's two quads.
    for q in out.iter_mut().skip(2) { q.add = true; }
    w.svc.draw_callbacks.screen.extend(out);
}
