//! U553 (census 2026-09-29; the ids renumber per run): class 568, the rolling mines of level 18 (Veldin, 20 created
//! instances; level18 `0x2d5918`, its tick `0x2d5f20`). A **pool**: every placed mine parks itself on its first update
//! (state 5: hidden, no collision, untargetable) and waits for level18 `0x2d5cf8` ([`throw`]) — called only by the boss
//! class 1422 (`units::veldin_boss`, state 0x10) — to pick a parked one of a group, put it at a launch point
//! and throw it (state 1: the ballistic velocity pvar +0x180, the landing height +0x178, the fuse +0x194). It falls,
//! lands and rolls out (state 2), then (state 3) rolls toward a target within 6 (Ratchet or a decoy, the shared search
//! `0x274b78`); it blows up — a beam explosion, then parked again — on a weapon hit, when the fuse runs out, when
//! Ratchet comes within 2 while it is still falling (small: no damage), or on contact (Ratchet's capsule touching it,
//! `0x13f58c` / `0x13f590`, or within 2 of a decoy it chases: large, a 1.5 damage sphere). The Suck Cannon takes it
//! (its own table, [`react::ROLLING_MINE_568`]: state 4 → back to the state it was taken in).
//!
//! **Pvars** (0x1a0): +0x30 0.05 (written by the init, read by nothing here), +0x40 the roll velocity, +0x60 the suck
//! record (+0xc8 its state; +0xd0 the sequence table gp−0x5278), +0x110 the knockback record the carried update uses,
//! +0x170 the landing point (its z +0x178), +0x180 the throw velocity (z +0x188, then the roll-out speed), +0x194
//! (s32) the fuse, +0x198 the launcher's word, +0x19c (s32) "touched a decoy": the large blast next tick.
//!
//! ## Coverage (level18 `0x2d5918`, `0x2d5f20`, the table 0x2d6108..0x2d6280)
//!
//! | address | what it does | ported / not |
//! |---|---|---|
//! | `0x2d5f20` states ∉ {0, 4, 5} | the tick | [`tick`] |
//! | Ratchet in state 0x72 (`0x1413d4`) → state 5, parked (no blast) | | [`tick`], [`park_fields`] |
//! | `MobyGetHitMessage(m, 0x330000, 0)` (0x26f320), +0xa4 = 0xff | a weapon hit (every weapon's record, the resolver's mask) | [`tick`] (`World::get_hit`) |
//! | capsule moby `0x13f58c` = m, or `0x13f590` = m, or +0x19c → `SpawnBeamExplosion(1.5, 1, 3, 1.5, 9, 1.5, 0, m, 0x15f580, pos, 20, 6, 32, sound 1, shake, 0, −1, 0)` | the large blast: damage sphere 1.5 / 1 (flags 0x810001, type 2 / 1) on Ratchet and every moby, streaks, sparks, puffs, flashes, camera shake, class sound 1 | [`tick`] ([`LARGE`], `fx::beam_explosion`); the capsule words are the hero's (`Hero::cap_moby`, not filled by the port's hero yet: the seam gaps.md G-HERO-033) |
//! | fuse `FastDecTimer(+0x194)` out, or the hit, or (state 1 and Ratchet within 2, xy) → `SpawnBeamExplosion(0, 0, 1, 0.5, 9, 0.5, 0, m, …, 5, 2, 8, sound 1, no shake)` | the small blast (no damage) | [`tick`] ([`SMALL`]) |
//! | after a blast: state 5, +0x94 = 0, mode = mode & ~0x1000 \| 0x41 | parked | [`park_fields`] |
//! | case 0: +0x94 = 0, mode & ~0x1000 \| 0x41, +0x30 = 0.05, +0xd0 = gp−0x5278, state 5 | init: parked | [`update`] (the sequence table is [`react::ROLLING_MINE_568`]'s) |
//! | case 1: pos += +0x180; +0x188 −= 10·dt²; below +0x178 → z = +0x178, state 2, +0x188 = −(2.5 + 2.5)·dt | the fall | [`update`] |
//! | case 2: `Approach(0, 5·dt², +0x188)` → d; rot.y += d; pos += (cos yaw, sin yaw, 0)·d; stopped → blend 1 (`ticks(12)`), +0xa4 = 0xff, state 3 | the roll-out | [`update`] |
//! | case 3: sequence 1 wrapped → blend 2 (`ticks(12)`) | | [`update`] |
//! | case 3: drawn, `0x274b78(6, m)` kind ≠ 2, a moby, within 6 (xy): d = target − pos, d.z = −10·dt², xy ≤ 6·dt²; vel += d, xy ≤ 3·dt; `0x26d610(0, 0.5, 0, m, &vel, 0)`; vel = pos − old, vel.z ∈ [−3·dt, 0] | the chase | [`chase`] (`target::acquire`, `walker::move_collide`) |
//! | a target that is not Ratchet's moby (`0x1413d0`) within 2 (3-D) → +0x19c = 1 | on a decoy: the large blast next tick | [`chase`] |
//! | case 4: `0x305260(m, +0x110)` done → state 3, record state 0 | the Suck Cannon's carried update | [`update`] (`react::carried`) |
//! | table 0x2d6108 / 0x2d6190 / 0x2d61e0 / 0x2d6230 / 0x2d6250 | the Suck Cannon's wrappers: taken only in states 3 / 4 (else record state 0), the state saved in +0xbc, record +0x60 | `react::slot_*` with [`react::ROLLING_MINE_568`] |
//! | table 0x2d6280 | slot +0x14: state 5, blend 0 (`ticks(10)`) when not on 0, +0x31 = 0, +0x94 = 0, mode & ~0x1000 \| 1 | `react::pool_park` |
//! | level18 `0x2d5cf8(s, boss, group, from, to, fuse)` (the boss 1422's call) | the last parked (5) mine of the group in list order: update / draw 0xff, state 1, mode & ~0x41 \| 0x1000, drawn, +0xbc = 0, collision on; blend 0 (1 tick) when not on 0; position from; rot.x 0, rot.y −2.5; to.z + 0.75; landing = from + (to − from) at xy length −2.5 shorter; +0x170 = it; +0x180 = unit(it − from)·s with z 0, +0xc8 = 0, +0x188 = `0x25cf30(s, −10·dt², from, landing, 0)`; yaw = atan(+0x180); +0x194 fuse, +0x198 the boss, +0x19c 0; `PlayClassSound(0, 0)` | [`throw`] (2026-10-01, with the boss) |
//!
//! No bolts, no death bits, no pieces: the mine is never deleted. The large blast is the only hit on Ratchet (the
//! beam explosion's damage sphere → his moby's hit record → the hero's intake).
//!
//! Native `f32`; the rand draws are the beam explosion's.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::fx::{beam_explosion, Beam};
use crate::moby_update::creature::{self as c, react, target, turn, walker};
use crate::moby_update::services::World;

