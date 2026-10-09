//! **Umbris' floating mines**: the lone mines 1110 (level07 `0x319040`, 12 placed; census U272), the chain mines 1112
//! (`0x3196e0`, its death `0x319698`; 31 placed; U273) and their invisible chain leader 871 (`0x30b000`, its revive
//! `0x30af40`; 4 placed; U260). The names are descriptive [L]. A lone mine spins in place until Ratchet (or whoever
//! last hit it) comes within 8 (or 20 / 3 in height), then bobs after him, accelerating to 6·dt a tick, and blows up
//! on the first moby it touches (a damage-1 contact) or after ten seconds; 64 from home it just vanishes. Chain
//! mines ride their leader's path in a line (the leader moves each one's path point along at its speed), turning to
//! face the way; one whose target comes within the leader's range and arc (+0x8c, ±+0x88°, beyond 2 of the leader)
//! leaves the line and chases him (pitching toward him, 30·dt² acceleration to 10·dt), and blows up the same way; a
//! dead chain mine is revived on the line after the leader's respawn time (+0x84, −1 never). Shot mines blow up. Read
//! from the level07 decomp and disassembly (the sphere radii and the explosions' stack arguments). Native `f32`.
//!
//! **Pvars** (mines): +0x00 the header (+0x20 the damage record, its +0x18 the last attacker; +0x60 the 1112's
//! velocity), 1110: +0x60 home, +0x70 the chase speed, +0x74 a group told 2 when it wakes, +0x78 the life timer,
//! +0x7c the bob timer; 1112: +0x70 home, +0x80 its path point, +0x90 / +0x94 its segment and the next, +0x98 the
//! life (respawn) timer, +0x9c the chase speed, +0xa0 the leader, +0xa4 / +0xa8 the yaw / pitch springs, +0xac the
//! bob timer. Leader 871: +0x20 the damage record, +0x60 the flash, +0x70 the path, +0x74 the group, +0x78 the
//! count, +0x7c the speed, +0x80 the spacing, +0x84 the respawn ticks, +0x88 the arc (°), +0x8c the range, +0x90 the
//! start segment, +0x94 byte shootable, +0x95 byte shown.
//!
//! | address | what | port |
//! |---|---|---|
//! | mines, top | the target (`0x2886d8(8 / the leader's +0x8c)` = L01 `0x274b78`); drawn within 27 of the camera: the shadow probe, +0x7f 0x15; the hit (mask 0x230000) through the resolver (`0x282ed8`): out ≥ 2 and not from 0x456 / 0x365 / 0x367 (1112: 0x363 / 0x458 / 0x367) → the explosion, `SetDeathBits`, deleted | [`lone_update`], [`chain_update`] (`target::acquire`, `damage::resolve`) |
//! | 1110 state 0 | targetable, home, the class's collision → 1 | [`lone_update`] |
//! | 1110 state 1 | life `ticks(600)`; yaw += 2π·dt; the last attacker within 20 / 3, or the target within 8 → speed 0, life `ticks(600)`, bob up `ticks(7)`, the group +0x74 told 2, → 2 | [`lone_update`] |
//! | 1110 state 2 / 3 | sequence 1, then 2 once 1 wrapped; speed += 30·dt² (≤ 6·dt); the bob (z ±0.1 by `ticks(7)`); toward the target at the speed; beyond 64 from home → deleted; a 0.75 sphere 0.75 up (`coll_sphere_mobys` 0x10, damage 1, flags 0x10001 along the move) touching a moby not 0x363 / 0x456 / 0x458 / 0x365 / 0x367 → the explosion; else pushed out of the world (a 0.55 sphere 0.55 up); the life out → the explosion | [`lone_update`] |
//! | 1112 state 0 | with a leader: targetable, velocity 0, home, pitch 0, facing its point, → 1 | [`chain_update`] |
//! | 1112 state 1 | velocity = point − position, the yaw spring (2π·dt², π·dt, 2π·dt) to its point, onto it; the attacker near, or the target in range, beyond 2 of the leader and within the arc → sequence 1, speed = leader +0x7c·dt, life `ticks(600)`, bob, → 2 | [`chain_update`] |
//! | 1112 state 2 | the bob; the yaw (π·dt², π·dt, 2π·dt) and pitch springs toward the target; velocity += (target − position, flat while bobbing) at 30·dt², at most 10·dt; beyond 64 from home → dies; the contact sphere → the explosion and dies; the world push; life 0 → dies | [`chain_update`] |
//! | `0x319698` | life = `ticks(leader +0x84)`, deleted | [`chain_die`] |
//! | 871 | shootable (+0x94): the hit → health 0 on reactions 1 / 2; out > 1: health ≤ damage → the group told 3, the death explosion, deleted; else health −= damage and the red flash | [`leader_update`] (`flash`) |
//! | 871 state 0 | the path's segment lengths; the members (group +0x74, any state) placed from point 0 on, `SplineProject` by +0x80 from the point nearest it (`0x29e610`); +0x78 cut to the members found; not shootable → no collision, untargetable; not shown → hidden; → 1 | [`leader_update`] |
//! | 871 state 1 | each member's point moved along the path at +0x7c·dt; a dead member (respawn ≠ −1) kept on its point, its life timer down, revived (`0x30af40`) when out at the leader's start segment; the blob shadow 1.5 | [`leader_update`], [`revive`] (`shadows::blob`) |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::{self as c, damage, flash, fx, target, turn, DT, DT2, V};
use crate::moby_update::scheduler::{self, GroupWalk};
use crate::moby_update::services::{pf, pv as v4, HitTemplate, World};
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 7;
pub const LONE_FN: u32 = 0x31_9040;
pub const LONE_CLASSES: [i16; 1] = [1110];
pub const CHAIN_FN: u32 = 0x31_96e0;
pub const CHAIN_CLASSES: [i16; 1] = [1112];
pub const LEADER_FN: u32 = 0x30_b000;
pub const LEADER_CLASSES: [i16; 1] = [871];

