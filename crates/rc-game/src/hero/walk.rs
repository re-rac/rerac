//! Walk / run 2 and wading 0x73 (group 1): the SetState entry (0x23cf98 case 2 / 0x3f / 0x73), physics
//! (0x2370b8, case 0x23a2e4), transitions (0x242930 case 2 / 0x2f / 0x73 / 0x7e) and the walk / run anim
//! selector 0x241a08. Spec: docs/plan/player_controller.md §3–§4, §14 (wading); docs/plan/hero_states.md.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use super::common::*;
use super::physics::*;
use super::states::Ctx;
use super::Hero;
use crate::pad::{button, fast_diff_rots};
use crate::ps2v::Pf;

/// Walk ↔ run frame phase offsets `[0x15f6c8]` indexed by `new + 2·old`.
const PHASE_OFF: [i32; 4] = [0, 1, 5, 0];

impl Hero {
    /// SetState entry of 2 / 0x73 (0x3f shares the case in the game; it is [`super::boots`]'s).
    pub(super) fn walk_entry(&mut self, c: &mut Ctx, id: i32, play: bool) -> Option<bool> {
        self.group = 1;
        self.f15d4 = 0;
        // No `0x242858` here (SetState case 2 does not call it; the stop 3 and the walk's slow stop do): with a
        // weapon that keeps the arm raised out, Ratchet walks off from its stance, the arm layers coming up through
        // `0x22eca0` (super::weapons::arm_on_state_change).
        if self.prev_group != 4 && self.prev_prev_group != 4 { self.speed = len2(self.eff); }
        if DT * Pf::b(0x40e0_0000) < self.speed { self.speed = DT * Pf::b(0x40e0_0000); }
        self.stick_target(c.env, Pf::ONE);
        self.idle.blink_period = 0x68;
        self.sharp_turn = 0;
        if HALF_PI < fast_diff_rots(self.rot[2], self.target_yaw)
            && (self.prev_group == 7 || self.prev_state == 3 || self.prev_prev_state == 3)
        {
            self.speed = Pf::ZERO;
        }
        if self.f063a != 0 && self.f65c == 0 { return Some(self.set_state(c, 0x79, true) && false); }
        // A slippery floor (0x140632) or 0x2f itself: the slide's branch (level00; super::surface).
        if let Some(r) = super::surface::slippery_walk_entry(self, c, id) { return r; }
        if self.f13f9 != 0 || id == 0x73 {
            // Wading 0x73 (the walk in 0.25..0.85 of water; its anim is set even without `play`).
            self.state = 0x73;
            self.set_anim(c.anim, c.rng, blend(8), 0x60, 0);
            return None;
        }
        if self.f658 == 1 || id == 0x3f {
            // The Magneboots walk 0x3f (super::boots; its anim is set even without `play`).
            self.state = 0x3f;
            self.set_anim(c.anim, c.rng, blend(8), 0x5b, 0);
            return None;
        }
        self.substate = 0;
        self.turn_to_target = 0;
        self.land_run_blend = 0;
        let mut b = ticks(8);
        if self.health == 1 { b = ticks(12); }
        if self.prev_prev_state == 0x23 { b = ticks(15); }
        if self.prev_prev_state == 6 || self.prev_prev_state == 0x2d { b = ticks(18); }
        if self.prev_prev_group == 4 && ticks(5) < self.jump.landed { b = ticks(13); }
        if play { self.set_anim(c.anim, c.rng, Pf::from_i32(b), 3, 8); }
        None
    }

