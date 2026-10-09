//! **The Comet-Strike and the thrown wrench** (level01; docs/plan/hero_states.md "Weapons + first person").
//!
//! * **State 0x15 "comet strike"** (group 6, crouch + □, or □ again late in a comet): its entry is
//!   [`super::melee`]'s group-6 entry (combo row 3, aim, the side toggle 0x13fdcc, `SetAnim(10, 0x1a, 0)` and the
//!   loop `0x247cb8(6, 0x15)`); its physics (`0x2370b8` case 0x15, [`physics`]): target speed 0, the swap lockout,
//!   the aim (or the turn toward it at 15 rad/s, `0x2709f8(aim, yaw, 1) ≤ 250°` forcing the turn's direction), the
//!   **throw `0x236da0` when the key time passes 33**, `SpeedStep(37, 28)·dt²`, the combo's gravity; its
//!   transitions (`0x242930` case 0x13/0x14/0x15, [`super::melee`]'s `tr_melee`): the combo row's jump / idle
//!   frames, then — with the wrench held — after 100 ticks, when the loop end 0x13fe04 is still ≤ 0x19, the loop
//!   exit `0x247d18(2)` with `0x13fe04 = 0x1a`; another hand item: idle.
//! * **The throw `0x236da0`** ([`throw_wrench`]): in the look stances (1 / 0x1e after 20 ticks: the first-person
//!   throw, `HeroPdaGadget` 0x240ed8 item 8 calls it directly, no state change) aimed along the camera (yaw 0x167258,
//!   pitch −0x167254 + 7°) or at the point the camera's 10-unit line hits; else along Ratchet's facing. The wrench
//!   moby's state byte +0x20 = 10, the slot detached (0x1403fa = 1), speed 23 u/s (+0x60), deceleration 0 (+0x64),
//!   its rotation +0x40 = `0x2721f0` of the camera's Euler rows (look: the model x axis along the view, z along the
//!   view's up) or of Ratchet's moby rows, the spin sequence 6 over 5 ticks, the wall-close flag
//!   +0x76 (0x13f598 < 1.2 with the wall normal steep, 0x13f5a4 = 0); voice 0x1b in the look stance.
//! * **The wrench's flight** (`0x2be1c0` with +0x20 = 10 / 11, [`thrown_update`]; run by the slot loop through the
//!   wrench's row of `HAND_ITEMS`): the swap lock 0x1403fc = 2; the whoosh (Ratchet's class sound 0xe looping in
//!   hero slot 0x14156c) while out; the spin 24.43 rad/s about the wrench's own z axis ([`turn_local`], `0x277380`:
//!   a matrix product in the moby's frame, then back to Euler); 0.55 above the ground (`GroundHeight(0.5)`)
//!   when lower than 0.8; **out (10)**: carried with 0.7 of the platform displacement 0x13f490, along its
//!   direction at +0x60, the deceleration growing by 170·dt³ per tick until the speed is 0 → **back (11)**: toward
//!   the hand point 0x1403c0 (`HeroItemsAttach` keeps it for a detached item), accelerating by 0.9·dt² per tick;
//!   back to sequence 1 within 5 ticks of the hand; within 4 ticks the catch: Ratchet's loop exit (the anim leaves
//!   the comet loop: `0x247d18(2)`, `0x13fe04 = 0x1a`) and the whoosh stops; on arrival voice 5, the slot attached
//!   again, +0x20 = 0. The world line old → new (flags 2) — or the wall-close flag once — bounces it: the clank
//!   (wrench class sound 2, once per 30 ticks), and going out: half speed and back. **Hits**: a 0.4 sphere at the
//!   wrench's joint list 1 (else list 0) through the hero's hit path (`coll_sphere_mobys`, template flags 0x10000,
//!   damage 1, dir = its flight direction ·1.2 (−0.37 on the way back), +0x1a = 0x13fdcc + 1): crates break; the
//!   first hit of 30 ticks plays the wrench's hit sound.
//! * **The bolts it brings back** (`0x2bdfb8`, [`collect`]): each tick on the way back, before the move, the first
//!   moby of the run list within 2.7 of the wrench that is a bolt (class type 0x13) starts flying to Ratchet
//!   (`0x2bcb90(randf(−30, 30)°, 0)`), or an ammo pickup (`0x2732b8`) is collected (`0x2db850`).
//!
//! Native `f32` for the flight (the moby side); the state code keeps the hero block's PS2 float type at its
//! boundary. The after-images of 0x140b00 (`0x277400` / `0x277428` at the throw,
//! `0x277508` each flight tick, `0x277740` at the catch) are `crate::afterimage`'s. **Not ported**: the look stance's aiming beam `0x20fb60`, the wall-hit sparks
//! `0x2bdd20` / hit sparkles `0x2bdb18` (particle types 0x2d / 0x35 records; their draws are not made), the hit-record checks of the hit sound (`FUN_002711f8` records of the
//! same throw, target type 0x14), and the world line's moby part (the port's line is the world mesh: a non-crate
//! moby does not bounce the wrench; it is still hit by the sphere).

use super::anim::{AnimCtl, AnimView};
use super::items::{HitSink, ItemEnv};
use super::melee::COMBO;
use super::packs::SoundCmd;
use super::physics::*;
use super::Hero;
use crate::moby_runtime::MobyTable;
use crate::moby_update::services::{self as sv, HitTemplate};
use crate::ps2v::Pf;
use rc_formats::moby_anim;

