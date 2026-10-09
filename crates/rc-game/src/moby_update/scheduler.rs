//! The moby update scheduler (level01 addresses): the class dispatch of the level's update table, the
//! load pass `FUN_002792d0`, the active-list builder `fun_0020d868` (0x265548) and the per-frame loop
//! `FUN_002793d8` (called by `UpdateMobysPartB` 0x279470 outside the profiler mode), each moby running
//! `MobyAnimAdvance` 0x265260 (unless mode & 0x40) → its update → the matrix rebuild `fun_0020def8` 0x265bd8
//! (unless mode & 4). Spec: `docs/plan/moby_update_catalogue.md` §1 and "In the port: the scheduler".
//!
//! **Active rule** (`fun_0020d868`, read from the disassembly). Walk the moby array from index 0 until the
//! first state byte 0xff; skip state ≥ 0x80 (deleted) and mode & 2. A moby is active when
//! `+0x31 != 0` (drawn last frame), or update distance `+0x30 == 0xff`, or
//! `((d·d − dx²) − dy²) − dz²` has its sign bit clear, with `d = (f32)+0x30` and `dx.. = pos − camera`
//! (0x167240) on VU0 (`vmula.x`, three `vmsuba`/`vmsub` with 1.0). An active moby with group `+0x21 ≥ 0` is
//! **not** added itself: it flags its group, and after the walk every member of every flagged group is
//! added (groups 0..0x70 in order, members in list order, only `state < 0x80` checked, not mode 2 or the
//! distance). Additions go to per-class-slot lists (224 slots, `0x70000000 + slot·8` = head/tail), which are
//! then chained in slot order: **run order = class slot, then direct actives in index order, then
//! group-added members in group order**. Every added moby with mode & 0x1000 goes to the target list
//! 0x1abe80 (NULL-terminated), in addition order.
//!
//! **Loop.** The list is built before any update runs: a moby created during the loop waits a tick, a moby
//! deleted during the loop (state ≥ 0x80) is skipped when reached. The update function is only called when
//! the moby has one (`+0x74 != 0`); group-added mobys without one still get the advance and the matrix.
//!
//! **Load pass** (`FUN_002792d0`, once from `LoadLevelCoreData`): every moby up to the first 0xff with
//! state < 0x80 and no mode 2, in **array order** (no distance gate, no class order), target list cleared.
//!
//! **Dispatch.** The game calls `moby+0x74`, the level table's function for the class. The port keeps the
//! address in [`Moby::update_fn`](crate::moby_runtime::Moby::update_fn) and maps it to a Rust port with
//! [`ported`]; [`port_update_fn`] gives the address to put in `ClassInfo::update_fn` for a class. A class
//! without a port gets `None` there, so `InitMobyInstance` sets mode 2 exactly as for a class with no table
//! entry (such mobys are still advanced/rebuilt when their sequence 0 has ≥ 2 frames, which clears mode 2,
//! or when a group adds them — as in the game).

use crate::hero::physics::V4;
use crate::moby_runtime::{mode, state, Moby, MobyId, MobyTable};
use crate::moby_update::classes::{self, ClassUpdate};
use crate::moby_update::services::{pv, World};
#[allow(unused_imports)]
use crate::moby_update::services::ClassData;
use crate::ps2v::Pf;
use rc_formats::moby_anim::{self, MobyAnimClass};

/// Class-slot lists of the builder (`0x70000000..0x70000700`, 8 bytes each).
pub const CLASS_SLOTS: usize = 0xe0;
/// Group flags of the builder (`0x70000c80..0x70000cf0`).
pub const GROUPS: usize = 0x70;

/// The level-table address the port uses for `o_class` (`Some` only for classes with a Rust port).
pub fn port_update_fn(o_class: i16) -> Option<u32> { classes::for_class(o_class).map(ClassUpdate::key) }

/// [`port_update_fn`], then the external ports.
pub fn update_fn_for(o_class: i16, external: Option<&dyn crate::moby_update::services::ExternalUpdates>) -> Option<u32> {
    port_update_fn(o_class).or_else(|| external.and_then(|e| e.update_fn(o_class)))
}

/// The Rust port behind an update-slot key ([`ClassUpdate::key`]).
pub fn ported(addr: u32) -> Option<ClassUpdate> { ClassUpdate::from_address(addr) }

