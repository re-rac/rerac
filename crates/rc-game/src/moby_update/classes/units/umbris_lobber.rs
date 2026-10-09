//! Umbris' lobbing turrets, class 1041 (level 07, 8 created instances), and their shot 882: level07 0x30d4d8 and
//! 0x30bbc8 (census U252; the shot is created by code, its update in the class table), with the shot spawner 0x30ba90.
//! A turret with health that spins its head (a manipulator on joint list 0) and, in volleys, lobs spinning shots at a
//! point 3–11 ahead of its aim heading (toward Ratchet when he is within 20), every 7 ticks while it is drawn (or
//! every other 7 ticks within 90 of the camera, or within 40); the volleys are 3 s apart. Hits flash it red; when a
//! hit's damage reaches its health it breaks (save bits, beam explosion, two pieces). A shot flies under gravity and
//! blows up (damage 1 in radius 2) on what it touches. Read from the level07 decomp of the three functions and the
//! disassembly of the two explosion calls (their stack arguments and branches). Native `f32`; the `rand` draws in
//! the game's order.
//!
//! **Pvar block** (turret): +0x20 the damage record (health +0x20, +0x26 s16), +0x60 the flash record (+0x67 the red),
//! +0x70 the manipulator record (quaternion +0x80), +0xb0 s32 the start delay (ticks), +0xb4 s32 the next volley's
//! frame, +0xb8 s16 the shot timer, +0xba s16 the shots of the volley, +0xbc the aim heading, +0xc0 the head's spin,
//! +0xc4 the shot toggle. **Shot**: +0x00 the velocity, +0x10 the turret, +0x14 s32 its number in the volley,
//! +0x18 / +0x1c the spin rates of rotation y / z.
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x30d4d8 | drawn (+0x31) and within 27 of the camera (0x166e40): the shadow probe 0x282b80 (L01 0x26f020), +0x7f = 0x15 | [`update`] (`shadows::probe_down`) |
//! | | no pvar block → nothing; `MobyGetHitMessage(m, 0x230000, 0)` (0x282e80); `0x282ed8(m, hit, +0x20, 0, &out, 0, 0, 4)` (L01 0x26f378): reaction 1 / 2 → health 0 | [`update`] (`damage::resolve`) |
//! | | out > 1: a hit from a class-0x372 shot is ignored; damage = the hit's +0x2c (no hit: 0 [L], the game reads address 0x2c); health ≤ damage → `SetDeathBits(m, 0, −1)` (0x27fe98 = L01 0x26c250), health 0, `SpawnBeamExplosion(0, 0, 4, 2, 9, 1, 15, m, 0, NULL, 10, 3, 16, sound 1, shake, 1 debris, −1, 0)` (0x286e70), `BreakFxB(0, m, 0x621 / 0x622, position, rotation, 0, 0, 0, 0)` (0x28c6d0), `DeleteMoby`; else health −= damage, red 0xfa, +0x26 = `ticks(60)`, the flash (0x285e78 = L01 0x272318) | [`update`] (`crate_::set_death_bits`, `fx::beam_explosion`, `fx::break_piece`, `flash::start`) |
//! | | +0xa4 = 0xff; the flash 0x285f58 (L01 0x2723f8); the target (0x2886d8 = L01 0x274b78, range 20) | [`update`] (`flash::update`, `target::acquire`) |
//! | state 0 | next volley = frame + `ticks(+0xb0)` + `ticks(rand_range(0, 10))`; `AttachManipulator(m, 0, +0x70)`; spin 0; `PlayClassSound(0, 4, m)` (loop); → 1; toggle 0 | [`update`] (`manip::attach`) |
//! | state 1 | frame ≥ next → next = frame + `ticks(180)`, shot timer 0, aim = `rand_angle`, shots 0, → 2 | [`update`] |
//! | state 2 | `FastDecTimer(s16 +0xb8)` out → timer = `ticks(7)`; drawn → fire; else within 90 of the camera (xy, `vec_distance2`) with toggle 1, or within 40 → fire | [`update`] |
//! | fire | toggle ^= 1; the target within 20 (xy) → aim = `atan`(target − position); v = (`randf(3, 11)`, `randf(−2, 2)`, 0) turned by the aim (`euler_to_matrix`, `0x1f9d20`); step = unit(v)·7·dt; landing = position + v | [`fire`] |
//! | | the muzzle: of joints 1..3 (0x2781f0 = L01 0x2645a8) the one whose heading from the turret is nearest aim − 1020°·dt (start π); vz = `0x283650(7·dt, −9.8·dt², muzzle, landing, 0)` (L01 0x26faf0); muzzle += step; `0x30ba90(muzzle, (step.xy, vz), m, shots)`; shots += 1 | [`fire`] (`knock::lob_up`) |
//! | | frame ≥ next → next = frame + `ticks(120)`, → 1 | [`update`] |
//! | state ≠ 0 | spin += 1020°·dt (gp−0x7e94 = dt); `FUN_00221e38(spin, +0x80, 2)` (0x227d90) | [`update`] (`manip::set_axis`) |
//! | 0x30ba90 | `CreateMoby(0x372)`: draw / update distance 0xff, drawn, three `rand_angle` rotations, spin rates `randf_sym(100, 300)`·1°·dt ×2 (0x280670 = L01 0x26ca28), position = muzzle, velocity, turret, number; its sphere | [`spawn_shot`] |
//! | 0x30bbc8 | old = position; position += velocity; vz −= 9.8·dt²; rot.y += +0x18, rot.z += +0x1c | [`shot_update`] |
//! | | `CollLine_Fix(old, position, 0, turret)` or `coll_sphere(0.5, position, 0, turret)` (0x218b48 = L01 0x212960): none, or the hit moby is a 0x371 / 0x372 → outside [2, 1021]³ → `DeleteMoby`, else nothing | [`shot_update`] |
//! | | not drawn and Ratchet 5 or more away (xy) → `PlayClassSound(0, 0, m)`, `DeleteMoby` | [`shot_update`] |
//! | | light = number % 3 = 0 ? 10 : 0; camera > 50 away (xy): odd → scale 1.3, 1 streak, 3 puffs, no sound; even → 1.2, 0, 3, sound 0; near: scale 1, 4 puffs, sound 0, odd → flash 1, 0 streaks, even → flash 0, 1 streak; `SpawnBeamExplosion(2, 1, 0, flash, 1000, scale, light, m, velocity, NULL, streaks, 2, puffs, sound, no shake, 1 debris, −1, 0)`; `DeleteMoby` | [`shot_update`] (`fx::beam_explosion`) |
//! | | no light (the explosions' own), flag, HUD | n/a |

