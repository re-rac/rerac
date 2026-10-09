//! **Qwark's ship, class 388** (level13 `0x2eb098`, census U434, one instance): the boss of the Gemlik base battle,
//! fought from Gemlik's ship 69 ([`super::gemlik_ship`]). It flies the level's paths in phases (moby +0xbc, 1..7),
//! fires homing missiles 82 and drifting mines 83 ([`super::qwark_ship_parts`]), grabs the player's ship with a
//! tractor beam (state 5), raises a shield (352) in phase 4, loses a part at every phase, taunts (the `qwark_boss_audio`
//! lines), and at 0 health spins down a crash path (state 7) and blows up (state 8): the player's ship is set to land
//! (its state 8) and the story director 1353 ([`super::gemlik_story`]) frees it, which unlocks planet 14. Read from
//! the level13 decomp and disassembly (`0x2eb098` and its 19 helpers, the level's names mapped to level 01 with
//! `rc-trace overlay-diff`); the tables from the overlay. Native `f32`.
//!
//! **System or not.** Per-class code (a cluster of one), on the shared pieces: the creature layer's damage resolver
//! (`creature::damage`, the record at +0x20), hit flash (`creature::flash`, the record at +0x60) and death explosion,
//! the level paths (`Services::splines`, `path::nearest_at_distance`), the vehicle record ([`crate::vehicle`]), the
//! boss meter ([`crate::hud::Element::Boss`], slot 1), the dialogue player (`help::Help::{voice, continue_stream}`), the
//! point lights.
//!
//! **Pvar block** (0x150; the path / cuboid / camera words come with the instance): +0x20 the damage record (health;
//! +0x24 s16 the full health, +0x26 the own cooldown), +0x60 the flash record (+0x67 its red), +0x70 the velocity,
//! +0x80 the tractor's end, +0x9c / +0x9e (s16) the path point ahead / behind, +0xa4 / +0xa8 the pitch / yaw turn
//! rates, +0xac the speed, +0xb0..+0xc0 five paths, +0xc4..+0xe8 the ten parts, +0xec itself, +0xf0 the hit shield
//! (401), +0xf8 the timer, +0xfc..+0xff bytes (the gun side, the shot count, the weapon, the tractor caught), +0x100
//! the tractor's timer, +0x104 the meter, +0x108 its value, +0x10c (s8) the direction on the path, +0x10e, +0x110 the
//! shield 352, +0x114 / +0x118 / +0x11c the engine / shield / tractor voices, +0x120 the light slot, +0x124 a timer,
//! +0x12c the crash camera record, +0x130 the crash cuboid, +0x134 the player's ship (moby index), +0x138 (u16) the
//! taunt flags, +0x13a a taunt loading, +0x13b the path in use, +0x13c the lines used, +0x144 / +0x146 (s16) the taunt
//! pauses, +0x148 a phase almost done.
//!
//! **Level data** (the overlay): the parts' classes 0x1d34d8 ([`PARTS`]), the path by phase 0x1d3448, the part lost by
//! phase 0x1d3478, the last parts 0x1d3490, the shield joint by phase 0x1d34a8, the smoke parts / joints 0x1d3508 /
//! 0x1d3520 / 0x1d3538, the taunt lines 0x1d3558 ([`LINES`]), the light offset 0x160660 (1, 0, 0); the words gp−0x52ac
//! (the time of the first attack, −1 at load) and gp−0x52b0 (the win-taunt cycle).
//!
//! ## Coverage (`0x2eb098`)
//! | address | what | port |
//! |---|---|---|
//! | entry | +0x108 = the meter value: `trunc(health·16) & 0xfff0`, `\| 0xf` from `(full − 0.5)·16`; above 0.5 + `((trunc(health) % (full/6)) << 4) / (full/6)` (≥ 0, & 0xf), else 0 | [`update`] |
//! | 0 | the mission done → `DeleteMoby`; scale = class scale·4; the ten parts (`0x2ecd10`); +0xec = itself; each path whose last point is within 0.5 of its first loses it; speed 4.2·dt, velocity along the yaw; at path 0's first point; the voices / light / timers reset, health = full, the meter released; collision off, mode \| 0x41; → 1 | [`init`] |
//! | 1 | Ratchet in 0x32 and the vehicle record set: shown, draw distance 0x3ff, collision on; the first attack's time (0x15eea4) kept; 26+ (41+) minutes·6 since it → full health 120 (102), the ship's lock time 0x234 (0x221); → 2 | [`update`] (`units::Globals::word_or`) |
//! | 2 | the ship (0x45) +0xbc = 0; the meter (`queue_animation_update(0x11, …, &+0x108, full·16 \| 0xf)`); speed → 42·dt by a tenth; path 0 to its end (`0x2e9160`): phase 1, the ship's lock cleared, the path of phase 1 (`0x28b510` + `rand_range(3, 7)`), +0xf8 = `ticks(20)`, → 3 | [`update`] (`hud::Calls::{data, queue}`) |
//! | 3 / 4 | the meter; the ship +0xbc = 0; the attack (`0x2eac00`); the path steer (`0x2e9408`): at a point with w > 0 the phase's path taken; phase 4 → the flash, else the hits (`0x2e9a48`) | [`update`] |
//! | 5 | the tractor: the meter, the attack; the tractor voice (`PlayClassSound(4, 4)`); not caught: the ship +0xbc = 0, speed → 30·dt, the beam's end runs at 40·dt to Ratchet, reached → +0xf8 = `ticks(120)`, the taunt 0x17, caught; the path; the beam broken by the world or class 0xd4 between joint 0x13 + 8 and its end, its end out of view (`FastBSphereCheck(100)`) while Qwark is not drawn, or Ratchet 100 away → +0x100 = 0 | [`tractor`] |
//! | 5 | caught: turned at Ratchet (`0x2e9018`), the ship +0xbc = 1, speed ·= 0.8, the end pulled to 30 from joint 0x13 at 15·dt, the ship's position = the end, its yaw / pitch pulled toward Qwark beyond 22°; then the hits or the flash; within 60° of Ratchet → the beam's draw (`0x2ea8c8`) | [`tractor`] (`Callback::UnitQuads`, [`beam_quads`]) |
//! | 6 | the ship +0xbc = 0; the path; the dialogue player idle and +0x138 < 1: a win taunt (0x1d + the cycle), the cycle + 1 mod 4, +0x138 + 1 | [`update`] |
//! | 7 | the crash: the ship +0xbc = 0; +0xf8 = 1 → part 8 lost (`0x2e9680`), `PlayClassSound(7, 0)`; the crash path without turning; its end or its last point → +0xf8 = `ticks(180)`, → 8; roll + 240°·dt; turned toward the point 3 ahead (`SpringTurn2` / `SpringTurn`, π / 2π·dt², 2π·dt); scale ·= 0.99925, speed ·= 0.9992; 1 in 17 a death explosion (`0x273f50(1.6·s, 13, m, …, −1)`) within (±5, ±4, ±4)·s; the smoke (`0x2e8b58`); the taunts 0x23 / 0x24 | [`update`] (`fx::death_explosion`) |
//! | 8 | the ship +0xbc = 0; roll + 240°·dt; facing the camera (0x1670d4 / 0x1670d8); scale ·= 0.98; at `ticks(30)` left `SpawnBeamExplosion(0, 0, 5, 3, 0, 1, 0, m, …, 30, 8, 32, 7, 1)`; the smoke; the timer out: the player's ship (+0x134) → state 8, mode \| 2; the light freed; the crash camera disarmed (`0x316f48`); `DeleteMoby` | [`update`] |
//! | 9 | the quit: the shield, 352 and the parts deleted, the voices and the light released, the meter released; back at path 0's start facing its second point; the state words reset; → 0 | [`reset`] |
//! | tail | the position clamped to [22, 1001]; phase 4: the shield 352 (`0x2e8438`) made or kept up (state > 2 → 1), its voice (`PlayClassSound(6, 4)`); else its voice released and 352 (still 352, alive) faded (state < 3 → 3) | [`tail`] |
//! | tail | state ≥ 2: the parts held to their joints (`0x2e8d28`), the part smoke (`0x2e8920`, not in the last `ticks(120)` of state 8), the engine voice, the light (`WritePointLight_B(50, 0, 3, 0.5, 0.01)` 7 behind it, then its position) | [`tail`] (`PointLights`) |
//! | tail | not in 6: the player's ship gone (not 0x45, deleted or wrecked) → 6 with a win taunt | [`tail`] |
//! | tail | no taunt loading: not in 6, Ratchet flying the ship, the ship hit by a fighter shot this tick (+0x120 in 1..9999), 1 in 2 → the taunt 3 (of 9); a taunt loading and the player buffered (0x15172a = 3): `continue_audio_stream_if_ready`, +0x144 = `ticks(360)`, +0x146 = `ticks(60)` | [`tail`] (`Help::continue_stream`) |
//! | tail | the taunt pauses tick; states 2..5 and the vehicle's quit → 9 | [`tail`] |
//!
//! ## Coverage (the helpers)
//! | address | what | port |
//! |---|---|---|
//! | 0x2e86f8 | `(base, n)`: line `LINES[base + rand() % n]`; under 0x1d the pauses and +0x148 refuse it; the player idle (or base > 0x1c): 0x1516ec = line + 50000, +0x13a = 1; under 0x1d skipping the lines already used (+0x13c bits; under 0xc a used one is freed 1 in 4) | [`taunt`] |
//! | 0x2e9160 | `(sel, turn)`: along path +0xb0[sel]: past the point ahead (by speed) → the next (direction +0x10c), wrapping (the return); f = progress clamped [0.05, 1]; velocity = (1 − f)·velocity + f·(speed toward the point), at speed; position += it; turned toward it (state 5: toward Ratchet) by `0x2e9018` | [`follow`] |
//! | 0x2e9408 | `(sel, turn)`: the nearer of the next / previous points to Ratchet: away from him → speed eased to (30, 90 in phase 4)/distance, else slowed by a tenth and turned round below 0.15; `0x2e9160`; the point ahead's w > 0 → \| 2 | [`steer`] |
//! | 0x2e9018 | `SpringTurn2(yaw)`, `SpringTurn(pitch)` (π / 2π·dt², 2π·dt); roll −= the yaw rate·(π/2 − \|roll\|)/(π/2)·0.5, clamped ±π/2, ·0.985 | [`turn_to`] |
//! | 0x2e9a48 | state < 6: the hit shield 401, then the parts and itself (`MobyGetHitMessage(0x230000)`, `0x26f378(m, hit, +0x20, 0, …, 4)`): fighter shots (0x52 / 0x53 / 0xd4) ignored; the laser 0x3f1 deals 0.3 to the shield, 0.1 to a part, the missile 0x127 half; phase 7 ×1.5; fatal → `0x2e97e0`; else health − damage, +0x67 = 0xfa, the flash; below the phase's line (`(6 − phase')·full/6`, +0.3 of a sixth: +0x148): the part of the phase lost, `PlayClassSound(7, 0)`, the tractor dropped (5 → 4), the next phase, the ship's lock cleared; at phase 7 parts 4..7 lost; the taunts of the new phase (0xc, 0x10 + `ticks(120)` on +0x100, 0x11, 0x13, 0x16) or of a phase 7 almost done (0x14 ×2); the flash update | [`hits`] (`creature::{damage, flash}`) |
//! | 0x2e97e0 | death: health 0; not targetable; `SetDeathBits(m, 0, −1)`; +0x67 = 0x78, the flash; path 4 from its first point at 80·dt, velocity along the crash cuboid's first row at 10·dt; the player's ship hidden and stopped (mode \| 3, not drawn); the crash camera armed (`0x316f20`); → 7, +0xf8 = `ticks(600)`; the taunt 0x22; the meter released; +0x13c = −1 | [`die`] (the crash camera: NOT ported, G-HERO-027 class 19) |
//! | 0x2e9680 | part k lost: `0x273f50(2, 13, m, its position, −1)`; its velocity 0.95·Qwark's jittered 30·dt, its spin `randf_sym(20°·dt)`, `(π·dt)`, `(π·dt)`; state 3; position += velocity; ambient 0x40404000; hit slot 0xff; the slot cleared | [`lose_part`] |
//! | 0x2e8d28 | the parts at their joints (clamped [22, 1001]) with Qwark's rotation and rows; the hit shield 401 (`0x2ecd10`) at the phase's joint (none in state 5: deleted unless dying); the shield 352 at Qwark | [`attach`] |
//! | 0x2e8920 | the six smoke parts (when present): at their joints, odd ticks a white-hot puff (`PartType04Spawn(…, 0xcf0000ff, 0xcf, ticks(15..20), 350·s, −100, 1)`), even ticks unless the phase's own part (1 in 9) a dark one (0x6000ffff, 0x80, 600·s); then `(0xefff7f4f, 0xff0000, ticks(8..12), randf(100, 250)·s, −that, 1)` | [`smoke`] |
//! | 0x2e8b58 | the crash smoke at the joints 20, 9, 14, 4, 6, 8: jittered 1.4·s·`randf(0.8, 1.1)`, odd ticks `(0xdf000fff, 0xcf, ticks(20..60), randf(450, 550)·s)`, even ticks (1 in 12 with a phase) `(0x8000ffff, 0x80, ticks(20..50), randf(600, 800)·s)` | [`crash_smoke`] |
//! | 0x2eac00 | the attack by phase: 1 the missile volleys (11 a minute, then 3); 2 / 7 missiles and mines alternating (`FastDecTimer__FRc(+0xfd)`); 3 / 5 / 6 the tractor (Ratchet 60..100 from joint 0x13, +0x100 / +0x124 / +0xf8 out → 5), else missiles / mines; 4 the shield's start (+0x10e = 0 → phase 5, the taunt 0x1b) | [`attack`] |
//! | 0x2ea540 | a missile: joint 0x10 / 0x12 alternating (in phase 6 1 in 2), `0x2c13b0(18·dt + speed, 18·dt, m, p, the ship, (0, 25°, yaw), ticks(300))`, `PlayClassSound(2, 0)` (the lead `0x2ea400`: its result unused) | [`fire_missile`] |
//! | 0x2ea6b8 | a mine: joint 0xe; toward the ship's lead point (`0x2ea330`, 20·dt) at 20·dt plus its rows 1 / 2 by `randf_sym(30·dt)` at `randf(5·dt, 30·dt)` (no ship: `rand_vec(0, 30·dt)`), + Qwark's velocity; `0x2c1f28`; `PlayClassSound(3, 0)` | [`fire_mine`] |
//! | 0x2ea8c8 | the beam's draw: from joint 0x13 toward +0x80 (at most 100), one FX 0x34 quad every max(d/100, 1), scrolled by the frame mod 20, widening; alpha fading over the last 10 | [`beam_quads`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::fx::{self, polar};
use crate::moby_update::creature::{self as c, add, add_rot, atan, diff_rots, dist2, dist3, set_len3, sub, sub_rot, DT, DT2, SPEED};
use crate::moby_update::services::World;
use crate::moby_update::story;

