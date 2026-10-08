//! Per-frame fog: fog zones, the underwater state and the fog every renderer reads
//! (docs/plan/world_animation.md §5, §6; the port notes are in its "In the port" sections).
//!
//! Each frame, in `PostUpdate` before the occlusion/LOD systems ([`FogSet`]):
//! 1. **`UpdateFog`** (level01 0x218d70, run by the game at the end of the previous frame's render): the
//!    view context gets the level fog globals, or the alternate fog while the underwater flag is set, and
//!    the particle far (500 / 64 units). Written to [`GameFog`]; consumers: every material's fog uniform
//!    (`push_fog`), the tie per-instance F (`TieLodState::fog`), the tfrag LOD constants
//!    (`tfrag_lod::update_lod_constants`, `SetTfragDists`) and the particle cull (`particle_render`).
//!    Because the game copies at the end of the render, the drawn fog lags the camera by one update; the
//!    order here keeps that lag (inferred from the call order, see the doc).
//! 2. **Camera update** (0x20eca8) for the camera (the fly camera for now): the underwater test 0x20e9f0
//!    (`rc_game::fog_zones::UnderwaterState`, collision line queries against the level mesh and, with the game tick,
//!    the moby meshes (the patch managers' water); the water height
//!    is the level's water, `rc_game::water::world::WaterWorld::water_height`: the active ripple patch, the flat
//!    plane, else the hit point), then the fog
//!    zones `fun_001ee4b0` (`rc_game::fog_zones::update`), which overwrite the level fog globals. Nothing is
//!    restored on leaving a zone.
//! 3. **Tint**: while the flag is set `DrawDebugProfiler` draws full-screen sprites with RGBAQ 0x161200..03
//!    and ALPHA_1 = 0x8000000044 (`Cd + (Cs − Cd)·A >> 7`: A 0x40 = 50 %, 0x30 = 37.5 %), untextured, no Z,
//!    **after the HUD** (its `0x15f3f4 & 0x40` pass follows the `& 0x80` HUD pass). Here: the
//!    [`UnderwaterTint`] full-screen pass on the main camera, after `bevy::ui_render::ui_pass`, blending in
//!    display-encoded bytes like the GS (`assets/shaders/underwater_tint.wgsl`).
//!
//! Environment: `RC_FOG_ZONES=0` disables the zones (the level settings stay); `RC_UNDERWATER=1` / `=0` forces
//! the underwater flag on / off; `RC_CAM_PATH=ex,ey,ez,tx,ty,tz;ex,ey,ez,tx,ty,tz` moves the camera linearly from
//! the first to the second `RC_CAM` value over `RC_CAM_PATH_FRAMES` (default 120) frames, for walk-through
//! captures.

use crate::determinism::FrameNumber;
use crate::fly_cam::FlyCam;
use crate::game_camera::{game_eye, GameFog, LevelFog};
use crate::tfrag_render::game_to_bevy;
use anyhow::{Context, Result};
use bevy::core_pipeline::fullscreen_material::FullscreenMaterial;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;
use rc_formats::collision::Collision;
use rc_formats::gameplay::FogZones;
use rc_game::fog_zones::{self, FogGlobals, UnderwaterLook, UnderwaterState};
use std::path::Path;
use std::time::Instant;

/// The per-frame fog update; every fog consumer runs after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FogSet;

/// Level data for the fog state: gameplay section 0x80 and the collision mesh for the underwater test.
pub struct FogLevelData {
    pub zones: FogZones,
    pub mesh: Option<Collision>,
    pub lights: rc_formats::gameplay::StaticLights,
}

/// Reads the level's fog zones (gameplay section 0x80) and collision mesh.
pub fn load(root: &Path, index: u32) -> Result<FogLevelData> {
    let t0 = Instant::now();
    let read = |name: &str| crate::disc_source::level_file(root, index, name);
    let gameplay = rc_data::level_gameplay(root, index).context("decompressing gameplay_ntsc")?;
    let zones = rc_formats::gameplay::parse_fog_zones(&gameplay).context("parsing fog zones")?;
    let core_index = read("core_index.bin")?;
    let core_data = rc_data::level_core_data(root, index).context("decompressing core_data")?;
    let core = rc_formats::level::parse_level_core(&core_index, core_data.len()).context("parsing core index")?;
    let mesh = match rc_formats::collision::parse_collision(&core, &core_data) {
        Ok(m) => Some(m),
        Err(e) => {
            eprintln!("fog: no collision mesh ({e}); the underwater test is off");
            None
        }
    };
    println!(
        "fog: {} fog zones, collision {}; load {:.1} ms (RC_FOG_ZONES=0 disables zones, RC_UNDERWATER=1 forces underwater)",
        zones.zones.len(),
        if mesh.is_some() { "loaded" } else { "missing" },
        t0.elapsed().as_secs_f64() * 1e3
    );
    let lights = rc_formats::gameplay::parse_static_lights(&gameplay).context("parsing static point lights")?;
    Ok(FogLevelData { zones, mesh, lights })
}

