//! The title world's scene (G-SAV-012): the space flight behind the title and the main menu, the boot's update
//! `fun_001eb0a8` 0x1eb0a8 (its world part) with `transition_update_movie_camera` 0x1eaf88 and the scene sounds of
//! `transition_do_transition` 0x1eb798's loop. The world itself (its tfrags, ties, shrubs, sky, the actors' classes)
//! is the title lump's (`rc_formats::frontend::TitleWorld`), drawn by `rc-engine`'s `title_world`; the boot flow
//! around it is [`crate::frontend`].
//!
//! The scene is the space-scene chunk format of the take-off / landing scenes and the flight
//! (`parse_space_scene_chunk` 0x2049f0: the boot's copy of the levels' `FUN_00259288`), played in a loop.
//!
//! | address | what | here |
//! |---|---|---|
//! | `transition_load_wad` 0x1ea830 | the chunk table (`hdr[0x20]`, ≤ 70 chunks), `parse_space_scene_chunk(0)`; 0x15ef58 = 0 | [`TitleScene::new`] |
//! | 0x2049f0 / `update_world_object_animation` 0x204790 | the chunk decompressed, chunk tick 0x18cb58 = 0, the end tick 0x18cb60, the actor count 0x18cb64, the camera table 0x18cb74; per actor: created on first sight (`create_moby(class)`; class 0x215 → `0x160488[ship]` only in boot mode 6, never on the title), the class's streamed slot (+0x52 / +0x53 = its sequence count, the count +1), +0x32 = 0x1ff, +0x72 = 0xff, mode \|= 6, +0x94 = 0, the light words 0x38383800000000 (no hero on the title), +0x73 = 0x18 when class +6; the position track +0x78; the slot's sequence pointer | [`TitleScene::actors`] (the engine creates the actors' draws: `ACTOR_AMBIENT`) |
//! | `fun_001eb0a8` | `noop_callback_i`; the fade 0x15f43c −= 0.0625 (the front end's, [`crate::frontend::FrontEnd`]); chunk tick +1, tick 0x18cb54 +1; tick < end: chunk tick > 0x5f → the next chunk; else chunk 0, tick 0 (the loop) | [`TitleScene::tick`] |
//! | `transition_update_movie_camera` | tan(hfov/2) 0x18cdb0 = 0.63 (not the record's); the eye and the rotation x, y, z of the record at the chunk tick (0x20-byte records) | [`TitleScene::tick`] (`scene_player::scene_camera`, tan 0.63) |
//! | `fun_001eb0a8` actors | per actor: frames chunk tick >> 1, +1; `update_moby_animation_state`; t = (chunk tick & 1)·0.5; position = the track's lerp (no ship frame); +0x71 = 0xff; `moby_build_rotation`; +0x7f = 0 (no shadow); +0xa6 == 0 → `noop_callback_e` | [`TitleScene::tick`] ([`ActorPose`]) |
//! | `noop_callback_h` | nothing | n/a |
//! | `transition_default_draw` mode 3 | `fun_002196b8` instead of `draw_mobys`: `draw_moby_list(0x15ff18, 4)` (`moby_proc` over the first 4 moby slots: end = start + 4·0x100), then the page menu's frame mobys. The table holds the actors from slot 0 in chunk-0 order (`init_mem_slots`, then `parse_space_scene_chunk(0)`), so actors 0..3 stay drawn under the main menu and the fifth does not | [`TitleScene::actors_drawn`] |
//! | 0x1eb798 loop, before the update | the scene sounds (boot 0x1862b0, [`TITLE_SOUNDS`]): per entry `{start, end, def, slot}` until start = −1: end = −1: tick < start → slot −1; else no slot → `allocate_voice_for_group_entry(def, 0, 0)`; otherwise tick < start or tick > end: the slot released when it still plays the def (`is_active_state_entry`), slot −1; else the def started again when its slot no longer plays it | [`TitleScene::sounds`] |
//! | 0x1eb798 attract | after the attract movie: tick 0, chunk 0, `parse_space_scene_chunk(0)` | [`TitleScene::restart`] |

