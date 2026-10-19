//! **The Morph-o-Ray** (item 21, class 185 — the level01 item table 0x179f40 and `lvl.vtbl` 0x20bb00; the same code on
//! every level): the item update `0x2d2450`, its search `0x2d2018`, the beam `0x2d2d08` with its reset `0x2d2c00` and
//! its draw callback `0x2d38b8` (the strands `0x2d3ce0`, the pulses `0x2d42b8`, the muzzle glow `0x2d44f8`); read from
//! the decompiler output and the disassembly (the draw callback and the pulses have no Ghidra function). The morph and
//! the chicken are the moby side: `crate::moby_update::classes::chicken` (docs/plan/hero_gameplay.md §12).
//!
//! **No ammo, no weapon-check case, holding class 1** (def +0x18 = 1: the hand-item arm's holding layer). ○ held while
//! Ratchet is in movement group 0x1413dc < 2 / 4 / 5 / 0xc (idle, walk or run, the jumps, the packs, crouch), the item's
//! draw sequence past key time 6, not being put away: **firing**. The arm comes up (`0x22ee08` when 0x1413f8 is clear);
//! from the look stance 1 Ratchet goes into the first-person stance (`SetState(0x1e, 1)`). The aim is the camera's
//! forward in first person, the item's −row 1 with the weapon raised at a wall (0x141618 = 1, `super::idle`), else
//! Ratchet's facing: `(yaw, pitch)` = `(FastArcTan(x, y), FastArcTan(|xy|, z))` at +0x48 / +0x4c. The muzzle is the
//! item's joint list 0 (+0x00).
//!
//! **The search** ([`search`], `0x2d2018`) over the target list: nothing when the line from Ratchet (at the muzzle's
//! height) to the muzzle is blocked; creatures (class type 5) with a target record, not deleted, not classes 0x58e /
//! 0x452; the aim point = position + record +0x10. Within 2.5 of the muzzle, within 60° of Ratchet's yaw, less than 2
//! above or below, with a clear line: taken at once (the aim turns to it). Else the nearest within the range 8 whose
//! aim point is within the 10° cone (`targeting::cone_miss` with the record's radius byte, the pitch plus half the
//! ground pitch) and whose line is clear or blocked only by the creature itself: the pitch turns to it, the yaw only
//! within 10°. (The pitch it writes is −elevation where the update wrote +elevation: the game's own sign, kept.)
//!
//! **The morph meter**: a new target (none held, lock +0x44 out) becomes the target (+0x10) with the lock `ticks(30)`;
//! the meter (+0x18) starts at its record's health and the full scale (+0x14) is its record's s16 +0x04 (a scale of 1
//! or less: the meter starts empty, an instant morph). Each tick with a target (or the lock) the meter falls by
//! `3·dt·(1 + 0.33·gold)` (`gp−0x54f0`); the HUD's meter (`gp−0x54e4`, element `queue_animation_update(4, 0x7533)`) shows
//! `10000 − 10000·meter / scale`. At 0 the target is **morphed** (`react::morph_target`; class 625 on level 5 first earns
//! skill point 0x13d410 through `story::award_skill_point`) and dropped. A target deleted meanwhile is dropped.
//!
//! **The beam** ([`Beam`]): twelve points `range/12` apart bending from the last tick's shape toward the aim (a lerp of
//! `0.9 − 0.5·i/12`; from point 7 on also toward the target), a ring of 20 around each (radius `0.05 + 1.184·i/12`),
//! two helix strands (radius `0.01 + 0.49·i/12`, phase `+0.2007` a tick, `0.39968` a point), sparkle pairs (type 53) at
//! the strands' ends, homing sparks along the beam (type 78, `crate::particles::type78`), and three pulses (a new one every 45 ticks)
//! running out along it at 8 u/s, growing from 0.1 to 1.5 and fading in their last 30 ticks. Its length is the target's
//! distance, else the line's (flags 0x14) up to the range 8. **The draw** ([`Beam::quads`]): the tube (FX 13, two
//! counter-scrolling layers, back faces culled, the far ring transparent), the strands (FX 14 core 0.05 wide and FX 16
//! glow 0.4 wide, colours 0x7f2020 / 0x20207f), the pulses (FX 8) and the muzzle glow (FX 0xb, 0.4 coloured and 0.1
//! white); all additive, colour 0x407f207f (gold 0x60007f00).
//!
//! **The beam's sparks** (`0x2d2d08`, 0x2d3388..0x2d35e8; `PartType78Spawn` 0x28ad08, `crate::particles::type78`):
//!
//! | address | what it does | port |
//! |---|---|---|
//! | 0x2d3388 | `randi(2)`: 1 → no sparks this tick | [`beam`] |
//! | 0x2d33a0..0x2d33c4 | the step: `pts[1] − pts[0]` at 0.2 (gp−0x549c × 0x15ed60); the start: `pts[0]` + the item's row 1 (+0xd0) at −0.25 (gp−0x5490) | [`beam`] |
//! | 0x2d33c8..0x2d34fc | the colour: `randi(5)` into the table 0x20ab10 ([`SPARK_RGBA`]) | [`beam`] |
//! | 0x2d34f8..0x2d3544 | `randf_sym(0, 15°)` twice (a1, a2); the step turned by a2 about the gravity 0x13f5e0 (`0x274ac8`), then by a1 about step × gravity (`FastVecCross` 0x2212d0) | [`beam`] |
//! | 0x2d3548..0x2d3580 | life = trunc(1.2 · beam length / 0.2), ×3 with a target (+0x10) | [`beam`] |
//! | 0x2d35a0 | spark 1: `(0.1·s, s, start, life, colour, mode 0, spin sgn, vel, target)` (s, sgn: the sparkles' draws above) | [`beam`] → `super::fx::PartSpawn::Spark78` |
//! | 0x2d35e4 | spark 2: `(0.07·s, 0.7·s, start, life, 0x7f7f7f7f, mode 1, −sgn, vel, target)` | [`beam`] → `PartSpawn::Spark78` |
//! | `PartType78Spawn` | a record (none when the pool is full); modes 0 / 1: no draw; +0x38 the target, +0x3c its record +0x10 | `crate::particles::type78::spawn` (created by the particle hook, `super::fx::create_particles`) |
//! | `UpdateParts` type 78 | homing on the target's position, around the gravity 0x13f5e0 | `type78::update`; the hook writes `Particles::gravity` from `Hero::gravity_dir` |
//!
//! Side effects: the two particle records only (no sound, light, HUD or hit; the target is only read).
//!
//! **The light** (`WritePointLight_A(7.5, 0, …, 0x207f7f)`): 0.5 behind the muzzle along the item's row 1, radius 7.5,
//! colour (1, 1, 0.25)·127/128; after the firing it fades over 20 ticks and is freed (also when put away). **The sound**:
//! class sound 0 looping (flags 4) while firing, on its own channel ([`CHANNEL`] = `super::fx::LOOP_MORPH`: the game
//! keeps the slot in its own global 0x1617f8), released when the firing stops.
//!
//! Native `f32`. [L]: the end point's x / y of a targeted beam (the decompiler kept only z: the aim point's); the pulse
//! index past the last point is clamped (the game reads the next array); the lines ignore Ratchet (the game: the item
//! for the cone line); `FUN_002731d0` wraps the phase to (−π, π].

