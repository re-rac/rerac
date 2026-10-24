//! **The Morph-o-Ray's chicken** (class 270 = 0x10e, on every level: level01 update `0x2df448`, 5.6 KB with no Ghidra
//! function, read from the disassembly), **its feathers** (class 428 = 0x1ac: spawner `0x2e2cc0`, update `0x2e2f68`)
//! and **the morph** itself (`0x2defb0`, with the suck-record defaults `0x2def40`). docs/plan/hero_gameplay.md §12.
//!
//! **The morph is a spawn and a delete** (not a class swap, not a state): `CreateMoby(0x10e)` at the target's position
//! facing away from Ratchet (tilted to the ground under it when it stands at or below the ground), with the target's
//! lighting (its saved ambient when it has a hit-flash record), a random animation speed `randf(0.85, 1.15)`, its
//! class sound 1, state 1; then the target's `SetDeathBits` (its bolts) and `DeleteMoby` — unless its damage record's
//! +0x0e is 1, which becomes 2 (the class keeps its moby: a script-owned creature) — and a type-5 flash at the chicken
//! (the particle type is not ported: counted). The chickens live in a ring of 20 (`0x1dd580`, next `gp−0x537c`,
//! [`crate::moby_update::creature::react::Globals::chickens`]): the chicken already in the slot is **displaced**
//! (its +0x70 = 1). Each slot owns a knockback record (0x1de390 + 0x60·i) and a suck record (0x1dd5d0 + 0xb0·i, with
//! the chicken's sequence table gp−0x5390): the chicken gets them in its header (+0x10 / +0x14, mode 0x20 | 0x1000:
//! targetable, suckable by the Suck Cannon through its reaction table 0x2e0a28.., `react::CHICKEN`) unless it is gold,
//! or unless the displaced chicken of its slot is in the Suck Cannon right now (then it is born with 0 health and
//! bursts at once). **Gold** (the gold Morph-o-Ray, 0x13e535): 4 health, +0xbc = 1 (a decoy the enemies' target search
//! prefers, `creature::target`), no records, and it grows to 4× the class scale.
//!
//! **The update** (pvars below): Ratchet's heading away from it `f21` and the size `f22` = scale / class scale scale
//! every distance. Hits (`MobyGetHitMessage(m, −1)`, cooldown `ticks(25)`) take their damage off the health (+0x78)
//! with class sound 1. **Below 1 health it bursts**: 20·size type-22 puffs (normal) or `SpawnBeamExplosion` with the
//! colour shift and a damage-1 sphere of 3 on every moby around (gold), then 5–8 feathers (fewer under a heavy frame
//! load), class sound 1, and it hides (state 6, no collision, not targetable) — or, displaced, is deleted. Otherwise a
//! cluck (class sound 0 or 1, 3 : 1) every `rand_range(120, 300)` ticks, and the states: **1 peck** (sequences 0 / 1 at
//! random as each wraps; when its timer runs out and Ratchet is within 5 it steps away (3, or back to waiting 60–180
//! ticks when blocked); within 5 and more than 30° from facing away → 2), **2 turn** away (a spring turn; timer out
//! and Ratchet within 5 and the ground ahead level → 3; facing away within 5°, or Ratchet beyond 6 → 1), **3 run**
//! (beyond 6 → 1; within 3: faster (2 u/s unscaled) → 4; else 1·size u/s), **4 sprint** (2·size u/s; timer out and
//! beyond 4 → 3), **5 in the Suck Cannon** (`react::carried`; a feather 1 tick in 4; the let-go ends it in state 0,
//! which re-initialises it: health 1). Every move is the shared `walker::move_ground` (0x26d270: up 0.2·size,
//! radius 0.5·size, height step 0.333 (2·dt·size while displaced), slope 15°); a refused move is replaced by a step of
//! at most dt·size toward where it was going if the ground there is level; then gravity 9.8·dt², the snap onto the
//! ground within 0.25·size, **surface 0 under it kills it** (health 0), z = the ground line's hit and the pitch / roll
//! eased toward the ground's normal at 2π·dt; below z 0 it is deleted. A displaced chicken (+0x70 ≥ 0) is deleted as
//! soon as it is not drawn; under frame loads above 0.9 / 1.0 it may be deleted or burst at random.
//!
//! **Feathers** (428): 0.2..0.7 above the point, 3 ticks of their velocity ahead, the velocity kept in the pvars with
//! drag 0.97 a tick, gravity 15·dt² down to a floor `randf(−1.2, −0.4)·dt`, a yaw spin `−randf(200°, 550°)·dt`, a
//! rocking pitch `randf(200°, 370°)·dt` flipping at ±45°, 9× the class scale (18× gold); life `ticks(rand_range(168,
//! 288))`: fading out (alpha +0x23 → 0) in its last 30 ticks, deleted when it ends or is not drawn.
//!
//! **Pvars** (0x80 in the game; the port appends the slot's two global records): +0x00 header, +0x20 the big-head
//! manipulator record (the big-head cheat 0x15edb7: `manip::big_head`, 5.0 gold / 7.7), +0x64 s16 cluck timer, +0x66 s16 state timer, +0x68 turn
//! velocity, +0x6c fall speed, +0x70 displaced timer (−1: not displaced), +0x74 gold, +0x78 health, +0x7c hit
//! cooldown; +0x80 the knockback record ([`KNOCK`]), +0xe0 the suck record ([`SUCK`] = `react::CHICKEN.record`).
//! The last two belong to a **morph-created** chicken only: a level-placed one (Kalebo III's breakroom places 16) keeps
//! the game's 0x80-byte block ([`BLOCK`]), so it is never suckable and its update reads only the base fields.
//!
//! Native `f32`; the rand draws are the game's, at its points. [L]: a `GroundHeight` miss leaves the chicken's z as it
//! was (the game reads the collision output's stale point) and reads surface −1 (not 0: no death).

