//! **The Metal Detector** (item 27 = 0x1b, gadget class 585; its update is `0x2f2280` on level 01, the same code in every
//! overlay's class table: L02 `0x2d68a8`, L05 `0x3010a0`, …; its scan draw `0x2f1e28` and the dig's nibble write
//! `0x2f21c0`). Read from the level01 decompiler output and the disassembly of `0x2f1e28`. Its targets are the buried
//! caches 605 (`crate::moby_update::classes::buried_bolts`, ported): every tick each cache offers itself as the nearest
//! (0x141390 / 0x141394) to the detector's tip (this item's pvar +0x10, which the caches read while the detector is the
//! hand item).
//!
//! **What it does.** Held out with ○ held (after the first 15 ticks in hand): the head (joint list 3, a joint node
//! `AttachManipulator(item, 3, 0x1e1600)`) eases toward the nearest cache (yaw within ±70° of Ratchet's, pitch within
//! ±80° of straight down, slerp 0.1 a tick); the scan rings flow out of the head (the draw `0x2f1e28`: up to ten
//! additive squares 0.1 apart along the head's axis, FX texture 8, fading with their distance, in the item's colour);
//! within 10 of a cache it beeps (class sound 1 looping, the pitch rising as it nears: `(10 − 2d)·32767/60 − 0x1fff`)
//! and the item turns red (`(r, g, b) = (255, 25.5·d, 25.5·d)`); within 1 it digs: the cache pays out its value as
//! 5-bolt bolts (`BoltSpawn(cache, tip ± 0.2, v, 0x13, 5, 0)`, v = (`randf(1, 10)`, `randf(−10, 10)`, `randf(1, 12)`)·dt
//! turned by Ratchet's rows), each one adding a dig to the cache's save nibble (`0x2f21c0`: +1, at most 15), the cache
//! is deleted, the search reset, class sound 3. Released, or in the first 15 ticks: the colour back to 0x80808080, the
//! beep released, no rings.
//!
//! **Item pvars** ([`Detector`]): +0x00 / +0x04 / +0x0c sound slots (only +0x0c is ever played: the beep), +0x08 = 0x7fff,
//! +0x10 the tip (joint list 0's point, every tick), +0x20 a byte of the pad record (0x13cac9), +0x24 / +0x28 the
//! scan's phase and reach, +0x30 the head's axis (joint list 3's first row); the node at 0x1e1600 (its quaternion
//! +0x10).
//!
//! **Coverage** (`address | what | status`):
//!
//! | address | what | status |
//! |---|---|---|
//! | 0x2f2280 head | no pvars → return; `0x2645a8(item, 0, +0x10)`: the tip (the posed joints, the head's node in) | ported ([`update`], `item_rows`) |
//! | state 0 | 0x141394 = 100000, 0x141390 = 0; +0x0c = +0x00 = +0x04 = −1, +0x08 = 0x7fff, +0x28 = 0, +0x20 = 0x13cac9, +0x24 = 0; the node cleared; `AttachManipulator(item, 3, 0x1e1600)`; state 1 | ported (the pad byte 0x13cac9 is not modelled: n/a, no reader) |
//! | states 1 / 2 | the aim: nearest valid (class 0x25d, state not 0xfe / 0xfd): xy distance tip → cache, elevation `atan(d, dz)`, bearing `atan(dx, dy)`; Δyaw = bearing − Ratchet's yaw clamped to ±gp−0x5120 (70°), Δpitch = elevation + 90° clamped to ±gp−0x511c (80°); dir = `polar(−3, yaw + Δyaw, Δpitch − 90°)` (0x277b50); the frame: row 0 = dir (length 1), row 1 = −unit((−1, 0, 0) × row 0), row 2 = row 0 × row 1 (`FastVecCross(out, a, b)` = b × a); times the transpose of joint list 2's world rows (normalised): the quaternion (`fun_00214260`); none: the identity | ported ([`aim`], `quat_of`) |
//! | | `0x218828(tip, tip + dir)` | n/a (an empty function: a debug hook) |
//! | | node quaternion = `fun_001fa400(0.1, node, q)` (the nlerp); +0x30 = joint list 3's first world row | ported ([`Detector::node`], `services::quat_nlerp`) |
//! | | ○ pressed (0x13cae4 & 0x20): +0x24 = +0x28 = 0 | ported ([`buried_bolts::Scan`](crate::moby_update::classes::buried_bolts::Scan)) |
//! | | not ○ held (0x13cae0 & 0x20) or ticks in hand 0x140400 ≤ 15: +0x90 = 0x80808080, the slots +0x00 / +0x04 / +0x0c released when still the item's, set −1; return (no draw) | ported (the beep's loop through `fx.item_voices`) |
//! | | distance 0x141394 < 10: the beep: not alive and +0x0c = −1 → `PlayClassSound(1, 4, item)`; the pitch bend `(10 − 2d)·32767/60 − 0x1fff` (`0x2a1988`); +0x90 = `v << 16 \| v << 8 \| 0xff0000ff`, v = trunc(d·255/10) | ported (`SoundCmd::ItemLoop` / `ItemPitch`; a slot lost while −1 is not set replays [L]) |
//! | | a valid nearest and d < 1: the dig loop (module doc), `DeleteMoby(cache)`, 0x141390 = 0, 0x141394 = 100000, +0x08 = 0x7fff, `PlayClassSound(3, 0, item)`, the beep released | ported ([`dig`], `bolt::spawn`, `GameWrite::MetalDetectorDig`) |
//! | | d ≥ 10: +0x90 = 0x80808080, the slots released | ported |
//! | | `RegisterDrawCallback(0x2f1e28, item)` | ported (`Callback::DetectorScan` on Ratchet's moby: the item is not a table moby) |
//! | 0x2f1e28 | phase += dt (wraps at 1), reach = max(reach, phase); axis = unit(+0x30), a = unit(axis × (1, 0, 0)), b = unit(axis × a); from d = phase back in steps of 0.1 (wrapping below 0) for one turn, d ≤ reach: the square centred on tip − d·axis, half-size 0.5·d + 0.23 along a and b, ST (0, 0) (1, 0) (0, 1) (1, 1), colour = (+0x90 & 0xffffff) \| trunc((1 − d)·255) << 24, FX 8, ALPHA (Cs − 0)·As + Cd (additive), TEX1 0xff9000000260 | ported (`buried_bolts::scan_frame`, drawn by `rc-engine` fx_draw) |
//! | 0x2f21c0 | level < 20: the nibble `0x14bf10[level·16 + (n − 1)/2]` (+0xbc = n; odd n − 1: the low nibble, even: the high) + 1, at most 15 | ported (`GameWrite::MetalDetectorDig`, applied by the engine to the saved game) |
//! | sounds, particles, lights, stats, save flags | the class sounds 1 / 3, the bolts, the nibble; no particle, light or stat | — |
//!
//! **Native.** Plain `f32`.

