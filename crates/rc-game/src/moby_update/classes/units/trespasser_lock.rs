//! The Trespasser locks, class 615 (census U96; level02 `0x2d8ad0`, the same code on levels 04 `0x2d4ae0`, 06
//! `0x2e1940`, 08 `0x2f2030`, 11 `0x2fcda8`, 13 `0x2f5618`, 18 `0x2d8fd0`: 19 created instances), and their minigame,
//! the draw callback `0x2d93e8` (which also decides which outputs are powered). The gadget is
//! `crate::hero::trespasser` (it only tells the lock that ○ is held). Read from the level02 decompiler output and the
//! disassembly of `0x2d8ad0`, `0x2da6c0`, `0x2d93e8`; the constants from the level02 data (gp = 0x166c00: gp−0x5160 ..
//! gp−0x5034, the node corners 0x1d33b0, the background quads 0x1d3310).
//!
//! **The puzzle.** Three concentric rings of 12 slots (+0x20 outer, +0x50 middle, +0x80 inner; each turned by its
//! offset +0x14 / +0x18 / +0x1c) and 12 outputs (+0xb0) around them. A slot holds 0 (empty), 1 (a block) or 2 (an
//! emitter). Every emitter fires a beam inward through the centre: blocked by any non-empty slot of the rings inside it,
//! then of the opposite side's rings (inner to outer), the beam stops there; a clear beam reaches the opposite output
//! and powers it (2). The lock opens when no output is left unpowered (1). ↑ / ↓ pick the ring (sound 1), ← / → turn it
//! one slot (30°, at 2π·dt a tick; sound 0); while it turns, the beams already use the next slot.
//!
//! **Pvars** (P): +0x00 the hit flash record (`creature::flash`), +0x10 (s16) the timer, +0x12 (s16) Ratchet on it,
//! +0x14..+0x1c the rings' offsets, +0x20 / +0x50 / +0x80 the rings, +0xb0 the outputs, +0xe0 the chosen ring (1..3),
//! +0xe4 the turn angle, +0xe8 the camera cuboid (−1: computed), +0xec the background texture (FX + 0x28 / + 0x29),
//! +0xf0 / +0x100 the camera's start (position / Euler), +0x110 / +0x114 the camera spring's `t` and velocity, +0x11c /
//! +0x11e (s16) input flags (+0x11e: bit 0 turned, bit 1 picked).
//!
//! **Coverage** (`address | what | status`):
//!
//! | address | what | status |
//! |---|---|---|
//! | head | no pvars → return | ported ([`update`]) |
//! | state 0 | +0x12 = 0; state 1; the node corner table (±17); gp−0x50e4 = +0xec (the background, a level global the draw reads); mission byte +0xb0 ≠ 0xff and (the collected byte `0x1bbc84[spawn id]` or the persistent death bit `0x14c190[level]`) → state 4 | ported (the global: `units::Globals` word 0x161b1c) |
//! | state 1 | Ratchet on it (ground moby 0x13f64c = m, air ticks 0): the first tick `PlayClassSound(3, 0, m)` (+0x12 = 1); +0x90 = 0x8040c040 with the Trespasser (item 0x1a) in hand, else 0x804040c0; Trespasser and its +0xbc (○ held): +0x10 = `ticks(300)`, +0x11c = +0x11e = 0, `SetState(0x72, 0)`, `SetAnim(ticks(6), 0x57, 0)`, `CameraScript(camera, its Euler, 1, 0, 0)`, state 2, +0x10 = `ticks(60)`, +0x110 = 0, +0xf0 / +0x100 = the camera's position / Euler; off it: +0x12 = 0, +0x90 = 0x80808080 | ported |
//! | state 2 `0x2d92f0` | Ratchet pulled to joint list 0's point (≤ 4·dt a tick) and turned to the lock's yaw + π (≤ 2π·dt) | ported ([`pull`], `HeroFields::pose`) |
//! | state 2 `0x2d8910` | the camera: cuboid +0xe8, or (−1) between Ratchet and the lock, 2 out on the side (yaw ± 90°) nearer the camera's bearing, 1 up, facing back (side + π) | ported ([`camera_goal`]) |
//! | state 2 | `Spring(1, 2·dt², 2·dt², 2·dt, &t, &v)` (0x25df98); position = lerp(t, start, goal), Euler z / y = lerp_rot(t, start, goal); `0x2f8958` / `0x2f89b0`; 0x17eb08 = 1 (no HUD); `force_help_message(6, 0)`; the camera not the script camera (type 5) → `CameraScript(camera, Euler, 1, 0, 0)` | ported (`cinematic::camera_targets` / `camera_script_unless_script`, the HUD-off frame through `Services::visibomb.hud_off_at` (the same global 0x17e988 / 0x17eb08)) |
//! | state 2 | not arrived (t < 0.99 or the camera's yaw > 1° off): Ratchet's sequence wrapped → `SetAnim(ticks(6), 0x58, 0)` | ported |
//! | state 2 arrived | `RegisterDrawCallback(0x2d93e8, m)`; s1 = 0 when output 0 is unpowered, else the first unpowered of 1..11, 12 when none | ported (`Callback::TrespasserRings`) |
//! | | turning (+0xe4 > 0): += 2π·dt; past 30°: 0, the chosen ring's offset += 11 (mod 12); (< 0): −= 2π·dt, past −30°: offset += 13 | ported |
//! | | solved (s1 = 12): `FastDecTimer_s16(+0x10)` out → state 3 | ported |
//! | | ← / → (0x13cb04 & 0xa000): +0x11e \|= 1, `PlayClassSound(0, 0, m)`, +0xe4 = −2π·dt (←) / 2π·dt (→); else ↑ (0x13cae4 & 0x1000): ring − 1 (at 0 → 1, no sound), \|= 2; ↓ (& 0x4000): ring + 1 (at 4 → 3, no sound); a change → `PlayClassSound(1, 0, m)` | ported |
//! | | not solved and (△ with the help box idle (0x179a10 = 0, 0x179a34 = −1), or Ratchet not in 0x72, or the hand item not 0x1a): state 1, `CameraScript2(1)`, `0x22ea90` (body 0 → `SetState(0, 1)`, body 3 → `SetState(0x53, 1)`), +0x10 = `ticks(60)` | ported |
//! | state 3 | Ratchet's sequence wrapped: mission ≠ 0xff → the death bits (persistent and this visit); `CameraScript2(1)`; `SetState(0, 1)`; +0xbc = 1; state 4 (the doors read state 4: 743 / 744 (L02), 1101 / 1102 / 1531 / 1532 (L04), 1343 (L04), 467 / 472 (L08), 1159 (L11), 1392 (L18)) | ported |
//! | every state | `0x25fb60(m, P)`: the hit flash on its ambient | ported (`creature::flash::update`) |
//! | `0x2d93e8` | the solve (module doc), the background `0x2dae88` (four quads ±192 px, FX +0xec + 0x28, colour 0x60808080, ST (0, 0) / (0, 128) / (128, 0) / (128, 128)), the beams `0x2da938` (FX 7, scrolling, ±2 px and ±5 px; orange 0x2000c0ff, green 0x2000ff40 when the output lights, the blocked beam's return in red 0x200000ff), the ring bands `0x2da3d8` (FX 0x10; the chosen ring pulsing 0x6000d0ff, the others 0), the nodes `0x2da6c0` (34 px squares turned with the ring, FX +0xec + 0x29, cells by kind: emitter lit 0, emitter 1, block 2, colour 0x60808080, drawn twice), the outputs (orange 0x80004080 / red 0x80000080 every 32 frames; powered green 0x80008000) | ported ([`frame`], [`RingPrim`]: drawn by `rc-engine` scene_render; GS 2048 = the screen centre (256, 208); the VU1 TEST register writes are n/a) |
//! | sounds, particles, lights, stats, bolts | class sounds 0 / 1 / 3; no particle, light, stat or bolt | — |
//!
//! **Native.** Plain `f32` (the draw keeps the game's truncation of the 1/16-pixel coordinates).

