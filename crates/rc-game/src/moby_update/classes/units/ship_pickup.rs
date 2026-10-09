//! **The ship pickups, classes 1218 (missiles) and 1220 (health), and their parachute 1219** (created by code only:
//! the spawner level11 `0x30f5a8`, a shot-down fighter 1319 drops one, [`super::ship_fighter`]): the pickup
//! hangs under its parachute (`0x30fd98` makes it), drifts down and is drawn toward the flown ship 1242 / 69 / 1379
//! ([`crate::vehicle`]) within 40; the ship touching it (or within 4) collects it: five missiles or a quarter of the
//! health. Shooting the parachute drops the pickup. Read from the level11 decomp and disassembly. Native `f32`.
//!
//! **Pickup pvars** (0x38): +0x00 the hanging point (the parachute's joint 0), +0x10 the velocity, +0x20 the offset
//! from it to the parachute, +0x30 the swing phase, +0x34 the parachute (index + 1). **Parachute pvars** (0x74): +0x20
//! the damage record (health), +0x60 the flash record, +0x70 the pickup (index + 1).
//!
//! ## Coverage (the pickup, `0x30f728`)
//! | address | what | port |
//! |---|---|---|
//! | entry | joint 0's point; `coll_sphere(0.55, it, 0, m)`: the world or a moby not a ship (0x4da, 0x45, 0x563) → `SpawnBeamExplosion(0, 0, 0, 0, 9, 1, 0, m, velocity, the point, 3, 2, 5, −1, 0)`, → 5; a ship → touched | [`update`] (`fx::beam_explosion`) |
//! | entry | below state 4: Ratchet in 0x32 with the record's ship alive: d = \|ship − position\|, e = \|ship − (position + hanging point)/2\|xy; touched, d < 4 or e < 4 → collision off (and the parachute's); 1218: missiles + 5 (u8) up to the most, `PlayClassSound(4, 0, ship)`; 1220: health + a quarter of the most, up to it, `PlayClassSound(5, 0, ship)`; → 4 | [`update`] |
//! | 0 | +0x34 = 0; `0x30f660`; +0x30 = 0; → 1 | [`update`], [`attach`] |
//! | 1 | velocity ·= 0.9, z −= 9.8·dt²; the hanging point += velocity; position = it + offset; `0x30f660` held: the parachute's sequence (+0x52) 1 → velocity ·= 0.8, → 2 | [`update`] |
//! | 2 | rot.z += 2°·dt; phase += 90°·dt, rot.y = cos(phase)·45°; velocity ·= 0.8, z −= 9.8·dt²; within 40 of the ship and drawn: velocity += (ship − position) at 0.8 / √d; the hanging point += velocity; position = it + offset; `0x30f660` held and the parachute in state 2 → 2 | [`update`] |
//! | 3 | z −= 9.8·dt² (velocity); position += velocity | [`update`] |
//! | 4 | → 5 | [`update`] |
//! | 5, z < 20 | `DeleteMoby` | [`update`] |
//! | tail | position clamped to [20, 1003]; the blob shadow (`0x26eec8(0.5, m)`) | [`update`] (`crate::shadows::blob`) |
//! | 0x30f660 | no parachute: `0x30fd98`; a live parachute: its position / Euler = the pickup's, `MobyBuildMatrix`; the hanging point = its joint 0, offset = its position − that; 1. None: the hanging point = position, offset 0; 0 | [`attach`] |
//! | 0x30fd98 | `CreateMoby(0x4c3)`: distances 0xff, drawn, state 0, +0xbc = 0, the pickup's position / Euler, +0x70 = the pickup; sequence 2 (`MobyAnimBlend(2, 13, 0)`), `MobyBuildMatrix`, Ratchet's light (`0x272078`) | [`make_chute`] |
//! | 0x30f5a8 | `CreateMoby(class)`: distances 0xff, drawn, state 0, +0xbc = 0, position, velocity, rot (0, 45°, random); `MobyBuildMatrix`, Ratchet's light | [`spawn`] |
//!
//! ## Coverage (the parachute 1219, `0x310028`)
//! | address | what | port |
//! |---|---|---|
//! | entry | the pickup gone or deleted: below state 3 → 4 | [`chute_update`] |
//! | 0 | sequence 1 (blend 15) → 1 | [`chute_update`] |
//! | 1 | the anim wrapped (+0x70 & 2) → 2, +0xa4 = 0xff | [`chute_update`] |
//! | 2 | the hits (`0x30fe50`); the anim wrapped → sequence 1 (blend 10) | [`chute_update`], [`chute_hits`] |
//! | 3 | alpha − 1 while not 0; the key time past 13.5 → 4 | [`chute_update`] (`ground::key_time`) |
//! | 4 | `DeleteMoby` | [`chute_update`] |
//! | 0x30fe50 | not 4: `MobyGetHitMessage(m, 0x330000, 0)` (a `STUB_printf` of it: n/a); `0x26f378(m, hit, +0x20, 0, &out, 0, 0, 4)`: reactions 1 / 2 → health 0; out > 1: health ≤ damage → health 0, not targetable (mode &= ~0x1000), the flash red 0x78 (`0x272318`), sequence 2 (blend 3), the pickup → 3, → 3; else health −= damage, the flash red 0xfa, +0x26 = `ticks(60)`, sequence 4 (blend 3); +0xa4 = 0xff. Always the flash (`0x2723f8`) | [`chute_hits`] (`damage::resolve`, `flash::{start, update}`) |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::fx;
use crate::moby_update::creature::{self as c, add, dist2, dist3, flash, set_len3, DT, DT2};
use crate::moby_update::services::{pf as to_pf, pv, World};
use crate::moby_update::story;

