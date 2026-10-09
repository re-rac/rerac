//! **Oltanis's small classes** (level 14; read from the level14 decomp; native `f32`):
//!
//! * **250 the hatches** (`0x2d96e0`, census U494, 9 placed): no collision while Ratchet is Clank-free in mode 2
//!   (0x1413f4 = 2), else the class's. With +0x00 set, once the current camera is of class 0x14 it waits +0x04 ticks,
//!   opens (sequence 1, then 2) and stays open until the camera changes (sequence 0, 4). States 5..7 (closing again:
//!   sequence 2, the wait, sequence 3, 0) are entered by no code here.
//! * **309 the floats** (`0x2de1f8`, the tilt `0x2de2b8`, U495, 4 placed): platforms 0.35 above their placement whose
//!   up row leans toward Ratchet while he stands on (or hangs from) it, by up to 10° (gp−0x5100) about the axis across
//!   his offset (0.1 more height: 2, gp−0x50fc), springing back level when he leaves (`Spring(·, 0.005, 0.2, 0)` per
//!   component, the rows rebuilt from it: right = (1, 0, 0) × up); a lean change starts the class sound 0 (flags 8)
//!   at most every `ticks(60)`. The platform block +0x20 carries nothing (no motion).
//!
//! * **643 the floating mines** (`0x2ec810`, U499, 5 placed): drift along their path +0x60 (its chords into the points'
//!   w; a random way round) at 6 a second (gp−0x4fbc), each axis springing to the path point (0.005, 0.2), tumbling at
//!   random rates, their glow pulsing (`(sin·48 + 0xcf, sin·32 + 0x30, …)`); any hit (0x230000) hard enough but by
//!   class 0x283, or Ratchet touching it (his contact moby 0x13f58c): sound 0, a beam explosion (damaging, 3 / 1
//!   radius, when he touched it), deleted.
//! * **1352 the risers** (`0x305408`, U515, one placed): its group +0x00 waits hidden 20 below; once the moby +0x0c
//!   reaches state 2 it marks itself dead (persistent and this visit), arms the camera record +0x10 and raises the
//!   group over `ticks(120)` on the curve `0x26cc00(−1, 0, 1, 0, ·)`. Already marked (or collected): nothing.
//!
//! * **1416 the pressure pads** (`0x306b78`, U519, 2 placed): glowing (once a second, as the lifts) until Ratchet stands
//!   on one: marked dead (persistent and this visit), command 1, green, sound 0, and the walls of the path +0x00 whose
//!   points' w equal +0x04 opened (w = 0). Already marked, collected, or (with +0x08) its mission done: green and open
//!   from the start (command 2).
//! * **1559** (`0x3087c0`, U523, one placed): the group +0x00 emptied → skill point 0x17 (0x13d41f; the jingle, banner
//!   0x53d6); in scenes 3..5 the thrusters on the scene's second actor.
//! * **1395 the stair builder** (`0x305d28`, U517, one placed): makes the base 0x574 at itself (0.48 along its row 0)
//!   and stacks its six steps (+0x28.. → +0x04..) on it 1.05 apart; once the moby +0x1c reaches state 2 the steps flip
//!   down one by one from the top (each `ticks(15)`: its pitch 0 → 90°, the class 0x575's sound 0), then the base
//!   swings round 182° over `ticks(120)` carrying them (its hum looping), and the steps are set to state 2.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2d96e0` | 250 | [`hatch_update`] |
//! | `0x2de1f8` / `0x2de2b8` | 309 | [`float_update`] |
//! | `0x2ec810` | 643 | [`mine_update`] |
//! | `0x305408` / `0x305600` / `0x305680` | 1352 | [`riser_update`] |
//! | `0x306b78` | 1416 | [`pad_update`] |
//! | `0x3087c0` | 1559 | [`skill_update`] |
//! | `0x305d28` / `0x306158` | 1395 | [`stairs_update`] |
//!
//! [L] 309's sound is played 2 above it in the game (`fun_0022d7f0` at the raised position): at the moby here. 1352's
//! camera record (+0x38 = 1, +0x39 = 0: a fly-by) is logged as unported (G-HERO-027); the members' heights the game
//! keeps in the table 0x1ee5e0 are kept in its own pvars.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature as c;
use crate::moby_update::services::World;
use crate::moby_update::story;
use crate::moby_update::triggers;
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 14;
pub const HATCH_FN: u32 = 0x2d_96e0;
pub const HATCH_CLASSES: [i16; 1] = [250];
pub const FLOAT_FN: u32 = 0x2d_e1f8;
pub const FLOAT_CLASSES: [i16; 1] = [309];
pub const MINE_FN: u32 = 0x2e_c810;
pub const MINE_CLASSES: [i16; 1] = [643];
pub const RISER_FN: u32 = 0x30_5408;
pub const RISER_CLASSES: [i16; 1] = [1352];
pub const PAD_FN: u32 = 0x30_6b78;
pub const PAD_CLASSES: [i16; 1] = [1416];
pub const SKILL_FN: u32 = 0x30_87c0;
pub const SKILL_CLASSES: [i16; 1] = [1559];
pub const STAIRS_FN: u32 = 0x30_5d28;
pub const STAIRS_CLASSES: [i16; 1] = [1395];

