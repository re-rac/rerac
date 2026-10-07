//! Bolts, classes 13 / 14 / 15 / 16 (value 1 / 5 / 20 / 50): `BoltUpdate` level01 0x2bb758, with the
//! idle pose `FUN_002bc768`, the fly-to-hero start `FUN_002bcb90` and the pickup `CollectBolt` 0x2bc4f0.
//! Spec: `docs/plan/moby_update_catalogue.md` §4 "Bolts" and the "Bolts" notes of "In the port".
//!
//! Pvar block (0x80 bytes; placed bolts have one in the gameplay file, spawned bolts get `CreateMoby`'s
//! zeroed block):
//!
//! | off | type | meaning |
//! |---|---|---|
//! | 0x00 | vec4 | rest position (ground hit of the init probe, or the spawn position) |
//! | 0x10 | vec4 | rest Euler angles (x 0, y = tilt of the ground normal, z = its yaw) |
//! | 0x20 | vec4 | velocity (u/tick) in states 1 and 4 |
//! | 0x30 | vec4 | per-tick spin quaternion (states 1, 2, 4) |
//! | 0x40 | u8 | "challenge" bolt: the pickup also adds to 0x15ee2c |
//! | 0x48 / 0x4c | f32 | fly-to-hero path bend angles (hero-local y / z) |
//! | 0x50 / 0x58 | f32 | idle spin angles (x += k, z −= 0.0125 per tick; direction by bit 0 of 0x55) |
//! | 0x54 | u8 | set: the init does not snap to the ground |
//! | 0x55 | u8 | bit 0: spin direction (`randi(2) == 0` at init) |
//! | 0x56 / 0x57 | u8 | hop length / hop ticks left |
//! | 0x5c | ptr | the carrier the bolt rests on (moby index + 1; set on a moby hit when `FUN_00275290(moby)`: mode 0x20 and pvar +8 ≠ 0; +0x00 / +0x10 then hold the rest pose in its frame, [`attach`]) |
//! | 0x60 | ptr | fly-to target (the hero moby) |
//! | 0x64 | f32 | fly speed (approaches 48·dt by 16·dt² per tick); the pickup happens within it |
//! | 0x68 | f32 | z at init (a falling bolt 2 below it and below the hero flies to the hero) |
//! | 0x6c | s16 | glint slot (−1 = none) |
//! | 0x6e | s16 | ticks to the next glint |
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern, clippy::needless_range_loop)] // FPU compare semantics and op order are spelled out on purpose.

use crate::collision_query::CollOutput;
#[allow(unused_imports)]
use crate::moby_update::services::HitTemplate;
use crate::hero::physics::{self as ph, V4};
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::services::{self as sv, pvar as p, pv, fv, World, DEG, DT, DT2, QUARTER_PI};
use crate::moby_update::triggers;
use crate::pad::fast_arctan as atan;
use crate::ps2v::Pf;

/// The bolt update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x2bb758;

/// Classes that run [`update`].
pub const CLASSES: [i16; 4] = [13, 14, 15, 16];

/// Bolt value by class (`CollectBolt` 0x2bc4f0): 13 → 1, 14 → 5, 15 → 20, 16 → 50.
pub fn value(o_class: i16) -> i32 {
    match o_class {
        14 => 5,
        15 => 0x14,
        16 => 0x32,
        _ => 1,
    }
}

/// Collision radius of a falling bolt (state 1) and of the idle pose's detach test: 13 → 0.15,
/// 14 / 15 → 0.23, 16 → 0.25.
fn radius(o_class: i16) -> Pf {
    match o_class {
        13 => Pf::b(0x3e19_999a),
        14 | 15 => Pf::b(0x3e6b_851f),
        _ => Pf::b(0x3e80_0000),
    }
}

/// `FUN_002bc768`'s per-class constants: (spin step, tilt, up offset, row-0 offset, radius).
fn pose_consts(o_class: i16) -> (Pf, Pf, Pf, Pf, Pf) {
    match o_class {
        13 => (Pf::b(0x3d23_d70a), Pf::b(0xbe80_0000), Pf::b(0x3d47_ae14), Pf::b(0x3ea6_6666), Pf::b(0x3e19_999a)),
        14 => (Pf::b(0x3cf5_c28f), Pf::b(0xbf00_0000), Pf::b(0x3cc8_b439), Pf::b(0x3eb3_3333), Pf::b(0x3e6b_851f)),
        15 => (Pf::b(0x3ccc_cccd), Pf::b(0xbecc_cccd), Pf::b(0x3d66_6666), Pf::b(0x3ec0_0000), Pf::b(0x3e6b_851f)),
        _ => (Pf::b(0x3c8f_5c29), Pf::b(0xbeb3_3333), Pf::b(0x3e0a_3d71), Pf::b(0x3e99_999a), Pf::b(0x3e80_0000)),
    }
}

/// The glint offset of each bolt class, `0x1d7850 + o_class·0x10` (model space, rotated by the rows).
pub fn glint_offset(o_class: i16) -> V4 {
    let o = GLINT_OFFSETS[(o_class.clamp(13, 16) - 13) as usize];
    o.map(Pf)
}

