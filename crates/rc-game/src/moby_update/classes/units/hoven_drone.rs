//! **Hoven's gun drones, class 326** (level12 `0x2e9f68`, census U397, 8 instances) and **their shot, class 409**
//! (spawner `0x2ecce8`, update `0x2ece00`). A drone carries its rider 1371 ([`super::drone_rider`]), waits hidden until
//! Ratchet enters its cuboid (pvar +0x80), flies in along its path +0xa0, then hovers and fires bursts of shots at its
//! target (Ratchet or a decoy), moving to and fro along its second path +0xa4 between bursts; a wrench-like push knocks
//! it back. **In the turret game** (pvar +0xc0 ≠ 0) the drone is one of the turret 1267's attackers: it starts on a
//! timer once the game is on (sooner after many mounts), aims at the camera (the turret's seat), fires faster
//! shots that do 3 to the turret (2 past 15 mounts), fires fewer per burst past 6 mounts, and respawns after it is shot
//! down; the turret's shells 458 do 10 to it. During the game the ordinary drones not on screen are removed, and the
//! carrier's death camera removes the turret-game drones (their riders knocked off). Read from the level12 decomp and
//! disassembly (`0x2e9f68`, `0x2eb220`, `0x2eaf68`, `0x2eae00`, `0x2ecce8`, `0x2ece00`). Native `f32`.
//!
//! **System or not.** Per-class code (clusters of one) on the creature layer: the target (`target::acquire`), the hit
//! resolver (`damage::resolve`, record +0x20), the hit flash (`flash`, record +0x60), the springs
//! (`turn::spring_turn`, `turn::approach`), `SetDeathBits` (the bolts, `crate_::set_death_bits`), the big-head cheat
//! (`manip::big_head`), the shadow probe (`shadows::probe_down`), the help request 12002 (`Help::request`), the
//! follow camera's focus moby (`cinematic::focus_moby` / `focus_ticks`; the moby loop's view of 0x16735c is the unit
//! word [`FOCUS`], cleared when its moby is gone as the camera clears it).
//!
//! **Pvar block** (0x110): +0x20 the damage record (health 6, column 2), +0x58..+0x5a three bytes (0x16, 1, 7), +0x60 the
//! flash record (+0x67 its red), +0x70 the step, +0x80 the start cuboid, +0x88 / +0x98 / +0x9c the yaw / roll / pitch
//! spring velocities, +0x8c (s16) the fire timer, +0x8e (s16) the knock timer, +0x90 the bob phase, +0x94 the speed,
//! +0xa0 / +0xa4 the paths, +0xac the rider, +0xb0 the hum voice, +0xb4 the path point, +0xb8 the barrel, +0xbc (s16)
//! the shots of the burst, +0xbe (s16) the side of path +0xa4, +0xc0 turret mode, +0xc4 the turret-mode start timer,
//! +0xd0 the rider's big-head manipulator. The shot 409: +0x00 the velocity, +0x10 the owner, +0x14 the life, +0x18 0,
//! +0x1c the heading, +0x24 the speed, +0x28 0.
//!
//! ## Coverage (`0x2e9f68`)
//! | address | what | port |
//! |---|---|---|
//! | entry | game mode 2 → nothing; the target (`0x278ba8(70, self)`); a live rider: the big-head cheat on it (`0x27c220(2.7, rider, 0, +0xd0)`); the hits `0x2eb220` | [`update`], [`intake`] |
//! | entry | Giant Clank (body 2): scale = class scale / 2 (the rider's too), else the class scales | [`update`] |
//! | entry | states 2..5: the hum `PlayClassSound(3, 4, self)` once (+0xb0); else released | [`update`] |
//! | entry | level 12 and the turret game on: an ordinary drone gives up the camera focus and, off screen, is deleted (its rider too); a turret-mode drone: mission done → deleted (with the rider); the carrier's camera (0x1619a0): the rider knocked off (`0x308708`), deleted | [`update`] |
//! | state 0 | the damage record (6, 6, column 2, 0), +0x94 / +0x98 / +0x9c = 0, collision off, mode = (mode & ~0x1000) \| 0x41; the paths' segment lengths (`0x27ba70` on +0xa0 twice, +0xa4); draw distance ≥ 0x80; no rider: one made (`0x308670`; update distance 0, hidden); position = path +0xa0's first point; turret mode: +0xc4 = `rand_range(ticks(240), ticks(400))` + clamp(0x161f20 − 1, 0, 10)·`ticks(35)` (when not set); +0xb4 = 0; → 1 | [`init`], [`segment_lengths`] |
//! | state 1 | Ratchet in cuboid +0x80 (≠ −1), or (turret mode, game on) the timer +0xc4 out: +0x58..+0x5a = 0x16, 1, 7; hit dropped; shown (and the rider); mode \|= 0x1000; → 2; collision on (class +0x10) | [`update`] |
//! | state 2 | ordinary: takes the camera focus when free (0x167360 = 0); turret mode: +0x8c ≥ clamp(0x161f20 − 3, 0, 5)·`ticks(15)`; the path flight `0x2eaf68` on +0xa0 → 3 at its end | [`update`], [`follow_path`] |
//! | state 3 | level 12, item 11 not owned, help record 0x5d unseen → `Help_Request(12002, 0x5d)` | [`update`] |
//! | state 3 | ordinary: the focus (dropped beyond 21 or less than 1 above Ratchet, else taken when free) | [`update`] |
//! | state 3 | target within 40 (xy) and not 2 below, or turret mode: yaw / pitch `SpringTurn` toward it (0.004, 0.3, 45°·dt); step 0; the tilt; `FastDecTimer(+0x8c)` out: a shot (+0xbc + 1; muzzle joint +0xb8, which toggles; aim = the target's aim point, turret mode: the camera with z + `randf(−0.5, 0.1)`, x + `randf_sym(0, 0.4)`; velocity 15·dt (22·dt) toward it; `0x2ecce8`; `PlayClassSound(0, 0, self)`; 25 puffs `PartType04Spawn(muzzle + 0.5·dir, randf_sym³(1.3·dt), 0x7f000000 \| rand_range(110, 180)·0x100 \| rand_range(170, 240), 0xff, ticks(rand_range(30, 70)), 0x1e, rand_range(120, 170), 1)`); the burst 4 (turret mode: 3 past 6 mounts; the game's test for 2 past 10 is overwritten); short or no path +0xa4: +0x8c = `ticks(40)`, else +0xbc = 0, → 4 | [`attack`], [`fire`] |
//! | state 4 | yaw toward the target; to path +0xa4's last (+0xbe = 0) or first point: within 5.5 `Approach(0, 9·dt²)` and stopped: +0x8c = `ticks(50)`, +0xbe flips, → 3; else `Approach(10·dt, 7·dt²)`; step, move, the tilt | [`update`] |
//! | state 5 | `FastDecTimer(+0x8e)` running: speed −= 4·dt² (≥ 0), pushed back along −yaw; out: back to the state in +0xbc (cmd) | [`update`] |
//! | state 6 | the focus dropped; the rider knocked off; `SpawnBeamExplosion(0, 0, 4, 2, 100000, 3, 15, self, 0, pos, 20, 3, 4, 2, 1, 1, −1, 0)`; ordinary: `SetDeathBits(self, 0, −1)` (bolts); the hum released; ordinary: deleted; turret mode: +0xc4 = `rand_range(ticks(240), ticks(400))`, collision off, → 0, mode = (mode & ~0x1000) \| 0x41 | [`update`] |
//! | tail | the bob: z −= cos(+0x90)·0.17, +0x90 += 2π/`ticks(120)`, z += cos(+0x90)·0.17 | [`update`] |
//! | tail | the rider posed on the drone, `MobyAnimAdvance`, `MobyBuildMatrix`, mode \|= 6 | [`update`] |
//! | tail | ordinary, state > 1, drawn, within 30 of the camera: the shadow probe, +0x7f = 0x18 | [`update`] (`shadows::probe_down`) |
//! | 0x2eb220 | the hit (0x330000) through `0x273368(self, hit, +0x20, 0, &out5, &dmg, 0, 4)`; out5 ≠ 1, not dying, not its own shot (409) nor a gun shot (184): the shell 458 does 10; health −= it; reaction 1 or health ≤ 0: `PlayClassSound(2, 0, self)`, → 6; else not 0xb: the flash (red 0xb4); an exact push (w 5627.925) not already knocked: the tilt away from it (roll −20°·sin, pitch 5°·cos when larger), speed 4·dt, +0x8e = `ticks(60)`, cmd = state, → 5, `PlayClassSound(1, 0, self)` | [`intake`] |
//! | 0x2eb220 | hit dropped; the flash's tick (`0x276428`) | [`intake`] |
//! | 0x2eaf68 | the rest of the path ≤ 5: yaw toward the target (`target(48)`), speed `Approach(0, 9·dt²)`, at 0 → done; else yaw toward the point, `Approach(10·dt, 7·dt²)`; step = toward the point at the speed; move; within 2 the next point (the last kept); the tilt | [`follow_path`] |
//! | 0x2eae00 | the tilt from the step's heading relative to the yaw: pitch `SpringTurn` to speed·13°·cos / (10·dt) (30° / 60°·dt², 45°·dt), roll to −speed·13°·sin / (10·dt) (60° / 120°·dt², 70°·dt) | [`tilt`] |
//!
//! ## Coverage (the shot 409)
//! | address | what | port |
//! |---|---|---|
//! | 0x2ecce8 | `CreateMoby(409)`; update distance 0xff, draw distance 0x7e, state 1, +0x31 = 1; position; velocity; yaw / pitch from it; +0x10 owner, +0x18 0, +0x1c heading, +0x24 speed, +0x28 0, +0x14 = `ticks(life)`; `MobyBuildMatrix` | [`spawn_shot`] |
//! | 0x2ece00 state 1 | the hit (0x230000) from another than its owner → 2 | [`shot_update`] |
//! | 0x2ece00 state 1 | level 12, Ratchet in state 0x32, an owner: owner gone → deleted; owner in state < 2 → 2 | [`shot_update`] |
//! | 0x2ece00 state 1 | position += velocity; yaw = +0x1c; roll += 2π·dt; two puffs at pos − 0.2·dir (`PartType04Spawn(…, 0.2·dir, 0x6f00afff, 0xff, ticks(rand_range(15, 22)), 0x28, rand_range(20, 35), 1)`, `(…, 0x1fffffff, 0x4f4f4f, ticks(rand_range(30, 60)), 0x28, rand_range(50, 75), 0)`) | [`shot_update`] |
//! | 0x2ece00 state 1 | `CollLine_Fix(pos, old, 0, self)` or `coll_sphere(0.2, pos, 0, self)`: a moby hit other than the owner gets the hit (0x26e808 + 0x26e968: damage 1, level 12 in state 0x32: 3, past 15 mounts 2; flags 0x10003, dir (cos, sin, 1, 5627.98), type 1 / 1, the class); → 2 | [`shot_update`] |
//! | 0x2ece00 state 1 | `FastDecTimer(+0x14)` out → 2 | [`shot_update`] |
//! | 0x2ece00 state 2 | moved 0.4 away from the camera; level 12 in state 0x32: `SpawnBeamExplosion(0, 0, 0.3, 0.2, 0, 0.15, 0, self, 0, its position, 3, 3, 5, −1, 0, 0, …)` and the camera shake 0.025 for `ticks(20)` (up); else the death explosion `0x273f50(0.25, 13, self, pos, 0)`; deleted | [`shot_update`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::fx::{beam_explosion, death_explosion, part04, Beam};
use crate::moby_update::creature::{self as c, add, add_rot, atan, dist2, dist3, len2, set_len3, sub, sub_rot, DT, DT2};
use crate::moby_update::services::{fl, HitTemplate, World};
use super::hoven_turret::lw as tw;

