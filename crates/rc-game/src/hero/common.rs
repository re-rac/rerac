//! Helpers the hero's state groups share (docs/plan/hero_states.md "Restructure"): the jump choosers the
//! ground, walk, fall and jump transitions call (`StickSector` 0x23ceb0, `FlipWindow` 0x23ce50, `TryJump`
//! 0x241968, `CrouchJump` 0x242420, `JumpOrRunningJump` 0x246690, `StepDownAhead` 0x2426d0), the landing ETA
//! 0x248268 with its root finder, the gravity step 0x248b68, the idle sequence and the yaw history.
//!
//! Every group module (`ground`, `walk`, `air`, `jump`, `melee`, `swim` and the package stubs) may call these;
//! a package that needs a change here asks for it (this file has no package owner).
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use super::physics::*;
use super::states::Ctx;
use super::{state, Hero};
use crate::pad::{button, fast_diff_rots};
use crate::ps2v::Pf;

pub(super) const HALF_PI: Pf = Pf::b(0x3fc9_0fdb);
pub(super) const QUARTER_PI: Pf = Pf::b(0x3f49_0fdb);

/// A blend over `n` ticks (`(float)ticks(n)`).
pub(super) fn blend(n: i32) -> Pf { Pf::from_i32(ticks(n)) }

impl Hero {
    /// Idle sequence `0x226f10(0)`: 0, 0x54 at health 1, 0x6e on a slippery floor; 0 in the other bodies (L00 0x205000:
    /// Clank, Giant Clank and the disguise have their own sequence 0).
    pub fn idle_seq(&self) -> u8 {
        if self.mode != 0 { return 0; }
        if self.health == 1 { return 0x54; }
        // On a slippery floor (0x140632, level00's 0x205000): 0x6e (super::surface).
        if super::surface::slippery(self) { return 0x6e; }
        0
    }

    /// Yaw `n` ticks ago from the 32-entry ring (`0x22df98`).
    pub(super) fn yaw_ago(&self, n: i32) -> Pf {
        let m = n.min(self.yaw_hist_count);
        self.yaw_hist[((self.yaw_hist_idx - m) & 31) as usize]
    }

    /// Gravity step 0x248b68 (mode 0): `dst.z = src.z − amount`.
    pub(super) fn gravity_from(&mut self, src_z: Pf, amount: Pf) { self.vel[2] = src_z - amount; }

    // --------------------------------------------------------------------------------------------
    // Input helpers (the jump choosers).

    /// `StickSector()` (0x23ceb0): 3 back, 1 right, 0 left, 2 forward / neutral (45° sectors relative to
    /// the facing). Runs StickTarget(1, 0).
    pub(super) fn stick_sector(&mut self, c: &mut Ctx) -> i32 {
        self.stick_target(c.env, Pf::ONE);
        let d = fast_subtract_rotations(self.target_yaw, self.rot[2]);
        if fast_diff_rots(d, PI) < QUARTER_PI { return 3; }
        if fast_diff_rots(d, -HALF_PI) < QUARTER_PI { return 1; }
        if fast_diff_rots(d, HALF_PI) < QUARTER_PI { return 0; }
        2
    }

    /// `FlipWindow()` (0x23ce50): 0 for a forward stick, else 12 ticks.
    pub(super) fn flip_window(&mut self, c: &mut Ctx) -> i32 { if self.stick_sector(c) == 2 { 0 } else { ticks(12) } }

    /// `TryJump(n, noPlainJump)` (0x241968): ✕ within `n` ticks → flip 0xb (crouch + side/back stick +
    /// a direction press near the ✕) or jump 7.
    pub(super) fn try_jump(&mut self, c: &mut Ctx, n: i32, no_plain: bool) -> bool {
        if c.env.pad.pressed_within(button::CROSS, n).is_none() { return false; }
        // Port-only: the Port Options strafe's flip (super::strafe).
        if super::strafe::flip(self, c) { return true; }
        let w = self.flip_window(c);
        if c.env.pad.held & button::CROUCH != 0 && w > 0 && c.env.pad.combo(button::CROSS, 0x1f000, n + w, w) {
            self.set_state(c, 0xb, true);
            return true;
        }
        if no_plain { return false; }
        self.set_state(c, 7, true);
        true
    }

    /// `CrouchJump()` (0x242420): flip, else (after 6 ticks of crouch) a Clank-pack jump ([`super::packs::crouch_jump`]) or a plain
    /// jump 7.
    pub(super) fn crouch_jump(&mut self, c: &mut Ctx) -> bool {
        if self.state == state::DOUBLE_JUMP { return false; }
        let w = self.flip_window(c);
        if c.env.pad.held & button::CROUCH != 0 && w > 0 && c.env.pad.combo(button::CROSS, 0x1f000, ticks(5) + w, w) {
            self.set_state(c, 0xb, true);
            return true;
        }
        if !(ticks(6) < self.timer) { return false; }
        if c.env.pad.pressed_within(button::CROSS, ticks(9)).is_none() { return false; }
        // The Clank-pack crouch jumps (Heli 10 / 0xf, Thruster 0x10 / 0xd): super::packs.
        if super::packs::crouch_jump(self, c) { return true; }
        self.set_state(c, 7, true);
        true
    }

