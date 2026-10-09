//! Gold bolts, class 1134: `GoldBoltUpdate` level01 0x307ca0 with its helpers 0x308380 / 0x308470 / 0x308550 /
//! 0x3087e0 / 0x3089f0: the same 1760-byte function in 17 overlays byte for byte (clusters.tsv a37bacdb) and in
//! Gemlik's (13) up to relocations (`LevelPorts`' matcher), so every level with a gold bolt (all but 00) runs it.
//! Spec: docs/plan/hero_gameplay.md §6. Native `f32`.
//!
//! **Pvars** (0x80 bytes; +0x00..+0x0c from the level data, the rest written at init):
//! +0x00 s32 the gold bolt's index on its level (the byte `0x14bec0 + level·4 + index`, save chunk 3003,
//! `GameState::levels[l].gold_bolts`; −1: none), +0x04 / +0x08 s32 two cuboids the pickup camera moves between
//! (−1: the orbit about Ratchet), +0x0c s16 ≠ 0: no cutaway (collected on the spot), +0x10 vec4 the spawn position
//! (+0x1c = its yaw), +0x20..+0x2c f32 the four glow sprites' angles, +0x30..+0x3c their spins, +0x40..+0x4c s32
//! their colour timers, +0x50..+0x5c f32 their sizes, +0x60 / +0x64 / +0x68 f32 spin rates z / y / x, +0x6c s32 the
//! end timer.
//!
//! **States** (+0x20):
//! * **0** (load pass): collected already (`0x14bec0[level·4 + index]` ≠ 0) or no index → `DeleteMoby` (so a revisit
//!   finds nothing); else the spawn point kept, four `random_angle_radians()` for the Euler and the bob phase
//!   (+0x4c), the glow init 0x308470 (four `randi(255)` angles, spins −1 / −2.25 / 1.25 / 2.5, timers `ticks(63 +
//!   64k)`, sizes 2.5 / 3 / 3 / 2.5) → 1.
//! * **1** (idle): scale = class scale; z = spawn z + 1 + 0.3·sin(phase); Euler x / y / z += 120 / 40 / 20 °/s,
//!   phase += 90 °/s; the glow 0x308550 (four soft type-59 sprites 0.3 behind the bolt from the camera, 0.1 apart,
//!   gold `FastTweenColor(|0.5 − (ticks(255) − timer)/ticks(255)|, 0x4040ffff, 0x1040ffff)`, life 2); 1 in 40 ticks the
//!   glints 0x3089f0 (one type-60 glint 0.5 `0x50408080` drifting `rand_vec(1, 2)·dt`, three 0.3 `0x40408080` with
//!   `rand_vec(0, 1/3)·dt` added, life 60). **Pickup**: Ratchet within 3 (xy) and 2 (z), health left, control mode 0
//!   or 3:
//!   * +0x0c = 0 (every gold bolt on the disc but one on level 16): `FadeToBlack(ticks(10))`, `HeroTeleport(spawn +
//!     2.5·(cos yaw, sin yaw), (0, 0, yaw + π), 0x72, 0)` (Ratchet held, facing it), sound 0 of the class,
//!     `CameraScript(spawn + 1.25·fwd + 4·left + 1 up, (0, 0, yaw − π/2), 1 = snap, 0, 0)` and its targets, the bolt
//!     on Ratchet (his position and Euler), sequence 1, Ratchet's animation 0x82 (cut), the hand item hidden
//!     (0x1413ff), the letterbox (0x15f404), `ticks(60)` at +0x6c → 2;
//!   * else: `ShowBanner(21427)`, the byte set, `memcard_Save`, `DeleteMoby`.
//! * **2** (the cutaway): the camera 0x3087e0 (f = key time / 170 while sequence 1 plays, else 1): between the two
//!   cuboids (position lerp, Euler lerp the short way), or 5 units from Ratchet at his yaw + (40°·f − 20°), 1 up,
//!   looking back at him (yaw + π, pitch −0.17·f); the bolt stays on Ratchet; while sequence 1 plays: the glow at
//!   joint list 0's point, fading over key times 150..160, and the glints (1 in 40, plus 1 in 10); when it wraps:
//!   sequence 0, hidden, four glint bursts. Ratchet's animation wrapping → `SetAnim(ticks(30), 0, 0)` (idle). Once the
//!   bolt is on sequence 0 and the +0x6c timer runs out: `ShowBanner(21427)` ("Gold Bolt Acquired"), the byte set,
//!   the letterbox off, `HeroTeleport(where he is, 0)` (control back), `CameraScript2(0)`, `memcard_Save`,
//!   `DeleteMoby`.
//!
//! **Game state.** The collected byte is [`GameWrite::GoldBolt`] (the in-memory save chunk 3003, applied by the
//! engine after the tick) and the moby loop's mirror [`TalkGame::gold_bolt_bits`] (read by state 0 on a revisit,
//! and counted into [`TalkGame::gold_bolts`], the count the gold-weapon offers' talk condition 6 spends). The save
//! is an [`EngineRequest::Save`] (the game state already holds everything; there is no memory-card writer yet).
//!
//! Ratchet in the Hologuise disguise (body 3, 0x1413f4 = 3) leaves it before the teleport (`FUN_00231450`, the level's
//! leave copy: `crate::hero::bodies::queue_leave`, run before the teleport's calls).
//!
//! The cheat byte 0x15edb5 (only a save can set it: `crate::cheats`) mirrors the bolt: mode |= 0x8000 at the pickup.
//!
//! [`GameWrite::GoldBolt`]: crate::moby_update::interact::GameWrite::GoldBolt
//! [`TalkGame::gold_bolt_bits`]: crate::moby_update::interact::TalkGame::gold_bolt_bits
//! [`TalkGame::gold_bolts`]: crate::moby_update::interact::TalkGame::gold_bolts
//! [`EngineRequest::Save`]: crate::cinematic::EngineRequest::Save

