//! **The Snagglebeast, class 1106** (level07 `0x314150` with its hits `0x313420`, turn `0x313de8`, walk `0x313af0` /
//! `0x313988` / `0x313d30`, walls `0x313350`, spit `0x3122b0`, look `0x313f60` and big head `0x314058`; 1 placed; census
//! U271; the name is descriptive [L]). Umbris' boss: it wakes when Ratchet is in its arena (cuboid +0x1a4), shows a
//! boss meter of six segments, and fights in three phases (+0xbc / 3), each in three steps (+0xbc % 3). It roams the
//! arena graph (+0x188, walled by the paths +0x170..; the arena's outline +0x290) after Ratchet, turning in place when
//! he is more than +0x98 off its nose, and attacks: a stomp (two shockwave rings 1046, dust, a camera shake), a fire
//! sweep along the ground in front of where he is going (fire particles 58, a ground line drawn), a spit volley
//! (twenty 1049 globs), the tongue grab (its 10-segment tongue reaches for him; caught, he is held (state 0x78) and
//! shaken: his ammo of the weapons in gp−0x5940 falls out in pickups along the ammo path +0x18c), and against a
//! platform he stands on (a 0x459..0x45c above it) a breath beam that breaks it. Each sixth of its health ends a
//! step: it reels, phase steps advance, the walls of the next arena open (its gates +0x1d8.. get +0xbc = 1) and it
//! retreats to that arena's node; while it is stunned a shimmer grows round its head and its health drains. At zero it
//! dies in a great explosion: the mission is done, the checkpoint moves (cuboid +0x1f0), the infobot +0x1cc appears.
//! Its head grows with the enemies' big-head cheat (list 0xb), and in scenes with the actors' one. Read from the
//! level07 decomp and disassembly (the stack arguments, the vector operands the decompile drops). Native `f32`.
//!
//! The effects (the tongue, the beam, the shimmer, the ground line, the light, the shockwave rings 1046 and the spit
//! 1049) are in [`super::umbris_beast_fx`].
//!
//! **Pvars** ([`pv`]): +0x20 the damage record (health, s16 max), +0x60 the flash, +0x70 the walker record (radius
//! 0x400, top speed +0x94, the turn limit +0x98 (100°)), +0xc0 the point it turns to, +0x100 the step, +0x110 / +0x120
//! the beam's / sweep's ends, +0x130 its step, +0x140 / +0x150 the sweep head and the lunge point, +0x160 a timer,
//! +0x164 a sub-step, +0x168 the yaw spring, +0x16c a length, +0x170..+0x184 the walls, +0x188 the graph, +0x18c /
//! +0x190 the ammo path and its length, +0x194 / +0x196 / +0x198 / +0x19a s16 the stomp, grab, spit and beam timers,
//! +0x19c the open walls, +0x19d the arena, +0x19e s16 the ammo a pickup, +0x1a0 the arena nodes path, +0x1a4 the
//! arena cuboid, +0x1a8 the node it heads for, +0x1ac the ammo drop's distance, +0x1b0 the meter, +0x1b4 its value,
//! +0x1b8 the floor, +0x1bc the speed, +0x1c0 the nearest node, +0x1c4 / +0x1c5 / +0x1c6 / +0x1c7 bytes (the grab's
//! hold, the shimmer's fade, its flash, the grab allowed), +0x1c8 the shimmer, +0x1cc the infobot, +0x1d0 a riser,
//! +0x1d4 the arenas' nodes, +0x1d8 the gates (two per arena), +0x1f0 the checkpoint cuboid, +0x1f4..+0x1f7 the turn's
//! return state / its fixed point / the sequence / the retreat cooldown, +0x1f8 the loop sound, +0x1fc the shimmer's
//! drop, +0x200 the look-at record, +0x280.. the light, +0x290 the outline, +0x294 the grab's start, +0x298 a count,
//! +0x29c the cuboid of the platforms, +0x2a0 the big head.
//!
//! | address | what | port |
//! |---|---|---|
//! | top | the big heads; game mode 2 → nothing; Ratchet outside the arena cuboid → nothing; the shadow within 35; the hits; the target (30); the retreat / stomp / beam / grab / spit gates and the phase's mask of them; the meter (health·16, the step's sixths); the tongue's fade (not grabbing); the shimmer's fade (sound 15 looped); the look-at; the meter's HUD request | [`update`], [`gates`] |
//! | 0 | the paths' lengths, the arena nodes by their w, the walker flags, targetable, the walls graph (`0x28a440`), update / draw 0xff, the nearest node, the tongue reset (`0x317c98`), scale ×1.75, the turn limit 100°, +0x29 1, +0x58 / +0x5a 30 / 40, → 1 | [`init`] |
//! | 1 | idle (sequence 0 / 1 at random); the timer out: beam (4), stomp (3), grab (8), spit (2); retreat → 5 (sequence 10) | [`update`] |
//! | 2 | spit: dust at joints 0 / 1 every 6 ticks, the turn check, the spring to him, sequence 9 / 13 by the turn; spitting (`0x3122b0`): 21 globs at 9 a second; ending far (13 / 20) → spit timer `ticks(35)`, → 1 | [`spit`] |
//! | 3 | stomp: keys 31 / 67 / 101 a stomp (+0x164): a shockwave 1046 (25, 2, 8·dt), the light at joint 5 / 6, 55 dust bursts (type 65), the shake; after three and key 51: timers, → 1 | [`stomp`] |
//! | 4 | the fire sweep: the turn check; glow and type 61 at joints 2 / 3; at sequence 5: the sweep's ends ahead of him (his motion over 40 ticks), then its head moves over 60 ticks with the ground line drawn (`0x312420`, fire particles); → 1 | [`sweep`] |
//! | 5 / 6 | the chase: the target node (the platforms' cuboid: the nearest arena point; else the outline's point nearest him, or him when near it), the pace, the route through the graph (every 4th tick), turning (> 100°: → 7), the walk; stuck or arrived → 1; sequences 10 / 12; on the timer out the attack gates, the beam onto the platform (→ 9), the grab (→ 8); retreat over → 1; stomps at keys 27.5 / 73 | [`chase`] |
//! | 7 | turn in place (sequences 3 / 14 / 13), stomps at keys 16 / 37 / 34 | [`turn_in_place`] |
//! | 8 | the grab: the tongue (`0x317e60`) from joint 4 to him; at sequence 6: reaching, caught (within 0.6): held (0x78), lifted 2 above, shaken: the weapons' ammo out in pickups along the ammo path (`0x2ee3b8`, flung `0x26faf0`), sound 11; done → released (sequence 9 to spit, or 1) | [`grab`] |
//! | 9 | the beam onto a platform (+0x150): sequence 7; key 9.5: its start between joints 0 / 1; type 61 every 10 ticks; key 31.5: it grows to the point, then runs, then ends → 1 | [`beam`] |
//! | 0xa | dead: the meter released; wrapped: the death explosion (7, 5, 10, 1, 50; 20 streaks, 12 sparks, 64 puffs, shake) at the floor + 1, hidden, the infobot shown, → 0x10, z 20 | [`update`] |
//! | 0xb | reeling: wrapped → 9 (on a platform) or 1 | [`update`] |
//! | 0xc | the step ends: sequence 8 / 11, the phase advanced (at most 8), the nearer gate of the arena told 1, the yaw to it | [`update`] |
//! | 0xd | stunned: drool (type 45 at joint 10 at the water); phase 8: falls (×0.7, gravity) into the pit; else key 79 → shimmer out, → 0xe, sixth of the health to drain | [`update`] |
//! | 0xe | drained: slows; drool; stomps at keys 168 / 168.5; the health drains a sixth (flash red) a tick of `ticks(10)` → at 0: dead (`SetMissionDone`, the checkpoint, → 0xa); the drain done: the next arena's point (its walls closed), the route, → 0xf | [`update`] |
//! | 0xf | to the arena: key 198 → sequence 10, the jump's landing (joint 10, the arena point's height, the root's yaw); at sequence 10: there, the gate back to 0, speed 1.5, phase + 1, → 6 | [`update`] |
//! | 0x10 | the light freed; the riser +0x1d0 raised to 34.6, then deleted | [`update`] |
//! | end | the step (position − start), the blob shadow (2.5) below 0xa, the light's fade (`0x312150`) | [`update`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::units::umbris_beast_fx as bfx;
use crate::moby_update::creature::{self as c, damage, flash, fx, region, target, turn, walker, DT, DT2, SPEED, V};
use crate::moby_update::services::{HeroPose, World};

pub const REFERENCE_LEVEL: u32 = 7;
pub const UPDATE_FN: u32 = 0x31_4150;
pub const CLASSES: [i16; 1] = [1106];
const PVARS: usize = 0x320;
const DEG120: f32 = 2.094_395_2;
const DEG40: f32 = 0.698_131_7;
/// `0x20c600` (stride 0x10): the weapons the grab shakes out, −1 ends; and each one's least a pickup (its item
/// definition's s16 +0x3c, level07 0x179b40 + 0x4c·item).
const SHAKE_ITEMS: [(i32, i16); 12] = [(10, 5), (11, 3), (13, 2), (15, 50), (16, 50), (17, 5), (19, 50), (20, 4), (23, 5), (24, 10), (25, 4), (-1, 0)];
/// The death explosion (`SpawnBeamExplosion(0, 0, 7, 5, 10, 1, 50, m, step, floor + 1, 20, 12, 64, no sound, shake, 1)`).
const DEATH: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 7.0, flash2: 5.0, flash_dist: 10.0, scale: 1.0, light: 50.0, streaks: 20, sparks: 12, puffs: 0x40, debris: 1, sound: -1, shake: true };