pub const REFERENCE_LEVEL: u32 = 12;
pub const UPDATE_FN: u32 = 0x2e_9f68;
pub const CLASSES: [i16; 1] = [326];
pub const SHOT_FN: u32 = 0x2e_ce00;
pub const SHOT_CLASSES: [i16; 1] = [SHOT];
/// The muzzles (joint lists 0 / 1) of the drone are read.
pub const JOINTS: [i16; 1] = [326];

const SHOT: i16 = 0x199;
const GUN_SHOT: i16 = 0xb8;
const SHELL: i16 = 0x1ca;
/// The follow camera's focus moby 0x16735c (index + 1; 0 none) as the moby loop sees it.
pub const FOCUS: u32 = 0x16_735c;

pub mod pvo {
    pub const D: usize = 0x20;
    pub const B58: usize = 0x58;
    pub const FLASH: usize = 0x60;
    pub const STEP: usize = 0x70;
    pub const CUBOID: usize = 0x80;
    pub const YAW_V: usize = 0x88;
    pub const FIRE_T: usize = 0x8c;
    pub const KNOCK_T: usize = 0x8e;
    pub const BOB: usize = 0x90;
    pub const SPEED: usize = 0x94;
    pub const ROLL_V: usize = 0x98;
    pub const PITCH_V: usize = 0x9c;
    pub const PATH_A: usize = 0xa0;
    pub const PATH_B: usize = 0xa4;
    pub const RIDER: usize = 0xac;
    pub const VOICE: usize = 0xb0;
    pub const POINT: usize = 0xb4;
    pub const BARREL: usize = 0xb8;
    pub const SHOTS: usize = 0xbc;
    pub const SIDE: usize = 0xbe;
    pub const TURRET: usize = 0xc0;
    pub const START_T: usize = 0xc4;
    pub const HEAD: usize = 0xd0;
    pub const LEN: usize = 0x110;
}

