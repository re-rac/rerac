//! U103 (census 2026-10-02): class 713, Aridia's launch tube (level02 0x2ddc88, the only copy: 1 placed), its two child
//! rings 1057 / 1058 (no update of their own: `0x2df0b0` / `0x2def80` move them) and its four doors (the moby indices at
//! pvar +0x70..+0x7c). Read from the level02 decomp and disassembly. Native `f32`.
//!
//! Its pad rises at whichever end (cuboid A +0x60, cuboid B +0x64) Ratchet comes near. Once its mission is done its doors
//! slide open. When Ratchet stands on the pad he walks to its centre; then he is held (state 0x72), the pad sinks, he is
//! moved to the other end and the camera flies along the path (+0x68) from end to end, looking at the pad, while the
//! pad rises there; his control comes back and, if he has not seen it for a while, a help message (2013) shows.
//!
//! **Pvars**: +0x08 0x20 (the platform block's offset), +0x20 the platform block, +0x60 / +0x64 cuboids A / B, +0x68 the
//! path, +0x70..+0x7c the doors, +0x80 s32 the door timer, +0x84 / +0x88 the rings (the port: index + 1), +0x8c / +0x90
//! their spring velocities, +0x94 / +0x98 their heights over the pad, +0x9c the pad's vertical speed, +0xa0 s32 the
//! ride's end (0 or the last point), +0xa4 / +0xa8 the ride's place on the path and its speed, +0xac s32 the walk's
//! time-out, +0xb0 / +0xb4 the ride's top speed (0.3·dt·points) and its acceleration (0.1·dt²·points); `cmd` (+0xbc):
//! the end it waits at (1: A).
//!
//! | address | what | port |
//! |---|---|---|
//! | every tick | gp−0x4fbc ≠ 0: debug particles along the path (0 on the disc) | n/a |
//! | 0 | update distance 0xff; `cmd` 1; at cuboid B's centre, 6 down; 1; the rings created (1057 / 1058: draw distance 0x40, drawn, the tube's light words and mode, its position and Euler); their heights −1 / −1; the ride's speeds; `0x2df0b0` | [`update`], [`spawn_rings`], [`place_rings`] |
//! | 1 | the mission done → door timer `ticks(30)`, 2 | [`update`] |
//! | 2 | the doors slide ∓ 5·dt along their row 0 (`MobyBuildMatrix` each); the timer out → 3, class sound 1 | [`update`] |
//! | 3 | z springs to cuboid A's height (`0x25df98` = L01 0x270830: 16·dt², 16·dt², 8·dt; gp−0x4fcc / −0x4fd0); `0x2df0b0`; at it → 4, class sound 0 | [`update`] |
//! | 4 | the rings' heights 2 / −4; `0x2def80`; both within 0.0001 → 5 | [`update`], [`spring_rings`] |
//! | 5 | `cmd` 0 and Ratchet within 16 of cuboid B's centre (or `cmd` 1 and of A's): at that centre, 6 down, rings −1 / −1, `0x2df0b0`, `cmd` flipped, 3, class sound 1 | [`update`] |
//! | | else Ratchet on the pad (0x13f64c, air ticks 0, body 0): the ride's ends (from the far end to 0 or back), the camera from the ride's first point looking at the pad + 2 up, 6, time-out `ticks(180)`; the walk to the pad facing the far cuboid's yaw (`0x2373e0` = L01 0x249580); `CameraScript(the view, 3, ticks(240))`; its springs (0.02, 0.01, 0) ×2 (`0x2f8aa0`, gp−0x4fb8..); its targets | [`update`] (`HeroCall::WalkTo`, `cinematic`) |
//! | 6 | the walk (`0x237430` = L01 0x2495d0): standing → 7, class sound 2, `SetState(0x72, 0)`; walking → the time-out: out → `CameraScript2(2)`, 0xd, the walk cancelled (`0x237478`: `SetState(0 or 3, 1)` by his speed 0x13f4b0 < 2.7·dt); neither → `CameraScript2(2)`, 0xd | [`update`] |
//! | 7 | rings −1 / −1; `0x2def80`; both within 0.01 → 8 | [`update`] |
//! | 8 | z springs to cuboid A's height − 6; `0x2df0b0`; below it − 5.999: `cmd` flipped, at the other cuboid's centre (6 down), Ratchet moved by the centres' difference and `HeroTeleport(…, that cuboid's Euler, 0x72, 0)`, the vertical speed negated, 9, the camera's mode 2 for `ticks(180)` (`0x2f8a18`) | [`update`] (`cinematic::hero_teleport`, `script_mode`) |
//! | 9 | the ride springs to its end (+0xb4, +0xb4, +0xb0); at it → 10; the camera at the path's pose there (`0x264718` = L01 0x277d40, open), looking at the pad + 8 up | [`update`] (`path::pose`) |
//! | 10 | z springs to cuboid A's height; at it → 0xb, class sound 0 | [`update`] |
//! | 0xb | rings 2 / −4; both within 0.01: the help 2013 (record 0x48) when `ticks(0xe10)·60 < ticks(play) − 600·(move record 9's time)` and help record 72's mask lacks this level; 0xd; `CameraScript2(2)`; `SetState(0, 1)` | [`update`] (`hints::request`) |
//! | 0xc | z += 4·dt; at cuboid A's height → 5 | [`update`] |
//! | 0xd | Ratchet off the pad → 5 | [`update`] |
//! | 0xe..0x11 | rings re-created; the rings' test heights (gp−0x4fe0 / −0x4fdc, the gate gp−0x4fc0: 0 on the disc) | [`update`] |
//! | tail | `CarryRiders(+0x20, position − old (zero when longer than 4), old rotation, rotation)` | [`update`] |
//! | **0x2df0b0** | the rings at the pad, at its height + their heights, their velocities 0, their yaw = the pad's + (their height − the pad's)·(−56°) (gp−0x4fc4), `MobyBuildMatrix` | [`place_rings`] |
//! | **0x2def80** | the rings' heights spring to the pad's + theirs (16·dt², 16·dt², 8·dt), their yaw as above, `MobyBuildMatrix` | [`spring_rings`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, turn, DT, DT2};
use crate::moby_update::services::{HeroCall, World};
use rc_formats::volumes::{Shape, ShapeKind};

