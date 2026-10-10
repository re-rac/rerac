//! In-engine scenes (game mode 2) in the engine: `rc_game::scene_player` driven once per 60 Hz frame, its
//! actors drawn as extra mobys, its camera, fade, subtitles, HUD hiding and audio requests applied, and the
//! Novalis first-arrival trigger. Spec: docs/plan/cutscenes_transitions.md §3, §4.3–4.4 ("In the port").
//!
//! * **Trigger** (`FixedUpdate`, before the gameplay tick): the classes' `DialogStreamStart(k)` calls of the last
//!   tick (`rc_game::cinematic::EngineRequest::StartScene`, and the talkers' `Handoff::Scene` of
//!   `rc_game::moby_update::interact`). The mission NPC 730/790 (`rc_game::moby_update::classes::mission_npc`)
//!   runs its state 0 in the load pass, so its state-1 branch runs in the first gameplay tick: mission byte ≠ 0xff,
//!   mode 0 and global flag 0x13d397 (`GameState::global.flags[15]`, mirrored into
//!   `Cinematic::arrival_seen`) clear → `DialogStreamStart(5)`, flag := 1; later its mission starts scenes 3 and 4.
//!   The port starts the scene on the frame after the tick that asked for it (the tick itself ran whole), sets the
//!   mode (`MenuMode` → `Mode::Cutscene`). `RC_SCENE` only concerns the arrival scene: `RC_SCENE=0` drops the
//!   arrival request (the classes go on), `RC_SCENE=<k>` plays scene k of the current level after gameplay tick 1
//!   instead of it; every other hand-off (talkers, the mission's scenes 3 / 4) always plays. A scene that cannot be
//!   loaded is reported as ended (skipped) to its talker.
//! * **Movies** (`EngineRequest::StartMovie`, `Handoff::Movie`: `DialogStreamUpdate(n)` → `StartPssMovie`):
//!   [`play_movie`] hands them to crate::movie_render, which plays them natively (decision U10) and at the end
//!   (`MovieExitToGameplay` 0x2ad2b8) restores mode 0 and refreshes the talker's dialogue.
//! * **The other requests** of the moby loop: `SetMissionDone` (the level's mission bytes and the saved game), the
//!   ship hidden / shown (`FUN_002a2450` / `0x2a2480`), the save (logged: the in-memory game state is the save;
//!   `UnlockPlanet` and the banners reach the game state and the HUD through `rc_game::cinematic`), and a class's
//!   `FadeToBlack(n)` ([`FadeHold`]: the n frames after the asking tick fade the last view to black, tick held).
//! * **While it runs** the world keeps running as in `CutsceneModeUpdate` 0x2aca80 (docs/plan/cutscenes.md §7): on
//!   every frame whose scene update runs ([`ActiveScene::world_runs`]: the scene ticks, not the blocking fades and
//!   waits) the gameplay tick runs in its mode-2 form (`Game::camera_paused`: moby loop, level callbacks, hero,
//!   particles, sound step, counter; no follow camera, no free-slot pass, no glints) with game mode 2 for the classes
//!   (`Services::game_mode`), the moby loop's camera 0x167240 / rows = the last scene camera record, and the scene
//!   (id, tick, actors) published to the classes ([`rc_game::scene_player::SceneState`]: the cutscene FX driver 1546).
//!   Ratchet `SetState(100, 2)` (the scene body, rc_game::hero::scripted) and hidden with his items (`FUN_002486c0`), the talker 0x179588
//!   hidden (mode |= 1), classes 74 / 203 hidden (mode |= 0x80), the HUD hidden (draw mask 0x7f,
//!   [`crate::hud_render::SceneLayer`]); all undone at the end.
//! * **Actors**: `CreateMoby(class)` per chunk-0 actor record: a table moby (0x16ce58[k], mode |= 6; the port adds
//!   the hidden bit because it draws them itself), posed and `MobyBuildMatrix`ed every scene tick (the owner of the
//!   effects classes spawn on them, the shadow casters of crate::shadow_render) and drawn through one [`ExtraMobys`]
//!   (record + palette per actor; a new scene never uploads the last scene's records / palette). The streamed sequence goes into the actor's own copy of its class animation at slot
//!   = the class's sequence count (`class+0xc`, then `+0xc++`), re-pointed at every chunk; the pose is
//!   `MobyAnimEval` of (slot, frame f, slot, f + 1, t). Rows = identity (no rotation track; `CreateMoby`
//!   leaves the rotation 0), scale = the class scale, lights = Ratchet's light word and ambient (+0x38 copied
//!   from the hero). A class missing from the level core that is a spaceship class (530..=533) is loaded from
//!   the global `spaceships` file (entry class − 530) like crate::moby_spawn's ship.
//! * **Camera**: [`ActiveScene::camera`] (`SceneCamera`: eye, forward/left/up rows, tan(hfov/2)) replaces the
//!   play camera's transform and projection tangent after `play_camera::apply` (same `RunFixedMainLoop`
//!   slot); when the scene ends `play_camera::apply` restores 0.63 by itself.
//! * **Fade**: a full-screen [`SceneFade`] pass on the main camera after the UI pass and the underwater tint
//!   (`assets/shaders/fade.wgsl`, GS integer blend), alpha `⌊black·128⌋`.
//! * **Subtitles** (`fun_001f4be0`, option 0x15ee40, on unless `RC_SUBTITLES=0`): `FontSetWindow(200, 0x208,
//!   0x28, 0x1d8, 0x100, h − 0x38, 0x12, 7)` measure, y = h − 0x3c (raised when the box would pass h − 0x14),
//!   `DrawUIFrame(y ∓ (height/2 + 5), 0x100 − (width/2 + 10), width/2 + 0x10a, 0x60)`, then the text in
//!   0x80b0b0b0 (regular font), appended to the 2D pass.
//! * **Audio**: the player's requests become `rc_game::audio::scene` commands (speech VAG from
//!   `levels/NN/speech/KK_<lang>.bin`), applied by the same frame's audio frame: the mode-2 tick's sound step when
//!   the world runs, else `crate::audio_out`'s scene sound step (the IOP frame of the blocking fades and waits).
//! * **End** (`FUN_002ac608`): Ratchet `SetState(0, 1)` (queued on the hero-block channel for the next tick); for a
//!   talker's scene Ratchet is put in front of it, facing it (`Interact::scene_end_place`, 0x16cd26), and its
//!   dialogue refreshed (`Interact::scene_ended`).
//! * **Letterbox** (`DrawScreenFade` 0x21b7d8) and the HUD while `0x15f404` (`creature::Globals::cutscene`: the
//!   camera trigger, the gunship, the troopers, the bolt crank) is set in gameplay: black bars top and bottom grow by
//!   one pixel per frame to 24 and shrink the same way after, drawn after the HUD; `HudDraw` draws nothing
//!   meanwhile.
//!
//! Not modelled: the hero's state 100 body (frozen instead), the draw callbacks the FX driver registers (the ship's
//! glow 0x2a70a8), the actors' moby-grid collision, the mirror / FOV cheats. The mode-6 space scenes (the ship's take-off
//! and landing) are crate::travel_render's.

use crate::fly_cam::FlyCam;
use crate::game_camera::CameraSource;
use crate::gameplay::{GameTick, Persistent, Play};
use crate::hud_render::{Hud2d, Prim, SceneLayer};
use crate::input_map::PadFrame;
use crate::moby_attach::AttachedTo;
use crate::moby_render::{self, ExtraMobys, MobyMaterial};
use crate::tfrag_render::game_to_bevy;
use anyhow::{Context, Result};
use bevy::camera::visibility::VisibilitySystems;
use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::font::{Font, GlyphTable};
use rc_formats::level::{ClassEntry, TextureEntry};
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass};
use rc_formats::moby_light::{self as light, V4};
use rc_formats::scene::Scene;
use rc_formats::texture::{LevelTexture, TextureSource, TextureTable};
use rc_game::audio::scene::{self as scene_audio, SceneAudioCmd};
use rc_game::hud::text;
use rc_game::menus::mode::Mode;
use rc_game::moby_runtime::MobyId;
use rc_game::pad::PadState;
use rc_game::ps2v::Pf;
use rc_game::scene_player::{AudioRequest, Frame, SceneCamera, SceneContext, ScenePlayer, SceneTick, REGION};
use std::sync::Arc;

/// Global flag 0x13d397 = `flags[0x13d397 − 0x13d388]`.
pub const ARRIVAL_FLAG: usize = 0x13d397 - 0x13d388;

