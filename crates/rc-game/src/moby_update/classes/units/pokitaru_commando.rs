//! U353: class 114, Pokitaru's commando (level11 `0x2d0fa8`, the only copy: 1 placed, instance #45), and the gate 65
//! he opens (U350, level11 `0x2cb810`). The name is descriptive [L]. The commando is the boats' driver (G-ENM-011): a
//! talking NPC (the talk system's block at pvar +0x20, talk slot 1) who, once talked to, follows Ratchet through four
//! **phases** (+0x158), each an area of the beach (a polygon path +0x60[k], its waypoint graph +0x70[k], extra walls
//! +0x80..+0x88 in phases 2 / 3) with the enemy groups to clear (+0x90 + 0x10·k: four s16 group indices). While enemies
//! are alive he keeps near Ratchet (walk 1.5 / run 5 u/s through the graph, jumping down ledges inside the hazard
//! cuboids +0xf8 / +0xfc); when none is left he walks to the phase's point (cuboid +0xe0[k]) and: phases 0 / 1 jump onto
//! boat 0 / 1 (+0xf0 / +0xf4), walk to the helm (3.2, 0, 0.95 in the boat's frame) and **start the boat** when Ratchet
//! stands on it (`pokitaru_boat::try_start`), ride it to its end (`arrived`), step off (−3, 0, 1) and jump to the next
//! phase's point (cuboid +0xd0[k + 1]); phase 2 waits for the cutaway machine 1157 (+0x100) to finish (its state 5);
//! phase 3 talks again (the scene of the next node: mission +0x10c done, global flag 84, the gate 65 (+0x104) opened
//! at the scene's tick 2000) and finally hands over the **O2 Mask** (item 6) on a last talk: `GiveItem(6, 1)`, the save,
//! the help message 11000, gone. Every tick he moves with the creature walker's collision, kept inside the phase's
//! walls (`region::push_out`), turns his head and torso toward Ratchet, and drops a shadow near the camera.
//!
//! **Pvars** (0x2e0): +0x20 the talk block (radius +0x2c, node +0x56), +0x60[4] the areas, +0x70[4] the graphs, +0x80 /
//! +0x84 / +0x88 the walls, +0x90 + 0x10·k the groups, +0xd0.. / +0xe0.. cuboid indices (the phases' points), +0xf0 /
//! +0xf4 the boats, +0xf8 / +0xfc the hazard cuboids, +0x100 the cutaway machine 1157, +0x104 the gate 65, +0x108 /
//! +0x10c / +0x118 missions, +0x110 / +0x114 the cuboids moved 500 down while not wanted, +0x120 the target point,
//! +0x130 the velocity, +0x140 the last move, +0x150 the turn velocity, +0x154 the speed (per tick), +0x158 the phase,
//! +0x15c the state after a jump, +0x160 / +0x1e0 the head / torso look-at records (joint lists 0 / 1).
//!
//! The level11 words: gp−0x5830 1.5 / gp−0x582c 5.0 (walk / run speed), gp−0x5820 (3.2, 0, 0.95) the helm, gp−0x5810 (0,
//! 0, 0) its Euler, gp−0x5800 (−3, 0, 1) the step-off point, gp−0x57f0 / −0x57ec / −0x57e8 1, 2, 0 (the boarding point's
//! rows 2 / 1 / 0 lengths).
//!
//! ## Coverage
//!
//! **The update** `0x2d0fa8`:
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | `0x2d0ec0` | the 0x15edb0 cheat in scenes: the scene actor of this class gets a manipulator at 2.75 | [`update`] (`manip::scene_big_head`) |
//! | drawn and within 38 (3-D) of the camera (0x1677c0): `0x280000` (= `0x26f020`), +0x7f = 0x1e | the shadow probe | [`update`] (`shadows::probe_down`) |
//! | `0x2d1030`, `0x2d1340`, `0x2d14b0`, `0x2d27b0` in that order | motion, move, the states, the look-at | [`motion`], [`moving`], [`brain`], [`look`] |
//! | (port) the end of the talker's scene: `NpcTalkRefresh(m, +0x20, 1)` (the game's scene end calls it) | | [`update`] (`interact::poll_scene_end_at`) |
//!
//! **The motion** `0x2d1030`:
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | 4, 5, 6, 8, 10: `0x281ca0(atan(+0x120 − pos), 4π·dt², 8π·dt², 4π·dt, &yaw, +0x150)` (= `0x270cc0`); +0x130 = (cos, sin)·+0x154, vz = +0x148 − 20·dt² (≤ 0) | walking | [`motion`] (`turn::turn_toward_pvar`) |
//! | 7: key 9 passed (`0x286cb0` = `0x2765b0`): +0x130 = (+0x120 − pos) / `ticks(60)`, vz `0x280ad0`(xy speed, −10.8·dt², pos, +0x120) (= `0x26faf0`), anim speed 2/3; moving → vz −= 10.8·dt²; key 29 passed → +0x130 = 0, anim speed 1 | the jump | [`motion`] (`ground::passed_frame`, `knock::lob_up`) |
//! | 9: `0x285ab0(m, boat, helm, 0, pos, rot)` (= `0x2753b0`); `0x30a480` (`pokitaru_boat::try_start`) → seq 5 (`ticks(20)`) unless on it | riding: starts the boat | [`motion`] (`triggers::from_local`) |
//! | 0xd: scene 1 (0x16d290) at its tick `ticks(2000)` (0x16d294) → `0x2cb990(gate)` | the gate opens in the scene | [`motion`], [`open_gate`] |
//!
//! **The move** `0x2d1340`: `0x27e660(0.5, 0.3, 0, m, &vel, 0x10)` (= `0x26d610`, the walker's move with collision, on a
//! copy of +0x130); states 4..6, and 7 when the jump goes back to 3: `0x287548(0.3, area)` (`region::push_out`), and
//! the walls +0x80 / +0x84 / +0x88 (phase 2) or +0x80 / +0x84 (phase 3); +0x140 = pos − old. [`moving`].
//!
//! **The states** `0x2d14b0` (entered through `0x2d2088`, [`set_state`]: 1, 3, 0xb, 0xc, 0xe seq 0 (`ticks(20)`),
//! velocity 0, 0xe: talk radius 10 and node 2; 4: seq 2, speed 1.5·dt; 5 / 6: seq 3, speed 5·dt; 7: seq 4, velocity 0;
//! 8: seq 3, +0x120 = the helm of the phase's boat; 9 / 0x10: seq 0, velocity 0; 10: seq 3, +0x120 = the step-off point):
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | `0x2d2460`: the phase's groups: 2 when a live member (not a 1246 held / dying / waiting, `0x316128`) is within 10 (3-D), 1 when one lives, else 0 | | [`enemies`] |
//! | 3..5 and no enemy → 6 | | [`brain`] |
//! | 4..6: 3 ahead (row 0); `0x2d25e8`: in a hazard cuboid (`0x2851a0`), the probe (clamped to 4) inside the area (`0x27f6c0`), its ground (`0x27f690` = `0x26e690`, from 5 above) more than 0.5 off and the line from 0.15 above the feet to 0.15 above it blocked (`CollLine_Fix`, flags 2) → +0x120 = (probe, ground), +0x128 = its ground, +0x15c 3, 7 | jump down a ledge | [`brain`], [`ledge`] |
//! | 0: `NpcTalkRegister(m, +0x20)`; update distance 0xff; `0x2873b0` the graphs 0..2 (`region::graph_init`); +0x130 / +0x140 = 0 | init | [`init`] |
//! | 0: O2 Mask not acquired (0x13d4ee): mission +0x10c done → phase 3 at cuboid +0xec, the gate opened, 0xe, radius 10, both boats `skip_to_end` (`0x30a840`); acquired: mission +0x10c done → both boats skipped, the gate opened, `DeleteMoby` | resume after the mission | [`init`] |
//! | 0: mission +0x108 done and Ratchet within 12 (3-D) of cuboid +0xe4 → phase 1 there, 6, cuboid +0x114 500 down, boat 0 skipped | | [`init`] |
//! | 0: else cuboids +0x110 / +0x114 500 down, phase 0, radius 5, 1 | | [`init`] |
//! | 1: node 0: Ratchet on the ground (0x13f65e = 0) and `NpcTalkUpdate` → `0x288e98(2.5, m)` (= `0x2783a8`, placed after the scene), 2; another node: Ratchet within 6 (xy) → 3 | talk | [`brain`] (`interact::talk_update_at`, `interact::place_after_scene`) |
//! | 2: game mode ≠ 2 → 3 | the scene ends | [`brain`] |
//! | 3: Ratchet beyond 6 (xy), reachable (`0x2d2340`: in the area, `0x287b30` = `LineOfSightTest`(0.2, the walls, the graph) → +0x120) and enemies none within 10 → 4 | wait | [`brain`], [`reach`] |
//! | 4: Ratchet within 3, unreachable or an enemy within 10 → 3; beyond 8 → 5 | walk | [`brain`] |
//! | 5: the same; within 4 → 4 | run | [`brain`] |
//! | 6: reach the phase's point (cuboid +0xe0[k]); within 1 (xy): phase 0: +0x120 = boat 0 + 2·row 1 + 1·row 2 (+ 0·row 0), +0x15c 8, 7; phase 1: boat 1 with −2·row 1, cuboid +0x110 back up, +0x15c 8, 7; 2 → 0xb; 3 → cuboid +0x114 back up, 0xc | to the phase's point | [`brain`] |
//! | 7: anim done → +0x15c | | [`brain`] |
//! | 8: within 0.5 (xy) of the helm → 9 | | [`brain`] |
//! | 9: `0x30a830` (`pokitaru_boat::arrived`) → 10 | | [`brain`] |
//! | 10: within 0.5 of the step-off point → phase + 1, +0x120 = cuboid +0xd0[phase] centre, +0x15c 4, 7 | | [`brain`] |
//! | 0xb: `0x30e078` (1157 in state 5) → phase + 1, 3 | | [`brain`] |
//! | 0xc: talk radius 255; `NpcTalkUpdate` → `SetMissionDone(+0x10c)` (`0x2760d0`), flag 0x13d3dc (84) = 1, `0x28bc28(m, 2)` (the talked word), at cuboid +0xe0[k] (pos, Euler, rows), the scene's end place 2 ahead facing back (0x16d270.. / 0x16d2a6), `0x288e98(2.5, m)`, 0xd | the last scene | [`brain`] (`cinematic::set_mission_done`, `interact::set_global_flag`, `interact::set_talked`) |
//! | 0xd: game mode ≠ 2 → `0x2cb990(gate)`, 0xe | | [`brain`] |
//! | 0xe: mission +0x118 done, Ratchet not in 0x32 and `NpcTalkUpdate` → `0x288e98(2.5, m)`, 0xf | the reward talk | [`brain`] |
//! | 0xf: game mode ≠ 2 → `GiveItem(6, 1)` (`0x285e60`), `memcard_Save(0, −1)`, `Help_Request(11000, 0x3a)` (`0x235270`), `DeleteMoby` | the O2 Mask | [`brain`] (`interact::give_item`, `cinematic::save`, `help`) |
//!
//! **The look-at** `0x2d27b0`: in 0..5, 0xb, 0xc, 0xe, 0x10: from 1 above the feet to Ratchet: yaw (relative, ±70°) and
//! pitch (−atan, clamped to [−30°, 15°]): head record pitch = it, yaw ·0.7, torso yaw ·0.3; the 0x15edb0 cheat's head
//! scale 2.75 (P+0x1d0); `0x288320(0.02, 0.3, m, +0x160, 0)` / `(…, +0x1e0, 1)` (= `0x2777d8`). [`look`].
//!
//! **The gate 65** (U350, `0x2cb810`, pvars 0x10: +0x00 the turn velocity, +0x04 the closed yaw, +0x08 the twin): 0 → 1,
//! +0x04 = yaw; not mirrored: `CreateMoby(65)` the twin (draw / update distance, mode \| 0x8000, position, rotation; its
//! +0x04 = the yaw), +0x08 = it. 2: `0x281ca0(+0x04 ± 0.2077, 20°·dt², 20°·dt², 45°·dt, &yaw, +0x00)` (+ unmirrored, −
//! mirrored); a turn of 0 → 3. 1 / 3: nothing. `0x2cb990(gate)` (the commando's): a 65 in state 1 → flag 0x13d3e2 (90) = 1,
//! state 2, its twin state 2. [`gate_update`], [`open_gate`].
//!
//! **Depends on** 1157 (the level11 cutaway machine at
//! 0x30d800, `units::pokitaru_cutaway`, ported 2026-10-01): phase 2 (state 0xb) ends when it reaches its state 5.
//!
//! **Not the game's, noted [L]:** the moby links (the gate's twin) are kept as index + 1; `0x285ab0` writes back the helm
//! pose when the boat's block flag 4 is set (never: the boats set flag 1), here a copy; the w lane of a pushed-out
//! position keeps the old one.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::units::pokitaru_boat as boat;
use crate::moby_update::creature::{self as c, ground, knock, region, turn, walker};
use crate::moby_update::interact;
use crate::moby_update::services::World;
use crate::moby_update::triggers;

