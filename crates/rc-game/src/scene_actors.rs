//! **The scene actors' per-tick extras** (level01): the calls the scene loops make on an actor after posing it, shared
//! by the mode-2 scenes (`CutsceneModeUpdate` 0x2aca80's actor loop: class 0 → `FUN_0024a1d0`, classes 10 / 0x1a3 /
//! 0x555 → `FUN_00228bc0`) and mode 6's take-off / landing (`crate::travel::space`: class 0 and class 10). Native `f32`.
//!
//! | address | what | port |
//! |---|---|---|
//! | `FUN_0024a1d0` | Ratchet's head: s = 0x15ee14 (his head scale, at most 1.33 in game mode 6); the record 0x17c7c0 not attached → `AttachManipulator(actor, 0x18, 0x17c7c0)`, its scale s; on levels 2, 9, 11, 13 every actor of class 0x1b1 / 0x50a / 0x509 takes its class scale · s; then `FUN_0024a0e8` (the worn items on the actor: NOT ported, G-CUT-009) | [`ratchet`] |
//! | `FUN_00228bc0` | mode \|= 0x10, the glow word +0x90 = `fast_sin(2π·(0x15f5cc % ticks(100))/ticks(100) − π)` → `(0x4c + 24s) << 16 \| 0x80000000 \| (0xa2 + 52s) << 8 \| (0x44 + 24s)`; the glow sprite `FUN_00229738`: class 0x1a3 at joint list 6 − 0.07 z when it is more than 2 above the actor (0.25, 0.15), else at joint list 1 − 0.007 z (0.047, 0.15), colour 0x200000c0; the cheat 0x15edb3: the noggin record (0x1a3: 0x17c840 on list 1; 0x555: 0x17c880; else 0x17c800, on list 2) attached once, scale 0x15ee18 | [`clank`] |
//!
//! `FUN_00229738` adds to the hero's glow list 0x141120 and registers the hero's draw callback (`0x229400`); the port
//! keeps the actors' sprites in [`ActorRecords::glow`] (the hero's own are `Hero::glow`), drawn with the list by the
//! callback `Callback::HeroGlow`.

use crate::hero::glow::{At, Sprite};
use crate::moby_runtime::MobyId;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::services::World;

/// The scene actor classes this module reads joint data of: Ratchet (0: [`ratchet`]'s head manipulator on his joint
/// list 0x18) and the Clank actors (10, 0x1a3, 0x555: [`clank`]'s antenna sprite on joint list 1, 0x1a3's 6). None
/// is in the loader's `Services::joint_lists` set (`needs_joint_lists`), so the loader fills them
/// (`rc-engine` `gameplay::wants_joint_lists`) — without them the antenna sprite sat at the actor's origin and
/// Ratchet's head scale and his helmet were never placed.
pub const ACTOR_CLASSES: [i32; 4] = [0, 10, 0x1a3, 0x555];

/// The key of the Ratchet actor's head manipulator (record 0x17c7c0) in its joint-modifier list.
pub const HEAD_KEY: u32 = 0xffff_ffe0;
/// The keys of the Clank actors' noggin manipulators (records 0x17c800 / 0x17c840 / 0x17c880).
pub const NOGGIN_KEYS: [u32; 3] = [0xffff_ffe1, 0xffff_ffe2, 0xffff_ffe3];

/// The actors' manipulator records (attached or clear) and their glow sprites of this tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActorRecords {
    /// 0x17c7c0 (byte +1: attached).
    pub head: bool,
    /// 0x17c800, 0x17c840, 0x17c880.
    pub noggin: [bool; 3],
    /// The sprites `FUN_00229738` added to the glow list 0x141120 this tick.
    pub glow: Vec<Sprite>,
}

impl ActorRecords {
    /// `DialogStreamStart` 0x2ac330: the five scene records cleared (0x17c8c0 is `Services::scene_head`).
    pub fn clear(&mut self) {
        self.head = false;
        self.noggin = [false; 3];
    }
}

/// `AttachManipulator(id, list, rec)` with the record's scale set to `scale` (all three axes).
fn attach(w: &mut World, id: MobyId, list: u8, key: u32, scale: f32) {
    let Some(joint) = crate::moby_update::manip::target_joint(w, id, list) else {
        w.svc.unported("scene actor manipulator: the actor's joint list not loaded");
        return;
    };
    let node = rc_formats::moby_anim::JointModifier { joint, mode: 0, weight: 0.0, quat: [0.0, 0.0, 0.0, 1.0], scale: [scale; 3], trans: [0.0; 3] };
    let m = w.mm(id);
    if m.joint_mod_keys.len() != m.joint_mods.len() { m.joint_mod_keys.resize(m.joint_mods.len(), u32::MAX); }
    m.joint_mods.insert(0, node);
    m.joint_mod_keys.insert(0, key);
}