use crate::moby_runtime::MobyId;
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::creature::{self as c, damage, flash, fx, knock, target, DT, DT2};
use crate::moby_update::manip;
use crate::moby_update::services::{fast_dec_timer_s16, World};

/// The updates in the level07 class table.
pub const UPDATE_FN: u32 = 0x30_d4d8;
pub const SHOT_FN: u32 = 0x30_bbc8;
pub const REFERENCE_LEVEL: u32 = 7;
pub const CLASSES: [i16; 1] = [1041];
pub const SHOT_CLASSES: [i16; 1] = [0x372];
pub const PIECES: [i16; 2] = [0x621, 0x622];

pub mod pv {
    pub const RECORD: usize = 0x20;
    pub const FLASH: usize = 0x60;
    pub const REC: usize = 0x70;
    pub const DELAY: usize = 0xb0;
    pub const NEXT: usize = 0xb4;
    pub const TIMER: usize = 0xb8;
    pub const SHOTS: usize = 0xba;
    pub const AIM: usize = 0xbc;
    pub const SPIN: usize = 0xc0;
    pub const TOGGLE: usize = 0xc4;
    pub const SIZE: usize = 0xc8;
}

/// 1020°/s (17.802359 rad).
const SPIN: f32 = 17.802_359;
const DEG: f32 = 0.017_453_292;
pub const DEATH: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 4.0, flash2: 2.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 10, sparks: 3, puffs: 0x10, debris: 1, sound: 1, shake: true };

fn frame(w: &World) -> i32 { w.counter as i32 }

