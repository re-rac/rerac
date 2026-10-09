//! **Aridia's story NPCs and item scenes** (level 02; the item scene code also on level 06): read from the level02
//! decomp and disassembly.
//!
//! * **786 "McGuffin Surfer"** (`0x2e0dc0`, census U114, one instance): Skid McMarx stranded among the sand sharks.
//!   Talk to him; after the talk his mission is done and a checkpoint is set; the talk's node 2 ends in his item movie
//!   (`mpegs[64 + 2]`, the level's item-movie player `0x298c68`), then the Sonic Summoner (item 5) and a save. He
//!   holds a prop (class 0x38) on his joint 2 and looks at Ratchet.
//! * **788 "Surfer Agent"** (`0x2e1950`, U115, one instance): Skid's agent. Inside his cuboid +0x148 the shark /
//!   nest counters (+0x14c / +0x150, kept by the sand sharks 580 and nests 668: `aridia_sandshark`) are shown; when
//!   both are 0 and his 2-second timer runs out: a short fade, he moves to cuboid +0x154, Ratchet stands 1.5 in front
//!   of him, the mobys +0x158 / +0x15c get command 1 and his talk jumps to node 2 (auto). After the talk's node 2:
//!   help 2000, the Hoverboard (item 30), his mission, a checkpoint at Ratchet, a save.
//! * **1005 / 1016 the item scenes** (`0x2ea210` level 02 = `0x2f4638` level 06, U118: the same code in both
//!   overlays; G-CUT-004's "item" scenes): a spinning, glowing item. Within 1 (XY) and 2 (z) of Ratchet, checked every
//!   tenth tick: hidden, scene 4 (1005, the Trespasser 26) or 2 (1016, the Hydrodisplacer 22). After the scene the
//!   item is given (equipped unless Clank / Giant Clank is the hero body; then the Hydrodisplacer's banner is cut);
//!   1016 saves and is gone; 1005 shows banner 2015 and help 2009, then a camera glide to the Trespasser's machine
//!   under a fade, puts Ratchet back, and saves.
//!
//! **System or not.** Per-class code on the shared systems: the talk system (`interact`), the NPC head look-at
//! (`talking_npc::look_at_layout`: these two are layout rows with their own eye, factors and timer widths), the
//! checkpoint record, the cinematic calls, the item glow (`gold_bolt::glow_init` / `item_glow`: the same code), the
//! story services (`story`).
//!
//! ## 786 coverage (level02 `0x2e0dc0`)
//!
//! | address | what | status |
//! |---|---|---|
//! | top | `0x2e0cd8`: in a scene with the big-head cheat, its manipulator (2.75) on this class's scene actors | `manip::scene_big_head` |
//! | state 0 | Sonic Summoner acquired (0x13d4ed) → `DeleteMoby`; else the debug name, → 1, `NpcTalkRegister` (talk block +0x00), the shadow slab z ± 0.2 (`0x25c758`), `CreateMoby(0x38)` → +0x178: draw distance 0x40, drawn, light word +0x38 copied, mode \|= 0x100 | [`surfer_update`] |
//! | state 1 | talk radius 255 when the node is auto and Ratchet in cuboid +0x154, else 3.7; `NpcTalkUpdate` → `PlaceAfterScene(2.2)`, → 2 | [`surfer_update`] |
//! | | Sonic Summoner not owned (0x13d4c5) and Ratchet within 3 (XY): help record 0x0e: count 0 → 1 + time / mask; else the 18-second reminder → `Help_Request(2004, 0x0e)` | [`surfer_update`] (`hints::arm_or_remind`) |
//! | | the idle: seq B 0 and the s16 timer +0x172 out → `ticks(randf(1200, 2400))`, blend to seq 1 over `ticks(10)` (unless already 1); seq B 1 at its end (+0x70 & 2) → blend to 0 | `story_npc::idle_anim` |
//! | state 2 | game mode ≠ 2: → 1; the node played (talk +0x04) 0: `SetMissionDone(+0xb0)`, the checkpoint at cuboid +0x150 (−1: none); node 2: hidden (+0x31 = 0, collision off, mode \|= 1), the prop hidden, the item movie 2 (`0x298c68`: `mpegs[66]`), → 5 | [`surfer_update`] (`cinematic::item_movie`) |
//! | | game mode 2, the scene-end place not set (0x16cea6): Ratchet goes 2.5 along row 0, grounded (`GroundHeight(0.5)`), facing the NPC (yaw + 0x40490fd0) | [`surfer_update`] (`story_npc::place_grounded`) [L: the port's place is set until the scene ends, so this runs only when none is pending] |
//! | states 3 / 4 | the camera glide between cuboids +0x158 / +0x15c (`0x25df98(1, 0.1·dt², 0.25·dt², 0.5·dt)` on +0x180 / +0x184, lerped position and Euler), the fade +0x17c up to 1, then `CameraScript2(0)` and → 4; 4: fade down, the prop deleted, `DeleteMoby` | [`surfer_update`] (no code in the class sets state 3: kept as in the game) |
//! | state 5 | `GiveItem(5, 1)`, `memcard_Save`, `DeleteMoby` | [`surfer_update`] |
//! | tail | the prop on joint 2: its rows the joint's (`moby_attach_to_joint`), its position the joint point + rows · (0.05, −0.05, −0.05) (gp−0x4f20), `MobyBuildMatrix` | [`hold_prop`] |
//! | tail | the head look-at (records +0x40 list 0 / +0xc0 list 1, glance +0x160, s16 timers +0x174 / +0x176, eye 1, pitch × 1, yaw 0.7 / 0.3, gated on seq B) | [`talking_npc::look_at_layout`] |
//! | tail | the big-head cheat: +0xb0 = 2.75 (the head record's scale), k / d × 0x15ed64 (1) | `talking_npc::look_springs` |
//! | | no sound, particle, light of its own | n/a |
//!
//! ## 788 coverage (level02 `0x2e1950`)
//!
//! | address | what | status |
//! |---|---|---|
//! | top | `0x2e1868` (the big-head scene manipulator) | `manip::scene_big_head` |
//! | `0x2e1fb8` | race state +0x180 = 0 and Ratchet in cuboid +0x148 → 1, the HUD counters `queue_animation_update(0x15, 2000, …, +0x14c, 100)` / `(0x17, 2001, …, +0x150, 7)` (handles +0x164 / +0x160) | the state: [`agent_race`]; the two level-02 HUD elements (`0x23c990` / `0x23ccc8` / `0x23cda8`): NOT ported (G-UI-011) |
//! | | 1: out of the cuboid → 0, both elements released (`0x238f50(h, 0)`), handles −1; counters 0 and the s16 timer +0x168 out → 2, `FadeToBlack(ticks(16))`, the elements released, the agent at cuboid +0x154's centre facing its row 0, `HeroTeleport(1.5 in front, facing him, 0, 1)`, mobys +0x158 / +0x15c +0xbc = 1, talk auto (+0x08) = 1, node (+0x36) = 2 | [`agent_race`] |
//! | state 0 | Hoverboard acquired (0x13d506) → `DeleteMoby`; +0x180 = 0, the name, → 1, update distance 0xff, `NpcTalkRegister`, the shadow slab | [`agent_update`] |
//! | state 1 | talk radius 255 (auto, in cuboid +0x16c) else 2.5; `NpcTalkUpdate` → `PlaceAfterScene(2.5)`, → 2; the node played = 2: `Help_Request(2000, 10)`, `GiveItem(30, 1)`, `SetMissionDone(+0xb0)`, the checkpoint at Ratchet (0x13f3d0 / 0x13f3e0), `memcard_Save`, `DeleteMoby` | [`agent_update`] |
//! | | the idle: seq B 0 and the s16 timer +0x16a out → `ticks(randf(1200, 2400))`, seq `randi(2) = 0 ? 2 : 1` (unless already), blend `ticks(10)`; at the end of a seq ≠ 0 → 0 | `story_npc::idle_anim` |
//! | state 2 | the grounded scene-end place (probed from 10 up), then game mode ≠ 2 → 1 | [`agent_update`] |
//! | tail | the look-at (records +0x40 / +0xc0, glance +0x170, s32 timers +0x188 / +0x18c, eye 2, yaw 0.6 / 0.4) | [`talking_npc::look_at_layout`] |
//! | tail | the big-head cheat (the head record's scale 2.75) | `talking_npc::look_springs` |
//!
//! ## 1005 / 1016 coverage (level02 `0x2ea210`)
//!
//! | address | what | status |
//! |---|---|---|
//! | head | the item: 0x3ed (1005) → 26, else 22; the turn rate π / `ticks(60)` | [`item_scene_update`] |
//! | state 0 | the glow init (`0x2ea7d0` = the gold bolt's `glow_init`); item acquired → `DeleteMoby`; else z = ground (`GroundHeight(0.5)`) + 0.2, rot x π/2, rot y π, → 1 | [`item_scene_update`] |
//! | state 1 | yaw += π / `ticks(60)`; the glow (`0x2ea8b0` = the gold bolt's `item_glow` at z + 0.5); every 10th tick, Ratchet within 1 (XY) and \|Δz\| < 2: → 3, hidden, collision off, `DialogStreamStart(4)` (1005) / `(2)` (1016) | [`item_scene_update`] |
//! | state 3 | game mode ≠ 2: body 0 → `GiveItem(item, 1)`; else `GiveItem(item, 0)` and for 22 the banner countdown 0x15f640 = 0; 1005: `ShowBanner(2015, −1)`, the help box killed, `Help_Request(2009, 0x13)`, fade 0x15f3fc = 1, → 4, `HeroTeleport(here, 0x72, 1)`; 1016: `memcard_Save`, `DeleteMoby` | [`item_scene_update`] |
//! | state 4 | the camera glide (91.2, 270.21, 64.85) / (0.07, 0.33, 0.55) → (106.98, 273.46, 62.65) / (0.07, 0.43, 0.72) on `0x25df98(1, 0.15·dt², 0.15·dt², 0.15·dt)` (+0x40 / +0x44); `CameraScript(…, 1, 0, 0)` unless the script camera is up; the targets; the fade to 0 while a help box is up / pending or t ≤ 0.2, else to 1 (4·dt); at 1: `HeroTeleport(here, 0, 0)`, `CameraScript2(0)`, → 5 | [`item_scene_update`] (`cinematic::camera_script_unless_script`) |
//! | state 5 | the fade down (4·dt); at 0: `memcard_Save`, `DeleteMoby` | [`item_scene_update`] |
//! | | no sound of its own (the scene carries it) | n/a |

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::gold_bolt;
use crate::moby_update::classes::talking_npc::{look_at_layout, LookLayout};
use crate::moby_update::creature::{self as c, turn, DT, DT2};
use crate::moby_update::interact::{self, talk};
use crate::moby_update::services::{pvar as p, World};
use crate::moby_update::story;
use crate::ps2v::Pf;
use super::story_npc::{self, Idle};

