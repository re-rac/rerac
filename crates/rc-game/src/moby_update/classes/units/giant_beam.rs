//! **Giant Clank's head beam** (class 0x5f3; create level15 `0x29ec00` / level18 `0x2ab8f0` (level00 `0x2a9aa0`), its
//! point `0x29ece0` / `0x2ab9d0` (`0x2a9b80`), its end `0x29ed40` / `0x2aba30` (`0x2a9be0`), update `0x29edb0` /
//! `0x2abaa0`, the same code on both levels (`clusters.tsv`)). Giant Clank's state 0x61 (△) holds the beam at a point
//! 3 ahead and 5 up of him every tick up to key 22 of his beam sequence ([`point`]); while it is held it charges: blue
//! sparks drawn into it, a growing light and flash, the screen flash record; the first tick nobody holds it, it flies
//! off along the hero's yaw at 40 u/s for a second, growing, smashing everything it touches (20 damage) in a storm of
//! glows, streaks and twinkles. A hit on Giant Clank while it charges deletes it ([`end`], the hit intake's body-2
//! branch). Read from the level15 decompiler output and the level18 disassembly.
//!
//! **System or not.** Per-class code (one class, one copy per Giant Clank level); the shared pieces are the port's:
//! the area hit (`creature::attack::area_hit` = `0x26f8f8`), the particle spawners (types 5, 15, 26, 31, 69), the
//! point-light bank ([`crate::point_lights`]). The beam moby is a level global (level15 gp−0x577c, level18 gp−0x57c4):
//! [`GLOBAL`] in the unit words.
//!
//! **Pvars**: +0x00..+0x08 the flight velocity, +0x10 (s32) the charge ticks, +0x14 (s32) the flight ticks, +0x2c (s16)
//! the point-light slot (−1 none), +0x2e (s16) held this tick (set by [`point`], cleared by every update).
//!
//! **Coverage** (level15 addresses):
//!
//! | address | what | status |
//! |---|---|---|
//! | 0x29ec00 | `CreateMoby(0x5f3)`; made and no beam: the beam = it, +0x32 = 0xff, +0x31 = 1, +0x30 = 0xff, state 0, at the point; the screen record: 0x15f324 = 0xff, 0x15f334 = 0xffffff, 0x15f338 = 0x1f00000042, 0x15f328 = 0x15f320 = 0x15f330 = 0; +0x10 = 0; the light `0x22dbe0(0, 0, point, 0)` (none above the frame load 0.8 or with eight taken) | ported ([`create`]) |
//! | 0x29ece0 | the beam (made if none) of class 0x5f3: at the point, +0x2e = 1 | ported ([`point`]) |
//! | 0x29ed40 | a beam of class 0x5f3 in state 0: 0x15f320 = 0x15f330 = 0, its light freed, `DeleteMoby`, beam = 0; of another class: beam = 0 (a flying beam stays) | ported ([`end`]) |
//! | 0x29edb0 state 0, held, body 2 | +0x10: 0 → 0x15f330 = 1, 0x15f334 = 0x00ffff00; +1; k = min(+0x10 / 30, 1); trunc(10·k²) sparks: a random direction (3 × `randf(−1, 1)`), green `rand_range(0x4f, 0xaf)`, blue `rand_range(0x4f, 0xff)`, length `randf(20, 40)`: a type-31 line from there into the beam (0x7f, b, g, 0x4f) | ported ([`update`], `particles::type31`) |
//! | | +0x10 < 50: 0x15f334's alpha → 0x25 by `ticks(1)` a tick (from 0x25 when it was 0xff and 0x15f338 = 0), 0x15f338 = 0x1f00000042; else alpha 0xff | ported (the words; the draw: NOT ported, G-REN-030) |
//! | | no light: every 4th tick (0x15f5cc & 3 = 0) `0x22dbe0(0, 0, pos, 0)` | ported |
//! | | the light: at the beam, radius = clamp(+0x10, 0, 50)·7/50; a type-5 flash at it (`grow = k·4·210000`, size 0, rgb `rand_range` (0xf..0x2f, 0x2f..0x3f, 0x3f..0x7f), `ticks(20)`); its r = `randf(0.8·r, 2·radius/7)`, g = `randf(min(0.8·g, r), r)` | ported |
//! | state 0, not held (or not body 2) | 0x15f320 = 0, 0x15f330 = 0, +0x10 = 0; velocity = 40·dt along the hero's yaw (0x13f3e8); state 1, +0x14 = 0 | ported |
//! | state 1 | f = 40·dt·t/2 + 3 (t = +0x14), +0x14 += 1; the light's radius ×(1 − 0.6·speed), freed below 0.0001 | ported |
//! | | +0x14 > `ticks(60)`, body ≠ 2, or out of [2, 1021]³ after the step → `DeleteMoby(beam)`, beam = 0 (the light stays) | ported |
//! | | `coll_sphere_mobys(f, pos, 0x10, m)` + `0x24a2e0(20, 1, 1, m, pos, list, 0, 0x8b0000, 2, 1)` | ported (`attack::area_hit`) |
//! | | a type-26 glow on the beam (size f·0.5·210000, `rand_range` 0x3f..0x7f / 0x2f..0x3f / 0xf..0x2f at alpha 0x4f, life min(`ticks(60)` − t, `ticks(5)`)) | ported (`projectile::part26`) |
//! | | a type-15 streak (210000, a random direction of `randf(f·dt, 2f·dt)`, the colours at alpha 0x7f → 0x4f7f4f2f, `ticks(45)`, split, texture `def[11][0]`) | ported |
//! | | 8 twinkles (type 69, alpha 0x7f) within f/2 of the beam drifting `randf(f·dt, 2f·dt)`, size `randf(6000, 32000)·f·0.4` | ported (`particles::type69`) |
//! | | 12 twinkles at the beam, still, mode 3 (they follow the beam's pvar point +0xe0), size `randf(80000, 120000)·f` (the third one 180000·f one time in 8), `ticks(2)`, colour 0x7f7f7f | ported (type 69's mode 3: `particles::type69`) |
//! | | +0x2e = 0 | ported |
//! | the screen record 0x15f320..0x15f33c | read by the frame's draw (a full-screen flash; reader not identified [L]) | the words kept ([`ScreenFlash`]); drawn: NOT ported (G-REN-030) |
//!
//! Native `f32`, the game's draws in its order.

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::fx;
use crate::moby_update::services::{pvar as p, World};
use crate::point_lights::PointLight;
use crate::targeting::polar;

