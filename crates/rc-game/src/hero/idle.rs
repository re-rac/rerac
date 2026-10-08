//! Ratchet's idle behaviour (docs/plan/player_controller.md §13 "Idle behaviour"): the fidget chance
//! `0x241e00` and its caller in the idle transitions, the head look `0x22b928`, the idle secondaries
//! `0x22bdd0`, Ratchet's blink (`0x227590` → `0x2274e8`), Clank's glow and blink `0x2278c0`, the joint
//! springs of the records those write (`0x2273d0` → `0x227050`), and the back items (the pack and Clank)
//! as far as the idle code drives them: the sequence table the hero's animation calls draw from
//! (`0x2476d0`, called from `SetAnim`'s `0x247550` and the advance's `0x247800`), Clank's fidget
//! `0x2473e0`, and their creation / advance in `HeroItemsUpdate` (`0x22f3c0` / `0x22fec0`).
//!
//! Every routine here draws from the game's one `rand` stream in the game's order within the hero update
//! `0x228870`:
//! 1. advance `0x247d48` → `0x247800`: one `rand_range(0, n − 1)` per tick while Ratchet's sequence has `n > 0`
//!    rows in the back table (sequence 0 has 3 on Novalis: one draw per idle tick);
//! 2. the idle transitions: `SetAnim` → `0x247550` (one draw when the new sequence has rows, e.g. every blend
//!    back to sequence 0), the fidget chance (one `randf` per tick in sequence 0 once the 120-tick cooldown is
//!    over and a record is available), Clank's fidget (`rand_range(110, 270)` + one table draw);
//! 3. the head look: in sequence 1/2 one `rand_range(40, 70)` per tick, in sequence 0 four draws per expiry;
//! 4. the secondaries (sequence 0 only): 2, 2, 2, 3 draws per expiry of their four timers;
//! 5. Ratchet's blink: two `randi(period)` per blink; Clank's blink: one `rand_range(50, 200)` per blink.
//!
//! Standard floats (`f32`) except where an existing PS2-float helper of the hero code is reused (the joint
//! springs `turn_spring`, `fast_sin`); the random helpers are [`crate::rng`]'s.
//!
//! The edge look-down branch of `0x22b928` reads the edge probe `Hero::edge_probe`. The other producers the hero update
//! runs around these (the feet `0x22c5c0`, `HeroScanTargets` 0x22c080, the Magneboots lean `0x2352e0`) are
//! `super::pose`. The cheats (`crate::cheats`, [`Hero::cheats`]): 0x15edb1 (record 17's scale 1.57), 0x15edb3 (0x15ee18 → 1.8,
//! record 18 = 0x15ee18). Not ported: the cheat 0x15edb5's mirror (below), Clank hidden (0x141628, 0 on Novalis), the
//! hit flash 0x13f53e in the glow, and the sound triggers of the advances (`PlayClassSound` pitch draws, the sound layer).
//!
//! **The joint modifiers** (Ratchet's moby +0x64, docs/plan/hero_gameplay.md §7): the records the springs update are
//! linked into [`Idle::manips`] while active (`AttachManipulator` / `DetachManipulator`), with the eyelid nodes of the
//! blink ([`BLINK_NODES`]); [`Idle::modifiers`] is the list as the evaluator reads it (written to his moby by the
//! write-back). The walk / air lean `HeroLean` 0x235638 is [`Hero::lean`].
#![allow(clippy::neg_cmp_op_on_partial_ord)] // compare semantics spelled out as in the game.

use super::physics::{fast_sin, ticks, turn_spring};
use super::states::Ctx;
use super::Hero;
use crate::ps2v::Pf;
use crate::rng::Rng;
use rc_formats::moby_anim::{self, AnimState, JointModifier, MobyAnimClass, MobyFrame};
use std::sync::Arc;

/// A fidget record (0x179d10 + k·0x70, the fields from +0x44; level01 data, the same on every level).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FidgetDef {
    /// +0x44: the fidget is a state (`SetState(0x40)`) rather than a sequence.
    pub state: bool,
    /// +0x48: Ratchet's sequence.
    pub seq: u8,
    /// +0x50: required hand item (`0x22ddd8(0)`), −1 = any.
    pub item: i32,
    /// +0x58: mean interval in seconds (chance `1 / (int)(s·60)` per tick; 0 = never).
    pub mean_s: f32,
    /// +0x5c: the record's own cooldown in seconds (`(int)(s·60)` ticks into +0x60).
    pub cooldown_s: f32,
}

/// The five fidget records. 0/1: sequences 1 and 2, the Novalis idle fidgets. 2: a state fidget with chance 0
/// (never picked). 3: sequence 0x53, only on level 0xc with nothing 8 units above the body point. 4: sequence
/// 0x5a, only within 50 ticks of a walk at health ≤ 1 (and the health-1 path of `0x241e00`).
pub const FIDGETS: [FidgetDef; 5] = [
    FidgetDef { state: false, seq: 1, item: -1, mean_s: 10.0, cooldown_s: 5.5 },
    FidgetDef { state: false, seq: 2, item: -1, mean_s: 10.0, cooldown_s: 5.5 },
    FidgetDef { state: true, seq: 0x63, item: -1, mean_s: 0.0, cooldown_s: 8.0 },
    FidgetDef { state: false, seq: 0x53, item: -1, mean_s: 8.0, cooldown_s: 5.0 },
    FidgetDef { state: false, seq: 0x5a, item: -1, mean_s: 4.0, cooldown_s: 2.5 },
];

/// The back-item sequence tables `(Ratchet's seq, pack seq, Clank seq)`, chosen by the back item id
/// (`0x22ddd8(3)`): 2 → 0x17c070 (the Clank pack, Novalis), 3 → 0x17c050, 4 → gp−0x7548 (0x15f6b8).
pub const BACK_TABLE_2: [(i16, u8, u8); 5] = [(0, 4, 11), (0, 3, 10), (0, 7, 14), (21, 5, 12), (18, 8, 15)];
pub const BACK_TABLE_3: [(i16, u8, u8); 6] = [(0, 3, 3), (0, 4, 4), (0, 8, 5), (0, 5, 6), (0, 6, 7), (42, 7, 8)];
pub const BACK_TABLE_4: [(i16, u8, u8); 2] = [(101, 3, 16), (101, 4, 17)];

/// Blend lengths of the eased curves (gp−0x7520) as `SetAnim` passes them to the back items: −1 = 11 ticks,
/// −2 = 16; the third word is the float 48.0's bits read as an integer (the game's value, not a tick count).
pub const CURVE_TICKS: [i32; 3] = [11, 16, 0x4240_0000];

/// Ratchet's blink values per frame (0x17c610) and Clank's (0x17c720): the eyelid manipulators' +0xc.
pub const BLINK: [f32; 12] = [0.0, 0.25, 0.5, 0.75, 1.0, 1.0, 1.0, 0.8, 0.6, 0.4, 0.2, 0.0];
pub const CLANK_BLINK: [f32; 22] =
    [0.0, 0.2, 0.4, 0.6, 0.7, 0.8, 0.9, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0];

const fn f(bits: u32) -> f32 { f32::from_bits(bits) }

/// `FastDecTimer` (0x220e78 int / 0x220ea8 s16): 1 when the timer was already 0, 2 when it reaches 0 now,
/// else 0 (a timer at 0 counts as expired on every call).
pub fn dec_timer(t: &mut i32) -> i32 {
    if *t == 0 { return 1; }
    *t = (*t).max(1) - 1;
    if 0 < *t { 0 } else { 2 }
}
pub fn dec_timer_s16(t: &mut i16) -> i32 {
    if *t == 0 { return 1; }
    *t = (*t).max(1) - 1;
    if 0 < *t { 0 } else { 2 }
}

/// One joint-manipulator record of the block at 0x17ab00 (stride 0xb0). Its first 0x40 bytes are the joint-modifier
/// node `FUN_00227050` links into its moby's `+0x64` list while the record is active ([`JointRec::modifier`],
/// [`Manip`]): Ratchet's for kind 0, Clank's for kind 1 (`FUN_00226ff8(kind)`: 0x1413d0 / 0x1404d4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointRec {
    /// Record index (address 0x17ab00 + 0xb0·rec).
    pub rec: u8,
    /// +0xa0: the moby's class joint list the node is attached for (its target joint: the list's second byte list's
    /// first entry, `rc_formats::moby_anim::list_target`).
    pub joint: i16,
    /// +0xa2: the kind (0 Ratchet, 1 Clank; the port models no other kind).
    pub kind: u8,
    /// +0x01: manipulator attached.
    pub attached: bool,
    /// +0x40 / +0x50 / +0x60: Euler angles, their spring velocities, this tick's targets (cleared after the
    /// spring).
    pub cur: [f32; 3],
    pub vel: [f32; 3],
    pub target: [f32; 3],
    /// +0x70 / +0x80 / +0x90: the translation (the node's +0x30), its spring velocities and this tick's targets
    /// (cleared after the spring).
    pub trans: [f32; 3],
    pub trans_vel: [f32; 3],
    pub trans_target: [f32; 3],
    /// +0xa4 / +0xa8: spring stiffness and damping.
    pub k: f32,
    pub d: f32,
    /// +0xac: scale (reset to 1 after the spring).
    pub scale: f32,
    /// Node +0x20..+0x28: the scale of the last active update (`+0xac` before its reset).
    pub node_scale: f32,
}

impl JointRec {
    const fn new(rec: u8, joint: i16, k: u32, d: u32) -> JointRec { JointRec::of_kind(rec, joint, 0, k, d) }

    pub(crate) const fn of_kind(rec: u8, joint: i16, kind: u8, k: u32, d: u32) -> JointRec {
        JointRec {
            rec, joint, kind, attached: false, cur: [0.0; 3], vel: [0.0; 3], target: [0.0; 3], trans: [0.0; 3], trans_vel: [0.0; 3],
            trans_target: [0.0; 3], k: f(k), d: f(d), scale: 1.0, node_scale: 1.0,
        }
    }

    /// The record's joint-modifier node (mode 0, composing): the quaternion `FUN_0026ee30(+0x10, +0x40)` builds from
    /// the angles, [`euler_quat`], the scale `+0xac` of the update that attached or refreshed it and the translation
    /// +0x70 (the node's +0x30). `target` = the joint of the record's list.
    pub fn modifier(&self, target: u8) -> JointModifier {
        let s = self.node_scale;
        JointModifier { quat: euler_quat(self.cur), scale: [s, s, s], trans: self.trans, ..JointModifier::compose(target) }
    }

    /// `0x227050` in mode 0 for a record whose moby exists: when anything is set or still moving (angle targets or
    /// translation targets ≠ 0, |translation| ≥ 0.003, scale ≠ 1, |angle| ≥ 0.005), the angular spring `0x270b58(target,
    /// k, d, 0, &angle, &vel, 2)` on x, y, z, the linear spring `0x270780(target, k, d, 0, &trans, &vel)` on x, y, z and
    /// the manipulator is attached; otherwise it is detached (the angles stay). Then the targets are cleared and the
    /// scale reset. (The option 0x15edb5's mirror of x / z is not ported: options.)
    pub(crate) fn update(&mut self) {
        let small = |v: f32| v.abs() < f(0x3ba3_d70a);
        let still = |v: f32| v.abs() < f(0x3b44_9ba6);
        let active = self.target.iter().any(|&t| t != 0.0)
            || self.trans_target.iter().any(|&t| t != 0.0)
            || !self.trans.iter().all(|&x| still(x))
            || self.scale != 1.0
            || !self.cur.iter().all(|&c| small(c));
        if active {
            for i in 0..3 {
                let (mut a, mut v) = (Pf::f(self.cur[i]), Pf::f(self.vel[i]));
                turn_spring(Pf::f(self.target[i]), Pf::f(self.k), Pf::f(self.d), Pf::ZERO, &mut a, &mut v, 2);
                self.cur[i] = a.to_f32();
                self.vel[i] = v.to_f32();
            }
            for i in 0..3 {
                let (mut x, mut v) = (Pf::f(self.trans[i]), Pf::f(self.trans_vel[i]));
                super::physics::spring(Pf::f(self.trans_target[i]), Pf::f(self.k), Pf::f(self.d), Pf::ZERO, &mut x, &mut v);
                self.trans[i] = x.to_f32();
                self.trans_vel[i] = v.to_f32();
            }
            self.attached = true;
            self.node_scale = self.scale;
        } else {
            self.attached = false;
        }
        self.target = [0.0; 3];
        self.trans_target = [0.0; 3];
        self.scale = 1.0;
    }
}