const D: usize = 0x20;
const TAU: f32 = std::f32::consts::TAU;
const PI: f32 = std::f32::consts::PI;
/// The mines' explosion (`SpawnBeamExplosion(0, 0, 2, 1, 4, 1, 7, m, v, NULL, 3, 3, 5, sound 2, no shake, 1 debris)`).
const MINE_BEAM: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 4.0, scale: 1.0, light: 7.0, streaks: 3, sparks: 3, puffs: 5, debris: 1, sound: 2, shake: false };
/// The leader's (`SpawnBeamExplosion(0, 0, 4, 2, 9, 1, 15, m, 0, NULL, 10, 3, 16, no sound, shake)`).
const LEADER_BEAM: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 10, sparks: 3, puffs: 0x10, debris: 1, sound: -1, shake: true };

fn moby_at(w: &World, i: i32) -> Option<MobyId> { usize::try_from(i).ok().filter(|&m| m < w.table.mobys.len()) }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn class_coll(w: &World, id: MobyId) -> bool { w.classes.info(w.m(id).o_class).is_some_and(|i| i.has_collision) }
fn seq(w: &World, id: MobyId) -> (u8, u8) { (w.m(id).anim.seq_a, w.m(id).anim.seq_b) }
fn blend(w: &mut World, id: MobyId, s: u8, t: i32) { c::blend_to(w, id, s, 0, t); }
fn deleted(w: &World, m: MobyId) -> bool { matches!(w.m(m).state, 0xfd | 0xfe) }

/// Drawn within 27 of the camera: the shadow probe, +0x7f 0x15.
fn shadow(w: &mut World, id: MobyId) {
    let cam = w.camera_point();
    if w.m(id).visible != 0 && c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 27.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0x15;
    }
}

/// The hit through the resolver; `Some(class)` of the attacker when the result's out is ≥ 2.
fn hit(w: &mut World, id: MobyId) -> Option<Option<i16>> {
    let h = w.get_hit(id, 0x23_0000, false);
    let r = damage::resolve(w, id, h, D, 0, 4);
    (2 <= r.out5).then(|| h.and_then(|h| h.attacker).filter(|&a| a < w.table.mobys.len()).map(|a| w.m(a).o_class))
}

