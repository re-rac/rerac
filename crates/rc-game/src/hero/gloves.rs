//! **The throw gloves' item update: one mechanism, a row per glove** (level01; docs/plan/hero_gameplay.md §13).
//!
//! The four hand gloves' moby updates are copies of one function (the decompiler output of each, compared line by
//! line): the Bomb Glove `0x2d8330` (item 10, class 192), the Mine Glove `0x2d7738` (17, class 190), the Glove of Doom
//! `0x2dd6c0` (20, class 229) and the Decoy Glove `0x2ed5d8` (25, class 562); every level's class table runs the same
//! code for the same classes (`lvl.vtbl` of the 19 overlays). The shared body ([`update`]):
//!
//! 1. the warm-up (Bomb and Mine Glove only: 3 ticks, pvar +0x58 / +0x5c);
//! 2. **the hand point** (+0x40): joint list 0 of the glove plus the row's offset in the glove's frame ([`GloveRow::hand`]);
//! 3. the held object (+0x50): collision off (+0x98 = 1), forgotten once deleted (+0x20 = −2 / −3);
//! 4. the look stance 1 → `SetState(0x1e, 1)` (made right after the slot loop, `super::weapons::after_items`);
//! 5. **the fire trigger** once the 20-tick lockout (+0x54, the Mine Glove's +0x58) is out and 0x1413fc is clear:
//!    the weapon arm out 17 ticks, or ○ in the first-person stance 0x1e with ammo, or the throw state 0x23 at tick 16
//!    → Ratchet's voice 0x1a, one ammo (`0x249450`), the glove's state 3;
//! 6. **the aim of the held object** (the row's [`GloveRow::aim`]: every glove aims ahead of Ratchet's launch point or,
//!    with L1 / L2, along the first-person camera ([`first_person_target`]), then solves the throw's velocity into
//!    the object's pvars; the Bomb Glove searches the target list too);
//! 7. the glove's state machine (+0x20): 0 → 1 / 2 on the animation's wrap; **3 the throw** — the object released
//!    (the row's [`GloveRow::release`]), the throw statistics, state 4 and the 20-tick lockout; a glove with nothing
//!    in hand creates one (Bomb), clicks (class sound 0: Doom, Decoy) or just goes on (Mine); 4 → 2 once neither the
//!    throw state nor the arm is up; 5 → 6; 6 deletes the object;
//! 8. **the tail**: no object → a new one ([`GloveRow::create`]) when the row's refill rule allows ([`Refill`]); else
//!    the object follows the hand point.
//!
//! What differs per glove is data (the hand offset, the warm-up, the refill rule, what an empty throw does) and the
//! glove's own functions (its aim and velocity solver, its object's create and release): the row points at them. The
//! objects are classes of their own (`crate::moby_update::classes::{bomb, mine, decoy, doom_canister}`; the
//! canister lets out the Glove of Doom's bots, `doom_bot`).
//!
//! **The Drone Device** (item 24) is not one of them: its hand class 483 has no update; picking it (the quick select
//! `0x24d238`, the vendor, the Gadgets page) sets 0x141345, and `UpdateWrenchSelected` 0x2307e0 launches the drones
//! (`0x2e8c20`): `super::gadgets` (not ported, gaps.md G-WPN-002).
//!
//! Native `f32` (`atan2` for `FastArcTan`). **Inferred [L]**: the glove's pvars start at 0 when its moby is created
//! (the port keeps one record, [`super::weapons::Glove`], and clears it with the hand moby's creation); the gold
//! gloves (0x13e52a / 0x13e531 / 0x13e534 / 0x13e539) are not mirrored (0).

use super::items::{HitSink, ItemEnv};
use super::packs::SoundCmd;
use super::physics::*;
use super::Hero;
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::classes::{bomb, decoy, doom_canister, mine};
use crate::pad::button;
use crate::rng::Rng;
use crate::targeting;
use rc_formats::moby_anim;

const DT: f32 = 1.0 / 60.0;

/// What the throw state 3 does with no object in the glove.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Empty {
    /// The Bomb Glove: a new bomb (the glove stays in state 3 and throws it next tick).
    Create,
    /// The Glove of Doom and the Decoy Glove: the glove's class sound 0 (the empty click), state 4.
    Click,
    /// The Mine Glove: state 4.
    Nothing,
}