/// Indices into [`Idle::joints`] (in record order: the springs `0x2273d0` update the block in that order).
pub mod joint {
    /// Record 0 (list 9): the walk lean's x (`HeroLean` 0x235638); the grind's body lean x / y (L00 0x217970).
    pub const REC0: usize = 0;
    /// Record 1 (list 10): follows the head look (×0.55 / ×0.52); the lean's z.
    pub const NECK: usize = 1;
    /// Record 2 (list 11): the lean's x.
    pub const REC2: usize = 2;
    /// Record 3 (list 4): the head look (0x17ad54 / 0x17ad58 = its y / z angles); the lean's x, y, z.
    pub const HEAD: usize = 3;
    /// Records 4 / 5 (lists 22 / 23): Ratchet's feet; scale 0.01 every tick a feet item is worn and the slope tilt
    /// (`0x22c5c0`, [`super::super::pose`]).
    pub const FOOT_L: usize = 4;
    pub const FOOT_R: usize = 5;
    /// Records 6 / 7 (lists 7 / 8): the legs' slope tilt (`0x22c5c0`).
    pub const LEG_L: usize = 6;
    pub const LEG_R: usize = 7;
    /// Record 12 (list 21): follows the head look (/2.8, ×0.25).
    pub const REC12: usize = 8;
    /// Records 13..16 (lists 25..28): the idle secondaries; the lean's y / z.
    pub const SECONDARY: usize = 9;
    /// Record 17 (list 24): scale from 0x15ee14.
    pub const REC17: usize = 13;
    /// Record 18 (kind 1: Clank's list 0): the walk's sway of Clank on the back (`0x235e60`).
    pub const CLANK0: usize = 14;
    /// The records the port models.
    pub const COUNT: usize = 15;
}

/// A node of Ratchet's joint-modifier list (moby `+0x64`) in the order the game links them (`AttachManipulator`
/// 0x264370 puts a node in front; `DetachManipulator` 0x2643e8 unlinks it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Manip {
    /// A joint record of the block 0x17ab00, by index into [`Idle::joints`].
    Rec(u8),
    /// Eyelid node k (0..7) of Ratchet's blink, 0x140080 + 0x40·k.
    Blink(u8),
}

/// Ratchet's eyelid nodes (`0x227590`): the joint lists 0x17c640 and the blended poses (quaternion 0x17c4c0, scale
/// 0x17c530, translation 0x17c5a0, 16 bytes each; level01 data), mode 1 with the weight `BLINK[frame]`.
pub const BLINK_NODES: [(u8, [f32; 3], [f32; 3]); 7] = [
    (15, [1.0, 1.0, 1.0], [f(0x4352_e1c0), f(0x42ab_40c7), f(0x4423_1d6f)]),
    (16, [1.0, 1.0, 1.0], [f(0x4352_e1c0), f(0xc2ab_40c7), f(0x4423_1d6f)]),
    (17, [1.0, 1.0, f(0x3f68_f5c3)], [f(0x4443_aacf), f(0x4401_8053), f(0xc402_357c)]),
    (18, [1.0, 1.0, f(0x3f68_f5c3)], [f(0x4443_aacf), f(0xc400_7095), f(0xc402_357c)]),
    (19, [1.0, 1.0, 1.0], [f(0x4476_43c3), f(0x440d_2d78), f(0x429d_1993)]),
    (20, [1.0, 1.0, 1.0], [f(0x4476_43c3), f(0xc40c_1dba), f(0x429d_1993)]),
    (21, [f(0x3f7a_e148), 1.0, 1.0], [f(0x439c_646a), f(0x4007_dec4), f(0x43f3_148a)]),
];

/// `FUN_00221e38(a, out, axis)`: the rotation quaternion about axis 0 / 1 / 2 by `−a` as the game builds it from the
/// full angle: `c = 2·cos a`, `s = −sin a`, `k = sqrt(|c| + 2)`; `c ≥ 0`: `w = k/2`, `v = s/k`; `c < 0`: `v = k/2`,
/// `w = s/k` (so `v ≥ 0` there); `v` on the axis.
pub fn axis_quat(a: f32, axis: usize) -> [f32; 4] {
    let (c, s) = (2.0 * a.cos(), -a.sin());
    let k = (c.abs() + 2.0).sqrt();
    let (v, w) = if c < 0.0 { (0.5 * k, s / k) } else { (s / k, 0.5 * k) };
    let mut q = [0.0, 0.0, 0.0, w];
    q[axis] = v;
    q
}

/// `FUN_0026ee30(out, e)`: Euler angles → quaternion `(q_x(e.x) ⊗ q_y(e.y)) ⊗ q_z(e.z)` ([`axis_quat`]).
pub fn euler_quat(e: [f32; 3]) -> [f32; 4] {
    use rc_formats::moby_anim::quat_product;
    quat_product(quat_product(axis_quat(e[0], 0), axis_quat(e[1], 1)), axis_quat(e[2], 2))
}

/// The idle fields of the hero block and the globals the idle routines keep.
#[derive(Clone, Debug)]
pub struct Idle {
    /// 0x15f5cc as the hero update sees it (the tick counter before its increment). The hero keeps a mirror
    /// that [`super::hero_update`] increments; the tick driver should overwrite it with the game's counter.
    pub counter: i32,
    /// 0x15ed84: the current level (−1 = unknown; fidget record 3 and the ground probe's water footstep read it).
    pub level: i32,
    /// 0x13f538 (s16): the fidget cooldown (ticks(120) at every fidget start; `HeroTickStateTimer` decrements it).
    pub cooldown: i16,
    /// 0x1415c0: the last fidget record picked.
    pub fidget: i32,
    /// +0x60 of the fidget records (0x179d70 + k·0x70): per-record cooldowns, decremented by `HeroTickStateTimer`.
    pub fidget_cool: [i32; 5],
    /// 0x140354 / 0x140358: head-look pitch (y) / yaw (z) targets; 0x140364 / 0x140368: their spring k / d.
    /// (The timer 0x140360 is [`Hero::fidget_timer`].)
    pub look_pitch: f32,
    pub look_yaw: f32,
    pub look_k: f32,
    pub look_d: f32,
    /// 0x1415c4: the head's look target (`HeroScanTargets`, `super::pose`): its moby index + 1, 0 none.
    pub look_target: i32,
    /// 0x1403b0..0x1403bc: the secondaries' timers; 0x140374 + 0x10·k / 0x140378 + 0x10·k: their y / z targets.
    pub sec_timer: [i32; 4],
    pub sec: [[f32; 2]; 4],
    /// 0x140340: Ratchet's blink frame (0 = not blinking, 2..11 while blinking); 0x140344: tick of the next
    /// blink; 0x140348: blink period (0x68 = 104 after SetState of 0, 2, 4, 6; 0 otherwise).
    pub blink: i32,
    pub blink_next: i32,
    pub blink_period: i32,
    /// 0x14034c (s16): Clank's blink timer; 0x14034e (s16): Clank's blink frame (1..21).
    pub clank_blink_timer: i16,
    pub clank_blink: i16,
    /// 0x141614: Clank's fidget timer.
    pub clank_fidget_timer: i32,
    /// 0x15ee14: record 17's scale source (approaches 0.92 by ≤ 0.05 per tick).
    pub rec17_scale: f32,
    /// Records 0..7, 12..18 of the joint block 0x17ab00 (the others have no writer in the port).
    pub joints: [JointRec; joint::COUNT],
    /// Ratchet's joint-modifier list (moby `+0x64`), head first: the attached joint records and eyelid nodes.
    pub manips: Vec<Manip>,
    /// 0x13f610..0x13f624: the ground probe's side probes (`super::pose`).
    pub side: super::pose::SideProbes,
    /// 0x1405f0..0x1405fe: the foot motes (`0x248920`, `super::pose`).
    pub motes: super::pose::FootMotes,
    /// Clank's moby +0x2c (his class scale; [`Hero::set_clank_scale`]): record 18's sway is in his joint units.
    pub clank_scale: f32,
}

impl Default for Idle {
    fn default() -> Self { Idle::new() }
}

impl Idle {
    /// Zeroed, with `HeroInit`'s 0x140364 / 0x140368 and the records' spring constants (level01 data).
    pub fn new() -> Idle {
        Idle {
            counter: 0,
            level: -1,
            cooldown: 0,
            fidget: 0,
            fidget_cool: [0; 5],
            look_pitch: 0.0,
            look_yaw: 0.0,
            look_k: f(0x3be5_6042),
            look_d: f(0x3e99_999a),
            look_target: 0,
            sec_timer: [0; 4],
            sec: [[0.0; 2]; 4],
            blink: 0,
            blink_next: 0,
            blink_period: 0,
            clank_blink_timer: 0,
            clank_blink: 0,
            clank_fidget_timer: 0,
            rec17_scale: f(0x3f6b_851f),
            joints: [
                JointRec::new(0, 9, 0, 0),
                JointRec::new(1, 10, 0x3c03_126f, 0x3e99_999a),
                JointRec::new(2, 11, 0, 0),
                JointRec::new(3, 4, 0x3be5_6042, 0x3e99_999a),
                JointRec::new(4, 22, 0x3dcc_cccd, 0x3e99_999a),
                JointRec::new(5, 23, 0x3dcc_cccd, 0x3e99_999a),
                JointRec::new(6, 7, 0x3dcc_cccd, 0x3e99_999a),
                JointRec::new(7, 8, 0x3dcc_cccd, 0x3e99_999a),
                JointRec::new(12, 21, 0x3df5_c28f, 0x3e99_999a),
                JointRec::new(13, 25, 0x3c75_c28f, 0x3e99_999a),
                JointRec::new(14, 26, 0x3c75_c28f, 0x3e99_999a),
                JointRec::new(15, 27, 0x3c54_fdf4, 0x3e80_0000),
                JointRec::new(16, 28, 0x3c8b_4396, 0x3e57_0a3d),
                JointRec::new(17, 24, 0x3cf5_c28f, 0x3e4c_cccd),
                JointRec::of_kind(18, 0, 1, 0, 0),
            ],
            manips: Vec::new(),
            side: Default::default(),
            motes: Default::default(),
            clank_scale: 1.0,
        }
    }

    /// `0x22b8e8` (SetState 0, the hand swap): clears the look angles and the secondaries' targets.
    pub fn clear_look(&mut self) {
        self.look_pitch = 0.0;
        self.look_yaw = 0.0;
        self.sec = [[0.0; 2]; 4];
    }

    /// Record 3's current (y, z) angles: 0x17ad54 / 0x17ad58.
    pub fn head(&self) -> (f32, f32) {
        let h = &self.joints[joint::HEAD];
        (h.cur[1], h.cur[2])
    }

    /// Ratchet's eyelid value this tick (`BLINK[frame]`, 0 when not blinking).
    pub fn blink_value(&self) -> f32 { if self.blink == 0 { 0.0 } else { BLINK[self.blink as usize % 12] } }