/// `RC_SCENE`: None = the game's triggers, Some(None) = never, Some(Some(k)) = force scene k.
fn scene_env() -> Option<Option<usize>> {
    let v = std::env::var("RC_SCENE").ok()?;
    let v = v.trim();
    if v.is_empty() { return None; }
    Some(if v == "0" { None } else { v.parse().ok() })
}

/// The mode-2 black quad.
#[derive(Component, ExtractComponent, Clone, Copy, Default, PartialEq, ShaderType)]
pub struct SceneFade {
    /// GS RGBA bytes (A: 0x80 = 1.0).
    pub rgba: UVec4,
}

impl FullscreenMaterial for SceneFade {
    fn fragment_shader() -> ShaderRef { "shaders/fade.wgsl".into() }

    /// Last: after the UI pass (HUD, subtitles) and the underwater tint (docs: fade quad after `DrawWorld`).
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system
            .after(bevy::core_pipeline::Core3dSystems::PostProcess)
            .after(bevy::ui_render::ui_pass)
            .after(crate::gs_post::pass::<crate::fog_state::UnderwaterTint>)
            .before(bevy::core_pipeline::upscaling::upscaling)
    }
}

/// The scene frame (the classes' scene / movie / level-exit requests taken: crate::travel_render runs after it).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SceneFrameSet;

/// Marks entities hidden while a scene runs (the hero and his items).
#[derive(Component)]
pub(crate) struct SceneHidden;

struct Actor {
    /// The class animation with the streamed sequence in `slot` (shared with the moby loop's [`SceneState`]).
    anim: Arc<MobyAnimClass>,
    o_class: i32,
    /// The class's joint lists (`FUN_002645a8`).
    joint_lists: Arc<Vec<Vec<u8>>>,
    /// The last pose drawn (moby+0x50..0x54, moby+0x10): what the next tick's moby loop sees.
    pose: Option<(AnimState, [f32; 3])>,
    /// Its moby in the game's moby table (0x16ce58[k], [`enter_mode2`]).
    moby: Option<MobyId>,
    /// Extra sequence slot (class+0xc at spawn).
    slot: u8,
    /// The chunk whose sequence the slot points at.
    chunk: Option<usize>,
    scale: f32,
    base: u32,
    slots: u32,
    entities: Vec<Entity>,
}

/// The running scene, published for the play camera, the HUD and the reports.
#[derive(Resource, Default)]
pub struct ActiveScene {
    /// The scene camera of this frame (None = the play camera).
    pub camera: Option<SceneCamera>,
    /// Black coverage to draw over this frame (0..1).
    pub black: f32,
    pub running: bool,
    /// This frame runs the world update of `CutsceneModeUpdate` (the moby loop, the hero, the particles, the sound
    /// step, the counter): the gameplay tick runs in its scene form (`rc_game::tick::Game::camera_paused`).
    pub world_runs: bool,
    /// The last frame's output (reports / tests).
    pub last: Option<SceneTick>,
    /// The actors' table mobys (0x16ce58) and their class animation with the streamed sequence: the scene renderer
    /// draws them (their table mobys carry the port-only hidden bit), crate::shadow_render casts their shadows.
    pub actors: Vec<(MobyId, Arc<MobyAnimClass>)>,
    /// The white quad 0x15f400 (`DrawWorld` draws it after the black one, white at `0x15f400·128`): mode 6's take-off
    /// flash (crate::travel_render).
    pub white: f32,
    /// Game mode 6's take-off / landing actors (crate::travel_render): their table mobys and class animation with the
    /// streamed sequence. crate::shadow_render casts their shadows (`GameStateUpdate`: +0x7f ≠ 0 → `FUN_0026f0e0`, then
    /// `MobyProc`'s shadow pass) as it does a mode-2 scene's actors.
    pub space_actors: Vec<(MobyId, Arc<MobyAnimClass>)>,
    /// Mode 6 hides Ratchet and his items (`FUN_002486c0`, crate::travel_render): `MobyProc` skips his shadow too.
    pub space_hero_hidden: bool,
}

#[derive(Resource)]
pub(crate) struct SceneRuntime {
    mode: Option<Option<usize>>,
    player: Option<ScenePlayer>,
    triggered: bool,
    pad: PadState,
    actors: Vec<Actor>,
    extra: Option<ExtraMobys>,
    palette_len: u32,
    light_word: u32,
    ambient: [u8; 3],
    language: usize,
    /// `RC_SUBTITLES` (None: the game's option 0x15ee40).
    subtitles: Option<bool>,
    glyphs: Option<[GlyphTable; 3]>,
    frames: u32,
    uploaded: Option<u32>,
    /// Actor records / palette of the last frame (uploaded in PostUpdate).
    records: Vec<u8>,
    palette: Vec<u8>,
    /// Scenes the classes asked for that wait for the running one: (scene, arrival flag, talker).
    pending: std::collections::VecDeque<(usize, bool, Option<MobyId>)>,
    /// The talker whose scene runs (`0x179588`).
    talker: Option<MobyId>,
    /// `DrawScreenFade`'s bar height 0x15f408 (pixels, 0..=24).
    letterbox: i32,
    /// The mobys the scene hid (the talker 0x179588: mode |= 1; classes 74 / 203: mode |= 0x80) and the bit.
    hid: Vec<(MobyId, u16)>,
}

pub struct SceneRenderPlugin;

impl Plugin for SceneRenderPlugin {
    fn build(&self, app: &mut App) {
        let mode = scene_env();
        // `RC_SUBTITLES=0|1` forces them; else the game's option 0x15ee40 (pause → Options → Subtitles) decides.
        let subtitles = std::env::var("RC_SUBTITLES").ok().map(|v| v.trim() != "0");
        app.add_plugins(FullscreenMaterialPlugin::<SceneFade>::default())
            .init_resource::<ActiveScene>()
            // A runtime level change (crate::level_switch): the scene state of the old level dropped.
            .add_systems(crate::level_switch::LevelUnload, (crate::level_switch::reset::<ActiveScene>, crate::level_switch::reset::<FadeHold>, level_reset))
            .insert_resource(SceneRuntime {
                mode,
                player: None,
                triggered: false,
                pad: PadState::default(),
                actors: Vec::new(),
                extra: None,
                palette_len: 0,
                light_word: 0,
                ambient: [0x38; 3],
                language: crate::hud_render::language() as usize,
                subtitles,
                glyphs: None,
                frames: 0,
                uploaded: None,
                records: Vec::new(),
                palette: Vec::new(),
                pending: Default::default(),
                talker: None,
                letterbox: 0,
                hid: Vec::new(),
            })
            // While a scene runs the gameplay tick runs only on the frames whose CutsceneModeUpdate updates the world
            // (not during the blocking fades and waits), in its mode-2 form (enter_mode2).
            .configure_sets(FixedUpdate, GameTick.run_if(|a: Res<ActiveScene>| !a.running || a.world_runs))
            // A class's FadeToBlack(n) holds the next n frames (fade_take / fade_step).
            .init_resource::<FadeHold>()
            .configure_sets(FixedUpdate, GameTick.run_if(|h: Res<FadeHold>| h.frames == 0))
            .add_systems(FixedUpdate, (fade_step.before(GameTick), fade_take.after(GameTick)))
            .add_systems(RunFixedMainLoop, hold_camera.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop).after(crate::play_camera::apply))
            .add_systems(FixedUpdate, (scene_frame.before(GameTick).in_set(SceneFrameSet), actor_mobys.after(GameTick)))
            .add_systems(RunFixedMainLoop, apply_camera.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop).after(crate::play_camera::apply))
            .add_systems(Update, subtitle_layer.before(crate::hud_render::HudBuild))
            .add_systems(PostUpdate, (upload, fade_pass))
            .add_systems(PostUpdate, force_hidden.after(moby_render::update_moby_occlusion).before(VisibilitySystems::VisibilityPropagate));
        match mode {
            Some(None) => println!("scene: RC_SCENE=0: the arrival scene is not played (other scenes are)"),
            Some(Some(k)) => println!("scene: RC_SCENE={k}: scene {k} of this level starts after gameplay tick 1"),
            None => {}
        }
    }
}

/// The per-level part of [`SceneRuntime`] dropped at a runtime level change (its actors' entities are gone).
fn level_reset(mut rt: ResMut<SceneRuntime>) {
    let rt = &mut *rt;
    rt.player = None;
    rt.actors.clear();
    rt.extra = None;
    rt.palette_len = 0;
    rt.glyphs = None;
    rt.frames = 0;
    rt.uploaded = None;
    rt.records.clear();
    rt.palette.clear();
    rt.pending.clear();
    rt.talker = None;
    rt.letterbox = 0;
    rt.hid.clear();
}

