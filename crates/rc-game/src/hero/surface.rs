//! **Package P1 — surfaces, slopes and sliding.** Owner: the P1 agent, which also owns [`super::platform`]
//! (docs/plan/hero_states.md "Packages"). Addresses level01 unless marked L00 (level00, the superset build:
//! SetState 0x2223f8, physics 0x217970, transitions 0x229b70, reaction 0x20b960, ground probe 0x2127b8).
//!
//! **The surface reaction** `HeroSurfaceReaction` 0x22cd48 (L00 0x20b960), after the move: the ground probe
//! 0x232dc0 left the hit's surface id in 0x140630 (`Hero::surface_id`); the reaction clears the per-tick flags
//! 0x140630..0x14063f (`FastMemZero16`, the footstep class 0x14063d survives) and sets them from the id, then
//! takes the surface states:
//!
//! | surface | flag | effect |
//! |---|---|---|
//! | 0 water | 0x140634 | depth 0x1415f4 = level − floor, wading 0x1413f9 for depths in (0.25, 0.85) (super::swim) |
//! | 1 hot floor | 0x140635 | grounded: Clank (body 1) → 0x7d; else 0x3c burn bounce / 0x7c burn death without health (not while rising in 0x3c) |
//! | 2 magnetic | 0x140637 | near the ground (0x13f65c = 0 or height < 0.3), on some levels always; 0x13f658 on the class-0xad floor (Magneboots, super::boots) |
//! | 3 sinking liquid | 0x140636 | falling into it (0 < level − z, |level − (z + 0.25)| < 1, disp.z < 0): 0x68 (0x7b without health), not in 0x68 / 0x7b / a rising 0x69 |
//! | 4 sinking floor | 0x140633 | 0x31 while grounded (0x13f650 ≠ 0) outside groups 7 / 0x10 / 0x14 |
//! | 5, 6 | 0x140638 / 0x140639 | Hoverboard levels 5 and 16 (read by the Hoverboard) |
//! | 7 slippery | 0x140632 | the slide 0x2f and the slippery variants of the ground code |
//! | 8, 0xc pit | 0x14063a | 0x79 pit fall from the ground states (super::damage) |
//! | 9 | 0x14063e | read by the jump transitions |
//! | 0xb | 0x14063b | falling into the liquid level 0x13f644 (as 3): 0x7b (only where 3 is handled) |
//! | 0xd deadly liquid | 0x14063c | falling into the water level 0x13f640 (as 3): 0x7f sinking death (super::damage) |
//! | 0xe shallow water | 0x140634 | water level = ground + 0.2 |
//!
//! Level 0xd also takes 0x7b when a sphere at the body point (radius r + 0.03, flags 2) touches collision of
//! type 0xb. The order of the rules is L00's: 4 → 1 → 0xd → 3 → 0xb.
//!
//! **Per level.** Each overlay compiles its own reaction with only the rules of the surfaces that level can
//! meet ([`LEVEL_RULES`], read from the 18 distinct functions: level 3 shares level 1's). The port implements the
//! superset once and switches the rules per level with that table. The test `tests/hero/hero_surfaces.rs` checks the
//! data: every surface id a level's collision (world mesh and moby class collision) uses is handled by that
//! level's own rules or by no level at all (0xa, 0xf..0x1e never are), so the per-level switch only matters
//! for data a level does not have.
//!
//! **States.**
//! * 0x2f slide (slippery floor, group 1; L00): the walk's entry case with the slippery branch (speed ≤ 7.7 u/s,
//!   vel.z = 0, |vel| ≤ 7.7·dt, anim 0x37), its own physics (the slide move L00 0x2167d0 + gravity), the walk
//!   transitions with the slide's anims 0x37 / 0x6d and playback speed ([`walk_surface`], [`walk_tail`]).
//! * 0x31 sinking floor (group 0x10): sinks by 40·dt² a tick (≥ 50·dt² faster than it falls), turns toward the
//!   push in 0x13f440, rolls into turns, back to idle off the surface once 0x13f530 has run out (the sand flow
//!   class 679 holds it and writes the push; not ported).
//! * 0x68 sinking liquid (group 0x19; L00): sinks at 1 u/s, slowing to 0.25 u/s once deeper than (n + 1)/2 for
//!   the n-th try; ✕ (after 15 + 25·n ticks, at most twice) → 0x69; the death fade (super::damage) below the
//!   level − 1.2 or after 170 ticks.
//! * 0x69 jump out (the jump system, group 0x19, h = depth + 1.7).
//! * 0x7b no health / level-0xd contact (group 0x19): sinks at 1 u/s, the fade below the level − 1.2 or after 150.
//!
//! **Slopes.** Steeper than 50° is not ground (the ported probe): the hero falls (6) and the capsule slides him
//! down; the edge brake 0x236a68, the steep-wall stop 0x232820 and the slope ratio 0x13f4bc are ported
//! (physics.rs). The slide 0x2f adds the slippery floor's own downhill pull (8·slope/45° u/s).
//!
//! **Seams** (one call each in the shared files, all inert unless 0x140632 is set, i.e. on the surface-7 levels
//! 0, 4, 7, 9, 10, 12, 13, 14, 17): the idle / stop / walk entries ([`idle_entry`], [`stop_entry`],
//! [`slippery_walk_entry`]), the ground physics' drag and crouch brake ([`ground_drag`], [`crouch_decel`]), the idle
//! transitions ([`idle_slide`]), the idle fidgets and sequence ([`slippery`]), the combo on ice ([`slide_move`]),
//! the walk transitions ([`walk_surface`], [`walk_tail`]), the jump windup (jump.rs, on the latched 0x13f7f8: the
//! takeoff runs [`slide_move`] so the slide's momentum carries into the jump), the capsule sizing
//! ([`slippery_capsule`]) and the ground probe ([`probe_surface`]).
//!
//! **Not ported** (cosmetic, recorded as [`SurfaceEvent`]s for the engine where it can use them): the body leans
//! of 0x2f / 0x31 (joint records 0x17a680 / 0x17ab00 and their spring settings, as the walk's; the sinking floor's
//! bubbles `0x22b140(4, 2)` and sand puff `0x286cb0` are ported, through `super::fx`), level 15's lava splash 0x217450 on
//! 0x7b, the Hologuise end 0x231450 before the surface states (mode 3 is not in the port). Landing on a slippery
//! floor goes through the landing picker (jump.rs, P3 / P4): its leading L00 test (0x2293a8: slippery and
//! |eff.xy| > 0.5·dt → 0x2f) is ported; the idle / walk / stop entries it picks redirect to 0x2f as the game's do.
//! The Thruster hover's slippery brake (L00 case 0x81) is P4's.
#![allow(clippy::neg_cmp_op_on_partial_ord)] // compare order kept from the original.

use super::anim::AnimCtl;
use super::common::blend;
use super::physics::*;
use super::states::Ctx;
use super::Hero;
use crate::collision_query::{coll_sphere_m, CollOutput, QueryFlags};
use crate::ps2v::Pf;
use crate::rng::Rng;

// ------------------------------------------------------------------------------------------------
// The per-level rules.

/// The rules one level's `HeroSurfaceReaction` compiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceRules {
    /// Bit `n`: surface id `n` is handled (its flag and, for 1 / 3 / 4 / 0xb / 0xd, its state rule; 0xb's 0x7b
    /// rule exists exactly where 3 is handled).
    pub handled: u16,
    /// Surface 2 sets 0x140637 unconditionally too (a second, unconditional test after the near-ground one).
    pub magnet_always: bool,
    /// Surface 1 turns Clank (body 1) into 0x7d (the Clank levels).
    pub clank_burn: bool,
}

impl SurfaceRules {
    pub const fn handles(&self, id: i16) -> bool { 0 <= id && id < 16 && self.handled & (1 << id) != 0 }
}

const fn ids(list: &[u8]) -> u16 {
    let mut m = 0u16;
    let mut i = 0;
    while i < list.len() {
        m |= 1 << list[i];
        i += 1;
    }
    m
}

const fn r(list: &[u8], magnet_always: bool, clank_burn: bool) -> SurfaceRules { SurfaceRules { handled: ids(list), magnet_always, clank_burn } }