pub const REFERENCE_LEVEL: u32 = 2;
pub const UPDATE_FN: u32 = 0x2d_dc88;
pub const CLASSES: [i16; 1] = [713];
/// The two rings.
pub const RINGS: [i16; 2] = [1057, 1058];

const BLOCK: usize = 0x20;
const CUB_A: usize = 0x60;
const CUB_B: usize = 0x64;
const PATH: usize = 0x68;
const DOORS: usize = 0x70;
const DOOR_T: usize = 0x80;
const RING: usize = 0x84;
const RING_V: usize = 0x8c;
const RING_H: usize = 0x94;
const VZ: usize = 0x9c;
const END: usize = 0xa0;
const RIDE: usize = 0xa4;
const RIDE_V: usize = 0xa8;
const WALK_T: usize = 0xac;
const RIDE_MAX: usize = 0xb0;
const RIDE_ACC: usize = 0xb4;
const SIZE: usize = 0xb8;
/// gp−0x4fd0 / −0x4fcc / −0x4fc8 / −0x4fc4 (level02): the springs' top speed (·dt), acceleration and braking (·dt²),
/// the rings' twist per unit of height (degrees).
const MAX: f32 = 8.0;
const ACC: f32 = 16.0;
const BRAKE: f32 = 16.0;
const TWIST: f32 = -56.0;
const DEG: f32 = 0.017_453_292;
/// The held state.
const HELD: i32 = 0x72;
const HELP_MSG: i32 = 0x7dd;
const HELP_REC: i32 = 0x48;