use crate::cinematic;
use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, add_rot, atan, diff_rots, sub_rot};
use crate::moby_update::services::{pvar as p, HeroCall, HeroPose, World};
use crate::pad::button;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub const UPDATE_FN: u32 = 0x2d_8ad0;
pub const REFERENCE_LEVEL: u32 = 2;
pub const CLASSES: [i16; 1] = [615];
/// The minigame's draw callback (level02).
pub const DRAW_FN: u32 = 0x2d_93e8;
/// The Trespasser's item id.
pub const TRESPASSER: i32 = 0x1a;
/// The level global the draw reads the background from (gp−0x50e4 on level02).
pub const BACKGROUND_WORD: u32 = 0x161b1c;

/// Pvar offsets.
pub mod pv {
    pub const FLASH: usize = 0x00;
    pub const TIMER: usize = 0x10;
    pub const ON: usize = 0x12;
    pub const ROT: usize = 0x14;
    pub const RINGS: usize = 0x20;
    pub const OUT: usize = 0xb0;
    pub const ACTIVE: usize = 0xe0;
    pub const SPIN: usize = 0xe4;
    pub const CAMERA: usize = 0xe8;
    pub const BACKGROUND: usize = 0xec;
    pub const CAM_POS: usize = 0xf0;
    pub const CAM_EULER: usize = 0x100;
    pub const T: usize = 0x110;
    pub const TV: usize = 0x114;
    pub const IN_A: usize = 0x11c;
    pub const IN_B: usize = 0x11e;
    pub const LEN: usize = 0x120;
}