/// Pvar offsets (module doc).
pub mod pv {
    pub const D: usize = 0x20;
    pub const FLASH: usize = 0x60;
    pub const J: usize = 0x70;
    pub const J_SPEED: usize = 0x80;
    pub const J_TOP: usize = 0x94;
    pub const TURN_MAX: usize = 0x98;
    pub const ARRIVE: usize = 0x9c;
    pub const J_FLAGS: usize = 0xa8;
    pub const TURN_AT: usize = 0xc0;
    pub const STEP: usize = 0x100;
    pub const A: usize = 0x110;
    pub const B: usize = 0x120;
    pub const AB: usize = 0x130;
    pub const HEAD: usize = 0x140;
    pub const LUNGE: usize = 0x150;
    pub const HIT_GROUND: usize = 0x15c;
    pub const TIMER: usize = 0x160;
    pub const SUB: usize = 0x164;
    pub const YAW_V: usize = 0x168;
    pub const LEN: usize = 0x16c;
    pub const WALLS: usize = 0x170;
    pub const GRAPH: usize = 0x188;
    pub const AMMO_PATH: usize = 0x18c;
    pub const AMMO_LEN: usize = 0x190;
    pub const STOMP_T: usize = 0x194;
    pub const GRAB_T: usize = 0x196;
    pub const SPIT_T: usize = 0x198;
    pub const BEAM_T: usize = 0x19a;
    pub const OPEN: usize = 0x19c;
    pub const ARENA: usize = 0x19d;
    pub const AMMO_EACH: usize = 0x19e;
    pub const POINTS: usize = 0x1a0;
    pub const ZONE: usize = 0x1a4;
    pub const NODE: usize = 0x1a8;
    pub const AMMO_AT: usize = 0x1ac;
    pub const METER_SLOT: usize = 0x1b0;
    pub const METER: usize = 0x1b4;
    pub const FLOOR: usize = 0x1b8;
    pub const SPEED: usize = 0x1bc;
    pub const NEAREST: usize = 0x1c0;
    pub const HOLD: usize = 0x1c4;
    pub const SHIMMER_MODE: usize = 0x1c5;
    pub const SHIMMER_FLASH: usize = 0x1c6;
    pub const GRAB_OK: usize = 0x1c7;
    pub const SHIMMER: usize = 0x1c8;
    pub const INFOBOT: usize = 0x1cc;
    pub const RISER: usize = 0x1d0;
    pub const NODES: usize = 0x1d4;
    pub const GATES: usize = 0x1d8;
    pub const CHECKPOINT: usize = 0x1f0;
    pub const BACK_STATE: usize = 0x1f4;
    pub const FIXED: usize = 0x1f5;
    pub const BACK_SEQ: usize = 0x1f6;
    pub const COOLDOWN: usize = 0x1f7;
    pub const SLOT: usize = 0x1f8;
    pub const SHIMMER_DZ: usize = 0x1fc;
    pub const LOOK: usize = 0x200;
    pub const LOOK_PITCH: usize = 0x264;
    pub const LOOK_YAW: usize = 0x268;
    pub const OUTLINE: usize = 0x290;
    pub const GRABBED: usize = 0x294;
    pub const COUNT: usize = 0x298;
    pub const PLATFORMS: usize = 0x29c;
    pub const BIG_HEAD: usize = 0x2a0;
}

