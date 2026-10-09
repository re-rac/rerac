//! **Blarg's fire-wave bots, class 1068** (level06 `0x2fdbd0` with its hits `0x2feb40`, its wave `0x2ff100` /
//! `0x2ff2c8` / `0x2ff978` and the wave's draw callback `0x2ff680`; 10 placed, 8 made; level 10 runs the same class
//! at `0x2dabe8`; census U238; the name is descriptive [L]). Big robots that stand and face their target; from afar
//! they slam the ground and send a wall of fire rolling out (16 points over 45°, 6 a second, half as fast for Clank,
//! following the ground, each point dying where it meets a wall), which hurts along its live segments; close by they
//! swing (keys 9–15, swept lines from the weapon joint). Some wait in a cuboid and walk out along a path when Ratchet
//! enters. Alerted, they walk at their target. Two health; knocked back; killed in a blast and three pieces (thrown
//! shards on the way). Read from the level06 decomp and disassembly (the stack arguments); its words gp−0x4c58..
//! −0x4bc0 and the strip's ST 0x1f2e90. Native `f32`.
//!
//! **Pvars** (0x460; the header: damage +0x20, flash +0x110, knockback +0x60, walker +0xc0): +0x120 the target record
//! (+0x160 its moby, here the runtime index + 1; +0x164 its kind), +0x170 home, +0x180 the region, +0x188 the alert,
//! +0x18c the turn velocity, +0x198 the sight, +0x19c the timer, +0x1a4 the walls, +0x1a8 the waiting cuboid, +0x1ac
//! the path out, +0x200 the weapon joint last tick, +0x210 the wave's origin, +0x21c its life, +0x220 the ground's
//! normal, +0x230 its 16 points (w: 1 live, 0.98.. dying, 0 gone), +0x330 (port) the strip's flicker, +0x420 the big
//! head.
//!
//! | state | what | port |
//! |---|---|---|
//! | top | the hits (`0x2feb40`, the wave's tick inside); the big head (2.5, list 1); within 27 of the camera the shadow probe (+0x7f 0x15); anim speed 1 | [`update`] |
//! | 0 | column 1, health 2, +0x30 2, +0x29 1, meter 2; +0x58 12, +0x5a 13; `SeedJumpPattern(J)`, J2 2, J3 0.5, J0 768, J9 7·dt, J1 0.75, J flags \|= 8; home; → 1 | [`init`] |
//! | 1 | a path out: Ratchet in the cuboid → 0xe; else beyond 7 of the target → wait; turn to the target (`0x26ac10`: 2π·dt², π·dt); no target: alerted → 0xb (seq 1); beyond 4 and facing within 10° → 2 (seq 3); within 4 → 5 (seq 6) | |
//! | 2 / 3 / 4 | turn; the end → the wave (`0x2ff100`), 3 (seq 5); 3's end: a target → 4 (seq 4), timer `ticks(90)`; none → 1 (seq 0); 4: turn; the timer out → the wave, 3 (seq 5); within 4 → 5 (seq 6); alerted → 0xb | |
//! | 5 / 0xc | turn (5); keys 9–15: the template (1, flags 1, (cos, sin, 1, 5627.925), type 0 / 1, the class), `0x268b18(joint, pos + 0.3 / 1.5 up, last joint, …, 5)` (= `0x26ebe8`); as Clank before key 9 the anim speed 0.125 (5); the end → 6, +0xbc 1 (seq 2) / → 0xb (seq 1) | [`swing`] (`attack::swept_lines`) |
//! | 6 | the end → +0xbc's state (seq 0) | |
//! | 0xb | `SpringTurn2` at the target, the walker step, 0.75 off the walls; `ClampToPath` within 0.8 or within 4 → 0xc (seq 6); not alerted → 0xd | |
//! | 0xd | home (stepping while facing within 45°); within 1 → 1 (seq 0) | |
//! | 0xe | seq 1; to the path's last point (steps of at most 2); there: turned to the target, within 15° → 1 (seq 0), the path cleared | |
//! | 0xf | the knockback; landed → 1 (seq 0); out of the world → deleted | |
//! | 0x10 | the death flight; down → `SpawnBeamExplosion(0, 0, 2, 1, 9, 1, 15, …, 5, 2, 4, −1, shake, 1)` 1 above, `BreakFxB` 0x6a6 / 0x6a7 / 0x6a8, deleted | |
//! | `0x2feb40` | scale class · 2.5; the wave; the lure → alert; the hit (0x330000) through the resolver (column 4); out5 ≠ 1 outside 0x10: on level 10 a hit by class 0x31a the first time → flag 0x13d419, banner 0x53d6; health − damage; K radius 500, +0x28 0.5; 1 / 2: 0x10, not targetable, keys 9 / 18, K 18·dt / 10.5·dt, seq 9, ten shards from 2.5 up (`0x300c60`, scale ×0.4), flash 0xfa, `SetDeathBits`; 3..10: 0xf, D+6 `ticks(60)`, K 11·dt / 5.5·dt, keys 8 / 15, seq 7, flash 0xfa; the flash; sight 20 (+6 alerted); the target (`0x26eac8` in the region; beyond the sight of home or 3 in height → none; none → Ratchet) | [`hits`] |
//! | `0x2ff100` | `GroundHeight(0.5)`'s normal; the origin at joint list 0 (its z the moby's); 16 points 0.2 out at (i/16 − 0.5)·45° about the yaw and 0.15 up, the ends dead; life `ticks(150)` | [`launch`] |
//! | `0x2ff2c8` | life ticks; at 20 the live points start dying (0.98); each point out 6·dt (3·dt as Clank) from the origin, 2·dt down / up to keep 0.3..0.4 over the ground's plane; a live point whose step hits (flags 2) starts dying; a dying one `Approach`es 0 by 2·dt; segments with both ends past 0.6: `CollLine_Fix(…, 0, m, tmpl)` (damage 1, flags 0x10001 with both ends live, else 0x10000); the draw; the flames | [`wave`] |
//! | `0x2ff978` | per segment with a live end, 1 in 4: a type-2 flame at its first point (the game lerps the point with itself), velocities `randf(0.25, 0.75)` out in two random directions over their phases, up `randf(−0.065, 0.125)`, sizes 0.25 / 0.1, colours 0x800040ff / 0x4000cc80, phases `scale(randf(10, 20))` ×3, def 0x18 | [`flames`] |
//! | `0x2ff680` | FX 0xb (TEST 0x5380b while drawn), blended; per segment two quads: the outer edge (colour from w, 0 → 0x600040ff) and 0.5 in / 0.125 down (0 → 0x6000cccc), then the inner edge 0.25 up; ST s 0.5, t 0.5 / the flicker (`randf(0, 0.2)` a segment, drawn per frame) | [`frame`], [`fx_quads`] |

