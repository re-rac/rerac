//! **Package P5 — boots and spline riding: Magneboots, Grind Boots, the cable slide.** Owner: the P5 agent
//! (docs/plan/hero_states.md "Packages"). The grind and cable code only exists in the overlays of the levels with
//! rails, so the reference is **level00** (SetState 0x2223f8, physics 0x217970, transitions 0x229b70, the hero
//! update 0x2070d0); library functions level01 unless marked L00.
//!
//! **Grind Boots** (item 29, owned flag 0x13d4dd). The rails are the level's grind paths (gameplay section 0x74,
//! `rc_formats::volumes::GrindPath`: bounding sphere, spline, closed flag), ridden with the general spline
//! follower [`crate::spline`].
//! * **Rail contact** `0x20cf58` ([`contacts`], after the surface reaction and the wall probe, before the
//!   transitions; only in the overlays of [`RAIL_LEVELS`]): with the boots owned and 0x13f518 = 0, in groups
//!   0 / 1 / 2, a descending jump (or 0x13f51a), or a grind jump off the rail, the first path whose bounding
//!   sphere holds the feet and whose nearest point (`nearest(12, 10, band 0)`) is within 0.9 horizontally (0.3
//!   standing / walking; +0.5 with 0x13f51a) and 1.5 vertically (10), with the feet −0.5..0.57 above it, becomes
//!   the rail (0x13f850 point, 0x13f8b0 path, 0x13f8b4 / 0x13f8b8 cursor, 0x13f8d4 closed) and 0x13f8bc = 1.
//!   Levels 6, 0xe and 0x10 only take a rail within 14 / 12 of fixed mount points (the rail starts).
//! * **0x28 grind** (group 0xf, anim 0x31 / 0x32 by stance 0x13f8dc, leans 0x4a..0x4d): each tick the feet are
//!   projected onto the rail (`nearest(999, 2, band 2.5)`), the cursor advances by the speed 0x13f8c4, vel = the
//!   step, the feet are pulled onto the rail by a spring (0x13f92c, ≤ 7 u/s); facing = rail yaw − 90° (side-on).
//!   Speed = base 0x13f900 (→ 12 u/s at 15·dt²; the entry takes the displacement along the rail) + the slope term
//!   0x13f8fc (uphill −2.5 u/s per 30°, downhill +8 u/s per 20°, clamped −2.5..12 u/s). The end of an open rail
//!   (0x13f8c0 > 0) lets him fly on with his momentum; > 5 ticks → fall 6. Sparks (type 25) every tick.
//! * **0x29 grind jump** (✕; the jump system, h 2.4 (+4.7 on a booster floor 0x140638, +0.7 more near one Kalebo
//!   booster), g 27·dt², anim 0x50) and **0x2a rail switch** (✕ + stick sideways in the first 10 ticks of 0x29:
//!   a rail 2..4.5 to that side; the position blends from the old rail to the new over 48 ticks, anim 0x1e / 0x1f
//!   by side; h 2.5, g 24·dt²). While in the air above the rail he keeps riding it horizontally; the landing
//!   (feet at the rail) returns to 0x28 two ticks later (a raw state write, as the game does).
//! * **0x2b grind wrench** (□; anims 0x4e / 0x4f alternating, wrench seq 0xc / 0xd; the melee rows 0 / 1).
//! * **0x42 grind hurt** (the hit intake's grind flag 0x13f90e, `damage::Damage::grind_hit`, with health ≥ 2 and
//!   the rail ahead clear; else knocked off into 0x16): one damage, 77 ticks invulnerable, a 10 u/s hop along the
//!   rail at 2.5 u/s, anim 0x10; back to 0x28 on the wrap or when falling onto the rail.
//!
//! **Magneboots** (item 28, 0x13d4dc). Surface 2 (0x140637) near the ground with the boots worn sets 0x13f658
//! ([`contacts`]: the game tests the feet item moby 0x140430 for class 0xad, the Magneboots model, which item slot
//! 2 creates from the owned boots — **L**: the port reads the ownership). Gravity mode 0x141403 ([`gravity_mode`],
//! 0x248ad8): 1 in 0x3f / 0x70 / 0x71, or in idle 0 with fewer than 4 air ticks on surface 2 with the boots, else
//! 0x13f658. In mode 1 gravity pulls along −normal (0x248b68), the ground probe runs along the gravity direction
//! 0x13f5e0 = −normal, TurnTo turns in the hero's own frame, SetPlanarVel follows the tilted facing, the step
//! snap moves along the normal, the capsule's sphere sits 0.6 up his own z axis (0x233940 → 0x248ea8) and the
//! post-move straightening aligns his up axis with the floor (0x236358). The
//! ground physics' magnetic branch ([`ground_magnet`]) replaces the wall check and gravity.
//! * **0x3f** Magneboots walk (group 1, anim 0x5b): 2 ↔ 0x3f on 0x13f658; speed 2..3.5 u/s toward the stick
//!   inside ±70° of the facing; crouch → 4, ✕ → 0x71, falls off (5 air ticks above 0.8) → 6, off the magnetic
//!   floor → 2, no stick for 30 ticks → 0.
//! * **0x71** Magneboots hop (group 1, anim 0x5e; ✕ in 0 / 4 / 0x3f with 0x13f658 = 1): the ground case, back to 0
//!   after frame 25. **0x70** Magneboots wrench swing (group 6, □ with 0x13f658 = 1: the weapon check): melee rows
//!   5 / 6 alternating, anim 0x5c / 0x5d, back to 0 after the row's idle frame.
//!
//! **Cable slide 0x74** (the zipline: Ratchet hangs from a cable by the wrench; group 0x1a, anims 0x73 then 0x66;
//! docs/plan/hero_states.md "Cable slide 0x74"). The cables are grind paths too; only Kerwan's (3) are cables. Cable
//! contact `0x20d330` (the overlays of [`CABLE_LEVELS`]): a rising jump (group 4, 0x13f76e = 0) or the fall
//! (group 2), 0x13f534 = 0, the hand point (0.3 ahead, 1.34 up in his frame) near a grind path (the rail search
//! above; Kerwan's own search reaches 1.7 across and 1.5 up or down in every state: [`FIXED_REACH_LEVELS`]) and −0.7 (−1.2 on the tick □ is pressed) .. 0.4 above it. Taken by the jump group ([`jump_contacts`]) and
//! the fall; the weapon check leaves □ to the grab (no jump attack 0x14 at a cable). Entry: the wrench forced into
//! the hand (0x1413f7). Physics: the hands follow the path at a speed that approaches 14 u/s at 9·dt² (half the
//! entry speed along it to start), a spring pulls the hands onto it; past the end he flies on and falls (6, 8-tick
//! lockout) after 5 ticks. No jump-off.
//!
//! **Effects** (hero polish): the rail's and the cable's loop (class sound 0 in the hero's sound slot 0x141568,
//! `super::packs::loop_sound`; released by the grind jump / switch / hurt and by a group change), the type-25 sparks
//! (`super::fx::spark`: the spawner's draw at the game's point, the particle created by the particle hook), the
//! grind wrench's hit sphere (radius 1 at the hip, queued with the pack hits and delivered through the hit sink after
//! the hero update), the cable grab's voice 0xd and sparkle burst `0x2a7e20`.
//!
//! **The grind's joint records** (L00 0x17a6e0.. = records 0..3 of the block, `super::idle::joint`): [`grind_lean`].
//!
//! **Not ported** (cosmetic or outside the hero): the look-at moby
//! 0x13f928 and the moby-armed targeted grind jump (0x13f908 / 0x13f90c / 0x13f8a0, set by no ported class);
//! level 16's class-0x101 bump; the aim lean's steering with a weapon in hand (0x13f920 only springs back to 0); the Hologuise / other bodies.
//!
//! Standard `f32` for the new code (the hero block's PS2 floats converted at the boundary); the random draws are
//! the game's, in its order.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::anim::AnimCtl;
use super::physics::*;
use super::states::Ctx;
use super::Hero;
use crate::pad::{button, fast_arctan, fast_diff_rots};
use crate::ps2v::Pf;
use crate::rng::Rng;
use crate::spline::{self, Cursor};
use rc_formats::volumes::GrindPath;

/// Item ids (`0x13d4c0 + id`).
pub const MAGNEBOOTS: usize = 28;
pub const GRIND_BOOTS: usize = 29;
/// The levels whose overlay has the rail contact 0x20cf58 (and the grind states).
pub const RAIL_LEVELS: [i32; 12] = [0, 4, 6, 7, 8, 9, 10, 13, 14, 16, 17, 18];
/// The levels whose overlay has the cable contact 0x20d330 (and 0x74).
pub const CABLE_LEVELS: [i32; 8] = [0, 3, 4, 7, 9, 10, 13, 17];

