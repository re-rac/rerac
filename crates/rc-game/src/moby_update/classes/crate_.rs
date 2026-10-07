//! Crates, classes 500 (bolt crate), 501 (nanotech crate), 502 (reinforced), 505 (TNT), 511 (ammo crate):
//! `CrateUpdate` level01 0x2ea178 with the stacking init `FUN_002eac18`, the stack physics `FUN_002ec388`,
//! the break effect `CrateBreakFx` 0x2eb918 and the drop `CrateDropBolts` 0x2eb498 → `SetDeathBits` 0x26c250 →
//! `BoltBurst` 0x275988 → `BoltSpawn` 0x2bcdb8 (bolts) or the ammo pickups (`pickup::ammo_pick` 0x26bff8,
//! `pickup::pickup_spawn` 0x2daf10). Spec: `docs/plan/moby_update_catalogue.md` "In the port: crates" and
//! `docs/plan/hero_gameplay.md` §2.
//!
//! **What each crate gives** (read from `CrateDropBolts` and the class 806 update; the catalogue's "501 ammo crate /
//! 511 nanotech crate" names were swapped): 500 / 502 / 505 bolts (`SetDeathBits`: the placed +0xb4 count); **511 the
//! ammo crate** (init sets pvar+0xc6 = 100): 1 or 2 ammo pickups of pvar+0xcb's item or a re-picked owned one
//! (`pickup::ammo_update`); **501 the nanotech crate**: no drop of its own (its +0xb4 is cleared before the death
//! bits); the cluster 806 its init creates (`CrateSpawnIconMoby`) holds 8 nanotech orbs over it, which fly to
//! Ratchet and heal 1 HP once the crate is gone (`pickup::nanotech_update`).
//!
//! Pvar block (0x100 bytes): 0x3e u16 flags (bit 0 = nothing on top), 0x40 vec velocity (0x48 = vz),
//! 0xa0 / 0xa4 crate above / below (pointers), 0xa8, 0xac u32 flags (1 stacking done, 2 physics off, 4 moved,
//! 8 on a platform / moving, 0x10), 0xb0 f32 height (1.0), 0xb4 tick of the last stack pass, 0xb8 saved light
//! word + ambient (8 bytes), 0xc0 s32 path (−1), 0xc4 u16, 0xc6 s16 drop mode (≠ 0: ammo pickups; 511 sets
//! 100), 0xc8 u8 respawns, 0xc9 u8 respawn timer, 0xca u8 TNT blast growth, 0xcb u8 ammo type, 0xcc,
//! 0xd0 / 0xe0 platform-local pose, 0xf0 platform, 0xf4 f32 platform height, 0xf8, 0xfc tick.
//! Pointers are stored as `moby index + 1` (0 = null).
//!
//! **Crates on carriers** (G-CLS-024, `triggers`' moby side): the stacking init attaches a crate that settles on a
//! carrier ([`attach_platform`]; the stack above it inherits the pose one height up), and the stack physics then
//! places it through the carrier's frame every tick ([`ride_platform`]).
//!
//! **Not ported, with the evidence** (G-CLS-009, closed 2026-09-30): the multi-hit branch (classes 0x1fb–0x1fd,
//! `FUN_0026f378` and the class swap toward 0x1fe) is code no level reaches: no level's class table (`lvl.vtbl`)
//! maps 503, 504 or 506–510 to the crate update or to anything else (each maps only 500 / 501 / 502 / 505 / 511,
//! level 00 / 02 / 03 / 04 / 08 / 13 / 17 without 502), so no such crate is ever loaded, placed or created.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use crate::hero::physics::{self as ph, V4};
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::{bolt, debris, pickup};
use crate::moby_update::services::{self as sv, normalize_angle, pvar as p, pv, fv, HitRecord, HitTemplate, World, DT, DT2};
use crate::pad::fast_arctan as atan;
use crate::ps2v::Pf;

/// The crate update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x2ea178;

/// Classes that run [`update`] (0x1fb–0x1fd, the multi-hit crates, are not ported).
pub const CLASSES: [i16; 5] = [500, 501, 502, 505, 511];

/// The hit mask of crates and bolts.
pub const HIT_MASK: u32 = 0x183_0000;

const TNT: i16 = 0x1f9;

fn ptr(pvars: &[u8], o: usize) -> Option<MobyId> { let v = p::i32(pvars, o); (v > 0).then(|| (v - 1) as usize) }
fn set_ptr(pvars: &mut [u8], o: usize, id: Option<MobyId>) { p::set_i32(pvars, o, id.map(|i| i as i32 + 1).unwrap_or(0)); }

/// `FUN_00273278`: a live moby of class 500..540.
pub(crate) fn is_crate(w: &World, id: Option<MobyId>) -> bool {
    match id {
        Some(i) if i < w.table.mobys.len() => (w.m(i).o_class as i32 - 500) as u32 & 0xffff < 0x29,
        _ => false,
    }
}

/// `m+0x38` as the crates save it (light word, ambient bytes).
fn save_light(w: &mut World, id: MobyId) {
    let (l, a) = { let m = w.m(id); (m.light, m.ambient) };
    let m = w.mm(id);
    p::set_u32(&mut m.pvars, 0xb8, l);
    m.pvars[0xbc..0xc0].copy_from_slice(&a);
}
fn restore_light(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.light = p::u32(&m.pvars, 0xb8);
    let a = [m.pvars[0xbc], m.pvars[0xbd], m.pvars[0xbe], m.pvars[0xbf]];
    m.ambient = a;
}

/// `CrateUpdate` (0x2ea178).
pub fn update(w: &mut World, id: MobyId) {
    // The layout is 0x100 bytes (a shorter block would make the game read the next pvar block: zeros here).
    if w.m(id).pvars.len() < 0x100 { w.mm(id).pvars.resize(0x100, 0); }
    let tick = w.counter as i32;
    for o in [0xa0, 0xa4] {
        let v = ptr(&w.m(id).pvars, o);
        if v.is_some() && !is_crate(w, v) { set_ptr(&mut w.mm(id).pvars, o, None); }
    }
    {
        let m = w.mm(id);
        p::set_i32(&mut m.pvars, 0xfc, tick);
        let f = p::u16(&m.pvars, 0x3e);
        let f = if p::i32(&m.pvars, 0xa0) == 0 { f | 1 } else { f & 0xfffe };
        p::set_i16(&mut m.pvars, 0x3e, f as i16);
    }
    {
        let m = w.m(id);
        if p::i32(&m.pvars, 0xb4) != tick && m.state != 0 && p::u32(&m.pvars, 0xac) & 0xc != 0 { stack_physics(w, id); }
    }
    let mut hit = w.get_hit(id, HIT_MASK, false);
    let mut contact = false;
    if hit.map(|h| h.flags == 0x100_0000).unwrap_or(false) {
        hit = None;
        contact = true;
    }
    let class = w.m(id).o_class;
    match w.m(id).state {
        0 => init(w, id),
        1 => idle(w, id, hit, contact),
        2 => {
            let m = w.mm(id);
            m.cmd = m.cmd.wrapping_sub(1);
            if m.cmd == 0xff {
                m.hit_slot = 0xff;
                m.state = 1;
            }
        }
        3 => {
            if class == TNT && (w.m(id).pvars[0xca] as i32) <= w.ticks(0x14) {
                blast(w, id);
            } else if w.m(id).pvars[0xc8] == 0 {
                w.delete_moby(id);
                return;
            } else {
                let lvl12 = w.svc.level == 0x12;
                {
                    let m = w.mm(id);
                    m.has_collision = false;
                    m.mode = (m.mode | mode::HIDDEN) & !mode::TARGETABLE;
                    m.visible = 0;
                    if m.b4 >= 2 { m.b4 = 1; }
                    m.state = 6;
                }
                let t = if lvl12 { w.rng.rand_range(300, 0x2d0) } else { w.rng.rand_range(0x1e, 0x3c) };
                w.mm(id).pvars[0xc9] = t as u8;
            }
        }
        4 => {
            let m = w.mm(id);
            if sv::fast_dec_timer_u8(&mut m.cmd) != 0 {
                m.state = 1;
                m.mode &= !mode::HIDDEN;
            }
        }
        5 => fuse(w, id, hit),
        6 => respawn(w, id),
        _ => {}
    }
    w.mm(id).hit_slot = 0xff;
}

