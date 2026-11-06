//! The in-engine scene player (game mode 2): docs/plan/cutscenes_transitions.md §3. Ported from level01
//! `DialogStreamStart` 0x2ac330 (start), `CutsceneModeUpdate` 0x2aca80 (tick), `FUN_002ac8d8` (camera
//! record), `FUN_00259288` (chunk parse → actors), `FUN_002ac608` (end) and the subtitle draw
//! `fun_001f4be0` (0x21b620); data from [`rc_formats::scene`].
//!
//! One [`ScenePlayer::tick`] = one 60 Hz frame (vsync). The game's blocking steps are spelled out as frames:
//!
//! | frames | what | world |
//! |---|---|---|
//! | 6 | `FadeToBlack(ticks(6))`: black quads of alpha `0x80 − i·0x80/6` (i = 5..0) drawn over the frozen last image | frozen |
//! | 1 | the fade's closing vsync (black) | frozen |
//! | 3 | the speech stream starts ([`AudioRequest::Speech`]), `0x16cd28 = −3`, 3 vsync waits (black) | frozen |
//! | 1 | the rest of the triggering gameplay frame; its render is skipped (0x15f5d8 = 1): black | runs |
//! | end − 1 | scene ticks 1 .. end − 1 (`CutsceneModeUpdate`) | runs |
//! | 1 | tick = end → `FUN_002ac608`: the first vsync of `FadeToBlack(12)` shows the last image | frozen |
//! | 12 + 1 | `FadeToBlack(12)` and its closing vsync; then mode 0 ([`SceneEnd`]) | frozen |
//!
//! The frame accounting of the blocking loops (one vsync per `func_0x00122298(0)`) is read from the code,
//! not traced [M]. Novalis scene 5: 11 frames before tick 1, 1507 scene frames, 14 after: 1532 frames.
//!
//! **Per tick** (0x2aca80, in order): fade 0x15f3fc −= 0.34 (clamped at 0 after the store); scene tick and
//! chunk tick +1; end when tick ≥ end tick; skip (only when tick ≥ `ticks(18)` and the fade is 0): Start
//! (pressed & 0x800) alone when the game is beaten (0x15eea0), completed (0x15ee20), replayed from the menu
//! (0x15eed8) or on level 0, else Start plus L1 L2 R1 R2 held (0x13cae0 & 0xf); chunk rollover when the
//! chunk tick reaches 96 (80 PAL): parse the next chunk (chunk tick = 0, every actor's sequence slot
//! re-pointed); camera from record `chunk tick` (no interpolation); actors: key frames `f = chunk tick >> 1`
//! and `f + 1`, blend `t = (chunk tick & 1)·0.5`, or 1.0 on an odd tick whose camera record has the cut
//! flag; position = `(1 − t)·track[f] + t·track[f + 1]` (`VecScale`, `VecScale`, `VecAdd`).
//!
//! **Camera** (`FUN_002ac8d8`): `M = I`, then `sceVu0RotMatrixX(M, M, a)` (0x125360, angle +0x10),
//! `sceVu0RotMatrixY` (0x125408, +0x14), `sceVu0RotMatrixZ` (0x1252b8, +0x18; unnamed in Lombyte, same SDK
//! code). Each routine left-multiplies every stored row q_i of M by the rotation (`q_i ← R·q_i`, one VU
//! `vmula/vmadda/vmadd` chain), so starting from I the stored rows are the **columns of
//! Rz(c)·Ry(b)·Rx(a)** (X applied first). The camera rows are forward 0x167450 = −q2, left 0x167460 = −q0,
//! up 0x167470 = q1; the eye is copied to 0x167240 and tan(hfov/2) to 0x16cf70. Sine and cosine come from
//! `sceVu0ECosSin` (0x125240): cos = the 9th-order sine polynomial (coefficients at boot 0x132e00) of
//! π/2 − |θ|, sin = ±√(1 − cos²) with the sign of θ. All of it runs on the PS2 float model
//! ([`crate::ps2v::Pf`]), so the rows are the game's bit patterns.
//!
//! **End** (0x2ac608): speech stop, `FadeToBlack(12)`, mode 0, tan(hfov/2) = 0.63 (0x3f2147ae), fade 0,
//! actors deleted (their class slots freed), hero `SetState(0, 1)` and the ground snap (`GroundHeight(0.5,
//! hero)`: accepted when > 2 and within 4.5 of the hero z), hero items shown again (`FUN_002487a8`), the
//! optional teleport (0x16cd26, written only by `TalkingNpcUpdate` via 0x2783a8: in front of the NPC, facing
//! it), the NPC dialog continues, and `FUN_0027a460(ticks(30))`: music resumes 30 ticks later.
//!
//! Not modelled here (engine / other ports): the mirror cheat's left-row cross product and the 0x15edb0
//! FOV cheat scale, mode 6 space scenes (same chunks, ship-local transform). The world update of the scene frame
//! (the moby loop, the hero, the particles), the actor mobys, classes 74 / 203 (mode bit 0x80, [`SCENE_HIDDEN_CLASSES`])
//! and the state the classes read ([`SceneState`], e.g. the FX driver 1546) are the engine's (`rc-engine`
//! `scene_render`).