use crate::hero::physics::to_f32x3;
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::creature::{self as c, attack, fx, ground, react, walker};
use crate::moby_update::services::World;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

pub const UPDATE_FN: u32 = 0x2df448;
pub const CLASSES: [i16; 1] = [CLASS];
pub const CLASS: i16 = 0x10e;
/// The feathers.
pub const FEATHER_FN: u32 = 0x2e2f68;
pub const FEATHER_CLASSES: [i16; 1] = [FEATHER];
pub const FEATHER: i16 = 0x1ac;

/// The chicken's pvar block as the game's level data gives it (0x80 bytes). [`PVARS`] adds the ring slot's knockback
/// and suck records, which only the morph sets up; a level-placed chicken does without them.
pub const BLOCK: usize = 0x80;
/// The morph-created chicken's pvar block: the game's 0x80 bytes, then the knockback record (0x60) and the suck
/// record (0xb0) of its ring slot.
pub const PVARS: usize = 0x190;
pub const KNOCK: usize = 0x80;
pub const SUCK: usize = 0xe0;
const CLUCK: usize = 0x64;
const TIMER: usize = 0x66;
const TURN_V: usize = 0x68;
const VZ: usize = 0x6c;
const DISPLACED: usize = 0x70;
const GOLD: usize = 0x74;
const HEALTH: usize = 0x78;
const HIT_CD: usize = 0x7c;

/// The ring of chickens (`0x1dd580`, 20).
pub const RING: usize = 20;
/// 15°: the steepest ground a chicken walks (`0x26d270`'s slope, and the elevation of a push).
const SLOPE: f32 = f32::from_bits(0x3e86_0a92);
/// The feather's spawn (gp−0x5368 / −0x5364 / −0x536c / −0x5374 / −0x5370 / −0x5378: 0, 0.35, 0.25, 5, 9, 70°).
const FEATHER_R: f32 = 0.35;
const FEATHER_UP: f32 = 0.25;
const FEATHER_SPEED: [f32; 2] = [5.0, 9.0];
const FEATHER_PITCH: f32 = 70.0;
/// gp−0x5320 0.97 (drag), gp−0x5328 9 (scale), gp−0x5324 240 (life base).
const FEATHER_DRAG: f32 = 0.97;
const FEATHER_SCALE: f32 = 9.0;
const FEATHER_LIFE: i32 = 240;

fn hero_pos(w: &World) -> c::V {
    let p = to_f32x3(w.hero.pos);
    [p[0], p[1], p[2], 0.0]
}

/// `0x15ed6c` dt and `0x15ed70` dt² (NTSC).
fn dt(_w: &World) -> f32 { c::DT }
fn dt2(_w: &World) -> f32 { c::DT2 }

fn blend(w: &mut World, id: MobyId, seq: u8, n: i32) {
    let t = w.ticks(n);
    w.anim_blend(id, seq, 0, t);
}

/// The ground's normal under `p` (the `GroundHeight` line's raw normal: `CollOutput +0x40`) and the hit z.
fn ground_hit(w: &World, p: c::V) -> Option<([f32; 3], f32)> {
    let a = [p[0], p[1], p[2] + 0.5, p[3]];
    let b = [p[0], p[1], 0.01, p[3]];
    w.coll_line(crate::moby_update::services::pv(a), crate::moby_update::services::pv(b), 2, None).map(|o| (o.normal, o.point[2]))
}

/// The pitch and roll that stand a moby of yaw `yaw` on ground of normal `n` (the chicken's and the morph's shared
/// formula): `(−atan2(n.y·cos − n.x·sin, √(f² + n.z²)), atan2(f, n.z))` with `f = n.x·cos + n.y·sin`.
fn ground_tilt(n: [f32; 3], yaw: f32) -> (f32, f32) {
    let (cy, sy) = (yaw.cos(), yaw.sin());
    let f = n[0] * cy + n[1] * sy;
    let side = n[1] * cy - n[0] * sy;
    let l = (f * f + n[2] * n[2]).sqrt();
    (-c::atan(l, side), c::atan(n[2], f))
}