/// The update in the level11 class table.
pub const UPDATE_FN: u32 = 0x2d_0fa8;
/// The gate 65's update.
pub const GATE_FN: u32 = 0x2c_b810;
pub const REFERENCE_LEVEL: u32 = 11;
pub const CLASSES: [i16; 1] = [114];
pub const GATE_CLASSES: [i16; 1] = [GATE];
/// The gate (0x41) and the boarders the enemy count skips when held / dying / waiting (0x4de).
pub const GATE: i16 = 65;
pub const BOARDER: i16 = 1246;
/// The O2 Mask (`GiveItem(6, 1)`), the help message and its record of the end.
pub const REWARD_ITEM: usize = 6;
pub const HELP_MSG: i32 = 11000;
pub const HELP_REC: i32 = 0x3a;
/// The global flags written: 0x13d3dc (84, the last scene) and the gate's 0x13d3e2 (90).
pub const FLAG_SCENE: usize = 84;
pub const FLAG_GATE: usize = 90;

/// Pvar offsets (module doc).
pub mod pv {
    pub const TALK: usize = 0x20;
    pub const AREA: usize = 0x60;
    pub const GRAPH: usize = 0x70;
    pub const WALLS: usize = 0x80;
    pub const GROUPS: usize = 0x90;
    pub const POINTS_D0: usize = 0xd0;
    pub const POINTS_E0: usize = 0xe0;
    pub const BOATS: usize = 0xf0;
    pub const HAZARDS: usize = 0xf8;
    pub const MACHINE: usize = 0x100;
    pub const GATE: usize = 0x104;
    pub const MISSION_A: usize = 0x108;
    pub const MISSION_B: usize = 0x10c;
    pub const CUBOID_A: usize = 0x110;
    pub const CUBOID_B: usize = 0x114;
    pub const MISSION_C: usize = 0x118;
    pub const TARGET: usize = 0x120;
    pub const VEL: usize = 0x130;
    pub const MOVE: usize = 0x140;
    pub const TURN_V: usize = 0x150;
    pub const SPEED: usize = 0x154;
    pub const PHASE: usize = 0x158;
    pub const AFTER_JUMP: usize = 0x15c;
    pub const HEAD: usize = 0x160;
    pub const TORSO: usize = 0x1e0;
    pub const SIZE: usize = 0x2e0;
}

