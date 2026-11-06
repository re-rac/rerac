//! **Batalia's anti-aircraft turret, class 440** (level08 `0x2e0328`, census U275, one placed: #339), the gunship
//! battle: Ratchet steps into its seat (hero state 0x32, the script camera behind the barrels), turns it with the
//! stick and fires the shells 458 (`turret_shell`, two barrels 566 in turn, homing on a missile in the sights) at the
//! four gunships of pvar +0x80.. (`batalia_gunship`), which close in and send missiles 435 (`batalia_bomber`) at it in
//! waves; each missile that reaches it costs 60 of its 200 health. The waves speed up as the gunships fall (the
//! missiles' speed +0x98: 10, 12.5, 15, 17.5; the wave timer 285 / 240 / 195 / 150 ticks). All four down: the mission's
//! end with the host NPC (#820, `batalia_story`'s 1283: its talk node 3); the health gone: Ratchet put by the host,
//! node 4. Once the mission is done the turret is free: Ratchet walks into its seat again and △ leaves it (during the battle Quit is the death sequence). The HUD (draw callback `0x2deee0`) shows the radar (missiles,
//! blinking while they dive, and the gunships), the health ring and the lock-on reticle. Read from the level08
//! decomp; the level data from the overlay (gp 0x166c00). Native `f32`.
//!
//! **Pvars** (0xb0): +0x40 (s16) the fire cooldown, +0x42 (s16) the re-mount cooldown, +0x48 / +0x4c the pitch / yaw
//! speeds, +0x52 (s16) the camera's start, +0x54 (s16) the barrel, +0x58 (s32) the health, +0x5c (s16) the wave timer,
//! +0x5e / +0x5f the gunship chosen to fire and its re-choice timer, +0x60.. eight missile slots (the gunships register
//! theirs; the port: moby index + 1), +0x80.. the four gunships, +0x90 / +0x94 the barrels (index + 1), +0x98 the
//! missiles' speed, +0x9c the host, +0xa0 the lock-on count, +0xa4 class 1808 (not in Batalia's class table: none),
//! +0xa8 its yaw offset. Moby +0xbc: 0 free, 1 manned, 2 just left (free again once Ratchet is 1 away, after `ticks(60)`).
//!
//! | address | what | port |
//! |---|---|---|
//! | top | the seat (position + 0.4 up); `FastDecTimer(+0x42)`: running, or not manned → the mount test: just left and Ratchet beyond 1 (xy) → free, +0x42 = `ticks(60)`; free, the mission done, Ratchet within 1 (xy), less than 7 above, +0x42 = 0, health left → manned, `SetState(0x32, 1)` | [`update`] (`cinematic::hero_state`) |
//! | manned | Ratchet in the seat (0.7 behind it, the turret's Euler), his motion cleared; health 0 → put by the host, node 4 (`0x306b30(host, 0)`), `CameraScript2(0)`, → 4; the mission open and Quit (0x14095f & 1) → the death sequence (`0x2319b0`); no △ / Quit, or the mission open → (first tick: +0x52 = 2, `CameraScript(position, Euler, 1, 0, 0)`) → 5, the gunships waiting (2) → 3; else the leave: Quit cleared, `SetState(0, 1)`, pitch 0, → just left, `HeroTeleport(rows·(−2.5, 0, 5.9) + position, yaw)` facing back, `CameraScript2(0)`, → 4 | [`manned`] |
//! | 0 | → 4; health 200, missile speed 10; `CreateMoby(0x710)` (none); pitch 0; +0x40 = `ticks(30)`, the speeds and slots cleared, the wave `ticks(180)`; +0xa8 = 0 while the mission is open, π/2 once done | [`update`] |
//! | 5 | the stick (y negated unless 0x15eddc): yaw speed `Approach`es −180°·dt·x (45° / 360° dt²), pitch speed −22.5°·dt·y (180° / 720°); Euler y clamped to [−45°, 20°]; the lock (`0x2e0228`: a registered missile within 4° of the aim from the camera) counts +0xa0 up to `ticks(30)` or down; ○ held (0x13cae0 & 0x20), +0x40 out: the shot (module doc of [`fire`]); +0x52 out → the camera's mode 1 (`0x313ba8`), targets rows·(4.07, 0, 6.5) + position and the Euler; the HUD; the wave (below) | [`fly_turret`] |
//! | wave | +0x5c out, no registered missile alive and not diving (2): the gunships down → speed / timer (0 → 10 / 285, 1 → 12.5 / 240, 2 → 15 / 195, 3 → 17.5 / 150); 4 and the mission open → the end (host node 3, +0xa8 = π/2); the chosen gunship kept while alive and +0x5f runs, else the live one nearest Ratchet's facing; it fires (its +0xbc = 1) | [`wave`] |
//! | tail | the barrels (`CreateMoby(0x236)`, 0xff / 0x7e, the turret's light) at rows·(3.035, ∓1.533, 5.262) + position with its Euler; one's recoil (sequence 1) over → sequence 0 | [`barrels`] |
//! | `0x2deee0` | ALPHA 0x44; the radar backdrop (±55 about (180, 140) from the centre, FX 16 + 0x28, ST 0..128, 0x60808080); the missiles (dead slots cleared; 0x60ffffff, diving 0x600000ff blinking off every 16 ticks; 5 px; distance ·7.5 in 1/16 px at the bearing from the turret's yaw, x mirrored with the mirror cheat; FX 6 ST 0..32); the gunships (0x600080ff, 8 px); the health backdrop about (−180, 140), its ring (radius 40, `FastTweenColor` of the health, the lost slices at alpha 0x20, FX 6) and the reticle (the lock-on colour; FX 0x11 turned by 4·yaw, FX 0x12 by 8·pitch; radius 40) | [`hud_frame`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::fx::{part27, polar};
use crate::moby_update::creature::{self as c, add, add_rot, atan, diff_rots, dist2, dist3, len3, set_len3, sub_rot, DT, DT2, SPEED, V};
use crate::moby_update::services::{HeroCall, HeroPose, World};
use crate::moby_update::story;
use super::trespasser_lock::RingPrim;

