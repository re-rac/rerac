//! Umbris' small level classes (level 07; read from the level07 decomp and disassembly; the names are descriptive [L]):
//!
//! * **529, the parked ship** (`0x2fbdb8`, 3 placed; census U258): update distance 8, its class word +0xa6 set to
//!   0x215 (533: only the word, nothing else of the class swap, `class_swap`'s scan missed it [L]) and the ships'
//!   canopy glass (`RegisterDrawCallback(0x2ba260)` = L01 `0x2a70a8`) every tick.
//! * **1789, the scene jets** (`0x31f900`, 1 placed; U284): in scenes 1 / 2, actor 3 / 4 gets three jet glows at its
//!   joint lists 0..2, moving 0.1 away from lists 3..5 (`0x28c408` = L01 `0x278810`: 150000, 0).
//! * **1552, the scene FX driver** (`0x31eba8`, 1 placed; U280): scenes 3 / 4: the infobot's thrusters on actor 2
//!   (`0x28c020` = L01 `0x278450`); scene 2 at tick `ticks(2698)` (actor 7) or `ticks(2766)` (actor 6): ten dust puffs at
//!   each of its joint lists 0..3 (type 23: 0.6, 1 → 1.01, `randf(20000, 200000)`, still, white at
//!   `rand_range(0x20, 0x80)` alpha, spin `±rand_range(0, 4)`, life `ticks(rand_range(60, 90))`, fading).
//! * **1133, the swinging part** (`0x31b3f0`, 1 placed; U278): waits for its link (+0x00) to be gone or in state A
//!   (+0x04), then swings in pitch, +0x44 (a kick of +0x0c·dt, then `v += cos(pitch)·+0x0c·dt`, `v ·= +0x10`) with sound 0,
//!   until the link is in state B (+0x08); then it swings back by +0x14 a tick to 0 and waits again (the link gone or
//!   in A meanwhile: sound 0 and it stops for good).
//! * **1142, the ammo drop** (`0x31d2e0`, 1 placed; U279): its path's segment lengths into the points' w and their
//!   sum (+0x20); when Ratchet enters its cuboid (+0x00), for each weapon of the list gp−0x4f60 (items 10, 15, 20)
//!   whose ammo fraction lies in [+0x08, +0x0c) %: an amount (+0x10 % of the max less the ammo, against +0x14 % of
//!   the max: the larger with +0x18, else the smaller), in pickups of amount / +0x1c, dropped along the path from a
//!   random distance on (`SplineProject`, 10 farther while within 12 of the moby +0x24), each the next a little on;
//!   then done.
//! * **1113–1116, the beast's walls** (`0x319f48`, 4 placed; U274): a hit (mask 0x30000) of exactly 1.000123 damage
//!   from the Snagglebeast (0x452 in state 9) or 0x415 in state 8 breaks it: +0x00 debris bits (classes 696–698,
//!   `SpawnDebrisMoby`) from a box (+0x04..+0x0c, turned with it) flying `rand_vec(5·dt, 10·dt)` upward, a beam
//!   explosion (3, 1, 4, 2, 9, 1, 15; 10 streaks, 3 sparks, 16 puffs, sound 0, a shake, 1 debris) at the hit
//!   record's +0x00 (a sphere hit's is (0, 0, 0, 1): the world origin, the game's [L]), deleted.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2fbdb8` | +0x30 = 8, +0xa6 = 0x215, `RegisterDrawCallback(0x2ba260, m)` | [`ship_update`] (`Callback::ShipGlass`) |
//! | `0x31f900` | 0 → 1 (+0x30 0xff); 1: game mode 2, scene (0x16c910) 1 / 2 → actor 3 / 4 (0x16ca58), the three pairs | [`jets_update`] (`fx::jet_puffs`) |
//! | `0x31eba8` | 0 → 1 (+0x30 0xff); 1: game mode 2: scene 3 / 4 → actor 2's thrusters; scene 2 at its tick (0x16c914) | [`scene_fx_update`] (`cutscene_fx::infobot_thrusters`) |
//! | `0x31b3f0` | the states 0..4 (above) | [`swing_update`] |
//! | `0x31d2e0` | 0: no cuboid or path → deleted; the lengths; 1: the drop (printfs n/a); → 2 | [`ammo_drop_update`] (`spline::advance`, `pickup::pickup_spawn`) |
//! | `0x319f48` | the hit test, the debris and the explosion; the hit slot cleared otherwise | [`wall_update`] (`gunship::spawn_ember`, `fx::beam_explosion`) |

