//! Planet travel in the engine: game mode 6 (`rc_game::travel::space`: the take-off and landing scenes, the
//! fly-away, the flight), `DoSpaceTransition` (`rc_game::travel::transition`: the story cards, the transition movies,
//! the flight, the load) and the hand-over to the next level through the one runtime level change
//! (crate::level_switch). Spec: docs/plan/progression.md `## travel`.
//!
//! **API** (the other lanes' entry points):
//! * [`request_leave`]`(dest)` = the game's `0x15f5c0 = dest; 0x15f570 = 1`: the level's main loop ends and
//!   `DoSpaceTransition` runs (−1: the front end). `rc_game::cinematic::EngineRequest::LeaveLevel` (the classes'
//!   `0x2a29a0(p)`, `crate::media_render::request_level_exit`: Quit Game, New Game, Load Game, the end page) arrives
//!   here through crate::scene_render.
//! * [`request_ship_travel`]`(dest)`: the planet page's post-action 2 (`ShipTravelTo(0x184894)`, crate::menu_render).
//!
//! **Frames.** [`pre_tick`] (before the gameplay tick): the requests, the level's `entry` after a level change
//! (`ShipLandingStart`), and which frames of mode 6 run the world (`GameStateUpdate`'s moby loop, particles, sound,
//! counter: `MenuMode::world_tick`; the scene form for subs 0 / 8, with the follow camera for sub 3). [`post_tick`]
//! (after it, before the menus' frame): the take-off (the ship's △ of the last tick: `InLevelFrameUpdate`'s
//! `0x15f630` test), the mode-6 frame (`ShipMode::frame` on the moby table) and its outputs (the scene camera through
//! `ActiveScene::camera`, the fade 0x15f3fc through `Cinematic::fade`, the white quad 0x15f400 through
//! `ActiveScene::white`, the actors, the audio, the planet page, the landing's end, the leave), and the transition's
//! steps. [`hud_layer`]: the HUD is not drawn in mode 6 (draw mask 0x7f), the story cards and the black screen of the
//! transition. [`upload_actors`]: the scene / flight actors as extra mobys (the streamed sequence in an extra class
//! slot, crate::scene_render's scheme; ship-local rows from the ship's yaw).
//!
//! **The transition** (`Transition`): the plan's steps one after the other: `Begin` (all sounds and the music stopped,
//! mode 6), `Fade(n)` (n black quads over the last image, `Cinematic::fade`), `Card` (`rc_game::travel::cards`, over
//! black: the world view is replaced by an empty layer, [`TRAVEL_LAYER`], the sky camera off), `Movie(n)`
//! (crate::movie_render, `mpegs[40 + n]`, back to mode 6), `SetLevel` (`GameState::apply_transition`, 0x13e056 for the
//! loader, the background load of `dest`), `Flight` (sub 4 on the old level's moby table: the variants of the
//! `transition` lump, the ship actor on [`TRAVEL_LAYER`], until the load is done and the approach played), `Enter`
//! (the level start's saved-game rules `GameState::apply_level_start` with the destination's item tables, then the
//! swap; the next frames: the gameplay set-up, then `ShipLandingStart`).
//!
//! **The flight's scenery** is crate::flight_render's (the transition lump's sky, planet picture, caption, trail, glass,
//! card icon and sound); **the weapon demo** ([`request_weapon_demo`], crate::interact_render's vendor substate 3) is the
//! same space-scene player (`space::SUB_DEMO`) in the vendor's frame, with [`take_weapon_demo_done`] for its
//! `VendorExit(1)`; **sub 3's camera** runs after the fly-away's body ([`fly_away_camera`]); the mode-6 actors' shadows go
//! through `ActiveScene::space_actors` (crate::shadow_render).
//!
//! Environment: `RC_TRAVEL_TRACE=1` prints every step and mode-6 event; `RC_LANDING=1` runs `entry`'s landing on the
//! boot's level.

use crate::fly_cam::FlyCam;
use crate::gameplay::{GameTick, Persistent, Play, Session};
use crate::hud_render::{Hud2d, Hud2dHook, HudBuild};
use crate::level_switch::LevelChange;
use crate::menu_render::{MenuMode, MenuPrims};
use crate::moby_attach::AttachedTo;
use crate::moby_render::{self, ExtraMobys, MobyMaterial};
use crate::scene_render::ActiveScene;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::level::{ClassEntry, TextureEntry};
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass};
use rc_formats::scene::{Region, Scene};
use rc_formats::texture::{LevelTexture, TextureSource, TextureTable};
use rc_game::audio::scene::{self as scene_audio, SceneAudioCmd};
use rc_game::menus::mode::Mode;
use rc_game::moby_update::services::World;
use rc_game::travel::cards::{CardFrame, CardPlayer, Plates};
use rc_game::travel::space::{self, ActorPose, SpaceAudio, SpaceEvent, SpaceFrame, SpaceLump};
use rc_game::travel::transition::{self, Plan, Step};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// The render layer of the transition's view (the flight's actors; nothing during the cards and movies).
pub const TRAVEL_LAYER: usize = 26;

/// What the game asked for outside a frame of this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request {
    Leave(i32),
    ShipTravel(i32),
    /// A weapon demo scene `unknown_1530[scene]` in the frame of moby `frame` (the vendor).
    Demo { scene: i32, frame: usize },
}

static QUEUE: Mutex<VecDeque<Request>> = Mutex::new(VecDeque::new());

fn queue() -> std::sync::MutexGuard<'static, VecDeque<Request>> { QUEUE.lock().unwrap_or_else(|e| e.into_inner()) }

/// `0x15f5c0 = dest; 0x15f570 = 1` (module docs): `DoSpaceTransition` to `dest` from the next frame.
pub fn request_leave(dest: i32) {
    println!("travel: leave the level for {dest} (0x15f570 = 1: DoSpaceTransition)");
    queue().push_back(Request::Leave(dest));
}

/// `ShipTravelTo(dest)` 0x2a2848 (the planet page's post-action 2).
pub fn request_ship_travel(dest: i32) {
    println!("travel: ShipTravelTo({dest})");
    queue().push_back(Request::ShipTravel(dest));
}

/// `VendorStartWeaponDemo` 0x2ae7f8's scene (crate::interact_render, the vendor's substate 3): `unknown_1530[scene]`
/// played in the vendor's frame (`rc_game::travel::space::SUB_DEMO`); its end is [`take_weapon_demo_done`].
pub fn request_weapon_demo(scene: i32, vendor: usize) {
    println!("travel: VendorStartWeaponDemo: the demo scene unknown_1530[{scene}] in moby {vendor}'s frame");
    queue().push_back(Request::Demo { scene, frame: vendor });
}

static DEMO_DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The weapon demo ended this frame (its actors deleted): the vendor's `VendorExit(1)` follows (crate::interact_render).
pub fn take_weapon_demo_done() -> bool { DEMO_DONE.swap(false, std::sync::atomic::Ordering::Relaxed) }

/// The weapon demo scene `unknown_1530[k]` (NTSC: k < 14; PAL k + 14).
fn demo_lump(k: usize) -> Option<SpaceLump> {
    let root = crate::level_load::extracted_root();
    let b = crate::disc_source::read(&root, &format!("global/unknown_1530/{k:03}.bin")).map_err(|e| eprintln!("travel: weapon demo {k}: {e:#}")).ok()?;
    let scene = Scene::from_lump(&b, k, Region::Ntsc).map_err(|e| eprintln!("travel: weapon demo {k}: {e}")).ok()?;
    Some(SpaceLump { index: k, scene: Arc::new(scene) })
}

fn trace() -> bool { std::env::var("RC_TRAVEL_TRACE").is_ok_and(|v| v.trim() == "1") }

/// A step's running state.
#[derive(Debug)]
enum StepState {
    /// Not started.
    New,
    Fade { n: u32, k: u32 },
    Card(CardPlayer),
    Movie,
    Flight,
    /// The swap was asked for: the next level's `entry` waits for its set-up.
    Swapped,
}