pub const REFERENCE_LEVEL: u32 = 2;
pub const SURFER_FN: u32 = 0x2e_0dc0;
pub const SURFER_CLASSES: [i16; 1] = [786];
pub const AGENT_FN: u32 = 0x2e_1950;
pub const AGENT_CLASSES: [i16; 1] = [788];
pub const ITEM_SCENE_FN: u32 = 0x2e_a210;
pub const ITEM_SCENE_CLASSES: [i16; 2] = [1005, 1016];

/// The surfer's held prop.
const PROP_CLASS: i16 = 0x38;
/// gp−0x4f20: the prop's offset in the joint's frame.
const PROP_OFFSET: [f32; 3] = [0.05, -0.05, -0.05];

const SURFER_LOOK: LookLayout = LookLayout { pitch: (0x40, 0), yaw: (0xc0, 1), glance: 0x160, seen: 0x174, glance_timer: 0x176, gate_seq_b: true, eye: 1.0, pitch_k: 1.0, yaw_a: 0.7, yaw_b: 0.3, short_timers: true, gate_main: 0, gate_alt: 0xff, k_seen: 0.04 };
const SURFER_IDLE: Idle = Idle { timer: 0x172, short: true, rest: story_npc::rest0, pick: |_| 1, at_end: |_, s| (s == 1).then_some(0) };
const AGENT_IDLE: Idle = Idle { timer: 0x16a, short: true, rest: story_npc::rest0, pick: |w| if w.rng.randi(2) == 0 { 2 } else { 1 }, at_end: story_npc::back0 };
const AGENT_LOOK: LookLayout = LookLayout { pitch: (0x40, 0), yaw: (0xc0, 1), glance: 0x170, seen: 0x188, glance_timer: 0x18c, gate_seq_b: true, eye: 2.0, pitch_k: 1.0, yaw_a: 0.6, yaw_b: 0.4, short_timers: false, gate_main: 0, gate_alt: 0xff, k_seen: 0.04 };