/// The last attacker (the damage record's +0x18) within 20 (xy) and 3 in height.
fn attacker_near(w: &World, id: MobyId) -> bool {
    let raw = c::pi32(w, id, D + 0x18);
    let Some(a) = moby_at(w, raw).filter(|_| raw != 0) else { return false };
    let (p, q) = (c::pos(w, id), w.m(a).position);
    c::dist2(p, q) < 20.0 && (p[2] - q[2]).abs() < 3.0
}

/// The bob while chasing: +0xbc 1 rises 0.1 a tick, 2 sinks, each `ticks(7)`.
fn bob(w: &mut World, id: MobyId, timer: usize) {
    match w.m(id).cmd {
        1 => {
            w.mm(id).position[2] += c::SPEED * 0.1;
            if c::dec_timer_pvar_i32(w, id, timer) != 0 {
                let t = w.ticks(7);
                c::set_pi32(w, id, timer, t);
                w.mm(id).cmd = 2;
            }
        }
        2 => {
            w.mm(id).position[2] -= c::SPEED * 0.1;
            if c::dec_timer_pvar_i32(w, id, timer) != 0 { w.mm(id).cmd = 0; }
        }
        _ => {}
    }
}

/// The contact: a 0.75 sphere 0.75 up with a damage-1 template along `dir`; true when it touched a moby that is not
/// one of `spared`.
fn contact(w: &mut World, id: MobyId, dir: V, spared: &[i16]) -> bool {
    let tmpl = HitTemplate { dir: v4(dir), attacker: Some(id), flags: 0x1_0001, damage: Pf::ONE, w20: 1, ..Default::default() };
    let p = c::pos(w, id);
    let hit = w.sphere_mobys_list(pf(0.75), v4([p[0], p[1], p[2] + 0.75, p[3]]), 0x10, Some(id), Some(&tmpl));
    hit.last().is_some_and(|&m| !spared.contains(&w.m(m).o_class))
}

/// Pushed out of the world by a 0.55 sphere 0.55 up.
fn push_out(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    let ctr = [p[0], p[1], p[2] + 0.55, p[3]];
    if let Some(o) = w.coll_sphere(v4(ctr), pf(0.55), 0, Some(id)) {
        if let Some(pc) = o.pushed_centre {
            let d = [pc[0] - ctr[0], pc[1] - ctr[1], pc[2] - ctr[2], 0.0];
            c::set_pos(w, id, c::add(p, d));
        }
    }
}

fn explode(w: &mut World, id: MobyId, v: V) {
    let _ = v;
    let p = c::pos(w, id);
    fx::beam_explosion(w, &MINE_BEAM, Some(id), p);
}

/// Level07 `0x319040` (module doc).
pub fn lone_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x80 { return; }
    let t = target::acquire(w, id, 8.0);
    shadow(w, id);
    if let Some(by) = hit(w, id) {
        if by.is_some_and(|c| c != 0x456 && c != 0x365 && c != 0x367) {
            explode(w, id, [0.0; 4]);
            crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
            w.delete_moby(id);
            return;
        }
    }
    let state_now = w.m(id).state;
    match state_now {
        0 => {
            w.mm(id).mode |= mode::TARGETABLE;
            let p = c::pos(w, id);
            c::set_pv4(w, id, 0x60, p);
            w.mm(id).state = 1;
            let coll = class_coll(w, id);
            w.mm(id).has_collision = coll;
        }
        1 => {
            let life = w.ticks(600);
            c::set_pi32(w, id, 0x78, life);
            w.mm(id).rotation[2] = c::add_rot(w.m(id).rotation[2], DT * TAU);
            let wake = attacker_near(w, id) || (t.moby.is_some() && c::dist3(c::pos(w, id), t.pos) < 8.0);
            if wake {
                c::set_pf(w, id, 0x70, 0.0);
                let life = w.ticks(600);
                c::set_pi32(w, id, 0x78, life);
                w.mm(id).cmd = 1;
                let b = w.ticks(7);
                c::set_pi32(w, id, 0x7c, b);
                let g = c::pi32(w, id, 0x74);
                if g != -1 { scheduler::group_state(w, g as i8, 2); }
                w.mm(id).state = 2;
            }
        }
        2 | 3 => {
            if state_now == 2 {
                blend(w, id, 1, 10);
                w.mm(id).state = 3;
            }
            let (a, b) = seq(w, id);
            if w.m(id).anim.flags & 2 != 0 && a == 1 && b != 2 { blend(w, id, 2, 10); }
            let sp = (c::pf(w, id, 0x70) + DT2 * 30.0).min(DT * 6.0);
            c::set_pf(w, id, 0x70, sp);
            bob(w, id, 0x7c);
            let p = c::pos(w, id);
            let dir = c::set_len3(c::sub(t.pos, p), sp);
            let p = c::add(p, dir);
            c::set_pos(w, id, p);
            if 64.0 < c::dist3(p, c::pv4(w, id, 0x60)) {
                w.delete_moby(id);
                return;
            }
            if contact(w, id, dir, &[0x363, 0x456, 0x458, 0x365, 0x367]) {
                explode(w, id, dir);
                w.delete_moby(id);
                return;
            }
            push_out(w, id);
            if c::dec_timer_pvar_i32(w, id, 0x78) != 0 {
                explode(w, id, dir);
                w.delete_moby(id);
                return;
            }
        }
        _ => {}
    }
    crate::shadows::blob(w, f32::from_bits(0x3f33_3333), id);
}

