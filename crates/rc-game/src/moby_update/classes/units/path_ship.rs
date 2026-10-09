//! The path ships, class 1212 (levels 02: 6, 09: 3 created instances): level02 0x2ec4b8, the same code on 09
//! (0x305dc0; census U118 / U119), and the big path ships 1213 (02: 6; as 1973 on 09 / 10; level02 0x2ece18, see
//! [`BIG`] below), with the exhaust 0x2eca30 (`engine_trail`), the glow points 0x2ec3f0 and the
//! group's glow callback 0x2ec308. A ship flies its path at 20 units/s (closed on level 09, open elsewhere) keeping
//! its class loop sound 1 alive, trails three type-2 exhaust blobs a tick from its joints 0..2 while drawn, and
//! pulses a glow (its glow colour, drawn as three glow quads at its joints 3..5). Any hit with damage shoots it down:
//! class sound 0 and three falling pieces. Read from the level02 decomp of the four functions and the overlay's data
//! (gp−0x4c9c .. −0x4c44). Native `f32`; the `rand` draws in the game's order.
//!
//! **Pvar block**: +0x2b byte 1, +0x2c byte 0x28 (set at the init), +0x60 s32 the path, +0x64 t (points along it),
//! +0x68 the glow phase, +0x6c s32 the voice slot, +0x70 the first segment's length, +0x74 the start fraction, +0x80
//! / +0x90 / +0xa0 the glow points, +0xb0 the glow quads' colour, +0xb4 the group's once-a-frame word (an offset into
//! the pvar shared data [L]).
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x2ec4b8 | every tick `MobyGetHitMessage(m, 0x10000, 0)` (0x25ca88 = L01 0x26f320) | [`update`] |
//! | state 0 | update / draw distance 0xff; path −1 or no points → `DeleteMoby`; +0x2c = 0x28, +0x2b = 1, +0x70 = `VecDistance`(point 0, point 1); position = point 0; phase = `rand_angle`, slot −1, → 1; mode \|= 0x10 (glow); t = +0x74·count | [`update`] |
//! | state 1 | `SoundIsAlive(m, slot)` (0x28cfa0) no → slot = `PlayClassSound(1, 4, m)` (loop); t += 20·dt / +0x70 (gp−0x4c9c); t > count → t −= count; `0x264718(t, path, level 9, &position, &rotation, 0)` (L01 0x277d40) | [`update`] (`path::pose`) |
//! | | collision (+0x94) on (class +0x10) when x ≥ 8 and y ≥ 8, else off; drawn → 0x2eca30(m, old position); a hit with damage > 0 → 2 | [`update`] |
//! | 0x2eca30 | for joints 0..2 (0x251e38 = L01 0x2645a8): one blob: k1 0.666, k2 0.333, jitter 1.5, w1 `randf(0.333, 0.75)`, w2 `randf(1, 2)`, colours 0x600080ff / 0x6020a0c0 and 0x20802040 / 0x20601020, phases 23 / 11 / 11 with spread 0.5, byte 9 = 8 + 0x60 (gp−0x4c7c .. −0x4c44) | [`exhaust`] (`engine_trail::blob`) |
//! | state 2 | level 02: the kill counter 0x15edf4 + 1 > 2 and skill point 0x13d40a clear → set, `PlayLevelSoundAtMoby(1, 0, 0)`, `ShowBanner(0x53d6, −1)`; level 09: Ratchet in state 0x32, counter 0x15edfc + 1 > 4, skill point 0x13d417 | [`update`] (`story::award_skill_point`; the counters in `Services::level_words` [L: core words, kept per level load]) |
//! | | `PlayClassSound(0, 0, m)`; v = row 0·0.15 (gp−0x4c84 · [0x15ed60]), v.z += 0.08 (gp−0x4c80); `BreakFxB(12·dt², m, 0x5f2 / 0x5f5 / 0x790, position, rotation, ticks(90), 0, v, 0)` (0x2655b0 = L01 0x278ad8); `DeleteMoby` | [`update`] (`fx::break_piece_with`) |
//! | drawn | glow (+0x90) = `FastTweenColor(clamp(sin(phase)·4 − 3, 0, 1), 0x80302020, 0x803030c0)` (gp−0x4c8c / −0x4c88); phase += 360°·dt (gp−0x4c90) | [`update`] |
//! | 0x2ec3f0 | group ≠ −1: joints 3, 4, 5 → +0x80 / +0x90 / +0xa0; +0xb0 = 0x30 alpha with the glow's r, g, b (+0x90..+0x92); frame ≠ 0 and the word ≠ frame → word = frame, `RegisterDrawCallback(0x2ec308, m)` (0x20a2c8 = L01 0x21afe0) | [`glow_points`] (`Callback::UnitGlow`) |
//! | 0x2ec308 | each member of the group (`0x25b8b8` / `0x25b9a0` = L01 0x26e150 / 0x26e238): drawn, same class, +0xb0's low byte > 0x80: three glow quads (0x264b50 = L01 0x2781d0) size 0.9, pull 0.45, at its three points, colour +0xb0 | [`glow_quads`] |
//! | | no light, flag (but the skill points), HUD | n/a |

