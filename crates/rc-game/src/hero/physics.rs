//! Hero math helpers (level01 `0x2206xx..0x2222xx` library, `0x270658..0x270b58` springs), the stick / turn /
//! speed / air-control primitives every state uses, the per-state physics driver `0x2370b8` (the states' own
//! code is in their modules through [`super::registry`]) and the move + collide pipeline `0x23c458` / `0x233de0`.
//!
//! Every float op follows the game's order on the PS2 FPU/VU0 model ([`Pf`]); the VU0 macro helpers are
//! written lane by lane with the ACC order of the instructions (`vmula/vmadda`), so the results are
//! bit-exact by construction as far as that model is (it does not model the multiplier's rare last-bit
//! deviations). The trig helpers are the game's own (VU0 28259 sine polynomial via
//! [`rc_formats::moby_light::vu0_sin_cos`]; `FastArcTan` via [`crate::pad::fast_arctan`]).
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use crate::ps2v::{Pf, SIGN};
use rc_formats::moby_light::vu0_sin_cos;

/// A VU vector (x, y, z, w).
pub type V4 = [Pf; 4];
pub const V0: V4 = [Pf::ZERO; 4];

/// NTSC `dt` = `0x15ed6c` = 1/60 (`0x3c888889`, written by `fun_00214970(0)`).
pub const DT: Pf = Pf::b(0x3c88_8889);
/// NTSC `dt²` = `0x15ed70` (`0x3991a2b4`).
pub const DT2: Pf = Pf::b(0x3991_a2b4);
/// `0x15ed60` / `0x15ed64`: PAL speed scales, 1.0 on NTSC.
pub const SCALE60: Pf = Pf::ONE;
pub const SCALE64: Pf = Pf::ONE;
/// π as the library loads it (`0x40490fdb`).
pub const PI: Pf = Pf::b(0x4049_0fdb);

/// `fun_001f96f8` ticks(n) = `(int)(n·0x15ed68 + 0.5)`, identity on NTSC.
#[inline]
pub fn ticks(n: i32) -> i32 { n }

/// `abs.s` (0x221128).
#[inline]
pub fn fabs(x: Pf) -> Pf { x.abs() }

/// `fast_cos` (0x2216f8, `vcallms 0x190`): the VU0 sine polynomial of 28259 at `a + π/2`.
#[inline]
pub fn fast_cos(a: Pf) -> Pf { Pf(vu0_sin_cos(a.0).1) }
/// `fast_sin` (0x221710, `vcallms 0x192`).
#[inline]
pub fn fast_sin(a: Pf) -> Pf { Pf(vu0_sin_cos(a.0).0) }

/// `fast_add_rotations` (0x221ff8): `a + b` wrapped once into [−π, π); the lower test reads the unwrapped
/// sum (it runs in the branch delay slot).
pub fn fast_add_rotations(a: Pf, b: Pf) -> Pf {
    let s = a + b;
    let below = s < -PI;
    if !(s < PI) { return (s - PI) - PI; }
    if below { return (s + PI) + PI; }
    s
}

/// `fast_subtract_rotations` (0x222040): `a − b` wrapped the same way.
pub fn fast_subtract_rotations(a: Pf, b: Pf) -> Pf {
    let s = a - b;
    let below = s < -PI;
    if !(s < PI) { return (s - PI) - PI; }
    if below { return (s + PI) + PI; }
    s
}

/// `Approach(t, step, &x)` (0x270728): x moves toward t by at most `step`; returns |t − x_new|.
pub fn approach(t: Pf, step: Pf, x: &mut Pf) -> Pf {
    let mut d = t - *x;
    if step < d {
        d = step;
    } else {
        let ns = -step;
        if d < ns { d = ns; }
    }
    *x = *x + d;
    (t - *x).abs()
}

/// The velocity half of the spring (0x270658): `v += k·e − d·v`, clamped to ±max (when max > 0) and
/// to ±|e|.
pub fn spring_vel(e: Pf, k: Pf, d: Pf, max: Pf, v: &mut Pf) {
    let ke = k * e;
    let dv = d * *v;
    let nv = *v + (ke - dv);
    *v = nv;
    if Pf::ZERO < max {
        if max < nv { *v = max; } else if nv < -max { *v = -max; }
    }
    let ae = e.abs();
    if ae < *v { *v = ae; } else if *v < -ae { *v = -ae; }
}

/// `Spring(t, k, d, max, &x, &v)` (0x270780): [`spring_vel`] on `t − x`, `x += v`; when
/// `|t − x| < 0.01·max`, snap `x = t, v = 0`. Returns the remaining error (0 after a snap).
pub fn spring(t: Pf, k: Pf, d: Pf, max: Pf, x: &mut Pf, v: &mut Pf) -> Pf {
    spring_vel(t - *x, k, d, max, v);
    let nx = *x + *v;
    let e = t - nx;
    *x = nx;
    if e.abs() < max * Pf::b(0x3c23_d70a) {
        *x = t;
        *v = Pf::ZERO;
        return Pf::ZERO;
    }
    e
}

/// Angle difference `0x2709f8(t, x, mode)`: `fast_subtract_rotations(t, x)`; with a non-zero `mode`
/// (±1: force the turn direction) a difference of the wrong sign whose magnitude exceeds 0.001745
/// (0.1°) is taken the long way round (±2π).
pub fn angle_diff(t: Pf, x: Pf, mode: i32) -> Pf {
    let d = fast_subtract_rotations(t, x);
    if mode == 0 { return d; }
    if Pf::ZERO < d * Pf::from_i32(mode) { return d; }
    if d.abs() <= Pf::b(0x3ae4_c388) { return Pf::ZERO; }
    let two_pi = Pf::b(0x40c9_0fdb);
    if Pf::ZERO < d { d - two_pi } else { d + two_pi }
}

/// Angular spring (0x270b58): `(target, k, d, max, &angle, &vel, mode)`. Mode 2 picks ±1 from the signs
/// of target and angle. Returns the residual `angle_diff(target, angle)` (0 after the snap).
pub fn turn_spring(target: Pf, k: Pf, d: Pf, max: Pf, angle: &mut Pf, vel: &mut Pf, mut mode: i32) -> Pf {
    if mode == 2 {
        mode = 0;
        let sticky = (Pf::ZERO < target && *angle < Pf::ZERO) || (target < Pf::ZERO && Pf::ZERO < *angle);
        if sticky { mode = if target < Pf::ZERO { -1 } else { 1 }; }
    }
    let e = angle_diff(target, *angle, mode);
    spring_vel(e, k, d, max, vel);
    *angle = fast_add_rotations(*angle, *vel);
    let r = angle_diff(target, *angle, mode);
    if r.abs() < max * Pf::b(0x3c23_d70a) {
        *angle = target;
        *vel = Pf::ZERO;
        return Pf::ZERO;
    }
    r
}

// ------------------------------------------------------------------------------------------------
// VU0 vector helpers (level01 0x221188..0x221634). Lanes not written by an instruction keep the source.

/// `vadd.xyz` (0x221188).
pub fn vadd(a: V4, b: V4) -> V4 { [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3]] }
/// `vsub.xyz` (0x2211b8).
pub fn vsub(a: V4, b: V4) -> V4 { [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3]] }
/// `vmulx.xyz` by a scalar (0x221210).
pub fn vscale(a: V4, s: Pf) -> V4 { [a[0] * s, a[1] * s, a[2] * s, a[3]] }
/// `FastVecDot` (0x2212a8): `(x·x' + y·y') + 1·(z·z')`.
pub fn dot3(a: V4, b: V4) -> Pf { (a[0] * b[0] + a[1] * b[1]) + Pf::ONE * (a[2] * b[2]) }
/// 3-D length on VU0 (`vmul.xyz; vadday.x ACC; vmaddz.x vf3(1)`; `vsqrt`; `vaddq.x vf0`).
pub fn len3(a: V4) -> Pf { Pf::ZERO + ((a[0] * a[0] + a[1] * a[1]) + Pf::ONE * (a[2] * a[2])).sqrt() }
/// `fun_001f9b20` (0x221318): 2-D length.
pub fn len2(a: V4) -> Pf { crate::pad::len2(a[0], a[1]) }

/// `FastVecNormalize(len, dst, src)` (0x221410): `src · (len / sqrt(x² + y² + z²))` (VU `vrsqrt`), the zero
/// vector when `x² + y² + z²` has zero bits.
pub fn set_len3(v: V4, len: Pf) -> V4 {
    let l2 = (v[0] * v[0] + v[1] * v[1]) + Pf::ONE * (v[2] * v[2]);
    if l2.0 == 0 { return [Pf::ZERO, Pf::ZERO, Pf::ZERO, v[3]]; }
    let q = len / l2.sqrt();
    [v[0] * q, v[1] * q, v[2] * q, v[3]]
}

/// 0x221460: xy scaled to length `len` (`vrsqrt Q = len / sqrt(x² + y²)`), xy = 0 when `x² + y²` is 0.
pub fn set_len2(v: V4, len: Pf) -> V4 {
    let l2 = v[0] * v[0] + v[1] * v[1];
    if l2.0 == 0 { return [Pf::ZERO, Pf::ZERO, v[2], v[3]]; }
    let q = len / l2.sqrt();
    [v[0] * q, v[1] * q, v[2], v[3]]
}