/// The full set (L00 0x20b960 and the other 142-line builds: levels 0, 4, 7, 9, 10, 13, 17).
pub const SUPERSET: SurfaceRules = r(&[0, 1, 2, 3, 4, 7, 8, 9, 0xb, 0xc, 0xd, 0xe], true, true);
const NOVALIS: SurfaceRules = r(&[0, 2, 4, 8, 9, 0xb, 0xc, 0xd, 0xe], false, false);

/// Each level's reaction (index = level number 0x15ed84), from the functions listed in the P1 report:
/// 00 0x20b960, 01 0x22cd48 (= 03 0x2047d8), 02 0x21b698, 04 0x1fef40, 05 0x239fc0, 06 0x222fb8, 07 0x2337c8,
/// 08 0x21d9c0, 09 0x22b640, 10 0x1fefc8, 11 0x23c7a0, 12 0x22de30, 13 0x217448, 14 0x21dd30, 15 0x2044c8,
/// 16 0x2097a0, 17 0x2088b8, 18 0x213b40.
pub const LEVEL_RULES: [SurfaceRules; 19] = [
    SUPERSET,
    NOVALIS,
    r(&[2, 3, 8, 9, 0xb, 0xc, 0xd], true, false),
    NOVALIS,
    SUPERSET,
    r(&[0, 1, 2, 3, 4, 5, 6, 8, 9, 0xb, 0xc, 0xd, 0xe], false, false),
    r(&[1, 2, 8, 9, 0xc, 0xd], false, true),
    SUPERSET,
    r(&[0, 2, 4, 8, 9, 0xb, 0xc, 0xd, 0xe], true, false),
    SUPERSET,
    SUPERSET,
    r(&[0, 2, 8, 9, 0xb, 0xc, 0xd, 0xe], false, false),
    r(&[0, 2, 7, 8, 9, 0xc, 0xd, 0xe], true, false),
    SUPERSET,
    r(&[2, 7, 8, 9, 0xb, 0xc, 0xd], true, false),
    r(&[0, 2, 3, 4, 8, 9, 0xb, 0xc, 0xe], false, false),
    r(&[0, 2, 4, 5, 6, 8, 9, 0xb, 0xc, 0xd, 0xe], true, false),
    SUPERSET,
    r(&[0, 2, 3, 4, 8, 9, 0xb, 0xc, 0xe], true, false),
];

/// The rules of level `level` (0x15ed84, `Idle::level`, set by the engine at the level load). An unknown level
/// (−1: the unit tests and the headless Novalis runs) keeps level01's rules, the build the ported hero code comes
/// from (the hand-built test floors are surface 1, which level01 ignores); above 18 the superset.
pub fn rules(level: i32) -> SurfaceRules {
    if level < 0 { return NOVALIS; }
    usize::try_from(level).ok().and_then(|i| LEVEL_RULES.get(i)).copied().unwrap_or(SUPERSET)
}

/// The surface id Ratchet's own collision passes through, per level (index = 0x15ed84): the capsule passes'
/// query flags `0x20 | id << 8 | 4` (`HeroCapsulePasses` 0x233940, `0x24` on level01; state 0x7f always uses
/// 0xd24) and `LandEta` 0x248268's ground line (the same flags). Each overlay compiles its own constant
/// (docs/plan/level_generalisation.md H2, `rc-trace overlay-diff`): 0 (water) on 16 levels, 3 (the quicksand)
/// on level 2 (capsule 0x222270: `state == 0x7f ? 0xd24 : 0x324`; `LandEta` 0x2360c8), 0xd (the deadly liquid)
/// on levels 6 and 0xe (capsule 0x22a1d0 / 0x224db0 and `LandEta` 0x242950 / 0x23bed0: 0xd24 in every state).
/// On Aridia the capsule no longer holds Ratchet 0.25 into the quicksand: he sinks as 0x68 says. (On 6 and 0xe the
/// sinking death 0x7f uses 0xd24 on every level anyway, and their data has no surface-0xd face.)
pub const PASS_SURFACE: [u8; 19] = [0, 0, 3, 0, 0, 0, 0xd, 0, 0, 0, 0, 0, 0, 0, 0xd, 0, 0, 0, 0];

/// [`PASS_SURFACE`] of level `level` (unknown: level01's 0).
pub fn pass_surface(level: i32) -> u8 { usize::try_from(level).ok().and_then(|i| PASS_SURFACE.get(i)).copied().unwrap_or(0) }

/// The query flags of Ratchet's own collision (module doc of [`PASS_SURFACE`]): `exclude_surface(id) | 4`.
pub fn pass_flags(level: i32) -> QueryFlags { QueryFlags::exclude_surface(pass_surface(level)) | QueryFlags::MOBY_SUBMASK }

// ------------------------------------------------------------------------------------------------
// The block fields.

/// A sound the surface code plays, queued in [`Surf::events`] at the game's call and played by [`flush`] through
/// the hero's sound layer ([`super::HeroSounds`]) at the next of the hero update's flush points (right after the
/// physics, the surface reaction and the transitions, the steps that queue them); the queue is empty between hero
/// updates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceEvent {
    /// Ratchet's class sound `0x216de8(id, 0)` (L00; level01 `0x236738`): 9 sinking into the liquid, 10 the jump
    /// out.
    Sound(i32),
    /// The sinking floor's loop sound in hero sound slot 2 (`0x236798(2, Ratchet, id)`: `PlayClassSound(id, 4,
    /// Ratchet)` when the slot's voice 0x141570 is free; the voice it returns goes to 0x141570): 6, on level 15 0x1d.
    LoopSound(i32),
    /// The slot-2 voice (the value) released (`release_voice_slot` 0x2a1348) when the sinking floor lets go.
    StopLoop(i32),
}

/// The surface fields of the hero block that are not `Hero` fields of the ported code.
#[derive(Clone, PartialEq)]
pub struct Surf {
    /// Per-tick flags of the reaction: 0x140633 (4), 0x140635 (1), 0x140636 (3), 0x140638 (5), 0x140639 (6),
    /// 0x14063b (0xb), 0x14063c (0xd), 0x14063e (9). (0x140632, 0x140634, 0x140637, 0x14063a are `Hero` fields.)
    pub f0633: u8,
    pub f0635: u8,
    pub f0636: u8,
    pub f0638: u8,
    pub f0639: u8,
    pub f063b: u8,
    pub f063c: u8,
    pub f063e: u8,
    /// 0x13f644: the sinking liquid's level (the z of the last surface-3 / 0xb probe hit).
    pub liquid: f32,
    /// 0x1409b0: the jumps out of the liquid in a row (0x68's entry counts them).
    pub sink_jumps: i32,
    /// 0x13f700: the slide factor (velocity along the stick direction, 1 = full) the slide anim reads; 0x13f704:
    /// the ground pitch last tick (0x31: cleared at the entry).
    pub slide: f32,
    pub last_pitch: f32,
    /// 0x13fd20..0x13fd30: the sand flow's yaw, pitch, –, speed, pull (written by the flow class 679, not
    /// ported; 0x13fd24 is the pitch the sinking floor tilts to after 100 ticks); cleared on entering 0x31.
    pub flow: [f32; 5],
    /// 0x13fd34 / 0x13fd38: the sinking floor's roll / pitch spring velocities.
    pub tilt_vel: [Pf; 2],
    /// 0x141570: the voice of hero sound slot 2 (−1 free).
    pub voice: i32,
    /// Sounds and fades for the engine.
    pub events: Vec<SurfaceEvent>,
}

impl Default for Surf {
    fn default() -> Surf {
        Surf {
            f0633: 0, f0635: 0, f0636: 0, f0638: 0, f0639: 0, f063b: 0, f063c: 0, f063e: 0, liquid: 0.0,
            sink_jumps: 0, slide: 0.0, last_pitch: 0.0, flow: [0.0; 5], tilt_vel: [Pf::ZERO; 2], voice: -1,
            events: Vec::new(),
        }
    }
}