pub const REFERENCE_LEVEL: u32 = 13;
pub const UPDATE_FN: u32 = 0x2e_b098;
pub const BEAM_FN: u32 = 0x2e_a8c8;
pub const CLASS: i16 = 0x184;
pub const CLASSES: [i16; 1] = [CLASS];

pub mod pvo {
    pub const HEALTH: usize = 0x20;
    pub const FULL: usize = 0x24;
    pub const OWN_COOLDOWN: usize = 0x26;
    pub const FLASH: usize = 0x60;
    pub const FLASH_RED: usize = 0x67;
    pub const VEL: usize = 0x70;
    pub const BEAM_END: usize = 0x80;
    pub const AHEAD: usize = 0x9c;
    pub const BEHIND: usize = 0x9e;
    pub const PITCH_V: usize = 0xa4;
    pub const YAW_V: usize = 0xa8;
    pub const SPEED: usize = 0xac;
    pub const PATHS: usize = 0xb0;
    pub const PARTS: usize = 0xc4;
    pub const SELF: usize = 0xec;
    pub const HIT_SHIELD: usize = 0xf0;
    pub const TIMER: usize = 0xf8;
    pub const GUN_SIDE: usize = 0xfc;
    pub const COUNT: usize = 0xfd;
    pub const WEAPON: usize = 0xfe;
    pub const CAUGHT: usize = 0xff;
    pub const TRACTOR_T: usize = 0x100;
    pub const METER: usize = 0x104;
    pub const METER_VALUE: usize = 0x108;
    pub const DIR: usize = 0x10c;
    pub const SHIELD_START: usize = 0x10e;
    pub const F10F: usize = 0x10f;
    pub const SHIELD: usize = 0x110;
    pub const ENGINE_VOICE: usize = 0x114;
    pub const SHIELD_VOICE: usize = 0x118;
    pub const TRACTOR_VOICE: usize = 0x11c;
    pub const LIGHT: usize = 0x120;
    pub const T124: usize = 0x124;
    pub const CRASH_CAMERA: usize = 0x12c;
    pub const CRASH_CUBOID: usize = 0x130;
    pub const SHIP: usize = 0x134;
    pub const TAUNT_FLAGS: usize = 0x138;
    pub const TAUNTING: usize = 0x13a;
    pub const PATH_SEL: usize = 0x13b;
    pub const LINES_USED: usize = 0x13c;
    pub const PAUSE_A: usize = 0x144;
    pub const PAUSE_B: usize = 0x146;
    pub const ALMOST: usize = 0x148;
    /// Port slots past the game's block: the beam's draw inputs taken when it registers (joint 0x13's point, the
    /// tick mod 20), since the draw-only callbacks have no pose or tick.
    pub const BEAM_JOINT: usize = 0x150;
    pub const BEAM_SCROLL: usize = 0x160;
    pub const LEN: usize = 0x170;
}

/// 0x1d34d8: the parts' classes, in slot order.
pub const PARTS: [i16; 10] = [395, 396, 399, 400, 390, 391, 392, 393, 394, 389];
/// The hit shield (`0x2ecd10` with class 0x191).
pub const HIT_SHIELD: i16 = 0x191;
/// The shield of phase 4 (`0x2e8438`).
pub const SHIELD: i16 = 0x160;
/// 0x1d3448: the path by phase.
const PATH_BY_PHASE: [usize; 12] = [0, 1, 1, 2, 2, 2, 3, 1, 1, 1, 1, 1];
/// 0x1d3478: the part a phase loses (−1 none).
const PART_BY_PHASE: [i32; 6] = [3, 2, 1, -1, 0, 9];
/// 0x1d3490: the parts lost at phase 7.
const LAST_PARTS: [usize; 4] = [4, 5, 6, 7];
/// 0x1d34a8: the hit shield's joint by phase (−1 none).
const SHIELD_JOINT: [i32; 12] = [-1, 13, 12, 11, -1, 10, 9, 20, 20, 20, 20, 20];
/// 0x1d3508 / 0x1d3520: the smoking parts (10: Qwark himself) and their joints.
const SMOKE_PARTS: [usize; 6] = [9, 0, 1, 2, 3, 10];
const SMOKE_JOINTS: [usize; 6] = [9, 10, 11, 12, 13, 20];
/// 0x1d3538: the crash smoke's joints.
const CRASH_JOINTS: [usize; 6] = [20, 9, 14, 4, 6, 8];
/// 0x1d3558: the taunt lines (`qwark_boss_audio` ids − 50000).
pub const LINES: [u8; 37] = [0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 21, 19, 15, 16, 17, 18, 27, 22, 23, 24, 20, 25, 26, 31, 30, 29, 28, 32, 33, 34, 35];
/// gp−0x52ac / gp−0x52b0.
const FIRST_ATTACK: u32 = 0x16_1954;
const WIN_CYCLE: u32 = 0x16_1950;
/// The fighter shots (and class 0xd4) whose hits the ship's parts ignore.
const OWN_SHOTS: [i16; 3] = [0x52, 0x53, 0xd4];
/// The player's ship.
const SHIP_CLASS: i16 = 0x45;

