//! Runtime entry point: loads one level from the game data folder and runs it.
//!
//! Arguments (launcher contract, docs/plan/launcher_contract.md; crate::disc_source):
//! - `--data-dir <dir>` the game data folder (`rerac-extract` output); `--version-json` print the version and exit
//!
//! Environment:
//! - `RC_DATA_DIR`   the game data folder when `--data-dir` is absent; else `RC_EXTRACTED`, else `<workspace>/extracted`
//! - `RC_LEVEL`      start straight in this level (skips the front end unless `RC_FRONTEND=1`; without it the front
//!   end's New Game / Load decides)
//! - `RC_SCREENSHOT` if set, save a PNG of the window there after the scene has rendered, then exit
//! - `RC_SCREENSHOT_DELAY` seconds to wait before that capture (default 3); with `RC_SCREENSHOT_FRAME`
//!   the capture is frame-exact instead (crate::determinism) and this wall-clock one is not scheduled
//! - `RC_CAM`        optional camera override in game coordinates: `eye_x,eye_y,eye_z,target_x,target_y,target_z`
//! - `RC_GS_ALPHA=0` the previous alpha mapping (one blended draw without Z for any As != 0x80), crate::gs_state
//! - `RC_FOG_ZONES=0`, `RC_UNDERWATER=1`, `RC_CAM_PATH` fog zones / underwater / camera walk-through, crate::fog_state
//! - `RC_PLAY=0`, `RC_PLAY_SCRIPT`, `RC_PLAY_TRACE=1` the game tick / scripted pad / per-tick trace, crate::gameplay
//! - `RC_PLAY_FLY=1` start on the fly camera with the game ticking; `RC_DEBUG_HIT=moby@tick,...` debug hits
//!   (flags 0x10000, damage 1) before those ticks' moby loop, crate::gameplay
//! - `RC_FRONTEND`   the front end (card check, logos, title, main menu) is the default start; `0` skips it, `1` keeps it
//!   with `RC_LEVEL`, crate::saves; `RC_SAVE_DIR=<dir>` the
//!   memory-card folder (`0`: no card), `RC_SAVE_TRACE=1` the card's states
//! - `RC_SCENE=0` / `RC_SCENE=<k>`, `RC_SUBTITLES=0` in-engine scenes (Novalis arrival = scene 5), crate::scene_render
//! - `RC_UNLOCK_PLANETS=<p>,…` planets unlocked from the start (the ship's planet page), crate::gameplay;
//!   `RC_TRAVEL_TRACE=1` the ship / mode-6 / transition trace, crate::travel_render
//! - `RC_HUD=0`, `RC_HUD_DEMO=1`, `RC_HUD_TEXT`, `RC_HUD_HELP=<id>`, `RC_LANG` the HUD and its demos, crate::hud_render
//! - `RC_MSAA=0|2|4|8` world-camera multisampling at start (default 0 = off, like the GS), crate::render_settings
//! - `RC_SETTINGS_FILE=<path>` the port-settings file (`0` or empty: none), crate::render_settings;
//!   `RC_SETTINGS_PAGE=0` no "Port Options" page in the Options menu, crate::menu_render
//! - `RC_SHADOWS=0|1`, `RC_SHADOW_DEBUG=1`, `RC_SHADOW_TRACE=1` the moby shadows, crate::shadow_render

mod crash_log;
mod audio_out;
mod determinism;
mod display;
mod display_blend;
mod disc_source;
mod fly_cam;
mod fx_draw;
mod fog_state;
mod screen_tint;
mod game_camera;
mod gameplay;
mod gs_state;
mod hud_render;
mod hud_images;
mod input_map;
mod interact_render;
mod level_load;
mod level_switch;
mod media_render;
mod frame_pace;
mod menu_models;
mod menu_render;
mod mirror_render;
mod moby_anim;
mod moby_attach;
mod moby_light;
mod moby_lod;
mod moby_render;
mod moby_spawn;
mod movie_render;
mod occlusion;
mod particle_render;
mod play_camera;
mod render_settings;
mod reticle_render;
mod marker_render;
mod tesla_render;
mod walloper_render;
mod visibomb_view;
mod gs_post;
mod reactive_render;
mod saves;
mod scene_render;
mod sea_render;
mod screen_canvas;
mod shadow_render;
mod pre_shadow;
mod frame_log;
mod shrub_billboard;
mod shrub_light;
mod shrub_render;
mod sky_render;
mod sky_stars;
mod text_render;
mod tfrag_light;
mod tfrag_lod;
mod tfrag_render;
mod tie_light;
mod tie_lod;
mod tie_render;
mod travel_render;
mod flight_render;
mod title_world;
mod vendor_render;
mod water_render;
mod afterimage_render;
mod thruster_render;
mod world_lights;

