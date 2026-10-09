//! U301 (census 2026-09-29): class 193, the pack biters of Gaspar and Quartu (level09 `0x2e27d8`: 49 created,
//! level15 `0x2bc608`: 44, the same code). A creature of the shared layer (mode 0x20 header: damage record +0x20,
//! flash +0x110, knockback +0x120, suck record +0x60, walker +0x180) that grazes near its home, turns to face a target
//! (Ratchet or a decoy within its range: 12, 20 while alerted), charges at it with a swerve, bites (the key 13 of
//! sequence 1: a hit on the target for 1), goes home when it lost the target or got stuck, is knocked back by a hit
//! and dies on the second (health 1; a knock flight, then the death explosion and `SetDeathBits`' bolts). The pack is
//! its moby group: a bite, a hit or a sighting sets every member's +0xbc (`0x278d00` = `0x26e090`), which makes the
//! others join the charge. On Quartu a path (+0x21c) keeps them to walkways: they head for its next waypoint toward the
//! target ([`crate::path::toward`]). Ground surface 1 under a creature standing on it kills it (the death explosion).
//! The Suck Cannon takes it (the table [`react::HELD7`], held state 7).
//!
//! **Pvars** (0x270): +0x20 the damage record (+0x20 health, +0x24 s16 the morph meter, +0x26 s16 the class's hit
//! cooldown, +0x29 its column), +0x38 the lure, +0x110 the flash, +0x120 the knockback record, +0x180 the walker,
//! +0x190 its speed, +0x198 the vertical speed, +0x1d0 home, +0x1f0 the turn velocity, +0x1f4 the swerve (radians),
//! +0x1f8 the swerve (degrees, instance data 30), +0x1fc the range, +0x204 the speed (4), +0x208 (s32, counted down,
//! read by nothing here), +0x210 the swerve timer, +0x214 (s16) the alert, +0x216 (s16) the leash radius, +0x218 the
//! leash moby (state 0xd), +0x21c the path (−1 none), +0x224 the stuck timer, +0x228 "only for Giant Clank", +0x230
//! the big-head cheat's.
//!
//! ## Coverage (level09 `0x2e27d8`; the tick is inline)
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | states ∉ {99, 7}: `MobyGetHitMessage(m, 0x330000, 0)` (0x26f320), `0x26f378(m, hit, +0x20, 0, …, col 4)` | every weapon's hit record through the resolver (damage tables, cooldown, the wrench's push) | [`pre`] (`World::get_hit`, `damage::resolve`) |
//! | `FastDecTimer_s16(+0x26)`; a hit while +0x26 < `ticks(45)`: an attacker of class 0xb0 / 0xb1 deals 1 (cooldown out) or 0 | the class's own cooldown | [`pre`] |
//! | group ≠ −1 → `0x278d00(group, 1)` | the pack joins in | [`pre`] (`attack::group_command`) |
//! | damage ≠ 0: health −= damage; K: flags 9, gravity 0.008, drag 0.0005, speeds 3·dt / 7·dt (gp−0x5334 / −0x5338), +0x15d = 0 | | [`pre`] |
//! | no health: untargetable, 8·dt / 10·dt (×1.35 for a hit of class 0xb4), `0x271418(atan(pos − target), m, K, 5, 1, 0)`, keys 11 / 18, `SetDeathBits(m, 0x200 on Giant Clank (body 2) else 0, −1)`, +0x94 = 0, state 99, flash 0x78 | the death flight, the bolts and the save bit | [`pre`] (`knock::start`, `crate_::set_death_bits`); the direction reads a stale stack target in the game: the port's is Ratchet [L] |
//! | else: (×1.35), `0x271418(…, 6, 1, 0)`, keys 5 / 10, state 6, flash 0xfa, +0x26 = `ticks(60)` | the knockback | [`pre`] |
//! | `0x27cf10` flash start; +0xa4 = 0xff | | [`pre`] (`flash::start`) |
//! | `0x282a90(2.1, m, 0, +0x230)` | the big-head cheat manipulator | [`update`] (`manip::big_head`) |
//! | drawn and within 38 of the camera: `0x26f020`, +0x7f = 0x1e | the shadow probe | [`pre`] (`shadows::probe_down`) |
//! | farther than 40 (xy) from Ratchet → nothing more this tick | | [`update`] |
//! | +0x228: body ≠ 2 → hidden (+0x30 0xff, +0x94 0, mode \| 0x41, untargetable), done; body 2 → shown (+0x30 0x40, the class's collision, targetable) | the Giant Clank-sized pack (Quartu) | [`pre`] (reads `Hero::mode`: never 2 until G-HERO-005's body switch) |
//! | lure +0x38 → +0x214 = `ticks(240)` | the Taunter | [`pre`] |
//! | `FastDecTimer(+0x208)`; +0x214 counting → range 20, else 12; `0x27f770(range, m, &t)` | the target search (decoys win) | [`pre`] (`target::acquire`) |
//! | `FastDecTimer(+0x224)` | the stuck timer | [`update`] |
//! | case 0: home, state 1, D fields (column 0, health 1, meter 1), +0x58 8 / +0x5a 2, `SeedJumpPattern(+0x180)`, radius 0x38d, probes 2 / 2, top speed +0x204·dt, `rand() & 1` → mirror, +0xd0 = gp−0x5328, blend 2 (`ticks(rand_range(10, 13))`), z = `GroundHeight(0.5)` | init | [`init`] (+0xd0: `react::SEQS_193`) |
//! | case 1: a target (kind < 2, or none within range + 4) within 8 in z → one in 19: state 2, blend 4; +0xbc = 1 → one in 19: the charge | graze | [`graze`] |
//! | cases 2 / 8 / 10 / 11: `SpringTurn2(atan(target − pos), 0.02, 0.3, 0.1)`; within range / 8 in z → one in 19: group call, the charge; +0xbc = 1 → one in 19: the charge; sequence 4 wrapped → blend 2 | face the target | [`face`] |
//! | the charge: `randi(256) & 1` flips +0x1f8, +0x1f4 = +0x1f8·π/180, +0x210 = `ticks(randf(30, 90))`, state 3, blend 0, +0xbc = 0 | | [`charge_start`] |
//! | case 3: aim = target or `0x294cb0(path, target)`; `SpringTurn2(atan + swerve, 0.05, 0.3, 0.2)`; `0x278618(1, m, J, pos + 2·dir)`; swerve timer → flip; stuck (bit 4), or within 1.5 unblocked → bite 4; slow (< dt), steep (dz / dxy > 1) or blocked → +0x224 = `ticks(50)`, home 5; far from home (range + 4) or 8 in z → home unless +0xbc; else group call | the charge | [`charge`] (`walker::step`, `path::toward`) |
//! | case 4: `SpringTurn2(…, 0.05, 0.3, 0.2)`; not blending and key 13: within 0.5 in z, 2 in xy, facing 15°: `0x2796c0(1, target, m, 1, target + 0.75 z, 0.2·dir)` (= `0x26eaa8`); wrapped and farther than 1.5 → 3; gravity 9.8·dt² to the ground | the bite on Ratchet (his moby's hit record: the hero's intake) | [`bite`] (`attack::hit_moby`) |
//! | case 5: aim = home or `0x294cb0(path, home)`; turn (0.02, 0.3, 0.1), step; within 2 of home → 1 (blend 2); stuck timer out: target near home (range) → group call, 3; +0xbc → 3; blocked → back to the old xy, 0xc (blend 4) | go home | [`home`] |
//! | case 6: `0x27c150(m, K)` landed → 9 (blend 3), done; below z 0 → deleted | knockback | [`update`] (`knock::update`) |
//! | case 7: `0x2fdea0(m, K)` (= `0x305260`) done → 1, blend 2, record state 0 | the Suck Cannon's carried update | [`update`] (`react::carried`) |
//! | case 9: key time > 29 → 3 (blend 0), done | back up | [`update`] |
//! | case 0xc: as 3 toward home; stuck or near and not steep → bite; steep: blocked → the stuck timer (out: 1, else the wrap blend: `randi(4)` twice, 2 or 4); unblocked, not steep → 5 | stuck | [`stuck`] |
//! | case 0xd: blend 0; walk `+0x204·dt` ahead, sliding round Ratchet's and class 0xc1's mobys within 2 (turn π/2·dt); gravity; within +0x216 of the leash moby +0x218 → stay, else 5 | (entered by no code here) | [`leashed`] |
//! | case 99: `0x27c150` & 0x40 → death explosion, deleted; below z 0 → deleted | the death | [`update`] (`fx::death_explosion(0.5, 13, sound 6)`) |
//! | tail: ground (`GroundHeight(0.5)` unless the state probed it); standing (< 0.1) on surface 1 → the death explosion, deleted; else `0x27cff0` flash update | the deadly floor | [`update`] (`ground::ground`, `flash::update`) |
//! | table (level09 0x209b00): 0x2e2680 / 0x2e26d0 / 0x2e2720 / 0x2e2770 (held 7, refused → 1), 0x2e27a0 (+0x60), `DeleteMoby` | the Suck Cannon | `react::slot_*` with [`react::HELD7`] and the class's sequence table |
//!
//! **Not the game's, noted [L]:** the hit's knock direction reads the target record from a stack slot this tick has not
//! written yet (whatever the previous update left there); the port uses Ratchet's position. The hit's rewritten
//! damage is kept local (the game writes it into the log record, which nothing reads after the slot is cleared).
//!
//! Native `f32`; the rand draws at the game's points.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::creature::{self as c, attack, damage, flash, fx, ground, knock, react, target, turn, walker};
use crate::moby_update::services::World;