/// Wrench moby states (+0x20): in hand, flying out, flying back.
pub const IN_HAND: u8 = 0;
pub const OUT: u8 = 10;
pub const BACK: u8 = 11;
/// Ratchet's class sound of the whoosh (looping, flags 4) and its hero slot (0x14156c = 0x141568 + 4).
pub const WHOOSH_SOUND: i32 = 0xe;
pub const WHOOSH_SLOT: usize = 1;
/// Voices: the first-person throw (0x1b) and the catch (5).
pub const LOOK_THROW_VOICE: i32 = 0x1b;
pub const CATCH_VOICE: i32 = 5;
/// The wrench's class sound of a bounce off the world.
pub const CLANK_SOUND: i32 = 2;

const DT: f32 = 1.0 / 60.0;

/// The wrench's flight fields (its pvars +0x40..+0x7e while thrown) and its rotation (moby +0x40).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flight {
    /// +0x40: the unit direction out.
    pub dir: [f32; 3],
    /// +0x60 / +0x64: speed (u/tick) and its change per tick.
    pub speed: f32,
    pub accel: f32,
    /// +0x76: the wall is close ahead at the throw (bounce at once).
    pub wall_close: bool,
    /// Moby +0x40: Euler rotation (x, y, z; the moby matrix `Rz·Ry·Rx` is built from it while detached).
    pub euler: [f32; 3],
}