/// `0x1d7850 + class·0x10` for classes 13..16 (level01 .data): (0.14, 0, −0.15), (0.07, 0.09, 0.17),
/// (0.07, 0.10, 0.18), (0.07, 0.11, 0.22).
pub const GLINT_OFFSETS: [[u32; 4]; 4] = [
    [0x3e0f_5c29, 0, 0xbe19_999a, 0],
    [0x3d8f_5c29, 0x3db8_51ec, 0x3e2e_147b, 0],
    [0x3d8f_5c29, 0x3dcc_cccd, 0x3e38_51ec, 0],
    [0x3d8f_5c29, 0x3de1_47ae, 0x3e61_47ae, 0],
];

/// Class scale factor of the init (`moby+0x2c = class+0x24 · k`).
fn scale_factor(o_class: i16) -> Option<Pf> {
    match o_class {
        13 => Some(Pf::b(0x3f26_6666)),
        14 => Some(Pf::b(0x3f33_3333)),
        15 => Some(Pf::b(0x3f40_0000)),
        16 => Some(Pf::b(0x3f19_999a)),
        _ => None,
    }
}

fn rows3(m: &crate::moby_runtime::Moby) -> [V4; 3] { [pv(m.rows[0]), pv(m.rows[1]), pv(m.rows[2])] }
fn set_rows3(m: &mut crate::moby_runtime::Moby, r: [V4; 3]) { for k in 0..3 { m.rows[k] = fv(r[k]); } }
fn rows4(m: &crate::moby_runtime::Moby) -> [V4; 4] { [pv(m.rows[0]), pv(m.rows[1]), pv(m.rows[2]), pv(m.rows[3])] }

/// The collision output fields the bolt copies (`0x1742e0` point quadword, `0x174300` raw normal).
fn hit_point(o: &CollOutput) -> V4 { [sv::pf(o.point[0]), sv::pf(o.point[1]), sv::pf(o.point[2]), Pf::ZERO] }
fn hit_normal(o: &CollOutput) -> V4 { [sv::pf(o.normal[0]), sv::pf(o.normal[1]), sv::pf(o.normal[2]), Pf::ZERO] }

/// Rest Euler of a ground hit: `y = atan(n.z, |n.xy|)`, `z = atan(n.x, n.y)` (x = 0).
fn rest_euler(pv_: &mut [u8], n: V4) {
    p::set_v4(pv_, 0x10, [Pf::ZERO; 4]);
    p::set_f(pv_, 0x18, atan(n[0], n[1]));
    let l = ph::len2(n);
    p::set_f(pv_, 0x14, atan(n[2], l));
}

/// `BoltUpdate` (0x2bb758).
pub fn update(w: &mut World, id: MobyId) {
    // The layout is 0x80 bytes (a shorter block would make the game read the next pvar block: zeros here).
    if w.m(id).pvars.len() < 0x80 { w.mm(id).pvars.resize(0x80, 0); }
    // Hidden during cutscenes (0x15f5c4 == 2); otherwise "drawn" (+0x31 = 1) and shown.
    {
        let cut = w.svc.game_mode == 2;
        let m = w.mm(id);
        if cut { m.visible = 0; m.mode |= mode::HIDDEN; } else { m.visible = 1; m.mode &= !mode::HIDDEN; }
    }
    match w.m(id).state {
        0 => init(w, id),
        1 => fall(w, id),
        2 => settle(w, id),
        3 => idle(w, id),
        4 => fly(w, id),
        _ => {}
    }
}

/// State 0: the init (placed bolts; spawned bolts are put in state 1 by their spawner).
fn init(w: &mut World, id: MobyId) {
    {
        let m = w.mm(id);
        m.state = 3;
        m.b73 = 0x14;
        m.mode |= mode::KEEP_ROWS;
        m.update_dist = 0x20;
        m.draw_dist = 0x20;
    }
    if w.rng.randi(2) == 0 {
        let pvars = &mut w.mm(id).pvars;
        pvars[0x55] |= 1;
    }
    let (pos, up) = { let m = w.m(id); (pv(m.position), pv(m.rows[2])) };
    let a = ph::vadd(ph::set_len3(up, Pf::b(0x3e4c_cccd)), pos);
    let b = ph::vadd(ph::set_len3(up, Pf::b(0xc040_0000)), pos);
    let hit = w.coll_line(a, b, 0x22, None); // CollLine_Fix(a, b, 0x22, 0, 0): world + mobys
    let no_snap = w.m(id).pvars[0x54] != 0;
    let z = w.m(id).position[2];
    {
        let m = w.mm(id);
        match hit {
            Some(h) if !no_snap => {
                p::set_v4(&mut m.pvars, 0, hit_point(&h));
                rest_euler(&mut m.pvars, hit_normal(&h));
            }
            _ => {
                p::set_v4(&mut m.pvars, 0, pos);
                p::set_v4(&mut m.pvars, 0x10, [Pf::ZERO; 4]);
            }
        }
    }
    if let Some(h) = hit.as_ref().filter(|_| !no_snap) { attach(w, id, h); }
    let a1 = Pf(w.rng.rand_angle_bits());
    let a2 = Pf(w.rng.rand_angle_bits());
    let t600 = w.ticks(600);
    let wait = w.rng.rand_range(0, t600);
    let o_class = w.m(id).o_class;
    let cs = w.class_scale(o_class);
    let m = w.mm(id);
    p::set_f(&mut m.pvars, 0x50, a1);
    p::set_f(&mut m.pvars, 0x58, a2);
    p::set_f(&mut m.pvars, 0x68, sv::pf(z));
    p::set_i16(&mut m.pvars, 0x6e, wait as i16);
    p::set_i16(&mut m.pvars, 0x6c, -1);
    if let Some(k) = scale_factor(o_class) { m.scale = sv::fl(cs * k); }
}

