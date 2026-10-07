//! The story services the level classes share (docs/plan/progression.md `## story`, level_scripting.md §2): the
//! saved-game stores the directors, mission NPCs and item givers make from the moby loop, written once here so every
//! class goes through one path into [`crate::game_state::GameState`] (save layout; the `saves` lane persists it).
//!
//! | game | here | effect |
//! |---|---|---|
//! | `0x13d388[i]` read / `= v` (chunk 5) | [`flag`] / [`set_flag`] | the global flags as one read / write view (G-SAV-010): the moby loop's mirror ([`crate::moby_update::interact::TalkGame::flags`]) and the write [`GameWrite::Flag`] applied after the tick |
//! | `0x13d408[k]` (chunk 8) | [`skill_point`] | a skill point earned |
//! | `if !0x13d408[k] { 0x13d408[k] = 1; PlayLevelSoundAtMoby(1, 0, 0); ShowBanner(0x53d6, −1) }` | [`award_skill_point`] | the one award sequence every site inlines (G-SAV-007): the byte, the jingle (level sound def 1, 2-D; level03's `allocate_voice_for_bank_entry(1, 0, 0)` and level04's `0x27eca0` are the same code), the banner "Skill Point" `ticks(180)` |
//! | `0x13d4c0[i] = v` / `0x13d4e8[i] = v` (chunks 10 / 11) | [`set_owned`] / [`set_acquired`] | a class's direct item store (not `GiveItem`: no banner, no ammo, no quick select) |
//! | `0x15eda0 = n` (chunk 19) | [`set_max_hp`] | max health (the nanotech upgrades: 5 premium, 8 ultra; G-SAV-005) |
//! | `0x15ed98 += n` | [`add_bolts`] | the bolt count (the counters the engine writes back into chunk 1) |
//! | `0x1415f8 = n` | [`set_health`] | Ratchet's health (the hero-block channel) |
//! | `0x13dd40[p]` | [`planet_unlocked`] | the planet bits (chunk 14) as the classes read them |
//! | `0x15ed84` | [`level`] | the current level |
//!
//! The help / move / gadget-help records ("stats", G-SAV-009) are written through [`crate::help::Help::records`]
//! (`units::hints::{bump, bump_move, touch}`) and go back to chunks 16 / 17 / 18 with `Help::sync_out`.

use crate::moby_runtime::MobyId;
use crate::moby_update::interact::{self, GameWrite};
use crate::moby_update::services::World;

/// `0x13d408`: the skill points (chunk 8, u8[32]; 30 used).
pub const SKILL_POINTS: u32 = 0x13_d408;
/// `ShowBanner(0x53d6, −1)`: "Skill Point" (level message 21462).
pub const SKILL_BANNER: i32 = 0x53d6;
/// `PlayLevelSoundAtMoby(1, 0, 0)`: the skill point jingle (level def 1).
pub const SKILL_SOUND: i32 = 1;
/// `0x13d388`: the global flags (chunk 5, u8[128]).
pub const FLAGS: u32 = 0x13_d388;
/// Global flag 4: the premium nanotech bought (max health 5).
pub const FLAG_PREMIUM_NANOTECH: usize = 4;
/// Global flag 5: the ultra nanotech bought (max health 8).
pub const FLAG_ULTRA_NANOTECH: usize = 5;

/// Index of the skill point byte at EE address `addr` (0x13d408 + k).
pub const fn skill_index(addr: u32) -> usize { (addr - SKILL_POINTS) as usize }

/// Index of the global flag at EE address `addr` (0x13d388 + i).
pub const fn flag_index(addr: u32) -> usize {
    // 0x13d388..0x13d408 (128 flags; the skill points follow). The owned / acquired items (0x13d4c0 / 0x13d4e8) are
    // not flags: `w.hero.owned` and [`acquired`].
    debug_assert!(addr >= FLAGS && addr < FLAGS + 0x80, "not a global flag address");
    (addr - FLAGS) as usize
}