use crate::cinematic;
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::{add_rot, sub_rot};
use crate::moby_update::interact::GameWrite;
use crate::moby_update::services::{pvar as p, HeroCall, World};
use crate::particles::type59;
use std::f32::consts::{FRAC_PI_2, PI};

/// The update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x307ca0;
/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [1134];
/// The overlays whose class table runs this function for 1134 (byte-identical in all but 13, which differs only in
/// relocated addresses; checked by `tests/classes/gold_bolt_infobot_novalis.rs` through `LevelPorts`).
pub const LEVELS: [u32; 18] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18];
/// `ShowBanner(0x53b3, −1)`: "Gold Bolt Acquired".
pub const BANNER_MSG: i32 = 0x53b3;
/// Ratchet's animation while he holds the bolt up (`SetAnim(0, 0x82, 0)`).
pub const HERO_ANIM: u8 = 0x82;
/// `HeroTeleport(…, 0x72, 0)`: the scripted hold.
pub const HOLD: i32 = 0x72;

const DT: f32 = 1.0 / 60.0;
const DEG: f32 = 0.017_453_292;

// .lit 0x161f10.. (gp −0x4cf0..)
/// Scale factor (b310), bob lift (b314), bob amplitude (b318).
const SCALE_K: f32 = 1.0;
const BOB_LIFT: f32 = 1.0;
const BOB_AMP: f32 = 0.3;
/// Spin rates in °/s: Euler x (b320), y (b324), z (b328), bob phase (b32c).
const SPIN_X: f32 = 120.0;
const SPIN_Y: f32 = 40.0;
const SPIN_Z: f32 = 20.0;
const SPIN_PHASE: f32 = 90.0;
/// The glow colours (b33c, b340), blend (b348: soft), texture type (b34c), glint chance (b350).
pub const GLOW_A: u32 = 0x4040_ffff;
pub const GLOW_B: u32 = 0x1040_ffff;
pub const GLOW_TEX: u8 = 0x35;
const GLINT_CHANCE: i32 = 40;
/// Glint drift ranges (× dt): first (b354, b358), the others' extra (b35c, b360); life (b364), colours (b368, b36c),
/// sizes (b370, b374).
const GLINT_V: (f32, f32) = (1.0, 2.0);
const GLINT_V2: (f32, f32) = (0.0, 0.333_333_34);
const GLINT_LIFE: u16 = 60;
const GLINT_RGBA: [u32; 2] = [0x5040_8080, 0x4040_8080];
const GLINT_SIZE: [f32; 2] = [0.5, 0.3];