struct Transition {
    plan: Plan,
    step: usize,
    state: StepState,
    /// The view replaced by the black screen (the cards, the movies, the flight).
    blanked: bool,
}

/// The actors drawn for a space scene, the flight or the title world (crate::scene_render's actor scheme).
#[derive(Default)]
pub(crate) struct ActorGfx {
    actors: Vec<GfxActor>,
    extra: Option<ExtraMobys>,
    palette_len: u32,
    frames: u64,
    uploaded: Option<u64>,
    light_word: u32,
    ambient: [u8; 3],
    /// The lump whose actors these are.
    lump: Option<(usize, bool)>,
}

struct GfxActor {
    anim: MobyAnimClass,
    slot: u8,
    chunk: Option<usize>,
    scale: f32,
    base: u32,
    slots: u32,
    entities: Vec<Entity>,
    /// The glow word last written to the actor's slot (Clank's pulse: `FUN_00228bc0`).
    glow: Option<u32>,
}

/// The mode-6 and transition state.
#[derive(Resource, Default)]
pub struct Travel {
    pub mode: space::ShipMode,
    tr: Option<Transition>,
    /// After a level change: `entry`'s landing once the set-up has built the play state.
    entry_pending: bool,
    /// `RC_LANDING=1`: the boot's level runs `entry`'s landing too (done once).
    boot_entry: bool,
    gfx: ActorGfx,
    card: Option<CardFrame>,
    /// The HUD is not drawn (mode 6, the transition).
    hud_off: bool,
    /// The hero and his items hidden (`FUN_002486c0`).
    hero_hidden: bool,
    /// The world view replaced ([`TRAVEL_LAYER`]).
    blank: bool,
    blanked_now: bool,
    /// The last frame's actor poses and the lump they belong to.
    poses: Vec<ActorPose>,
    lump: Option<SpaceLump>,
    trace: bool,
}

impl Travel {
    /// The transition's black screen replaces the world view ([`TRAVEL_LAYER`], [`view_layers`]): crate::title_world
    /// gives the cameras back to it (until then the title world is the last image the transition's fades darken).
    pub(crate) fn blanked(&self) -> bool { self.blank }

    /// The fog of a running level transition (`DoSpaceTransition`), whose loops never run `UpdateFog`: its own block
    /// for the cards and movies, the reset view context's for the flight; None outside a transition (the level's).
    pub(crate) fn transition_fog(&self) -> Option<rc_game::fog_zones::FogGlobals> {
        use rc_game::fog_zones::{FLIGHT_FOG, TRANSITION_FOG};
        self.tr.as_ref().map(|tr| if matches!(tr.state, StepState::Flight) { FLIGHT_FOG } else { TRANSITION_FOG })
    }
}

/// Marks the hero's entities while mode 6 hides him.
#[derive(Component)]
pub(crate) struct TravelHidden;

pub struct TravelPlugin;

impl Plugin for TravelPlugin {
    fn build(&self, app: &mut App) {
        let boot_entry = std::env::var("RC_LANDING").is_ok_and(|v| v.trim() == "1");
        app.insert_resource(Travel { trace: trace(), boot_entry, ..Default::default() });
        if !crate::gameplay::enabled() { return; }
        app.add_systems(crate::level_switch::LevelUnload, level_unload)
            .add_systems(FixedUpdate, pre_tick.before(GameTick).after(crate::scene_render::SceneFrameSet))
            .add_systems(FixedUpdate, post_tick.after(GameTick).after(crate::scene_render::actor_mobys).before(crate::menu_render::MenuFrameSet))
            .add_systems(Update, hud_layer.after(MenuPrims).before(HudBuild))
            .add_systems(PostUpdate, (upload_actors, view_layers))
            .add_systems(PostUpdate, force_hidden.after(moby_render::update_moby_occlusion).before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate));
    }
}

impl Travel {
    /// The level being set up runs `entry` before its first tick ([`pre_tick`]: after a level change, or the
    /// `RC_LANDING=1` boot). `entry` stores game mode 6 (0x15f5c4) before `LoadLevelCoreData`'s load pass; its
    /// `ShipLandingStart` then keeps 6 (the landing) or stores 0.
    pub(crate) fn entry_follows(&self) -> bool {
        !crate::saves::front_end_active() && ((self.entry_pending && self.tr.is_some()) || (self.boot_entry && self.tr.is_none()))
    }
}

/// A level change drops the actors (their entities are gone) and arms the new level's `entry`.
fn level_unload(mut t: ResMut<Travel>, mut active: ResMut<ActiveScene>) {
    active.space_actors.clear();
    active.space_hero_hidden = false;
    t.gfx = ActorGfx::default();
    t.poses.clear();
    t.lump = None;
    t.hero_hidden = false;
    t.entry_pending = true;
}

/// The space-scene lump `anim_looking_thing_2[k]` (NTSC: k < 10).
fn space_lump(k: usize) -> Option<SpaceLump> {
    let root = crate::level_load::extracted_root();
    let b = crate::disc_source::read(&root, &format!("global/unknown_12e8/{k:03}.bin")).map_err(|e| eprintln!("travel: space scene {k}: {e:#}")).ok()?;
    let scene = Scene::from_lump(&b, k, Region::Ntsc).map_err(|e| eprintln!("travel: space scene {k}: {e}")).ok()?;
    Some(SpaceLump { index: k, scene: Arc::new(scene) })
}

/// `space_audio[k]` (stream id 40000 + k).
fn space_audio(k: i32) -> Option<Arc<[u8]>> {
    let root = crate::level_load::extracted_root();
    crate::disc_source::read(&root, &format!("global/space_audio/{k:03}.bin")).ok().map(Arc::from)
}

/// The joint lists of a ship class (531..533): from the level core when it has the class, else from the `spaceships`
/// file (the canopy glass's joint list 0, `Services::joint_lists`).
pub(crate) fn ship_joint_lists(o_class: i16) -> Option<Vec<Vec<u8>>> {
    let blob = crate::interact_render::class_blob(o_class as i32).ok().or_else(|| {
        let root = crate::level_load::extracted_root();
        let file = crate::disc_source::read(&root, &format!("global/spaceships/{:03}.bin", o_class - 530)).ok()?;
        Some(rc_formats::moby_spawn::parse_spaceship(&file).ok()?.class.to_vec())
    })?;
    let c = rc_formats::moby::parse_moby_class(&blob).ok()?;
    Some((0..16).map_while(|l| rc_formats::gadget::joint_list(&blob, &c.header, l).ok().map(|(a, _)| a)).collect())
}

/// A ship class (530..=533) or a ship's extra class (535..=537) from the `spaceships` file, its texture appended to the
/// moby texture table (crate::scene_render's `load_ship_class`).
fn load_spaceship_class(level: &mut crate::level_load::LoadedLevel, o_class: i32) -> anyhow::Result<(LevelMobyClass, MobyAnimClass)> {
    use anyhow::Context;
    let (entry, extra) = match o_class {
        530..=533 => (o_class - 530, false),
        535..=537 => (o_class - 534, true),
        _ => anyhow::bail!("class {o_class} is not a spaceship class"),
    };
    let root = crate::level_load::extracted_root();
    let file = crate::disc_source::read(&root, &format!("global/spaceships/{entry:03}.bin"))?;
    let s = rc_formats::moby_spawn::parse_spaceship(&file)?;
    let (blob, texture, size) = if extra { (s.extra_class, s.extra_texture, 128) } else { (s.class, s.texture, 256) };
    let class = rc_formats::moby::parse_moby_class(blob).context("parsing the ship class")?;
    let seqs = moby_anim::parse_sequences(blob, &class).context("parsing the ship sequences")?;
    let tex = level.textures.iter().filter(|t| t.table == TextureTable::Moby).map(|t| t.index + 1).max().unwrap_or(0);
    let slot = u8::try_from(tex).ok().filter(|&t| t != 0xff).context("moby texture table full")?;
    let mut textures = [0xff; 16];
    textures[0] = slot;
    let desc = TextureEntry { data_offset: 0, width: size, height: size, ty: 4, palette: 0, mipmap: 0, pad: 0 };
    level.textures.push(LevelTexture { table: TextureTable::Moby, index: tex, source: TextureSource::Entry(desc), texture });
    let anim = MobyAnimClass::new(&class, seqs);
    Ok((LevelMobyClass { o_class, entry: ClassEntry { offset_in_asset_wad: 0, o_class, unknown_8: 0, unknown_c: 0, textures }, class }, anim))
}