    /// Walk / run 2 (0x23a2e4).
    pub(super) fn phys_walk(&mut self, env: &Env) {
        // Port-only: the Port Options strafe (super::strafe) has no sharp turn and no turn toward the stick.
        if self.strafe { self.sharp_turn = 0; self.turn_to_target = 0; }
        if self.eff_len_xy < DT * Pf::b(0x3e99_999a) && Pf::b(0x3f59_999a) < self.stick_mag { self.turn_to_target = 1; }
        if self.yaw_residual.abs() < Pf::b(0x3d56_7750) || self.f13f8 != 0 || self.substate == 1 { self.turn_to_target = 0; }
        if self.sharp_turn != 0 && (self.yaw_residual.abs() < Pf::b(0x3d8e_fa35) || self.f13f8 != 0) { self.sharp_turn = 0; }
        self.stick_target_table(env);
        if Pf::ZERO < self.push_strength {
            let k = Pf::b(0x3f33_3333);
            self.drag(k, Pf::ZERO);
            let t = self.target_speed + push_speed_along(self.push, self.rot[2]);
            self.target_speed = t;
            let floor = DT * k;
            if t < floor { self.target_speed = floor; }
            self.push = V0;
        }
        let mut f = Pf::ONE;
        if self.strafe {
            super::strafe::face_camera(self, env);
        } else if self.substate == 1 && self.sharp_turn == 0 {
            self.turn_to(SCALE64 * Pf::b(0x3c03_126f), SCALE64 * Pf::b(0x3e19_999a), DT * Pf::b(0x411f_2c8d));
        } else {
            if self.f13f8 != 0 {
                f = Pf::ONE - self.yaw_residual * Pf::b(0x3f0c_cccd);
                if f < Pf::ZERO { f = Pf::ZERO; }
            } else if self.stick_mag < Pf::b(0x3f4c_cccd) && Pf::b(0x3f06_0a92) < self.yaw_residual {
                // The residual is compared signed: only left turns cut the acceleration.
                f = Pf::ZERO;
            }
            if SPEED_TABLE[0][3] < self.stick_mag {
                self.run_turn();
            } else {
                let s = self.stick_mag + Pf::b(0x3eb3_3333);
                self.turn_to((SCALE64 * Pf::b(0x3ba3_d70a)) * s, SCALE64 * Pf::b(0x3dcc_cccd), (DT * Pf::b(0x4096_cbe4)) * s);
            }
        }
        self.lean(); // HeroLean 0x235638 after the turn: super::idle.
        self.clank_sway(); // 0x235e60: Clank's sway on the back (record 18), super::pose.
        let acc = (DT2 * Pf::b(0x40f0_0000)) * f;
        let mut dec = DT2 * Pf::b(0x4108_0000);
        if self.sharp_turn != 0 {
            let mut k = Pf::b(0x4027_8d36) - self.yaw_residual;
            if Pf::ONE < k { k = Pf::ONE; }
            if k < Pf::b(0x3e4c_cccd) { k = Pf::b(0x3e4c_cccd); }
            self.target_speed = self.target_speed * k;
            dec = DT2 * Pf::b(0x4188_0000);
        }
        self.speed_step(acc, dec);
        if self.strafe {
            let y = super::strafe::move_yaw(self);
            self.set_planar_vel(y);
        } else if self.turn_to_target != 0 && self.f658 == 0 {
            self.set_planar_vel(self.target_yaw);
        } else {
            self.set_planar_vel(Pf::b(0x47c3_4f80));
        }
        self.wall_check(env, 1);
        if self.edge_brake != 0 { self.edge_brake(env, Pf::b(0x40a0_0000), Pf::b(0x3e4c_cccd)); }
        if self.air_ticks != 0 {
            self.gravity_from(self.eff_v[2], DT2 * Pf::b(0x41c8_0000));
            self.climb_check(env);
        } else {
            let z = self.vel[2];
            self.gravity_from(z, DT2 * Pf::b(0x4258_0000));
        }
    }