use super::items::{HitSink, ItemEnv};
use super::physics::{ticks, DT};
use super::Hero;
use crate::moby_runtime::MobyTable;
use crate::moby_update::classes::buried_bolts;
use crate::moby_update::creature::{add_rot, atan, sub_rot};
use crate::pad::button;
use crate::rng::Rng;
use rc_formats::moby_anim::JointModifier;

pub const METAL_DETECTOR: i32 = 0x1b;
/// The caches' class (0x25d).
pub const CACHE_CLASS: i16 = 605;
/// The beep (class sound 1, looping) and the dig (class sound 3).
pub const SOUND_BEEP: i32 = 1;
pub const SOUND_DIG: i32 = 3;
/// The aim's limits (gp−0x5120 / gp−0x511c, degrees).
pub const YAW_LIMIT: f32 = 70.0;
pub const PITCH_LIMIT: f32 = 80.0;
/// The beep range and the dig range.
pub const BEEP_RANGE: f32 = 10.0;
pub const DIG_RANGE: f32 = 1.0;
/// The idle colour of the item (+0x90).
pub const GREY: u32 = 0x8080_8080;
/// The joint list the head's node acts on; the list whose rows the aim is taken in; the tip.
pub const HEAD_LIST: usize = 3;
pub const NECK_LIST: usize = 2;
pub const TIP_LIST: usize = 0;