pub const REFERENCE_LEVEL: u32 = 8;
pub const UPDATE_FN: u32 = 0x2e_0328;
pub const CLASSES: [i16; 1] = [440];
pub const HUD_FN: u32 = 0x2d_eee0;

const BARREL: i16 = 0x236;
const PI: f32 = std::f32::consts::PI;
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const DEG: f32 = 0.017_453_292;

pub mod pvo {
    pub const FIRE_T: usize = 0x40;
    pub const MOUNT_T: usize = 0x42;
    pub const PITCH_V: usize = 0x48;
    pub const YAW_V: usize = 0x4c;
    pub const CAM_T: usize = 0x52;
    pub const BARREL: usize = 0x54;
    pub const HEALTH: usize = 0x58;
    pub const WAVE_T: usize = 0x5c;
    pub const CHOSEN: usize = 0x5e;
    pub const CHOSEN_T: usize = 0x5f;
    pub const MISSILES: usize = 0x60;
    pub const SHIPS: usize = 0x80;
    pub const BARRELS: usize = 0x90;
    pub const SPEED: usize = 0x98;
    pub const HOST: usize = 0x9c;
    pub const LOCK: usize = 0xa0;
    pub const DISH_YAW: usize = 0xa8;
    pub const LEN: usize = 0xb0;
}

