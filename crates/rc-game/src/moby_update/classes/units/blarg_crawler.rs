//! **Blarg's crawlers, class 827** (level06 `0x2e8678` with its hits `0x2e94f0`, its ground probe `0x2e9aa8`, its
//! orientation `0x2e9998`, its move `0x2e9b60`, its pod break `0x2e9f30`, the goo `0x2ea198`, the burn puffs
//! `0x2ea498` and the gadgetbot pick `0x2f34d8` / `0x2f33e8`; 169 placed, 131 made; level 10 runs the same class at
//! `0x2cc8b8`; census U223; the name is descriptive [L]). Small goo creatures that crawl on any surface, their rows
//! turned to the ground under them. Some wait in a pod (+0x200: class 500, broken when the target comes within 2), some
//! hide until other code shows them (+0x202), some walk in along a path (+0x1f8) and leap off its end. At home they
//! wander within 2; they notice their target (Ratchet inside the region path +0x1f4 and within the sight of home,
//! or a forced moby +0x204; as Clank, the nearest gadgetbot 857 or Clank by distance and angle) and creep at it with a
//! random angle (±+0x1d8°), lunge-bite (keys 10–15: a 0.333 sphere at joint list 0, damage 1) and go home when it is
//! lost. Alerted (the lure), they push along the walls (path +0x1ec) and rear up when one is in the way. Hits burst
//! them into goo; with lives (+0x240) left they hide at home (state 0xf) for other code to wake. The Suck Cannon takes
//! them ([`crate::moby_update::creature::react::BLARG_827`]). They run only on their map side (+0x242: the alternate
//! side, `MapState::alt`). Read from the level06 decomp and disassembly; its words gp−0x5010..−0x4f9c. Native `f32`.
//!
//! **Pvars** (0x290; the header: damage +0x20, flash +0x110, knockback +0x120, the suck record +0x60): +0x20 the damage
//! record (+0x24 the health to restore, +0x29 1, +0x30 the up's z / 4), +0x38 the lure, +0x60 the suck record, +0x110
//! the flash, +0x120 the knockback record (+0x14e its burn mark), +0x180 the target record (+0x1c0 its moby, here the
//! runtime index + 1; +0x1c4 its kind), +0x1d4 the approach angle (±+0x1d8°), +0x1dc / +0x1e0 the sight / this tick's
//! (+6 alerted), +0x1e4 the speed, +0x1e8 (s16) the alert, +0x1ea (s16) hidden by game mode 2, +0x1ec the walls,
//! +0x1f4 the region, +0x1f8 the entry path, +0x1fc the wander timer, +0x200 / +0x202 (s16) in a pod / hidden, +0x204
//! a forced target, +0x208 (s16) the entry node, +0x20a (s16) its eighth of the ticks, +0x20c the pod (moby + 1),
//! +0x210 home, +0x220 its Euler at home, +0x230 the wander point, +0x240 (s16) lives, +0x242 (s16) the map side,
//! +0x244 the wall reach (`randf(0.5, 3)`), +0x248 the glow phase, +0x24c the entry path's wake distance, +0x250 the
//! big-head node.
//!
//! | state | what | port |
//! |---|---|---|
//! | top | (gp−0x4ff0 ≠ 0 freezes it: never set); game mode 2 → hidden (mode \|= 0x41, +0x1ea); mode 0 → shown; off its map side (outside 0x10) → nothing; the big head (2.1, list 1, +0x250); the hits; drawn within 14 of the camera → the shadow probe, +0x7f 0xb | [`update`] |
//! | 0 | +0x20a = the class counter (gp−0x4fec) mod 8, the counter + 1; moby +0xb4 /= lives; mode \|= 0x100; +0x58 8, +0x5a 3; home, its Euler, the wander point; node 0; wall reach `randf(0.5, 3)`; K +0x28 0, +0x14 0, radius 0x200, gravity 20·dt²; glow phase `random_angle_radians`; +0x29 1; a pod (+0x200) → 1, the pod 500 made here (its light, mode \|= 0x4002, drawn, draw distance 0x40) and itself hidden (none → deleted); hidden (+0x202) → 0xf (no collision, not targetable, hidden); no entry path → 8, seq 4 at a random frame; else 3, seq 0 | [`init`] |
//! | 1 | the target within 2 (xy) → shown, the pod broken (`0x2e9f30`), 8, seq 4 (`ticks(5)`) | [`update`] |
//! | 3 | turned to its target (`0x2e9aa8` probe, `0x2e9998`); Ratchet within +0x24c (xy) → 4 (seq 4, `ticks(5)`) | [`update`] |
//! | 4 | toward node +0x208 of the entry path (the probe, the turn, a step along row 0 of speed·dt, `0x2e9b60(0.3, 0.3, step, 0)`); within 0.5 (xy, the offset through its rows): the last two nodes → home = the wander point = the last point, a lob there (speed = distance / `ticks(30)`, up `0x269a00`, K +0x3d 3, flags 5, `0x26b368(heading, m, K, 2, 1, 0)`), 5; else the next node | [`entry`] |
//! | 5 / 6 | the flight (5 turned up along its velocity); landed → 7, seq 3 (1 tick), anim time 1, z = `GroundHeight(0.5)`; 6 out of the world (0x120) → `SetDeathBits`, deleted | [`update`] |
//! | 7 | the animation's end → 8 (seq 4, `ticks(5)`) | [`update`] |
//! | 8 | drawn: at the wander point (0.05) or the timer out → a point within `randf(0.5, 2)` of home, timer `ticks(randf(45, 75))`; crawl there (walls pushed off); no target: alerted → 10; a target: `randi(9)` = 0 → 9, else alerted → 10; both with a new angle (`randf(±+0x1d8)`°) and seq 4 | [`wander`] |
//! | 9 / 10 | crawl along the angle off the target heading; 10: `ClampToPath(walls, here, target)` within the wall reach → 0xb (seq 6, `ticks(10)`); 9 / 10 within 2 → 0xc (seq 5, `ticks(10)`; 9 from frame 4); 9 without a target → 0xd; 10 not alerted → 0xd | [`chase`] |
//! | 0xb | the animation's end → 10 (seq 4, `ticks(20)`) | [`update`] |
//! | 0xc | turned to the target about its up; blended in, keys 10–15 and Ratchet not in state 0x51: a double step and `0x2687f8(0.333, 1, 1, m, 0, 1, 0, 1, 0)`; the animation's end beyond 2 → 9 (seq 4, `ticks(10)`) | [`bite`] (`attack::joint_hit`) |
//! | 0xd | crawl home; within 2 → 8 (seq 4, `ticks(10)`); a target → 9; alerted → 10 | [`update`] |
//! | 0x10 | the Suck Cannon's (`0x2ed658` = `react::carried`); released → 8 (seq 0, `ticks(10)`), suck record +0x68 0 | [`update`] |
//! | `0x2e94f0` | +0x30 = row 2's z / 4; a hit on the pod (0x330000; not by class 0xaf) → the goo, the pod broken, `SetDeathBits`, deleted; its own hit (0x330000) through the resolver (column 4); out5 ≠ 1 outside 0xe: health −= damage (≤ 0 → reaction 1); reactions 1 / 2: `SetDeathBits`, the goo along the hit, burned → the burn puffs, sound 3, the pod broken; no lives → deleted; else lives − 1, health = the meter, home, 0xf hidden | [`hits`] |
//! | `0x2e94f0` tail | +0xa4 0xff; the glow (phase += 360°·dt, 0x80808080 → 0x80202020); the lure → alert `scale(randf(180, 240))`; the alert ticks; sight +6 alerted; the target: forced (+0x204) → it, kind 1; Clank (0x1413f4 = 1) on its eighth of the ticks: [`gadgetbot`], outside the region or a gadgetbot beyond 2 → none; else `0x26eac8(sight, m, +0x180, 0, 0, region)`, beyond the sight of home or 3 in height → none; none → Ratchet's moby | [`hits`] (`target::acquire_in`) |
//! | `0x2e9b60(r, s, m, step, walls)` | step += −dt·up; from 0.3·up above: a line along the step longer than `s` hitting (flags 0) → stopped there; six sphere push-outs (0.3, flags 4); `walls` → pushed 0.5 off the walls (`0x270d00`) | [`crawl`] |
//! | `0x2e9998(m, n, d)` | rows toward (d without its n part, n×…, n), slerped 0.1 of the way (gp−0x5000) | [`orient`] |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::debris;
use crate::moby_update::creature::{self as c, attack, damage, fx, knock, react, region, target, V};
use crate::moby_update::manip;
use crate::moby_update::services::{pf, pv as v4, World};
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 6;
pub const UPDATE_FN: u32 = 0x2e_8678;
pub const CLASSES: [i16; 1] = [827];
/// The pod (class 500) and its shards (0x165, 0x166 / 0x167).
pub const POD: i16 = 500;
pub const SHARDS: [i16; 3] = [0x165, 0x166, 0x167];
/// Clank's gadgetbots, which the crawlers chase while Ratchet is Clank.
pub const GADGETBOT: i16 = 0x359;
/// gp−0x5000: the turn's slerp; gp−0x4ffc (360): the glow's turn; gp−0x4ff8 / −0x4ff4 the glow colours.
const ORIENT_T: f32 = 0.1;
const GLOW_RATE: f32 = 360.0;
const GLOW: (u32, u32) = (0x8080_8080, 0x8020_2020);
/// The goo of `0x2ea198` (gp−0x4fe8..−0x4f9c).
const GOO: fx::Goo = fx::Goo { spread: 0.25, lift: 0.25, blob_spread: 0.125, k: 0.0 };
const DEG: f32 = 0.017_453_292;