pub const UPDATE_FN: u32 = 0x2d_5918;
pub const REFERENCE_LEVEL: u32 = 18;
pub const CLASSES: [i16; 1] = [568];

pub mod pv {
    pub const VEL: usize = 0x40;
    pub const RECORD: usize = 0x60;
    pub const KNOCK: usize = 0x110;
    pub const LAND: usize = 0x170;
    pub const THROW: usize = 0x180;
    pub const FUSE: usize = 0x194;
    pub const WORD: usize = 0x198;
    pub const TOUCHED: usize = 0x19c;
    pub const SIZE: usize = 0x1a0;
}

pub mod st {
    pub const INIT: u8 = 0;
    pub const FALL: u8 = 1;
    pub const ROLL_OUT: u8 = 2;
    pub const CHASE: u8 = 3;
    pub const HELD: u8 = 4;
    pub const PARKED: u8 = 5;
}

/// gp−0x526c (level18 0x161994): 2.5.
const K_ROLL: f32 = 2.5;

/// The contact blast.
pub const LARGE: Beam = Beam { damage_r: 1.5, damage: 1.0, flash: 3.0, flash2: 1.5, flash_dist: 9.0, scale: 1.5, light: 0.0, streaks: 20, sparks: 6, puffs: 0x20, debris: 0, sound: 1, shake: true };
/// The blast of a hit, of the fuse, or of Ratchet near a falling mine.
pub const SMALL: Beam = Beam { damage_r: 0.0, damage: 0.0, flash: 1.0, flash2: 0.5, flash_dist: 9.0, scale: 0.5, light: 0.0, streaks: 5, sparks: 2, puffs: 8, debris: 0, sound: 1, shake: false };