const DTF: f32 = 1.0 / 60.0;
const DT2F: f32 = 1.0 / 3600.0;
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
const PI: f32 = std::f32::consts::PI;
/// 70° and 45°.
const DEG70: f32 = 1.221_730_5;
const DEG45: f32 = std::f32::consts::FRAC_PI_4;

/// Item ownership (the game state's owned table, mirrored into [`Hero::owned`]): the one read the boots make.
fn owns(h: &Hero, item: usize) -> bool { h.owned.has(item) }

/// The boots' fields of the hero block (0x13f850..0x13f96c). Native `f32`.
#[derive(Clone, Default, PartialEq)]
pub struct Boots {
    /// 0x13f850: the rail point under the feet; 0x13f8b0: the rail (grind path index); 0x13f8b4 / 0x13f8b8: its
    /// cursor; 0x13f8d4: closed; 0x13f8bc: rail contact this tick.
    pub rail_pt: [f32; 3],
    pub rail: Option<usize>,
    pub cur: Cursor,
    pub closed: bool,
    pub contact: bool,
    /// 0x13f8c0: ticks off the end of the rail (0 = on it).
    pub off_rail: i32,
    /// 0x13f8c4: speed (u/tick); 0x13f8c8: direction along the spline (always 1).
    pub speed: f32,
    pub dir: i32,
    /// 0x13f8cc / 0x13f8d0: rail yaw / pitch; 0x13f8d8: facing offset (−90°); 0x13f8dc: stance (anim 0x31 + it).
    pub yaw: f32,
    pub pitch: f32,
    pub face: f32,
    pub stance: i32,
    /// 0x13f8e0: rail switch running; 0x13f890 / 0x13f8e4..0x13f8f0: the old rail's point, path, cursor, closed;
    /// 0x13f8f4: ticks of the blend (48).
    pub switching: bool,
    pub old_pt: [f32; 3],
    pub old_rail: Option<usize>,
    pub old_cur: Cursor,
    pub old_closed: bool,
    pub switch_ticks: i32,
    /// 0x13f8f8: lean anim (−1 none, 0 uphill, 1 downhill, 2 / 3 stick sideways: anims 0x4a + it).
    pub lean: i32,
    /// 0x13f8fc: slope speed term; 0x13f900: base speed.
    pub slope_acc: f32,
    pub base: f32,
    /// 0x13f908 / 0x13f90c / 0x13f910 / 0x13f914: the targeted grind jump (armed by a moby; none ported), its
    /// target 0x13f8a0 and gravity 0x13f918.
    pub f908: i32,
    pub f90c: i16,
    pub f910: i32,
    pub f914: i32,
    pub target: [f32; 3],
    pub target_g: f32,
    /// 0x13f91c: the grind hurt's speed.
    pub hurt_speed: f32,
    /// 0x13f920 / 0x13f924: body lean angle / velocity (the aim lean; drives joint records only).
    pub lean_angle: f32,
    pub lean_vel: f32,
    /// 0x13f92c: the rail pull (spring velocity).
    pub pull: f32,
    /// The cable: 0x13f930 point, 0x13f940 path, 0x13f944 / 0x13f948 cursor, 0x13f94c contact, 0x13f950 ticks off
    /// the end, 0x13f954 speed, 0x13f95c closed, 0x13f960 / 0x13f964 yaw / pitch, 0x13f968 pull.
    pub cable_pt: [f32; 3],
    pub cable_rail: Option<usize>,
    pub cable_cur: Cursor,
    pub cable: bool,
    pub cable_off: i32,
    pub cable_speed: f32,
    pub cable_closed: bool,
    pub cable_yaw: f32,
    pub cable_pitch: f32,
    pub cable_pull: f32,
}

impl std::fmt::Debug for Boots {
    /// `0` while untouched (the hero digest drops new zero fields), else the fields.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Boots::default() { return write!(f, "0"); }
        f.debug_struct("Boots")
            .field("rail", &(self.rail, self.cur, self.closed, self.contact, self.rail_pt))
            .field("ride", &(self.off_rail, self.speed, self.base, self.slope_acc, self.yaw, self.pitch, self.lean, self.stance, self.pull))
            .field("switch", &(self.switching, self.old_rail, self.old_cur, self.switch_ticks))
            .field("hurt", &self.hurt_speed)
            .field("cable", &(self.cable, self.cable_rail, self.cable_cur, self.cable_off, self.cable_speed, self.cable_yaw, self.cable_pull))
            .finish()
    }
}

// ------------------------------------------------------------------------------------------------
// Small native vector helpers.

type V3 = [f32; 3];
fn f3(v: V4) -> V3 { to_f32x3(v) }
fn p4(v: V3, w: Pf) -> V4 { [Pf::f(v[0]), Pf::f(v[1]), Pf::f(v[2]), w] }
fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn add(a: V3, b: V3) -> V3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn scl(a: V3, k: f32) -> V3 { [a[0] * k, a[1] * k, a[2] * k] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn len(a: V3) -> f32 { dot(a, a).sqrt() }
fn set_len(a: V3, l: f32) -> V3 { let n = len(a); if n == 0.0 { [0.0; 3] } else { scl(a, l / n) } }
fn pf(x: f32) -> Pf { Pf::f(x) }

/// `Approach(t, step, &x)` 0x270728 on `f32`.
fn approach(t: f32, step: f32, x: &mut f32) {
    let d = (t - *x).clamp(-step, step);
    *x += d;
}

/// `Spring(t, k, d, max, &x, &v)` 0x270780 on `f32` (velocity clamped to ±max and ±|t − x|; a snap within 1 %
/// of max).
fn spring(t: f32, k: f32, d: f32, max: f32, x: &mut f32, v: &mut f32) {
    let e = t - *x;
    let mut nv = *v + (k * e - d * *v);
    if 0.0 < max { nv = nv.clamp(-max, max); }
    nv = nv.clamp(-e.abs(), e.abs());
    *v = nv;
    *x += nv;
    if (t - *x).abs() < max * 0.01 {
        *x = t;
        *v = 0.0;
    }
}

/// The grind paths the hero can reach this tick ([`super::platform::HeroWorld::grind_paths`]).
fn paths<'a>(env: &'a Env) -> &'a [GrindPath] { env.world.map(|w| w.grind_paths()).unwrap_or(&[]) }

fn path_pts(paths: &[GrindPath], i: Option<usize>) -> Option<&[spline::Point]> { paths.get(i?).map(|p| p.points.as_slice()) }

// ------------------------------------------------------------------------------------------------
// Contacts (the hero update between the wall probe and the transitions).

/// The levels whose overlay compiles the fixed-reach path search (Kerwan's `0x205830`, `rc-trace overlay-diff`,
/// docs/plan/level_generalisation.md H2): the only cable level without grind rails. The others with a search
/// compile level00's `0x20cd08` (levels 4, 6–10, 13, 14, 16–18: the same code).
pub const FIXED_REACH_LEVELS: [i32; 1] = [3];

/// The reach of the path search: (horizontal, vertical). Level00's `0x20cd08`: 0.3 in groups 0 / 1, else 0.9;
/// with 0x13f51a set 0.5 more and 10 vertically, else 1.5. Kerwan's `0x205830`: 1.7 and 1.5 in every state.
fn reach(level: i32, group: i32, f51a: bool) -> (f32, f32) {
    if FIXED_REACH_LEVELS.contains(&level) { return (1.7, 1.5); }
    let (mut hr, mut vr) = (if (group as u32) < 2 { 0.3 } else { 0.9 }, 1.5);
    if f51a {
        vr = 10.0;
        hr += 0.5;
    }
    (hr, vr)
}

/// `0x20cd08(p, …, exclude, 0)` (Kerwan: `0x205830`): the first grind path near `p` (module doc): inside its
/// bounding sphere, the nearest point `q` (the 12 / 10 search) within the [`reach`] of `p` (xy distance, |dz|).
fn find_rail(paths: &[GrindPath], p: V3, level: i32, group: i32, f51a: bool, exclude: Option<usize>) -> Option<(usize, V3, Cursor, bool)> {
    for (i, gp) in paths.iter().enumerate() {
        if Some(i) == exclude || gp.points.is_empty() { continue; }
        let c = [gp.bsphere[0], gp.bsphere[1], gp.bsphere[2]];
        if !(spline::dist3(c, p) <= gp.bsphere[3]) { continue; }
        let closed = gp.flag != 0;
        let Some((q, cur)) = spline::nearest(&gp.points, closed, 12.0, 10.0, 0.0, p) else { continue };
        let (hr, vr) = reach(level, group, f51a);
        if spline::dist2(p, q) < hr && (p[2] - q[2]).abs() < vr { return Some((i, q, cur, closed)); }
    }
    None
}