    /// `AttachManipulator`: link `m` in front of the list (a node already linked stays where it is).
    fn attach(&mut self, m: Manip) {
        if !self.manips.contains(&m) { self.manips.insert(0, m); }
    }

    /// `DetachManipulator`: unlink `m`.
    fn detach(&mut self, m: Manip) { self.manips.retain(|&x| x != m); }

    /// `FUN_00227420` (`SwitchCharacter` and leaving a body, `super::bodies`): every record of the block 0x17ab00
    /// whose manipulator is attached is detached from its moby (`FUN_00226ff8(kind)`), and its angles +0x40, angle
    /// targets +0x60 and translation targets +0x90 are zeroed, scale +0xac = 1 (the translation +0x70 and the
    /// spring velocities are kept). Ratchet's eyelid nodes are not records (left as they are).
    pub(crate) fn detach_all(&mut self) {
        for r in self.joints.iter_mut() {
            r.attached = false;
            r.cur = [0.0; 3];
            r.target = [0.0; 3];
            r.trans_target = [0.0; 3];
            r.scale = 1.0;
        }
        self.manips.retain(|m| !matches!(m, Manip::Rec(_)));
    }

    /// Ratchet's joint-modifier list (moby `+0x64`) as the evaluator reads it, head first, each node's joint list
    /// resolved through `targets` (per class joint list, its target joint; 0xff or missing: the node is dropped).
    pub fn modifiers(&self, targets: &[u8]) -> Vec<JointModifier> {
        let target = |list: i16| targets.get(list as usize).copied().filter(|&t| t != 0xff);
        self.manips.iter().filter_map(|&m| match m {
            Manip::Rec(i) => {
                let r = &self.joints[i as usize];
                Some(r.modifier(target(r.joint)?))
            }
            Manip::Blink(k) => {
                let (list, scale, trans) = BLINK_NODES[k as usize];
                let joint = target(list as i16)?;
                Some(JointModifier { joint, mode: 1, weight: self.blink_value(), quat: [0.0, 0.0, 0.0, 1.0], scale, trans })
            }
        }).collect()
    }
}

// ------------------------------------------------------------------------------------------------
// The back items.

/// The anim classes of the back items: the pack moby of each back item (`CreateMoby` of the item definition's
/// class +0x10: 2 Heli-Pack 607, 3 Thruster-Pack 608, 4 Hydro-Pack 609, read from level01's definitions at
/// 0x179f40) and Clank (item 1's 601).
#[derive(Clone, Debug)]
pub struct BackClasses {
    /// `(item id, o_class, class)`.
    pub packs: Vec<(i32, i16, MobyAnimClass)>,
    pub clank: MobyAnimClass,
}

impl BackClasses {
    /// The pack class of back item `id`.
    pub fn pack(&self, id: i32) -> Option<(i16, &MobyAnimClass)> { self.packs.iter().find(|p| p.0 == id).map(|p| (p.1, &p.2)) }
}

/// An item slot's bookkeeping besides its moby (the records at 0x1403e0 + 0x50·slot and the per-slot globals of
/// `UpdateWrenchSelected(slot)` 0x2307e0 at 0x141408 / 0x141424 / 0x141440 / 0x14145c / 0x141660 + 4·slot).
/// [`Hero::back_slot`] is slot 3 (the back: pack and Clank); it exists whether or not the back mobys are
/// modelled, so `GetClankModule(3)` ([`Hero::back_module`]) answers without class data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemSlot {
    /// +0x24: 2 ready, 3 being put away, 0 empty (before the first hero update, or after a put-away).
    pub state: i32,
    /// +0x28: the item id (slot 3: 2 Heli-Pack, 3 Thruster-Pack, 4 Hydro-Pack).
    pub id: i32,
    /// 0x141424 + 4·slot: the item the next creation takes (0: none).
    pub target: i32,
    /// 0x141440 / 0x14145c + 4·slot: the item to restore (0x26 = nothing) and its request.
    pub restore: i32,
    pub restore_pending: i32,
    /// 0x141408 + 4·slot: the request (slot 3: `SessionState::temp_back` 0x141414, what `GiveItem` with equip
    /// writes); 0x141660 + 4·slot: the saved item (slot 3: `equipped[3]` 0x14166c). The engine syncs both.
    pub request: i32,
    pub saved: i32,
    /// +0x20: ticks ready (3 when a swap starts); +0x1b: put-away ticks.
    pub ticks_ready: i32,
    pub putaway_ticks: u8,
}

impl ItemSlot {
    /// The common part of `UpdateWrenchSelected(slot)` for the slots besides the hand: the restore request
    /// (0x14145c + 4·slot → target = saved = the restore item, 0x26 meaning none), then the request 0x141408 +
    /// 4·slot (0x26: nothing; the item is saved unless `keep_unsaved` names it). True when the slot starts a swap
    /// (ticks ready = 3, request cleared); the caller puts the item away (slot 3: [`Hero::back_swap`]).
    pub fn swap_requests(&mut self, keep_unsaved: Option<i32>) -> bool {
        let s = self;
        let mut changed = false;
        if s.restore_pending != 0 {
            s.restore_pending = 0;
            let mut v = s.restore;
            if v != 0 && v != s.id {
                if v == 0x26 { v = 0; }
                s.restore = 0;
                s.saved = v;
                s.target = v;
                changed = true;
            }
        }
        let r = s.request;
        if r != 0 {
            if r == s.target {
                s.request = 0;
            } else {
                if r == 0x26 {
                    s.target = 0;
                    s.saved = 0;
                } else {
                    s.target = r;
                    if Some(r) != keep_unsaved { s.saved = r; }
                }
                changed = true;
            }
        }
        if !changed { return false; }
        s.ticks_ready = 3;
        s.request = 0;
        true
    }

}

/// Item slot 3 with the globals only the back reads: 0x15ed94 (the Thruster-Pack was the last back item,
/// `GameState::global.thruster_last`) and 0x141628 (Clank hidden, `SessionState::clank_hidden`; it hides
/// the back mobys and disables every pack move). Synced by the engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackSlot {
    pub slot: ItemSlot,
    pub thruster_last: i32,
    pub clank_hidden: i16,
}

/// A back moby's animation (moby+0x50..0x70) and its snapshot frame.
#[derive(Clone, Debug)]
pub struct BackMoby {
    pub anim: AnimState,
    pub snapshot: Option<MobyFrame>,
}

/// Item slot 3 (0x1404d0): the pack moby (+0x00), Clank (+0x04), state (+0x24 = 0x1404f4), item id (+0x28) —
/// the modelled back mobys; `state` / `id` follow [`Hero::back_slot`].
#[derive(Clone, Debug)]
pub struct Back {
    pub state: i32,
    pub id: i32,
    /// The pack moby's item and class (the engine draws that model; `id` can differ while the slot is empty).
    pub pack_item: i32,
    pub pack_o_class: i16,
    pub pack: BackMoby,
    pub clank: BackMoby,
    /// Clank's moby+0x90 colour word (the pulsing glow of `0x2278c0`).
    pub clank_color: u32,
    pub classes: Arc<BackClasses>,
}

impl Back {
    /// The table of `0x2476d0` / `0x247488` (`0x22ddd8(3)`: the id while the slot state is 2).
    pub fn table(&self) -> &'static [(i16, u8, u8)] {
        if self.state != 2 { return &[]; }
        match self.id {
            2 => &BACK_TABLE_2,
            3 => &BACK_TABLE_3,
            4 => &BACK_TABLE_4,
            _ => &[],
        }
    }

    /// `0x2476d0(seq)`: the rows of Ratchet's sequence `seq`; with n > 0 of them, `rand_range(0, n − 1)` picks
    /// one: `(pack seq, Clank seq)`. None (no draw) without rows.
    pub fn pick(&self, seq: u8, rng: &mut Rng) -> Option<(u8, u8)> {
        let t = self.table();
        let rows: Vec<usize> = (0..t.len()).filter(|&i| t[i].0 == seq as i16).collect();
        if rows.is_empty() { return None; }
        let r = rng.rand_range(0, rows.len() as i32 - 1);
        let e = t[rows[r as usize]];
        Some((e.1, e.2))
    }

    /// `0x247488(pack_seq)`: Ratchet's sequence of the first row whose pack sequence is `pack_seq`.
    pub fn owner(&self, pack_seq: u8) -> Option<i16> { self.table().iter().find(|e| e.1 == pack_seq).map(|e| e.0) }

    /// `MobyAnimBlend(pack, seq, frame, ticks)` (0x26c660).
    pub fn blend_pack(&mut self, seq: u8, frame: i32, ticks: i32) {
        let Some((_, c)) = self.classes.pack(self.pack_item) else { return };
        moby_anim::set_sequence(&mut self.pack.anim, c, seq, frame, ticks, &mut self.pack.snapshot);
    }
    pub fn blend_clank(&mut self, seq: u8, frame: i32, ticks: i32) {
        let c = &self.classes.clank;
        moby_anim::set_sequence(&mut self.clank.anim, c, seq, frame, ticks, &mut self.clank.snapshot);
    }
}

// ------------------------------------------------------------------------------------------------

/// `fast_add_rotations` / `fast_subtract_rotations`: an angle wrapped into [−π, π].
fn wrap_angle(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut a = a % TAU;
    if PI < a { a -= TAU } else if a < -PI { a += TAU }
    a
}

/// `0x2705a8(a, b, t)`: `a + (b − a)·t`.
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

impl Hero {
    /// `0x22ddd8(0)`: the hand item while the hand slot is ready (state 2), else −1.
    pub fn held_item(&self) -> i32 { if self.items.slot.state == 2 { self.items.slot.id } else { -1 } }

    /// `FUN_002487a8`'s hand rule: the hand moby hidden (mode |= 0x41) while 0x1413fe holds with the wrench in the hand
    /// slot (0x140408 = 8: the water, the ledge, [`super::items::items_flags`]) or while 0x1413ff holds (the gold
    /// bolt's pickup).
    pub fn hand_hidden(&self) -> bool { (self.items.f13fe != 0 && self.items.slot.id == 8) || self.f13ff != 0 }

    /// `GetClankModule(3)` (0x22ddd8(3)): the back item while slot 3 is ready (state 2), else −1.
    pub fn back_module(&self) -> i32 { if self.back_slot.slot.state == 2 { self.back_slot.slot.id } else { -1 } }

    /// Give the hero the back items' classes (the pack of back item 2 and Clank): they are created on the next
    /// hero update, as `HeroItemsCreate` does on the first one. Without them the back is not modelled (no back
    /// table draws, no Clank fidget or blink, no back swaps: the slot keeps the item it was created with).
    /// More packs: [`Hero::add_back_pack`].
    pub fn set_back_classes(&mut self, pack: MobyAnimClass, clank: MobyAnimClass) { self.set_back_packs(vec![(2, 607, pack)], clank); }

    /// [`Hero::set_back_classes`] with every pack the level has: `(back item id, o_class, class)`.
    pub fn set_back_packs(&mut self, packs: Vec<(i32, i16, MobyAnimClass)>, clank: MobyAnimClass) {
        self.back_classes = Some(Arc::new(BackClasses { packs, clank }));
    }

    /// The pack moby class of back item `id` (`o_class` = the item definition's +0x10), after
    /// [`Hero::set_back_classes`].
    pub fn add_back_pack(&mut self, id: i32, o_class: i16, class: MobyAnimClass) {
        let Some(c) = self.back_classes.as_mut() else { return };
        let c = Arc::make_mut(c);
        c.packs.retain(|p| p.0 != id);
        c.packs.push((id, o_class, class));
    }