/// State 1: falling (dropped from a crate): gravity `10.8·dt²`, spin, sphere collision → bounce or settle.
fn fall(w: &mut World, id: MobyId) {
    let o_class = w.m(id).o_class;
    {
        let m = w.mm(id);
        let vz = p::f(&m.pvars, 0x28) - DT2 * Pf::b(0x412c_cccd);
        p::set_f(&mut m.pvars, 0x28, vz);
        let pos = ph::vadd(pv(m.position), p::v4(&m.pvars, 0x20));
        m.position = fv(pos);
        let q = sv::quat_rows(p::v4(&m.pvars, 0x30));
        let r = sv::mat3_mul(&q, &rows3(m));
        set_rows3(m, r);
    }
    let r = radius(o_class);
    let (pos, z0) = { let m = w.m(id); (pv(m.position), p::f(&m.pvars, 0x68)) };
    let two = Pf::b(0x4000_0000);
    if pos[2] < z0 - two && pos[2] < w.hero.pos[2] - two {
        // Fell through: fly to the hero with a random bend.
        let a = w.rng.randf_bits(0xc170_0000, 0x4170_0000);
        start_fly(w, id, Pf(a) * DEG, Pf::ZERO);
        return;
    }
    let Some(h) = w.coll_sphere(pos, r, 0x22, Some(id)) else { return }; // 0x212960(r, pos, 0x22, bolt)
    let point = hit_point(&h);
    let n = hit_normal(&h);
    let (rx, rz) = w.bolt_radii();
    let sep;
    {
        let m = w.mm(id);
        p::set_v4(&mut m.pvars, 0, point);
        rest_euler(&mut m.pvars, n);
        sep = ph::vsub(pos, point);
        let vel = sv::reflect(p::v4(&m.pvars, 0x20), sep);
        p::set_v4(&mut m.pvars, 0x20, vel);
    }
    let d = ph::dist2(pos, w.hero.pos);
    if d < rx && (pos[2] - w.hero.pos[2]).abs() < rz {
        if w.hero.health == 0 { return; }
        let s = Pf(w.rng.randf_bits(0x4040_0000, 0x40e0_0000));
        let m = w.mm(id);
        let vel = ph::set_len3(p::v4(&m.pvars, 0x20), s * DT);
        p::set_v4(&mut m.pvars, 0x20, vel);
        start_fly(w, id, Pf::ZERO, Pf::ZERO);
        return;
    }
    // Settle only on a floor below the bolt (both slopes under 45°).
    if !(atan(sep[2], ph::len2(sep)) < QUARTER_PI) { return; }
    if !(atan(n[2], ph::len2(n)) < QUARTER_PI) { return; }
    w.mm(id).state = 2;
    let k = hop_ticks(w, 0x41f0_0000, 0x4220_0000);
    let m = w.mm(id);
    m.pvars[0x56] = k;
    m.pvars[0x57] = k;
    attach(w, id, &h);
    let m = w.mm(id);
    let e = sv::rows_euler(&rows4(m));
    p::set_v4(&mut m.pvars, 0x30, sv::euler_quat(e));
}

/// `truncate_float_to_s32(multiply_global_scale(randf(a, b)))`, stored as a byte.
fn hop_ticks(w: &mut World, a: u32, b: u32) -> u8 {
    let r = Pf(w.rng.randf_bits(a, b));
    w.svc.timing.scale(r).to_i32() as u8
}

/// State 2: settle into the rest pose over `pvar+0x57` ticks, with a hop.
fn settle(w: &mut World, id: MobyId) {
    let base = pose(w, id, true);
    let m = w.mm(id);
    let mut target = base;
    target[2] = target[2] + Pf::b(0x3e80_0000);
    let e = sv::rows_euler(&rows4(m));
    let q = sv::euler_quat(e);
    let n57 = Pf::from_i32(m.pvars[0x57] as i32);
    let t = Pf::ONE / n57;
    let f = Pf::from_i32(m.pvars[0x57] as i32) / Pf::from_i32(m.pvars[0x56] as i32);
    let spin = sv::quat_nlerp(t, p::v4(&m.pvars, 0x30), q);
    p::set_v4(&mut m.pvars, 0x30, spin);
    let pos = sv::lerp3(pv(m.position), target, f);
    m.position = fv(pos);
    let f = f - Pf::b(0x3f00_0000);
    let k = Pf::from_i32(m.pvars[0x56] as i32) * Pf::b(0x3ccc_cccd);
    let f = f * f;
    let f = f - Pf::b(0x3e80_0000);
    let f = -f;
    let f = f * k;
    m.position[2] = sv::fl(sv::pf(m.position[2]) + f);
    let r = sv::quat_rows(spin);
    set_rows3(m, r);
    m.pvars[0x57] = m.pvars[0x57].wrapping_sub(1);
    if m.pvars[0x57] == 0 {
        m.state = 3;
        let k = hop_ticks(w, 0x4120_0000, 0x41a0_0000);
        let m = w.mm(id);
        m.pvars[0x56] = k;
        m.pvars[0x57] = k;
    }
}