/// Pvar offsets.
pub mod pv {
    pub const INDEX: usize = 0x00;
    pub const CUBOID_A: usize = 0x04;
    pub const CUBOID_B: usize = 0x08;
    pub const NO_CUTAWAY: usize = 0x0c;
    pub const SPAWN: usize = 0x10;
    pub const YAW: usize = 0x1c;
    pub const GLOW: usize = 0x20;
    pub const RATE_Z: usize = 0x60;
    pub const RATE_Y: usize = 0x64;
    pub const RATE_X: usize = 0x68;
    pub const TIMER: usize = 0x6c;
    pub const LEN: usize = 0x80;
}

/// The glow block (4 sprites): angles +0, spins +0x10, timers +0x20, sizes +0x30 from its base (the gold bolt's at
/// +0x20, the infobot's at +0x10).
pub mod glow {
    pub const ANGLE: usize = 0x00;
    pub const SPIN: usize = 0x10;
    pub const TIMER: usize = 0x20;
    pub const SIZE: usize = 0x30;
    pub const SPINS: [f32; 4] = [-1.0, -2.25, 1.25, 2.5];
    pub const SIZES: [f32; 4] = [2.5, 3.0, 3.0, 2.5];
}

fn f3(v: [f32; 4]) -> [f32; 3] { [v[0], v[1], v[2]] }
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
/// `FastVecNormalize(len, v)`.
fn set_len(a: [f32; 3], l: f32) -> [f32; 3] {
    let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if n == 0.0 { [0.0; 3] } else { a.map(|x| x * (l / n)) }
}

/// `FastDecTimer` on an s32: 1 when already 0; else `t = max(t, 1) − 1`, 2 when that is ≤ 0; 0 while running.
pub fn fast_dec_timer(t: &mut i32) -> i32 {
    if *t == 0 { return 1; }
    *t = (*t).max(1) - 1;
    if *t < 1 { 2 } else { 0 }
}

/// The glow init (0x308470 / the infobot's 0x2fcd88): four `randi(255)` angles, the spins, `ticks(63 + 64k)` timers,
/// the sizes.
pub fn glow_init(w: &mut World, id: MobyId, base: usize) { glow_init_with(w, id, base, glow::SPINS, glow::SIZES) }

/// [`glow_init`] with a class's own spins and sizes (the same code with other constants: the orb holders 1038's
/// `0x2f7548` on level 06, `units::orb_holder`).
pub fn glow_init_with(w: &mut World, id: MobyId, base: usize, spins: [f32; 4], sizes: [f32; 4]) {
    let a: [f32; 4] = std::array::from_fn(|_| w.rng.randi(0xff) as f32);
    let t: [i32; 4] = std::array::from_fn(|k| w.ticks(0x3f + 0x40 * k as i32));
    let pv_ = &mut w.mm(id).pvars;
    for k in 0..4 {
        p::set_ff(pv_, base + glow::ANGLE + 4 * k, a[k]);
        p::set_ff(pv_, base + glow::SPIN + 4 * k, spins[k]);
        p::set_i32(pv_, base + glow::TIMER + 4 * k, t[k]);
        p::set_ff(pv_, base + glow::SIZE + 4 * k, sizes[k]);
    }
}

