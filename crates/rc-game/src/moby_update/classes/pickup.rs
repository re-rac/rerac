//! Pickups: the ammo pickups (classes 204, 213, 214, 222, 223, 225, 226, 1006, 1438, 1447, 1449: `AmmoPickupUpdate`
//! level01 0x2db028, Lombyte's `FxDebrisGroupUpdateB`), the nanotech cluster over a nanotech crate (class 806,
//! `NanotechUpdate` 0x300de0), the item pickup spawner `0x2daf10` and the ammo type pick `0x26bff8` that the ammo
//! crate and other droppers share. Spec: `docs/plan/hero_gameplay.md` §2.
//!
//! **What the game shares.** Bolts (`bolt`), ammo pickups and the nanotech orbs are three separate update
//! functions; what they share is the hero's pickup volume (`0x1415d8` xy radius / `0x1415dc` height, rewritten by
//! every hero tick: 3.0 / 1.75, 12 / 4.5 with the bolt grabber; [`World::bolt_radii`]), the target point
//! (Ratchet's body point 0x13f420 for bolts and ammo, his chest `pos + 0.75 up` for the orbs), the "fly to him and
//! vanish within one step" rule and the item tables (`0x1c4530` records: max ammo +0xe, pickup amount +0xc; the
//! 0x4c-byte item records `0x179f40`: pickup class +0x3a, the pickup banner texts +0x34 / +0x36). The port keeps
//! those shared parts here ([`in_reach`], [`add_ammo`], [`Inventory`] through [`ItemTables`]) and each update
//! in the game's own shape.
//!
//! ## Ammo pickups (0x2db028)
//!
//! Pvars: +0x00 amount, +0x04 s16 (the dropper's +0xc4), +0x06 s16 the dim state's timer, +0x08 item, +0x0c f32,
//! +0x10 velocity (u/tick), +0x20 s32 pickup delay (`ticks(16)`, `FastDecTimer` every tick), +0x24 f32 rise speed,
//! +0x28 f32 spawn z. States (moby +0x20):
//! * physics in 1 / 3 / 4: moving → a sphere (0.4, 0.5 up, flags 2) pushes it out; gravity `4.9·dt²` before and
//!   after the step (`pos + vel`), a line from 0.1 above to the new point (flags 2) lands it (velocity 0);
//! * **1** (idle): alpha → 0x80 over 30 ticks; once the delay is out and it rests: collected when Ratchet is
//!   within the pickup volume (or it fell 2 below its spawn height) with the ammo below max and health left →
//!   `AddAmmo` 0x2494d8 (clamped to max), the banner `ShowBannerf(+0x36 for 1 else +0x34, n)`, the picked-up stat
//!   0x13de08, state 2, scale ×1.2, the rise speed `8·dt`, sound 0 of class 213 at most every 10 ticks; else alpha +1
//!   and, when Ratchet's bounding sphere overlaps its own by 0.2 (full ammo), state 3;
//! * **2** (collected): rises (the rise speed falls to 0 by `24·dt²`) and flies at the speed `|vel|` → `64·dt`
//!   (`24·dt²`) to the body point; deleted within one step or at scale 0; the scale follows `class scale ·
//!   clamp(d / (30 ticks · speed), 0.3, 1)`;
//! * **3** (full ammo, touched): alpha pulses `10 + 5 + 5·sin` over a second; back to 1 when Ratchet leaves or
//!   the ammo drops below max;
//! * **4** (grow in: scale → class scale by 3 % a tick, then state 1). 0 → 1.
//!
//! ## Nanotech cluster 806 (0x300de0)
//!
//! Created by the nanotech crate 501 at its init (`CrateSpawnIconMoby` 0x300528, pvar +0x0c = the crate). The moby
//! is hidden (mode 0x41); what is drawn are 8 type-62 orbs (kind 1, size 60000, 0x7f7f4040) and their trail puffs
//! (kind 2, 10000, attached). The first cluster to init is the **master** (0x15f63c): it keeps the 8 orbit
//! directions `0x1fe600` (the crate's rows ±, 0.25 long) and turns each about its own axis `0x1fe680` by 4° a tick
//! ([`Globals`]), and counts the clusters on their way (pvar +0x7c).
//! * **init** (state 0): bob phase `randf(0, π)`, 4 × `randi(255)` hues, 4 × `ticks(90)` timers; a random tilt
//!   (axis `randf(0, 2π)` / `randf(−π/2, π/2)`, angle `randf_sym(π/4, π/2)`) of the whole cluster;
//! * **1** (on the crate): follows the crate; the crate deleted (broken) → **2**;
//! * **2** (free): falls to the ground (9.8·dt², ≤ 0.3; `0x3005a8`), the orbs breathe (radius `0.25 + 0.15·sin`);
//!   when Ratchet has health, is within 10 with a clear line (flags 0x12) and `HP + clusters on their way < max HP`
//!   (4 as Clank) → **3**: the orbs fly `ticks(rand_range(20, 80))` ticks each on a cubic from where they are
//!   to his chest, fanned about the line; each leaves two trail puffs a tick; the **first to arrive heals 1 HP**
//!   (max HP 0x15eda0) with sound 1 of class 501; after the heal the healing ring (type-59 sparkles around him,
//!   `0x300900`) and, 1 in 5 while it is young, two type-60 glints (`0x300808`); all orbs gone after `ticks(300)`
//!   → deleted (the master waits in 4, turning its table);
//! * in 0 / 1 / 2 the cluster bobs `0.06·sin` (2° a tick), and when in view (64) places its orbs and a trail puff
//!   each (life 5 or 40 ticks by camera distance 10), and registers its draw callback `0x301c00` (list 1: the glow
//!   sphere, its halo and, on the crate, the glass sheen; drawn by `rc-engine`'s fx_draw from [`nanotech_glow`]).
//!
//! The cheat 6 (0x15edb6, `crate::cheats`): a cluster is also taken at full health while 0x13f510 is 0, the take sets
//! 0x13f510 = `ticks(600)`, and at full health while invulnerable the flying orbs' life is held at `ticks(250)` and the
//! ring's timers +1. Not ported: the platform ride of a free
//! cluster on a moving moby (kept: the offset bookkeeping).

use crate::hero::physics::{self as ph, V4};
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::services::{self as sv, fv, pv, pvar as p, Inventory, World};
use crate::particles::{rec, type59, type62};
use crate::ps2v::Pf;

/// The ammo pickup update address in the level01 class table.
pub const AMMO_UPDATE_FN: u32 = 0x2db028;
/// The nanotech cluster update address.
pub const NANOTECH_UPDATE_FN: u32 = 0x300de0;
/// The item pickup classes (level01 table: every class mapped to 0x2db028).
pub const AMMO_CLASSES: [i16; 11] = [204, 213, 214, 222, 223, 225, 226, 1006, 1438, 1447, 1449];
/// The nanotech cluster.
pub const NANOTECH_CLASSES: [i16; 1] = [806];
/// The nanotech crate (its sound bank plays the heal).
pub const NANOTECH_CRATE: i16 = 501;
/// The class whose sound 0 is the ammo pickup sound.
pub const PICKUP_SOUND_CLASS: i16 = 213;
const N: usize = rc_formats::save_game::ITEM_COUNT;
const DT: f32 = 1.0 / 60.0;
const DT2: f32 = DT * DT;