/// The rail mount points of levels 0x10, 0xe and 6 (`0x20cf58`): no rail contact farther than the radius from
/// all of them.
fn near_mount(level: i32, p: V3) -> bool {
    let (pts, r): (&[V3], f32) = match level {
        0x10 => (&[[115.0, 264.0, 133.0], [257.0, 301.0, 128.0]], 12.0),
        0xe => (&[[157.0, 265.0, 51.0], [162.0, 341.0, 48.0], [308.0, 104.0, 69.0]], 12.0),
        6 => (&[[177.0, 304.0, 143.0]], 14.0),
        _ => return true,
    };
    pts.iter().any(|&m| spline::dist3(p, m) <= r)
}

/// The hand point `0x20d2f8` (`0x233660(0.3, 0, 1.34)`): pos + rows·(0.3, 0, 1.34).
fn hand_point(h: &Hero) -> V3 {
    let r = h.rows.map(f3);
    let l = [0.3f32, 0.0, 1.34];
    add(f3(h.pos), std::array::from_fn(|k| l[0] * r[0][k] + l[1] * r[1][k] + l[2] * r[2][k]))
}

/// After the surface reaction (L00 hero update 0x2070d0): the magnetic floor 0x13f658 (the reaction's surface-2
/// branch, module doc), the rail contact `0x20cf58` and the cable contact `0x20d330`.
pub(super) fn contacts(h: &mut Hero, env: &Env) {
    if h.f0637 != 0 && (h.f65c == 0 || h.height < Pf::b(0x3e99_999a)) && h.mode == 0 && h.magneboots_on() { h.f658 = 1; }
    let level = h.idle.level;
    let paths = paths(env);
    if RAIL_LEVELS.contains(&level) { rail_contact_probe(h, paths, level); }
    if CABLE_LEVELS.contains(&level) { cable_contact_probe(h, env, paths); }
}

fn rail_contact_probe(h: &mut Hero, paths: &[GrindPath], level: i32) {
    if !owns(h, GRIND_BOOTS) { return; }
    let b = &mut h.boots;
    b.contact = false;
    if h.f518 != 0 { return; }
    let pos = f3(h.pos);
    if !near_mount(level, pos) { return; }
    let st = h.state;
    let ok = matches!(h.group, 0..=2)
        || (matches!(st, 0x29 | 0x2a) && (h.boots.off_rail != 0 || h.boots.f914 != 0))
        || (h.group == 4 && (h.jump.descending != 0 || h.f51a != 0));
    if !ok { return; }
    let Some((i, q, cur, closed)) = find_rail(paths, pos, h.idle.level, h.group, h.f51a != 0, None) else { return };
    let dz = pos[2] - q[2];
    let b = &mut h.boots;
    if (st != 0x29 || b.f914 == 0 || Some(i) != b.rail) && -0.5 < dz && (dz < 0.57 || h.f51a != 0) {
        (b.rail_pt, b.rail, b.cur, b.closed, b.contact) = (q, Some(i), cur, closed, true);
    }
}

fn cable_contact_probe(h: &mut Hero, env: &Env, paths: &[GrindPath]) {
    h.boots.cable = false;
    if h.f534 != 0 { return; }
    let hand = hand_point(h);
    if !((h.group == 4 && h.jump.descending == 0) || h.group == 2) { return; }
    let Some((i, q, cur, closed)) = find_rail(paths, hand, h.idle.level, h.group, h.f51a != 0, None) else { return };
    let lo = if env.pad.pressed & button::SQUARE != 0 { -1.2 } else { -0.7 };
    let dz = hand[2] - q[2];
    if lo < dz && dz < 0.4 {
        let b = &mut h.boots;
        (b.cable_pt, b.cable_rail, b.cable_cur, b.cable_closed, b.cable) = (q, Some(i), cur, closed, true);
    }
}

/// A grind rail under / at the hero (0x13f8bc ≠ 0).
pub(super) fn rail_contact(h: &Hero) -> bool { h.boots.contact }

/// A cable caught (0x13f94c ≠ 0).
pub(super) fn cable_contact(h: &Hero) -> bool { h.boots.cable }

/// The jump group's contact line (level00 0x229b70 case 7 / 9 / 10 / 0xb..0x12 / 0x1c / 0x3c / 0x69, right after the
/// Thruster stomp's R1 test and before the forced fall; Kerwan's 0x21c668 has only the cable half): `0x28` on the
/// rail contact 0x13f8bc, else `0x74` on the cable contact 0x13f94c. The rail contact takes a descending jump, the
/// cable contact a rising one ([`contacts`]). True when the state changed.
pub(super) fn jump_contacts(h: &mut Hero, c: &mut Ctx) -> bool {
    let to = if rail_contact(h) { 0x28 } else if cable_contact(h) { 0x74 } else { return false };
    h.set_state(c, to, true);
    true
}

// ------------------------------------------------------------------------------------------------
// The gravity mode and frame (level01 0x248ad8, 0x248b68, 0x2323d8, 0x232738, 0x232cc0, 0x233588, 0x236358).

/// `0x248ad8`: the gravity mode 0x141403 for this tick.
pub(super) fn gravity_mode(h: &Hero) -> u8 {
    if matches!(h.state, 0x3f | 0x71 | 0x70) { return 1; }
    if h.f0637 != 0 && owns(h, MAGNEBOOTS) && h.state == 0 && (h.air_ticks as i32) < ticks(4) { return 1; }
    h.f658 as u8
}

/// The gravity direction of mode 1 (and the up of the frame): the unit ground normal 0x13f5c0.
fn up(h: &Hero) -> V3 { set_len(f3(h.ground_normal), 1.0) }

/// `0x248b68(amount, dst, src)` in mode 1: `src − unit(normal)·amount`.
pub(super) fn gravity_vec(h: &Hero, src: V4, amount: Pf) -> V4 {
    let g = scl(up(h), -amount.to_f32());
    p4(add(f3(src), g), src[3])
}

/// Rows (row i = image of axis i) of an Euler rotation, as `f32`.
fn rows_of(rot: V4) -> [V3; 3] {
    let r = euler_rows(rot);
    [f3(r[0]), f3(r[1]), f3(r[2])]
}

/// `MatrixToEuler` 0x2721f0 of `f32` rows.
fn euler_of(rows: &[V3; 3]) -> V4 {
    let m = rows.map(|r| p4(r, Pf::ZERO));
    crate::moby_update::services::rows_euler(&[m[0], m[1], m[2], [Pf::ZERO, Pf::ZERO, Pf::ZERO, Pf::ONE]])
}

/// Rotation of `v` about the unit axis `k` by `a` (Rodrigues).
fn rotate(v: V3, k: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    add(add(scl(v, c), scl(cross(k, v), s)), scl(k, dot(k, v) * (1.0 - c)))
}

/// `TurnTo(k, d, max)` in gravity modes 1 / 2 (`0x2323d8`): the stick direction in the hero's own frame, an
/// angular spring from 0 toward its yaw there (yaw velocity 0x13f4d4), the frame turned about its own up axis.
pub(super) fn frame_turn(h: &mut Hero, k: Pf, d: Pf, max: Pf) {
    let r = rows_of(h.rot);
    let s = f3(h.stick_world);
    let local = [dot(r[0], s), dot(r[1], s)];
    let target = Pf::f(local[1].atan2(local[0]));
    let (mut a, mut v) = (Pf::ZERO, h.yaw_vel);
    h.yaw_residual = turn_spring(target, k, d, max, &mut a, &mut v, 0);
    h.yaw_vel = v;
    let a = a.to_f32();
    if a == 0.0 { return; }
    let axis = r[2];
    h.rot = euler_of(&[rotate(r[0], axis, a), rotate(r[1], axis, a), r[2]]);
}

/// `SetPlanarVel` in gravity modes ≠ 0 (`0x232738`): vel = the moby's facing row × speed.
pub(super) fn frame_planar_vel(h: &mut Hero) {
    let r = rows_of(h.moby_rot);
    h.vel = p4(scl(r[0], h.speed.to_f32()), h.vel[3]);
}

/// The ground probe's line in gravity modes ≠ 0 (`0x232cc0`): from `up_len` against the gravity direction to 48
/// along it, clipped at the world's lower bounds.
pub(super) fn frame_probe_line(h: &Hero, up_len: Pf) -> (V4, V4) {
    let g = set_len(f3(h.gravity_dir), 1.0);
    let p = f3(h.pos);
    let p0 = p4(sub(p, scl(g, up_len.to_f32())), h.pos[3]);
    let mut p1 = p4(add(p, scl(g, 48.0)), h.pos[3]);
    super::common::clip_to_world(p0, &mut p1);
    (p0, p1)
}