/// The item glow (0x308550 / the infobot's 0x2fce68): four soft type-59 sprites stacked from 0.3 behind `centre`
/// (away from the camera) 0.1 apart, each turning by its spin and pulsing between [`GLOW_A`] and [`GLOW_B`] on its
/// `ticks(255)` timer, size × `fade`, life 2. `spawn` false: the angles and timers advance, no sprite (the infobot
/// in game mode 2).
pub fn item_glow(w: &mut World, id: MobyId, base: usize, centre: [f32; 3], fade: f32, spawn: bool) { item_glow_with(w, id, base, centre, fade, spawn, (GLOW_A, GLOW_B)) }

/// [`item_glow`] in a class's own colours (the orb holders 1038's `0x2f7628` on level 06: `units::orb_holder`).
pub fn item_glow_with(w: &mut World, id: MobyId, base: usize, centre: [f32; 3], fade: f32, spawn: bool, colours: (u32, u32)) {
    let away = set_len(sub(f3(w.camera.map(|x| f32::from_bits(x.0))), centre), -0.3);
    let step = set_len(away, 0.1);
    let mut at = add(away, centre);
    let t255 = w.ticks(0xff);
    let hero = crate::hero::physics::to_f32x3(w.hero.pos);
    for k in 0..4 {
        let (angle, size, timer) = {
            let pv_ = &mut w.mm(id).pvars;
            let mut a = p::ff(pv_, base + glow::ANGLE + 4 * k) + p::ff(pv_, base + glow::SPIN + 4 * k);
            if 255.0 <= a { a -= 255.0; } else if a <= 0.0 { a += 255.0; }
            p::set_ff(pv_, base + glow::ANGLE + 4 * k, a);
            let mut t = p::i32(pv_, base + glow::TIMER + 4 * k);
            if fast_dec_timer(&mut t) != 0 { t = t255; }
            p::set_i32(pv_, base + glow::TIMER + 4 * k, t);
            (a, p::ff(pv_, base + glow::SIZE + 4 * k), t)
        };
        let f = (0.5 - (t255 - timer) as f32 / t255 as f32).abs();
        let rgba = crate::particles::tween_color(f.to_bits(), colours.0, colours.1);
        if spawn {
            if let Some(s) = w.particles.as_deref_mut() {
                s.hero = hero;
                type59::spawn(s, size * fade, [at[0], at[1], at[2], 0.0], rgba, angle as i32 as u8, GLOW_TEX, true, 2, 0);
                *w.svc.fx.part_spawns.entry(type59::TYPE).or_default() += 1;
            }
        }
        at = add(at, step);
    }
}

/// The glints 0x3089f0 at `at`: one 0.5 glint drifting `rand_vec(1, 2)·dt`, then three 0.3 ones, each adding
/// `rand_vec(0, 1/3)·dt` to the drift; a `randi(255)` rotation each.
fn glints(w: &mut World, at: [f32; 3]) {
    let q = [at[0], at[1], at[2], 0.0];
    let mut v = w.rng.rand_vec(GLINT_V.0 * DT, GLINT_V.1 * DT);
    let r = w.rng.randi(0xff) as u8;
    w.part60(GLINT_SIZE[0], q, [v[0], v[1], v[2], 0.0], GLINT_RGBA[0], GLINT_LIFE, r, 0);
    for _ in 0..3 {
        let d = w.rng.rand_vec(GLINT_V2.0 * DT, GLINT_V2.1 * DT);
        v = add(v, d);
        let r = w.rng.randi(0xff) as u8;
        w.part60(GLINT_SIZE[1], q, [v[0], v[1], v[2], 0.0], GLINT_RGBA[1], GLINT_LIFE, r, 0);
    }
}

/// The bounding sphere's centre (`vec_scale(1/1024, moby+0x00)`).
fn bsphere_centre(w: &World, id: MobyId) -> [f32; 3] { let b = w.m(id).bsphere; [b[0], b[1], b[2]].map(|x| x * (1.0 / 1024.0)) }