//! **The big ship 1213** (level02 0x2ece18, census U118; the same code as 1973 on levels 09 / 10, `LevelPorts` runs this
//! row there) is the same machine with its own rows ([`BIG`], gp−0x4c40 .. −0x4bd4): level 10 halves its scale at the
//! init, quarters its speed, deletes it while Clank is the hero (0x1413f4 = 1) and makes its exhaust with the level-10
//! row (`0x2ed658`: the factors negative, the sizes over 1.5; else `0x2ed420`), two blobs from joints 0..1; its path is
//! closed on 09 and 10; its glow points are joints 2..7 (+0x80 .. +0xd0, colour +0xe0, word +0xe4, `0x2ecd50`), drawn
//! by `0x2ecc68` as six quads of size 1.2, pull 0.57; it breaks into 0x5f6 / 0x5f8 / 0x5f9; its skill points are the
//! small ship's (the same counters).

use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, fx, DT, SPEED};
use crate::moby_update::services::{self as sv, Services, World};

use super::engine_trail::{self, Blob};
use super::GlowQuad;

/// The update in the level02 class table.
pub const UPDATE_FN: u32 = 0x2e_c4b8;
pub const REFERENCE_LEVEL: u32 = 2;
pub const CLASSES: [i16; 1] = [1212];
/// 0x15edf4: the path ships shot down on level 02 (gp−0x7e0c).
pub const KILLS_02: u32 = 0x15_edf4;
/// 0x15edfc: the path ships shot down from the turret on level 09 (gp−0x7e04).
pub const KILLS_09: u32 = 0x15_edfc;
/// The big ship's update (level02) and classes.
pub const BIG_FN: u32 = 0x2e_ce18;
pub const BIG_CLASSES: [i16; 2] = [1213, 1973];
pub const EXHAUST: Blob = Blob {
    k1: f32::from_bits(0x3f2a_7efa),
    k2: f32::from_bits(0x3eaa_7efa),
    jitter: 1.5,
    w1: (f32::from_bits(0x3eaa_7efa), 0.75),
    w2: (1.0, 2.0),
    c1: (0x6000_80ff, 0x6020_a0c0),
    c2: (0x2080_2040, 0x2060_1020),
    t: [23, 11, 11],
    spread: 0.5,
    byte9: 0x60,
    size_div: 1.0,
};

/// One kind of path ship (module doc).
pub struct Kind {
    pub speed: f32,
    pub glow_rate: f32,
    pub glow: (u32, u32),
    pub pieces: [i16; 3],
    /// The pieces' velocity: row 0 · .0, z + .1 (· 0x15ed60).
    pub piece_v: (f32, f32),
    pub exhaust_joints: usize,
    pub exhaust: Blob,
    /// The level-10 exhaust row (the big ship's `0x2ed658`).
    pub exhaust_10: Option<Blob>,
    /// The glow points' first joint and count, the colour and word offsets, the quads' size and pull.
    pub glow_joint: usize,
    pub glow_points: usize,
    pub colour: usize,
    pub word: usize,
    pub quad: (f32, f32),
    /// The big ship's level-10 rules (half scale, a quarter of the speed, gone while Clank is the hero) and its path
    /// closed on 10 as on 09.
    pub big: bool,
}

/// Class 1212 (level02 0x2ec4b8; gp−0x4c9c ..).
pub const SMALL: Kind = Kind { speed: 20.0, glow_rate: 360.0, glow: (0x8030_2020, 0x8030_30c0), pieces: [0x5f2, 0x5f5, 0x790], piece_v: (0.15, 0.08), exhaust_joints: 3, exhaust: EXHAUST, exhaust_10: None, glow_joint: 3, glow_points: 3, colour: 0xb0, word: 0xb4, quad: (0.9, 0.45), big: false };

