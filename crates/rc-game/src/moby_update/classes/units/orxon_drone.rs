//! **Orxon's gun drones, class 1229** (level10 `0x2e4a88`, census U363; 5 placed) and **their shots, class 819**
//! (`0x2cc2e8`, made by `0x2cc1b8`).
//!
//! **A drone** (pvars: +0x20 the damage record, health 9; +0xc0 the flash; +0xd0 the target record (`0x274df8`,
//! the moby at +0x110, its kind +0x114); +0x120 the velocity; +0x140 its rise path, +0x144 its patrol path, +0x148
//! the wake cuboid, +0x158 the point, +0x160 / +0x164 the attack / rest seconds, +0x168 the timer, +0x190 the area
//! path, +0x198 / +0x19c the turn speeds, +0x1b0 the flee cuboid, +0x1d0 the big-head node). Each tick: the big-head
//! cheat (2.5, list 2); the hits (`0x2e5968`: mask 0x370000, the resolver with flags 5 into the record: dead (health
//! ≤ 0 or flag 0x40000) → 8; reactions 9 / 10 / 12 a red flash; else the flash, sequence 4, → 7 keeping its state in
//! +0xbc); the target (Clank, 0x1413f4 = 1: the nearest gadgetbot 857 not asleep or Clank within 100, scored by
//! distance + 5·turn, Clank −2 (`0x2d7510`), dropped outside the area path; on foot: `0x274df8(100)` in the area path;
//! none: Ratchet's moby).
//! * 0: targetable, the health 9, hidden at its rise path's start, → 1 (its mission done: deleted).
//! * 1: the target in the wake cuboid → 2: shown, collision on, sequence 1; on foot, the help 10005 (0x5c) unless the
//!   O2 Mask (item 11) is owned.
//! * 2 / 3: it rises 4 a second, then flies its rise path at 8 a second turning to it (yaw `SpringTurn2` 4π·dt, pitch
//!   toward the target), → 4 at its end with the attack timer.
//! * 4: facing the target within 10° sequence 3 (anim speed gp−0x4d14); in sequence 3 at keys 1 and 16 a shot from
//!   joint list 1 / 0 (1 out along its aim) at the target's aim point (on foot: if a line from it reaches the target;
//!   Clank: led by his velocity over `ticks(20)`, at the ground + 0.2) at gp−0x4d18·dt with a muzzle flash (24 type-44
//!   puffs and 6 type-21 sparks ringed about its forward 3 steps out); the timer out → 5 (rest). 5: the same aim, no
//!   shots, back to 4. Both patrol their path at 8 a second. The target in the flee cuboid → 6: it rises 10 a second
//!   and shrinks, deleted above 250 or tiny. 7: the stun's sequence 4 done → back. 8: a beam explosion, its bolts
//!   (`SetDeathBits(m, 0, −1)`), two pieces 0x5fd / 0x5fe (`BreakFxB`), deleted.
//!
//! Every tick, drawn within 28 of the camera: the shadow probe, +0x7f = 0x16.
//!
//! **A shot** 819: life `ticks(300)`, tumbling; at Clank (or Giant Clank, ground + 2.2) it homes, steering yaw and
//! pitch (`SpringTurn` 2π·dt² / π·dt² / 2π·dt) to the interception of his velocity; a red and a smoke spark a tick
//! (type 4); a whizz (sound 0) once it moves away from Ratchet within 5; a line hit (flags 0x10) on anything but its
//! drone: a moby gets damage 1, flags 0x10003; then (or its life or target gone) the death explosion (0.25, light 13
//! but 0 on level 10), sound 0 of class 0x99, deleted.
//!
//! Read from the level10 decomp; gp−0x4d1c ticks, −0x4d18 the shot speed, −0x4d14 the anim speed. [L] The help's
//! guard 0x141c48 (an s16 of the hero block) is taken as 0.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2e4a88` | the drone | [`update`] |
//! | `0x2e5968` | its hits and target | `hits` |
//! | `0x2d7510` / `0x2d7420` | the Clank-mode target | `clank_target` |
//! | `0x255ab0(m, p, off)` | the muzzle flash | `muzzle_flash` |
//! | `0x2cc1b8` / `0x2cc2e8` | the shot / its update | `shot` / [`shot_update`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::classes::flyer::spring_turn;
use crate::moby_update::creature::{self as c, damage, flash, fx, target, turn, V, DT, DT2};
use crate::moby_update::services::{pf, pv, World};
use crate::moby_update::story;
use std::f32::consts::{PI, TAU};