/// When the tail makes a new object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refill {
    /// With ammo, or with the weapon arm out (0x1413f8): the Bomb and Decoy Gloves.
    AmmoOrArm,
    /// With ammo: the Glove of Doom.
    Ammo,
    /// With ammo once the 10-tick delay after a throw (+0x54) is out: the Mine Glove.
    AmmoAfterDelay,
}

/// State 6 (the glove put away): what happens to the object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnDelete {
    /// `DeleteMoby(object)`, then the tail (the Bomb Glove keeps its pointer; the Mine Glove clears it).
    Delete { clear: bool },
    /// Nothing, and no tail (the Glove of Doom, the Decoy Glove: their objects delete themselves).
    Keep,
}

/// Where the throw leaves from: the launch point in front of Ratchet ([`super::weapons::launch_point`]) and the
/// glove's hand point (+0x40).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Throw {
    pub launch: [f32; 3],
    pub hand: [f32; 3],
}

/// One throw glove.
pub struct GloveRow {
    /// Item id (`0x140408`).
    pub id: i32,
    /// The update's address (level01).
    pub update: u32,
    /// The classes the object in the glove can have (for the port's check that the pointer still holds one).
    pub objects: &'static [i16],
    /// The hand point's offset in the glove's frame (added to joint list 0).
    pub hand: [f32; 3],
    /// 3 ticks of warm-up before anything.
    pub warmup: bool,
    pub empty: Empty,
    pub refill: Refill,
    pub on_delete: OnDelete,
    /// The aim of the held object (step 6): writes its velocity; returns the launch point.
    pub aim: fn(&mut Hero, &mut MobyTable, &ItemEnv, MobyId) -> [f32; 3],
    /// A new object in the glove at the hand point (`CreateMoby` through the hit sink).
    pub create: fn(&mut Hero, &mut MobyTable, &ItemEnv, &mut dyn HitSink, &mut Rng) -> Option<MobyId>,
    /// The release (step 7, state 3).
    pub release: fn(&mut Hero, &mut MobyTable, &ItemEnv, &mut dyn HitSink, &mut Rng, MobyId, Throw),
}

/// The throw gloves (see the module doc).
pub static GLOVES: [GloveRow; 4] = [
    GloveRow {
        id: super::items::item::BOMB_GLOVE,
        update: 0x2d8330,
        objects: &[bomb::BOMB_CLASS],
        hand: [0.0, f32::from_bits(0xbdb8_51ec), f32::from_bits(0xbca3_d70a)],
        warmup: true,
        empty: Empty::Create,
        refill: Refill::AmmoOrArm,
        on_delete: OnDelete::Delete { clear: false },
        aim: bomb_aim,
        create: create_bomb,
        release: release_bomb,
    },
    GloveRow {
        id: mine::MINE_GLOVE,
        update: 0x2d7738,
        objects: &mine::CLASSES,
        hand: [0.0, f32::from_bits(0xbdb8_51ec), f32::from_bits(0xbca3_d70a)],
        warmup: true,
        empty: Empty::Nothing,
        refill: Refill::AmmoAfterDelay,
        on_delete: OnDelete::Delete { clear: true },
        aim: mine_aim,
        create: create_mine,
        release: release_mine,
    },
    GloveRow {
        id: decoy::DECOY_GLOVE,
        update: 0x2ed5d8,
        objects: &decoy::CLASSES,
        hand: [0.0, f32::from_bits(0xbe42_8f5c), f32::from_bits(0xbde1_47ae)],
        warmup: false,
        empty: Empty::Click,
        refill: Refill::AmmoOrArm,
        on_delete: OnDelete::Keep,
        aim: decoy_aim,
        create: create_decoy,
        release: release_decoy,
    },
    GloveRow {
        id: doom_canister::GLOVE_OF_DOOM,
        update: 0x2dd6c0,
        objects: &doom_canister::CLASSES,
        hand: [f32::from_bits(0x3c23_d70a), f32::from_bits(0xbe0f_5c29), f32::from_bits(0xbd4c_cccd)],
        warmup: false,
        empty: Empty::Click,
        refill: Refill::Ammo,
        on_delete: OnDelete::Keep,
        aim: doom_aim,
        create: create_canister,
        release: release_canister,
    },
];