/// The level11 words (module doc).
pub mod k {
    pub const WALK: f32 = 1.5;
    pub const RUN: f32 = 5.0;
    pub const HELM: [f32; 3] = [3.2, 0.0, 0.95];
    pub const STEP_OFF: [f32; 3] = [-3.0, 0.0, 1.0];
    /// The boarding point's lengths along the boat's rows 0 / 1 / 2 (row 1 negated for boat 1).
    pub const BOARD: [f32; 3] = [0.0, 2.0, 1.0];
    /// The gate's opening (0x3e54adc8 rad).
    pub const GATE_OPEN_BITS: u32 = 0x3e54_adc8;
}

fn state(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o)).ok().filter(|&m| m < w.table.mobys.len()) }
fn phase(w: &World, id: MobyId) -> usize { (c::pi32(w, id, pv::PHASE) & 3) as usize }
fn mission_done(w: &World, m: i32) -> bool { u8::try_from(m).is_ok_and(|m| w.mission_done(w.svc.level, m) == 0xff) }
fn hero(w: &World) -> c::V { crate::moby_update::classes::units::hero_pos(w) }
fn heading_to(p: c::V, t: c::V) -> f32 { c::atan(t[0] - p[0], t[1] - p[1]) }
fn spline(w: &World, i: i32) -> Option<usize> { usize::try_from(i).ok().filter(|&p| p < w.svc.splines.len()) }