    /// Test / debug helper: the saved back item 0x14166c (`equipped[3]`) and, once the slot exists, the slot's
    /// item itself (no put-away).
    pub fn equip_back(&mut self, id: i32) {
        let s = &mut self.back_slot.slot;
        s.saved = id;
        if s.state != 0 { s.id = id; s.state = 2; }
        if let Some(b) = self.back.as_mut() {
            if let Some((o, c)) = b.classes.pack(id).map(|(o, c)| (o, c.clone())) {
                b.pack = BackMoby { anim: AnimState::spawn(&c), snapshot: None };
                (b.pack_item, b.pack_o_class) = (id, o);
            }
            (b.state, b.id) = (s.state, s.id);
        }
    }

    /// `0x247800` (the end of Ratchet's advance): in mode 0 with the back ready, a table row for Ratchet's
    /// sequence copies his playback speed 0x13fde0 into both back mobys' speed (+0x58).
    pub(super) fn back_follow_speed(&mut self, seq_b: u8, rng: &mut Rng) {
        if self.mode != 0 { return; }
        let speed = self.anim_speed.to_f32();
        let Some(b) = self.back.as_mut() else { return };
        if b.state != 2 { return; }
        if b.pick(seq_b, rng).is_some() {
            b.pack.anim.speed = speed;
            b.clank.anim.speed = speed;
        }
    }

    /// `0x247550(blend, seq, frame)` (from `SetAnim` in mode 0): with the back ready, a table row for the new
    /// sequence (≠ 0) blends pack and Clank to that row's sequences unless the pack already plays it; otherwise
    /// both return to sequence 1 over 7 ticks (19 after state 8 with pack 2), except in group 0 while the pack
    /// plays one of sequence 0's rows (a Clank fidget runs on).
    pub(super) fn back_follow_anim(&mut self, blend: i32, seq: u8, frame: i32, rng: &mut Rng) {
        let (group, prev_state) = (self.group, self.prev_state);
        let Some(b) = self.back.as_mut() else { return };
        if b.state != 2 { return; }
        let pick = b.pick(seq, rng);
        if let (Some((p, q)), true) = (pick, seq != 0) {
            if b.pack.anim.seq_b != p {
                b.blend_pack(p, frame, blend);
                b.blend_clank(q, frame, blend);
            }
            return;
        }
        if group == 0 && b.owner(b.pack.anim.seq_b) == Some(0) { return; }
        let t = if prev_state == 8 && b.id == 2 { ticks(19) } else { ticks(7) };
        if b.pack.anim.seq_b != 1 { b.blend_pack(1, 0, t); }
        if b.clank.anim.seq_b != 1 { b.blend_clank(1, 0, t); }
    }

    /// `HeroItemsCreate` 0x22f3c0 / `HeroItemsAttach` 0x22fec0 / the slot loop 0x231088 for slot 3 (after the
    /// write-back). Creation: an empty slot takes the target 0x141430, else the saved back item 0x14166c, else 2
    /// (3 when 0x15ed94) — back item 2 on a new game — state 2; the pack moby is the item's class, Clank is created
    /// once. Then both advance every tick (`MobyAnimAdvance`, sound triggers not modelled) and the slot loop runs
    /// [`Hero::back_slot_loop`]. Without the classes only the slot's bookkeeping exists (created, never swapped).
    pub(super) fn back_items_update(&mut self, rng: &mut Rng) {
        let s = &mut self.back_slot.slot;
        let created = s.state == 0;
        if created {
            let mut id = s.target;
            if id == 0 {
                id = s.saved;
                if id == 0 { id = if self.back_slot.thruster_last != 0 { 3 } else { 2 }; }
            }
            s.id = id;
            s.state = 2;
        }
        let (state, id) = (s.state, s.id);
        if let Some(c) = self.back_classes.clone() {
            let pack = c.pack(id).map(|(o, pc)| (o, BackMoby { anim: AnimState::spawn(pc), snapshot: None }));
            match (self.back.as_mut(), pack) {
                (None, Some((o, pack))) => {
                    let clank = BackMoby { anim: AnimState::spawn(&c.clank), snapshot: None };
                    self.back = Some(Back { state, id, pack_item: id, pack_o_class: o, pack, clank, clank_color: 0, classes: c });
                }
                (Some(b), Some((o, pack))) if created => {
                    (b.pack, b.pack_item, b.pack_o_class) = (pack, id, o);
                }
                _ => {}
            }
        }
        if let Some(b) = self.back.as_mut() {
            (b.state, b.id) = (state, id);
            let c = b.classes.clone();
            if let Some((_, pc)) = c.pack(b.pack_item) { moby_anim::advance(&mut b.pack.anim, pc); }
            moby_anim::advance(&mut b.clank.anim, &c.clank);
            self.back_slot_loop(rng);
        }
    }

    /// The slot loop 0x231088 for slot 3 with the back modelled. Ready (2): `UpdateWrenchSelected(3)`
    /// ([`Hero::back_swap`]); a wrapped pack on sequence 0 blends to 1 (2 ticks). Put away (3): once the pack's
    /// put-away animation wraps the pack is deleted (`0x2305e8`: the slot is empty; the next hero update creates
    /// the target item). Then the pack moby's own update (+0x74, [`Hero::back_pack_update`]).
    pub(super) fn back_slot_loop(&mut self, rng: &mut Rng) {
        self.back_slot_states(rng);
        self.back_pack_update();
    }

    /// The pack moby's update `(*moby+0x74)(moby)` in the slot loop: only the Heli-Pack's class 607 has one
    /// (`ClankPackUpdate` 0x2f30c0: in the glide 8 the rotor sequence 6, blended over 10 ticks); 608 / 609 have
    /// none (docs/plan/moby_update_catalogue.md).
    fn back_pack_update(&mut self) {
        let state = self.state;
        let Some(b) = self.back.as_mut() else { return };
        if b.state == 0 { return; }
        if b.pack_o_class == 607 && state == 8 && b.pack.anim.seq_b != 6 { b.blend_pack(6, 0, ticks(10)); }
    }

    fn back_slot_states(&mut self, rng: &mut Rng) {
        match self.back_slot.slot.state {
            2 => {
                self.back_slot.slot.ticks_ready += 1;
                self.back_swap(rng);
                let Some(b) = self.back.as_mut() else { return };
                if b.pack.anim.flags & 2 != 0 && b.pack.anim.seq_b == 0 { b.blend_pack(1, 0, 2); }
            }
            3 => {
                let s = &mut self.back_slot.slot;
                s.putaway_ticks = s.putaway_ticks.wrapping_add(1);
                let Some(b) = self.back.as_mut() else { return };
                if b.pack.anim.flags & 2 != 0 {
                    (s.state, s.id) = (0, 0);
                    (b.state, b.id) = (0, 0);
                }
            }
            _ => {}
        }
    }

    /// `UpdateWrenchSelected(3)` 0x2307e0, the back's rules: in the water groups (`0x22dea8`: groups 0x11 / 0x12,
    /// states 0x6a / 0x82 / 0x75 / 0x76) with the Hydro-Pack owned the back becomes the Hydro-Pack (request 4, the
    /// current item kept to restore; 4 is not saved); out of the water (not 0x12) the Hydro-Pack restores that
    /// item; the glide 8 with the Hydro-Pack saved requests the Heli-Pack. Then the slots' common swap
    /// ([`Hero::slot_swap`]).
    fn back_swap(&mut self, rng: &mut Rng) {
        let water = self.in_water_groups();
        let hydro = self.owned.has(super::swim::ITEM_HYDRO_PACK);
        let s = &mut self.back_slot.slot;
        let mut keep_unsaved = None;
        if !water && self.state != 0x12 {
            if s.id == 4 && s.restore != 0 { s.restore_pending = 1; }
        } else if s.saved != 4 && s.id != 4 && hydro {
            s.request = 4;
            keep_unsaved = Some(4);
            s.restore = s.id;
        }
        if self.state == 8 && s.saved == 4 { s.request = 2; }
        if !s.swap_requests(keep_unsaved) { return; }
        // Slot 3's part of the swap: 0x15ed94 from the item being put away; the pack plays its put-away
        // (sequence 2, 2 ticks); Clank is not blended.
        self.slot_swap_effects(rng);
        self.back_slot.thruster_last = (self.back_slot.slot.id == 3) as i32;
        if let Some(b) = self.back.as_mut() {
            b.blend_pack(2, 0, 2);
            b.state = 3;
        }
        self.back_slot.slot.state = 3;
    }

    /// What every slot's swap does besides its own slot (`UpdateWrenchSelected` 0x2307e0 when a swap starts):
    /// `FUN_0022b8e8` (head look and idle secondaries cleared), the fidget timer 0x140360 = `rand_range(50, 90)`,
    /// and a raised weapon put away (`0x1413f8` set → `0x22efd8`, `super::weapons::put_away`).
    pub(super) fn slot_swap_effects(&mut self, rng: &mut Rng) {
        self.idle.clear_look();
        self.fidget_timer = rng.rand_range(ticks(50), ticks(90));
        super::weapons::put_away(self);
    }

    /// `FUN_0022dea8`: in the water groups 0x11 / 0x12 or the states 0x6a / 0x82 / 0x76 / 0x75.
    pub fn in_water_groups(&self) -> bool { matches!(self.group, 0x11 | 0x12) || matches!(self.state, 0x6a | 0x82 | 0x75 | 0x76) }

    // --------------------------------------------------------------------------------------------
    // The idle transitions (0x242930 state 0, the part before StickTarget).

    /// State 0 of `0x242930` after the strafe check and the loop clear: the item-0x1b rule, the back items'
    /// return to sequence 1, the fidget or the return to the idle sequence, Clank's fidget.
    pub(super) fn idle_anim(&mut self, c: &mut Ctx) {
        let idle = self.idle_seq();
        let mut can_fidget = true;
        if self.held_item() == 0x1b {
            if c.anim.view().seq_b != idle { self.set_anim(c.anim, c.rng, Pf::b(0xc000_0000), idle, 0); }
            can_fidget = false;
        }
        // No fidgets on a slippery floor (0x140632, level00; super::surface).
        if super::surface::slippery(self) { can_fidget = false; }
        let (head_y, head_z) = self.idle.head();
        if let Some(b) = self.back.as_mut() {
            if b.state == 2 {
                if b.pack.anim.flags & 2 != 0 && b.pack.anim.seq_b != 1 { b.blend_pack(1, 0, ticks(7)); }
                if b.clank.anim.flags & 2 != 0 && b.clank.anim.seq_b != 1 { b.blend_clank(1, 0, ticks(7)); }
            }
            if f(0x3f1c_61aa) < head_z || f(0x3edf_66f3) < head_y {
                if b.pack.anim.seq_b != 1 { b.blend_pack(1, 0, ticks(12)); }
                if b.clank.anim.seq_b != 1 { b.blend_clank(1, 0, ticks(12)); }
            }
        }
        let v = c.anim.view();
        if v.seq_b == idle && can_fidget {
            if let Some(k) = self.fidget_chance(c, v.flags) {
                self.idle.fidget = k as i32;
                self.idle.cooldown = ticks(120) as i16;
                let d = FIDGETS[k];
                self.idle.fidget_cool[k] = (d.cooldown_s * 60.0) as i32;
                if d.state {
                    self.set_state(c, 0x40, true);
                } else {
                    self.set_anim(c.anim, c.rng, Pf::from_i32(ticks(12)), d.seq, 0);
                }
            }
        } else if v.flags & 2 != 0 {
            // A weapon with the arm raised keeps its stance (0x1413f8 / 0x1413fa with 0x1415e4, 0x140058).
            let keep = super::weapons::stance_kept(self, v.seq_b);
            if !keep {
                self.f13f8 = 0;
                if idle == 0x54 {
                    self.set_anim(c.anim, c.rng, Pf::from_i32(ticks(18)), 0x54, 0);
                } else {
                    self.set_anim(c.anim, c.rng, Pf::b(0xc000_0000), idle, 0);
                }
            }
        }
        // Clank's fidget: after 10 ticks in idle, Clank on sequence 1, the head not turned far.
        let (head_y, head_z) = self.idle.head();
        let clank_ready = self.back.as_ref().is_some_and(|b| b.clank.anim.seq_b == 1);
        if ticks(10) < self.timer
            && clank_ready
            && dec_timer(&mut self.idle.clank_fidget_timer) != 0
            && head_z < f(0x3f0e_fa35)
            && head_y < f(0x3eb2_b8c2)
            && self.back.as_ref().is_some_and(|b| b.state == 2)
        {
            self.idle.clank_fidget_timer = c.rng.rand_range(ticks(110), ticks(270));
            self.clank_fidget(c.rng);
        }
        // The weapon's standing stance while it is out with the arm raised (super::weapons).
        super::weapons::idle_stance(self, c);
    }