use super::{FxQuad, FxQuads};
use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, attack, damage, flash, fx, knock, region, target, turn, walker, V};
use crate::moby_update::manip;
use crate::moby_update::services::{pf, pv as v4, pvar as p, HitTemplate, Services, World};
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 6;
pub const UPDATE_FN: u32 = 0x2f_dbd0;
pub const DRAW_FN: u32 = 0x2f_f680;
pub const CLASSES: [i16; 1] = [1068];
pub const PIECES: [i16; 3] = [0x6a6, 0x6a7, 0x6a8];
const SCALE: f32 = 2.5;
const SIGHT: f32 = 20.0;
const NEAR: f32 = 4.0;
const SPEED: f32 = 7.0;
const WAVE_SPEED: f32 = 6.0;
const WAVE_ARC: f32 = 45.0;
const WAVE_LIFE: i32 = 0x96;
const WAIT: i32 = 0x5a;
const CLANK_SWING: f32 = 0.125;
const DEG: f32 = 0.017_453_292;
const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 5, sparks: 2, puffs: 4, debris: 1, sound: -1, shake: true };
const FX: usize = 0xb;
const EDGE: [u32; 2] = [0x6000_40ff, 0x6000_cccc];
const FLAME: (u32, u32) = (0x8000_40ff, 0x4000_cc80);