fn st(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_st(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn phase(w: &World, id: MobyId) -> u8 { w.m(id).cmd }
fn seq_a(w: &World, id: MobyId) -> u8 { w.m(id).anim.seq_a }
fn seq_b(w: &World, id: MobyId) -> u8 { w.m(id).anim.seq_b }
fn wrapped(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn key(w: &World, id: MobyId) -> f32 { c::ground::key_time(w, id) }
fn passed(w: &World, id: MobyId, k: f32) -> bool { c::ground::passed_frame(w, id, k) }
fn set_anim_speed(w: &mut World, id: MobyId, s: f32) { w.mm(id).anim.speed = s; }
/// `if (+0x53 != s) MobyAnimBlend(m, s, rand_range(f0, f1), rand_range(t0, t1))`: the frame drawn first.
fn blend_r(w: &mut World, id: MobyId, s: u8, f: (i32, i32), t: (i32, i32)) {
    if seq_b(w, id) == s { return; }
    let fr = w.rng.rand_range(f.0, f.1);
    let tk = w.rng.rand_range(t.0, t.1);
    w.anim_blend(id, s, fr, tk);
}
/// The idles' blend: `rand() & 1` differing from the sequence → sequence `rand() & 1` at random frame / ticks.
fn idle_blend(w: &mut World, id: MobyId, f: (i32, i32), t: (i32, i32)) {
    let b = seq_b(w, id);
    let r = w.rng.rand() as u8 & 1;
    if b == r { return; }
    let s = w.rng.rand() as u8 & 1;
    let fr = w.rng.rand_range(f.0, f.1);
    let tk = w.rng.rand_range(t.0, t.1);
    w.anim_blend(id, s, fr, tk);
}
fn path_pts(w: &World, p: i32) -> Vec<V> {
    usize::try_from(p).ok().and_then(|p| w.svc.splines.get(p)).map(|v| v.iter().map(|q| q.map(f32::from_bits)).collect()).unwrap_or_default()
}
fn moby_at(w: &World, i: i32) -> Option<MobyId> { usize::try_from(i).ok().filter(|&m| m < w.table.mobys.len()) }
fn heading(a: V, b: V) -> f32 { c::atan(b[0] - a[0], b[1] - a[1]) }
fn yaw_spring(w: &mut World, id: MobyId, h: f32, acc: f32, damp: f32, max: f32) { turn::spring_turn2_pvar(w, id, h, acc, damp, max, pv::YAW_V); }
fn release_slot(w: &mut World, id: MobyId) {
    let s = c::pi32(w, id, pv::SLOT);
    if s != -1 { w.release_sound(s, id); }
    c::set_pi32(w, id, pv::SLOT, -1);
}
fn hero_held(w: &World) -> bool { w.hero.state == 0x78 }

/// `0x29e610(0, p, path)`: the index of the path point nearest `p`.
pub fn nearest_point(pts: &[V], p: V) -> usize {
    let mut best = 1e11f32;
    let mut at = 0;
    for (i, q) in pts.iter().enumerate() {
        let d = c::dist3(*q, p);
        if d < best {
            best = d;
            at = i;
        }
    }
    at
}

/// `0x313350`: the walls: +0x170 / +0x174 / +0x178 always, +0x17c / +0x180 / +0x184 while their bit of +0x19c is clear.
fn walls(w: &World, id: MobyId) -> Vec<usize> {
    let open = c::pu8(w, id, pv::OPEN);
    let mut out = Vec::new();
    for (k, bit) in [(0, None), (1, None), (2, None), (3, Some(1)), (4, Some(2)), (5, Some(4))] {
        let p = c::pi32(w, id, pv::WALLS + 4 * k);
        if p == -1 { continue; }
        if bit.is_some_and(|b| open & b != 0) { continue; }
        out.push(p as usize);
    }
    out
}

/// Level07 `0x314150` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < PVARS { w.mm(id).pvars.resize(PVARS, 0); }
    crate::moby_update::manip::big_head(w, 2.5, id, 0xb, id, pv::BIG_HEAD);
    crate::moby_update::manip::scene_big_head(w, &CLASSES, 0xb, f32::from_bits(0x4030_0000));
    if w.svc.game_mode == 2 { return; }
    let zone = c::pi32(w, id, pv::ZONE);
    if zone != -1 {
        let h = super::hero_pos(w);
        if !w.in_cuboid([h[0], h[1], h[2]], zone) { return; }
    }
    let cam = w.camera_point();
    if w.m(id).visible != 0 && c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 35.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0x1c;
    }
    let start = c::pos(w, id);
    hits(w, id);
    let t = target::acquire(w, id, 30.0);
    let pos = c::pos(w, id);
    let d_t = c::dist2(t.pos, pos);
    let h_t = heading(pos, t.pos);
    let hero = super::hero_pos(w);
    let h_h = heading(pos, hero);
    let g = gates(w, id, &t, d_t, h_t);
    meter(w, id);
    bfx::keep_draw_inputs(w, id);
    if st(w, id) != 8 && st(w, id) != 0 { bfx::tongue_fade(w, id); }
    shimmer_fade(w, id);
    if 0.0 < c::pf(w, id, pv::SHIMMER) { bfx::register(w, id, bfx::SHIMMER_FN); }
    look_at(w, id, hero);
    if !matches!(st(w, id), 0 | 0xa | 0x10) {
        let v = c::pi32(w, id, pv::METER);
        let max = (c::pi16(w, id, pv::D + 4) as i32) << 4 | 0xf;
        w.svc.hud.boss_meter(meter_key(w, id), v, max);
        c::set_pi32(w, id, pv::METER_SLOT, 1);
    }
    let state_now = st(w, id);
    match state_now {
        0 => init(w, id),
        1 => idle(w, id, &g),
        2 => spit_state(w, id, &t, h_t, d_t),
        3 => stomp(w, id, h_h, &g),
        4 => sweep(w, id, &t),
        5 | 6 => chase(w, id, &t, d_t, &g),
        7 => turn_in_place(w, id, hero, &g),
        8 => grab(w, id, h_h, &g),
        9 => beam(w, id),
        0xa => {
            if c::pi32(w, id, pv::METER_SLOT) != -1 {
                let max = (c::pi16(w, id, pv::D + 4) as i32) << 4 | 0xf;
                w.svc.hud.release(crate::hud::Request::boss(meter_key(w, id), max));
                c::set_pi32(w, id, pv::METER_SLOT, -1);
            }
            if wrapped(w, id) {
                let s = c::set_len3(c::pv4(w, id, pv::STEP), 1.0);
                c::set_pv4(w, id, pv::STEP, [s[0], s[1], 1.0, s[3]]);
                let p = c::pos(w, id);
                let at = [p[0], p[1], c::pf(w, id, pv::FLOOR) + 1.0, p[3]];
                fx::beam_explosion(w, &DEATH, Some(id), at);
                w.mm(id).mode |= 0x41;
                if let Some(ib) = moby_at(w, c::pi32(w, id, pv::INFOBOT)) { crate::moby_update::classes::infobot::show(w, ib); }
                set_st(w, id, 0x10);
                w.mm(id).position[2] = 20.0;
            }
        }
        0xb => {
            if wrapped(w, id) {
                if g.platform {
                    lunge_at_platform(w, id);
                } else {
                    to_idle(w, id);
                }
            }
        }
        0xc => step_over(w, id),
        0xd => stunned(w, id),
        0xe => drain(w, id),
        0xf => to_arena(w, id),
        0x10 => {
            bfx::light_free(w, id);
            let Some(r) = moby_at(w, c::pi32(w, id, pv::RISER)) else {
                w.delete_moby(id);
                return;
            };
            let z = w.m(r).position[2];
            if 34.5 <= z {
                w.delete_moby(id);
                return;
            }
            w.mm(r).position[2] = z + (34.6 - z) * 0.1;
        }
        _ => {}
    }
    let p = c::pos(w, id);
    c::set_pv4(w, id, pv::STEP, c::sub(p, start));
    if st(w, id) < 10 { crate::shadows::blob(w, 2.5, id); }
    bfx::light_tick(w, id);
}

fn meter_key(w: &World, id: MobyId) -> u32 { crate::hud::calls::pvar_key(w.m(id).spawn_id, id, pv::METER) }

/// The attack gates of the top (module doc).
pub struct Gates {
    retreat: bool,
    stomp: bool,
    beam: bool,
    grab: bool,
    spit: bool,
    on_node: bool,
    platform: bool,
}

fn gates(w: &mut World, id: MobyId, t: &target::Target, d_t: f32, h_t: f32) -> Gates {
    let pos = c::pos(w, id);
    let d_h = c::dist2(pos, super::hero_pos(w));
    let s = st(w, id);
    let ph = phase(w, id);
    let mut retreat = false;
    let mut cool = c::pu8(w, id, pv::COOLDOWN);
    let out = crate::moby_update::services::fast_dec_timer_u8(&mut cool) != 0;
    c::set_pu8(w, id, pv::COOLDOWN, cool);
    if out && ((8.0 < d_h && s != 5) || (6.0 < d_h && (s == 5 || ph % 3 == 1))) { retreat = true; }
    let mut stomp = c::dec_timer_pvar_s16(w, id, pv::STOMP_T) != 0;
    let beam_out = c::dec_timer_pvar_s16(w, id, pv::BEAM_T) != 0;
    let mut beam = beam_out && 9.0 < d_t;
    let mut grab = false;
    if c::pu8(w, id, pv::GRAB_OK) != 0 && t.moby.is_some() && t.moby == w.hero_moby && d_t < 30.0 && c::sub_rot(h_t, c::yaw(w, id)).abs() < f32::from_bits(0x3fb2_b8c2) {
        grab = true;
    }
    let mut spit = false;
    if c::dec_timer_pvar_s16(w, id, pv::SPIT_T) != 0 {
        if s == 5 || ph % 3 == 1 {
            if d_t < 14.0 { spit = true; }
        } else if s != 5 && d_t < 10.0 {
            spit = true;
        }
    }
    let mut on_node = false;
    let mut platform = false;
    if w.m(id).position[2] + 1.0 < w.hero.ground_z.to_f32() {
        if let Some(m) = w.hero.ground_moby {
            let oc = w.m(m).o_class;
            if 0x458 < oc { platform = oc < 0x45d; }
        }
    }
    match ph {
        0 => {
            retreat = false;
            grab = false;
        }
        1 | 4 | 7 => {
            stomp = false;
            grab = false;
            if s != 1 { beam = false; }
        }
        2 | 5 | 8 => {
            spit = false;
            stomp = false;
            grab = false;
            beam = false;
        }
        3 => {
            if c::pi32(w, id, pv::NODE) == -1 {
                retreat = false;
            } else {
                retreat = true;
                on_node = true;
                stomp = false;
                spit = false;
            }
            grab = false;
        }
        6 => {
            if c::pi32(w, id, pv::NODE) == -1 {
                retreat = false;
            } else {
                retreat = true;
                on_node = true;
                stomp = false;
                spit = false;
            }
        }
        _ => {}
    }
    Gates { retreat, stomp, beam, grab, spit, on_node, platform }
}

/// The meter's value +0x1b4 (module doc).
fn meter(w: &mut World, id: MobyId) {
    let hp = c::pf(w, id, pv::D);
    let max = c::pi16(w, id, pv::D + 4) as i32;
    let mut v = ((hp * 16.0) as i32 as u32 & 0xfff0) as i32;
    if (((max as f32 - 0.5) * 16.0) as i32) <= v { v |= 0xf; }
    let ph = phase(w, id) as i32;
    if ph % 3 == 0 {
        if 0.5 < hp {
            let sixth = max / 6;
            let mut s = if sixth != 0 { ((hp as i32 % sixth) << 4) / sixth } else { 0 };
            if s < 0 { s = 0; }
            v += s & 0xf;
        } else {
            v = 0;
        }
    }
    if ph % 3 == 2 { v += 0xf; }
    c::set_pi32(w, id, pv::METER, v);
}

/// The shimmer's fade in (+0x1c5 1) / out (2), sound 15 looped while it changes.
fn shimmer_fade(w: &mut World, id: MobyId) {
    let mode = c::pu8(w, id, pv::SHIMMER_MODE);
    if mode != 1 && mode != 2 { return; }
    if c::pi32(w, id, pv::SLOT) == -1 {
        let s = w.play_sound(0xf, 4, id);
        c::set_pi32(w, id, pv::SLOT, s);
    }
    let mut v = c::pf(w, id, pv::SHIMMER);
    if mode == 1 {
        v += SPEED * 0.01;
        if 1.0 <= v { c::set_pu8(w, id, pv::SHIMMER_MODE, 0); }
        v = v.min(1.0);
    } else {
        v -= SPEED * 0.01;
        if v <= 0.0 {
            release_slot(w, id);
            c::set_pu8(w, id, pv::SHIMMER_MODE, 0);
        }
        v = v.max(0.0);
    }
    c::set_pf(w, id, pv::SHIMMER, v);
}

/// `0x313f60(m, P, hero)`: the head's look (list 0xb) at Ratchet, within ±80° (else ±100°, or 0 from state 0xa).
fn look_at(w: &mut World, id: MobyId, at: V) {
    let mut yaw = if st(w, id) < 10 { c::sub_rot(heading(c::pos(w, id), at), c::yaw(w, id)) } else { 0.0 };
    if 1.396_263_4 < yaw {
        yaw = f32::from_bits(0x3fb2_b8c2);
    } else if yaw < -1.396_263_4 {
        yaw = f32::from_bits(0xbfb2_b8c2);
    }
    c::set_pf(w, id, pv::LOOK_YAW, yaw);
    let p = c::pf(w, id, pv::SHIMMER_DZ);
    c::set_pf(w, id, pv::LOOK_PITCH, p);
    crate::moby_update::manip::look(w, id, id, pv::LOOK, 0xb, SPEED * 0.03, SPEED * 0.3);
}

/// State 0 (module doc).
fn init(w: &mut World, id: MobyId) {
    set_st(w, id, 1);
    w.mm(id).cmd = 0;
    c::set_pi16(w, id, bfx::pv::LIGHT, -1);
    let ammo = c::pi32(w, id, pv::AMMO_PATH);
    c::set_pf(w, id, pv::AMMO_LEN, 0.0);
    if let Some(p) = usize::try_from(ammo).ok().filter(|&p| p < w.svc.splines.len()) {
        let n = w.svc.splines[p].len();
        let mut total = 0.0;
        for i in 0..n {
            let (a, b) = (w.svc.splines[p][i].map(f32::from_bits), w.svc.splines[p][(i + 1) % n].map(f32::from_bits));
            let l = c::dist3(a, b);
            w.svc.splines[p][i][3] = l.to_bits();
            total += l;
        }
        c::set_pf(w, id, pv::AMMO_LEN, total);
    }
    if let Some(p) = usize::try_from(c::pi32(w, id, pv::OUTLINE)).ok().filter(|&p| p < w.svc.splines.len()) {
        let n = w.svc.splines[p].len();
        for i in 0..n {
            let (a, b) = (w.svc.splines[p][i].map(f32::from_bits), w.svc.splines[p][(i + 1) % n].map(f32::from_bits));
            w.svc.splines[p][i][3] = c::dist3(a, b).to_bits();
        }
    }
    let graph = c::pi32(w, id, pv::GRAPH);
    let pts = path_pts(w, graph);
    for (i, q) in pts.iter().enumerate() {
        let k = q[3] as i32 as u32;
        if k < 4 { c::set_pu8(w, id, pv::NODES + k as usize, i as u8); }
    }
    let f = c::pi32(w, id, pv::J_FLAGS) as u32 & !8;
    c::set_pi32(w, id, pv::J_FLAGS, f as i32);
    w.mm(id).mode |= mode::TARGETABLE;
    if let Ok(g) = usize::try_from(graph) {
        let wl: Vec<usize> = (0..3).filter_map(|k| usize::try_from(c::pi32(w, id, pv::WALLS + 4 * k)).ok()).collect();
        region::graph_init(w, &wl, g);
    }
    c::set_pu8(w, id, pv::OPEN, 7);
    let m = w.mm(id);
    m.update_dist = 0xff;
    m.draw_dist = 0xff;
    c::set_pi32(w, id, pv::METER_SLOT, -1);
    c::set_pi32(w, id, pv::NODE, -1);
    let n = nearest_point(&pts, c::pos(w, id));
    c::set_pi32(w, id, pv::NEAREST, n as i32);
    bfx::tongue_reset(w);
    w.mm(id).scale *= 1.75;
    c::set_pu8(w, id, pv::GRAB_OK, 0);
    c::set_pf(w, id, pv::TURN_MAX, f32::from_bits(0x3fdf_66f3));
    c::set_pu8(w, id, pv::D + 9, 1);
    c::set_pu8(w, id, 0x58, 30);
    c::set_pu8(w, id, 0x5a, 40);
}

/// State 1 (module doc).
fn idle(w: &mut World, id: MobyId, g: &Gates) {
    if wrapped(w, id) { idle_blend(w, id, (0, 0xf), (0x1e, 0x2a)); }
    if c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 {
        if g.beam {
            c::set_pi32(w, id, pv::SUB, 0);
            c::set_pi32(w, id, pv::TIMER, 0);
            blend_r(w, id, 5, (0, 2), (0x12, 0x14));
            set_st(w, id, 4);
        }
        let mut stomp_now = g.stomp;
        if stomp_now && st(w, id) == 5 && w.rng.randi(4) != 0 { stomp_now = false; }
        if stomp_now {
            c::set_pi32(w, id, pv::SUB, 0);
            blend_r(w, id, 4, (0, 9), (0x14, 0x28));
            set_st(w, id, 3);
            set_anim_speed(w, id, 1.0);
        }
        if g.grab { start_grab(w, id, (0x0f, 0x19)); }
        if g.spit {
            blend_r(w, id, 9, (0, 2), (0x11, 0x19));
            set_st(w, id, 2);
        }
    }
    if g.retreat && st(w, id) != 8 {
        blend_r(w, id, 10, (0, 4), (0x14, 0x1e));
        set_anim_speed(w, id, 1.5);
        c::set_pi32(w, id, pv::COUNT, 0);
        set_st(w, id, 5);
    }
}

fn start_grab(w: &mut World, id: MobyId, t: (i32, i32)) {
    blend_r(w, id, 6, (0, 2), t);
    c::set_pi32(w, id, pv::SUB, 0);
    c::set_pf(w, id, pv::LEN, 0.0);
    c::set_pu8(w, id, pv::HOLD, 0);
    set_st(w, id, 8);
    c::set_pi32(w, id, pv::GRABBED, 0);
    let s = w.play_sound(9, 4, id);
    c::set_pi32(w, id, pv::SLOT, s);
}

fn to_idle(w: &mut World, id: MobyId) { set_st(w, id, 1); }

/// The timers the attacks end with (`LAB_00315078` / `LAB_00317804`): +0x160 = `ticks(n)`, → 1.
fn rest(w: &mut World, id: MobyId, n: i32) {
    let t = w.ticks(n);
    c::set_pi32(w, id, pv::TIMER, t);
    to_idle(w, id);
}

/// `0x313de8(m, P, p, seq, fixed)`: more than +0x98 off its nose → the turn (sequence 3 / 14 by the side), the return
/// state, → 7; true.
fn turn_check(w: &mut World, id: MobyId, p: V, seq: u8, fixed: u8) -> bool {
    let pos = c::pos(w, id);
    let h = heading(pos, p);
    if c::diff_rots(c::yaw(w, id), h) <= c::pf(w, id, pv::TURN_MAX) { return false; }
    c::set_pf(w, id, pv::YAW_V, 0.0);
    set_anim_speed(w, id, 1.0);
    let s = st(w, id);
    c::set_pu8(w, id, pv::BACK_STATE, s);
    c::set_pu8(w, id, pv::BACK_SEQ, seq);
    c::set_pu8(w, id, pv::FIXED, fixed);
    c::set_pv4(w, id, pv::TURN_AT, p);
    if h < 0.0 { blend_r(w, id, 3, (0, 1), (0xf, 0x14)) } else { blend_r(w, id, 0xe, (0, 1), (0xf, 0x14)) }
    set_st(w, id, 7);
    true
}

/// State 2 (module doc).
fn spit_state(w: &mut World, id: MobyId, t: &target::Target, h_t: f32, d_t: f32) {
    let yaw0 = c::yaw(w, id);
    if w.counter.is_multiple_of(6) { bfx::part61(w, 600_000.0, 5.0, id, 0); }
    if (w.counter.wrapping_sub(2)).is_multiple_of(6) { bfx::part61(w, 600_000.0, 5.0, id, 1); }
    let hero = super::hero_pos(w);
    if turn_check(w, id, hero, 9, 0) { return; }
    yaw_spring(w, id, h_t, DT2 * DEG120, DT2 * DEG40, DT * DEG120);
    if DT * 0.174_532_92 < c::diff_rots(c::yaw(w, id), yaw0) {
        blend_r(w, id, 0xd, (0, 1), (0xf, 0x16));
    } else {
        blend_r(w, id, 9, (0, 1), (0xf, 0x16));
    }
    let a = seq_a(w, id);
    if a == 9 || a == 0xd {
        let s = spit(w, id, t.pos);
        set_st(w, id, s);
    }
    let ph = phase(w, id) % 3;
    let far = (ph == 0 && 13.0 < d_t) || (ph == 1 && 20.0 < d_t);
    if !far { return; }
    let t35 = w.ticks(0x23);
    c::set_pi16(w, id, pv::SPIT_T, t35 as i16);
    rest(w, id, 0x1e);
}

/// `0x3122b0(m, P, at)`: every 9 ticks a glob 1049 from joint 0 / 1 (alternately) at `at` (lobbed: the flat
/// direction scaled by 10.7 with z + 0.5, at most 0, at 9·dt), sound 0, the light; after 21 → the spit timer and 1.
fn spit(w: &mut World, id: MobyId, at: V) -> u8 {
    if c::dec_timer_pvar_i32(w, id, pv::TIMER) == 0 { return st(w, id); }
    let k = c::pi32(w, id, pv::SUB) as usize & 1;
    let jp = w.joint_point(id, k);
    let mut d = c::sub(at, jp);
    d = [d[0] * f32::from_bits(0x412b_3333), d[1] * f32::from_bits(0x412b_3333), d[2] * f32::from_bits(0x412b_3333), d[3]];
    d[2] = (d[2] + 0.5).min(0.0);
    let v = c::set_len3(d, DT * 9.0);
    bfx::glob(w, jp, v, id);
    w.play_sound(0, 0, id);
    let t30 = w.ticks(0x1e);
    bfx::light(w, id, 10.0, 0, Some(jp), t30, 0x8000_60ff, 0);
    let t9 = w.ticks(9);
    c::set_pi32(w, id, pv::TIMER, t9);
    let n = c::pi32(w, id, pv::SUB) + 1;
    c::set_pi32(w, id, pv::SUB, n);
    if 0x14 < n {
        let t35 = w.ticks(0x23);
        c::set_pi16(w, id, pv::SPIT_T, t35 as i16);
        let t30 = w.ticks(0x1e);
        c::set_pi32(w, id, pv::TIMER, t30);
        return 1;
    }
    st(w, id)
}

/// State 3 (module doc).
fn stomp(w: &mut World, id: MobyId, h_h: f32, g: &Gates) {
    yaw_spring(w, id, h_h, DT2 * std::f32::consts::FRAC_PI_2, DT2 * DEG40, DT * std::f32::consts::FRAC_PI_3);
    if passed(w, id, 31.0) || passed(w, id, 67.0) || passed(w, id, 101.0) {
        let n = c::pi32(w, id, pv::SUB) + 1;
        c::set_pi32(w, id, pv::SUB, n);
        let p = c::pos(w, id);
        c::set_pf(w, id, pv::YAW_V, 0.0);
        bfx::shockwave(w, 1.0, 25.0, 2.0, DT * 8.0, 1.0, id, p, 0x3a);
        let jp = w.joint_point(id, (n as usize & 1) + 5);
        let t30 = w.ticks(0x1e);
        bfx::light(w, id, 10.0, 1, Some(jp), t30, 0x8000_60ff, 0);
        let z = w.m(id).position[2];
        let base = [jp[0], jp[1], z, jp[3]];
        for _ in 0..0x37 {
            let a = w.rng.rand_angle();
            let b = w.rng.randf(f32::from_bits(0x3ec9_0fdb), f32::from_bits(0x3f49_0fdb));
            let v = fx::polar(DT * 5.0, a, b);
            let mut q = c::set_len3(v, 0.75);
            q[2] += 0.1;
            let q = c::add(q, base);
            bfx::part65(w, [q[0], q[1], q[2], 0.0], v);
        }
        bfx::shake(w, id);
    }
    if c::pi32(w, id, pv::SUB) < 3 || !passed(w, id, 51.0) { return; }
    if g.beam {
        c::set_pi16(w, id, pv::STOMP_T, 0xc0);
        c::set_pi32(w, id, pv::TIMER, 0xaa);
    } else {
        let t = w.ticks(0x3c);
        c::set_pi16(w, id, pv::STOMP_T, t as i16);
        let t55 = w.ticks(0x37);
        c::set_pi32(w, id, pv::TIMER, t55);
        let s = c::pi16(w, id, pv::BEAM_T);
        if s as i32 <= w.ticks(0x37) {
            let add = w.ticks(0x3c) as i16;
            c::set_pi16(w, id, pv::BEAM_T, s.wrapping_add(add));
        }
    }
    set_anim_speed(w, id, 1.0);
    idle_blend(w, id, (0, 0xf), (0x1e, 0x2a));
    to_idle(w, id);
}

/// State 4, the fire sweep (module doc).
fn sweep(w: &mut World, id: MobyId, t: &target::Target) {
    let hero = super::hero_pos(w);
    if turn_check(w, id, hero, 5, 0) { return; }
    let t60 = w.ticks(0x3c);
    if c::pi32(w, id, pv::TIMER) < t60 / 2 {
        let jp = w.joint_point(id, 2);
        let t30 = w.ticks(0x1e);
        bfx::light(w, id, 10.0, 2, Some(jp), t30, 0x8000_60ff, 0);
        if w.counter & 7 == 0 { bfx::part61(w, 0.0, 0.2, id, 2); }
        if w.counter.wrapping_sub(4) & 7 == 0 { bfx::part61(w, 0.0, 0.2, id, 3); }
    }
    if seq_a(w, id) != 5 { return; }
    let mut p = t.pos;
    p[2] = w.ground_height(crate::ps2v::Pf::f(0.5), crate::moby_update::services::pv(p), 0).to_f32();
    c::set_pi32(w, id, pv::TIMER, c::pi32(w, id, pv::TIMER) + 1);
    if c::pi32(w, id, pv::SUB) == 0 {
        let pos = c::pos(w, id);
        let mut d = c::sub(pos, p);
        d[2] = 0.0;
        let d = c::set_len3(d, 1.0);
        let small = c::set_len3(d, f32::from_bits(0x3d99_999a));
        let ty = t.rot[2];
        let f = [ty.cos(), ty.sin(), 0.0, 0.0];
        let dot = c::dot3(f, d);
        let mut side = c::sub(f, c::set_len3(d, dot));
        side[2] = 0.0;
        let a = c::sub(c::add(c::set_len3(d, 8.0), p), c::set_len3(side, 4.0));
        let b = c::sub(c::add(c::set_len3(side, 8.0), p), c::set_len3(d, 2.0));
        let disp = w.hero.disp.map(|x| f32::from_bits(x.0));
        let lead = c::scale([disp[0], disp[1], 0.0, 0.0], w.ticks(0x28) as f32);
        let mid = c::add(c::scale(c::sub(b, a), 0.5), a);
        let shift = c::sub(lead, c::sub(c::sub(mid, p), small));
        let (a, b) = (c::add(a, shift), c::add(b, shift));
        c::set_pv4(w, id, pv::A, a);
        c::set_pv4(w, id, pv::B, b);
        c::set_pv4(w, id, pv::HEAD, a);
        let n = w.ticks(0x3c) as f32;
        c::set_pv4(w, id, pv::AB, c::scale(c::sub(b, a), 1.0 / n));
        c::set_pv4(w, id, pv::LUNGE, a);
        c::set_pf(w, id, pv::HIT_GROUND, 0.0);
        c::set_pi32(w, id, pv::SUB, 1);
        return;
    }
    if c::pi32(w, id, pv::TIMER) < w.ticks(0x3c) {
        let h = c::add(c::pv4(w, id, pv::HEAD), c::pv4(w, id, pv::AB));
        c::set_pv4(w, id, pv::HEAD, h);
        bfx::register(w, id, bfx::GROUND_FN);
        c::set_pi32(w, id, pv::SUB, 2);
        return;
    }
    c::set_pi32(w, id, pv::SUB, 0);
    let t240 = w.ticks(0xf0);
    c::set_pi16(w, id, pv::BEAM_T, t240 as i16);
    rest(w, id, 200);
}

/// States 5 / 6, the chase (module doc).
fn chase(w: &mut World, id: MobyId, t: &target::Target, d_t: f32, g: &Gates) {
    let walls = walls(w, id);
    let pos = c::pos(w, id);
    let mut arrive = 2.0;
    let far = 6.0;
    let (goal, d_goal);
    if !g.on_node {
        arrive = 6.0;
        let plat = c::pi32(w, id, pv::PLATFORMS);
        if plat != -1 && c::pu8(w, id, pv::OPEN) & 3 == 0 && w.in_cuboid([t.pos[0], t.pos[1], t.pos[2]], plat) {
            let pts = path_pts(w, c::pi32(w, id, pv::POINTS));
            let q = pts.get(nearest_point(&pts, t.pos)).copied().unwrap_or(t.pos);
            goal = q;
            d_goal = c::dist2(pos, q);
            arrive = far;
        } else {
            let pts = path_pts(w, c::pi32(w, id, pv::OUTLINE));
            let q = pts.get(nearest_point(&pts, t.pos)).copied().unwrap_or(t.pos);
            let dq = c::dist2(pos, q);
            if dq < c::pf(w, id, pv::ARRIVE) + 1.0 {
                goal = t.pos;
                arrive = far;
            } else {
                goal = q;
            }
            d_goal = dq;
        }
    } else {
        let pts = path_pts(w, c::pi32(w, id, pv::GRAPH));
        let q = pts.get(c::pi32(w, id, pv::NODE).max(0) as usize).copied().unwrap_or(pos);
        goal = q;
        d_goal = c::dist2(pos, q);
    }
    pace(w, id, d_goal, arrive, 1.5);
    if c::pi32(w, id, pv::COUNT) & 3 == 0 {
        let mut via = c::pv4(w, id, pv::HEAD);
        route(w, id, &walls, goal, &mut via);
        c::set_pv4(w, id, pv::HEAD, via);
    }
    let via = c::pv4(w, id, pv::HEAD);
    c::set_pi32(w, id, pv::COUNT, c::pi32(w, id, pv::COUNT) + 1);
    let h = heading(c::pos(w, id), via);
    if c::pf(w, id, pv::TURN_MAX) < c::diff_rots(c::yaw(w, id), h) {
        set_anim_speed(w, id, 1.0);
        let s = st(w, id);
        c::set_pu8(w, id, pv::BACK_STATE, s);
        c::set_pu8(w, id, pv::BACK_SEQ, 10);
        c::set_pu8(w, id, pv::FIXED, 1);
        c::set_pv4(w, id, pv::TURN_AT, via);
        blend_r(w, id, 3, (0, 1), (0xf, 0x16));
        set_st(w, id, 7);
    } else {
        let r = walk(w, id, arrive, goal, via);
        let moved = c::len3(c::pv4(w, id, pv::STEP));
        let near = !g.on_node && d_t < 20.0;
        if r == 4 || moved <= DT * 0.01 {
            let t90 = w.ticks(0x5a);
            c::set_pu8(w, id, pv::COOLDOWN, t90 as u8);
            idle_blend(w, id, (0, 9), (0x14, 0x28));
            c::set_pi32(w, id, pv::NODE, -1);
            to_idle(w, id);
            return;
        }
        if seq_b(w, id) == 10 {
            if near && wrapped(w, id) {
                let tk = w.rng.rand_range(10, 0xd);
                w.anim_blend(id, 0xc, 5, tk);
            }
        } else if !near && wrapped(w, id) {
            let tk = w.rng.rand_range(10, 0xd);
            w.anim_blend(id, 10, 5, tk);
        }
        if c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 {
            if g.beam {
                c::set_pi32(w, id, pv::SUB, 0);
                c::set_pi32(w, id, pv::TIMER, 0);
                set_anim_speed(w, id, 1.0);
                blend_r(w, id, 5, (0, 2), (0x14, 0x19));
                set_st(w, id, 4);
            }
            if g.stomp {
                c::set_pi32(w, id, pv::SUB, 0);
                blend_r(w, id, 4, (0, 5), (0x14, 0x28));
                set_st(w, id, 3);
                set_anim_speed(w, id, 1.0);
            }
            if g.platform { lunge_at_platform(w, id); }
            if g.grab {
                set_anim_speed(w, id, 1.0);
                start_grab(w, id, (0x14, 0x19));
            }
        }
        if !g.retreat {
            set_anim_speed(w, id, 1.0);
            idle_blend(w, id, (0, 2), (0x1c, 0x28));
            to_idle(w, id);
        }
        if seq_a(w, id) == 0xc { spit(w, id, t.pos); }
    }
    let a = seq_a(w, id);
    if a != 10 && a != 0xc { return; }
    if passed(w, id, 27.5) { bfx::shake(w, id); }
    if passed(w, id, 73.0) { bfx::shake(w, id); }
}

/// The beam onto the platform Ratchet stands on: +0x150 = its position 3 up, → 9.
fn lunge_at_platform(w: &mut World, id: MobyId) {
    c::set_pi32(w, id, pv::SUB, 0);
    if let Some(m) = w.hero.ground_moby {
        let mut p = w.m(m).position;
        p[2] += 3.0;
        c::set_pv4(w, id, pv::LUNGE, p);
    }
    set_st(w, id, 9);
}

/// `0x313d30(d, far, k, m, P)`: the speed by the distance: 0.06 within `far`, rising to 0.1 at 50 beyond (squared);
/// the walker's top speed the same; the animation speed = J's speed / 0.06 · k.
fn pace(w: &mut World, id: MobyId, d: f32, far: f32, k: f32) {
    let lo = SPEED * 0.06;
    let mut s = lo;
    if far < d {
        let x = (d - far).clamp(0.0, 50.0) / 50.0;
        s = x * x * (SPEED * 0.1 - lo) + lo;
    }
    c::set_pf(w, id, pv::SPEED, s);
    c::set_pf(w, id, pv::J_TOP, s);
    let a = c::pf(w, id, pv::J_SPEED) / lo * k;
    set_anim_speed(w, id, a);
}

/// `0x313988(m, P, goal, &via)`: a line of sight through the graph to `goal`; else to the graph point nearest the
/// outline point nearest `goal`; else the graph point nearest the beast.
fn route(w: &World, id: MobyId, walls: &[usize], goal: V, via: &mut V) {
    let pos = c::pos(w, id);
    let Ok(graph) = usize::try_from(c::pi32(w, id, pv::GRAPH)) else { return };
    if let Some(q) = region::line_of_sight(w, 0.0, walls, graph, pos, goal) {
        *via = q;
        return;
    }
    let outline = path_pts(w, c::pi32(w, id, pv::OUTLINE));
    let gp = path_pts(w, graph as i32);
    if let Some(o) = outline.get(nearest_point(&outline, goal)) {
        if let Some(g2) = gp.get(nearest_point(&gp, *o)) {
            if let Some(q) = region::line_of_sight(w, 0.0, walls, graph, pos, *g2) {
                *via = q;
                return;
            }
        }
    }
    if let Some(q) = gp.get(nearest_point(&gp, pos)) { *via = *q; }
}

/// `0x313af0(arrive, m, P, goal, via)`: beyond `arrive` of `goal` the walk toward `via` (`0x26de80`, J at +0x70); blocked
/// (2) → nudged toward the outline point nearest it (at most the top speed); else turning at Ratchet. Every fourth tick a
/// ledge probe down; on a 0x42d moby with +0xbc 0: pushed half way to it when its +0xbe is set, and its +0xbc = 1.
/// Returns 4 when arrived.
fn walk(w: &mut World, id: MobyId, arrive: f32, goal: V, via: V) -> u32 {
    let pos = c::pos(w, id);
    let mut r = 4;
    if arrive < c::dist2(pos, goal) {
        let mut out = c::pv4(w, id, pv::STEP);
        r = walker::walk_to(w, id, pv::J, via, &mut out);
        c::set_pv4(w, id, pv::STEP, out);
        if r == 2 {
            let outline = path_pts(w, c::pi32(w, id, pv::OUTLINE));
            let p = c::pos(w, id);
            if let Some(q) = outline.get(nearest_point(&outline, p)) {
                let d = c::clamp_len3(c::sub(*q, p), c::pf(w, id, pv::J_TOP));
                c::set_pos(w, id, c::add(p, d));
            }
        }
    } else {
        let h = heading(pos, super::hero_pos(w));
        yaw_spring(w, id, h, DT2 * DEG120, DT2 * DEG40, DT * DEG120);
    }
    if w.counter & 3 == 3 {
        // `0x280e18(0.1, 1, position, m, 0)` (L01 `0x26d1d0`, the ledge probe).
        let p = c::pos(w, id);
        if let Some(o) = w.coll_line(crate::moby_update::services::pv([p[0], p[1], p[2] + 0.1, p[3]]), crate::moby_update::services::pv([p[0], p[1], (p[2] - 1.0).max(0.0), p[3]]), 0x22, Some(id)) {
            if let Some(m) = o.moby.filter(|_| 0 < o.kind) {
                if w.m(m).o_class == 0x42d && w.m(m).cmd == 0 {
                    if w.m(m).pvars.get(0xbe).is_some_and(|&b| b != 0) {
                        let q = w.m(m).position;
                        let mut d = c::sub(q, p);
                        d[2] = 0.0;
                        c::set_pos(w, id, c::add(p, c::scale(d, 0.5)));
                    }
                    w.mm(m).cmd = 1;
                }
            }
        }
    }
    r
}

/// State 7, turning in place (module doc).
fn turn_in_place(w: &mut World, id: MobyId, hero: V, g: &Gates) {
    if c::pu8(w, id, pv::FIXED) == 0 { c::set_pv4(w, id, pv::TURN_AT, hero); }
    let at = c::pv4(w, id, pv::TURN_AT);
    let pos = c::pos(w, id);
    let side = c::sub_rot(heading(pos, at), c::yaw(w, id));
    if !g.spit && wrapped(w, id) {
        if side < 0.0001 {
            if seq_b(w, id) != 3 {
                let tk = w.rng.rand_range(10, 0xd);
                w.anim_blend(id, 3, 5, tk);
            }
        } else if seq_b(w, id) != 0xe {
            let tk = w.rng.rand_range(10, 0xd);
            w.anim_blend(id, 0xe, 5, tk);
        }
    }
    let a = seq_a(w, id);
    if a == 3 || a == 0xe || a == 0xd {
        let k = key(w, id);
        let turning = k < 14.5 || (19.0 < k && k < 35.5) || 40.5 < k;
        if turning {
            let h = heading(c::pos(w, id), at);
            yaw_spring(w, id, h, DT2 * DEG120, DT2 * DEG40, DT * DEG120);
        }
        if side > 0.001 && seq_b(w, id) == 3 {
            let tk = w.rng.rand_range(10, 0xd);
            w.anim_blend(id, 0xe, 5, tk);
        } else if side < 0.001 && seq_b(w, id) == 0xe {
            let tk = w.rng.rand_range(10, 0xd);
            w.anim_blend(id, 3, 5, tk);
        }
    } else {
        c::set_pf(w, id, 0x84, 0.0);
    }
    let h = heading(c::pos(w, id), at);
    if c::diff_rots(c::yaw(w, id), h) < c::pf(w, id, pv::TURN_MAX) * 0.25 {
        c::set_pv4(w, id, pv::STEP, [0.0; 4]);
        let s = c::pu8(w, id, pv::BACK_SEQ);
        blend_r(w, id, s, (0, 2), (0xf, 0x1b));
        let b = c::pu8(w, id, pv::BACK_STATE);
        set_st(w, id, b);
    }
    let a = seq_a(w, id);
    if a == 3 || a == 0xe {
        if passed(w, id, 16.0) { bfx::shake(w, id); }
        if passed(w, id, 37.0) { bfx::shake(w, id); }
    } else if a == 0xd {
        if passed(w, id, 13.0) { bfx::shake(w, id); }
        if passed(w, id, 34.0) { bfx::shake(w, id); }
    }
}

/// State 8, the grab (module doc).
fn grab(w: &mut World, id: MobyId, h_h: f32, g: &Gates) {
    if c::pi32(w, id, pv::GRABBED) < 1 {
        let hero = super::hero_pos(w);
        if turn_check(w, id, hero, 6, 0) { return; }
    }
    c::set_pi32(w, id, pv::GRABBED, 1);
    yaw_spring(w, id, h_h, DT2 * DEG120, DT2 * DEG40, DT * DEG120);
    let mouth = w.joint_point(id, 4);
    let reach = bfx::tongue_step(w, id, mouth);
    if seq_a(w, id) == 6 {
        let hp = super::hero_pos(w);
        c::set_pv4(w, id, pv::HEAD, [hp[0], hp[1], hp[2] + 0.6, hp[3]]);
        let d_h = c::dist2(c::pos(w, id), hp);
        let len = c::pf(w, id, pv::LEN);
        if len < d_h {
            c::set_pi32(w, id, pv::TIMER, 0);
            c::set_pf(w, id, pv::LEN, len + DT * 15.0);
        } else {
            let caught = c::pu8(w, id, pv::HOLD) != 0 || (c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 && reach < 0.6);
            if caught {
                c::set_pu8(w, id, pv::HOLD, 1);
                shake_ammo(w, id, d_h);
            }
        }
    }
    if !g.grab {
        c::set_pi32(w, id, pv::SUB, 0);
        let t15 = w.ticks(0xf);
        c::set_pi16(w, id, pv::GRAB_T, t15 as i16);
        bfx::tongue_release(w, id);
        if hero_held(w) { crate::cinematic::hero_state(w, 0, true); }
        let t70 = w.ticks(0x46);
        c::set_pi32(w, id, pv::TIMER, t70);
        c::set_pu8(w, id, pv::GRAB_OK, 0);
        release_slot(w, id);
        to_idle(w, id);
    }
    if !g.spit { return; }
    let t30 = w.ticks(0x1e);
    c::set_pi16(w, id, pv::GRAB_T, t30 as i16);
    blend_r(w, id, 9, (0, 2), (0x10, 0x19));
    bfx::tongue_release(w, id);
    if hero_held(w) { crate::cinematic::hero_state(w, 0, true); }
    release_slot(w, id);
    set_st(w, id, 2);
}

/// The grab's catch (module doc): Ratchet held (0x78) and carried 2 above, spun 4 turns a second; a pickup of the
/// current weapon's ammo a tick; the list done → released, the grab's timers, → 1.
fn shake_ammo(w: &mut World, id: MobyId, d_h: f32) {
    if !hero_held(w) {
        let hp = super::hero_pos(w);
        c::set_pv4(w, id, pv::A, hp);
        let z = w.m(id).position[2];
        c::set_pv4(w, id, pv::B, [hp[0], hp[1], hp[2].max(z) + 2.0, hp[3]]);
        crate::cinematic::hero_state(w, 0x78, true);
        c::set_pi32(w, id, pv::SUB, 0);
        c::set_pf(w, id, pv::AMMO_AT, 20.0);
        c::set_pi16(w, id, pv::AMMO_EACH, 0);
    }
    let a = c::pv4(w, id, pv::A);
    let p = c::add(a, c::scale(c::sub(c::pv4(w, id, pv::B), a), SPEED * 0.1));
    c::set_pv4(w, id, pv::A, p);
    // 0x141098 (a yaw the hero block keeps) and the yaw 0x13f3e8 both turn 8π·dt; the port turns the yaw [L].
    let yaw = c::add_rot(w.hero.rot[2].to_f32(), DT * 25.132_742);
    w.hero_fields_mut().pose = Some(HeroPose { pos: [p[0], p[1], p[2]], yaw, target_yaw: w.hero.target_yaw.to_f32() });
    c::set_pf(w, id, pv::LEN, d_h);
    let k = (c::pi32(w, id, pv::SUB).max(0) as usize).min(SHAKE_ITEMS.len() - 1);
    let (item, least) = SHAKE_ITEMS[k];
    if item == -1 {
        c::set_pi32(w, id, pv::SUB, 0);
        let t800 = w.ticks(800);
        c::set_pi16(w, id, pv::GRAB_T, t800 as i16);
        let t240 = w.ticks(0xf0);
        c::set_pi32(w, id, pv::TIMER, t240);
        bfx::tongue_release(w, id);
        if hero_held(w) { crate::cinematic::hero_state(w, 0, true); }
        c::set_pu8(w, id, pv::GRAB_OK, 0);
        release_slot(w, id);
        to_idle(w, id);
        return;
    }
    if c::pi16(w, id, pv::AMMO_EACH) == 0 {
        let ammo = crate::moby_update::classes::pickup::item_ammo(w, item as usize);
        let mut each = (ammo / 8) as i16;
        if each < least { each = least; }
        c::set_pi16(w, id, pv::AMMO_EACH, each);
        let at = c::pf(w, id, pv::AMMO_AT);
        let r = w.rng.randf(at + 20.0, at + 100.0);
        c::set_pf(w, id, pv::AMMO_AT, r);
    }
    let ammo = crate::moby_update::classes::pickup::item_ammo(w, item as usize);
    let mut n = c::pi16(w, id, pv::AMMO_EACH) as i32;
    if ammo < n {
        c::set_pi16(w, id, pv::AMMO_EACH, 0);
        c::set_pi32(w, id, pv::SUB, k as i32 + 1);
        n = ammo;
    }
    let rest = ammo - n * 8;
    let amount = if 0 < rest { rest } else { 0 } + n;
    if 0 < amount { drop_ammo(w, id, item, amount); }
}

/// One pickup of `amount` of `item` (`0x2ee3b8`, sound 11): without the ammo path at `rand_vec(4·dt, 10·dt)` (its z at
/// least 2·dt: else `randf(3·dt, 6·dt)`) 0.9 out from Ratchet; along it, from the point nearest him the cursor stepped
/// by the running distance + 10 until the point is 8 from him and 12 from the beast (the distance kept), jittered ±0.5,
/// lobbed from 0.9 toward it at 15·dt (`0x26faf0`), the distance + `randf(0.7, 2)`; the pickup's velocity +0x10 and its
/// landing height +0x28; his ammo less the amount.
fn drop_ammo(w: &mut World, id: MobyId, item: i32, amount: i32) {
    let hero = super::hero_pos(w);
    let path = c::pi32(w, id, pv::AMMO_PATH);
    let (p, v, land);
    if path == -1 {
        let r = w.rng.rand_vec(DT * 4.0, DT * 10.0);
        let mut rv = [r[0], r[1], r[2], 0.0];
        if rv[2] < DT + DT { rv[2] = w.rng.randf(DT * 3.0, DT * 6.0); }
        p = c::add(c::set_len3(rv, f32::from_bits(0x3f66_6666)), hero);
        v = rv;
        land = 0.0;
    } else {
        let pts = path_pts(w, path);
        let mut cur = crate::spline::Cursor { seg: nearest_point(&pts, hero) as i32, t: 0.0 };
        let mut dist = c::pf(w, id, pv::AMMO_AT);
        let mut at;
        loop {
            let step = dist + 10.0;
            c::set_pf(w, id, pv::AMMO_AT, step);
            let (q, _) = crate::spline::advance(&pts, true, step, &mut cur);
            at = [q[0], q[1], q[2], 0.0];
            dist = step;
            if 8.0 <= c::dist2(hero, at) && 12.0 <= c::dist2(c::pos(w, id), at) { break; }
        }
        at[0] += w.rng.randf(-0.5, 0.5);
        at[1] += w.rng.randf(-0.5, 0.5);
        let d = c::set_len3(c::sub(at, hero), f32::from_bits(0x3f66_6666));
        let flat = c::set_len3([d[0], d[1], 0.0, d[3]], DT * 15.0);
        p = c::add(d, hero);
        let mut time = 0.0;
        let up = crate::moby_update::creature::knock::lob_up(DT * 15.0, DT2 * -9.8, p, at, &mut time);
        v = [flat[0], flat[1], up, flat[3]];
        land = at[2];
        let r = w.rng.randf(f32::from_bits(0x3f33_3333), 2.0);
        c::set_pf(w, id, pv::AMMO_AT, c::pf(w, id, pv::AMMO_AT) + r);
    }
    w.play_sound(0xb, 0, id);
    if let Some(m) = crate::moby_update::classes::pickup::pickup_spawn(w, crate::moby_update::services::pv(p), item, amount, 0) {
        if w.m(m).pvars.len() < 0x2c { w.mm(m).pvars.resize(0x2c, 0); }
        c::set_pv4(w, m, 0x10, v);
        c::set_pf(w, m, 0x28, land);
        w.hero_fields_mut().ammo[item as usize] -= amount;
        c::set_pi32(w, id, pv::TIMER, 0);
    }
}

/// State 9, the beam onto a platform (module doc).
fn beam(w: &mut World, id: MobyId) {
    if c::pi32(w, id, pv::SUB) < 1 {
        let l = c::pv4(w, id, pv::LUNGE);
        if turn_check(w, id, l, 7, 1) { return; }
    }
    let l = c::pv4(w, id, pv::LUNGE);
    let h = heading(c::pos(w, id), l);
    yaw_spring(w, id, h, DT2 * DEG120, DT2 * DEG40, DT * DEG120);
    match c::pi32(w, id, pv::SUB) {
        0 => {
            if seq_b(w, id) != 7 {
                let tk = w.rng.rand_range(10, 0xd);
                w.anim_blend(id, 7, 0, tk);
            }
            c::set_pi32(w, id, pv::TIMER, 0);
            c::set_pi32(w, id, pv::SUB, 1);
        }
        1 => {
            if !(seq_a(w, id) == 7 && seq_b(w, id) == 7) && seq_b(w, id) != 7 {
                let tk = w.rng.rand_range(10, 0xd);
                w.anim_blend(id, 7, 0, tk);
            }
            if seq_a(w, id) != 7 || key(w, id) < 9.5 { return; }
            let (a, b) = (w.joint_point(id, 0), w.joint_point(id, 1));
            let s = c::scale(c::add(a, b), 0.5);
            c::set_pv4(w, id, pv::A, s);
            let d = c::set_len3(c::sub(l, s), SPEED);
            c::set_pv4(w, id, pv::AB, d);
            c::set_pi32(w, id, pv::SUB, 2);
        }
        2 => {
            if !(seq_a(w, id) == 7 && seq_b(w, id) == 7) && seq_b(w, id) != 7 {
                let tk = w.rng.rand_range(7, 10);
                w.anim_blend(id, 7, 0, tk);
            }
            if c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 {
                let t10 = w.ticks(10);
                c::set_pi32(w, id, pv::TIMER, t10);
                bfx::part61(w, 800_000.0, 5.0, id, 0);
                bfx::part61(w, 800_000.0, 5.0, id, 1);
            }
            if 31.5 <= key(w, id) {
                c::set_pf(w, id, pv::LEN, 0.0);
                c::set_pi32(w, id, pv::SUB, 3);
                c::set_pf(w, id, pv::A + 0xc, 0.0);
            }
        }
        3 => {
            // The game compares with a stack word left from an earlier call (the beam's point here) [L].
            let d = c::dist2(c::pos(w, id), l);
            let len = c::pf(w, id, pv::LEN) + SPEED;
            c::set_pf(w, id, pv::LEN, len);
            if d <= len { c::set_pi32(w, id, pv::SUB, 4); }
            if c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 {
                let t20 = w.ticks(0x14);
                c::set_pi32(w, id, pv::TIMER, t20);
            }
            bfx::register(w, id, bfx::BEAM_FN);
        }
        4 => {
            let a = c::pv4(w, id, pv::A);
            let d = c::set_len3(c::sub(c::pv4(w, id, pv::B), a), SPEED);
            let a = c::add(a, d);
            c::set_pv4(w, id, pv::A, a);
            let t30 = w.ticks(0x1e);
            bfx::light(w, id, 10.0, 3, Some(a), t30, 0x8000_60ff, 0);
            if c::pi32(w, id, pv::SUB) != 0 && c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 { c::set_pi32(w, id, pv::SUB, 5); }
            bfx::register(w, id, bfx::BEAM_FN);
        }
        5 => {
            set_anim_speed(w, id, 1.0);
            c::set_pi32(w, id, pv::SUB, 0);
            c::set_pi32(w, id, pv::TIMER, 0);
            idle_blend(w, id, (0, 7), (0x1b, 0x2d));
            to_idle(w, id);
        }
        _ => {}
    }
}

/// State 0xc, the step's end (module doc).
fn step_over(w: &mut World, id: MobyId) {
    let sixth = (c::pi16(w, id, pv::D + 4) as i32 / 6) as f32;
    if c::pf(w, id, pv::D) <= sixth {
        if seq_b(w, id) != 8 {
            let tk = w.rng.rand_range(10, 0xd);
            w.anim_blend(id, 8, 0x35, tk);
        }
    } else if seq_b(w, id) != 0xb {
        w.anim_blend(id, 0xb, 0x3d, 1);
    }
    set_st(w, id, 0xd);
    c::set_pf(w, id, pv::STEP + 8, 0.0);
    for _ in 0..2 {
        if (phase(w, id) % 3) < 2 { w.mm(id).cmd += 1; }
    }
    if 8 <= phase(w, id) { w.mm(id).cmd = 8; }
    let arena = c::pu8(w, id, pv::ARENA) as usize;
    let first = c::pi32(w, id, pv::GATES + 4 * arena);
    if first == -1 { return; }
    let mut pick = arena;
    let second = c::pi32(w, id, pv::GATES + 4 * (arena + 3));
    if second != -1 {
        let cub = |w: &World, m: i32| moby_at(w, m).and_then(|m| (w.m(m).pvars.len() >= 0x2c).then(|| c::pi32(w, m, 0x28)));
        if let (Some(ca), Some(cb)) = (cub(w, first), cub(w, second)) {
            if ca != -1 && cb != -1 {
                let hero = super::hero_pos(w);
                let centre = |w: &World, q: i32| w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, q).map_or([0.0; 3], |s| s.centre());
                let (pa, pb) = (centre(w, ca), centre(w, cb));
                let (da, db) = (c::dist2(hero, [pa[0], pa[1], pa[2], 0.0]), c::dist2(hero, [pb[0], pb[1], pb[2], 0.0]));
                let pc = if db < da {
                    pick = arena + 3;
                    pb
                } else {
                    pa
                };
                let p = c::pos(w, id);
                w.mm(id).rotation[2] = c::atan(pc[0] - p[0], pc[1] - p[1]);
            }
        }
    }
    if let Some(m) = moby_at(w, c::pi32(w, id, pv::GATES + 4 * pick)) { w.mm(m).cmd = 1; }
}

