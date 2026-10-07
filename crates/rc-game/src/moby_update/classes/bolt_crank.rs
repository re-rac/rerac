//! Bolt cranks, class 280: `BoltCrankUpdate` level01 0x2e0c68 (the same function on Eudora 0x2bfd30 and Batalia
//! 0x2d8270, cluster dd9dd80d; jump table 0x20ae00), with its tail `0x2e0bd8`; and the two Novalis classes the crank
//! drives, the rotator 641 (`CrankRotatorUpdate` 0x2f4348) and the slider 665 (`CrankSliderUpdate` 0x2f4710).
//! docs/plan/hero_states.md "Bolt crank"; docs/plan/triggers.md §5a. Native `f32`.
//!
//! Ratchet holds □ at the bolt with the wrench; the crank latches him (hero state 0x3b) and, while he walks round
//! it, turns and counts the angle: **progress** (pvar f32 +0x00, 0..1) = angle / (turns · 2π). Anything driven by
//! a crank reads that one number through its own pvar link (the crank's moby index; the class must be 280). A
//! crank let go before it is done unwinds (1/60 of the way a tick, spinning back) and what it drives follows back.
//! Done, it sinks 0.5 into the ground, its spawn id's death bits are set (it stays done on later visits: at load
//! it goes straight to done) and it plays its class sound 0.
//!
//! The hero side (the latch test, the ring, the speed, the animation, the release) is [`crate::hero::crank`]; the
//! crank's stores into the hero block and its `SetState` / `SetAnim` calls go through the hero-write channel
//! ([`HeroFields`](crate::moby_update::services::HeroFields): `pose`, `clear_motion`, `calls`).
//!
//! Pvar block (0x40 bytes):
//!
//! | off | type | meaning |
//! |---|---|---|
//! | 0x00 | f32 | progress 0..1 (read by the driven classes) |
//! | 0x14 | u32 | ticks since the last release (counts while undone and not held; > 30 to latch again) |
//! | 0x18 | f32 | the angle turned (reset to progress · turns · 2π at each latch) |
//! | 0x1c | f32 | Ratchet's speed round the bolt, u/tick ([`crank::Turn::speed`]) |
//! | 0x20 | s32 | the camera cuboid (the camera script's placement) |
//! | 0x24 | f32 | Ratchet's fall while on the bolt |
//! | 0x28 / 0x2c | f32 | the camera script's two parameters |
//! | 0x30 | f32 | turns to do (1 on Novalis, 1.5 / 3.5 on Eudora, 1 / 3.5 on Batalia) |
//! | 0x34 | s32 | checkpoint cuboid (−1: none): done with its mission not done → the mission and a checkpoint there |
//! | 0x38 | s32 | 0: done ends it (state 4); else it lets go into state 1 and sinks there (no disc crank drives one) |
//! | 0x3c | f32 | the bolt's top z (its z at init) |
//!
//! States (+0x20): 0 init, 1 waiting / unwinding, 2 nothing, 3 held, 4 sinking, 5 done. A crank without a pvar
//! block only spins (one turn a second).
//!
//! **Camera**: at the latch the script camera (`crate::cinematic`: `CameraScript(camera, Euler, 3, ticks(180), 0)`,
//! targets at cuboid +0x20), at the end `CameraScript2(4)` (blend back); `SetMissionDone` goes out as the cinematic
//! layer's engine request; the swing's curve `0x316e88(+0x28, +0x2c)` goes with it (the script camera's mode 3 is the
//! timed swing, `crate::follow_camera::script`). The visit-state record of the progress `0x29b0a0(+0x00, 4, m, 2)`
//! (`crate::moby_update::visit`, restored after a death reload) and the tail's two Novalis global flags (`0x13d394` /
//! `0x13d395` = 1 once the cranks with spawn ids 0x34 / 0x35 are in state 5: `story::set_flag(0xc / 0xd)`).
//!
//! Novalis: crank #296 (spawn id 0x34, (172.85, 90.59, 62.0)) drives the door pair 665 #683 / #684; crank #297 (spawn
//! id 0x35, (109.81, 93.46, 57.0), mission 1, checkpoint cuboid 55 at the landing pad) drives the door pair #685 /
//! #686 and the rotator 641 #672. Eudora (9) and Batalia (5) have cranks whose consumers are other classes (not
//! ported: they read the same progress).

