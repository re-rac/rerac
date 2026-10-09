//! **Clank's gadgetbot pads, class 1302** (level06 `0x307d30` with its draw callback `0x307578` and its link trigger
//! `0x307c48`; 1 placed (level 10's 5 run the same class at `0x2e7cd8`); census U246; the name is descriptive [L]). A
//! pad that wants a number of gadgetbots (+0x10; the gadgetbots 857 count +0x14 down as they arrive). Within 32 of the
//! camera a hologram arrow over it grows in (and shrinks away beyond 36): a camera-facing arrow (four quads, FX 0x18,
//! its texture scrolling) and a rising, widening, fading square (FX 0x1b); while Ratchet is Clank the remaining count
//! floats above it in digits (type-41 sprites, pulsing in colour). At 0, or with its mission done, it triggers its
//! four links (+0x00..+0x0c: a door 0x404 opens 1 → 2; 0x3f7 / 0x502 go 2 → 3 with their sound 0; 0x542 goes to 2
//! with the sound of class 0x434 and its mission done) and fades out (state 2). Its glow pulses. Read from the level06 decomp
//! and its data (0x1fe8d0 the arrow's nine points, 0x1fe960 their ST, 0x1fe9a8 their colours, 0x1fe9d0 the quads)
//! and words gp−0x4924..−0x4914. Native `f32`.
//!
//! **Pvars** (0x50): +0x00..+0x0c the links, +0x10 the count wanted, +0x14 the count left, +0x18 the digits' bob phase,
//! +0x1c the arrow's ST scroll, +0x20 the square's rise (0..1), +0x24 the hologram's scale (0..0.75), +0x28 the
//! glow phase, +0x2c (the port's) the camera yaw for the draw.
//!
//! | state | what | port |
//! |---|---|---|
//! | every tick | no first link → nothing; glow phase += (360°/2)·dt, glow `FastTweenColor((cos + 1)/2, 0x80464646, 0x80828282)` | [`update`] |
//! | 0 | count left = wanted; the phases `random_angle_radians`; → 1; scale 0.75 | [`update`] |
//! | 1 | camera within 32 → scale + 2·dt (at most 0.75); beyond 36 → − 2·dt (at least 0); scale > 0: the draw (`RegisterDrawCallback2`), the bob phase += (360°/2.5)·dt; Clank (0x1413f4 = 1): the digits at z + 2.75 + 0.025·cos(bob) (one below 10, else two 0.15 / −0.4 across its yaw + 90°, swapped with "Levels are mirrored"), frame digit + 2, size 0.5, colour from c = trunc(36·cos(glow)): `0x20000000 \| (c/3 + 13) << 16 \| (c/2 + 120) << 8 \| (c + 150)`, b9 0x7f, timer 4 | [`update`] (`particles::type41`) |
//! | 1 | count left < 1, or its mission done (0x14c050[level·16 + mission] = −1) → each link `0x307c48`, 2 | [`trigger`] |
//! | 2 | scale − 2·dt (at least 0); > 0 → the draw | [`update`] |
//! | `0x307578` | the arrow: the nine points turned by the camera yaw, x / y · scale, at the pad; ST + the scroll (+0.01 a draw); then the square: rise += 0.01 (0 past 1), corners ±1.2·rise·scale at 1.1 + 1.4·rise (two 0.2 lower), turned by the camera yaw, alpha (1 − rise)·128 | [`fx_quad_groups`] |

use super::{FxQuad, FxQuads};
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::creature as c;
use crate::moby_update::services::{pvar as p, Services, World};

pub const REFERENCE_LEVEL: u32 = 6;
pub const UPDATE_FN: u32 = 0x30_7d30;
pub const DRAW_FN: u32 = 0x30_7578;
pub const CLASSES: [i16; 1] = [1302];
/// gp−0x4924 (0.025), −0x4920 (2.5), −0x491c (2), −0x4918 (36), −0x4914 (0.75).
const BOB: f32 = 0.025;
const BOB_PERIOD: f32 = 2.5;
const GLOW_PERIOD: f32 = 2.0;
const PULSE: f32 = 36.0;
const SCALE: f32 = 0.75;
const DEG: f32 = 0.017_453_292;
/// 0x1fe8d0: the arrow's points (x 0; y, z).
const ARROW: [[f32; 2]; 9] = [[-1.0, 2.6], [0.0, 2.6], [1.0, 2.6], [-0.3, 1.4], [0.0, 1.6], [0.3, 1.4], [-0.05, 1.1], [0.0, 1.1], [0.05, 1.1]];
/// 0x1fe960: their ST.
const ARROW_ST: [[f32; 2]; 9] = [[0.0, 0.0], [0.5, 0.0], [1.0, 0.0], [0.0, 0.5], [0.5, 0.5], [1.0, 0.5], [0.0, 1.0], [0.5, 1.0], [1.0, 1.0]];
/// 0x1fe9d0: the quads (strip order).
const QUADS: [[usize; 4]; 4] = [[0, 1, 3, 4], [1, 2, 4, 5], [3, 4, 6, 7], [4, 5, 7, 8]];
const ARROW_FX: usize = 0x18;
const SQUARE_FX: usize = 0x1b;