use crate::audio::AudioSystem;
use crate::scene_player::SceneCamera;
use rc_formats::scene::Scene;
use std::sync::Arc;

/// `transition_update_movie_camera`: tan(hfov/2) 0x18cdb0 = 0x3f2147ae.
pub const TITLE_TAN: f32 = 0.63;
/// Chunk ticks (NTSC).
pub const CHUNK_TICKS: i32 = 0x60;
/// The actors' light words without a hero (`0x38383800000000`): light word 0, ambient (0x38, 0x38, 0x38).
pub const ACTOR_AMBIENT: [u8; 3] = [0x38, 0x38, 0x38];

/// The scene sounds (boot 0x1862b0): `(start tick, end tick, boot sound def)`. The defs are the boot's level sound
/// defs 0x186100 (`0x15f634`, 7 of them: `0x15f630`), the global sound bank's sounds 0, 0, 8, 9, 2, 3, 4.
pub const TITLE_SOUNDS: [(i32, i32, usize); 5] = [(0, 99_999_999, 2), (0, 99_999_999, 3), (0, 99_999_999, 0), (0x2e4, 0x4d8, 4), (0x114, 0x12c, 1)];
/// `0x15f5b4`: the base `allocate_voice_for_group_entry` adds to a scene sound's def (and `is_active_state_entry`
/// compares against), 2 on the boot (its initial data; nothing in the boot writes it). So the table's defs 2, 3, 0, 4, 1
/// play level defs 4, 5, 2, 6, 3 = bank sounds 2 (the random critter program), 3 (the modulated loop), 8 (the cricket
/// loop), 4 (Ratchet's tool bursts, ticks 0x2e4..0x4d8) and 9 (the cog pick-up, ticks 0x114..0x12c).
pub const DEF_BASE: usize = 2;

/// One actor's pose this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorPose {
    pub actor: usize,
    pub class: i32,
    pub chunk: usize,
    pub frame_a: u8,
    pub frame_b: u8,
    pub t: f32,
    pub position: [f32; 3],
}

/// The title scene's state (boot 0x18cb54.. while the title runs).
#[derive(Clone, Debug)]
pub struct TitleScene {
    pub scene: Arc<Scene>,
    /// 0x18cb54 / 0x18cb5c / 0x18cb58.
    pub tick: i32,
    pub chunk: usize,
    pub chunk_tick: i32,
    /// The scene sounds' slots (boot 0x1862bc + 0x10·k; −1 none).
    pub slots: [i32; TITLE_SOUNDS.len()],
}

/// One tick's output.
#[derive(Clone, Debug, Default)]
pub struct TitleFrame {
    pub camera: Option<SceneCamera>,
    pub actors: Vec<ActorPose>,
}

impl TitleScene {
    /// `transition_load_wad`'s `parse_space_scene_chunk(0)`.
    pub fn new(scene: Arc<Scene>) -> TitleScene { TitleScene { scene, tick: 0, chunk: 0, chunk_tick: 0, slots: [-1; TITLE_SOUNDS.len()] } }

    /// The scene's end tick (0x18cb60).
    pub fn end(&self) -> i32 { self.scene.chunks.first().map_or(0, |c| c.header.end_tick as i32) }

    /// The attract movie's end (0x1eb798): tick 0, chunk 0 (its chunk tick 0).
    pub fn restart(&mut self) {
        self.tick = 0;
        self.chunk = 0;
        self.chunk_tick = 0;
    }

    /// `fun_002196b8`'s `draw_moby_list` count: the moby slots mode 3 still draws.
    pub const MENU_DRAWN_ACTORS: usize = 4;

