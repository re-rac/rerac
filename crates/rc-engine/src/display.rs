//! **Port-only: the game frame and its presentation.** The PS2 drew one 512×416 buffer; the port draws the whole
//! game (world, sky, HUD, menus, canvases) into one offscreen image of that shape, [`GameFrame`], and a last camera
//! ([`PresentCamera`]) draws it into the window, scaled to fit and centred on black. Every camera that would render
//! to the primary window is pointed at the frame as it is spawned ([`redirect`]), so nothing else needs to know
//! the window's size: overlays laid out on the frame (the HUD composite, the menu layer and its snapshot, the item
//! canvases) match it at any window size, the menu snapshot copies the frame without bars, and a screenshot is the
//! frame itself.
//!
//! **Aspect.** The PS2 drew 512×416 and the TV showed it at 4:3; the frame is 4:3 ([`Aspect::Original`], the
//! default), 16:10 ([`Aspect::Wide16x10`], a MacBook's screen) or 16:9 ([`Aspect::Wide16x9`]). The game's 512×416
//! screen (the HUD, the menus, the item canvases) always maps onto the frame's centred 4:3 box ([`ui_box`]: the whole
//! frame at 4:3) and the 3D view widens around it: the world cameras' horizontal tangent is the game's ×6/5 at 16:10,
//! ×4/3 at 16:9 ([`hor_scale`], Hor+: the vertical view is the game's).
//!
//! Gameplay and cutscenes have an aspect each ([`DisplaySettings::gameplay`] / [`DisplaySettings::cutscenes`]). The
//! frame is the wider of the two ([`DisplaySettings::frame_aspect`]) and every projection is the frame's; the view
//! shown now ([`DisplaySettings::shown`]) is pillarboxed when narrower ([`pillar_bars`], drawn by crate::hud_render).
//! Hor+ keeps the vertical view, so the frame's centre at a narrower aspect is exactly that aspect's view.
//!
//! **Settings** ([`DisplaySettings`], the Port Options page; persisted in the port settings file,
//! `crate::render_settings::save_key`): the two aspects, the render resolution — "Window" (the largest frame of the
//! aspect that fits the window, in physical pixels) or a frame of a fixed height (416p, the PS2's lines, or 720p /
//! 1080p / 1440p / 2160p; the width follows the aspect) scaled to the window — and fullscreen (borderless on the
//! current monitor). `RC_ASPECT=4:3|16:10|16:9` (both aspects), `RC_CUTSCENE_ASPECT` (the cutscenes' over it),
//! `RC_RESOLUTION=window|416|720|1080|1440|2160` and `RC_FULLSCREEN=0|1` override the file at start; F11 toggles
//! fullscreen and saves it.
//!
//! The frame's format is the window's swap-chain format (`Bgra8UnormSrgb`), so every pipeline is specialised as it
//! was for the window; it can be copied (screenshots) and sampled (the present sprite, linear filtering).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureFormat, TextureUsages};
use bevy::render::view::Msaa;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode, WindowRef};

use crate::game_camera::SCREEN_W;

/// The present camera's own layer (nothing else draws in it).
pub const PRESENT_LAYER: usize = 63;

/// The offscreen image every game camera renders into, and its current size (physical pixels).
#[derive(Resource, Clone, Debug)]
pub struct GameFrame {
    pub image: Handle<Image>,
    pub size: UVec2,
}

/// The frame's shape: the TV's 4:3, or a wide screen's 16:10 / 16:9 (the Port Options row's order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aspect {
    Original,
    Wide16x10,
    Wide16x9,
}

impl Aspect {
    /// The Port Options row's values, in order.
    pub const ALL: [Aspect; 3] = [Aspect::Original, Aspect::Wide16x10, Aspect::Wide16x9];
    pub fn index(self) -> u8 { Self::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8 }
    pub fn from_index(i: u8) -> Self { Self::ALL.get(i as usize).copied().unwrap_or(Aspect::Original) }
    /// Width / height.
    pub fn ratio(self) -> f32 {
        match self {
            Aspect::Original => 4.0 / 3.0,
            Aspect::Wide16x10 => 16.0 / 10.0,
            Aspect::Wide16x9 => 16.0 / 9.0,
        }
    }
    fn parse(v: &str) -> Option<Self> { Self::ALL.into_iter().find(|a| a.key() == v.trim()) }
    fn key(self) -> &'static str {
        match self {
            Aspect::Original => "4:3",
            Aspect::Wide16x10 => "16:10",
            Aspect::Wide16x9 => "16:9",
        }
    }
}

