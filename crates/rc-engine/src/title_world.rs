//! The title world behind the front end (G-SAV-012): the space flight the boot draws behind the logo, PRESS START and
//! the main menu (`transition_do_transition` 0x1eb798: `transition_load_wad` 0x1ea830 loads it, `fun_001eb0a8` updates
//! it, `transition_default_draw` 0x1eb410 draws it). Its data is the title lump (`rc_formats::frontend::TitleWorld`,
//! a level world in the core / gameplay formats); its scene and sounds are `rc_game::travel::title`.
//!
//! **How it is drawn.** The front end runs over a loaded level (its menus, its page tree and its Play state are the
//! level's: crate::saves, crate::menu_render). The title world is built as a second world ([`load`]: a
//! `LoadedLevel` from the lump through the level's own parsers) and drawn by the level renderers on its own render
//! layers ([`TITLE_LAYER`], [`TITLE_SKY_LAYER`]): the main camera and the sky camera show those layers while the title
//! runs (the loaded level's world stays hidden), the level renderers' per-level states (the tfrag and tie LOD states,
//! the shrub sway, the fog state, the clear colour) are the title world's until the next level change replaces them
//! (the front end always ends in one: New Game, Load Game; Quit Game returns here through one).
//!
//! | game | what | here |
//! |---|---|---|
//! | `transition_load_wad` | the GS upload, the tfrags (`fun_002040e0`), the sky, the moby / tie / shrub classes, the textures, the FX and particle banks, the chrome map TEX0, the gameplay block (`level_init_read_settings`: the fog and background colour, the lights, `light_tfrags`, the tie and shrub instances with their LightTies / LightShrubs colours, every object's occlusion word 0x7f80 = always visible), the camera chunks (`parse_space_scene_chunk(0)`), the logo; the class 0x472 sound defs (0x1861e0) | [`load`]: `TitleWorld::parse` and the level parsers (tfrags lit with the title's lights; no occlusion grid: every object always visible); [`title_audio`] |
//! | `startlevel` 0x1e9658 | the boot's sound bank (`global/sound_bank`), the level sound defs 0x15f634 = 0x186100 (7, 0x15f630) | [`title_audio`]: the audio system's data while the title runs |
//! | `transition_default_draw` | the clear (sky header +4), the sky (`update_sky_effects`: shells 0, 1, the stars, shells 2, 3), tfrags, ties, shrubs, the mobys (mode 3: the first 4 moby slots, then the page menu's frames), the list-1 callbacks, the particles; the logo, PRESS START, the fade (crate::menu_render) | [`spawn`] (tfrags, ties, shrubs and their billboards, the sky shells and the stars on [`TITLE_SKY_LAYER`]: `rc_game::sky_stars::TITLE_LEVEL`, the actors on [`TITLE_LAYER`]); the list-1 callbacks and the particles: n/a (nothing on the title registers one or spawns one: the actors' updates are `noop_callback_e`, the moby instances of the gameplay block are never loaded) |
//! | `fun_001eb0a8` / `transition_update_movie_camera` | the scene loop, the camera (tan 0.63), the actors' poses | [`tick`] (`rc_game::travel::title::TitleScene`; the camera through `ActiveScene::camera`) |
//! | 0x1eb798 scene sounds, `sound_update` in boot modes 0 / 3 / 4 | the ambience and the timed sounds of the global bank | [`tick`] (mode 3's `sound_update` is the page menu's, crate::menu_render) |
//! | 0x1eb798 attract | after the attract movie the scene restarts at tick 0 | [`tick`] (a movie's end in the title phase) |

use crate::fly_cam::FlyCam;
use crate::level_load::LoadedLevel;
use crate::sky_render::{SkyCamera, SkyMaterial};
use crate::travel_render::ActorGfx;
use anyhow::Context;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::scene::{Region, Scene};
use rc_game::travel::title::{TitleScene, ACTOR_AMBIENT};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// The title world's render layer (its tfrags, ties, shrubs and actors).
pub const TITLE_LAYER: usize = 32;
/// The title world's sky (drawn by the sky camera while the title runs).
pub const TITLE_SKY_LAYER: usize = 33;

