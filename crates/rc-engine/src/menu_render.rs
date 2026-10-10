//! The in-level menus in the engine: the mode system (`rc_game::menus::mode`), the quick-select ring (HUD
//! slot 3) and the mode-3 page menus (pause, Options, map, planet select), drawn through the HUD's 2D pass.
//! Spec: docs/plan/menus.md; the state machines are `rc_game::menus`.
//!
//! **Frame** (`FixedUpdate`, after `gameplay::tick`; one main-loop frame of the game, `entry` 0x259c40):
//! * mode 0: the gameplay tick has run (it is skipped in every mode whose update does not tick,
//!   [`Mode::advances_tick`]); then the ring's hero part (open / double tap) and its HUD update, the pad lock
//!   it requests (PAD+0x1cc = 2 for the next `ProcessPadInput`), and the pause triggers of 0x2aba68 (Start
//!   or pad lost → kind 0, Select|R3 → kind 10, after ≥ 8 frames in mode 0). Draws: the ring.
//! * mode 3: `UpdatePad` on the game's pad (the main loop runs it in every mode), the menu frame
//!   (`SceneController`), its draws (`PageMenuDraw`). The close's post-action: resume; the ship travel (logged:
//!   the travel lane's); a movie / scene / the slideshow after the 16-frame fade over the menu image
//!   ([`post_fade_step`], crate::media_render), the return page reopening the menu before the tick
//!   ([`return_page_reopen`]).
//! * mode 7: the credits slideshow (crate::media_render::Slides). A class's `EnterMenuMode` / `EnterSlideshowMode` /
//!   `PlayMovieB` are taken after its tick (crate::media_render::take_class_requests).
//! * [`MenuMode::loop_frame`] counts main-loop frames: the scripted pad (`RC_PLAY_SCRIPT`) is indexed by it,
//!   so a script keeps its timeline across a pause (the gameplay tick counter stops in mode 3).
//!
//! **Drawing.** The frame's [`MenuDraw`] list becomes HUD primitives ([`Hud2dHook`]): panels are drawn in
//! place (translated, scissored to the panel, cleared to navy first) instead of into a render target copied
//! 1:1, which is the same image for the 1:1 blit every ported widget returns. `fun_00200e08` rectangles and
//! `fun_00200c80` lines use the corner offset of those packets (pixel x − 1). In mode 3 the HUD's own calls
//! are not drawn and it does not tick (the HUD pass belongs to the mode-0 render).
//!
//! **Snapshot** (`DownloadFrameBuffer` at the menu's first tick, a synchronous VRAM download at 0x2b4c88; every
//! later menu frame re-uploads it): the frame on which the menu is entered is still rendered as a gameplay
//! frame (as on the PS2, where the mode switch takes effect next frame), and at the end of that frame's
//! render graph (`RenderGraphSystems::Finish`, after every camera, before present) the GPU copies the main
//! target's finished output attachment (world + HUD, as the game's frame-buffer copy) into the snapshot
//! texture (`assets/shaders/menu_snapshot.wgsl`, `darken`), with the menu's black at alpha 0x30 applied on the
//! way with the GS formula on display bytes (`Cd + ((−Cd·0x30) >> 7)`, i.e. ×0.625), written through a UNORM
//! view so the stored bytes are exact. No CPU read-back: the snapshot is on the GPU before the first menu
//! frame renders, so every menu frame shows it (a function of the frame number, not of the wall clock).
//! The source is whatever the target's `ViewTargetAttachments` entry is at that point: the offscreen capture
//! image in frame-exact runs (`determinism.rs`), a Bevy `Screenshot`'s offscreen texture when one is taken of
//! the same target on that frame (it then blits it to the target after the graph), or, for a window target,
//! a texture the window's cameras render into on that frame only (a swap-chain texture cannot be sampled),
//! which is then copied back to the swap chain (`copy`). The world keeps rendering underneath (hidden).
//! Port-only: when the frame changes size under the menu (Port Options' aspect / resolution, a window resize), the
//! snapshot is taken again from the next frame's render with the menu layer and the HUD hidden (`MenuRt::resnap`).
//!
//! **Menu layer** (`PageMenuDraw` steps 1–3: snapshot, black, then the 14 class-0x472 frame mobys with
//! `DrawMobyList(m, 1)` under TEST_1 0x5360b over a Z buffer the frame's clear packet set to 0): an
//! offscreen image of the main target's size, composited by a UI node on the main camera between the world
//! and the HUD composite (whose 2D pass then draws the navy panel rects and the widgets, steps 4–6). Two
//! cameras render it: a `Camera2d` (order −3) that clears it to black at alpha 0x30/0x80 and draws the
//! darkened snapshot (a UI node), then, only in mode 3, a `Camera3d` (order −2,
//! [`MENU_3D_LAYER`], no colour clear, depth cleared to 0 = far) that draws the frame mobys. The frame mobys
//! (`rc_game::menus::pause::frame`: spawn, animation, corners, rects) are `moby_render::ExtraMobys` instances
//! lit with the menu's light set 14 (`FUN_0028c128` writes it from 0x160280 / 0x160290; light word 0x0e0e,
//! ambient 0x202020 from `SpawnHandGadgetMoby`), palettes from `MobyAnimEval` of each slot's state.
//! **Menu camera.** The game draws them from the menu camera `fun_00218d10` sets: position (256, 256, 64),
//! rows identity (looking along +x, up +z), FOV 0.63 (the default projection). Every system that picks "the"
//! `Camera3d` would also see this one, so it carries the main camera's transform, and the menu view is
//! folded into the model matrices instead: `model' = C_main · C_menu⁻¹ · model`, which puts each moby in the
//! main camera's view space exactly where the menu camera sees it (same view-space depth, so fog and the
//! projection are unchanged).
//!
//! **Also here** (batch 6): the pause tests through `rc_game::menus::mode::in_level_trigger` (the freeze kinds 0 / 1 / 4
//! and their effects, `freeze_effects`), the dialogue player's step in every menu frame (`menu_voice_frame`: the end
//! page's Helpdesk girl, `post_credits_audio`), the language switch (`apply_language`, the front end's `all_text`).
//!
//! **Not ported**: the ring's draw position between HUD slots 2 and 4 (it is drawn after the whole HUD); the frame mobys are fogged with
//! the level's fog (`fog_state` pushes it into every `MobyMaterial`), not the menu's view-context fog
//! (0..524288, F 255..0): at the frames' depth (≈ 5 units) the two differ by at most one step of F.
//!
//! **Port Options** (port-only, `rc_game::menus::pause::port`): an extra "Port Options" entry in the Options
//! list opens a page built from the game's machinery; its anti-aliasing row is synced with
//! [`RenderSettings::msaa`] around every menu tick (the shadows row likewise with `shadow_render::ShadowSettings`) (the resource's value is shown; a ✕ writes it back, applied
//! by `render_settings::apply` and saved to the port settings file). The row offers only the sample counts the
//! GPU supports ([`SupportedMsaa`]; ✕ skips the rest, e.g. 8x on Apple M-series).
//!
//! Environment: `RC_MENU_TRACE=1` prints menu events (open, transitions, sounds, equip requests, close);
//! `RC_SETTINGS_PAGE=0` leaves the Options page as on the disc (no Port Options entry).

use crate::gameplay::{Persistent, Play, Session};
use crate::hud_render::{Hud2d, Hud2dHook, HudBuild, Prim, Tex, H, W};
use crate::input_map::{PadFrame, Script};
use crate::moby_render::{self, ExtraMobys, MobyMaterial};
use crate::render_settings::{self, RenderSettings, SupportedMsaa};
use crate::text_render::{self, TextState};
use anyhow::{anyhow, Context};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::camera::NormalizedRenderTarget;
use bevy::core_pipeline::FullscreenShader;
use bevy::ecs::entity::ContainsEntity;
use bevy::platform::collections::HashMap;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::texture_2d;
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
    CommandEncoder, CommandEncoderDescriptor, Extent3d, FragmentState, LoadOp, Operations, PipelineCache, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, VertexState,
};
use bevy::render::renderer::{RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue};
use bevy::render::storage::ShaderBuffer;
use bevy::render::texture::{GpuImage, OutputColorAttachment};
use bevy::render::view::{prepare_view_attachments, prepare_view_targets, ExtractedWindows, Msaa, ViewTargetAttachments};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::ShaderDefVal;
use bevy::transform::TransformSystems;
use bevy::ui::UiTargetCamera;
use bevy::window::PrimaryWindow;
use rc_formats::moby_light::{self, MobyLights};
use rc_formats::save_game::ItemTables;
use rc_formats::tfrag_light::{ps2, DirLightSet};
use rc_game::hud::{Draw, HudAssets};
use rc_game::menus::mode::{Mode, ModeState};
use rc_game::menus::pause::frame::{self, FrameClass, FrameMobys};
use rc_game::menus::pause::port::Setting;
use rc_game::menus::pause::{MenuEnv, PageMenu, PostAction, DARKEN};
use rc_game::menus::quick_select::{HeroGate, QuickSelect};
use rc_game::menus::screen_static::{StaticDraw, StaticTex};
use rc_game::menus::{MenuAssets, MenuDraw, MenuInput, MenuSound, Overlay};
use rc_game::pad::button;

/// The main-loop frame of the menus (crate::travel_render's mode-6 frame runs before it).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuFrameSet;

/// The system set that builds the menus' 2D primitives into `Hud2dHook` (crate::vendor_render adds to them after).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuPrims;

/// The mode globals and the main-loop frame counter (read by `gameplay::tick`).
#[derive(Resource, Default, Debug)]
pub struct MenuMode {
    pub state: ModeState,
    /// Main-loop frames so far (the index of the frame being run while `gameplay::tick` runs).
    pub loop_frame: u64,
    /// Mode 5 (crate::interact_render): the next frame's update runs the world (`VendorModeUpdate` substates 0 / 2), so
    /// `gameplay::tick` runs once although the mode does not advance the tick; `world_ticked` = it did this frame.
    pub world_tick: bool,
    pub world_ticked: bool,
    /// 0x15eed8, the replay mode the movie and scene players' skip rules read: 0 play, 1 a replay from the page menu
    /// (Start alone skips), 2 the menu's in-level movie replays and the title's attract movies (any button skips), −1
    /// never skippable. The page menu's post-actions set it (keeping the old value in its 0x1ba2a0 for the reopen).
    pub replay: i32,
}

#[derive(Resource)]
struct MenuRt {
    assets: MenuAssets,
    qs: Option<QuickSelect>,
    menu: Option<PageMenu>,
    draws: Vec<MenuDraw>,
    /// The mode the frame's draws belong to (the mode at the frame's start).
    render_mode: Mode,
    script: Option<Script>,
    last_counter: Option<u64>,
    /// The menu was entered this frame: the snapshot is taken of this frame's render.
    snapshot_request: bool,
    /// The snapshot texture holds the entering frame (with the menu's black 0x30 applied to its bytes).
    snapshot_ready: bool,
    /// The snapshot texture (written on the GPU, module docs).
    snapshot: Handle<Image>,
    /// Port-only: frames left with the menu layer and the HUD hidden while the snapshot is taken again after the frame
    /// changed size under the menu (Port Options' aspect / resolution; crate::display).
    resnap: u8,
    trace: bool,
    /// The menu layer (module docs).
    layer: MenuLayer,
    /// The close's movie / scene / slideshow post-action and its `FadeToBlack(ticks(16))` step (crate::media_render).
    post_fade: Option<(PostAction, u32)>,
    /// 0x1ba2a0: the replay mode before the post-action (restored when the return page reopens the menu).
    replay_saved: i32,
    /// Mode 7 (crate::media_render) and its tables.
    slides: Option<crate::media_render::Slides>,
    slide_tables: rc_game::slideshow::Tables,
    /// Mode 4, the card dialog / the save notice (`rc_game::menus::freeze`; crate::saves).
    freeze: Option<rc_game::menus::freeze::Freeze>,
    /// The dialogue line the menu frames loaded (the end page's Helpdesk girl: `post_credits_audio`).
    voice_vag: Option<std::sync::Arc<[u8]>>,
    /// The language whose `all_text` block is the front end's message table (`queue_dma_transfer(0x15ed88)`).
    front_text: Option<u32>,
}

#[derive(Component)]
struct SnapshotNode;

/// Render layers of the menu layer's cameras (the HUD uses 29, the sky 1).
pub const MENU_3D_LAYER: usize = 27;
const MENU_2D_LAYER: usize = 28;

/// The UI node that composites the menu layer onto the main camera.
#[derive(Component)]
struct MenuLayerNode;

/// The mode-3 `Camera3d` of the menu layer.
#[derive(Component)]
struct MenuLayerCam;

struct MenuLayer {
    image: Handle<Image>,
    cam2d: Entity,
    node: Entity,
    cam3d: Option<Entity>,
    frame: Option<FrameRender>,
}