/// State 3: idle. Pick up range → fly; far dynamic bolts that are not drawn are deleted; else spin, hop
/// out and glint.
fn idle(w: &mut World, id: MobyId) {
    let pos = pv(w.m(id).position);
    let d = ph::dist2(pos, w.hero.pos);
    let (rx, rz) = w.bolt_radii();
    if d < rx && (pos[2] - w.hero.pos[2]).abs() < rz {
        start_fly(w, id, Pf::ZERO, Pf::ZERO);
        return;
    }
    // `sltu first_dynamic, moby`: the first dynamic slot itself does not count as dynamic.
    let visible = w.m(id).visible;
    let mut do_pose = visible != 0;
    if id > w.table.first_dynamic && w.table.free_slots < 200 && Pf::b(0x4240_0000) < d {
        if visible == 0 {
            w.delete_moby(id);
            return;
        }
        do_pose = true;
    }
    if !do_pose {
        let m = w.mm(id);
        if p::i32(&m.pvars, 0x5c) == 0 {
            if m.pvars[0x57] != 0 { m.pvars[0x57] -= 1; }
            return;
        }
    }
    pose(w, id, false);
    let m = w.mm(id);
    if m.pvars[0x57] != 0 {
        let t = Pf::from_i32(m.pvars[0x57] as i32) / Pf::from_i32(m.pvars[0x56] as i32);
        let k = Pf::from_i32(m.pvars[0x56] as i32);
        let t = t - Pf::b(0x3f00_0000);
        let k = k * Pf::b(0x3c4c_cccd);
        let t = t * t;
        let t = t - Pf::b(0x3e80_0000);
        let t = -t;
        let t = t * k;
        m.position[2] = sv::fl(sv::pf(m.position[2]) + t);
        m.pvars[0x57] -= 1;
    }
    let slot = p::i16(&m.pvars, 0x6c);
    let off = ph::mul_rows3(&rows3(m), glint_offset(m.o_class));
    let pos = pv(m.position);
    if slot >= 0 {
        let g = &mut w.svc.glints.entries[slot as usize & 0xf];
        g.pos = ph::vadd(off, pos);
        if g.moby as i32 != id as i32 { p::set_i16(&mut w.mm(id).pvars, 0x6c, -1); }
        return;
    }
    let t = (p::u16(&m.pvars, 0x6e)).wrapping_sub(1);
    p::set_i16(&mut m.pvars, 0x6e, t as i16);
    if (t as i16) > 0 { return; }
    let at = ph::vadd(off, pos);
    let slot = w.svc.glints.create(Pf::b(0x3fb3_3333), at, id as i16, w.rng);
    p::set_i16(&mut w.mm(id).pvars, 0x6c, slot);
    let (lo, hi) = (w.ticks(300), w.ticks(600));
    let wait = w.rng.rand_range(lo, hi);
    p::set_i16(&mut w.mm(id).pvars, 0x6e, wait as i16);
}

/// The carrier moby the bolt rests on (+0x5c; None: none, or the index is outside the table).
fn resting_on(w: &World, id: MobyId) -> Option<MobyId> {
    let v = p::i32(&w.m(id).pvars, 0x5c);
    usize::try_from(v - 1).ok().filter(|&i| v > 0 && i < w.table.mobys.len())
}

/// A settle on a moby (the collision's hit moby `0x1742d8`): when it is a carrier (`FUN_00275290`), +0x5c = it and
/// the rest pose +0x00 / +0x10 goes into its frame (`FUN_00275528(bolt, carrier, +0x00, +0x10, +0x00, +0x10)`).
/// Level01 `BoltUpdate` 0x2bb758, the init (after the ground probe) and the fall's settle.
fn attach(w: &mut World, id: MobyId, h: &CollOutput) {
    let Some(mid) = h.moby else { return };
    let Some(c) = w.table.mobys.get(mid).and_then(triggers::carrier) else { return };
    let m = w.mm(id);
    let (b, e) = (p::v4(&m.pvars, 0), p::v4(&m.pvars, 0x10));
    let (l, lr) = triggers::to_local(&c, [sv::fl(b[0]), sv::fl(b[1]), sv::fl(b[2])], [sv::fl(e[0]), sv::fl(e[1]), sv::fl(e[2])]);
    p::set_v4(&mut m.pvars, 0, [sv::pf(l[0]), sv::pf(l[1]), sv::pf(l[2]), b[3]]);
    p::set_v4(&mut m.pvars, 0x10, [sv::pf(lr[0]), sv::pf(lr[1]), sv::pf(lr[2]), e[3]]);
    p::set_i32(&mut m.pvars, 0x5c, mid as i32 + 1);
}