/// The parked fields (state 5, no collision, untargetable, hidden, no animation).
fn park_fields(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.state = st::PARKED;
    m.has_collision = false;
    m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN | mode::NO_ANIM;
}

/// Level18 `0x2d5cf8(s, boss, group, from, to, fuse)` (module doc): the boss 1422 throws a parked mine of `group`.
pub fn throw(w: &mut World, s: f32, boss: MobyId, group: i32, from: [f32; 4], to: [f32; 4], fuse: i32) -> Option<MobyId> {
    let g = i8::try_from(group).ok()?;
    let id = crate::moby_update::scheduler::group_ids(w, g).into_iter().rfind(|&m| w.m(m).o_class == 0x238 && w.m(m).state == st::PARKED)?;
    if w.m(id).pvars.len() < pv::SIZE { return None; }
    let col = super::class_collision(w, 0x238);
    {
        let m = w.mm(id);
        m.update_dist = 0xff;
        m.draw_dist = 0xff;
        m.state = st::FALL;
        m.mode = (m.mode & !0x41) | mode::TARGETABLE;
        m.visible = 1;
        m.cmd = 0;
        m.has_collision = col;
    }
    if w.m(id).anim.seq_b != 0 { w.anim_blend(id, 0, 0, 1); }
    let m = w.mm(id);
    m.position = from;
    m.rotation[0] = 0.0;
    m.rotation[1] = -K_ROLL;
    let mut to = to;
    to[2] += 0.75;
    let a = c::sub(to, from);
    let a = c::add(c::set_len2(a, c::len2(a) - K_ROLL), from);
    c::set_pv4(w, id, pv::LAND, a);
    let mut v = c::sub(a, from);
    v[2] = 0.0;
    let v = c::set_len3(v, s);
    c::set_pi16(w, id, 0xc8, 0);
    let mut t = 0.0;
    let vz = crate::moby_update::creature::knock::lob_up(s, -(c::DT2 * 10.0), from, a, &mut t);
    c::set_pv4(w, id, pv::THROW, [v[0], v[1], vz, v[3]]);
    w.mm(id).rotation[2] = c::atan(v[0], v[1]);
    c::set_pi32(w, id, pv::FUSE, fuse);
    c::set_pi32(w, id, pv::WORD, boss as i32 + 1);
    c::set_pi32(w, id, pv::TOUCHED, 0);
    w.play_sound(0, 0, id);
    Some(id)
}

/// `0x2d5f20` (module doc).
fn tick(w: &mut World, id: MobyId) {
    if matches!(w.m(id).state, st::PARKED | st::INIT | st::HELD) { return; }
    if w.hero.state == 0x72 {
        park_fields(w, id);
        return;
    }
    let hit = w.get_hit(id, 0x33_0000, false);
    w.mm(id).hit_slot = 0xff;
    // 0x13f58c / 0x13f590: the moby Ratchet's capsule last pushed out of (player_controller.md, the capsule resolve);
    // `Hero::cap_moby` and `Hero::wall_moby`.
    let contact = w.hero.cap_moby == Some(id) || w.hero.wall_moby == Some(id) || c::pi32(w, id, pv::TOUCHED) != 0;
    let blast = if contact {
        LARGE
    } else {
        let fuse = c::dec_timer_pvar_i32(w, id, pv::FUSE);
        if fuse == 0 && hit.is_none() {
            if w.m(id).state != st::FALL { return; }
            let hp = crate::moby_update::classes::units::hero_pos(w);
            if 2.0 <= c::dist2(c::pos(w, id), hp) { return; }
        }
        SMALL
    };
    let p = c::pos(w, id);
    beam_explosion(w, &blast, Some(id), p);
    park_fields(w, id);
}

