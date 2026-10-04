//! The hero (Ratchet): the global hero block (level01 `0x13f350..0x141660`), the per-tick update `0x228870`
//! and its sub-steps. Spec: `docs/plan/player_controller.md`; every state of the game's hero state machine,
//! which module owns it and the port packages: `docs/plan/hero_states.md`.
//!
//! Structure (one module per state group, mirroring the game's switches):
//! * [`registry`]: the state table 0..=0x82 (name, the game's group byte, owning module, ported) and the three
//!   dispatchers: SetState's per-state entry, the per-state physics `0x2370b8`, the per-state transitions
//!   `0x242930`.
//! * [`states`]: the driver — SetState `0x23cf98` (refusals, bookkeeping, epilogue) and the transitions'
//!   prologue; [`common`]: the helpers the groups share (jump choosers, landing ETA, gravity step).
//! * Ported groups: [`ground`] (0, 3, 4), [`walk`] (2, 0x73), [`air`] (6), [`jump`] (the jump group: 7, 9,
//!   0xb, 0xe, 0x12), [`melee`] (0x13, 0x14), [`swim`] (0x33..0x37, 0x6a).
//! * Packages (stubs routed by the registry, one owner each): [`surface`] + [`platform`] (P1), [`damage`] +
//!   [`stance`] (P2), [`ledge`] (P3), [`packs`] (P4), [`boots`] (P5), [`swingshot`] + [`gadgets`] (P6).
//! * [`physics`]: math helpers, stick → target, turns, speed, air control, the per-state physics driver, the
//!   move + collide pipeline `0x23c458`/`0x233de0` and the ground probe.
//! * [`anim`]: the animation interface (`SetAnim` 0x247a90, Ratchet's advance 0x247d48).
//! * [`idle`]: the idle behaviour (fidgets 0x241e00, head look 0x22b928, secondaries 0x22bdd0, blinks) and the
//!   back items (pack and Clank) as the hero code drives them; [`items`]: the hand slot.
//!
//! Field names are ours; each carries the game address so a PCSX2 trace of the block can be diffed
//! field by field. Floats are PS2 bit patterns ([`Pf`]).
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

pub mod anim;
pub mod idle;
pub mod pose;
pub mod items;
pub mod melee;
pub mod physics;
pub mod registry;
pub mod states;
pub mod swim;

mod air;
mod common;
mod ground;
mod jump;
mod walk;

// Package modules (docs/plan/hero_states.md "Packages").
pub mod boots;
pub mod damage;
pub mod fx;
pub mod gadgets;
mod ledge;
pub mod packs;
pub mod platform;
pub mod stance;
pub mod surface;
pub mod swingshot;
pub mod comet;
pub mod weapons;
pub mod gloves;
pub mod pyrocitor;
pub mod guns;
pub mod blaster;
pub mod ryno;
pub mod devastator;
pub mod tesla;
pub mod reactive;
pub mod suck_cannon;
pub mod suck_vortex;
pub mod taunter;
pub mod morph_ray;
pub mod walloper;
pub mod visibomb;
pub mod hydrodisplacer;
pub mod trespasser;
pub mod metal_detector;
pub mod hologuise;
pub mod pda;
pub mod crank;
pub mod scripted;
pub mod worn;
pub mod bodies;
pub mod hoverboard;
pub mod strafe;

use crate::ps2v::Pf;
pub use ledge::LedgeBlock;
use physics::V4;

/// Hero state ids (`0x1413d4`) the port implements.
pub mod state {
    pub const IDLE: i32 = 0;
    pub const WALK: i32 = 2;
    pub const STOP: i32 = 3;
    pub const CROUCH: i32 = 4;
    pub const FALL: i32 = 6;
    pub const JUMP: i32 = 7;
    pub const RUN_JUMP: i32 = 9;
    pub const LONG_JUMP: i32 = 0xb;
    pub const DOUBLE_JUMP: i32 = 0xe;
    /// The wrench combo (three swings) and the jump attack.
    pub const COMBO: i32 = 0x13;
    pub const JUMP_ATTACK: i32 = 0x14;
    /// Whether the port implements `s` ([`super::registry::STATES`]).
    pub fn implemented(s: i32) -> bool { super::registry::implemented(s) }
}

/// Size of the position / yaw history rings (0x140e50, 0x141478).
pub const HISTORY: usize = 32;

/// The jump block `0x13f720..0x13f7ff` (the fields the on-foot jumps use).
#[derive(Clone, Copy, Debug, Default)]
pub struct JumpBlock {
    /// 0x13f720 / 0x13f724: scripted-curve window (with 0x13f76c).
    pub curve_from: i32,
    pub curve_to: i32,
    /// 0x13f728: the anim frame the playback aims at before the curve window; 0x13f72c; 0x13f730: the curve's
    /// anim length in ticks (the water jump 0x12: 0, 18.0, 40).
    pub curve_frame: Pf,
    pub f72c: Pf,
    pub curve_len: i32,
    /// 0x13f738 / 0x13f73c / 0x13f740: scripted vertical curve row, tick in the row, acceleration (the table
    /// pointer 0x13f734 is [`swim::WATER_JUMP_CURVE`], the only curve the port has).
    pub curve_idx: i32,
    pub curve_tick: i32,
    pub curve_acc: Pf,
    /// 0x13f744: pack-jump target speed.
    pub f744: Pf,
    /// 0x13f748: gravity once descending (0 = keep).
    pub g_down: Pf,
    /// 0x13f750: position at takeoff.
    pub takeoff_pos: V4,
    /// 0x13f760: reference height (springs up to the ground, never above the feet).
    pub ref_z: Pf,
    /// 0x13f764: air turn rate cap (rad/tick).
    pub turn_max: Pf,
    /// 0x13f768: landed counter (0 = in the air; counts ticks since landing).
    pub landed: i32,
    /// 0x13f76c: scripted curve active; 0x13f76e: descending.
    pub curve_on: i16,
    pub descending: i16,
    /// 0x13f770: takeoff tick (windup before leaving the ground; −1 for 0xe).
    pub takeoff: i32,
    /// 0x13f784: capsule bottom target after takeoff.
    pub bottom784: Pf,
    /// 0x13f774: air stick speed scale.
    pub air_speed: Pf,
    /// 0x13f778 / 0x13f77c / 0x13f780: pending vz, applied vz, current height target h.
    pub pending: Pf,
    pub applied: Pf,
    pub h: Pf,
    /// Flip 0xb: 0x13f788 stick direction (target yaw at entry) / 0x13f78c facing 10 ticks ago; 0x13f79c
    /// landing ETA (ticks, all jumps); 0x13f7a0 stick sector (0 left, 1 right, 3 back); 0x13f7a4 side speed;
    /// 0x13f7a8 flip speed; 0x13f7ac.
    pub dir_yaw: Pf,
    pub face_yaw: Pf,
    pub land_eta: i32,
    pub kind7a0: i32,
    pub side_speed: Pf,
    pub fwd_speed: Pf,
    pub speed7ac: Pf,
    /// 0x13f790 / 0x13f794 / 0x13f798: anim frames: apex, pre-touchdown hold, landing − 2.
    pub f_apex: Pf,
    pub f_hold: Pf,
    pub f_land: Pf,
    /// 0x13f7e0: fall-over height; 0x13f7e4: anim speed factor; 0x13f7f6 (s16); 0x13f7fa (s16): fall-over
    /// after; 0x13f7fc (s16); 0x13f7fe (u8): flip chain allowed.
    pub fallover_h: Pf,
    pub ak: Pf,
    pub f7f6: i16,
    pub fallover_after: i16,
    pub f7fc: i16,
    pub flip_chain: u8,
    /// 0x13f7d0 / 0x13f7d4: windup accel / decel.
    pub acc: Pf,
    pub dec: Pf,
    /// 0x13f7d8 / 0x13f7dc: min / max jump height.
    pub hmin: Pf,
    pub hmax: Pf,
    /// 0x13f7e8 (s16): ticks to ramp min → max; 0x13f7ea (s16): ticks before forcing fall 6.
    pub ramp: i16,
    pub max_air: i16,
    /// 0x13f7f0: gravity (u/tick²).
    pub g: Pf,
    /// 0x13f7f4 (s16): blocked ahead in the first 10 ticks (caps air speed at 2.1·dt).
    pub blocked: i16,
    /// 0x13f7f8 (s16): keep the ground speed as air target.
    pub keep_speed: i16,
    /// 0x13f7ff (u8): double jump allowed (set by 7 / 9).
    pub can_double: u8,
}