/// The item's pvars and its joint node (module doc).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detector {
    /// +0x08.
    pub w8: i32,
    /// +0x10: the tip (joint list 0's point).
    pub tip: [f32; 3],
    /// +0x30: joint list 3's first world row (the scan's axis).
    pub axis: [f32; 3],
    /// The node 0x1e1600's quaternion (+0x10) and whether it is attached.
    pub quat: [f32; 4],
    pub attached: bool,
    /// The item's colour word +0x90 (the engine draws the hand item with it).
    pub glow: u32,
}

impl Default for Detector {
    fn default() -> Self { Detector { w8: 0, tip: [0.0; 3], axis: [0.0; 3], quat: [0.0, 0.0, 0.0, 1.0], attached: false, glow: GREY } }
}

/// The node `AttachManipulator(item, 3, 0x1e1600)` as the evaluator reads it (None: not attached or no target).
pub fn node(hero: &Hero, targets: &[u8]) -> Option<JointModifier> {
    let d = &hero.gadgets.detector;
    if !d.attached { return None; }
    let t = targets.get(HEAD_LIST).copied().filter(|&t| t != 0xff)?;
    Some(JointModifier { quat: d.quat, ..JointModifier::compose(t) })
}

/// The world rows (x, y, z, point) of the hand item's joint list `list` (`fun_0020cca8` / `0x2645a8`): the item's
/// posed joints, with its modifier list as the last frame drew it (the head's node turns the head, the tip and the
/// scan's axis with it).
fn item_rows(hero: &Hero, env: &ItemEnv, list: usize) -> Option<[[f32; 4]; 4]> {
    let it = hero.items.slot.item.as_ref()?;
    let class = env.data.class(it.o_class)?;
    let chain = class.chains.get(list).filter(|c| !c.is_empty())?;
    let p = rc_formats::moby_anim::evaluate_chains_posed(&class.anim, &it.anim, it.snapshot.as_ref(), &[chain.as_slice()], &[], &hero.gadgets.hand_mods);
    let w = rc_formats::moby_anim::attach_matrix(p.first()?, &it.rows, it.position, it.scale);
    Some(w)
}

fn norm(a: [f32; 3]) -> [f32; 3] { super::guns::with_len(a, 1.0) }

/// The rotation rows → quaternion (`fun_00214260`: x = m21 − m12, …; `creature::react::rows_quat`, the inverse of the
/// evaluator's `quat_rows`).
pub(crate) fn quat_of(m: [[f32; 3]; 3]) -> [f32; 4] { crate::moby_update::creature::react::rows_quat(m) }

/// The aim's target quaternion (module doc, states 1 / 2). `cache` = the nearest valid cache's position.
fn aim(hero: &Hero, env: &ItemEnv, tip: [f32; 3], cache: Option<[f32; 3]>) -> [f32; 4] {
    let Some(c) = cache else { return [0.0, 0.0, 0.0, 1.0] };
    let dxy = ((c[0] - tip[0]).powi(2) + (c[1] - tip[1]).powi(2)).sqrt();
    let elev = atan(dxy, c[2] - tip[2]);
    let bearing = atan(c[0] - tip[0], c[1] - tip[1]);
    let yaw0 = hero.rot[2].to_f32();
    let down = -std::f32::consts::FRAC_PI_2;
    let mut dy = sub_rot(bearing, yaw0);
    let mut dp = sub_rot(elev, down);
    let (ly, lp) = (YAW_LIMIT.to_radians(), PITCH_LIMIT.to_radians());
    if ly < dy.abs() { dy = dy / dy.abs() * ly; }
    if lp < dp.abs() { dp = dp / dp.abs() * lp; }
    let dir = crate::targeting::polar(-3.0, add_rot(dy, yaw0), add_rot(dp, down));
    // `FastVecCross(out, a, b)` = b × a: row 1 = −unit((−1, 0, 0) × row 0), row 2 = row 0 × row 1.
    let r0 = norm(dir);
    let r1 = super::guns::scale3(norm(super::guns::cross3([-1.0, 0.0, 0.0], r0)), -1.0);
    let r2 = super::guns::cross3(r0, r1);
    let Some(j) = item_rows(hero, env, NECK_LIST) else { return [0.0, 0.0, 0.0, 1.0] };
    let jr: [[f32; 3]; 3] = std::array::from_fn(|k| norm([j[k][0], j[k][1], j[k][2]]));
    // `fun_001fa378(out, Jᵀ, frame)`: out_i = Σ_k frame_i[k]·Jᵀ_k, the frame's rows in joint list 2's axes.
    let frame = [r0, r1, r2];
    let l: [[f32; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|c| frame[i][0] * jr[c][0] + frame[i][1] * jr[c][1] + frame[i][2] * jr[c][2]));
    quat_of(l)
}

