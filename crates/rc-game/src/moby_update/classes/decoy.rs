//! **The Decoy Glove's decoy** (class 203 = 0xcb, the gold one 1900 = 0x76c; one update on every level: level01
//! `0x2d9fe8`), its create `0x2d9228`, release `0x2d94f0`, landing preview `0x2d9760` (a copy of the bomb's, drawn with
//! the same reticle), the push `0x2d9d08`, the surface pop `0x2d9f90` / `0x2d90a8`. The glove's side is the shared
//! glove update (`crate::hero::gloves`, item 25). docs/plan/hero_gameplay.md §13.
//!
//! **How creatures go for it.** Nothing here tells an enemy anything: the decoys are in the gadget list `0x1b0cb0`
//! (at most 6: [`cap_count`]) and the creatures' own target search `0x274b78` (`crate::moby_update::creature::target`,
//! ported before) takes a decoy **in state 3** that is nearer than its range instead of Ratchet. So every creature
//! built on that search (577, 572 / 865 / 866, 459, …) turns on it, walks to it and attacks it with its usual hit
//! (a hit record for the decoy); the decoy takes the hits (mask 0x30001) off its health (4, 8 gold) and pops.
//!
//! **States** (+0x20):
//! * **0 / 1 held** (the glove places it every tick; hidden while the first-person camera is up): deleted when the
//!   hand no longer holds the Decoy Glove (or Ratchet is not on foot). The landing preview runs while held.
//! * **2 flying**: grows to the class scale (0.02 a tick), gravity 9·dt², pushed apart from the other decoys within
//!   1.8, moved by `walker::move_collide` (`0x26d610`: up 0.2, radius 0.2; Ratchet's collision off while he aims in
//!   first person). On a wall or floor (move bit 0) a sphere test (0.23, 0.2 up) gives the face: the velocity
//!   reflected ×0.2 (×1.05 off a moby's primitive), pushed away from Ratchet within 1.8 when falling; slower than 0.01
//!   a tick on ground less than 35° from flat it **lands** (a moby primitive only after 240 ticks of contact): the
//!   inflate sequence 1, collision on, state 3, tilted to the face. Otherwise a bounce sound every 20 ticks and a
//!   jitter of ±dt on the velocity. A surface of kinds 0, 1, 3, 8, 0xb, 0xc, 0xd (water, lava, …) pops it at once.
//! * **3 standing** (the target): hits (MobyGetHitMessage mask 0x30001; one attacker every 30 ticks) → class sound 0,
//!   the red flash (4 ticks in, 15 out), damage off the health; below 0 it **bursts** (`SpawnBeamExplosion`: flashes
//!   4 / 3 beyond 9, light 12, sound 2, shake, debris 1) and is deleted. It wobbles (sequence 2 at a random speed
//!   0.7..1.05 each wrap), falls to the ground (9.8·dt²) and tilts to it, and Ratchet stuck on top of it (0x13f532)
//!   pushes it out from under him (a spring to 1.4 away, `move_collide` up 0.5 radius 0.4, gravity 24·dt²).
//! * **4** (released into a wall: set by the release): as 3, growing, for 30 ticks, then deflates: **5** (sequence 0
//!   over 60 ticks) → **6** shrinking 2 % a tick, deleted below a tenth of the class scale. The oldest decoy past the
//!   cap goes straight to 6.
//! * Out of the world box [2, 1021]³: deleted.
//!
//! **Pvars** (0x70): +0x00 the hit-flash record (`creature::flash`), +0x10 velocity, +0x20 / +0x30 the preview's point
//! and normal (the reticle), +0x40 the owner (the glove: 1 here), +0x44 s16 timer (the flight's ground contacts; the
//! 600 / 30-tick timers), +0x46 s16 a new decoy in the glove, +0x48 health, +0x4c / +0x50 the tilt it eases to,
//! +0x54 age (ticks), +0x58 the last attacker (+1), +0x5c its 30-tick cooldown, +0x60 the push's spring speed, +0x64
//! the push's fall speed, +0x68 pushed, +0x6c s16 / +0x6e the 100-tick rule of a type-0x101 hit, +0x6f the bounce
//! sound timer.
//!
//! Native `f32`; the rand draws are the game's. **Inferred [L]**: the list 0x1b0cb0 is the table's decoys not in
//! states 5 / 6 (the port has no writer of the game's list; the order of equally old decoys could differ); the owner
//! is "the hand holds the Decoy Glove" (the glove is not a table moby); a decoy on a moving platform does not ride
//! it (`FUN_002752c0`, 0x13f64c: never set in the port); the burst's debris (`0x2c4c20`) and the pop's type-5 puffs
//! are records only (particle type 5: G-PRT-001); the gold decoy (0x13e539: its burst's colour shift and area hit) reads the mirrored gold table.

use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::creature::{self as c, flash, fx, walker, V};
use crate::moby_update::services::{pvar as p, HitTemplate, World};
use crate::ps2v::Pf;
use crate::targeting as tg;
use std::f32::consts::PI;

