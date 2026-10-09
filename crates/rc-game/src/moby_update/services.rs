//! The engine services the ported class updates call (`docs/plan/moby_update_catalogue.md` §7), level01
//! addresses. Everything a class update reads besides its own moby lives in [`World`] (the game's globals
//! as the moby loop sees them); everything that persists between ticks and is owned by the moby system
//! lives in [`Services`].
//!
//! * **Vector / quaternion library** (VU0 macro code at 0x2211xx..0x222xxx and 0x26ee30, 0x2721f0,
//!   0x272090): lane-by-lane on the PS2 float model ([`Pf`]), ACC order as in the instructions. The helpers
//!   that already exist in [`crate::hero::physics`] (vadd, set_len3, mul_rows3/4, euler_rows, …) are reused.
//! * **Hit messages** (0x178580 records, `MobyGetHitMessage` 0x26f320): [`HitLog`], [`World::get_hit`],
//!   [`World::deliver_hit`].
//! * **Sounds** (`fun_0022da68` = level01 0x2a1618 `PlayClassSound(idx, flags, moby)`): recorded into
//!   [`Services::sounds`] for the audio port to drain.
//! * **Glints** (0x16eec0, 16 × 0x20: `FUN_002208a0` create, `FUN_00220928` update): [`Glints`].
//! * **Save / game-state writes** of the bolt pickup: [`SaveBits`], [`GameCounters`].
//! * **Timers**: `ticks(n)` 0x220e30, `multiply_global_scale` 0x220e20, `FastDecTimer` 0x220ea8 / 0x220ed8.
//! * **Hero-block writes** (the classes' stores into 0x13f350..0x141660): [`HeroFields`], [`World::hero_fields`] /
//!   [`World::hero_fields_mut`], applied by the tick after the moby loop ([`crate::tick::MobySystem::take_hero_writes`]).
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern, clippy::needless_range_loop, clippy::too_many_arguments)] // FPU compare semantics, op order and lane loops are spelled out on purpose.

use std::collections::HashMap;

use crate::hero::physics::{self as ph, V4};
use crate::hero::Hero;
use crate::moby_runtime::{ClassInfo, Moby, MobyId, MobyTable};
use crate::moby_update::scheduler::Groups;
use crate::collision_query::{coll_line_m, coll_sphere_m, coll_sphere_mobys, CollOutput, MobyGrid, MobyScene, PoseCache, QueryFlags, TableMobys};
use crate::particles::{BSphereView, Particles};
use crate::ps2v::Pf;
use crate::rng::Rng;
use rc_formats::collision::Collision;
use rc_formats::moby_anim::{MobyAnimClass, MobyFrame};
use rc_formats::moby_collision::MobyCollision;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------------------------------
// Constants and small conversions

/// `0x15ed6c` dt (NTSC 1/60) and `0x15ed70` dt².
pub use crate::hero::physics::{DT, DT2};
/// π/180 as the updates load it (`0x3c8efa35`).
pub const DEG: Pf = Pf::b(0x3c8e_fa35);
/// π/4 (`0x3f490fdb`).
pub const QUARTER_PI: Pf = Pf::b(0x3f49_0fdb);

/// A moby field as a PS2 float vector, keeping the bits (the moby fields hold what the PS2 stored).
#[inline]
pub fn pv(a: [f32; 4]) -> V4 { a.map(|x| Pf(x.to_bits())) }
#[inline]
pub fn pf(x: f32) -> Pf { Pf(x.to_bits()) }
#[inline]
pub fn fl(x: Pf) -> f32 { f32::from_bits(x.0) }
/// Back to the moby's storage (bits kept).
#[inline]
pub fn fv(a: V4) -> [f32; 4] { a.map(fl) }

/// Little-endian pvar accessors (the per-class block at moby+0x78, raw bytes).
pub mod pvar {
    use crate::ps2v::Pf;
    #[inline]
    pub fn u8(p: &[u8], o: usize) -> u8 { p[o] }
    #[inline]
    pub fn set_u8(p: &mut [u8], o: usize, x: u8) { p[o] = x; }
    #[inline]
    pub fn i16(p: &[u8], o: usize) -> i16 { i16::from_le_bytes([p[o], p[o + 1]]) }
    #[inline]
    pub fn set_i16(p: &mut [u8], o: usize, x: i16) { p[o..o + 2].copy_from_slice(&x.to_le_bytes()); }
    #[inline]
    pub fn u16(p: &[u8], o: usize) -> u16 { u16::from_le_bytes([p[o], p[o + 1]]) }
    #[inline]
    pub fn i32(p: &[u8], o: usize) -> i32 { i32::from_le_bytes(p[o..o + 4].try_into().unwrap()) }
    #[inline]
    pub fn set_i32(p: &mut [u8], o: usize, x: i32) { p[o..o + 4].copy_from_slice(&x.to_le_bytes()); }
    #[inline]
    pub fn u32(p: &[u8], o: usize) -> u32 { i32(p, o) as u32 }
    #[inline]
    pub fn set_u32(p: &mut [u8], o: usize, x: u32) { set_i32(p, o, x as i32); }
    #[inline]
    pub fn f(p: &[u8], o: usize) -> Pf { Pf(u32(p, o)) }
    #[inline]
    pub fn set_f(p: &mut [u8], o: usize, x: Pf) { set_u32(p, o, x.0); }
    #[inline]
    pub fn v4(p: &[u8], o: usize) -> [Pf; 4] { [f(p, o), f(p, o + 4), f(p, o + 8), f(p, o + 12)] }
    #[inline]
    pub fn set_v4(p: &mut [u8], o: usize, x: [Pf; 4]) { for (k, c) in x.iter().enumerate() { set_f(p, o + 4 * k, *c); } }
    /// Native `f32` views (the classes written on standard floats).
    #[inline]
    pub fn ff(p: &[u8], o: usize) -> f32 { f32::from_bits(u32(p, o)) }
    #[inline]
    pub fn set_ff(p: &mut [u8], o: usize, x: f32) { set_u32(p, o, x.to_bits()); }
    #[inline]
    pub fn v4f(p: &[u8], o: usize) -> [f32; 4] { [ff(p, o), ff(p, o + 4), ff(p, o + 8), ff(p, o + 12)] }
    #[inline]
    pub fn set_v4f(p: &mut [u8], o: usize, x: [f32; 4]) { for (k, c) in x.iter().enumerate() { set_ff(p, o + 4 * k, *c); } }
}

// ---------------------------------------------------------------------------------------------------
// Timers (level01 0x220e20..0x220ed8)

/// The time base: `0x15ed68` timer scale (1.0 NTSC, 0.8333 PAL).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    pub timer_scale: Pf,
}

impl Timing {
    pub const NTSC: Timing = Timing { timer_scale: Pf::ONE };

    /// `ticks(n)` 0x220e30: `cvt.w.s((0.25 + 0.25) + (f32)n · scale)` (`adda.s`, `madd.s`).
    pub fn ticks(&self, n: i32) -> i32 {
        let acc = Pf::b(0x3e80_0000) + Pf::b(0x3e80_0000);
        (acc + Pf::from_i32(n) * self.timer_scale).to_i32()
    }

    /// `multiply_global_scale` 0x220e20: `scale · x` (`mul.s f0, f0(scale), f12`).
    pub fn scale(&self, x: Pf) -> Pf { self.timer_scale * x }
}

/// `FastDecTimer__FRs` 0x220ea8 on an s16: 1 if it is 0 (untouched), else `t = max(t, 1) − 1` and 2 when
/// that is ≤ 0, 0 otherwise.
pub fn fast_dec_timer_s16(t: &mut i16) -> i32 {
    if *t == 0 { return 1; }
    let n = (*t as i32).max(1) - 1;
    *t = n as i16;
    if n < 1 { 2 } else { 0 }
}

/// `FUN_00220ed8` on a u8 (the byte variant): 1 if it is 0, else `t = max(t, 1) − 1`, 2 when that is ≤ 0.
pub fn fast_dec_timer_u8(t: &mut u8) -> i32 {
    if *t == 0 { return 1; }
    let n = (*t as i32).max(1) - 1;
    *t = n as u8;
    if n < 1 { 2 } else { 0 }
}

// ---------------------------------------------------------------------------------------------------
// VU0 vector / quaternion library (level01 addresses; boot copies in brackets)

/// `FastVecCross(out, a, b)` 0x2212d0: `vopmula.xyz ACC, b, a; vopmsub.xyz out, a, b` = **b × a**.
pub fn cross_ba(a: V4, b: V4) -> V4 {
    let c = crate::ps2v::cross([b[0].0, b[1].0, b[2].0], [a[0].0, a[1].0, a[2].0]);
    [Pf(c[0]), Pf(c[1]), Pf(c[2]), a[3]]
}

/// `fun_001f9a68` [boot 0x1f9a68]: `out.xyz = a.xyz · s` (w of `out` is the source's w; the store writes
/// the whole quadword from `vf1`, whose w is `a.w`).
pub fn scale3(a: V4, s: Pf) -> V4 { [a[0] * s, a[1] * s, a[2] * s, a[3]] }

/// `fun_001f9a40` 0x2211e8: `a + (b − a) · t` (`vsub`, `vmulx`, `vadd`).
pub fn lerp3(a: V4, b: V4, t: Pf) -> V4 {
    let d = ph::vsub(b, a);
    let d = [d[0] * t, d[1] * t, d[2] * t, d[3]];
    ph::vadd(a, d)
}

/// `FUN_002745f0(max, v)`: `if max < FastVecLength(v) { v = FastVecNormalize(max, v) }`.
pub fn clamp_len(v: &mut V4, max: Pf) {
    let l = ph::len3(*v);
    if max < l { *v = ph::set_len3(*v, max); }
}

/// `FUN_00221570(out, v, n)`: reflect `v` off the plane with normal direction `n` when `v` moves into it.
/// `a = 0 − v`; `n̂ = n · rsqrt(n·n)` (`vrsqrt Q, vf0w`); `d = a·n̂` ((x+y)+1·z); `m = n̂ · d`; when the sign
/// bit of `d` is set `out = v`, else `out = a + ((m − a) + (m − a))`.
pub fn reflect(vel: V4, n: V4) -> V4 {
    let a = [Pf::ZERO - vel[0], Pf::ZERO - vel[1], Pf::ZERO - vel[2], vel[3]];
    let nn = (n[0] * n[0] + n[1] * n[1]) + Pf::ONE * (n[2] * n[2]);
    let q = Pf::ONE / nn.sqrt();
    let nh = [n[0] * q, n[1] * q, n[2] * q];
    let d = (a[0] * nh[0] + a[1] * nh[1]) + Pf::ONE * (a[2] * nh[2]);
    if d.sign() { return vel; }
    let m = [nh[0] * d, nh[1] * d, nh[2] * d];
    let e = [m[0] - a[0], m[1] - a[1], m[2] - a[2]];
    let e = [e[0] + e[0], e[1] + e[1], e[2] + e[2]];
    [a[0] + e[0], a[1] + e[1], a[2] + e[2], a[3]]
}

/// `fun_001fa480` 0x221ef8: quaternion (x, y, z, w) → rotation rows (3 rows, w lanes 0).
pub fn quat_rows(q: V4) -> [V4; 3] {
    let z = Pf::ZERO;
    let one = Pf::ONE;
    let q2 = [q[0] + q[0], q[1] + q[1], q[2] + q[2], q[3] + q[3]]; // vadd vf9 = q + q
    let w = [q2[0] * q[3], q2[1] * q[3], q2[2] * q[3]]; // vf10 = 2q · w
    let x = [q2[0] * q[0], q2[1] * q[0], q2[2] * q[0]]; // vf11 = 2q · x
    let y = [q2[1] * q[1], q2[2] * q[1]]; // vf12.yz = 2q.yz · y
    let zz = q2[2] * q[2]; // vf13.z
    let mut r0 = [one, z, z, z];
    let mut r1 = [z, one, z, z];
    let mut r2 = [z, z, one, z];
    r1[0] = z + w[2]; // vaddz.x vf15, vf0, vf10
    r2[0] = z - w[1]; // vsuby.x vf16, vf0, vf10
    r2[1] = z + w[0]; // vaddx.y vf16, vf0, vf10
    r0[0] = r0[0] - y[0]; // vsuby.x vf14, vf14, vf12
    r1[1] = r1[1] - x[0]; // vsubx.y vf15, vf15, vf11
    r2[2] = r2[2] - x[0]; // vsubx.z vf16, vf16, vf11
    r0[1] = x[1] - w[2]; // vsubz.y vf14, vf11, vf10
    r0[2] = x[2] + w[1]; // vaddy.z vf14, vf11, vf10
    r1[2] = y[1] - w[0]; // vsubx.z vf15, vf12, vf10
    r1[0] = r1[0] + x[1]; // vaddy.x vf15, vf15, vf11
    r2[0] = r2[0] + x[2]; // vaddz.x vf16, vf16, vf11
    r2[1] = r2[1] + y[1]; // vaddz.y vf16, vf16, vf12
    r0[0] = r0[0] - zz; // vsubz.x vf14, vf14, vf13
    r1[1] = r1[1] - zz; // vsubz.y vf15, vf15, vf13
    r2[2] = r2[2] - y[0]; // vsuby.z vf16, vf16, vf12
    [r0, r1, r2]
}

/// `fun_001fa328` 0x221c98 `(out, a, b)`: `out_i = ((a0·b_i.x + a1·b_i.y) + a2·b_i.z)` over xyzw
/// (`vmulax`, `vmadday`, `vmaddz`), for i = 0..2.
pub fn mat3_mul(a: &[V4; 3], b: &[V4; 3]) -> [V4; 3] {
    b.map(|bi| std::array::from_fn(|k| (a[0][k] * bi[0] + a[1][k] * bi[1]) + a[2][k] * bi[2]))
}

/// 0x221ce8 `(out, a, b)`: 4-row version, `out_i = (((a0·b_i.x + a1·b_i.y) + a2·b_i.z) + a3·b_i.w)`.
pub fn mat4_mul(a: &[V4; 4], b: &[V4; 4]) -> [V4; 4] {
    b.map(|bi| std::array::from_fn(|k| ((a[0][k] * bi[0] + a[1][k] * bi[1]) + a[2][k] * bi[2]) + a[3][k] * bi[3]))
}

/// `fun_001fa2d8` [boot 0x1fa2d8]: the transpose of the 3×3 (`vaddx.x vf4, vf0, vf1` …, each lane `0 + v`),
/// w lanes `1 − 1 = 0`, row 3 = (0, 0, 0, 1).
#[allow(clippy::eq_op)] // `vsubw.w vf4, vf0, vf0` is 1 − 1 on the VU.
pub fn transpose(m: &[V4; 3]) -> [V4; 4] {
    let z = Pf::ZERO;
    let t = |k: usize| [z + m[0][k], z + m[1][k], z + m[2][k], Pf::ONE - Pf::ONE];
    [t(0), t(1), t(2), [z, z, z, Pf::ONE]]
}

/// `fun_001fa3c0` 0x221d78: the quaternion product `a·b` (Hamilton): xyz = `(b·a.w + a·b.w) + a × b`,
/// w = `a.w·b.w − ((a.x·b.x + a.y·b.y) + 1·a.z·b.z)`.
pub fn quat_mul(a: V4, b: V4) -> V4 {
    let w3 = b[3] * a[3]; // vmul.w vf3, vf2, vf1
    let p = [b[0] * a[0], b[1] * a[1], b[2] * a[2]]; // vmul.xyz vf4, vf2, vf1
    let s5 = [b[0] * a[3], b[1] * a[3], b[2] * a[3]]; // vmulw.xyz vf5, vf2, vf1
    let s6 = [a[0] * b[3], a[1] * b[3], a[2] * b[3]]; // vmulw.xyz vf6, vf1, vf2
    let c = crate::ps2v::cross([a[0].0, a[1].0, a[2].0], [b[0].0, b[1].0, b[2].0]); // vopmula ACC, vf1, vf2; vopmsub vf7, vf2, vf1
    let dot = (p[0] + p[1]) + Pf::ONE * p[2];
    let s8 = [s5[0] + s6[0], s5[1] + s6[1], s5[2] + s6[2]];
    [s8[0] + Pf(c[0]), s8[1] + Pf(c[1]), s8[2] + Pf(c[2]), w3 - dot]
}

/// `fun_001fa400` 0x221db8 `(t, out, a, b)`: nlerp. `a' = a·(1 − t)`, `b' = b·t` (xyzw); the sign test is on
/// `d = (((a'·b').y + (a'·b').x) + 1·z) + 1·w` (lane y, read by `qmfc2` + `bgez`); `q = d < 0 ? a' − b' : a' + b'`;
/// `out = q · rsqrt((((q.x² + q.y²) + 1·q.z²) + 1·q.w²))`.
pub fn quat_nlerp(t: Pf, a: V4, b: V4) -> V4 {
    let u = Pf::ONE - t; // vsubx.w vf3, vf0, vf3
    let a1 = a.map(|x| x * u);
    let b1 = b.map(|x| x * t);
    let mut q: V4 = std::array::from_fn(|k| a1[k] + b1[k]);
    let p: V4 = std::array::from_fn(|k| a1[k] * b1[k]);
    let d = ((p[1] + p[0]) + Pf::ONE * p[2]) + Pf::ONE * p[3];
    if d.sign() { q = std::array::from_fn(|k| a1[k] - b1[k]); }
    let s = q.map(|x| x * x);
    let l = ((s[0] + s[1]) + Pf::ONE * s[2]) + Pf::ONE * s[3];
    let r = Pf::ONE / l.sqrt(); // vrsqrt Q, vf0w, vf5x
    q.map(|x| x * r)
}

/// `FUN_00221e38(a, out, axis)`: the half-angle quaternion about axis 0/1/2 as the game builds it (note the
/// negated sine): `c2 = 2·cos a`, `s = −sin a`, `k = sqrt(|c2| + 2)`; `c2 ≥ 0`: `w = 0.5·k`, `v = s / k`;
/// `c2 < 0` (sign bit): `v = 0.5·k`, `w = s / k`; `v` goes to lane `axis`.
pub fn axis_quat(a: Pf, axis: usize) -> V4 {
    let s = -ph::fast_sin(a); // neg.s f0
    let c = ph::fast_cos(a);
    let c2 = c + c; // add.s f0, f0, f0
    let k = (c2.abs() + Pf::b(0x4000_0000)).sqrt(); // vabs, vadd.x vf1, vf3(2.0), vsqrt
    let half = Pf::b(0x3f00_0000);
    let z = Pf::ZERO;
    let (vv, w) = if c2.sign() {
        let x = half * (z + k); // vaddq.x vf6, vf0, Q; vmulq.x vf4, vf5(0.5), Q
        (x, z + s / (z + k)) // vdiv Q, vf2x, vf6x; vaddq.w vf4, vf4(0), Q
    } else {
        let w = half * (z + k);
        (z + s / (z + k), w)
    };
    let mut q = [z, z, z, w];
    match axis {
        0 => q[0] = vv,
        1 => q[1] = z + vv, // vaddx.y vf4, vf4, vf4; vadd.x vf4, vf0, vf0
        _ => q[2] = z + vv,
    }
    if axis != 0 { q[0] = z + z; }
    q
}