use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::path::PathBuf;
use std::time::Duration;

use fly_cam::{FlyCam, FlyCamPlugin};
use level_load::LoadedLevel;
use tfrag_render::{game_to_bevy, TfragMaterial};

#[derive(Resource)]
struct Level(LoadedLevel);

#[derive(Resource)]
struct ScreenshotRequest {
    path: PathBuf,
    /// Minimum wall time and frame count before capturing, so the shader has compiled and the meshes have been uploaded.
    delay: Duration,
    min_frames: u32,
}

fn main() -> anyhow::Result<()> {
    crash_log::install();
    // `--version-json` exits here; a missing or wrong data folder exits with a clear error (crate::disc_source).
    let root = disc_source::startup();
    let index = level_load::level_index();
    // The one level load (the boot's and every runtime level change's, crate::level_switch): the level data, the loader's
    // ship and the load-time update pass (crate::moby_spawn; RC_SPAWN_RULES=0 skips it), the fog zones and the
    // underwater test (crate::fog_state).
    // The boot's game state first: the level's spawn test reads its save bits (crate::gameplay::level_spawn_save).
    let boot = gameplay::boot_state(&root, index);
    let save = gameplay::level_spawn_save(boot.as_ref().map(|(gs, _)| gs), index);
    let level_switch::LevelBundle { mut level, spawn, fog: fog_state, .. } = level_switch::load_bundle(&root, index, &save)?;
    let t = &level.timings;
    let bg = level.background;
    println!(
        "level {index:02} from {}: {} tfrags, {} textures; load {:.1} ms (read {:.1}, WAD {:.1}, core {:.1}, tfrags {:.1}, textures {:.1})",
        root.display(), level.tfrags.len(), level.textures.len(), ms(t.total()),
        ms(t.read), ms(t.decompress), ms(t.parse_core), ms(t.parse_tfrags), ms(t.parse_textures)
    );
    println!("background (GS clear colour): {bg:?}");
    println!("{}", fly_cam::CONTROLS);

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                // 4:3, as the TV showed the 512x416 GS draw buffer (crate::display; docs/plan/render_pipeline.md).
                primary_window: Some(Window { title: "ReRAC".into(), resolution: (1024u32, 768u32).into(), ..default() }),
                ..default()
            })
            .set(AssetPlugin { file_path: asset_dir(), ..default() }),
    )
    // The game frame every camera renders into and its presentation in the window (before anything spawns a camera,
    // and before the capture, which captures the frame).
    .add_plugins(display::DisplayPlugin)
    // Frame-exact ticks / capture and the deterministic phase order, before anything spawns a camera.
    .add_plugins(determinism::DeterminismPlugin)
    // One game tick per drawn frame, at most 60 frames a second (crate::frame_pace).
    .add_plugins(frame_pace::FramePacePlugin)
    // MSAA of the world cameras (default off; `RenderSettings` can change it at run time).
    .add_plugins(render_settings::RenderSettingsPlugin)
    .add_plugins((MaterialPlugin::<TfragMaterial>::default(), FlyCamPlugin, game_camera::GameCameraPlugin { fog: level.fog }))
    .add_plugins(occlusion::OcclusionPlugin)
    .add_plugins(tfrag_lod::TfragLodPlugin)
    .add_plugins(moby_render::MobyRenderPlugin)
    .add_plugins(moby_spawn::MobySpawnPlugin)
    .add_plugins(moby_attach::MobyAttachPlugin)
    // The moby shadows: the game's shadow rules and a native shadow-volume pass (crate::shadow_render).
    .add_plugins(shadow_render::ShadowPlugin)
    .add_plugins(pre_shadow::PreShadowPlugin)
    .add_plugins(frame_log::FrameLogPlugin)
    .insert_resource(spawn)
    .add_plugins(tie_render::TieRenderPlugin)
    .add_plugins(shrub_render::ShrubRenderPlugin)
    // Point lights on tfrags, ties and shrubs (the game's Light* point passes; crate::world_lights).
    .add_plugins(world_lights::WorldLightsPlugin)
    .add_plugins(sky_render::SkyRenderPlugin)
    .add_plugins(display_blend::DisplayBlendPlugin)
    .add_plugins(particle_render::ParticlePlugin)
    .add_plugins(water_render::WaterPlugin)
    .add_plugins(sea_render::SeaRenderPlugin)
    .add_plugins(fx_draw::FxDrawPlugin)
    .add_plugins(afterimage_render::AfterImagePlugin)
    .add_plugins(reticle_render::ReticlePlugin)
    .add_plugins(tesla_render::TeslaPlugin)
    .add_plugins(walloper_render::WalloperPlugin)
    .add_plugins(reactive_render::ReactivePlugin)
    .add_plugins(sky_stars::SkyStarsPlugin)
    .add_plugins(fog_state::FogStatePlugin)
    .add_plugins(screen_tint::ScreenTintPlugin)
    .add_plugins(visibomb_view::VisibombViewPlugin)
    .add_plugins(hud_render::HudPlugin)
    // The game tick (hero, pad, follow camera) driving Ratchet and the view; RC_PLAY=0 keeps the fly camera only.
    .add_plugins(gameplay::GameplayPlugin { boot: std::sync::Mutex::new(boot) })
    .add_plugins(menu_render::MenuPlugin)
    .add_plugins((screen_canvas::CanvasPlugin, vendor_render::VendorRenderPlugin))
    // The Gadgets / Weapons pages' 3D Ratchet and item preview (crate::menu_models, on crate::screen_canvas).
    .add_plugins(menu_models::MenuModelsPlugin)
    // In-engine scenes (mode 2) and the Novalis arrival (crate::scene_render; RC_SCENE=0 / RC_SCENE=<k>).
    .add_plugins(scene_render::SceneRenderPlugin)
    // PSS movies (mode 1) played from the original files (crate::movie_render; RC_PLAY_MOVIE=<n>).
    .add_plugins(movie_render::MovieRenderPlugin)
    .add_plugins(mirror_render::MirrorRenderPlugin)
    // Sound slots, 989snd, music and the software SPU2 mixed per game tick (crate::audio_out; RC_AUDIO=0 off).
    .add_plugins(audio_out::AudioOutPlugin::new(level.audio.take()))
    .insert_resource(fog_state)
    // The GS frame clear: level settings +0x00..+0x08 via `set_background_color` (game_camera::level_background).
    // srgb_u8: the bytes are display-encoded like every GS colour; the sRGB view target stores them back exactly.
    .insert_resource(ClearColor(Color::srgb_u8(bg[0], bg[1], bg[2])))
    .insert_resource(Level(level))
    // The runtime level change and the level start's schedules (crate::level_switch), planet travel (crate::travel_render).
    .add_plugins(level_switch::LevelSwitchPlugin)
    .add_plugins(travel_render::TravelPlugin)
    .add_plugins(flight_render::FlightRenderPlugin)
    .add_plugins(title_world::TitleWorldPlugin)
    .add_systems(Startup, setup_camera)
    .add_systems(level_switch::LevelStartup, setup)
    .add_systems(Update, report_fps);

    if let Some(path) = std::env::var_os("RC_SCREENSHOT").filter(|_| determinism::screenshot_frame().is_none()) {
        let secs = std::env::var("RC_SCREENSHOT_DELAY").ok().and_then(|s| s.parse().ok()).unwrap_or(3.0);
        app.insert_resource(ScreenshotRequest { path: path.into(), delay: Duration::from_secs_f32(secs), min_frames: 30 })
            .add_systems(Update, take_screenshot);
    }
    app.run();
    Ok(())
}