/// The frame mobys' render side: 14 extra instances of class 0x472 (slot i = record / palette block i).
struct FrameRender {
    extra: ExtraMobys,
    entities: Vec<Vec<Entity>>,
    /// Palette slots per moby.
    slots: u32,
    /// Light block for rotation (0, 0, 0) with light set 14 = the menu light; None with `RC_NO_LIGHT`.
    lights: Option<MobyLights>,
    /// `game_to_bevy · [s/1024 · R | p]` of a frame moby (rows from rotation 0, class scale, (256, 256, 64)).
    model: Mat4,
    /// The menu camera's Bevy transform (position (256, 256, 64), rows identity).
    camera: Mat4,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuMode>().init_resource::<crate::interact_render::VendorRt>();
        // A runtime level change (crate::level_switch): the level's menus are loaded again by `setup`.
        app.add_systems(crate::level_switch::LevelUnload, (crate::level_switch::remove::<MenuRt>, crate::level_switch::reset::<crate::interact_render::VendorRt>));
        if !crate::gameplay::enabled() { return; }
        app.init_resource::<SnapshotRequest>()
            .add_systems(First, |mut r: ResMut<SnapshotRequest>| r.0 = None)
            .add_systems(PreUpdate, setup)
            .add_systems(FixedUpdate, menu_frame.after(crate::gameplay::GameTick).in_set(MenuFrameSet))
            // memcard_Update + the card monitor after the mode's update, every frame (crate::saves).
            .init_resource::<crate::saves::FrontEndRt>()
            .add_systems(FixedUpdate, crate::saves::card_frame.after(menu_frame))
            .add_systems(FixedUpdate, return_page_reopen.before(crate::gameplay::GameTick))
            .add_systems(Update, (target_main_camera, build_prims).chain().in_set(MenuPrims).before(HudBuild))
            .add_systems(PostUpdate, menu_layer.before(TransformSystems::Propagate).in_set(crate::display::ResizeTargets))
            .add_systems(PostUpdate, crate::interact_render::hide_hero.after(crate::moby_render::update_moby_occlusion).before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .init_resource::<SnapshotJob>()
            .init_resource::<SnapshotOffscreen>()
            .add_systems(RenderStartup, init_snapshot_pipelines)
            .add_systems(ExtractSchedule, extract_snapshot)
            .add_systems(
                Render,
                prepare_snapshot.in_set(RenderSystems::PrepareViews).after(prepare_view_attachments).before(prepare_view_targets),
            )
            .add_systems(RenderGraph, snapshot_copy.in_set(RenderGraphSystems::Finish));
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut done: Local<bool>,
    generation: Res<crate::level_switch::LevelGeneration>,
    mut commands: Commands,
    level: Res<crate::Level>,
    frame: Res<crate::display::GameFrame>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    (mut mm, mut fer): (ResMut<MenuMode>, ResMut<crate::saves::FrontEndRt>),
) {
    // Once per level (crate::level_switch: a runtime level change runs it again).
    if generation.is_changed() { *done = false; }
    if *done { return; }
    *done = true;
    let Some(lh) = level.0.hud.as_ref() else {
        eprintln!("menus: no HUD data for this level: no menus");
        return;
    };
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    // The level's overlay with its menu address map against level 01's (the code names the records by their
    // level-01 addresses: rc_game::menus::Overlay::relocated).
    let reference = crate::disc_source::level_file(&root, 1, "overlay.bin");
    let overlay = match crate::disc_source::level_file(&root, index, "overlay.bin").and_then(|b| Ok(match &reference {
        Ok(r) => Overlay::relocated(&b, r)?,
        Err(_) => Overlay::parse(&b)?,
    })) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("menus: overlay not read ({e:#}): no menus");
            return;
        }
    };
    let items = crate::disc_source::read_path(&root, &root.join("boot/SCUS_971.99"))
        .and_then(|elf| Ok(ItemTables::load(&elf, &crate::disc_source::level_file(&root, index, "overlay.bin")?)?));
    let qs = match &items {
        Ok(t) => QuickSelect::load(&overlay, t),
        Err(e) => {
            eprintln!("menus: item tables not read ({e:#}): no quick select");
            None
        }
    };
    let mut menu = PageMenu::load(&overlay);
    // The map's mission and marker lists from the level-01 overlay: the same records in every overlay, and their
    // callbacks are ported by their level-01 addresses (rc_game::menus::pause::map_page::callback).
    if let (Some(m), Ok(r)) = (menu.as_mut(), reference.as_ref()) {
        if let Ok(ov1) = rc_game::menus::Overlay::parse(r) { m.map.tables = rc_game::menus::pause::map_page::MapTables::read(&ov1); }
    }
    // The item definitions for the Gadgets / Weapons grids (rc_game::inventory; the overlay's 0x179f40 table).
    if let (Some(m), Ok(t)) = (menu.as_mut(), &items) {
        let n = rc_formats::save_game::ITEM_COUNT * rc_formats::save_game::ITEM_DEF_SIZE;
        if let Some(raw) = overlay.bytes(t.item_defs_addr, n) { m.items = Some(std::sync::Arc::new(rc_game::inventory::ItemInfos::from_raw(raw))); }
        // The price records' "has ammo" (+8) and max ammo (+0xe) for the Weapons page's ammo text.
        let u = |r: &rc_formats::save_game::ItemRecord, o: usize| u16::from_le_bytes([r.0[o], r.0[o + 1]]);
        m.ammo_records = t.records.iter().map(|r| (u(r, 8), u(r, 0xe))).collect();
    }
    if let Some(m) = menu.as_mut() { m.lang = lh.lang; }
    // The end page's Helpdesk girl: class 0x7a5 (when the level has it: `SpawnHandGadgetMoby`) with her streamed
    // sequences (rc_game::menus::pause::media, crate::menu_models::girl_anim_class).
    if let Some(m) = menu.as_mut() {
        let mobys = &level.0.mobys;
        let girl = rc_game::menus::pause::media::GIRL_CLASS as i32;
        if let Some(ci) = mobys.classes.iter().position(|c| c.o_class == girl) {
            m.media.girl_class = crate::menu_models::girl_anim_class(&mobys.anim[ci]).map(|a| rc_game::menus::pause::media::GirlClass(std::sync::Arc::new(a)));
        }
    }
    // The frame mobys: class 0x472 from the level core.
    let frame_class = match load_frame_class(&level.0, &overlay) {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!("menus: no frame mobys ({e:#}): no panels are placed");
            None
        }
    };
    if let (Some(m), Some((fc, _))) = (menu.as_mut(), &frame_class) { m.frames = Some(FrameMobys::new(fc.clone(), false)); }
    // Port-only: the "Port Options" page and its entry in the Options list (RC_SETTINGS_PAGE=0: none).
    if let Some(m) = menu.as_mut().filter(|_| std::env::var("RC_SETTINGS_PAGE").map_or(true, |v| v.trim() != "0")) {
        if !m.install_port_page(&overlay) { eprintln!("menus: Port Options page not installed (Options page records missing)"); }
    }
    println!(
        "menus: quick select {} (neighbour table {:#x}), page menu {} ({} pages, {} widgets); RC_MENU_TRACE=1 logs events",
        if qs.is_some() { "ready" } else { "missing" },
        qs.as_ref().map_or(0, |q| q.tables.table_addr),
        if menu.is_some() { "ready" } else { "missing" },
        menu.as_ref().map_or(0, |m| m.pages.len()),
        menu.as_ref().map_or(0, |m| m.widgets.len()),
    );
    let script = std::env::var("RC_PLAY_SCRIPT").ok().filter(|s| !s.trim().is_empty()).and_then(|s| Script::parse(&s).ok());
    // The game frame's size (crate::display): the layer and the snapshot are laid over it.
    let size = frame.size.max(UVec2::ONE);
    let mut layer_image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    layer_image.sampler = bevy::image::ImageSampler::nearest();
    let image = images.add(layer_image);
    // The snapshot: sampled through its sRGB view by the UI, written through a UNORM view (exact bytes).
    let mut snap_image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    snap_image.texture_descriptor.view_formats = &[TextureFormat::Rgba8Unorm];
    snap_image.sampler = bevy::image::ImageSampler::nearest();
    let snapshot = images.add(snap_image);
    let cam2d = commands
        .spawn((
            Camera2d,
            Camera {
                order: -3,
                is_active: false,
                clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, DARKEN as f32 / 128.0)),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Msaa::Off,
            Tonemapping::None,
            DebandDither::Disabled,
            RenderLayers::layer(MENU_2D_LAYER),
            Name::new("menu layer 2D camera (snapshot + black)"),
        ))
        .id();
    let node = commands
        .spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            ImageNode::new(image.clone()),
            GlobalZIndex(i32::MAX - 1),
            Visibility::Hidden,
            MenuLayerNode,
            Name::new("menu layer (snapshot, black, frame mobys)"),
        ))
        .id();
    let frame = frame_class.map(|(fc, ci)| frame_render(&mut commands, &level.0, &overlay, &fc, ci, &mut meshes, &mut images, &mut materials, &mut buffers));
    let layer = MenuLayer { image, cam2d, node, cam3d: None, frame };
    // The map picture's palettes: the grid icon's frame per level (rc_game::menus::pause::map_page::MapPage::palettes).
    if let Some(m) = menu.as_mut() {
        m.map.palettes = (0..rc_game::map::LEVELS as i32)
            .map(|l| lh.hud.frame_image(lh.hud.icon_frame(rc_game::menus::pause::map_page::GRID_ICON, l)).ok().map(|im| im.clut.to_vec()))
            .collect();
    }
    let slide_tables = rc_game::slideshow::Tables::read(&overlay);
    let mut assets = MenuAssets::new(HudAssets::new(&lh.hud, lh.glyphs, lh.messages.clone()), overlay);
    // The global `all_text` in the game's language (the Help Log page's message table, `MenuTextLoad`).
    assets.all_text = load_all_text(&root, lh.lang);
    commands.insert_resource(MenuRt {
        assets,
        qs,
        menu,
        draws: Vec::new(),
        render_mode: Mode::Gameplay,
        script,
        last_counter: None,
        snapshot_request: false,
        snapshot_ready: false,
        snapshot: snapshot.clone(),
        resnap: 0,
        trace: std::env::var("RC_MENU_TRACE").is_ok_and(|v| v.trim() == "1"),
        layer,
        post_fade: None,
        replay_saved: 0,
        slides: None,
        slide_tables,
        freeze: None,
        voice_vag: None,
        front_text: None,
    });
    // The memory card (crate::saves) and, unless a level start was asked for (crate::saves::front_end_requested), the boot flow (the first rand() of the boot: newlib's
    // initial state [L]).
    // Once per process: the boot's (crate::level_switch runs this set-up again for every level; a Quit Game's level change
    // to the front end sets the flag itself, crate::travel_render).
    if generation.0 == 0 {
        if crate::saves::front_end_requested() { crate::saves::set_front_end_active(true); }
        crate::saves::setup_card();
    }
    if crate::saves::front_end_active() {
        fer.fe = Some(crate::saves::front_end(lh.lang, rc_game::rng::Rng::new().rand()));
        mm.state.set(Mode::Menu);
        println!("menus: front end: the boot flow runs over the loaded level, the world hidden");
    }
    // The snapshot is the menu layer's background: a UI node of its 2D camera.
    commands.spawn((
        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        ImageNode::new(snapshot),
        Visibility::Hidden,
        SnapshotNode,
        UiTargetCamera(cam2d),
        Name::new("menu snapshot (frame buffer copy)"),
    ));
}

/// Class 0x472 (index into the level's moby classes) and its corner chains: the joint lists named at
/// 0x161fe0, read from the class blob in the level core.
fn load_frame_class(level: &crate::level_load::LoadedLevel, ov: &Overlay) -> anyhow::Result<(FrameClass, usize)> {
    let m = &level.mobys;
    let ci = m.classes.iter().position(|c| c.o_class == frame::O_CLASS).ok_or_else(|| anyhow!("class {:#x} is not on this level", frame::O_CLASS))?;
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let read = |name: &str| crate::disc_source::level_file(&root, index, name);
    let data = rc_data::level_core_data(&root, index).context("decompressing core_data")?;
    let core = rc_formats::level::parse_level_core(&read("core_index.bin")?, data.len()).context("parsing core index")?;
    let name = format!("moby_class/{:04}", frame::O_CLASS);
    let blk = core.blocks.iter().find(|b| b.name == name).ok_or_else(|| anyhow!("no {name} block"))?;
    let blob = data.get(blk.offset..blk.offset + blk.size).ok_or_else(|| anyhow!("{name} out of range"))?;
    let header = &m.classes[ci].class.header;
    let mut chains: [Vec<u8>; 4] = Default::default();
    for (k, c) in chains.iter_mut().enumerate() {
        let id = ov.i32(ov.at(frame::CORNER_LISTS_ADDR) + 4 * k as u32).ok_or_else(|| anyhow!("0x161fe0 not in the overlay"))?;
        *c = rc_formats::gadget::joint_list(blob, header, usize::try_from(id)?).with_context(|| format!("joint list {id}"))?.0;
    }
    Ok((FrameClass { anim: m.anim[ci].clone(), chains, scale: header.scale }, ci))
}

