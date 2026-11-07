//! Audio output: the game's audio (`rc_game::audio`: sound slots, 989snd, music, software SPU2) run once per
//! 60 Hz game tick and played through Bevy's audio as one endless rodio source (docs/plan/audio.md §6.7).
//!
//! * **With the game tick** (crate::gameplay): the sound layer is part of the tick
//!   (`rc_game::audio::class_sounds`): the moby loop's class sounds get their slot during the moby loop, and
//!   the tick's sound step (after the camera, before the counter increment) plays Ratchet's animation-trigger
//!   sounds and runs the EE frame (`sound_update`) and its 800 samples ([`AudioOut::push_frame`]), all on the
//!   game's one `rand` stream (`Game::rng`). The listener is the game camera 0x167240 of that tick (also while
//!   the fly camera is shown: the game's draws must not depend on the debug view), the owners' positions come
//!   from the moby table. A rendered frame whose game ticks did not run (a menu pausing the tick, a frame the
//!   catch-up rule skipped) gets IOP-only frames here (989snd and the SPU keep playing; no EE update, no draw).
//! * **In a scene** (crate::scene_render, game mode 2: the gameplay tick is suspended): [`scene_sound`] runs
//!   the audio frame of each scene frame (`AudioSystem::scene_frame_with`): the scene's requests (music pause,
//!   speech, speech stop, music resume) apply at once, the EE frame (`sound_update`, `music_Update`) runs on the
//!   frames whose world update runs (`CutsceneModeUpdate`), on the game's stream with the scene camera as the
//!   listener (0x167240), and the blocking fades / vsync waits get IOP-only frames.
//! * **Without it** (`RC_PLAY=0`): each 60 Hz tick ([`GameTicks`]) runs the EE part of the frame on the audio
//!   system's private stream (`srand(1234)`) with the listener = the main camera and the hero position = the
//!   camera, then renders exactly 800 samples (48 kHz stereo).
//! * The mix is a pure function of the tick sequence and those inputs, so frame-exact runs give identical
//!   audio. `RC_AUDIO=0` removes the sound layer's draws from the game stream (the game without sound).
//! * The samples go to a ring buffer read by [`MixerDecoder`], a `rodio::Source` behind the `Decodable` asset
//!   [`MixerStream`] (bevy_audio 0.19, `add_audio_source`). The reader waits for 50 ms of audio before it
//!   starts (and again after an underrun, outputting silence meanwhile); the ring is capped at 0.25 s.
//! * The underwater flag 0x167494 comes from crate::fog_state (`UnderwaterTest` 0x20e9f0 in its camera update, which
//!   runs after the frame's ticks: the sound step reads the flag of the previous frame's camera, one tick late
//!   [L]); [`sync_underwater`] copies it into the audio system before the tick. The reverb (zones and effect:
//!   `rc_game::audio::reverb`) runs inside the audio system.
//! * Not connected yet: the flag-0x40 rotated owner offsets.
//!
//! Environment:
//! - `RC_AUDIO=0`: no audio (nothing is loaded).
//! - `RC_AUDIO_WAV=path`: also write the first 10 s of the mix as a 16-bit 48 kHz stereo WAV (on reaching
//!   10 s, or whatever was rendered when the app exits earlier); `RC_AUDIO_WAV_SECONDS=n` changes the length.
//! - `RC_AUDIO_TRACE=1`: one line per 60 output frames: music stream voices playing / held, speech, the main
//!   music player's state and track, the cutscene volumes, the pending resume; and one line per class sound played
//!   (tick, class, index, flags, the slot it got or −1), footstep and level sound (`PlayLevelSoundAtMoby`), and one
//!   line per reverb command (`SetReverbEx` / `snd_AutoReverb`: type, depth).
//! - `RC_REVERB=0`: no reverb output (a developer switch for A/B captures; the zones still run).
//! - `RC_AUDIO_MUSIC` / `RC_AUDIO_SFX`: the options menu volumes 0..=1024 (0x15edec / 0x15edf0; defaults 716 and
//!   1024 from the boot data), e.g. `RC_AUDIO_MUSIC=0` to hear or record the effects alone.