/// The factor on the game camera's horizontal tangent (0.63) for `aspect`: 1 at 4:3, 6/5 at 16:10, 4/3 at 16:9 (the
/// frame is that much wider at the same height, and the view keeps the TV's proportions).
pub fn hor_scale(aspect: Aspect) -> f32 { aspect.ratio() / Aspect::Original.ratio() }

/// The game pixels the frame extends past the 512-wide game screen on each side (0 at 4:3, 51 at 16:10, 85 at 16:9): the HUD's
/// layer is that much wider (crate::hud_render), its left / right slots move out by it (`rc_game::hud`).
pub fn side_extra(aspect: Aspect) -> i32 { ((SCREEN_W * hor_scale(aspect) - SCREEN_W) * 0.5).round() as i32 }

/// The black bars that narrow a frame of aspect `frame` to `shown` (HUD-layer game pixels: x from `−side_extra(frame)`
/// to `512 + side_extra(frame)`): `[left, right]` x spans, None when `shown` is not narrower.
pub fn pillar_bars(frame: Aspect, shown: Aspect) -> Option<[[i32; 2]; 2]> {
    let (f, s) = (side_extra(frame), side_extra(shown));
    (s < f).then(|| [[-f, -s], [SCREEN_W as i32 + s, SCREEN_W as i32 + f]])
}

/// The render resolution: fit to the window, or a fixed frame height (the width follows the aspect): the PS2's 416
/// lines or a common screen height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Window,
    Height(u16),
}

impl Resolution {
    /// The Port Options row's values, in order.
    pub const ALL: [Resolution; 6] = [
        Resolution::Window,
        Resolution::Height(416),
        Resolution::Height(720),
        Resolution::Height(1080),
        Resolution::Height(1440),
        Resolution::Height(2160),
    ];

    pub fn index(self) -> u8 { Self::ALL.iter().position(|&r| r == self).unwrap_or(0) as u8 }
    pub fn from_index(i: u8) -> Self { Self::ALL.get(i as usize).copied().unwrap_or(Resolution::Window) }

    /// `window`, or a height of the list (`720p` or `720`).
    fn parse(v: &str) -> Option<Self> {
        let v = v.trim();
        if v == "window" { return Some(Resolution::Window); }
        let h = v.strip_suffix('p').unwrap_or(v).parse::<u16>().ok()?;
        Self::ALL.into_iter().find(|&r| r == Resolution::Height(h))
    }

    fn key(self) -> String {
        match self {
            Resolution::Window => "window".into(),
            Resolution::Height(h) => format!("{h}p"),
        }
    }
}

/// The display settings a menu can change at run time.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplaySettings {
    /// The gameplay view's aspect (the HUD's side slots move out to its edges).
    pub gameplay: Aspect,
    /// The cutscenes' aspect ([`CutsceneView`]).
    pub cutscenes: Aspect,
    pub resolution: Resolution,
    pub fullscreen: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self { DisplaySettings { gameplay: Aspect::Original, cutscenes: Aspect::Original, resolution: Resolution::Window, fullscreen: false } }
}

impl DisplaySettings {
    /// The frame's aspect: the wider of the two (every projection is the frame's).
    pub fn frame_aspect(&self) -> Aspect { if self.cutscenes.ratio() > self.gameplay.ratio() { self.cutscenes } else { self.gameplay } }

    /// The aspect shown now: the cutscenes' during one, else the gameplay's.
    pub fn shown(&self, cutscene: bool) -> Aspect { if cutscene { self.cutscenes } else { self.gameplay } }

