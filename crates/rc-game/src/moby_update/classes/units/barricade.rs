//! Veldin's grouped breakable pieces, classes 885, 888, 891, 892, 894, 900, 901, 936 (level 18, 119 instances): level18
//! 0x2e9768, one update for all eight (census unit U553). A hit on any piece (mask 0x80000) sets its whole group to
//! state 2: every piece flies off away from Ratchet and explodes where it lands; the class 892 (0x37c) bursts into
//! sparks and puffs in place instead. Read from the level18 decomp and disassembly. Native `f32`. The name is
//! descriptive [L].
//!
//! **Pvar block** (P, bytes): +0x00 a pointer the init writes 3 through when it is not 0 (self-relative in the level
//! data: 52 of the 119 point at their own +0x08, the spin, which state 2 overwrites), +0x08 the spin per tick, +0x0c a
//! second spin (drawn, not read), +0x10 the velocity (xyz), +0x1c the z it started from.
//!
//! * **0** init: with a mission (+0xb0 ≠ 0xff) that is not done (the level's mission byte 0xff) the piece is deleted;
//!   else → **1**.
//! * **1**: a hit message with flag 0x80000: the group's state = 2 (`0x26e0e0`), `PlayClassSoundByClass(0, 0, moby,
//!   885)`; +0xa4 = 0xff.
//! * **2** (not 892): collision off, → **3**; spins `randf_sym(0, 90)`°·dt ×2; velocity = `randf(10, 15)·dt` along the
//!   level direction from Ratchet (z 0), plus `rand_vec` of that length, z made positive; start z kept.
//! * **2** (892): 10 spark pairs `PartType11Spawn(1.5e6, randf(8, 10)·dt [·0.5 for the white one], pos + (0, 0,
//!   randf(0, 6)))` (the explosion colours, lives `rand_range(ticks(15), ticks(20))` / `(ticks(30), ticks(45))` and
//!   `(ticks(5), ticks(10))` / `(ticks(15), ticks(20))`), then 10 puffs `PartType08Spawn(5e5, pos + (0, 0, randf(0,
//!   6)), (randf(−1, 1) ×3) normalised to randf(0, 3)·dt, colours, rand_range(ticks(30), ticks(45)))`; deleted.
//! * **3** flying: x and y rotation += spin, velocity z −= 20·dt², pos += velocity; below 16 under its start, or on
//!   contact (`coll_sphere(pos, 1, 2, self)`): the death explosion `0x273f50(1.5, 0, moby, pos, −1)` and deleted.
//! * other states: deleted.

#![allow(clippy::needless_range_loop)] // lane loops spelled out.

use super::hero_pos;
use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, fx, DT, DT2};
use crate::moby_update::scheduler::group_state;
use crate::moby_update::services::{pf, pv, World};

pub const UPDATE_FN: u32 = 0x2e_9768;
pub const REFERENCE_LEVEL: u32 = 18;
pub const CLASSES: [i16; 8] = [885, 888, 891, 892, 894, 900, 901, 936];
/// The class that bursts in place (0x37c).
pub const BURST: i16 = 892;
/// The class whose sound table the hit plays (0x375).
pub const SOUND_CLASS: i16 = 885;
pub const HIT_MASK: u32 = 0x8_0000;
/// 0.017453292 (π/180).
const DEG: f32 = 0.017_453_292;

pub mod pv_ {
    pub const LINK: usize = 0x00;
    pub const SPIN: usize = 0x08;
    pub const SPIN2: usize = 0x0c;
    pub const VEL: usize = 0x10;
    pub const START_Z: usize = 0x1c;
    pub const LEN: usize = 0x20;
}
use pv_ as o;

/// Level18 0x2e9768 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < o::LEN { return; }
    match w.m(id).state {
        0 => {
            let (ms, level) = (w.m(id).mission, w.svc.level);
            if ms == 0xff || w.mission_done(level, ms) != 0xff {
                w.mm(id).state = 1;
                // The pointer is self-relative in the level data (a pvar offset, `rc_formats::gameplay::parse_pvars`).
                let at = c::pi32(w, id, o::LINK);
                if at != 0 && (at as usize) + 4 <= w.m(id).pvars.len() { c::set_pi32(w, id, at as usize, 3); }
                return;
            }
            w.delete_moby(id);
        }
        1 => {
            if w.get_hit(id, HIT_MASK, false).is_some() {
                let g = w.m(id).group;
                group_state(w, g, 2);
                w.play_sound_as(0, 0, id, SOUND_CLASS);
            }
            w.mm(id).hit_slot = 0xff;
        }
        2 if w.m(id).o_class != BURST => launch(w, id),
        2 => {
            burst(w, id);
            w.delete_moby(id);
        }
        3 => fly(w, id),
        _ => w.delete_moby(id),
    }
}