/// The update in the level15 class table (level18's 0x2abaa0 is the same code).
pub const UPDATE_FN: u32 = 0x29_edb0;
pub const REFERENCE_LEVEL: u32 = 15;
pub const CLASS: i16 = 0x5f3;
pub const CLASSES: [i16; 1] = [CLASS];
const DT: f32 = 1.0 / 60.0;

/// The beam moby (level15 gp−0x577c = 0x161484; level18 gp−0x57c4): moby + 1, 0 none.
pub const GLOBAL: u32 = 0x16_1484;

/// The screen record's words (boot globals the beam writes).
pub mod scr {
    pub const ON: u32 = 0x15_f320;
    pub const A: u32 = 0x15_f324;
    pub const B: u32 = 0x15_f328;
    pub const FLASH: u32 = 0x15_f330;
    pub const RGBA: u32 = 0x15_f334;
    pub const PACKET_LO: u32 = 0x15_f338;
    pub const PACKET_HI: u32 = 0x15_f33c;
}

pub mod pv {
    pub const VEL: usize = 0x00;
    pub const CHARGE: usize = 0x10;
    pub const FLIGHT: usize = 0x14;
    pub const LIGHT: usize = 0x2c;
    pub const HELD: usize = 0x2e;
}

/// The screen record as the beam leaves it (its draw is G-REN-030).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScreenFlash {
    pub on: u32,
    pub flash: u32,
    pub rgba: u32,
    pub packet: u64,
}

