//! **The fleet's ship 1379's HUD** (the draw callback level17 `0x2eb9a8`, registered by the flight every tick; run by
//! the frame's callbacks). The level's copy of Gemlik's ship's HUD ([`super::gemlik_ship_hud`], the same tables: the
//! pips, rings, arrow and blips; their helpers `0x2e9c28` / `0x2e9e48` / `0x2ea0c8` / `0x2ea5e0` are Gemlik's): the
//! gauge drawn first (ALPHA 0x44, then 0x48), the lock's timings from the level data (500 / 100 / 600), its class kept
//! at +0x128, the marker of FX 0x2e, the blips mirrored under the mirror cheat, "Mission complete" while landing. Its
//! target search (`0x2eace8`) is the level's own: **it counts the turrets 347 left** (+0xe8; the ship lands at 0,
//! [`super::fleet_ship`]), rings them (red, 2 and 2.5) and, when none is on screen, points an arrow at the nearest.
//! Read from the level17 decomp and disassembly. Screen pixels of the 512 × 416 frame. Native `f32`.
//!
//! ## Coverage
//! | address | what | port |
//! |---|---|---|
//! | 0x2eb9a8 | ALPHA 0x44; the gauge (FX set + 0x28 with its CLUT cut, FX set + 0x29 over it) at (368, h − 144); ALPHA 0x48 | [`hud_frame`] (the rest under 0x48: `ScreenPrim::add`) |
//! | 0x2eb9a8 | the lock (`0x2eb768`); a lock: locking past 500: f of 100, the marker (FX 0x2e) at f·5 + 1 turned by frame / 30, green, `trunc(min(2·(1 − f), 1)·96)`, +0xec = 0; locked: red / green blinking every `ticks(20)`, 0x60, +0xec = the lock | [`hud_frame`] (`gemlik_ship_hud::marker`) |
//! | 0x2eb9a8 | the pips (0x1d99e0 = Gemlik's), the crosshair, the low-health text (msg 0x5269 every other 90 ticks below a tenth), state 8: `font_print_center_small(256, 200, 0x80005080, msg 0x523e, 100)` | [`hud_frame`] |
//! | 0x2eb9a8 | the gauge's cut (Gemlik's) | [`hud_frame`] (`ScreenTex::FxCut`) |
//! | 0x2eb768 | Gemlik's `0x2b95b8` with the lock's class at +0x128 and the lock time 600 (gp−0x4a94) | [`lock_update`] |
//! | 0x2eace8 | +0xe8 = 0; the run list: targetable and drawn creatures (class type 5) under the crosshair as Gemlik's (the lock, the skipped one) | [`search`] |
//! | 0x2eace8 | 0x52 / 0x53: blip 1 (0x70004080 / 0x70007080); 0x6f / 0x4be shown (0x4bf shown past state 1): drawn → the green ring, blip 1 white; 0xe0 / 0x4c2 / 0xe4 / 0x4c4: blip 2; 0x18f, 0x190, 0x185..0x18c, 0x534..0x53a: nothing; other creatures: blip 1 white | [`search`], `gemlik_ship_hud::blip_from` |
//! | 0x2eace8 | 0x15b: drawn → the red rings 0x400000ff (2 and 2.5, ±frame/20) at 4 above it, "one on screen"; the nearest to Ratchet kept; blip 0 white taken from the raised point (so at the radar's centre); +0xe8 + 1 | [`search`] |
//! | 0x2eace8 | 0x184 / 0x3eb / 0x191 / 0x504: drawn and not 0x184 → the red rings 0x500000ff (1, 1.3); blip 0 white; +0xe8 + 1 but for 0x191 | [`search`] |
//! | 0x2eace8 | none of 0x15b on screen and a nearest: the camera-space direction (the transposed camera rows) from Ratchet, at 160 px from the centre, turned to it; +0x114 += 0.1 / (r / π) (r: the off-angle from the ship's yaw / pitch); the arrow (240 / distance + 1.5, alpha `trunc((sin + 1.4)·53)`, 0x8000) | [`search`] |

use super::fleet_ship::pvo;
use super::gemlik_ship::screen_point;
use super::gemlik_ship_hud::{blip_from, marker, ring, sprite, strip, wrap, ARROW, H, PIP};
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::draw_callbacks::{ScreenPrim, ScreenTex, ScreenText};
use crate::moby_update::creature::{self as c, add_rot, atan, dist2, dist3, sub_rot};
use crate::moby_update::services::World;