/// `FUN_0026ee30(out, e)`: Euler → quaternion `(qx(e.x) · qy(e.y)) · qz(e.z)` ([`axis_quat`], [`quat_mul`]).
pub fn euler_quat(e: V4) -> V4 {
    let qx = axis_quat(e[0], 0);
    let qy = axis_quat(e[1], 1);
    let qz = axis_quat(e[2], 2);
    quat_mul(quat_mul(qx, qy), qz)
}

/// `FUN_00272090(angle, out, axis)` (build_quaternion_from_axis_angle): `out.xyz = axis · fast_sin(angle·0.5)`,
/// `out.w = fast_cos(angle·0.5)`.
pub fn axis_angle_quat(angle: Pf, axis: V4) -> V4 {
    let h = angle * Pf::b(0x3f00_0000);
    let mut q = scale3(axis, ph::fast_sin(h));
    q[3] = ph::fast_cos(h); // the second `angle · 0.5` gives the same bits
    q
}

/// The FPU sine polynomial of `fun_001fa070` 0x2219a0: fold `x` into [−π/2, π/2] (`x ≥ π/2 → π − x`; the
/// second test reads the unfolded `x`: `x < −π/2 → −π − x`), then `(((x + 0) + x³·c1) + x⁵·c2) + x⁷·c3) + x⁹·c4`
/// (`adda.s`, `madda.s` ×3, `madd.s`).
pub fn fpu_sin(x0: Pf) -> Pf {
    let (hp, nhp) = (Pf::b(0x3fc9_0fdb), -Pf::b(0x3fc9_0fdb));
    let (p, np) = (Pf::b(0x4049_0fdb), -Pf::b(0x4049_0fdb));
    let mut x = x0;
    let below = x0 < nhp;
    if !(x0 < hp) { x = p - x0; }
    if below { x = np - x; }
    let (c1, c2, c3, c4) = (Pf::b(0xbe2a_aaa4), Pf::b(0x3c08_873e), Pf::b(0xb94f_b21f), Pf::b(0x362e_9c14));
    let x2 = x * x;
    let x3 = x * x2;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    let x9 = x7 * x2;
    let acc = x + Pf::ZERO;
    let acc = acc + x3 * c1;
    let acc = acc + x5 * c2;
    let acc = acc + x7 * c3;
    acc + x9 * c4
}

/// `fun_001fa070` 0x2219a0 `(out, e)`: rows of the inverse-order Euler rotation built on the FPU polynomial:
/// from the identity, z (if `e.z` bits ≠ 0: rows 0/1 = (c, s, 0, 0) / (−s, c, 0, 0), written into the
/// identity), then y (`rows_i = ((Y0·r.x + Y1·r.y) + Y2·r.z)`, Y = (c,0,−s,0), (0,1,0,0), (s,0,c,0)), then x
/// (X = (1,0,0,0), (0,c,s,0), (0,−s,c,0)); row 3 = (0,0,0,1). `s = fpu_sin(a)`, `c = fpu_sin(a + π/2)`.
pub fn euler_rows_fpu(e: V4) -> [V4; 4] {
    let (z, one) = (Pf::ZERO, Pf::ONE);
    let hp = Pf::b(0x3fc9_0fdb);
    let mut r: [V4; 3] = [[z + one, z, z, z], [z, z + one, z, z], [z, z, one, z]];
    let sc = |a: Pf| (fpu_sin(a), fpu_sin(a + hp));
    let apply = |m: [V4; 3], r: [V4; 3]| r.map(|ri| std::array::from_fn(|k| (m[0][k] * ri[0] + m[1][k] * ri[1]) + m[2][k] * ri[2]));
    if e[2].0 != 0 {
        let (s, c) = sc(e[2]);
        r[0][0] = z + c;
        r[0][1] = z + s;
        r[1][1] = z + c;
        r[1][0] = z - s;
    }
    if e[1].0 != 0 {
        let (s, c) = sc(e[1]);
        let m = [[z + c, z, z - s, z], [z, z + one, z, z], [z + s, z, z + c, z]];
        r = apply(m, r);
    }
    if e[0].0 != 0 {
        let (s, c) = sc(e[0]);
        let m = [[z + one, z, z, z], [z, z + c, z + s, z], [z, z - s, z + c, z]];
        r = apply(m, r);
    }
    [r[0], r[1], r[2], [z, z, z, one]]
}

/// Rows from Euler angles through VU0 program 0x1a3 (`fun_001fa030` 0x221960 stores 3 rows, `fun_001fa050`
/// 0x221980 also row 3 = (0,0,0,1)).
pub fn euler_rows(e: V4) -> [V4; 4] { ph::euler_rows(e) }

/// `FUN_002721f0(m, out)`: rotation rows (4) → Euler angles (x, y, z). `z = atan(r0.x, r0.y)`;
/// `m' = Rz(−z)·m` (0x221980 then 0x221ce8 with m's row 3 replaced by (0,0,0,1)); `y = atan(m'.r0.x, −m'.r0.z)`;
/// `m'' = Ry(−y)·m'` ([`euler_rows_fpu`] with (0, −y, 0)); `x = atan(m''.r1.y, m''.r1.z)` (stored as is: the
/// `neg.s f1, f0` at 0x2722d8 only feeds a dead stack word).
pub fn rows_euler(m: &[V4; 4]) -> V4 {
    use crate::pad::fast_arctan as atan;
    let z = Pf::ZERO;
    let mut mm = [m[0], m[1], m[2], [z, z, z, Pf::ONE]];
    let a_z = atan(mm[0][0], mm[0][1]);
    let rz = euler_rows([z, z, -a_z, z]);
    mm = mat4_mul(&rz, &mm);
    let a_y = atan(mm[0][0], -mm[0][2]);
    let ry = euler_rows_fpu([z, -a_y, z, z]);
    mm = mat4_mul(&ry, &mm);
    let a_x = atan(mm[1][1], mm[1][2]);
    [a_x, a_y, a_z, z]
}

// ---------------------------------------------------------------------------------------------------
// Class data

/// What the scheduler needs to know about the level's classes (the class headers `0x197780[slot]`).
pub trait ClassData {
    /// `InitMobyInstance`'s view of the class, for `CreateMoby(o_class)` (None: class not loaded).
    fn info(&self, o_class: i16) -> Option<ClassInfo>;
    /// Animation data (sequences) for `MobyAnimAdvance` and the sequence helpers.
    fn anim(&self, o_class: i16) -> Option<&MobyAnimClass>;
}

/// A [`ClassData`] built from per-class records.
#[derive(Default)]
pub struct ClassTable {
    pub classes: HashMap<i16, (ClassInfo, Option<MobyAnimClass>)>,
}

impl ClassData for ClassTable {
    fn info(&self, o_class: i16) -> Option<ClassInfo> { self.classes.get(&o_class).map(|c| c.0) }
    fn anim(&self, o_class: i16) -> Option<&MobyAnimClass> { self.classes.get(&o_class).and_then(|c| c.1.as_ref()) }
}

// ---------------------------------------------------------------------------------------------------
// Hit messages (0x178580)

/// One hit record (0x40 bytes at `0x178580 + slot·0x40`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HitRecord {
    /// +0x00: zero (`deliver_hit`) or vf0 = (0,0,0,1) (the sphere kernel).
    pub pos: V4,
    /// +0x10: direction / push vector (template +0x00).
    pub dir: V4,
    /// +0x20: attacker moby (template +0x10; None = 0).
    pub attacker: Option<MobyId>,
    /// +0x24: damage flags (template +0x14). Receivers test masks: bolts/crates 0x1830000, 577 0x330000, 459
    /// 0x210000; 0x20000 = heavy (breaks reinforced crates), 0x1000000 exactly = "touched" (TNT crate fuse).
    pub flags: u32,
    /// +0x28 / +0x29 / +0x2a: attack type bytes and class (template +0x18..+0x1b).
    pub b28: u8,
    pub b29: u8,
    pub h2a: u16,
    /// +0x2c: damage (template +0x1c).
    pub damage: Pf,
    /// +0x30: template +0x20.
    pub w30: u32,
    /// +0x34: target moby.
    pub target: MobyId,
    /// +0x38: `CollLine_Fix`'s hit: the primitive index hit, −1 for a triangle; 0 from the sphere kernel
    /// and `deliver_hit`.
    pub prim: i32,
}

/// The attack a sender fills in (the 0x24-byte template the hit functions copy into a record).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HitTemplate {
    /// +0x00: direction / push vector.
    pub dir: V4,
    /// +0x10: attacker.
    pub attacker: Option<MobyId>,
    /// +0x14: flags.
    pub flags: u32,
    /// +0x18..+0x1b (the TNT crate leaves them as stale stack; 0 here).
    pub b18: u8,
    pub b19: u8,
    pub h1a: u16,
    /// +0x1c: damage.
    pub damage: Pf,
    /// +0x20.
    pub w20: u32,
}

/// The hit-message records and the next-slot counter `0x1742d4` (CollOutput+0x14). A target's `moby+0xa4`
/// holds its record slot (0xff = none).
#[derive(Clone, Debug)]
pub struct HitLog {
    pub records: Vec<HitRecord>,
    pub next: u8,
}

impl Default for HitLog {
    fn default() -> Self { HitLog { records: vec![HitRecord::default(); HIT_SLOTS], next: 0 } }
}

/// Number of records at 0x178580 (0x40 each; `slot = (slot + 1) & 0x3f`).
pub const HIT_SLOTS: usize = 0x40;

impl HitLog {
    fn write(&mut self, table: &mut MobyTable, target: MobyId, t: &HitTemplate, pos: V4) { self.write_prim(table, target, t, pos, 0) }

    fn write_prim(&mut self, table: &mut MobyTable, target: MobyId, t: &HitTemplate, pos: V4, prim: i32) {
        let slot = self.next;
        self.records[slot as usize] = HitRecord {
            pos,
            dir: t.dir,
            attacker: t.attacker,
            flags: t.flags,
            b28: t.b18,
            b29: t.b19,
            h2a: t.h1a,
            damage: t.damage,
            w30: t.w20,
            target,
            prim,
        };
        table.mobys[target].hit_slot = slot;
        self.next = (slot + 1) & 0x3f;
    }

    /// The record currently addressed by `target+0xa4`, if it is for `target`.
    fn current(&self, table: &MobyTable, target: MobyId) -> Option<&HitRecord> {
        let s = table.mobys[target].hit_slot;
        if s == 0xff { return None; }
        self.records.get(s as usize).filter(|r| r.target == target)
    }
}

// ---------------------------------------------------------------------------------------------------
// Sounds, glints, save bits, counters, inventory

/// A `PlayClassSound(idx, flags, moby)` (level01 0x2a1618) call: class sound `index` of `o_class`
/// (`sound_class` is the class whose sound table is used: 0x2a16c0 passes another class).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundEvent {
    pub index: i32,
    pub flags: u32,
    pub moby: MobyId,
    pub o_class: i16,
    pub sound_class: i16,
    pub pos: [f32; 3],
    pub tick: u64,
}

/// The sound system as the updates see it (`PlayClassSound` → `SoundSlotAlloc` 0x2a13a0). The audio port
/// implements it with `audio::voices::SoundSlots::play` on the same RNG: the slot allocator draws
/// `randi(pb_hi − pb_lo)` for a class sound with a pitch-bend range (on Novalis: crate 502's sound 0) when a
/// slot is free, so the draw must happen during the moby update. Without a sink the call is only recorded
/// in [`Services::sounds`] and draws nothing.
pub trait SoundSink {
    /// Returns the voice slot, or −1 when refused.
    fn play_class_sound(&mut self, ev: &SoundEvent, rng: &mut Rng) -> i32;
    /// `SoundIsAlive(moby, slot)` 0x27e820: slot `slot` still plays a sound of `moby`. Default: any slot ≠ −1.
    fn alive(&self, slot: i32, _moby: MobyId) -> bool { slot != -1 }
    /// `release_voice_slot(slot)` 0x2a1348 when the slot still plays a sound of `moby` (the classes' guard: owner
    /// and state checked first): a looping sound stops. Default: nothing.
    fn release(&mut self, _slot: i32, _moby: MobyId) {}
    /// Slot `slot`'s owner (+0x18, zeroed when the voice is freed) and class-sound index (+0xe), whatever its state:
    /// what the loop-sound refresh `FUN_002637d8` reads (`anim_sound`). None: no owner. Default: None.
    fn slot_owner(&self, _slot: i32) -> Option<(MobyId, u16)> { None }
    /// `PlayLevelSoundAtMoby(index, flags, moby)` 0x2a1770: a level-bank def (0 help box, 1 skill point) at the moby
    /// `at` (id, position), or 2-D at the listener for None (every caller on the disc). Default: −1.
    fn play_level_sound(&mut self, _index: i32, _flags: u32, _at: Option<(MobyId, [f32; 3])>, _tick: u64, _rng: &mut Rng) -> i32 { -1 }
    /// Level03's `0x27a618(index, flags, moby)`: level def `index + 0x15f574` (past the two moby defs; below the level def
    /// count 0x15f5f0) at the moby, `SoundSlotAlloc(def, flags, moby, 0, 0x400)` (Kerwan's air-traffic wreck). Default: −1.
    fn play_level_def(&mut self, _index: i32, _flags: u32, _at: Option<(MobyId, [f32; 3])>, _tick: u64, _rng: &mut Rng) -> i32 { -1 }
    /// `HeroTeleport` 0x2368e0 moved Ratchet to `pos` (its `EnvNearestSamplePoint`: reverb and music track). Default:
    /// nothing.
    fn hero_teleported(&mut self, _pos: [f32; 3]) {}
    /// `MusicRequestTrack(track, stinger)` 0x27a248 called by a class (the teleporter pads 1135's P+0x40 / P+0x44).
    /// Default: nothing.
    fn music_request(&mut self, _track: i16, _stinger: i16) {}
    /// `SoundSetPitchBend(slot, pb)` 0x2a1988: the slot's pitch bend (+0x14), sent with its next parameters (the
    /// Visibomb's loop). Default: nothing.
    fn set_pitch_bend(&mut self, _slot: i32, _pb: i32) {}
    /// `SoundSetVolume(slot, vol)` 0x2a1968: the slot's volume scale (+0x10, 0x400 = 1). Default: nothing.
    fn set_volume(&mut self, _slot: i32, _vol: i32) {}
    /// The checkpoint record `0x29ac10` saves the sound layer's reverb request. Default: nothing.
    fn checkpoint_saved(&mut self) {}
    /// The voice handoff (G-AUD-010): a class's own store of a new owner and position into a playing slot's record
    /// (`0x13e5c0 + slot·0x70` +0x18 owner, +0x20 position), by which one looping voice moves between the emitters of a
    /// class (the laser fences 838, the spouts 855 on Rilgar). Default: nothing.
    fn hand_over(&mut self, _slot: i32, _moby: MobyId, _pos: [f32; 3]) {}
}

/// One glint (0x16eec0 + i·0x20): the sparkle drawn on idle bolts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Glint {
    /// +0x00: position.
    pub pos: V4,
    /// +0x10: remaining ticks (0x18 at creation; the entry is free when ≤ 0).
    pub timer: i16,
    /// +0x12: brightness ramp (+5 per tick while timer ≥ 13, then −5; not reset at creation).
    pub alpha: i16,
    /// +0x14: owner moby index.
    pub moby: i16,
    /// +0x18: spin angle.
    pub angle: Pf,
    /// +0x1c: size.
    pub size: Pf,
}

/// The glint table.
#[derive(Clone, Debug, Default)]
pub struct Glints {
    pub entries: [Glint; 16],
}

impl Glints {
    /// `FUN_002208a0(size, pos, moby)`: the first entry with timer < 1 gets `pos`, size, timer 0x18, owner and
    /// `rand_angle()`; returns its index, −1 when all 16 are busy (no RNG draw then).
    pub fn create(&mut self, size: Pf, pos: V4, moby: i16, rng: &mut Rng) -> i16 {
        for (i, e) in self.entries.iter_mut().enumerate() {
            if e.timer < 1 {
                e.pos = pos;
                e.size = size;
                e.timer = 0x18;
                e.moby = moby;
                e.angle = Pf(rng.rand_angle_bits());
                return i as i16;
            }
        }
        -1
    }

    /// `FUN_00220928`, run right after `UpdateParts` every tick: live entries spin by `−dt·5.2359877`
    /// (wrapped), brightness ±5, timer − 1, and are zeroed (`FastMemSet(e, 0, 0x20)`) when it reaches ≤ 0.
    pub fn update(&mut self) {
        for e in self.entries.iter_mut() {
            if e.timer > 0 {
                e.angle = ph::fast_subtract_rotations(e.angle, DT * Pf::b(0x40a7_8d36));
                e.alpha = e.alpha.wrapping_add(if e.timer < 0xd { -5 } else { 5 });
                e.timer = e.timer.wrapping_sub(1);
                if e.timer < 1 { *e = Glint::default(); }
            }
        }
    }
}

/// The per-spawn-id save state the bolt pickup and `SetDeathBits` 0x26c250 write (indexed by moby+0xb2).
#[derive(Clone, Debug, Default)]
pub struct SaveBits {
    /// `0x14c190 + level·0x100 + (id >> 5)·4`, bit `id & 31`: the persistent death bits, as (level, id).
    pub death: std::collections::HashSet<(u32, i16)>,
    /// `0x1ba950 + (id >> 5)·4`, bit `id & 31`: this visit's death bits.
    pub death_level: std::collections::HashSet<i16>,
    /// `0x1baea4[id] = mission + 2` (the level's "killed" bytes).
    pub killed: HashMap<i16, u8>,
    /// `0x1bbb04[id] = mission + 2` (persistent: a collected placed bolt does not respawn).
    pub collected: HashMap<i16, u8>,
    /// `0x1bb6b0..`: the checkpoint record (class 805, `FUN_0029ac10`): the death reload's respawn point.
    pub checkpoint: Option<crate::moby_update::classes::checkpoint::Record>,
    /// `0x1bb6e0` / `0x1bb6e4`: Ratchet's moby's light word and ambient (+0x38 / +0x3c) the record saved; the respawn
    /// `0x29adc8` puts them back (+0x80, the hero lighting's colour `0x26be04`, is not modelled: the engine does not
    /// drive the hero's zone lighting yet).
    pub checkpoint_light: Option<(u32, [u8; 4])>,
    /// `0x1bb6f4` / `0x1bb6fc`: the body word 0x1413f4 and 0x14161c the record saved (the reload `0x29adc8` switches
    /// back into body 1 / 2 with them: `crate::hero::bodies`).
    pub checkpoint_body: (u8, i32),
    /// `0x1baaa0`: this visit's records of words to restore after a death reload (`crate::moby_update::visit`).
    pub visit: Vec<crate::moby_update::visit::Visit>,
    /// `0x1bb700`: the checkpoint's copy of [`SaveBits::visit`] (`0x29ac10`), written back by the reload's respawn.
    pub checkpoint_visit: Vec<crate::moby_update::visit::Visit>,
}