/// The hero global block. Addresses are level01 (= boot, .bss).
#[derive(Clone, Debug)]
pub struct Hero {
    /// 0x13f350: rotation rows from the Euler angles (copied to moby+0xc0 each tick).
    pub rows: [V4; 4],
    /// **0x13f3d0: position (feet).**
    pub pos: V4,
    /// 0x13f3e0: Euler rotation; `.z` (0x13f3e8) = **yaw**.
    pub rot: V4,
    /// **0x13f430: velocity (u/tick).**
    pub vel: V4,
    /// 0x13f440: platform delta this tick (not modelled: always 0).
    pub platform: V4,
    /// 0x13f450: displacement this tick (pos − old).
    pub disp: V4,
    /// 0x13f460 / 0x13f470 / 0x13f480: effective velocity, its vertical and horizontal parts.
    pub eff: V4,
    pub eff_v: V4,
    pub eff_h: V4,
    /// 0x13f490: platform displacement applied.
    pub plat_applied: V4,
    /// 0x13f4a0: carried momentum.
    pub momentum: V4,
    /// 0x13f4b0 / 4b4 / 4b8 / 4bc: |eff|, |eff.xy|, forward speed, slope ratio.
    pub eff_len: Pf,
    pub eff_len_xy: Pf,
    pub fwd_speed: Pf,
    pub slope_ratio: Pf,
    /// 0x13f4c0: stick direction in the world.
    pub stick_world: V4,
    /// **0x13f4d0 / 4d4 / 4d8: target yaw, yaw velocity, yaw residual.**
    pub target_yaw: Pf,
    pub yaw_vel: Pf,
    pub yaw_residual: Pf,
    /// **0x13f4e0 / 4e4: target speed / current speed (u/tick).**
    pub target_speed: Pf,
    pub speed: Pf,
    /// **0x13f4e8: ticks in the current state.**
    pub timer: i32,
    /// 0x13f4f0 / 0x13f4f4: ticks since the substate / the target sequence last changed.
    pub substate_timer: i32,
    pub seq_timer: i32,
    /// 0x13f504: wall-jump / ledge window (set by the ledge probes of [`ledge`]; stays 0 until ported).
    pub ledge: i32,
    /// FastDecTimer counters of `0x23c710` the package states set (0 until then): 0x13f500 (s16, ledge regrab
    /// lockout), 0x13f502 (s16), 0x13f508 (Thruster long-jump lockout), 0x13f510 (hit invulnerability),
    /// 0x13f518 / 0x13f51a (s16), 0x13f520 (weapon stance), 0x13f524 (jump buffer override after the glide 8:
    /// the idle / walk / fall jump tests use 1 tick while it runs), 0x13f530 (s16, sinking floor), 0x13f534 /
    /// 0x13f536 (s16, swing), 0x13f53e (s16, hit flash), 0x13f546 (s16, platform).
    pub f500: i16,
    pub f502: i16,
    pub f508: i32,
    pub f510: i32,
    pub f518: i16,
    pub f51a: i16,
    pub f520: i32,
    pub f524: i32,
    pub f530: i16,
    pub f534: i16,
    pub f536: i16,
    pub f53e: i16,
    pub f546: i16,
    /// 0x13f514: landing lockout (blocks jump / crouch / walk and the fall air control).
    pub lockout: i32,
    /// 0x13f51c: skip the velocity clamp.
    pub no_vel_clamp: i32,
    /// 0x14162a (s16): the spline the next capsule pass keeps Ratchet off (0: none). A class sets it during the moby
    /// loop (Pokitaru's 361); the pass pushes him away from the spline and clears it ([`physics`], `spline_wall`).
    pub wall_spline: i16,
    /// 0x13f542 (s16): jump lockout (SetState of a jump undoes itself while set).
    pub jump_lockout: i16,
    /// 0x13f70c (s16): landing-into-run blend running.
    pub land_run_blend: i16,
    /// 0x1415d4.
    pub f15d4: i32,
    /// 0x1413ec: state before the previous one.
    pub prev_prev_state: i32,
    /// 0x13f532 (s16): stuck-in-air counter (airborne, falling, < 1 u/s over the last 8 history steps).
    pub f532: i16,
    /// 0x13f540 (s16).
    pub f540: i16,
    /// 0x13f544 (s16): extra edge brake in walk.
    pub edge_brake: i16,
    /// 0x13f550 / 0x13f560: last capsule contact normal (raw) / point.
    pub contact_normal: V4,
    pub contact_point: V4,
    /// 0x13f570 / 574 / 584: capsule top / bottom / radius; 0x13f578 / 57c / 580: their targets.
    pub cap_top: Pf,
    pub cap_bottom: Pf,
    pub cap_top_target: Pf,
    pub cap_bottom_target: Pf,
    pub cap_radius_target: Pf,
    pub cap_radius: Pf,
    /// 0x13f58c: moby hit by the capsule (never in this port); 0x13f590; 0x13f5a4/5a5; 0x13f5a7 capsule hit this tick.
    pub cap_moby: Option<usize>,
    pub f590: i32,
    pub f5a4: u8,
    pub f5a5: u8,
    pub cap_hit: u8,
    /// 0x13f5c0: ground normal; 0x13f5e0: gravity direction (0, 0, −1); 0x13f5f0: ground point.
    pub ground_normal: V4,
    pub gravity_dir: V4,
    pub ground_point: V4,
    /// 0x13f628: ground z; **0x13f62c: height above ground**; 0x13f630 slope angle; 0x13f634 / 638 pitch /
    /// roll under the hero; 0x13f63c slope yaw.
    pub ground_z: Pf,
    pub height: Pf,
    pub slope: Pf,
    pub pitch: Pf,
    pub roll: Pf,
    pub slope_yaw: Pf,
    /// 0x13f64c: ground moby (never); 0x13f650: grounded tick count; 0x13f654.
    pub ground_moby: Option<usize>,
    pub grounded_ticks: i32,
    pub f654: Pf,
    /// 0x13f658 (s16): gravity-mode source (wall walking; 0); 0x13f65a (s16); **0x13f65e (s16): air ticks**.
    pub f658: i16,
    pub f65a: i16,
    pub air_ticks: i16,
    /// 0x13f708 (s16): walk along the target yaw; 0x13f70e (s16): sharp turn.
    pub turn_to_target: i16,
    pub sharp_turn: i16,
    /// 0x13f720..0x13f7ff.
    pub jump: JumpBlock,
    /// 0x13f838: ledge found by the ledge probes (the fall and the jumps take 0x18 while it is set; [`ledge`]).
    pub f838: i32,
    /// The ledge / wall-jump fields of the block (0x13f7b0..0x13f7cc, 0x13f820..0x13f848, 0x141604; [`ledge`]).
    pub ledge_blk: LedgeBlock,
    /// The damage fields: this tick's hit message, the killer 0x1415d0, 0x77's tumble, the events ([`damage`]).
    pub damage: damage::Damage,
    /// 0x140990 / 0x14099c / 0x1409a0: the walk-to-point target of 0x65..0x67 ([`stance`]).
    pub walk_to: stance::WalkTo,
    /// 0x13fc70: external push (knockback); 0x13fc80: its strength; 0x13fc90: group gravity; 0x13fc94.
    pub push: V4,
    pub push_strength: Pf,
    pub group_gravity: Pf,
    pub fc94: Pf,
    /// 0x13fde0: anim playback speed.
    pub anim_speed: Pf,
    /// 0x140e50: position history (32 × vec4), next slot 0x141500, count 0x141504.
    pub pos_hist: [V4; HISTORY],
    pub pos_hist_idx: i32,
    pub pos_hist_count: i32,
    /// 0x141478: yaw history, next slot 0x1414f8, count 0x1414fc.
    pub yaw_hist: [Pf; HISTORY],
    pub yaw_hist_idx: i32,
    pub yaw_hist_count: i32,
    /// 0x141070: stick used this tick (d-pad fallback applied).
    pub stick: [Pf; 2],
    /// **0x1413d4: state**; 0x1413d8 substate; **0x1413dc movement group**; 0x1413e0 / e4 previous state /
    /// group; 0x1413e8 previous timer; 0x1413f0 group before that.
    pub state: i32,
    pub substate: i32,
    pub group: i32,
    pub prev_state: i32,
    pub prev_group: i32,
    pub prev_timer: i32,
    pub prev_prev_group: i32,
    /// 0x14063a: surface 8 / 0xc under the hero (→ state 0x79, not ported).
    pub f063a: u8,
    /// 0x140360: the **head-look timer** of `0x22b928` (the name predates the idle port; it is not a fidget
    /// countdown): `rand_range(180, 300)` at hero init, `rand_range(50, 100)` on SetState(0), re-armed to 40..70
    /// every tick of the fidgets 1/2 and to 90..280 at each look change ([`idle`]).
    pub fidget_timer: i32,
    /// 0x1413f4: character / control mode (0 = Ratchet on foot).
    pub mode: u8,
    /// 0x1413f8 / 0x1413f9 / 0x1413fa: weapon-in-hand flags (0 with no gadget).
    pub f13f8: u8,
    pub f13f9: u8,
    pub f13fa: u8,
    /// 0x141403: gravity mode (0 = normal −Z).
    pub gravity_mode: u8,
    /// 0x1415e0: stop decel; **0x1415ec: stick magnitude** (min(|s|, 1), one tick late); 0x1415f4.
    pub stop_decel: Pf,
    pub stick_mag: Pf,
    pub f15f4: Pf,
    /// 0x1415f8: health.
    pub health: i32,
    /// Copy of Ratchet's moby rows (+0xc0..) and Euler (+0x40) as the last write-back (0x229f20) left them:
    /// the "moby frame" some hero code reads (SetPlanarVel's zero-stick direction, 0x248da0, pitch/roll).
    pub moby_rows: [V4; 4],
    pub moby_rot: V4,
    /// 0x140632 / 0x140634 / 0x140637: per-level/surface flags (0 on Novalis ground).
    pub f0632: u8,
    pub f0634: u8,
    pub f0637: u8,
    /// 0x13f680 / 684 / 688: knockback magnitude / pitch / yaw (the push generator 0x233850's inputs).
    pub knock: [Pf; 3],
    /// 0x13f588: capsule radius spring velocity.
    pub cap_radius_vel: Pf,
    /// 0x13f548 (s16): raised capsule base timer.
    pub f548: i16,
    /// 0x13f53a (s16) / 0x13f594: edge-nudge timer / speed.
    pub nudge_timer: i16,
    pub nudge_speed: Pf,
    /// 0x13f600: last grounded point; **0x13f640 (f32): water level**, written by the ground probe 0x232dc0 only
    /// on a water hit (surface 0: `SetWaterLevel`; 0xd: the hit z) and by the surface reaction for surface 0xe
    /// (not a surface id: that is 0x140630, the footstep class 0x14063d); 0x13f648: in-water counter;
    /// 0x13f65c (u16): ticks since height < 0.02.
    pub last_ground_point: V4,
    pub water_level: Pf,
    pub in_water: i32,
    pub f65c: u16,
    /// 0x140630 (s16): surface id of the ground hit; 0x14063d: footstep class.
    pub surface_id: i16,
    pub footstep: u8,
    /// 0x13f4ec, 0x13f4f8 / 0x13f4fc (ticks with / without a direction held), 0x13f50c.
    pub f4ec: i32,
    pub f4f8: i32,
    pub f4fc: i32,
    pub f50c: i32,
    /// 0x13f420 body point, 0x13f410 shadow / target point.
    pub body_point: V4,
    pub shadow_point: V4,
    /// 0x1413fd: hero frozen (no move); 0x141401: fell out of the world (fade requested).
    pub frozen: u8,
    pub fell_out: u8,
    /// States not ported that were reached (logged once each).
    pub unimplemented_seen: Vec<i32>,
    /// 0x13fd40..0x13fdcc: the melee block.
    pub melee: melee::Melee,
    /// The hand slot 0x1403e0 and the hand-item fields of the block.
    pub items: items::HeroItems,
    /// The jump attack's shockwave `(centre, dir)` queued by its physics for the hit sink.
    pub shockwave: Option<(V4, V4)>,
    /// The idle fields (fidget cooldowns, look, secondaries, blinks, joint records) and 0x15f5cc / 0x15ed84.
    pub idle: idle::Idle,
    /// Item slot 3: the pack and Clank (None until created; created only with [`Hero::back_classes`]).
    pub back: Option<idle::Back>,
    /// The back items' anim classes ([`Hero::set_back_classes`]).
    pub back_classes: Option<std::sync::Arc<idle::BackClasses>>,
    /// The swim fields (bob, oxygen, pitch / roll springs, the Hydro-Pack / O2 mask flags) and swim events.
    pub swim: swim::Swim,
    /// The platform carry fields 0x13f660..0x13f6b4 ([`platform`], package P1).
    pub carry: platform::Carry,
    /// The surface fields: the reaction's other flags, the liquid level 0x13f644, the slide and sinking-floor
    /// fields (`surface`, package P1).
    pub surf: surface::Surf,
    /// The game state's item-owned table `0x13d4c0 + id` as the hero code reads it ([`Owned`]).
    pub owned: Owned,
    /// Item slot 3's bookkeeping (`GetClankModule(3)`, the back item's swap; [`idle::BackSlot`]).
    pub back_slot: idle::BackSlot,
    /// Item slots 1 (feet) and 2 (head): their bookkeeping and the head moby's put-away ([`worn`]).
    pub feet_slot: idle::ItemSlot,
    pub head_slot: idle::ItemSlot,
    pub worn: worn::Worn,
    /// The Heli-Pack / Thruster-Pack fields (stomp, rebound, hover latch and taps, the hero's looping sound
    /// slots; [`packs`], package P4).
    pub packs: packs::Packs,
    /// 0x13f598 / 0x13f5a0: the wall-ahead probe of `0x23c458` (every third tick: distance to the wall 0.7 above
    /// the feet within 4 ahead, 4.0 without one; the elevation of its normal). Native `f32`.
    pub wall_ahead: [f32; 2],
    /// 0x13f5b0 / 0x13f5a8 / 0x13f5ac: the edge probe of `0x23c458` (every fifth tick on the ground: a drop ahead, its
    /// depth, the room before the edge; `Hero::wall_ahead_probe`), read by the head look's look-down (`0x22b928`).
    pub edge: (bool, f32, f32),
    /// The grind / cable / Magneboots fields 0x13f850..0x13f96c ([`boots`], package P5).
    pub boots: boots::Boots,
    /// The hand-item mechanism's pending item update ([`gadgets`], package P6).
    pub gadgets: gadgets::Gadgets,
    /// The Swingshot fields 0x13fcb0..0x13fd1c and its hand item's ([`swingshot`], package P6).
    pub swing: swingshot::Swing,
    /// The hero's requests of other systems (camera shakes, …) queued for the tick ([`fx`]).
    pub fx: fx::HeroFx,
    /// 0x1413f5: the first-person camera is up (the camera type 4 `0x316330` sets it every tick once its blend-in
    /// is over, `crate::follow_camera`; every SetState clears it): the look stances turn Ratchet to the camera
    /// and `HeroSyncMoby` 0x229f20 hides him and his items ([`Hero::write_back`]).
    pub f13f5: u8,
    /// 0x1413ff: the hand item hidden (`FUN_002487a8` hides the hand moby while it is set: the gold bolt 1134's
    /// pickup); cleared by `SetState` on foot.
    pub f13ff: u8,
    /// The weapons: the ammo mirror, the weapon-out arm (0x1413f8), the bomb glove's pvars ([`weapons`]).
    pub weapons: weapons::Weapons,
    /// The Comet-Strike's catch request ([`comet`]).
    pub comet: comet::Comet,
    /// The globals outside the moby system the moby loop's classes read through the hero (the pad, the camera
    /// Euler, Ratchet's anim fields), filled by the tick right before the moby loop
    /// ([`crate::moby_update::services::LoopGlobals`]). Not part of the game's hero block.
    pub loop_in: crate::moby_update::services::LoopGlobals,
    /// Per joint list of Ratchet's class, the joint a joint-modifier node made for it acts on
    /// (`rc_formats::moby_anim::list_target`; 0xff: none), set by the loader ([`Hero::set_joint_targets`]). Empty:
    /// his joint-modifier list is not written to his moby. Not part of the game's hero block.
    pub joint_targets: std::sync::Arc<Vec<u8>>,
    /// The hero's help calls and their counters (0x141404 / 0x141405, gp 0x15f688 / 0x15f68c) and the help records as the
    /// tick started ([`crate::help::HeroHelp`]).
    pub help: crate::help::HeroHelp,
    /// The Walloper's arcs, glow and the gadget lunge's queued hits ([`walloper`]; the game's globals 0x1dc020.. and
    /// 0x1616fc..).
    pub walloper: walloper::Walloper,
    /// The other bodies' fields (Clank's health, Giant Clank's energy, the body moby; [`bodies`], G-HERO-005).
    pub bodies: bodies::Bodies,
    /// The Hoverboard's block 0x13fa10..0x13fc1f and its globals ([`hoverboard`], levels 5 / 16).
    pub board: hoverboard::Board,
    /// The cheat bytes 0x15edb0 as the hero code reads them (the tick copies `GameOptions::cheats` in; `crate::cheats`).
    pub cheats: crate::cheats::Cheats,
    /// **Port-only**: the Port Options strafe ([`strafe`]): `strafe_mode` = the option, `strafe` = L2 / R2 held this
    /// tick while it applies (the tick sets both before the hero update).
    pub strafe_mode: bool,
    pub strafe: bool,
    /// The direction the strafe last moved Ratchet ([`strafe::move_yaw`]).
    pub strafe_yaw: Pf,
    /// The move ring 0x141514[16] / 0x141524 of the cheat entry `0x2285a0` (`crate::cheats::MoveEntry`).
    pub cheat_moves: crate::cheats::MoveEntry,
}