/// The scene k of the current level, from `level_header.bin` and `scene/KK_ntsc.bin`.
fn load_scene(k: usize) -> Result<Scene> {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let header = crate::disc_source::level_file(&root, index, "level_header.bin")?;
    let h = rc_formats::toc::parse_level_header(&header)?;
    let region = crate::disc_source::level_file(&root, index, &format!("scene/{k:02}_{}.bin", REGION.name()))?;
    Ok(Scene::load(&h, &region, k, REGION)?)
}

/// `levels/NN/speech/KK_<lang>.bin` (None when the scene has no speech in that language).
fn load_speech(k: usize, language: usize) -> Option<Arc<[u8]>> {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let lang = rc_formats::toc::SCENE_LANGUAGES.get(language)?;
    crate::disc_source::level_file(&root, index, &format!("speech/{k:02}_{lang}.bin")).ok().map(Arc::from)
}

/// Gameplay ticks run since the level load: `0x15f5cc` is 1 after the load (`Game::finish_load`, the load's
/// `0x15f5cc++`) and counts up once per tick.
fn ticks_since_load(counter: u64) -> u64 { counter.saturating_sub(1) }

/// `RC_DEBUG_KILL=t` (dev check, not a game option): before gameplay tick index `t` the three mobys the mission NPC
/// 790 waits for (its links +0x14 / +0x18 / +0x1c) are deleted, to reach its bridge cutaway without a fight.
fn debug_kills(p: &mut Play) {
    static KILL: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    let Some(t) = *KILL.get_or_init(|| std::env::var("RC_DEBUG_KILL").ok()?.trim().parse().ok()) else { return };
    if p.game.counter != t + 1 { return; }
    let Some(npc) = p.game.mobys.mobys.iter().position(|m| m.o_class == 790 && !m.is_deleted()) else { return };
    let pv = p.game.mobys.mobys[npc].pvars.clone();
    for o in [0x14usize, 0x18, 0x1c] {
        let Some(id) = pv.get(o..o + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap())).and_then(|v| usize::try_from(v).ok()) else { continue };
        if id >= p.game.mobys.mobys.len() || p.game.mobys.mobys[id].is_deleted() { continue; }
        let c = p.game.counter;
        p.game.mobys.delete(id, c);
        println!("scene: RC_DEBUG_KILL: moby {id} (class {}) deleted before tick {t}", p.game.mobys.mobys[id].o_class);
    }
}

/// `RC_SCENE=<k>`: scene k after gameplay tick 1 (once).
fn forced(rt: &SceneRuntime, play: Option<&Play>) -> Option<usize> {
    if rt.triggered { return None; }
    let play = play?;
    if ticks_since_load(play.game.counter) < 1 { return None; }
    rt.mode.flatten()
}

/// The moby loop's requests of the last tick (module docs): scenes queued in `rt.pending`, the rest applied.
fn take_requests(rt: &mut SceneRuntime, play: &mut Play, state: Option<&mut Persistent>) {
    use rc_game::cinematic::EngineRequest as R;
    use rc_game::moby_update::interact::Handoff;
    let level = crate::level_load::level_index() as usize;
    let mut state = state;
    // Global flag 0x13d397 as the saved game has it (the NPC sets it with its request; the engine only ever raises it).
    if state.as_deref().is_some_and(|s| s.0.global.flags[ARRIVAL_FLAG] != 0) { play.svc.cinematic.arrival_seen = true; }
    let mut scenes: Vec<(usize, bool, Option<MobyId>)> = Vec::new();
    let mut movies: Vec<(i32, Option<MobyId>)> = Vec::new();
    for r in std::mem::take(&mut play.svc.cinematic.requests) {
        match r {
            R::StartScene { scene, arrival } => scenes.push((scene, arrival, None)),
            R::StartMovie { movie } => movies.push((movie, None)),
            R::MissionDone { mission } => {
                if let Some(b) = play.missions.done.get_mut(mission as usize) { *b = 0xff; }
                if let Some(b) = state.as_deref_mut().and_then(|s| s.0.levels.get_mut(level)).and_then(|l| l.missions.get_mut(mission as usize)) { *b = 0xff; }
                println!("scene: SetMissionDone({mission}) on level {level}");
            }
            // Taken right after its tick by fade_take (a request left here was already applied).
            R::FadeToBlack { .. } => {}
            R::Save => {
                // memcard_Save(0, −1) 0x261448: the capture (clock, landmarks, the map mask 0x25deb8) and the card's
                // incremental save (crate::saves).
                if let Some(st) = state.as_deref_mut() { crate::saves::memcard_save(play, &mut st.0, false, -1); }
            }
            // memcard_Save(0, pretend) from a class (Umbris' director 436: 8).
            R::SaveAs { pretend } => {
                if let Some(st) = state.as_deref_mut() { crate::saves::memcard_save(play, &mut st.0, false, pretend); }
            }
            // Class 1750 (level 18): the ending buffer (crate::saves).
            R::EndingSave => {
                if let Some(st) = state.as_deref() { crate::saves::store_ending(&st.0); }
            }
            // The slideshow mode 7, PlayMovieB and EnterMenuMode from a class (the boss 1422's ending): taken right after
            // their tick by crate::menu_render (crate::media_render::take_class_requests); one left here goes the same way.
            R::Slideshow => crate::media_render::request_slideshow(false),
            R::MovieB { movie } => crate::movie_render::request(crate::movie_render::MovieRequest::movie_b(movie)),
            R::EnterMenu { kind } => crate::media_render::request_menu(kind),
            // The level item-movie player (level02 0x298c68): mpegs[64 + n] in the game language, back to gameplay.
            R::ItemMovie { movie } => crate::movie_render::request(crate::movie_render::MovieRequest { file: (64 + movie) as u32, ..crate::movie_render::MovieRequest::in_level(movie, None) }),
            // 0x2a29a0(dest) / the menus' level exits: DoSpaceTransition (crate::travel_render).
            R::LeaveLevel { dest } => crate::travel_render::request_leave(dest),
            R::ShipHidden(h) => {
                if let Some(id) = play.ship_moby() {
                    let m = &mut play.game.mobys.mobys[id];
                    let was = m.mode & 3 == 3;
                    if h { m.mode |= 3 } else { m.mode &= !3 }
                    m.has_collision = !h;
                    if was != h { println!("scene: the ship {}", if h { "hidden" } else { "shown" }); }
                }
            }
        }
    }
    // The talkers' scene / movie hand-offs (the other hand-offs stay for their owner).
    let mut keep = Vec::new();
    for h in std::mem::take(&mut play.svc.interact.handoffs) {
        match h {
            Handoff::Scene { scene, npc } => scenes.push((scene.max(0) as usize, false, npc)),
            Handoff::Movie { movie, npc } => movies.push((movie, npc)),
            other => keep.push(other),
        }
    }
    play.svc.interact.handoffs = keep;
    // The talker's dialogue continues when the movie ends (MovieExitToGameplay, crate::movie_render).
    for (movie, npc) in movies { play_movie(movie, npc); }
    for (scene, arrival, npc) in scenes {
        // RC_SCENE only replaces the arrival scene; every other hand-off plays.
        if arrival && rt.mode.is_some() {
            println!("scene: DialogStreamStart({scene}) (the arrival) dropped: RC_SCENE is set");
            continue;
        }
        if arrival {
            if let Some(s) = state.as_deref_mut() { s.0.global.flags[ARRIVAL_FLAG] = 1; }
        }
        rt.pending.push_back((scene, arrival, npc));
    }
}

/// **The movie hook** (`StartPssMovie` 0x2ad0c0 → `MovieModeUpdate` 0x2ad498): in-level movie `n` is `mpegs[2 + n]`
/// (NTSC; PAL `21 + n`), raw PSS in Tier 0 at `global/mpegs/NNN.bin`, played natively by crate::movie_render
/// (decision U10; docs/plan/cutscenes.md §5) from this frame on, with the gameplay tick suspended; `npc`'s
/// dialogue is refreshed when it ends.
pub fn play_movie(n: i32, npc: Option<MobyId>) { crate::movie_render::request_in_level(n, npc); }

