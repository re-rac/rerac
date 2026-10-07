//! **Blarg's escape, class 1108** (level06 `0x301b90` with its countdown draw `0x3021d8` and its lines `0x302400`; 1
//! placed; census U242; the name is descriptive [L]). The station's self-destruct run. While its mission is open it
//! holds the level's later parts (group +0x00 waits in state 4, groups +0x04 / +0x08 hidden in 0x12). When the group
//! +0x1c is all gone it opens the launch tube 1118 (+0x2c); when Ratchet takes it (the tube past 3) a line plays
//! (set 0); when the ride is over (the tube in 7) the station wakes: music (6, 9), the line goes on, group +0x10 to
//! 0x11, the crawlers of groups +0x14 / +0x18 hidden, the held groups let go, the moby +0x20 to 3, the cuboid +0x0c
//! lifted 10, and the script camera flies the path +0x24 looking back at Ratchet (set down at cuboid +0x28) while the
//! 0x5e8 group +0x34 in view starts; at its end the countdown (2700 ticks, 45 s) starts with a line (set 1): an
//! on-screen timer, the station shaking with blasts every 480–600 ticks, a last line (set 2) at 15 s; Ratchet out
//! (state 0x72) stops it (6); at 0 the moby +0x30 goes to 5 and the station whites out: Ratchet dies. Read from the
//! level06 decomp and its words gp−0x4b68..−0x4b18. Native `f32`.
//!
//! **Pvars** (0xc4): +0x00..+0x34 the groups, cuboids, path and mobys above, +0x38 / +0x50 the lines of sets 0 / 2
//! (by the language 0x15ed88 − 1), +0x68 set 1's three choices (×6), +0xb0 the camera's
//! point, +0xb4 the countdown, +0xb8 the white, +0xbc its delay, +0xc0 the blast timer.
//!
//! | state | what | port |
//! |---|---|---|
//! | 0 | the mission byte 0 (open) → group +0x00 to 4 (`0x2f45f0`), groups +0x04 / +0x08 to 0x12 (`0x2f9948`: no collision, not targetable, hidden), update distance 0xff, 1; else deleted | [`update`] |
//! | 1 | `MobyGroupCount(+0x1c, −1)` = 0 → the tube (`0x3047b8`: 1 → 2, its sound 0); the tube past 3 (and not 7) → 2, the line set 0 | [`update`] |
//! | 2 | the tube in 7 → 3, point 1; `MusicRequestTrack(6, 9)`; a line buffered → on; group +0x10 to 0x11 (`0x2ffc38`), groups +0x14 / +0x18 hidden in 0xf (`0x2e9e30`), +0x00's 4 → 0 (`0x2f45a0`), +0x04 / +0x08's 0x12 → 0 (`0x2f99b0`), the moby +0x20 to 3, cuboid +0x0c up 10; `CameraScript(the path's point 1 looking at its point 0, 2, ticks(300), 0)`, springs (0.0001, 1, 0, 0.001, 1, 0), the targets; `HeroTeleport(cuboid +0x28, 0x72, 0)` | [`update`] |
//! | 3 | the camera at point +0xb0 (then + 1) looking back at point −30; the 0x5e8 group +0x34 (`0x309240`: each in state 1 within view of its z + 0.25 → 2, timer `scale(randf(30, 60))`); the last point → 4, the line set 1, the countdown `scale(2700)`, `CameraScript2(2)`, `SetState(0, 1)`, the letterbox off | [`update`] |
//! | 4 | at `ticks(1800)` / `ticks(300)` a buffered line goes on; at `ticks(900)` the line set 2; blasts: `0x29aec0(0)` on Ratchet (the level def 2 + 0, `World::play_level_def`), a shake (0.4 up, `ticks(60)`), the next in `scale(randf(480, 600))`; mode 0 → the countdown draw; Ratchet in 0x72 → 6; mode 0 and the countdown out → 5, the white 0 after `ticks(15)`, the moby +0x30 to 5, `0x29aec0(1)` on Ratchet | [`update`] |
//! | 5 | the white at 1 → Ratchet dies (`0x228230` = `0x2319b0`), the white 0; else after its delay `Approach(1, 6·dt)` | [`update`] (`HeroCall::Death`, `cinematic::set_white`) |
//! | `0x302400(m, set)` | set 0 / 2: +0x38 / +0x50 by the language (0x15ed88 − 1, at least 0); set 1: a random one of three not yet used (all used: reset), +0x68 + language·4 + choice·0x18; a line ≠ −1 → the dialogue request 20000 + it | [`line`] |
//! | `0x3021d8` | the countdown "0M:SS:Cr" at (250, 375) | [`frame`] (`veldin_pads::push_countdown`) |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, turn};
use crate::moby_update::scheduler;
use crate::moby_update::services::{HeroCall, World};
use crate::moby_update::story;

