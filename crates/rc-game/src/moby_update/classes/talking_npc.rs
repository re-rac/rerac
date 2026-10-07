//! Talking NPCs, class 774: `TalkingNpcUpdate` level01 0x2ff118 (Novalis' "Water Pump Worker", Batalia's "Big
//! Turret Guy"). The dialogue is the shared talk system ([`interact::talk_update`], docs/plan/interaction.md §3);
//! this class only decides when to run it and what the end of a scene means. Native `f32`.
//!
//! The pvar block starts with the talk block (`crate::moby_update::interact::talk`); +0x20 the debug name, +0x44
//! cleared at init, +0x4c a checkpoint cuboid (−1 none).
//!
//! States (+0x20):
//! * **0**: +0x30 = 0xff (update distance), +0x44 = 0. Level 1: planet 2 (Aridia) already unlocked
//!   (`0x13dd42`) → state 3 (the worker is gone); else state 1. Level 8: state 4; others 7. Then
//!   `NpcTalkRegister`.
//! * **1**: `NpcTalkUpdate`; when it starts a scene: `FUN_002783a8(2.2, npc)` (after the scene Ratchet stands 2.2
//!   in front of the NPC, facing it: [`Interact::scene_end_place`]) and state 2.
//! * **2**: waits while game mode 2 (a scene) runs; then, the NPC's mission (+0xb0) not done yet: `SetMissionDone`
//!   ([`crate::cinematic::set_mission_done`]) and, with a checkpoint cuboid +0x4c, the checkpoint record
//!   `FUN_0029ac10(centre, Euler)` of that cuboid ([`super::checkpoint::record`]); state 1; when the node that
//!   played last (talk +0x04) is 4 (Novalis:
//!   the Infobot sold): `UnlockPlanet(2)` and `ShowPlanetBanner(2)` (`crate::cinematic`: the saved game's planet
//!   bits and the HUD banner "Infobot for Planet Aridia acquired"), state 3, the save (`crate::cinematic::save`).
//! * **3**: `DeleteMoby`.
//! * **4 / 5** (Batalia's turret guy; state 5 moves Ratchet to a fixed point): not ported (counted).
//!
//! **The head look-at** (every state but 3; [`look_at`], docs/plan/moby_animation.md §9), coverage of 0x2ff3d0..0x2ff734:
//!
//! | address | what it does | port |
//! |---|---|---|
//! | 0x2ff3d0 | only while sequence B (+0x53) is 0 (not talking): the targets below; else the records only spring back | [`look_at`] |
//! | 0x2ff3f4..0x2ff4b8 | XY distance to Ratchet (0x13f3d0) < 8 and his body point (0x13f420) within π/2 of the yaw (`FastDiffRots`): P+0x164 = `ticks(120)` while he moves (\|0x13f450\| > 0.01), else `FastDecTimer(P+0x164)` | [`look_at`] |
//! | 0x2ff4bc..0x2ff4d8 | otherwise, P+0x164 ≠ 0: P+0x164 = 0, P+0x150 = his body point (the last place he was seen) | [`look_at`] |
//! | 0x2ff4dc..0x2ff57c | `FastDecTimer(P+0x168)` out: P+0x168 = (int)(`randf(180, 300)`·0x15ed68); a glance point 6 units away at yaw + `randf(−90, 90)`° and pitch `randf(0, 30)`° (`0x277b50`), + the NPC's position → P+0x150 (3 draws) | [`look_at`] |
//! | 0x2ff580..0x2ff5c0 | P+0x164 ≠ 0: look at his body point, k 0.04; else at P+0x150, k 0.02 (d 0.3) | [`look_at`] |
//! | 0x2ff5cc..0x2ff6d4 | from the eye (position + 1 z): yaw rel. to the NPC's (±π/2), pitch −atan2(z, xy) (±π/6); P+0x138 (list 1 z) = yaw/2, P+0xb4 (list 0 y) = pitch·1.25, P+0xb8 (list 0 z) = yaw/2 | [`look_at`] |
//! | 0x2ff6d8..0x2ff6fc | the big-head cheat 0x15edb0: P+0xc0 (list 0's scale) = 2.75 (k, d × 0x15ed64 = 1 either way) | [`look_springs`] |
//! | 0x2ff700..0x2ff734 | `FUN_002777d8(k, d, npc, P+0x50, 0)`, `(k, d, npc, P+0xd0, 1)` (k, d × 0x15ed64 = 1) | [`crate::moby_update::manip::look`] |
//!
//! `FUN_002ff028` at the top: the big-head cheat's manipulator on the scene actors of this class or 0x32b at 2.1
//! ([`crate::moby_update::manip::scene_big_head`]).