use super::guns::{add3, cross3, len3, scale3, sub3, with_len};
use super::items::{HitSink, ItemEnv};
use super::packs::SoundCmd;
use super::physics::{from_f32x3, ticks, to_f32x3};
use super::Hero;
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::classes::blaster_shot::rotate;
use crate::moby_update::creature::{add_rot, atan, diff_rots};
use crate::point_lights::PointLight;
use crate::rng::Rng;
use crate::targeting;

pub const MORPH: i32 = 21;
pub const CLASS: i16 = 185;
/// gp−0x54f4 (range), gp−0x54f8 (the cone in degrees), gp−0x54f0 (the meter's fall a second).
pub const RANGE: f32 = 8.0;
pub const CONE_DEG: f32 = 10.0;
pub const DRAIN: f32 = 3.0;
/// The classes the search skips.
pub const SKIP: [i16; 2] = [0x58e, 0x452];
/// The loop sound's channel.
pub const CHANNEL: usize = super::fx::LOOP_MORPH;
/// Beam points and ring points.
pub const POINTS: usize = 12;
pub const RING: usize = 20;
/// FX textures (gp−0x5434, −0x542c, −0x5428; the pulses 8, the glow 0xb).
pub const FX_TUBE: usize = 13;
pub const FX_CORE: usize = 14;
pub const FX_GLOW: usize = 16;
pub const FX_PULSE: usize = 8;
pub const FX_MUZZLE: usize = 0xb;
/// gp−0x543c / −0x5438 (the tube, the pulses), gp−0x5418 (the muzzle glow), gp−0x544c / −0x5448 (the strands).
pub const COLOUR: u32 = 0x407f_207f;
pub const COLOUR_GOLD: u32 = 0x6000_7f00;
pub const MUZZLE_RGBA: u32 = 0x7f20_7f7f;
pub const STRAND_RGB: [u32; 2] = [0x7f_2020, 0x20_207f];
/// 0x20ab10: the sparks' colours (`randi(5)`).
pub const SPARK_RGBA: [u32; 5] = [0x7f20_7f7f, 0x7f7f_2020, 0x7f20_207f, 0x7f7f_207f, 0x7f7f_7f20];
/// 0x1dc3a0: the unit quad the pulses and the muzzle glow scale (x along the matrix's row 0).
const CORNERS: [[f32; 4]; 4] = [[0.0, -1.0, 1.0, 1.0], [0.0, -1.0, -1.0, 1.0], [0.0, 1.0, 1.0, 1.0], [0.0, 1.0, -1.0, 1.0]];

/// A beam quad: FX texture, corners, texture coordinates, colours.
pub type BeamQuad = (usize, [[f32; 3]; 4], [[f32; 2]; 4], [u32; 4]);

/// One pulse (0x161848 active, 0x161808 timer, 0x161818 alpha, 0x161828 distance, 0x161838 size, 0x1dd520 position,
/// 0x1dd550 direction).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pulse {
    pub active: bool,
    pub timer: i32,
    pub alpha: f32,
    pub dist: f32,
    pub size: f32,
    pub pos: [f32; 3],
    pub dir: [f32; 3],
}

/// The beam's globals (0x1617ec.., 0x1dc3e0..).
#[derive(Clone, Debug, PartialEq)]
pub struct Beam {
    /// 0x1617ec: lay the beam straight before the next step.
    pub reset: bool,
    /// 0x1617fc: the point spacing; 0x161800: the strands' phase.
    pub seg: f32,
    pub phase: f32,
    pub pts: [[f32; 3]; POINTS],
    /// 0x1dc620 + 0x140·i (ring 0 unused).
    pub rings: [[[f32; 3]; RING]; POINTS],
    pub strands: [[[f32; 3]; POINTS]; 2],
    pub pulse_timer: i32,
    pub pulses: [Pulse; 3],
    /// The draw's scrolls: gp−0x5444 (the strands), gp−0x54b8 / −0x54b0 / −0x54ac (the tube).
    pub scroll: [f32; 4],
    /// The muzzle glow's point (0.5 behind the muzzle) and the gravity at the step.
    pub glow_at: [f32; 3],
    pub gravity: [f32; 3],
    pub gold: bool,
    /// The tick the draw callback was registered for (`RegisterDrawCallback2(0x2d38b8, item)`).
    pub drawn: Option<u64>,
    /// Type-78 sparks queued (`super::fx::PartSpawn::Spark78`), a count for the tests.
    pub sparks78: u32,
    /// gp−0x54f4 (level18 gp−0x54b4): the range the lay and the pulses use (8; level 18's scene 1 sets 24).
    pub range: f32,
}

impl Default for Beam {
    fn default() -> Self {
        Beam { reset: true, seg: 0.0, phase: 0.0, pts: [[0.0; 3]; POINTS], rings: [[[0.0; 3]; RING]; POINTS], strands: [[[0.0; 3]; POINTS]; 2], pulse_timer: 0, pulses: [Pulse::default(); 3], scroll: [0.0; 4], glow_at: [0.0; 3], gravity: [0.0, 0.0, -1.0], gold: false, drawn: None, sparks78: 0, range: RANGE }
    }
}