/// The 14 frame-moby instances (hidden, [`MENU_3D_LAYER`]) and their light block.
#[allow(clippy::too_many_arguments)]
fn frame_render(
    commands: &mut Commands,
    level: &crate::level_load::LoadedLevel,
    ov: &Overlay,
    fc: &FrameClass,
    ci: usize,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> FrameRender {
    let class = &level.mobys.classes[ci];
    let slots = (fc.anim.joint_count as u32).max(ExtraMobys::max_skinned_joint(class) as u32 + 1).max(1);
    let n = frame::SLOTS as u32;
    let records = vec![0u8; frame::SLOTS * moby_render::EXTRA_RECORD_SIZE];
    let mut extra = ExtraMobys::new(level, records, crate::moby_anim::identity_palette(n * slots), buffers);
    let entities: Vec<Vec<Entity>> = (0..n)
        .map(|slot| {
            let es = extra.spawn(commands, level, class, slot, Transform::IDENTITY, "menu frame", meshes, images, materials);
            for &e in &es { commands.entity(e).insert((RenderLayers::layer(MENU_3D_LAYER), Visibility::Hidden)); }
            es
        })
        .collect();
    let rows = moby_light::rotation_rows([0.0; 3]);
    // FUN_0028c128: light set 14 (0x1806c0) = { colour A = *0x160290, direction A = VecScale(1.0, *0x160280), 0, 0 }.
    let q = |a: u32| -> [f32; 4] { std::array::from_fn(|k| f32::from_bits(ov.u32(ov.at(a) + 4 * k as u32).unwrap_or(0))) };
    let dir = q(frame::LIGHT_DIR_ADDR);
    let set = DirLightSet {
        color_a: q(frame::LIGHT_COLOR_ADDR),
        dir_a: [0, 1, 2, 3].map(|k| if k < 3 { f32::from_bits(ps2::mul(dir[k].to_bits(), ps2::ONE)) } else { dir[3] }),
        color_b: [0.0; 4],
        dir_b: [0.0; 4],
    };
    let lights = level.mobys.lighting.as_ref().map(|l| {
        let mut bank = l.bank.clone();
        bank.sets[frame::LIGHT_SET] = set;
        moby_light::moby_lights(&rows, &bank, frame::LIGHT_WORD, frame::AMBIENT, 0x80)
    });
    let rows_f32 = rows.map(|r| [0, 1, 2].map(|k| f32::from_bits(r[k])));
    let model = moby_render::extra_model(rows_f32, fc.scale, frame::CAMERA_POS);
    let camera = Transform::from_translation(crate::tfrag_render::game_to_bevy(frame::CAMERA_POS)).looking_to(Vec3::X, Vec3::Y).to_matrix();
    println!(
        "menus: frame mobys: class {:#x} ({} joints, {} sequences, corner joints {:?}), {} palette slots each; menu light {:?} / {:?}",
        frame::O_CLASS, fc.anim.joint_count, fc.anim.sequences.len(), fc.chains.iter().map(|c| c.last().copied()).collect::<Vec<_>>(), slots, set.color_a, set.dir_a
    );
    FrameRender { extra, entities, slots, lights, model, camera }
}

fn target_main_camera(mut commands: Commands, nodes: Query<Entity, (With<MenuLayerNode>, Without<UiTargetCamera>)>, cams: Query<Entity, With<crate::fly_cam::FlyCam>>) {
    let Some(cam) = cams.iter().next() else { return };
    for n in &nodes { commands.entity(n).insert(UiTargetCamera(cam)); }
}

/// The anti-aliasing row's values (Off, 2x, 4x, 8x) as sample counts.
const AA_SAMPLES: [u32; 4] = [1, 2, 4, 8];

/// The graphics rows (crate::graphics) from the settings, before the menu tick: the preset row shows the preset the
/// options are (Custom included) but ✕ offers only Original and Enhanced.
fn sync_graphics_rows(menu: &mut PageMenu, g: &crate::graphics::GraphicsSettings) {
    use crate::graphics::GfxOption;
    menu.set_port_choices(Setting::Preset, 0b011);
    menu.set_port_value(Setting::Preset, g.preset().index());
    menu.set_port_value(Setting::Hud, g.hud.index());
}

/// The graphics rows back into the settings after the menu tick: a new preset sets every option; otherwise each row is
/// read. A change is saved to the port settings file.
fn read_graphics_rows(menu: &PageMenu, g: &mut crate::graphics::GraphicsSettings, before: &crate::graphics::GraphicsSettings, frame: u64) {
    use crate::graphics::{GfxOption, Hud, Preset};
    let preset = menu.port_value(Setting::Preset).map(Preset::from_index);
    match preset {
        Some(p) if p != Preset::Custom && p != before.preset() => *g = before.with_preset(p),
        _ => {
            if let Some(v) = menu.port_value(Setting::Hud) { g.hud = Hud::from_index(v); }
        }
    }
    if *g != *before {
        println!("menus: frame {frame}: Port Options: graphics {g:?} ({:?})", g.preset());
        g.save();
    }
}

fn aa_index(m: Msaa) -> u8 { AA_SAMPLES.iter().position(|&n| n == m.samples()).unwrap_or(0) as u8 }

/// The anti-aliasing row's selectable values (bit k = `AA_SAMPLES[k]`) on this device.
fn aa_choices(s: &SupportedMsaa) -> u32 {
    AA_SAMPLES.iter().enumerate().filter(|(_, n)| s.0.contains(n)).fold(0, |m, (k, _)| m | 1 << k)
}

/// The hand item shown now (0x140408): the wrench while `0x15ed90` (wrench held) is set, else the
/// equipped hand item [M: the hand-swap state machine is not ported].
fn held_item(g: &rc_game::game_state::Global) -> i32 { if g.wrench_held != 0 { 8 } else { g.equipped[0] } }

/// The "use" system's resources (crate::interact_render).
type InteractParams<'w> = (
    ResMut<'w, crate::interact_render::VendorRt>,
    ResMut<'w, crate::hud_render::HudFeed>,
    Option<ResMut<'w, crate::play_camera::PlayView>>,
    Option<ResMut<'w, crate::audio_out::AudioOut>>,
);

