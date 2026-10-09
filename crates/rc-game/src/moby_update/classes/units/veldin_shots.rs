//! The boss 1422's shots and effects on level 18 (the names are descriptive [L]); each is a class of its own, created
//! by the boss through a spawner that sits in the class's code:
//!
//! * **564, the lobbed shell** (level18 `0x2d5050`, its spawner `0x2d5348`, its trail `0x2d5488`, its blast
//!   `0x2d5610`, its target marker `0x2d5768`): flies a curve from the boss to a point (the velocity eased from the
//!   launch direction to the straight line, turned by a yaw / pitch offset), leaves smoke and a spark, and bursts on
//!   the ground or Giant Clank (a beam explosion that hurts within 1.5, and with Giant Clank a 3-damage sphere hit).
//! * **624, the ring shell** (`0x2db460`, its spawner `0x2db938`, the ring's hits `0x2dbe80`, its sparks `0x2dc0c8`, its
//!   draw `0x2dba20`): falls, lands, then a ring of 64 segments grows to 64 hurting whatever crosses it (3 damage).
//! * **628, the aura** (`0x2dc260`, its spawner `0x2dc3e0`, its follow `0x2dc458`, its draw `0x2dc4b8`): a 5.8 sphere
//!   round the charging boss that pushes and hurts Ratchet (1 damage), alive while the boss keeps it.
//! * **983, the beam** (`0x2e9e70`, its spawner `0x2ea0f0`, its aim / fire `0x2ea168`, its sparks `0x2ea598`, its
//!   impact `0x2ea800`, its strands `0x2eacd8` / `0x2eaea0`, its draws `0x2ea1f8` / `0x2eb9f8`): held at the boss's
//!   joint while it charges, then fired at a point at 70 a second hurting (5) along the way and bursting there.
//! * **1898, the shock flash** (`0x2fb548`, its spawner `0x2fb5e0`, its draw `0x2fb690`): the beam impact's flash.
//!
//! The level18 words: gp−0x52a0 5.0 (564's scale factor), gp−0x529c 60 (its marker's last ticks), gp−0x5294 3.0 (its
//! Giant Clank damage), gp−0x5290..−0x5284 the marker's ALPHA (0, 1, 0, 1 = 0x44) and colours 0x800020f0 / 0x8000f0f0;
//! gp−0x500c.. 624's (below), gp−0x4d30.. 983's (below).
//!
//! ## Coverage
//!
//! **564** `0x2d5050` (Ratchet in 0x72 or game mode 2 → deleted; only state 0 runs):
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | | f = min(+0x2c / +0x30, 1); d = (+0x10 − position)/(+0x34 − +0x2c); d turned into the frame of its yaw (Rz(atan d)ᵀ), by Rx·Ry·Rz(0, +0x28, +0x24) and back; v = lerp(+0x00, d, f) | the curve | [`lob_update`] |
//! | | `0x2d5488(m, v)`; position += v; yaw / pitch from v; rot.x += 6π·dt; +0x2c + 1 | | [`lob_update`] |
//! | | `CollLine_Fix(old, new, 0, m)`: a moby hit counts only for 0x1df (+0x38 &= 4) and Giant Clank 0x1a3 | the hit | [`lob_update`] (`World::coll_line`) |
//! | | not hit and not at +0x34: +0x38 & 2 and the last 60 ticks → `RegisterDrawCallback(0x2d5768)` | the target marker | [`lob_update`] (`Callback::UnitQuads`, [`lob_quads`]) |
//! | | else `0x2d5610`, `DeleteMoby` | | [`lob_update`] |
//! | `0x2d5348(yaw, pitch, from, dir, at, n, k)` | `CreateMoby(0x234)`: drawn, state 0, update / draw distance 0xff, ambient 0xc0c0c0 (`0x2525d8`), scale ·5; position `from`; +0x00 dir, +0x10 at, +0x38 k, +0x30 n/2, +0x24 yaw, +0x28 pitch, +0x2c 0, +0x34 n; yaw / pitch from dir; `PlayClassSound(0, 0)` | the spawner | [`lob`] |
//! | `0x2d5488` | `rand_vec(0.1·dt, 0.2·dt)`; a point `randf(0, 1)` of the way along v; `PartType44Spawn(40000, 1000, 1, −0.0002, 0, …, ticks(20), 0x7f, 0x606060, 3)` with the record's def / blend patched (23, 0x44); `PartType21Spawn(20000, p, row 2·0.02, 0x4f007fff, 0x1fffffff, ticks(20), 1)` | the trail | [`trail`] |
//! | `0x2d5610` | `SpawnBeamExplosion(+0x38 & 4 ? 0 : 1.5, 1, 2 / 4, 1 / 2, 9, 0.75 / 2, 5, …, 20, 6, 32, sound 1)` (big with +0x38 & 1); hurting and Giant Clank (0x1413f4 = 2): `0x25bbf0(2, 3, 1, m, pos, 2, 0, 1, 0)` | the blast | [`burst`] |
//! | `0x2d5768` | one quad at the target: corners (±1, ±1, 0.1), ST (1, 0), (1, 1), (0, 0), (0, 1), FX 0xd, ALPHA 0x44, colour `FastTweenColor((sin(2π·(+0x2c & 15)/16) + 1)/2, 0x800020f0, 0x8000f0f0)` | | [`lob_quads`] |
//!
//! **624** `0x2db460` (Ratchet in 0x72 or game mode 2 → deleted):
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | 0 | one spark (`PartType02Spawn`: v1 = vel·0.5 + `rand_vec(0, dt)`, at `rand_vec(0, 0.5)` off, sizes `randf(0.75, 0.5)` / `randf(0.333, 0.25)`, colours `tween(0x804080ff, 0x8040ffff)` / `(0x80104040, 0x80002080)`, phases `1·r + 1`, `30·(randf(±0.5) + 1)`, `10·(…)`, v2.z −= 4·dt²·t1, def 0x10019) | the shell's sparks | [`ring_update`] (`fx::part02`) |
//! | | phase += 6π·dt, +0x90 = `tween((sin + 1)/2, 0x80c0c0c0, 0x802020c0)`; vel.z −= 10·dt²; position += vel; landed (z − 0.333 < ground) → 1, z = ground + 0.333, +0x2c = `ticks(1)`; rot.y += 500°·dt; z < 10 → deleted; else the blob shadow (`0x25c328`) | the fall | [`ring_update`] (`crate::shadows::blob`) |
//! | 1 | the glow; `FastDecTimer(+0x2c)` out → `PlayClassSound(0xf, 0)` as class 0x58e (`0x28d738`), 2, radius 0.5, collision off, hidden | | [`ring_update`] |
//! | 2 | radius += +0x20; `0x2dbe80`; `RegisterDrawCallback(0x2dba20)`; radius > +0x28 → 3 | the ring | [`ring_update`] (`Callback::UnitQuads`) |
//! | 3 | `DeleteMoby` | | [`ring_update`] |
//! | `0x2dbe80` | template `0x25bbc8(3, …, m, 0x10001, d)` (d = unit(Ratchet − position)·4·dt xy, z 4·dt); 64 segments of the circle (from −π, 2π/64) 0.5 up: `CollLine_Fix(a, b, 1, the boss, tmpl)`; radius < 3 → `0x2dc0c8` | the hits | [`ring_hits`] (`services::line_hit_in`) |
//! | `0x2dc0c8` | the segment's midpoint in view (`FastBSphereCheck(48, ·)`), `randf(0, 1)` < 0.25: a point along it, its glint (`PartType60Spawn(2, p, unit(p − centre)·+0x20, tween(0x80100080, 0x80108080), ticks(45), randi(255), 0)`) and a flat one (z 0, `ticks(120)`) | | [`ring_sparks`] (`World::part60`) |
//! | `0x2db938(r, k, from, dir, boss)` | `CreateMoby(0x270)`: update 0xff, drawn, state 0, draw 0xff, mode \| 0x10, scale ·1, glow 0x80c0c0c0; position; yaw atan(dir); +0x00 dir, +0x34 boss, +0x28 r, +0x20 k, +0x30 0 | the spawner | [`ring_new`] |
//! | `0x2dba20` | the ring's bands: 36 copies of four additive FX 0xe quads | [`ring_quads`] ([`band_quads`]) |
//!
//! **628** `0x2dc260`:
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | | `FastDecTimer(+0x08)` out → `DeleteMoby` | | [`aura_update`] |
//! | | Ratchet within +0x00 + 0.5 (xy): template `0x25bbc8(1, …, m, 0x10001, d)` (d as 624's), delivered to Ratchet's moby (`0x25bdc8`) | the push | [`aura_update`] (`World::deliver_hit`) |
//! | | +0x04 += 2·dt; frame & 4 = 0 → `0x25bbf0(+0x00, 1, 1, the boss, position, 0x10000, 0, 1, 0)`; `RegisterDrawCallback(0x2dc4b8)` | the hurt | [`aura_update`] (`attack::sphere_hit`; the draw `Callback::UnitQuads`, [`aura_quads`]) |
//! | `0x2dc3e0(r, boss, p)` | `CreateMoby(0x274)`: update 0xff; position; +0x0c boss, +0x00 r, +0x08 3 | | [`aura_new`] |
//! | `0x2dc458(r, aura, p)` | position; +0x00 r; +0x08 += 2, at most `ticks(20)` | | [`aura_follow`] |
//!
//! **983** `0x2e9e70` (every tick: +0x2c += 720°·dt, +0x30 −= 540°·dt):
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | 0 | `FastDecTimer(+0x28)` out → `DeleteMoby` (no aim this tick) | the charge | [`beam_update`] |
//! | 1 | d = +0x00 − position; `0x2ea598`; +0x10 = position + `rand_vec(2·+0x24, 2·+0x24)`; `0x2eacd8`, `0x2eaea0`, `RegisterDrawCallback2(0x2eb9f8)` | the flight | [`beam_update`] |
//! | `0x2eacd8` / `0x2eaea0` / `0x2eb9f8` | the strands: the set-up (once per level load; also 0x1620d8 = `ticks(90)` and gp−0x4b24 = 0, read by nothing), the sim and the draw of the Tesla Claw's chain as a level effect, from the beam's position onto +0x10 (the glow at the beam: 0x307f7f7f, then the strands' colour with alpha 0) | the lightning around the beam | `super::tesla_bolt` ([`strands_frame`], [`strand_quads`]) |
//! | | |d| < 70·dt → `0x2ea800`, deleted; else position += d at 70·dt; two `0x25bbf0(2, 5, 1, the boss, ·, 0x10001, 0, 1, 0)` (at it and on the ground below); `FastDecTimer(+0x34)` out → `Approach(0, 0.2, &+0x24)`, 0 → deleted | | [`beam_update`] |
//! | | `RegisterDrawCallback(0x2ea1f8)` | the core's draw: four additive camera-facing quads (FX 8 / 11 / 20 / 20) | [`core_quads`] (`Callback::UnitQuads`) |
//! | `0x2ea598` | 3 sparks: v1 = `rand_vec(6·dt, 12·dt)`, v2 = v1·0.1, v2.z −= `randf(2·dt, 4·dt)`; phases `randf(1, 5)`, `randf(20, 30)`, `randf(30, 45)` (scaled, truncated); sizes `randf(0.5, 0.333)`, `randf(0.333, 0.2)`; at `randf(0, 1)` of d (60·dt long); colours `tween(0x80807060, 0x80807000)` / `(0x80805020, 0x80802010)`; def 53 | | [`beam_sparks`] |
//! | `0x2ea800` | the flash 1898 (`0x2fb5e0(pos, ticks(30), 0x80806060, 0x801010)`); `PlayClassSound(0x10, 0)` as 0x58e; 100 sparks (`polar(randf(10, 30)·dt, rand_angle, randf(±20°))`, v2 = v1·0.02, v2.z += 2·`randf(0.8, 1.2)`·dt, sizes 0.25, colours 0x60c08010 / 0x50c08010, `ticks(30 / 30 / 240)·randf(0.8, 1.2)`, def 53); 50 TNT sparks (`PartType11Spawn(1.2e6, randf(20, 50)·dt, …)`, `polar(·, rand_angle, randf(60°, 80°))`, at `randf(0, 10)` round, colours of the tables 0x1d9c70 / 0x1d9c88, `ticks(rand_range(30, 60))` ×2); three flashes (`FlashSpawn(10 / 15 / 25, …, ticks(30), 0x10, 0x40, 0xc0, 0x40 / 0x20)`); the camera shake 0.35 for `ticks(45)` | the impact | [`impact`] |
//! | `0x2ea0f0(boss, p, n)` | `CreateMoby(0x3d7)`: update 0xff; position; +0x20 boss, +0x28 5, +0x34 n, +0x24 0 | | [`beam_new`] |
//! | `0x2ea168(t, beam, p, at, fire)` | position = p; +0x28 = `ticks(5)`; +0x24 = t; fire and state 0 → 1, +0x28 = 0, +0x00 = at | | [`beam_aim`] |
//!
//! **1898** `0x2fb548`: 0 → deleted; 1: `RegisterDrawCallback(0x2fb690)`, +0x00 += +0x04, past 2 → deleted. `0x2fb5e0(p, n,
//! c1, c2)`: `CreateMoby(0x76a)`: position, +0x0c c2, +0x08 c1, +0x04 = 2/n, state 1, update 0xff. [`flash_update`],
//! [`flash_new`] (the draw `0x2fb690`: [`flash_sprite`], drawn by `rc-engine` marker_render).

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, attack, fx, ground, turn, DT, DT2};
use crate::moby_update::services::{self as sv, HitTemplate, World};
use crate::particles::type02::Spawn;
use crate::ps2v::Pf;

