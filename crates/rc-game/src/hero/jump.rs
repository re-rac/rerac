//! The jump group (group 4): the shared jump system of SetState (0x23cf98 case 7 … 0x69: the jump block
//! 0x13f720..0x13f7ff), physics (0x2370b8 case 0x23b4e4 with the vertical 0x2345f0 and the air control
//! 0x234b40) and transitions (0x242930 case 7 … 0x69), for the ids the port implements: 7 jump, 9 running jump,
//! 0xb flip, 0xe double jump, 0x12 water jump. The other ids of the group (Heli / Thruster jumps 10, 0xd, 0xf,
//! 0x10, the wall jump 0x11, the ledge climb 0x1c, the grind jumps 0x29 / 0x2a, the burn bounce 0x3c, 0x69 …)
//! belong to their packages (docs/plan/hero_states.md), which reuse [`Hero::jump_block_defaults`],
//! [`Hero::phys_jump`] and [`Hero::tr_jump`]. Spec: docs/plan/player_controller.md §3.1, §4.4.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use super::common::*;
use super::physics::*;
use super::states::Ctx;
use super::{state, Hero};
use crate::pad::button;
use crate::ps2v::Pf;

/// Running-jump sequence by run-cycle frame (0x17c258).
const RUN_JUMP_SEQ: [u8; 24] = [9, 9, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9];

impl Hero {
    /// SetState entry of the jump group for the ported ids: the jump lockout 0x13f542 undoes the state change
    /// (SetState returns 0), else [`Hero::jump_entry`].
    pub(super) fn jump_group_entry(&mut self, c: &mut Ctx, id: i32, play: bool, old_sub: i32) -> Option<bool> {
        if self.jump_lockout != 0 {
            self.state = self.prev_state;
            self.timer = self.prev_timer;
            self.substate = old_sub;
            return Some(false);
        }
        self.jump_entry(c, id, play);
        None
    }

    /// The part of the jump-group entry every jump id shares (0x23e7e8 up to the per-id parameters): group 4,
    /// 0x1415d4 = 0x50, vel = eff, and the jump block defaults (gravity 29.7·dt², windup 20 / 11·dt², reference
    /// height, the counters and flags cleared, fall-over 2.5 after 70, max air 80, air speed 5.7·dt, turn cap
    /// 860°/s, anim factor 0.5, no descending gravity). The package jump ids (10, 0xd, 0xf, 0x10, 0x11, 0x1c,
    /// 0x29, 0x2a, 0x3c, 0x69) call it and then set their own parameters (0x29 / 0x2a also 0x1415d4 = 3).
    /// A jump from the water (in water 0x140634, the feet under the level + 0.5), other than the deep-water jump 0x12,
    /// splashes: `0x22b3a8(3, 16, 0)` (0x23e890).
    pub(super) fn jump_block_defaults(&mut self, rng: &mut crate::rng::Rng) {
        self.group = 4;
        self.f15d4 = 0x50;
        self.vel = self.eff;
        if self.state != super::swim::id::WATER_JUMP && self.f0634 != 0 && self.pos[2].to_f32() < self.water_level.to_f32() + 0.5 {
            super::swim::effects::splash(self, rng, 3, 0x10, false);
        }
        let g = DT2;
        let j = &mut self.jump;
        j.g = g * Pf::b(0x41ed_999a);
        j.acc = g * Pf::b(0x41a0_0000);
        j.dec = g * Pf::b(0x4130_0000);
        j.ref_z = self.pos[2];
        j.curve_idx = -1;
        j.keep_speed = self.f0632 as i16;
        j.descending = 0;
        j.landed = 0;
        j.pending = Pf::ZERO;
        j.applied = Pf::ZERO;
        j.curve_on = 0;
        j.fallover_h = Pf::b(0x4020_0000);
        j.blocked = 0;
        j.f7fc = 0;
        j.flip_chain = 0;
        j.can_double = 0;
        j.max_air = ticks(80) as i16;
        j.fallover_after = ticks(70) as i16;
        j.air_speed = DT * Pf::b(0x40b6_6666);
        j.turn_max = DT * Pf::b(0x4170_2845);
        j.f7f6 = 0;
        j.ak = Pf::b(0x3f00_0000);
        j.g_down = Pf::ZERO;
        // 0x141604: only the wall jump 0x11 sets it again (super::ledge).
        self.ledge_blk.wall_chain = 0;
    }