fn cuboid(w: &World, id: MobyId, o: usize) -> Option<Shape> { w.svc.volumes.shape(ShapeKind::Cuboid, c::pi32(w, id, o)).copied() }
fn centre(s: &Shape) -> c::V { s.matrix[3] }

fn ring(w: &World, id: MobyId, k: usize) -> Option<MobyId> {
    let v = c::pi32(w, id, RING + 4 * k);
    usize::try_from(v - 1).ok().filter(|&m| v > 0 && m < w.table.mobys.len())
}

fn path(w: &World, id: MobyId) -> Vec<[f32; 4]> {
    usize::try_from(c::pi32(w, id, PATH)).ok().and_then(|i| w.svc.splines.get(i)).map(|s| s.iter().map(|q| q.map(f32::from_bits)).collect()).unwrap_or_default()
}

/// The pad's z spring toward `target` (0x25df98: 16·dt², 16·dt², 8·dt).
fn spring_z(w: &mut World, id: MobyId, target: f32) {
    let mut z = w.m(id).position[2];
    let mut v = c::pf(w, id, VZ);
    turn::spring(target, ACC * DT2, ACC * DT2, MAX * DT, &mut z, &mut v);
    w.mm(id).position[2] = z;
    c::set_pf(w, id, VZ, v);
}

/// A ring's yaw: the pad's + (its height − the pad's) · −56°.
fn twist(w: &mut World, id: MobyId, r: MobyId) {
    let a = c::add_rot(w.m(id).rotation[2], (w.m(r).position[2] - w.m(id).position[2]) * TWIST * DEG);
    w.mm(r).rotation[2] = a;
    w.build_matrix(r);
}

/// `0x2df0b0`: the rings placed at the pad (module table).
fn place_rings(w: &mut World, id: MobyId) {
    let pos = w.m(id).position;
    for k in 0..2 {
        let Some(r) = ring(w, id, k) else { continue };
        let h = c::pf(w, id, RING_H + 4 * k);
        let m = w.mm(r);
        m.position = pos;
        m.position[2] = pos[2] + h;
        c::set_pf(w, id, RING_V + 4 * k, 0.0);
    }
    for k in 0..2 {
        if let Some(r) = ring(w, id, k) { twist(w, id, r); }
    }
}

/// `0x2def80`: the rings spring to their heights (module table).
fn spring_rings(w: &mut World, id: MobyId) {
    let z = w.m(id).position[2];
    for k in 0..2 {
        let Some(r) = ring(w, id, k) else { continue };
        let target = z + c::pf(w, id, RING_H + 4 * k);
        let mut rz = w.m(r).position[2];
        let mut v = c::pf(w, id, RING_V + 4 * k);
        turn::spring(target, ACC * DT2, BRAKE * DT2, MAX * DT, &mut rz, &mut v);
        w.mm(r).position[2] = rz;
        c::set_pf(w, id, RING_V + 4 * k, v);
    }
    for k in 0..2 {
        if let Some(r) = ring(w, id, k) { twist(w, id, r); }
    }
}

fn set_heights(w: &mut World, id: MobyId, a: f32, b: f32) {
    c::set_pf(w, id, RING_H, a);
    c::set_pf(w, id, RING_H + 4, b);
}

/// Both rings within `eps` of the pad's height + (2, −4) (states 4 / 0xb) or + (−1, −1) (state 7).
fn rings_at(w: &World, id: MobyId, off: [f32; 2], eps: f32) -> bool {
    let z = w.m(id).position[2];
    (0..2).all(|k| ring(w, id, k).is_some_and(|r| ((w.m(r).position[2] - z) - off[k]).abs() < eps))
}