/// `0x2defb0(target)`: the morph (module doc). Returns the chicken.
pub fn morph(w: &mut World, target: MobyId, gold: u8) -> Option<MobyId> {
    let ch = w.create_moby(CLASS)?;
    {
        let m = w.mm(ch);
        m.update_dist = 0x7f;
        m.draw_dist = 0x7f;
        m.visible = 1;
        m.pvars.resize(PVARS, 0);
    }
    let h = c::header(w, target);
    match h.flash {
        Some(f) => {
            let a = [w.m(target).pvars[f + 4], w.m(target).pvars[f + 5], w.m(target).pvars[f + 6]];
            // FUN_002650d0: +0x3c..+0x3e = r, g, b and +0x3f = 0 (the light word +0x38 is the chicken's own).
            w.mm(ch).ambient = [a[0], a[1], a[2], 0];
        }
        None => {
            let (l, a) = (w.m(target).light, w.m(target).ambient);
            let m = w.mm(ch);
            m.light = l;
            m.ambient = a;
        }
    }
    let speed = w.rng.randf(0.85, 1.15);
    w.mm(ch).anim.speed = speed;
    let p = w.m(target).position;
    w.mm(ch).position = p;
    w.mm(ch).rotation = [0.0; 4];
    let hp = hero_pos(w);
    w.mm(ch).rotation[2] = c::atan(p[0] - hp[0], p[1] - hp[1]);
    let g = ground::ground(w, p, 0.5, 0);
    if p[2] <= g.z {
        if let Some((n, z)) = ground_hit(w, p) {
            w.mm(ch).position[2] = z;
            let (rx, ry) = ground_tilt(n, w.m(ch).rotation[2]);
            w.mm(ch).rotation[0] = rx;
            w.mm(ch).rotation[1] = ry;
        }
    }
    let coll = w.classes.info(CLASS).is_some_and(|i| i.has_collision);
    w.mm(ch).has_collision = coll;
    w.play_sound(1, 0, ch);
    w.mm(ch).state = 1;
    c::set_pi16(w, ch, TIMER, 0);
    let (a, b) = (w.ticks(120), w.ticks(300));
    let r = w.rng.rand_range(a, b);
    c::set_pi16(w, ch, CLUCK, r as i16);
    c::set_pi32(w, ch, DISPLACED, -1);
    c::set_pf(w, ch, HEALTH, gold as f32 * 3.0 + 1.0);
    c::set_pi32(w, ch, GOLD, gold as i32);
    w.mm(ch).cmd = gold;
    set_death_bits(w, target, 0, -1);
    let keep = h.damage.is_some_and(|d| w.m(target).pvars.get(d + 0x0e) == Some(&1));
    if keep {
        let d = h.damage.unwrap();
        w.mm(target).pvars[d + 0x0e] = 2;
    } else {
        w.delete_moby(target);
    }
    // PartType05Spawn(1e6, 0, chicken pos, 0x7f, 0x7f, 0x7f, ticks(15)): the morph's flash (`particles::type05`).
    let (at, t15) = (w.m(ch).position, w.ticks(15));
    fx::part05(w, 1e6, 0.0, at, [0x7f; 3], t15);
    for b in &mut w.mm(ch).pvars[..0x20] { *b = 0; }
    let g_next = w.svc.creatures.react.chicken_next;
    let old = w.svc.creatures.react.chickens[g_next];
    let old = (old > 0).then(|| (old - 1) as MobyId).filter(|&o| o < w.table.mobys.len() && w.m(o).o_class == CLASS && w.m(o).state != 0xfe && w.m(o).state != 0xfd);
    let mut records = gold == 0;
    if let Some(o) = old {
        if w.m(o).state == react::CHICKEN.held {
            c::set_pf(w, ch, HEALTH, 0.0);
            w.mm(ch).mode &= !c::PVAR_HEADER;
            records = false;
        }
        if w.m(o).pvars.len() >= PVARS { c::set_pi32(w, o, DISPLACED, 1); }
    }
    if records {
        c::set_pi32(w, ch, 0x10, KNOCK as i32);
        c::set_pi32(w, ch, 0x14, SUCK as i32);
        w.mm(ch).mode |= c::PVAR_HEADER | mode::TARGETABLE;
        suck_defaults(w, ch);
    }
    w.svc.creatures.react.chickens[g_next] = ch as u32 + 1;
    if gold != 0 { w.mm(ch).mode &= !c::PVAR_HEADER; }
    w.svc.creatures.react.chicken_next = (g_next + 1) % RING;
    w.build_matrix(ch);
    Some(ch)
}

