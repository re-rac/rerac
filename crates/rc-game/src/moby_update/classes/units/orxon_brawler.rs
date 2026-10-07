//! Orxon's brawler 1202 (level 10, census U337: 57 created of 65 placed; the rest fail the load's spawn test on a
//! fresh save), level10 `0x2e1d38` with its tick `0x2e3ef0`, its flank `0x2e4840`, its group wake `0x2e47d8`, its
//! death sparks `0x2e3a90` and scorch `0x2e3de8`. A melee creature in packs (moby groups): it waits (or sleeps until a
//! path scout 1196, a watched moby's end, the Taunter, the Morph-o-Ray's aim or a target wakes its group), runs at the
//! target spreading round it with its pack (each takes the widest free angle about the target), swings (sequences 6 /
//! 7: two 0.75 spheres at a joint and a line, damage 3.1) and goes home when the target leaves; five health, knocked
//! back and counter-swinging, a pit or a hazard surface under it kills it. Read from the level10 decomp, the
//! disassembly where the decompile hides a stack argument (docs/plan/creatures.md §11).
//!
//! **System or not.** The class code is its own (census cluster a40e51766144, one level); the scorch `0x2e3de8` is the
//! explosive tank's code (cluster 98f51a1cea64, `explosive_tank::scorch` with level10's words); everything else it
//! calls is the creature layer (`creature::*`) or the engine (`BreakFxB`, `PartType02Spawn`, the collision kernels).
//! The flank `0x2e4840` is per class (one caller).
//!
//! **Pvars** (0x2f0): +0x20 the damage record (health 5, +0x24 5, +0x26 s16 the hit cooldown, +0x28 column 3,
//! +0x29 1, +0x30 D+0x10 0.5), +0x38 the lure, +0x60 the flash, +0x70 the knockback record, +0xd0 the walker (J9 =
//! +0xf4: the speed), +0x120 the target record (+0x160 moby, +0x164 kind), +0x170 home (+0x17c: the home yaw), +0x190
//! the turn velocity, +0x194 the flank offset, +0x198 its spread (degrees), +0x1a0 the range now, +0x1a4 the speed,
//! +0x1a8 a timer, +0x1ac the alert, +0x1b0 the area path, +0x1b4 the idle kind, +0x1b8 the wait's maximum, +0x1bc a
//! watched moby, +0x1c0 the last position, +0x1d4 the lost-target count, +0x1d8 stuck, +0x1dc "gone in Clank's
//! part", +0x2b0 the big-head cheat's.
//!
//! ## Coverage: the tick `0x2e3ef0`
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | the hand item is the Morph-o-Ray (0x140408 = 0x15) aiming at it (its moby +0x78 +0x10) → the group wakes (`0x2e47d8`), 1 → 2 | aimed at | [`tick`] (`Reactive::morph`) |
//! | lure +0x38 → the group wakes, +0x1ac = `trunc(scale(randf(180, 240)))`; `FastDecTimer(+0x1ac)` | the Taunter | [`tick`] |
//! | `MobyGetHitMessage(m, 0xb30000, 0)`, `0x26f378(m, hit, +0x20, 0, &out5, &damage, 0, col 4)`, `FastDecTimer_s16(+0x26)` | every weapon's hit record through the resolver | [`tick`] (`World::get_hit`, `damage::resolve`) |
//! | out5 < 2 or dying / falling → no reaction | | [`tick`] |
//! | level 10 (gp−0x7e7c), attacker class 0x31a, alerted, skill point 0x13d419 not earned → the point, level sound 1, banner 0x53d6 | the skill point | ported (`story::award_skill_point`) |
//! | the cooldown running and a type-0 hit: reaction 0xb, damage < 3 → 0 | | [`tick`] |
//! | Ratchet in state 0x20 (or just out of it, `ticks(30)`): damage ≤ 2 | the Comet-Strike's cap | [`tick`] |
//! | its own class's hit; mid-swing with Clank (body 1) the attacker → none | | [`tick`] (the body branch: G-HERO-005) |
//! | the wrench (class 0x47): the line Ratchet + 1 → it + 1 blocked (flags 2) → none | no wrench through walls | [`tick`] (`World::coll_line`) |
//! | another attacker (not a 857): the line from it (+0.25 for 0xba / Ratchet) to it + 1 (flags 0x12) blocked and the face's normal within 60° of the line → none | no shots through walls | [`tick`] |
//! | health −= damage, ≤ 0 → reaction 1; the group wakes; K: radius 768, zoff 0.75, flags 9, +0x3d 0 | | [`tick`] |
//! | reaction 1 / 2: 0xb, untargetable | death | [`tick`] |
//! | 3 / 6–10: 0xa, +0x26 `ticks(60)`, gravity 30·dt², `0x255a40(0.5, 0.5, K)` (the arc), keys 8 / 10, `0x26fa48`, `0x271418(a, m, K, 9, 1, 0)`, flash 0xfa | knockback | [`tick`], [`arc`] (`knock::*`, `flash::start`) |
//! | 4 / 5: the same with the arc (3, 0.5) (Clank: 1), keys 3.5 / 7 | the big knockback | [`tick`] |
//! | +0xa4 0xff, `0x2723f8`; states 10..12 → +0x1d4 0, done | | [`tick`] |
//! | the range: alerted 36, state 1: 6 when Ratchet's capsule touches it (0x13f590 / 0x13f58c) else 0 (Clank: the group wakes), else 30 | | [`tick`] (the capsule contact: G-HERO-033) |
//! | not alerted: the last record within the range (xy) and 3 in z of home → `0x274df8` in the area; else `0x274b78` and none; alerted: `0x274b78`, none beyond it | | [`tick`] (`target::acquire`, `acquire_in`) |
//! | none for `ticks(300)`: far from home → 8 (blend 4), else 1 (blend 0) (Clank: stay); no moby → Ratchet's | give up | [`tick`] |
//!
//! ## Coverage: the update `0x2e1d38`
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | scale = class scale · 0.8; `0x256da0(2.5, m, 2, +0x2b0)` | the big-head cheat manipulator | [`update`] (`manip::big_head`) |
//! | reachable: pos + 1 → target + 1 within 15, in the world box, clear (`CollLine_Fix` flags 2), and within 1 in z unless Ratchet stands on a crate (0x13f64c, `0x273278`) | | [`reachable`] (`crate_::is_crate`) |
//! | case 0: +0x1dc and Clank → deleted; D; +0x58 10, +0x5a 16; mode \| 0x5000; home + yaw; the walker (radius 819, J1 0.8, J2 2, J3 0.5, J9 speed·dt); the watched moby gone / killed and its mission done → −1; 1 (blend 0) | init | [`init`] (`walker::seed`) |
//! | case 1: `SpringTurn2(home yaw)`; the watched moby gone / killed, a target, or +0x1b4 → 2, +0x1a8 `ticks(1)`, the group wakes | sleep | [`update`] |
//! | case 2: idle: a timer `randf(0, 20)`, then blend 2 (+0x1b4 0) or 1; the anim over: Ratchet within 30 (xy) → the chase (blend 3), else 3 (blend 1) | wake | [`update`], [`chase_start`] |
//! | case 3: turn to the record; a target → 4, `randf(0, +0x1b8)` | wait | [`update`] |
//! | case 4: the timer out → the chase | | [`update`] |
//! | case 5: J9 to the speed (8·dt²; within 2 / 3.57: 0.3 of it at 20·dt²; at the swing distance and reachable: 0 at 30·dt²; stuck: 0 at 10·dt²); facing 50° off: J9 ≤ speed·dt/4; `SpringTurn2` unless stuck and within 45°; the flank point `0x2e4840`; Clank: blend only; else the step (`0x26d9a8`) toward it, the line to the target blocked within 3 (not by a 794) → 6 (`ticks(60)`, blend 1); stuck (moved < J9/4); every 7th tick the 7-tick check → 6 or blend 3; home beyond 30 (Clank: 1) or no target → 8 (blend 4); within 2.45 / 4.02 (xy), Ratchet not hit-invulnerable (0x13f510 < `ticks(10)`), facing 20° and reachable → 7 (sequence 6 or 7 by `randi(100) & 1`) | the chase | [`chase`], [`flank`] (`walker::step`) |
//! | case 6: facing; the timer out (or facing 45° off): the line clear → the chase (`ticks(15)`); home / no target → 8; near, reachable → 7; falls into 7 | blocked | [`update`] |
//! | case 7: `0x241840` (= `MobyAnimKeyTime`); turn (0.03); sequences 6 / 7 keys 4..7: the template (push along the yaw, exact, flags 0x10001, type 0 / 1, its class, damage 3.1) at joint 0 (6) / 1 (7): `coll_sphere_mobys(0.75)` there and 0.5 above, `CollLine_Fix(pos + 0.25, joint, 9, m, tmpl)`; wrapped → 8 or the chase | the swing at Ratchet | [`update`] (`World::sphere_mobys`, `services::line_hit_in`) |
//! | case 8: J9 to the speed (5·dt²), turn home, the step; within 1 → 3 (blend 1); a target → the chase | go home | [`update`] |
//! | case 9: J9 3·speed, the step backwards; reachable and the timer out or beyond 5 → 7 | (entered by no code) | [`update`] |
//! | case 10: `0x271558` & 0x41 → 7 (J9 5·dt, sequence 6 / 7); below home − 0.2 → 0xc (blend 0xe), vz / 3 | knocked back → counter | [`update`] |
//! | case 0xb: the scorch (`0x2e3de8`), the sparks twice (`0x2e3a90`), `0x251ef8(1.25, 20, m, pos)` (= `0x2742a8`), `BreakFxB` 0x775, 0x77a–0x77c, two `randf_sym(0, 0.3)` (a point nothing reads), 0x623, 0x624, `SetDeathBits(m, 0, −1)`, deleted | the death: pieces, bolts, save bit | [`explode`] (`explosive_tank::scorch`, `fx::piece_explosion`, `fx::break_piece`, `crate_::set_death_bits`) |
//! | case 0xc: vz −= gravity, pos += K velocity; the ground at or below → the death (0xb's body); else the surface (`CollType`) > 1 and ≠ 3 → a death flight (radius 768, gravity 37.5·dt², the arc (2, 1.5), keys 7 / 13, sequence 0xd, flash 0xfa, `SetDeathBits(m, 0, −1)`), else `SetDeathBits(m, 0x200, −1)`, deleted | falling | [`update`] |
//! | tail (3, 5, 7, 8): above the ground: vz −= gravity (≥ −7·dt), not below; the ground 2 below home → 0xc (blend 0xe) | gravity, pits | [`update`] |
//! | drawn within 18 of the camera: `0x26f020`, +0x7f 0xe | the shadow probe | `shadows::probe_down` |
//! | `0x2e3a90`: 10 clumps from pos jittered 0.5 (`0x2747a0`), 0.55 up (0xb: speeds × 0.55, 0.2 lower), `randf(1, 5)` out and `randf(2, 5)` up per second plus half the knockback velocity, the phase-B velocity falling 20·dt² over `ticks(30)`; 8 type-2 blobs each (r = `randf(0.5, 1.5)`, sizes 0.125·r / 0.065·r, jitter 0.2 / 0.5·dt, colours 0x8000eeee–0x8000ff90 / 0xffee, `ticks(10)`, `ticks(30)`, `randf(5, 25)`) | the death sparks | [`sparks`] (`fx::part02`) |
//! | reaction table level10 0x1dde28 (the shared no-op table) | no Suck Cannon reaction | the Suck Cannon's shot is a weapon hit |
//!
//! Native `f32`; the rand draws at the game's points. the contact words are `Hero::wall_moby` (0x13f590) and `Hero::cap_moby`
//! (0x13f58c); a target moby with no position reads Ratchet's.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::{is_crate, set_death_bits};
use crate::moby_update::classes::units::{class_scale, explosive_tank, hero_pos, orxon_flyers::wake_group_of};
use crate::moby_update::creature::{self as c, damage, flash, fx, ground, knock, target, turn, walker};
use crate::moby_update::services::{HitTemplate, World};
use crate::ps2v::Pf;