fn ms(d: Duration) -> f64 { d.as_secs_f64() * 1e3 }

/// The shader folder: `assets/` next to the executable (a packaged version, `tools/package`), or
/// `../Resources/assets` inside a macOS `.app`; otherwise the repo's `crates/rc-engine/assets` (`cargo dev`,
/// `target/*/rerac`). The executable path is resolved through symlinks first, so a symlinked binary finds the
/// folder beside its real file.
fn asset_dir() -> String {
    let exe = std::env::current_exe().and_then(std::fs::canonicalize).ok();
    let dir = exe.as_deref().and_then(std::path::Path::parent);
    [dir.map(|d| d.join("assets")), dir.map(|d| d.join("../Resources/assets"))]
        .into_iter()
        .flatten()
        .find(|p| p.join("shaders").is_dir())
        .map_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(), |p| p.to_string_lossy().into_owned())
}

fn setup(
    mut commands: Commands,
    level: Res<Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TfragMaterial>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    let level = &level.0;
    let s = tfrag_render::spawn_tfrags(&mut commands, level, &mut meshes, &mut images, &mut materials, &mut buffers);
    println!(
        "tfrags: {} tfrags, {} LOD-0 triangles, {} vertices, {} meshes ({} with the GS alpha-test split), {} images; mesh build {:.1} ms, asset upload {:.1} ms; vertex AABB {:.1?} .. {:.1?}",
        s.tfrags, s.triangles, s.vertices, s.meshes, s.blended_meshes, s.images, ms(s.mesh_build), ms(s.upload), s.min, s.max
    );
}