/// `0x2def40(record)`: the suck record's defaults (+0x7c 0.05, +0x80 0.863, +0x84 4, +0x88 3, +0x8c 0.625, sounds
/// −1, +0x78 0, +0xa0..+0xac −1).
fn suck_defaults(w: &mut World, id: MobyId) {
    let r = SUCK;
    c::set_pi32(w, id, r + 0xac, -1);
    c::set_pf(w, id, r + 0x7c, f32::from_bits(0x3d4c_cccd));
    c::set_pf(w, id, r + 0x80, f32::from_bits(0x3f5c_ed91));
    c::set_pi32(w, id, r + 0x84, 4);
    c::set_pi32(w, id, r + 0x88, 3);
    c::set_pf(w, id, r + 0x8c, 0.625);
    c::set_pi16(w, id, r + 0x9e, -1);
    c::set_pf(w, id, r + 0x78, 0.0);
    c::set_pi16(w, id, r + 0x9c, -1);
    c::set_pi32(w, id, r + 0xa0, -1);
    c::set_pi32(w, id, r + 0xa4, -1);
    c::set_pi32(w, id, r + 0xa8, -1);
}

/// One feather's spawn point and velocity (the chicken's burst and its held state): `pos` + a random offset within
/// `0.35·size` + `0.25·size` up; velocity `randf(5, 9)·dt` at 70° in a random direction, then its z `randf(3.5,
/// 5.5)·dt`.
fn feather(w: &mut World, id: MobyId, pos: c::V, size: f32) {
    let r = w.rng.randf(0.0, FEATHER_R) * size;
    let a = w.rng.rand_angle();
    let b = w.rng.rand_angle();
    let mut p = fx::polar(r, a, b);
    p[2] += FEATHER_UP * size;
    let p = c::add(p, pos);
    let d = dt(w);
    let s = w.rng.randf(FEATHER_SPEED[0] * d, FEATHER_SPEED[1] * d);
    let a = w.rng.rand_angle();
    let mut v = fx::polar(s, a, FEATHER_PITCH * 0.017_453_292);
    v[2] = w.rng.randf(3.5, 5.5) * d;
    spawn_feather(w, id, p, v);
}

/// Level16 `0x2c44f0(chicken, pos)` (the level's chicken code; Kalebo's chicken pad 1923,
/// `units::kalebo_chicken_pad`): the chicken moved to `pos` (all four lanes); the sphere `coll_sphere(0.2, pos + 0.3z, 0,
/// chicken)` clear → health 1, sequence 2 (`MobyAnimBlend(m, 2, 0, 1)`) unless it is already on it, mode `| 0x1000 &
/// 0xffbe` (targetable, shown), state 4 (sprint), the class collision, state timer `ticks(180)`; blocked → three
/// feathers and class sound 1.
pub fn respawn_at(w: &mut World, id: MobyId, pos: c::V) {
    w.mm(id).position = pos;
    let probe = [pos[0], pos[1], pos[2] + 0.3, pos[3]];
    if w.coll_sphere(crate::moby_update::services::pv(probe), crate::moby_update::services::pf(0.2), 0, Some(id)).is_none() {
        c::set_pf(w, id, HEALTH, 1.0);
        if w.m(id).anim.seq_b != 2 { w.anim_blend(id, 2, 0, 1); }
        let coll = super::units::class_collision(w, w.m(id).o_class);
        let t = w.ticks(180);
        let m = w.mm(id);
        m.mode = (m.mode | 0x1000) & 0xffbe;
        m.state = 4;
        m.has_collision = coll;
        c::set_pi16(w, id, TIMER, t as i16);
    } else {
        for _ in 0..3 { feather(w, id, pos, 1.0); }
        w.play_sound(1, 0, id);
    }
}

/// Level16 `0x2c4710(chicken)`: the chicken is hidden after a burst (state 6), free to be sent again.
pub fn available(w: &World, id: MobyId) -> bool { w.m(id).state == 6 }

