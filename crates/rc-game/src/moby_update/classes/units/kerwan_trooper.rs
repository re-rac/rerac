//! U130 (census 2026-09-30; U132 in the unit table): class 574, Kerwan's gun troopers (level03 `0x2c6fd0`, the only
//! copy: 19 placed, 17 created), and the rocket 833 they fire (level03 `0x2d43c8`, created by code). The name is
//! descriptive [L]. A ground creature of the shared layer (mode 0x20 header: damage record +0x20, flash +0x60,
//! knockback +0x70, walker +0xd0) that patrols its path (+0x134), turns its head and torso toward Ratchet (two NPC
//! look-at records, joint lists 2 / 3), fires rockets from joint 0 when he is in range and in front, punches him up
//! close, jumps down from a ledge along a second path (+0x290) when its group is alerted or he enters a cuboid, waits
//! with a 573 (+0x284) until he comes, rides the joint-carried platforms 1210 (+0x280: it and its path ride along),
//! and blows up (a beam explosion, three pieces) when killed, when it falls below 5 or 10 under its path, and when a
//! knocked flight lands it off a ledge. With a 822 link (+0x264) it is not deleted but respawns hidden at its path's
//! start (state 0xe), shown again in state 0xf.
//!
//! **Pvars** (0x2a0): +0x20 damage record (health 3, +0x24 the meter, +0x28 / +0x29 column bytes, +0x30 1.75, +0x38
//! the lure), +0x40 the walk's output move, +0x58 / +0x5a bytes 10 / 0xc, +0x60 the flash, +0x70 the knockback record,
//! +0xd0 the walker J, +0x120 the patrol node, +0x124 its step (s8), +0x130 the path (the game: its pointer; the port:
//! the index), +0x134 the path id, +0x150 / +0x1d0 the head / torso look-at records (yaw targets +0x1b8 / +0x238, the
//! head's scale +0x1c0), +0x250 the attack cooldown (s16), +0x252 the alert timer (s16), +0x254 rockets fired (u8),
//! +0x255 the death flight's ticks (u8), +0x256 the jump-down flag (s16), +0x258 the turn velocity, +0x262 hidden until
//! the mission (u8), +0x263 the turn-check pause (u8 timer), +0x264 the 822 link, +0x26c the sight range (+0x270 this
//! tick's: + 5 alerted), +0x274 the patrol mode (0: pick the node toward Ratchet), +0x278 the leash, +0x27c the
//! jump-down range, +0x280 the moby it stands on, +0x284 the 573 it waits with, +0x288 its draw distance, +0x28a its
//! tick slot (of 8), +0x28c the turn-to angle, +0x290 the jump path, +0x294 its range, +0x298 its node, +0x29c the
//! cuboid. The moby's `cmd` (+0xbc) keeps the state to return to after a turn (0xc).
//!
//! The level03 `$gp` words (gp = 0x166c00): gp−0x5320 / −0x531c / −0x5318 = 1, 2.5, 100 (the knockback flights'
//! height, distance and drag), gp−0x530c the troopers' tick-slot counter, gp−0x50d8 0.35 (the rocket's share of
//! Ratchet's platform climb).
//!
//! ## Coverage
//!
//! **The update** `0x2c6fd0`, before the states:
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | `0x24e830(+0x270, m, &rec)` (= `0x274b78`); d = xy distance to the target | | [`update`] (`target::acquire`) |
//! | `0x251d70(2.5, +0x150)`: the head record's scale = 2.5 under the big-head cheat (0x15edb7), else 1.0 | | [`update`] (`manip::big_head_scale`) |
//! | +0x262 = 1: mission (+0xb0) not done → return; done → no collision (+0x94 = 0), mode \|= 0x41, return | the one hidden until its mission | [`update`] |
//! | game mode 0x15f5c4 = 2 → mode \|= 0x41; else (state ≠ 0xe) mode &= ~0x41 | hidden in cutscenes | [`update`] |
//! | out of [2, 1021]³, or no path (+0x134 = −1) outside 0xd → `DeleteMoby` | | [`update`] |
//! | `0x2c95a8` the hits; the byte timer +0x263 (`0x1f8930`) | | [`hits`] |
//! | drawn or d < 24: the head / torso yaw targets (state 0xc: 0.25 / 0.75 of the turn to +0x28c; below 7 and in sight: the torso the whole turn to the target), each clamped to ±45°; `0x250db8(0.03, 0.3, m, +0x150, 2)` / `(…, +0x1d0, 3)` (= `0x2777d8`) | the look-at | [`look`] (`manip::look`) |
//!
//! The states (0 init, 1 stand, 2 wait with a 573, 4 aim, 5 fire, 6 punch, 7 patrol, 8 follow the path down, 9 jump
//! down the jump path, 10 knocked, 0xb fall, 0xc turn, 0xd death flight, 0xe respawn wait, 0xf respawn):
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | 0: +0x7f = 0x1c; Ratchet's moby's light and ambient; `SeedJumpPattern(J)`; J: 3, vz 0, π·dt, 6·dt, 2, π, 4π·dt², 2π·dt², flags 9, arrive 6·dt·15; health 3, meter 3, +0x29 1, +0x28 2, +0x30 1.75, K z 0.7, air 0, radius 716; +0x58 10, +0x5a 0xc; +0x40 zeroed; mode \|= 0x1000; cooldown 0; node 0, at the path's first point; state 1; +0x288 = draw distance; +0x28a = the slot counter & 7 (counter + 1); +0x263 0; each point's w but the last two = the distance to the next (the shared spline written); +0x284 and no +0x290 → seq 0, state 2 | init | [`init`] |
//! | 1: a done sequence not 0 → seq 0; a jump path: at its first point facing its second; the group alerted (`0x2c6d98`), Ratchet in the cuboid (`0x24e3a8`), the mission done, or within +0x294 and 4 in height → node 0, state 9, seq 1 | | [`stand`], [`group_alerted`] |
//! | 1: a 573 link → seq 0, state 2; in sight or jump range: the turn check (`0x2c6e18`); jump range and 4 in height → +0x27c 0, +0x256 1, state 8, seq 1; within 3.5 and 3 in height → 6, seq 2; in sight, 10 in height, cooldown out, facing within 45°: drawn → 4, seq 3 | | [`stand`], [`turn_check`] |
//! | 2: out of sight or seq 10 playing: the 573 alive and seq 10 not done → the turn check; else state 1, seq 0, cooldown `ticks(60)`, +0x284 −1; in sight → seq 10 | waiting with the 573 | [`update`] |
//! | 4: not done: punch range → 6; within the leash and 10 in height → the next node (`0x2c9ca0` or +1), 7; done → 5, rockets 0, seq 4 (0 ticks) | aim | [`aim`] |
//! | 5: key 8 passed (`0x24ff18`): rockets + 1; v = the aim point − joint 0, 6·dt long, z halved; more than 30° off the facing → (cos, sin, 0)·6·dt at facing ± 30°; the muzzle (joint 0 + v·10): `0x250ae8` the smoke ring; `0x2d4288(heading, 6·dt, 0.75, v, joint 0, m, ticks(130))` the rocket | fire | [`fire`] (`fx::muzzle_smoke`, [`spawn_rocket`]) |
//! | 5: not done: as 4 (punch range only after the turn check); done and fired: jump range → 8; the turn check; in sight, 10 in height, cooldown out, facing within 5°, drawn → keep firing; else punch range → 6 | | [`fire`] |
//! | 6: `MobyAnimKeyTime` (0x23d938); `SpringTurn2(target, 0.03, 0.3, 0.3)`; joint 0 read (unused); keys 25..31, 2 in height, within 3.5, facing within 15°: the hit (`0x2484f0` = `0x26e968`, flags 0x10001, damage 1, type 0 / 1, push (cos, sin, 1) exact) on the target moby; done → the next node, 7 (or 1 at the path's end) | punch | [`punch`] |
//! | 7: +0x256 → 8; the turn check toward the node; `0x247c10` (= `0x26de80`) walk; arrived (0x14): out of the leash or 10 in height → 1, seq 0, the turn check; else the next node, 7 | patrol | [`patrol`] |
//! | 8: the path's nearest segment (`0x24c9b0` = `0x272e28`, 1000 / 5) at least the node; walk; arrived: the end → 1; else node + 1 | follow down | [`update`] (`spline::nearest`, `walker::walk_to`) |
//! | 9: the same on the jump path (+0x298); its end → +0x290 −1, 1, seq 0; a 573 → seq 0, state 2 | jump down | [`update`] |
//! | 10: phase 2 → anim speed 1, result = done bit; else `0x24b0e0` (= `0x271558`); +0x290 −1; not done: z < 5 → [`explode`] (death bits 0); done: the next node, 7 (or 1 and the turn check); ground (`0x248348` = `0x26e690`) more than 5 below → class sound 10 (flags 0x21), seq 0xb, 0xb | knocked | [`knocked`] |
//! | 0xb: K vz −= gravity; the xy speed `Approach`ed (0x24a2b0) to the air speed (0 landed) by the drag; moved; a long move (over the radius ×1024: never) `CollLine_Fix` 0x24; `coll_sphere(radius, 0x24)` (0x1ea6c8): the pushed centre, z by vz only; z < 5 → [`explode`] (death bits 0x200); then falls into 0xc | fall | [`fall`] |
//! | 0xc: seq 1: `SpringTurn2(+0x28c, 0.09, 0.27, 0.15)`; seq 9: `(0.01, 0.3, 0.2)`; within 2°: +0x263 `ticks(60)`; back to `cmd` (7 / 8: seq 1, 7 picks the node; seq 1 sets anim speed 1) or 1 (seq 0, cooldown `ticks(10)`) | turn | [`turn`] |
//! | 0xd: `0x24b0e0`; +0x255 + 1; done, 5 over the ground, `cmd` ≠ 0xb → class sound 10, `cmd` 0xb; mode &= ~0x1000; landed → +0x255 0; done → [`explode`] (death bits 0); else z < 5 → `SetDeathBits(m, 0, −1)` and the respawn / delete | death flight | [`dying`] |
//! | 0xe: seq 0; mode &= ~0x1000, \|= 0x41; at the path's start; no collision | respawn wait | [`update`] |
//! | 0xf: the mission done → `DeleteMoby`; the class collision; mode \|= 0x1000, &= ~0x41; health = the meter; at the path's start; 1 | respawn (no setter found on the disc) | [`update`] |
//!
//! **The tail** (every state that did not return):
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | drawn and within 36 of the camera (0x166ec0): `0x248ba8` (= `0x26f020`) | the shadow probe | [`tail`] (`shadows::probe_down`) |
//! | +0x280 gone → −1; a class 821 / 1210: every path point carried (`0x24eaf0` = `0x2752c0`); the position carried, the yaw the carried rotation's z | riding a platform | [`tail`] (`triggers::carried`) |
//! | not 0xb..0xe: +0xe8 = 0 (10) or += 9.8·dt²; z −= it; `GroundHeight(0.5, pos, 0)` (0x2482d0): below → stand on it (a hit moby with a platform block becomes +0x280; else a +0x280 not 821 / 1210 is dropped); `0x24fba8` (its result is written to a stack copy: no effect); more than 5 above → seq 0xb, 0xb, K zeroed, vz −+0xe8, gravity 19.6·dt² | gravity | [`tail`] |
//! | +0x262 = 0, within 70 (xy) of the camera, on its tick slot (0x15f5cc & 7): the path's nearest point; z < 5 or 10 below it → [`explode`] (death bits 0x200) | fell off | [`tail`] |
//!
//! **[`explode`]**: `SpawnBeamExplosion(0, 0, 2, 1, 9, 1, 15, m, +0x40, pos + 1 z, 5, 2, 4, 6, 1, 1, −1, 0)` (0x24ce98;
//! class sound 6, shake, one fireball), `BreakFxB` 0x6d0 / 0x6d1 / 0x6d2 (0x2520b0), `SetDeathBits(m, bits, −1)`
//! (0x245fe0); a 822 link → at the path's start, +0x40 zeroed, +0xe8 0, state 0xe hidden without collision; else
//! `DeleteMoby`.
//!
//! **The hits** `0x2c95a8`: the lure → the alert `ticks(240)`; sight + 5 while alerted; `MobyGetHitMessage(0x330000)`,
//! the resolver (`0x248f00` = `0x26f378`, column 4); out5 ≠ 1 and not 0xd: health −= damage (≤ 0 → 1). 1: K gravity
//! 40·dt², drag 100·dt², the flight (`0x2495d0` = `0x26fa48` on the hit's push; seq 7, 5 ticks, frame 2), keys 10 / 20,
//! air 2·dt, +0x255 0, untargetable, flash 0xfa, 0xd, class sound 3, seq 7 (6); 3 / 7 / 8: flash 0x78, gravity 80·dt²,
//! seq 5 (frame 0), keys 6 / 13; 4 / 5: seq 6 (frame 2), keys 5 / 10; 6: half the push, facing within 60° → turned
//! away, seq 5 then the hard cut `fun_00212ed8(m, 5, 5)`, keys 14 / 28; then 10 and class sound 1; 0xc / 0xd: flash
//! 0x78; every passed hit: `0x24bea0` (= `0x272318`) the flash. Always: +0xa4 = 0xff, `0x24bf80` the flash update.
//! (The attacker heading 0x2c96f0 is overwritten by `0x26fa48`: a dead store.) [`hits`].
//!
//! **The turn check** `0x2c6e18(h, m, rec)`: no target moby → 0; a 822 link whose `cmd` ≠ 4 → 0; more than 45° off
//! (any when +0x263 runs or seq 1 plays) and at most 90° → 0; within 20 of the target: beyond 67.5° → seq 1 (anim speed
//! 1), else seq 9 (both unguarded, ticks 10); +0x28c = h, `cmd` = the state, 0xc → 1. [`turn_check`].
//!
//! **The next node** `0x2c9ca0`: a fresh target; the current segment (`0x24c9b0`); every node of the path (stepping
//! +0x124 from 0 until it wraps to 0) but the current one is weighed by its turn from the target's direction and its
//! distance from the target (the game's selection, kept branch for branch). [`pick_node`].
//!
//! **The group** `0x2c6d98(group)`: a live member that is a 574 in state 9 (`0x2c6d58`) or a 573 in state 5
//! (`0x2c5b78`). [`group_alerted`]. **For the train 822** ([`super::kerwan_train`], 2026-10-01): `0x2c6c60` ([`waiting`]:
//! a 574 in 0xe) and `0x2c6c90` ([`send_away`]: state 0xe); its `cmd` 4 (the ride) is the turn check's gate. The 573 the
//! trooper waits by (+0x284) is [`super::kerwan_hound`] (its `0x2c6ca0` reads the trooper's seq 10 / states 10, 13 / +0x284).
//!
//! **The rocket 833** `0x2d4288` / `0x2d43c8`: [`spawn_rocket`] (`CreateMoby(833)`: update distance 0xff, draw 0x7e,
//! state 1, drawn; at joint 0, velocity v, yaw atan(v), life `ticks(ticks(130))` (the caller's ticks scaled again),
//! +0x18 0.75, heading, speed, shooter; a line from the shooter's xy to the muzzle (flags 2) that hits → at the hit, life
//! and speed 0; `MobyBuildMatrix`) and [`rocket_update`]: a target (range 2·life·speed); pos += v + Ratchet's platform
//! motion (0x13f490, z ×0.35); yaw = heading, roll += 2π·dt; two type-4 puffs half a speed back
//! (0x6f00afff / 0xff, `ticks(rand_range(15, 22))`, growth `rand_range(20, 35)`, additive; 0x1fffffff / 0x4f4f4f,
//! `ticks(rand_range(30, 60))`, `rand_range(50, 75)`); a target moby: the pitch springs toward its aim point
//! (`SpringTurn` 0x246c80: π/4·dt², π·dt², π/6·dt) and z += sin(−pitch)·speed; `CollLine_Fix(pos, old, 0, shooter)` or
//! `coll_sphere(0.2, pos, 0, shooter)`: a moby hit gets the hit (flags 0x10001, damage 1, type 1 / 1, the exact
//! push (cos, sin, 1), `0x2484f0`), state 2; life out → 2; 2: `0x24dad8(0.25, 13, m, pos, 0)` (= `0x273f50`, class
//! sound 0), `DeleteMoby`.
//!
//! **The game's, noted:** the knocked flight's long-move test compares the radius word (716, ×1024) with the move's
//! length: never true; `0x24fba8` writes its Euler angles to a copy; the rocket's life is `ticks` twice (equal on NTSC);
//! a trooper on its fourth wall slot reads its 573 link only when set. **Not the game's, noted [L]:** a missed ground
//! probe leaves the collision output stale in the game (the port: no moby, z 0); state 8 reads the segment output of a
//! failed nearest-point search uninitialised in the game (the port: the node is kept).

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::creature::{self as c, damage, flash, fx, knock, target, turn, walker};
use crate::moby_update::services::{pf as to_pf, pv as to_pv, HitTemplate, World};
use crate::ps2v::Pf;