/// The step snap in gravity modes ≠ 0 (`0x233588`): grounded below the floor, onto the ground point (at once when
/// deeper than the velocity into the normal, else by at most 0.05, or half the gap from 0.21).
pub(super) fn frame_snap(h: &mut Hero) {
    let into = dot(f3(h.vel), f3(h.ground_normal));
    if h.air_ticks != 0 || !(h.height < Pf::ZERO) { return; }
    if h.height.to_f32() < into + 0.0001 {
        h.pos = h.ground_point;
        return;
    }
    let d = sub(f3(h.ground_point), f3(h.pos));
    let l = len(d);
    let step = if l < 0.21 { if l <= 0.05 { l } else { 0.05 } } else { l * 0.5 };
    h.pos = p4(add(f3(h.pos), set_len(d, step)), h.pos[3]);
}

/// The post-move straightening in gravity modes 1 / 2 or with 0x13f548 (`0x236358`): the up axis springs toward
/// the ground normal (0.3, 0.3; 0.001 airborne with 0x13f548; velocity 0x13f3f4), in mode 1 airborne about the
/// body point 0.8 up.
pub(super) fn frame_align(h: &mut Hero) {
    let target = up(h);
    let r = rows_of(h.rot);
    let k = if h.air_ticks != 0 && h.f548 != 0 { 0.001 } else { 0.3 };
    let c = dot(target, r[2]).clamp(-1.0, 1.0);
    let theta = (1.0 - c * c).sqrt().atan2(c);
    let (mut a, mut v) = (Pf::ZERO, h.swim.euler_vel[1]);
    turn_spring(Pf::f(theta), Pf::f(k), Pf::f(k), Pf::ZERO, &mut a, &mut v, 0);
    h.swim.euler_vel[1] = v;
    let a = a.to_f32();
    let axis = cross(r[2], target);
    if a == 0.0 || len(axis) < 1e-6 { return; }
    let axis = set_len(axis, 1.0);
    let nr = [rotate(r[0], axis, a), rotate(r[1], axis, a), rotate(r[2], axis, a)];
    let body_old = scl(r[2], 0.8);
    h.rot = euler_of(&nr);
    if h.gravity_mode == 1 && h.air_ticks != 0 {
        let body_new = scl(rows_of(h.rot)[2], 0.8);
        h.pos = p4(add(sub(f3(h.pos), body_new), body_old), h.pos[3]);
    }
}

/// The ground physics' magnetic branch (0x140637 with the Magneboots owned; L00 0x217970 ground case): gravity
/// along the gravity mode's direction (24·dt² from the effective velocity above 0.2, else 54·dt² with the feet
/// held on a floor moby's ground point), no wall check. True: it replaced the rest of the ground physics.
pub(super) fn ground_magnet(h: &mut Hero, _env: &Env) -> bool {
    if h.f0637 == 0 || !owns(h, MAGNEBOOTS) { return false; }
    if Pf::b(0x3e4c_cccd) < h.height {
        h.vel = gravity(h, h.eff, DT2 * Pf::f(24.0));
    } else {
        h.vel = gravity(h, h.vel, DT2 * Pf::f(54.0));
        if h.ground_moby.is_some() {
            h.pos = h.ground_point;
            h.air_ticks = 0;
            h.height = Pf::ZERO;
        }
    }
    true
}

/// `0x248b68(amount, dst, src)` by gravity mode: mode 0 `src` with z − amount, mode 1 along −normal.
pub(super) fn gravity(h: &Hero, src: V4, amount: Pf) -> V4 {
    if h.gravity_mode == 1 { return gravity_vec(h, src, amount); }
    let mut v = h.vel;
    v[2] = src[2] - amount;
    if h.gravity_mode == 0 { v } else { src }
}

// ------------------------------------------------------------------------------------------------
// SetState entries.

/// The rows of the melee table the boots use (L00 0x17bc28, stride 0x2c; rows 0 / 1 = the combo's first two,
/// 5 / 6 the Magneboots swings): `[kind, step, chain-before, input-ref, chain-from, jump-after, idle-after, hit-from,
/// hit-to, +0x24, +0x28]`.
const ROW_STEP: usize = 1;
const ROW_JUMP_AFTER: usize = 5;
const ROW_IDLE_AFTER: usize = 6;

/// The melee table row of the swing (`super::melee::COMBO`, rows 5 / 6 for 0x70).
fn row(h: &Hero) -> &'static [i32; 11] { &super::melee::COMBO[h.melee.combo.clamp(0, 6) as usize] }

fn hand_is_wrench(h: &Hero) -> bool { h.items.ready_item().is_some_and(|m| m.o_class == super::melee::WRENCH_CLASS) }

/// SetState entry of 0x28..0x2b, 0x3f, 0x42, 0x70, 0x71, 0x74. `None`: continue with SetState's epilogue.
pub(super) fn entry(h: &mut Hero, c: &mut Ctx, id: i32, play: bool, old_sub: i32) -> Option<bool> {
    match id {
        0x28 => grind_entry(h, c, play),
        0x29 | 0x2a => return grind_jump_entry(h, c, id, play, old_sub),
        0x2b => {
            h.group = 0xf;
            h.melee.hit = 0;
            h.melee.combo = if h.prev_state == 0x2b { (h.melee.combo == 0) as i32 } else { 0 };
            if play {
                let step = h.melee.combo;
                h.set_anim(c.anim, c.rng, blend(4), (0x4e + step) as u8, 0);
                if hand_is_wrench(h) { h.items.pending_blend = Some(((0xc + step) as u8, 0, ticks(4))); }
            }
        }
        0x42 => {
            h.group = 0xf;
            h.f15d4 = 0;
            if h.boots.off_rail == 0 { super::damage::take_damage(h, 1); }
            h.f510 = ticks(0x4d);
            let b = &mut h.boots;
            b.hurt_speed = DTF * 2.5;
            b.base = b.base.min(b.hurt_speed);
            b.slope_acc = 0.0;
            h.vel = V0;
            h.disp = V0;
            h.vel[2] = pf(DTF * 10.0);
            if play { h.set_anim(c.anim, c.rng, Pf::f(-3.0), 0x10, 2); }
        }
        0x3f => return h.walk_entry(c, 0x3f, play),
        0x70 => {
            h.group = 6;
            h.melee.hit = 0;
            h.melee.combo = if h.prev_state == 0x70 && row(h)[ROW_STEP] == 0 { 6 } else { 5 };
            if play {
                let step = row(h)[ROW_STEP];
                h.set_anim(c.anim, c.rng, blend(5), (0x5c + step) as u8, (step == 0) as i32);
                if hand_is_wrench(h) { h.items.pending_blend = Some(((0xc + step) as u8, (step == 0) as i32, ticks(5))); }
            }
        }
        0x71 => {
            h.group = 1;
            h.f15d4 = 0;
            if play {
                if h.prev_state != 4 { h.set_anim(c.anim, c.rng, blend(8), 0x5e, 1); } else { h.set_anim(c.anim, c.rng, blend(5), 0x5e, 2); }
            }
        }
        0x74 => cable_entry(h, c),
        _ => {}
    }
    None
}

fn blend(n: i32) -> Pf { Pf::from_i32(ticks(n)) }

/// 0x28: speed and cursor from the contact.
fn grind_entry(h: &mut Hero, c: &mut Ctx, play: bool) {
    h.f15d4 = 3;
    h.idle.blink_period = 0x68;
    let from_grind = h.prev_group == 0xf;
    let b = &mut h.boots;
    b.dir = 1;
    b.face = -HALF_PI;
    b.off_rail = 0;
    b.stance = 0;
    if !from_grind {
        b.slope_acc = 0.0;
        b.lean = -1;
        b.base = 0.0;
    }
    (b.f908, b.f90c, b.f910) = (0, 0, 0);
    h.group = 0xf;
    let paths = paths(c.env);
    if let Some(pts) = path_pts(paths, h.boots.rail) {
        let mut cur = h.boots.cur;
        let (p, _) = spline::advance(pts, h.boots.closed, 1.0, &mut cur);
        let d = sub(p, f3(h.pos));
        if 0.001 < len(d) && !from_grind { h.boots.base = dot(f3(h.disp), set_len(d, 1.0)); }
    }
    if play { h.set_anim(c.anim, c.rng, blend(6), (0x31 + h.boots.stance) as u8, 0); }
}

