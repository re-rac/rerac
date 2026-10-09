//! **Gemlik's space fighters, class 111** (level13 `0x2c4428`; census U457; 8 placed, a squadron under a leader moby
//! +0xbc). They wait hidden until their leader's command is 4, then appear on their path (+0xa0) at the point nearest
//! 40 behind the camera and fly it (`0x2c3fc0`): toward a target on the path (the leader's position, or with Ratchet's
//! ship 69 flying: halfway to it, 100 out from the leader; no leader: 140 ahead of the ship), choosing the shorter way
//! round (turning about only slowly, when slow, unseen or within 40 of Ratchet), easing their speed to a sixtieth of
//! the distance (at most 100·dt), banking into their turns and facing Ratchet. With the ship in state 4 and at most
//! two squadron mates attacking, a seen fighter attacks for `ticks(600)`: every `ticks(6)` + 1.3 × the frame load
//! (when the frame is not loaded beyond 0.9) a fighter shot 1017 (range 121, 50·dt, 5) at the ship, aimed off by a
//! shrinking spread circling it and leading its motion; after `ticks(20)` unseen or the time out it rests
//! (`ticks(30)` / `ticks(120)`). Nine health; hits push it; shot down: a beam explosion (5 / 3 / 10, no sound),
//! a ship pickup (224 one time in ten, 228 six in ten; `0x2e1c50`, as level 17's spawn) 4 behind it toward the camera, the leader's count +0x10e − 1,
//! hidden until the leader calls again (health back). Its hum loops (sound 1); it is kept inside 20..1003.
//!
//! **Pvars**: +0x20 the damage record, +0x60 the flash, +0x70 the velocity, +0x84 / +0x88 the pitch / yaw turn
//! velocities, +0x90 the path point it appeared at, +0xa0 the path, +0xa4 the shot timer, +0xa8 ticks unseen, +0xac
//! the spread, +0xb0 its angle, +0xb4 / +0xb6 s16 the next / last node, +0xb8 the attack timer, +0xbc the leader,
//! +0xcc the speed, +0xd0 the way round (±1), +0xd2 s16 the turn-about timer, +0xd4 the hum.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2c4428` | the states (module doc) | [`update`] |
//! | `0x2c37e0` | the hits (0x330000, column 4): reaction 1 / 2 → no health; damage ≥ health → untargetable, flash 0x78, 3; else health − damage, flash 0xfa, cooldown `ticks(60)`, pushed 0.1 along the hit and `rand_vec(−0.1, 0.1)` | [`hits`] |
//! | `0x2c3fc0` / `0x2c3ac8` | the flight's target and the path point nearest it (searching both ways from the node, a growing bias) | [`fly`], [`nearest`] |
//! | `0x2c3c88` | the move between the nodes (the velocity turned toward the next by the fraction covered, at the speed) | [`advance`] |
//! | `0x2c3988` | the turn: `SpringTurn2(yaw, 8π·dt², 2π·dt², 8π·dt)`, `SpringTurn(pitch, …)`, the bank from the yaw rate (±90°, × 0.98) | [`face`] |
//! | `0x2c3f08` | 140 ahead of the ship along its yaw / pitch (+0x90 / +0x94) × its speed | [`ahead_of_ship`] |
//!
//! Read from the level13 decomp and disassembly. [L] `0x2c3988`'s last call (`0x2731d0`, the angle wrap) returns into
//! nothing. Native `f32`.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::{self as c, damage, flash, fx, turn};
use crate::moby_update::services::World;
use crate::moby_update::story;
use std::f32::consts::FRAC_PI_2;

pub const REFERENCE_LEVEL: u32 = 13;
pub const UPDATE_FN: u32 = 0x2c_4428;
pub const CLASSES: [i16; 1] = [111];

const DT: f32 = c::DT;
const DT2: f32 = c::DT2;
const SHIP: i16 = 0x45;
const SHIP2: i16 = 0x4da;