fn hide_self(w: &mut World, id: MobyId) {
    let m = w.mm(id);
    m.visible = 0;
    m.has_collision = false;
    m.mode |= mode::HIDDEN;
}

/// The surfer's prop on its joint 2 (module doc, tail).
fn hold_prop(w: &mut World, id: MobyId) {
    let Some(prop) = story::link(w, p::i32(&w.m(id).pvars, 0x178)).filter(|&q| q != 0 && story::alive(w, q)) else { return };
    let jp = w.joint_point(id, 2);
    let mx = w.joint_matrix(id, 2);
    let off: [f32; 3] = std::array::from_fn(|l| mx[0][l] * PROP_OFFSET[0] + mx[1][l] * PROP_OFFSET[1] + mx[2][l] * PROP_OFFSET[2]);
    let m = w.mm(prop);
    m.rows[0] = mx[0];
    m.rows[1] = mx[1];
    m.rows[2] = mx[2];
    m.position = [jp[0] + off[0], jp[1] + off[1], jp[2] + off[2], jp[3]];
    w.build_matrix(prop);
}

/// Level02 `0x2e0dc0` (786; module doc).
pub fn surfer_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x188 { w.mm(id).pvars.resize(0x188, 0); }
    interact::poll_scene_end(w, id);
    // 0x2e0cd8: the actors' big-head cheat on the scene actors of [786] (list 0, 2.75).
    crate::moby_update::manip::scene_big_head(w, &[786], 0, 2.75);
    match w.m(id).state {
        0 => {
            if w.svc.interact.game.acquired.get(5).is_some_and(|&b| b != 0) {
                w.delete_moby(id);
                return;
            }
            w.mm(id).state = 1;
            interact::talk_register(w, id);
            let m = w.mm(id);
            m.shadow_hi = m.position[2] + 0.2;
            m.shadow_lo = m.position[2] - 0.2;
            match w.create_moby(PROP_CLASS) {
                Some(q) => {
                    let (light, ambient) = (w.m(id).light, w.m(id).ambient);
                    let m = w.mm(q);
                    m.draw_dist = 0x40;
                    m.visible = 1;
                    m.light = light;
                    m.ambient = ambient;
                    m.mode |= mode::KEEP_ROWS;
                    p::set_i32(&mut w.mm(id).pvars, 0x178, q as i32);
                }
                None => p::set_i32(&mut w.mm(id).pvars, 0x178, 0),
            }
        }
        1 => {
            story_npc::talk_radius(w, id, 0, 0x154, f32::from_bits(0x406c_cccd));
            if interact::talk_update(w, id) {
                interact::place_after_scene(w, id, f32::from_bits(0x400c_cccd));
                w.mm(id).state = 2;
            } else if !w.inventory.owned(5) && c::dist2(c::pos(w, id), story::hero4(w)) < 3.0 {
                super::hints::arm_or_remind(w, 0xe, 0x7d4, 0xe);
            }
            story_npc::idle_anim(w, id, &SURFER_IDLE);
        }
        2 => {
            if w.svc.game_mode != 2 && w.svc.interact.talker != Some(id) {
                w.mm(id).state = 1;
                match p::i16(&w.m(id).pvars, talk::LAST) {
                    0 => {
                        let mission = w.m(id).mission;
                        crate::cinematic::set_mission_done(w, mission);
                        let cb = p::i32(&w.m(id).pvars, 0x150);
                        story::checkpoint_at(w, cb);
                    }
                    2 => {
                        hide_self(w, id);
                        if let Some(q) = story::link(w, p::i32(&w.m(id).pvars, 0x178)).filter(|&q| q != 0) {
                            let m = w.mm(q);
                            m.mode |= mode::HIDDEN;
                            m.visible = 0;
                        }
                        crate::cinematic::item_movie(w, 2);
                        w.mm(id).state = 5;
                    }
                    _ => {}
                }
            } else if w.svc.game_mode == 2 {
                story_npc::place_grounded(w, id, 2.5, 0.0);
            }
        }
        3 => {
            glide(w, id, 0x158, 0x15c, 0x180, 0x184);
            let mut f = c::pf(w, id, 0x17c);
            if c::pf(w, id, 0x180) < 1.0 {
                turn::approach(0.0, 0.1, &mut f);
            } else {
                turn::approach(1.0, 0.1, &mut f);
                if f == 1.0 {
                    w.mm(id).state = 4;
                    crate::cinematic::camera_script2(w, 0);
                }
            }
            c::set_pf(w, id, 0x17c, f);
            crate::cinematic::set_fade(w, f);
        }
        4 => {
            let mut f = c::pf(w, id, 0x17c);
            turn::approach(0.0, 0.1, &mut f);
            c::set_pf(w, id, 0x17c, f);
            crate::cinematic::set_fade(w, f);
            if f == 0.0 {
                if let Some(q) = story::link(w, p::i32(&w.m(id).pvars, 0x178)).filter(|&q| q != 0) { w.delete_moby(q); }
                w.delete_moby(id);
                return;
            }
        }
        5 => {
            interact::give_item(w, 5, true);
            crate::cinematic::save(w);
            w.delete_moby(id);
            return;
        }
        _ => {}
    }
    hold_prop(w, id);
    look_at_layout(w, id, &SURFER_LOOK);
}

