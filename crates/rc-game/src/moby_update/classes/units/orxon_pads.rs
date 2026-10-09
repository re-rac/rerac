//! **Clank's teleport pads on Orxon, classes 351 and 1301** (level10 `0x2be858`, census U347; 5 + 5 placed, each
//! linked to its partner by pvar 0). Clank (0x1413f4 = 1) stepping within 0.5 (xy) of a pad: ten sparkles (type 53)
//! about him, sound 0, `HeroTeleport` onto the partner (its point and rotation, state 0x43), the follow camera's row
//! blend (when it is current: D+0x20 = `ticks(90)`, its saved forward from its forward), the camera timer 0x1618a8 =
//! `ticks(120)`, and ten sparkles at the partner. Every `ticks(60)` ticks a linked pad sends up 16 rings (type 45,
//! 1.8 up, 0.1/16 apart, sinking at 1.5·dt); its glow pulses over 150 ticks between 0x80464646 and 0x80828282.
//! While the camera timer runs (every pad counts it down: the game's one global) the follow camera's horizontal
//! spring is `0.002` (0.002 to 0.01 over its last `ticks(60)`) with 0.2, the stick off, the look from the smoothed
//! target and the leash 0.
//!
//! **A sparkle**: size `randf(0.1, 1)`·(0.07, 0.7), a colour of random channels `rand_range(0x40, 0xff)` at alpha 0x7f,
//! at a random point within 0.5 (each axis by its own angle and length; z `randf(0, 0.5)`), moving out at `randf(0,
//! dt)` (the first ten from a zero direction: the game normalises a stack vector it never set [L]) and up at
//! `randf(2.5, 5)·dt`, falling at 15·dt², `ticks(rand_range(15, 30))` ticks, spinning either way.
//!
//! Read from the level10 decomp. The camera timer and its two bounds (gp−0x5360 / −0x535c: 120, 60, re-`ticks`ed by
//! every pad's start) are unit words. Native `f32`.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2be858` | the pads | [`update`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, V, DT};
use crate::moby_update::services::{pf, pv, World};
use crate::moby_update::story;
use std::f32::consts::PI;

pub const REFERENCE_LEVEL: u32 = 10;
pub const UPDATE_FN: u32 = 0x2b_e858;
pub const CLASSES: [i16; 2] = [351, 1301];

/// gp−0x5358: the camera timer (0x1618a8).
const TIMER_KEY: u32 = 0x16_18a8;
const SPAN: i32 = 0x78;
const EASE: i32 = 0x3c;

fn link(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, 0)).ok().filter(|&m| m < w.table.mobys.len()) }

/// Ten sparkles about `at` (`zero_dir`: the first batch's unset direction).
fn sparkles(w: &mut World, at: V, zero_dir: bool) {
    for _ in 0..10 {
        let mut dir = [0.0f32; 4];
        if !zero_dir {
            dir[0] = w.rng.randf(-1.0, 1.0);
            dir[1] = w.rng.randf(-1.0, 1.0);
        }
        let k = w.rng.randf(f32::from_bits(0x3dcc_cccd), 1.0);
        let r = w.rng.rand_range(0x40, 0xff) as u32;
        let g = w.rng.rand_range(0x40, 0xff) as u32;
        let b = w.rng.rand_range(0x40, 0xff) as u32;
        let a = w.rng.rand_angle();
        let x = a.cos() * w.rng.randf(0.0, 0.5);
        let a = w.rng.rand_angle();
        let y = a.sin() * w.rng.randf(0.0, 0.5);
        let z = w.rng.randf(0.0, 0.5);
        let p = [x + at[0], y + at[1], z + at[2], at[3]];
        let l = w.rng.randf(0.0, DT);
        let mut v = c::set_len3(dir, l);
        v[2] = w.rng.randf(DT * 2.5, DT * 5.0);
        let n = w.rng.rand_range(0xf, 0x1e);
        let life = w.ticks(n);
        let spin = if w.rng.randi(2) == 0 { -1 } else { 1 };
        let rgba = b << 16 | g << 8 | 0x7f00_0000 | r;
        w.part53(pf(k * 0.07), pf(k * 0.7), pf(DT * DT * 15.0), pv(p), life, rgba, 0, spin, pv(v));
    }
}

/// Level10 `0x2be858` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 4);
    if w.m(id).state == 0 {
        w.svc.units.set_word(TIMER_KEY, 0);
        w.mm(id).state = 1;
    }
    if w.body() == 1 {
        if let Some(dest) = link(w, id) {
            let p = c::pos(w, id);
            let h = w.hero_point();
            if ((p[0] - h[0]).powi(2) + (p[1] - h[1]).powi(2)).sqrt() < 0.5 {
                sparkles(w, [h[0], h[1], h[2], 0.0], true);
                w.play_sound(0, 0, id);
                let (dp, dr) = (w.m(dest).position, w.m(dest).rotation);
                crate::cinematic::hero_teleport(w, [dp[0], dp[1], dp[2]], [dr[0], dr[1], dr[2]], 0x43, false);
                let t = w.ticks(SPAN) as u32;
                w.svc.units.set_word(TIMER_KEY, t);
                w.svc.cinematic.calls.push(crate::cinematic::CinematicCall::FollowRowBlend);
                sparkles(w, dp, false);
            }
        }
    }
    let every = w.ticks(0x3c).max(1) as u64;
    if w.counter.is_multiple_of(every) && link(w, id).is_some() {
        let p = c::pos(w, id);
        for i in 0..16 {
            let q = [p[0], p[1], p[2] + i as f32 * 0.1 * 0.0625 + 1.8, p[3]];
            if let Some(sys) = w.particles.as_deref_mut() {
                if let Some(r) = crate::particles::type45::spawn45(sys, w.rng, 0.0, DT * 315_000.0, q, 0x1eff_7f40) {
                    let rec = &mut sys.pool.recs[r];
                    crate::particles::rec::set_ff(rec, 0x20, 0.0);
                    crate::particles::rec::set_ff(rec, 0x24, 0.0);
                    crate::particles::rec::set_ff(rec, 0x28, DT * -1.5);
                }
            }
        }
    }
    let mut f = ((w.counter % 0x96) as f32 / 150.0) * 2.0 * PI;
    if PI < f { f -= 2.0 * PI; }
    w.mm(id).glow = crate::particles::tween_color(((f.sin() + 1.0) * 0.5).to_bits(), 0x8046_4646, 0x8082_8282);
    let mut t = w.svc.units.word(TIMER_KEY) as i32;
    crate::moby_update::creature::dec_timer_i32(&mut t);
    w.svc.units.set_word(TIMER_KEY, t as u32);
    if t != 0 {
        let ease = w.ticks(EASE);
        let k = if ease < t { 0.002 } else { (t as f32 / ease as f32) * f32::from_bits(0x3c03_126e) + 0.002 };
        use crate::cinematic::CinematicCall as C;
        let calls = &mut w.svc.cinematic.calls;
        calls.push(C::FollowHSpring { k, d: f32::from_bits(0x3e4c_cccd) });
        calls.push(C::FollowStickOff);
        calls.push(C::FollowLookSmoothed);
        calls.push(C::FollowLeash(0));
    }
}