pub const REFERENCE_LEVEL: u32 = 10;
pub const UPDATE_FN: u32 = 0x2e_4a88;
pub const SHOT_FN: u32 = 0x2c_c2e8;
pub const CLASSES: [i16; 1] = [1229];
pub const SHOT_CLASSES: [i16; 1] = [819];

const SHOT: i16 = 0x333;
const LEN: usize = 0x1f0;
const DEG10: f32 = 0.174_532_92;

mod pvo {
    pub const RECORD: usize = 0x20;
    pub const FLASH: usize = 0xc0;
    pub const TARGET: usize = 0xd0;
    pub const TMOBY: usize = 0x110;
    pub const KIND: usize = 0x114;
    pub const VEL: usize = 0x120;
    pub const RISE: usize = 0x140;
    pub const PATROL: usize = 0x144;
    pub const WAKE: usize = 0x148;
    pub const NODE: usize = 0x158;
    pub const ATTACK: usize = 0x160;
    pub const REST: usize = 0x164;
    pub const TIMER: usize = 0x168;
    pub const AREA: usize = 0x190;
    pub const T196: usize = 0x196;
    pub const YAW_V: usize = 0x198;
    pub const PITCH_V: usize = 0x19c;
    pub const FLEE: usize = 0x1b0;
    pub const HEAD: usize = 0x1d0;
}

fn path(w: &World, id: MobyId, o: usize) -> Vec<V> {
    usize::try_from(c::pi32(w, id, o)).ok().and_then(|i| w.svc.splines.get(i)).map(|v| v.iter().map(|q| q.map(f32::from_bits)).collect()).unwrap_or_default()
}
fn tmoby(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, pvo::TMOBY) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn set_tmoby(w: &mut World, id: MobyId, m: Option<MobyId>) { c::set_pi32(w, id, pvo::TMOBY, m.map_or(0, |m| m as i32 + 1)); }
/// gp−0x4d1c / −0x4d18 / −0x4d14 (level10 0x161ee4 / 0x161ee8 / 0x161eec): 90 ticks, the shot speed 32, the anim speed 2.
const T196: i32 = 90;
const SHOT_SPEED: f32 = 32.0;
const ANIM_SPEED: f32 = 2.0;
/// 0x13f450: Ratchet's displacement this tick.
fn hero_disp(w: &World) -> [f32; 3] { [w.hero.disp[0], w.hero.disp[1], w.hero.disp[2]].map(|x| f32::from_bits(x.0)) }

fn blend(w: &mut World, id: MobyId, seq: u8, t: i32) {
    let t = w.ticks(t);
    w.anim_blend(id, seq, 0, t);
}

fn set_timer(w: &mut World, id: MobyId, secs_off: usize) {
    let t = (c::pf(w, id, secs_off) * 60.0 * crate::moby_update::services::fl(w.svc.timing.timer_scale)) as i32;
    c::set_pi32(w, id, pvo::TIMER, t);
}

/// `0x2d7420`: a candidate's score (xy distance + 5·the turn to it, the current target −2) within `range`.
fn consider(w: &World, id: MobyId, cur: Option<MobyId>, m: MobyId, best: &mut (Option<MobyId>, f32), range: f32) {
    let (p, q) = (c::pos(w, id), w.m(m).position);
    let d = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)).sqrt();
    if range < d { return; }
    let mut s = d + c::diff_rots(w.m(id).rotation[2], c::atan(q[0] - p[0], q[1] - p[1])) * 5.0;
    if Some(m) == cur { s -= 2.0; }
    if s < best.1 { *best = (Some(m), s); }
}

