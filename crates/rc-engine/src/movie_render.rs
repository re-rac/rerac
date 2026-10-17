//! PSS movies (FMV, game mode 1) played natively from the original files at runtime (decision U10): the request
//! (`DialogStreamUpdate(n)` → `StartPssMovie`), the frame sequence and skip rule of `rc_game::movie_player`, the
//! pictures decoded by `rc_video` on a worker thread, the audio mixed into crate::audio_out, and a full-screen
//! picture. Spec: docs/plan/cutscenes.md §5, docs/formats/pss.md.
//!
//! * **Request.** crate::scene_render hands over the classes' `EngineRequest::StartMovie` and the talkers'
//!   `Handoff::Movie` through [`request_in_level`]: in-level movie n is `global/mpegs/{2 + n}.bin` (NTSC). From
//!   that frame on the gameplay tick is suspended (mode 1; `GameTick` gets a `run_if`, the menu mode is
//!   `Mode::Movie`), as `StartPssMovie` leaves the moby loop's tick and the main loop runs `MovieModeUpdate`.
//! * **Frames** ([`movie_frame`], `FixedUpdate` after the tick, one call per 60 Hz vsync): `FadeToBlack(4)` over
//!   the last game image and its closing black vsync, the movie (the audio is the clock: picture
//!   `(800·k − latency)·fps / 48000` at vsync k), `FadeToBlack(4)` over the last picture, then mode 0:
//!   `music_start_track(track, 1, 0x400)` and the talker's dialogue refresh (`MovieExitToGameplay` 0x2ad2b8).
//!   Skip: `readMpeg`'s rule (Start + L1 L2 R1 R2, or Start alone once the game is beaten / completed / on
//!   level 0), from the first movie vsync.
//! * **Decoding.** A worker thread reads the file, demuxes it, decodes the audio channel of the game language
//!   (channel 0 when the file has no such channel) and then the pictures in display order, converted to RGBA,
//!   through a 4-deep channel. The frame that needs picture p waits for it: frame-exact runs are identical, and a
//!   live run only waits when decoding falls behind (it runs at 20–30× real time in a dev build).
//! * **Audio.** `StartPssMovie` / `MovieModeUpdate` stop all sounds and the music (`AudioSystem::movie_stop`);
//!   each movie vsync outputs the next 800 samples of the movie's 48 kHz audio through `AudioSystem::mix_movie`
//!   (the two SPU movie voices, group 5 at the sfx option), the fades output silence; the exit restarts the level
//!   track (`AudioSystem::movie_exit`). The latency used by the clock is the audio ring's fill (0 in frame-exact
//!   runs, so their PNGs and WAVs do not depend on the device).
//! * **Picture.** The movie is 512×416, the size of the GS draw buffer, and `setImageTag` 0x31b0b8 uploads it at
//!   (0, 0) of the display buffer: it fills exactly the picture the game's frames fill. The port shows it as a UI
//!   image over the main camera, nearest-sampled, above the HUD, stretched to the cutscene aspect's box of the frame
//!   (crate::display::CutsceneBoxed: the 4:3 box, or the whole wide frame); the black fade is a UI quad whose alpha makes the
//!   linear-light blend give `C·(1 − coverage)` in display bytes, like the GS's black quad.
//!
//! * **Other callers** (docs/plan/progression.md `## media`): [`request`] with [`MovieRequest::movie_b`] (`PlayMovieB`:
//!   the Cinematics page, the ending `PlayMovieB(11)`), [`MovieRequest::movie_c`] (`PlayMovieC`: the extras, language 0),
//!   [`play_attract`] (the title's attract loop, boot `fun_001e9488`: back to the interrupted mode, no music restart),
//!   and the page menu's replays ([`MovieRequest::after_menu_fade`]: its `FadeToBlack(ticks(16))` before, the replay mode
//!   0x15eed8 in [`crate::menu_render::MenuMode::replay`]).
//!
//! Environment (dev checks, not game options):
//! - `RC_PLAY_MOVIE=<n>`: in-level movie n after gameplay tick 1 (as `RC_SCENE`), e.g. 3 = the Novalis holofilm.
//! - `RC_MOVIE_SKIP=<k>`: press Start with L1 L2 R1 R2 held on movie-mode frame k (the skip test).
//! - `RC_MOVIE_TRACE=1`: one line per 30 pictures and per phase change.