pub mod pv {
    pub const D: usize = 0x20;
    pub const LURE: usize = 0x38;
    pub const K: usize = 0x60;
    pub const J: usize = 0xc0;
    pub const F: usize = 0x110;
    pub const TGT_REC: usize = 0x120;
    pub const TGT: usize = 0x160;
    pub const KIND: usize = 0x164;
    pub const HOME: usize = 0x170;
    pub const REGION: usize = 0x180;
    pub const ALERT: usize = 0x188;
    pub const TURN_V: usize = 0x18c;
    pub const SIGHT: usize = 0x198;
    pub const TIMER: usize = 0x19c;
    pub const WALLS: usize = 0x1a4;
    pub const WAIT_CUBOID: usize = 0x1a8;
    pub const PATH: usize = 0x1ac;
    pub const LAST_JOINT: usize = 0x200;
    pub const ORIGIN: usize = 0x210;
    pub const LIFE: usize = 0x21c;
    pub const NORMAL: usize = 0x220;
    pub const POINTS: usize = 0x230;
    pub const FLICKER: usize = 0x330;
    pub const BIG_HEAD: usize = 0x420;
    pub const SIZE: usize = 0x460;
}

fn st(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn done(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn blend(w: &mut World, id: MobyId, s: u8, n: i32) {
    let t = w.ticks(n);
    c::blend_to(w, id, s, 0, t);
}
fn tgt(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, pv::TGT) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn tgt_pos(w: &World, id: MobyId) -> V { tgt(w, id).map_or([0.0; 4], |m| w.m(m).position) }
fn heading(a: V, b: V) -> f32 { c::atan(b[0] - a[0], b[1] - a[1]) }
fn no_target(w: &World, id: MobyId) -> bool { c::pi32(w, id, pv::KIND) == 2 }
fn alerted(w: &World, id: MobyId) -> bool { c::pi32(w, id, pv::ALERT) != 0 }
fn face(w: &mut World, id: MobyId) {
    let h = heading(c::pos(w, id), tgt_pos(w, id));
    turn::turn_toward_pvar(w, id, h, c::DT2 * std::f32::consts::TAU, c::DT2 * std::f32::consts::TAU, c::DT * std::f32::consts::PI, pv::TURN_V);
}
fn spring2(w: &mut World, id: MobyId, h: f32, acc: f32, vmax: f32) { turn::spring_turn2_pvar(w, id, h, acc, f32::from_bits(0x3e99_999a), vmax, pv::TURN_V); }
fn step(w: &mut World, id: MobyId, dir: V) {
    let mut out = [0.0; 4];
    walker::step(w, id, pv::J, 1.0, dir, &mut out);
}
fn forward(w: &World, id: MobyId) -> V { let y = c::yaw(w, id); [y.cos() * 2.0, y.sin() * 2.0, 0.0, 0.0] }
fn walls(w: &mut World, id: MobyId) {
    let Some(pts) = usize::try_from(c::pi32(w, id, pv::WALLS)).ok().and_then(|p| w.svc.splines.get(p)).cloned() else { return };
    if let Some(q) = crate::path::push_from_walls(&pts, 0.75, c::pos(w, id)) { c::set_pos(w, id, q); }
}
fn point(w: &World, id: MobyId, i: usize) -> V { c::pv4(w, id, pv::POINTS + 0x10 * i) }

/// `0x2ff100(m)` (module doc).
fn launch(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    let n = match w.coll_line(v4([p[0], p[1], p[2] + 0.5, p[3]]), v4([p[0], p[1], 0.01, p[3]]), 2, None) {
        Some(o) => c::set_len3([o.normal[0], o.normal[1], o.normal[2], 0.0], 1.0),
        None => [0.0, 0.0, 1.0, 0.0],
    };
    c::set_pv4(w, id, pv::NORMAL, n);
    let mut o = w.joint_point(id, 0);
    o[2] = p[2];
    c::set_pv4(w, id, pv::ORIGIN, o);
    let yaw = c::yaw(w, id);
    for i in 0..16 {
        let a = c::add_rot((i as f32 * 0.0625 - 0.5) * WAVE_ARC * DEG, yaw);
        let live = if i == 0 || i == 15 { 0.0 } else { 1.0 };
        c::set_pv4(w, id, pv::POINTS + 0x10 * i, [o[0] + a.cos() * 0.2, o[1] + a.sin() * 0.2, o[2] + 0.15, live]);
    }
    let t = w.ticks(WAVE_LIFE) as f32;
    c::set_pf(w, id, pv::LIFE, t);
}

/// `0x2ff2c8(m)` (module doc).
fn wave(w: &mut World, id: MobyId) {
    let mut life = c::pf(w, id, pv::LIFE) as i32;
    c::dec_timer_i32(&mut life);
    c::set_pf(w, id, pv::LIFE, life as f32);
    if life == 0 { return; }
    if life == 0x14 {
        for i in 0..16 {
            if 1.0 <= point(w, id, i)[3] { c::set_pf(w, id, pv::POINTS + 0x10 * i + 12, f32::from_bits(0x3f7a_e148)); }
        }
    }
    let (o, n, me) = (c::pv4(w, id, pv::ORIGIN), c::pv4(w, id, pv::NORMAL), c::pos(w, id));
    for i in 0..16 {
        let p0 = point(w, id, i);
        let sp = if w.body() == 1 { WAVE_SPEED * c::DT * 0.5 } else { WAVE_SPEED * c::DT };
        let out = c::set_len3([p0[0] - o[0], p0[1] - o[1], 0.0, 0.0], sp);
        let mut q = [p0[0] + out[0], p0[1] + out[1], p0[2], p0[3]];
        let d = c::dot3(c::sub(q, me), n);
        if 0.4 < d { q[2] -= c::DT + c::DT; } else if d < 0.3 { q[2] += c::DT + c::DT; }
        if 1.0 <= p0[3] && w.coll_line(v4(p0), v4(q), 2, None).is_some() {
            q[3] = 0.98;
        } else if q[3] < 1.0 {
            turn::approach(0.0, c::DT + c::DT, &mut q[3]);
        }
        c::set_pv4(w, id, pv::POINTS + 0x10 * i, q);
    }
    let yaw = c::yaw(w, id);
    for i in 0..15 {
        let (a, b) = (point(w, id, i), point(w, id, i + 1));
        if a[3] < 0.6 || b[3] < 0.6 { continue; }
        let flags = if 1.0 <= a[3] && 1.0 <= b[3] { 0x1_0001 } else { 0x1_0000 };
        let o_class = w.m(id).o_class;
        let t = HitTemplate { dir: [pf(yaw.cos()), pf(yaw.sin()), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags, b18: 0, b19: 1, h1a: o_class as u16, damage: Pf::ONE, w20: 1 };
        crate::moby_update::services::line_hit_in(w.table, w.svc, w.classes, w.coll, v4(a), v4(b), 0, Some(id), &t);
    }
    if let Some(r) = super::row(REFERENCE_LEVEL, DRAW_FN) {
        w.svc.draw_callbacks.register(Callback::UnitFrame(r), id);
        w.svc.draw_callbacks.register(Callback::UnitQuads(r), id);
    }
    flames(w, id);
}

/// `0x2ff978(m)` (module doc).
fn flames(w: &mut World, id: MobyId) {
    for i in 0..15 {
        let (a, b) = (point(w, id, i), point(w, id, i + 1));
        if a[3] < 0.99 && b[3] < 0.99 { continue; }
        if 0.25 < w.rng.randf(0.0, 1.0) { continue; }
        let (ra, rb) = (w.rng.rand_angle(), w.rng.rand_angle());
        let mut v1 = [0.0f32; 4];
        let mut v2 = [0.0f32; 4];
        v1[0] = ra.cos() * w.rng.randf(0.25, 0.75);
        v1[1] = ra.sin() * w.rng.randf(0.25, 0.75);
        v2[0] = rb.cos() * w.rng.randf(0.25, 0.75);
        v2[1] = rb.sin() * w.rng.randf(0.25, 0.75);
        v1[2] = w.rng.randf(-0.065, 0.125);
        v2[2] = w.rng.randf(-0.065, 0.125);
        let ph = |w: &mut World| { let f = w.rng.randf(10.0, 20.0); w.svc.timing.scale(Pf::f(f)).to_f32() as i32 };
        let (t0, t1, t2) = (ph(w), ph(w), ph(w));
        let v1 = [v1[0] / t0 as f32, v1[1] / t0 as f32, v1[2] / t0 as f32, 0.25];
        let v2 = [v2[0] / t2 as f32, v2[1] / t2 as f32, v2[2] / t2 as f32, 0.1];
        w.rng.randf(0.0, 1.0);
        let s = crate::particles::type02::Spawn { pos: a, v1, v2, c1: FLAME.0, c2: FLAME.1, t: [t0, t1, t2], def: 0x18 };
        fx::part02(w, &s);
    }
}

/// `0x2ff680`'s draws of the flicker (module doc): one `randf(0, 0.2)` a segment, each segment's second t the last's.
pub fn frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { return; }
    let last = c::pf(w, id, pv::FLICKER + 4 * 15);
    c::set_pf(w, id, pv::FLICKER, last);
    for k in 0..15 {
        let r = w.rng.randf(0.0, 0.2);
        c::set_pf(w, id, pv::FLICKER + 4 * (k + 1), r);
    }
}

/// `0x2ff680` (module doc; draw only).
pub fn fx_quads(table: &MobyTable, _svc: &Services, id: MobyId) -> Option<FxQuads> {
    let m = table.mobys.get(id)?;
    if m.pvars.len() < pv::SIZE { return None; }
    let pv = &m.pvars;
    let o = p::v4f(pv, pv::ORIGIN);
    let pt = |i: usize| p::v4f(pv, pv::POINTS + 0x10 * i);
    let mut quads = Vec::new();
    for k in 0..15 {
        let (t_prev, t_new) = (p::ff(pv, pv::FLICKER + 4 * k), p::ff(pv, pv::FLICKER + 4 * (k + 1)));
        let mut corners = [[0.0f32; 3]; 4];
        let mut rgba = [0u32; 4];
        for j in 0..4 {
            let q = pt(k + j / 2);
            let w0 = q[3];
            if j & 1 == 0 {
                corners[j] = [q[0], q[1], q[2]];
                rgba[j] = crate::particles::tween_color(w0.to_bits(), 0, EDGE[0]);
            } else {
                let d = c::set_len3([q[0] - o[0], q[1] - o[1], q[2] - o[2], 0.0], 0.5);
                corners[j] = [q[0] - d[0], q[1] - d[1], q[2] - d[2] - 0.125];
                rgba[j] = crate::particles::tween_color(w0.to_bits(), 0, EDGE[1]);
            }
        }
        let st = [[0.5, 0.5], [0.5, t_prev], [0.5, 0.5], [0.5, t_new]];
        quads.push(FxQuad { corners, st, rgba });
        let mut up = corners;
        up[1][2] += 0.25;
        up[3][2] += 0.25;
        quads.push(FxQuad { corners: up, st, rgba });
    }
    Some(FxQuads { fx: FX, additive: false, subtract: false, quads })
}

/// States 5 / 0xc (module doc).
fn swing(w: &mut World, id: MobyId, up: f32, clank_slow: bool) {
    let j = w.joint_point(id, 0);
    let p0 = c::pos(w, id);
    let b = [p0[0], p0[1], p0[2] + up, p0[3]];
    let key = c::ground::key_time(w, id);
    if clank_slow && w.body() == 1 && key < 9.0 { w.mm(id).anim.speed = CLANK_SWING; }
    if (9.0..=15.0).contains(&key) {
        let yaw = c::yaw(w, id);
        let o_class = w.m(id).o_class;
        let t = HitTemplate { dir: [pf(yaw.cos()), pf(yaw.sin()), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 1, b18: 0, b19: 1, h1a: o_class as u16, damage: Pf::ONE, w20: 1 };
        let prev = c::pv4(w, id, pv::LAST_JOINT);
        attack::swept_lines(w, j, b, prev, b, id, &t, 5);
    }
    c::set_pv4(w, id, pv::LAST_JOINT, j);
}

/// `0x2feb40` (module doc).
fn hits(w: &mut World, id: MobyId) {
    let o = w.m(id).o_class;
    w.mm(id).scale = super::class_scale(w, o) * SCALE;
    wave(w, id);
    if c::pi32(w, id, pv::LURE) != 0 {
        let f = w.rng.randf(180.0, 240.0);
        let t = w.svc.timing.scale(Pf::f(f)).to_f32() as i32;
        c::set_pi32(w, id, pv::ALERT, t);
        c::set_pi32(w, id, pv::LURE, 0);
    }
    c::dec_timer_pvar_i32(w, id, pv::ALERT);
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    if res.out5 != 1 && st(w, id) != 0x10 {
        if w.svc.level == 10 && hit.and_then(|h| h.attacker).is_some_and(|a| w.m(a).o_class == 0x31a) {
            // Skill point 0x13d419 (the skill-point table 0x13d408, not the global flags).
            let k = crate::moby_update::story::skill_index(0x13_d419);
            if !crate::moby_update::story::skill_point(w, k) {
                crate::moby_update::story::set_skill_point(w, k);
                crate::cinematic::show_banner(w, 0x53d6, -1);
            }
        }
        let hp = c::pf(w, id, pv::D) - res.damage;
        c::set_pf(w, id, pv::D, hp);
        let reaction = if hp <= 0.0 { 1 } else { res.reaction };
        c::set_pi32(w, id, pv::K + knock::k::RADIUS, 500);
        c::set_pf(w, id, pv::K + knock::k::ZOFF, 0.5);
        let dir = res.hit.map_or([0.0; 4], |h| h.dir.map(|x| f32::from_bits(x.0)));
        let a = c::atan(dir[0], dir[1]);
        match reaction {
            1 | 2 => {
                set(w, id, 0x10);
                w.mm(id).mode &= !mode::TARGETABLE;
                c::set_pf(w, id, pv::K + knock::k::KEY_APEX, 9.0);
                c::set_pf(w, id, pv::K + knock::k::KEY_LAND, 18.0);
                c::set_pf(w, id, pv::K + knock::k::UP, 10.5 * c::DT);
                c::set_pf(w, id, pv::K + knock::k::SPEED, 18.0 * c::DT);
                knock::start(w, id, pv::K, a, 9, 1, 0);
                for _ in 0..10 {
                    let mut at = c::pos(w, id);
                    at[2] += 2.5;
                    for q in at.iter_mut().take(3) { *q += w.rng.randf_sym(0.0, 0.2); }
                    let ra = w.rng.rand_angle();
                    let s = w.rng.randf(3.0, 5.7) * c::DT;
                    let vz = w.rng.randf(3.0, 8.5) * c::DT;
                    let v = [ra.cos() * s + a.cos() * c::DT * 7.0, ra.sin() * s + a.sin() * c::DT * 7.0, vz, 0.0];
                    super::blarg_glass::shard_from(w, at, v, 0.4);
                }
                c::set_pu8(w, id, pv::F + 7, 0xfa);
                set_death_bits(w, id, 0, -1);
            }
            3..=10 => {
                set(w, id, 0xf);
                let t = w.ticks(0x3c);
                c::set_pi16(w, id, pv::D + 6, t as i16);
                c::set_pf(w, id, pv::K + knock::k::SPEED, 11.0 * c::DT);
                c::set_pf(w, id, pv::K + knock::k::KEY_APEX, 8.0);
                c::set_pf(w, id, pv::K + knock::k::KEY_LAND, 15.0);
                c::set_pf(w, id, pv::K + knock::k::UP, 5.5 * c::DT);
                knock::start(w, id, pv::K, a, 7, 1, 0);
                c::set_pu8(w, id, pv::F + 7, 0xfa);
            }
            _ => {}
        }
        flash::start(w, id, pv::F);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pv::F);
    let sight = SIGHT + if alerted(w, id) { 6.0 } else { 0.0 };
    c::set_pf(w, id, pv::SIGHT, sight);
    let reg = usize::try_from(c::pi32(w, id, pv::REGION)).ok();
    let t = target::acquire_in(w, id, sight, reg);
    c::set_pv4(w, id, pv::TGT_REC, t.pos);
    c::set_pi32(w, id, pv::TGT, t.moby.map_or(0, |m| m as i32 + 1));
    let mut kind = t.kind as i32;
    if kind != 2 && (sight < c::dist2(c::pv4(w, id, pv::HOME), t.pos) || 3.0 < (c::pos(w, id)[2] - t.pos[2]).abs()) { kind = 2; }
    c::set_pi32(w, id, pv::KIND, kind);
    if c::pi32(w, id, pv::TGT) == 0 {
        let h = w.hero_moby.map_or(0, |m| m as i32 + 1);
        c::set_pi32(w, id, pv::TGT, h);
    }
}

/// State 0 (module doc).
fn init(w: &mut World, id: MobyId) {
    c::set_pu8(w, id, pv::D + 8, 1);
    c::set_pf(w, id, pv::D, 2.0);
    c::set_pf(w, id, pv::D + 0x10, 2.0);
    c::set_pu8(w, id, pv::D + 9, 1);
    c::set_pi16(w, id, pv::D + 4, 2);
    c::set_pu8(w, id, 0x58, 12);
    c::set_pu8(w, id, 0x5a, f32::from_bits(0x4159_999a) as u8);
    walker::seed(&mut w.mm(id).pvars, pv::J);
    c::set_pf(w, id, pv::J + 8, 2.0);
    c::set_pf(w, id, pv::J + 0xc, 0.5);
    c::set_pi32(w, id, pv::J, (SCALE * 307.2) as i32);
    c::set_pf(w, id, pv::J + 0x24, SPEED * c::DT);
    c::set_pf(w, id, pv::J + 4, SCALE * 0.3);
    let fl = c::pi32(w, id, pv::J + 0x38) | 8;
    c::set_pi32(w, id, pv::J + 0x38, fl);
    let p = c::pos(w, id);
    c::set_pv4(w, id, pv::HOME, p);
    set(w, id, 1);
}

/// Level06 `0x2fdbd0` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { w.mm(id).pvars.resize(pv::SIZE, 0); }
    hits(w, id);
    manip::big_head(w, SCALE, id, 1, id, pv::BIG_HEAD);
    if w.m(id).visible != 0 {
        let cam = w.camera.map(|x| f32::from_bits(x.0));
        if c::dist3(c::pos(w, id), cam) < 27.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0x15;
        }
    }
    w.mm(id).anim.speed = 1.0;
    match st(w, id) {
        0 => init(w, id),
        1 => {
            if c::pi32(w, id, pv::PATH) != -1 {
                let cub = c::pi32(w, id, pv::WAIT_CUBOID);
                if cub != -1 {
                    if w.in_cuboid(w.hero_point(), cub) {
                        set(w, id, 0xe);
                        return;
                    }
                    if 7.0 < c::dist2(c::pos(w, id), tgt_pos(w, id)) { return; }
                }
            }
            face(w, id);
            if no_target(w, id) {
                if alerted(w, id) {
                    set(w, id, 0xb);
                    blend(w, id, 1, 20);
                }
                return;
            }
            let (me, t) = (c::pos(w, id), tgt_pos(w, id));
            if NEAR < c::dist2(me, t) {
                if c::diff_rots(c::yaw(w, id), heading(me, t)) < 10.0 * DEG {
                    set(w, id, 2);
                    blend(w, id, 3, 10);
                }
            } else {
                set(w, id, 5);
                blend(w, id, 6, 10);
            }
        }
        2 => {
            face(w, id);
            if !done(w, id) { return; }
            launch(w, id);
            set(w, id, 3);
            blend(w, id, 5, 10);
        }
        3 => {
            if !done(w, id) { return; }
            if !no_target(w, id) {
                set(w, id, 4);
                blend(w, id, 4, 20);
                let t = w.ticks(WAIT);
                c::set_pi32(w, id, pv::TIMER, t);
            } else {
                set(w, id, 1);
                blend(w, id, 0, 20);
            }
        }
        4 => {
            face(w, id);
            if c::dec_timer_pvar_i32(w, id, pv::TIMER) == 0 {
                if c::dist2(c::pos(w, id), tgt_pos(w, id)) < NEAR {
                    set(w, id, 5);
                    blend(w, id, 6, 10);
                } else if alerted(w, id) {
                    set(w, id, 0xb);
                    blend(w, id, 1, 20);
                }
            } else {
                launch(w, id);
                set(w, id, 3);
                blend(w, id, 5, 5);
            }
        }
        5 => {
            face(w, id);
            swing(w, id, 0.3, true);
            if done(w, id) {
                set(w, id, 6);
                w.mm(id).cmd = 1;
                blend(w, id, 2, 10);
            }
        }
        6 => {
            if done(w, id) {
                let back = w.m(id).cmd;
                set(w, id, back);
                blend(w, id, 0, 20);
            }
        }
        0xb => {
            let t = tgt_pos(w, id);
            spring2(w, id, heading(c::pos(w, id), t), f32::from_bits(0x3d4c_cccd), c::DT * std::f32::consts::TAU);
            let f = forward(w, id);
            step(w, id, f);
            walls(w, id);
            let me = c::pos(w, id);
            let wl = usize::try_from(c::pi32(w, id, pv::WALLS)).unwrap_or(0);
            let (crossed, at) = region::clamp(w, wl, me, t);
            let wall = crossed && c::dist3(me, at) < 0.8;
            if !alerted(w, id) {
                set(w, id, 0xd);
            } else if wall || c::dist2(me, t) < NEAR {
                set(w, id, 0xc);
                blend(w, id, 6, 10);
            }
        }
        0xc => {
            swing(w, id, 1.5, false);
            if done(w, id) {
                set(w, id, 0xb);
                blend(w, id, 1, 20);
            }
        }
        0xd => {
            let home = c::pv4(w, id, pv::HOME);
            let h = heading(c::pos(w, id), home);
            spring2(w, id, h, f32::from_bits(0x3d4c_cccd), c::DT * std::f32::consts::TAU);
            if c::diff_rots(c::yaw(w, id), h) < std::f32::consts::FRAC_PI_4 {
                let f = forward(w, id);
                step(w, id, f);
                walls(w, id);
            }
            if c::dist3(home, c::pos(w, id)) < 1.0 {
                set(w, id, 1);
                blend(w, id, 0, 20);
            }
        }
        0xe => {
            if w.m(id).anim.seq_b != 1 { blend(w, id, 1, 12); }
            let pts: Vec<V> = usize::try_from(c::pi32(w, id, pv::PATH)).ok().and_then(|p| w.svc.splines.get(p)).map_or(Vec::new(), |s| s.iter().map(|q| q.map(f32::from_bits)).collect());
            let Some(&end) = pts.last() else { return };
            let d = c::dist2(c::pos(w, id), end);
            let rate = c::DT * f32::from_bits(0x40b2_b8c2);
            if 0.25 <= d {
                spring2(w, id, heading(c::pos(w, id), end), f32::from_bits(0x3ccc_cccd), rate);
                let y = c::yaw(w, id);
                let r = d.min(2.0);
                step(w, id, [y.cos() * r, y.sin() * r, 0.0, 0.0]);
                return;
            }
            let h = heading(c::pos(w, id), tgt_pos(w, id));
            spring2(w, id, h, f32::from_bits(0x3d4c_cccd), rate);
            if c::diff_rots(h, c::yaw(w, id)) < 0.261_799_4 {
                c::set_pf(w, id, pv::TURN_V, 0.0);
                c::set_pi32(w, id, pv::PATH, -1);
                set(w, id, 1);
                blend(w, id, 0, 18);
            }
        }
        0xf => {
            let r = knock::update(w, id, pv::K);
            if r & 0x40 != 0 {
                set(w, id, 1);
                blend(w, id, 0, 10);
            } else if r & 0x120 != 0 {
                w.delete_moby(id);
            }
        }
        0x10 if knock::update(w, id, pv::K) & 0x160 != 0 => {
            let mut p = c::pos(w, id);
            p[2] += 1.0;
            fx::beam_explosion(w, &BLAST, Some(id), p);
            let (pos, rot) = (c::pos(w, id), w.m(id).rotation);
            for class in PIECES { fx::break_piece(w, id, class, pos, rot, 0, 0); }
            w.delete_moby(id);
        }
        _ => {}
    }
}