/// The Morph-o-Ray's pvars (item moby +0x78) and globals.
#[derive(Clone, Debug, PartialEq)]
pub struct MorphRay {
    /// +0x00: the muzzle.
    pub muzzle: [f32; 3],
    /// +0x10 the target, +0x14 the meter's full scale, +0x18 the meter, +0x44 the lock timer.
    pub target: Option<MobyId>,
    pub full: f32,
    pub meter: f32,
    pub lock: i32,
    /// +0x1c (s16) a timer set to `ticks(15)` while firing; +0x1e the "fired" flag (the shot statistics 0x141728..).
    pub t1c: i16,
    pub fired: bool,
    /// +0x34: the HUD meter element is up; gp−0x54e4: its value (0..10000).
    pub hud: bool,
    pub hud_value: i32,
    /// +0x48 / +0x4c: the aim.
    pub yaw: f32,
    pub pitch: f32,
    /// 0x1617f0 the light slot (−1 none), 0x1617f4 its fade timer.
    pub light: i32,
    pub light_t: i32,
    pub beam: Beam,
    /// Firing this tick; the morphs made (the port's records for the tests).
    pub firing: bool,
    pub morphs: Vec<MobyId>,
}

impl Default for MorphRay {
    fn default() -> Self { MorphRay { muzzle: [0.0; 3], target: None, full: 0.0, meter: 0.0, lock: 0, t1c: 0, fired: false, hud: false, hud_value: 0, yaw: 0.0, pitch: 0.0, light: -1, light_t: 0, beam: Beam::default(), firing: false, morphs: Vec::new() } }
}

/// `FUN_002731d0`: an angle wrapped to (−π, π] [L].
fn wrap(a: f32) -> f32 { add_rot(a, 0.0) }

/// A target record's s16 at `o`.
fn rec_i16(m: &crate::moby_runtime::Moby, o: usize) -> Option<i16> {
    let r = targeting::record(m)?;
    m.pvars.get(r + o..r + o + 2).map(|b| i16::from_le_bytes([b[0], b[1]]))
}

/// A creature the search may take (read from the target list before the lines are tested).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub id: MobyId,
    /// The moby's position and its aim point (position + record +0x10).
    pub pos: [f32; 3],
    pub aim: [f32; 3],
    /// The record's radius byte +0x0a.
    pub radius: Option<u8>,
}

/// The search's candidates in list order: live creatures (class type 5) with a target record, not of [`SKIP`].
pub fn candidates(table: &MobyTable, list: &[MobyId], class_type: &dyn Fn(i16) -> Option<u8>) -> Vec<Candidate> {
    let mut out = Vec::new();
    for &id in list {
        let Some(m) = table.mobys.get(id) else { continue };
        if (m.state as i8) < 0 || SKIP.contains(&m.o_class) { continue; }
        let Some(r) = targeting::record(m) else { continue };
        if !m.has_class || class_type(m.o_class) != Some(5) { continue; }
        let h = f32::from_le_bytes(m.pvars[r + 0x10..r + 0x14].try_into().unwrap());
        let pos = [m.position[0], m.position[1], m.position[2]];
        out.push(Candidate { id, pos, aim: [pos[0], pos[1], pos[2] + h], radius: targeting::record_radius(m) });
    }
    out
}

/// `0x2d2018(item, &muzzle, &yaw, &pitch)` (module doc) over [`candidates`]. `blocked(a, b)` = the line `a → b` (flags
/// 6, ignoring Ratchet) hits something: `Some(None)` the world, `Some(Some(m))` moby `m`.
#[allow(clippy::too_many_arguments)]
pub fn search(cands: &[Candidate], muzzle: [f32; 3], yaw: &mut f32, pitch: &mut f32, hero_pos: [f32; 3], hero_moby_pos: [f32; 3], hero_yaw: f32, ground_pitch: f32, blocked: &mut dyn FnMut([f32; 3], [f32; 3]) -> Option<Option<MobyId>>) -> Option<MobyId> {
    let start = [hero_moby_pos[0], hero_moby_pos[1], muzzle[2]];
    if blocked(start, muzzle).is_some() { return None; }
    let mut best = RANGE;
    let mut found = None;
    let cone = CONE_DEG * 0.017_453_292;
    for c in cands {
        let p = c.aim;
        let ty = atan(p[0] - muzzle[0], p[1] - muzzle[1]);
        let d2 = ((p[0] - muzzle[0]).powi(2) + (p[1] - muzzle[1]).powi(2)).sqrt();
        let elev = atan(d2, p[2] - muzzle[2]);
        let off = diff_rots(*yaw, ty);
        let d = len3(sub3(p, muzzle));
        if d < 2.5 {
            let to = atan(c.pos[0] - hero_pos[0], c.pos[1] - hero_pos[1]);
            if diff_rots(hero_yaw, to) < f32::from_bits(0x3f86_0a92) && (muzzle[2] - p[2]).abs() < 2.0 && blocked(muzzle, p).is_none() {
                *pitch = -elev;
                *yaw = ty;
                return Some(c.id);
            }
        }
        if best < d { continue; }
        let miss = targeting::cone_miss(d, *yaw, *pitch + ground_pitch * 0.5, muzzle, p, c.radius, cone);
        if miss < cone {
            let ok = match blocked(muzzle, p) { None => true, Some(hit) => hit == Some(c.id) };
            if ok {
                *pitch = -elev;
                best = d;
                found = Some(c.id);
                if off < 0.174_532_92 { *yaw = ty; }
            }
        }
    }
    found
}

impl Beam {
    /// `0x2d2c00`: once after a pause, the points laid straight along the item's −row 1 (`range/12` apart), the
    /// scrolls, the phase and the pulses cleared.
    pub fn lay(&mut self, muzzle: [f32; 3], row1: [f32; 3]) {
        if !self.reset { return; }
        self.reset = false;
        self.seg = self.range / 12.0;
        let d = scale3(row1, -self.seg);
        self.pts[0] = muzzle;
        for i in 1..POINTS { self.pts[i] = add3(self.pts[i - 1], d); }
        self.scroll[1] = 0.0;
        self.scroll[2] = 0.0;
        self.scroll[3] = 0.0;
        self.phase = 0.0;
        self.pulse_timer = 0;
        for p in &mut self.pulses {
            p.active = false;
            p.dir = d;
        }
    }