/// `FUN_002bc768(m, base_out, euler_out)`: the idle pose. The rest pose is +0x00 / +0x10, or, resting on a carrier
/// (+0x5c), that pose mapped out of the carrier's frame now (`FUN_002753b0`, [`triggers::from_local`]). Spins the
/// angles (bit 0 of +0x55 picks the direction), rows = spin rows (x = +0x50, y = tilt, z = +0x58) × rest rows,
/// position = rest + rest_up·k1 + row0·k2. Returns the rest position. In state 3 on a carrier with no hop left, the
/// bolt falls off (state 1) when the carrier is deleted or nothing touches its sphere any more
/// (`coll_sphere(r, pos, 0x22, bolt)`): spin 0, +0x3c = 1, velocity = the carrier's block displacement (0 without a
/// block) + `dt·unit(rest_up)` + `dt·unit(row 0)`, position += velocity, +0x5c = 0.
fn pose(w: &mut World, id: MobyId, _want_base: bool) -> V4 {
    let on = resting_on(w, id);
    let (mut base, mut base_euler) = { let m = w.m(id); (p::v4(&m.pvars, 0), p::v4(&m.pvars, 0x10)) };
    if let Some(c) = on.and_then(|k| triggers::carrier(w.m(k))) {
        let slot = w.m(id).class_slot;
        let mut l = [sv::fl(base[0]), sv::fl(base[1]), sv::fl(base[2])];
        let mut lr = [sv::fl(base_euler[0]), sv::fl(base_euler[1]), sv::fl(base_euler[2])];
        let (q, qr) = triggers::from_local(&c, slot, &mut l, &mut lr);
        let m = w.mm(id);
        p::set_v4(&mut m.pvars, 0, [sv::pf(l[0]), sv::pf(l[1]), sv::pf(l[2]), base[3]]);
        p::set_v4(&mut m.pvars, 0x10, [sv::pf(lr[0]), sv::pf(lr[1]), sv::pf(lr[2]), base_euler[3]]);
        base = [sv::pf(q[0]), sv::pf(q[1]), sv::pf(q[2]), base[3]];
        base_euler = [sv::pf(qr[0]), sv::pf(qr[1]), sv::pf(qr[2]), base_euler[3]];
    }
    let m = w.mm(id);
    let (step, tilt, k_up, k_row0, _r) = pose_consts(m.o_class);
    let (a50, a58) = (p::f(&m.pvars, 0x50), p::f(&m.pvars, 0x58));
    let small = Pf::b(0xbc4c_cccd);
    let (n50, n58) = if m.pvars[0x55] & 1 != 0 {
        (ph::fast_add_rotations(a50, step), ph::fast_add_rotations(a58, small))
    } else {
        (ph::fast_subtract_rotations(a50, step), ph::fast_subtract_rotations(a58, small))
    };
    p::set_f(&mut m.pvars, 0x50, n50);
    p::set_f(&mut m.pvars, 0x58, n58);
    let spin_e = [n50, tilt, n58, p::f(&m.pvars, 0x5c)];
    let spin = sv::euler_rows(spin_e);
    let rest = sv::euler_rows(base_euler);
    let rows = sv::mat3_mul(&[rest[0], rest[1], rest[2]], &[spin[0], spin[1], spin[2]]);
    set_rows3(m, rows);
    // pos = rest_up·k1 (w = rest_up.w, the whole quadword is stored); + row0·k2; + rest position.
    let a = sv::scale3(rest[2], k_up);
    let b = sv::scale3(rows[0], k_row0);
    let pos = ph::vadd(ph::vadd(a, b), base);
    m.position = fv(pos);
    if let Some(k) = on {
        if m.state == 3 && m.pvars[0x57] == 0 {
            let r = pose_consts(m.o_class).4;
            let off = w.m(k).is_deleted() || w.coll_sphere(pos, r, 0x22, Some(id)).is_none();
            if off {
                let carried = triggers::platform_delta(w.m(k)).map_or([0.0; 4], |d| d.displacement);
                let (up, row0) = (rest[2], rows[0]);
                let m = w.mm(id);
                m.state = 1;
                p::set_v4(&mut m.pvars, 0x30, [Pf::ZERO; 4]);
                p::set_f(&mut m.pvars, 0x3c, Pf::ONE);
                let v = ph::vadd(ph::set_len3(up, DT), ph::set_len3(row0, DT));
                let vel = ph::vadd(pv(carried), v);
                p::set_v4(&mut m.pvars, 0x20, vel);
                m.position = fv(ph::vadd(pv(m.position), vel));
                p::set_i32(&mut m.pvars, 0x5c, 0);
            }
        }
    }
    base
}

/// `FUN_002bcb90(a, b, m, target)` with the hero moby as the target ([`start_fly_to`]).
pub fn start_fly(w: &mut World, id: MobyId, a: Pf, b: Pf) -> bool {
    let t = w.hero_moby;
    start_fly_to(w, id, a, b, t)
}

/// `FUN_002bcb90(a, b, m, target)`: start flying to `target` (+0x60; the spin axis is taken toward it). The pickup
/// sound plays at most once per 3 ticks; bolts started within that window get a random path bend instead. (The flight
/// itself, state 4 of `BoltUpdate` 0x2bb758, homes on Ratchet's body point 0x13f420 whatever the target.)
pub fn start_fly_to(w: &mut World, id: MobyId, a: Pf, b: Pf, target: Option<MobyId>) -> bool {
    if w.m(id).state == 4 { return false; }
    let (mut a, mut b) = (a, b);
    let t = w.ticks(3);
    let c = w.counter as i32;
    let last = w.svc.last_bolt_sound;
    if last.wrapping_add(t) < c || c < last {
        w.svc.last_bolt_sound = c;
        w.play_sound(0, 0x20, id);
    } else {
        if a == Pf::ZERO { a = Pf(w.rng.randf_bits(0xc1f0_0000, 0x41f0_0000)) * DEG; }
        if b == Pf::ZERO { b = Pf(w.rng.randf_bits(0, 0x41f0_0000)) * DEG; }
    }
    if w.m(id).state == 3 {
        let rest = sv::euler_rows(p::v4(&w.m(id).pvars, 0x10));
        let s = Pf(w.rng.randf_bits(0x4040_0000, 0x40e0_0000));
        let vel = ph::set_len3(rest[2], s * DT);
        p::set_v4(&mut w.mm(id).pvars, 0x20, vel);
    }
    let tpos = target.map(|t| pv(w.m(t).position)).unwrap_or(w.hero.pos);
    let spin = w.svc.bolt_spin;
    let m = w.mm(id);
    p::set_f(&mut m.pvars, 0x4c, a);
    p::set_f(&mut m.pvars, 0x48, b);
    p::set_f(&mut m.pvars, 0x64, Pf::ZERO);
    p::set_i32(&mut m.pvars, 0x60, target.map(|t| t as i32 + 1).unwrap_or(0));
    let to = ph::vsub(tpos, pv(m.position));
    let axis = sv::cross_ba(p::v4(&m.pvars, 0x20), to);
    let axis = ph::set_len3(axis, Pf::ONE);
    p::set_v4(&mut m.pvars, 0x30, axis);
    let r = Pf(w.rng.randf_bits((spin * Pf::b(0x3eaa_7efa)).0, spin.0));
    let ang = (r * DEG) * DT;
    let m = w.mm(id);
    let q = sv::axis_angle_quat(ang, p::v4(&m.pvars, 0x30));
    p::set_v4(&mut m.pvars, 0x30, q);
    m.state = 4;
    true
}