impl std::fmt::Debug for Surf {
    /// `0` while untouched (the hero digest drops new zero fields), else the fields.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Surf::default() { return write!(f, "0"); }
        f.debug_struct("Surf")
            .field("flags", &[self.f0633, self.f0635, self.f0636, self.f0638, self.f0639, self.f063b, self.f063c, self.f063e])
            .field("liquid", &self.liquid)
            .field("sink_jumps", &self.sink_jumps)
            .field("slide", &self.slide)
            .field("last_pitch", &self.last_pitch)
            .field("flow", &self.flow)
            .field("tilt_vel", &self.tilt_vel)
            .field("voice", &self.voice)
            .field("events", &self.events)
            .finish()
    }
}

fn f(x: Pf) -> f32 { x.to_f32() }
fn p(x: f32) -> Pf { Pf::f(x) }
const DTF: f32 = 1.0 / 60.0;
const DT2F: f32 = 1.0 / 3600.0;
/// 5° (0.08726646): the slope above which the slippery floor pulls.
const FIVE_DEG: f32 = 0.087_266_46;

// ------------------------------------------------------------------------------------------------
// The reaction.

impl Hero {
    /// `HeroSurfaceReaction` 0x22cd48 (the superset, L00 0x20b960, with this level's [`rules`]).
    pub fn surface_reaction(&mut self, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng) {
        let rules = rules(self.idle.level);
        let sid = self.surface_id;
        // FastMemZero16(0x140630, 0x10): the id and every flag; the footstep class 0x14063d survives.
        self.surface_id = -1;
        (self.f0632, self.f0634, self.f0637, self.f063a) = (0, 0, 0, 0);
        let s = &mut self.surf;
        (s.f0633, s.f0635, s.f0636, s.f0638, s.f0639, s.f063b, s.f063c, s.f063e) = (0, 0, 0, 0, 0, 0, 0, 0);
        self.f13f9 = 0;
        self.f658 = 0;
        if sid == -1 { return; }
        let has = |id: i16| sid == id && rules.handles(id);
        // Surface 2 near the ground; 0x13f658 on the class-0xad floor moby with Ratchet's body is P5's.
        if has(2) && (self.f65c == 0 || self.height < Pf::b(0x3e99_999a)) { self.f0637 = 1; }
        if has(0xe) {
            self.f0634 = 1;
            self.water_level = self.ground_z + Pf::b(0x3e4c_cccd);
            self.f15f4 = Pf::b(0x3e4c_cccd);
        }
        if has(0) {
            self.f15f4 = self.water_level - self.ground_z;
            if self.f15f4 < Pf::b(0x3f59_999a) && Pf::b(0x3e80_0000) < self.f15f4 { self.f13f9 = 1; }
            self.f0634 = 1;
        }
        if has(3) { self.surf.f0636 = 1; }
        let mut c = Ctx { env, anim, rng, voice: None };
        // Level 0xd: a body-point sphere touching collision of type 0xb.
        if self.idle.level == 0xd && self.state != 0x7b {
            let r = f(self.cap_radius) + 0.03;
            let hit = coll_sphere_m(env.coll, env.mobys, to_f32x3(self.body_point), r, QueryFlags(2), None);
            if hit.is_some_and(|o| o.surface_id() == 0xb) {
                self.set_state(&mut c, 0x7b, true);
                return;
            }
        }
        if has(0xb) { self.surf.f063b = 1; }
        if sid == 2 && rules.magnet_always && rules.handles(2) { self.f0637 = 1; }
        if has(5) { self.surf.f0638 = 1; }
        if has(6) { self.surf.f0639 = 1; }
        if has(4) { self.surf.f0633 = 1; }
        if has(7) { self.f0632 = 1; }
        if has(0xd) { self.surf.f063c = 1; }
        if has(1) { self.surf.f0635 = 1; }
        if has(8) || has(0xc) { self.f063a = 1; }
        if has(9) { self.surf.f063e = 1; }
        // Surface 4: the sinking floor.
        if self.surf.f0633 != 0 && self.grounded_ticks != 0 && !matches!(self.group, 0x10 | 0x14 | 7) {
            self.set_state(&mut c, 0x31, true);
            return;
        }
        // Surface 1: Clank's burn, the burn bounce / death (super::damage).
        if self.surf.f0635 != 0 {
            if rules.clank_burn && self.grounded_ticks != 0 && self.mode == 1 && self.state != 0x7d {
                self.set_state(&mut c, 0x7d, true);
                return;
            }
            if self.grounded_ticks != 0 && (self.state != 0x3c || self.jump.descending != 0) {
                self.leave_disguise();
                let id = if self.health == 0 { 0x7c } else { 0x3c };
                self.set_state(&mut c, id, true);
                return;
            }
        }
        let z = f(self.pos[2]);
        let falling_into = |level: f32, disp_z: f32| (level - (z + 0.25)).abs() < 1.0 && 0.0 < level - z && disp_z < 0.0;
        let dz = f(self.disp[2]);
        // Surface 0xd: the sinking death (super::damage).
        if self.surf.f063c != 0 && self.state != 0x7f && falling_into(f(self.water_level), dz) {
            self.leave_disguise();
            self.set_state(&mut c, 0x7f, true);
            return;
        }
        // Surface 3: the sinking liquid.
        if self.surf.f0636 != 0 && !matches!(self.state, 0x68 | 0x7b) && !(self.state == 0x69 && self.jump.descending == 0)
            && falling_into(self.surf.liquid, dz)
        {
            self.leave_disguise();
            let id = if self.health == 0 { 0x7b } else { 0x68 };
            self.set_state(&mut c, id, true);
            return;
        }
        // Surface 0xb (where 3 is handled): straight to 0x7b.
        if self.surf.f063b != 0 && rules.handles(3) && self.state != 0x7b && falling_into(self.surf.liquid, dz) {
            self.leave_disguise();
            self.set_state(&mut c, 0x7b, true);
        }
    }

    /// `0x22cd18` before the surface states: in mode 3 (the Hologuise) `0x231450` ends the disguise (mode 3 is
    /// not in the port).
    fn leave_disguise(&mut self) {}
}

/// The ground probe's surface branches of L00 0x2127b8 after the 0xd one (the port's probe is level01's):
/// surface 3 → the liquid level 0x13f644 = the hit's z and the probe re-cast (flags 4) from just below it (the
/// start point's z), so the ground is the floor under the liquid (none: no ground); 0xb → the liquid level (the
/// probe then ends without ground, as before); 1 in the burn deaths 0x7c / 0x7d → no ground. False: the probe
/// ends here (no ground).
pub(super) fn probe_surface(h: &mut Hero, env: &Env, p0: &mut V4, p1: V4, o: &mut CollOutput) -> bool {
    match h.surface_id {
        3 => {
            h.surf.liquid = o.point[2];
            p0[2] = Pf::f(o.point[2]) - Pf::b(0x3c23_d70a);
            match env.line(*p0, p1, 4) {
                Some(o2) => {
                    *o = o2;
                    true
                }
                None => false,
            }
        }
        0xb => {
            h.surf.liquid = o.point[2];
            true
        }
        1 if matches!(h.state, 0x7c | 0x7d) => false,
        _ => true,
    }
}

// ------------------------------------------------------------------------------------------------
// The registry's entry / physics / transitions.