/// `0x2e2cc0(chicken, pos, vel)`: a feather (class 428; module doc).
pub fn spawn_feather(w: &mut World, _parent: MobyId, pos: c::V, vel: c::V) -> Option<MobyId> {
    let f = w.create_moby(FEATHER)?;
    {
        let m = w.mm(f);
        m.update_dist = 0x40;
        m.mode |= mode::TARGETABLE;
        m.draw_dist = 0x20;
        m.visible = 1;
        m.state = 0;
        m.cmd = 0;
        m.position = pos;
        if m.pvars.len() < 0x30 { m.pvars.resize(0x80, 0); }
    }
    let up = w.rng.randf(0.2, 0.7);
    let d = dt(w);
    {
        let m = w.mm(f);
        m.position[2] += up;
        m.position[0] += vel[0] * 3.0;
        m.position[1] += vel[1] * 3.0;
    }
    c::set_pv4(w, f, 0, vel);
    let ry = w.rng.randf_sym(0.174_532_92, f32::from_bits(0x3f06_0a92));
    w.mm(f).rotation[1] = ry;
    let rx = w.rng.randf(0.174_532_92, FRAC_PI_4);
    w.mm(f).rotation[0] = rx;
    let rate = w.rng.randf(200.0, 370.0);
    c::set_pi32(w, f, 0x24, -1);
    c::set_pf(w, f, 0x2c, rate * 0.017_453_292 * d);
    if w.rng.rand() & 1 != 0 {
        c::set_pi32(w, f, 0x24, 1);
        w.mm(f).rotation[1] = -w.m(f).rotation[1];
    }
    let yaw = w.rng.rand_angle();
    w.mm(f).rotation[2] = yaw;
    let spin = w.rng.randf(200.0, 550.0);
    c::set_pf(w, f, 0x28, -(spin * 0.017_453_292 * d));
    let a = w.rng.rand_angle();
    c::set_pf(w, f, 0x10, a);
    let b = w.rng.rand_angle();
    c::set_pf(w, f, 0x14, b);
    let lo = (FEATHER_LIFE as f32 * 0.7) as i32;
    let hi = (FEATHER_LIFE as f32 * 1.2) as i32;
    let n = w.rng.rand_range(lo, hi);
    let life = w.ticks(n);
    c::set_pi32(w, f, 0x1c, life);
    let mut s = w.m(f).scale * FEATHER_SCALE;
    w.mm(f).scale = s;
    if w.svc.creatures.react.gold_morph != 0 {
        s += s;
        w.mm(f).scale = s;
    }
    let lo = w.rng.randf(-1.2, -0.4);
    c::set_pf(w, f, 0x20, lo * d);
    w.build_matrix(f);
    // FUN_00272078: Ratchet's lighting (+0x38).
    if let Some(hm) = w.hero_moby { let (l, a) = (w.m(hm).light, w.m(hm).ambient); w.mm(f).light = l; w.mm(f).ambient = a; }
    Some(f)
}

/// `0x2e2f68`: a feather's update (module doc).
pub fn feather_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x30 { return; }
    let d2 = dt2(w);
    let mut v = c::pv4(w, id, 0);
    let p = c::add(c::pos(w, id), v);
    c::set_pos(w, id, p);
    v[2] -= d2 * 15.0;
    v[1] *= FEATHER_DRAG;
    v[0] *= FEATHER_DRAG;
    let floor = c::pf(w, id, 0x20);
    if v[2] < floor { v[2] = floor; }
    c::set_pv4(w, id, 0, v);
    let spin = c::pf(w, id, 0x28);
    let yaw = c::add_rot(w.m(id).rotation[2], spin);
    w.mm(id).rotation[2] = yaw;
    let sign = c::pi32(w, id, 0x24);
    let rate = c::pf(w, id, 0x2c);
    let pitch = c::add_rot(w.m(id).rotation[0], rate * sign as f32);
    w.mm(id).rotation[0] = pitch;
    if FRAC_PI_4 < pitch.abs() { c::set_pi32(w, id, 0x24, -sign); }
    let mut life = c::pi32(w, id, 0x1c);
    let r = c::dec_timer_i32(&mut life);
    c::set_pi32(w, id, 0x1c, life);
    if r == 0 && w.m(id).visible != 0 {
        if life < w.ticks(30) {
            let mut a = w.m(id).alpha;
            let run = if a == 0 { 1 } else { a -= 1; if a == 0 { 2 } else { 0 } };
            w.mm(id).alpha = a;
            if run != 0 { w.delete_moby(id); }
        }
    } else {
        w.delete_moby(id);
    }
}

/// The frame loads 0x15f5d0 / 0x15f5d4.
fn loads(w: &World) -> (f32, f32) { (w.svc.frame_load[0].to_f32(), w.svc.frame_load[1].to_f32()) }