use super::{FxQuad, FxQuads};

pub const REFERENCE_LEVEL: u32 = 18;
pub const LOB_FN: u32 = 0x2d_5050;
pub const LOB_MARK_FN: u32 = 0x2d_5768;
pub const RING_FN: u32 = 0x2d_b460;
pub const AURA_FN: u32 = 0x2d_c260;
pub const AURA_DRAW_FN: u32 = 0x2d_c4b8;
pub const BEAM_FN: u32 = 0x2e_9e70;
pub const FLASH_FN: u32 = 0x2f_b548;
/// 983's core draw `0x2ea1f8` (`RegisterDrawCallback`, list 1).
pub const CORE_DRAW_FN: u32 = 0x2e_a1f8;
/// 983's strands' draw `0x2eb9f8` (`RegisterDrawCallback2`, list 2).
pub const STRAND_DRAW_FN: u32 = 0x2e_b9f8;
pub const LOB: i16 = 0x234;
pub const RING: i16 = 0x270;
pub const AURA: i16 = 0x274;
pub const BEAM: i16 = 0x3d7;
pub const FLASH: i16 = 0x76a;
pub const LOB_CLASSES: [i16; 1] = [LOB];
pub const RING_CLASSES: [i16; 1] = [RING];
pub const AURA_CLASSES: [i16; 1] = [AURA];
pub const BEAM_CLASSES: [i16; 1] = [BEAM];
pub const FLASH_CLASSES: [i16; 1] = [FLASH];
/// The sound class the ring and the beam play from.
pub const SOUND_CLASS: i16 = 0x58e;
const DEG: f32 = 0.017_453_292;