/// The level data (gp−0x5170..−0x5014).
mod k {
    /// The leave's offset in the turret's frame (gp−0x5170).
    pub const EXIT: [f32; 3] = [-2.5, 0.0, 5.9];
    /// Degrees a second (yaw, pitch) and their accelerations: up 45 / 180, down 360 / 720.
    pub const YAW_SPEED: f32 = 180.0;
    pub const PITCH_SPEED: f32 = 22.5;
    pub const YAW_ACC: f32 = 45.0;
    pub const PITCH_ACC: f32 = 180.0;
    pub const YAW_DEC: f32 = 360.0;
    pub const PITCH_DEC: f32 = 720.0;
    pub const SHELL_SPEED: f32 = 180.0;
    /// The barrels and the muzzles in the turret's frame (gp−0x50f0.., −0x50d0..), the camera's (gp−0x50b0).
    pub const BARRELS: [[f32; 3]; 2] = [[3.035, -1.533, 5.262], [3.035, 1.533, 5.262]];
    pub const MUZZLES: [[f32; 3]; 2] = [[11.035, -1.533, 5.262], [11.035, 1.533, 5.262]];
    pub const CAMERA: [f32; 3] = [4.074, 0.0, 6.5];
    /// The HUD: the health panel and the radar about the screen centre, their half sizes, the ring and reticle radii,
    /// the blips' half sizes, the radar scale (1/16 px a unit), the FX textures and colours.
    pub const HEALTH_PANEL: [f32; 2] = [-180.0, 140.0];
    pub const RADAR: [f32; 2] = [180.0, 140.0];
    pub const HALF: [f32; 2] = [55.0, 55.0];
    pub const RING_R: [f32; 2] = [40.0, 40.0];
    pub const RETICLE_R: [f32; 2] = [40.0, 40.0];
    pub const MISSILE_HALF: [f32; 2] = [5.0, 5.0];
    pub const SHIP_HALF: [f32; 2] = [8.0, 8.0];
    pub const RADAR_SCALE: f32 = 7.5;
    pub const FX_PANEL: usize = 0x10 + 0x28;
    pub const FX_DOT: usize = 6;
    pub const FX_RETICLE_YAW: usize = 0x11;
    pub const FX_RETICLE_PITCH: usize = 0x12;
    pub const PANEL_RGBA: u32 = 0x6080_8080;
    pub const MISSILE_RGBA: u32 = 0x60ff_ffff;
    pub const DIVING_RGBA: u32 = 0x6000_00ff;
    pub const SHIP_RGBA: u32 = 0x6000_80ff;
}

fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn ship(w: &World, id: MobyId, i: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, pvo::SHIPS + 4 * i)).ok().filter(|&m| m < w.table.mobys.len()) }
fn alive(w: &World, m: MobyId) -> bool { w.m(m).state < 0x80 }
fn mission_open(w: &World, id: MobyId) -> bool { !story::mission_done(w, w.m(id).mission as i32) }
fn local(w: &World, id: MobyId, o: [f32; 3]) -> V {
    let m = w.m(id);
    let r = &m.rows;
    std::array::from_fn(|l| if l == 3 { m.position[3] } else { r[0][l] * o[0] + r[1][l] * o[1] + r[2][l] * o[2] + m.position[l] })
}

/// A missile a gunship sends at the turret registers into its first free slot of eight (the gunships' `0x2e24e8`..).
pub fn register_missile(w: &mut World, turret: MobyId, missile: MobyId) {
    if w.m(turret).pvars.len() < pvo::LEN { return; }
    for i in 0..8 {
        let o = pvo::MISSILES + 4 * i;
        if c::pi32(w, turret, o) == 0 {
            c::set_pi32(w, turret, o, missile as i32 + 1);
            return;
        }
    }
}

/// The missiles' speed (+0x98) and the hit (+0x58 −= 60), for `batalia_bomber`.
pub fn missile_speed(w: &World, turret: MobyId) -> f32 { if w.m(turret).pvars.len() < pvo::LEN { 0.0 } else { c::pf(w, turret, pvo::SPEED) } }
pub fn hit(w: &mut World, turret: MobyId) {
    if w.m(turret).pvars.len() < pvo::LEN { return; }
    let h = c::pi32(w, turret, pvo::HEALTH) - 0x3c;
    c::set_pi32(w, turret, pvo::HEALTH, h);
}

/// Level08 `0x2e0328` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, pvo::LEN);
    let p = c::pos(w, id);
    let seat = [p[0], p[1], p[2] + 0.4, p[3]];
    let fired = c::dec_timer_pvar_s16(w, id, pvo::MOUNT_T) != 0;
    if fired && w.m(id).cmd == 1 {
        if manned(w, id, seat) { return; }
    } else {
        mount_test(w, id, seat);
    }
    match w.m(id).state {
        0 => init(w, id),
        5 => {
            if fly_turret(w, id) { return; }
        }
        6 => {
            for t in [link(w, id, pvo::BARRELS), link(w, id, pvo::BARRELS + 4)].into_iter().flatten() { w.delete_moby(t); }
            w.delete_moby(id);
            return;
        }
        _ => {}
    }
    barrels(w, id);
}