/// The Comet-Strike fields of the hero block that are not the melee block's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Comet {
    /// The catch's `0x247d18(2)` + `0x13fe04 = 0x1a` asked for by the wrench's update (it runs after Ratchet's
    /// advance; the port applies it before the next advance, where the game's store takes effect).
    pub catch_exit: bool,
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn scale3(a: [f32; 3], k: f32) -> [f32; 3] { [a[0] * k, a[1] * k, a[2] * k] }
fn len3f(a: [f32; 3]) -> f32 { (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt() }
fn setlen(a: [f32; 3], l: f32) -> [f32; 3] {
    let n = len3f(a);
    if n == 0.0 { [0.0; 3] } else { scale3(a, l / n) }
}
fn wrap(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    let mut x = a % t;
    if x > std::f32::consts::PI { x -= t; }
    if x < -std::f32::consts::PI { x += t; }
    x
}
/// `0x2721f0(0x221980(e))` in the game's rows convention: rows of `e`, back to Euler (x, y, z).
fn rows_to_euler(rows: &[V4; 4]) -> [f32; 3] { to_f32x3(sv::rows_euler(rows)) }

/// The throw's rotation (`0x236da0`): the identity rows through the source rows (`0x1fa328(M, src, M)`: M = src),
/// row 3 = (0, 0, 0, 1) (`0x1fa298`), back to Euler (`0x2721f0`). The source is the camera's Euler 0x167250
/// (look stance) or Ratchet's moby rows +0xc0.
pub fn launch_euler(src: &[V4; 4]) -> [f32; 3] {
    let z = Pf::ZERO;
    rows_to_euler(&[src[0], src[1], src[2], [z, z, z, Pf::ONE]])
}

/// `0x277380(ax, ay, az, moby)`: turn a moby's Euler rotation +0x40 by an Euler step **in its own frame**:
/// `W = 0x221980(+0x40)`, `S = 0x221980(step)`, `W ← 0x221ce8(W, W, S)` (row i of S through W: R = R·S, the step
/// applied to the model before the moby's rotation), then `+0x40 = 0x2721f0(W)`. The thrown wrench's spin
/// `(0, 0, dt·24.43)` is thus about its own z axis, whatever its pitch.
pub fn turn_local(euler: [f32; 3], step: [f32; 3]) -> [f32; 3] {
    let w = sv::euler_rows(from_f32x3(euler));
    let s = sv::euler_rows(from_f32x3(step));
    rows_to_euler(&sv::mat4_mul(&w, &s))
}

/// `0x277b50(len, yaw, pitch)`.
fn polar(len: f32, yaw: f32, pitch: f32) -> [f32; 3] { [yaw.cos() * len * pitch.cos(), yaw.sin() * len * pitch.cos(), pitch.sin() * len] }
fn approach(target: f32, step: f32, x: &mut f32) {
    if *x < target { *x = (*x + step).min(target) } else { *x = (*x - step).max(target) }
}

/// 0x15's physics (`0x2370b8` case 0x15).
pub(super) fn physics(h: &mut Hero, env: &Env, anim: &mut dyn AnimCtl) -> bool {
    h.target_speed = Pf::ZERO;
    let v = anim.view();
    let frame = v.frame;
    let blending = v.blending();
    let row = &COMBO[3];
    if blending || frame < row[6] as f32 { h.items.slot.swap_timer = 2; }
    let from_look = h.prev_state == 1 && ticks(20) < h.prev_timer;
    if !from_look {
        if h.melee.aimed == 0 && h.melee.target.is_none() {
            // Aim while blending or before frame 6 (FUN_002351d0(14, 30°, 45°): no targets in the port).
            if (blending || !(6.0 < frame)) && h.gravity_mode == 0 { h.melee_aim_pub(env); }
        } else {
            let mut force = false;
            if h.f658 == 0 {
                let d = fast_subtract_rotations(h.melee.aim_yaw, h.rot[2]);
                let dd = super::physics::angle_diff(h.melee.aim_yaw, h.rot[2], 1);
                force = dd <= Pf::b(0x408b_a058);
                if Pf::ZERO < d { h.anim_speed = Pf::ONE / (dd + Pf::ONE); }
            }
            h.target_yaw = h.melee.aim_yaw;
            turn_to_mode(h, SCALE64 * Pf::b(0x3d4c_cccd), SCALE64 * Pf::b(0x3e4c_cccd), DT_PF * Pf::b(0x4170_2845), force as i32);
        }
    }
    if !blending && passed(&v, 33.0) { throw_wrench(h, env, false); }
    h.speed_step(DT2 * Pf::b(0x4214_0000), DT2 * Pf::b(0x41e0_0000));
    h.set_planar_vel(Pf::b(0x47c3_4f80));
    // 0x248b68: along −normal on the Magneboots (mode 0: z only).
    let (src, amount) = if h.air_ticks != 0 { (h.eff_v, DT2 * Pf::b(0x41c8_0000)) } else { (h.vel, DT2 * Pf::b(0x4258_0000)) };
    h.vel = super::boots::gravity(h, src, amount);
    true
}

const DT_PF: Pf = super::physics::DT;

/// `TurnTo(k, d, max, mode)` 0x232490 with the direction mode of the angular spring (gravity mode 0; the other
/// modes turn in the hero's frame without a mode).
fn turn_to_mode(h: &mut Hero, k: Pf, d: Pf, max: Pf, mode: i32) {
    if h.gravity_mode != 0 || mode == 0 { return h.turn_to(k, d, max); }
    let (mut yaw, mut vel) = (h.rot[2], h.yaw_vel);
    h.yaw_residual = super::physics::turn_spring(h.target_yaw, k, d, max, &mut yaw, &mut vel, mode);
    h.rot[2] = yaw;
    h.yaw_vel = vel;
}

/// `0x231f18(f)`: the key time 0x13fdf8 passed `f` this tick.
pub fn passed(v: &AnimView, f: f32) -> bool { f < v.frame && v.frame - f <= v.frame_step }

/// The 0x15 part of the transitions after the combo row's jump / idle tests (`0x242930`, case 0x13..0x15 tail):
/// the wrench held → after 100 ticks the loop exit (once: 0x13fe04 ≤ 0x19); another hand item → idle.
pub(super) fn transitions_tail(h: &mut Hero, c: &mut super::states::Ctx) {
    if h.items.slot.id == super::items::item::WRENCH {
        let (_, end) = c.anim.loop_state();
        if !(ticks(100) < h.timer) || 0x19 < end { return; }
        c.anim.exit_loop(0x1a);
        return;
    }
    h.set_state(c, 0, true);
}

/// Before Ratchet's advance: the catch's loop exit the wrench's update asked for last tick.
pub(super) fn before_advance(h: &mut Hero, anim: &mut dyn AnimCtl) {
    if std::mem::take(&mut h.comet.catch_exit) { anim.exit_loop(0x1a); }
}

/// `0x236da0`: the throw. `look`: called from the look stance by the weapon check (the stance's own test of
/// state 1 / 0x1e after 20 ticks is repeated here, as the game does).
pub fn throw_wrench(h: &mut Hero, env: &Env, _from_check: bool) {
    let look = (h.state == 1 || h.state == 0x1e) && ticks(20) < h.timer;
    let cam = env.world.and_then(|w| w.camera());
    let (mut yaw, mut pitch) = (0.0f32, 0.0f32);
    let mut cam_euler = [0.0f32; 3];
    let moby_rows = h.moby_rows;
    if look {
        let (cpos, cyaw, cpitch) = cam.unwrap_or(([0.0; 3], env.cam_yaw.to_f32(), 0.0));
        cam_euler = [0.0, cpitch, cyaw];
        // The in-hand update's look line (state 1: from the camera, 10 units along its view; +0x7e / +0x50).
        let d = polar(10.0, cyaw, -cpitch);
        let a = from_f32x3(cpos);
        let b = from_f32x3(add3(cpos, d));
        let target = if cam.is_some() { env.line(a, b, 4).map(|o| o.point) } else { None };
        match target {
            Some(t) => {
                // From the hero-local point (0.5, 0, 0.5) (`0x248cf8`).
                let r = h.rows.map(to_f32x3);
                let p0 = to_f32x3(h.pos);
                let p: [f32; 3] = std::array::from_fn(|k| (r[0][k] * 0.5 + r[2][k] * 0.5) + p0[k]);
                let d = sub3(t, p);
                yaw = d[1].atan2(d[0]);
                pitch = d[2].atan2((d[0] * d[0] + d[1] * d[1]).sqrt());
            }
            None => {
                yaw = cyaw;
                pitch = wrap(-cpitch + f32::from_bits(0x3dfa_35dd));
            }
        }
    }
    let Some(it) = h.items.slot.item.as_mut().filter(|m| m.o_class == super::melee::WRENCH_CLASS) else { return };
    if h.items.slot.state != 2 { return; }
    h.items.slot.detached = 1;
    if look { h.packs.sounds.push(SoundCmd::Voice { index: LOOK_THROW_VOICE, flags: 0 }); }
    it.mstate = OUT;
    let f = &mut it.flight;
    f.wall_close = h.wall_ahead[0] < 1.2 && f32::from_bits(0x3f29_c91f) < h.wall_ahead[1] && h.f5a4 == 0;
    if look {
        f.dir = polar(1.0, yaw, pitch);
        // The camera's Euler 0x167250 (roll, pitch — positive looking down —, yaw; roll is 0 on foot) through
        // its rows: the wrench's model x axis points along the view, its z axis along the view's up.
        f.euler = launch_euler(&sv::euler_rows(from_f32x3(cam_euler)));
    } else {
        // 0x248da0(1.0): along the moby's yaw (+0x48) — on the Magneboots its model x axis (the rows of +0x40);
        // the rotation from Ratchet's moby rows +0xc0.
        f.dir = if h.gravity_mode == 0 {
            let y = h.moby_rot[2].to_f32();
            [y.cos(), y.sin(), 0.0]
        } else {
            to_f32x3(sv::euler_rows(h.moby_rot)[0])
        };
        f.euler = launch_euler(&moby_rows);
    }
    f.accel = 0.0;
    f.speed = DT * 23.0;
    // fun_00212f90(wrench, 6, 0, 5): the spin sequence (applied at the start of the item update).
    h.items.pending_blend = Some((6, 0, ticks(5)));
    // The wrench's after-images (0x277400(wrench, 0x140b00), 0x277428 × 2: 0x30 / 0x17 at 3 / 5 ticks back;
    // crate::afterimage).
    let t = &mut h.fx.trails.wrench;
    t.start(super::melee::WRENCH_CLASS);
    t.add(0x30, 3);
    t.add(0x17, 5);
}

/// The wrench's update with +0x20 = 10 / 11 (`0x2be1c0`; the slot loop calls it for the wrench's row).
pub fn thrown_update(hero: &mut Hero, table: &mut MobyTable, anim: &dyn AnimCtl, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut crate::rng::Rng) {
    let Some(class) = hero.items.slot.item.as_ref().and_then(|m| env.data.class(m.o_class)).cloned() else { return };
    hero.items.slot.swap = 2;
    // 0x277508(0x140b00, 0): the after-images follow the wrench as its last update left it (crate::afterimage).
    if let Some(it) = hero.items.slot.item.as_ref() {
        let place = crate::afterimage::Place { rows: it.rows.map(|r| r.map(f32::from_bits)), position: [it.position[0], it.position[1], it.position[2], 1.0] };
        let a = &it.anim;
        let pose = crate::afterimage::Pose { seq_a: a.seq_a, seq_b: a.seq_b, frame_a: a.frame_a, frame_b: a.frame_b, t: a.t };
        hero.fx.trails.wrench.update(Some((place, pose)), 0);
    }
    let hand = hero.items.slot.hand_point;
    let mstate = hero.items.slot.item.as_ref().map_or(0, |m| m.mstate);
    // The whoosh (0x14156c): started while going out when the slot is free.
    if mstate == OUT && hero.packs.loops[WHOOSH_SLOT] == -1 && !hero.fx.item_voices.iter().any(|c| matches!(c, SoundCmd::Loop { n: WHOOSH_SLOT, .. })) {
        hero.fx.item_voices.push(SoundCmd::Loop { n: WHOOSH_SLOT, sound: WHOOSH_SOUND });
    }
    // The hit flag 0x13fdb4 clears when the wrench's hit timer (+0x6c) runs out (`FastDecTimer` ≠ 0: it is 0
    // after the count).
    if hero.melee.hit != 0 {
        if let Some(it) = hero.items.slot.item.as_mut() {
            if it.hit_timer > 0 { it.hit_timer -= 1; }
            if it.hit_timer == 0 { hero.melee.hit = 0; }
        }
    }
    let disp = to_f32x3(hero.plat_applied);
    let gravity_mode = hero.gravity_mode;
    let loop_active = anim.loop_state().0;
    let it = hero.items.slot.item.as_mut().unwrap();
    // The spin: 24.43 rad/s about the wrench's own z axis (`0x277380(0, 0, dt·24.43)`: a matrix product in the
    // moby's frame, not an Euler z step; they agree only while the wrench is level).
    it.flight.euler = turn_local(it.flight.euler, [0.0, 0.0, DT * 24.434_608]);
    let mut pos = it.position;
    if gravity_mode == 0 {
        if let Some(coll) = env.coll {
            // GroundHeight(0.5, pos): a line from 0.5 above down to z 0.01, flags 2.
            let a = [pos[0], pos[1], pos[2] + 0.5];
            let b = [pos[0], pos[1], 0.01];
            let g = line_world(coll, from_f32x3(a), from_f32x3(b), 2).map_or(0.0, |o| o.point[2]);
            if pos[2] - g < 0.8 {
                let t = g + 0.55;
                let step = if pos[2] < t { DT * 4.0 } else { DT };
                approach(t, step, &mut pos[2]);
            }
        }
    }
    let old = pos;
    if it.mstate == BACK {
        // 0x2bdfb8 at the wrench's height-corrected position, before the move.
        it.position = pos;
        collect(hero, table, env, hits, rng, pos);
    }
    let it = hero.items.slot.item.as_mut().unwrap();
    let mut catch_exit = false;
    let mut arrived = false;
    let f = &mut it.flight;
    if it.mstate == OUT {
        pos = add3(pos, scale3(disp, 0.7));
        pos = add3(pos, setlen(f.dir, f.speed));
        f.accel += DT * DT * DT * 170.0;
        approach(0.0, f.accel, &mut f.speed);
        if f.speed == 0.0 { it.mstate = BACK; }
    } else {
        let d = sub3(hand, pos);
        f.accel += DT * DT * 0.9;
        f.speed += f.accel;
        let dist = len3f(d);
        if dist / f.speed < 5.0 && it.anim.seq_b != 1 {
            moby_anim::set_sequence(&mut it.anim, &class.anim, 1, 0, ticks(5), &mut it.snapshot);
        }
        arrived = dist <= f.speed;
        if arrived { f.speed = dist; }
        pos = add3(pos, setlen(d, f.speed));
        if dist / f.speed < 4.0 && loop_active { catch_exit = true; }
    }
    it.position = pos;
    let rows = euler_rows(from_f32x3(f.euler));
    it.rows = [0, 1, 2].map(|i| rows[i].map(|x| x.0));
    let (dir, mstate) = (f.dir, it.mstate);
    if catch_exit {
        hero.comet.catch_exit = true;
        release_whoosh(hero);
    }
    if arrived {
        hero.fx.item_voices.push(SoundCmd::Voice { index: CATCH_VOICE, flags: 0 });
        release_whoosh(hero);
        hero.items.slot.detached = 0;
        if let Some(it) = hero.items.slot.item.as_mut() { it.mstate = IN_HAND; }
        hero.fx.trails.wrench.kill();
    }
    // The bounce: the world line old → new (flags 2) on a non-water face, or the wall-close flag once.
    let hit_world = env.coll.and_then(|c| line_world(c, from_f32x3(old), from_f32x3(pos), 2)).is_some_and(|o| o.surface_id() != 0);
    let it = hero.items.slot.item.as_mut().unwrap();
    let forced = std::mem::take(&mut it.flight.wall_close);
    if hit_world || forced {
        if hero.melee.hit == 0 {
            it.hit_timer = ticks(30);
            hero.melee.hit = 1;
            hero.fx.item_sounds.push(CLANK_SOUND);
        }
        if it.mstate == OUT {
            it.flight.speed *= 0.5;
            it.mstate = BACK;
        }
    }
    if arrived { return; }
    // The hit test: 0.4 at joint list 1, else list 0.
    let k = if mstate == BACK { f32::from_bits(0xbebd_70a4) } else { f32::from_bits(0x3f99_999a) };
    let d = setlen(dir, k);
    let tmpl = HitTemplate {
        dir: [Pf::f(d[0]), Pf::f(d[1]), Pf::ONE, Pf::b(0x45af_df66)],
        attacker: None,
        flags: 0x1_0000,
        b18: 0,
        b19: 1,
        h1a: (hero.melee.comet_side + 1) as u16,
        damage: Pf::ONE,
        w20: 1,
    };
    let ignore = Some(env.hero_moby);
    let it = hero.items.slot.item.as_ref().unwrap();
    let point = |list: usize| -> Option<V4> {
        let chain = class.chains.get(list).filter(|c| !c.is_empty())?;
        let p = moby_anim::evaluate_chains(&class.anim, &it.anim, it.snapshot.as_ref(), &[chain.as_slice()]);
        Some(super::melee::list_point(&p[0], &it.rows, it.position, it.scale))
    };
    let r = Pf::b(0x3ecc_cccd);
    let mut hit = None;
    for list in [1, 0] {
        let c = point(list).unwrap_or_else(|| from_f32x3(it.position));
        if let Some(m) = hits.sphere(table, r, c, 0, ignore, &tmpl) {
            hit = Some(m);
            break;
        }
    }
    if hit.is_some() && hero.melee.hit == 0 {
        if let Some(it) = hero.items.slot.item.as_mut() { it.hit_timer = ticks(30); }
        hero.melee.hit = 1;
        hero.items.hit_sounds += 1;
        hero.fx.item_sounds.push(super::melee::wrench_hit_sound(table, hit));
    }
}

/// `0x2bdfb8(wrench)`: the run list (0x15ffe4) in order, up to a deleted moby; the first within 2.7 of the wrench at `at`
/// that is a bolt (class type 0x13) flying off to Ratchet (`0x2bcb90(randf(−30, 30)·π/180, 0)`) or an ammo pickup
/// (`0x2732b8`) collected (`0x2db850`) ends the walk. Their writes to Ratchet's block (the ammo) are applied at once,
/// as the Suck Cannon's vacuum does.
fn collect(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut crate::rng::Rng, at: [f32; 3]) {
    use crate::moby_runtime::state;
    use crate::moby_update::classes::{bolt, pickup};
    let cam = env.camera.map_or([0.0; 3], |c| c.0);
    let mut writes = None;
    hits.world(table, &*hero, rng, env.frame as u64, &mut |w| {
        w.camera = [cam[0], cam[1], cam[2], 1.0].map(Pf::f);
        let list = crate::moby_update::scheduler::build_active_list(w.table, w.camera, &w.svc.groups).0;
        for m in list {
            let st = w.m(m).state;
            if st == state::DELETED || st == state::DELETED_STATIC { break; }
            let o = w.m(m).o_class;
            let bolt = w.m(m).has_class && w.classes.info(o).map(|i| i.ty) == Some(0x13);
            if !bolt && !super::suck_cannon::PICKUP_CLASSES.contains(&o) { continue; }
            let p = w.m(m).position;
            if 2.7 <= len3f(sub3([p[0], p[1], p[2]], at)) { continue; }
            let taken = if bolt {
                let a = w.rng.randf(-30.0, 30.0) * 0.017_453_292;
                bolt::start_fly(w, m, Pf::f(a), Pf::ZERO)
            } else {
                pickup::collect(w, m)
            };
            if taken { break; }
        }
        let now = w.counter;
        writes = w.svc.hero_writes.take_if(|(c, _)| *c == now).map(|(_, f)| f);
    });
    if let Some(f) = writes { f.apply(hero); }
}

/// The whoosh's slot released (`release_voice_slot(0x14156c)` when it still plays Ratchet's sound; −1).
fn release_whoosh(h: &mut Hero) {
    let s = std::mem::replace(&mut h.packs.loops[WHOOSH_SLOT], -1);
    if s != -1 { h.fx.item_voices.push(SoundCmd::Release { slot: s }); }
    h.fx.item_voices.retain(|c| !matches!(c, SoundCmd::Loop { n: WHOOSH_SLOT, .. }));
}

#[cfg(test)]
mod tests {
    use super::super::items::{self, HandItem, ItemClass, ItemData, ItemEnv, NoHits};
    use super::super::testkit::*;
    use super::super::AnimView;
    use super::*;
    use crate::pad::{button, PadInput};
    use crate::rng::Rng;
    use rc_formats::moby_anim::{AnimState, MobyAnimClass};

    /// A data-free animation whose key time advances by the playback speed per tick (blends land at once), with
    /// the loop range and its exit.
    #[derive(Default)]
    struct KeyAnim {
        v: AnimView,
        calls: Vec<(u8, i32)>,
        exit: Option<i32>,
        looped: Option<(i32, i32)>,
    }

    impl AnimCtl for KeyAnim {
        fn set_anim(&mut self, _: Pf, seq: u8, frame: i32) {
            self.calls.push((seq, frame));
            self.v.seq_a = seq;
            self.v.seq_b = seq;
            self.v.frame = frame as f32;
            self.looped = None;
            self.exit = None;
        }
        fn advance(&mut self, speed: Pf) {
            let old = self.v.frame;
            self.v.frame += speed.to_f32();
            if let Some(to) = self.exit.take() {
                self.v.frame = to as f32 + 1.0;
            } else if let Some((a, b)) = self.looped {
                if self.v.frame > b as f32 { self.v.frame = a as f32; }
            }
            self.v.frame_step = if old <= self.v.frame { self.v.frame - old } else { 0.0 };
        }
        fn view(&self) -> AnimView { self.v }
        fn frame_count(&self, _: u8) -> u8 { 100 }
        fn set_loop(&mut self, a: i32, b: i32) { self.looped = Some((a, b)); }
        fn clear_loop(&mut self) { self.looped = None; }
        fn exit_loop(&mut self, to: i32) {
            self.exit = Some(to);
            self.looped = None;
        }
        fn loop_state(&self) -> (bool, i32) { (self.looped.is_some(), self.looped.map_or(0x1a, |l| l.1)) }
    }

    fn wrench() -> HandItem {
        let anim = AnimState { seq_a: 1, frame_a: 0, seq_b: 1, frame_b: 0, t: 0.0, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false };
        HandItem { o_class: super::super::melee::WRENCH_CLASS, mstate: 0, anim, snapshot: None, scale: 1.0, position: [410.0, 410.0, 101.0], rows: [[0; 4]; 3], hit_timer: 0, flight: Default::default() }
    }

    fn data() -> ItemData {
        let empty = MobyAnimClass { joint_count: 1, skeleton: vec![], rest: vec![[0.0; 3]], parent_word: vec![0], sequences: vec![] };
        ItemData { defs: vec![items::ItemDef::default(); 37], hero_chains: vec![], classes: vec![ItemClass { o_class: 0x47, anim: empty, scale: 1.0, chains: vec![] }] }
    }

    fn runner() -> (Runner, rc_formats::collision::Collision, KeyAnim) {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.hero.items.slot = items::HandSlot { item: Some(wrench()), fire_mask: button::SQUARE, state: 2, id: 8, ..Default::default() };
        let mut a = KeyAnim::default();
        for _ in 0..3 { tick(&mut r, &coll, &mut a, PadInput::neutral()); }
        (r, coll, a)
    }

    /// The voices the hero update played.
    #[derive(Default)]
    struct Voices(Vec<i32>);
    impl super::super::HeroSounds for Voices {
        fn anim_advanced(&mut self, _: &crate::moby_runtime::Moby, _: &AnimView, _: &AnimView, _: &mut Rng) {}
        fn voice(&mut self, _: &crate::moby_runtime::Moby, index: i32, _: u32, _: &mut Rng) -> i32 {
            self.0.push(index);
            -1
        }
    }

    fn tick(r: &mut Runner, coll: &rc_formats::collision::Collision, a: &mut KeyAnim, input: PadInput) -> Vec<i32> {
        r.pad.update(Some(&input.bytes()), false);
        let env = Env { coll, pad: &r.pad, cam_yaw: r.cam_yaw, cam_rows: r.cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
        let mut v = Voices::default();
        super::super::hero_update_with_sounds(&mut r.hero, &mut r.moby, &env, a, &mut r.rng, &mut v);
        v.0
    }

    /// Crouch + □ with the wrench: 0x15 (entry anim 0x1a, the loop 6..0x15), the throw when the key time passes 33
    /// (+0x20 = 10, detached, 23 u/s along the facing, the spin sequence 6 over 5 ticks), idle after the catch.
    #[test]
    fn crouch_square_throws_at_frame_33() {
        let (mut r, coll, mut a) = runner();
        let crouch = PadInput::neutral().press(button::R1);
        for _ in 0..8 { tick(&mut r, &coll, &mut a, crouch); }
        assert_eq!(r.hero.state, 4);
        tick(&mut r, &coll, &mut a, PadInput::neutral().press(button::R1 | button::SQUARE));
        assert_eq!(r.hero.state, 0x15);
        assert_eq!(a.calls.last(), Some(&(0x1a, 0)));
        assert_eq!(a.looped, Some((6, 0x15)));
        // The loop keeps the key time below 22: the test anim's loop stands in for the frames; move it past 33.
        a.looped = None;
        let mut thrown_at = None;
        for t in 0..40 {
            tick(&mut r, &coll, &mut a, PadInput::neutral());
            if thrown_at.is_none() && r.hero.items.slot.item.as_ref().unwrap().mstate == OUT { thrown_at = Some((t, a.v.frame)); }
        }
        let (_, f) = thrown_at.expect("no throw");
        assert!((33.0..35.0).contains(&f), "thrown at key time {f}");
        let it = r.hero.items.slot.item.as_ref().unwrap();
        assert_eq!(r.hero.items.slot.detached, 1);
        assert!((it.flight.speed - 23.0 / 60.0).abs() < 1e-6);
        assert!((it.flight.dir[0] - 1.0).abs() < 1e-6 && it.flight.dir[1].abs() < 1e-6);
        assert_eq!(r.hero.items.pending_blend, Some((6, 0, 5)));
    }

    /// The flight: out along +x decelerating (170·dt³ per tick more each tick) until the speed is 0 (~31 ticks, ~7.8
    /// units), back to the hand point accelerating, the whoosh started going out and released at the catch, the
    /// catch's loop exit, voice 5 and +0x20 = 0 on arrival.
    #[test]
    fn flight_out_and_back() {
        let mut h = Hero::spawn([410.0, 410.0, 100.0], 0.0);
        let mut it = wrench();
        it.mstate = OUT;
        it.flight = Flight { dir: [1.0, 0.0, 0.0], speed: 23.0 / 60.0, accel: 0.0, wall_close: false, euler: [0.0; 3] };
        h.items.slot = items::HandSlot { item: Some(it), fire_mask: button::SQUARE, state: 2, id: 8, detached: 1, hand_point: [410.0, 410.0, 101.0], ..Default::default() };
        let d = data();
        let pad = crate::pad::PadState::default();
        let mut table = crate::moby_runtime::MobyTable::new(vec![crate::moby_runtime::Moby::zeroed()], 4);
        let mut anim = KeyAnim { looped: Some((6, 0x15)), ..Default::default() };
        let env = ItemEnv { data: &d, pad: &pad, frame: 0, hero_moby: 0, coll: None, camera: None, camera_up: None, targets: &[] };
        let mut far = 0.0f32;
        let mut back_at = None;
        let mut home_at = None;
        // The first tick starts the whoosh (the flush then stores its slot).
        thrown_update(&mut h, &mut table, &anim, &env, &mut NoHits, &mut Rng::new());
        assert_eq!(h.fx.item_voices, vec![SoundCmd::Loop { n: WHOOSH_SLOT, sound: WHOOSH_SOUND }]);
        h.fx.item_voices.clear();
        h.packs.loops[WHOOSH_SLOT] = 7;
        for t in 1..200 {
            thrown_update(&mut h, &mut table, &anim, &env, &mut NoHits, &mut Rng::new());
            let it = h.items.slot.item.as_ref().unwrap();
            far = far.max(it.position[0] - 410.0);
            if back_at.is_none() && it.mstate == BACK { back_at = Some(t); }
            if h.comet.catch_exit { anim.looped = None; }
            if it.mstate == IN_HAND {
                home_at = Some(t);
                break;
            }
        }
        let (b, e) = (back_at.expect("never back"), home_at.expect("never home"));
        assert!((28..=34).contains(&b), "turned back at {b}");
        assert!((7.0..8.5).contains(&far), "went {far}");
        assert!(e > b + 10, "home at {e}");
        assert!(h.comet.catch_exit, "no catch exit");
        assert_eq!(h.fx.item_voices, vec![SoundCmd::Release { slot: 7 }, SoundCmd::Voice { index: CATCH_VOICE, flags: 0 }]);
        assert_eq!(h.items.slot.detached, 0);
    }

    /// Right-handed rotation matrices (columns = images of the model axes), as `0x221980` builds them
    /// (rows = images: X rows 1/2 = (0, c, s) / (0, −s, c), Y0 = (c, 0, −s), Z0 = (c, s, 0)).
    fn rz(a: f64) -> [[f64; 3]; 3] { let (s, c) = a.sin_cos(); [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]] }
    fn ry(a: f64) -> [[f64; 3]; 3] { let (s, c) = a.sin_cos(); [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]] }
    fn mm(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
        std::array::from_fn(|i| std::array::from_fn(|k| (0..3).map(|j| a[i][j] * b[j][k]).sum()))
    }
    /// Image of model axis `i` (the moby row `i`).
    fn axis(m: [[f64; 3]; 3], i: usize) -> [f64; 3] { [m[0][i], m[1][i], m[2][i]] }
    fn rows_of(e: [f32; 3]) -> [[f64; 3]; 3] {
        let r = sv::euler_rows(from_f32x3(e));
        std::array::from_fn(|i| std::array::from_fn(|k| r[i][k].to_f32() as f64))
    }

    /// The first-person throw's orientation and spin at 0°, 30°, 60° and 84° up, against the game's formula:
    /// launch = `0x2721f0(rows(camera Euler (0, −p, yaw)))` = Rz(yaw)·Ry(−p); each tick `0x277380(0, 0, dt·24.43)`
    /// multiplies in the moby's frame, so after n ticks R = Rz(yaw)·Ry(−p)·Rz(n·dt·24.43): the model z axis stays
    /// the view's up and the wrench turns in the plane of the view direction and the view's left.
    #[test]
    fn first_person_orientation_and_spin_by_pitch() {
        let yaw = 0.7f64;
        let w = (DT * 24.434_608) as f64;
        for deg in [0.0f64, 30.0, 60.0, 84.0] {
            let p = deg.to_radians();
            // The camera's pitch 0x167254 is positive looking down: looking up p is −p.
            let cam = [0.0, -p as f32, yaw as f32];
            let mut e = launch_euler(&sv::euler_rows(from_f32x3(cam)));
            assert!(e[0].abs() < 1e-4 && (e[1] as f64 + p).abs() < 1e-4 && (e[2] as f64 - yaw).abs() < 1e-4, "{deg}°: {e:?}");
            let up = axis(mm(rz(yaw), ry(-p)), 2);
            for n in 1..=40 {
                e = turn_local(e, [0.0, 0.0, DT * 24.434_608]);
                let want = mm(mm(rz(yaw), ry(-p)), rz(n as f64 * w));
                let got = rows_of(e);
                for i in 0..3 {
                    let (a, b) = (axis(want, i), got[i]);
                    for k in 0..3 { assert!((a[k] - b[k]).abs() < 2e-3, "{deg}° tick {n} row {i}: {got:?} vs {a:?}"); }
                }
                // The spin axis (model z) stays the view's up.
                for k in 0..3 { assert!((got[2][k] - up[k]).abs() < 2e-3, "{deg}° tick {n}: z {:?} vs up {up:?}", got[2]); }
            }
        }
    }

    /// The Euler-z step the port used before (a spin about the world z axis) matches the game only while the
    /// wrench is level: at 60° up it tilts the spin axis off the view's up within a few ticks.
    #[test]
    fn world_z_step_differs_when_pitched() {
        let p = 60f64.to_radians();
        let e0 = [0.0f32, -p as f32, 0.0];
        let game = turn_local(turn_local(e0, [0.0, 0.0, 0.4]), [0.0, 0.0, 0.4]);
        let world = [0.0f32, -p as f32, 0.8];
        let (g, w) = (rows_of(game), rows_of(world));
        assert!((0..3).any(|k| (g[2][k] - w[2][k]).abs() > 0.1), "{g:?} vs {w:?}");
        // Level (0°), both agree.
        let game = turn_local(turn_local([0.0, 0.0, 0.3], [0.0, 0.0, 0.4]), [0.0, 0.0, 0.4]);
        assert!((game[0].abs() < 1e-5) && (game[1].abs() < 1e-5) && (game[2] - 1.1).abs() < 1e-4, "{game:?}");
    }

    /// The look stance after 20 ticks: □ throws (no state change) along the camera with voice 0x1b; before 20 ticks
    /// □ is the Comet-Strike state.
    #[test]
    fn look_stance_throw() {
        let (mut r, coll, mut a) = runner();
        let l1 = PadInput::neutral().press(button::L1);
        tick(&mut r, &coll, &mut a, l1);
        assert_eq!(r.hero.state, 1);
        for _ in 0..24 { tick(&mut r, &coll, &mut a, l1); }
        let voices = tick(&mut r, &coll, &mut a, PadInput::neutral().press(button::L1 | button::SQUARE));
        assert_eq!(r.hero.state, 1, "the throw keeps the stance");
        let it = r.hero.items.slot.item.as_ref().unwrap();
        assert_eq!(it.mstate, OUT);
        assert!(voices.contains(&LOOK_THROW_VOICE), "voices {voices:?}");
        // No camera line target here: the camera's yaw and 7° above its pitch.
        assert!((it.flight.dir[2] - f32::from_bits(0x3dfa_35dd).sin()).abs() < 1e-4, "{:?}", it.flight.dir);
    }
}