#[allow(clippy::too_many_arguments)]
fn menu_frame(
    rt: Option<ResMut<MenuRt>>,
    mut mm: ResMut<MenuMode>,
    play: Option<ResMut<Play>>,
    gs: Option<ResMut<Persistent>>,
    sess: Option<ResMut<Session>>,
    pad: Res<PadFrame>,
    source: Res<crate::game_camera::CameraSource>,
    mut render: Option<ResMut<RenderSettings>>,
    supported: Option<Res<SupportedMsaa>>,
    (mut vr, mut feed, mut view, mut audio): InteractParams,
    (mut shadows, mut display, mut gfx): (
        Option<ResMut<crate::shadow_render::ShadowSettings>>,
        ResMut<crate::display::DisplaySettings>,
        ResMut<crate::graphics::GraphicsSettings>,
    ),
    mut widgets3d: ResMut<crate::menu_models::GadgetsPreview>,
    (mut fer, movies): (ResMut<crate::saves::FrontEndRt>, Option<Res<crate::movie_render::MovieState>>),
) {
    let (Some(mut rt), Some(mut play), Some(mut gs), Some(mut sess)) = (rt, play, gs, sess) else { return };
    let rt = &mut *rt;
    let (gs, sess) = (&mut gs.0, &mut sess.0);
    // The front end (crate::saves): the boot flow instead of the level's modes.
    if fer.fe.is_some() {
        let busy = movies.as_deref().is_some_and(|m| m.busy());
        let fer = &mut *fer;
        front_end_frame(rt, &mut mm, &mut play, gs, fer, &pad, &source, busy, audio.as_deref_mut());
        mm.state.end_frame();
        mm.loop_frame += 1;
        return;
    }
    let mode = mm.state.mode;
    let counter = play.game.counter;
    if mode == Mode::Gameplay && rt.last_counter == Some(counter) { return; }
    rt.last_counter = Some(counter);
    mm.state.begin_frame();
    let frame = mm.loop_frame;
    let vsync = frame as u32;
    rt.render_mode = mode;
    rt.draws.clear();
    let trace = rt.trace;
    match mode {
        Mode::Gameplay => {
            let inp = MenuInput::from_pad(&play.game.pad, true);
            // 0x15f594 as the hero update saw it: a context-prompt owner blocks the ring (the vendor's △, Blarg's
            // button are not also a ring open). 0x1403fc the hand swap; 0x1413fc no control (the scripted holds
            // 0x72 / 0x32 / 0x78: the camera runs, the tube ride) and 0x1413f7 unless 0x1413f4 = 1 or 0x13f502;
            // 0x1413f4: the ring opens on foot or in the disguise; Clank's △ opens his command menu (the same slot).
            let h = &play.game.hero;
            let gate = HeroGate {
                early_exit: sess.hp < 1,
                swap_state: h.items.slot.swap,
                f594: play.svc.interact.owner_at_hero,
                b13fc: h.items.f13fc,
                b13f7: h.items.f13f7,
                b13f4: h.mode,
                s3f502: h.f502,
                held_item: held_item(&gs.global),
            };
            if let Some(qs) = rt.qs.as_mut() {
                let listeners = play.svc.units.bot_listeners;
                let h = qs.hero(&inp, &gate, &mut gs.global, sess);
                let u = qs.update(&inp, &gate, &mut gs.global, sess, vsync, listeners);
                if u.lock_pad { play.game.pad.lock = 2; }
                // Clank's command menu: 0x141610 for the gadgetbots and HeroUpdateAlt (next tick).
                if let Some(c) = u.command { play.game.hero.bodies.command = c; }
                // 0x242930: the ring's △ also keeps health and bolts up (`HudShowHealth` + the bolt counter, ScaleTicks(180)).
                if h.show_health_bolts { feed.calls.push(rc_game::hud::Call::ShowHealthBolts(rc_game::hud::scale_ticks(180))); }
                if trace || h.request.is_some() || u.closed || h.opened {
                    if h.opened { println!("menus: frame {frame}: quick select opened (slots {:?})", gs.global.quick_select); }
                    if let Some(r) = h.request { println!("menus: frame {frame}: double tap: hand request item {r} (0x141408)"); }
                    if u.closed {
                        println!("menus: frame {frame}: quick select closed, selection {} → hand request {:?} (0x141408 = {}; the hand swap FUN_002307e0 runs in hero/items.rs)", qs.sel, u.request, sess.temp_hand);
                    }
                }
                qs.draw(&rt.assets, &gs.global, play.game.counter, listeners, &mut rt.draws);
            }
            // 0x2abfb0: once the map page has found the level's map (0x184694), move record 9 ("the map was used")
            // is bumped every mode-0 frame (the Novalis director's map hint waits for it).
            if rt.menu.as_ref().is_some_and(|m| m.map.available) {
                let (level, t) = (gs.global.level, gs.global.play_time);
                rc_game::help::bump(&mut gs.global.move_help[9], level, t);
            }
            // 0x2aba68: the pause tests (rc_game::menus::mode::in_level_trigger): Giant Clank's / the vehicles' / the
            // riders' "Quit?" dialogs (mode_freezeInit 4 / 1 / 0), Start / pad lost → the pause menu, Select | R3 → the map.
            let h = &play.game.hero;
            let t = rc_game::menus::mode::TriggerIn {
                mode: mm.state.mode.raw(),
                frames_in_mode: mm.state.frames_in_mode,
                level: gs.global.level,
                pressed: inp.pressed,
                connected: inp.connected,
                state: h.state,
                group: h.group,
                body: h.mode,
                // 0x1403fc (the hand swap) and the camera's script lock +0x86: not kept by the port's hero / camera
                // (G-UI-019). The ridden moby 0x140940 is the vehicle record's (`rc_game::vehicle`).
                swap_state: 0,
                fell: h.fell_out != 0,
                hp: sess.hp,
                riding_class: play.svc.vehicle.moby.and_then(|v| play.game.mobys.mobys.get(v)).map(|m| m.o_class),
                camera_lock: 0,
                debug_step: false,
                c5c4: 0,
            };
            match rc_game::menus::mode::in_level_trigger(&t) {
                Some(rc_game::menus::mode::Trigger::Menu(k)) if rt.menu.is_some() => open_menu(rt, &mut mm, &mut play, gs, audio.as_deref_mut(), k, frame),
                Some(rc_game::menus::mode::Trigger::Freeze(k)) => open_freeze(rt, &mut mm, &mut play, audio.as_deref_mut(), k, 0, frame),
                _ => {}
            }
            // The classes' front-end requests of this tick (crate::media_render): the credits (mode 7), the ending movie,
            // `EnterMenuMode(kind)`.
            if mm.state.mode == Mode::Gameplay {
                crate::media_render::take_class_requests(&mut play);
                while let Some(r) = crate::media_render::take() {
                    match r {
                        crate::media_render::MediaRequest::Menu { kind } => {
                            if mm.state.mode == Mode::Gameplay { open_menu(rt, &mut mm, &mut play, gs, audio.as_deref_mut(), kind, frame); }
                        }
                        crate::media_render::MediaRequest::Slideshow { black_entry } => start_slideshow(rt, &mut mm, &mut play, black_entry, frame),
                    }
                }
            }
            // L00 0x297f78: the save notice, once auto-save is on (crate::saves).
            if mm.state.mode == Mode::Gameplay {
                let gate = rc_game::menus::freeze::NoticeGate { frames_in_mode: mm.state.frames_in_mode, dying: play.game.hero.fell_out != 0, hp: sess.hp, tick: play.game.counter };
                if crate::saves::with_card(|c| rc_game::menus::freeze::save_notice_due(c, gs, &gate)).unwrap_or(false) {
                    gs.global.flags[0x10] = 1;
                    open_freeze(rt, &mut mm, &mut play, audio.as_deref_mut(), rc_game::menus::freeze::KIND_SAVE_NOTICE, 0, frame);
                }
            }
        }
        Mode::Menu => {
            let input = match &rt.script {
                Some(s) => s.at(frame),
                None if *source == crate::game_camera::CameraSource::Play => pad.0,
                None => rc_game::pad::PadInput::neutral(),
            };
            let mirror = gs.options().mirror;
            play.game.pad.update(Some(&input.bytes()), mirror);
            let inp = MenuInput::from_pad(&play.game.pad, true);
            let env = MenuEnv { vsync, b13f4: play.game.hero.mode, pal: false };
            let mut start_fade = None;
            let mut freeze_req: Option<u32> = None;
            if rt.post_fade.is_some() {
                // The close's post-action 3 / 4 / 5 / 6 / 7: `FadeToBlack(ticks(16))` over the menu image, then the action.
                post_fade_step(rt, &mut mm, &mut play, gs, sess.hp, audio.as_deref_mut(), frame, true);
            } else if let Some(menu) = rt.menu.as_mut() {
                menu.set_port_choices(Setting::Msaa, supported.as_deref().map_or_else(|| aa_choices(&SupportedMsaa::default()), aa_choices));
                if let Some(r) = render.as_deref() { menu.set_port_value(Setting::Msaa, aa_index(r.msaa)); }
                if let Some(s) = shadows.as_deref() { menu.set_port_value(Setting::Shadows, !s.enabled as u8); }
                menu.set_port_value(Setting::Resolution, display.resolution.index());
                menu.set_port_value(Setting::Aspect, display.aspect.index());
                menu.set_port_value(Setting::Fullscreen, display.fullscreen as u8);
                let gfx_before = *gfx;
                sync_graphics_rows(menu, &gfx_before);
                // The card and the save inputs moved into the menu for its tick (crate::saves).
                saves_in(menu, &play);
                // 0x15172a as the widgets read it (the Helpdesk girl).
                menu.media.voice_state = play.svc.help.voice.state;
                let out = menu.tick(&inp, gs, &env);
                if let (Some(v), Some(s)) = (menu.port_value(Setting::Shadows), shadows.as_mut()) {
                    if s.enabled != (v == 0) {
                        s.enabled = v == 0;
                        println!("menus: frame {frame}: Port Options: shadows {}", if s.enabled { "on" } else { "off" });
                        s.save();
                    }
                }
                read_graphics_rows(menu, &mut gfx, &gfx_before, frame);
                // The display (crate::display): the game frame's resolution and the window mode.
                let want = (
                    menu.port_value(Setting::Aspect).map(crate::display::Aspect::from_index),
                    menu.port_value(Setting::Resolution).map(crate::display::Resolution::from_index),
                    menu.port_value(Setting::Fullscreen).map(|v| v == 1),
                );
                if let (Some(aspect), Some(res), Some(full)) = want {
                    if display.aspect != aspect || display.resolution != res || display.fullscreen != full {
                        (display.aspect, display.resolution, display.fullscreen) = (aspect, res, full);
                        println!("menus: frame {frame}: Port Options: aspect {aspect:?}, resolution {res:?}, fullscreen {full}");
                        display.save();
                    }
                }
                if let (Some(v), Some(r)) = (menu.port_value(Setting::Msaa), render.as_mut()) {
                    let msaa = render_settings::msaa_from_samples(AA_SAMPLES[v as usize % AA_SAMPLES.len()]);
                    if r.msaa != msaa {
                        r.msaa = msaa;
                        println!("menus: frame {frame}: Port Options: anti-aliasing {} samples", msaa.samples());
                        render_settings::save(r);
                    }
                }
                if trace && !out.sounds.is_empty() { println!("menus: frame {frame}: sounds {:?}", out.sounds); }
                // The class-0x472 sounds (`PlayClassSound(n, 0x11, frame moby)`), the close's audio, then the frame's sound_update.
                if let Some(a) = audio.as_deref_mut() {
                    // The Sound page (kind 0x20): group 2 at the music volume while it is open (`AudioSystem::sound_page`).
                    a.system().sound_page = menu.kind == 0x20 && out.exit.is_none();
                    play_menu_sounds(&mut play, a, &out.sounds);
                    // The movie / scene / slideshow post-actions unpause after their fade and action (post_fade_step).
                    if let Some(x) = out.exit.filter(|x| !is_media_action(x)) { a.system().menu_close(!matches!(x, PostAction::ShipTravel(_))); }
                    menu_sound_frame(&mut play, a);
                }
                // The dialogue player's part of `sound_update` (music_Update 0x27a688): the widgets' requests, starts and
                // stops (the Helpdesk girl's lines), then its step.
                let mut voice_vag = rt.voice_vag.take();
                menu_voice_frame(&mut play, audio.as_deref_mut(), &mut voice_vag, out.voice, out.voice_continue, out.voice_stop, frame);
                rt.voice_vag = voice_vag;
                if let Some((a, b)) = out.transition { println!("menus: frame {frame}: transition {a:#x} → {b:#x} (kind 1, 12 ticks)"); }
                if let Some(p) = out.entered { println!("menus: frame {frame}: page {p:#x} entered (kind {:#x})", menu.kind); }
                if out.quit { println!("menus: frame {frame}: Quit Game ○ (0x13d384 = 0, 0x15f5c0 = −1, 0x15f570 = 1)"); }
                // 0x15f5c0 = dest, 0x15f570 = 1: the level loop ends (crate::media_render, the travel lane's level change).
                if let Some(d) = out.level_exit { crate::media_render::request_level_exit(&mut play, d); }
                if let Some(c) = out.end_choice { crate::media_render::request_end_choice(c); }
                // MenuInput 0x298f80: a recognised code (the game state already written): the banner and the jingle.
                if let Some((e, o)) = out.code {
                    println!("menus: frame {frame}: code entry: {e:?}");
                    if let Some((msg, ticks)) = o.banner { rc_game::cinematic::banner_call(&mut play.svc.cinematic, msg, ticks); }
                    if o.jingle {
                        if let Some(a) = audio.as_deref_mut() {
                            let g = &mut play.game;
                            let listener = rc_game::audio::class_sounds::listener_of(&g.camera.out);
                            a.system().play_level_sound_at_moby(rc_game::audio::class_sounds::level_sound::SKILL_POINT, 0, None, None, &listener, &mut g.rng, g.counter);
                        }
                    }
                }
                // mode_freezeInit(3, page): the card dialog (crate::saves), opened after the menu's borrow.
                if let Some(p) = out.freeze { freeze_req = Some(p); }
                // 0x13e05a with a level exit (New Game 1, a load 0): the ship block's reset trip (rc_game::travel).
                if let Some(st) = out.story { play.svc.travel.reset_trip = st; }
                if let Some(l) = out.language {
                    // (Only the front end's Options carry the Language list; the level's own text keeps its language
                    // until the next level load, as on the PS2, where the level text is loaded once per level.)
                    apply_language(&mut play, l);
                    rt.assets.all_text = load_all_text(&crate::level_load::extracted_root(), l);
                    println!("menus: frame {frame}: language 0x15ed88 = {l}");
                }
                // PageMenuClose 0x28c6c8 (its equip requests are always set): health and bolts kept up ScaleTicks(180).
                if out.equip.is_some() { feed.calls.push(rc_game::hud::Call::ShowHealthBolts(rc_game::hud::scale_ticks(180))); }
                // PageMenuClose: the Gadgets / Weapons pages' changed slots are requested (rc_game::inventory).
                if let Some(req) = out.equip.filter(|r| r.iter().any(Option::is_some)) {
                    rc_game::inventory::apply_close_requests(sess, &req);
                    println!("menus: frame {frame}: equip requests (hand, feet, head, back) {req:?}");
                }
                if let Some(x) = out.exit {
                    // The map system back to the level (its view, zooms and a Map-o-Matic switch kept).
                    play.svc.map = std::mem::take(&mut menu.map.state);
                    // A class's EnterMenuMode set the moby loop's mode word 3 (rc_game::cinematic::enter_menu_mode).
                    if play.svc.game_mode == 3 { play.svc.game_mode = 0; }
                    if is_media_action(&x) {
                        println!("menus: frame {frame}: menu closed, post-action {x:?}: FadeToBlack(16) over the menu image first");
                        start_fade = Some(x);
                    } else {
                        if let PostAction::ShipTravel(d) = x {
                            println!("menus: frame {frame}: post-action ShipTravelTo({d})");
                            crate::travel_render::request_ship_travel(d);
                        }
                        mm.state.set(Mode::Gameplay);
                        println!("menus: frame {frame}: menu closed, mode 0 (stub calls {:?})", menu.stub_calls);
                    }
                } else {
                    menu.draw(&rt.assets, gs, &env, &mut play.game.rng, &mut rt.draws);
                }
                // The page's 3D widgets (crate::menu_models).
                widgets3d.view = menu.view;
                widgets3d.girl = menu.media.girl_view.clone();
                widgets3d.frame += 1;
                // The card back to crate::saves after the menu's tick and draw (the slot pages draw from it).
                saves_out(menu);
            }
            if let Some(x) = start_fade {
                rt.post_fade = Some((x, 0));
                // The close frame already ran its sound_update.
                post_fade_step(rt, &mut mm, &mut play, gs, sess.hp, audio.as_deref_mut(), frame, false);
            }
            // The end page's choice (crate::media_render): the time warp and challenge mode's state (crate::saves).
            if let Some(c) = crate::media_render::take_end_choice() { end_choice(rt, &mut play, gs, c); }
            if let Some(p) = freeze_req.filter(|_| mm.state.mode == Mode::Menu) {
                open_freeze(rt, &mut mm, &mut play, audio.as_deref_mut(), rc_game::menus::freeze::KIND_CARD, p, frame);
            }
        }
        Mode::Freeze => {
            // UpdateModeFreeze (the card dialog / the save notice; crate::saves).
            let input = match &rt.script {
                Some(s) => s.at(frame),
                None if *source == crate::game_camera::CameraSource::Play => pad.0,
                None => rc_game::pad::PadInput::neutral(),
            };
            let mirror = gs.options().mirror;
            play.game.pad.update(Some(&input.bytes()), mirror);
            freeze_frame(rt, &mut mm, &mut play, gs, audio.as_deref_mut(), vsync, frame);
        }
        Mode::Slideshow => {
            // UpdatePad (every mode but the movie's), then SlideshowModeUpdate / Render (crate::media_render).
            let input = match &rt.script {
                Some(s) => s.at(frame),
                None if *source == crate::game_camera::CameraSource::Play => pad.0,
                None => rc_game::pad::PadInput::neutral(),
            };
            let mirror = gs.options().mirror;
            play.game.pad.update(Some(&input.bytes()), mirror);
            let start = play.game.pad.pressed & button::START != 0;
            let done = match rt.slides.as_mut() {
                Some(sl) => {
                    sl.frame(&rt.slide_tables, start, &rt.assets, &mut rt.draws);
                    sl.done()
                }
                None => true,
            };
            // `sound_update` (SlideshowModeUpdate's tail) and the frame's samples.
            if let Some(a) = audio.as_deref_mut() { menu_sound_frame(&mut play, a); }
            if done {
                let n = rt.slides.take().map_or(0, |s| s.frames);
                mm.state.set(Mode::Gameplay);
                if play.svc.game_mode == 7 { play.svc.game_mode = 0; }
                println!("menus: frame {frame}: slideshow over after {n} frames: mode 0");
            }
        }
        Mode::Vendor => {
            let input = match &rt.script {
                Some(s) => s.at(frame),
                None if *source == crate::game_camera::CameraSource::Play => pad.0,
                None => rc_game::pad::PadInput::neutral(),
            };
            let mirror = gs.options().mirror;
            play.game.pad.update(Some(&input.bytes()), mirror);
            let inp = MenuInput::from_pad(&play.game.pad, true);
            crate::interact_render::vendor_frame(
                &mut vr, &mut play, gs, sess, &mut mm, inp, &rt.assets, frame, view.as_deref_mut(), Some(&mut feed), audio.as_deref_mut(), &mut rt.draws,
            );
        }
        _ => {}
    }
    crate::interact_render::feed_release(&play, &mut feed);
    // The "use" system after a gameplay tick: the vendor's hand-off, the prompt for the HUD.
    if mode == Mode::Gameplay && mm.state.mode == Mode::Gameplay {
        crate::interact_render::feed_idle(&mut feed);
        crate::interact_render::after_tick(&mut vr, &mut play, gs, &mut mm.state, frame, Some(&mut feed), audio.as_deref_mut(), Some(&rt.assets));
    }
    // The race's end (`mode_freezeInit(0, 0)` from the Hoverboard's race, rc_game::hero::hoverboard).
    if mm.state.mode != Mode::Freeze && std::mem::take(&mut play.game.hero.board.freeze) {
        open_freeze(rt, &mut mm, &mut play, audio.as_deref_mut(), rc_game::menus::freeze::KIND_RACE, 0, frame);
    }
    // The card monitor's failed auto-save (status 22: `mode_freezeInit(3, 0)`; crate::saves).
    if mm.state.mode != Mode::Freeze && crate::saves::with_card(|c| c.take_freeze_request()).unwrap_or(false) {
        open_freeze(rt, &mut mm, &mut play, audio.as_deref_mut(), rc_game::menus::freeze::KIND_CARD, 0, frame);
    }
    mm.state.end_frame();
    mm.loop_frame += 1;
}

/// `EnterMenuMode(kind)` 0x28bf50 (the pause triggers, a class's request, the return page): the page menu's entry, the
/// map system joined, `snd_PauseAllSoundsInGroup(0x1d)` and `music_Pause(0)`, mode 3, the snapshot of this frame.
fn open_menu(rt: &mut MenuRt, mm: &mut MenuMode, play: &mut Play, gs: &rc_game::game_state::GameState, audio: Option<&mut crate::audio_out::AudioOut>, k: i32, frame: u64) {
    let counter = play.game.counter;
    let Some(menu) = rt.menu.as_mut() else { return };
    menu.enter(k, gs);
    // The map system joins the page menu while it is open (rc_game::menus::pause::map_page).
    menu.map.state = std::mem::take(&mut play.svc.map);
    menu.map.loader = rc_game::menus::pause::map_page::Loader(Some(std::sync::Arc::new(crate::gameplay::map_file)));
    let h = &play.game.hero;
    menu.map.hero = rc_game::menus::pause::map_page::HeroMark { pos: [h.pos[0].to_f32(), h.pos[1].to_f32(), h.pos[2].to_f32()], yaw: h.rot[2].to_f32(), group: h.group };
    menu.map.mirror = gs.global.cheats_active[rc_game::cheats::slot::MIRROR] != 0;
    // The hook table 0x179638 (the map markers' mobys on this level): slot → position and angle.
    menu.map.hooks = play.svc.interact.talk_slots.iter().filter_map(|(&id, &k)| {
        let m = play.game.mobys.mobys.get(id)?;
        Some((k, (m.position[0], m.position[1], m.rotation[2])))
    }).collect();
    // EnterMenuMode 0x28bf50: snd_PauseAllSoundsInGroup(0x1d), music_Pause(0).
    if let Some(a) = audio { a.system().menu_open(); }
    mm.state.set(Mode::Menu);
    rt.snapshot_request = true;
    rt.snapshot_ready = false;
    println!("menus: frame {frame}: enter mode 3, kind {k} (game tick frozen at {counter})");
}