/// The glow words (gp−0x5160 / −0x515c / −0x5158).
pub const GLOW_READY: u32 = 0x8040_c040;
pub const GLOW_NO_ITEM: u32 = 0x8040_40c0;
pub const GLOW_OFF: u32 = 0x8080_8080;
/// Ratchet's sequences at the lock: the reach, the hold.
pub const SEQ_REACH: u8 = 0x57;
pub const SEQ_HOLD: u8 = 0x58;

fn wrapped(w: &World) -> bool { w.hero.loop_in.anim.flags & 2 != 0 }
fn ring_word(w: &World, id: MobyId, base: usize, k: i32) -> i32 {
    // `base + (k % 12)·4` with C's remainder (a negative index reads the word before the ring, as the game does).
    let o = base as i64 + 4 * (k % 12) as i64;
    usize::try_from(o).ok().filter(|&o| o + 4 <= w.m(id).pvars.len()).map_or(0, |o| p::i32(&w.m(id).pvars, o))
}
fn set_ring_word(w: &mut World, id: MobyId, base: usize, k: i32, v: i32) {
    let o = base as i64 + 4 * (k % 12) as i64;
    if let Some(o) = usize::try_from(o).ok().filter(|&o| o + 4 <= w.m(id).pvars.len()) { p::set_i32(&mut w.mm(id).pvars, o, v); }
}

/// The lock's update (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::LEN { return; }
    match w.m(id).state {
        0 => {
            c::set_pi16(w, id, pv::ON, 0);
            w.mm(id).state = 1;
            let bg = c::pi32(w, id, pv::BACKGROUND);
            w.svc.units.set_word(BACKGROUND_WORD, bg as u32);
            if w.m(id).mission != 0xff {
                let (level, sid) = (w.svc.level, w.m(id).spawn_id);
                let done = w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(level, sid));
                if done { w.mm(id).state = 4; }
            }
        }
        1 => standing(w, id),
        2 => minigame(w, id),
        3 if wrapped(w) => {
            if w.m(id).mission != 0xff { crate::moby_update::story::death_bits(w, id); }
            cinematic::camera_script2(w, 1);
            cinematic::hero_state(w, 0, true);
            w.mm(id).cmd = 1;
            w.mm(id).state = 4;
        }
        _ => {}
    }
    crate::moby_update::creature::flash::update(w, id, pv::FLASH);
}