/// The main world camera, once per process (crate::level_switch keeps it across level changes); framed on the boot
/// level's tfrags (the play camera takes over from the first tick).
fn setup_camera(mut commands: Commands, level: Res<Level>) {
    let level = &level.0;

    // Camera framing from the tfrag bounding spheres. They are stored in integer position units
    // (x1024 like origin + local), not world units: on Novalis the raw centroid is ~(166138, 189596, 59301)
    // while the LOD-0 vertices span (22..322, 45..388, 30..112) after /1024.
    let spheres: Vec<[f32; 4]> = level.tfrags.iter().map(|t| t.header.bsphere.map(|v| v / 1024.0)).collect();
    let n = spheres.len().max(1) as f32;
    let c = spheres.iter().fold(Vec3::ZERO, |a, s| a + Vec3::new(s[0], s[1], s[2])) / n;
    let extent = spheres.iter().map(|s| (Vec3::new(s[0], s[1], s[2]) - c).length() + s[3]).fold(0.0f32, f32::max);
    println!("bounds (game units): sphere centroid {:.1?}, extent {:.1}", c.to_array(), extent);
    let (eye, target) = match std::env::var("RC_CAM").ok().and_then(|v| parse_cam(&v)) {
        Some((e, t)) => (game_to_bevy(e), game_to_bevy(t)),
        // Up (game +Z) and back (game -Y) from the centroid, looking at it.
        None => (game_to_bevy((c + Vec3::new(0.0, -0.35 * extent, 0.25 * extent)).to_array()), game_to_bevy(c.to_array())),
    };
    let transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);
    commands.spawn((
        Camera3d::default(),
        game_camera::game_projection(),
        // No tonemapping or dithering: the shader already produces the final display value.
        Tonemapping::None,
        DebandDither::Disabled,
        render_settings::WorldCamera,
        // The UI nodes without a target camera draw on the game frame (not on the present camera).
        bevy::ui::IsDefaultUiCamera,
        FlyCam::from_transform(&transform, (extent / 20.0).clamp(5.0, 200.0)),
        transform,
        level_switch::KeepAcrossLevels,
    ));
}

/// `ex,ey,ez,tx,ty,tz` in game coordinates.
fn parse_cam(s: &str) -> Option<([f32; 3], [f32; 3])> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse().ok()).collect::<Option<_>>()?;
    (v.len() == 6).then(|| ([v[0], v[1], v[2]], [v[3], v[4], v[5]]))
}

/// Drawn frames per second over 5 s of wall-clock time (the game clock advances by whole ticks: crate::frame_pace).
fn report_fps(mut acc: Local<Option<(std::time::Instant, u32)>>) {
    let now = std::time::Instant::now();
    let (start, n) = acc.get_or_insert((now, 0));
    *n += 1;
    let t = (now - *start).as_secs_f32();
    if t >= 5.0 {
        println!("fps: {:.1}", *n as f32 / t);
        *acc = Some((now, 0));
    }
}

fn take_screenshot(
    mut commands: Commands,
    req: Res<ScreenshotRequest>,
    frame: Res<display::GameFrame>,
    time: Res<Time<Real>>,
    mut frames: Local<u32>,
    mut requested: Local<bool>,
) {
    *frames += 1;
    if *requested || *frames < req.min_frames || time.elapsed() < req.delay { return; }
    *requested = true;
    let path = req.path.clone();
    // The game frame (crate::display), not the window: no bars, and no blank capture of an occluded window.
    commands.spawn(Screenshot::image(frame.image.clone())).observe(move |shot: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
        match shot.image.clone().try_into_dynamic() {
            Ok(img) => match img.to_rgb8().save(&path) {
                Ok(()) => println!("screenshot saved to {}", path.display()),
                Err(e) => eprintln!("cannot save screenshot to {}: {e}", path.display()),
            },
            Err(e) => eprintln!("cannot convert screenshot: {e:?}"),
        }
        exit.write(AppExit::Success);
    });
}