use crate::hero::crank::{self, Seen, Turn};
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::creature::{self as c, add_rot, diff_rots, sub_rot, DT};
use crate::moby_update::services::{pvar as p, HeroCall, World};
use crate::ps2v::Pf;
use std::f32::consts::TAU;

/// The crank update's address in the level01 class table.
pub const UPDATE_FN: u32 = 0x2e0c68;
/// The rotator's and the slider's.
pub const ROTATOR_UPDATE_FN: u32 = 0x2f4348;
pub const SLIDER_UPDATE_FN: u32 = 0x2f4710;

/// The crank class (the driven classes check the linked moby's +0xa6 against it).
pub const CLASS: i16 = 280;
/// Classes that run [`update`], [`rotator_update`], [`slider_update`].
pub const CLASSES: [i16; 1] = [CLASS];
pub const ROTATOR_CLASSES: [i16; 1] = [641];
pub const SLIDER_CLASSES: [i16; 1] = [665];

/// The globals of the crank code.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Globals {
    /// gp−0x5360: the ring spring's velocity ([`Turn::spring_v`]; 0 at each latch).
    pub spring_v: f32,
}

const PROGRESS: usize = 0x00;
const IDLE: usize = 0x14;
const ANGLE: usize = 0x18;
const SPEED: usize = 0x1c;
const FALL: usize = 0x24;
const TURNS: usize = 0x30;
const CHECKPOINT: usize = 0x34;
const REARM: usize = 0x38;
const TOP: usize = 0x3c;

fn f(w: &World, id: MobyId, o: usize) -> f32 { p::ff(&w.m(id).pvars, o) }
fn set_f(w: &mut World, id: MobyId, o: usize, x: f32) { p::set_ff(&mut w.mm(id).pvars, o, x); }
fn i(w: &World, id: MobyId, o: usize) -> i32 { p::i32(&w.m(id).pvars, o) }

/// The crank's progress (pvar+0x00) when `id` is a crank with a pvar block.
pub fn progress(w: &World, id: MobyId) -> Option<f32> {
    let m = w.table.mobys.get(id)?;
    (m.o_class == CLASS && m.pvars.len() >= 4).then(|| p::ff(&m.pvars, PROGRESS))
}

/// `BoltCrankUpdate` (0x2e0c68).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.is_empty() {
        let m = w.mm(id);
        m.rotation[2] = add_rot(m.rotation[2], DT * TAU);
        return;
    }
    if w.m(id).pvars.len() < 0x40 { return; }
    let st = w.m(id).state;
    if st == 3 && w.hero.state != crank::STATE {
        // He left the state some other way (a hit, a fall): the crank lets go.
        w.mm(id).state = 1;
        camera_end(w);
        return;
    }
    match st {
        0 => {
            let m = w.mm(id);
            let z = m.position[2];
            p::set_ff(&mut m.pvars, TOP, z);
            m.state = 1;
        }
        1 => wait(w, id),
        3 => {
            if held(w, id) { return; }
        }
        4 => {
            let top = f(w, id, TOP);
            let m = w.mm(id);
            m.position[2] -= DT * 0.5;
            if m.position[2] <= top - 0.5 {
                m.position[2] = top - 0.5;
                m.state = 5;
                p::set_ff(&mut m.pvars, PROGRESS, 1.0);
            }
        }
        5 => set_f(w, id, PROGRESS, 1.0),
        _ => {}
    }
    tail(w, id);
}