    /// `0x241e00`: the fidget chance. Returns the record picked, or None. Draws one `randf(0, 1)` whenever a
    /// record is available (or on a wrap in the health-1 path).
    fn fidget_chance(&mut self, c: &mut Ctx, anim_flags: u8) -> Option<usize> {
        if self.idle.cooldown != 0 { return None; }
        if self.health == 1 {
            if self.idle.fidget_cool[4] != 0 || anim_flags & 2 == 0 { return None; }
            return (c.rng.randf(0.0, 1.0) < f(0x3ea8_f5c3)).then_some(4);
        }
        let mut cands = [0usize; 5];
        let mut n = 0;
        for (k, d) in FIDGETS.iter().enumerate() {
            if self.idle.fidget_cool[k] != 0 || d.mean_s == 0.0 { continue; }
            let item = self.held_item();
            if d.item != -1 && d.item != item { continue; }
            if d.state && self.ground_moby.is_some() { continue; }
            if k == 3 {
                if self.idle.level != 0xc || self.held_item() == 0x17 { continue; }
                let a = self.body_point;
                let mut b = a;
                b[2] = a[2] + Pf::b(0x4100_0000);
                if c.env.line(a, b, 2).is_some() { continue; }
            }
            if k == 4 && (self.prev_group != 1 || ticks(50) < self.timer || 1 < self.health) { continue; }
            cands[n] = k;
            n += 1;
        }
        if n == 0 { return None; }
        let chance = |k: usize| 1.0 / ((FIDGETS[k].mean_s * 60.0) as i32) as f32;
        let mut sum = 0.0f32;
        for &k in &cands[..n] { sum += chance(k); }
        let r = c.rng.randf(0.0, 1.0);
        if !(r < sum) { return None; }
        let mut pick = cands[0];
        for &k in &cands[..n] {
            pick = k;
            sum -= chance(k);
            if sum < r { break; }
        }
        Some(pick)
    }

    /// `0x2473e0`: with the pack on sequence 1, a random row of sequence 0 for pack and Clank (7-tick blends).
    fn clank_fidget(&mut self, rng: &mut Rng) {
        let Some(b) = self.back.as_mut() else { return };
        if b.pack.anim.seq_b != 1 { return; }
        if let Some((p, q)) = b.pick(0, rng) {
            b.blend_pack(p, 0, ticks(7));
            b.blend_clank(q, 0, ticks(7));
        }
    }

    // --------------------------------------------------------------------------------------------
    // The sub-updates after the transitions (mode 0).

    /// `0x22c5c0`, `0x22b928`, `HeroScanTargets`, `0x22bdd0`, `0x227590`, `0x2278c0`, `0x2352e0` and the joint springs
    /// `0x2273d0`, in the hero update's order (`counter` = 0x15f5cc; Ratchet's sequence B after the transitions from
    /// `anim`). (`0x22b700`, the arm lowered at a wall, record 9: not ported, 0x141618 is never set.)
    pub(super) fn idle_updates(&mut self, env: &super::physics::Env, anim: &dyn super::AnimCtl, counter: i32, rng: &mut Rng) {
        let seq_b = anim.view().seq_b;
        self.feet_update(anim, rng);
        self.head_look(seq_b, rng);
        self.scan_targets(env, seq_b, counter);
        self.idle_secondaries(seq_b, rng);
        self.blink_update(seq_b, counter, rng);
        self.clank_glow_blink(counter, rng);
        self.magnet_lean();
        // 0x2273d0: each record in order; an active one is (re)attached, an idle one detached. Ratchet's records link
        // into his list; record 18 (kind 1) into Clank's, and only while Clank's moby exists (`FUN_00226ff8(1)` = 0:
        // the record is left as it is).
        let clank = self.back.is_some();
        let i = &mut self.idle;
        for k in 0..i.joints.len() {
            if i.joints[k].kind != 0 {
                if clank { i.joints[k].update(); }
                continue;
            }
            i.joints[k].update();
            let m = Manip::Rec(k as u8);
            if i.joints[k].attached { i.attach(m) } else { i.detach(m) }
        }
    }

    /// `HeroLean` 0x235638 (called by the physics of 2 / 0x73, 6 / 0x2d, the jump group and 0x65 on foot, 8 and 0x81):
    /// the body lean from the turn residual 0x13f4d8 as joint-record targets (records 0..3 and 13..16, their springs
    /// run in [`Hero::idle_updates`]) and the spring constants of records 0..3. State 2: substate 1 (running) leans by
    /// the clamped residual (±1.4) and, for record 0, by the speed `|eff.xy| / (5.7·dt)`; otherwise by
    /// `1.25·r·|r|` of half the residual (±0.8). 8 / 0x81 (pack glide / Thruster hover): softer springs and bigger
    /// angles (0x81: record 0 also from the drift of the displacement against the facing). Groups 2 / 4 (falling,
    /// jumping): as running, without record 0. Native `f32`.
    pub(super) fn lean(&mut self) {
        use joint::{HEAD, NECK, REC0, REC2, SECONDARY};
        let r = self.yaw_residual.to_f32();
        let dt = super::physics::DT.to_f32();
        let (state, substate, group) = (self.state, self.substate, self.group);
        let (speed, disp, yaw) = (self.eff_len_xy.to_f32(), self.disp, self.rot[2].to_f32());
        let j = &mut self.idle.joints;
        let springs = |j: &mut [JointRec; joint::COUNT], kd: [(u32, u32); 4]| {
            for (i, (k, d)) in [REC0, NECK, REC2, HEAD].into_iter().zip(kd) { (j[i].k, j[i].d) = (f(k), f(d)); }
        };
        let body = |j: &mut [JointRec; joint::COUNT], z13: f32, z14: f32| {
            j[SECONDARY].target[2] = z13;
            for k in 1..4 { j[SECONDARY + k].target[2] = z14; }
        };
        let walk_kd = [(0x3d23_d70a, 0x3e4c_cccd), (0x3d23_d70a, 0x3e4c_cccd), (0x3d23_d70a, 0x3e4c_cccd), (0x3ca3_d70a, 0x3e4c_cccd)];
        if state == 2 {
            springs(j, walk_kd);
            if substate == 1 {
                let v = r.clamp(-1.4, 1.4);
                let sp = (speed / (dt * 5.7)).clamp(0.0, 1.0);
                j[REC0].target[0] = -v * 0.14 * sp;
                j[NECK].target[2] = v * 0.16;
                j[REC2].target[0] = v * -0.12;
                j[HEAD].target = [v.abs() * 0.16, v.abs() * -0.15, v * 0.65];
                body(j, -v * 0.35, -v * 0.3);
                j[SECONDARY].target[1] = v.abs() * 0.35;
                j[SECONDARY + 1].target[1] = v.abs() * 0.2;
            } else {
                let h = (r * 0.5).clamp(-0.8, 0.8);
                let v = h * h.abs() * 1.25;
                j[NECK].target[2] = v;
                j[HEAD].target[2] = v;
                j[HEAD].target[1] = v.abs() * 0.25;
                body(j, -v * 0.35, -v * 0.3);
            }
        } else if state == 8 || state == 0x81 {
            springs(j, [(0x3c03_126f, 0x3da3_d70a), (0x3c75_c28f, 0x3da3_d70a), (0x3c75_c28f, 0x3da3_d70a), (0x3c03_126f, 0x3dcc_cccd)]);
            let v = if state == 0x81 { (r * 0.8).clamp(-1.1, 1.1) } else { (r * 1.6).clamp(-1.25, 1.25) };
            j[REC0].target[0] = (-v * 0.52).clamp(-0.28, 0.28);
            j[HEAD].target[2] = (v * 1.2).clamp(-0.6, 0.6);
            j[NECK].target[2] = v * 0.58;
            j[REC2].target[0] = v * -0.28;
            j[HEAD].target[1] = (v.abs() * 0.2).clamp(-0.2, 0.2);
            j[HEAD].target[0] = v.abs() * 0.16;
            if state == 0x81 {
                if dt * 0.5 < speed {
                    let a = wrap_angle(disp[1].to_f32().atan2(disp[0].to_f32()) - yaw);
                    let k = speed * 3.0;
                    let lim = f(0x3edf_66f3);
                    j[REC0].target[0] = (-a.sin() * k).clamp(-lim, lim);
                    j[REC0].target[1] = (a.cos() * k).clamp(-lim, lim);
                }
                j[REC0].target[1] = wrap_angle(j[REC0].target[1] + f(0xbdfa_35dd));
            }
            body(j, -v * 0.35, -v * 0.3);
        } else if group == 2 || group == 4 {
            springs(j, walk_kd);
            let v = r.clamp(-1.4, 1.4);
            j[SECONDARY].target[2] = -v * 0.35;
            j[NECK].target[2] = v * 0.29;
            j[REC2].target[0] = v * -0.27;
            j[HEAD].target[2] = v * 0.87;
            for k in 1..4 { j[SECONDARY + k].target[2] = -v * 0.3; }
        }
    }

