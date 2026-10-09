//! **The Mine Glove's mine** (class 74 = 0x4a; one update on every level: level01 `0x2bfe40`, 1115 decompiled lines,
//! checked against the disassembly where the decompiler lost stack copies and float arguments), its create `0x2bf6a0`,
//! release `0x2bf7d8` and landing preview `0x2bfa78` (a copy of the bomb's, drawn with the same reticle). The glove's
//! side is the shared glove update (`crate::hero::gloves`, item 17). docs/plan/hero_gameplay.md §13.
//!
//! **States** (+0x20, bits): 0 **held** (the glove places it; hidden in first person; no collision); 1 **out** (thrown);
//! 2 **armed** (120 ticks after the release: +0x34); 4 **landed**; 0x10 **seeking** a target.
//! * **Out, not landed**: it moves by its velocity (+0x00), gravity 9.8·dt² (a tenth in water), its path tested with
//!   a hit template (damage 3, flags 0x830000: a moby on the way takes it) from its position at the update's start —
//!   the shown one, 0.4 above the physical. Onto water from above: the water entry (`bomb_water::entry`: the ripple,
//!   the splash, 16 drops) and the sinking (1.5 u/s, bubbles); the next tick's path, starting 0.4 up, meets the surface
//!   again while in water, so it goes off at once in its water burst (the game's code: its sinking branch never lasts
//!   more than a tick; reproduced). A crate or a
//!   targetable moby (mode 0x5000) other than a mine or Ratchet: it explodes. Another mine: bounces off it (×0.4).
//!   Ground (not water, not falling faster than 9.8 u/s): bounces (reflected ×0.6, half the vertical), the first
//!   bounce starts the bob (twice the fall speed, fading in over 6 ticks, decaying 1.2 % a tick to 0.05); on ground
//!   flatter than 45° it flips upward, and **lands** once slower than 0.5 u/s (collision on). Anything else: explodes.
//!   It eases its tilt to the face (3° a tick).
//! * **Landed**: snapped to the ground (it is out again, unarmed, if the ground drops 40·dt² away); a slow wobble
//!   (`0x277a80`: 0.525 rad, phases at 30°/s and 17°/s); Ratchet within 0.75 pushes it out from under him
//!   (`walker::move_ground`, up 0.4, radius 0.25, step 0.25, slope 0.525).
//! * **Armed and landed, the proximity search** (the target list 0x1abe80; Ratchet farther than 0.75): any target
//!   within 1.5 sets it off; else the nearest (xy) non-crate target within `4 + record radius / 8` (×3 while the
//!   Taunter lures it, +0x78), less than 2 above or below, becomes its target and it **seeks** (0x10): sequence 1 then
//!   2, class sound 0 once and its loop sound 1, speed up by 15·dt² to 4 u/s straight at the target, hopping (0.05 a
//!   tick for 7 up, 7 down), a 0.35 sphere 0.35 up (damage 1, flags 0x10000) — any moby it touches but mines and a few
//!   classes (0x363, 0x365, 0x367, 0x458) sets it off — pushed out of walls (0.25), for at most 600 ticks. A target
//!   gone or no longer targetable: it stops (out again, the loop released). Armed while still out: it explodes. A hit
//!   (mask 0x830000) from anything but a mine explodes it; more than 8 mines: the oldest (+0x5c) goes (a death
//!   explosion `0x273f50`).
//! * **The explosion** (+0xbc): dry — an area hit of radius 2 (damage 3, push 0.25 / 1.5, flags 0x810000, type 4 / 1;
//!   `creature::attack::area_hit`), then in the air (2 above the ground) `SpawnBeamExplosion` (flashes 2 / 1 beyond 4,
//!   light 7, 3 streaks, 3 spark pairs, 5 puffs, sound 2, debris 1), on the ground 15 type-16 smoke / fire / spark
//!   puffs of four kinds; in water — 150 bubbles (`bomb_water::bubbles`) and the scorch from the surface
//!   (`bomb_water::scorch`, [`bomb_water::MINE_SCORCH`]); class sound 2, the camera shake (0.4 − 0.0175·d within 20),
//!   the Bomb Glove's explosion light (template 0x20a890 = 0x20a930, [`fx::LIGHT_BOMB`]); deleted.
//!
//! **Pvars** (0x80): +0x00 velocity, +0x10 / +0x20 the preview's point and normal (the reticle), +0x30 the owner (the
//! glove: 1 here), +0x34 s16 the arming / preview timer, +0x36 s16 a new mine in the glove, +0x38 / +0x3c spin rates
//! (drawn at the creation), +0x40 the seek speed, +0x48 the hop timer, +0x4c the platform (never set in the port),
//! +0x50 s16 in water, +0x52 s16 the hop phase, +0x54 the target (+1), +0x58 the seek's life, +0x5c the creation
//! frame, +0x60 / +0x64 the wobble phases, +0x68 the loop sound slot, +0x6c / +0x70 / +0x74 the bob's amplitude,
//! phase and start frame, +0x78 lured by the Taunter.
//!
//! Native `f32`; the rand draws are the game's. **Inferred [L]**: the path's start and the landed velocity use the
//! position at the start of the update (the decompiler lost the quadword copy; the disassembly's `lq`/`sq` pair); the
//! hit template's direction is the unit xy velocity with z 1 and its type bytes 2 / 1 (the seek template's type bytes,
//! which the game leaves as the stack held them, are 0); the held mine's orientation (the glove's joint-0 frame turned
//! by π about x) is not written (the glove is not a table moby); the release's wall test starts at Ratchet (at the
//! launch height; the disassembly's `lq`); the mines' list 0x1b0c30 is the table's mines; a mine on a moving platform
//! does not ride it (0x13f64c: never set in the port); the blob shadow `0x26eec8(0.25)` is `crate::shadows::blob`; the gold
//! mine (0x13e531, item 17: the seek range ×2, the blast and its puffs ×1.5, their colours shifted, class sound 3, the
//! light 0x20a8e0) reads the mirrored gold table (`Weapons::gold`); the global 0x1613d0 that blows every mine up (0 in level01's data; its writer is
//! not in the ported code) is not read.

