//! **Gemlik's stomper robots, class 1262** (level13 `0x307e98`, its tick `0x308d10`; census U475; 16 placed). Big
//! walkers (scale 2.5) in an area (+0x180) behind walls (+0x198). Seeing their target (within +0x19c of home, +0x1a0
//! for `scale(randf(180, 240))` ticks after a lure; within 3 in z; past a trigger cuboid +0x1a4 once) they turn to it:
//! beyond 4 and facing it they stomp (sequence 3, then 5), sending a shockwave ring along the ground (16 points over
//! 45° from joint 0 rolling out at 6 a second, hurting along the ring, fading after `ticks(130)`), and keep stomping
//! every `ticks(90)` while it stays within 15; not facing it they walk at it (sidestepping 63° when a ledge blocks);
//! within 4 they swing (sequence 6: lines swept from joint 0 to the body, frames 9..15). Alerted without a target they
//! roam and swing; calm again they walk home. Two health; hits knock them back (11·dt / 5.5·dt, sequence 7); dead they
//! fly (19·dt / 5.7·dt, sequence 8) and burst. Hurtful ground (surfaces 0, 1, 3, 0xb, 0xd) or leaving 2..1021 kills
//! them; they slide off slopes steeper than 40° and fall at 9.8·dt².
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x307e98` | the states (module doc) | [`update`] |
//! | `0x308d10` | the tick: the scale, the ring, the lure, the hits (0x330000; its own class: no damage), the target, gravity and ground | [`tick`] |
//! | `0x309370` | the ring's start: the ground normal, the centre at joint 0, 16 points 0.5 out across 45° (the end ones faded), `ticks(150)` | [`ring_start`] |
//! | `0x3094f8` | the ring: rolling out, kept 0.35 over the ground, fading from `ticks(20)` left or where a wall cuts it; each live segment a line hit (1, flags 0x10001, b19 1, the class); the draw; the sparks | [`ring`] |
//! | `0x309bc8` | the sparks: on live segments one in four a tick, a type-2 spark | [`ring_sparks`] |
//! | `0x3098d0` | the ring's draw: per segment two additive quads (FX 0xb) rising from 0.5 inside it, the colours by the points' fade, the ST t from the shared table 0x1d9b70 whose last row is re-rolled `randf(0, 0.2)` a segment | [`frame`], [`fx_quads`] |
//!
//! Read from the level13 decomp and disassembly (the ring's probes, the swing's ends). [L] The draw's TEST change (an
//! alpha reference of 0x80 for the z write) has no counterpart: the effect quads write no z. Native `f32`.

use super::{FxQuad, FxQuads};
use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, damage, flash, fx, knock, region, target, turn, walker};
use crate::moby_update::services::{line_hit_in, pv, pvar as p, HitTemplate, Services, World};
use crate::moby_update::story;
use crate::ps2v::Pf;
use std::f32::consts::{PI, TAU};

pub const REFERENCE_LEVEL: u32 = 13;
pub const UPDATE_FN: u32 = 0x30_7e98;
pub const DRAW_FN: u32 = 0x30_98d0;
pub const CLASSES: [i16; 1] = [1262];

const DT: f32 = c::DT;
const DT2: f32 = c::DT2;
const SCALE: f32 = 2.5;
/// The shared ST row (0x1d9b8c, f32 bits in a unit word).
const ST_ROW: u32 = 0x1d_9b8c;

mod pv_ {
    pub const D: usize = 0x20;
    pub const COOLDOWN: usize = 0x26;
    pub const LURE: usize = 0x38;
    pub const K: usize = 0x60;
    pub const J: usize = 0xc0;
    pub const F: usize = 0x110;
    pub const T: usize = 0x120;
    pub const T_MOBY: usize = 0x160;
    pub const T_KIND: usize = 0x164;
    pub const HOME: usize = 0x170;
    pub const AREA: usize = 0x180;
    pub const ALERT: usize = 0x184;
    pub const TURN_V: usize = 0x188;
    pub const RANGE: usize = 0x190;
    pub const WAIT: usize = 0x194;
    pub const WALLS: usize = 0x198;
    pub const RANGE_A: usize = 0x19c;
    pub const RANGE_B: usize = 0x1a0;
    pub const TRIGGER: usize = 0x1a4;
    pub const SIDE: usize = 0x1a8;
    pub const SWING: usize = 0x1e0;
    pub const CENTRE: usize = 0x1f0;
    pub const RING_T: usize = 0x1fc;
    pub const NORMAL: usize = 0x200;
    pub const RING: usize = 0x210;
    /// The port's: the draw's 15 (last, new) ST rows.
    pub const ST: usize = 0x310;
    pub const HEAD: usize = 0x400;
    pub const SIZE: usize = 0x440;
}
use pv_ as o;

const DEATH: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 10, sparks: 3, puffs: 0x10, debris: 0, sound: -1, shake: true };

fn tmoby(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o::T_MOBY) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn tpos(w: &World, id: MobyId) -> c::V { tmoby(w, id).map_or([0.0; 4], |m| w.m(m).position) }
fn kind(w: &World, id: MobyId) -> i32 { c::pi32(w, id, o::T_KIND) }
fn done(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn blend(w: &mut World, id: MobyId, seq: u8, n: i32) {
    if w.m(id).anim.seq_b != seq {
        let t = w.ticks(n);
        w.anim_blend(id, seq, 0, t);
    }
}
fn heading(w: &World, id: MobyId, at: c::V) -> f32 { let p = c::pos(w, id); c::atan(at[0] - p[0], at[1] - p[1]) }
fn turn_to_target(w: &mut World, id: MobyId) {
    let h = heading(w, id, tpos(w, id));
    turn::turn_toward_pvar(w, id, h, DT2 * TAU, DT2 * TAU, DT * PI, o::TURN_V);
}
fn ring_pt(w: &World, id: MobyId, j: usize) -> c::V { c::pv4(w, id, o::RING + 0x10 * j) }

/// `0x309370(m)`: the ring's start (module doc).
fn ring_start(w: &mut World, id: MobyId) {
    let pos = c::pos(w, id);
    let a = [pos[0], pos[1], pos[2] + 0.5, pos[3]];
    let b = [pos[0], pos[1], 0.01, pos[3]];
    let n = w.coll_line(pv(a), pv(b), 2, None).map_or([0.0; 4], |h| [h.normal[0], h.normal[1], h.normal[2], 0.0]);
    c::set_pv4(w, id, o::NORMAL, c::set_len3(n, 1.0));
    let centre = w.joint_point(id, 0);
    c::set_pv4(w, id, o::CENTRE, centre);
    let yaw = c::yaw(w, id);
    for j in 0..16 {
        let f = (j as f32 * 0.0625 - 0.5) * 45.0 * 0.017_453_292;
        let a = c::add_rot(f, yaw);
        let mut q = c::add(centre, [a.cos() * 0.5, a.sin() * 0.5, 0.0, 0.0]);
        q[3] = if j == 0 || j == 15 { 0.0 } else { 1.0 };
        c::set_pv4(w, id, o::RING + 0x10 * j, q);
    }
    let t = w.ticks(0x96);
    c::set_pf(w, id, o::RING_T, t as f32);
}

/// `0x309bc8(m)`: the ring's sparks (module doc).
fn ring_sparks(w: &mut World, id: MobyId) {
    for j in 0..15 {
        if !(0.99 <= ring_pt(w, id, j)[3] || 0.99 <= ring_pt(w, id, j + 1)[3]) { continue; }
        if 0.25 < w.rng.randf(0.0, 1.0) { continue; }
        let a = w.rng.rand_angle();
        let b = w.rng.rand_angle();
        let mut v1 = [a.cos() * w.rng.randf(0.25, 0.75), 0.0, 0.0, 0.0];
        v1[1] = a.sin() * w.rng.randf(0.25, 0.75);
        let mut v2 = [b.cos() * w.rng.randf(0.25, 0.75), 0.0, 0.0, 0.0];
        v2[1] = b.sin() * w.rng.randf(0.25, 0.75);
        v1[2] = w.rng.randf(-0.065, 0.125);
        v2[2] = w.rng.randf(-0.065, 0.125);
        let sc = |w: &mut World| { let f = w.rng.randf(10.0, 20.0); w.svc.timing.scale(Pf::f(f)).to_f32() as i32 };
        let (t1, t2, t3) = (sc(w), sc(w), sc(w));
        let mut v1 = c::scale(v1, 1.0 / t1 as f32);
        v1[3] = 0.25;
        let mut v2 = c::scale(v2, 1.0 / t3 as f32);
        v2[3] = 0.1;
        let q = ring_pt(w, id, j);
        let k = w.rng.randf(0.0, 1.0);
        let at = c::add(q, c::scale(c::sub(q, q), k));
        fx::part02(w, &crate::particles::type02::Spawn { pos: at, v1, v2, c1: 0x8000_40ff, c2: 0x4000_cc80, t: [t1, t2, t3], def: -1 });
    }
}

/// `0x3094f8(m)`: the ring (module doc).
fn ring(w: &mut World, id: MobyId) {
    let mut t = c::pf(w, id, o::RING_T) as i32;
    c::dec_timer_i32(&mut t);
    c::set_pf(w, id, o::RING_T, t as f32);
    if t == 0 { return; }
    if t == 0x14 {
        for j in 0..16 {
            if 1.0 <= ring_pt(w, id, j)[3] { c::set_pf(w, id, o::RING + 0x10 * j + 0xc, f32::from_bits(0x3f7a_e148)); }
        }
    }
    let (centre, normal, pos) = (c::pv4(w, id, o::CENTRE), c::pv4(w, id, o::NORMAL), c::pos(w, id));
    for j in 0..16 {
        let pt = ring_pt(w, id, j);
        // The game's VecSub / VecAdd / FastVecNormalize write x, y, z only: the point keeps its own fade (w), not
        // the centre's (+0x1fc, the ring's timer).
        let d = c::set_len3([pt[0] - centre[0], pt[1] - centre[1], 0.0, 0.0], 6.0 * DT);
        let mut q = [pt[0] + d[0], pt[1] + d[1], pt[2] + d[2], pt[3]];
        let rel = c::sub(q, pos);
        let e = c::set_len3(normal, -c::dot3(rel, normal) + f32::from_bits(0x3eb3_3333));
        let a = [pt[0], pt[1], pt[2] + 1.0, pt[3]];
        let b = [pt[0], pt[1], pt[2] - 3.0, pt[3]];
        if let Some(h) = w.coll_line(pv(a), pv(b), 2, None) {
            if e[2] < q[2] {
                let mut z = q[2];
                turn::approach(h.point[2] + f32::from_bits(0x3eb3_3333), DT * 4.0, &mut z);
                q[2] = z;
            }
        }
        if pt[3] < 1.0 || w.coll_line(pv(pt), pv(q), 2, None).is_none() {
            if q[3] < 1.0 {
                let mut a = q[3];
                turn::approach(0.0, DT + DT, &mut a);
                q[3] = a;
            }
        } else {
            q[3] = f32::from_bits(0x3f7a_e148);
        }
        c::set_pv4(w, id, o::RING + 0x10 * j, q);
    }
    for j in 0..15 {
        let (a, b) = (ring_pt(w, id, j), ring_pt(w, id, j + 1));
        if !(0.98 <= a[3] && 0.98 <= b[3]) { continue; }
        let y = c::yaw(w, id);
        let tmpl = HitTemplate { dir: [Pf::f(y.cos()), Pf::f(y.sin()), Pf::ONE, Pf::f(5627.925)], attacker: Some(id), flags: 0x1_0001, b18: 0, b19: 1, h1a: w.m(id).o_class as u16, damage: Pf::ONE, w20: 1 };
        line_hit_in(w.table, w.svc, w.classes, w.coll, pv(a), pv(b), 0, Some(id), &tmpl);
    }
    if let Some(row) = super::row(REFERENCE_LEVEL, DRAW_FN) {
        w.svc.draw_callbacks.register(Callback::UnitFrame(row), id);
        w.svc.draw_callbacks.register(Callback::UnitQuads(row), id);
    }
    ring_sparks(w, id);
}

/// The draw's `rand` part: per segment the shared row moves to "last" and is re-rolled (module doc).
pub fn frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < o::SIZE { return; }
    for j in 0..15 {
        let last = f32::from_bits(w.svc.units.word(ST_ROW));
        let new = w.rng.randf(0.0, f32::from_bits(0x3e4c_cccd));
        w.svc.units.set_word(ST_ROW, new.to_bits());
        c::set_pf(w, id, o::ST + 8 * j, last);
        c::set_pf(w, id, o::ST + 8 * j + 4, new);
    }
}

/// Level13 `0x3098d0`: the ring's quads (module doc).
pub fn fx_quads(table: &MobyTable, _svc: &Services, id: MobyId) -> Option<FxQuads> {
    let m = table.mobys.get(id).filter(|m| m.pvars.len() >= o::SIZE)?;
    let q = &m.pvars;
    let centre = p::v4f(q, o::CENTRE);
    let mut quads = Vec::new();
    for j in 0..15 {
        let (last, new) = (p::ff(q, o::ST + 8 * j), p::ff(q, o::ST + 8 * j + 4));
        let st = [[0.0, 1.0], [0.0, last], [1.0, 1.0], [1.0, new]];
        let mut corners = [[0.0f32; 3]; 4];
        let mut rgba = [0u32; 4];
        for (k, (cn, col)) in corners.iter_mut().zip(rgba.iter_mut()).enumerate() {
            let mut pt = p::v4f(q, o::RING + 0x10 * (j + k / 2));
            let f = pt[3];
            let (a, b) = if k & 1 != 0 {
                let d = c::set_len3(c::sub(pt, centre), 0.5);
                pt = c::sub(pt, d);
                pt[2] -= 0.125;
                (0x0000_cccc, 0x6000_cccc)
            } else {
                (0x0000_40ff, 0x6000_40ff)
            };
            *col = crate::particles::tween_color(f.to_bits(), a, b);
            *cn = [pt[0], pt[1], pt[2]];
        }
        quads.push(FxQuad { corners, st, rgba });
        for k in [1, 3] { corners[k][2] += 0.25; }
        quads.push(FxQuad { corners, st, rgba });
    }
    Some(FxQuads { fx: 0xb, additive: true, subtract: false, quads })
}

/// `0x308d10(m)`: the tick (module doc). False: deleted.
fn tick(w: &mut World, id: MobyId) -> bool {
    let oc = w.m(id).o_class;
    w.mm(id).scale = w.class_scale(oc).to_f32() * SCALE;
    ring(w, id);
    if c::pi32(w, id, o::LURE) != 0 {
        let f = w.rng.randf(180.0, 240.0);
        let t = w.svc.timing.scale(Pf::f(f)).to_f32() as i32;
        c::set_pi32(w, id, o::ALERT, t);
        c::set_pi32(w, id, o::LURE, 0);
    }
    c::dec_timer_pvar_i32(w, id, o::ALERT);
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, o::D, 0, 4);
    if res.out5 != 1 && w.m(id).state != 0x10 {
        let own = hit.and_then(|h| h.attacker).is_some_and(|a| w.m(a).o_class == 0x4ee);
        let dmg = if own { 0.0 } else { res.damage };
        let hp = c::pf(w, id, o::D) - dmg;
        c::set_pf(w, id, o::D, hp);
        let reaction = if hp <= 0.0 { 1 } else { res.reaction };
        let kr = o::K;
        c::set_pi32(w, id, kr + knock::k::RADIUS, 0x2cd);
        c::set_pf(w, id, kr + knock::k::ZOFF, f32::from_bits(0x3f33_3333));
        let dir = hit.map_or([0.0; 4], |h| h.dir.map(|x| x.to_f32()));
        let a = c::atan(dir[0], dir[1]);
        match reaction {
            1 | 2 => {
                w.mm(id).state = 0x10;
                w.mm(id).mode &= !mode::TARGETABLE;
                c::set_pf(w, id, kr + knock::k::KEY_APEX, 9.0);
                c::set_pf(w, id, kr + knock::k::KEY_LAND, 17.0);
                c::set_pf(w, id, kr + knock::k::UP, 5.7 * DT);
                c::set_pf(w, id, kr + knock::k::SPEED, 19.0 * DT);
                knock::start(w, id, kr, a, 8, 1, 0);
                c::set_pu8(w, id, o::F + 7, 0xfa);
                set_death_bits(w, id, 0, -1);
            }
            3..=10 => {
                w.mm(id).state = 0xf;
                let t = w.ticks(0x3c);
                c::set_pi16(w, id, o::COOLDOWN, t as i16);
                c::set_pf(w, id, kr + knock::k::KEY_APEX, 8.0);
                c::set_pf(w, id, kr + knock::k::UP, 5.5 * DT);
                c::set_pf(w, id, kr + knock::k::KEY_LAND, 15.0);
                c::set_pf(w, id, kr + knock::k::SPEED, 11.0 * DT);
                knock::start(w, id, kr, a, 7, 1, 0);
                c::set_pu8(w, id, o::F + 7, 0xfa);
            }
            _ => {}
        }
        flash::start(w, id, o::F);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, o::F);
    let range = c::pf(w, id, if c::pi32(w, id, o::ALERT) == 0 { o::RANGE_A } else { o::RANGE_B });
    c::set_pf(w, id, o::RANGE, range);
    let area = usize::try_from(c::pi32(w, id, o::AREA)).ok();
    let t = target::acquire_in(w, id, range, area);
    c::set_pv4(w, id, o::T, t.pos);
    c::set_pi32(w, id, o::T_MOBY, t.moby.map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, o::T_KIND, t.kind as i32);
    if t.kind != 2 {
        let home = c::pv4(w, id, o::HOME);
        if range < c::dist2(home, t.pos) || 3.0 < (c::pos(w, id)[2] - t.pos[2]).abs() {
            c::set_pi32(w, id, o::T_KIND, 2);
        } else {
            let cub = c::pi32(w, id, o::TRIGGER);
            if cub != -1 {
                if w.in_cuboid([t.pos[0], t.pos[1], t.pos[2]], cub) { c::set_pi32(w, id, o::TRIGGER, -1); } else { c::set_pi32(w, id, o::T_KIND, 2); }
            }
        }
    }
    if c::pi32(w, id, o::T_MOBY) == 0 { c::set_pi32(w, id, o::T_MOBY, w.hero_moby.map_or(0, |m| m as i32 + 1)); }
    let g = c::ground::ground(w, c::pos(w, id), 0.5, 0);
    let st = w.m(id).state;
    if !g.hit || g.z == 0.0 {
        if st < 0xf { fall(w, id); }
    } else {
        if matches!(g.surface, 0 | 1 | 3 | 0xb | 0xd) {
            w.mm(id).mode &= !mode::TARGETABLE;
            set_death_bits(w, id, 0, -1);
            let p = c::pos(w, id);
            fx::beam_explosion(w, &DEATH, Some(id), p);
            w.delete_moby(id);
            return false;
        }
        if st < 0xf {
            let pp = c::pos(w, id);
            let a = [pp[0], pp[1], pp[2] + 0.5, pp[3]];
            let b = [pp[0], pp[1], 0.01, pp[3]];
            if let Some(h) = w.coll_line(pv(a), pv(b), 2, None) {
                let n = [h.normal[0], h.normal[1], h.normal[2], 0.0];
                if 0.698_131_7 < c::atan(n[2], c::len2(n)) {
                    let s = c::set_len3(n, f32::from_bits(0x3d4c_cccd));
                    w.mm(id).position = c::add(c::pos(w, id), s);
                }
            }
            let z = c::pos(w, id)[2];
            if z <= g.z {
                if z < g.z { w.mm(id).position[2] = z + 0.05; }
            } else {
                fall(w, id);
                if c::pos(w, id)[2] < g.z {
                    c::set_pf(w, id, o::K + 8, 0.0);
                    w.mm(id).position[2] = g.z;
                }
            }
        }
    }
    let p = c::pos(w, id);
    if !p[..3].iter().all(|&x| (2.0..=1021.0).contains(&x)) {
        w.delete_moby(id);
        return false;
    }
    true
}

fn fall(w: &mut World, id: MobyId) {
    let v = c::pf(w, id, o::K + 8) - DT2 * 9.8;
    c::set_pf(w, id, o::K + 8, v);
    w.mm(id).position[2] += v;
}

/// The walker step 2 ahead; true when a ledge stopped it (result bit 1).
fn step(w: &mut World, id: MobyId) -> bool {
    let (cy, sy) = c::cs(c::yaw(w, id));
    let mut out = [0.0; 4];
    walker::step(w, id, o::J, 1.0, [cy + cy, sy + sy, 0.0, 0.0], &mut out) & 2 != 0
}
fn keep_off_walls(w: &mut World, id: MobyId) {
    let Some(path) = usize::try_from(c::pi32(w, id, o::WALLS)).ok() else { return };
    if let Some(p) = region::push_out(w, 0.75, path, c::pos(w, id)) { w.mm(id).position = p; }
}
fn walled_off(w: &World, id: MobyId) -> bool {
    let Some(path) = usize::try_from(c::pi32(w, id, o::WALLS)).ok() else { return false };
    let p = c::pos(w, id);
    let (crossed, at) = region::clamp(w, path, p, tpos(w, id));
    crossed && c::dist3(p, at) < 0.8
}
/// The swing: lines swept from joint 0 to the body at `up`, frames 9..15 (module doc).
fn swing(w: &mut World, id: MobyId, up: f32) {
    let a = w.joint_point(id, 0);
    let mut b = c::pos(w, id);
    b[2] += up;
    let key = c::ground::key_time(w, id);
    if (9.0..=15.0).contains(&key) {
        let y = c::yaw(w, id);
        let tmpl = HitTemplate { dir: [Pf::f(y.cos()), Pf::f(y.sin()), Pf::ONE, Pf::f(f32::from_bits(0x45af_df66))], attacker: Some(id), flags: 1, b18: 0, b19: 1, h1a: w.m(id).o_class as u16, damage: Pf::ONE, w20: 1 };
        let prev = c::pv4(w, id, o::SWING);
        c::attack::swept_lines(w, a, b, prev, b, id, &tmpl, 5);
    }
    c::set_pv4(w, id, o::SWING, a);
}

/// Level13 `0x307e98` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    if !tick(w, id) { return; }
    crate::moby_update::manip::big_head(w, 2.5, id, 1, id, o::HEAD);
    if w.m(id).visible != 0 {
        let cam = w.camera.map(|x| f32::from_bits(x.0));
        if c::dist3(c::pos(w, id), cam) < 28.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0x16;
        }
    }
    let to = |w: &mut World, id: MobyId, s: u8| w.mm(id).state = s;
    match w.m(id).state {
        0 => {
            c::set_pf(w, id, o::D, 2.0);
            c::set_pu8(w, id, o::D + 8, 1);
            c::set_pu8(w, id, 0x58, 0xc);
            c::set_pu8(w, id, 0x5a, 8);
            c::set_pi16(w, id, o::D + 4, 2);
            c::set_pf(w, id, o::D + 0x10, 1.5);
            walker::seed(&mut w.mm(id).pvars, o::J);
            c::set_pf(w, id, o::J + 8, 2.0);
            c::set_pf(w, id, o::J + 0xc, 0.5);
            c::set_pi32(w, id, o::J, (SCALE * 716.8) as i32);
            c::set_pf(w, id, o::J + 4, SCALE * 0.7);
            let r = w.rng.randf(f32::from_bits(0x3f73_3333), f32::from_bits(0x3f86_6666));
            c::set_pf(w, id, o::J + 0x24, 4.0 * DT * r);
            let p = c::pos(w, id);
            c::set_pv4(w, id, o::HOME, p);
            let j = w.joint_point(id, 0);
            c::set_pv4(w, id, o::SWING, j);
            to(w, id, 1);
        }
        1 => {
            turn_to_target(w, id);
            if kind(w, id) == 2 {
                if c::pi32(w, id, o::ALERT) != 0 {
                    to(w, id, 0xb);
                    blend(w, id, 1, 0x14);
                }
            } else {
                let t = tpos(w, id);
                if 4.0 < c::dist2(c::pos(w, id), t) {
                    if c::diff_rots(c::yaw(w, id), heading(w, id, t)) < 0.261_799_4 {
                        to(w, id, 2);
                        blend(w, id, 3, 10);
                    } else {
                        to(w, id, 7);
                        blend(w, id, 1, 0x14);
                    }
                } else {
                    to(w, id, 5);
                    blend(w, id, 6, 10);
                }
            }
        }
        2 => {
            if !done(w, id) { return; }
            to(w, id, 3);
            ring_start(w, id);
            blend(w, id, 5, 10);
        }
        3 => {
            if !done(w, id) { return; }
            let d = c::dist2(c::pos(w, id), tpos(w, id));
            if kind(w, id) != 2 && d < 15.0 {
                to(w, id, 4);
                blend(w, id, 4, 0x14);
                let t = w.ticks(0x5a);
                c::set_pi32(w, id, o::WAIT, t);
                return;
            }
            to(w, id, 1);
            blend(w, id, 0, 0x14);
        }
        4 => {
            turn_to_target(w, id);
            if c::dec_timer_pvar_i32(w, id, o::WAIT) != 0 {
                to(w, id, 3);
                ring_start(w, id);
                blend(w, id, 5, 5);
                return;
            }
            if c::dist2(c::pos(w, id), tpos(w, id)) < 4.0 {
                let j = w.joint_point(id, 0);
                c::set_pv4(w, id, o::SWING, j);
                to(w, id, 5);
                blend(w, id, 6, 10);
            } else if c::pi32(w, id, o::ALERT) != 0 {
                to(w, id, 0xb);
                blend(w, id, 1, 0x14);
            }
        }
        5 => {
            turn_to_target(w, id);
            swing(w, id, f32::from_bits(0x3e99_999a));
            if done(w, id) {
                to(w, id, 6);
                w.mm(id).cmd = 1;
                blend(w, id, 2, 10);
            }
        }
        6 => {
            if !done(w, id) { return; }
            let cmd = w.m(id).cmd;
            to(w, id, cmd);
            blend(w, id, 0, 0x14);
        }
        7 => {
            let h = c::add_rot(heading(w, id, tpos(w, id)), c::pf(w, id, o::SIDE));
            turn::spring_turn2_pvar(w, id, h, f32::from_bits(0x3d4c_cccd), f32::from_bits(0x3e99_999a), DT * TAU, o::TURN_V);
            if step(w, id) {
                w.rng.rand();
                c::set_pf(w, id, o::SIDE, -1.099_557_4);
            } else {
                c::set_pf(w, id, o::SIDE, 0.0);
            }
            keep_off_walls(w, id);
            let blocked = walled_off(w, id);
            let d = c::dist2(c::pos(w, id), tpos(w, id));
            if d < 15.0 || blocked {
                to(w, id, 2);
                blend(w, id, 3, 10);
            } else if d < 4.0 {
                to(w, id, 5);
                blend(w, id, 6, 10);
            }
        }
        0xb => {
            let h = heading(w, id, tpos(w, id));
            turn::spring_turn2_pvar(w, id, h, f32::from_bits(0x3d4c_cccd), f32::from_bits(0x3e99_999a), DT * TAU, o::TURN_V);
            step(w, id);
            keep_off_walls(w, id);
            let blocked = walled_off(w, id);
            if c::pi32(w, id, o::ALERT) == 0 {
                to(w, id, 0xd);
            } else if blocked {
                to(w, id, 0xc);
                blend(w, id, 6, 10);
            } else if c::dist2(c::pos(w, id), tpos(w, id)) < 4.0 {
                let j = w.joint_point(id, 0);
                c::set_pv4(w, id, o::SWING, j);
                to(w, id, 0xc);
                blend(w, id, 6, 10);
            }
        }
        0xc => {
            swing(w, id, 1.5);
            if done(w, id) {
                to(w, id, 0xb);
                blend(w, id, 1, 0x14);
            }
        }
        0xd => {
            let home = c::pv4(w, id, o::HOME);
            let h = heading(w, id, home);
            turn::spring_turn2_pvar(w, id, h, f32::from_bits(0x3d4c_cccd), f32::from_bits(0x3e99_999a), DT * TAU, o::TURN_V);
            if c::diff_rots(c::yaw(w, id), heading(w, id, home)) < std::f32::consts::FRAC_PI_4 {
                step(w, id);
                keep_off_walls(w, id);
            }
            if c::dist3(home, c::pos(w, id)) < 1.0 {
                to(w, id, 1);
                blend(w, id, 0, 0x14);
            }
        }
        0xf => {
            let r = knock::update(w, id, o::K);
            if r & 0x40 != 0 {
                c::set_pv4(w, id, o::K, [0.0; 4]);
                to(w, id, 1);
                blend(w, id, 0, 10);
            } else if r & 0x120 != 0 {
                w.delete_moby(id);
            }
        }
        0x10 => {
            let r = knock::update(w, id, o::K);
            if r & 0x160 != 0 {
                let p = c::pos(w, id);
                fx::beam_explosion(w, &DEATH, Some(id), p);
                w.delete_moby(id);
            }
        }
        _ => {}
    }
}