/// `fun_001fa400(t, out, a, b)` (native).
fn nlerp(t: f32, a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let a1 = a.map(|x| x * (1.0 - t));
    let b1 = b.map(|x| x * t);
    let d = a1[0] * b1[0] + a1[1] * b1[1] + a1[2] * b1[2] + a1[3] * b1[3];
    let q: [f32; 4] = std::array::from_fn(|k| if d < 0.0 { a1[k] - b1[k] } else { a1[k] + b1[k] });
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if l == 0.0 { return [0.0, 0.0, 0.0, 1.0]; }
    q.map(|x| x / l)
}

/// `0x2f2280`, the Metal Detector's update (module doc).
pub fn update(hero: &mut Hero, table: &mut MobyTable, _anim: &dyn super::anim::AnimCtl, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng) {
    if hero.items.slot.item.is_none() { return; }
    hero.gadgets.detector.tip = item_rows(hero, env, TIP_LIST).map_or_else(|| super::guns::item_point(hero, env, TIP_LIST), |w| [w[3][0], w[3][1], w[3][2]]);
    let state = hero.items.slot.item.as_ref().map_or(0, |it| it.mstate);
    if state == 0 {
        let d = &mut hero.gadgets.detector;
        *d = Detector { tip: d.tip, ..Default::default() };
        d.w8 = 0x7fff;
        d.attached = true;
        let hero_ref: &Hero = hero;
        hits.world(table, hero_ref, rng, env.frame as u64, &mut |w| {
            let g = &mut w.svc.buried;
            g.distance = buried_bolts::FAR;
            g.nearest = None;
            g.seen = None;
            g.scan.phase = 0.0;
            g.scan.reach = 0.0;
        });
        if let Some(it) = hero.items.slot.item.as_mut() { it.mstate = 1; }
        return;
    }
    if !matches!(state, 1 | 2) { return; }
    // The nearest cache as the moby loop left it (0x141390 / 0x141394; `buried_bolts::alert_frame` snapshots them
    // before it resets the search for the next tick).
    let (mut nearest, mut distance) = (None, buried_bolts::FAR);
    let hero_ref: &Hero = hero;
    hits.world(table, hero_ref, rng, env.frame as u64, &mut |w| {
        if let Some((id, d)) = w.svc.buried.seen {
            distance = d;
            nearest = w.table.mobys.get(id).filter(|m| m.o_class == CACHE_CLASS && m.state < 0x80).map(|m| (id, [m.position[0], m.position[1], m.position[2]]));
        }
    });
    let tip = hero.gadgets.detector.tip;
    let q = aim(hero, env, tip, nearest.map(|n| n.1));
    let d = &mut hero.gadgets.detector;
    d.quat = nlerp(0.1, d.quat, q);
    if let Some(r) = item_rows(hero, env, HEAD_LIST) { hero.gadgets.detector.axis = [r[0][0], r[0][1], r[0][2]]; }
    let pressed = env.pad.pressed & button::CIRCLE != 0;
    let scanning = env.pad.held & button::CIRCLE != 0 && ticks(15) < hero.items.slot.ticks_ready;
    let (axis, tip) = (hero.gadgets.detector.axis, hero.gadgets.detector.tip);
    let hero_ref: &Hero = hero;
    hits.world(table, hero_ref, rng, env.frame as u64, &mut |w| {
        let s = &mut w.svc.buried.scan;
        if pressed { s.phase = 0.0; s.reach = 0.0; }
        s.origin = tip;
        s.axis = axis;
    });
    if !scanning {
        hero.gadgets.detector.glow = GREY;
        release_beep(hero);
        return;
    }
    if distance < BEEP_RANGE {
        // The beep: the loop (class sound 1, flags 4) and its pitch bend.
        hero.fx.item_voices.push(super::packs::SoundCmd::ItemLoop { n: super::fx::LOOP_ITEM, index: SOUND_BEEP, flags: 4 });
        let pb = (((BEEP_RANGE - (distance + distance)) * 32767.0) / 60.0) as i32 - 0x1fff;
        hero.fx.item_voices.push(super::packs::SoundCmd::ItemPitch { n: super::fx::LOOP_ITEM, pb });
        let v = ((distance * 255.0) / 10.0) as i32 as u32;
        hero.gadgets.detector.glow = (v << 16) | (v << 8) | 0xff00_00ff;
        match nearest.filter(|_| distance < DIG_RANGE) {
            Some((cache, _)) => dig(hero, table, env, hits, rng, cache),
            None => {
                register_scan(hero, table, env, hits, rng);
                return;
            }
        }
    } else {
        hero.gadgets.detector.glow = GREY;
    }
    release_beep(hero);
    register_scan(hero, table, env, hits, rng);
}