/// The rings 1057 / 1058 created at the tube (module table).
fn spawn_rings(w: &mut World, id: MobyId) {
    let (pos, rot, light, ambient, md) = { let m = w.m(id); (m.position, m.rotation, m.light, m.ambient, m.mode) };
    for (k, cl) in RINGS.into_iter().enumerate() {
        let r = w.create_moby(cl);
        if let Some(r) = r {
            let m = w.mm(r);
            m.draw_dist = 0x40;
            m.visible = 1;
            m.light = light;
            m.ambient = ambient;
            m.mode = md;
            m.position = pos;
            m.rotation = rot;
        }
        c::set_pi32(w, id, RING + 4 * k, r.map_or(0, |r| r as i32 + 1));
    }
}

/// The camera at `p` looking at `at`: Euler (0, −atan(|at − p|, at.z − p.z), atan(at − p)).
fn look(p: c::V, at: c::V) -> [f32; 3] {
    let yaw = c::atan(at[0] - p[0], at[1] - p[1]);
    let pitch = -c::atan(c::dist3(p, at), at[2] - p[2]);
    [0.0, pitch, yaw]
}

/// Moves the pad to cuboid `o`'s centre, 6 down.
fn to_end(w: &mut World, id: MobyId, o: usize) {
    if let Some(s) = cuboid(w, id, o) {
        let m = w.mm(id);
        m.position = centre(&s);
        m.position[2] -= 6.0;
    }
}