pub const HUD_FN: u32 = 0x2e_b9a8;

const LOCK_WAIT: i32 = 500;
const LOCK_GROW: i32 = 100;
const LOCK_TIME: i32 = 600;
const MARKER_FX: usize = 0x2e;
const WHITE: u32 = 0x7080_8080;
const GREEN_RING: u32 = 0xff00_ff00;
const TURRET_RING: u32 = 0x4000_00ff;
const RED_RING: u32 = 0x5000_00ff;
const TURRET: i16 = 0x15b;
const WARNING: i32 = 0x5269;
const DONE_MSG: i32 = 0x523e;

fn pi(w: &World, id: MobyId, o: usize) -> i32 { c::pi32(w, id, o) }
fn seti(w: &mut World, id: MobyId, o: usize, v: i32) { c::set_pi32(w, id, o, v) }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(pi(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn set_link(w: &mut World, id: MobyId, o: usize, m: Option<MobyId>) { seti(w, id, o, m.map_or(0, |m| m as i32 + 1)) }
fn alive(w: &World, m: MobyId) -> bool { let s = w.m(m).state; s != 0xfe && s != 0xfd }
fn class_type(w: &World, m: MobyId) -> Option<u8> { w.classes.info(w.m(m).o_class).map(|i| i.ty) }

fn drop_lock(w: &mut World, id: MobyId) {
    set_link(w, id, pvo::MISSILE_TARGET, None);
    seti(w, id, pvo::LOCK_CLASS, -1);
    set_link(w, id, pvo::LOCK, None);
}

/// `0x2eace8(11.25°, 11.25°, 255, m, pvars, camera, euler, lock, skip)` (module doc).
fn search(w: &mut World, id: MobyId, out: &mut Vec<ScreenPrim>, lock: Option<MobyId>, skip: Option<MobyId>) -> Option<MobyId> {
    seti(w, id, pvo::TARGETS, 0);
    let set = pi(w, id, pvo::HUD_SET);
    let cross = [pi(w, id, pvo::CROSS_X), pi(w, id, pvo::CROSS_Y)];
    let left = crate::follow_camera::script::euler_rows(w.hero.loop_in.cam_euler)[1];
    let cam = w.camera_point();
    let cam = [cam[0], cam[1], cam[2]];
    let frame = w.counter as f32;
    let hero = super::hero_pos(w);
    let (mut best, mut best_d) = (None, i32::MAX);
    let mut on_screen = false;
    let (mut nearest, mut nd) = (None, 100_000.0f32);
    for m in super::hints::run_list(w) {
        let mo = w.m(m);
        let class = mo.o_class;
        if mo.mode & mode::TARGETABLE != 0 && mo.visible != 0 && mo.has_class && class_type(w, m) == Some(5) {
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
        let state = mo.state;
        match class {
            0x52 => blip_from(w, out, cam, m, 1, 0x7000_4080, set, true),
            0x53 => blip_from(w, out, cam, m, 1, 0x7000_7080, set, true),
            0x6f | 0x4be | 0x4bf => {
                if hidden || (class == 0x4bf && state <= 1) { continue; }
                if drawn { if let Some([x, y]) = screen_point(w, pos, 0) { ring(out, x as f32, y as f32, 1.0, -wrap(frame / 20.0), GREEN_RING); } }
                blip_from(w, out, cam, m, 1, WHITE, set, true);
            }
            0xe0 | 0x4c2 | 0xe4 | 0x4c4 => blip_from(w, out, cam, m, 2, WHITE, set, true),
            TURRET => {
                let raised = [pos[0], pos[1], pos[2] + 4.0];
                if drawn {
                    on_screen = true;
                    if let Some([x, y]) = screen_point(w, raised, 0) {
                        ring(out, x as f32, y as f32, 2.0, wrap(frame / 20.0), TURRET_RING);
                        ring(out, x as f32, y as f32, 2.5, -wrap(frame / 20.0), TURRET_RING);
                    }
                }
                let d = dist3(w.m(m).position, hero);
                if d < nd {
                    nearest = Some(m);
                    nd = d;
                }
                blip_from(w, out, raised, m, 0, WHITE, set, true);
                let n = pi(w, id, pvo::TARGETS) + 1;
                seti(w, id, pvo::TARGETS, n);
            }
            0x184 | 0x3eb | 0x191 | 0x504 => {
                if class != 0x184 && drawn {
                    if let Some([x, y]) = screen_point(w, pos, 0) {
                        ring(out, x as f32, y as f32, 1.0, wrap(frame / 20.0), RED_RING);
                        ring(out, x as f32, y as f32, f32::from_bits(0x3fa6_6666), -wrap(frame / 20.0), RED_RING);
                    }
                }
                if class != 0x191 {
                    let n = pi(w, id, pvo::TARGETS) + 1;
                    seti(w, id, pvo::TARGETS, n);
                }
                blip_from(w, out, cam, m, 0, WHITE, set, true);
            }
            0x18f | 0x190 | 0x185..=0x18c | 0x534..=0x53a => {}
            _ => if class_type(w, m) == Some(5) { blip_from(w, out, cam, m, 1, WHITE, set, true); },
        }
    }
    if let Some(t) = nearest.filter(|_| !on_screen) { arrow(w, id, out, t); }
    best
}

/// The arrow toward the nearest turret off screen (module doc).
fn arrow(w: &mut World, id: MobyId, out: &mut Vec<ScreenPrim>, t: MobyId) {
    let p = w.m(t).position;
    let hero = super::hero_pos(w);
    let d = [p[0] - hero[0], p[1] - hero[1], p[2] - hero[2]];
    let rows = crate::follow_camera::script::euler_rows(w.hero.loop_in.cam_euler);
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
    let dist = dist3(ship, p);
    let dy = sub_rot(atan(p[0] - ship[0], p[1] - ship[1]), c::pf(w, id, pvo::YAW)).abs();
    let dp = sub_rot(-atan(dist2(ship, p), p[2] - ship[2]), c::pf(w, id, pvo::PITCH)).abs();
    let r = (dy * dy + dp * dp).abs().sqrt();
    let ph = add_rot(c::pf(w, id, pvo::ARROW_PHASE), 0.1 / (r / std::f32::consts::PI));
    c::set_pf(w, id, pvo::ARROW_PHASE, ph);
    let pulse = ((ph.sin() + 1.4) * 53.0) as i32;
    strip(out, x, y, 240.0 / dist + 1.5, ang, &ARROW, ((pulse as u32) << 24) | 0x8000);
}

/// `0x2eb768(m, pvars)` (module doc).
fn lock_update(w: &mut World, id: MobyId, out: &mut Vec<ScreenPrim>) {
    let gm = w.svc.game_mode;
    if !(gm == 3 || gm == 4) { c::dec_timer_pvar_i32(w, id, pvo::LOCK_T); }
    let pressed = w.hero.loop_in.pad.raw_pressed & 0x2000 != 0;
    let lock = link(w, id, pvo::LOCK);
    let stale = pi(w, id, pvo::LOCK_T) == 0 || lock.is_none_or(|l| w.m(l).o_class as i32 != pi(w, id, pvo::LOCK_CLASS) || !alive(w, l) || w.m(l).visible == 0);
    let mut skip = None;
    if stale || pressed || w.svc.vehicle.missiles == 0 {
        if pressed { skip = lock; }
        drop_lock(w, id);
    }
    let cur = link(w, id, pvo::LOCK);
    let found = search(w, id, out, cur, skip);
    if (found != cur && LOCK_WAIT < pi(w, id, pvo::LOCK_T)) || w.svc.vehicle.missiles == 0 { drop_lock(w, id); }
    let mut cur = link(w, id, pvo::LOCK);
    if let Some(f) = found {
        let keep = cur.is_some_and(|l| w.m(l).o_class == 0x3eb || w.m(f).o_class != 0x3eb);
        if !keep && w.svc.vehicle.missiles != 0 {
            set_link(w, id, pvo::LOCK, Some(f));
            seti(w, id, pvo::LOCK_T, LOCK_TIME);
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

/// Level17 `0x2eb9a8` (list 1), run by the frame's callbacks (module doc).
pub fn hud_frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pvo::LEN { return; }
    let mut out = Vec::new();
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
    lock_update(w, id, &mut out);
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
    if w.m(id).state == 8 {
        let text = w.svc.interact.msg(DONE_MSG);
        w.svc.draw_callbacks.screen_texts.push(ScreenText { x: 256, y: 200, rgba: 0x8000_5080, text, len: 100, font: rc_formats::font::Font::Small });
    }
    // ALPHA_1 0x48 (`VU1_addGSregister(0x42, 0x8000000048)`) after the gauge's two quads.
    for q in out.iter_mut().skip(2) { q.add = true; }
    w.svc.draw_callbacks.screen.extend(out);
}