use crate::fly_cam::FlyCam;
use crate::gameplay::{GameTick, Persistent, Play};
use crate::input_map::PadFrame;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::UiTargetCamera;
use rc_game::menus::mode::Mode;
use rc_game::moby_runtime::MobyId;
use rc_game::movie_player::{MovieContext, MovieFrame, MoviePlayer, Phase, MOVIE_ATTRACT_BASE_NTSC, MOVIE_BASE_NTSC, MOVIE_B_BASE_NTSC, MOVIE_C_BASE_NTSC, SAMPLES_PER_VSYNC};
use rc_game::pad::{button, PadState};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Mutex;

/// A movie the game asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovieRequest {
    /// `mpegs[file]` = `global/mpegs/NNN.bin`.
    pub file: u32,
    /// The in-level movie number n (`DialogStreamUpdate(n)`), for the logs.
    pub movie: i32,
    /// The talker whose dialogue continues after the movie (0x179588).
    pub talker: Option<MobyId>,
    /// The audio language (`StartPssMovie`'s third argument: 0x15ed88, or 0 for `PlayMovieC` and the attract movies).
    pub language: Option<u8>,
    /// The caller faded the last image to black before (the page menu's `FadeToBlack(ticks(16))`), and the entry fade
    /// (4; the attract movies 12).
    pub black_entry: bool,
    pub fade_in: u32,
    /// The replay mode 0x15eed8 the caller sets for this movie and clears after it (the attract movies: 2); None: the
    /// global as it is ([`crate::menu_render::MenuMode::replay`], the page menu's post-actions set it).
    pub replay: Option<i32>,
    /// Who the movie returns to.
    pub exit: MovieExit,
}

/// What follows the movie.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MovieExit {
    /// `MovieExitToGameplay` 0x2ad2b8: mode 0, `music_start_track(level track, 1, 0x400)`, the talker refreshed.
    #[default]
    Gameplay,
    /// The title's attract movie (boot `fun_001e9488`): the mode it interrupted again, no music restart.
    #[allow(dead_code)] // the saves lane's title plays the attract movies ([`play_attract`])
    FrontEnd,
}

impl MovieRequest {
    /// `DialogStreamUpdate(n)` 0x2acf50: in-level movie n = `mpegs[2 + n]`, the game language.
    pub fn in_level(n: i32, talker: Option<MobyId>) -> MovieRequest {
        MovieRequest { file: (MOVIE_BASE_NTSC + n) as u32, movie: n, talker, language: None, black_entry: false, fade_in: rc_game::movie_player::FADE_FRAMES, replay: None, exit: MovieExit::Gameplay }
    }
    /// `PlayMovieB(n)` 0x2ad050: `mpegs[40 + n]`, the game language.
    pub fn movie_b(n: i32) -> MovieRequest { MovieRequest { file: (MOVIE_B_BASE_NTSC + n) as u32, ..MovieRequest::in_level(n, None) } }
    /// `PlayMovieC(n)` 0x2acfe8: `mpegs[70 + n]`, language 0.
    pub fn movie_c(n: i32) -> MovieRequest { MovieRequest { file: (MOVIE_C_BASE_NTSC + n) as u32, language: Some(0), ..MovieRequest::in_level(n, None) } }
    /// Boot `fun_001e9488(i)`: the title's attract movie `mpegs[80 + i]` (replay 2: any button skips; language 0; entry
    /// fade `ticks(12)`; back to the title).
    #[allow(dead_code)] // the saves lane's title plays the attract movies ([`play_attract`])
    pub fn attract(i: i32) -> MovieRequest {
        MovieRequest {
            file: (MOVIE_ATTRACT_BASE_NTSC + i) as u32,
            language: Some(0),
            fade_in: rc_game::movie_player::ATTRACT_FADE,
            replay: Some(2),
            exit: MovieExit::FrontEnd,
            ..MovieRequest::in_level(i, None)
        }
    }
    /// After the page menu's `FadeToBlack(ticks(16))` (drawn by crate::menu_render over the menu image).
    pub fn after_menu_fade(mut self) -> MovieRequest {
        self.black_entry = true;
        self
    }
}