/// `0x319698(m, leader)`: the life timer = `ticks(leader +0x84)`, deleted.
pub fn chain_die(w: &mut World, id: MobyId, leader: Option<MobyId>) {
    if let Some(l) = leader {
        let t = w.ticks(c::pi32(w, l, 0x84));
        c::set_pi32(w, id, 0x98, t);
    }
    w.delete_moby(id);
}

/// Level07 `0x3196e0` (module doc).
pub fn chain_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0xb0 { return; }
    let Some(leader) = link(w, id, 0xa0) else { return };
    let range = c::pf(w, leader, 0x8c);
    let t = target::acquire(w, id, range);
    shadow(w, id);
    if let Some(by) = hit(w, id) {
        if by.is_some_and(|c| c != 0x363 && c != 0x458 && c != 0x367) {
            explode(w, id, c::pv4(w, id, 0x60));
            crate::moby_update::classes::crate_::set_death_bits(w, id, 0, -1);
            chain_die(w, id, Some(leader));
        }
    }
    let state_now = w.m(id).state;
    match state_now {
        0 => {
            w.mm(id).mode |= mode::TARGETABLE;
            c::set_pv4(w, id, 0x60, [0.0; 4]);
            let p = c::pos(w, id);
            c::set_pv4(w, id, 0x70, p);
            w.mm(id).rotation[0] = 0.0;
            let q = c::pv4(w, id, 0x80);
            w.mm(id).rotation[2] = c::atan(q[0] - p[0], q[1] - p[1]);
            c::set_pu8(w, id, D + 9, 0);
            w.mm(id).state = 1;
            w.mm(id).cmd = 0;
        }
        1 => {
            let p = c::pos(w, id);
            let q = c::pv4(w, id, 0x80);
            c::set_pv4(w, id, 0x60, c::sub(q, p));
            let h = c::atan(q[0] - p[0], q[1] - p[1]);
            turn::spring_turn2_pvar(w, id, h, DT2 * TAU, DT * PI, DT * TAU, 0xa4);
            c::set_pos(w, id, q);
            let lp = w.m(leader).position;
            let chase = attacker_near(w, id)
                || (t.moby.is_some()
                    && c::dist3(q, t.pos) < range
                    && 2.0 < c::dist2(q, lp)
                    && c::diff_rots(w.m(id).rotation[2], c::atan(t.pos[0] - q[0], t.pos[1] - q[1])) < c::pf(w, leader, 0x88) * 0.017_453_292);
            if chase {
                blend(w, id, 1, 10);
                c::set_pf(w, id, 0x9c, c::pf(w, leader, 0x7c) * DT);
                let life = w.ticks(600);
                c::set_pi32(w, id, 0x98, life);
                w.mm(id).cmd = 1;
                let b = w.ticks(7);
                c::set_pi32(w, id, 0xac, b);
                w.mm(id).state = 2;
            }
        }
        2 => {
            let (a, b) = seq(w, id);
            if w.m(id).anim.flags & 2 != 0 && a == 1 && b != 2 { blend(w, id, 2, 10); }
            bob(w, id, 0xac);
            let p = c::pos(w, id);
            turn::spring_turn2_pvar(w, id, c::atan(t.pos[0] - p[0], t.pos[1] - p[1]), DT2 * PI, DT * PI, DT * TAU, 0xa4);
            let up = c::atan(c::dist2(p, t.pos), t.pos[2] - p[2]);
            let mut pv = c::pf(w, id, 0xa8);
            let pitch = turn::spring_turn(w.m(id).rotation[0], up, DT2 * PI, DT * PI, DT * TAU, &mut pv);
            c::set_pf(w, id, 0xa8, pv);
            w.mm(id).rotation[0] = pitch;
            let mut d = c::sub(t.pos, p);
            if w.m(id).cmd != 0 { d[3] = 0.0; d[2] = 0.0; }
            let v = c::clamp_len3(c::add(c::pv4(w, id, 0x60), c::set_len3(d, DT2 * 30.0)), DT * 10.0);
            c::set_pv4(w, id, 0x60, v);
            let p = c::add(p, v);
            c::set_pos(w, id, p);
            // The game goes on after each death this tick (the moby is only marked deleted).
            if 64.0 < c::dist3(p, c::pv4(w, id, 0x70)) { chain_die(w, id, Some(leader)); }
            if contact(w, id, v, &[0x363, 0x456, 0x458, 0x365, 0x367]) {
                explode(w, id, v);
                chain_die(w, id, Some(leader));
            }
            push_out(w, id);
            if c::pi32(w, id, 0x98) == 0 { chain_die(w, id, Some(leader)); }
        }
        3 => {
            explode(w, id, c::pv4(w, id, 0x60));
            chain_die(w, id, Some(leader));
        }
        _ => {}
    }
    crate::shadows::blob(w, f32::from_bits(0x3f33_3333), id);
}