    /// The points, rings and strands of one step of `0x2d2d08` (`aim` the first segment along the aim, `target` its
    /// position, `len` the beam's length).
    pub fn shape(&mut self, muzzle: [f32; 3], yaw: f32, pitch: f32, target: Option<[f32; 3]>, len: f32, gravity: [f32; 3]) {
        self.phase = wrap(self.phase + f32::from_bits(0x3e4d_87ac));
        let mut aim = targeting::polar(self.seg, yaw, pitch);
        self.seg = len * f32::from_bits(0x3daa_a8eb);
        self.gravity = gravity;
        self.pts[0] = muzzle;
        self.strands[0][0] = muzzle;
        self.strands[1][0] = muzzle;
        for i in 1..POINTS {
            let prev = sub3(self.pts[i], self.pts[i - 1]);
            let fi = i as f32 * f32::from_bits(0x3daa_a8eb);
            if 6 < i {
                if let Some(t) = target {
                    let to = with_len(sub3(t, self.pts[i]), self.seg);
                    aim = add3(aim, scale3(sub3(to, aim), 0.2));
                }
            }
            let v = add3(prev, scale3(sub3(aim, prev), fi * -0.499_999_97 + 0.9));
            let l = len3(v);
            self.pts[i] = if l == 0.0 { self.pts[i - 1] } else { add3(self.pts[i - 1], scale3(v, self.seg / l)) };
            let d = sub3(self.pts[i], self.pts[i - 1]);
            let perp = with_len(cross3(d, gravity), 0.05 + (1.234 - 0.05) * fi);
            for k in 0..RING { self.rings[i][k] = add3(rotate(perp, k as f32 * f32::from_bits(0x3ea0_d97a), d), self.pts[i]); }
            let e = with_len(d, 1.0);
            let n = with_len(perp, 1.0);
            let b = cross3(n, e);
            let amp = 0.01 + (0.5 - 0.01) * fi;
            let a = wrap(self.phase + f32::from_bits(0x3ecc_a2e8) * i as f32);
            let (y, z) = (amp * a.sin(), amp * a.cos());
            self.strands[0][i] = add3(self.pts[i], add3(scale3(n, y), scale3(b, z)));
            self.strands[1][i] = add3(self.pts[i], add3(scale3(n, y), scale3(b, -z)));
        }
    }
}

/// `FastDecTimer__FRi`: 0 running, 2 on reaching 0, 1 idle (0 before).
fn fast_dec(t: &mut i32) -> i32 {
    if *t == 0 { return 1; }
    *t = (*t).max(1) - 1;
    if *t <= 0 { 2 } else { 0 }
}

impl Beam {
    /// The pulse spawn and update of `0x2d2d08` (module doc).
    pub fn pulses_step(&mut self) {
        let dt = crate::moby_update::creature::DT;
        if fast_dec(&mut self.pulse_timer) != 0 {
            self.pulse_timer = ticks(45);
            if let Some(p) = self.pulses.iter_mut().find(|p| !p.active) {
                p.active = true;
                p.timer = ticks(60);
                p.size = 0.1;
                p.dist = 0.0;
                p.alpha = (COLOUR >> 24) as f32;
                p.pos = self.pts[0];
                p.dir = with_len(sub3(self.pts[1], self.pts[0]), 1.0);
            }
        }
        for k in 0..3 {
            if !self.pulses[k].active { continue; }
            if fast_dec(&mut self.pulses[k].timer) != 0 {
                self.pulses[k].active = false;
                continue;
            }
            let seg = self.seg;
            let pts = self.pts;
            let p = &mut self.pulses[k];
            if p.timer < ticks(30) { p.alpha -= f32::from_bits(0x4008_887b); }
            p.dist += 8.0 * dt;
            let f = (p.dist / self.range).min(1.0);
            p.size = 0.1 + (1.5 - 0.1) * f;
            if self.range <= p.dist {
                p.pos = add3(p.pos, scale3(p.dir, 8.0 * dt));
            } else {
                let idx = if seg == 0.0 { 0 } else { ((p.dist / seg) as i32).max(0) as usize };
                let rem = p.dist - idx as f32 * seg;
                let idx = idx.min(POINTS - 1);
                p.pos = pts[idx];
                let v = if idx == POINTS - 1 { sub3(pts[11], pts[10]) } else { sub3(pts[idx + 1], pts[idx]) };
                let l = len3(v);
                if l != 0.0 {
                    let inv = 1.0 / l;
                    p.pos = add3(p.pos, scale3(v, rem * inv));
                    let t = with_len(v, inv);
                    p.dir = with_len(add3(p.dir, scale3(sub3(t, p.dir), 0.1)), 1.0);
                }
            }
        }
    }

    /// The draw callback `0x2d38b8`'s scroll steps (made once per drawn tick).
    pub fn step_scrolls(&mut self) {
        let s = &mut self.scroll;
        s[0] -= 0.3;
        if s[0] <= -8.0 { s[0] += 8.0; }
        s[1] -= 0.04;
        if s[1] <= -8.0 { s[1] += 8.0; }
        s[2] -= 0.01;
        if s[2] <= -7.0 { s[2] += 7.0; }
        s[3] += 0.01;
        if 5.0 <= s[3] { s[3] -= 5.0; }
    }