/// The row of item `id`.
pub fn row(id: i32) -> Option<&'static GloveRow> { GLOVES.iter().find(|r| r.id == id) }

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn scale3(a: [f32; 3], k: f32) -> [f32; 3] { [a[0] * k, a[1] * k, a[2] * k] }
fn len2f(a: [f32; 3]) -> f32 { (a[0] * a[0] + a[1] * a[1]).sqrt() }

/// The shared glove update (the module doc's steps 1–8), for the row of the hand item.
pub fn update(hero: &mut Hero, table: &mut MobyTable, _anim: &dyn super::anim::AnimCtl, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng) {
    let Some(row) = row(hero.items.slot.id) else { return };
    let Some(class) = hero.items.slot.item.as_ref().and_then(|m| env.data.class(m.o_class)).cloned() else { return };
    // 1. The warm-up.
    if row.warmup && hero.weapons.glove.warmup < 3 {
        hero.weapons.glove.warmup += 1;
        return;
    }
    // 2. The hand point: joint list 0 + the row's offset in the glove's frame. The joint-0 matrix is kept too
    // (`MobyAttachToJoint(glove, 0)`): the held mine's frame (`0x2bfe40`) is it turned by π about x.
    {
        let it = hero.items.slot.item.as_ref().unwrap();
        let rows: [[f32; 3]; 3] = [0, 1, 2].map(|i| [0, 1, 2].map(|k| f32::from_bits(it.rows[i][k])));
        let j0 = class.chains.first().filter(|c| !c.is_empty())
            .map(|chain| moby_anim::evaluate_chains(&class.anim, &it.anim, it.snapshot.as_ref(), &[chain.as_slice()])[0]);
        let base = j0.map_or(from_f32x3(it.position), |j| super::melee::list_point(&j, &it.rows, it.position, it.scale));
        let mtx = j0.map_or_else(
            || {
                let mut f = [[0.0f32; 4]; 4];
                for i in 0..3 { f[i] = it.rows[i].map(f32::from_bits); }
                f[3] = [it.position[0], it.position[1], it.position[2], 1.0];
                f
            },
            |j| moby_anim::attach_matrix(&j, &it.rows, it.position, it.scale),
        );
        let o = row.hand;
        let off: [f32; 3] = std::array::from_fn(|k| rows[0][k] * o[0] + rows[1][k] * o[1] + rows[2][k] * o[2]);
        let g = &mut hero.weapons.glove;
        g.hand = add3(to_f32x3(base), off);
        g.hand_mtx = mtx;
    }
    // 3. The held object.
    if let Some(b) = hero.weapons.glove.held {
        let m = &mut table.mobys[b];
        if m.is_deleted() || !row.objects.contains(&m.o_class) { hero.weapons.glove.held = None; } else { m.coll_disable = 1; }
    }
    // 4. The look stance → first person.
    if hero.state == 1 { hero.weapons.deferred = Some(0x1e); }
    let st = hero.state;
    // 5. The fire trigger (`FastDecTimer` ≠ 0: out).
    let g = &mut hero.weapons.glove;
    if g.fire_timer > 0 { g.fire_timer -= 1; }
    let ready = g.fire_timer == 0;
    let id = hero.items.slot.id;
    if ready && hero.items.f13fc == 0 {
        let arm = hero.f13f8 != 0 && hero.f50c == ticks(0x11);
        let look = env.pad.pressed & hero.items.slot.fire_mask != 0 && (st == 0x1e || hero.weapons.deferred == Some(0x1e)) && hero.weapons.has_ammo(id) != 0;
        let state = st == super::weapons::THROW && hero.timer == ticks(0x10);
        if arm || look || state {
            hero.fx.item_voices.push(SoundCmd::Voice { index: super::weapons::THROW_VOICE, flags: 0 });
            hero.weapons.use_ammo(id, 1);
            if let Some(it) = hero.items.slot.item.as_mut() { it.mstate = 3; }
        }
    }
    // 6. The aim of the held object.
    let mut throw = Throw { launch: [0.0; 3], hand: hero.weapons.glove.hand };
    if let Some(b) = hero.weapons.glove.held { throw.launch = (row.aim)(hero, table, env, b); }
    // 7. The glove's state.
    let mstate = hero.items.slot.item.as_ref().map_or(0, |m| m.mstate);
    let set = |hero: &mut Hero, s: u8| if let Some(it) = hero.items.slot.item.as_mut() { it.mstate = s; };
    match mstate {
        0 | 1 => {
            if mstate == 0 { hero.weapons.glove.held = None; }
            if let Some(it) = hero.items.slot.item.as_mut() {
                if mstate == 0 { it.mstate = 1; }
                if it.anim.flags & 2 != 0 { it.mstate = 2; }
            }
        }
        3 => match hero.weapons.glove.held {
            None => match row.empty {
                Empty::Create => {
                    hero.weapons.glove.held = (row.create)(hero, table, env, hits, rng);
                }
                Empty::Click | Empty::Nothing => {
                    if row.empty == Empty::Click { hero.fx.item_sounds.push(0); }
                    set(hero, 4);
                    hero.weapons.glove.fire_timer = ticks(0x14);
                }
            },
            Some(b) => {
                (row.release)(hero, table, env, hits, rng, b, throw);
                hero.weapons.throws += 1;
                hero.weapons.glove.held = None;
                if row.refill == Refill::AmmoAfterDelay { hero.weapons.glove.respawn = ticks(10); }
                set(hero, 4);
                hero.weapons.glove.fire_timer = ticks(0x14);
            }
        },
        4 => {
            if !(st == super::weapons::THROW || hero.f13f8 != 0) { set(hero, 2); }
        }
        5 => set(hero, 6),
        6 => match row.on_delete {
            OnDelete::Delete { clear } => {
                if let Some(b) = hero.weapons.glove.held {
                    hits.delete_moby(table, b, env.frame as u64);
                    if clear { hero.weapons.glove.held = None; }
                }
            }
            OnDelete::Keep => return,
        },
        _ => {}
    }
    // 8. The tail.
    if row.refill == Refill::AmmoAfterDelay && hero.weapons.glove.respawn > 0 { hero.weapons.glove.respawn -= 1; }
    match hero.weapons.glove.held {
        None => {
            let ammo = hero.weapons.has_ammo(id) != 0;
            let make = match row.refill {
                Refill::AmmoOrArm => ammo || hero.f13f8 != 0,
                Refill::Ammo => ammo,
                Refill::AmmoAfterDelay => hero.weapons.glove.respawn == 0 && ammo,
            };
            if make { hero.weapons.glove.held = (row.create)(hero, table, env, hits, rng); }
        }
        Some(b) => {
            let h = hero.weapons.glove.hand;
            let m = &mut table.mobys[b];
            m.position = [h[0], h[1], h[2], m.position[3]];
        }
    }
}