/// Item ownership as the hero code reads it: the game state's owned table `0x13d4c0 + id` (37 items,
/// `GameState::global.owned`), mirrored into the hero by the engine before every tick. Item ids:
/// docs/plan/hero_states.md §0.1 (`crate::game_state::item`). Tests grant items with [`Hero::grant_items`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Owned(pub [u8; rc_formats::save_game::ITEM_COUNT]);

impl Default for Owned {
    fn default() -> Self { Owned([0; rc_formats::save_game::ITEM_COUNT]) }
}

impl std::fmt::Debug for Owned {
    /// The owned ids only.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries((0..self.0.len()).filter(|&i| self.has(i))).finish()
    }
}

impl Owned {
    /// `0x13d4c0[id] != 0`.
    pub fn has(&self, id: usize) -> bool { self.0.get(id).is_some_and(|&b| b != 0) }
    pub fn set(&mut self, id: usize, owned: bool) {
        if let Some(b) = self.0.get_mut(id) { *b = owned as u8; }
    }
}

impl Default for Hero {
    fn default() -> Self { Hero::new() }
}

impl Hero {
    /// The block as `FUN_00226b70` (hero init) leaves it: zeroed, then the fields the port needs non-zero.
    pub fn new() -> Hero {
        let z = Pf::ZERO;
        let v = physics::V0;
        Hero {
            rows: physics::euler_rows(v), pos: v, rot: v, vel: v, platform: v, disp: v, eff: v, eff_v: v, eff_h: v,
            plat_applied: v, momentum: v, eff_len: z, eff_len_xy: z, fwd_speed: z, slope_ratio: z, stick_world: v,
            target_yaw: z, yaw_vel: z, yaw_residual: z, target_speed: z, speed: z, timer: 0, substate_timer: 0,
            seq_timer: 0, ledge: 0, f500: 0, f502: 0, f508: 0, f510: 0, f518: 0, f51a: 0, f520: 0, f524: 0, f530: 0,
            f534: 0, f536: 0, f53e: 0, f546: 0, lockout: 0, no_vel_clamp: 0, wall_spline: 0, f532: 0, f540: 0, edge_brake: 0,
            contact_normal: v, contact_point: v,
            cap_top: Pf::b(0x3f4c_cccd), cap_bottom: Pf::b(0x3f33_3333), cap_top_target: Pf::b(0x3f4c_cccd),
            cap_bottom_target: Pf::b(0x3f33_3333), cap_radius_target: Pf::b(0x3ee6_6666), cap_radius: Pf::b(0x3ee6_6666),
            cap_moby: None, f590: 0, f5a4: 0, f5a5: 0, cap_hit: 0,
            ground_normal: [z, z, Pf::ONE, z], gravity_dir: [z, z, -Pf::ONE, z], ground_point: v,
            ground_z: z, height: z, slope: z, pitch: z, roll: z, slope_yaw: z,
            ground_moby: None, grounded_ticks: 0, f654: z, f658: 0, f65a: 0, air_ticks: 0,
            turn_to_target: 0, sharp_turn: 0, jump: JumpBlock::default(), f838: 0, ledge_blk: LedgeBlock::default(), damage: damage::Damage::default(), walk_to: stance::WalkTo::default(),
            push: v, push_strength: z, group_gravity: z, fc94: z, anim_speed: Pf::ONE,
            pos_hist: [v; HISTORY], pos_hist_idx: 0, pos_hist_count: 0,
            yaw_hist: [z; HISTORY], yaw_hist_idx: 0, yaw_hist_count: 0, stick: [z, z],
            state: 0, substate: 0, group: 0, prev_state: 0, prev_group: 0, prev_timer: 0, prev_prev_group: 0,
            mode: 0, f13f8: 0, f13f9: 0, f13fa: 0, gravity_mode: 0,
            stop_decel: z, stick_mag: z, f15f4: z, health: 4, f063a: 0, fidget_timer: 0, jump_lockout: 0, land_run_blend: 0, f15d4: 0, prev_prev_state: 0,
            moby_rows: physics::euler_rows(v), moby_rot: v, f0632: 0, f0634: 0, f0637: 0, knock: [z; 3],
            cap_radius_vel: z, f548: 0, nudge_timer: 0, nudge_speed: z, last_ground_point: v, water_level: z,
            in_water: 0, f65c: 0, surface_id: 0, footstep: 0, f4ec: 0, f4f8: 0, f4fc: 0, f50c: 0,
            body_point: v, shadow_point: v, frozen: 0, fell_out: 0, unimplemented_seen: Vec::new(),
            melee: melee::Melee::default(), items: items::HeroItems::default(), shockwave: None,
            idle: idle::Idle::new(), back: None, back_classes: None, swim: swim::Swim::new(),
            carry: platform::Carry::default(), surf: surface::Surf::default(),
            owned: Owned::default(), back_slot: idle::BackSlot::default(), feet_slot: idle::ItemSlot::default(), head_slot: idle::ItemSlot::default(), worn: worn::Worn::default(), packs: packs::Packs::default(), wall_ahead: [0.0; 2], edge: (false, 0.0, 0.0), boots: boots::Boots::default(),
            gadgets: gadgets::Gadgets::default(), swing: swingshot::Swing::default(), fx: fx::HeroFx::default(),
            f13f5: 0, f13ff: 0, weapons: weapons::Weapons::default(), comet: comet::Comet::default(), loop_in: Default::default(),
            joint_targets: Default::default(), help: Default::default(), walloper: Default::default(), bodies: bodies::Bodies::default(),
            cheats: Default::default(), strafe_mode: false, strafe: false, strafe_yaw: Pf::ZERO, cheat_moves: Default::default(), board: hoverboard::Board::default(),
        }
    }