pub const UPDATE_FN: u32 = 0x2d9fe8;
pub const CLASS: i16 = 0xcb;
pub const GOLD_CLASS: i16 = 0x76c;
pub const CLASSES: [i16; 2] = [CLASS, GOLD_CLASS];
/// The Decoy Glove (item 25) and its hand class.
pub const DECOY_GLOVE: i32 = 25;
pub const GLOVE_CLASS: i16 = 0x232;

/// States (+0x20).
pub const HELD: u8 = 1;
pub const FLYING: u8 = 2;
pub const STANDING: u8 = 3;
pub const BURST_WALL: u8 = 4;
pub const DEFLATE: u8 = 5;
pub const SHRINK: u8 = 6;

/// Pvar offsets.
pub mod pv {
    pub const FLASH: usize = 0x00;
    pub const VEL: usize = 0x10;
    pub const POINT: usize = 0x20;
    pub const NORMAL: usize = 0x30;
    pub const OWNER: usize = 0x40;
    pub const TIMER: usize = 0x44;
    pub const NEW_HELD: usize = 0x46;
    pub const HEALTH: usize = 0x48;
    pub const TILT_X: usize = 0x4c;
    pub const TILT_Y: usize = 0x50;
    pub const AGE: usize = 0x54;
    pub const ATTACKER: usize = 0x58;
    pub const COOLDOWN: usize = 0x5c;
    pub const PUSH_V: usize = 0x60;
    pub const PUSH_FALL: usize = 0x64;
    pub const PUSHED: usize = 0x68;
    pub const T6C: usize = 0x6c;
    pub const B6E: usize = 0x6e;
    pub const SOUND_T: usize = 0x6f;
    pub const SIZE: usize = 0x70;
}

const DT: f32 = c::DT;
const DT2: f32 = c::DT2;

/// The decoys in the list 0x1b0cb0 (the table's, in index order; see the module doc).
fn in_list(m: &crate::moby_runtime::Moby) -> bool { CLASSES.contains(&m.o_class) && !m.is_deleted() && m.state != DEFLATE && m.state != SHRINK }

/// The velocity the glove's aim solved (`0x2ece98` into +0x10).
pub fn set_velocity(m: &mut crate::moby_runtime::Moby, v: [f32; 3]) {
    if m.pvars.len() < pv::SIZE { m.pvars.resize(pv::SIZE, 0); }
    p::set_v4f(&mut m.pvars, pv::VEL, [v[0], v[1], v[2], 0.0]);
}

/// `0x2d9228(glove, point, vel)`: the new decoy (the caller's `CreateMoby`): update / draw distances 0xff, visible,
/// state 0 at the hand point, the velocity word = the point (the glove's aim writes it before the decoy's next update),
/// the owner, half the class scale, health 4 (8 gold), no collision (+0x94 = 0, +0x98 = −1), a random yaw
/// `randf(−π, π)`, Ratchet's lighting, the red flash record (0x80, 4 in, 15 out); hidden in first person or with the
/// hand hidden.
pub fn init_held(table: &mut MobyTable, id: MobyId, rng: &mut crate::rng::Rng, point: [f32; 3], gold: bool, hidden: bool) {
    let hero_light = table.hero().map(|h| (table.mobys[h].light, table.mobys[h].ambient));
    let m = &mut table.mobys[id];
    m.update_dist = 0xff;
    m.draw_dist = 0xff;
    m.visible = 1;
    m.state = 0;
    m.position = [point[0], point[1], point[2], m.position[3]];
    if m.pvars.len() < pv::SIZE { m.pvars.resize(pv::SIZE, 0); }
    let pvs = &mut m.pvars;
    p::set_v4f(pvs, pv::VEL, [point[0], point[1], point[2], 0.0]);
    p::set_i32(pvs, pv::OWNER, 1);
    p::set_i16(pvs, pv::TIMER, 0);
    p::set_i32(pvs, pv::AGE, 0);
    m.scale *= 1.0 - 0.5;
    p::set_ff(pvs, pv::HEALTH, ((gold as i32) as f32 + 1.0) * 4.0);
    m.has_collision = false;
    m.coll_disable = 0xffff_ffff;
    m.rotation[2] = rng.randf(-PI, PI);
    p::set_i32(pvs, pv::ATTACKER, 0);
    p::set_i32(pvs, pv::COOLDOWN, 0);
    if let Some((l, a)) = hero_light {
        m.light = l;
        m.ambient = a;
    }
    // The flash record: timer 0, in 4, out 15, red.
    p::set_i16(pvs, pv::FLASH + 0xc, 4);
    p::set_i16(pvs, pv::FLASH, 0);
    pvs[pv::FLASH + 8] = 0;
    pvs[pv::FLASH + 9] = 0;
    p::set_i16(pvs, pv::FLASH + 0xe, 0xf);
    pvs[pv::FLASH + 7] = 0x80;
    if hidden { m.mode |= 0x41; }
}