    /// `JumpOrRunningJump()` (0x246690): 9 when running straight and fast, else 7.
    pub(super) fn jump_or_running_jump(&mut self, c: &mut Ctx) {
        let run = SPEED_TABLE[0][3] < self.target_speed
            && fast_diff_rots(self.target_yaw, self.rot[2]) < Pf::b(0x3f86_0a92)
            && (SPEED_TABLE[1][2] * DT) * Pf::b(0x3f59_999a) < self.eff_len_xy;
        self.set_state(c, if run { 9 } else { 7 }, true);
    }

    /// `StepDownAhead()` (0x2426d0): walkable ground within (−1, 0.37) below a point 0.75·r ahead.
    pub(super) fn step_down_ahead(&self, c: &Ctx) -> bool {
        let r = self.cap_radius;
        if self.eff_len_xy < r / Pf::from_i32(ticks(20)) { return false; }
        if Pf::b(0x3f06_0a92) < fast_diff_rots(crate::pad::fast_arctan(self.disp[0], self.disp[1]), self.rot[2]) { return false; }
        let mut p = self.pos;
        p[0] = p[0] + (fast_cos(self.rot[2]) * r) * Pf::b(0x3f40_0000);
        p[1] = p[1] + (fast_sin(self.rot[2]) * r) * Pf::b(0x3f40_0000);
        let mut a = p;
        a[2] = p[2] + Pf::b(0x3ef0_a3d7);
        let mut b = p;
        b[2] = p[2] - Pf::ONE;
        let Some(o) = c.env.line(a, b, 2) else { return false };
        let d = Pf::f(o.point[2]) - self.pos[2];
        d < Pf::b(0x3ebd_70a4) && Pf::b(0xbf80_0000) < d
    }

    /// `LandEta(maxT, g, h)` 0x248268 (mode 0): ticks until the feet reach height `h` under constant gravity
    /// `g` (h < 0: the ground where the parabola ends after `maxT` ticks, found with a line, flags 0x24 on level01:
    /// [`super::surface::pass_flags`]).
    /// Returns `maxT` when there is no such ground or root.
    pub(super) fn land_eta(&self, c: &Ctx, max_t: Pf, g: Pf, mut h: Pf) -> Pf {
        if h < Pf::ZERO {
            let mut e = vscale(self.vel, max_t);
            e[2] = e[2] - (((max_t * max_t) + max_t) * Pf::b(0x3f00_0000)) * g;
            e = vadd(e, self.pos);
            clip_to_world(self.pos, &mut e);
            if let Some(o) = c.env.line(self.pos, e, super::surface::pass_flags(self.idle.level).bits()) {
                if slope_of(from_f32x3(o.normal)) <= SLOPE_MAX { h = Pf::f(o.point[2]); }
            }
        }
        if !(Pf::ZERO < h) { return max_t; }
        let a = g * Pf::b(0xbf00_0000);
        let (n, r1) = quadratic(a, self.vel[2] - a, self.pos[2] - h);
        if n > 0 && Pf::ZERO < r1 { r1 } else { max_t }
    }
}

/// `0x26e520(a, b, c)`: roots of `a·t² + b·t + c`; returns (count, larger root). D = b² − (4a)·c; D = 0: one
/// root −b/(a+a); otherwise both (as if |D|), count 2 when D > 0 else 0.
pub(super) fn quadratic(a: Pf, b: Pf, c: Pf) -> (i32, Pf) {
    let d = (b * b) - ((a * Pf::b(0x4080_0000)) * c);
    if d == Pf::ZERO { return (1, (-b) / (a + a)); }
    let s = Pf::ZERO + d.abs().sqrt();
    let r1 = (-b + s) / (a + a);
    let r2 = (-b - s) / (a + a);
    let hi = if r1 < r2 { r2 } else { r1 };
    (if Pf::ZERO < d { 2 } else { 0 }, hi)
}

/// `0x2720f8(start, end)`: pull `end` back along the segment so no coordinate is below 0 (clipped at 0.1).
pub(super) fn clip_to_world(start: V4, end: &mut V4) {
    for k in 0..3 {
        if end[k] < Pf::ZERO {
            let d = vsub(*end, start);
            let f = ((start[k] - Pf::b(0x3dcc_cccd)) / d[k]).abs();
            *end = vadd(start, vscale(d, f));
        }
    }
}

/// `ClampLen(max, v)` 0x2745f0: `v = normalize(v)·max` when `max < |v|` (3-D).
pub fn clamp_len_2745f0(v: &mut V4, max: Pf) {
    if max < len3(*v) { *v = set_len3(*v, max); }
}