    /// The settings file's values, then `RC_ASPECT` / `RC_CUTSCENE_ASPECT` / `RC_RESOLUTION` / `RC_FULLSCREEN` over
    /// them. Without a `cutscene_aspect` key the cutscenes take the gameplay's (a file of the single aspect row).
    pub fn startup() -> Self {
        let mut s = DisplaySettings::default();
        if let Some(a) = crate::render_settings::load_key("aspect").as_deref().and_then(Aspect::parse) { s.gameplay = a; }
        s.cutscenes = crate::render_settings::load_key("cutscene_aspect").as_deref().and_then(Aspect::parse).unwrap_or(s.gameplay);
        if let Some(a) = std::env::var("RC_ASPECT").ok().as_deref().and_then(Aspect::parse) { (s.gameplay, s.cutscenes) = (a, a); }
        if let Some(a) = std::env::var("RC_CUTSCENE_ASPECT").ok().as_deref().and_then(Aspect::parse) { s.cutscenes = a; }
        if let Some(r) = crate::render_settings::load_key("resolution").as_deref().and_then(Resolution::parse) { s.resolution = r; }
        if let Some(f) = crate::render_settings::load_key("fullscreen") { s.fullscreen = f.trim() == "1"; }
        if let Some(r) = std::env::var("RC_RESOLUTION").ok().as_deref().and_then(Resolution::parse) { s.resolution = r; }
        if let Ok(f) = std::env::var("RC_FULLSCREEN") { s.fullscreen = f.trim() == "1"; }
        s
    }

    /// Writes the keys into the port settings file.
    pub fn save(&self) {
        crate::render_settings::save_key("aspect", self.gameplay.key());
        crate::render_settings::save_key("cutscene_aspect", self.cutscenes.key());
        crate::render_settings::save_key("resolution", &self.resolution.key());
        crate::render_settings::save_key("fullscreen", if self.fullscreen { "1" } else { "0" });
    }
}

/// The frame's size for `res` and `aspect` in a window of `window` physical pixels: the largest rectangle of the
/// aspect that fits, or one of the chosen height.
pub fn frame_size(res: Resolution, aspect: Aspect, window: UVec2) -> UVec2 {
    let a = aspect.ratio();
    match res {
        Resolution::Height(h) => {
            let h = h as f32;
            UVec2::new((h * a).round() as u32, h as u32)
        }
        Resolution::Window => {
            let h = (window.y as f32).min(window.x as f32 / a);
            UVec2::new((h * a).round() as u32, h.round() as u32).min(window).max(UVec2::ONE)
        }
    }
}

/// The window's size at start (physical pixels; `scale` the window's scale factor): a fixed height's frame, or for
/// "Window" 1024 logical pixels wide at the aspect; shrunk to 90 % of `limit` (the monitor) when larger, keeping the
/// aspect.
pub fn start_window_size(res: Resolution, aspect: Aspect, scale: f32, limit: Option<UVec2>) -> UVec2 {
    let a = aspect.ratio();
    let size = match res {
        Resolution::Height(h) => Vec2::new(h as f32 * a, h as f32),
        Resolution::Window => Vec2::new(1024.0, 1024.0 / a) * scale,
    };
    let k = limit.map_or(1.0, |l| (l.x as f32 * 0.9 / size.x).min(l.y as f32 * 0.9 / size.y).min(1.0));
    (size * k).round().as_uvec2().max(UVec2::ONE)
}

/// The primary window as `main` creates it: [`start_window_size`] at scale 1 (corrected by [`size_window_at_start`] once
/// the scale factor and the monitor are known) and the fullscreen mode, so the first frames already have the settings.
pub fn start_window(title: &str) -> Window {
    let s = DisplaySettings::startup();
    let size = start_window_size(s.resolution, s.frame_aspect(), 1.0, None);
    let mode = if s.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
    Window { title: title.into(), resolution: (size.x, size.y).into(), mode, ..default() }
}

/// Once, on the first frame the window has a size: its physical size from the settings ([`start_window_size`], within
/// the primary monitor). Fullscreen keeps the monitor's.
fn size_window_at_start(
    mut done: Local<bool>,
    settings: Res<DisplaySettings>,
    mut window: Option<Single<&mut Window, With<PrimaryWindow>>>,
    monitors: Query<&bevy::window::Monitor, With<bevy::window::PrimaryMonitor>>,
) {
    if *done { return; }
    let Some(w) = window.as_deref_mut() else { return };
    if w.physical_width() == 0 || w.physical_height() == 0 { return; }
    *done = true;
    if settings.fullscreen { return; }
    let limit = monitors.iter().next().map(|m| UVec2::new(m.physical_width, m.physical_height));
    let want = start_window_size(settings.resolution, settings.frame_aspect(), w.scale_factor(), limit);
    if w.physical_size() != want {
        println!("display: window {}x{} (the settings' {:?} at {:?})", want.x, want.y, settings.resolution, settings.frame_aspect());
        w.resolution.set_physical_resolution(want.x, want.y);
    }
}