use crate::pad::{button, PadState};
use crate::ps2v::Pf;
use rc_formats::scene::{CamRecord, Region, Scene};
use std::sync::Arc;

/// `FadeToBlack(ticks(6))` at the start.
pub const START_FADE_FRAMES: u32 = 6;
/// vsyncs between the speech stream start and the first scene tick's update (0x16cd28 = −3 wait loop).
pub const AUDIO_LEAD: u32 = 3;
/// `FadeToBlack(12)` at the end (not scaled by `ticks`).
pub const END_FADE_FRAMES: u32 = 12;
/// Fade decrement per tick (0x2aca80: 1.0 → 0.66 → 0.32 → 0).
pub const FADE_STEP: Pf = Pf::b(0x3eae_147b);
/// Skip is possible from this scene tick on (`ticks(0x12)`).
pub const SKIP_MIN_TICKS: i32 = 18;
/// `FUN_0027a460(ticks(30))`: music resume delay after the end.
pub const MUSIC_RESUME_TICKS: i32 = 30;
/// Gameplay tan(hfov/2) the end restores (0x16cf70 = 0x3f2147ae).
pub const GAMEPLAY_TAN_HALF_FOV: Pf = Pf::b(0x3f21_47ae);
/// Hero state during a scene (`SetState(100, 2)`).
pub const HERO_SCENE_STATE: i32 = 100;
/// The draw mask of the mode-2 render (0x21aa90): HUD bit 0x80 cleared.
pub const SCENE_DRAW_MASK: u8 = 0x7f;

/// `sceVu0ECosSin` polynomial coefficients (boot 0x132e00, x y z w = x⁹ x⁷ x⁵ x³ terms).
const SIN_COEF: [Pf; 4] = [Pf::b(0x362e_9c14), Pf::b(0xb94f_b21f), Pf::b(0x3c08_873e), Pf::b(0xbe2a_aaa4)];
/// π/2 as the rotation routines load it (`lui 0x3fc9; ori 0x0fdb`).
const HALF_PI: Pf = Pf::b(0x3fc9_0fdb);

/// What the skip rule and the subtitle draw read from the game state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneContext {
    /// 0x15eea0.
    pub game_beaten: bool,
    /// 0x15ee20 (times completed).
    pub completes: i32,
    /// 0x15eed8: replaying from the pause menu.
    pub replay: bool,
    /// 0x15ed84: current level.
    pub level: i32,
    /// 0x15ed88: 0 En, 2 Fr, 3 De, 4 Es, 5 It (speech VAG index).
    pub language: usize,
    /// Option 0x15ee40.
    pub subtitles: bool,
}

impl SceneContext {
    /// Subtitle text index: `language − 1` for 2..5, else English.
    pub fn subtitle_language(&self) -> usize { if (2..=5).contains(&self.language) { self.language - 1 } else { 0 } }
    /// Start alone skips (else Start + L1 L2 R1 R2).
    fn start_alone_skips(&self) -> bool { self.game_beaten || self.completes != 0 || self.replay || self.level <= 0 }
}

/// The camera a scene frame uses (the play camera consumes it while a scene runs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneCamera {
    /// 0x167240.
    pub eye: [f32; 3],
    /// 0x167450 / 0x167460 / 0x167470: forward, left, up (game space).
    pub rows: [[f32; 3]; 3],
    /// 0x16cf70.
    pub tan_half_fov: f32,
    /// The record's cut flag.
    pub cut: bool,
}

/// One actor's pose this frame: key frames of its streamed sequence (the sequence of `chunk`'s actor
/// record `actor`, registered as an extra class sequence slot) and the lerped position (moby+0x10).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorPose {
    pub actor: usize,
    pub class: i32,
    pub chunk: usize,
    /// moby+0x50 / +0x51 / +0x54.
    pub frame_a: u8,
    pub frame_b: u8,
    pub t: f32,
    pub position: [f32; 3],
}

/// The subtitle line drawn this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubtitleLine {
    pub start: i16,
    pub end: i16,
    /// Latin-1 bytes in the game's font encoding.
    pub text: Vec<u8>,
}

/// Audio requests (see `crate::audio::scene`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioRequest {
    /// `music_Pause(0)` (at the start, before the fade).
    PauseMusic,
    /// Stream `scenes[scene].speech[language]`; the first scene tick follows `lead` frames (+1) later.
    Speech { scene: usize, language: usize, start_tick: i32, lead: u32 },
    /// Speech stop (0x15172a → 5).
    StopSpeech,
    /// Music resumes after `after` ticks.
    ResumeMusic { after: i32 },
}

/// What the end hands back to gameplay (applied on the frame after the last fade frame).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneEnd {
    pub skipped: bool,
    /// `SetState(0, 1)`.
    pub hero_state: i32,
    /// Ground snap: probe 0.5 up; accept when the ground is > 2 and within 4.5 of the hero z.
    pub ground_snap_up: f32,
    pub ground_snap_min: f32,
    pub ground_snap_max_dz: f32,
    /// 0x16cf70.
    pub tan_half_fov: f32,
    pub music_resume_after: i32,
}