/// Global flag `i` (0x13d388[i]) as this tick's class writes left it (0 past the table).
pub fn flag(w: &World, i: usize) -> u8 { w.svc.interact.game.flags.get(i).copied().unwrap_or(0) }

/// `0x13d388[i] = v` from a class update ([`interact::set_global_flag`]).
pub fn set_flag(w: &mut World, i: usize, v: u8) {
    let f = &mut w.svc.interact.game.flags;
    if f.len() < 128 { f.resize(128, 0); }
    interact::set_global_flag(w, i, v);
}

/// Skill point `k` earned (0x13d408[k] ≠ 0).
pub fn skill_point(w: &World, k: usize) -> bool { w.svc.interact.game.skill_points.get(k).is_some_and(|&b| b != 0) }

/// The skill point award every site inlines: when skill point `k` is not earned, it is set, the jingle plays
/// (`PlayLevelSoundAtMoby(1, 0, 0)`: 2-D at the listener) and the banner 0x53d6 shows for `ticks(180)`. Returns whether
/// it was awarded now.
pub fn award_skill_point(w: &mut World, k: usize) -> bool {
    if k >= 32 || skill_point(w, k) { return false; }
    set_skill_point(w, k);
    w.play_level_sound(SKILL_SOUND, 0, None);
    let t = w.ticks(180);
    crate::cinematic::show_banner(w, SKILL_BANNER, t);
    true
}

/// `0x13d408[k] = 1` alone (the mirror and the saved-game write).
pub fn set_skill_point(w: &mut World, k: usize) {
    let s = &mut w.svc.interact.game.skill_points;
    if s.len() < 32 { s.resize(32, 0); }
    s[k] = 1;
    w.svc.interact.writes.push(GameWrite::SkillPoint(k));
}

/// `0x13d4c0[item] = v`.
pub fn set_owned(w: &mut World, item: usize, v: u8) { w.svc.interact.writes.push(GameWrite::Owned(item, v)); }

/// `0x13d4e8[item]`: the item was ever acquired.
pub fn acquired(w: &World, item: usize) -> bool { w.svc.interact.game.acquired.get(item).is_some_and(|&b| b != 0) }

/// `0x13d4e8[item] = v` (and the talk conditions' mirror).
pub fn set_acquired(w: &mut World, item: usize, v: u8) {
    if let Some(b) = w.svc.interact.game.acquired.get_mut(item) { *b = v; }
    w.svc.interact.writes.push(GameWrite::Acquired(item, v));
}

/// `0x15eda0 = n` (the moby loop's mirror [`crate::moby_update::services::GameCounters::max_hp`] and chunk 19).
pub fn set_max_hp(w: &mut World, n: i32) {
    w.svc.counters.max_hp = n;
    w.svc.interact.writes.push(GameWrite::MaxHp(n));
}

/// `0x15ed98 += n` (the engine writes the counter back into chunk 1 after the tick; `0x13df38[level]` is the
/// pickups' and is not touched by a purchase).
pub fn add_bolts(w: &mut World, n: i32) { w.svc.counters.bolts += n; }

/// `0x1415f8 = n`: Ratchet's health (the hero-block channel, [`crate::moby_update::services::HeroFields::health`]).
pub fn set_health(w: &mut World, n: i32) { w.hero_fields_mut().health = n; }

/// Ratchet's feet (0x13f3d0) as a 4-vector (w 0).
pub fn hero4(w: &World) -> [f32; 4] { let h = w.hero_point(); [h[0], h[1], h[2], 0.0] }

/// `0x13dd40[p] ≠ 0`.
pub fn planet_unlocked(w: &World, p: usize) -> bool { w.svc.interact.game.planet_unlocked.get(p).is_some_and(|&b| b != 0) }

/// `0x15ed84`.
pub fn level(w: &World) -> u32 { w.svc.level }