/// State 1: Ratchet on the lock, the Trespasser held out with ○.
fn standing(w: &mut World, id: MobyId) {
    if !(w.hero.ground_moby == Some(id) && w.hero.air_ticks == 0) {
        c::set_pi16(w, id, pv::ON, 0);
        w.mm(id).glow = GLOW_OFF;
        return;
    }
    if c::pi16(w, id, pv::ON) == 0 {
        c::set_pi16(w, id, pv::ON, 1);
        w.play_sound(3, 0, id);
    }
    let slot = &w.hero.items.slot;
    let ready = slot.item.is_some() && slot.id == TRESPASSER;
    w.mm(id).glow = if ready { GLOW_READY } else { GLOW_NO_ITEM };
    if !ready || w.hero.gadgets.item_bc == 0 { return; }
    let t300 = w.ticks(300);
    c::set_pi16(w, id, pv::TIMER, t300 as i16);
    c::set_pi16(w, id, pv::IN_A, 0);
    c::set_pi16(w, id, pv::IN_B, 0);
    cinematic::hero_state(w, 0x72, false);
    let b = w.ticks(6) as f32;
    w.hero_fields_mut().call(HeroCall::SetAnim { blend: b, seq: SEQ_REACH, frame: 0 });
    let (cam, euler) = camera_now(w);
    cinematic::camera_script(w, cam, euler, 1, 0, false);
    w.mm(id).state = 2;
    let t60 = w.ticks(0x3c);
    c::set_pi16(w, id, pv::TIMER, t60 as i16);
    c::set_pf(w, id, pv::T, 0.0);
    c::set_pv4(w, id, pv::CAM_POS, [cam[0], cam[1], cam[2], w.camera[3].to_f32()]);
    c::set_pv4(w, id, pv::CAM_EULER, [euler[0], euler[1], euler[2], 0.0]);
}

/// The camera's position 0x1673c0 and Euler 0x1673d0 as the last camera update left them.
fn camera_now(w: &World) -> ([f32; 3], [f32; 3]) {
    ([w.camera[0].to_f32(), w.camera[1].to_f32(), w.camera[2].to_f32()], w.hero.loop_in.cam_euler)
}

/// `0x2d92f0`: Ratchet drawn to the lock's joint list 0 and turned to face away from it.
fn pull(w: &mut World, id: MobyId) {
    let dt = crate::moby_update::services::DT.to_f32();
    let j = w.joint_point(id, 0);
    let h = w.hero.position();
    let mut v = [j[0] - h[0], j[1] - h[1], j[2] - h[2]];
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let max = dt * 4.0;
    if max < l { v = v.map(|x| x * (max / l)); }
    let pos = [h[0] + v[0], h[1] + v[1], h[2] + v[2]];
    let yaw = w.hero.rot[2].to_f32();
    let target = add_rot(c::yaw(w, id), f32::from_bits(0x4049_0fd0));
    let lim = dt * TAU;
    let d = sub_rot(target, yaw).clamp(-lim, lim);
    let ty = w.hero.target_yaw.to_f32();
    w.hero_fields_mut().pose = Some(HeroPose { pos, yaw: add_rot(d, yaw), target_yaw: ty });
}

/// `0x2d8910`: the camera's goal (position, Euler).
fn camera_goal(w: &World, id: MobyId) -> ([f32; 3], [f32; 3]) {
    let cub = c::pi32(w, id, pv::CAMERA);
    if cub != -1 {
        return w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, cub).map_or(([0.0; 3], [0.0; 3]), |s| (s.centre(), s.euler));
    }
    let m = w.m(id);
    let yaw = m.rotation[2];
    let (a, b) = (add_rot(yaw, FRAC_PI_2), add_rot(yaw, -FRAC_PI_2));
    let cam = w.camera;
    let bearing = atan(cam[0].to_f32() - m.position[0], cam[1].to_f32() - m.position[1]);
    let side = if diff_rots(bearing, a) < diff_rots(bearing, b) { a } else { b };
    let h = w.hero.position();
    let mut pos = [(h[0] + m.position[0]) * 0.5, (h[1] + m.position[1]) * 0.5, (h[2] + m.position[2]) * 0.5];
    pos[0] += side.cos() + side.cos();
    pos[1] += side.sin() + side.sin();
    pos[2] += 1.0;
    (pos, [0.0, 0.0, add_rot(side, PI)])
}