/// Moby part entities that are not hero items (Ratchet's play entities are found by name among them).
type HeroEntities<'w, 's> = Query<'w, 's, (Entity, &'static Name), (With<MeshMaterial3d<MobyMaterial>>, Without<AttachedTo>)>;

#[allow(clippy::too_many_arguments)]
fn scene_frame(
    mut commands: Commands,
    mut rt: ResMut<SceneRuntime>,
    mut active: ResMut<ActiveScene>,
    mut level: ResMut<crate::Level>,
    mut play: Option<ResMut<Play>>,
    mut state: Option<ResMut<Persistent>>,
    mut menu: Option<ResMut<crate::menu_render::MenuMode>>,
    pad: Option<Res<PadFrame>>,
    frame: Res<crate::determinism::FrameNumber>,
    hero: HeroEntities,
    items: Query<Entity, With<AttachedTo>>,
    hidden: Query<(Entity, Has<crate::gameplay::RatchetMesh>), With<SceneHidden>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let rt = &mut *rt;
    // Clean-up on the frame after the last scene frame: gameplay resumes this very frame.
    if rt.player.as_ref().is_some_and(|p| p.done()) {
        rt.player = None;
        for a in rt.actors.drain(..) { for e in a.entities { commands.entity(e).despawn(); } }
        // Ratchet's own meshes go back to crate::gameplay's visibility sync (his moby's hidden bit: a body may be in);
        // the rest (his items) are shown again.
        for (e, ratchet) in &hidden {
            if ratchet { commands.entity(e).remove::<SceneHidden>(); } else { commands.entity(e).remove::<SceneHidden>().insert(Visibility::Inherited); }
        }
        if let Some(m) = menu.as_mut() { m.state.set(Mode::Gameplay); }
        *active = ActiveScene::default();
        rt.records.clear();
        rt.palette.clear();
        if let Some(p) = play.as_mut() { leave_mode2(rt, p); }
        // FUN_002ac608: SetState(0, 1) (the next tick runs it), the talker's teleport and dialogue refresh.
        if let Some(p) = play.as_mut() {
            let counter = p.game.counter;
            let mut f = rc_game::moby_update::services::HeroFields::of(&p.game.hero);
            if let Some(npc) = rt.talker.take() {
                if let Some((pos, yaw)) = p.svc.interact.scene_end_place.take() {
                    f.clear_motion();
                    f.pose = Some(rc_game::moby_update::services::HeroPose { pos, yaw, target_yaw: yaw });
                }
                p.svc.interact.talker = Some(npc);
                p.svc.interact.scene_ended = true;
            }
            f.call(rc_game::moby_update::services::HeroCall::SetState { id: 0, play: true });
            p.svc.hero_writes = Some((counter, f));
        }
        println!("scene: control returns to gameplay at frame {} (hero SetState(0, 1) on the next tick)", rt.frames);
        return;
    }
    if let Some(p) = play.as_mut() {
        debug_kills(p);
        take_requests(rt, p, state.as_deref_mut());
    }
    if rt.player.is_none() {
        let is_forced = forced(rt, play.as_deref()).is_some();
        let (k, natural, talker) = if let Some(k) = forced(rt, play.as_deref()) {
            (k, false, None)
        } else if let Some(r) = rt.pending.pop_front() {
            r
        } else {
            // `cinematic::start_scene` stores game mode 2 in the tick (DialogStreamStart's 0x15f5c4 = 2); a request
            // the engine dropped (RC_SCENE's arrival) leaves no scene behind it: back to mode 0.
            if let Some(p) = play.as_mut() { if p.svc.game_mode == 2 { p.svc.game_mode = 0; } }
            return;
        };
        rt.triggered = true;
        rt.talker = talker;
        // The last scene's actor records / palette belong to its buffers (other sizes): never upload them into this
        // scene's (a second scene after the arrival drew its actors from stale, mis-sized buffers).
        rt.records.clear();
        rt.palette.clear();
        rt.uploaded = None;
        let scene = match load_scene(k) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                warn!("scene: scene {k} not loaded: {e:#}");
                if let Some(p) = play.as_mut() { if p.svc.game_mode == 2 && rt.pending.is_empty() { p.svc.game_mode = 0; } }
                // Reported as ended (a skipped scene) so a talker's dialogue goes on.
                if let (Some(npc), Some(p)) = (rt.talker.take(), play.as_mut()) {
                    p.svc.interact.talker = Some(npc);
                    p.svc.interact.scene_ended = true;
                }
                return;
            }
        };
        let gs = state.as_deref().map(|s| &s.0.global);
        // 0x15ed88 is the runtime language (the front end's Language list sets it, `hud_render::set_language`).
        // `SceneRuntime::language` is captured once when this plugin is built, i.e. before the player can pick a
        // language, so read the live value here, when the scene starts (the same moment the speech is streamed).
        rt.language = crate::hud_render::language() as usize;
        let ctx = SceneContext {
            game_beaten: gs.is_some_and(|g| g.game_beaten != 0),
            completes: gs.map_or(0, |g| g.completes),
            // 0x15eed8 (a scene replayed from the page menu's post-action 5 runs with 1).
            replay: menu.as_ref().is_some_and(|m| m.replay != 0),
            level: crate::level_load::level_index() as i32,
            language: rt.language,
            subtitles: rt.subtitles.unwrap_or_else(|| gs.is_none_or(|g| g.subtitles != 0)),
        };
        println!(
            "scene: app frame {}: DialogStreamStart({k}) after gameplay tick {} ({}): {} ticks, {} chunks, actors {:?}, cuts {:?}; subtitles {} (game option 0x15ee40 = {}), language {}",
            frame.0, play.as_ref().map_or(0, |p| ticks_since_load(p.game.counter)), if natural { "mission NPC 730/790, flag 0x13d397 := 1" } else if is_forced { "RC_SCENE" } else if talker.is_some() { "a talker's hand-off" } else { "a class" },
            scene.end_tick(), scene.chunks.len(), scene.actor_classes(), scene.cut_ticks(), ctx.subtitles, gs.map_or(-1, |g| g.subtitles as i32), ctx.language
        );
        if let Err(e) = spawn_actors(rt, &mut commands, &mut level.0, &scene, &mut meshes, &mut images, &mut materials, &mut buffers) {
            warn!("scene: actors not drawn: {e:#}");
        }
        rt.glyphs = level.0.hud.as_ref().map(|h| h.glyphs);
        let player = ScenePlayer::start(scene, ctx);
        post_audio(rt, &player, &player.start_audio());
        rt.player = Some(player);
        rt.frames = 0;
        if let Some(m) = menu.as_mut() { m.state.set(Mode::Cutscene); }
        // FUN_002486c0: the hero and his items are hidden.
        for (e, name) in &hero {
            if name.as_str().starts_with("Ratchet (play)") { commands.entity(e).insert((SceneHidden, Visibility::Hidden)); }
        }
        for e in &items { commands.entity(e).insert((SceneHidden, Visibility::Hidden)); }
        active.running = true;
        if let Some(p) = play.as_mut() { enter_mode2(rt, p); }
    }

    // One frame of the player.
    if let Some(p) = &pad { rt.pad.update(Some(&p.0.bytes()), false); }
    let Some(mut player) = rt.player.take() else { return };
    // What this frame's moby loop reads (it runs before the scene advances): the scene id, tick and actors as the
    // last frame left them, and the scene camera 0x167240 / rows of the last scene tick.
    if let Some(p) = play.as_mut() { publish_scene(rt, &player, active.camera, p); }
    let out = player.tick(&rt.pad);
    rt.frames += 1;
    post_audio(rt, &player, &out.audio);
    if out.frame == Frame::Playing && (out.scene_tick == 1 || out.camera.is_some_and(|c| c.cut)) {
        println!("scene: app frame {} (scene frame {}) = scene tick {}{}", frame.0, rt.frames, out.scene_tick, if out.scene_tick > 1 { " (camera cut)" } else { "" });
    }
    if let Some(e) = out.end {
        println!("scene: ended after {} frames ({}); tan(hfov/2) {} , music resumes in {} ticks", rt.frames, if e.skipped { "skipped" } else { "played through" }, e.tan_half_fov, e.music_resume_after);
    }
    if !out.actors.is_empty() {
        // The actors' joint-modifier lists (the scene big-head cheat's node, rc_game::moby_update::manip::scene_big_head).
        let table = play.as_ref().map(|p| &p.game.mobys);
        pose_actors(rt, &player, &out, &level.0, table);
    }
    active.camera = out.camera;
    active.black = out.black;
    active.world_runs = out.world_runs;
    active.actors = rt.actors.iter().filter_map(|a| Some((a.moby?, a.anim.clone()))).collect();
    active.last = Some(out);
    rt.player = Some(player);
}

