//! **Port-only: the "Preparing shaders" screen** (G-REN-037). Bevy compiles each render pipeline in the background the
//! first time it is used and draws nothing that needs one before it is ready; on a cold driver cache (the first start,
//! after a driver update) that can take seconds, and the window showed black or a flat purple title meanwhile. While
//! any pipeline has been waiting longer than [`SHOW_AFTER`], this screen covers the window: a progress bar (ready of
//! all pipelines the cache knows) and the pipelines still compiling, longest-waiting first, with how long each waited.
//! It is a Bevy UI node on the present camera (crate::display), so it shows even while the game frame's own pipelines
//! are not ready. While it is up the game holds (crate::frame_pace::HOLD: no ticks, so the game, its sound and the
//! movies stand still) and goes on where it was afterwards. It waits while a movie plays (the boot's Insomniac logos:
//! the shaders compile behind it) and shows after it if pipelines still wait. It only comes up in a load: the front
//! end, or the first [`LOAD_WINDOW`] seconds after the start or a level change, never in the middle of play (a
//! pipeline compiling then only keeps that one thing undrawn for a moment, not worth stopping the game); once up it
//! stays until the wave is done, at most [`MAX_HOLD`] seconds (a pipeline that never finishes cannot hold the game).
//! `RC_PIPELINE_OVERLAY=0` turns it off, `=always` keeps it up (to look at it).
//!
//! **The boot gate.** Bevy compiles on a few threads, first come first served: when the world's heavy pipelines are
//! queued first they can hold every thread, and the window's own (the present sprite, the UI this screen draws with)
//! wait behind them (a cold NVIDIA cache: black or this screen missing for a minute). So at boot every camera but the
//! present one stays inactive until the pipelines queued so far are ready (the window's and the UI's: a fraction of a
//! second), then they are switched back on and the world's pipelines queue behind a screen that can already draw.
//! `RC_BOOT_GATE=0` turns it off.
use bevy::prelude::*;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache, PipelineDescriptor};
use bevy::render::{Render, RenderApp, RenderSystems};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A pipeline must have waited this long (seconds) before the screen shows: a normal start never flashes it.
pub const SHOW_AFTER: f32 = 0.5;
/// The screen stays this long after the last pipeline is ready (seconds), so a run of short waves does not flicker.
pub const HIDE_AFTER: f32 = 0.4;
/// The seconds after the start or a level change that count as its load (the screen may come up in them).
pub const LOAD_WINDOW: f64 = 5.0;
/// The longest the screen holds the game (seconds): then it hides and the game goes on, whatever still compiles.
pub const MAX_HOLD: f64 = 180.0;
/// Pipelines listed at most.
pub const LIST_MAX: usize = 10;

/// What the render world saw this frame.
#[derive(Default, Clone)]
struct Snapshot {
    total: usize,
    ready: usize,
    failed: usize,
    /// (label, seconds waited), longest first.
    waiting: Vec<(String, f32)>,
}

#[derive(Resource, Clone, Default)]
struct Shared(Arc<Mutex<Snapshot>>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Off,
    Auto,
    Always,
}

#[derive(Resource)]
struct Overlay {
    mode: Mode,
    /// Real time the screen was last needed.
    last_needed: Option<f64>,
    /// Logged once per showing.
    shown: bool,
    /// The wall-clock time of the last `RC_PIPELINE_OVERLAY_LOG` line.
    last_log: f64,
    /// The wall-clock time of the start or the last level change (the load window's start).
    level_start: f64,
    /// When the screen came up (for [`MAX_HOLD`]), and whether this load gave up on it.
    shown_at: Option<f64>,
    gave_up: bool,
}

pub struct PipelineOverlayPlugin;