use crate::determinism::GameTicks;
use bevy::audio::{AddAudioSource, AudioPlayer, ChannelCount, Decodable, SampleRate, Source};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use rc_formats::level::LevelCore;
use rc_game::audio::{self, AudioSystem, FrameInput, LevelAudio};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// `RC_AUDIO` is not "0".
pub fn enabled() -> bool { !std::env::var("RC_AUDIO").is_ok_and(|v| v.trim() == "0") }

/// Loads the level's audio data (bank, defs, sound instances, music VAGs; the collision mesh for the occlusion
/// rays). None when disabled or when the bank is missing (with a warning).
pub fn load(root: &Path, index: u32, core_index: &[u8], core: &LevelCore, core_data: &[u8], gameplay: &[u8], collision: Option<rc_formats::collision::Collision>) -> Option<LevelAudio> {
    if !enabled() { return None; }
    let t0 = Instant::now();
    let read = |name: &str| crate::disc_source::level_file(root, index, name);
    let run = move || -> anyhow::Result<LevelAudio> {
        let bank = read("sound_bank.bin")?;
        let header = read("level_header.bin")?;
        let table = rc_formats::sound_bank::music_table(&header)?;
        let music: [Option<Vec<u8>>; 15] = std::array::from_fn(|k| (table[k] != 0).then(|| read(&format!("music/{k:03}.bin")).ok()).flatten());
        Ok(LevelAudio::from_parts(&bank, core_index, core, core_data, gameplay, &header, &music, collision)?)
    };
    match run() {
        Ok(a) => {
            let names: Vec<&str> = a.music.iter().flatten().map(|m| m.name.as_str()).collect();
            println!(
                "audio: {} sounds, {} samples, {} level defs, {} sound instances, music {:?}; load {:.1} ms (RC_AUDIO=0 disables, RC_AUDIO_WAV=path records 10 s)",
                a.bank.sounds.len(), a.bank.vags.len(), a.sounds.level_defs.len(), a.instances.len(), names, t0.elapsed().as_secs_f64() * 1e3
            );
            Some(a)
        }
        Err(e) => {
            eprintln!("audio: cannot load the level's sound data ({e:#}); no audio");
            None
        }
    }
}

/// Frames the reader waits for before (re)starting: 50 ms.
const PREFILL: usize = 2400;
/// Ring cap: 0.25 s (older frames are dropped when the game runs ahead of the device).
const RING_CAP: usize = 12_000;
/// `RC_AUDIO_WAV` length: 10 s unless `RC_AUDIO_WAV_SECONDS`.
fn wav_frames() -> usize {
    let secs = std::env::var("RC_AUDIO_WAV_SECONDS").ok().and_then(|v| v.trim().parse::<usize>().ok()).filter(|&s| s > 0).unwrap_or(10);
    secs * audio::OUTPUT_RATE as usize
}

#[derive(Default)]
struct RingState {
    frames: VecDeque<[i16; 2]>,
    running: bool,
    underruns: u64,
    /// Frames dropped at the cap (the game ran ahead of the device).
    dropped: u64,
}

/// Rendered frames waiting for the audio device.
#[derive(Default)]
pub struct Ring(Mutex<RingState>);

impl Ring {
    fn push(&self, frames: &[[i16; 2]]) {
        let mut r = self.0.lock().unwrap_or_else(|e| e.into_inner());
        r.frames.extend(frames.iter().copied());
        let over = r.frames.len().saturating_sub(RING_CAP);
        r.frames.drain(..over);
        r.dropped += over as u64;
    }