/// Moby groups `0x1abcc0[g]` (gameplay header +0x48): member lists of runtime moby indices, the last one
/// flagged with bit 15 in the game; here plain lists.
#[derive(Clone, Debug, Default)]
pub struct Groups {
    pub lists: Vec<Option<Vec<u16>>>,
}

impl Groups {
    /// The loader's copy (`InitLevelRenderGlobals` 0x255958): section `s32 count, s32 data_size, pad[2],
    /// s32 offset[count]`, then the data; each list is u16 instance indices ending at bit 15, mapped through
    /// the instance → moby table (`instance_to_moby`; −1 drops an entry), and a list left empty is absent.
    pub fn parse(gameplay: &[u8], instance_to_moby: &dyn Fn(usize) -> Option<usize>) -> Groups {
        let rd = |o: usize| -> Option<i32> { Some(i32::from_le_bytes(gameplay.get(o..o + 4)?.try_into().ok()?)) };
        let Some(base) = rd(0x48).filter(|&b| b > 0).map(|b| b as usize) else { return Groups::default() };
        let (Some(count), Some(_size)) = (rd(base), rd(base + 4)) else { return Groups::default() };
        let offs = base + 0x10;
        let data = offs + 4 * count.max(0) as usize;
        let mut lists = Vec::new();
        for g in 0..count.max(0) as usize {
            let Some(o) = rd(offs + 4 * g) else { break };
            if o < 0 { lists.push(None); continue; }
            let mut p = data + o as usize;
            let mut out = Vec::new();
            while let Some(b) = gameplay.get(p..p + 2) {
                let e = u16::from_le_bytes([b[0], b[1]]);
                if let Some(m) = instance_to_moby((e & 0x7fff) as usize) { out.push(m as u16); }
                if e & 0x8000 != 0 { break; }
                p += 2;
            }
            lists.push(if out.is_empty() { None } else { Some(out) });
        }
        Groups { lists }
    }
}

// ---------------------------------------------------------------------------------------------------
// The moby groups: the engine's group services (G-CLS-023)
//
// A group is a list of the level's moby indices (gameplay +0x48 → `0x1abcc0[g]`, [`Groups`]); a member's +0x21 holds
// its group. Besides the activity rule of the run-list builder above (one active member runs the whole group), the
// engine has five functions a class calls on a group; the level overlays link them only where a class uses them, so
// this table is the set every level has (the callers per level: `docs/plan/gaps.md` G-CLS-023). Any class that issues
// a group command calls these: there is no other group state.
//
// | address (level01) | game | what | here |
// |---|---|---|---|
// | 0x26e008 | `MobyGroupCount(g, skip)` | members alive (state < 0x80) and not in state `skip` (−1: any state) | [`group_count`] |
// | 0x26e090 | `0x26e090(g, c)` | every member's command byte +0xbc = c (dead ones too) | [`group_cmd`] |
// | 0x26e0e0 | `0x26e0e0(g, s)` | every live member's state = s (the "group command" of the teleporters, water managers, switches) | [`group_state`] |
// | 0x26e150 | `0x26e150(&out, g, a, b)` | the walk's first member passing the filter (a, b) | [`group_first`] |
// | 0x26e238 | `0x26e238(&out, cur, a, b)` | the next member after `cur` in `cur`'s group (+0x21) passing the filter | [`group_next`] |
// | level06 0x2f9948 | (Blarg's copy, 1051 / 1108) | park: every member state 0x12, collision off (+0x94 = 0), mode `&~0x1000 \| 1` | [`group_park`] |
// | level06 0x2f99b0 | (Blarg's copy, 1051 / 1108) | unpark: members in 0x12 → state 0, class collision, mode `&~1 \| 0x1000` | [`group_unpark`] |
//
// The walk keeps its cursor in globals (0x160114 the current moby, 0x16010c its list entry, 0x160110 its index): the
// port walks the list directly, which visits the same members in the same order. Its filter (a2, a3), read from the
// level02 copies 0x25b8b8 / 0x25b9a0 (the same code on every level): (0, 0) the live members (state < 0x80), (1, 0)
// every member, (1, 1) the dead ones only, (0, 1) none (the walk runs to the end and returns −1). A group index out of
// range (0x26e150: `g < 0` or `g > [0x15fff4]`; 0x26e238: `cur+0x21 > [0x15fff4]`) or without a list gives none.