fn scaled(w: &World, x: f32) -> i32 { (x * f32::from_bits(w.svc.timing.timer_scale.0)) as i32 }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> {
    usize::try_from(c::pi32(w, id, o) - 1).ok().filter(|&m| w.table.mobys.get(m).is_some_and(|x| x.state < 0xfd))
}
fn held(w: &World) -> bool { w.hero.state == 0x72 || w.svc.game_mode == 2 }
fn tween(w: &mut World, a: u32, b: u32) -> u32 { let t = w.rng.randf(0.0, 1.0); crate::hud::tween_color(t, a, b) }
fn v3(v: c::V) -> [f32; 3] { [v[0], v[1], v[2]] }
fn rotate(rows: &[[u32; 4]], v: c::V) -> c::V {
    let r = |k: usize| rows[k].map(f32::from_bits);
    let (a, b, d) = (r(0), r(1), r(2));
    std::array::from_fn(|k| a[k] * v[0] + b[k] * v[1] + d[k] * v[2] + if k == 3 { v[3] } else { 0.0 })
}

// ---------------------------------------------------------------------------------------------------------------
// 564

pub mod lob_pv {
    pub const DIR: usize = 0x00;
    pub const AT: usize = 0x10;
    pub const YAW: usize = 0x24;
    pub const PITCH: usize = 0x28;
    pub const TICK: usize = 0x2c;
    pub const HALF: usize = 0x30;
    pub const TICKS: usize = 0x34;
    pub const KIND: usize = 0x38;
    pub const SIZE: usize = 0x3c;
}

/// `0x2d5348(yaw, pitch, from, dir, at, n, kind)` (module doc).
#[allow(clippy::too_many_arguments)]
pub fn lob(w: &mut World, yaw: f32, pitch: f32, from: c::V, dir: c::V, at: c::V, n: i32, kind: i32) -> Option<MobyId> {
    let id = w.create_moby(LOB)?;
    if w.m(id).pvars.len() < lob_pv::SIZE { w.mm(id).pvars.resize(0x40, 0); }
    let sc = super::class_scale(w, LOB) * 5.0;
    let m = w.mm(id);
    m.visible = 1;
    m.state = 0;
    m.update_dist = 0xff;
    m.draw_dist = 0xff;
    m.ambient = [0xc0, 0xc0, 0xc0, m.ambient[3]];
    m.scale = sc;
    m.position = from;
    c::set_pv4(w, id, lob_pv::DIR, dir);
    c::set_pv4(w, id, lob_pv::AT, at);
    c::set_pi32(w, id, lob_pv::KIND, kind);
    c::set_pi32(w, id, lob_pv::HALF, n >> 1);
    c::set_pf(w, id, lob_pv::YAW, yaw);
    c::set_pf(w, id, lob_pv::PITCH, pitch);
    c::set_pi32(w, id, lob_pv::TICK, 0);
    c::set_pi32(w, id, lob_pv::TICKS, n);
    w.mm(id).rotation[2] = c::atan(dir[0], dir[1]);
    w.mm(id).rotation[1] = -c::atan(c::len2(dir), dir[2]);
    w.play_sound(0, 0, id);
    Some(id)
}

/// `0x2d5488(m, v)` (module doc).
fn trail(w: &mut World, id: MobyId, v: c::V) { trail_for(w, id, v, 0x14) }

/// The smoke and glow of [`trail`] with both puffs living `ticks(life)` (the tank shot 41's `0x2a7220` makes the same
/// two with 10, `units::veldin_tank`).
pub(crate) fn trail_for(w: &mut World, id: MobyId, v: c::V, life: i32) {
    let rv = w.rng.rand_vec(0.1 * DT, 0.2 * DT);
    let f = w.rng.randf(0.0, 1.0);
    let q = c::add(c::scale(v, f), c::pos(w, id));
    let n = w.ticks(life);
    let a = crate::particles::type44::Spawn { size: 40000.0, growth: 1000.0, damp: 1.0, fall: f32::from_bits(0xb951_b717), w: 0.0, pos: v3(q), vel: rv, life: n, alpha: 0x7f, rgb: 0x60_6060, spin: 3 };
    if let Some(sys) = w.particles.as_deref_mut() {
        *w.svc.fx.part_spawns.entry(44).or_default() += 1;
        if let Some(i) = crate::particles::type44::spawn_rng(sys, w.rng, &a) {
            let def = sys.def_first(23);
            let r = &mut sys.pool.recs[i];
            r[3] = 0x44;
            r[2] = def;
        } else {
            w.svc.fx.part_failed += 1;
        }
    } else {
        fx::part44(w, &a);
    }
    let up = rotate(&w.m(id).rows.map(|r| r.map(f32::to_bits)), [0.0, 0.0, 0.02, 0.0]);
    let n = w.ticks(life);
    fx::part21(w, 20000.0, q, up, 0x4f00_7fff, 0x1fff_ffff, n, 1);
}

/// `0x2d5610(m, v)` (module doc).
fn burst(w: &mut World, id: MobyId) {
    let k = c::pi32(w, id, lob_pv::KIND);
    let big = k & 1 != 0;
    let r = if k & 4 != 0 { 0.0 } else { 1.5 };
    let b = fx::Beam { damage_r: r, damage: 1.0, flash: if big { 4.0 } else { 2.0 }, flash2: if big { 2.0 } else { 1.0 }, flash_dist: 9.0, scale: if big { 2.0 } else { 0.75 }, light: 5.0, streaks: 0x14, sparks: 6, puffs: 0x20, debris: 0, sound: 1, shake: false };
    let p = c::pos(w, id);
    fx::beam_explosion(w, &b, Some(id), p);
    if r != 0.0 && w.body() == 2 { attack::sphere_hit(w, 2.0, 3.0, 1.0, id, p, 2, 0, 1, 0); }
}