    fn pop(&self) -> [i16; 2] {
        let mut r = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if !r.running {
            if r.frames.len() < PREFILL { return [0, 0]; }
            r.running = true;
        }
        match r.frames.pop_front() {
            Some(f) => f,
            None => {
                r.running = false;
                r.underruns += 1;
                [0, 0]
            }
        }
    }
}

/// The asset bevy_audio plays: a handle on the ring.
#[derive(Asset, TypePath)]
pub struct MixerStream {
    ring: Arc<Ring>,
}

/// The rodio source: interleaved stereo f32 at 48 kHz, endless.
pub struct MixerDecoder {
    ring: Arc<Ring>,
    right: Option<f32>,
}

impl Iterator for MixerDecoder {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(r) = self.right.take() { return Some(r); }
        let [l, r] = self.ring.pop();
        self.right = Some(r as f32 / 32768.0);
        Some(l as f32 / 32768.0)
    }
}

impl Source for MixerDecoder {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> ChannelCount { ChannelCount::new(2).unwrap() }
    fn sample_rate(&self) -> SampleRate { SampleRate::new(audio::OUTPUT_RATE).unwrap() }
    fn total_duration(&self) -> Option<std::time::Duration> { None }
}

impl Decodable for MixerStream {
    type Decoder = MixerDecoder;
    fn decoder(&self) -> MixerDecoder { MixerDecoder { ring: Arc::clone(&self.ring), right: None } }
}

/// `RC_AUDIO_WAV`: the first 10 s (`RC_AUDIO_WAV_SECONDS`).
struct WavCapture {
    path: PathBuf,
    frames: Vec<[i16; 2]>,
    limit: usize,
    written: bool,
}

impl WavCapture {
    fn write(&mut self) {
        if self.written { return; }
        self.written = true;
        match std::fs::write(&self.path, audio::wav_bytes(&self.frames)) {
            Ok(()) => println!("audio: wrote {:.2} s to {}", self.frames.len() as f64 / audio::OUTPUT_RATE as f64, self.path.display()),
            Err(e) => eprintln!("audio: cannot write {}: {e}", self.path.display()),
        }
    }
}

impl Drop for WavCapture {
    fn drop(&mut self) { self.write(); }
}

/// The audio system and its output (ring, WAV capture). crate::gameplay runs the sound step of each game tick
/// on it ([`AudioOut::system`], then [`AudioOut::push_frame`]).
#[derive(Resource)]
pub struct AudioOut {
    system: AudioSystem,
    ring: Arc<Ring>,
    wav: Option<WavCapture>,
    /// 800-sample frames output so far.
    done: u64,
    /// The frame being rendered (the sound step appends to it).
    pub buf: Vec<[i16; 2]>,
    last_report: u64,
    /// `RC_AUDIO_TRACE`.
    trace: bool,
    /// `RC_AUDIO_SFX` / `RC_AUDIO_MUSIC` given: they win over the saved game's options ([`sync_options`]).
    fixed_sfx: bool,
    fixed_music: bool,
}

impl AudioOut {
    pub fn system(&mut self) -> &mut AudioSystem { &mut self.system }

    /// A runtime level change (crate::level_switch): the next level's sound data (bank, defs, sound instances, music) in a
    /// new audio system, with the options, the reverb switch and the logs of the old one; the output (ring, WAV capture,
    /// frame count) goes on.
    pub fn swap_level(&mut self, data: LevelAudio) {
        let old = &self.system;
        let mut system = AudioSystem::new(data);
        (system.music_option, system.sfx_option) = (old.music_option, old.sfx_option);
        system.spu.reverb.enabled = old.spu.reverb.enabled;
        if old.play_log.is_some() { system.play_log = Some(Vec::new()); }
        if old.reverb_log.is_some() { system.reverb_log = Some(Vec::new()); }
        self.system = system;
    }

    pub fn stats(&self) -> audio::AudioStats { self.system.stats }