/// The game state behind the fog: the level fog globals, the underwater flag and look.
#[derive(Resource)]
pub struct FogState {
    zones: FogZones,
    /// The level's static point lights and their grid (0x15fb80 / 0x15fbc0: `HeroEnvLighting`).
    lights: rc_formats::gameplay::StaticLights,
    mesh: Option<Collision>,
    /// 0x15f444..0x15f454.
    level: FogGlobals,
    underwater: UnderwaterState,
    look: UnderwaterLook,
    zones_enabled: bool,
    /// `RC_UNDERWATER`: Some(true) forces the flag on, Some(false) off; None runs the test.
    force_underwater: Option<bool>,
    /// For the log line: the zone and t last printed.
    last_zone: Option<usize>,
    last_t: f32,
    /// The Visibomb's saved fog and the last of its writes applied (crate::visibomb_view).
    visibomb: crate::visibomb_view::FogSwap,
    /// The tick of the last `WaterWorld::underwater_store` applied.
    store_tick: Option<u64>,
    /// The tick of the last `WaterWorld::fog_store` applied.
    fog_store_tick: Option<u64>,
}

impl FogState {
    /// `level_fog`: the level settings (`fun_001e9b10` → 0x15f444..).
    pub fn new(data: FogLevelData, level_fog: &LevelFog) -> Self {
        let env = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string());
        FogState {
            zones: data.zones,
            mesh: data.mesh,
            lights: data.lights,
            level: level_fog.globals(),
            underwater: UnderwaterState::default(),
            look: UnderwaterLook::default(),
            zones_enabled: env("RC_FOG_ZONES").as_deref() != Some("0"),
            force_underwater: match env("RC_UNDERWATER").as_deref() { Some("1") => Some(true), Some("0") => Some(false), _ => None },
            last_zone: None,
            last_t: 0.0,
            visibomb: Default::default(),
            store_tick: None,
            fog_store_tick: None,
        }
    }

    /// The underwater flag 0x167494 as the last camera update left it (the sound layer reads it: crate::audio_out).
    pub fn underwater_flag(&self) -> bool { self.underwater.flag }
}

/// The underwater full-screen tint (`DrawDebugProfiler`, 0x161200..03): GS RGBA bytes, A = 0x80 → 1.0.
#[derive(Component, ExtractComponent, Clone, Copy, Default, PartialEq, ShaderType)]
pub struct UnderwaterTint {
    pub rgba: UVec4,
}

impl FullscreenMaterial for UnderwaterTint {
    fn fragment_shader() -> ShaderRef { "shaders/underwater_tint.wgsl".into() }

    /// After the main passes (so after particles) and after the UI pass (the HUD), before upscaling.
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system
            .after(bevy::core_pipeline::Core3dSystems::PostProcess)
            .after(bevy::ui_render::ui_pass)
            .before(bevy::core_pipeline::upscaling::upscaling)
    }
}

/// `RC_CAM_PATH`: (eye, target) at the start and the end, and the frame count.
#[derive(Resource)]
struct CamPath {
    from: ([f32; 3], [f32; 3]),
    to: ([f32; 3], [f32; 3]),
    frames: u64,
}

pub struct FogStatePlugin;