mod pv_ {
    pub const D: usize = 0x20;
    pub const COOLDOWN: usize = 0x26;
    pub const F: usize = 0x60;
    pub const VEL: usize = 0x70;
    pub const PITCH_V: usize = 0x84;
    pub const YAW_V: usize = 0x88;
    pub const POINT: usize = 0x90;
    pub const PATH: usize = 0xa0;
    pub const SHOT_T: usize = 0xa4;
    pub const UNSEEN: usize = 0xa8;
    pub const SPREAD: usize = 0xac;
    pub const SPREAD_A: usize = 0xb0;
    pub const NODE: usize = 0xb4;
    pub const LAST: usize = 0xb6;
    pub const ATTACK_T: usize = 0xb8;
    pub const LEADER: usize = 0xbc;
    pub const SPEED: usize = 0xcc;
    pub const WAY: usize = 0xd0;
    pub const TURN_T: usize = 0xd2;
    pub const HUM: usize = 0xd4;
    pub const SIZE: usize = 0xd8;
}
use pv_ as o;

/// The leader's pvar bytes: +0x10e the fighters out, +0x10f the attackers this tick.
pub const LEADER_OUT: usize = 0x10e;
pub const LEADER_ATTACKING: usize = 0x10f;

const BOOM: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 3.0, flash_dist: 9.0, scale: f32::from_bits(0x3f8c_cccd), light: 20.0, streaks: 5, sparks: 3, puffs: 10, debris: 0, sound: -1, shake: false };

fn path(w: &World, id: MobyId) -> Option<usize> { usize::try_from(c::pi32(w, id, o::PATH)).ok().filter(|&p| p < w.svc.splines.len()) }
fn pts(w: &World, p: usize) -> Vec<c::V> { w.svc.splines[p].iter().map(|q| q.map(f32::from_bits)).collect() }
fn leader(w: &World, id: MobyId) -> Option<MobyId> {
    usize::try_from(c::pi32(w, id, o::LEADER)).ok().filter(|&m| m < w.table.mobys.len()).filter(|&m| { let s = w.m(m).state; s != 0xfe && s != 0xfd })
}
fn ship(w: &World) -> Option<MobyId> { w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len()).filter(|&v| { let s = w.m(v).state; s != 0xfe && s != 0xfd }) }
fn hide(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.state = 4;
    m.has_collision = false;
    m.mode = (m.mode & !mode::TARGETABLE) | 0x41;
}

/// `0x2c37e0(m, P, D)`: the hits (module doc).
fn hits(w: &mut World, id: MobyId) {
    if w.m(id).state != 3 {
        let hit = w.get_hit(id, 0x33_0000, false);
        let res = damage::resolve(w, id, hit, o::D, 0, 4);
        if matches!(res.reaction, 1 | 2) { c::set_pf(w, id, o::D, 0.0); }
        if let Some(h) = res.hit.filter(|_| 1 < res.out5) {
            let dmg = h.damage.to_f32();
            let hp = c::pf(w, id, o::D);
            if hp <= dmg {
                c::set_pf(w, id, o::D, 0.0);
                w.mm(id).mode &= !mode::TARGETABLE;
                c::set_pu8(w, id, o::F + 7, 0x78);
                flash::start(w, id, o::F);
                w.mm(id).state = 3;
            } else {
                c::set_pf(w, id, o::D, hp - dmg);
                c::set_pu8(w, id, o::F + 7, 0xfa);
                let t = w.ticks(0x3c);
                c::set_pi16(w, id, o::COOLDOWN, t as i16);
                flash::start(w, id, o::F);
                let d = c::set_len3(h.dir.map(|x| x.to_f32()), 0.1);
                let p = c::add(c::pos(w, id), d);
                let r = w.rng.rand_vec(-0.1, 0.1);
                w.mm(id).position = c::add(p, [r[0], r[1], r[2], 0.0]);
            }
        }
        w.mm(id).hit_slot = 0xff;
    }
    flash::update(w, id, o::F);
}

