//! **Gemlik's base towers, class 170** (level13 `0x2cefc0`, the hits `0x2cecb8`, the burst `0x2ce688`; census U459;
//! 5 placed) and their **hit proxy, class 1632** (0x660, `0x30c060`, created by the tower). The ship's ground targets:
//! a tower makes a proxy 1.1 above its joint 0 (scale 2.7 × the class's) that passes every hit it takes on to the
//! tower, hums (sound 0, placed 8.7 up) and throws sparks (type 5, one tick in nine). Only hits from above (the
//! attacker more than 8.7 over it) count; its health spent, it is marked dead (`SetDeathBits`), blasts at joint 0
//! (sound 1), plays sequence 1 and blows apart in stages: at frame 8.5 at joint 1, at 17 at joints 2 and 3, the anim
//! over a last blast and five debris bursts along to joint 3, then deleted. Its slot in the targets table 0x1847a8
//! (+0x74) is 0 while it stands, 1 once it is gone. Already dead in the save: gone at once.
//!
//! **The burst** (`0x2ce688(m, p, big)`): six type-13 puffs 0.3 up, four debris pieces 0x162 and four 0x163 / 0x164
//! (out of a box ±1.875 × ±1.875 × 0..3.75, at 0.1..0.23 a tick); big: type-11 streaks (2 + the camera distance under
//! 8, else 10; slower near the camera), and the flashes (9 / 6 white, then orange 9, 6 and 3; the first two only
//! when the frame load is under 0.95).
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2cefc0` | the tower (module doc) | [`update`] |
//! | `0x2cecb8` | its hits (0x330000, column 4; two debug prints) | [`hits`] |
//! | `0x2ce688` | the burst | [`burst`] |
//! | `0x30bfb0` | the proxy's spawn (class 0x660, its parent, scale, position, the parent's class) | [`spawn_proxy`] |
//! | `0x30c060` | the proxy: its parent gone or changed class → deleted; a hit (any) delivered to the parent | [`proxy_update`] |
//!
//! Read from the level13 decomp and disassembly (the explosions' stack arguments). [L] The blasts' velocity argument
//! (0, 0, 8·dt) has no input in the port's beam explosion. Native `f32`.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::debris;
use crate::moby_update::creature::{self as c, damage, flash, fx};
use crate::moby_update::services::{pv, HitTemplate, World};
use crate::moby_update::story;
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 13;
pub const UPDATE_FN: u32 = 0x2c_efc0;
pub const CLASSES: [i16; 1] = [170];
pub const PROXY_FN: u32 = 0x30_c060;
pub const PROXY_CLASSES: [i16; 1] = [PROXY];

const DT: f32 = c::DT;
const PROXY: i16 = 0x660;
/// The targets table 0x1847a8 (bytes; a unit word each here).
const TARGETS: u32 = 0x18_47a8;

mod pv_ {
    pub const D: usize = 0x20;
    pub const COOLDOWN: usize = 0x26;
    pub const F: usize = 0x60;
    pub const HUM: usize = 0x70;
    pub const SLOT: usize = 0x74;
    pub const SIZE: usize = 0x78;
}
use pv_ as o;

const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.0, flash_dist: 0.0, scale: 1.0, light: 20.0, streaks: 10, sparks: 3, puffs: 0x10, debris: 1, sound: -1, shake: true };
const COLOURS_A: [u32; 6] = [0x4f00_8fff, 0x4f00_8fff, 0x4f00_7fff, 0x4f00_6fff, 0x2fff_ffff, 0x2fff_ffff];
const COLOURS_B: [u32; 6] = [0x2f00_5f7f, 0x2f00_4f7f, 0x2f00_3f7f, 0x2f00_004f, 0x2f00_0000, 0x3f00_0000];

fn mark(w: &mut World, id: MobyId, v: u32) {
    let s = c::pi32(w, id, o::SLOT);
    if s != -1 { w.svc.units.set_word(TARGETS + s as u32, v); }
}

/// `0x30bfb0(scale, parent, p)` (module doc).
fn spawn_proxy(w: &mut World, scale: f32, parent: MobyId, p: c::V) {
    let Some(m) = w.create_moby(PROXY) else { return };
    story::pvars(w, m, 0x68);
    let s = w.class_scale(PROXY).to_f32() * scale;
    {
        let mm = w.mm(m);
        mm.update_dist = 0xff;
        mm.draw_dist = 0xff;
        mm.visible = 1;
        mm.state = 0;
        mm.scale = s;
        mm.position = p;
    }
    c::set_pi32(w, m, 0x60, parent as i32 + 1);
    let pc = w.m(parent).o_class;
    c::set_pi32(w, m, 0x64, pc as i32);
    w.build_matrix(m);
}

/// Level13 `0x30c060`: the proxy (module doc).
pub fn proxy_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x68);
    let parent = usize::try_from(c::pi32(w, id, 0x60) - 1).ok().filter(|&p| p < w.table.mobys.len());
    let ok = parent.filter(|&p| { let m = w.m(p); m.o_class as i32 == c::pi32(w, id, 0x64) && m.state != 0xfe && m.state != 0xfd });
    let Some(p) = ok else {
        w.delete_moby(id);
        return;
    };
    if let Some(h) = w.get_hit(id, u32::MAX, false) {
        let t = HitTemplate { dir: h.dir, attacker: h.attacker, flags: h.flags, b18: h.b28, b19: h.b29, h1a: h.h2a, damage: h.damage, w20: h.w30 };
        w.deliver_hit(p, &t);
    }
    w.mm(id).hit_slot = 0xff;
}

/// `0x2ce688(m, p, big)`: the burst (module doc).
pub fn burst(w: &mut World, id: MobyId, p: c::V, big: bool) {
    let up = [p[0], p[1], p[2] + 0.3, p[3]];
    let (j, lo, g, size) = (Pf::f(f32::from_bits(0x3e99_999a)), Pf::f(f32::from_bits(0x3f81_47ae)), Pf::f(f32::from_bits(0x3d4c_cccd)), Pf::f(150_000.0));
    w.part13(j, lo, Pf::f(f32::from_bits(0x3f9a_3d71)), g, size, pv(up), 1, 0x4080_8080);
    for _ in 0..5 { w.part13(j, lo, Pf::f(f32::from_bits(0x3f97_0a3e)), g, size, pv(up), 1, 0x4080_8080); }
    let piece = |w: &mut World, a: f32, b: f32, big_piece: bool| {
        let x = w.rng.randf(-1.875, 1.875);
        let y = w.rng.randf(-1.875, 1.875);
        let z = w.rng.randf(0.0, 3.75);
        let q = c::add([x, y, z, 0.0], up);
        let l = w.rng.randf(0.1, f32::from_bits(0x3e6b_851f));
        let v = c::set_len3(c::sub(q, p), l);
        let class = if big_piece { w.rng.randi(2) as i16 + 0x163 } else { 0x162 };
        debris::spawn(w, a, b, id, v, q, [0.0; 4], class, 0);
    };
    for _ in 0..4 { piece(w, f32::from_bits(0x3f66_6667), f32::from_bits(0x4006_6666), false); }
    for _ in 0..4 { piece(w, f32::from_bits(0x3f99_999a), f32::from_bits(0x4019_999a), true); }
    if !big { return; }
    let cam = w.camera.map(|x| f32::from_bits(x.0));
    let l = c::len3(c::sub(cam, p));
    let n = if l < 8.0 { l as i32 + 2 } else { 10 };
    let extra = if l < 7.0 { 7.0 - l } else { 0.0 };
    for _ in 0..n {
        let s = w.rng.randf(8.0, 10.0) * DT - extra * DT;
        let c1 = COLOURS_A[w.rng.randi(6) as usize];
        let c2 = COLOURS_B[w.rng.randi(6) as usize];
        let (a, b) = (w.ticks(0xf), w.ticks(0x14));
        let l1 = w.rng.rand_range(a, b);
        let (a, b) = (w.ticks(0x19), w.ticks(0x1e));
        let l2 = w.rng.rand_range(a, b);
        w.part11(Pf::f(1_200_000.0), Pf::f(s), pv(p), pv([0.0; 4]), c1, c2, l1, l2, 0, 0);
    }
    let zero = [0.0; 4];
    if fx::frame_load(w).0 < 0.95 {
        let t = w.ticks(0xf);
        debris::flash_spawn(w, 9.0, id, p, zero, t, 0x7f, 0x7f, 0x7f, 0x20);
        let t = w.ticks(0x18);
        debris::flash_spawn(w, 6.0, id, p, zero, t, 0x7f, 0x20, 0, 0x20);
    }
    let t = w.ticks(0x14);
    debris::flash_spawn(w, 9.0, id, p, zero, t, 0x7f, 0x40, 0, 0x30);
    let t = w.ticks(0x1b);
    debris::flash_spawn(w, 6.0, id, p, zero, t, 0x60, 0x10, 0, 0x40);
    let t = w.ticks(0x1d);
    debris::flash_spawn(w, 3.0, id, p, zero, t, 0x20, 0, 0, 0x20);
}

/// `0x2cecb8(m, P, D)`: the hits (module doc).
fn hits(w: &mut World, id: MobyId) {
    if w.m(id).state != 2 {
        let hit = w.get_hit(id, 0x33_0000, false);
        let res = damage::resolve(w, id, hit, o::D, 0, 4);
        if matches!(res.reaction, 1 | 2) { c::set_pf(w, id, o::D, 0.0); }
        if 1 < res.out5 {
            let Some(h) = res.hit else {
                w.mm(id).hit_slot = 0xff;
                flash::update(w, id, o::F);
                return;
            };
            let above = h.attacker.is_some_and(|a| c::pos(w, id)[2] + 8.7 < w.m(a).position[2]);
            if above {
                let dmg = h.damage.to_f32();
                let hp = c::pf(w, id, o::D);
                if hp <= dmg {
                    c::set_pf(w, id, o::D, 0.0);
                    w.mm(id).mode &= !mode::TARGETABLE;
                    set_death_bits(w, id, 0, -1);
                    c::set_pu8(w, id, o::F + 7, 0x78);
                    flash::start(w, id, o::F);
                    let j0 = w.joint_point(id, 0);
                    fx::beam_explosion(w, &fx::Beam { sound: 1, ..BLAST }, Some(id), j0);
                    if w.m(id).anim.seq_b != 1 { w.anim_blend(id, 1, 0, 0); }
                    let v = c::pi32(w, id, o::HUM);
                    if v != -1 { w.release_sound(v, id); }
                    c::set_pi32(w, id, o::HUM, -1);
                    w.mm(id).state = 2;
                } else {
                    c::set_pf(w, id, o::D, hp - dmg);
                    c::set_pu8(w, id, o::F + 7, 0xfa);
                    let t = w.ticks(0x3c);
                    c::set_pi16(w, id, o::COOLDOWN, t as i16);
                    flash::start(w, id, o::F);
                }
            }
        }
        w.mm(id).hit_slot = 0xff;
    }
    flash::update(w, id, o::F);
}

/// Level13 `0x2cefc0` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.cmd = 0;
            m.state = 1;
            if (m.update_dist as i16) < m.draw_dist { m.update_dist = m.draw_dist as u8; }
            c::set_pi32(w, id, o::HUM, -1);
            let dead = w.svc.save.death.contains(&(w.svc.level, w.m(id).spawn_id));
            if dead {
                mark(w, id, 1);
                w.delete_moby(id);
                return;
            }
            let mut p = w.joint_point(id, 0);
            p[2] += 1.1;
            spawn_proxy(w, f32::from_bits(0x402c_cccd), id, p);
            w.mm(id).mode &= !mode::TARGETABLE;
        }
        1 => {
            mark(w, id, 0);
            let v = c::pi32(w, id, o::HUM);
            let alive = v != -1 && w.sound_alive(v, id);
            if !alive {
                let v = if v == -1 { w.play_sound(0, 4, id) } else { v };
                c::set_pi32(w, id, o::HUM, v);
                if v != -1 {
                    let p = c::pos(w, id);
                    w.hand_over_sound(v, id, [p[0], p[1], p[2] + 8.7]);
                }
            }
            if w.m(id).visible != 0 && w.rng.randi(9) == 0 {
                let r = w.rng.rand();
                let g = w.rng.rand();
                let b = w.rng.rand();
                let mut p = w.joint_point(id, 0);
                p[2] += 0.5;
                let grow = w.rng.randf(900_000.0, 1_800_000.0);
                let k = w.rng.rand() % 0x28 + 0x28;
                let life = w.ticks(k);
                fx::part05(w, grow, 0.0, p, [(r + 0x30) as u32 & 0x3f, (g + 0x20) as u32 & 0x3f, b as u32 & 0x2f], life);
            }
            hits(w, id);
        }
        2 => {
            let k = w.m(id).cmd;
            let frame = match k {
                0 => 8.5,
                1 | 2 => 17.0,
                _ => 1000.0,
            };
            if frame < 100.0 && c::ground::passed_frame(w, id, frame) {
                let jp = w.joint_point(id, k as usize + 1);
                fx::beam_explosion(w, &BLAST, Some(id), jp);
                w.mm(id).cmd = k + 1;
            }
            if w.m(id).anim.flags & 2 == 0 { return; }
            let p = c::pos(w, id);
            fx::beam_explosion(w, &BLAST, Some(id), p);
            let d = c::sub(w.joint_point(id, 3), p);
            for _ in 0..5 {
                let k = w.rng.randf(0.0, 3.0);
                let q = c::add(c::scale(d, k), p);
                burst(w, id, q, true);
            }
            mark(w, id, 1);
            w.delete_moby(id);
        }
        _ => {}
    }
}