mod so {
    pub const VEL: usize = 0x00;
    pub const OWNER: usize = 0x10;
    pub const LIFE: usize = 0x14;
    pub const F18: usize = 0x18;
    pub const YAW: usize = 0x1c;
    pub const SPEED: usize = 0x24;
    pub const F28: usize = 0x28;
    pub const LEN: usize = 0x30;
}

const DEG45: f32 = f32::from_bits(0x3f49_0fdb);

/// The death blast.
const DEATH: Beam = Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.0, flash_dist: 100_000.0, scale: 3.0, light: 15.0, streaks: 20, sparks: 3, puffs: 4, debris: 1, sound: 2, shake: true };
/// The shot's blast in the turret game.
const SHOT_BLAST: Beam = Beam { damage_r: 0.0, damage: 0.0, flash: 0.3, flash2: 0.2, flash_dist: 0.0, scale: 0.15, light: 0.0, streaks: 3, sparks: 3, puffs: 5, debris: 0, sound: -1, shake: false };

fn word(w: &World, a: u32) -> u32 { w.svc.units.word(a) }
fn link1(w: &World, v: i32) -> Option<MobyId> { usize::try_from(v - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn rider(w: &World, id: MobyId) -> Option<MobyId> { link1(w, c::pi32(w, id, pvo::RIDER)) }
fn turret_mode(w: &World, id: MobyId) -> bool { c::pi32(w, id, pvo::TURRET) != 0 }

/// The focus moby (0x16735c) as the moby loop sees it: gone (state ≥ 0x80) reads as none (the camera clears it).
pub fn focus(w: &World) -> Option<MobyId> { link1(w, word(w, FOCUS) as i32).filter(|&m| w.m(m).state < 0x80) }
fn set_focus(w: &mut World, m: Option<MobyId>) {
    w.svc.units.set_word(FOCUS, m.map_or(0, |m| m as u32 + 1));
    crate::cinematic::focus_moby(w, m);
}
/// `if (0x16735c == self) 0x16735c = 0`.
fn drop_focus(w: &mut World, id: MobyId) { if focus(w) == Some(id) { set_focus(w, None); } }
/// `if (0x16735c == 0) { 0x167360 = 0; 0x16735c = self }`.
fn take_focus(w: &mut World, id: MobyId) {
    if focus(w).is_none() {
        crate::cinematic::focus_ticks(w, 0);
        set_focus(w, Some(id));
    }
}

fn path(w: &World, i: i32) -> Vec<[f32; 4]> {
    usize::try_from(i).ok().and_then(|i| w.svc.splines.get(i)).map(|v| v.iter().map(|q| q.map(f32::from_bits)).collect()).unwrap_or_default()
}

/// `0x27ba70(path)`: each point's w = the distance to the next one (the last point keeps its w).
pub fn segment_lengths(w: &mut World, i: i32) {
    let Some(sp) = usize::try_from(i).ok().and_then(|i| w.svc.splines.get_mut(i)) else { return };
    let n = sp.len();
    for k in 0..n.saturating_sub(1) {
        let (a, b) = (sp[k].map(f32::from_bits), sp[(k + 1) % n].map(f32::from_bits));
        let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        sp[k][3] = d.to_bits();
    }
}

/// Level12 `0x2e9f68` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    crate::moby_update::story::pvars(w, id, pvo::LEN);
    if w.svc.game_mode == 2 { return; }
    let tgt = c::target::acquire(w, id, 70.0);
    if let Some(r) = rider(w, id) {
        if w.m(r).state < 0x80 { crate::moby_update::manip::big_head(w, f32::from_bits(0x402c_cccd), r, 0, id, pvo::HEAD); }
    }
    intake(w, id);
    let giant = w.body() == 2;
    let s = super::class_scale(w, 326);
    w.mm(id).scale = if giant { s * 0.5 } else { s };
    if let Some(r) = rider(w, id) {
        let rs = super::class_scale(w, w.m(r).o_class);
        w.mm(r).scale = if giant { rs * 0.5 } else { rs };
    }
    if (2..=5).contains(&w.m(id).state) {
        if c::pi32(w, id, pvo::VOICE) == -1 {
            let v = w.play_sound(3, 4, id);
            c::set_pi32(w, id, pvo::VOICE, v);
        }
    } else {
        release_voice(w, id);
    }
    let tm = turret_mode(w, id);
    if w.svc.level == 0xc && word(w, tw::ACTIVE) != 0 && !tm {
        drop_focus(w, id);
        if w.m(id).visible == 0 {
            if let Some(r) = rider(w, id) {
                w.delete_moby(r);
                c::set_pi32(w, id, pvo::RIDER, 0);
            }
            w.delete_moby(id);
            return;
        }
    }
    if tm {
        if super::hints::mission_done(w, w.m(id).mission as i32) {
            if let Some(r) = rider(w, id) {
                w.delete_moby(r);
                c::set_pi32(w, id, pvo::RIDER, 0);
            }
            w.delete_moby(id);
            return;
        }
        if word(w, tw::CARRIER_CAM) != 0 {
            if let Some(r) = rider(w, id) {
                super::drone_rider::knock_off(w, id, r);
                c::set_pi32(w, id, pvo::RIDER, 0);
            }
            w.delete_moby(id);
            return;
        }
    }
    match w.m(id).state {
        0 => init(w, id),
        1 => {
            let mut go = false;
            if tm && word(w, tw::ACTIVE) != 0 { go = c::dec_timer_pvar_i32(w, id, pvo::START_T) != 0; }
            let cub = c::pi32(w, id, pvo::CUBOID);
            if (cub != -1 && w.in_cuboid(w.hero_point(), cub)) || go {
                c::set_pu8(w, id, pvo::B58, 0x16);
                c::set_pu8(w, id, pvo::B58 + 1, 1);
                c::set_pu8(w, id, pvo::B58 + 2, 7);
                w.mm(id).hit_slot = 0xff;
                w.mm(id).mode &= !(mode::HIDDEN | mode::NO_ANIM);
                if let Some(r) = rider(w, id) { w.mm(r).mode &= !(mode::HIDDEN | mode::NO_ANIM); }
                w.mm(id).mode |= mode::TARGETABLE;
                w.mm(id).state = 2;
                let coll = super::class_collision(w, 326);
                w.mm(id).has_collision = coll;
            }
        }
        2 => {
            if !tm {
                take_focus(w, id);
            } else {
                let mut n = word(w, tw::MOUNTS) as i32 - 3;
                n = n.clamp(0, 5);
                let t15 = w.ticks(15);
                if c::pi16(w, id, pvo::FIRE_T) < (n * t15) as i16 { c::set_pi16(w, id, pvo::FIRE_T, (n * t15) as i16); }
            }
            let pa = c::pi32(w, id, pvo::PATH_A);
            if follow_path(w, id, pa) { w.mm(id).state = 3; }
        }
        3 => attack(w, id, &tgt),
        4 => {
            let tp = tgt.pos;
            let p = w.m(id).position;
            spring_yaw(w, id, atan(tp[0] - p[0], tp[1] - p[1]));
            let pts = path(w, c::pi32(w, id, pvo::PATH_B));
            if pts.is_empty() { return tail(w, id); }
            let goal = if c::pi16(w, id, pvo::SIDE) == 0 { pts[pts.len() - 1] } else { pts[0] };
            let d = sub(goal, p);
            let mut sp = c::pf(w, id, pvo::SPEED);
            if c::len3(d) <= 5.5 {
                c::turn::approach(0.0, DT2 * 9.0, &mut sp);
                c::set_pf(w, id, pvo::SPEED, sp);
                if sp == 0.0 {
                    let t = w.ticks(0x32) as i16;
                    c::set_pi16(w, id, pvo::FIRE_T, t);
                    let side = c::pi16(w, id, pvo::SIDE);
                    c::set_pi16(w, id, pvo::SIDE, (side == 0) as i16);
                    w.mm(id).state = 3;
                    return tail(w, id);
                }
            } else {
                c::turn::approach(DT * 10.0, DT2 * 7.0, &mut sp);
                c::set_pf(w, id, pvo::SPEED, sp);
            }
            let step = set_len3([d[0], d[1], d[2], 0.0], sp);
            c::set_pv4(w, id, pvo::STEP, step);
            w.mm(id).position = add(p, step);
            tilt(w, id);
        }
        5 => {
            if c::dec_timer_pvar_s16(w, id, pvo::KNOCK_T) == 0 {
                let mut sp = c::pf(w, id, pvo::SPEED) - DT2 * 4.0;
                if sp < 0.0 { sp = 0.0; }
                c::set_pf(w, id, pvo::SPEED, sp);
                let y = w.m(id).rotation[2];
                let p = w.m(id).position;
                w.mm(id).position = [p[0] + y.cos() * -sp, p[1] + y.sin() * -sp, p[2], p[3]];
            } else {
                let s = w.m(id).cmd;
                w.mm(id).state = s;
            }
        }
        6 => {
            drop_focus(w, id);
            if let Some(r) = rider(w, id) {
                super::drone_rider::knock_off(w, id, r);
                c::set_pi32(w, id, pvo::RIDER, 0);
            }
            let p = w.m(id).position;
            beam_explosion(w, &DEATH, Some(id), p);
            if !tm { crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1); }
            release_voice(w, id);
            if !tm {
                w.delete_moby(id);
                return;
            }
            let (a, b) = (w.ticks(0xf0), w.ticks(400));
            let t = w.rng.rand_range(a, b);
            c::set_pi32(w, id, pvo::START_T, t);
            let m = w.mm(id);
            m.has_collision = false;
            m.state = 0;
            m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN | mode::NO_ANIM;
        }
        _ => {}
    }
    tail(w, id);
}