fn f3(v: V4) -> [f32; 3] { ph::to_f32x3(v) }
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn scale(a: [f32; 3], k: f32) -> [f32; 3] { [a[0] * k, a[1] * k, a[2] * k] }
fn len(a: [f32; 3]) -> f32 { (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt() }
fn set_len(a: [f32; 3], l: f32) -> [f32; 3] {
    let n = len(a);
    if n == 0.0 { [0.0; 3] } else { scale(a, l / n) }
}
/// `FastVecCross(out, a, b)` 0x2212d0 = `b × a`.
fn cross_ba(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [b[1] * a[2] - b[2] * a[1], b[2] * a[0] - b[0] * a[2], b[0] * a[1] - b[1] * a[0]] }
/// `FUN_00274ac8(angle, out, v, axis)`: `v` turned by `angle` about `axis` (the axis-angle quaternion 0x272090
/// applied by 0x274a38); a (near) zero angle copies `v`.
fn rotate(v: [f32; 3], angle: f32, axis: [f32; 3]) -> [f32; 3] {
    if angle.abs() < 1e-5 { return v; }
    let k = set_len(axis, 1.0);
    let (s, c) = angle.sin_cos();
    let d = k[0] * v[0] + k[1] * v[1] + k[2] * v[2];
    let x = [k[1] * v[2] - k[2] * v[1], k[2] * v[0] - k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
    std::array::from_fn(|i| v[i] * c + x[i] * s + k[i] * d * (1.0 - c))
}
fn add_rot(a: f32, b: f32) -> f32 {
    let mut r = a + b;
    let tau = std::f32::consts::TAU;
    if r > std::f32::consts::PI { r -= tau; }
    if r < -std::f32::consts::PI { r += tau; }
    r
}
/// `approach(x, target, step)` 0x270728 (`crate::hero::physics::approach` on native floats).
fn approach(x: &mut f32, target: f32, step: f32) {
    let d = (target - *x).clamp(-step, step);
    *x += d;
}

// ------------------------------------------------------------------------------------------------
// The shared parts

/// The item tables the pickups read, filled by the engine before the moby loop (the game's globals): owned
/// `0x13d4c0`, ammo `0x13d428` (Ratchet's mirror, [`crate::hero::weapons::Weapons::ammo`]), the records
/// `0x1c4530` (max ammo +0xe, pickup amount +0xc) and the vendor's list `0x15edd0` (the ammo pick's order).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemTables {
    pub owned: [bool; N],
    pub ammo: [i32; N],
    pub max: [u16; N],
    pub amount: [u16; N],
    pub list: [u8; 12],
    /// The item definitions' slot types (+0x08; −1 unknown).
    pub slot: [i32; N],
}

impl Default for ItemTables {
    fn default() -> Self { ItemTables { owned: [false; N], ammo: [0; N], max: [0; N], amount: [0; N], list: [0xff; 12], slot: [-1; N] } }
}

impl ItemTables {
    /// From the price records (`crate::moby_update::interact::ShopTable`, 0x18 bytes each) and the vendor list.
    pub fn new(records: &[[u8; 0x18]], list: [u8; 12]) -> ItemTables {
        let mut t = ItemTables { list, ..Default::default() };
        for (i, r) in records.iter().take(N).enumerate() {
            t.amount[i] = u16::from_le_bytes([r[0xc], r[0xd]]);
            t.max[i] = u16::from_le_bytes([r[0xe], r[0xf]]);
        }
        t
    }

    /// The item definitions' slot types (`crate::inventory::ItemInfos`, +0x08).
    pub fn with_slots(mut self, slots: &[i32]) -> ItemTables {
        for (d, &s) in self.slot.iter_mut().zip(slots) { *d = s; }
        self
    }

    /// This tick's owned items and ammo (Ratchet's mirrors).
    pub fn with_hero(mut self, h: &crate::hero::Hero) -> ItemTables {
        for i in 0..N {
            self.owned[i] = h.owned.0[i] != 0;
            self.ammo[i] = h.weapons.ammo[i];
        }
        self
    }
}

impl Inventory for ItemTables {
    fn owned(&self, item: usize) -> bool { self.owned.get(item).copied().unwrap_or(false) }
    fn ammo(&self, item: usize) -> i32 { self.ammo.get(item).copied().unwrap_or(0) }
    fn max_ammo(&self, item: usize) -> u16 { self.max.get(item).copied().unwrap_or(0) }
    fn pickup_amount(&self, item: usize) -> u16 { self.amount.get(item).copied().unwrap_or(0) }
    fn ammo_list(&self) -> [u8; 12] { self.list }
    fn slot_type(&self, item: usize) -> i32 { self.slot.get(item).copied().unwrap_or(-1) }
}

/// `0x13d428[item]` as this tick's earlier class updates left it (a pickup earlier in the loop counts).
pub fn item_ammo(w: &World, item: usize) -> i32 {
    match w.svc.hero_writes {
        Some((c, f)) if c == w.counter => f.ammo.get(item).copied().unwrap_or(0),
        _ => w.inventory.ammo(item),
    }
}

/// `AddAmmo(item, n)` 0x2494d8: ammo += n, clamped to the record's max ammo (when it has one); returns the
/// overflow. Written through the hero-block writes ([`crate::moby_update::services::HeroFields::ammo`]).
pub fn add_ammo(w: &mut World, item: usize, n: i32) -> i32 {
    if item >= N { return 0; }
    let max = w.inventory.max_ammo(item) as i32;
    let old = item_ammo(w, item);
    let mut v = old + n;
    let mut over = 0;
    if max != 0 && v - max > 0 {
        over = v - max;
        v = max;
    }
    w.hero_fields_mut().ammo[item] = v;
    over
}

/// Ratchet's pickup volume (bolts 0x2bb758 and ammo 0x2db028): |dz| ≤ `0x1415dc` and xy distance ≤ `0x1415d8`.
pub fn in_reach(w: &World, pos: [f32; 3]) -> bool {
    let (rx, rz) = w.bolt_radii();
    let h = f3(w.hero.pos);
    if (pos[2] - h[2]).abs() > rz.to_f32() { return false; }
    let d = ((pos[0] - h[0]).powi(2) + (pos[1] - h[1]).powi(2)).sqrt();
    d <= rx.to_f32()
}

/// `FUN_00265210(a, b)`: the gap between two mobys' bounding spheres (+0x00, stored ×1024), negative when they
/// overlap.
pub fn sphere_gap(w: &World, a: MobyId, b: MobyId) -> f32 {
    let (sa, sb) = (w.m(a).bsphere, w.m(b).bsphere);
    let d = len([sa[0] - sb[0], sa[1] - sb[1], sa[2] - sb[2]]);
    d / 1024.0 - (sa[3] + sb[3]) / 1024.0
}

/// Max health as the pickups read it: 0x15eda0, or 4 when Clank is the body (0x1413f4 = 1).
pub fn max_health(w: &World) -> i32 { if w.body() == 1 { 4 } else { w.svc.counters.max_hp } }

/// `0x2daf10(pos, item, amount, a3)`: an item pickup moby of `item`'s pickup class (none for a class ≤ 0),
/// update and draw distance 0xff, state 1, drawn, the amount (−1: the record's pickup amount), the item, `a3`, the
/// pickup delay `ticks(16)` and the spawn z; the hero's light (`FUN_00272078`).
pub fn pickup_spawn(w: &mut World, pos: V4, ty: i32, amount: i32, a3: i32) -> Option<MobyId> {
    let class = w.inventory.pickup_class(ty);
    if class <= 0 { return None; }
    let m = w.create_moby(class)?;
    w.svc.fx.pickups += 1;
    let amt = if amount != -1 { amount } else { w.inventory.pickup_amount(ty.max(0) as usize) as i32 };
    let t16 = w.ticks(0x10);
    let hero_light = w.hero_moby.map(|h| (w.m(h).light, w.m(h).ambient));
    {
        let mo = w.mm(m);
        if mo.pvars.len() < 0x30 { mo.pvars.resize(0x30, 0); }
        mo.update_dist = 0xff;
        mo.draw_dist = 0xff;
        mo.state = 1;
        mo.visible = 1;
        mo.cmd = 0;
        mo.position = fv(pos);
        p::set_i32(&mut mo.pvars, 0, amt);
        p::set_i32(&mut mo.pvars, 8, ty);
        p::set_f(&mut mo.pvars, 0xc, Pf::from_i32(a3));
        p::set_i32(&mut mo.pvars, 0x20, t16);
        let z = sv::pf(mo.position[2]);
        p::set_f(&mut mo.pvars, 0x28, z);
    }
    w.build_matrix(m);
    if let Some((l, a)) = hero_light {
        let mo = w.mm(m);
        mo.light = l;
        mo.ambient = a;
    }
    Some(m)
}

/// `0x26bff8(m, &type)`: keeps `type` unless it is ≤ 0 or the dropper's death bit is set; else picks an owned
/// ammo item of the vendor list, weighted to those below max (none: one with a max; nothing owned: 10). Returns
/// the pickup count, `randi(5) ≠ 0 ? 1 : 2`.
pub fn ammo_pick(w: &mut World, id: Option<MobyId>, ty: &mut i32) -> i32 {
    let repick = if *ty > 0 {
        match id {
            None => false,
            Some(i) => { let s = w.m(i).spawn_id; w.svc.save.death.contains(&(w.svc.level, s)) }
        }
    } else {
        true
    };
    if repick {
        let list = w.inventory.ammo_list();
        let owned = |w: &World, e: usize| w.inventory.owned(e);
        let max = |w: &World, e: usize| w.inventory.max_ammo(e) as i32;
        let (mut t1, mut t2, mut t3) = (0, 0, 0);
        for &e in list.iter().take_while(|&&e| e != 0xff) {
            let i = (e & 0x3f) as usize;
            if !owned(w, i) { continue; }
            t3 += 1;
            if max(w, i) != 0 { t1 += 1; }
            if item_ammo(w, i) < max(w, i) { t2 += 1; }
        }
        let pick = |w: &mut World, n: i32, low: bool| -> i32 {
            let mut r = w.rng.randi(n);
            let mut s = 0usize;
            loop {
                let e = (list.get(s).copied().unwrap_or(0xff) & 0x3f) as usize;
                if owned(w, e) {
                    let hit = if low { item_ammo(w, e) < max(w, e) } else { max(w, e) != 0 };
                    r -= hit as i32;
                }
                if r < 0 { break; }
                s += 1;
            }
            (list.get(s).copied().unwrap_or(0) & 0x3f) as i32
        };
        *ty = if t2 != 0 {
            pick(w, t2, true)
        } else if t3 == 0 {
            10
        } else {
            pick(w, t1, false)
        };
    }
    if w.rng.randi(5) != 0 { 1 } else { 2 }
}

/// The level's pickup globals (the ammo pickup sound's rate limit, the nanotech master and its direction table).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Globals {
    /// gp−0x5398 (0x161868): tick of the last ammo pickup sound.
    pub last_sound: i32,
    /// 0x15f63c: the master nanotech cluster.
    pub master: Option<MobyId>,
    /// 0x1fe600: the 8 orbit directions (0.25 long).
    pub dirs: [[f32; 3]; 8],
    /// 0x1fe680: the axis each direction turns about.
    pub axes: [[f32; 3]; 8],
}