pub const UPDATE_FN: u32 = 0x2e_1d38;
pub const REFERENCE_LEVEL: u32 = 10;
pub const CLASSES: [i16; 1] = [1202];

pub mod pv {
    pub const D: usize = 0x20;
    pub const COOLDOWN: usize = 0x26;
    pub const LURE: usize = 0x38;
    pub const FLASH: usize = 0x60;
    pub const K: usize = 0x70;
    pub const J: usize = 0xd0;
    /// J word 9: the walking speed.
    pub const J9: usize = 0xf4;
    pub const TGT: usize = 0x120;
    pub const TGT_MOBY: usize = 0x160;
    pub const TGT_KIND: usize = 0x164;
    pub const HOME: usize = 0x170;
    pub const HOME_YAW: usize = 0x17c;
    pub const TURN_V: usize = 0x190;
    pub const FLANK: usize = 0x194;
    pub const SPREAD: usize = 0x198;
    pub const RANGE: usize = 0x1a0;
    pub const SPEED: usize = 0x1a4;
    pub const TIMER: usize = 0x1a8;
    pub const ALERT: usize = 0x1ac;
    pub const AREA: usize = 0x1b0;
    pub const IDLE_KIND: usize = 0x1b4;
    pub const WAIT_MAX: usize = 0x1b8;
    pub const WATCH: usize = 0x1bc;
    pub const LAST: usize = 0x1c0;
    pub const LOST: usize = 0x1d4;
    pub const STUCK: usize = 0x1d8;
    pub const RATCHET_ONLY: usize = 0x1dc;
    pub const SIZE: usize = 0x2b4;
}