fn release_voice(w: &mut World, id: MobyId) {
    let v = c::pi32(w, id, pvo::VOICE);
    if v != -1 && w.sound_owner(v) == Some(id) { w.release_sound(v, id); }
    c::set_pi32(w, id, pvo::VOICE, -1);
}

/// The bob, the rider and the shadow (module doc).
fn tail(w: &mut World, id: MobyId) {
    let ph = c::pf(w, id, pvo::BOB);
    w.mm(id).position[2] -= ph.cos() * 0.17;
    let n = w.ticks(0x78) as f32;
    let ph = add_rot(ph, f32::from_bits(0x40c9_0fdb) / n);
    c::set_pf(w, id, pvo::BOB, ph);
    w.mm(id).position[2] += ph.cos() * 0.17;
    if let Some(r) = rider(w, id) {
        let (p, e) = (w.m(id).position, w.m(id).rotation);
        let m = w.mm(r);
        m.position = p;
        m.rotation = e;
        crate::moby_update::anim_sound::advance(w, r);
        w.build_matrix(r);
        w.mm(r).mode |= mode::NO_UPDATE | mode::KEEP_MATRIX;
    } else if !turret_mode(w, id) && 1 < w.m(id).state && w.m(id).visible != 0 {
        let cam = w.camera_point();
        if dist3(w.m(id).position, [cam[0], cam[1], cam[2], 0.0]) < 30.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0x18;
        }
    }
}