/// Game-state words the ported classes read or add to (owned here until the game-state port takes them).
#[derive(Clone, Debug, Default)]
pub struct GameCounters {
    /// `0x15ed98`: the bolt count.
    pub bolts: i32,
    /// `0x13df38[level]`: bolts collected per planet.
    pub level_bolts: [i32; 20],
    /// `0x15ee2c`: bolts of the current challenge (the pickup adds when the bolt's pvar+0x40 is set, or when
    /// `0x15f598` and a placed bolt). BoltBurst's farm limiter reads it.
    pub challenge_bolts: i32,
    /// `0x15f598`: "the dropped bolts count for the challenge" (set by `BoltBurst` from the mission table
    /// for non-crate droppers).
    pub challenge_on: i32,
    /// `0x15ee28`: the challenge's time limit (BoltBurst limiter; 0 = off).
    pub challenge_limit: i32,
    /// `0x15eea0` (game beaten): BoltBurst doubles N when it or [`times_completed`](Self::times_completed) is set.
    pub double_a: i32,
    /// `0x15ee20` (gp−0x7de0): times completed / challenge-mode count (`GameState::global.completes`). BoltBurst
    /// doubles N when it is set; the challenge-mode content reads it: `TeleporterPadUpdate` 0x308bd8 hides a pad
    /// with pvar+0x3c ≠ 0 and `ItemOfferUpdate` 0x2e1ac0 deletes the even gold-weapon offers while it is 0.
    pub times_completed: i32,
    /// `0x13e520` u8[40]: gold weapon owned, by item id (`GameState::global.gold_weapons`; empty = none).
    pub gold_weapons: Vec<u8>,
    /// `0x14d592 + level·0x100 + spawner(+0xb1)·4` (s16), keyed by (level, spawner byte).
    pub spawner_bolts: HashMap<(u32, u8), i16>,
    /// `0x14d590 + level·0x100 + k·4` (s16): the spawner slots `FUN_0029ab50` handed out at this load (spawn id + 1,
    /// 0 free), from the loader's spawn test ([`Services::sync_save`] writes them into the save).
    pub spawner_first: Vec<i16>,
    /// HUD bolt-counter refreshes requested (`queue_animation_update(2, 0x754e, …)`).
    pub hud_bolt_refresh: u32,
    /// `0x15eda0`: max health (4; 5 / 8 with the nanotech upgrades), synced from the game state by the engine;
    /// the nanotech orbs heal up to it.
    pub max_hp: i32,
}

/// The mission tables the death-bit and pickup writes consult: `0x15fc88[mission]` and
/// `0x14c050[level·16 + mission]`, and the per-mission deaths `0x14ee90`. The level's state is
/// [`LevelMissions`]; the trait's defaults ([`NoMissions`]: every byte 0xff, no ammo-crate gate) are for
/// worlds without one.
pub trait MissionState {
    fn mission_slot(&self, _mission: u8) -> u8 { 0xff }
    fn mission_done(&self, _level: u32, _mission: u8) -> u8 { 0xff }
    /// `0x14ee90[mission]` (s32): an ammo crate (501) whose pvar+0xf8 exceeds it is deleted at init.
    fn ammo_crate_gate(&self, _mission: u8) -> i32 { i32::MAX }
}
/// [`MissionState`] with nothing started.
pub struct NoMissions;
impl MissionState for NoMissions {}

/// The mission state of the loaded level (L = 0x15ed84) the moby code reads:
/// * `0x15fc88[16]`: the mission bytes as `LoadLevelCoreData` 0x258128 copied them from the save at load;
/// * `0x14c050 + L·16`: the live save bytes (`SetMissionDone` writes them: `crate::cinematic::set_mission_done`, applied
///   by the engine after the tick);
/// * `0x14ee90` s32[16]: hero deaths per mission since the fresh entry. `LoadLevelCoreData` clears it right
///   before `MobyLoadTimeUpdatePass`, but only on a fresh load (`param_2 == 0`; `entry` passes 1 for the death
///   reload, 0x141401 set); the hero death routine `FUN_002319b0` does `deaths[killer+0xb0]++` (killer =
///   0x1415d0, when its +0xb0 ≠ 0xff). Not in any save descriptor. The class-501 ammo crates read it
///   (`FUN_002eac18`: a crate with mission m and pvar+0xf8 = N exists only once the player died N times to
///   mission m's enemies since entering the level).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LevelMissions {
    pub level: u32,
    pub slot: [u8; 16],
    pub done: [u8; 16],
    pub deaths: [i32; 16],
}

impl LevelMissions {
    /// A fresh load of level `level` whose save mission bytes are `save` (`0x14c050 + L·16`): the copy, the
    /// live bytes, deaths cleared. Only a full load takes the copy `slot` (0x15fc88); the death reload
    /// (`LoadLevelCoreData(0, 1)`) keeps it, and `SetMissionDone` writes `done` (0x14c050) alone.
    pub fn fresh_load(level: u32, save: [u8; 16]) -> LevelMissions { LevelMissions { level, slot: save, done: save, deaths: [0; 16] } }

    /// `FUN_002319b0`'s count: `deaths[killer+0xb0]++` when the killer (0x1415d0) has a mission byte.
    pub fn hero_death(&mut self, killer_mission: Option<u8>) {
        if let Some(m) = killer_mission.filter(|&m| m != 0xff) {
            if let Some(d) = self.deaths.get_mut(m as usize) { *d = d.wrapping_add(1); }
        }
    }
}

impl MissionState for LevelMissions {
    fn mission_slot(&self, mission: u8) -> u8 { self.slot.get(mission as usize).copied().unwrap_or(0) }
    fn mission_done(&self, level: u32, mission: u8) -> u8 {
        if level == self.level { self.done.get(mission as usize).copied().unwrap_or(0) } else { 0 }
    }
    fn ammo_crate_gate(&self, mission: u8) -> i32 { self.deaths.get(mission as usize).copied().unwrap_or(0) }
}

/// The inventory the ammo crates read (`0x26bff8`, `0x2daf10`, crate state 6 on planet 0x12).
pub trait Inventory {
    /// `0x13d4c0[i]`: item `i` owned.
    fn owned(&self, _item: usize) -> bool { false }
    /// `0x13d428[i]`: ammo of item `i`.
    fn ammo(&self, _item: usize) -> i32 { 0 }
    /// `0x1c4530 + i·0x18 + 0xe` / `+0xc`: max ammo / pickup amount.
    fn max_ammo(&self, _item: usize) -> u16 { 0 }
    fn pickup_amount(&self, _item: usize) -> u16 { 0 }
    /// The item definition's slot type (+0x08 of `0x179f40 + 0x4c·i`: 0 hand, 1 feet, 2 head, 3 back, −1 none): the
    /// directors' "two hand items owned" tests. Default: −1.
    fn slot_type(&self, _item: usize) -> i32 { -1 }
    /// `0x15edd0[0..12]`: the ammo item list (0xff ends it).
    fn ammo_list(&self) -> [u8; 12] { [0xff; 12] }
    /// `0x179f40 + type·0x4c + 0x3a` (s16): the pickup class of item `ty` (level01 .data).
    fn pickup_class(&self, ty: i32) -> i16 {
        match ty {
            10 => 226,
            11 => 204,
            13 => 222,
            15 => 1006,
            16 => 214,
            17 => 225,
            19 => 213,
            20 => 223,
            23 => 1438,
            24 => 1447,
            25 => 1449,
            _ => 0,
        }
    }
    /// The pickup banner text of item `ty` (`ShowBannerf` in 0x2db028): the item record's `+0x36` for one, else
    /// `+0x34` (`0x179f40 + ty·0x4c`, level01 .data); 0 for items without a pickup.
    fn pickup_text(&self, ty: usize, one: bool) -> i32 {
        let base = match ty {
            10 => 21429,
            11 => 21431,
            13 => 21433,
            15 => 21435,
            16 => 21437,
            17 => 21439,
            19 => 21441,
            20 => 21443,
            23 => 21445,
            24 => 21447,
            25 => 21449,
            _ => return 0,
        };
        if one { base + 1 } else { base }
    }
}
/// An empty inventory.
pub struct NoInventory;
impl Inventory for NoInventory {}

/// Spawns the ported code asked for that have no port of their own, counted (particle types without a
/// spawn port, the debris-moby updates …).
#[derive(Clone, Debug, Default)]
pub struct FxStats {
    pub part_spawns: HashMap<u8, u64>,
    pub part_failed: u64,
    pub debris: u64,
    pub flashes: u64,
    pub pickups: u64,
    /// Class states / branches the ported updates reached but do not port (combat, teleports, dialogs …), by
    /// name ([`Services::unported`]).
    pub unported: std::collections::BTreeMap<&'static str, u64>,
}

/// State the moby system keeps between ticks (the game's globals it owns).
#[derive(Clone, Debug)]
pub struct Services {
    pub timing: Timing,
    /// `0x15ed84`: the level (planet) number.
    pub level: u32,
    /// `0x15f5c4`: game mode (2 = cutscene; bolts hide).
    pub game_mode: i32,
    /// 0x15edb0: the cheat bytes as the class updates read them (`crate::cheats`; the engine copies the saved game's
    /// chunk 7 in before each tick).
    pub cheats: crate::cheats::Cheats,
    /// The scene big-head record 0x17c8c0 (`manip::scene_big_head`): the actor it was attached to, None while clear
    /// (every `DialogStreamStart` clears it).
    pub scene_head: Option<crate::moby_runtime::MobyId>,
    /// `0x15f638`: the level's death height (gameplay header +0x28; set by the engine at the load): Veldin's floating
    /// platforms 587 kill Ratchet riding one below it.
    pub death_z: f32,
    /// The level's camera records as the moby loop sees them (`0x15ef50`, set by the engine at the load): each
    /// record's class (the slots `0x167b50 + i·0xa0` +0x86; Veldin's boss 1422 looks for the class-18 one).
    pub camera_classes: Vec<i32>,
    /// The class-18 records' distance / pivot height (pvar +0x34 / +0x38) as the moby loop last wrote them (the file's
    /// values at the load; the boss 1422's camera tweak springs them, `cinematic::focus_record`).
    pub camera_focus: Vec<[f32; 2]>,
    /// `0x13d4e2`: bolt grabber owned (the hero tick then sets the pickup radii to 12 / 4.5).
    pub bolt_grabber: bool,
    /// The level overlays' own mutable words a class keeps between ticks, by address (e.g. Blarg's Clank station's
    /// gp−0x4c88 0x161f78, −1 at load: `classes::units::blarg_clank_lift`). Missing: the load value the class knows.
    pub level_words: HashMap<u32, u32>,
    /// `0x15f5d0` / `0x15f5d4`: the frame-load ratios (RCNT1 based) that throttle sparks and flashes. The port
    /// cannot reproduce them; 0 = never throttled.
    pub frame_load: [Pf; 2],
    pub hits: HitLog,
    pub sounds: Vec<SoundEvent>,
    /// The level-def plays of level03's `0x27a618` ([`World::play_level_def`]): (index, moby), in order (a test log).
    pub level_defs_played: Vec<(i32, MobyId)>,
    pub glints: Glints,
    /// The blob shadows of this tick (`0x16e500`, `crate::shadows::blob`), stamped with the tick counter.
    pub blobs: (u64, Vec<(MobyId, crate::shadows::Blob)>),
    pub save: SaveBits,
    pub counters: GameCounters,
    pub fx: FxStats,
    /// `gp−0x5854` (0x1613ac): tick of the last bolt pickup sound (.lit, 0 at boot).
    pub last_bolt_sound: i32,
    /// `gp−0x5858` (0x1613a8): the bolt spin rate, 720.0 degrees per second (.lit, never written).
    pub bolt_spin: Pf,
    /// `0x16196c` / `0x161970`: ticks of the last crate break sound / break effect (rate limits).
    pub break_sound_tick: i32,
    pub break_fx_tick: i32,
    /// `0x1b0930[i]`: the level's splines (first point's w is used as a counter by the ammo-crate init), as
    /// raw words.
    pub splines: Vec<Vec<[u32; 4]>>,
    /// Snapshot frames of the mobys' blends (`0x1aabc0` slots), by moby index.
    pub snapshots: Vec<Option<MobyFrame>>,
    /// `0x1abcc0`: the moby groups (the active-list builder and the crates' group iteration read them).
    pub groups: Groups,
    /// `0x19bc60`: the moby grid the collision kernels walk (kept by `MobyBuildMatrix` / `DeleteMoby`:
    /// [`World::build_matrix`], the scheduler, [`World::delete_moby`]). Built at load with
    /// [`MobyGrid::build`]. Shared (`Arc`) so the hero's per-tick scene can hold it without a copy.
    pub grid: Arc<MobyGrid>,
    /// The level's moby class collision blobs (class header +0x10) by `o_class` ([`Services::set_moby_collision`]).
    /// Empty: no moby collision (every moby pass finds nothing).
    pub coll_classes: Arc<HashMap<i16, MobyCollision>>,
    /// `CollOutput +4/+8/+0xc`: the collision kernels' joint-pose cache.
    pub pose_cache: Arc<Mutex<PoseCache>>,
    /// Per class: the first byte list of each joint list (class header `joints`, `rc_formats::gadget::joint_list`),
    /// for `FUN_002645a8(moby, list, out)` ([`World::joint_point`]). Filled for the classes whose update needs
    /// it ([`crate::moby_update::classes::needs_joint_lists`]).
    pub joint_lists: HashMap<i16, Vec<Vec<u8>>>,
    /// Per (class, sequence): the gait record its sequence header's +0x14 points to (`rc_formats::moby_anim::gait_records`),
    /// for the leg walker (`crate::moby_update::creature::legs`). Filled with [`Services::joint_lists`] for the same classes.
    pub gaits: HashMap<(i16, u8), [u32; 20]>,
    /// Per class: each joint list's manipulator target joint (the second byte list's first entry,
    /// `rc_formats::moby_anim::list_target`; 0xff: none), for `AttachManipulator` on that class's mobys
    /// ([`crate::moby_update::manip`]). Filled with [`Services::joint_lists`] for the same classes.
    pub joint_targets: HashMap<i16, Vec<u8>>,
    /// The level's volume sections (cuboids 0x1600ec, spheres, cylinders, pills, paths, grind paths) for the
    /// trigger tests ([`crate::moby_update::triggers`], [`Services::set_volumes`]).
    pub volumes: Arc<rc_formats::volumes::Volumes>,
    /// The hero-block fields this tick's class updates wrote ([`HeroFields`], stamped with the tick counter
    /// 0x15f5cc of the moby loop that wrote them); taken and applied by the tick before the hero update.
    pub hero_writes: Option<(u64, HeroFields)>,
    /// The camera shake requests this tick's class updates made (their stores into 0x167260 / 0x167270), in order;
    /// taken by the tick and applied to the camera before the hero update ([`World::shake_camera`]).
    pub camera_shakes: Vec<crate::follow_camera::ShakeRequest>,
    /// The classes' screen-space sprites of the last ticks (`crate::targeting::ScreenSprite`; the engine draws the
    /// last tick's).
    pub screen_sprites: Vec<crate::targeting::ScreenSprite>,
    /// The race cameras' stores of the last camera update (`crate::follow_camera::race::RaceOut`), made before the next
    /// moby loop (`classes::units::oltanis_rail_bot::camera_stores`).
    pub camera_race: Vec<crate::follow_camera::race::RaceOut>,
    /// The frame's draw-callback lists 0x21afe0 / 0x21b198 ([`crate::moby_update::classes::draw_callbacks`]).
    pub draw_callbacks: crate::moby_update::classes::draw_callbacks::DrawCallbacks,
    /// The glove reticle draw callbacks (`0x2c23c0` and its copies `0x2bf420` / `0x2d8e28`, list 1) registered by the
    /// bombs', mines' and decoys' landing previews (`crate::targeting`).
    pub reticles: crate::targeting::Reticles,
    /// The Drone Device's drones' globals (0x141344..; [`crate::moby_update::classes::drone::Globals`]).
    pub drones: crate::moby_update::classes::drone::Globals,
    /// `0x1abe80`: this tick's target list (the scheduler builds it with the run list; the mines' proximity search
    /// `classes::mine` walks it).
    pub targets: Vec<MobyId>,
    /// The fire / smoke fields' globals and elements (classes 760 / 809, [`crate::moby_update::classes::fire_field`]).
    pub fire_fields: crate::moby_update::classes::fire_field::FireFieldState,
    /// The creature layer's globals ([`crate::moby_update::creature::Globals`]: rate limiters, class spheres).
    pub creatures: crate::moby_update::creature::Globals,
    /// The bolt cranks' globals (gp−0x5360: the spring pulling Ratchet onto the bolt's ring; class 280,
    /// [`crate::moby_update::classes::bolt_crank`]).
    pub cranks: crate::moby_update::classes::bolt_crank::Globals,
    /// The point-light bank 0x180740 (the explosion lights own their slots; the renderer reads it).
    pub point_lights: crate::point_lights::PointLights,
    /// The "use" system: the context prompt lease, the NPC talk tables and the hand-offs (`interact`).
    pub interact: crate::moby_update::interact::Interact,
    /// The in-level cinematic calls and engine requests of the moby loop ([`crate::cinematic`]).
    pub cinematic: crate::cinematic::Cinematic,
    /// The level's water: the ripple module, the flat water plane, the underwater look and the managers' data
    /// (`crate::water::world`; empty until the level's water data is loaded).
    pub water: crate::water::world::WaterWorld,
    /// The pickups' globals (ammo pickup sound, the nanotech master and its orbit table;
    /// [`crate::moby_update::classes::pickup`]).
    pub pickups: crate::moby_update::classes::pickup::Globals,
    /// The last pickup banner request (`ShowBannerf`), for the HUD.
    pub pickups_banner: crate::moby_update::classes::pickup::Banner,
    /// The buried bolt caches' globals (0x141390..98, the counter gp−0x5110): `classes::buried_bolts`.
    pub buried: crate::moby_update::classes::buried_bolts::Globals,
    /// The Sonic Summoner's mouse (0x1deb88): `classes::mouse`.
    pub mouse: crate::moby_update::classes::mouse::Globals,
    /// The level words the census unit ports keep (`classes::units::Globals`).
    pub units: crate::moby_update::classes::units::Globals,
    /// The level's pvar shared data (gameplay section 0x4c, `rc_formats::gameplay::parse_pvar_shared_data`): a pvar
    /// field the loader pointed into it holds the offset here (the lamps' per-group registration tick, …).
    pub pvar_shared: Vec<u8>,
    /// 0x140940..0x14095f: the ridden vehicle record ([`crate::vehicle`]), written by the flown ships' mounts (Gemlik's
    /// 69), read by the pause triggers, Gemlik's water managers' pause (`crate::water::managers`) and the ride classes.
    pub vehicle: crate::vehicle::Record,
    /// 0x15f608: the occlusion fallback a tick's code set for its frame (`UpdateOcclusion`; cleared after every frame's
    /// render): (the tick counter, the value: 1 everything visible (the Visibomb's missile), 2 the level's octant
    /// override (`UpdateModeFreeze`, Gemlik's ship 69 while it explodes)).
    pub occlusion_fallback: Option<(u64, u8)>,
    /// The view's horizontal half-angle tangent (level01 0x16cf70, level13 0x16cdf0): `InitViewContext`'s 0.63 at the
    /// level load; a class that writes it calls `UpdateViewContext` (Gemlik's ship 69's speed FOV); the engine's
    /// projection follows it.
    pub view_tan_x: f32,
    /// The help / hint message system (`Help_Request` 0x225818, `Help_Update` 0x225bd0, the records and the log:
    /// [`crate::help`]); the classes request through it, the engine runs its update after the tick.
    pub help: crate::help::Help,
    /// The in-game map's live state (`crate::map`: the fog mask the hero reveals; moved into the page menu while it is open).
    pub map: crate::map::MapState,
    /// The Visibomb's globals (the missile 0x141330, the HUD / occlusion flags, the missile view's look, the range
    /// static): [`crate::moby_update::classes::visibomb::Globals`].
    pub visibomb: crate::moby_update::classes::visibomb::Globals,
    /// The HUD slot calls of the classes (`queue_animation_update` and the handle calls: [`crate::hud::calls`]); the HUD
    /// replays the new ones before its next update loop.
    pub hud: crate::hud::Calls,
    /// The ship block 0x13e030.. (the ship moby, its index, the take-off flag 0x15f630, mode 6's substate and tick, the
    /// trail): [`crate::travel::ShipGlobals`].
    pub travel: crate::travel::ShipGlobals,
    /// The Hoverboard's stores queued by the hero code and the race records (`classes::units::hoverboard`).
    pub board: crate::moby_update::classes::units::hoverboard::Globals,
}