    /// The draw callback's quads `(fx, corners, st, rgba)` seen from `cam` (module doc), in the game's order: the two
    /// strands (core, then glow), the pulses, the muzzle glow, the tube.
    pub fn quads(&self, cam: [f32; 3]) -> Vec<BeamQuad> {
        let mut out = Vec::new();
        let tube = if self.gold { COLOUR_GOLD } else { COLOUR };
        for (s, rgb) in self.strands.iter().zip(STRAND_RGB) { strand(&mut out, s, cam, self.scroll[0], rgb); }
        let up = scale3(self.gravity, -1.0);
        for p in self.pulses.iter().filter(|p| p.active) {
            let rgba = (tube & 0xff_ffff) | ((p.alpha as i32 as u32) << 24);
            let side = with_len(cross3(p.dir, up), 1.0);
            let up2 = cross3(side, p.dir);
            let c = CORNERS.map(|k| add3(add3(scale3(p.dir, k[0] * p.size), scale3(side, k[1] * p.size)), add3(scale3(up2, k[2] * p.size), scale3(p.pos, k[3]))));
            out.push((FX_PULSE, c, [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]], [rgba; 4]));
        }
        {
            let e = with_len(sub3(cam, self.glow_at), 1.0);
            let side = with_len(cross3(e, self.gravity), -1.0);
            let up2 = cross3(side, e);
            for (size, rgba) in [(0.4, MUZZLE_RGBA), (0.1, 0xffff_ffff)] {
                let c = CORNERS.map(|k| add3(add3(scale3(e, k[0] * size), scale3(side, k[1] * size)), add3(scale3(up2, k[2] * size), scale3(self.glow_at, k[3]))));
                out.push((FX_MUZZLE, c, [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]], [rgba; 4]));
            }
        }
        let mut u = self.scroll[1];
        let mut rgba = [tube; 4];
        for i in 2..POINTS {
            let (mut va, mut vb) = (self.scroll[2], self.scroll[3]);
            for k in 0..RING {
                let p0 = self.rings[i][k];
                let p1 = self.rings[i][(k + 1) % RING];
                let p2 = self.rings[i - 1][k];
                let p3 = self.rings[i - 1][(k + 1) % RING];
                let n = cross3(sub3(p2, p1), sub3(p0, p1));
                if 0.0 <= super::guns::dot3(n, sub3(p1, cam)) {
                    va += 0.05;
                    vb += 0.05;
                    continue;
                }
                if i == POINTS - 1 {
                    rgba[0] = tube & 0xff_ffff;
                    rgba[1] = tube & 0xff_ffff;
                }
                let u1 = u - f32::from_bits(0x3d88_88b5);
                out.push((FX_TUBE, [p0, p1, p2, p3], [[u, va], [u, va + 0.05], [u1, va], [u1, va + 0.05]], rgba));
                va += 0.05;
                out.push((FX_TUBE, [p0, p1, p2, p3], [[u, vb], [u, vb + 0.05], [u1, vb], [u1, vb + 0.05]], rgba));
                vb += 0.05;
            }
            u += f32::from_bits(0x3d88_88b5);
        }
        out
    }
}

/// `0x2d3ce0(0.05, 0.4, 0.9, points, 128, 64, 12, rgb)`: a strand as a camera-facing core ribbon (FX 14, 0.05 wide,
/// white at alpha 128, transparent at both ends) along points 0..10, then glow quads (FX 16, 0.4 wide, `rgb` at alpha
/// 64, transparent at both ends) between each pair of points brought to the same camera distance and stretched by 0.9
/// of their length each way.
fn strand(out: &mut Vec<BeamQuad>, p: &[[f32; 3]; POINTS], cam: [f32; 3], scroll: f32, rgb: u32) {
    let n = POINTS;
    let core = 0x7f_7f7f | (128 << 24);
    let clear = 0x7f_7f7f;
    let st = [[scroll, 0.0], [scroll, 1.0], [scroll + 1.0, 0.0], [scroll + 1.0, 1.0]];
    let side_at = |a: [f32; 3], b: [f32; 3]| {
        let s = cross3(sub3(b, a), sub3(a, cam));
        let l = len3(s);
        scale3(s, if l != 0.0 { 0.05 / l } else { 0.05 })
    };
    let s0 = side_at(p[0], p[1]);
    let (mut c0, mut c1) = (add3(p[0], s0), sub3(p[0], s0));
    let mut rgba = [core; 4];
    for i in 1..n - 1 {
        let s = side_at(p[i], p[i + 1]);
        let (c2, c3) = (add3(p[i], s), sub3(p[i], s));
        if i == 1 {
            rgba[0] = clear;
            rgba[1] = clear;
        } else if i == n - 2 {
            rgba[2] = clear;
            rgba[3] = clear;
        } else if i == 2 {
            rgba[0] = core;
            rgba[1] = core;
        }
        out.push((FX_CORE, [c0, c1, c2, c3], st, rgba));
        c0 = c2;
        c1 = c3;
    }
    let glow = rgb | (64 << 24);
    let mut rgba = [glow; 4];
    for i in 0..n - 1 {
        let mut a = sub3(p[i], cam);
        let mut b = sub3(p[i + 1], cam);
        let (la, lb) = (len3(a), len3(b));
        if lb < la {
            if lb != 0.0 { b = scale3(b, la / lb); }
        } else if la != 0.0 {
            a = scale3(a, lb / la);
        }
        let mut pa = add3(cam, a);
        let mut pb = add3(cam, b);
        let d = sub3(pb, pa);
        let ext = scale3(d, 0.9);
        pa = sub3(pa, ext);
        pb = add3(pb, ext);
        let c = sub3(cam, pa);
        let s = cross3(c, d);
        let l = len3(s);
        let s = scale3(s, if l != 0.0 { 0.4 / l } else { 0.4 });
        if i == 0 {
            rgba[0] = rgb;
            rgba[1] = rgb;
        } else if i == n - 2 {
            rgba[2] = rgb;
            rgba[3] = rgb;
        } else if i == 1 {
            rgba[0] = glow;
            rgba[1] = glow;
        }
        out.push((FX_GLOW, [add3(pa, s), sub3(pa, s), add3(pb, s), sub3(pb, s)], [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]], rgba));
    }
}

fn item_row1(hero: &Hero) -> [f32; 3] {
    hero.items.slot.item.as_ref().map(|it| [f32::from_bits(it.rows[1][0]), f32::from_bits(it.rows[1][1]), f32::from_bits(it.rows[1][2])]).unwrap_or([0.0, 1.0, 0.0])
}