/// The member filter of the group walk `0x26e150` / `0x26e238` (its arguments a2, a3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupWalk {
    /// (0, 0): state < 0x80.
    Alive,
    /// (1, 0): every member.
    Any,
    /// (1, 1): state ≥ 0x80.
    Dead,
    /// (0, 1): no member passes (the walk runs to the end).
    Nothing,
}

impl GroupWalk {
    /// The filter of the call `(a2, a3)`.
    pub fn of(a2: bool, a3: bool) -> GroupWalk {
        match (a2, a3) {
            (false, false) => GroupWalk::Alive,
            (true, false) => GroupWalk::Any,
            (true, true) => GroupWalk::Dead,
            (false, true) => GroupWalk::Nothing,
        }
    }

    fn takes(self, m: &Moby) -> bool {
        let dead = m.state >= 0x80;
        match self {
            GroupWalk::Alive => !dead,
            GroupWalk::Any => true,
            GroupWalk::Dead => dead,
            GroupWalk::Nothing => false,
        }
    }
}

/// The moby ids of group `g` (`0x1abcc0[g]`, list order; none for `g < 0`).
pub fn group_ids(w: &World, g: i8) -> Vec<MobyId> {
    if g < 0 { return Vec::new(); }
    w.svc.groups.lists.get(g as usize).and_then(|l| l.clone()).map(|l| l.into_iter().map(|m| m as MobyId).collect()).unwrap_or_default()
}

/// The list of group `g` as an `i32` index (the classes keep group indices in pvar words).
fn group_list(w: &World, g: i32) -> Vec<MobyId> { i8::try_from(g).map(|g| group_ids(w, g)).unwrap_or_default() }

/// `MobyGroupCount(g, skip_state)` 0x26e008: the members of group `g` that are alive (state < 0x80) and not in
/// `skip_state` (−1: count every live member). No list: 0.
pub fn group_count(w: &World, g: i32, skip_state: i32) -> i32 {
    group_list(w, g)
        .into_iter()
        .filter_map(|i| w.table.mobys.get(i))
        .filter(|m| m.state < 0x80 && (skip_state == -1 || m.state as i32 != skip_state))
        .count() as i32
}

/// `0x26e0e0(g, s)`: every live member's state byte = `s` (a dead member, state ≥ 0x80, is left).
pub fn group_state(w: &mut World, g: i8, s: u8) {
    for m in group_ids(w, g) {
        if let Some(mm) = w.table.mobys.get_mut(m) {
            if (mm.state as i8) >= 0 { mm.state = s; }
        }
    }
}

/// `0x26e090(g, c)`: every member's command byte +0xbc = `c`.
pub fn group_cmd(w: &mut World, g: i8, c: u8) {
    for m in group_ids(w, g) {
        if let Some(mm) = w.table.mobys.get_mut(m) { mm.cmd = c; }
    }
}

/// `0x26e150(&out, g, a2, a3)`: the first member of group `g` (list order) that passes `f` (module table).
pub fn group_first(w: &World, g: i32, f: GroupWalk) -> Option<MobyId> {
    group_list(w, g).into_iter().find(|&i| w.table.mobys.get(i).is_some_and(|m| f.takes(m)))
}

/// `0x26e238(&out, cur, a2, a3)`: the next member after `cur` in `cur`'s group (its +0x21, read unsigned) that passes
/// `f`; none when `cur` is not in its group's list or is its last member.
pub fn group_next(w: &World, cur: MobyId, f: GroupWalk) -> Option<MobyId> {
    let g = w.table.mobys.get(cur)?.group as u8;
    let list = w.svc.groups.lists.get(g as usize)?.as_ref()?;
    let at = list.iter().position(|&e| (e & 0x7fff) as usize == cur)?;
    list[at + 1..].iter().map(|&e| (e & 0x7fff) as usize).find(|&i| w.table.mobys.get(i).is_some_and(|m| f.takes(m)))
}

/// The whole walk `0x26e150` then `0x26e238` until −1: the members of `g` passing `f`, in list order.
pub fn group_walk(w: &World, g: i32, f: GroupWalk) -> Vec<MobyId> {
    group_list(w, g).into_iter().filter(|&i| w.table.mobys.get(i).is_some_and(|m| f.takes(m))).collect()
}