/// Level07 0x30d4d8 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    let cam = w.camera_point();
    let cam = [cam[0], cam[1], cam[2], 0.0];
    if w.m(id).visible != 0 && c::dist3(w.m(id).position, cam) < 27.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0x15;
    }
    if w.m(id).pvars.len() < pv::SIZE { return; }
    let hit = w.get_hit(id, 0x23_0000, false);
    let r = damage::resolve(w, id, hit, pv::RECORD, 0, 4);
    if matches!(r.reaction, 1 | 2) { c::set_pf(w, id, pv::RECORD, 0.0); }
    let own_shot = hit.and_then(|h| h.attacker).is_some_and(|a| w.table.mobys.get(a).is_some_and(|m| m.o_class == SHOT_CLASSES[0]));
    if r.out5 > 1 && !own_shot {
        let dmg = r.damage;
        let hp = c::pf(w, id, pv::RECORD);
        if hp <= dmg {
            set_death_bits(w, id, 0, -1);
            c::set_pf(w, id, pv::RECORD, 0.0);
            let (pos, rot) = (w.m(id).position, w.m(id).rotation);
            fx::beam_explosion(w, &DEATH, Some(id), pos);
            for cl in PIECES { fx::break_piece(w, id, cl, pos, rot, 0, 0); }
            w.delete_moby(id);
            return;
        }
        c::set_pf(w, id, pv::RECORD, hp - dmg);
        c::set_pu8(w, id, pv::FLASH + 7, 0xfa);
        let t = w.ticks(0x3c);
        c::set_pi16(w, id, pv::RECORD + 6, t as i16);
        flash::start(w, id, pv::FLASH);
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pv::FLASH);
    let tgt = target::acquire(w, id, 20.0);
    match w.m(id).state {
        0 => {
            let d = w.ticks(c::pi32(w, id, pv::DELAY));
            let k = w.rng.rand_range(0, 10);
            let k = w.ticks(k);
            c::set_pi32(w, id, pv::NEXT, frame(w) + d + k);
            manip::attach(w, id, 0, id, pv::REC);
            c::set_pf(w, id, pv::SPIN, 0.0);
            w.play_sound(0, 4, id);
            w.mm(id).state = 1;
            c::set_pi32(w, id, pv::TOGGLE, 0);
        }
        1 => {
            if frame(w) >= c::pi32(w, id, pv::NEXT) {
                let t = w.ticks(0xb4);
                c::set_pi32(w, id, pv::NEXT, frame(w) + t);
                c::set_pi16(w, id, pv::TIMER, 0);
                let a = w.rng.rand_angle();
                c::set_pf(w, id, pv::AIM, a);
                c::set_pi16(w, id, pv::SHOTS, 0);
                w.mm(id).state = 2;
            }
        }
        2 => {
            let mut t = c::pi16(w, id, pv::TIMER);
            let fired = fast_dec_timer_s16(&mut t);
            c::set_pi16(w, id, pv::TIMER, t);
            if fired != 0 {
                let t7 = w.ticks(7);
                c::set_pi16(w, id, pv::TIMER, t7 as i16);
                let d = c::dist2(w.m(id).position, cam);
                let toggle = c::pi32(w, id, pv::TOGGLE);
                if w.m(id).visible != 0 || (d < 90.0 && toggle & 1 != 0) || d < 40.0 { fire(w, id, tgt.pos); }
            }
            if frame(w) >= c::pi32(w, id, pv::NEXT) {
                let t = w.ticks(0x78);
                c::set_pi32(w, id, pv::NEXT, frame(w) + t);
                w.mm(id).state = 1;
            }
        }
        _ => {}
    }
    if w.m(id).state != 0 {
        let a = c::add_rot(c::pf(w, id, pv::SPIN), DT * SPIN);
        c::set_pf(w, id, pv::SPIN, a);
        manip::set_axis(w, id, id, pv::REC, a, 2);
    }
}

/// The volley's shot (`LAB_0030d8e4`, module doc).
fn fire(w: &mut World, id: MobyId, tpos: [f32; 4]) {
    let t = c::pi32(w, id, pv::TOGGLE) ^ 1;
    c::set_pi32(w, id, pv::TOGGLE, t);
    let pos = w.m(id).position;
    if c::dist2(pos, tpos) < 20.0 {
        let a = c::atan(tpos[0] - pos[0], tpos[1] - pos[1]);
        c::set_pf(w, id, pv::AIM, a);
    }
    let x = w.rng.randf(3.0, 11.0);
    let y = w.rng.randf(-2.0, 2.0);
    let aim = c::pf(w, id, pv::AIM);
    let (s, co) = (aim.sin(), aim.cos());
    let v = [x * co - y * s, x * s + y * co, 0.0, 0.0];
    let step = c::set_len3(v, DT * 7.0);
    let landing = c::add(v, pos);
    let mut best = std::f32::consts::PI;
    let mut muzzle = [0.0, 0.0, 0.0, -1.0];
    for j in 1..4 {
        let p = w.joint_point(id, j);
        let h = c::atan(p[0] - pos[0], p[1] - pos[1]);
        let d = c::diff_rots(h, c::sub_rot(aim, DT * SPIN));
        if d < best {
            best = d;
            muzzle = p;
        }
    }
    let mut time = 0.0;
    let vz = knock::lob_up(DT * 7.0, -(DT2 * 9.8), muzzle, landing, &mut time);
    let muzzle = c::add(muzzle, step);
    let vel = [step[0], step[1], vz, step[3]];
    let n = c::pi16(w, id, pv::SHOTS);
    spawn_shot(w, muzzle, vel, id, n);
    c::set_pi16(w, id, pv::SHOTS, n.wrapping_add(1));
}