static QUEUE: Mutex<VecDeque<MovieRequest>> = Mutex::new(VecDeque::new());

fn queue() -> std::sync::MutexGuard<'static, VecDeque<MovieRequest>> { QUEUE.lock().unwrap_or_else(|e| e.into_inner()) }

/// `DialogStreamUpdate(n)` 0x2acf50 → `StartPssMovie` 0x2ad0c0: in-level movie n (`mpegs[2 + n]`, NTSC); a
/// negative n is ignored as in the game. The movie starts this frame (the rest of the tick that asked has run).
pub fn request_in_level(n: i32, talker: Option<MobyId>) {
    if n < 0 { return; }
    request(MovieRequest::in_level(n, talker));
}

/// Any movie (`PlayMovieB` / `PlayMovieC` / the attract movies / the page menu's replays: [`MovieRequest`]'s
/// constructors). A negative movie number is ignored as in the game (`if (-1 < n)`). It starts on the next movie frame.
pub fn request(req: MovieRequest) {
    if req.movie < 0 { return; }
    queue().push_back(req);
}

/// The title's attract movie i (boot `fun_001e9488`, the `saves` lane's title calls it after its idle time): see
/// [`MovieRequest::attract`]. [`MovieState::busy`] tells when it is over.
#[allow(dead_code)] // its caller is the saves lane's title (docs/plan/progression.md `## media`)
pub fn play_attract(i: i32) { request(MovieRequest::attract(i)); }

/// A movie is queued or playing: the gameplay tick is suspended.
fn blocking(state: &MovieState) -> bool { state.running.is_some() || !queue().is_empty() }

/// Messages of the decoder thread.
enum Msg {
    Opened { fps: (u32, u32), audio: Option<Vec<[i16; 2]>>, info: String },
    Picture(u32, Vec<u8>),
    End(u32),
    Failed(String),
}

struct Running {
    req: MovieRequest,
    player: MoviePlayer,
    /// In a mutex only to make the resource `Sync` (only this system touches it: `get_mut`, no locking).
    rx: Mutex<Receiver<Msg>>,
    opened: bool,
    audio: Option<Vec<[i16; 2]>>,
    /// Pictures in the stream, once the decoder reached the end.
    total: Option<u32>,
    /// Pictures received so far; the last one is [`Running::rgba`].
    received: u32,
    rgba: Vec<u8>,
    /// The picture on screen (uploaded).
    shown: Option<u32>,
    pad: PadState,
    /// Movie-mode frames so far.
    frames: u32,
    last_phase: Option<Phase>,
}

/// The movie state, published for the tick's run condition and the reports.
#[derive(Resource, Default)]
pub struct MovieState {
    running: Option<Running>,
    image: Handle<Image>,
    root: Option<Entity>,
    picture: Option<Entity>,
    fade: Option<Entity>,
    /// `RC_PLAY_MOVIE`: (movie, requested).
    debug: Option<(i32, bool)>,
    skip_at: Option<u32>,
    trace: bool,
    /// Movies played to the end or skipped (reports).
    pub finished: Vec<(u32, bool)>,
    /// The mode the running movie interrupted (restored by a [`MovieExit::FrontEnd`] exit).
    interrupted: Option<Mode>,
}