/// 0x221288: `src.xyz · (1 / s)` (`vdiv Q, vf0w, s`).
pub fn div_scalar(v: V4, s: Pf) -> V4 {
    let q = Pf::ONE / s;
    [v[0] * q, v[1] * q, v[2] * q, v[3]]
}

/// `fun_001f9c90` (0x2214a8): if `|v| > max` (or `max − |v|` is +0 bits) scale v to length `max`. Returns
/// whether it scaled.
pub fn clamp_len3(v: &mut V4, max: Pf) -> bool {
    let p = [v[0] * v[0], v[1] * v[1], v[2] * v[2]];
    let l2 = (p[0] + p[1]) + Pf::ONE * p[2];
    let len = Pf::ZERO + l2.sqrt();
    let d = max - len;
    if len.0 == 0 && p[1].0 == 0 { return false; }
    if d.0 & SIGN == 0 && d.0 != 0 { return false; }
    let q = max / len;
    *v = [v[0] * q, v[1] * q, v[2] * q, v[3]];
    true
}

/// 0x221510: the 2-D version of [`clamp_len3`] (xy only).
pub fn clamp_len2(v: &mut V4, max: Pf) -> bool {
    let p = [v[0] * v[0], v[1] * v[1]];
    let len = Pf::ZERO + (p[0] + p[1]).sqrt();
    let d = max - len;
    if len.0 == 0 && p[1].0 == 0 { return false; }
    if d.0 & SIGN == 0 && d.0 != 0 { return false; }
    let q = max / len;
    v[0] = v[0] * q;
    v[1] = v[1] * q;
    true
}

/// `vmulax/vmadday/vmaddaz/vmaddw` of `v` by a 3-row matrix plus `vf0·v.w` (0x2215e0): lane k =
/// `((r0.k·x + r1.k·y) + r2.k·z) + (0,0,0,1).k·w`.
pub fn mul_rows3(rows: &[V4; 3], v: V4) -> V4 {
    let w = [Pf::ZERO, Pf::ZERO, Pf::ZERO, Pf::ONE];
    std::array::from_fn(|k| ((rows[0][k] * v[0] + rows[1][k] * v[1]) + rows[2][k] * v[2]) + w[k] * v[3])
}

/// Same with a 4th row (0x221608, `fun_001f9d20`).
pub fn mul_rows4(rows: &[V4; 4], v: V4) -> V4 {
    std::array::from_fn(|k| ((rows[0][k] * v[0] + rows[1][k] * v[1]) + rows[2][k] * v[2]) + rows[3][k] * v[3])
}

/// Rows from Euler angles (`fun_001fa050` 0x221980: `vcallms 0x1a3`, VU0 28259; row 3 = (0,0,0,1)).
pub fn euler_rows(rot: V4) -> [V4; 4] {
    let r = rc_formats::moby_light::rotation_rows([rot[0].to_f32(), rot[1].to_f32(), rot[2].to_f32()]);
    let c = |v: [u32; 4]| v.map(Pf);
    [c(r[0]), c(r[1]), c(r[2]), [Pf::ZERO, Pf::ZERO, Pf::ZERO, Pf::ONE]]
}

pub fn v4(x: f32, y: f32, z: f32) -> V4 { [Pf::f(x), Pf::f(y), Pf::f(z), Pf::ZERO] }
pub fn to_f32x3(v: V4) -> [f32; 3] { [v[0].to_f32(), v[1].to_f32(), v[2].to_f32()] }
pub fn from_f32x3(v: [f32; 3]) -> V4 { [Pf::f(v[0]), Pf::f(v[1]), Pf::f(v[2]), Pf::ZERO] }


// ================================================================================================
// Stick → target (0x231f70), speed table (0x232290), turns, speed step, planar velocity, air control.

use super::{state, Hero};
use crate::collision_query::{coll_line_m, MobyScene, QueryFlags};
use crate::pad::{fast_arctan, PadState};
use rc_formats::collision::Collision;

/// What the hero update reads from outside the hero block.
pub struct Env<'a> {
    pub coll: &'a Collision,
    pub pad: &'a PadState,
    /// Camera yaw `0x167258` (derived by the camera update of the previous tick).
    pub cam_yaw: Pf,
    /// Camera rows `0x167450/60/70` (forward, left, up).
    pub cam_rows: [V4; 3],
    /// Option `0x15edb4`: mirrored controls (0x15edb5, the anim mirror, follows it).
    pub mirror: bool,
    /// `0x15f638`: the level's death height (gameplay header +0x28).
    pub death_z: Pf,
    /// The moby collision the hero's line / sphere / capsule queries test after the world mesh (the table, grid
    /// 0x19bc60, class blobs, pose cache as the moby loop of this tick left them; None: world only).
    pub mobys: Option<&'a MobyScene<'a>>,
    /// The moby those queries pass as `a2` (ignored): Ratchet's moby `0x1413d0`.
    pub hero_moby: Option<usize>,
    /// The level's water-height tables (`0x26ed38`: ripple patches, flat planes) the ground probe refines a
    /// water hit with (None: the hit's own z).
    pub water: Option<&'a dyn super::swim::WaterQuery>,
    /// The level / moby data outside the hero block the hero code reads: the carriers' platform blocks for the
    /// platform carry ([`super::platform::HeroWorld`]; None: nothing carries the hero).
    pub world: Option<&'a dyn super::platform::HeroWorld>,
}

impl Env<'_> {
    /// `CollLine_Fix(a, b, flags, Ratchet, 0)`, PS2 float in/out: the world mesh, then the mobys of [`Env::mobys`].
    pub fn line(&self, a: V4, b: V4, flags: u32) -> Option<crate::collision_query::CollOutput> {
        coll_line_m(self.coll, self.mobys, to_f32x3(a), to_f32x3(b), QueryFlags(flags), self.hero_moby)
    }
}

/// `FUN_00276cb8(r, spline, p, out)` (level01; `FUN_00249e58` calls it with the capsule radius 0x13f584, the spline
/// 0x14162a and Ratchet's position): `p` kept `r` away from the spline's segments in the plane across the gravity
/// direction `up`. Segment by segment (both points' w = 0: skipped), from the point as the earlier segments left it:
/// with `d` = `p − a`, `e` = `b − a` (both flattened across `up`) and `n` = `e`'s direction, a point within `r` of the
/// line (`|(d × n)·up|`) is pushed to `r` from it when its foot `t = d·n` lies on the segment, or else to `r` from
/// `a` when within `r` of `a` (the game's: never from `b`). A push returns the point at its old height along `up`;
/// None: no push. Native `f32` (the VU vector ops).
pub fn spline_wall(r: f32, pts: &[[f32; 4]], p: [f32; 3], up: [f32; 3]) -> Option<[f32; 3]> {
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let add = |a: [f32; 3], b: [f32; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    let len = |a: [f32; 3]| dot(a, a).sqrt();
    // FastVecNormalize(l, out, v): v's direction, length l.
    let norm = |l: f32, v: [f32; 3]| { let k = l / len(v); [v[0] * k, v[1] * k, v[2] * k] };
    let flat = |v: [f32; 3]| sub(v, norm(dot(v, up), up));
    let h0 = dot(p, up);
    let mut p = p;
    let mut hit = false;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a[3] == 0.0 && b[3] == 0.0 { continue; }
        let a3 = [a[0], a[1], a[2]];
        let d = flat(sub(p, a3));
        let e = flat(sub([b[0], b[1], b[2]], a3));
        let n = norm(1.0, e);
        let c = [d[1] * n[2] - d[2] * n[1], d[2] * n[0] - d[0] * n[2], d[0] * n[1] - d[1] * n[0]];
        if dot(c, up).abs() > r { continue; }
        let t = dot(d, n);
        if len(e) < t || t < 0.0 {
            if len(d) < r {
                hit = true;
                p = add(norm(r, d), a3);
            }
        } else {
            hit = true;
            let along = norm(t, n);
            p = add(add(norm(r, sub(d, along)), along), a3);
        }
    }
    hit.then(|| add(p, norm(h0 - dot(p, up), up)))
}

/// The walk/run speed table at 0x17c238: rows `{down, up, speed, stick}` (anim thresholds in u/s, speed
/// in u/s, stick threshold).
pub const SPEED_TABLE: [[Pf; 4]; 2] = [
    [Pf::b(0), Pf::b(0x4016_6666), Pf::b(0x3f66_6666), Pf::b(0x3f51_eb85)],
    [Pf::b(0x3ff3_3333), Pf::b(0x4120_0000), Pf::b(0x40b6_6666), Pf::b(0x3f80_0000)],
];


impl Hero {
    /// `StickTarget(scale, mode)` (0x231f70), mode 0 only (1–4 are strafe/quantised modes, not used on foot).
    pub fn stick_target(&mut self, env: &Env, scale: Pf) {
        let (mut sx, mut sy) = (self.stick[0], self.stick[1]);
        let l = crate::pad::len2(sx, sy);
        if Pf::ONE < l {
            let q = Pf::ONE / l;
            sx = sx * q;
            sy = sy * q;
        }
        if sx == Pf::ZERO && sy == Pf::ZERO {
            self.target_speed = Pf::ZERO;
            self.target_yaw = self.rot[2];
            self.stick_world = mul_rows3(&[self.moby_rows[0], self.moby_rows[1], self.moby_rows[2]], [Pf::ONE, Pf::ZERO, Pf::ZERO, Pf::ZERO]);
            return;
        }
        self.target_yaw = fast_arctan(-sy, -sx);
        self.target_yaw = fast_add_rotations(self.target_yaw, env.cam_yaw);
        let v = [-sy, if env.mirror { sx } else { -sx }, Pf::ZERO, Pf::ZERO];
        self.stick_world = mul_rows3(&env.cam_rows, v);
        self.target_speed = scale * len2(v);
    }