/// Pvar offsets (module doc).
pub mod pv {
    pub const D: usize = 0x20;
    pub const LURE: usize = 0x38;
    pub const SUCK: usize = 0x60;
    pub const F: usize = 0x110;
    pub const K: usize = 0x120;
    pub const TGT_REC: usize = 0x180;
    pub const TGT: usize = 0x1c0;
    pub const KIND: usize = 0x1c4;
    pub const ANGLE: usize = 0x1d4;
    pub const ANGLE_DEG: usize = 0x1d8;
    pub const SIGHT_BASE: usize = 0x1dc;
    pub const SIGHT: usize = 0x1e0;
    pub const SPEED: usize = 0x1e4;
    pub const ALERT: usize = 0x1e8;
    pub const MODE2: usize = 0x1ea;
    pub const WALLS: usize = 0x1ec;
    pub const REGION: usize = 0x1f4;
    pub const ENTRY: usize = 0x1f8;
    pub const WANDER_T: usize = 0x1fc;
    pub const IN_POD: usize = 0x200;
    pub const HIDDEN: usize = 0x202;
    pub const FORCED: usize = 0x204;
    pub const NODE: usize = 0x208;
    pub const PHASE: usize = 0x20a;
    pub const POD: usize = 0x20c;
    pub const HOME: usize = 0x210;
    pub const HOME_ROT: usize = 0x220;
    pub const WANDER: usize = 0x230;
    pub const LIVES: usize = 0x240;
    pub const SIDE: usize = 0x242;
    pub const WALL_REACH: usize = 0x244;
    pub const GLOW: usize = 0x248;
    pub const WAKE: usize = 0x24c;
    pub const BIG_HEAD: usize = 0x250;
    pub const SIZE: usize = 0x290;
}
/// The class counter gp−0x4fec (level06 0x161c14): each crawler takes the next eighth of the ticks.
const COUNTER_KEY: u32 = 0x16_1c14;