/// The slots released (+0x00, +0x04 never play; +0x0c the beep) and forgotten.
fn release_beep(hero: &mut Hero) {
    if hero.fx.item_loops[super::fx::LOOP_ITEM].is_some() { hero.fx.item_voices.push(super::packs::SoundCmd::ItemRelease { n: super::fx::LOOP_ITEM }); }
}

/// `RegisterDrawCallback(0x2f1e28, item)`.
fn register_scan(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng) {
    let me = env.hero_moby;
    let glow = hero.gadgets.detector.glow;
    let hero_ref: &Hero = hero;
    hits.world(table, hero_ref, rng, env.frame as u64, &mut |w| {
        w.svc.buried.scan.glow = glow;
        w.svc.draw_callbacks.register(crate::moby_update::classes::draw_callbacks::Callback::DetectorScan, me);
    });
}

/// The dig (module doc): the cache's value paid out as 5-bolt bolts, a dig per bolt into its save nibble, the cache
/// deleted, the search reset, class sound 3.
fn dig(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng, cache: usize) {
    let rows = hero.rows.map(super::physics::to_f32x3);
    let tip = hero.gadgets.detector.tip;
    let hero_ref: &Hero = hero;
    hits.world(table, hero_ref, rng, env.frame as u64, &mut |w| {
        use crate::moby_update::services::pvar as p;
        let dt = DT.to_f32();
        let level = w.svc.level;
        let w_tip = w.m(cache).position[3];
        let at = [tip[0], tip[1], tip[2], w_tip];
        loop {
            let value = if w.m(cache).pvars.len() >= buried_bolts::pv::LEN { p::i32(&w.m(cache).pvars, buried_bolts::pv::VALUE) } else { 0 };
            if value <= 0 { break; }
            let v = [w.rng.randf(1.0, 10.0) * dt, w.rng.randf(-10.0, 10.0) * dt, w.rng.randf(1.0, 12.0) * dt];
            // `fun_001f9cf8(v, v, 0x13f350)`: v in Ratchet's frame.
            let v: [f32; 3] = std::array::from_fn(|k| v[0] * rows[0][k] + v[1] * rows[1][k] + v[2] * rows[2][k]);
            w.mm(cache).position = at;
            let mut pos = at;
            crate::moby_update::creature::fx::jitter(w, 0.2, &mut pos);
            w.mm(cache).position = pos;
            let pf = |a: [f32; 4]| a.map(crate::ps2v::Pf::f);
            crate::moby_update::classes::bolt::spawn(w, cache, pf(pos), pf([v[0], v[1], v[2], 0.0]), 0x13, 5, 0);
            w.mm(cache).position = at;
            let left = value - buried_bolts::BOLTS_PER_DIG;
            p::set_i32(&mut w.mm(cache).pvars, buried_bolts::pv::VALUE, left);
            // `0x2f21c0`: the cache's dug nibble + 1 (saturating at 15) in the saved game.
            let n = w.m(cache).cmd;
            if level < 20 && n != 0 { buried_bolts::dig(w, level, n); }
        }
        w.delete_moby(cache);
        let g = &mut w.svc.buried;
        g.nearest = None;
        g.seen = None;
        g.distance = buried_bolts::FAR;
    });
    hero.gadgets.detector.w8 = 0x7fff;
    hero.fx.item_sounds.push(SOUND_DIG);
}
