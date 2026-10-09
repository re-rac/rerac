//! The cutscene FX driver, class 1546: `CutsceneFxUpdate` level01 0x30c190. Spec: docs/plan/cutscenes.md §7.
//!
//! The mode-2 scenes carry no effects of their own (no particle or event track in the chunks,
//! docs/plan/cutscenes.md §7.1 "Scene data coverage"): the game adds them with this ordinary moby class, which the
//! moby loop keeps running during a scene (`CutsceneModeUpdate` 0x2aca80 runs `MobyUpdateLoop`). It reads the scene
//! id 0x16cd10, the scene tick 0x16cd14 and the actor slots 0x16ce58[k] ([`crate::scene_player::SceneState`]) and
//! spawns particles at the actors' joint points (`FUN_002645a8`). Novalis instance 976 (on the landing pad).
//!
//! **0** (load pass): state 1, update distance 0xff. **1**, only in game mode 2 (else nothing):
//! * scenes 1, 3, 4 (and 2 up to tick `ticks(148)`): `FUN_00278450(actor)` on the infobot (actor 3; actor 2 in
//!   scene 2): its thruster particles (class 750 only): two red-orange glow puffs and five white cores (type 23)
//!   behind its root joint, sized by that joint's animated scale ([`infobot_thrusters`]).
//! * scene 1, ticks `ticks(472)..=ticks(537)`: a type-46 particle at actor 0's joint list 1 (`randf(0.5, 2)`,
//!   `randi(2)` then `PartType46Spawn`, life `ticks(60)`); at `ticks(472)` exactly the water splash `FUN_002ff768(3,
//!   p)` (+0x23 = 0x70) and 16 type-35 drops (angle, `randf(0, 3·dt)`, `randf(3·dt, 6.5·dt)`, `rand_range(90, 120)`,
//!   `randi(2)`): `crate::particles::type46` / `type35` and the splash class 775 (`super::splash`).
//! * scene 4, actor 4 of class 531 (the courier ship): `RegisterDrawCallback(0x2a70a8, ship)` (the canopy
//!   glass's shine: `Callback::ShipGlass` with the matrix of the ship's joint list 0 (the canopy joint), drawn by `rc-engine`'s fx_draw) and, ticks `ticks(850)..=ticks(1160)`, two type-23 thruster puffs at each
//!   of its joint lists 1..6 (`PartType23Spawn(0.1, 1, 0.9, 200000, p, ±randi(16), dt, 0x7f204080)`, life
//!   `ticks(6)` (`ticks(30)` from `ticks(1100)` on)).
//! * scene 5 (the arrival), ticks < `ticks(120)`, actor 2 (the ship 530): the point of its joint list 5, the step
//!   since the last tick (from tick 2) and 0x162070 := the point; at tick `ticks(118)` **the crash explosion**:
//!   `SpawnBeamExplosion(0, 0, 5, 2.5, 9, 1, 60, ship, 0, p, 3, 200, 200, −1, 0, 60, −1, 0)` (60 Bomb Glove fireballs 122
//!   thrown out) and
//!   `SpawnBeamExplosion(0, 0, 1, 0.5, 9, 1, 60, ship, 0, p − (0, 0, 1), 300, 2, 2, −1, 0)`; then the ship's draw
//!   callback (the glass, as in scene 4) and its smoke trail: joint lists 1 and 2, stepped back by the tick's step and forward a
//!   quarter step four times, `FUN_00278810(150000, 60000, a, a, dt)` at each (two type-23 puffs of `randi(16)`
//!   spin and three of ±16, `randi(255)` each).
//!
//! The trail and the thruster puffs are type-23 particles (`crate::particles::type23`). The explosions are the
//! general `creature::fx::beam_explosion` with the ship actor's table moby as their moby (its flash shells need one):
//! streaks (15), sparks (11), puffs (8), the flash shells (class 112) and the explosion light. Native `f32`.

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::fx::{self, Beam};
use crate::scene_player::SceneActorState;
use crate::moby_update::creature::DT;
use crate::moby_update::services::World;

/// The update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x30c190;
/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [1546];

/// The infobot's class (`FUN_00278450` does nothing for any other).
pub const INFOBOT_CLASS: i16 = 750;
/// The courier ship of scene 4 (0x213).
pub const COURIER_SHIP_CLASS: i16 = 531;
/// The arrival scene and its crash tick (`ticks(0x76)`), and the end of its ship effects (`ticks(0x78)`).
pub const ARRIVAL_SCENE: usize = 5;
pub const CRASH_TICK: i32 = 0x76;
pub const ARRIVAL_FX_END: i32 = 0x78;