impl Plugin for PipelineOverlayPlugin {
    fn build(&self, app: &mut App) {
        let mode = match std::env::var("RC_PIPELINE_OVERLAY").as_deref().map(str::trim) {
            Ok("0") => Mode::Off,
            Ok("always") => Mode::Always,
            _ => Mode::Auto,
        };
        println!("pipeline overlay: {mode:?} (\"Preparing shaders\" while a pipeline waits over {SHOW_AFTER} s; RC_PIPELINE_OVERLAY=0|always)");
        let shared = Shared::default();
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(shared.clone()).add_systems(Render, observe.in_set(RenderSystems::Cleanup));
        }
        app.insert_resource(shared)
            .insert_resource(Overlay { mode, last_needed: None, shown: false, last_log: 0.0, level_start: 0.0, shown_at: None, gave_up: false })
            .add_systems(Startup, spawn)
            .add_systems(Update, (target_present_camera, update))
            .insert_resource(BootGate { open: std::env::var("RC_BOOT_GATE").is_ok_and(|v| v.trim() == "0"), frames: 0, held: Vec::new() })
            .add_systems(PostUpdate, boot_gate);
    }
}

fn label(d: &PipelineDescriptor) -> &str {
    match d {
        PipelineDescriptor::RenderPipelineDescriptor(d) => d.label.as_deref().unwrap_or("(unlabelled)"),
        PipelineDescriptor::ComputePipelineDescriptor(d) => d.label.as_deref().unwrap_or("(unlabelled)"),
    }
}

/// Render world: the cache's pipelines, ready / failed / waiting (with how long each has waited, from the first frame
/// it was seen waiting).
fn observe(cache: Res<PipelineCache>, shared: Res<Shared>, mut since: Local<HashMap<usize, Instant>>) {
    let now = Instant::now();
    let mut s = Snapshot::default();
    for (i, p) in cache.pipelines().enumerate() {
        s.total += 1;
        let name = label(&p.descriptor);
        match &p.state {
            CachedPipelineState::Ok(_) => {
                s.ready += 1;
                since.remove(&i);
            }
            CachedPipelineState::Err(e) if !matches!(e, bevy::shader::ShaderCacheError::ShaderNotLoaded(_) | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable) => s.failed += 1,
            _ => {
                let t0 = *since.entry(i).or_insert(now);
                s.waiting.push((name.to_string(), now.duration_since(t0).as_secs_f32()));
            }
        }
    }
    s.waiting.sort_by(|a, b| b.1.total_cmp(&a.1));
    *shared.0.lock().unwrap_or_else(|e| e.into_inner()) = s;
}

#[derive(Component)]
struct OverlayRoot;
#[derive(Component)]
struct OverlayCount;
#[derive(Component)]
struct OverlayBar;
#[derive(Component)]
struct OverlayList;

fn spawn(mut commands: Commands) {
    let text = |size: f32, c: Color| (TextFont::from_font_size(size), TextColor(c));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.02, 0.02, 0.05)),
            GlobalZIndex(i32::MAX - 1),
            Visibility::Hidden,
            OverlayRoot,
            crate::level_switch::KeepAcrossLevels,
            Name::new("pipeline overlay"),
        ))
        .with_children(|b| {
            b.spawn((Text::new("Preparing shaders"), text(34.0, Color::srgb(0.95, 0.85, 0.45))));
            b.spawn((Text::new("The first start after an install or a driver update compiles the game's shaders once."), text(16.0, Color::srgb(0.7, 0.7, 0.75))));
            // the bar: a frame, its fill
            b.spawn((
                Node { width: Val::Px(560.0), height: Val::Px(22.0), border: UiRect::all(Val::Px(2.0)), padding: UiRect::all(Val::Px(2.0)), ..default() },
                BorderColor::all(Color::srgb(0.6, 0.6, 0.7)),
                BackgroundColor(Color::srgb(0.08, 0.08, 0.12)),
            ))
            .with_children(|f| {
                f.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::srgb(0.35, 0.75, 1.0)), OverlayBar));
            });
            b.spawn((Text::new(""), text(20.0, Color::srgb(0.9, 0.9, 0.95)), OverlayCount));
            b.spawn((
                Text::new(""),
                text(15.0, Color::srgb(0.6, 0.85, 0.6)),
                TextLayout::justify(Justify::Left),
                Node { width: Val::Px(560.0), ..default() },
                OverlayList,
            ));
        });
}