/// `0x2d9228`'s cap: while more than 5 decoys are listed, the oldest (largest age) in a state below 5 leaves the
/// list and shrinks away (state 6); then the new one is listed.
pub fn cap_count(table: &mut MobyTable, new: MobyId) {
    loop {
        let listed: Vec<MobyId> = table.mobys.iter().enumerate().filter(|(i, m)| *i != new && in_list(m)).map(|(i, _)| i).collect();
        if listed.len() <= 5 { break; }
        let mut best: Option<(MobyId, i32)> = None;
        for &i in &listed {
            let m = &table.mobys[i];
            let age = p::i32(&m.pvars, pv::AGE);
            if m.state < 5 && best.is_none_or(|(_, a)| a <= age) { best = Some((i, age)); }
        }
        let Some((i, _)) = best else { break };
        table.mobys[i].state = SHRINK;
    }
}

/// The release's line (`0x2d94f0`): from Ratchet at the point's height to 0.2 past the point (the game extends the
/// glove's hand point in place, so the next decoy is made there).
pub fn release_line(from: [f32; 3], point: [f32; 3]) -> [f32; 3] {
    let d = [point[0] - from[0], point[1] - from[1], point[2] - from[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if l == 0.0 { from } else { std::array::from_fn(|k| from[k] + d[k] * (l + 0.2) / l) }
}

/// `0x2d94f0(point, decoy, vel)`: the release from the hand point (the velocity kept); the line of [`release_line`]
/// (the world and the mobys, Ratchet ignored: `hit`) — none → state 2 (flying); a hit → it stands there, velocity 0,
/// state 4 with a 30-tick timer, tilted 10 % toward the face.
pub fn release(m: &mut crate::moby_runtime::Moby, point: [f32; 3], hit: Option<([f32; 3], [f32; 3])>) {
    m.position = [point[0], point[1], point[2], m.position[3]];
    match hit {
        None => m.state = FLYING,
        Some((hp, n)) => {
            m.position = [hp[0], hp[1], hp[2], m.position[3]];
            p::set_v4f(&mut m.pvars, pv::VEL, [0.0; 4]);
            m.state = BURST_WALL;
            p::set_i16(&mut m.pvars, pv::TIMER, 30);
            let (tx, ty) = tilt(m.rotation[2], n);
            p::set_ff(&mut m.pvars, pv::TILT_X, tx);
            p::set_ff(&mut m.pvars, pv::TILT_Y, ty);
            ease_tilt(m);
        }
    }
}

/// The tilt that puts a face's normal `n` up under a moby of yaw `yaw` (`+0x4c`, `+0x50`): `n` turned by −yaw into
/// `(a, b, c)`, `−atan2(b, √(a² + c²))` and `atan2(a, c)`.
fn tilt(yaw: f32, n: [f32; 3]) -> (f32, f32) {
    let (co, si) = (yaw.cos(), yaw.sin());
    let a = n[0] * co + n[1] * si;
    let b = n[1] * co - n[0] * si;
    let cz = n[2];
    (-c::atan((a * a + cz * cz).sqrt(), b), c::atan(cz, a))
}

/// The rotation's x / y eased 10 % a tick toward the tilt.
fn ease_tilt(m: &mut crate::moby_runtime::Moby) {
    let tx = p::ff(&m.pvars, pv::TILT_X);
    let ty = p::ff(&m.pvars, pv::TILT_Y);
    m.rotation[0] = c::add_rot(m.rotation[0], c::sub_rot(tx, m.rotation[0]) * 0.1);
    m.rotation[1] = c::add_rot(m.rotation[1], c::sub_rot(ty, m.rotation[1]) * 0.1);
}

/// Whether the hand holds the Decoy Glove (`0x1403e0`'s class is 0x232).
fn glove_in_hand(w: &World) -> bool { w.hero.items.slot.item.as_ref().is_some_and(|m| m.o_class == GLOVE_CLASS) }

/// `0x2d9fe8`.
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { w.mm(id).pvars.resize(pv::SIZE, 0); }
    let pos = c::pos(w, id);
    if !(0..3).all(|k| (2.0..=1021.0).contains(&pos[k])) {
        w.delete_moby(id);
        return;
    }
    let st = w.m(id).state;
    if (!glove_in_hand(w) || w.body() != 0) && st == HELD {
        w.delete_moby(id);
        return;
    }
    let age = c::pi32(w, id, pv::AGE);
    c::set_pi32(w, id, pv::AGE, age + 1);
    {
        let fp = w.hero.f13f5 != 0;
        let m = w.mm(id);
        if fp && m.state <= 1 { m.mode |= 0x41; } else { m.mode &= !0x41; }
    }
    // The landing preview (the owner is the glove in hand, no new decoy in it, a velocity).
    if glove_in_hand(w) && c::pi16(w, id, pv::NEW_HELD) == 0 && 0.0 < c::len3(c::pv4(w, id, pv::VEL)) { preview(w, id); }
    match w.m(id).state {
        0 => {
            w.mm(id).state = HELD;
            c::set_pi16(w, id, pv::TIMER, 0);
            w.mm(id).coll_disable = 0xffff_ffff;
            c::set_pu8(w, id, pv::B6E, 0);
            c::set_pi16(w, id, pv::T6C, 0);
        }
        HELD => {
            c::set_pi16(w, id, pv::TIMER, 0);
            // The owner alive and the glove in hand: nothing (the glove places it).
        }
        FLYING => fly(w, id),
        STANDING | BURST_WALL => stand(w, id),
        DEFLATE => {
            let r = c::dec_timer_pvar_s16(w, id, pv::TIMER);
            let wrapped = w.m(id).anim.flags & 2 != 0;
            if wrapped { w.mm(id).anim.speed = 0.0; }
            if r != 0 || wrapped { w.mm(id).state = SHRINK; }
        }
        SHRINK => {
            let m = w.mm(id);
            m.scale -= m.scale * 0.02;
            let class_scale = w.class_scale(w.m(id).o_class).to_f32();
            if w.m(id).scale < class_scale * 0.1 { w.delete_moby(id); }
        }
        _ => w.mm(id).state = 0,
    }
}

/// Grows toward the class scale by 0.02 a tick.
fn grow(w: &mut World, id: MobyId) {
    let full = w.class_scale(w.m(id).o_class).to_f32();
    let m = w.mm(id);
    if m.scale < full { m.scale += 0.02; }
}

/// State 2 (module doc). The move writes the displacement it made back into the velocity (`0x26d610` takes the
/// pvar itself).
fn fly(w: &mut World, id: MobyId) {
    grow(w, id);
    let pos = c::pos(w, id);
    let mut vel = c::pv4(w, id, pv::VEL);
    vel[2] -= DT2 * 9.0;
    // Apart from the other listed decoys that are out of the glove.
    let others: Vec<MobyId> = w.table.mobys.iter().enumerate().filter(|(i, m)| *i != id && in_list(m) && 1 < m.state).map(|(i, _)| i).collect();
    for o in others {
        let mut d = c::sub(pos, w.m(o).position);
        d[2] = 0.0;
        d[3] = 0.0;
        let l = c::len3(d);
        if l < 1.8 { vel = c::add(vel, c::scale(d, (1.8 - l) * 0.015)); }
    }
    let first_person = w.hero.state == 0x1e;
    let hero = w.hero_moby;
    if first_person { if let Some(h) = hero { w.mm(h).has_collision = false; } }
    let r = walker::move_collide(w, id, 0.2, 0.2, 0.0, &mut vel, 0);
    if r & 1 != 0 {
        let p = c::pos(w, id);
        let centre = [p[0], p[1], p[2] + 0.2, p[3]];
        if let Some(o) = w.coll_sphere(pv(centre), Pf::f(f32::from_bits(0x3e6b_851f)), 0, Some(id)) {
            if vel[2] < 0.0 {
                let hp = w.hero.pos.map(|x| x.to_f32());
                let mut d = c::sub(p, hp);
                d[2] = 0.0;
                d[3] = 0.0;
                let l = c::len3(d);
                if l < 1.8 { vel = c::add(vel, c::scale(d, (1.8 - l) * 0.015)); }
            }
            let n = [o.normal[0], o.normal[1], o.normal[2], 0.0];
            vel = reflect(vel, n);
            let prim = o.moby.is_some() && o.kind <= 0;
            vel = c::scale(vel, if prim { f32::from_bits(0x3f86_6666) } else { 0.2 });
            if c::len3(vel) < 0.01 && c::atan(n[2], c::len2(n)) < f32::from_bits(0x3f1c_61aa) {
                let t = c::pi16(w, id, pv::TIMER).wrapping_add(1);
                c::set_pi16(w, id, pv::TIMER, t);
                if !prim || t as i32 > w.ticks(0xf0) {
                    land(w, id, o.point[2], [n[0], n[1], n[2]]);
                    vel = [0.0; 4];
                }
            }
            if pop_surface(w, id, o.surface_id()) {
                if first_person { if let Some(h) = hero { restore_collision(w, h); } }
                return;
            }
            // The bounce sound every 20 ticks (`0x220ed8` on +0x6f), a jitter of the velocity in between.
            let mut t = c::pu8(w, id, pv::SOUND_T);
            let r = if t == 0 { 1 } else { t -= 1; if t < 1 { 2 } else { 0 } };
            c::set_pu8(w, id, pv::SOUND_T, t);
            if r == 0 {
                for v in vel.iter_mut().take(3) { *v += w.rng.randf(-DT, DT); }
            } else {
                w.play_sound(0, 0, id);
                let t20 = w.ticks(0x14) as u8;
                c::set_pu8(w, id, pv::SOUND_T, t20);
            }
        }
    }
    c::set_pv4(w, id, pv::VEL, vel);
    if first_person { if let Some(h) = hero { restore_collision(w, h); } }
}

fn restore_collision(w: &mut World, h: MobyId) {
    let has = w.classes.info(w.m(h).o_class).is_some_and(|i| i.has_collision);
    w.mm(h).has_collision = has;
}

fn pv(v: V) -> [Pf; 4] { v.map(Pf::f) }

/// `FUN_00221570(out, v, n)`: `v` reflected off the plane of `n` (`v − 2·(v·n̂)·n̂`).
pub(crate) fn reflect(v: V, n: V) -> V {
    let l = c::len3(n);
    if l == 0.0 { return v; }
    let u = c::scale(n, 1.0 / l);
    let d = c::dot3(v, u);
    c::sub(v, c::scale(u, 2.0 * d))
}

/// The landing (state 2 → 3): the inflate sequence 1 over 10 ticks, collision on, velocity 0, state 3, the 600-tick
/// timer, the height of the face, the tilt to it.
fn land(w: &mut World, id: MobyId, z: f32, n: [f32; 3]) {
    let t10 = w.ticks(10);
    c::blend_to(w, id, 1, 0, t10);
    let coll = w.classes.info(w.m(id).o_class).is_some_and(|i| i.has_collision);
    let t600 = w.ticks(600) as i16;
    let m = w.mm(id);
    m.coll_disable = 0;
    m.has_collision = coll;
    p::set_v4f(&mut m.pvars, pv::VEL, [0.0; 4]);
    m.state = STANDING;
    p::set_i16(&mut m.pvars, pv::TIMER, t600);
    m.position[2] = z;
    let (tx, ty) = tilt(m.rotation[2], n);
    p::set_ff(&mut m.pvars, pv::TILT_X, tx);
    p::set_ff(&mut m.pvars, pv::TILT_Y, ty);
    ease_tilt(m);
}

/// `0x2d9f90`: a surface of kind 0, 1, 3, 8, 0xb, 0xc or 0xd under it pops it (`0x2d90a8`). True when it popped.
fn pop_surface(w: &mut World, id: MobyId, surface: i32) -> bool {
    if !matches!(surface, 0 | 1 | 3 | 8 | 0xb | 0xc | 0xd) { return false; }
    // `0x2d90a8`: ten type-5 puffs around it (`particles::type05`: 3 jitter draws ±0.2, 3 colour draws, the growth, a
    // life), then deleted.
    for _ in 0..10 {
        let mut at = w.m(id).position;
        for x in &mut at[..3] { *x += w.rng.randf(f32::from_bits(0xbe4c_cccd), f32::from_bits(0x3e4c_cccd)); }
        let (r, g, b) = (w.rng.rand(), w.rng.rand(), w.rng.rand());
        let grow = w.rng.randf(f32::from_bits(0x47c3_5000), f32::from_bits(0x4912_7c00));
        let n = w.rng.rand();
        let life = w.ticks(n % 0x28 + 10);
        fx::part05(w, grow, 0.0, at, [(r + 0x30) as u32 & 0x3f, (g + 0x20) as u32 & 0x3f, b as u32 & 0x2f], life);
    }
    w.delete_moby(id);
    true
}

/// States 3 / 4.
fn stand(w: &mut World, id: MobyId) {
    let hit = w.get_hit(id, 0x30001, false);
    {
        let m = w.mm(id);
        ease_tilt(m);
    }
    push(w, id);
    c::dec_timer_pvar_i32(w, id, pv::COOLDOWN);
    c::dec_timer_pvar_s16(w, id, pv::T6C);
    if let Some(h) = hit {
        let attacker = h.attacker.map_or(0, |a| a as i32 + 1);
        let skip = attacker == c::pi32(w, id, pv::ATTACKER) && 0 < c::pi32(w, id, pv::COOLDOWN);
        if !skip {
            c::set_pi32(w, id, pv::ATTACKER, attacker);
            let t30 = w.ticks(0x1e);
            c::set_pi32(w, id, pv::COOLDOWN, t30);
            if c::pi16(w, id, pv::T6C) == 0 {
                if h.b28 == 1 && h.b29 == 1 {
                    // (the u64 at +0x68) & 0x00ffffff_00000000: the bytes +0x6c..+0x6e.
                    if c::pi16(w, id, pv::T6C) == 0 && c::pu8(w, id, pv::B6E) == 0 {
                        let t = w.ticks(100) as i16;
                        c::set_pi16(w, id, pv::T6C, t);
                        c::set_pu8(w, id, pv::B6E, 1);
                    } else {
                        c::set_pu8(w, id, pv::B6E, 0);
                    }
                } else {
                    c::set_pu8(w, id, pv::B6E, 0);
                }
                w.play_sound(0, 0, id);
                c::set_pu8(w, id, pv::FLASH + 7, 0xfa);
                let health = c::pf(w, id, pv::HEALTH) - h.damage.to_f32();
                c::set_pf(w, id, pv::HEALTH, health);
                flash::start(w, id, pv::FLASH);
                if health < 0.0 {
                    burst(w, id);
                    return;
                }
            }
        }
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pv::FLASH);
    if w.m(id).anim.flags & 2 != 0 {
        let t10 = w.ticks(10);
        c::blend_to(w, id, 2, 0, t10);
        let s = w.rng.randf(f32::from_bits(0x3f33_3333), f32::from_bits(0x3f86_6666));
        w.mm(id).anim.speed = s;
    }
    if w.m(id).state == BURST_WALL {
        grow(w, id);
        if c::dec_timer_pvar_s16(w, id, pv::TIMER) != 0 {
            let t60 = w.ticks(0x3c);
            c::blend_to(w, id, 0, 0, t60);
            w.mm(id).state = DEFLATE;
        }
        return;
    }
    let pos = c::pos(w, id);
    let g = w.ground_height(Pf::f(0.5), pv(pos), 0).to_f32();
    if g == 0.0 { return; }
    let surface = w.coll_line(pv([pos[0], pos[1], pos[2] + 0.5, 0.0]), pv([pos[0], pos[1], 0.01, 0.0]), 2, None).map_or(-1, |o| o.surface_id());
    if pop_surface(w, id, surface) { return; }
    if g + 0.02 < pos[2] {
        let mut vel = c::pv4(w, id, pv::VEL);
        vel[2] -= DT2 * 9.8;
        let q = c::add(pos, vel);
        c::set_pos(w, id, q);
        c::set_pv4(w, id, pv::VEL, vel);
        let a = [q[0], q[1], q[2] + 0.5, 0.0];
        let b = [q[0], q[1], 0.01, 0.0];
        if let Some(o) = w.coll_line(pv(a), pv(b), 2, None) {
            if q[2] < o.point[2] {
                w.mm(id).position[2] = o.point[2];
                c::set_pv4(w, id, pv::VEL, [0.0; 4]);
                let (tx, ty) = tilt(w.m(id).rotation[2], o.normal);
                c::set_pf(w, id, pv::TILT_X, tx);
                c::set_pf(w, id, pv::TILT_Y, ty);
            }
        }
    } else {
        c::set_pv4(w, id, pv::VEL, [0.0; 4]);
    }
}

/// The burst (health below 0): a jitter of ±0.5 on (0.5, 0.5, 0.7) for the debris' drift, `SpawnBeamExplosion(0, 0,
/// 4, 3, 9, 1, 12, decoy, drift, 0, 0, 0, 0, 2, 1, 1, −1, gold)`, off the list, deleted.
fn burst(w: &mut World, id: MobyId) {
    for _ in 0..3 { w.rng.randf(-0.5, 0.5); }
    let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 3.0, flash_dist: 9.0, scale: 1.0, light: 12.0, streaks: 0, sparks: 0, puffs: 0, debris: 1, sound: 2, shake: true };
    let p = c::pos(w, id);
    // The gold decoy (class 1900 and 0x13e539, item 25; 0 for the normal class 0xcb): the burst's colour shift, then an
    // area hit of 3·gold (damage 2·gold, flags 0x810000, type 2 / 1 [L: the second byte is a pointer's low byte]).
    let gold = if w.m(id).o_class == GOLD_CLASS { w.hero.weapons.gold.get(DECOY_GLOVE as usize).copied().unwrap_or(0) } else { 0 };
    fx::beam_explosion_shift(w, &b, Some(id), p, gold);
    if gold != 0 {
        let g = gold as f32;
        let tmpl = HitTemplate { attacker: Some(id), flags: 0x81_0000, b18: 2, b19: 1, h1a: w.m(id).o_class as u16, damage: Pf::f(g + g), ..Default::default() };
        w.sphere_mobys(Pf::f(g * 3.0), p.map(Pf::f), 0x10, Some(id), Some(&tmpl));
    }
    w.delete_moby(id);
}