/// `0x2c3ac8(0, t, path, node)` (module doc).
fn nearest(pts: &[c::V], t: c::V, node: i32) -> i32 {
    let n = pts.len() as i32;
    let mut best = 1e11f32;
    let (mut a, mut b) = (node, node);
    let mut i = 0;
    while i < n / 2 {
        a = a.rem_euclid(n);
        let mut da = c::dist3(pts[a as usize], t).abs();
        if best + i as f32 * 0.1 <= da + i as f32 * 0.4 {
            da = best;
            a = b;
        }
        b = ((n + node) - i).rem_euclid(n);
        best = c::dist3(pts[b as usize], t).abs();
        if da <= best + i as f32 * 0.4 {
            best = da;
            b = a;
        }
        i += 1;
        a = node + i;
    }
    b
}

/// `0x2c3f08(140, &t)` (module doc).
fn ahead_of_ship(w: &World, k: f32) -> c::V {
    let Some(v) = w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len()) else { return super::hero_pos(w) };
    let m = w.m(v);
    let yaw = c::add_rot(m.rotation[2], c::pf(w, v, 0x90));
    let pitch = c::add_rot(m.rotation[1], c::pf(w, v, 0x94));
    c::add(fx::polar(k * c::pf(w, v, 0x64), yaw, -pitch), m.position)
}

/// `0x2c3988(yaw, pitch, m, P)` (module doc).
fn face(w: &mut World, id: MobyId, yaw: f32, pitch: f32) {
    turn::spring_turn2_pvar(w, id, yaw, DT2 * 25.132_742, DT2 * 6.283_185_5, DT * 25.132_742, o::YAW_V);
    let mut v = c::pf(w, id, o::PITCH_V);
    let ry = turn::spring_turn(w.m(id).rotation[1], pitch, DT2 * 25.132_742, DT2 * 6.283_185_5, DT * 25.132_742, &mut v);
    c::set_pf(w, id, o::PITCH_V, v);
    w.mm(id).rotation[1] = ry;
    let rx = w.m(id).rotation[0];
    let mut x = rx + -c::pf(w, id, o::YAW_V) * ((FRAC_PI_2 - rx.abs()) / FRAC_PI_2);
    x = x.clamp(-FRAC_PI_2, FRAC_PI_2);
    w.mm(id).rotation[0] = x * 0.98;
}

/// `0x2c3c88(m, P, face)` (module doc). True: the node wrapped.
fn advance(w: &mut World, id: MobyId, p: usize) -> bool {
    let pts = pts(w, p);
    let n = pts.len() as i32;
    let get = |i: i16| pts.get(i.max(0) as usize).copied().unwrap_or([0.0; 4]);
    let (prev, node) = (get(c::pi16(w, id, o::LAST)), get(c::pi16(w, id, o::NODE)));
    let a = c::dist3(prev, c::pos(w, id));
    let l = c::dist3(prev, node);
    let s = c::pf(w, id, o::SPEED);
    let mut wrapped = false;
    let keep = if 0.0 <= l - a && s > l - a { false } else { s <= l };
    let mut t = if keep { a / l } else { 0.0 };
    if !keep {
        let cur = c::pi16(w, id, o::NODE);
        c::set_pi16(w, id, o::LAST, cur);
        let mut nx = cur + c::pu8(w, id, o::WAY) as i8 as i16;
        let over = n <= nx as i32;
        if over { nx = 0; }
        let under = nx < 0;
        if under { nx = (n - 1) as i16; }
        c::set_pi16(w, id, o::NODE, nx);
        wrapped = over || under;
        t = 0.0;
    }
    let t = t.clamp(0.05, 1.0);
    let target = get(c::pi16(w, id, o::NODE));
    let mut vel = c::scale(c::pv4(w, id, o::VEL), 1.0 - t);
    let d = c::scale(c::set_len3(c::sub(target, c::pos(w, id)), s), t);
    vel = c::set_len3(c::add(vel, d), s);
    c::set_pv4(w, id, o::VEL, vel);
    w.mm(id).position = c::add(c::pos(w, id), vel);
    let h = super::hero_pos(w);
    let p = c::pos(w, id);
    let yaw = c::atan(h[0] - p[0], h[1] - p[1]);
    let pitch = c::atan(c::dist2(p, h), h[2] - p[2]);
    face(w, id, yaw, -pitch);
    wrapped
}