    /// The audio system and the frame buffer, borrowed together (the sound step renders into `buf`).
    pub fn parts(&mut self) -> (&mut AudioSystem, &mut Vec<[i16; 2]>) { (&mut self.system, &mut self.buf) }

    /// Frames rendered but not yet taken by the audio device (the output latency the movie clock subtracts).
    pub fn queued(&self) -> usize { self.ring.0.lock().map(|r| r.frames.len()).unwrap_or(0) }

    /// 800-sample frames output so far (one per 60 Hz tick).
    pub fn frames_out(&self) -> u64 { self.done }

    /// Output the rendered frame [`AudioOut::buf`] (ring and WAV capture) and clear it.
    pub fn push_frame(&mut self) {
        self.done += 1;
        self.ring.push(&self.buf);
        if let Some(w) = &mut self.wav {
            if !w.written {
                let take = (w.limit - w.frames.len()).min(self.buf.len());
                w.frames.extend_from_slice(&self.buf[..take]);
                if w.frames.len() == w.limit { w.write(); }
            }
        }
        self.buf.clear();
        if self.trace {
            for (tick, class, index, flags, slot) in self.system.play_log.as_mut().map(std::mem::take).unwrap_or_default() {
                if class == audio::class_sounds::FOOTSTEP_LOG_CLASS {
                    println!("audio trace: tick {tick}: footstep, level def {index}, flags {flags:#x} -> slot {slot}");
                    continue;
                }
                if class == audio::class_sounds::LEVEL_SOUND_LOG_CLASS {
                    println!("audio trace: tick {tick}: level sound {index}, flags {flags:#x} -> slot {slot}");
                    continue;
                }
                let who = match class { 0 => "Ratchet", 0x47 => "wrench", 0xd0 => "Swingshot", _ => "moby" };
                println!("audio trace: tick {tick}: class sound {index} of class {class} ({who}), flags {flags:#x} -> slot {slot}");
            }
        }
        if self.trace {
            for (frame, c) in self.system.reverb_log.as_mut().map(std::mem::take).unwrap_or_default() {
                let name = |k: u8| audio::reverb::mode::NAMES.get(k as usize).copied().unwrap_or("?");
                match c {
                    audio::voices::SndCommand::SetReverb { kind, depth, delay, feedback } => {
                        println!("audio trace: frame {frame}: SetReverbEx type {kind} ({}), depth {depth}, delay {delay}, feedback {feedback}", name(kind))
                    }
                    audio::voices::SndCommand::AutoReverb { depth, delta, .. } => println!("audio trace: frame {frame}: AutoReverb depth {depth} over {delta}"),
                    _ => {}
                }
            }
        }
        if self.trace && self.done.is_multiple_of(60) {
            let s = &self.system;
            let (music, held) = s.music_voices();
            let m = s.music.main;
            println!(
                "audio trace: frame {} ({:.2} s): music voices {music} playing, {held} held; speech {}; main player state {:#x} track {} handle {}; cutscene {}; resume in {:?}",
                self.done, self.done as f64 / 60.0, s.speech_playing(), m.state, m.track, m.handle, s.scene.cutscene, s.scene.resume_in
            );
        }
    }
}

/// Adds the audio when the level's data loaded.
pub struct AudioOutPlugin(Mutex<Option<LevelAudio>>);

impl AudioOutPlugin {
    pub fn new(data: Option<LevelAudio>) -> Self { AudioOutPlugin(Mutex::new(data)) }
}