/// A HUD banner request (`ShowBannerf(text, n, −1)` 0x278a50: `sprintf(msg(text), n)`, 180 ticks); the HUD shows
/// each new `seq`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Banner {
    pub seq: u32,
    pub text: i32,
    pub arg: i32,
}

// ------------------------------------------------------------------------------------------------
// Ammo pickups 0x2db028

/// `AmmoPickupUpdate` (0x2db028).
pub fn ammo_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x30 { w.mm(id).pvars.resize(0x30, 0); }
    {
        let pv_ = &mut w.mm(id).pvars;
        let t = p::i32(pv_, 0x20);
        if t != 0 { p::set_i32(pv_, 0x20, t.max(1) - 1); }
    }
    let st = w.m(id).state;
    if matches!(st, 1 | 3 | 4) { physics(w, id); }
    match w.m(id).state {
        0 => w.mm(id).state = 1,
        1 => idle(w, id),
        2 => collected(w, id),
        3 => dim(w, id),
        4 => grow(w, id),
        _ => {}
    }
}

fn vel(w: &World, id: MobyId) -> [f32; 3] { let v = p::v4f(&w.m(id).pvars, 0x10); [v[0], v[1], v[2]] }
fn set_vel(w: &mut World, id: MobyId, v: [f32; 3]) { let w4 = p::ff(&w.m(id).pvars, 0x1c); p::set_v4f(&mut w.mm(id).pvars, 0x10, [v[0], v[1], v[2], w4]); }
fn pos3(w: &World, id: MobyId) -> [f32; 3] { let q = w.m(id).position; [q[0], q[1], q[2]] }
fn set_pos3(w: &mut World, id: MobyId, q: [f32; 3]) { let m = w.mm(id); m.position[0] = q[0]; m.position[1] = q[1]; m.position[2] = q[2]; }
fn v4(a: [f32; 3]) -> V4 { ph::from_f32x3(a) }

/// The fall of states 1 / 3 / 4.
fn physics(w: &mut World, id: MobyId) {
    let mut v = vel(w, id);
    if len(v) != 0.0 {
        let mut c = pos3(w, id);
        c[2] += 0.5;
        if let Some(h) = w.coll_sphere(v4(c), Pf::f(0.4), 2, None) {
            if let Some(pc) = h.pushed_centre { set_pos3(w, id, [pc[0], pc[1], pc[2] - 0.5]); }
        }
    }
    v[2] -= DT2 * 4.9;
    let pos = pos3(w, id);
    let next = add(pos, v);
    v[2] -= DT2 * 4.9;
    set_vel(w, id, v);
    let from = [pos[0], pos[1], pos[2] + 0.1];
    match w.line(v4(from), v4(next), 2, None) {
        None => set_pos3(w, id, next),
        Some(h) => {
            set_vel(w, id, [0.0; 3]);
            set_pos3(w, id, f3(h.point));
        }
    }
}

fn alpha_toward(w: &mut World, id: MobyId, target: f32) {
    let step = 128.0 / w.ticks(0x1e) as f32;
    let mut a = w.m(id).alpha as f32;
    approach(&mut a, target, step);
    w.mm(id).alpha = a as i32 as u8;
}