/// `0x2d7510(range, m, &target)`: the gadgetbots 857 awake (state > 1 and not 9), then Clank.
fn clank_target(w: &World, id: MobyId, range: f32) -> Option<MobyId> {
    let cur = tmoby(w, id);
    let mut best = (None, f32::from_bits(0x4974_23f0));
    for m in 0..w.table.mobys.len() {
        let mo = &w.table.mobys[m];
        if mo.o_class == 0x359 && 1 < mo.state && mo.state != 9 && mo.state < 0x80 { consider(w, id, cur, m, &mut best, range); }
    }
    if let Some(h) = w.hero_moby { consider(w, id, cur, h, &mut best, range); }
    best.0
}

/// `0x2e5968` (module doc).
fn hits(w: &mut World, id: MobyId) {
    let hit = w.get_hit(id, 0x37_0000, false);
    let r = damage::resolve(w, id, hit, pvo::RECORD, 5, 4);
    if let Some(h) = hit {
        if r.out5 != 1 && w.m(id).state != 8 {
            let health = c::pf(w, id, pvo::RECORD) - r.damage;
            c::set_pf(w, id, pvo::RECORD, health);
            let mut reaction = r.reaction;
            if health <= 0.0 || h.flags & 0x4_0000 != 0 { reaction = 1; }
            match reaction {
                1 => {
                    w.mm(id).pvars[pvo::FLASH + 7] = 0xfa;
                    flash::start(w, id, pvo::FLASH);
                    w.mm(id).state = 8;
                }
                9 | 10 | 12 => {
                    w.mm(id).pvars[pvo::FLASH + 7] = 0xfa;
                    flash::start(w, id, pvo::FLASH);
                }
                13 => {}
                _ => {
                    w.mm(id).pvars[pvo::FLASH + 7] = 0xfa;
                    let t = w.ticks(0x3c) as i16;
                    c::set_pi16(w, id, 0x26, t);
                    flash::start(w, id, pvo::FLASH);
                    blend(w, id, 4, 6);
                    let s = w.m(id).state;
                    w.mm(id).state = 7;
                    w.mm(id).cmd = s;
                }
            }
        }
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pvo::FLASH);
    let area = c::pi32(w, id, pvo::AREA);
    if w.body() == 1 {
        let t = clank_target(w, id, 100.0);
        set_tmoby(w, id, t);
        match t {
            None => c::set_pi32(w, id, pvo::KIND, 2),
            Some(m) => {
                let q = w.m(m).position;
                let inside = w.in_path([q[0], q[1], q[2]], area);
                if inside {
                    c::set_pi32(w, id, pvo::KIND, 0);
                } else {
                    set_tmoby(w, id, None);
                    c::set_pi32(w, id, pvo::KIND, 2);
                }
            }
        }
    } else {
        let t = target::acquire_in(w, id, 100.0, usize::try_from(area).ok());
        c::set_pv4(w, id, pvo::TARGET, t.pos);
        c::set_pv4(w, id, pvo::TARGET + 0x10, t.rot);
        c::set_pv4(w, id, pvo::TARGET + 0x20, t.aim);
        c::set_pv4(w, id, pvo::TARGET + 0x30, t.body);
        set_tmoby(w, id, t.moby);
        c::set_pi32(w, id, pvo::KIND, t.kind as i32);
        if t.kind != 2 && w.m(id).visible == 0 { c::set_pi32(w, id, pvo::KIND, 2); }
    }
    if tmoby(w, id).is_none() { set_tmoby(w, id, w.hero_moby); }
}

