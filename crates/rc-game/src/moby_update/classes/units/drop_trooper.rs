//! **The hover troopers, class 638** (level18 `0x2dc918` with its prologue `0x2dd4d8` and helpers `0x2dd9e8`,
//! `0x2dda08`, `0x2ddae0`, `0x2ddba0`, `0x2ddc98`, `0x2dde20`, `0x2de1b8`, `0x2de530`; census U504; 46 placed on 18,
//! level16 `0x2d2cf0` is the same code with 18's level checks failing) **and their shots, class 49** (`0x2a76e0`, its
//! spawner `0x2a7a98`, trail `0x2a78c8` and impact `0x2a7a38`). A trooper hovers about an anchor point with a slow
//! three-axis wobble, humming, its two jets puffing and its head, body and gun (three look-at records) turned toward
//! its target. Without a target it drifts between two points of its patrol path; with one it either moves to the other
//! point first or squares up and fires bursts of three shots (animation keys 2, 17 and 32), then waits a second. A
//! trooper marked as carried (+0x128) starts hidden on a dropship 1356 (`units::dropship`) until it is dropped. A hit
//! knocks it back (it backs off until the animation ends) and alerts its group for 5..6 seconds (their range becomes
//! 255); 3 damage sends it flying (a death flight that ends in sparks and flashes). On level 18 ten kills made without
//! any non-0x47 hit landing on them earn skill point 0x1d. In the finale's scenes 3..5 every trooper is removed. Read
//! from the level18 decomp and data (gp−0x4f24..−0x4f1c = 1, 1, 1 on 18 and 16 alike: the wobble, the looks and the
//! jets are always on; 0x1f2710.. the death sparks' colours). Native `f32`.
//!
//! **Pvars** (0x30c): +0x20 the damage record (health 3.0, +0x24 3, +0x28 2, +0x29 1, +0x30 1.5; +0x38 "hit" from the
//! resolver), +0x60 the flash, +0x70 the flight record K, +0xd0 the target record (+0x110 the moby, +0x114 the kind),
//! +0x120 the patrol path, +0x124 the target range, +0x128 carried, +0x12c the target region path (−1 none), +0x130 /
//! +0x1b0 / +0x230 the look records (joint lists 1, 2, 3), +0x2b0 the drop point, +0x2c0 the anchor (carried: the seat
//! offset, w the yaw offset), +0x2d0 the carrier (the port: moby index + 1), +0x2d4 the speed, +0x2d8 the yaw
//! velocity, +0x2dc (s16) the wait, +0x2de (s16) the shots, +0x2e0 the alert, +0x2e4..+0x2fc the wobble rates and
//! phases, +0x2ec its amplitude, +0x300 (s16) the patrol point, +0x302 (s16) the idle time, +0x304 the hum's voice,
//! +0x308 "no foreign hit" (the skill point).
//!
//! ## Coverage
//! | address | what | port |
//! |---|---|---|
//! | `0x2dd4d8` | scale = the class's; `MobyGetHitMessage(0x330000)` minus hits by its own shots (0x31); `0x26f378(+0x20, 0, col 4)`: a hit (out ≠ 1) and state ≠ 9: the group alerted (`0x2ddba0(300, 360)`), an attacker not 0x47 clears +0x308, health −= damage (≤ 0 → reaction 1); K: radius 512, drag 0, gravity 20·dt², flags 1, z offset 0.5, +0x3d 0 | [`prologue`] |
//! | | reaction 1 / 2: on 18 with +0x308 the kill count gp−0x7df0 + 1, past 9 → skill point 0x1d (`award_skill_point`); not targetable; gravity 20·dt²; Giant Clank (0x1413f4 = 2): anim speed 0.5, speed 24·dt, up 8·dt, else 8·dt both; keys −1; the flight (`0x26fa48`, `0x271418(…, 7, 1, 0)`) away from the attacker, → 9, flash 0xf0, `SetDeathBits` | [`prologue`] (`knock::aim` / `start`) |
//! | | reactions 3..8: speeds 6·dt, keys −1, flight seq 6, → 8, flash 0x78; 9 / 10: flash 0xfa; then `0x272318` | [`prologue`] |
//! | | +0xa4 = 0xff, `0x2723f8`; the alert +0x2e0 counts down, a new hit (+0x38) restarts it at `randf(180, 240)`; range = 255 while alerted, else +0x124; Ratchet in group 0x16 → kind 2; else `0x274b78(24)` (no region or alerted) or `0x274df8(24, region +0x12c)` into +0xd0; farther (xy) than the range → kind 2; no moby → Ratchet | [`prologue`] (`target::acquire` / `acquire_in`) |
//! | `0x2dc918` head | the aim point: Ratchet (class 0) or Giant Clank (0x1a3) → 0x13f410, else the target moby's position; the big-head cheat 0x15edb7: `0x266108(2.5, +0x130)` | [`update`] (`manip::big_head_scale`) |
//! | state 0 | the record (3.0, 3, 2, 1, 1.5); three random phases and rates `randf(90°, 180°)·dt`; +0x302 = `randf(180, 360)`; mirror bit `randi(2)`; a patrol path: +0x300 = `trunc(randf(0, n − 1))`, placed there; +0x308 = 1; anchor = drop point = position; carried → 10, hidden, collision off, blend 1 / 20; else → 2, blend 0 / 10 | [`update`] |
//! | 1 (dropped) | wait +0x2dc running: steer to position + row 0·(wait·0.1); done: yaw to the drop point (`0x270cc0` 4π·dt², 4π·dt), steer there, blend 0 / 10, within 1 → 2 | [`update`], [`steer`] |
//! | 2 (idle) | wait −1; no target: the idle time runs out → (patrol) the other of points 0 / 1, → 3, 180 ticks, the turn animation (`0x2ddc98`); a target: the wait out → (patrol) → 7, wait `randf(30, 90)`, the other point, the turn animation; (no patrol) 1 in 20 → 4, blend 2 / 10 | [`update`] |
//! | 3 (patrol move) | steer to the point; there (< 0.01) or the idle time out → 2, blend 0 / 10, idle `randf(180, 360)`; slow (< 2·dt) and within 0.5: blend 0 / 30 | [`update`] |
//! | 4 (square up) | yaw to the aim point; the animation wrapped (+0x70 & 2) → 5, blend 3 / 10 | [`update`] |
//! | 5 (fire) | yaw to the aim point; key 2, 17 or 32 passed: a shot (`0x2a7a98`) from joint list 0 at 20·dt toward the aim point; key 10 passed: the third → 2, blend 0 / 10, wait 60 | [`update`] |
//! | 7 (move to fire) | steer to the point, wait −1; there or the wait out → 4, blend 2 / 30; slow and within 0.5: blend 2 / 30 | [`update`] |
//! | 8 (knocked) | until the animation wraps: backs off dt along row 0; then → 2, blend 0 / 10 | [`update`] |
//! | 9 (flight) | `0x271558(K)`; bits & 0x160 → the death effects (`0x2dde20`), class sound 0, deleted; else the anchor follows the move | [`update`] (`knock::update`) |
//! | 10 (carried) | a carrier: shown; position = carrier rows · offset + carrier position, yaw = carrier yaw + offset w | [`update`] |
//! | tail | (gp−0x4f24) +0x2bc = z; not 0 / 10: amplitude → 0.25 at dt/4, phases += rates, position = anchor + amplitude·sin(phase) per axis; within 32 of the camera: the hum (class sound 7, looped) | [`update`] |
//! | | (gp−0x4f20) the looks `0x2de1b8`; (gp−0x4f1c, Ratchet not in 0x16) the jets `0x2de530`; level 18, game mode 2, scene 3..5 → deleted | [`update`] |
//! | `0x2ddae0(m, p)` | d = \|p − anchor\|; `0x270830(d, 4·dt², 4·dt², 5·dt)` on the speed; anchor += unit(p − anchor)·speed; returns d | [`steer`] |
//! | `0x2ddba0(m, lo, hi)` | no group: +0x2e0 = `randf(lo, hi)`; else each live member's | [`alert`] |
//! | `0x2ddc98` | the angle to the patrol point off the yaw by 45° or more: turning the mirrored way → 4, else 5; within: 1 (blend / 20, unless playing) | [`turn_anim`] |
//! | `0x2dde20` | at joint list 2: n = trunc(d) + 2 (d < 8) or 10 sparks `PartType11Spawn(100000, randf(8, 10)·dt − (7 − d)⁺·dt, p, 0, c1[randi(6)], c2[randi(6)], rand_range(15, 20), rand_range(25, 30))`; frame load < 0.95 and d > 9: flashes (4, 15, 7f7f7f / 20) (4, 24, 7f2000 / 20); then (4, 20, 7f4000 / 30) (3.5, 27, 601000 / 40) (3, 29, 200000 / 20) | [`death_fx`] |
//! | `0x2de1b8` | a target (kind ≠ 2): the point (Ratchet's body 0x13f420, else the record's) − (position + 2 z) in the moby's frame; yaw / pitch clamped to ±45°, halves into the records +0x130 and +0x1b0 (+0x68 / +0x64); in 4 / 5 the gun record +0x230: from the gun point (position + 2 z + row 0 + row 1·0.5): pitch ±45°, yaw ±20°; the three records `0x2777d8(0.03, 0.3, lists 1, 2, 3)` | [`looks`] (`manip::look`) |
//! | `0x2de530` | drawn and not carried: joint lists 4 / 5, each 3 in 4 (`randf ≤ 0.75`): a type-2 puff down its row 2 at `(1 ± 0.2)·3·dt`, the second velocity `(1 ± 0.2)·−2·dt` in z plus `rand_vec(±1.5·dt)`, sizes 0.2 / 0.4, colours tweened 0x801010ff..0x801080ff / 0x400000c0..0x4030ffff, phases 5 / 20 / 20 (±10 %) | [`jets`] |
//! | `0x2a7a98` | `CreateMoby(0x31)`: distances 0xff, state 0, scale ·3, the owner's light; +0 v, +0x14 `ticks(240)`, +0x18 the owner, +0x10 0; yaw / pitch from v; hidden | [`spawn_shot`] |
//! | `0x2a76e0` | within 15 of the camera once: `PlayClassSoundByClass(9, 0, m, 0x27e)`; state 0: rot x += 2π·dt; the hit (1 damage; Giant Clank 1 in 5) flags 0x10003 along v, its class; `CollLine_Fix(position − 2v, position + v, 0x10, m)`: no hit and the timer running, or the owner hit: moves on, the trail; else sound 8, the impact flash (0.5, 15, 90c000 / 60), deleted | [`shot_update`] |
//! | `0x2a78c8` | a type-2 puff: velocities `rand_vec(0.25·dt)` / `rand_vec(2·dt)` (v ·0), sizes 0.25 / 0.125, phases 2 / 5..10 / 5..10, colours 0x8000c070 / 0x4000c0c0 | [`shot_trail`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::debris::flash_spawn;
use crate::moby_update::creature::{self as c, damage, flash, ground, knock, target, turn, DT, DT2};
use crate::moby_update::manip;
use crate::moby_update::scheduler::group_ids;
use crate::moby_update::services::{self as sv, pvar as p, HitTemplate, World};
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 18;
pub const UPDATE_FN: u32 = 0x2d_c918;
pub const CLASSES: [i16; 1] = [638];
pub const SHOT_FN: u32 = 0x2a_76e0;
pub const SHOT_CLASSES: [i16; 1] = [SHOT];