pub const REFERENCE_LEVEL: u32 = 6;
pub const UPDATE_FN: u32 = 0x30_1b90;
pub const DRAW_FN: u32 = 0x30_21d8;
pub const CLASSES: [i16; 1] = [1108];
/// gp−0x4b68..−0x4b50: the script camera's springs; −0x4b3c (2700): the countdown; −0x4b38 / −0x4b34 its centre;
/// −0x4b30 / −0x4b2c its colours; −0x4b48.. the line set 1's used flags (0x1620b8..).
const SPRINGS: [f32; 6] = [f32::from_bits(0x38d1_b717), 1.0, 0.0, f32::from_bits(0x3a83_126f), 1.0, 0.0];
const COUNTDOWN: f32 = 2700.0;
const CENTRE: (i32, i32) = (250, 375);
const COLOURS: [u32; 2] = [0x80c0_c0c0, 0x8040_40ff];
const USED_KEY: u32 = 0x16_20b8;
/// The bots 0x5e8 woken in view, the tube 1118.
const WAKER: i16 = 0x5e8;
/// The dialogue streams of the 20000 range (`PlayDialogue` → `fun_002157d0`, the table 0x13a508).
const LINE_BASE: i32 = 20000;

fn pi(w: &World, id: MobyId, o: usize) -> i32 { c::pi32(w, id, o) }
fn group(w: &World, g: i32) -> Vec<MobyId> { scheduler::group_ids(w, i8::try_from(g).unwrap_or(-1)) }
fn path(w: &World, i: i32) -> Vec<c::V> {
    usize::try_from(i).ok().and_then(|p| w.svc.splines.get(p)).map_or(Vec::new(), |s| s.iter().map(|q| q.map(f32::from_bits)).collect())
}
/// 0x15ed88 (the language) − 1, at least 0: the line sets' column.
fn option(w: &World) -> usize { w.svc.help.text.lang.saturating_sub(1) as usize }

/// `0x302400(m, set)` (module doc).
fn line(w: &mut World, id: MobyId, set: i32) {
    let o = option(w);
    let l = match set {
        0 => pi(w, id, 0x38 + 4 * o),
        2 => pi(w, id, 0x50 + 4 * o),
        1 => {
            let mut k = w.rng.randi(3) as u32;
            let used = |w: &World, k: u32| w.svc.units.word(USED_KEY + 4 * k) != 0;
            if !used(w, k) {
                w.svc.units.set_word(USED_KEY + 4 * k, 1);
            } else {
                let mut tries = 0;
                loop {
                    k = (k + 1) % 3;
                    if tries == 2 {
                        for j in 0..3 { w.svc.units.set_word(USED_KEY + 4 * j, 0); }
                    }
                    tries += 1;
                    if 3 < tries { break; }
                    if !used(w, k) {
                        w.svc.units.set_word(USED_KEY + 4 * k, 1);
                        break;
                    }
                }
            }
            pi(w, id, 0x68 + 4 * o + 0x18 * k as usize)
        }
        _ => -1,
    };
    if l != -1 { w.svc.help.voice.request = l + LINE_BASE; }
}

/// The tube's progress (`0x3047f8`): 2 at the end (state 7), 1 past 3, else 0.
fn tube(w: &World, id: MobyId) -> i32 {
    let Some(t) = story::link(w, pi(w, id, 0x2c)) else { return 0 };
    match w.m(t).state {
        7 => 2,
        s if 3 < s => 1,
        _ => 0,
    }
}