fn init(w: &mut World, id: MobyId) {
    w.mm(id).state = 4;
    c::set_pi32(w, id, pvo::HEALTH, 200);
    c::set_pf(w, id, pvo::SPEED, 10.0);
    w.mm(id).rotation[1] = 0.0;
    let t30 = w.ticks(0x1e) as i16;
    c::set_pi16(w, id, pvo::FIRE_T, t30);
    c::set_pi16(w, id, pvo::MOUNT_T, 0);
    c::set_pf(w, id, pvo::PITCH_V, 0.0);
    c::set_pf(w, id, pvo::YAW_V, 0.0);
    for i in 0..8 { c::set_pi32(w, id, pvo::MISSILES + 4 * i, 0); }
    let t180 = w.ticks(0xb4) as i16;
    c::set_pi16(w, id, pvo::WAVE_T, t180);
    c::set_pu8(w, id, pvo::CHOSEN, 0);
    c::set_pu8(w, id, pvo::CHOSEN_T, 0);
    let dish = if mission_open(w, id) { 0.0 } else { HALF_PI };
    c::set_pf(w, id, pvo::DISH_YAW, dish);
}

/// The mount test (module doc).
fn mount_test(w: &mut World, id: MobyId, seat: V) {
    let Some(h) = w.hero_moby else { return };
    let hp = w.m(h).position;
    if w.m(id).cmd == 2 {
        if 1.0 < dist2(seat, hp) {
            w.mm(id).cmd = 0;
            let t = w.ticks(0x3c) as i16;
            c::set_pi16(w, id, pvo::MOUNT_T, t);
        }
        return;
    }
    if mission_open(w, id) { return; }
    let feet = super::hero_pos(w);
    if dist2(seat, hp) < 1.0 && feet[2] - w.m(id).position[2] < 7.0 && c::pi16(w, id, pvo::MOUNT_T) == 0 && 0 < c::pi32(w, id, pvo::HEALTH) {
        w.mm(id).cmd = 1;
        crate::cinematic::hero_state(w, 0x32, true);
    }
}

/// Ratchet put by the host (#820) facing back, the host's talk node (`0x306b30`).
fn to_host(w: &mut World, id: MobyId, node_ok: bool) {
    w.mm(id).rotation[1] = 0.0;
    w.mm(id).cmd = 2;
    crate::cinematic::camera_script2(w, 0);
    w.mm(id).state = 4;
    let Some(host) = usize::try_from(c::pi32(w, id, pvo::HOST)).ok().filter(|&m| m < w.table.mobys.len()) else { return };
    let r0 = w.m(host).rows[0];
    let hp = w.m(host).position;
    let off = set_len3(r0, 2.5);
    let at = add(off, hp);
    let yaw = add_rot(w.m(host).rotation[2], PI);
    crate::cinematic::hero_teleport(w, [at[0], at[1], at[2]], [0.0, 0.0, yaw], 0, true);
    super::batalia_story::host_talk(w, host, node_ok);
}

/// The manned part (module doc); true when the update ends there.
fn manned(w: &mut World, id: MobyId, seat: V) -> bool {
    let rot = w.m(id).rotation;
    let back = [rot[2].cos() * -0.7, rot[2].sin() * -0.7];
    {
        let f = w.hero_fields_mut();
        f.pose = Some(HeroPose { pos: [seat[0] + back[0], seat[1] + back[1], seat[2]], yaw: rot[2], target_yaw: rot[2] });
        f.clear_motion = true;
    }
    if c::pi32(w, id, pvo::HEALTH) == 0 {
        to_host(w, id, false);
        return true;
    }
    let open = mission_open(w, id);
    // Quit during the battle (the Water Worker's mission still open) is the death sequence; once it is done the seat
    // is free: Ratchet walks in again (the mount test) and △ / Quit put him back down.
    if open && w.svc.vehicle.quit & 1 != 0 { w.hero_fields_mut().call(HeroCall::Death); }
    let leave = w.hero.loop_in.pad.pressed & crate::pad::button::TRIANGLE != 0 || w.svc.vehicle.quit & 1 != 0;
    if !leave || open {
        if w.m(id).state != 5 {
            c::set_pi16(w, id, pvo::CAM_T, 2);
            let p = c::pos(w, id);
            crate::cinematic::camera_script(w, [p[0], p[1], p[2]], [rot[0], rot[1], rot[2]], 1, 0, false);
        }
        w.mm(id).state = 5;
        for i in 0..4 {
            if let Some(s) = ship(w, id, i) {
                if w.m(s).state == 2 { w.mm(s).state = 3; }
            }
        }
        return false;
    }
    w.svc.vehicle.quit = 0;
    crate::cinematic::hero_state(w, 0, true);
    w.mm(id).rotation[1] = 0.0;
    w.mm(id).cmd = 2;
    let at = local(w, id, k::EXIT);
    // `HeroTeleport(…, yaw)` then 0x13f3e8 += π: put down facing away from the turret.
    let back = add_rot(w.m(id).rotation[2], PI);
    crate::cinematic::hero_teleport(w, [at[0], at[1], at[2]], [0.0, 0.0, back], 0, true);
    crate::cinematic::camera_script2(w, 0);
    w.mm(id).state = 4;
    false
}