    /// Test / debug helper: mark `ids` owned in the hero's mirror (the engine overwrites it from the game state).
    pub fn grant_items(&mut self, ids: &[usize]) {
        for &i in ids { self.owned.set(i, true); }
    }

    /// Place the hero (spawn / respawn): position, yaw, rows; velocities and histories cleared.
    pub fn spawn(pos: [f32; 3], yaw: f32) -> Hero {
        let mut h = Hero::new();
        h.pos = physics::from_f32x3(pos);
        h.rot[2] = Pf::f(yaw);
        h.target_yaw = h.rot[2];
        h.rows = physics::euler_rows(h.rot);
        h.moby_rows = h.rows;
        h.moby_rot = h.rot;
        h.jump.turn_max = physics::DT * Pf::b(0x4170_2845);
        h
    }

    /// Hero init `FUN_00226b70` on Ratchet's moby (+0xa6 == 0): ground-snap the moby (`0x26e618(0.5)`: a
    /// line from 0.5 above its position down to z = 0.01, flags 2; a hit above 0 becomes its z), mode |= 2
    /// (no generic update), hero position and yaw from the moby, ground probe, write-back; at its end the fidget
    /// timer 0x140360 = `rand_range(ticks(180), ticks(300))` on the game's stream (`HeroInit`'s one draw, the
    /// first after `srand(1234)`: the loader runs the hero init before the load pass).
    pub fn init_from_moby(moby: &mut crate::moby_runtime::Moby, coll: &rc_formats::collision::Collision, rng: &mut crate::rng::Rng) -> Hero {
        let p = physics::v4(moby.position[0], moby.position[1], moby.position[2]);
        let mut a = p;
        a[2] = p[2] + Pf::b(0x3f00_0000);
        let mut b = p;
        b[2] = Pf::b(0x3c23_d70a);
        if let Some(o) = physics::line_world(coll, a, b, 2) {
            if 0.0 < o.point[2] { moby.position[2] = o.point[2]; }
        }
        moby.mode |= crate::moby_runtime::mode::NO_UPDATE;
        let mut h = Hero::spawn([moby.position[0], moby.position[1], moby.position[2]], moby.rotation[2]);
        let env_pad = crate::pad::PadState::default();
        let env = Env { coll, pad: &env_pad, cam_yaw: Pf::ZERO, cam_rows: [physics::V0; 3], mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
        h.ground_probe(&env);
        h.write_back(moby);
        h.fidget_timer = rng.rand_range(physics::ticks(180), physics::ticks(300));
        // The loader runs the hero init at counter 0; the first gameplay tick sees 1 (Game::finish_load).
        h.idle.counter = 1;
        h
    }

    pub fn yaw(&self) -> Pf { self.rot[2] }

    /// `SetAnim(blend, seq, frame)` 0x247a90 as the hero code calls it: the anim change, the playback
    /// speed `0x13fde0` back to 1, then in mode 0 the back items' follow `0x247550` with the blend in ticks
    /// (`blend`, or the eased curve's length), which draws once when `seq` has rows in the back table.
    pub fn set_anim(&mut self, anim: &mut dyn AnimCtl, rng: &mut crate::rng::Rng, blend: Pf, seq: u8, frame: i32) {
        anim.set_anim(blend, seq, frame);
        self.anim_speed = Pf::ONE;
        if self.mode == 0 {
            let t = if Pf::ZERO < blend {
                blend.to_f32() as i32
            } else {
                let c = ((-blend).to_f32() as i32) - 1;
                idle::CURVE_TICKS.get(c.max(0) as usize).copied().unwrap_or(0)
            };
            self.back_follow_anim(t, seq, frame, rng);
        }
        // Levels 5 / 16 (`0x25b538` + `0x2478f8`): the board plays its sequence for this one.
        hoverboard::link_anim(self, blend, seq, frame);
    }

    /// `HeroTeleport(pos, euler, state, reset_cam)` 0x2368e0 from the hero code (the Hoverboard's respawns): position and
    /// Euler (rows, the moby's rows), 0x13f4e0 / 0x13f4e4 = 0, the after-images ended, airborne (0x13f65e = 1), the
    /// platform carry dropped, the motion block (0x13f430 / 440 / 450 / 460 / 470 / 4a0) and the straightening's
    /// velocities 0x13f3f0 cleared, the yaw velocity, the hover latch 0x14161a, the pad's 0x13cace / 0x13cad0, a weapon put
    /// away (`0x22efd8`), the joint records reset (`0x227420`), `SetState(state, 1)` (unless −1), the ground probe, and
    /// with `reset_cam` the camera reset behind him ([`fx::HeroFx::camera_reset`]). Not here: the anim snap `0x247c78`
    /// (key A = key B at frame B, t 0) [L], the underwater flag 0x167494 and `EnvNearestSamplePoint` (the caller's).
    pub fn teleport(&mut self, c: &mut states::Ctx, pos: [f32; 4], euler: [f32; 4], state: i32, reset_cam: bool) {
        self.pos = pos.map(Pf::f);
        self.rot = euler.map(Pf::f);
        self.rows = physics::euler_rows(self.rot);
        self.moby_rows = self.rows;
        self.moby_rot = self.rot;
        (self.target_speed, self.speed) = (Pf::ZERO, Pf::ZERO);
        self.fx.trails.hero.kill();
        self.air_ticks = 1;
        self.carry.flags &= !3;
        self.carry.moby = None;
        let z = physics::V0;
        (self.platform, self.vel, self.disp, self.eff, self.eff_v, self.momentum) = (z, z, z, z, z, z);
        self.swim.euler_vel = [Pf::ZERO; 2];
        self.yaw_vel = Pf::ZERO;
        self.packs.hover_latch = 0;
        weapons::put_away(self);
        for j in self.idle.joints.iter_mut() {
            (j.cur, j.target, j.trans_target, j.scale, j.attached) = ([0.0; 3], [0.0; 3], [0.0; 3], 1.0, false);
        }
        if state != -1 { self.set_state(c, state, true); }
        self.ground_probe(c.env);
        if reset_cam { self.fx.camera_reset = true; }
    }
    pub fn position(&self) -> [f32; 3] { physics::to_f32x3(self.pos) }
    pub fn grounded(&self) -> bool { self.air_ticks == 0 }
}

// ------------------------------------------------------------------------------------------------
// The per-tick update 0x228870.

pub use anim::{AnimCtl, AnimView, RecordingAnim};
pub use physics::Env;

/// Result of one hero tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroTick {
    /// The update ran.
    Ran,
    /// The hero is outside x/y 2..1022 (the game fades to black and respawns; the port only reports it).
    OutOfBounds,
    /// The hero is in a state the port does not implement: nothing moved (logged once per state).
    Unimplemented(i32),
}

