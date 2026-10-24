//! **Port-only: the game frame and its presentation.** The PS2 drew one 512×416 buffer; the port draws the whole
//! game (world, sky, HUD, menus, canvases) into one offscreen image of that shape, [`GameFrame`], and a last camera
//! ([`PresentCamera`]) draws it into the window, scaled to fit and centred on black. Every camera that would render
//! to the primary window is pointed at the frame as it is spawned ([`redirect`]), so nothing else needs to know
//! the window's size: overlays laid out on the frame (the HUD composite, the menu layer and its snapshot, the item
//! canvases) match it at any window size, the menu snapshot copies the frame without bars, and a screenshot is the
//! frame itself.
//!
//! **Aspect.** The PS2 drew 512×416 and the TV showed it at 4:3; the frame is 4:3 ([`Aspect::Original`], the
//! default), 16:10 ([`Aspect::Wide16x10`], a MacBook's screen), 16:9 ([`Aspect::Wide16x9`]) or the window's own
//! ([`Aspect::Fill`]: the frame is the window's shape — an ultrawide monitor's 3440×1440, whose 43:18 is not the 21:9
//! the label suggests — never narrower than 4:3 ([`MIN_RATIO`], so a window narrower than the TV's is letterboxed
//! rather than squashed). The game's 512×416 screen (the HUD, the menus, the item canvases) always maps onto the
//! frame's centred 4:3 box ([`ui_box`]: the whole frame at 4:3) and the 3D view widens around it: the world cameras'
//! horizontal tangent is the game's ×6/5 at 16:10, ×4/3 at 16:9, the frame's ratio ÷ 4:3 at Fill ([`hor_scale`], Hor+:
//! the vertical view is the game's).
//!
//! **Settings** ([`DisplaySettings`], the Port Options page; persisted in the port settings file,
//! `crate::render_settings::save_key`): the aspect, the render resolution — "Window" (the largest frame of the aspect
//! that fits the window, in physical pixels) or a frame of a fixed height (416p, the PS2's lines, or 720p / 1080p /
//! 1440p / 2160p; the width follows the aspect) scaled to the window — and fullscreen (borderless on the current
//! monitor). `RC_ASPECT=4:3|16:10|16:9|fill`, `RC_RESOLUTION=window|416|720|1080|1440|2160` and
//! `RC_FULLSCREEN=0|1` override the file at start; F11 toggles fullscreen and saves it.
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

impl GameFrame {
    /// The frame's width / height: its effective aspect (the window's own for [`Aspect::Fill`]). Floored at
    /// [`MIN_RATIO`]: a frame whose pixels round a hair narrower than the game's 4:3 is treated as 4:3, so the UI is
    /// never squashed and [`side_extra`] stays ≥ 0.
    pub fn ratio(&self) -> f32 { (self.size.x.max(1) as f32 / self.size.y.max(1) as f32).max(MIN_RATIO) }
}

/// The game's own shape, 512×416 shown at 4:3: the narrowest the frame may be. [`Aspect::Fill`] in a narrower window
/// keeps this ratio and letterboxes (top / bottom bars), so the 512×416 screen's UI and menus are never squashed.
pub const MIN_RATIO: f32 = 4.0 / 3.0;

/// The frame's shape: the TV's 4:3, a wide screen's 16:10 / 16:9, or the window's own (the Port Options row's order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aspect {
    Original,
    Wide16x10,
    Wide16x9,
    Fill,
}

impl Aspect {
    /// The Port Options row's values, in order.
    pub const ALL: [Aspect; 4] = [Aspect::Original, Aspect::Wide16x10, Aspect::Wide16x9, Aspect::Fill];
    pub fn index(self) -> u8 { Self::ALL.iter().position(|&a| a == self).unwrap_or(0) as u8 }
    pub fn from_index(i: u8) -> Self { Self::ALL.get(i as usize).copied().unwrap_or(Aspect::Original) }
    /// Width / height: the fixed ratio, or the `window`'s own for [`Aspect::Fill`], never below [`MIN_RATIO`] (the
    /// game's 4:3: a narrower window is letterboxed rather than squashed).
    pub fn ratio(self, window: UVec2) -> f32 {
        match self {
            Aspect::Original => 4.0 / 3.0,
            Aspect::Wide16x10 => 16.0 / 10.0,
            Aspect::Wide16x9 => 16.0 / 9.0,
            Aspect::Fill => (window.x.max(1) as f32 / window.y.max(1) as f32).max(MIN_RATIO),
        }
    }
    /// The value for a settings-file / `RC_ASPECT` string, case-insensitively (`fill` or `Fill`).
    fn parse(v: &str) -> Option<Self> { let v = v.trim(); Self::ALL.into_iter().find(|a| a.key().eq_ignore_ascii_case(v)) }
    fn key(self) -> &'static str {
        match self {
            Aspect::Original => "4:3",
            Aspect::Wide16x10 => "16:10",
            Aspect::Wide16x9 => "16:9",
            Aspect::Fill => "fill",
        }
    }
}