/// Level06 `0x2f9948(g)` (Blarg's group park, 1051 / 1108): every member (no state test) state 0x12, `+0x94 = 0`
/// (collision off), mode `& ~0x1000 | 1` (hidden, not targetable).
pub fn group_park(w: &mut World, g: i32) {
    for i in group_list(w, g) {
        if let Some(m) = w.table.mobys.get_mut(i) {
            m.state = 0x12;
            m.has_collision = false;
            m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN;
        }
    }
}

/// Level06 `0x2f99b0(g)` (Blarg's group unpark): the members in state 0x12 → state 0, `+0x94` = the class's collision
/// (class +0x10), mode `& ~1 | 0x1000`.
pub fn group_unpark(w: &mut World, g: i32) {
    for i in group_list(w, g) {
        let Some(oc) = w.table.mobys.get(i).filter(|m| m.state == 0x12).map(|m| m.o_class) else { continue };
        let coll = w.classes.info(oc).is_some_and(|c| c.has_collision);
        let m = &mut w.table.mobys[i];
        m.state = 0;
        m.has_collision = coll;
        m.mode = (m.mode & !mode::HIDDEN) | mode::TARGETABLE;
    }
}

/// Scheduler state kept between ticks (the groups live in [`Services::groups`](crate::moby_update::Services)).
#[derive(Clone, Debug, Default)]
pub struct Scheduler {
    /// `0x1abe80`: the targetable (mode 0x1000) mobys of the last list, in addition order.
    pub targets: Vec<MobyId>,
    /// The run list of the last tick (for stats and tests).
    pub last_list: Vec<MobyId>,
}

/// `fun_0020d868` 0x265548: `(run list, target list)` for the camera at `cam` (0x167240).
pub fn build_active_list(table: &MobyTable, cam: V4, groups: &Groups) -> (Vec<MobyId>, Vec<MobyId>) {
    let mut slots: Vec<Vec<MobyId>> = vec![Vec::new(); CLASS_SLOTS];
    let mut group_flag = [false; GROUPS];
    let mut targets = Vec::new();
    let add = |slots: &mut Vec<Vec<MobyId>>, targets: &mut Vec<MobyId>, id: MobyId, m: &Moby| {
        if m.mode & mode::TARGETABLE != 0 { targets.push(id); }
        slots[m.class_slot as usize % CLASS_SLOTS].push(id);
    };
    for (id, m) in table.mobys.iter().enumerate() {
        if m.state == state::END { break; }
        if m.state >= 0x80 || m.mode & mode::NO_UPDATE != 0 { continue; }
        if !is_active(m, cam) { continue; }
        if m.group >= 0 {
            group_flag[m.group as usize % GROUPS] = true;
            continue;
        }
        add(&mut slots, &mut targets, id, m);
    }
    for (g, &on) in group_flag.iter().enumerate() {
        if !on { continue; }
        let Some(Some(list)) = groups.lists.get(g) else { continue };
        for &e in list {
            let id = (e & 0x7fff) as usize;
            let Some(m) = table.mobys.get(id) else { continue };
            if m.state >= 0x80 { continue; }
            add(&mut slots, &mut targets, id, m);
        }
    }
    (slots.into_iter().flatten().collect(), targets)
}

/// The per-moby activity test of `fun_0020d868` (without the group step).
pub fn is_active(m: &Moby, cam: V4) -> bool {
    if m.visible != 0 || m.update_dist == 0xff { return true; }
    let p = pv(m.position);
    let d = [p[0] - cam[0], p[1] - cam[1], p[2] - cam[2]];
    let sq = d.map(|x| x * x);
    let r = Pf::from_i32(m.update_dist as i32); // vitof0.x
    let acc = r * r; // vmula.x ACC, vf3, vf3
    let acc = acc - Pf::ONE * sq[0]; // vmsubax.x ACC, vf4, vf2
    let acc = acc - Pf::ONE * sq[1]; // vmsubay.x
    let v = acc - Pf::ONE * sq[2]; // vmsubz.x vf2, vf4, vf2
    !v.sign() // qmfc2, dsll32, bltz
}