use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::creature::{self as c, attack, fx, walker, V};
use crate::moby_update::services::{self as sv, pvar as p, HitTemplate, World};
use crate::ps2v::Pf;
use crate::targeting as tg;

use super::bomb_water;

pub const UPDATE_FN: u32 = 0x2bfe40;
pub const CLASS: i16 = 0x4a;
pub const CLASSES: [i16; 1] = [CLASS];
/// The Mine Glove (item 17) and its hand class.
pub const MINE_GLOVE: i32 = 17;
pub const GLOVE_CLASS: i16 = 0xbe;

/// State bits (+0x20).
pub const OUT: u8 = 1;
pub const ARMED: u8 = 2;
pub const LANDED: u8 = 4;
pub const SEEKING: u8 = 0x10;

/// Pvar offsets.
pub mod pv {
    pub const VEL: usize = 0x00;
    pub const POINT: usize = 0x10;
    pub const NORMAL: usize = 0x20;
    pub const OWNER: usize = 0x30;
    pub const TIMER: usize = 0x34;
    pub const NEW_HELD: usize = 0x36;
    pub const SPIN_A: usize = 0x38;
    pub const SPIN_B: usize = 0x3c;
    pub const SPEED: usize = 0x40;
    pub const HOP_T: usize = 0x48;
    pub const WATER: usize = 0x50;
    pub const HOP: usize = 0x52;
    pub const TARGET: usize = 0x54;
    pub const LIFE: usize = 0x58;
    pub const BORN: usize = 0x5c;
    pub const WOB_A: usize = 0x60;
    pub const WOB_B: usize = 0x64;
    pub const SOUND: usize = 0x68;
    pub const BOB_AMP: usize = 0x6c;
    pub const BOB_PHASE: usize = 0x70;
    pub const BOB_START: usize = 0x74;
    pub const LURED: usize = 0x78;
    pub const SIZE: usize = 0x80;
}

const DT: f32 = c::DT;
const DT2: f32 = c::DT2;

fn pvq(v: V) -> [Pf; 4] { v.map(Pf::f) }

/// `0x2bf6a0(glove, point)`: the new mine (the caller's `CreateMoby(0x4a)`): update distance 0xff, draw distance 0x7f,
/// visible, half the class scale, state 0, the owner, at the hand point, velocity 0, the spin rates (2 draws
/// `randf(60°·dt, 180°·dt)`), no sound slot, the bob cleared, born now (`frame`), no collision.
pub fn init_held(m: &mut crate::moby_runtime::Moby, rng: &mut crate::rng::Rng, point: [f32; 3], frame: i32) {
    m.update_dist = 0xff;
    m.draw_dist = 0x7f;
    m.visible = 1;
    m.scale *= 0.5;
    m.state = 0;
    if m.pvars.len() < pv::SIZE { m.pvars.resize(pv::SIZE, 0); }
    let pvs = &mut m.pvars;
    p::set_i32(pvs, pv::OWNER, 1);
    p::set_i16(pvs, pv::NEW_HELD, 0);
    p::set_i32(pvs, pv::LURED, 0);
    m.position = [point[0], point[1], point[2], m.position[3]];
    p::set_v4f(pvs, pv::VEL, [0.0; 4]);
    let a = rng.randf(DT * std::f32::consts::FRAC_PI_3, DT * std::f32::consts::PI);
    p::set_ff(pvs, pv::SPIN_A, a);
    let b = rng.randf(DT * std::f32::consts::FRAC_PI_3, DT * std::f32::consts::PI);
    p::set_i32(pvs, pv::SOUND, -1);
    p::set_ff(pvs, pv::SPIN_B, b);
    p::set_ff(pvs, pv::BOB_AMP, 0.0);
    p::set_i32(pvs, pv::BORN, frame);
    p::set_ff(pvs, pv::WOB_B, 0.0);
    p::set_ff(pvs, pv::WOB_A, 0.0);
    p::set_ff(pvs, pv::BOB_PHASE, 0.0);
    p::set_i32(pvs, pv::BOB_START, 0);
    m.has_collision = false;
}