    /// The draws of `transition_default_draw`: every actor outside boot mode 3; in mode 3 (the page menu) the first
    /// [`Self::MENU_DRAWN_ACTORS`] moby slots = actors 0..3 (module docs).
    pub fn actors_drawn(mode: i32, actor: usize) -> bool { mode != 3 || actor < Self::MENU_DRAWN_ACTORS }

    /// The classes of the actors (chunk 0 order = the slot order).
    pub fn actors(&self) -> Vec<i32> { self.scene.actor_classes() }

    /// `fun_001eb0a8`'s world part and `transition_update_movie_camera` (module docs).
    pub fn tick(&mut self) -> TitleFrame {
        self.chunk_tick += 1;
        self.tick += 1;
        if self.tick < self.end() {
            if 0x5f < self.chunk_tick && self.chunk + 1 < self.scene.chunks.len() {
                self.chunk += 1;
                self.chunk_tick = 0;
            }
        } else {
            self.restart();
        }
        let mut out = TitleFrame::default();
        let Some(chunk) = self.scene.chunks.get(self.chunk) else { return out };
        if let Some(rec) = chunk.camera.get(self.chunk_tick.max(0) as usize) {
            let mut cam = crate::scene_player::scene_camera(rec);
            cam.tan_half_fov = TITLE_TAN;
            out.camera = Some(cam);
        }
        let f = (self.chunk_tick >> 1) as usize;
        let t = (self.chunk_tick & 1) as f32 * 0.5;
        for (k, a) in chunk.actors.iter().enumerate() {
            let p0 = a.positions.get(f).copied().unwrap_or([0.0; 4]);
            let p1 = a.positions.get(f + 1).copied().unwrap_or(p0);
            let position = std::array::from_fn(|i| p0[i] * (1.0 - t) + p1[i] * t);
            out.actors.push(ActorPose { actor: k, class: a.class, chunk: self.chunk, frame_a: f as u8, frame_b: (f + 1) as u8, t, position });
        }
        out
    }

    /// The scene sounds of this vsync (module docs), on `audio` whose level defs are the boot's (`listener` = the
    /// title camera; the plays have no owner and no position: 2-D at the listener).
    pub fn sounds(&mut self, audio: &mut AudioSystem, listener: &crate::audio::voices::Listener, rng: &mut crate::rng::Rng) {
        use crate::audio::voices::state;
        // is_active_state_entry 0x1eb740: the slot remembers def + 0x15f5b4 ([`DEF_BASE`]) and plays (state 1 or 2).
        let playing = |audio: &AudioSystem, slot: i32, def: usize| {
            usize::try_from(slot).ok().and_then(|i| audio.slots.slots.get(i)).is_some_and(|s| s.class_index as usize == def + DEF_BASE && matches!(s.state, state::PLAYING | state::CONFIRMED))
        };
        // allocate_voice_for_group_entry 0x22dba0 (def, 0, 0): def < 0x15f630 → sound_slot_alloc(def, 0, no owner, no
        // position, 0x400), the slot remembers the def.
        let play = |audio: &mut AudioSystem, def: usize, rng: &mut crate::rng::Rng| -> i32 {
            let def = def + DEF_BASE;
            let Some(d) = audio.data.sounds.level_defs.get(def).copied() else { return -1 };
            let k = audio.slots.play(&d, 0, None, None, None, 0x400, listener, rng);
            if k >= 0 { audio.slots.slots[k as usize].class_index = def as u16; }
            k
        };
        for (k, &(start, end, def)) in TITLE_SOUNDS.iter().enumerate() {
            let slot = self.slots[k];
            if end == -1 {
                if self.tick < start {
                    self.slots[k] = -1;
                } else if slot == -1 {
                    self.slots[k] = play(audio, def, rng);
                }
                continue;
            }
            if start <= self.tick && self.tick <= end {
                if !playing(audio, slot, def) { self.slots[k] = play(audio, def, rng); }
                continue;
            }
            if playing(audio, slot, def) { audio.slots.release(slot); }
            self.slots[k] = -1;
        }
    }
}