/// The frame's centred 4:3 box (origin and size, frame pixels): where the game's 512×416 screen lands.
pub fn ui_box(frame: UVec2) -> (Vec2, Vec2) { aspect_box(frame, Aspect::Original) }

/// The frame's centred box of `aspect` (origin and size, frame pixels; the whole frame when it is not narrower).
pub fn aspect_box(frame: UVec2, aspect: Aspect) -> (Vec2, Vec2) {
    let h = frame.y as f32;
    let w = (h * aspect.ratio()).round().min(frame.x as f32);
    (Vec2::new(((frame.x as f32 - w) * 0.5).round(), 0.0), Vec2::new(w, h))
}

/// The largest rectangle of the frame's aspect that fits `window` (logical units): the present sprite's size.
pub fn fit(frame: UVec2, window: Vec2) -> Vec2 {
    let (fw, fh) = (frame.x.max(1) as f32, frame.y.max(1) as f32);
    // Cross-multiplied (no aspect quotient): a window of the frame's own aspect gets exactly its size.
    if window.x * fh > window.y * fw { Vec2::new(window.y * fw / fh, window.y) } else { Vec2::new(window.x, window.x * fh / fw) }
}

/// This frame shows a cutscene ([`DisplaySettings::cutscenes`]'s aspect): a story scene, a letterboxed cutaway
/// (`0x15f404`), the ship's take-off / landing / fly-away (crate::scene_render's `cutscene_view`).
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutsceneView(pub bool);

/// A UI node kept on the frame's 4:3 box ([`ui_box`]; percent of its parent, which must span the frame): the HUD
/// composite (and the canvases composed with it, its children).
#[derive(Component)]
pub struct UiBoxed;

/// A UI node kept on the frame's box of the cutscenes' aspect ([`aspect_box`] of [`DisplaySettings::cutscenes`]): the
/// movie picture, stretched from the disc's 4:3 to a wide cutscene aspect.
#[derive(Component)]
pub struct CutsceneBoxed;

/// A 3D camera that draws the game's 512×416 screen (the menu's frame mobys): its viewport is the frame's 4:3 box.
#[derive(Component)]
pub struct BoxedView;

/// The camera that draws the frame into the window.
#[derive(Component)]
pub struct PresentCamera;

/// The frame's sprite on the present camera.
#[derive(Component)]
struct PresentSprite;

pub struct DisplayPlugin;

impl Plugin for DisplayPlugin {
    /// The frame exists from `build` on, so plugins built later (crate::determinism's capture) can use it.
    fn build(&self, app: &mut App) {
        let size = UVec2::new(1024, 768);
        let mut img = Image::new_target_texture(size.x, size.y, TextureFormat::Bgra8UnormSrgb, None);
        img.asset_usage = RenderAssetUsages::RENDER_WORLD;
        img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        img.sampler = ImageSampler::linear();
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(img);
        let settings = DisplaySettings::startup();
        println!(
            "display: gameplay aspect {:?}, cutscene aspect {:?}, render resolution {:?}, fullscreen {} (Port Options; RC_ASPECT / RC_CUTSCENE_ASPECT / RC_RESOLUTION / RC_FULLSCREEN)",
            settings.gameplay, settings.cutscenes, settings.resolution, settings.fullscreen
        );
        app.insert_resource(GameFrame { image, size })
            .insert_resource(settings)
            .init_resource::<CutsceneView>()
            .add_observer(redirect)
            .add_systems(Startup, spawn_present)
            .add_systems(First, sync_frame)
            .add_systems(Update, (size_window_at_start, toggle_fullscreen, apply_mode, fit_present).chain())
            .add_systems(PostUpdate, (place_boxed, box_views, fly_view, refresh_resized_targets
                .after(ResizeTargets)
                .before(bevy::camera::CameraUpdateSystems)));
    }
}

/// A camera spawned for the primary window renders into the frame instead (all but the present camera).
fn redirect(add: On<Add, Camera>, frame: Res<GameFrame>, targets: Query<&RenderTarget>, present: Query<(), With<PresentCamera>>, mut commands: Commands) {
    if present.contains(add.entity) { return; }
    if matches!(targets.get(add.entity), Ok(RenderTarget::Window(WindowRef::Primary))) {
        commands.entity(add.entity).insert(RenderTarget::Image(frame.image.clone().into()));
    }
}

