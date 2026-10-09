//! **The Fleet's crew, class 1382** (level17 `0x2eeb68`; census U612, 40 placed). Blarg crewmen who idle at their
//! posts; seeing Ratchet in front they alert, look about, salute the Hologuise (hero mode 3) or point him out, then
//! give chase inside their area (+0x1d0) and punch; one hit knocks them flying and breaks them. Read from the level17
//! decomp; native `f32`.
//!
//! * **Every tick**: game mode — culled by the camera cuboids +0x2b0..+0x2bc (but in the death flight 0xf): not drawn.
//!   The senses (`0x2efa48`): the idle timer +0x1d4; a hit (mask 0x330000, health +0x20 1): the knock flight (19·dt
//!   out, 15.7·dt up, 42·dt² down; toward home when the hitter is class 99), seq 11, → 0xf, red 0xf0; the target
//!   (`0x274df8`, 128) and sight +0x1e4 (a clear line to Ratchet's body point within 64: 1, in front within 60°: 2).
//!   The head's look-at at Ratchet (±60°, ±30°; list 0), the big-head cheat (2.1), the shadow (within 29), the glow.
//! * **States**: 0 set up (health 1, home, walk speed `randf(7.2, 8.7)·dt`, a random first frame) → 1; 1 idle: seen,
//!   1 in 31 → 2 (seq 1); 2 alert: seen in front → 1 in 3 seq 2 again, else the salute (6, seq 7) to the Hologuise or
//!   7 (seq 8); else → 5 (seq 3, timers `ticks(120)`); 3 settles back to 1; 5 looks about turning to the target (seq
//!   4 / 5 turning, 3 still; speed 1.7), seen in front again → 2 / 6 / 7, else (its timers) → 3; 6 / 7: in front and
//!   outside the area (with no idle time) it keeps pointing, else 1 in 20 → 8 (seq 1); 8 turns to the target → 9 /
//!   0xb (seq 9); 9 chases (`0x2efd48` at its speed, anim 1.8) inside its area: within 1.8 → 10 (seq 10 from frame
//!   5); too far from home (32) or 3 below → 0xb; 10 punches at key 17.5 (within 15°, 1 in height, 2.75: a hit of
//!   1 at the target) → 0xb; 0xb walks home (5·dt, anim 1.35): there → 5; on the way back with the target close
//!   inside its area → 10; 0xc carried by the Suck Cannon (`0x305260`): landed → 0; 4 / 0xf the knock flights:
//!   landed → 5 / broken (the death explosion, pieces 0x6ce / 0x6cf, the shards 0x77d), out of the world → gone.
//! * **The glow** (+0x90): red pulses every `ticks(50)` when chasing, `ticks(90)` alerted, else every `ticks(170)`
//!   green; three glows (0.2, pull 0.08, alpha 0x30) on joint lists 1..3 (`0x2f0448`).
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2eeb68` | 1382 | [`update`] |
//! | `0x2efa48` / `0x2efd48` | the senses, the walk | [`senses`], [`walk`] |
//! | `0x2f00a0` / `0x2f0210` / `0x2f0448` | the look, the glow, the eyes | [`look`], [`glow`], [`glow_quads`] |
//!
//! [L] The alerted timer +0x1e0 is set but never run down here (the game's own), so a crewman that saw Ratchet keeps
//! looking about.

use super::GlowQuad;
use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::classes::breakables::burst_pieces;
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, damage, flash, fx, knock, target, turn, walker, DT, DT2, SPEED};
use crate::moby_update::services::{pv, pvar as p, Services, World};
use crate::moby_update::{manip, story};
use crate::ps2v::Pf;
use std::f32::consts::{FRAC_PI_3, FRAC_PI_6};

pub const REFERENCE_LEVEL: u32 = 17;
pub const UPDATE_FN: u32 = 0x2e_eb68;
pub const DRAW_FN: u32 = 0x2f_0448;
pub const CLASSES: [i16; 1] = [1382];

mod o {
    pub const D: usize = 0x20;
    pub const IDLE: usize = 0x38;
    pub const F: usize = 0x110;
    pub const K: usize = 0x120;
    pub const TGT: usize = 0x180;
    pub const TMOBY: usize = 0x1c0;
    pub const KIND: usize = 0x1c4;
    pub const AREA: usize = 0x1d0;
    pub const IDLE_T: usize = 0x1d4;
    pub const SPEED: usize = 0x1d8;
    pub const YAW_V: usize = 0x1dc;
    pub const SEEN_T: usize = 0x1e0;
    pub const LOOK_T: usize = 0x1e2;
    pub const SEES: usize = 0x1e4;
    pub const PATH: usize = 0x1e8;
    pub const WALK: usize = 0x1ec;
    pub const HOME: usize = 0x1f0;
    pub const EYES: usize = 0x200;
    pub const LOOK: usize = 0x230;
    pub const CUB: usize = 0x2b0;
    pub const SIZE: usize = 0x2c0;
}

const DISGUISED: u8 = 3;

fn st(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_st(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn wrapped(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn seq(w: &mut World, id: MobyId, s: u8) {
    if w.m(id).anim.seq_b != s {
        let t = w.ticks(10);
        w.anim_blend(id, s, 0, t);
    }
}
fn tpos(w: &World, id: MobyId) -> c::V { c::pv4(w, id, o::TGT) }
fn tmoby(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o::TMOBY) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn sees(w: &World, id: MobyId) -> u8 { c::pu8(w, id, o::SEES) }
fn in_area(w: &World, id: MobyId) -> bool {
    usize::try_from(c::pi32(w, id, o::AREA)).ok().is_some_and(|a| c::region::point_in_polygon(w, a, super::hero_pos(w)))
}
fn frames0(w: &World, id: MobyId) -> i32 {
    w.classes.anim(w.m(id).o_class).and_then(|cl| cl.sequences.first().and_then(|s| s.as_ref())).map_or(1, |s| s.header.frame_count as i32)
}
/// Back to idle at a random frame of sequence 0 (`n` frames to choose from).
fn idle(w: &mut World, id: MobyId, n: i32) {
    set_st(w, id, 3);
    if w.m(id).anim.seq_b == 0 { return; }
    let f = w.rng.randi(n.max(1));
    let t = w.ticks(10);
    w.anim_blend(id, 0, f, t);
}
/// The salute or the pointing (6 / 7) after a 1-in-3 alert (seq 2).
fn react(w: &mut World, id: MobyId) {
    if w.rng.randi(3) == 0 {
        seq(w, id, 2);
    } else if w.body() == DISGUISED {
        set_st(w, id, 6);
        seq(w, id, 7);
    } else {
        set_st(w, id, 7);
        seq(w, id, 8);
    }
}
fn look_about(w: &mut World, id: MobyId) {
    set_st(w, id, 5);
    seq(w, id, 3);
    let t = w.ticks(0x78);
    c::set_pi16(w, id, o::LOOK_T, t as i16);
    c::set_pi16(w, id, o::SEEN_T, t as i16);
}

/// `0x2efd48(speed, m, point)`: turn toward `point` (4π·dt², 420°·dt), the speed eased to `speed` (13·dt²), the move
/// with collision (0.5, 0.5), sinking 0.5·dt; whether within 0.2 (xy).
pub fn walk(w: &mut World, id: MobyId, speed: f32, point: c::V) -> bool {
    let p = c::pos(w, id);
    let a = c::atan(point[0] - p[0], point[1] - p[1]);
    let (mut yaw, mut v) = (w.m(id).rotation[2], c::pf(w, id, o::YAW_V));
    turn::turn_toward(a, DT2 * 12.566_371, DT2 * 12.566_371, DT * 7.330_383, &mut yaw, &mut v);
    w.mm(id).rotation[2] = yaw;
    c::set_pf(w, id, o::YAW_V, v);
    let mut s = c::pf(w, id, o::SPEED);
    turn::approach(speed, DT2 * 13.0, &mut s);
    c::set_pf(w, id, o::SPEED, s);
    let mut mv = [yaw.cos() * s, yaw.sin() * s, -(DT * 0.5), 0.0];
    walker::move_collide(w, id, 0.5, 0.5, 0.0, &mut mv, 0);
    c::dist2(c::pos(w, id), point) < 0.2
}

/// `0x2efa48(m, culled)` (module doc).
pub fn senses(w: &mut World, id: MobyId, culled: bool) {
    if st(w, id) == 0 { return; }
    if c::pi32(w, id, o::IDLE) != 0 {
        let r = w.rng.randf(180.0, 240.0);
        let t = w.svc.timing.scale(Pf::f(r)).to_i32();
        c::set_pi32(w, id, o::IDLE, 0);
        c::set_pf(w, id, o::IDLE_T, t as f32);
    }
    let it = (c::pf(w, id, o::IDLE_T) - 1.0).max(0.0);
    c::set_pf(w, id, o::IDLE_T, it);
    let hit = w.get_hit(id, 0x33_0000, false);
    let r = damage::resolve(w, id, hit, o::D, 0, 4);
    if r.out5 != 1 && st(w, id) != 0xf {
        let kr = o::K;
        c::set_pu8(w, id, kr + 0x3d, 0);
        let hp = c::pf(w, id, o::D) - r.damage;
        c::set_pf(w, id, o::D, hp);
        c::set_pf(w, id, kr + knock::k::GRAVITY, DT2 * 42.0);
        c::set_pf(w, id, kr + knock::k::DRAG, DT2 * 24.0);
        c::set_pi32(w, id, kr + knock::k::FLAGS, 9);
        c::set_pf(w, id, kr + knock::k::UP, DT * 15.7);
        c::set_pf(w, id, kr + knock::k::SPEED, DT * 19.0);
        c::set_pf(w, id, kr + knock::k::KEY_APEX, -1.0);
        c::set_pf(w, id, kr + knock::k::KEY_LAND, -1.0);
        let dir = hit.map_or([0.0; 4], |h| h.dir.map(|x| x.to_f32()));
        let (mut sp, mut up) = (c::pf(w, id, kr + knock::k::SPEED), c::pf(w, id, kr + knock::k::UP));
        let mut a = knock::aim(dir, &mut sp, &mut up);
        c::set_pf(w, id, kr + knock::k::SPEED, sp);
        c::set_pf(w, id, kr + knock::k::UP, up);
        if hit.and_then(|h| h.attacker).filter(|&m| m < w.table.mobys.len()).is_some_and(|m| w.m(m).o_class == 99) {
            let (h, p) = (c::pv4(w, id, o::HOME), c::pos(w, id));
            a = c::atan(h[0] - p[0], h[1] - p[1]);
        }
        knock::start(w, id, kr, a, 0xb, 1, 0);
        set_st(w, id, 0xf);
        c::set_pu8(w, id, o::F + 7, 0xf0);
        flash::start(w, id, o::F);
    }
    w.mm(id).hit_slot = 0xff;
    if culled { return; }
    let t = target::acquire(w, id, 128.0);
    c::set_pv4(w, id, o::TGT, t.pos);
    c::set_pv4(w, id, o::TGT + 0x10, t.rot);
    c::set_pv4(w, id, o::TGT + 0x20, t.aim);
    c::set_pv4(w, id, o::TGT + 0x30, t.body);
    c::set_pi32(w, id, o::TMOBY, t.moby.map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, o::KIND, t.kind as i32);
    let p = c::pos(w, id);
    if t.kind != 2 && c::dist3(p, t.body) < 64.0 {
        let off = c::diff_rots(w.m(id).rotation[2], c::atan(t.pos[0] - p[0], t.pos[1] - p[1]));
        let b = w.hero.body_point.map(|x| x.to_f32());
        let clear = w.coll_line(pv([p[0], p[1], p[2] + 1.0, p[3]]), pv(b), 2, None).is_none();
        let s = if off < FRAC_PI_3 { (clear as u8) << 1 } else { clear as u8 };
        c::set_pu8(w, id, o::SEES, s);
    }
}

/// `0x2f00a0`: the head (list 0) at Ratchet's body point.
pub fn look(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    let b = w.hero.body_point.map(|x| x.to_f32());
    let d = [b[0] - p[0], b[1] - p[1], b[2] - (p[2] + 0.5), 0.0];
    let yaw = c::sub_rot(c::atan(d[0], d[1]), w.m(id).rotation[2]).clamp(-FRAC_PI_3, FRAC_PI_3);
    let pitch = (-c::atan(c::len2(d), d[2])).clamp(-FRAC_PI_6, FRAC_PI_6);
    c::set_pf(w, id, o::LOOK + 0x64, pitch);
    c::set_pf(w, id, o::LOOK + 0x68, yaw);
    let s2 = SPEED * SPEED;
    manip::look(w, id, id, o::LOOK, 0, s2 * 0.02, s2 * 0.3);
}

/// `0x2f0210`: the glow's pulse by state (module doc), the eyes' points, the draw.
pub fn glow(w: &mut World, id: MobyId) {
    let level = match st(w, id) {
        4..=6 => 1,
        7..=0xb | 0xd..=0xf => 2,
        0xc => return,
        _ => 0,
    };
    let per = w.ticks(match level { 2 => 0x32, 1 => 0x5a, _ => 0xaa }).max(1);
    let f = (w.counter % per as u64) as f32 / per as f32;
    let s = ((f + f) * std::f32::consts::PI - std::f32::consts::PI).sin();
    let (g, r, b) = ((s * 20.0) as i32, (s * 70.0) as i32, (s * 10.0) as i32);
    let rr = (r + 0xb4).min(0xff) as u32;
    let gg = (g + 0x32).min(0xff) as u32;
    let bb = (b + 0x14) as u32;
    let col = if level != 0 { bb << 16 | 0x8000_0000 | (gg / 2) << 8 | rr } else { bb << 16 | 0x8000_0000 | rr << 8 | gg };
    let gc = crate::hud::tween_color(0.1, w.m(id).glow, col);
    w.mm(id).glow = gc;
    for k in 0..3 {
        let j = w.joint_point(id, k + 1);
        c::set_pv4(w, id, o::EYES + 0x10 * k, j);
    }
    if let Some(row) = super::row(REFERENCE_LEVEL, DRAW_FN) { w.svc.draw_callbacks.register(Callback::UnitGlow(row), id); }
}

/// Level17 `0x2f0448`: three glows (0.2, pull 0.08) on the eyes, the glow colour at alpha 0x30.
pub fn glow_quads(table: &MobyTable, _svc: &Services, id: MobyId) -> Vec<GlowQuad> {
    let Some(m) = table.mobys.get(id).filter(|m| m.pvars.len() >= o::SIZE) else { return Vec::new() };
    (0..3).map(|k| {
        let q = p::v4f(&m.pvars, o::EYES + 0x10 * k);
        GlowQuad { size: 0.2, pull: 0.08, point: [q[0], q[1], q[2]], rgba: (m.glow & 0xff_ffff) | 0x3000_0000 }
    }).collect()
}

/// Level17 `0x2eeb68` (module doc).
#[allow(clippy::too_many_lines)]
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    let cam = w.camera_point();
    let cam = [cam[0], cam[1], cam[2]];
    let mut culled = false;
    if st(w, id) != 0xc {
        let cin = |w: &World, k: usize| w.in_cuboid(cam, c::pi32(w, id, o::CUB + k));
        culled = if c::pi32(w, id, o::CUB) == -1 {
            let mut u = false;
            if c::pi32(w, id, o::CUB + 8) != -1 {
                u = cin(w, 8);
                if c::pi32(w, id, o::CUB + 0xc) != -1 { u |= cin(w, 0xc); }
            }
            u
        } else {
            let mut u = !cin(w, 0);
            if c::pi32(w, id, o::CUB + 4) != -1 && cin(w, 4) { u = false; }
            u
        };
    }
    senses(w, id, culled);
    if culled && st(w, id) != 0xf {
        let m = w.mm(id);
        m.visible = 0;
        m.mode |= 1;
        return;
    }
    let m = w.mm(id);
    m.visible = 1;
    m.mode &= 0xfffe;
    look(w, id);
    glow(w, id);
    manip::big_head_scale(w, 2.1, id, o::LOOK);
    if w.m(id).visible != 0 && c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 29.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0x17;
    }
    let tm = tmoby(w, id);
    match st(w, id) {
        0 => {
            w.mm(id).mode |= mode::TARGETABLE;
            let p = c::pos(w, id);
            c::set_pv4(w, id, o::HOME, p);
            c::set_pu8(w, id, 0x58, 8);
            c::set_pu8(w, id, 0x5a, 7);
            c::set_pu8(w, id, o::D + 9, 0);
            c::set_pi16(w, id, o::D + 4, 1);
            c::set_pf(w, id, o::D, 1.0);
            let s = w.rng.randf(DT * 7.2, DT * 8.7);
            c::set_pf(w, id, o::WALK, s);
            set_st(w, id, 1);
            let n = frames0(w, id);
            let f = w.rng.randi(n.max(1));
            w.anim_blend(id, 0, f, 0);
            if c::pi32(w, id, o::PATH) == -1 { let a = c::pi32(w, id, o::AREA); c::set_pi32(w, id, o::PATH, a); }
        }
        1 => {
            if sees(w, id) != 0 && w.rng.randi(0x1f) == 0 {
                set_st(w, id, 2);
                seq(w, id, 1);
            }
        }
        2 => {
            if wrapped(w, id) {
                if sees(w, id) == 2 { react(w, id); } else { look_about(w, id); }
            }
        }
        3 => {
            if wrapped(w, id) {
                set_st(w, id, 1);
                if w.m(id).anim.seq_b != 0 {
                    let n = frames0(w, id) - 1;
                    let f = w.rng.randi(n.max(1));
                    let t = w.ticks(10);
                    w.anim_blend(id, 0, f, t);
                }
            }
        }
        4 => {
            let r = knock::update(w, id, o::K);
            if r & 0x60 == 0 {
                if w.m(id).position[2] < 2.0 {
                    set_death_bits(w, id, 0, -1);
                    w.delete_moby(id);
                }
                return;
            }
            look_about(w, id);
        }
        5 => {
            let Some(t) = tm else {
                if w.rng.randi(0x1f) == 0 { let n = frames0(w, id); idle(w, id, n); }
                return;
            };
            let (p, q) = (c::pos(w, id), c::pos(w, t));
            let (mut yaw, mut v) = (w.m(id).rotation[2], c::pf(w, id, o::YAW_V));
            turn::turn_toward(c::atan(q[0] - p[0], q[1] - p[1]), DT2 * 4.712_389, DT2 * 4.712_389, DT * 3.490_658_5, &mut yaw, &mut v);
            w.mm(id).rotation[2] = yaw;
            c::set_pf(w, id, o::YAW_V, v);
            if DT * 0.087_266_46 < v.abs() { seq(w, id, if v < 0.0 { 4 } else { 5 }); } else { seq(w, id, 3); }
            w.mm(id).anim.speed = f32::from_bits(0x3fd9_999a);
            if sees(w, id) == 2 && (c::dec_timer_pvar_s16(w, id, o::LOOK_T) != 0 || w.body() != DISGUISED) {
                react(w, id);
                return;
            }
            if c::pi16(w, id, o::SEEN_T) != 0 || sees(w, id) != 0 { return; }
            let n = frames0(w, id);
            idle(w, id, n);
        }
        6 | 7 => {
            if sees(w, id) == 2 {
                if c::pf(w, id, o::IDLE_T) == 0.0 && !in_area(w, id) {
                    if !wrapped(w, id) { return; }
                    if w.rng.randi(3) == 0 {
                        seq(w, id, 2);
                    } else if w.rng.randi(7) == 0 {
                        seq(w, id, 8);
                    } else {
                        seq(w, id, 7);
                    }
                } else if w.rng.randi(0x14) == 0 {
                    set_st(w, id, 8);
                    seq(w, id, 1);
                }
            } else if w.rng.randi(0x14) == 0 {
                look_about(w, id);
            }
        }
        8 => {
            let (p, q) = (c::pos(w, id), tpos(w, id));
            let mut v = c::pf(w, id, o::YAW_V);
            turn::spring_turn2(w, id, c::atan(q[0] - p[0], q[1] - p[1]), 0.03, 0.3, DT * 7.853_981_5, &mut v);
            c::set_pf(w, id, o::YAW_V, v);
            if wrapped(w, id) {
                let chase = c::pf(w, id, o::IDLE_T) != 0.0 || in_area(w, id);
                set_st(w, id, if chase { 9 } else { 0xb });
                seq(w, id, 9);
            }
        }
        9 => {
            let s = c::pf(w, id, o::WALK);
            let q = tpos(w, id);
            walk(w, id, s, q);
            w.mm(id).anim.speed = f32::from_bits(0x3fe6_6666);
            if in_area(w, id) || c::pf(w, id, o::IDLE_T) != 0.0 {
                if c::dist2(c::pos(w, id), q) < 1.8 {
                    set_st(w, id, 10);
                    if w.m(id).anim.seq_b != 10 {
                        let t = w.ticks(10);
                        w.anim_blend(id, 10, 5, t);
                    }
                    return;
                }
                let home = c::pv4(w, id, o::HOME);
                if c::dist3(q, home) <= 32.0 && (q[2] - w.m(id).position[2]).abs() <= 3.0 { return; }
            }
            set_st(w, id, 0xb);
        }
        10 => {
            if wrapped(w, id) {
                set_st(w, id, 0xb);
                seq(w, id, 9);
                return;
            }
            let kt = c::ground::key_time(w, id);
            if 0.25 < (kt - 17.5).abs() { return; }
            let (p, q) = (c::pos(w, id), tpos(w, id));
            if 1.0 <= (q[2] - p[2]).abs() { return; }
            if 0.261_799_4 <= c::diff_rots(c::atan(q[0] - p[0], q[1] - p[1]), w.m(id).rotation[2]) { return; }
            if let Some(t) = tm {
                if c::dist2(p, q) < 2.75 {
                    let y = w.m(id).rotation[2];
                    let at = [q[0], q[1], q[2] + 0.75, q[3]];
                    crate::moby_update::creature::attack::hit_moby(w, t, id, 1.0, 1, at, [y.cos() * 0.2, y.sin() * 0.2, 0.0, 0.0]);
                }
            }
        }
        0xb => {
            let home = c::pv4(w, id, o::HOME);
            walk(w, id, DT * 5.0, home);
            w.mm(id).anim.speed = f32::from_bits(0x3fac_cccd);
            let d = c::dist2(c::pos(w, id), home);
            if (d < 1.4 && wrapped(w, id)) || d < 0.3 {
                look_about(w, id);
                return;
            }
            if !wrapped(w, id) { return; }
            let dt = c::dist2(c::pos(w, id), tpos(w, id));
            let v = c::pf(w, id, o::SPEED) * DT;
            let (s40, s10) = (w.svc.timing.scale(Pf::f(40.0)).to_f32(), w.svc.timing.scale(Pf::f(10.0)).to_f32());
            if v * s40 * 0.5 + v * s10 + 1.5 <= dt { return; }
            if !in_area(w, id) { return; }
            set_st(w, id, 10);
            seq(w, id, 10);
        }
        0xc => {
            if c::react::carried(w, id, o::K) != 0 {
                set_st(w, id, 0);
                c::set_pi16(w, id, 0xc8, 0);
            }
        }
        0xf => {
            w.mm(id).mode &= !mode::TARGETABLE;
            let r = knock::update(w, id, o::K);
            let p = c::pos(w, id);
            if r & 0x60 != 0 {
                fx::death_explosion(w, 0.5, 13.0, Some(id), p, -1);
                let rot = w.m(id).rotation;
                fx::break_piece(w, id, 0x6ce, p, rot, 0, 0);
                fx::break_piece(w, id, 0x6cf, p, rot, 0, 0);
                burst_pieces(w, id, 0x77d, 1, 0x77d, 1, 3, 2);
                set_death_bits(w, id, 0, -1);
                w.delete_moby(id);
            } else if p[2] < 2.0 {
                set_death_bits(w, id, 0, -1);
                w.delete_moby(id);
            }
        }
        _ => {}
    }
}