/// State 2: the camera's move, then the minigame's input once it has arrived.
fn minigame(w: &mut World, id: MobyId) {
    pull(w, id);
    let (goal, mut euler) = camera_goal(w, id);
    let dt = crate::moby_update::services::DT.to_f32();
    let (mut t, mut tv) = (c::pf(w, id, pv::T), c::pf(w, id, pv::TV));
    crate::moby_update::creature::turn::spring(1.0, dt * dt * 2.0, dt * dt * 2.0, dt + dt, &mut t, &mut tv);
    c::set_pf(w, id, pv::T, t);
    c::set_pf(w, id, pv::TV, tv);
    let start = c::pv4(w, id, pv::CAM_POS);
    let pos: [f32; 3] = std::array::from_fn(|k| start[k] + (goal[k] - start[k]) * t);
    let se = c::pv4(w, id, pv::CAM_EULER);
    euler[2] = add_rot(sub_rot(euler[2], se[2]) * t, se[2]);
    euler[1] = add_rot(sub_rot(euler[1], se[1]) * t, se[1]);
    cinematic::camera_targets(w, Some(pos), Some(euler));
    // 0x17eb08 = 1: `HudDraw` skips the HUD this frame (the same global as the Visibomb's 0x17e988).
    w.svc.visibomb.hud_off_at = Some(w.counter);
    w.svc.interact.force_prompt(6, 0);
    let (cam, ce) = camera_now(w);
    cinematic::camera_script_unless_script(w, cam, ce, 1, 0, false);
    let mut solved = 0;
    let arrived = 0.99 <= t && diff_rots(w.hero.loop_in.cam_euler[2], euler[2]) < f32::from_bits(0x3c8e_fa35);
    if arrived {
        w.svc.draw_callbacks.register(crate::moby_update::classes::draw_callbacks::Callback::TrespasserRings, id);
        solved = (0..12).find(|&k| c::pi32(w, id, pv::OUT + 4 * k) == 1).unwrap_or(12) as i32;
        let spin = c::pf(w, id, pv::SPIN);
        let step = dt * TAU;
        let ring = c::pi32(w, id, pv::ACTIVE);
        let rot_of = |r: i32| pv::ROT + 4 * (r.clamp(1, 3) as usize - 1);
        if 0.0 < spin {
            let a = add_rot(spin, step);
            c::set_pf(w, id, pv::SPIN, a);
            if f32::from_bits(0x3f06_0a92) < a {
                c::set_pf(w, id, pv::SPIN, 0.0);
                let o = rot_of(ring);
                let v = (c::pi32(w, id, o) + 0xb) % 0xc;
                c::set_pi32(w, id, o, v);
            }
        } else if spin < 0.0 {
            let a = add_rot(spin, -step);
            c::set_pf(w, id, pv::SPIN, a);
            if a < f32::from_bits(0xbf06_0a92) {
                c::set_pf(w, id, pv::SPIN, 0.0);
                let o = rot_of(ring);
                let v = (c::pi32(w, id, o) + 0xd) % 0xc;
                c::set_pi32(w, id, o, v);
            }
        } else if solved == 12 {
            if crate::moby_update::creature::dec_timer_pvar_s16(w, id, pv::TIMER) != 0 { w.mm(id).state = 3; }
        } else if w.hero.loop_in.pad.pressed_unmirrored & (button::LEFT | button::RIGHT) != 0 {
            let f = c::pi16(w, id, pv::IN_B) | 1;
            c::set_pi16(w, id, pv::IN_B, f);
            w.play_sound(0, 0, id);
            let left = w.hero.loop_in.pad.pressed_unmirrored & button::LEFT != 0;
            c::set_pf(w, id, pv::SPIN, if left { -step } else { step });
        } else {
            let pressed = w.hero.loop_in.pad.pressed;
            let pick = if pressed & button::UP != 0 { Some((-1, 0, 1)) } else if pressed & button::DOWN != 0 { Some((1, 4, 3)) } else { None };
            if let Some((d, edge, back)) = pick {
                let f = c::pi16(w, id, pv::IN_B) | 2;
                c::set_pi16(w, id, pv::IN_B, f);
                let r = ring + d;
                if r == edge {
                    c::set_pi32(w, id, pv::ACTIVE, back);
                } else {
                    c::set_pi32(w, id, pv::ACTIVE, r);
                    w.play_sound(1, 0, id);
                }
            }
        }
    } else if wrapped(w) {
        let b = w.ticks(6) as f32;
        w.hero_fields_mut().call(HeroCall::SetAnim { blend: b, seq: SEQ_HOLD, frame: 0 });
    }
    if solved == 12 { return; }
    let triangle = w.hero.loop_in.pad.pressed & button::TRIANGLE != 0 && w.svc.help.idle();
    let slot = &w.hero.items.slot;
    if triangle || w.hero.state != 0x72 || slot.id != TRESPASSER {
        w.mm(id).state = 1;
        cinematic::camera_script2(w, 1);
        // `0x22ea90`: body 0 → `SetState(0, 1)`, body 3 → `SetState(0x53, 1)`.
        match w.body() {
            0 => cinematic::hero_state(w, 0, true),
            3 => cinematic::hero_state(w, 0x53, true),
            _ => {}
        }
        let t60 = w.ticks(0x3c);
        c::set_pi16(w, id, pv::TIMER, t60 as i16);
    }
}