/// State 0.
fn init(w: &mut World, id: MobyId) {
    let lvl = w.svc.level;
    {
        let m = w.mm(id);
        p::set_v4(&mut m.pvars, 0x40, [Pf::ZERO; 4]);
        if lvl == 0x12 && m.o_class == 0x1ff && m.pvars[0xc8] != 0 {
            m.state = 6;
            m.has_collision = false;
            m.mode = (m.mode | 0x41) & 0xefff;
        } else {
            m.state = 1;
        }
        p::set_i32(&mut m.pvars, 0xa8, 0);
    }
    if p::u32(&w.m(id).pvars, 0xac) & 1 == 0 { stack_init(w, id); }
    if w.m(id).is_deleted() { return; }
    let class = w.m(id).o_class;
    {
        let m = w.mm(id);
        let f = p::u32(&m.pvars, 0xac) | 0x14;
        p::set_u32(&mut m.pvars, 0xac, f);
        if class == 0x1ff { p::set_i16(&mut m.pvars, 0xc6, 100); }
    }
    if class == 0x1f5 { spawn_icon(w, id); }
}

/// `CrateSpawnIconMoby` 0x300528: the nanotech cluster (class 806) of a nanotech crate.
fn spawn_icon(w: &mut World, id: MobyId) {
    if w.m(id).state >= 0x80 { return; }
    let Some(i) = w.create_moby(0x326) else { return };
    let pos = w.m(id).position;
    let m = w.mm(i);
    m.update_dist = 0x40;
    m.state = 0;
    set_ptr(&mut m.pvars, 0xc, Some(id));
    m.has_collision = false;
    m.mode |= 0x41;
    m.position = pos;
}

/// Crate probe of the stacking init: `CollLine_Fix((x, y, z + up), (x, y, 0.1), 2)`.
fn probe(w: &World, id: MobyId, up: Pf) -> Option<sv::LineHit> {
    let pos = pv(w.m(id).position);
    let a = [pos[0], pos[1], pos[2] + up, pos[3]];
    let b = [pos[0], pos[1], Pf::b(0x3dcc_cccd), pos[3]];
    w.line(a, b, 2, None)
}

/// Steps 1–3 of the per-crate part of the stacking init: the random quarter turn of class 500 (one `randi(2)`
/// per class-500 crate), the ammo-crate removal, and the stack fields. Returns false when the crate was
/// deleted.
fn settle_fields(w: &mut World, c: MobyId) -> bool {
    if w.m(c).o_class == 500 {
        let r = w.rng.randi(2);
        let m = w.mm(c);
        if r != 0 && m.rotation[0].to_bits() & 0x7fff_ffff == 0 && m.rotation[1].to_bits() & 0x7fff_ffff == 0 {
            m.rotation[2] = sv::fl(ph::fast_add_rotations(sv::pf(m.rotation[2]), Pf::b(0x3fc9_0fdb)));
        }
    }
    let (class, mission) = { let m = w.m(c); (m.o_class, m.mission) };
    if class == 0x1f5 && mission != 0xff && w.missions.ammo_crate_gate(mission) < p::i32(&w.m(c).pvars, 0xf8) {
        w.delete_moby(c);
        return false;
    }
    let m = w.mm(c);
    let uid_lo = m.uid_hi as u16;
    p::set_i16(&mut m.pvars, 0xc4, uid_lo as i16);
    p::set_f(&mut m.pvars, 0xb0, Pf::ONE);
    let f = p::u32(&m.pvars, 0xac) | 1;
    p::set_u32(&mut m.pvars, 0xac, f);
    true
}

/// `FUN_002eac18`: stack the crates of the group (or the lone crate) on what is below them.
fn stack_init(w: &mut World, id: MobyId) {
    {
        let m = w.mm(id);
        let f = p::u32(&m.pvars, 0xac) | 4;
        p::set_u32(&mut m.pvars, 0xac, f);
    }
    let g = w.m(id).group;
    if g < 0 {
        if !settle_fields(w, id) { return; }
        match probe(w, id, Pf::b(0x3f00_0000)) {
            None => set_ptr(&mut w.mm(id).pvars, 0xa4, None),
            Some(h) if h.moby.is_none() => {
                let m = w.mm(id);
                m.position[2] = sv::fl(h.point[2]);
                set_ptr(&mut m.pvars, 0xa4, None);
            }
            Some(h) if is_crate(w, h.moby) => {}
            Some(h) => {
                let m = w.mm(id);
                set_ptr(&mut m.pvars, 0xa4, None);
                let f = p::u32(&m.pvars, 0xac) | 8;
                p::set_u32(&mut m.pvars, 0xac, f);
                if let Some(k) = h.moby { attach_platform(w, id, k); }
            }
        }
        return;
    }
    let members = group_members(w, g);
    // Loop 1.
    for &c in &members {
        if w.m(c).state >= 0x80 || !is_crate(w, Some(c)) { continue; }
        if !settle_fields(w, c) { continue; }
        let hit = probe(w, c, Pf::b(0x3f00_0000));
        match hit {
            None => set_ptr(&mut w.mm(c).pvars, 0xa4, None),
            Some(h) if h.moby.is_none() => {
                let m = w.mm(c);
                m.position[2] = sv::fl(h.point[2]);
                set_ptr(&mut m.pvars, 0xa4, None);
            }
            Some(h) => {
                let hm = h.moby.unwrap();
                let hc = w.m(hm).o_class;
                if !is_crate(w, Some(hm)) && (hc as i32 - 0x128) as u32 & 0xffff >= 2 {
                    let m = w.mm(c);
                    set_ptr(&mut m.pvars, 0xa4, None);
                    m.update_dist = 0xff;
                    let f = p::u32(&m.pvars, 0xac) | 8;
                    p::set_u32(&mut m.pvars, 0xac, f);
                } else if w.m(hm).pvars.is_empty() || w.m(hm).group != w.m(c).group {
                    set_ptr(&mut w.mm(c).pvars, 0xa4, None);
                } else if is_crate(w, Some(hm)) {
                    let (hpos, hup) = { let h = w.m(hm); (pv(h.position), pv(h.rows[2])) };
                    {
                        let m = w.mm(c);
                        set_ptr(&mut m.pvars, 0xa4, Some(hm));
                        m.position = fv(ph::vadd(hpos, hup));
                    }
                    set_ptr(&mut w.mm(hm).pvars, 0xa0, Some(c));
                } else {
                    let m = w.mm(c);
                    p::set_i32(&mut m.pvars, 0xcc, 0);
                    set_ptr(&mut m.pvars, 0xa4, None);
                    m.position[2] = sv::fl(h.point[2]);
                }
            }
        }
    }
    // Loop 2: bottom crates (a platform under them is not ported), then up each stack.
    for &c2 in &members {
        if w.m(c2).state >= 0x80 || !is_crate(w, Some(c2)) || p::i32(&w.m(c2).pvars, 0xa4) != 0 { continue; }
        let up = p::f(&w.m(c2).pvars, 0xb0) * Pf::b(0x3f00_0000);
        if let Some(k) = probe(w, c2, up).and_then(|h| h.moby) {
            if attach_platform(w, c2, k) {
                let m = w.mm(c2);
                let f = p::u32(&m.pvars, 0xac) | 8;
                p::set_u32(&mut m.pvars, 0xac, f);
            }
        }
        let mut cur = Some(c2);
        while let Some(c) = cur {
            if let Some(b) = ptr(&w.m(c).pvars, 0xa4) {
                let bcc = p::i32(&w.m(b).pvars, 0xcc);
                p::set_i32(&mut w.mm(c).pvars, 0xcc, bcc);
                inherit_platform(w, c, b);
            }
            let (c6, class, above, path) = { let m = w.m(c); (p::i16(&m.pvars, 0xc6), m.o_class, p::i32(&m.pvars, 0xa0), p::i32(&m.pvars, 0xc0)) };
            if (c6 != 0 || class == 0x1f5) && above != 0 && path != -1 {
                path_counter(w, c, path);
            }
            cur = ptr(&w.m(c).pvars, 0xa0);
        }
    }
    // Loop 3 (distances between every pair, results unused): nothing to do.
}