/// The first explosion of the crash (`SpawnBeamExplosion(0, 0, 5, 2.5, 9, 1, 60, …, 3, 200, 200, −1, 0, 60, −1, 0)`:
/// param_16 = 60 fireballs, the stack word `sw v0(0x3c), 0(sp)` at 0x30c6c4 that the decompiler drops; fixed
/// 2026-09-29, docs/plan/explosions.md §A).
pub const CRASH_BEAM: Beam = Beam { damage_r: 0.0, damage: 0.0, flash: 5.0, flash2: 2.5, flash_dist: 9.0, scale: 1.0, light: 60.0, streaks: 3, sparks: 200, puffs: 200, debris: 60, sound: -1, shake: false };
/// The second, one unit lower (`SpawnBeamExplosion(0, 0, 1, 0.5, 9, 1, 60, …, 300, 2, 2, −1, 0)`).
pub const CRASH_BEAM2: Beam = Beam { damage_r: 0.0, damage: 0.0, flash: 1.0, flash2: 0.5, flash_dist: 9.0, scale: 1.0, light: 60.0, streaks: 300, sparks: 2, puffs: 2, debris: 0, sound: -1, shake: false };

/// `CutsceneFxUpdate` 0x30c190 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.state = 1;
            m.update_dist = 0xff;
            return;
        }
        1 => {}
        _ => return,
    }
    if w.svc.game_mode != 2 { return; }
    let Some(scene) = w.svc.cinematic.scene.clone() else { return };
    let (sid, tick) = (scene.id, scene.tick);
    let actor = |k: usize| scene.actors.get(k);
    // The infobot's thrusters.
    if matches!(sid, 1 | 3 | 4) || (sid == 2 && tick <= w.ticks(0x94)) {
        let k = if sid == 2 { 2 } else { 3 };
        if let Some(a) = actor(k).filter(|a| a.o_class == INFOBOT_CLASS) { infobot_thrusters(w, a); }
    }
    if sid == 1 {
        if let Some(a) = actor(0) {
            if w.ticks(0x1d8) <= tick && tick <= w.ticks(0x219) {
                let p = a.joint_point(1);
                let size = w.rng.randf(0.5, 2.0);
                let spin = if w.rng.randi(2) != 0 { 2.0 } else { -2.0 };
                // PartType46Spawn(size, spin, p, 0x15f580 (zero), &p.z): the ring keeps to the point's height.
                let life = w.ticks(0x3c);
                ring46(w, size, spin, p, life);
            }
            if tick == w.ticks(0x1d8) {
                let p = a.joint_point(1);
                if let Some(m) = super::splash::spawn(w, 3.0, p) { w.mm(m).alpha = 0x70; }
                for _ in 0..16 {
                    let ang = w.rng.rand_angle();
                    let r = w.rng.randf(DT * 0.0, DT * 3.0);
                    let vz = w.rng.randf(DT * 3.0, DT * 6.5);
                    let life = w.rng.rand_range(0x5a, 0x78);
                    let kind = w.rng.randi(2);
                    drop35(w, p, [ang.cos() * r, ang.sin() * r, vz, 0.0], kind, life);
                }
            }
        }
    }
    if sid == 4 {
        if let Some(ship) = actor(4).filter(|a| a.o_class == COURIER_SHIP_CLASS) {
            w.svc.draw_callbacks.register_with_matrix(super::draw_callbacks::Callback::ShipGlass, ship.moby.unwrap_or(usize::MAX), ship.joint_matrix(0));
            if w.ticks(0x352) <= tick && tick <= w.ticks(0x488) {
                let life = if w.ticks(0x44c) <= tick { w.ticks(0x1e) } else { w.ticks(6) };
                for list in 1..7 {
                    let p = ship.joint_point(list);
                    for _ in 0..2 {
                        let r = w.rng.randi(0x10);
                        let spin = if w.rng.randi(2) == 0 { r } else { -r };
                        puff(w, [0.1, 1.0, 0.9, THRUSTER_SIZE], p, spin, THRUSTER_RGBA, life, None);
                    }
                }
            }
        }
    }
    if sid == ARRIVAL_SCENE && tick < w.ticks(ARRIVAL_FX_END) {
        let Some(ship) = actor(2) else { return };
        let p = ship.joint_point(5);
        // The tick's step of the point (from tick 2): the trail's spacing (its type-23 puffs are not drawn).
        let q = w.svc.cinematic.fx_ship_point;
        let step = if 1 < tick { [p[0] - q[0], p[1] - q[1], p[2] - q[2], p[3] - q[3]] } else { [0.0; 4] };
        w.svc.cinematic.fx_ship_point = p;
        if tick == w.ticks(CRASH_TICK) {
            // The ship actor is the explosions' moby (their flash shells need one; damage 0 and sound −1 use it not).
            fx::beam_explosion(w, &CRASH_BEAM, ship.moby, p);
            fx::beam_explosion(w, &CRASH_BEAM2, ship.moby, [p[0], p[1], p[2] - 1.0, p[3]]);
        }
        w.svc.draw_callbacks.register_with_matrix(super::draw_callbacks::Callback::ShipGlass, ship.moby.unwrap_or(usize::MAX), ship.joint_matrix(0));
        // The glowing trail from joint lists 1 and 2: from a step back, a quarter step forward four times.
        let back = |x: [f32; 4]| [x[0] - step[0], x[1] - step[1], x[2] - step[2], x[3] - step[3]];
        let (mut a, mut b) = (back(ship.joint_point(1)), back(ship.joint_point(2)));
        let q4 = step.map(|v| v * 0.25);
        for _ in 0..4 {
            trail(w, a);
            trail(w, b);
            for k in 0..4 {
                a[k] += q4[k];
                b[k] += q4[k];
            }
        }
    }
}