/// Level18 0x2d5050 (module doc).
pub fn lob_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < lob_pv::SIZE { return; }
    if held(w) {
        w.delete_moby(id);
        return;
    }
    if w.m(id).state != 0 { return; }
    let old = c::pos(w, id);
    let tick = c::pi32(w, id, lob_pv::TICK);
    let f = (tick as f32 / c::pi32(w, id, lob_pv::HALF) as f32).min(1.0);
    let rem = (c::pi32(w, id, lob_pv::TICKS) - tick) as f32;
    let mut d = c::scale(c::sub(c::pv4(w, id, lob_pv::AT), old), 1.0 / rem);
    let r1 = rc_formats::moby_light::rotation_rows([0.0, 0.0, c::atan(d[0], d[1])]);
    let rt: [[u32; 4]; 3] = std::array::from_fn(|i| std::array::from_fn(|k| if k < 3 { r1[k][i] } else { 0 }));
    let r2 = rc_formats::moby_light::rotation_rows([0.0, c::pf(w, id, lob_pv::PITCH), c::pf(w, id, lob_pv::YAW)]);
    d = rotate(&rt, d);
    d = rotate(&r2, d);
    d = rotate(&r1, d);
    let p0 = c::pv4(w, id, lob_pv::DIR);
    let v: c::V = std::array::from_fn(|k| p0[k] + (d[k] - p0[k]) * f);
    trail(w, id, v);
    let p = c::add(old, v);
    c::set_pos(w, id, p);
    w.mm(id).rotation[2] = c::atan(v[0], v[1]);
    w.mm(id).rotation[1] = -c::atan(c::len2(v), v[2]);
    let rx = c::add_rot(w.m(id).rotation[0], DT * 18.849_556);
    w.mm(id).rotation[0] = rx;
    c::set_pi32(w, id, lob_pv::TICK, tick + 1);
    let out = w.coll_line(sv::pv(old), sv::pv(p), 0, Some(id));
    let mut hit = out.is_some();
    if hit {
        // 0x174858: the moby the line hit.
        if let Some(m) = out.and_then(|o| o.moby) {
            match w.m(m).o_class {
                0x1df => {
                    let k = c::pi32(w, id, lob_pv::KIND) & 4;
                    c::set_pi32(w, id, lob_pv::KIND, k);
                }
                0x1a3 => {}
                _ => hit = false,
            }
        }
    }
    if tick + 1 != c::pi32(w, id, lob_pv::TICKS) && !hit {
        if c::pi32(w, id, lob_pv::KIND) & 2 == 0 { return; }
        if (60.0 * f32::from_bits(w.svc.timing.timer_scale.0)) < (c::pi32(w, id, lob_pv::TICKS) - (tick + 1)) as f32 { return; }
        if let Some(r) = super::row(REFERENCE_LEVEL, LOB_MARK_FN) { w.svc.draw_callbacks.register(Callback::UnitQuads(r), id); }
        return;
    }
    burst(w, id);
    w.delete_moby(id);
}

/// `0x2d5768` (draw only; module doc).
pub fn lob_quads(table: &crate::moby_runtime::MobyTable, _svc: &crate::moby_update::Services, id: MobyId) -> Option<FxQuads> {
    let m = table.mobys.get(id).filter(|m| m.o_class == LOB && m.pvars.len() >= lob_pv::SIZE)?;
    let at = sv::pvar::v4f(&m.pvars, lob_pv::AT);
    let t = sv::pvar::i32(&m.pvars, lob_pv::TICK);
    let s = ((t as u32 & 0xf) as f32 * 0.0625 * f32::from_bits(0x40c9_0fd0)).sin();
    let rgba = crate::hud::tween_color((s + 1.0) * 0.5, 0x8000_20f0, 0x8000_f0f0);
    let off = [[1.0, 1.0], [1.0, -1.0], [-1.0, 1.0], [-1.0, -1.0]];
    let corners = off.map(|o| [at[0] + o[0], at[1] + o[1], at[2] + 0.1]);
    Some(FxQuads { fx: 0xd, additive: false, subtract: false, quads: vec![FxQuad { corners, st: [[1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 1.0]], rgba: [rgba; 4] }] })
}

// ---------------------------------------------------------------------------------------------------------------
// 624

pub mod ring_pv {
    pub const VEL: usize = 0x00;
    pub const GROW: usize = 0x20;
    pub const RADIUS: usize = 0x24;
    pub const MAX: usize = 0x28;
    pub const TIMER: usize = 0x2c;
    pub const PHASE: usize = 0x30;
    pub const BOSS: usize = 0x34;
    pub const SIZE: usize = 0x38;
}

/// `0x2db938(r, k, from, dir, boss)` (module doc).
pub fn ring_new(w: &mut World, r: f32, k: f32, from: c::V, dir: c::V, boss: MobyId) -> Option<MobyId> {
    let id = w.create_moby(RING)?;
    if w.m(id).pvars.len() < ring_pv::SIZE { w.mm(id).pvars.resize(0x40, 0); }
    let sc = w.m(id).scale * 1.0;
    let m = w.mm(id);
    m.update_dist = 0xff;
    m.visible = 1;
    m.state = 0;
    m.draw_dist = 0xff;
    m.mode |= mode::GLOW;
    m.scale = sc;
    m.glow = 0x80c0_c0c0;
    m.position = from;
    m.rotation[2] = c::atan(dir[0], dir[1]);
    c::set_pv4(w, id, ring_pv::VEL, dir);
    c::set_pi32(w, id, ring_pv::BOSS, boss as i32 + 1);
    c::set_pf(w, id, ring_pv::MAX, r);
    c::set_pf(w, id, ring_pv::GROW, k);
    c::set_pf(w, id, ring_pv::PHASE, 0.0);
    Some(id)
}

fn ring_glow(w: &mut World, id: MobyId) {
    let a = c::add_rot(c::pf(w, id, ring_pv::PHASE), DT * 18.849_556);
    c::set_pf(w, id, ring_pv::PHASE, a);
    w.mm(id).glow = crate::hud::tween_color((a.sin() + 1.0) * 0.5, 0x80c0_c0c0, 0x8020_20c0);
}

/// Level18 0x2db460 (module doc).
pub fn ring_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < ring_pv::SIZE { return; }
    if held(w) {
        w.delete_moby(id);
        return;
    }
    match w.m(id).state {
        0 => {
            let vel = c::pv4(w, id, ring_pv::VEL);
            let mut v1 = c::scale(vel, 0.5);
            let r1 = w.rng.rand_vec(0.0, DT);
            v1 = c::add(v1, [r1[0], r1[1], r1[2], 0.0]);
            let r2 = w.rng.rand_vec(0.0, 0.5);
            let p = c::add([r2[0], r2[1], r2[2], 0.0], c::pos(w, id));
            let mut v2 = [0.0f32; 4];
            v1[3] = w.rng.randf(0.75, 0.5);
            v2[3] = w.rng.randf(0.333, 0.25);
            let c1 = tween(w, 0x8040_80ff, 0x8040_ffff);
            let c2 = tween(w, 0x8010_4040, 0x8000_2080);
            let f = w.rng.randf(0.0, 1.0);
            let t0 = scaled(w, 1.0 * f + 1.0);
            let f = w.rng.randf(-0.5, 0.5);
            let t1 = scaled(w, 30.0 * (f + 1.0));
            let f = w.rng.randf(-0.5, 0.5);
            let t2 = scaled(w, 10.0 * (f + 1.0));
            v2[2] -= 4.0 * DT2 * t1 as f32;
            fx::part02(w, &Spawn { pos: p, v1, v2, c1, c2, t: [t0, t1, t2], def: 0x1_0019 });
            ring_glow(w, id);
            let mut vel = c::pv4(w, id, ring_pv::VEL);
            vel[2] -= DT2 * 10.0;
            c::set_pv4(w, id, ring_pv::VEL, vel);
            let p = c::add(c::pos(w, id), vel);
            c::set_pos(w, id, p);
            let g = ground::ground(w, p, 0.5, 0).z;
            if w.m(id).position[2] - 0.333 < g {
                w.mm(id).state = 1;
                w.mm(id).position[2] = g + 0.333;
                let t = w.ticks(1);
                c::set_pi32(w, id, ring_pv::TIMER, t);
            }
            let r = c::add_rot(w.m(id).rotation[1], DT * f32::from_bits(0x410b_a058));
            w.mm(id).rotation[1] = r;
            // z ≥ 10: the blob shadow `0x25c328(1, m)`; below 10: deleted.
            if w.m(id).position[2] < 10.0 { w.delete_moby(id); } else { crate::shadows::blob(w, 1.0, id); }
        }
        1 => {
            ring_glow(w, id);
            if c::dec_timer_pvar_i32(w, id, ring_pv::TIMER) == 0 { return; }
            w.play_sound_as(0xf, 0, id, SOUND_CLASS);
            let m = w.mm(id);
            m.state = 2;
            m.has_collision = false;
            m.visible = 0;
            m.mode |= mode::HIDDEN;
            c::set_pf(w, id, ring_pv::RADIUS, 0.5);
        }
        2 => {
            let r = c::pf(w, id, ring_pv::RADIUS) + c::pf(w, id, ring_pv::GROW);
            c::set_pf(w, id, ring_pv::RADIUS, r);
            ring_hits(w, id);
            if c::pf(w, id, ring_pv::MAX) < r { w.mm(id).state = 3; }
        }
        3 => w.delete_moby(id),
        _ => {}
    }
}