/// State 0 (module doc).
fn init(w: &mut World, id: MobyId) {
    c::set_pu8(w, id, pvo::D + 8, 2);
    c::set_pf(w, id, pvo::D, 6.0);
    c::set_pi16(w, id, pvo::D + 4, 6);
    c::set_pu8(w, id, pvo::D + 9, 0);
    c::set_pf(w, id, pvo::SPEED, 0.0);
    c::set_pf(w, id, pvo::ROLL_V, 0.0);
    c::set_pf(w, id, pvo::PITCH_V, 0.0);
    w.mm(id).has_collision = false;
    let m = w.mm(id);
    m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN | mode::NO_ANIM;
    let (pa, pb) = (c::pi32(w, id, pvo::PATH_A), c::pi32(w, id, pvo::PATH_B));
    segment_lengths(w, pa);
    segment_lengths(w, pa);
    segment_lengths(w, pb);
    if w.m(id).draw_dist < 0x80 { w.mm(id).draw_dist = 0x80; }
    if rider(w, id).is_none() {
        if let Some(r) = super::drone_rider::spawn(w, id) {
            c::set_pi32(w, id, pvo::RIDER, r as i32 + 1);
            w.mm(r).update_dist = 0;
            w.mm(r).mode |= mode::HIDDEN | mode::NO_ANIM;
        }
    }
    if let Some(&p0) = path(w, pa).first() { w.mm(id).position = p0; }
    if turret_mode(w, id) && c::pi32(w, id, pvo::START_T) == 0 {
        let (a, b) = (w.ticks(0xf0), w.ticks(400));
        let t = w.rng.rand_range(a, b);
        let mut n = word(w, tw::MOUNTS) as i32 - 1;
        n = n.clamp(0, 10);
        let t35 = w.ticks(0x23);
        c::set_pi32(w, id, pvo::START_T, t + n * t35);
    }
    c::set_pi32(w, id, pvo::POINT, 0);
    w.mm(id).state = 1;
}