/// The fly target stored at pvar+0x60 (moby index + 1, 0 = none).
fn target_of(pv_: &[u8]) -> Option<MobyId> { let t = p::i32(pv_, 0x60); (t > 0).then(|| (t - 1) as usize) }

/// State 4: fly to the hero's body point; pick up within the fly speed.
fn fly(w: &mut World, id: MobyId) {
    let target = target_of(&w.m(id).pvars);
    let mut below = false;
    if target.map(|t| w.m(t).o_class == 0).unwrap_or(false) {
        let rel = ph::vsub(pv(w.m(id).position), w.hero.ground_point);
        let rel = ph::mul_rows4(&w.hero_inverse(), rel);
        if rel[2] < Pf::ZERO { below = true; }
    }
    if below {
        let up = ph::set_len3(w.hero.rows[2], DT * Pf::b(0x40e0_0000));
        let m = w.mm(id);
        let vel = p::v4(&m.pvars, 0x20);
        let mut dv = ph::vsub(up, vel);
        sv::clamp_len(&mut dv, DT2 * Pf::b(0x4248_0000));
        p::set_v4(&mut m.pvars, 0x20, ph::vadd(vel, dv));
    } else {
        let (eff_len, disp) = (w.hero.eff_len, w.hero.disp);
        let m = w.mm(id);
        let drag = DT2 * Pf::b(0x4120_0000);
        let vel = p::v4(&m.pvars, 0x20);
        if drag < ph::len3(vel) {
            let d = ph::set_len3(vel, drag);
            p::set_v4(&mut m.pvars, 0x20, ph::vsub(vel, d));
        }
        let mut speed = p::f(&m.pvars, 0x64);
        ph::approach(DT * Pf::b(0x4240_0000), DT2 * Pf::b(0x4180_0000), &mut speed);
        p::set_f(&mut m.pvars, 0x64, speed);
        let k = DT * Pf::b(0x4100_0000);
        if k < eff_len {
            let push = ph::set_len3(disp, (eff_len - k) * Pf::b(0x3f00_0000));
            m.position = fv(ph::vadd(pv(m.position), push));
        }
    }
    let (hero_rows, inv, body) = (w.hero.rows, w.hero_inverse(), w.hero.body_point);
    let m = w.mm(id);
    let q = sv::quat_rows(p::v4(&m.pvars, 0x30));
    let r = sv::mat3_mul(&q, &rows3(m));
    set_rows3(m, r);
    let pos = ph::vadd(pv(m.position), p::v4(&m.pvars, 0x20));
    m.position = fv(pos);
    let mut to = ph::vsub(body, pos);
    let speed = p::f(&m.pvars, 0x64);
    if speed < ph::len3(to) {
        let (b48, b4c) = (p::f(&m.pvars, 0x48), p::f(&m.pvars, 0x4c));
        if !(b4c == Pf::ZERO && b48 == Pf::ZERO) {
            to = ph::mul_rows3(&[inv[0], inv[1], inv[2]], to);
            let bend = sv::euler_rows([Pf::ZERO, b48, b4c, Pf::ZERO]);
            to = ph::mul_rows3(&[bend[0], bend[1], bend[2]], to);
            to = ph::mul_rows3(&[hero_rows[0], hero_rows[1], hero_rows[2]], to);
        }
        let step = ph::set_len3(to, speed);
        m.position = fv(ph::vadd(pos, step));
        return;
    }
    collect(w, id);
    if id < w.table.first_dynamic {
        let (spawn, mission) = (w.m(id).spawn_id, w.m(id).mission);
        w.svc.save.killed.insert(spawn, mission.wrapping_add(2));
        let write = mission == 0xff
            || (w.missions.mission_slot(mission) != 0xff && w.mission_done(w.svc.level, mission) == 0xff);
        if write { w.svc.save.collected.insert(spawn, mission.wrapping_add(2)); }
    }
    w.delete_moby(id);
}