/// `0x2d9d08`: Ratchet stuck on top of it (the stuck-in-air counter 0x13f532, within 1 across and 1.3 in height)
/// starts the push (+0x68): a spring (`0x270780`: 0.02, 0.3, at most 4·dt) takes the speed toward pushing it 1.4
/// out from under his feet; the move through `walker::move_collide` (up 0.5, radius 0.4); gravity 24·dt² down to the
/// ground. The push ends once settled on the ground.
fn push(w: &mut World, id: MobyId) {
    let stuck = w.hero.f532 != 0;
    let feet = w.hero.pos.map(|x| x.to_f32());
    let pos = c::pos(w, id);
    if c::pi32(w, id, pv::PUSHED) == 0 {
        if !stuck { return; }
        if 1.0 <= c::dist2(feet, pos) { return; }
        if 1.3 <= (feet[2] - pos[2]).abs() { return; }
        c::set_pi32(w, id, pv::PUSHED, 1);
    }
    let mut d = c::sub(pos, feet);
    d[2] = 0.0;
    d[3] = 0.0;
    let mut x = (c::len2(d) - 1.4).min(0.0);
    if !stuck { x = 0.0; }
    let mut v = c::pf(w, id, pv::PUSH_V);
    super::flow::spring(0.02, 0.3, DT * 4.0, &mut x, &mut v);
    c::set_pf(w, id, pv::PUSH_V, v);
    if x == 0.0 && v == 0.0 {
        let g = w.ground_height(Pf::f(0.5), pv(pos), 0).to_f32();
        if (pos[2] - g).abs() < 0.02 { c::set_pi32(w, id, pv::PUSHED, 0); }
    }
    let l = c::len3(d);
    if l < c::pf(w, id, pv::PUSH_V) { c::set_pf(w, id, pv::PUSH_V, l); }
    let speed = c::pf(w, id, pv::PUSH_V);
    if speed < l { d = c::set_len3(d, speed); }
    walker::move_collide(w, id, 0.5, 0.4, 0.0, &mut d, 0);
    let pos = c::pos(w, id);
    let g = w.ground_height(Pf::f(0.5), pv(pos), 0).to_f32();
    if pos[2] < g {
        w.mm(id).position[2] = g;
        c::set_pf(w, id, pv::PUSH_FALL, 0.0);
    } else {
        let f = c::pf(w, id, pv::PUSH_FALL) - DT2 * 24.0;
        c::set_pf(w, id, pv::PUSH_FALL, f);
        let z = (pos[2] + f).max(g);
        w.mm(id).position[2] = z;
    }
}