/// On the present camera (the window), not the game frame's: it must show while the game frame cannot draw.
fn target_present_camera(mut commands: Commands, nodes: Query<Entity, (With<OverlayRoot>, Without<UiTargetCamera>)>, cams: Query<Entity, With<crate::display::PresentCamera>>) {
    let Some(cam) = cams.iter().next() else { return };
    for n in &nodes { commands.entity(n).insert(UiTargetCamera(cam)); }
}

fn update(
    shared: Res<Shared>,
    movie: Option<Res<crate::movie_render::MovieState>>,
    generation: Option<Res<crate::level_switch::LevelGeneration>>,
    mut overlay: ResMut<Overlay>,
    mut root: Query<&mut Visibility, With<OverlayRoot>>,
    mut bar: Query<&mut Node, With<OverlayBar>>,
    mut count: Query<&mut Text, (With<OverlayCount>, Without<OverlayList>)>,
    mut list: Query<&mut Text, (With<OverlayList>, Without<OverlayCount>)>,
) {
    let Ok(mut vis) = root.single_mut() else { return };
    let s = shared.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
    // The wall clock, not Bevy's: holding the game (no ticks) stops `Time<Real>` too, and the screen could never hide.
    static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let now = T0.get_or_init(Instant::now).elapsed().as_secs_f64();
    if generation.as_ref().is_some_and(|g| g.is_changed() && !g.is_added()) {
        overlay.level_start = now;
        overlay.gave_up = false;
    }
    // A load: the front end, or the first LOAD_WINDOW seconds after the start or a level change; once up, the screen
    // stays for the rest of the wave (it ends with the wave or at MAX_HOLD).
    let in_load = crate::saves::front_end_active() || now - overlay.level_start < LOAD_WINDOW || overlay.shown;
    let waited = s.waiting.first().is_some_and(|w| w.1 >= SHOW_AFTER);
    let needed = match overlay.mode {
        Mode::Off => false,
        Mode::Always => true,
        // (not over a movie: it needs no world pipeline and the compiles go on behind it)
        Mode::Auto => waited && in_load && !overlay.gave_up && !movie.as_ref().is_some_and(|m| m.busy()),
    };
    if !waited && !crate::saves::front_end_active() && now - overlay.level_start >= LOAD_WINDOW { overlay.gave_up = false; }
    if needed { overlay.last_needed = Some(now); }
    let mut show = overlay.mode != Mode::Off && overlay.last_needed.is_some_and(|t| now - t < HIDE_AFTER as f64);
    if show && overlay.mode == Mode::Auto {
        let t0 = *overlay.shown_at.get_or_insert(now);
        // (`RC_PIPELINE_OVERLAY_MAX_HOLD=<seconds>` overrides it, to test the cap)
        static CAP: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
        let cap = *CAP.get_or_init(|| std::env::var("RC_PIPELINE_OVERLAY_MAX_HOLD").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(MAX_HOLD));
        if now - t0 >= cap {
            println!("pipeline overlay: up {cap:.0} s with {} still compiling: hidden, the game goes on (their draws wait)", s.waiting.len());
            overlay.gave_up = true;
            overlay.last_needed = None;
            show = false;
        }
    }
    if !show { overlay.shown_at = None; }
    // The game holds while the screen is up (never in frame-exact runs: crate::frame_pace does not pace those).
    crate::frame_pace::HOLD.store(show && overlay.mode == Mode::Auto, std::sync::atomic::Ordering::Relaxed);
    if show != overlay.shown {
        println!("pipeline overlay: {} ({} of {} ready, {} waiting{})", if show { "shown, the game holds" } else { "hidden, the game goes on" }, s.ready, s.total, s.waiting.len(),
            s.waiting.first().map_or(String::new(), |w| format!(", longest {:.1} s: {}", w.1, w.0)));
        overlay.shown = show;
    }
    // RC_PIPELINE_OVERLAY_LOG=1: what it lists, every 5 s while shown (when the screen itself cannot draw).
    static LOG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if show && *LOG.get_or_init(|| std::env::var("RC_PIPELINE_OVERLAY_LOG").is_ok_and(|v| v == "1")) && (now / 5.0).floor() != (overlay.last_log / 5.0).floor() {
        overlay.last_log = now;
        println!("pipeline overlay: {:.0} s: {} of {} ready; waiting: {}", now, s.ready, s.total,
            s.waiting.iter().filter(|(n, _)| !n.contains("_mesh_pipeline")).take(16).map(|(n, a)| format!("{n} {a:.1}s")).collect::<Vec<_>>().join(", "));
    }
    let want = if show { Visibility::Visible } else { Visibility::Hidden };
    if *vis != want { *vis = want; }
    if !show { return; }
    let frac = if s.total == 0 { 0.0 } else { s.ready as f32 / s.total as f32 };
    if let Ok(mut n) = bar.single_mut() { n.width = Val::Percent(frac * 100.0); }
    let mut line = format!("{} of {} pipelines ready ({:.0}%), {} compiling", s.ready, s.total, frac * 100.0, s.waiting.len());
    if s.failed > 0 { line += &format!(", {} failed", s.failed); }
    if let Ok(mut t) = count.single_mut() { if t.0 != line { t.0 = line; } }
    let mut l = String::new();
    // (several pipelines share a label: one line per label, with their count)
    let mut seen: Vec<(String, f32, usize)> = Vec::new();
    for (name, age) in &s.waiting {
        match seen.iter_mut().find(|e| &e.0 == name) {
            Some(e) => e.2 += 1,
            None => seen.push((name.clone(), *age, 1)),
        }
    }
    for (name, age, n) in seen.iter().take(LIST_MAX) {
        l += &format!("{age:6.1} s   {name}{}\n", if *n > 1 { format!("  (x{n})") } else { String::new() });
    }
    if seen.len() > LIST_MAX { l += &format!("          … and {} more\n", seen.len() - LIST_MAX); }
    if let Ok(mut t) = list.single_mut() { if t.0 != l { t.0 = l; } }
}