const SHOT: i16 = 0x31;
/// The class whose hits keep the skill point (0x47).
const KEEPS_SKILL: i16 = 0x47;
const SKILL: usize = 0x1d;
/// gp−0x7df0 (0x15e810 on 18): the kills toward the skill point.
const KILLS_WORD: u32 = 0x15_e810;
const SPARK_C1: [u32; 6] = [0x4f00_8fff, 0x4f00_8fff, 0x4f00_7fff, 0x4f00_6fff, 0x2fff_ffff, 0x2fff_ffff];
const SPARK_C2: [u32; 6] = [0x2f00_5f7f, 0x2f00_4f7f, 0x2f00_3f7f, 0x2f00_004f, 0x2f00_0000, 0x3f00_0000];

pub mod pv {
    pub const LEN: usize = 0x30c;
    pub const D: usize = 0x20;
    pub const HIT: usize = 0x38;
    pub const F: usize = 0x60;
    pub const K: usize = 0x70;
    pub const T: usize = 0xd0;
    pub const T_MOBY: usize = 0x110;
    pub const T_KIND: usize = 0x114;
    pub const PATROL: usize = 0x120;
    pub const RANGE: usize = 0x124;
    pub const CARRIED: usize = 0x128;
    pub const REGION: usize = 0x12c;
    pub const LOOKS: [usize; 3] = [0x130, 0x1b0, 0x230];
    pub const DROP: usize = 0x2b0;
    pub const ANCHOR: usize = 0x2c0;
    pub const CARRIER: usize = 0x2d0;
    pub const SPEED: usize = 0x2d4;
    pub const YAW_V: usize = 0x2d8;
    pub const WAIT: usize = 0x2dc;
    pub const SHOTS: usize = 0x2de;
    pub const ALERT: usize = 0x2e0;
    pub const RATE_Z: usize = 0x2e4;
    pub const PHASE_Z: usize = 0x2e8;
    pub const AMP: usize = 0x2ec;
    pub const RATE_X: usize = 0x2f0;
    pub const PHASE_X: usize = 0x2f4;
    pub const RATE_Y: usize = 0x2f8;
    pub const PHASE_Y: usize = 0x2fc;
    pub const POINT: usize = 0x300;
    pub const IDLE: usize = 0x302;
    pub const HUM: usize = 0x304;
    pub const CLEAN: usize = 0x308;
}