/// The update in the level03 class table.
pub const UPDATE_FN: u32 = 0x2c_6fd0;
/// The rocket's update.
pub const ROCKET_FN: u32 = 0x2d_43c8;
pub const REFERENCE_LEVEL: u32 = 3;
pub const CLASSES: [i16; 1] = [574];
pub const ROCKET_CLASSES: [i16; 1] = [ROCKET];
/// The rocket (0x341).
pub const ROCKET: i16 = 833;
/// The 573 (0x23d), the 821 / 1210 platforms (0x335 / 0x4ba) and the 822 (0x336) it knows.
pub const POD: i16 = 573;
pub const PLATFORMS: [i16; 2] = [821, 1210];
pub const CONTROLLER: i16 = 822;
/// The pieces it breaks into.
pub const PIECES: [i16; 3] = [0x6d0, 0x6d1, 0x6d2];
/// Joint lists read: 0 the gun, 2 / 3 the look-at records.
pub const JOINTS: [i16; 1] = [574];
/// Level03 gp−0x530c: the troopers' tick-slot counter (a level word).
pub const SLOT_COUNTER: u32 = 0x16_18f4;

/// Pvar offsets (module doc).
pub mod pv {
    pub const D: usize = 0x20;
    pub const METER: usize = 0x24;
    pub const LURE: usize = 0x38;
    pub const MOVE: usize = 0x40;
    pub const F: usize = 0x60;
    pub const K: usize = 0x70;
    pub const J: usize = 0xd0;
    pub const VZ: usize = 0xe8;
    pub const NODE: usize = 0x120;
    pub const STEP: usize = 0x124;
    pub const PATH: usize = 0x130;
    pub const PATH_ID: usize = 0x134;
    pub const HEAD: usize = 0x150;
    pub const TORSO: usize = 0x1d0;
    pub const COOLDOWN: usize = 0x250;
    pub const ALERT: usize = 0x252;
    pub const SHOTS: usize = 0x254;
    pub const FLIGHT_T: usize = 0x255;
    pub const JUMP: usize = 0x256;
    pub const TURN_V: usize = 0x258;
    pub const HIDDEN: usize = 0x262;
    pub const PAUSE: usize = 0x263;
    pub const LINK: usize = 0x264;
    pub const SIGHT_BASE: usize = 0x26c;
    pub const SIGHT: usize = 0x270;
    pub const PATROL: usize = 0x274;
    pub const LEASH: usize = 0x278;
    pub const JUMP_RANGE: usize = 0x27c;
    pub const RIDE: usize = 0x280;
    pub const POD: usize = 0x284;
    pub const DRAW: usize = 0x288;
    pub const SLOT: usize = 0x28a;
    pub const TURN_TO: usize = 0x28c;
    pub const JPATH: usize = 0x290;
    pub const JPATH_RANGE: usize = 0x294;
    pub const JNODE: usize = 0x298;
    pub const CUBOID: usize = 0x29c;
    pub const SIZE: usize = 0x2a0;
}