/// `0x2bf7d8(point, mine, vel)`: the release: out (state 1), the scale back up (÷0.65), a random nudge of the velocity
/// (`randf(±1)` ×3 normalised to `randf(0.01·dt, 0.1·dt)`), the 120-tick arming timer; with more than 7 other mines the
/// oldest is told to go (+0x5c = −1); at the launch point. The caller then runs the wall test (Ratchet at the point's
/// height → the point, the world and the mobys, Ratchet ignored: the disassembly's `lq` of his position) and hands a
/// hit to [`release_hit`].
pub fn release(table: &mut MobyTable, id: MobyId, rng: &mut crate::rng::Rng, point: [f32; 3]) {
    let x = rng.randf(-1.0, 1.0);
    let y = rng.randf(-1.0, 1.0);
    let z = rng.randf(-1.0, 1.0);
    {
        let m = &mut table.mobys[id];
        m.state = OUT;
        m.scale /= 0.65;
    }
    let s = rng.randf(DT * 0.01, DT * 0.1);
    let nudge = c::set_len3([x, y, z, 0.0], s);
    {
        let m = &mut table.mobys[id];
        let v = p::v4f(&m.pvars, pv::VEL);
        p::set_v4f(&mut m.pvars, pv::VEL, c::add(nudge, v));
        p::set_i16(&mut m.pvars, pv::TIMER, 120);
    }
    // More than 7 other mines: the oldest goes.
    let mut oldest: Option<(MobyId, i32)> = None;
    let mut n = 0;
    for (i, m) in table.mobys.iter().enumerate() {
        if i == id || m.o_class != CLASS || m.is_deleted() { continue; }
        let born = p::i32(&m.pvars, pv::BORN);
        if oldest.is_none_or(|(_, b)| born < b) { oldest = Some((i, born)); }
        n += 1;
    }
    if 7 < n {
        if let Some((o, _)) = oldest { p::set_i32(&mut table.mobys[o].pvars, pv::BORN, -1); }
    }
    let m = &mut table.mobys[id];
    m.position = [point[0], point[1], point[2], m.position[3]];
}

/// The release's wall hit: on the wall (9.7·dt² off it), the velocity reflected ×0.6.
pub fn release_hit(m: &mut crate::moby_runtime::Moby, point: [f32; 3], normal: [f32; 3]) {
    let n4 = [normal[0], normal[1], normal[2], 0.0];
    let q = c::add([point[0], point[1], point[2], m.position[3]], c::set_len3(n4, DT2 * 9.7));
    m.position = q;
    let v = p::v4f(&m.pvars, pv::VEL);
    p::set_v4f(&mut m.pvars, pv::VEL, c::scale(reflect(v, n4), 0.6));
}

/// `FUN_00221570(out, v, n)`: `v` reflected off the plane of `n`.
fn reflect(v: V, n: V) -> V {
    let l = c::len3(n);
    if l == 0.0 { return v; }
    let u = c::scale(n, 1.0 / l);
    let d = c::dot3(v, u);
    c::sub(v, c::scale(u, 2.0 * d))
}

/// `FUN_00222420(a, target, step)`: `a` turned toward `target` by at most `step`.
pub(crate) fn approach_angle(a: f32, target: f32, step: f32) -> f32 {
    let d = c::sub_rot(target, a);
    let s = if d <= 0.0 { -step } else { step };
    let s = s.clamp(-d.abs(), d.abs());
    c::add_rot(a, s)
}

/// `FUN_00273278(m)`: a crate (classes 500..540 of the moby array).
fn is_crate(w: &World, m: MobyId) -> bool { (500..=540).contains(&(w.m(m).o_class as i32)) }

/// Whether the hand holds the Mine Glove.
fn glove_in_hand(w: &World) -> bool { w.hero.items.slot.item.as_ref().is_some_and(|m| m.o_class == GLOVE_CLASS) }

/// The bob's fade-in: `(frame − start) / 6`, clamped to [0, 1].
fn bob_fade(w: &World, id: MobyId) -> f32 {
    let t = (w.counter as i32).wrapping_sub(c::pi32(w, id, pv::BOB_START)) as f32 / 6.0;
    t.clamp(0.0, 1.0)
}

/// `0x2bfe40`.
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { w.mm(id).pvars.resize(pv::SIZE, 0); }
    let st = w.m(id).state;
    {
        let hide = w.hero.f13f5 != 0 || w.hero.f13ff != 0;
        let m = w.mm(id);
        if !hide || st != 0 { m.mode &= !0x41; } else { m.mode |= 0x41; }
    }
    let glove = glove_in_hand(w);
    if (!glove || w.body() != 0) && st == 0 {
        w.delete_moby(id);
        return;
    }
    if glove && c::pi16(w, id, pv::NEW_HELD) == 0 && 0.0 < c::len3(c::pv4(w, id, pv::VEL)) { preview(w, id); }
    if st == 0 {
        // Held: the glove places it; no collision.
        let m = w.mm(id);
        m.has_collision = false;
        m.mode |= 0x100;
        return;
    }
    w.mm(id).mode &= !0x100;
    let old = c::pos(w, id);
    let mut drift: V = [0.0, 0.0, DT + DT, 0.0];
    let mut normal: V = [0.0, 0.0, 1.0, 0.0];
    let mut vel = c::pv4(w, id, pv::VEL);
    if st & LANDED == 0 {
        c::set_pos(w, id, c::add(old, vel));
        if c::pi16(w, id, pv::WATER) == 0 {
            vel[2] -= DT2 * 9.8;
        } else {
            vel[2] -= (DT2 * 9.8) / 10.0;
            bomb_water::sink_bubble(w, id, vel);
        }
        c::set_pv4(w, id, pv::VEL, vel);
    }
    let tmpl = {
        let d = c::set_len2([vel[0], vel[1], 0.0, 0.0], 1.0);
        HitTemplate { dir: [Pf::f(d[0]), Pf::f(d[1]), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x83_0000, b18: 2, b19: 1, h1a: CLASS as u16, damage: Pf::f(3.0), w20: 1 }
    };
    // Undo the display bob of the last tick.
    {
        let fade = bob_fade(w, id);
        let (amp, ph) = (c::pf(w, id, pv::BOB_AMP), c::pf(w, id, pv::BOB_PHASE));
        let m = w.mm(id);
        m.position[2] -= 0.4;
        m.position[2] -= amp * ph.sin() * fade;
    }
    if st & LANDED == 0 {
        let pos = c::pos(w, id);
        if let Some(h) = sv::line_hit_in(w.table, w.svc, w.classes, w.coll, pvq(old), pvq(pos), 0x10, Some(id), &tmpl) {
            flight_hit(w, id, &h, vel, &mut drift, &mut normal);
        }
    } else {
        landed(w, id, old);
    }
    after_move(w, id, drift, normal);
}