pub const UPDATE_FN: u32 = 0x2e_27d8;
pub const REFERENCE_LEVEL: u32 = 9;
pub const CLASSES: [i16; 1] = [193];

pub mod pv {
    pub const D: usize = 0x20;
    pub const COOLDOWN: usize = 0x26;
    pub const LURE: usize = 0x38;
    pub const SUCK: usize = 0x60;
    pub const FLASH: usize = 0x110;
    pub const K: usize = 0x120;
    pub const J: usize = 0x180;
    pub const SPEED_NOW: usize = 0x190;
    pub const VZ: usize = 0x198;
    pub const HOME: usize = 0x1d0;
    pub const TURN_V: usize = 0x1f0;
    pub const SWERVE: usize = 0x1f4;
    pub const SWERVE_DEG: usize = 0x1f8;
    pub const RANGE: usize = 0x1fc;
    pub const SPEED: usize = 0x204;
    pub const T208: usize = 0x208;
    pub const SWERVE_T: usize = 0x210;
    pub const ALERT: usize = 0x214;
    pub const LEASH_R: usize = 0x216;
    pub const LEASH: usize = 0x218;
    pub const PATH: usize = 0x21c;
    pub const STUCK_T: usize = 0x224;
    pub const GIANT_ONLY: usize = 0x228;
    pub const SIZE: usize = 0x234;
}