const PI: f32 = std::f32::consts::PI;
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;

fn pf(w: &World, id: MobyId, o: usize) -> f32 { c::pf(w, id, o) }
fn set(w: &mut World, id: MobyId, o: usize, v: f32) { c::set_pf(w, id, o, v) }
fn pi(w: &World, id: MobyId, o: usize) -> i32 { c::pi32(w, id, o) }
fn seti(w: &mut World, id: MobyId, o: usize, v: i32) { c::set_pi32(w, id, o, v) }
fn p16(w: &World, id: MobyId, o: usize) -> i16 { c::pi16(w, id, o) }
fn set16(w: &mut World, id: MobyId, o: usize, v: i16) { c::set_pi16(w, id, o, v) }
fn pu8(w: &World, id: MobyId, o: usize) -> u8 { c::pu8(w, id, o) }
fn setu8(w: &mut World, id: MobyId, o: usize, v: u8) { c::set_pu8(w, id, o, v) }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(pi(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn set_link(w: &mut World, id: MobyId, o: usize, m: Option<MobyId>) { seti(w, id, o, m.map_or(0, |m| m as i32 + 1)) }
fn gone(w: &World, m: MobyId) -> bool { let s = w.m(m).state; s == 0xfe || s == 0xfd }
fn hero(w: &World) -> [f32; 4] { let h = w.hero_point(); [h[0], h[1], h[2], 0.0] }
fn word(w: &World, a: u32, init: u32) -> u32 { w.svc.units.word_or(a, init) }
fn set_word(w: &mut World, a: u32, v: u32) { w.svc.units.set_word(a, v) }
fn row(name: u32) -> Option<u16> { super::row(REFERENCE_LEVEL, name) }

/// The player's ship when the vehicle record names a moby of class 0x45.
fn ship(w: &World) -> Option<MobyId> { w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len() && w.m(v).o_class == SHIP_CLASS) }

/// The ship's +0xbc (the tractor's hold).
fn hold_ship(w: &mut World, on: u8) { if let Some(v) = ship(w) { w.mm(v).cmd = on; } }

/// The ship's lock cleared (+0x118, +0x88, +0xec, +0x8c).
fn clear_ship_lock(w: &mut World) {
    use super::gemlik_ship::pvo as s;
    let Some(v) = w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len()) else { return };
    story::pvars(w, v, s::LEN);
    for o in [s::LOCK_LOST, s::LOCK, s::MISSILE_TARGET, s::LOCK_T] { c::set_pi32(w, v, o, 0); }
}

/// Path `+0xb0[sel]`'s points (empty: no path).
fn path(w: &World, id: MobyId, sel: usize) -> Vec<[f32; 4]> {
    let i = pi(w, id, pvo::PATHS + 4 * sel);
    usize::try_from(i).ok().and_then(|i| w.svc.splines.get(i)).map_or(Vec::new(), |p| p.iter().map(|q| q.map(f32::from_bits)).collect())
}

fn point(p: &[[f32; 4]], i: i32) -> [f32; 4] { usize::try_from(i).ok().and_then(|i| p.get(i)).copied().unwrap_or([0.0; 4]) }

fn release_voice(w: &mut World, id: MobyId, o: usize) {
    let s = pi(w, id, o);
    if s != -1 { w.release_sound(s, id); }
    seti(w, id, o, -1);
}

fn keep_voice(w: &mut World, id: MobyId, o: usize, index: i32) {
    let s = pi(w, id, o);
    if s == -1 || !w.sound_alive(s, id) {
        seti(w, id, o, -1);
        let n = w.play_sound(index, 4, id);
        seti(w, id, o, n);
    }
}

/// The meter's data key: the moby's +0x108.
pub fn meter_key(w: &World, id: MobyId) -> u32 { crate::hud::calls::pvar_key(w.m(id).spawn_id, id, pvo::METER_VALUE) }

fn meter_request(w: &World, id: MobyId) -> crate::hud::Request {
    let full = p16(w, id, pvo::FULL) as i32;
    crate::hud::Request { slot: 1, flags: 0x10, icon: 0xffff, element: crate::hud::Element::Boss { key: meter_key(w, id) }, max: (full << 4) | 0xf }
}

/// `queue_animation_update(0x11, 0xffff, …, &+0x108, full·16 | 0xf)` → +0x104.
fn meter(w: &mut World, id: MobyId) {
    let r = meter_request(w, id);
    let v = pi(w, id, pvo::METER_VALUE);
    w.svc.hud.data(meter_key(w, id), v);
    w.svc.hud.queue(r);
    seti(w, id, pvo::METER, 1);
}

/// `FUN_0024b090(+0x104, 0)`, +0x104 = −1.
fn release_meter(w: &mut World, id: MobyId) {
    if pi(w, id, pvo::METER) == -1 { return; }
    let r = meter_request(w, id);
    w.svc.hud.set_flags(r, 0);
    seti(w, id, pvo::METER, -1);
}

/// The path of `sel` from its point nearest Qwark plus `rand_range(3, 7)` (the start of a phase's path).
fn start_path(w: &mut World, id: MobyId, sel: usize) {
    let p = path(w, id, sel);
    let pts: Vec<[u32; 4]> = p.iter().map(|q| q.map(f32::to_bits)).collect();
    let pos = w.m(id).position;
    let n0 = crate::path::nearest_at_distance(&pts, 0.0, pos) as i16;
    let r = w.rng.rand_range(3, 7) as i16;
    let count = p.len() as i32;
    if count == 0 { return; }
    let a = ((n0.wrapping_add(r)) as i32 % count) as i16;
    set16(w, id, pvo::AHEAD, a);
    let b = a - 1;
    set16(w, id, pvo::BEHIND, if b < 0 { (count - 1) as i16 } else { b });
}

/// The phase's path taken (the path by the phase, its start).
fn take_phase_path(w: &mut World, id: MobyId) {
    let want = PATH_BY_PHASE[(w.m(id).cmd as usize).min(11)];
    setu8(w, id, pvo::PATH_SEL, want as u8);
    start_path(w, id, want);
}

/// Level13 `0x2e86f8(base, n)`: a taunt (module doc).
fn taunt(w: &mut World, id: MobyId, base: usize, n: i32) -> bool {
    let mut r = w.rng.rand() % n.max(1);
    let line = |r: i32| LINES.get(base + r as usize).copied().unwrap_or(0) as u32;
    let mut l = line(r);
    if base < 0x1d {
        if base < 0xc {
            if pu8(w, id, pvo::ALMOST) != 0 || 0 < p16(w, id, pvo::PAUSE_A) { return false; }
        } else if 0 < p16(w, id, pvo::PAUSE_B) || (pu8(w, id, pvo::ALMOST) != 0 && 0x16 < base && base < 0x1b) {
            return false;
        }
    }
    let idle = !w.svc.help.voice.busy && w.svc.help.voice.request == -1;
    if !idle && base <= 0x1c { return false; }
    if base > 0x1c {
        w.svc.help.voice.request = l as i32 + crate::help::QWARK_BASE;
        setu8(w, id, pvo::TAUNTING, 1);
        return true;
    }
    let used = |w: &World| pi(w, id, pvo::LINES_USED) as u32;
    let mut tries = 0;
    while (used(w) >> (l & 0x1f)) & 1 != 0 {
        if base < 0xc && w.rng.randi(4) == 0 {
            let u = used(w) & !(1 << (l & 0x1f));
            seti(w, id, pvo::LINES_USED, u as i32);
        }
        r = (r + 1) % n.max(1);
        l = line(r);
        tries += 1;
        if n <= tries {
            if base < 0xc {
                let u = used(w) & !(1 << (l & 0x1f));
                seti(w, id, pvo::LINES_USED, u as i32);
            }
            return false;
        }
    }
    w.svc.help.voice.request = l as i32 + crate::help::QWARK_BASE;
    setu8(w, id, pvo::TAUNTING, 1);
    let u = used(w) | (1 << (l & 0x1f));
    seti(w, id, pvo::LINES_USED, u as i32);
    true
}

/// A win taunt (0x1d + the cycle) and the cycle advanced when it played.
fn win_taunt(w: &mut World, id: MobyId) -> bool {
    let k = word(w, WIN_CYCLE, 0) as i32;
    if !taunt(w, id, (k + 0x1d) as usize, 1) { return false; }
    set_word(w, WIN_CYCLE, ((k + 1).rem_euclid(4)) as u32);
    true
}