/// `0x2c3fc0(m, P, 1)` (module doc).
fn fly(w: &mut World, id: MobyId) -> bool {
    let Some(p) = path(w, id) else { return false };
    let pts = pts(w, p);
    let n = pts.len() as i32;
    if n == 0 { return false; }
    let lead = leader(w, id);
    let ship_on = w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len()).is_some_and(|v| w.m(v).o_class == SHIP);
    let target = if !ship_on {
        lead.map_or_else(|| super::hero_pos(w), |l| w.m(l).position)
    } else {
        match lead {
            None => ahead_of_ship(w, 140.0),
            Some(l) => {
                let s = w.m(w.svc.vehicle.moby.unwrap()).position;
                let lp = w.m(l).position;
                c::add(c::set_len3(c::scale(c::sub(s, lp), 0.5), 100.0), lp)
            }
        }
    };
    let node = c::pi16(w, id, o::NODE) as i32;
    let i = nearest(&pts, target, node);
    let a = (i + 5).rem_euclid(n);
    let b = if i - 5 < 0 { i - 5 + n } else { i - 5 };
    let hero = super::hero_pos(w);
    let (da, db) = (c::dist3(pts[a as usize], hero), c::dist3(pts[b as usize], hero));
    let way_now = c::pu8(w, id, o::WAY) as i8 as i32;
    let (dir, mut k) = if db < da || (db <= da && way_now == 1) { (1, a) } else { (-1, b) };
    let mut count = 0;
    while c::dist3(target, pts[k.rem_euclid(n) as usize]) < 30.0 && count < n {
        k = (k + dir).rem_euclid(n);
        count += 1;
    }
    let cur = node;
    let (fwd, back) = if cur < k { (k - cur, (n - k) + cur) } else { (cur - k, (n - cur) + k) };
    let mut go = if cur < k { -1 } else { 1 };
    if fwd < back { go = -go; }
    let d = c::dist3(c::pos(w, id), pts[k.rem_euclid(n) as usize]);
    let scale = c::SPEED;
    if go != way_now {
        if c::dec_timer_pvar_s16(w, id, o::TURN_T) != 0 {
            c::set_pi16(w, id, o::TURN_T, 10);
            let sp = c::pf(w, id, o::SPEED) * (scale * -0.040_000_02 + 1.0);
            c::set_pf(w, id, o::SPEED, sp);
            let near = c::dist2(c::pos(w, id), hero) < 40.0;
            if sp < scale * 0.1 || w.m(id).visible == 0 || near {
                c::set_pu8(w, id, o::WAY, go as i8 as u8);
                let (nd, ls) = (c::pi16(w, id, o::NODE), c::pi16(w, id, o::LAST));
                c::set_pi16(w, id, o::NODE, ls);
                c::set_pi16(w, id, o::LAST, nd);
            }
        }
    } else {
        let sp = c::pf(w, id, o::SPEED);
        let sp = (sp + (d / 60.0 - sp) * scale * 0.07).min(DT * 100.0);
        c::set_pf(w, id, o::SPEED, sp);
    }
    advance(w, id, p)
}

/// The attack's shot (module doc).
fn shoot(w: &mut World, id: MobyId, ship_pos: c::V) {
    let Some(v) = w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len()) else { return };
    let sv = c::pv4(w, v, 0x10);
    let up = [0.0, 0.0, 1.0, 0.0];
    let cross = |a: c::V, b: c::V| -> c::V { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0], 0.0] };
    let side = cross(sv, up);
    let vert = cross(side, sv);
    let (k, ang) = (c::pf(w, id, o::SPREAD), c::pf(w, id, o::SPREAD_A));
    let vert = c::set_len3(vert, k * 5.0 * ang.sin());
    let side = c::set_len3(side, k * 5.0 * ang.cos());
    let mut p = c::add(c::add(ship_pos, vert), side);
    fx::jitter(w, f32::from_bits(0x3f19_999a), &mut p);
    let me = c::pos(w, id);
    let d = c::dist2(me, super::hero_pos(w));
    let speed = c::pf(w, v, 0x64);
    let lead = c::set_len3(sv, speed * (d / (speed + DT * 50.0)));
    let dir = c::sub(c::add(p, lead), me);
    let muzzle = c::add(c::set_len3(dir, f32::from_bits(0x4026_6666)), me);
    super::fighter_shot::spawn(w, 121.0, DT * 50.0, 5.0, id, dir, muzzle);
    w.play_sound(0, 0, id);
}