/// A level cuboid's centre and Euler angles.
fn cuboid(w: &World, i: i32) -> Option<([f32; 3], [f32; 3])> {
    let s = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, i)?;
    Some((s.centre(), s.euler))
}

/// A cuboid moved by `dz` (its centre, `0x1600ec + i·0x80 + 0x38`): the game's way of switching a trigger off (−500) and
/// on (+500).
fn shift_cuboid(w: &mut World, i: i32, dz: f32) {
    let v = std::sync::Arc::make_mut(&mut w.svc.volumes);
    if let Some(s) = usize::try_from(i).ok().and_then(|i| v.cuboids.get_mut(i)) { s.matrix[3][2] += dz; }
}

/// The moby at the cuboid: position = its centre, rotation = its Euler.
fn to_cuboid(w: &mut World, id: MobyId, i: i32) {
    let Some((p, e)) = cuboid(w, i) else { return };
    let m = w.mm(id);
    m.position = [p[0], p[1], p[2], m.position[3]];
    m.rotation = [e[0], e[1], e[2], m.rotation[3]];
}

/// The boat of the phase (`+0xf0 + 4·(phase ≠ 0)`).
fn phase_boat(w: &World, id: MobyId) -> Option<MobyId> { link(w, id, pv::BOATS + if c::pi32(w, id, pv::PHASE) != 0 { 4 } else { 0 }) }

/// `boat.pos + rows·l` (`fun_001f9cf8(out, l, boat + 0xc0)`, `VecAdd`).
fn boat_point(w: &World, b: MobyId, l: [f32; 3]) -> c::V {
    let m = w.m(b);
    let r = m.rows;
    let p = m.position;
    [p[0] + l[0] * r[0][0] + l[1] * r[1][0] + l[2] * r[2][0], p[1] + l[0] * r[0][1] + l[1] * r[1][1] + l[2] * r[2][1], p[2] + l[0] * r[0][2] + l[1] * r[1][2] + l[2] * r[2][2], p[3]]
}

/// `0x2d2088(m, s)`: the state change with its sequence and speed (module doc).
pub fn set_state(w: &mut World, id: MobyId, s: u8) {
    w.mm(id).state = s;
    let blend = |w: &mut World, seq: u8| {
        let t = w.ticks(20);
        c::blend_to(w, id, seq, 0, t);
    };
    match s {
        1 | 3 | 0xb | 0xc | 0xe => {
            blend(w, 0);
            if s == 0xe { c::set_pf(w, id, pv::TALK + interact::talk::RADIUS, 10.0); }
            c::set_pv4(w, id, pv::VEL, [0.0; 4]);
            if s == 0xe { c::set_pi16(w, id, pv::TALK + interact::talk::NODE, 2); }
        }
        4 => {
            blend(w, 2);
            c::set_pf(w, id, pv::SPEED, k::WALK * c::DT);
        }
        5 | 6 => {
            blend(w, 3);
            c::set_pf(w, id, pv::SPEED, k::RUN * c::DT);
        }
        7 => {
            blend(w, 4);
            c::set_pv4(w, id, pv::VEL, [0.0; 4]);
        }
        8 | 10 => {
            blend(w, 3);
            if let Some(b) = phase_boat(w, id) {
                let p = boat_point(w, b, if s == 8 { k::HELM } else { k::STEP_OFF });
                c::set_pv4(w, id, pv::TARGET, p);
            }
        }
        9 | 0x10 => {
            blend(w, 0);
            c::set_pv4(w, id, pv::VEL, [0.0; 4]);
        }
        _ => {}
    }
}

/// `0x2d2460(m)`: the enemies of the phase's groups (module doc): 0 none, 1 alive, 2 one within 10.
fn enemies(w: &World, id: MobyId) -> i32 {
    let base = pv::GROUPS + 0x10 * phase(w, id);
    let mut r = 0;
    for i in 0..4 {
        let g = c::pi16(w, id, base + 4 * i);
        if g < 0 { return r; }
        let Ok(g) = i8::try_from(g) else { return r };
        if w.svc.groups.lists.get(g as usize).is_none() { return r; }
        for m in crate::moby_update::scheduler::group_ids(w, g) {
            let Some(o) = w.table.mobys.get(m) else { continue };
            if (o.state as i8) < 0 { continue; }
            if o.o_class == BOARDER && [0x14, 0xb, 0x12, 0xc].contains(&o.state) { continue; }
            r = 1;
            if c::dist3(c::pos(w, id), o.position) < 10.0 { return 2; }
        }
    }
    r
}