/// The record now.
pub fn screen_flash(svc: &crate::moby_update::services::Services) -> ScreenFlash {
    let u = &svc.units;
    ScreenFlash { on: u.word(scr::ON), flash: u.word(scr::FLASH), rgba: u.word(scr::RGBA), packet: (u.word(scr::PACKET_HI) as u64) << 32 | u.word(scr::PACKET_LO) as u64 }
}

fn global(w: &World) -> Option<MobyId> { (w.svc.units.word(GLOBAL) as usize).checked_sub(1) }
fn set_global(w: &mut World, id: Option<MobyId>) { w.svc.units.set_word(GLOBAL, id.map_or(0, |i| i as u32 + 1)); }
fn v3(a: [f32; 4]) -> [f32; 3] { [a[0], a[1], a[2]] }

/// `0x22dbe0(radius, intensity, pos, rgb)` (level15; level02 `0x240130`): the first free point-light slot (none above
/// the frame load 0.8): colour = the rgb bytes / 128, −1 none.
fn light_alloc(w: &mut World, radius: f32, intensity: f32, pos: [f32; 3], rgb: u32) -> i16 {
    let load = fx::frame_load(w).1;
    let c = [(rgb & 0xff) as f32 * 0.0078125, ((rgb >> 8) & 0xff) as f32 * 0.0078125, ((rgb >> 16) & 0xff) as f32 * 0.0078125];
    w.svc.point_lights.alloc(PointLight { color: c, intensity, pos, radius }, load).map_or(-1, |i| i as i16)
}

/// `0x29ec00(hero, point)`: see the module doc.
pub fn create(w: &mut World, point: [f32; 3]) -> Option<MobyId> {
    let id = w.create_moby(CLASS)?;
    if global(w).is_some() { return Some(id); }
    set_global(w, Some(id));
    {
        let m = w.mm(id);
        if m.pvars.len() < 0x80 { m.pvars.resize(0x80, 0); }
        m.draw_dist = 0xff;
        m.visible = 1;
        m.update_dist = 0xff;
        m.state = 0;
        m.position = [point[0], point[1], point[2], m.position[3]];
    }
    let u = &mut w.svc.units;
    u.set_word(scr::A, 0xff);
    u.set_word(scr::RGBA, 0xff_ffff);
    u.set_word(scr::PACKET_LO, 0x42);
    u.set_word(scr::PACKET_HI, 0x1f);
    u.set_word(scr::B, 0);
    u.set_word(scr::ON, 0);
    u.set_word(scr::FLASH, 0);
    p::set_i32(&mut w.mm(id).pvars, pv::CHARGE, 0);
    let slot = light_alloc(w, 0.0, 0.0, point, 0);
    p::set_i16(&mut w.mm(id).pvars, pv::LIGHT, slot);
    Some(id)
}

/// `0x29ece0(hero, point)` (Giant Clank's state 0x61 up to key 22): see the module doc.
pub fn point(w: &mut World, point: [f32; 3]) {
    if global(w).is_none() { create(w, point); }
    let Some(b) = global(w) else { return };
    if w.m(b).o_class != CLASS { return; }
    let m = w.mm(b);
    m.position = [point[0], point[1], point[2], m.position[3]];
    if m.pvars.len() < 0x80 { m.pvars.resize(0x80, 0); }
    p::set_i16(&mut m.pvars, pv::HELD, 1);
}

/// `0x29ed40()` (the hit intake's body-2 branch): see the module doc.
pub fn end(w: &mut World) {
    let Some(b) = global(w) else { return };
    if w.m(b).o_class != CLASS {
        set_global(w, None);
        return;
    }
    if w.m(b).state != 0 { return; }
    w.svc.units.set_word(scr::ON, 0);
    w.svc.units.set_word(scr::FLASH, 0);
    let slot = p::i16(&w.m(b).pvars, pv::LIGHT);
    if slot != -1 { w.svc.point_lights.free(slot as usize); }
    w.delete_moby(b);
    set_global(w, None);
}