/// A crate settling on moby `k` (the stacking init's probe hit `0x1742d8`): when `k` is a carrier
/// (`FUN_00275290`), the occlusion bits +0x36 = 0x7f80, the crate's pose in the carrier's frame
/// (`FUN_00275528(…, k, pos, rot, +0xd0, +0xe0)`), +0xf4 = 0 (its height above that pose), +0xf0 = the carrier.
/// Returns whether it attached. (In the group branch the game writes the occlusion bits of whatever moby its loop
/// variable last held, the previous stack's top crate [L]; the port writes the crate's own.)
fn attach_platform(w: &mut World, id: MobyId, k: MobyId) -> bool {
    let Some(c) = w.table.mobys.get(k).and_then(crate::moby_update::triggers::carrier) else { return false };
    let (pos, rot) = { let m = w.m(id); ([m.position[0], m.position[1], m.position[2]], [m.rotation[0], m.rotation[1], m.rotation[2]]) };
    let (l, lr) = crate::moby_update::triggers::to_local(&c, pos, rot);
    let m = w.mm(id);
    m.occlusion = 0x7f80;
    let (dw, ew) = (p::f(&m.pvars, 0xdc), p::f(&m.pvars, 0xec));
    p::set_v4(&mut m.pvars, 0xd0, [sv::pf(l[0]), sv::pf(l[1]), sv::pf(l[2]), dw]);
    p::set_v4(&mut m.pvars, 0xe0, [sv::pf(lr[0]), sv::pf(lr[1]), sv::pf(lr[2]), ew]);
    p::set_f(&mut m.pvars, 0xf4, Pf::ZERO);
    set_ptr(&mut m.pvars, 0xf0, Some(k));
    true
}

/// Loop 2 of the stacking init, up a stack: the crate `b` below `c` rides a carrier, so `c` does too, one crate
/// height higher in its frame: update distance 0xff, flags | 8, +0xd0 / +0xe0 / +0xf0 copied, +0xf4 = b's + 1.
fn inherit_platform(w: &mut World, c: MobyId, b: MobyId) {
    if p::i32(&w.m(b).pvars, 0xf0) == 0 { return; }
    let (d0, e0, f0, f4) = { let bp = &w.m(b).pvars; (p::v4(bp, 0xd0), p::v4(bp, 0xe0), p::i32(bp, 0xf0), p::f(bp, 0xf4)) };
    let m = w.mm(c);
    m.update_dist = 0xff;
    let f = p::u32(&m.pvars, 0xac) | 8;
    p::set_u32(&mut m.pvars, 0xac, f);
    p::set_v4(&mut m.pvars, 0xd0, d0);
    p::set_v4(&mut m.pvars, 0xe0, e0);
    p::set_i32(&mut m.pvars, 0xf0, f0);
    p::set_f(&mut m.pvars, 0xf4, f4 + Pf::ONE);
}

/// The crate's carrier (+0xf0) as a carrier view (None: none, or no longer a carrier).
fn platform_of(w: &World, id: MobyId) -> Option<crate::moby_update::triggers::Carrier> {
    ptr(&w.m(id).pvars, 0xf0).and_then(|k| w.table.mobys.get(k)).and_then(crate::moby_update::triggers::carrier)
}

/// `FUN_002ec388`'s branch for a crate on a carrier (+0xf0 ≠ 0), after the gravity on +0x48: the height above the
/// recorded pose +0xf4 settles (`Approach` toward 0, or the crate below's +0xf4 + 1, by `|+0x4c|` with +0x4c −=
/// 10·dt² per tick; +0x4c = 0 once there); the occlusion bits +0x36 = 0x7f80; the pose is the local one (+0xd0
/// raised by +0xf4, +0xe0) through the carrier's rows now (`FUN_002753b0`; block flag bit 2 re-records +0xd0
/// without the height); the velocity +0x40 = this tick's move (w: +0x4c) and `CarryRiders(+0x60, +0x40, old rot,
/// rot)`: a crate on a platform carries what stands on it.
fn ride_platform(w: &mut World, c: MobyId) {
    let below = ptr(&w.m(c).pvars, 0xa4);
    let t = match below { None => 0.0, Some(b) => p::ff(&w.m(b).pvars, 0xf4) + 1.0 };
    {
        let m = w.mm(c);
        let v = p::ff(&m.pvars, 0x4c) - sv::fl(DT2) * 10.0;
        p::set_ff(&mut m.pvars, 0x4c, v);
        let mut h = p::ff(&m.pvars, 0xf4);
        let left = crate::moby_update::creature::turn::approach(t, v.abs(), &mut h);
        p::set_ff(&mut m.pvars, 0xf4, h);
        if left == 0.0 { p::set_ff(&mut m.pvars, 0x4c, 0.0); }
        m.occlusion = 0x7f80;
    }
    let (old_p, old_r) = { let m = w.m(c); (m.position, m.rotation) };
    let Some(k) = platform_of(w, c) else { return };
    let (d0, h) = { let m = w.m(c); (p::v4f(&m.pvars, 0xd0), p::ff(&m.pvars, 0xf4)) };
    let mut l = [d0[0], d0[1], d0[2] + h];
    let e0 = p::v4f(&w.m(c).pvars, 0xe0);
    let mut lr = [e0[0], e0[1], e0[2]];
    let slot = w.m(c).class_slot;
    let (q, qr) = crate::moby_update::triggers::from_local(&k, slot, &mut l, &mut lr);
    let m = w.mm(c);
    p::set_v4f(&mut m.pvars, 0xe0, [lr[0], lr[1], lr[2], e0[3]]);
    if k.delta.flags & 4 != 0 { p::set_v4f(&mut m.pvars, 0xd0, [l[0], l[1], l[2] - h, d0[3]]); }
    m.position[..3].copy_from_slice(&q);
    m.rotation[..3].copy_from_slice(&qr);
    let w4c = p::ff(&m.pvars, 0x4c);
    let vel = [q[0] - old_p[0], q[1] - old_p[1], q[2] - old_p[2], w4c];
    p::set_v4f(&mut m.pvars, 0x40, vel);
    let new_r = m.rotation;
    if m.pvars.len() >= 0xa0 { crate::moby_update::triggers::carry_riders(&mut m.pvars, 0x60, vel, old_r, new_r); }
}