/// 0x29 / 0x2a: the jump system's entry (lockout, the shared defaults) with the grind parameters.
fn grind_jump_entry(h: &mut Hero, c: &mut Ctx, id: i32, play: bool, old_sub: i32) -> Option<bool> {
    if h.jump_lockout != 0 {
        h.state = h.prev_state;
        h.timer = h.prev_timer;
        h.substate = old_sub;
        return Some(false);
    }
    h.jump_block_defaults(c.rng);
    h.f15d4 = 3;
    let j = &mut h.jump;
    if id == 0x29 {
        let (mut hh, mut g) = (2.4f32, 27.0f32);
        let mut hmax = 2.45f32;
        (j.f_apex, j.f_hold, j.f_land) = (pf(18.0), pf(28.0), pf(14.0));
        j.takeoff = ticks(5);
        j.ramp = ticks(0x11) as i16;
        if h.surf.f0638 != 0 {
            // A booster floor (surface 5): higher and floatier; one Kalebo booster adds 0.7.
            g = 14.0;
            hh += 4.7;
            hmax += 4.7;
            if spline::dist3(f3(h.pos), [331.0, 104.0, 145.0]) < 10.0 {
                hh += 0.7;
                hmax += 0.7;
            }
        }
        (j.h, j.hmin, j.hmax) = (pf(hh), pf(hh), pf(hmax));
        j.g = pf(DT2F * g);
    } else {
        (j.f_apex, j.f_hold, j.f_land) = (pf(18.0), pf(31.0), pf(14.0));
        (j.h, j.hmin, j.hmax) = (pf(2.5), pf(2.5), pf(2.55));
        j.takeoff = ticks(2);
        j.ramp = ticks(10) as i16;
        j.g = pf(DT2F * 24.0);
    }
    j.bottom784 = Pf::b(0x3f4c_cccd);
    h.group = 0xf;
    h.boots.f914 = 0;
    if !play { return None; }
    if id == 0x29 {
        h.set_anim(c.anim, c.rng, blend(5), 0x50, 4);
    } else {
        let b = &h.boots;
        let side = (b.rail_pt[1] - b.old_pt[1]).atan2(b.rail_pt[0] - b.old_pt[0]);
        let seq = if HALF_PI < fast_diff_rots(h.rot[2], pf(side)).to_f32() { 0x1f } else { 0x1e };
        h.set_anim(c.anim, c.rng, blend(5), seq, 0);
    }
    None
}

/// 0x74: the cable's start speed (half the displacement along it).
fn cable_entry(h: &mut Hero, c: &mut Ctx) {
    h.f15d4 = 3;
    h.group = 0x1a;
    h.items.f13f7 = 1;
    h.idle.blink_period = 0x68;
    h.boots.cable_off = 0;
    h.boots.cable_pull = 0.0;
    if let Some(pts) = path_pts(paths(c.env), h.boots.cable_rail) {
        let mut cur = h.boots.cable_cur;
        let (p, _) = spline::advance(pts, h.boots.cable_closed, 1.0, &mut cur);
        let d = sub(p, f3(h.pos));
        if 0.001 < len(d) { h.boots.cable_speed = dot(f3(h.disp), set_len(d, 1.0)); }
    }
    h.boots.cable_speed = (h.boots.cable_speed * 0.5).max(0.0);
    h.set_anim(c.anim, c.rng, blend(8), 0x73, 0);
}

// ------------------------------------------------------------------------------------------------
// Per-state physics.

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(h: &mut Hero, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng) -> bool {
    match h.state {
        0x28..=0x2b | 0x42 => phys_grind(h, env, anim.view().seq_b, rng),
        0x3f => phys_magnet_walk(h, env),
        0x70 | 0x71 => h.phys_ground(env, anim, rng),
        0x74 => phys_cable(h, env, anim, rng),
        _ => return false,
    }
    true
}

/// A type-25 spark `0x2825f8(pos, vel, 0)` (vel.w = its gravity) through the hero's particle queue
/// ([`super::fx::spark`]: the spawner's size draw now).
fn spark(h: &mut Hero, rng: &mut Rng, pos: V3, vel: V3, gravity: f32) {
    let w = h.pos[3].to_f32();
    super::fx::spark(h, rng, [pos[0], pos[1], pos[2], w], [vel[0], vel[1], vel[2], gravity], false);
}

/// The grind case of L00 0x217970 (0x28..0x2b, 0x42). `seq_b` = Ratchet's moby +0x53.
fn phys_grind(h: &mut Hero, env: &Env, seq_b: u8, rng: &mut Rng) {
    let st = h.state;
    // The rail's loop (class sound 0) in the hero's sound slot 0 (0x141568) while riding or swinging on it; the grind
    // jump, the rail switch and the grind hurt release it (the game tests the slot's owner and state first).
    if matches!(st, 0x29 | 0x2a | 0x42) { super::packs::release_loop(h, 0); } else { super::packs::loop_sound(h, 0, 0); }
    if st == 0x28 || st == 0x2b {
        let y = h.boots.yaw + rng.randf_sym(0.0, std::f32::consts::FRAC_PI_6);
        let s = rng.randf(DTF * 4.0, DTF * 7.0);
        let vz = rng.randf(DTF * 3.0, DTF * 5.0);
        spark(h, rng, f3(h.pos), [y.cos() * s, y.sin() * s, vz], -(DT2F * 30.0));
    }
    // The aim lean needs a weapon in hand (0x1413fa and a weapon-type slot item): never on the boots' levels here.
    let aim = false;
    if matches!(h.boots.lean, 2 | 3) { h.boots.lean = -1; }
    let paths = paths(env);
    let Some(pts) = path_pts(paths, h.boots.rail) else { return };
    let closed = h.boots.closed;
    let pos = f3(h.pos);
    if let Some((q, c)) = spline::nearest(pts, closed, 999.0, 2.0, 2.5, pos) { (h.boots.rail_pt, h.boots.cur) = (q, c); }
    let old = if h.boots.switching { path_pts(paths, h.boots.old_rail) } else { None };
    if let Some(op) = old {
        if let Some((q, c)) = spline::nearest(op, h.boots.old_closed, 999.0, 2.0, 2.5, pos) { (h.boots.old_pt, h.boots.old_cur) = (q, c); }
    }
    let high = Pf::b(0x3f5e_b852) < h.height && st != 0x29 && st != 0x42 && !h.boots.switching && ticks(30) < h.f4ec;
    if h.boots.off_rail != 0 || high {
        // Off the rail: his momentum and plain gravity (the jumps keep the jump system's).
        h.vel = h.eff;
        if matches!(st, 0x29 | 0x2a) { h.jump_vertical(env); } else { h.vel[2] = pf(h.eff[2].to_f32() - DT2F * 24.0); }
        h.boots.off_rail += 1;
        return;
    }
    let b = &mut h.boots;
    let sp = if st == 0x42 { b.hurt_speed } else { b.speed };
    let d = sp * b.dir as f32;
    let (p, _) = spline::advance(pts, closed, d, &mut b.cur);
    let q_old = old.map(|op| spline::advance(op, b.old_closed, d, &mut b.old_cur).0);
    grind_lean(h, pts, closed, seq_b);
    let b = &mut h.boots;
    let next = spline::step_index(pts, b.cur.seg, b.dir, closed);
    let dv = sub(xyz(pts[next as usize]), p);
    if len(dv) < 0.001 {
        if b.cur.seg == next {
            // The end of an open rail: fly on (vel unchanged).
            b.off_rail = ticks(1);
            return;
        }
    } else {
        b.yaw = dv[1].atan2(dv[0]);
        b.pitch = dv[2].atan2((dv[0] * dv[0] + dv[1] * dv[1]).sqrt());
    }
    h.target_yaw = fast_add_rotations(pf(h.boots.yaw), pf(h.boots.face));
    h.turn_to(pf(0.015), pf(0.15), pf(DTF * 6.283_185_5));
    let airborne = (matches!(st, 0x29 | 0x2a) && h.jump.landed == 0) || st == 0x42;
    if airborne {
        let b = &mut h.boots;
        if !b.switching || ticks(0x30) < b.switch_ticks {
            let e = sub(p, b.rail_pt);
            let mut pos = f3(h.pos);
            h.vel[0] = pf(e[0]);
            h.vel[1] = pf(e[1]);
            pos[2] += e[2];
            let mut f = sub(b.rail_pt, pos);
            f[2] = 0.0;
            if DTF < len(f) { f = set_len(f, DTF); }
            pos[0] += f[0];
            pos[1] += f[1];
            h.pos = p4(pos, h.pos[3]);
        } else {
            let q = q_old.unwrap_or(b.old_pt);
            b.switch_ticks += 1;
            let n = ticks(0x30) as f32;
            let a = scl(sub(p, q), b.switch_ticks as f32 / n);
            let a0 = scl(sub(p, q), (b.switch_ticks - 1) as f32 / n);
            let mut pos = f3(h.pos);
            pos[2] += a[2] - a0[2];
            let t = add(q, a);
            h.vel[0] = pf(t[0] - pos[0]);
            h.vel[1] = pf(t[1] - pos[1]);
            let e = sub(p, b.rail_pt);
            pos[2] += e[2];
            if pos[2] < b.rail_pt[2] && b.switch_ticks <= ticks(0x30) { pos[2] = b.rail_pt[2]; }
            h.pos = p4(pos, h.pos[3]);
        }
        if st == 0x42 { h.vel[2] = pf(h.vel[2].to_f32() - DT2F * 23.0); } else { h.jump_vertical(env); }
    } else {
        h.boots.switching = false;
        let base_t = DTF * 12.0;
        if Pf::b(0x3f00_0000) < h.stick_mag && !aim {
            h.stick_target(env, Pf::ONE);
            let a = fast_diff_rots(h.target_yaw, env.cam_yaw);
            let hp = pf(HALF_PI);
            if fast_diff_rots(a, hp).to_f32() < DEG70 || fast_diff_rots(a, -hp).to_f32() < DEG70 {
                let s = fast_subtract_rotations(h.target_yaw, env.cam_yaw);
                h.boots.lean = if fast_diff_rots(s, hp).to_f32() < DEG45 { 2 } else { 3 };
            }
        }
        let b = &mut h.boots;
        let (mut la, mut lv) = (pf(b.lean_angle), pf(b.lean_vel));
        turn_spring(Pf::ZERO, pf(0.015), pf(0.3), Pf::ZERO, &mut la, &mut lv, 0);
        (b.lean_angle, b.lean_vel) = (la.to_f32(), lv.to_f32());
        approach(base_t, DT2F * 15.0, &mut b.base);
        // The slope 0.3 ahead along the spline.
        let mut ahead = b.cur;
        let (x, _) = spline::advance(pts, closed, 0.3, &mut ahead);
        let s = (x[2] - b.rail_pt[2]).atan2(spline::dist2(b.rail_pt, x));
        let acc_t;
        if 0.0 < s {
            acc_t = s * 1.909_859_3 * DTF * -2.5;
            if 0.261_799_4 < s && !matches!(b.lean, 2 | 3) { b.lean = 0; } else if s < 0.139_626_34 && b.lean == 0 { b.lean = -1; }
        } else {
            acc_t = -s * 2.864_789 * DTF * 8.0;
            if s < -0.191_986_22 && !matches!(b.lean, 2 | 3) { b.lean = 1; } else if -0.122_173_05 < s && b.lean == 1 { b.lean = -1; }
        }
        if b.slope_acc < acc_t { approach(acc_t, DT2F * 8.0, &mut b.slope_acc); } else { approach(acc_t, DT2F * 2.0, &mut b.slope_acc); }
        b.slope_acc = b.slope_acc.clamp(DTF * -2.5, DTF * 12.0);
        b.speed = (b.base + b.slope_acc).max(0.0);
        h.vel = p4(sub(p, b.rail_pt), h.vel[3]);
        // The pull onto the rail.
        let mut e = sub(b.rail_pt, f3(h.pos));
        let l = len(e);
        let mut x = -l;
        spring(0.0, 0.03, 0.3, DTF * 7.0, &mut x, &mut b.pull);
        if l < b.pull { b.pull = l; }
        if b.pull < l { e = set_len(e, b.pull); }
        let mut pos = add(f3(h.pos), e);
        if pos[2] < b.rail_pt[2] && h.height.to_f32() < 1.0 { pos[2] = b.rail_pt[2]; }
        h.pos = p4(pos, h.pos[3]);
    }
    if st == 0x2b && h.timer < ticks(0x14) {
        // The wrench's sweep on the rail: `0x259888(1.0, tmpl, Ratchet, 0x10000, dir)` (dir = the rail yaw ±20°..35°)
        // and `coll_sphere_mobys(1.0, hip, 0x10, Ratchet, tmpl)`, delivered after the hero update with the pack hits.
        let y = h.boots.yaw + rng.randf_sym(0.349_065_85, 0.610_865_2);
        let mut c = h.pos;
        c[2] = pf(c[2].to_f32() + 0.6);
        let dir = [pf(y.cos()), pf(y.sin()), Pf::ZERO, Pf::ZERO];
        let tmpl = super::packs::template(env, 1.0, 0x1_0000, dir);
        h.packs.hits.push(super::packs::PackHit::Sphere { r: Pf::ONE, centre: c, flags: 0x10, tmpl });
    }
}