// ------------------------------------------------------------------------------------------------
// The shared aim pieces

/// Ratchet's facing row (0x13f9c0: `(1, 0, 0)` through his rows 0x13f990) and its yaw.
fn facing(hero: &Hero) -> ([f32; 3], f32) {
    let fwd = to_f32x3(hero.moby_rows[0]);
    (fwd, fwd[1].atan2(fwd[0]))
}

/// The first-person aim of every glove (the L1 / L2 branch): the camera's yaw and pitch (clamped −80°..34°), the
/// slope of a `polar(k, yaw, pitch)` step, the reach `2·r²·(1 − slope)/g` with `r = √(g·k/2)`, and the camera's line
/// (world, flags 2) that far: its hit, or five times the reach from the camera. None without a camera or when the
/// reach is not positive (the target stays the point ahead).
pub fn first_person_target(env: &ItemEnv, k: f32, g: f32) -> Option<[f32; 3]> {
    let (cam, cf) = env.camera?;
    let yaw = cf[1].atan2(cf[0]);
    let pitch = cf[2].atan2(len2f(cf)).min(f32::from_bits(0x3f17_e9d8)).max(f32::from_bits(0xbfb2_b8c2));
    let v = [yaw.cos() * k * pitch.cos(), yaw.sin() * k * pitch.cos(), pitch.sin() * k];
    let mut l = len2f(v);
    if l == 0.0 { l = 0.01; }
    let slope = v[2] / l;
    let r = (g * k * 0.5).sqrt();
    let range = ((r + r) * r * (1.0 - slope)) / g;
    if !(0.0 < range) { return None; }
    let d = [yaw.cos() * range, yaw.sin() * range, slope * range];
    let end = add3(cam, d);
    let hit = env.coll.and_then(|c| line_world(c, from_f32x3(cam), from_f32x3(end), 2));
    Some(match hit {
        Some(o) => o.point,
        None => add3(cam, scale3(d, 5.0)),
    })
}