/// The camera glide of 786's states 3 / 4: `t` springs to 1 (`0x25df98(1, 0.1·dt², 0.25·dt², 0.5·dt)`), the camera
/// targets the lerp of cuboid `a`'s and `b`'s centres and the per-axis rotation lerp of their Euler.
fn glide(w: &mut World, id: MobyId, a: usize, b: usize, t_off: usize, v_off: usize) {
    let (mut t, mut v) = (c::pf(w, id, t_off), c::pf(w, id, v_off));
    turn::spring(1.0, DT2 * 0.1, DT2 * 0.25, DT * 0.5, &mut t, &mut v);
    c::set_pf(w, id, t_off, t);
    c::set_pf(w, id, v_off, v);
    let (Some(ca), Some(cb)) = (story::cuboid(w, p::i32(&w.m(id).pvars, a)), story::cuboid(w, p::i32(&w.m(id).pvars, b))) else { return };
    let pos: [f32; 3] = std::array::from_fn(|l| ca.0[l] + (cb.0[l] - ca.0[l]) * t);
    let e: [f32; 3] = std::array::from_fn(|l| c::lerp_rot(ca.1[l], cb.1[l], t));
    crate::cinematic::camera_targets(w, Some(pos), Some(e));
}

/// `0x2e1fb8`: the agent's race watch (module doc).
fn agent_race(w: &mut World, id: MobyId) {
    let pv = |w: &World, o: usize| p::i32(&w.m(id).pvars, o);
    match pv(w, 0x180) {
        1 => {
            if !story::hero_in(w, pv(w, 0x148)) {
                p::set_i32(&mut w.mm(id).pvars, 0x180, 0);
                p::set_i32(&mut w.mm(id).pvars, 0x160, -1);
                p::set_i32(&mut w.mm(id).pvars, 0x164, -1);
                w.svc.unported("aridia agent 788: HUD counters 0x23c990 released (G-UI-011)");
            } else if pv(w, 0x14c) == 0 && pv(w, 0x150) == 0 && c::dec_timer_pvar_s16(w, id, 0x168) != 0 {
                p::set_i32(&mut w.mm(id).pvars, 0x180, 2);
                let n = w.ticks(0x10);
                crate::cinematic::fade_to_black(w, n);
                p::set_i32(&mut w.mm(id).pvars, 0x160, -1);
                p::set_i32(&mut w.mm(id).pvars, 0x164, -1);
                if let Some((centre, _)) = story::cuboid(w, pv(w, 0x154)) {
                    let row0 = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, pv(w, 0x154)).map(|s| s.matrix[0]);
                    let yaw = row0.map_or(0.0, |r| c::atan(r[0], r[1]));
                    let m = w.mm(id);
                    m.position = [centre[0], centre[1], centre[2], m.position[3]];
                    m.rotation[2] = yaw;
                    let at = [centre[0] + yaw.cos() * 1.5, centre[1] + yaw.sin() * 1.5, centre[2]];
                    crate::cinematic::hero_teleport(w, at, [0.0, 0.0, c::add_rot(yaw, std::f32::consts::PI)], 0, true);
                }
                for o in [0x158, 0x15c] {
                    if let Some(m) = story::link(w, pv(w, o)) { w.mm(m).cmd = 1; }
                }
                let pvm = &mut w.mm(id).pvars;
                p::set_u8(pvm, talk::AUTO, 1);
                p::set_i16(pvm, talk::NODE, 2);
            }
        }
        0 if story::hero_in(w, pv(w, 0x148)) => {
            p::set_i32(&mut w.mm(id).pvars, 0x180, 1);
            w.svc.unported("aridia agent 788: HUD counters 0x23c990 (G-UI-011)");
        }
        _ => {}
    }
}