/// The frame kinds of the timeline (module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    /// `FadeToBlack(6)` step (0-based).
    FadeIn(u32),
    /// Black frames before the first tick: the fade's last vsync, the audio lead, the skipped render.
    Black(u32),
    /// Scene tick update.
    Playing,
    /// The end-tick frame (last image held).
    EndHold,
    /// `FadeToBlack(12)` step (0-based).
    FadeOut(u32),
    /// The fade's closing vsync; the scene is over after this frame.
    Finished,
}

/// One frame's output.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneTick {
    pub frame: Frame,
    /// 0x16cd14 after this frame's update.
    pub scene_tick: i32,
    /// The world update runs this frame (mobys, particles, lights: `CutsceneModeUpdate`); false during the
    /// blocking fades and waits.
    pub world_runs: bool,
    /// Camera override; None = keep the current view (before the first tick).
    pub camera: Option<SceneCamera>,
    /// Actor poses (empty before the first tick).
    pub actors: Vec<ActorPose>,
    /// Black full-screen coverage to draw over this frame's image, 0..1 (the 0x15f3fc quad, or the
    /// accumulated `FadeToBlack` quads over the held image).
    pub black: f32,
    /// 0x15f3fc.
    pub fade: f32,
    pub subtitle: Option<SubtitleLine>,
    pub audio: Vec<AudioRequest>,
    /// Draw mask 0x7f: no HUD.
    pub hud_hidden: bool,
    /// Set on the [`Frame::Finished`] frame.
    pub end: Option<SceneEnd>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    FadeIn(u32),
    Black(u32),
    Playing,
    FadeOut(u32),
    Finished,
    Done,
}

/// Mode 2.
#[derive(Clone, Debug)]
pub struct ScenePlayer {
    scene: Arc<Scene>,
    ctx: SceneContext,
    phase: Phase,
    /// 0x16cd14 / 0x16cd1c / 0x16cd18.
    tick: i32,
    chunk: usize,
    chunk_tick: u32,
    /// 0x15f3fc.
    fade: Pf,
    skipped: bool,
    /// Black coverage accumulated by the running `FadeToBlack`.
    black: f32,
    /// The image under the start fade is already black ([`ScenePlayer::start_from_black`]).
    from_black: bool,
    last_camera: Option<SceneCamera>,
    last_actors: Vec<ActorPose>,
    last_subtitle: Option<SubtitleLine>,
}

/// `sceVu0ECosSin` as the rotation routines call it: (sin θ, cos θ).
pub fn ecossin(theta: Pf) -> (Pf, Pf) {
    let neg = theta < Pf::ZERO;
    let x = if neg { HALF_PI + theta } else { HALF_PI - theta };
    let x2 = x * x;
    // vf8 = coef·x, then ·x² per power; the terms are added highest power last.
    let c = SIN_COEF.map(|k| k * x);
    let c = c.map(|v| v * x2);
    let w3 = c[3];
    let c = [c[0] * x2, c[1] * x2, c[2] * x2];
    let mut s = (Pf::ZERO + x) + w3;
    let z5 = c[2];
    let c = [c[0] * x2, c[1] * x2];
    s += z5;
    let y7 = c[1];
    let x9 = c[0] * x2;
    s += y7;
    s += x9;
    let cos = Pf::ZERO + s;
    let q = (Pf::ONE - cos * cos).sqrt();
    let sin_mag = Pf::ZERO + q;
    let sin = if neg { Pf::ZERO - sin_mag } else { Pf::ZERO + sin_mag };
    (sin, cos)
}

type Q = [Pf; 4];

/// One `R · q` (vmulax / vmadday / vmaddaz / vmaddw).
fn apply(r: &[Q; 4], q: Q) -> Q {
    std::array::from_fn(|k| {
        let acc = r[0][k] * q[0];
        let acc = acc + r[1][k] * q[1];
        let acc = acc + r[2][k] * q[2];
        acc + r[3][k] * q[3]
    })
}

/// The rotation "columns" (vf6, vf7, vf8, vf9) of `sceVu0RotMatrixX/Y/Z` for (sin, cos).
fn rot(axis: usize, s: Pf, c: Pf) -> [Q; 4] {
    let (z, o) = (Pf::ZERO, Pf::ONE);
    let (ps, ms, pc) = (z + s, z - s, z + c);
    match axis {
        0 => [[o, z, z, z], [z, pc, ps, z], [z, ms, pc, z], [z, z, z, o]],
        1 => [[pc, z, ms, z], [z, o, z, z], [ps, z, pc, z], [z, z, z, o]],
        _ => [[pc, ps, z, z], [ms, pc, z, z], [z, z, o, z], [z, z, z, o]],
    }
}

/// `FUN_002ac8d8`'s camera from a record (module docs).
pub fn scene_camera(r: &CamRecord) -> SceneCamera {
    let (z, o) = (Pf::ZERO, Pf::ONE);
    let mut m: [Q; 4] = [[o, z, z, z], [z, o, z, z], [z, z, o, z], [z, z, z, o]];
    for (axis, &a) in r.angles.iter().enumerate() {
        let (s, c) = ecossin(Pf::f(a));
        let rm = rot(axis, s, c);
        m = m.map(|q| apply(&rm, q));
    }
    let v = |q: Q, neg: bool| [0, 1, 2].map(|k| if neg { (-q[k]).to_f32() } else { q[k].to_f32() });
    SceneCamera { eye: r.eye, rows: [v(m[2], true), v(m[0], true), v(m[1], false)], tan_half_fov: r.tan_half_fov, cut: r.is_cut() }
}