/// A hit on the flight path (the module doc's "out, not landed").
fn flight_hit(w: &mut World, id: MobyId, h: &crate::collision_query::CollOutput, vel: V, drift: &mut V, normal: &mut V) {
    let water = c::pi16(w, id, pv::WATER) != 0;
    let hn: V = [h.normal[0], h.normal[1], h.normal[2], 0.0];
    let hp: V = [h.point[0], h.point[1], h.point[2], w.m(id).position[3]];
    let surface = h.surface_id();
    if !water && surface == 0 {
        if vel[2] < 0.0 {
            c::set_pi16(w, id, pv::WATER, 1);
            c::set_pv4(w, id, pv::VEL, [0.0, 0.0, DT * -1.5, 0.0]);
            bomb_water::entry(w, id, h.point[2]);
        }
        return;
    }
    let mut explode_at = |w: &mut World, id: MobyId, drift: &mut V, normal: &mut V| {
        w.mm(id).cmd = 1;
        c::set_pos(w, id, hp);
        *drift = c::set_len3(reflect(vel, hn), DT + DT);
        *normal = c::set_len3(hn, 1.0);
    };
    let landing = |w: &mut World, id: MobyId, drift: &mut V, normal: &mut V, explode_at: &mut dyn FnMut(&mut World, MobyId, &mut V, &mut V)| {
        if !water && surface != 0 && DT * -9.8 <= vel[2] {
            ground_bounce(w, id, hp, hn, vel);
        } else {
            explode_at(w, id, drift, normal);
        }
    };
    match h.moby {
        None => {
            if h.kind < 1 { return; }
            landing(w, id, drift, normal, &mut explode_at);
        }
        Some(m) => {
            let hero = w.hero_moby == Some(m);
            let other_class = w.m(m).o_class != CLASS;
            if (is_crate(w, m) || w.m(m).mode & 0x5000 != 0) && other_class && !hero {
                w.mm(id).cmd = 1;
                if w.m(id).state != OUT {
                    c::set_pos(w, id, hp);
                    let g = w.ground_height(Pf::f(0.5), pvq(hp), 0).to_f32();
                    w.mm(id).position[2] = g;
                }
                return;
            }
            if other_class {
                if hero && h.kind < 1 { return; }
                landing(w, id, drift, normal, &mut explode_at);
                return;
            }
            // Another mine.
            let n = c::sub(c::pos(w, id), w.m(m).position);
            if DT * -9.8 <= vel[2] {
                let q = c::add(hp, c::set_len3(n, DT2 * 9.7));
                c::set_pos(w, id, q);
                c::set_pv4(w, id, pv::VEL, c::scale(reflect(vel, n), f32::from_bits(0x3ecc_cccd)));
                orient(w, id, n);
            } else {
                w.mm(id).cmd = 1;
                c::set_pos(w, id, hp);
                *drift = c::set_len3(reflect(vel, n), DT + DT);
                *normal = c::set_len3(n, 1.0);
            }
        }
    }
}

/// A bounce off the ground or a wall (module doc): the point 9.7·dt² off the face, the velocity reflected ×0.6 and
/// half its vertical; the first bounce starts the bob; onto a floor (more than 45° flat) while falling: flipped up,
/// landed when slow; the tilt eased toward the face.
fn ground_bounce(w: &mut World, id: MobyId, hp: V, hn: V, vel: V) {
    let q = c::add(hp, c::set_len3(hn, DT2 * 9.7));
    c::set_pos(w, id, q);
    let vz0 = vel[2];
    let mut v = c::scale(reflect(vel, hn), 0.6);
    v[2] *= 0.5;
    if c::pf(w, id, pv::BOB_AMP) == 0.0 {
        c::set_pf(w, id, pv::BOB_AMP, vz0.abs() + vz0.abs());
        c::set_pf(w, id, pv::BOB_PHASE, if 0.0 < vz0 { f32::from_bits(0x3f49_0fdb) } else { f32::from_bits(0xc016_cbe4) });
        let now = w.counter as i32;
        c::set_pi32(w, id, pv::BOB_START, now);
    }
    let up = c::set_len3(hn, 1.0);
    if 0.707 < up[2] && vz0 < 0.0 {
        let g = w.ground_height(Pf::f(0.5), pvq(q), 0).to_f32();
        if w.m(id).position[2] < g { w.mm(id).position[2] = g; }
        v[2] = -v[2];
        if c::len3(v) < DT * 0.5 {
            v = [0.0; 4];
            let coll = w.classes.info(CLASS).is_some_and(|i| i.has_collision);
            let m = w.mm(id);
            m.state |= LANDED;
            m.has_collision = coll;
        }
    }
    c::set_pv4(w, id, pv::VEL, v);
    orient(w, id, hn);
}