/// `Approach` of a speed toward `target`: faster while it speeds up in the same direction (`acc`), else `dec`.
fn speed_toward(v: &mut f32, target: f32, acc: f32, dec: f32) {
    let x = *v;
    let rate = if (0.0 <= x && x < target) || (x <= 0.0 && target < x) { acc } else { dec };
    c::turn::approach(target, rate * DEG * DT2, v);
}

/// `0x2e0228`: a registered missile within 4° of the aim, seen from the camera.
fn locked(w: &World, id: MobyId) -> Option<MobyId> {
    let rot = w.m(id).rotation;
    let aim = polar(1.0, rot[2], -rot[1]);
    let cam = w.camera.map(|x| x.to_f32());
    let cone = f32::from_bits(0x3d8e_fa35).cos();
    (0..8).filter_map(|i| link(w, id, pvo::MISSILES + 4 * i)).find(|&m| {
        let d = set_len3(c::sub(w.m(m).position, cam), 1.0);
        cone < c::dot3(d, aim)
    })
}

/// State 5 (module doc); true when the battle ended (the update ends there).
fn fly_turret(w: &mut World, id: MobyId) -> bool {
    if c::pi32(w, id, pvo::HEALTH) < 0 { c::set_pi32(w, id, pvo::HEALTH, 0); }
    let [sx, sy] = w.hero.stick.map(|x| f32::from_bits(x.0));
    let sy = if w.svc.help.cam_pitch_word == 0 { -sy } else { sy };
    let l = (sx * sx + sy * sy).sqrt();
    let (sx, sy) = if 1.0 < l { (sx / l, sy / l) } else { (sx, sy) };
    let mut yv = c::pf(w, id, pvo::YAW_V);
    speed_toward(&mut yv, k::YAW_SPEED * DEG * DT * -sx, k::YAW_ACC, k::YAW_DEC);
    c::set_pf(w, id, pvo::YAW_V, yv);
    let mut pv_ = c::pf(w, id, pvo::PITCH_V);
    speed_toward(&mut pv_, -(k::PITCH_SPEED * DEG * DT) * sy, k::PITCH_ACC, k::PITCH_DEC);
    c::set_pf(w, id, pvo::PITCH_V, pv_);
    let rz = add_rot(w.m(id).rotation[2], yv);
    let mut ry = add_rot(w.m(id).rotation[1], pv_);
    if ry < -std::f32::consts::FRAC_PI_4 {
        ry = f32::from_bits(0xbf49_0fdb);
    } else if 0.349_065_84 < ry {
        ry = f32::from_bits(0x3eb2_b8c2);
    }
    w.mm(id).rotation[2] = rz;
    w.mm(id).rotation[1] = ry;
    let lock = locked(w, id);
    let n = c::pi32(w, id, pvo::LOCK);
    if lock.is_some() {
        if n < w.ticks(0x1e) { c::set_pi32(w, id, pvo::LOCK, n + 1); }
    } else if n != 0 {
        c::set_pi32(w, id, pvo::LOCK, n - 1);
    }
    c::dec_timer_pvar_s16(w, id, pvo::FIRE_T);
    if w.hero.loop_in.pad.held & 0x20 != 0 && c::pi16(w, id, pvo::FIRE_T) == 0 { fire(w, id, lock); }
    if c::dec_timer_pvar_s16(w, id, pvo::CAM_T) != 0 {
        let p = local(w, id, k::CAMERA);
        let r = w.m(id).rotation;
        crate::cinematic::camera_targets(w, Some([p[0], p[1], p[2]]), Some([r[0], r[1], r[2]]));
    }
    if let Some(i) = super::row(REFERENCE_LEVEL, HUD_FN) { w.svc.draw_callbacks.register2(Callback::UnitFrame(i), id); }
    wave(w, id)
}