/// 0x30ba90(pos, vel, turret, number): a shot 882.
pub fn spawn_shot(w: &mut World, pos: [f32; 4], vel: [f32; 4], turret: MobyId, n: i16) -> Option<MobyId> {
    let s = w.create_moby(SHOT_CLASSES[0])?;
    {
        let m = w.mm(s);
        m.draw_dist = 0xff;
        m.update_dist = 0xff;
        m.visible = 1;
    }
    for k in 0..3 {
        let a = w.rng.rand_angle();
        w.mm(s).rotation[k] = a;
    }
    let r1 = w.rng.randf_sym(100.0, 300.0) * DEG * DT;
    let r2 = w.rng.randf_sym(100.0, 300.0) * DEG * DT;
    w.mm(s).position = pos;
    if w.m(s).pvars.len() >= 0x20 {
        c::set_pv4(w, s, 0, vel);
        c::set_pi32(w, s, 0x10, turret as i32);
        c::set_pi32(w, s, 0x14, n as i32);
        c::set_pf(w, s, 0x18, r1);
        c::set_pf(w, s, 0x1c, r2);
    }
    w.build_matrix(s);
    Some(s)
}

/// Level07 0x30bbc8 (module doc).
pub fn shot_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x20 { return; }
    let old = w.m(id).position;
    let vel = c::pv4(w, id, 0);
    let pos = c::add(old, vel);
    w.mm(id).position = pos;
    c::set_pf(w, id, 8, vel[2] - DT2 * 9.8);
    let (ry, rz) = (c::pf(w, id, 0x18), c::pf(w, id, 0x1c));
    {
        let m = w.mm(id);
        m.rotation[1] = c::add_rot(m.rotation[1], ry);
        m.rotation[2] = c::add_rot(m.rotation[2], rz);
    }
    let owner = usize::try_from(c::pi32(w, id, 0x10)).ok().filter(|&o| o < w.table.mobys.len());
    use crate::moby_update::services::{pf, pv as pv4};
    let hit = w.coll_line(pv4(old), pv4(pos), 0, owner).or_else(|| w.coll_sphere(pv4(pos), pf(0.5), 0, owner));
    let shot_hit = hit.as_ref().is_some_and(|h| h.moby.and_then(|m| w.table.mobys.get(m)).is_some_and(|m| (0x371..=0x372).contains(&m.o_class)));
    if hit.is_none() || shot_hit {
        if pos[..3].iter().any(|&x| !(2.0..=1021.0).contains(&x)) { w.delete_moby(id); }
        return;
    }
    if w.m(id).visible == 0 && 5.0 <= c::dist2(pos, super::hero_pos(w)) {
        w.play_sound(0, 0, id);
        w.delete_moby(id);
        return;
    }
    let n = c::pi32(w, id, 0x14);
    let light = if n % 3 == 0 { 10.0 } else { 0.0 };
    let cam = w.camera_point();
    let odd = n & 1 != 0;
    let (flash2, scale, streaks, puffs, sound) = if 50.0 < c::dist2(pos, [cam[0], cam[1], cam[2], 0.0]) {
        if odd { (0.0, 1.3, 1, 3, -1) } else { (0.0, 1.2, 0, 3, 0) }
    } else if odd {
        (1.0, 1.0, 0, 4, 0)
    } else {
        (0.0, 1.0, 1, 4, 0)
    };
    let b = fx::Beam { damage_r: 2.0, damage: 1.0, flash: 0.0, flash2, flash_dist: 1000.0, scale, light, streaks, sparks: 2, puffs, debris: 1, sound, shake: false };
    fx::beam_explosion(w, &b, Some(id), pos);
    w.delete_moby(id);
}