/// The boot's level sound defs (`0x15f634` = 0x186100, `0x15f630` = 7) and class 0x472's (0x1861e0, 5 defs: the
/// menus' Confirm / Cursor / Denied / Open / PageChange).
const BOOT_LEVEL_DEFS: (u32, usize) = (0x186100, 7);
const BOOT_MENU_DEFS: (u32, usize) = (0x1861e0, 5);
/// The class the boot's menu sound defs belong to (`DAT_001b3200[DAT_001b3f32] + 0x28`).
const MENU_CLASS: i32 = 0x472;

/// The title world's data.
pub struct TitleData {
    pub level: LoadedLevel,
    pub scene: Arc<Scene>,
    pub audio: Option<rc_game::audio::LevelAudio>,
    /// The title's fog zones (gameplay section 0x80).
    pub zones: rc_formats::gameplay::FogZones,
}

/// The title lump as level data (module docs).
pub fn load(root: &std::path::Path) -> anyhow::Result<TitleData> {
    use rc_formats::{tfrag, tfrag_light, texture};
    let raw = crate::disc_source::read(root, "global/unknown_14e8.bin").context("reading the title world")?;
    let tw = rc_formats::frontend::TitleWorld::parse(&raw).context("parsing the title world")?;
    let (core, data, gs, gameplay) = (&tw.core, &tw.data[..], &tw.gs[..], &tw.gameplay[..]);
    let mut tfrags = tfrag::parse_level_tfrags(core, data).context("title tfrags")?;
    // The tfrag renderer expects strips the level data validated (its golden test): check the title's here, so a bad
    // one fails the load instead of the draw.
    for (i, t) in tfrags.iter().enumerate() {
        for list in 0..3 { tfrag::tfrag_triangles(t, list).with_context(|| format!("title tfrag {i} strip list {list}"))?; }
    }
    // level_init_read_settings: `light_tfrags` with the title's directional lights (`RC_NO_LIGHT=1` keeps the stored RGBA).
    if !std::env::var("RC_NO_LIGHT").is_ok_and(|v| v != "0") {
        let elf = crate::disc_source::read_path(root, &root.join("boot/SCUS_971.99"))?;
        let bank = tfrag_light::parse_light_bank(gameplay).context("title lights")?;
        let table = tfrag_light::NormalTable::from_elf(&elf, tfrag_light::BOOT_NORMAL_TABLE_VADDR).context("the normal table")?;
        for t in tfrags.iter_mut() { t.rgba = tfrag_light::light_tfrag(t, &bank, &table, None); }
    }
    let textures = texture::parse_textures(core, data, gs).context("title textures")?;
    let tfrag_lod = crate::tfrag_lod::load(core, data, gs)?;
    let fog = crate::game_camera::LevelFog::parse(gameplay).context("title level settings")?;
    let background = crate::game_camera::level_background(gameplay).context("title level settings")?;
    let mobys = crate::moby_render::load_mobys_with_gs(root, core, data, gameplay, gs).context("title mobys")?;
    let ties = crate::tie_render::load_ties(core, data, gs, gameplay, &textures).context("title ties")?;
    let shrubs = crate::shrub_render::load_shrubs(core, data, gs, gameplay, &textures).context("title shrubs")?;
    let sky = crate::sky_render::load(core, data, rc_game::sky_stars::TITLE_LEVEL).context("title sky")?;
    let occlusion = crate::occlusion::load(core, data, gameplay, &tfrags, &ties.instances, &mobys.instances).context("title occlusion words")?;
    let particles = crate::particle_render::LevelParticles {
        textures: rc_formats::particle_tex::parse_particle_textures(core, &tw.index, data).map_err(|e| eprintln!("title world: particle textures: {e}")).ok(),
        owners: Vec::new(),
        level: u32::MAX,
        grid: None,
    };
    let zones = rc_formats::gameplay::parse_fog_zones(gameplay).unwrap_or_default();
    let scene = Scene::from_lump(&tw.scene, 0, Region::Ntsc).context("title scene chunks")?;
    let moby_class_order = core.moby_classes.iter().map(|e| e.o_class).collect();
    let level = LoadedLevel {
        tfrags,
        textures,
        fog,
        background,
        mobys,
        ties,
        shrubs,
        sky,
        occlusion,
        tfrag_lod,
        particles,
        hud: None,
        water: Default::default(),
        sea: Default::default(),
        collision: None,
        death_z: 0.0,
        audio: None,
        gameplay: gameplay.to_vec(),
        moby_class_order,
        timings: Default::default(),
    };
    Ok(TitleData { level, scene: Arc::new(scene), audio: title_audio(root), zones })
}