/// `SpringTurn` of the yaw (0.004, 0.3, 45°·dt, +0x88).
fn spring_yaw(w: &mut World, id: MobyId, target: f32) {
    let mut v = c::pf(w, id, pvo::YAW_V);
    let y = c::turn::spring_turn(w.m(id).rotation[2], target, f32::from_bits(0x3b83_126f), f32::from_bits(0x3e99_999a), DT * DEG45, &mut v);
    w.mm(id).rotation[2] = y;
    c::set_pf(w, id, pvo::YAW_V, v);
}

/// `0x2eaf68(self, path)`: true at the end of the path (module doc).
fn follow_path(w: &mut World, id: MobyId, pi: i32) -> bool {
    let pts = path(w, pi);
    let n = pts.len() as i32;
    if n == 0 { return false; }
    let i = c::pi32(w, id, pvo::POINT).clamp(0, n - 1);
    let rest: f32 = (i..n - 1).map(|k| pts[k as usize][3]).sum();
    let p = w.m(id).position;
    let pt = pts[i as usize];
    let mut sp = c::pf(w, id, pvo::SPEED);
    if rest <= 5.0 {
        let t = c::target::acquire(w, id, 48.0).pos;
        spring_yaw(w, id, atan(t[0] - p[0], t[1] - p[1]));
        c::turn::approach(0.0, DT2 * 9.0, &mut sp);
        c::set_pf(w, id, pvo::SPEED, sp);
        if sp == 0.0 { return true; }
    } else {
        spring_yaw(w, id, atan(pt[0] - p[0], pt[1] - p[1]));
        c::turn::approach(DT * 10.0, DT2 * 7.0, &mut sp);
        c::set_pf(w, id, pvo::SPEED, sp);
    }
    let d = sub(pt, p);
    let step = set_len3([d[0], d[1], d[2], 0.0], sp);
    c::set_pv4(w, id, pvo::STEP, step);
    let np = add(p, step);
    w.mm(id).position = np;
    if dist3(np, pt) < 2.0 {
        let k = (i + 1).min(n - 1);
        c::set_pi32(w, id, pvo::POINT, k);
    }
    tilt(w, id);
    false
}

/// `0x2eae00`: the tilt (module doc).
fn tilt(w: &mut World, id: MobyId) {
    let st = c::pv4(w, id, pvo::STEP);
    let a = sub_rot(atan(st[0], st[1]), w.m(id).rotation[2]);
    let sp = c::pf(w, id, pvo::SPEED);
    let k = f32::from_bits(0x3e68_5696);
    let roll = (sp * -k * a.sin()) / (DT * 10.0);
    let pitch = (sp * k * a.cos()) / (DT * 10.0);
    let mut v = c::pf(w, id, pvo::PITCH_V);
    let ry = c::turn::spring_turn(w.m(id).rotation[1], pitch, DT2 * f32::from_bits(0x3f06_0a92), DT2 * f32::from_bits(0x3f86_0a92), DT * DEG45, &mut v);
    w.mm(id).rotation[1] = ry;
    c::set_pf(w, id, pvo::PITCH_V, v);
    let mut v = c::pf(w, id, pvo::ROLL_V);
    let rx = c::turn::spring_turn(w.m(id).rotation[0], roll, DT2 * f32::from_bits(0x3f86_0a92), DT2 * f32::from_bits(0x4006_0a92), DT * f32::from_bits(0x3f9c_61aa), &mut v);
    w.mm(id).rotation[0] = rx;
    c::set_pf(w, id, pvo::ROLL_V, v);
}

/// State 3 (module doc).
fn attack(w: &mut World, id: MobyId, tgt: &c::target::Target) {
    if w.svc.level == 0xc && !super::hints::owned(w, 11) && w.svc.help.records.help[0x5d].count == 0 { w.svc.help.request(0x2ee2, 0x5d); }
    let p = w.m(id).position;
    let h = w.hero_point();
    let tm = turret_mode(w, id);
    if !tm {
        let d = dist2(p, [h[0], h[1], h[2], 0.0]);
        if 21.0 <= d || p[2] - h[2] <= 1.0 { drop_focus(w, id); } else { take_focus(w, id); }
    }
    let tp = tgt.pos;
    if !((dist2(p, tp) < 40.0 && -2.0 < p[2] - tp[2]) || tm) { return; }
    spring_yaw(w, id, atan(tp[0] - p[0], tp[1] - p[1]));
    let mut v = c::pf(w, id, pvo::PITCH_V);
    let ry = c::turn::spring_turn(w.m(id).rotation[1], atan(dist2(tp, p), p[2] - tp[2]), f32::from_bits(0x3b83_126f), f32::from_bits(0x3e99_999a), DT * DEG45, &mut v);
    w.mm(id).rotation[1] = ry;
    c::set_pf(w, id, pvo::PITCH_V, v);
    c::set_pv4(w, id, pvo::STEP, [0.0; 4]);
    tilt(w, id);
    if c::dec_timer_pvar_s16(w, id, pvo::FIRE_T) == 0 { return; }
    fire(w, id, tgt);
    let mut burst = 4;
    if tm {
        let n = word(w, tw::MOUNTS) as i32;
        if 10 < n { burst = 2; }
        if 6 < n { burst = 3; }
    }
    if c::pi16(w, id, pvo::SHOTS) < burst || c::pi32(w, id, pvo::PATH_B) == -1 {
        let t = w.ticks(0x28) as i16;
        c::set_pi16(w, id, pvo::FIRE_T, t);
    } else {
        c::set_pi16(w, id, pvo::SHOTS, 0);
        w.mm(id).state = 4;
    }
}