/// State 2 of the flying pieces.
fn launch(w: &mut World, id: MobyId) {
    let speed = w.rng.randf(10.0, 15.0) * DT;
    let m = w.mm(id);
    m.has_collision = false;
    m.state = 3;
    let s = w.rng.randf_sym(0.0, 90.0) * DEG * DT;
    c::set_pf(w, id, o::SPIN, s);
    let s = w.rng.randf_sym(0.0, 90.0) * DEG * DT;
    c::set_pf(w, id, o::SPIN2, s);
    let p = c::pos(w, id);
    let d = c::sub(p, hero_pos(w));
    let v = c::set_len3([d[0], d[1], 0.0, 0.0], speed);
    let r = w.rng.rand_vec(speed, speed);
    let v = [v[0] + r[0], v[1] + r[1], (v[2] + r[2]).abs(), p[2]];
    c::set_pv4(w, id, o::VEL, v);
}

/// State 3.
fn fly(w: &mut World, id: MobyId) {
    let spin = c::pf(w, id, o::SPIN);
    let mut v = c::pv4(w, id, o::VEL);
    v[2] -= DT2 * 20.0;
    c::set_pv4(w, id, o::VEL, v);
    let m = w.mm(id);
    m.rotation[0] = c::add_rot(m.rotation[0], spin);
    m.rotation[1] = c::add_rot(m.rotation[1], spin);
    for k in 0..3 { m.position[k] += v[k]; }
    let p = m.position;
    let low = p[2] < v[3] - 16.0;
    if !low && w.coll_sphere(pv(p), pf(1.0), 2, Some(id)).is_none() { return; }
    fx::death_explosion(w, 1.5, 0.0, Some(id), p, -1);
    w.delete_moby(id);
}

/// State 2 of 892: the sparks and puffs.
fn burst(w: &mut World, id: MobyId) {
    let base = c::pos(w, id);
    for _ in 0..10 {
        let sp = w.rng.randf(8.0, 10.0) * DT;
        let mut p = base;
        p[2] += w.rng.randf(0.0, 6.0);
        let a = w.rng.randi(6) as usize;
        let b = w.rng.randi(6) as usize;
        let (t15, t20) = (w.ticks(15), w.ticks(20));
        let life = w.rng.rand_range(t15, t20);
        let (t30, t45) = (w.ticks(30), w.ticks(45));
        let t1 = w.rng.rand_range(t30, t45);
        w.part11(pf(1.5e6), pf(sp), pv(p), pv([0.0; 4]), fx::SPARK_A[a], fx::SPARK_B[b], life, t1, 0, 0);
        let (t5, t10) = (w.ticks(5), w.ticks(10));
        let life = w.rng.rand_range(t5, t10);
        let (t15, t20) = (w.ticks(15), w.ticks(20));
        let t1 = w.rng.rand_range(t15, t20);
        w.part11(pf(1.5e6), pf(sp * 0.5), pv(p), pv([0.0; 4]), 0x7fff_ffff, 0x00ff_ffff, life, t1, 0, 0);
    }
    for _ in 0..10 {
        let mut p = base;
        p[2] += w.rng.randf(0.0, 6.0);
        let v = [w.rng.randf(-1.0, 1.0), w.rng.randf(-1.0, 1.0), w.rng.randf(-1.0, 1.0), 0.0];
        let l = w.rng.randf(0.0, 3.0) * DT;
        let v = c::set_len3(v, l);
        let a = w.rng.randi(6) as usize;
        let b = w.rng.randi(6) as usize;
        let (t30, t45) = (w.ticks(30), w.ticks(45));
        let life = w.rng.rand_range(t30, t45);
        fx::part08(w, 5e5, p, v, fx::SPARK_A[a], fx::SPARK_B[b], life);
    }
}