/// State 0xd, stunned (module doc).
fn stunned(w: &mut World, id: MobyId) {
    if 10.0 < key(w, id) { drool(w, id); }
    let done = if phase(w, id) == 8 {
        let s = c::scale(c::pv4(w, id, pv::STEP), f32::from_bits(0x3f33_3333));
        let vz = c::pf(w, id, pv::STEP + 8);
        let s = [s[0], s[1], vz - DT * 9.8, s[3]];
        c::set_pv4(w, id, pv::STEP, s);
        let p = c::add(c::pos(w, id), s);
        c::set_pos(w, id, p);
        p[2] < c::pf(w, id, pv::FLOOR)
    } else {
        79.0 < key(w, id)
    };
    if !done { return; }
    let t10 = w.ticks(10);
    c::set_pi32(w, id, pv::TIMER, t10);
    c::set_pu8(w, id, pv::SHIMMER_MODE, 2);
    set_st(w, id, 0xe);
    let sixth = c::pi16(w, id, pv::D + 4) as i32 / 6;
    c::set_pi32(w, id, pv::SUB, sixth);
}

/// The drool: one chance in 26 a type-45 ring at joint 10 on the water (34.6).
fn drool(w: &mut World, id: MobyId) {
    let mut jp = w.joint_point(id, 10);
    jp[2] = 34.6;
    if w.rng.randi(0x1a) == 0 {
        *w.svc.fx.part_spawns.entry(45).or_default() += 1;
        if let Some(sys) = w.particles.as_deref_mut() {
            if crate::particles::type45::spawn45(sys, w.rng, f32::from_bits(0x3fcc_cccd), 14700.0, jp, 0x4010_3080).is_none() { w.svc.fx.part_failed += 1; }
        }
    }
}