/// Level14 `0x2d96e0`: 250 (module doc).
pub fn hatch_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x10);
    let coll = if w.body() == 2 { false } else { super::class_collision(w, w.m(id).o_class) };
    w.mm(id).has_collision = coll;
    let cut = |w: &mut World, id: MobyId, s: u8| c::hard_cut(w, id, s, 0);
    let done = w.m(id).anim.flags & 2 != 0;
    let st = w.m(id).state;
    match st {
        0 => {
            if c::pi32(w, id, 0) != 0 {
                if w.camera_class != 0x14 { return; }
                w.mm(id).state = 1;
                let t = w.ticks(c::pi32(w, id, 4));
                c::set_pi32(w, id, 8, t);
            }
        }
        1 => {
            if c::dec_timer_pvar_i32(w, id, 8) == 0 { return; }
            w.mm(id).state = 2;
            cut(w, id, 1);
        }
        2 if done => {
            w.mm(id).state = 3;
            cut(w, id, 2);
        }
        3 if w.camera_class != 0x14 => {
            cut(w, id, 0);
            w.mm(id).state = 4;
        }
        5 if done => {
            w.mm(id).state = 6;
            cut(w, id, 2);
        }
        6 if c::dec_timer_pvar_i32(w, id, 8) != 0 => {
            cut(w, id, 3);
            w.mm(id).state = 7;
        }
        7 if done => {
            w.mm(id).state = 0;
            cut(w, id, 0);
        }
        _ => {}
    }
}

/// Level14 `0x2de2b8`: the lean (module doc).
fn lean(w: &mut World, id: MobyId) {
    let up = [0.0f32, 0.0, 1.0, 1.0];
    let h = &w.hero;
    let on = (h.ground_moby == Some(id) && h.air_ticks == 0) || (h.ledge_blk.moby == Some(id) && h.group == 3);
    let mut t = up;
    if on {
        w.mm(id).cmd = 1;
        let hp = super::hero_pos(w);
        let p = c::pos(w, id);
        if 0.1 <= (hp[0] - p[0]).abs() || 0.1 <= (hp[1] - p[1]).abs() {
            let mut v = c::sub(hp, p);
            v[2] += 2.0;
            let l = c::len3(v);
            if l != 0.0 {
                let a = (c::dot3(up, v) / l).clamp(-1.0, 1.0).asin();
                let tilt = (std::f32::consts::FRAC_PI_2 - a).min(10.0 * 0.017_453_292);
                let axis = [v[1] * up[2] - v[2] * up[1], v[2] * up[0] - v[0] * up[2], v[0] * up[1] - v[1] * up[0]];
                let r = crate::moby_update::classes::blaster_shot::rotate([up[0], up[1], up[2]], tilt, axis);
                t = c::set_len3([r[0], r[1], r[2], up[3]], 1.0);
            }
        }
    } else {
        w.mm(id).cmd = 0;
    }
    let row = w.m(id).rows[2];
    if c::dec_timer_pvar_i32(w, id, 0x70) != 0 && (0..3).any(|k| 0.1 < (row[k] - t[k]).abs()) {
        let tk = w.ticks(0x3c);
        c::set_pi32(w, id, 0x70, tk);
        w.play_sound(0, 8, id);
    }
    let mut r2 = row;
    for k in 0..3 {
        let (mut x, mut v) = (Pf::f(r2[k]), Pf::f(c::pf(w, id, 0x60 + 4 * k)));
        crate::hero::physics::spring(Pf::f(t[k]), Pf::f(f32::from_bits(0x3ba3_d70a)), Pf::f(f32::from_bits(0x3e4c_cccd)), Pf::ZERO, &mut x, &mut v);
        r2[k] = x.to_f32();
        c::set_pf(w, id, 0x60 + 4 * k, v.to_f32());
    }
    let r2 = c::set_len3(r2, 1.0);
    let x = [1.0f32, 0.0, 0.0];
    let r1 = c::set_len3([x[1] * r2[2] - x[2] * r2[1], x[2] * r2[0] - x[0] * r2[2], x[0] * r2[1] - x[1] * r2[0], 0.0], 1.0);
    let r0 = [r2[1] * r1[2] - r2[2] * r1[1], r2[2] * r1[0] - r2[0] * r1[2], r2[0] * r1[1] - r2[1] * r1[0], 0.0];
    let m = w.mm(id);
    m.rows[0] = r0;
    m.rows[1] = [r1[0], r1[1], r1[2], 0.0];
    m.rows[2] = [r2[0], r2[1], r2[2], 0.0];
}