/// Alpha (of 0x80) of `FadeToBlack(n)` step `k` (0-based): `0x80 − i·0x80/n` with i = n − 1 − k.
pub fn fade_to_black_alpha(n: u32, k: u32) -> u32 { 0x80 - (n - 1 - k) * 0x80 / n }

/// Black coverage after steps 0..=k of `FadeToBlack(n)` drawn over one held image (each quad blends
/// `Cd·(1 − a/128)`).
pub fn fade_to_black_coverage(n: u32, k: u32) -> f32 {
    1.0 - (0..=k).map(|j| 1.0 - fade_to_black_alpha(n, j) as f32 / 128.0).product::<f32>()
}

impl ScenePlayer {
    /// `DialogStreamStart(id)`: the scene's chunks are already loaded (`Scene::load`). Call on the gameplay
    /// frame whose update triggers the scene; [`ScenePlayer::start_audio`] gives that frame's requests.
    pub fn start(scene: Arc<Scene>, ctx: SceneContext) -> ScenePlayer {
        ScenePlayer {
            scene,
            ctx,
            phase: Phase::FadeIn(0),
            tick: 0,
            chunk: 0,
            chunk_tick: 0,
            fade: Pf::ONE,
            skipped: false,
            black: 0.0,
            from_black: false,
            last_camera: None,
            last_actors: Vec::new(),
            last_subtitle: None,
        }
    }

    /// The scene follows one that just ended: the game's start fade (`fade_to_black(6)`) draws over the last image
    /// shown, and the frames in between were never shown (`0x15f5d8`: `finish_cutscene` and `start_scene` both skip
    /// their frame's draw), so that image is the last scene's black end: the start fade stays black.
    pub fn start_from_black(&mut self) { self.from_black = true; }

    /// The trigger frame's requests (`music_Pause(0)`).
    pub fn start_audio(&self) -> Vec<AudioRequest> { vec![AudioRequest::PauseMusic] }

    pub fn scene(&self) -> &Arc<Scene> { &self.scene }
    /// 0x16cd10.
    pub fn scene_id(&self) -> usize { self.scene.index }
    /// 0x16cd14.
    pub fn scene_tick(&self) -> i32 { self.tick }
    pub fn chunk(&self) -> usize { self.chunk }
    pub fn done(&self) -> bool { self.phase == Phase::Done }
    /// Frames the whole scene takes from the frame after the trigger to the last fade frame (unskipped).
    pub fn total_frames(&self) -> u32 {
        START_FADE_FRAMES + 1 + AUDIO_LEAD + 1 + (self.scene.end_tick() as u32 - 1) + 1 + END_FADE_FRAMES + 1
    }

    fn out(&self, frame: Frame, world_runs: bool, black: f32) -> SceneTick {
        SceneTick {
            frame,
            scene_tick: self.tick,
            world_runs,
            camera: self.last_camera,
            actors: self.last_actors.clone(),
            black,
            fade: self.fade.to_f32(),
            subtitle: self.last_subtitle.clone(),
            audio: Vec::new(),
            hud_hidden: true,
            end: None,
        }
    }

    /// One frame (module docs). `pad` is the pad record of this frame (held 0x13cae0, pressed 0x13cae4).
    pub fn tick(&mut self, pad: &PadState) -> SceneTick {
        match self.phase {
            Phase::FadeIn(k) => {
                self.black = if self.from_black { 1.0 } else { fade_to_black_coverage(START_FADE_FRAMES, k) };
                self.phase = if k + 1 < START_FADE_FRAMES { Phase::FadeIn(k + 1) } else { Phase::Black(0) };
                // The fade draws over the last gameplay image: no camera change, no actors yet.
                self.out(Frame::FadeIn(k), false, self.black)
            }
            Phase::Black(k) => {
                let n = 1 + AUDIO_LEAD + 1;
                self.phase = if k + 1 < n { Phase::Black(k + 1) } else { Phase::Playing };
                let mut o = self.out(Frame::Black(k), k + 1 == n, 1.0);
                if k == 1 {
                    // After the fade: the stream is buffered, 0x16cd28 = −3, `continue_audio_stream_if_ready`.
                    o.audio.push(AudioRequest::Speech { scene: self.scene.index, language: self.ctx.language, start_tick: -(AUDIO_LEAD as i32), lead: AUDIO_LEAD });
                }
                o
            }
            Phase::Playing => self.play(pad),
            Phase::FadeOut(k) => {
                self.black = fade_to_black_coverage(END_FADE_FRAMES, k);
                self.phase = if k + 1 < END_FADE_FRAMES { Phase::FadeOut(k + 1) } else { Phase::Finished };
                self.out(Frame::FadeOut(k), false, self.black)
            }
            Phase::Finished | Phase::Done => {
                let first = self.phase == Phase::Finished;
                self.phase = Phase::Done;
                self.fade = Pf::ZERO;
                let mut o = self.out(Frame::Finished, false, 1.0);
                o.subtitle = None;
                if first {
                    o.audio.push(AudioRequest::ResumeMusic { after: MUSIC_RESUME_TICKS });
                    o.end = Some(SceneEnd {
                        skipped: self.skipped,
                        hero_state: 0,
                        ground_snap_up: 0.5,
                        ground_snap_min: 2.0,
                        ground_snap_max_dz: 4.5,
                        tan_half_fov: GAMEPLAY_TAN_HALF_FOV.to_f32(),
                        music_resume_after: MUSIC_RESUME_TICKS,
                    });
                }
                o
            }
        }
    }