/// The collected byte of gold bolt `index` on the current level, as the moby loop sees the save (the mirror).
pub fn collected(w: &World, index: i32) -> bool {
    let (Ok(l), Ok(i)) = (usize::try_from(w.svc.level), usize::try_from(index)) else { return false };
    w.svc.interact.game.gold_bolt_bits.get(l).and_then(|b| b.get(i)).is_some_and(|&b| b != 0)
}

/// `*(0x14bec0 + level·4 + index) = 1`: the mirror now, the game state after the tick.
fn set_collected(w: &mut World, index: i32) {
    let (Ok(l), Ok(i)) = (usize::try_from(w.svc.level), usize::try_from(index)) else { return };
    let g = &mut w.svc.interact.game;
    if g.gold_bolt_bits.len() <= l { g.gold_bolt_bits.resize(l + 1, [0; 4]); }
    if let Some(b) = g.gold_bolt_bits[l].get_mut(i) {
        if *b == 0 { g.gold_bolts += 1; }
        *b = 1;
    }
    w.svc.interact.writes.push(GameWrite::GoldBolt { level: l, index: i });
}

/// The pickup's banner, byte and save (both paths).
fn award(w: &mut World, id: MobyId) {
    let t = w.ticks(0xb4);
    cinematic::show_banner(w, BANNER_MSG, t);
    let index = p::i32(&w.m(id).pvars, pv::INDEX);
    set_collected(w, index);
}

/// `GoldBoltUpdate` (0x307ca0).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::LEN { w.mm(id).pvars.resize(pv::LEN, 0); }
    match w.m(id).state {
        0 => init(w, id),
        1 => idle(w, id),
        2 => cutaway(w, id),
        _ => {}
    }
}

fn init(w: &mut World, id: MobyId) {
    let index = p::i32(&w.m(id).pvars, pv::INDEX);
    if index == -1 || collected(w, index) {
        w.delete_moby(id);
        return;
    }
    let m = w.m(id);
    let (pos, yaw) = (m.position, m.rotation[2]);
    {
        let pv_ = &mut w.mm(id).pvars;
        p::set_v4f(pv_, pv::SPAWN, [pos[0], pos[1], pos[2], pos[3]]);
        p::set_ff(pv_, pv::YAW, yaw);
    }
    let e: [f32; 4] = std::array::from_fn(|_| w.rng.rand_angle());
    w.mm(id).rotation = e;
    {
        let pv_ = &mut w.mm(id).pvars;
        p::set_ff(pv_, pv::RATE_X, SPIN_X * DEG * DT);
        p::set_ff(pv_, pv::RATE_Z, SPIN_Z * DEG * DT);
        p::set_ff(pv_, pv::RATE_Y, SPIN_Y * DEG * DT);
    }
    w.mm(id).state = 1;
    glow_init(w, id, pv::GLOW);
}

fn class_scale(w: &World, id: MobyId) -> f32 { f32::from_bits(w.class_scale(w.m(id).o_class).0) * SCALE_K }

fn idle(w: &mut World, id: MobyId) {
    let scale = class_scale(w, id);
    {
        let spawn_z = p::ff(&w.m(id).pvars, pv::SPAWN + 8);
        let m = w.mm(id);
        m.scale = scale;
        m.position[2] = spawn_z + BOB_LIFT + BOB_AMP * m.rotation[3].sin();
        m.rotation[0] = add_rot(m.rotation[0], SPIN_X * DEG * DT);
        m.rotation[1] = add_rot(m.rotation[1], SPIN_Y * DEG * DT);
        m.rotation[2] = add_rot(m.rotation[2], SPIN_Z * DEG * DT);
        m.rotation[3] = add_rot(m.rotation[3], SPIN_PHASE * DEG * DT);
    }
    let c = bsphere_centre(w, id);
    item_glow(w, id, pv::GLOW, c, 1.0, true);
    if w.rng.randi(GLINT_CHANCE) == 0 { glints(w, bsphere_centre(w, id)); }
    // The pickup: Ratchet within 3 (xy) and 2 (z), health left, on foot (0) or in control mode 3.
    let hero = crate::hero::physics::to_f32x3(w.hero.pos);
    let pos = f3(w.m(id).position);
    if ((pos[0] - hero[0]).powi(2) + (pos[1] - hero[1]).powi(2)).sqrt() >= 3.0 { return; }
    if (pos[2] - hero[2]).abs() >= 2.0 { return; }
    if w.hero.health == 0 { return; }
    let hmode = w.body();
    if hmode != 0 && hmode != 3 { return; }
    if p::i16(&w.m(id).pvars, pv::NO_CUTAWAY) != 0 {
        award(w, id);
        cinematic::save(w);
        w.delete_moby(id);
        return;
    }
    start_cutaway(w, id);
}