use crate::moby_runtime::MobyId;
use crate::moby_update::creature as c;
use crate::moby_update::interact::{self, talk};
use crate::moby_update::manip;
use crate::moby_update::services::{pvar as p, World};

pub const UPDATE_FN: u32 = 0x2ff118;
pub const CLASSES: [i16; 1] = [774];
/// `FUN_002783a8(2.2, npc)`.
const PLACE_DISTANCE: f32 = 2.2;

/// `TalkingNpcUpdate` (0x2ff118).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x50 { return; }
    // FUN_002ff028: the actors' big-head cheat on this class's (or 0x32b's) scene actors at 2.1 (gp−0x4ed0).
    let own = w.m(id).o_class;
    manip::scene_big_head(w, &[own, 0x32b], 0, f32::from_bits(0x4006_6666));
    interact::poll_scene_end(w, id);
    // Drawn last frame and within 32 units of the camera: the shadow probe, shadow range 0x1a.
    if w.m(id).visible != 0 {
        let (p, c) = (w.m(id).position, w.camera.map(|x| f32::from_bits(x.0)));
        if ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2)).sqrt() < 32.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0x1a;
        }
    }
    match w.m(id).state {
        0 => {
            w.mm(id).update_dist = 0xff;
            p::set_i32(&mut w.mm(id).pvars, 0x44, 0);
            if w.svc.level == 1 {
                if w.svc.interact.game.planet_unlocked.get(2).copied().unwrap_or(0) != 0 {
                    w.delete_moby(id);
                    return;
                }
                w.mm(id).state = 1;
            } else {
                w.mm(id).state = if w.svc.level == 8 { 4 } else { 7 };
            }
            interact::talk_register(w, id);
        }
        1 => {
            if interact::talk_update(w, id) {
                let m = w.m(id);
                let yaw = m.rotation[2];
                let pos = [m.position[0] + yaw.cos() * PLACE_DISTANCE, m.position[1] + yaw.sin() * PLACE_DISTANCE, m.position[2]];
                w.svc.interact.scene_end_place = Some((pos, interact::add_rot(yaw, std::f32::consts::PI)));
                w.mm(id).state = 2;
            }
        }
        2 => {
            if w.svc.interact.talker == Some(id) { return look_at(w, id); }
            let mission = w.m(id).mission;
            if mission != 0xff && w.mission_done(w.svc.level, mission) != 0xff {
                crate::cinematic::set_mission_done(w, mission);
                let c = p::i32(&w.m(id).pvars, 0x4c);
                if let Some(s) = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c).copied() {
                    super::checkpoint::record(w, super::checkpoint::Record { pos: s.centre(), rot: s.euler });
                }
            }
            w.mm(id).state = 1;
            if p::i16(&w.m(id).pvars, talk::LAST) == 4 {
                // `UnlockPlanet(2)` (its own banner unless on level 2), `ShowPlanetBanner(2)`, the save.
                crate::cinematic::unlock_planet(w, 2);
                crate::cinematic::show_planet_banner(w, 2);
                w.mm(id).state = 3;
                // `0x29b0a0(m + 0x20, 1, m, 1, 0x1baaa0)`: the sold state survives a death reload.
                crate::moby_update::visit::record_field(w, id, id, 0x20);
                crate::cinematic::save(w);
            }
        }
        3 => return w.delete_moby(id),
        _ => w.svc.unported("talking npc: Batalia turret guy states 4/5"),
    }
    look_at(w, id);
}