/// Inside [2, 1021]³ (the game's `c.lt.s` pairs; a NaN coordinate passes as in the game).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn in_box(p: [f32; 3]) -> bool { p.iter().all(|&x| !(x < 2.0) && !(1021.0 < x)) }

/// A random direction of length `randf(lo, hi)` (`fun_00213358`: two angles, then the length).
fn rand_polar(w: &mut World, lo: f32, hi: f32) -> [f32; 3] {
    let a = w.rng.rand_angle();
    let b = w.rng.rand_angle();
    let l = w.rng.randf(lo, hi);
    polar(l, a, b)
}

/// Level15 `0x29edb0` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x80 { w.mm(id).pvars.resize(0x80, 0); }
    match w.m(id).state {
        0 => charge(w, id),
        1 => fly(w, id),
        _ => p::set_i16(&mut w.mm(id).pvars, pv::HELD, 0),
    }
}

fn charge(w: &mut World, id: MobyId) {
    let held = p::i16(&w.m(id).pvars, pv::HELD) != 0;
    if held && w.body() == crate::hero::bodies::body::GIANT {
        let mut t = p::i32(&w.m(id).pvars, pv::CHARGE);
        if t == 0 {
            w.svc.units.set_word(scr::FLASH, 1);
            w.svc.units.set_word(scr::RGBA, 0x00ff_ff00);
        }
        t += 1;
        p::set_i32(&mut w.mm(id).pvars, pv::CHARGE, t);
        let t30 = w.ticks(30) as f32;
        let k = (t as f32 / t30).min(1.0);
        let k2 = k * k;
        let pos = v3(w.m(id).position);
        let mut i = 0;
        while i < (k2 * 10.0) as i32 {
            let x = w.rng.randf(-1.0, 1.0);
            let y = w.rng.randf(-1.0, 1.0);
            let z = w.rng.randf(-1.0, 1.0);
            i += 1;
            let g = w.rng.rand_range(0x4f, 0xaf);
            let b = w.rng.rand_range(0x4f, 0xff);
            let (g, b) = (if g < 0x100 { g } else { 0xff }, if b < 0x100 { b } else { 0xff });
            let l = w.rng.randf(20.0, 40.0);
            let v = crate::hero::guns::with_len([x, y, z], l);
            let at = [v[0] + pos[0], v[1] + pos[1], v[2] + pos[2], 0.0];
            let rgba = (b as u32) << 16 | (g as u32) << 8 | 0x7f00_0000 | 0x4f;
            *w.svc.fx.part_spawns.entry(crate::particles::type31::TYPE).or_default() += 1;
            if let Some(sys) = w.particles.as_deref_mut() {
                if crate::particles::type31::spawn(sys, rgba, at, id, pos).is_none() { w.svc.fx.part_failed += 1; }
            }
        }
        if t < 50 {
            let rgba = w.svc.units.word(scr::RGBA);
            let mut a = (rgba >> 24) as i32;
            let packet = (w.svc.units.word(scr::PACKET_HI) as u64) << 32 | w.svc.units.word(scr::PACKET_LO) as u64;
            if packet == 0 && a == 0xff { a = 0x25; }
            w.svc.units.set_word(scr::PACKET_LO, 0x42);
            w.svc.units.set_word(scr::PACKET_HI, 0x1f);
            let one = w.ticks(1);
            a = if a < 0x25 - one { a + one } else { 0x25 };
            w.svc.units.set_word(scr::RGBA, rgba & 0x00ff_ff00 | (a as u32) << 24);
        } else {
            let rgba = w.svc.units.word(scr::RGBA);
            w.svc.units.set_word(scr::RGBA, rgba | 0xff00_0000);
        }
        let slot = p::i16(&w.m(id).pvars, pv::LIGHT);
        if slot == -1 {
            if w.counter & 3 == 0 {
                let s = light_alloc(w, 0.0, 0.0, pos, 0);
                p::set_i16(&mut w.mm(id).pvars, pv::LIGHT, s);
            }
            p::set_i16(&mut w.mm(id).pvars, pv::HELD, 0);
            return;
        }
        let i = slot as usize;
        let mut l = w.svc.point_lights.slots.get(i).copied().flatten().unwrap_or(PointLight { color: [0.0; 3], intensity: 0.0, pos, radius: 0.0 });
        l.pos = pos;
        l.radius = (t.clamp(0, 50) as f32 * 7.0) / 50.0;
        let k = (t as f32 / t30).min(1.0);
        let r8 = w.rng.rand_range(0xf, 0x2f) as u32 & 0xff;
        let g8 = w.rng.rand_range(0x2f, 0x3f) as u32 & 0xff;
        let b8 = w.rng.rand_range(0x3f, 0x7f) as u32 & 0xff;
        let life = w.ticks(20);
        fx::part05(w, k * 4.0 * 210_000.0, 0.0, [l.pos[0], l.pos[1], l.pos[2], 0.0], [r8, g8, b8], life);
        l.color[0] = w.rng.randf(l.color[0] * 0.8, (l.radius + l.radius) / 7.0);
        let lo = (l.color[1] * 0.8).min(l.color[0]);
        l.color[1] = w.rng.randf(lo, l.color[0]);
        w.svc.point_lights.set(i, l);
        p::set_i16(&mut w.mm(id).pvars, pv::HELD, 0);
        return;
    }
    // Not held (or not Giant Clank): off it goes along the hero's yaw.
    w.svc.units.set_word(scr::ON, 0);
    w.svc.units.set_word(scr::FLASH, 0);
    let yaw = w.hero.rot[2].to_f32();
    let m = w.mm(id);
    p::set_i32(&mut m.pvars, pv::CHARGE, 0);
    p::set_ff(&mut m.pvars, pv::VEL, yaw.cos() * DT * 40.0);
    p::set_ff(&mut m.pvars, pv::VEL + 4, yaw.sin() * DT * 40.0);
    p::set_ff(&mut m.pvars, pv::VEL + 8, 0.0);
    m.state = 1;
    p::set_i32(&mut m.pvars, pv::FLIGHT, 0);
    p::set_i16(&mut m.pvars, pv::HELD, 0);
}