impl Plugin for AudioOutPlugin {
    fn build(&self, app: &mut App) {
        let Some(data) = self.0.lock().unwrap_or_else(|e| e.into_inner()).take() else { return };
        let ring = Arc::new(Ring::default());
        let mut system = AudioSystem::new(data);
        let option = |name: &str| std::env::var(name).ok().and_then(|v| v.trim().parse::<i32>().ok()).map(|v| v.clamp(0, 0x400));
        let (fixed_music, fixed_sfx) = (option("RC_AUDIO_MUSIC"), option("RC_AUDIO_SFX"));
        if let Some(v) = fixed_music { system.music_option = v; }
        if let Some(v) = fixed_sfx { system.sfx_option = v; }
        let limit = wav_frames();
        let wav = std::env::var_os("RC_AUDIO_WAV").filter(|v| !v.is_empty()).map(|p| WavCapture { path: p.into(), frames: Vec::with_capacity(limit), limit, written: false });
        let trace = std::env::var("RC_AUDIO_TRACE").is_ok_and(|v| v.trim() == "1");
        if trace {
            system.play_log = Some(Vec::new());
            system.reverb_log = Some(Vec::new());
        }
        if std::env::var("RC_REVERB").is_ok_and(|v| v.trim() == "0") {
            system.spu.reverb.enabled = false;
            println!("audio: RC_REVERB=0: no reverb output");
        }
        app.add_audio_source::<MixerStream>()
            .insert_resource(AudioOut { system, ring, wav, done: 0, buf: Vec::new(), last_report: 0, trace, fixed_sfx: fixed_sfx.is_some(), fixed_music: fixed_music.is_some() })
            .add_systems(Startup, start_output)
            // After the scene frame (which runs before the tick) and the tick, which a running scene suspends.
            .add_systems(FixedUpdate, scene_sound.after(crate::gameplay::GameTick))
            .add_systems(FixedUpdate, sync_underwater.before(crate::gameplay::GameTick))
            .add_systems(FixedUpdate, sync_options.before(crate::gameplay::GameTick))
            .add_systems(FixedUpdate, sync_game_mode.before(crate::gameplay::GameTick))
            .add_systems(PostUpdate, run_audio);
    }
}

/// The underwater flag 0x167494 into the audio system (the sound step's listener; module docs).
fn sync_underwater(mut out: ResMut<AudioOut>, fog: Option<Res<crate::fog_state::FogState>>) {
    let flag = fog.is_some_and(|f| f.underwater_flag());
    if out.system.underwater != flag { out.system.underwater = flag; }
}

/// The game mode 0x15f5c4 into the audio system: `sound_update` only polls the running sounds outside modes 0, 2 and 6
/// and tests the occlusion in 0 and 2 only. The vendor and page-menu frames copy it again before their `sound_update`.
fn sync_game_mode(mut out: ResMut<AudioOut>, play: Option<Res<crate::gameplay::Play>>) {
    let mode = play.map_or(0, |p| p.svc.game_mode);
    if out.system.game_mode != mode { out.system.game_mode = mode; }
}

/// The saved game's sound options into the mixer every frame, as the game's `sound_update` resends the master volumes
/// from them: effects 0x15edf0 and music 0x15edec (0..0x400; `voices::master_volumes`), and mono when the stereo option
/// 0x15ede8 is 0 (`snd_SetPlaybackMode`). `RC_AUDIO_SFX` / `RC_AUDIO_MUSIC` keep their values.
fn sync_options(mut out: ResMut<AudioOut>, gs: Option<Res<crate::gameplay::Persistent>>) {
    let Some(gs) = gs else { return };
    let g = &gs.0.global;
    let (fixed_sfx, fixed_music) = (out.fixed_sfx, out.fixed_music);
    let s = &mut out.system;
    if !fixed_sfx && s.sfx_option != g.effects_volume.clamp(0, 0x400) { s.sfx_option = g.effects_volume.clamp(0, 0x400); }
    if !fixed_music && s.music_option != g.music_volume.clamp(0, 0x400) { s.music_option = g.music_volume.clamp(0, 0x400); }
    let mono = g.stereo & 0xff == 0;
    if s.snd.vm.mono != mono { s.snd.vm.mono = mono; }
}

fn start_output(mut commands: Commands, out: Res<AudioOut>, mut streams: ResMut<Assets<MixerStream>>) {
    let handle = streams.add(MixerStream { ring: Arc::clone(&out.ring) });
    commands.spawn(AudioPlayer(handle));
}