/// Level14 `0x2de1f8`: 309 (module doc).
pub fn float_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x78);
    let rot = w.m(id).rotation;
    if w.m(id).state == 0 {
        let m = w.mm(id);
        m.state = 1;
        m.cmd = 0;
        m.mode |= mode::KEEP_ROWS;
        m.position[2] += 0.35;
        m.rotation = [0.0; 4];
        c::set_pi32(w, id, 0x5c, 1);
        c::set_pi32(w, id, 0x70, 0);
        c::set_pv4(w, id, 0x60, [0.0; 4]);
    }
    lean(w, id);
    let r = w.m(id).rotation;
    triggers::carry_riders(&mut w.mm(id).pvars, 0x20, [0.0; 4], [rot[0], rot[1], rot[2], rot[3]], r);
}

const MINE_BOOM: crate::moby_update::creature::fx::Beam = crate::moby_update::creature::fx::Beam { damage_r: 0.0, damage: 0.0, flash: 3.0, flash2: 5.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 10, sparks: 0x14, puffs: 0x3c, debris: 0, sound: -1, shake: true };

/// Level14 `0x2ec810`: 643 (module doc).
pub fn mine_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x90);
    let path = usize::try_from(c::pi32(w, id, 0x60)).ok().filter(|&p| p < w.svc.splines.len());
    match w.m(id).state {
        0 => {
            let Some(p) = path else { return };
            let n = w.svc.splines[p].len();
            if n == 0 { return; }
            for i in 0..n {
                let a = w.svc.splines[p][i].map(f32::from_bits);
                let b = w.svc.splines[p][(i + 1) % n].map(f32::from_bits);
                w.svc.splines[p][i][3] = c::dist3(a, b).to_bits();
            }
            c::set_pi32(w, id, 0x64, 0);
            c::set_pf(w, id, 0x68, 0.0);
            let dir = if w.rng.randi(2) == 0 { -1.0 } else { 1.0 };
            c::set_pf(w, id, 0x6c, dir);
            w.mm(id).position = w.svc.splines[p][0].map(f32::from_bits);
            w.mm(id).state = 1;
            for (o, lo, hi) in [(0x80, 0x3d56_7750u32, 0x3dd6_7750u32), (0x84, 0x3d0e_fa35, 0x3d8e_fa35), (0x88, 0x3c8e_fa35, 0x3d0e_fa35)] {
                let r = w.rng.randf(f32::from_bits(lo), f32::from_bits(hi));
                c::set_pf(w, id, o, r);
            }
            c::set_pv4(w, id, 0x70, [0.0; 4]);
            let ph = w.rng.randf_sym(0.0, std::f32::consts::PI);
            c::set_pf(w, id, 0x7c, ph);
        }
        1 => {
            let Some(p) = path else { return };
            let pts: Vec<[f32; 4]> = w.svc.splines[p].iter().map(|q| q.map(f32::from_bits)).collect();
            let mut cur = crate::spline::Cursor { seg: c::pi32(w, id, 0x64), t: c::pf(w, id, 0x68) };
            let (q, _) = crate::spline::advance(&pts, true, 6.0 * c::DT * c::pf(w, id, 0x6c), &mut cur);
            c::set_pi32(w, id, 0x64, cur.seg);
            c::set_pf(w, id, 0x68, cur.t);
            for (k, &qk) in q.iter().enumerate() {
                let (mut x, mut v) = (Pf::f(w.m(id).position[k]), Pf::f(c::pf(w, id, 0x70 + 4 * k)));
                crate::hero::physics::spring(Pf::f(qk), Pf::f(f32::from_bits(0x3ca3_d70a)), Pf::f(f32::from_bits(0x3e4c_cccd)), Pf::ZERO, &mut x, &mut v);
                w.mm(id).position[k] = x.to_f32();
                c::set_pf(w, id, 0x70 + 4 * k, v.to_f32());
            }
            let hit = w.get_hit(id, 0x23_0000, false);
            let res = crate::moby_update::creature::damage::resolve(w, id, hit, 0x20, 0, 4);
            let by_283 = hit.and_then(|h| h.attacker).is_some_and(|a| w.m(a).o_class == 0x283);
            let touched = w.hero.cap_moby == Some(id);
            if (res.out5 < 2 || hit.is_none() || by_283) && !touched {
                let (a, b, cc) = (c::pf(w, id, 0x80), c::pf(w, id, 0x84), c::pf(w, id, 0x88));
                let m = w.mm(id);
                m.rotation[0] = c::add_rot(m.rotation[0], a);
                m.rotation[1] = c::add_rot(m.rotation[1], b);
                m.rotation[2] = c::add_rot(m.rotation[2], cc);
                let ph = c::add_rot(c::pf(w, id, 0x7c), f32::from_bits(0x3d56_7750));
                c::set_pf(w, id, 0x7c, ph);
                let s = ph.sin();
                let (r, g) = ((s * 48.0) as i32, (s * 32.0) as i32 + 0x30);
                w.mm(id).glow = (g as u32) << 16 | 0xff00_0000 | (g as u32) << 8 | (r + 0xcf) as u32;
                return;
            }
            w.play_sound(0, 0, id);
            let at = c::pos(w, id);
            let b = if touched { crate::moby_update::creature::fx::Beam { damage_r: 3.0, damage: 1.0, ..MINE_BOOM } } else { MINE_BOOM };
            crate::moby_update::creature::fx::beam_explosion(w, &b, Some(id), at);
            w.delete_moby(id);
        }
        _ => {}
    }
}