use crate::moby_runtime::MobyId;
use crate::moby_update::classes::cutscene_fx::infobot_thrusters;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, fx, DT};
use crate::moby_update::services::World;

pub const REFERENCE_LEVEL: u32 = 7;
pub const SHIP_FN: u32 = 0x2f_bdb8;
pub const SHIP_CLASSES: [i16; 1] = [529];
pub const JETS_FN: u32 = 0x31_f900;
pub const JETS_CLASSES: [i16; 1] = [1789];
pub const SCENE_FX_FN: u32 = 0x31_eba8;
pub const SCENE_FX_CLASSES: [i16; 1] = [1552];
pub const SWING_FN: u32 = 0x31_b3f0;
pub const SWING_CLASSES: [i16; 1] = [1133];
pub const AMMO_FN: u32 = 0x31_d2e0;
pub const AMMO_CLASSES: [i16; 1] = [1142];
pub const WALL_FN: u32 = 0x31_9f48;
pub const WALL_CLASSES: [i16; 4] = [1113, 1114, 1115, 1116];

/// Level07 `0x2fbdb8` (module doc).
pub fn ship_update(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.update_dist = 8;
    m.o_class = 0x215;
    w.svc.draw_callbacks.register(Callback::ShipGlass, id);
}

fn scene_start(w: &mut World, id: MobyId) -> bool {
    if w.m(id).state == 0 {
        let m = w.mm(id);
        m.state = 1;
        m.update_dist = 0xff;
        return false;
    }
    w.m(id).state == 1 && w.svc.game_mode == 2
}

/// Level07 `0x31f900` (module doc).
pub fn jets_update(w: &mut World, id: MobyId) {
    if !scene_start(w, id) { return; }
    let Some(scene) = w.svc.cinematic.scene.clone() else { return };
    let actor = match scene.id {
        1 => 3,
        2 => 4,
        _ => return,
    };
    let Some(a) = scene.actors.get(actor) else { return };
    for k in 0..3 {
        let (p, q) = (a.joint_point(k), a.joint_point(k + 3));
        let d = c::set_len3(c::sub(p, q), 0.1);
        fx::jet_puffs(w, 150_000.0, 0.0, p, p, d);
    }
}

/// Level07 `0x31eba8` (module doc).
pub fn scene_fx_update(w: &mut World, id: MobyId) {
    if !scene_start(w, id) { return; }
    let Some(scene) = w.svc.cinematic.scene.clone() else { return };
    if scene.id == 3 || scene.id == 4 {
        if let Some(a) = scene.actors.get(2) { infobot_thrusters(w, a); }
    }
    if scene.id != 2 { return; }
    let actor = if scene.tick == w.ticks(0xace) {
        6
    } else if scene.tick == w.ticks(0xa8a) {
        7
    } else {
        return;
    };
    let Some(a) = scene.actors.get(actor) else { return };
    for k in 0..4 {
        let p = a.joint_point(k);
        for _ in 0..10 {
            let size = w.rng.randf(20000.0, 200_000.0);
            let alpha = w.rng.rand_range(0x20, 0x80) as u8;
            let s = w.rng.rand_range(0, 4);
            let spin = if w.rng.randi(2) == 0 { s } else { -s };
            dust(w, size, p, spin, alpha);
        }
    }
}

/// `PartType23Spawn(0.6, 1, 1.01, size, p, spin, 0, alpha << 24 | 0x7f7f7f)`, then its timer `ticks(rand_range(60,
/// 90))`, phase 2 fading from `alpha` over the timer's low byte.
fn dust(w: &mut World, size: f32, p: c::V, spin: i32, alpha: u8) {
    *w.svc.fx.part_spawns.entry(23).or_default() += 1;
    let Some(sys) = w.particles.as_deref_mut() else { return };
    let rgba = (alpha as u32) << 24 | 0x7f_7f7f;
    let Some(i) = crate::particles::type23::spawn(sys, w.rng, 0.6, 1.0, f32::from_bits(0x3f81_47ae), size, p, spin, [0.0; 4], rgba) else {
        w.svc.fx.part_failed += 1;
        return;
    };
    let n = w.rng.rand_range(0x3c, 0x5a);
    let life = w.ticks(n);
    let r = &mut w.particles.as_deref_mut().unwrap().pool.recs[i];
    use crate::particles::rec;
    rec::set_i16(r, 0xa, life as i16);
    rec::set_u32(r, 0x24, 2);
    r[0x2a] = alpha;
    r[0x2b] = r[0xa];
}