impl MovieState {
    /// A movie is queued or playing (the title waits for its attract movie with this).
    pub fn busy(&self) -> bool { blocking(self) }
}

#[derive(Component)]
struct MovieUi;

pub struct MovieRenderPlugin;

impl Plugin for MovieRenderPlugin {
    fn build(&self, app: &mut App) {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let debug = env("RC_PLAY_MOVIE").and_then(|v| v.trim().parse::<i32>().ok()).map(|n| (n, false));
        let skip_at = env("RC_MOVIE_SKIP").and_then(|v| v.trim().parse().ok());
        if let Some((n, _)) = debug { println!("movie: RC_PLAY_MOVIE={n}: in-level movie {n} (mpegs[{}]) after gameplay tick 1", MOVIE_BASE_NTSC + n); }
        app.insert_resource(MovieState { debug, skip_at, trace: env("RC_MOVIE_TRACE").as_deref() == Some("1"), ..default() })
            // Mode 1 runs MovieModeUpdate instead of the game tick.
            .configure_sets(FixedUpdate, GameTick.run_if(|s: Res<MovieState>| !blocking(&s)))
            .add_systems(Startup, setup)
            .add_systems(Update, target_main_camera)
            .add_systems(FixedUpdate, (movie_exit.before(GameTick), movie_frame.after(GameTick)));
    }
}

fn setup(mut commands: Commands, mut state: ResMut<MovieState>, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d { width: 512, height: 416, depth_or_array_layers: 1 };
    let mut img = Image::new_fill(size, TextureDimension::D2, &[0, 0, 0, 255], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
    img.sampler = ImageSampler::nearest();
    state.image = images.add(img);
    let full = || Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    // Filling the cutscene aspect's box (crate::display), not kept at the picture's 512:416; the root is black around it.
    let picture = commands.spawn((full(), ImageNode::new(state.image.clone()).with_mode(bevy::ui::widget::NodeImageMode::Stretch), Visibility::Hidden, crate::display::CutsceneBoxed, Name::new("movie picture"))).id();
    let fade = commands.spawn((full(), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)), Name::new("movie fade"))).id();
    // Above the HUD composite (same global z, higher local z).
    let root = commands.spawn((full(), BackgroundColor(Color::BLACK), GlobalZIndex(i32::MAX), ZIndex(1), Visibility::Hidden, MovieUi, Name::new("movie"), crate::level_switch::KeepAcrossLevels)).add_children(&[picture, fade]).id();
    (state.root, state.picture, state.fade) = (Some(root), Some(picture), Some(fade));
}

fn target_main_camera(mut commands: Commands, nodes: Query<Entity, (With<MovieUi>, Without<UiTargetCamera>)>, cams: Query<Entity, With<FlyCam>>) {
    let Some(cam) = cams.iter().next() else { return };
    for n in &nodes { commands.entity(n).insert(UiTargetCamera(cam)); }
}

fn movie_path(file: u32) -> PathBuf { crate::level_load::extracted_root().join(format!("global/mpegs/{file:03}.bin")) }

/// The worker: open, send the audio, then every picture as RGBA in display order.
fn decode_thread(path: PathBuf, language: u8, tx: SyncSender<Msg>) {
    let file = match std::fs::read(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(Msg::Failed(format!("{}: {e}", path.display())));
            return;
        }
    };
    let mut m = match rc_video::Movie::open(&file, language) {
        Ok(m) => m,
        Err(e) => {
            let _ = tx.send(Msg::Failed(format!("{}: {e}", path.display())));
            return;
        }
    };
    drop(file);
    let s = m.sequence;
    let info = format!(
        "{}×{} at {:.3} fps, audio channel {:?} of {:?} at {} Hz ({:.2} s), {} packs",
        s.width, s.height, m.fps(), m.audio_channel, m.channels, m.audio_rate, m.audio.as_ref().map_or(0.0, |a| a.len() as f64 / 48_000.0), m.packs
    );
    if tx.send(Msg::Opened { fps: (s.fps_num, s.fps_den), audio: m.audio.take(), info }).is_err() { return; }
    let mut n = 0u32;
    loop {
        match m.next_frame() {
            Ok(Some(f)) => {
                let mut rgba = Vec::new();
                rc_video::movie::frame_to_rgba(&f, &mut rgba);
                if tx.send(Msg::Picture(n, rgba)).is_err() { return; }
                n += 1;
            }
            Ok(None) => {
                let _ = tx.send(Msg::End(n));
                return;
            }
            Err(e) => {
                let _ = tx.send(Msg::Failed(format!("{} after {n} pictures: {e}", path.display())));
                return;
            }
        }
    }
}