/// State 1: done on an earlier visit → done; the latch; else unwind (or, done with pvar+0x38, sink).
fn wait(w: &mut World, id: MobyId) {
    let (sid, level) = (w.m(id).spawn_id, w.svc.level);
    if w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(level, sid)) {
        let m = w.mm(id);
        m.position[2] -= 0.5;
        m.state = 5;
        p::set_ff(&mut m.pvars, PROGRESS, 1.0);
        return;
    }
    let pos = w.m(id).position;
    let bolt = [pos[0], pos[1], pos[2]];
    let prog = f(w, id, PROGRESS);
    let seen = Seen::of(w.hero);
    let ready = (w.ticks(30) as u32) < p::u32(&w.m(id).pvars, IDLE) && pos[2] == f(w, id, TOP) && prog < 1.0;
    if crank::latch(&seen, bolt) && ready {
        p::set_u32(&mut w.mm(id).pvars, IDLE, 0);
        w.svc.cranks.spring_v = 0.0;
        // No targetable moby (an enemy) within 3.25 of the bolt (the 0x15ffe4 chain of this tick's run list).
        let (_, targets) = crate::moby_update::scheduler::build_active_list(w.table, w.camera, &w.svc.groups);
        if targets.iter().any(|&t| c::dist2(pos, w.m(t).position) < 3.25) { return; }
        w.hero_fields_mut().call(HeroCall::SetState { id: crank::STATE, play: true });
        camera_start(w, id);
        let turns = f(w, id, TURNS);
        let m = w.mm(id);
        m.state = 3;
        p::set_ff(&mut m.pvars, FALL, 0.0);
        p::set_ff(&mut m.pvars, ANGLE, prog * turns * TAU);
        return;
    }
    if prog < 1.0 {
        let n = p::u32(&w.m(id).pvars, IDLE).wrapping_add(1);
        p::set_u32(&mut w.mm(id).pvars, IDLE, n);
        let t60 = w.ticks(60) as f32;
        let mut x = prog;
        c::turn::approach(0.0, 1.0 / t60, &mut x);
        set_f(w, id, PROGRESS, x);
        if 0.0 < x {
            let step = f(w, id, TURNS) * TAU / t60;
            let m = w.mm(id);
            m.rotation[2] = sub_rot(m.rotation[2], step);
        }
        let top = f(w, id, TOP);
        let m = w.mm(id);
        m.position[2] += DT * 0.5;
        if top <= m.position[2] { m.position[2] = top; }
    } else {
        if i(w, id, REARM) == 0 { return; }
        mission_checkpoint(w, id);
        let top = f(w, id, TOP);
        let m = w.mm(id);
        m.position[2] -= DT * 0.5;
        if m.position[2] <= top - 0.5 { m.position[2] = top - 0.5; }
    }
}

/// State 3 with Ratchet in 0x3b: let go (□ after 60 ticks, or done), else one tick of turning. True: it let go
/// (the update returns without its tail).
fn held(w: &mut World, id: MobyId) -> bool {
    let seen = Seen::of(w.hero);
    let prog = f(w, id, PROGRESS);
    if crank::release(&seen) || prog == 1.0 {
        w.hero_fields_mut().call(HeroCall::SetState { id: 0, play: true });
        if i(w, id, REARM) != 0 {
            w.mm(id).state = 1;
        } else if prog == 1.0 {
            done(w, id);
        }
        camera_end(w);
        return true;
    }
    let pos = w.m(id).position;
    let bolt = [pos[0], pos[1], pos[2]];
    let mut t = Turn { speed: f(w, id, SPEED), fall: f(w, id, FALL), spring_v: w.svc.cranks.spring_v };
    let r = {
        let wr: &World = w;
        let ground = |q: [f32; 3]| wr.ground_height(Pf::f(0.5), [Pf::f(q[0]), Pf::f(q[1]), Pf::f(q[2]), Pf::ZERO], 0).to_f32();
        // The mirrored-animation cheat 0x15edb5 (`crate::cheats`): the tangent the other way round.
        crank::turn(&seen, bolt, &mut t, wr.svc.cheats.on(crate::cheats::slot::MIRROR_ANIM), ground)
    };
    set_f(w, id, SPEED, t.speed);
    set_f(w, id, FALL, t.fall);
    w.svc.cranks.spring_v = t.spring_v;
    {
        let hf = w.hero_fields_mut();
        if let Some((seq, n)) = r.anim { hf.call(HeroCall::SetAnim { blend: n as f32, seq, frame: 0 }); }
        hf.pose = Some(r.pose);
        hf.clear_motion();
    }
    if r.seq_b != crank::SEQ_LATCH {
        let m = w.mm(id);
        m.rotation[2] = crank::bolt_follow(m.rotation[2], r.after);
    }
    let angle = f(w, id, ANGLE) + diff_rots(r.after, r.before);
    set_f(w, id, ANGLE, angle);
    let prog = (angle / (f(w, id, TURNS) * TAU)).clamp(0.0, 1.0);
    set_f(w, id, PROGRESS, prog);
    false
}

/// Done (state 3 → 4): the visit-state save, its death bits, class sound 0, then the mission and checkpoint.
fn done(w: &mut World, id: MobyId) {
    crate::moby_update::visit::record_pvar(w, id, id, 0, 4);
    let (sid, level) = (w.m(id).spawn_id, w.svc.level);
    w.mm(id).state = 4;
    w.svc.save.death.insert((level, sid));
    w.svc.save.death_level.insert(sid);
    w.play_sound(0, 0, id);
    mission_checkpoint(w, id);
}