/// The tilt toward a face's normal (`LAB_002c0968`): 3° a tick.
fn orient(w: &mut World, id: MobyId, n: V) {
    let yaw = w.m(id).rotation[2];
    let (co, si) = (yaw.cos(), yaw.sin());
    let a = n[0] * co + n[1] * si;
    let b = n[1] * co - n[0] * si;
    let cz = n[2];
    let tx = c::atan((a * a + cz * cz).sqrt(), b);
    let ty = c::atan(cz, a);
    let step = f32::from_bits(0x3d56_7750);
    let m = w.mm(id);
    m.rotation[0] = approach_angle(m.rotation[0], -tx, step);
    m.rotation[1] = approach_angle(m.rotation[1], ty, step);
}

/// Stop seeking: out again (unarmed), the loop released, the arming timer again, sequence 0.
fn stop_seek(w: &mut World, id: MobyId) {
    w.mm(id).state = OUT;
    release_loop(w, id);
    let mut v = c::pv4(w, id, pv::VEL);
    v[2] = 0.0;
    c::set_pv4(w, id, pv::VEL, v);
    c::set_pi32(w, id, pv::TARGET, 0);
    c::set_pi16(w, id, pv::TIMER, 120);
    c::blend_to(w, id, 0, 0, 10);
}

fn release_loop(w: &mut World, id: MobyId) {
    let s = c::pi32(w, id, pv::SOUND);
    if s != -1 { w.release_sound(s, id); }
    c::set_pi32(w, id, pv::SOUND, -1);
}

/// The target (+0x54 holds the moby + 1).
fn target(w: &World, id: MobyId) -> Option<MobyId> {
    let t = c::pi32(w, id, pv::TARGET);
    (t > 0).then(|| (t - 1) as MobyId).filter(|&m| m < w.table.mobys.len())
}

/// Landed (`LAB_002c0a38`): the ground, the seek, the wobble; the velocity becomes the move made.
fn landed(w: &mut World, id: MobyId, old: V) {
    let pos = c::pos(w, id);
    let g = w.ground_height(Pf::f(0.5), pvq(pos), 0).to_f32();
    let mut drop = false;
    if w.m(id).state & SEEKING == 0 {
        if DT2 * 40.0 <= pos[2] - g {
            w.mm(id).state = OUT;
            release_loop(w, id);
            let mut v = c::pv4(w, id, pv::VEL);
            v[2] = 0.0;
            c::set_pv4(w, id, pv::VEL, v);
            c::set_pi16(w, id, pv::TIMER, 120);
            drop = true;
        } else {
            w.mm(id).position[2] = g;
            c::set_pi16(w, id, pv::TIMER, 0);
        }
    }
    let _ = drop;
    let live = target(w, id).filter(|&t| { let m = w.m(t); !m.is_deleted() && m.mode & mode::TARGETABLE != 0 });
    if live.is_none() && w.m(id).state & SEEKING != 0 { stop_seek(w, id); }
    if let (Some(t), true) = (live, w.m(id).state & SEEKING != 0) {
        if w.m(id).anim.flags & 2 != 0 && w.m(id).anim.seq_a == 1 { c::blend_to(w, id, 2, 0, 10); }
        let slot = c::pi32(w, id, pv::SOUND);
        if !w.sound_alive(slot, id) {
            let s = w.play_sound(1, 4, id);
            c::set_pi32(w, id, pv::SOUND, s);
        }
        let speed = (c::pf(w, id, pv::SPEED) + DT2 * 15.0).min(DT * 4.0);
        c::set_pf(w, id, pv::SPEED, speed);
        match c::pi16(w, id, pv::HOP) {
            1 => {
                w.mm(id).position[2] += 0.05;
                if c::dec_timer_pvar_i32(w, id, pv::HOP_T) != 0 {
                    let t7 = w.ticks(7);
                    c::set_pi32(w, id, pv::HOP_T, t7);
                    c::set_pi16(w, id, pv::HOP, 2);
                }
            }
            2 => {
                w.mm(id).position[2] -= 0.05;
                if c::dec_timer_pvar_i32(w, id, pv::HOP_T) != 0 { c::set_pi16(w, id, pv::HOP, 0); }
            }
            _ => {}
        }
        let pos = c::pos(w, id);
        let d = c::set_len3(c::sub(w.m(t).position, pos), speed);
        let pos = c::add(pos, d);
        c::set_pos(w, id, pos);
        let seek = HitTemplate { dir: pvq(d), attacker: Some(id), flags: 0x1_0000, b18: 0, b19: 0, h1a: 0, damage: Pf::ONE, w20: 1 };
        let centre = [pos[0], pos[1], pos[2] + 0.35, pos[3]];
        let list = sv::sphere_mobys_in(w.table, w.svc, w.classes, Pf::f(f32::from_bits(0x3eb3_3333)), pvq(centre), 0x10, Some(id), Some(&seek));
        if let Some(&first) = list.first() {
            if !matches!(w.m(first).o_class, 0x363 | 0x4a | 0x458 | 0x365 | 0x367) { w.mm(id).cmd = 1; }
        }
        let centre = [pos[0], pos[1], pos[2] + 0.25, pos[3]];
        if let Some(o) = w.coll_sphere(pvq(centre), Pf::f(0.25), 0, Some(id)) {
            if let Some(pc) = o.pushed_centre {
                let q = c::add(pos, [pc[0] - centre[0], pc[1] - centre[1], pc[2] - centre[2], 0.0]);
                c::set_pos(w, id, q);
            }
        }
        if c::dec_timer_pvar_i32(w, id, pv::LIFE) != 0 { w.mm(id).cmd = 1; }
    }
    // `0x277a80(0.525, 30°·dt, 17°·dt, mine, +0x60, +0x64)`: the wobble.
    {
        let (a, b) = (c::pf(w, id, pv::WOB_A), c::pf(w, id, pv::WOB_B));
        let k = f32::from_bits(0x3f06_0a92);
        let m = w.mm(id);
        m.rotation[0] = k * a.sin() * b.sin();
        m.rotation[1] = k * a.sin() * b.cos();
        c::set_pf(w, id, pv::WOB_A, c::add_rot(a, DT * std::f32::consts::FRAC_PI_6));
        c::set_pf(w, id, pv::WOB_B, c::add_rot(b, DT * 0.296_705_96));
    }
    if w.m(id).state & LANDED != 0 {
        let v = c::sub(c::pos(w, id), old);
        c::set_pv4(w, id, pv::VEL, v);
    }
}

