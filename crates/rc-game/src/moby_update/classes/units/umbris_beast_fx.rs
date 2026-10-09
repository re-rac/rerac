//! **The Snagglebeast's effects** (level07; [`super::umbris_beast`] is the beast): its point light (`0x311eb8` /
//! `0x312150`), the camera shakes (`0x3120d8`), the dust and glow particles (type 61 `0x29b8d8`, type 65 `0x29c4e8`),
//! the shockwave rings **1046** (made by `0x30dbb0`, update `0x30e1f8` with its hit `0x30e0e0` and draw `0x30dcf0`), the
//! spit globs **1049** (made by `0x30e370`, update `0x30e458`), the tongue (`0x317c98` / `0x317cb0` / `0x317e60` /
//! `0x318e98` / `0x318e30`, draw `0x317910`), the beam (`0x3131b0` with its strips `0x312e70`), the shimmer
//! (`0x312dc8` / `0x312948`) and the fire sweep's ground line (`0x312420`, the fire particles 58). Read from the level07
//! decomp and data (the tongue's ring 0x20c700 is a circle of 20: computed). Native `f32`.
//!
//! The draw callbacks draw through [`FxQuads`]; their game-state halves (the beam's hits, the ground line's probes and
//! fire) run in the frame part ([`frame`]). The GS details the quads cannot carry: the tongue's and the shimmer's
//! z-write and alpha-test switches, the shimmer's blend (A 1 B 2 C FIX 0xa0 D 0: 1.25·Cd + Cs, drawn additive) and
//! the beam's second fan's colour (a stack copy taken before the colour is set: drawn in the same colour) [L].
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x311eb8(r, m, P, k, p, ticks, c, c2)` | the light: none and `ticks` ≥ 1 → `WritePointLight_A(r (15), 0, p (the moby), c (0x80004080))`; the same key `k` while lit keeps the longer of its timer and ⅞ of the duration; else the timer / duration = `ticks`; the radius, the colour (+0x288, /128), the second colour (+0x28c); with `p` its position and (the game's) its w as the radius; the key | [`light`] |
//! | `0x312150` | the timer down: out → `FreePointLight`, the key −1; else the colour tweened in (0 → c) over the first eighth, then out (c → c2) | [`light_tick`] |
//! | `0x3120d8` | the shake: 0.1·(80 − d)/80, d the camera's distance (at most 80), 20 ticks | [`shake`] |
//! | `0x30dbb0(r0, r1, h, grow, dmg, m, p, fx)` | `CreateMoby(0x416)`: update / draw 0xff, drawn, at `p`; +0x00 r0, +0x04 r1, +0x08 r0 (the radius), +0x0c grow, +0x10 h, +0x14 the owner, +0x18 the texture, +0x1c the damage; mode \| 0x200; a shake 0.3 − d/100 for `ticks(20)`; sound 0 | [`shockwave`] |
//! | `0x30e1f8` (1046) | radius += grow (the scale too); the alpha (in 0x7f·5 over the first and last fifth, else 0x80); past r1 → deleted; else the draw, and Ratchet's moby and the run list hit (`0x30e0e0`: below 0.3 of the height up, within 0.3 of the ring's edge (xy): `0x26eaa8(dmg, it, ring, 0x10001, edge point, out + up)`) | [`ring_update`] (`attack::hit_moby`) |
//! | `0x30dcf0` | ten bands (0..0.9): the colour (0x80 − 11·band greys, alpha / 5), radius r·band (or r − 0.9·band past 0.9), 20 sectors of two quads: the wall from the inner edge (height (1 − band/2)·h) to the outer and its skirt 5 down, the ST scrolling by band | [`ring_quads`] |
//! | `0x30e370(p, v, m)` | `CreateMoby(0x419)`: update / draw 0xff, drawn, a random Euler, at `p`, +0x00 v, +0x10 the owner, alpha 0x30, mode \| 0x200, a type-26 glow (500000, 0x800040ff, 128) | [`glob`] |
//! | `0x30e458` (1049) | p += v, v.z −= 9.8·dt², yaw + 2π·dt, scale ×1.2 up to 2.8 of the class; every third tick a type-61 at it (400000, 5); the line from the last position (or a 0.1 sphere) meeting nothing of 0x415 / 0x419 / the beast: the explosion (2, 1; 0, 0, 1000, 1.2, 10; 0 streaks, 3 sparks, 5 puffs, sound 0) when drawn, else sound 0; deleted; out of the world → deleted | [`glob_update`] |
//! | `0x317c98` / `0x317cb0` | the tongue reset (fresh, cleared); fresh: along row 0 at 2.8 from the mouth, faded, the wobble from 0 to a random turn of (0, 0, 0.1) about x, the scroll 0.006, the drips cleared | [`tongue_reset`], [`tongue_step`] |
//! | `0x317e60(m, mouth)` | the fades (+4 at the root, −32 a node); the aim (to Ratchet + 0.6, 2.8 long) wobbling over 60 ticks; nine nodes each turned toward it by a spring (0.06, 0.2) with the slack of π/10, the last four eased toward the old; the rings of 20 around each (0.02 + 0.2·i thick, breathing); the nearest point to Ratchet; the drips (type 23 along the tongue, five at a time, two dust puffs) | [`tongue_step`] |
//! | `0x318e98` / `0x318e30` | release: the scroll halved, sound 10, the drips flung off (speed from their node); fade: every node's fade −4 (at most 0x40), the draw while any is left | [`tongue_release`], [`tongue_fade`] |
//! | `0x317910` | the tube: FX 0x15 additive, nine segments of 20 quads, the fades as alpha (0..0x40; the root 0x20, the tip 0), the ST scrolling | [`tongue_quads`] |
//! | `0x3131b0` | the beam from +0x110 at its length (+0x16c; the head at 0.8 of it); three strips (1 / 0.7 / 0.4 wide, colours 0x20000080 / 0x28008080 / 0x30b0ffff); the hit (the line, else a 0.5 sphere at the end; damage 1.000123, flags 0x30000) | [`beam_quads`], [`frame`] |
//! | `0x312dc8` / `0x312948` | the shimmer: a sphere of radius 5.8 round joint list 9 of 16 strips × n quads, each strip starting at the point 5 below it, the arc by the shimmer, FX 0x3c, the colour the flash; ALPHA 0x29 (`Cs + Cd·0xa0/0x80`, the vertex alpha 0) drawn as the additive 0x48 at As 0x80 (`Cs + Cd`: the port's effects have no FIX brightening of the frame) | [`shimmer_quads`] |
//! | `0x312420` | the ground line: the head +0x140 wobbling in x, from joint list 2 30 out, its probe (flags 2) setting the end's height; on the ground a fire (type 58) there and four along from the last end; the quad FX 0x3b from the joint to the end | [`ground_quads`], [`frame`] |

use super::{FxQuad, FxQuads};
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::classes::units::umbris_beast::pv as bp;
use crate::moby_update::creature::{self as c, fx, DT, DT2, SPEED, V};
use crate::moby_update::services::{pf, pv as v4, HitTemplate, Services, World};
use crate::particles::{rec, type23};
use crate::point_lights::PointLight;
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 7;
pub const RING_FN: u32 = 0x30_e1f8;
pub const RING_CLASSES: [i16; 1] = [1046];
pub const RING_DRAW_FN: u32 = 0x30_dcf0;
pub const GLOB_FN: u32 = 0x30_e458;
pub const GLOB_CLASSES: [i16; 1] = [1049];
pub const TONGUE_FN: u32 = 0x31_7910;
pub const BEAM_FN: u32 = 0x31_31b0;
pub const SHIMMER_FN: u32 = 0x31_2dc8;
pub const GROUND_FN: u32 = 0x31_2420;
/// The draw rows ([`quad_groups`]).
pub const DRAW_FNS: [u32; 5] = [TONGUE_FN, BEAM_FN, SHIMMER_FN, GROUND_FN, RING_DRAW_FN];
const RING: i16 = 0x416;
const GLOB: i16 = 0x419;
const BEAST: i16 = 0x452;
const TAU: f32 = std::f32::consts::TAU;
const PI: f32 = std::f32::consts::PI;
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;

/// The light's pvars on the beast.
pub mod pv {
    pub const LIGHT: usize = 0x280;
    pub const LIGHT_KEY: usize = 0x282;
    pub const LIGHT_T: usize = 0x284;
    pub const LIGHT_D: usize = 0x286;
    pub const LIGHT_C: usize = 0x288;
    pub const LIGHT_C2: usize = 0x28c;
}

/// The draw rows: the frame part (game state) and the quads.
pub fn register(w: &mut World, id: MobyId, f: u32) {
    use crate::moby_update::classes::draw_callbacks::Callback;
    let Some(r) = super::row(REFERENCE_LEVEL, f) else { return };
    if f == BEAM_FN || f == GROUND_FN { w.svc.draw_callbacks.register(Callback::UnitFrame(r), id); }
    w.svc.draw_callbacks.register(Callback::UnitQuads(r), id);
}

// -------------------------------------------------------------------------------------------------
// The light, the shakes, the particles

/// `0x311eb8` (module doc). `colour2` −1: kept.
#[allow(clippy::too_many_arguments)]
pub fn light(w: &mut World, id: MobyId, radius: f32, key: i16, at: Option<V>, ticks: i32, colour: u32, colour2: u32) {
    let mut slot = c::pi16(w, id, pv::LIGHT);
    let p = at.unwrap_or_else(|| c::pos(w, id));
    if slot == -1 {
        if ticks < 1 { return; }
        let l = PointLight { color: rgb(colour), intensity: 0.0, pos: [p[0], p[1], p[2]], radius };
        let load = f32::from_bits(w.svc.frame_load[1].0);
        slot = w.svc.point_lights.alloc(l, load).map_or(-1, |i| i as i16);
        c::set_pi16(w, id, pv::LIGHT, slot);
        c::set_pi16(w, id, pv::LIGHT_T, 0);
        if slot == -1 { return; }
    }
    let t = c::pi16(w, id, pv::LIGHT_T);
    if c::pi16(w, id, pv::LIGHT_KEY) == key && key != -1 && t != 0 {
        let v = eighth7(c::pi16(w, id, pv::LIGHT_D));
        if t <= v { c::set_pi16(w, id, pv::LIGHT_T, v); }
    } else {
        c::set_pi16(w, id, pv::LIGHT_D, ticks as i16);
        c::set_pi16(w, id, pv::LIGHT_T, ticks as i16);
    }
    let i = slot as usize;
    let mut l = w.svc.point_lights.slots.get(i).copied().flatten().unwrap_or(PointLight { color: [0.0; 3], intensity: 0.0, pos: [p[0], p[1], p[2]], radius });
    l.radius = radius;
    c::set_pi32(w, id, pv::LIGHT_C, colour as i32);
    l.color = rgb(colour);
    c::set_pi32(w, id, pv::LIGHT_C2, colour2 as i32);
    if let Some(q) = at {
        l.pos = [q[0], q[1], q[2]];
        l.radius = q[3];
    }
    w.svc.point_lights.set(i, l);
    if key != -1 { c::set_pi16(w, id, pv::LIGHT_KEY, key); }
}

fn rgb(c: u32) -> [f32; 3] { [(c & 0xff) as f32 / 128.0, ((c >> 8) & 0xff) as f32 / 128.0, ((c >> 16) & 0xff) as f32 / 128.0] }
/// `(d·7) / 8` rounded toward zero (the game's `>> 3` after the sign fix).
fn eighth7(d: i16) -> i16 { ((d as i32 * 7) / 8) as i16 }

/// `0x312150` (module doc).
pub fn light_tick(w: &mut World, id: MobyId) {
    let slot = c::pi16(w, id, pv::LIGHT);
    let out = c::dec_timer_pvar_s16(w, id, pv::LIGHT_T) != 0;
    if out {
        if slot != -1 {
            w.svc.point_lights.free(slot as usize);
            c::set_pi16(w, id, pv::LIGHT_KEY, -1);
            c::set_pi16(w, id, pv::LIGHT, -1);
        }
        return;
    }
    if slot == -1 { return; }
    let s = c::pi16(w, id, pv::LIGHT_T) as i32;
    let d = c::pi16(w, id, pv::LIGHT_D) as i32;
    let v = eighth7(d as i16) as i32;
    let (f, from, to) = if v < s {
        ((s - v) as f32 / (d - v) as f32, 0, c::pi32(w, id, pv::LIGHT_C) as u32)
    } else {
        (s as f32 / v as f32, c::pi32(w, id, pv::LIGHT_C) as u32, c::pi32(w, id, pv::LIGHT_C2) as u32)
    };
    let col = crate::particles::tween_color((1.0 - f).to_bits(), from, to);
    if let Some(Some(mut l)) = w.svc.point_lights.slots.get(slot as usize).copied() {
        l.color = rgb(col);
        w.svc.point_lights.set(slot as usize, l);
    }
}

/// The light freed (state 0x10).
pub fn light_free(w: &mut World, id: MobyId) {
    let slot = c::pi16(w, id, pv::LIGHT);
    if slot != -1 {
        w.svc.point_lights.free(slot as usize);
        c::set_pi16(w, id, pv::LIGHT, -1);
    }
}

/// `0x3120d8` (module doc).
pub fn shake(w: &mut World, id: MobyId) {
    let cam = w.camera_point();
    let d = c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]).min(80.0);
    let amp = (80.0 - d) / 80.0 * 0.1;
    w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp, ticks: 0x14 });
}

/// `0x29b8d8(size, pull, m, list)`: a type-61 glow on the joint list's point.
pub fn part61(w: &mut World, size: f32, pull: f32, m: MobyId, list: i16) {
    *w.svc.fx.part_spawns.entry(61).or_default() += 1;
    let jp = w.joint_point(m, list as usize);
    let Some(sys) = w.particles.as_deref_mut() else { return };
    if crate::particles::type61::spawn(sys, w.rng, size, pull, m, list, [jp[0], jp[1], jp[2]]).is_none() { w.svc.fx.part_failed += 1; }
}

/// `0x29c4e8(p, v)`: a type-65 dust spark.
pub fn part65(w: &mut World, p: V, v: V) {
    *w.svc.fx.part_spawns.entry(65).or_default() += 1;
    let Some(sys) = w.particles.as_deref_mut() else { return };
    if crate::particles::type25::spawn65(sys, w.rng, p, v).is_none() { w.svc.fx.part_failed += 1; }
}

// -------------------------------------------------------------------------------------------------
// The shockwave rings 1046

/// `0x30dbb0` (module doc).
#[allow(clippy::too_many_arguments)]
pub fn shockwave(w: &mut World, r0: f32, r1: f32, h: f32, grow: f32, dmg: f32, owner: MobyId, at: V, tex: i32) -> Option<MobyId> {
    let m = w.create_moby(RING)?;
    if w.m(m).pvars.len() < 0x20 { w.mm(m).pvars.resize(0x20, 0); }
    {
        let mo = w.mm(m);
        mo.draw_dist = 0xff;
        mo.update_dist = 0xff;
        mo.visible = 1;
        mo.position = at;
        mo.mode |= 0x200;
    }
    c::set_pf(w, m, 0x1c, dmg);
    c::set_pi32(w, m, 0x14, owner as i32 + 1);
    c::set_pf(w, m, 0x04, r1);
    c::set_pf(w, m, 0x10, h);
    c::set_pf(w, m, 0x0c, grow);
    c::set_pf(w, m, 0x08, r0);
    c::set_pi32(w, m, 0x18, tex);
    c::set_pf(w, m, 0x00, r0);
    let cam = w.camera_point();
    let d = c::dist2(at, [cam[0], cam[1], cam[2], 0.0]);
    let t = w.ticks(0x14);
    w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp: 0.3 - d / 100.0, ticks: t });
    w.build_matrix(m);
    w.play_sound(0, 0, m);
    Some(m)
}

/// Level07 `0x30e1f8` (module doc).
pub fn ring_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x20 { return; }
    let r = c::pf(w, id, 8) + c::pf(w, id, 0xc);
    c::set_pf(w, id, 8, r);
    w.mm(id).scale = r;
    let (r0, r1) = (c::pf(w, id, 0), c::pf(w, id, 4));
    let f = (r - r0) / (r1 - r0);
    w.mm(id).alpha = if f < 0.2 {
        (f * 5.0 * 127.0) as i32 as u8
    } else if 0.8 < f {
        ((1.0 - f) * 5.0 * 127.0) as i32 as u8
    } else {
        0x80
    };
    if 1.0 <= f {
        w.delete_moby(id);
        return;
    }
    w.svc.units.umbris_beast.counter = w.counter;
    register(w, id, RING_DRAW_FN);
    if let Some(h) = w.hero_moby { ring_hit(w, h, id); }
    for m in super::hints::run_list(w) { ring_hit(w, m, id); }
}

/// `0x30e0e0(it, ring, P)` (module doc).
fn ring_hit(w: &mut World, it: MobyId, ring: MobyId) {
    let (p, q) = (w.m(it).position, w.m(ring).position);
    if q[2] + c::pf(w, ring, 0x10) * 0.3 <= p[2] { return; }
    let r = c::pf(w, ring, 8);
    if 0.3 <= (c::dist2(q, p) - r).abs() { return; }
    let d = c::sub(p, q);
    let edge = c::set_len3(d, r - 0.05);
    let mut dir = c::set_len3(d, 1.0);
    dir[2] = 1.0;
    let dmg = c::pf(w, ring, 0x1c);
    crate::moby_update::creature::attack::hit_moby(w, it, ring, dmg, 0x1_0001, edge, dir);
}

/// `0x30dcf0` for the ring `id` (module doc).
pub fn ring_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Option<FxQuads> {
    let m = table.mobys.get(id)?;
    if m.pvars.len() < 0x20 { return None; }
    let pvf = |o: usize| crate::moby_update::services::pvar::ff(&m.pvars, o);
    let tex = crate::moby_update::services::pvar::i32(&m.pvars, 0x18).max(0) as usize;
    let (r, h) = (pvf(8), pvf(0x10));
    let p = m.position;
    let a5 = (m.alpha / 5) as u32;
    let t = svc.units.umbris_beast.counter as f32;
    let mut quads = Vec::new();
    let mut grey = 0x80u32;
    let mut band = 0.0f32;
    let mut layer = 0u32;
    while band < 1.0 {
        let rgba = a5 << 24 | grey << 16 | grey << 8 | 0x80;
        let rb = if r < 0.9 { r * band } else { r - band * 0.9 };
        let mut s = (t / (band * 40.0 + 40.0)).fract();
        if layer & 1 == 0 { s = -s; }
        layer += 1;
        let n = layer.min(5) as f32;
        let st = [[s, 0.0], [s + n, 0.0], [s, 1.0], [s + n, 1.0]];
        let top = (1.0 - band * 0.5) * h + p[2];
        let inner_k = 0.96 - band * 0.03;
        let mut in_prev = [p[0] - rb * inner_k, p[1], top];
        let mut out_prev = [p[0] - rb, p[1], p[2]];
        let mut a = -2.827_433_6f32;
        while a < PI {
            let (cs, sn) = (a.cos(), a.sin());
            let inner = [p[0] + rb * inner_k * cs, p[1] + rb * inner_k * sn, top];
            let outer = [p[0] + rb * cs, p[1] + rb * sn, p[2]];
            quads.push(FxQuad { corners: [in_prev, inner, out_prev, outer], st, rgba: [rgba; 4] });
            quads.push(FxQuad { corners: [[in_prev[0], in_prev[1], p[2] - 5.0], [inner[0], inner[1], p[2] - 5.0], out_prev, outer], st, rgba: [rgba; 4] });
            in_prev = inner;
            out_prev = outer;
            a += 0.314_159_27;
        }
        band += 0.1;
        grey = (grey.wrapping_sub(0xb)) & 0xff;
    }
    Some(FxQuads { fx: tex, additive: true, subtract: false, quads })
}

// -------------------------------------------------------------------------------------------------
// The spit globs 1049

/// `0x30e370(p, v, m)` (module doc).
pub fn glob(w: &mut World, p: V, v: V, owner: MobyId) -> Option<MobyId> {
    let m = w.create_moby(GLOB)?;
    if w.m(m).pvars.len() < 0x14 { w.mm(m).pvars.resize(0x14, 0); }
    let (a, b, cc) = (w.rng.rand_angle(), w.rng.rand_angle(), w.rng.rand_angle());
    {
        let mo = w.mm(m);
        mo.draw_dist = 0xff;
        mo.update_dist = 0xff;
        mo.visible = 1;
        mo.rotation = [a, b, cc, 0.0];
        mo.position = p;
        mo.alpha = 0x30;
        mo.mode |= 0x200;
    }
    c::set_pv4(w, m, 0, v);
    c::set_pi32(w, m, 0x10, owner as i32 + 1);
    w.build_matrix(m);
    crate::moby_update::creature::projectile::part26(w, 500_000.0, m, 0x8000_40ff, 0x80);
    Some(m)
}

/// The glob's explosion (`SpawnBeamExplosion(2, 1, 0, 0, 1000, 1.2, 10, m, v, NULL, 0, 3, 5, sound 0, no shake)`).
const GLOB_BEAM: fx::Beam = fx::Beam { damage_r: 2.0, damage: 1.0, flash: 0.0, flash2: 0.0, flash_dist: 1000.0, scale: 1.2, light: 10.0, streaks: 0, sparks: 3, puffs: 5, debris: 1, sound: 0, shake: false };

/// Level07 `0x30e458` (module doc).
pub fn glob_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x14 { return; }
    let from = c::pos(w, id);
    let mut v = c::pv4(w, id, 0);
    let p = c::add(from, v);
    c::set_pos(w, id, p);
    v[2] -= DT2 * 9.8;
    c::set_pv4(w, id, 0, v);
    w.mm(id).rotation[2] += DT * TAU;
    let oc = w.m(id).o_class;
    if w.m(id).scale < super::class_scale(w, oc) * 2.8 { w.mm(id).scale *= 1.2; }
    if w.counter.is_multiple_of(3) { part61(w, 400_000.0, 5.0, id, 0); }
    let owner = usize::try_from(c::pi32(w, id, 0x10) - 1).ok();
    let hit = w.coll_line(v4(from), v4(p), 0, owner).or_else(|| w.coll_sphere(v4(p), pf(0.1), 0, owner));
    if let Some(o) = hit {
        let by = o.moby.map(|m| w.m(m).o_class);
        let ignored = matches!(by, Some(0x415) | Some(GLOB) | Some(BEAST));
        if !ignored {
            if w.m(id).visible != 0 {
                let p = c::pos(w, id);
                fx::beam_explosion(w, &GLOB_BEAM, Some(id), p);
            } else {
                w.play_sound(0, 0, id);
            }
            w.delete_moby(id);
            return;
        }
    }
    if !crate::moby_update::creature::projectile::in_world(c::pos(w, id)) { w.delete_moby(id); }
}

// -------------------------------------------------------------------------------------------------
// The tongue

/// The effects' shared state: the tongue and what the draws read (`units::Globals::umbris_beast`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fx {
    pub tongue: Tongue,
    /// The tick the draws scroll by.
    pub counter: u64,
    /// Joint list 9's point (the shimmer's centre).
    pub shimmer_at: [f32; 3],
    /// The ground line's ends this frame: joint list 2 and the hit point (or the unhit far end).
    pub ground: [V; 2],
    /// The camera's first row (0x167050; the ground line faces it).
    pub cam: [f32; 3],
}

/// One drip riding the tongue (0x20c840 + 0x18·i).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Drip {
    pub node: i16,
    pub k: i16,
    pub prog: f32,
    pub t: f32,
    pub t_speed: f32,
    pub speed: f32,
    /// The type-23 record + 1 (0 none).
    pub part: usize,
}

/// The tongue (level07 0x20daf0.., 0x161b90.., gp−0x5094..−0x507c).
#[derive(Clone, Debug, PartialEq)]
pub struct Tongue {
    pub nodes: [V; 10],
    pub rings: [[V; 20]; 10],
    pub fade: [i16; 10],
    pub bend: [f32; 10],
    pub wob_from: V,
    pub wob_to: V,
    pub wob_angle: f32,
    pub fresh: bool,
    pub clear: bool,
    pub scroll: f32,
    pub scroll2: f32,
    pub scroll_rate: f32,
    pub wob_t: i32,
    pub phase: f32,
    pub drips: Vec<Drip>,
}

impl Default for Tongue {
    fn default() -> Self {
        Tongue { nodes: [[0.0; 4]; 10], rings: [[[0.0; 4]; 20]; 10], fade: [0; 10], bend: [0.0; 10], wob_from: [0.0; 4], wob_to: [0.0; 4], wob_angle: 0.0, fresh: false, clear: false, scroll: 0.0, scroll2: 0.0, scroll_rate: 0.0, wob_t: 0, phase: 0.0, drips: vec![Drip::default(); 200] }
    }
}

/// The ring template 0x20c700: (0, −cos θ, −sin θ, 1), θ = (k + 1)·18°.
fn template(k: usize) -> V {
    let a = (k + 1) as f32 * (PI / 10.0);
    [0.0, -a.cos(), -a.sin(), 1.0]
}

fn rows3(r: &[[f32; 4]; 4], v: V) -> V { std::array::from_fn(|l| if l == 3 { v[3] } else { r[0][l] * v[0] + r[1][l] * v[1] + r[2][l] * v[2] }) }
/// `matrix_mul_vec3(M, v)` for a matrix of three rows.
fn mul3(m: [V; 3], v: V) -> V { std::array::from_fn(|l| if l == 3 { v[3] } else { m[0][l] * v[0] + m[1][l] * v[1] + m[2][l] * v[2] }) }
fn cross(a: V, b: V) -> V { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0], 0.0] }
fn lerp(t: f32, a: V, b: V) -> V { std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t) }
fn rotate(t: f32, v: V, axis: V) -> V {
    let r = crate::moby_update::classes::blaster_shot::rotate([v[0], v[1], v[2]], t, [axis[0], axis[1], axis[2]]);
    [r[0], r[1], r[2], v[3]]
}

/// `0x317c98`: fresh and cleared on the next start.
pub fn tongue_reset(w: &mut World) {
    let t = &mut w.svc.units.umbris_beast.tongue;
    t.clear = true;
    t.fade[0] = 0;
    t.fresh = true;
}

/// `0x317cb0(m, mouth)` (module doc).
fn tongue_start(w: &mut World, id: MobyId, mouth: V) {
    if !w.svc.units.umbris_beast.tongue.fresh { return; }
    let r = w.m(id).rows;
    let a = c::set_len3(r[0], 2.8);
    let ang = w.rng.rand_angle();
    let t = &mut w.svc.units.umbris_beast.tongue;
    t.fresh = false;
    t.nodes[0] = mouth;
    t.fade[0] = 0;
    for i in 0..9 {
        t.nodes[i + 1] = c::add(t.nodes[i], a);
        t.fade[i + 1] = 0;
    }
    t.wob_from = [0.0; 4];
    t.wob_to = rotate(ang, [0.0, 0.0, 0.1, 1.0], [1.0, 0.0, 0.0, 1.0]);
    t.scroll_rate = f32::from_bits(0x3bc4_9ba6);
    t.wob_t = 0;
    t.phase = 0.0;
    t.wob_angle = ang;
    if t.clear {
        t.clear = false;
        for d in &mut t.drips { d.part = 0; }
    }
}

/// `0x29e6d8(0, out, _, c, a, b)`: the distance from `c` to the segment a → b.
fn seg_dist(c: V, a: V, b: V) -> f32 {
    let ab = c::sub(b, a);
    let l2 = c::dot3(ab, ab);
    let t = if l2 == 0.0 { 0.0 } else { -c::dot3(c::sub(a, c), ab) / l2 };
    let q = if t < 0.0 { a } else if t <= 1.0 { c::add(a, c::scale(ab, t)) } else { b };
    let d = c::dist3(q, c);
    if d < 0.0 { 0.0 } else { d }
}

/// `0x317e60(m, mouth)` (module doc): returns the tongue's nearest distance to Ratchet (0.6 up).
pub fn tongue_step(w: &mut World, id: MobyId, mouth: V) -> f32 {
    tongue_start(w, id, mouth);
    let hero = super::hero_pos(w);
    let hc = [hero[0], hero[1], hero[2] + 0.6, hero[3]];
    let r = w.m(id).rows;
    let t60 = w.ticks(0x3c);
    let grav = w.hero.gravity_dir.map(|x| f32::from_bits(x.0));
    // The wobble's next turn draws first (only when its 60 ticks are up).
    let wob_t_next = w.svc.units.umbris_beast.tongue.wob_t + 1;
    let new_turn = if t60 <= wob_t_next {
        let d = w.rng.randf(45.0, 315.0);
        Some(c::add_rot(w.svc.units.umbris_beast.tongue.wob_angle, d * 0.017_453_292))
    } else {
        None
    };
    let t = &mut w.svc.units.umbris_beast.tongue;
    t.fade[0] = t.fade[0].wrapping_add(4);
    for i in 1..10 { t.fade[i] = t.fade[i - 1].wrapping_sub(0x20); }
    let mut dir = c::set_len3(c::sub(hc, mouth), 2.8);
    t.nodes[0] = mouth;
    let wf = c::add(rows3(&r, t.wob_from), dir);
    let wt = c::add(rows3(&r, t.wob_to), dir);
    let f = t.wob_t as f32 / t60 as f32;
    t.wob_t += 1;
    dir = [wf[0] + (wt[0] - wf[0]) * f, wf[1] + (wt[1] - wf[1]) * f, wf[2] + (wt[2] - wf[2]) * f, dir[3]];
    if let Some(a) = new_turn {
        t.wob_from = t.wob_to;
        t.wob_to = rotate(a, [0.0, 0.0, 0.1, 1.0], [1.0, 0.0, 0.0, 1.0]);
        t.wob_t = 0;
        t.wob_angle = a;
    }
    t.phase = c::add_rot(t.phase, 0.1);
    let mut up = c::set_len3(grav, -1.0);
    let cr1 = c::set_len3(cross(dir, up), 1.0);
    let cr2 = cross(up, cr1);
    let m0 = [cr2, cr1, up];
    for k in 0..20 {
        let q = mul3(m0, c::scale(template(k), 0.02));
        t.rings[0][k] = c::add(t.nodes[0], q);
    }
    let mut best = 1000.0f32;
    for i in 1..10 {
        let seg = c::sub(t.nodes[i], t.nodes[i - 1]);
        let axis = cross(dir, seg);
        let a = (c::dot3(seg, dir) / (c::len3(seg) * 2.8)).clamp(-1.0, 1.0).asin();
        let target = HALF_PI - a;
        let mut vel = t.bend[i];
        let mut ang = crate::follow_camera::ang_interp(Pf::ZERO, Pf::f(target), Pf::f(0.06), Pf::f(0.2), Pf::ZERO, &mut { Pf::f(vel) }).to_f32();
        vel = ang;
        t.bend[i] = vel;
        let diff = c::sub_rot(target, ang);
        if 0.314_159_27 < diff.abs() {
            let slack = c::sub_rot(diff, if diff < 0.0 { -0.314_159_27 } else { 0.314_159_27 });
            ang = c::add_rot(ang, slack);
        }
        let mut ns = rotate(ang, seg, axis);
        ns = c::set_len3(ns, 2.8);
        let l = c::len3(ns);
        if 3.9 < l { ns = c::scale(ns, 3.9 / l); }
        if 5 < i { ns = lerp(i as f32 / 10.0, ns, seg); }
        t.nodes[i] = c::add(t.nodes[i - 1], ns);
        let d = seg_dist(hc, t.nodes[i - 1], t.nodes[i]);
        if d < best { best = d; }
        let mut up2;
        if 2.1 < dir[2].abs() {
            up2 = up;
            let c1 = cross(up2, dir);
            let prev2 = if 2 <= i { t.nodes[i - 2] } else { t.nodes[0] };
            let c3 = cross(up2, c::sub(prev2, t.nodes[i - 1]));
            if 0.0 < c::dot3(c1, c3) { up2 = c::scale(up2, -1.0); }
        } else {
            up2 = c::set_len3(grav, -1.0);
        }
        let fwd = c::set_len3(dir, 1.0);
        let r1 = c::set_len3(cross(fwd, up2), 1.0);
        up2 = cross(r1, fwd);
        up = up2;
        let m = [fwd, r1, up2];
        let fi = i as f32;
        for k in 0..20 {
            let rad = fi * 0.2 + 0.02;
            let s = c::add_rot(t.phase + fi * 0.7, 0.0).sin();
            let q = mul3(m, c::scale(template(k), rad + rad * 0.25 * s));
            t.rings[i][k] = c::add(t.nodes[i], q);
        }
        dir = c::set_len3(ns, 2.8);
    }
    register(w, id, TONGUE_FN);
    drips(w, mouth);
    best
}

/// The drips of `0x317e60` (module doc).
fn drips(w: &mut World, mouth: V) {
    let mut spawned = 0;
    let mut dust = 0;
    for di in 0..200 {
        let d = w.svc.units.umbris_beast.tongue.drips[di];
        if d.part == 0 && spawned < 5 {
            let fade = w.svc.units.umbris_beast.tongue.fade;
            let mut k9: i32 = 9;
            if fade[9] < 0x40 {
                k9 = -1;
                for i in (0..9).rev() {
                    if 0x3f < fade[i] {
                        k9 = i as i32;
                        break;
                    }
                }
            }
            if 0 < k9 {
                let k9 = k9 as usize;
                if 1 < k9 && dust < 2 {
                    let rk = w.rng.randi(0x14) as usize;
                    let tt = w.rng.randf(0.2, 1.0);
                    let t = &w.svc.units.umbris_beast.tongue;
                    let a = lerp(tt, t.rings[0][rk], t.rings[k9][rk]);
                    let b = lerp(tt, t.nodes[0], t.nodes[k9]);
                    let size = w.rng.randf(60000.0, 180_000.0);
                    let spin = w.rng.randi(5) + 1;
                    if let Some(i) = puff23(w, [0.0, 1.0, f32::from_bits(0x3f83_d70a), size], a, spin, [0.0; 4], 0x40_4040) {
                        dust += 1;
                        let v = c::set_len3(c::sub(a, b), 0.01);
                        if let Some(sys) = w.particles.as_deref_mut() {
                            let r = &mut sys.pool.recs[i];
                            rec::set_v3(r, 0x30, [v[0], v[1], v[2] + 0.02]);
                        }
                    }
                }
                let at = w.svc.units.umbris_beast.tongue.nodes[k9];
                let size = w.rng.randf(60000.0, 180_000.0);
                let p = puff23(w, [0.0, 1.0, 1.0, size], at, 6, [0.0; 4], 0x7f40_4040);
                if let (Some(i), Some(sys)) = (p, w.particles.as_deref_mut()) {
                    let r = &mut sys.pool.recs[i];
                    rec::set_i16(r, 10, 200);
                    rec::set_u32(r, 0x24, 2);
                    r[0x2a] = 0x7f;
                    r[0x2b] = 200;
                    spawned += 1;
                }
                let k = w.rng.randi(0x14) as i16;
                let ts = w.rng.randf(0.1, 0.8);
                let sp = w.rng.randf(0.1, 0.4);
                let dr = &mut w.svc.units.umbris_beast.tongue.drips[di];
                dr.part = p.map_or(0, |i| i + 1);
                dr.node = k9 as i16;
                dr.prog = 0.0;
                dr.t = 0.0;
                dr.k = k;
                dr.t_speed = ts;
                dr.speed = sp;
            }
        }
        ride(w, di, mouth);
    }
}

/// One drip's ride (module doc).
fn ride(w: &mut World, di: usize, mouth: V) {
    let d = w.svc.units.umbris_beast.tongue.drips[di];
    if d.part == 0 { return; }
    let pi = d.part - 1;
    let (mut node, mut prog) = (d.node.max(1) as usize, d.prog + d.speed);
    let t = w.svc.units.umbris_beast.tongue.clone();
    let mut seg = c::sub(t.nodes[node - 1], t.nodes[node]);
    let mut l = c::len3(seg);
    set_part_timer(w, pi, 200);
    let mut alive = true;
    if l < prog {
        if node <= 1 {
            kill_part(w, pi);
            alive = false;
        } else {
            node -= 1;
            prog -= l;
            seg = c::sub(t.nodes[node - 1], t.nodes[node]);
            l = c::len3(seg);
        }
    }
    let dr = &mut w.svc.units.umbris_beast.tongue.drips[di];
    dr.node = node as i16;
    dr.prog = prog;
    if !alive {
        dr.part = 0;
        return;
    }
    let f = prog / l;
    if node == 2 {
        set_part_timer(w, pi, 100);
    } else if node < 2 {
        let v = (((1.0 - f) * 100.0) as i32).max(0x3c);
        set_part_timer(w, pi, v as i16);
    }
    let Some(sys) = w.particles.as_deref_mut() else { return };
    let r = &mut sys.pool.recs[pi];
    if node == 1 {
        let p = rec::v3(r, 0x10);
        let dv = c::set_len3([mouth[0] - p[0], mouth[1] - p[1], mouth[2] - p[2], 0.0], d.speed);
        rec::set_v3(r, 0x10, [p[0] + dv[0], p[1] + dv[1], p[2] + dv[2]]);
        let s = rec::ff(r, 0xc) * (SPEED * -0.050_000_012 + 1.0);
        rec::set_ff(r, 0xc, s);
        return;
    }
    let base = c::add(t.nodes[node], c::scale(seg, f));
    let k = d.k.clamp(0, 19) as usize;
    let a1 = lerp(f, c::sub(t.rings[node][k], t.nodes[node]), c::sub(t.rings[node - 1][k], t.nodes[node - 1]));
    let k2 = (k + 19) % 20;
    let a2 = lerp(f, c::sub(t.rings[node][k2], t.nodes[node]), c::sub(t.rings[node - 1][k2], t.nodes[node - 1]));
    let o = lerp(d.t, a1, a2);
    rec::set_v3(r, 0x10, [base[0] + o[0], base[1] + o[1], base[2] + o[2]]);
    let dr = &mut w.svc.units.umbris_beast.tongue.drips[di];
    dr.t += dr.t_speed;
    if 1.0 <= dr.t {
        dr.t -= 1.0;
        dr.k = k2 as i16;
    }
}

fn set_part_timer(w: &mut World, i: usize, t: i16) {
    if let Some(sys) = w.particles.as_deref_mut() { rec::set_i16(&mut sys.pool.recs[i], 10, t); }
}
fn kill_part(w: &mut World, i: usize) {
    if let Some(sys) = w.particles.as_deref_mut() { sys.kill_part(i); }
}

/// `PartType23Spawn(jitter, lo, hi, size, p, spin, v, rgba)` keeping its record.
fn puff23(w: &mut World, [j, lo, hi, size]: [f32; 4], p: V, spin: i32, v: V, rgba: u32) -> Option<usize> {
    *w.svc.fx.part_spawns.entry(23).or_default() += 1;
    let sys = w.particles.as_deref_mut()?;
    let r = type23::spawn(sys, w.rng, j, lo, hi, size, p, spin, v, rgba);
    if r.is_none() { w.svc.fx.part_failed += 1; }
    r
}

/// `0x318e98(m)` (module doc).
pub fn tongue_release(w: &mut World, id: MobyId) {
    {
        let t = &mut w.svc.units.umbris_beast.tongue;
        t.scroll_rate *= 0.5;
        t.clear = true;
        t.fresh = true;
    }
    w.play_sound(10, 0, id);
    let t = w.svc.units.umbris_beast.tongue.clone();
    for di in 0..200 {
        let d = t.drips[di];
        if d.part == 0 { continue; }
        w.svc.units.umbris_beast.tongue.drips[di].part = 0;
        let n = d.node.clamp(1, 9) as usize;
        let mut e = c::sub(t.nodes[n - 1], t.nodes[n]);
        let l = c::len3(e);
        if l == 0.0 { continue; }
        e = c::scale(e, d.prog / l);
        let at = c::add(t.nodes[n], e);
        let Some(sys) = w.particles.as_deref_mut() else { continue };
        let r = &mut sys.pool.recs[d.part - 1];
        let p = rec::v3(r, 0x10);
        let off = c::set_len3([p[0] - at[0], p[1] - at[1], p[2] - at[2], 0.0], f32::from_bits(0x3cf5_c28f));
        let e = c::scale(e, 0.1 / l);
        rec::set_v3(r, 0x30, [off[0] + e[0], off[1] + e[1], off[2] + e[2]]);
        rec::set_i16(r, 10, 0x46);
    }
    tongue_fade(w, id);
}

/// `0x318e30(m)` (module doc).
pub fn tongue_fade(w: &mut World, id: MobyId) {
    let t = &mut w.svc.units.umbris_beast.tongue;
    if t.fade[0] <= 0 { return; }
    for f in &mut t.fade {
        if 0x40 < *f { *f = 0x40; }
        *f -= 4;
    }
    register(w, id, TONGUE_FN);
}

/// `0x317910` (module doc).
pub fn tongue_quads(svc: &Services) -> Option<FxQuads> {
    let t = &svc.units.umbris_beast.tongue;
    let mut quads = Vec::new();
    let mut s_col = t.scroll + 0.3;
    let mut s2 = t.scroll2;
    for i in 1..10 {
        let t0 = s2;
        s2 += t.scroll_rate;
        if 7.0 < s2 { s2 -= 7.0; }
        let (sa, sb) = (s_col, s_col - 0.3);
        s_col += 0.3;
        let clamp = |a: i16| -> u32 { a.clamp(0, 0x40) as u32 };
        let mut col = [clamp(t.fade[i]) << 24 | 0x40_4040; 4];
        let prev = clamp(t.fade[i - 1]) << 24 | 0x40_4040;
        col[2] = prev;
        col[3] = prev;
        if i == 1 {
            col[2] = 0x2040_4040;
            col[3] = 0x2040_4040;
        } else if i == 9 {
            col[0] = 0x40_4040;
            col[1] = 0x40_4040;
        }
        let mut tt = t0;
        for k in 0..20 {
            let k1 = (k + 1) % 20;
            let v = |q: V| [q[0], q[1], q[2]];
            let t1 = tt + 0.05;
            quads.push(FxQuad { corners: [v(t.rings[i][k]), v(t.rings[i][k1]), v(t.rings[i - 1][k]), v(t.rings[i - 1][k1])], st: [[sa, tt], [sa, t1], [sb, tt], [sb, t1]], rgba: col });
            tt = t1;
        }
    }
    Some(FxQuads { fx: 0x15, additive: true, subtract: false, quads })
}

// -------------------------------------------------------------------------------------------------
// The beam, the shimmer, the ground line

/// `0x3131b0`'s positions: the end +0x120 at the length, the head +0x140 at 0.8 of it, from +0x110 along +0x130.
fn beam_ends(w: &mut World, id: MobyId) -> (V, V, V) {
    let a = c::pv4(w, id, bp::A);
    let dir = c::pv4(w, id, bp::AB);
    let len = c::pf(w, id, bp::LEN);
    let h = c::add(c::set_len3(dir, len * 0.8), a);
    c::set_pv4(w, id, bp::HEAD, h);
    let b = c::add(c::set_len3(c::sub(h, a), len), a);
    c::set_pv4(w, id, bp::B, b);
    (a, h, b)
}

/// The frame parts (game state, rand) of the beam and the ground line (module doc).
pub fn frame(w: &mut World, f: u32, id: MobyId) {
    if f == BEAM_FN {
        let (a, _, b) = beam_ends(w, id);
        let dir = c::set_len3(c::sub(b, a), 1.0);
        let tmpl = HitTemplate { dir: v4(dir), attacker: Some(id), flags: 0x3_0000, damage: Pf::b(0x3f80_0408), w20: 1, ..Default::default() };
        let line = crate::moby_update::services::line_hit_in(w.table, w.svc, w.classes, w.coll, v4(a), v4(b), 0, Some(id), &tmpl);
        if line.is_none() { w.sphere_mobys(pf(0.5), v4(b), 0, Some(id), Some(&tmpl)); }
    } else if f == GROUND_FN {
        ground_frame(w, id);
    }
}

/// The ground line's end this frame (the head wobbling in x, 30 past it from joint list 2, cut at the hit).
fn ground_end(w: &World, id: MobyId) -> (V, V, bool) {
    let t = w.counter as f32;
    let mut head = c::pv4(w, id, bp::HEAD);
    let a = crate::moby_update::classes::flyer::wrap_frac(t / 11.0);
    let b = crate::moby_update::classes::flyer::wrap_frac(t / 10.0 + a.cos() * HALF_PI);
    head[0] += b.sin();
    let jp = w.joint_point(id, 2);
    let d = c::set_len3(c::sub(head, jp), 30.0);
    let mut far = c::add(head, d);
    let hit = w.coll_line(v4(jp), v4(far), 2, Some(id));
    if let Some(o) = &hit { far = [o.point[0], o.point[1], o.point[2], far[3]]; }
    (jp, far, hit.is_some())
}

/// `0x312420`'s game state: on the ground, the fires (module doc).
fn ground_frame(w: &mut World, id: MobyId) {
    let (jp, far, hit) = ground_end(w, id);
    w.svc.units.umbris_beast.ground = [jp, far];
    if !hit {
        c::set_pf(w, id, bp::HIT_GROUND, 0.0);
        return;
    }
    let (r1, r2, r3) = (w.rng.rand() & 1, w.rng.rand() & 7, w.rng.rand() & 3);
    let (a, b, cc) = (w.ticks(r1 + 0x2d), w.ticks(r2 + 0x37), w.ticks(r3 + 0xd));
    fire58(w, far, 0, a, b, cc, id);
    if c::pf(w, id, bp::HIT_GROUND) != 0.0 {
        let last = c::pv4(w, id, bp::LUNGE);
        let step = c::set_len3(c::sub(far, last), 0.2);
        let mut p = last;
        for i in 0..4 {
            p = c::add(p, step);
            let d = c::set_len3(c::sub(p, jp), 5.0);
            if let Some(o) = w.coll_line(v4(c::sub(p, d)), v4(c::add(p, d)), 2, Some(id)) { p = [o.point[0], o.point[1], o.point[2], p[3]]; }
            if i == 2 {
                let (r1, r2, r3) = (w.rng.rand() & 1, w.rng.rand() & 7, w.rng.rand() & 3);
                let (a, b, cc) = (w.ticks(r1 + 0x2d), w.ticks(r2 + 0x37), w.ticks(r3 + 0xd));
                fire58(w, p, 0, a, b, cc, id);
            } else {
                let (a, b, cc) = (w.ticks(0x2d), w.ticks(0x37), w.ticks(0xd));
                fire58(w, p, 1, a, b, cc, id);
            }
        }
    }
    c::set_pv4(w, id, bp::LUNGE, far);
    c::set_pf(w, id, bp::HIT_GROUND, 1.0);
}

/// `PartType58Spawn(1, 1, p, kind, life, emit, 1, flame, m)` 0x287e70 (`crate::particles::type58`).
fn fire58(w: &mut World, p: V, kind: u8, life: i32, emit: i32, flame: i32, owner: MobyId) {
    *w.svc.fx.part_spawns.entry(58).or_default() += 1;
    let Some(sys) = w.particles.as_deref_mut() else { return };
    let s = crate::particles::type58::Spawn { damage: 1.0, speed: 1.0, pos: p, kind, life: life as u8, emit: emit as u8, count: 1, flame: flame as u8, owner: owner as u32 + 1 };
    if crate::particles::type58::spawn(sys, w.rng, &s).is_none() { w.svc.fx.part_failed += 1; }
}

/// `0x312420`'s quad (module doc): from joint list 2 to the end, a camera-facing 0.25 width.
pub fn ground_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Option<FxQuads> {
    table.mobys.get(id)?;
    let [jp, far] = svc.units.umbris_beast.ground;
    let cam = svc.units.umbris_beast.cam;
    let seg = c::sub(jp, far);
    let side = c::set_len3(cross([cam[0], cam[1], cam[2], 0.0], seg), 0.25);
    let v = |q: V| [q[0], q[1], q[2]];
    let quad = FxQuad { corners: [v(jp), v(c::add(jp, side)), v(far), v(c::add(far, side))], st: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], rgba: [0x8080_8080; 4] };
    Some(FxQuads { fx: 0x3b, additive: true, subtract: false, quads: vec![quad] })
}

/// `0x3131b0` / `0x312e70` (module doc).
pub fn beam_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Option<Vec<FxQuads>> {
    let m = table.mobys.get(id)?;
    let p = |o: usize| crate::moby_update::services::pvar::v4f(&m.pvars, o);
    let sub = crate::moby_update::services::pvar::i32(&m.pvars, bp::SUB);
    if 4 < sub { return Some(Vec::new()); }
    let timer = crate::moby_update::services::pvar::i32(&m.pvars, bp::TIMER);
    let (a, h, b) = (p(bp::A), p(bp::HEAD), p(bp::B));
    let r = m.rows;
    let t20 = svc.ticks(0x14) as f32;
    let mut out = Vec::new();
    for (width, speed, off, colour) in [(1.0f32, 1.0f32, 0u64, 0x2000_0080u32), (0.7, 2.0, 10, 0x2800_8080), (0.4, 3.0, 20, 0x30b0_ffff)] {
        let mut col = colour;
        if sub == 4 {
            let al = (((colour >> 24) as f32) * (timer as f32 / t20)).max(0.0) as u32;
            col = (colour & 0xff_ffff) | al << 24;
        }
        let ph = (svc.units.umbris_beast.counter + off) as f32 * speed * 0.02;
        let st = [[0.0, 0.0], [1.0, 0.0], [0.0, 0.5], [1.0, 0.5]];
        let v = |q: V| [q[0], q[1], q[2]];
        let ring = |x: f32| -> V {
            let ang = crate::moby_update::classes::flyer::wrap_frac(x + ph);
            c::add(h, rows3(&r, [0.0, width * ang.cos(), width * ang.sin(), 0.0]))
        };
        let mut quads = Vec::new();
        let step = std::f32::consts::FRAC_PI_6;
        let mut x = -PI;
        let mut prev = ring(x);
        x += step;
        while x < step + PI {
            let e = ring(x);
            quads.push(FxQuad { corners: [v(a), v(a), v(prev), v(e)], st, rgba: [col; 4] });
            quads.push(FxQuad { corners: [v(b), v(b), v(prev), v(e)], st, rgba: [col; 4] });
            prev = e;
            x += step;
        }
        out.push(FxQuads { fx: 0x3d, additive: true, subtract: false, quads });
    }
    Some(out)
}

/// `0x312dc8` / `0x312948` (module doc).
pub fn shimmer_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Option<FxQuads> {
    let m = table.mobys.get(id)?;
    if m.pvars.len() < bp::SHIMMER + 4 { return None; }
    let sh = crate::moby_update::services::pvar::ff(&m.pvars, bp::SHIMMER);
    let flash = m.pvars[bp::SHIMMER_FLASH] as u32;
    let arc = if 0.8 <= sh { PI } else { ((sh * 3.043_418) / 0.8 + 0.098_174_77).min(PI) };
    let radius = f32::from_bits(0x40b9_999a);
    let col = (flash | flash << 8 | flash << 16) | 0x8040_4040;
    let t = svc.units.umbris_beast.counter as f32;
    let centre = svc.units.umbris_beast.shimmer_at;
    let low = [centre[0], centre[1], centre[2] - (radius - 0.8)];
    let s0 = (t * -0.01).fract();
    let t0 = (t * -0.02).fract();
    let st = [[s0, t0], [s0 + 1.0, t0], [s0, t0 + 1.0], [s0 + 1.0, t0 + 1.0]];
    let step = arc * 0.0625;
    let mut quads = Vec::new();
    let mut a = -2.748_893_7f32;
    while a < PI {
        let mut el = step - HALF_PI;
        let mut prev = (low, low);
        let start = |el: f32| -> ([f32; 3], [f32; 3]) {
            let az = crate::moby_update::classes::flyer::wrap_frac(a + t * 0.02);
            let pa = fx::polar(radius, az, el);
            let pb = fx::polar(radius, c::add_rot(az, std::f32::consts::FRAC_PI_8), el);
            let dz = if 0.0 < el { 0.6 } else { -0.375 };
            ([pa[0] + centre[0], pa[1] + centre[1], pa[2] + dz + centre[2]], [pb[0] + centre[0], pb[1] + centre[1], pb[2] + dz + centre[2]])
        };
        while el <= arc - HALF_PI {
            let cur = start(el);
            quads.push(FxQuad { corners: [prev.0, prev.1, cur.0, cur.1], st, rgba: [col; 4] });
            prev = cur;
            el += step;
        }
        a += std::f32::consts::FRAC_PI_8;
    }
    Some(FxQuads { fx: 0x3c, additive: true, subtract: false, quads })
}

/// The draw rows' quads (`units::fx_quad_groups`).
pub fn quad_groups(table: &MobyTable, svc: &Services, f: u32, id: MobyId) -> Vec<FxQuads> {
    match f {
        TONGUE_FN => tongue_quads(svc).into_iter().collect(),
        BEAM_FN => beam_quads(table, svc, id).unwrap_or_default(),
        SHIMMER_FN => shimmer_quads(table, svc, id).into_iter().collect(),
        GROUND_FN => ground_quads(table, svc, id).into_iter().collect(),
        RING_DRAW_FN => ring_quads(table, svc, id).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// The draws' inputs the update keeps (the tick, joint list 9 for the shimmer, the camera row for the ground line).
pub fn keep_draw_inputs(w: &mut World, id: MobyId) {
    let j9 = w.joint_point(id, 9);
    let r = w.camera_rows[0];
    let f = &mut w.svc.units.umbris_beast;
    f.counter = w.counter;
    f.shimmer_at = [j9[0], j9[1], j9[2]];
    f.cam = r;
}