/// A shot: the barrel by +0x54, the shell, the shakes, the sparks and the flash (as Hoven's turret's).
fn fire(w: &mut World, id: MobyId, lock: Option<MobyId>) {
    let rot = w.m(id).rotation;
    let vel = polar(k::SHELL_SPEED * DT, rot[2], -rot[1]);
    let first = c::pi16(w, id, pvo::BARREL) == 1;
    let (barrel, muzzle, target) = if first { (0, 0, lock) } else { (1, 1, locked(w, id)) };
    if let Some(b) = link(w, id, pvo::BARRELS + 4 * barrel) { w.anim_blend(b, 1, 0, 0); }
    let p = local(w, id, k::MUZZLES[muzzle]);
    super::turret_shell::spawn(w, Some(id), p, vel, target);
    w.play_sound(0, 0, id);
    let (t10, t7) = (w.ticks(10), w.ticks(7));
    use crate::follow_camera::{ShakeAxis, ShakeRequest};
    w.shake_camera(ShakeRequest { axis: ShakeAxis::Up, amp: f32::from_bits(0x3cf5_c28f), ticks: t10 });
    w.shake_camera(ShakeRequest { axis: ShakeAxis::Forward, amp: f32::from_bits(0x3cf5_c28f), ticks: t7 });
    c::set_pi16(w, id, pvo::BARREL, if first { 0 } else { 1 });
    let t20 = w.ticks(20) as i16;
    c::set_pi16(w, id, pvo::FIRE_T, t20);
    let speed = len3(vel);
    for _ in 0..5 {
        let x = w.rng.randf(-1.0, 1.0);
        let y = w.rng.randf(-1.0, 1.0);
        let z = w.rng.randf(-1.0, 1.0);
        let r = set_len3([x, y, z, 0.0], speed * (SPEED * -0.35 + 1.0));
        let r = add([r[0], r[1], r[2], 0.0], vel);
        let s = w.rng.randf(DT * 3.0, DT * 6.0);
        let r = set_len3(r, s);
        let (t5, t10) = (w.ticks(5), w.ticks(10));
        let life = w.rng.rand_range(t5, t10);
        part27(w, 50000.0, p, r, 0x5f2f_4f6f, life);
    }
    let v = set_len3(vel, DT * 0.5);
    let (t4, t7) = (w.ticks(4), w.ticks(7));
    let life = w.rng.rand_range(t4, t7);
    part27(w, 500_000.0, p, v, 0x2f4f_7f7f, life);
}

/// The wave (module doc); true when the battle was won.
fn wave(w: &mut World, id: MobyId) -> bool {
    if c::dec_timer_pvar_s16(w, id, pvo::WAVE_T) == 0 { return false; }
    let busy = (0..8).filter_map(|i| link(w, id, pvo::MISSILES + 4 * i)).any(|m| alive(w, m) && w.m(m).state != 2);
    if busy { return false; }
    let down = (0..4).filter(|&i| ship(w, id, i).is_none_or(|s| !alive(w, s))).count();
    let (speed, t) = match down {
        0 => (10.0, 0x11d),
        1 => (12.5, 0xf0),
        2 => (15.0, 0xc3),
        3 => (17.5, 0x96),
        _ => {
            if mission_open(w, id) {
                to_host(w, id, true);
                c::set_pf(w, id, pvo::DISH_YAW, HALF_PI);
                return true;
            }
            return false;
        }
    };
    c::set_pf(w, id, pvo::SPEED, speed);
    let t = w.ticks(t) as i16;
    c::set_pi16(w, id, pvo::WAVE_T, t);
    let mut timer = c::pu8(w, id, pvo::CHOSEN_T);
    let out = crate::moby_update::services::fast_dec_timer_u8(&mut timer) != 0;
    c::set_pu8(w, id, pvo::CHOSEN_T, timer);
    let chosen = ship(w, id, c::pu8(w, id, pvo::CHOSEN) as usize);
    let pick = if !out && chosen.is_some_and(|s| alive(w, s)) {
        c::pu8(w, id, pvo::CHOSEN) as usize
    } else {
        c::set_pu8(w, id, pvo::CHOSEN_T, 1);
        let h = super::hero_pos(w);
        let facing = f32::from_bits(w.hero.rot[2].0);
        let mut best = (PI, 0xff);
        for i in 0..4 {
            let Some(s) = ship(w, id, i) else { continue };
            let p = w.m(s).position;
            let d = diff_rots(atan(p[0] - h[0], p[1] - h[1]), facing);
            if d < best.0 && alive(w, s) { best = (d, i); }
        }
        c::set_pu8(w, id, pvo::CHOSEN, best.1 as u8);
        best.1
    };
    if let Some(s) = ship(w, id, pick) { w.mm(s).cmd = 1; }
    false
}