    /// `CutsceneModeUpdate` (0x2aca80).
    fn play(&mut self, pad: &PadState) -> SceneTick {
        self.fade -= FADE_STEP;
        self.tick += 1;
        self.chunk_tick += 1;
        if self.fade < Pf::ZERO { self.fade = Pf::ZERO; }
        let mut end = self.scene.end_tick() <= self.tick;
        if SKIP_MIN_TICKS <= self.tick && self.fade == Pf::ZERO {
            let start = pad.pressed & button::START != 0;
            let all_shoulders = pad.held & 0xf == 0xf;
            let skip = if !self.ctx.start_alone_skips() || !start { start && all_shoulders } else { true };
            if skip && !end { self.skipped = true; }
            end |= skip;
        }
        if end {
            // FUN_002ac608 runs inside this frame: its FadeToBlack's first vsync shows the last image.
            let mut o = self.out(Frame::EndHold, false, 0.0);
            o.audio.push(AudioRequest::StopSpeech);
            self.phase = Phase::FadeOut(0);
            return o;
        }
        if self.chunk_tick >= self.scene.ticks_per_chunk() {
            self.chunk += 1;
            self.chunk_tick = 0;
        }
        let c = &self.scene.chunks[self.chunk];
        let rec = c.camera[self.chunk_tick as usize];
        let camera = scene_camera(&rec);
        let f = (self.chunk_tick >> 1) as u8;
        let mut t = if self.chunk_tick & 1 != 0 { 0.5f32 } else { 0.0 };
        if rec.is_cut() && self.chunk_tick & 1 != 0 { t = 1.0; }
        let (tp, one_minus) = (Pf::f(t), Pf::ONE - Pf::f(t));
        self.last_actors = c
            .actors
            .iter()
            .enumerate()
            .map(|(k, a)| {
                let (pa, pb) = (a.positions[f as usize], a.positions[f as usize + 1]);
                let position = [0, 1, 2].map(|i| (one_minus * Pf::f(pa[i]) + tp * Pf::f(pb[i])).to_f32());
                ActorPose { actor: k, class: a.class, chunk: self.chunk, frame_a: f, frame_b: f + 1, t, position }
            })
            .collect();
        self.last_camera = Some(camera);
        self.last_subtitle = if self.ctx.subtitles {
            let lang = self.ctx.subtitle_language();
            self.scene.subtitle_at(self.tick).map(|s| SubtitleLine { start: s.start, end: s.end, text: s.text[lang].clone() })
        } else {
            None
        };
        let fade = self.fade.to_f32();
        self.out(Frame::Playing, true, fade)
    }
}

/// Scene k's region for this build (the port runs NTSC).
pub const REGION: Region = Region::Ntsc;

// ---------------------------------------------------------------------------------------------------
// What the rest of the game sees of a running scene

/// Moby classes that are hidden (mode |= 0x80) for the length of every scene and shown again at its end
/// (`DialogStreamStart` 0x2ac330 / `FUN_002ac608`: every live moby of class 74 or 203 whose mode lacks 0x80).
pub const SCENE_HIDDEN_CLASSES: [i16; 2] = [74, 203];
/// The mode bit those classes get (MobyProc skips `mode & 0x81`).
pub const SCENE_HIDDEN_BIT: u16 = 0x80;

/// The running scene as the moby loop reads it during mode 2: 0x16cd10 (the scene id), 0x16cd14 (the scene tick)
/// and the actor slots 0x16ce58[k] (`CreateMoby` actors of chunk 0's records). The scene's `CutsceneModeUpdate`
/// runs the moby loop **before** it advances the tick and re-poses the actors, so a class sees the previous tick's
/// values; the engine publishes them in that order. Classes read it through
/// `Services::cinematic.scene` (the cutscene FX driver 1546, `classes::cutscene_fx`).
#[derive(Clone, Debug)]
pub struct SceneState {
    /// 0x16cd10.
    pub id: usize,
    /// 0x16cd14.
    pub tick: i32,
    /// 0x16ce58[k], in actor-record order.
    pub actors: Vec<SceneActorState>,
}

/// One actor moby of the running scene: its class, its position (moby+0x10), its animation fields and the class
/// animation with the streamed sequence in its extra slot (`class+0x48 + slot·4`), so a joint point of the actor
/// (`FUN_002645a8`) evaluates exactly the pose the renderer draws.
#[derive(Clone)]
pub struct SceneActorState {
    /// The actor's moby in the moby table (`CreateMoby` in `FUN_00259288`: a dynamic slot, mode |= 6 so the moby loop
    /// never runs it), the value of 0x16ce58[k]: what a class hands to the general effect code as the owner (e.g.
    /// `SpawnBeamExplosion`'s moby, which its flash shells need). None: the table was full.
    pub moby: Option<crate::moby_runtime::MobyId>,
    pub o_class: i16,
    /// moby+0x10.
    pub position: [f32; 3],
    /// moby+0x2c (the class scale).
    pub scale: f32,
    /// The class animation with the streamed sequence in slot `state.seq_a`.
    pub anim: Arc<rc_formats::moby_anim::MobyAnimClass>,
    /// moby+0x50..0x54 (key frames f / f + 1 of the streamed slot, blend t).
    pub state: rc_formats::moby_anim::AnimState,
    /// The class's joint lists (class header `joints`: the first byte list of each), for `FUN_002645a8`.
    pub joint_lists: Arc<Vec<Vec<u8>>>,
}