    /// 0x232290: `StickTarget(1)`, then a non-zero target becomes the table speed (walk 0.9 u/s below stick
    /// 0.82, else run 5.7 u/s) × dt. Wading (0x73): `max(0.8·target, 2.5·dt)`.
    pub fn stick_target_table(&mut self, env: &Env) {
        self.stick_target(env, Pf::ONE);
        if Pf::ZERO < self.target_speed {
            let s = if !(self.target_speed < SPEED_TABLE[0][3]) { SPEED_TABLE[1][2] } else { SPEED_TABLE[0][2] };
            self.target_speed = s * DT;
        }
        if self.state == super::swim::id::WADE {
            let t = (self.target_speed.to_f32() * 0.8).max(DT.to_f32() * 2.5);
            self.target_speed = Pf::f(t);
        }
    }

    /// `TurnTo(k, d, max)` (0x232490) in gravity mode 0: the angular spring on yaw toward the target yaw;
    /// the residual goes to 0x13f4d8.
    pub fn turn_to(&mut self, k: Pf, d: Pf, max: Pf) {
        // Gravity modes 1 / 2: the turn in the hero's own frame (0x2323d8, super::boots).
        if self.gravity_mode != 0 { return super::boots::frame_turn(self, k, d, max); }
        let (mut yaw, mut v) = (self.rot[2], self.yaw_vel);
        self.yaw_residual = turn_spring(self.target_yaw, k, d, max, &mut yaw, &mut v, 0);
        self.rot[2] = yaw;
        self.yaw_vel = v;
    }

    /// 0x232598: the run turn `TurnTo(0.019·(s+1)/2, 0.1, 570°/s·dt·(s+1)/2)` (×1.1 / ×1.2 on a sharp
    /// turn). The weapon-module variant (0x22ddd8(0) in 15..16 or 21 with a weapon out) is not reachable
    /// without gadgets.
    pub fn run_turn(&mut self) {
        let h = (self.stick_mag + Pf::ONE) * Pf::b(0x3f00_0000);
        let mut k = (SCALE64 * Pf::b(0x3c9b_a5e3)) * h;
        let d = SCALE64 * Pf::b(0x3dcc_cccd);
        let mut max = (DT * Pf::b(0x411f_2c8d)) * h;
        if self.sharp_turn != 0 {
            k = k * Pf::b(0x3f8c_cccd);
            max = max * Pf::b(0x3f99_999a);
        }
        self.turn_to(k, d, max);
    }

    /// `SpeedStep(acc, dec)` (0x2326e8): speed approaches the target by `acc` when below it, else by `dec`.
    pub fn speed_step(&mut self, acc: Pf, dec: Pf) {
        let mut s = self.speed;
        if s < self.target_speed { approach(self.target_speed, acc, &mut s); } else { approach(self.target_speed, dec, &mut s); }
        self.speed = s;
    }

    /// `SetPlanarVel(yaw)` (0x232738 → 0x277b50), gravity mode 0: yaw > π (the 99999 sentinel) means the
    /// current yaw; `vel = ((cos y · s) · cos p, (sin y · s) · cos p, sin p · s)` with p = ground pitch
    /// 0x13f634 (so on a slope the velocity follows the slope; vel.z is overwritten).
    pub fn set_planar_vel(&mut self, yaw: Pf) {
        // Gravity modes ≠ 0: along the moby's (tilted) facing (super::boots).
        if self.gravity_mode != 0 { return super::boots::frame_planar_vel(self); }
        let y = if PI < yaw { self.rot[2] } else { yaw };
        let s = self.speed;
        let cp = fast_cos(self.pitch);
        self.vel[0] = (fast_cos(y) * s) * cp;
        self.vel[1] = (fast_sin(y) * s) * fast_cos(self.pitch);
        self.vel[2] = fast_sin(self.pitch) * s;
    }

    /// `AirAccel(a)` (0x234358), gravity mode 0.
    pub fn air_accel(&mut self, a: Pf) {
        let mut t = self.target_speed;
        if self.group == 4 && self.f65a == 0 {
            if self.jump.blocked != 0 && self.cap_hit != 0 { t = DT * Pf::b(0x4006_6666); }
        } else if self.f65a != 0 && self.state == state::DOUBLE_JUMP && DT < t {
            t = DT;
        }
        let dx = fast_cos(self.target_yaw) * t;
        let dy = fast_sin(self.target_yaw) * t;
        let d = [dx, dy, self.vel[2], self.vel[3]];
        let c = -(self.vel[0] * dx + self.vel[1] * dy);
        let ld = len2(d);
        let lv = len2(self.vel);
        let c = if ld == Pf::ZERO || lv == Pf::ZERO { Pf::ZERO } else { (c / ld) / lv };
        let mut f = (c + Pf::ONE) * Pf::b(0x3fc0_0000);
        let cap = Pf::b(0x4040_0000);
        if cap < f { f = cap; }
        if f < Pf::ONE { f = Pf::ONE; }
        let a = a * f;
        let mut step = vsub(d, self.vel);
        if a < len2(step) { step = set_len2(step, a); }
        self.vel = vadd(self.vel, step);
        let p = self.push;
        self.speed = self.fwd_speed - push_speed_along(p, self.rot[2]);
    }

    /// The air horizontal control `0x234b40`, default branch (states 6, 7, 9, 0xe): turn (0.04, 0.2, 0x13f764),
    /// 0x233850(0.7, 0), momentum = push, then with a stick target ≥ 0.2·5.7·dt AirAccel(20·dt²); below
    /// that AirAccel(2·dt²) (6·dt² with 0x13f65a), except in the fall group, where |vel.xy| approaches 0 by
    /// 0x13fc94 instead.
    pub fn air_control(&mut self, env: &Env) {
        let _ = env;
        if self.state == state::LONG_JUMP {
            self.long_jump_air();
            return;
        }
        self.turn_to(SCALE64 * Pf::b(0x3d23_d70a), SCALE64 * Pf::b(0x3e4c_cccd), self.jump.turn_max);
        self.drag(Pf::b(0x3f33_3333), DT * Pf::ZERO);
        self.momentum = self.push;
        let mut a = DT2 * Pf::b(0x41a0_0000);
        if self.target_speed < (SPEED_TABLE[1][2] * DT) * Pf::b(0x3e4c_cccd) {
            if self.group == 2 {
                // Fall group, small stick: no air control, the horizontal speed decays by 0x13fc94 per tick
                // (0x235150; the decompiler drops this branch).
                let mut l = len2(self.vel);
                approach(Pf::ZERO, self.fc94, &mut l);
                self.vel = set_len2(self.vel, l);
                return;
            }
            a = if self.f65a == 0 { DT2 + DT2 } else { DT2 * Pf::b(0x40c0_0000) };
        }
        self.air_accel(a);
    }

    /// 0x234b40 for 0xb (long jump): fixed direction 0x13f788 at 0x13f7a8 plus the facing component.
    fn long_jump_air(&mut self) {
        self.target_yaw = self.jump.face_yaw;
        self.turn_to(SCALE64 * Pf::b(0x3d23_d70a), SCALE64 * Pf::b(0x3e4c_cccd), DT * Pf::b(0x40df_66f3));
        self.speed = if self.jump.kind7a0 == 3 { Pf::ZERO } else { self.jump.side_speed };
        let mut f = self.jump.fwd_speed;
        if self.jump.descending == 0 || ticks(12) <= self.jump.land_eta {
            approach(DT * Pf::b(0x4086_6666), DT2 * Pf::b(0x4100_0000), &mut f);
        } else {
            approach(Pf::ZERO, DT2 * Pf::b(0x4100_0000), &mut f);
        }
        self.jump.fwd_speed = f;
        self.vel[0] = fast_cos(self.jump.dir_yaw) * self.jump.fwd_speed;
        self.vel[1] = fast_sin(self.jump.dir_yaw) * self.jump.fwd_speed;
        let s = self.jump.side_speed * fast_sin(crate::pad::fast_diff_rots(self.jump.face_yaw, self.jump.dir_yaw));
        self.vel[0] = self.vel[0] + fast_cos(self.jump.face_yaw) * s;
        self.vel[1] = self.vel[1] + fast_sin(self.jump.face_yaw) * s;
        self.drag(Pf::b(0x3f33_3333), DT * Pf::ZERO);
        self.momentum = self.push;
    }
}

/// Line query helper (`CollLine_Fix(a, b, flags, 0, 0)`), PS2 float in/out, world mesh only (the hero's own
/// queries use [`Env::line`], which adds the moby pass).
pub fn line_world(coll: &Collision, a: V4, b: V4, flags: u32) -> Option<crate::collision_query::CollOutput> {
    coll_line_m(coll, None, to_f32x3(a), to_f32x3(b), QueryFlags(flags), None)
}

// ================================================================================================
// Per-state physics 0x2370b8 (on-foot cases).