impl Plugin for FogStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::gs_post::GsPostPlugin::<UnderwaterTint>::default())
            .configure_sets(PostUpdate, FogSet.before(crate::occlusion::OcclusionSet))
            .add_systems(PostUpdate, update_fog_state.in_set(FogSet))
            .add_systems(FixedUpdate, hero_env_lighting.after(crate::gameplay::GameTick));
        if let Ok(v) = std::env::var("RC_CAM_PATH") {
            let mut parts = v.split(';').map(crate::parse_cam);
            match (parts.next().flatten(), parts.next().flatten()) {
                (Some(from), Some(to)) => {
                    let frames = std::env::var("RC_CAM_PATH_FRAMES").ok().and_then(|s| s.trim().parse().ok()).unwrap_or(120u64).max(1);
                    app.insert_resource(CamPath { from, to, frames }).add_systems(Update, camera_path);
                }
                _ => eprintln!("RC_CAM_PATH must be two RC_CAM values separated by ';', got {v:?}"),
            }
        }
    }
}

/// `HeroEnvLighting` 0x26be04 (every hero update, after the map fog writer): Ratchet's light word +0x38 and ambient
/// +0x3c from the fog zone and the static point light holding him (`rc_game::fog_zones::hero_env_lighting`), which
/// the renderer and the classes copying his lighting read. Run after each gameplay tick.
fn hero_env_lighting(state: Option<Res<FogState>>, play: Option<ResMut<crate::gameplay::Play>>) {
    let (Some(state), Some(mut play)) = (state, play) else { return };
    let g = &mut play.game;
    let pos = g.hero.position();
    let Some(m) = g.mobys.mobys.get_mut(g.hero_moby) else { return };
    let base = g.hero.env_base.get_or_insert(m.ambient);
    let zones = state.zones_enabled.then_some(&state.zones);
    fog_zones::hero_env_lighting(zones, &state.lights, pos, base, &mut m.light, &mut m.ambient);
}

type MainCamera<'w, 's> = Query<'w, 's, (Entity, &'static Transform, Option<&'static UnderwaterTint>), With<FlyCam>>;