/// Level13 `0x2eb098` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, pvo::LEN);
    let h = pf(w, id, pvo::HEALTH);
    let full = p16(w, id, pvo::FULL) as i32;
    let mut v = ((h * 16.0) as i32) & 0xfff0;
    if ((full as f32 - 0.5) * 16.0) as i32 <= v { v |= 0xf; }
    if 0.5 < h {
        let n = full / 6;
        let mut f = if n == 0 { 0 } else { ((h as i32 % n) << 4) / n };
        if f < 0 { f = 0; }
        v += f & 0xf;
    } else {
        v = 0;
    }
    seti(w, id, pvo::METER_VALUE, v);
    match w.m(id).state {
        0 => {
            if !init(w, id) { return; }
        }
        1 => {
            if w.hero.state == 0x32 && w.svc.vehicle.moby.is_some() {
                let col = super::class_collision(w, CLASS);
                {
                    let m = w.mm(id);
                    m.mode &= !(mode::HIDDEN | mode::NO_ANIM);
                    m.draw_dist = 0x3ff;
                    m.has_collision = col;
                }
                let now = w.svc.help.play_time;
                if word(w, FIRST_ATTACK, u32::MAX) as i32 == -1 { set_word(w, FIRST_ATTACK, now as u32); }
                let mins = (now - word(w, FIRST_ATTACK, u32::MAX) as i32) / 0x168;
                let set_full = if 0x29 <= mins { Some((102.0, 0x66, 0x221)) } else if 0x1a <= mins { Some((120.0, 0x78, 0x234)) } else { None };
                if let Some((hp, f, lock)) = set_full {
                    set16(w, id, pvo::FULL, f);
                    set(w, id, pvo::HEALTH, hp);
                    if let Some(s) = w.svc.vehicle.moby.filter(|&s| s < w.table.mobys.len()) {
                        story::pvars(w, s, super::gemlik_ship::pvo::LEN);
                        c::set_pi32(w, s, super::gemlik_ship::pvo::LOCK_TIME, lock);
                    }
                }
                w.mm(id).state = 2;
            }
        }
        2 => {
            hold_ship(w, 0);
            meter(w, id);
            let s = pf(w, id, pvo::SPEED);
            set(w, id, pvo::SPEED, s + (DT * 42.0 - s) * 0.1);
            if follow(w, id, 0, true) {
                w.mm(id).cmd = 1;
                clear_ship_lock(w);
                setu8(w, id, pvo::PATH_SEL, PATH_BY_PHASE[1] as u8);
                let t = w.ticks(0x14);
                seti(w, id, pvo::TIMER, t);
                w.mm(id).state = 3;
                start_path(w, id, PATH_BY_PHASE[1]);
            }
        }
        3 | 4 => {
            meter(w, id);
            hold_ship(w, 0);
            attack(w, id);
            let sel = pu8(w, id, pvo::PATH_SEL) as usize;
            let r = steer(w, id, sel, true);
            if r & 2 != 0 && sel != PATH_BY_PHASE[(w.m(id).cmd as usize).min(11)] { take_phase_path(w, id); }
            if w.m(id).cmd == 4 { c::flash::update(w, id, pvo::FLASH); } else { hits(w, id); }
        }
        5 => tractor(w, id),
        6 => {
            hold_ship(w, 0);
            let sel = pu8(w, id, pvo::PATH_SEL) as usize;
            follow(w, id, sel, true);
            if !w.svc.help.voice.busy && w.svc.help.voice.request == -1 && p16(w, id, pvo::TAUNT_FLAGS) < 1 && win_taunt(w, id) {
                let n = p16(w, id, pvo::TAUNT_FLAGS) + 1;
                set16(w, id, pvo::TAUNT_FLAGS, n);
            }
        }
        7 => crash(w, id),
        8 => {
            hold_ship(w, 0);
            let r = add_rot(w.m(id).rotation[0], DT * 4.188_790_3);
            let e = w.hero.loop_in.cam_euler;
            {
                let m = w.mm(id);
                m.rotation[0] = r;
                m.rotation[2] = e[2];
                m.scale *= 0.98;
                m.rotation[1] = e[1];
            }
            if pi(w, id, pvo::TIMER) == w.ticks(0x1e) {
                let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 5.0, flash2: 3.0, flash_dist: 0.0, scale: 1.0, light: 0.0, streaks: 30, sparks: 8, puffs: 32, debris: 0, sound: 7, shake: true };
                let p = w.m(id).position;
                fx::beam_explosion(w, &b, Some(id), p);
            }
            crash_smoke(w, id);
            if c::dec_timer_pvar_i32(w, id, pvo::TIMER) != 0 {
                let s = pi(w, id, pvo::SHIP);
                if let Some(v) = story::link(w, s) {
                    let m = w.mm(v);
                    m.state = 8;
                    m.mode |= mode::NO_UPDATE;
                }
                let l = pi(w, id, pvo::LIGHT);
                if let Ok(i) = usize::try_from(l) { w.svc.point_lights.free(i); }
                // 0x316f48: the crash camera ended while it is current (camera class 19).
                let rec = pi(w, id, pvo::CRASH_CAMERA);
                crate::cinematic::flyby_end(w, rec);
                w.delete_moby(id);
                return;
            }
        }
        9 => {
            reset(w, id);
            return;
        }
        _ => {}
    }
    tail(w, id);
}

/// State 0 (module doc); false when the moby deleted itself.
fn init(w: &mut World, id: MobyId) -> bool {
    let mission = w.m(id).mission;
    if mission != 0xff && story::mission_done(w, mission as i32) {
        w.delete_moby(id);
        return false;
    }
    let s = super::class_scale(w, CLASS) * 4.0;
    w.mm(id).scale = s;
    for (i, &k) in PARTS.iter().enumerate() {
        let p = super::qwark_ship_parts::spawn_part(w, id, i, k);
        set_link(w, id, pvo::PARTS + 4 * i, p);
    }
    set_link(w, id, pvo::SELF, Some(id));
    for k in 0..5 {
        let i = pi(w, id, pvo::PATHS + 4 * k);
        if let Some(p) = usize::try_from(i).ok().and_then(|i| w.svc.splines.get_mut(i)) {
            if let (Some(a), Some(b)) = (p.first().copied(), p.last().copied()) {
                let (a, b) = (a.map(f32::from_bits), b.map(f32::from_bits));
                if dist3(a, b) < 0.5 { p.pop(); }
            }
        }
    }
    let sp = DT * 42.0 * 0.1;
    set(w, id, pvo::SPEED, sp);
    let yaw = w.m(id).rotation[2];
    c::set_pv4(w, id, pvo::VEL, [yaw.cos() * sp, yaw.sin() * sp, 0.0, 0.0]);
    let p0 = point(&path(w, id, 0), 0);
    w.mm(id).position = p0;
    set16(w, id, pvo::BEHIND, 0);
    set16(w, id, pvo::AHEAD, 1);
    {
        let m = w.mm(id);
        m.draw_dist = 0xff;
        m.update_dist = 0xff;
        m.state = 1;
    }
    setu8(w, id, pvo::DIR, 1);
    setu8(w, id, pvo::WEAPON, 0);
    setu8(w, id, pvo::GUN_SIDE, 0);
    setu8(w, id, pvo::SHIELD_START, 8);
    seti(w, id, pvo::SHIELD, 0);
    for o in [pvo::ENGINE_VOICE, pvo::SHIELD_VOICE, pvo::TRACTOR_VOICE, pvo::LIGHT] { seti(w, id, o, -1); }
    let f = p16(w, id, pvo::FULL) as f32;
    set(w, id, pvo::HEALTH, f);
    set16(w, id, pvo::TAUNT_FLAGS, 0);
    setu8(w, id, pvo::ALMOST, 0);
    seti(w, id, pvo::LINES_USED, 0);
    if pi(w, id, pvo::METER) != -1 { release_meter(w, id); }
    let m = w.mm(id);
    m.has_collision = false;
    m.mode |= mode::HIDDEN | mode::NO_ANIM;
    true
}

/// The tail of `0x2eb098` (module doc).
fn tail(w: &mut World, id: MobyId) {
    let p = w.m(id).position;
    let cl = |x: f32| x.clamp(22.0, 1001.0);
    w.mm(id).position = [cl(p[0]), cl(p[1]), cl(p[2]), p[3]];
    if w.m(id).cmd == 4 {
        match link(w, id, pvo::SHIELD) {
            None => {
                let s = super::qwark_ship_parts::spawn_shield(w, id);
                set_link(w, id, pvo::SHIELD, s);
            }
            Some(s) => if 2 < w.m(s).state { w.mm(s).state = 1; },
        }
        keep_voice(w, id, pvo::SHIELD_VOICE, 6);
    } else if pi(w, id, pvo::SHIELD) != 0 {
        release_voice(w, id, pvo::SHIELD_VOICE);
        match link(w, id, pvo::SHIELD) {
            Some(s) if w.m(s).o_class == SHIELD && !gone(w, s) => if w.m(s).state < 3 { w.mm(s).state = 3; },
            _ => seti(w, id, pvo::SHIELD, 0),
        }
    }
    let st = w.m(id).state;
    if st < 2 {
        setu8(w, id, pvo::F10F, 0);
    } else {
        attach(w, id);
        if st != 8 || w.ticks(0x78) < pi(w, id, pvo::TIMER) { smoke(w, id); }
        keep_voice(w, id, pvo::ENGINE_VOICE, 0);
        let m = w.m(id);
        let back = set_len3([m.rows[0][0], m.rows[0][1], m.rows[0][2], 0.0], 7.0);
        let at = sub(m.position, back);
        let slot = pi(w, id, pvo::LIGHT);
        let l = crate::point_lights::PointLight { color: [3.0, 0.5, 0.01], intensity: 0.0, pos: [at[0], at[1], at[2]], radius: 50.0 };
        match usize::try_from(slot) {
            Ok(i) => {
                let old = w.svc.point_lights.slots.get(i).copied().flatten();
                let l = old.map_or(l, |o| crate::point_lights::PointLight { pos: l.pos, radius: 50.0, ..o });
                w.svc.point_lights.set(i, l);
            }
            Err(_) => {
                let load = f32::from_bits(w.svc.frame_load[1].0);
                let got = w.svc.point_lights.alloc(l, load).map_or(-1, |i| i as i32);
                seti(w, id, pvo::LIGHT, got);
            }
        }
        setu8(w, id, pvo::F10F, 0);
    }
    if w.m(id).state != 6 {
        let s = story::link(w, pi(w, id, pvo::SHIP));
        let ok = s.is_some_and(|s| w.m(s).o_class == SHIP_CLASS && !gone(w, s) && w.m(s).state != 7);
        if !ok {
            w.mm(id).state = 6;
            let k = if win_taunt(w, id) { 1 } else { 0 };
            set16(w, id, pvo::TAUNT_FLAGS, k);
        }
    }
    if pu8(w, id, pvo::TAUNTING) == 0 {
        if w.m(id).state != 6 && w.hero.state == 0x32 {
            if let Some(v) = ship(w) {
                story::pvars(w, v, super::gemlik_ship::pvo::LEN);
                let a = c::pi32(w, v, super::gemlik_ship::pvo::ATTACKER);
                if (a.wrapping_sub(1) as u32) < 9999 && w.rng.randi(2) == 0 { taunt(w, id, 3, 9); }
            }
        }
    } else if w.svc.help.voice.state == 3 {
        w.svc.help.continue_stream();
        let (a, b) = (w.ticks(0x168) as i16, w.ticks(0x3c) as i16);
        set16(w, id, pvo::PAUSE_A, a);
        set16(w, id, pvo::PAUSE_B, b);
        setu8(w, id, pvo::TAUNTING, 0);
    }
    c::dec_timer_pvar_s16(w, id, pvo::PAUSE_A);
    c::dec_timer_pvar_s16(w, id, pvo::PAUSE_B);
    if (2..6).contains(&w.m(id).state) && w.svc.vehicle.quit & 1 != 0 { w.mm(id).state = 9; }
}