/// The launch point `height` above Ratchet's feet (0.859 ahead-left: `super::weapons::launch_point` at 0.49082; the
/// Doom and Decoy Gloves' 0.39082).
pub fn launch_at(hero: &Hero, height: f32) -> [f32; 3] {
    let (fwd, _) = facing(hero);
    let mut p = super::weapons::launch_point(to_f32x3(hero.pos), fwd);
    p[2] += height - 0.490_82;
    p
}

/// The releases' wall test `CollLine_Fix(from, to, 0, Ratchet, 0)` (the mine's and the decoy's: the world and the
/// mobys, Ratchet ignored): the hit point and its raw normal. Through the hit sink's probe; a sink without collision
/// tests the world mesh alone.
fn wall_probe(table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, from: [f32; 3], to: [f32; 3]) -> Option<([f32; 3], [f32; 3])> {
    match hits.probe_moby(table, from_f32x3(from), from_f32x3(to), 0, Some(env.hero_moby)) {
        Some(p) => p.map(|p| (p.point, p.normal)),
        None => env.coll.and_then(|c| line_world(c, from_f32x3(from), from_f32x3(to), 0)).map(|o| (o.point, o.normal)),
    }
}

// ------------------------------------------------------------------------------------------------
// The Bomb Glove (0x2d8330): the bomb class 121 (crate::moby_update::classes::bomb)

/// The Bomb Glove's aim: 8.5 ahead of the launch point, or at the target the glove's search picks (without L1 / L2),
/// or along the camera (with them); `0x2d80f0` with gravity 11 into the bomb's velocity.
fn bomb_aim(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, b: MobyId) -> [f32; 3] {
    let (fwd, yawf) = facing(hero);
    let launch = super::weapons::launch_point(to_f32x3(hero.pos), fwd);
    let k = 8.5f32;
    let strafe = env.pad.held & button::STRAFE != 0;
    let mut target = add3(launch, scale3(fwd, k));
    if !strafe || hero.items.f13fc != 0 {
        // The target search over 0x1abe80 (crate::targeting); its line of sight from the camera, world only.
        hero.weapons.aim = None;
        let cam = env.camera.map(|c| c.0);
        let clear = |p: [f32; 3]| match (cam, env.coll) {
            (Some(c), Some(coll)) => line_world(coll, from_f32x3(c), from_f32x3(p), targeting::BOMB_GLOVE.los_flags).is_none(),
            _ => true,
        };
        if let Some(t) = targeting::aim_search(&targeting::BOMB_GLOVE, table, env.targets, launch, yawf, clear) {
            target = t.point;
            hero.weapons.aim = Some(t.id);
            let m = &table.mobys[t.id];
            hero.weapons.aim_pos = [m.position[0], m.position[1], m.position[2]];
        }
    } else if let Some(t) = first_person_target(env, k, DT * DT * 11.0) {
        target = t;
    }
    let v = super::weapons::launch_velocity(DT * DT * 11.0, k, hero.rot[2].to_f32(), launch, target);
    if let Some(m) = table.mobys.get_mut(b) {
        if m.pvars.len() < 0x80 { m.pvars.resize(0x80, 0); }
        crate::moby_update::services::pvar::set_v4f(&mut m.pvars, bomb::pv::VEL, [v[0], v[1], v[2], 0.0]);
    }
    launch
}

/// `0x2c2640`: a bomb in the glove (`CreateMoby(0x79)` through the hit sink).
fn create_bomb(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng) -> Option<MobyId> {
    let b = hits.create_moby(table, bomb::BOMB_CLASS, env.frame as u64)?;
    let (_, yaw) = facing(hero);
    bomb::init_held(&mut table.mobys[b], rng, hero.weapons.glove.hand, yaw);
    Some(b)
}

/// `0x2c27a8`: the bomb released at the launch point (plus Ratchet's displacement while grinding); a wall between
/// Ratchet (at the launch height) and the launch point explodes it there.
fn release_bomb(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, _hits: &mut dyn HitSink, _rng: &mut Rng, b: MobyId, t: Throw) {
    let grind = if hero.group == 0xf { Some(to_f32x3(hero.disp)) } else { None };
    bomb::release(&mut table.mobys[b], t.launch, grind);
    let r = &table.mobys[env.hero_moby];
    let from = [r.position[0], r.position[1], t.launch[2]];
    if let Some(o) = env.coll.and_then(|c| line_world(c, from_f32x3(from), from_f32x3(t.launch), 0)) {
        bomb::release_hit(&mut table.mobys[b], o.point);
    }
}