/// The spline counter of the ammo-crate stacks (loop 2 of `FUN_002eac18`): the first point's w word counts.
fn path_counter(w: &mut World, c: MobyId, path: i32) {
    let Some(s) = usize::try_from(path).ok().filter(|&i| i < w.svc.splines.len()) else { return };
    let n = w.svc.splines[s].len() as i32;
    let k = if n > 0 { w.svc.splines[s][0][3] as i32 } else { 0 };
    let clear = |w: &mut World| p::set_i32(&mut w.mm(c).pvars, 0xc0, -1);
    if n == 0 || k == 1 || k == 3 || k == 0x111 { clear(w); return; }
    if (k.wrapping_sub(0x11) as u32) < 0x100 {
        if k - 0x10 < n { w.svc.splines[s][0][3] = (k + 1) as u32; } else { clear(w); }
        return;
    }
    w.svc.splines[s][0][3] = 0x11;
    for pt in w.svc.splines[s].iter_mut() { pt[3] = 0; }
}

/// The group's members in list order, skipping deleted mobys (`FUN_0026e150` / `FUN_0026e238` with the filter (0, 0)).
fn group_members(w: &World, g: i8) -> Vec<MobyId> {
    use crate::moby_update::scheduler::{group_walk, GroupWalk};
    group_walk(w, g as i32, GroupWalk::Alive)
}

/// State 1: wait for a hit (or, for TNT, a touch).
fn idle(w: &mut World, id: MobyId, hit: Option<HitRecord>, contact: bool) {
    let class = w.m(id).o_class;
    if class == TNT {
        let h = w.hero;
        let touched = h.cap_moby == Some(id) || h.wall_moby == Some(id) || (h.ground_moby == Some(id) && h.air_ticks == 0) || contact;
        if touched {
            let m = w.mm(id);
            m.cmd = 0;
            m.state = 5;
            return;
        }
    }
    let Some(h) = hit else { return };
    if !(Pf::ZERO < h.damage) { return; }
    let mut brk = true;
    if class == 0x1f6 && h.flags & 0x2_0000 == 0 { brk = false; }
    if class == TNT {
        w.mm(id).state = 5;
        let bc = if h.flags & 0x2_0000 != 0 {
            1
        } else {
            let r = Pf(w.rng.randf_bits(0x40a0_0000, 0x4170_0000));
            w.svc.timing.scale(r).to_i32() as u8
        };
        w.mm(id).cmd = bc;
        save_light(w, id);
        brk = false;
    }
    if brk {
        break_fx(w, id, class == TNT, Some(h));
        let special = w.m(id).pvars[0xc8] != 0
            && h.attacker.map(|a| matches!(w.m(a).o_class, 0x2cd | 0x22c)).unwrap_or(false);
        if special {
            let b4 = w.m(id).b4;
            w.mm(id).b4 = 0;
            drop_bolts(w, id);
            w.mm(id).b4 = b4;
        } else {
            drop_bolts(w, id);
        }
    }
}

/// State 5: the TNT fuse (`moby+0xbc`): 179 ticks from a touch, beeps at 177 / 117 / 57 / 13, red blink.
fn fuse(w: &mut World, id: MobyId, hit: Option<HitRecord>) {
    let (t60, t15, t13, t40) = (w.ticks(0x3c), w.ticks(0xf), w.ticks(0xd), w.ticks(0x28));
    if w.m(id).cmd == 0 {
        w.mm(id).cmd = (t60 * 3 - 1) as u8;
        save_light(w, id);
    } else if t15 < w.m(id).cmd as i32 && hit.is_some() {
        let r = Pf(w.rng.randf_bits(0x40a0_0000, 0x4170_0000));
        w.mm(id).cmd = w.svc.timing.scale(r).to_i32() as u8;
    }
    let bc = w.m(id).cmd as i32;
    if bc % t60 == t60 - 3 || bc == t13 { w.play_sound(1, 0x20, id); }
    let bc = w.m(id).cmd as i32;
    if t40 < bc % t60 || bc < t15 {
        w.mm(id).ambient = [200, 0x40, 0x40, 0];
    } else {
        restore_light(w, id);
    }
    if w.m(id).cmd == 1 || w.hero.group == 0x16 {
        restore_light(w, id);
        break_fx(w, id, true, None);
        drop_bolts(w, id);
    }
    let m = w.mm(id);
    m.cmd = m.cmd.wrapping_sub(1);
}

/// State 3 of the TNT crate: the blast grows over `ticks(20)` updates (radius `k / (20·scale)`), hitting the
/// hero (flags 1, damage 2, radius 2r) and everything else (flags 0x30000, radius 3r) each update.
fn blast(w: &mut World, id: MobyId) {
    let (pos, up, k) = { let m = w.m(id); (pv(m.position), pv(m.rows[2]), m.pvars[0xca]) };
    let c = ph::vadd(ph::set_len3(up, Pf::b(0x3f00_0000)), pos);
    let f = Pf::from_i32(k as i32) / w.svc.timing.scale(Pf::b(0x41a0_0000));
    let mut t = HitTemplate { attacker: Some(id), flags: 1, damage: Pf::b(0x4000_0000), ..Default::default() };
    w.sphere_mobys(f + f, c, 0x10, Some(id), Some(&t));
    t.flags = 0x3_0000;
    let hero = w.hero_moby;
    w.sphere_mobys(f * Pf::b(0x4040_0000), c, 0x10, hero, Some(&t));
    let m = w.mm(id);
    m.pvars[0xca] = m.pvars[0xca].wrapping_add(1);
}

/// State 6: hidden, waiting out of view to respawn (on the group's crate below, or on the ground).
fn respawn(w: &mut World, id: MobyId) {
    let (pos, dd) = { let m = w.m(id); (pv(m.position), m.draw_dist) };
    let sphere = [sv::fl(pos[0]), sv::fl(pos[1]), sv::fl(pos[2] + Pf::b(0x3f00_0000)), 1.0];
    let culled = w.view.map(|v| v.culled(dd as f32, sphere)).unwrap_or(false);
    if !culled { return; }
    if sv::fast_dec_timer_u8(&mut w.mm(id).pvars[0xc9]) == 0 { return; }
    let class_coll = w.classes.info(w.m(id).o_class).map(|c| c.has_collision).unwrap_or(false);
    let show = |w: &mut World| {
        let m = w.mm(id);
        m.visible = 1;
        m.state = 1;
        m.mode = (m.mode & !mode::HIDDEN) | mode::TARGETABLE;
        m.has_collision = class_coll;
    };
    let g = w.m(id).group;
    if g < 0 {
        show(w);
        return;
    }
    if w.svc.level == 0x12 {
        let inv = w.inventory;
        let (mut own, mut low) = (0, 0);
        for i in 0..0x25 {
            if inv.owned(i) {
                own += 1;
                let max = inv.max_ammo(i);
                if max != 0 && Pf::from_i32(inv.ammo(i)) < Pf::from_i32(max as i32) * Pf::b(0x3f4c_cccd) { low += 1; }
            }
        }
        let ok = low >= 3 || (own < 3 && low == own);
        if !ok { return; }
    }
    for c in group_members(w, g) {
        if c == id { continue; }
        let d = ph::dist2(pv(w.m(id).position), pv(w.m(c).position));
        if Pf::b(0x3dcc_cccd) < d || w.m(c).state != 1 || p::i32(&w.m(c).pvars, 0xa0) != 0 { continue; }
        let cpos = w.m(c).position;
        set_ptr(&mut w.mm(c).pvars, 0xa0, Some(id));
        show(w);
        let m = w.mm(id);
        m.position = cpos;
        m.position[2] = sv::fl(sv::pf(m.position[2]) + Pf::ONE);
        set_ptr(&mut m.pvars, 0xa0, None);
        set_ptr(&mut m.pvars, 0xa4, Some(c));
        let f = p::u32(&m.pvars, 0xac) | 4;
        p::set_u32(&mut m.pvars, 0xac, f);
        break;
    }
    if w.m(id).state != 1 {
        let z = w.ground_height(Pf::b(0x3f00_0000), pv(w.m(id).position), 0);
        {
            let m = w.mm(id);
            m.position[2] = sv::fl(z);
            set_ptr(&mut m.pvars, 0xa4, None);
            set_ptr(&mut m.pvars, 0xa0, None);
        }
        show(w);
    }
}