/// The present camera and sprite live for the whole run: a level change keeps them (crate::level_switch despawns
/// every other camera), else the window shows nothing after the first change.
fn spawn_present(mut commands: Commands, frame: Res<GameFrame>) {
    commands.spawn((
        Camera2d,
        crate::level_switch::KeepAcrossLevels,
        Camera { order: 1000, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
        RenderTarget::Window(WindowRef::Primary),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        RenderLayers::layer(PRESENT_LAYER),
        PresentCamera,
        Name::new("present camera (game frame → window)"),
    ));
    commands.spawn((
        Sprite { image: frame.image.clone(), custom_size: Some(Vec2::new(1024.0, 768.0)), ..default() },
        crate::level_switch::KeepAcrossLevels,
        RenderLayers::layer(PRESENT_LAYER),
        PresentSprite,
        Name::new("game frame"),
    ));
}

/// Keeps the frame at the size the settings and the window ask for.
fn sync_frame(mut frame: ResMut<GameFrame>, settings: Res<DisplaySettings>, window: Option<Single<&Window, With<PrimaryWindow>>>, mut images: ResMut<Assets<Image>>) {
    let Some(w) = window else { return };
    let ws = w.physical_size();
    if ws.x == 0 || ws.y == 0 { return; }
    let want = frame_size(settings.resolution, settings.frame_aspect(), ws);
    if want == frame.size { return; }
    if let Some(mut img) = images.get_mut(&frame.image) { img.resize(Extent3d { width: want.x, height: want.y, depth_or_array_layers: 1 }); }
    frame.size = want;
}

/// A copy of the image target `old` at `size`, added as a new image: the caller points its cameras, nodes and
/// materials at it. A target resized in place keeps a stale render-world size (its content came out scaled by the new /
/// old width after an aspect switch); a fresh image starts clean. The old image drops with its last handle.
pub fn fresh_target(images: &mut Assets<Image>, old: &Handle<Image>, size: UVec2) -> Option<Handle<Image>> {
    let mut img = images.get(old)?.clone();
    img.resize(Extent3d { width: size.x.max(1), height: size.y.max(1), depth_or_array_layers: 1 });
    Some(images.add(img))
}

/// The PostUpdate systems that resize camera image targets (crate::menu_render's layer, crate::screen_canvas):
/// [`refresh_resized_targets`] runs after them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResizeTargets;

/// A camera whose image target was resized this frame (the frame, the HUD layer, the menu layer, the canvases: after
/// their resizing systems) takes the new size before Bevy's camera update. Bevy re-reads an image target's size only
/// on the image's asset event, which reaches `camera_system` a frame later, while a camera whose projection changed
/// (crate::display's tangents on an aspect switch) re-reads it at once: the cameras sharing the frame's depth texture
/// (sized by whichever comes first) would disagree and wgpu rejects the pass (depth 555×416 against colour 740×416).
fn refresh_resized_targets(images: Res<Assets<Image>>, mut cams: Query<(&Camera, &RenderTarget, &mut Projection)>) {
    for (c, t, mut p) in &mut cams {
        let RenderTarget::Image(i) = t else { continue };
        let Some(img) = images.get(&i.handle) else { continue };
        if c.physical_target_size().is_some_and(|s| s != img.size()) { p.set_changed(); }
    }
}

/// F11: fullscreen on / off (saved).
fn toggle_fullscreen(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<DisplaySettings>) {
    if !keys.just_pressed(KeyCode::F11) { return; }
    settings.fullscreen = !settings.fullscreen;
    settings.save();
}

/// The window mode follows the setting.
fn apply_mode(settings: Res<DisplaySettings>, mut window: Option<Single<&mut Window, With<PrimaryWindow>>>) {
    let Some(w) = window.as_deref_mut() else { return };
    let want = if settings.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
    if w.mode != want { w.mode = want; }
}

/// The [`UiBoxed`] nodes on the 4:3 box, the [`CutsceneBoxed`] ones on the cutscenes' box.
fn place_boxed(
    frame: Res<GameFrame>,
    settings: Res<DisplaySettings>,
    mut boxed: Query<&mut Node, (With<UiBoxed>, Without<CutsceneBoxed>)>,
    mut cutscene: Query<&mut Node, (With<CutsceneBoxed>, Without<UiBoxed>)>,
) {
    let fw = frame.size.x.max(1) as f32;
    let span = |(o, s): (Vec2, Vec2)| (Val::Percent(o.x * 100.0 / fw), Val::Percent(s.x * 100.0 / fw));
    let place = |n: &mut Node, want: (Val, Val)| {
        if (n.left, n.width) != want {
            (n.left, n.width) = want;
            (n.top, n.height) = (Val::Px(0.0), Val::Percent(100.0));
        }
    };
    let want = span(ui_box(frame.size));
    for mut n in &mut boxed { place(&mut n, want); }
    let want = span(aspect_box(frame.size, settings.cutscenes));
    for mut n in &mut cutscene { place(&mut n, want); }
}

/// The viewport of the frame's 4:3 box (none when it is the whole frame).
pub fn box_viewport(frame: UVec2) -> Option<bevy::camera::Viewport> {
    let (o, s) = ui_box(frame);
    (s.x < frame.x as f32).then(|| bevy::camera::Viewport { physical_position: o.as_uvec2(), physical_size: s.as_uvec2().max(UVec2::ONE), ..default() })
}

/// The [`BoxedView`] cameras' viewport: [`box_viewport`]. A camera spawned with it already ([`box_viewport`] at the
/// spawn) draws its first frame boxed too: that frame can be the one the menu's snapshot copies.
fn box_views(frame: Res<GameFrame>, mut cams: Query<&mut Camera, With<BoxedView>>) {
    let want = box_viewport(frame.size);
    for mut c in &mut cams {
        let same = match (&c.viewport, &want) {
            (Some(a), Some(b)) => a.physical_position == b.physical_position && a.physical_size == b.physical_size,
            (None, None) => true,
            _ => false,
        };
        if !same { c.viewport = want.clone(); }
    }
}

/// The projection tangents for the game's horizontal tangent `game_tan` (`0x16cf70`, a scene camera's): x widened by
/// [`hor_scale`] (Hor+), y the game's (`× 0.775`, NTSC). Every writer of the world cameras' projection goes through
/// this (crate::play_camera, crate::scene_render, crate::flight_render, [`fly_view`]); the game's own screen-space
/// code keeps `game_tan` (its 512×416 screen is the 4:3 box).
pub fn view_tans(game_tan: f32, aspect: Aspect) -> (f32, f32) { (game_tan * hor_scale(aspect), game_tan * crate::game_camera::NTSC_Y_RATIO) }

/// Sets `p`'s tangents to [`view_tans`] of `game_tan` when they differ; true when it changed them.
pub fn set_view_tans(p: &mut Projection, game_tan: f32, aspect: Aspect) -> bool {
    let (tx, ty) = view_tans(game_tan, aspect);
    let Projection::Custom(c) = p else { return false };
    let Some(g) = c.get_mut::<crate::game_camera::GameProjection>() else { return false };
    if g.tan_x == tx && g.tan_y == ty { return false; }
    (g.tan_x, g.tan_y) = (tx, ty);
    true
}

/// The fly camera (no game camera writes the projection): the game's default tangent through [`view_tans`].
fn fly_view(settings: Res<DisplaySettings>, source: Option<Res<crate::game_camera::CameraSource>>, mut cams: Query<&mut Projection, With<crate::fly_cam::FlyCam>>) {
    if source.is_some_and(|s| *s == crate::game_camera::CameraSource::Play) { return; }
    for mut p in &mut cams { set_view_tans(&mut p, crate::game_camera::TAN_HALF_FOV_X, settings.frame_aspect()); }
}

/// The frame's sprite fills the window at the frame's aspect, centred (the camera clears the rest to black).
fn fit_present(frame: Res<GameFrame>, window: Option<Single<&Window, With<PrimaryWindow>>>, mut sprites: Query<&mut Sprite, With<PresentSprite>>) {
    let Some(w) = window else { return };
    let size = fit(frame.size, Vec2::new(w.width(), w.height()));
    for mut s in &mut sprites {
        if s.custom_size != Some(size) { s.custom_size = Some(size); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        let (o, w) = (Aspect::Original, Aspect::Wide16x9);
        assert_eq!(frame_size(Resolution::Window, o, UVec2::new(1024, 768)), UVec2::new(1024, 768));
        assert_eq!(frame_size(Resolution::Window, o, UVec2::new(2560, 1440)), UVec2::new(1920, 1440));
        assert_eq!(frame_size(Resolution::Window, w, UVec2::new(2560, 1440)), UVec2::new(2560, 1440));
        assert_eq!(frame_size(Resolution::Height(1080), o, UVec2::new(300, 300)), UVec2::new(1440, 1080));
        assert_eq!(frame_size(Resolution::Height(1080), Aspect::Wide16x10, UVec2::new(300, 300)), UVec2::new(1728, 1080));
        assert_eq!(frame_size(Resolution::Height(1080), w, UVec2::new(300, 300)), UVec2::new(1920, 1080));
        assert_eq!(frame_size(Resolution::Window, Aspect::Wide16x10, UVec2::new(3024, 1890)), UVec2::new(3024, 1890));
        assert_eq!(side_extra(Aspect::Wide16x10), 51);
        assert_eq!(Resolution::parse("720"), Some(Resolution::Height(720)));
        assert_eq!(ui_box(UVec2::new(2560, 1440)), (Vec2::new(320.0, 0.0), Vec2::new(1920.0, 1440.0)));
        assert_eq!(ui_box(UVec2::new(1024, 768)), (Vec2::ZERO, Vec2::new(1024.0, 768.0)));
        assert_eq!(aspect_box(UVec2::new(2560, 1440), w), (Vec2::ZERO, Vec2::new(2560.0, 1440.0)));
        assert_eq!(aspect_box(UVec2::new(2560, 1440), Aspect::Wide16x10), (Vec2::new(128.0, 0.0), Vec2::new(2304.0, 1440.0)));
        assert_eq!(hor_scale(o), 1.0);
        assert!((hor_scale(w) - 4.0 / 3.0).abs() < 1e-6);
        assert_eq!(fit(UVec2::new(1024, 832), Vec2::new(1600.0, 832.0)), Vec2::new(1024.0, 832.0));
        assert_eq!(fit(UVec2::new(1024, 832), Vec2::new(1024.0, 1000.0)), Vec2::new(1024.0, 832.0));
        for r in Resolution::ALL { assert_eq!(Resolution::from_index(r.index()), r); assert_eq!(Resolution::parse(&r.key()), Some(r)); }
        for a in Aspect::ALL { assert_eq!(Aspect::from_index(a.index()), a); assert_eq!(Aspect::parse(a.key()), Some(a)); }
    }

    #[test]
    fn start_sizes() {
        let w = Aspect::Wide16x9;
        assert_eq!(start_window_size(Resolution::Height(720), w, 1.5, Some(UVec2::new(2560, 1440))), UVec2::new(1280, 720));
        assert_eq!(start_window_size(Resolution::Window, Aspect::Original, 1.0, None), UVec2::new(1024, 768));
        assert_eq!(start_window_size(Resolution::Window, w, 2.0, None), UVec2::new(2048, 1152));
        // 2160p on a 1080p monitor: 90 % of it, at the aspect.
        assert_eq!(start_window_size(Resolution::Height(2160), w, 1.0, Some(UVec2::new(1920, 1080))), UVec2::new(1728, 972));
    }

    #[test]
    fn split_aspects() {
        let s = DisplaySettings { gameplay: Aspect::Original, cutscenes: Aspect::Wide16x9, ..default() };
        assert_eq!(s.frame_aspect(), Aspect::Wide16x9);
        assert_eq!((s.shown(false), s.shown(true)), (Aspect::Original, Aspect::Wide16x9));
        let s = DisplaySettings { gameplay: Aspect::Wide16x9, cutscenes: Aspect::Wide16x10, ..default() };
        assert_eq!(s.frame_aspect(), Aspect::Wide16x9);
        // 16:9 frame: 85 game px past the screen each side; 4:3 covers all of it, 16:10 keeps 51 of it.
        assert_eq!(pillar_bars(Aspect::Wide16x9, Aspect::Original), Some([[-85, 0], [512, 597]]));
        assert_eq!(pillar_bars(Aspect::Wide16x9, Aspect::Wide16x10), Some([[-85, -51], [563, 597]]));
        assert_eq!(pillar_bars(Aspect::Wide16x9, Aspect::Wide16x9), None);
        assert_eq!(pillar_bars(Aspect::Original, Aspect::Original), None);
    }
}