/// The rest of the update (`LAB_002c0fb8` on): arming, hits, the proximity search or Ratchet's push, the oldest-mine
/// rule, the bob, the explosion.
fn after_move(w: &mut World, id: MobyId, drift: V, normal: V) {
    if w.m(id).state & ARMED == 0 && c::dec_timer_pvar_s16(w, id, pv::TIMER) != 0 {
        w.mm(id).state |= ARMED;
        c::set_pi32(w, id, pv::TARGET, 0);
    }
    if let Some(h) = w.get_hit(id, 0x83_0000, false) {
        if let Some(a) = h.attacker { if w.m(a).o_class != CLASS { w.mm(id).cmd = 1; } }
    }
    w.mm(id).hit_slot = 0xff;
    let st = w.m(id).state;
    if st & ARMED != 0 {
        let pos = c::pos(w, id);
        let hero = w.hero_moby.map(|h| w.m(h).position).unwrap_or(pos);
        let touch = c::dist2(hero, pos) < 0.75 && (pos[2] - w.hero.pos[2].to_f32()).abs() < 0.25 && st & SEEKING != 0;
        if st & LANDED == 0 || touch {
            w.mm(id).cmd = 1;
        } else if 0.75 <= c::dist3(hero, pos) {
            seek_search(w, id);
        } else {
            // Ratchet on it: pushed out from under him.
            let d = c::set_len3(c::sub(pos, hero), 0.75);
            let mut to = c::add(hero, d);
            let mut from = pos;
            walker::move_ground(w, id, 0.4, 0.25, 0.25, f32::from_bits(0x3f06_0a92), &mut from, &mut to, 0);
            let z = w.m(id).position[2];
            c::set_pos(w, id, [from[0], from[1], z, from[3]]);
        }
    }
    c::set_pi32(w, id, pv::LURED, 0);
    if c::pi32(w, id, pv::BORN) < 0 {
        let pos = c::pos(w, id);
        fx::death_explosion(w, f32::from_bits(0x3ea8_f5c3), 13.0, Some(id), pos, -1);
        w.delete_moby(id);
    }
    // The bob: the amplitude decays to 0.05; shown 0.4 up.
    let amp = c::pf(w, id, pv::BOB_AMP);
    if 0.05 < amp {
        let a = amp - 0.012_000_024 * amp;
        c::set_pf(w, id, pv::BOB_AMP, if a < 0.05 { f32::from_bits(0x3d4c_cccd) } else { a });
    }
    let fade = bob_fade(w, id);
    let ph = c::add_rot(c::pf(w, id, pv::BOB_PHASE), DT * std::f32::consts::TAU);
    c::set_pf(w, id, pv::BOB_PHASE, ph);
    let amp = c::pf(w, id, pv::BOB_AMP);
    w.mm(id).position[2] += amp * ph.sin() * fade + 0.4;
    if w.m(id).cmd == 0 {
        // `FUN_0026eec8(0.25)`: the blob shadow.
        crate::shadows::blob(w, 0.25, id);
        return;
    }
    explode(w, id, drift, normal);
}

/// The proximity search over the target list (module doc).
/// The gold Mine Glove 0x13e531 (item 17), the byte as `0x2bfe40` reads it.
fn gold(w: &World) -> u8 { w.hero.weapons.gold.get(17).copied().unwrap_or(0) }

fn seek_search(w: &mut World, id: MobyId) {
    let pos = c::pos(w, id);
    let mut range = (gold(w) as f32 + 1.0) * 4.0;
    if c::pi32(w, id, pv::LURED) != 0 { range *= 3.0; }
    let list: Vec<MobyId> = w.svc.targets.clone();
    let mut best: Option<MobyId> = None;
    for t in list {
        let tp = w.m(t).position;
        let d3 = c::dist3(pos, tp);
        if 30.0 < d3 { continue; }
        if d3 < 1.5 {
            w.mm(id).cmd = 1;
            break;
        }
        if is_crate(w, t) { continue; }
        let rad = range + tg::record_radius(w.m(t)).map_or(0.0, |r| r as f32 * 0.125);
        let d2 = c::dist2(pos, tp);
        let cur_dead = target(w, id).is_none_or(|m| w.m(m).is_deleted());
        if d2 < rad && (tp[2] - pos[2]).abs() < 2.0 && cur_dead {
            if let Some(b) = best { if c::dist2(pos, w.m(b).position) <= d2 { continue; } }
            w.mm(id).state |= SEEKING;
            let t3 = w.ticks(3);
            w.anim_blend(id, 1, 0, t3);
            w.play_sound(0, 0, id);
            c::set_pf(w, id, pv::SPEED, 0.0);
            let t600 = w.ticks(600);
            c::set_pi32(w, id, pv::LIFE, t600);
            c::set_pi16(w, id, pv::HOP, 1);
            let t7 = w.ticks(7);
            c::set_pi32(w, id, pv::HOP_T, t7);
            best = Some(t);
        }
    }
    if let Some(b) = best { c::set_pi32(w, id, pv::TARGET, b as i32 + 1); }
}