/// SetState entry of 0x2f, 0x31, 0x68, 0x69, 0x7b. `None`: continue with SetState's epilogue.
pub(super) fn entry(h: &mut Hero, c: &mut Ctx, id: i32, play: bool, old_sub: i32) -> Option<bool> {
    match id {
        // The walk's case (L00 0x2232d4) with its slippery branch (`slippery_walk_entry`).
        0x2f => h.walk_entry(c, id, play),
        0x31 => {
            // 0x23e06c: the wrench stays in hand (0x1413f7), 0x1415d4 = 3, group 0x10, 0x13fd20..0x13fd3f
            // cleared, the momentum = this tick's displacement.
            h.items.f13f7 = 1;
            h.f15d4 = 3;
            h.group = 0x10;
            h.surf.flow = [0.0; 5];
            h.surf.tilt_vel = [Pf::ZERO; 2];
            h.momentum = h.disp;
            if play { h.set_anim(c.anim, c.rng, blend(15), 100, 0); }
            None
        }
        0x68 => {
            // L00 0x22668c: count the tries (right after a jump out, or after one with a detour that was not a
            // plain jump from the ground).
            let again = h.prev_state == 0x69 || (h.prev_prev_state == 0x69 && h.prev_group != 0 && h.prev_state != 7);
            h.surf.sink_jumps = if again { h.surf.sink_jumps + 1 } else { 0 };
            h.group = 0x19;
            h.f15d4 = 0;
            h.surf.events.push(SurfaceEvent::Sound(9));
            h.speed = Pf::ZERO;
            h.idle.blink_period = 0x68;
            h.target_speed = Pf::ZERO;
            if play { h.set_anim(c.anim, c.rng, blend(8), 0x71, 0); }
            None
        }
        0x69 => {
            // The jump group's entry (L00 0x224590) with 0x69's parameters (0x224a68).
            if h.jump_lockout != 0 {
                h.state = h.prev_state;
                h.timer = h.prev_timer;
                h.substate = old_sub;
                return Some(false);
            }
            h.surf.events.push(SurfaceEvent::Sound(10));
            h.jump_block_defaults(c.rng);
            h.group = 0x19;
            let z = f(h.pos[2]);
            let depth = if z < h.surf.liquid { h.surf.liquid - z } else { f(h.jump.g_down) };
            let j = &mut h.jump;
            j.h = p(depth + 1.7);
            j.takeoff = ticks(5);
            j.bottom784 = Pf::b(0x3f19_999a);
            (j.f_apex, j.f_hold, j.f_land) = (Pf::f(18.0), Pf::f(30.0), Pf::f(12.0));
            j.hmax = p(depth + 1.75);
            j.hmin = j.h;
            j.ramp = ticks(19) as i16;
            if j.fallover_h == Pf::ZERO { j.fallover_h = j.h * Pf::b(0x3fc0_0000); }
            if play { h.set_anim(c.anim, c.rng, blend(5), 7, 0); }
            None
        }
        0x7b => {
            // L00 0x226724 (level 15 also splashes lava: the burn death's fire 0x217450 at the liquid level).
            h.group = 0x19;
            h.f15d4 = 0;
            if h.idle.level == 0xf {
                let floor = h.surf.liquid;
                super::fx::burn_fire(h, c.rng, floor);
            }
            h.no_vel_clamp = 10000;
            h.surf.events.push(SurfaceEvent::Sound(9));
            h.idle.blink_period = 0x68;
            if play { h.set_anim(c.anim, c.rng, blend(8), 0x71, 0); }
            None
        }
        _ => None,
    }
}

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(h: &mut Hero, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng) -> bool {
    match h.state {
        0x2f => {
            // L00 0x21c438: the slide move, then gravity (in the air from the displacement, 25·dt²; on the
            // ground vel.z = −150·dt²).
            slide_move(h, env);
            if h.air_ticks != 0 {
                h.gravity_from(h.disp[2], DT2 * Pf::b(0x41c8_0000));
            } else {
                h.vel[2] = Pf::ZERO;
                h.gravity_from(Pf::ZERO, DT2 * Pf::b(0x4316_0000));
            }
        }
        0x31 => sinking_floor(h, env, &*anim, rng),
        0x68 => {
            // L00 0x21f710: held by the liquid, sinking at 1 u/s, 0.25 u/s once deeper than (n + 1)/2.
            h.vel = V0;
            let depth = h.surf.liquid - f(h.pos[2]);
            let rate = if (h.surf.sink_jumps + 1) as f32 * 0.5 < depth { DTF * 0.25 } else { DTF };
            h.vel[2] = p(-rate);
        }
        0x69 => h.phys_jump(env),
        0x7b => {
            // L00 0x21f77c: sinking at 1 u/s.
            h.vel = V0;
            h.vel[2] = p(-DTF);
        }
        _ => return false,
    }
    true
}

/// Per-state transitions.
pub(super) fn transitions(h: &mut Hero, c: &mut Ctx) {
    match h.state {
        // The walk's case (2 / 0x2f / 0x73 / 0x7e share it).
        0x2f => h.tr_walk(c),
        0x31 => {
            // 0x244778: off the surface once 0x13f530 has run out: the loop sound stops, idle, the momentum =
            // the last platform step 0x13f490.
            if h.surf.f0633 != 0 || h.f530 != 0 { return; }
            if h.surf.voice != -1 { h.surf.events.push(SurfaceEvent::StopLoop(h.surf.voice)); }
            h.surf.voice = -1;
            if h.set_state(c, 0, false) {
                let seq = h.idle_seq();
                h.set_anim(c.anim, c.rng, blend(17), seq, 0);
                h.momentum = h.plat_applied;
            }
        }
        0x68 => {
            // L00 0x22be90: ✕ gets out (at most twice in a row, later each time); the fade.
            let n = h.surf.sink_jumps;
            if n < 2 && ticks(15) + n * ticks(25) < h.timer && c.env.pad.pressed_within(crate::pad::button::CROSS, ticks(7)).is_some() {
                h.set_state(c, 0x69, true);
            }
            sink_fade(h, 170);
        }
        0x69 => h.tr_jump(c),
        0x7b => sink_fade(h, 150),
        _ => {}
    }
}

/// The liquid states' end (L00 0x22bf00 / 0x22bf30): below the level − 1.2, or after `n` ticks, the death fade.
fn sink_fade(h: &mut Hero, n: i32) {
    if f(h.pos[2]) < h.surf.liquid - 1.2 || ticks(n) < h.timer {
        // 0x211250 = 0x2319b0 (super::damage); called every tick in the game, once here (0x141401 latches).
        if h.fell_out == 0 { super::damage::death_fade(h); }
    }
}

/// Plays the queued surface sounds through the hero's sound layer, in queue order (their pitch-bend draws land
/// where the game's calls make them): [`SurfaceEvent::Sound`] with `HeroSounds::voice(id, 0)`, the loop with
/// `voice(id, 4)` into the slot-2 voice 0x141570 (−1 when it got no slot: the physics asks again next tick, as the
/// game does), the stop with `HeroSounds::release`. Called right after the physics and the surface reaction.
pub(super) fn flush(h: &mut Hero, moby: &crate::moby_runtime::Moby, sounds: &mut dyn super::HeroSounds, rng: &mut Rng) {
    for ev in std::mem::take(&mut h.surf.events) {
        match ev {
            SurfaceEvent::Sound(id) => {
                sounds.voice(moby, id, 0, rng);
            }
            SurfaceEvent::LoopSound(id) => h.surf.voice = sounds.voice(moby, id, 4, rng),
            SurfaceEvent::StopLoop(slot) => sounds.release(moby, slot),
        }
    }
}

/// After the transitions: [`flush`], then the slot-2 part of `0x2283a8` (the end of 0x242930 on a group change
/// releases the hero's sound slots 0x141568..; slot 2 is the sinking floor's loop, e.g. when a hit or a death ends
/// 0x31 without its own stop).
pub(super) fn after_transitions(h: &mut Hero, moby: &crate::moby_runtime::Moby, sounds: &mut dyn super::HeroSounds, rng: &mut Rng, group: i32) {
    flush(h, moby, sounds, rng);
    if h.group != group && h.surf.voice != -1 {
        sounds.release(moby, h.surf.voice);
        h.surf.voice = -1;
    }
}

// ------------------------------------------------------------------------------------------------
// 0x31: the sinking floor (0x23a7cc).