/// The firing movement groups (0x1413dc: < 2, 4, 5, 0xc): idle, walk / run, crouch, the jumps and the packs.
fn can_fire(group: i32) -> bool { matches!(group, 0 | 1 | 4 | 5 | 0xc) }

/// `0x2d2450`, the Morph-o-Ray's update (from the slot loop with the slot ready).
#[allow(clippy::too_many_lines)]
pub fn update(hero: &mut Hero, table: &mut MobyTable, _anim: &dyn super::anim::AnimCtl, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng) {
    let Some(class) = hero.items.slot.item.as_ref().and_then(|m| env.data.class(m.o_class)).cloned() else { return };
    let tick = env.frame as u64;
    let away = hero.items.slot.state == 3;
    let gold = hero.weapons.gold[MORPH as usize];
    if let Some(it) = hero.items.slot.item.as_mut() {
        if it.mstate == 0 {
            it.mstate = 1;
            let m = &mut hero.weapons.reactive.morph;
            m.light = -1;
            m.light_t = 0;
            if hero.fx.item_loops[CHANNEL].is_some() { hero.fx.item_voices.push(SoundCmd::ItemRelease { n: CHANNEL }); }
        }
    }
    let ready = hero.items.slot.item.as_ref().is_some_and(|it| it.anim.seq_b != 0 || 6.0 <= crate::moby_update::creature::ground::key_time_of(&class.anim, &it.anim));
    guns_dec16(&mut hero.weapons.reactive.morph.t1c);
    let mask = hero.items.slot.fire_mask;
    let firing = can_fire(hero.group) && env.pad.held & mask != 0 && !away && ready;
    hero.weapons.reactive.morph.firing = firing;
    let hero_pos = to_f32x3(hero.pos);
    if firing {
        if hero.f13f8 == 0 { hero.weapons.pending_draw = true; }
        let m = &mut hero.weapons.reactive.morph;
        m.fired = true;
        if m.t1c == 0 { m.t1c = ticks(15) as i16; }
        let muzzle = super::guns::item_point(hero, env, 0);
        hero.weapons.reactive.morph.muzzle = muzzle;
        if hero.state == 1 { hero.weapons.deferred = Some(0x1e); }
        let dir = if super::guns::first_person(hero) {
            env.camera.map(|c| c.1).unwrap_or(to_f32x3(hero.moby_rows[0]))
        } else if let (true, Some(r)) = (super::guns::arm_raised(hero), super::guns::item_rows(hero)) {
            // The arm raised at a wall (0x141618 = 1): along the gun's −row 1 (its +0xd0).
            [-r[1][0], -r[1][1], -r[1][2]]
        } else {
            to_f32x3(hero.moby_rows[0])
        };
        let m = &mut hero.weapons.reactive.morph;
        m.yaw = atan(dir[0], dir[1]);
        m.pitch = atan((dir[0] * dir[0] + dir[1] * dir[1]).sqrt(), dir[2]);
        let (mut yaw, mut pitch) = (m.yaw, m.pitch);
        let hero_moby_pos = table.mobys.get(env.hero_moby).map(|h| [h.position[0], h.position[1], h.position[2]]).unwrap_or(hero_pos);
        let hero_yaw = table.mobys.get(env.hero_moby).map(|h| h.rotation[2]).unwrap_or(hero.rot[2].to_f32());
        let ground_pitch = hero.pitch.to_f32();
        let cands = { let ct = |o: i16| hits.class_type(o); candidates(table, env.targets, &ct) };
        let coll = env.coll;
        let hero_moby = env.hero_moby;
        let found = {
            let mut blocked = |a: [f32; 3], b: [f32; 3]| -> Option<Option<MobyId>> {
                match hits.probe_moby(table, from_f32x3(a), from_f32x3(b), 6, Some(hero_moby)) {
                    Some(r) => r.map(|pr| pr.moby),
                    None => coll.and_then(|c| super::physics::line_world(c, from_f32x3(a), from_f32x3(b), 6)).map(|_| None),
                }
            };
            search(&cands, muzzle, &mut yaw, &mut pitch, hero_pos, hero_moby_pos, hero_yaw, ground_pitch, &mut blocked)
        };
        let m = &mut hero.weapons.reactive.morph;
        m.yaw = yaw;
        m.pitch = pitch;
        guns_dec(&mut m.lock);
        if m.target.is_some_and(|t| table.mobys.get(t).is_none_or(|x| x.state == 0xfe || x.state == 0xfd)) {
            m.target = None;
            m.lock = 0;
        }
        let drain = match found {
            None => m.lock != 0,
            Some(t) => {
                if Some(t) != m.target && m.lock == 0 {
                    let mm = &table.mobys[t];
                    m.target = Some(t);
                    m.full = rec_i16(mm, 4).unwrap_or(0) as f32;
                    m.meter = targeting::record_health(mm).unwrap_or(0.0);
                    m.lock = ticks(30);
                    if m.full <= 1.0 { m.meter = 0.0; }
                    m.hud = true;
                }
                true
            }
        };
        if drain {
            let dt = crate::moby_update::creature::DT;
            m.meter -= DRAIN * dt * (gold as f32 * 0.33 + 1.0);
            if m.meter < 0.0 { m.meter = 0.0; }
            m.hud_value = 10000 - if m.full != 0.0 { ((m.meter * 10000.0) / m.full) as i32 } else { 0 };
        }
        if let (Some(t), true) = (m.target, m.meter <= 0.0) {
            let mut made = None;
            hits.world(table, &*hero, rng, tick, &mut |w| {
                // Level 5, class 625 (0x271) morphed: skill point 0x13d410, level sound 1, banner 0x53d6.
                if w.svc.level == 5 && w.m(t).o_class == 0x271 {
                    crate::moby_update::story::award_skill_point(w, crate::moby_update::story::skill_index(0x13_d410));
                }
                made = crate::moby_update::creature::react::morph_target(w, t);
            });
            let m = &mut hero.weapons.reactive.morph;
            if let Some(c) = made { m.morphs.push(c); }
            m.target = None;
        }
        beam(hero, table, env, hits, rng, tick);
        // The light.
        let row1 = item_row1(hero);
        let m = &mut hero.weapons.reactive.morph;
        let item_pos = hero.items.slot.item.as_ref().map(|it| it.position).unwrap_or(hero_pos);
        if m.light == -1 {
            m.light = hits.light_alloc(PointLight { color: [127.0 / 128.0, 127.0 / 128.0, 32.0 / 128.0], intensity: 0.0, pos: item_pos, radius: 7.5 });
        }
        if 0 <= m.light {
            m.light_t = ticks(20);
            let at = add3(m.muzzle, with_len(row1, -0.5));
            let mut l = hits.light_get(m.light).unwrap_or(PointLight { color: [127.0 / 128.0, 127.0 / 128.0, 32.0 / 128.0], intensity: 0.0, pos: at, radius: 7.5 });
            l.pos = at;
            l.radius = 7.5;
            hits.light_set(m.light, l);
        }
        hero.fx.item_voices.push(SoundCmd::ItemLoop { n: CHANNEL, index: 0, flags: 4 });
        if away { free_light(hero, hits); }
        return;
    }
    // Not firing.
    let row1 = item_row1(hero);
    let m = &mut hero.weapons.reactive.morph;
    m.target = None;
    m.fired = false;
    m.lock = 0;
    m.hud = false;
    m.beam.reset = true;
    if 0 <= m.light {
        let at = add3(m.muzzle, with_len(row1, -0.5));
        let k = m.light_t as f32 * 0.05;
        hits.light_set(m.light, PointLight { color: [k, k, k * 0.25], intensity: 0.0, pos: at, radius: 7.5 });
        if fast_dec(&mut m.light_t) != 0 {
            hits.light_free(m.light);
            m.light = -1;
        }
    }
    if hero.fx.item_loops[CHANNEL].is_some() { hero.fx.item_voices.push(SoundCmd::ItemRelease { n: CHANNEL }); }
    super::weapons::put_away(hero);
    if away { free_light(hero, hits); }
}