    /// `0x22b928`: record 17's scale; in state 0 on sequence 0..2 or 0x3a with no look target, the head-look
    /// timer 0x140360 (re-armed to 40..70 every tick of the fidgets 1/2, so it only expires in sequence 0: then
    /// a new pitch `−randf(−0.157, 0.436)`, yaw `randf_sym(0.349, 0.995)`, spring constants from `k = randf(0, 1)`
    /// and the timer `rand_range(90, 200) + (int)(80·k)`); the fidgets clamp the look to ±20° / 0..15°. Record 3
    /// gets the look, record 1 55 % / 52 % of it.
    fn head_look(&mut self, seq_b: u8, rng: &mut Rng) {
        // The Clank cheat 0x15edb3 with Clank on the back (0x1404d4): record 18's scale = 0x15ee18.
        if self.cheats.on(crate::cheats::slot::CLANK) && self.back.is_some() { self.idle.joints[joint::CLANK0].scale = self.bodies.scale18; }
        let rec17 = &mut self.idle.joints[joint::REC17];
        rec17.scale = self.idle.rec17_scale;
        // Approach(0.92, 0.05, &0x15ee14) (1.57 with the cheat 0x15edb1, "Ratchet has a big head").
        let t = if self.cheats.on(crate::cheats::slot::RATCHET) { f(0x3fc8_f5c3) } else { f(0x3f6b_851f) };
        let step = f(0x3d4c_cccd);
        let mut d = t - self.idle.rec17_scale;
        if step < d { d = step; } else if d < -step { d = -step; }
        self.idle.rec17_scale += d;
        // The edge look-down (a drop over 1.4 ahead, no wall within 1): the head (spring 0.015 / 0.3) pitches down by
        // 1.5° per unit of drop + 11°, at most 27° and at most 35° × (1 − the room before the edge); the neck (0.008 /
        // 0.3) by 0.7 of it.
        let (edge, depth, room) = self.edge;
        if edge && 1.4 < depth && 1.0 < self.wall_ahead[0] {
            let deg = f(0x3c8e_fa35);
            let mut y = (depth * 1.5 * deg + f(0x3e44_9809)).min(f(0x3ef1_4639));
            let cap = (1.0 - room) * 35.0 * deg;
            if cap < y { y = cap; }
            let i = &mut self.idle;
            (i.joints[joint::HEAD].k, i.joints[joint::HEAD].d) = (f(0x3c75_c28f), f(0x3e99_999a));
            i.joints[joint::HEAD].target[1] = y;
            (i.joints[joint::NECK].k, i.joints[joint::NECK].d) = (f(0x3c03_126f), f(0x3e99_999a));
            i.joints[joint::NECK].target[1] = y * f(0x3f33_3333);
            return;
        }
        if self.state != 0 || !(seq_b < 3 || seq_b == 0x3a) || self.idle.look_target != 0 { return; }
        let fidget = seq_b.wrapping_sub(1) < 2;
        if fidget { self.fidget_timer = rng.rand_range(ticks(40), ticks(70)); }
        if dec_timer(&mut self.fidget_timer) != 0 {
            let i = &mut self.idle;
            i.look_pitch = -rng.randf(f(0xbe20_d97c), f(0x3edf_66f3));
            i.look_yaw = rng.randf_sym(f(0x3eb2_b8c2), f(0x3f7e_adae));
            let k = rng.randf(0.0, 1.0);
            i.look_k = lerp(f(0x3c75_c28f), f(0x3be5_6042), k);
            i.look_d = lerp(f(0x3e94_7ae1), f(0x3e99_999a), k);
            self.fidget_timer = rng.rand_range(ticks(90), ticks(200));
            self.fidget_timer += (ticks(80) as f32 * k) as i32;
        }
        let i = &mut self.idle;
        if fidget {
            let m = f(0x3eb2_b8c2);
            if m < i.look_yaw { i.look_yaw = m; } else if i.look_yaw < -m { i.look_yaw = -m; }
            if f(0x3e86_0a92) < i.look_pitch { i.look_pitch = f(0x3e86_0a92); }
            if i.look_pitch < 0.0 { i.look_pitch = 0.0; }
        }
        let head = &mut i.joints[joint::HEAD];
        head.k = i.look_k;
        head.d = i.look_d;
        head.target[1] = i.look_pitch;
        head.target[2] = i.look_yaw;
        let (hy, hz) = (head.target[1], head.target[2]);
        let neck = &mut i.joints[joint::NECK];
        neck.k = f(0x3c03_126f);
        neck.d = f(0x3e99_999a);
        neck.target[2] = hz * f(0x3f05_1eb8);
        neck.target[1] = hy * f(0x3f0c_cccd);
    }

    /// `0x22bdd0` (state 0, sequence 0): four timers; on expiry new targets for records 13..16 (y of record 16
    /// `randf(−0.087, 0.524)`, z `randf_sym` ranges per record, record 16's z alternating sign) and a new
    /// timer; every tick the targets are written.
    fn idle_secondaries(&mut self, seq_b: u8, rng: &mut Rng) {
        if self.state != 0 || seq_b != 0 { return; }
        let i = &mut self.idle;
        for k in 0..4 {
            if dec_timer(&mut i.sec_timer[k]) != 0 {
                match k {
                    0 => {
                        i.sec[0][1] = rng.randf_sym(f(0x3e32_b8c2), f(0x3f06_0a92));
                        i.sec_timer[0] = rng.rand_range(ticks(70), ticks(150));
                    }
                    1 => {
                        i.sec[1][1] = rng.randf_sym(f(0x3e32_b8c2), f(0x3f06_0a92));
                        i.sec_timer[1] = rng.rand_range(ticks(40), ticks(90));
                    }
                    2 => {
                        i.sec[2][1] = rng.randf_sym(f(0x3e86_0a92), f(0x3f5f_66f3));
                        i.sec_timer[2] = rng.rand_range(ticks(40), ticks(90));
                    }
                    _ => {
                        i.sec[3][0] = rng.randf(f(0xbdb2_b8c2), f(0x3f06_0a92));
                        let old = i.sec[3][1];
                        let new = rng.randf_sym(f(0x3eb2_b8c2), f(0x3f75_be0b));
                        i.sec[3][1] = new;
                        let flip = (old <= 0.0 && new <= 0.0) || (0.0 <= old && 0.0 <= new);
                        if flip { i.sec[3][1] = -new; }
                        i.sec_timer[3] = rng.rand_range(ticks(35), ticks(70));
                    }
                }
            }
            let r = &mut i.joints[joint::SECONDARY + k];
            r.target[1] = i.sec[k][0];
            r.target[2] = i.sec[k][1];
        }
    }

    /// `0x227590`: record 12 follows the head look (z ×0.25 within ±8.5°, y /2.8 within −10°..17°; ±2° / ±7° in
    /// the fidgets 1/2), then Ratchet's blink (`0x2274e8`: with a period, a blink starts once the counter passes
    /// 0x140344 and schedules the next at `counter + 20 + randi(p) + randi(p)`; without, the next is pushed to
    /// `counter + 30`) and its frame 1 → 11.
    fn blink_update(&mut self, seq_b: u8, counter: i32, rng: &mut Rng) {
        let i = &mut self.idle;
        let (hy, hz) = { let h = &i.joints[joint::HEAD]; (h.target[1], h.target[2]) };
        let mut z = hz * 0.25;
        let zm = f(0x3e17_e9d8);
        if zm < z { z = zm; } else if z < -zm { z = -zm; }
        let mut y = hy / f(0x4033_3333);
        if f(0x3e97_e9d8) < y { y = f(0x3e97_e9d8); }
        if y < f(0xbe32_b8c2) { y = f(0xbe32_b8c2); }
        if seq_b.wrapping_sub(1) < 2 {
            let m = f(0x3d0e_fa35);
            if m < z { z = m; }
            if z < -m { z = -m; }
            let m = f(0x3dfa_35dd);
            if m < y { y = m; }
            if y < -m { y = -m; }
        }
        let r12 = &mut i.joints[joint::REC12];
        r12.target[2] = z;
        r12.target[1] = y;
        // 0x2274e8.
        if i.blink == 0 {
            if i.blink_period == 0 {
                i.blink_next = counter + ticks(30);
            } else if i.blink_next < counter {
                i.blink = 1;
                let a = ticks(20);
                let b = rng.randi(i.blink_period);
                let c = rng.randi(i.blink_period);
                i.blink_next = counter + a + b + c;
            }
        }
        if i.blink != 0 {
            i.blink += 1;
            if 12 <= i.blink { i.blink = 0; }
            // The eyelid nodes: all detached at the end, else each attached (in front, node 0 first) if it is not.
            for k in 0..BLINK_NODES.len() as u8 {
                if i.blink == 0 { i.detach(Manip::Blink(k)) } else { i.attach(Manip::Blink(k)) }
            }
        }
    }

    /// `0x2278c0` in mode 0 on Clank (docs/plan/hero_gameplay.md §19): nothing while Clank is hidden (0x141628); the glow
    /// colour (+0x90, a ticks(110) sine pulse; red while Ratchet's hit flash 0x13f53e runs) and Clank's blink (timer
    /// 0x14034c re-armed to 50..200 when it has run out and Clank is on sequence 1; frames 1..21: the eyelid nodes
    /// 0x140240.., [`Hero::clank_modifiers`]).
    fn clank_glow_blink(&mut self, counter: i32, rng: &mut Rng) {
        // Approach(1.0, 0.05, &0x15ee18) (1.8 with the cheat 0x15edb3, "Clank has a large noggin"), every tick.
        super::bodies::approach_head_scale(self);
        if self.back_slot.clank_hidden != 0 { return; }
        let (health, flash) = (self.health, self.f53e);
        let Some(b) = self.back.as_mut() else { return };
        b.clank_color = clank_glow_word(counter, health, flash);
        let i = &mut self.idle;
        if dec_timer_s16(&mut i.clank_blink_timer) != 0 && b.clank.anim.seq_b == 1 {
            i.clank_blink = 1;
            i.clank_blink_timer = rng.rand_range(ticks(50), ticks(200)) as i16;
        }
        if i.clank_blink != 0 {
            i.clank_blink += 1;
            if 0x16 <= i.clank_blink { i.clank_blink = 0; }
        }
    }

    /// Clank's joint-modifier list (his moby's +0x64): record 18 while attached (`0x235e60`'s sway, `super::pose`), and
    /// while he blinks the four eyelid nodes 0x140240 + 0x40·k on his
    /// joint lists 2..5 (gp−0x7508), node 3 first (attached 0, 1, 2, 3, each in front), mode 1, weight
    /// `CLANK_BLINK[frame]`, the poses 0x17c660 / 0x17c6a0 / 0x17c6e0 (the translation × Clank's scale +0x2c,
    /// `clank_scale`); empty when he is not blinking (all detached at frame 0). `targets` = Clank's class joint lists'
    /// targets (`rc_formats::moby_anim::list_target`).
    pub fn clank_modifiers(&self, targets: &[u8], clank_scale: f32) -> Vec<JointModifier> {
        let target = |list: i16| targets.get(list as usize).copied().filter(|&t| t != 0xff);
        // Record 18 (Clank's list 0, `0x235e60`'s sway) while attached; linked before the eyelids here [L: the
        // nodes turn different joints, so their order in the list does not matter].
        let r = &self.idle.joints[joint::CLANK0];
        let mut out: Vec<JointModifier> = if r.attached && self.back.is_some() { target(r.joint).map(|t| r.modifier(t)).into_iter().collect() } else { Vec::new() };
        let k = self.idle.clank_blink;
        if k == 0 || self.back.is_none() { return out; }
        let w = CLANK_BLINK[k as usize % CLANK_BLINK.len()];
        out.extend(CLANK_EYELIDS.iter().rev().filter_map(|&(list, quat, scale, trans)| {
            let joint = targets.get(list as usize).copied().filter(|&t| t != 0xff)?;
            Some(JointModifier { joint, mode: 1, weight: w, quat, scale, trans: trans.map(|x| x * clank_scale) })
        }));
        out
    }

}

/// `0x2278c0`'s glow word (moby +0x90 of Clank: on Ratchet's back in mode 0, the hero moby itself as Clank or Giant
/// Clank, `super::bodies`) at tick `counter` (0x15f5cc): a ticks(110) sine s; red `base + (int)(24s) + 0xc` (base 0x38,
/// 0x88 at health 1), green `(int)(48s) + 0xa0`, blue `(int)(24s) + 0x4c`, alpha 0x80; the hit flash 0x13f53e
/// (`flash`) fades the pulse toward red over its first 5 ticks (of 45) and back over its last 20.
pub fn clank_glow_word(counter: i32, health: i32, flash: i16) -> u32 {
    let base = if health == 1 { 0x88 } else { 0x38 };
    let p = ticks(110);
    let ph = (counter % p) as f32 / p as f32;
    let s = fast_sin(Pf::f((ph + ph) * std::f32::consts::PI + -std::f32::consts::PI)).to_f32();
    let v = s * 24.0;
    let (vi, gi) = (v as i32, (s * 48.0) as i32);
    let (mut r, mut g, mut bl) = (base + vi + 0xc, gi + 0xa0, vi + 0x4c);
    if flash != 0 {
        // The hit flash: fading in over its first 5 ticks (of 45), out over its last 20, toward red.
        let (t5, t20, t45) = (ticks(5), ticks(20), ticks(45));
        let mut t = 1.0f32;
        if t45 - t5 < flash as i32 { t = (t45 - flash as i32) as f32 / t5 as f32; }
        if (flash as i32) < t20 { t = flash as f32 / t20 as f32; }
        bl -= ((vi + 0xc) as f32 * t) as i32;
        r = (r - ((vi + 0xc) as f32 * t) as i32) + (t * 112.0) as i32;
        g = (g - ((gi + 0x18) as f32 * t) as i32) - (t * 72.0) as i32;
    }
    (0x80u32 << 24) | ((bl as u32 & 0xff) << 16) | ((g as u32 & 0xff) << 8) | (r as u32 & 0xff)
}