/// `0x2d9760(pvars, decoy, 1)`: the landing preview ([`tg::arc_landing`] with gravity 9, primitives snapped without
/// the height limit): held, from the launch point 0.39082 up with the aimed velocity (z − 0.5·dt, plus Ratchet's
/// displacement while grinding) for 300 ticks; flying, from the decoy for the +0x44 ticks (0 after a release: no
/// reticle); the point pulled toward the camera and the glove reticle registered (`0x2d8e28`, the bomb's `0x2c23c0`
/// copy: [`tg::BOMB_RETICLE`]).
fn preview(w: &mut World, id: MobyId) -> bool {
    if w.hero.state == 0x72 || w.svc.game_mode != 0 { return false; }
    let v4 = |a: [f32; 4]| [a[0], a[1], a[2]];
    let held = w.m(id).state == HELD;
    let vel = v4(c::pv4(w, id, pv::VEL));
    let budget = if held {
        w.ticks(300)
    } else {
        if w.hero.weapons.glove.held.is_some() { c::set_pi16(w, id, pv::NEW_HELD, 1); }
        c::pi16(w, id, pv::TIMER) as i32
    };
    let Some((point, normal)) = short_glove_preview(w, id, held, vel, budget) else { return false };
    c::set_pv4(w, id, pv::POINT, [point[0], point[1], point[2], 0.0]);
    c::set_pv4(w, id, pv::NORMAL, [normal[0], normal[1], normal[2], 0.0]);
    true
}