/// State 5: the tractor beam (module doc).
fn tractor(w: &mut World, id: MobyId) {
    meter(w, id);
    attack(w, id);
    let j = w.joint_point(id, 0x13);
    let end = c::pv4(w, id, pvo::BEAM_END);
    let hp = hero(w);
    let d = dist3(end, hp);
    keep_voice(w, id, pvo::TRACTOR_VOICE, 4);
    if pu8(w, id, pvo::CAUGHT) == 0 {
        hold_ship(w, 0);
        let s = pf(w, id, pvo::SPEED);
        set(w, id, pvo::SPEED, s + (DT * 30.0 - s) * 0.1);
        if DT * 40.0 < d {
            let step = set_len3(sub(hp, end), DT * 40.0);
            c::set_pv4(w, id, pvo::BEAM_END, add(end, step));
        } else {
            let t = w.ticks(0x78);
            seti(w, id, pvo::TIMER, t);
            if p16(w, id, pvo::TAUNT_FLAGS) & 4 == 0 && taunt(w, id, 0x17, 4) {
                let f = p16(w, id, pvo::TAUNT_FLAGS) | 4;
                set16(w, id, pvo::TAUNT_FLAGS, f);
            }
            setu8(w, id, pvo::CAUGHT, 1);
        }
        let sel = pu8(w, id, pvo::PATH_SEL) as usize;
        let r = steer_follow_bits(w, id, sel);
        if r & 2 != 0 && sel != PATH_BY_PHASE[(w.m(id).cmd as usize).min(11)] { take_phase_path(w, id); }
        let end = c::pv4(w, id, pvo::BEAM_END);
        let from = add(set_len3(sub(end, j), 8.0), j);
        if let Some(h) = w.coll_line(crate::moby_update::services::pv(from), crate::moby_update::services::pv(end), 0, Some(id)) {
            if h.moby.is_none_or(|m| w.m(m).o_class == 0xd4) { seti(w, id, pvo::TRACTOR_T, 0); }
        }
        if !fx::in_view(w, 100.0, [end[0], end[1], end[2], 0.0], 1.0) && w.m(id).visible == 0 { seti(w, id, pvo::TRACTOR_T, 0); }
        if 100.0 < dist3(from, hp) { seti(w, id, pvo::TRACTOR_T, 0); }
    } else {
        let p = w.m(id).position;
        let yaw = atan(hp[0] - p[0], hp[1] - p[1]);
        let pitch = atan(dist2(p, hp), hp[2] - p[2]);
        turn_to(w, id, yaw, -pitch);
        hold_ship(w, 1);
        let s = pf(w, id, pvo::SPEED) * 0.8;
        set(w, id, pvo::SPEED, s);
        let j = w.joint_point(id, 0x13);
        let end = c::pv4(w, id, pvo::BEAM_END);
        let hold = add(set_len3(sub(end, j), 30.0), j);
        let gap = sub(hold, end);
        let dd = dist3(hold, hp);
        if dd != 30.0 {
            let step = if dd <= DT * 15.0 { dd } else { DT * 15.0 };
            c::set_pv4(w, id, pvo::BEAM_END, add(end, set_len3(gap, step)));
        }
        if let Some(v) = w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len()) {
            use super::gemlik_ship::pvo as sp;
            story::pvars(w, v, sp::LEN);
            let end = c::pv4(w, id, pvo::BEAM_END);
            w.mm(v).position = [end[0], end[1], end[2], end[3]];
            let vp = w.m(v).position;
            let q = w.m(id).position;
            let dy = sub_rot(atan(q[0] - vp[0], q[1] - vp[1]), c::pf(w, v, sp::YAW));
            let ay = dy.abs();
            let dp = sub_rot(-atan(dist2(vp, q), q[2] - vp[2]), c::pf(w, v, sp::PITCH));
            let ap = dp.abs();
            const LIMIT: f32 = 0.383_972_44;
            const TEN: f32 = 0.174_532_92;
            if LIMIT < ay {
                let y = add_rot(c::pf(w, v, sp::YAW), crate::moby_update::classes::flyer::wrap_frac(((ay - LIMIT) / TEN) * dy));
                c::set_pf(w, v, sp::YAW, y);
            }
            if LIMIT < ap {
                let y = add_rot(c::pf(w, v, sp::PITCH), crate::moby_update::classes::flyer::wrap_frac(((ap - LIMIT) / TEN) * dp));
                c::set_pf(w, v, sp::PITCH, y);
            }
        }
    }
    if w.m(id).cmd == 4 { c::flash::update(w, id, pvo::FLASH); } else { hits(w, id); }
    let p = w.m(id).position;
    if diff_rots(w.m(id).rotation[2], atan(hp[0] - p[0], hp[1] - p[1])) < std::f32::consts::FRAC_PI_3 {
        if let Some(i) = row(BEAM_FN) {
            let j = w.joint_point(id, 0x13);
            c::set_pv4(w, id, pvo::BEAM_JOINT, j);
            seti(w, id, pvo::BEAM_SCROLL, (w.counter % 20) as i32);
            w.svc.draw_callbacks.register(Callback::UnitQuads(i), id);
        }
    }
}

/// `0x2e9160(sel, 1)` with the phase-path bit of state 5's call (`0x2e9160` returns the wrap only; the w test is the
/// caller's own).
fn steer_follow_bits(w: &mut World, id: MobyId, sel: usize) -> u32 {
    let wrapped = follow(w, id, sel, true) as u32;
    let p = path(w, id, sel);
    if 0.0 < point(&p, p16(w, id, pvo::AHEAD) as i32)[3] { wrapped | 2 } else { wrapped }
}

/// State 7: the crash (module doc).
fn crash(w: &mut World, id: MobyId) {
    hold_ship(w, 0);
    if pi(w, id, pvo::TIMER) == 1 {
        lose_part(w, id, 8);
        w.play_sound(7, 0, id);
    }
    c::dec_timer_pvar_i32(w, id, pvo::TIMER);
    let sel = pu8(w, id, pvo::PATH_SEL) as usize;
    let done = follow(w, id, sel, false);
    let p = path(w, id, sel);
    let count = p.len() as i32;
    if done || count - 1 <= p16(w, id, pvo::AHEAD) as i32 {
        let t = w.ticks(0xb4);
        seti(w, id, pvo::TIMER, t);
        w.mm(id).state = 8;
    }
    let r = add_rot(w.m(id).rotation[0], DT * 4.188_790_3);
    w.mm(id).rotation[0] = r;
    let a = p16(w, id, pvo::AHEAD) as i32;
    let b = (a + 3).min(count - 1);
    let (pa, pb) = (point(&p, a), point(&p, b));
    let yaw = atan(pb[0] - pa[0], pb[1] - pa[1]);
    let pitch = atan(dist2(pa, pb), pb[2] - pa[2]);
    c::turn::spring_turn2_pvar(w, id, yaw, DT2 * PI, DT2 * 2.0 * PI, DT * 2.0 * PI, pvo::YAW_V);
    let mut v = pf(w, id, pvo::PITCH_V);
    let ry = c::turn::spring_turn(w.m(id).rotation[1], -pitch, DT2 * PI, DT2 * 2.0 * PI, DT * 2.0 * PI, &mut v);
    set(w, id, pvo::PITCH_V, v);
    {
        let m = w.mm(id);
        m.rotation[1] = ry;
        m.scale *= 0.999_25;
    }
    let s = pf(w, id, pvo::SPEED) * 0.9992;
    set(w, id, pvo::SPEED, s);
    if w.rng.randi(0x11) == 0 {
        let k = w.m(id).scale / (super::class_scale(w, CLASS) * 3.0);
        let rows = w.m(id).rows;
        let mut q = w.m(id).position;
        for (r, lim) in [(0usize, 5.0f32), (1, 4.0), (2, 4.0)] {
            let f = w.rng.randf(-lim, lim);
            let v = set_len3([rows[r][0], rows[r][1], rows[r][2], 0.0], f * k);
            q = add(q, v);
        }
        fx::death_explosion(w, k * 1.6, 13.0, Some(id), q, -1);
    }
    crash_smoke(w, id);
    if !w.svc.help.voice.busy && w.svc.help.voice.request == -1 {
        let t = pi(w, id, pvo::TIMER);
        let used = pi(w, id, pvo::LINES_USED);
        if t < w.ticks(0x1a4) && used < 0 {
            if taunt(w, id, 0x23, 1) { seti(w, id, pvo::LINES_USED, used & 0x7fff_ffff); }
        } else if t < w.ticks(0xb4) && used & 0x4000_0000 != 0 && taunt(w, id, 0x24, 1) {
            let u = pi(w, id, pvo::LINES_USED) & !0x4000_0000;
            seti(w, id, pvo::LINES_USED, u);
        }
    }
}