/// The level's scene hook, called by `CutsceneModeUpdate` between the frame's draw callbacks (`0x2ab920`) and the moby
/// loop (level01 `0x2525f0`, empty; level07 `0x265bd8`, level00 `0x23e268` with every level's part). Ported: the canopy
/// glass `RegisterDrawCallback(glass, actor)` (level00 `0x2935b8`, level07 `0x2ba260` = L01 `0x2a70a8`) on Veldin
/// (00) for every scene actor of class 530, on Umbris (07) for actor 4 of scene 4 when it is the ship 532 (the
/// escape in Qwark's ship). Not ported: Aridia's (02) mode |= 0x800 on the actors of class 0x1b1 (no reader of that
/// mode bit is known), Kerwan's (03) scene-5 particles, level 14's scene-0 call and level 15's scenes 3 / 4 call per
/// actor.
pub fn level_hook(w: &mut World) {
    if w.svc.game_mode != 2 { return; }
    let Some(scene) = w.svc.cinematic.scene.clone() else { return };
    let glass = |w: &mut World, a: &SceneActorState| {
        w.svc.draw_callbacks.register_with_matrix(super::draw_callbacks::Callback::ShipGlass, a.moby.unwrap_or(usize::MAX), a.joint_matrix(0));
    };
    match w.svc.level {
        0 => {
            for a in scene.actors.iter().filter(|a| a.o_class == 0x212) { glass(w, a); }
        }
        7 if scene.id == 4 => {
            if let Some(a) = scene.actors.get(4).filter(|a| a.moby.is_some() && a.o_class == 0x214) { glass(w, a); }
        }
        _ => {}
    }
}

/// `PartType46Spawn(size, spin, p, 0, &p.z)` with the caller's timer `life` (the scene-1 ripples). Without a particle
/// system its draw is made and the slot counted.
fn ring46(w: &mut World, size: f32, spin: f32, p: [f32; 4], life: i32) {
    *w.svc.fx.part_spawns.entry(46).or_default() += 1;
    let Some(sys) = w.particles.as_deref_mut() else {
        w.rng.randi(10);
        w.rng.randf(0.0, 256.0);
        return;
    };
    match crate::particles::type46::spawn(sys, w.rng, size, spin, p, [0.0; 4], p[2]) {
        Some(i) => crate::particles::rec::set_i16(&mut sys.pool.recs[i], 0xa, life as i16),
        None => w.svc.fx.part_failed += 1,
    }
}

/// `PartType35Spawn(p, vel, kind, life)` (the splash's drops).
fn drop35(w: &mut World, p: [f32; 4], vel: [f32; 4], kind: i32, life: i32) {
    *w.svc.fx.part_spawns.entry(35).or_default() += 1;
    let Some(sys) = w.particles.as_deref_mut() else {
        w.rng.randf(10500.0, 16800.0);
        w.rng.randf(0.0, 1.0);
        return;
    };
    if crate::particles::type35::spawn(sys, w.rng, p, vel, kind, life).is_none() { w.svc.fx.part_failed += 1; }
}