/// The states (module doc).
pub mod st {
    pub const INIT: u8 = 0;
    pub const STAND: u8 = 1;
    pub const POD: u8 = 2;
    pub const AIM: u8 = 4;
    pub const FIRE: u8 = 5;
    pub const PUNCH: u8 = 6;
    pub const PATROL: u8 = 7;
    pub const DOWN: u8 = 8;
    pub const JUMP: u8 = 9;
    pub const KNOCKED: u8 = 10;
    pub const FALL: u8 = 0xb;
    pub const TURN: u8 = 0xc;
    pub const DYING: u8 = 0xd;
    pub const AWAY: u8 = 0xe;
    pub const BACK: u8 = 0xf;
}

/// The level03 `$gp` words and literals (module doc).
pub mod k {
    pub const FLIGHT_UP: f32 = 1.0;
    pub const FLIGHT_OUT: f32 = 2.5;
    pub const FLIGHT_DRAG: f32 = 100.0;
    pub const PLATFORM_CLIMB: f32 = 0.35;
    pub const PUNCH_RANGE: f32 = 3.5;
    pub const QUARTER: f32 = std::f32::consts::FRAC_PI_4;
}

fn state(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_state(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn seq_b(w: &World, id: MobyId) -> u8 { w.m(id).anim.seq_b }
fn done(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
/// `if (m+0x53 != seq) fun_00212f90(m, seq, 0, ticks(n))`.
fn blend(w: &mut World, id: MobyId, seq: u8, n: i32) {
    let t = w.ticks(n);
    c::blend_to(w, id, seq, 0, t);
}
/// `fun_00212f90(m, seq, 0, ticks(n))` without the guard.
fn blend_now(w: &mut World, id: MobyId, seq: u8, n: i32) {
    let t = w.ticks(n);
    w.anim_blend(id, seq, 0, t);
}
fn mission_done(w: &World, id: MobyId) -> bool { w.mission_done(w.svc.level, w.m(id).mission) == 0xff }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o)).ok().filter(|&m| m < w.table.mobys.len()) }
fn alive(w: &World, m: MobyId) -> bool { let s = w.m(m).state; s != 0xfe && s != 0xfd }
fn spline(w: &World, o: i32) -> Option<usize> { usize::try_from(o).ok().filter(|&p| p < w.svc.splines.len()) }
fn path(w: &World, id: MobyId) -> Option<usize> { spline(w, c::pi32(w, id, pv::PATH)) }
fn count(w: &World, p: Option<usize>) -> i32 { p.map_or(0, |p| w.svc.splines[p].len() as i32) }
fn point(w: &World, p: Option<usize>, i: i32) -> c::V {
    p.zip(usize::try_from(i).ok()).and_then(|(p, i)| w.svc.splines[p].get(i)).map(|q| q.map(f32::from_bits)).unwrap_or([0.0; 4])
}
fn points(w: &World, p: usize) -> Vec<[f32; 4]> { w.svc.splines[p].iter().map(|q| q.map(f32::from_bits)).collect() }
/// `0x24c9b0(1000, 5, 0, spline, pos, &out, &seg, &t, 0)` (= `SplineSample` 0x272e28): the nearest point and segment.
fn nearest(w: &World, p: Option<usize>, at: c::V) -> Option<([f32; 3], i32)> {
    let p = p?;
    crate::spline::nearest(&points(w, p), false, 1000.0, 5.0, 0.0, [at[0], at[1], at[2]]).map(|(q, cur)| (q, cur.seg))
}
fn dz_abs(t: &target::Target, p: c::V) -> f32 { (t.pos[2] - p[2]).abs() }
fn heading_to(p: c::V, t: c::V) -> f32 { c::atan(t[0] - p[0], t[1] - p[1]) }

/// `0x2c6c60(m)`: a 574 waiting hidden (state 0xe) (822's test).
pub fn waiting(w: &World, m: MobyId) -> bool { w.m(m).o_class == CLASSES[0] && w.m(m).state == st::AWAY }
/// `0x2c6c90(m)`: state 0xe (822 sends the trooper away).
pub fn send_away(w: &mut World, m: MobyId) { set_state(w, m, st::AWAY); }

/// `0x2c6d98(group)`: a live member is a 574 jumping down (state 9, `0x2c6d58`) or a 573 in state 5 (`0x2c5b78`).
pub fn group_alerted(w: &World, group: i8) -> bool {
    if group as u8 == 0xff { return false; }
    crate::moby_update::scheduler::group_ids(w, group).into_iter().filter(|&m| m < w.table.mobys.len() && w.m(m).state < 0x80).any(|m| {
        let o = w.m(m);
        (o.o_class == CLASSES[0] && o.state == st::JUMP) || (o.o_class == POD && o.state == 5)
    })
}

/// `0x2c6e18(h, m, rec)`: a target behind it turns it (module doc). True when it turned (state 0xc).
fn turn_check(w: &mut World, id: MobyId, h: f32, t: &target::Target) -> bool {
    if t.moby.is_none() { return false; }
    if let Some(l) = link(w, id, pv::LINK) {
        if w.m(l).o_class == CONTROLLER && w.m(l).cmd != 4 { return false; }
    }
    let a = c::sub_rot(h, c::yaw(w, id)).abs();
    if (a < k::QUARTER || c::pu8(w, id, pv::PAUSE) != 0 || seq_b(w, id) == 1) && a <= std::f32::consts::FRAC_PI_2 { return false; }
    if c::dist2(c::pos(w, id), t.pos) >= 20.0 { return false; }
    if 1.178_097_2 < a {
        blend_now(w, id, 1, 10);
        w.mm(id).anim.speed = 1.0;
    } else {
        blend_now(w, id, 9, 10);
    }
    c::set_pf(w, id, pv::TURN_TO, h);
    let s = state(w, id);
    w.mm(id).cmd = s;
    set_state(w, id, st::TURN);
    true
}