fn start_cutaway(w: &mut World, id: MobyId) {
    let (spawn, yaw) = {
        let pv_ = &w.m(id).pvars;
        (f3(p::v4f(pv_, pv::SPAWN)), p::ff(pv_, pv::YAW))
    };
    // Ratchet 2.5 in front of the bolt's spot, facing it.
    let at = add([yaw.cos() * 2.5, yaw.sin() * 2.5, 0.0], spawn);
    let face = [0.0, 0.0, add_rot(yaw, PI)];
    let fade = w.ticks(10);
    cinematic::fade_to_black(w, fade);
    // Ratchet in the Hologuise (body 3): out of it first (the level's leave copy, `FUN_00231450` on level 01).
    if w.body() == crate::hero::bodies::body::DISGUISE { crate::hero::bodies::queue_leave(w); }
    cinematic::hero_teleport(w, at, face, HOLD, false);
    w.play_sound(0, 0, id);
    // The camera: 1.25 ahead of the spot, 4 to its left, 1 up, looking across (yaw − π/2); snap (mode 1).
    let side = add_rot(yaw, FRAC_PI_2);
    let mut cam = add([yaw.cos() * 1.25 + side.cos() * 4.0, yaw.sin() * 1.25 + side.sin() * 4.0, 0.0], spawn);
    cam[2] += 1.0;
    let ce = [0.0, 0.0, sub_rot(yaw, FRAC_PI_2)];
    cinematic::camera_script(w, cam, ce, 1, 0, false);
    cinematic::camera_targets(w, Some(cam), Some(ce));
    // The bolt on Ratchet (0x13f3d0 / 0x13f3e0 as HeroTeleport just wrote them).
    let rw = w.hero.rot[3].to_f32();
    {
        let m = w.mm(id);
        m.position = [at[0], at[1], at[2], m.position[3]];
        m.rotation = [face[0], face[1], face[2], rw];
    }
    if w.m(id).anim.seq_b != 1 { w.anim_blend(id, 1, 0, 0); }
    w.hero_fields_mut().call(HeroCall::SetAnim { blend: 0.0, seq: HERO_ANIM, frame: 0 });
    // The mirrored-animation cheat 0x15edb5 (`crate::cheats`): the bolt mirrored too (mode |= 0x8000).
    if w.svc.cheats.on(crate::cheats::slot::MIRROR_ANIM) { w.mm(id).mode |= 0x8000; }
    w.hero_fields_mut().hide_hand = true;
    cinematic::letterbox(w, true);
    w.mm(id).state = 2;
    let t = w.ticks(0x3c);
    p::set_i32(&mut w.mm(id).pvars, pv::TIMER, t);
}

fn cuboid(w: &World, i: i32) -> Option<([f32; 3], [f32; 3])> {
    let s = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, i)?;
    Some((s.centre(), s.euler))
}

