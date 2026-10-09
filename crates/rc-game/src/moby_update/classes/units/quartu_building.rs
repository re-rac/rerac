//! **Quartu's buildings, classes 148 and 255** (level15 `0x2a7880`, one code; census U539, 7 + 4 placed). Buildings
//! that only Giant Clank can knock down: at first updated always (draw distance 0x80, twice the size; gone already
//! when its death bit is set once planet 16 is unlocked, 0x13dd50); after that only while Giant Clank plays. A hit
//! record of the heavy kind 0x80000 on it brings it down: 25 burning bits (`SpawnDebrisMoby`, classes 200 / 205 / 206 /
//! 210, out from the hit with the push, three times as far round a 148), a big piece 201 off a 148's top, the blast,
//! 20 smoke puffs (type 16), its death bits, gone. Read from the level15 decomp and disassembly; native `f32`.
//!
//! **The walls, class 154** (`0x2aa280`, U540, 7 placed) fall the same way but along their length: the bits and the
//! smoke at `randf(yaw, 90°)` (the game's range) times `randf(−3, 3)`, the bits living 100..160 ticks; no size change.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2a7880` | 148 / 255 | [`update`] |
//! | `0x2aa280` | 154 | [`wall_update`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::gunship::spawn_ember;
use crate::moby_update::creature::{self as c, fx, DT};
use crate::moby_update::World;
use crate::particles::tween_color;

pub const REFERENCE_LEVEL: u32 = 15;
pub const UPDATE_FN: u32 = 0x2a_7880;
pub const CLASSES: [i16; 2] = [148, 255];
pub const WALL_FN: u32 = 0x2a_a280;
pub const WALL_CLASSES: [i16; 1] = [154];

const BIG: i16 = 0x94;
/// Level15 0x161568: the burning bits' classes; 0x161578: the smoke kinds.
const BITS: [i16; 4] = [0xc8, 0xcd, 0xce, 0xd2];
const SMOKE: [i16; 3] = [0, 1, 2];
const GIANT: u8 = 2;
const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 10.0, flash2: 5.0, flash_dist: 100_000.0, scale: 3.0, light: 15.0, streaks: 20, sparks: 3, puffs: 4, debris: 1, sound: 0, shake: true };

fn tween(f: f32, a: u32, b: u32) -> u32 { tween_color(f.to_bits(), a, b) }

/// Level15 `0x2a7880` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.body() != GIANT && w.m(id).state == 1 { return; }
    let rec = heavy_hit(w, id);
    w.mm(id).hit_slot = 0xff;
    let Some(i) = rec else {
        if w.m(id).state == 0 {
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.state = 1;
            m.draw_dist = 0x80;
            m.scale += m.scale;
            let (lvl, sid) = (w.svc.level, w.m(id).spawn_id);
            let unlocked = w.svc.interact.game.planet_unlocked.get(16).copied().unwrap_or(0) != 0;
            if unlocked && w.svc.save.death.contains(&(lvl, sid)) { w.delete_moby(id); }
        }
        return;
    };
    let r = w.svc.hits.records[i];
    let class = w.m(id).o_class;
    let pos = c::pos(w, id);
    let dir = r.dir.map(|x| x.to_f32());
    let push = c::set_len3([dir[0], dir[1], dir[2], 0.0], DT * 10.0);
    let push2 = push.map(|x| x * 2.0);
    let hp = r.pos.map(|x| x.to_f32());
    let centre = if c::len3(hp) == 0.0 { [pos[0], pos[1], pos[2] + 6.0, pos[3]] } else { hp };
    for _ in 0..25 {
        let mut rr = w.rng.randf(0.5, 1.5);
        let a = w.rng.rand_angle();
        if class == BIG { rr *= 3.0; }
        let z = w.rng.randf(1.0, 7.0);
        let p = c::add([a.cos() * rr, a.sin() * rr, z, 0.0], pos);
        let mut v = c::set_len3(c::sub(centre, p), DT * 5.0);
        v = c::add(v, push2);
        v[2] += DT * 3.0;
        let k = w.rng.randf(0.75, 1.25);
        v = v.map(|x| x * k);
        let bit = BITS[w.rng.randi(4) as usize];
        let scale = w.rng.randf(0.1, 0.3);
        let life = w.rng.rand_range(0x3c, 100);
        let g = w.rng.randf(3.0, 7.0);
        spawn_ember(w, scale, g, 0.15, 2.0, p, v, bit, life, 1);
    }
    if class == BIG {
        let p = [pos[0], pos[1], pos[2] + 10.0, pos[3]];
        let mut v = c::set_len3(c::sub(centre, p), DT * 5.0);
        v = c::add(v, push2);
        v[2] += DT * 3.0;
        let k = w.rng.randf(0.75, 1.25);
        v = v.map(|x| x * k);
        let life = w.rng.rand_range(0x3c, 100);
        if let Some(m) = spawn_ember(w, 1.0, 5.0, 1.0, 2.0, p, v, 0xc9, life, 1) {
            let r = w.m(m).rotation.map(|x| x * f32::from_bits(0x3eb3_3333));
            w.mm(m).rotation = r;
        }
    }
    fx::beam_explosion(w, &BLAST, Some(id), centre);
    for _ in 0..20 {
        let mut rr = w.rng.randf(0.0, 1.5);
        let a = w.rng.rand_angle();
        if class == BIG { rr *= 3.0; }
        let mut p = c::add([a.cos() * rr, a.sin() * rr, 0.0, 0.0], pos);
        p[2] += w.rng.randf(1.0, 5.5);
        let mut v = c::set_len3(c::sub(centre, p), DT * 5.0);
        v = c::add(v, push);
        let k = w.rng.randf(0.75, 1.25);
        v = v.map(|x| x * k);
        let f1 = w.rng.randf(0.25, 1.0);
        let c1 = tween(f1, 0x7f00_0000, 0x7f18_2030);
        let f2 = w.rng.randf(0.5, 1.0);
        let c2 = tween(f2, 0, 0x5f_5f5f);
        let size = w.rng.randf(1.0, 3.0);
        let life = w.ticks(0xb4);
        let kind = SMOKE[w.rng.randi(3) as usize];
        let s = crate::particles::type16::Spawn { size: size * 210_000.0, pos: p, vel: v, c1, c2, life, kind };
        if let Some(i) = crate::moby_update::creature::projectile::part16(w, &s) {
            if let Some(r) = fx::rec_mut(w, i) { crate::particles::rec::set_ff(r, 0x2c, pos[2] - 0.5); }
        }
    }
    set_death_bits(w, id, 0, -1);
    w.delete_moby(id);
}