/// `0x2df448`: the chicken's update (module doc).
#[allow(clippy::too_many_lines)]
pub fn update(w: &mut World, id: MobyId) {
    // A level-placed chicken (Kalebo's breakroom places 16) has the game's own 0x80-byte block: the ring slot's records
    // after it exist only on morph-created chickens (`0x2defb0`), and without them the chicken is simply never suckable.
    // Demanding [`PVARS`] here left those chickens frozen: no init (state stayed 0), so no peck, no cluck and no hit intake.
    if w.m(id).pvars.len() < BLOCK { return; }
    let hp = hero_pos(w);
    let pos0 = c::pos(w, id);
    let away = c::atan(pos0[0] - hp[0], pos0[1] - hp[1]);
    let speed0 = w.m(id).anim.speed;
    if w.m(id).state == 0 {
        w.mm(id).state = 1;
        c::set_pi16(w, id, TIMER, 0);
        c::set_pi32(w, id, DISPLACED, -1);
        let (a, b) = (w.ticks(120), w.ticks(300));
        let r = w.rng.rand_range(a, b);
        c::set_pi16(w, id, CLUCK, r as i16);
        c::set_pf(w, id, HEALTH, 1.0);
    }
    if w.m(id).state == 6 { return; }
    // 0x2df510: `0x278720(gold Morph-o-Ray 0x13e535 ? 5.0 : 7.7, m, *gp−0x5380 = 0, P+0x20)`, the big-head cheat (0x15edb7);
    // the gold byte is the one the morph copied into +0x74 [L: the game reads 0x13e535 each tick].
    let s = if c::pi32(w, id, GOLD) != 0 { 5.0 } else { f32::from_bits(0x40f6_6666) };
    crate::moby_update::manip::big_head(w, s, id, 0, id, 0x20);
    let mut here = pos0;
    // The displaced chicken (module doc).
    if 0 <= c::pi32(w, id, DISPLACED) {
        let r = c::dec_timer_pvar_i32(w, id, DISPLACED);
        if r != 0 && w.m(id).visible == 0 {
            w.delete_moby(id);
            return;
        }
        if 0 <= c::pi32(w, id, DISPLACED) {
            let (l0, l1) = loads(w);
            if (0.9 < l1 || 0.9 < l0) && w.m(id).visible == 0 && c::pi32(w, id, DISPLACED) != -1 {
                if w.rng.randi(0xc7) == 0 {
                    w.delete_moby(id);
                    return;
                }
                let t = w.rng.rand_range(0x14, 0x1e);
                c::set_pi32(w, id, DISPLACED, t);
            }
            let (l0, l1) = loads(w);
            if 1.0 < l1 || 1.0 < l0 {
                if w.m(id).visible == 0 {
                    if w.rng.randi(0x18) == 0 {
                        w.delete_moby(id);
                        return;
                    }
                } else if w.rng.randi(0x63) != 0 {
                    let t = w.rng.rand_range(0x14, 0x1e);
                    c::set_pi32(w, id, DISPLACED, t);
                } else {
                    c::set_pf(w, id, HEALTH, 0.0);
                }
            }
        }
    }
    let class_scale = w.class_scale(crate::moby_update::classes::chicken::CLASS).to_f32();
    let gold = c::pi32(w, id, GOLD);
    if 0 < gold {
        let s = w.m(id).scale;
        w.mm(id).scale = s + ((gold as f32 * 3.0 + 1.0) * class_scale - s) * 0.1;
    }
    let size = if class_scale != 0.0 { w.m(id).scale / class_scale } else { 1.0 };
    if c::dec_timer_pvar_i32(w, id, HIT_CD) != 0 {
        if let Some(hit) = w.get_hit(id, u32::MAX, false) {
            w.play_sound(1, 0, id);
            let h = c::pf(w, id, HEALTH) - hit.damage.to_f32();
            c::set_pf(w, id, HEALTH, h);
            let t = w.ticks(25);
            c::set_pi32(w, id, HIT_CD, t);
        }
    }
    w.mm(id).hit_slot = 0xff;
    if c::pf(w, id, HEALTH) < 1.0 {
        burst(w, id, size, here);
        return;
    }
    if c::dec_timer_pvar_s16(w, id, CLUCK) != 0 {
        let r = w.rng.randi(100);
        w.play_sound((r >= 0x4b) as i32, 0, id);
        let (a, b) = (w.ticks(120), w.ticks(300));
        let r = w.rng.rand_range(a, b);
        c::set_pi16(w, id, CLUCK, r as i16);
    }
    let d = dt(w);
    let dz = if 0 <= c::pi32(w, id, DISPLACED) { d + d } else { 0.0 };
    let max_dz = if 0 <= c::pi32(w, id, DISPLACED) { dz * size } else { 0.333 };
    let (up, radius) = (0.2 * size, 0.5 * size);
    let hero_d = |w: &World| c::dist2(c::pos(w, id), hp);
    let ahead = |w: &World, k: f32, here: c::V| -> c::V {
        let y = c::yaw(w, id);
        c::add([y.cos() * k, y.sin() * k, 0.0, 0.0], here)
    };
    let turn = |w: &mut World| {
        let (d2, d1) = (dt2(w), dt(w));
        crate::moby_update::creature::turn::spring_turn2_pvar(w, id, away, d2 * FRAC_PI_4, d2 * FRAC_PI_2, d1 * FRAC_PI_4, TURN_V);
    };
    // Did a refused move still go somewhere ahead? (the "blocked" tests after 0x26d270).
    let went_ahead = |w: &World, here: c::V| -> bool {
        let p = c::pos(w, id);
        let a = c::atan(here[0] - p[0], here[1] - p[1]);
        c::diff_rots(c::yaw(w, id), a) < FRAC_PI_2 && 0.01 * size < c::dist3(p, here)
    };
    let to_state1 = |w: &mut World| {
        w.mm(id).state = 1;
        let (a, b) = (w.ticks(60), w.ticks(180));
        let r = w.rng.rand_range(a, b);
        c::set_pi16(w, id, TIMER, r as i16);
        let s = (w.rng.randi(2) == 0) as u8;
        blend(w, id, s, 5);
    };
    let mut moved = 0;
    match w.m(id).state {
        1 => {
            if w.m(id).anim.flags & 2 != 0 {
                let s = (w.rng.randi(2) == 0) as u8;
                blend(w, id, s, 5);
            }
            let timer_out = c::dec_timer_pvar_s16(w, id, TIMER) != 0;
            if timer_out && hero_d(w) < 5.0 * size {
                let mut to = ahead(w, d * size, here);
                moved = walker::move_ground(w, id, up, radius, max_dz, SLOPE, &mut here, &mut to, 0);
                if moved != 0 || went_ahead(w, here) {
                    w.mm(id).state = 3;
                    blend(w, id, 3, 10);
                } else {
                    let (a, b) = (w.ticks(60), w.ticks(180));
                    let r = w.rng.rand_range(a, b);
                    c::set_pi16(w, id, TIMER, r as i16);
                }
            } else if hero_d(w) < 5.0 * size && f32::from_bits(0x3f06_0a92) < c::diff_rots(away, c::yaw(w, id)) {
                walker::settle(w, id, up, radius, max_dz, SLOPE, &mut here, 0);
                w.mm(id).state = 2;
                blend(w, id, 3, 10);
            } else {
                walker::settle(w, id, up, radius, max_dz, SLOPE, &mut here, 0);
            }
        }
        2 => {
            turn(w);
            walker::settle(w, id, up, 0.5 * size, max_dz, SLOPE, &mut here, 0);
            let timer_out = c::dec_timer_pvar_s16(w, id, TIMER) != 0;
            if timer_out && hero_d(w) < 5.0 * size {
                let t = ahead(w, d * size, here);
                let g = ground::ground(w, t, 0.5, 0).z;
                if (g - t[2]).abs() <= (d + d) * size {
                    w.mm(id).state = 3;
                    blend(w, id, 3, 10);
                }
            } else if c::diff_rots(away, c::yaw(w, id)) < f32::from_bits(0x3db2_b8c2) || 6.0 * size < hero_d(w) {
                to_state1(w);
            }
        }
        3 => {
            if 6.0 * size < hero_d(w) {
                w.mm(id).state = 1;
                let (a, b) = (w.ticks(60), w.ticks(180));
                let r = w.rng.rand_range(a, b);
                c::set_pi16(w, id, TIMER, r as i16);
                let s = (w.rng.randi(2) == 0) as u8;
                blend(w, id, s, 5);
                walker::settle(w, id, up, radius, max_dz, SLOPE, &mut here, 0);
            } else if hero_d(w) < 3.0 * size {
                turn(w);
                let mut to = ahead(w, d + d, here);
                moved = walker::move_ground(w, id, up, radius, max_dz, SLOPE, &mut here, &mut to, 0);
                if moved == 0 && !went_ahead(w, here) {
                    to_state1(w);
                } else {
                    w.mm(id).state = 4;
                    blend(w, id, 2, 10);
                }
            } else {
                turn(w);
                let mut to = ahead(w, d * size, here);
                moved = walker::move_ground(w, id, up, radius, max_dz, SLOPE, &mut here, &mut to, 0);
                if moved == 0 && !went_ahead(w, here) { to_state1(w); }
            }
        }
        4 => {
            let timer_out = c::dec_timer_pvar_s16(w, id, TIMER) != 0;
            if timer_out && 4.0 * size < hero_d(w) {
                turn(w);
                let mut to = ahead(w, d * size, here);
                moved = walker::move_ground(w, id, up, radius, max_dz, SLOPE, &mut here, &mut to, 0);
                if moved == 0 && !went_ahead(w, here) {
                    to_state1(w);
                } else {
                    w.mm(id).state = 3;
                    blend(w, id, 3, 10);
                }
            } else {
                turn(w);
                let mut to = ahead(w, (d + d) * size, here);
                moved = walker::move_ground(w, id, up, radius, max_dz, SLOPE, &mut here, &mut to, 0);
                if moved == 0 && !went_ahead(w, here) { to_state1(w); }
            }
        }
        5 => {
            // Only the Suck Cannon reaches this state, and it takes a chicken only with its record (`react::record`: the
            // suck record at +0xe0 of a full [`PVARS`] block).
            if react::carried(w, id, KNOCK) != 0 {
                w.mm(id).state = 0;
                c::set_pi16(w, id, SUCK + react::rec::STATE, 0);
            }
            if w.rng.randi(4) == 0 { feather(w, id, here, size); }
            return;
        }
        _ => {}
    }
    tail(w, id, moved, here, size);
    if w.m(id).state != 0xfd && w.m(id).state != 0xfe { w.mm(id).anim.speed = speed0; }
}

