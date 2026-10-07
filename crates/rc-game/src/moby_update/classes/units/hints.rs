//! The reads and inline patterns the level help directors share (docs/plan/level_scripting.md §4, §8). Not a
//! system of the game: each director inlines these few lines in its own code; they are written once here because
//! the directors 1413, 1000, 558, 422, 1348, 1349, 1344, 1343, 1342, 1324 and 1347 all repeat them byte for byte.
//!
//! * `PointInCuboid(0x13f3d0, P[off])` with Ratchet's feet: [`in_cuboid`].
//! * "group ok": the movement group 0x1413dc < 2 or = 9 ([`group_ok`]); "idle": the help box idle and nothing pending
//!   (0x179890 = 0, 0x1798b4 = −1: [`crate::help::Help::idle`]).
//! * a moby index of the pvars "gone": −1 is not tested by the callers themselves; the moby's state 0xfe / 0xfd
//!   (deleted) ([`gone`]).
//! * the **record bump** (`count++` unless 0xffff; `time = max(time, ScaleTicks(play) / 600)`; `mask |= 1 << level |
//!   0x80000000`): [`crate::help::bump`]; the **arm** (`count == 0` → `count = 1` and the time / mask part):
//!   [`arm_or_remind`];
//! * the **reminder** (`60·ScaleTicks(18) < ScaleTicks(play) − 600·time` or `time == 0` → `Help_Request`; else the
//!   time is refreshed): [`remind`], so a hint comes back when its trigger holds again after more than 18 s.

use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pvar as p, World};

/// `ScaleTicks` 0x220e30.
pub fn ticks(n: i32) -> i32 { crate::hud::scale_ticks(n) }

/// The pvar s32 at `off` (0 past the block).
pub fn pvi(w: &World, id: MobyId, off: usize) -> i32 { w.m(id).pvars.get(off..off + 4).map_or(0, |_| p::i32(&w.m(id).pvars, off)) }

pub fn set_pvi(w: &mut World, id: MobyId, off: usize, v: i32) {
    let pv = &mut w.mm(id).pvars;
    if pv.len() < off + 4 { pv.resize(off + 4, 0); }
    p::set_i32(pv, off, v);
}

/// `PointInCuboid(0x13f3d0, P[off])` (−1: never).
pub fn in_cuboid(w: &World, id: MobyId, off: usize) -> bool { w.in_cuboid(w.hero_point(), pvi(w, id, off)) }

/// The movement group 0x1413dc < 2 or = 9.
pub fn group_ok(w: &World) -> bool { (w.hero.group as u32) < 2 || w.hero.group == 9 }

/// The help box idle and no request pending (0x179890 = 0, 0x1798b4 = −1).
pub fn idle(w: &World) -> bool { w.svc.help.idle() }

/// Moby `idx` deleted (state 0xfe / 0xfd). An index outside the table: not gone [L] (the game reads past it).
pub fn gone(w: &World, idx: i32) -> bool { usize::try_from(idx).ok().and_then(|i| w.table.mobys.get(i)).is_some_and(|m| m.state == 0xfe || m.state == 0xfd) }

/// Moby `idx`'s class and state, when it is in the table.
pub fn class_state(w: &World, idx: i32) -> Option<(i16, u8)> { usize::try_from(idx).ok().and_then(|i| w.table.mobys.get(i)).map(|m| (m.o_class, m.state)) }

/// `Help_Request(msg, rec)`.
pub fn request(w: &mut World, msg: i32, rec: i32) { w.svc.help.request(msg, rec); }

/// The record bump of help record `r`.
pub fn bump(w: &mut World, r: usize) {
    let (l, t) = (w.svc.help.level, w.svc.help.play_time);
    crate::help::bump(&mut w.svc.help.records.help[r], l, t);
}

/// The record bump of move record `r` (0x141848 + 8r).
pub fn bump_move(w: &mut World, r: usize) {
    let (l, t) = (w.svc.help.level, w.svc.help.play_time);
    crate::help::bump(&mut w.svc.help.records.moves[r], l, t);
}