pub const REFERENCE_LEVEL: u32 = 11;
pub const UPDATE_FN: u32 = 0x30_f728;
pub const SPAWN_FN: u32 = 0x30_f5a8;
pub const MISSILES: i16 = 0x4c2;
pub const HEALTH: i16 = 0x4c4;
pub const CLASSES: [i16; 2] = [MISSILES, HEALTH];
pub const CHUTE_FN: u32 = 0x31_0028;
pub const CHUTE: i16 = 0x4c3;
pub const CHUTE_CLASSES: [i16; 1] = [CHUTE];

pub mod pvo {
    pub const HANG: usize = 0x00;
    pub const VEL: usize = 0x10;
    pub const OFFSET: usize = 0x20;
    pub const PHASE: usize = 0x30;
    pub const CHUTE: usize = 0x34;
    pub const LEN: usize = 0x38;
}

/// The parachute's pvars.
pub mod chute_pvo {
    pub const RECORD: usize = 0x20;
    pub const FLASH: usize = 0x60;
    pub const PICKUP: usize = 0x70;
    pub const LEN: usize = 0x74;
}

/// The flown ships (Pokitaru's jet, Gemlik's ship, the fleet's ship).
pub const SHIPS: [i16; 3] = [0x4da, 0x45, 0x563];

fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn set_link(w: &mut World, id: MobyId, o: usize, m: Option<MobyId>) { c::set_pi32(w, id, o, m.map_or(0, |m| m as i32 + 1)) }
fn alive(w: &World, m: MobyId) -> bool { let s = w.m(m).state; s != 0xfe && s != 0xfd }

/// Level11 `0x30f5a8(pos, vel, class)` (module doc).
pub fn spawn(w: &mut World, pos: [f32; 4], vel: [f32; 4], class: i16) -> Option<MobyId> {
    let id = w.create_moby(class)?;
    story::pvars(w, id, pvo::LEN);
    {
        let m = w.mm(id);
        m.draw_dist = 0xff;
        m.update_dist = 0xff;
        m.visible = 1;
        m.state = 0;
        m.cmd = 0;
        m.position = pos;
    }
    c::set_pv4(w, id, pvo::VEL, vel);
    let a = w.rng.rand_angle();
    {
        let m = w.mm(id);
        m.rotation[0] = 0.0;
        m.rotation[2] = a;
        m.rotation[1] = std::f32::consts::FRAC_PI_4;
    }
    w.build_matrix(id);
    story::copy_hero_light(w, id);
    Some(id)
}

/// Level11 `0x30fd98(pickup)` (module doc).
fn make_chute(w: &mut World, p: MobyId) -> Option<MobyId> {
    let id = w.create_moby(CHUTE)?;
    story::pvars(w, id, chute_pvo::LEN);
    let (pos, rot) = (w.m(p).position, w.m(p).rotation);
    {
        let m = w.mm(id);
        m.draw_dist = 0xff;
        m.update_dist = 0xff;
        m.visible = 1;
        m.state = 0;
        m.cmd = 0;
        m.position = pos;
        m.rotation = rot;
    }
    set_link(w, id, chute_pvo::PICKUP, Some(p));
    if w.m(id).anim.seq_b != 2 { w.anim_blend(id, 2, 0xd, 0); }
    w.build_matrix(id);
    story::copy_hero_light(w, id);
    Some(id)
}