mod shot {
    pub const LEN: usize = 0x1c;
    pub const VEL: usize = 0x00;
    pub const HEARD: usize = 0x10;
    pub const TIMER: usize = 0x14;
    pub const OWNER: usize = 0x18;
}

fn gscale(w: &World, x: f32) -> i32 { w.svc.timing.scale(Pf::f(x)).to_f32() as i32 }
fn s16(w: &World, id: MobyId, o: usize) -> i16 { p::i16(&w.m(id).pvars, o) }
fn set_s16(w: &mut World, id: MobyId, o: usize, v: i16) { p::set_i16(&mut w.mm(id).pvars, o, v); }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn seq(w: &World, id: MobyId) -> u8 { w.m(id).anim.seq_b }

/// `fun_00212f90(m, s, 0, ticks(n))` unless sequence `s` plays.
fn blend(w: &mut World, id: MobyId, s: u8, n: i32) {
    if seq(w, id) == s { return; }
    let t = w.ticks(n);
    w.anim_blend(id, s, 0, t);
}

/// Patrol point `k` of the trooper's path.
fn patrol_point(w: &World, id: MobyId, k: i32) -> Option<c::V> {
    let path = usize::try_from(c::pi32(w, id, pv::PATROL)).ok()?;
    w.svc.volumes.paths.get(path)?.get(usize::try_from(k).ok()?).copied()
}

