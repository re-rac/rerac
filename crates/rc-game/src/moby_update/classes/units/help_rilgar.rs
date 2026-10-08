//! The Rilgar help director, class 1347 (census U204; level05 `0x31bf40`, one instance). Read from the level05
//! decomp. `H[n]` / `M[n]` / `G[n]`: help, move and gadget-help records.
//!
//! **Pvars** (s32): +0x00..+0x0c Ratchet's position, +0x10..+0x1c his last grounded position (both stored every
//! tick); +0x20 / +0x58 missions; cuboids +0x24, +0x2c, +0x34, +0x38, +0x40, +0x48, +0x50; +0x54 a pad moby (−1
//! once handled); +0x60 the dive counter; +0x68 the swim tick counter.
//!
//! | address | what | port |
//! |---|---|---|
//! | entry | update distance 0xff | [`update`] |
//! | 0x31bf60 | in cuboid +0x50 swimming (group 0x12): +0x68 += 1; past `ScaleTicks(360)` with `H[0x44]` lacking this level's bit → `(1008, 0x44)`; otherwise group 0x11, or global flag 0x0d (0x13d395) set → `H[0x44]` count := 0xffff | [`update`] |
//! | 0x31c038 | in cuboid +0x40, an hour of play since `M[9]`'s time, `H[0x48]` without this level's bit → `(2013, 0x48)` | [`update`] |
//! | 0x31c0d0 | tick 0x15f5cc < `ScaleTicks(60)`: `M[12]` used, `M[11]` not, `H[0x51]` never shown → `(20007, 0x51)`; `M[29]` used → `(20008, 0x72)` when `H[0x72]` was never shown, and `M[29]` count := 0 | [`update`] |
//! | 0x31c158 | the Hydrodisplacer (item 22) not owned: in cuboid +0x24 off any moby (0x13f64c = 0) → the reminder on `H[0x23]` → `(5002, 0x23)`; owned: the same place → the arm / reminder on `H[0x24]` → `(5003, 0x24)`, and group 0x13 → `H[0x24]` count := 0xffff | [`update`] |
//! | 0x31c3c0 | `H[0x26]` < 2: mission +0x20 done → `H[0x25]` bumped; else its deaths > 2 in cuboid +0x2c with `G[16]`, `G[15]`, `G[20]`, `G[10]`, `G[17]` all older than `ScaleTicks(18000)` and `H[0x26]` never shown → `(5005, 0x26)` | [`update`] |
//! | 0x31c5d8 | global flag 0 (0x13d388) set, `H[0x22]` 0, group 0x16 → `(5001, 0x22)` | [`update`] |
//! | 0x31c630 | `H[0x5e]` 0: state 10 in cuboid +0x34 / +0x38 → `H[0x5e]` bumped; swimming with the last grounded point in those cuboids → that point zeroed, +0x60 += 1; not swimming, +0x60 > 1 and Ratchet in them → `(5006, 0x5e)` | [`update`] |
//! | 0x31c718 | `M[18]` and `H[0x4a]` 0, the Blaster (item 15) owned, in cuboid +0x48 with its ammo 0x13d464 ≥ 20 → `(20000, 0x4a)` | [`update`] |
//! | 0x31c7e4 | the pad +0x54 (≠ −1), missions +0x58 and +0x20 done, and the first 5 ticks or Ratchet on a class 998 / 830 moby → the pad's activation `FUN_0030c928` (its state 2, command byte, glow 0x80208020, sound 0, the level's per-instance map / save bytes 0x1baea4 / 0x1bbb04, a spline point of 0x1b0930 cleared); +0x54 := −1 | +0x54 := −1, the activation [`crate::moby_update::classes::floor_switch::press`]: [`update`] |
//! | exit | +0x00..+0x0c := Ratchet's position; airborne ticks 0x13f65e = 0 → +0x10..+0x1c := it too | [`update`] |
//! | no particle, flag write, other moby written (besides the pad's activation) | | n/a |

use super::hints::{arm_or_remind, bump, flag, in_cuboid, mission_done, older_than, owned, pvi, remind, request, set_pvi, ticks};
use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

pub const UPDATE_FN: u32 = 0x31_bf40;
pub const REFERENCE_LEVEL: u32 = 5;
pub const CLASSES: [i16; 1] = [1347];

fn h(w: &World, r: usize) -> u16 { w.svc.help.records.help[r].count }
fn m(w: &World, r: usize) -> u16 { w.svc.help.records.moves[r].count }
fn level_bit(w: &World) -> u32 { 1u32 << (w.svc.help.level as u32 & 31) }
fn never_shown(w: &World, r: usize) -> bool { (w.svc.help.records.help[r].mask as i32) >= 0 }

/// The point stored at pvar `off` (x, y, z).
fn point(w: &World, id: MobyId, off: usize) -> [f32; 3] { std::array::from_fn(|k| f32::from_bits(pvi(w, id, off + 4 * k) as u32)) }