/// Level11 `0x30f660(m, pvars)`: the parachute follows; the hanging point and offset (module doc).
fn attach(w: &mut World, id: MobyId) -> bool {
    if link(w, id, pvo::CHUTE).is_none() {
        let ch = make_chute(w, id);
        set_link(w, id, pvo::CHUTE, ch);
    }
    match link(w, id, pvo::CHUTE).filter(|&ch| alive(w, ch)) {
        Some(ch) => {
            let (pos, rot) = (w.m(id).position, w.m(id).rotation);
            {
                let m = w.mm(ch);
                m.position = pos;
                m.rotation = rot;
            }
            w.build_matrix(ch);
            let j = w.joint_point(ch, 0);
            c::set_pv4(w, id, pvo::HANG, j);
            let cp = w.m(ch).position;
            c::set_pv4(w, id, pvo::OFFSET, [cp[0] - j[0], cp[1] - j[1], cp[2] - j[2], cp[3] - j[3]]);
            true
        }
        None => {
            let p = w.m(id).position;
            c::set_pv4(w, id, pvo::HANG, p);
            c::set_pv4(w, id, pvo::OFFSET, [0.0; 4]);
            false
        }
    }
}

fn scale_vel(w: &mut World, id: MobyId, k: f32) {
    let v = c::pv4(w, id, pvo::VEL);
    c::set_pv4(w, id, pvo::VEL, v.map(|x| x * k));
}

fn fall(w: &mut World, id: MobyId) {
    let mut v = c::pv4(w, id, pvo::VEL);
    v[2] -= DT2 * 9.8;
    c::set_pv4(w, id, pvo::VEL, v);
}

/// The hanging point moved by the velocity and the position from it.
fn hang(w: &mut World, id: MobyId) {
    let h = add(c::pv4(w, id, pvo::HANG), c::pv4(w, id, pvo::VEL));
    c::set_pv4(w, id, pvo::HANG, h);
    let p = add(h, c::pv4(w, id, pvo::OFFSET));
    w.mm(id).position = p;
}

/// Level11 `0x30f728` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pvo::LEN { story::pvars(w, id, pvo::LEN); }
    let ship = w.svc.vehicle.moby;
    let joint = w.joint_point(id, 0);
    let mut touched = false;
    if let Some(h) = w.coll_sphere(pv(joint), to_pf(f32::from_bits(0x3f0c_cccd)), 0, Some(id)) {
        if h.moby.is_none_or(|m| !SHIPS.contains(&w.m(m).o_class)) {
            let b = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 0.0, flash2: 0.0, flash_dist: 9.0, scale: 1.0, light: 0.0, streaks: 3, sparks: 2, puffs: 5, debris: 0, sound: -1, shake: false };
            fx::beam_explosion(w, &b, Some(id), joint);
            w.mm(id).state = 5;
        } else {
            touched = true;
        }
    }
    let (mut d, mut e) = (100_000.0f32, 100_000.0f32);
    if w.m(id).state < 4 {
        if w.hero.state == 0x32 {
            if let Some(s) = ship.filter(|&s| alive(w, s) && SHIPS.contains(&w.m(s).o_class)) {
                let (sp, p) = (w.m(s).position, w.m(id).position);
                d = dist3(sp, p);
                let h = c::pv4(w, id, pvo::HANG);
                let mid = [(p[0] + h[0]) * 0.5, (p[1] + h[1]) * 0.5, (p[2] + h[2]) * 0.5, (p[3] + h[3]) * 0.5];
                e = dist2(sp, mid);
            }
        }
        if touched || d < 4.0 || e < 4.0 {
            w.mm(id).has_collision = false;
            if let Some(ch) = link(w, id, pvo::CHUTE) { w.mm(ch).has_collision = false; }
            let live_ship = ship.filter(|&s| alive(w, s));
            match w.m(id).o_class {
                MISSILES => {
                    let v = &mut w.svc.vehicle;
                    v.missiles = v.missiles.wrapping_add(5);
                    if let Some(s) = live_ship { w.play_sound(4, 0, s); }
                    let v = &mut w.svc.vehicle;
                    if v.missiles_max < v.missiles { v.missiles = v.missiles_max; }
                }
                HEALTH => {
                    let v = &mut w.svc.vehicle;
                    v.health += v.health_max * 0.25;
                    if let Some(s) = live_ship { w.play_sound(5, 0, s); }
                    let v = &mut w.svc.vehicle;
                    if v.health_max < v.health { v.health = v.health_max; }
                }
                _ => {}
            }
            w.mm(id).state = 4;
        }
    }
    match w.m(id).state {
        0 => {
            seti_chute_none(w, id);
            attach(w, id);
            c::set_pf(w, id, pvo::PHASE, 0.0);
            w.mm(id).state = 1;
        }
        1 => {
            scale_vel(w, id, f32::from_bits(0x3f66_6666));
            fall(w, id);
            hang(w, id);
            if attach(w, id) {
                let ch = link(w, id, pvo::CHUTE);
                if ch.is_some_and(|ch| w.m(ch).anim.seq_a == 1) {
                    scale_vel(w, id, f32::from_bits(0x3f4c_cccd));
                    w.mm(id).state = 2;
                }
            }
        }
        2 => {
            let rz = c::add_rot(w.m(id).rotation[2], DT * 0.034_906_585);
            w.mm(id).rotation[2] = rz;
            let ph = c::add_rot(c::pf(w, id, pvo::PHASE), DT * std::f32::consts::FRAC_PI_2);
            c::set_pf(w, id, pvo::PHASE, ph);
            w.mm(id).rotation[1] = ph.cos() * std::f32::consts::FRAC_PI_4;
            scale_vel(w, id, f32::from_bits(0x3f4c_cccd));
            fall(w, id);
            if let Some(s) = ship {
                if d < 40.0 && w.m(id).visible != 0 {
                    let k = 0.8 / d.sqrt();
                    let dv = set_len3(c::sub(w.m(s).position, w.m(id).position), k);
                    let v = add(c::pv4(w, id, pvo::VEL), dv);
                    c::set_pv4(w, id, pvo::VEL, v);
                }
            }
            hang(w, id);
            if attach(w, id) {
                let ch = link(w, id, pvo::CHUTE);
                if ch.is_some_and(|ch| w.m(ch).state == 2) { w.mm(id).state = 2; }
            }
        }
        3 => {
            fall(w, id);
            let p = add(w.m(id).position, c::pv4(w, id, pvo::VEL));
            w.mm(id).position = p;
        }
        4 => w.mm(id).state = 5,
        _ => {
            w.delete_moby(id);
            return;
        }
    }
    let p = w.m(id).position;
    if p[2] < 20.0 {
        w.delete_moby(id);
        return;
    }
    let cl = |x: f32| x.clamp(20.0, 1003.0);
    w.mm(id).position = [cl(p[0]), cl(p[1]), cl(p[2]), p[3]];
    crate::shadows::blob(w, 0.5, id);
}