/// A shot (module doc).
fn fire(w: &mut World, id: MobyId, tgt: &c::target::Target) {
    let n = c::pi16(w, id, pvo::SHOTS) + 1;
    c::set_pi16(w, id, pvo::SHOTS, n);
    let b = c::pi32(w, id, pvo::BARREL);
    let muzzle = w.joint_point(id, b as usize);
    c::set_pi32(w, id, pvo::BARREL, (b == 0) as i32);
    let tm = turret_mode(w, id);
    let mut aim = tgt.aim;
    if tm {
        let cam = w.camera_point();
        aim = [cam[0], cam[1], cam[2], fl(w.camera[3])];
        aim[2] += w.rng.randf(-0.5, f32::from_bits(0x3dcc_cccd));
        aim[0] += w.rng.randf_sym(0.0, f32::from_bits(0x3ecc_cccd));
    }
    let d = sub(aim, muzzle);
    let v = set_len3([d[0], d[1], d[2], 0.0], DT * if tm { 22.0 } else { 15.0 });
    let yaw = atan(v[0], v[1]);
    let life = w.ticks(0x82);
    spawn_shot(w, yaw, DT * 15.0, v, muzzle, Some(id), life);
    w.play_sound(0, 0, id);
    let p = add(muzzle, set_len3(v, 0.5));
    for _ in 0..25 {
        let vv = [w.rng.randf_sym(0.0, DT * 1.3), w.rng.randf_sym(0.0, DT * 1.3), w.rng.randf_sym(0.0, DT * 1.3), 0.0];
        let r = w.rng.rand_range(0xaa, 0xf0) as u32;
        let g = w.rng.rand_range(0x6e, 0xb4) as u32;
        let tl = w.rng.rand_range(0x1e, 0x46);
        let life = w.ticks(tl);
        let x = w.rng.rand_range(0x78, 0xaa);
        part04(w, &crate::particles::type04::Spawn { pos: p, vel: vv, c1: g << 8 | r | 0x7f00_0000, c2: 0xff, life, base: 0x1e, growth: x as i16, additive: true });
    }
}

/// `0x2eb220`: the hits (module doc).
fn intake(w: &mut World, id: MobyId) {
    let hit = w.get_hit(id, 0x33_0000, false);
    let r = c::damage::resolve(w, id, hit, pvo::D, 0, 4);
    if r.out5 != 1 && w.m(id).state != 6 {
        if let Some(rec) = r.hit.or(hit) {
            let class = rec.attacker.map_or(-1, |a| w.m(a).o_class);
            if class != GUN_SHOT && class != SHOT {
                let dmg = if class == SHELL { 10.0 } else { r.damage };
                let hp = c::pf(w, id, pvo::D) - dmg;
                c::set_pf(w, id, pvo::D, hp);
                if r.reaction == 1 || hp <= 0.0 {
                    w.play_sound(2, 0, id);
                    w.mm(id).state = 6;
                } else if r.reaction != 0xb {
                    c::set_pu8(w, id, pvo::FLASH + 7, 0xb4);
                    c::flash::start(w, id, pvo::FLASH);
                    if fl(rec.dir[3]) == crate::hero::damage::EXACT_PUSH_W && w.m(id).state != 5 {
                        let p = w.m(id).position;
                        let a = sub_rot(atan(p[0] - fl(rec.pos[0]), p[1] - fl(rec.pos[1])), w.m(id).rotation[2]);
                        let (cc, s) = (a.cos(), a.sin() * -f32::from_bits(0x3eb2_b8c2));
                        if w.m(id).rotation[0].abs() < s.abs() {
                            w.mm(id).rotation[0] = s;
                            c::set_pf(w, id, pvo::ROLL_V, 0.0);
                        }
                        let pt = cc * f32::from_bits(0x3db2_b8c2);
                        if w.m(id).rotation[1].abs() < pt.abs() {
                            w.mm(id).rotation[1] = pt;
                            c::set_pf(w, id, pvo::PITCH_V, 0.0);
                        }
                        c::set_pf(w, id, pvo::SPEED, DT * 4.0);
                        let t = w.ticks(0x3c) as i16;
                        c::set_pi16(w, id, pvo::KNOCK_T, t);
                        let st = w.m(id).state;
                        w.mm(id).cmd = st;
                        w.mm(id).state = 5;
                        w.play_sound(1, 0, id);
                    }
                }
            }
        }
    }
    w.mm(id).hit_slot = 0xff;
    c::flash::update(w, id, pvo::FLASH);
}