/// With a checkpoint cuboid (+0x34) and its mission (+0xb0) not done: the mission done and the checkpoint record
/// at the cuboid (`0x29ac10(centre, Euler)`).
fn mission_checkpoint(w: &mut World, id: MobyId) {
    let cub = i(w, id, CHECKPOINT);
    if cub == -1 || w.mission_done(w.svc.level, w.m(id).mission) == 0xff { return; }
    crate::moby_update::visit::record_pvar(w, id, id, 0, 4);
    let mission = w.m(id).mission;
    crate::cinematic::set_mission_done(w, mission);
    let Some(s) = usize::try_from(cub).ok().and_then(|k| w.svc.volumes.cuboids.get(k)) else { return };
    let (centre, euler) = (s.matrix[3], s.euler);
    super::checkpoint::record(w, super::checkpoint::Record { pos: [centre[0], centre[1], centre[2]], rot: [euler[0], euler[1], euler[2]] });
}

/// The latch's camera: `CameraScript(camera 0x167240, its Euler 0x167250, 3, ticks(180), 0)` (mode 3: a timed
/// swing from the current view), its targets = the camera cuboid +0x20 (Euler +0x70, centre +0x30), and the swing's
/// two curve parameters `0x316e88(+0x28, +0x2c)` (D+0x124 / D+0x128: the distance curve's end slopes).
fn camera_start(w: &mut World, id: MobyId) {
    let cam = [w.camera[0].to_f32(), w.camera[1].to_f32(), w.camera[2].to_f32()];
    let euler = w.hero.loop_in.cam_euler;
    let t = w.ticks(180);
    crate::cinematic::camera_script(w, cam, euler, 3, t, false);
    let cub = i(w, id, 0x20);
    if let Some(s) = usize::try_from(cub).ok().and_then(|k| w.svc.volumes.cuboids.get(k)) {
        let (c, e) = (s.matrix[3], s.euler);
        crate::cinematic::camera_targets(w, Some([c[0], c[1], c[2]]), Some([e[0], e[1], e[2]]));
    }
    let (a, b) = (f(w, id, 0x28), f(w, id, 0x2c));
    crate::cinematic::camera_curve(w, a, b);
}

/// `CameraScript2(4)` (a blend back to the follow camera; nothing when no script camera is up).
fn camera_end(w: &mut World) { crate::cinematic::camera_script2(w, 4); }

/// `0x2e0bd8`: on Novalis, the cranks with spawn ids 0x34 / 0x35 done set the global flags 0x13d394 / 0x13d395.
fn tail(w: &mut World, id: MobyId) {
    let m = w.m(id);
    if w.svc.level == 1 && m.state == 5 && (m.spawn_id == 0x34 || m.spawn_id == 0x35) {
        let flag = if m.spawn_id == 0x34 { 0xc } else { 0xd };
        // The game stores the byte every tick in state 5; the port writes it once (the same flag).
        if crate::moby_update::story::flag(w, flag) != 1 { crate::moby_update::story::set_flag(w, flag, 1); }
    }
}

/// The crank a driven moby links (its pvar s32 +0x00, a moby index; −1: none), when it is a crank.
fn linked(w: &World, id: MobyId) -> Option<MobyId> {
    let k = usize::try_from(p::i32(&w.m(id).pvars, 0)).ok()?;
    progress(w, k).map(|_| k)
}

/// `CrankRotatorUpdate` (0x2f4348), class 641: its Euler x from its start to pvar+0x08 (degrees) by the crank's
/// progress; keeps the crank updating at any distance (+0x30 = 0xff).
pub fn rotator_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0xc { return; }
    if w.m(id).state == 0 {
        let m = w.mm(id);
        let x = m.rotation[0];
        let deg = p::ff(&m.pvars, 8);
        p::set_ff(&mut m.pvars, 4, x);
        p::set_ff(&mut m.pvars, 8, deg * 0.017_453_292);
        m.state = 1;
        return;
    }
    if w.m(id).state != 1 { return; }
    let Some(k) = linked(w, id) else { return };
    let prog = p::ff(&w.m(k).pvars, PROGRESS);
    w.mm(k).update_dist = 0xff;
    let (start, end) = (f(w, id, 4), f(w, id, 8));
    let turn = normalize(add_rot(end, -start) * prog);
    w.mm(id).rotation[0] = add_rot(start, turn);
}