/// `FUN_002ec388`: the stack pass, bottom to top: the hero standing inside a crate breaks it; gravity
/// `9.8·dt²` (clamped ±0.3) until the crate rests on the ground (a `CollLine_Fix` probe) or on the crate
/// below; a hard landing lights a TNT fuse.
fn stack_physics(w: &mut World, id: MobyId) {
    let tick = w.counter as i32;
    if p::i32(&w.m(id).pvars, 0xb4) == tick { return; }
    let mut c = id;
    while let Some(b) = ptr(&w.m(c).pvars, 0xa4) { c = b; }
    let mut cur = Some(c);
    while let Some(c) = cur {
        let next = ptr(&w.m(c).pvars, 0xa0);
        {
            let m = w.mm(c);
            let mut f = p::u32(&m.pvars, 0xac);
            if f & 4 == 0 { f &= !0x10; }
            f &= !4;
            p::set_u32(&mut m.pvars, 0xac, f);
        }
        if !is_crate(w, Some(c)) || p::u32(&w.m(c).pvars, 0xac) & 2 != 0 { return; }
        p::set_i32(&mut w.mm(c).pvars, 0xb4, tick);
        if w.m(c).state == 0 { cur = next; continue; }
        // Hero inside the crate's footprint, below its top and above its capsule bottom → break.
        let (cpos, rows) = { let m = w.m(c); (pv(m.position), [pv(m.rows[0]), pv(m.rows[1]), pv(m.rows[2])]) };
        let d = ph::vsub(w.hero.pos, cpos);
        let l = ph::mul_rows3(&rows, d);
        let k = Pf::b(0x3f0c_cccd);
        let st = w.m(c).state;
        if l[0].abs() < k && l[1].abs() < k && l[2] < Pf::ZERO && -(w.hero.cap_top + w.hero.cap_radius) < l[2] && (st == 1 || st == 5) {
            let tnt = w.m(c).o_class == TNT;
            break_fx(w, c, tnt, None);
            drop_bolts(w, c);
        }
        let mut vz = p::f(&w.m(c).pvars, 0x48) - DT2 * Pf::b(0x411c_cccd);
        p::set_f(&mut w.mm(c).pvars, 0x48, vz);
        if vz < Pf::b(0xbe99_999a) { vz = Pf::b(0xbe99_999a); p::set_f(&mut w.mm(c).pvars, 0x48, vz); }
        if Pf::b(0x3e99_999a) < vz { vz = Pf::b(0x3e99_999a); p::set_f(&mut w.mm(c).pvars, 0x48, vz); }
        if p::i32(&w.m(c).pvars, 0xf0) != 0 {
            ride_platform(w, c);
            cur = next;
            continue;
        }
        let below = ptr(&w.m(c).pvars, 0xa4);
        let pos = pv(w.m(c).position);
        let (hit, sup) = match below {
            None => {
                let a = [pos[0], pos[1], pos[2] + p::f(&w.m(c).pvars, 0xb0) * Pf::b(0x3f40_0000), pos[3]];
                let b = [pos[0], pos[1], pos[2] + vz, pos[3]];
                match w.line(a, b, 2, None) {
                    Some(h) => (true, h.point),
                    None => (false, [Pf::ZERO; 4]),
                }
            }
            Some(b) => {
                let mut s = pv(w.m(b).position);
                s[2] = s[2] + Pf::ONE;
                (pos[2] + vz < s[2], s)
            }
        };
        if !hit {
            let m = w.mm(c);
            let f = p::u32(&m.pvars, 0xac) | 4;
            p::set_u32(&mut m.pvars, 0xac, f);
            let v = p::v4(&m.pvars, 0x40);
            m.position = fv(ph::vadd(pv(m.position), v));
        } else {
            if DT * Pf::b(0x3f00_0000) < ph::len3(p::v4(&w.m(c).pvars, 0x40)) {
                if w.m(c).o_class == TNT && w.m(c).state == 1 {
                    let m = w.mm(c);
                    m.cmd = 0;
                    m.state = 5;
                } else if let Some(b) = below.filter(|&b| w.m(b).o_class == TNT && w.m(b).state == 1) {
                    let m = w.mm(b);
                    m.state = 5;
                    m.cmd = 0;
                }
            }
            let m = w.mm(c);
            m.position[2] = sv::fl(sup[2]);
            p::set_f(&mut m.pvars, 0x48, Pf::ZERO);
        }
        cur = next;
    }
}