/// The phase's walls: its area, and +0x80 / +0x84 / +0x88 in phase 2, +0x80 / +0x84 in phase 3.
fn walls(w: &World, id: MobyId) -> Vec<usize> {
    let ph = phase(w, id);
    let mut v = vec![c::pi32(w, id, pv::AREA + 4 * ph)];
    match ph {
        2 => v.extend([0, 4, 8].map(|o| c::pi32(w, id, pv::WALLS + o))),
        3 => v.extend([0, 4].map(|o| c::pi32(w, id, pv::WALLS + o))),
        _ => {}
    }
    v.into_iter().filter_map(|p| spline(w, p)).collect()
}

/// `0x2d2340(m, p)`: `p` inside the phase's area and a route to it (`LineOfSightTest(0.2, walls, graph)`): +0x120 = the
/// way point. False: outside, or no route.
fn reach(w: &mut World, id: MobyId, p: c::V) -> bool {
    let ph = phase(w, id);
    let Some(area) = spline(w, c::pi32(w, id, pv::AREA + 4 * ph)) else { return false };
    if !region::point_in_polygon(w, area, p) { return false; }
    let ws = walls(w, id);
    let Some(graph) = spline(w, c::pi32(w, id, pv::GRAPH + 4 * ph)) else { return false };
    match region::line_of_sight(w, 0.2, &ws, graph, c::pos(w, id), p) {
        Some(q) => {
            c::set_pv4(w, id, pv::TARGET, q);
            true
        }
        None => false,
    }
}

/// `0x2d25e8(m, &probe)`: the ledge jump target (module doc).
fn ledge(w: &World, id: MobyId, probe: c::V) -> Option<c::V> {
    let me = c::pos(w, id);
    let inside = (0..2).any(|i| w.in_cuboid([me[0], me[1], me[2]], c::pi32(w, id, pv::HAZARDS + 4 * i)));
    if !inside { return None; }
    let mut d = c::sub(probe, me);
    if 4.0 < c::len3(d) { d = c::set_len3(d, 4.0); }
    let mut q = c::add(d, me);
    let area = spline(w, c::pi32(w, id, pv::AREA + 4 * phase(w, id)))?;
    if !region::point_in_polygon(w, area, q) { return None; }
    q[2] += 5.0;
    let g = ground::ground(w, q, 0.5, 0x20).z;
    if (g - me[2]).abs() <= 0.5 { return None; }
    let from = [me[0], me[1], me[2] + 0.15, me[3]];
    let to = [q[0], q[1], g + 0.15, q[3]];
    let pv = crate::moby_update::services::pv;
    w.coll_line(pv(from), pv(to), 2, Some(id))?;
    Some([q[0], q[1], g, me[3]])
}

/// `0x2cb990(gate)`: a gate 65 in state 1 opens (module doc).
pub fn open_gate(w: &mut World, gate: Option<MobyId>) {
    let Some(g) = gate else { return };
    if w.m(g).o_class != GATE || w.m(g).state != 1 { return; }
    interact::set_global_flag(w, FLAG_GATE, 1);
    w.mm(g).state = 2;
    if w.m(g).pvars.len() >= 0xc {
        let twin = c::pi32(w, g, 8) - 1;
        if let Some(t) = usize::try_from(twin).ok().filter(|&t| t < w.table.mobys.len()) { w.mm(t).state = 2; }
    }
}

/// `0x30a840` on both boats.
fn skip_boats(w: &mut World, id: MobyId) {
    for o in [0, 4] {
        if let Some(b) = link(w, id, pv::BOATS + o) { boat::skip_to_end(w, b); }
    }
}

/// State 0 (module doc).
fn init(w: &mut World, id: MobyId) {
    interact::talk_register_at(w, id, pv::TALK);
    w.mm(id).update_dist = 0xff;
    for i in 0..3 {
        let a = spline(w, c::pi32(w, id, pv::AREA + 4 * i));
        let g = spline(w, c::pi32(w, id, pv::GRAPH + 4 * i));
        if let Some(g) = g { region::graph_init(w, &a.into_iter().collect::<Vec<_>>(), g); }
    }
    c::set_pv4(w, id, pv::VEL, [0.0; 4]);
    c::set_pv4(w, id, pv::MOVE, [0.0; 4]);
    let acquired = w.svc.interact.game.acquired.get(REWARD_ITEM).copied().unwrap_or(0) != 0;
    let gate = link(w, id, pv::GATE);
    if mission_done(w, c::pi32(w, id, pv::MISSION_B)) {
        if !acquired {
            c::set_pi32(w, id, pv::PHASE, 3);
            to_cuboid(w, id, c::pi32(w, id, pv::POINTS_E0 + 0xc));
            open_gate(w, gate);
            set_state(w, id, 0xe);
            c::set_pf(w, id, pv::TALK + interact::talk::RADIUS, 10.0);
            skip_boats(w, id);
        } else {
            skip_boats(w, id);
            open_gate(w, gate);
            w.delete_moby(id);
        }
        return;
    }
    if mission_done(w, c::pi32(w, id, pv::MISSION_A)) {
        if let Some((cp, _)) = cuboid(w, c::pi32(w, id, pv::POINTS_E0 + 4)) {
            let h = hero(w);
            if c::dist3(h, [cp[0], cp[1], cp[2], 0.0]) < 12.0 {
                c::set_pi32(w, id, pv::PHASE, 1);
                to_cuboid(w, id, c::pi32(w, id, pv::POINTS_E0 + 4));
                set_state(w, id, 6);
                let cb = c::pi32(w, id, pv::CUBOID_B);
                shift_cuboid(w, cb, -500.0);
                if let Some(b) = link(w, id, pv::BOATS) { boat::skip_to_end(w, b); }
                return;
            }
        }
    }
    let (ca, cb) = (c::pi32(w, id, pv::CUBOID_A), c::pi32(w, id, pv::CUBOID_B));
    shift_cuboid(w, ca, -500.0);
    shift_cuboid(w, cb, -500.0);
    c::set_pi32(w, id, pv::PHASE, 0);
    c::set_pf(w, id, pv::TALK + interact::talk::RADIUS, 5.0);
    set_state(w, id, 1);
}