impl Default for Services {
    fn default() -> Self { Services::new() }
}

impl Services {
    pub fn new() -> Services {
        Services {
            timing: Timing::NTSC,
            level: 1,
            game_mode: 0,
            cheats: Default::default(),
            scene_head: None,
            death_z: 0.0,
            camera_classes: Vec::new(),
            camera_focus: Vec::new(),
            bolt_grabber: false,
            level_words: HashMap::new(),
            frame_load: [Pf::ZERO; 2],
            hits: HitLog::default(),
            sounds: Vec::new(),
            level_defs_played: Vec::new(),
            glints: Glints::default(),
            blobs: (0, Vec::new()),
            save: SaveBits::default(),
            counters: GameCounters::default(),
            fx: FxStats::default(),
            last_bolt_sound: 0,
            bolt_spin: Pf::b(0x4434_0000),
            break_sound_tick: 0,
            break_fx_tick: 0,
            splines: Vec::new(),
            snapshots: Vec::new(),
            groups: Groups::default(),
            grid: Arc::new(MobyGrid::new()),
            coll_classes: Arc::new(HashMap::new()),
            pose_cache: Arc::new(Mutex::new(PoseCache::default())),
            joint_lists: HashMap::new(),
            gaits: HashMap::new(),
            joint_targets: HashMap::new(),
            volumes: Arc::new(rc_formats::volumes::Volumes::default()),
            hero_writes: None,
            camera_shakes: Vec::new(),
            screen_sprites: Vec::new(),
            camera_race: Vec::new(),
            draw_callbacks: Default::default(),
            reticles: Default::default(),
            targets: Vec::new(),
            drones: Default::default(),
            fire_fields: Default::default(),
            creatures: Default::default(),
            cranks: Default::default(),
            point_lights: Default::default(),
            cinematic: Default::default(),
            water: Default::default(),
            interact: Default::default(),
            pickups: Default::default(),
            pickups_banner: Default::default(),
            buried: Default::default(),
            mouse: Default::default(),
            units: Default::default(),
            help: Default::default(), map: Default::default(),
            visibomb: Default::default(),
            hud: Default::default(),
            travel: Default::default(),
            board: Default::default(),
            pvar_shared: Vec::new(),
            vehicle: Default::default(),
            occlusion_fallback: None,
            view_tan_x: 0.63,
        }
    }

    /// The s32 at `ofs` of the pvar shared data (0 past its end, as an empty blob reads on levels without one).
    pub fn shared_i32(&self, ofs: i32) -> i32 {
        usize::try_from(ofs).ok().and_then(|o| self.pvar_shared.get(o..o + 4)).map_or(0, |b| i32::from_le_bytes(b.try_into().unwrap()))
    }

    /// Writes the s32 at `ofs` of the pvar shared data (ignored past its end).
    pub fn set_shared_i32(&mut self, ofs: i32, v: i32) {
        if let Some(b) = usize::try_from(ofs).ok().and_then(|o| self.pvar_shared.get_mut(o..o + 4)) { b.copy_from_slice(&v.to_le_bytes()); }
    }

    /// The hero-block writes of the moby loop, for the tick to apply ([`HeroFields::apply`]); None: no class
    /// wrote the hero this tick.
    pub fn take_hero_writes(&mut self) -> Option<HeroFields> { self.hero_writes.take().map(|(_, f)| f) }

    /// Counts a reached-but-unported state or branch of a ported class (the stats line / trace report).
    pub fn unported(&mut self, what: &'static str) { *self.fx.unported.entry(what).or_default() += 1; }

    /// The class collision blobs of the level (`rc_formats::moby_collision::parse_level`).
    pub fn set_moby_collision(&mut self, blobs: Vec<(i32, MobyCollision)>) {
        self.coll_classes = Arc::new(blobs.into_iter().map(|(oc, c)| (oc as i16, c)).collect());
    }

    /// The level loader's grid registrations over the loaded table ([`MobyGrid::build`]).
    pub fn build_grid(&mut self, table: &mut MobyTable) { self.grid = Arc::new(MobyGrid::build(table)); }

    /// `MobyBuildMatrix` 0x265bd8 on moby `id` of `table`: matrix and bounding sphere
    /// ([`crate::moby_update::scheduler::rebuild_matrix`]), then the grid re-registration.
    pub fn build_matrix_in(&mut self, table: &mut MobyTable, classes: &dyn ClassData, id: MobyId) {
        let m = &mut table.mobys[id];
        crate::moby_update::scheduler::rebuild_matrix(m, classes.anim(m.o_class));
        Arc::make_mut(&mut self.grid).register(m);
    }

    /// The moby pass's view of `table` (with `classes` for the joint poses).
    pub fn scene_parts<'s>(&'s self, table: &'s MobyTable, classes: &'s dyn ClassData) -> TableMobys<'s> {
        TableMobys { table, classes, snapshots: &self.snapshots }
    }

    /// A [`MobyScene`] over `mobys` with this level's grid, blobs and pose cache.
    pub fn scene<'s>(&'s self, mobys: &'s TableMobys<'s>) -> MobyScene<'s> {
        MobyScene { mobys, grid: &self.grid, classes: &self.coll_classes, cache: &self.pose_cache }
    }

    pub fn ticks(&self, n: i32) -> i32 { self.timing.ticks(n) }

    /// The level chunks' words the game writes straight into the save's memory while it plays (chunks 3005 / 3006 of
    /// `level`): the persistent death bits `0x14c190 + L·0x100` (`SetDeathBits` and the other writers of
    /// [`SaveBits::death`]) and the bolt-drop spawner slots `0x14d590 + L·0x100` (`first` from the load's spawn test,
    /// `collected` from `CollectBolt`). The port keeps them in the services; this puts them into the game state (the
    /// save the card writes and the next arrival's spawn test reads), as their writes in the game land there at once.
    pub fn sync_save(&self, gs: &mut crate::game_state::GameState) {
        for &(l, id) in &self.save.death {
            let (Some(lv), Ok(id)) = (gs.levels.get_mut(l as usize), usize::try_from(id)) else { continue };
            if let Some(b) = lv.killed.get_mut(id >> 3) { *b |= 1 << (id & 7); }
        }
        let Some(lv) = gs.levels.get_mut(self.level as usize) else { return };
        for (k, d) in lv.bolt_drops.iter_mut().enumerate() {
            if let Some(&f) = self.counters.spawner_first.get(k).filter(|&&f| f != 0) { d.first = f; }
            if let Some(&n) = self.counters.spawner_bolts.get(&(self.level, k as u8)) { d.collected = n; }
        }
    }

    /// The persistent death bits `0x14c190 + L·0x100` of every level as the game state holds them (chunk 3005: the
    /// loaded save and this session's earlier levels, [`Services::sync_save`]) into [`SaveBits::death`], at the level
    /// entry: in the game they are the save's memory itself, so a class's latch on them (Orxon's lift 1141 activated
    /// once, the gates, the story directors' "done before") holds after a load as after a death.
    pub fn load_death_bits(&mut self, gs: &crate::game_state::GameState) {
        for (l, lv) in gs.levels.iter().enumerate() {
            for (b, &byte) in lv.killed.iter().enumerate() {
                for k in 0..8 {
                    if byte >> k & 1 != 0 { self.save.death.insert((l as u32, (b * 8 + k) as i16)); }
                }
            }
        }
    }

    /// The splines from `rc_formats::gameplay::parse_splines`, as raw words.
    pub fn set_splines(&mut self, s: &[Vec<[f32; 4]>]) {
        self.splines = s.iter().map(|v| v.iter().map(|p| p.map(f32::to_bits)).collect()).collect();
    }
}

// ---------------------------------------------------------------------------------------------------
// Hero-block writes

/// The hero-block fields the moby classes write (level01 stores found in the class updates; every class that
/// writes the hero goes through this, none writes `Hero` itself). In the game the classes store straight into the
/// hero block during the moby loop; the port's loop sees the hero read-only ([`World::hero`], last tick's block), so
/// a class reads and writes these fields through [`World::hero_fields`] / [`World::hero_fields_mut`] (the block as
/// this tick's earlier writes left it: a second class sees the first one's writes, as in the game), and the tick
/// applies the result ([`HeroFields::apply`]) right after the moby loop and before the hero update
/// (`crate::tick::Game::tick_with_hero_sounds`). Nothing reads the hero block between the two in the game's order,
/// so the hero update sees exactly what the classes left. Values keep the hero block's bits (no conversion).
///
/// | field | address | writers (level01) |
/// |---|---|---|
/// | `platform` | 0x13f440..0x13f44c (the push the move adds, its yaw) | flow 679 `0x2f6328` ([`super::classes::flow`]); water current 613 `0x2f3120` (`units::water_current`); the riding floats 664 / 1293 / 1320 (09) and 1069 (07) through [`HeroFields::ride`] (`units::riding_floats`) |
/// | `momentum` | 0x13f4a0..0x13f4ac (carried momentum) | flow 679 |
/// | `sink_hold` | 0x13f530 (s16, holds the sinking floor 0x31) | flow 679 |
/// | `flow` | 0x13fd20 yaw, 0x13fd24 pitch, 0x13fd28, 0x13fd2c speed, 0x13fd30 pull | flow 679 |
/// | `jump_lockout` / `edge_brake` | 0x13f542 / 0x13f544 (s16) | path lift 726 `0x2b9eb0` while ridden with pvar+0xc8 = 0 |
/// | `pose` | 0x13f3d0 position, 0x13f3e8 yaw, 0x13f4d0 target yaw | bolt crank 280 `0x2e0c68` while it turns him ([`super::classes::bolt_crank`]) |
/// | `clear_motion` | `FastMemZero16(0x13f430, 0x90)`: velocity .. slope ratio (0x13f430..0x13f4bf) | bolt crank 280 |
/// | `calls` | the classes' calls into the hero code: `SetState` 0x23cf98, `SetAnim` 0x247a90 ([`HeroCall`]) | bolt crank 280 |
/// | `health` | 0x1415f8 | nanotech cluster 806 `0x300de0` ([`super::classes::pickup`]) |
/// | `water_level` / `dive_lock` | 0x13f640 / 0x13f52e | the water managers of 05 / 12, the water plane 982 of 05 (`crate::water::managers`) |
/// | `ammo` / `ammo_picked` | 0x13d428 / 0x13de08 (game state, Ratchet's mirror) | ammo pickups `0x2db028` (`AddAmmo` 0x2494d8) |
/// | `fall_voice_clear` | 0x141602 (u16 = 0xffff) | level 08's liquid 327 `0x2da0f0` when Ratchet falls in (`crate::water::sea`) |
/// | `speed` / `jump_lock` / `current_dist` | 0x13f4e4 / 0x13f528 / 0x141608 | the water current 613 `0x2f3120` (`units::water_current`; also `platform`, `momentum`) |
/// | `wall_spline` | 0x14162a (s16) | Pokitaru's spline wall 361 `0x2f3350` (`units::pokitaru_wall`) |
///
/// Other class stores into the block, for the classes that are not ported yet (add a field here when one is):
/// the camera / focus objects 0x13fda0; talking NPCs 0x13f3d0
/// (position: `pose`); the Swingshot targets 0x13f904 / 0x13fcd8 / 0x13fcec (`0x2dbdc0`); `0x300de0` 0x13f510 (cheat 6
/// only); the checkpoint record's respawn `0x29adc8` (position / Euler: the engine's respawn); the mode / control
/// bytes 0x1413f5 / 0x1413fc of the vendor, ship and teleporter code. The scripted sequences' `SetState` calls
/// (gunship 688, trooper cameras, vendors' walk to a point) belong in `calls`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeroFields {
    pub platform: [f32; 4],
    pub momentum: [f32; 4],
    pub sink_hold: i16,
    pub flow: [f32; 5],
    pub jump_lockout: i16,
    pub edge_brake: i16,
    /// A class's stores of Ratchet's position / yaw / target yaw (None: unchanged).
    pub pose: Option<HeroPose>,
    /// 0x13f430..0x13f4bf cleared by a class (the `platform` and `momentum` fields above are cleared with it).
    pub clear_motion: bool,
    /// The classes' calls into the hero code this tick, in order ([`HeroFields::call`]).
    pub calls: [Option<HeroCall>; 4],
    /// 0x1415f8: health (the nanotech orbs `0x300de0` heal it).
    pub health: i32,
    /// 0x13d428: ammo by item (the game state's table, mirrored in [`crate::hero::weapons::Weapons::ammo`]; the ammo
    /// pickups' `AddAmmo` 0x2494d8).
    pub ammo: [i32; rc_formats::save_game::ITEM_COUNT],
    /// 0x13de08: ammo picked up (stat) added this tick, by item.
    pub ammo_picked: [i32; rc_formats::save_game::ITEM_COUNT],
    /// `0x1413ff = 1`: the hand item hidden (the gold bolt's pickup; `SetState` clears it on foot).
    pub hide_hand: bool,
    /// A class's screen marker this tick (`FUN_0020fb60`, `crate::targeting::Markers`: Gaspar's cannon 1201's
    /// crosshair), with the tick it was registered in.
    pub marker: Option<(u64, crate::targeting::Marker)>,
    /// 0x13f640: the water level (the water managers of levels 05 / 12 set it near their water).
    pub water_level: f32,
    /// 0x13f52e (s16): the dive lock (level 05's water plane 982 holds it at 5).
    pub dive_lock: i16,
    /// `0x13fcd8 = 0`: the Swingshot's miss flag taken (Kerwan's director 1342 counts the misses).
    pub swing_help_clear: bool,
    /// A class's store of 0x1413f5 (Ratchet hidden: the teleporter pads 1135 while their beam is narrow and after the
    /// teleport), stored after the calls (a `SetState` among them clears it first, as in the game's order).
    pub hero_hidden: Option<u8>,
    /// `0x141602 = 0xffff`: the voice slot of Ratchet's death fall 0x77 forgotten (level 08's liquid 327 releases the
    /// voice when he falls in: `crate::water::sea`).
    pub fall_voice_clear: bool,
    /// 0x13f4e4: Ratchet's current speed (the water current 613 clamps it to 1.5·dt while he swims in it).
    pub speed: f32,
    /// 0x13f528 (s16): the surface-jump lock (`Swim::jump_lock`; the water current 613 holds it at `ticks(35)`).
    pub jump_lock: i16,
    /// 0x141608: the xy distance from Ratchet to the nearest water current this tick (`HeroTickStateTimer` resets it to
    /// 9999 every hero update, so the moby loop starts from 9999; only the currents read it: the nearest one pushes).
    pub current_dist: f32,
    /// A class's store of 0x14161b (no air: the O2 Mask goes on; the Clank-section classes, `crate::hero::bodies`).
    pub airless: Option<u8>,
    /// A class's copy of the hero's position / Euler into 0x141050 / 0x141060 (Giant Clank's pads 1451 / 1899: where he
    /// got in; `crate::hero::bodies::Bodies::entry_pose`).
    pub save_entry_pose: bool,
    /// A class's store of a fixed pose into 0x141050 / 0x141060 (Rilgar's race girl 918: the race's finish point).
    pub set_entry_pose: Option<([f32; 3], [f32; 3])>,
    /// A class's store of 0x13f510 (Ratchet's hit invulnerability, ticks): Veldin's cutaway director 644 (`0x2dfaa0`)
    /// holds him invulnerable for the length of its camera move.
    pub invulnerable: Option<i32>,
    /// A class's store of 0x141414 (the back slot's item request, `SessionState::temp_back`): the boss 1422's state 5
    /// asks for the Thruster-Pack (3).
    pub back_request: Option<i32>,
    /// A class's store of 0x141410 (the head slot's item request, `SessionState::temp_head`): the Pilot's Helmet pickup
    /// 1290 (level01 `0x30a6d0`) asks for item 7.
    pub head_request: Option<i32>,
    /// A class's store of 0x141628 (Clank hidden on Ratchet's back, `SessionState::clank_hidden`): Veldin's Clank 834.
    pub clank_hidden: Option<i16>,
    /// A class's store of 0x13f51c (`Hero::no_vel_clamp`): Hoven's turret 1267 holds it at `ticks(120)` while Ratchet
    /// rides it.
    pub no_vel_clamp: Option<i32>,
    /// A class's store of the level's death height 0x15f638 (`Services::death_z` is the moby loop's copy): Kalebo's race
    /// host 1455 (100 / 115, 74 during the race). The tick takes it into `Game::death_z`.
    pub death_z: Option<f32>,
    /// 0x13fbbc = the board moby (the Hoverboard's class 439 before its `SetState(0x6b, 1)`; `hero::hoverboard`).
    pub board: Option<MobyId>,
    /// 0x13fc14 += n: the Hoverboard's boost from a pickup 133 (`classes::units::board_boost`).
    pub board_boost: i32,
    /// A class's store of 0x13f51a (s16, `Hero::f51a`: the grind reach's wider catch): Veldin's rail chooser 582 holds
    /// it at 5.
    pub rail_reach: Option<i16>,
    /// A class's stores of grind path radii (`0x15f70c[i]` +0x0c, the bounding sphere's: 0 takes the rail out of the
    /// grind search), in order: Veldin's rail chooser 582. The tick applies them to the hero's rails
    /// ([`HeroFields::set_rail_radius`]; the moby loop's own copy is `Services::volumes`).
    pub rail_radius: [Option<(u16, f32)>; 8],
    /// A class's store of 0x141400 (`Gadgets::hydro_full`: the Hydrodisplacer holds water): Veldin's pools 1402 give a
    /// full gadget back at their first update.
    pub hydro_full: Option<bool>,
    /// A class's store of 0x14162a (`Hero::wall_spline`: the spline the capsule pass keeps Ratchet off): Pokitaru's 361.
    pub wall_spline: Option<i16>,
    /// A store of Ratchet's rail cursor 0x13f8b4 / 0x13f8b8 (segment, units along it): Oltanis's race start
    /// (`crate::follow_camera::race`).
    pub rail_cursor: Option<(i32, f32)>,
    /// A store of Ratchet's head record (3) targets y / z (0x17aff4 / 0x17aff8 on level 14): the race camera's stage 1.
    pub head_look: Option<[f32; 2]>,
}