impl Hero {
    /// `0x2370b8`: the per-state physics of the current state through the registry
    /// ([`Hero::dispatch_physics`]: ground.rs, walk.rs, air.rs, jump.rs, melee.rs, swim.rs and the package
    /// modules). Returns false for a state without ported physics (the caller freezes the hero). The water effects
    /// at the top of the ground and walk / wade cases (wake, bow rings, spray, feet bubbles) run first
    /// ([`super::swim::effects::physics_prologue`]).
    pub fn state_physics(&mut self, env: &Env, anim: &mut dyn super::anim::AnimCtl, rng: &mut crate::rng::Rng) -> bool {
        let substate0 = self.substate;
        let seq0 = anim.view().seq_b;
        if super::registry::implemented(self.state) { super::swim::effects::physics_prologue(self, &*anim, rng); }
        if !self.dispatch_physics(env, anim, rng) { return false; }
        // 0x23c3f4: the substate / sequence timers restart when those changed during the tick.
        if substate0 != self.substate { self.substate_timer = 0; }
        if seq0 != anim.view().seq_b { self.seq_timer = 0; }
        true
    }
}

impl Hero {
    /// 0x22a620(target_frame, ticks, k, step): the playback speed that reaches `target_frame` in `ticks`
    /// ticks: distance = target − frame (or, when already past it, target + (count·c − frame) with
    /// c = 2 while blending, else 0.5 / rate); `speed = (k·distance) / (ticks·r)` with r = rate (frame B's
    /// rate while blending); `step` = −1 sets it, otherwise the speed approaches it by `step`.
    pub fn anim_speed_for_ticks(&mut self, target: Pf, ticks_: Pf, k: Pf, step: Pf, v: &super::anim::AnimView) {
        let cur = Pf::f(v.frame);
        let dist = if cur < target {
            target - cur
        } else {
            let c = if v.blending() { Pf::b(0x4000_0000) } else { Pf::b(0x3f00_0000) / Pf::f(v.rate) };
            target + (c * Pf::from_i32(v.frame_count_b as i32) - cur)
        };
        if !(Pf::ZERO < ticks_) { return; }
        let r = if v.blending() { Pf::f(v.frame_b_rate) } else { Pf::f(v.rate) };
        let sp = (k * dist) / (ticks_ * r);
        if step == Pf::b(0xbf80_0000) { self.anim_speed = sp; } else { approach(sp, step, &mut self.anim_speed); }
    }
}

// ================================================================================================
// Move + collide (0x23c458 → 0x233de0), capsule (0x231ae0, 0x233d08/0x233940), ground probe 0x232dc0,
// step-up/snap 0x233588, post-move 0x23c710, and the wall / brake / momentum helpers the states call.
// Spec: player_controller.md §5 and "In the port".

use crate::collision_query::{coll_capsule_m, coll_sphere_hero_groups, coll_sphere_m, CollOutput};

/// 0.87266463 (50°): the walkable slope limit.
pub const SLOPE_MAX: Pf = Pf::b(0x3f5f_66f3);

/// Boot 0x125180: unit vector `a·(1/√((x²+y²)+z²))` (VU `vsqrt`, `vdiv 1/s`), w = 0; no zero guard.
pub fn unit(a: V4) -> V4 {
    let n = (a[0] * a[0] + a[1] * a[1]) + a[2] * a[2];
    let s = Pf::ZERO + n.sqrt();
    let q = Pf::ONE / s;
    [a[0] * q, a[1] * q, a[2] * q, Pf::ZERO]
}

/// 0x221360: 3-D distance (`vsub`, squares, `(x+y)+1·z`, `vsqrt`).
pub fn dist3(a: V4, b: V4) -> Pf { len3(vsub(a, b)) }
/// 0x221398: 2-D distance.
pub fn dist2(a: V4, b: V4) -> Pf {
    let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
    Pf::ZERO + (dx * dx + dy * dy).sqrt()
}
/// 0x2493c8: slope angle of a raw normal, `FastArcTan(n.z, |n.xy|)` = atan2(|n.xy|, n.z).
pub fn slope_of(n: V4) -> Pf { fast_arctan(n[2], len2(n)) }

fn raw_normal(o: &CollOutput) -> V4 { from_f32x3(o.normal) }

impl Hero {
    /// `0x233850(a, b)`: the knockback push `push = sph(0x13f680, yaw 0x13f688, pitch 0x13f684)·a`, shortened
    /// by b (0 when |push| ≤ b). With no knockback (0x13f680 = 0) it is 0.
    pub fn drag(&mut self, a: Pf, b: Pf) {
        let [r, pitch, yaw] = self.knock;
        let cp = fast_cos(pitch);
        let p = [(fast_cos(yaw) * r) * cp, (fast_sin(yaw) * r) * fast_cos(pitch), fast_sin(pitch) * r, self.push[3]];
        let p = vscale(p, a);
        let l = len3(p);
        self.push = if l <= b { [Pf::ZERO, Pf::ZERO, Pf::ZERO, Pf::ZERO] } else { set_len3(p, l - b) };
    }

    /// `0x236678(dec)`: the momentum shrinks by `dec` (xy length) and is added to vel; below 0.001 it is cleared.
    pub fn momentum_decay(&mut self, dec: Pf) {
        let h = len2(self.momentum);
        if Pf::b(0x3a83_126f) < h {
            self.momentum[2] = Pf::ZERO;
            let mut l = h - dec;
            if l < Pf::ZERO { l = Pf::ZERO; }
            self.momentum = set_len2(self.momentum, l);
            self.vel = vadd(self.vel, self.momentum);
        } else {
            self.momentum = V0;
        }
    }

    /// `0x2342d8(k)`: scale disp, eff, vel, momentum (xyz) and the speed.
    pub(super) fn scale_motion(&mut self, k: Pf) {
        self.disp = vscale(self.disp, k);
        self.eff = vscale(self.eff, k);
        self.vel = vscale(self.vel, k);
        self.momentum = vscale(self.momentum, k);
        self.speed = self.speed * k;
    }

    /// Edge brake `0x236a68(dist, xmin)`: probe the ground at `pos + vel·dist` (line from +0.3 to −0.2, flags
    /// 4); no walkable ground there → all motion ×0.
    pub fn edge_brake(&mut self, env: &Env, dist: Pf, xmin: Pf) {
        if len2(self.vel) == Pf::ZERO { return; }
        if Pf::b(0x3f00_0000) < self.stick_mag && self.group != 6 && self.edge_brake == 0 && self.f0637 == 0 { return; }
        if self.air_ticks != 0 { return; }
        let mut p = vscale(self.vel, dist);
        if Pf::ZERO < xmin && len2(p) < xmin {
            p = set_len2(p, Pf::ZERO);
            p[0] = p[0] + fast_cos(self.moby_rot[2]) * xmin;
            p[1] = p[1] + fast_sin(self.moby_rot[2]) * xmin;
        }
        p[2] = Pf::ZERO;
        p = vadd(p, self.pos);
        let mut q = p;
        p[2] = p[2] + Pf::b(0x3e99_999a);
        let down = if self.mode == 2 { Pf::b(0xbf33_3333) } else { Pf::b(0xbe4c_cccd) };
        q[2] = q[2] + down;
        let ok = match env.line(p, q, 4) {
            Some(o) => o.kind > 0 && slope_of(raw_normal(&o)) <= SLOPE_MAX,
            None => false,
        };
        if !ok { self.scale_motion(Pf::ZERO); }
    }

    /// Wall check `0x232978(mode)`: two short lines ahead (low: feet + 0.111, 0.7 long, surface 8/0xc; high:
    /// feet + 0.37, 0.5 long, slope ≥ 50°) along vel (or the facing with `mode` ≠ 0 or |vel.xy| < 0.01);
    /// on a wall the velocity component into its horizontal normal is removed.
    pub fn wall_check(&mut self, env: &Env, mode: i32) {
        if self.f658 != 0 { return; }
        if len2(self.vel) < DT * Pf::b(0x3dcc_cccd) { return; }
        // The water groups, then the bodies (L00 0x212318: Clank 0.35 up / 0.27 long, Giant Clank 1.5 / 5.0).
        let (up, l) = if self.group == 0x11 || self.group == 0x12 {
            (Pf::ZERO, Pf::b(0x3f33_3333))
        } else if self.mode == 1 {
            (Pf::b(0x3eb3_3333), Pf::b(0x3e8a_3d71))
        } else if self.mode == 2 {
            (Pf::b(0x3fc0_0000), Pf::b(0x40a0_0000))
        } else {
            (Pf::b(0x3ebd_70a4), Pf::b(0x3f00_0000))
        };
        let mut a = self.pos;
        a[2] = self.pos[2] + up;
        let mut bq = self.pos;
        bq[2] = self.pos[2] + up * Pf::b(0x3e99_999a);
        let k14 = Pf::b(0x3fb3_3333);
        let (d1, d2) = if mode != 0 || len2(self.vel) < Pf::b(0x3c23_d70a) {
            let (c, s) = (fast_cos(self.rot[2]) * l, fast_sin(self.rot[2]) * l);
            ([c, s, Pf::ZERO, Pf::ZERO], [c * k14, s * k14, Pf::ZERO, Pf::ZERO])
        } else {
            let mut d = self.vel;
            d[2] = Pf::ZERO;
            let d = vscale(d, l / len2(d));
            (d, vscale(d, k14))
        };
        let e1 = vadd(d1, a);
        let e2 = vadd(d2, bq);
        let mut last: Option<CollOutput> = None;
        let mut sid = -1;
        if let Some(o) = env.line(bq, e2, 4) {
            if o.kind > 0 { sid = o.surface_id(); }
            last = Some(o);
        }
        let mut s = Pf::ZERO;
        if let Some(o) = env.line(a, e1, 4) {
            if o.kind > 0 { s = slope_of(raw_normal(&o)); }
            last = Some(o);
        }
        if !(SLOPE_MAX <= s) && sid != 8 && sid != 0xc { return; }
        let Some(o) = last else { return };
        // 0x13f590 = the last line's moby (0x1742d8).
        self.wall_moby = o.moby;
        let mut n = raw_normal(&o);
        n[2] = Pf::ZERO;
        let n = unit(n);
        let k = ((-self.vel[0]) * n[0]) - (self.vel[1] * n[1]);
        if Pf::ZERO < k { self.vel = vadd(self.vel, vscale(n, k)); }
    }