fn state(w: &World, id: MobyId) -> u8 { w.m(id).state }
fn set_state(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }
fn done(w: &World, id: MobyId) -> bool { w.m(id).anim.flags & 2 != 0 }
/// `if (m+0x53 != seq) fun_00212f90(m, seq, frame, t)` (`t` in ticks already).
fn blend(w: &mut World, id: MobyId, seq: u8, frame: i32, t: i32) { c::blend_to(w, id, seq, frame, t); }
fn blend_t(w: &mut World, id: MobyId, seq: u8, n: i32) {
    let t = w.ticks(n);
    c::blend_to(w, id, seq, 0, t);
}
fn gscale(w: &World, x: f32) -> f32 { w.svc.timing.scale(Pf::f(x)).to_f32() }
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> {
    usize::try_from(c::pi32(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len())
}
fn tgt(w: &World, id: MobyId) -> Option<MobyId> { link(w, id, pv::TGT) }
fn tgt_pos(w: &World, id: MobyId) -> V { tgt(w, id).map_or([0.0; 4], |m| w.m(m).position) }
fn path(w: &World, o: i32) -> Vec<V> {
    usize::try_from(o).ok().and_then(|p| w.svc.splines.get(p)).map_or(Vec::new(), |s| s.iter().map(|q| q.map(f32::from_bits)).collect())
}
fn rows(w: &World, id: MobyId) -> [V; 3] { let r = w.m(id).rows; [r[0], r[1], r[2]] }
fn alerted(w: &World, id: MobyId) -> bool { c::pi16(w, id, pv::ALERT) != 0 }
fn no_target(w: &World, id: MobyId) -> bool { c::pi32(w, id, pv::KIND) == 2 }

/// `0x2e9aa8(m)`: a line from 1 above to 1 below along its up (flags 2); the ground's normal, or its own up when the
/// line misses (the game reads the collision output's last normal then [L]).
fn probe(w: &World, id: MobyId) -> V {
    let p = c::pos(w, id);
    let up = c::set_len3(rows(w, id)[2], 1.0);
    let a = c::add(p, up);
    let b = c::sub(p, up);
    match w.coll_line(v4(a), v4(b), 2, None) {
        Some(o) => [o.normal[0], o.normal[1], o.normal[2], 0.0],
        None => rows(w, id)[2],
    }
}

/// `0x2e9998(m, n, d)` (module doc).
fn orient(w: &mut World, id: MobyId, n: V, d: V) {
    let u = c::set_len3(n, 1.0);
    let a = c::scale(u, c::dot3(u, d));
    let f = c::set_len3(c::sub(d, a), 1.0);
    // FastVecCross(out, f, u) = u × f.
    let s = [u[1] * f[2] - u[2] * f[1], u[2] * f[0] - u[0] * f[2], u[0] * f[1] - u[1] * f[0], 0.0];
    let want = react::rows_quat([[f[0], f[1], f[2]], [s[0], s[1], s[2]], [u[0], u[1], u[2]]]);
    let r = rows(w, id);
    let have = react::rows_quat([[r[0][0], r[0][1], r[0][2]], [r[1][0], r[1][1], r[1][2]], [r[2][0], r[2][1], r[2][2]]]);
    let q = react::slerp(ORIENT_T, have, want);
    let m = react::quat_rows(q);
    let mm = w.mm(id);
    for (i, row) in m.iter().enumerate() { mm.rows[i] = [row[0], row[1], row[2], mm.rows[i][3]]; }
}

/// `0x2e9b60(r, s, m, step, walls)` (module doc).
fn crawl(w: &mut World, id: MobyId, step: V, walls: bool) {
    let (r, s) = (0.3f32, 0.3f32);
    let up = c::set_len3(rows(w, id)[2], r);
    let step = c::add(step, c::set_len3(up, -c::DT));
    let from = c::add(c::pos(w, id), up);
    let p = c::add(c::pos(w, id), step);
    c::set_pos(w, id, p);
    let mut at = c::add(p, up);
    if s < c::len3(step) {
        if let Some(o) = w.coll_line(v4(from), v4(at), 0, Some(id)) {
            at = [o.point[0], o.point[1], o.point[2], at[3]];
            c::set_pos(w, id, c::sub(at, up));
        }
    }
    for _ in 0..6 {
        let Some(o) = w.coll_sphere(v4(at), pf(s), 4, Some(id)) else { break };
        let Some(q) = o.pushed_centre else { break };
        at = [q[0], q[1], q[2], at[3]];
        c::set_pos(w, id, c::sub(at, up));
    }
    if walls {
        let wl = c::pi32(w, id, pv::WALLS);
        if let Ok(path) = usize::try_from(wl) {
            let p = c::pos(w, id);
            if let Some(q) = region::push_out(w, 0.5, path, p) { c::set_pos(w, id, q); }
        }
    }
}

/// Probe, turn toward `d` and crawl a step of `k`·speed·dt along row 0.
fn crawl_toward(w: &mut World, id: MobyId, d: V, k: f32, walls: bool) {
    let n = probe(w, id);
    orient(w, id, n, d);
    let step = c::set_len3(rows(w, id)[0], k * c::pf(w, id, pv::SPEED) * c::DT);
    crawl(w, id, step, walls);
}

/// `0x2ea198(m, dir)`: the goo burst.
fn goo(w: &mut World, id: MobyId, dir: V) { fx::goo_burst_with(w, id, dir, &GOO); }

/// `0x2ea498(m)`: 20 burn puffs (type 4) within 0.5 of it, 0.5 up.
fn burn_puffs(w: &mut World, id: MobyId) {
    for _ in 0..20 {
        let a = w.rng.rand_angle();
        let b = w.rng.rand_angle();
        let mut v = fx::polar(c::DT * 1.8, a, b);
        v[2] += c::DT * 0.9;
        let r = w.rng.rand_vec(0.0, 0.5);
        let p0 = c::pos(w, id);
        let p = [r[0] + p0[0], r[1] + p0[1], r[2] + p0[2] + 0.5, p0[3]];
        let n = w.rng.rand_range(20, 35);
        let life = w.ticks(n);
        let growth = w.rng.rand_range(40, 60) as i16;
        let s = crate::particles::type04::Spawn { pos: p, vel: v, c1: 0x7000_a0ff, c2: 0xff, life, base: 0x1e, growth, additive: true };
        fx::part04(w, &s);
    }
}

/// `0x2e9f30(m)`: the pod broken: sound 4, 4 + 8 shards (0x165; 0x166 / 0x167) thrown from 0.5 above, the pod deleted.
fn break_pod(w: &mut World, id: MobyId) {
    let Some(pod) = link(w, id, pv::POD) else { return };
    w.play_sound(4, 0, id);
    let p0 = c::pos(w, id);
    let base = [p0[0], p0[1], p0[2] + 0.5, p0[3]];
    for k in 0..12 {
        let s = w.rng.randf(1.0, 4.0) * c::DT;
        let x = w.rng.randf(-0.625, 0.625);
        let y = w.rng.randf(-0.625, 0.625);
        let z = w.rng.randf(0.0, 1.25);
        let off = [x, y, z, 0.0];
        let vel = c::set_len3(off, s);
        let at = c::add(off, base);
        if k < 4 {
            debris::spawn(w, 0.3, 0.6, pod, vel, at, [0.0; 4], SHARDS[0], 0);
        } else {
            let i = w.rng.randi(2) as usize;
            debris::spawn(w, 0.5, 0.7, pod, vel, at, [0.0; 4], SHARDS[1 + i], 0);
        }
    }
    w.delete_moby(pod);
    c::set_pi32(w, id, pv::POD, 0);
}

/// `0x2f34d8(range, m, &target)` with `0x2f33e8`: Clank's chasers pick a gadgetbot (state 2.., not 9) or Clank by
/// distance + 5·angle (the current one 2 nearer).
fn gadgetbot(w: &World, id: MobyId, range: f32) -> Option<MobyId> {
    let me = c::pos(w, id);
    let cur = tgt(w, id);
    let mut cands: Vec<MobyId> = (0..w.table.mobys.len())
        .filter(|&m| { let o = w.m(m); o.o_class == GADGETBOT && 1 < o.state && o.state != 9 && o.state < 0x7f })
        .collect();
    if let Some(h) = w.hero_moby { cands.push(h); }
    let mut best = (1.0e6f32, None);
    for m in cands {
        let q = w.m(m).position;
        let d = c::dist2(me, q);
        if range < d { continue; }
        let mut s = d + c::diff_rots(c::yaw(w, id), c::atan(q[0] - me[0], q[1] - me[1])) * 5.0;
        if Some(m) == cur { s -= 2.0; }
        if s < best.0 { best = (s, Some(m)); }
    }
    best.1
}

/// State 0 (module doc). True: it returned early (made its pod, hid or was deleted).
fn init(w: &mut World, id: MobyId) -> bool {
    let n = w.svc.units.word(COUNTER_KEY);
    c::set_pi16(w, id, pv::PHASE, (n % 8) as i16);
    w.svc.units.set_word(COUNTER_KEY, n + 1);
    let lives = c::pi16(w, id, pv::LIVES);
    if lives != 0 { w.mm(id).b4 /= lives; }
    w.mm(id).mode |= mode::KEEP_ROWS;
    c::set_pu8(w, id, 0x58, 8);
    c::set_pu8(w, id, 0x5a, f32::from_bits(0x404c_cccd) as u8);
    let (p, r) = (c::pos(w, id), w.m(id).rotation);
    c::set_pv4(w, id, pv::HOME, p);
    c::set_pv4(w, id, pv::WANDER, p);
    c::set_pv4(w, id, pv::HOME_ROT, r);
    c::set_pi16(w, id, pv::NODE, 0);
    let reach = w.rng.randf(0.5, 3.0);
    c::set_pf(w, id, pv::K + knock::k::ZOFF, 0.0);
    c::set_pf(w, id, pv::K + knock::k::DRAG, 0.0);
    c::set_pi32(w, id, pv::K + knock::k::RADIUS, 0x200);
    c::set_pf(w, id, pv::WALL_REACH, reach);
    c::set_pf(w, id, pv::K + knock::k::GRAVITY, c::DT2 * 20.0);
    let a = w.rng.rand_angle();
    c::set_pf(w, id, pv::GLOW, a);
    c::set_pu8(w, id, pv::D + 9, 1);
    if c::pi16(w, id, pv::IN_POD) != 0 {
        set_state(w, id, 1);
        let Some(pod) = w.create_moby(POD) else {
            w.delete_moby(id);
            return true;
        };
        c::set_pi32(w, id, pv::POD, pod as i32 + 1);
        let (light, ambient, rot) = { let m = w.m(id); (m.light, m.ambient, m.rotation) };
        let m = w.mm(pod);
        m.light = light;
        m.ambient = ambient;
        m.mode |= 0x4002;
        m.position = p;
        m.rotation = rot;
        m.draw_dist = 0x40;
        m.visible = 1;
        w.build_matrix(pod);
        w.mm(id).mode |= mode::HIDDEN | mode::NO_ANIM;
        return true;
    }
    if c::pi16(w, id, pv::HIDDEN) != 0 {
        hide(w, id);
        return true;
    }
    if c::pi32(w, id, pv::ENTRY) < 0 {
        set_state(w, id, 8);
        if w.m(id).anim.seq_b == 4 { return true; }
        let o = w.m(id).o_class;
        let n = w.classes.anim(o).and_then(|a| a.sequence(4)).map_or(0, |q| q.header.frame_count as i32);
        let f = w.rng.randi(n);
        w.anim_blend(id, 4, f, 0);
        return true;
    }
    set_state(w, id, 3);
    blend(w, id, 0, 0, 0);
    false
}

/// State 0xf: hidden, no collision, not targetable.
fn hide(w: &mut World, id: MobyId) {
    set_state(w, id, 0xf);
    let m = w.mm(id);
    m.has_collision = false;
    m.mode = (m.mode & !mode::TARGETABLE) | mode::HIDDEN | mode::NO_ANIM;
}

/// State 4 (module doc).
fn entry(w: &mut World, id: MobyId) {
    let pts = path(w, c::pi32(w, id, pv::ENTRY));
    let n = pts.len() as i32;
    let node = c::pi16(w, id, pv::NODE) as i32;
    let Some(&q) = usize::try_from(node).ok().and_then(|i| pts.get(i)) else { return };
    let d = c::sub(q, c::pos(w, id));
    crawl_toward(w, id, d, 1.0, false);
    let r = rows(w, id);
    let local = [0, 1, 2].map(|k| r[0][k] * d[0] + r[1][k] * d[1] + r[2][k] * d[2]);
    if 0.5 <= (local[0] * local[0] + local[1] * local[1]).sqrt() { return; }
    if node < n - 2 {
        c::set_pi16(w, id, pv::NODE, (node + 1) as i16);
        return;
    }
    let last = pts[n as usize - 1];
    c::set_pv4(w, id, pv::HOME, last);
    c::set_pv4(w, id, pv::WANDER, last);
    let me = c::pos(w, id);
    let dist = c::dist2(me, last);
    let t = w.ticks(30) as f32;
    let sp = dist / t;
    c::set_pf(w, id, pv::K + knock::k::SPEED, sp);
    let mut time = 0.0;
    let up = knock::lob_up(sp, -c::pf(w, id, pv::K + knock::k::GRAVITY), me, last, &mut time);
    c::set_pf(w, id, pv::K + knock::k::UP, up);
    c::set_pu8(w, id, pv::K + 0x3d, 3);
    c::set_pi32(w, id, pv::K + knock::k::FLAGS, 5);
    let h = c::atan(last[0] - me[0], last[1] - me[1]);
    knock::start(w, id, pv::K, h, 2, 1, 0);
    set_state(w, id, 5);
}

/// The landing of 5 / 6.
fn land(w: &mut World, id: MobyId) {
    set_state(w, id, 7);
    blend(w, id, 3, 0, 1);
    w.mm(id).anim.t = 1.0;
    let p = c::pos(w, id);
    let z = w.ground_height(Pf::f(0.5), v4(p), 0).to_f32();
    w.mm(id).position[2] = z;
}

/// State 8 (module doc).
fn wander(w: &mut World, id: MobyId) {
    if w.m(id).visible != 0 {
        let wp = c::pv4(w, id, pv::WANDER);
        if c::dist3(c::pos(w, id), wp) < 0.05 || c::dec_timer_pvar_i32(w, id, pv::WANDER_T) != 0 {
            let a = w.rng.rand_angle();
            let r = w.rng.randf(0.5, 2.0);
            let home = c::pv4(w, id, pv::HOME);
            c::set_pv4(w, id, pv::WANDER, [home[0] + a.cos() * r, home[1] + a.sin() * r, home[2], home[3]]);
            let f = w.rng.randf(45.0, 75.0);
            let t = w.ticks(f as i32);
            c::set_pi32(w, id, pv::WANDER_T, t);
        }
        let d = c::sub(c::pv4(w, id, pv::WANDER), c::pos(w, id));
        crawl_toward(w, id, d, 1.0, true);
    }
    // No target, or a target and `randi(9)` ≠ 0: only alerted, to 10; else to 9.
    let next = if no_target(w, id) || w.rng.randi(9) != 0 {
        if !alerted(w, id) { return; }
        10
    } else {
        9
    };
    let r = c::pf(w, id, pv::ANGLE_DEG);
    let a = w.rng.randf(-r, r);
    c::set_pf(w, id, pv::ANGLE, a * DEG);
    set_state(w, id, next);
    blend(w, id, 4, 0, 0);
}

/// States 9 / 10 (module doc).
fn chase(w: &mut World, id: MobyId, alert: bool) {
    let t = tgt_pos(w, id);
    let me = c::pos(w, id);
    let h = c::add_rot(c::atan(t[0] - me[0], t[1] - me[1]), c::pf(w, id, pv::ANGLE));
    crawl_toward(w, id, [h.cos(), h.sin(), 0.0, 0.0], 1.0, true);
    let me = c::pos(w, id);
    if !alert {
        if c::dist2(me, t) < 2.0 {
            set_state(w, id, 0xc);
            if w.m(id).anim.seq_b != 5 {
                let tk = w.ticks(10);
                w.anim_blend(id, 5, 4, tk);
            }
            return;
        }
        if no_target(w, id) { set_state(w, id, 0xd); }
        return;
    }
    let wl = c::pi32(w, id, pv::WALLS).max(0) as usize;
    let (crossed, at) = region::clamp(w, wl, me, t);
    let wall = crossed && c::dist3(me, at) < c::pf(w, id, pv::WALL_REACH);
    if !alerted(w, id) {
        set_state(w, id, 0xd);
    } else if wall {
        set_state(w, id, 0xb);
        blend_t(w, id, 6, 10);
    } else if c::dist2(me, t) < 2.0 {
        set_state(w, id, 0xc);
        blend_t(w, id, 5, 10);
    }
}

/// State 0xc (module doc).
fn bite(w: &mut World, id: MobyId) {
    let t = tgt_pos(w, id);
    let me = c::pos(w, id);
    let h = c::atan(t[0] - me[0], t[1] - me[1]);
    let up = rows(w, id)[2];
    orient(w, id, up, [h.cos(), h.sin(), 0.0, 0.0]);
    let key = c::ground::key_time(w, id);
    let a = &w.m(id).anim;
    if a.seq_a == a.seq_b && (10.0..=15.0).contains(&key) && w.hero.state != 0x51 {
        let step = c::set_len3(rows(w, id)[0], 2.0 * c::pf(w, id, pv::SPEED) * c::DT);
        crawl(w, id, step, true);
        attack::joint_hit(w, f32::from_bits(0x3eaa_7efa), 1.0, id, 0, 1, 0, 1, 0);
        return;
    }
    if !done(w, id) || c::dist2(c::pos(w, id), t) <= 2.0 { return; }
    set_state(w, id, 9);
    blend_t(w, id, 4, 10);
}

/// `0x2e94f0`: the hits and the tail (module doc). True: deleted.
fn hits(w: &mut World, id: MobyId) -> bool {
    let up_z = w.m(id).rows[2][2];
    c::set_pf(w, id, pv::D + 0x10, up_z * 0.25);
    if let Some(pod) = link(w, id, pv::POD) {
        if let Some(h) = w.get_hit(pod, 0x33_0000, false) {
            if h.attacker.is_none_or(|a| w.m(a).o_class != 0xaf) {
                goo(w, id, [0.0; 4]);
                break_pod(w, id);
                set_death_bits(w, id, 0, -1);
                w.delete_moby(id);
                return true;
            }
        }
    }
    let hit = w.get_hit(id, 0x33_0000, false);
    let res = damage::resolve(w, id, hit, pv::D, 0, 4);
    if res.out5 != 1 && state(w, id) != 0xe {
        let hp = c::pf(w, id, pv::D) - res.damage;
        c::set_pf(w, id, pv::D, hp);
        let reaction = if hp <= 0.0 { 1 } else { res.reaction };
        if reaction == 1 || reaction == 2 {
            set_death_bits(w, id, 0, -1);
            let dir = hit.map_or([0.0; 4], |h| h.dir.map(|x| f32::from_bits(x.0)));
            goo(w, id, dir);
            if c::pu8(w, id, pv::K + knock::k::BURN) == 1 {
                burn_puffs(w, id);
                c::set_pu8(w, id, pv::K + knock::k::BURN, 0);
            }
            w.play_sound(3, 0, id);
            break_pod(w, id);
            let lives = c::pi16(w, id, pv::LIVES);
            if lives == 0 {
                w.delete_moby(id);
                return true;
            }
            c::set_pi16(w, id, pv::LIVES, lives - 1);
            let meter = c::pi16(w, id, pv::D + 4) as f32;
            c::set_pf(w, id, pv::D, meter);
            let home = c::pv4(w, id, pv::HOME);
            c::set_pos(w, id, home);
            hide(w, id);
        }
    }
    w.mm(id).hit_slot = 0xff;
    let ph = c::add_rot(c::pf(w, id, pv::GLOW), GLOW_RATE * DEG * c::DT);
    c::set_pf(w, id, pv::GLOW, ph);
    w.mm(id).glow = crate::particles::tween_color(((ph.sin() + 1.0) * 0.5).to_bits(), GLOW.0, GLOW.1);
    if c::pi32(w, id, pv::LURE) != 0 {
        let f = w.rng.randf(180.0, 240.0);
        let t = gscale(w, f) as i16;
        c::set_pi16(w, id, pv::ALERT, t);
        c::set_pi32(w, id, pv::LURE, 0);
    }
    c::dec_timer_pvar_s16(w, id, pv::ALERT);
    let sight = c::pf(w, id, pv::SIGHT_BASE) + if alerted(w, id) { 6.0 } else { 0.0 };
    c::set_pf(w, id, pv::SIGHT, sight);
    let forced = c::pi32(w, id, pv::FORCED);
    if let Some(f) = usize::try_from(forced).ok().filter(|&f| f < w.table.mobys.len()) {
        c::set_pi32(w, id, pv::KIND, 1);
        c::set_pi32(w, id, pv::TGT, f as i32 + 1);
        let p = w.m(f).position;
        c::set_pv4(w, id, pv::TGT_REC, p);
    } else if w.body() == 1 {
        if (w.counter % 8) as i16 == c::pi16(w, id, pv::PHASE) {
            let pick = gadgetbot(w, id, sight);
            c::set_pi32(w, id, pv::TGT, pick.map_or(0, |m| m as i32 + 1));
            match pick {
                None => c::set_pi32(w, id, pv::KIND, 2),
                Some(m) => {
                    let q = w.m(m).position;
                    let inside = usize::try_from(c::pi32(w, id, pv::REGION)).is_ok_and(|r| region::point_in_polygon(w, r, q));
                    let far_bot = w.m(m).o_class == GADGETBOT && 2.0 < c::dist2(c::pos(w, id), q);
                    if !inside || far_bot {
                        c::set_pi32(w, id, pv::TGT, 0);
                        c::set_pi32(w, id, pv::KIND, 2);
                    } else {
                        c::set_pi32(w, id, pv::KIND, 0);
                    }
                }
            }
        }
    } else {
        let reg = usize::try_from(c::pi32(w, id, pv::REGION)).ok();
        let t = target::acquire_in(w, id, sight, reg);
        c::set_pv4(w, id, pv::TGT_REC, t.pos);
        c::set_pi32(w, id, pv::TGT, t.moby.map_or(0, |m| m as i32 + 1));
        let mut kind = t.kind as i32;
        if kind != 2 {
            let home = c::pv4(w, id, pv::HOME);
            if sight < c::dist2(home, t.pos) || 3.0 < (c::pos(w, id)[2] - t.pos[2]).abs() { kind = 2; }
        }
        c::set_pi32(w, id, pv::KIND, kind);
    }
    if c::pi32(w, id, pv::TGT) == 0 {
        let h = w.hero_moby.map_or(0, |m| m as i32 + 1);
        c::set_pi32(w, id, pv::TGT, h);
    }
    false
}

/// Level06 `0x2e8678` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { return; }
    if w.svc.game_mode == 2 {
        if w.m(id).mode & mode::HIDDEN == 0 {
            c::set_pi16(w, id, pv::MODE2, 1);
            w.mm(id).mode |= mode::HIDDEN | mode::NO_ANIM;
        }
    } else if w.svc.game_mode == 0 && c::pi16(w, id, pv::MODE2) != 0 {
        c::set_pi16(w, id, pv::MODE2, 0);
        w.mm(id).mode &= !(mode::HIDDEN | mode::NO_ANIM);
    }
    if state(w, id) != 0x10 && (c::pi16(w, id, pv::SIDE) != 0) != w.svc.map.alt { return; }
    manip::big_head(w, 2.1, id, 1, id, pv::BIG_HEAD);
    if hits(w, id) { return; }
    if w.m(id).visible != 0 {
        let cam = w.camera.map(|x| f32::from_bits(x.0));
        if c::dist3(c::pos(w, id), cam) < 14.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0xb;
        }
    }
    match state(w, id) {
        0 => { init(w, id); }
        1 => {
            if c::dist2(c::pos(w, id), tgt_pos(w, id)) < 4.0 {
                w.mm(id).mode &= !(mode::HIDDEN | mode::NO_ANIM);
                break_pod(w, id);
                set_state(w, id, 8);
                blend_t(w, id, 4, 5);
            }
        }
        3 => {
            let d = c::sub(tgt_pos(w, id), c::pos(w, id));
            let n = probe(w, id);
            orient(w, id, n, d);
            let h = super::hero_pos(w);
            if c::dist2(h, c::pos(w, id)) < c::pf(w, id, pv::WAKE) {
                set_state(w, id, 4);
                blend_t(w, id, 4, 5);
            }
        }
        4 => entry(w, id),
        5 => {
            let vel = c::pv4(w, id, pv::K + knock::k::VEL);
            orient(w, id, [0.0, 0.0, 1.0, 0.0], vel);
            if knock::update(w, id, pv::K) & 1 != 0 { land(w, id); }
        }
        6 => {
            let r = knock::update(w, id, pv::K);
            if r & 1 != 0 {
                land(w, id);
            } else if r & 0x120 != 0 {
                set_death_bits(w, id, 0, -1);
                w.delete_moby(id);
            }
        }
        7 => {
            if done(w, id) {
                set_state(w, id, 8);
                blend_t(w, id, 4, 5);
            }
        }
        8 => wander(w, id),
        9 => chase(w, id, false),
        10 => chase(w, id, true),
        0xb => {
            if done(w, id) {
                set_state(w, id, 10);
                blend_t(w, id, 4, 20);
            }
        }
        0xc => bite(w, id),
        0xd => {
            let home = c::pv4(w, id, pv::HOME);
            let d = c::sub(home, c::pos(w, id));
            crawl_toward(w, id, d, 1.0, true);
            if c::dist2(c::pos(w, id), home) < 2.0 {
                set_state(w, id, 8);
                blend_t(w, id, 4, 10);
            } else if !no_target(w, id) {
                set_state(w, id, 9);
            } else if alerted(w, id) {
                set_state(w, id, 10);
            }
        }
        0x10 if react::carried(w, id, pv::K) != 0 => {
            set_state(w, id, 8);
            blend_t(w, id, 0, 10);
            c::set_pi16(w, id, pv::SUCK + 0x68, 0);
        }
        _ => {}
    }
}