/// `CrateBreakFx(m, tnt, hit)` 0x2eb918: sound (rate limited), unlink from the stack, dust (6 type-13
/// particles), debris mobys (count by free slots), hide the crate (state 3); TNT adds sparks and flashes.
pub fn break_fx(w: &mut World, id: MobyId, tnt: bool, _hit: Option<HitRecord>) {
    let tick = w.counter as i32;
    if (tick - w.svc.break_sound_tick).abs() >= 2 {
        if w.m(id).o_class == 0x1fa { w.play_sound_as(0, 0, id, 500); } else { w.play_sound(0, 0, id); }
        w.svc.break_sound_tick = tick;
    }
    let suppress = if (tick - w.svc.break_fx_tick).abs() < 2 { true } else { w.svc.break_fx_tick = tick; false };
    // Unlink from the stack.
    let (above, below) = { let m = w.m(id); (ptr(&m.pvars, 0xa0), ptr(&m.pvars, 0xa4)) };
    if let Some(a) = above {
        let m = w.mm(a);
        set_ptr(&mut m.pvars, 0xa4, below);
        let f = p::u32(&m.pvars, 0xac) | 4;
        p::set_u32(&mut m.pvars, 0xac, f);
    }
    if let Some(b) = below {
        let m = w.mm(b);
        set_ptr(&mut m.pvars, 0xa0, above);
        let f = p::u32(&m.pvars, 0xac) | 4;
        p::set_u32(&mut m.pvars, 0xac, f);
    }
    // Dust.
    let pos = pv(w.m(id).position);
    let mut d0 = pos;
    d0[2] = pos[2] + Pf::b(0x3e99_999a);
    let (j, lo, g, size) = (Pf::b(0x3e99_999a), Pf::b(0x3f81_47ae), Pf::b(0x3d4c_cccd), Pf::b(0x4743_5000));
    w.part13(j, lo, Pf::b(0x3f89_999a), g, size, d0, 1, 0x4080_8080);
    for _ in 0..5 { w.part13(j, lo, Pf::b(0x3f88_f5c3), g, size, d0, 1, 0x4080_8080); }
    // Debris.
    let class = w.m(id).o_class;
    let dclass: i16 = match class {
        500 | 511 => 0x165,
        501 => 0x15c,
        502 => 0x162,
        503 => 0x15f,
        504 => 0x168,
        505 => 0x16b,
        506 => 0x16e,
        507..=510 => 0x171,
        _ => 0x95,
    };
    let free = w.table.free_slots;
    let (n, nc) = if free > 200 { (4, 4) } else if free > 150 { (2, 2) } else if free > 100 { (2, 1) } else { (1, 0) };
    let cam = w.camera;
    let a = atan(cam[0] - pos[0], cam[1] - pos[1]);
    let r = ph::fast_subtract_rotations(a, sv::pf(w.m(id).rotation[2]));
    let t = r + Pf::b(0x4049_0fda);
    let q = Pf::b(0x3fc8_f5c3);
    let i = (t / q).to_i32();
    let th = normalize_angle(Pf::from_i32(i) * q);
    let rows = sv::euler_rows([Pf::ZERO, Pf::ZERO, th, Pf::ZERO]);
    let rows3 = [rows[0], rows[1], rows[2]];
    let mut rot80: V4 = [Pf::ZERO; 4];
    for k in 0..n {
        let off = DEBRIS_OFF[k].map(Pf);
        let pp = ph::vadd(ph::mul_rows3(&rows3, off), pos);
        rot80 = DEBRIS_ROT[k].map(Pf);
        rot80[2] = ph::fast_add_rotations(rot80[2], th);
        let mut v = ph::vsub(pp, pos);
        v[2] = Pf::ZERO;
        let s = Pf(w.rng.randf_bits(0x3f80_0000, 0x40a0_0000));
        v = ph::set_len3(v, s * DT);
        v[2] = Pf(w.rng.randf_bits(0x4080_0000, 0x4110_0000)) * DT;
        let k2 = w.rng.randi(2);
        if let Some(m) = debris::spawn(w, 1.0, 1.0, id, fv(v), fv(pp), fv(rot80), dclass, k2) {
            let mo = w.mm(m);
            mo.rotation = DEBRIS_ROT[k].map(f32::from_bits);
            mo.rotation[2] = sv::fl(ph::fast_add_rotations(sv::pf(mo.rotation[2]), th));
            w.build_matrix(m);
        }
    }
    for _ in 0..nc {
        let mut pp: V4 = [
            Pf(w.rng.randf_bits(0xbf00_0000, 0x3f00_0000)),
            Pf(w.rng.randf_bits(0xbf00_0000, 0x3f00_0000)),
            Pf(w.rng.randf_bits(0, 0x3f80_0000)),
            Pf::ZERO,
        ];
        pp = ph::vadd(pp, pos);
        let l = Pf(w.rng.randf_bits(0x40c0_0000, 0x4140_0000)) * DT;
        let vv = w.rng.rand_vec(sv::fl(l), sv::fl(l));
        let mut v: V4 = [sv::pf(vv[0]), sv::pf(vv[1]), sv::pf(vv[2]), Pf::ZERO];
        if v[2] < Pf::ZERO { v[2] = -v[2]; }
        rot80[0] = Pf(w.rng.rand_angle_bits());
        rot80[2] = Pf(w.rng.rand_angle_bits());
        let mut t1 = 1;
        if w.rng.randi(6) == 0 { t1 = Pf(w.rng.randf_bits(0x4040_0000, 0x40c0_0000)).to_i32(); }
        debris::spawn(w, 0.25, 0.5, id, fv(v), fv(pp), fv(rot80), dclass + 1, t1);
    }
    // Hide.
    let t15 = w.ticks(0xf);
    {
        let m = w.mm(id);
        m.has_collision = false;
        m.state = 3;
        m.mode |= mode::HIDDEN;
        m.cmd = t15 as u8;
    }
    if !tnt { return; }
    let k = if w.hero.group == 0x16 { w.ticks(5) as u8 } else { 1 };
    w.mm(id).pvars[0xca] = k;
    let (pos, up) = { let m = w.m(id); (pv(m.position), pv(m.rows[2])) };
    let ep = ph::vadd(ph::set_len3(up, Pf::b(0x3f00_0000)), pos);
    let z0: V4 = [Pf::ZERO; 4];
    let dist = ph::len3(ph::vsub(w.camera, pos));
    let mut nsp = 10;
    if dist < Pf::b(0x4100_0000) { nsp = dist.to_i32() + 2; }
    let mut sub = Pf::ZERO;
    if dist < Pf::b(0x40e0_0000) { sub = Pf::b(0x40e0_0000) - dist; }
    if suppress && nsp >= 7 { nsp = 6; }
    for _ in 0..nsp.max(0) {
        let sp = Pf(w.rng.randf_bits(0x4100_0000, 0x4120_0000)) * DT - sub * DT;
        let c1 = SPARK_C1[w.rng.randi(6) as usize];
        let c2 = SPARK_C2[w.rng.randi(6) as usize];
        let (a, b) = (w.ticks(0xf), w.ticks(0x14));
        let life = w.rng.rand_range(a, b);
        let (a, b) = (w.ticks(0x19), w.ticks(0x1e));
        let t1 = w.rng.rand_range(a, b);
        w.part11(Pf::b(0x48c3_5000), sp, ep, z0, c1, c2, life, t1, 0, 0);
    }
    // The flashes (`FlashSpawn`, the shared effect spawner in `debris`).
    let (ep, z0) = (fv(ep), [0.0f32; 4]);
    if w.svc.frame_load[0] < Pf::b(0x3f73_3333) && Pf::b(0x4110_0000) < dist && !suppress {
        let four = 4.0;
        let t = w.ticks(0xf);
        debris::flash_spawn(w, four, id, ep, z0, t, 0x7f, 0x7f, 0x7f, 0x20);
        let t = w.ticks(0x18);
        debris::flash_spawn(w, four, id, ep, z0, t, 0x7f, 0x20, 0, 0x20);
        let t = w.ticks(0x14);
        debris::flash_spawn(w, four, id, ep, z0, t, 0x7f, 0x40, 0, 0x30);
    }
    let t = w.ticks(0x1b);
    debris::flash_spawn(w, 3.5, id, ep, z0, t, 0x60, 0x10, 0, 0x40);
    let t = w.ticks(0x1d);
    debris::flash_spawn(w, 3.0, id, ep, z0, t, 0x20, 0, 0, 0x20);
}

/// Debris offsets / rotations (level01 .data 0x1deb90 / 0x1debf0, entries 0..3).
const DEBRIS_OFF: [[u32; 4]; 4] = [
    [0xbe80_8312, 0xbe28_f5c3, 0x3f56_0419, 0],
    [0xbeab_851f, 0xbe82_0c4a, 0x3f2b_851f, 0],
    [0xbe2f_1aa0, 0xbead_0e56, 0x3f3f_7cee, 0],
    [0x3e80_0000, 0x3e26_e979, 0x3f56_0419, 0],
];
const DEBRIS_ROT: [[u32; 4]; 4] = [
    [0x4049_0625, 0x4049_0625, 0xbfc8_f5c3, 0],
    [0x3fc8_f5c3, 0x3fc8_f5c3, 0xbfc8_f5c3, 0],
    [0x3fc8_f5c3, 0, 0, 0],
    [0x4049_0625, 0x4049_0625, 0x3fc8_f5c3, 0],
];
/// TNT spark colours (0x20b0b0 / 0x20b0c8).
const SPARK_C1: [u32; 6] = crate::moby_update::creature::fx::SPARK_A;
const SPARK_C2: [u32; 6] = crate::moby_update::creature::fx::SPARK_B;