/// Level18 `0x2dc918` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    crate::moby_update::story::pvars(w, id, pv::LEN);
    if !prologue(w, id) { return; }
    let aim = match link(w, id, pv::T_MOBY) {
        Some(t) if w.m(t).o_class != 0 && w.m(t).o_class != 0x1a3 => w.m(t).position,
        _ => sv::fv(w.hero.shadow_point),
    };
    if w.svc.cheats.on(crate::cheats::slot::ENEMIES) { manip::big_head_scale(w, 2.5, id, pv::LOOKS[0]); }
    let st = w.m(id).state;
    let acc = DT2 * 12.566371;
    match st {
        0 => init(w, id),
        1 => {
            if c::dec_timer_pvar_s16(w, id, pv::WAIT) == 0 {
                let k = s16(w, id, pv::WAIT) as f32 * 0.1;
                let p = c::add(c::set_len3(w.m(id).rows[0], k), c::pos(w, id));
                steer(w, id, p);
            } else {
                let d = c::sub(c::pv4(w, id, pv::DROP), c::pos(w, id));
                turn_yaw(w, id, c::atan(d[0], d[1]), acc, DT * 12.566371);
                let drop = c::pv4(w, id, pv::DROP);
                let dist = steer(w, id, drop);
                blend(w, id, 0, 10);
                if dist < 1.0 { w.mm(id).state = 2; }
            }
        }
        2 => idle(w, id),
        3 => {
            let k = s16(w, id, pv::POINT) as i32;
            let Some(pt) = patrol_point(w, id, k) else { return tail(w, id) };
            let d = steer(w, id, pt);
            if d < 0.01 || c::dec_timer_pvar_s16(w, id, pv::IDLE) != 0 {
                w.mm(id).state = 2;
                blend(w, id, 0, 10);
                let r = w.rng.randf(180.0, 360.0);
                let t = gscale(w, r);
                set_s16(w, id, pv::IDLE, t as i16);
            }
            if c::pf(w, id, pv::SPEED) < DT + DT && d < 0.5 { blend(w, id, 0, 0x1e); }
        }
        4 => {
            let d = c::sub(aim, c::pos(w, id));
            turn_yaw(w, id, c::atan(d[0], d[1]), acc, DT * 12.566371);
            if w.m(id).anim.flags & 2 != 0 {
                w.mm(id).state = 5;
                blend(w, id, 3, 10);
            }
        }
        5 => {
            let d = c::sub(aim, c::pos(w, id));
            turn_yaw(w, id, c::atan(d[0], d[1]), acc, DT * 12.566371);
            if ground::passed_frame(w, id, 2.0) || ground::passed_frame(w, id, 17.0) || ground::passed_frame(w, id, 32.0) {
                let m = w.joint_point(id, 0);
                let v = c::set_len3(c::sub(aim, m), DT * 20.0);
                spawn_shot(w, id, v, m);
            } else if ground::passed_frame(w, id, 10.0) {
                let n = s16(w, id, pv::SHOTS) + 1;
                set_s16(w, id, pv::SHOTS, n);
                if 2 < n {
                    w.mm(id).state = 2;
                    blend(w, id, 0, 10);
                    let t = w.ticks(0x3c);
                    set_s16(w, id, pv::WAIT, t as i16);
                    set_s16(w, id, pv::SHOTS, 0);
                }
            }
        }
        7 => {
            let k = s16(w, id, pv::POINT) as i32;
            let Some(pt) = patrol_point(w, id, k) else { return tail(w, id) };
            let d = steer(w, id, pt);
            c::dec_timer_pvar_s16(w, id, pv::WAIT);
            if d < 0.01 || s16(w, id, pv::WAIT) == 0 {
                w.mm(id).state = 4;
                blend(w, id, 2, 0x1e);
            }
            if d < 0.5 && c::pf(w, id, pv::SPEED) < DT + DT { blend(w, id, 2, 0x1e); }
        }
        8 => {
            if w.m(id).anim.flags & 2 == 0 {
                let b = c::set_len3(w.m(id).rows[0], -DT);
                let p = c::add(c::pos(w, id), b);
                w.mm(id).position = p;
            } else {
                w.mm(id).state = 2;
                blend(w, id, 0, 10);
            }
        }
        9 => {
            let before = c::pos(w, id);
            let bits = knock::update(w, id, pv::K);
            if bits & 0x160 != 0 {
                death_fx(w, id);
                w.play_sound(0, 0, id);
                w.delete_moby(id);
                return;
            }
            let moved = c::sub(c::pos(w, id), before);
            let a = c::add(c::pv4(w, id, pv::ANCHOR), moved);
            c::set_pv4(w, id, pv::ANCHOR, a);
        }
        10 => {
            if let Some(k) = link(w, id, pv::CARRIER) {
                let (rows, cp, cyaw) = { let m = w.m(k); (m.rows, m.position, m.rotation[2]) };
                let o = c::pv4(w, id, pv::ANCHOR);
                let p: [f32; 4] = std::array::from_fn(|j| if j == 3 { cp[3] } else { o[0] * rows[0][j] + o[1] * rows[1][j] + o[2] * rows[2][j] + cp[j] });
                let m = w.mm(id);
                m.visible = 1;
                m.mode &= 0xfffe;
                m.position = p;
                m.rotation[2] = c::add_rot(cyaw, o[3]);
            }
        }
        _ => {}
    }
    tail(w, id);
}

/// State 0 (module table).
fn init(w: &mut World, id: MobyId) {
    {
        let pvs = &mut w.mm(id).pvars;
        p::set_ff(pvs, pv::AMP, 0.0);
        p::set_u8(pvs, pv::D + 9, 1);
        p::set_i16(pvs, pv::D + 4, 3);
        p::set_u8(pvs, pv::D + 8, 2);
        p::set_ff(pvs, pv::D + 0x10, 1.5);
        p::set_ff(pvs, pv::D, 3.0);
    }
    let a = w.rng.rand_angle();
    c::set_pf(w, id, pv::PHASE_X, a);
    let a = w.rng.rand_angle();
    c::set_pf(w, id, pv::PHASE_Y, a);
    let a = w.rng.rand_angle();
    c::set_pf(w, id, pv::PHASE_Z, a);
    let deg = 0.017453292;
    for o in [pv::RATE_X, pv::RATE_Y, pv::RATE_Z] {
        let r = w.rng.randf(90.0, 180.0);
        c::set_pf(w, id, o, r * deg * DT);
    }
    let r = w.rng.randf(180.0, 360.0);
    let t = gscale(w, r);
    set_s16(w, id, pv::IDLE, t as i16);
    let m = w.rng.randi(2);
    if m == 0 { w.mm(id).mode &= 0x7fff; } else { w.mm(id).mode |= 0x8000; }
    if let Some(n) = usize::try_from(c::pi32(w, id, pv::PATROL)).ok().and_then(|i| w.svc.volumes.paths.get(i)).map(|p| p.len()) {
        let k = w.rng.randf(0.0, (n as i32 - 1) as f32) as i32;
        set_s16(w, id, pv::POINT, k as i16);
        if let Some(pt) = patrol_point(w, id, k) { w.mm(id).position = pt; }
    }
    c::set_pi32(w, id, pv::CLEAN, 1);
    let pos = c::pos(w, id);
    c::set_pv4(w, id, pv::ANCHOR, pos);
    c::set_pv4(w, id, pv::DROP, pos);
    if c::pi32(w, id, pv::CARRIED) == 0 {
        w.mm(id).state = 2;
        blend(w, id, 0, 10);
        return;
    }
    let m = w.mm(id);
    m.state = 10;
    m.visible = 0;
    m.mode = (m.mode & 0xefff) | 1;
    m.has_collision = false;
    blend(w, id, 1, 0x14);
}