impl Running {
    /// Handles one worker message (blocking); false when the worker is gone.
    fn receive(&mut self) -> bool {
        let rx = self.rx.get_mut().unwrap_or_else(|e| e.into_inner());
        match rx.recv() {
            Ok(Msg::Opened { fps, audio, info }) => {
                println!("movie: mpegs[{}] opened: {info}", self.req.file);
                self.player.set_rate(fps.0, fps.1);
                self.audio = audio;
                self.opened = true;
            }
            Ok(Msg::Picture(i, rgba)) => {
                self.received = i + 1;
                self.rgba = rgba;
            }
            Ok(Msg::End(n)) => self.total = Some(n),
            Ok(Msg::Failed(e)) => {
                eprintln!("movie: {e}; the movie ends here");
                self.opened = true;
                self.total = Some(self.received);
            }
            Err(_) => {
                self.opened = true;
                self.total = Some(self.received);
                return false;
            }
        }
        true
    }

    /// Waits until picture `p` has arrived (or the stream ended before it).
    fn fetch(&mut self, p: u32) {
        while !self.opened { if !self.receive() { break; } }
        while self.received <= p && self.total.is_none() { if !self.receive() { break; } }
    }
}

/// The fade quad's alpha so that the linear-light blend `C·(1 − α)` shows `C·(1 − coverage)` in display bytes.
fn fade_alpha(coverage: f32) -> f32 { if coverage >= 1.0 { 1.0 } else { 1.0 - (1.0 - coverage.clamp(0.0, 1.0)).powf(2.2) } }