impl Hero {
    /// The driver `0x231d18`: stick (d-pad fallback below 0.25), per-state physics, the yaw and position
    /// histories, the rows, capsule sizing, the move and the post-move.
    pub fn input_physics_move(&mut self, env: &Env, anim: &mut dyn AnimCtl, rng: &mut crate::rng::Rng) -> bool {
        // 0x248ad8: gravity mode (super::boots).
        self.gravity_mode = boots::gravity_mode(self);
        // 0x231ed8: magnitude of LAST tick's stick.
        let l = crate::pad::len2(self.stick[0], self.stick[1]);
        self.stick_mag = if Pf::ONE < l { Pf::ONE } else { l };
        self.f590 = 0;
        self.stick = [env.pad.lx, env.pad.ly];
        if crate::pad::len2(self.stick[0], self.stick[1]) < Pf::b(0x3e80_0000) {
            let b = env.pad.raw;
            let x = ((b >> 13) & 1) as i32 - ((b >> 15) & 1) as i32;
            let y = ((b >> 14) & 1) as i32 - ((b >> 12) & 1) as i32;
            self.stick = [Pf::from_i32(x), Pf::from_i32(y)];
        }
        if !self.state_physics(env, anim, rng) { return false; }
        self.yaw_hist[self.yaw_hist_idx as usize] = self.rot[2];
        self.yaw_hist_idx = (self.yaw_hist_idx + 1) & 31;
        self.yaw_hist_count = (self.yaw_hist_count + 1).min(32);
        self.pos_hist[self.pos_hist_idx as usize] = self.pos;
        self.pos_hist_idx = (self.pos_hist_idx + 1) & 31;
        self.pos_hist_count = (self.pos_hist_count + 1).min(32);
        self.rows = physics::euler_rows(self.rot);
        self.size_capsule();
        if self.frozen == 0 { self.move_collide(env); }
        // 0x23c458's probes after the move: the wall ahead (every third tick; 0x13f598..0x13f5a5).
        self.wall_ahead_probe(env);
        // The board under Ratchet's feet (groups 0x15 / 0x16, the tail of level05's 0x24bdc0).
        hoverboard::carry(self);
        self.post_move(env);
        true
    }

    /// Write-back `0x229f20` (the part the port needs): moby position = hero position, Euler = hero Euler,
    /// rows = 0x13f350 (`MemCopy(moby + 0xc0, 0x13f350, 0x30)`: the three rotation rows; +0xf0, the bounding
    /// sphere `MobyBuildMatrix` reads, is kept). The tick then runs `MobyBuildMatrix` on the moby.
    pub fn write_back(&mut self, moby: &mut crate::moby_runtime::Moby) {
        moby.position = self.pos.map(Pf::to_f32);
        moby.rotation = self.rot.map(Pf::to_f32);
        for (k, r) in self.rows.iter().take(3).enumerate() { moby.rows[k] = r.map(Pf::to_f32); }
        self.moby_rows = self.rows;
        self.moby_rot = self.rot;
        // 0x1413f5 (first person): `0x2486c0` hides Ratchet (and his items: the engine), else `0x2487a8` shows him.
        if self.f13f5 != 0 { moby.mode |= crate::moby_runtime::mode::HIDDEN; } else { moby.mode &= !crate::moby_runtime::mode::HIDDEN; }
        // His joint-modifier list (moby +0x64: the idle / lean joint records and the eyelids, idle::Manip).
        if !self.joint_targets.is_empty() { moby.joint_mods = self.idle.modifiers(&self.joint_targets); }
    }

    /// Ratchet's class joint lists' modifier targets ([`Hero::joint_targets`]): for each list, the second byte list
    /// (`rc_formats::gadget::joint_list(..).1`).
    pub fn set_joint_targets(&mut self, second_lists: &[Vec<u8>]) {
        self.joint_targets = std::sync::Arc::new(second_lists.iter().map(|b| rc_formats::moby_anim::list_target(b).unwrap_or(0xff)).collect());
    }