/// The factor on the game camera's horizontal tangent (0.63) for a frame of width/height `ratio`: 1 at 4:3, 6/5 at
/// 16:10, 4/3 at 16:9, 43/24 at an ultrawide's ratio of 43/18 (3440×1440): the frame is that much wider at the same
/// height, and the view keeps the TV's proportions.
pub fn hor_scale(ratio: f32) -> f32 { ratio / (4.0 / 3.0) }

/// The game pixels the frame extends past the 512-wide game screen on each side (0 at 4:3, 51 at 16:10, 85 at 16:9,
/// the frame's own margin at Fill): the HUD's layer is that much wider (crate::hud_render), its left / right slots
/// move out by it (`rc_game::hud`).
pub fn side_extra(ratio: f32) -> i32 { ((SCREEN_W * hor_scale(ratio) - SCREEN_W) * 0.5).round() as i32 }

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
    pub aspect: Aspect,
    pub resolution: Resolution,
    pub fullscreen: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self { DisplaySettings { aspect: Aspect::Original, resolution: Resolution::Window, fullscreen: false } }
}

impl DisplaySettings {
    /// The settings file's values, then `RC_ASPECT` / `RC_RESOLUTION` / `RC_FULLSCREEN` over them.
    pub fn startup() -> Self {
        let mut s = DisplaySettings::default();
        if let Some(a) = crate::render_settings::load_key("aspect").as_deref().and_then(Aspect::parse) { s.aspect = a; }
        if let Some(a) = std::env::var("RC_ASPECT").ok().as_deref().and_then(Aspect::parse) { s.aspect = a; }
        if let Some(r) = crate::render_settings::load_key("resolution").as_deref().and_then(Resolution::parse) { s.resolution = r; }
        if let Some(f) = crate::render_settings::load_key("fullscreen") { s.fullscreen = f.trim() == "1"; }
        if let Some(r) = std::env::var("RC_RESOLUTION").ok().as_deref().and_then(Resolution::parse) { s.resolution = r; }
        if let Ok(f) = std::env::var("RC_FULLSCREEN") { s.fullscreen = f.trim() == "1"; }
        s
    }

    /// Writes the keys into the port settings file.
    pub fn save(&self) {
        crate::render_settings::save_key("aspect", self.aspect.key());
        crate::render_settings::save_key("resolution", &self.resolution.key());
        crate::render_settings::save_key("fullscreen", if self.fullscreen { "1" } else { "0" });
    }
}