/// The end of the update (0x2e0780..): the fallback step, gravity, the ground, the tilt, the fall-out.
fn tail(w: &mut World, id: MobyId, moved: i32, mut here: c::V, size: f32) {
    let d = dt(w);
    let pos = c::pos(w, id);
    let mut take = moved != 0;
    if !take {
        let mut step = c::sub(here, pos);
        if d * size < c::len3(step) { step = c::set_len3(step, d * size); }
        here = c::add(pos, step);
        let g = ground::ground(w, here, 0.5, 0x20).z;
        take = (g - w.m(id).position[2]).abs() <= d * size;
    }
    if take { c::set_pos(w, id, here); }
    let vz = c::pf(w, id, VZ) + dt2(w) * 9.8;
    c::set_pf(w, id, VZ, vz);
    w.mm(id).position[2] -= vz;
    let p = c::pos(w, id);
    let g = ground::ground(w, p, 0.5, 0);
    if p[2] < g.z && g.z < p[2] + 0.25 * size {
        c::set_pf(w, id, VZ, 0.0);
        w.mm(id).position[2] = g.z;
    }
    if g.surface == 0 { c::set_pf(w, id, HEALTH, 0.0); }
    if w.m(id).state != react::CHICKEN.held {
        if let Some((n, z)) = ground_hit(w, c::pos(w, id)) {
            w.mm(id).position[2] = z;
            let (tx, ty) = ground_tilt(n, c::yaw(w, id));
            let k = d * TAU;
            let r = w.m(id).rotation;
            let rx = c::add_rot(r[0], c::sub_rot(tx, r[0]) * k);
            let ry = c::add_rot(r[1], c::sub_rot(ty, r[1]) * k);
            w.mm(id).rotation[0] = rx;
            w.mm(id).rotation[1] = ry;
        }
    }
    if w.m(id).position[2] < 0.0 { w.delete_moby(id); }
}