impl std::fmt::Debug for SceneActorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneActorState").field("moby", &self.moby).field("o_class", &self.o_class).field("position", &self.position).field("state", &self.state).finish()
    }
}

impl SceneActorState {
    /// `FUN_002645a8(actor, list, out)`: the world point of the last joint of joint list `list` in the actor's pose
    /// (as `World::joint_point`): `q = P.r3 · (scale / 1024)`, rows = identity (`CreateMoby` leaves the rotation 0
    /// and nothing turns a scene actor: orientation is in its root joint), `out = q + position`. Without the list
    /// the point is the actor's origin. Native `f32`.
    pub fn joint_point(&self, list: usize) -> [f32; 4] {
        let chain = self.joint_lists.get(list).filter(|c| !c.is_empty());
        let t = match chain {
            Some(chain) => rc_formats::moby_anim::evaluate_chain(&self.anim, &self.state, None, chain)[3],
            None => [0.0, 0.0, 0.0, 1.0],
        };
        let k = self.scale * (1.0 / 1024.0);
        [t[0] * k + self.position[0], t[1] * k + self.position[1], t[2] * k + self.position[2], t[3]]
    }

    /// `MobyAttachToJoint` 0x264508(actor, list, out): the world matrix of the last joint of the class's joint list
    /// `list` (its second argument is a joint-**list** index: `MobyMarkJointChain` 0x268d80 reads the list from class
    /// header +0x1c, as `FUN_002645a8` does): rows 0..2 the joint's axes, with its animated scale, times the actor's
    /// identity rotation; row 3 its point `P.r3·(scale/1024) + position`. Without the list: the origin frame.
    pub fn joint_matrix(&self, list: usize) -> [[f32; 4]; 4] {
        let chain = self.joint_lists.get(list).filter(|c| !c.is_empty());
        let m = match chain {
            Some(chain) => rc_formats::moby_anim::evaluate_chain(&self.anim, &self.state, None, chain),
            None => [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
        };
        let k = self.scale * (1.0 / 1024.0);
        let t = m[3];
        [m[0], m[1], m[2], [t[0] * k + self.position[0], t[1] * k + self.position[1], t[2] * k + self.position[2], t[3]]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::moby_anim::{MobyFrame, MobyFrameHeader, MobySequence, MobySequenceHeader};
    use rc_formats::scene::{ChunkHeader, SceneActor, SceneChunk, Subtitle};

    /// A synthetic scene: `end` ticks, one actor whose track x = 2·(chunk·48 + frame), camera eye.x = scene
    /// tick of the record, cut on `cuts`, one subtitle line 100..150.
    fn synth(end: i32, cuts: &[i32]) -> Arc<Scene> {
        let tpc = 96;
        let n = ((end + tpc - 1) / tpc) as usize;
        let chunks = (0..n)
            .map(|i| {
                let records = (end - i as i32 * tpc).min(tpc) as usize + 1;
                let camera = (0..records)
                    .map(|r| {
                        let tick = (i * 96 + r) as i32;
                        CamRecord { eye: [tick as f32, 0.0, 0.0], cut: cuts.contains(&tick) as u32, angles: [0.3, -0.2, 1.1], tan_half_fov: 0.414 }
                    })
                    .collect();
                let frames = 49u8;
                let frame = MobyFrame { header: MobyFrameHeader::default(), quats: vec![], scales: vec![], trans: vec![], payload: vec![] };
                let actor = SceneActor {
                    offset: 0,
                    class: 10,
                    unknown_04: 16,
                    unknown_08: 0,
                    track_offset: 0,
                    frame_offsets: vec![0; frames as usize],
                    sequence: MobySequence { header: MobySequenceHeader { frame_count: frames, ..Default::default() }, frames: vec![frame; frames as usize], triggers: vec![] },
                    positions: (0..frames as usize).map(|f| [2.0 * (i * 48 + f) as f32, 1.0, 2.0, 0.0]).collect(),
                };
                let sub = Subtitle { start: 100, end: 150, text_offsets: [0; 5], pad: 0, text: [b"en".to_vec(), b"fr".to_vec(), b"de".to_vec(), b"es".to_vec(), b"it".to_vec()] };
                let header = ChunkHeader { end_tick: end as i16, audio_start: -6, unknown_0a: -1, actor_count: 1, ..Default::default() };
                SceneChunk { header, camera, actors: vec![actor], subtitles: vec![sub] }
            })
            .collect();
        Arc::new(Scene::from_chunks(5, Region::Ntsc, chunks).unwrap())
    }

    fn ctx() -> SceneContext { SceneContext { game_beaten: false, completes: 0, replay: false, level: 1, language: 0, subtitles: true } }

    fn pad(held: u32, pressed: u32) -> PadState { PadState { held, pressed, ..PadState::default() } }

    /// Runs to the first Playing frame; returns the frames before it.
    fn to_first_tick(p: &mut ScenePlayer) -> Vec<SceneTick> {
        let mut v = Vec::new();
        loop {
            let t = p.tick(&PadState::default());
            let playing = t.frame == Frame::Playing;
            v.push(t);
            if playing { return v; }
        }
    }

    #[test]
    fn start_sequence_fade_black_and_speech_lead() {
        let mut p = ScenePlayer::start(synth(300, &[]), ctx());
        assert_eq!(p.start_audio(), [AudioRequest::PauseMusic]);
        let v = to_first_tick(&mut p);
        // 6 fade frames, 5 black frames, then tick 1.
        assert_eq!(v.len(), 12);
        let alphas: Vec<u32> = (0..6).map(|k| fade_to_black_alpha(6, k)).collect();
        assert_eq!(alphas, [22, 43, 64, 86, 107, 128]);
        assert!(v[..6].iter().zip(1..).all(|(t, k)| t.frame == Frame::FadeIn(k - 1) && !t.world_runs && t.camera.is_none()));
        assert_eq!(v[5].black, 1.0);
        assert!(v[..5].windows(2).all(|w| w[0].black < w[1].black));
        // The speech starts 3 vsyncs + the skipped render before tick 1.
        let speech: Vec<usize> = v.iter().enumerate().filter(|(_, t)| t.audio.iter().any(|a| matches!(a, AudioRequest::Speech { .. }))).map(|(i, _)| i).collect();
        assert_eq!(speech, [7]);
        assert_eq!(v[7].audio[0], AudioRequest::Speech { scene: 5, language: 0, start_tick: -3, lead: 3 });
        assert!(v[6..11].iter().all(|t| t.black == 1.0 && t.hud_hidden));
        assert!(v[10].world_runs && !v[9].world_runs);
        let t1 = &v[11];
        assert_eq!((t1.scene_tick, t1.camera.unwrap().eye[0]), (1, 1.0));
        assert_eq!(t1.fade, (Pf::ONE - FADE_STEP).to_f32());
    }

    #[test]
    fn fade_clears_in_three_ticks() {
        let mut p = ScenePlayer::start(synth(300, &[]), ctx());
        let first = to_first_tick(&mut p).pop().unwrap();
        let f2 = p.tick(&PadState::default());
        let f3 = p.tick(&PadState::default());
        assert_eq!(first.fade.to_bits(), 0x3f28_f5c3); // 0.66
        assert!((f2.fade - 0.32).abs() < 1e-6);
        assert_eq!(f3.fade, 0.0);
        assert_eq!(f3.black, 0.0);
    }

    #[test]
    fn chunk_rollover_camera_continuity_and_actor_lerp() {
        let s = synth(300, &[107]);
        let mut p = ScenePlayer::start(s.clone(), ctx());
        let mut frames = vec![to_first_tick(&mut p).pop().unwrap()];
        while frames.last().unwrap().frame == Frame::Playing { frames.push(p.tick(&PadState::default())); }
        let playing: Vec<&SceneTick> = frames.iter().filter(|t| t.frame == Frame::Playing).collect();
        assert_eq!(playing.len(), 299);
        for (k, t) in playing.iter().enumerate() {
            let tick = k as i32 + 1;
            assert_eq!(t.scene_tick, tick);
            // The camera shows record `tick` of the whole scene, across the chunk boundary at 96 / 192.
            assert_eq!(t.camera.unwrap().eye[0], tick as f32, "tick {tick}");
            let a = t.actors[0];
            assert_eq!(a.chunk, tick as usize / 96);
            let ct = tick as u32 % 96;
            assert_eq!((a.frame_a, a.frame_b), ((ct >> 1) as u8, (ct >> 1) as u8 + 1));
            // Track x = 2·(chunk·48 + f): the lerp gives tick exactly (t = 0 or 0.5), except the cut snap.
            if tick == 107 {
                assert_eq!((a.t, a.position[0]), (1.0, 2.0 * (48 + 6) as f32));
            } else {
                assert_eq!(a.t, if ct & 1 == 1 { 0.5 } else { 0.0 });
                let expect = 2.0 * (a.chunk * 48) as f32 + ct as f32;
                assert_eq!(a.position[0], expect, "tick {tick}");
            }
        }
        // Record 0 of chunk 1 = record 96 of chunk 0.
        assert_eq!(s.chunks[0].camera[96], s.chunks[1].camera[0]);
        // Subtitles on 100..=150 only.
        assert!(playing[98].subtitle.is_none());
        assert_eq!(playing[99].subtitle.as_ref().unwrap().text, b"en");
        assert!(playing[149].subtitle.is_some() && playing[150].subtitle.is_none());
    }

    #[test]
    fn end_fade_and_hand_back() {
        let mut p = ScenePlayer::start(synth(100, &[]), ctx());
        let mut all = to_first_tick(&mut p);
        while !p.done() { all.push(p.tick(&PadState::default())); }
        let n = all.len() as u32;
        assert_eq!(n, p.total_frames());
        let tail: Vec<Frame> = all[all.len() - 15..].iter().map(|t| t.frame).collect();
        assert_eq!(tail[0], Frame::Playing);
        assert_eq!(tail[1], Frame::EndHold);
        assert_eq!(&tail[2..14], &(0..12).map(Frame::FadeOut).collect::<Vec<_>>()[..]);
        assert_eq!(tail[14], Frame::Finished);
        assert_eq!(all[all.len() - 15].scene_tick, 99);
        assert!(all[all.len() - 14].audio.contains(&AudioRequest::StopSpeech));
        let last = all.last().unwrap();
        let e = last.end.unwrap();
        assert_eq!((e.hero_state, e.tan_half_fov.to_bits(), e.music_resume_after, e.skipped), (0, 0x3f21_47ae, 30, false));
        assert!(last.audio.contains(&AudioRequest::ResumeMusic { after: 30 }));
        assert_eq!(fade_to_black_alpha(12, 0), 11);
        assert_eq!(all[all.len() - 2].black, 1.0);
    }

    fn run_with(ctx: SceneContext, press_at: i32, held: u32) -> (bool, i32) {
        let mut p = ScenePlayer::start(synth(300, &[]), ctx);
        to_first_tick(&mut p);
        loop {
            let tk = p.scene_tick() + 1;
            let t = p.tick(&if tk >= press_at { pad(held, button::START) } else { PadState::default() });
            if t.frame == Frame::EndHold { return (true, t.scene_tick); }
            if tk > 250 { return (false, tk); }
        }
    }

    #[test]
    fn skip_gating() {
        const SHOULDERS: u32 = button::L1 | button::L2 | button::R1 | button::R2;
        // Start alone does nothing mid-game (level > 0, not beaten).
        assert!(!run_with(ctx(), 20, 0).0);
        // Start + all four shoulders skips; not before tick 18.
        assert_eq!(run_with(ctx(), 5, SHOULDERS), (true, 18));
        assert_eq!(run_with(ctx(), 40, SHOULDERS), (true, 40));
        // Three shoulders are not enough.
        assert!(!run_with(ctx(), 20, button::L1 | button::L2 | button::R1).0);
        // Beaten / completed / replay / level 0: Start alone.
        for c in [SceneContext { game_beaten: true, ..ctx() }, SceneContext { completes: 1, ..ctx() }, SceneContext { replay: true, ..ctx() }, SceneContext { level: 0, ..ctx() }] {
            assert_eq!(run_with(c, 30, 0), (true, 30));
        }
        let mut p = ScenePlayer::start(synth(300, &[]), SceneContext { level: 0, ..ctx() });
        to_first_tick(&mut p);
        while !p.done() { let t = p.tick(&pad(0, button::START)); if let Some(e) = t.end { assert!(e.skipped); } }
    }

    #[test]
    fn camera_rows_are_rz_ry_rx_columns() {
        for angles in [[0.0f32, 0.0, 0.0], [1.911_753_5, -1.110_223e-16, 1.574_286_9], [0.3, -0.7, 2.9], [-2.0, 0.4, -1.0]] {
            let c = scene_camera(&CamRecord { eye: [1.0, 2.0, 3.0], cut: 0, angles, tan_half_fov: 0.414 });
            let [a, b, g] = angles.map(|x| x as f64);
            let rx = [[1.0, 0.0, 0.0], [0.0, a.cos(), -a.sin()], [0.0, a.sin(), a.cos()]];
            let ry = [[b.cos(), 0.0, b.sin()], [0.0, 1.0, 0.0], [-b.sin(), 0.0, b.cos()]];
            let rz = [[g.cos(), -g.sin(), 0.0], [g.sin(), g.cos(), 0.0], [0.0, 0.0, 1.0]];
            let mul = |p: [[f64; 3]; 3], q: [[f64; 3]; 3]| -> [[f64; 3]; 3] { std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| p[i][k] * q[k][j]).sum())) };
            let m = mul(rz, mul(ry, rx));
            let col = |j: usize| [m[0][j], m[1][j], m[2][j]];
            let expect = [col(2).map(|v| -v), col(0).map(|v| -v), col(1)];
            for (r, e) in c.rows.iter().zip(expect) {
                for k in 0..3 { assert!((r[k] as f64 - e[k]).abs() < 2e-5, "angles {angles:?}: rows {:?} vs {expect:?}", c.rows); }
            }
        }
        // Novalis scene 5's first record looks along −x and slightly up (towards the incoming ship).
        let c = scene_camera(&CamRecord { eye: [162.6, 129.8, 82.7], cut: 0, angles: [1.911_753_5, -1.110_223e-16, 1.574_286_9], tan_half_fov: 0.414 });
        assert!(c.rows[0][0] < -0.9 && c.rows[0][2] > 0.3 && c.rows[2][2] > 0.9);
    }

    #[test]
    fn ecossin_matches_libm() {
        for k in -300..=300 {
            let th = k as f32 * 0.01;
            let (s, c) = ecossin(Pf::f(th));
            assert!((s.to_f32() - th.sin()).abs() < 2e-5 && (c.to_f32() - th.cos()).abs() < 2e-5, "{th}: {s:?} {c:?}");
        }
    }
}