/// The cutaway camera 0x3087e0 (module doc).
fn cutaway_camera(w: &mut World, id: MobyId) {
    let f = if w.m(id).anim.seq_a == 1 { crate::moby_update::creature::ground::key_time(w, id) } else { 170.0 } / 170.0;
    let (a, b) = (p::i32(&w.m(id).pvars, pv::CUBOID_A), p::i32(&w.m(id).pvars, pv::CUBOID_B));
    let path = if a != -1 && b != -1 { cuboid(w, a).zip(cuboid(w, b)) } else { None };
    let (pos, euler) = match path {
        Some(((pa, ea), (pb, eb))) => {
            let pos = std::array::from_fn(|k| pa[k] + (pb[k] - pa[k]) * f);
            let euler = std::array::from_fn(|k| add_rot(sub_rot(eb[k], ea[k]) * f, ea[k]));
            (pos, euler)
        }
        None => {
            let hero = crate::hero::physics::to_f32x3(w.hero.pos);
            let ang = add_rot(f * 0.698_131_7 - 0.349_065_84, w.hero.rot[2].to_f32());
            let pos = [ang.cos() * 5.0 + hero[0], ang.sin() * 5.0 + hero[1], hero[2] + 1.0];
            (pos, [0.0, f * -0.17, add_rot(ang, f32::from_bits(0x4049_0fd0))])
        }
    };
    cinematic::camera_targets(w, Some(pos), Some(euler));
}

fn cutaway(w: &mut World, id: MobyId) {
    cutaway_camera(w, id);
    let (hpos, hrot) = (w.hero.pos.map(|x| x.to_f32()), w.hero.rot.map(|x| x.to_f32()));
    {
        let m = w.mm(id);
        m.position = hpos;
        m.rotation = hrot;
    }
    if w.m(id).anim.seq_a == 1 {
        // 0x308380: the scale, the glow at the held point (fading out over key times 150..160), the glints.
        w.mm(id).scale = class_scale(w, id);
        let c = f3(w.joint_point(id, 0));
        let fade = (((160.0 - crate::moby_update::creature::ground::key_time(w, id)) / 160.0) * 16.0).min(1.0);
        item_glow(w, id, pv::GLOW, c, fade, true);
        if w.rng.randi(GLINT_CHANCE) == 0 { glints(w, f3(w.joint_point(id, 0))); }
        if w.rng.randi(10) == 0 { glints(w, f3(w.joint_point(id, 0))); }
        if w.m(id).anim.flags & 2 != 0 {
            if w.m(id).anim.seq_b != 0 { w.anim_blend(id, 0, 0, 0); }
            let m = w.mm(id);
            m.visible = 0;
            m.mode |= mode::HIDDEN;
            for _ in 0..4 { glints(w, f3(w.joint_point(id, 0))); }
        }
    }
    if w.hero.loop_in.anim.flags & 2 != 0 {
        let b = w.ticks(0x1e) as f32;
        w.hero_fields_mut().call(HeroCall::SetAnim { blend: b, seq: 0, frame: 0 });
    }
    if w.m(id).anim.seq_a != 0 { return; }
    let mut t = p::i32(&w.m(id).pvars, pv::TIMER);
    let done = fast_dec_timer(&mut t);
    p::set_i32(&mut w.mm(id).pvars, pv::TIMER, t);
    if done == 0 { return; }
    award(w, id);
    cinematic::letterbox(w, false);
    let (pos, rot) = (crate::hero::physics::to_f32x3(w.hero.pos), crate::hero::physics::to_f32x3(w.hero.rot));
    cinematic::hero_teleport(w, pos, rot, 0, false);
    cinematic::camera_script2(w, 0);
    cinematic::save(w);
    w.delete_moby(id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_counts_down_like_fast_dec_timer() {
        let mut t = 3;
        assert_eq!([fast_dec_timer(&mut t), fast_dec_timer(&mut t), fast_dec_timer(&mut t), fast_dec_timer(&mut t)], [0, 0, 2, 1]);
        assert_eq!(t, 0);
    }

    #[test]
    fn levels_are_all_but_veldin() {
        assert!(!LEVELS.contains(&0));
        assert_eq!(LEVELS.len(), 18);
    }
}