    /// Ratchet's class joint lists (the first byte list of each: root-to-joint chains, by list index) for the joint
    /// points of the hero's effects ([`fx::joint_point`]: the water bubbles at his hands, feet and mouth).
    pub fn set_joint_chains(&mut self, first_lists: Vec<Vec<u8>>) { self.fx.joints.hero = std::sync::Arc::new(first_lists); }

    /// The back pack classes' joint lists and class scales `(o_class, scale, first byte lists)` (the Hydro-Pack's
    /// jets: [`fx::pack_point`]).
    pub fn set_pack_joint_lists(&mut self, packs: Vec<(i16, f32, Vec<Vec<u8>>)>) { self.fx.joints.packs = std::sync::Arc::new(packs); }

    /// The hand item classes' joint-list targets `(o_class, target per list)` ([`fx::JointData::items`]: the hand records
    /// and the Metal Detector's node, [`gadgets::hand_modifiers`]).
    pub fn set_item_joint_targets(&mut self, items: Vec<(i16, Vec<u8>)>) { self.fx.joints.items = std::sync::Arc::new(items); }
}

/// Sounds the hero update plays at the game's point inside `0x228870`, so their RNG draws (the class sound's
/// pitch bend) land where the game makes them. The audio layer implements it (`rc_game::audio`).
pub trait HeroSounds {
    /// Right after Ratchet's own advance (`RatchetAnimAdvance` 0x247d48, where the game plays the class sound of
    /// the animation trigger that fired) and before the back items' `0x247800` draw. `moby` is Ratchet's moby as
    /// the last write-back left it (the sound's owner / position); `before` / `after` are his anim views around
    /// the advance (`audio::class_sounds::ratchet_trigger` finds the trigger).
    fn anim_advanced(&mut self, moby: &crate::moby_runtime::Moby, before: &AnimView, after: &AnimView, rng: &mut crate::rng::Rng);
    /// `0x236738(index, flags)`: `PlayClassSound(index, flags, Ratchet)` (the hurt / death voices of
    /// [`damage`]); returns the sound slot (−1: none). Played right after the transitions ([`damage`] module doc).
    fn voice(&mut self, _moby: &crate::moby_runtime::Moby, _index: i32, _flags: u32, _rng: &mut crate::rng::Rng) -> i32 { -1 }
    /// `release_voice_slot(slot)` when the slot still plays a sound of Ratchet's (`moby`): the stop of a looping
    /// sound the hero started with [`HeroSounds::voice`] (flags 4; [`packs`]).
    fn release(&mut self, _moby: &crate::moby_runtime::Moby, _slot: i32) {}
    /// `release_voice_slot(slot)` when the slot still plays a sound of moby `id`'s (a hero loop with another owner: the
    /// Hoverboard's on the board, [`hoverboard`]).
    fn release_of(&mut self, _id: crate::moby_runtime::MobyId, _slot: i32) {}
    /// `PlayClassSound(index, flags, item)` (0x2a1618) on the hand item (the moby of slot 0x1403e0: class
    /// `o_class` at `pos`), e.g. the Swingshot's fire / hit / pull and the wrench's hit; returns the sound slot (−1:
    /// none). Played by the tick right after the hand item's update ([`gadgets::flush_item_sounds`]).
    fn item_sound(&mut self, _o_class: i16, _pos: [f32; 3], _index: i32, _flags: u32, _rng: &mut crate::rng::Rng) -> i32 { -1 }
    /// `PlayFootstepSound(class, foot, variant, 0, Ratchet)` (0x2a1898) on level `level` (0x15ed84): a level def
    /// ([`fx::footstep`], [`fx::walk_footsteps`]); returns the sound slot (−1: none).
    fn footstep(&mut self, _moby: &crate::moby_runtime::Moby, _level: i32, _class: u8, _foot: u8, _variant: u8, _rng: &mut crate::rng::Rng) -> i32 { -1 }
    /// `SoundIsAlive(owner, slot)` 0x2a12f0 for a slot the hero's sounds took (the hand item's loops).
    fn alive(&mut self, _slot: i32) -> bool { false }
    /// `SoundSetPitchBend(slot, pb)` 0x2a1988 on a slot the hand item's sounds took (the Metal Detector's beep).
    fn set_pitch_bend(&mut self, _slot: i32, _pb: i32) {}
    /// `PlayClassSound(index, flags, moby)` on a moby the hand item's update created (the R.Y.N.O.'s missile, class
    /// `o_class` at `pos`, table moby `id`). Default: as [`HeroSounds::item_sound`].
    fn moby_sound(&mut self, _id: crate::moby_runtime::MobyId, o_class: i16, pos: [f32; 3], index: i32, flags: u32, rng: &mut crate::rng::Rng) -> i32 { self.item_sound(o_class, pos, index, flags, rng) }
}

/// No sound layer: [`hero_update`].
pub struct NoHeroSounds;

impl HeroSounds for NoHeroSounds {
    fn anim_advanced(&mut self, _: &crate::moby_runtime::Moby, _: &AnimView, _: &AnimView, _: &mut crate::rng::Rng) {}
}

/// `0x228870` for Ratchet: anim advance 0x247d48 (with the back items' `0x247800`), input + physics + move
/// 0x231d18, surface reaction 0x22cd48, the wall / ledge probe 0x22d090 ([`ledge`]), transitions 0x242930, the
/// idle sub-updates (head look 0x22b928, secondaries 0x22bdd0, blinks 0x227590 / 0x2278c0, joint springs
/// 0x2273d0), the write-back 0x229f20, then the back items' part of `HeroItemsUpdate` (the hand is
/// [`items::items_update`]). The other cosmetic / combat / HUD sub-updates are not ported. No sound layer:
/// [`hero_update_with_sounds`] with [`NoHeroSounds`].
pub fn hero_update(hero: &mut Hero, moby: &mut crate::moby_runtime::Moby, env: &Env, anim: &mut dyn AnimCtl, rng: &mut crate::rng::Rng) -> HeroTick {
    hero_update_with_sounds(hero, moby, env, anim, rng, &mut NoHeroSounds)
}

/// [`hero_update`] with the hero's sounds played at the game's points ([`HeroSounds`]).
pub fn hero_update_with_sounds(
    hero: &mut Hero,
    moby: &mut crate::moby_runtime::Moby,
    env: &Env,
    anim: &mut dyn AnimCtl,
    rng: &mut crate::rng::Rng,
    sounds: &mut dyn HeroSounds,
) -> HeroTick {
    fx::begin(hero, moby);
    let counter = hero.idle.counter;
    hero.idle.counter = counter.wrapping_add(1);
    let (x, y) = (hero.pos[0], hero.pos[1]);
    let (lo, hi) = (Pf::b(0x4000_0000), Pf::b(0x447f_8000));
    if x < lo || y < lo || hi < x || hi < y {
        hero.fell_out = 1;
        return HeroTick::OutOfBounds;
    }
    if !state::implemented(hero.state) {
        if !hero.unimplemented_seen.contains(&hero.state) {
            hero.unimplemented_seen.push(hero.state);
            eprintln!("rc_game::hero: state {:#x} is not ported; the hero is frozen", hero.state);
        }
        return HeroTick::Unimplemented(hero.state);
    }
    // A body is the hero moby (`0x2070d0` → `HeroUpdateAlt` 0x2062b0, G-HERO-005): its own update.
    if hero.mode != 0 {
        if hero.mode == bodies::body::DISGUISE {
            // Body 3: `0x236738` plays Ratchet's class sounds on Ratchet's moby moved to the hero (not on the body moby):
            // the hero's voices of this update are queued and played by the tick on Ratchet's moby (hologuise).
            let mut q = hologuise::DisguiseVoices { inner: sounds, queued: Vec::new() };
            bodies::body_update(hero, moby, env, anim, rng, &mut q, counter);
            let queued = q.queued;
            hologuise::queue_voices(hero, queued);
        } else {
            bodies::body_update(hero, moby, env, anim, rng, sounds, counter);
        }
        // (`fx::end`'s back placement is Ratchet's: the back items do not update in a body.)
        hero.fx.view = anim.view();
        return HeroTick::Ran;
    }
    // The Comet-Strike's catch (the wrench's update asked for Ratchet's loop exit after last tick's advance).
    comet::before_advance(hero, anim);
    let before = anim.view();
    anim.advance(hero.anim_speed);
    sounds.anim_advanced(moby, &before, &anim.view(), rng);
    hero.back_follow_speed(anim.view().seq_b, rng);
    hero.input_physics_move(env, anim, rng);
    // The disguise's timer ran out in the move (`HeroTickStateTimer`): the disguise moby asked for (super::hologuise).
    hologuise::enter(hero);
    // The sounds the per-state physics started (the packs' loops, 0x236798): played at the physics' point.
    packs::flush_sounds(hero, moby, sounds, rng);
    surface::flush(hero, moby, sounds, rng);
    hero.surface_reaction(env, anim, rng);
    surface::flush(hero, moby, sounds, rng);
    ledge::wall_ledge_probe_b(hero, env, &anim.view());
    // The magnetic floor 0x13f658, the rail contact 0x20cf58 and the cable contact 0x20d330 (L00; super::boots).
    boots::contacts(hero, env);
    let group = hero.group;
    // The transitions with Ratchet's voices played at their call points (the splash voices before the splash draws).
    {
        let m: &crate::moby_runtime::Moby = moby;
        let mut voice = |index: i32, flags: u32, r: &mut crate::rng::Rng| { sounds.voice(m, index, flags, r); };
        hero.transitions_with_voice(env, anim, rng, Some(&mut voice));
    }
    // A group change stops the hero's looping sounds (0x2283a8 at the end of 0x242930).
    packs::after_transitions(hero, moby, sounds, rng, group);
    surface::after_transitions(hero, moby, sounds, rng, group);
    damage::flush(hero, moby, sounds, rng);
    // The swim voices and the delayed-voice queue `0x236860` (mode 0, right after the transitions).
    fx::flush(hero, moby, sounds, rng);
    if hero.mode == 0 { hero.idle_updates(env, &*anim, counter, rng); }
    hero.write_back(moby);
    // HeroItemsUpdate's slots 1 (feet) and 2 (head), then 3 (the back).
    hero.worn_items_update(rng);
    let back_empty = hero.back_slot.slot.state == 0;
    hero.back_items_update(rng);
    // HeroItemsCreate's Thruster-Pack flames (class 0xa7) with a new Thruster-Pack: packs::flames_on_create.
    packs::flames_on_create(hero, back_empty);
    // HeroItemsAttach's back placement (the Hydro-Pack's jets read it next tick).
    fx::end(hero, moby, &*anim);
    HeroTick::Ran
}