fn fly(w: &mut World, id: MobyId) {
    let t0 = p::i32(&w.m(id).pvars, pv::FLIGHT);
    let f = DT * 40.0 * t0 as f32 * 0.5 + 3.0;
    let t = t0 + 1;
    p::set_i32(&mut w.mm(id).pvars, pv::FLIGHT, t);
    let slot = p::i16(&w.m(id).pvars, pv::LIGHT);
    if slot != -1 {
        let i = slot as usize;
        if let Some(mut l) = w.svc.point_lights.slots.get(i).copied().flatten() {
            l.radius *= crate::moby_update::creature::SPEED * -0.6 + 1.0;
            w.svc.point_lights.set(i, l);
            if l.radius < f32::from_bits(0x38d1_b717) {
                w.svc.point_lights.free(i);
                p::set_i16(&mut w.mm(id).pvars, pv::LIGHT, -1);
            }
        }
    }
    let gone = |w: &mut World| {
        if let Some(b) = global(w) { w.delete_moby(b); }
        set_global(w, None);
    };
    if w.ticks(60) < t || w.body() != crate::hero::bodies::body::GIANT {
        gone(w);
        return;
    }
    let vel = { let pv_ = &w.m(id).pvars; [p::ff(pv_, pv::VEL), p::ff(pv_, pv::VEL + 4), p::ff(pv_, pv::VEL + 8)] };
    let pos = { let q = v3(w.m(id).position); [q[0] + vel[0], q[1] + vel[1], q[2] + vel[2]] };
    { let m = w.mm(id); m.position = [pos[0], pos[1], pos[2], m.position[3]]; }
    if !in_box(pos) {
        gone(w);
        return;
    }
    let pos4 = w.m(id).position;
    crate::moby_update::creature::attack::area_hit(w, f, pos4, id, 20.0, 1.0, 1.0, None, 0x8b_0000, 2, 1);
    let (t60, t5) = (w.ticks(60), w.ticks(5));
    let life = if t60 - t < t5 { t60 - t } else { t5 };
    let r = w.rng.rand_range(0x3f, 0x7f) as u32;
    let g = w.rng.rand_range(0x2f, 0x3f) as u32;
    let b = w.rng.rand_range(0xf, 0x2f) as u32;
    crate::moby_update::creature::projectile::part26(w, f * 0.5 * 210_000.0, id, r << 16 | g << 8 | 0x4f00_0000 | b, life);
    let v = rand_polar(w, f * DT, (f + f) * DT);
    let r = w.rng.rand_range(0x3f, 0x7f) as u32;
    let g = w.rng.rand_range(0x2f, 0x3f) as u32;
    let b = w.rng.rand_range(0xf, 0x2f) as u32;
    let life = w.ticks(45);
    let def = w.particles.as_deref().map_or(-1, |s| s.def_first(11) as i32);
    let a = crate::particles::type15::Spawn { size: 210_000.0, pos: pos4, vel: [v[0], v[1], v[2], 0.0], c1: r << 16 | g << 8 | 0x7f00_0000 | b, c2: 0x4f7f_4f2f, life, split: 1, def, blend: -1 };
    fx::part15(w, &a);
    for _ in 0..8 {
        let v1 = rand_polar(w, f * DT, (f + f) * DT);
        let v2 = rand_polar(w, 0.0, f * 0.5);
        let at = [v2[0] + pos[0], v2[1] + pos[1], v2[2] + pos[2], pos4[3]];
        if let Some(i) = part69(w, at, [v1[0], v1[1], v1[2], 0.0], 0x7f, id) {
            let s = w.rng.randf(6000.0, 32000.0) * f * 0.4;
            if let Some(sys) = w.particles.as_deref_mut() { crate::particles::rec::set_ff(&mut sys.pool.recs[i], 0xc, s); }
        }
    }
    for k in 0..12 {
        let Some(i) = part69(w, pos4, [0.0; 4], 0x7f, id) else { continue };
        let size = if k == 2 && w.rng.randi(8) == 0 { 180_000.0 } else { w.rng.randf(80_000.0, 120_000.0) };
        let t2 = w.ticks(2);
        if let Some(sys) = w.particles.as_deref_mut() {
            use crate::particles::rec;
            let r = &mut sys.pool.recs[i];
            rec::set_ff(r, 0xc, size * f);
            rec::set_i16(r, 0xa, t2 as i16);
            rec::set_i16(r, 0x36, 3);
            rec::set_u32(r, 0x38, 0x7f_7f7f);
            rec::set_ff(r, 0x30, 1.0 / t2 as f32);
        }
    }
    p::set_i16(&mut w.mm(id).pvars, pv::HELD, 0);
}

/// Level15 `0x2643e0(pos, vel, alpha, moby)` (`particles::type69`): the record, counted.
fn part69(w: &mut World, pos: [f32; 4], vel: [f32; 4], alpha: i32, moby: MobyId) -> Option<usize> {
    *w.svc.fx.part_spawns.entry(crate::particles::type69::TYPE).or_default() += 1;
    let Some(sys) = w.particles.as_deref_mut() else {
        // No particle system: the spawner's three draws.
        w.rng.randf(8000.0, 50000.0);
        w.rng.randi(0x100);
        w.rng.rand_range(10, 0x1e);
        return None;
    };
    let r = crate::particles::type69::spawn(sys, w.rng, pos, vel, alpha, moby as u32 + 1);
    if r.is_none() { w.svc.fx.part_failed += 1; }
    r
}