/// `fun_0020def8` 0x265bd8: rows from the Euler angles (unless mode 0x100, which keeps the stored rows),
/// row 1 negated with mode 0x8000 (also on kept rows, as in the code), the sequence bounding sphere
/// (`+0xf0` cache of the key-A sequence's sphere when A = B, else `((B·t) + A) − A·t`) scaled, rotated and
/// placed at `pos · 1024` into +0x00, and the +0xa8 change counter + 1. The moby-grid re-insert (0x265900,
/// for mobys with collision) is `collision_query::MobyGrid::register`, which the callers with the grid run
/// right after ([`run_moby`], `World::build_matrix`; the loader's through `MobyGrid::build`). A snapshot key
/// A (seq 0xff) uses sequence B's sphere (the snapshot sphere table 0x197180 is not ported).
pub fn rebuild_matrix(m: &mut Moby, class: Option<&MobyAnimClass>) {
    if m.state >= 0x80 { return; }
    let mut rows: [V4; 3] = if m.mode & mode::KEEP_ROWS != 0 {
        [pv(m.rows[0]), pv(m.rows[1]), pv(m.rows[2])]
    } else {
        rc_formats::moby_light::rotation_rows([m.rotation[0], m.rotation[1], m.rotation[2]]).map(|r| r.map(Pf))
    };
    let sphere_of = |seq: u8| -> Option<V4> {
        let q = class?.sequence(seq)?;
        Some(q.header.sphere.map(|x| Pf(x.to_bits())))
    };
    let (a, b) = (m.anim.seq_a, m.anim.seq_b);
    let sphere: Option<V4> = if a == b {
        if a != m.b71 {
            m.b71 = a;
            if let Some(s) = sphere_of(a) { m.rows[3] = s.map(|x| f32::from_bits(x.0)); }
        }
        Some(pv(m.rows[3]))
    } else {
        let sb = sphere_of(b);
        let sa = if a == moby_anim::SNAPSHOT_SEQ { sb } else { sphere_of(a) };
        match (sa, sb) {
            (Some(sa), Some(sb)) => {
                let t = Pf(m.anim.t.to_bits());
                Some(std::array::from_fn(|k| ((sb[k] * t) + sa[k] * Pf::ONE) - sa[k] * t))
            }
            _ => None,
        }
    };
    if m.mode & mode::MIRROR != 0 {
        for x in &mut rows[1][..3] { *x = Pf::ZERO - *x; }
    }
    for (k, r) in rows.iter().enumerate() { m.rows[k] = r.map(|x| f32::from_bits(x.0)); }
    if let Some(s) = sphere {
        let sc = Pf(m.scale.to_bits());
        let s5 = s.map(|x| x * sc);
        let p = pv(m.position);
        let k1024 = Pf::b(0x4480_0000);
        let p7 = [p[0] * k1024, p[1] * k1024, p[2] * k1024];
        let c: [Pf; 3] = std::array::from_fn(|k| ((rows[0][k] * s5[0] + rows[1][k] * s5[1]) + rows[2][k] * s5[2]) + p7[k] * Pf::ONE);
        m.bsphere = [c[0], c[1], c[2], s5[3]].map(|x| f32::from_bits(x.0));
    }
    let lo = (m.uid_hi as u16).wrapping_add(1);
    m.uid_hi = (m.uid_hi & 0xffff_0000) | lo as u32;
}

/// One moby's step of the loop: advance (unless mode 0x40), update (if it has a function), matrix (unless
/// mode 4). Skipped when the state byte is ≥ 0x80 (deleted earlier in the loop).
pub fn run_moby(w: &mut World, id: MobyId) {
    if w.m(id).state >= 0x80 { return; }
    let o_class = w.m(id).o_class;
    // MobyAnimAdvance with its sound triggers and loop sound (crate::moby_update::anim_sound).
    if w.m(id).mode & mode::NO_ANIM == 0 { crate::moby_update::anim_sound::advance(w, id); }
    if let Some(addr) = w.m(id).update_fn {
        if let Some(u) = ported(addr) {
            classes::dispatch(u, w, id);
        } else if let Some(ext) = w.external.as_deref_mut() {
            ext.update(addr, id, w.table, w.rng, w.camera, w.counter, w.particles.as_deref_mut());
        }
    }
    if w.m(id).mode & mode::KEEP_MATRIX == 0 {
        let class = w.classes.anim(o_class);
        rebuild_matrix(&mut w.table.mobys[id], class);
        // MobyBuildMatrix's tail: the moby-grid re-insert (UpdateMobyGrids 0x265900).
        std::sync::Arc::make_mut(&mut w.svc.grid).register(&mut w.table.mobys[id]);
    }
}