/// `0x2db850(pickup)`: collected when `AddAmmo` raised the ammo: the banner, the picked-up stat, state 2 (flying to
/// Ratchet), scale ×1.2, the rise `8·dt`, sound 0 of class 213 at most every `ticks(10)`. Also the Suck Cannon's
/// vacuum's take (`crate::hero::suck_cannon::vacuum`). True when taken; with the ammo already full (or the pickup
/// already flying) nothing happens and it stays.
pub fn collect(w: &mut World, id: MobyId) -> bool {
    if w.m(id).state == 2 { return false; }
    let item = p::i32(&w.m(id).pvars, 8);
    let i = item.clamp(0, N as i32 - 1) as usize;
    let old = item_ammo(w, i);
    let amount = p::i32(&w.m(id).pvars, 0);
    let over = add_ammo(w, i, amount);
    let new = item_ammo(w, i);
    if new <= old { return false; }
    let n = new - old;
    let text = w.inventory.pickup_text(i, n == 1);
    let b = &mut w.svc.pickups_banner;
    *b = Banner { seq: b.seq.wrapping_add(1), text, arg: n };
    // `(float)overflow < (float)amount`: always, once the ammo rose.
    if over >= amount { return true; }
    w.hero_fields_mut().ammo_picked[i] += n;
    {
        let m = w.mm(id);
        m.state = 2;
        m.scale *= 0.2 + 1.0;
    }
    set_vel(w, id, [0.0; 3]);
    p::set_ff(&mut w.mm(id).pvars, 0x24, DT * 8.0);
    let c = w.counter as i32;
    if (c - w.svc.pickups.last_sound).abs() > w.ticks(10) {
        w.play_sound_as(0, 0, id, PICKUP_SOUND_CLASS);
        w.svc.pickups.last_sound = c;
    }
    true
}

/// State 1.
fn idle(w: &mut World, id: MobyId) {
    alpha_toward(w, id, 128.0);
    if p::i32(&w.m(id).pvars, 0x20) > 0 { return; }
    if len(vel(w, id)) != 0.0 { return; }
    let pos = pos3(w, id);
    let item = p::i32(&w.m(id).pvars, 8);
    let i = item.clamp(0, N as i32 - 1) as usize;
    let old = item_ammo(w, i);
    let below_max = old < w.inventory.max_ammo(i) as i32;
    let mut take = in_reach(w, pos) && below_max;
    if pos[2] < p::ff(&w.m(id).pvars, 0x28) - 2.0 { take = true; }
    if w.hero_fields().health == 0 { take = false; }
    if take {
        collect(w, id);
        return;
    }
    {
        let m = w.mm(id);
        m.alpha = m.alpha.saturating_add(1);
    }
    if let Some(h) = w.hero_moby {
        if sphere_gap(w, id, h) <= -0.2 {
            let t = w.ticks(0x3c);
            let m = w.mm(id);
            m.state = 3;
            p::set_i16(&mut m.pvars, 6, t as i16);
        }
    }
}

/// State 2: the flight to Ratchet.
fn collected(w: &mut World, id: MobyId) {
    let mut rise = p::ff(&w.m(id).pvars, 0x24);
    approach(&mut rise, 0.0, DT2 * 24.0);
    p::set_ff(&mut w.mm(id).pvars, 0x24, rise);
    let mut pos = pos3(w, id);
    pos[2] += rise;
    set_pos3(w, id, pos);
    let mut speed = len(vel(w, id));
    approach(&mut speed, DT * 64.0, DT2 * 24.0);
    let body = f3(w.hero.body_point);
    let to = sub(body, pos);
    set_vel(w, id, to);
    if w.m(id).scale == 0.0 || len(to) < speed {
        w.delete_moby(id);
        return;
    }
    let step = set_len(to, speed);
    set_vel(w, id, step);
    let pos = add(pos, step);
    set_pos3(w, id, pos);
    let d = len(sub(pos, body));
    let k = (d / (w.ticks(0x1e) as f32 * speed + 0.01)).clamp(0.3, 1.0);
    let cs = w.class_scale(w.m(id).o_class).to_f32();
    let mut s = w.m(id).scale;
    approach(&mut s, cs * k, cs * 0.1);
    w.mm(id).scale = s;
}

/// State 3: full ammo while Ratchet touches it.
fn dim(w: &mut World, id: MobyId) {
    let ph_ = (w.counter % 60) as f32 / 60.0 * std::f32::consts::TAU - std::f32::consts::PI;
    let target = (ph_.sin() * 5.0 + 5.0) as i32 + 10;
    alpha_toward(w, id, target as f32);
    let gap = w.hero_moby.map(|h| sphere_gap(w, id, h)).unwrap_or(1.0);
    let item = p::i32(&w.m(id).pvars, 8).clamp(0, N as i32 - 1) as usize;
    if 0.0 <= gap || item_ammo(w, item) < w.inventory.max_ammo(item) as i32 { w.mm(id).state = 1; }
}

/// State 4: grow to the class scale.
fn grow(w: &mut World, id: MobyId) {
    let o_class = w.m(id).o_class;
    let cs = w.class_scale(o_class).to_f32();
    let s = w.m(id).scale + (cs - w.m(id).scale) * 0.03;
    w.mm(id).scale = s;
    if cs != 0.0 && (cs - s) / cs < 0.01 {
        let coll = w.classes.info(o_class).map(|c| c.has_collision).unwrap_or(false);
        let m = w.mm(id);
        m.scale = cs;
        m.state = 1;
        m.has_collision = coll;
    }
}

// ------------------------------------------------------------------------------------------------
// The nanotech cluster 0x300de0

mod nt {
    //! Pvar offsets of class 806.
    pub const BOB: usize = 0x00;
    pub const PHASE: usize = 0x04;
    pub const ON_PLATFORM: usize = 0x0a;
    pub const CRATE: usize = 0x0c;
    pub const CENTRE: usize = 0x10;
    pub const AXIS: usize = 0x20;
    pub const TILT: usize = 0x2c;
    pub const ORBS: usize = 0x30;
    pub const FALL: usize = 0x50;
    pub const TIMER: usize = 0x54;
    pub const HEALED: usize = 0x56;
    pub const RING_T: usize = 0x58;
    pub const HUE: usize = 0x60;
    pub const PLAT_OFF: usize = 0x70;
    pub const PENDING: usize = 0x7c;
    pub const SIZE: usize = 0x80;
}

/// Orb record fields (type 62 kind 1) the cluster keeps: +0x28 radius, +0x2c breathing phase (state 2); +0x24 the
/// flight's control offset (over those), +0x30 start, +0x3c / +0x3e flight ticks / left.
mod orb {
    pub const RADIUS: usize = 0x28;
    pub const BREATH: usize = 0x2c;
    pub const CTRL: usize = 0x24;
    pub const START: usize = 0x30;
    pub const TICKS: usize = 0x3c;
    pub const LEFT: usize = 0x3e;
}

