//! **Port-only: iOS start-up.** An iOS app has no launcher and no command line, so this does the launcher's part
//! (docs/plan/launcher_contract.md) inside the app:
//!
//! * **Folders.** The game data folder is `Library/Application Support/rerac/data` in the app's sandbox
//!   (`RC_DATA_DIR`, crate::disc_source); the port settings and the memory card live next to it
//!   (crate::render_settings, crate::saves). `Documents/` is the folder the Files app shows ("On My iPhone ›
//!   ReRAC"): the player copies the disc image there.
//! * **First run.** Without an extracted data folder the game cannot start, so [`startup`] runs a small setup
//!   app instead: it waits for a `.iso` in `Documents/`, extracts it with `rc_extract` on a worker thread (the same
//!   identify / copy / verify / prepare stages as `rerac-extract extract`), shows the progress, and then asks for a
//!   restart (iOS gives an app one event loop per process, so the game cannot start in the same run).
//! * **Defaults.** Full screen, and a 720p render resolution (`RC_RESOLUTION`) instead of the screen's native one.

use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::window::WindowMode;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The app sandbox's home (`HOME` is the sandbox container on iOS).
fn home() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")) }

/// The folder the Files app shows for this app.
pub fn documents() -> PathBuf { home().join("Documents") }

/// The extracted game data.
pub fn data_dir() -> PathBuf { home().join("Library/Application Support/rerac/data") }

/// The window every iOS run uses: full screen, the home indicator and status bar hidden.
pub fn window(title: &str) -> Window {
    Window {
        title: title.into(),
        mode: WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current),
        prefers_home_indicator_hidden: true,
        prefers_status_bar_hidden: true,
        ..default()
    }
}

#[link(name = "AVFAudio", kind = "framework")]
extern "C" {}

/// The game's sound plays as media: the Playback category of the shared `AVAudioSession`. cpal (under bevy_audio)
/// leaves the session alone, and the default category (SoloAmbient) is muted by the Ring / Silent switch.
fn audio_session() {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    use objc2_foundation::NSString;
    let Some(class) = AnyClass::get(c"AVAudioSession") else { return };
    let mut error: *mut AnyObject = std::ptr::null_mut();
    unsafe {
        let session: *mut AnyObject = msg_send![class, sharedInstance];
        if session.is_null() { return; }
        let category = NSString::from_str("AVAudioSessionCategoryPlayback");
        let ok: Bool = msg_send![session, setCategory: &*category, error: &mut error];
        let active: Bool = msg_send![session, setActive: Bool::YES, error: &mut error];
        if !ok.as_bool() || !active.as_bool() { eprintln!("audio: the AVAudioSession Playback category was refused"); }
    }
}

/// Sets the iOS defaults, then returns when the game data is ready; otherwise runs the setup app (never returns).
pub fn startup() {
    audio_session();
    let data = data_dir();
    let set_default = |k: &str, v: &std::ffi::OsStr| {
        if std::env::var_os(k).is_none_or(|x| x.is_empty()) {
            // Edition 2021, single-threaded here (before Bevy starts any thread).
            std::env::set_var(k, v);
        }
    };
    set_default("RC_DATA_DIR", data.as_os_str());
    set_default("RC_RESOLUTION", "720".as_ref());
    // An iOS window is always the whole screen; Windowed would only fight the system.
    set_default("RC_FULLSCREEN", "1".as_ref());
    let _ = std::fs::create_dir_all(documents());
    // A placeholder so the app's folder shows up in the Files app before anything is copied there.
    let readme = documents().join("COPY YOUR ISO HERE.txt");
    if !readme.exists() {
        let _ = std::fs::write(&readme, "Copy your Ratchet & Clank (NTSC-U, SCUS-97199) disc image here as a .iso file, then open ReRAC.\n");
    }
    if data.join(rc_extract::INFO_FILE).is_file() { return; }
    run_setup(data);
}

/// What the setup screen shows.
#[derive(Clone, Default)]
struct Status {
    headline: String,
    detail: String,
    progress: Option<f32>,
}

#[derive(Resource, Clone)]
struct Shared(Arc<Mutex<Status>>);

impl Shared {
    fn set(&self, f: impl FnOnce(&mut Status)) {
        let mut s = self.0.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut s);
    }
}

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct Bar;