/// `0x2a29a0(p)`: the trip to planet `p` (destination 0x15f5c0, pending 0x15f5d8; the main loop runs
/// `DoSpaceTransition`): the travel lane's one leave-the-level path ([`crate::cinematic::leave_level`]).
pub fn level_exit(w: &mut World, planet: i32) { crate::cinematic::leave_level(w, planet); }

/// Cuboid `c`'s centre and Euler (None for −1 / a missing shape).
pub fn cuboid(w: &World, c: i32) -> Option<([f32; 3], [f32; 3])> {
    if c < 0 { return None; }
    w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c).map(|s| (s.centre(), s.euler))
}

/// `PointInCuboid(0x13f3d0, c)`: Ratchet's feet in cuboid `c` (−1: never).
pub fn hero_in(w: &World, c: i32) -> bool { c >= 0 && w.in_cuboid(w.hero_point(), c) }

/// The moby index in a pvar, when it names a moby of the table (−1 / out of range: None).
pub fn link(w: &World, idx: i32) -> Option<MobyId> { usize::try_from(idx).ok().filter(|&m| m < w.table.mobys.len()) }

/// Moby `m` alive (not state 0xfe / 0xfd).
pub fn alive(w: &World, m: MobyId) -> bool { let s = w.m(m).state; s != 0xfe && s != 0xfd }

/// `mode |= 0x41` (hidden, no animation).
pub fn hide(w: &mut World, m: MobyId) {
    let mo = w.mm(m);
    mo.mode |= crate::moby_runtime::mode::HIDDEN | crate::moby_runtime::mode::NO_ANIM;
}

/// `mode &= ~0x41`.
pub fn show(w: &mut World, m: MobyId) {
    let mo = w.mm(m);
    mo.mode &= !(crate::moby_runtime::mode::HIDDEN | crate::moby_runtime::mode::NO_ANIM);
}

/// The mission byte `0x14c050[level·16 + m]` is 0xff (done); m = 0xff / out of range: false.
pub fn mission_done(w: &World, m: i32) -> bool { crate::moby_update::classes::units::hints::mission_done(w, m) }

/// The checkpoint record of cuboid `c` (`FUN_0029ac10(centre, Euler)` via `0x274af8`).
pub fn checkpoint_at(w: &mut World, c: i32) {
    if let Some((pos, rot)) = cuboid(w, c) {
        crate::moby_update::classes::checkpoint::record(w, crate::moby_update::classes::checkpoint::Record { pos, rot });
    }
}

/// `HeroTeleport(centre, Euler, state, reset_cam)` onto cuboid `c` (`0x20e370`-style helpers).
pub fn teleport_to(w: &mut World, c: i32, state: i32, reset_cam: bool) {
    if let Some((pos, rot)) = cuboid(w, c) { crate::cinematic::hero_teleport(w, pos, rot, state, reset_cam); }
}

/// The classes' inline "made permanent" stores on moby `m` (no death bits): `0x1baea4[spawn] = mission + 2`, and when
/// its mission is 0xff, or loaded (`0x15fc88[mission] ≠ −1`) and done in the live bytes, `0x1bbb04[spawn] = mission + 2`
/// (the same test as `SetDeathBits`).
pub fn kill_record(w: &mut World, m: MobyId) {
    let (sid, mission) = (w.m(m).spawn_id, w.m(m).mission);
    w.svc.save.killed.insert(sid, mission.wrapping_add(2));
    let lvl = w.svc.level;
    if mission == 0xff || (w.missions.mission_slot(mission) != 0xff && w.mission_done(lvl, mission) == 0xff) {
        w.svc.save.collected.insert(sid, mission.wrapping_add(2));
    }
}

/// `moby+0x38 = *(u64 *)(0x1413d0->+0x38)`: the light word and ambient copied from Ratchet's moby.
pub fn copy_hero_light(w: &mut World, id: MobyId) {
    let Some(h) = w.hero_moby.filter(|&h| h < w.table.mobys.len()) else { return };
    let (light, ambient) = (w.m(h).light, w.m(h).ambient);
    let m = w.mm(id);
    m.light = light;
    m.ambient = ambient;
}