/// State 3's chase (module doc).
fn chase(w: &mut World, id: MobyId) {
    let t = target::acquire(w, id, 6.0);
    let Some(tm) = t.moby.filter(|_| t.kind != 2) else { return };
    let p = c::pos(w, id);
    let tp = c::pos(w, tm);
    if 6.0 <= c::dist2(p, tp) { return; }
    let max = c::DT * 3.0;
    let acc = c::DT2 * 6.0;
    let mut d = c::sub(tp, p);
    d[2] = -(c::DT2 * 10.0);
    if acc < c::len2(d) { d = c::set_len2(d, acc); }
    let mut v = c::add(c::pv4(w, id, pv::VEL), d);
    if max < c::len2(v) { v = c::set_len2(v, max); }
    walker::move_collide(w, id, 0.0, 0.5, 0.0, &mut v, 0);
    let mut v = c::sub(c::pos(w, id), p);
    if 0.0 < v[2] { v[2] = 0.0; } else if v[2] < -max { v[2] = -max; }
    c::set_pv4(w, id, pv::VEL, v);
    if Some(tm) != w.hero_moby && c::dist3(tp, c::pos(w, id)) < 2.0 { c::set_pi32(w, id, pv::TOUCHED, 1); }
}

/// Level18 `0x2d5918` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { return; }
    tick(w, id);
    let s = w.m(id).state;
    match s {
        st::INIT => {
            park_fields(w, id);
            c::set_pf(w, id, 0x30, f32::from_bits(0x3d4c_cccd));
        }
        st::FALL => {
            let mut p = c::pos(w, id);
            let v = c::pv4(w, id, pv::THROW);
            p = [p[0] + v[0], p[1] + v[1], p[2] + v[2], p[3]];
            c::set_pos(w, id, p);
            let vz = c::pf(w, id, pv::THROW + 8) - c::DT2 * 10.0;
            c::set_pf(w, id, pv::THROW + 8, vz);
            let floor = c::pf(w, id, pv::LAND + 8);
            if p[2] < floor {
                w.mm(id).position[2] = floor;
                w.mm(id).state = st::ROLL_OUT;
                c::set_pf(w, id, pv::THROW + 8, -((K_ROLL + K_ROLL) * c::DT));
            }
        }
        st::ROLL_OUT => {
            let mut s = c::pf(w, id, pv::THROW + 8);
            let d = turn::approach(0.0, (K_ROLL + K_ROLL) * c::DT2, &mut s);
            c::set_pf(w, id, pv::THROW + 8, s);
            let yaw = c::yaw(w, id);
            let m = w.mm(id);
            m.rotation[1] += d;
            m.position[0] += yaw.cos() * d;
            m.position[1] += yaw.sin() * d;
            if s == 0.0 {
                if w.m(id).anim.seq_b != 1 {
                    let t = w.ticks(12);
                    w.anim_blend(id, 1, 0, t);
                }
                w.mm(id).hit_slot = 0xff;
                w.mm(id).state = st::CHASE;
            }
        }
        st::CHASE => {
            if w.m(id).anim.seq_b == 1 && w.m(id).anim.flags & 2 != 0 {
                let t = w.ticks(12);
                w.anim_blend(id, 2, 0, t);
            }
            if w.m(id).visible != 0 { chase(w, id); }
        }
        st::HELD if react::carried(w, id, pv::KNOCK) != 0 => {
            w.mm(id).state = st::CHASE;
            c::set_pi16(w, id, pv::RECORD + react::rec::STATE, 0);
        }
        _ => {}
    }
}