/// Level12 `0x2ecce8(yaw, speed, 0, vel, pos, owner, life)` (module doc).
pub fn spawn_shot(w: &mut World, yaw: f32, speed: f32, vel: [f32; 4], pos: [f32; 4], owner: Option<MobyId>, life: i32) -> Option<MobyId> {
    let id = w.create_moby(SHOT)?;
    crate::moby_update::story::pvars(w, id, so::LEN);
    {
        let m = w.mm(id);
        m.update_dist = 0xff;
        m.draw_dist = 0x7e;
        m.state = 1;
        m.visible = 1;
        m.position = pos;
        m.rotation[2] = atan(vel[0], vel[1]);
        m.rotation[1] = -atan(len2(vel), vel[2]);
    }
    c::set_pv4(w, id, so::VEL, vel);
    let t = w.ticks(life);
    c::set_pi32(w, id, so::OWNER, owner.map_or(0, |o| o as i32 + 1));
    c::set_pi32(w, id, so::F18, 0);
    c::set_pf(w, id, so::YAW, yaw);
    c::set_pf(w, id, so::SPEED, speed);
    c::set_pi32(w, id, so::F28, 0);
    c::set_pi32(w, id, so::LIFE, t);
    w.build_matrix(id);
    Some(id)
}

/// Level12 `0x2ece00` (module doc).
pub fn shot_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < so::LEN { w.delete_moby(id); return; }
    let owner = link1(w, c::pi32(w, id, so::OWNER));
    let ridden = w.svc.level == 0xc && w.hero.state == 0x32;
    if w.m(id).state == 1 {
        let hit = w.get_hit(id, 0x23_0000, false);
        let own_hit = hit.is_none_or(|h| h.attacker == owner);
        let mut down = !own_hit;
        if own_hit {
            if let (true, Some(o)) = (ridden, owner) {
                if w.m(o).state >= 0x80 {
                    w.delete_moby(id);
                    return;
                }
                if w.m(o).state < 2 { down = true; }
            }
        }
        if !down {
            let old = w.m(id).position;
            let vel = c::pv4(w, id, so::VEL);
            let p = add(old, vel);
            w.mm(id).position = p;
            let yaw = c::pf(w, id, so::YAW);
            w.mm(id).rotation[2] = yaw;
            let rx = w.m(id).rotation[0];
            w.mm(id).rotation[0] = add_rot(rx, DT * f32::from_bits(0x40c9_0fdb));
            let v1 = set_len3(vel, f32::from_bits(0x3e4c_cccd));
            let back = add(set_len3(vel, -f32::from_bits(0x3e4c_cccd)), p);
            let back = [back[0], back[1], back[2], p[3]];
            let n = w.rng.rand_range(0xf, 0x16);
            let life = w.ticks(n);
            let g = w.rng.rand_range(0x14, 0x23);
            part04(w, &crate::particles::type04::Spawn { pos: back, vel: v1, c1: 0x6f00_afff, c2: 0xff, life, base: 0x28, growth: g as i16, additive: true });
            let n = w.rng.rand_range(0x1e, 0x3c);
            let life = w.ticks(n);
            let g = w.rng.rand_range(0x32, 0x4b);
            part04(w, &crate::particles::type04::Spawn { pos: back, vel: v1, c1: 0x1fff_ffff, c2: 0x4f_4f4f, life, base: 0x28, growth: g as i16, additive: false });
            let pv_ = crate::moby_update::services::pv;
            let h = w.coll_line(pv_(p), pv_(old), 0, Some(id)).or_else(|| w.coll_sphere(pv_(p), crate::moby_update::services::pf(f32::from_bits(0x3e4c_cccd)), 0, Some(id)));
            if let Some(h) = h {
                if let Some(hm) = h.moby.filter(|&m| Some(m) != owner) {
                    let mut dmg = 1.0;
                    if ridden { dmg = if 0xf < word(w, tw::MOUNTS) as i32 { 2.0 } else { 3.0 }; }
                    let yaw = c::pf(w, id, so::YAW);
                    let t = HitTemplate {
                        dir: pv_([yaw.cos(), yaw.sin(), 1.0, f32::from_bits(0x45af_df66)]),
                        attacker: Some(id),
                        flags: 0x1_0003,
                        b18: 1,
                        b19: 1,
                        h1a: w.m(id).o_class as u16,
                        damage: crate::moby_update::services::pf(dmg),
                        w20: 1,
                    };
                    w.deliver_hit(hm, &t);
                }
                down = true;
            }
            if down { w.mm(id).state = 2; }
            if c::dec_timer_pvar_i32(w, id, so::LIFE) == 0 { return; }
        }
        w.mm(id).state = 2;
        return;
    }
    if w.m(id).state != 2 { return; }
    let p = w.m(id).position;
    let cam = w.camera_point();
    let d = set_len3(sub(p, [cam[0], cam[1], cam[2], p[3]]), f32::from_bits(0x3ecc_cccd));
    let p = add(p, [d[0], d[1], d[2], 0.0]);
    w.mm(id).position = p;
    if ridden {
        beam_explosion(w, &SHOT_BLAST, Some(id), p);
        let t = w.ticks(0x14);
        w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp: f32::from_bits(0x3ccc_cccd), ticks: t });
    } else {
        death_explosion(w, 0.25, 13.0, Some(id), p, 0);
    }
    w.delete_moby(id);
}