/// Level02 `0x2e1950` (788; module doc).
pub fn agent_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x190 { w.mm(id).pvars.resize(0x190, 0); }
    interact::poll_scene_end(w, id);
    // 0x2e1868: the actors' big-head cheat on the scene actors of [788] (list 0, 2.75).
    crate::moby_update::manip::scene_big_head(w, &[788], 0, 2.75);
    agent_race(w, id);
    match w.m(id).state {
        0 => {
            if w.svc.interact.game.acquired.get(30).is_some_and(|&b| b != 0) {
                w.delete_moby(id);
                return;
            }
            p::set_i32(&mut w.mm(id).pvars, 0x180, 0);
            let m = w.mm(id);
            m.state = 1;
            m.update_dist = 0xff;
            interact::talk_register(w, id);
            let m = w.mm(id);
            m.shadow_hi = m.position[2] + 0.2;
            m.shadow_lo = m.position[2] - 0.2;
        }
        1 => {
            story_npc::talk_radius(w, id, 0, 0x16c, 2.5);
            if interact::talk_update(w, id) {
                interact::place_after_scene(w, id, 2.5);
                w.mm(id).state = 2;
            }
            if p::i16(&w.m(id).pvars, talk::LAST) == 2 {
                w.svc.help.request(2000, 10);
                interact::give_item(w, 30, true);
                let mission = w.m(id).mission;
                crate::cinematic::set_mission_done(w, mission);
                let (pos, yaw) = (crate::hero::physics::to_f32x3(w.hero.pos), w.hero.yaw().to_f32());
                crate::moby_update::classes::checkpoint::record(w, crate::moby_update::classes::checkpoint::Record { pos, rot: [0.0, 0.0, yaw] });
                crate::cinematic::save(w);
                w.delete_moby(id);
                return;
            }
            story_npc::idle_anim(w, id, &AGENT_IDLE);
        }
        2 => {
            story_npc::place_grounded(w, id, 2.5, 10.0);
            if w.svc.game_mode != 2 && w.svc.interact.talker != Some(id) { w.mm(id).state = 1; }
        }
        _ => {}
    }
    look_at_layout(w, id, &AGENT_LOOK);
}