/// `CrateDropBolts` 0x2eb498: bolts (`SetDeathBits(m, 0x100, pvar+0xc0)` → `BoltBurst`) unless pvar+0xc6 ≠ 0
/// (the ammo crate 511: ammo pickups of pvar+0xcb) or class 501 (the nanotech crate: +0xb4 cleared, death bits
/// only).
pub fn drop_bolts(w: &mut World, id: MobyId) {
    let (c6, class) = { let m = w.m(id); (p::i16(&m.pvars, 0xc6), m.o_class) };
    let bolt_path = c6 == 0 && class != 0x1f5;
    if bolt_path {
        if p::i32(&w.m(id).pvars, 0xf0) != 0 { w.mm(id).occlusion = 0x7f80; }
        let pos = pv(w.m(id).position);
        if w.svc.level == 0xf
            && (pos[0] - Pf::b(0x4326_0000)).abs() < Pf::b(0x40c0_0000)
            && (pos[1] - Pf::b(0x4341_0000)).abs() < Pf::b(0x40c0_0000)
        {
            let save = w.svc.counters.challenge_on;
            w.svc.counters.challenge_on = 0;
            set_death_bits(w, id, 0x100, -1);
            w.svc.counters.challenge_on = save;
            return;
        }
        let path = p::i32(&w.m(id).pvars, 0xc0);
        set_death_bits(w, id, 0x100, path);
        return;
    }
    if class == 0x1f5 {
        if w.m(id).b4 != 0 { w.mm(id).b4 = 0; }
        set_death_bits(w, id, 0x100, -1);
        return;
    }
    let mut ty = match w.m(id).pvars[0xcb] {
        1 => 10,
        2 => 0x10,
        3 => 0xf,
        4 => 0x14,
        5 => 0x11,
        6 => 0xb,
        7 => 0x18,
        8 => 0xd,
        9 => 0x19,
        10 => 0x13,
        11 => 0x17,
        _ => -1,
    };
    let count = pickup::ammo_pick(w, Some(id), &mut ty);
    let pos = pv(w.m(id).position);
    for _ in 0..count {
        let Some(mm) = pickup::pickup_spawn(w, pos, ty, -1, 0) else { break };
        let c4 = p::i16(&w.m(id).pvars, 0xc4);
        p::set_i16(&mut w.mm(mm).pvars, 4, c4);
        let ang = Pf(w.rng.rand_angle_bits());
        let r = Pf(w.rng.randf_bits(0, 0x4020_0000));
        let s = r * DT;
        let vx = ph::fast_cos(ang) * s;
        let vy = ph::fast_sin(ang) * s;
        let vz = Pf(w.rng.randf_bits(0x4080_0000, 0x40c0_0000)) * DT;
        let mut vel = [vx, vy, vz, Pf::ZERO];
        let path = p::i32(&w.m(id).pvars, 0xc0);
        let mpos = pv(w.m(mm).position);
        if path >= 0 { w.land_correct(path, &mut vel, pos, mpos, mpos[2]); }
        let m = w.mm(mm);
        p::set_f(&mut m.pvars, 0x10, vel[0]);
        p::set_f(&mut m.pvars, 0x14, vel[1]);
        p::set_f(&mut m.pvars, 0x18, vel[2]);
    }
    if w.m(id).b4 != 0 { w.mm(id).b4 = 0; }
    set_death_bits(w, id, 0x100, -1);
}

/// `SetDeathBits(m, fl, path)` 0x26c250: the save bits of the dropper, then `BoltBurst` with `n ± spread`
/// bolts, `n` = the dropper's +0xb4 (or what is left of +0xb6 for spawner-counted droppers).
pub fn set_death_bits(w: &mut World, id: MobyId, fl: u32, path: i32) {
    let (b4, b1, b2, b0, b6) = { let m = w.m(id); (m.b4, m.spawn_flag, m.spawn_id, m.mission, m.b6) };
    let mut f = Pf::from_i32(b4 as i32);
    if b1 == 0xfe || b2 < 0 { return; }
    let lvl = w.svc.level;
    w.svc.save.death.insert((lvl, b2));
    w.svc.save.death_level.insert(b2);
    w.svc.save.killed.insert(b2, b0.wrapping_add(2));
    if b0 == 0xff || (w.missions.mission_slot(b0) != 0xff && w.mission_done(lvl, b0) == 0xff) {
        w.svc.save.collected.insert(b2, b0.wrapping_add(2));
    }
    if (b1 as i8) >= 0 {
        let got = w.svc.counters.spawner_bolts.get(&(lvl, b1)).copied().unwrap_or(0);
        let b6i = b6 as i32;
        let mut rem = b6i - got as i32;
        if !((b6i + 1) / 2 < rem) { rem = 0; }
        if rem != 0 { f = Pf::from_i32(rem); }
        if f < Pf::ONE { f = Pf::ONE; }
    }
    let mut flags: u32 = if fl & 0x100 != 0 { 5 } else { 1 };
    if w.hero.board.f141402 != 0 || fl & 0x200 != 0 { flags |= 2; }
    if w.hero.state == 0x10 && ph::dist3(w.hero.pos, pv(w.m(id).position)) < Pf::b(0x4080_0000) { flags |= 2; }
    let free = w.table.free_slots;
    if fl & 0x400 != 0 { flags |= 0x10; }
    if free < 100 || fl & 0x800 != 0 { flags |= 8; }
    if free <= 0x31 { flags |= 2; }
    if f < Pf::ZERO { f = Pf::ZERO; }
    if !(Pf::ZERO < f) { return; }
    let spread = if Pf::b(0x4100_0000) <= f {
        (f * Pf::b(0x3e80_0000)).to_i32()
    } else if Pf::b(0x4000_0000) <= f {
        1
    } else {
        0
    };
    let n = f.to_i32();
    bolt_burst(w, id, n - spread, n + spread, flags, path);
}