/// The barrels (module doc).
fn barrels(w: &mut World, id: MobyId) {
    let rot = w.m(id).rotation;
    let (light, ambient) = (w.m(id).light, w.m(id).ambient);
    for k in 0..2 {
        let at = local(w, id, k::BARRELS[k]);
        match link(w, id, pvo::BARRELS + 4 * k) {
            None => {
                let Some(b) = w.create_moby(BARREL) else { continue };
                {
                    let m = w.mm(b);
                    m.update_dist = 0xff;
                    m.draw_dist = 0x7e;
                    m.visible = 1;
                    m.light = light;
                    m.ambient = ambient;
                    m.position = at;
                    m.rotation = rot;
                }
                w.build_matrix(b);
                c::set_pi32(w, id, pvo::BARRELS + 4 * k, b as i32 + 1);
            }
            Some(b) => {
                {
                    let m = w.mm(b);
                    m.position = at;
                    m.rotation = rot;
                }
                if w.m(b).anim.flags & 2 != 0 && w.m(b).anim.seq_b == 1 { w.anim_blend(b, 0, 0, 0); }
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------
// The HUD (`0x2deee0`).

/// A screen point from centre-relative GS offsets: x truncated to whole pixels, y to 1/16 pixel.
fn at(x: f32, y: f32) -> [f32; 2] {
    let c0 = super::trespasser_lock::CENTRE;
    [c0[0] + (x as i32) as f32, c0[1] + ((y * 16.0) as i32) as f32 / 16.0]
}
/// A point given in 1/16 pixel on both axes.
fn at16(x16: f32, y16: f32) -> [f32; 2] {
    let c0 = super::trespasser_lock::CENTRE;
    [c0[0] + (x16 as i32) as f32 / 16.0, c0[1] + (y16 as i32) as f32 / 16.0]
}
fn tween(f: f32, a: u32, b: u32) -> u32 { crate::particles::tween_color(f.to_bits(), a, b) }

fn panel(out: &mut Vec<RingPrim>, [px, py]: [f32; 2]) {
    let [hx, hy] = k::HALF;
    let uv = [[0.0, 0.0], [0.0, 128.0], [128.0, 0.0], [128.0, 128.0]];
    for q in 0..4 {
        let a = if q & 1 == 0 { 1.0 } else { -1.0 };
        let b = if q < 2 { 1.0 } else { -1.0 };
        let pos = [at(px + hx * a, py + hy * b), at(px + hx * a, py), at(px, py + hy * b), at(px, py)];
        out.push(RingPrim { fx: k::FX_PANEL, pos, uv, rgba: k::PANEL_RGBA, repeat: false });
    }
}

/// A radar blip `d` units away at bearing `rel` from the turret's yaw (x mirrored by `sx`).
fn blip(out: &mut Vec<RingPrim>, rel: f32, d: f32, sx: f32, [hx, hy]: [f32; 2], rgba: u32) {
    let [px, py] = k::RADAR;
    let r = d * k::RADAR_SCALE;
    let (ox, oy) = (-sx * rel.sin() * r, -rel.cos() * r);
    let p = |a: f32, b: f32| at16((px + hx * a) * 16.0 + ox, (py + hy * b) * 16.0 + oy);
    let uv = [[0.0, 0.0], [0.0, 32.0], [32.0, 0.0], [32.0, 32.0]];
    out.push(RingPrim { fx: k::FX_DOT, pos: [p(1.0, 1.0), p(1.0, -1.0), p(-1.0, 1.0), p(-1.0, -1.0)], uv, rgba, repeat: false });
}

/// `0x2deee0`, run by the frame's callbacks: the HUD into `DrawCallbacks::rings` (module doc).
pub fn hud_frame(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pvo::LEN { return; }
    let mut out = Vec::new();
    panel(&mut out, k::RADAR);
    let me = c::pos(w, id);
    let yaw = w.m(id).rotation[2];
    let mirror = if w.svc.cheats.on(crate::cheats::slot::MIRROR) { -1.0 } else { 1.0 };
    for i in 0..8 {
        let o = pvo::MISSILES + 4 * i;
        let Some(m) = link(w, id, o) else { continue };
        if !alive(w, m) {
            c::set_pi32(w, id, o, 0);
            continue;
        }
        let mut rgba = k::MISSILE_RGBA;
        if w.m(m).state == 2 {
            rgba = k::DIVING_RGBA;
            if w.counter & 0x10 == 0 { rgba = 0x6000_0000; }
        }
        let p = w.m(m).position;
        let rel = sub_rot(atan(p[0] - me[0], p[1] - me[1]), yaw);
        blip(&mut out, rel, dist3(p, me), mirror, k::MISSILE_HALF, rgba);
    }
    for i in 0..4 {
        let Some(s) = ship(w, id, i).filter(|&s| w.m(s).state < 0x7f) else { continue };
        let p = w.m(s).position;
        let rel = sub_rot(atan(p[0] - me[0], p[1] - me[1]), yaw);
        blip(&mut out, rel, dist3(p, me), 1.0, k::SHIP_HALF, k::SHIP_RGBA);
    }
    panel(&mut out, k::HEALTH_PANEL);
    let [px, py] = k::HEALTH_PANEL;
    let hp = c::pi32(w, id, pvo::HEALTH) as f32;
    let (f, ca, cb) = if 100.0 < hp { (hp - 100.0, 0x6000_ffff, 0x6000_ff00) } else { (hp, 0x6000_00ff, 0x6000_ffff) };
    let colour = tween(f / 100.0, ca, cb);
    let dim = ((1.0 - hp / 200.0) * 32.0) as i32;
    let slice = f32::from_bits(0x3e49_0fdb);
    let [rx, ry] = k::RING_R;
    let uv_ring = [[22.0, 22.0], [10.0, 10.0], [22.0, 22.0], [0.0, 0.0]];
    for i in 0..32 {
        let rgba = if i < dim { (colour & 0xff_ffff).wrapping_add(0x2000_0000) } else { colour };
        let a0 = add_rot(i as f32 * slice, 0.0);
        let a1 = add_rot(a0, slice);
        let (x0, y0) = (((rx * a0.sin()) as i32) as f32, ((ry * -a0.cos()) as i32) as f32);
        let (x1, y1) = (((rx * a1.sin()) as i32) as f32, ((ry * -a1.cos()) as i32) as f32);
        let pos = [at(px + x1, py + y1), at(px, py), at(px + x0, py + y0), at(px, py)];
        out.push(RingPrim { fx: k::FX_DOT, pos, uv: uv_ring, rgba, repeat: false });
    }
    let n = c::pi32(w, id, pvo::LOCK);
    let t15 = w.ticks(15);
    let rgba = if t15 < n { tween((n - t15) as f32 / t15 as f32, 0x5000_ffff, 0x5000_00ff) } else { tween(n as f32 / t15 as f32, 0x5000_ff00, 0x5000_ffff) };
    let uv_ret = [[0.0, 0.0], [0.0, 64.0], [64.0, 0.0], [64.0, 64.0]];
    let [qx, qy] = k::RETICLE_R;
    let corners = |base: f32| -> [[f32; 2]; 4] {
        [0.0, HALF_PI, -HALF_PI, PI].map(|o| {
            let a = add_rot(add_rot(base, o), 0.0);
            at16(a.cos() * qx * 16.0, a.sin() * qy * 16.0)
        })
    };
    out.push(RingPrim { fx: k::FX_RETICLE_YAW, pos: corners(yaw * 4.0), uv: uv_ret, rgba, repeat: false });
    let pitch8 = w.m(id).rotation[1] * 8.0;
    out.push(RingPrim { fx: k::FX_RETICLE_PITCH, pos: corners(pitch8), uv: uv_ret, rgba, repeat: false });
    w.svc.draw_callbacks.rings.extend(out);
}