fn colour(i: usize) -> u32 { if i < 3 { 0x1080_8080 } else { 0x7080_8080 } }

fn link(w: &World, o: i32) -> Option<MobyId> { usize::try_from(o).ok().filter(|&m| m < w.table.mobys.len()) }

/// `0x307c48(m, link)` for each link, then 2.
/// `RC_TRACE_BOTS=1`: the pads' counts and triggers and the gadgetbots' merges and knock-backs on stderr (dev).
pub fn trace() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("RC_TRACE_BOTS").is_ok_and(|v| v.trim() == "1"))
}

fn trigger(w: &mut World, id: MobyId) {
    if trace() {
        let links: Vec<String> = (0..4).filter_map(|k| link(w, c::pi32(w, id, 4 * k))).map(|l| format!("{l} (class {} state {})", w.m(l).o_class, w.m(l).state)).collect();
        eprintln!("bots: tick {}: pad {id} triggers (count left {}, mission {} done {}): {}", w.counter, c::pi32(w, id, 0x14), w.m(id).mission, w.mission_done(w.svc.level, w.m(id).mission) == 0xff, links.join(", "));
    }
    for k in 0..4 {
        let Some(l) = link(w, c::pi32(w, id, 4 * k)) else { continue };
        match w.m(l).o_class {
            0x404 => {
                if w.m(l).state == 1 { w.mm(l).state = 2; }
            }
            0x3f7 | 0x502 => {
                if w.m(l).state == 2 {
                    w.mm(l).state = 3;
                    w.play_sound(0, 0, l);
                }
            }
            0x542 => {
                w.mm(l).state = 2;
                w.play_sound_as(0, 0, l, 0x434);
                w.mm(l).update_dist = 0xff;
                let mission = w.m(id).mission;
                crate::cinematic::set_mission_done(w, mission);
            }
            _ => {}
        }
    }
    w.mm(id).state = 2;
}

fn shrink(w: &mut World, id: MobyId) -> f32 {
    let s = (c::pf(w, id, 0x24) - (c::DT + c::DT)).max(0.0);
    c::set_pf(w, id, 0x24, s);
    s
}

fn register(w: &mut World, id: MobyId) {
    let yaw = w.camera_yaw;
    c::set_pf(w, id, 0x2c, yaw);
    let mut sc = c::pf(w, id, 0x1c) + 0.01;
    if 1.0 < sc { sc -= 1.0; }
    c::set_pf(w, id, 0x1c, sc);
    let mut r = c::pf(w, id, 0x20) + 0.01;
    if 1.0 < r { r = 0.0; }
    c::set_pf(w, id, 0x20, r);
    if let Some(row) = super::row(REFERENCE_LEVEL, DRAW_FN) { w.svc.draw_callbacks.register2(crate::moby_update::classes::draw_callbacks::Callback::UnitQuads(row), id); }
}