fn sinking_floor(h: &mut Hero, _env: &Env, anim: &dyn super::AnimCtl, rng: &mut Rng) {
    // The loop sound in slot 2 (level 15: 0x1d), played while the slot's voice is free.
    if h.surf.voice == -1 {
        h.surf.events.push(SurfaceEvent::LoopSound(if h.idle.level == 0xf { 0x1d } else { 6 }));
        h.surf.voice = 0;
    }
    // Level ≠ 15: the sand. First `0x22b140(4, 2)`: four type-34 bubbles at Ratchet's joint points 0x17 / 0x16 (their
    // pop level the hero's z + 0.4). Then one puff of sand `PartType47Spawn(randf(37800, 75600), pos, vel)` 0x286cb0
    // around the feet (±0.25, −0.1..0.4) moving with 0.9 of the displacement ± 0.7 u/s, rising 1..2 u/s.
    if h.idle.level != 0xf {
        super::fx::bubbles_at(h, rng, Some(anim), 4, 2);
        let dtf = DTF;
        let p0 = to_f32x3(h.pos);
        let x = p0[0] + rng.randf(-0.25, 0.25);
        let y = p0[1] + rng.randf(-0.25, 0.25);
        let z = p0[2] + rng.randf(-0.1, 0.4);
        let d = to_f32x3(h.disp);
        let (vx, vy) = (d[0] * 0.9 + rng.randf(dtf * -0.7, dtf * 0.7), d[1] * 0.9 + rng.randf(dtf * -0.7, dtf * 0.7));
        let vz = d[2] * 0.9 + rng.randf(dtf, dtf + dtf);
        let size = rng.randf(f32::from_bits(0x4713_a800), f32::from_bits(0x4793_a800));
        let w = h.pos[3].to_f32();
        super::fx::dust(h, rng, size, [x, y, z, w], [vx, vy, vz, h.disp[3].to_f32() * 0.9]);
    }
    let push = to_f32x3(h.platform);
    let pl = (push[0] * push[0] + push[1] * push[1] + push[2] * push[2]).sqrt();
    h.target_yaw = if DTF * 0.5 < pl { Pf::f(push[1].atan2(push[0])) } else { h.rot[2] };
    let yaw0 = h.rot[2];
    h.turn_to(SCALE64 * p(pl * 0.125), SCALE64 * Pf::b(0x3e4c_cccd), DT * Pf::b(0x4086_0a92));
    // FastDiffRots 0x222100: the turn's size (≥ 0, so the body only ever rolls one way).
    let mut dyaw = f(crate::pad::fast_diff_rots(h.rot[2], yaw0));
    h.vel = V0;
    h.momentum_decay(DT2 * Pf::b(0x40a0_0000));
    let fall = f(h.disp[2]) - DT2F * 50.0;
    h.vel[2] = p(fall.min(DT2F * -40.0));
    let cap = DTF * 1.396_263_4;
    if cap < dyaw { dyaw = cap; }
    dyaw *= 9.0;
    // The body rolls into the turn; after 100 ticks it tilts to the flow's pitch 0x13fd24.
    let (mut a, mut v) = (h.rot[0], h.surf.tilt_vel[0]);
    turn_spring(p(dyaw * 0.5), Pf::b(0x3be5_6042), Pf::b(0x3e2e_147b), DT * Pf::b(0x3f9c_61aa), &mut a, &mut v, 0);
    (h.rot[0], h.surf.tilt_vel[0]) = (a, v);
    if ticks(100) < h.timer {
        let (mut a, mut v) = (h.rot[1], h.surf.tilt_vel[1]);
        turn_spring(p(-h.surf.flow[1]), SCALE64 * Pf::b(0x3d0f_5c29), SCALE64 * Pf::b(0x3e99_999a), DT * Pf::b(0x3ff5_be0b), &mut a, &mut v, 0);
        (h.rot[1], h.surf.tilt_vel[1]) = (a, v);
    }
    // The lean (0x23ab60..0x23abbc): the springs of records 1 / 2 / 3 (0x22b5e0 / 0x22b5f8 / 0x22b610) and record 1's
    // (the neck's) x / z targets from the turn: 3.5·Δ, 2.1·Δ.
    {
        use super::idle::joint::{HEAD, NECK, REC2};
        let j = &mut h.idle.joints;
        (j[NECK].k, j[NECK].d) = (f32::from_bits(0x3c13_74bc), f32::from_bits(0x3e61_47ae));
        (j[REC2].k, j[REC2].d) = (f32::from_bits(0x3d23_d70a), f32::from_bits(0x3e4c_cccd));
        (j[HEAD].k, j[HEAD].d) = (f32::from_bits(0x3ca3_d70a), f32::from_bits(0x3e4c_cccd));
        j[NECK].target[2] = dyaw * f32::from_bits(0x4006_6666);
        j[NECK].target[0] = dyaw * 3.5;
    }
}

// ------------------------------------------------------------------------------------------------
// 0x2f: the slide.

/// The slide move (L00 0x2167d0; the combo 0x13 on a slippery floor runs it instead of its own physics): the
/// stick accelerates along the target yaw by 5.9·target speed u/s through a capped velocity change (the cap:
/// 2.45·target speed + 2·(how much the stick opposes the motion) ·dt², 1.1·dt² with the stick released, 13·dt²
/// above 5.9 u/s), the slope pulls downhill (8·slope/45° u/s, change ≤ 8.5·slope/45°·dt²), the wall check, a
/// pitch step into a slope scales the velocity by cos(pitch), and the feet snap to the ground within 0.15.
pub(super) fn slide_move(h: &mut Hero, env: &Env) {
    h.stick_target(env, Pf::ONE);
    h.turn_to(SCALE64 * Pf::b(0x3ca3_d70a), SCALE64 * Pf::b(0x3e4c_cccd), DT * Pf::b(0x406a_927f));
    if h.state == 0x13 {
        if h.stick_mag < Pf::b(0x3e4c_cccd) { h.target_yaw = h.rot[2]; }
        h.target_speed = Pf::b(0x3ecc_cccd);
    } else if h.state == 0x15 {
        h.target_speed = Pf::ZERO;
    }
    let ts = f(h.target_speed);
    let ty = f(h.target_yaw);
    let k = DTF * 5.9;
    let a0 = [ty.cos() * ts * k, ty.sin() * ts * k, 0.0];
    let mut a = a0;
    let mut push0 = [0.0f32; 3];
    if Pf::ZERO < h.push_strength {
        h.drag(Pf::b(0x3f33_3333), Pf::ZERO);
        push0 = to_f32x3(h.push);
        a = [a[0] + push0[0], a[1] + push0[1], a[2] + push0[2]];
        h.push = V0;
    }
    let vel = to_f32x3(h.vel);
    let len2 = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1]).sqrt();
    let len3 = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    // How much the stick opposes the motion (0..1).
    let mut opp = -(vel[0] * a[0] + vel[1] * a[1]);
    if 0.0 < opp { opp = (opp / len2(a)) / len2(vel); }
    opp = opp.clamp(0.0, 1.0);
    let mut acc = DT2F * 2.45 * ts + (DT2F + DT2F) * opp;
    if env.pad.no_direction { acc = DT2F * 1.1; }
    let eff_len = f(h.eff_len);
    if h.state == 0x13 && h.timer < ticks(15) && eff_len < DTF * 3.0 { acc = DT2F * 11.0; }
    if DTF * 5.9 < eff_len { acc = DT2F * 13.0; }
    if Pf::ZERO < h.push_strength {
        let mut g = -(a0[0] * push0[0] + a0[1] * push0[1] + a0[2] * push0[2]);
        let la = len3(a0);
        if 0.0 < la { g /= la; }
        let lp = len2(push0);
        let mut m = DT2F * 3.0 * ((lp - g) / lp);
        if f(h.stick_mag) < 0.3 && m < DT2F * 3.0 { m = DT2F * 3.0; }
        if acc < m { acc = m; }
    }
    // The velocity change toward the stick target, capped.
    let eh = to_f32x3(h.eff_h);
    let mut d = [a[0] - eh[0], a[1] - eh[1], 0.0];
    let ld = len3(d);
    if acc < ld { d = d.map(|x| x * (acc / ld)); }
    let mut v = [eh[0] + d[0], eh[1] + d[1], eh[2] + d[2]];
    // The downhill pull.
    let slope = f(h.slope);
    if FIVE_DEG < slope {
        let n = to_f32x3(h.ground_normal);
        let yaw = n[1].atan2(n[0]);
        let s = DTF * 8.0 * slope / std::f32::consts::FRAC_PI_4;
        let lim = DT2F * 8.5 * slope / std::f32::consts::FRAC_PI_4;
        let mut e = [yaw.cos() * s - v[0], yaw.sin() * s - v[1], 0.0];
        let le = len3(e);
        if lim < le { e = e.map(|x| x * (lim / le)); }
        v = [v[0] + e[0], v[1] + e[1], v[2] + e[2]];
    }
    h.vel = [p(v[0]), p(v[1]), p(v[2]), h.eff_h[3]];
    // 0x13f700: the velocity along the stick direction, 1 = the full 5.9 u/s.
    let la = len3(a);
    let mut sl = v[0] * a[0] + v[1] * a[1];
    if 0.0 < la { sl /= la * (DTF * 5.9); }
    h.surf.slide = sl;
    h.wall_check(env, 0);
    let pitch = f(h.pitch);
    if h.surf.last_pitch < 0.017_453_292 && FIVE_DEG < pitch {
        let c = pitch.cos();
        let w = to_f32x3(h.vel);
        h.vel = [p(w[0] * c), p(w[1] * c), p(w[2] * c), h.vel[3]];
    }
    h.surf.last_pitch = pitch;
    if (f(h.pos[2]) - f(h.ground_z)).abs() < 0.15 { h.pos[2] = h.ground_z; }
}