/// `0x2c9ca0(m)`: the next patrol node (module doc), into +0x120.
fn pick_node(w: &mut World, id: MobyId) {
    let range = c::pf(w, id, pv::SIGHT);
    let t = target::acquire(w, id, range);
    let p = path(w, id);
    let me = c::pos(w, id);
    let seg = nearest(w, p, me).map_or(c::pi32(w, id, pv::NODE), |(_, s)| s);
    c::set_pi32(w, id, pv::NODE, 0);
    let (mut best, mut bd, mut bh) = (0i32, 0.0f32, 0.0f32);
    let step = c::pu8(w, id, pv::STEP) as i8 as i32;
    let n = count(w, p);
    if n == 0 { return; }
    let to_t = heading_to(me, t.pos);
    for _ in 0..=n.max(1) * 2 {
        let i = c::pi32(w, id, pv::NODE);
        if i != seg {
            let q = point(w, p, i);
            let dh = c::diff_rots(heading_to(me, q), to_t);
            let dd = c::dist3(t.pos, q);
            let take = if dh < std::f32::consts::FRAC_PI_2 {
                bh < dh
            } else if 16.0 <= dd && (bd > dd || bd < 16.0) {
                true
            } else {
                dd < 16.0 && bd < dd
            };
            if take { (best, bd, bh) = (i, dd, dh); }
        }
        let next = (i + n + step).rem_euclid(n);
        c::set_pi32(w, id, pv::NODE, next);
        if next == 0 {
            c::set_pi32(w, id, pv::NODE, best);
            return;
        }
    }
    // A step of 0 never wraps in the game (an endless loop); the port stops after two rounds [L].
    c::set_pi32(w, id, pv::NODE, best);
}

/// The next node after a done attack or a walk (`patrol == 0`: [`pick_node`]; else +1 while not at the end). False:
/// at the path's end (the caller goes to 1).
fn next_node(w: &mut World, id: MobyId) -> bool {
    if c::pi32(w, id, pv::PATROL) == 0 {
        pick_node(w, id);
        return true;
    }
    let n = c::pi32(w, id, pv::NODE);
    if n < count(w, path(w, id)) - 1 {
        c::set_pi32(w, id, pv::NODE, n + 1);
        return true;
    }
    false
}

/// To 7 with seq 1 (`joined_r0x002c8ce4` / `LAB_002c8cec`).
fn to_patrol(w: &mut World, id: MobyId) {
    set_state(w, id, st::PATROL);
    blend(w, id, 1, 10);
}

/// To 1 with seq 0 (guarded).
fn to_stand(w: &mut World, id: MobyId) {
    set_state(w, id, st::STAND);
    blend(w, id, 0, 10);
}

/// The position at the path's first point (the respawn).
fn to_start(w: &mut World, id: MobyId) {
    let p = point(w, path(w, id), 0);
    c::set_pos(w, id, p);
}

/// `LAB_002c9550`: state 0xe, no collision (+0x94 = 0), mode |= 0x41.
fn hide(w: &mut World, id: MobyId) {
    set_state(w, id, st::AWAY);
    let m = w.mm(id);
    m.has_collision = false;
    m.mode |= mode::HIDDEN | mode::NO_ANIM;
}

/// The respawn of a linked trooper or the delete (module doc, after the explosion). False: deleted.
fn respawn_or_delete(w: &mut World, id: MobyId) -> bool {
    if c::pi32(w, id, pv::LINK) == -1 {
        w.delete_moby(id);
        return false;
    }
    to_start(w, id);
    c::set_pv4(w, id, pv::MOVE, [0.0; 4]);
    c::set_pf(w, id, pv::VZ, 0.0);
    hide(w, id);
    true
}

/// The blast of `SpawnBeamExplosion(0, 0, 2, 1, 9, 1, 15, m, +0x40, pos + 1 z, 5, 2, 4, 6, 1, 1, −1, 0)`.
pub const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 5, sparks: 2, puffs: 4, debris: 1, sound: 6, shake: true };

/// The explosion, the pieces, the death bits, then the respawn or the delete (module doc). False: deleted.
fn explode(w: &mut World, id: MobyId, bits: u32) -> bool {
    let p = c::pos(w, id);
    fx::beam_explosion(w, &BLAST, Some(id), [p[0], p[1], p[2] + 1.0, p[3]]);
    let rot = w.m(id).rotation;
    for class in PIECES { fx::break_piece(w, id, class, p, rot, 0, 0); }
    set_death_bits(w, id, bits, -1);
    respawn_or_delete(w, id)
}

/// The head / torso look-at (module doc).
fn look(w: &mut World, id: MobyId, d: f32, t: &target::Target) {
    let head_t = pv::HEAD + crate::moby_update::manip::rec::TARGET + 8;
    let torso_t = pv::TORSO + crate::moby_update::manip::rec::TARGET + 8;
    let s = state(w, id);
    if s == st::TURN {
        let a = c::sub_rot(c::pf(w, id, pv::TURN_TO), c::yaw(w, id));
        c::set_pf(w, id, head_t, a * 0.25);
        c::set_pf(w, id, torso_t, a * 0.75);
    } else if s < 7 && d < c::pf(w, id, pv::SIGHT) {
        let a = c::sub_rot(heading_to(c::pos(w, id), t.pos), c::yaw(w, id));
        c::set_pf(w, id, torso_t, a);
    }
    for o in [head_t, torso_t] {
        let a = c::pf(w, id, o);
        if k::QUARTER < a { c::set_pf(w, id, o, k::QUARTER); } else if a < -k::QUARTER { c::set_pf(w, id, o, -k::QUARTER); }
    }
    crate::moby_update::manip::look(w, id, id, pv::HEAD, 2, 0.03, 0.3);
    crate::moby_update::manip::look(w, id, id, pv::TORSO, 3, 0.03, 0.3);
}

/// `0x2c95a8`: the hits (module doc).
fn hits(w: &mut World, id: MobyId) {
    if c::pi32(w, id, pv::D + 0x18) != 0 {
        c::set_pi32(w, id, pv::D + 0x18, 0);
        let t = w.ticks(240);
        c::set_pi16(w, id, pv::ALERT, t as i16);
    }
    let base = c::pf(w, id, pv::SIGHT_BASE);
    let r = if c::dec_timer_pvar_s16(w, id, pv::ALERT) == 0 { base + 5.0 } else { base };
    c::set_pf(w, id, pv::SIGHT, r);
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    if let Some(h) = res.hit.filter(|_| res.out5 != 1 && state(w, id) != st::DYING) {
        let hp = c::pf(w, id, pv::D) - res.damage;
        c::set_pf(w, id, pv::D, hp);
        let reaction = if hp <= 0.0 { 1 } else { res.reaction };
        let dir = h.dir.map(|x| f32::from_bits(x.0));
        let kr = pv::K;
        // `sqrt(2·g)` (`fun_001f9988`) and the flight's speed: `out / (2·s/g) + drag·½·(2·s/g)`.
        let flight = |w: &mut World, g: f32, up_mul: f32, out: f32, half: bool| {
            c::set_pf(w, id, kr + knock::k::GRAVITY, g);
            let s = (up_mul * g).abs().sqrt();
            let t = (s + s) / g;
            let mut sp = out / t + k::FLIGHT_DRAG * c::DT2 * 0.5 * t;
            if half { sp *= 0.5; }
            c::set_pf(w, id, kr + knock::k::SPEED, sp);
            c::set_pf(w, id, kr + knock::k::UP, s);
            c::set_pi32(w, id, kr + knock::k::FLAGS, 9);
            c::set_pu8(w, id, kr + 0x3d, 0);
            let (mut sp, mut up) = (sp, s);
            let a = knock::aim(dir, &mut sp, &mut up);
            c::set_pf(w, id, kr + knock::k::SPEED, sp);
            c::set_pf(w, id, kr + knock::k::UP, up);
            a
        };
        let knock_tail = |w: &mut World, apex: f32, land: f32| {
            c::set_pf(w, id, kr + knock::k::KEY_APEX, apex);
            c::set_pf(w, id, kr + knock::k::KEY_LAND, land);
            set_state(w, id, st::KNOCKED);
            w.play_sound(1, 0, id);
        };
        let up2 = k::FLIGHT_UP + k::FLIGHT_UP;
        match reaction {
            1 => {
                c::set_pf(w, id, kr + knock::k::DRAG, k::FLIGHT_DRAG * c::DT2);
                let a = flight(w, c::DT2 * 40.0, 4.0, 3.0, false);
                knock::start(w, id, kr, a, 7, 5, 2);
                c::set_pf(w, id, kr + knock::k::KEY_APEX, 10.0);
                c::set_pf(w, id, kr + knock::k::KEY_LAND, 20.0);
                c::set_pf(w, id, kr + knock::k::AIR_SPEED, c::DT + c::DT);
                c::set_pu8(w, id, pv::FLIGHT_T, 0);
                w.mm(id).mode &= !mode::TARGETABLE;
                c::set_pu8(w, id, pv::F + 7, 0xfa);
                set_state(w, id, st::DYING);
                w.play_sound(3, 0, id);
                if seq_b(w, id) != 7 {
                    let t = w.ticks(6);
                    w.anim_blend(id, 7, 0, t);
                }
            }
            3 | 7 | 8 => {
                c::set_pu8(w, id, pv::F + 7, 0x78);
                c::set_pf(w, id, kr + knock::k::DRAG, k::FLIGHT_DRAG * c::DT2);
                let a = flight(w, c::DT2 * 80.0, up2, k::FLIGHT_OUT, false);
                knock::start(w, id, kr, a, 5, 5, 0);
                knock_tail(w, 6.0, 13.0);
            }
            4 | 5 => {
                c::set_pu8(w, id, pv::F + 7, 0x78);
                c::set_pf(w, id, kr + knock::k::DRAG, k::FLIGHT_DRAG * c::DT2);
                let a = flight(w, c::DT2 * 80.0, up2, k::FLIGHT_OUT, false);
                knock::start(w, id, kr, a, 6, 5, 2);
                knock_tail(w, 5.0, 10.0);
            }
            6 => {
                c::set_pu8(w, id, pv::F + 7, 0x78);
                // The drag is not written in this case (the record keeps the last one).
                let a = flight(w, c::DT2 * 80.0, up2, k::FLIGHT_OUT, true);
                if c::diff_rots(c::yaw(w, id), a) < std::f32::consts::FRAC_PI_3 { c::set_yaw(w, id, c::add_rot(a, std::f32::consts::PI)); }
                knock::start(w, id, kr, a, 5, 5, 0);
                c::hard_cut(w, id, 5, 5);
                knock_tail(w, 14.0, 28.0);
            }
            0xc | 0xd => c::set_pu8(w, id, pv::F + 7, 0x78),
            _ => {}
        }
        flash::start(w, id, pv::F);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pv::F);
}