    pub(super) fn tr_walk(&mut self, c: &mut Ctx) {
        let pad = c.env.pad;
        if self.land_run_blend != 0 {
            let v = c.anim.view();
            if 0.9 < v.t || v.seq_a != 0xff { self.land_run_blend = 0; }
        }
        if self.f063a != 0 && self.f65c == 0 { self.set_state(c, 0x79, true); return; }
        let air = self.air_ticks as i32;
        if ticks(5) <= air && (self.prev_group != 0x12 || self.f13f9 == 0) && Pf::b(0x3fac_cccd) < self.height
            && !self.step_down_ahead(c)
        {
            self.set_state(c, 6, true);
            return;
        }
        // 0x13f524 (set by the pack glide 8) shortens the jump buffer to this tick.
        let n = if self.f524 == 0 { ticks(9) } else { 1 };
        if pad.pressed_within(button::CROSS, n).is_some() && (!(ticks(4) < air) || self.f532 != 0) {
            self.stick_target(c.env, Pf::ONE);
            if self.try_jump(c, n, true) { return; }
            if self.substate <= 0 { self.set_state(c, 7, true); return; }
            self.jump_or_running_jump(c);
            return;
        }
        if air != 0 { return; }
        let strafe_ok = !(self.f0632 != 0 && (DT + DT <= self.eff_len_xy || Pf::b(0x3db2_b8c2) <= self.slope));
        if strafe_ok && pad.held & button::STRAFE != 0 { self.set_state(c, 1, true); return; }
        if pad.held & button::CROUCH != 0 { self.set_state(c, 4, true); return; }
        // The slippery-floor walk 0x2f (surface 7, 0x140632; level00's walk case): super::surface.
        super::surface::walk_surface(self, c);
        if self.state == 2 {
            if self.f13f9 != 0 { self.set_state(c, 0x73, true); }
        } else if self.state == 0x73 && self.f13f9 == 0 {
            self.set_state(c, 2, true);
        }
        // Magnetic floor (0x13f658, set by the surface reaction on the Magneboots' floor moby): 0x3f.
        if self.state == 2 && self.f658 != 0 { self.set_state(c, 0x3f, true); }
        // A grind rail under the feet (0x13f8bc): 0x28 (super::boots).
        if super::boots::rail_contact(self) { self.set_state(c, 0x28, true); return; }
        if (ticks(15) < self.timer || self.prev_group as u32 >= 2) && pad.pressed & button::FLICK != 0 && self.f13fa == 0 && !self.strafe {
            self.sharp_turn = 1;
        }
        if self.state == 0x73 {
            // Wading: stop when nothing is held and the speed is below 3.2 u/s; playback 35 × |eff| in 0.6..1.05.
            if pad.no_direction && len2(self.vel).to_f32() < DT.to_f32() * 3.2 && self.set_state(c, 0, false) {
                let seq = self.idle_seq();
                self.set_anim(c.anim, c.rng, blend(17), seq, 0);
            }
            self.anim_speed = Pf::f((self.eff_len.to_f32() * 35.0).clamp(0.6, 1.05));
            return;
        }
        if self.state != 2 { super::surface::walk_tail(self, c); return; }
        if !(self.stick_mag < Pf::b(0x3e2e_147b)) {
            self.walk_run_anim(c);
            return;
        }
        if DT * Pf::b(0x402c_cccd) < self.eff_len && self.f13fa == 0 {
            let short = self.eff_len < DT * Pf::b(0x408c_cccd) || pad.stick_len_ago(4) < Pf::b(0x3f4c_cccd);
            if !short { self.set_state(c, 3, true); return; }
            if !self.set_state(c, 3, false) { return; }
            if self.health == 1 { self.set_anim(c.anim, c.rng, blend(18), 0x54, 0); } else { self.set_anim(c.anim, c.rng, blend(9), 0x14, 7); }
        } else {
            if !(ticks(25) < self.timer) { return; }
            if super::weapons::gun_stance(self, c) || !self.set_state(c, 0, false) { return; }
            if self.health == 1 { self.set_anim(c.anim, c.rng, blend(18), 0x54, 0); } else { self.set_anim(c.anim, c.rng, Pf::b(0xbf80_0000), 0, 0); }
        }
    }

    /// Walk / run anim selector 0x241a08 (state 2, stick ≥ 0.17).
    pub(super) fn walk_run_anim(&mut self, c: &mut Ctx) {
        let old = self.substate;
        if ticks(4) < self.substate_timer && self.land_run_blend == 0 {
            let row = |s: i32| SPEED_TABLE[s.clamp(0, 1) as usize];
            let down = self.speed < row(old)[0] * DT;
            if down || row(old)[1] * DT < self.speed {
                let mut s = old;
                if down {
                    loop {
                        s -= 1;
                        if s < 0 || !(self.speed < row(s)[0] * DT) { break; }
                    }
                } else {
                    loop {
                        s += 1;
                        if s > 1 || !(row(s)[1] * DT < self.speed) { break; }
                    }
                }
                if s < 0 { s = 0; }
                if !(s < 2) { s = 1; }
                self.substate = s;
                if s != old {
                    let fc_n = c.anim.frame_count((s + 3) as u8) as i32;
                    let fc_o = c.anim.frame_count((old + 3) as u8) as i32;
                    let fb = c.anim.view().frame_b as i32;
                    let f = if fc_o > 0 && fc_n > 0 { ((fb * fc_n) / fc_o + PHASE_OFF[(s + 2 * old) as usize]) % fc_n } else { 0 };
                    self.set_anim(c.anim, c.rng, blend(8), (s + 3) as u8, f);
                }
            }
        }
        let (k, hi) = if self.substate == 0 { (Pf::b(0x431d_0000), Pf::b(0x4080_0000)) } else { (Pf::b(0x4160_0000), Pf::b(0x400c_cccd)) };
        let mut v = self.speed * k;
        if v < Pf::b(0x3f19_999a) { v = Pf::b(0x3f19_999a); }
        if hi < v { v = hi; }
        self.anim_speed = v;
    }
}