fn look(from: c::V, to: c::V) -> [f32; 3] {
    let yaw = c::atan(to[0] - from[0], to[1] - from[1]);
    [0.0, -c::atan(c::dist2(from, to), to[2] - from[2]), yaw]
}

/// Level06 `0x301b90` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0xc4 { w.mm(id).pvars.resize(0xc4, 0); }
    match w.m(id).state {
        0 => {
            let m = w.m(id).mission as i32;
            if u8::try_from(m).is_ok_and(|mm| w.mission_done(w.svc.level, mm) == 0) {
                for g in group(w, pi(w, id, 0x00)) { w.mm(g).state = 4; }
                for o in [0x04, 0x08] {
                    for g in group(w, pi(w, id, o)) {
                        let mm = w.mm(g);
                        mm.state = 0x12;
                        mm.has_collision = false;
                        mm.mode = (mm.mode & !0x1000) | 1;
                    }
                }
                w.mm(id).update_dist = 0xff;
                w.mm(id).state = 1;
            } else {
                w.delete_moby(id);
            }
        }
        1 => {
            if scheduler::group_count(w, pi(w, id, 0x1c), -1) != 0 { return; }
            if let Some(t) = story::link(w, pi(w, id, 0x2c)) {
                if w.m(t).state == 1 {
                    w.mm(t).state = 2;
                    w.play_sound(0, 0, t);
                }
            }
            if tube(w, id) == 1 {
                w.mm(id).state = 2;
                line(w, id, 0);
            }
        }
        2 => {
            if tube(w, id) != 2 { return; }
            w.mm(id).state = 3;
            c::set_pi32(w, id, 0xb0, 1);
            if let Some(s) = w.sound.as_deref_mut() { s.music_request(6, 9); }
            if w.svc.help.voice.state == 3 { w.svc.help.continue_stream(); }
            for g in group(w, pi(w, id, 0x10)) {
                if w.m(g).state < 0x7f {
                    let m = w.mm(g);
                    m.state = 0x11;
                    m.has_collision = false;
                    m.mode = (m.mode & !0x1000) | 1;
                }
            }
            for o in [0x14, 0x18] {
                for g in group(w, pi(w, id, o)) {
                    if w.m(g).state < 0x7f {
                        let m = w.mm(g);
                        m.state = 0xf;
                        m.has_collision = false;
                        m.mode = (m.mode & !0x1000) | 0x41;
                    }
                }
            }
            for g in group(w, pi(w, id, 0x00)) {
                if w.m(g).state == 4 { w.mm(g).state = 0; }
            }
            for o in [0x04, 0x08] {
                for g in group(w, pi(w, id, o)) {
                    if w.m(g).state == 0x12 {
                        let col = w.m(g).has_class;
                        let m = w.mm(g);
                        m.state = 0;
                        m.mode = (m.mode & !1) | 0x1000;
                        m.has_collision = col;
                    }
                }
            }
            if let Some(m) = story::link(w, pi(w, id, 0x20)) { w.mm(m).state = 3; }
            let i = pi(w, id, 0x0c);
            let v = std::sync::Arc::make_mut(&mut w.svc.volumes);
            if let Some(s) = usize::try_from(i).ok().and_then(|i| v.cuboids.get_mut(i)) { s.matrix[3][2] += 10.0; }
            let pts = path(w, pi(w, id, 0x24));
            if 2 <= pts.len() {
                let (at, e) = (pts[1], look(pts[1], pts[0]));
                let t = w.ticks(300);
                crate::cinematic::camera_script(w, [at[0], at[1], at[2]], e, 2, t, false);
                crate::cinematic::script_springs(w, SPRINGS);
                crate::cinematic::camera_targets(w, Some([at[0], at[1], at[2]]), Some(e));
            }
            story::teleport_to(w, pi(w, id, 0x28), 0x72, false);
        }
        3 => {
            let pts = path(w, pi(w, id, 0x24));
            let k = pi(w, id, 0xb0);
            let p = usize::try_from(k).ok().and_then(|i| pts.get(i)).copied().unwrap_or([0.0; 4]);
            let k = k + 1;
            c::set_pi32(w, id, 0xb0, k);
            if k == pts.len() as i32 - 1 {
                w.mm(id).state = 4;
                line(w, id, 1);
                let n = w.svc.timing.scale(crate::ps2v::Pf::f(COUNTDOWN)).to_f32() as i32;
                c::set_pi32(w, id, 0xb4, n);
                crate::cinematic::camera_script2(w, 2);
                crate::cinematic::hero_state(w, 0, true);
                crate::cinematic::letterbox(w, false);
            } else {
                wake_in_view(w, pi(w, id, 0x34));
                let q = pts[(k - 0x1e).max(0) as usize];
                let e = look(p, q);
                crate::cinematic::camera_targets(w, Some([p[0], p[1], p[2]]), Some(e));
            }
        }
        4 => {
            let n = pi(w, id, 0xb4);
            if n == w.ticks(0x708) || n == w.ticks(300) {
                if w.svc.help.voice.state == 3 { w.svc.help.continue_stream(); }
            } else if n == w.ticks(900) {
                line(w, id, 2);
            }
            if c::dec_timer_pvar_i32(w, id, 0xc0) != 0 {
                if let Some(h) = w.hero_moby { w.play_level_def(0, 0, h); }
                let f = w.rng.randf(480.0, 600.0);
                let t = w.svc.timing.scale(crate::ps2v::Pf::f(f)).to_f32() as i32;
                c::set_pi32(w, id, 0xc0, t);
                let ticks = w.ticks(0x3c);
                w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp: f32::from_bits(0x3ecc_cccd), ticks });
            }
            if w.svc.game_mode == 0 {
                if let Some(r) = super::row(REFERENCE_LEVEL, DRAW_FN) { w.svc.draw_callbacks.register(crate::moby_update::classes::draw_callbacks::Callback::UnitFrame(r), id); }
            }
            if w.hero.state == 0x72 {
                w.mm(id).state = 6;
            } else if w.svc.game_mode == 0 && c::dec_timer_pvar_i32(w, id, 0xb4) != 0 {
                w.mm(id).state = 5;
                c::set_pf(w, id, 0xb8, 0.0);
                let t = w.ticks(0xf);
                c::set_pi32(w, id, 0xbc, t);
                if let Some(m) = story::link(w, pi(w, id, 0x30)) { w.mm(m).state = 5; }
                if let Some(h) = w.hero_moby { w.play_level_def(1, 0, h); }
            }
        }
        5 => {
            let f = c::pf(w, id, 0xb8);
            if f == 1.0 {
                w.hero_fields_mut().call(HeroCall::Death);
                crate::cinematic::set_white(w, 0.0);
            } else if c::dec_timer_pvar_i32(w, id, 0xbc) != 0 {
                let mut f = f;
                turn::approach(1.0, c::DT * 6.0, &mut f);
                c::set_pf(w, id, 0xb8, f);
                crate::cinematic::set_white(w, f);
            }
        }
        _ => {}
    }
}

/// `0x309240(group)`: the group's 0x5e8 bots in state 1 whose point (their z, w 0.25) is in view → 2, their timer
/// (+0x28) `scale(randf(30, 60))`.
fn wake_in_view(w: &mut World, g: i32) {
    for m in group(w, g) {
        if w.m(m).o_class != WAKER || w.m(m).state != 1 { continue; }
        let p = w.m(m).position;
        if !crate::moby_update::creature::fx::in_view(w, 64.0, p, 0.25) { continue; }
        w.mm(m).state = 2;
        let f = w.rng.randf(30.0, 60.0);
        let t = w.svc.timing.scale(crate::ps2v::Pf::f(f)).to_f32() as i32;
        if w.m(m).pvars.len() >= 0x2c { c::set_pi32(w, m, 0x28, t); }
    }
}

/// `0x3021d8` (module doc): the countdown text this frame.
pub fn frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0xc4 { return; }
    let n = pi(w, id, 0xb4);
    super::veldin_pads::push_countdown(w, n, CENTRE, COLOURS);
}