/// The shadow slab `z ± 0.2` (`0x2499d8` and its level copies).
pub fn shadow_slab(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.shadow_hi = m.position[2] + 0.2;
    m.shadow_lo = m.position[2] - 0.2;
}

/// The pvar block of moby `id` grown to `n` bytes.
pub fn pvars(w: &mut World, id: MobyId, n: usize) { if w.m(id).pvars.len() < n { w.mm(id).pvars.resize(n, 0); } }

/// Moby `m`'s class when the pvar link `idx` names a moby of class `class` (the classes' `+0xa6 == class` guards).
pub fn link_of(w: &World, idx: i32, class: i16) -> Option<MobyId> { link(w, idx).filter(|&m| w.m(m).o_class == class) }

/// The classes' inline death-bit stores on moby `m` (no bolts): the persistent bit `0x14c190[level]` and this visit's
/// `0x1babd0` (level14's copy of 0x1ba950) for its spawn id.
pub fn death_bits(w: &mut World, m: MobyId) {
    let (lvl, sid) = (w.svc.level, w.m(m).spawn_id);
    w.svc.save.death.insert((lvl, sid));
    w.svc.save.death_level.insert(sid);
}

/// `0x254a30(group, update, drawn, collision)` (level09; level13 `0x2650a8`; −1: that part left alone): every live
/// member of the moby group (`0x1abb40[group]`): update off = mode | 2, drawn off = +0x31 = 0 and mode | 1, collision
/// = the class's blob or none.
pub fn group_set(w: &mut World, group: i32, update: i32, drawn: i32, coll: i32) {
    let Some(list) = usize::try_from(group).ok().and_then(|g| w.svc.groups.lists.get(g).cloned().flatten()) else { return };
    for k in list {
        let m = k as usize;
        if m >= w.table.mobys.len() || w.m(m).state >= 0x80 { continue; }
        set_flags(w, m, update, drawn, coll);
    }
}

/// The update / drawn / collision stores of [`group_set`] on one moby.
fn set_flags(w: &mut World, m: MobyId, update: i32, drawn: i32, coll: i32) {
    use crate::moby_runtime::mode;
    let has = crate::moby_update::classes::units::class_collision(w, w.m(m).o_class);
    let mo = w.mm(m);
    if update != -1 { if update == 0 { mo.mode |= mode::NO_UPDATE } else { mo.mode &= !mode::NO_UPDATE } }
    if drawn != -1 {
        if drawn == 0 {
            mo.visible = 0;
            mo.mode |= mode::HIDDEN;
        } else {
            mo.visible = 1;
            mo.mode &= !mode::HIDDEN;
        }
    }
    if coll != -1 { mo.has_collision = coll != 0 && has; }
}

/// Level13 `0x2651c8(classes, update, drawn, collision)`: [`group_set`]'s stores on every live moby (state < 0x80) of
/// the table whose class is in the list (a 0- or ≥ 0x800-terminated s16 list, at most 0xe0 entries; an empty list
/// touches nothing).
pub fn class_list_set(w: &mut World, classes: &[i16], update: i32, drawn: i32, coll: i32) {
    let list: Vec<i16> = classes.iter().copied().take_while(|&c| c != 0 && c < 0x800).take(0xe0).collect();
    if list.is_empty() { return; }
    for m in 0..w.table.mobys.len() {
        if w.m(m).state == crate::moby_runtime::state::END { break; }
        if w.m(m).state >= 0x80 || !list.contains(&w.m(m).o_class) { continue; }
        set_flags(w, m, update, drawn, coll);
    }
}

/// Level13 `0x265170(class, state)`: every live moby (state < 0x80) of the class gets `state`.
pub fn class_state_set(w: &mut World, class: i16, state: u8) {
    for m in 0..w.table.mobys.len() {
        if w.m(m).state == crate::moby_runtime::state::END { break; }
        if w.m(m).o_class == class && w.m(m).state < 0x80 { w.mm(m).state = state; }
    }
}