/// State 9: the quit (module doc).
fn reset(w: &mut World, id: MobyId) {
    for o in [pvo::HIT_SHIELD, pvo::SHIELD] {
        if let Some(m) = link(w, id, o) { w.delete_moby(m); }
        seti(w, id, o, 0);
    }
    for i in 0..10 {
        if let Some(m) = link(w, id, pvo::PARTS + 4 * i) { w.delete_moby(m); }
        seti(w, id, pvo::PARTS + 4 * i, 0);
    }
    for o in [pvo::SHIELD_VOICE, pvo::ENGINE_VOICE, pvo::TRACTOR_VOICE] { release_voice(w, id, o); }
    let l = pi(w, id, pvo::LIGHT);
    if let Ok(i) = usize::try_from(l) { w.svc.point_lights.free(i); }
    seti(w, id, pvo::LIGHT, -1);
    release_meter(w, id);
    let p = path(w, id, 0);
    let (p0, p1) = (point(&p, 0), point(&p, 1));
    {
        let m = w.mm(id);
        m.position = p0;
        m.rotation[2] = atan(p1[0] - p0[0], p1[1] - p0[1]);
    }
    setu8(w, id, pvo::DIR, 1);
    setu8(w, id, pvo::ALMOST, 0);
    for o in [pvo::AHEAD, pvo::BEHIND] { set16(w, id, o, 0); }
    for o in [0xa0, pvo::PITCH_V, pvo::YAW_V, pvo::SPEED, 0xf4, pvo::TIMER, pvo::TRACTOR_T, 0x124, pvo::LINES_USED] { seti(w, id, o, 0); }
    for o in [pvo::GUN_SIDE, pvo::COUNT, pvo::WEAPON, pvo::CAUGHT, 0x10d, pvo::SHIELD_START, pvo::F10F, pvo::TAUNTING, pvo::PATH_SEL] { setu8(w, id, o, 0); }
    for o in [pvo::TAUNT_FLAGS, pvo::PAUSE_A, pvo::PAUSE_B] { set16(w, id, o, 0); }
    w.mm(id).cmd = 0;
    c::set_pv4(w, id, pvo::VEL, [0.0; 4]);
    c::set_pv4(w, id, pvo::BEAM_END, [0.0; 4]);
    w.mm(id).state = 0;
}

/// Level13 `0x2e9018(yaw, pitch)` (module doc).
fn turn_to(w: &mut World, id: MobyId, yaw: f32, pitch: f32) {
    c::turn::spring_turn2_pvar(w, id, yaw, DT2 * PI, DT2 * 2.0 * PI, DT * 2.0 * PI, pvo::YAW_V);
    let mut v = pf(w, id, pvo::PITCH_V);
    let ry = c::turn::spring_turn(w.m(id).rotation[1], pitch, DT2 * PI, DT2 * 2.0 * PI, DT * 2.0 * PI, &mut v);
    set(w, id, pvo::PITCH_V, v);
    let yv = pf(w, id, pvo::YAW_V);
    let m = w.mm(id);
    m.rotation[1] = ry;
    let rx = m.rotation[0];
    let mut x = rx + -yv * ((HALF_PI - rx.abs()) / HALF_PI) * 0.5;
    x = x.clamp(-HALF_PI, HALF_PI);
    m.rotation[0] = x * 0.985;
}

/// Level13 `0x2e9160(sel, turn)`: one step along the path; true when it wrapped (module doc).
fn follow(w: &mut World, id: MobyId, sel: usize, turn: bool) -> bool {
    let p = path(w, id, sel);
    let count = p.len() as i32;
    if count == 0 { return false; }
    let pos = w.m(id).position;
    let (a, b) = (p16(w, id, pvo::AHEAD) as i32, p16(w, id, pvo::BEHIND) as i32);
    let d_me = dist3(point(&p, b), pos);
    let d_seg = dist3(point(&p, b), point(&p, a));
    let speed = pf(w, id, pvo::SPEED);
    let mut wrapped = false;
    let mut f = d_me / d_seg;
    if 0.0 <= d_seg - d_me && d_seg - d_me < speed {
        set16(w, id, pvo::BEHIND, a as i16);
        let mut n = (a + c::pu8(w, id, pvo::DIR) as i8 as i32) as i16;
        wrapped = count <= n as i32;
        if wrapped { n = 0; }
        f = 0.0;
        if n < 0 {
            wrapped = true;
            n = (count - 1) as i16;
        }
        set16(w, id, pvo::AHEAD, n);
    }
    let f = f.clamp(0.05, 1.0);
    let ahead = point(&p, p16(w, id, pvo::AHEAD) as i32);
    let vel = c::scale(c::pv4(w, id, pvo::VEL), 1.0 - f);
    let to = c::scale(set_len3(sub(ahead, pos), speed), f);
    let vel = set_len3(add(vel, to), speed);
    c::set_pv4(w, id, pvo::VEL, vel);
    let np = add(pos, vel);
    w.mm(id).position = np;
    let (mut yaw, mut pitch) = (atan(to[0], to[1]), atan((to[0] * to[0] + to[1] * to[1]).sqrt(), to[2]));
    if w.m(id).state == 5 {
        let hp = hero(w);
        yaw = atan(hp[0] - np[0], hp[1] - np[1]);
        pitch = atan(dist2(np, hp), hp[2] - np[2]);
    }
    if turn { turn_to(w, id, yaw, -pitch); }
    wrapped
}

/// Level13 `0x2e9408(sel, turn)` (module doc).
fn steer(w: &mut World, id: MobyId, sel: usize, turn: bool) -> u32 {
    let p = path(w, id, sel);
    let count = p.len() as i32;
    let mut r = 0;
    if count != 0 {
        let a = p16(w, id, pvo::AHEAD) as i32;
        let n = if count <= a + 1 { 0 } else { a + 1 };
        let hp = hero(w);
        let d_next = dist3(point(&p, n), hp);
        let pv_ = if a - 1 < 0 { count - 1 } else { a - 1 };
        let d_prev = dist3(point(&p, pv_), hp);
        let dir = c::pu8(w, id, pvo::DIR) as i8;
        let want: i8 = if d_next < d_prev { -1 } else if d_prev < d_next || dir == 1 { 1 } else { -1 };
        if want == dir {
            let k = if w.m(id).cmd == 4 { 3.0 } else { 1.0 };
            let d = dist3(w.m(id).position, hp);
            let s = pf(w, id, pvo::SPEED);
            let ns = s + ((k * 30.0) / d - s) * SPEED * 0.12;
            set(w, id, pvo::SPEED, if ns < 0.0 { 0.0 } else { ns });
        } else {
            let s = pf(w, id, pvo::SPEED) * (SPEED * -0.100_000_024 + 1.0);
            set(w, id, pvo::SPEED, s);
            if s < SPEED * 0.15 {
                let (x, y) = (p16(w, id, pvo::AHEAD), p16(w, id, pvo::BEHIND));
                setu8(w, id, pvo::DIR, want as u8);
                set16(w, id, pvo::AHEAD, y);
                set16(w, id, pvo::BEHIND, x);
            }
        }
        r = follow(w, id, sel, turn) as u32;
    }
    if 0.0 < point(&p, p16(w, id, pvo::AHEAD) as i32)[3] { r |= 2; }
    r
}

/// Level13 `0x2e9680(k)` (module doc).
fn lose_part(w: &mut World, id: MobyId, k: usize) {
    let Some(m) = link(w, id, pvo::PARTS + 4 * k) else { return };
    let pos = w.m(m).position;
    fx::death_explosion(w, 2.0, 13.0, Some(id), pos, -1);
    use super::qwark_ship_parts::part_pvo as pp;
    story::pvars(w, m, pp::LEN);
    let v = c::scale(c::pv4(w, id, pvo::VEL), f32::from_bits(0x3f73_3333));
    let mut v = [v[0], v[1], v[2], v[3]];
    fx::jitter(w, DT * 30.0, &mut v);
    c::set_pv4(w, m, pp::VEL, v);
    w.mm(m).state = 3;
    let p = add(w.m(m).position, v);
    w.mm(m).position = p;
    let a = w.rng.randf(-(DT * 0.349_065_84), DT * 0.349_065_84);
    c::set_pf(w, m, pp::SPIN, a);
    let b = w.rng.randf(-(DT * PI), DT * PI);
    c::set_pf(w, m, pp::SPIN + 4, b);
    let d = w.rng.randf(-(DT * PI), DT * PI);
    c::set_pf(w, m, pp::SPIN + 8, d);
    {
        let mo = w.mm(m);
        mo.light = 0;
        mo.ambient = [0x40, 0x40, 0x40, 0];
        mo.hit_slot = 0xff;
    }
    seti(w, id, pvo::PARTS + 4 * k, 0);
}

/// Level13 `0x2e97e0`: Qwark shot down (module doc).
fn die(w: &mut World, id: MobyId) {
    set(w, id, pvo::HEALTH, 0.0);
    w.mm(id).mode &= !mode::TARGETABLE;
    crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
    setu8(w, id, pvo::FLASH_RED, 0x78);
    c::flash::start(w, id, pvo::FLASH);
    setu8(w, id, pvo::PATH_SEL, 4);
    set16(w, id, pvo::AHEAD, 0);
    let dir = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, pi(w, id, pvo::CRASH_CUBOID)).map_or([0.0; 4], |s| s.matrix[0]);
    c::set_pv4(w, id, pvo::VEL, set_len3(dir, DT * 10.0));
    let p = path(w, id, 4);
    let (p0, p1) = (point(&p, 0), point(&p, 1));
    {
        let m = w.mm(id);
        m.position = p0;
        m.rotation[2] = atan(p1[0] - p0[0], p1[1] - p0[1]);
        m.rotation[1] = -atan(dist2(p0, p1), p1[2] - p0[2]);
    }
    setu8(w, id, pvo::DIR, 1);
    if let Some(v) = story::link(w, pi(w, id, pvo::SHIP)) {
        let m = w.mm(v);
        m.mode |= mode::NO_UPDATE | mode::HIDDEN;
        m.visible = 0;
    }
    // 0x316f20: the crash camera armed (camera class 19).
    let rec = pi(w, id, pvo::CRASH_CAMERA);
    crate::cinematic::flyby_arm(w, rec);
    w.mm(id).state = 7;
    set(w, id, pvo::SPEED, DT * 80.0);
    let t = w.ticks(600);
    seti(w, id, pvo::TIMER, t);
    taunt(w, id, 0x22, 1);
    set16(w, id, pvo::TAUNT_FLAGS, 0);
    setu8(w, id, pvo::ALMOST, 0);
    release_meter(w, id);
    seti(w, id, pvo::LINES_USED, -1);
}