/// `0x1fbe00`: the orbs' breathing steps (4°, 2°, 6°, 3°, 1°, 5°, 2.5°, 4.5°).
const BREATH_STEP: [u32; 8] = [0x3d8e_fa35, 0x3d0e_fa35, 0x3dd6_77d0, 0x3d56_77d0, 0x3c8e_fa35, 0x3db2_b8c2, 0x3d32_b8c2, 0x3da0_d97c];
/// The orbs' colour (gp−0x4e90) and sizes (orb gp−0x4e94, trail gp−0x4e98).
const ORB_RGBA: u32 = 0x7f7f_4040;
const ORB_SIZE: f32 = 60000.0;
const TRAIL_SIZE: f32 = 10000.0;
/// gp−0x4ea0: the master's turn per tick (4°).
const TURN: f32 = f32::from_bits(0x3d8e_fa35);
/// The healing ring (`0x300900`): hue speeds gp−0x4e00, sizes gp−0x4df0, heights gp−0x4de0; colours gp−0x4e14 /
/// gp−0x4e10; the texture type gp−0x4e08 (53).
const RING_HUE: [f32; 4] = [-2.0, -4.25, 1.25, 3.5];
const RING_SIZE: [f32; 4] = [4.0, 5.0, 3.0, 6.0];
const RING_Z: [f32; 4] = [-0.25, -0.1, 0.15, 0.3];
const RING_A: u32 = 0x407f_2020;
const RING_B: u32 = 0x107f_2020;

fn ptr(pv_: &[u8], o: usize) -> Option<usize> { let v = p::i32(pv_, o); (v > 0).then(|| (v - 1) as usize) }
fn set_ptr(pv_: &mut [u8], o: usize, id: Option<usize>) { p::set_i32(pv_, o, id.map(|i| i as i32 + 1).unwrap_or(0)); }
fn v3p(pv_: &[u8], o: usize) -> [f32; 3] { [p::ff(pv_, o), p::ff(pv_, o + 4), p::ff(pv_, o + 8)] }
fn set_v3p(pv_: &mut [u8], o: usize, v: [f32; 3]) { for (k, x) in v.iter().enumerate() { p::set_ff(pv_, o + 4 * k, *x); } }

/// A type-62 record (see [`type62::spawn`]); counted like the other spawners; no draw without a particle system.
fn part62(w: &mut World, pos: [f32; 3], life: i32, kind: i16) -> Option<usize> {
    let rng = &mut *w.rng;
    let sys = w.particles.as_deref_mut()?;
    let r = type62::spawn(sys, rng, [pos[0], pos[1], pos[2], 0.0], life, kind);
    if r.is_none() { w.svc.fx.part_failed += 1; }
    *w.svc.fx.part_spawns.entry(type62::TYPE).or_default() += 1;
    r
}
fn orb_rec<R>(w: &mut World, i: usize, f: impl FnOnce(&mut crate::particles::Record) -> R) -> Option<R> {
    w.particles.as_deref_mut().map(|s| f(&mut s.pool.recs[i]))
}
fn kill_orb(w: &mut World, i: usize) {
    if let Some(s) = w.particles.as_deref_mut() {
        if s.pool.recs[i][1] & crate::particles::FLAG_DEAD == 0 { s.kill_part(i); }
    }
}

/// `NanotechUpdate` (0x300de0).
pub fn nanotech_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < nt::SIZE { w.mm(id).pvars.resize(nt::SIZE, 0); }
    match w.m(id).state {
        0 => {
            nt_init(w, id);
            nt_follow(w, id);
            nt_common(w, id);
        }
        1 => {
            nt_follow(w, id);
            nt_common(w, id);
        }
        2 => nt_common(w, id),
        3 => nt_heal(w, id),
        4 if w.svc.pickups.master == Some(id) => turn_dirs(w),
        _ => {}
    }
}

/// What the glow callback `0x301c00` reads of a cluster: its position (moby +0x10), the bob (pvar +0x00), the state
/// (1 = on the crate: the glass sheen is drawn) and its crate's rotation rows (the crate moby's +0xc0 / +0xd0 /
/// +0xe0; the cluster's own rotation, which is the crate's from the init, when the crate is gone).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NanotechGlow {
    pub pos: [f32; 3],
    pub bob: f32,
    pub on_crate: bool,
    pub crate_rows: [[f32; 3]; 3],
    pub crate_pos: [f32; 3],
}

/// The glow callback's inputs for cluster `id` (None when it is not a live 806).
pub fn nanotech_glow(t: &crate::moby_runtime::MobyTable, id: MobyId) -> Option<NanotechGlow> {
    let m = t.mobys.get(id).filter(|m| m.state < 0x80 && m.pvars.len() >= nt::SIZE)?;
    let c = ptr(&m.pvars, nt::CRATE).and_then(|c| t.mobys.get(c)).unwrap_or(m);
    let r = sv::euler_rows(pv(c.rotation));
    Some(NanotechGlow {
        pos: [m.position[0], m.position[1], m.position[2]],
        bob: p::ff(&m.pvars, nt::BOB),
        on_crate: m.state == 1,
        crate_rows: [f3(r[0]), f3(r[1]), f3(r[2])],
        crate_pos: [c.position[0], c.position[1], c.position[2]],
    })
}