/// The World of the play state (the mode-6 code runs on the level's moby table, outside the tick).
fn with_world<R>(p: &mut Play, coll: Option<&rc_formats::collision::Collision>, f: impl FnOnce(&mut World) -> R) -> R { with_world_view(p, coll, None, f) }

/// [`with_world`] with the last drawn frame's `FastBSphereCheck` view (the ship shadow's depth fade reads it).
fn with_world_view<R>(p: &mut Play, coll: Option<&rc_formats::collision::Collision>, view: Option<&rc_game::particles::BSphereView>, f: impl FnOnce(&mut World) -> R) -> R {
    let classes = p.classes.clone();
    let cam = p.game.camera.out.pos;
    let mut w = World::new(&mut p.game.mobys, &p.game.hero, &mut p.game.rng, &*classes, &mut p.svc, p.game.counter);
    w.camera = cam;
    w.coll = coll;
    w.view = view;
    f(&mut w)
}

/// The requests, the level's entry, the world frames of mode 6 (module docs).
#[allow(clippy::too_many_arguments)]
fn pre_tick(
    mut t: ResMut<Travel>,
    play: Option<ResMut<Play>>,
    mut mm: ResMut<MenuMode>,
    mut gs: Option<ResMut<Persistent>>,
    level: Res<crate::Level>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
) {
    let t = &mut *t;
    let Some(mut play) = play else { return };
    let p = &mut *play;
    // `entry` after a level change: the landing (the set-up has built the play state).
    if std::mem::take(&mut t.entry_pending) && t.tr.is_some() {
        level_entry(t, p, &mut mm, gs.as_deref_mut(), &level.0, audio.as_deref_mut());
    }
    // `RC_LANDING=1`: the boot's level enters as the game enters every level (`entry` 0x259c40 after
    // `DoSpaceTransition`: the music pause / unpause and `ShipLandingStart`); the default boot starts in mode 0 at the
    // level's start (or `RC_HERO_AT`), a debug start the game has no equivalent of.
    if t.boot_entry && t.tr.is_none() && !crate::saves::front_end_active() {
        t.boot_entry = false;
        println!("travel: RC_LANDING=1: the boot's level entry with its landing");
        level_entry(t, p, &mut mm, gs.as_deref_mut(), &level.0, audio.as_deref_mut());
    }
    // `0x15f570` is one flag: setting it again while `DoSpaceTransition` runs asks for nothing more (the transition
    // clears it), so a leave request made during a transition is dropped (Veldin's Clank 834 asks on two ticks).
    if t.tr.is_some() {
        let mut q = queue();
        let before = q.len();
        q.retain(|r| !matches!(r, Request::Leave(_)));
        if t.trace && q.len() != before { println!("travel: a leave request during the transition dropped (0x15f570 already set)"); }
    }
    if t.tr.is_none() {
        let reqs: Vec<Request> = queue().drain(..).collect();
        for r in reqs {
            match r {
                Request::Leave(dest) if t.tr.is_none() => {
                    if let Some(g) = gs.as_deref() {
                        let plan = transition::plan(&g.0, dest, &|n| p.svc.ticks(n));
                        if t.trace { println!("travel: DoSpaceTransition({dest}) from {}: {:?}, ship {}", plan.from, plan.steps, plan.ship); }
                        t.tr = Some(Transition { plan, step: 0, state: StepState::New, blanked: false });
                    }
                }
                Request::ShipTravel(dest) => ship_travel(t, p, &mut mm, gs.as_deref_mut(), &level.0, dest),
                Request::Demo { scene, frame } => match usize::try_from(scene).ok().and_then(demo_lump) {
                    Some(l) => {
                        with_world(p, level.0.collision.as_ref(), |w| {
                            let fr = space::DemoFrame::of_moby(w, frame, space::VENDOR_DEMO_OFFSET, space::VENDOR_DEMO_YAW);
                            t.mode.demo_start(w, l.clone(), fr, space::DEMO_STREAM);
                        });
                        start_actors(t, l);
                    }
                    None => {
                        eprintln!("travel: weapon demo {scene}: no scene: the vendor exits");
                        DEMO_DONE.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                },
                Request::Leave(_) => {}
            }
        }
    }
    // The world part of GameStateUpdate on the mode-6 frames that run it.
    t.mode.flight.loaded = false;
    let runs = t.tr.is_none() && t.mode.phase == space::Phase::Run && matches!(t.mode.sub, space::SUB_TAKEOFF | space::SUB_LANDING | space::SUB_FLYAWAY);
    // The weapon demo (the vendor's substate 3): its world part every frame it runs.
    let demo = t.tr.is_none() && t.mode.phase == space::Phase::Run && t.mode.sub == space::SUB_DEMO && mm.state.mode == Mode::Vendor;
    if demo {
        mm.world_tick = true;
        p.game.camera_paused = true;
    }
    if runs && mm.state.mode == Mode::Ship {
        mm.world_tick = true;
        // The tick in its scene form: subs 0 / 8 have no `CameraUpdate`; sub 3's runs after its body
        // ([`fly_away_camera`] in post_tick), so the camera targets the body sets reach the camera in the same tick.
        p.game.camera_paused = true;
        p.svc.game_mode = 6;
    }
}

/// `ShipTravelTo(dest)` (module docs): the revisit landing on the current planet, else `memcard_Save(0, dest)` and the
/// fly-away.
fn ship_travel(t: &mut Travel, p: &mut Play, mm: &mut MenuMode, gs: Option<&mut Persistent>, lv: &crate::level_load::LoadedLevel, dest: i32) {
    if let Some(g) = gs.as_deref() { sync_mirrors(p, &g.0); }
    let mut out = SpaceFrame::default();
    let level = p.svc.level as i32;
    let ship = p.svc.travel.ship;
    let o2 = p.game.hero.owned.0.get(space::ITEM_O2_MASK).is_some_and(|&b| b != 0);
    let landing = (dest == level).then(|| space::lump_and_stream(space::SUB_LANDING, level, ship, o2, false).0).and_then(space_lump);
    let revisit = with_world(p, lv.collision.as_ref(), |w| t.mode.travel_to(w, dest, landing.clone(), &mut out));
    if !revisit {
        if let Some(g) = gs { crate::saves::memcard_save(p, &mut g.0, false, dest); }
    } else if let Some(l) = landing {
        start_actors(t, l);
    }
    mm.state.set(Mode::Ship);
    apply_audio(&out.audio);
    if t.trace { println!("travel: ShipTravelTo({dest}): {}", if revisit { "the revisit landing (sub 8 at tick 240)" } else { "the fly-away (sub 3)" }); }
}

/// The saved-game mirrors the moby loop reads (the tick refreshes them before its moby loop; mode 6's entries run outside
/// it): the talk conditions' values (the planet bits, the global flags) and Ratchet's owned items.
fn sync_mirrors(p: &mut Play, gs: &rc_game::game_state::GameState) {
    p.svc.interact.sync_game(gs);
    p.game.hero.owned.0 = gs.global.owned;
}

/// The new level's `entry` after a level change (level01 0x259c40): `ShipLandingStart` 0x2a29c0, then
/// `music_start_track(0, 1, 0x400)` and `music_Unpause` on the first Novalis arrival (level 1, planet 3 locked) and on
/// Veldin with global flag 8, else `music_Pause(0)` (the port's order: the new audio system starts its track on its first
/// EE frame, `AudioSystem::game_frame_with`, before that frame's scene inbox applies the pause; so the pause follows the
/// start as in `entry`); the landing's end unpauses it (`ShipMode::scene_end`).
fn level_entry(t: &mut Travel, p: &mut Play, mm: &mut MenuMode, gs: Option<&mut Persistent>, lv: &crate::level_load::LoadedLevel, _audio: Option<&mut crate::audio_out::AudioOut>) {
    t.tr = None;
    t.card = None;
    t.blank = false;
    t.hud_off = false;
    // Quit Game's front end (crate::saves): the menus' set-up runs it over this level; no landing.
    if crate::saves::front_end_active() {
        println!("travel: level {:02} loaded behind the front end", p.svc.level);
        return;
    }
    // The saved game as the tick mirrors it (the set-up ran no tick yet).
    if let Some(g) = gs.as_deref() { sync_mirrors(p, &g.0); }
    let level = p.svc.level as i32;
    let p3 = p.svc.interact.game.planet_unlocked.get(3).is_some_and(|&b| b != 0);
    let flag8 = gs.as_deref().is_some_and(|g| g.0.global.flags[8] != 0);
    if !((level == 1 && !p3) || (level == 0 && flag8)) { scene_audio::post(SceneAudioCmd::PauseMusic); }
    let ship = p.svc.travel.ship;
    let o2 = p.game.hero.owned.0.get(space::ITEM_O2_MASK).is_some_and(|&b| b != 0);
    let lump = space_lump(space::lump_and_stream(space::SUB_LANDING, level, ship, o2, true).0);
    // 0x13d3f0 = global flag 0x68 (level 14's landing).
    let flag = gs.as_deref().is_some_and(|g| g.0.global.flags[0x68] != 0);
    let mut out = SpaceFrame::default();
    let lands = with_world(p, lv.collision.as_ref(), |w| t.mode.landing_start(w, lump.clone(), flag, &mut out));
    if lands {
        mm.state.set(Mode::Ship);
        if let Some(l) = lump { start_actors(t, l); }
        t.hero_hidden = true;
    } else {
        // Mode 0 at once: the music stays as `entry` left it (paused but on the first Novalis arrival and on Veldin with
        // global flag 8; the level's story scene that follows resumes it at its end).
        mm.state.set(Mode::Gameplay);
    }
    apply_audio(&out.audio);
    println!("travel: level {level:02} entry: {}", if lands { "the landing (mode 6, sub 8)" } else { "mode 0 at once" });
}

/// The space scene's actors are set up from the lump on the next upload.
fn start_actors(t: &mut Travel, l: SpaceLump) {
    t.lump = Some(l);
    t.gfx = ActorGfx::default();
    t.poses.clear();
}

fn apply_audio(reqs: &[SpaceAudio]) {
    for r in reqs {
        let cmd = match *r {
            SpaceAudio::PauseMusic => SceneAudioCmd::PauseMusic,
            SpaceAudio::UnpauseMusic => SceneAudioCmd::ResumeMusic { after: 0 },
            SpaceAudio::StopStream => SceneAudioCmd::StopSpeech,
            SpaceAudio::Stream(k) => match space_audio(k) {
                Some(vag) => SceneAudioCmd::Speech { vag },
                None => {
                    eprintln!("travel: no space audio stream {k}");
                    continue;
                }
            },
            SpaceAudio::VendorStream(k) => {
                let root = crate::level_load::extracted_root();
                match crate::disc_source::read(&root, &format!("global/vendor_audio/{k:03}.bin")) {
                    Ok(b) => SceneAudioCmd::Speech { vag: Arc::from(b) },
                    Err(e) => {
                        eprintln!("travel: no vendor audio stream {k}: {e:#}");
                        continue;
                    }
                }
            }
            // The flight's sound bank and its sound v (crate::flight_render).
            SpaceAudio::FlightBank => {
                crate::flight_render::post_audio(crate::flight_render::FlightAudio::Bank);
                continue;
            }
            SpaceAudio::FlightSound(v) => {
                crate::flight_render::post_audio(crate::flight_render::FlightAudio::Sound(v));
                continue;
            }
        };
        scene_audio::post(cmd);
    }
}

/// The main camera's transform and projection (the view of the last drawn frame).
type ViewCams<'w, 's> = Query<'w, 's, (&'static Transform, &'static Projection), (With<FlyCam>, Without<crate::sky_render::SkyCamera>)>;

/// The take-off, the mode-6 frame and its outputs, the transition's steps (module docs).
#[allow(clippy::too_many_arguments)]
fn post_tick(
    mut t: ResMut<Travel>,
    play: Option<ResMut<Play>>,
    mut mm: ResMut<MenuMode>,
    mut gs: Option<ResMut<Persistent>>,
    mut session: Option<ResMut<Session>>,
    level: Res<crate::Level>,
    mut active: ResMut<ActiveScene>,
    mut change: ResMut<LevelChange>,
    movie: Res<crate::movie_render::MovieState>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
    mut flight: ResMut<crate::flight_render::FlightRender>,
    cams: ViewCams,
) {
    let t = &mut *t;
    let Some(mut play) = play else { return };
    let p = &mut *play;
    let lv = &level.0;
    if let Some(mut tr) = t.tr.take() {
        let done = transition_frame(t, &mut tr, p, &mut mm, gs.as_deref_mut(), session.as_deref_mut(), lv, &mut active, &mut change, &movie, audio.as_deref_mut(), &mut flight);
        if !done { t.tr = Some(tr); }
        return;
    }
    // InLevelFrameUpdate: `0x15f630` (the ship's △ of the last tick) with HP ≠ 0 → ShipTakeOff.
    if mm.state.mode == Mode::Gameplay && std::mem::take(&mut p.svc.travel.take_off) && p.game.hero.health != 0 {
        if let Some(g) = gs.as_deref() { sync_mirrors(p, &g.0); }
        let level = p.svc.level as i32;
        let ship = p.svc.travel.ship;
        let o2 = p.game.hero.owned.0.get(space::ITEM_O2_MASK).is_some_and(|&b| b != 0);
        let (k, _) = space::lump_and_stream(space::SUB_TAKEOFF, level, ship, o2, false);
        match space_lump(k) {
            Some(l) => {
                let mut out = SpaceFrame::default();
                with_world(p, lv.collision.as_ref(), |w| t.mode.take_off(w, l.clone(), &mut out));
                apply_audio(&out.audio);
                start_actors(t, l);
                t.hero_hidden = true;
                t.hud_off = true;
                mm.state.set(Mode::Ship);
                println!("travel: tick {}: ShipTakeOff: mode 6, sub 0, space scene {k} ({} ticks)", p.game.counter, t.lump.as_ref().map_or(0, |l| l.scene.end_tick()));
            }
            None => eprintln!("travel: the take-off scene {k} is missing: no take-off"),
        }
        return;
    }
    let demo = t.mode.sub == space::SUB_DEMO && mm.state.mode == Mode::Vendor;
    if (mm.state.mode != Mode::Ship && !demo) || !t.mode.active() { return; }
    let pressed = p.game.pad.pressed;
    // The vendor's frame reads the flag after the demo's (its `sound_update` on the frames without a world tick).
    let world_ran = if demo { mm.world_ticked } else { std::mem::take(&mut mm.world_ticked) };
    let fly = t.mode.sub == space::SUB_FLYAWAY;
    // The view of the last drawn frame (0x16d140): the main camera with its projection's tangents.
    let view = cams.iter().next().map(|(tr, proj)| crate::particle_render::bsphere_view(tr, crate::particle_render::view_tans(Some(proj))));
    let out = with_world_view(p, lv.collision.as_ref(), view.as_ref(), |w| t.mode.frame(w, pressed, world_ran));
    // GameStateUpdate sub 3: `CameraUpdate` after the body (the tick ran without its camera update).
    if fly && world_ran {
        if let Some(coll) = lv.collision.as_ref() { fly_away_camera(p, coll); }
    }
    apply_frame(t, p, &out, &mut active);
    match out.event {
        Some(SpaceEvent::PlanetPage) => {
            // GameStateUpdate's end of the take-off: EnterMenuMode(0) with kind 0xe (the planet select page).
            mm.state.set(Mode::Gameplay);
            crate::media_render::request_menu(0xe);
            t.hud_off = false;
            println!("travel: tick {}: the take-off ended: the planet page (kind 0xe)", p.game.counter);
        }
        Some(SpaceEvent::Landed) => {
            mm.state.set(Mode::Gameplay);
            t.hero_hidden = false;
            active.space_hero_hidden = false;
            active.space_actors.clear();
            t.hud_off = false;
            p.game.camera_paused = false;
            active.camera = None;
            active.white = 0.0;
            p.svc.cinematic.fade = 0.0;
            println!("travel: tick {}: the landing ended: mode 0", p.game.counter);
        }
        Some(SpaceEvent::Leave { dest }) => {
            active.camera = None;
            p.game.camera_paused = false;
            println!("travel: tick {}: the fly-away ended: 0x15f570 = 1, destination {dest}", p.game.counter);
            request_leave(dest);
        }
        Some(SpaceEvent::DemoDone) => {
            // The demo's end (`VendorModeUpdate` substate 3): the play camera back, the vendor's `VendorExit(1)` next
            // (crate::interact_render).
            active.camera = None;
            active.space_actors.clear();
            t.hud_off = false;
            t.lump = None;
            t.poses.clear();
            DEMO_DONE.store(true, std::sync::atomic::Ordering::Relaxed);
            println!("travel: tick {}: the weapon demo ended", p.game.counter);
        }
        Some(SpaceEvent::FlightDone) | None => {}
    }
}

/// Sub 3's `CameraUpdate` after the fly-away's body (`GameStateUpdate` 0x2a4080: the body's `CameraScript` / camera
/// targets, then `CameraUpdate`, then `sound_update`): the camera calls the body queued, the camera update with the moby
/// scene the tick's would use, the camera moby's create / delete (`Camera_handleCollWithHero`).
fn fly_away_camera(p: &mut Play, coll: &rc_formats::collision::Collision) {
    use rc_game::follow_camera::CamInput;
    let calls = rc_game::cinematic::take_calls(&mut p.svc);
    let hero_moby = Some(p.game.hero_moby);
    {
        let inp = CamInput { hero: &p.game.hero, pad: &p.game.pad, coll, mobys: None, hero_moby };
        rc_game::cinematic::apply_camera_calls(&mut p.game.camera, &calls, &inp);
    }
    let scene = p.svc.hero_scene(&p.game.mobys, p.classes.clone(), None);
    {
        let sc = scene.scene();
        let inp = CamInput { hero: &p.game.hero, pad: &p.game.pad, coll, mobys: Some(&sc), hero_moby };
        p.game.camera.update(&inp);
    }
    drop(scene);
    match p.game.camera.cam_moby_call.take() {
        Some(rc_game::follow_camera::camera_moby::Call::Delete(id)) => {
            with_world(p, Some(coll), |w| w.delete_moby(id));
            p.game.camera.camera_moby_created(None);
        }
        // The follow camera is not current in the fly-away (the script camera is): no create [L: a create would
        // need the tick's loader hook; logged].
        Some(rc_game::follow_camera::camera_moby::Call::Create { .. }) => eprintln!("travel: fly-away: camera moby create skipped"),
        None => {}
    }
}

/// A mode-6 frame's outputs (module docs).
fn apply_frame(t: &mut Travel, p: &mut Play, out: &SpaceFrame, active: &mut ActiveScene) {
    active.camera = out.camera.or(if t.mode.sub == space::SUB_FLYAWAY { None } else { active.camera });
    // The scene camera is the game's 0x167240 / rows (the fly-away's `CameraScript` starts from it; the listener).
    if let Some(c) = out.camera {
        use rc_game::ps2v::Pf;
        let v = |x: [f32; 3]| [Pf::f(x[0]), Pf::f(x[1]), Pf::f(x[2]), Pf::ZERO];
        let o = &mut p.game.camera.out;
        o.pos = [Pf::f(c.eye[0]), Pf::f(c.eye[1]), Pf::f(c.eye[2]), o.pos[3]];
        o.rows = c.rows.map(v);
    }
    p.svc.cinematic.fade = out.black;
    active.white = out.white;
    // The take-off / landing actors' shadows (crate::shadow_render), and the hero's hidden while mode 6 hides him.
    active.space_actors = out.scene_actors.clone();
    active.space_hero_hidden = t.hero_hidden;
    apply_audio(&out.audio);
    t.poses = out.actors.clone();
    t.hud_off = true;
    t.gfx.frames += 1;
}

/// One frame of the transition; true when it is over (module docs).
#[allow(clippy::too_many_arguments)]
fn transition_frame(
    t: &mut Travel,
    tr: &mut Transition,
    p: &mut Play,
    mm: &mut MenuMode,
    mut gs: Option<&mut Persistent>,
    session: Option<&mut Session>,
    lv: &crate::level_load::LoadedLevel,
    active: &mut ActiveScene,
    change: &mut LevelChange,
    movie: &crate::movie_render::MovieState,
    mut audio: Option<&mut crate::audio_out::AudioOut>,
    flight: &mut crate::flight_render::FlightRender,
) -> bool {
    let loaded = change.loaded();
    loop {
        let Some(&step) = tr.plan.steps.get(tr.step) else { return true };
        let next = match (&mut tr.state, step) {
            (StepState::New, Step::Begin) => {
                // 0x1742d0 |= 0x80000000, mode 6, the ship index, all sounds and the music stopped, the black background.
                mm.state.set(Mode::Ship);
                p.svc.game_mode = 6;
                p.game.camera_paused = true;
                if let Some(a) = audio.as_deref_mut() { a.system().movie_stop(); }
                scene_audio::post(SceneAudioCmd::StopSpeech);
                crate::level_load::set_ship(tr.plan.ship);
                p.svc.travel.ship = tr.plan.ship;
                t.hud_off = true;
                t.hero_hidden = true;
                true
            }
            (StepState::New, Step::Fade(n)) => {
                tr.state = StepState::Fade { n: n.max(1), k: 0 };
                false
            }
            (StepState::Fade { n, k }, Step::Fade(_)) => {
                p.svc.cinematic.fade = rc_game::scene_player::fade_to_black_coverage(*n, (*k).min(*n - 1));
                *k += 1;
                if *k >= *n {
                    // From here the screen is black (the cards / movies / flight over the black background).
                    tr.blanked = true;
                    t.blank = true;
                    p.svc.cinematic.fade = 0.0;
                    tr.state = StepState::New;
                    tr.step += 1;
                }
                return false;
            }
            (StepState::New, Step::Card { a, b, ticks, load }) => {
                tr.blanked = true;
                t.blank = true;
                tr.state = StepState::Card(CardPlayer::new(a, b, ticks, load));
                false
            }
            (StepState::Card(c), Step::Card { .. }) => {
                let f = c.frame(loaded);
                let done = f.done;
                t.card = Some(f);
                if done {
                    t.card = None;
                    tr.state = StepState::New;
                    tr.step += 1;
                }
                return false;
            }
            (StepState::New, Step::Movie(n)) => {
                tr.blanked = true;
                t.blank = true;
                let req = crate::movie_render::MovieRequest { exit: crate::movie_render::MovieExit::FrontEnd, replay: Some(0), ..crate::movie_render::MovieRequest::movie_b(n) };
                crate::movie_render::request(req);
                tr.state = StepState::Movie;
                return false;
            }
            (StepState::Movie, Step::Movie(_)) => {
                if movie.busy() { return false; }
                mm.state.set(Mode::Ship);
                true
            }
            (StepState::New, Step::SetLevel) => {
                // The branch's saved-game writes and 0x15ed84 = dest; the load of dest starts (read_file_entry_with_retry).
                if let Some(g) = gs.as_deref_mut() { g.0.apply_transition(tr.plan.dest); }
                if tr.plan.dest >= 0 {
                    let dest = tr.plan.dest as u32;
                    change.start_load(dest, crate::gameplay::level_spawn_save(gs.as_deref().map(|g| &g.0), dest));
                }
                true
            }
            (StepState::New, Step::Flight) => {
                tr.blanked = true;
                t.blank = true;
                // EnterSpaceLoadingLoop: the transition lump (its variants, sky, pictures, FX bank, sound bank:
                // crate::flight_render) and the actors' skeletons.
                let variants = crate::flight_render::variants();
                let rigs = crate::flight_render::rigs();
                let (first, dest) = (tr.plan.first_arrival, tr.plan.dest);
                let mut out = SpaceFrame::default();
                with_world(p, lv.collision.as_ref(), |w| t.mode.flight_start(w, variants, first, dest, rigs, &mut out));
                apply_audio(&out.audio);
                flight.active = true;
                flight.draw = None;
                t.lump = None;
                t.gfx = ActorGfx::default();
                tr.state = StepState::Flight;
                return false;
            }
            (StepState::Flight, Step::Flight) => {
                t.mode.flight.loaded = loaded;
                let out = with_world(p, lv.collision.as_ref(), |w| t.mode.frame(w, 0, false));
                t.lump = t.mode.lump.clone();
                apply_frame(t, p, &out, active);
                flight.draw = out.flight.clone();
                if out.event != Some(SpaceEvent::FlightDone) { return false; }
                active.camera = None;
                flight.active = false;
                flight.draw = None;
                true
            }
            (StepState::New, Step::FrontEnd) => {
                // load_level_chunk_from_disc with level −1: the front end. The port's front end runs over a loaded level
                // (crate::saves, crate::menu_render's set-up starts it when the flag is set): the current level is loaded
                // again behind it for the menus' data; the title world itself is drawn over it (crate::title_world).
                println!("travel: DoSpaceTransition(−1): the front end over level {:02} (crate::saves)", crate::level_load::level_index());
                crate::saves::set_front_end_active(true);
                let index = crate::level_load::level_index();
                change.start_load(index, crate::gameplay::level_spawn_save(gs.as_deref().map(|g| &g.0), index));
                tr.state = StepState::New;
                tr.step += 1;
                tr.plan.steps.insert(tr.step, Step::Enter);
                tr.plan.dest = -1;
                return false;
            }
            (StepState::New, Step::Enter) => {
                if !loaded { return false; }
                // The level start's saved-game rules (`FUN_00251c30` / `FUN_00251da0` / the hero init / visited / record 3007)
                // with the destination's item tables, before its set-up reads the save.
                if let (Some(g), Some(s), true) = (gs.as_deref_mut(), session, tr.plan.dest >= 0) {
                    match dest_items(tr.plan.dest) {
                        Some(items) => g.0.apply_level_start(tr.plan.dest, &items, &mut s.0),
                        None => eprintln!("travel: level {:02}: no item tables: the level start's rules not applied", tr.plan.dest),
                    }
                }
                change.swap();
                tr.state = StepState::Swapped;
                return false;
            }
            (StepState::Swapped, _) => {
                // The swap ran at the end of the last frame; the entry runs when the set-up has built the play state.
                return false;
            }
            (_, s) => {
                eprintln!("travel: unexpected transition step {s:?} in state {:?}", tr.state);
                true
            }
        };
        if next {
            tr.state = StepState::New;
            tr.step += 1;
            if t.trace { println!("travel: tick {}: transition step {:?} done; next {:?}", p.game.counter, step, tr.plan.steps.get(tr.step)); }
        }
    }
}

/// The destination's item tables (`ItemTables::load` with its overlay).
fn dest_items(dest: i32) -> Option<rc_formats::save_game::ItemTables> {
    let root = crate::level_load::extracted_root();
    let elf = crate::disc_source::read_path(&root, &root.join("boot/SCUS_971.99")).ok()?;
    let ov = crate::disc_source::level_file(&root, u32::try_from(dest).ok()?, "overlay.bin").ok()?;
    rc_formats::save_game::ItemTables::load(&elf, &ov).ok()
}

/// The HUD in mode 6 and the transition (module docs): no HUD; the black screen and the story cards.
fn hud_layer(
    t: Res<Travel>,
    mut hook: ResMut<Hud2dHook>,
    mut images: Option<ResMut<crate::hud_images::HudImages>>,
    mut assets: ResMut<Assets<Image>>,
    mut plates: Local<Option<(usize, Arc<Plates>)>>,
    mut flight: ResMut<crate::flight_render::FlightRender>,
) {
    if !t.hud_off && !t.blank && t.card.is_none() { return; }
    hook.replace_hud = true;
    hook.freeze = true;
    if !t.blank && t.card.is_none() { return; }
    let mut h = Hud2d::default();
    // The black frame the transition draws on (the flight shows through: its actors are on the travel layer).
    if t.card.is_some() || (t.blank && t.lump.is_none()) { h.rect(0, crate::hud_render::H, 0, crate::hud_render::W, 0x8000_0000); }
    if let (Some(card), Some(img)) = (t.card.as_ref(), images.as_deref_mut()) {
        let lang = rc_game::travel::cards::lang_index(crate::hud_render::language() as i32);
        if plates.as_ref().is_none_or(|p| p.0 != lang) {
            let root = crate::level_load::extracted_root();
            *plates = crate::disc_source::read(&root, &format!("global/space_plates/{lang:03}.bin")).ok().and_then(|b| Plates::parse(&b)).map(|p| (lang, Arc::new(p)));
            if plates.is_none() { eprintln!("travel: space_plates {lang} not read: no story cards"); }
        }
        if let Some((_, pl)) = plates.as_ref() {
            for line in &card.lines {
                let rgba = line.alpha << 24 | 0x80_8080;
                // The band: the 64×64 texture at s 0..4, t over 0.4 of it, scrolling (one 512×64 picture per frame).
                let band = band_picture(&pl.band, line.band_t);
                // One picture per frame for both lines (their bands scroll together).
                let key = (line.band_t[0] * 600.0) as u64;
                if let Some(s) = img.resolve(&rc_game::menus::ImageSrc::Pixels { key: 0x7a00_0000 | key, tex: Arc::new(band) }, &mut assets) {
                    h.prims.push(crate::hud_images::prim(s, 0, line.y, 0x200, 0x40, 0, 0, 0x200, 0x40, rgba));
                }
                if let Some(tex) = pl.cards.get(line.card) {
                    if let Some(s) = img.resolve(&rc_game::menus::ImageSrc::Pixels { key: 0x7b00_0000 | line.card as u64, tex: Arc::new(tex.clone()) }, &mut assets) {
                        h.prims.push(crate::hud_images::prim(s, 0, line.y, 0x200, 0x40, 0, 0, 0x200, 0x40, rgba));
                    }
                }
            }
        }
        // The closing FadeToBlack(2) over the last card image.
        if card.black > 0.0 { h.rect(0, crate::hud_render::H, 0, crate::hud_render::W, ((card.black * 128.0) as u32) << 24); }
    }
    // The flight's caption and card icon (crate::flight_render).
    if let Some(img) = images.as_deref_mut() {
        let busy = crate::saves::with_card(|c| c.state > 2 || c.req_state >= 0).unwrap_or(false);
        crate::flight_render::hud_prims(&mut flight, &mut h, img, &mut assets, crate::hud_render::language(), busy);
    }
    hook.prims.extend(h.prims);
}

/// The band strip of one frame: 512×64 texels sampling the 64×64 band at s = x/128 (0..4 repeats), t = t0 + y/64·0.4.
fn band_picture(band: &rc_formats::texture::Texture, t: [f32; 2]) -> rc_formats::texture::Texture {
    let (bw, bh) = (band.width.max(1), band.height.max(1));
    let mut rgba = vec![0u8; 512 * 64 * 4];
    for y in 0..64u32 {
        let v = t[0] + (t[1] - t[0]) * (y as f32 + 0.5) / 64.0;
        let ty = ((v.rem_euclid(1.0)) * bh as f32) as u32 % bh;
        for x in 0..512u32 {
            let tx = ((x as f32 + 0.5) / 128.0 * bw as f32) as u32 % bw;
            let s = ((ty * bw + tx) * 4) as usize;
            let d = ((y * 512 + x) * 4) as usize;
            rgba[d..d + 4].copy_from_slice(&band.rgba[s..s + 4]);
        }
    }
    rc_formats::texture::Texture { width: 512, height: 64, rgba }
}

/// The scene / flight actors as extra mobys (module docs).
#[allow(clippy::too_many_arguments)]
fn upload_actors(
    mut commands: Commands,
    mut t: ResMut<Travel>,
    mut level: ResMut<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut q: Query<(&mut Transform, &mut Visibility)>,
    play: Option<Res<Play>>,
) {
    let t = &mut *t;
    let Some(lump) = t.lump.clone() else {
        if !t.gfx.actors.is_empty() { despawn_gfx(&mut t.gfx, &mut commands); }
        return;
    };
    let flight = t.mode.sub == space::SUB_FLIGHT;
    if t.gfx.lump != Some((lump.index, flight)) {
        despawn_gfx(&mut t.gfx, &mut commands);
        t.gfx.lump = Some((lump.index, flight));
        let layer = flight.then(|| RenderLayers::layer(TRAVEL_LAYER));
        if let Err(e) = spawn_gfx(&mut t.gfx, &mut commands, &mut level.0, &lump.scene, t.mode.actors.iter().map(|a| a.1).collect(), layer, None, &mut meshes, &mut images, &mut materials, &mut buffers) {
            warn!("travel: space actors not drawn: {e:#}");
        }
    }
    if t.poses.is_empty() {
        hide_gfx(&t.gfx, &mut q);
        return;
    }
    if t.gfx.uploaded == Some(t.gfx.frames) { return; }
    t.gfx.uploaded = Some(t.gfx.frames);
    upload_gfx(&mut t.gfx, &level.0, &lump.scene, &t.poses, play.as_deref().map(|p| &p.game.mobys), &mut commands, &mut buffers, &mut q);
}

/// One frame of a scene's actors (module docs): each actor's slot re-pointed at its chunk's streamed sequence, its pose
/// evaluated into the palette (with its table moby's joint modifiers: the head manipulators of `FUN_0024a1d0` /
/// `FUN_00228bc0`), its instance record (ship-local rows from its yaw, its lights), its glow word (mode 0x10: Clank's
/// pulse), its entities moved and shown (hidden when it has no pose or is hidden).
#[allow(clippy::too_many_arguments)]
pub(crate) fn upload_gfx(
    g: &mut ActorGfx,
    lv: &crate::level_load::LoadedLevel,
    scene: &Scene,
    poses: &[ActorPose],
    table: Option<&rc_game::moby_runtime::MobyTable>,
    commands: &mut Commands,
    buffers: &mut Assets<ShaderBuffer>,
    q: &mut Query<(&mut Transform, &mut Visibility)>,
) {
    let mut palette = crate::moby_anim::identity_palette(g.palette_len);
    let mut records = Vec::with_capacity(g.actors.len() * moby_render::EXTRA_RECORD_SIZE);
    let lighting = lv.mobys.lighting.as_ref();
    let (lw, amb) = (g.light_word, g.ambient);
    let mut shown = Vec::new();
    for (k, a) in g.actors.iter_mut().enumerate() {
        let Some(pose) = poses.iter().find(|p| p.actor == k) else {
            shown.push((a.entities.clone(), None));
            records.extend_from_slice(&moby_render::extra_record(&Mat4::IDENTITY, None, a.base));
            continue;
        };
        if a.chunk != Some(pose.chunk) {
            // FUN_00259288: the slot re-pointed at this chunk's sequence.
            if let Some(q) = a.anim.sequences.get_mut(a.slot as usize) {
                *q = scene.chunks.get(pose.chunk).and_then(|c| c.actors.get(pose.actor)).map(|x| x.sequence.clone());
            }
            a.chunk = Some(pose.chunk);
        }
        let mods = pose.moby.and_then(|id| table.and_then(|t| t.mobys.get(id))).map(|m| m.joint_mods.as_slice()).unwrap_or(&[]);
        let f = match pose.head {
            // The flight: sequence 2 frame 0 with the streamed frame's joints 0..3 (`space::flight_sequence`), lent to
            // the slot for this evaluation.
            Some(h) => {
                let streamed = scene.chunks.get(pose.chunk).and_then(|c| c.actors.get(pose.actor)).map(|x| &x.sequence);
                let patched = streamed.and_then(|q| space::flight_sequence(&a.anim, q, h as usize));
                let s = AnimState { seq_a: a.slot, frame_a: 0, seq_b: a.slot, frame_b: 0, t: pose.t, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: true };
                let keep = a.anim.sequences.get_mut(a.slot as usize).map(|q| std::mem::replace(q, patched));
                let f = moby_anim::evaluate_posed(&a.anim, &s, None, &[], mods);
                if let (Some(k), Some(q)) = (keep, a.anim.sequences.get_mut(a.slot as usize)) { *q = k; }
                f
            }
            None => {
                let s = AnimState { seq_a: a.slot, frame_a: pose.frame_a, seq_b: a.slot, frame_b: pose.frame_b, t: pose.t, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: true };
                moby_anim::evaluate_posed(&a.anim, &s, None, &[], mods)
            }
        };
        let at = a.base as usize * 64;
        for (i, b) in f.iter().take(a.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() {
            if let Some(x) = palette.get_mut(at + i) { *x = b; }
        }
        // MobyBuildMatrix from (0, 0, yaw): the ship-local actors turn with the ship.
        let (c, sn) = (pose.yaw.cos(), pose.yaw.sin());
        let rows = [[c, sn, 0.0], [-sn, c, 0.0], [0.0, 0.0, 1.0]];
        let rbits: [rc_formats::moby_light::V4; 3] = rows.map(|r| [r[0].to_bits(), r[1].to_bits(), r[2].to_bits(), 0]);
        let lights = lighting.map(|l| rc_formats::moby_light::moby_lights(&rbits, &l.bank, lw, amb, 0x80));
        let model = moby_render::extra_model(rows, pose.scale.unwrap_or(a.scale), pose.position);
        records.extend_from_slice(&moby_render::extra_record(&model, lights.as_ref(), a.base));
        shown.push((a.entities.clone(), (!pose.hidden).then_some(model)));
        // The glow list: the moby's +0x90 while mode 0x10 (`moby_lod::glow_word`).
        let word = pose.glow.map(|w| crate::moby_lod::glow_word(0x10, w));
        if word != a.glow {
            if let Some(extra) = &g.extra {
                match word {
                    Some(w) => extra.set_glow(commands, k as u32, w),
                    None => extra.clear_glow(commands, k as u32),
                }
            }
            a.glow = word;
        }
    }
    if let Some(extra) = &g.extra {
        let fits = |b: &ShaderBuffer, n: usize| b.data.as_ref().is_some_and(|d| d.len() == n);
        if buffers.get(&extra.palette).is_some_and(|b| fits(b, palette.len())) {
            if let Some(mut b) = buffers.get_mut(&extra.palette) { b.data = Some(palette); }
        }
        if buffers.get(&extra.instances).is_some_and(|b| fits(b, records.len())) {
            if let Some(mut b) = buffers.get_mut(&extra.instances) { b.data = Some(records); }
        }
    }
    for (ents, model) in shown {
        for e in ents {
            if let Ok((mut tr, mut vis)) = q.get_mut(e) {
                match model {
                    Some(m) => {
                        *tr = Transform::from_matrix(m);
                        if *vis != Visibility::Inherited { *vis = Visibility::Inherited; }
                    }
                    None => if *vis != Visibility::Hidden { *vis = Visibility::Hidden; },
                }
            }
        }
    }
}

/// The number of actor draws of `g`.
pub(crate) fn gfx_len(g: &ActorGfx) -> usize { g.actors.len() }

/// Despawns every entity of `g` and empties it.
pub(crate) fn despawn_gfx(g: &mut ActorGfx, commands: &mut Commands) {
    for a in g.actors.drain(..) { for e in a.entities { commands.entity(e).despawn(); } }
    *g = ActorGfx::default();
}

/// Hides every entity of `g` (no pose this frame).
pub(crate) fn hide_gfx(g: &ActorGfx, q: &mut Query<(&mut Transform, &mut Visibility)>) {
    for a in &g.actors { for &e in &a.entities { if let Ok((_, mut v)) = q.get_mut(e) { *v = Visibility::Hidden; } } }
}

/// Class `o_class` of the level's gadget table (`crate::moby_attach::load_blobs`) with its sequences.
fn gadget_class(o_class: i32) -> Option<(LevelMobyClass, MobyAnimClass)> {
    let (_, gadgets) = crate::moby_attach::load_blobs().map_err(|e| eprintln!("travel: gadget classes: {e:#}")).ok()?;
    let g = gadgets.into_iter().find(|g| g.moby.o_class == o_class)?;
    let seqs = moby_anim::parse_sequences(&g.blob, &g.moby.class).map_err(|e| eprintln!("travel: gadget {o_class} sequences: {e}")).ok()?;
    let anim = MobyAnimClass::new(&g.moby.class, seqs);
    Some((g.moby, anim))
}

/// The actors' classes (the level's, the spaceships file's 530..537, or the level's gadget table) as extra mobys, each with its streamed sequence in
/// an extra class slot (crate::scene_render's `spawn_actors`), on render layer `layer` (None: the default layer).
/// `light`: the actors' light words (`parse_space_scene_chunk`: the hero's, else light 0 and ambient 0x38); None: the
/// level's Ratchet instance's.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_gfx(
    g: &mut ActorGfx,
    commands: &mut Commands,
    level: &mut crate::level_load::LoadedLevel,
    scene: &Scene,
    classes: Vec<i32>,
    layer: Option<RenderLayers>,
    light: Option<(u32, [u8; 3])>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> anyhow::Result<()> {
    if let Some(i) = level.mobys.instances.iter().find(|i| i.o_class == 0) { (g.light_word, g.ambient) = (i.light_word(), i.ambient_rgb()); }
    if let Some(l) = light { (g.light_word, g.ambient) = l; }
    let mut specs = Vec::new();
    let mut palette_len = 0u32;
    let Some(c0) = scene.chunks.first() else { return Ok(()) };
    for (k, a) in c0.actors.iter().enumerate() {
        let class = classes.get(k).copied().unwrap_or(a.class);
        let found = match level.mobys.classes.iter().position(|c| c.o_class == class) {
            Some(ci) => Some((level.mobys.classes[ci].clone(), level.mobys.anim[ci].clone())),
            None if (530..=537).contains(&class) => Some(load_spaceship_class(level, class)?),
            // A gadget class (a weapon demo's weapon: `select_world_object_resource_tables(item class)` loads it from the
            // level's gadget table), its sequences in its own blob.
            None => match gadget_class(class) {
                Some(g) => Some(g),
                None => {
                    warn!("travel: space actor class {class} is not on this level: not drawn");
                    None
                }
            },
        };
        let Some((cl, mut anim)) = found else {
            specs.push((None, MobyAnimClass { joint_count: 0, skeleton: vec![], rest: vec![], parent_word: vec![], sequences: vec![None] }, 0u8, palette_len, 0u32));
            continue;
        };
        let slot = cl.class.header.sequence_count;
        if anim.sequences.len() <= slot as usize { anim.sequences.resize(slot as usize + 1, None); }
        let slots = (anim.joint_count as u32).max(ExtraMobys::max_skinned_joint(&cl) as u32 + 1).max(1);
        specs.push((Some(cl), anim, slot, palette_len, slots));
        palette_len += slots;
    }
    let n = specs.len();
    let mut extra = ExtraMobys::new(level, vec![0; n.max(1) * moby_render::EXTRA_RECORD_SIZE], crate::moby_anim::identity_palette(palette_len), buffers);
    for (k, (cl, anim, slot, base, slots)) in specs.into_iter().enumerate() {
        let entities = match &cl {
            Some(c) => extra.spawn(commands, level, c, k as u32, Transform::IDENTITY, &format!("space actor {k}"), meshes, images, materials),
            None => Vec::new(),
        };
        for &e in &entities {
            commands.entity(e).insert(Visibility::Hidden);
            if let Some(l) = &layer { commands.entity(e).insert(l.clone()); }
        }
        let scale = cl.as_ref().map_or(1.0, |c| c.class.header.scale);
        g.actors.push(GfxActor { anim, slot, chunk: None, scale, base, slots, entities, glow: None });
    }
    g.extra = Some(extra);
    g.palette_len = palette_len;
    Ok(())
}

/// Hidden wins over the occlusion system's visibility writes (the hero and his items while mode 6 hides them).
#[allow(clippy::type_complexity)]
fn force_hidden(
    mut commands: Commands,
    t: Res<Travel>,
    hero: Query<(Entity, &Name), (With<MeshMaterial3d<MobyMaterial>>, Without<AttachedTo>, Without<TravelHidden>)>,
    items: Query<Entity, (With<AttachedTo>, Without<TravelHidden>)>,
    mut hidden: Query<(Entity, &mut Visibility, Has<crate::gameplay::RatchetMesh>), With<TravelHidden>>,
) {
    if t.hero_hidden {
        for (e, name) in &hero {
            if name.as_str().starts_with("Ratchet (play)") { commands.entity(e).insert((TravelHidden, Visibility::Hidden)); }
        }
        for e in &items { commands.entity(e).insert((TravelHidden, Visibility::Hidden)); }
        for (_, mut v, _) in &mut hidden { if *v != Visibility::Hidden { *v = Visibility::Hidden; } }
    } else {
        // Ratchet's own meshes go back to crate::gameplay's visibility sync; his items are shown again.
        for (e, mut v, ratchet) in &mut hidden {
            commands.entity(e).remove::<TravelHidden>();
            if !ratchet { *v = Visibility::Inherited; }
        }
    }
}

/// The main world camera (not the sky camera).
type MainCams<'w, 's> = Query<'w, 's, (Entity, &'static mut Camera, Option<&'static RenderLayers>), (With<FlyCam>, Without<crate::sky_render::SkyCamera>)>;

/// The world view replaced by the transition's ([`TRAVEL_LAYER`], cleared to black; the sky camera off), and back.
pub(crate) fn view_layers(
    mut t: ResMut<Travel>,
    mut commands: Commands,
    mut main: MainCams,
    mut sky: Query<(&mut Camera, &mut RenderLayers), With<crate::sky_render::SkyCamera>>,
) {
    let want = t.blank;
    if want == t.blanked_now { return; }
    t.blanked_now = want;
    for (e, mut cam, _) in &mut main {
        if want {
            commands.entity(e).insert(RenderLayers::layer(TRAVEL_LAYER));
            cam.clear_color = ClearColorConfig::Custom(Color::BLACK);
        } else {
            commands.entity(e).remove::<RenderLayers>();
            cam.clear_color = ClearColorConfig::None;
        }
    }
    for (mut cam, mut layers) in &mut sky {
        if want {
            *layers = RenderLayers::none();
            cam.clear_color = ClearColorConfig::Custom(Color::BLACK);
        } else {
            *layers = RenderLayers::layer(crate::sky_render::SKY_LAYER);
        }
    }
}