/// A boot sound def (0x20 bytes, `SoundDef`; its +0x1a is already the bank sound id).
fn sound_def(b: &[u8]) -> rc_formats::sound_bank::SoundDef {
    let f = |o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let i = |o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    rc_formats::sound_bank::SoundDef {
        near: f(0),
        far: f(4),
        vol_far: i(8),
        vol_near: i(0xc),
        pb_lo: i(0x10),
        pb_hi: i(0x14),
        looped: b[0x18],
        flags: b[0x19],
        index: u16::from_le_bytes([b[0x1a], b[0x1b]]),
        bank_handle: 0,
    }
}

/// The boot's sounds (`startlevel`): the global sound bank, the level defs 0x186100 and class 0x472's 0x1861e0 (module
/// docs). None without the data (the title is then silent).
fn title_audio(root: &std::path::Path) -> Option<rc_game::audio::LevelAudio> {
    let bank = crate::disc_source::read(root, "global/sound_bank.bin").ok()?;
    let bank = rc_formats::sound_bank::parse_bank(&bank).map_err(|e| eprintln!("title world: sound bank: {e}")).ok()?;
    let elf = crate::disc_source::read_path(root, &root.join("boot/SCUS_971.99")).ok()?;
    let defs = |(at, n): (u32, usize)| -> Option<Vec<rc_formats::sound_bank::SoundDef>> {
        let b = rc_formats::tfrag_light::elf_read(&elf, at, n * 0x20).ok()?;
        Some(b.as_chunks::<0x20>().0.iter().map(|c| sound_def(c)).collect())
    };
    let level_defs = defs(BOOT_LEVEL_DEFS)?;
    let menu = defs(BOOT_MENU_DEFS)?;
    let sounds = rc_formats::sound_bank::LevelSounds {
        level_defs,
        map: Vec::new(),
        classes: vec![rc_formats::sound_bank::ClassSounds { o_class: MENU_CLASS, bank_ids: menu.iter().map(|d| d.index).collect(), defs: menu, header_count: Some(BOOT_MENU_DEFS.1 as u8) }],
    };
    Some(rc_game::audio::LevelAudio {
        bank: Arc::new(bank),
        sounds,
        instances: Vec::new(),
        instance_pvars: Vec::new(),
        music: Default::default(),
        env_points: Vec::new(),
        collision: None,
    })
}

/// The title world's runtime state.
#[derive(Resource, Default)]
pub struct TitleWorldRt {
    load: Option<Mutex<Option<JoinHandle<anyhow::Result<TitleData>>>>>,
    data: Option<Box<TitleData>>,
    failed: bool,
    /// The world's entities exist (until the next level change).
    spawned: bool,
    scene: Option<TitleScene>,
    gfx: ActorGfx,
    poses: Vec<rc_game::travel::space::ActorPose>,
    frames: u64,
    uploaded: Option<u64>,
    /// The cameras show the title world.
    shown: bool,
    /// A movie played last tick (the attract movie's end restarts the scene).
    movie_was: bool,
}

pub struct TitleWorldPlugin;

impl Plugin for TitleWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TitleWorldRt>()
            .add_systems(crate::level_switch::LevelUnload, unload)
            .add_systems(Update, spawn)
            .add_systems(FixedUpdate, tick.after(crate::menu_render::MenuFrameSet))
            .add_systems(
                PostUpdate,
                cameras.after(crate::travel_render::view_layers).after(crate::sky_render::follow_main_camera).before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(PostUpdate, upload);
    }
}

/// A level change: the title world's entities and states are gone (the data stays loaded for the next front end); the
/// cameras back on the level's view if the title still had them.
fn unload(
    mut commands: Commands,
    mut rt: ResMut<TitleWorldRt>,
    main: Query<Entity, (With<FlyCam>, Without<SkyCamera>)>,
    mut sky: Query<&mut RenderLayers, With<SkyCamera>>,
) {
    if rt.shown {
        for e in &main { commands.entity(e).remove::<RenderLayers>(); }
        for mut layers in &mut sky { *layers = RenderLayers::layer(crate::sky_render::SKY_LAYER); }
    }
    let data = rt.data.take();
    let failed = rt.failed;
    *rt = TitleWorldRt { data, failed, ..Default::default() };
    crate::saves::set_title_world_shown(false);
}