fn update_fog_state(
    mut commands: Commands,
    state: Option<ResMut<FogState>>,
    mut game_fog: ResMut<GameFog>,
    tie: Option<ResMut<crate::tie_lod::TieLodState>>,
    water: Option<Res<crate::water_render::WaterState>>,
    mut play: Option<ResMut<crate::gameplay::Play>>,
    travel: Option<Res<crate::travel_render::Travel>>,
    cams: MainCamera,
) {
    let Some(mut state) = state else { return };
    let state = &mut *state;
    // The Visibomb's missile view writes the level fog globals during the ticks (crate::visibomb_view).
    if let Some(p) = play.as_deref() { crate::visibomb_view::apply_fog(&p.svc.visibomb, &mut state.level, &mut state.underwater.flag, &mut state.visibomb); }
    // 1. UpdateFog (end of the previous frame's render): what this frame draws with.
    let (mut view, far12) = fog_zones::update_fog(&state.level, state.underwater.flag, &state.look);
    // A level transition's loops do not run UpdateFog: its own fog block stays (fog_zones::TRANSITION_FOG / FLIGHT_FOG).
    if let Some(f) = travel.as_deref().and_then(|t| t.transition_fog()) { view = f; }
    let view = LevelFog::from_globals(&view);
    if game_fog.fog != view || game_fog.particle_far12 != far12 { *game_fog = GameFog::new(view, far12); }
    if let Some(mut tie) = tie {
        if tie.fog != view { tie.fog = view; }
    }

    // 2. Camera update: underwater test, then the fog zones.
    let Some((entity, t, tint)) = cams.iter().next() else { return };
    let eye = game_eye(t).to_array();
    let was = state.underwater.flag;
    // The game code's own stores into 0x167494 during the ticks (HeroTeleport, the surface jump: `UnderwaterStore`),
    // each applied once, before the camera's test.
    if let Some((tick, store)) = play.as_deref().and_then(|p| p.svc.water.underwater_store) {
        if state.store_tick != Some(tick) {
            state.store_tick = Some(tick);
            state.underwater.flag = match store {
                rc_game::water::world::UnderwaterStore::Off => false,
                rc_game::water::world::UnderwaterStore::HeroGroup => play.as_deref().is_some_and(|p| p.game.hero.group == 0x11),
            };
        }
    }
    // The classes' stores into the level fog globals (Rilgar's 841), each applied once, before the fog zones.
    if let Some((tick, g)) = play.as_deref().and_then(|p| p.svc.water.fog_store) {
        if state.fog_store_tick != Some(tick) {
            state.fog_store_tick = Some(tick);
            state.level = g;
        }
    }
    if let Some(forced) = state.force_underwater {
        state.underwater.flag = forced;
    } else if play.as_deref().is_some_and(|p| p.game.camera.type6_active() || p.svc.game_mode != 0) {
        // UnderwaterTest 0x20e9f0: 0 while the active camera is of type 6 (`+0x86 == 6`: the Visibomb's view) or the
        // game mode 0x15f5c4 is not 0 (a cutscene).
        state.underwater.flag = false;
    } else if let Some(mesh) = &state.mesh {
        // Water height: 0x26ed38 = the level's water (`Services::water`: the active ripple patch, then the flat
        // plane), falling back to the hit point; without the game tick the plugin's Novalis ripples.
        match play.as_deref() {
            // `CollLine_Fix(…, 0x12)`: the world mesh, then the moby meshes (the patch managers' water is moby
            // collision).
            Some(p) => {
                let src = p.svc.scene_parts(&p.game.mobys, &*p.classes);
                let scene = p.svc.scene(&src);
                state.underwater.update_with_scene(mesh, &scene, eye, |q| p.svc.water.water_height(q));
            }
            None => {
                let ripple = water.as_ref().and_then(|w| w.fallback.as_ref());
                state.underwater.update_with_mesh(mesh, eye, |q| ripple.and_then(|r| r.patch_height(q[0], q[1], q[2])));
            }
        }
    }
    // The alternate colours: as the level's water managers leave 0x161200.. (751 per zone on Novalis, the patch
    // managers' init values elsewhere); without the game tick, the plugin's 751 zone.
    match play.as_deref() {
        Some(p) => state.look = p.svc.water.look,
        None => {
            let zone = water.as_ref().and_then(|w| w.fallback.as_ref()).map_or(-1, |r| r.last.zone);
            state.look.set_from_ripple_zone(usize::try_from(zone).ok());
        }
    }
    if state.underwater.flag != was { println!("underwater: {} at {eye:.2?}", if state.underwater.flag { "on" } else { "off" }); }
    if state.zones_enabled {
        let hit = fog_zones::update(&mut state.level, &state.zones, eye);
        let zone = hit.map(|(i, _)| i);
        // Log on entering/leaving and every 0.1 of t inside a zone.
        if zone != state.last_zone || hit.is_some_and(|(_, t)| (t - state.last_t).abs() >= 0.1) {
            let g = &state.level;
            match hit {
                Some((i, t)) => println!(
                    "fog zone {i} (flags {}) t {t:.3}: FOGCOL {:?}, near {:.1} far {:.1} units, F {:.1}..{:.1}",
                    state.zones.zones[i].flags, g.color, g.near_dist / 1024.0, g.far_dist / 1024.0, g.near_intensity, g.far_intensity
                ),
                None => println!("fog zone: left (the level fog keeps its last values)"),
            }
            state.last_zone = zone;
            state.last_t = hit.map_or(0.0, |(_, t)| t);
        }
    }
    // The moby code's copy of 0x15f444.. as the camera update leaves it (a store the next tick makes reads it).
    if let Some(p) = play.as_deref_mut() {
        if p.svc.water.fog != Some(state.level) { p.svc.water.fog = Some(state.level); }
    }

    // 3. The tint reads the flag as this frame's camera update left it.
    let want = state.underwater.flag.then(|| UnderwaterTint { rgba: UVec4::from_array(state.look.tint.map(u32::from)) });
    match (want, tint) {
        (Some(w), Some(have)) if *have == w => {}
        (Some(w), _) => { commands.entity(entity).insert(w); }
        (None, Some(_)) => { commands.entity(entity).remove::<UnderwaterTint>(); }
        (None, None) => {}
    }
}

/// `RC_CAM_PATH`: frame n (1-based) puts the camera at `lerp(from, to, min(n, frames) / frames)`.
fn camera_path(path: Res<CamPath>, frame: Res<FrameNumber>, mut cams: Query<&mut Transform, With<FlyCam>>) {
    let s = frame.0.min(path.frames) as f32 / path.frames as f32;
    let lerp = |a: [f32; 3], b: [f32; 3]| Vec3::from_array(a).lerp(Vec3::from_array(b), s).to_array();
    let (eye, target) = (lerp(path.from.0, path.to.0), lerp(path.from.1, path.to.1));
    for mut t in &mut cams {
        *t = Transform::from_translation(game_to_bevy(eye)).looking_at(game_to_bevy(target), Vec3::Y);
    }
}