/// `0x2d1030` (module doc).
fn motion(w: &mut World, id: MobyId) {
    match state(w, id) {
        4 | 5 | 6 | 8 | 10 => {
            let h = heading_to(c::pos(w, id), c::pv4(w, id, pv::TARGET));
            let r = 12.566_371;
            turn::turn_toward_pvar(w, id, h, c::DT2 * r, c::DT2 * 25.132_742, c::DT * r, pv::TURN_V);
            let (cy, sy) = c::cs(c::yaw(w, id));
            let sp = c::pf(w, id, pv::SPEED);
            let mut vz = c::pf(w, id, pv::MOVE + 8) - c::DT2 * 20.0;
            if 0.0 < vz { vz = 0.0; }
            let v = c::pv4(w, id, pv::VEL);
            c::set_pv4(w, id, pv::VEL, [cy * sp, sy * sp, vz, v[3]]);
        }
        7 => {
            if ground::passed_frame(w, id, 9.0) {
                let t = c::pv4(w, id, pv::TARGET);
                let p = c::pos(w, id);
                let n = w.ticks(60) as f32;
                let mut v = c::scale(c::sub(t, p), 1.0 / n);
                let mut time = 0.0;
                v[2] = knock::lob_up(c::len2(v), -(c::DT2 * 10.8), p, t, &mut time);
                c::set_pv4(w, id, pv::VEL, v);
                w.mm(id).anim.speed = f32::from_bits(0x3f2a_aaab);
            }
            let v = c::pv4(w, id, pv::VEL);
            if 0.0 < c::len3(v) { c::set_pf(w, id, pv::VEL + 8, v[2] - c::DT2 * 10.8); }
            if !ground::passed_frame(w, id, 29.0) { return; }
            c::set_pv4(w, id, pv::VEL, [0.0; 4]);
            w.mm(id).anim.speed = 1.0;
        }
        9 => {
            let Some(b) = phase_boat(w, id) else { return };
            if let Some(car) = triggers::carrier(w.m(b)) {
                let slot = w.m(id).class_slot;
                let (mut l, mut lr) = (k::HELM, [0.0; 3]);
                let (p, r) = triggers::from_local(&car, slot, &mut l, &mut lr);
                let m = w.mm(id);
                m.position[..3].copy_from_slice(&p);
                m.rotation[..3].copy_from_slice(&r);
            }
            if boat::try_start(w, b) {
                let t = w.ticks(20);
                c::blend_to(w, id, 5, 0, t);
            }
        }
        0xd => {
            let t2000 = w.ticks(2000);
            if w.svc.cinematic.scene.as_ref().is_some_and(|s| s.id == 1 && s.tick == t2000) {
                let g = link(w, id, pv::GATE);
                open_gate(w, g);
            }
        }
        _ => {}
    }
}

/// `0x2d1340` (module doc).
fn moving(w: &mut World, id: MobyId) {
    let old = c::pos(w, id);
    let mut v = c::pv4(w, id, pv::VEL);
    walker::move_collide(w, id, 0.5, 0.3, 0.0, &mut v, 0x10);
    let s = state(w, id);
    if (4..7).contains(&s) || (s == 7 && c::pi32(w, id, pv::AFTER_JUMP) == 3) {
        for p in walls(w, id) {
            if let Some(q) = region::push_out(w, 0.3, p, c::pos(w, id)) { c::set_pos(w, id, q); }
        }
    }
    let d = c::sub(c::pos(w, id), old);
    c::set_pv4(w, id, pv::MOVE, d);
}