/// Level05 0x31bf40 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    w.mm(id).update_dist = 0xff;
    let group = w.hero.group;
    if in_cuboid(w, id, 0x50) {
        let mut retire = true;
        if group == 0x12 {
            let n = pvi(w, id, 0x68);
            set_pvi(w, id, 0x68, n + 1);
            if ticks(0x168) < n {
                retire = false;
                if w.svc.help.records.help[0x44].mask & level_bit(w) == 0 { request(w, 0x3f0, 0x44); }
            }
        }
        if retire && (group == 0x11 || flag(w, 0xd)) { w.svc.help.records.help[0x44].count = 0xffff; }
    }
    if in_cuboid(w, id, 0x40) {
        let hour = (ticks(0xe10) as f32 * 60.0) as i32;
        let t = ticks(w.svc.help.play_time) - 600 * w.svc.help.records.moves[9].time as i32;
        if hour < t && w.svc.help.records.help[0x48].mask & level_bit(w) == 0 { request(w, 0x7dd, 0x48); }
    }
    let early = (w.counter as i32) < ticks(0x3c);
    if early && m(w, 12) != 0 && m(w, 11) == 0 && never_shown(w, 0x51) { request(w, 0x4e27, 0x51); }
    if early && m(w, 29) != 0 {
        if never_shown(w, 0x72) { request(w, 0x4e28, 0x72); }
        w.svc.help.records.moves[29].count = 0;
    }
    let dock = in_cuboid(w, id, 0x24) && w.hero.ground_moby.is_none();
    if !owned(w, 22) {
        if dock { remind(w, 0x23, 0x138a, 0x23); }
    } else {
        if dock { arm_or_remind(w, 0x24, 0x138b, 0x24); }
        if group == 0x13 { w.svc.help.records.help[0x24].count = 0xffff; }
    }
    if h(w, 0x26) < 2 {
        let mission = pvi(w, id, 0x20);
        if mission_done(w, mission) {
            bump(w, 0x25);
        } else if u8::try_from(mission).map_or(0, |mi| w.missions.ammo_crate_gate(mi)) > 2 && in_cuboid(w, id, 0x2c) {
            let g = w.svc.help.records.gadget;
            if [16, 15, 20, 10, 17].iter().all(|&i| older_than(w, g[i].time, 18000)) && never_shown(w, 0x26) { request(w, 0x138d, 0x26); }
        }
    }
    if flag(w, 0) && h(w, 0x22) == 0 && group == 0x16 { request(w, 0x1389, 0x22); }
    if h(w, 0x5e) == 0 {
        let in_pool = |w: &World| in_cuboid(w, id, 0x34) || in_cuboid(w, id, 0x38);
        if w.hero.state == 10 && in_pool(w) { bump(w, 0x5e); }
        if group == 0x12 {
            let p = point(w, id, 0x10);
            if w.in_cuboid(p, pvi(w, id, 0x34)) || w.in_cuboid(p, pvi(w, id, 0x38)) {
                for k in 0..4 { set_pvi(w, id, 0x10 + 4 * k, 0); }
                set_pvi(w, id, 0x60, pvi(w, id, 0x60) + 1);
            }
        } else if pvi(w, id, 0x60) > 1 && in_pool(w) {
            request(w, 0x138e, 0x5e);
        }
    }
    let ammo = crate::moby_update::classes::pickup::item_ammo(w, 15);
    if m(w, 18) == 0 && h(w, 0x4a) == 0 && owned(w, 15) && in_cuboid(w, id, 0x48) && ammo >= 0x14 { request(w, 20000, 0x4a); }
    let pad = pvi(w, id, 0x54);
    let on_pad = w.hero.ground_moby.and_then(|g| w.table.mobys.get(g)).is_some_and(|g| g.o_class == 0x3e6 || g.o_class == 0x33e);
    if pad != -1 && mission_done(w, pvi(w, id, 0x58)) && mission_done(w, pvi(w, id, 0x20)) && ((w.counter as i32) < 5 || on_pad) {
        if let Some(m) = crate::moby_update::story::link(w, pad) { crate::moby_update::classes::floor_switch::press(w, m); }
        set_pvi(w, id, 0x54, -1);
    }
    let p = w.hero.pos.map(|x| x.0 as i32);
    for (k, v) in p.iter().enumerate() { set_pvi(w, id, 4 * k, *v); }
    if w.hero.air_ticks == 0 {
        for (k, v) in p.iter().enumerate() { set_pvi(w, id, 0x10 + 4 * k, *v); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};

    /// The first ticks with both missions done press the pad (`0x30c928`: its kill / never-again bytes, state 2) and
    /// forget it.
    #[test]
    fn early_ticks_press_the_pad() {
        let mut pv = vec![0u8; 0x70];
        for off in [0x24, 0x2c, 0x34, 0x38, 0x40, 0x48, 0x50] { pv[off..off + 4].copy_from_slice(&(-1i32).to_le_bytes()); }
        pv[0x54..0x58].copy_from_slice(&1i32.to_le_bytes());
        let director = Moby { o_class: 1347, pvars: pv, ..Moby::default() };
        let pad = Moby { o_class: 830, state: 1, mission: 0xff, spawn_id: 12, pvars: vec![0xff; 16], ..Moby::default() };
        let mut t = MobyTable::new(vec![director, pad], 4);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        update(&mut w, 0);
        assert_eq!((w.m(1).state, pvi(&w, 0, 0x54)), (2, -1));
        assert_eq!((w.svc.save.killed.get(&12), w.svc.save.collected.get(&12)), (Some(&1), Some(&1)));
    }
}