    /// `0x232820`: just airborne, rising, touching a ≥ 50° contact: remove the into-wall part and pull down
    /// by 108·dt².
    pub fn climb_check(&mut self, _env: &Env) {
        if self.air_ticks == 0 || !(self.height < Pf::b(0x3e19_999a)) { return; }
        if !(Pf::ZERO < self.disp[2]) || self.cap_hit == 0 || !(Pf::ZERO < self.contact_normal[2]) { return; }
        if !(SLOPE_MAX <= slope_of(self.contact_normal)) { return; }
        let mut n = self.contact_normal;
        n[2] = Pf::ZERO;
        let n = unit(n);
        let k = ((-self.vel[0]) * n[0]) - (self.vel[1] * n[1]);
        if Pf::ZERO < k {
            self.vel = vadd(self.vel, vscale(n, k));
            let g = DT2 * Pf::b(0x4258_0000);
            self.vel[2] = self.vel[2] - (g + g);
        }
    }

    /// Capsule sizing `0x231ae0`.
    pub fn size_capsule(&mut self) {
        let c = self.mode;
        if c == 0 { self.cap_bottom_target = Pf::b(0x3f33_3333); } else if c == 3 { self.cap_bottom_target = Pf::b(0x3f19_999a); }
        if c == 0 || c == 3 { self.cap_top_target = Pf::b(0x3f4c_cccd); self.cap_radius_target = Pf::b(0x3ee6_6666); }
        // The other bodies (L00 0x211380, super::bodies): Clank bottom 0.45 / radius 0.3 / top 0.6, Giant Clank 4.45 / 3.75 /
        // 5.25.
        if c == 1 {
            (self.cap_bottom_target, self.cap_radius_target, self.cap_top_target) = (Pf::b(0x3ee6_6667), Pf::b(0x3e99_999a), Pf::b(0x3f19_9999));
        } else if c == 2 {
            (self.cap_bottom_target, self.cap_radius_target, self.cap_top_target) = (Pf::b(0x408e_6666), Pf::b(0x4070_0000), Pf::b(0x40a8_0000));
        }
        if self.group == 4 {
            if self.jump.takeoff < self.timer && self.jump.descending == 0 { self.cap_bottom_target = self.jump.bottom784; }
        } else if self.state == 6 {
            self.cap_bottom_target = Pf::b(0x3f00_0000);
        } else if self.state == 4 {
            self.cap_top_target = Pf::b(0x3eb3_3334);
        } else if self.group == 0x11 || self.group == 0x12 {
            self.cap_bottom_target = Pf::ZERO;
            self.cap_top_target = Pf::ZERO;
        } else if super::surface::slippery_capsule(self) {
            // Grounded on a slippery floor: the raised capsule (level00; super::surface).
        } else if self.state == 0x7f {
            self.cap_radius_target = Pf::b(0x3f4c_cccd);
        }
        let ease = self.cap_hit == 0 || self.prev_group == 0x12 || self.group == 0x11 || self.f0634 != 0
            || self.cap_radius * Pf::b(0x3f00_0000) < dist2(self.contact_point, self.pos);
        if ease {
            let step = SCALE60 * Pf::b(0x3ca3_d70a);
            approach(self.cap_top_target, step, &mut self.cap_top);
            approach(self.cap_bottom_target, step, &mut self.cap_bottom);
            let (mut r, mut v) = (self.cap_radius, self.cap_radius_vel);
            spring(self.cap_radius_target, SCALE64 * Pf::b(0x3ca3_d70a), SCALE64 * Pf::b(0x3e99_999a), DT * Pf::b(0x4080_0000), &mut r, &mut v);
            self.cap_radius = r;
            self.cap_radius_vel = v;
        }
    }

    /// `0x233940(try)`: up to 8 capsule passes; `false` when the total push exceeds 1.5·r.
    fn capsule_try(&mut self, env: &Env) -> bool {
        if self.no_vel_clamp != 0 { return true; }
        let old = self.pos;
        // The spline wall 0x14162a (`FUN_00249e58`): off the spline a class named this tick, then cleared.
        if self.wall_spline != 0 {
            let pts = env.world.and_then(|w| w.spline(self.wall_spline as u16 as usize));
            if let Some(p) = pts.and_then(|pts| spline_wall(self.cap_radius.to_f32(), pts, to_f32x3(self.pos), to_f32x3(self.gravity_dir))) {
                self.pos = [Pf::f(p[0]), Pf::f(p[1]), Pf::f(p[2]), self.pos[3]];
                self.f546 = 4;
            }
            self.wall_spline = 0;
        }
        let mut off = V0;
        if self.group != 0x11 {
            if self.group == super::hoverboard::GROUP {
                // The Hoverboard: the sphere's centre 0.7 up the body (`0x248cf8(0, 0, 0.7)` − the feet).
                off = vsub(from_f32x3(super::packs::local(self, [0.0, 0.0, 0.7])), self.pos);
            } else if self.gravity_mode == 0 && self.f548 == 0 {
                off[2] = self.cap_bottom;
            } else if self.gravity_mode == 1 || self.f548 != 0 || self.group == 0x15 {
                // `0x248ea8(0.6)`: up the world z in mode 0, in modes 1 / 2 up the moby's own z axis (+0x40),
                // so on a tilted magnetic floor the sphere stays 0.6 off it.
                if matches!(self.gravity_mode, 1 | 2) {
                    off = vadd(off, mul_rows4(&self.rows, [Pf::ZERO, Pf::ZERO, Pf::b(0x3f19_999a), Pf::ZERO]));
                } else {
                    off[2] = off[2] + Pf::b(0x3f19_999a);
                }
            } else {
                off[2] = off[2] - (-self.cap_bottom);
            }
        }
        self.pos = vadd(self.pos, off);
        // 0x24 on level01; the surface passed through is per level (super::surface::PASS_SURFACE).
        let flags = if self.state == 0x7f { QueryFlags(0xd24) } else { super::surface::pass_flags(self.idle.level) };
        let r = self.cap_radius;
        for _ in 0..8 {
            // The capsule / sphere kernels with a2 = Ratchet (0x1413d0): world mesh, then the mobys of `env.mobys`.
            let hit = {
                let (sc, ig) = (env.mobys, env.hero_moby);
                if self.gravity_mode != 0 {
                    coll_sphere_m(env.coll, sc, to_f32x3(self.pos), (SCALE60 * Pf::b(0x3ecc_cccd)).to_f32(), flags, ig)
                } else if self.group == 0x11 {
                    coll_sphere_m(env.coll, sc, to_f32x3(self.pos), 0.6, flags, ig)
                } else if self.group == super::hoverboard::GROUP {
                    coll_sphere_m(env.coll, sc, to_f32x3(self.pos), (SCALE60 * Pf::b(0x3f00_0000)).to_f32(), flags, ig)
                } else if self.group == 0xf {
                    // Grinding (level00 0x2133a8; level01 has no grind): a 0.45 sphere (super::boots).
                    coll_sphere_m(env.coll, sc, to_f32x3(self.pos), (SCALE60 * Pf::b(0x3ee6_6666)).to_f32(), flags, ig)
                } else {
                    let mut h = self.cap_top - self.cap_bottom;
                    if h < Pf::b(0x3d4c_cccd) { h = Pf::b(0x3d4c_cccd); }
                    let mut o = coll_capsule_m(env.coll, sc, to_f32x3(self.pos), h.to_f32(), r.to_f32(), flags, ig);
                    // Hero-only collision groups (0x214d70), from the same position; a hit replaces the output.
                    if let Some(g) = coll_sphere_hero_groups(env.coll, to_f32x3(self.pos), r.to_f32()) { o = Some(g); }
                    o
                }
            };
            let Some(o) = hit else { break };
            let pc = o.pushed_centre.expect("sphere/capsule hits carry a pushed centre");
            self.pos = [Pf::f(pc[0]), Pf::f(pc[1]), Pf::f(pc[2]), self.pos[3]];
            self.contact_normal = raw_normal(&o);
            self.contact_point = from_f32x3(o.point);
            self.cap_moby = o.moby;
            self.cap_hit = 1;
        }
        self.pos = vsub(self.pos, off);
        let push = vsub(self.pos, old);
        !(r * Pf::b(0x3fc0_0000) < len3(push))
    }

    /// `0x233d08`: resolve, rolling back through the position history (up to 16 tries) when rejected.
    fn capsule_resolve(&mut self, env: &Env) {
        for _ in 0..16 {
            if self.capsule_try(env) { return; }
            let n = self.pos_hist_count;
            if n >= 2 {
                self.pos_hist_count = n - 1;
                self.pos_hist_idx = (self.pos_hist_idx + 31) & 31;
            }
            self.pos = self.pos_hist[self.pos_hist_idx as usize];
        }
    }