/// State 0.
fn nt_init(w: &mut World, id: MobyId) {
    let crate_id = ptr(&w.m(id).pvars, nt::CRATE).unwrap_or(id);
    let (cpos, crot) = { let c = w.m(crate_id); (c.position, c.rotation) };
    {
        let m = w.mm(id);
        m.state = 1;
        m.has_collision = false;
        m.mode |= 0x41;
        m.cmd = 0;
        m.position = cpos;
        m.rotation = crot;
        let pv_ = &mut m.pvars;
        set_v3p(pv_, nt::CENTRE, [cpos[0], cpos[1], cpos[2] + 0.5]);
        p::set_ff(pv_, nt::BOB, 0.0);
        p::set_i16(pv_, 8, 0);
    }
    let phase = w.rng.randf(0.0, std::f32::consts::PI);
    {
        let pv_ = &mut w.mm(id).pvars;
        p::set_ff(pv_, nt::PHASE, phase);
        p::set_i16(pv_, nt::HEALED, 0);
        p::set_i16(pv_, nt::ON_PLATFORM, 0);
    }
    for k in 0..4 {
        let h = w.rng.randi(0xff) as f32;
        p::set_ff(&mut w.mm(id).pvars, nt::HUE + 4 * k, h);
    }
    let t90 = w.ticks(90);
    for k in 0..4 { p::set_i16(&mut w.mm(id).pvars, nt::RING_T + 2 * k, t90 as i16); }
    for k in 0..8 { p::set_i32(&mut w.mm(id).pvars, nt::ORBS + 4 * k, 0); }
    if w.svc.pickups.master.is_none() {
        w.svc.pickups.master = Some(id);
        w.mm(id).update_dist = 0xff;
        p::set_i32(&mut w.mm(id).pvars, nt::PENDING, 0);
        let r = sv::euler_rows(pv(w.m(id).rotation));
        let (r0, r1, r2) = (f3(r[0]), f3(r[1]), f3(r[2]));
        let dirs = [r0, r1, r2, add(r0, r2), add(r0, r1), sub(r1, r2), sub(r0, r2), sub(r0, r1)].map(|d| set_len(d, 0.25));
        let g = &mut w.svc.pickups;
        g.dirs = dirs;
        g.axes[0] = r2;
        g.axes[1] = r0;
        g.axes[2] = r1;
        for k in 3..8 {
            let other = [r1, r2, r0, r1, r2][k - 3];
            g.axes[k] = cross_ba(dirs[k], other);
        }
    }
    let yaw = w.rng.randf(0.0, std::f32::consts::TAU);
    let pitch = w.rng.randf(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
    let axis = [yaw.cos() * pitch.cos(), yaw.sin() * pitch.cos(), pitch.sin()];
    let tilt = w.rng.randf_sym(std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_2);
    let pv_ = &mut w.mm(id).pvars;
    set_v3p(pv_, nt::AXIS, axis);
    p::set_ff(pv_, nt::TILT, tilt);
}

/// `FUN_00300cf8` (the master's per-tick turn of the direction table).
fn turn_dirs(w: &mut World) {
    let g = &mut w.svc.pickups;
    for k in 0..8 { g.dirs[k] = set_len(rotate(g.dirs[k], TURN, g.axes[k]), 0.25); }
}

/// State 1's part: on the crate (the crate deleted → state 2).
fn nt_follow(w: &mut World, id: MobyId) {
    let Some(c) = ptr(&w.m(id).pvars, nt::CRATE) else { return };
    let cpos = w.m(c).position;
    {
        let m = w.mm(id);
        m.position = cpos;
        set_v3p(&mut m.pvars, nt::CENTRE, [cpos[0], cpos[1], cpos[2] + 0.5]);
    }
    if w.m(c).state >= 0x80 || c == id {
        let m = w.mm(id);
        m.state = 2;
        p::set_ff(&mut m.pvars, nt::FALL, 0.0);
    }
}

/// `FUN_00300c40(m, out, i)`: orb `i`'s place.
fn orb_pos(w: &World, id: MobyId, i: usize) -> [f32; 3] {
    let pv_ = &w.m(id).pvars;
    let mut v = rotate(w.svc.pickups.dirs[i], p::ff(pv_, nt::TILT), v3p(pv_, nt::AXIS));
    let o = ptr(pv_, nt::ORBS + 4 * i);
    if let (Some(o), 2) = (o, w.m(id).state) {
        if let Some(s) = w.particles.as_deref() { v = set_len(v, rec::ff(&s.pool.recs[o], orb::RADIUS)); }
    }
    let mut q = add(v3p(pv_, nt::CENTRE), v);
    q[2] += p::ff(pv_, nt::BOB);
    q
}

/// The states 0 / 1 / 2 (after their own part): the free fall and breathing (2), the heal check, the bob, the
/// master's turn and the orbs.
fn nt_common(w: &mut World, id: MobyId) {
    if w.m(id).state == 2 {
        let pos = pos3(w, id);
        set_v3p(&mut w.mm(id).pvars, nt::CENTRE, [pos[0], pos[1], pos[2] + 0.5]);
        nt_fall(w, id);
        for (k, step) in BREATH_STEP.iter().enumerate() {
            let Some(o) = ptr(&w.m(id).pvars, nt::ORBS + 4 * k) else { continue };
            orb_rec(w, o, |r| {
                let ph_ = rec::ff(r, orb::BREATH);
                rec::set_ff(r, orb::RADIUS, 0.15 * ph_.sin() + 0.25);
                rec::set_ff(r, orb::BREATH, add_rot(ph_, f32::from_bits(*step)));
            });
        }
    }
    nt_heal_check(w, id);
    {
        let pv_ = &mut w.mm(id).pvars;
        let ph_ = p::ff(pv_, nt::PHASE);
        p::set_ff(pv_, nt::BOB, 0.06 * ph_.sin());
        p::set_ff(pv_, nt::PHASE, add_rot(ph_, f32::from_bits(0x3d0e_fa35)));
    }
    if w.svc.pickups.master == Some(id) { turn_dirs(w); }
    let pos = pos3(w, id);
    let sphere = [pos[0], pos[1], pos[2] + 0.5, 1.0];
    if w.view.map(|v| v.culled(64.0, sphere)).unwrap_or(false) { return; }
    w.svc.draw_callbacks.register(super::draw_callbacks::Callback::NanotechGlow, id);
    let cam = f3(w.camera);
    let life = w.ticks(if len(sub(pos, cam)) > 10.0 { 5 } else { 0x28 });
    let anchor = if w.m(id).state == 2 { id } else { ptr(&w.m(id).pvars, nt::CRATE).unwrap_or(id) };
    if let Some(s) = w.particles.as_deref_mut() {
        s.anchors.insert(anchor, pos);
    }
    for k in 0..8 {
        let q = orb_pos(w, id, k);
        if let Some(t) = part62(w, q, life, 2) {
            orb_rec(w, t, |r| {
                type62::attach(r, anchor, pos);
                rec::set_u32(r, 4, ORB_RGBA);
                rec::set_ff(r, 0xc, TRAIL_SIZE);
            });
        }
        match ptr(&w.m(id).pvars, nt::ORBS + 4 * k) {
            None => {
                let o = part62(w, q, 0, 1);
                if let Some(o) = o {
                    orb_rec(w, o, |r| {
                        rec::set_ff(r, orb::BREATH, 0.0);
                        rec::set_ff(r, orb::RADIUS, 0.25);
                        rec::set_ff(r, 0xc, ORB_SIZE);
                        rec::set_u32(r, 4, ORB_RGBA);
                    });
                }
                set_ptr(&mut w.mm(id).pvars, nt::ORBS + 4 * k, o);
            }
            Some(o) => {
                orb_rec(w, o, |r| {
                    rec::set_v3(r, 0x10, q);
                    r[8] = r[8].wrapping_add(4);
                });
            }
        }
    }
}

/// `FUN_003005a8`: the free cluster falls onto what is below it (the ground, a crate top, or a moving moby it
/// then rides).
fn nt_fall(w: &mut World, id: MobyId) {
    let pos = pos3(w, id);
    let a = [pos[0], pos[1], pos[2] + 0.5];
    let b = [pos[0], pos[1], pos[2] - 1.0];
    let mut floor = b[2];
    let mut z = pos[2];
    if let Some(h) = w.line(v4(a), v4(b), 2, Some(id)) {
        let hp = f3(h.point);
        match h.moby {
            None => floor = hp[2],
            Some(mo) => {
                let is_crate = (w.m(mo).o_class as i32 - 500) as u32 & 0xffff < 0x29;
                let mp = pos3(w, mo);
                if !is_crate {
                    let pv_ = &mut w.mm(id).pvars;
                    if p::i16(pv_, nt::ON_PLATFORM) == 0 {
                        p::set_i16(pv_, nt::ON_PLATFORM, 1);
                        set_v3p(pv_, nt::PLAT_OFF, sub(pos, mp));
                    } else {
                        let off = v3p(pv_, nt::PLAT_OFF);
                        let q = add(mp, off);
                        set_v3p(pv_, nt::CENTRE, [q[0], q[1], q[2] + 0.5]);
                        set_pos3(w, id, q);
                    }
                } else {
                    let q = [mp[0], mp[1], hp[2]];
                    set_pos3(w, id, q);
                    set_v3p(&mut w.mm(id).pvars, nt::CENTRE, [q[0], q[1], q[2] + 0.5]);
                }
                floor = hp[2];
                z = w.m(id).position[2];
            }
        }
    }
    let plat = p::i16(&w.m(id).pvars, nt::ON_PLATFORM) != 0;
    if z <= floor {
        w.mm(id).cmd = 0;
        if plat {
            let pv_ = &mut w.mm(id).pvars;
            let oz = p::ff(pv_, nt::PLAT_OFF + 8);
            p::set_ff(pv_, nt::PLAT_OFF + 8, oz + (floor - z));
        }
        w.mm(id).position[2] = floor;
        p::set_ff(&mut w.mm(id).pvars, nt::FALL, 0.0);
    } else {
        let v = p::ff(&w.m(id).pvars, nt::FALL);
        let m = w.mm(id);
        m.cmd = 1;
        m.position[2] = z - v;
        let v = (v + DT2 * 9.8).min(0.3);
        p::set_ff(&mut m.pvars, nt::FALL, v);
        if plat {
            let oz = p::ff(&m.pvars, nt::PLAT_OFF + 8);
            p::set_ff(&mut m.pvars, nt::PLAT_OFF + 8, oz - v);
        }
    }
}

/// Ratchet's chest (`0x13f3d0 − 0.75 · gravity 0x13f5e0`).
fn chest(w: &World) -> [f32; 3] { let h = f3(w.hero.pos); [h[0], h[1], h[2] + 0.75] }

/// The heal check of the states 0 / 1 / 2 → state 3 and the orbs' flight paths.
fn nt_heal_check(w: &mut World, id: MobyId) {
    if w.m(id).state == 1 { return; }
    let Some(master) = w.svc.pickups.master.filter(|&m| w.m(m).pvars.len() >= nt::SIZE) else { return };
    let hp = w.hero_fields().health;
    if hp == 0 { return; }
    let pending = p::i32(&w.m(master).pvars, nt::PENDING);
    // The cheat 6 (0x15edb6, "Health gives invincibility at max"): also taken at full health while 0x13f510 is 0.
    let cheat = w.svc.cheats.on(crate::cheats::slot::HEALTH);
    let f510 = w.hero_fields().invulnerable.unwrap_or(w.hero.f510);
    if !(hp + pending < max_health(w) || (cheat && f510 == 0)) { return; }
    let pos = pos3(w, id);
    if len(sub(pos, f3(w.hero.pos))) >= 10.0 { return; }
    // FUN_00300d70: a clear line (flags 0x12) from 0.5 above the cluster to 0.5 above Ratchet.
    let hp_ = f3(w.hero.pos);
    if w.line(v4([pos[0], pos[1], pos[2] + 0.5]), v4([hp_[0], hp_[1], hp_[2] + 0.5]), 0x12, None).is_some() { return; }
    w.mm(id).state = 3;
    let n = p::i32(&w.m(master).pvars, nt::PENDING) + 1;
    p::set_i32(&mut w.mm(master).pvars, nt::PENDING, n);
    // The cheat 6: 0x13f510 = ticks(600) (ten seconds of invulnerability).
    if cheat { w.hero_fields_mut().invulnerable = Some(w.ticks(600)); }
    let t300 = w.ticks(300);
    p::set_i16(&mut w.mm(id).pvars, nt::TIMER, t300 as i16);
    let target = chest(w);
    let up5 = [0.0, 0.0, 5.0];
    let mut d = sub(target, pos);
    d[2] -= 0.5;
    let c = cross_ba(d, up5);
    let mut fan = cross_ba(c, d);
    fan = rotate(fan, -std::f32::consts::FRAC_PI_2, d);
    fan = set_len(fan, 5.0);
    let half = scale(d, 0.5);
    for k in 0..8 {
        let Some(o) = ptr(&w.m(id).pvars, nt::ORBS + 4 * k) else { continue };
        let r = rotate(fan, k as f32 * 0.448_549_63, d);
        let ctrl = add(half, r);
        let lo = w.ticks(0x14);
        let hi = w.ticks(0x50);
        let n = w.rng.rand_range(lo, hi);
        let dur = w.ticks(n);
        orb_rec(w, o, |rr| {
            rec::set_v3(rr, orb::CTRL, ctrl);
            let at = rec::pos(rr);
            rec::set_v3(rr, orb::START, at);
            rec::set_i16(rr, orb::TICKS, dur as i16);
            rec::set_i16(rr, orb::LEFT, dur as i16);
        });
    }
}

/// `FUN_0026cc00(a, b, c, d, t)`: the cubic from `b` (t = 0) to `c` (t = 1).
fn cubic(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    let f = (d - c) - (a - b);
    f * t * t * t + ((a - b) - f) * t * t + (c - a) * t + b
}

/// State 3: the orbs fly to Ratchet; the first to arrive heals.
fn nt_heal(w: &mut World, id: MobyId) {
    if w.svc.pickups.master == Some(id) { turn_dirs(w); }
    if p::i16(&w.m(id).pvars, nt::HEALED) != 0 {
        let c = chest(w);
        set_pos3(w, id, c);
        if heal_ring(w, id) && w.rng.randi(5) == 0 { heal_glints(w, id); }
    }
    // The cheat 6 at full health and invulnerable: the orbs' life held at ticks(250) (gp−0x4e50) and the ring's four
    // timers +1 (they stay young).
    let f510 = w.hero_fields().invulnerable.unwrap_or(w.hero.f510);
    if w.hero_fields().health == max_health(w) && f510 != 0 && w.svc.cheats.on(crate::cheats::slot::HEALTH) {
        let hold = w.ticks(250) as i16;
        if p::i16(&w.m(id).pvars, nt::TIMER) < hold {
            let pv_ = &mut w.mm(id).pvars;
            p::set_i16(pv_, nt::TIMER, hold);
            for k in 0..4 {
                let v = p::i16(pv_, nt::RING_T + 2 * k);
                p::set_i16(pv_, nt::RING_T + 2 * k, v.wrapping_add(1));
            }
        }
    }
    let mut t = p::i16(&w.m(id).pvars, nt::TIMER);
    let running = sv::fast_dec_timer_s16(&mut t) == 0;
    p::set_i16(&mut w.mm(id).pvars, nt::TIMER, t);
    if !running {
        for k in 0..8 {
            if let Some(o) = ptr(&w.m(id).pvars, nt::ORBS + 4 * k) { kill_orb(w, o); }
            set_ptr(&mut w.mm(id).pvars, nt::ORBS + 4 * k, None);
        }
        if w.svc.pickups.master == Some(id) { w.mm(id).state = 4; } else { w.delete_moby(id); }
        return;
    }
    let target = chest(w);
    for k in 0..8 {
        let Some(o) = ptr(&w.m(id).pvars, nt::ORBS + 4 * k) else { continue };
        let Some((left, n, start, ctrl, prev)) = orb_rec(w, o, |r| {
            let mut l = rec::i16(r, orb::LEFT);
            let res = sv::fast_dec_timer_s16(&mut l);
            rec::set_i16(r, orb::LEFT, l);
            (res, rec::i16(r, orb::TICKS), rec::v3(r, orb::START), rec::v3(r, orb::CTRL), rec::pos(r))
        }) else {
            continue;
        };
        if left == 0 {
            let l = orb_rec(w, o, |r| rec::i16(r, orb::LEFT)).unwrap_or(0);
            let tt = 1.0 - l as f32 / n as f32;
            let q: [f32; 3] = std::array::from_fn(|i| cubic(start[i] - ctrl[i], start[i], target[i], target[i] - ctrl[i], tt));
            orb_rec(w, o, |r| rec::set_v3(r, 0x10, q));
            for at in [q, scale(add(prev, q), 0.5)] {
                let j: [f32; 3] = std::array::from_fn(|_| w.rng.randf_sym(0.0, 0.01));
                let life = w.ticks(0x14);
                if let Some(t) = part62(w, add(at, j), life, 0) {
                    orb_rec(w, t, |r| {
                        rec::set_u32(r, 4, ORB_RGBA);
                        rec::set_ff(r, 0xc, TRAIL_SIZE);
                    });
                }
            }
        } else {
            if p::i16(&w.m(id).pvars, nt::HEALED) == 0 {
                let max = max_health(w);
                let hp = w.hero_fields().health;
                if hp != 0 { w.hero_fields_mut().health = if hp < max { hp + 1 } else { max }; }
                if let Some(m) = w.svc.pickups.master.filter(|&m| w.m(m).pvars.len() >= nt::SIZE) {
                    let n = (p::i32(&w.m(m).pvars, nt::PENDING) - 1).max(0);
                    p::set_i32(&mut w.mm(m).pvars, nt::PENDING, n);
                }
                w.play_sound_as(1, 0, id, NANOTECH_CRATE);
                p::set_i16(&mut w.mm(id).pvars, nt::HEALED, 1);
            }
            kill_orb(w, o);
            set_ptr(&mut w.mm(id).pvars, nt::ORBS + 4 * k, None);
        }
    }
}

/// `FUN_00300900`: the healing ring, 4 pairs of type-59 sparkles at Ratchet (behind him from the camera, stacked
/// 0.1 apart), each while its `ticks(90)` timer runs; true while any timer is above `ticks(48)`.
fn heal_ring(w: &mut World, id: MobyId) -> bool {
    let pos = pos3(w, id);
    let away = set_len(sub(f3(w.camera), pos), -0.3);
    let step = set_len(away, 0.1);
    let mut at = add(away, pos);
    let z0 = at[2];
    let mut young = false;
    let hero = f3(w.hero.pos);
    for k in 0..4 {
        let mut t = p::i16(&w.m(id).pvars, nt::RING_T + 2 * k);
        let res = sv::fast_dec_timer_s16(&mut t);
        p::set_i16(&mut w.mm(id).pvars, nt::RING_T + 2 * k, t);
        if res == 0 {
            if w.ticks(0x30) < t as i32 { young = true; }
            let pv_ = &mut w.mm(id).pvars;
            let mut hue = p::ff(pv_, nt::HUE + 4 * k) + RING_HUE[k];
            if 255.0 <= hue { hue -= 255.0; } else if hue <= 0.0 { hue += 255.0; }
            p::set_ff(pv_, nt::HUE + 4 * k, hue);
            let t255 = w.ticks(0xff) as f32;
            let f = (0.5 - (t255 - t as f32) / t255).abs();
            let c = crate::particles::tween_color(f.to_bits(), RING_A, RING_B);
            let life_f = 1.0 - t as f32 / w.ticks(90) as f32;
            let a = (((c >> 24) as f32) * (1.0 - life_f)) as i32 as u32;
            let q = [at[0], at[1], z0 + RING_Z[k], 0.0];
            if let Some(s) = w.particles.as_deref_mut() {
                s.hero = hero;
                let def62 = s.def_first(62);
                let rot = hue as i32 as u8;
                for (size, rgba) in [(RING_SIZE[k] * life_f, a << 24 | (c & 0xff_ffff)), (RING_SIZE[k] * life_f * 0.7, a << 24 | 0xff_ffff)] {
                    if let Some(r) = type59::spawn(s, size, q, rgba, rot, 53, false, 2, 1) { s.pool.recs[r][2] = def62; }
                    *w.svc.fx.part_spawns.entry(type59::TYPE).or_default() += 1;
                }
            }
        }
        at = add(at, step);
    }
    young
}

/// `FUN_00300808`: two type-60 glints at the cluster (on Ratchet) drifting `randf(1, 2)·dt` along a random
/// direction about the view line.
fn heal_glints(w: &mut World, id: MobyId) {
    let pos = pos3(w, id);
    let d = sub(f3(w.camera), pos);
    let s = w.rng.randf(DT, 2.0 * DT);
    let v = [0.0, 0.0, -s];
    let a = w.rng.rand_angle();
    let v = rotate(v, a, d);
    let q = [pos[0], pos[1], pos[2], 0.0];
    let vv = [v[0], v[1], v[2], 0.0];
    let r = w.rng.randi(0xff) as u8;
    w.part60(0.4, q, vv, 0x5080_4040, 0x3c, r, 1);
    let r = w.rng.randi(0xff) as u8;
    w.part60(0.2, q, vv, 0x407f_7f7f, 0x3c, r, 1);
}

/// Mode bits the pickups touch (kept for the doc links).
#[allow(dead_code)]
const HIDDEN: u16 = mode::HIDDEN;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_runs_from_b_to_c() {
        assert_eq!(cubic(3.0, 1.0, 5.0, 2.0, 0.0), 1.0);
        assert!((cubic(3.0, 1.0, 5.0, 2.0, 1.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn rotate_turns_right_handed() {
        let v = rotate([1.0, 0.0, 0.0], std::f32::consts::FRAC_PI_2, [0.0, 0.0, 1.0]);
        assert!((v[0]).abs() < 1e-6 && (v[1] - 1.0).abs() < 1e-6, "{v:?}");
    }

    #[test]
    fn item_tables_read_the_records() {
        let mut recs = vec![[0u8; 0x18]; 37];
        recs[10][0xc] = 3;
        recs[10][0xe] = 40;
        let t = ItemTables::new(&recs, [10, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!((t.max_ammo(10), t.pickup_amount(10), t.ammo_list()[0]), (40, 3, 10));
    }

    /// `0x2db850`: with the ammo full nothing happens (the pickup stays, no sound, no stat); below the max the ammo
    /// rises (clamped), the pickup flies (state 2) and plays sound 0 of class 213; a flying one is not taken again.
    #[test]
    fn collect_only_when_the_ammo_rises() {
        use crate::moby_runtime::{Moby, MobyTable};
        let mut recs = vec![[0u8; 0x18]; N];
        recs[10][0xe] = 40;
        let mut pv = vec![0u8; 0x30];
        p::set_i32(&mut pv, 0, 5);
        p::set_i32(&mut pv, 8, 10);
        let mut t = MobyTable::new(vec![Moby { o_class: 222, state: 1, pvars: pv, ..Moby::default() }], 4);
        let (mut hero, mut rng, classes, mut svc) = (crate::hero::Hero::new(), crate::rng::Rng::new(), crate::moby_update::ClassTable::default(), crate::moby_update::Services::new());
        hero.weapons.ammo[10] = 40;
        let full = ItemTables::new(&recs, [0xff; 12]).with_hero(&hero);
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 100);
        w.inventory = &full;
        assert!(!collect(&mut w, 0), "full: not taken");
        assert_eq!(w.m(0).state, 1, "it stays");
        assert!(w.svc.sounds.is_empty());
        assert_eq!(w.hero_fields().ammo_picked[10], 0);
        svc.hero_writes = None;
        hero.weapons.ammo[10] = 37;
        let low = ItemTables::new(&recs, [0xff; 12]).with_hero(&hero);
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 100);
        w.inventory = &low;
        assert!(collect(&mut w, 0), "taken");
        assert_eq!((w.m(0).state, w.hero_fields().ammo[10], w.hero_fields().ammo_picked[10]), (2, 40, 3), "clamped to 40, 3 picked");
        assert_eq!(w.svc.sounds.iter().map(|e| (e.sound_class, e.index, e.flags)).collect::<Vec<_>>(), [(PICKUP_SOUND_CLASS, 0, 0)]);
        assert!(!collect(&mut w, 0), "already flying");
    }
}