/// The boot gate (module docs): open once the pipelines queued with only the present camera active are ready.
#[derive(Resource)]
struct BootGate {
    open: bool,
    frames: u32,
    /// The cameras it switched off (switched back on when it opens).
    held: Vec<Entity>,
}

/// Frames the gate waits at least (the present camera's and the UI's pipelines are queued in the first ones).
const GATE_MIN_FRAMES: u32 = 3;

fn boot_gate(mut gate: ResMut<BootGate>, shared: Res<Shared>, time: Res<Time<Real>>, mut cams: Query<(Entity, &mut Camera), Without<crate::display::PresentCamera>>) {
    if gate.open { return; }
    gate.frames += 1;
    let (total, waiting) = {
        let s = shared.0.lock().unwrap_or_else(|e| e.into_inner());
        (s.total, s.waiting.len())
    };
    if gate.frames > GATE_MIN_FRAMES && total > 0 && waiting == 0 {
        for e in std::mem::take(&mut gate.held) {
            if let Ok((_, mut c)) = cams.get_mut(e) { c.is_active = true; }
        }
        gate.open = true;
        println!("pipeline overlay: boot gate open after {} frames, {:.2} s ({total} pipelines ready); the game cameras are on", gate.frames, time.elapsed_secs_f64());
        return;
    }
    for (e, mut c) in &mut cams {
        if c.is_active {
            c.is_active = false;
            gate.held.push(e);
        }
    }
}