/// `CollectBolt` 0x2bc4f0: the counters, the HUD refresh and two pickup sparkles (type 53). Draws:
/// `rand_vec(0.7·dt, dt)` (3), `randi(2)`, `randf(0.4, 0.5)`.
pub fn collect(w: &mut World, id: MobyId) {
    let (o_class, challenge, parent, pos) = {
        let m = w.m(id);
        (m.o_class, m.pvars.get(0x40).copied().unwrap_or(0), m.parent, pv(m.position))
    };
    let val = value(o_class);
    let lvl = w.svc.level;
    if let Some(par) = parent {
        let b1 = w.m(par).spawn_flag;
        if b1 & 0x80 == 0 {
            let e = w.svc.counters.spawner_bolts.entry((lvl, b1)).or_default();
            *e = e.wrapping_add(val as i16);
        }
    }
    w.svc.counters.bolts += val;
    if challenge != 0 || (w.svc.counters.challenge_on != 0 && id < w.table.first_dynamic) {
        w.svc.counters.challenge_bolts += val;
    }
    w.svc.counters.level_bolts[(lvl as usize).min(19)] += val;
    w.svc.counters.hud_bolt_refresh += 1;
    // `queue_animation_update(2, 0x754e, …)`: the bolt counter (crate::hud).
    w.svc.hud.queue(crate::hud::Request::BOLTS);
    let sp = w.rng.rand_vec(sv::fl(DT * Pf::b(0x3f33_3333)), sv::fl(DT));
    let sp: V4 = [sv::pf(sp[0]), sv::pf(sp[1]), sv::pf(sp[2]), Pf::ZERO];
    let at = ph::vadd(sv::scale3(sp, Pf::b(0x4120_0000)), pos);
    let r = w.rng.randi(2);
    let sgn = if r != 0 { r } else { -1 };
    let g = Pf(w.rng.randf_bits(0x3ecc_cccd, 0x3f00_0000));
    let t = w.ticks(0x19);
    w.part53(g * Pf::b(0x3e4c_cccd), g, Pf::ZERO, at, t, 0x7f20_7f7f, 0, sgn as i8, sp);
    let t = w.ticks(0x19);
    w.part53(g * Pf::b(0x3e0f_5c29), g * Pf::b(0x3f33_3333), Pf::ZERO, at, t, 0x7f7f_7f7f, 1, (-sgn) as i8, sp);
}