#[allow(clippy::too_many_arguments)]
fn movie_frame(
    mut state: ResMut<MovieState>,
    play: Option<Res<Play>>,
    persistent: Option<Res<Persistent>>,
    mut menu: Option<ResMut<crate::menu_render::MenuMode>>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
    pad: Option<Res<PadFrame>>,
    det: Res<crate::determinism::Deterministic>,
    frame_no: Res<crate::determinism::FrameNumber>,
    ticks: Res<crate::determinism::GameTicks>,
    mut images: ResMut<Assets<Image>>,
    mut vis: Query<&mut Visibility>,
    mut bg: Query<&mut BackgroundColor>,
) {
    let st = &mut *state;
    // RC_PLAY_MOVIE: after gameplay tick 1.
    if let (Some((n, false)), Some(p)) = (st.debug, play.as_ref()) {
        if p.game.counter >= 2 {
            st.debug = Some((n, true));
            request_in_level(n, None);
        }
    }
    if st.running.is_none() {
        let Some(req) = queue().pop_front() else { return };
        let path = movie_path(req.file);
        let language = req.language.unwrap_or(crate::hud_render::language() as u8);
        let (tx, rx) = sync_channel(4);
        let spawn = std::thread::Builder::new().name(format!("movie {:03}", req.file)).spawn({
            let path = path.clone();
            move || decode_thread(path, language, tx)
        });
        if let Err(e) = spawn { eprintln!("movie: cannot start the decoder thread: {e}"); }
        let gs = persistent.as_ref().map(|s| &s.0.global);
        // 0x15eed8: the caller's replay mode (fun_001e9488 sets 2), else the global as the page menu left it.
        if let (Some(r), Some(m)) = (req.replay, menu.as_mut()) { m.replay = r; }
        let replay = req.replay.unwrap_or_else(|| menu.as_ref().map_or(0, |m| m.replay));
        let ctx = MovieContext { replay, game_beaten: gs.is_some_and(|g| g.game_beaten != 0), completes: gs.map_or(0, |g| g.completes), level: crate::level_load::level_index() as i32 };
        println!(
            "movie: app frame {}: DialogStreamUpdate({}) → StartPssMovie(mpegs[{}], language {language}) {}: mode 1, sounds and music stopped",
            frame_no.0, req.movie, req.file, path.display()
        );
        if let Some(a) = audio.as_mut() { a.system().movie_stop(); }
        st.interrupted = menu.as_ref().map(|m| m.state.mode);
        if let Some(m) = menu.as_mut() { m.state.set(Mode::Movie); }
        let player = MoviePlayer::new(ctx).with_entry(req.black_entry, req.fade_in);
        st.running = Some(Running {
            req,
            player,
            rx: Mutex::new(rx),
            opened: false,
            audio: None,
            total: None,
            received: 0,
            rgba: Vec::new(),
            shown: None,
            pad: PadState::default(),
            frames: 0,
            last_phase: None,
        });
    }
    let Some(rt) = st.running.as_mut() else { return };
    // The pad (readMpeg polls it each loop); RC_MOVIE_SKIP presses the skip combination on one frame.
    if let Some(p) = &pad { rt.pad.update(Some(&p.0.bytes()), false); }
    if st.skip_at == Some(rt.frames) {
        let shoulders = button::L1 | button::L2 | button::R1 | button::R2;
        rt.pad.held |= shoulders | button::START;
        rt.pad.pressed |= button::START;
        println!("movie: RC_MOVIE_SKIP: Start + L1 L2 R1 R2 on movie frame {}", rt.frames);
    }
    let latency = if det.0 { 0 } else { audio.as_ref().map_or(0, |a| a.queued()) as u32 };
    if let Some(p) = rt.player.wanted_picture(latency) { rt.fetch(p); }
    let out: MovieFrame = rt.player.tick(&rt.pad, rt.total, latency);
    rt.frames += 1;
    // The picture.
    if let Some(p) = out.picture {
        if rt.shown != Some(p) && rt.received > 0 {
            if let Some(mut img) = images.get_mut(&st.image) { img.data = Some(rt.rgba.clone()); }
            if st.trace && p.is_multiple_of(30) { println!("movie: app frame {} (movie frame {}): picture {} of {:?}", frame_no.0, rt.frames, rt.received - 1, rt.total); }
            rt.shown = Some(p);
        }
    }
    if st.trace && rt.last_phase.as_ref().map(std::mem::discriminant) != Some(std::mem::discriminant(&out.phase)) {
        println!("movie: app frame {} (movie frame {}): {:?}", frame_no.0, rt.frames, out.phase);
    }
    rt.last_phase = Some(out.phase);
    // The audio: the movie's next 800 samples while it plays, silence in the fades; one frame per vsync (none on the
    // start frame, whose gameplay tick already output it).
    if let Some(a) = audio.as_mut().filter(|a| rt.frames > 1 || a.frames_out() < ticks.0) {
        let a = &mut **a;
        let mut buf = std::mem::take(&mut a.buf);
        buf.clear();
        match (out.phase, rt.audio.as_ref()) {
            (Phase::Playing(k), Some(pcm)) => {
                let at = (k as u64 * SAMPLES_PER_VSYNC) as usize;
                let mut chunk = [[0i16; 2]; SAMPLES_PER_VSYNC as usize];
                if at < pcm.len() {
                    let n = (pcm.len() - at).min(chunk.len());
                    chunk[..n].copy_from_slice(&pcm[at..at + n]);
                }
                a.system().mix_movie(&chunk, &mut buf);
            }
            _ => buf.resize(SAMPLES_PER_VSYNC as usize, [0, 0]),
        }
        a.buf = buf;
        a.push_frame();
    }
    // The UI.
    let playing = out.picture.is_some() && rt.shown.is_some();
    let set = set_visible;
    set(&mut vis, st.root, true);
    set(&mut vis, st.picture, playing);
    if let Some(mut c) = st.fade.and_then(|e| bg.get_mut(e).ok()) {
        let want = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, fade_alpha(out.black)));
        if *c != want { *c = want; }
    }
}