/// Test fixtures: hand-built collision meshes and a scripted runner (also used by the integration tests).
pub mod testkit {
    use super::*;
    use crate::moby_runtime::Moby;
    use crate::pad::{PadInput, PadState};
    use crate::rng::Rng;
    use rc_formats::collision::{Collision, CollisionCell, CollisionFace, CollisionLeafHeader, PackedCollisionVertex};

    /// A cell at cell coordinates `c` with world-space `verts` (exact at 1/16 in x/y, 1/64 in z relative to
    /// the cell centre) and quads.
    pub fn cell(c: [i16; 3], verts: &[[f32; 3]], quads: &[([u8; 4], u8)]) -> CollisionCell {
        let mut cell = CollisionCell { x: c[0], y: c[1], z: c[2], ..Default::default() };
        let centre = cell.centre();
        for v in verts {
            let f = [(v[0] - centre[0]) * 16.0, (v[1] - centre[1]) * 16.0, (v[2] - centre[2]) * 64.0];
            assert!(f.iter().all(|x| x.fract() == 0.0 && x.abs() < 512.0), "vertex {v:?} not representable in cell {c:?}");
            cell.packed.push(PackedCollisionVertex::pack(f[0] as i32, f[1] as i32, f[2] as i32));
            cell.vertices.push(*v);
        }
        for (q, ty) in quads {
            cell.faces.push(CollisionFace { v: [q[0], q[1], q[2]], surface: *ty });
            cell.quad_v3.push(q[3]);
        }
        cell.header = CollisionLeafHeader { face_count: cell.faces.len() as u16, vertex_count: verts.len() as u8, quad_count: quads.len() as u8 };
        cell
    }

    pub fn mesh(mut cells: Vec<CollisionCell>) -> Collision {
        cells.sort_by_key(|c| (c.z, c.y, c.x));
        Collision { cells, ..Default::default() }
    }

    /// Floor quads (type 0x21: surface 1, sound class 1) at height `z` over the cells `x0..x1 × y0..y1`
    /// (cell units of 4), plus, per `step`, a raised block `(cx0, cx1, dz)` spanning all y from cell x `cx0`.
    pub fn floor(z: f32, x0: i16, x1: i16, y0: i16, y1: i16) -> Collision {
        let cz = ((z - 2.0) / 4.0).floor() as i16;
        let mut cells = Vec::new();
        for cx in x0..x1 {
            for cy in y0..y1 {
                let (x, y) = (cx as f32 * 4.0, cy as f32 * 4.0);
                cells.push(cell([cx, cy, cz], &[[x, y, z], [x, y + 4.0, z], [x + 4.0, y + 4.0, z], [x + 4.0, y, z]], &[([0, 1, 2, 3], 0x21)]));
            }
        }
        mesh(cells)
    }

    /// A camera looking along +x (rows forward (1,0,0), left (0,1,0), up (0,0,1)), yaw 0.
    pub fn cam_x() -> ([V4; 3], Pf) {
        let o = Pf::ONE;
        let z = Pf::ZERO;
        ([[o, z, z, z], [z, o, z, z], [z, z, o, z]], z)
    }

    /// Hero + moby + pad + anim driven tick by tick.
    pub struct Runner {
        pub hero: Hero,
        pub moby: Moby,
        pub pad: PadState,
        pub anim: RecordingAnim,
        pub rng: Rng,
        pub cam_rows: [V4; 3],
        pub cam_yaw: Pf,
        pub log: Vec<(i32, i32, [f32; 3])>,
        /// The level water tables the probe sees (`Env::water`).
        pub water: Option<Box<dyn super::swim::WaterQuery>>,
    }

    impl Runner {
        pub fn new(pos: [f32; 3], yaw: f32) -> Runner {
            let hero = Hero::spawn(pos, yaw);
            let mut moby = Moby::zeroed();
            moby.position = hero.pos.map(Pf::to_f32);
            let (cam_rows, cam_yaw) = cam_x();
            Runner { hero, moby, pad: PadState::default(), anim: RecordingAnim::default(), rng: Rng::new(), cam_rows, cam_yaw, log: Vec::new(), water: None }
        }

        /// One tick with `input`.
        pub fn tick(&mut self, coll: &Collision, input: PadInput) -> HeroTick {
            self.pad.update(Some(&input.bytes()), false);
            let env = Env { coll, pad: &self.pad, cam_yaw: self.cam_yaw, cam_rows: self.cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: self.water.as_deref(), world: None };
            let r = hero_update(&mut self.hero, &mut self.moby, &env, &mut self.anim, &mut self.rng);
            self.log.push((self.hero.state, self.hero.timer, self.hero.position()));
            r
        }

        pub fn run(&mut self, coll: &Collision, input: PadInput, n: usize) {
            for _ in 0..n { self.tick(coll, input); }
        }