fn free_light(hero: &mut Hero, hits: &mut dyn HitSink) {
    let m = &mut hero.weapons.reactive.morph;
    if 0 <= m.light {
        hits.light_free(m.light);
        m.light = -1;
    }
}

fn guns_dec(t: &mut i32) { let _ = fast_dec(t); }
fn guns_dec16(t: &mut i16) {
    if *t == 0 { return; }
    *t = (*t).max(1) - 1;
}


/// `0x2d2d08`: one step of the beam (module doc) and the draw callback's registration.
fn beam(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng, tick: u64) {
    let row1 = item_row1(hero);
    let gravity = to_f32x3(hero.gravity_dir);
    let gold = hero.weapons.gold[MORPH as usize] != 0;
    let m = &mut hero.weapons.reactive.morph;
    let muzzle = m.muzzle;
    m.beam.lay(muzzle, row1);
    m.beam.gold = gold;
    let target = m.target.and_then(|t| table.mobys.get(t).filter(|x| targeting::record(x).is_some()).map(|x| (t, [x.position[0], x.position[1], x.position[2]], targeting::aim_point(x))));
    let len = match target {
        Some((_, p, _)) => ((p[0] - muzzle[0]).powi(2) + (p[1] - muzzle[1]).powi(2)).sqrt(),
        None => {
            let end = add3(muzzle, targeting::polar(RANGE, m.yaw, m.pitch));
            let hit = match hits.probe(table, from_f32x3(muzzle), from_f32x3(end), 0x14, Some(env.hero_moby)) {
                Some(h) => h,
                None => env.coll.and_then(|c| super::physics::line_world(c, from_f32x3(muzzle), from_f32x3(end), 0x14)).map(|o| o.point),
            };
            match hit {
                Some(h) => ((h[0] - muzzle[0]).powi(2) + (h[1] - muzzle[1]).powi(2)).sqrt(),
                None => RANGE,
            }
        }
    };
    let word = target.and_then(|t| table.mobys.get(t.0)).and_then(targeting::aim_height).map_or(0, f32::to_bits);
    let m = &mut hero.weapons.reactive.morph;
    let (yaw, pitch) = (m.yaw, m.pitch);
    m.beam.drawn = Some(tick);
    let mut parts = std::mem::take(&mut hero.fx.parts);
    beam_step(&mut hero.weapons.reactive.morph.beam, muzzle, row1, yaw, pitch, target.map(|t| (t.0, t.1, word)), len, gravity, rng, &mut parts);
    hero.fx.parts = parts;
}