/// `0x27fe88(p, out, centre, e_old, e_new)` (level09; level07's copy `0x288968`, the same code): `p` turned about
/// `centre` by the change from the Euler rotation `e_old` to `e_new`: `out = centre + (p − centre)·E(e_old)ᵀ·E(e_new)`
/// (`euler_to_matrix` 0x1fa050 twice, the transpose 0x1fa2d8, two `fun_001f9d20` row-vector products). The riding
/// floats ([`HeroFields::ride`]).
pub fn turn_about(p: [f32; 3], centre: [f32; 3], e_old: [f32; 3], e_new: [f32; 3]) -> [f32; 3] {
    let (a, b) = (super::triggers::euler_matrix(e_old), super::triggers::euler_matrix(e_new));
    let v = [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]].map(|x| x as f64);
    // v·Aᵀ: component i = row i of A · v.
    let u: [f64; 3] = std::array::from_fn(|i| (0..3).map(|j| a[i][j] * v[j]).sum());
    // u·B: Σ u_j · row j of B.
    let o: [f64; 3] = std::array::from_fn(|k| (0..3).map(|j| u[j] * b[j][k]).sum());
    [o[0] as f32 + centre[0], o[1] as f32 + centre[1], o[2] as f32 + centre[2]]
}

/// Ratchet's pose as a class stores it (native `f32`): position 0x13f3d0 (x, y, z; w kept), yaw 0x13f3e8, target
/// yaw 0x13f4d0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeroPose {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub target_yaw: f32,
}

/// A call a class makes into the hero code during the moby loop. The port runs it right before the hero update
/// with the hero's context ([`HeroFields::run_calls`]); in the game it runs at the class's point in the loop (the
/// only difference: an RNG draw of the entry, e.g. idle's head-look timer, lands after the loop's later draws).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HeroCall {
    /// `SetState(id, play)` 0x23cf98.
    SetState { id: i32, play: bool },
    /// The rest of a class's `HeroTeleport` 0x2368e0 ([`crate::cinematic::hero_teleport`]) at the position and yaw its
    /// `pose` stored: the hero side's [`Hero::teleport`] (airborne, the platform carry dropped, the motion block
    /// cleared, the weapon put away, `SetState(state, 1)` unless −1, the ground probe). The camera reset is the
    /// class side's call.
    Teleport { state: i32 },
    /// `SetAnim(blend, seq, frame)` 0x247a90 (`blend` = `(float)ticks(n)`), as the hero code's own
    /// ([`Hero::set_anim`]).
    SetAnim { blend: f32, seq: u8, frame: i32 },
    /// The death sequence `0x2319b0` ([`crate::hero::damage::death_fade`]: the deaths counted, the fade and the
    /// reload flag 0x141401), called by a class (the kill cuboids 1039).
    Death,
    /// `0x249580(yaw, point, release)`: Ratchet walks to `point` and turns to `yaw` (the walk-to states 0x65..0x67,
    /// `crate::hero::stance`); the teleporter pads 1135 on △.
    WalkTo { point: [f32; 3], yaw: f32, release: i32 },
    /// `SwitchCharacter(mode, state, moby)` (0x231348 and its level copies, `crate::hero::bodies::switch_character`): the
    /// body moby `moby` (class `o_class`, its animation fields `anim` when the class asked) becomes the hero.
    SwitchCharacter { mode: u8, state: i32, moby: MobyId, o_class: i16, anim: rc_formats::moby_anim::AnimState },
    /// The level's leave-the-body copy (0x231450 and its copies, `crate::hero::bodies::leave_body`); `game_mode` = 0x15f5c4.
    LeaveBody { game_mode: i32 },
    /// The per-body idle `0x227638` (`crate::hero::bodies::body_idle`).
    BodyIdle,
    /// A class's store `0x141401 = 1` without the death sequence (the reload of the level, `Hero::fell_out`; no death
    /// counted): Kerwan's train 822 when Ratchet falls off it (its `FadeToBlack(ticks(16))` goes with it).
    Reload,
    /// The race score 0x13fbf8 += n (Kalebo's racers 556 when shot: `hero::hoverboard`'s score).
    RaceScore(i32),
}

impl HeroFields {
    /// The fields of `h` now.
    pub fn of(h: &Hero) -> HeroFields {
        HeroFields {
            platform: fv(h.platform),
            momentum: fv(h.momentum),
            sink_hold: h.f530,
            flow: h.surf.flow,
            jump_lockout: h.jump_lockout,
            edge_brake: h.edge_brake,
            pose: None,
            clear_motion: false,
            calls: [None; 4],
            health: h.health,
            ammo: h.weapons.ammo,
            ammo_picked: [0; rc_formats::save_game::ITEM_COUNT],
            hide_hand: false,
            marker: None,
            water_level: f32::from_bits(h.water_level.0),
            dive_lock: h.swim.dive_lock,
            swing_help_clear: false,
            hero_hidden: None,
            fall_voice_clear: false,
            speed: f32::from_bits(h.speed.0),
            jump_lock: h.swim.jump_lock,
            current_dist: 9999.0,
            airless: None,
            save_entry_pose: false,
            set_entry_pose: None,
            invulnerable: None,
            back_request: None,
            head_request: None,
            clank_hidden: None,
            no_vel_clamp: None,
            death_z: None,
            board: None,
            board_boost: 0,
            rail_reach: None,
            rail_radius: [None; 8],
            hydro_full: None,
            wall_spline: None,
            rail_cursor: None,
            head_look: None,
        }
    }

    /// Queues a grind path's radius store (a ninth in one tick is dropped: the chooser 582 makes eight at most).
    pub fn set_rail_radius(&mut self, rail: usize, r: f32) {
        if let Some(s) = self.rail_radius.iter_mut().find(|s| s.is_none()) { *s = Some((rail as u16, r)); }
    }

    /// Queues a call into the hero code (in order; a fifth call in one tick is dropped: no class makes more
    /// than one).
    pub fn call(&mut self, c: HeroCall) {
        if let Some(s) = self.calls.iter_mut().find(|s| s.is_none()) { *s = Some(c); }
    }

    /// A riding class's store of Ratchet's platform delta (G-HERO-034: the floats 664 / 1293 / 1320 of level 09, 1069 of
    /// level 07, while he stands on them): `vec_sub(0x13f440, out, 0x13f3d0)` (xyz = `out − hero`, w = `out.w` = the
    /// hero's w through the rotation's identity row) then `0x13f448 += dz` (the float's own height change this tick).
    /// The hero's move adds it to his position ([`HeroFields::platform`]; `Hero::platform`). `hero` = Ratchet's position
    /// as the class read it (0x13f3d0, x y z w), `out` = where the class carries him ([`turn_about`]).
    pub fn ride(&mut self, hero: [f32; 4], out: [f32; 3], dz: f32) {
        self.platform = [out[0] - hero[0], out[1] - hero[1], (out[2] - hero[2]) + dz, hero[3]];
    }

    /// `FastMemZero16(0x13f430, 0x90)`: velocity, platform delta, displacement, effective velocities, applied
    /// platform displacement, momentum, their lengths and the slope ratio.
    pub fn clear_motion(&mut self) {
        self.platform = [0.0; 4];
        self.momentum = [0.0; 4];
        self.clear_motion = true;
    }

    /// Stores the fields into `h` (bit for bit: a field no class changed keeps its value).
    pub fn apply(&self, h: &mut Hero) {
        h.platform = pv(self.platform);
        h.momentum = pv(self.momentum);
        h.f530 = self.sink_hold;
        h.surf.flow = self.flow;
        h.jump_lockout = self.jump_lockout;
        h.edge_brake = self.edge_brake;
        if self.clear_motion {
            let z = ph::V0;
            (h.vel, h.disp, h.eff, h.eff_v, h.eff_h, h.plat_applied) = (z, z, z, z, z, z);
            (h.eff_len, h.eff_len_xy, h.fwd_speed, h.slope_ratio) = (Pf::ZERO, Pf::ZERO, Pf::ZERO, Pf::ZERO);
        }
        h.health = self.health;
        h.weapons.ammo = self.ammo;
        h.water_level = Pf(self.water_level.to_bits());
        h.swim.dive_lock = self.dive_lock;
        if self.swing_help_clear { h.swing.help = 0; }
        if self.fall_voice_clear { h.damage.voice_slot = -1; }
        h.speed = Pf(self.speed.to_bits());
        h.swim.jump_lock = self.jump_lock;
        for (t, n) in h.weapons.picked.iter_mut().zip(self.ammo_picked.iter()) { *t += n; }
        if let Some(v) = self.airless { h.worn.airless = v; }
        if self.save_entry_pose { h.bodies.entry_pose = Some((ph::to_f32x3(h.pos), ph::to_f32x3(h.rot))); }
        if let Some(pose) = self.set_entry_pose { h.bodies.entry_pose = Some(pose); }
        if let Some(t) = self.invulnerable { h.f510 = t; }
        if let Some(v) = self.back_request { h.back_slot.slot.request = v; }
        if let Some((t, m)) = self.marker { h.weapons.markers.register(t, m); }
        if let Some(v) = self.head_request { h.head_slot.request = v; }
        if let Some(v) = self.clank_hidden { h.back_slot.clank_hidden = v; }
        if let Some(v) = self.no_vel_clamp { h.no_vel_clamp = v; }
        if let Some(v) = self.rail_reach { h.f51a = v; }
        if let Some(v) = self.hydro_full { h.gadgets.hydro_full = v; }
        if let Some(v) = self.wall_spline { h.wall_spline = v; }
        if let Some((seg, t)) = self.rail_cursor { h.boots.cur = crate::spline::Cursor { seg, t }; }
        if let Some([y, z]) = self.head_look {
            let r = &mut h.idle.joints[crate::hero::idle::joint::HEAD];
            r.target[1] = y;
            r.target[2] = z;
        }
        if let Some(b) = self.board { h.board.moby = Some(b); }
        h.board.boost_timer += self.board_boost;
        if let Some(p) = self.pose {
            h.pos = [pf(p.pos[0]), pf(p.pos[1]), pf(p.pos[2]), h.pos[3]];
            h.rot[2] = pf(p.yaw);
            h.target_yaw = pf(p.target_yaw);
        }
    }

    /// Runs the queued calls on the hero ([`HeroCall`]), in order, with the hero's context of this tick.
    pub fn run_calls(&self, h: &mut Hero, c: &mut crate::hero::states::Ctx) { self.run_calls_with(h, c, None) }

    /// [`HeroFields::run_calls`] with the item slots' pass a `SwitchCharacter` makes (`crate::hero::items::slot_pass`
    /// with the tick's item environment: `crate::hero::bodies::switch_character_with`).
    pub fn run_calls_with(&self, h: &mut Hero, c: &mut crate::hero::states::Ctx, mut pass: Option<crate::hero::bodies::SlotPass>) {
        for call in self.calls.iter().flatten() {
            match *call {
                HeroCall::SetState { id, play } => { h.set_state(c, id, play); }
                HeroCall::Teleport { state } => {
                    let (p, r) = (h.pos.map(|x| x.to_f32()), h.rot.map(|x| x.to_f32()));
                    h.teleport(c, p, r, state, false);
                }
                HeroCall::SetAnim { blend, seq, frame } => h.set_anim(c.anim, c.rng, pf(blend), seq, frame),
                HeroCall::Death => crate::hero::damage::death_fade(h),
                HeroCall::WalkTo { point, yaw, release } => {
                    // `0x249580(yaw, point, release)`: the walk-to target 0x140990 / 0x14099c / 0x1409a0, then
                    // `SetState(0x65, 1)` unless Ratchet already walks to a point (0x65..0x67).
                    h.walk_to.point = [pf(point[0]), pf(point[1]), pf(point[2]), h.walk_to.point[3]];
                    h.walk_to.yaw = pf(yaw);
                    h.walk_to.release = release;
                    if !(0x65..=0x67).contains(&h.state) { h.set_state(c, 0x65, true); }
                }
                HeroCall::SwitchCharacter { mode, state, moby, o_class, anim } => {
                    let b = crate::hero::bodies::BodyMoby { id: moby, o_class, anim };
                    match pass {
                        Some(ref mut f) => crate::hero::bodies::switch_character_with(h, c, mode, state, b, Some(&mut **f)),
                        None => crate::hero::bodies::switch_character_with(h, c, mode, state, b, None),
                    }
                }
                HeroCall::LeaveBody { game_mode } => crate::hero::bodies::leave_body(h, c, game_mode),
                HeroCall::BodyIdle => crate::hero::bodies::body_idle(h, c),
                HeroCall::Reload => h.fell_out = 1,
                HeroCall::RaceScore(n) => h.board.score += n,
            }
        }
        // After the calls: the gold bolt stores 0x1413ff after its HeroTeleport's SetState (which clears it).
        if self.hide_hand { h.f13ff = 1; }
        // The teleporter's store of 0x1413f5 after its HeroTeleport's SetState (which clears it).
        if let Some(v) = self.hero_hidden { h.f13f5 = v; }
    }
}

/// The game's globals outside the moby system that the moby loop's classes read, as the loop sees them: the pad
/// 0x13c940 after this tick's `UpdatePad`, the camera Euler 0x167250 of the last camera update and Ratchet's
/// animation fields (+0x50..+0x54 and 0x13fde8 / 0x13fdec) after his last update. The tick fills
/// [`Hero::loop_in`] with them right before the moby loop (`crate::tick::Game::tick_with_hero_sounds`); a class
/// reads them through [`World::hero`].
#[derive(Clone, Debug, Default)]
pub struct LoopGlobals {
    pub pad: crate::pad::PadState,
    pub cam_euler: [f32; 3],
    pub anim: crate::hero::AnimView,
    /// The current camera's own position (0x167280 +0x30) after the last camera update (the camera moby 1007 reads
    /// it: `crate::follow_camera::camera_moby`).
    pub cam_pos: [f32; 3],
}

// ---------------------------------------------------------------------------------------------------
// The world a class update sees

/// What a class update reads and writes besides its own moby: the game's globals at the time the moby loop
/// runs (after the pad, before the hero: the hero fields are last tick's).
pub struct World<'a> {
    pub table: &'a mut MobyTable,
    /// The hero block (0x13f350..0x141660).
    pub hero: &'a Hero,
    /// `0x1413d0`: the hero moby (Ratchet's, or the body moby while a body is in).
    pub hero_moby: Option<MobyId>,
    /// `0x167240`: camera position (written by the previous tick's camera update).
    pub camera: V4,
    /// `0x167258`: the camera's yaw (the previous tick's; level18's copy of the camera block has it at 0x1677d8).
    pub camera_yaw: f32,
    /// `0x167450` / `0x167460` / `0x167470`: the camera's forward, left and up rows (the previous tick's; level18's
    /// copy at 0x1679d0..).
    pub camera_rows: [[f32; 3]; 3],
    /// The current camera's class (`[0x167280]+0x86`, the previous tick's camera update).
    pub camera_class: i32,
    /// `0x16d140..`: the view the crate respawn's `FastBSphereCheck` uses (None: never out of view).
    pub view: Option<&'a BSphereView>,
    /// The game's one `rand` stream (shared with the particles and the hero).
    pub rng: &'a mut Rng,
    /// The level collision (world mesh). None: the world pass finds nothing (the moby pass still runs, on
    /// [`Services::grid`] / [`Services::coll_classes`]).
    pub coll: Option<&'a Collision>,
    /// `0x15f5cc`: the tick counter.
    pub counter: u64,
    pub classes: &'a dyn ClassData,
    /// The particle system (None: spawns are only counted in [`FxStats`], without their RNG draws).
    pub particles: Option<&'a mut Particles>,
    pub sound: Option<&'a mut dyn SoundSink>,
    pub missions: &'a dyn MissionState,
    pub inventory: &'a dyn Inventory,
    /// Class updates ported outside `moby_update` (the emitters 27, …), run in the moby order.
    pub external: Option<&'a mut dyn ExternalUpdates>,
    pub svc: &'a mut Services,
}

/// Updates of classes whose ports live outside `moby_update` (the
/// class-27 emitters of `particles::type06`, …). The scheduler calls them at the moby's place in the run
/// order (load pass: array order; ticks: class-slot order) with the one shared RNG, so their draws land where
/// the game makes them.
pub trait ExternalUpdates {
    /// The level-table address this handles for `o_class` (`Some`: the class gets an update function, so
    /// `InitMobyInstance` does not set mode 2).
    fn update_fn(&self, o_class: i16) -> Option<u32>;
    /// `(*moby+0x74)(moby)` for an address [`update_fn`](Self::update_fn) returned. `particles` is the
    /// world's particle system (the class-27 emitters spawn into it).
    fn update(&mut self, addr: u32, id: MobyId, table: &mut MobyTable, rng: &mut Rng, camera: V4, counter: u64, particles: Option<&mut Particles>);
}

/// A `CollLine_Fix` result with its moby (`CollOutput+0x18`).
#[derive(Clone, Copy, Debug)]
pub struct LineHit {
    pub moby: Option<MobyId>,
    /// +0x20 hit point (w from the kernel: 0 here).
    pub point: V4,
    /// +0x40 raw normal (a moby primitive: hit − primitive centre).
    pub normal: V4,
}

impl LineHit {
    fn of(h: &CollOutput) -> LineHit {
        LineHit { moby: h.moby, point: [pf(h.point[0]), pf(h.point[1]), pf(h.point[2]), Pf::ZERO], normal: [pf(h.normal[0]), pf(h.normal[1]), pf(h.normal[2]), Pf::ZERO] }
    }
}

/// The empty mesh for a world without collision.
fn no_mesh() -> &'static Collision {
    static EMPTY: std::sync::OnceLock<Collision> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Collision::default)
}