/// The heavy hit on `id` (a record of kind 0x80000), if any.
fn heavy_hit(w: &World, id: MobyId) -> Option<usize> {
    (0..w.svc.hits.records.len().min(0x40)).find(|&i| { let r = &w.svc.hits.records[i]; r.target == id && r.flags & 0x8_0000 != 0 })
}

/// Level15 `0x2aa280` (module doc).
pub fn wall_update(w: &mut World, id: MobyId) {
    if w.body() != GIANT && w.m(id).state == 1 { return; }
    let rec = heavy_hit(w, id);
    w.mm(id).hit_slot = 0xff;
    let Some(i) = rec else {
        if w.m(id).state == 0 {
            let m = w.mm(id);
            m.draw_dist = 0x80;
            m.update_dist = 0xff;
            m.state = 1;
            let (lvl, sid) = (w.svc.level, w.m(id).spawn_id);
            let unlocked = w.svc.interact.game.planet_unlocked.get(16).copied().unwrap_or(0) != 0;
            if unlocked && w.svc.save.death.contains(&(lvl, sid)) { w.delete_moby(id); }
        }
        return;
    };
    let r = w.svc.hits.records[i];
    let pos = c::pos(w, id);
    let yaw = w.m(id).rotation[2];
    let dir = r.dir.map(|x| x.to_f32());
    let push = c::set_len3([dir[0], dir[1], dir[2], 0.0], DT * 10.0);
    let push2 = push.map(|x| x * 2.0);
    let hp = r.pos.map(|x| x.to_f32());
    let centre = if c::len3(hp) == 0.0 { [pos[0], pos[1], pos[2] + 6.0, pos[3]] } else { hp };
    let along = |w: &mut World| -> [f32; 2] {
        let a = w.rng.randf(yaw, std::f32::consts::FRAC_PI_2);
        let x = a.cos() * w.rng.randf(-3.0, 3.0);
        let b = w.rng.randf(yaw, std::f32::consts::FRAC_PI_2);
        let y = b.sin() * w.rng.randf(-3.0, 3.0);
        [x, y]
    };
    for _ in 0..25 {
        let [x, y] = along(w);
        let z = w.rng.randf(1.0, 7.0);
        let p = c::add([x, y, z, 0.0], pos);
        let mut v = c::set_len3(c::sub(centre, p), DT * 5.0);
        v = c::add(v, push2);
        v[2] += DT * 3.0;
        let k = w.rng.randf(0.75, 1.25);
        v = v.map(|x| x * k);
        let bit = BITS[w.rng.randi(4) as usize];
        let scale = w.rng.randf(0.1, 0.3);
        let life = w.rng.rand_range(100, 0xa0);
        let g = w.rng.randf(3.0, 7.0);
        spawn_ember(w, scale, g, 0.15, 2.0, p, v, bit, life, 1);
    }
    fx::beam_explosion(w, &BLAST, Some(id), centre);
    for _ in 0..20 {
        let [x, y] = along(w);
        let mut p = c::add([x, y, 0.0, 0.0], pos);
        p[2] += w.rng.randf(1.0, 5.5);
        let mut v = c::set_len3(c::sub(centre, p), DT * 5.0);
        v = c::add(v, push);
        let k = w.rng.randf(0.75, 1.25);
        v = v.map(|x| x * k);
        let f1 = w.rng.randf(0.25, 1.0);
        let c1 = tween(f1, 0x7f00_0000, 0x7f18_2030);
        let f2 = w.rng.randf(0.5, 1.0);
        let c2 = tween(f2, 0, 0x5f_5f5f);
        let size = w.rng.randf(1.0, 3.0);
        let life = w.ticks(0xb4);
        let kind = SMOKE[w.rng.randi(3) as usize];
        let s = crate::particles::type16::Spawn { size: size * 210_000.0, pos: p, vel: v, c1, c2, life, kind };
        if let Some(i) = crate::moby_update::creature::projectile::part16(w, &s) {
            if let Some(r) = fx::rec_mut(w, i) { crate::particles::rec::set_ff(r, 0x2c, pos[2] - 0.5); }
        }
    }
    set_death_bits(w, id, 0, -1);
    w.delete_moby(id);
}