/// State 2 (module table).
fn idle(w: &mut World, id: MobyId) {
    c::dec_timer_pvar_s16(w, id, pv::WAIT);
    let patrol = 0 <= c::pi32(w, id, pv::PATROL);
    if c::pi32(w, id, pv::T_KIND) == 2 {
        if c::dec_timer_pvar_s16(w, id, pv::IDLE) == 0 || !patrol { return; }
        next_point(w, id);
        w.mm(id).state = 3;
        let t = w.ticks(0xb4);
        set_s16(w, id, pv::IDLE, t as i16);
        turn_anim(w, id);
        return;
    }
    if s16(w, id, pv::WAIT) != 0 { return; }
    if patrol {
        w.mm(id).state = 7;
        let r = w.rng.randf(30.0, 90.0);
        let t = gscale(w, r);
        set_s16(w, id, pv::WAIT, t as i16);
        next_point(w, id);
        turn_anim(w, id);
        return;
    }
    if w.rng.randi(0x14) != 0 { return; }
    w.mm(id).state = 4;
    blend(w, id, 2, 10);
}

/// `(point + 1) % 2`: the trooper only ever uses points 0 and 1 of its path.
fn next_point(w: &mut World, id: MobyId) {
    let k = s16(w, id, pv::POINT) as i32 + 1;
    set_s16(w, id, pv::POINT, (k - (k / 2) * 2) as i16);
}

/// `0x270cc0(t, acc, acc, vmax, &yaw, &+0x2d8)`.
fn turn_yaw(w: &mut World, id: MobyId, t: f32, acc: f32, vmax: f32) {
    let (mut a, mut v) = (w.m(id).rotation[2], c::pf(w, id, pv::YAW_V));
    turn::turn_toward(t, acc, acc, vmax, &mut a, &mut v);
    w.mm(id).rotation[2] = a;
    c::set_pf(w, id, pv::YAW_V, v);
}

/// `0x2ddae0(m, p)` (module table).
fn steer(w: &mut World, id: MobyId, to: c::V) -> f32 {
    let anchor = c::pv4(w, id, pv::ANCHOR);
    let d = c::dist3(anchor, to);
    let (mut x, mut v) = (0.0, c::pf(w, id, pv::SPEED));
    turn::spring(d, DT2 * 4.0, DT2 * 4.0, DT * 5.0, &mut x, &mut v);
    c::set_pf(w, id, pv::SPEED, v);
    let step = c::set_len3(c::sub(to, anchor), v);
    c::set_pv4(w, id, pv::ANCHOR, c::add(anchor, step));
    d
}

/// `0x2ddba0(m, lo, hi)` (module table).
fn alert(w: &mut World, id: MobyId, lo: f32, hi: f32) {
    let g = w.m(id).group;
    if g as u8 == 0xff {
        let r = w.rng.randf(lo, hi);
        let t = gscale(w, r);
        c::set_pi32(w, id, pv::ALERT, t);
        return;
    }
    for k in group_ids(w, g) {
        if (w.m(k).state as i8) < 0 { continue; }
        let r = w.rng.randf(lo, hi);
        let t = gscale(w, r);
        if w.m(k).pvars.len() >= pv::LEN { c::set_pi32(w, k, pv::ALERT, t); }
    }
}

/// `0x2ddc98` (module table).
fn turn_anim(w: &mut World, id: MobyId) {
    let k = s16(w, id, pv::POINT) as i32;
    let Some(pt) = patrol_point(w, id, k) else { return };
    let me = c::pos(w, id);
    let a = c::atan(pt[0] - me[0], pt[1] - me[1]);
    let d = c::sub_rot(a, w.m(id).rotation[2]);
    let mirrored = w.m(id).mode & 0x8000 != 0;
    let s = if std::f32::consts::FRAC_PI_4 <= d || d <= -std::f32::consts::FRAC_PI_4 {
        if (d < 0.0) == mirrored { 4 } else { 5 }
    } else {
        1
    };
    blend(w, id, s, 0x14);
}