impl Scheduler {
    pub fn new() -> Scheduler { Scheduler::default() }

    /// `FUN_002792d0`: the load pass. Returns the number of mobys run.
    pub fn load_pass(&mut self, w: &mut World) -> usize {
        self.targets.clear();
        // `0x15ffbc` as the loader leaves it (the count of 0x263300 over the fresh dynamic slots).
        w.table.free_slot_pass(w.counter);
        let mut list = Vec::new();
        for (id, m) in w.table.mobys.iter().enumerate() {
            if m.state == state::END { break; }
            if m.state & 0x80 == 0 && m.mode & mode::NO_UPDATE == 0 { list.push(id); }
        }
        if w.svc.snapshots.len() < w.table.mobys.len() { w.svc.snapshots.resize(w.table.mobys.len(), None); }
        for &id in &list { run_moby(w, id); }
        // The points the moby-riding particles (types 26 / 55) follow in the UpdateParts that comes next.
        w.refresh_particle_anchors();
        let n = list.len();
        self.last_list = list;
        n
    }

    /// `FUN_002793d8`: one frame of moby updates (build the list with the camera of `w`, run it). Returns the
    /// number of mobys in the list. The free-slot pass 0x263300 (`MobyTable::free_slot_pass`) is the tick's
    /// first call and belongs to the caller (`rc_game::tick::Game::tick`).
    pub fn tick(&mut self, w: &mut World) -> usize {
        // The race cameras' stores of the last camera update (its `rand` draw before the frame's draw callbacks'):
        // classes::units::oltanis_rail_bot.
        classes::units::oltanis_rail_bot::camera_stores(w);
        // The last frame's draw callbacks (their state and rand parts), before any update: classes::draw_callbacks.
        classes::draw_callbacks::run_frame(w);
        w.svc.draw_callbacks.tick = w.counter + 1;
        // The level's scene hook (mode 2 only): classes::cutscene_fx::level_hook.
        classes::cutscene_fx::level_hook(w);
        // The hits the last UpdateParts gave (type 58's fire: particles::type58).
        part_hits(w);
        // The Hoverboard hero code's stores of the last hero update (classes::units::hoverboard).
        classes::units::hoverboard::apply(w);
        let (list, targets) = build_active_list(w.table, w.camera, &w.svc.groups);
        w.svc.targets.clone_from(&targets);
        self.targets = targets;
        if w.svc.snapshots.len() < w.table.mobys.len() { w.svc.snapshots.resize(w.table.mobys.len(), None); }
        for &id in &list { run_moby(w, id); }
        // The points the moby-riding particles (types 26 / 55) follow in the UpdateParts that comes next.
        w.refresh_particle_anchors();
        // `HudBoltAlertShow` 0x227d90 (the hero's frame, after the moby pass): classes::buried_bolts.
        classes::buried_bolts::alert_frame(w);
        let n = list.len();
        self.last_list = list;
        n
    }
}

// ---------------------------------------------------------------------------------------------------
// Level load: the loader's instance loop, for building the table

/// `InitMobyInstance`'s view of a class blob (header fields by offset) with its slot and update address.
/// (`has_collision` = class +0x10 ≠ 0: `moby+0x94`, the grid registration and the collision kernels' test.)
pub fn class_info(class: &rc_formats::moby::MobyClass, slot: u8, update_fn: Option<u32>) -> crate::moby_runtime::ClassInfo {
    let h = &class.header;
    crate::moby_runtime::ClassInfo {
        slot,
        no_header: false,
        update_fn,
        b06: h.metal_count,
        b0c: h.sequence_count,
        b0e: h.lod_trans,
        b0f: h.shadow,
        has_collision: h.collision != 0,
        scale: h.scale,
        glow: (h.glow_rgba != 0).then_some(h.glow_rgba as u32),
        mode_bits: h.mode_bits as u16,
        ty: h.ty,
        seq0: None,
        has_sounds: h.sound_defs != 0,
    }
}