/// State 0 (module doc).
fn init(w: &mut World, id: MobyId) {
    w.mm(id).b7f = 0x1c;
    crate::moby_update::classes::units::take_hero_light(w, id);
    walker::seed(&mut w.mm(id).pvars, pv::J);
    let j = pv::J;
    c::set_pf(w, id, j + 0xc, 3.0);
    c::set_pf(w, id, j + 0x18, 0.0);
    c::set_pf(w, id, j + 0x44, c::DT * std::f32::consts::PI);
    c::set_pf(w, id, j + 0x24, c::DT * 6.0);
    c::set_pf(w, id, j + 8, 2.0);
    c::set_pf(w, id, j + 0x28, std::f32::consts::PI);
    c::set_pf(w, id, j + 0x40, c::DT2 * 12.566_371);
    c::set_pf(w, id, j + 0x3c, c::DT2 * 6.283_185_5);
    let g15 = w.svc.timing.scale(Pf::f(15.0)).to_f32();
    c::set_pf(w, id, pv::D, 3.0);
    c::set_pi32(w, id, j + 0x38, 9);
    c::set_pf(w, id, j + 0x2c, c::pf(w, id, j + 0x24) * g15);
    c::set_pu8(w, id, pv::D + 9, 1);
    c::set_pu8(w, id, pv::D + 8, 2);
    c::set_pf(w, id, pv::D + 0x10, 1.75);
    c::set_pf(w, id, pv::K + knock::k::ZOFF, 0.7);
    c::set_pi16(w, id, pv::METER, 3);
    c::set_pf(w, id, pv::K + knock::k::AIR_SPEED, 0.0);
    c::set_pi32(w, id, pv::K + knock::k::RADIUS, 716);
    c::set_pu8(w, id, 0x58, 10);
    c::set_pu8(w, id, 0x5a, 0xc);
    c::set_pv4(w, id, pv::MOVE, [0.0; 4]);
    w.mm(id).mode |= mode::TARGETABLE;
    c::set_pi16(w, id, pv::COOLDOWN, 0);
    let pid = c::pi32(w, id, pv::PATH_ID);
    c::set_pi32(w, id, pv::NODE, 0);
    c::set_pi32(w, id, pv::PATH, pid);
    to_start(w, id);
    set_state(w, id, st::STAND);
    let n = w.svc.units.word(SLOT_COUNTER);
    let dd = w.m(id).draw_dist;
    c::set_pi16(w, id, pv::DRAW, dd);
    w.svc.units.set_word(SLOT_COUNTER, n.wrapping_add(1));
    c::set_pi16(w, id, pv::SLOT, (n & 7) as i16);
    c::set_pu8(w, id, pv::PAUSE, 0);
    if let Some(p) = spline(w, pid) {
        let len = w.svc.splines[p].len();
        for i in 0..len.saturating_sub(2) {
            let (a, b) = (point(w, Some(p), i as i32), point(w, Some(p), i as i32 + 1));
            w.svc.splines[p][i][3] = c::dist3(a, b).to_bits();
        }
    }
    if c::pi32(w, id, pv::POD) != -1 && c::pi32(w, id, pv::JPATH) == -1 {
        blend_now(w, id, 0, 10);
        set_state(w, id, st::POD);
    }
}

/// State 1 (module doc).
fn stand(w: &mut World, id: MobyId, d: f32, t: &target::Target) {
    if done(w, id) && w.m(id).anim.seq_a != 0 { blend_now(w, id, 0, 10); }
    if let Some(jp) = spline(w, c::pi32(w, id, pv::JPATH)) {
        let (a, b) = (point(w, Some(jp), 0), point(w, Some(jp), 1));
        c::set_pos(w, id, a);
        c::set_yaw(w, id, heading_to(a, b));
        let go = group_alerted(w, w.m(id).group)
            || w.in_cuboid([t.pos[0], t.pos[1], t.pos[2]], c::pi32(w, id, pv::CUBOID))
            || mission_done(w, id)
            || (d < c::pf(w, id, pv::JPATH_RANGE) && dz_abs(t, c::pos(w, id)) < 4.0);
        if !go { return; }
        c::set_pi32(w, id, pv::JNODE, 0);
        set_state(w, id, st::JUMP);
        blend(w, id, 1, 10);
        return;
    }
    if c::pi32(w, id, pv::POD) != -1 {
        blend_now(w, id, 0, 10);
        set_state(w, id, st::POD);
        return;
    }
    let p = c::pos(w, id);
    let dz = dz_abs(t, p);
    if d < c::pf(w, id, pv::SIGHT) || d < c::pf(w, id, pv::JUMP_RANGE) {
        if turn_check(w, id, heading_to(p, t.pos), t) { return; }
        if d < c::pf(w, id, pv::JUMP_RANGE) && dz < 4.0 {
            c::set_pf(w, id, pv::JUMP_RANGE, 0.0);
            c::set_pi16(w, id, pv::JUMP, 1);
            set_state(w, id, st::DOWN);
            blend(w, id, 1, 10);
        }
    }
    if d < k::PUNCH_RANGE && dz < 3.0 {
        set_state(w, id, st::PUNCH);
        blend(w, id, 2, 10);
        return;
    }
    if c::pf(w, id, pv::SIGHT) <= d || 10.0 <= dz { return; }
    if c::dec_timer_pvar_s16(w, id, pv::COOLDOWN) == 0 { return; }
    if k::QUARTER <= c::diff_rots(c::yaw(w, id), heading_to(p, t.pos)) { return; }
    if w.m(id).visible != 0 {
        set_state(w, id, st::AIM);
        blend(w, id, 3, 10);
    }
}

/// Within the leash and 10 in height: the next node and 7 (the shared tail of states 4 / 5).
fn leash_walk(w: &mut World, id: MobyId, d: f32, dz: f32) {
    if c::pf(w, id, pv::LEASH) <= d || 10.0 <= dz { return; }
    if c::pi32(w, id, pv::PATROL) == 0 {
        pick_node(w, id);
        to_patrol(w, id);
        return;
    }
    let n = c::pi32(w, id, pv::NODE);
    if count(w, path(w, id)) - 1 <= n { return; }
    c::set_pi32(w, id, pv::NODE, n + 1);
    to_patrol(w, id);
}

/// State 4 (module doc).
fn aim(w: &mut World, id: MobyId, d: f32, t: &target::Target) {
    if done(w, id) {
        set_state(w, id, st::FIRE);
        c::set_pu8(w, id, pv::SHOTS, 0);
        if seq_b(w, id) != 4 { w.anim_blend(id, 4, 0, 0); }
        return;
    }
    let dz = dz_abs(t, c::pos(w, id));
    if d < k::PUNCH_RANGE && dz < 3.0 {
        set_state(w, id, st::PUNCH);
        blend(w, id, 2, 10);
        return;
    }
    leash_walk(w, id, d, dz);
}