/// `0x25bbc8(damage, tmpl, m, flags, dir)`: the template (+0x18..+0x1b stale stack: 0 [L]).
fn template(id: MobyId, damage: f32, flags: u32, dir: c::V) -> HitTemplate {
    HitTemplate { dir: sv::pv(dir), attacker: Some(id), flags, b18: 0, b19: 0, h1a: 0, damage: Pf::f(damage), w20: 1 }
}

/// The push toward Ratchet: unit(Ratchet − position, xy)·4·dt, z 4·dt.
fn push_dir(w: &World, id: MobyId) -> c::V {
    let mut d = c::sub(super::hero_pos(w), c::pos(w, id));
    d[2] = 0.0;
    let mut d = c::set_len3(d, 4.0 * DT);
    d[2] = 4.0 * DT;
    d
}

/// `0x2dbe80` (module doc).
fn ring_hits(w: &mut World, id: MobyId) {
    let tmpl = template(id, 3.0, 0x1_0001, push_dir(w, id));
    let boss = link(w, id, ring_pv::BOSS);
    let r = c::pf(w, id, ring_pv::RADIUS);
    let p = c::pos(w, id);
    let step = (1.0 / 64.0) * f32::from_bits(0x40c9_0fd0);
    let mut a = f32::from_bits(0xc049_0fd0);
    let mut b = c::add_rot(a, step);
    for _ in 0..64 {
        let s = [a.cos() * r + p[0], a.sin() * r + p[1], p[2] + 0.5, p[3]];
        let e = [b.cos() * r + p[0], b.sin() * r + p[1], p[2] + 0.5, p[3]];
        sv::line_hit_in(w.table, w.svc, w.classes, w.coll, sv::pv(s), sv::pv(e), 1, boss, &tmpl);
        if r < 3.0 { ring_sparks(w, id, s, e); }
        a = c::add_rot(a, step);
        b = c::add_rot(b, step);
    }
    if let Some(row) = super::row(REFERENCE_LEVEL, RING_FN) { w.svc.draw_callbacks.register(Callback::UnitQuads(row), id); }
}

/// `0x2dc0c8(m, a, b)` (module doc).
fn ring_sparks(w: &mut World, id: MobyId, a: c::V, b: c::V) {
    let mid: c::V = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * 0.5);
    let r = c::dist3(a, b) * 0.5;
    if !fx::in_view(w, 48.0, mid, r) { return; }
    if 0.25 <= w.rng.randf(0.0, 1.0) { return; }
    let t = w.rng.randf(0.0, 1.0);
    let q: c::V = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t);
    let mut d = c::set_len3(c::sub(q, c::pos(w, id)), c::pf(w, id, ring_pv::GROW));
    let col = tween(w, 0x8010_0080, 0x8010_8080);
    let n = w.ticks(0x2d);
    let rot = w.rng.randi(0xff);
    w.part60(2.0, q, d, col, n as u16, rot as u8, 0);
    d[2] = 0.0;
    let n = w.ticks(0x78);
    let rot = w.rng.randi(0xff);
    w.part60(2.0, q, d, col, n as u16, rot as u8, 0);
}

/// The constants of one band draw: `0x2dba20` (624) and `0x2dc4b8` (628) are the same code with their own gp words.
struct Band {
    /// The segment angle in degrees (gp−0x4f9c 10 / gp−0x4f40 30); `360 / it` copies round the ring.
    step: f32,
    /// The height above the moby's position (gp−0x4f98 0.25 / gp−0x4f3c 1).
    z: f32,
    /// The half-height per unit of the phase (gp−0x4f94 5 / gp−0x4f38 3).
    height: f32,
    /// The radius added per band (gp−0x4f90 0.1 / gp−0x4f34 0.05).
    spread: f32,
    /// The band colours (gp−0x4fa4 0x800060ff, gp−0x4fa0 0x000000ff; 628: gp−0x4f48 / −0x4f44, the same).
    from: u32,
    to: u32,
}

const RING_BAND: Band = Band { step: 10.0, z: 0.25, height: 5.0, spread: 0.1, from: 0x8000_60ff, to: 0x0000_00ff };
const AURA_BAND: Band = Band { step: 30.0, z: 1.0, height: 3.0, spread: 0.05, from: 0x8000_60ff, to: 0x0000_00ff };

/// The band draw (`0x2dba20` / `0x2dc4b8`): four bands `i` = 0..3 of radius `r + spread·i` and phase `f = frac(ph +
/// 0.25·i)`; colour `tween(f, from, to)`, its alpha faded in by `min(4f, 1)` (`tween(·, c & 0xffffff, c)`) and by
/// `fade` (clamped 0..1); a band's quad (FX 0xe, ALPHA 0x48 additive, st (0,0) (0,1) (1,0) (1,1) from 0x1d4310)
/// spans one step of the circle, `(a, −h) (a, +h) (a + step, −h) (a + step, +h)` with `h = height·f`, `a = −π +
/// step·0.25·i`, at radius `r + spread·i`. The matrix is the identity with the moby's position (z + `z`) and is
/// turned by `Rz(step)` (rows only, `fun_001fa328`) after each copy, `360 / step` copies: copy `k`'s vertex angles
/// are `a + k·step` (the turn's sign is a convention; the ring is the same either way [L]).
fn band_quads(b: &Band, pos: c::V, r: f32, ph: f32, fade: f32) -> FxQuads {
    let fade = fade.clamp(0.0, 1.0);
    let step = b.step * DEG;
    let copies = (360.0 / b.step).ceil() as usize;
    let mut quads = Vec::with_capacity(copies * 4);
    let mut bands = [(0.0f32, 0.0f32, 0.0f32, 0u32); 4];
    for (i, band) in bands.iter_mut().enumerate() {
        let fi = i as f32;
        let rad = r + b.spread * fi;
        let f = ph + fi * 0.25;
        let f = f - (f as i32) as f32;
        let c0 = crate::hud::tween_color(f, b.from, b.to);
        let c1 = crate::hud::tween_color((f * 4.0).min(1.0), c0 & 0xff_ffff, c0);
        let col = crate::hud::tween_color(fade, c1 & 0xff_ffff, c1);
        let a0 = c::add_rot(-f32::from_bits(0x4049_0fd0), b.step * 0.25 * fi * DEG);
        *band = (rad, a0, f * b.height, col);
    }
    let base = [pos[0], pos[1], pos[2] + b.z];
    for k in 0..copies {
        let turn = k as f32 * step;
        for &(rad, a0, h, col) in &bands {
            let a = a0 + turn;
            let e = a + step;
            let at = |ang: f32, z: f32| [base[0] + ang.cos() * rad, base[1] + ang.sin() * rad, base[2] + z];
            quads.push(FxQuad {
                corners: [at(a, -h), at(a, h), at(e, -h), at(e, h)],
                st: [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
                rgba: [col; 4],
            });
        }
    }
    FxQuads { fx: 0xe, additive: true, subtract: false, quads }
}

/// `0x2dba20`: the ring's bands ([`band_quads`]: radius +0x24, phase `+0x24·0.1` (gp−0x4f8c), fade `(+0x28 −
/// +0x24) / (+0x20·scale(20))` (gp−0x4f84 20.0, `multiply_global_scale`)).
pub fn ring_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, id: MobyId) -> Option<FxQuads> {
    let m = table.mobys.get(id).filter(|m| m.o_class == RING && m.pvars.len() >= ring_pv::SIZE)?;
    let r = sv::pvar::ff(&m.pvars, ring_pv::RADIUS);
    let max = sv::pvar::ff(&m.pvars, ring_pv::MAX);
    let grow = sv::pvar::ff(&m.pvars, ring_pv::GROW);
    let fade = (max - r) / (grow * (f32::from_bits(svc.timing.timer_scale.0) * 20.0));
    let fade = fade.clamp(0.0, 1.0);
    Some(band_quads(&RING_BAND, m.position, r, r * 0.1, fade))
}