/// Class 1213 / 1973 (level02 0x2ece18; gp−0x4c40 ..).
pub const BIG: Kind = Kind {
    speed: 20.0,
    glow_rate: 360.0,
    glow: (0x8030_2020, 0x8030_30c0),
    pieces: [0x5f6, 0x5f8, 0x5f9],
    piece_v: (0.15, 0.08),
    exhaust_joints: 2,
    exhaust: Blob { k1: f32::from_bits(0x3f2a_7efa), k2: f32::from_bits(0x3eaa_7efa), jitter: 1.5, w1: (f32::from_bits(0x3eaa_7efa), 0.75), w2: (1.0, 2.0), c1: (0x6000_80ff, 0x6020_4060), c2: (0x2080_2040, 0x2060_1020), t: [15, 15, 15], spread: 0.5, byte9: 0x60, size_div: 1.0 },
    exhaust_10: Some(Blob { k1: f32::from_bits(0xbe2a_7efa), k2: f32::from_bits(0xbdaa_7efa), jitter: -0.375, w1: (f32::from_bits(0x3eaa_7efa), 0.75), w2: (1.0, 2.0), c1: (0x6000_80ff, 0x6020_4060), c2: (0x1030_6020, 0x2060_6050), t: [15, 15, 15], spread: 0.5, byte9: 0x60, size_div: 1.5 }),
    glow_joint: 2,
    glow_points: 6,
    colour: 0xe0,
    word: 0xe4,
    quad: (f32::from_bits(0x3f99_999a), f32::from_bits(0x3f11_eb85)),
    big: true,
};

fn kind(o_class: i16) -> &'static Kind { if BIG_CLASSES.contains(&o_class) { &BIG } else { &SMALL } }
const DEG: f32 = 0.017_453_292;

pub mod pv {
    pub const B2B: usize = 0x2b;
    pub const B2C: usize = 0x2c;
    pub const PATH: usize = 0x60;
    pub const T: usize = 0x64;
    pub const PHASE: usize = 0x68;
    pub const SLOT: usize = 0x6c;
    pub const SEG: usize = 0x70;
    pub const START: usize = 0x74;
    pub const POINTS: usize = 0x80;
    pub const SIZE: usize = 0xb8;
    pub const BIG_SIZE: usize = 0xe8;
}

/// Level02 0x2ec4b8 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    let k = kind(w.m(id).o_class);
    if w.m(id).pvars.len() < if k.big { pv::BIG_SIZE } else { pv::SIZE } { return; }
    let hit = w.get_hit(id, 0x1_0000, false);
    let pts = usize::try_from(c::pi32(w, id, pv::PATH)).ok().and_then(|i| w.svc.splines.get(i)).cloned().unwrap_or_default();
    match w.m(id).state {
        0 => {
            let lvl = w.svc.level;
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.draw_dist = 0xff;
            if k.big && lvl == 10 { m.scale *= 0.5; }
            if c::pi32(w, id, pv::PATH) == -1 || pts.is_empty() {
                w.delete_moby(id);
                return;
            }
            let q = |k: usize| pts.get(k).map(|p| p.map(f32::from_bits)).unwrap_or([0.0; 4]);
            let seg = c::dist3(q(0), q(1));
            c::set_pu8(w, id, pv::B2C, 0x28);
            c::set_pf(w, id, pv::SEG, seg);
            c::set_pu8(w, id, pv::B2B, 1);
            w.mm(id).position = q(0);
            let a = w.rng.rand_angle();
            c::set_pf(w, id, pv::PHASE, a);
            c::set_pi32(w, id, pv::SLOT, -1);
            let m = w.mm(id);
            m.state = 1;
            m.mode |= mode::GLOW;
            let t = c::pf(w, id, pv::START) * pts.len() as f32;
            c::set_pf(w, id, pv::T, t);
        }
        1 => {
            if k.big && w.body() == 1 && w.svc.level == 10 {
                w.delete_moby(id);
                return;
            }
            let old = w.m(id).position;
            let slot = c::pi32(w, id, pv::SLOT);
            if !w.sound_alive(slot, id) {
                let s = w.play_sound(1, 4, id);
                c::set_pi32(w, id, pv::SLOT, s);
            }
            let count = pts.len() as f32;
            let mut sp = k.speed * DT;
            if k.big && w.svc.level == 10 { sp *= 0.25; }
            let mut t = c::pf(w, id, pv::T) + sp / c::pf(w, id, pv::SEG);
            if count < t { t -= count; }
            c::set_pf(w, id, pv::T, t);
            let closed = w.svc.level == 9 || (k.big && w.svc.level == 10);
            let (pos, rot) = crate::path::pose(&pts, closed, t, true);
            {
                let m = w.mm(id);
                m.position = pos;
                m.rotation = rot;
            }
            let on = 8.0 <= pos[0] && 8.0 <= pos[1] && super::class_collision(w, w.m(id).o_class);
            w.mm(id).has_collision = on;
            if w.m(id).visible != 0 { exhaust(w, id, k, old); }
            if hit.is_some_and(|h| 0.0 < h.damage.to_f32()) { w.mm(id).state = 2; }
        }
        2 => {
            // The kill counters (session words 0x15edf4 / 0x15edfc) and their skill points: level 02, the third kill
            // → 0x13d40a; level 09 from the turret (state 0x32), the fifth → 0x13d417.
            use crate::moby_update::story::{award_skill_point, skill_index};
            if w.svc.level == 2 {
                let n = w.svc.level_words.get(&KILLS_02).copied().unwrap_or(0) as i32 + 1;
                w.svc.level_words.insert(KILLS_02, n as u32);
                if 2 < n { award_skill_point(w, skill_index(0x13_d40a)); }
            }
            if w.svc.level == 9 && w.hero.state == 0x32 {
                let n = w.svc.level_words.get(&KILLS_09).copied().unwrap_or(0) as i32 + 1;
                w.svc.level_words.insert(KILLS_09, n as u32);
                if 4 < n { award_skill_point(w, skill_index(0x13_d417)); }
            }
            w.play_sound(0, 0, id);
            let (pos, rot, r0) = (w.m(id).position, w.m(id).rotation, w.m(id).rows[0]);
            let mut v = c::scale(r0, k.piece_v.0 * SPEED);
            v[2] += k.piece_v.1 * SPEED;
            let t = w.ticks(0x5a);
            for cl in k.pieces { fx::break_piece_with(w, id, cl, pos, rot, t, 0, v, [0.0; 4], [0.0; 4]); }
            w.delete_moby(id);
            return;
        }
        _ => {}
    }
    if w.m(id).visible != 0 {
        let ph = c::pf(w, id, pv::PHASE);
        let f = (ph.sin() * 4.0 - 3.0).clamp(0.0, 1.0);
        w.mm(id).glow = crate::hud::tween_color(f, k.glow.0, k.glow.1);
        let a = c::add_rot(ph, k.glow_rate * DEG * DT);
        c::set_pf(w, id, pv::PHASE, a);
        glow_points(w, id, k);
    }
}