/// State 0xe, the drain (module doc).
fn drain(w: &mut World, id: MobyId) {
    let s = c::scale(c::pv4(w, id, pv::STEP), f32::from_bits(0x3ecc_cccd));
    c::set_pv4(w, id, pv::STEP, s);
    let p = c::add(c::pos(w, id), s);
    c::set_pos(w, id, p);
    if key(w, id) < 168.0 { drool(w, id); }
    if passed(w, id, 168.0) {
        let jp = w.joint_point(id, 5);
        bfx::shockwave(w, 1.0, 25.0, 2.0, DT * 8.0, 1.0, id, jp, 0x3a);
    }
    if passed(w, id, 168.5) {
        let jp = w.joint_point(id, 6);
        bfx::shockwave(w, 1.0, 25.0, 2.0, DT * 8.0, 1.0, id, jp, 0x3a);
        bfx::shake(w, id);
    }
    if 0 < c::pi32(w, id, pv::SUB) {
        if c::dec_timer_pvar_i32(w, id, pv::TIMER) == 0 { return; }
        c::set_pi32(w, id, pv::SUB, c::pi32(w, id, pv::SUB) - 1);
        let hp = c::pf(w, id, pv::D) - 1.0;
        c::set_pf(w, id, pv::D, hp);
        c::set_pu8(w, id, pv::FLASH + 7, 0xfa);
        flash::start(w, id, pv::FLASH);
        if 0.0 < hp {
            let n = if c::pi32(w, id, pv::SUB) < 1 { 0x14 } else { 10 };
            let t = w.ticks(n);
            c::set_pi32(w, id, pv::TIMER, t);
            return;
        }
        c::set_pf(w, id, pv::D, 0.0);
        if seq_b(w, id) != 8 {
            let tk = w.rng.rand_range(10, 0xd);
            w.anim_blend(id, 8, 0, tk);
        }
        die(w, id);
        set_st(w, id, 0xa);
        return;
    }
    if c::dec_timer_pvar_i32(w, id, pv::TIMER) == 0 { return; }
    let arena = c::pu8(w, id, pv::ARENA);
    let pts = path_pts(w, c::pi32(w, id, pv::POINTS));
    let mut open = c::pu8(w, id, pv::OPEN);
    let target = match arena {
        0 => {
            open &= 0xfe;
            pts.first().copied()
        }
        1 => {
            open &= 0xfd;
            pts.get(1).copied()
        }
        2 => {
            let hero = super::hero_pos(w);
            let (a, b) = (pts.get(2).copied().unwrap_or_default(), pts.get(3).copied().unwrap_or_default());
            open &= 0xfb;
            Some(if c::dist2(hero, b) < c::dist2(hero, a) { b } else { a })
        }
        _ => None,
    };
    c::set_pu8(w, id, pv::OPEN, open);
    if let Some(q) = target { c::set_pv4(w, id, pv::B, q); }
    let node = c::pu8(w, id, pv::NODES + arena as usize) as i32;
    c::set_pi32(w, id, pv::NODE, node);
    if let Ok(g) = usize::try_from(c::pi32(w, id, pv::GRAPH)) {
        let wl = walls(w, id);
        region::graph_init(w, &wl, g);
    }
    set_st(w, id, 0xf);
}