/// The item of an item-scene moby (class 1005: the Trespasser 26; else the Hydrodisplacer 22).
fn scene_item(o_class: i16) -> usize { if o_class == 0x3ed { 0x1a } else { 0x16 } }

/// Level02 `0x2ea210` / level06 `0x2f4638` (1005 / 1016; module doc).
pub fn item_scene_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x48 { w.mm(id).pvars.resize(0x48, 0); }
    let oc = w.m(id).o_class;
    let item = scene_item(oc);
    let rate = std::f32::consts::PI / w.ticks(60) as f32;
    match w.m(id).state {
        0 => {
            gold_bolt::glow_init(w, id, 0);
            if w.svc.interact.game.acquired.get(item).is_some_and(|&b| b != 0) {
                w.delete_moby(id);
                return;
            }
            let g = w.ground_height(Pf::f(0.5), w.m(id).position.map(Pf::f), 0).to_f32();
            let m = w.mm(id);
            m.rotation[1] = std::f32::consts::PI;
            m.rotation[0] = std::f32::consts::FRAC_PI_2;
            m.state = 1;
            m.position[2] = g + 0.2;
        }
        1 => {
            let y = c::add_rot(c::yaw(w, id), rate);
            c::set_yaw(w, id, y);
            let mut centre = w.m(id).position;
            centre[2] += 0.5;
            gold_bolt::item_glow(w, id, 0, [centre[0], centre[1], centre[2]], 1.0, true);
            if !w.counter.is_multiple_of(10) { return; }
            let h = story::hero4(w);
            if 1.0 <= c::dist2(c::pos(w, id), h) || 2.0 <= (w.m(id).position[2] - h[2]).abs() { return; }
            w.mm(id).state = 3;
            hide_self(w, id);
            crate::cinematic::start_scene(w, if oc == 0x3ed { 4 } else { 2 }, false);
        }
        3 => {
            if w.svc.game_mode == 2 { return; }
            if w.body() == 0 {
                interact::give_item(w, item, true);
            } else {
                interact::give_item(w, item, false);
                // The Hydrodisplacer's banner cut (0x15f640 = 0).
                if item == 0x16 { w.svc.cinematic.banner.ticks = 0; }
            }
            if oc == 0x3ed {
                let t = w.ticks(180);
                crate::cinematic::show_banner(w, 0x7df, t);
                w.svc.help.kill();
                w.svc.help.request(0x7d9, 0x13);
                crate::cinematic::set_fade(w, 1.0);
                w.mm(id).state = 4;
                let (pos, yaw) = (crate::hero::physics::to_f32x3(w.hero.pos), w.hero.yaw().to_f32());
                crate::cinematic::hero_teleport(w, pos, [0.0, 0.0, yaw], 0x72, true);
                return;
            }
            crate::cinematic::save(w);
            w.delete_moby(id);
        }
        4 => {
            const A: ([f32; 3], [f32; 3]) = ([91.2, 270.21, 64.85], [0.07, 0.33, 0.55]);
            const B: ([f32; 3], [f32; 3]) = ([106.98, 273.46, 62.65], [0.07, 0.43, 0.72]);
            let (mut t, mut v) = (c::pf(w, id, 0x40), c::pf(w, id, 0x44));
            turn::spring(1.0, DT2 * 0.15, DT2 * 0.15, DT * 0.15, &mut t, &mut v);
            c::set_pf(w, id, 0x40, t);
            c::set_pf(w, id, 0x44, v);
            let pos: [f32; 3] = std::array::from_fn(|l| A.0[l] + (B.0[l] - A.0[l]) * t);
            let e: [f32; 3] = std::array::from_fn(|l| c::lerp_rot(A.1[l], B.1[l], t));
            crate::cinematic::camera_script_unless_script(w, pos, e, 1, 0, false);
            crate::cinematic::camera_targets(w, Some(pos), Some(e));
            let mut f = w.svc.cinematic.fade;
            let down = !w.svc.help.idle() || t <= 0.2;
            turn::approach(if down { 0.0 } else { 1.0 }, DT * 4.0, &mut f);
            crate::cinematic::set_fade(w, f);
            if f == 1.0 {
                let (pos, yaw) = (crate::hero::physics::to_f32x3(w.hero.pos), w.hero.yaw().to_f32());
                crate::cinematic::hero_teleport(w, pos, [0.0, 0.0, yaw], 0, false);
                crate::cinematic::camera_script2(w, 0);
                w.mm(id).state = 5;
            }
        }
        5 => {
            let mut f = w.svc.cinematic.fade;
            turn::approach(0.0, DT * 4.0, &mut f);
            crate::cinematic::set_fade(w, f);
            if f != 0.0 { return; }
            crate::cinematic::save(w);
            w.delete_moby(id);
        }
        _ => {}
    }
}