/// The explosion (module doc).
fn explode(w: &mut World, id: MobyId, _drift: V, _normal: V) {
    let pos = c::pos(w, id);
    let cam = w.camera.map(|x| x.to_f32());
    let dcam = c::dist2(pos, cam);
    let water = c::pi16(w, id, pv::WATER) != 0;
    // The blast sphere 2·(0x13e531·0.5 + 1).
    let g = gold(w);
    let gk = g as f32 * 0.5 + 1.0;
    if !water { attack::area_hit(w, gk + gk, pos, id, 3.0, 0.25, 1.5, None, 0x81_0000, 4, 1); }
    let gz = w.ground_height(Pf::f(0.5), pvq(pos), 0x20).to_f32();
    if !water {
        if gz + 2.0 < pos[2] {
            let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 4.0, scale: 1.0, light: 7.0, streaks: 3, sparks: 3, puffs: 5, debris: 1, sound: 2, shake: false };
            fx::beam_explosion(w, &b, Some(id), pos);
        } else {
            smoke(w, pos, g);
        }
    } else {
        let surf = w.ground_height(Pf::f(0.5), pvq([pos[0], pos[1], pos[2] + 3.5, pos[3]]), 0).to_f32();
        bomb_water::bubbles(w, pos, surf, 1.0, -2.0);
        bomb_water::scorch(w, pos, surf, &bomb_water::MINE_SCORCH);
    }
    w.mm(id).cmd = 0;
    w.play_sound(if g == 0 { 2 } else { 3 }, 0, id);
    let amp = if dcam < 20.0 { 0.4 - dcam * 0.0175 } else { f32::from_bits(0x3d4c_ccd0) };
    let t = w.ticks(25);
    w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp, ticks: t });
    fx::light_spawn(w, if g == 0 { &fx::LIGHT_BOMB } else { &fx::LIGHT_GOLD }, pos);
    w.delete_moby(id);
}

/// The ground explosion's 15 type-16 puffs (`0x2bfe40` at 0x2c1aa8): per puff `randi(4)` picks the kind (0 smoke,
/// 1 fire, 2 flash, 3 embers), a velocity `(randf(±0.5·dt), randf(±0.5·dt), randf(4·dt, [14, 22, 12, 0][kind]·dt))`, a
/// spot `randf(0, 0.25)` (0.5 for fire) out at a random angle and sunk by its cosine of 45°, the rise less 8·dt per
/// unit out, then the kind's colours, size and life. The gold mine (`gold` ≠ 0): velocities, spot and sizes ×1.5
/// (0x13e531·0.5 + 1) and every colour through `0x270fa8(c, gold)` ([`fx::colour_shift`]).
fn smoke(w: &mut World, pos: V, gold: u8) {
    let gk = gold as f32 * 0.5 + 1.0;
    let sh = |c: u32| fx::colour_shift(c, gold);
    use crate::particles::{tween_color, type16};
    let n = 15;
    for _ in 0..n {
        let kind = w.rng.randi(4);
        let tops = [DT * 14.0, DT * 22.0, DT * 12.0, 0.0];
        let vx = w.rng.randf(DT * -0.5, DT * 0.5) * gk;
        let vy = w.rng.randf(DT * -0.5, DT * 0.5) * gk;
        let mut vz = w.rng.randf(DT * 4.0, tops[kind as usize]) * gk;
        let r = w.rng.randf(0.0, if kind == 1 { 0.5 } else { 0.25 }) * gk;
        let a = w.rng.rand_angle();
        let at = [a.cos() * r + pos[0], a.sin() * r + pos[1], pos[2] - r * f32::from_bits(0x3f49_0fd8).cos(), pos[3]];
        vz -= r * DT * 8.0;
        let mut vel = [vx, vy, vz, 0.0];
        let (size, c1, c2, life, k) = match kind {
            1 => {
                let size = w.rng.randf(50000.0, 100000.0);
                (size * gk, sh(0x3f08_1020), sh(0x0f08_1020), w.ticks(0xb4), 1)
            }
            0 => {
                let size = w.rng.randf(200_000.0, 300_000.0);
                let (a, b) = (w.ticks(0xb4), w.ticks(0xf0));
                let life = w.rng.rand_range(a, b);
                (size * gk, sh(0x1f10_1820), 0x10_1010, life, 0)
            }
            2 => {
                let c = if 0x27 < w.rng.randi(100) { 0x2f48_6078 } else { 0x5ff8_f8f8 };
                let (a, b) = (w.ticks(0x1e), w.ticks(0x2d));
                let life = w.rng.rand_range(a, b);
                (150_000.0 * gk, sh(c), sh(0x0f00_0020), life, 2)
            }
            _ => {
                let f = w.rng.randf(0.25, 1.0);
                let c1 = tween_color(f.to_bits(), 0x7f00_0000, 0x7f18_2030);
                let f = w.rng.randf(0.5, 1.0);
                let c2 = tween_color(f.to_bits(), 0, 0x5f_5f5f);
                let s = w.rng.randf(0.0, 1.0) * DT;
                let pitch = w.rng.rand_angle();
                let v = tg::polar(s, a, pitch);
                vel = [v[0], v[1], v[2], 0.0];
                let size = w.rng.randf(200_000.0, 300_000.0);
                let (lo, hi) = (w.ticks(0xf0), w.ticks(300));
                let life = w.rng.rand_range(lo, hi);
                (size * gk, sh(c1), sh(c2), life, 3)
            }
        };
        let s = type16::Spawn { size, pos: at, vel, c1, c2, life, kind: k };
        crate::moby_update::creature::projectile::part16(w, &s);
    }
}