/// `FastNormalizeAngle`: into [−π, π).
fn normalize(a: f32) -> f32 { (a + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI }

/// `CrankSliderUpdate` (0x2f4710), class 665: its x / y from its start to pvar+0x0c / +0x10 by the crank's
/// progress; the loop sound 0 while it moves, sound 1 at either end.
pub fn slider_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x1c { return; }
    const VOICE: usize = 0x14;
    const LAST: usize = 0x18;
    if w.m(id).state == 0 {
        let m = w.mm(id);
        let (x, y) = (m.position[0], m.position[1]);
        p::set_ff(&mut m.pvars, 4, x);
        p::set_ff(&mut m.pvars, 8, y);
        p::set_i32(&mut m.pvars, VOICE, -1);
        p::set_ff(&mut m.pvars, LAST, 0.0);
        m.state = 1;
        return;
    }
    if w.m(id).state != 1 || p::i32(&w.m(id).pvars, 0) == -1 { return; }
    let (x1, y1) = (f(w, id, 0xc), f(w, id, 0x10));
    if x1 == 0.0 || y1 == 0.0 { return; }
    let Some(k) = linked(w, id) else { return };
    let prog = p::ff(&w.m(k).pvars, PROGRESS);
    let (x0, y0) = (f(w, id, 4), f(w, id, 8));
    {
        let m = w.mm(id);
        m.position[0] = x0 + (x1 - x0) * prog;
        m.position[1] = y0 + (y1 - y0) * prog;
    }
    let voice = i(w, id, VOICE);
    if prog == f(w, id, LAST) {
        // Still: its loop stops.
        if w.sound_alive(voice, id) {
            w.release_sound(voice, id);
            p::set_i32(&mut w.mm(id).pvars, VOICE, -1);
        }
    } else if prog == 1.0 || prog == 0.0 {
        // Arrived at an end: the loop stops, the end sound.
        if w.sound_alive(voice, id) {
            w.release_sound(voice, id);
            p::set_i32(&mut w.mm(id).pvars, VOICE, -1);
        }
        w.play_sound(1, 0, id);
    } else if !w.sound_alive(voice, id) {
        let v = w.play_sound(0, 4, id);
        p::set_i32(&mut w.mm(id).pvars, VOICE, v);
    }
    set_f(w, id, LAST, prog);
}