/// The grind's joint-record targets (L00 0x21ce0c..0x21d1b0, after the rail advance; records 0..3 of the block, L00
/// 0x17a680 = L01 0x17ab00: `super::idle::joint`):
///
/// | address | what it does | port |
/// |---|---|---|
/// | 0x21ce0c..0x21cf7c | more than 90 ticks in the state (0x13f4ec): the rail 30·\|eff\| and 0.1 further ahead (two copies of the cursor, `0x25d808`); both inside the rail (or a closed one): the turn to it `d = fast_subtract_rotations(0x13f8cc, yaw ahead)`, record 0 springs (0.03, 0.2), y = clamp(0.37·d, ±45°), negated in the other stance 0x13f8dc | [`grind_lean`] |
/// | 0x21cf84..0x21cfbc | not 0x29 / 0x42 / 0x2a: record 0 x = the displacement's pitch `atan2(0x13f458, \|0x13f450.xy\|)` | [`grind_lean`] |
/// | 0x21cfc0..0x21d024 | a gun in hand (0x140408 ≠ 0xc and its definition's +0x18 ≠ 0): records 3 / 1 / 2 springs (0.017, 0.3), (0.027, 0.3), (0.027, 0.3) | [`grind_lean`] |
/// | 0x21d024..0x21d0cc | a look-at moby 0x13f928: records 3 / 2 z from its bearing in the hero's frame | n/a: 0x13f928 is set by no ported class (module doc) |
/// | 0x21d0d0..0x21d1ac | Ratchet on sequence 0x31 / 0x4a: record 3 z = −47° + lean 0x13f920, records 1 / 2 z = 42° + lean / 1.7; on 0x4b: record 3 z = 8° + lean, records 1 / 2 z = −4° + lean / 2 (`fast_add_rotations`) | [`grind_lean`] |
///
/// Side effects: the records only.
fn grind_lean(h: &mut Hero, pts: &[spline::Point], closed: bool, seq_b: u8) {
    use super::idle::joint::{HEAD, NECK, REC0, REC2};
    let add_rot = |a: f32, b: f32| fast_add_rotations(pf(a), pf(b)).to_f32();
    if ticks(0x5a) < h.f4ec {
        let a = 30.0 * h.eff_len.to_f32();
        let (mut c1, mut c2) = (h.boots.cur, h.boots.cur);
        let (p1, e1) = spline::advance(pts, closed, a, &mut c1);
        let (p2, e2) = spline::advance(pts, closed, a + 0.1, &mut c2);
        if !(e1 || e2) || closed {
            let ahead = fast_arctan(pf(p2[0] - p1[0]), pf(p2[1] - p1[1]));
            let t = (fast_subtract_rotations(pf(h.boots.yaw), ahead).to_f32() * f32::from_bits(0x3ebd_70a4)).clamp(-DEG45, DEG45);
            let r = &mut h.idle.joints[REC0];
            (r.k, r.d) = (f32::from_bits(0x3cf5_c28f), f32::from_bits(0x3e4c_cccd));
            r.target[1] = if h.boots.stance != 0 { -t } else { t };
        }
    }
    if !matches!(h.state, 0x29 | 0x42 | 0x2a) {
        let d = h.disp;
        h.idle.joints[REC0].target[0] = fast_arctan(Pf::ZERO + (d[0] * d[0] + d[1] * d[1]).sqrt(), d[2]).to_f32();
    }
    let id = h.items.slot.id;
    let gun = id != 0xc && h.weapons.defs.get(id.max(0) as usize).is_some_and(|w| w.w18 != 0);
    if !gun { return; }
    let j = &mut h.idle.joints;
    (j[HEAD].k, j[HEAD].d) = (f32::from_bits(0x3c8b_4396), f32::from_bits(0x3e99_999a));
    (j[NECK].k, j[NECK].d) = (f32::from_bits(0x3cdd_2f1b), f32::from_bits(0x3e99_999a));
    (j[REC2].k, j[REC2].d) = (f32::from_bits(0x3cdd_2f1b), f32::from_bits(0x3e99_999a));
    let lean = h.boots.lean_angle;
    let (head, neck) = match seq_b {
        0x31 | 0x4a => (add_rot(f32::from_bits(0xbf51_ff7e), lean), add_rot(f32::from_bits(0x3f3b_a866), lean / 1.7)),
        0x4b => (add_rot(f32::from_bits(0x3e0e_fa35), lean), add_rot(f32::from_bits(0xbd8e_fa35), lean * 0.5)),
        _ => return,
    };
    j[HEAD].target[2] = head;
    j[NECK].target[2] = neck;
    j[REC2].target[2] = neck;
}