// ------------------------------------------------------------------------------------------------
// The draw callback 0x2d93e8.

/// One 2-D primitive of the minigame: four corners in screen pixels (strip order), texel ST, the FX texture, one
/// colour; `repeat`: the GS CLAMP_1 = 0 (REPEAT) the beams use (the others: 5, clamp).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingPrim {
    pub fx: usize,
    pub pos: [[f32; 2]; 4],
    pub uv: [[f32; 2]; 4],
    pub rgba: u32,
    pub repeat: bool,
}

/// The screen centre (GS 2048, 2048 with XYOFFSET (2048 − 256, 2048 − 208)).
pub const CENTRE: [f32; 2] = [256.0, 208.0];
/// Radii (gp−0x50d0 ..): ring bands (outer, inner) and node radii, outputs, the far beam.
pub const BANDS: [[f32; 2]; 3] = [[138.0, 132.0], [100.0, 95.0], [61.0, 56.0]];
pub const NODE_R: [f32; 3] = [135.0, 96.0, 59.0];
pub const OUT_BAND: [f32; 2] = [185.0, 180.0];
pub const FAR: f32 = 160.0;
/// The colours (gp−0x5068 ..).
pub const RGBA_BACKGROUND: u32 = 0x6080_8080;
pub const RGBA_NODE: u32 = 0x6080_8080;
pub const RGBA_RING_ACTIVE: u32 = 0x6000_d0ff;
pub const RGBA_RING: u32 = 0;
pub const RGBA_BEAM: u32 = 0x2000_c0ff;
pub const RGBA_BEAM_LIT: u32 = 0x2000_ff40;
pub const RGBA_BEAM_BACK: u32 = 0x2000_00ff;
pub const RGBA_OUT_B: u32 = 0x8000_0080;
pub const RGBA_OUT_LIT: u32 = 0x8000_4080;
pub const RGBA_OUT_POWERED: u32 = 0x8000_8000;
/// The FX textures: the beams (gp−0x509c = 7), the bands (0x10); the nodes and the background are +0xec + 0x29 / 0x28.
pub const FX_BEAM: usize = 7;
pub const FX_BAND: usize = 0x10;

fn deg(d: i32) -> f32 { d as f32 * 0.017_453_292 }
/// A point at `angle` (0 up, clockwise) and radius `r` from the centre, truncated to 1/16 pixel.
fn at(angle: f32, r: f32) -> [f32; 2] {
    let x = ((angle.sin() * r * 16.0) as i32) as f32 / 16.0;
    let y = ((angle.cos() * r * 16.0) as i32) as f32 / 16.0;
    [CENTRE[0] + x, CENTRE[1] - y]
}

/// `0x2da3d8(a0, a1, r_out, r_in, tex, rgba)`: five quads from a0 to a1.
fn band(out: &mut Vec<RingPrim>, a0: f32, a1: f32, ro: f32, ri: f32, rgba: u32) {
    let span = sub_rot(a1, a0);
    for k in 0..5 {
        let a = add_rot(a0, (span / 5.0) * k as f32);
        let b = add_rot(a0, (span / 5.0) * (k + 1) as f32);
        out.push(RingPrim { fx: FX_BAND, pos: [at(a, ro), at(a, ri), at(b, ro), at(b, ri)], uv: [[32.0, 16.0], [0.0, 16.0], [32.0, 16.0], [0.0, 16.0]], rgba, repeat: false });
    }
}