/// Whether moby `m`'s mode makes it a target the latch refuses near (mode 0x1000).
pub fn blocks_latch(mode_bits: u16) -> bool { mode_bits & mode::TARGETABLE != 0 }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::Hero;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::services::{ClassTable, Services};
    use crate::pad::{button, PadInput};
    use crate::rng::Rng;

    fn crank_moby(pos: [f32; 3], turns: f32) -> Moby {
        let mut m = Moby::zeroed();
        m.o_class = CLASS;
        m.position = [pos[0], pos[1], pos[2], 1.0];
        m.pvars = vec![0; 0x40];
        p::set_ff(&mut m.pvars, TURNS, turns);
        p::set_i32(&mut m.pvars, CHECKPOINT, -1);
        p::set_i32(&mut m.pvars, 0x20, -1);
        m.spawn_id = 7;
        m.mission = 0xff;
        m
    }

    fn slider(link: i32, from: [f32; 2], to: [f32; 2]) -> Moby {
        let mut m = Moby::zeroed();
        m.o_class = 665;
        m.position = [from[0], from[1], 3.0, 1.0];
        m.pvars = vec![0; 0x20];
        p::set_i32(&mut m.pvars, 0, link);
        p::set_ff(&mut m.pvars, 0xc, to[0]);
        p::set_ff(&mut m.pvars, 0x10, to[1]);
        m
    }

    /// Ratchet 1.2 west of the bolt, facing it, feet 0.3 above it, the wrench in hand, □ pressed `square_ago`
    /// ticks ago (None: not pressed).
    fn hero_at_bolt(bolt: [f32; 3], square: bool) -> Hero {
        let mut h = Hero::spawn([bolt[0] - 1.2, bolt[1], bolt[2] + 0.3], 0.0);
        h.items.slot.id = 8;
        let anim = rc_formats::moby_anim::AnimState { seq_a: 1, frame_a: 0, seq_b: 1, frame_b: 0, t: 0.0, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false };
        h.items.slot.item = Some(crate::hero::items::HandItem { o_class: crate::hero::melee::WRENCH_CLASS, mstate: 0, anim, snapshot: None, scale: 1.0, position: [0.0; 3], rows: [[0; 4]; 3], hit_timer: 0, flight: Default::default() });
        if square {
            let input = PadInput::neutral().press(button::SQUARE);
            h.loop_in.pad.update(Some(&input.bytes()), false);
        }
        h
    }

    fn run(table: &mut MobyTable, hero: &Hero, svc: &mut Services, ids: &[MobyId], counter: u64) -> Option<crate::moby_update::services::HeroFields> {
        let classes = ClassTable::default();
        let mut rng = Rng::new();
        let mut w = World::new(table, hero, &mut rng, &classes, svc, counter);
        for &id in ids {
            match w.m(id).o_class {
                CLASS => update(&mut w, id),
                665 => slider_update(&mut w, id),
                641 => rotator_update(&mut w, id),
                _ => {}
            }
        }
        svc.take_hero_writes()
    }

    /// Init, the 30-tick wait, the latch (SetState 0x3b queued, state 3, the angle from the progress), then a
    /// release before done unwinds it and the slider follows back.
    #[test]
    fn latch_turn_release_unwind() {
        let bolt = [20.0, 20.0, 4.0];
        let mut table = MobyTable::new(vec![crank_moby(bolt, 1.0), slider(0, [22.0, 20.0], [23.0, 20.0])], 0);
        let mut svc = Services::new();
        let idle = Hero::spawn([0.0, 0.0, 0.0], 0.0);
        // Init + 31 ticks of waiting (the 30-tick rearm).
        for t in 0..32 { assert!(run(&mut table, &idle, &mut svc, &[0, 1], t).is_none()); }
        assert_eq!(table.mobys[0].state, 1);
        assert_eq!(p::ff(&table.mobys[0].pvars, TOP), 4.0);
        let mut h = hero_at_bolt(bolt, true);
        let f = run(&mut table, &h, &mut svc, &[0, 1], 40).expect("the latch writes the hero");
        assert_eq!(f.calls[0], Some(HeroCall::SetState { id: crank::STATE, play: true }));
        assert_eq!(table.mobys[0].state, 3);
        // In 0x3b with the stick along his facing (he faces the bolt: +x; the tangent there is −y ... the stick
        // pushes his facing): the crank turns, the progress grows, the slider moves.
        h.state = crank::STATE;
        h.loop_in.pad = Default::default();
        h.stick = [Pf::ZERO, Pf::f(-1.0)];
        let mut last = 0.0;
        for t in 0..90 {
            let f = run(&mut table, &h, &mut svc, &[0, 1], 41 + t).unwrap();
            let pose = f.pose.expect("the crank places him");
            h.pos = [Pf::f(pose.pos[0]), Pf::f(pose.pos[1]), Pf::f(pose.pos[2]), Pf::ZERO];
            h.rot[2] = Pf::f(pose.yaw);
            h.timer += 1;
            assert!(f.clear_motion);
        }
        let prog = p::ff(&table.mobys[0].pvars, PROGRESS);
        assert!(prog > 0.2 && prog < 1.0, "progress {prog}");
        let sx = table.mobys[1].position[0];
        assert!((sx - (22.0 + prog)).abs() < 1e-5, "slider at {sx} for {prog}");
        // Let go (□ after 60 ticks): SetState(0) queued, and once he is out of 0x3b it unwinds in ≤ 60 ticks.
        h.loop_in.pad.update(Some(&PadInput::neutral().press(button::SQUARE).bytes()), false);
        let f = run(&mut table, &h, &mut svc, &[0, 1], 200).unwrap();
        assert_eq!(f.calls[0], Some(HeroCall::SetState { id: 0, play: true }));
        h.state = 0;
        h.pos = [Pf::f(0.0); 4];
        for t in 0..70 {
            run(&mut table, &h, &mut svc, &[0, 1], 201 + t);
            let p0 = p::ff(&table.mobys[0].pvars, PROGRESS);
            assert!(p0 <= prog);
            last = p0;
        }
        assert_eq!(last, 0.0, "unwound");
        assert_eq!(table.mobys[1].position[0], 22.0, "the door back");
    }

    /// Done: state 4, sinking 0.5, then 5 with the death bits set; a later load starts done.
    #[test]
    fn done_sinks_and_persists() {
        let bolt = [20.0, 20.0, 4.0];
        let mut table = MobyTable::new(vec![crank_moby(bolt, 1.0)], 0);
        let mut svc = Services::new();
        let h = hero_at_bolt(bolt, false);
        run(&mut table, &h, &mut svc, &[0], 1);
        table.mobys[0].state = 3;
        p::set_ff(&mut table.mobys[0].pvars, PROGRESS, 1.0);
        let mut h3 = h.clone();
        h3.state = crank::STATE;
        let f = run(&mut table, &h3, &mut svc, &[0], 2).unwrap();
        assert_eq!(f.calls[0], Some(HeroCall::SetState { id: 0, play: true }));
        assert_eq!(table.mobys[0].state, 4);
        assert!(svc.save.death.contains(&(1, 7)));
        assert_eq!(svc.sounds.last().map(|e| (e.index, e.flags)), Some((0, 0)));
        for t in 0..70 { run(&mut table, &h, &mut svc, &[0], 3 + t); }
        assert_eq!((table.mobys[0].state, table.mobys[0].position[2]), (5, 3.5));
        // A new load with the death bit: straight to done, 0.5 down.
        let mut fresh = MobyTable::new(vec![crank_moby(bolt, 1.0)], 0);
        run(&mut fresh, &h, &mut svc, &[0], 100);
        run(&mut fresh, &h, &mut svc, &[0], 101);
        assert_eq!((fresh.mobys[0].state, fresh.mobys[0].position[2], p::ff(&fresh.mobys[0].pvars, PROGRESS)), (5, 3.5, 1.0));
    }

    /// No latch while an enemy (a targetable moby) is within 3.25 of the bolt, nor with an undone rearm.
    #[test]
    fn latch_refusals() {
        let bolt = [20.0, 20.0, 4.0];
        let mut enemy = Moby::zeroed();
        enemy.o_class = 459;
        enemy.position = [21.0, 22.0, 4.0, 1.0];
        enemy.mode |= mode::TARGETABLE;
        enemy.update_dist = 0xff;
        enemy.group = -1;
        let mut table = MobyTable::new(vec![crank_moby(bolt, 1.0), enemy], 0);
        table.mobys[0].update_dist = 0xff;
        let mut svc = Services::new();
        let h = hero_at_bolt(bolt, true);
        // Too early: the rearm counter is 0.
        run(&mut table, &h, &mut svc, &[0], 1);
        assert!(run(&mut table, &h, &mut svc, &[0], 2).is_none());
        assert!(blocks_latch(table.mobys[1].mode));
        // Rearmed, but the enemy 2.2 away: the counter restarts, no latch.
        p::set_u32(&mut table.mobys[0].pvars, IDLE, 100);
        assert!(run(&mut table, &h, &mut svc, &[0], 3).is_none());
        assert_eq!((table.mobys[0].state, p::u32(&table.mobys[0].pvars, IDLE)), (1, 0));
        // The enemy gone 4 away: it latches.
        table.mobys[1].position = [24.0, 20.0, 4.0, 1.0];
        p::set_u32(&mut table.mobys[0].pvars, IDLE, 100);
        assert!(run(&mut table, &h, &mut svc, &[0], 4).is_some());
        assert_eq!(table.mobys[0].state, 3);
    }

    /// The rotator: Euler x from its start to the target angle (degrees) by the progress.
    #[test]
    fn rotator_follows_progress() {
        let mut rot = Moby::zeroed();
        rot.o_class = 641;
        rot.pvars = vec![0; 0x10];
        p::set_i32(&mut rot.pvars, 0, 0);
        p::set_ff(&mut rot.pvars, 8, 90.0);
        let mut table = MobyTable::new(vec![crank_moby([0.0; 3], 1.0), rot], 0);
        let mut svc = Services::new();
        let h = Hero::spawn([50.0, 50.0, 0.0], 0.0);
        run(&mut table, &h, &mut svc, &[1], 1);
        p::set_ff(&mut table.mobys[0].pvars, PROGRESS, 0.5);
        run(&mut table, &h, &mut svc, &[1], 2);
        assert!((table.mobys[1].rotation[0] - std::f32::consts::FRAC_PI_4).abs() < 1e-6);
        assert_eq!(table.mobys[0].update_dist, 0xff);
    }
}