/// The time / mask part alone.
pub fn touch(w: &mut World, r: usize) {
    let (l, t) = (w.svc.help.level, w.svc.help.play_time);
    crate::help::touch(&mut w.svc.help.records.help[r], l, t);
}

/// Help record `r`'s time (10-second units) is more than `n` scaled ticks old (`ticks(play) − 600·time > ticks(n)`).
pub fn older_than(w: &World, time: u16, n: i32) -> bool { ticks(n) < ticks(w.svc.help.play_time) - 600 * time as i32 }

/// The reminder on help record `r`'s time (module doc).
pub fn remind(w: &mut World, r: usize, msg: i32, rec: i32) {
    let now = ticks(w.svc.help.play_time);
    let time = w.svc.help.records.help[r].time as i32;
    if ((ticks(0x12) as f32 * 60.0) as i32) < now - time * 600 || time == 0 {
        request(w, msg, rec);
    } else if time < now / 600 {
        w.svc.help.records.help[r].time = (now / 600) as u16;
    }
}

/// `count == 0` → count 1 and the time / mask update; else the reminder (module doc).
pub fn arm_or_remind(w: &mut World, r: usize, msg: i32, rec: i32) {
    if w.svc.help.records.help[r].count == 0 {
        w.svc.help.records.help[r].count = 1;
        touch(w, r);
    } else {
        remind(w, r, msg, rec);
    }
}

/// Item `i` owned (0x13d4c0[i]).
pub fn owned(w: &World, i: usize) -> bool { w.inventory.owned(i) }

/// Global flag `i` (0x13d388[i]) as this tick's class writes left it.
pub fn flag(w: &World, i: usize) -> bool { w.svc.interact.game.flags.get(i).is_some_and(|&b| b != 0) }

/// Global flag `i` := 1 (G-SAV-010: `interact::set_global_flag`), when it is not set already (the game stores 1 again
/// each tick; the value is the same).
pub fn set_flag(w: &mut World, i: usize) {
    if !flag(w, i) { crate::moby_update::interact::set_global_flag(w, i, 1); }
}

/// The mission byte `0x14c050[level·16 + m]` is 0xff (done).
pub fn mission_done(w: &World, m: i32) -> bool { u8::try_from(m).is_ok_and(|m| w.mission_done(w.svc.level, m) == 0xff) }

/// The distance from Ratchet's feet to moby `idx` (`VecDistance`).
pub fn hero_dist(w: &World, idx: usize) -> f32 {
    let (h, m) = (w.hero_point(), w.m(idx).position);
    ((h[0] - m[0]).powi(2) + (h[1] - m[1]).powi(2) + (h[2] - m[2]).powi(2)).sqrt()
}

/// The moby loop's run list (the 0x15ffe4 chain the directors walk).
pub fn run_list(w: &World) -> Vec<MobyId> { crate::moby_update::scheduler::build_active_list(w.table, w.camera, &w.svc.groups).0 }

/// The 1341 / 1343 crank hint: Ratchet just left the crank state (previous state 0x3b, state timer 0): each class-280
/// crank of the run list within 3 whose progress (P+0) is not 1.0 adds one to the counter at `counter`; 1006 (record
/// 0x42) on the 3rd and 6th.
pub fn crank_tries(w: &mut World, id: MobyId, counter: usize) {
    if w.hero.prev_state != 0x3b || w.hero.timer != 0 { return; }
    for c in run_list(w) {
        if w.m(c).o_class != 0x118 || 3.0 <= hero_dist(w, c) { continue; }
        if w.m(c).pvars.len() >= 4 && p::ff(&w.m(c).pvars, 0) == 1.0 { continue; }
        let n = pvi(w, id, counter) + 1;
        set_pvi(w, id, counter, n);
        if n == 3 || n == 6 { request(w, 0x3ee, 0x42); }
    }
}