/// `0x2da938(a, r0, r1, tex, rgba)`: the beam (±2 px, its ST scrolling with the frame counter) and its glow (±5 px).
fn beam(out: &mut Vec<RingPrim>, a: f32, r0: f32, r1: f32, rgba: u32, counter: u64) {
    let (s, c) = (a.sin(), a.cos());
    let corner = |r: f32, w: f32, sign: f32| -> [f32; 2] {
        let x = ((s * r + sign * w * c) as i32) as f32;
        let y = ((c * r - sign * w * s) as i32) as f32;
        [CENTRE[0] + x, CENTRE[1] - y]
    };
    let scroll = (((counter as f32 * 16.0) as i32).rem_euclid(0x200)) as f32 / 16.0;
    let pos = |w: f32| [corner(r0, w, 1.0), corner(r0, w, -1.0), corner(r1, w, 1.0), corner(r1, w, -1.0)];
    out.push(RingPrim { fx: FX_BEAM, pos: pos(2.0), uv: [[32.0, 32.0 + scroll], [0.0, 32.0 + scroll], [32.0, scroll], [0.0, scroll]], rgba, repeat: true });
    out.push(RingPrim { fx: FX_BEAM, pos: pos(5.0), uv: [[32.0, 16.0], [0.0, 16.0], [32.0, 16.0], [0.0, 16.0]], rgba, repeat: true });
}

/// `0x2da6c0(a, r, tex, rgba, kind)`: a 34-pixel square turned with the ring, drawn twice (colour 0x60808080).
fn node(out: &mut Vec<RingPrim>, a: f32, r: f32, kind: i32, bg: usize) {
    let c0 = at(a, r);
    // The corners (±17, ±17) of 0x1d33b0 turned by the Euler (0, 0, −a) (`fun_001fa050` / `fun_001fa378`); the result's
    // x goes to the GS y (down) and its y to the GS x [L: the rotation matrix's row convention].
    let rot = |x: f32, y: f32| {
        let (ox, oy) = (x * a.cos() + y * a.sin(), -x * a.sin() + y * a.cos());
        [c0[0] + oy, c0[1] + ox]
    };
    let pos = [rot(17.0, 17.0), rot(17.0, -17.0), rot(-17.0, 17.0), rot(-17.0, -17.0)];
    let uv = match kind {
        0 => [[32.0, 0.0], [0.0, 0.0], [32.0, 32.0], [0.0, 32.0]],
        1 => [[64.0, 0.0], [32.0, 0.0], [64.0, 32.0], [32.0, 32.0]],
        _ => [[64.0, 32.0], [0.0, 32.0], [64.0, 64.0], [0.0, 64.0]],
    };
    let p = RingPrim { fx: bg + 0x29, pos, uv, rgba: RGBA_NODE, repeat: false };
    out.push(p);
    out.push(p);
}

/// `0x2dae88`: the background, four quarter quads ±192 px around the centre.
fn background(out: &mut Vec<RingPrim>, bg: usize) {
    let o = |x: f32, y: f32| [CENTRE[0] + x, CENTRE[1] + y];
    let quads = [(192.0, 192.0), (-192.0, -192.0), (192.0, -192.0), (-192.0, 192.0)];
    for (x, y) in quads {
        out.push(RingPrim { fx: bg + 0x28, pos: [o(x, y), o(x, 0.0), o(0.0, y), o(0.0, 0.0)], uv: [[0.0, 0.0], [0.0, 128.0], [128.0, 0.0], [128.0, 128.0]], rgba: RGBA_BACKGROUND, repeat: false });
    }
}