/// `0x26cc00(−1, 0, 1, 0, t)` (`follow_camera::script`'s Hermite with these points).
fn rise_curve(t: f32) -> f32 {
    let (p1, p2, p3, p4) = (-1.0f32, 0.0f32, 1.0f32, 0.0f32);
    let f = (p4 - p3) - (p1 - p2);
    f * t * t * t + ((p1 - p2) - f) * t * t + (p3 - p1) * t + p2
}

/// Level14 `0x305408`: 1352 (module doc).
pub fn riser_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x20 + 4 * 64);
    let g = c::pi32(w, id, 0);
    let members = |w: &World| crate::moby_update::scheduler::group_walk(w, g, crate::moby_update::scheduler::GroupWalk::of(true, false));
    match w.m(id).state {
        0 => {
            if c::pi32(w, id, 0xc) < 0 || g < 0 || c::pi32(w, id, 0x10) < 0 {
                w.delete_moby(id);
                return;
            }
            let sid = w.m(id).spawn_id;
            let gone = w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(w.svc.level, sid));
            if gone {
                w.mm(id).state = 3;
                return;
            }
            w.mm(id).state = 1;
            for (k, m) in members(w).into_iter().enumerate().take(64) {
                let z = w.m(m).position[2];
                c::set_pf(w, id, 0x20 + 4 * k, z);
                let mm = w.mm(m);
                mm.mode |= 0x41;
                mm.position[2] -= 20.0;
            }
        }
        1 => {
            let watched = usize::try_from(c::pi32(w, id, 0xc)).ok().filter(|&m| m < w.table.mobys.len());
            if watched.is_none_or(|m| w.m(m).state != 2) { return; }
            story::death_bits(w, id);
            w.mm(id).state = 2;
            let t = w.ticks(0x78);
            c::set_pi32(w, id, 4, t);
            c::set_pf(w, id, 8, 1.0 / t as f32);
            // 0x314168: the camera record +0x10 armed (a fly-by, camera class 19).
            let rec = c::pi32(w, id, 0x10);
            crate::cinematic::flyby_arm(w, rec);
        }
        2 => {
            c::dec_timer_pvar_i32(w, id, 4);
            let f = rise_curve(1.0 - c::pi32(w, id, 4) as f32 * c::pf(w, id, 8));
            for (k, m) in members(w).into_iter().enumerate().take(64) {
                let z0 = c::pf(w, id, 0x20 + 4 * k);
                let lo = z0 - 20.0;
                let mm = w.mm(m);
                mm.mode &= !0x41;
                mm.position[2] = lo + (z0 - lo) * f;
            }
            if c::pi32(w, id, 4) == 0 { w.mm(id).state = 3; }
        }
        _ => {}
    }
}