/// `InLevelFrameUpdate` 0x2aba68's first test: a return page (0x1ba260, set by a movie / scene / slideshow post-action)
/// in mode 0 → `EnterMenuMode(0)` before the tick (no tick that frame); the first tick reopens that page and restores the
/// replay mode 0x15eed8 from 0x1ba2a0.
#[allow(clippy::too_many_arguments)]
fn return_page_reopen(
    rt: Option<ResMut<MenuRt>>,
    mut mm: ResMut<MenuMode>,
    play: Option<ResMut<Play>>,
    gs: Option<Res<Persistent>>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
    movies: Option<Res<crate::movie_render::MovieState>>,
) {
    let (Some(mut rt), Some(mut play), Some(gs)) = (rt, play, gs) else { return };
    if mm.state.mode != Mode::Gameplay || rt.post_fade.is_some() || movies.is_some_and(|m| m.busy()) { return; }
    if !rt.menu.as_ref().is_some_and(|m| m.return_page != 0) { return; }
    // A scene the post-action asked for is still to start (crate::scene_render takes it this frame).
    if play.svc.cinematic.requests.iter().any(|r| matches!(r, rc_game::cinematic::EngineRequest::StartScene { .. })) { return; }
    mm.replay = rt.replay_saved;
    let frame = mm.loop_frame;
    open_menu(&mut rt, &mut mm, &mut play, &gs.0, audio.as_deref_mut(), 0, frame);
}

/// The post-actions that fade and leave for a movie, a scene or the slideshow (0x28c990: 3, 4, 5, 6, 7).
fn is_media_action(x: &PostAction) -> bool { matches!(x, PostAction::Movie { .. } | PostAction::Scene(_) | PostAction::Slideshow) }

/// Mode 7 from a request (`EnterSlideshowMode` 0x2ad558): the mode word 7, the runtime (crate::media_render).
fn start_slideshow(rt: &mut MenuRt, mm: &mut MenuMode, play: &mut Play, black_entry: bool, frame: u64) {
    rt.slides = Some(crate::media_render::Slides::start(black_entry));
    mm.state.set(Mode::Slideshow);
    play.svc.game_mode = 7;
    println!("menus: frame {frame}: EnterSlideshowMode: mode 7 (FadeToBlack(8){})", if black_entry { " over black" } else { "" });
}

/// One step of the close's `FadeToBlack(ticks(16))` over the menu image (0x28c990); after the last step, the action:
/// the mode word 0 (0x15f5c4 = 0), the replay mode (3: 2; 4 / 5 / 6: 1; the old value into 0x1ba2a0), the movie
/// (`DialogStreamUpdate` / `PlayMovieB` / `PlayMovieC`), the scene (`DialogStreamStart`) or the slideshow, then
/// `snd_ContinueAllSoundsInGroup(0x1d)` and `music_Unpause`.
#[allow(clippy::too_many_arguments)]
fn post_fade_step(rt: &mut MenuRt, mm: &mut MenuMode, play: &mut Play, _gs: &mut rc_game::game_state::GameState, hp: i32, audio: Option<&mut crate::audio_out::AudioOut>, frame: u64, sound: bool) {
    let Some((x, k)) = rt.post_fade else { return };
    crate::media_render::menu_fade_draws(k, &mut rt.draws);
    let mut audio = audio;
    if k + 1 < rc_game::movie_player::MENU_FADE {
        rt.post_fade = Some((x, k + 1));
        if let Some(a) = audio.as_deref_mut().filter(|_| sound) { menu_sound_frame(play, a); }
        return;
    }
    rt.post_fade = None;
    play.svc.game_mode = 0;
    rt.replay_saved = mm.replay;
    use crate::movie_render::{request, MovieRequest};
    match x {
        PostAction::Movie { kind: 3, arg } => {
            mm.replay = 2;
            // DialogStreamUpdate: with no health the death fade instead (the menu does not open at 0 health).
            if hp != 0 { request(MovieRequest::in_level(arg as i32, None).after_menu_fade()); }
        }
        PostAction::Movie { kind: 4, arg } => {
            mm.replay = 1;
            request(MovieRequest::movie_b(arg as i32).after_menu_fade());
        }
        PostAction::Movie { arg, .. } => {
            mm.replay = 1;
            request(MovieRequest::movie_c(arg as i32).after_menu_fade());
        }
        PostAction::Scene(arg) => {
            mm.replay = 1;
            // DialogStreamStart: the help box closed, 0x15f3fc = 1 (the scene ramps its own copy).
            play.svc.help.kill();
            play.svc.cinematic.fade = 0.0;
            play.svc.cinematic.requests.push(rc_game::cinematic::EngineRequest::StartScene { scene: arg as usize, arrival: false });
        }
        _ => {}
    }
    let movie = matches!(x, PostAction::Movie { .. });
    if let Some(a) = audio {
        // StartPssMovie's sound_StopAllSounds / music_Stop come before the close's unpause.
        if movie { a.system().movie_stop(); }
        a.system().menu_close(true);
        if sound { menu_sound_frame(play, a); }
    }
    if x == PostAction::Slideshow {
        start_slideshow(rt, mm, play, true, frame);
    } else {
        mm.state.set(Mode::Gameplay);
    }
    println!("menus: frame {frame}: post-action {x:?} after FadeToBlack(16) (replay mode 0x15eed8 = {})", mm.replay);
}

/// 0x15ed88 = `lang`: the runtime language every later load reads (crate::hud_render::language), the page menu's
/// language (the Controls / Items / Epilogue pictures), the help voice bank (`help_audio[lang·150 + n]`).
fn apply_language(play: &mut Play, lang: u32) {
    crate::hud_render::set_language(lang);
    play.svc.help.text.lang = lang;
}

/// The global `all_text` lump's block of language `lang` (`+4·lang` offsets; docs/plan/hud_text.md §5), empty when absent.
fn load_all_text(root: &std::path::Path, lang: u32) -> Vec<rc_formats::strings::Message> {
    let Ok(b) = crate::disc_source::read(root, "global/all_text.bin") else { return Vec::new() };
    let bytes = if rc_formats::wad::is_wad(&b) { rc_formats::wad::decompress(&b).unwrap_or_default() } else { b.to_vec() };
    let off = bytes.get(4 * lang as usize..4 * lang as usize + 4).map_or(0, |w| u32::from_le_bytes(w.try_into().unwrap()) as usize);
    bytes.get(off..).and_then(|blk| rc_formats::strings::parse_text_block(blk).ok()).unwrap_or_default()
}

/// One page-menu frame's audio: `sound_update` (`SceneController` 0x28c990 runs it after `MobyUpdateLoop`; the level's
/// sound instances are not updated), then the frame's samples.
fn menu_sound_frame(play: &mut Play, audio: &mut crate::audio_out::AudioOut) {
    let game = &mut play.game;
    let h = game.hero.pos;
    let listener = rc_game::audio::class_sounds::listener_of(&game.camera.out);
    let input = rc_game::audio::FrameInput { listener, hero_pos: [h[0].to_f32(), h[1].to_f32(), h[2].to_f32()] };
    let table = &game.mobys;
    let (sys, buf) = audio.parts();
    buf.clear();
    sys.sound_update_with(&input, game.counter as u32, &mut game.rng, &|id| rc_game::audio::class_sounds::owner_position(table, id));
    sys.render(rc_game::audio::SAMPLES_PER_FRAME, buf);
    audio.push_frame();
}

/// The stream a dialogue id plays (`rc_game::help::dialogue_stream`).
fn dialogue_stream(id: i32, lang: u32, level: u32) -> Option<String> { rc_game::help::dialogue_stream(id, lang, level) }

/// One menu frame of the dialogue player 0x151720 (`rc_game::help::Voice`): a widget's request (0x1516ec), its
/// `continue_audio_stream_if_ready` (the line starts at full volume: `fun_00215440` ignores the HelpDesk voice option)
/// and its stop (0x15172a ∉ {6, 7} → 5), then `Help::voice_frame` (music_Update's dialogue part runs in every
/// `sound_update`, the page menu's included) with its load / play / stop carried out.
#[allow(clippy::too_many_arguments)]
fn menu_voice_frame(play: &mut Play, mut audio: Option<&mut crate::audio_out::AudioOut>, vag: &mut Option<std::sync::Arc<[u8]>>, request: Option<i32>, cont: bool, stop: bool, frame: u64) {
    use rc_game::help::VoiceCmd;
    let help = &mut play.svc.help;
    if let Some(id) = request { help.voice.request = id; }
    let mut cmds = Vec::new();
    if cont && help.voice.busy && help.voice.state == 3 {
        help.voice.state = 4;
        cmds.push(VoiceCmd::Play { audible: true });
    }
    if stop && help.voice.state.wrapping_sub(6) > 1 { help.voice.state = 5; }
    help.voice_frame();
    let (lang, level) = (help.text.lang, play.svc.level);
    cmds.extend(std::mem::take(&mut help.out.voice));
    for cmd in cmds {
        match cmd {
            VoiceCmd::Load { id } => {
                let root = crate::level_load::extracted_root();
                let b = dialogue_stream(id, lang, level).and_then(|f| crate::disc_source::read(&root, &f).ok());
                let len = b.as_deref().and_then(rc_game::help::vag_ticks);
                println!("menus: frame {frame}: dialogue line {id}: {}", len.map_or("missing".to_string(), |t| format!("{t} ticks")));
                *vag = b.filter(|_| len.is_some()).map(std::sync::Arc::from);
                play.svc.help.voice_loaded(len);
            }
            VoiceCmd::Play { audible } => {
                if let (Some(v), Some(a), true) = (vag.clone(), audio.as_deref_mut(), audible) {
                    a.system().scene_command(rc_game::audio::scene::SceneAudioCmd::Speech { vag: v });
                }
            }
            VoiceCmd::Stop => {
                if vag.take().is_some() {
                    if let Some(a) = audio.as_deref_mut() { a.system().scene_command(rc_game::audio::scene::SceneAudioCmd::StopSpeech); }
                }
            }
        }
    }
}