/// The landing preview of the Decoy Glove's decoy (`0x2d9760`) and the Glove of Doom's canister (`0x2de0a8`, the
/// same code over its own pvars): [`tg::arc_landing`] with gravity 9, primitives snapped without the height limit,
/// from the launch point 0.39082 up with the aimed velocity (z − 0.5·dt, plus Ratchet's displacement while grinding)
/// when `held`, else from the object; for `budget` ticks. The point is pulled toward the camera (0.95 of the way from
/// it) and the glove reticle registered (`0x2d8e28` / `0x2dda60`, the bomb's `0x2c23c0` copies:
/// [`tg::BOMB_RETICLE`]). Returns the point and the normal the caller keeps in its pvars; None: no landing (or
/// Ratchet in 0x72, a game mode).
pub(crate) fn short_glove_preview(w: &mut World, id: MobyId, held: bool, vel: [f32; 3], budget: i32) -> Option<([f32; 3], [f32; 3])> {
    use crate::targeting::{ArcStart, LineHit, Prims};
    if w.hero.state == 0x72 || w.svc.game_mode != 0 { return None; }
    let v4 = |a: [f32; 4]| [a[0], a[1], a[2]];
    let mut vel = vel;
    let from = if held {
        let from = crate::hero::gloves::launch_at(w.hero, 0.390_82);
        if w.hero.group == 0xf {
            let d = crate::hero::physics::to_f32x3(w.hero.disp);
            vel = [vel[0] + d[0], vel[1] + d[1], vel[2] + d[2]];
        }
        from
    } else {
        v4(c::pos(w, id))
    };
    vel[2] -= DT * 0.5;
    let hero_pos = w.hero_moby.and_then(|h| w.table.mobys.get(h)).map_or(from, |m| [m.position[0], m.position[1], m.position[2]]);
    let start = ArcStart {
        from,
        vel,
        budget,
        gravity: DT2 * 9.0,
        first: [hero_pos[0], hero_pos[1], from[2]],
        first_ignore: w.hero_moby,
        ignore: Some(id),
        own: [w.hero_moby, None],
        gold: false,
        prims: Prims::Snap { max_rise: None },
        first_test: true,
    };
    let landing = {
        let ww: &World = w;
        tg::arc_landing(
            &start,
            |a, b, ignore| {
                let h = ww.coll_line(pv([a[0], a[1], a[2], 0.0]), pv([b[0], b[1], b[2], 0.0]), 0x10, ignore)?;
                Some(LineHit { point: h.point, normal: h.normal, moby: h.moby, kind: h.kind, surface: h.surface_id() })
            },
            |m| { let q = ww.m(m).position; [q[0], q[1], q[2]] },
            |a, b| ww.coll_line(pv([a[0], a[1], a[2], 0.0]), pv([b[0], b[1], b[2], 0.0]), 2, None).map(|h| h.normal),
        )
    };
    let l = landing?;
    let cam = [w.camera[0].to_f32(), w.camera[1].to_f32(), w.camera[2].to_f32()];
    let point = tg::pull_toward(cam, l.point);
    w.svc.reticles.register(w.counter, tg::Reticle { point, normal: l.normal, style: &tg::BOMB_RETICLE });
    Some((point, l.normal))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hero::testkit::{cell, mesh};
    use crate::moby_update::classes::bomb_water::tests::{pool, Bench};

    /// A floor of surface `kind & 0x1f` at `z` over x, y in 0..48.
    pub(crate) fn floor(z: f32, kind: u8) -> rc_formats::collision::Collision {
        let mut cells = Vec::new();
        for cx in 0..12i16 {
            for cy in 0..12i16 {
                let (x, y) = (cx as f32 * 4.0, cy as f32 * 4.0);
                let q = [[x, y, z], [x, y + 4.0, z], [x + 4.0, y + 4.0, z], [x + 4.0, y, z]];
                cells.push(cell([cx, cy, ((z - 2.0) / 4.0).round() as i16], &q, &[([0, 1, 2, 3], kind)]));
            }
        }
        mesh(cells)
    }

    fn thrown(b: &mut Bench, at: [f32; 3], vel: [f32; 3]) -> MobyId {
        let d = b.create(CLASS);
        let mut rng = crate::rng::Rng::new();
        init_held(&mut b.table, d, &mut rng, at, false, false);
        set_velocity(&mut b.table.mobys[d], vel);
        release(&mut b.table.mobys[d], at, None);
        d
    }

    /// `0x2d9f90` / `0x2d90a8`: a decoy bouncing on a pop surface (the pool's floor, surface 1) pops at once: 10 type-5
    /// puffs (live records, `particles::type05`) and deleted; on an ordinary floor (surface 2) it lands and stands.
    #[test]
    fn pops_on_a_pop_surface() {
        let mut b = Bench::new(pool(8.0, 4.0), &[CLASS], [40.0, 40.0, 9.0]);
        let d = thrown(&mut b, [20.0, 20.0, 9.0], [0.02, 0.0, 0.0]);
        let p5 = b.parts(5);
        for _ in 0..200 {
            b.tick(None);
            if b.table.mobys[d].is_deleted() { break; }
        }
        assert!(b.table.mobys[d].is_deleted(), "popped");
        assert_eq!(b.parts(5) - p5, 10, "ten type-5 puffs");
        // The puffs live their game lives (`particles::type05`): 10 records, faded and gone within 50 updates, none
        // killed unported.
        assert_eq!(b.parts.live_by_type()[5], 10);
        let mut rng = crate::rng::Rng::new();
        for _ in 0..50 { b.parts.update_parts(&mut rng); }
        assert_eq!((b.parts.live_by_type()[5], b.parts.stats.unported_kills[5]), (0, 0));
        let mut b = Bench::new(floor(8.0, 0x22), &[CLASS], [40.0, 40.0, 9.0]);
        let d = thrown(&mut b, [20.0, 20.0, 9.0], [0.02, 0.0, 0.0]);
        for _ in 0..200 { b.tick(None); }
        assert_eq!(b.table.mobys[d].state, STANDING, "stands on surface 2");
    }

    /// `0x2d9d08`: Ratchet stuck on top of a standing decoy (the stuck-in-air counter 0x13f532) pushes it out from under
    /// him (+0x68 set, sliding to about 1.4 away through `walker::move_collide`), and the push ends once he is off.
    #[test]
    fn slides_from_under_ratchet() {
        let mut b = Bench::new(floor(8.0, 0x22), &[CLASS], [20.3, 20.0, 8.6]);
        let d = thrown(&mut b, [20.0, 20.0, 8.0], [0.0; 3]);
        b.table.mobys[d].state = STANDING;
        b.hero.f532 = 5;
        let feet = [20.3f32, 20.0];
        let dist = |b: &Bench| { let q = b.table.mobys[d].position; ((q[0] - feet[0]).powi(2) + (q[1] - feet[1]).powi(2)).sqrt() };
        let d0 = dist(&b);
        b.tick(None);
        assert_eq!(p::i32(&b.table.mobys[d].pvars, pv::PUSHED), 1, "pushed");
        for _ in 0..90 { b.tick(None); }
        let d1 = dist(&b);
        assert!(d0 < 0.5 && (1.2..1.5).contains(&d1), "slid from {d0} to {d1}");
        b.hero.f532 = 0;
        for _ in 0..90 { b.tick(None); }
        assert_eq!(p::i32(&b.table.mobys[d].pvars, pv::PUSHED), 0, "the push ends");
    }
}