/// `0x30af40(m)`: a dead member back: state 0, +0xbc 0, the class's mode, draw / update distance 0xff, the class
/// scale, drawn, the light words (`UpdateMobyGrids(m, 0x80807f7f)`), +0x71 / +0x72 0xff, occlusion 0x7f80, no hit,
/// the class's collision, Ratchet's light (`0x285bd8` = L01 `0x272078`), `MobyBuildMatrix`, the damage record's
/// health from its s16 +0x04.
pub fn revive(w: &mut World, m: MobyId) {
    let info = w.classes.info(w.m(m).o_class).unwrap_or_default();
    {
        let mm = w.mm(m);
        mm.state = 0;
        mm.cmd = 0;
        mm.mode = info.mode_bits;
        mm.draw_dist = 0xff;
        mm.scale = info.scale;
        mm.update_dist = 0xff;
        mm.visible = 1;
        mm.b71 = 0xff;
        mm.occlusion = 0x7f80;
        mm.b72 = 0xff;
        mm.hit_slot = 0xff;
        mm.has_collision = info.has_collision;
        mm.delete_tick = 0;
    }
    crate::moby_update::classes::units::take_hero_light(w, m);
    w.build_matrix(m);
    if let Some(d) = crate::moby_update::triggers::pvar_record(w.m(m)) {
        let hp = c::pi16(w, m, d + 4) as f32;
        c::set_pf(w, m, d, hp);
    }
}