fn seti_chute_none(w: &mut World, id: MobyId) { set_link(w, id, pvo::CHUTE, None) }

/// Level11 `0x310028`: the parachute (module doc).
pub fn chute_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < chute_pvo::LEN { story::pvars(w, id, chute_pvo::LEN); }
    let gone = link(w, id, chute_pvo::PICKUP).is_none_or(|p| !alive(w, p));
    if gone && w.m(id).state < 3 { w.mm(id).state = 4; }
    match w.m(id).state {
        0 => {
            if w.m(id).anim.seq_b != 1 { w.anim_blend(id, 1, 0, 0xf); }
            w.mm(id).state = 1;
        }
        1 => {
            if w.m(id).anim.flags & 2 != 0 {
                w.mm(id).state = 2;
                w.mm(id).hit_slot = 0xff;
            }
        }
        2 => {
            chute_hits(w, id);
            if w.m(id).anim.flags & 2 != 0 && w.m(id).anim.seq_b != 1 { w.anim_blend(id, 1, 0, 10); }
        }
        3 => {
            let a = w.m(id).alpha;
            if a != 0 { w.mm(id).alpha = a - 1; }
            if 13.5 < crate::moby_update::creature::ground::key_time(w, id) { w.mm(id).state = 4; }
        }
        4 => w.delete_moby(id),
        _ => {}
    }
}

/// Level11 `0x30fe50(m, pvars, &health)` (module doc).
fn chute_hits(w: &mut World, id: MobyId) {
    use chute_pvo::{FLASH, PICKUP, RECORD};
    if w.m(id).state != 4 {
        let hit = w.get_hit(id, 0x33_0000, false);
        let r = c::damage::resolve(w, id, hit, RECORD, 0, 4);
        if matches!(r.reaction, 1 | 2) { c::set_pf(w, id, RECORD, 0.0); }
        if 1 < r.out5 {
            let dmg = r.damage;
            if c::pf(w, id, RECORD) <= dmg {
                c::set_pf(w, id, RECORD, 0.0);
                w.mm(id).mode &= !mode::TARGETABLE;
                c::set_pu8(w, id, FLASH + 7, 0x78);
                flash::start(w, id, FLASH);
                if w.m(id).anim.seq_b != 2 { w.anim_blend(id, 2, 0, 3); }
                if let Some(p) = link(w, id, PICKUP).filter(|&p| alive(w, p)) { w.mm(p).state = 3; }
                w.mm(id).state = 3;
            } else {
                let hp = c::pf(w, id, RECORD) - dmg;
                c::set_pf(w, id, RECORD, hp);
                c::set_pu8(w, id, FLASH + 7, 0xfa);
                let t = w.ticks(0x3c) as i16;
                c::set_pi16(w, id, RECORD + 6, t);
                flash::start(w, id, FLASH);
                if w.m(id).anim.seq_b != 4 { w.anim_blend(id, 4, 0, 3); }
            }
        }
        w.mm(id).hit_slot = 0xff;
    }
    flash::update(w, id, FLASH);
}