/// Level02 0x2ddc88 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < SIZE { return; }
    let (old, old_rot) = (w.m(id).position, w.m(id).rotation);
    let hero = crate::moby_update::classes::units::hero_pos(w);
    let a_z = cuboid(w, id, CUB_A).map_or(0.0, |s| centre(&s)[2]);
    match w.m(id).state {
        0 => {
            w.mm(id).update_dist = 0xff;
            w.mm(id).cmd = 1;
            to_end(w, id, CUB_B);
            w.mm(id).state = 1;
            spawn_rings(w, id);
            set_heights(w, id, -1.0, -1.0);
            let n = path(w, id).len() as f32;
            c::set_pf(w, id, RIDE_ACC, DT2 * 0.1 * n);
            c::set_pf(w, id, RIDE_MAX, DT * 0.3 * n);
            place_rings(w, id);
        }
        1 => {
            let level = w.svc.level;
            if w.mission_done(level, w.m(id).mission) == 0xff {
                let t = w.ticks(0x1e);
                c::set_pi32(w, id, DOOR_T, t);
                w.mm(id).state = 2;
            }
        }
        2 => {
            for k in 0..4 {
                let Some(d) = usize::try_from(c::pi32(w, id, DOORS + 4 * k)).ok().filter(|&d| d < w.table.mobys.len()) else { continue };
                let s = if k % 2 == 0 { -5.0 * DT } else { 5.0 * DT };
                let r0 = w.m(d).rows[0];
                let v = c::set_len3([r0[0], r0[1], r0[2], 0.0], s);
                let m = w.mm(d);
                for (p, d) in m.position.iter_mut().zip(v).take(3) { *p += d; }
            }
            for k in 0..4 {
                if let Some(d) = usize::try_from(c::pi32(w, id, DOORS + 4 * k)).ok().filter(|&d| d < w.table.mobys.len()) { w.build_matrix(d); }
            }
            let t = c::pi32(w, id, DOOR_T);
            let out = if t == 0 { true } else {
                c::set_pi32(w, id, DOOR_T, t.max(1) - 1);
                t.max(1) - 1 < 1
            };
            if out {
                w.mm(id).state = 3;
                w.play_sound(1, 0, id);
            }
        }
        3 => {
            spring_z(w, id, a_z);
            place_rings(w, id);
            if a_z - 0.0001 <= w.m(id).position[2] {
                w.mm(id).state = 4;
                w.play_sound(0, 0, id);
            }
        }
        4 => {
            set_heights(w, id, 2.0, -4.0);
            spring_rings(w, id);
            if rings_at(w, id, [2.0, -4.0], 0.0001) { w.mm(id).state = 5; }
        }
        5 => idle(w, id, hero),
        6 => {
            let walk = match w.hero.state { 0x65 | 0x66 => 1, 0x67 => 2, _ => 0 };
            match walk {
                2 => {
                    w.mm(id).state = 7;
                    w.play_sound(2, 0, id);
                    crate::cinematic::hero_state(w, HELD, false);
                }
                1 => {
                    let t = c::pi32(w, id, WALK_T);
                    let out = if t == 0 { true } else {
                        c::set_pi32(w, id, WALK_T, t.max(1) - 1);
                        t.max(1) - 1 < 1
                    };
                    if out {
                        crate::cinematic::camera_script2(w, 2);
                        w.mm(id).state = 0xd;
                        let st = if w.hero.eff_len.to_f32() < DT * 2.7 { 0 } else { 3 };
                        crate::cinematic::hero_state(w, st, true);
                    }
                }
                _ => {
                    crate::cinematic::camera_script2(w, 2);
                    w.mm(id).state = 0xd;
                }
            }
        }
        7 => {
            set_heights(w, id, -1.0, -1.0);
            spring_rings(w, id);
            if rings_at(w, id, [-1.0, -1.0], 0.01) { w.mm(id).state = 8; }
        }
        8 => {
            spring_z(w, id, a_z - 6.0);
            place_rings(w, id);
            if w.m(id).position[2] < a_z - 5.999 {
                let (to, from) = if w.m(id).cmd == 0 { (CUB_B, CUB_A) } else { (CUB_A, CUB_B) };
                w.mm(id).cmd ^= 1;
                let (Some(t), Some(f)) = (cuboid(w, id, to), cuboid(w, id, from)) else { return };
                w.mm(id).position = centre(&t);
                let d = c::sub(centre(&t), centre(&f));
                w.mm(id).position[2] -= 6.0;
                let p = [hero[0] + d[0], hero[1] + d[1], hero[2] + d[2]];
                crate::cinematic::hero_teleport(w, p, t.euler, HELD, false);
                let v = c::pf(w, id, VZ);
                c::set_pf(w, id, VZ, -v);
                w.mm(id).state = 9;
                let n = w.ticks(0xb4);
                crate::cinematic::script_mode(w, 2, n);
            }
        }
        9 => {
            let end = c::pi32(w, id, END) as f32;
            let mut t = c::pf(w, id, RIDE);
            let mut v = c::pf(w, id, RIDE_V);
            let acc = c::pf(w, id, RIDE_ACC);
            turn::spring(end, acc, acc, c::pf(w, id, RIDE_MAX), &mut t, &mut v);
            c::set_pf(w, id, RIDE, t);
            c::set_pf(w, id, RIDE_V, v);
            let done = if c::pi32(w, id, END) == 0 { t <= 0.0 } else { end <= t };
            if done { w.mm(id).state = 10; }
            let raw = usize::try_from(c::pi32(w, id, PATH)).ok().and_then(|i| w.svc.splines.get(i)).cloned().unwrap_or_default();
            let (p, _) = crate::path::pose(&raw, false, t, true);
            let pos = w.m(id).position;
            let at = [pos[0], pos[1], pos[2] + 8.0, pos[3]];
            crate::cinematic::camera_targets(w, Some([p[0], p[1], p[2]]), Some(look(p, at)));
        }
        10 => {
            spring_z(w, id, a_z);
            place_rings(w, id);
            if a_z <= w.m(id).position[2] {
                w.mm(id).state = 0xb;
                w.play_sound(0, 0, id);
            }
        }
        0xb => {
            set_heights(w, id, 2.0, -4.0);
            spring_rings(w, id);
            if rings_at(w, id, [2.0, -4.0], 0.01) {
                let play = crate::moby_update::creature::ticks(w, w.svc.help.play_time);
                let time = w.svc.help.records.moves[9].time as i32;
                let gap = ((w.ticks(0xe10) as f32) * 60.0) as i32;
                let lvl = w.svc.level;
                if gap < play - time * 600 && w.svc.help.records.help[72].mask & (1u32 << (lvl & 31)) == 0 {
                    crate::moby_update::classes::units::hints::request(w, HELP_MSG, HELP_REC);
                }
                w.mm(id).state = 0xd;
                crate::cinematic::camera_script2(w, 2);
                crate::cinematic::hero_state(w, 0, true);
            }
        }
        0xc => {
            w.mm(id).position[2] += DT * 4.0;
            if a_z <= w.m(id).position[2] { w.mm(id).state = 5; }
        }
        0xd => {
            if w.hero.ground_moby != Some(id) { w.mm(id).state = 5; }
        }
        0xe => {
            spawn_rings(w, id);
            w.mm(id).state = 0xf;
        }
        // 0xf..0x11: the rings' test heights behind the debug gate gp−0x4fc0 (0 on the disc): 0xf stays.
        0xf => {
            set_heights(w, id, 0.0, 0.0);
            spring_rings(w, id);
        }
        0x10 | 0x11 => w.mm(id).state = 0xf,
        _ => {}
    }
    let pos = w.m(id).position;
    let rot = w.m(id).rotation;
    let mut delta = [pos[0] - old[0], pos[1] - old[1], pos[2] - old[2], pos[3] - old[3]];
    if 4.0 < c::len3(delta) { delta = [0.0; 4]; }
    crate::moby_update::triggers::carry_riders(&mut w.mm(id).pvars, BLOCK, delta, old_rot, rot);
}