/// Level13 `0x2e9a48` (module doc).
fn hits(w: &mut World, id: MobyId) {
    if 6 <= w.m(id).state {
        c::flash::update(w, id, pvo::FLASH);
        return;
    }
    let full = p16(w, id, pvo::FULL) as f32;
    let sixth = full / 6.0;
    let ph = w.m(id).cmd;
    let k = if 4 < ph { 6.0 - (ph - 1) as f32 } else { 6.0 - ph as f32 };
    let lo = k * sixth;
    let hi = lo + sixth * 0.3;
    let mut result = 0;
    let mut phase_change = false;
    // The hit shield first, then the ten parts and Qwark himself (slot 10 is +0xec).
    let sources: Vec<(usize, bool)> = std::iter::once((pvo::HIT_SHIELD, true)).chain((0..11).map(|i| (pvo::PARTS + 4 * i, false))).collect();
    for (o, shield) in sources {
        if result != 0 { break; }
        let Some(s) = link(w, id, o) else { continue };
        let hit = w.get_hit(s, 0x23_0000, false);
        let r = c::damage::resolve(w, id, hit, pvo::HEALTH, 0, 4);
        let rec = r.hit.or(hit);
        if let Some(h) = rec {
            if let Some(a) = h.attacker {
                if OWN_SHOTS.contains(&w.m(a).o_class) {
                    w.mm(s).hit_slot = 0xff;
                    continue;
                }
            }
        }
        if r.out5 <= 1 { continue; }
        let mut dmg = rec.map_or(0.0, |h| crate::moby_update::services::fl(h.damage));
        if let Some(h) = rec {
            if 0.0 < dmg {
                if shield {
                    if h.h2a == 0x3f1 { dmg = 0.3; }
                } else if h.h2a == 0x127 {
                    dmg *= 0.5;
                } else if h.h2a == 0x3f1 {
                    dmg = 0.1;
                }
            }
            if ph == 7 { dmg *= 1.5; }
        }
        let health = pf(w, id, pvo::HEALTH);
        if health <= dmg {
            die(w, id);
            result = 3;
        } else {
            result = 1;
            set(w, id, pvo::HEALTH, health - dmg);
            setu8(w, id, pvo::FLASH_RED, 0xfa);
            set16(w, id, pvo::OWN_COOLDOWN, 0);
            c::flash::start(w, id, pvo::FLASH);
        }
        w.mm(s).hit_slot = 0xff;
        let health = pf(w, id, pvo::HEALTH);
        if w.m(id).state == 7 {
            phase_change = true;
            w.mm(id).cmd = w.m(id).cmd.wrapping_add(1);
            clear_ship_lock(w);
        } else if lo <= health {
            if health < hi { setu8(w, id, pvo::ALMOST, 1); }
        } else {
            let part = PART_BY_PHASE[(w.m(id).cmd as usize).saturating_sub(1).min(5)];
            if shield {
                if part != -1 {
                    w.play_sound(7, 0, id);
                    lose_part(w, id, part as usize);
                }
            } else {
                if part != -1 { lose_part(w, id, part as usize); }
                w.play_sound(7, 0, id);
            }
            if w.m(id).state == 5 { w.mm(id).state = 4; }
            w.mm(id).cmd = w.m(id).cmd.wrapping_add(1);
            release_voice(w, id, pvo::TRACTOR_VOICE);
            phase_change = true;
            clear_ship_lock(w);
            set16(w, id, pvo::TAUNT_FLAGS, 0);
            setu8(w, id, pvo::ALMOST, 0);
            result = 2;
            if w.m(id).cmd == 7 {
                for &p in &LAST_PARTS { lose_part(w, id, p); }
                w.play_sound(7, 0, id);
            }
        }
    }
    if result != 0 {
        if phase_change {
            match w.m(id).cmd {
                2 => { taunt(w, id, 0xc, 4); }
                3 => {
                    let t = w.ticks(0x78);
                    seti(w, id, pvo::TRACTOR_T, t);
                    taunt(w, id, 0x10, 1);
                }
                4 => { taunt(w, id, 0x11, 1); }
                6 => { taunt(w, id, 0x13, 1); }
                7 => { taunt(w, id, 0x16, 1); }
                _ => {}
            }
        } else if w.m(id).cmd == 7 && pu8(w, id, pvo::ALMOST) != 0 {
            if p16(w, id, pvo::TAUNT_FLAGS) & 1 == 0 && taunt(w, id, 0x14, 2) {
                let f = p16(w, id, pvo::TAUNT_FLAGS) | 1;
                set16(w, id, pvo::TAUNT_FLAGS, f);
            }
            if p16(w, id, pvo::TAUNT_FLAGS) & 2 == 0 && taunt(w, id, 0x14, 2) {
                let f = p16(w, id, pvo::TAUNT_FLAGS) | 2;
                set16(w, id, pvo::TAUNT_FLAGS, f);
            }
        }
    }
    c::flash::update(w, id, pvo::FLASH);
}

/// Level13 `0x2e8d28` (module doc).
fn attach(w: &mut World, id: MobyId) {
    let (rot, rows, pos) = { let m = w.m(id); (m.rotation, m.rows, m.position) };
    let cl = |x: f32| x.clamp(22.0, 1001.0);
    let place = |w: &mut World, m: MobyId, joint: usize| {
        let p = w.joint_point(id, joint);
        let mo = w.mm(m);
        mo.position = [cl(p[0]), cl(p[1]), cl(p[2]), p[3]];
        mo.rotation = rot;
        mo.rows[0] = rows[0];
        mo.rows[1] = rows[1];
        mo.rows[2] = rows[2];
    };
    for i in 0..10 {
        if let Some(m) = link(w, id, pvo::PARTS + 4 * i) { place(w, m, i); }
    }
    let mut joint = SHIELD_JOINT[(w.m(id).cmd as usize).min(11)];
    if w.m(id).state == 5 { joint = -1; }
    let mut skip_shield = false;
    if joint == -1 {
        if let Some(s) = link(w, id, pvo::HIT_SHIELD) {
            match w.m(s).state {
                0xfe => {}
                0xfd => skip_shield = true,
                _ => {
                    w.delete_moby(s);
                    seti(w, id, pvo::HIT_SHIELD, 0);
                }
            }
        }
    } else {
        let s = match link(w, id, pvo::HIT_SHIELD) {
            Some(s) if !gone(w, s) => Some(s),
            _ => {
                let s = super::qwark_ship_parts::spawn_part(w, id, joint as usize, HIT_SHIELD);
                set_link(w, id, pvo::HIT_SHIELD, s);
                s
            }
        };
        if let Some(s) = s.filter(|&s| !gone(w, s)) { place(w, s, joint as usize); }
    }
    let _ = skip_shield;
    if let Some(s) = link(w, id, pvo::SHIELD) {
        let mo = w.mm(s);
        mo.position = pos;
        mo.rotation = rot;
        mo.rows[0] = rows[0];
        mo.rows[1] = rows[1];
        mo.rows[2] = rows[2];
    }
}

fn puff04(w: &mut World, pos: [f32; 4], c1: u32, c2: u32, life: i32, base: i32, growth: i32) {
    let a = crate::particles::type04::Spawn { pos, vel: [0.0; 4], c1, c2, life, base: base as i16, growth: growth as i16, additive: true };
    fx::part04(w, &a);
}

/// Level13 `0x2e8920` (module doc).
fn smoke(w: &mut World, id: MobyId) {
    for (&slot, &joint) in SMOKE_PARTS.iter().zip(&SMOKE_JOINTS) {
        if pi(w, id, pvo::PARTS + 4 * slot) == 0 { continue; }
        let s = w.m(id).scale / (super::class_scale(w, CLASS) * 4.0);
        let p = w.joint_point(id, joint);
        if w.counter & 1 == 0 {
            let ph = w.m(id).cmd;
            let own = ph != 0 && PART_BY_PHASE.get(ph as usize - 1).is_some_and(|&k| k == slot as i32);
            if !own || w.rng.randi(9) == 0 {
                let n = w.rng.rand_range(0xf, 0x16);
                let life = w.ticks(n);
                puff04(w, p, 0x6000_ffff, 0x80, life, (s * 600.0) as i32, -100);
            }
        } else {
            let n = w.rng.rand_range(0xf, 0x14);
            let life = w.ticks(n);
            puff04(w, p, 0xcf00_00ff, 0xcf, life, (s * 350.0) as i32, -100);
        }
        let f = w.rng.randf(100.0, 250.0);
        let b = (f * s) as i32;
        let n = w.rng.rand_range(8, 0xc);
        let life = w.ticks(n);
        puff04(w, p, 0xefff_7f4f, 0xff_0000, life, b, -b);
    }
}

/// Level13 `0x2e8b58` (module doc).
fn crash_smoke(w: &mut World, id: MobyId) {
    for &joint in &CRASH_JOINTS {
        let f = w.rng.randf(0.8, 1.1);
        let s = (w.m(id).scale / (super::class_scale(w, CLASS) * 4.0)) * f;
        let mut p = w.joint_point(id, joint);
        fx::jitter(w, s * 1.4, &mut p);
        if w.counter & 1 == 0 {
            if w.m(id).cmd == 0 || w.rng.randi(0xc) == 0 {
                let n = w.rng.rand_range(0x14, 0x32);
                let life = w.ticks(n);
                let g = w.rng.randf(600.0, 800.0);
                puff04(w, p, 0x8000_ffff, 0x80, life, (g * s) as i32, -100);
            }
        } else {
            let n = w.rng.rand_range(0x14, 0x3c);
            let life = w.ticks(n);
            let g = w.rng.randf(450.0, 550.0);
            puff04(w, p, 0xdf00_0fff, 0xcf, life, (g * s) as i32, -100);
        }
    }
}

/// `FastDecTimer__FRc` on a pvar byte: 1 when it is 0, else − 1 and 2 when that is 0.
fn dec_u8(w: &mut World, id: MobyId, o: usize) -> i32 {
    let t = pu8(w, id, o);
    if t == 0 { return 1; }
    setu8(w, id, o, t - 1);
    if t - 1 == 0 { 2 } else { 0 }
}