/// The frame's page-menu sounds (`rc_game::menus::MenuSound::event`: class 0x472's sound n, flags 0x11).
pub fn play_menu_sounds(play: &mut Play, audio: &mut crate::audio_out::AudioOut, sounds: &[MenuSound]) {
    if sounds.is_empty() { return; }
    let listener = rc_game::audio::class_sounds::listener_of(&play.game.camera.out);
    for &s in sounds {
        audio.system().play_class_sound(&s.event(play.game.counter), None, &listener, &mut play.game.rng);
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// The memory-card pages, the card dialog (mode 4) and the front end (crate::saves).

/// The card dialog was opened from the page menu (its render draws the menu under it).
fn menu_under_freeze(rt: &MenuRt) -> bool { rt.render_mode == Mode::Freeze && rt.freeze.is_some_and(|f| f.prev == 3) }

/// Before a page-menu tick: the card and the save inputs moved into the menu (the clock, the landmark ranges, the new
/// game's template, the ending buffer).
fn saves_in(menu: &mut PageMenu, play: &Play) {
    if let Some(c) = crate::saves::take_card() { menu.saves.card = c; }
    menu.saves.clock = crate::saves::clock_now();
    menu.saves.landmark_base = play.svc.interact.tables.base.clone();
    if menu.saves.template.is_none() { menu.saves.template = menu.saves.card.lump.as_ref().map(|l| l.template.clone()); }
    menu.saves.ending = crate::saves::take_ending();
}

/// After the tick: the card back to crate::saves.
fn saves_out(menu: &mut PageMenu) { crate::saves::put_card(std::mem::replace(&mut menu.saves.card, rc_game::memcard::MemCard::absent())); }

/// `mode_freezeInit(kind, arg)` from the current mode.
fn open_freeze(rt: &mut MenuRt, mm: &mut MenuMode, play: &mut Play, audio: Option<&mut crate::audio_out::AudioOut>, kind: i32, arg: u32, frame: u64) {
    let (f, o) = rc_game::menus::freeze::Freeze::init(kind, arg, mm.state.mode.raw());
    if o.pause_sounds {
        if let Some(a) = audio { a.system().menu_open(); }
    }
    if let Some(m) = o.log { play.svc.help.log_append(m); }
    println!("menus: frame {frame}: mode_freezeInit({kind}, {arg:#x}) from mode {}", f.prev);
    rt.freeze = Some(f);
    mm.state.set(Mode::Freeze);
}

/// The new game without a card (`load_and_initialize_level_chunk`, `FUN_002a29a0(0)`, 0x13e05a = 1).
fn new_game_no_card(play: &mut Play, gs: &mut rc_game::game_state::GameState) {
    let t = crate::saves::with_card(|c| c.lump.as_ref().map(|l| l.template.clone())).flatten();
    rc_game::menus::pause::saves::reset_game(gs, t.as_deref());
    play.svc.travel.reset_trip = true;
    crate::media_render::request_level_exit(play, 0);
}

/// One frame of mode 4 (`UpdateModeFreeze` + its render: the page menu under a kind-3 dialog opened from it, the black
/// 0x40, `DrawDialogText`).
fn freeze_frame(rt: &mut MenuRt, mm: &mut MenuMode, play: &mut Play, gs: &mut rc_game::game_state::GameState, audio: Option<&mut crate::audio_out::AudioOut>, vsync: u32, frame: u64) {
    let Some(mut f) = rt.freeze.take() else {
        mm.state.set(Mode::Gameplay);
        return;
    };
    let (pressed, level) = (play.game.pad.pressed, gs.global.level);
    // Kind 0's inputs (rc_game::menus::freeze::FreezeCtx): the level and the hero's state ticks 0x13f4ec; the race's own
    // values (stage, place, time, score, sound, records) come from the race classes (not ported: G-UI-019).
    f.ctx.level = level;
    f.ctx.hero_state_ticks = play.game.hero.f4ec;
    // The Hoverboard's race (rc_game::hero::hoverboard): the stage 0x13fbea, place, time, score, the HUD handle 0x13fbd0,
    // the records.
    {
        let b = &play.game.hero.board;
        let r = &play.svc.board.records;
        (f.ctx.race_stage, f.ctx.place, f.ctx.time, f.ctx.score, f.ctx.sound) = (b.lap, b.place, b.race_ticks, b.score, b.hud);
        (f.ctx.best_time, f.ctx.best_score) = (r.best_time, r.best_score);
    }
    // The kinds without the card (0, 1, 2, 4, 6) run without one set up too.
    let out = crate::saves::with_card(|c| f.update(pressed, c, level)).unwrap_or_else(|| f.update(pressed, &mut rc_game::memcard::MemCard::absent(), level));
    if let Some(t) = out.target {
        if let Some(m) = rt.menu.as_mut() { m.target = t; }
    }
    freeze_effects(play, &out, frame);
    if out.new_game { new_game_no_card(play, gs); }
    if out.save { crate::saves::memcard_save(play, gs, false, -1); }
    if out.resume_sounds {
        if let Some(a) = audio { a.system().menu_close(true); }
    }
    if f.prev == 3 {
        if let Some(m) = rt.menu.as_mut() {
            saves_in(m, play);
            m.draw(&rt.assets, gs, &MenuEnv { vsync, b13f4: play.game.hero.mode, pal: false }, &mut play.game.rng, &mut rt.draws);
            saves_out(m);
        }
    }
    rt.draws.push(MenuDraw::Darken { alpha: rc_game::menus::pause::DARKEN_FREEZE });
    if crate::saves::with_card(|c| f.draw(&rt.assets, c, vsync, &mut rt.draws)).is_none() {
        f.draw(&rt.assets, &rc_game::memcard::MemCard::absent(), vsync, &mut rt.draws);
    }
    match out.mode {
        Some(m) => {
            mm.state.set(Mode::from_raw(m).unwrap_or(Mode::Gameplay));
            println!("menus: frame {frame}: the dialog closed: mode {m}");
        }
        None => rt.freeze = Some(f),
    }
}

/// The kinds 0 / 1 / 2 / 4 / 6 effects of an `UpdateModeFreeze` frame (rc_game::menus::freeze::FreezeOut).
fn freeze_effects(play: &mut Play, out: &rc_game::menus::freeze::FreezeOut, frame: u64) {
    use rc_game::moby_update::services::{HeroCall, HeroFields, HeroPose};
    // 0x15f608 = 2: the occlusion's octant fallback this frame (crate::visibomb_view::occlusion_fallback).
    if out.all_visible { play.svc.occlusion_fallback = Some((play.game.counter.wrapping_sub(1), 2)); }
    let teleport_entry = |play: &mut Play, f: &mut HeroFields| {
        // HeroTeleport(0x141050, 0x141060, 0, 1): the pose saved where he got in (crate::hero::bodies::Bodies::entry_pose).
        if let Some((pos, euler)) = play.game.hero.bodies.entry_pose {
            f.clear_motion();
            f.pose = Some(HeroPose { pos, yaw: euler[2], target_yaw: euler[2] });
            f.call(HeroCall::SetState { id: 0, play: true });
            play.svc.cinematic.calls.push(rc_game::cinematic::CinematicCall::CameraResetBehindHero);
        }
    };
    if let Some(q) = out.race_quit {
        let mut f = HeroFields::of(&play.game.hero);
        teleport_entry(play, &mut f);
        play.svc.hero_writes = Some((play.game.counter, f));
        // The HUD handle 0x13fbd0 stopped (the freeze set its copy to −1) and the finished counter (0x15ee38 / 0x15ee3c).
        if q.stop_sound.is_some() { play.game.hero.board.hud = -1; }
        if let Some(k) = q.count { play.svc.board.records.finished[k] += 1; }
        println!("menus: frame {frame}: Quit Race: HUD handle {:?} stopped, finished counter {:?}, update_resource_counter, HeroTeleport(entry pose)", q.stop_sound, q.count);
    }
    if out.race_rewind {
        // The race stage 0x13fbea − 1 and the race moby 0x13fbe0 +0xbc = 3 (Rilgar's race girl restarts the race).
        let b = &mut play.game.hero.board;
        b.lap -= 1;
        if let Some(m) = b.host.and_then(|h| play.game.mobys.mobys.get_mut(h)) { m.cmd = 3; }
        println!("menus: frame {frame}: Quit Race? no: the race stage − 1, the race moby +0xbc = 3");
    }
    if out.vehicle_quit {
        play.svc.vehicle.request_quit();
        println!("menus: frame {frame}: Quit? yes: 0x14095f |= 1");
    }
    if out.leave_body {
        // The body moby 0x1413d0 hidden, collision off, mode |= 1; the body left (FUN_00231450) and the entry pose.
        if let Some(id) = play.game.hero.bodies.moby {
            if let Some(m) = play.game.mobys.mobys.get_mut(id) {
                m.visible = 0;
                m.has_collision = false;
                m.mode |= 1;
            }
        }
        let mut f = HeroFields::of(&play.game.hero);
        f.call(HeroCall::LeaveBody { game_mode: rc_game::menus::mode::Mode::Freeze.raw() });
        teleport_entry(play, &mut f);
        play.svc.hero_writes = Some((play.game.counter, f));
        println!("menus: frame {frame}: Giant Clank's Quit? yes: the body left, HeroTeleport(entry pose)");
    }
    if out.fade_to_black.is_some() || out.video_mode.is_some() {
        println!("menus: frame {frame}: the 60 Hz test: FadeToBlack {:?}, 0x16040c = {:?} (PAL only: no video mode switch on NTSC)", out.fade_to_black, out.video_mode);
    }
}

/// The end page's choice (`media::EndChoice`): the time warp, or challenge mode without a card.
fn end_choice(rt: &mut MenuRt, play: &mut Play, gs: &mut rc_game::game_state::GameState, c: rc_game::menus::pause::media::EndChoice) {
    use rc_game::menus::pause::media::EndChoice;
    match c {
        EndChoice::Timewarp => crate::saves::time_warp(play, gs),
        EndChoice::Challenge => {
            let keep = rt.menu.as_ref().map(|m| m.saves.challenge_items.clone()).unwrap_or_default();
            let t = crate::saves::with_card(|c| c.lump.as_ref().map(|l| l.template.clone())).flatten();
            let clock = crate::saves::clock_now();
            crate::saves::with_card(|card| card.challenge_save(gs, -1, clock, &keep, |g| rc_game::menus::pause::saves::reset_game(g, t.as_deref())));
            play.svc.travel.reset_trip = true;
            println!("saves: challenge mode without a card: times completed {}", gs.global.completes);
        }
    }
}

/// One main-loop frame of the front end (crate::saves; `rc_game::frontend`).
#[allow(clippy::too_many_arguments)]
fn front_end_frame(
    rt: &mut MenuRt,
    mm: &mut MenuMode,
    play: &mut Play,
    gs: &mut rc_game::game_state::GameState,
    fer: &mut crate::saves::FrontEndRt,
    pad: &PadFrame,
    source: &crate::game_camera::CameraSource,
    movie_busy: bool,
    mut audio: Option<&mut crate::audio_out::AudioOut>,
) {
    let frame = mm.loop_frame;
    let vsync = frame as u32;
    rt.render_mode = Mode::Menu;
    rt.draws.clear();
    if movie_busy { return; }
    if mm.state.mode != Mode::Menu { mm.state.set(Mode::Menu); }
    let input = match &rt.script {
        Some(s) => s.at(frame),
        None if *source == crate::game_camera::CameraSource::Play => pad.0,
        None => rc_game::pad::PadInput::neutral(),
    };
    let mirror = gs.options().mirror;
    play.game.pad.update(Some(&input.bytes()), mirror);
    let pressed = play.game.pad.pressed;
    let inp = MenuInput::from_pad(&play.game.pad, true);
    let env = MenuEnv { vsync, b13f4: play.game.hero.mode, pal: false };
    // The level under the front end stays silent (`snd_PauseAllSoundsInGroup(0x1d)`, `music_Pause`) [L: the boot has its
    // own sound bank and title music, G-SAV-012].
    if !fer.paused {
        if let Some(a) = audio.as_deref_mut() { a.system().menu_open(); }
        fer.paused = true;
    }
    let Some(fe) = fer.fe.as_mut() else { return };
    // `transition_do_transition` 0x1eb798, every vsync: `queue_dma_transfer(0x15ed88)` makes `all_text`'s block of the
    // language the message table (the front end's text; `transition_load_wad` loaded all eight blocks).
    if rt.front_text != Some(fe.lang) {
        let t = load_all_text(&crate::level_load::extracted_root(), fe.lang);
        if !t.is_empty() { rt.assets.hud.messages = t.clone(); }
        rt.assets.all_text = t;
        rt.front_text = Some(fe.lang);
    }
    let out = match fe.phase {
        rc_game::frontend::Phase::CardCheck { .. } => {
            let check = crate::saves::with_card(|c| {
                let d = c.dir.clone();
                c.fs.boot_check(&d)
            })
            .unwrap_or(0);
            fe.card_frame(check, pressed)
        }
        _ => fe.frame(pressed, movie_busy),
    };
    // `RC_SKIP_LOGOS=1` (dev switch): the logos movie is not played; the boot goes straight on to the still and the title.
    if out.logos && std::env::var("RC_SKIP_LOGOS").is_ok_and(|v| v.trim() == "1") {
        println!("menus: frame {frame}: front end: the logos skipped (RC_SKIP_LOGOS=1)");
    } else if out.logos {
        // startlevel: the logos PSS `mpegs[0]` (NTSC), language 0, never skipped, back to the front end.
        use crate::movie_render::{MovieExit, MovieRequest};
        crate::movie_render::request(MovieRequest { file: 0, movie: 0, language: Some(0), replay: Some(-1), exit: MovieExit::FrontEnd, ..MovieRequest::in_level(0, None) });
        println!("menus: frame {frame}: front end: the logos (mpegs[0])");
    }
    if let Some(i) = out.attract {
        crate::movie_render::play_attract(i);
        println!("menus: frame {frame}: front end: attract movie {i} (mpegs[{}])", 80 + i);
    }
    if out.open_menu {
        if let Some(m) = rt.menu.as_mut() { m.enter(0x2d, gs); }
        println!("menus: frame {frame}: front end: the main menu (kind 0x2d)");
    }
    let mut exit = None;
    match fe.mode {
        3 => {
            if let Some(menu) = rt.menu.as_mut() {
                saves_in(menu, play);
                menu.saves.card.front_end = true;
                let o = menu.tick(&inp, gs, &env);
                if let Some(a) = audio.as_deref_mut() {
                    play_menu_sounds(play, a, &o.sounds);
                    menu_sound_frame(play, a);
                }
                if let Some(st) = o.story { play.svc.travel.reset_trip = st; }
                if let Some(l) = o.language {
                    fe.set_language(l);
                    apply_language(play, l);
                    println!("menus: frame {frame}: front end: language 0x15ed88 = {l} (the text, PRESS START and every later load follow)");
                }
                if let Some(p) = o.freeze {
                    let (f, _) = rc_game::menus::freeze::Freeze::init(rc_game::menus::freeze::KIND_CARD, p, 3);
                    fer.freeze = Some(f);
                    fe.mode = 4;
                }
                if let Some(l) = o.level_exit { exit = Some(l); }
                if o.exit.is_some() {
                    fe.menu_closed();
                    println!("menus: frame {frame}: front end: the menu closed, the title");
                } else {
                    menu.draw(&rt.assets, gs, &env, &mut play.game.rng, &mut rt.draws);
                }
                saves_out(menu);
            }
        }
        4 => {
            if let Some(f) = fer.freeze.as_mut() {
                let level = gs.global.level;
                let o = crate::saves::with_card(|c| f.update(pressed, c, level)).unwrap_or_default();
                if let Some(t) = o.target {
                    if let Some(m) = rt.menu.as_mut() { m.target = t; }
                }
                if o.new_game {
                    let t = crate::saves::with_card(|c| c.lump.as_ref().map(|l| l.template.clone())).flatten();
                    rc_game::menus::pause::saves::reset_game(gs, t.as_deref());
                    play.svc.travel.reset_trip = true;
                    exit = Some(0);
                }
                if let Some(m) = rt.menu.as_mut() {
                    saves_in(m, play);
                    m.draw(&rt.assets, gs, &env, &mut play.game.rng, &mut rt.draws);
                    saves_out(m);
                }
                if let Some(m) = o.mode { fe.mode = m; }
            }
        }
        _ => {}
    }
    fe.draw(&mut rt.draws);
    fe.draw_fade(&mut rt.draws);
    if fe.mode == 4 {
        if let Some(f) = fer.freeze.as_ref() { crate::saves::with_card(|c| f.draw(&rt.assets, c, vsync, &mut rt.draws)); }
    }
    if let Some(l) = exit {
        fe.exit(l);
        // The front end's loop ends straight into `DoSpaceTransition` (`0x15f5c0 = l; 0x15f570 = 1`): no tick of the level
        // loaded behind the front end runs in between (a gameplay tick here fired that level's first-arrival scene). The
        // request goes to the travel lane directly (the cinematic inbox is only read by a gameplay tick), and the mode is
        // the transition's mode 6 until its `Begin` step takes over.
        crate::travel_render::request_leave(l);
        fer.fe = None;
        fer.freeze = None;
        crate::saves::set_front_end_active(false);
        crate::saves::with_card(|c| c.front_end = false);
        if let Some(a) = audio { a.system().menu_close(true); }
        mm.state.set(Mode::Ship);
        println!("menus: frame {frame}: front end over: level {l} (the travel lane's level change loads it)");
    }
}


/// The frame's draws → HUD primitives; the snapshot node and its capture.
#[allow(clippy::too_many_arguments)]
fn build_prims(
    rt: Option<ResMut<MenuRt>>,
    level: Res<crate::Level>,
    mut hook: ResMut<Hud2dHook>,
    mut snap: Query<&mut Visibility, With<SnapshotNode>>,
    cams: Query<&RenderTarget, With<crate::fly_cam::FlyCam>>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut request: ResMut<SnapshotRequest>,
    (mut pictures, mut images): (Option<ResMut<crate::hud_images::HudImages>>, ResMut<Assets<Image>>),
) {
    let Some(mut rt) = rt else { return };
    let Some(lh) = level.0.hud.as_ref() else { return };
    let mut h = Hud2d::default();
    h.frame_sizes = rt.assets.hud.frame_sizes.clone();
    let mut st = TextState::default();
    let mut statics: [Vec<Prim>; 3] = Default::default();
    if let Some(p) = pictures.as_deref_mut() { p.frame += 1; }
    let mut resolve = |src: &rc_game::menus::ImageSrc| pictures.as_deref_mut().and_then(|p| p.resolve(src, &mut images));
    let snapshot = convert(&rt.draws, &mut h, &mut statics, &mut st, &lh.glyphs, &mut resolve);
    // The pages stay clipped to the 512×416 screen in a 16:9 frame (`Prim::boxed`): their unused panels lie off it.
    for p in h.prims.iter_mut().chain(statics.iter_mut().flatten()) { p.boxed = true; }
    hook.prims = h.prims;
    hook.statics = statics;
    hook.replace_hud = matches!(rt.render_mode, Mode::Menu | Mode::Slideshow) || menu_under_freeze(&rt);
    // The vendor (mode 5) runs `HudUpdate(1)` every frame: the HUD ticks and draws over its screens.
    hook.freeze = !matches!(rt.render_mode, Mode::Gameplay | Mode::Vendor);
    let show = snapshot && rt.snapshot_ready;
    for mut v in &mut snap { *v = if show { Visibility::Visible } else { Visibility::Hidden }; }
    if std::mem::take(&mut rt.snapshot_request) {
        // This frame's render (still a gameplay frame) is copied on the GPU at the end of its render graph,
        // before the first menu frame renders: the snapshot is ready from that frame on.
        let target = cams.iter().next().cloned().unwrap_or_default().normalize(primary.iter().next());
        match target {
            Some(t) => {
                request.0 = Some((t, rt.snapshot.id()));
                rt.snapshot_ready = true;
            }
            None => eprintln!("menus: the main camera has no render target: no snapshot"),
        }
    }
}

/// The menu layer each frame: its cameras and node follow the mode; the frame mobys' visibility, palettes
/// and records follow `PageMenu::frames` (after this frame's menu tick).
#[allow(clippy::too_many_arguments)]
fn menu_layer(
    mut commands: Commands,
    rt: Option<ResMut<MenuRt>>,
    frame: Res<crate::display::GameFrame>,
    main: Query<&Transform, With<crate::fly_cam::FlyCam>>,
    mut transforms: Query<&mut Transform, Without<crate::fly_cam::FlyCam>>,
    mut cams: Query<&mut Camera>,
    mut vis: Query<&mut Visibility>,
    spawn_hidden: Query<(), With<crate::moby_spawn::SpawnHidden>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    (mut targets, mut image_nodes, hud_nodes): (Query<&mut RenderTarget>, Query<&mut ImageNode>, Query<Entity, With<crate::hud_render::HudCompositeNode>>),
) {
    let Some(mut rt) = rt else { return };
    let rt = &mut *rt;
    // Mode 3, or the card dialog opened from it (its render keeps the page menu: crate::saves).
    let active = rt.render_mode == Mode::Menu || menu_under_freeze(rt);
    let Some(main_t) = main.iter().next().copied() else { return };
    let layer = &mut rt.layer;
    // The layer image and the snapshot follow the main target: the game frame (crate::display).
    if let Some(size) = Some(frame.size).filter(|s| s.x > 0 && s.y > 0) {
        // Fresh images of the new size (crate::display::fresh_target): the cameras and nodes showing them move along.
        // A snapshot of the old size is taken again (an aspect / resolution change from Port Options): the next frame's
        // render is copied with the layer and the HUD hidden, as the gameplay frame the menu was entered on.
        if images.get(&layer.image).is_some_and(|i| i.size() != size) {
            rt.snapshot_ready = false;
            // Not in the front end (crate::saves): its layer is opaque over the hidden world and shows no snapshot.
            if active && !crate::saves::front_end_active() {
                rt.snapshot_request = true;
                rt.resnap = 2;
            }
        }
        for h in [&mut layer.image, &mut rt.snapshot] {
            let same = images.get(&*h).is_some_and(|i| i.texture_descriptor.size.width == size.x && i.texture_descriptor.size.height == size.y);
            if same { continue; }
            let Some(new) = crate::display::fresh_target(&mut images, h, size) else { continue };
            for mut t in &mut targets {
                if matches!(&*t, RenderTarget::Image(i) if i.handle.id() == h.id()) { *t = RenderTarget::Image(new.clone().into()); }
            }
            for mut n in &mut image_nodes { if n.image.id() == h.id() { n.image = new.clone(); } }
            *h = new;
        }
    }
    if let Ok(mut c) = cams.get_mut(layer.cam2d) {
        if c.is_active != active { c.is_active = active; }
        // The front end (crate::saves): the layer is opaque black (the level's world stays hidden behind it), or clear
        // over the title world when it is drawn (crate::title_world).
        let clear = if crate::saves::title_world_shown() && crate::saves::front_end_active() {
            Color::NONE
        } else if crate::saves::front_end_active() {
            Color::BLACK
        } else {
            Color::srgba(0.0, 0.0, 0.0, DARKEN as f32 / 128.0)
        };
        if !matches!(c.clear_color, ClearColorConfig::Custom(k) if k == clear) { c.clear_color = ClearColorConfig::Custom(clear); }
    }
    let resnap = rt.resnap > 0;
    rt.resnap = rt.resnap.saturating_sub(1);
    if let Ok(mut v) = vis.get_mut(layer.node) {
        let want = if active && !resnap { Visibility::Visible } else { Visibility::Hidden };
        if *v != want { *v = want; }
    }
    for e in &hud_nodes {
        let Ok(mut v) = vis.get_mut(e) else { continue };
        let want = if resnap { Visibility::Hidden } else { Visibility::Inherited };
        if *v != want { *v = want; }
    }
    // The 3D camera exists only in mode 3 (other systems take "the" Camera3d; it carries the main transform).
    match (active && layer.frame.is_some(), layer.cam3d) {
        (true, None) => {
            layer.cam3d = Some(
                commands
                    .spawn((
                        Camera3d::default(),
                        Camera { order: -2, clear_color: ClearColorConfig::None, ..default() },
                        RenderTarget::Image(layer.image.clone().into()),
                        crate::game_camera::game_projection(),
                        Msaa::Off,
                        Tonemapping::None,
                        DebandDither::Disabled,
                        RenderLayers::layer(MENU_3D_LAYER),
                        main_t,
                        MenuLayerCam,
                        // The frame mobys frame the 512×416 screen: the frame's 4:3 box (crate::display).
                        crate::display::BoxedView,
                        Name::new("menu layer 3D camera (frame mobys)"),
                    ))
                    .id(),
            );
        }
        (true, Some(e)) => {
            if let Ok(mut t) = transforms.get_mut(e) {
                if *t != main_t { *t = main_t; }
            }
        }
        (false, Some(e)) => {
            commands.entity(e).despawn();
            layer.cam3d = None;
        }
        (false, None) => {}
    }
    let Some(fr) = layer.frame.as_mut() else { return };
    let menu = rt.menu.as_ref();
    let frames = menu.and_then(|m| m.frames.as_ref()).and_then(|f| f.slots.as_ref().map(|s| (f, s)));
    let goodies = menu.is_some_and(|m| m.goodies);
    let fold = main_t.to_matrix() * fr.camera.inverse() * fr.model;
    let mut palette: Option<Vec<u8>> = None;
    let mut records: Option<Vec<u8>> = None;
    for i in 0..frame::SLOTS {
        let show = active && frames.is_some() && (i != 6 || goodies);
        let want = if show { Visibility::Inherited } else { Visibility::Hidden };
        for &e in &fr.entities[i] {
            // The gameplay setup hides every moby entity tagged with Ratchet's instance index (MeshTag 0 here)
            // by `SpawnHidden`, which `moby_spawn` enforces each frame: these are not his.
            if spawn_hidden.contains(e) { commands.entity(e).remove::<crate::moby_spawn::SpawnHidden>(); }
            if let Ok(mut v) = vis.get_mut(e) {
                if *v != want { *v = want; }
            }
        }
        let (true, Some((f, slots))) = (show, frames) else { continue };
        let pal = palette.get_or_insert_with(|| buffers.get(&fr.extra.palette).and_then(|b| b.data.clone()).unwrap_or_default());
        let rec = records.get_or_insert_with(|| buffers.get(&fr.extra.instances).and_then(|b| b.data.clone()).unwrap_or_default());
        // MobyAnimEval of the slot's state (the frame mobys have no pose layers and no blends).
        let m = rc_formats::moby_anim::evaluate_with_snapshot(&f.class.anim, &slots[i].state, None);
        let base = i * fr.slots as usize;
        let bytes: Vec<u8> = m.iter().take(fr.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).collect();
        if let Some(dst) = pal.get_mut(base * 64..base * 64 + bytes.len()) { dst.copy_from_slice(&bytes); }
        let r = moby_render::extra_record(&fold, fr.lights.as_ref(), base as u32);
        let at = i * moby_render::EXTRA_RECORD_SIZE;
        if let Some(dst) = rec.get_mut(at..at + r.len()) { dst.copy_from_slice(&r); }
        let t = Transform::from_matrix(fold);
        for &e in &fr.entities[i] {
            if let Ok(mut tr) = transforms.get_mut(e) {
                if *tr != t { *tr = t; }
            }
        }
    }
    for (h, data) in [(&fr.extra.palette, palette), (&fr.extra.instances, records)] {
        let Some(data) = data else { continue };
        if buffers.get(h).and_then(|b| b.data.as_ref()).is_some_and(|d| *d == data) { continue; }
        crate::asset_write::set_buffer(&mut buffers, h, &data);
    }
}

fn clip(a: [i32; 4], b: [i32; 4]) -> [i32; 4] { [a[0].max(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].min(b[3])] }

/// A draw translated into the panel at (ox, oy).
fn translate(d: &Draw, ox: i32, oy: i32) -> Draw {
    let mut d = d.clone();
    match &mut d {
        Draw::Sprite { x, y, .. } | Draw::Text { x, y, .. } | Draw::FxQuad { x, y, .. } | Draw::SpriteSub { x, y, .. } => {
            *x += ox;
            *y += oy;
        }
        // (The HUD's 1/16-pixel calls: the slot meters only, never in a panel; translated for completeness.)
        Draw::Sprite16 { x, y, .. } => {
            *x += ox << 4;
            *y += oy << 4;
        }
        Draw::Rect16 { x0, y0, x1, y1, .. } => {
            *x0 += ox << 4;
            *x1 += ox << 4;
            *y0 += oy << 4;
            *y1 += oy << 4;
        }
        Draw::TextWindow { window: w, .. } => {
            w.x_min += ox as i16;
            w.x_max += ox as i16;
            w.x_anchor += ox as i16;
            w.y_min += oy as i16;
            w.y_max += oy as i16;
            w.y_start += oy as i16;
        }
        Draw::UiFrame { top, bottom, left, right, .. } => {
            *top += oy;
            *bottom += oy;
            *left += ox;
            *right += ox;
        }
    }
    d
}

/// Converts the menu draws to primitives; returns whether the snapshot is shown. The snapshot and the
/// menu's black 0x30 are the menu layer's (module docs), not primitives.
fn convert(
    draws: &[MenuDraw],
    h: &mut Hud2d,
    statics: &mut [Vec<Prim>; 3],
    st: &mut TextState,
    glyphs: &[rc_formats::font::GlyphTable; 3],
    pictures: &mut dyn FnMut(&rc_game::menus::ImageSrc) -> Option<usize>,
) -> bool {
    let full = [0, W - 1, 0, H - 1];
    let mut panel = full;
    let (mut ox, mut oy) = (0, 0);
    let mut snapshot = false;
    for d in draws {
        let start = h.prims.len();
        match d {
            MenuDraw::Hud(x) => text_render::execute(h, st, glyphs, std::slice::from_ref(&translate(x, ox, oy))),
            MenuDraw::Rect { x0, y0, x1, y1, rgba } => h.rect(y0 - 1 + oy, y1 - 1 + oy, x0 - 1 + ox, x1 - 1 + ox, *rgba),
            MenuDraw::Line { x0, y0, x1, y1, rgba } => {
                let (ax, ay, bx, by) = (x0 - 1 + ox, y0 - 1 + oy, x1 - 1 + ox, y1 - 1 + oy);
                // A 1-pixel-wide quad along the line (GS LINE, [M] for the end pixels).
                let pos = if (bx - ax).abs() >= (by - ay).abs() {
                    [[ax, ay], [bx, by], [ax, ay + 1], [bx, by + 1]]
                } else {
                    [[ax, ay], [ax + 1, ay], [bx, by], [bx + 1, by]]
                };
                h.prims.push(Prim { tex: Tex::None, pos, uv: [[0, 0]; 4], rgba: *rgba, scissor: full, repeat: false, nearest: false, boxed: false, uv16: false });
            }
            MenuDraw::SpriteUv { frame, x0, y0, x1, y1, u0, v0, u1, v1, alpha, repeat_u } => {
                let rgba = ((*alpha as u32) & 0xff) << 24 | 0x007f_7f7f;
                let (px0, py0, px1, py1) = (x0 / 16 + ox, y0 / 16 + oy, x1 / 16 + ox, y1 / 16 + oy);
                let (ua, ub, va, vb) = (u0 / 16, u1 / 16, v0 / 16, v1 / 16);
                let tw = h.frame_sizes.get(*frame).map_or(0, |s| s.0);
                let mut quad = |xa: i32, xb: i32, ua: i32, ub: i32| {
                    h.prims.push(Prim { tex: Tex::Frame(*frame), pos: [[xa, py0], [xb, py0], [xa, py1], [xb, py1]], uv: [[ua, va], [ub, va], [ua, vb], [ub, vb]], rgba, scissor: full, repeat: false, nearest: false, boxed: false, uv16: false });
                };
                if *repeat_u && tw > 0 && ub - ua == tw {
                    // CLAMP_1 = REPEAT: split at the texture's wrap.
                    let us = ua.rem_euclid(tw);
                    let xm = px0 + ((px1 - px0) * (tw - us)) / tw;
                    quad(px0, xm, us, tw);
                    if us != 0 { quad(xm, px1, 0, us); }
                } else {
                    quad(px0, px1, ua, ub);
                }
            }
            MenuDraw::FrameQuad { frame, x, y, w, h: ph, u, v, tw, th, rgba } => {
                let (xa, ya, xb, yb) = (x + ox, y + oy, x + w + ox, y + ph + oy);
                let (ub, vb) = (u + tw, v + th);
                h.prims.push(Prim { tex: Tex::Frame(*frame), pos: [[xa, ya], [xb, ya], [xa, yb], [xb, yb]], uv: [[*u, *v], [ub, *v], [*u, vb], [ub, vb]], rgba: *rgba, scissor: full, repeat: false, nearest: false, boxed: false, uv16: false });
            }
            MenuDraw::Snapshot => snapshot = true,
            // The menu layer's clear colour until the snapshot arrives, then in the snapshot's bytes.
            MenuDraw::Darken { alpha } if *alpha != DARKEN => h.rect(0, H, 0, W, ((*alpha as u32) & 0xff) << 24),
            MenuDraw::Darken { .. } => {}
            MenuDraw::PanelBegin { x, y, w, h: ph, clear } => {
                panel = [*x, x + w - 1, *y, y + ph - 1];
                (ox, oy) = (*x, *y);
                h.rect(*y, y + ph, *x, x + w, *clear);
            }
            MenuDraw::PanelEnd => {
                panel = full;
                (ox, oy) = (0, 0);
            }
            MenuDraw::Stub(_) => {}
            MenuDraw::Image { src, x, y, w, h: ph, u, v, tw, th, rgba } => {
                if let Some(slot) = pictures(src) { h.prims.push(crate::hud_images::prim(slot, x + ox, y + oy, *w, *ph, *u, *v, *tw, *th, *rgba)); }
            }
            MenuDraw::Quad { tex, pos, uv, rgba, repeat, uv16 } => {
                let tex = match tex {
                    rc_game::menus::QuadTex::Frame(f) => Some(Tex::Frame(*f)),
                    rc_game::menus::QuadTex::Image(src) => pictures(src).map(Tex::Dyn),
                };
                if let Some(tex) = tex {
                    let pos = pos.map(|[x, y]| [x + ox, y + oy]);
                    h.prims.push(Prim { tex, pos, uv: *uv, rgba: *rgba, scissor: full, repeat: *repeat, nearest: false, boxed: false, uv16: *uv16 });
                }
            }
            MenuDraw::Static(s) => {
                let p = static_prim(s, ox, oy);
                statics[(s.pass as usize).min(2)].push(Prim { scissor: panel, ..p });
            }
        }
        for p in &mut h.prims[start..] { p.scissor = clip(p.scissor, panel); }
    }
    snapshot
}

/// A static draw (rc_game::menus::screen_static, `DrawTexturedQuad` / `fun_00200080` after CLAMP_1 = 0) as a
/// primitive of the static layer (`Hud2dHook::statics`).
pub(crate) fn static_prim(s: &StaticDraw, ox: i32, oy: i32) -> Prim {
    let tex = match s.tex {
        StaticTex::Fx(i) => Tex::Fx(i),
        StaticTex::Frame(i) => Tex::Frame(i),
    };
    let (x0, y0, x1, y1) = (s.x + ox, s.y + oy, s.x + s.w + ox, s.y + s.h + oy);
    let (u0, v0, u1, v1) = (s.u, s.v, s.u + s.tw, s.v + s.th);
    Prim { tex, pos: [[x0, y0], [x1, y0], [x0, y1], [x1, y1]], uv: [[u0, v0], [u1, v0], [u0, v1], [u1, v1]], rgba: s.rgba, scissor: [0, W - 1, 0, H - 1], repeat: true, nearest: false, boxed: false, uv16: false }
}

// ---- Snapshot, render world (module docs, "Snapshot") ----

/// Main world: the entering frame's snapshot job (the main camera's target, the snapshot image); set by
/// `build_prims`, cleared in `First`, so exactly the entering frame's extraction sees it.
#[derive(Resource, Default)]
struct SnapshotRequest(Option<(NormalizedRenderTarget, AssetId<Image>)>);

/// Render world: this frame's snapshot job.
#[derive(Resource, Default)]
struct SnapshotJob(Option<(NormalizedRenderTarget, AssetId<Image>)>);

/// Render world: the texture a window target renders into on the snapshot frame (None otherwise, or when a
/// `Screenshot` already renders that window offscreen).
#[derive(Resource, Default)]
struct SnapshotOffscreen(Option<TextureView>);

/// Pipelines of `menu_snapshot.wgsl`, by (copy, source view is sRGB, target format).
#[derive(Resource)]
struct SnapshotPipelines {
    layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
    vertex: VertexState,
    ids: HashMap<(bool, bool, TextureFormat), CachedRenderPipelineId>,
}

impl SnapshotPipelines {
    fn id(&mut self, cache: &PipelineCache, copy: bool, src_srgb: bool, format: TextureFormat) -> CachedRenderPipelineId {
        let (layout, shader, vertex) = (&self.layout, &self.shader, &self.vertex);
        *self.ids.entry((copy, src_srgb, format)).or_insert_with(|| {
            let mut shader_defs = vec![ShaderDefVal::Int("DARKEN".into(), DARKEN)];
            if src_srgb { shader_defs.push("SRC_SRGB".into()); }
            cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(if copy { "menu snapshot: window copy" } else { "menu snapshot: darken" }.into()),
                layout: vec![layout.clone()],
                vertex: vertex.clone(),
                fragment: Some(FragmentState {
                    shader: shader.clone(),
                    shader_defs,
                    entry_point: Some(if copy { "copy" } else { "darken" }.into()),
                    targets: vec![Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })],
                }),
                ..default()
            })
        })
    }

    fn get<'a>(&self, cache: &'a PipelineCache, copy: bool, src_srgb: bool, format: TextureFormat) -> Option<&'a RenderPipeline> {
        self.ids.get(&(copy, src_srgb, format)).and_then(|id| cache.get_render_pipeline(*id))
    }
}