/// Yaw toward `toward` (`SpringTurn2` 0.02 / 0.3 / `max`) and pitch toward the target's `aim`.
fn face(w: &mut World, id: MobyId, toward: V, aim: V, max: f32) {
    let p = c::pos(w, id);
    turn::spring_turn2_pvar(w, id, c::atan(toward[0] - p[0], toward[1] - p[1]), 0.02, f32::from_bits(0x3e99_999a), max, pvo::YAW_V);
    let dxy = ((aim[0] - p[0]).powi(2) + (aim[1] - p[1]).powi(2)).sqrt();
    let e = c::atan(dxy, aim[2] - p[2]);
    let mut v = c::pf(w, id, pvo::PITCH_V);
    let y = spring_turn(w.m(id).rotation[1], -e, 0.02, f32::from_bits(0x3e99_999a), DT * PI, &mut v);
    c::set_pf(w, id, pvo::PITCH_V, v);
    w.mm(id).rotation[1] = y;
}

/// The aim point and whether a line reaches the target (module doc).
fn aim(w: &mut World, id: MobyId, t: MobyId) -> (V, bool) {
    let mut a = w.m(t).position;
    if w.body() == 1 {
        if Some(t) == w.hero_moby {
            let k = w.ticks(0x14) as f32;
            let hv = hero_disp(w);
            a = [a[0] + hv[0] * k, a[1] + hv[1] * k, a[2] + hv[2] * k, a[3]];
            a[2] = f32::from_bits(w.ground_height(pf(0.5), pv(a), 0).0);
        }
        a[2] += 0.2;
        return (a, true);
    }
    a = c::pv4(w, id, pvo::TARGET + 0x20);
    let p = c::pos(w, id);
    let h = w.coll_line(pv(p), pv(a), 0, Some(id));
    (a, h.and_then(|h| h.moby) == Some(t))
}

/// `0x255ab0(m, p, off)`: the muzzle flash about the drone's forward (module doc).
pub(super) fn muzzle_flash(w: &mut World, id: MobyId, p: V) {
    let r = w.m(id).rows;
    let (fwd, up) = ([r[0][0], r[0][1], r[0][2]], [r[2][0], r[2][1], r[2][2]]);
    for i in 0..24 {
        let j = w.rng.randf_sym(0.0, f32::from_bits(0x3e06_0a92));
        let d = crate::moby_update::classes::blaster_shot::rotate(up, i as f32 * f32::from_bits(0x3e86_0a92) + j, fwd);
        let s = w.rng.randf(f32::from_bits(0x3d4c_cccd), f32::from_bits(0x3dcc_cccd));
        let d = c::set_len3([d[0], d[1], d[2], 0.0], s);
        let mut spin = w.rng.rand_range(0, 3);
        if w.rng.randi(2) != 0 { spin = -spin; }
        let (a, b) = (w.ticks(0x1e), w.ticks(0x5a));
        let life = w.rng.rand_range(a, b);
        let s = crate::particles::type44::Spawn { size: 60000.0, growth: 3000.0, damp: f32::from_bits(0x3f59_999a), fall: f32::from_bits(0xba83_126f), w: f32::from_bits(0x3f59_999a), pos: [p[0], p[1], p[2]], vel: [d[0], d[1], d[2]], life, alpha: 0x1e, rgb: 0xff_ffff, spin };
        fx::part44(w, &s);
    }
    for i in 0..6 {
        let j = w.rng.randf(0.0, std::f32::consts::FRAC_PI_4);
        let d = crate::moby_update::classes::blaster_shot::rotate(up, i as f32 * std::f32::consts::FRAC_PI_3 + j, fwd);
        let s = w.rng.randf(f32::from_bits(0x3d4c_cccd), f32::from_bits(0x3dcc_cccd));
        let d = c::set_len3([d[0], d[1], d[2], 0.0], s);
        let (a, b) = (w.ticks(0x14), w.ticks(0x3c));
        let life = w.rng.rand_range(a, b);
        fx::part21(w, 10000.0, p, d, 0x4f00_7fff, 0x1fff_ffff, life, 1);
    }
}