fn xyz(p: spline::Point) -> V3 { [p[0], p[1], p[2]] }

/// The Magneboots walk 0x3f (L00 0x217970 case 0x3f).
fn phys_magnet_walk(h: &mut Hero, env: &Env) {
    // 0x211be8: StickTarget(1), then 2..3.5 u/s.
    h.stick_target(env, Pf::ONE);
    let ts = (DTF * 3.5 * h.target_speed.to_f32()).max(DTF + DTF);
    h.target_speed = pf(ts);
    let r = rows_of(h.rot);
    let s = f3(h.stick_world);
    let a = dot(r[1], s).atan2(dot(r[0], s)).abs();
    if DEG70 < a {
        h.target_speed = Pf::ZERO;
    } else {
        let k = ((DEG70 - a) * 2.0).min(1.0);
        h.target_speed = pf(h.target_speed.to_f32() * k);
    }
    let m = h.stick_mag.to_f32() + 0.35;
    h.turn_to(pf(0.025 * m), pf(0.3), pf(DTF * 5.235_987_7 * m));
    h.speed_step(pf(DT2F * 7.5), pf(DT2F * 8.5));
    h.set_planar_vel(Pf::b(0x47c3_4f80));
    if h.ground_moby.is_some() {
        h.pos = h.ground_point;
        h.height = Pf::ZERO;
        h.air_ticks = 0;
    }
    h.vel = if h.air_ticks == 0 { gravity(h, h.vel, pf(DT2F * 54.0)) } else { gravity(h, h.eff, pf(DT2F * 24.0)) };
}

/// The cable slide 0x74 (L00 0x217970 case 0x74).
fn phys_cable(h: &mut Hero, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng) {
    let hand = hand_point(h);
    // Sliding (anim 0x66): the cable's loop (class sound 0, hero sound slot 0) and sparks every other tick.
    if anim.view().seq_b == 0x66 { super::packs::loop_sound(h, 0, 0); }
    if anim.view().seq_b == 0x66 && h.idle.counter & 1 != 0 {
        let base = scl(f3(h.disp), 0.4);
        let y = h.boots.cable_yaw + rng.randf_sym(0.0, std::f32::consts::FRAC_PI_6);
        let s = rng.randf(0.0, DTF * 4.0);
        let vz = rng.randf(DTF * 3.0, DTF * 5.0);
        spark(h, rng, hand, [base[0] + y.cos() * s, base[1] + y.sin() * s, vz], -(DT2F * 35.0));
    }
    let paths = paths(env);
    let Some(pts) = path_pts(paths, h.boots.cable_rail) else { return };
    let b = &mut h.boots;
    let closed = b.cable_closed;
    if let Some((q, c)) = spline::nearest(pts, closed, 999.0, 8.0, 2.5, hand) { (b.cable_pt, b.cable_cur) = (q, c); }
    if b.cable_off != 0 {
        let e = h.eff;
        h.vel = [e[0], e[1], pf(e[2].to_f32() - DT2F * 24.0), e[3]];
        b.cable_off += 1;
        return;
    }
    let (p, _) = spline::advance(pts, closed, b.cable_speed, &mut b.cable_cur);
    let next = spline::step_index(pts, b.cable_cur.seg, 1, closed);
    let dv = sub(xyz(pts[next as usize]), p);
    if len(dv) < 0.001 {
        if b.cable_cur.seg == next {
            b.cable_off = ticks(1);
            return;
        }
    } else {
        b.cable_yaw = dv[1].atan2(dv[0]);
        b.cable_pitch = dv[2].atan2((dv[0] * dv[0] + dv[1] * dv[1]).sqrt());
    }
    h.target_yaw = pf(h.boots.cable_yaw);
    h.turn_to(pf(0.018), pf(0.2), pf(DTF * 6.457_718_4));
    let b = &mut h.boots;
    approach(DTF * 14.0, DT2F * 9.0, &mut b.cable_speed);
    h.vel = p4(sub(p, b.cable_pt), h.vel[3]);
    let mut e = sub(b.cable_pt, hand);
    let l = len(e);
    let mut x = -l;
    spring(0.0, 0.05, 0.3, DTF * 10.0, &mut x, &mut b.cable_pull);
    if l < b.cable_pull { b.cable_pull = l; }
    if b.cable_pull < l { e = set_len(e, b.cable_pull); }
    h.pos = p4(add(f3(h.pos), e), h.pos[3]);
}

// ------------------------------------------------------------------------------------------------
// Per-state transitions.

/// Per-state transitions.
pub(super) fn transitions(h: &mut Hero, c: &mut Ctx) {
    match h.state {
        0x28..=0x2b | 0x42 => tr_grind(h, c),
        0x3f => tr_magnet_walk(h, c),
        0x70 => {
            if (row(h)[ROW_IDLE_AFTER] as f32) < c.anim.view().frame { h.set_state(c, 0, true); }
        }
        0x71 => {
            if 25.0 < c.anim.view().frame { h.set_state(c, 0, true); }
        }
        0x74 => {
            let v = c.anim.view();
            // The grab (anim 0x73 passing frame 11): voice 0xd and the sparkle burst `0x2a7e20(0x13f930, 4)` at the
            // hands' cable point (which the burst jitters in place).
            if v.seq_b == 0x73 && passed(&v, 11.0) {
                h.packs.sounds.push(super::packs::SoundCmd::Voice { index: 0xd, flags: 0 });
                let mut pt = h.boots.cable_pt;
                super::fx::sparkle_burst(h, c.rng, &mut pt, 4);
                h.boots.cable_pt = pt;
            }
            if v.seq_b == 0x73 && v.flags & 2 != 0 { h.set_anim(c.anim, c.rng, blend(0x1e), 0x66, 0); }
            if ticks(5) < h.boots.cable_off {
                h.set_state(c, 6, true);
                h.f534 = ticks(8) as i16;
            }
        }
        _ => {}
    }
}

/// `0x211870(f)` (L00; level01 `0x231f18`): Ratchet's key time 0x13fdf8 passed `f` this tick (`f < frame` and
/// `frame − f ≤` this tick's step 0x13fdfc).
pub(super) fn passed(v: &super::AnimView, f: f32) -> bool { f < v.frame && v.frame - f <= v.frame_step }

fn pressed(c: &Ctx, mask: u32, n: i32) -> bool { c.env.pad.pressed_within(mask, ticks(n)).is_some() }

/// The grind transitions (L00 0x229b70 case 0x28..0x2b, 0x42).
fn tr_grind(h: &mut Hero, c: &mut Ctx) {
    let st = h.state;
    if st == 0x42 {
        h.damage.grind_hit = 0;
    } else {
        // Blocked (moving less than half the velocity) or hit (0x13f90e): the grind hurt, or knocked off.
        // (Level 16's class-0x101 bump is not ported.)
        let blocked = len2(h.disp).to_f32() < len2(h.vel).to_f32() * 0.5;
        if blocked || h.damage.grind_hit != 0 {
            let mut clear = true;
            if h.idle.level != 0x10 {
                let b = &h.boots;
                let a = [b.rail_pt[0], b.rail_pt[1], b.rail_pt[2] + 2.0];
                let e = add(a, [b.yaw.cos() * 5.0, b.yaw.sin() * 5.0, 0.0]);
                if c.env.line(p4(a, Pf::ZERO), p4(e, Pf::ZERO), 2).is_some() { clear = false; }
            }
            h.damage.grind_hit = 0;
            if h.health < 2 || !clear {
                let y = h.vel[1].to_f32().atan2(h.vel[0].to_f32()) + PI + c.rng.randf_sym(std::f32::consts::FRAC_PI_6, 0.610_865_2);
                let s = h.boots.speed * 0.7;
                h.eff[0] = pf(y.cos() * s);
                h.eff[1] = pf(y.sin() * s);
                h.eff[2] = pf(DTF * 4.5);
                if ticks(0xf) < h.boots.off_rail && h.health < 2 { h.health = 2; }
                h.set_state(c, 0x16, true);
                h.f518 = ticks(0x46) as i16;
            } else {
                h.set_state(c, 0x42, true);
            }
            return;
        }
    }
    match st {
        0x28 => {
            let mut jump_ok = true;
            if h.idle.level == 8 && spline::dist3(f3(h.pos), [342.0, 111.1, 42.0]) < 7.0 {
                jump_ok = false;
                h.lockout = ticks(100);
            }
            if jump_ok && pressed(c, button::CROSS, 7) {
                if h.boots.f908 != 0 {
                    h.boots.f910 = 1;
                    return;
                }
                h.set_state(c, 0x29, true);
                return;
            }
            if ticks(5) < h.boots.off_rail {
                if Pf::f(8.0) < h.height { h.lockout = ticks(0x46); }
                h.set_state(c, 6, true);
                h.f518 = ticks(8) as i16;
                return;
            }
            if pressed(c, button::SQUARE, 7) {
                h.set_state(c, 0x2b, true);
                return;
            }
            let b = &h.boots;
            let want = if b.lean == -1 { 0x31 + b.stance } else { 0x4a + b.lean } as u8;
            if c.anim.view().seq_b != want { h.set_anim(c.anim, c.rng, blend(0xc), want, 0); }
            // (0x13f90c with 0x13f910: the targeted jump a moby arms; none is ported.)
            (h.boots.f90c, h.boots.f908) = (0, 0);
        }
        0x2b => {
            let r = row(h);
            let frame = c.anim.view().frame;
            if pressed(c, button::CROSS, 5) && !(frame < r[ROW_JUMP_AFTER] as f32) {
                h.set_state(c, 0x29, true);
                return;
            }
            let thr = if h.melee.combo == 0 { 0x17 } else { 0xe };
            if pressed(c, button::SQUARE, 7) {
                if (thr as f32) < frame { h.set_state(c, 0x2b, true); }
                return;
            }
            if (r[ROW_IDLE_AFTER] as f32) < frame {
                h.set_state(c, 0x28, true);
            } else if ticks(5) < h.boots.off_rail {
                h.set_state(c, 6, true);
                h.f518 = ticks(8) as i16;
            }
        }
        0x29 | 0x2a => tr_grind_jump(h, c),
        _ => {
            // 0x42.
            if Pf::ZERO < h.vel[2] {
                let n = (h.vel[2].to_f32() / (DT2F * 23.0)) as i32;
                let v = c.anim.view();
                h.anim_speed_for_ticks(Pf::f(9.0), Pf::from_i32(n), Pf::f(0.5), Pf::f(-1.0), &v);
            }
            let wrapped = c.anim.view().flags & 2 != 0;
            if wrapped || (h.vel[2] < Pf::ZERO && (h.pos[2].to_f32() - h.boots.rail_pt[2]).abs() < 0.3) {
                h.set_state(c, 0x28, true);
            }
        }
    }
}