fn post_audio(rt: &SceneRuntime, player: &ScenePlayer, reqs: &[AudioRequest]) {
    for r in reqs {
        let cmd = match r {
            AudioRequest::PauseMusic => SceneAudioCmd::PauseMusic,
            AudioRequest::Speech { scene, language, .. } => match load_speech(*scene, *language) {
                Some(vag) => SceneAudioCmd::Speech { vag },
                None => {
                    warn!("scene: no speech for scene {} language {} ({})", player.scene_id(), language, rt.language);
                    continue;
                }
            },
            AudioRequest::StopSpeech => SceneAudioCmd::StopSpeech,
            AudioRequest::ResumeMusic { after } => SceneAudioCmd::ResumeMusic { after: *after },
        };
        scene_audio::post(cmd);
    }
}

/// Registers a spaceship class (530..=533) from the global `spaceships` file (entry class − 530), texture
/// appended to the moby texture table, as crate::moby_spawn does for the loader's ship.
fn load_ship_class(level: &mut crate::level_load::LoadedLevel, o_class: i32) -> Result<(LevelMobyClass, MobyAnimClass)> {
    let root = crate::level_load::extracted_root();
    let file = crate::disc_source::read(&root, &format!("global/spaceships/{:03}.bin", o_class - 530))?;
    let s = rc_formats::moby_spawn::parse_spaceship(&file)?;
    let class = rc_formats::moby::parse_moby_class(s.class).context("parsing the ship class")?;
    let seqs = moby_anim::parse_sequences(s.class, &class).context("parsing the ship sequences")?;
    let tex = level.textures.iter().filter(|t| t.table == TextureTable::Moby).map(|t| t.index + 1).max().unwrap_or(0);
    let slot = u8::try_from(tex).ok().filter(|&t| t != 0xff).context("moby texture table full")?;
    let mut textures = [0xff; 16];
    textures[0] = slot;
    let desc = TextureEntry { data_offset: 0, width: 256, height: 256, ty: 4, palette: 0, mipmap: 0, pad: 0 };
    level.textures.push(LevelTexture { table: TextureTable::Moby, index: tex, source: TextureSource::Entry(desc), texture: s.texture });
    let anim = MobyAnimClass::new(&class, seqs);
    Ok((LevelMobyClass { o_class, entry: ClassEntry { offset_in_asset_wad: 0, o_class, unknown_8: 0, unknown_c: 0, textures }, class }, anim))
}