/// `0x2cc1b8(scale, v, p, owner, target, life)`: a shot.
pub(super) fn shot(w: &mut World, scale: f32, v: V, p: V, owner: MobyId, target: Option<MobyId>, life: i32) {
    let Some(m) = w.create_moby(SHOT) else { return };
    story::pvars(w, m, 0x40);
    let hp = w.hero_point();
    {
        let mo = w.mm(m);
        mo.update_dist = 0xff;
        mo.draw_dist = 0x7e;
        mo.state = 1;
        mo.scale *= scale;
        mo.visible = 1;
        mo.position = p;
    }
    c::set_pv4(w, m, 0, v);
    let t = w.ticks(life);
    c::set_pi32(w, m, 0x10, owner as i32 + 1);
    c::set_pi32(w, m, 0x14, target.map_or(0, |t| t as i32 + 1));
    c::set_pf(w, m, 0x28, 0.0);
    c::set_pi32(w, m, 0x18, t);
    c::set_pi32(w, m, 0x30, 0);
    c::set_pi32(w, m, 0x34, 0);
    c::set_pf(w, m, 0x38, c::dist3(p, [hp[0], hp[1], hp[2], 0.0]));
    let mo = w.mm(m);
    mo.rotation[2] = c::atan(v[0], v[1]);
    mo.rotation[1] = -c::atan((v[0] * v[0] + v[1] * v[1]).sqrt(), v[2]);
    w.build_matrix(m);
}