/// `0x2dc4b8`: the aura's bands ([`band_quads`]: radius +0x00, phase +0x04, fade `+0x08 / ticks(20)` (gp−0x4f64)).
pub fn aura_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, id: MobyId) -> Option<FxQuads> {
    let m = table.mobys.get(id).filter(|m| m.o_class == AURA && m.pvars.len() >= aura_pv::SIZE)?;
    let r = sv::pvar::ff(&m.pvars, aura_pv::R);
    let ph = sv::pvar::ff(&m.pvars, aura_pv::PHASE);
    let t = sv::pvar::i32(&m.pvars, aura_pv::TIMER) as f32;
    let fade = t / svc.timing.ticks(0x14) as f32;
    Some(band_quads(&AURA_BAND, m.position, r, ph, fade))
}

// ---------------------------------------------------------------------------------------------------------------
// 628

pub mod aura_pv {
    pub const R: usize = 0x00;
    pub const PHASE: usize = 0x04;
    pub const TIMER: usize = 0x08;
    pub const BOSS: usize = 0x0c;
    pub const SIZE: usize = 0x10;
}

/// `0x2dc3e0(r, boss, p)` (module doc).
pub fn aura_new(w: &mut World, r: f32, boss: MobyId, p: c::V) -> Option<MobyId> {
    let id = w.create_moby(AURA)?;
    if w.m(id).pvars.len() < aura_pv::SIZE { w.mm(id).pvars.resize(0x10, 0); }
    w.mm(id).update_dist = 0xff;
    w.mm(id).position = p;
    c::set_pi32(w, id, aura_pv::BOSS, boss as i32 + 1);
    c::set_pf(w, id, aura_pv::R, r);
    c::set_pi32(w, id, aura_pv::TIMER, 3);
    Some(id)
}

/// `0x2dc458(r, aura, p)` (module doc).
pub fn aura_follow(w: &mut World, r: f32, id: MobyId, p: c::V) {
    if w.m(id).pvars.len() < aura_pv::SIZE { return; }
    w.mm(id).position = p;
    c::set_pf(w, id, aura_pv::R, r);
    let t = c::pi32(w, id, aura_pv::TIMER) + 2;
    let cap = w.ticks(0x14);
    c::set_pi32(w, id, aura_pv::TIMER, if cap < t { cap } else { t });
}

/// Level18 0x2dc260 (module doc).
pub fn aura_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < aura_pv::SIZE { return; }
    if c::dec_timer_pvar_i32(w, id, aura_pv::TIMER) != 0 {
        w.delete_moby(id);
        return;
    }
    let p = c::pos(w, id);
    let r = c::pf(w, id, aura_pv::R);
    if c::dist2(p, super::hero_pos(w)) < r + 0.5 {
        let tmpl = template(id, 1.0, 0x1_0001, push_dir(w, id));
        if let Some(h) = w.hero_moby { w.deliver_hit(h, &tmpl); }
    }
    let ph = c::pf(w, id, aura_pv::PHASE) + 2.0 * DT;
    c::set_pf(w, id, aura_pv::PHASE, ph);
    if w.counter & 4 == 0 {
        if let Some(b) = link(w, id, aura_pv::BOSS) { attack::sphere_hit(w, r, 1.0, 1.0, b, p, 0x1_0000, 0, 1, 0); }
    }
    // `RegisterDrawCallback(0x2dc4b8)`: the bands ([`aura_quads`]).
    if let Some(row) = super::row(REFERENCE_LEVEL, AURA_DRAW_FN) { w.svc.draw_callbacks.register(Callback::UnitQuads(row), id); }
}

// ---------------------------------------------------------------------------------------------------------------
// 983

pub mod beam_pv {
    pub const AT: usize = 0x00;
    pub const HEAD: usize = 0x10;
    pub const BOSS: usize = 0x20;
    pub const T: usize = 0x24;
    pub const TIMER: usize = 0x28;
    pub const SPIN_A: usize = 0x2c;
    pub const SPIN_B: usize = 0x30;
    pub const LIFE: usize = 0x34;
    pub const SIZE: usize = 0x38;
    /// The port's: the camera position the core draw faces (0x1677c0), kept by the update [L: last tick's].
    pub const CAM: usize = 0x40;
    pub const LEN: usize = 0x50;
}

/// gp−0x4d1c..−0x4d10 (level18 0x161ee4..): the core's ALPHA bytes A / B / C / D = Cs, 0, As, Cd (additive); its
/// colours 0x161ef8.. (quad 0 tweened in from alpha 0 by +0x24) and FX 0x161f08..; the quads' corners (y, z) in the
/// plane facing the camera (0x1d9b70: half sizes 2, 1, 3, 3) and their ST (0x1d9b50).
const CORE_RGBA: [u32; 4] = [0x7040_2000, 0x7060_6010, 0x7080_4010, 0x7080_4010];
const CORE_FX: [usize; 4] = [8, 11, 20, 20];
const CORE_HALF: [f32; 4] = [2.0, 1.0, 3.0, 3.0];
const CORE_ST: [[f32; 2]; 4] = [[1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 1.0]];