/// The pvar layout of an NPC's head look-at: the same code is compiled into several NPC classes with their own offsets
/// (774 here; Quartu's Giant Clank mission NPC 1446, `units::quartu_giant_mission`). `pitch` = the record that takes the
/// pitch (+0x64, ×1.25) and half the yaw (+0x68), `yaw` = the one that takes half the yaw (+0x68); both are run in that
/// order (`FUN_002777d8(k, d, npc, rec, list)`).
#[derive(Clone, Copy, Debug)]
pub struct LookLayout {
    pub pitch: (usize, u8),
    pub yaw: (usize, u8),
    pub glance: usize,
    pub seen: usize,
    pub glance_timer: usize,
    /// Only while sequence B (+0x53) is 0 (774's gate; 1446 has none).
    pub gate_seq_b: bool,
    /// The eye's height over the NPC's position (774: 1; Aridia's surfer agent 788: 2).
    pub eye: f32,
    /// The pitch record's y = pitch · `pitch_k` (774: 1.25; the Aridia NPCs: 1).
    pub pitch_k: f32,
    /// The pitch record's z = yaw · `yaw_a`, the yaw record's z = yaw · `yaw_b` (774: 0.5 / 0.5; 786: 0.7 / 0.3;
    /// 788: 0.6 / 0.4).
    pub yaw_a: f32,
    pub yaw_b: f32,
    /// The "seen" and glance timers are s16 (`FastDecTimer__FRs`: 786) instead of s32.
    pub short_timers: bool,
    /// The sequence B under which the targets are written when gated (0; Batalia's deserter 1144: 1).
    pub gate_main: u8,
    /// A second sequence B under which the targets are still written (Rilgar's race girl 918: 2; 0xff: none).
    pub gate_alt: u8,
    /// The spring rate while Ratchet is seen (774: 0.04; 918: 0.03).
    pub k_seen: f32,
}

impl LookLayout {
    /// The layout of a class whose look-at matches 774's but for its records and timers (the defaults of the
    /// optional fields: eye 1, pitch × 1.25, yaw 0.5 / 0.5, s32 timers, no second gate, k 0.04).
    pub const fn talker(pitch: (usize, u8), yaw: (usize, u8), glance: usize, seen: usize, glance_timer: usize, gate_seq_b: bool) -> LookLayout {
        LookLayout { pitch, yaw, glance, seen, glance_timer, gate_seq_b, eye: 1.0, pitch_k: 1.25, yaw_a: 0.5, yaw_b: 0.5, short_timers: false, gate_main: 0, gate_alt: 0xff, k_seen: f32::from_bits(0x3d23_d70a) }
    }
}

/// P+0x50 / P+0xd0: the look records on joint lists 0 (head) and 1 (neck); P+0x150 the glance point, P+0x164 the
/// "seen Ratchet" timer, P+0x168 the glance timer.
pub const LAYOUT: LookLayout = LookLayout::talker((0x50, 0), (0xd0, 1), 0x150, 0x164, 0x168, true);

/// The head look-at tail of `TalkingNpcUpdate` (module doc table).
pub fn look_at(w: &mut World, id: MobyId) { look_at_layout(w, id, &LAYOUT) }

/// The head look-at on the layout `l` (module doc table): the targets, then the two records' springs.
pub fn look_at_layout(w: &mut World, id: MobyId, l: &LookLayout) {
    let (k, d) = look_targets(w, id, l);
    look_springs(w, id, l, k, d);
}

/// The two records' springs `FUN_002777d8(k, d, npc, rec, list)` (k, d × 0x15ed64 = 1).
pub fn look_springs(w: &mut World, id: MobyId, l: &LookLayout, k: f32, d: f32) {
    // 0x2ff6d8: the actors' big-head cheat 0x15edb0: the head record's scale request (+0x70) = 2.75.
    if w.svc.cheats.on(crate::cheats::slot::ACTORS) { p::set_ff(&mut w.mm(id).pvars, l.pitch.0 + manip::rec::REC_SCALE, f32::from_bits(0x4030_0000)); }
    manip::look(w, id, id, l.pitch.0, l.pitch.1, k, d);
    manip::look(w, id, id, l.yaw.0, l.yaw.1, k, d);
}