    /// The jump-group entry (0x23e7e8) and the per-id jump block values.
    pub(super) fn jump_entry(&mut self, c: &mut Ctx, id: i32, play: bool) {
        self.jump_block_defaults(c.rng);
        let j = &mut self.jump;
        let six = Pf::b(0x3f19_999a);
        match id {
            7 => {
                j.h = Pf::b(0x3fbc_28f6);
                j.hmin = j.h;
                j.hmax = Pf::b(0x4027_ae14);
                j.ramp = ticks(15) as i16;
                j.takeoff = ticks(5);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(18.0), Pf::f(30.0), Pf::f(12.0));
                j.bottom784 = six;
                j.g_down = j.g * Pf::b(0x3f95_c28f);
                (j.can_double, j.f7fc, j.flip_chain) = (1, 1, 1);
            }
            9 => {
                j.h = Pf::ONE;
                j.hmin = j.h;
                j.hmax = Pf::b(0x402c_cccd);
                j.ramp = ticks(15) as i16;
                j.takeoff = ticks(5);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(18.0), Pf::f(29.0), Pf::f(11.0));
                j.bottom784 = six;
                j.air_speed = DT * Pf::b(0x40b6_6666);
                (j.can_double, j.f7fc, j.flip_chain) = (1, 1, 1);
            }
            0xe => {
                let mut h = Pf::b(0x3f33_3333);
                if self.height < Pf::b(0x4000_0000) {
                    let t = (Pf::b(0x4020_0000) - self.height) * Pf::b(0x3f00_0000);
                    h = t + Pf::b(0x3f33_3333);
                }
                j.h = h;
                j.hmin = h;
                j.hmax = h + Pf::b(0x3d4c_cccd);
                j.ramp = ticks(14) as i16;
                j.takeoff = -1;
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(32.0), Pf::f(43.0), Pf::f(19.0));
                j.bottom784 = six;
                j.ak = Pf::ONE;
                j.air_speed = DT * Pf::b(0x4060_0000);
                j.f7f6 = ticks(30) as i16;
            }
            0x12 => {
                // The jump out of the water: from shallow water (depth < 1.25) a plain 1.95 jump after a
                // 15-tick windup; from deep water vz = 0 and the scripted rise 0x17c3e0 over ticks 2..37.
                j.h = Pf::f(0.1);
                j.hmin = j.h;
                j.hmax = Pf::f(0.2);
                j.ramp = ticks(10) as i16;
                j.takeoff = ticks(2);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(18.0), Pf::f(28.0), Pf::f(14.0));
                j.bottom784 = six;
                if self.f15f4.to_f32() < 1.25 {
                    j.takeoff = ticks(15);
                    j.h = Pf::f(1.95);
                    j.hmin = j.h;
                    j.hmax = Pf::f(1.98);
                } else {
                    self.vel[2] = Pf::ZERO;
                    j.curve_on = 1;
                    j.curve_from = ticks(2);
                    j.curve_to = ticks(35) + j.curve_from;
                    j.curve_frame = Pf::ZERO;
                    j.f72c = Pf::f(18.0);
                    j.curve_len = ticks(40);
                }
                self.items.f13f7 = 1;
            }
            0x1c => {
                // The ledge climb (super::ledge): straight up 2.0 after a 10-tick windup; the forward push is
                // the climb branch of the horizontal control (ledge::jump_horizontal).
                j.h = Pf::f(2.0);
                j.hmin = j.h;
                j.hmax = Pf::f(2.05);
                j.ramp = ticks(1) as i16;
                j.takeoff = ticks(10);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(20.0), Pf::f(29.0), Pf::f(19.0));
                j.bottom784 = Pf::b(0x3f59_999a);
                j.g = DT2 * Pf::f(25.0);
                j.flip_chain = 1;
                j.ak = Pf::ONE;
                j.speed7ac = Pf::ZERO;
                self.items.f13f7 = 1;
            }
            0x11 => {
                // The wall jump (super::ledge): h 3.3 after a 9-tick windup on the wall, then along the wall
                // normal probe A accepted (0x13f7b0 → 0x13f7c0); 0x141604 marks the chain.
                j.h = Pf::f(3.3);
                j.hmin = j.h;
                j.hmax = Pf::f(3.35);
                j.ramp = ticks(12) as i16;
                j.takeoff = ticks(9);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(12.0), Pf::f(20.0), Pf::f(10.0));
                j.bottom784 = six;
                j.fallover_after = ticks(80) as i16;
                j.g = DT2 * Pf::f(29.0);
                j.f7fc = 1;
                let l = &mut self.ledge_blk;
                (l.jump_normal, l.jump_yaw, l.wall_chain) = (l.wall_normal, l.wall_yaw, 1);
                self.items.f13f7 = 1;
            }
            10 => {
                // The Heli-Pack long jump (super::packs): h 1.9 after a 5-tick windup (pushed forward 53·dt²·T
                // then), gravity 11·dt², no air control; 120 ticks before the forced fall.
                j.h = Pf::f(1.9);
                j.hmin = j.h;
                j.hmax = Pf::f(1.95);
                j.ramp = ticks(14) as i16;
                j.takeoff = ticks(5);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(18.0), Pf::f(29.0), Pf::f(10.0));
                j.bottom784 = six;
                j.flip_chain = 1;
                j.g = DT2 * Pf::f(11.0);
                j.max_air = ticks(120) as i16;
                j.fallover_after = ticks(90) as i16;
                j.f7f6 = ticks(70) as i16;
            }
            0x10 => {
                // The Thruster-Pack long jump: a 0.4..0.9 hop, gravity 8.5·dt², straight along the facing at
                // 0x13f744 = 11.5 u/s (5.7 from above 1.4), windup acc 44 / dec 50·dt². Its after-images: three ghosts
                // 2 / 4 / 6 ticks back (crate::afterimage; faded by super::packs::thruster_trail).
                super::packs::thruster_trail_start(self, &[(0x28, 2), (0x14, 4), (0x0a, 6)]);
                let j = &mut self.jump;
                j.h = Pf::f(0.4);
                j.hmin = Pf::f(0.4);
                j.hmax = Pf::f(0.9);
                j.ramp = ticks(1) as i16;
                j.takeoff = ticks(5);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(18.0), Pf::f(30.0), Pf::f(11.0));
                j.bottom784 = Pf::b(0x3f4c_cccd);
                j.f744 = DT * Pf::f(11.5);
                j.flip_chain = 1;
                if Pf::f(1.4) < self.height { j.f744 = DT * Pf::b(0x40b6_6666); }
                j.g = DT2 * Pf::f(8.5);
                j.acc = DT2 * Pf::f(44.0);
                j.dec = DT2 * Pf::f(50.0);
            }
            0xd => {
                // The Thruster-Pack high jump: a 0.1..0.2 hop after a 9-tick windup, then the curve
                // packs::THRUSTER_HIGH_JUMP_CURVE over ticks 9..44 (the anim aims at frame 26). Its after-images: two
                // ghosts 3 / 5 ticks back (crate::afterimage).
                super::packs::thruster_trail_start(self, &[(0x28, 3), (0x14, 5)]);
                let j = &mut self.jump;
                j.h = Pf::f(0.1);
                j.hmin = j.h;
                j.hmax = Pf::f(0.2);
                j.ramp = ticks(14) as i16;
                j.takeoff = ticks(9);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(26.0), Pf::f(38.0), Pf::f(18.0));
                j.bottom784 = six;
                j.fallover_h = Pf::f(4.5);
                j.max_air = ticks(120) as i16;
                j.air_speed = DT * Pf::f(1.9);
                j.curve_on = 1;
                j.curve_from = ticks(9);
                j.curve_to = ticks(35) + j.curve_from;
                j.curve_frame = Pf::ZERO;
                j.f72c = Pf::f(26.0);
                j.curve_len = ticks(45);
                j.flip_chain = 1;
            }
            0xf => {
                // The Heli-Pack high jump: h 1.9 after a 9-tick windup, then the rotor's lift
                // packs::HELI_HIGH_JUMP_CURVE over ticks 35..71 (the anim aims at frame 29, then 50); no glide
                // before tick 85, no ledge regrab for 70 ticks.
                j.h = Pf::f(1.9);
                j.hmin = j.h;
                j.hmax = Pf::f(1.95);
                j.ramp = ticks(5) as i16;
                j.takeoff = ticks(9);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(50.0), Pf::f(64.0), Pf::f(28.0));
                j.bottom784 = six;
                j.fallover_h = Pf::f(4.5);
                j.max_air = ticks(130) as i16;
                j.air_speed = DT * Pf::f(3.3);
                j.f7f6 = ticks(85) as i16;
                j.curve_on = 1;
                // ticks(0x22) on PAL (0x15ed80), 0x23 on NTSC.
                j.curve_from = ticks(0x23);
                j.curve_to = ticks(0x24) + j.curve_from;
                j.curve_frame = Pf::f(29.0);
                j.f72c = Pf::f(50.0);
                j.curve_len = ticks(0x56);
                j.flip_chain = 1;
                self.f500 = ticks(70) as i16;
            }
            _ => {
                // 0xb: the side / back flip.
                j.h = Pf::b(0x404c_cccd);
                j.hmin = j.h;
                j.hmax = Pf::b(0x4050_0000);
                j.ramp = ticks(14) as i16;
                j.takeoff = ticks(5);
                (j.f_apex, j.f_hold, j.f_land) = (Pf::f(18.0), Pf::f(31.0), Pf::f(24.0));
                j.bottom784 = six;
                j.fallover_h = Pf::b(0x4090_0000);
                j.fallover_after = ticks(50) as i16;
                j.ak = Pf::ONE;
                let face = self.yaw_ago(ticks(10));
                self.jump.face_yaw = face;
                let sector = self.stick_sector(c);
                self.jump.kind7a0 = sector;
                self.jump.dir_yaw = self.target_yaw;
                let l = len2(self.eff);
                self.jump.side_speed = if DT * Pf::b(0x3fc0_0000) < l { l } else { Pf::ZERO };
                self.jump.flip_chain = 1;
                self.jump.fwd_speed = Pf::ZERO;
                self.jump.f7fc = 1;
            }
        }
        if !play { return; }
        match id {
            7 => self.set_anim(c.anim, c.rng, blend(5), 7, 0),
            9 => {
                let v = c.anim.view();
                let f = Pf::f(v.frame).to_i32();
                let seq = if v.seq_a == 4 { RUN_JUMP_SEQ.get(f.max(0) as usize).copied().unwrap_or(8) } else { 8 };
                self.set_anim(c.anim, c.rng, blend(5), seq, 0);
            }
            0xe => {
                self.set_anim(c.anim, c.rng, blend(7), 0x16, 3);
                self.anim_speed = SCALE60 * Pf::b(0x3f4c_cccd);
            }
            0x12 => self.set_anim(c.anim, c.rng, Pf::b(0xbf80_0000), 0x72, 1),
            0x1c => self.set_anim(c.anim, c.rng, blend(4), 0x22, 0),
            0x11 => self.set_anim(c.anim, c.rng, blend(7), 0x23, 2),
            10 => self.set_anim(c.anim, c.rng, blend(5), 0x12, 0),
            0xd => self.set_anim(c.anim, c.rng, blend(5), 0x11, 0),
            0xf => self.set_anim(c.anim, c.rng, blend(5), 0x15, 0),
            0x10 => {
                // From a jump (or the first 7 ticks of a crouch): a longer blend and 3 more windup ticks.
                if self.prev_group == 4 || (self.prev_state == 4 && self.timer < ticks(7)) {
                    self.set_anim(c.anim, c.rng, blend(8), 0x26, 0);
                    self.jump.takeoff += ticks(3);
                } else {
                    self.set_anim(c.anim, c.rng, blend(5), 0x26, 0);
                }
            }
            _ => {
                let mut d = self.jump.kind7a0;
                if d == 2 { d = 0; self.jump.kind7a0 = 0; }
                let mut seq = 0x1c + d as u8;
                if c.env.mirror && (seq == 0x1c || seq == 0x1d) { seq ^= 1; }
                if self.prev_group == 4 {
                    self.set_anim(c.anim, c.rng, blend(4), seq, 1);
                    self.jump.takeoff = ticks(1);
                } else {
                    self.set_anim(c.anim, c.rng, blend(5), seq, 0);
                }
            }
        }
    }

    /// Jumps 7, 9, 0xb, 0xe (0x23b4e4).
    pub(super) fn phys_jump(&mut self, env: &Env) { self.phys_jump_anim(env, 0.0) }

    /// The jump-group physics (0x23b4e4) with Ratchet's key time 0x13fdf8 (read by the ledge climb 0x1c's horizontal
    /// control; the other ids ignore it).
    pub(super) fn phys_jump_anim(&mut self, env: &Env, key: f32) {
        if self.jump.keep_speed != 0 && self.f0632 != 0 {
            self.target_speed = self.speed;
        } else {
            if self.jump.landed != 0 { self.stick_target(env, DT * Pf::b(0x40b6_6666)); }
            self.stick_target(env, self.jump.air_speed);
        }
        if self.timer == self.jump.takeoff { self.jump.takeoff_pos = self.pos; }
        // The horizontal control runs from tick 0 (no windup) for 0xb, 0xc, 0x10, 0x11, 0x1c, 0x4c.
        let from_start = matches!(self.state, 0xb | 0xc | 0x10 | 0x11 | 0x1c | 0x4c);
        let windup = self.jump.landed != 0 || (self.timer < self.jump.takeoff && !from_start);
        if windup {
            self.turn_to(SCALE64 * Pf::b(0x3c75_c28f), SCALE64 * Pf::b(0x3e4c_cccd), DT * Pf::b(0x4170_2845));
            let s = self.speed;
            if self.jump.landed >= 2 { self.speed = s * Pf::b(0x3ecc_cccd); }
            self.set_planar_vel(Pf::b(0x47c3_4f80));
            self.speed = s;
            if self.jump.landed != 1 { self.speed_step(self.jump.acc, self.jump.dec); }
            if self.f658 != 0 { self.speed = Pf::ZERO; }
            if self.jump.landed != 0 {
                if env.pad.no_direction && self.f65a != 0 { self.speed = Pf::ZERO; }
                self.momentum_decay(self.jump.dec);
            }
            self.drag(Pf::b(0x3f00_0000), DT + DT);
            if self.state == state::RUN_JUMP && self.timer < self.jump.takeoff + ticks(4) {
                // 0x248da0: push forward along the moby's yaw by 16·dt²·T.
                let a = (DT2 * Pf::b(0x4180_0000)) * Pf::from_i32(self.timer);
                self.vel[0] = self.vel[0] + fast_cos(self.moby_rot[2]) * a;
                self.vel[1] = self.vel[1] + fast_sin(self.moby_rot[2]) * a;
            }
            if self.state == 10 && self.timer < ticks(34) {
                // The Heli long jump's push: 53·dt²·T along the moby's yaw.
                let a = (DT2 * Pf::f(53.0)) * Pf::from_i32(self.timer);
                self.vel[0] = self.vel[0] + fast_cos(self.moby_rot[2]) * a;
                self.vel[1] = self.vel[1] + fast_sin(self.moby_rot[2]) * a;
            }
            if self.timer < ticks(10) && Pf::b(0x3e4c_cccd) < self.stick_mag {
                let mut a = self.pos;
                a[2] = a[2] + Pf::b(0x3f00_0000);
                let k = Pf::b(0x3f59_999a);
                let b = [a[0] + fast_cos(self.rot[2]) * k, a[1] + fast_sin(self.rot[2]) * k, a[2], a[3]];
                if env.line(a, b, 4).is_some() { self.jump.blocked = 1; }
            }
        } else {
            if matches!(self.state, 0x11 | 0x1c) {
                super::ledge::jump_horizontal(self, env, key);
            } else if self.state == 0x4c {
                // Clank's climb (L00 0x214658's 0x4c branch: super::bodies::clank).
                super::bodies::clank::climb_horizontal(self, key);
            } else if matches!(self.state, 10 | 0x10) {
                super::packs::jump_horizontal(self);
            } else {
                self.air_control(env);
            }
            if self.state == super::swim::id::WATER_JUMP {
                // The water jump caps the air speed at 3.7 u/s.
                let l = len2(self.vel).to_f32();
                let m = DT.to_f32() * 3.7;
                if m < l { self.vel = set_len2(self.vel, Pf::f(m)); }
            }
        }
        // Reference height 0x13f760.
        if self.jump.ref_z < self.ground_z {
            let r = self.jump.ref_z + DT * Pf::b(0x40c0_0000);
            self.jump.ref_z = r;
            if self.ground_z < r { self.jump.ref_z = self.ground_z; }
        }
        if self.pos[2] < self.jump.ref_z { self.jump.ref_z = self.pos[2]; }
        // Descending flag (and the descending gravity).
        if self.jump.takeoff < self.timer && self.vel[2] < Pf::b(0x3a83_126f)
            && (self.jump.curve_on == 0 || self.jump.curve_to < self.timer)
        {
            self.jump.descending = 1;
            if self.jump.g_down != Pf::ZERO { self.jump.g = self.jump.g_down; }
        }
        // The Thruster long jump smashes the crates ahead: super::packs.
        super::packs::thruster_crates(self, env, key);
        // HeroLean 0x235638 on foot (0x1413f4 = 0): super::idle; Clank's lean 0x215b68 in body 1 (super::bodies::clank).
        if self.mode == 0 { self.lean(); }
        if self.mode == 1 { super::bodies::clank::lean(self); }
        self.jump_vertical(env);
        // The wall / ledge probe 0x22c9a0 (sets 0x13f504 / 0x13f838): super::ledge.
        super::ledge::wall_ledge_probe_a(self, env);
    }

    /// Jump vertical 0x2345f0 (7 / 9 / 0xb: variable height; 0xe: the double-jump boost), then gravity.
    pub(super) fn jump_vertical(&mut self, env: &Env) {
        if self.state != state::DOUBLE_JUMP {
            let j = &mut self.jump;
            let nothing_yet = j.pending == Pf::ZERO && j.applied == Pf::ZERO;
            let water_jump = self.state == super::swim::id::WATER_JUMP;
            if (env.pad.held & crate::pad::button::CROSS != 0 || nothing_yet || water_jump) && j.h < j.hmax && self.timer <= ticks(15) {
                let h = j.h + (j.hmax - j.hmin) / Pf::from_i32(j.ramp as i32);
                j.h = h;
                if j.hmax < h { j.h = j.hmax; }
                let v = ((j.h + j.h) * j.g).sqrt();
                j.pending = j.pending + ((v - j.applied) - j.pending);
            }
            if !(self.timer < j.takeoff) && Pf::ZERO < j.pending {
                self.vel[2] = self.vel[2] + j.pending;
                let p = j.pending;
                j.pending = Pf::ZERO;
                j.applied = j.applied + p;
            }
            // 0x13f76c scripted vertical curve (the deep-water jump 0x12, table 0x17c3e0): inside the window
            // the row's acceleration (·dt², kept for a −999999 row) plus its increment per further tick is added
            // to vz.
            if j.curve_on != 0 && j.curve_from <= self.timer && self.timer < j.curve_to {
                // The table 0x13f734 the entry set: the pack high jumps' or the deep-water jump's.
                let rows: &[(f32, f32, i32)] = match self.state {
                    0xd => &super::packs::THRUSTER_HIGH_JUMP_CURVE,
                    0xf => &super::packs::HELI_HIGH_JUMP_CURVE,
                    _ => &super::swim::WATER_JUMP_CURVE,
                };
                j.curve_tick += 1;
                let dt2 = DT2.to_f32();
                // (The table ends in a 100-tick row, past any window; the port stops there instead of reading on.)
                while j.curve_idx < 0 || (ticks(rows[j.curve_idx as usize].2) <= j.curve_tick && (j.curve_idx as usize) + 1 < rows.len()) {
                    j.curve_idx += 1;
                    j.curve_tick = 0;
                    let base = rows[j.curve_idx as usize].0;
                    if base != -999_999.0 { j.curve_acc = Pf::f(base * dt2); }
                }
                if 0 < j.curve_tick { j.curve_acc = Pf::f(j.curve_acc.to_f32() + rows[j.curve_idx as usize].1 * dt2); }
                self.vel[2] = Pf::f(self.vel[2].to_f32() + j.curve_acc.to_f32());
            }
        } else {
            let mut v = DT * Pf::b(0x40e0_0000);
            // The Heli-Pack owned with the Thruster-Pack on the back (0x13d4c2, `GetClankModule(3)` = 3, Clank
            // shown) keeps more of the boost higher up; the Thruster-Pack adds a quarter.
            let thruster = self.back_module() == 3 && self.back_slot.clank_hidden == 0;
            let above = vsub(self.pos, self.jump.takeoff_pos)[2];
            if thruster && self.owned.has(2) {
                if Pf::b(0x402c_cccd) < above {
                    v = v * Pf::b(0x3ef0_a3d7);
                } else if Pf::b(0x4006_6666) < above {
                    v = v * Pf::b(0x3f33_3333);
                }
            } else if Pf::b(0x4006_6666) < above {
                v = v * Pf::b(0x3ee6_6666);
            } else if Pf::b(0x3fd9_999a) < above {
                v = v * Pf::b(0x3f33_3333);
            }
            if thruster { v = v * Pf::b(0x3fa0_0000); }
            if self.timer < ticks(8) {
                let mut z = self.vel[2];
                if z < v { approach(v, DT2 * Pf::b(0x4316_0000), &mut z); }
                self.vel[2] = z;
            }
        }
        if self.timer < self.jump.takeoff {
            self.vel[2] = Pf::ZERO;
            let z = self.vel[2];
            self.gravity_from(z, DT2 * Pf::b(0x4240_0000));
        } else {
            let z = self.vel[2];
            self.gravity_from(z, self.jump.g);
            let mut vz = self.vel[2];
            let lo = self.eff[2] - Pf::b(0x3dcc_cccd);
            if vz < lo { vz = lo; }
            let floor = -(DT * Pf::b(0x4248_0000));
            if vz < floor { vz = floor; }
            self.vel[2] = vz;
        }
    }

    pub(super) fn tr_jump(&mut self, c: &mut Ctx) {
        let pad = c.env.pad;
        // A: flip chain.
        if self.jump.flip_chain != 0 {
            let w = self.flip_window(c);
            if w > 0 && (self.timer < w || self.jump.landed != 0) && self.grounded_ticks != 0 {
                let x7 = pad.pressed_within(button::CROSS, ticks(7)).is_some();
                if !(self.state == 0xb && !x7) {
                    let mut ok = true;
                    if self.state == 0x1c || self.jump.landed != 0 { ok = x7; }
                    if self.state == 0xb && self.jump.landed == 0 { ok = false; }
                    // Port-only: while the Port Options strafe is held, the stick itself chains the flips (super::strafe).
                    let strafe_chain = self.strafe && Pf::f(0.2) < self.stick_mag;
                    if ok && (strafe_chain || (pad.pressed_within(0x1f000, ticks(5)).is_some() && pad.held & button::CROUCH != 0)) {
                        self.set_state(c, 0xb, true);
                        if strafe_chain && self.state == 0xb { super::strafe::carry_along_stick(self); }
                        return;
                    }
                }
            }
        }
        // C: pack moves (the Thruster ✕ combo → 0x10 ends the pass; an R1 tap → Heli 10 / Thruster 0x10 goes on in
        // the new state): super::packs.
        if super::packs::jump_thruster_combo(self, c) { return; }
        super::packs::jump_pack_tap(self, c);
        // D: R1/R2 in the air: the Thruster's stomp 0x22 (super::packs); the pass ends either way.
        if self.jump.takeoff + ticks(8) < self.timer
            && (self.jump.descending == 0 || ticks(10) < self.jump.land_eta)
            && self.stick_mag < Pf::b(0x3e99_999a)
            && matches!(self.state, 7 | 9 | 0xd | 0xf | 0xe)
            && pad.pressed_within(button::CROUCH, ticks(9)).is_some()
        {
            super::packs::jump_crouch_press(self, c);
            return;
        }
        // The rail / cable overlays' line (level00 0x229b70 case 7.., Kerwan 0x21c668): a grind rail under the feet
        // (0x13f8bc) → 0x28, a cable at the hands (0x13f94c) → 0x74 (super::boots; both flags stay 0 elsewhere).
        if super::boots::jump_contacts(self, c) { return; }
        // E: force fall.
        if (self.jump.max_air as i32) < self.timer {
            if self.jump.landed == 0 {
                self.set_state(c, 6, true);
                return;
            }
        } else if super::packs::pack_jump_wall_hit(self, c) {
            // 10 / 0x10 into a wall → the rebound 0x7a.
            return;
        }
        let v = c.anim.view();
        let hold_land = self.state == 0xe && v.frame < 35.0;
        if !(self.jump.takeoff < self.timer) { return; }
        if self.jump.curve_on != 0 && self.timer == self.jump.takeoff + 1 {
            // Scripted curve: aim the playback at 0x13f728 by the window start.
            let k = self.jump.curve_from - self.timer;
            if 0 < k { self.anim_speed_for_ticks(self.jump.curve_frame, Pf::from_i32(k), self.jump.ak, Pf::b(0xbf80_0000), &v); }
            return;
        }
        if self.jump.curve_on != 0 && self.jump.curve_from <= self.timer && self.timer < self.jump.curve_to {
            // Inside the window: the playback reaches the apex frame over the curve's length.
            let k = self.jump.curve_len - self.jump.curve_from;
            if 0 < k { self.anim_speed_for_ticks(self.jump.f_apex, Pf::from_i32(k), self.jump.ak, Pf::b(0xbf80_0000), &v); }
            return;
        }
        if self.timer == self.jump.takeoff + 1 {
            if self.state == 0xe { return; }
            let n = (self.vel[2] / self.jump.g).to_i32();
            self.anim_speed_for_ticks(self.jump.f_apex, Pf::from_i32(n), self.jump.ak, Pf::b(0xbf80_0000), &v);
            if Pf::b(0x3fc0_0000) < self.anim_speed { self.anim_speed = Pf::b(0x3fc0_0000); }
            return;
        }
        // Coming down into water (the probe's first tick under the level, 0x13f648 == 1, 0x24621c): the landing voice
        // 0x11 and the big splash 0x22b3a8(3, 24, 1).
        if self.in_water == 1 {
            let e = if c.voice(0x11, 0) { super::swim::SwimEvent::Played(0x11) } else { super::swim::SwimEvent::Sound(0x11) };
            self.swim.events.push(e);
            super::swim::effects::splash(self, c.rng, 3, 0x18, true);
            self.swim.events.push(super::swim::SwimEvent::Splash { rings: 3, drops: 0x18, big: true });
        }
        // J: landing detection.
        if self.jump.landed != 0 {
            self.jump.landed += 1;
        } else if self.air_ticks == 0 && self.jump.descending != 0 && !hold_land {
            self.jump.landed = ticks(1);
            // `0x248920`: the landing's foot motes (super::pose).
            self.land_motes(super::pose::motes::LAND);
            let f = self.jump.f_land.to_i32();
            if Pf::f(v.frame) < self.jump.f_hold - Pf::b(0x3fc0_0000) {
                self.set_anim(c.anim, c.rng, blend(7), v.seq_b, f + 2);
            }
        }
        if self.jump.landed != 0 {
            self.anim_speed = Pf::ONE;
        } else if self.jump.descending != 0 {
            self.descent_anim(c, hold_land);
        }
        // The Heli-Pack long jump 10 lands into the skid 3: super::packs.
        if self.jump.landed != 0 && self.state == 10 {
            super::packs::heli_long_jump_landed(self, c);
            return;
        }
        // K: bunny hop.
        if self.jump.landed != 0 && self.f063a == 0 && self.lockout == 0
            && pad.pressed_within(button::CROSS, ticks(8)).is_some() && (self.air_ticks as i32) < ticks(4)
        {
            self.jump_or_running_jump(c);
            return;
        }
        // L: landing picker / fall-over.
        if ticks(1) < self.jump.landed && self.state != 10 {
            self.landing_picker(c);
        } else if self.jump.descending != 0 && (self.jump.fallover_after as i32) <= self.timer && self.jump.fallover_h < self.height {
            self.set_state(c, 6, true);
        }
        // The Thruster long jump 0x10 falls over after 60 ticks above 1.5: super::packs.
        if super::packs::thruster_fallover(self, c) { return; }
        // N: wall jump (0x13f504 wall window from the ledge probes): super::ledge.
        if super::ledge::wall_jump(self, c) { return; }
        // O: the long jumps' glide hold-off (super::packs); the glide's earliest tick (20; 8 in 0x10; 17 in 0xe
        // above 4).
        super::packs::long_jump_glide_holdoff(self);
        let mut glide_after = if self.state == 0x10 { ticks(8) } else { ticks(20) };
        // P / Q: double jump.
        if self.state == state::DOUBLE_JUMP && Pf::b(0x4080_0000) < self.height {
            glide_after = ticks(17);
            if self.jump.descending == 0 && self.anim_speed < Pf::b(0x3fa6_6666) {
                approach(Pf::b(0x3fa6_6666), SCALE60 * Pf::b(0x3dcc_cccd), &mut self.anim_speed);
            }
            self.jump.f7f6 = ticks(17) as i16;
        }
        let mut k = self.timer - 4;
        if k <= 1 { k = 2; }
        if ticks(30) < k { k = ticks(30); }
        if self.jump.landed == 0 && !matches!(self.state, 0x12 | 0x1c | 0x11 | 0xb | 0x3c | 0x69) {
            let p = pad.pressed_within(button::CROSS, k).is_some();
            if (p || matches!(self.state, 0xf | 0xd))
                && self.state != 0xe && self.f13f8 == 0 && (self.f063a == 0 || !(self.height < Pf::b(0x3ecc_cccd)))
                && self.jump.can_double != 0 && ticks(14) < self.timer
                && (self.jump.descending == 0
                    || (Pf::b(0x3f33_3333) < self.height && self.vel[2].abs() < DT * Pf::b(0x40b9_999a)))
            {
                self.set_state(c, 0xe, true);
            } else if p || matches!(self.state, 0xd..=0xf) {
                // No double jump: the Heli-Pack glide 8 (✕ held): super::packs.
                super::packs::jump_to_glide(self, c, glide_after);
            }
        }
        // The ledge grab 0x18 (0x13f838) from any jump but the flip.
        if self.f838 != 0 && self.state != 0xb { self.set_state(c, 0x18, true); }
    }

    /// `LandingPicker()` 0x242198 (first runs the tick after touchdown).
    pub(super) fn landing_picker(&mut self, c: &mut Ctx) {
        self.f15d4 = 0;
        // Level00's picker (0x2293a8) first: landing on a slippery floor (0x140632) faster than 0.5 u/s → the
        // slide 0x2f.
        if self.f0632 != 0 && DT * Pf::b(0x3f00_0000) < self.eff_len_xy {
            self.set_state(c, 0x2f, true);
            return;
        }
        self.stick_target(c.env, Pf::ONE);
        if c.env.pad.held & button::CROUCH != 0 {
            self.set_state(c, 4, false);
            self.set_anim(c.anim, c.rng, Pf::b(0xc000_0000), 0xd, 2);
            c.anim.set_loop(3, 18);
            return;
        }
        if Pf::b(0x3e99_999a) < self.stick_mag && self.lockout == 0 {
            if self.f0632 != 0 || self.f13f9 != 0 { self.set_state(c, 2, true); return; }
            let landed = self.jump.landed;
            if self.set_state(c, 2, false) {
                self.substate = 1;
                let b = if ticks(5) < landed { ticks(14) } else { ticks(8) };
                self.set_anim(c.anim, c.rng, Pf::from_i32(b), 4, 0);
                self.land_run_blend = 1;
            }
            return;
        }
        if (DT * Pf::b(0x4040_0000) < self.fwd_speed && ticks(3) < self.jump.landed) || self.state == 0x10 {
            if self.set_state(c, 3, false) {
                self.set_anim(c.anim, c.rng, Pf::b(0xbf80_0000), 5, 0);
                clamp_len_2745f0(&mut self.momentum, DT * Pf::b(0x40a0_0000));
            }
            return;
        }
        let finished = c.anim.view().flags & 2 != 0;
        if (ticks(12) < self.jump.landed || finished) && self.set_state(c, 0, false) {
            if self.idle_seq() == 0x54 {
                self.set_anim(c.anim, c.rng, blend(15), 0x54, 0);
            } else if finished {
                self.set_anim(c.anim, c.rng, Pf::b(0xc000_0000), self.idle_seq(), 0);
            }
        }
    }

    /// Descent anim control 0x246310 (jump group, descending, not landed).
    pub(super) fn descent_anim(&mut self, c: &mut Ctx, hold_land: bool) {
        let v = c.anim.view();
        let f3 = self.jump.f_land.to_i32();
        if (v.frame_b as i32) < f3 {
            if self.state == 0xe && v.frame < 37.0 {
                approach(Pf::ONE, SCALE64 * Pf::b(0x3e19_999a), &mut self.anim_speed);
            } else {
                let e = self.land_eta(c, Pf::b(0x4270_0000), self.jump.g, Pf::b(0xbf80_0000));
                self.jump.land_eta = e.to_i32();
                self.anim_speed_for_ticks(self.jump.f_hold, Pf::from_i32(self.jump.land_eta), self.jump.ak, SCALE64 * Pf::b(0x3e4c_cccd), &v);
            }
            if !hold_land && Pf::b(0x3fe6_6666) < self.anim_speed {
                self.set_anim(c.anim, c.rng, Pf::from_i32(self.jump.land_eta + 3), v.seq_b, f3 + 2);
            }
            if self.anim_speed < Pf::b(0x3e4c_cccd) { self.anim_speed = Pf::b(0x3e4c_cccd); }
            if Pf::b(0x402c_cccd) < self.anim_speed { self.anim_speed = Pf::b(0x402c_cccd); }
        }
        let v = c.anim.view();
        if self.state == state::RUN_JUMP && Pf::b(0x40c0_0000) < self.jump.f_hold - Pf::f(v.frame)
            && (v.seq_a == 8 || v.seq_a == 9) && v.seq_a == v.seq_b
            && self.target_speed < (SPEED_TABLE[1][2] * DT) * Pf::b(0x3dcc_cccd)
            && self.eff_len_xy < DT * Pf::b(0x4020_0000)
        {
            let mut b = Pf::from_i32(ticks(9)) / self.anim_speed;
            if Pf::from_i32(ticks(10)) < b { b = Pf::from_i32(ticks(10)); }
            self.set_anim(c.anim, c.rng, b, 7, (Pf::f(v.frame).to_i32() + 4) / 2);
            self.state = state::JUMP; // raw write, no SetState
        }
        let v = c.anim.view();
        if Pf::f(v.frame).to_i32() == self.jump.f_hold.to_i32() { self.anim_speed = Pf::ZERO; }
    }
}