// ------------------------------------------------------------------------------------------------
// The Mine Glove (0x2d7738): the mine class 74 (crate::moby_update::classes::mine)

/// The Mine Glove's aim (inline in `0x2d7738`; skipped with 0x1413fc): the launch point, the point 4 ahead (the
/// velocity `(4·dt, 0, 0)` through Ratchet's rows, 60 ticks of it; 6 with L1 / L2, 12 gold), or the camera's with
/// L1 / L2 (gravity 9.8); `0x2d7590` ([`mine_velocity`]) into the mine's velocity.
fn mine_aim(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, b: MobyId) -> [f32; 3] {
    let (fwd, _) = facing(hero);
    let launch = super::weapons::launch_point(to_f32x3(hero.pos), fwd);
    if hero.items.f13fc != 0 { return launch; }
    let strafe = env.pad.held & button::STRAFE != 0;
    // The gold Mine Glove (0x13e531, item 17) reaches 12 with L1 / L2.
    let gold = hero.weapons.gold.get(17).is_some_and(|&g| g != 0);
    let k = if !strafe { 4.0 } else if gold { 12.0 } else { 6.0 };
    let mut target = add3(launch, scale3(fwd, k));
    if strafe {
        if let Some(t) = first_person_target(env, k, DT * DT * 9.8) { target = t; }
    }
    let v = mine_velocity(DT * DT * 9.8, 1.0, k, launch, target);
    if let Some(m) = table.mobys.get_mut(b) {
        if m.pvars.len() < mine::pv::SIZE { m.pvars.resize(mine::pv::SIZE, 0); }
        crate::moby_update::services::pvar::set_v4f(&mut m.pvars, mine::pv::VEL, [v[0], v[1], v[2], 0.0]);
    }
    launch
}

/// `0x2d7590(g, h0, k, from, to)`: the mine's lob at 45°: `to` pulled within `k − 0.01` of `from`, the xy distance `h`
/// (at least `h0`), the rise `dz` (below `h`), the speed `√min(g·h²/(2·(h − dz)), g·k/2)` both across and up.
pub fn mine_velocity(g: f32, h0: f32, k: f32, from: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    let cap = g * k * 0.5;
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let t = if k < l { add3(from, scale3(d, (k - 0.01) / l)) } else { to };
    let h = len2f([t[0] - from[0], t[1] - from[1], 0.0]).max(h0);
    let mut dz = t[2] - from[2];
    let yaw = (t[1] - from[1]).atan2(t[0] - from[0]);
    if h <= dz { dz = h - 0.01; }
    let v = ((g * h * h) / ((h - dz) + (h - dz))).min(cap).sqrt();
    [yaw.cos() * v, yaw.sin() * v, v]
}

/// `0x2bf6a0`: a mine in the glove (`CreateMoby(0x4a)`).
fn create_mine(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng) -> Option<MobyId> {
    let m = hits.create_moby(table, mine::CLASS, env.frame as u64)?;
    mine::init_held(&mut table.mobys[m], rng, hero.weapons.glove.hand, env.frame);
    Some(m)
}

/// `0x2bf7d8`: the mine leaves from the launch point (the mines' list 0x1b0c30 is the table's).
fn release_mine(_hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng, m: MobyId, t: Throw) {
    mine::release(table, m, rng, t.launch);
    let r = table.mobys[env.hero_moby].position;
    let from = [r[0], r[1], t.launch[2]];
    if let Some(h) = wall_probe(table, env, hits, from, t.launch) { mine::release_hit(&mut table.mobys[m], h.0, h.1); }
}

// ------------------------------------------------------------------------------------------------
// The Decoy Glove (0x2ed5d8): the decoys 203 / 1900 (crate::moby_update::classes::decoy)