pub mod st {
    pub const INIT: u8 = 0;
    pub const SLEEP: u8 = 1;
    pub const WAKE: u8 = 2;
    pub const WAIT: u8 = 3;
    pub const WAIT_T: u8 = 4;
    pub const CHASE: u8 = 5;
    pub const BLOCKED: u8 = 6;
    pub const SWING: u8 = 7;
    pub const HOME: u8 = 8;
    pub const BACK: u8 = 9;
    pub const KNOCKED: u8 = 10;
    pub const DEAD: u8 = 0xb;
    pub const FALL: u8 = 0xc;
}

/// Level10 `$gp` words (gp = 0x166c00): −0x4ddc 0.8 (the scale, the step-up), −0x4dd8 37.5 (the fall's gravity),
/// −0x4dd4 30 (the knockback's gravity), −0x4dcc 3 (a blocking wall's distance), −0x4dc0 30 (the range), −0x4dbc 5
/// (case 9's reach), −0x4db8 90 (the flank's half angle), −0x4db4 45 (the facing limit); −0x4da8..−0x4d90 the
/// scorch (`explosive_tank::scorch`'s shape), −0x4d8c..−0x4d44 the sparks.
mod k {
    pub const SCALE: f32 = 0.8;
    pub const FALL_G: f32 = 37.5;
    pub const KNOCK_G: f32 = 30.0;
    pub const WALL: f32 = 3.0;
    pub const RANGE: f32 = 30.0;
    pub const BACK_REACH: f32 = 5.0;
    pub const FLANK_DEG: f32 = 90.0;
    pub const FACE_DEG: f32 = 45.0;
    /// The target's class the distances shrink for (857, the gadgebots Clank leads).
    pub const GADGEBOT: i16 = 0x359;
    /// A class a blocking line ignores (794).
    pub const SEE_THROUGH: i16 = 0x31a;
}

/// Level10 0x2e3de8 (gp−0x4da8..−0x4d90): 0x38002028 / 0x2020, sizes 1.5 / 5.7, life 100..180, radius 0.8.
const SCORCH: explosive_tank::Scorch = explosive_tank::Scorch { c: (0x3800_2028, 0x2020), s: (1.5, 5.7), life: (100, 180), r: 0.8 };

const DT: f32 = c::DT;
const DT2: f32 = c::DT2;
const TAU: f32 = std::f32::consts::TAU;
const DEG: f32 = 0.017_453_292;