/// The loader's instance loop over every record as if every spawn test passed (runtime index = instance
/// index): for tools and tests that want the whole instance list. The level loader is [`load_level_mobys`].
pub fn load_static_mobys(
    instances: &[rc_formats::gameplay::MobyInstance],
    classes: &mut crate::moby_update::ClassTable,
    pvars: &[Option<Vec<u8>>],
) -> Vec<Moby> {
    let all = rc_formats::moby_spawn::SpawnTest { spawn: true, b1: 0, b4: 0, b6: 0 };
    let tests: Vec<_> = instances.iter().map(|i| rc_formats::moby_spawn::SpawnTest {
        b1: if i.spawn_flags == 0 { 0xfe } else { 0xff },
        b4: i.unknown_10 as i16,
        b6: i.unknown_10 as i16,
        ..all
    }).collect();
    load_level_mobys(instances, classes, pvars, &tests).mobys
}

/// The static mobys the loader created, and its instance ↔ moby maps.
#[derive(Clone, Debug, Default)]
pub struct LevelStatics {
    /// The created mobys in runtime order (moby index = position here).
    pub mobys: Vec<Moby>,
    /// `0x1acc00`: gameplay instance → runtime moby index (None: not created on this load).
    pub instance_to_moby: Vec<Option<MobyId>>,
    /// Runtime moby index → gameplay instance.
    pub moby_to_instance: Vec<usize>,
    /// Instances the spawn test rejected, in instance order. The game has no in-level path that creates them
    /// (only the loader reads the instance records): they appear on a later load of the level whose save state
    /// passes the test (`rc_formats::moby_spawn::spawn_test`, e.g. the flag-2 records once their mission is done).
    pub pending: Vec<usize>,
}

impl LevelStatics {
    /// The loader's groups (gameplay +0x48) through this load's instance → moby map.
    pub fn groups(&self, gameplay: &[u8]) -> Groups {
        Groups::parse(gameplay, &|i| self.instance_to_moby.get(i).copied().flatten())
    }
}

/// The loader's instance loop (`InitLevelRenderGlobals` 0x255958, after `InitMobyInstance`): the records whose
/// spawn test passed (`tests`, `rc_formats::moby_spawn::loader_spawns`) become consecutive mobys; the others
/// get `0x1acc00[i] = −1` and no slot. Per created moby: +0xb2 spawn id, +0xb1 / +0xb4 / +0xb6 from the test,
/// +0xb0 = record +0x04, scale = class scale · instance scale, draw / update distance, position, Euler, group,
/// occlusion word, pvars (`pvars` indexed by pvar index, moby links already remapped:
/// `rc_formats::gameplay::parse_pvars_spawned`), mode bits (OR'd into the class's +0x44 as well, so later
/// instances and `CreateMoby` see them: `classes` is updated), `MobyBuildMatrix`, ambient and light word. The
/// is-rooted ground snap (none on Novalis) is not applied.
pub fn load_level_mobys(
    instances: &[rc_formats::gameplay::MobyInstance],
    classes: &mut crate::moby_update::ClassTable,
    pvars: &[Option<Vec<u8>>],
    tests: &[rc_formats::moby_spawn::SpawnTest],
) -> LevelStatics {
    let mut out = LevelStatics { mobys: Vec::with_capacity(instances.len()), ..Default::default() };
    for (i, inst) in instances.iter().enumerate() {
        let t = tests.get(i).copied().unwrap_or(rc_formats::moby_spawn::SpawnTest { spawn: true, b1: 0xff, b4: 0, b6: 0 });
        if !t.spawn {
            out.instance_to_moby.push(None);
            out.pending.push(i);
            continue;
        }
        let id = out.mobys.len();
        out.instance_to_moby.push(Some(id));
        out.moby_to_instance.push(i);
        let oc = inst.o_class as i16;
        let info = classes.classes.get(&oc).map(|c| c.0);
        let mut m = Moby::init_instance(id as u32, oc, info.as_ref());
        crate::moby_update::anim_sound::init(&mut m, classes.classes.get(&oc).and_then(|c| c.1.as_ref()));
        m.spawn_id = inst.spawn_id as i16;
        m.spawn_flag = t.b1;
        m.b4 = t.b4;
        m.mission = inst.unknown_4 as u8;
        m.b6 = t.b6;
        if let Some(c) = info.filter(|c| !c.no_header) { m.scale = f32::from_bits((Pf(c.scale.to_bits()) * Pf(inst.scale.to_bits())).0); }
        m.draw_dist = inst.draw_distance as i16;
        m.update_dist = inst.update_distance as u8;
        m.position = [inst.position[0], inst.position[1], inst.position[2], 0.0];
        m.rotation = [inst.rotation[0], inst.rotation[1], inst.rotation[2], 0.0];
        m.group = inst.group as i8;
        m.occlusion = if inst.occlusion == 0 { 0 } else { 0x7f80 };
        m.pvars = inst.pvar(pvars).map(|p| p.to_vec()).unwrap_or_default();
        if let Some(c) = classes.classes.get_mut(&oc) { c.0.mode_bits |= inst.mode(); }
        m.mode |= inst.mode();
        if m.has_class {
            let anim = classes.classes.get(&oc).and_then(|c| c.1.as_ref());
            rebuild_matrix(&mut m, anim);
        }
        let rgb = inst.ambient_rgb();
        m.ambient = [rgb[0], rgb[1], rgb[2], 0];
        m.light = inst.light_word();
        out.mobys.push(m);
    }
    out
}