fn init_snapshot_pipelines(mut commands: Commands, assets: Res<AssetServer>, fullscreen: Res<FullscreenShader>) {
    commands.insert_resource(SnapshotPipelines {
        layout: BindGroupLayoutDescriptor::new(
            "menu snapshot",
            &BindGroupLayoutEntries::single(ShaderStages::FRAGMENT, texture_2d(TextureSampleType::Float { filterable: false })),
        ),
        shader: assets.load("shaders/menu_snapshot.wgsl"),
        vertex: fullscreen.to_vertex_state(),
        ids: HashMap::default(),
    });
}

fn extract_snapshot(mut job: ResMut<SnapshotJob>, request: Extract<Res<SnapshotRequest>>) { job.0 = request.0.clone(); }

/// Queues the pipelines every frame (so they are compiled before the menu is first entered) and, on the
/// snapshot frame of a window target, points the window's output attachment at a samplable texture.
fn prepare_snapshot(
    job: Res<SnapshotJob>,
    windows: Res<ExtractedWindows>,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    mut pipes: ResMut<SnapshotPipelines>,
    mut attachments: ResMut<ViewTargetAttachments>,
    mut offscreen: ResMut<SnapshotOffscreen>,
) {
    offscreen.0 = None;
    for srgb in [false, true] { pipes.id(&cache, false, srgb, TextureFormat::Rgba8Unorm); }
    for w in windows.windows.values() {
        if let Some(f) = w.swap_chain_texture_view_format { pipes.id(&cache, true, f.is_srgb(), f); }
    }
    let Some((target @ NormalizedRenderTarget::Window(window), _)) = &job.0 else { return };
    let Some(w) = windows.windows.get(&window.entity()) else { return };
    let (Some(swap), Some(att)) = (w.swap_chain_texture_view.as_ref(), attachments.get(target)) else { return };
    // A `Screenshot` of this window already renders it into a samplable texture: that is read instead.
    if att.view.id() != swap.id() { return; }
    let format = att.view_format;
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("menu snapshot: window frame"),
        size: Extent3d { width: w.physical_width, height: w.physical_height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    attachments.insert(target.clone(), OutputColorAttachment::new(view.clone(), format));
    offscreen.0 = Some(view);
}

fn fullscreen_pass(encoder: &mut CommandEncoder, device: &RenderDevice, cache: &PipelineCache, layout: &BindGroupLayoutDescriptor, pipeline: &RenderPipeline, src: &TextureView, dst: &TextureView) {
    let bind_group = device.create_bind_group("menu snapshot", &cache.get_bind_group_layout(layout), &BindGroupEntries::single(src));
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("menu snapshot"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: dst,
            depth_slice: None,
            resolve_target: None,
            ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
}

/// After every camera of the frame has been submitted (`RenderGraphSystems::Finish`), before present and before
/// Bevy's screenshot copies: the target's finished frame → the snapshot texture, darkened; a window frame
/// rendered offscreen for it → back to the swap chain.
#[allow(clippy::too_many_arguments)]
fn snapshot_copy(
    job: Res<SnapshotJob>,
    offscreen: Res<SnapshotOffscreen>,
    attachments: Res<ViewTargetAttachments>,
    windows: Res<ExtractedWindows>,
    images: Res<RenderAssets<GpuImage>>,
    pipes: Option<Res<SnapshotPipelines>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let (Some((target, dst)), Some(pipes)) = (&job.0, pipes) else { return };
    let Some(src) = attachments.get(target) else {
        eprintln!("menus: the main target has no output attachment this frame: no snapshot");
        return;
    };
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: Some("menu snapshot") });
    match (images.get(*dst), pipes.get(&cache, false, src.view_format.is_srgb(), TextureFormat::Rgba8Unorm)) {
        (Some(dst), Some(pipeline)) => {
            let view = dst.texture.create_view(&TextureViewDescriptor { format: Some(TextureFormat::Rgba8Unorm), ..default() });
            fullscreen_pass(&mut encoder, &device, &cache, &pipes.layout, pipeline, &src.view, &view);
        }
        (None, _) => eprintln!("menus: the snapshot texture is not on the GPU: no snapshot"),
        (_, None) => eprintln!("menus: the snapshot pipeline is not compiled yet: no snapshot"),
    }
    // The window's frame went to our texture (not replaced by a screenshot's): present it.
    let window = match (&offscreen.0, target) {
        (Some(own), NormalizedRenderTarget::Window(w)) if own.id() == src.view.id() => windows.windows.get(&w.entity()).map(|w| (own, w)),
        _ => None,
    };
    if let Some((own, (Some(swap), Some(f)))) = window.map(|(own, w)| (own, (&w.swap_chain_texture_view, w.swap_chain_texture_view_format))) {
        match pipes.get(&cache, true, f.is_srgb(), f) {
            Some(pipeline) => fullscreen_pass(&mut encoder, &device, &cache, &pipes.layout, pipeline, own, swap),
            None => eprintln!("menus: the window copy pipeline is not compiled yet: this frame is not presented"),
        }
    }
    queue.submit([encoder.finish()]);
}