/// 0x2eca30(m, old) (the big ship's 0x2ed420 / 0x2ed658): the exhaust blobs from joints 0.. of the kind.
fn exhaust(w: &mut World, id: MobyId, k: &Kind, old: [f32; 4]) {
    let d = c::sub(w.m(id).position, old);
    let row = if w.svc.level == 10 { k.exhaust_10.as_ref().unwrap_or(&k.exhaust) } else { &k.exhaust };
    for j in 0..k.exhaust_joints {
        let p = w.joint_point(id, j);
        engine_trail::blob(w, row, p, d);
    }
}

/// 0x2ec3f0 (the big ship's 0x2ecd50): the glow points, the colour and the group's callback (once a frame).
fn glow_points(w: &mut World, id: MobyId, k: &Kind) {
    if w.m(id).group == -1 { return; }
    for i in 0..k.glow_points {
        let p = w.joint_point(id, k.glow_joint + i);
        c::set_pv4(w, id, pv::POINTS + 0x10 * i, p);
    }
    let g = w.m(id).glow;
    c::set_pi32(w, id, k.colour, (0x3000_0000 | (g & 0xff_ffff)) as i32);
    let frame = w.counter as i32;
    let word = c::pi32(w, id, k.word);
    if frame != 0 && w.svc.shared_i32(word) != frame {
        w.svc.set_shared_i32(word, frame);
        let func = if k.big { BIG_FN } else { UPDATE_FN };
        if let Some(i) = super::row(REFERENCE_LEVEL, func) { w.svc.draw_callbacks.register(Callback::UnitGlow(i), id); }
    }
}

/// 0x2ec308 (the big ship's 0x2ecc68) for the group of `id`: the glow quads.
pub fn glow_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Vec<GlowQuad> {
    let Some(m) = table.mobys.get(id) else { return Vec::new() };
    let k = kind(m.o_class);
    let size = if k.big { pv::BIG_SIZE } else { pv::SIZE };
    let Some(ids) = usize::try_from(m.group).ok().and_then(|g| svc.groups.lists.get(g)).and_then(|l| l.clone()) else { return Vec::new() };
    let mut out = Vec::new();
    for mi in ids {
        let Some(o) = table.mobys.get(mi as usize) else { continue };
        if o.visible == 0 || o.o_class != m.o_class || o.pvars.len() < size { continue; }
        let rgba = sv::pvar::u32(&o.pvars, k.colour);
        if rgba & 0xff <= 0x80 { continue; }
        for i in 0..k.glow_points {
            let p = sv::pvar::v4f(&o.pvars, pv::POINTS + 0x10 * i);
            out.push(GlowQuad { size: k.quad.0, pull: k.quad.1, point: [p[0], p[1], p[2]], rgba });
        }
    }
    out
}