/// Level10 `0x2e4a88` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, LEN);
    crate::moby_update::manip::big_head(w, 2.5, id, 2, id, pvo::HEAD);
    if c::pi32(w, id, pvo::RISE) == -1 {
        w.delete_moby(id);
        return;
    }
    hits(w, id);
    let t = tmoby(w, id).unwrap_or(id);
    let tpos = w.m(t).position;
    match w.m(id).state {
        0 => {
            w.mm(id).mode |= 0x1000;
            c::set_pf(w, id, pvo::RECORD, 9.0);
            c::set_pi16(w, id, pvo::RECORD + 4, 9);
            w.mm(id).pvars[pvo::RECORD + 8] = 3;
            w.mm(id).pvars[pvo::RECORD + 9] = 0;
            c::set_pf(w, id, pvo::PITCH_V, 0.0);
            w.mm(id).draw_dist = 0x7f;
            if let Some(&p0) = path(w, id, pvo::RISE).first() { c::set_pos(w, id, p0); }
            let t196 = w.ticks(T196) as i16;
            c::set_pi16(w, id, pvo::T196, t196);
            w.mm(id).has_collision = false;
            w.mm(id).mode |= 0x41;
            if w.m(id).anim.seq_b != 0 { w.anim_blend(id, 0, 0, 0); }
            w.mm(id).state = 1;
            if story::mission_done(w, w.m(id).mission as i32) {
                w.delete_moby(id);
                return;
            }
        }
        1 => {
            if w.in_cuboid([tpos[0], tpos[1], tpos[2]], c::pi32(w, id, pvo::WAKE)) {
                w.mm(id).state = 2;
                if w.m(id).anim.seq_b != 1 { blend(w, id, 1, 0x14); }
                w.mm(id).mode &= !0x41;
                w.mm(id).has_collision = super::class_collision(w, 1229);
                if w.svc.help.idle() && w.body() == 0 && !w.inventory.owned(11) { w.svc.help.request(0x2715, 0x5c); }
            }
        }
        2 => {
            let max = DT * PI;
            face(w, id, tpos, tpos, max);
            if w.m(id).anim.flags & 2 != 0 && w.m(id).anim.seq_b == 1 { blend(w, id, 2, 0x14); }
            w.mm(id).position[2] += DT * 4.0;
            let p0 = path(w, id, pvo::RISE).first().copied().unwrap_or([0.0; 4]);
            if c::dist3(c::pos(w, id), p0) < 0.5 {
                w.mm(id).state = 3;
                if w.m(id).anim.seq_b != 2 { blend(w, id, 2, 10); }
            }
        }
        3 => {
            let pts = path(w, id, pvo::RISE);
            let node = c::pi32(w, id, pvo::NODE).max(0) as usize;
            let q = pts.get(node).copied().unwrap_or(tpos);
            face(w, id, q, tpos, DT * 4.0 * PI);
            let p = c::pos(w, id);
            let d = c::sub(q, p);
            let v = c::set_len3(d, DT * 8.0);
            c::set_pv4(w, id, pvo::VEL, v);
            c::set_pos(w, id, c::add(p, v));
            if c::len3(d) < 0.5 {
                if pts.len() <= node + 1 {
                    c::set_pi32(w, id, pvo::NODE, 0);
                    w.mm(id).state = 4;
                    set_timer(w, id, pvo::ATTACK);
                } else {
                    c::set_pi32(w, id, pvo::NODE, node as i32 + 1);
                }
            }
        }
        s @ (4 | 5) => {
            if w.in_cuboid(w.hero_point(), c::pi32(w, id, pvo::FLEE)) {
                w.mm(id).state = 6;
                return shadow(w, id);
            }
            let (a, reaches) = aim(w, id, t);
            face(w, id, tpos, a, DT * 4.0 * PI);
            if s == 4 {
                let p = c::pos(w, id);
                if c::diff_rots(w.m(id).rotation[2], c::atan(tpos[0] - p[0], tpos[1] - p[1])) < DEG10 {
                    if w.m(id).anim.seq_b != 3 { blend(w, id, 3, 6); }
                    w.mm(id).anim.speed = ANIM_SPEED;
                }
                if w.m(id).anim.seq_a == 3 { fire(w, id, t, a, reaches); }
                if c::dec_timer_pvar_i32(w, id, pvo::TIMER) != 0 {
                    if w.m(id).anim.seq_b != 2 { blend(w, id, 2, 10); }
                    w.mm(id).state = 5;
                    set_timer(w, id, pvo::REST);
                }
            } else if c::dec_timer_pvar_i32(w, id, pvo::TIMER) != 0 {
                blend(w, id, 3, 10);
                w.mm(id).state = 4;
                w.mm(id).anim.speed = ANIM_SPEED;
                set_timer(w, id, pvo::ATTACK);
            }
            patrol(w, id);
        }
        6 => {
            let z = w.m(id).position[2] + DT * 10.0;
            let s = w.m(id).scale * (f32::from_bits(0xbc7a_9f00) + 1.0);
            w.mm(id).position[2] = z;
            w.mm(id).scale = s;
            if 250.0 < z || s < 0.01 {
                w.delete_moby(id);
                return;
            }
        }
        7 => {
            if w.m(id).anim.seq_a == 4 && w.m(id).anim.flags & 2 != 0 {
                match w.m(id).cmd {
                    4 => {
                        blend(w, id, 3, 10);
                        w.mm(id).state = 4;
                        w.mm(id).anim.speed = ANIM_SPEED;
                        set_timer(w, id, pvo::ATTACK);
                    }
                    6 => {
                        blend(w, id, 2, 10);
                        w.mm(id).state = 6;
                    }
                    _ => {
                        blend(w, id, 2, 10);
                        w.mm(id).state = 5;
                        set_timer(w, id, pvo::REST);
                    }
                }
            }
        }
        8 => {
            use crate::moby_update::creature::fx::{beam_explosion, Beam};
            const B: Beam = Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 5, sparks: 2, puffs: 4, debris: 0, sound: 5, shake: true };
            let p = c::pos(w, id);
            beam_explosion(w, &B, Some(id), p);
            crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
            let r = w.m(id).rotation;
            fx::break_piece(w, id, 0x5fd, p, r, 0, 0);
            fx::break_piece(w, id, 0x5fe, p, r, 0, 0);
            w.delete_moby(id);
            return;
        }
        _ => {}
    }
    shadow(w, id);
}