// ------------------------------------------------------------------------------------------------
// The seams in the shared ground / walk / idle code.

/// 0x140632: on a slippery floor this tick (the idle fidgets pause, the idle sequence is 0x6e: L00 0x205000).
pub(super) fn slippery(h: &Hero) -> bool { h.f0632 != 0 }

/// The idle entry (L00 0x222610) after the momentum copy: sliding faster than 0.5 u/s on a slippery floor → 0x2f.
pub(super) fn idle_entry(h: &mut Hero, c: &mut Ctx) -> Option<bool> {
    if h.f0632 != 0 && DT * Pf::b(0x3f00_0000) < len2(h.momentum) {
        h.set_state(c, 0x2f, true);
        return Some(false);
    }
    None
}

/// The stop entry (L00 0x223850) after the momentum copy: a slippery floor → 0x2f.
pub(super) fn stop_entry(h: &mut Hero, c: &mut Ctx) -> Option<bool> {
    if h.f0632 != 0 {
        h.set_state(c, 0x2f, true);
        return Some(false);
    }
    None
}

/// The walk entry's slippery branch (L00 0x2233cc): on a slippery floor, or entering 0x2f: speed ≤ 7.7 u/s, no
/// vertical velocity, |vel| ≤ 7.7·dt, state 0x2f, 0x13f704 = 0, anim 0x37 (set even without `play`).
pub(super) fn slippery_walk_entry(h: &mut Hero, c: &mut Ctx, id: i32) -> Option<Option<bool>> {
    if h.f0632 == 0 && id != 0x2f { return None; }
    let cap = DT * Pf::b(0x40f6_6666);
    if cap < h.speed { h.speed = cap; }
    h.vel[2] = Pf::ZERO;
    super::common::clamp_len_2745f0(&mut h.vel, cap);
    h.state = 0x2f;
    h.surf.last_pitch = 0.0;
    h.set_anim(c.anim, c.rng, blend(8), 0x37, 0);
    Some(None)
}

/// Capsule sizing (L00 0x211380): grounded on a slippery floor the capsule's targets are top 0.8, bottom 0.9
/// (after the jump / fall / crouch / swim cases). True when it set them.
pub(super) fn slippery_capsule(h: &mut Hero) -> bool {
    if h.f0632 == 0 || h.grounded_ticks == 0 { return false; }
    h.cap_top_target = Pf::b(0x3f4c_cccd);
    h.cap_bottom_target = Pf::b(0x3f66_6666);
    true
}

/// The ground states' drag (0x233850): (0.5, 2·dt), on a slippery floor (0.7, 0) (L00 0x217970 case 0).
pub(super) fn ground_drag(h: &mut Hero) {
    if h.f0632 == 0 { h.drag(Pf::b(0x3f00_0000), DT + DT); } else { h.drag(Pf::b(0x3f33_3333), SCALE60 * Pf::ZERO); }
}

/// The crouch's brake: 6.48·dt², on a slippery floor 1.7·dt².
pub(super) fn crouch_decel(h: &Hero) -> Pf { if h.f0632 == 0 { DT2 * Pf::b(0x40cf_5c29) } else { DT2 * Pf::b(0x3fd9_999a) } }

/// The idle transitions after the crouch (L00 0x22a5d0..): a slippery floor steeper than 5° → 0x2f.
pub(super) fn idle_slide(h: &mut Hero, c: &mut Ctx) -> bool {
    if h.f0632 != 0 && Pf::b(0x3db2_b8c2) < h.slope {
        h.set_state(c, 0x2f, true);
        return true;
    }
    false
}

/// The walk transitions' slippery part (L00 0x22b608): the slide's anims, then 2 / 0x7e ↔ 0x2f by 0x140632.
pub(super) fn walk_surface(h: &mut Hero, c: &mut Ctx) {
    if h.f0632 != 0 && h.state == 0x2f {
        let d = f(len2(h.disp));
        let v = c.anim.view();
        let (seq, busy) = (v.seq_b, v.blending());
        let idle = h.idle_seq();
        let stick = f(h.stick_mag);
        let slope = f(h.slope);
        let still = stick < 0.17 && (seq == 0x37 || (seq == idle && FIVE_DEG < slope)) && ticks(8) < h.f4fc && !busy;
        if still {
            h.set_anim(c.anim, c.rng, blend(14), 0x6d, 0);
        } else if 0.3 < stick && seq != 0x37 && !busy {
            h.set_anim(c.anim, c.rng, blend(14), 0x37, 0);
        } else if seq == 0x6d && !busy && stick < 0.17 && slope < FIVE_DEG && d < DTF * 1.7 {
            h.set_anim(c.anim, c.rng, blend(20), idle, 0);
        }
    }
    if h.state == 2 || h.state == 0x7e {
        if h.f0632 != 0 { h.set_state(c, 0x2f, true); }
    } else if h.state == 0x2f && h.f0632 == 0 {
        h.set_state(c, 2, true);
    }
}