/// `0x2d14b0` (module doc).
fn brain(w: &mut World, id: MobyId) {
    let e = enemies(w, id);
    let s = state(w, id);
    if (3..6).contains(&s) && e == 0 {
        set_state(w, id, 6);
        return;
    }
    if (4..7).contains(&s) {
        // `FastVecNormalize(3, &probe, m + 0xc0)` (row 0, 3 long), plus the position.
        let r0 = w.m(id).rows[0];
        let probe = c::add(c::set_len3([r0[0], r0[1], r0[2], 0.0], 3.0), c::pos(w, id));
        if let Some(q) = ledge(w, id, probe) {
            c::set_pv4(w, id, pv::TARGET, q);
            let g = ground::ground(w, q, 0.5, 0x20).z;
            c::set_pf(w, id, pv::TARGET + 8, g);
            c::set_pi32(w, id, pv::AFTER_JUMP, 3);
            set_state(w, id, 7);
        }
    }
    let h = hero(w);
    let d = |w: &World| c::dist2(c::pos(w, id), hero(w));
    match state(w, id) {
        0 => init(w, id),
        1 => {
            if c::pi16(w, id, pv::TALK + interact::talk::NODE) == 0 {
                if w.hero.air_ticks != 0 { return; }
                if !interact::talk_update_at(w, id, pv::TALK) { return; }
                interact::place_after_scene(w, id, 2.5);
                set_state(w, id, 2);
                return;
            }
            if 6.0 <= d(w) { return; }
            set_state(w, id, 3);
        }
        2 => {
            if w.svc.game_mode == 2 { return; }
            set_state(w, id, 3);
        }
        3 => {
            if d(w) <= 6.0 { return; }
            if !reach(w, id, h) { return; }
            if e != 1 { return; }
            set_state(w, id, 4);
        }
        4 => {
            if 3.0 <= d(w) && reach(w, id, h) && e != 2 {
                if d(w) <= 8.0 { return; }
                set_state(w, id, 5);
                return;
            }
            set_state(w, id, 3);
        }
        5 => {
            if 3.0 <= d(w) && reach(w, id, h) && e != 2 {
                if 4.0 <= d(w) { return; }
                set_state(w, id, 4);
                return;
            }
            set_state(w, id, 3);
        }
        6 => {
            let ph = phase(w, id);
            let ci = c::pi32(w, id, pv::POINTS_E0 + 4 * ph);
            let cp = cuboid(w, ci).map_or([0.0; 4], |(p, _)| [p[0], p[1], p[2], 0.0]);
            reach(w, id, cp);
            if 1.0 <= c::dist2(c::pos(w, id), c::pv4(w, id, pv::TARGET)) { return; }
            match ph {
                0 | 1 => {
                    let Some(b) = link(w, id, pv::BOATS + 4 * ph) else { return };
                    let side = if ph == 0 { k::BOARD[1] } else { -k::BOARD[1] };
                    let m = w.m(b);
                    let unit = |r: [f32; 4], l: f32| {
                        let n = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
                        if n == 0.0 { [0.0; 3] } else { [r[0] * l / n, r[1] * l / n, r[2] * l / n] }
                    };
                    let (a, bb, cc) = (unit(m.rows[0], k::BOARD[0]), unit(m.rows[1], side), unit(m.rows[2], k::BOARD[2]));
                    let p = m.position;
                    let t = [p[0] + a[0] + bb[0] + cc[0], p[1] + a[1] + bb[1] + cc[1], p[2] + a[2] + bb[2] + cc[2], p[3]];
                    c::set_pv4(w, id, pv::TARGET, t);
                    if ph == 1 {
                        let ca = c::pi32(w, id, pv::CUBOID_A);
                        shift_cuboid(w, ca, 500.0);
                    }
                    c::set_pi32(w, id, pv::AFTER_JUMP, 8);
                    set_state(w, id, 7);
                }
                2 => set_state(w, id, 0xb),
                _ => {
                    let cb = c::pi32(w, id, pv::CUBOID_B);
                    shift_cuboid(w, cb, 500.0);
                    set_state(w, id, 0xc);
                }
            }
        }
        7 => {
            if w.m(id).anim.flags & 2 != 0 {
                let s = c::pi32(w, id, pv::AFTER_JUMP) as u8;
                set_state(w, id, s);
            }
        }
        8 => {
            if c::dist2(c::pos(w, id), c::pv4(w, id, pv::TARGET)) < 0.5 { set_state(w, id, 9); }
        }
        9 => {
            if phase_boat(w, id).is_some_and(|b| boat::arrived(w, b)) { set_state(w, id, 10); }
        }
        10 => {
            if 0.5 <= c::dist2(c::pos(w, id), c::pv4(w, id, pv::TARGET)) { return; }
            let ph = c::pi32(w, id, pv::PHASE) + 1;
            c::set_pi32(w, id, pv::PHASE, ph);
            let ci = c::pi32(w, id, pv::POINTS_D0 + 4 * (ph & 7) as usize);
            // The cuboid's +0x30..+0x3f (its centre row, w included).
            if let Some(s) = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, ci) {
                let r = s.matrix[3];
                c::set_pv4(w, id, pv::TARGET, r);
            }
            c::set_pi32(w, id, pv::AFTER_JUMP, 4);
            set_state(w, id, 7);
        }
        0xb => {
            if link(w, id, pv::MACHINE).is_some_and(|m| w.m(m).state == 5) {
                let ph = c::pi32(w, id, pv::PHASE) + 1;
                c::set_pi32(w, id, pv::PHASE, ph);
                set_state(w, id, 3);
            }
        }
        0xc => {
            c::set_pf(w, id, pv::TALK + interact::talk::RADIUS, 255.0);
            if !interact::talk_update_at(w, id, pv::TALK) { return; }
            let mb = c::pi32(w, id, pv::MISSION_B);
            if let Ok(m) = u8::try_from(mb) { crate::cinematic::set_mission_done(w, m); }
            interact::set_global_flag(w, FLAG_SCENE, 1);
            interact::set_talked(w, id, 2);
            let ci = c::pi32(w, id, pv::POINTS_E0 + 4 * phase(w, id));
            to_cuboid(w, id, ci);
            // The end place 2 ahead along row 0 of the new Euler, facing back (0x16d270 / 0x16d280 / 0x16d2a6); the
            // `0x288e98(2.5, m)` right after replaces it.
            let e = w.m(id).rotation;
            let f = [e[2].cos() * e[1].cos(), e[2].sin() * e[1].cos()];
            let p = c::pos(w, id);
            w.svc.interact.scene_end_place = Some(([p[0] + f[0] * 2.0, p[1] + f[1] * 2.0, p[2] - e[1].sin() * 2.0], c::add_rot(e[2], std::f32::consts::PI)));
            interact::place_after_scene(w, id, 2.5);
            set_state(w, id, 0xd);
        }
        0xd => {
            if w.svc.game_mode != 2 {
                let g = link(w, id, pv::GATE);
                open_gate(w, g);
                set_state(w, id, 0xe);
            }
        }
        0xe => {
            if mission_done(w, c::pi32(w, id, pv::MISSION_C)) && w.hero.state != 0x32 && interact::talk_update_at(w, id, pv::TALK) {
                interact::place_after_scene(w, id, 2.5);
                set_state(w, id, 0xf);
            }
        }
        0xf if w.svc.game_mode != 2 => {
            interact::give_item(w, REWARD_ITEM, true);
            crate::cinematic::save(w);
            w.svc.help.request(HELP_MSG, HELP_REC);
            w.delete_moby(id);
        }
        _ => {}
    }
}