pub mod st {
    pub const INIT: u8 = 0;
    pub const GRAZE: u8 = 1;
    pub const FACE: u8 = 2;
    pub const CHARGE: u8 = 3;
    pub const BITE: u8 = 4;
    pub const HOME: u8 = 5;
    pub const KNOCKED: u8 = 6;
    pub const HELD: u8 = 7;
    pub const RECOVER: u8 = 9;
    pub const STUCK: u8 = 0xc;
    pub const LEASHED: u8 = 0xd;
    pub const DYING: u8 = 99;
}

/// Level09 gp−0x5338 / −0x5334 (0x1618c8 / 0x1618cc): the knockback's up and out speeds (per second).
const KNOCK_UP: f32 = 7.0;
const KNOCK_OUT: f32 = 3.0;

fn state(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_state(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn seq(w: &World, id: MobyId) -> u8 { w.m(id).anim.seq_b }
fn wrapped(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
/// `MobyAnimBlend(m, s, 0, ticks(rand_range(10, 13)))` behind `if (m+0x53 != s)`.
fn blend_r(w: &mut World, id: MobyId, s: u8) {
    if seq(w, id) != s {
        let n = w.rng.rand_range(10, 0xd);
        let t = w.ticks(n);
        w.anim_blend(id, s, 0, t);
    }
}
fn range(w: &World, id: MobyId) -> f32 { c::pf(w, id, pv::RANGE) }
fn group_call(w: &mut World, id: MobyId) {
    let g = w.m(id).group;
    if g != -1 { attack::group_command(w, g, 1); }
}
fn path(w: &World, id: MobyId) -> Option<usize> { usize::try_from(c::pi32(w, id, pv::PATH)).ok().filter(|&p| p < w.svc.splines.len()) }
/// The point to head for on the way to `to` (`0x294cb0` when the creature has a path).
fn aim(w: &World, id: MobyId, to: c::V) -> c::V {
    match path(w, id) {
        Some(p) => crate::path::toward(&w.svc.splines[p], c::pos(w, id), to),
        None => to,
    }
}
/// `0x278618(1, m, J, pos + 2·(cos yaw, sin yaw, 0), &out)`.
fn step(w: &mut World, id: MobyId) -> u32 {
    let (co, si) = c::cs(c::yaw(w, id));
    let p = c::pos(w, id);
    let to = [p[0] + co + co, p[1] + si + si, p[2], p[3]];
    let mut out = [0.0; 4];
    walker::step(w, id, pv::J, 1.0, to, &mut out)
}
fn turn_to(w: &mut World, id: MobyId, to: c::V, extra: f32, acc: f32, max: f32) {
    let p = c::pos(w, id);
    let h = c::atan(to[0] - p[0], to[1] - p[1]) + extra;
    turn::spring_turn2_pvar(w, id, h, acc, 0.3, max, pv::TURN_V);
}
/// The gravity of cases 4 / 0xd: `vz −= 9.8·dt²`, z += vz, not below `GroundHeight(0.5)`. Returns the probe.
fn fall(w: &mut World, id: MobyId) -> ground::Ground {
    let g = ground::ground(w, c::pos(w, id), 0.5, 0);
    let vz = c::pf(w, id, pv::VZ) - c::DT2 * 9.8;
    c::set_pf(w, id, pv::VZ, vz);
    let z = w.m(id).position[2] + vz;
    w.mm(id).position[2] = z;
    if z < g.z {
        c::set_pf(w, id, pv::VZ, 0.0);
        w.mm(id).position[2] = g.z;
    }
    g
}

/// The inline tick (states ∉ {99, 7}; module doc). None: nothing more this tick.
fn pre(w: &mut World, id: MobyId) -> Option<target::Target> {
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    c::dec_timer_pvar_s16(w, id, pv::COOLDOWN);
    let cool = c::pi16(w, id, pv::COOLDOWN);
    if (cool as i32) < w.ticks(0x2d) {
        if let Some(h) = res.hit {
            let mut dmg = f32::from_bits(h.damage.0);
            if h.attacker.is_some_and(|a| (w.m(a).o_class as u16).wrapping_sub(0xb0) < 2) {
                dmg = if cool == 0 { 1.0 } else { 0.0 };
            }
            group_call(w, id);
            if dmg != 0.0 {
                let hp = c::pf(w, id, pv::D) - dmg;
                let k = pv::K;
                c::set_pi32(w, id, k + knock::k::FLAGS, 9);
                c::set_pf(w, id, k + knock::k::GRAVITY, f32::from_bits(0x3c03_126f));
                c::set_pf(w, id, k + knock::k::DRAG, f32::from_bits(0x3a03_126f));
                c::set_pf(w, id, pv::D, hp);
                c::set_pf(w, id, k + knock::k::SPEED, KNOCK_OUT * c::DT);
                c::set_pf(w, id, k + knock::k::UP, KNOCK_UP * c::DT);
                c::set_pu8(w, id, k + 0x3d, 0);
                let big = h.h2a == 0xb4;
                // [L] the game's stack target record (module doc): Ratchet.
                let from = crate::moby_update::classes::units::hero_pos(w);
                let p = c::pos(w, id);
                let a = c::atan(p[0] - from[0], p[1] - from[1]);
                if hp <= 0.0 {
                    w.mm(id).mode &= !mode::TARGETABLE;
                    let (mut out, mut up) = (c::DT * 8.0, c::DT * 10.0);
                    if big { up *= 1.35; out *= 1.35; }
                    c::set_pf(w, id, k + knock::k::SPEED, out);
                    c::set_pf(w, id, k + knock::k::UP, up);
                    knock::start(w, id, k, a, 5, 1, 0);
                    c::set_pf(w, id, k + knock::k::KEY_APEX, 11.0);
                    c::set_pf(w, id, k + knock::k::KEY_LAND, 18.0);
                    let fl = if w.body() == 2 { 0x200 } else { 0 };
                    set_death_bits(w, id, fl, -1);
                    w.mm(id).has_collision = false;
                    set_state(w, id, st::DYING);
                    c::set_pu8(w, id, pv::FLASH + 7, 0x78);
                } else {
                    if big {
                        c::set_pf(w, id, k + knock::k::UP, KNOCK_UP * c::DT * 1.35);
                        c::set_pf(w, id, k + knock::k::SPEED, KNOCK_OUT * c::DT * 1.35);
                    }
                    knock::start(w, id, k, a, 6, 1, 0);
                    c::set_pf(w, id, k + knock::k::KEY_APEX, 5.0);
                    c::set_pf(w, id, k + knock::k::KEY_LAND, 10.0);
                    set_state(w, id, st::KNOCKED);
                    c::set_pu8(w, id, pv::FLASH + 7, 0xfa);
                    let t = w.ticks(0x3c);
                    c::set_pi16(w, id, pv::COOLDOWN, t as i16);
                }
                flash::start(w, id, pv::FLASH);
            }
        }
    }
    w.mm(id).hit_slot = 0xff;
    // 0x282a90(2.1, m, 0, +0x230): the big-head cheat (0x15edb7).
    crate::moby_update::manip::big_head(w, 2.1, id, 0, id, 0x230);
    let cam = w.camera.map(|x| f32::from_bits(x.0));
    if w.m(id).visible != 0 && c::dist3(c::pos(w, id), cam) < 38.0 {
        crate::shadows::probe_down(w, id);
        w.mm(id).b7f = 0x1e;
    }
    let hp = crate::moby_update::classes::units::hero_pos(w);
    if 40.0 < c::dist2(c::pos(w, id), hp) { return None; }
    if c::pi32(w, id, pv::GIANT_ONLY) != 0 {
        if w.body() != 2 {
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.has_collision = false;
            m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN | mode::NO_ANIM;
            return None;
        }
        let o = w.m(id).o_class;
        let coll = crate::moby_update::classes::units::class_collision(w, o);
        let m = w.mm(id);
        m.mode &= !(mode::HIDDEN | mode::NO_ANIM);
        m.update_dist = 0x40;
        m.has_collision = coll;
        m.mode |= mode::TARGETABLE;
    }
    if c::pi32(w, id, pv::LURE) != 0 {
        let t = w.ticks(0xf0);
        c::set_pi16(w, id, pv::ALERT, t as i16);
    }
    c::set_pi32(w, id, pv::LURE, 0);
    c::dec_timer_pvar_i32(w, id, pv::T208);
    let r = if c::dec_timer_pvar_s16(w, id, pv::ALERT) == 0 { 20.0 } else { 12.0 };
    c::set_pf(w, id, pv::RANGE, r);
    Some(target::acquire(w, id, r))
}

/// Case 0.
fn init(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    c::set_pv4(w, id, pv::HOME, p);
    set_state(w, id, st::GRAZE);
    c::set_pu8(w, id, pv::D + 9, 0);
    c::set_pi16(w, id, pv::ALERT, 0);
    c::set_pi32(w, id, pv::T208, 0);
    c::set_pf(w, id, pv::D, 1.0);
    c::set_pi16(w, id, pv::D + 4, 1);
    c::set_pu8(w, id, 0x5a, 2);
    c::set_pu8(w, id, 0x58, 8);
    walker::seed(&mut w.mm(id).pvars, pv::J);
    let sp = c::pf(w, id, pv::SPEED) * c::DT;
    c::set_pi32(w, id, pv::J, 0x38d);
    c::set_pf(w, id, pv::J + 8, 2.0);
    c::set_pf(w, id, pv::J + 0xc, 2.0);
    c::set_pf(w, id, pv::J + 0x24, sp);
    if w.rng.rand() & 1 != 0 { w.mm(id).mode |= mode::MIRROR; }
    c::set_pi32(w, id, pv::SUCK + react::rec::SEQS, react::seq_table_id(react::SEQS_193));
    blend_r(w, id, 2);
    let g = ground::ground(w, c::pos(w, id), 0.5, 0);
    w.mm(id).position[2] = g.z;
}

/// The charge's start (level09 0x2e3000).
fn charge_start(w: &mut World, id: MobyId) {
    if w.rng.randi(0x100) & 1 != 0 {
        let d = c::pf(w, id, pv::SWERVE_DEG);
        c::set_pf(w, id, pv::SWERVE_DEG, -d);
    }
    let s = c::pf(w, id, pv::SWERVE_DEG) * 0.017_453_292;
    c::set_pf(w, id, pv::SWERVE, s);
    let f = w.rng.randf(30.0, 90.0);
    let t = w.ticks(f as i32);
    c::set_pi32(w, id, pv::SWERVE_T, t);
    set_state(w, id, st::CHARGE);
    blend_r(w, id, 0);
    w.mm(id).cmd = 0;
}

/// The target in sight: a target (kind < 2), or none but its record within `r` (the zero vector: never in practice);
/// then within 8 in z.
fn near(w: &World, id: MobyId, t: &target::Target, r: f32) -> bool {
    let p = c::pos(w, id);
    let ok = t.kind < 2 || c::len3(c::sub(t.pos, p)) < r;
    ok && (p[2] - t.pos[2]).abs() < 8.0
}

/// Case 1.
fn graze(w: &mut World, id: MobyId, t: &target::Target) {
    let r = range(w, id) + 4.0;
    if near(w, id, t, r) && w.rng.randi(0x13) == 0 {
        set_state(w, id, st::FACE);
        blend_r(w, id, 4);
        return;
    }
    if w.m(id).cmd == 1 && w.rng.randi(0x13) == 0 { charge_start(w, id); }
}

/// Cases 2 / 8 / 10 / 11.
fn face(w: &mut World, id: MobyId, t: &target::Target) {
    turn_to(w, id, t.pos, 0.0, 0.02, 0.1);
    let r = range(w, id);
    let mut go = false;
    if near(w, id, t, r) && w.rng.randi(0x13) == 0 {
        group_call(w, id);
        go = true;
    } else if w.m(id).cmd == 1 && w.rng.randi(0x13) == 0 {
        go = true;
    }
    if go { charge_start(w, id); }
    if seq(w, id) == 4 && wrapped(w, id) {
        let n = w.rng.rand_range(10, 0xd);
        let tk = w.ticks(n);
        w.anim_blend(id, 2, 0, tk);
    }
}

/// To the bite (level09 0x2e32ac).
fn to_bite(w: &mut World, id: MobyId) {
    set_state(w, id, st::BITE);
    blend_r(w, id, 1);
    w.mm(id).cmd = 0;
}

/// `|z − target z| / xy distance`.
fn steepness(w: &World, id: MobyId, t: &target::Target) -> (f32, f32) {
    let p = c::pos(w, id);
    ((p[2] - t.pos[2]).abs(), c::dist2(p, t.pos))
}

/// Case 3.
fn charge(w: &mut World, id: MobyId, t: &target::Target) {
    let (dz, dxy) = steepness(w, id, t);
    let a = aim(w, id, t.pos);
    let sw = c::pf(w, id, pv::SWERVE);
    turn_to(w, id, a, sw, 0.05, 0.2);
    let r = step(w, id);
    if c::dec_timer_pvar_i32(w, id, pv::SWERVE_T) != 0 {
        let f = w.rng.randf(30.0, 90.0);
        let tk = w.ticks(f as i32);
        c::set_pi32(w, id, pv::SWERVE_T, tk);
        c::set_pf(w, id, pv::SWERVE, -sw);
    }
    let e = c::sub(t.pos, c::pv4(w, id, pv::HOME));
    if r & 4 != 0 { return to_bite(w, id); }
    if c::dist2(c::pos(w, id), t.pos) < 1.5 && r & 2 == 0 { return to_bite(w, id); }
    let speed = c::pf(w, id, pv::SPEED_NOW);
    if speed < c::DT || 1.0 < dz / dxy || r & 2 != 0 {
        let tk = w.ticks(0x32);
        c::set_pi32(w, id, pv::STUCK_T, tk);
        set_state(w, id, st::HOME);
        w.mm(id).cmd = 0;
        return;
    }
    let p = c::pos(w, id);
    if range(w, id) + 4.0 <= c::len3(e) || 8.0 <= (p[2] - t.pos[2]).abs() {
        if w.m(id).cmd != 1 { set_state(w, id, st::HOME); }
        w.mm(id).cmd = 0;
    } else {
        group_call(w, id);
        w.mm(id).cmd = 0;
    }
}

/// Case 4. Returns the ground probe.
fn bite(w: &mut World, id: MobyId, t: &target::Target) -> ground::Ground {
    turn_to(w, id, t.pos, 0.0, 0.05, 0.2);
    let a = w.m(id).anim;
    if a.seq_a == a.seq_b && ground::key_time(w, id) == 13.0 {
        let p = c::pos(w, id);
        if (p[2] - t.pos[2]).abs() < 0.5 && c::dist2(p, t.pos) < 2.0 {
            let h = c::atan(t.pos[0] - p[0], t.pos[1] - p[1]);
            let yaw = c::yaw(w, id);
            if c::diff_rots(h, yaw) < 0.261_799_4 {
                let (co, si) = c::cs(yaw);
                let dir = [co * 0.2, si * 0.2, 0.0, 0.0];
                let at = [t.pos[0], t.pos[1], t.pos[2] + 0.75, t.pos[3]];
                if let Some(m) = t.moby { attack::hit_moby(w, m, id, 1.0, 1, at, dir); }
            }
        }
    }
    if wrapped(w, id) && 1.5 < c::dist2(c::pos(w, id), t.pos) {
        set_state(w, id, st::CHARGE);
        blend_r(w, id, 0);
    }
    fall(w, id)
}

/// Case 5; `old` the position before the states.
fn home(w: &mut World, id: MobyId, t: &target::Target, stuck: i32, old: c::V) {
    let h = c::pv4(w, id, pv::HOME);
    let a = aim(w, id, h);
    turn_to(w, id, a, 0.0, 0.02, 0.1);
    let r = step(w, id);
    if c::dist2(c::pos(w, id), h) < 2.0 {
        set_state(w, id, st::GRAZE);
        blend_r(w, id, 2);
    }
    let e = c::sub(t.pos, h);
    if stuck != 0 {
        let go = if c::len3(e) < range(w, id) {
            group_call(w, id);
            true
        } else {
            w.m(id).cmd == 1
        };
        if go {
            set_state(w, id, st::CHARGE);
            blend_r(w, id, 0);
            w.mm(id).cmd = 0;
            return;
        }
    }
    if r & 2 != 0 {
        let m = w.mm(id);
        m.position[0] = old[0];
        m.position[1] = old[1];
        set_state(w, id, st::STUCK);
        blend_r(w, id, 4);
    }
    w.mm(id).cmd = 0;
}

/// Case 0xc.
fn stuck(w: &mut World, id: MobyId, t: &target::Target, stuck: i32) {
    let (dz, dxy) = steepness(w, id, t);
    let h = c::pv4(w, id, pv::HOME);
    let a = aim(w, id, h);
    turn_to(w, id, a, 0.0, 0.05, 0.2);
    let r = step(w, id);
    let steep = 1.0 < dz / dxy;
    if r & 4 != 0 || !(1.5 <= c::dist2(c::pos(w, id), t.pos) || steep) {
        return to_bite(w, id);
    }
    let blocked = r & 2 != 0;
    if !steep && !blocked {
        set_state(w, id, st::HOME);
        blend_r(w, id, 0);
        w.mm(id).cmd = 0;
        return;
    }
    if blocked && stuck != 0 {
        set_state(w, id, st::GRAZE);
        blend_r(w, id, 2);
        w.mm(id).cmd = 0;
        return;
    }
    // The wrap blend (level09 0x2e3bd0).
    if wrapped(w, id) {
        w.rng.randi(4);
        let s = if w.rng.randi(4) != 0 { 2 } else { 4 };
        let n = w.rng.rand_range(10, 0xd);
        let tk = w.ticks(n);
        w.anim_blend(id, s, 0, tk);
    }
    w.mm(id).cmd = 0;
}

/// Case 0xd. Returns the ground probe.
fn leashed(w: &mut World, id: MobyId) -> ground::Ground {
    let a = w.m(id).anim;
    if a.seq_a != 0 && a.seq_b != 0 {
        let n = w.rng.rand_range(10, 0xd);
        let tk = w.ticks(n);
        w.anim_blend(id, 0, 0, tk);
    }
    let yaw = c::yaw(w, id);
    let sp = c::pf(w, id, pv::SPEED) * c::DT;
    let p = c::pos(w, id);
    let mut to = [p[0] + yaw.cos() * sp, p[1] + yaw.sin() * sp, p[2], p[3]];
    let n = w.table.mobys.len();
    for o in 0..n {
        let m = &w.table.mobys[o];
        if m.state == crate::moby_runtime::state::END { break; }
        if o == id || m.state >= 0x80 || !(m.o_class == 0 || m.o_class == 0xc1) { continue; }
        let op = m.position;
        if c::dist2(to, op) < 2.0 {
            let z = w.m(id).position[2];
            let me = c::pos(w, id);
            let h = c::atan(op[0] - me[0], op[1] - me[1]);
            let d = c::sub_rot(c::yaw(w, id), h);
            let mut turn = c::DT * std::f32::consts::FRAC_PI_2;
            if d <= 0.0 { turn = -turn; }
            let y = c::add_rot(c::yaw(w, id), turn);
            c::set_yaw(w, id, y);
            let v = c::set_len3(c::sub(me, op), 2.0);
            to = c::add(v, op);
            to[2] = z;
        }
    }
    c::set_pos(w, id, to);
    let g = fall(w, id);
    let leash = c::pi32(w, id, pv::LEASH);
    if leash > 0 {
        let l = (leash - 1) as usize;
        if l < w.table.mobys.len() && c::dist2(c::pos(w, id), c::pos(w, l)) <= c::pi16(w, id, pv::LEASH_R) as f32 {
            w.mm(id).cmd = 0;
            return g;
        }
    }
    set_state(w, id, st::HOME);
    blend_r(w, id, 0);
    w.mm(id).cmd = 0;
    g
}

fn explode_and_delete(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    fx::death_explosion(w, 0.5, 13.0, Some(id), p, 6);
    w.delete_moby(id);
}

/// Level09 `0x2e27d8` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { return; }
    let mut t = target::Target { kind: 2, ..Default::default() };
    if !matches!(state(w, id), st::DYING | st::HELD) {
        match pre(w, id) {
            Some(x) => t = x,
            None => return,
        }
    }
    let old = c::pos(w, id);
    // The stuck timer (non-zero: ran out).
    let stuck_t = c::dec_timer_pvar_i32(w, id, pv::STUCK_T);
    let mut probe: Option<ground::Ground> = None;
    match state(w, id) {
        st::INIT => init(w, id),
        st::GRAZE => graze(w, id, &t),
        2 | 8 | 10 | 11 => face(w, id, &t),
        st::CHARGE => charge(w, id, &t),
        st::BITE => probe = Some(bite(w, id, &t)),
        st::HOME => home(w, id, &t, stuck_t, old),
        st::KNOCKED => {
            let r = knock::update(w, id, pv::K);
            if r & knock::res::LANDED != 0 {
                set_state(w, id, st::RECOVER);
                blend_r(w, id, 3);
                return;
            }
            if w.m(id).position[2] < 0.0 { w.delete_moby(id); return; }
        }
        st::HELD => {
            if react::carried(w, id, pv::K) != 0 {
                set_state(w, id, st::GRAZE);
                if seq(w, id) != 2 { w.anim_blend(id, 2, 0, 0); }
                c::set_pi16(w, id, pv::SUCK + react::rec::STATE, 0);
            }
        }
        st::RECOVER => {
            if 29.0 < ground::key_time(w, id) {
                set_state(w, id, st::CHARGE);
                blend_r(w, id, 0);
                return;
            }
        }
        st::STUCK => stuck(w, id, &t, stuck_t),
        st::LEASHED => probe = Some(leashed(w, id)),
        st::DYING => {
            let r = knock::update(w, id, pv::K);
            if r & knock::res::ANIM_WRAPPED != 0 { explode_and_delete(w, id); return; }
            if w.m(id).position[2] < 0.0 { w.delete_moby(id); return; }
        }
        _ => {}
    }
    if state(w, id) >= 0x80 { return; }
    let g = match probe {
        Some(g) => g,
        None => ground::ground(w, c::pos(w, id), 0.5, 0),
    };
    if g.z == 0.0 || 0.1 <= w.m(id).position[2] - g.z || g.surface != 1 {
        flash::update(w, id, pv::FLASH);
    } else {
        explode_and_delete(w, id);
    }
}