/// `0x2d93e8`, run by the frame's callbacks (module doc): the outputs and the lit emitters decided, the primitives laid
/// out into `DrawCallbacks::rings`.
pub fn frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::LEN { return; }
    let mut out = Vec::new();
    let bg = w.svc.units.word(BACKGROUND_WORD) as usize;
    background(&mut out, bg);
    for k in 0..12 {
        if c::pi32(w, id, pv::OUT + 4 * k) != 0 { c::set_pi32(w, id, pv::OUT + 4 * k, 1); }
    }
    let spin = c::pf(w, id, pv::SPIN);
    let active = c::pi32(w, id, pv::ACTIVE);
    // The ring turning shows (and fires) one slot on past 18°.
    let mut off = [0i32; 3];
    let tenth = std::f32::consts::PI / 10.0;
    let dir = if tenth < spin { 1 } else if spin < -tenth { -1 } else { 0 };
    if dir != 0 { off[(active.clamp(1, 3) - 1) as usize] = dir; }
    let rot = [c::pi32(w, id, pv::ROT), c::pi32(w, id, pv::ROT + 4), c::pi32(w, id, pv::ROT + 8)];
    let base = |r: usize| pv::RINGS + 0x30 * r;
    let counter = w.counter;
    // The beams of rings A (outer), B and C (inner).
    for r in 0..3 {
        for i in 0..12i32 {
            let mut a = f32::from_bits(0xbea0_d97c);
            if active == r as i32 + 1 { a = add_rot(a, spin); }
            a = add_rot(a, deg(i * 30));
            let own = i + rot[r];
            if ring_word(w, id, base(r), own) != 2 { continue; }
            // The rings inside this one at the same angle (with the turning ring's offset relative to this one), then
            // the opposite side from the inside out.
            let idx = |q: usize| i + rot[q] + off[r] - off[q];
            let mut reach = None;
            for (q, &node_r) in NODE_R.iter().enumerate().skip(r + 1) {
                if ring_word(w, id, base(q), idx(q)) != 0 {
                    reach = Some(node_r);
                    break;
                }
            }
            let mut through = false;
            let r_to = match reach {
                Some(x) => x,
                None => {
                    let mut stop = None;
                    for q in (0..3).rev() {
                        let k = if q == r { own } else { idx(q) };
                        if ring_word(w, id, base(q), k + 6) != 0 {
                            stop = Some(NODE_R[q]);
                            break;
                        }
                    }
                    match stop {
                        Some(x) => -x,
                        None => {
                            through = true;
                            -FAR
                        }
                    }
                }
            };
            let mut rgba = RGBA_BEAM;
            if through {
                let o = i + off[r] + 6;
                if ring_word(w, id, pv::OUT, o) != 0 {
                    set_ring_word(w, id, base(r), own, 3);
                    set_ring_word(w, id, pv::OUT, o, 2);
                    rgba = RGBA_BEAM_LIT;
                }
            }
            let at_a = add_rot(a, f32::from_bits(0x3ea0_d97c));
            beam(&mut out, at_a, NODE_R[r], r_to, rgba, counter);
            if !through { beam(&mut out, at_a, r_to, NODE_R[r], RGBA_BEAM_BACK, counter); }
        }
    }
    // The bands and the nodes.
    let pulse = ((((counter % 0x3c) * 6) as f32) * 0.017_453_292).sin() * 0.5 + 0.5;
    for r in 0..3 {
        for i in 0..12i32 {
            let mut a = f32::from_bits(0xbe86_0a92);
            let mut rgba = RGBA_RING;
            if active == r as i32 + 1 {
                rgba = crate::hud::tween_color(pulse, RGBA_RING_ACTIVE, RGBA_RING_ACTIVE & 0x00ff_ffff);
                if spin != 0.0 { a = add_rot(a, spin); }
            }
            a = add_rot(a, deg(i * 30));
            band(&mut out, a, add_rot(a, f32::from_bits(0x3f06_0a92)), BANDS[r][0], BANDS[r][1], rgba);
            let k = i + rot[r];
            let v = ring_word(w, id, base(r), k);
            if v == 0 { continue; }
            let na = add_rot(a, f32::from_bits(0x3e86_0a92));
            let kind = if v >= 3 { 0 } else if v == 2 { 1 } else { 2 };
            node(&mut out, na, NODE_R[r], kind, bg);
            if v >= 3 { set_ring_word(w, id, base(r), k, 2); }
        }
    }
    // The outputs.
    for i in 0..12i32 {
        let v = c::pi32(w, id, pv::OUT + 4 * i as usize);
        if v == 0 { continue; }
        let a = add_rot(f32::from_bits(0xbdb2_b8c2), i as f32 * 30.0 * 0.017_453_292);
        let rgba = if v == 2 { RGBA_OUT_POWERED } else if counter & 0x20 != 0 { RGBA_OUT_LIT } else { RGBA_OUT_B };
        band(&mut out, a, add_rot(a, f32::from_bits(0x3e32_b8c2)), OUT_BAND[0], OUT_BAND[1], rgba);
    }
    w.svc.draw_callbacks.rings.extend(out);
}