/// `0x2ea1f8`: the core, four additive quads at the beam's position facing the camera (Euler (roll, −pitch, yaw) to it:
/// rolls 0, 0, +0x2c, +0x30), their y / z rows scaled by +0x24, `FastDrawQuadReal` each.
pub fn core_quads(table: &crate::moby_runtime::MobyTable, _svc: &crate::moby_update::Services, id: MobyId) -> Vec<FxQuads> {
    let Some(m) = table.mobys.get(id).filter(|m| m.o_class == BEAM && m.pvars.len() >= beam_pv::LEN) else { return Vec::new() };
    let pf = |o: usize| sv::pvar::ff(&m.pvars, o);
    let p = [m.position[0], m.position[1], m.position[2]];
    let cam = [pf(beam_pv::CAM), pf(beam_pv::CAM + 4), pf(beam_pv::CAM + 8)];
    let yaw = c::atan(cam[0] - p[0], cam[1] - p[1]);
    let dxy = ((cam[0] - p[0]).powi(2) + (cam[1] - p[1]).powi(2)).sqrt();
    let pitch = -c::atan(dxy, cam[2] - p[2]);
    let s = pf(beam_pv::T);
    let rolls = [0.0, 0.0, pf(beam_pv::SPIN_A), pf(beam_pv::SPIN_B)];
    (0..4).map(|k| {
        let r = crate::moby_update::triggers::euler_matrix([rolls[k], pitch, yaw]);
        let (ry, rz) = (r[1].map(|v| v as f32 * s), r[2].map(|v| v as f32 * s));
        let h = CORE_HALF[k];
        let corner = |y: f32, z: f32| std::array::from_fn(|i| p[i] + ry[i] * y + rz[i] * z);
        let corners = [corner(h, -h), corner(-h, -h), corner(h, h), corner(-h, h)];
        let rgba = if k == 0 { crate::hud::tween_color(s, CORE_RGBA[0] & 0x00ff_ffff, CORE_RGBA[0]) } else { CORE_RGBA[k] };
        FxQuads { fx: CORE_FX[k], additive: true, subtract: false, quads: vec![FxQuad { corners, st: CORE_ST, rgba: [rgba; 4] }] }
    }).collect()
}

/// `0x2eb9f8`'s frame part: the scroll step and the glow's two `rand` draws (`super::tesla_bolt::frame`).
pub fn strands_frame(w: &mut World, _id: MobyId) {
    let mut b = std::mem::take(&mut w.svc.units.veldin_strands);
    super::tesla_bolt::frame(w, &mut b, true);
    w.svc.units.veldin_strands = b;
}

/// `0x2eb9f8`: the strands' strips and the glow at the beam (`0x2ec130`: 0.1 / 0.35 across plus the frame's jitters;
/// the second quad's colour is the strands' with alpha 0, as the game's gp−0x4bac).
pub fn strand_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, id: MobyId) -> Vec<FxQuads> {
    let b = &svc.units.veldin_strands;
    let Some(m) = table.mobys.get(id) else { return Vec::new() };
    let mut q = super::tesla_bolt::strips(b);
    let sizes = [0.1 + b.glow[0], 0.35 + b.glow[1]];
    q.extend(super::tesla_bolt::glow(b, [m.position[0], m.position[1], m.position[2]], sizes, [super::tesla_bolt::GLOW_WHITE, b.t.chain.color]));
    super::tesla_bolt::groups(q)
}

/// `0x2ea0f0(boss, p, n)` (module doc).
pub fn beam_new(w: &mut World, boss: MobyId, p: c::V, n: i32) -> Option<MobyId> {
    let id = w.create_moby(BEAM)?;
    if w.m(id).pvars.len() < beam_pv::LEN { w.mm(id).pvars.resize(beam_pv::LEN, 0); }
    w.mm(id).update_dist = 0xff;
    w.mm(id).position = p;
    c::set_pi32(w, id, beam_pv::BOSS, boss as i32 + 1);
    c::set_pi32(w, id, beam_pv::TIMER, 5);
    c::set_pi32(w, id, beam_pv::LIFE, n);
    c::set_pf(w, id, beam_pv::T, 0.0);
    Some(id)
}

/// `0x2ea168(t, beam, p, at, fire)` (module doc).
pub fn beam_aim(w: &mut World, t: f32, id: MobyId, p: c::V, at: c::V, fire: bool) {
    if w.m(id).pvars.len() < beam_pv::SIZE { return; }
    w.mm(id).position = p;
    let n = w.ticks(5);
    c::set_pi32(w, id, beam_pv::TIMER, n);
    c::set_pf(w, id, beam_pv::T, t);
    if fire && w.m(id).state == 0 {
        w.mm(id).state = 1;
        c::set_pi32(w, id, beam_pv::TIMER, 0);
        c::set_pv4(w, id, beam_pv::AT, at);
    }
}

/// `0x2ea598` (module doc).
fn beam_sparks(w: &mut World, id: MobyId) {
    let d = c::set_len3(c::sub(c::pv4(w, id, beam_pv::AT), c::pos(w, id)), 60.0 * DT);
    for _ in 0..3 {
        let r = w.rng.rand_vec(6.0 * DT, 12.0 * DT);
        let mut v1 = [r[0], r[1], r[2], 0.0];
        let mut v2 = c::scale(v1, 0.1);
        let z = w.rng.randf(2.0 * DT, 4.0 * DT);
        v2[2] -= z;
        let f = w.rng.randf(1.0, 5.0);
        let t0 = scaled(w, f);
        let f = w.rng.randf(20.0, 30.0);
        let t1 = scaled(w, f);
        let f = w.rng.randf(30.0, 45.0);
        let t2 = scaled(w, f);
        v1[3] = w.rng.randf(0.5, 0.333);
        v2[3] = w.rng.randf(0.333, 0.2);
        let f = w.rng.randf(0.0, 1.0);
        let p = c::add(c::scale(d, f), c::pos(w, id));
        let c1 = tween(w, 0x8080_7060, 0x8080_7000);
        let c2 = tween(w, 0x8080_5020, 0x8080_2010);
        fx::part02(w, &Spawn { pos: p, v1, v2, c1, c2, t: [t0, t1, t2], def: 0x35 });
    }
}

/// The impact's TNT spark colours (level18 0x1d9c70 / 0x1d9c88).
pub const IMPACT_A: [u32; 6] = [0x4fff_8f00, 0x4fff_8f00, 0x4fff_7f00, 0x4fff_6f00, 0x2fff_ffff, 0x2fff_ffff];
pub const IMPACT_B: [u32; 6] = [0x2f7f_5f00, 0x2f7f_4f00, 0x2f7f_3f00, 0x2f4f_4f4f, 0x2fc0_c0c0, 0x3fc0_c0c0];

/// `0x2ea800` (module doc).
fn impact(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    let n = w.ticks(0x1e);
    flash_new(w, p, n, 0x8080_6060, 0x0080_1010);
    w.play_sound_as(0x10, 0, id, SOUND_CLASS);
    for _ in 0..100 {
        let pitch = w.rng.randf(f32::from_bits(0xbeb2_b8c2), f32::from_bits(0x3eb2_b8c2));
        let ang = w.rng.rand_angle();
        let sp = w.rng.randf(10.0, 30.0);
        let mut v1 = fx::polar(sp * DT, ang, pitch);
        let mut v2 = c::scale(v1, 0.02);
        let k = w.rng.randf(0.8, 1.2);
        v2[2] += 2.0 * k * DT;
        v1[3] = 0.25;
        v2[3] = 0.25;
        let k = w.rng.randf(0.8, 1.2);
        let t0 = (w.ticks(0x1e) as f32 * k) as i32;
        let t1 = (w.ticks(0x1e) as f32 * k) as i32;
        let t2 = (w.ticks(0xf0) as f32 * k) as i32;
        fx::part02(w, &Spawn { pos: p, v1, v2, c1: 0x60c0_8010, c2: 0x50c0_8010, t: [t0, t1, t2], def: 0x35 });
    }
    for _ in 0..50 {
        let sp = w.rng.randf(20.0, 50.0) * DT;
        let pitch = w.rng.randf(f32::from_bits(0x3f86_0a92), f32::from_bits(0x3fb2_b8c2));
        let ang = w.rng.rand_angle();
        let vel = fx::polar(sp, ang, pitch);
        let a = w.rng.rand_angle();
        let (ca, sa) = (a.cos(), a.sin());
        let rx = w.rng.randf(0.0, 10.0);
        let ry = w.rng.randf(0.0, 10.0);
        let at = c::add([ca * rx, sa * ry, 0.0, 0.0], p);
        let c1 = IMPACT_A[w.rng.randi(6) as usize];
        let c2 = IMPACT_B[w.rng.randi(6) as usize];
        let n = w.rng.rand_range(0x1e, 0x3c);
        let t1 = w.ticks(n);
        let n = w.rng.rand_range(0x1e, 0x3c);
        let t2 = w.ticks(n);
        w.part11(Pf::f(3.0 * 400000.0), Pf::f(sp), sv::pv(at), sv::pv(vel), c1, c2, t1, t2, 0, 0);
    }
    for (size, rgba) in [(10.0, [0x10, 0x40, 0xc0, 0x40]), (15.0, [0x10, 0x40, 0xc0, 0x40]), (25.0, [0x10, 0x40, 0xc0, 0x20])] {
        let n = w.ticks(0x1e);
        crate::moby_update::classes::debris::flash_spawn(w, size, id, p, [0.0; 4], n, rgba[0], rgba[1], rgba[2], rgba[3]);
    }
    let n = w.ticks(0x2d);
    w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp: 0.35, ticks: n });
}