/// `0x2d27b0` (module doc).
fn look(w: &mut World, id: MobyId) {
    if matches!(state(w, id), 0..=5 | 0xb | 0xc | 0xe | 0x10) {
        let mut e = c::pos(w, id);
        e[2] += 1.0;
        let d = c::sub(hero(w), e);
        let yaw = c::sub_rot(c::atan(d[0], d[1]), c::yaw(w, id)).clamp(-1.221_730_5, 1.221_730_5);
        let pitch = -c::atan(c::len2(d), d[2]);
        let pitch = pitch.clamp(-std::f32::consts::FRAC_PI_6, 0.261_799_4);
        let t = crate::moby_update::manip::rec::TARGET;
        c::set_pf(w, id, pv::HEAD + t + 4, pitch);
        c::set_pf(w, id, pv::HEAD + t + 8, yaw * 0.7);
        c::set_pf(w, id, pv::TORSO + t + 8, yaw * 0.3);
    }
    // The actors' big-head cheat 0x15edb0: the head record's scale request (+0x70 = P+0x1d0) = 2.75.
    if w.svc.cheats.on(crate::cheats::slot::ACTORS) { c::set_pf(w, id, pv::HEAD + crate::moby_update::manip::rec::REC_SCALE, f32::from_bits(0x4030_0000)); }
    crate::moby_update::manip::look(w, id, id, pv::HEAD, 0, 0.02, 0.3);
    crate::moby_update::manip::look(w, id, id, pv::TORSO, 1, 0.02, 0.3);
}

/// `0x2d0fa8`.
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { return; }
    // 0x2d0ec0: the actors' big-head cheat on this class's scene actor at 2.75.
    let own = w.m(id).o_class;
    crate::moby_update::manip::scene_big_head(w, &[own], 0, f32::from_bits(0x4030_0000));
    interact::poll_scene_end_at(w, id, pv::TALK);
    if w.m(id).visible != 0 {
        let cam = w.camera.map(|x| f32::from_bits(x.0));
        if c::dist3(c::pos(w, id), cam) < 38.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0x1e;
        }
    }
    motion(w, id);
    moving(w, id);
    brain(w, id);
    if w.m(id).state >= 0x80 { return; }
    look(w, id);
}

/// `0x2cb810`: the gate 65 (module doc).
pub fn gate_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0xc { return; }
    match w.m(id).state {
        0 => {
            w.mm(id).state = 1;
            let yaw = c::yaw(w, id);
            c::set_pf(w, id, 4, yaw);
            if w.m(id).mode & mode::MIRROR != 0 { return; }
            let Some(t) = w.create_moby(GATE) else { return };
            let (dd, ud, md, p, r) = { let m = w.m(id); (m.draw_dist, m.update_dist, m.mode, m.position, m.rotation) };
            {
                let m = w.mm(t);
                m.draw_dist = dd;
                m.update_dist = ud;
                m.mode = md | mode::MIRROR;
                m.position = p;
                m.rotation = r;
            }
            c::set_pi32(w, id, 8, t as i32 + 1);
            if w.m(t).pvars.len() >= 8 { c::set_pf(w, t, 4, yaw); }
        }
        2 => {
            let base = c::pf(w, id, 4);
            let open = f32::from_bits(k::GATE_OPEN_BITS);
            let to = if w.m(id).mode & mode::MIRROR == 0 { c::add_rot(base, open) } else { c::sub_rot(base, open) };
            let deg = 0.017_453_292_f32;
            let r = turn::turn_toward_pvar(w, id, to, c::DT2 * 20.0 * deg, c::DT2 * 20.0 * deg, c::DT * 45.0 * deg, 0);
            if r == 0.0 { w.mm(id).state = 3; }
        }
        _ => {}
    }
}