/// The death's records: the talked word, the death bits, the mission done, the checkpoint (cuboid +0x1f0).
fn die(w: &mut World, id: MobyId) {
    let cp = c::pi32(w, id, pv::CHECKPOINT);
    if cp == -1 { return; }
    crate::moby_update::interact::set_talked(w, id, 1);
    crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
    let mission = w.m(id).mission;
    crate::cinematic::set_mission_done(w, mission);
    crate::moby_update::story::checkpoint_at(w, cp);
}

/// State 0xf, back to the arena (module doc).
fn to_arena(w: &mut World, id: MobyId) {
    if 198.0 < key(w, id) {
        if seq_b(w, id) != 10 { w.anim_blend(id, 10, 0, 0); }
        let jp = w.joint_point(id, 10);
        let b = c::pv4(w, id, pv::B);
        // The game: joint 10's pose matrix (`0x210850`) to Euler (`0x285d50`), its z added to the yaw; here joint
        // list 10's world matrix's heading (the same with the pose upright) [L].
        let r0 = w.joint_matrix(id, 10)[0];
        let yaw = r0[1].atan2(r0[0]);
        c::set_pv4(w, id, pv::TURN_AT, [jp[0], jp[1], b[2], yaw]);
    }
    if seq_a(w, id) != 10 { return; }
    let at = c::pv4(w, id, pv::TURN_AT);
    c::set_pos(w, id, [at[0], at[1], at[2], 1.0]);
    w.mm(id).rotation[2] = at[3];
    let arena = c::pu8(w, id, pv::ARENA) as usize;
    if let Some(m) = moby_at(w, c::pi32(w, id, pv::GATES + 4 * arena)) { w.mm(m).cmd = 0; }
    set_anim_speed(w, id, 1.5);
    w.mm(id).cmd += 1;
    c::set_pi32(w, id, pv::COUNT, 0);
    set_st(w, id, 6);
    let t55 = w.ticks(0x37);
    c::set_pi32(w, id, pv::TIMER, t55);
}