/// Level18 0x2e9e70 (module doc).
pub fn beam_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < beam_pv::SIZE { return; }
    let a = c::add_rot(c::pf(w, id, beam_pv::SPIN_A), 720.0 * DEG * DT);
    c::set_pf(w, id, beam_pv::SPIN_A, a);
    let b = c::add_rot(c::pf(w, id, beam_pv::SPIN_B), -540.0 * DEG * DT);
    c::set_pf(w, id, beam_pv::SPIN_B, b);
    match w.m(id).state {
        0 => {
            if c::dec_timer_pvar_i32(w, id, beam_pv::TIMER) != 0 {
                w.delete_moby(id);
            }
        }
        1 => {
            let p = c::pos(w, id);
            let d = c::sub(c::pv4(w, id, beam_pv::AT), p);
            let dist = c::len3(d);
            beam_sparks(w, id);
            let r = 2.0 * c::pf(w, id, beam_pv::T);
            let rv = w.rng.rand_vec(r, r);
            c::set_pv4(w, id, beam_pv::HEAD, c::add([rv[0], rv[1], rv[2], 0.0], p));
            // `0x2eacd8` / `0x2eaea0`: the strands from here onto the head; `RegisterDrawCallback2(0x2eb9f8)`.
            let (from, head) = (v3(p), v3(c::pv4(w, id, beam_pv::HEAD)));
            let mut b = std::mem::take(&mut w.svc.units.veldin_strands);
            super::tesla_bolt::reset(&mut b, from, head);
            super::tesla_bolt::build(w, &mut b, from, head, head);
            w.svc.units.veldin_strands = b;
            if let Some(r) = super::row(REFERENCE_LEVEL, STRAND_DRAW_FN) {
                w.svc.draw_callbacks.register2(Callback::UnitFrame(r), id);
                w.svc.draw_callbacks.register2(Callback::UnitQuads(r), id);
            }
            let speed = 70.0 * DT;
            if dist < speed {
                impact(w, id);
                w.delete_moby(id);
                return;
            }
            let p = c::add(p, c::set_len3(d, speed));
            c::set_pos(w, id, p);
            let mut g = p;
            g[2] = ground::ground(w, p, 0.5, 0).z;
            if let Some(boss) = link(w, id, beam_pv::BOSS) {
                attack::sphere_hit(w, 2.0, 5.0, 1.0, boss, p, 0x1_0001, 0, 1, 0);
                attack::sphere_hit(w, 2.0, 5.0, 1.0, boss, g, 0x1_0001, 0, 1, 0);
            }
            if c::dec_timer_pvar_i32(w, id, beam_pv::LIFE) != 0 {
                let mut t = c::pf(w, id, beam_pv::T);
                turn::approach(0.0, 0.2, &mut t);
                c::set_pf(w, id, beam_pv::T, t);
                if t == 0.0 {
                    w.delete_moby(id);
                }
            }
        }
        _ => {}
    }
    // `RegisterDrawCallback(0x2ea1f8)` ([`core_quads`], facing the camera of this tick).
    if w.m(id).pvars.len() >= beam_pv::LEN {
        let cam = w.camera.map(|x| x.to_f32());
        c::set_pv4(w, id, beam_pv::CAM, [cam[0], cam[1], cam[2], 0.0]);
        if let Some(r) = super::row(REFERENCE_LEVEL, CORE_DRAW_FN) { w.svc.draw_callbacks.register(Callback::UnitQuads(r), id); }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 1898

/// `0x2fb5e0(p, n, c1, c2)` (module doc).
pub fn flash_new(w: &mut World, p: c::V, n: i32, c1: u32, c2: u32) -> Option<MobyId> {
    let id = w.create_moby(FLASH)?;
    if w.m(id).pvars.len() < 0x10 { w.mm(id).pvars.resize(0x10, 0); }
    w.mm(id).position = p;
    c::set_pi32(w, id, 0xc, c2 as i32);
    c::set_pi32(w, id, 8, c1 as i32);
    c::set_pf(w, id, 4, 2.0 / n as f32);
    w.mm(id).state = 1;
    w.mm(id).update_dist = 0xff;
    Some(id)
}

/// gp−0x462c / −0x4628 / −0x4624..−0x4618 (level18 0x1625d4..): the flash's FX 11, its size 800 (GS pixels at t = 1)
/// and its UVs (texels ×16: (0, 0), (0, 0x400), (0x400, 0), (0x400, 0x400)).
const FLASH_FX: u16 = 11;
const FLASH_SIZE: f32 = 800.0;
const FLASH_UV: [[i32; 2]; 4] = [[0, 0], [0, 64], [64, 0], [64, 64]];

/// The draw `0x2fb690` of 1898 (TEST 0x5380b around it): a screen sprite on its position's projection, `800·min(t, 1)`
/// pixels each way, colour `tween(clamp(t − 1, 0, 1), +0x08, +0x0c)`.
pub fn flash_sprite(w: &World, id: MobyId) -> crate::targeting::ScreenSprite {
    let t = c::pf(w, id, 0);
    let grow = t.min(1.0);
    let fade = (t - 1.0).clamp(0.0, 1.0);
    let rgba = crate::hud::tween_color(fade, c::pi32(w, id, 8) as u32, c::pi32(w, id, 0xc) as u32);
    let p = c::pos(w, id);
    let half = ((FLASH_SIZE * 16.0 * grow) as i32) as f32 / 16.0;
    crate::targeting::ScreenSprite { tick: w.counter, at: [p[0], p[1], p[2]], half, fx: FLASH_FX, uv: FLASH_UV, rgba }
}

/// Level18 0x2fb548 (module doc).
pub fn flash_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x10 { return; }
    match w.m(id).state {
        0 => w.delete_moby(id),
        1 => {
            // `RegisterDrawCallback(0x2fb690)` (its draw after this update: [`flash_sprite`]).
            let t = c::pf(w, id, 0) + c::pf(w, id, 4);
            c::set_pf(w, id, 0, t);
            if 2.0 < t {
                w.delete_moby(id);
                return;
            }
            let s = flash_sprite(w, id);
            let tick = w.counter;
            w.svc.screen_sprites.retain(|x| x.tick == tick);
            w.svc.screen_sprites.push(s);
        }
        _ => {}
    }
}