/// The first `.iso` in Documents (or its `Inbox/`, where "Open in…" puts files), by name.
fn find_iso(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = [dir.to_path_buf(), dir.join("Inbox")]
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso")) && p.is_file())
        .collect();
    found.sort();
    found.into_iter().next()
}

fn extract_worker(shared: Shared, data: PathBuf) {
    use rc_extract::{build_db::BUILDS, extract, Code, Event};
    let iso = loop {
        if let Some(p) = find_iso(&documents()) { break p; }
        shared.set(|s| {
            s.headline = "Copia tu ISO de Ratchet & Clank / Copy your disc image".into();
            s.detail = "App Archivos › En mi iPhone › ReRAC  (NTSC-U, SCUS-97199, v1.00)\nFiles app › On My iPhone › ReRAC".into();
            s.progress = None;
        });
        std::thread::sleep(std::time::Duration::from_secs(2));
    };
    let name = iso.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    shared.set(|s| {
        s.headline = format!("Preparando / Preparing {name}");
        s.detail.clear();
    });
    let _ = std::fs::create_dir_all(&data);
    let opts = extract::Options { ntsc_only: true, threads: 2, cancel: None };
    let mut emit = |e: Event| match e {
        Event::Progress { stage, done, total, .. } => shared.set(|s| {
            s.detail = format!("{} {:.0} / {:.0} MiB", stage.name(), done as f64 / 1048576.0, total as f64 / 1048576.0);
            s.progress = (total > 0).then(|| done as f32 / total as f32);
        }),
        Event::Info(m) => println!("setup: {m}"),
        Event::Disc(d) => shared.set(|s| s.headline = format!("{} ({} {} v{})", d.title, d.serial, d.region, d.version)),
    };
    match extract::extract(&iso, &data, BUILDS, &opts, &mut emit) {
        Ok(_) => shared.set(|s| {
            s.headline = "¡Listo! Cierra ReRAC y vuelve a abrirlo / Done! Close and reopen ReRAC".into();
            s.detail = format!("Ya puedes borrar {name} desde la app Archivos para liberar espacio.\nYou can now delete {name} in the Files app.");
            s.progress = Some(1.0);
        }),
        Err(e) => shared.set(|s| {
            s.headline = match e.code {
                Code::NotRatchet | Code::Unsupported => "Disco no compatible / Unsupported disc".into(),
                Code::NoSpace => "No hay espacio suficiente / Not enough free space".into(),
                _ => "Error al preparar el juego / Setup failed".into(),
            };
            s.detail = e.message;
            s.progress = None;
        }),
    }
}

fn run_setup(data: PathBuf) -> ! {
    println!("setup: no game data at {}; waiting for a disc image in {}", data.display(), documents().display());
    let shared = Shared(Arc::new(Mutex::new(Status::default())));
    let worker = shared.clone();
    std::thread::spawn(move || extract_worker(worker, data));
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(window("ReRAC setup")), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.10)))
        .insert_resource(shared)
        .add_systems(Startup, spawn_setup_ui)
        .add_systems(Update, refresh_setup_ui)
        .run();
    std::process::exit(0)
}

fn spawn_setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(18.0),
            padding: UiRect::horizontal(Val::Px(70.0)),
            ..default()
        })
        .with_children(|p| {
            p.spawn((Text::new("ReRAC"), TextFont { font_size: FontSize::Px(40.0), ..default() }, TextColor(Color::srgb(1.0, 0.75, 0.2))));
            p.spawn((Text::new(""), TextFont { font_size: FontSize::Px(18.0), ..default() }, TextLayout::justify(Justify::Center), StatusText));
            p.spawn((Node { width: Val::Percent(70.0), height: Val::Px(10.0), ..default() }, BackgroundColor(Color::srgb(0.15, 0.17, 0.25))))
                .with_child((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::srgb(1.0, 0.75, 0.2)), Bar));
        });
}

fn refresh_setup_ui(shared: Res<Shared>, mut text: Single<&mut Text, With<StatusText>>, mut bar: Single<&mut Node, With<Bar>>) {
    let s = shared.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let want = format!("{}\n\n{}", s.headline, s.detail);
    if text.0 != want { text.0 = want; }
    let w = Val::Percent(s.progress.unwrap_or(0.0).clamp(0.0, 1.0) * 100.0);
    if bar.width != w { bar.width = w; }
}