fn shadow(w: &mut World, id: MobyId) {
    if w.m(id).visible == 0 { return; }
    let cam = w.camera.map(|x| f32::from_bits(x.0));
    if c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 28.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0x16;
    }
}

/// The patrol along +0x144 at 8 a second (states 4 and 5).
fn patrol(w: &mut World, id: MobyId) {
    let pts = path(w, id, pvo::PATROL);
    let node = c::pi32(w, id, pvo::NODE).max(0) as usize;
    let Some(&q) = pts.get(node) else { return };
    let p = c::pos(w, id);
    let d = c::clamp_len3(c::sub(q, p), DT * 8.0);
    c::set_pos(w, id, c::add(p, d));
    if c::len3(d) < DT && node + 1 < pts.len() { c::set_pi32(w, id, pvo::NODE, node as i32 + 1); }
}

/// The shots of sequence 3 (module doc).
fn fire(w: &mut World, id: MobyId, t: MobyId, a: V, reaches: bool) {
    if c::pi32(w, id, pvo::KIND) == 2 || !reaches {
        if w.m(id).anim.seq_b != 2 { blend(w, id, 2, 10); }
        return;
    }
    let k1 = crate::moby_update::creature::ground::passed_frame(w, id, 1.0);
    let k16 = crate::moby_update::creature::ground::passed_frame(w, id, 16.0);
    if !k1 && !k16 { return; }
    let mut p = w.joint_point(id, if k16 { 0 } else { 1 });
    let r = w.m(id).rotation;
    let f = fx::polar(1.0, r[2], -r[1]);
    p = [p[0] + f[0], p[1] + f[1], p[2] + f[2], p[3]];
    let v = c::set_len3(c::sub(a, p), SHOT_SPEED * DT);
    let k = 3.0 * crate::moby_update::services::fl(w.svc.timing.timer_scale);
    let flash_at = c::add(v.map(|x| x * k), p);
    muzzle_flash(w, id, flash_at);
    shot(w, 1.0, v, p, id, Some(t), 300);
}

/// Level10 `0x2cc2e8`: a shot (module doc).
pub fn shot_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x40);
    match w.m(id).state {
        1 => {
            let target = usize::try_from(c::pi32(w, id, 0x14) - 1).ok().filter(|&m| m < w.table.mobys.len());
            if target.is_none_or(|t| matches!(w.m(t).state, 0xfe | 0xfd)) { w.mm(id).state = 2; }
            let mode = w.body();
            if let Some(t) = target.filter(|&t| (1..=2).contains(&mode) && Some(t) == w.hero_moby) { home(w, id, t, mode); }
            spark(w, id);
            let old = c::pos(w, id);
            let hp = w.hero_point();
            let d = c::dist3(old, [hp[0], hp[1], hp[2], 0.0]);
            let p = c::add(old, c::pv4(w, id, 0));
            c::set_pos(w, id, p);
            let prev = c::pf(w, id, 0x38);
            if c::pi32(w, id, 0x34) == 0 && c::pi32(w, id, 0x30) != 0 && prev < d && d < 5.0 {
                w.play_sound(0, 0, id);
                c::set_pi32(w, id, 0x34, 1);
            }
            if d < prev { c::set_pi32(w, id, 0x30, 1); }
            c::set_pf(w, id, 0x38, d);
            let owner = usize::try_from(c::pi32(w, id, 0x10) - 1).ok().filter(|&m| m < w.table.mobys.len());
            if let Some(h) = w.coll_line(pv(old), pv(p), 0x10, owner) {
                if h.moby != owner {
                    if let Some(m) = h.moby {
                        let mut dir = c::set_len3(c::pv4(w, id, 0), 1.0);
                        dir[2] = 1.0;
                        let at = [h.point[0], h.point[1], h.point[2], 0.0];
                        crate::moby_update::creature::attack::hit_moby(w, m, id, 1.0, 0x1_0003, at, dir);
                    }
                    w.mm(id).state = 2;
                }
            }
            if c::dec_timer_pvar_i32(w, id, 0x18) != 0 { w.mm(id).state = 2; }
        }
        2 => {
            let light = if w.svc.level == 10 { 0.0 } else { 13.0 };
            let p = c::pos(w, id);
            fx::death_explosion(w, 0.25, light, Some(id), p, -1);
            w.play_sound_as(0, 0, id, 0x99);
            w.delete_moby(id);
        }
        _ => {}
    }
}