/// `BoltBurst(m, lo, hi, flags, path)` 0x275988: `N = randi(hi − lo + 1) + lo` bolts (doubled in the double
/// modes, scaled by the challenge limiter), split into 50/20/5/1 coins (at most 3 coins with flags 8), each
/// spawned from 0.5 above the dropper with a random horizontal speed (`randf(0, 3)·dt`) and `randf(3.7, 6)·dt`
/// up. Draws: 1 + 4 per coin + `BoltSpawn`'s.
pub fn bolt_burst(w: &mut World, id: MobyId, lo: i32, hi: i32, flags: u32, path: i32) {
    let mut base: V4 = [Pf::ZERO; 4];
    let (b0, class) = { let m = w.m(id); (m.mission, m.o_class) };
    if flags & 0x20 == 0 && b0 != 0xff && ((class as i32 - 500) as u32 & 0xffff) >= 0x29 {
        w.svc.counters.challenge_on = (w.missions.mission_slot(b0) == 0xff) as i32;
    }
    let v = w.svc.counters.challenge_on;
    let dc: u8 = if v != 0 { (flags & 0x20 == 0) as u8 } else { 0 };
    if flags == 0 || (lo == 0 && hi == 0) { return; }
    if flags & 4 != 0 {
        // `FUN_002711f8(dropper)` (the mode-0x20 pvar record, pvar +0x00): its +0x20 vector is the base velocity
        // every coin starts with (a dropper without the header: zero).
        let m = w.m(id);
        if let Some(o) = crate::moby_update::triggers::pvar_record(m).filter(|&o| o + 0x30 <= m.pvars.len()) {
            base = p::v4(&m.pvars, o + 0x20);
        }
    }
    let lo = if lo > 0 { lo } else { 1 };
    let mut n = w.rng.randi(hi - lo + 1) + lo;
    let dbl = w.svc.counters.double_a != 0 || w.svc.counters.times_completed != 0;
    if dbl { n <<= 1; }
    if n < 500 && dc != 0 && w.svc.counters.challenge_limit != 0 {
        let t60 = w.ticks(0x3c);
        let x = (Pf::from_i32(t60) * Pf::b(0x4270_0000)).to_i32();
        let mut k = x.wrapping_mul(w.svc.counters.challenge_bolts) / w.svc.counters.challenge_limit;
        if dbl { k /= 2; }
        if k < 0x96 {
            n = if k < 0x5b { n * 5 } else { n * ((0xbe - k) / 0x14) };
            n = if w.svc.counters.double_a != 0 { n.min(1000) } else { n.min(500) };
        } else if k >= 0x12d {
            n = if k < 0x160 { n * ((0x168 - k) / 0x50) } else { n / 10 };
            if n <= 0 { n = 1; }
        }
    }
    // Split into coins.
    let (mut n50, mut n20, mut n5) = (0, 0, 0);
    let mut v = n;
    if v >= 300 { loop { v -= 50; n50 += 1; if v < 300 { break; } } }
    while v >= 200 { v -= 20; n20 += 1; }
    if v >= 30 { loop { v -= 25; n20 += 1; n5 += 1; if v < 30 { break; } } }
    if v >= 7 { loop { v -= 5; n5 += 1; if v < 7 { break; } } }
    let mut n1 = v;
    if n50 + n20 + n5 + n1 >= 4 && flags & 8 != 0 {
        // A: three 50s broken down while above N; B: three 1s built up while below N.
        let (mut a50, mut a20, mut a5, mut a1) = (3, 0, 0, 0);
        let (mut v50, mut v20, mut v5) = (150, 0, 0);
        let mut va = 150;
        if n < 150 {
            loop {
                if a50 != 0 { v50 -= 50; a50 -= 1; v20 += 20; a20 += 1; }
                else if a20 != 0 { v20 -= 20; a20 -= 1; v5 += 5; a5 += 1; }
                else if a5 != 0 { v5 -= 5; a5 -= 1; a1 += 1; }
                va = v50 + v20 + v5 + a1;
                if !(n < va) || va < 4 { break; }
            }
        }
        let (mut b1, mut b5, mut b20, mut b50) = (3, 0, 0, 0);
        let mut vb = 3;
        if 3 < n {
            let (mut w5, mut w20, mut w50) = (0, 0, 0);
            loop {
                if b1 != 0 { b1 -= 1; w5 += 5; b5 += 1; }
                else if b5 != 0 { w5 -= 5; b5 -= 1; w20 += 20; b20 += 1; }
                else if b20 != 0 { w20 -= 20; b20 -= 1; w50 += 50; b50 += 1; }
                vb = w50 + w20 + w5 + b1;
                if !(vb < n) || !(vb < 150) { break; }
            }
        }
        if (va - n) < (n - vb) {
            (n1, n5, n20, n50) = (a1, a5, a20, a50);
        } else {
            (n1, n5, n20, n50) = (b1, b5, b20, b50);
        }
    }
    let cpos = pv(w.m(id).position);
    while n1 != 0 || n5 != 0 || n20 != 0 || n50 != 0 {
        let mut pos = cpos;
        pos[2] = pos[2] + Pf::b(0x3f00_0000);
        let ang = Pf(w.rng.rand_angle_bits());
        let c = ph::fast_cos(ang);
        let r = Pf(w.rng.randf_bits(0, 0x4040_0000));
        let vx = c * (r * DT);
        let s = ph::fast_sin(ang);
        let r = Pf(w.rng.randf_bits(0, 0x4040_0000));
        let vy = s * (r * DT);
        let vz = Pf(w.rng.randf_bits(0x406c_cccd, 0x40c0_0000)) * DT;
        let off = ph::set_len3([vx, vy, Pf::ZERO, Pf::ZERO], Pf::b(0x3f00_0000));
        pos = ph::vadd(pos, off);
        let mut vel = [vx, vy, vz, Pf::ZERO];
        if path >= 0 { w.land_correct(path, &mut vel, cpos, pos, pos[2]); }
        vel = ph::vadd(vel, base);
        let value = if n50 != 0 { n50 -= 1; 0x32 } else if n20 != 0 { n20 -= 1; 0x14 } else if n5 != 0 { n5 -= 1; 5 } else { n1 -= 1; 1 };
        bolt::spawn(w, id, pos, vel, flags, value, dc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::services::Services;
    use crate::moby_update::triggers;

    /// Carrier 0 at the origin (block +0x20) and two crates 1 (bottom, at (3, 0, 1)) and 2 (on it).
    fn scene() -> MobyTable {
        let mut k = Moby { o_class: 707, pvars: vec![0; 0x80], ..Moby::default() };
        k.mode |= 0x20;
        p::set_i32(&mut k.pvars, 8, 0x20);
        for i in 0..3 { k.rows[i][i] = 1.0; }
        let c1 = Moby { o_class: 500, state: 1, pvars: vec![0; 0x100], position: [3.0, 0.0, 1.0, 1.0], ..Moby::default() };
        let c2 = Moby { o_class: 500, state: 1, pvars: vec![0; 0x100], position: [3.0, 0.0, 2.0, 1.0], ..Moby::default() };
        MobyTable::new(vec![k, c1, c2], 4)
    }

    fn with_world(t: &mut MobyTable, f: impl FnOnce(&mut World)) {
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = Services::new();
        let mut w = World::new(t, &hero, &mut rng, &classes, &mut svc, 3);
        f(&mut w);
    }

    #[test]
    fn crate_attaches_to_a_carrier_and_the_stack_inherits() {
        let mut t = scene();
        t.mobys[1].occlusion = 0;
        with_world(&mut t, |w| {
            assert!(attach_platform(w, 1, 0));
            assert!(!attach_platform(w, 1, 2), "a crate is no carrier");
            inherit_platform(w, 2, 1);
        });
        let (c1, c2) = (&t.mobys[1], &t.mobys[2]);
        assert_eq!((c1.occlusion, p::i32(&c1.pvars, 0xf0), p::ff(&c1.pvars, 0xf4)), (0x7f80, 1, 0.0));
        assert_eq!(p::v4f(&c1.pvars, 0xd0)[..3], [3.0, 0.0, 1.0]);
        assert_eq!((p::i32(&c2.pvars, 0xf0), p::ff(&c2.pvars, 0xf4), c2.update_dist, p::u32(&c2.pvars, 0xac) & 8), (1, 1.0, 0xff, 8));
        assert_eq!(p::v4f(&c2.pvars, 0xd0), p::v4f(&c1.pvars, 0xd0));
    }

    /// The carrier turns a quarter and moves by (1, 0, 0): the crate goes round with it, publishes its own move to
    /// its block +0x60, and a raised crate settles toward its rest height.
    #[test]
    fn crate_rides_a_turning_carrier() {
        let mut t = scene();
        with_world(&mut t, |w| { attach_platform(w, 1, 0); });
        let q = std::f32::consts::FRAC_PI_2;
        let r = triggers::euler_matrix([0.0, 0.0, q]);
        {
            let k = &mut t.mobys[0];
            k.position[0] = 1.0;
            k.rotation[2] = q;
            for (row, src) in k.rows.iter_mut().zip(&r) { for (x, y) in row.iter_mut().zip(src) { *x = *y as f32; } }
        }
        p::set_ff(&mut t.mobys[1].pvars, 0xf4, 0.5);
        with_world(&mut t, |w| ride_platform(w, 1));
        let c = &t.mobys[1];
        let dt2 = sv::fl(DT2);
        let h = 0.5 - 10.0 * dt2;
        assert_eq!(p::ff(&c.pvars, 0x4c), -10.0 * dt2);
        assert_eq!(p::ff(&c.pvars, 0xf4), h);
        assert!((c.position[0] - 1.0).abs() < 1e-5 && (c.position[1] - 3.0).abs() < 1e-5 && (c.position[2] - (1.0 + h)).abs() < 1e-5, "{:?}", c.position);
        assert!((c.rotation[2] - q).abs() < 1e-5);
        let v = p::v4f(&c.pvars, 0x40);
        assert!((v[0] + 2.0).abs() < 1e-5 && (v[1] - 3.0).abs() < 1e-5 && v[3] == -10.0 * dt2, "{v:?}");
        let blk = &c.pvars[0x60..];
        let rz = f32::from_le_bytes(blk[8..12].try_into().unwrap());
        let dx = f32::from_le_bytes(blk[0x10..0x14].try_into().unwrap());
        assert!((rz - q).abs() < 1e-5 && (dx - v[0]).abs() < 1e-6);
        assert_eq!(c.occlusion, 0x7f80);
    }
}