/// The tail of the walk transitions for 0x2f (L00 0x22baf0): stop when nothing is held and it barely moves on
/// flat ground; the playback speed of 0x37 (2·(1 − slide factor), 0.75..1.7) and 0x6d (10·|eff|, 0.3..1.1).
pub(super) fn walk_tail(h: &mut Hero, c: &mut Ctx) {
    if h.state != 0x2f { return; }
    if c.env.pad.no_direction && f(len2(h.vel)) < DTF * 0.05 && f(h.slope) < FIVE_DEG && h.set_state(c, 0, false) {
        let seq = h.idle_seq();
        h.set_anim(c.anim, c.rng, blend(17), seq, 0);
    }
    let seq = c.anim.view().seq_b;
    if seq == 0x37 {
        h.anim_speed = p(((1.0 - h.surf.slide) * 2.0).clamp(0.75, 1.7));
    } else if seq == 0x6d {
        h.anim_speed = p((f(h.eff_len) * 10.0).clamp(0.3, 1.1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::testkit::*;
    use crate::hero::HeroTick;
    use crate::pad::{button, PadInput};
    use rc_formats::collision::Collision;
    use std::collections::BTreeMap;

    /// A ramp over cells `x0..x1 × y0..y1` (cell units of 4): height `z(x)` at each cell edge, face type `ty`,
    /// each quad in every cell layer its heights overlap.
    fn ramp(z: impl Fn(f32) -> f32, ty: impl Fn(i16) -> u8, x0: i16, x1: i16, y0: i16, y1: i16) -> Collision {
        type Faces = (Vec<[f32; 3]>, Vec<([u8; 4], u8)>);
        let mut cells: BTreeMap<(i16, i16, i16), Faces> = BTreeMap::new();
        for cx in x0..x1 {
            for cy in y0..y1 {
                let (x, y) = (cx as f32 * 4.0, cy as f32 * 4.0);
                let (za, zb) = (z(x), z(x + 4.0));
                let (lo, hi) = ((za.min(zb) / 4.0).floor() as i16, (za.max(zb) / 4.0).floor() as i16);
                for cz in lo..=hi {
                    let e = cells.entry((cx, cy, cz)).or_default();
                    let b = e.0.len() as u8;
                    e.0.extend_from_slice(&[[x, y, za], [x, y + 4.0, za], [x + 4.0, y + 4.0, zb], [x + 4.0, y, zb]]);
                    e.1.push(([b, b + 1, b + 2, b + 3], ty(cx)));
                }
            }
        }
        mesh(cells.into_iter().map(|((cx, cy, cz), (v, q))| cell([cx, cy, cz], &v, &q)).collect())
    }

    fn runner(p: [f32; 3], level: i32) -> Runner {
        let mut r = Runner::new(p, 0.0);
        r.hero.idle.level = level;
        r
    }

    fn states(r: &Runner) -> Vec<i32> {
        let mut v: Vec<i32> = Vec::new();
        for l in &r.log {
            if v.last() != Some(&l.0) { v.push(l.0); }
        }
        v
    }

    #[test]
    fn level_rules() {
        let novalis = rules(1);
        for id in [0, 2, 4, 8, 9, 0xb, 0xc, 0xd, 0xe] { assert!(novalis.handles(id), "{id:#x}"); }
        for id in [1, 3, 5, 6, 7, 0xa, 0xf, -1, 31] { assert!(!novalis.handles(id), "{id:#x}"); }
        assert_eq!(rules(3), novalis, "level 3 compiles level 1's reaction");
        assert_eq!(rules(-1), novalis, "unknown level: level01's build");
        assert_eq!(rules(0), SUPERSET);
        assert!(rules(5).handles(5) && rules(16).handles(6) && !SUPERSET.handles(5));
        assert!(rules(14).magnet_always && !novalis.magnet_always);
        assert!(rules(6).clank_burn && !rules(6).handles(0) && !rules(2).handles(4));
        // Rule 3 (and with it 0xb's 0x7b) on the quicksand / lava levels.
        for n in [0, 2, 4, 5, 7, 9, 10, 13, 15, 17, 18] { assert!(rules(n).handles(3), "{n}"); }
    }

    /// The flags of the reaction follow the level's rules: surface 7 is slippery on level 0 and nothing on
    /// level 1; 9, 5 and 0xd set theirs where handled; the footstep class survives the clear.
    #[test]
    fn reaction_flags_per_level() {
        let coll = Collision::default();
        let mut r = runner([10.0, 10.0, 5.0], 0);
        let mut flags = |level: i32, sid: i16| {
            r.hero.idle.level = level;
            r.hero.surface_id = sid;
            r.hero.footstep = 2;
            let (h, _) = (&mut r.hero, ());
            let env = Env { coll: &coll, pad: &r.pad, cam_yaw: r.cam_yaw, cam_rows: r.cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
            h.surface_reaction(&env, &mut r.anim, &mut r.rng);
            assert_eq!(h.surface_id, -1);
            assert_eq!(h.footstep, 2);
            let s = &h.surf;
            (h.f0632, s.f063e, s.f0638, s.f063c, s.f0633)
        };
        assert_eq!(flags(0, 7), (1, 0, 0, 0, 0));
        assert_eq!(flags(1, 7), (0, 0, 0, 0, 0));
        assert_eq!(flags(1, 9), (0, 1, 0, 0, 0));
        assert_eq!(flags(5, 5), (0, 0, 1, 0, 0));
        assert_eq!(flags(1, 5), (0, 0, 0, 0, 0));
        assert_eq!(flags(12, 0xd), (0, 0, 0, 1, 0));
        assert_eq!(flags(15, 0xd), (0, 0, 0, 0, 0));
        assert_eq!(flags(-1, -1), (0, 0, 0, 0, 0));
    }

    /// A slope steeper than 50° is not ground: Ratchet put on a 54° ramp slides down it (gravity against the
    /// capsule) to the flat floor at its foot (the ported probe, capsule and steep-wall stop; unchanged by this
    /// package).
    #[test]
    fn steep_ramp_slides_him_down() {
        let k = 1.375; // 54°
        let coll = ramp(|x| if x <= 32.0 { 20.0 } else { 20.0 + (x - 32.0) * k }, |_| 0x21, 0, 16, 0, 8);
        let mut r = runner([48.0, 16.0, 20.0 + 16.0 * k], -1);
        r.run(&coll, PadInput::neutral(), 150);
        let s = states(&r);
        assert_eq!(s, vec![0], "standing (the height to the face stays below 0.4)");
        let p = r.hero.position();
        assert!(p[0] < 33.0 && (p[2] - 20.0).abs() < 0.05, "ended at {p:?}: {s:?}");
        // Monotonic downhill slide.
        assert!(r.log.windows(2).all(|w| w[1].2[2] <= w[0].2[2] + 1e-4), "went up");
    }

    /// A slippery ramp (surface 7, level 0): standing on it is the slide 0x2f, which pulls him downhill faster
    /// and faster; on level 1 the same ramp (14°, walkable) holds him.
    #[test]
    fn slippery_ramp_slides_him_down() {
        let k = 0.25; // 14°
        let coll = ramp(|x| 20.0 + x * k, |_| 0x27, 0, 16, 0, 8);
        let mut r = runner([48.0, 16.0, 20.0 + 48.0 * k], 0);
        r.run(&coll, PadInput::neutral(), 90);
        let s = states(&r);
        assert_eq!(s, vec![0x2f], "{s:?}");
        assert_eq!(r.hero.group, 1);
        let xs: Vec<f32> = r.log.iter().map(|l| l.2[0]).collect();
        assert!(xs[89] < xs[30] - 1.0, "slid only {} → {}", xs[30], xs[89]);
        let v1 = xs[40] - xs[41];
        let v2 = xs[88] - xs[89];
        assert!(v2 > v1 && v1 > 0.0, "accelerating downhill: {v1} then {v2}");
        let mut r = runner([48.0, 16.0, 20.0 + 48.0 * k], 1);
        r.run(&coll, PadInput::neutral(), 90);
        assert_eq!(states(&r), vec![0]);
        assert!((r.hero.position()[0] - 48.0).abs() < 0.01);
    }

    /// Flat ice: the walk becomes the slide (anim 0x37), it keeps sliding after the stick is released and stops
    /// into idle once slower than 0.05 u/s; the idle sequence on ice is 0x6e.
    #[test]
    fn flat_ice_walk_and_stop() {
        let coll = ramp(|_| 20.0, |_| 0x27, 0, 16, 0, 16);
        let mut r = runner([16.0, 32.0, 20.0], 0);
        r.run(&coll, PadInput::neutral().stick(0.0, -1.0), 60);
        assert_eq!(states(&r), vec![0, 0x2f]);
        assert_eq!(r.anim.view().seq_b, 0x37);
        let x60 = r.hero.position()[0];
        assert!(x60 > 17.0, "walked to {x60}");
        r.run(&coll, PadInput::neutral(), 20);
        assert_eq!(r.hero.state, 0x2f, "still sliding");
        assert!(r.hero.position()[0] > x60 + 0.3, "coasting");
        r.run(&coll, PadInput::neutral(), 400);
        assert_eq!(r.hero.state, 0);
        assert_eq!(r.hero.idle_seq(), 0x6e);
    }

    /// A jump off ice keeps the slide's momentum: the takeoff windup runs the slide move (the superset build L00
    /// 0x217970, gated on 0x13f7f8 keep_speed) rather than rebuilding the velocity from the ground speed 0x13f4e4,
    /// which the slide never updates. Level 0 is the slippery build; the level 1 reaction ignores surface 7, so a
    /// jump there takes the plain branch and a jump here must not drop him to the stale ground speed.
    #[test]
    fn jump_off_ice_keeps_the_slide_momentum() {
        let coll = ramp(|_| 20.0, |_| 0x27, 0, 24, 0, 16);
        let mut r = runner([16.0, 32.0, 20.0], 0);
        // Slide forward, building speed (the walk on ice becomes the slide 0x2f).
        let fwd = PadInput::neutral().stick(0.0, -1.0);
        r.run(&coll, fwd, 60);
        assert_eq!(r.hero.state, 0x2f, "sliding on the ice");
        let v0 = f(len2(r.hero.vel));
        assert!(v0 > 0.03, "built up speed: {v0}");
        // The slide never writes ground_speed, so it lags well behind the slide's velocity.
        assert!(f(r.hero.speed) < 0.5 * v0, "ground_speed {} vs slide {v0}", f(r.hero.speed));

        // Jump; the windup tick must keep the slide velocity, not reset it to ground_speed.
        r.tick(&coll, fwd.press(button::CROSS));
        assert_eq!(r.hero.group, 4, "the jump group");
        assert_eq!(r.hero.jump.keep_speed, 1, "keep_speed latched from the slippery floor");
        r.tick(&coll, fwd);
        let v1 = f(len2(r.hero.vel));
        assert!(v1 > 0.8 * v0, "the jump kept the slide speed: {v0} -> {v1}");

        // The same ramp on level 1 (surface 7 ignored): no slide, so the jump is the plain one (keep_speed 0).
        let mut r = runner([16.0, 32.0, 20.0], 1);
        r.run(&coll, fwd, 60);
        assert_eq!(r.hero.state, 2, "level 1 walks the ice");
        r.tick(&coll, fwd.press(button::CROSS));
        assert_eq!(r.hero.jump.keep_speed, 0, "not latched without the slippery floor");
    }

    /// The hero's sounds, recorded: `('v', index, flags)` plays (slot 5 when `free`, else none), `('r', slot, 0)`
    /// releases.
    struct SoundRec {
        log: Vec<(char, i32, u32)>,
        free: bool,
    }
    impl crate::hero::HeroSounds for SoundRec {
        fn anim_advanced(&mut self, _: &crate::moby_runtime::Moby, _: &super::super::AnimView, _: &super::super::AnimView, _: &mut Rng) {}
        fn voice(&mut self, _: &crate::moby_runtime::Moby, index: i32, flags: u32, _: &mut Rng) -> i32 {
            self.log.push(('v', index, flags));
            if self.free { 5 } else { -1 }
        }
        fn release(&mut self, _: &crate::moby_runtime::Moby, slot: i32) { self.log.push(('r', slot, 0)); }
    }

    /// [`Runner::tick`] with a sound layer.
    fn tick_sounds(r: &mut Runner, coll: &Collision, input: PadInput, snd: &mut SoundRec) {
        r.pad.update(Some(&input.bytes()), false);
        let env = Env { coll, pad: &r.pad, cam_yaw: r.cam_yaw, cam_rows: r.cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
        crate::hero::hero_update_with_sounds(&mut r.hero, &mut r.moby, &env, &mut r.anim, &mut r.rng, snd);
        r.log.push((r.hero.state, r.hero.timer, r.hero.position()));
    }

    /// The sinking floor (surface 4, level 1): grounded on it → 0x31 (group 0x10, anim 100, the momentum = the
    /// displacement) and its loop sound 6 (flags 4) in slot 2, asked for again each tick while no slot is free;
    /// off it (and 0x13f530 run out) → idle, the loop released, the momentum = the last platform step.
    #[test]
    fn sinking_floor_state() {
        let coll = ramp(|_| 20.0, |cx| if cx < 8 { 0x24 } else { 0x21 }, 0, 16, 0, 8);
        let mut r = runner([16.0, 16.0, 20.0], 1);
        let mut snd = SoundRec { log: Vec::new(), free: false };
        for _ in 0..30 { tick_sounds(&mut r, &coll, PadInput::neutral(), &mut snd); }
        assert_eq!(states(&r), vec![0x31]);
        assert_eq!((r.hero.group, r.anim.view().seq_b), (0x10, 100));
        // The lean's springs (0x22b5e0 / 0x22b5f8 / 0x22b610); no turn, so no lean.
        {
            use crate::hero::idle::joint::{HEAD, NECK, REC2};
            let j = &r.hero.idle.joints;
            assert_eq!([j[NECK].k, j[REC2].k, j[HEAD].k], [f32::from_bits(0x3c13_74bc), f32::from_bits(0x3d23_d70a), f32::from_bits(0x3ca3_d70a)]);
            assert_eq!(j[NECK].cur, [0.0; 3]);
        }
        assert!(snd.log.len() > 20 && snd.log.iter().all(|&e| e == ('v', 6, 4)), "retried while no slot: {:?}", snd.log);
        assert_eq!(r.hero.surf.voice, -1);
        snd.free = true;
        snd.log.clear();
        for _ in 0..3 { tick_sounds(&mut r, &coll, PadInput::neutral(), &mut snd); }
        assert_eq!((snd.log.clone(), r.hero.surf.voice), (vec![('v', 6, 4)], 5), "played once it got a slot");
        assert!(r.hero.surf.events.is_empty());
        // Held by 0x13f530 (the flow class 679 sets it): no way off.
        r.hero.f530 = 5;
        r.hero.pos[0] = Pf::f(48.0);
        for _ in 0..3 { tick_sounds(&mut r, &coll, PadInput::neutral(), &mut snd); }
        assert_eq!(r.hero.state, 0x31);
        for _ in 0..5 { tick_sounds(&mut r, &coll, PadInput::neutral(), &mut snd); }
        assert_eq!(r.hero.state, 0);
        assert_eq!(r.hero.surf.voice, -1);
        assert_eq!(snd.log.last(), Some(&('r', 5, 0)), "the loop is released");
    }

    /// Sinking liquid (surface 3, level 0) over a floor 3 below: falling in is 0x68 (group 0x19, anim 0x71), ✕
    /// after 15 ticks jumps out (0x69, the jump system), falling back in is the second try (0x1409b0 = 1); with
    /// no health the liquid is 0x7b, which sinks at 1 u/s through everything (no capsule) to the fade.
    #[test]
    fn sinking_liquid_states() {
        let coll = ramp(|x| if x < 32.0 { 20.0 } else { 17.0 }, |cx| if cx < 8 { 0x23 } else { 0x21 }, 0, 12, 0, 8);
        let mut r = runner([16.0, 16.0, 21.0], 0);
        let x = |t: usize| if (40..42).contains(&t) { PadInput::neutral().press(button::CROSS) } else { PadInput::neutral() };
        let mut snd = SoundRec { log: Vec::new(), free: true };
        for t in 0..160 { tick_sounds(&mut r, &coll, x(t), &mut snd); }
        let s = states(&r);
        assert_eq!(s, vec![0, 6, 0x68, 0x69, 0x68], "{s:?}");
        assert_eq!(r.hero.surf.sink_jumps, 1);
        assert_eq!((r.hero.group, r.anim.view().seq_b), (0x19, 0x71));
        assert_eq!(r.hero.surf.liquid, 20.0);
        assert_eq!(snd.log, vec![('v', 9, 0), ('v', 10, 0), ('v', 9, 0)], "in, out, in again");
        // No health: 0x7b, through the liquid and the floor below at 1 u/s, the fade 1.2 below the top.
        let mut r = runner([16.0, 16.0, 22.5], 0);
        while r.hero.state != 6 { r.tick(&coll, PadInput::neutral()); }
        // (On the ground a hero without health dies, 0x3d; in the air he is taken by the liquid.)
        r.hero.health = 0;
        let mut ticks = 0;
        while r.hero.fell_out == 0 && ticks < 200 {
            assert_ne!(r.tick(&coll, PadInput::neutral()), HeroTick::OutOfBounds);
            ticks += 1;
        }
        assert!(states(&r).contains(&0x7b), "{:?}", states(&r));
        assert!(r.hero.position()[2] < 20.0 - 1.2 + 0.02, "faded at {:?}", r.hero.position());
    }

    /// The surface Ratchet's own collision passes through ([`PASS_SURFACE`], per level): 0 (water) on level01 and
    /// the unknown level (Novalis unchanged), 3 (the quicksand) on level 2, 0xd (the deadly liquid) on 6 and 0xe.
    /// (Aridia's is visible: `tests/hero/hero_surfaces.rs`; on 6 and 0xe the deadly-liquid death 0x7f already uses
    /// 0xd24 on every level and their data has no surface-0xd face, so nothing there shows it.)
    #[test]
    fn pass_surface_per_level() {
        assert_eq!((pass_surface(1), pass_surface(-1), pass_surface(2), pass_surface(6), pass_surface(14)), (0, 0, 3, 0xd, 0xd));
        assert_eq!(PASS_SURFACE.iter().filter(|&&s| s == 0).count(), 16);
        assert_eq!((pass_flags(1).bits(), pass_flags(2).bits(), pass_flags(6).bits(), pass_flags(14).bits()), (0x24, 0x324, 0xd24, 0xd24));
    }
}