/// The firing key of state 5 (module doc).
fn shoot(w: &mut World, id: MobyId, t: &target::Target) {
    let n = c::pu8(w, id, pv::SHOTS).wrapping_add(1);
    c::set_pu8(w, id, pv::SHOTS, n);
    let jp = w.joint_point(id, 0);
    let mut v = c::set_len3(c::sub(t.aim, jp), c::DT * 6.0);
    v[2] *= 0.5;
    let mut h = c::atan(v[0], v[1]);
    let dh = c::sub_rot(h, c::yaw(w, id));
    let lim = std::f32::consts::FRAC_PI_6;
    if lim < dh || dh < -lim {
        h = c::add_rot(c::yaw(w, id), if lim < dh { lim } else { -lim });
        let (cs, sn) = c::cs(h);
        v = [cs * c::DT * 6.0, sn * c::DT * 6.0, 0.0, v[3]];
    }
    let s = w.svc.timing.scale(Pf::f(10.0)).to_f32();
    let muzzle = c::add(c::scale(v, s), jp);
    fx::muzzle_smoke(w, id, muzzle, None);
    let life = w.ticks(130);
    spawn_rocket(w, h, c::DT * 6.0, 0.75, v, jp, id, life);
}

/// State 5 (module doc).
fn fire(w: &mut World, id: MobyId, d: f32, t: &target::Target) {
    if crate::moby_update::creature::ground::passed_frame(w, id, 8.0) { shoot(w, id, t); }
    let p = c::pos(w, id);
    let dz = dz_abs(t, p);
    if !done(w, id) {
        if !(d < k::PUNCH_RANGE && dz < 3.0) {
            leash_walk(w, id, d, dz);
        } else if !turn_check(w, id, heading_to(p, t.pos), t) {
            set_state(w, id, st::PUNCH);
            blend(w, id, 2, 10);
        }
        return;
    }
    if c::pu8(w, id, pv::SHOTS) == 0 { return; }
    if d < c::pf(w, id, pv::JUMP_RANGE) && dz < 4.0 {
        c::set_pf(w, id, pv::JUMP_RANGE, 0.0);
        set_state(w, id, st::DOWN);
        blend(w, id, 1, 10);
    }
    if turn_check(w, id, heading_to(p, t.pos), t) { return; }
    let keep = d < c::pf(w, id, pv::SIGHT)
        && dz < 10.0
        && c::dec_timer_pvar_s16(w, id, pv::COOLDOWN) != 0
        && c::diff_rots(c::yaw(w, id), heading_to(p, t.pos)) < 0.087_266_46
        && w.m(id).visible != 0;
    if keep { return; }
    if d < k::PUNCH_RANGE && dz < 3.0 {
        set_state(w, id, st::PUNCH);
        blend(w, id, 2, 10);
    }
}