    /// Ground probe `0x232dc0`.
    pub fn ground_probe(&mut self, env: &Env) {
        self.ground_normal = [Pf::ZERO, Pf::ZERO, Pf::ONE, self.ground_normal[3]];
        self.air_ticks = (self.air_ticks as u16).wrapping_add(1) as i16;
        self.f65c = self.f65c.wrapping_add(1);
        self.slope = Pf::ZERO;
        self.ground_z = Pf::ZERO;
        self.ground_point = V0;
        self.f65a = 0;
        self.height = Pf::b(0x4200_0000);
        self.ground_moby = None;
        if self.in_water != 0 { self.in_water += 1; }
        let r = self.cap_radius;
        let up = if self.group == 2 { r * Pf::b(0x4039_999a) } else { r * Pf::b(0x400c_cccd) };
        let mut p1 = self.pos;
        p1[2] = self.pos[2] + Pf::b(0xc240_0000);
        if p1[2] < Pf::ZERO { p1[2] = Pf::ZERO; }
        let mut p0 = self.pos;
        p0[2] = self.pos[2] + up;
        // Gravity modes ≠ 0: the line runs along the gravity direction 0x13f5e0 (0x232cc0, super::boots).
        if self.gravity_mode != 0 { (p0, p1) = super::boots::frame_probe_line(self, up); }
        let mut hit = env.line(p0, p1, 2);
        'probe: {
            let Some(mut o) = hit else { break 'probe };
            self.footstep = o.sound_class() as u8;
            self.surface_id = o.surface_id() as i16;
            if self.surface_id as u16 == 0 {
                // Water surface: the level from the level's water tables at the hit (0x26ed38: ripple patch /
                // flat plane, `Env::water`), else the hit's z; re-cast through it.
                self.water_level = Pf::f(env.water.and_then(|w| w.water_height(o.point)).unwrap_or(o.point[2]));
                if self.pos[2] < self.water_level && self.in_water <= 0 { self.in_water = 1; }
                hit = env.line(p0, p1, 0x24);
                let Some(o2) = hit else { self.gravity_frame(env); return };
                o = o2;
                // Levels 1 and 0x12 (0x15ed84) walk water with footstep class 3.
                self.footstep = if matches!(self.idle.level, 1 | 0x12) { 3 } else { o.sound_class() as u8 };
            }
            if self.surface_id == 0xd {
                // The re-cast starts just below the liquid's top (the start point's z: sp+0x18 at 0x232fe8), so
                // it finds the floor under the liquid, not the liquid again.
                let z = Pf::f(o.point[2]);
                self.water_level = z;
                p0[2] = z - Pf::b(0x3c23_d70a);
                match env.line(p0, p1, 4) { Some(o3) => o = o3, None => { self.gravity_frame(env); return; } }
            }
            // Surfaces 3 / 0xb (the liquid level 0x13f644) and 1 under the burn deaths (level00's probe): super::surface.
            if !super::surface::probe_surface(self, env, &mut p0, p1, &mut o) { self.gravity_frame(env); return; }
            if self.surface_id == 0xb {
                self.gravity_frame(env);
                return;
            }
            // A moby under the feet (`0x232dc0`): a crate (classes 500..540, `0x273278`) or a pvar record with bit 3 of +0x1e
            // sets 0x13f65a; on a crate the Hoverboard's groups take the ground 1 lower (`0x248b68(1, hit, hit)`).
            if let Some(m) = o.moby {
                if env.mobys.and_then(|sc| sc.mobys.moby(m)).is_some_and(|x| (500..=540).contains(&x.o_class)) {
                    self.f65a = 1;
                    if matches!(self.group, 0x15 | 0x16) { o.point[2] -= 1.0; }
                }
                if env.world.is_some_and(|w| w.moby_record_flags(m) & 8 != 0) { self.f65a = 1; }
            }
            if o.kind <= 0 { break 'probe; }
            self.ground_point = from_f32x3(o.point);
            self.ground_z = self.ground_point[2];
            self.height = dist3(self.pos, self.ground_point);
            self.ground_moby = o.moby;
            let raw = raw_normal(&o);
            self.ground_normal = unit(raw);
            self.slope = slope_of(raw);
            self.slope_yaw = fast_arctan(raw[0], raw[1]);
            if Pf::ZERO < dot3(vsub(self.ground_point, self.pos), self.ground_normal) { self.height = -self.height; }
            if self.height < Pf::b(0x3ca3_d70a) {
                self.f65c = 0;
                if self.slope <= SLOPE_MAX || self.gravity_mode == 1 || self.group == 0x16 { self.air_ticks = 0; }
            }
        }
        if self.air_ticks != 0 {
            self.grounded_ticks = 0;
        } else {
            self.grounded_ticks += 1;
            self.last_ground_point = self.ground_point;
        }
        if self.water_level < self.pos[2] { self.in_water = 0; }
        self.gravity_frame(env);
    }

    /// The tail of the probe (0x2331e8): gravity direction and the ground pitch / roll under the hero.
    fn gravity_frame(&mut self, env: &Env) {
        self.gravity_dir = [Pf::ZERO, Pf::ZERO, Pf::b(0xbf80_0000), Pf::ZERO];
        // Gravity mode 1: against the ground normal (the Magneboots, super::boots).
        if self.gravity_mode == 1 { self.gravity_dir = [-self.ground_normal[0], -self.ground_normal[1], -self.ground_normal[2], Pf::ZERO]; }
        self.pitch = Pf::ZERO;
        self.roll = Pf::ZERO;
        // The side probes 0x13f610..0x13f624 are cleared on every probe (super::pose).
        self.idle.side = super::pose::SideProbes::default();
        if matches!(self.group, 0xf | 0x15 | 6 | 4 | 5 | 3) { return; }
        if !(self.height < Pf::b(0x3e80_0000)) { return; }
        let (px, py) = self.frame_tilt(self.ground_normal);
        let q = Pf::b(0x3f49_0fdb);
        if -q < py && py < q { self.pitch = py; }
        if -q < px && px < q { self.roll = px; }
        // The two side probes (0x13f610..0x13f624) for the model's slope tilt (`0x22c5c0`): super::pose.
        self.side_probes(env);
    }

    /// `0x235fe0(n, out)`: the normal in the moby's frame (transposed rows), `out[0]` = atan2(l.y, h) (the roll
    /// 0x13f638), `out[1]` = −atan2(l.x, l.z) (the pitch 0x13f634).
    pub(super) fn frame_tilt(&self, n: V4) -> (Pf, Pf) {
        let m = euler_rows(self.moby_rot);
        let mt: [V4; 4] = [
            [m[0][0], m[1][0], m[2][0], Pf::ZERO],
            [m[0][1], m[1][1], m[2][1], Pf::ZERO],
            [m[0][2], m[1][2], m[2][2], Pf::ZERO],
            [Pf::ZERO, Pf::ZERO, Pf::ZERO, Pf::ONE],
        ];
        let l = mul_rows4(&mt, n);
        let h = Pf::ZERO + (l[0] * l[0] + l[2] * l[2]).sqrt();
        (fast_arctan(h, l[1]), -fast_arctan(l[2], l[0]))
    }

    /// Step-up / snap `0x233588` (mode 0).
    fn step_snap(&mut self) {
        // Gravity modes ≠ 0: along the normal (super::boots).
        if self.gravity_mode != 0 { return super::boots::frame_snap(self); }
        if self.air_ticks != 0 || !(self.pos[2] < self.ground_z) {
            self.f654 = Pf::ZERO;
            return;
        }
        let depth = (self.pos[2] - self.ground_z).abs();
        let lim = self.vel[2].abs() + Pf::b(0x3c23_d70a);
        if depth < lim {
            if self.cap_hit == 0 || self.cap_radius * Pf::b(0x3f00_0000) < dist2(self.contact_point, self.pos) {
                self.pos[2] = self.ground_z;
                return;
            }
        } else {
            let dz = self.disp[2];
            if dz < Pf::ZERO {
                self.pos[2] = self.pos[2] - dz;
                if self.ground_z < self.pos[2] { self.pos[2] = self.ground_z; }
            }
        }
        let (mut z, mut v) = (self.pos[2], self.f654);
        spring(self.ground_z, SCALE64 * Pf::b(0x3d69_78d5), SCALE64 * Pf::b(0x3e99_999a), DT + DT, &mut z, &mut v);
        self.pos[2] = z;
        self.f654 = v;
    }

    /// `eff = vel·(max(0, unit(disp)·vel) / |vel|)` (0x221410 normalisers).
    fn project(disp: V4, vel: V4) -> V4 {
        let d = set_len3(disp, Pf::ONE);
        let mut f = dot3(d, vel);
        if f < Pf::ZERO { f = Pf::ZERO; }
        set_len3(vel, f)
    }

    /// The move `0x233de0`: the platform carry `HeroPlatformUpdate` 0x249618 (super::platform), clamp,
    /// integrate, capsule, ground probe, snap, effective velocity, the platform step, slope ratio, speed cap.
    pub fn move_collide(&mut self, env: &Env) {
        super::platform::platform_update(self, env);
        let old = self.pos;
        // 1. Velocity clamp to r − 0.02 (3-D).
        if self.state == 0x22 || self.state == 0x14 {
            let l = self.cap_radius - Pf::b(0x3ca3_d70a);
            if self.no_vel_clamp == 0 && l < len2(self.vel) { self.vel = set_len3(self.vel, l); }
            let mut m = -self.height;
            if self.vel[2] < m {
                if Pf::ZERO < m { m = Pf::ZERO; }
                self.vel[2] = m;
            }
        } else if self.group == super::hoverboard::GROUP {
            // The Hoverboard (levels 5 / 16, `0x2413e0`): clamped in xy only when a line along the velocity (from the
            // feet to 0.5 up + vel) hits the world or a moby of class 0x1f6 / 0x59f.
            let mut b = self.pos;
            b[2] = self.pos[2] + Pf::b(0x3f00_0000);
            let b = vadd(b, self.vel);
            let hit = env.line(self.pos, b, 2).is_some_and(|o| match o.moby {
                None => true,
                Some(m) => env.mobys.and_then(|sc| sc.mobys.moby(m)).is_some_and(|x| x.o_class == 0x1f6 || x.o_class == 0x59f),
            });
            let l = self.cap_radius - Pf::b(0x3ca3_d70a);
            if hit && self.no_vel_clamp == 0 && l < len2(self.vel) { self.vel = set_len2(self.vel, l); }
        } else if self.no_vel_clamp == 0 {
            let l = self.cap_radius - Pf::b(0x3ca3_d70a);
            if l < len3(self.vel) { self.vel = set_len3(self.vel, l); }
        }
        // 2. Integrate.
        self.pos = vadd(self.pos, self.vel);
        self.pos = vadd(self.pos, self.push);
        self.push = V0;
        self.cap_moby = None;
        self.cap_hit = 0;
        let platform = Pf::b(0x38d1_b717) < len3(self.platform);
        if !platform {
            self.capsule_resolve(env);
            self.ground_probe(env);
            self.disp = vsub(self.pos, old);
            self.step_snap();
        } else {
            self.ground_probe(env);
        }
        // 3. Effective velocity (from the pre-platform displacement).
        let d = vsub(self.pos, old);
        self.eff = Self::project(d, self.vel);
        let mut hv = self.vel;
        hv[2] = Pf::ZERO;
        let mut dh = d;
        dh[2] = Pf::ZERO;
        self.eff_h = Self::project(dh, hv);
        let vv = set_len2(self.vel, Pf::ZERO);
        let dv = set_len2(d, Pf::ZERO);
        self.eff_v = Self::project(dv, vv);
        self.eff_len = len3(self.eff);
        self.eff_len_xy = len2(self.eff);
        let fs = push_speed_along(self.eff, self.rot[2]);
        self.fwd_speed = if fs < Pf::ZERO { Pf::ZERO } else { fs };
        // 4. Platform / edge-nudge step.
        let pp = self.pos;
        if platform {
            self.pos = vadd(self.pos, self.platform);
            let w = self.platform[3];
            self.platform = [Pf::ZERO, Pf::ZERO, Pf::ZERO, w];
            self.capsule_resolve(env);
            self.ground_probe(env);
            self.disp = vsub(self.pos, old);
            self.step_snap();
        }
        self.plat_applied = vsub(self.pos, pp);
        self.platform[3] = Pf::ZERO;
        self.disp = vsub(self.pos, old);
        // 5. Slope ratio.
        self.slope_ratio = Pf::ZERO;
        if Pf::b(0x3b83_126f) < self.eff_len_xy {
            let q = self.disp[2] / self.eff_len_xy;
            self.slope_ratio = q;
            if Pf::b(0x3f00_0000) < q { self.slope_ratio = Pf::b(0x3f00_0000); } else if q < Pf::b(0xbf00_0000) { self.slope_ratio = Pf::b(0xbf00_0000); }
        }
        // 6. Speed cap 52·dt on the displacement record.
        let c = DT * Pf::b(0x4250_0000);
        if c < self.eff_len {
            self.disp = vscale(self.disp, c / self.eff_len);
            self.eff_len = DT * Pf::b(0x4250_0000);
        }
    }

    /// The wall-ahead probe of `0x23c458` after the move pipeline (moved or frozen), every third tick
    /// (`0x15f5cc % 3 == 0`): `0x22b628(0.7, 4.0)` — a line (flags 2) 0.7 above the feet from r − 0.02 to 4.0
    /// ahead in the hero's frame; a hit sets 0x13f598 = the xy distance feet → hit point, 0x13f5a0 = the elevation
    /// of the hit normal (`FastArcTan(n.z, |n.xy|)`), 0x13f5a5 = a moby was hit, 0x13f5a4 = that moby is a crate
    /// (class 500..=540, `0x273278`); no hit: 4.0 and both flags 0 (0x13f5a0 keeps its value). Then every fifth tick
    /// the edge probe ([`Hero::edge_probe`]). Native `f32`.
    pub fn wall_ahead_probe(&mut self, env: &Env) {
        self.wall_probe(env);
        if self.idle.counter.wrapping_sub(1).rem_euclid(5) == 0 { self.edge_probe(env); }
    }

    /// The edge probe of `0x23c458` (every fifth tick): 0x13f5b0 = 0, 0x13f5a8 = 0; on the ground (0x13f650 ≠ 0) a line
    /// (flags 2) from the hero-local (1.1, 0, 1) down to (1.1, 0, −20) (its end at world z ≥ 0.5); no floor within 3 of
    /// the start: spheres of the capsule radius (0x13f584) at (1.1 + d, 0, 0) for d = −0.4, −0.3, … until one is free
    /// (none free before d reaches 0.5: no edge); then 0x13f5b0 = 1, 0x13f5a8 = the drop (20 without a floor),
    /// 0x13f5ac = radius + d (≥ 0).
    pub fn edge_probe(&mut self, env: &Env) {
        self.edge = (false, 0.0, self.edge.2);
        if self.grounded_ticks == 0 { return; }
        let r = self.rows.map(to_f32x3);
        let pos = to_f32x3(self.pos);
        let local = |v: [f32; 3]| -> [f32; 3] { std::array::from_fn(|k| ((r[0][k] * v[0] + r[1][k] * v[1]) + r[2][k] * v[2]) + pos[k]) };
        let a = local([1.1, 0.0, 1.0]);
        let mut b = local([1.1, 0.0, -20.0]);
        if b[2] < 0.5 { b[2] = 0.5; }
        let mut depth = 20.0;
        let hit = env.line(from_f32x3(a), from_f32x3(b), 2);
        if let Some(o) = &hit {
            depth = ((o.point[0] - a[0]).powi(2) + (o.point[1] - a[1]).powi(2) + (o.point[2] - a[2]).powi(2)).sqrt();
            if depth <= 3.0 { return; }
        }
        let rad = self.cap_radius.to_f32();
        let mut d = -0.4f32;
        loop {
            let c = local([1.1 + d, 0.0, 0.0]);
            if crate::collision_query::coll_sphere_m(env.coll, env.mobys, c, rad, crate::collision_query::QueryFlags(2), None).is_none() { break; }
            d += 0.1;
            if 0.5 <= d { return; }
        }
        self.edge = (true, if hit.is_none() { 20.0 } else { depth }, (rad + d).max(0.0));
    }

    fn wall_probe(&mut self, env: &Env) {
        if self.idle.counter.wrapping_sub(1).rem_euclid(3) != 0 { return; }
        self.wall_ahead[0] = 4.0;
        self.f5a4 = 0;
        self.f5a5 = 0;
        let r = self.rows.map(to_f32x3);
        let pos = to_f32x3(self.pos);
        let local = |fwd: f32| from_f32x3([0, 1, 2].map(|k| (r[0][k] * fwd + r[2][k] * 0.7) + pos[k]));
        let Some(o) = env.line(local(self.cap_radius.to_f32() - 0.02), local(4.0), 2) else { return };
        self.wall_ahead[0] = ((o.point[0] - pos[0]).powi(2) + (o.point[1] - pos[1]).powi(2)).sqrt();
        let n = from_f32x3(o.normal);
        self.wall_ahead[1] = fast_arctan(n[2], len2(n)).to_f32();
        if let Some(id) = o.moby {
            self.f5a5 = 1;
            let crate_ = env.mobys.and_then(|m| m.mobys.moby(id)).is_some_and(|m| (500..=540).contains(&m.o_class));
            if crate_ { self.f5a4 = 1; }
        }
    }

    /// Post-move `0x23c710` (timers, stuck-in-air counter, edge nudge, body/shadow points, death plane).
    /// The Euler x/y straightening 0x236520 is a no-op on foot (x = y = 0) and not ported.
    pub fn post_move(&mut self, env: &Env) {
        if self.frozen == 0 {
            // Gravity modes 1 / 2 or the raised capsule 0x13f548: the frame aligns with the floor (0x236358,
            // super::boots); else the straightening 0x236520.
            if self.gravity_mode == 1 || self.gravity_mode == 2 || self.f548 != 0 { super::boots::frame_align(self); } else { self.straighten_euler(); }
        }
        self.timer += 1;
        self.f4ec += 1;
        self.substate_timer += 1;
        self.seq_timer += 1;
        fn dec_i(t: &mut i32) { if *t != 0 { *t = (*t).max(1) - 1; } }
        fn dec_s(t: &mut i16) { if *t != 0 { *t = (*t).max(1) - 1; } }
        dec_i(&mut self.lockout);
        dec_i(&mut self.ledge);
        dec_s(&mut self.f540);
        dec_s(&mut self.jump_lockout);
        dec_i(&mut self.no_vel_clamp);
        dec_s(&mut self.nudge_timer);
        dec_s(&mut self.edge_brake);
        dec_s(&mut self.f548);
        dec_s(&mut self.swim.jump_lock);
        dec_s(&mut self.swim.dive_lock);
        // The other FastDecTimer counters of 0x23c710 (set by the package states; 0 until then).
        dec_i(&mut self.f508);
        dec_i(&mut self.f510);
        dec_i(&mut self.f520);
        dec_i(&mut self.f524);
        dec_s(&mut self.f500);
        dec_s(&mut self.f502);
        dec_s(&mut self.f518);
        dec_s(&mut self.f51a);
        dec_s(&mut self.f530);
        dec_s(&mut self.f534);
        dec_s(&mut self.f536);
        dec_s(&mut self.f53e);
        dec_s(&mut self.f546);
        // 0x13f52c / 0x13f52a: the swap timers `UpdateWrenchSelected` sets (`0x230720` / `0x2306c0`; G-HERO-025).
        dec_s(&mut self.items.f52c);
        dec_s(&mut self.items.f52a);
        // The disguise's timer 0x14162e (super::hologuise): its end is made by the caller (enter / leave body 3).
        let square = env.pad.pressed_within(crate::pad::button::SQUARE, ticks(0x12)).is_some();
        super::hologuise::tick_timer(self, square);
        // 0x13f538 (the fidget cooldown) and the fidget records' +0x60 (0x179d70 + k·0x70).
        dec_s(&mut self.idle.cooldown);
        // Body 2: Giant Clank's beam lockout 0x140986 (super::bodies::giant).
        if self.mode == 2 { dec_s(&mut self.bodies.beam_lock); }
        for t in &mut self.idle.fidget_cool { dec_i(t); }
        self.f50c = if self.f13f8 != 0 { self.f50c + 1 } else { 0 };
        if !env.pad.no_direction { self.f4fc = 0; self.f4f8 += 1; } else { self.f4f8 = 0; self.f4fc += 1; }
        // Stuck-in-air counter 0x13f532.
        let mut reset = true;
        if self.air_ticks != 0 && self.vel[2] < Pf::ZERO {
            if self.pos_hist_count != 32 {
                reset = false;
            } else {
                let idx = self.pos_hist_idx;
                let mut acc = V0;
                for i in 0..8 {
                    let a = self.pos_hist[((idx - i + 31) & 31) as usize];
                    let b = self.pos_hist[((idx - i + 30) & 31) as usize];
                    acc = vadd(acc, vsub(a, b));
                }
                acc = vscale(acc, Pf::ONE / Pf::b(0x4100_0000));
                if len3(acc) < DT {
                    self.f532 = (self.f532 as u16).wrapping_add(1) as i16;
                    reset = false;
                }
            }
        }
        if reset { self.f532 = 0; }
        self.edge_nudge();
        // Body point pos + R·(0, 0, 0.7) (0.4 for Clank, 4.0 for Giant Clank: super::bodies) and the shadow / target point.
        let up = match self.mode { 1 => Pf::b(0x3ecc_cccd), 2 => Pf::b(0x4080_0000), _ => Pf::b(0x3f33_3333) };
        let b = mul_rows4(&self.rows, [Pf::ZERO, Pf::ZERO, up, Pf::ZERO]);
        self.body_point = vadd(b, self.pos);
        if self.pos[2] - self.ground_z < Pf::b(0x4080_0000) {
            self.shadow_point = self.pos;
            self.shadow_point[2] = self.ground_z + Pf::b(0x3f00_0000);
        } else {
            self.shadow_point = self.body_point;
        }
        if self.pos[2] < Pf::ONE { self.fell_out = 1; }
    }

    /// Edge nudge `0x23cc20`: a hero perched on an edge by the capsule is pushed off (through 0x13f440).
    fn edge_nudge(&mut self) {
        let r = self.cap_radius;
        if self.cap_hit != 0 && Pf::ZERO < self.height && self.eff_len < DT * Pf::b(0x4040_0000)
            && len3(self.vel) < DT * Pf::b(0x4040_0000) && self.stick_mag < Pf::b(0x3e4c_cccd)
            && matches!(self.group, 0 | 2 | 4 | 0xa | 1)
        {
            self.nudge_timer = ticks(5) as i16;
        }
        if self.nudge_timer == 0 { self.nudge_speed = Pf::ZERO; return; }
        let d = dist2(self.pos, self.contact_point);
        let lim = r * Pf::b(0x3f66_6666);
        if !(d < lim) { return; }
        let mut yaw = fast_arctan(self.pos[0] - self.contact_point[0], self.pos[1] - self.contact_point[1]);
        if d < r * Pf::b(0x3d4c_cccd) { yaw = self.rot[2]; }
        let mut e = d - lim;
        let mut v = self.nudge_speed;
        spring(Pf::ZERO, SCALE64 * Pf::b(0x3cf5_c28f), SCALE64 * Pf::b(0x3e99_999a), DT * Pf::b(0x4033_3333), &mut e, &mut v);
        self.nudge_speed = v;
        self.platform[0] = fast_cos(yaw) * v;
        self.platform[1] = fast_sin(yaw) * v;
        self.platform[2] = Pf::ZERO;
    }
}

impl Hero {
    /// `0x22a7d0(ax, ay, az, pivot)`: turn the body by the Euler step (ax, ay, az) in its own frame about `pivot` (in the
    /// body's frame), keeping that point in place (`pos += p₀ − p₁`), and write the new Euler angles (`0x2721f0`;
    /// standard `atan2` for the game's `FastArcTan`). `0x22a8d8(ax, ay, az)` is it about (0, 0, 0.6).
    pub(super) fn turn_about(&mut self, e: [f32; 3], pivot: [f32; 3]) {
        let rows = |r: [f32; 3]| -> [[f32; 3]; 3] {
            let m = euler_rows([Pf::f(r[0]), Pf::f(r[1]), Pf::f(r[2]), Pf::ZERO]);
            std::array::from_fn(|i| [m[i][0].to_f32(), m[i][1].to_f32(), m[i][2].to_f32()])
        };
        let r = rows(to_f32x3(self.rot));
        let d = rows(e);
        let mul = |v: [f32; 3], m: &[[f32; 3]; 3]| -> [f32; 3] { std::array::from_fn(|k| v[0] * m[0][k] + v[1] * m[1][k] + v[2] * m[2][k]) };
        let p0 = mul(pivot, &r);
        let r2: [[f32; 3]; 3] = std::array::from_fn(|i| mul(d[i], &r));
        let p1 = mul(pivot, &r2);
        for k in 0..3 { self.pos[k] = Pf::f(self.pos[k].to_f32() + (p0[k] - p1[k])); }
        let z = r2[0][1].atan2(r2[0][0]);
        let y = (-r2[0][2]).atan2((r2[0][0] * r2[0][0] + r2[0][1] * r2[0][1]).sqrt());
        let x = r2[1][2].atan2(r2[2][2]);
        self.rot[0] = Pf::f(x);
        self.rot[1] = Pf::f(y);
        self.rot[2] = Pf::f(z);
    }
}

/// 0x2338e8: the component of `v` along the facing: `(v.x·cos y + v.y·sin y) + v.z·0`.
pub fn push_speed_along(v: V4, yaw: Pf) -> Pf { dot3(v, [fast_cos(yaw), fast_sin(yaw), Pf::ZERO, Pf::ZERO]) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_and_spring() {
        let mut x = Pf::ZERO;
        let r = approach(Pf::ONE, Pf::f(0.25), &mut x);
        assert_eq!(x, Pf::f(0.25));
        assert_eq!(r, Pf::f(0.75));
        approach(Pf::f(-1.0), Pf::f(2.0), &mut x);
        assert_eq!(x, Pf::f(-1.0));
        // Spring snaps within 1% of max.
        let (mut x, mut v) = (Pf::ZERO, Pf::ZERO);
        for _ in 0..400 { spring(Pf::ONE, Pf::f(0.057), Pf::f(0.3), Pf::f(1.0), &mut x, &mut v); }
        assert_eq!((x, v), (Pf::ONE, Pf::ZERO));
    }

    #[test]
    fn rotations_wrap() {
        let a = fast_add_rotations(Pf::f(3.0), Pf::f(0.5));
        assert!((a.to_f32() - (3.5 - 2.0 * std::f32::consts::PI)).abs() < 1e-6);
        let b = fast_subtract_rotations(Pf::f(-3.0), Pf::f(0.5));
        assert!((b.to_f32() - (-3.5 + 2.0 * std::f32::consts::PI)).abs() < 1e-6);
        assert!((fast_cos(Pf::ZERO).to_f32() - 1.0).abs() < 1e-6);
        assert!((fast_sin(Pf::f(1.0)).to_f32() - 1f32.sin()).abs() < 1e-5);
    }

    #[test]
    fn turn_spring_converges() {
        let (mut a, mut v) = (Pf::ZERO, Pf::ZERO);
        let max = DT * Pf::f(9.948377);
        for _ in 0..200 { turn_spring(Pf::f(2.0), Pf::f(0.019), Pf::f(0.1), max, &mut a, &mut v, 0); }
        assert_eq!(a, Pf::f(2.0));
    }
}