/// State 5 (module table).
fn idle(w: &mut World, id: MobyId, hero: c::V) {
    let near = |w: &World, o: usize| cuboid(w, id, o).is_some_and(|s| c::dist3(hero, centre(&s)) < 16.0);
    let cmd = w.m(id).cmd;
    let go = (cmd == 0 && near(w, CUB_B)) || (cmd == 1 && near(w, CUB_A));
    if go {
        to_end(w, id, if cmd == 0 { CUB_B } else { CUB_A });
        set_heights(w, id, -1.0, -1.0);
        place_rings(w, id);
        w.mm(id).state = 3;
        w.mm(id).cmd = cmd ^ 1;
        w.play_sound(1, 0, id);
        return;
    }
    let on = w.hero.ground_moby == Some(id) && w.hero.air_ticks == 0 && w.hero.mode == 0;
    if !on { return; }
    let pos = w.m(id).position;
    let at = [pos[0], pos[1], pos[2] + 2.0, pos[3]];
    let pts = path(w, id);
    let last = pts.len() as i32 - 1;
    let here = if cmd == 0 {
        c::set_pi32(w, id, END, 0);
        c::set_pf(w, id, RIDE, last as f32);
        cuboid(w, id, CUB_A)
    } else {
        c::set_pf(w, id, RIDE, 0.0);
        c::set_pi32(w, id, END, last);
        cuboid(w, id, CUB_B)
    };
    let i = (c::pf(w, id, RIDE) as i32).max(0) as usize;
    let p = pts.get(i).copied().unwrap_or([0.0; 4]);
    let e = look(p, at);
    w.mm(id).state = 6;
    let t = w.ticks(0xb4);
    c::set_pi32(w, id, WALK_T, t);
    let yaw = here.map_or(0.0, |s| s.euler[2]);
    w.hero_fields_mut().call(HeroCall::WalkTo { point: [pos[0], pos[1], pos[2]], yaw, release: 0 });
    let cam = w.camera.map(|x| f32::from_bits(x.0));
    let n = w.ticks(0xf0);
    crate::cinematic::camera_script(w, [cam[0], cam[1], cam[2]], [0.0; 3], 3, n, false);
    crate::cinematic::script_springs(w, [0.01, 0.02, 0.0, 0.01, 0.02, 0.0]);
    crate::cinematic::camera_targets(w, Some([p[0], p[1], p[2]]), Some(e));
}