/// `0x2dd4d8` (module table); false when the trooper is gone.
fn prologue(w: &mut World, id: MobyId) -> bool {
    let cs = w.classes.info(w.m(id).o_class).map_or(1.0, |i| i.scale);
    w.mm(id).scale = cs;
    let mut hit = w.get_hit(id, 0x33_0000, false);
    let attacker = hit.and_then(|h| h.attacker);
    if attacker.is_some_and(|a| w.m(a).o_class == SHOT) { hit = None; }
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    if res.out5 != 1 && w.m(id).state != 9 {
        alert(w, id, 300.0, 360.0);
        if let Some(a) = attacker.filter(|_| hit.is_some()) {
            if w.m(a).o_class != KEEPS_SKILL { c::set_pi32(w, id, pv::CLEAN, 0); }
        }
        let h = c::pf(w, id, pv::D) - res.damage;
        c::set_pf(w, id, pv::D, h);
        let reaction = if h <= 0.0 { 1 } else { res.reaction };
        {
            let k = pv::K;
            c::set_pi32(w, id, k + knock::k::RADIUS, 512);
            c::set_pf(w, id, k + knock::k::ZOFF, 0.5);
            c::set_pf(w, id, k + knock::k::DRAG, 0.0);
            c::set_pf(w, id, k + knock::k::GRAVITY, DT2 * 20.0);
            c::set_pi32(w, id, k + knock::k::FLAGS, 1);
            p::set_u8(&mut w.mm(id).pvars, k + 0x3d, 0);
        }
        let from = attacker.map(|a| w.m(a).position).unwrap_or(c::pos(w, id));
        let dir = hit.map(|h| sv::fv(h.dir)).unwrap_or([0.0; 4]);
        match reaction {
            1 | 2 => {
                if w.svc.level == 18 && c::pi32(w, id, pv::CLEAN) != 0 {
                    let n = w.svc.units.word(KILLS_WORD) + 1;
                    w.svc.units.set_word(KILLS_WORD, n);
                    if 9 < n && !crate::moby_update::story::skill_point(w, SKILL) {
                        crate::moby_update::story::award_skill_point(w, SKILL);
                    }
                }
                w.mm(id).mode &= 0xefff;
                c::set_pf(w, id, pv::K + knock::k::GRAVITY, DT2 * 20.0);
                if w.body() == 2 {
                    w.mm(id).anim.speed = 0.5;
                    c::set_pf(w, id, pv::K + knock::k::SPEED, DT * 24.0);
                    c::set_pf(w, id, pv::K + knock::k::UP, DT * 8.0);
                } else {
                    c::set_pf(w, id, pv::K + knock::k::UP, DT * 8.0);
                    c::set_pf(w, id, pv::K + knock::k::SPEED, DT * 8.0);
                }
                fly(w, id, from, dir, 7);
                w.mm(id).state = 9;
                p::set_u8(&mut w.mm(id).pvars, pv::F + 7, 0xf0);
                crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
            }
            3..=8 => {
                c::set_pf(w, id, pv::K + knock::k::UP, DT * 6.0);
                c::set_pf(w, id, pv::K + knock::k::SPEED, DT * 6.0);
                fly(w, id, from, dir, 6);
                w.mm(id).state = 8;
                p::set_u8(&mut w.mm(id).pvars, pv::F + 7, 0x78);
            }
            9 | 10 => p::set_u8(&mut w.mm(id).pvars, pv::F + 7, 0xfa),
            _ => {}
        }
        flash::start(w, id, pv::F);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pv::F);
    let mut alert_t = c::pi32(w, id, pv::ALERT);
    c::dec_timer_i32(&mut alert_t);
    if c::pi32(w, id, pv::HIT) != 0 {
        let r = w.rng.randf(180.0, 240.0);
        alert_t = gscale(w, r);
    }
    c::set_pi32(w, id, pv::ALERT, alert_t);
    c::set_pi32(w, id, pv::HIT, 0);
    let range = if alert_t == 0 { c::pf(w, id, pv::RANGE) } else { 255.0 };
    if w.hero.group == 0x16 {
        c::set_pi32(w, id, pv::T_KIND, 2);
    } else {
        let region = c::pi32(w, id, pv::REGION);
        let t = if region < 0 || alert_t != 0 { target::acquire(w, id, 24.0) } else { target::acquire_in(w, id, 24.0, Some(region as usize)) };
        store_target(w, id, &t);
    }
    if range < c::dist2(c::pos(w, id), c::pv4(w, id, pv::T)) { c::set_pi32(w, id, pv::T_KIND, 2); }
    if c::pi32(w, id, pv::T_MOBY) == 0 {
        let h = w.hero_moby.map_or(0, |h| h as i32 + 1);
        c::set_pi32(w, id, pv::T_MOBY, h);
    }
    true
}

/// The target record at +0xd0 (`0x274b78`'s out: point, moby, kind).
fn store_target(w: &mut World, id: MobyId, t: &target::Target) {
    c::set_pv4(w, id, pv::T, t.pos);
    c::set_pi32(w, id, pv::T_MOBY, t.moby.map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, pv::T_KIND, t.kind as i32);
}

/// The flight of a hit: keys −1, the heading away from `from` turned by the push (`0x26fa48`), `0x271418(…, s, 1, 0)`.
fn fly(w: &mut World, id: MobyId, from: c::V, dir: c::V, s: u8) {
    c::set_pf(w, id, pv::K + knock::k::KEY_LAND, -1.0);
    c::set_pf(w, id, pv::K + knock::k::KEY_APEX, -1.0);
    let me = c::pos(w, id);
    let mut angle = c::atan(me[0] - from[0], me[1] - from[1]);
    let (mut sp, mut up) = (c::pf(w, id, pv::K + knock::k::SPEED), c::pf(w, id, pv::K + knock::k::UP));
    if dir != [0.0; 4] { angle = knock::aim(dir, &mut sp, &mut up); }
    c::set_pf(w, id, pv::K + knock::k::SPEED, sp);
    c::set_pf(w, id, pv::K + knock::k::UP, up);
    knock::start(w, id, pv::K, angle, s, 1, 0);
}

/// The tail of `0x2dc918` (module table).
fn tail(w: &mut World, id: MobyId) {
    let z = w.m(id).position[2];
    c::set_pf(w, id, pv::DROP + 0xc, z);
    let st = w.m(id).state;
    if st != 0 && st != 10 {
        let mut a = c::pf(w, id, pv::AMP);
        turn::approach(0.25, DT * 0.25, &mut a);
        c::set_pf(w, id, pv::AMP, a);
        let mut s = [0.0; 3];
        for (k, (ph, rate)) in [(pv::PHASE_X, pv::RATE_X), (pv::PHASE_Y, pv::RATE_Y), (pv::PHASE_Z, pv::RATE_Z)].into_iter().enumerate() {
            let v = c::add_rot(c::pf(w, id, ph), c::pf(w, id, rate));
            c::set_pf(w, id, ph, v);
            s[k] = v.sin();
        }
        let o = c::pv4(w, id, pv::ANCHOR);
        let m = w.mm(id);
        for k in 0..3 { m.position[k] = o[k] + a * s[k]; }
    }
    let cam = crate::hero::physics::to_f32x3(w.camera);
    let me = c::pos(w, id);
    if c::dist3(me, [cam[0], cam[1], cam[2], 0.0]) < 32.0 {
        let slot = c::pi32(w, id, pv::HUM);
        if !w.sound_alive(slot, id) {
            let s = w.play_sound(7, 4, id);
            c::set_pi32(w, id, pv::HUM, s);
        }
    }
    looks(w, id);
    if w.hero.group != 0x16 { jets(w, id); }
    if w.svc.level == 18 && w.svc.game_mode == 2 && w.svc.cinematic.scene.as_ref().is_some_and(|s| (3..=5).contains(&s.id)) {
        w.delete_moby(id);
    }
}