/// The weapon swap of `0x2eac00`: missiles (`rand_range(4, 6)`, state 4) or mines (`rand_range(20, 50)`, state 3).
fn swap_weapon(w: &mut World, id: MobyId) {
    let wpn = pu8(w, id, pvo::WEAPON) ^ 1;
    setu8(w, id, pvo::WEAPON, wpn);
    if wpn == 0 {
        let n = w.rng.rand_range(4, 6) as u8;
        setu8(w, id, pvo::COUNT, n);
        w.mm(id).state = 4;
    } else {
        let n = w.rng.rand_range(0x14, 0x32) as u8;
        setu8(w, id, pvo::COUNT, n);
        w.mm(id).state = 3;
    }
}

/// Level13 `0x2eac00` (module doc).
fn attack(w: &mut World, id: MobyId) {
    let set_timer = |w: &mut World, n: i32| { let t = w.ticks(n); seti(w, id, pvo::TIMER, t); };
    match w.m(id).cmd {
        1 => {
            if w.m(id).state != 4 {
                if c::dec_timer_pvar_i32(w, id, pvo::TIMER) == 0 { return; }
                if p16(w, id, pvo::TAUNT_FLAGS) & 1 == 0 && taunt(w, id, 1, 2) {
                    let f = p16(w, id, pvo::TAUNT_FLAGS) | 1;
                    set16(w, id, pvo::TAUNT_FLAGS, f);
                }
                set_timer(w, 0x1e);
                w.mm(id).state = 4;
                return;
            }
            if c::dec_timer_pvar_i32(w, id, pvo::TIMER) == 0 { return; }
            fire_missile(w, id);
            set_timer(w, 0x3c);
            let n = pu8(w, id, pvo::COUNT).wrapping_add(1);
            setu8(w, id, pvo::COUNT, n);
            if n < 0xb { return; }
            setu8(w, id, pvo::COUNT, 0);
            w.mm(id).state = 3;
            set_timer(w, 300);
        }
        2 | 7 => {
            if c::dec_timer_pvar_i32(w, id, pvo::TIMER) == 0 { return; }
            if w.m(id).cmd == 2 {
                if dec_u8(w, id, pvo::COUNT) == 0 {
                    if pu8(w, id, pvo::WEAPON) == 0 {
                        fire_missile(w, id);
                        set_timer(w, 0x3c);
                    } else {
                        fire_mine(w, id);
                        set_timer(w, 0x14);
                    }
                } else {
                    swap_weapon(w, id);
                    set_timer(w, 0x3c);
                }
            } else {
                w.mm(id).state = 3;
                fire_mine(w, id);
                set_timer(w, 0x14);
            }
        }
        3 | 5 | 6 => {
            c::dec_timer_pvar_i32(w, id, pvo::T124);
            c::dec_timer_pvar_i32(w, id, pvo::TRACTOR_T);
            c::dec_timer_pvar_i32(w, id, pvo::TIMER);
            let after = |w: &mut World, n: i32| { let t = w.ticks(n); seti(w, id, pvo::T124, t); };
            if w.m(id).state == 5 {
                if pi(w, id, pvo::TRACTOR_T) == 0 {
                    if pi(w, id, pvo::T124) != 0 { return; }
                    let t = w.ticks(600);
                    seti(w, id, pvo::TRACTOR_T, t);
                    set_timer(w, 0x78);
                    w.mm(id).state = 4;
                    w.play_sound(5, 0, id);
                    return;
                }
                if !(pi(w, id, pvo::TIMER) == 0 && pu8(w, id, pvo::CAUGHT) == 1) { return; }
                fire_missile(w, id);
                set_timer(w, 0x3c);
                after(w, 0x78);
                return;
            }
            release_voice(w, id, pvo::TRACTOR_VOICE);
            let j = w.joint_point(id, 0x13);
            let d = dist3(j, hero(w));
            if pi(w, id, pvo::TRACTOR_T) == 0 && d < 100.0 && 60.0 < d {
                if pi(w, id, pvo::TIMER) != 0 || pi(w, id, pvo::T124) != 0 { return; }
                let t = w.ticks(600);
                seti(w, id, pvo::TRACTOR_T, t);
                c::set_pv4(w, id, pvo::BEAM_END, j);
                setu8(w, id, pvo::CAUGHT, 0);
                w.mm(id).state = 5;
                return;
            }
            if w.m(id).cmd == 3 {
                w.mm(id).state = 4;
                if pi(w, id, pvo::TIMER) != 0 { return; }
                fire_missile(w, id);
                set_timer(w, 0x3c);
                after(w, 0x78);
                return;
            }
            if pi(w, id, pvo::TIMER) != 0 { return; }
            if dec_u8(w, id, pvo::COUNT) != 0 && pi(w, id, pvo::T124) == 0 {
                swap_weapon(w, id);
                set_timer(w, 0x78);
                return;
            }
            if pu8(w, id, pvo::WEAPON) == 0 {
                fire_missile(w, id);
                set_timer(w, 0x3c);
                after(w, 0x78);
            } else {
                fire_mine(w, id);
                set_timer(w, 0x14);
                after(w, 0xf0);
            }
        }
        4 => {
            w.mm(id).state = 3;
            if pu8(w, id, pvo::SHIELD_START) == 0 {
                w.mm(id).cmd = 5;
                taunt(w, id, 0x1b, 2);
                clear_ship_lock(w);
            }
        }
        _ => {}
    }
}

/// Level13 `0x2ea540` (module doc).
fn fire_missile(w: &mut World, id: MobyId) -> Option<MobyId> {
    let joint = if pu8(w, id, pvo::GUN_SIDE) & 1 != 0 { 0x12 } else { 0x10 };
    if w.m(id).cmd != 6 || w.rng.rand() & 1 != 0 {
        let s = pu8(w, id, pvo::GUN_SIDE) ^ 1;
        setu8(w, id, pvo::GUN_SIDE, s);
    }
    let mut p = w.joint_point(id, joint);
    p[3] = 0.0;
    let speed = DT * 18.0 + pf(w, id, pvo::SPEED);
    let rot = [0.0, f32::from_bits(0x3edf_66f3), w.m(id).rotation[2], 0.0];
    let life = w.ticks(300);
    let target = w.svc.vehicle.moby;
    let m = super::qwark_ship_parts::spawn_missile(w, speed, DT * 18.0, id, p, target, rot, life);
    if m.is_some() { w.play_sound(2, 0, id); }
    m
}

/// Level13 `0x2ea330(k, from)`: where the player's ship will be: its heading (yaw / pitch + the stick springs' rates)
/// at its speed, as far as the distance takes at speed + k.
pub fn lead(w: &World, k: f32, from: [f32; 4]) -> Option<[f32; 4]> {
    use super::gemlik_ship::pvo as sp;
    let v = w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len() && w.m(v).pvars.len() >= sp::LEN)?;
    let m = w.m(v);
    let yaw = add_rot(m.rotation[2], c::pf(w, v, sp::STICK_XV));
    let pitch = add_rot(m.rotation[1], c::pf(w, v, sp::STICK_YV));
    let speed = c::pf(w, v, sp::SPEED);
    let d = dist3(from, m.position);
    Some(add(polar((d / (speed + k)) * speed, yaw, -pitch), m.position))
}

/// Level13 `0x2ea6b8` (module doc).
fn fire_mine(w: &mut World, id: MobyId) -> Option<MobyId> {
    let p = w.joint_point(id, 0xe);
    let mut v = match (ship(w), lead(w, DT * 20.0, p)) {
        (Some(s), Some(t)) => {
            let mut v = set_len3(sub(t, p), DT * 20.0);
            let rows = w.m(s).rows;
            let a = w.rng.randf(-(DT * 30.0), DT * 30.0);
            let b = set_len3(rows[1], a);
            let a2 = w.rng.randf(-(DT * 30.0), DT * 30.0);
            let c2 = set_len3(rows[2], a2);
            let s2 = w.rng.randf(DT * 5.0, DT * 30.0);
            let bc = set_len3(add(b, c2), s2);
            v = add(v, bc);
            v
        }
        _ => {
            let r = w.rng.rand_vec(DT * 0.0, DT * 30.0);
            [r[0], r[1], r[2], 0.0]
        }
    };
    v = add(v, c::pv4(w, id, pvo::VEL));
    let m = super::qwark_ship_parts::spawn_mine(w, id, p, v);
    if m.is_some() { w.play_sound(3, 0, id); }
    m
}

/// `0x2ea8c8` (list 1; draw only): the tractor beam's quads (module doc).
pub fn beam_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, id: MobyId) -> Option<super::FxQuads> {
    let _ = svc;
    let m = table.mobys.get(id).filter(|m| m.o_class == CLASS && m.pvars.len() >= pvo::LEN)?;
    let start = crate::moby_update::services::pvar::v4f(&m.pvars, pvo::BEAM_END);
    let joint = crate::moby_update::services::pvar::v4f(&m.pvars, pvo::BEAM_JOINT);
    let mut d = sub(start, joint);
    let l0 = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if 100.0 < l0 { d = set_len3(d, 100.0); }
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let step = (len / 100.0).max(1.0);
    let dir = set_len3(d, step);
    let scroll = crate::moby_update::services::pvar::i32(&m.pvars, pvo::BEAM_SCROLL) as f32 / 20.0;
    let mut at = sub(joint, set_len3(dir, scroll));
    let up = [0.0, 0.0, 1.0, 0.0];
    let mut out = Vec::new();
    let mut t = 0.0f32;
    let mut rgba = 0x4080_8080u32;
    while t < len {
        let half = (t * 0.0) / (len + len) + 0.5;
        let a = set_len3(cross(dir, up), half);
        let b = set_len3(cross(dir, a), half);
        let p = sub(at, b);
        let q = add(at, b);
        let corners = [sub(p, a), add(p, a), sub(q, a), add(q, a)].map(|v| [v[0], v[1], v[2]]);
        if 10.0 < len && len - 10.0 < t { rgba = (((((len - t) * 6.4) as i32) as u32) << 24) | 0x80_8080; }
        out.push(super::FxQuad { corners, st: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], rgba: [rgba; 4] });
        t += step;
        at = add(at, dir);
    }
    Some(super::FxQuads { fx: 0x34, additive: true, subtract: false, quads: out })
}

fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0], 0.0] }