/// The homing at Clank (module doc).
fn home(w: &mut World, id: MobyId, t: MobyId, mode: u8) {
    let v = c::pv4(w, id, 0);
    let speed = c::len3(v);
    let mut a = w.m(t).position;
    a[2] = f32::from_bits(w.ground_height(pf(0.5), pv(a), 0).0) + 0.2;
    if mode == 2 { a[2] += 2.0; }
    let hv = hero_disp(w);
    let hv = [hv[0], hv[1], 0.0];
    let p = c::pos(w, id);
    let d = c::sub(a, p);
    let qa = speed * speed - (hv[0] * hv[0] + hv[1] * hv[1] + hv[2] * hv[2]);
    let qb = (hv[0] * d[0] + hv[1] * d[1] + hv[2] * d[2]) * -2.0;
    let qc = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    let s = (qb * qb - qa * 4.0 * -qc).abs().sqrt();
    let t1 = (-qb + s) / (qa + qa);
    let t2 = (-qb - s) / (qa + qa);
    let tt = if t1 <= 0.0 || t2 <= 0.0 {
        if t1 <= 0.0 { if 0.0 < t2 { t2 } else { -1.0 } } else { t1 }
    } else if t1 <= t2 {
        t2
    } else {
        t1
    };
    let l = if 0.0 < tt { [hv[0] * tt + d[0], hv[1] * tt + d[1], hv[2] * tt + d[2]] } else { [d[0], d[1], d[2]] };
    let yaw = c::atan(l[0], l[1]);
    let pitch = c::atan((l[0] * l[0] + l[1] * l[1]).sqrt(), l[2]);
    let mut vy = c::pf(w, id, 0x28);
    let rz = spring_turn(w.m(id).rotation[2], yaw, DT2 * TAU, DT2 * PI, DT * TAU, &mut vy);
    c::set_pf(w, id, 0x28, vy);
    let mut vp = c::pf(w, id, 0x2c);
    let ry = spring_turn(w.m(id).rotation[1], -pitch, DT2 * TAU, DT2 * PI, DT * TAU, &mut vp);
    c::set_pf(w, id, 0x2c, vp);
    let rx = c::add_rot(w.m(id).rotation[0], DT * TAU);
    w.mm(id).rotation = [rx, ry, rz, w.m(id).rotation[3]];
    let nv = fx::polar(speed, rz, -ry);
    c::set_pv4(w, id, 0, [nv[0], nv[1], nv[2], 0.0]);
}

/// The shot's two sparks a tick (type 4).
fn spark(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    for (c1, c2, life, base, grow, add) in [(0x6f00_afffu32, 0xffu32, (0xf, 0x16), 0x28i16, (0x14, 0x23), true), (0x1fff_ffff, 0x4f_4f4f, (0x1e, 0x3c), 0x28, (0x32, 0x4b), false)] {
        let l = w.rng.rand_range(life.0, life.1);
        let l = w.ticks(l);
        let g = w.rng.rand_range(grow.0, grow.1);
        let s = crate::particles::type04::Spawn { pos: p, vel: [0.0; 4], c1, c2, life: l, base, growth: g as i16, additive: add };
        fx::part04(w, &s);
    }
}