        /// Run `f` with the SetState context of the last tick's pad (e.g. a SetState from outside the tick).
        pub fn with_ctx<R>(&mut self, coll: &Collision, f: impl FnOnce(&mut Hero, &mut states::Ctx) -> R) -> R {
            let env = Env { coll, pad: &self.pad, cam_yaw: self.cam_yaw, cam_rows: self.cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: self.water.as_deref(), world: None };
            let mut c = states::Ctx { env: &env, anim: &mut self.anim, rng: &mut self.rng, voice: None };
            f(&mut self.hero, &mut c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::physics::DT;
    use super::testkit::*;
    use super::*;
    use crate::pad::{button, PadInput};

    #[test]
    fn idle_on_flat_ground_stays_put() {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 120);
        assert_eq!(r.hero.state, 0);
        assert_eq!(r.hero.air_ticks, 0);
        assert_eq!(r.hero.position(), [410.0, 410.0, 100.0]);
    }

    #[test]
    fn run_ramp_and_brake() {
        let coll = floor(100.0, 100, 112, 100, 104);
        let mut r = Runner::new([404.0, 408.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        let fwd = PadInput::neutral().stick(0.0, -1.0);
        let mut speeds = Vec::new();
        for _ in 0..70 { r.tick(&coll, fwd); speeds.push(r.hero.speed.to_f32()); }
        let first = speeds.iter().position(|&s| s > 0.0).unwrap();
        let top = speeds.iter().position(|&s| s >= 0.095).unwrap();
        eprintln!("accel from tick {first}, 5.7 u/s at tick {top}; states {:?}", r.log[..6].iter().map(|l| l.0).collect::<Vec<_>>());
        eprintln!("{:?}", &speeds[..8]);
        assert_eq!(r.hero.speed, Pf::b(0x3dc2_8f5c)); // 5.7·dt
        let x0 = r.hero.position()[0];
        let mut n = 0;
        while r.hero.speed != Pf::ZERO && n < 200 { r.tick(&coll, PadInput::neutral()); n += 1; }
        eprintln!("stopped after {n} ticks, {} units, state {}", r.hero.position()[0] - x0, r.hero.state);
    }

    /// Ticks from the ✕ press to the apex / touchdown and the apex height for a jump held for `hold` ticks.
    fn jump_profile(state_id: i32, hold: usize) -> (usize, f32, usize) {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        if state_id == 9 {
            // Straight into 9 through SetState (a running jump from rest).
            let env_pad = r.pad.clone();
            let env = Env { coll: &coll, pad: &env_pad, cam_yaw: r.cam_yaw, cam_rows: r.cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
            let mut c = states::Ctx { env: &env, anim: &mut r.anim, rng: &mut r.rng, voice: None };
            r.hero.set_state(&mut c, 9, true);
        }
        // T = 0 is the first tick that runs in the jump state (the tick after the press for 7).
        let n0 = r.log.len() + if state_id == 9 { 0 } else { 1 };
        for i in 0..80 {
            let inp = if i < hold { PadInput::neutral().press(button::CROSS) } else { PadInput::neutral() };
            r.tick(&coll, inp);
        }
        let zs: Vec<f32> = r.log[n0..].iter().map(|l| l.2[2] - 100.0).collect();
        let apex = zs.iter().copied().fold(0.0, f32::max);
        let t_apex = zs.iter().position(|&z| z == apex).unwrap();
        let t_land = t_apex + zs[t_apex..].iter().position(|&z| z == 0.0).unwrap();
        (t_apex, apex, t_land)
    }

    /// The §4.4 model numbers (T counted from the first tick in the jump state; the press tick is T = 0).
    #[test]
    fn jump_heights_match_the_model() {
        let (ta, a, tl) = jump_profile(7, 1);
        assert_eq!((ta, tl), (21, 38));
        assert!((a - 1.2268).abs() < 2e-4, "tap apex {a}");
        let (ta, a, tl) = jump_profile(7, 20);
        eprintln!("hold: apex {a} at {ta}, lands {tl}");
        assert_eq!((ta, tl), (27, 48));
        assert!((a - 2.0545).abs() < 2e-4, "held apex {a}");
        let (ta, a, tl) = jump_profile(9, 0);
        eprintln!("state 9 tap: apex {a} at {ta}, lands {tl}");
        assert!((a - 0.845).abs() < 2e-3, "state 9 tap apex {a}");
        let (_, a, _) = jump_profile(9, 20);
        assert!((a - 2.056).abs() < 5e-3, "state 9 held apex {a}");
    }

    /// Falling: coyote 4 ticks, then vz −= 24·dt² per tick. The move's velocity clamp |vel| ≤ r − 0.02
    /// (0.43 u/tick = 25.8 u/s) binds long before the fall state's −50·dt terminal speed.
    #[test]
    fn fall_speed_is_capped_by_the_move_clamp() {
        let coll = floor(10.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 300.0], 0.0);
        let mut vz = Vec::new();
        for _ in 0..140 { r.tick(&coll, PadInput::neutral()); vz.push(r.hero.vel[2]); }
        let first6 = r.log.iter().position(|l| l.0 == 6).unwrap();
        assert_eq!(first6, 4);
        let d = (vz[40] - vz[41]).to_f32();
        assert!((d - 24.0 / 3600.0).abs() < 1e-6, "{d}");
        let cap = -(r.hero.cap_radius - Pf::b(0x3ca3_d70a));
        let t = vz.iter().position(|&v| v == cap).unwrap();
        eprintln!("vz capped at {:?} from tick {t}", cap);
        assert!((60..=70).contains(&t), "{t}");
        assert!(vz[t..].iter().all(|&v| v == cap));
    }

    #[test]
    fn idle_momentum_decays_in_28_ticks() {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        r.hero.momentum = [DT * Pf::b(0x40b6_6666), Pf::ZERO, Pf::ZERO, Pf::ZERO];
        let mut n = 0;
        while r.hero.momentum[0] != Pf::ZERO && n < 100 { r.tick(&coll, PadInput::neutral()); n += 1; }
        eprintln!("momentum gone after {n} ticks");
        assert!((28..=30).contains(&n), "momentum gone after {n} ticks");
    }

    #[test]
    fn double_jump_window() {
        let coll = floor(100.0, 100, 106, 100, 106);
        // A second tap at T = 20 (> 14, before the apex) → 0xe on that tick.
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        r.tick(&coll, PadInput::neutral().press(button::CROSS));
        r.run(&coll, PadInput::neutral(), 19);
        assert_eq!(r.hero.state, 7);
        r.tick(&coll, PadInput::neutral().press(button::CROSS));
        assert_eq!(r.hero.state, 0xe);
        // A tap at T = 8 is still in the buffer at T = 15 (window clamp(T − 4, 2, 30) = 11).
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        r.tick(&coll, PadInput::neutral().press(button::CROSS));
        r.run(&coll, PadInput::neutral(), 7);
        r.tick(&coll, PadInput::neutral().press(button::CROSS));
        let mut first = None;
        for i in 0..12 { r.tick(&coll, PadInput::neutral()); if r.hero.state == 0xe && first.is_none() { first = Some(9 + i); } }
        assert_eq!(first, Some(15), "buffered double jump");
        // No second tap: no double jump.
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        r.tick(&coll, PadInput::neutral().press(button::CROSS));
        r.run(&coll, PadInput::neutral(), 40);
        assert!(r.log.iter().all(|l| l.0 != 0xe));
    }

    #[test]
    fn jump_buffer_in_idle_is_7_ticks() {
        let coll = floor(100.0, 100, 106, 100, 106);
        // ✕ pressed during the last ticks of a stop (state 3 needs a press on the tick itself) is picked up
        // by idle within 7 ticks: pad history semantics.
        let mut p = crate::pad::PadState::default();
        p.update(Some(&PadInput::neutral().press(button::CROSS).bytes()), false);
        for _ in 0..6 { p.update(Some(&PadInput::neutral().bytes()), false); }
        assert_eq!(p.pressed_within(button::CROSS, 7), Some(6));
        p.update(Some(&PadInput::neutral().bytes()), false);
        assert_eq!(p.pressed_within(button::CROSS, 7), None);
        let _ = coll;
    }

    #[test]
    fn unported_state_freezes() {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        // The look stance 1 is ported (package P2): L1 enters it and it runs. A state no package ports yet (the
        // weapon stance 0x17) freezes the hero.
        r.tick(&coll, PadInput::neutral().press(button::L1));
        assert_eq!(r.hero.state, 1);
        assert_eq!(r.tick(&coll, PadInput::neutral().press(button::L1)), HeroTick::Ran);
        r.with_ctx(&coll, |h, c| h.set_state(c, 0x17, true));
        assert_eq!(r.tick(&coll, PadInput::neutral()), HeroTick::Unimplemented(0x17));
        assert_eq!(r.hero.unimplemented_seen, vec![0x17]);
    }

    #[test]
    fn crouch_and_flip() {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        r.run(&coll, PadInput::neutral().press(button::R1), 10);
        assert_eq!(r.hero.state, 4);
        // Crouch held, stick pushed back (facing +x, camera +x: back = stick down), then ✕ → flip 0xb.
        r.tick(&coll, PadInput::neutral().press(button::R1).stick(0.0, 1.0));
        r.tick(&coll, PadInput::neutral().press(button::R1 | button::CROSS).stick(0.0, 1.0));
        assert_eq!(r.hero.state, 0xb);
        assert_eq!(r.anim.calls.last().map(|c| c.1), Some(0x1f));
    }

    #[test]
    fn jump_from_rest() {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 2);
        r.tick(&coll, PadInput::neutral().press(button::CROSS));
        r.run(&coll, PadInput::neutral(), 70);
        let apex = r.log.iter().map(|l| l.2[2]).fold(0.0f32, f32::max);
        assert!((apex - 100.0 - 1.2268).abs() < 2e-4);
        // 7 → (lands at T = 38) → the landing picker → idle once the landing anim is done (here: 12 ticks).
        let states: Vec<i32> = r.log.iter().map(|l| l.0).collect();
        assert_eq!(states.iter().filter(|&&s| s == 7).count(), 51);
        assert_eq!(*states.last().unwrap(), 0);
    }
}