/// `BoltSpawn(src, pos, vel, flags, value, dc)` 0x2bcdb8: a dropped bolt of `value` (1/5/20/50 → class
/// 13/14/15/16) in state 1 (falling), spinning about `(src − bolt) × vel`. When `CreateMoby` fails the value
/// goes straight to the bolt count. Flags 2: it flies to the hero at once (with a random bend when flags
/// 0x10). Draws: `rand_range(0, ticks(600))`, `randi(2)`, 2 × `rand_angle`, `randf(0, 720)`, (+ `randf(−30,
/// 30)` with 0x12) + the fly start's.
pub fn spawn(w: &mut World, src: MobyId, pos: V4, vel: V4, flags: u32, value: i32, dc: u8) -> Option<MobyId> {
    let (class, k) = match value {
        5 => (14, Pf::b(0x3f33_3333)),
        0x14 => (15, Pf::b(0x3f40_0000)),
        0x32 => (16, Pf::b(0x3f19_999a)),
        _ => (13, Pf::b(0x3f26_6666)),
    };
    let Some(b) = w.create_moby(class) else {
        w.svc.counters.bolts += value;
        return None;
    };
    let mut root = src;
    if w.m(src).parent.is_some() {
        while let Some(p) = w.m(root).parent { root = p; }
    }
    let cs = w.class_scale(class);
    let (src_light, src_amb, src_pos) = { let s = w.m(src); (s.light, s.ambient, pv(s.position)) };
    // A dropper with the mode-0x20 pvar header and a hit-flash record (+0x0c): the flash record's saved ambient
    // (+4..+6, the colour before any flash) through `FUN_002650d0`; otherwise its light word and ambient (+0x38).
    let flash_ambient = {
        let s = w.m(src);
        let f = if s.mode & 0x20 != 0 && s.pvars.len() >= 0x10 { p::u32(&s.pvars, 0xc) as usize } else { 0 };
        (f != 0 && f + 7 <= s.pvars.len()).then(|| [s.pvars[f + 4], s.pvars[f + 5], s.pvars[f + 6]])
    };
    {
        let m = w.mm(b);
        m.pvars[0x40] = dc;
        m.mode |= mode::KEEP_ROWS;
        m.parent = Some(root);
        m.b73 = 0x14;
        m.update_dist = 0x40;
        m.draw_dist = 0x40;
        m.state = 1;
        m.visible = 1;
        m.scale = sv::fl(cs * k);
        let r = sv::euler_rows(pv(m.rotation));
        set_rows3(m, [r[0], r[1], r[2]]);
        match flash_ambient {
            Some(a) => m.ambient = [a[0], a[1], a[2], 0],
            None => {
                m.light = src_light;
                m.ambient = src_amb;
            }
        }
    }
    let t600 = w.ticks(600);
    let wait = w.rng.rand_range(0, t600);
    {
        let m = w.mm(b);
        p::set_i16(&mut m.pvars, 0x6e, wait as i16);
        p::set_i16(&mut m.pvars, 0x6c, -1);
        m.position = fv(pos);
        p::set_v4(&mut m.pvars, 0x20, vel);
    }
    if w.rng.randi(2) != 0 { w.mm(b).pvars[0x55] |= 1; }
    let a1 = Pf(w.rng.rand_angle_bits());
    let a2 = Pf(w.rng.rand_angle_bits());
    let spin = w.svc.bolt_spin;
    {
        let m = w.mm(b);
        p::set_f(&mut m.pvars, 0x50, a1);
        p::set_f(&mut m.pvars, 0x58, a2);
        p::set_f(&mut m.pvars, 0x68, sv::pf(m.position[2]));
    }
    let r = Pf(w.rng.randf_bits(0, spin.0));
    let ang = (r * DEG) * DT;
    {
        let m = w.mm(b);
        let to = ph::vsub(src_pos, pv(m.position));
        let axis = ph::set_len3(sv::cross_ba(p::v4(&m.pvars, 0x20), to), Pf::ONE);
        p::set_v4(&mut m.pvars, 0x30, axis);
        let q = sv::axis_angle_quat(ang, axis);
        p::set_v4(&mut m.pvars, 0x30, q);
    }
    if flags & 2 != 0 {
        let mut a = Pf::ZERO;
        if flags & 0x10 != 0 { a = Pf(w.rng.randf_bits(0xc1f0_0000, 0x41f0_0000)) * DEG; }
        start_fly(w, b, a, Pf::ZERO);
    }
    w.build_matrix(b);
    Some(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::services::Services;

    /// A carrier at the origin (yaw 0, block at +0x20) and a class-13 bolt resting on it at local (2, 0, 1).
    fn scene() -> MobyTable {
        let mut k = Moby { o_class: 707, pvars: vec![0; 0x80], ..Moby::default() };
        k.mode |= 0x20;
        p::set_i32(&mut k.pvars, 8, 0x20);
        for i in 0..3 { k.rows[i][i] = 1.0; }
        let mut b = Moby { o_class: 13, state: 3, pvars: vec![0; 0x80], ..Moby::default() };
        p::set_v4f(&mut b.pvars, 0, [2.0, 0.0, 1.0, 0.0]);
        p::set_i32(&mut b.pvars, 0x5c, 1);
        p::set_i16(&mut b.pvars, 0x6e, 100);
        p::set_i16(&mut b.pvars, 0x6c, -1);
        b.visible = 1;
        MobyTable::new(vec![k, b], 4)
    }

    fn tick(t: &mut MobyTable, f: impl Fn(&mut World)) {
        let mut hero = crate::hero::Hero::new();
        hero.pos[0] = Pf::b(0x4480_0000); // far away: no pickup
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = Services::new();
        let mut w = World::new(t, &hero, &mut rng, &classes, &mut svc, 1);
        f(&mut w);
    }

    /// A bolt on a carrier: the rest pose follows the carrier's turn and move (`FUN_002753b0`); with a hop left
    /// it stays on.
    #[test]
    fn resting_bolt_rides_its_carrier() {
        let mut t = scene();
        t.mobys[1].pvars[0x56] = 10;
        t.mobys[1].pvars[0x57] = 5;
        // The carrier turned a quarter about z and moved to (5, 0, 0).
        let q = std::f32::consts::FRAC_PI_2;
        let r = triggers::euler_matrix([0.0, 0.0, q]);
        let k = &mut t.mobys[0];
        k.position = [5.0, 0.0, 0.0, 1.0];
        for (row, src) in k.rows.iter_mut().zip(&r) { for (x, y) in row.iter_mut().zip(src) { *x = *y as f32; } }
        tick(&mut t, |w| { pose(w, 1, false); });
        let b = &t.mobys[1];
        let base_x = 5.0 + 2.0 * r[0][0] as f32;
        let base_y = 2.0 * r[0][1] as f32;
        assert_eq!(b.state, 3);
        assert_eq!(p::i32(&b.pvars, 0x5c), 1);
        // position = rest + rest_up·k1 + row0·k2: the rest point is (5, 2, 1) and the rest yaw the carrier's.
        let (_, _, k_up, k_row0, _) = pose_consts(13);
        let expect_x = base_x + b.rows[0][0] * sv::fl(k_row0);
        assert!((b.position[0] - expect_x).abs() < 1e-4 && (b.position[1] - (base_y + b.rows[0][1] * sv::fl(k_row0))).abs() < 1e-4, "{:?}", b.position);
        assert!((b.position[2] - (1.0 + sv::fl(k_up) + b.rows[0][2] * sv::fl(k_row0))).abs() < 1e-4);
        assert_eq!(p::v4f(&b.pvars, 0)[..3], [2.0, 0.0, 1.0], "the local pose is kept (block flag bit 2 clear)");
    }

    /// No hop left and nothing touching (no collision here): the bolt falls off with the carrier's velocity plus
    /// `dt` along its rest up and row 0, and forgets the carrier.
    #[test]
    fn bolt_falls_off_when_nothing_holds_it() {
        let mut t = scene();
        triggers::carry_riders(&mut t.mobys[0].pvars, 0x20, [0.1, 0.0, 0.0, 0.0], [0.0; 4], [0.0; 4]);
        tick(&mut t, |w| { pose(w, 1, false); });
        let b = &t.mobys[1];
        assert_eq!((b.state, p::i32(&b.pvars, 0x5c)), (1, 0));
        assert_eq!(p::v4f(&b.pvars, 0x30), [0.0, 0.0, 0.0, 1.0], "the spin is the identity quaternion");
        let v = p::v4f(&b.pvars, 0x20);
        // The rest rows are the identity (no turn, rest Euler 0): up = z; row 0 is the spun row the pose left.
        let (dt, r0) = (DT.to_f32(), b.rows[0]);
        let n = (r0[0] * r0[0] + r0[1] * r0[1] + r0[2] * r0[2]).sqrt();
        let expect = [0.1 + dt * r0[0] / n, dt * r0[1] / n, dt + dt * r0[2] / n];
        assert!((0..3).all(|k| (v[k] - expect[k]).abs() < 1e-6), "{v:?} vs {expect:?}");
    }
}