/// 1416's path walls opened: the points of path +0x00 whose w equals +0x04 get w = 0.
fn open_walls(w: &mut World, id: MobyId) {
    let Some(p) = usize::try_from(c::pi32(w, id, 0)).ok().filter(|&p| p < w.svc.splines.len()) else { return };
    let key = c::pi32(w, id, 4) as f32;
    for q in w.svc.splines[p].iter_mut() {
        if f32::from_bits(q[3]) == key { q[3] = 0; }
    }
}

/// Level14 `0x306b78`: 1416 (module doc).
pub fn pad_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x10);
    match w.m(id).state {
        0 => {
            let a = w.rng.rand_angle();
            c::set_pf(w, id, 0xc, a);
            w.mm(id).position[2] -= 0.35;
            let sid = w.m(id).spawn_id;
            let gone = w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(w.svc.level, sid));
            let mission = w.m(id).mission;
            let open = c::pi32(w, id, 8) != 0 && story::mission_done(w, mission as i32);
            if !gone && !open {
                w.mm(id).state = 1;
            } else {
                let m = w.mm(id);
                m.state = 2;
                m.glow = 0x8020_8020;
                m.cmd = 2;
                open_walls(w, id);
            }
        }
        1 => {
            let ph = c::add_rot(c::pf(w, id, 0xc), c::DT * std::f32::consts::TAU);
            c::set_pf(w, id, 0xc, ph);
            let g = (((ph.sin() * 4.0 - 3.0) * 128.0) as i32).clamp(0x20, 0x80) as u32;
            w.mm(id).glow = g << 16 | g << 8 | 0x8000_0000 | g;
            if w.hero.ground_moby == Some(id) && w.hero.air_ticks == 0 {
                story::death_bits(w, id);
                let m = w.mm(id);
                m.state = 2;
                m.cmd = 1;
                m.glow = 0x8020_8020;
                w.play_sound(0, 0, id);
                open_walls(w, id);
            }
        }
        _ => {}
    }
}

/// Level14 `0x3087c0`: 1559 (module doc).
pub fn skill_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 4);
    let g = c::pi32(w, id, 0);
    if g != -1 && crate::moby_update::scheduler::group_count(w, g, -1) == 0 && !story::skill_point(w, 0x17) {
        story::set_skill_point(w, 0x17);
        w.play_level_sound(story::SKILL_SOUND, 0, None);
        crate::cinematic::show_banner(w, story::SKILL_BANNER, -1);
    }
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.state = 1;
            m.update_dist = 0xff;
        }
        1 => {
            if w.svc.game_mode != 2 { return; }
            let Some(scene) = w.svc.cinematic.scene.clone() else { return };
            if !(3..=5).contains(&scene.id) { return; }
            if let Some(a) = scene.actors.get(1) { crate::moby_update::classes::cutscene_fx::infobot_thrusters(w, a); }
        }
        _ => {}
    }
}

/// The step of 1395 at `k` (its moby + 1 at pvar +0x04 + 4k).
fn step(w: &World, id: MobyId, k: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, 4 + 4 * k) - 1).ok().filter(|&m| m < w.table.mobys.len()) }