/// Level07 `0x30b000` (module doc).
pub fn leader_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x98 { return; }
    if c::pu8(w, id, 0x94) != 0 {
        let h = w.get_hit(id, 0x23_0000, false);
        let r = damage::resolve(w, id, h, D, 0, 4);
        if r.reaction == 1 || r.reaction == 2 { c::set_pf(w, id, D, 0.0); }
        if 1 < r.out5 {
            let dmg = r.damage;
            if c::pf(w, id, D) <= dmg {
                c::set_pf(w, id, D, 0.0);
                let g = c::pi32(w, id, 0x74);
                scheduler::group_state(w, g as i8, 3);
                let p = c::pos(w, id);
                fx::beam_explosion(w, &LEADER_BEAM, Some(id), p);
                w.delete_moby(id);
                return;
            }
            c::set_pf(w, id, D, c::pf(w, id, D) - dmg);
            c::set_pu8(w, id, 0x67, 0xfa);
            let t = w.ticks(60);
            c::set_pi16(w, id, D + 6, t as i16);
            flash::start(w, id, 0x60);
        }
        w.mm(id).hit_slot = 0xff;
    }
    flash::update(w, id, 0x60);
    let Some(path) = usize::try_from(c::pi32(w, id, 0x70)).ok().filter(|&p| p < w.svc.splines.len()) else { return };
    let g = c::pi32(w, id, 0x74);
    let members = scheduler::group_walk(w, g, GroupWalk::Any);
    match w.m(id).state {
        0 => {
            let pts = &mut w.svc.splines[path];
            let n = pts.len();
            for i in 0..n {
                let (p, q) = (pts[i].map(f32::from_bits), pts[(i + 1) % n].map(f32::from_bits));
                pts[i][3] = c::dist3(p, q).to_bits();
            }
            if members.is_empty() { return; }
            crate::shadows::blob(w, 1.5, id);
            let pts: Vec<[f32; 4]> = w.svc.splines[path].iter().map(|p| p.map(f32::from_bits)).collect();
            let me = c::pos(w, id);
            let start = nearest_point(&pts, me);
            c::set_pi32(w, id, 0x90, start as i32);
            let mut cur = crate::spline::Cursor { seg: start as i32, t: 0.0 };
            let mut at = pts[0];
            let count = c::pi32(w, id, 0x78);
            let spacing = c::pf(w, id, 0x80);
            let mut placed = 0;
            for &m in members.iter().take(count.max(0) as usize) {
                if w.m(m).pvars.len() < 0xb0 { w.mm(m).pvars.resize(0xb0, 0); }
                c::set_pi32(w, m, 0xa0, id as i32 + 1);
                c::set_pos(w, m, at);
                c::set_pv4(w, m, 0x80, at);
                c::set_pv4(w, m, 0x70, at);
                c::set_pi32(w, m, 0x90, cur.seg);
                c::set_pi32(w, m, 0x94, cur.seg + 1);
                placed += 1;
                let (p, _) = crate::spline::advance(&pts, true, spacing, &mut cur);
                at = [p[0], p[1], p[2], at[3]];
            }
            if placed < count { c::set_pi32(w, id, 0x78, placed); }
            if c::pu8(w, id, 0x94) == 0 {
                let m = w.mm(id);
                m.has_collision = false;
                m.mode &= !mode::TARGETABLE;
            }
            if c::pu8(w, id, 0x95) == 0 { w.mm(id).mode |= mode::HIDDEN; }
            w.mm(id).state = 1;
        }
        1 => {
            let pts: Vec<[f32; 4]> = w.svc.splines[path].iter().map(|p| p.map(f32::from_bits)).collect();
            if pts.is_empty() { return; }
            let speed = c::pf(w, id, 0x7c) * DT;
            let count = c::pi32(w, id, 0x78);
            let respawn = c::pi32(w, id, 0x84);
            let start = c::pi32(w, id, 0x90);
            for &m in members.iter().take(count.max(0) as usize) {
                if w.m(m).pvars.len() < 0xb0 { continue; }
                let seg = c::pi32(w, m, 0x90).clamp(0, pts.len() as i32 - 1);
                let p0 = c::pv4(w, m, 0x80);
                let mut cur = crate::spline::Cursor { seg, t: c::dist3(p0, pts[seg as usize]) };
                let (p, _) = crate::spline::advance(&pts, true, speed, &mut cur);
                c::set_pv4(w, m, 0x80, [p[0], p[1], p[2], p0[3]]);
                c::set_pi32(w, m, 0x90, cur.seg);
                c::set_pi32(w, m, 0x94, cur.seg + 1);
                if deleted(w, m) && respawn != -1 {
                    let q = c::pv4(w, m, 0x80);
                    let mp = c::pos(w, m);
                    c::set_pv4(w, m, 0x60, c::sub(q, mp));
                    c::set_pos(w, m, q);
                    if c::dec_timer_pvar_i32(w, m, 0x98) != 0 && c::pi32(w, m, 0x90) == start { revive(w, m); }
                }
            }
            crate::shadows::blob(w, 1.5, id);
        }
        _ => {}
    }
}

/// `0x29e610(0, p, path)`: the path point nearest `p` (3-D; the first of equals).
fn nearest_point(pts: &[[f32; 4]], p: V) -> usize {
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