/// State 6 (module doc).
fn punch(w: &mut World, id: MobyId, t: &target::Target) {
    let key = crate::moby_update::creature::ground::key_time(w, id);
    let p = c::pos(w, id);
    turn::spring_turn2_pvar(w, id, heading_to(p, t.pos), 0.03, 0.3, 0.3, pv::TURN_V);
    // `0x23e338(m, 0, &out)`: joint 0's point, not read afterwards.
    if (25.0..=31.0).contains(&key) {
        let p = c::pos(w, id);
        if (p[2] - t.pos[2]).abs() < 2.0 && c::dist2(p, t.pos) < k::PUNCH_RANGE && c::diff_rots(heading_to(p, t.pos), c::yaw(w, id)) < 0.261_799_4 {
            if let Some(m) = t.moby {
                let (cs, sn) = c::cs(c::yaw(w, id));
                let d = c::set_len2([cs * 0.2, sn * 0.2, 0.0, 0.0], 1.0);
                let tmpl = HitTemplate { dir: [to_pf(d[0]), to_pf(d[1]), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x1_0001, b18: 0, b19: 1, h1a: w.m(id).o_class as u16, damage: Pf::ONE, w20: 0 };
                w.deliver_hit(m, &tmpl);
            }
            return;
        }
    }
    if !done(w, id) { return; }
    if c::pi32(w, id, pv::PATROL) != 0 {
        let n = c::pi32(w, id, pv::NODE);
        if count(w, path(w, id)) - 1 <= n {
            to_stand(w, id);
            let p = c::pos(w, id);
            turn_check(w, id, heading_to(p, t.pos), t);
            return;
        }
        c::set_pi32(w, id, pv::NODE, n + 1);
        to_patrol(w, id);
        return;
    }
    pick_node(w, id);
    to_patrol(w, id);
}

/// State 7 (module doc).
fn patrol(w: &mut World, id: MobyId, d: f32, t: &target::Target) {
    if c::pi16(w, id, pv::JUMP) != 0 {
        set_state(w, id, st::DOWN);
        return;
    }
    let q = point(w, path(w, id), c::pi32(w, id, pv::NODE));
    if turn_check(w, id, heading_to(c::pos(w, id), q), t) { return; }
    let mut out = [0.0; 4];
    let r = walker::walk_to(w, id, pv::J, q, &mut out);
    c::set_pv4(w, id, pv::MOVE, out);
    if r & 0x14 == 0 { return; }
    let dz = dz_abs(t, c::pos(w, id));
    if c::pf(w, id, pv::LEASH) <= d || 10.0 <= dz || !next_node(w, id) {
        to_stand(w, id);
        let p = c::pos(w, id);
        turn_check(w, id, heading_to(p, t.pos), t);
        return;
    }
    to_patrol(w, id);
}

/// States 8 / 9: walk the path (+0x130 / node +0x120, or the jump path +0x290 / +0x298) from its nearest segment on.
/// True at its end.
fn follow(w: &mut World, id: MobyId, p: Option<usize>, node: usize) -> bool {
    let me = c::pos(w, id);
    if let Some((_, seg)) = nearest(w, p, me) {
        if c::pi32(w, id, node) < seg { c::set_pi32(w, id, node, seg); }
    }
    let n = c::pi32(w, id, node);
    let q = point(w, p, n);
    let mut out = [0.0; 4];
    let r = walker::walk_to(w, id, pv::J, q, &mut out);
    c::set_pv4(w, id, pv::MOVE, out);
    if r & 0x14 == 0 { return false; }
    if count(w, p) - 1 <= n { return true; }
    c::set_pi32(w, id, node, n + 1);
    false
}

/// State 10 (module doc). False: deleted (or hidden for the respawn: the update ends).
fn knocked(w: &mut World, id: MobyId, t: &target::Target) -> bool {
    let r = if c::pi16(w, id, pv::K + knock::k::PHASE) == 2 {
        w.mm(id).anim.speed = 1.0;
        (w.m(id).anim.flags as u32 & 2) << 5
    } else {
        knock::update(w, id, pv::K)
    };
    c::set_pi32(w, id, pv::JPATH, -1);
    if r & 0x40 == 0 {
        if c::pos(w, id)[2] < 5.0 || r & 0x20 != 0 {
            explode(w, id, 0);
            return false;
        }
    } else if next_node(w, id) {
        to_patrol(w, id);
    } else {
        to_stand(w, id);
        let p = c::pos(w, id);
        turn_check(w, id, heading_to(p, t.pos), t);
    }
    let mut p = c::pos(w, id);
    p[2] += 2.0;
    c::set_pos(w, id, p);
    let g = c::ground::ground(w, p, 0.5, 0x20).z;
    p[2] -= 2.0;
    c::set_pos(w, id, p);
    if g + 5.0 < p[2] {
        w.play_sound(10, 0x21, id);
        blend_now(w, id, 0xb, 10);
        set_state(w, id, st::FALL);
    }
    true
}

/// State 0xb (module doc). False: deleted / hidden.
fn fall(w: &mut World, id: MobyId) -> bool {
    let kr = pv::K;
    let g = c::pf(w, id, kr + knock::k::GRAVITY);
    let mut vel = c::pv4(w, id, kr);
    vel[2] -= g;
    let mut l = c::len2(vel);
    let air = if c::pi16(w, id, kr + knock::k::PHASE) != 2 { c::pf(w, id, kr + knock::k::AIR_SPEED) } else { 0.0 };
    turn::approach(air, c::pf(w, id, kr + knock::k::DRAG), &mut l);
    vel = c::set_len2(vel, l);
    c::set_pv4(w, id, kr, vel);
    let zoff = c::pf(w, id, kr + knock::k::ZOFF);
    let p0 = c::pos(w, id);
    let start = [p0[0], p0[1], p0[2] + zoff, p0[3]];
    let p1 = c::add(p0, vel);
    c::set_pos(w, id, p1);
    let mut end = [p1[0], p1[1], p1[2] + zoff, p1[3]];
    let radius = c::pi32(w, id, kr + knock::k::RADIUS);
    if (radius as f32) < c::len3(vel) {
        if let Some(o) = w.coll_line(to_pv(start), to_pv(end), 0x24, Some(id)) { end = [o.point[0], o.point[1], o.point[2], end[3]]; }
    }
    if let Some(o) = w.coll_sphere(to_pv(end), to_pf(radius as f32 * 0.000_976_562_5), 0x24, Some(id)) {
        if let Some(cc) = o.pushed_centre {
            let z = (start[2] + vel[2]) - zoff;
            c::set_pos(w, id, [cc[0], cc[1], z, p1[3]]);
        }
    }
    if c::pos(w, id)[2] < 5.0 {
        explode(w, id, 0x200);
        return false;
    }
    true
}

/// State 0xc (module doc).
fn turn(w: &mut World, id: MobyId) {
    let to = c::pf(w, id, pv::TURN_TO);
    let sb = seq_b(w, id);
    let (acc, damp, max) = match sb {
        1 => (0.09, 0.27, 0.15),
        9 => (0.01, 0.3, 0.2),
        _ => return,
    };
    turn::spring_turn2_pvar(w, id, to, acc, damp, max, pv::TURN_V);
    if 0.034_906_585 <= c::diff_rots(c::yaw(w, id), to) { return; }
    let t60 = w.ticks(60);
    c::set_pu8(w, id, pv::PAUSE, t60 as u8);
    let back = w.m(id).cmd;
    if back.wrapping_sub(7) < 2 {
        if back == 7 { pick_node(w, id); }
        set_state(w, id, back);
        blend(w, id, 1, 10);
        if sb == 1 { w.mm(id).anim.speed = 1.0; }
    } else {
        to_stand(w, id);
        let t = w.ticks(10);
        c::set_pi16(w, id, pv::COOLDOWN, t as i16);
    }
}

/// State 0xd (module doc). False: deleted / hidden.
fn dying(w: &mut World, id: MobyId) -> bool {
    let r = knock::update(w, id, pv::K);
    let n = c::pu8(w, id, pv::FLIGHT_T).wrapping_add(1);
    c::set_pu8(w, id, pv::FLIGHT_T, n);
    let g = c::ground::ground(w, c::pos(w, id), 0.5, 0x20).z;
    if r & 0x60 != 0 && g + 5.0 < c::pos(w, id)[2] && w.m(id).cmd != 0xb {
        w.play_sound(10, 0, id);
        w.mm(id).cmd = 0xb;
    }
    w.mm(id).mode &= !mode::TARGETABLE;
    if r & 1 != 0 { c::set_pu8(w, id, pv::FLIGHT_T, 0); }
    if r & 0x60 == 0 {
        if 5.0 <= c::pos(w, id)[2] { return true; }
        set_death_bits(w, id, 0, -1);
        respawn_or_delete(w, id);
        return false;
    }
    explode(w, id, 0);
    false
}

/// The ground probe of the tail (`GroundHeight(0.5, pos, 0)`, 0x2482d0) with the moby it hit.
fn ground_hit(w: &World, p: c::V) -> (f32, Option<MobyId>) {
    let a = [p[0], p[1], p[2] + 0.5, p[3]];
    let b = [p[0], p[1], 0.01, p[3]];
    match w.coll_line(to_pv(a), to_pv(b), 2, None) {
        Some(o) => (o.point[2], o.moby),
        None => (0.0, None),
    }
}

/// The tail (module doc). False: deleted / hidden.
fn tail(w: &mut World, id: MobyId) -> bool {
    if w.m(id).visible != 0 {
        let cam = w.camera.map(|x| f32::from_bits(x.0));
        if c::dist3(c::pos(w, id), cam) < 36.0 { crate::shadows::probe_down(w, id); }
    }
    if link(w, id, pv::RIDE).is_none_or(|r| !alive(w, r)) { c::set_pi32(w, id, pv::RIDE, -1); }
    if let Some(r) = link(w, id, pv::RIDE) {
        if let Some(car) = crate::moby_update::triggers::carrier(w.m(r)) {
            if PLATFORMS.contains(&w.m(r).o_class) {
                if let Some(p) = path(w, id) {
                    for i in 0..w.svc.splines[p].len() {
                        let q = point(w, Some(p), i as i32);
                        let (n, _) = crate::moby_update::triggers::carried(&car, [q[0], q[1], q[2]], [0.0; 3]);
                        w.svc.splines[p][i] = [n[0], n[1], n[2], q[3]].map(f32::to_bits);
                    }
                }
            }
            let (pos, rot) = (c::pos(w, id), w.m(id).rotation);
            let (n, e) = crate::moby_update::triggers::carried(&car, [pos[0], pos[1], pos[2]], [rot[0], rot[1], rot[2]]);
            c::set_pos(w, id, [n[0], n[1], n[2], pos[3]]);
            c::set_yaw(w, id, e[2]);
        }
    }
    let s = state(w, id);
    if !(st::FALL..=st::AWAY).contains(&s) {
        let vz = if s == st::KNOCKED { 0.0 } else { c::pf(w, id, pv::VZ) + c::DT2 * 9.8 };
        c::set_pf(w, id, pv::VZ, vz);
        let mut p = c::pos(w, id);
        p[2] -= vz - 2.0;
        let (g, hit) = ground_hit(w, p);
        p[2] -= 2.0;
        c::set_pos(w, id, p);
        if p[2] < g {
            match hit.filter(|&m| crate::moby_update::triggers::platform_block(w.m(m)).is_some()) {
                Some(m) => c::set_pi32(w, id, pv::RIDE, m as i32),
                None => {
                    if let Some(r) = link(w, id, pv::RIDE) {
                        if !PLATFORMS.contains(&w.m(r).o_class) { c::set_pi32(w, id, pv::RIDE, -1); }
                    }
                }
            }
            w.mm(id).position[2] = g;
            c::set_pf(w, id, pv::VZ, 0.0);
            // `0x24fba8(yaw, normal, rot)`: its Euler angles go to a stack copy (no effect).
        } else if g + 5.0 < p[2] {
            blend_now(w, id, 0xb, 10);
            set_state(w, id, st::FALL);
            c::set_pv4(w, id, pv::K, [0.0; 4]);
            c::set_pf(w, id, pv::K + 8, -vz);
            c::set_pf(w, id, pv::K + knock::k::GRAVITY, c::DT2 * 19.6);
        }
    }
    if c::pu8(w, id, pv::HIDDEN) != 0 { return true; }
    let cam = w.camera.map(|x| f32::from_bits(x.0));
    let p = c::pos(w, id);
    if !(c::dist2(p, cam) < 70.0 && (w.counter & 7) as i16 == c::pi16(w, id, pv::SLOT)) { return true; }
    let near_z = nearest(w, path(w, id), p).map_or(0.0, |(q, _)| q[2]);
    if p[2] < 5.0 || p[2] < near_z - 10.0 {
        explode(w, id, 0x200);
        return false;
    }
    true
}

/// `0x2c6fd0`.
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { return; }
    let range = c::pf(w, id, pv::SIGHT);
    let t = target::acquire(w, id, range);
    let d = c::dist2(c::pos(w, id), t.pos);
    // `0x251d70(2.5, +0x150)`: the head record's scale request, 2.5 with the big-head cheat (0x15edb7), else 1.0.
    crate::moby_update::manip::big_head_scale(w, 2.5, id, pv::HEAD);
    if c::pu8(w, id, pv::HIDDEN) == 1 {
        if !mission_done(w, id) { return; }
        let m = w.mm(id);
        m.has_collision = false;
        m.mode |= mode::HIDDEN | mode::NO_ANIM;
        return;
    }
    if w.svc.game_mode == 2 {
        w.mm(id).mode |= mode::HIDDEN | mode::NO_ANIM;
    } else if state(w, id) != st::AWAY {
        w.mm(id).mode &= !(mode::HIDDEN | mode::NO_ANIM);
    }
    let p = c::pos(w, id);
    if p.iter().take(3).any(|&x| !(2.0..=1021.0).contains(&x)) || (c::pi32(w, id, pv::PATH_ID) == -1 && state(w, id) != st::DYING) {
        w.delete_moby(id);
        return;
    }
    hits(w, id);
    {
        let mut b = c::pu8(w, id, pv::PAUSE);
        crate::moby_update::services::fast_dec_timer_u8(&mut b);
        c::set_pu8(w, id, pv::PAUSE, b);
    }
    if w.m(id).visible != 0 || d < 24.0 { look(w, id, d, &t); }
    match state(w, id) {
        st::INIT => init(w, id),
        st::STAND => stand(w, id, d, &t),
        st::POD => {
            let dz = dz_abs(&t, c::pos(w, id));
            if !(c::pf(w, id, pv::SIGHT) <= d || 4.0 <= dz) && seq_b(w, id) != 10 {
                blend_now(w, id, 10, 10);
            } else {
                let pod = link(w, id, pv::POD).filter(|&m| alive(w, m));
                let a = &w.m(id).anim;
                if pod.is_some() && !(a.seq_a == 10 && a.flags & 2 != 0) {
                    let p = c::pos(w, id);
                    turn_check(w, id, heading_to(p, t.pos), &t);
                } else {
                    set_state(w, id, st::STAND);
                    blend_now(w, id, 0, 10);
                    let t60 = w.ticks(60);
                    c::set_pi16(w, id, pv::COOLDOWN, t60 as i16);
                    c::set_pi32(w, id, pv::POD, -1);
                }
            }
        }
        st::AIM => aim(w, id, d, &t),
        st::FIRE => fire(w, id, d, &t),
        st::PUNCH => punch(w, id, &t),
        st::PATROL => patrol(w, id, d, &t),
        st::DOWN => {
            let p = path(w, id);
            if follow(w, id, p, pv::NODE) {
                to_stand(w, id);
                let q = c::pos(w, id);
                turn_check(w, id, heading_to(q, t.pos), &t);
            }
        }
        st::JUMP => {
            let jp = spline(w, c::pi32(w, id, pv::JPATH));
            if follow(w, id, jp, pv::JNODE) {
                c::set_pi32(w, id, pv::JPATH, -1);
                to_stand(w, id);
                if c::pi32(w, id, pv::POD) == -1 {
                    let q = c::pos(w, id);
                    turn_check(w, id, heading_to(q, t.pos), &t);
                } else {
                    blend_now(w, id, 0, 10);
                    set_state(w, id, st::POD);
                }
            }
        }
        st::KNOCKED => {
            if !knocked(w, id, &t) { return; }
        }
        st::FALL => {
            if !fall(w, id) { return; }
            turn(w, id);
        }
        st::TURN => turn(w, id),
        st::DYING => {
            if !dying(w, id) { return; }
        }
        st::AWAY => {
            blend(w, id, 0, 10);
            w.mm(id).mode &= !mode::TARGETABLE;
            to_start(w, id);
            let m = w.mm(id);
            m.mode |= mode::HIDDEN | mode::NO_ANIM;
            m.has_collision = false;
        }
        st::BACK => {
            if mission_done(w, id) {
                w.delete_moby(id);
                return;
            }
            let o = w.m(id).o_class;
            let col = crate::moby_update::classes::units::class_collision(w, o);
            let m = w.mm(id);
            m.has_collision = col;
            m.mode = (m.mode & !(mode::HIDDEN | mode::NO_ANIM)) | mode::TARGETABLE;
            let hp = c::pi16(w, id, pv::METER) as f32;
            c::set_pf(w, id, pv::D, hp);
            to_start(w, id);
            set_state(w, id, st::STAND);
        }
        _ => {}
    }
    tail(w, id);
}