impl<'a> World<'a> {
    /// A world with no collision, particles, sound sink, view or external updates, no missions and an
    /// empty inventory; the camera at the hero.
    pub fn new(table: &'a mut MobyTable, hero: &'a Hero, rng: &'a mut Rng, classes: &'a dyn ClassData, svc: &'a mut Services, counter: u64) -> World<'a> {
        // `0x1413d0`: the body moby while a body is in (Clank, Giant Clank, the disguise), else Ratchet's: the classes'
        // "is it the hero" tests (the gadgetbots' hits from Clank, ...) see the body (`Hero::hero_moby`).
        let hero_moby = table.hero().map(|r| hero.hero_moby(r));
        World {
            table,
            hero,
            hero_moby,
            camera: hero.pos,
            camera_yaw: 0.0,
            camera_rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            camera_class: 0,
            view: None,
            rng,
            coll: None,
            counter,
            classes,
            particles: None,
            sound: None,
            missions: &NoMissions,
            inventory: &NoInventory,
            external: None,
            svc,
        }
    }

    pub fn m(&self, id: MobyId) -> &Moby { &self.table.mobys[id] }
    pub fn mm(&mut self, id: MobyId) -> &mut Moby { &mut self.table.mobys[id] }
    pub fn ticks(&self, n: i32) -> i32 { self.svc.ticks(n) }

    /// The hero-block fields the classes write ([`HeroFields`]) as this tick's earlier class updates left them
    /// (else the hero's own).
    pub fn hero_fields(&self) -> HeroFields {
        match self.svc.hero_writes {
            Some((c, f)) if c == self.counter => f,
            _ => HeroFields::of(self.hero),
        }
    }

    /// The body word 0x1413f4 as the classes see it now (0 Ratchet, 1 Clank, 2 Giant Clank, 3 the disguise): the game's
    /// `SwitchCharacter` and leave copies store it at once, so a class later in the same moby loop sees the new body
    /// (Orxon's sliding blocks 1424 placed open in the loop where the Clank section 22 hands Clank the hero); the port
    /// makes those calls after the loop ([`HeroCall`]), so the last one queued this tick wins over the hero's word.
    pub fn body(&self) -> u8 {
        self.hero_fields().calls.iter().flatten().fold(self.hero.mode, |b, c| match c {
            HeroCall::SwitchCharacter { mode, .. } => *mode,
            HeroCall::LeaveBody { .. } => crate::hero::bodies::body::RATCHET,
            _ => b,
        })
    }

    /// Write access to the hero-block fields of [`HeroFields`] (a store of the game's class into the hero block);
    /// the tick applies them before the hero update.
    pub fn hero_fields_mut(&mut self) -> &mut HeroFields {
        let f = self.hero_fields();
        &mut self.svc.hero_writes.insert((self.counter, f)).1
    }

    /// A camera shake (a class's stores into the shake record 0x167260 / 0x167270: amplitude, ticks); the camera
    /// applies it in this tick's `CameraUpdate` (`crate::follow_camera::Shake`).
    pub fn shake_camera(&mut self, r: crate::follow_camera::ShakeRequest) { self.svc.camera_shakes.push(r); }

    /// The bolt pickup radii `(0x1415d8 xy, 0x1415dc z)`: hero init 0x226b70 writes 2.125 / 1.25; every hero
    /// tick (`HeroTickStateTimer` 0x23c710) rewrites them as 3.0 / 1.75, or 12 / 4.5 with the bolt grabber
    /// (0x13d4e2). The mobys run before the hero, so tick 0 sees the init values.
    pub fn bolt_radii(&self) -> (Pf, Pf) {
        if self.counter == 0 { return (Pf::b(0x4008_0000), Pf::b(0x3fa0_0000)); }
        // The other bodies (`HeroTickStateTimer`, crate::hero::bodies): Clank 2.125 / 1.25, Giant Clank 15 / 3.
        match self.body() {
            1 => return (Pf::b(0x4008_0000), Pf::b(0x3fa0_0000)),
            2 => return (Pf::b(0x4170_0000), Pf::b(0x4040_0000)),
            _ => {}
        }
        if self.svc.bolt_grabber { (Pf::b(0x4140_0000), Pf::b(0x4090_0000)) } else { (Pf::b(0x4040_0000), Pf::b(0x3fe0_0000)) }
    }

    /// `0x13f390`: the transpose of the hero rows 0x13f350 (`HeroMotionUpdate` 0x231d18: identity, then
    /// `fun_001fa2d8(0x13f390, 0x13f350)`).
    pub fn hero_inverse(&self) -> [V4; 4] {
        let r = self.hero.rows;
        transpose(&[r[0], r[1], r[2]])
    }

    /// `CollLine_Fix(p0, p1, flags, ignore, 0)`: the world mesh, then the mobys ([`coll_line_m`]).
    pub fn line(&self, p0: V4, p1: V4, flags: u32, ignore: Option<MobyId>) -> Option<LineHit> {
        self.coll_line(p0, p1, flags, ignore).map(|h| LineHit::of(&h))
    }

    /// `CollLine_Fix(p0, p1, flags, ignore, 0)` with the whole `CollOutput`.
    pub fn coll_line(&self, p0: V4, p1: V4, flags: u32, ignore: Option<MobyId>) -> Option<CollOutput> {
        let src = self.svc.scene_parts(self.table, self.classes);
        let sc = self.svc.scene(&src);
        coll_line_m(self.coll.unwrap_or(no_mesh()), Some(&sc), ph::to_f32x3(p0), ph::to_f32x3(p1), QueryFlags(flags), ignore)
    }

    /// The sphere kernel 0x212960 `(r, &centre, flags, ignore)`: the world mesh, then the mobys.
    /// `coll_capsule(r, h, &base, flags, ignore)` 0x2135a0 (level04 `0x1e4308`): the vertical capsule `base .. base +
    /// (0, 0, h)` of radius `r`, world and mobys; the output's pushed centre is the base moved out of the hit.
    pub fn coll_capsule(&self, base: [f32; 3], height: f32, r: f32, flags: u32, ignore: Option<MobyId>) -> Option<CollOutput> {
        let src = self.svc.scene_parts(self.table, self.classes);
        let sc = self.svc.scene(&src);
        crate::collision_query::coll_capsule_m(self.coll.unwrap_or(no_mesh()), Some(&sc), base, height, r, QueryFlags(flags), ignore)
    }

    pub fn coll_sphere(&self, centre: V4, r: Pf, flags: u32, ignore: Option<MobyId>) -> Option<CollOutput> {
        let src = self.svc.scene_parts(self.table, self.classes);
        let sc = self.svc.scene(&src);
        coll_sphere_m(self.coll.unwrap_or(no_mesh()), Some(&sc), ph::to_f32x3(centre), fl(r), QueryFlags(flags), ignore)
    }

    /// `GroundHeight(up, pos, fl)` 0x26e618: `CollLine_Fix((x, y, z + up), (x, y, 0.01), fl | 2)`; the hit z,
    /// else 0.
    pub fn ground_height(&self, up: Pf, pos: V4, fl: u32) -> Pf {
        let a = [pos[0], pos[1], pos[2] + up, pos[3]];
        let b = [pos[0], pos[1], Pf::b(0x3c23_d70a), pos[3]];
        self.line(a, b, fl | 2, None).map(|h| h.point[2]).unwrap_or(Pf::ZERO)
    }

    /// `PlayClassSound(idx, flags, moby)` 0x2a1618 (`sound_class` = the moby's class) or 0x2a16c0 (another
    /// class's table): through the [`SoundSink`] (its RNG draw included), and recorded.
    pub fn play_sound_as(&mut self, index: i32, flags: u32, id: MobyId, sound_class: i16) -> i32 {
        let m = &self.table.mobys[id];
        let ev = SoundEvent { index, flags, moby: id, o_class: m.o_class, sound_class, pos: [m.position[0], m.position[1], m.position[2]], tick: self.counter };
        self.svc.sounds.push(ev);
        match self.sound.as_deref_mut() {
            Some(s) => s.play_class_sound(&ev, self.rng),
            None => -1,
        }
    }

    pub fn play_sound(&mut self, index: i32, flags: u32, id: MobyId) -> i32 {
        let c = self.table.mobys[id].o_class;
        self.play_sound_as(index, flags, id, c)
    }

    /// `PlayLevelSoundAtMoby(index, flags, moby)` 0x2a1770 ([`SoundSink::play_level_sound`]): `id` None = moby 0 (2-D
    /// at the listener), as every caller passes.
    pub fn play_level_sound(&mut self, index: i32, flags: u32, id: Option<MobyId>) -> i32 {
        let at = id.map(|i| {
            let p = self.table.mobys[i].position;
            (i, [p[0], p[1], p[2]])
        });
        let tick = self.counter;
        match self.sound.as_deref_mut() {
            Some(s) => s.play_level_sound(index, flags, at, tick, self.rng),
            None => -1,
        }
    }

    /// Level03 `0x27a618(index, flags, moby)` ([`SoundSink::play_level_def`]): recorded in [`Services::level_defs_played`].
    pub fn play_level_def(&mut self, index: i32, flags: u32, id: MobyId) -> i32 {
        let p = self.table.mobys[id].position;
        self.svc.level_defs_played.push((index, id));
        let tick = self.counter;
        match self.sound.as_deref_mut() {
            Some(s) => s.play_level_def(index, flags, Some((id, [p[0], p[1], p[2]])), tick, self.rng),
            None => -1,
        }
    }

    /// `SoundIsAlive(moby, slot)` 0x27e820 ([`SoundSink::alive`]; without a sink: any slot ≠ −1).
    pub fn sound_alive(&self, slot: i32, id: MobyId) -> bool {
        match self.sound.as_deref() {
            Some(s) => s.alive(slot, id),
            None => slot != -1,
        }
    }

    /// `release_voice_slot(slot)` guarded by the slot's owner ([`SoundSink::release`]).
    pub fn release_sound(&mut self, slot: i32, id: MobyId) {
        if let Some(s) = self.sound.as_deref_mut() { s.release(slot, id); }
    }

    /// The owner of voice slot `slot` (+0x18: None when freed or out of range) ([`SoundSink::slot_owner`]).
    pub fn sound_owner(&self, slot: i32) -> Option<MobyId> {
        if slot < 0 { return None; }
        self.sound.as_deref().and_then(|s| s.slot_owner(slot)).map(|(o, _)| o)
    }

    /// A class's store of itself and `pos` into voice slot `slot`'s owner and position ([`SoundSink::hand_over`]).
    pub fn hand_over_sound(&mut self, slot: i32, id: MobyId, pos: [f32; 3]) {
        if slot < 0 { return; }
        if let Some(s) = self.sound.as_deref_mut() { s.hand_over(slot, id, pos); }
    }

    /// `SoundSetPitchBend(slot, pb)` 0x2a1988 ([`SoundSink::set_pitch_bend`]; the game writes slot −1's field too:
    /// memory before the table, nothing here).
    pub fn set_pitch_bend(&mut self, slot: i32, pb: i32) {
        if slot < 0 { return; }
        if let Some(s) = self.sound.as_deref_mut() { s.set_pitch_bend(slot, pb); }
    }

    /// `SoundSetVolume(slot, vol)` 0x2a1968 ([`SoundSink::set_volume`]).
    pub fn set_volume(&mut self, slot: i32, vol: i32) {
        if slot < 0 { return; }
        if let Some(s) = self.sound.as_deref_mut() { s.set_volume(slot, vol); }
    }

    /// `CreateMoby(o_class)` 0x263390 with the class as loaded (the game's init defaults, a zeroed 0x80-byte
    /// pvar block); decrements the free-slot count (`MobyTable::free_slots`, in [`MobyTable::create`]). The
    /// snapshot slot of the new moby is cleared.
    pub fn create_moby(&mut self, o_class: i16) -> Option<MobyId> {
        let info = self.classes.info(o_class);
        let id = self.table.create(o_class, info.as_ref(), self.counter)?;
        crate::moby_update::anim_sound::init(&mut self.table.mobys[id], self.classes.anim(o_class));
        if self.svc.snapshots.len() <= id { self.svc.snapshots.resize(id + 1, None); }
        self.svc.snapshots[id] = None;
        Some(id)
    }

    /// `DeleteMoby` 0x2636c0: state 0xfd / 0xfe, the reuse tick, and the grid removal
    /// (`UpdateMobyGrids(moby, 0x80807f7f)`).
    pub fn delete_moby(&mut self, id: MobyId) {
        self.table.delete(id, self.counter);
        Arc::make_mut(&mut self.svc.grid).remove(&mut self.table.mobys[id]);
    }

    /// `MobyGetHitMessage(m, mask, keep)` 0x26f320: the record in slot `moby+0xa4` when it targets this moby
    /// and `flags & mask ≠ 0`. A record for this moby whose flags miss the mask is dropped (`+0xa4 = 0xff`)
    /// unless `keep`.
    pub fn get_hit(&mut self, id: MobyId, mask: u32, keep: bool) -> Option<HitRecord> {
        let slot = self.table.mobys[id].hit_slot;
        if slot == 0xff { return None; }
        let r = *self.svc.hits.records.get(slot as usize)?;
        if r.target != id { return None; }
        if r.flags & mask == 0 {
            if !keep { self.table.mobys[r.target].hit_slot = 0xff; }
            return None;
        }
        Some(r)
    }

    /// `FUN_0026e968(target, tmpl)`: the canonical hit delivery. Skipped when `target+0xa4` already holds a
    /// record for it with a larger damage (`tmpl.damage < old`, `c.lt.s`); else the next slot (`0x1742d4`) gets
    /// the record (+0x00 zero) and `target+0xa4` points at it. For the wrench / weapons (not ported yet).
    pub fn deliver_hit(&mut self, target: MobyId, t: &HitTemplate) { deliver_hit_in(self.table, &mut self.svc.hits, target, t); }

    /// `coll_sphere_mobys(r, centre, flags, ignore, tmpl)` 0x214468 ([`sphere_mobys_in`]). Returns the number
    /// of mobys listed.
    pub fn sphere_mobys(&mut self, r: Pf, centre: V4, flags: u32, ignore: Option<MobyId>, tmpl: Option<&HitTemplate>) -> usize {
        sphere_mobys_in(self.table, self.svc, self.classes, r, centre, flags, ignore, tmpl).len()
    }

    /// [`Self::sphere_mobys`] returning the mobys listed, in order.
    pub fn sphere_mobys_list(&mut self, r: Pf, centre: V4, flags: u32, ignore: Option<MobyId>, tmpl: Option<&HitTemplate>) -> Vec<MobyId> {
        sphere_mobys_in(self.table, self.svc, self.classes, r, centre, flags, ignore, tmpl)
    }

    pub fn class_scale(&self, o_class: i16) -> Pf {
        self.classes.info(o_class).map(|c| pf(c.scale)).unwrap_or(Pf::ZERO)
    }

    pub fn anim_class(&self, o_class: i16) -> Option<&MobyAnimClass> { self.classes.anim(o_class) }

    /// `fun_00212f90` (0x26c660) `MobyAnimBlend(m, seq, frame, ticks)` on the moby's animation state and
    /// snapshot slot. False when the class has no such sequence (nothing changes).
    pub fn anim_blend(&mut self, id: MobyId, seq: u8, frame: i32, ticks: i32) -> bool {
        let o = self.table.mobys[id].o_class;
        let Some(class) = self.classes.anim(o) else { return false };
        if self.svc.snapshots.len() <= id { self.svc.snapshots.resize(id + 1, None); }
        let snap = &mut self.svc.snapshots[id];
        let ok = rc_formats::moby_anim::set_sequence(&mut self.table.mobys[id].anim, class, seq, frame, ticks, snap);
        // +0x7c = the target's loop sound (crate::moby_update::anim_sound).
        if ok { crate::moby_update::anim_sound::after_sequence_change(&mut self.table.mobys[id], class); }
        ok
    }

    /// `MobyAnimBlendEx(m, seq, frame, ticks, flags)` 0x26c7a8 (boot `0x2130d8`): [`World::anim_blend`] that also
    /// snapshots the current pose when no blend runs with flag 4 (`rc_formats::moby_anim::set_sequence_ex`).
    pub fn anim_blend_ex(&mut self, id: MobyId, seq: u8, frame: i32, ticks: i32, flags: u32) -> bool {
        let o = self.table.mobys[id].o_class;
        let Some(class) = self.classes.anim(o) else { return false };
        if self.svc.snapshots.len() <= id { self.svc.snapshots.resize(id + 1, None); }
        let snap = &mut self.svc.snapshots[id];
        let ok = rc_formats::moby_anim::set_sequence_ex(&mut self.table.mobys[id].anim, class, seq, frame, ticks, snap, flags & 4 != 0);
        if ok { crate::moby_update::anim_sound::after_sequence_change(&mut self.table.mobys[id], class); }
        ok
    }

    /// `MobyBuildMatrix` 0x265bd8 on one moby.
    /// 0x14c050 + L·16 as the game has it at this point of the tick: `SetMissionDone` 0x265080 writes it at once, the
    /// port's request ([`crate::cinematic::set_mission_done`]) is applied by the engine after the tick, so a mission
    /// done earlier in this tick (a checkpoint's own, which its record reads right after) already counts.
    pub fn mission_done(&self, level: u32, mission: u8) -> u8 {
        let pending = level == self.svc.level
            && self.svc.cinematic.requests.iter().any(|r| matches!(r, crate::cinematic::EngineRequest::MissionDone { mission: m } if *m == mission));
        if pending { 0xff } else { self.missions.mission_done(level, mission) }
    }

    pub fn build_matrix(&mut self, id: MobyId) { self.svc.build_matrix_in(self.table, self.classes, id); }

    /// `FUN_002645a8(moby, list, out)` 0x2645a8: the world point of the last joint of the class's joint list
    /// `list` in the moby's current pose. The partial pose `fun_00210850` (`rc_formats::moby_anim::evaluate_chain`)
    /// gives the joint's translation `P.r3`; then `q = P.r3.xyz · (scale / 1024)` (w kept), `r = rows(+0xc0) · q`
    /// with `(0, 0, 0, 1)` as row 3 (0x2215e0), `out.xyz = r.xyz + position` (w = r.w). The rows are the ones the
    /// last `MobyBuildMatrix` left (an update that turns the moby this tick still sees last tick's). Without the
    /// class's joint list or animation data the joint is the moby origin. Native `f32`.
    pub fn joint_point(&self, id: MobyId, list: usize) -> [f32; 4] {
        let q = self.joint_local(id, list);
        let m = &self.table.mobys[id];
        let r = &m.rows;
        let v: [f32; 4] = std::array::from_fn(|l| r[0][l] * q[0] + r[1][l] * q[1] + r[2][l] * q[2] + if l == 3 { q[3] } else { 0.0 });
        [v[0] + m.position[0], v[1] + m.position[1], v[2] + m.position[2], v[3]]
    }

    /// The first half of [`World::joint_point`]: the joint's translation in the moby's frame, `P.r3.xyz · (scale /
    /// 1024)` (w kept), before the rows and the position (`fun_00210850` then `VecScale(scale / 1024)`, as the
    /// classes that keep a joint's offset call them).
    pub fn joint_local(&self, id: MobyId, list: usize) -> [f32; 4] {
        let m = &self.table.mobys[id];
        let chain = self.svc.joint_lists.get(&m.o_class).and_then(|l| l.get(list)).filter(|c| !c.is_empty());
        let t = match (self.classes.anim(m.o_class), chain) {
            (Some(class), Some(chain)) => {
                let snap = self.svc.snapshots.get(id).and_then(|s| s.as_ref());
                // With the moby's joint-modifier list (`MobyAnimEvalChain` 0x268ee8 applies +0x64; empty for most).
                rc_formats::moby_anim::evaluate_chains_posed(class, &m.anim, snap, &[chain.as_slice()], &[], &m.joint_mods)[0][3]
            }
            _ => [0.0, 0.0, 0.0, 1.0],
        };
        let k = m.scale * (1.0 / 1024.0);
        [t[0] * k, t[1] * k, t[2] * k, t[3]]
    }

    /// The points the live type-26 and type-55 particle records follow ([`Particles::joint_anchors`]), after the moby
    /// loop: per (moby, joint list) the moby's position (list −1) or the list's point (type 26: moved 0.4 toward the
    /// camera 0x167240, as `PartType26Update` does). A moby that is gone (state ≥ 0x80), or for type 55 has another
    /// class than the record's (+0x38), is not listed, so its records die on their next update.
    pub fn refresh_particle_anchors(&mut self) {
        use crate::particles::{rec, type19, type26};
        self.refresh_particle_mobys();
        let Some(p) = self.particles.as_deref() else { return };
        let keys: Vec<(usize, i16, bool, i16)> = p
            .pool
            .live()
            .filter_map(|(_, r)| {
                type26::anchor_of(r).map(|(m, j)| (m, j, true, -1)).or_else(|| type19::anchor55(r).map(|(m, j)| (m, j, false, rec::i16(r, 0x38))))
            })
            .collect();
        if keys.is_empty() {
            if let Some(p) = self.particles.as_deref_mut() { p.joint_anchors.clear(); }
            return;
        }
        let cam = fv(self.camera);
        let mut out = std::collections::HashMap::new();
        for (m, j, glow, class) in keys {
            let Some(mo) = self.table.mobys.get(m) else { continue };
            if mo.state >= 0x80 || (!glow && mo.o_class != class) { continue; }
            let mut q = if j < 0 { [mo.position[0], mo.position[1], mo.position[2]] } else { let t = self.joint_point(m, j as usize); [t[0], t[1], t[2]] };
            if glow && j >= 0 {
                let d = [cam[0] - q[0], cam[1] - q[1], cam[2] - q[2]];
                let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                if l != 0.0 { q = [q[0] + d[0] * 0.4 / l, q[1] + d[1] * 0.4 / l, q[2] + d[2] * 0.4 / l]; }
            }
            out.insert((m, j), q);
        }
        if let Some(p) = self.particles.as_deref_mut() { p.joint_anchors = out; }
    }

    /// `MobyAttachToJoint(moby, list, M)` 0x264508: the world matrix of joint list `list`'s last joint (its pose rows
    /// turned by the moby's rows, row 3 its point: `rc_formats::moby_anim::attach_matrix`). Without the class's joint
    /// list or animation data: the moby's rows and position.
    pub fn joint_matrix(&self, id: MobyId, list: usize) -> [[f32; 4]; 4] { self.joint_matrix_with(id, list, None) }

    /// [`World::joint_matrix`] on the animation class `anim` when given (a scene actor's: its class with the scene's
    /// streamed sequence in the slot its +0x52 / +0x53 name), else the class table's.
    pub fn joint_matrix_with(&self, id: MobyId, list: usize, anim: Option<&rc_formats::moby_anim::MobyAnimClass>) -> [[f32; 4]; 4] {
        let m = &self.table.mobys[id];
        let chain = self.svc.joint_lists.get(&m.o_class).and_then(|l| l.get(list)).filter(|c| !c.is_empty());
        let class = anim.or_else(|| self.classes.anim(m.o_class));
        let Some((class, chain)) = class.zip(chain) else {
            return [m.rows[0], m.rows[1], m.rows[2], [m.position[0], m.position[1], m.position[2], 1.0]];
        };
        let snap = self.svc.snapshots.get(id).and_then(|s| s.as_ref());
        // `MobyAttachToJoint` 0x24f728 marks the chain through `MobyMarkJointChain` as `0x24f7c8` ([`World::joint_point`])
        // does: with the moby's joint-modifier list +0x64 (Clank's lean moves his antenna glow 0x4b4 with the dot).
        let p = rc_formats::moby_anim::evaluate_chains_posed(class, &m.anim, snap, &[chain.as_slice()], &[], &m.joint_mods)[0];
        let rows = [m.rows[0], m.rows[1], m.rows[2]].map(|r| r.map(f32::to_bits));
        rc_formats::moby_anim::attach_matrix(&p, &rows, [m.position[0], m.position[1], m.position[2]], m.scale)
    }

    /// The mobys and joints the live particle records hold by pointer (`Particles::moby_refs`: types 67, 68, 74, 78),
    /// as their updates read them during `UpdateParts`, after the moby loop ([`crate::particles::Particles::moby_frames`],
    /// `joint_frames`). A moby past the table is left out (gone); the updates test its state and class themselves.
    pub fn refresh_particle_mobys(&mut self) {
        let Some(p) = self.particles.as_deref() else { return };
        let (mobys, joints) = p.moby_refs();
        let frames: std::collections::HashMap<usize, crate::particles::MobyFrame> = mobys
            .into_iter()
            .filter_map(|m| {
                let mo = self.table.mobys.get(m)?;
                let rows = [0, 1, 2].map(|k| [mo.rows[k][0], mo.rows[k][1], mo.rows[k][2]]);
                Some((m, crate::particles::MobyFrame { pos: [mo.position[0], mo.position[1], mo.position[2]], rows, state: mo.state, o_class: mo.o_class }))
            })
            .collect();
        let jf: std::collections::HashMap<(usize, u8), [[f32; 4]; 4]> =
            joints.into_iter().filter(|(m, _)| *m < self.table.mobys.len()).map(|(m, l)| ((m, l), self.joint_matrix(m, l as usize))).collect();
        let pvp: std::collections::HashMap<usize, [[f32; 3]; 3]> =
            frames.keys().map(|&m| (m, [0xd0, 0x1f0, 0xe0].map(|o| pvar_block_point(self.table, m, o)))).collect();
        let heads: std::collections::HashMap<usize, [f32; 3]> = frames.keys().map(|&m| (m, pvar_block_point(self.table, m, 0))).collect();
        if let Some(p) = self.particles.as_deref_mut() {
            p.moby_frames = frames;
            p.joint_frames = jf;
            p.pvar_points = pvp;
            p.pvar_heads = heads;
        }
    }

    // -----------------------------------------------------------------------------------------------
    // Particle spawners (the record writes and RNG draws of the game's spawn functions; the per-type
    // updates are the particle port's: an unported type kills itself on its first update and is counted)

    fn part(&mut self, ty: u8) -> Option<usize> {
        let p = self.particles.as_deref_mut()?;
        let r = p.create_part(ty);
        if r.is_none() { self.svc.fx.part_failed += 1; }
        *self.svc.fx.part_spawns.entry(ty).or_default() += 1;
        r
    }

    /// `PartType13Spawn(J, lo, hi, g, size, pos, s, rgba)` 0x280698 (crate break dust): 5 draws when a record
    /// is free (3 × `randf_sym(0, J)` jitter, `randi(2)` spin sign, `randf(lo, hi)`), none otherwise.
    #[allow(clippy::too_many_arguments)]
    pub fn part13(&mut self, j: Pf, lo: Pf, hi: Pf, g: Pf, size: Pf, pos: V4, s: i32, rgba: u32) {
        let Some(i) = self.part(13) else { return };
        let def = self.particles.as_ref().map(|p| p.def_first(13)).unwrap_or(0);
        let t10 = self.ticks(10);
        let jit: [Pf; 3] = std::array::from_fn(|_| pf(self.rng.randf_sym(0.0, fl(j))));
        let sign = if self.rng.randi(2) != 0 { s } else { -s };
        let w = Pf(self.rng.randf_bits(lo.0, hi.0));
        let r = &mut self.particles.as_deref_mut().unwrap().pool.recs[i];
        use crate::particles::rec;
        for k in 0..4 { rec::set_f(r, 0x10 + 4 * k, pos[k].0); }
        rec::set_u32(r, 4, rgba);
        r[9] = 0x44;
        r[3] = 0x48;
        rec::set_f(r, 0xc, size.0);
        r[1] = 0;
        r[8] = 0;
        r[2] = def;
        for k in 0..3 { let v = Pf(rec::f(r, 0x10 + 4 * k)) + jit[k]; rec::set_f(r, 0x10 + 4 * k, v.0); }
        rec::set_i16(r, 0xa, t10 as i16);
        rec::set_u32(r, 0x24, 0);
        rec::set_u32(r, 0x28, sign as u32);
        rec::set_u32(r, 0x34, rgba & 0xff_ffff);
        rec::set_f(r, 0x2c, w.0);
        rec::set_f(r, 0x30, g.0);
    }

    /// `PartType11Spawn(size, speed, pos, base, c1, c2, life, t1, t2, t3)` 0x27f8f8 (TNT sparks). The frame-load
    /// throttle draws `randi(3)` / `randi(2)` / `randi(1)` only when [`Services::frame_load`] exceeds 0.85 / 0.9
    /// / 1.0. With a record: `randi(100)`, a raw `rand()`, 3 × `randf(−1, 1)`.
    #[allow(clippy::too_many_arguments)]
    pub fn part11(&mut self, size: Pf, speed: Pf, pos: V4, base: V4, c1: u32, c2: u32, life: i32, t1: i32, t2: u8, t3: u8) {
        use crate::particles::type11;
        // The throttle reads the one frame-load global 0x15f5d0 / 0x15f5d4 (kept here and in the particles).
        let load = self.svc.frame_load.map(|x| x.0);
        let Some(p) = self.particles.as_deref_mut() else {
            // No particle system (tests): the throttle's draws only, as with a full pool.
            if life != 0 { type11::throttle(load, self.rng); }
            return;
        };
        p.frame_load = load;
        let (created, failed) = (p.stats.created, p.stats.create_failed);
        type11::spawn(p, self.rng, size.0, speed.0, pos.map(|x| x.0), base.map(|x| x.0), c1, c2, life, t1 as u8, t2, t3);
        // The spawn counters of `part` (a record was asked for when the throttle let it through).
        if (p.stats.created, p.stats.create_failed) != (created, failed) {
            if p.stats.create_failed != failed { self.svc.fx.part_failed += 1; }
            *self.svc.fx.part_spawns.entry(11).or_default() += 1;
        }
    }

    /// `PartType53Spawn(f12, f13, f14, pos, life, rgba, a3, t0, vel)` 0x287328 (bolt pickup sparkle): no draw
    /// for `a3` 0 / 1 (`randi(255)` otherwise).
    #[allow(clippy::too_many_arguments)]
    pub fn part53(&mut self, s12: Pf, s13: Pf, s14: Pf, pos: V4, life: i32, rgba: u32, a3: i32, t0: i8, vel: V4) {
        let Some(i) = self.part(0x35) else { return };
        let def = self.particles.as_ref().map(|p| p.def_first(0x35)).unwrap_or(0);
        let b8 = match a3 { 0 => 0, 1 => 0x20, _ => self.rng.randi(0xff) as u8 };
        let r = &mut self.particles.as_deref_mut().unwrap().pool.recs[i];
        crate::particles::type53::fill(r, def, s12.0, s13.0, s14.0, pos.map(|x| x.0), life, rgba, b8, t0, [vel[0].0, vel[1].0, vel[2].0]);
    }

    /// `PartType60Spawn(size, pos, vel, rgba, life, rot, attach)` 0x288588 (the glint; no draw: `rot` is the caller's
    /// draw): refused outside [2, 1021]³, else a record when one is free (counted as a type-60 spawn).
    #[allow(clippy::too_many_arguments)]
    pub fn part60(&mut self, size: f32, pos: [f32; 4], vel: [f32; 4], rgba: u32, life: u16, rot: u8, attach: i32) {
        let hero = crate::hero::physics::to_f32x3(self.hero.pos);
        let Some(p) = self.particles.as_deref_mut() else { return };
        let (created, failed) = (p.stats.created, p.stats.create_failed);
        p.hero = hero;
        crate::particles::type60::spawn(p, size, pos, vel, rgba, life, rot, attach);
        if (p.stats.created, p.stats.create_failed) != (created, failed) {
            if p.stats.create_failed != failed { self.svc.fx.part_failed += 1; }
            *self.svc.fx.part_spawns.entry(60).or_default() += 1;
        }
    }

    /// Level17 `0x26fb60(moby, pos, vel)`: a type-79 spark ([`crate::particles::type79::spawn79`]), riding `moby`
    /// when given (counted as a type-79 spawn). First consumer: the fleet lasers 99 (`units::fleet_laser`).
    pub fn part79(&mut self, moby: Option<MobyId>, pos: [f32; 4], vel: [f32; 3]) {
        let mp = moby.map(|m| { let p = self.table.mobys[m].position; (m, [p[0], p[1], p[2]]) });
        let Some(p) = self.particles.as_deref_mut() else { return };
        let (created, failed) = (p.stats.created, p.stats.create_failed);
        crate::particles::type79::spawn79(p, self.rng, mp, pos, vel);
        if (p.stats.created, p.stats.create_failed) != (created, failed) {
            if p.stats.create_failed != failed { self.svc.fx.part_failed += 1; }
            *self.svc.fx.part_spawns.entry(79).or_default() += 1;
        }
    }

    /// Level17 `0x26fdd0(pos, kind, k)`: a type-80 glow ([`crate::particles::type79::spawn80`]; counted as a type-80
    /// spawn when one is made or the pool refuses it). First consumer: the fleet turrets' bolts 1368
    /// (`units::fleet_turret`).
    pub fn part80(&mut self, pos: [f32; 4], kind: i32, k: i32) {
        let n = if kind != 0 { 5 } else { 15 };
        if n - k < 0 { return; }
        let Some(p) = self.particles.as_deref_mut() else { return };
        let (created, failed) = (p.stats.created, p.stats.create_failed);
        crate::particles::type79::spawn80(p, self.rng, pos, kind, k);
        if (p.stats.created, p.stats.create_failed) != (created, failed) {
            if p.stats.create_failed != failed { self.svc.fx.part_failed += 1; }
            *self.svc.fx.part_spawns.entry(80).or_default() += 1;
        }
    }

    // (The effect mobys' spawners, `DebrisSpawn` 0x2c5080 and `FlashSpawn` 0x2c20e0, live with their updates in
    // `classes::debris`.)

    // -----------------------------------------------------------------------------------------------
    // Paths (0x1b0930 splines)

    /// `ClampToPath(idx, a, b, out)` 0x276820: where the segment a → b (flattened) first crosses spline `idx`
    /// (as a 2-D polyline in the segment's frame), `lerp(a, b, t)` with the smallest crossing `t` in (0, 1),
    /// else `b`.
    pub fn clamp_to_path(&self, idx: i32, a: V4, b: V4) -> V4 { self.clamp_to_path_hit(idx, a, b).0 }

    /// [`World::clamp_to_path`] with the function's return value: whether a wall was crossed. The other
    /// overlays link the same function as their own copy (level00 `0x261b48`, level02 `0x263710`: identical
    /// code over their path table; `crate::path`), and its callers there read the flag.
    pub fn clamp_to_path_hit(&self, idx: i32, a: V4, b: V4) -> (V4, bool) {
        let Some(pts) = usize::try_from(idx).ok().and_then(|i| self.svc.splines.get(i)) else { return (b, false) };
        let mut d = ph::vsub(b, a);
        d[2] = Pf::ZERO;
        let l = ph::len3(d);
        d = ph::set_len3(d, Pf::ONE / l);
        let z = Pf::ZERO;
        let m = [d, [d[1], -d[0], z, z], [z, z, Pf::ONE, z]];
        let xf = |p: [u32; 4]| ph::mul_rows3(&m, ph::vsub(p.map(Pf), a));
        let mut best = Pf::ONE;
        let mut hit = false;
        let Some(&first) = pts.first() else { return (b, false) };
        let mut p0 = xf(first);
        for &pt in pts.iter().skip(1) {
            let p1 = xf(pt);
            if !(p0[3].is_zero() && p1[3].is_zero()) && (p1[1] * p0[1]) < Pf::ZERO {
                let t = ((p0[0] - p1[0]) / (p0[1] - p1[1])) * (-p1[1]) + p1[0];
                if Pf::ZERO < t && t < best {
                    best = t;
                    hit = true;
                }
            }
            p0 = p1;
        }
        (lerp3(a, b, best), hit)
    }

    /// The landing correction of `BoltBurst` 0x275988 / `CrateDropBolts` 0x2eb498 for a dropper with a path
    /// (`idx ≥ 0`): pick the fall time `t` from the quadratic (gravity `10.8·dt²`), clamp the landing point
    /// to the path, and aim the xy velocity at it (0.25 short).
    pub fn land_correct(&self, idx: i32, vel: &mut V4, base: V4, spawn: V4, z0: Pf) {
        let Some(s) = usize::try_from(idx).ok().and_then(|i| self.svc.splines.get(i)) else { return };
        let a = (DT2 * Pf::b(0x412c_cccd)) * Pf::b(0xbf00_0000);
        let first_z = s.first().map(|p| Pf(p[2])).unwrap_or(Pf::ZERO);
        let (n, r0) = quad(a, vel[2] - a, z0 - first_z);
        let mut t = Pf::b(0x42f0_0000);
        if Pf::ONE <= Pf::from_i32(n) && Pf::ZERO < r0 && r0 < t { t = r0; }
        let target = ph::vadd(scale3(*vel, t), base);
        let o = self.clamp_to_path(idx, base, target);
        let mut d = ph::vsub(o, spawn);
        d[2] = Pf::ZERO;
        let nrm = ph::set_len3(d, Pf::b(0x3e80_0000));
        let d = ph::vsub(d, nrm);
        let d = scale3(d, Pf::ONE / t);
        vel[0] = d[0];
        vel[1] = d[1];
    }
}

// ---------------------------------------------------------------------------------------------------
// Hit delivery outside the moby loop (the hero's attacks)

/// `FUN_0026e968(target, tmpl)`: the canonical hit delivery. Skipped when `target+0xa4` already holds a
/// record for it with a larger damage (`tmpl.damage < old`, `c.lt.s`); else the next slot (`0x1742d4`) gets
/// the record (+0x00 zero) and `target+0xa4` points at it. The wrench's line sweep and [`World::deliver_hit`].
pub fn deliver_hit_in(table: &mut MobyTable, hits: &mut HitLog, target: MobyId, t: &HitTemplate) {
    if let Some(r) = hits.current(table, target) {
        if t.damage < r.damage { return; }
    }
    hits.write(table, target, t, [Pf::ZERO; 4]);
}

/// The point (three f32) at `off` in moby `m`'s pvar block as the game's pointer `*(m + 0x78) + off` reads it: the
/// moby's own block when it is long enough; past a 0x80-byte block (`CreateMoby`'s, one per slot from 0x15ffe8, in slot
/// order) the read lands in the following slots' blocks [L: the blocks of the created mobys are contiguous]; zeros past
/// the table. Type 69's modes 1..3 read +0xd0 / +0x1f0 / +0xe0 (`crate::particles::type69`).
pub fn pvar_block_point(table: &MobyTable, m: MobyId, off: usize) -> [f32; 3] {
    let read = |id: usize, o: usize| -> f32 {
        table.mobys.get(id).and_then(|mo| mo.pvars.get(o..o + 4)).map_or(0.0, |b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let own = table.mobys.get(m).map_or(0, |mo| mo.pvars.len());
    [0, 4, 8].map(|k| {
        let o = off + k;
        if o + 4 <= own || own != 0x80 { read(m, o) } else { read(m + o / 0x80, o % 0x80) }
    })
}

/// `coll_sphere_mobys(r, centre, flags, ignore, tmpl)` 0x214468 ([`coll_sphere_mobys`]): the mobys the sphere
/// touches, in grid order; with a template, every listed moby with `mode & 0x4000` gets a record
/// (+0x00 = (0,0,0,1), +0x38 = 0) — unless its current record (`+0xa4`, for this moby) has a larger damage
/// (the kernel compares the float bits as integers: `old − new > 0` skips).
#[allow(clippy::too_many_arguments)]
pub fn sphere_mobys_in(table: &mut MobyTable, svc: &mut Services, classes: &dyn ClassData, r: Pf, centre: V4, flags: u32, ignore: Option<MobyId>, tmpl: Option<&HitTemplate>) -> Vec<MobyId> {
    let hits = {
        let src = svc.scene_parts(table, classes);
        let sc = svc.scene(&src);
        coll_sphere_mobys(&sc, ph::to_f32x3(centre), fl(r), QueryFlags(flags), ignore)
    };
    if let Some(t) = tmpl {
        for &id in &hits {
            if table.mobys[id].mode & 0x4000 == 0 { continue; }
            if let Some(old) = svc.hits.current(table, id) {
                if (old.damage.0 as i32).wrapping_sub(t.damage.0 as i32) > 0 { continue; }
            }
            svc.hits.write(table, id, t, [Pf::ZERO, Pf::ZERO, Pf::ZERO, Pf::ONE]);
        }
    }
    hits
}

/// `CollLine_Fix(a, b, flags, ignore, tmpl)` with a hit template (0x212888..0x212904): on a moby hit, a moby
/// with `mode & 0x4000` gets a record (+0x00 = the hit point, +0x38 = the primitive index or −1 for a
/// triangle) unless its current record has a larger damage (bits compared as integers, as the sphere list).
#[allow(clippy::too_many_arguments)]
pub fn line_hit_in(table: &mut MobyTable, svc: &mut Services, classes: &dyn ClassData, coll: Option<&Collision>, a: V4, b: V4, flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<CollOutput> {
    let h = {
        let src = svc.scene_parts(table, classes);
        let sc = svc.scene(&src);
        coll_line_m(coll.unwrap_or(no_mesh()), Some(&sc), ph::to_f32x3(a), ph::to_f32x3(b), QueryFlags(flags), ignore)?
    };
    if let Some(id) = h.moby {
        if table.mobys[id].mode & 0x4000 != 0 {
            let skip = svc.hits.current(table, id).is_some_and(|old| (old.damage.0 as i32).wrapping_sub(tmpl.damage.0 as i32) > 0);
            if !skip {
                let p = [pf(h.point[0]), pf(h.point[1]), pf(h.point[2]), Pf::ZERO];
                svc.hits.write_prim(table, id, tmpl, p, h.primitive.map_or(-1, |i| i as i32));
            }
        }
    }
    Some(h)
}

/// The hero's [`crate::hero::items::HitSink`] on the moby system: spheres through [`sphere_mobys_in`], lines
/// through [`line_hit_in`] (the kernels write the hit records themselves).
pub struct ServiceHits<'a> {
    pub svc: &'a mut Services,
    pub classes: &'a dyn ClassData,
    pub coll: Option<&'a Collision>,
}

impl crate::hero::items::HitSink for ServiceHits<'_> {
    fn sphere(&mut self, table: &mut MobyTable, r: Pf, centre: V4, flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<MobyId> {
        sphere_mobys_in(table, self.svc, self.classes, r, centre, flags, ignore, Some(tmpl)).first().copied()
    }

    fn line(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<Option<MobyId>> {
        line_hit_in(table, self.svc, self.classes, self.coll, a, b, flags, ignore, tmpl).map(|h| h.moby)
    }

    fn probe(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<MobyId>) -> Option<Option<[f32; 3]>> {
        let src = self.svc.scene_parts(table, self.classes);
        let sc = self.svc.scene(&src);
        Some(coll_line_m(self.coll.unwrap_or(no_mesh()), Some(&sc), ph::to_f32x3(a), ph::to_f32x3(b), QueryFlags(flags), ignore).map(|h| h.point))
    }

    fn probe_moby(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<MobyId>) -> Option<Option<crate::hero::items::Probe>> {
        let src = self.svc.scene_parts(table, self.classes);
        let sc = self.svc.scene(&src);
        let h = coll_line_m(self.coll.unwrap_or(no_mesh()), Some(&sc), ph::to_f32x3(a), ph::to_f32x3(b), QueryFlags(flags), ignore);
        Some(h.map(|h| crate::hero::items::Probe { moby: h.moby, point: h.point, normal: h.normal, surface: h.surface_id() }))
    }

    fn class_type(&self, o_class: i16) -> Option<u8> { self.classes.info(o_class).map(|i| i.ty) }

    fn light_alloc(&mut self, l: crate::point_lights::PointLight) -> i32 {
        let load = f32::from_bits(self.svc.frame_load[1].0);
        self.svc.point_lights.alloc(l, load).map_or(-1, |i| i as i32)
    }

    fn light_get(&mut self, slot: i32) -> Option<crate::point_lights::PointLight> { *self.svc.point_lights.slots.get(usize::try_from(slot).ok()?)? }

    fn light_set(&mut self, slot: i32, l: crate::point_lights::PointLight) {
        if let Ok(i) = usize::try_from(slot) { self.svc.point_lights.set(i, l); }
    }

    fn light_free(&mut self, slot: i32) {
        if let Ok(i) = usize::try_from(slot) { self.svc.point_lights.free(i); }
    }
}

/// A copy of what the hero's and the camera's collision queries read of the moby system (the table's collision
/// fields, the grid, the blobs, the pose cache; [`crate::tick::MobySystem::scene`]): the hero runs after the
/// moby loop with its own moby borrowed mutably, so it cannot see the live table.
pub struct HeroMobys {
    pub mobys: Vec<crate::collision_query::CollMoby>,
    pub anim: Vec<rc_formats::moby_anim::AnimState>,
    pub o_class: Vec<i16>,
    pub snapshots: Vec<Option<MobyFrame>>,
    pub classes: Arc<dyn ClassData + Send + Sync>,
}

impl crate::collision_query::MobySource for HeroMobys {
    fn moby(&self, id: usize) -> Option<crate::collision_query::CollMoby> { self.mobys.get(id).copied() }
    fn joints(&self, id: usize, count: usize) -> Vec<[u32; 4]> {
        let (Some(a), Some(&oc)) = (self.anim.get(id), self.o_class.get(id)) else { return vec![[0; 4]; count] };
        let snap = self.snapshots.get(id).and_then(Option::as_ref);
        crate::collision_query::pose_joints(self.classes.anim(oc), a, snap, count)
    }
}

impl Services {
    /// The scene over `table` now (see [`HeroMobys`]); `hero` is only stored as the scene's `ignore`.
    pub fn hero_scene(&self, table: &MobyTable, classes: Arc<dyn ClassData + Send + Sync>, hero: Option<MobyId>) -> crate::collision_query::OwnedScene {
        let n = table.mobys.len();
        let mobys = HeroMobys {
            mobys: table.mobys.iter().map(crate::collision_query::CollMoby::of).collect(),
            anim: table.mobys.iter().map(|m| m.anim).collect(),
            o_class: table.mobys.iter().map(|m| m.o_class).collect(),
            snapshots: if self.coll_classes.values().any(|c| c.joint_counts != [0, 0]) { self.snapshots.iter().take(n).cloned().collect() } else { Vec::new() },
            classes,
        };
        crate::collision_query::OwnedScene { mobys: Box::new(mobys), grid: self.grid.clone(), classes: self.coll_classes.clone(), cache: self.pose_cache.clone(), ignore: hero }
    }
}

/// [`crate::tick::MobySystem`] over the moby loop's services, shared with the moby hook (and the hero's hit
/// sink) through a `RefCell`, with the class data for the poses and the bounding spheres.
pub struct SharedServices<'a, 'b> {
    pub svc: &'a std::cell::RefCell<&'b mut Services>,
    pub classes: Arc<dyn ClassData + Send + Sync>,
}

impl crate::tick::MobySystem for SharedServices<'_, '_> {
    fn scene(&mut self, table: &MobyTable) -> Option<crate::collision_query::OwnedScene> {
        Some(self.svc.borrow().hero_scene(table, self.classes.clone(), None))
    }
    fn build_matrix(&mut self, table: &mut MobyTable, id: MobyId) { self.svc.borrow_mut().build_matrix_in(table, &*self.classes, id); }
    fn deliver_hit(&mut self, table: &mut MobyTable, target: MobyId, tmpl: &HitTemplate) {
        let mut s = self.svc.borrow_mut();
        deliver_hit_in(table, &mut s.hits, target, tmpl);
    }
    fn hit_message(&self, table: &MobyTable, target: MobyId) -> Option<HitRecord> { self.svc.borrow().hits.current(table, target).copied() }
    fn take_hero_writes(&mut self) -> Option<HeroFields> { self.svc.borrow_mut().take_hero_writes() }
    fn spline(&self, i: usize) -> Option<Vec<[f32; 4]>> {
        self.svc.borrow().splines.get(i).map(|v| v.iter().map(|p| p.map(f32::from_bits)).collect())
    }
    fn take_camera_shakes(&mut self) -> Vec<crate::follow_camera::ShakeRequest> { std::mem::take(&mut self.svc.borrow_mut().camera_shakes) }
    fn take_cinematic(&mut self) -> Vec<crate::cinematic::CinematicCall> { crate::cinematic::take_calls(&mut self.svc.borrow_mut()) }
    fn game_mode(&self) -> i32 { self.svc.borrow().game_mode }
    fn board_world(&self, table: &MobyTable, board: MobyId) -> Option<crate::hero::hoverboard::BoardWorld> {
        crate::moby_update::classes::units::hoverboard::world(&self.svc.borrow(), table, board)
    }
    fn queue_board(&mut self, cmds: Vec<crate::hero::hoverboard::BoardCmd>) { self.svc.borrow_mut().board.cmds.extend(cmds); }
    fn fade(&self) -> f32 { self.svc.borrow().cinematic.fade }
    fn set_fade(&mut self, v: f32) { self.svc.borrow_mut().cinematic.fade = v; }
    fn view_tan(&self) -> f32 { self.svc.borrow().view_tan_x }
    fn set_view_tan(&mut self, v: f32) { self.svc.borrow_mut().view_tan_x = v; }
    fn set_letterbox(&mut self, on: bool) { self.svc.borrow_mut().creatures.cutscene = on; }
    fn queue_hero_state(&mut self, state: i32) { self.svc.borrow_mut().cinematic.calls.push(crate::cinematic::CinematicCall::HeroState { state, play: true }); }
    fn queue_race(&mut self, out: crate::follow_camera::race::RaceOut) { self.svc.borrow_mut().camera_race.push(out); }
    fn run_list(&self, table: &MobyTable, camera: V4) -> Option<Vec<MobyId>> {
        Some(crate::moby_update::scheduler::build_active_list(table, camera, &self.svc.borrow().groups).0)
    }
    fn volumes(&self) -> Option<Arc<rc_formats::volumes::Volumes>> { Some(self.svc.borrow().volumes.clone()) }
    fn water(&self) -> Option<&dyn crate::hero::swim::WaterQuery> { Some(self) }
    fn group(&self, g: i8) -> Vec<MobyId> {
        if g < 0 { return Vec::new(); }
        self.svc.borrow().groups.lists.get(g as usize).and_then(|l| l.clone()).map(|l| l.into_iter().map(|m| m as MobyId).collect()).unwrap_or_default()
    }
    fn create_moby(&mut self, table: &mut MobyTable, o_class: i16, counter: u64) -> Option<MobyId> {
        // A class the level does not load has no slot (0x198040[o_class]): no moby [L].
        let info = self.classes.info(o_class)?;
        let id = table.create(o_class, Some(&info), counter)?;
        crate::moby_update::anim_sound::init(&mut table.mobys[id], self.classes.anim(o_class));
        let mut s = self.svc.borrow_mut();
        if s.snapshots.len() <= id { s.snapshots.resize(id + 1, None); }
        s.snapshots[id] = None;
        Some(id)
    }
    fn delete_moby(&mut self, table: &mut MobyTable, id: MobyId, counter: u64) {
        table.delete(id, counter);
        Arc::make_mut(&mut self.svc.borrow_mut().grid).remove(&mut table.mobys[id]);
    }
}

/// `SetWaterLevel` 0x26ed38 over the level's water (`Services::water`, borrowed per query).
impl crate::hero::swim::WaterQuery for SharedServices<'_, '_> {
    fn water_height(&self, p: [f32; 3]) -> Option<f32> { self.svc.borrow().water.water_height(p) }
}

/// `Quad(a, b, c, &r0, &r1)` 0x26e520: roots of `a·t² + b·t + c`: `(count, larger root)`.
pub fn quad(a: Pf, b: Pf, c: Pf) -> (i32, Pf) {
    let disc = b * b - (a * Pf::b(0x4080_0000)) * c;
    let two_a = a + a;
    if disc == Pf::ZERO { return (1, (-b) / two_a); }
    let s = Pf::ZERO + disc.abs().sqrt();
    let r0 = ((-b) + s) / two_a;
    let r1 = ((-b) - s) / two_a;
    let r = if r0 < r1 { r1 } else { r0 };
    (if Pf::ZERO < disc { 2 } else { 0 }, r)
}

/// `FastNormalizeAngle` 0x222088: wrap into [−π, π) by repeated ±2π (`(x − π) − π`).
pub fn normalize_angle(a: Pf) -> Pf {
    let p = Pf::b(0x4049_0fdb);
    let mut x = a;
    if !(x < p) { loop { x = (x - p) - p; if x < p { break; } } }
    if x < -p { loop { x = (x + p) + p; if !(x < -p) { break; } } }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::type11;

    /// `SetDeathBits` writes the persistent bits straight into the save's chunk 3005 (0x14c190 + L·0x100), and the load's
    /// spawn test / `CollectBolt` the spawner slots into chunk 3006: the save and the next arrival see them.
    #[test]
    fn sync_save_writes_the_death_bits_and_bolt_slots_into_the_save() {
        let mut gs = crate::game_state::GameState::zeroed(rc_formats::save_game::ChunkTables { global: vec![], level: vec![] });
        let mut svc = Services::new();
        svc.level = 3;
        svc.save.death.insert((3, 129));
        svc.save.death.insert((3, 9));
        svc.save.death.insert((5, 1));
        svc.counters.spawner_first = vec![0; 64];
        svc.counters.spawner_first[63] = 41;
        svc.counters.spawner_bolts.insert((3, 63), 7);
        svc.sync_save(&mut gs);
        assert_eq!(gs.levels[3].killed[129 >> 3], 1 << (129 & 7));
        assert_eq!(gs.levels[3].killed[1], 1 << 1);
        assert_eq!(gs.levels[5].killed[0], 1 << 1, "another level's bits go to its own chunk");
        assert_eq!((gs.levels[3].bolt_drops[63].first, gs.levels[3].bolt_drops[63].collected), (41, 7));
        assert_eq!((gs.levels[3].bolt_drops[0].first, gs.levels[3].bolt_drops[0].collected), (0, 0));
    }

    /// `0x2721f0` inverts `0x221980` (R = Rz·Ry·Rx) for |y| < π/2: x, y and z come back with their signs (the
    /// disassembly stores the last `FastArcTan` as is: `neg.s f1, f0` at 0x2722d8 goes to a dead stack word).
    #[test]
    fn rows_euler_inverts_euler_rows() {
        for e in [[0.3f32, 0.0, 0.0], [-0.7, 0.4, 1.2], [0.2, -1.3, -2.9], [1.1, 1.4, 0.5]] {
            let v = [Pf::f(e[0]), Pf::f(e[1]), Pf::f(e[2]), Pf::ZERO];
            let back = rows_euler(&euler_rows(v));
            for k in 0..3 { assert!((back[k].to_f32() - e[k]).abs() < 1e-4, "{e:?} -> {back:?}"); }
        }
    }

    /// The bolts' settle reads a tumbled bolt's rows back as a quaternion (`0x26ee30(0x2721f0(rows))`): it is the
    /// same rotation (`quat_rows` of it gives the rows back), so the settle starts where the tumble left the bolt.
    #[test]
    fn rows_euler_quat_round_trip() {
        for e in [[0.3f32, 0.0, 0.0], [-0.7, 0.4, 1.2], [0.2, -1.3, -2.9], [1.1, 1.4, 0.5]] {
            let r = euler_rows([Pf::f(e[0]), Pf::f(e[1]), Pf::f(e[2]), Pf::ZERO]);
            let q = quat_rows(euler_quat(rows_euler(&r)));
            for i in 0..3 {
                for k in 0..3 { assert!((q[i][k].to_f32() - r[i][k].to_f32()).abs() < 1e-3, "{e:?}: {q:?} vs {r:?}"); }
            }
        }
    }

    /// `World::part11` is `type11::spawn` (plus the spawn counters): same draws, same record, for each frame load,
    /// with and without a free record, and throttle-only without a particle system.
    #[test]
    fn part11_is_type11_spawn_draw_for_draw() {
        let hero = Hero::new();
        let classes = ClassTable::default();
        let (size, speed) = (Pf::b(0x48c3_5000), Pf::b(0x3e4c_cccd));
        let pos: V4 = [Pf::f(150.0), Pf::f(152.5), Pf::f(40.25), Pf::ZERO];
        let base: V4 = [Pf::f(0.01), Pf::f(-0.02), Pf::f(0.05), Pf::ZERO];
        for load in [0.0f32, 0.86, 0.95, 1.5] {
            let load = [Pf::f(load), Pf::ZERO];
            let mut seed = Rng::new();
            seed.srand(crate::rng::LEVEL_SEED);
            let (mut ra, mut rb) = (seed, seed);
            let mut pa = Particles::new(None, Vec::new());
            let mut pb = Particles::new(None, Vec::new());
            // Fill the pool but one record for the last spawns (pool full → no record, no further draws).
            for _ in 0..0x7f8 { pa.create_part(1); pb.create_part(1); }
            pb.frame_load = load.map(|x| x.0);
            let mut table = MobyTable::new(Vec::new(), 1);
            let mut svc = Services::new();
            svc.frame_load = load;
            {
                let mut w = World::new(&mut table, &hero, &mut ra, &classes, &mut svc, 0);
                w.particles = Some(&mut pa);
                for k in 0..12 { w.part11(size, speed, pos, base, 0x80ff_8040, 0x0010_2030, 20 + k, 6, 0, 0); }
                w.part11(size, speed, pos, base, 0, 0, 0, 0, 0, 0);
            }
            for k in 0..12 {
                type11::spawn(&mut pb, &mut rb, size.0, speed.0, pos.map(|x| x.0), base.map(|x| x.0), 0x80ff_8040, 0x0010_2030, 20 + k, 6, 0, 0);
            }
            type11::spawn(&mut pb, &mut rb, size.0, speed.0, pos.map(|x| x.0), base.map(|x| x.0), 0, 0, 0, 0, 0, 0);
            assert_eq!(ra.state, rb.state, "rng after the spawns, load {load:?}");
            assert_ne!(ra.state, seed.state);
            assert!(pa.pool.recs.iter().zip(pb.pool.recs.iter()).all(|(a, b)| a == b), "records, load {load:?}");
            assert_eq!(pa.pool.count, pb.pool.count);
            let spawns = svc.fx.part_spawns.get(&11).copied().unwrap_or(0);
            assert_eq!(spawns, pa.stats.created + pa.stats.create_failed - 0x7f8);
            assert_eq!(svc.fx.part_failed, pa.stats.create_failed);
            // No particle system: the throttle's draws only.
            let (mut rc, mut rd) = (seed, seed);
            {
                let mut w = World::new(&mut table, &hero, &mut rc, &classes, &mut svc, 0);
                for _ in 0..3 { w.part11(size, speed, pos, base, 1, 2, 20, 6, 0, 0); }
            }
            for _ in 0..3 { type11::throttle(load.map(|x| x.0), &mut rd); }
            assert_eq!(rc.state, rd.state, "throttle-only draws, load {load:?}");
        }
    }
}