/// `FUN_00278450(infobot)` 0x278450: the infobot's two thrusters, from the last joint of its joint list 0 (`0x264508(m, 0, M)`): f =
/// min(|M.r0| / 1.026, 1) (the flame joint's animated scale), then 2 glow puffs `PartType23Spawn(0.01 + 0.02·f, 1,
/// 0.75, 6000 + 44000·f, M.r3 − unit(M.r2)·(0.025 + 0.025·f), ±randi(16), 0, 0x503030ff)` (life `ticks(6)`, phase 2
/// from 0x50) and 5 white cores `PartType23Spawn(0.01 + 0.015·f, 1, 1, 5000 + 25000·f, M.r3 − unit(M.r2)·(0.01 +
/// 0.015·f), 16, 0, 0x7fffffff)` (life `ticks(2)`, a `randi(0xff)` rotation, phase 2 from 0x7f).
pub(crate) fn infobot_thrusters(w: &mut World, a: &SceneActorState) {
    let m = a.joint_matrix(0);
    let len = (m[0][0] * m[0][0] + m[0][1] * m[0][1] + m[0][2] * m[0][2]).sqrt();
    let f = (len / 1.026).min(1.0);
    let n = (m[2][0] * m[2][0] + m[2][1] * m[2][1] + m[2][2] * m[2][2]).sqrt();
    let at = |d: f32| -> [f32; 4] {
        let k = if n == 0.0 { 0.0 } else { d / n };
        [m[3][0] + m[2][0] * k, m[3][1] + m[2][1] * k, m[3][2] + m[2][2] * k, m[3][3] + m[2][3] * k]
    };
    let p = at(f * -0.025 + -0.025);
    for _ in 0..2 {
        let r = w.rng.randi(0x10);
        let spin = if w.rng.randi(2) == 0 { r } else { -r };
        let life = w.ticks(6);
        puff23(w, [f * 0.02 + 0.01, 1.0, 0.75, f * 44000.0 + 6000.0], p, spin, 0x5030_30ff, life, None, 0x50);
    }
    let p = at(f * -0.015 + -0.01);
    for _ in 0..5 {
        let life = w.ticks(2);
        puff23(w, [f * 0.015 + 0.01, 1.0, 1.0, f * 25000.0 + 5000.0], p, 0x10, 0x7fff_ffff, life, Some(0), 0x7f);
    }
}

/// The thrusters' size and colour (`PartType23Spawn(0.1, 1, 0.9, 200000, p, spin, 0x15f580, 0x7f204080)`).
pub const THRUSTER_SIZE: f32 = 200000.0;
pub const THRUSTER_RGBA: u32 = 0x7f20_4080;
/// `FUN_00278810(150000, 60000, p, p, 0x15f580)`: the glow and core sizes of the trail.
pub const TRAIL_GLOW_SIZE: f32 = 150000.0;
pub const TRAIL_CORE_SIZE: f32 = 60000.0;

/// One type-23 puff (`PartType23Spawn(jitter, grow_lo, grow_hi, size, pos, spin, 0x15f580, rgba)`, velocity 0) with
/// the callers' patch: timer +0x0a = `life`, byte9 0x44, phase 2 (a timed fade from +0x2a = 0x7f over +0x2b = the
/// timer's low byte), and the rotation byte when given. Without a particle system only the pool slot is counted.
fn puff(w: &mut World, g: [f32; 4], p: [f32; 4], spin: i32, rgba: u32, life: i32, rotation: Option<u8>) -> bool { puff23(w, g, p, spin, rgba, life, rotation, 0x7f) }

/// [`puff`] with the phase-2 start alpha `a0` (+0x2a).
#[allow(clippy::too_many_arguments)]
fn puff23(w: &mut World, g: [f32; 4], p: [f32; 4], spin: i32, rgba: u32, life: i32, rotation: Option<u8>, a0: u8) -> bool {
    fx::puff23(w, g, p, spin, [0.0; 4], rgba, life, rotation.is_some(), a0)
}

/// `FUN_00278810` 0x278810 at point `p`: two glow puffs (spin ±`randi(16)`, life `ticks(30)`) and three white cores
/// (spin 16, −16, 16, life `ticks(6)`, a random rotation byte `randi(255)`).
fn trail(w: &mut World, p: [f32; 4]) { fx::jet_puffs(w, TRAIL_GLOW_SIZE, TRAIL_CORE_SIZE, p, p, [0.0; 4]); }