/// Level06 `0x307d30` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x30 { w.mm(id).pvars.resize(0x30, 0); }
    if c::pi32(w, id, 0) == -1 { return; }
    let g = c::add_rot(c::pf(w, id, 0x28), (360.0 / GLOW_PERIOD) * DEG * c::DT);
    c::set_pf(w, id, 0x28, g);
    w.mm(id).glow = crate::particles::tween_color(((g.cos() + 1.0) * 0.5).to_bits(), 0x8046_4646, 0x8082_8282);
    let state_now = w.m(id).state;
    match state_now {
        0 => {
            let n = c::pi32(w, id, 0x10);
            c::set_pi32(w, id, 0x14, n);
            let a = w.rng.rand_angle();
            c::set_pf(w, id, 0x18, a);
            let b = w.rng.rand_angle();
            c::set_pf(w, id, 0x28, b);
            w.mm(id).state = 1;
            c::set_pf(w, id, 0x24, SCALE);
        }
        1 => {
            let p0 = c::pos(w, id);
            let mut at = [p0[0], p0[1], p0[2] + 2.75, p0[3]];
            let cam = w.camera.map(|x| f32::from_bits(x.0));
            let d = c::dist3(cam, p0);
            if d < 32.0 {
                let s = (c::pf(w, id, 0x24) + c::DT + c::DT).min(SCALE);
                c::set_pf(w, id, 0x24, s);
            } else if 36.0 < d {
                shrink(w, id);
            }
            if 0.0 < c::pf(w, id, 0x24) {
                register(w, id);
                let ph = c::add_rot(c::pf(w, id, 0x18), (360.0 / BOB_PERIOD) * DEG * c::DT);
                c::set_pf(w, id, 0x18, ph);
                at[2] += BOB * ph.cos();
                let k = (PULSE * c::pf(w, id, 0x28).cos()) as i32;
                let rgba = 0x2000_0000u32 | (((k / 3 + 0xd) as u32) << 16) | (((k / 2 + 0x78) as u32) << 8) | (k + 0x96) as u32;
                if w.body() == 1 { digits(w, id, at, rgba); }
            }
            let left = c::pi32(w, id, 0x14);
            let mission = w.m(id).mission as i32;
            if left < 1 || super::hints::mission_done(w, mission) { trigger(w, id); }
        }
        2 if 0.0 < shrink(w, id) => register(w, id),
        _ => {}
    }
}

/// The remaining count in digits (state 1, module doc).
fn digits(w: &mut World, id: MobyId, at: c::V, rgba: u32) {
    let n = c::pi32(w, id, 0x14);
    let put = |w: &mut World, p: c::V, digit: i32| {
        if let Some(sys) = w.particles.as_deref_mut() {
            crate::particles::type41::spawn(sys, 0.5, p, id as u32, (digit + 2).max(0) as usize, rgba, 0x7f, 4, -1);
        }
    };
    if n < 10 {
        put(w, at, n);
        return;
    }
    let a = c::add_rot(c::yaw(w, id), std::f32::consts::FRAC_PI_2);
    let off = |k: f32| -> c::V { c::add([a.cos() * k, a.sin() * k, 0.0, 0.0], at) };
    let (tens, units) = if w.svc.cheats.on(crate::cheats::slot::MIRROR) { (0.15, -0.4) } else { (-0.4, 0.15) };
    put(w, off(tens), n / 10);
    put(w, off(units), n % 10);
}

/// `0x307578` (module doc; draw only).
pub fn fx_quad_groups(table: &MobyTable, _svc: &Services, id: MobyId) -> Vec<FxQuads> {
    let Some(m) = table.mobys.get(id).filter(|m| m.pvars.len() >= 0x30) else { return Vec::new() };
    let pv = &m.pvars;
    let (scroll, rise, s, yaw) = (p::ff(pv, 0x1c), p::ff(pv, 0x20), p::ff(pv, 0x24), p::ff(pv, 0x2c));
    let (cy, sy) = (yaw.cos(), yaw.sin());
    // Rows of the yaw rotation (0x1fa030): (cos, sin, 0), (−sin, cos, 0), (0, 0, 1); a point x·r0 + y·r1 + z·r2.
    let turn = |x: f32, y: f32| -> (f32, f32) { (x * cy - y * sy, x * sy + y * cy) };
    let at = m.position;
    let mut arrow = Vec::new();
    for q in QUADS {
        let corners = q.map(|i| {
            let (x, y) = turn(0.0, ARROW[i][0]);
            [at[0] + x * s, at[1] + y * s, at[2] + ARROW[i][1]]
        });
        let st = q.map(|i| [ARROW_ST[i][0], ARROW_ST[i][1] + scroll]);
        arrow.push(FxQuad { corners, st, rgba: q.map(colour) });
    }
    let h = rise * 1.2 * s;
    let z = rise * 1.4 + 1.1;
    let local = [[-h, -h, z], [h, -h, z - 0.2], [-h, h, z], [h, h, z - 0.2]];
    let corners = local.map(|[x, y, zz]| {
        let (a, b) = turn(x, y);
        [at[0] + a, at[1] + b, at[2] + zz]
    });
    let rgba = ((((1.0 - rise) * 128.0) as i32 as u32) << 24) | 0x0080_8080;
    let square = FxQuad { corners, st: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], rgba: [rgba; 4] };
    vec![FxQuads { fx: ARROW_FX, additive: false, subtract: false, quads: arrow }, FxQuads { fx: SQUARE_FX, additive: false, subtract: false, quads: vec![square] }]
}