/// `0x2ed0c0`: 2.5 ahead of the launch point (0.39082 up), or along the camera with L1 / L2 (3.5, gravity 9);
/// 0x13fda0 cleared without them; `0x2ece98` (the Bomb Glove's solver with the reach 2.5 / 3.5) into the decoy's
/// velocity (+0x10).
fn decoy_aim(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, b: MobyId) -> [f32; 3] {
    let (launch, target, k) = short_throw_aim(hero, env);
    let v = super::weapons::launch_velocity(DT * DT * 9.0, k, hero.rot[2].to_f32(), launch, target);
    if let Some(m) = table.mobys.get_mut(b) { decoy::set_velocity(m, v); }
    launch
}

/// The Doom and Decoy Gloves' shared aim (`0x2dd1b0` / `0x2ed0c0`, the same code): the launch point 0.39082 up, the
/// point `k` = 2.5 ahead (3.5 with L1 / L2), the camera's with L1 / L2 (gravity 9).
fn short_throw_aim(hero: &mut Hero, env: &ItemEnv) -> ([f32; 3], [f32; 3], f32) {
    let (fwd, _) = facing(hero);
    let launch = launch_at(hero, 0.390_82);
    let strafe = env.pad.held & button::STRAFE != 0;
    let k = if strafe { 3.5 } else { 2.5 };
    let mut target = add3(launch, scale3(fwd, k));
    if !strafe || hero.items.f13fc != 0 {
        hero.weapons.aim = None;
    } else if let Some(t) = first_person_target(env, k, DT * DT * 9.0) {
        target = t;
    }
    (launch, target, k)
}

/// `0x2d9228`: a decoy in the glove (`CreateMoby(0xcb)`, 0x76c gold).
fn create_decoy(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut Rng) -> Option<MobyId> {
    let gold = hero.weapons.gold.get(decoy::DECOY_GLOVE as usize).is_some_and(|&g| g != 0);
    let class = if gold { decoy::GOLD_CLASS } else { decoy::CLASS };
    let d = hits.create_moby(table, class, env.frame as u64)?;
    decoy::init_held(table, d, rng, hero.weapons.glove.hand, gold, hero.f13f5 != 0 || hero.f13ff != 0);
    decoy::cap_count(table, d);
    Some(d)
}

/// `0x2d94f0`: the decoy leaves from the hand point.
fn release_decoy(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, _rng: &mut Rng, d: MobyId, t: Throw) {
    let r = table.mobys[env.hero_moby].position;
    let from = [r[0], r[1], t.hand[2]];
    let end = decoy::release_line(from, t.hand);
    let hit = wall_probe(table, env, hits, from, end);
    decoy::release(&mut table.mobys[d], t.hand, hit);
    hero.weapons.glove.hand = end;
}

// ------------------------------------------------------------------------------------------------
// The Glove of Doom (0x2dd6c0): the canister 230 (crate::moby_update::classes::doom_canister), its bots 186

/// `0x2dd1b0`: the Decoy Glove's aim ([`short_throw_aim`], the same code); `0x2dcf88` (the Bomb Glove's solver with
/// the reach 2.5 / 3.5 and gravity 9) into the canister's velocity (+0x00).
fn doom_aim(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, b: MobyId) -> [f32; 3] {
    let (launch, target, k) = short_throw_aim(hero, env);
    let v = super::weapons::launch_velocity(DT * DT * 9.0, k, hero.rot[2].to_f32(), launch, target);
    if let Some(m) = table.mobys.get_mut(b) { doom_canister::set_velocity(m, v); }
    launch
}

/// `0x2dde80`: a canister in the glove (`CreateMoby(0xe6)`).
fn create_canister(hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, _rng: &mut Rng) -> Option<MobyId> {
    let c = hits.create_moby(table, doom_canister::CLASS, env.frame as u64)?;
    doom_canister::init_held(table, c, hero.weapons.glove.hand, hero.f13f5 != 0 || hero.f13ff != 0);
    Some(c)
}

/// `0x2ddf58`: the canister leaves from the hand point; the line from Ratchet at its height (the world and the mobys,
/// Ratchet ignored) puts it on a wall in between.
fn release_canister(_hero: &mut Hero, table: &mut MobyTable, env: &ItemEnv, hits: &mut dyn HitSink, _rng: &mut Rng, c: MobyId, t: Throw) {
    let r = table.mobys[env.hero_moby].position;
    let from = [r[0], r[1], t.hand[2]];
    let hit = wall_probe(table, env, hits, from, t.hand);
    doom_canister::release(&mut table.mobys[c], t.hand, hit);
}