/// `0x2de1b8` (module table).
fn looks(w: &mut World, id: MobyId) {
    let k = 0.03;
    if c::pi32(w, id, pv::T_KIND) != 2 {
        let me = c::pos(w, id);
        let base = [me[0], me[1], me[2] + 2.0, me[3]];
        let target_is_hero = link(w, id, pv::T_MOBY).is_some_and(|t| w.m(t).o_class == 0);
        let pt = if target_is_hero { sv::fv(w.hero.body_point) } else { c::pv4(w, id, pv::T) };
        let r = w.m(id).rows;
        let local = |v: c::V| [0, 1, 2].map(|i| v[0] * r[i][0] + v[1] * r[i][1] + v[2] * r[i][2]);
        let d = local(c::sub(pt, base));
        let yaw = c::atan(d[0], d[1]).clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
        let pitch = (-c::atan((d[0] * d[0] + d[1] * d[1]).sqrt(), d[2])).clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
        for rec in [pv::LOOKS[1], pv::LOOKS[0]] {
            c::set_pf(w, id, rec + 0x68, yaw * 0.5);
            c::set_pf(w, id, rec + 0x64, pitch * 0.5);
        }
        if matches!(w.m(id).state, 4 | 5) {
            let a = c::set_len3(r[0], 1.0);
            let b = c::set_len3(r[1], 0.5);
            let gun = c::add(c::add(a, base), b);
            let d = local(c::sub(pt, gun));
            let pitch = (-c::atan((d[0] * d[0] + d[1] * d[1]).sqrt(), d[2])).clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
            let yaw = (-c::atan(d[0], d[1])).clamp(-0.34906584, 0.34906584);
            c::set_pf(w, id, pv::LOOKS[2] + 0x64, pitch);
            c::set_pf(w, id, pv::LOOKS[2] + 0x68, yaw);
        }
    }
    for (rec, list) in pv::LOOKS.into_iter().zip([1u8, 2, 3]) { manip::look(w, id, id, rec, list, k, 0.3); }
}

/// `0x2de530` (module table).
fn jets(w: &mut World, id: MobyId) {
    if w.m(id).visible == 0 || w.m(id).state == 10 { return; }
    for i in 0..2 {
        if 0.75 < w.rng.randf(0.0, 1.0) { continue; }
        let m = w.joint_matrix(id, i + 4);
        let r = w.rng.randf(-0.2, 0.2);
        let mut v1 = c::set_len3(m[2], (r + 1.0) * -3.0 * DT);
        let r = w.rng.randf(-0.2, 0.2);
        let mut v2 = [0.0, 0.0, (r + 1.0) * -2.0 * DT, 0.0];
        let r = w.rng.randf(-1.5, 1.5);
        let j = w.rng.rand_vec(r * DT, r * DT);
        v2 = c::add(v2, [j[0], j[1], j[2], 0.0]);
        v1[3] = w.rng.randf(0.2, 0.2);
        v2[3] = w.rng.randf(0.4, 0.4);
        let f = w.rng.randf(0.0, 1.0);
        let c1 = crate::particles::tween_color(f.to_bits(), 0x8010_10ff, 0x8010_80ff);
        let f = w.rng.randf(0.0, 1.0);
        let c2 = crate::particles::tween_color(f.to_bits(), 0x4000_00c0, 0x4030_ffff);
        let t: [i32; 3] = std::array::from_fn(|k| { let r = w.rng.randf(-0.1, 0.1); gscale(w, (r + 1.0) * [5.0, 20.0, 20.0][k]) });
        let a = crate::particles::type02::Spawn { pos: m[3], v1, v2, c1, c2, t, def: -1 };
        crate::moby_update::creature::fx::part02(w, &a);
    }
}

/// `0x2dde20` (module table).
fn death_fx(w: &mut World, id: MobyId) {
    let p = w.joint_point(id, 2);
    let cam = crate::hero::physics::to_f32x3(w.camera);
    let d = c::len3(c::sub([cam[0], cam[1], cam[2], 0.0], p));
    let n = if d < 8.0 { d as i32 + 2 } else { 10 };
    let extra = if d < 7.0 { 7.0 - d } else { 0.0 };
    for _ in 0..n {
        let s = w.rng.randf(8.0, 10.0) * DT;
        let i1 = w.rng.randi(6) as usize;
        let i2 = w.rng.randi(6) as usize;
        let (a, b) = (w.ticks(0xf), w.ticks(0x14));
        let life = w.rng.rand_range(a, b);
        let (a, b) = (w.ticks(0x19), w.ticks(0x1e));
        let t1 = w.rng.rand_range(a, b);
        w.part11(Pf::f(100000.0), Pf::f(s - extra * DT), sv::pv(p), sv::pv([0.0; 4]), SPARK_C1[i1], SPARK_C2[i2], life, t1, 0, 0);
    }
    let load = f32::from_bits(w.svc.frame_load[0].0);
    let z = [0.0; 4];
    if load < 0.95 && 9.0 < d {
        let t = w.ticks(0xf);
        flash_spawn(w, 4.0, id, p, z, t, 0x7f, 0x7f, 0x7f, 0x20);
        let t = w.ticks(0x18);
        flash_spawn(w, 4.0, id, p, z, t, 0x7f, 0x20, 0, 0x20);
    }
    let t = w.ticks(0x14);
    flash_spawn(w, 4.0, id, p, z, t, 0x7f, 0x40, 0, 0x30);
    let t = w.ticks(0x1b);
    flash_spawn(w, 3.5, id, p, z, t, 0x60, 0x10, 0, 0x40);
    let t = w.ticks(0x1d);
    flash_spawn(w, 3.0, id, p, z, t, 0x20, 0, 0, 0x20);
}