fn state(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_state(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn seq(w: &World, id: MobyId) -> u8 { w.m(id).anim.seq_b }
fn wrapped(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
fn blend(w: &mut World, id: MobyId, s: u8, n: i32) {
    if seq(w, id) != s {
        let t = w.ticks(n);
        w.anim_blend(id, s, 0, t);
    }
}
fn kind(w: &World, id: MobyId) -> i32 { c::pi32(w, id, pv::TGT_KIND) }
fn clank(w: &World) -> bool { w.hero.mode == 1 }
fn tgt_moby(w: &World, id: MobyId) -> Option<MobyId> { usize::try_from(c::pi32(w, id, pv::TGT_MOBY) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn tgt_pos(w: &World, id: MobyId) -> c::V { tgt_moby(w, id).map_or_else(|| hero_pos(w), |m| w.m(m).position) }
fn tgt_gadgebot(w: &World, id: MobyId) -> bool { tgt_moby(w, id).is_some_and(|m| w.m(m).o_class == k::GADGEBOT) }
fn j9(w: &World, id: MobyId) -> f32 { c::pf(w, id, pv::J9) }
fn set_j9(w: &mut World, id: MobyId, v: f32) { c::set_pf(w, id, pv::J9, v) }
fn approach_j9(w: &mut World, id: MobyId, t: f32, step: f32) {
    let mut x = j9(w, id);
    turn::approach(t, step, &mut x);
    set_j9(w, id, x);
}
fn spring_turn(w: &mut World, id: MobyId, h: f32, acc: f32) {
    turn::spring_turn2_pvar(w, id, h, acc, f32::from_bits(0x3e99_999a), DT * TAU, pv::TURN_V);
}
fn heading_to(w: &World, id: MobyId, t: c::V) -> f32 {
    let p = c::pos(w, id);
    c::atan(t[0] - p[0], t[1] - p[1])
}
fn up(v: c::V, dz: f32) -> c::V { [v[0], v[1], v[2] + dz, v[3]] }

/// The moby at table index `i` is gone: deleted (0xfe / 0xfd) or its death bit is set.
fn watched_gone(w: &World, i: i32) -> Option<bool> {
    let m = w.table.mobys.get(usize::try_from(i).ok()?)?;
    let killed = m.spawn_id >= 0 && w.svc.save.death.contains(&(w.svc.level, m.spawn_id));
    Some(m.state == 0xfe || m.state == 0xfd || killed)
}

/// `0x255a40(d, h, K)`: the arc to land `d` away after rising `h` under the record's gravity: up = √(2·h·g),
/// t = 2·up / g, speed = 2·d / t, drag = 2·d / t².
pub fn arc(w: &mut World, id: MobyId, d: f32, h: f32, kr: usize) {
    let g = c::pf(w, id, kr + knock::k::GRAVITY);
    let u = ((h + h) * g).sqrt();
    c::set_pf(w, id, kr + knock::k::UP, u);
    let t = (u + u) / g;
    c::set_pf(w, id, kr + knock::k::SPEED, (d + d) / t);
    c::set_pf(w, id, kr + knock::k::DRAG, (d + d) / (t * t));
}

/// `0x2e47d8(m)`: the group wakes (members in state 1 → 2).
fn wake(w: &mut World, id: MobyId) { wake_group_of(w, id) }

/// The line `a → b` (flags 2) is blocked within 3 of `a` by the world or a moby that is not a 794.
fn blocked_near(w: &World, id: MobyId, a: c::V, b: c::V) -> bool {
    match w.coll_line(a.map(Pf::f), b.map(Pf::f), 2, Some(id)) {
        Some(h) => {
            let see = h.moby.is_some_and(|m| w.m(m).o_class == k::SEE_THROUGH);
            !see && c::dist3(a, [h.point[0], h.point[1], h.point[2], 0.0]) < k::WALL
        }
        None => false,
    }
}

/// The update's reachability test (module doc).
fn reachable(w: &World, id: MobyId) -> bool {
    let a = up(c::pos(w, id), 1.0);
    let t = tgt_pos(w, id);
    let b = up(t, 1.0);
    let inb = |v: c::V| (0..3).all(|i| (2.0..=1021.0).contains(&v[i]));
    if !(c::dist3(a, b) < 15.0 && inb(a) && inb(b)) { return false; }
    if w.coll_line(a.map(Pf::f), b.map(Pf::f), 2, Some(id)).is_some() { return false; }
    if 1.0 <= (t[2] - w.m(id).position[2]).abs() { return is_crate(w, w.hero.ground_moby); }
    true
}

/// The tick `0x2e3ef0` (module doc).
fn tick(w: &mut World, id: MobyId) {
    let r = &w.hero.weapons.reactive.morph;
    if w.hero.items.slot.id == crate::hero::morph_ray::MORPH && r.target == Some(id) {
        wake(w, id);
        if state(w, id) == st::SLEEP { set_state(w, id, st::WAKE); }
    }
    if c::pi32(w, id, pv::LURE) != 0 {
        wake(w, id);
        let f = w.rng.randf(180.0, 240.0);
        let t = w.svc.timing.scale(Pf::f(f)).to_f32() as i32;
        c::set_pi32(w, id, pv::ALERT, t);
        c::set_pi32(w, id, pv::LURE, 0);
    }
    c::dec_timer_pvar_i32(w, id, pv::ALERT);
    let hit = w.get_hit(id, 0xb3_0000, false);
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    c::dec_timer_pvar_s16(w, id, pv::COOLDOWN);
    let s = state(w, id);
    if let (true, Some(h)) = (2 <= res.out5 && s != st::DEAD && s != st::FALL, res.hit) {
        let mut reaction = res.reaction;
        let mut dmg = res.damage;
        let own = w.m(id).o_class;
        let attacker_class = h.attacker.map(|a| w.m(a).o_class);
        if w.svc.level == 10 && attacker_class == Some(k::SEE_THROUGH) && c::pi32(w, id, pv::ALERT) != 0 {
            crate::moby_update::story::award_skill_point(w, crate::moby_update::story::skill_index(0x13_d419));
        }
        if c::pi16(w, id, pv::COOLDOWN) != 0 && h.b28 == 0 {
            reaction = 0xb;
            if dmg < 3.0 { dmg = 0.0; }
        }
        let comet = w.hero.state == 0x20 || (w.hero.prev_state == 0x20 && w.hero.timer < w.ticks(0x1e));
        if comet && 2.0 < dmg { dmg = 2.0; }
        if h.h2a as i16 == own {
            dmg = 0.0;
            reaction = 0xb;
        }
        if (6..=7).contains(&seq(w, id)) && clank(w) && h.attacker.is_some() && h.attacker == w.hero_moby {
            dmg = 0.0;
            reaction = 0xb;
        }
        if h.h2a == 0x47 {
            let hp = hero_pos(w);
            let a = up(hp, 1.0);
            let b = up(c::pos(w, id), 1.0);
            if w.coll_line(a.map(Pf::f), b.map(Pf::f), 2, Some(id)).is_some() {
                reaction = 0xb;
                dmg = 0.0;
            }
        } else if let Some(att) = h.attacker.filter(|&a| w.m(a).o_class != k::GADGEBOT) {
            let mut a = w.m(att).position;
            let ac = w.m(att).o_class;
            if ac == 0xba || ac == 0 { a[2] += 0.25; }
            let b = up(c::pos(w, id), 1.0);
            if let Some(o) = w.coll_line(a.map(Pf::f), b.map(Pf::f), 0x12, Some(id)) {
                let l = c::set_len3(c::sub(b, a), 1.0);
                let n = c::set_len3([o.normal[0], o.normal[1], o.normal[2], 0.0], 1.0);
                if 0.5 < c::dot3(l, n).abs() {
                    dmg = 0.0;
                    reaction = 0xb;
                }
            }
        }
        let hp = c::pf(w, id, pv::D) - dmg;
        c::set_pf(w, id, pv::D, hp);
        if hp <= 0.0 { reaction = 1; }
        wake(w, id);
        let kr = pv::K;
        c::set_pi32(w, id, kr + knock::k::RADIUS, 768);
        c::set_pf(w, id, kr + knock::k::ZOFF, 0.75);
        c::set_pi32(w, id, kr + knock::k::FLAGS, 9);
        c::set_pu8(w, id, kr + 0x3d, 0);
        let dir = h.dir.map(|x| x.to_f32());
        match reaction {
            1 | 2 => {
                set_state(w, id, st::DEAD);
                w.mm(id).mode &= !mode::TARGETABLE;
            }
            3..=10 => {
                set_state(w, id, st::KNOCKED);
                let t = w.ticks(0x3c);
                c::set_pi16(w, id, pv::COOLDOWN, t as i16);
                c::set_pf(w, id, kr + knock::k::GRAVITY, k::KNOCK_G * DT2);
                let big = matches!(reaction, 4 | 5);
                let d = if !big { 0.5 } else if clank(w) { 1.0 } else { 3.0 };
                arc(w, id, d, 0.5, kr);
                let (apex, land) = if big { (3.5, 7.0) } else { (8.0, 10.0) };
                c::set_pf(w, id, kr + knock::k::KEY_APEX, apex);
                c::set_pf(w, id, kr + knock::k::KEY_LAND, land);
                let (mut sp, mut u) = (c::pf(w, id, kr + knock::k::SPEED), c::pf(w, id, kr + knock::k::UP));
                let a = knock::aim(dir, &mut sp, &mut u);
                c::set_pf(w, id, kr + knock::k::SPEED, sp);
                c::set_pf(w, id, kr + knock::k::UP, u);
                knock::start(w, id, kr, a, 9, 1, 0);
                c::set_pu8(w, id, pv::FLASH + 7, 0xfa);
                flash::start(w, id, pv::FLASH);
            }
            _ => {}
        }
    }
    w.mm(id).hit_slot = 0xff;
    flash::update(w, id, pv::FLASH);
    if (st::KNOCKED..=st::FALL).contains(&state(w, id)) {
        c::set_pi32(w, id, pv::LOST, 0);
        return;
    }
    let alerted = c::pi32(w, id, pv::ALERT) != 0;
    if alerted {
        c::set_pf(w, id, pv::RANGE, k::RANGE + 6.0);
    } else if state(w, id) != st::SLEEP {
        c::set_pf(w, id, pv::RANGE, k::RANGE);
    } else {
        let touch = w.hero.wall_moby == Some(id) || w.hero.cap_moby == Some(id);
        c::set_pf(w, id, pv::RANGE, if touch { 6.0 } else { 0.0 });
        if clank(w) { wake(w, id); }
    }
    let range = c::pf(w, id, pv::RANGE);
    let home = c::pv4(w, id, pv::HOME);
    let near = |w: &World| c::dist2(home, c::pv4(w, id, pv::TGT)) <= range && (w.m(id).position[2] - c::pf(w, id, pv::TGT + 8)).abs() <= 3.0;
    let t = if !alerted {
        if near(w) {
            let area = usize::try_from(c::pi32(w, id, pv::AREA)).ok().filter(|&p| p < w.svc.splines.len());
            let t = target::acquire_in(w, id, range, area);
            store(w, id, &t);
            None
        } else {
            let t = target::acquire(w, id, range);
            store(w, id, &t);
            Some(2)
        }
    } else {
        let t = target::acquire(w, id, range);
        store(w, id, &t);
        (!near(w)).then_some(2)
    };
    if let Some(k2) = t { c::set_pi32(w, id, pv::TGT_KIND, k2); }
    if state(w, id) == st::SLEEP {
        c::set_pi32(w, id, pv::LOST, 0);
    } else if kind(w, id) == 2 {
        let n = c::pi32(w, id, pv::LOST) + 1;
        c::set_pi32(w, id, pv::LOST, n);
        if w.ticks(300) < n {
            if 1.0 < c::dist3(c::pos(w, id), home) {
                set_state(w, id, st::HOME);
                blend(w, id, 4, 0xc);
            } else if !clank(w) {
                set_state(w, id, st::SLEEP);
                blend(w, id, 0, 0x14);
            }
        }
    } else {
        c::set_pi32(w, id, pv::LOST, 0);
    }
    if c::pi32(w, id, pv::TGT_MOBY) == 0 {
        let h = w.hero_moby.map_or(0, |m| m as i32 + 1);
        c::set_pi32(w, id, pv::TGT_MOBY, h);
    }
}

fn store(w: &mut World, id: MobyId, t: &target::Target) {
    c::set_pv4(w, id, pv::TGT, t.pos);
    c::set_pv4(w, id, pv::TGT + 0x10, t.rot);
    c::set_pv4(w, id, pv::TGT + 0x20, t.aim);
    c::set_pv4(w, id, pv::TGT + 0x30, t.body);
    c::set_pi32(w, id, pv::TGT_MOBY, t.moby.map_or(0, |m| m as i32 + 1));
    c::set_pi32(w, id, pv::TGT_KIND, t.kind as i32);
}

/// Case 0 (module doc).
fn init(w: &mut World, id: MobyId) -> bool {
    if c::pi32(w, id, pv::RATCHET_ONLY) != 0 && clank(w) {
        w.delete_moby(id);
        return false;
    }
    c::set_pf(w, id, pv::D + 0x10, 0.5);
    c::set_pi16(w, id, pv::D + 4, 5);
    c::set_pu8(w, id, pv::D + 8, 3);
    c::set_pf(w, id, pv::D, 5.0);
    c::set_pu8(w, id, pv::D + 9, 1);
    c::set_pu8(w, id, 0x58, 10);
    c::set_pu8(w, id, 0x5a, 16);
    w.mm(id).mode |= 0x5000;
    let mut h = c::pos(w, id);
    h[3] = c::yaw(w, id);
    c::set_pv4(w, id, pv::HOME, h);
    walker::seed(&mut w.mm(id).pvars, pv::J);
    c::set_pf(w, id, pv::J + 0xc, 0.5);
    c::set_pf(w, id, pv::J + 8, 2.0);
    c::set_pi32(w, id, pv::J, (k::SCALE * 1024.0) as i32);
    c::set_pf(w, id, pv::J + 4, k::SCALE);
    let sp = c::pf(w, id, pv::SPEED) * DT;
    set_j9(w, id, sp);
    let wi = c::pi32(w, id, pv::WATCH);
    if wi != -1 {
        let gone = match usize::try_from(wi).ok().and_then(|i| w.table.mobys.get(i)) {
            None => true,
            Some(m) => {
                let killed = m.spawn_id >= 0 && w.svc.save.death.contains(&(w.svc.level, m.spawn_id));
                m.state == 0xfe || m.state == 0xfd || (killed && w.mission_done(w.svc.level, m.mission) == 0xff)
            }
        };
        if gone { c::set_pi32(w, id, pv::WATCH, -1); }
    }
    set_state(w, id, st::SLEEP);
    if seq(w, id) != 0 { w.anim_blend(id, 0, 0, 0); }
    c::set_pf(w, id, pv::RANGE, k::RANGE);
    true
}

/// The chase's start (cases 2, 4, 6, 7, 8): +0x1d8 0, +0x1c0 = pos, J9 0 (and the flank offset from case 2 / 4).
fn chase_reset(w: &mut World, id: MobyId) {
    set_state(w, id, st::CHASE);
    c::set_pi32(w, id, pv::STUCK, 0);
    let p = c::pos(w, id);
    c::set_pv4(w, id, pv::LAST, p);
    set_j9(w, id, 0.0);
}

/// Cases 2 / 4: the chase with a new flank offset `randf(spread°, −spread°)` (blend 3).
fn chase_start(w: &mut World, id: MobyId) {
    chase_reset(w, id);
    let s = c::pf(w, id, pv::SPREAD) * DEG;
    let f = w.rng.randf(s, -s);
    c::set_pf(w, id, pv::FLANK, f);
    blend(w, id, 3, 5);
}

/// A swing: sequence 6 or 7 (`randi(100) & 1`), state 7, blend over `ticks(n)`.
fn swing(w: &mut World, id: MobyId, n: i32) {
    let s = if w.rng.randi(100) & 1 != 0 { 6 } else { 7 };
    set_state(w, id, st::SWING);
    let t = w.ticks(n);
    w.anim_blend(id, s, 0, t);
}

/// `0x2e4840`: the heading from the target to stand at: its own when alone; with a group, the middle of the widest
/// free angle (±90°) about the target from its own side, the members charging or nearer the target closing it.
fn flank(w: &World, id: MobyId) -> f32 {
    let t = tgt_pos(w, id);
    let p = c::pos(w, id);
    let a0 = c::atan(p[0] - t[0], p[1] - t[1]);
    let g = w.m(id).group;
    if g < 0 { return a0; }
    let (mut hi, mut lo) = (k::FLANK_DEG * DEG, -(k::FLANK_DEG * DEG));
    let dme = c::dist3(t, p);
    if let Some(Some(list)) = w.svc.groups.lists.get(g as usize) {
        for &e in list {
            let Some(m) = w.table.mobys.get((e & 0x7fff) as usize) else { continue };
            if !(m.state == st::CHASE || c::dist3(m.position, t) < dme) { continue; }
            let d = c::sub_rot(c::atan(m.position[0] - t[0], m.position[1] - t[1]), a0);
            if 0.0 < d && d < hi {
                hi = d;
            } else if d < 0.0 && lo < d {
                lo = d;
            }
        }
    }
    let span = c::diff_rots(hi, lo);
    c::add_rot(c::add_rot(lo, span * 0.5), a0)
}

/// The step toward `yaw` (the yaw set for the step, restored after): `0x26d9a8(1, m, J, 2·(cos, sin), &out)`.
fn step_along(w: &mut World, id: MobyId, yaw: f32) -> u32 {
    let saved = c::yaw(w, id);
    c::set_yaw(w, id, yaw);
    let (cs, sn) = c::cs(yaw);
    let mut out = [0.0; 4];
    let r = walker::step(w, id, pv::J, 1.0, [cs + cs, sn + sn, 0.0, 0.0], &mut out);
    c::set_yaw(w, id, saved);
    r
}

/// The swing distance: 2 against a gadgebot, else 3.57.
fn reach(w: &World, id: MobyId) -> f32 { if tgt_gadgebot(w, id) { 2.0 } else { 3.57 } }
/// The swing range (xy): 2.45 / 4.02.
fn swing_range(w: &World, id: MobyId) -> f32 { if tgt_gadgebot(w, id) { 2.45 } else { 4.02 } }
/// Home too far (30; Clank: 1) or no target.
fn give_up(w: &World, id: MobyId) -> bool {
    let lim = if clank(w) { 1.0 } else { 30.0 };
    lim < c::dist3(c::pos(w, id), c::pv4(w, id, pv::HOME)) || kind(w, id) == 2
}
/// Swinging is allowed: Ratchet not hit-invulnerable for `ticks(10)` more.
fn may_swing(w: &World) -> bool { w.hero.f510 < w.ticks(10) }

/// Case 5 (module doc).
fn chase(w: &mut World, id: MobyId, ok: bool) {
    let p0 = c::pos(w, id);
    let t = tgt_pos(w, id);
    let d = c::dist3(p0, t);
    let sp = c::pf(w, id, pv::SPEED);
    let near = if tgt_gadgebot(w, id) { d <= 2.0 } else { d <= 3.57 };
    if near { approach_j9(w, id, sp * DT * 0.3, DT2 * 20.0) } else { approach_j9(w, id, sp * DT, DT2 * 8.0) }
    if ok && (d - reach(w, id)).abs() < 0.35 {
        approach_j9(w, id, 0.0, DT2 * 30.0);
    } else if c::pi32(w, id, pv::STUCK) != 0 {
        approach_j9(w, id, 0.0, DT2 * 10.0);
    }
    let h = heading_to(w, id, t);
    let diff = c::diff_rots(h, c::yaw(w, id));
    if 0.872_664_6 < diff && sp * DT * 0.25 < j9(w, id) { set_j9(w, id, sp * DT * 0.25); }
    if k::FACE_DEG * DEG < diff || c::pi32(w, id, pv::STUCK) == 0 { spring_turn(w, id, h, f32::from_bits(0x3ca3_d70a)); }
    let a = flank(w, id);
    let r = reach(w, id);
    let fp = [a.cos() * r + t[0], a.sin() * r + t[1], t[2], t[3]];
    if clank(w) {
        let turning = DT * 0.087_266_46 < c::pf(w, id, pv::TURN_V).abs();
        if turning { blend(w, id, 3, 7) } else { blend(w, id, 1, 7) }
    } else {
        let yaw = heading_to(w, id, fp);
        if blocked_near(w, id, up(c::pos(w, id), 1.0), up(t, 1.0)) {
            set_state(w, id, st::BLOCKED);
            let tt = w.ticks(0x3c);
            c::set_pi32(w, id, pv::TIMER, tt);
            blend(w, id, 1, 7);
        }
        let res = step_along(w, id, yaw);
        if res & 3 == 0 {
            c::set_pi32(w, id, pv::STUCK, 0);
        } else {
            let stuck = c::dist2(c::pos(w, id), p0) < j9(w, id) * 0.25;
            c::set_pi32(w, id, pv::STUCK, stuck as i32);
            if w.counter.is_multiple_of(7) && c::pi32(w, id, pv::TIMER) == 0 {
                let last = c::pv4(w, id, pv::LAST);
                if j9(w, id) <= c::dist2(last, c::pos(w, id)) || k::FACE_DEG * DEG < diff {
                    if seq(w, id) == 1 { blend_now(w, id, 3, 7); }
                } else {
                    set_state(w, id, st::BLOCKED);
                    let tt = w.ticks(0x3c);
                    c::set_pi32(w, id, pv::TIMER, tt);
                    blend(w, id, 1, 7);
                }
                let p = c::pos(w, id);
                c::set_pv4(w, id, pv::LAST, p);
            }
        }
        if state(w, id) == st::CHASE && c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 && 0.0 < j9(w, id) && c::pi32(w, id, pv::STUCK) == 0 && seq(w, id) == 1 {
            blend_now(w, id, 3, 7);
        }
    }
    w.mm(id).anim.speed = f32::from_bits(0x3f99_999a);
    if give_up(w, id) {
        set_state(w, id, st::HOME);
        blend_now(w, id, 4, 0xc);
        return;
    }
    if swing_range(w, id) <= c::dist2(c::pos(w, id), t) { return; }
    if !may_swing(w) || 0.349_065_84 <= diff { return; }
    if ok { swing(w, id, 7); }
}

/// `MobyAnimBlend(m, s, 0, ticks(n))` without the sequence guard.
fn blend_now(w: &mut World, id: MobyId, s: u8, n: i32) {
    let t = w.ticks(n);
    w.anim_blend(id, s, 0, t);
}

/// The swing's template (module doc).
fn swing_template(w: &World, id: MobyId) -> HitTemplate {
    let (cs, sn) = c::cs(c::yaw(w, id));
    HitTemplate { dir: [Pf::f(cs), Pf::f(sn), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x1_0001, b18: 0, b19: 1, h1a: w.m(id).o_class as u16, damage: Pf::b(0x4046_6666), w20: 1 }
}

/// Case 7 (and case 6 falling into it).
fn swing_state(w: &mut World, id: MobyId) {
    let key = ground::key_time(w, id);
    let t = tgt_pos(w, id);
    let h = heading_to(w, id, t);
    spring_turn(w, id, h, f32::from_bits(0x3cf5_c28f));
    let s = seq(w, id);
    if !(6..=7).contains(&s) { return; }
    if (4.0..=7.0).contains(&key) {
        let tmpl = swing_template(w, id);
        let mut j = w.joint_point(id, (s != 6) as usize);
        w.sphere_mobys(Pf::f(0.75), j.map(Pf::f), 0, Some(id), Some(&tmpl));
        j[2] += 0.5;
        w.sphere_mobys(Pf::f(0.75), j.map(Pf::f), 0, Some(id), Some(&tmpl));
        let a = up(c::pos(w, id), 0.25);
        crate::moby_update::services::line_hit_in(w.table, w.svc, w.classes, w.coll, a.map(Pf::f), j.map(Pf::f), 9, Some(id), &tmpl);
        return;
    }
    if !wrapped(w, id) { return; }
    if give_up(w, id) {
        set_state(w, id, st::HOME);
        blend_now(w, id, 4, 0xc);
    } else {
        chase_reset(w, id);
        blend(w, id, 3, 7);
    }
}

/// `0x2e3a90`: the death sparks (module doc; `fx::clump_sparks`).
fn sparks(w: &mut World, id: MobyId) {
    let dying = state(w, id) == st::DEAD;
    fx::clump_sparks(w, id, pv::K, 10, dying);
}

/// Case 0xb's body (module doc).
fn explode(w: &mut World, id: MobyId) {
    explosive_tank::scorch(w, id, &SCORCH);
    sparks(w, id);
    sparks(w, id);
    let p = c::pos(w, id);
    fx::piece_explosion(w, 1.25, 20.0, Some(id), p);
    let rot = w.m(id).rotation;
    for class in [0x775, 0x77a, 0x77b, 0x77c] { fx::break_piece(w, id, class, p, rot, 0, 2); }
    // A point 0.55 up jittered by `randf_sym(0, 0.3)` in x and y: computed, never read (the pieces take pos).
    w.rng.randf_sym(0.0, 0.3);
    w.rng.randf_sym(0.0, 0.3);
    for class in [0x623, 0x624] { fx::break_piece(w, id, class, p, rot, 0, 2); }
    set_death_bits(w, id, 0, -1);
    w.delete_moby(id);
}

/// Level10 `0x2e1d38` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { return; }
    tick(w, id);
    let o = w.m(id).o_class;
    w.mm(id).scale = class_scale(w, o) * k::SCALE;
    // 0x256da0(2.5, m, 2, +0x2b0): the big-head cheat (0x15edb7).
    crate::moby_update::manip::big_head(w, 2.5, id, 2, id, 0x2b0);
    let ok = reachable(w, id);
    let t = tgt_pos(w, id);
    match state(w, id) {
        st::INIT => {
            if !init(w, id) { return; }
        }
        st::SLEEP => {
            let hy = c::pf(w, id, pv::HOME_YAW);
            spring_turn(w, id, hy, f32::from_bits(0x3ca3_d70a));
            let wi = c::pi32(w, id, pv::WATCH);
            if wi != -1 && watched_gone(w, wi) == Some(true) {
                set_state(w, id, st::WAKE);
                let tt = w.ticks(1);
                c::set_pi32(w, id, pv::TIMER, tt);
                wake(w, id);
            }
            if kind(w, id) != 2 || c::pi32(w, id, pv::IDLE_KIND) != 0 {
                set_state(w, id, st::WAKE);
                let tt = w.ticks(1);
                c::set_pi32(w, id, pv::TIMER, tt);
                wake(w, id);
            }
        }
        st::WAKE => {
            if seq(w, id) == 0 {
                if c::pi32(w, id, pv::TIMER) == 0 {
                    let f = w.rng.randf(0.0, 20.0);
                    let v = w.svc.timing.scale(Pf::f(f)).to_f32() as i32;
                    c::set_pi32(w, id, pv::TIMER, v);
                } else if c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 {
                    if c::pi32(w, id, pv::IDLE_KIND) == 0 { blend(w, id, 2, 10) } else { blend(w, id, 1, 10) }
                }
            } else if wrapped(w, id) {
                if c::dist2(c::pos(w, id), hero_pos(w)) < k::RANGE {
                    chase_start(w, id);
                } else {
                    set_state(w, id, st::WAIT);
                    blend(w, id, 1, 10);
                }
            }
        }
        st::WAIT => {
            let r = c::pv4(w, id, pv::TGT);
            let h = heading_to(w, id, r);
            spring_turn(w, id, h, f32::from_bits(0x3ca3_d70a));
            if kind(w, id) != 2 {
                set_state(w, id, st::WAIT_T);
                let m = c::pi32(w, id, pv::WAIT_MAX) as f32;
                let f = w.rng.randf(0.0, m);
                c::set_pi32(w, id, pv::TIMER, f as i32);
            }
        }
        st::WAIT_T => {
            if c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 { chase_start(w, id); }
        }
        st::CHASE => chase(w, id, ok),
        st::BLOCKED => {
            let h = heading_to(w, id, t);
            let diff = c::diff_rots(h, c::yaw(w, id));
            let sp = c::pf(w, id, pv::SPEED);
            if 0.872_664_6 < diff && sp * DT * 0.25 < j9(w, id) { set_j9(w, id, sp * DT * 0.25); }
            let done = c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0;
            if (done || k::FACE_DEG * DEG < diff) && !blocked_near(w, id, up(c::pos(w, id), 1.0), up(t, 1.0)) {
                chase_reset(w, id);
                let tt = w.ticks(0xf);
                c::set_pi32(w, id, pv::TIMER, tt);
            }
            if give_up(w, id) {
                set_state(w, id, st::HOME);
                blend_now(w, id, 4, 0xc);
            } else if c::dist2(c::pos(w, id), t) < swing_range(w, id) && may_swing(w) && diff < k::FACE_DEG * DEG && ok {
                swing(w, id, 7);
            }
            swing_state(w, id);
        }
        st::SWING => swing_state(w, id),
        st::HOME => {
            let sp = c::pf(w, id, pv::SPEED);
            approach_j9(w, id, sp * DT, DT2 * 5.0);
            let home = c::pv4(w, id, pv::HOME);
            let h = heading_to(w, id, home);
            spring_turn(w, id, h, f32::from_bits(0x3ca3_d70a));
            let diff = c::diff_rots(h, c::yaw(w, id));
            if 0.872_664_6 < diff && sp * DT * 0.25 < j9(w, id) { set_j9(w, id, sp * DT * 0.25); }
            let y = c::yaw(w, id);
            step_along(w, id, y);
            if c::dist3(c::pos(w, id), home) < 1.0 {
                set_state(w, id, st::WAIT);
                blend(w, id, 1, 0x14);
            } else if kind(w, id) != 2 {
                chase_reset(w, id);
                blend(w, id, 3, 7);
            }
        }
        st::BACK => {
            let sp = c::pf(w, id, pv::SPEED);
            approach_j9(w, id, sp * 3.0 * DT, DT2 * 20.0);
            let h = heading_to(w, id, t);
            spring_turn(w, id, h, f32::from_bits(0x3ca3_d70a));
            step_along(w, id, c::add_rot(h, f32::from_bits(0x4049_0fd0)));
            if ok && (c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 || k::BACK_REACH < c::dist3(c::pos(w, id), t)) {
                c::set_pf(w, id, pv::J9, DT * 5.0);
                swing(w, id, 6);
            }
        }
        st::KNOCKED => {
            let r = knock::update(w, id, pv::K);
            if r & 0x41 != 0 {
                c::set_pf(w, id, pv::J9, DT * 5.0);
                swing(w, id, 6);
            } else if w.m(id).position[2] < c::pf(w, id, pv::HOME + 8) - 0.2 {
                blend(w, id, 0xe, 6);
                set_state(w, id, st::FALL);
                let vz = c::pf(w, id, pv::K + 8);
                c::set_pf(w, id, pv::K + 8, vz / 3.0);
            }
        }
        st::DEAD => {
            explode(w, id);
            return;
        }
        st::FALL => {
            let kr = pv::K;
            let vz = c::pf(w, id, kr + 8) - c::pf(w, id, kr + knock::k::GRAVITY);
            c::set_pf(w, id, kr + 8, vz);
            let p = c::add(c::pos(w, id), c::pv4(w, id, kr));
            c::set_pos(w, id, p);
            let g = ground::ground(w, p, 0.5, 0);
            if g.z <= p[2] {
                explode(w, id);
                return;
            }
            let ty = g.surface as u32;
            if 1 < ty && ty != 3 {
                set_state(w, id, st::DEAD);
                w.mm(id).mode &= !mode::TARGETABLE;
                c::set_pi32(w, id, kr + knock::k::RADIUS, 768);
                c::set_pf(w, id, kr + knock::k::GRAVITY, k::FALL_G * DT2);
                c::set_pu8(w, id, kr + 0x3d, 0);
                c::set_pf(w, id, kr + knock::k::ZOFF, 0.75);
                c::set_pi32(w, id, kr + knock::k::FLAGS, 9);
                arc(w, id, 2.0, 1.5, kr);
                c::set_pf(w, id, kr + knock::k::KEY_LAND, 13.0);
                c::set_pf(w, id, kr + knock::k::KEY_APEX, 7.0);
                let a = c::add_rot(c::yaw(w, id), std::f32::consts::PI);
                knock::start(w, id, kr, a, 0xd, 1, 0);
                c::set_pu8(w, id, pv::FLASH + 7, 0xfa);
                flash::start(w, id, pv::FLASH);
                set_death_bits(w, id, 0, -1);
            } else {
                set_death_bits(w, id, 0x200, -1);
                w.delete_moby(id);
                return;
            }
        }
        _ => {}
    }
    let s = state(w, id);
    if matches!(s, st::WAIT | st::CHASE | st::SWING | st::HOME) {
        let g = ground::ground(w, c::pos(w, id), 0.5, 0).z;
        let z = w.m(id).position[2];
        if g < z {
            let kr = pv::K;
            let mut vz = c::pf(w, id, kr + 8) - c::pf(w, id, kr + knock::k::GRAVITY);
            if vz < DT * -7.0 { vz = DT * -7.0; }
            c::set_pf(w, id, kr + 8, vz);
            let nz = z + vz;
            w.mm(id).position[2] = if nz < g { g } else { nz };
            if g < c::pf(w, id, pv::HOME + 8) - 2.0 {
                blend(w, id, 0xe, 6);
                set_state(w, id, st::FALL);
            }
        }
    }
    let cam = w.camera.map(|x| f32::from_bits(x.0));
    if w.m(id).visible != 0 && c::dist3(c::pos(w, id), cam) < 18.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0xe;
    }
}