/// The swinging part's link (+0x00): None when it is −1 or deleted, else its state.
fn link_state(w: &World, id: MobyId) -> Option<u32> {
    let l = usize::try_from(c::pi32(w, id, 0)).ok()?;
    let s = w.table.mobys.get(l)?.state;
    (s != 0xfe && s != 0xfd).then_some(s as u32)
}

/// Level07 `0x31b3f0` (module doc).
pub fn swing_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x1c { return; }
    let (a, b) = (c::pi32(w, id, 4) as u32, c::pi32(w, id, 8) as u32);
    let speed = c::pf(w, id, 0xc);
    match w.m(id).state {
        0 => {
            w.mm(id).rotation[1] = 0.0;
            w.mm(id).state = 1;
            c::set_pf(w, id, 0x18, 0.0);
        }
        1 => {
            if link_state(w, id).is_some_and(|s| s != a) { return; }
            let v = speed * DT;
            c::set_pf(w, id, 0x18, v);
            w.mm(id).rotation[1] = v;
            w.play_sound(0, 0, id);
            w.mm(id).state = 3;
        }
        3 => {
            let pitch = w.m(id).rotation[1];
            let v = c::add_rot(c::pf(w, id, 0x18), pitch.cos() * speed * DT);
            w.mm(id).rotation[1] = c::add_rot(pitch, v);
            c::set_pf(w, id, 0x18, v * c::pf(w, id, 0x10));
            if link_state(w, id) == Some(b) { w.mm(id).state = 4; }
        }
        4 => {
            if link_state(w, id).is_none_or(|s| s == a) {
                w.play_sound(0, 0, id);
                w.mm(id).state = 2;
                return;
            }
            let p = c::sub_rot(w.m(id).rotation[1], c::pf(w, id, 0x14));
            w.mm(id).rotation[1] = p;
            if 0.0 < p {
                w.mm(id).rotation[1] = 0.0;
                w.mm(id).state = 1;
                c::set_pf(w, id, 0x18, 0.0);
            }
        }
        _ => {}
    }
}

/// gp−0x4f60: the weapons the ammo drop serves (−1 ends the list).
const AMMO_ITEMS: [usize; 3] = [10, 15, 20];