// ---------------------------------------------------------------------------------------------------------
// The rocket 833

/// The rocket's pvars: +0x00 velocity, +0x10 the shooter (moby index, −1 none), +0x14 the life (ticks), +0x18 0.75,
/// +0x1c the heading, +0x20 the pitch's spring velocity, +0x24 the speed, +0x28 0.
pub mod rv {
    pub const VEL: usize = 0x00;
    pub const SHOOTER: usize = 0x10;
    pub const LIFE: usize = 0x14;
    pub const K18: usize = 0x18;
    pub const HEADING: usize = 0x1c;
    pub const PITCH_V: usize = 0x20;
    pub const SPEED: usize = 0x24;
    pub const W28: usize = 0x28;
    pub const SIZE: usize = 0x30;
}

/// Level03 `0x2d4288(heading, speed, k18, &vel, &pos, shooter, life)`: the rocket (module doc). None when no slot.
#[allow(clippy::too_many_arguments)]
pub fn spawn_rocket(w: &mut World, heading: f32, speed: f32, k18: f32, vel: c::V, pos: c::V, shooter: MobyId, life: i32) -> Option<MobyId> {
    let r = w.create_moby(ROCKET)?;
    {
        let m = w.mm(r);
        m.update_dist = 0xff;
        m.draw_dist = 0x7e;
        m.state = 1;
        m.visible = 1;
        m.position = pos;
        if m.pvars.len() < rv::SIZE { m.pvars.resize(rv::SIZE, 0); }
    }
    c::set_pv4(w, r, rv::VEL, vel);
    c::set_yaw(w, r, c::atan(vel[0], vel[1]));
    let l = w.ticks(life);
    c::set_pi32(w, r, rv::LIFE, l);
    c::set_pf(w, r, rv::K18, k18);
    c::set_pf(w, r, rv::HEADING, heading);
    c::set_pf(w, r, rv::SPEED, speed);
    c::set_pi32(w, r, rv::SHOOTER, shooter as i32);
    c::set_pi32(w, r, rv::W28, 0);
    let sp = c::pos(w, shooter);
    let from = [sp[0], sp[1], pos[2], sp[3]];
    if let Some(o) = w.coll_line(to_pv(from), to_pv(pos), 2, Some(shooter)) {
        c::set_pi32(w, r, rv::LIFE, 0);
        c::set_pos(w, r, [o.point[0], o.point[1], o.point[2], pos[3]]);
        c::set_pf(w, r, rv::SPEED, 0.0);
    }
    w.build_matrix(r);
    Some(r)
}

/// Level03 `0x2d43c8`: the rocket (module doc).
pub fn rocket_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < rv::SIZE { return; }
    match state(w, id) {
        1 => {
            let speed = c::pf(w, id, rv::SPEED);
            let life = c::pi32(w, id, rv::LIFE);
            let t = target::acquire(w, id, ((life << 1) as f32) * speed);
            let old = c::pos(w, id);
            let vel = c::pv4(w, id, rv::VEL);
            let pm = w.hero.plat_applied.map(|x| f32::from_bits(x.0));
            let lift = [pm[0], pm[1], pm[2] * k::PLATFORM_CLIMB, pm[3]];
            let mut p = c::add(old, vel);
            p = c::add(p, lift);
            c::set_pos(w, id, p);
            let h = c::pf(w, id, rv::HEADING);
            c::set_yaw(w, id, h);
            let roll = c::add_rot(w.m(id).rotation[0], c::DT * 6.283_185_5);
            w.mm(id).rotation[0] = roll;
            let (cs, sn) = c::cs(h);
            let back = [cs * speed * -0.5, sn * speed * -0.5, 0.0, 0.0];
            let pv_ = c::add(c::add(back, vel), lift);
            let at = c::add(c::set_len3(vel, -0.2), p);
            let n1 = w.rng.rand_range(0xf, 0x16);
            let l1 = w.ticks(n1);
            let g1 = w.rng.rand_range(0x14, 0x23) as i16;
            fx::part04(w, &crate::particles::type04::Spawn { pos: at, vel: pv_, c1: 0x6f00_afff, c2: 0xff, life: l1, base: 0x28, growth: g1, additive: true });
            let n2 = w.rng.rand_range(0x1e, 0x3c);
            let l2 = w.ticks(n2);
            let g2 = w.rng.rand_range(0x32, 0x4b) as i16;
            fx::part04(w, &crate::particles::type04::Spawn { pos: at, vel: pv_, c1: 0x1fff_ffff, c2: 0x4f_4f4f, life: l2, base: 0x28, growth: g2, additive: false });
            if t.moby.is_some() {
                let p = c::pos(w, id);
                let a = c::atan(c::dist2(p, t.aim), t.aim[2] - p[2]);
                let mut v = c::pf(w, id, rv::PITCH_V);
                let pitch = turn::spring_turn(w.m(id).rotation[1], -a, c::DT2 * std::f32::consts::FRAC_PI_4, c::DT2 * std::f32::consts::PI, c::DT * std::f32::consts::FRAC_PI_6, &mut v);
                c::set_pf(w, id, rv::PITCH_V, v);
                w.mm(id).rotation[1] = pitch;
                w.mm(id).position[2] += (-pitch).sin() * speed;
            }
            let shooter = usize::try_from(c::pi32(w, id, rv::SHOOTER)).ok().filter(|&s| s < w.table.mobys.len());
            let p = c::pos(w, id);
            let hit = w.coll_line(to_pv(p), to_pv(old), 0, shooter).or_else(|| w.coll_sphere(to_pv(p), to_pf(0.2), 0, shooter));
            if let Some(o) = hit {
                if let Some(m) = o.moby {
                    let (cs, sn) = c::cs(h);
                    let tmpl = HitTemplate { dir: [to_pf(cs), to_pf(sn), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x1_0001, b18: 1, b19: 1, h1a: w.m(id).o_class as u16, damage: Pf::ONE, w20: 1 };
                    w.deliver_hit(m, &tmpl);
                }
                set_state(w, id, 2);
            }
            if c::dec_timer_pvar_i32(w, id, rv::LIFE) != 0 { set_state(w, id, 2); }
        }
        2 => {
            let p = c::pos(w, id);
            fx::death_explosion(w, 0.25, 13.0, Some(id), p, 0);
            w.delete_moby(id);
        }
        _ => {}
    }
}