/// The targets half of [`look_at_layout`]; returns the springs' (k, d).
pub fn look_targets(w: &mut World, id: MobyId, l: &LookLayout) -> (f32, f32) {
    let need = [l.pitch.0 + manip::rec::SIZE, l.yaw.0 + manip::rec::SIZE, l.glance + 0x10, l.seen + 4, l.glance_timer + 4].into_iter().max().unwrap_or(0);
    if w.m(id).pvars.len() < need { w.mm(id).pvars.resize(need, 0); }
    let (mut k, d) = (f32::from_bits(0x3ca3_d70a), f32::from_bits(0x3e99_999a));
    let seq = w.m(id).anim.seq_b;
    if !l.gate_seq_b || seq == l.gate_main || seq == l.gate_alt {
        let h = w.hero;
        let hp = h.pos.map(|x| x.to_f32());
        let body = h.body_point.map(|x| x.to_f32());
        let pos = c::pos(w, id);
        let mut near = c::dist2(pos, hp) < 8.0;
        if near {
            let a = c::atan(body[0] - pos[0], body[1] - pos[1]);
            near = c::diff_rots(c::yaw(w, id), a) < std::f32::consts::FRAC_PI_2;
        }
        // The timers' width (s32 `FastDecTimer__FRi`, or s16 `__FRs`).
        let get = |w: &World, o: usize| if l.short_timers { c::pi16(w, id, o) as i32 } else { c::pi32(w, id, o) };
        let set = |w: &mut World, o: usize, v: i32| if l.short_timers { c::set_pi16(w, id, o, v as i16) } else { c::set_pi32(w, id, o, v) };
        let dec = |w: &mut World, o: usize| if l.short_timers { c::dec_timer_pvar_s16(w, id, o) } else { c::dec_timer_pvar_i32(w, id, o) };
        if near {
            if c::len3(h.disp.map(|x| x.to_f32())) > 0.01 {
                let t = c::ticks(w, 120);
                set(w, l.seen, t);
            } else {
                dec(w, l.seen);
            }
        } else if get(w, l.seen) != 0 {
            set(w, l.seen, 0);
            c::set_pv4(w, id, l.glance, body);
        }
        if dec(w, l.glance_timer) != 0 {
            let t = w.svc.timing.scale(crate::ps2v::Pf::f(w.rng.randf(180.0, 300.0))).to_f32() as i32;
            set(w, l.glance_timer, t);
            let deg = f32::from_bits(0x3c8e_fa35);
            let yaw = c::add_rot(c::yaw(w, id), w.rng.randf(-90.0, 90.0) * deg);
            let pitch = w.rng.randf(0.0, 30.0) * deg;
            let g = [6.0 * yaw.cos() * pitch.cos(), 6.0 * yaw.sin() * pitch.cos(), 6.0 * pitch.sin(), 0.0];
            c::set_pv4(w, id, l.glance, c::add(g, [pos[0], pos[1], pos[2], 0.0]));
        }
        let target = if get(w, l.seen) != 0 {
            k = l.k_seen;
            body
        } else {
            c::pv4(w, id, l.glance)
        };
        let eye = [pos[0], pos[1], pos[2] + l.eye, pos[3]];
        let v = c::sub(target, eye);
        let half_pi = std::f32::consts::FRAC_PI_2;
        let yaw = c::sub_rot(c::atan(v[0], v[1]), c::yaw(w, id)).clamp(-half_pi, half_pi);
        let lim = f32::from_bits(0x3f06_0a92);
        let pitch = (-c::atan(c::len2(v), v[2])).clamp(-lim, lim);
        c::set_pf(w, id, l.yaw.0 + manip::rec::TARGET + 8, yaw * l.yaw_b);
        c::set_pf(w, id, l.pitch.0 + manip::rec::TARGET + 4, pitch * l.pitch_k);
        c::set_pf(w, id, l.pitch.0 + manip::rec::TARGET + 8, yaw * l.yaw_a);
    }
    (k, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cinematic::EngineRequest;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::services::LevelMissions;

    /// The head look-at: the first tick draws a glance (timer + 3 `randf`), Ratchet near, in front and moving sets the
    /// "seen" timer and the NPC looks at his body point with k 0.04 (both records linked, list 0 in pitch and yaw,
    /// list 1 in yaw); away, the last place he was seen becomes the glance point; talking (seq B ≠ 0) writes no
    /// targets.
    #[test]
    fn head_looks_at_ratchet_and_glances() {
        let mut m = Moby { o_class: 774, state: 7, pvars: vec![0; 0x170], ..Moby::default() };
        for o in [0x50, 0xd0] { p::set_ff(&mut m.pvars, o + manip::rec::REC_SCALE, 1.0); }
        let mut t = MobyTable::new(vec![m], 4);
        let mut hero = crate::hero::Hero::new();
        let v = |x: f32, y: f32, z: f32| crate::hero::physics::v4(x, y, z);
        hero.pos = v(3.0, 0.0, 0.0);
        hero.body_point = v(3.0, 0.0, 1.5);
        hero.disp = v(0.1, 0.0, 0.0);
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.joint_targets.insert(774, vec![8, 9]);
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        let mut r = *w.rng;
        look_at(&mut w, 0);
        // The glance: timer, yaw offset, pitch (no other draw).
        let n = r.randf(180.0, 300.0) as i32;
        r.randf(-90.0, 90.0);
        r.randf(0.0, 30.0);
        assert_eq!(w.rng.state, r.state);
        assert_eq!(c::pi32(&w, 0, LAYOUT.glance_timer), n);
        assert_eq!(c::pi32(&w, 0, LAYOUT.seen), 120);
        // Looking at the body point from the eye (0, 0, 1): yaw 0, pitch −atan2(0.5, 3); k 0.04 → the spring's first step.
        let pitch = -(0.5f32).atan2(3.0);
        let mut a = crate::ps2v::Pf::ZERO;
        let mut vv = crate::ps2v::Pf::ZERO;
        crate::hero::physics::turn_spring(crate::ps2v::Pf::f(pitch * 1.25), crate::ps2v::Pf::f(0.04), crate::ps2v::Pf::f(0.3), crate::ps2v::Pf::ZERO, &mut a, &mut vv, 0);
        assert_eq!(c::pf(&w, 0, 0x50 + manip::rec::ANGLES + 4), a.to_f32());
        assert_eq!(w.m(0).joint_mods.iter().map(|m| m.joint).collect::<Vec<_>>(), vec![8], "list 1 has no yaw target yet");
        // Ratchet behind the NPC (yaw 0 faces +x): the seen timer is cleared, his body point (where he is now) becomes
        // the glance point.
        hero.pos = v(-3.0, 0.0, 0.0);
        hero.body_point = v(-3.0, 0.0, 1.5);
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 1);
        look_at(&mut w, 0);
        assert_eq!(c::pi32(&w, 0, LAYOUT.seen), 0);
        assert_eq!(c::pv4(&w, 0, LAYOUT.glance), [-3.0, 0.0, 1.5, 0.0]);
        // Talking: no targets written (the records only spring).
        w.mm(0).anim.seq_b = 3;
        look_at(&mut w, 0);
        assert_eq!(c::pf(&w, 0, 0xb8), 0.0, "cleared by the record update, not rewritten");
    }

    /// State 2 after the talker's scene: its open mission is set done through the one mission writer and its
    /// checkpoint cuboid (+0x4c) becomes the checkpoint record (`FUN_0029ac10(centre, Euler)`); once done, nothing.
    #[test]
    fn scene_end_sets_mission_and_checkpoint() {
        let mut m = Moby { o_class: 774, mission: 1, state: 2, pvars: vec![0; 0x50], ..Moby::default() };
        p::set_i32(&mut m.pvars, 0x4c, 0);
        let mut t = MobyTable::new(vec![m], 4);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.level = 1;
        let (c, e) = ([1.0, 2.0, 3.0], [0.0, 0.0, 1.5]);
        svc.volumes = std::sync::Arc::new(rc_formats::volumes::Volumes { cuboids: vec![super::super::checkpoint::tests::cube(c, e)], ..Default::default() });
        let open = LevelMissions::fresh_load(1, [0; 16]);
        {
            let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
            w.missions = &open;
            update(&mut w, 0);
            assert_eq!(w.m(0).state, 1);
            assert_eq!(w.svc.cinematic.requests, [EngineRequest::MissionDone { mission: 1 }]);
            assert_eq!(w.svc.save.checkpoint, Some(super::super::checkpoint::Record { pos: c, rot: e }));
        }
        let mut done = open.clone();
        done.done[1] = 0xff;
        svc.cinematic.requests.clear();
        svc.save.checkpoint = None;
        t.mobys[0].state = 2;
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 1);
        w.missions = &done;
        update(&mut w, 0);
        assert!(w.svc.cinematic.requests.is_empty() && w.svc.save.checkpoint.is_none());
    }
}