/// The frame's size for `res` and `aspect` in a window of `window` physical pixels: the largest rectangle of the
/// aspect that fits, or one of the chosen height.
pub fn frame_size(res: Resolution, aspect: Aspect, window: UVec2) -> UVec2 {
    let a = aspect.ratio(window);
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

/// The frame's centred 4:3 box (origin and size, frame pixels): where the game's 512×416 screen lands.
pub fn ui_box(frame: UVec2) -> (Vec2, Vec2) {
    let h = frame.y as f32;
    let w = (h * 4.0 / 3.0).round().min(frame.x as f32);
    (Vec2::new(((frame.x as f32 - w) * 0.5).round(), 0.0), Vec2::new(w, h))
}

/// The largest rectangle of the frame's aspect that fits `window` (logical units): the present sprite's size.
pub fn fit(frame: UVec2, window: Vec2) -> Vec2 {
    let (fw, fh) = (frame.x.max(1) as f32, frame.y.max(1) as f32);
    // Cross-multiplied (no aspect quotient): a window of the frame's own aspect gets exactly its size.
    if window.x * fh > window.y * fw { Vec2::new(window.y * fw / fh, window.y) } else { Vec2::new(window.x, window.x * fh / fw) }
}

/// A UI node kept on the frame's 4:3 box ([`ui_box`]; percent of its parent, which must span the frame): the HUD
/// composite (and the canvases composed with it, its children), the movie picture.
#[derive(Component)]
pub struct UiBoxed;

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
        println!("display: aspect {:?}, render resolution {:?}, fullscreen {} (Port Options; RC_ASPECT / RC_RESOLUTION / RC_FULLSCREEN)", settings.aspect, settings.resolution, settings.fullscreen);
        app.insert_resource(GameFrame { image, size })
            .insert_resource(settings)
            .add_observer(redirect)
            .add_systems(Startup, spawn_present)
            .add_systems(First, sync_frame)
            .add_systems(Update, (toggle_fullscreen, apply_mode, fit_present).chain())
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
    let want = frame_size(settings.resolution, settings.aspect, ws);
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

/// The [`UiBoxed`] nodes on the 4:3 box.
fn place_boxed(frame: Res<GameFrame>, mut nodes: Query<&mut Node, With<UiBoxed>>) {
    let (o, s) = ui_box(frame.size);
    let fw = frame.size.x.max(1) as f32;
    let want = (Val::Percent(o.x * 100.0 / fw), Val::Percent(s.x * 100.0 / fw));
    for mut n in &mut nodes {
        if (n.left, n.width) != want {
            (n.left, n.width) = want;
            (n.top, n.height) = (Val::Px(0.0), Val::Percent(100.0));
        }
    }
}

/// The [`BoxedView`] cameras' viewport: the 4:3 box (none when it is the whole frame).
fn box_views(frame: Res<GameFrame>, mut cams: Query<&mut Camera, With<BoxedView>>) {
    let (o, s) = ui_box(frame.size);
    let want = (s.x < frame.size.x as f32).then(|| bevy::camera::Viewport { physical_position: o.as_uvec2(), physical_size: s.as_uvec2().max(UVec2::ONE), ..default() });
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
pub fn view_tans(game_tan: f32, ratio: f32) -> (f32, f32) { (game_tan * hor_scale(ratio), game_tan * crate::game_camera::NTSC_Y_RATIO) }

/// Sets `p`'s tangents to [`view_tans`] of `game_tan` when they differ; true when it changed them.
pub fn set_view_tans(p: &mut Projection, game_tan: f32, ratio: f32) -> bool {
    let (tx, ty) = view_tans(game_tan, ratio);
    let Projection::Custom(c) = p else { return false };
    let Some(g) = c.get_mut::<crate::game_camera::GameProjection>() else { return false };
    if g.tan_x == tx && g.tan_y == ty { return false; }
    (g.tan_x, g.tan_y) = (tx, ty);
    true
}

/// The fly camera (no game camera writes the projection): the game's default tangent through [`view_tans`].
fn fly_view(frame: Res<GameFrame>, source: Option<Res<crate::game_camera::CameraSource>>, mut cams: Query<&mut Projection, With<crate::fly_cam::FlyCam>>) {
    if source.is_some_and(|s| *s == crate::game_camera::CameraSource::Play) { return; }
    for mut p in &mut cams { set_view_tans(&mut p, crate::game_camera::TAN_HALF_FOV_X, frame.ratio()); }
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
        // Fill: the frame is the window itself (a 3440×1440 ultrawide), or the same shape at a fixed height.
        assert_eq!(frame_size(Resolution::Window, Aspect::Fill, UVec2::new(3440, 1440)), UVec2::new(3440, 1440));
        assert_eq!(frame_size(Resolution::Height(1080), Aspect::Fill, UVec2::new(3440, 1440)), UVec2::new(2580, 1080));
        // A window narrower than 4:3 keeps the game's shape and letterboxes (the frame is shorter than the window).
        assert_eq!(Aspect::Fill.ratio(UVec2::new(1000, 1200)), MIN_RATIO);
        assert_eq!(frame_size(Resolution::Window, Aspect::Fill, UVec2::new(1000, 1200)), UVec2::new(1000, 750));
        // The frame's own ratio is floored too: pixels that round a hair narrower than 4:3 are treated as 4:3.
        let frame = |size: UVec2| GameFrame { image: Handle::default(), size };
        assert_eq!(frame(UVec2::new(101, 76)).ratio(), MIN_RATIO);
        assert_eq!(frame(UVec2::new(3440, 1440)).ratio(), 3440.0 / 1440.0);
        assert_eq!(side_extra(16.0 / 10.0), 51);
        assert_eq!(side_extra(16.0 / 9.0), 85);
        assert_eq!(Resolution::parse("720"), Some(Resolution::Height(720)));
        assert_eq!(ui_box(UVec2::new(2560, 1440)), (Vec2::new(320.0, 0.0), Vec2::new(1920.0, 1440.0)));
        assert_eq!(ui_box(UVec2::new(1024, 768)), (Vec2::ZERO, Vec2::new(1024.0, 768.0)));
        assert_eq!(hor_scale(4.0 / 3.0), 1.0);
        assert!((hor_scale(16.0 / 10.0) - 6.0 / 5.0).abs() < 1e-6);
        assert!((hor_scale(16.0 / 9.0) - 4.0 / 3.0).abs() < 1e-6);
        assert_eq!(fit(UVec2::new(1024, 832), Vec2::new(1600.0, 832.0)), Vec2::new(1024.0, 832.0));
        assert_eq!(fit(UVec2::new(1024, 832), Vec2::new(1024.0, 1000.0)), Vec2::new(1024.0, 832.0));
        for r in Resolution::ALL { assert_eq!(Resolution::from_index(r.index()), r); assert_eq!(Resolution::parse(&r.key()), Some(r)); }
        for a in Aspect::ALL { assert_eq!(Aspect::from_index(a.index()), a); assert_eq!(Aspect::parse(a.key()), Some(a)); }
        // `RC_ASPECT` / the settings file are read case-insensitively (`Fill` as the menu shows it).
        assert_eq!(Aspect::parse("Fill"), Some(Aspect::Fill));
        assert_eq!(Aspect::parse(" fill "), Some(Aspect::Fill));
    }
}