/// `0x2dd9e8(m, carrier, offset, rot)`: seated on `carrier` at `offset`, its yaw offset `yaw`.
pub fn seat(w: &mut World, id: MobyId, carrier: MobyId, offset: [f32; 3], yaw: f32) {
    if w.m(id).pvars.len() < pv::LEN { crate::moby_update::story::pvars(w, id, pv::LEN); }
    c::set_pi32(w, id, pv::CARRIER, carrier as i32 + 1);
    c::set_pv4(w, id, pv::ANCHOR, [offset[0], offset[1], offset[2], yaw]);
}

/// `0x2dda08(m, wait)`: dropped from the carrier (only while carried).
pub fn drop(w: &mut World, id: MobyId, wait: i32) {
    if w.m(id).state != 10 { return; }
    w.mm(id).state = 1;
    blend(w, id, 1, 0x14);
    let has = w.classes.info(w.m(id).o_class).is_some_and(|i| i.has_collision);
    {
        let m = w.mm(id);
        m.visible = 1;
        m.mode = (m.mode & 0xfffe) | mode::TARGETABLE;
        m.has_collision = has;
    }
    set_s16(w, id, pv::WAIT, wait as i16);
    let pos = c::pos(w, id);
    c::set_pf(w, id, pv::DROP + 0xc, pos[2]);
    c::set_pv4(w, id, pv::ANCHOR, pos);
    c::set_pf(w, id, pv::SPEED, DT * 5.0);
}

/// `0x2a7a98(m, v, p)` (module table).
pub fn spawn_shot(w: &mut World, owner: MobyId, v: c::V, p: c::V) -> Option<MobyId> {
    let s = w.create_moby(SHOT)?;
    let (light, ambient) = (w.m(owner).light, w.m(owner).ambient);
    let life = w.ticks(0xf0);
    let m = w.mm(s);
    m.update_dist = 0xff;
    m.draw_dist = 0xff;
    m.state = 0;
    m.scale *= 3.0;
    m.light = light;
    m.ambient = ambient;
    m.position = p;
    if m.pvars.len() < shot::LEN { m.pvars.resize(shot::LEN, 0); }
    p::set_v4f(&mut m.pvars, shot::VEL, v);
    p::set_i32(&mut m.pvars, shot::TIMER, life);
    p::set_i32(&mut m.pvars, shot::OWNER, owner as i32 + 1);
    p::set_i32(&mut m.pvars, shot::HEARD, 0);
    m.rotation[2] = c::atan(v[0], v[1]);
    m.visible = 0;
    m.rotation[1] = -c::atan((v[0] * v[0] + v[1] * v[1]).sqrt(), v[2]);
    m.mode |= 1;
    Some(s)
}

/// Level18 `0x2a76e0` (module table).
pub fn shot_update(w: &mut World, id: MobyId) {
    crate::moby_update::story::pvars(w, id, shot::LEN);
    if c::pi32(w, id, shot::HEARD) == 0 {
        let cam = crate::hero::physics::to_f32x3(w.camera);
        if c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 15.0 {
            w.play_sound_as(9, 0, id, 0x27e);
            c::set_pi32(w, id, shot::HEARD, 1);
        }
    }
    if w.m(id).state != 0 { return; }
    let r = c::add_rot(w.m(id).rotation[0], DT * std::f32::consts::TAU);
    w.mm(id).rotation[0] = r;
    let me = c::pos(w, id);
    let v = p::v4f(&w.m(id).pvars, shot::VEL);
    let next = c::add(me, v);
    let back = c::add(c::scale(v, -2.0), me);
    let mut dmg = 1.0;
    if w.body() == 2 && w.rng.randi(5) != 0 { dmg = 0.0; }
    let tmpl = HitTemplate { dir: sv::pv(v), attacker: Some(id), flags: 0x1_0003, damage: Pf::f(dmg), w20: 1, h1a: w.m(id).o_class as u16, ..Default::default() };
    let owner = link(w, id, shot::OWNER);
    let hit = sv::line_hit_in(w.table, w.svc, w.classes, w.coll, sv::pv([back[0], back[1], back[2], 1.0]), sv::pv([next[0], next[1], next[2], 1.0]), 0x10, Some(id), &tmpl);
    let owner_hit = hit.as_ref().is_some_and(|h| h.moby.is_some() && h.moby == owner);
    let goes_on = (hit.is_none() && c::dec_timer_pvar_i32(w, id, shot::TIMER) == 0) || owner_hit;
    if goes_on {
        w.mm(id).position = next;
        shot_trail(w, id);
    } else {
        w.play_sound_as(8, 0, id, 0x27e);
        let t = w.ticks(0xf);
        flash_spawn(w, 0.5, id, me, [0.0; 4], t, 0x90, 0xc0, 0, 0x60);
        w.delete_moby(id);
    }
}

/// `0x2a78c8` (module table).
fn shot_trail(w: &mut World, id: MobyId) {
    let v = p::v4f(&w.m(id).pvars, shot::VEL);
    let j = w.rng.rand_vec(0.25 * DT, 0.25 * DT);
    let mut v1 = c::add(c::scale(v, 0.0), [j[0], j[1], j[2], 0.0]);
    let j = w.rng.rand_vec(2.0 * DT, 2.0 * DT);
    let mut v2 = c::add(c::scale(v, 0.0), [j[0], j[1], j[2], 0.0]);
    v1[3] = 0.25;
    v2[3] = 0.125;
    let t: [i32; 3] = std::array::from_fn(|k| { let (a, b) = [(2.0, 2.0), (5.0, 10.0), (5.0, 10.0)][k]; let r = w.rng.randf(a, b); gscale(w, r) });
    let a = crate::particles::type02::Spawn { pos: c::pos(w, id), v1, v2, c1: 0x8000_c070, c2: 0x4000_c0c0, t, def: -1 };
    crate::moby_update::creature::fx::part02(w, &a);
}