/// 0x29 / 0x2a: the landing back on a rail, the rail switch, the jump anims and the landing on the rail.
fn tr_grind_jump(h: &mut Hero, c: &mut Ctx) {
    let b = &h.boots;
    if b.contact && (ticks(5) < b.off_rail || b.f914 != 0) && h.pos[2].to_f32() - b.rail_pt[2] < 0.35 {
        h.set_state(c, 0x28, true);
        return;
    }
    if h.state == 0x29 && h.timer < ticks(10) && h.jump.descending == 0 && h.surf.f063e == 0 && Pf::b(0x3f00_0000) < h.stick_mag
        && !h.boots.switching
    {
        rail_switch(h, c);
    }
    let j = h.jump;
    if j.takeoff < h.timer && h.vel[2].to_f32() < 0.001 { h.jump.descending = 1; }
    let early = h.timer < j.takeoff + ticks(1) || j.takeoff + ticks(9) <= h.timer;
    let mut descend = h.jump.descending != 0;
    if !early && !descend && !c.anim.view().blending() {
        // Rising: the anim reaches the apex frame at the apex.
        let n = (h.vel[2].to_f32() / h.jump.g.to_f32()) as i32;
        let v = c.anim.view();
        h.anim_speed_for_ticks(h.jump.f_apex, Pf::from_i32(n), Pf::f(0.5), Pf::f(0.25), &v);
        clamp_switch_speed(h);
        descend = false;
    }
    if descend && h.jump.landed == 0 {
        let mut eta = ticks(0x1e);
        if h.boots.off_rail == 0 { eta = h.land_eta(c, Pf::f(60.0), h.jump.g, Pf::f(h.boots.rail_pt[2])).to_i32(); }
        if eta != 0 {
            let v = c.anim.view();
            h.anim_speed_for_ticks(h.jump.f_hold, Pf::from_i32(eta), Pf::f(0.5), Pf::f(0.25), &v);
        }
        clamp_switch_speed(h);
        let rz = h.boots.rail_pt[2];
        if h.boots.off_rail == 0 && h.height.to_f32() < 0.5 && h.pos[2].to_f32() + h.vel[2].to_f32() < rz {
            h.vel[2] = pf(rz - h.pos[2].to_f32());
            h.jump.landed = ticks(1);
            h.anim_speed = Pf::ONE;
        }
        if h.pos[2].to_f32() - rz < -0.5 {
            h.set_state(c, 6, true);
            h.lockout = ticks(0x46);
            h.f518 = ticks(8) as i16;
            return;
        }
    }
    if h.jump.landed != 0 {
        h.jump.landed += 1;
        if ticks(2) < h.jump.landed {
            // Back on the rail: a raw state write, the stance anim.
            h.state = 0x28;
            let seq = (0x31 + h.boots.stance) as u8;
            h.set_anim(c.anim, c.rng, blend(7), seq, 0);
            return;
        }
    }
    if h.boots.off_rail != 0 && h.air_ticks == 0 && h.jump.descending != 0 && h.set_state(c, 0, false) {
        h.set_anim(c.anim, c.rng, blend(6), 0xc, 4);
    }
}

fn clamp_switch_speed(h: &mut Hero) {
    if h.boots.switching { h.anim_speed = pf(h.anim_speed.to_f32().clamp(0.5, 2.0)); }
}

/// The rail switch of 0x29: a rail 2..4.5 to the side the stick points (±90° of the rail), not the current one.
fn rail_switch(h: &mut Hero, c: &mut Ctx) {
    h.stick_target(c.env, Pf::ONE);
    let ry = pf(h.boots.yaw);
    let hp = pf(HALF_PI);
    let a = fast_diff_rots(h.target_yaw, ry);
    if !(fast_diff_rots(a, hp).to_f32() < DEG70) { return; }
    let mut side = fast_add_rotations(ry, hp);
    if HALF_PI < fast_diff_rots(h.target_yaw, side).to_f32() { side = fast_add_rotations(ry, -hp); }
    let side = side.to_f32();
    let paths = paths(c.env);
    let Some(pts) = path_pts(paths, h.boots.rail) else { return };
    let mut d = 2.0f32;
    while d <= 4.5 {
        let mut cur = h.boots.cur;
        let (mut q, ended) = spline::advance(pts, h.boots.closed, h.eff_len.to_f32() * ticks(0) as f32, &mut cur);
        if ended { q = add(f3(h.pos), scl(f3(h.disp), ticks(0x30) as f32)); }
        q[0] += side.cos() * d;
        q[1] += side.sin() * d;
        if let Some((i, _, _, closed)) = find_rail(paths, q, h.idle.level, h.group, h.f51a != 0, h.boots.rail) {
            let b = &mut h.boots;
            (b.old_rail, b.old_cur, b.old_closed, b.old_pt) = (b.rail, b.cur, b.closed, b.rail_pt);
            (b.rail, b.closed) = (Some(i), closed);
            if let Some((q, cur)) = spline::nearest(&paths[i].points, closed, 999.0, 2.0, 0.0, f3(h.pos)) { (b.rail_pt, b.cur) = (q, cur); }
            b.switch_ticks = 0;
            b.switching = true;
            h.set_state(c, 0x2a, true);
            return;
        }
        d += 0.75;
    }
}

/// The Magneboots walk's transitions (L00 0x229b70 case 0x3f).
fn tr_magnet_walk(h: &mut Hero, c: &mut Ctx) {
    h.anim_speed = pf((h.speed.to_f32() * 27.0).max(0.5));
    if c.env.pad.held & button::CROUCH != 0 {
        h.set_state(c, 4, true);
        return;
    }
    if pressed(c, button::CROSS, 7) {
        h.set_state(c, 0x71, true);
        return;
    }
    let air = h.air_ticks as i32;
    if ticks(5) <= air && Pf::b(0x3f4c_cccd) < h.height {
        h.set_state(c, 6, true);
        return;
    }
    h.f548 = ticks(0x28) as i16;
    if DEG45 < h.rot[1].to_f32().abs() || DEG45 < h.rot[0].to_f32().abs() { h.f500 = ticks(0x28) as i16; }
    let stay = (h.grounded_ticks == 0 || h.f0637 != 0) && (air <= ticks(7) || h.height.to_f32() <= 1.0);
    if !stay {
        h.set_state(c, 2, true);
        return;
    }
    if c.anim.view().seq_b != 0x5b { h.set_anim(c.anim, c.rng, blend(8), 0x5b, 0); }
    if c.env.pad.no_direction && ticks(0x1e) <= h.timer && h.set_state(c, 0, false) {
        h.momentum = V0;
        let seq = h.idle_seq();
        h.set_anim(c.anim, c.rng, blend(0xf), seq, 0);
    }
}

#[cfg(test)]
mod tests;