/// The audio frame of a scene frame while the scene suspends the gameplay tick (module docs): the EE frame
/// on the game's stream and counter when the world runs, the listener = the scene camera (the play camera
/// before the first scene tick), the hero position and the owners from the game.
fn scene_sound(
    mut out: ResMut<AudioOut>,
    scene: Option<Res<crate::scene_render::ActiveScene>>,
    play: Option<ResMut<crate::gameplay::Play>>,
    mut counter_seen: Local<Option<u64>>,
) {
    let Some(scene) = scene else { return };
    let ran = play.as_ref().map(|p| p.game.counter);
    let tick_ran = ran.is_some() && *counter_seen != ran;
    *counter_seen = ran;
    if !scene.running { return; }
    // A world-running scene frame whose gameplay tick ran (its mode-2 form) already made the EE audio frame in the
    // tick's sound step (crate::scene_render); only the frames without one are made here.
    if tick_ran && scene.world_runs { return; }
    let (Some(mut play), Some(last)) = (play, scene.last.as_ref()) else { return };
    let game = &mut play.game;
    let mut listener = rc_game::audio::class_sounds::listener_with_water(&game.camera.out, out.system.underwater, &game.hero);
    if let Some(c) = &last.camera { (listener.pos, listener.rows) = (c.eye, c.rows); }
    let h = game.hero.pos;
    let input = FrameInput { listener, hero_pos: [h[0].to_f32(), h[1].to_f32(), h[2].to_f32()] };
    let counter = game.counter as u32;
    let table = &game.mobys;
    let (sys, buf) = out.parts();
    buf.clear();
    sys.scene_frame_with(last.world_runs, &input, counter, &mut game.rng, &|id| rc_game::audio::class_sounds::owner_position(table, id), buf);
    out.push_frame();
}

fn run_audio(
    mut out: ResMut<AudioOut>,
    ticks: Res<GameTicks>,
    cams: Query<&Transform, With<crate::fly_cam::FlyCam>>,
    play: Option<Res<crate::gameplay::Play>>,
) {
    let Some(cam) = cams.iter().next() else { return };
    let eye = crate::game_camera::game_eye(cam);
    let rows = crate::game_camera::game_rows(cam);
    let listener = audio::voices::Listener { pos: eye.to_array(), rows: rows.map(|r| r.to_array()), underwater: out.system.underwater, water_height: 0.0 };
    let input = FrameInput { listener, hero_pos: listener.pos };
    let out = &mut *out;
    // With the game tick the EE frames ran in its sound step; fill the ticks it did not run with IOP-only frames.
    let driven = play.is_some();
    while out.done < ticks.0 {
        out.buf.clear();
        if driven {
            out.system.render(audio::SAMPLES_PER_FRAME, &mut out.buf);
        } else {
            out.system.tick(&input, &mut out.buf);
        }
        out.push_frame();
    }
    if out.done >= out.last_report + 600 {
        out.last_report = out.done;
        let s = &out.system;
        let (underruns, dropped, fill) = out.ring.0.lock().map(|r| (r.underruns, r.dropped, r.frames.len())).unwrap_or_default();
        println!(
            "audio: tick {}, {} slots busy, {} SPU voices active, {} sounds playing, {} plays so far ({} of {} class sounds got a slot), {} underruns, {} frames dropped, ring {} frames{}",
            out.done,
            s.slots.slots.iter().filter(|x| x.state != audio::voices::state::FREE).count(),
            s.spu.voices.iter().filter(|v| v.active()).count(),
            s.snd.playing(),
            s.stats.plays,
            s.stats.class_slots,
            s.stats.class_sounds,
            underruns,
            dropped,
            fill,
            if s.snd.unimplemented.is_empty() { String::new() } else { format!(", unported grains {:?}", s.snd.unimplemented) }
        );
    }
}