/// Below 1 health (0x2df760..0x2dfb68): the burst, the feathers, the sound, and the hidden state 6 (or the delete of
/// a displaced chicken).
fn burst(w: &mut World, id: MobyId, size: f32, here: c::V) {
    let gold = c::pi32(w, id, GOLD);
    if gold == 0 {
        let d = dt(w);
        let n = (size as i32) * 20;
        for _ in 0..n {
            let r = w.rng.randf(0.0, 0.37) * size;
            let a = w.rng.rand_angle();
            let b = w.rng.rand_angle();
            let mut p = fx::polar(r, a, b);
            p[2] += w.rng.randf(0.25, 0.5) * size;
            let p = c::add(p, here);
            let s = w.rng.randf(d * 0.1, d) * size;
            let a = w.rng.rand_angle();
            let b = w.rng.rand_angle();
            let v = fx::polar(s, a, b);
            let sz = size * 150_000.0;
            let (lo, hi) = (w.ticks(30), w.ticks(57));
            let life = w.rng.rand_range(lo, hi);
            crate::moby_update::creature::projectile::part22(w, &crate::particles::type22::Spawn { size: sz, pos: p, vel: v, c1: 0x3f7f_7f7f, c2: 0x002f_2f2f, life });
        }
    } else {
        let b = fx::Beam { damage_r: 0.0, damage: 2.0, flash: 4.0, flash2: 3.0, flash_dist: 9.0, scale: 1.0, light: 12.0, streaks: 0, sparks: 0, puffs: 0, debris: 1, sound: -1, shake: true };
        let p = c::pos(w, id);
        fx::beam_explosion_shift(w, &b, Some(id), p, gold as u8);
        attack::area_hit(w, 3.0, p, id, 1.0, 1.0, 1.0, None, 0x83_0000, 2, 1);
    }
    let mut n = w.rng.rand_range(5, 8);
    let (l0, l1) = loads(w);
    if 0.85 < l0 || 0.85 < l1 {
        n = if 1.0 < l0 || 1.0 < l1 { 1 } else { n / 2 };
    }
    for _ in 0..n.max(0) { feather(w, id, here, size); }
    w.play_sound(1, 0, id);
    if 0 <= c::pi32(w, id, DISPLACED) {
        w.delete_moby(id);
        return;
    }
    let m = w.mm(id);
    m.has_collision = false;
    m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN | mode::NO_ANIM;
    c::set_pi32(w, id, GOLD, 0);
    w.mm(id).state = 6;
    w.mm(id).cmd = 0;
}