/// `FUN_0024a1d0(actor)` (module doc) without its `FUN_0024a0e8`; `actors` = 0x16ce58[0..0x16cd24].
pub fn ratchet(w: &mut World, id: MobyId, actors: &[Option<MobyId>]) {
    let mut s = w.hero.idle.rec17_scale;
    if w.svc.game_mode == 6 && 1.33 < s { s = 1.33; }
    if !w.svc.actor_records.head {
        w.svc.actor_records.head = true;
        attach(w, id, 0x18, HEAD_KEY, s);
    }
    if matches!(w.svc.level, 2 | 9 | 11 | 13) {
        for &a in actors.iter().flatten() {
            let oc = w.m(a).o_class;
            if matches!(oc, 0x1b1 | 0x50a | 0x509) {
                let base = w.classes.info(oc).map_or(1.0, |i| i.scale);
                w.mm(a).scale = base * s;
            }
        }
    }
}

/// `FUN_00228bc0(actor)` (module doc); `anim`: the actor's class animation with the scene's streamed sequence (the
/// slot its +0x52 / +0x53 name), for `FUN_002645a8`'s joint points in the pose it is drawn with.
pub fn clank(w: &mut World, id: MobyId, anim: Option<&rc_formats::moby_anim::MobyAnimClass>) {
    let point = |w: &World, list: usize| w.joint_matrix_with(id, list, anim)[3];
    let period = w.ticks(100);
    let f = (w.counter as i64).rem_euclid(i64::from(period.max(1))) as f32 / period as f32;
    let s = crate::hero::physics::fast_sin(crate::ps2v::Pf::f((f + f) * std::f32::consts::PI - std::f32::consts::PI)).to_f32();
    let (a, b) = ((s * 24.0) as i32, (s * 52.0) as i32);
    let m = w.mm(id);
    m.mode |= 0x10;
    m.glow = ((a + 0x4c) as u32) << 16 | 0x8000_0000 | ((b + 0xa2) as u32) << 8 | (a + 0x44) as u32;
    let oc = w.m(id).o_class;
    let pull = f32::from_bits(0x3e19_999a);
    if oc == 0x1a3 {
        let mut p = point(w, 6);
        p[2] -= 0.07;
        if 2.0 < (w.m(id).position[2] - p[2]).abs() { push_sprite(w, f32::from_bits(0x3e80_0000), pull, p); }
    } else {
        let mut p = point(w, 1);
        p[2] -= 0.007;
        push_sprite(w, f32::from_bits(0x3d40_8312), pull, p);
    }
    let k = match oc {
        0x1a3 => 1,
        0x555 => 2,
        _ => 0,
    };
    if w.svc.cheats.on(crate::cheats::slot::CLANK) && !w.svc.actor_records.noggin[k] {
        w.svc.actor_records.noggin[k] = true;
        let list = if oc == 0x1a3 { 1 } else { 2 };
        let sc = w.hero.bodies.scale18;
        attach(w, id, list, NOGGIN_KEYS[k], sc);
    }
}