/// Level13 `0x2c4428` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    let p = path(w, id);
    match w.m(id).state {
        0 => {
            let m = w.m(id);
            if m.update_dist == 0 && m.draw_dist == 0 {
                w.delete_moby(id);
                return;
            }
            let Some(pp) = p else {
                w.delete_moby(id);
                return;
            };
            let pts = pts(w, pp);
            c::set_pi16(w, id, o::NODE, 1);
            c::set_pi16(w, id, o::LAST, pts.len() as i16 - 1);
            let q = pts.get(1).copied().unwrap_or([0.0; 4]);
            c::set_pv4(w, id, o::POINT, q);
            w.mm(id).position = q;
            let t = w.ticks(0x14);
            c::set_pi32(w, id, o::UNSEEN, t + 5);
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.draw_dist = 0xff;
            m.state = 1;
            m.cmd = 0;
            m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN;
            m.has_collision = false;
            m.visible = 0;
            c::set_pu8(w, id, o::WAY, 1);
        }
        1 => {
            if leader(w, id).is_some_and(|l| w.m(l).cmd != 4) {
                bounds(w, id);
                return;
            }
            let Some(pp) = p else { return };
            let f = w.camera_rows[0];
            let back = c::set_len3([f[0], f[1], f[2], 0.0], 40.0);
            let cam = w.camera.map(|x| f32::from_bits(x.0));
            let at = c::sub(cam, back);
            let node = crate::path::nearest_at_distance(&w.svc.splines[pp], 0.0, at);
            c::set_pi16(w, id, o::NODE, node as i16);
            c::set_pi16(w, id, o::LAST, node as i16);
            let q = pts(w, pp).get(node as usize).copied().unwrap_or([0.0; 4]);
            c::set_pv4(w, id, o::POINT, q);
            let coll = super::class_collision(w, w.m(id).o_class);
            let m = w.mm(id);
            m.position = q;
            m.visible = 1;
            m.has_collision = coll;
            m.mode = (m.mode & !mode::HIDDEN) | mode::TARGETABLE;
            m.state = 2;
        }
        2 => {
            hits(w, id);
            let ship = ship(w);
            let ship_pos = ship.map_or_else(|| super::hero_pos(w), |s| w.m(s).position);
            let mut seen = false;
            if let Some(s) = ship {
                let d = c::dist3(c::pos(w, id), ship_pos);
                if w.hero.state == 0x32 && matches!(w.m(s).o_class, SHIP | SHIP2) && d < 110.0 { seen = true; }
            }
            if w.m(id).visible == 0 {
                let n = c::pi32(w, id, o::UNSEEN) + 1;
                c::set_pi32(w, id, o::UNSEEN, n);
            } else if seen {
                c::set_pi32(w, id, o::UNSEEN, 0);
            }
            let lead = leader(w, id);
            if lead.is_none_or(|l| w.m(l).state == 1) {
                hide(w, id);
                return;
            }
            let lead = lead.unwrap();
            let hum = c::pi32(w, id, o::HUM);
            if hum == -1 || !w.sound_alive(hum, id) {
                let v = w.play_sound(1, 4, id);
                c::set_pi32(w, id, o::HUM, v);
            }
            fly(w, id);
            let cmd = w.m(id).cmd;
            match cmd {
                1 => {
                    let n = c::pu8(w, lead, LEADER_ATTACKING).wrapping_add(1);
                    c::set_pu8(w, lead, LEADER_ATTACKING, n);
                    if 3 < n {
                        bounds(w, id);
                        return;
                    }
                    let unseen = w.m(id).visible == 0;
                    if unseen {
                        let u = c::pi32(w, id, o::UNSEEN) + 1;
                        c::set_pi32(w, id, o::UNSEEN, u);
                    }
                    if unseen || !seen || w.ticks(0x14) <= c::pi32(w, id, o::UNSEEN) {
                        let t = w.ticks(0x1e);
                        c::set_pi32(w, id, o::ATTACK_T, t);
                        w.mm(id).cmd = 2;
                    }
                    if c::dec_timer_pvar_i32(w, id, o::ATTACK_T) != 0 {
                        let t = w.ticks(0x78);
                        c::set_pi32(w, id, o::ATTACK_T, t);
                        w.mm(id).cmd = 2;
                    }
                    let (l0, l1) = fx::frame_load(w);
                    if c::dec_timer_pvar_i32(w, id, o::SHOT_T) != 0 && l0 < 0.9 {
                        if 0.9 <= l1 {
                            bounds(w, id);
                            return;
                        }
                        let t = w.ticks(6) + ((l0 + l1) * 1.3) as i32;
                        c::set_pi32(w, id, o::SHOT_T, t);
                        if ship.is_some() { shoot(w, id, ship_pos); }
                        let scale = c::SPEED;
                        let r = w.rng.randf(f32::from_bits(0x3f6b_851f), f32::from_bits(0x3f70_a3d7));
                        let k = c::pf(w, id, o::SPREAD) * ((r - 1.0) * scale + 1.0);
                        c::set_pf(w, id, o::SPREAD, k);
                        let a = c::pf(w, id, o::SPREAD_A);
                        let s = w.rng.randf(20.0, 40.0) * 0.017_453_292 * DT;
                        c::set_pf(w, id, o::SPREAD_A, if FRAC_PI_2 < a { a - s } else { a + s });
                    }
                }
                0 => {
                    let ready = ship.is_some_and(|s| w.m(s).state == 4) && w.m(id).visible != 0 && seen && c::pu8(w, lead, LEADER_ATTACKING) <= 2;
                    if ready {
                        let t = w.ticks(600);
                        c::set_pi32(w, id, o::ATTACK_T, t);
                        c::set_pf(w, id, o::SPREAD, 1.0);
                        c::set_pi32(w, id, o::UNSEEN, 0);
                        w.mm(id).cmd = 1;
                    }
                }
                2 if c::dec_timer_pvar_i32(w, id, o::ATTACK_T) != 0 => w.mm(id).cmd = 0,
                _ => {}
            }
        }
        3 => {
            let hum = c::pi32(w, id, o::HUM);
            if hum != -1 {
                w.release_sound(hum, id);
                c::set_pi32(w, id, o::HUM, -1);
            }
            let cam = w.camera.map(|x| f32::from_bits(x.0));
            let to_cam = c::sub(cam, c::pos(w, id));
            let me = c::pos(w, id);
            fx::beam_explosion(w, &BOOM, Some(id), me);
            let back = c::set_len3(to_cam, 4.0);
            let at = c::sub(me, back);
            match w.rng.rand() % 10 {
                0 => { super::ship_pickup_float::spawn(w, at, super::ship_pickup_float::MISSILES); }
                1..=6 => { super::ship_pickup_float::spawn(w, at, super::ship_pickup_float::HEALTH); }
                _ => {}
            }
            if let Some(l) = leader(w, id) {
                let n = c::pu8(w, l, LEADER_OUT);
                if n != 0 { c::set_pu8(w, l, LEADER_OUT, n - 1); }
            }
            hide(w, id);
        }
        4 => {
            if let Some(l) = leader(w, id) {
                if w.m(l).cmd != 4 {
                    let hp = c::pi16(w, id, o::D + 4) as f32;
                    c::set_pf(w, id, o::D, hp);
                    w.mm(id).state = 0;
                }
            }
        }
        _ => {}
    }
    bounds(w, id);
}

/// The position kept inside 20..1003.
fn bounds(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    for k in 0..3 { m.position[k] = m.position[k].clamp(20.0, 1003.0); }
}