fn set_visible(vis: &mut Query<&mut Visibility>, e: Option<Entity>, show: bool) {
    if let Some(mut v) = e.and_then(|e| vis.get_mut(e).ok()) {
        let want = if show { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want { *v = want; }
    }
}

/// `MovieExitToGameplay` 0x2ad2b8, on the frame after the exit fade's closing vsync (before the tick, which runs again
/// this frame): mode 0, the level track restarted, the talker's dialogue refreshed.
fn movie_exit(
    mut state: ResMut<MovieState>,
    mut play: Option<ResMut<Play>>,
    mut menu: Option<ResMut<crate::menu_render::MenuMode>>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
    frame_no: Res<crate::determinism::FrameNumber>,
    mut vis: Query<&mut Visibility>,
) {
    let st = &mut *state;
    if !st.running.as_ref().is_some_and(|r| r.player.done()) { return; }
    let rt = st.running.take().unwrap();
    println!(
        "movie: app frame {}: mpegs[{}] {} after {} movie frames ({} of {:?} pictures decoded); mode 0, music_start_track, gameplay resumes",
        frame_no.0, rt.req.file, if rt.player.skipped { "skipped" } else { "played through" }, rt.frames, rt.received, rt.total
    );
    st.finished.push((rt.req.file, rt.player.skipped));
    set_visible(&mut vis, st.root, false);
    set_visible(&mut vis, st.picture, false);
    let interrupted = st.interrupted.take();
    match rt.req.exit {
        MovieExit::Gameplay => {
            // MovieExitToGameplay: replay mode 2 (the menu's in-level replays) back to 0 (0x15eed8).
            if let Some(m) = menu.as_mut() {
                if m.replay == 2 { m.replay = 0; }
            }
            if let Some(a) = audio.as_mut() { a.system().movie_exit(); }
            if let Some(m) = menu.as_mut() { m.state.set(Mode::Gameplay); }
            if let Some(p) = play.as_mut() {
                if p.svc.game_mode == 1 { p.svc.game_mode = 0; }
            }
            if let (Some(npc), Some(p)) = (rt.req.talker, play.as_mut()) {
                p.svc.interact.talker = Some(npc);
                p.svc.interact.scene_ended = true;
            }
        }
        MovieExit::FrontEnd => {
            // fun_001e9488's tail: 0x15eed8 = 0, FadeToBlack(4), the idle counter 0x15f604 = 0 (the title's); no music.
            if let Some(m) = menu.as_mut() {
                m.replay = 0;
                m.state.set(interrupted.unwrap_or(Mode::Gameplay));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_alpha_matches_display_bytes() {
        assert_eq!(fade_alpha(0.0), 0.0);
        assert_eq!(fade_alpha(1.0), 1.0);
        // Display C·(1 − c) ≈ ((C_lin·(1 − α))^(1/2.2)).
        let (c, cov) = (0.6f32, 0.25f32);
        let shown = (c.powf(2.2) * (1.0 - fade_alpha(cov))).powf(1.0 / 2.2);
        assert!((shown - c * (1.0 - cov)).abs() < 1e-5);
    }

    #[test]
    fn requests_map_to_the_ntsc_files() {
        queue().clear();
        request_in_level(3, None);
        request_in_level(-1, None);
        assert_eq!(queue().pop_front(), Some(MovieRequest::in_level(3, None)));
        assert_eq!(MovieRequest::in_level(3, None).file, 5);
        assert!(queue().is_empty());
    }
}