/// `FUN_00229738(size, pull, point, 0x200000c0)`: the sprite on the glow list, the hero's draw callback registered.
fn push_sprite(w: &mut World, size: f32, pull: f32, p: [f32; 4]) {
    w.svc.actor_records.glow.push(Sprite { size, pull, at: At::Point([p[0], p[1], p[2]]), rgba: 0x2000_00c0 });
    if let Some(h) = w.hero_moby { w.svc.draw_callbacks.register(Callback::HeroGlow, h); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};

    fn with(o_class: i16, f: impl FnOnce(&mut World, MobyId)) {
        let mut t = MobyTable::new(vec![Moby { o_class: 0, state: 1, ..Moby::default() }, Moby { o_class, state: 1, position: [0.0, 0.0, 10.0, 1.0], ..Moby::default() }], 4);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.joint_targets.insert(o_class, vec![3; 0x19]);
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        w.hero_moby = Some(0);
        f(&mut w, 1);
    }

    /// Ratchet's scene actor gets his head scale once per scene (record 0x17c7c0), not again on the next tick.
    #[test]
    fn the_ratchet_actor_takes_his_head_scale_once() {
        with(0, |w, id| {
            ratchet(w, id, &[Some(id)]);
            ratchet(w, id, &[Some(id)]);
            let m = w.m(id);
            assert_eq!(m.joint_mod_keys, [HEAD_KEY]);
            assert_eq!(m.joint_mods[0].scale, [f32::from_bits(0x3f6b_851f); 3]);
        });
    }

    /// A Clank actor glows (mode 0x10, the pulse word) and puts his antenna sprite on the glow list, 0.007 under his
    /// joint list 1's point, with the hero's draw callback registered.
    #[test]
    fn the_clank_actor_glows_and_lights_his_antenna() {
        with(10, |w, id| {
            // No joint list loaded: the point is the actor's origin (0, 0, 10).
            clank(w, id, None);
            let m = w.m(id);
            assert_ne!(m.mode & 0x10, 0);
            // counter 0: s = sin(−π) = 0 → (0x4c, 0xa2, 0x44).
            assert_eq!(m.glow, 0x804c_a244);
            let g = &w.svc.actor_records.glow;
            assert_eq!(g.len(), 1);
            assert_eq!(g[0].at, At::Point([0.0, 0.0, 10.0 - 0.007]));
            assert_eq!((g[0].size, g[0].rgba), (f32::from_bits(0x3d40_8312), 0x2000_00c0));
            assert!(w.svc.draw_callbacks.list1.contains(&(Callback::HeroGlow, 0)));
        });
    }

    /// A world whose class `o_class` has a two-joint skeleton (joint 1 one unit above the root, rest 1024 = 1.0 at
    /// scale 1) and joint list 1 = the chain 0 → 1, in `Services::joint_lists` and the class table — what the loader
    /// fills for the Clank actors (`rc-engine` `gameplay::wants_joint_lists`).
    fn with_joint_list(o_class: i16, f: impl FnOnce(&mut World, MobyId)) {
        use crate::moby_runtime::ClassInfo;
        use rc_formats::moby_anim::{MobyAnimClass, MobyFrame, MobyFrameHeader, MobySequence, MobySequenceHeader, IDENTITY};
        let q = [0u8, 0, 0, 0, 0, 0, 0xff, 0x7f];
        let frame = MobyFrame {
            header: MobyFrameHeader { rate: 0.0, time: 0, qwc: 1, quat_bytes: 16, scale_count: 0, trans_offset: 16, trans_count: 0 },
            quats: vec![[0, 0, 0, 0x7fff]; 2],
            scales: vec![],
            trans: vec![],
            payload: [q, q].concat(),
        };
        let seq = MobySequence { header: MobySequenceHeader { frame_count: 1, loop_sound: 0xff, ..Default::default() }, frames: vec![frame], triggers: vec![] };
        let class = MobyAnimClass {
            joint_count: 2,
            skeleton: vec![IDENTITY, IDENTITY],
            rest: vec![[0.0; 3], [0.0, 0.0, 1024.0]],
            parent_word: vec![0, 0x7000_0000],
            sequences: vec![Some(seq)],
        };
        let rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        let mut t = MobyTable::new(vec![Moby::default(), Moby { o_class, state: 1, position: [0.0, 0.0, 10.0, 1.0], scale: 1.0, rows, ..Moby::default() }], 4);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let mut classes = crate::moby_update::ClassTable::default();
        classes.classes.insert(o_class, (ClassInfo::default(), Some(class)));
        let mut svc = crate::moby_update::Services::new();
        svc.joint_lists.insert(o_class, vec![vec![], vec![0, 1]]);
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        w.hero_moby = Some(0);
        f(&mut w, 1);
    }

    /// With the class's joint lists loaded (`Services::joint_lists`), `FUN_00228bc0`'s antenna sprite sits at joint
    /// list 1's point (`FUN_002645a8`), above the actor — not at his origin (the bug when the loader left the Clank
    /// actors' classes out).
    #[test]
    fn the_clank_actors_antenna_sprite_sits_on_his_joint_list_1() {
        with_joint_list(10, |w, id| {
            clank(w, id, None);
            let g = &w.svc.actor_records.glow;
            assert_eq!(g.len(), 1);
            let At::Point(p) = g[0].at else { panic!("not a point") };
            assert_ne!(p, [0.0, 0.0, 10.0 - 0.007], "not the actor's origin");
            assert!(p[0].abs() < 1e-4 && p[1].abs() < 1e-4 && p[2] > 10.0, "on the joint above the actor: {p:?}");
        });
    }
}