/// The spawn test's save inputs for level slot `level` of a game state (`LoadLevelCoreData`'s copy of the
/// mission bytes 0x14c050 → 0x15fc88, the killed bits 0x14c190, the spawner slots 0x14d590). This visit's
/// death bits 0x1ba950 and the per-id flags 0x1bbb04 are left empty (no save chunk of the port holds them;
/// empty on a first visit, as in the Novalis savestate).
pub fn spawn_save(level: &crate::game_state::LevelState) -> rc_formats::moby_spawn::SpawnSave {
    let mut s = rc_formats::moby_spawn::SpawnSave { missions: level.missions, killed: level.killed, ..Default::default() };
    for (slot, d) in s.spawner.iter_mut().zip(&level.bolt_drops) { *slot = d.first; }
    s
}

/// The visit's bits for the death reload's spawn test (`LoadLevelCoreData(0, 1)`, G-CLS-030): the death bits of this
/// visit `0x1ba950` ([`SaveBits::death_level`]), the per-id flags `0x1bbb04` ([`SaveBits::collected`]) and the persistent
/// death bits `0x14c190 + L·0x100` this visit set ([`SaveBits::death`], not yet in the saved game's chunk the loader's
/// `spawn_save` copied).
pub fn add_visit_bits(s: &mut rc_formats::moby_spawn::SpawnSave, v: &crate::moby_update::services::SaveBits, level: u32) {
    s.visit_death.extend(v.death_level.iter().map(|&id| id as i32));
    for (&id, &f) in &v.collected { s.id_flags.insert(id as i32, f); }
    for &(l, id) in &v.death {
        if l != level || id < 0 { continue; }
        let id = id as usize;
        if let Some(b) = s.killed.get_mut(id >> 3) { *b |= 1 << (id & 7); }
    }
}

/// The hits the last `UpdateParts` gave Ratchet (`Particles::hits`: type 58's `0x26eaa8(damage, hero, owner, 1, pos,
/// dir)`), as their hit records.
fn part_hits(w: &mut World) {
    // Type 40's fresh flame lines: `CollLine_Fix(a, b, 0, owner, {dir, owner, 0x10001, 1.0})` against the mobys.
    let lines = w.particles.as_deref_mut().map(|p| std::mem::take(&mut p.lines)).unwrap_or_default();
    for l in lines {
        use crate::moby_update::services::{pf, pv, HitTemplate};
        let owner = (l.owner as usize).checked_sub(1).filter(|&o| o < w.table.mobys.len());
        let t = HitTemplate { dir: pv(l.dir), attacker: owner, flags: 0x1_0001, damage: pf(1.0), w20: 1, ..Default::default() };
        crate::moby_update::services::line_hit_in(w.table, w.svc, w.classes, w.coll, pv(l.a), pv(l.b), 0, owner, &t);
    }
    let Some(hits) = w.particles.as_deref_mut().map(|p| std::mem::take(&mut p.hits)) else { return };
    let Some(hero) = w.hero_moby else { return };
    for h in hits {
        let Some(owner) = (h.owner as usize).checked_sub(1).filter(|&o| o < w.table.mobys.len()) else { continue };
        crate::moby_update::creature::attack::hit_moby(w, hero, owner, h.damage, 1, h.pos, h.dir);
    }
}