/// The front end's title phase runs (the logo, PRESS START, the main menu, the card dialog).
fn title_phase(fer: &crate::saves::FrontEndRt) -> Option<&rc_game::frontend::FrontEnd> {
    fer.fe.as_ref().filter(|f| f.phase == rc_game::frontend::Phase::Title)
}

/// The material stores of the world renderers the title world spawns into.
type WorldMaterials<'w> = (
    ResMut<'w, Assets<crate::tfrag_render::TfragMaterial>>,
    ResMut<'w, Assets<crate::tie_render::TieMaterial>>,
    ResMut<'w, Assets<crate::shrub_render::ShrubMaterial>>,
    ResMut<'w, Assets<crate::shrub_billboard::BillboardMaterial>>,
    ResMut<'w, Assets<SkyMaterial>>,
    ResMut<'w, Assets<crate::moby_render::MobyMaterial>>,
    ResMut<'w, Assets<crate::sky_stars::SkyStarMaterial>>,
);

/// The main world camera (not the sky camera).
type MainCam<'w, 's> = Query<'w, 's, (Entity, &'static mut Camera), (With<FlyCam>, Without<SkyCamera>)>;

/// Loads the title world on a worker thread once the front end runs (the boot's still covers it), then spawns its
/// draws when the title phase starts (module docs).
#[allow(clippy::too_many_arguments)]
fn spawn(
    mut commands: Commands,
    mut rt: ResMut<TitleWorldRt>,
    fer: Option<Res<crate::saves::FrontEndRt>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    (mut tfrag_mats, mut tie_mats, mut shrub_mats, mut bb_mats, mut sky_mats, mut moby_mats, mut star_mats): WorldMaterials,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
) {
    let rt = &mut *rt;
    let Some(fer) = fer else { return };
    if fer.fe.is_none() || rt.failed { return; }
    if rt.data.is_none() {
        if rt.load.is_none() {
            let root = crate::level_load::extracted_root();
            println!("title world: loading the title lump (global/unknown_14e8) in the background");
            rt.load = Some(Mutex::new(std::thread::Builder::new().name("title world load".into()).spawn(move || load(&root)).ok()));
        }
        let done = rt.load.as_ref().and_then(|m| {
            let mut g = m.lock().unwrap_or_else(|e| e.into_inner());
            if g.as_ref().is_some_and(|h| h.is_finished()) { g.take() } else { None }
        });
        let Some(h) = done else { return };
        rt.load = None;
        match h.join().unwrap_or_else(|_| Err(anyhow::anyhow!("the title world loader panicked"))) {
            Ok(d) => {
                println!(
                    "title world: {} tfrags, {} ties, {} shrubs, {} moby classes, sky {}, scene {} chunks ({} ticks, {} actors)",
                    d.level.tfrags.len(), d.level.ties.instances.len(), d.level.shrubs.instances.len(), d.level.mobys.classes.len(),
                    d.level.sky.as_ref().map_or(0, |s| s.sky.shells.len()), d.scene.chunks.len(), d.scene.end_tick(), d.scene.actor_classes().len()
                );
                rt.data = Some(Box::new(d));
            }
            Err(e) => {
                eprintln!("title world: not loaded ({e:#}): the front end stays over black");
                rt.failed = true;
                return;
            }
        }
    }
    if rt.spawned || title_phase(&fer).is_none() { return; }
    let Some(d) = rt.data.as_deref_mut() else { return };
    rt.spawned = true;
    let lv = &mut d.level;
    let world = RenderLayers::layer(TITLE_LAYER);
    let t = crate::tfrag_render::spawn_tfrags_on(&mut commands, lv, &mut meshes, &mut images, &mut tfrag_mats, &mut buffers, Some(&world));
    let ties = crate::tie_render::spawn_ties_layered(&mut commands, lv, &mut meshes, &mut images, &mut tie_mats, &mut buffers, &world);
    let shrubs = crate::shrub_render::spawn_shrubs_layered(&mut commands, lv, &mut meshes, &mut images, &mut shrub_mats, &mut buffers, &world);
    crate::shrub_billboard::spawn_billboards(&mut commands, lv, &mut meshes, &mut images, &mut bb_mats, &mut buffers, Some(&world));
    // update_sky_effects: shells 0 and 1, the stars, shells 2 and 3, unturned.
    let sky = lv.sky.as_ref().map(|ls| crate::sky_render::spawn_shells(&mut commands, ls, Some(2), RenderLayers::layer(TITLE_SKY_LAYER), &mut meshes, &mut images, &mut sky_mats, |e, _| { e.insert(Transform::IDENTITY); }));
    // The stars of `update_sky_effects` (`rc_game::sky_stars::TITLE_LEVEL`) between shells 1 and 2: the title's star
    // simulation replaces the level's until the next level change.
    if let (Some(ls), Some(slot)) = (lv.sky.as_ref(), sky.as_ref().and_then(|s| s.star_order)) {
        if let Some(sim) = crate::sky_stars::spawn_stars(&mut commands, ls, slot, RenderLayers::layer(TITLE_SKY_LAYER), &mut meshes, &mut images, &mut star_mats, &mut buffers) {
            commands.insert_resource(sim);
        }
    }
    // The actors (`parse_space_scene_chunk`: no hero on the title, the light words 0x38383800000000).
    let classes = d.scene.actor_classes();
    if let Err(e) = crate::travel_render::spawn_gfx(&mut rt.gfx, &mut commands, lv, &d.scene, classes, Some(world.clone()), Some((0, ACTOR_AMBIENT)), &mut meshes, &mut images, &mut moby_mats, &mut buffers) {
        warn!("title world: actors not drawn: {e:#}");
    }
    rt.scene = Some(TitleScene::new(d.scene.clone()));
    // level_init_read_settings: the fog and the background colour are the title's (until the next level change).
    commands.insert_resource(crate::fog_state::FogState::new(crate::fog_state::FogLevelData { zones: d.zones.clone(), mesh: None, lights: Default::default() }, &lv.fog));
    let bg = lv.background;
    commands.insert_resource(ClearColor(Color::srgb_u8(bg[0], bg[1], bg[2])));
    // The boot's sound bank and defs (the level's sounds were stopped for the front end).
    if let (Some(a), Some(data)) = (audio.as_deref_mut(), d.audio.clone()) { a.swap_level(data); }
    println!(
        "title world: drawn ({} tfrag meshes, {} tie entities, {} shrub entities, {} sky draws, {} actors)",
        t.meshes, ties, shrubs, sky.map_or(0, |s| s.entities.len()), rt.gfx_len()
    );
}

impl TitleWorldRt {
    fn gfx_len(&self) -> usize { crate::travel_render::gfx_len(&self.gfx) }
}

/// One vsync of the title (module docs): the scene sounds, the scene's tick, the camera, the actors' poses, the
/// `sound_update` of boot modes 0 and 4.
#[allow(clippy::too_many_arguments)]
fn tick(
    mut rt: ResMut<TitleWorldRt>,
    fer: Option<Res<crate::saves::FrontEndRt>>,
    play: Option<ResMut<crate::gameplay::Play>>,
    mut active: ResMut<crate::scene_render::ActiveScene>,
    movies: Option<Res<crate::movie_render::MovieState>>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
) {
    let rt = &mut *rt;
    let Some(fer) = fer else { return };
    let Some(fe) = title_phase(&fer) else { return };
    if !rt.spawned { return; }
    let busy = movies.as_deref().is_some_and(|m| m.busy());
    // The attract movie's end: the scene from tick 0 again.
    if rt.movie_was && !busy {
        if let Some(s) = rt.scene.as_mut() { s.restart(); }
    }
    rt.movie_was = busy;
    if busy { return; }
    let Some(scene) = rt.scene.as_mut() else { return };
    let mut play = play;
    // The scene sounds (before the update, with the tick it left).
    if let (Some(a), Some(p)) = (audio.as_deref_mut(), play.as_deref_mut()) {
        let listener = active.camera.map(|c| rc_game::audio::voices::Listener { pos: c.eye, rows: c.rows, underwater: false, water_height: 0.0 }).unwrap_or_default();
        scene.sounds(a.system(), &listener, &mut p.game.rng);
    }
    let out = scene.tick();
    if let Some(c) = out.camera { active.camera = Some(c); }
    rt.poses = out
        .actors
        .iter()
        .map(|a| rc_game::travel::space::ActorPose {
            actor: a.actor,
            class: a.class,
            chunk: a.chunk,
            frame_a: a.frame_a,
            frame_b: a.frame_b,
            t: a.t,
            position: a.position,
            yaw: 0.0,
            moby: None,
            scale: None,
            hidden: !TitleScene::actors_drawn(fe.mode, a.actor),
            glow: None,
            head: None,
        })
        .collect();
    rt.frames += 1;
    // sound_update in boot modes 0 and 4 (mode 3: the page menu's frame runs it, crate::menu_render).
    if matches!(fe.mode, 0 | 4) {
        if let (Some(a), Some(p)) = (audio.as_deref_mut(), play.as_deref_mut()) {
            let listener = active.camera.map(|c| rc_game::audio::voices::Listener { pos: c.eye, rows: c.rows, underwater: false, water_height: 0.0 }).unwrap_or_default();
            let input = rc_game::audio::FrameInput { listener, hero_pos: listener.pos };
            let game = &mut p.game;
            let table = &game.mobys;
            let (sys, buf) = a.parts();
            buf.clear();
            sys.sound_update_with(&input, game.counter as u32, &mut game.rng, &|id| rc_game::audio::class_sounds::owner_position(table, id));
            sys.render(rc_game::audio::SAMPLES_PER_FRAME, buf);
            a.push_frame();
        }
    }
}

/// The cameras on the title world's layers while it is shown (module docs); given back to the transition's black
/// screen (crate::travel_render's `view_layers`) or the level's view when it ends.
#[allow(clippy::too_many_arguments)]
fn cameras(
    mut commands: Commands,
    mut rt: ResMut<TitleWorldRt>,
    fer: Option<Res<crate::saves::FrontEndRt>>,
    travel: Res<crate::travel_render::Travel>,
    mut active: ResMut<crate::scene_render::ActiveScene>,
    clear: Res<ClearColor>,
    mut main: MainCam,
    mut sky: Query<(&mut Camera, &mut RenderLayers), With<SkyCamera>>,
) {
    let front = fer.as_deref().is_some_and(|f| f.fe.is_some());
    let want = rt.spawned && !rt.failed && !travel.blanked() && (front || rt.shown);
    if want {
        for (e, mut cam) in &mut main {
            commands.entity(e).insert(RenderLayers::layer(TITLE_LAYER));
            if !matches!(cam.clear_color, ClearColorConfig::None) { cam.clear_color = ClearColorConfig::None; }
        }
        for (mut cam, mut layers) in &mut sky {
            if *layers != RenderLayers::layer(TITLE_SKY_LAYER) { *layers = RenderLayers::layer(TITLE_SKY_LAYER); }
            // The loader's sky header +4 = 1: the frame cleared to the title's background (the gouraud dome covers it).
            if !matches!(cam.clear_color, ClearColorConfig::Custom(k) if k == clear.0) { cam.clear_color = ClearColorConfig::Custom(clear.0); }
        }
        if !rt.shown {
            rt.shown = true;
            crate::saves::set_title_world_shown(true);
            println!("title world: shown behind the front end");
        }
    } else if rt.shown {
        rt.shown = false;
        crate::saves::set_title_world_shown(false);
        active.camera = None;
        // The transition's view_layers owns the cameras from its black screen on; else back to the level's view.
        if !travel.blanked() {
            for (e, _) in &mut main { commands.entity(e).remove::<RenderLayers>(); }
            for (_, mut layers) in &mut sky { *layers = RenderLayers::layer(crate::sky_render::SKY_LAYER); }
        }
        println!("title world: hidden");
    }
}

/// The actors' poses into their draws.
fn upload(mut commands: Commands, mut rt: ResMut<TitleWorldRt>, mut buffers: ResMut<Assets<ShaderBuffer>>, mut q: Query<(&mut Transform, &mut Visibility)>) {
    let rt = &mut *rt;
    if !rt.spawned || rt.uploaded == Some(rt.frames) { return; }
    rt.uploaded = Some(rt.frames);
    let Some(d) = rt.data.as_deref() else { return };
    if rt.poses.is_empty() {
        crate::travel_render::hide_gfx(&rt.gfx, &mut q);
        return;
    }
    crate::travel_render::upload_gfx(&mut rt.gfx, &d.level, &d.scene, &rt.poses, None, &mut commands, &mut buffers, &mut q);
}