/// The rest of `0x2d2d08` after the length: the shape, the sparkles at the strands' ends and the sparks along the beam
/// (queued in `parts` as [`super::fx::PartSpawn`]s), the pulses and the scrolls. `target`: the target, its position
/// and its aim word. Shared by the hero's item and the scenes that fire the beam (level 18's 1563,
/// `moby_update::classes::units::veldin_finale_fx`).
#[allow(clippy::too_many_arguments)]
pub fn beam_step(b: &mut Beam, muzzle: [f32; 3], row1: [f32; 3], yaw: f32, pitch: f32, target: Option<(MobyId, [f32; 3], u32)>, len: f32, gravity: [f32; 3], rng: &mut Rng, parts: &mut Vec<super::fx::PartSpawn>) {
    b.shape(muzzle, yaw, pitch, target.map(|t| t.1), len, gravity);
    // The sparkles at the strands' ends, the sparks along the beam.
    let s = rng.randf(0.1, 2.0);
    let r = rng.randi(2);
    let mut sgn: i8 = if r == 0 { -1 } else { r as i8 };
    let life = ticks(30);
    for (k, rgb) in [(0usize, 0x7f7f_2020u32), (1, 0x7f20_207f)] {
        if rng.randi(2) == 0 {
            let p = b.strands[k][POINTS - 1];
            let pos = [p[0], p[1], p[2], 1.0];
            parts.push(super::fx::PartSpawn::Sparkle { s12: s * 0.1, s13: s, s14: f32::from_bits(0x3b44_9ba6), pos, life, rgba: rgb, b8: 0, t0: sgn, vel: [0.0; 3] });
            sgn = -sgn;
            parts.push(super::fx::PartSpawn::Sparkle { s12: s * 0.07, s13: s * 0.7, s14: f32::from_bits(0x3b44_9ba6), pos, life, rgba: 0x7f7f_7f7f, b8: 0x20, t0: sgn, vel: [0.0; 3] });
        }
    }
    let dir = with_len(sub3(b.pts[1], b.pts[0]), 0.2);
    if rng.randi(2) == 0 {
        // The sparks (0x2d3388..0x2d35e8): from 0.25 ahead of the first point along the item's −row 1, the beam's step
        // turned by ±15° about the gravity and about beam × gravity, a colour of the table 0x20ab10; life = the
        // beam's length / its step ×1.2 (×3 with a target); a coloured one (mode 0, spin sgn) and a white core (mode
        // 1, −sgn), both homing on the target.
        let pos = add3(b.pts[0], with_len(row1, -0.25));
        let c = SPARK_RGBA[rng.randi(5) as usize];
        let a1 = rng.randf_sym(0.0, f32::from_bits(0x3e86_0a92));
        let a2 = rng.randf_sym(0.0, f32::from_bits(0x3e86_0a92));
        let d2 = rotate(dir, a2, gravity);
        let vel = rotate(d2, a1, cross3(dir, gravity));
        let tgt = target.map(|t| t.0);
        let mut life = ((len * 1.2) / 0.2) as i32;
        if tgt.is_some() { life *= 3; }
        let word = target.map_or(0, |t| t.2);
        let pos = [pos[0], pos[1], pos[2], 1.0];
        let spark = |s1: f32, s2: f32, rgba: u32, mode: i32, spin: i8| super::fx::PartSpawn::Spark78 {
            s1, s2, pos, life: life as i16, rgba, mode, spin: spin as u8, vel, target: tgt, target_word: word,
        };
        parts.push(spark(s * 0.1, s, c, 0, sgn));
        parts.push(spark(s * 0.07, s * 0.7, 0x7f7f_7f7f, 1, -sgn));
        b.sparks78 += 2;
    }
    b.pulses_step();
    b.glow_at = add3(b.pts[0], with_len(row1, -0.5));
    b.step_scrolls();
}

/// The Morph-o-Ray's leftovers when it leaves the hand without its own update (the light, the loop sound, the beam).
pub fn item_gone(hero: &mut Hero, hits: &mut dyn HitSink) {
    let m = &mut hero.weapons.reactive.morph;
    if 0 <= m.light {
        hits.light_free(m.light);
        m.light = -1;
    }
    m.target = None;
    m.hud = false;
    m.firing = false;
    m.beam.reset = true;
    m.beam.drawn = None;
    if hero.fx.item_loops[CHANNEL].is_some() { hero.fx.item_voices.push(SoundCmd::ItemRelease { n: CHANNEL }); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_in_the_games_movement_groups() {
        // 0x1413dc, unsigned: < 2 (idle, walk / run), 4 (the jumps), 5 (the packs), 0xc (crouch).
        for g in [0, 1, 4, 5, 0xc] { assert!(can_fire(g), "group {g:#x} fires"); }
        for g in [-1, 2, 3, 6, 7, 8, 9, 0xb, 0xd, 0xf, 0x14] { assert!(!can_fire(g), "group {g:#x} does not"); }
    }

    #[test]
    fn beam_lays_straight_then_bends_to_the_aim() {
        let mut b = Beam::default();
        b.lay([0.0, 0.0, 1.0], [0.0, -1.0, 0.0]);
        assert!((b.seg - RANGE / 12.0).abs() < 1e-6);
        assert!((b.pts[11][1] - 11.0 * RANGE / 12.0).abs() < 1e-4, "{:?}", b.pts[11]);
        // Aim along +x: the far points swing toward +x over the ticks.
        for _ in 0..60 { b.shape([0.0, 0.0, 1.0], 0.0, 0.0, None, RANGE, [0.0, 0.0, -1.0]); }
        let tip = b.pts[11];
        assert!(tip[0] > 7.0 && tip[1].abs() < 0.5, "{tip:?}");
        // The rings grow along the beam; the strands wind about it.
        let r1 = len3(sub3(b.rings[1][0], b.pts[1]));
        let r11 = len3(sub3(b.rings[11][0], b.pts[11]));
        assert!(r1 < 0.2 && (r11 - 1.13).abs() < 0.05, "{r1} {r11}");
        let s = len3(sub3(b.strands[0][11], b.pts[11]));
        assert!((s - (0.01 + 0.49 * 11.0 / 12.0)).abs() < 1e-3, "{s}");
    }

    #[test]
    fn pulses_run_out_and_fade() {
        let mut b = Beam::default();
        b.lay([0.0; 3], [0.0, -1.0, 0.0]);
        for _ in 0..3 { b.shape([0.0; 3], std::f32::consts::FRAC_PI_2, 0.0, None, RANGE, [0.0, 0.0, -1.0]); }
        b.pulses_step();
        assert!(b.pulses[0].active);
        for _ in 0..40 { b.pulses_step(); }
        let p = b.pulses[0];
        assert!(p.dist > 5.0 && p.size > 1.0 && p.alpha < 64.0, "{p:?}");
        for _ in 0..30 { b.pulses_step(); }
        assert!(!b.pulses[0].active && b.pulses[1].active);
    }

    #[test]
    fn quads_cull_back_faces_and_fade_the_far_ring() {
        let mut b = Beam::default();
        b.lay([0.0; 3], [-1.0, 0.0, 0.0]);
        for _ in 0..5 { b.shape([0.0; 3], 0.0, 0.0, None, RANGE, [0.0, 0.0, -1.0]); }
        let q = b.quads([-3.0, 0.0, 2.0]);
        let tube: Vec<_> = q.iter().filter(|x| x.0 == FX_TUBE).collect();
        assert!(!tube.is_empty() && tube.len() < 10 * RING * 2, "{}", tube.len());
        assert!(q.iter().any(|x| x.0 == FX_CORE) && q.iter().any(|x| x.0 == FX_GLOW) && q.iter().filter(|x| x.0 == FX_MUZZLE).count() == 2);
        // The far ring's quads have transparent near-edge vertices 0 / 1.
        assert!(tube.iter().any(|x| x.3[0] >> 24 == 0));
    }
}