/// The glow word of Clank's antenna moby 1204 (`HeroItemsAttach` 0x22fec0) at tick `counter` (0x15f5cc): a
/// ticks(120) sine s, red `min(0xd7 + 90s, 0xff)`, green `0x32 + 50s`, blue `0x14 + 10s`, alpha 0x80 (its creation's
/// 0x801432d7 is the value at s = 0).
pub fn antenna_glow(counter: i32) -> u32 {
    let p = ticks(120);
    let ph = counter.rem_euclid(p) as f32 / p as f32;
    let s = fast_sin(Pf::f((ph + ph) * std::f32::consts::PI + -std::f32::consts::PI)).to_f32();
    let r = ((s * 90.0) as i32 + 0xd7).min(0xff);
    let g = (s * 50.0) as i32 + 0x32;
    let b = (s * 10.0) as i32 + 0x14;
    ((b as u32 & 0xff) << 16) | 0x8000_0000 | ((g as u32 & 0xff) << 8) | (r as u32 & 0xff)
}

/// Clank's eyelid nodes (`0x2278c0`): (joint list (gp−0x7508), quaternion 0x17c660, scale 0x17c6a0, translation
/// 0x17c6e0; level01 data).
/// One eyelid node: (joint list, quaternion, scale, translation).
pub type EyelidNode = (u8, [f32; 4], [f32; 3], [f32; 3]);
pub const CLANK_EYELIDS: [EyelidNode; 4] = [
    (2, [f(0x3e18_f712), f(0xbd4b_48d4), f(0x3ea1_a60d), f(0x3f6f_88b9)], [1.0; 3], [f(0x44e2_1948), f(0x43d5_7c29), f(0x44be_451f)]),
    (3, [f(0x3e25_f84d), f(0xbd63_5e74), f(0x3ea1_0e02), f(0x3f6f_0111)], [1.0; 3], [f(0x44e3_a51f), f(0x43cd_63d7), f(0x450f_5852)]),
    (4, [f(0x3a12_ccf7), f(0x3b3c_be62), f(0x3ee1_0a13), f(0x3f65_efc8)], [1.0; 3], [f(0x44e2_1948), f(0xc3d5_7c29), f(0x44be_451f)]),
    (5, [f(0xba6b_edfa), f(0xbb91_2989), f(0x3edf_a2f0), f(0x3f66_464a)], [1.0; 3], [f(0x44e3_a51f), f(0xc3cd_63d7), f(0x450f_5852)]),
];

#[cfg(test)]
mod tests {
    use super::super::testkit::{cam_x, floor};
    use super::super::{hero_update, AnimCtl, AnimView, Env, Hero, RecordingAnim};
    use super::*;
    use crate::moby_runtime::Moby;
    use crate::pad::{PadInput, PadState};
    use rc_formats::collision::Collision;

    /// Draws between two stream states.
    fn draws(from: Rng, to: Rng) -> usize {
        let mut r = from;
        for n in 0..100_000 {
            if r.state == to.state { return n; }
            r.rand();
        }
        panic!("streams do not meet");
    }

    /// [`RecordingAnim`] whose non-idle sequences wrap (flag 2) `len` ticks after their `set_anim`.
    struct WrapAnim {
        rec: RecordingAnim,
        len: u32,
        t: u32,
    }