/// Level14 `0x305d28`: 1395 (module doc).
pub fn stairs_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x44);
    let base = usize::try_from(c::pi32(w, id, 0) - 1).ok().filter(|&m| m < w.table.mobys.len());
    match w.m(id).state {
        0 => {
            let r0 = w.m(id).rows[0];
            let at = c::add(c::pos(w, id), c::scale(r0, 0.48));
            if base.is_none() {
                if let Some(b) = w.create_moby(0x574) {
                    let (pos, rot) = (c::pos(w, id), w.m(id).rotation);
                    let m = w.mm(b);
                    m.position = pos;
                    m.rotation = rot;
                    m.update_dist = 0x40;
                    m.state = 0;
                    m.draw_dist = 0x40;
                    m.visible = 1;
                    w.build_matrix(b);
                    c::set_pi32(w, id, 0, b as i32 + 1);
                    w.mm(b).position = c::add(w.m(b).position, c::scale(r0, 0.48));
                }
            }
            for k in 0..6 {
                if step(w, id, k).is_some() { continue; }
                let Some(m) = usize::try_from(c::pi32(w, id, 0x28 + 4 * k)).ok().filter(|&m| m < w.table.mobys.len()) else { continue };
                c::set_pi32(w, id, 4 + 4 * k, m as i32 + 1);
                let mut p = at;
                p[2] += 1.05 * k as f32;
                w.mm(m).position = p;
            }
            c::set_pi32(w, id, 0x40, -1);
            if c::pi32(w, id, 0) != 0 { w.mm(id).state = 1; }
        }
        1 => {
            let watched = usize::try_from(c::pi32(w, id, 0x1c)).ok().filter(|&m| m < w.table.mobys.len());
            if watched.is_some_and(|m| w.m(m).state == 2) {
                w.mm(id).state = 2;
                c::set_pi16(w, id, 0x22, 5);
                let t = w.ticks(0xf);
                c::set_pi16(w, id, 0x20, t as i16);
                c::set_pf(w, id, 0x24, 1.0 / t as f32);
                if let Some(s) = step(w, id, 5) { w.play_sound_as(0, 0, s, 0x575); }
            }
        }
        2 => {
            let k = c::pi16(w, id, 0x22).clamp(0, 5) as usize;
            c::dec_timer_pvar_s16(w, id, 0x20);
            let f = c::pi16(w, id, 0x20) as f32 * c::pf(w, id, 0x24);
            if let Some(s) = step(w, id, k) { w.mm(s).rotation[1] = c::lerp_rot(std::f32::consts::FRAC_PI_2, 0.0, f); }
            if c::pi16(w, id, 0x20) == 0 {
                let t = w.ticks(0xf);
                c::set_pi16(w, id, 0x20, t as i16);
                let n = c::pi16(w, id, 0x22) - 1;
                c::set_pi16(w, id, 0x22, n);
                if n < 0 {
                    w.mm(id).state = 3;
                    let t = w.ticks(0x78);
                    c::set_pi16(w, id, 0x20, t as i16);
                    c::set_pf(w, id, 0x24, 1.0 / t as f32);
                } else if let Some(s) = step(w, id, n as usize) {
                    w.play_sound_as(0, 0, s, 0x575);
                }
            }
        }
        3 => {
            let Some(b) = base else { return };
            let hum = c::pi32(w, id, 0x40);
            if !w.sound_alive(hum, id) {
                let v = w.play_sound(0, 4, id);
                c::set_pi32(w, id, 0x40, v);
            }
            c::dec_timer_pvar_s16(w, id, 0x20);
            let f = c::pi16(w, id, 0x20) as f32 * c::pf(w, id, 0x24);
            w.mm(b).rotation[1] = 182.0 * 0.017_453_292 + (0.0 - 182.0 * 0.017_453_292) * f;
            w.build_matrix(b);
            let (bp, r2) = (w.m(b).position, w.m(b).rows[2]);
            for k in 0..6 {
                if let Some(s) = step(w, id, k) { w.mm(s).position = c::add(bp, c::scale(r2, 1.05 * k as f32)); }
            }
            if c::pi16(w, id, 0x20) == 0 {
                let v = c::pi32(w, id, 0x40);
                if v != -1 { w.release_sound(v, id); }
                c::set_pi32(w, id, 0x40, -1);
                w.mm(id).state = 4;
                for k in 0..6 {
                    if let Some(s) = step(w, id, k) { w.mm(s).state = 2; }
                }
            }
        }
        _ => {}
    }
}