/// `0x2bfa78(pvars, mine, 1)`: the landing preview ([`tg::arc_landing`] with gravity 9.8, only faces ending it): held,
/// from the launch point with the aimed velocity for 120 ticks; out, from the mine for the arming timer's ticks; a new
/// mine in the glove ends an older one's previews. The point pulled toward the camera and the glove reticle registered
/// (`0x2bf420`, the bomb's `0x2c23c0` copy: [`tg::BOMB_RETICLE`]).
fn preview(w: &mut World, id: MobyId) -> bool {
    use crate::targeting::{ArcStart, LineHit, Prims};
    if w.hero.state == 0x72 || w.svc.game_mode != 0 { return false; }
    let held = w.m(id).state == 0;
    let v = c::pv4(w, id, pv::VEL);
    let (from, budget) = if held {
        (crate::hero::gloves::launch_at(w.hero, 0.490_82), w.ticks(0x78))
    } else {
        if w.hero.weapons.glove.held.is_some() { c::set_pi16(w, id, pv::NEW_HELD, 1); }
        let q = c::pos(w, id);
        ([q[0], q[1], q[2]], c::pi16(w, id, pv::TIMER) as i32)
    };
    let start = ArcStart {
        from,
        vel: [v[0], v[1], v[2]],
        budget,
        gravity: DT2 * 9.8,
        first: from,
        first_ignore: w.hero_moby,
        ignore: Some(id),
        own: [w.hero_moby, None],
        gold: false,
        prims: Prims::Ignore,
        first_test: false,
    };
    let landing = {
        let ww: &World = w;
        tg::arc_landing(
            &start,
            |a, b, ignore| {
                let h = ww.coll_line(pvq([a[0], a[1], a[2], 0.0]), pvq([b[0], b[1], b[2], 0.0]), 0x12, ignore)?;
                Some(LineHit { point: h.point, normal: h.normal, moby: h.moby, kind: h.kind, surface: h.surface_id() })
            },
            |m| { let q = ww.m(m).position; [q[0], q[1], q[2]] },
            |_, _| None,
        )
    };
    let Some(l) = landing else { return false };
    let cam = [w.camera[0].to_f32(), w.camera[1].to_f32(), w.camera[2].to_f32()];
    let point = tg::pull_toward(cam, l.point);
    c::set_pv4(w, id, pv::POINT, [point[0], point[1], point[2], 0.0]);
    c::set_pv4(w, id, pv::NORMAL, [l.normal[0], l.normal[1], l.normal[2], 0.0]);
    w.svc.reticles.register(w.counter, tg::Reticle { point, normal: l.normal, style: &tg::BOMB_RETICLE });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_update::classes::bomb_water::tests::{pool, Bench};
    use crate::moby_update::creature::fx::LIGHT_CLASS;

    /// A mine thrown into water (the pool's water at 12 over a floor at 8): the water entry (16 type-35 drops, the
    /// splash), and on the next tick the explosion in water (its path starts at its shown position, 0.4 above the
    /// physical one, so it meets the surface again while in water) — no area hit, 150 bubbles (type 34), the scorch's
    /// 150 type-64 and 20 type-15 from the surface, class sound 2, the explosion light; deleted.
    #[test]
    fn explodes_in_water() {
        let mut b = Bench::new(pool(8.0, 12.0), &[CLASS], [40.0, 40.0, 20.0]);
        let m = b.create(CLASS);
        let mut rng = crate::rng::Rng::new();
        init_held(&mut b.table.mobys[m], &mut rng, [20.0, 20.0, 13.0], 0);
        p::set_v4f(&mut b.table.mobys[m].pvars, pv::VEL, [0.02, 0.0, 0.0, 0.0]);
        release(&mut b.table, m, &mut rng, [20.0, 20.0, 13.0]);
        let (d35, mut entered, mut entry_tick) = (b.parts(35), false, 0u64);
        for _ in 0..400 {
            let (p34, p64, p15, lights, sounds) = (b.parts(34), b.parts(64), b.parts(15), b.alive(LIGHT_CLASS), b.svc.sounds.len());
            b.tick(None);
            if !entered && p::i16(&b.table.mobys[m].pvars, pv::WATER) == 1 {
                entered = true;
                entry_tick = b.counter;
                assert_eq!(b.parts(35) - d35, 16, "16 drops");
            }
            if b.table.mobys[m].is_deleted() {
                assert!(entered && entry_tick + 1 == b.counter, "the tick after the entry ({entry_tick}): {}", b.counter);
                // 150 bubbles, plus the tick's sinking bubble when its 1-in-5 chance came up.
                assert!((150..=151).contains(&(b.parts(34) - p34)), "bubbles {}", b.parts(34) - p34);
                assert_eq!((b.parts(64) - p64, b.parts(15) - p15), (150, 20), "the scorch from the surface");
                assert!(b.svc.sounds[sounds..].iter().any(|e| e.o_class == CLASS && e.index == 2), "sound 2");
                assert_eq!(b.alive(LIGHT_CLASS), lights + 1, "the explosion light");
                return;
            }
        }
        panic!("never exploded");
    }
}