    impl AnimCtl for WrapAnim {
        fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32) {
            self.rec.set_anim(blend, seq, frame);
            self.t = 0;
        }
        fn advance(&mut self, speed: Pf) {
            self.rec.advance(speed);
            self.t += 1;
            if self.rec.v.seq_b != 0 && self.t == self.len { self.rec.v.flags |= 2; }
        }
        fn view(&self) -> AnimView { self.rec.view() }
        fn frame_count(&self, s: u8) -> u8 { self.rec.frame_count(s) }
        fn set_loop(&mut self, a: i32, b: i32) { self.rec.set_loop(a, b); }
        fn clear_loop(&mut self) { self.rec.clear_loop(); }
    }

    /// The joint-modifier list: a blink links the seven eyelid nodes in front (node 6 first, as `AttachManipulator`
    /// prepends in the order 0..6), weighted by `BLINK[frame]`, and unlinks them at its end; active joint records are
    /// linked (and sent to Ratchet's moby by the write-back), idle ones unlinked.
    #[test]
    fn modifier_list_order_and_blink() {
        let mut t = Idler::new(99, 150);
        t.hero.fidget_timer = 100_000;
        t.hero.idle.sec_timer = [100_000; 4];
        t.hero.idle.cooldown = 1000;
        t.hero.idle.blink_period = 0x68;
        t.hero.idle.blink_next = -1;
        t.hero.idle.counter = 500;
        let targets: Vec<Vec<u8>> = (0..32u8).map(|l| vec![l + 100]).collect();
        t.hero.set_joint_targets(&targets);
        t.tick();
        // The springs run after the blink: record 17 (its scale approaching 0.92) is linked in front of the eyelids.
        let m = &t.hero.idle.manips;
        let mut want = vec![Manip::Rec(joint::REC17 as u8)];
        want.extend((0..7u8).rev().map(Manip::Blink));
        assert_eq!(&m[..8], &want[..], "{m:?}");
        let mods = t.hero.idle.modifiers(&t.hero.joint_targets);
        assert_eq!((mods[1].joint, mods[1].mode, mods[1].weight), (121, 1, 0.5));
        assert_eq!(t.moby.joint_mods, mods, "the write-back carries the list");
        for _ in 3..=12 { t.tick(); }
        assert_eq!(t.hero.idle.blink, 0);
        assert!(!t.hero.idle.manips.iter().any(|x| matches!(x, Manip::Blink(_))));
        // A record at rest with nothing set is unlinked; a scale ≠ 1 links it again, in front.
        let rec17 = t.hero.idle.joints[joint::REC17];
        assert!(rec17.attached, "record 17 approaches 0.92");
        assert_eq!(t.hero.idle.modifiers(&t.hero.joint_targets).iter().find(|m| m.joint == 124).map(|m| m.scale[0]), Some(rec17.node_scale));
    }

    /// `FUN_00221e38` turns by −a about its axis; `FUN_0026ee30` composes x, then y, then z.
    #[test]
    fn euler_node_quaternions() {
        let q = axis_quat(0.3, 2);
        assert!((q[2] + (0.15f32).sin()).abs() < 1e-7 && (q[3] - (0.15f32).cos()).abs() < 1e-7);
        // Past 90° the game's other branch: the same rotation with the vector part ≥ 0.
        let q = axis_quat(-2.5, 0);
        assert!(q[0] >= 0.0 && ((q[0] * q[0] + q[3] * q[3]) - 1.0).abs() < 1e-6);
        assert!((q[0] - (1.25f32).sin()).abs() < 1e-6 && (q[3] - (1.25f32).cos()).abs() < 1e-6);
        // Record 1 at the Novalis idle savestate: angles (0, −0.004922, 0.186369) → RAM (−0.000229, 0.00245, −0.09305, 0.995658).
        let q = euler_quat([0.0, -0.004_922_098, 0.186_369_34]);
        let ram = [-0.000_229_000_01, 0.002_450_368_8, -0.093_049_56, 0.995_658_3];
        assert!(q.iter().zip(ram).all(|(a, b)| (a - b).abs() < 1e-6), "{q:?}");
    }

    /// `HeroLean` while running (state 2, substate 1): the targets of records 0..3 and 13..16 from the residual.
    #[test]
    fn running_lean_targets() {
        let mut h = Hero::new();
        (h.state, h.substate) = (2, 1);
        h.yaw_residual = Pf::f(0.5);
        h.eff_len_xy = super::super::physics::DT * Pf::f(5.7);
        h.lean();
        let j = &h.idle.joints;
        let near = |a: f32, b: f32| (a - b).abs() < 1e-6;
        assert!(near(j[joint::REC0].target[0], -0.07) && near(j[joint::NECK].target[2], 0.08) && near(j[joint::REC2].target[0], -0.06));
        assert!(near(j[joint::HEAD].target[0], 0.08) && near(j[joint::HEAD].target[1], -0.075) && near(j[joint::HEAD].target[2], 0.325));
        assert!(near(j[joint::SECONDARY].target[2], -0.175) && near(j[joint::SECONDARY + 3].target[2], -0.15));
        assert!(near(j[joint::HEAD].k, 0.02) && near(j[joint::REC0].k, 0.04));
        // Clamped at ±1.4.
        h.yaw_residual = Pf::f(3.0);
        h.lean();
        assert!(near(h.idle.joints[joint::HEAD].target[2], 1.4 * 0.65));
    }

    struct Idler {
        hero: Hero,
        moby: Moby,
        pad: PadState,
        anim: WrapAnim,
        rng: Rng,
        coll: Collision,
    }

    impl Idler {
        fn new(seed: u32, fidget_len: u32) -> Idler {
            let coll = floor(100.0, 100, 106, 100, 106);
            let hero = Hero::spawn([410.0, 410.0, 100.0], 0.0);
            let mut moby = Moby::zeroed();
            moby.position = hero.pos.map(Pf::to_f32);
            let mut rng = Rng::new();
            rng.srand(seed);
            let anim = WrapAnim { rec: RecordingAnim::default(), len: fidget_len, t: 0 };
            let mut pad = PadState::default();
            pad.update(Some(&PadInput::neutral().bytes()), false);
            Idler { hero, moby, pad, anim, rng, coll }
        }

        /// One idle tick; returns the draws it made.
        fn tick(&mut self) -> usize {
            self.pad.update(Some(&PadInput::neutral().bytes()), false);
            let (cam_rows, cam_yaw) = cam_x();
            let env = Env { coll: &self.coll, pad: &self.pad, cam_yaw, cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
            let before = self.rng;
            hero_update(&mut self.hero, &mut self.moby, &env, &mut self.anim, &mut self.rng);
            draws(before, self.rng)
        }
    }

    /// The fidget chance on its own: with the look, secondaries and blink out of the way, an idle tick draws
    /// one `randf(0, 1)`, and a fidget starts on the first tick whose value is below 1/600 + 1/600 (record 0
    /// above 1/600, else record 1), with the 120-tick global and 330-tick record cooldowns.
    #[test]
    fn fidget_chance_and_cooldowns() {
        for seed in [1234u32, 7, 99, 2024] {
            let mut t = Idler::new(seed, 150);
            t.hero.fidget_timer = 100_000;
            t.hero.idle.sec_timer = [100_000; 4];
            let mut started = None;
            for n in 0..20_000 {
                let mut probe = t.rng;
                let r = probe.randf(0.0, 1.0);
                let d = t.tick();
                if t.anim.rec.v.seq_b != 0 {
                    // The start tick: the chance, then the look timer's re-arm in the new sequence.
                    assert_eq!(d, 2, "seed {seed} tick {n}: fidget start");
                    let sum = 1.0f32 / 600.0 + 1.0 / 600.0;
                    assert!(r < sum, "fidget without a winning draw ({r})");
                    let want = if 1.0f32 / 600.0 < r { 1 } else { 2 };
                    assert_eq!(t.anim.rec.v.seq_b, want, "record by draw {r}");
                    started = Some(n);
                    break;
                }
                assert_eq!(d, 1, "seed {seed} tick {n}: one randf per idle tick");
                assert!(!(r < 2.0 / 600.0), "a winning draw {r} started no fidget");
            }
            let n0 = started.expect("no fidget in 20000 ticks");
            let k = t.hero.idle.fidget as usize;
            assert_eq!(t.anim.rec.calls.last().copied(), Some((Pf::from_i32(12), FIDGETS[k].seq, 0)));
            assert_eq!(t.hero.idle.cooldown, 120);
            assert_eq!(t.hero.idle.fidget_cool[k], 330);
            // The fidget (150 ticks here): the look timer is re-armed every tick (one draw), no chance draw.
            for i in 1..150 {
                assert_eq!(t.tick(), 1, "fidget tick {i}");
                assert!((39..=69).contains(&t.hero.fidget_timer), "look timer {}", t.hero.fidget_timer);
            }
            // The wrap: back to the idle sequence (eased curve −2); the global cooldown is over (150 > 120), the
            // other record is free: one chance draw per tick again, with the other record's chance only.
            t.tick();
            assert_eq!(t.anim.rec.calls.last().copied(), Some((Pf::b(0xc000_0000), 0, 0)));
            assert_eq!(t.hero.idle.cooldown, 0);
            assert_eq!(t.hero.idle.fidget_cool[k], 330 - 150);
            let mut probe = t.rng;
            let r = probe.randf(0.0, 1.0);
            assert_eq!(t.tick(), 1);
            if r < 1.0 / 600.0 { assert_eq!(t.anim.rec.v.seq_b, FIDGETS[1 - k].seq); }
            eprintln!("seed {seed}: fidget record {k} after {n0} ticks");
        }
    }

    /// Short fidgets: no chance draw until the 120-tick cooldown has run out.
    #[test]
    fn cooldown_blocks_the_chance_draw() {
        let mut t = Idler::new(4321, 40);
        t.hero.fidget_timer = 100_000;
        t.hero.idle.sec_timer = [100_000; 4];
        let mut n = 0;
        while t.anim.rec.v.seq_b == 0 {
            t.tick();
            n += 1;
            assert!(n < 50_000);
        }
        // 39 fidget ticks (1 look draw each), the wrap on the 40th (the blend back to the idle sequence happens in
        // the transitions, so the look sees sequence 0: no draw), then idle with the cooldown running: no draws
        // until it reaches 0.
        let mut counts = Vec::new();
        for _ in 0..130 { counts.push(t.tick()); }
        assert!(counts[..39].iter().all(|&d| d == 1), "{counts:?}");
        assert_eq!(counts[39], 0, "wrap tick");
        // The cooldown was set to 120 on the start tick; post-move counts it down before the transitions. The only
        // other draws: the look timer (re-armed to 40..70 by the fidget) running out once (4 draws).
        assert!(counts[40..119].iter().all(|&d| d == 0 || d == 4), "{counts:?}");
        assert_eq!(counts[40..119].iter().filter(|&&d| d == 4).count(), 1, "{counts:?}");
        assert!(counts[119..].iter().all(|&d| d == 1), "{counts:?}");
    }

    /// Draws per idle tick with every consumer live: the first tick expires the look timer (4 draws) and the four
    /// secondaries (2 + 2 + 2 + 3) besides the chance (1); then one per tick until a timer runs out; a blink
    /// (period 0x68 after SetState(0)) draws two.
    #[test]
    fn draw_count_per_idle_tick() {
        let mut t = Idler::new(1234, 150);
        t.hero.fidget_timer = 0;
        assert_eq!(t.tick(), 1 + 4 + 9);
        let look = t.hero.fidget_timer;
        assert!((89..=280).contains(&look), "look timer {look}");
        let next_sec = *t.hero.idle.sec_timer.iter().min().unwrap();
        assert!(next_sec >= 34);
        for i in 1..next_sec.min(look) {
            let d = t.tick();
            if t.anim.rec.v.seq_b != 0 { break; }
            assert_eq!(d, 1, "tick {i}");
        }
        // Ratchet's blink: period 0x68, the next blink due now.
        let mut t = Idler::new(99, 150);
        t.hero.fidget_timer = 100_000;
        t.hero.idle.sec_timer = [100_000; 4];
        t.hero.idle.cooldown = 1000;
        t.hero.idle.blink_period = 0x68;
        t.hero.idle.blink_next = -1;
        t.hero.idle.counter = 500;
        let mut probe = t.rng;
        let (b1, b2) = (probe.randi(0x68), probe.randi(0x68));
        assert_eq!(t.tick(), 2);
        assert_eq!(t.hero.idle.blink, 2);
        assert_eq!(t.hero.idle.blink_next, 500 + 20 + b1 + b2);
        for f in 3..12 {
            assert_eq!(t.tick(), 0);
            assert_eq!(t.hero.idle.blink, f);
        }
        assert_eq!(t.tick(), 0);
        assert_eq!(t.hero.idle.blink, 0);
    }

    /// SetState(0) sets the blink period and clears the look; state 3 clears the period.
    #[test]
    fn set_state_blink_period() {
        let mut t = Idler::new(1, 150);
        let (cam_rows, cam_yaw) = cam_x();
        let env = Env { coll: &t.coll, pad: &t.pad, cam_yaw, cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
        let mut c = super::super::states::Ctx { env: &env, anim: &mut t.anim, rng: &mut t.rng, voice: None };
        t.hero.idle.look_yaw = 0.5;
        t.hero.set_state(&mut c, 0, true);
        assert_eq!(t.hero.idle.blink_period, 0x68);
        assert_eq!(t.hero.idle.look_yaw, 0.0);
        t.hero.set_state(&mut c, 7, true);
        assert_eq!(t.hero.idle.blink_period, 0);
    }

    #[test]
    fn back_table_pick() {
        let classes = Arc::new(BackClasses { packs: vec![(2, 607, MobyAnimClass { joint_count: 0, skeleton: vec![], rest: vec![], parent_word: vec![], sequences: vec![] })], clank: MobyAnimClass { joint_count: 0, skeleton: vec![], rest: vec![], parent_word: vec![], sequences: vec![] } });
        let m = || BackMoby { anim: AnimState { seq_a: 1, frame_a: 0, seq_b: 1, frame_b: 0, t: 0.0, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false }, snapshot: None };
        let b = Back { state: 2, id: 2, pack_item: 2, pack_o_class: 607, pack: m(), clank: m(), clank_color: 0, classes };
        let mut r = Rng::new();
        r.srand(1234);
        let mut p = r;
        let k = p.rand_range(0, 2);
        assert_eq!(b.pick(0, &mut r), Some([(4, 11), (3, 10), (7, 14)][k as usize]));
        assert_eq!(draws(p, r), 0);
        let s = r;
        assert_eq!(b.pick(1, &mut r), None);
        assert_eq!(draws(s, r), 0, "no rows, no draw");
        assert_eq!(b.pick(21, &mut r), Some((5, 12)));
        assert_eq!(draws(s, r), 1, "one row still draws rand_range(0, 0)");
        assert_eq!(b.owner(3), Some(0));
        assert_eq!(b.owner(1), None);
    }

    /// `0x2278c0` on Clank: the pulse, red on the hit flash, nothing while Clank is hidden; a blink links the four
    /// eyelid nodes (node 3 first, mode 1, weight `CLANK_BLINK[frame]`, translation × Clank's scale) until frame 0.
    #[test]
    fn clank_glow_flash_and_eyelids() {
        let empty = || MobyAnimClass { joint_count: 0, skeleton: vec![], rest: vec![], parent_word: vec![], sequences: vec![] };
        let classes = Arc::new(BackClasses { packs: vec![(2, 607, empty())], clank: empty() });
        let m = || BackMoby { anim: AnimState { seq_a: 1, frame_a: 0, seq_b: 1, frame_b: 0, t: 0.0, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false }, snapshot: None };
        let mut h = Hero::new();
        h.health = 3;
        h.back = Some(Back { state: 2, id: 2, pack_item: 2, pack_o_class: 607, pack: m(), clank: m(), clank_color: 0, classes });
        let mut r = Rng::new();
        // Counter 0: sin(−π) = 0 → (0x44, 0xa0, 0x4c).
        h.clank_glow_blink(0, &mut r);
        assert_eq!(h.back.as_ref().unwrap().clank_color, 0x804c_a044);
        // The blink started (timer 0 ran out on sequence 1): frame 2 after the increment; four nodes.
        assert_eq!(h.idle.clank_blink, 2);
        let targets = [0xff, 0xff, 10, 11, 12, 13];
        let mods = h.clank_modifiers(&targets, 0.5);
        assert_eq!(mods.iter().map(|n| n.joint).collect::<Vec<_>>(), vec![13, 12, 11, 10]);
        assert!(mods.iter().all(|n| n.mode == 1 && n.weight == CLANK_BLINK[2]));
        assert_eq!(mods[3].trans, CLANK_EYELIDS[0].3.map(|x| x * 0.5));
        // The hit flash at its peak (t = 1): (0x38 + 112, 0xa0 − 0x18 − 72, 0x4c − 0xc) = (0xa8, 0x40, 0x40).
        h.f53e = 30;
        h.clank_glow_blink(0, &mut r);
        assert_eq!(h.back.as_ref().unwrap().clank_color, 0x8040_40a8);
        // Frames run out at 0x16: detached.
        for _ in 0..30 { h.clank_glow_blink(0, &mut r); if h.idle.clank_blink == 0 { break; } }
        assert!(h.clank_modifiers(&targets, 0.5).is_empty());
        // Hidden: nothing changes.
        h.back_slot.clank_hidden = 1;
        h.back.as_mut().unwrap().clank_color = 7;
        h.clank_glow_blink(5, &mut r);
        assert_eq!(h.back.as_ref().unwrap().clank_color, 7);
    }

    /// The antenna moby's glow word: 0x801432d7 at the pulse's zero (tick 60 of 120), brightest a quarter later.
    #[test]
    fn antenna_glow_pulses() {
        assert_eq!(antenna_glow(60), 0x8014_32d7);
        assert_eq!(antenna_glow(180), 0x8014_32d7);
        let peak = antenna_glow(90);
        assert_eq!(peak & 0xff, 0xff, "red saturates");
        assert_eq!(peak, 0x801e_64ff, "sin = 1: green 0x32 + 50, blue 0x14 + 10");
    }

    /// The edge probe (every fifth tick on the ground) finds the drop 1.1 ahead of Ratchet standing at a floor's edge:
    /// no floor within 20, a free sphere a little past the edge; the head look then pitches his head and neck down.
    #[test]
    fn standing_at_an_edge_looks_down() {
        use super::super::testkit::{floor, Runner};
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([423.4, 410.0, 100.0], 0.0);
        for _ in 0..30 { r.tick(&coll, PadInput::neutral()); }
        let (on, depth, room) = r.hero.edge;
        assert!(on && depth == 20.0 && (0.0..1.0).contains(&room), "{:?}", r.hero.edge);
        let (head, neck) = (&r.hero.idle.joints[joint::HEAD], &r.hero.idle.joints[joint::NECK]);
        assert!(head.cur[1] > 0.05 && neck.cur[1] > 0.03, "{:?} {:?}", head.cur, neck.cur);
        assert_eq!((head.k, neck.k), (f(0x3c75_c28f), f(0x3c03_126f)));
        // In the middle of the floor: no edge.
        let mut r = Runner::new([412.0, 410.0, 100.0], 0.0);
        for _ in 0..30 { r.tick(&coll, PadInput::neutral()); }
        assert!(!r.hero.edge.0);
    }
}