#[allow(clippy::too_many_arguments)]
fn spawn_actors(
    rt: &mut SceneRuntime,
    commands: &mut Commands,
    level: &mut crate::level_load::LoadedLevel,
    scene: &Scene,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> Result<()> {
    // +0x38: Ratchet's light word and ambient.
    if let Some(i) = level.mobys.instances.iter().find(|i| i.o_class == 0) { (rt.light_word, rt.ambient) = (i.light_word(), i.ambient_rgb()); }
    let mut specs = Vec::new();
    let mut palette_len = 0u32;
    let lists = actor_joint_lists(level, &scene.actor_classes());
    for a in &scene.chunks[0].actors {
        let found = match level.mobys.classes.iter().position(|c| c.o_class == a.class) {
            Some(ci) => Some((level.mobys.classes[ci].clone(), level.mobys.anim[ci].clone())),
            None if (530..=533).contains(&a.class) => Some(load_ship_class(level, a.class)?),
            // The game would `CreateMoby` a class that is not loaded (only unplayed scenes have one: the conformance
            // test lists them); the slot keeps its place and draws nothing.
            None => {
                warn!("scene: actor class {} is not on this level: not drawn", a.class);
                None
            }
        };
        let Some((class, mut anim)) = found else {
            let anim = MobyAnimClass { joint_count: 0, skeleton: vec![], rest: vec![], parent_word: vec![], sequences: vec![None] };
            specs.push((None, a.class, anim, 0, palette_len, 0));
            continue;
        };
        let slot = class.class.header.sequence_count;
        if anim.sequences.len() <= slot as usize { anim.sequences.resize(slot as usize + 1, None); }
        let slots = (anim.joint_count as u32).max(ExtraMobys::max_skinned_joint(&class) as u32 + 1).max(1);
        // The blend of a scene actor: +0x23 = 0x80 and mode = class bits (+0x44) | 6 (`CreateMoby`, `FUN_00259288`);
        // every scene actor class on the disc has neither 0x200 nor 8, so this is the regular draw
        // (moby_render::MobyBlend::pick, checked by rc-formats' scene conformance test).
        let blend = moby_render::MobyBlend::pick(class.class.header.mode_bits as u16 | 6, false, 0x80);
        if blend != moby_render::MobyBlend::Plain { warn!("scene: actor class {} asks for {blend:?}; drawn plain", class.o_class); }
        specs.push((Some(class), a.class, anim, slot, palette_len, slots));
        palette_len += slots;
    }
    let n = specs.len();
    let mut extra = ExtraMobys::new(level, vec![0; n.max(1) * moby_render::EXTRA_RECORD_SIZE], crate::moby_anim::identity_palette(palette_len), buffers);
    // The actors are dynamic mobys made in actor order, so `DrawMobys` draws them in it (ExtraMobys::set_ordered).
    extra.set_ordered();
    for (k, (class, o_class, anim, slot, base, slots)) in specs.into_iter().enumerate() {
        let entities = match &class {
            Some(class) => extra.spawn(commands, level, class, k as u32, Transform::IDENTITY, &format!("scene actor {k}"), meshes, images, materials),
            None => Vec::new(),
        };
        for &e in &entities { commands.entity(e).insert(Visibility::Hidden); }
        println!("scene: actor {k}: class {o_class} ({} joints, streamed sequence in slot {slot}), {} entities", anim.joint_count, entities.len());
        let scale = class.as_ref().map_or(1.0, |c| c.class.header.scale);
        let joint_lists = lists.get(&o_class).cloned().unwrap_or_default();
        rt.actors.push(Actor { scale, anim: Arc::new(anim), o_class, joint_lists, pose: None, moby: None, slot, chunk: None, base, slots, entities });
    }
    rt.extra = Some(extra);
    rt.palette_len = palette_len;
    Ok(())
}

/// The joint lists (class header `joints`) of the actor classes that have a blob in the level core, for the actors'
/// joint points (`FUN_002645a8`, [`rc_game::scene_player::SceneActorState::joint_point`]).
fn actor_joint_lists(level: &crate::level_load::LoadedLevel, classes: &[i32]) -> std::collections::HashMap<i32, Arc<Vec<Vec<u8>>>> {
    let mut out = std::collections::HashMap::new();
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let (Ok(data), Ok(idx)) = (rc_data::level_core_data(&root, index), crate::disc_source::level_file(&root, index, "core_index.bin")) else { return out };
    let Ok(core) = rc_formats::level::parse_level_core(&idx, data.len()) else { return out };
    for &oc in classes {
        if out.contains_key(&oc) { continue; }
        let Some(c) = level.mobys.classes.iter().find(|c| c.o_class == oc) else { continue };
        let name = format!("moby_class/{oc:04}");
        let Some(blob) = core.blocks.iter().find(|b| b.name == name).and_then(|b| data.get(b.offset..b.offset + b.size)) else { continue };
        let lists = (0..16).map_while(|l| rc_formats::gadget::joint_list(blob, &c.class.header, l).ok().map(|(a, _)| a)).collect();
        out.insert(oc, Arc::new(lists));
    }
    out
}

/// `DialogStreamStart`'s state changes for the world that keeps running in mode 2 (docs/plan/cutscenes.md §6): game
/// mode 2 for the classes (0x15f5c4), the tick in its mode-2 form (no follow camera, no free-slot pass: `CutsceneModeUpdate`
/// 0x2aca80), Ratchet `SetState(100, 2)` and then `0x1413f5 = 1` (his held objects hide: the Bomb Glove's bomb; both
/// applied by the next tick), the talker hidden (0x179588 mode |= 1) and every
/// live moby of class 74 / 203 hidden (mode |= 0x80).
fn enter_mode2(rt: &mut SceneRuntime, p: &mut Play) {
    use rc_game::moby_update::services::{HeroCall, HeroFields};
    use rc_game::scene_player::{SCENE_HIDDEN_BIT, SCENE_HIDDEN_CLASSES};
    p.svc.game_mode = 2;
    // DialogStreamStart 0x2ac330: the scene head record 0x17c8c0 (and 0x17c7c0 / 0x17c800) cleared.
    p.svc.scene_head = None;
    p.game.camera_paused = true;
    let counter = p.game.counter;
    let call = HeroCall::SetState { id: rc_game::scene_player::HERO_SCENE_STATE, play: true };
    match p.svc.hero_writes.as_mut() {
        Some((_, f)) => {
            f.call(call);
            f.hero_hidden = Some(1);
        }
        None => {
            let mut f = HeroFields::of(&p.game.hero);
            f.call(call);
            f.hero_hidden = Some(1);
            p.svc.hero_writes = Some((counter, f));
        }
    }
    // FUN_00259288's `CreateMoby(class)` per actor record: a dynamic slot of the moby table, +0x32 = 0x1ff, +0x72 = 0xff,
    // mode |= 6 (bit 2: the moby loop skips it; bit 4: no matrix rebuild), +0x94 = 0, +0x38 = Ratchet's light words. The
    // port draws the actors itself (the streamed sequence is not in the level's class animation), so the table moby also
    // carries mode bit 1 (not drawn by the generic moby draw); classes reach it as 0x16ce58[k] (SceneActorState::moby).
    {
        use rc_game::moby_update::services::ClassData;
        for a in rt.actors.iter_mut() {
            let oc = a.o_class as i16;
            let info = p.classes.info(oc);
            a.moby = p.game.mobys.create(oc, info.as_ref(), counter);
            if let Some(id) = a.moby {
                let m = &mut p.game.mobys.mobys[id];
                m.mode |= 6 | 1;
                m.state = 0;
            } else {
                warn!("scene: no free moby slot for actor class {oc}");
            }
        }
    }
    rt.hid.clear();
    if let Some(npc) = rt.talker.filter(|&n| n < p.game.mobys.mobys.len()) {
        let m = &mut p.game.mobys.mobys[npc];
        if m.mode & 1 == 0 { m.mode |= 1; rt.hid.push((npc, 1)); }
    }
    for (id, m) in p.game.mobys.mobys.iter_mut().enumerate() {
        if m.state < 0x80 && SCENE_HIDDEN_CLASSES.contains(&m.o_class) && m.mode & SCENE_HIDDEN_BIT == 0 {
            m.mode |= SCENE_HIDDEN_BIT;
            rt.hid.push((id, SCENE_HIDDEN_BIT));
        }
    }
}

/// `FUN_002ac608`'s side of [`enter_mode2`]: mode 0, the follow camera and the free-slot pass back, the hidden mobys
/// shown, the scene gone from the moby loop's view.
fn leave_mode2(rt: &mut SceneRuntime, p: &mut Play) {
    // A scene a class asked for meanwhile (queued) keeps mode 2: in the game its DialogStreamStart set it; it starts on
    // the next frame, so no tick in between sees mode 0.
    p.svc.game_mode = if rt.pending.is_empty() { 0 } else { 2 };
    p.game.camera_paused = false;
    p.svc.cinematic.scene = None;
    // FUN_002ac608: the actors deleted (their class sequence slots freed with them).
    let counter = p.game.counter;
    for a in &mut rt.actors {
        if let Some(id) = a.moby.take() { p.game.mobys.delete(id, counter); }
    }
    for (id, bit) in rt.hid.drain(..) {
        if let Some(m) = p.game.mobys.mobys.get_mut(id) { m.mode &= !bit; }
    }
}

/// Before a scene frame: what its moby loop reads (module docs).
/// An actor's [`rc_game::scene_player::SceneActorState`] with its current pose (the one it is drawn with once this
/// frame's pose is set).
fn actor_state(a: &Actor) -> rc_game::scene_player::SceneActorState {
    let rest = AnimState { seq_a: a.slot, frame_a: 0, seq_b: a.slot, frame_b: 0, t: 0.0, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: true };
    let (state, position) = a.pose.unwrap_or((rest, [0.0; 3]));
    rc_game::scene_player::SceneActorState { moby: a.moby, o_class: a.o_class as i16, position, scale: a.scale, anim: a.anim.clone(), state, joint_lists: a.joint_lists.clone() }
}

/// The scene actor that is moby `id`, with the pose it is drawn with this frame (after [`scene_frame`] posed it): what a
/// draw callback reads of its moby at draw time (the game's callbacks run after `CutsceneModeUpdate` wrote the pose,
/// e.g. the ship glass's `MobyAttachToJoint`, crate::fx_draw), unlike the moby loop, which sees the last frame's.
pub(crate) fn drawn_actor(rt: &SceneRuntime, id: MobyId) -> Option<rc_game::scene_player::SceneActorState> {
    rt.player.as_ref()?;
    rt.actors.iter().find(|a| a.moby == Some(id)).map(actor_state)
}

fn publish_scene(rt: &SceneRuntime, player: &ScenePlayer, camera: Option<SceneCamera>, p: &mut Play) {
    use rc_game::scene_player::SceneState;
    let actors = rt.actors.iter().map(actor_state).collect();
    // The table mobys still carry the last frame's pose here ([`actor_mobys`] wrote it after the last tick).
    p.svc.cinematic.scene = Some(SceneState { id: player.scene_id(), tick: player.scene_tick(), actors });
    // FUN_002ac8d8 writes 0x167240 and the rows 0x167450..; the Euler 0x167250 keeps the last gameplay value.
    if let Some(c) = camera {
        let v = |x: [f32; 3]| [Pf::f(x[0]), Pf::f(x[1]), Pf::f(x[2]), Pf::ZERO];
        let out = &mut p.game.camera.out;
        out.pos = [Pf::f(c.eye[0]), Pf::f(c.eye[1]), Pf::f(c.eye[2]), out.pos[3]];
        out.rows = c.rows.map(v);
    }
}

/// `CutsceneModeUpdate`'s write of the frame's pose into each actor moby, after the moby loop (the game's order:
/// `MobyUpdateLoop`, then the scene update, then the draw): +0x50..0x54, position (+0x10: the owner position of the
/// effects spawned on them), +0x71 = 0xff, `MobyBuildMatrix` (the rows and the bounding sphere from the streamed
/// sequence, rc_game::moby_update::scheduler::rebuild_matrix); drawn (+0x31). Runs after the gameplay tick, so this
/// frame's moby loop still read the last frame's pose while everything after it (the actors' *ShadowProbeAlongDir*
/// and shadow volumes, crate::shadow_render) sees the pose the actors are drawn with this frame.
pub(crate) fn actor_mobys(rt: Res<SceneRuntime>, play: Option<ResMut<Play>>) {
    let Some(mut p) = play else { return };
    if rt.player.is_none() { return; }
    for a in &rt.actors {
        if let (Some(id), Some((st, pos))) = (a.moby, a.pose) {
            if let Some(m) = p.game.mobys.mobys.get_mut(id) {
                (m.position[0], m.position[1], m.position[2]) = (pos[0], pos[1], pos[2]);
                (m.anim.seq_a, m.anim.frame_a, m.anim.seq_b, m.anim.frame_b, m.anim.t) = (st.seq_a, st.frame_a, st.seq_b, st.frame_b, st.t);
                m.b71 = 0xff;
                m.visible = 1;
                rc_game::moby_update::scheduler::rebuild_matrix(m, Some(&a.anim));
            }
        }
    }
}

/// The actors' records and palettes for this frame's poses.
fn pose_actors(rt: &mut SceneRuntime, player: &ScenePlayer, out: &SceneTick, level: &crate::level_load::LoadedLevel, table: Option<&rc_game::moby_runtime::MobyTable>) {
    let scene = player.scene();
    let mut palette = crate::moby_anim::identity_palette(rt.palette_len);
    let mut records = Vec::with_capacity(rt.actors.len() * moby_render::EXTRA_RECORD_SIZE);
    let rows: [V4; 3] = [[1f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|r| [r[0].to_bits(), r[1].to_bits(), r[2].to_bits(), 0]);
    let lighting = level.mobys.lighting.as_ref();
    for (a, pose) in rt.actors.iter_mut().zip(&out.actors) {
        if a.chunk != Some(pose.chunk) {
            // FUN_00259288: the slot is re-pointed at this chunk's sequence.
            if let Some(q) = Arc::make_mut(&mut a.anim).sequences.get_mut(a.slot as usize) {
                *q = Some(scene.chunks[pose.chunk].actors[pose.actor].sequence.clone());
            }
            a.chunk = Some(pose.chunk);
        }
        let s = AnimState { seq_a: a.slot, frame_a: pose.frame_a, seq_b: a.slot, frame_b: pose.frame_b, t: pose.t, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: true };
        a.pose = Some((s, pose.position));
        let mods = a.moby.and_then(|id| table.and_then(|t| t.mobys.get(id))).map(|m| m.joint_mods.as_slice()).unwrap_or(&[]);
        let f = moby_anim::evaluate_posed(&a.anim, &s, None, &[], mods);
        let at = a.base as usize * 64;
        for (k, b) in f.iter().take(a.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() { palette[at + k] = b; }
        let lights = lighting.map(|l| light::moby_lights(&rows, &l.bank, rt.light_word, rt.ambient, 0x80));
        let model = moby_render::extra_model([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], a.scale, pose.position);
        records.extend_from_slice(&moby_render::extra_record(&model, lights.as_ref(), a.base));
    }
    rt.records = records;
    rt.palette = palette;
}

/// Records, palettes, transforms and visibility of the actors after a frame.
fn upload(mut rt: ResMut<SceneRuntime>, active: Res<ActiveScene>, mut buffers: ResMut<Assets<ShaderBuffer>>, mut q: Query<(&mut Transform, &mut Visibility), Without<Camera3d>>) {
    let rt = &mut *rt;
    if rt.uploaded == Some(rt.frames) || rt.records.is_empty() { return; }
    rt.uploaded = Some(rt.frames);
    let Some(extra) = &rt.extra else { return };
    // Only data of this scene's layout: a buffer keeps the size it was created with (its materials are bound to it).
    let fits = |b: &ShaderBuffer, n: usize| b.data.as_ref().is_some_and(|d| d.len() == n);
    if buffers.get(&extra.palette).is_some_and(|b| fits(b, rt.palette.len())) {
        crate::asset_write::set_buffer(&mut buffers, &extra.palette, &rt.palette);
    }
    if buffers.get(&extra.instances).is_some_and(|b| fits(b, rt.records.len())) {
        crate::asset_write::set_buffer(&mut buffers, &extra.instances, &rt.records);
    }
    let shown = active.last.as_ref().is_some_and(|t| !t.actors.is_empty());
    for (k, a) in rt.actors.iter().enumerate() {
        let pos = active.last.as_ref().and_then(|t| t.actors.get(k)).map_or([0.0; 3], |p| p.position);
        let t = Transform::from_matrix(moby_render::extra_model([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], a.scale, pos));
        for &e in &a.entities {
            if let Ok((mut tr, mut vis)) = q.get_mut(e) {
                *tr = t;
                let want = if shown { Visibility::Inherited } else { Visibility::Hidden };
                if *vis != want { *vis = want; }
            }
        }
    }
}

/// The scene camera over the play camera (after `play_camera::apply`, before any reader of the transform).
fn apply_camera(active: Res<ActiveScene>, source: Option<Res<CameraSource>>, display: Res<crate::display::DisplaySettings>, mut cams: Query<(&mut Transform, &mut Projection), With<FlyCam>>) {
    let Some(c) = active.camera else { return };
    if source.is_some_and(|s| *s == CameraSource::Fly) { return; }
    let [fwd, _left, up] = c.rows;
    let t = Transform::from_translation(game_to_bevy(c.eye)).looking_to(game_to_bevy(fwd), game_to_bevy(up));
    for (mut tf, mut proj) in &mut cams {
        if *tf != t { *tf = t; }
        // The scene's tangent (Hor+ in 16:9, crate::display).
        crate::display::set_view_tans(&mut proj, c.tan_half_fov, display.aspect);
    }
}

/// `FadeToBlack(n)` 0x21b438 called by a class in gameplay (`rc_game::cinematic::EngineRequest::FadeToBlack`, the
/// gold bolt's pickup): the game draws n black quads over the last image inside the tick, one per vsync, then the
/// tick goes on. The port holds the n frames after the asking tick: the tick suspended, the view the last frame
/// showed (the asking tick's world under it: its hero teleport and camera cut are not shown until the hold ends),
/// the coverage of `scene_player::fade_to_black_coverage(n, k)` drawn by the fade pass, the HUD and letterbox as they
/// were.
#[derive(Resource, Default)]
pub struct FadeHold {
    /// n (0: no hold).
    pub frames: u32,
    /// The step drawn this frame (0..n + tail).
    pub k: u32,
    /// Black frames after the n fade steps (the death reload's black, [`crate::gameplay::RELOAD_BLACK`]; a second
    /// `FadeToBlack` of the same tick over the black image).
    pub tail: u32,
    /// The view the last frame showed (before the asking tick) and the one before each tick.
    view: Option<crate::play_camera::PlayView>,
    before: Option<crate::play_camera::PlayView>,
}

impl FadeHold {
    /// Black coverage of this frame (0 without a hold).
    pub fn coverage(&self) -> f32 {
        if self.frames == 0 { 0.0 } else if self.k >= self.frames { 1.0 } else { rc_game::scene_player::fade_to_black_coverage(self.frames, self.k) }
    }
}

/// Before each tick: the next step of a running hold (the tick stays suspended until n steps were drawn), and the
/// view the last frame showed.
fn fade_step(mut hold: ResMut<FadeHold>, view: Option<Res<crate::play_camera::PlayView>>) {
    if hold.frames > 0 {
        hold.k += 1;
        if hold.k >= hold.frames + hold.tail {
            println!("scene: FadeToBlack({}) done: the tick resumes", hold.frames);
            *hold = FadeHold::default();
        }
    }
    if hold.frames == 0 { hold.before = view.map(|v| *v); }
}

/// After each tick: a class's `FadeToBlack(n)` starts a hold (step 0 drawn this frame).
fn fade_take(mut hold: ResMut<FadeHold>, play: Option<ResMut<Play>>, frame: Res<crate::determinism::FrameNumber>) {
    use rc_game::cinematic::EngineRequest as R;
    let Some(mut p) = play else { return };
    let mut fades = Vec::new();
    p.svc.cinematic.requests.retain(|r| match *r {
        R::FadeToBlack { frames } => {
            fades.push(frames.max(1) as u32);
            false
        }
        _ => true,
    });
    let Some(&n) = fades.first() else { return };
    // A later fade of the same tick draws over the black image (the kill volume's 10, then the death sequence's 16);
    // a death adds the reload's black ([`crate::gameplay::RELOAD_BLACK`]).
    let tail = fades[1..].iter().sum::<u32>() + if p.death_pending { crate::gameplay::RELOAD_BLACK } else { 0 };
    println!("scene: app frame {}: FadeToBlack({n}) after gameplay tick {}: the next {n} frames fade the last view to black, {tail} more black", frame.0, ticks_since_load(p.game.counter));
    let view = hold.before;
    *hold = FadeHold { frames: n, k: 0, tail, view, before: view };
}

/// The held view over the play camera while a hold runs.
fn hold_camera(hold: Res<FadeHold>, source: Option<Res<CameraSource>>, mut cams: Query<&mut Transform, With<FlyCam>>) {
    let Some(v) = hold.view.filter(|_| hold.frames > 0) else { return };
    if source.is_some_and(|s| *s != CameraSource::Play) { return; }
    let t = crate::play_camera::view_transform(&v.view);
    for mut tf in &mut cams { if *tf != t { *tf = t; } }
}

/// The fade quad component on the main camera.
fn fade_pass(mut commands: Commands, active: Res<ActiveScene>, hold: Res<FadeHold>, play: Option<Res<Play>>, cams: Query<(Entity, Option<&SceneFade>), With<FlyCam>>) {
    // In gameplay without a hold: the classes' 0x15f3fc (`DrawWorld` 0x21a1b8: black at `trunc(min(f, 1)·128)` when
    // f > 0; `rc_game::cinematic::set_fade`).
    let gameplay = play.as_ref().map_or(0.0, |p| p.svc.cinematic.fade);
    let black = if active.running { active.black } else if hold.frames > 0 { hold.coverage() } else { gameplay };
    let alpha = (black.clamp(0.0, 1.0) * 128.0) as u32;
    let mut want = ((active.running || hold.frames > 0 || gameplay > 0.0) && alpha > 0).then(|| SceneFade { rgba: UVec4::new(0, 0, 0, alpha) });
    // The white quad 0x15f400 (mode 6, crate::travel_render) when no black one is drawn [L: the game draws both, black
    // first; they never overlap in the take-off].
    // In gameplay also the classes' white (`rc_game::cinematic::set_white`, Blarg's escape 1108).
    let white_g = if active.running { 0.0 } else { play.as_ref().map_or(0.0, |p| p.svc.cinematic.white) };
    let white = (active.white.max(white_g).clamp(0.0, 1.0) * 128.0) as u32;
    if want.is_none() && white > 0 { want = Some(SceneFade { rgba: UVec4::new(255, 255, 255, white) }); }
    for (e, have) in &cams {
        match (want, have) {
            (Some(w), Some(h)) if *h == w => {}
            (Some(w), _) => { commands.entity(e).insert(w); }
            (None, Some(_)) => { commands.entity(e).remove::<SceneFade>(); }
            (None, None) => {}
        }
    }
}

/// HUD hidden and the subtitle box (module docs) into the 2D pass; in gameplay, the letterbox of `0x15f404`.
fn subtitle_layer(mut rt: ResMut<SceneRuntime>, active: Res<ActiveScene>, hold: Res<FadeHold>, play: Option<Res<Play>>, mut layer: ResMut<SceneLayer>, level: Option<Res<crate::Level>>) {
    // The font of the level's HUD: the class draws below (the countdowns) need it without a scene having played first.
    if rt.glyphs.is_none() { rt.glyphs = level.as_ref().and_then(|l| l.0.hud.as_ref().map(|h| h.glyphs)); }
    // FadeToBlack blocks inside the tick: the last image (its bars, its HUD) stays under the fade.
    if hold.frames > 0 { return; }
    let flag = !active.running && play.as_ref().is_some_and(|p| p.svc.creatures.cutscene);
    // DrawScreenFade 0x21b7d8 (part of the HUD layer, so never in mode 2): the bars grow while 0x15f404 is set.
    if !active.running {
        if flag {
            if rt.letterbox < LETTERBOX_MAX { rt.letterbox += 1; }
        } else if rt.letterbox > 0 {
            rt.letterbox -= 1;
        }
    }
    // HudDraw 0x24fb50 draws no slot while 0x15f404 is set.
    layer.hide_hud = active.running || flag;
    layer.prims.clear();
    if !active.running && rt.letterbox > 0 { layer.prims = letterbox_prims(rt.letterbox); }
    // The census units' 2-D draw callbacks (Veldin's countdown 586, `0x2d8098`: `DrawUIFrame` and
    // `font_print_center_large`, `rc_game::moby_update::classes::units::veldin_pads::countdown_draw`).
    if !active.running {
        if let (Some(glyphs), Some(p)) = (rt.glyphs.as_ref(), play.as_ref()) {
            let texts = &p.svc.draw_callbacks.texts;
            if !texts.is_empty() {
                use crate::text_render::{draw_ui_frame, font_print, TextState};
                let mut out = Hud2d::default();
                let mut st = TextState::default();
                let g = &glyphs[Font::Large as usize];
                for t in texts {
                    draw_ui_frame(&mut out, t.frame[0], t.frame[1], t.frame[2], t.frame[3], t.frame[4]);
                    let width = rc_formats::font::measure_text_width(&t.text, 8, g);
                    font_print(&mut out, &mut st, g, Font::Large, t.x - (width >> 1), t.y, t.rgba, &t.text, 8);
                }
                layer.prims.extend(out.prims);
            }
        }
    }
    // The Trespasser locks' minigame (`0x2d93e8`, the frame's draw callback: rc_game's units::trespasser_lock::frame).
    if !active.running {
        if let Some(p) = play.as_ref() { layer.prims.extend(p.svc.draw_callbacks.rings.iter().map(ring_prim)); }
    }
    let (Some(glyphs), Some(line)) = (rt.glyphs.as_ref(), active.last.as_ref().and_then(|t| t.subtitle.as_ref())) else { return };
    if !active.running { return; }
    layer.prims = subtitle_prims(glyphs, &line.text);
}

/// One primitive of the Trespasser minigame as a HUD primitive (pixels and texels rounded; the FX texture).
fn ring_prim(r: &rc_game::moby_update::classes::units::trespasser_lock::RingPrim) -> Prim {
    const W: i32 = crate::hud_render::W;
    const H: i32 = crate::hud_render::H;
    Prim {
        tex: crate::hud_render::Tex::Fx(r.fx),
        pos: r.pos.map(|p| [p[0].round() as i32, p[1].round() as i32]),
        uv: r.uv.map(|u| [u[0] as i32, u[1] as i32]),
        rgba: r.rgba,
        scissor: [0, W - 1, 0, H - 1],
        repeat: r.repeat,
        nearest: false,
        boxed: false,
        uv16: false,
    }
}

/// `DrawScreenFade`'s largest bar height (0x15f408 < 0x18).
pub const LETTERBOX_MAX: i32 = 24;

/// The two opaque black bars of `DrawScreenFade`: `h` pixels at the top and the bottom of the frame (one triangle
/// strip, colour 0x80000000, no blending).
pub fn letterbox_prims(h: i32) -> Vec<Prim> {
    const W: i32 = crate::hud_render::W;
    const H: i32 = crate::hud_render::H;
    let mut out = Hud2d::default();
    out.rect(0, h, 0, W, 0x8000_0000);
    out.rect(H - h, H, 0, W, 0x8000_0000);
    out.prims
}

/// `fun_001f4be0`'s box and text for one line.
pub fn subtitle_prims(glyphs: &[GlyphTable; 3], text_bytes: &[u8]) -> Vec<Prim> {
    use crate::text_render::{draw_ui_frame, font_print_window, TextState};
    const H: i16 = crate::hud_render::H as i16;
    const RGBA: u32 = 0x80b0_b0b0;
    let g = &glyphs[Font::Regular as usize];
    let mut out = Hud2d::default();
    let mut st = TextState::default();
    let mut win = text::Window::new(0xc8, 0x208, 0x28, 0x1d8, 0x100, H - 0x38, 0x12, text::CENTRE_LINES | text::CENTRE_BLOCK | text::MEASURE_ONLY);
    font_print_window(&mut out, &mut st, g, Font::Regular, &mut win, RGBA, text_bytes, -1);
    let hh = (win.height >> 1) as i32 + 5;
    let hw = (win.max_width >> 1) as i32;
    let mut y = H as i32 - 0x3c;
    if (H as i32 - 0x14) < y + hh { y = H as i32 - ((win.height >> 1) as i32 + 0x19); }
    win.y_start = y as i16;
    out.prims.clear();
    draw_ui_frame(&mut out, y - hh, y + hh, 0x100 - (hw + 10), hw + 0x10a, 0x60);
    win.flags &= !text::MEASURE_ONLY;
    font_print_window(&mut out, &mut st, g, Font::Regular, &mut win, RGBA, text_bytes, -1);
    out.prims
}

/// Hidden wins over the occlusion system's visibility writes (as crate::moby_spawn::force_hidden).
fn force_hidden(mut q: Query<&mut Visibility, With<SceneHidden>>) {
    for mut v in &mut q {
        if *v != Visibility::Hidden { *v = Visibility::Hidden; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::font::{Glyph, GLYPHS};

    /// `DrawScreenFade`: two opaque black bars of `h` pixels, top and bottom, full width.
    #[test]
    fn letterbox_bars() {
        let p = letterbox_prims(LETTERBOX_MAX);
        let r: Vec<[i32; 4]> = p.iter().map(|q| q.rect()).collect();
        assert_eq!(r, [[0, 0, crate::hud_render::W, 24], [0, crate::hud_render::H - 24, crate::hud_render::W, 24]]);
        assert!(p.iter().all(|q| q.rgba == 0x8000_0000 && q.tex == crate::hud_render::Tex::None));
    }

    /// `fun_001f4be0`: one 60-pixel line → box of half width 30 + 10 around x = 256, 9 + 5 above and below
    /// y = 416 − 0x3c, text centred on the same line.
    #[test]
    fn subtitle_box_geometry() {
        let mut t = [Glyph::default(); GLYPHS];
        t[b'A' as usize] = Glyph { u: 0, v: 0, y_off: 0, advance: 12 };
        let glyphs = [t; 3];
        let prims = subtitle_prims(&glyphs, b"AAAAA");
        assert!(!prims.is_empty());
        // The first primitives are the DrawUIFrame box; its bounds are the union of their rectangles.
        let frame: Vec<[i32; 4]> = prims.iter().filter(|p| p.tex == crate::hud_render::Tex::None).map(|p| p.rect()).collect();
        assert!(!frame.is_empty());
        let (x0, y0) = (frame.iter().map(|r| r[0]).min().unwrap(), frame.iter().map(|r| r[1]).min().unwrap());
        let (x1, y1) = (frame.iter().map(|r| r[0] + r[2]).max().unwrap(), frame.iter().map(|r| r[1] + r[3]).max().unwrap());
        assert_eq!(((x0 + x1) / 2, (y0 + y1) / 2), (256, 356), "box {x0}..{x1} x {y0}..{y1}");
        assert!(prims.iter().any(|p| p.tex != crate::hud_render::Tex::None), "text drawn");
    }
}