/// `0x313420`: the hits (mask 0x330000). The Suck Cannon's (0xba / 0x79) count 1, the Bomb Glove's (0x131) 0.2; its
/// own shockwaves and globs 0x416 / 0x419 (or no attacker) are ignored. While stunned (the shimmer up or fading) a hit
/// only flashes the shimmer; else out > 1 hurts: health to 0 → dead (the records, → 0xa); else health −= damage, the
/// red flash, the head jerked (the shimmer's drop), the grab allowed once below 1.75 sixths, and crossing a sixth:
/// Ratchet let go, sequence 2, the shimmer in (sound 15), → 0xb, the phase + 1.
fn hits(w: &mut World, id: MobyId) {
    if st(w, id) == 0xa {
        hits_tail(w, id);
        return;
    }
    let h = w.get_hit(id, 0x33_0000, false);
    let rr = damage::resolve(w, id, h, pv::D, 0, 4);
    // The record as the resolver left it (the damage zeroed in a kind's cooldown), then this class's rewrite.
    let mut h = rr.hit;
    if let Some(r) = h.as_mut() {
        if 0.0 < f32::from_bits(r.damage.0) {
            match r.h2a {
                0xba | 0x79 => r.damage = crate::ps2v::Pf::ONE,
                0x131 => r.damage = crate::ps2v::Pf::f(f32::from_bits(0x3e4c_cccd)),
                _ => {}
            }
        }
    }
    let mut out = rr.out5;
    if let Some(r) = h {
        let a = r.attacker.map(|a| w.m(a).o_class);
        if a.is_none() || a == Some(0x419) || a == Some(0x416) { out = 1; }
    }
    if 0.0 < c::pf(w, id, pv::SHIMMER) || c::pu8(w, id, pv::SHIMMER_MODE) != 0 {
        if 2 <= out { c::set_pu8(w, id, pv::SHIMMER_FLASH, 0xff); }
    } else if 2 <= out {
        let dmg = h.map_or(0.0, |r| f32::from_bits(r.damage.0));
        let hp = c::pf(w, id, pv::D);
        if hp <= dmg {
            c::set_pf(w, id, pv::D, 0.0);
            w.mm(id).mode &= !mode::TARGETABLE;
            crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
            c::set_pu8(w, id, pv::FLASH + 7, 0x78);
            flash::start(w, id, pv::FLASH);
            if seq_b(w, id) != 2 {
                let tk = w.rng.rand_range(10, 0xd);
                w.anim_blend(id, 2, 0, tk);
            }
            if hero_held(w) { crate::cinematic::hero_state(w, 0, true); }
            die(w, id);
            set_st(w, id, 0xa);
        } else {
            let max = c::pi16(w, id, pv::D + 4) as f32;
            let sixth = max / 6.0;
            c::set_pf(w, id, pv::SHIMMER_DZ, if dmg + dmg <= 2.0 { -(dmg + dmg) } else { -2.0 });
            let segs = (hp / sixth).floor();
            let mut edge = segs;
            if max - 1.0 < hp { edge = 5.0; }
            let edge = edge * sixth;
            let after = hp - dmg;
            c::set_pf(w, id, pv::D, after);
            c::set_pu8(w, id, pv::FLASH + 7, 0xfa);
            let t0 = w.ticks(0);
            c::set_pi16(w, id, pv::D + 6, t0 as i16);
            flash::start(w, id, pv::FLASH);
            if st(w, id) < 10 && after < edge {
                if hero_held(w) { crate::cinematic::hero_state(w, 0, true); }
                blend_r(w, id, 2, (0, 2), (0x14, 0x16));
                c::set_pf(w, id, pv::SHIMMER, 0.0);
                c::set_pu8(w, id, pv::SHIMMER_MODE, 1);
                release_slot(w, id);
                let s = w.play_sound(0xf, 4, id);
                c::set_pi32(w, id, pv::SLOT, s);
                set_st(w, id, 0xb);
                w.mm(id).cmd += 1;
            }
            let grab_line = sixth * 1.75;
            if (hp > grab_line && after <= grab_line) || (hp > sixth * 1.25 && after <= sixth * 1.25) { c::set_pu8(w, id, pv::GRAB_OK, 1); }
        }
    }
    let f = c::pu8(w, id, pv::SHIMMER_FLASH);
    c::set_pu8(w, id, pv::SHIMMER_FLASH, if f < 0x15 { 0 } else { f - 0x14 });
    w.mm(id).hit_slot = 0xff;
    hits_tail(w, id);
}

/// `0x313420`'s tail: the flash update, the shimmer's drop easing back (+0.3 a tick, at most 0).
fn hits_tail(w: &mut World, id: MobyId) {
    flash::update(w, id, pv::FLASH);
    let d = (c::pf(w, id, pv::SHIMMER_DZ) + 0.3).min(0.0);
    c::set_pf(w, id, pv::SHIMMER_DZ, d);
}