/// Level07 `0x31d2e0` (module doc).
pub fn ammo_drop_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x28 { return; }
    let path = c::pi32(w, id, 4);
    match w.m(id).state {
        0 => {
            if c::pi32(w, id, 0) == -1 || path == -1 {
                w.delete_moby(id);
                return;
            }
            let Some(pts) = w.svc.splines.get_mut(path as usize) else { return };
            let n = pts.len();
            let mut total = 0.0f32;
            for i in 0..n {
                let (p, q) = (pts[i].map(f32::from_bits), pts[(i + 1) % n].map(f32::from_bits));
                let l = c::dist3(p, q);
                pts[i][3] = l.to_bits();
                total += l;
            }
            c::set_pf(w, id, 0x20, total);
            w.mm(id).state = 1;
        }
        1 => {
            let hero = super::hero_pos(w);
            if !w.in_cuboid([hero[0], hero[1], hero[2]], c::pi32(w, id, 0)) { return; }
            let pts: Vec<[f32; 4]> = w.svc.splines.get(path as usize).map(|v| v.iter().map(|p| p.map(f32::from_bits)).collect()).unwrap_or_default();
            let total = c::pf(w, id, 0x20);
            let pcts: [f32; 4] = [8, 0xc, 0x10, 0x14].map(|k| c::pi32(w, id, k) as f32 / 100.0);
            let pct = |k: usize| pcts[(k - 8) / 4];
            for item in AMMO_ITEMS {
                let ammo = w.inventory.ammo(item) as f32;
                let max = w.inventory.max_ammo(item) as f32;
                let frac = ammo / max;
                let mut r = w.rng.randf(0.0, total);
                if !(pct(8) <= frac && frac < pct(0xc)) { continue; }
                let want = max * pct(0x14);
                let need = max * pct(0x10) - ammo;
                let larger = c::pi32(w, id, 0x18) != 0;
                let keep = if larger { want < need } else { need < want };
                let amount = if keep { need } else { want };
                let all = (amount + 0.5) as i32;
                let per_n = c::pi32(w, id, 0x1c);
                if per_n == 0 { continue; }
                let mut per = if all / per_n != 0 { all / per_n } else { all };
                if all <= 0 { continue; }
                let mut left = all - per;
                let mut cur = crate::spline::Cursor::default();
                let away = index(w, c::pi32(w, id, 0x24));
                loop {
                    let p = loop {
                        let (p, _) = crate::spline::advance(&pts, true, r, &mut cur);
                        let far = away.is_none_or(|m| 12.0 < c::dist2(w.m(m).position, [p[0], p[1], p[2], 0.0]));
                        if far { break p; }
                        r += 10.0;
                    };
                    crate::moby_update::classes::pickup::pickup_spawn(w, crate::moby_update::services::pv([p[0], p[1], p[2], 0.0]), item as i32, per, 0);
                    r += w.rng.randf(total / 10.0, 2.0);
                    if left < per { per = left; }
                    let more = 0 < left;
                    left -= per;
                    if !more { break; }
                }
            }
            w.mm(id).state = 2;
        }
        _ => {}
    }
}

fn index(w: &World, i: i32) -> Option<MobyId> { usize::try_from(i).ok().filter(|&m| m < w.table.mobys.len()) }

/// The magic damage of the Snagglebeast's charge (1.000123).
const CHARGE_DAMAGE: f32 = 1.000_123;

/// Level07 `0x319f48` (module doc).
pub fn wall_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x10 { return; }
    if let Some(h) = w.get_hit(id, 0x3_0000, false) {
        let by = h.attacker.filter(|&a| a < w.table.mobys.len()).map(|a| (w.m(a).o_class, w.m(a).state));
        let charge = matches!(by, Some((0x415, 8)) | Some((0x452, 9)));
        if charge && f32::from_bits(h.damage.0) == CHARGE_DAMAGE {
            let ext = [c::pf(w, id, 4), c::pf(w, id, 8), c::pf(w, id, 0xc), 0.0];
            let r = w.m(id).rows;
            let e: c::V = std::array::from_fn(|l| if l == 3 { 0.0 } else { r[0][l] * ext[0] + r[1][l] * ext[1] + r[2][l] * ext[2] });
            let n = c::pi32(w, id, 0);
            for i in 0..n.max(0) {
                let x = w.rng.randf(-e[0], e[0]) * 0.5;
                let y = w.rng.randf(-e[1], e[1]) * 0.5;
                let z = w.rng.randf(1.5, e[2]);
                let p = c::add([x, y, z, 0.0], c::pos(w, id));
                let v3 = w.rng.rand_vec(DT * 5.0, DT * 10.0);
                let mut v = [v3[0], v3[1], v3[2], 0.0];
                if v[2] < 0.0 { v[2] = -v[2]; }
                if v[2] < DT + DT { v[2] += DT + DT; }
                let s = w.rng.randf(f32::from_bits(0x3ed1_eb85), 0.5);
                let life = w.rng.rand_range(0x32, 0x50);
                crate::moby_update::classes::gunship::spawn_ember(w, s, 1.0, 1.0, 0.75, p, v, (0x2b8 + i % 3) as i16, life, 0);
            }
            let at = h.pos.map(|x| f32::from_bits(x.0));
            let b = fx::Beam { damage_r: 3.0, damage: 1.0, flash: 4.0, flash2: 2.0, flash_dist: 9.0, scale: 1.0, light: 15.0, streaks: 10, sparks: 3, puffs: 0x10, debris: 1, sound: 0, shake: true };
            fx::beam_explosion(w, &b, Some(id), at);
            w.delete_moby(id);
            return;
        }
    }
    w.mm(id).hit_slot = 0xff;
}
