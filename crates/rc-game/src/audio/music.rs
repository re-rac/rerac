//! Music (docs/plan/audio.md §4): the EE state machine `music_Update` (level01 0x27a688) with its players,
//! the stream callbacks, the music trigger boxes (sound-instance class 6, 0x31a128), and the IOP side, a VAG
//! stream player that feeds SPU voices.
//!
//! EE sources: `music_start_track` 0x279fa8, `music_preseek_track` 0x279ed0, `music_StartTrackBody` 0x27a080,
//! `music_Transition` 0x27a168, the request `FUN_0027a248`, `music_UpdateStream` 0x27a4a0, and the callbacks
//! (disassembled here): start 0x27af20, body queued 0x27aec8, preseek 0x27ae18, transition start 0x27ae78,
//! buffered 0x27ad88, time remaining 0x27afb8, liveness 0x27af60. Player records: main 0x151704, transition
//! 0x15173c (`+0 handle, +4 track, +6 vol, +8 flags, +0xa state, +0x10 stinger flag, +0x18 time remaining`),
//! request 0x1516f2/f3, phase 0x1516f0, fade T/F 0x1516f4/f8, request timer 0x1516fc.
//!
//! Level start: track 0 plays once (stream flags 0x20); the first `music_Update` after its handle comes back
//! queues track 1 behind it with flags 0x24 (looping), so Start runs into Loop seamlessly.
//!
//! IOP side (989snd's stream player is not reversed, all inferred): one SPU voice per stream at pitch
//! `rate·0x1000/48000`, parts queued behind the running one continue with the decoder history, a looping
//! last part restarts, time remaining = output samples left in the current part. The stream voice volume is
//! the block-sound law with a 127 sfx volume: `MakeVolume(127, 0, min(127, 127·vol >> 10), pan, 127, 0)`,
//! then the group scale and `>> 1`.

use super::voices::{VoiceManager, VoiceSlot, VoiceUse};
use super::{Spu, StreamPart, VoiceData};
use rc_formats::sound_bank::SoundInstance;
use rc_formats::vag;
use std::collections::BTreeMap;
use std::sync::Arc;

/// One music VAG: its body up to and including the end frame (the 0x7 pad frame is never played).
#[derive(Clone, Debug)]
pub struct MusicTrack {
    pub name: String,
    pub rate: u32,
    pub body: Arc<[u8]>,
}

impl MusicTrack {
    pub fn from_vag(bytes: &[u8]) -> rc_formats::buf::Result<Self> {
        let (h, body) = vag::parse_vag(bytes)?;
        let ext = vag::sample_extent(body, 0)?;
        Ok(MusicTrack { name: h.name, rate: h.sample_rate, body: Arc::from(&body[..ext.bytes()]) })
    }
    /// Length in seconds at the recorded rate.
    pub fn seconds(&self) -> f64 { (self.body.len() / vag::FRAME_BYTES * vag::FRAME_SAMPLES) as f64 / self.rate as f64 }
}

/// Which EE player a command / reply belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerId {
    Main,
    Transition,
}

/// The callback a stream command registered (named by address).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamCallback {
    /// 0x27af20
    Start,
    /// 0x27aec8
    Body,
    /// 0x27ae18
    Preseek,
    /// 0x27ae78
    Transition,
    /// 0x27ad88
    Buffered,
    /// 0x27afb8
    Remaining,
    /// 0x27af60
    Alive,
}

/// EE → IOP stream commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamCommand {
    /// `PlayVAGStreamByLocEx(table[track], vol, group, queue, flags)`: flags 0x20 one-shot, 0x24 looping,
    /// 0x21 preseek (paused until continued); `queue` = a running handle to append to.
    Play { player: PlayerId, track: usize, vol: i32, group: u8, queue: i32, flags: u32, cb: StreamCallback },
    Stop { handle: i32 },
    Continue { handle: i32 },
    SetVolume { player: PlayerId, handle: i32, vol: i32 },
    Query { player: PlayerId, handle: i32, cb: StreamCallback },
}

/// A callback invocation: `value` is the handle, the time remaining or 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamReply {
    pub player: PlayerId,
    pub cb: StreamCallback,
    pub value: i32,
}

/// A `music_Playing` record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Player {
    pub handle: i32,
    pub track: i16,
    pub vol: i16,
    pub flags: u16,
    /// 0 idle, 1 requested, 2 started, 3 buffered, 4 playing, 5 stop, 6 stopping, 7 ended, 8 queue body,
    /// 9 body queued; bit 15 paused.
    pub state: u16,
    /// +0x10: set by `music_Transition` on the transition player.
    pub stinger: i16,
    /// +0x18: last time remaining.
    pub remaining: i32,
}

/// The music EE state.
#[derive(Clone, Debug, Default)]
pub struct Music {
    /// Tracks present in the level header table (`music[15]` non-zero).
    pub present: [bool; 15],
    pub main: Player,
    pub transition: Player,
    /// 0x1516f2 / 0x1516f3.
    pub pending_track: i8,
    pub pending_stinger: i8,
    /// 0x1516f0.
    pub phase: i16,
    /// 0x1516f4 / 0x1516f8: stinger length and quarter.
    pub fade_total: i32,
    pub fade_quarter: i32,
    /// 0x1516fc: request debounce (420 ticks after a change).
    pub timer: i32,
    /// The players' fade word +0x0c bit 15 (`music_Pause` sets it, `music_Unpause` clears it): each live stream is
    /// paused once by its `music_UpdateStream` (state bit 15) and its update stops there until the unpause.
    pub paused: bool,
    /// Commands a callback issued (the preseek retry); they go out with the next batch.
    deferred: Vec<StreamCommand>,
}

/// `FastDecTimer`.
fn fast_dec_timer(t: &mut i32) -> bool {
    if *t == 0 { return true; }
    *t = (*t).max(1) - 1;
    *t < 1
}

/// `ScaleTicks(7)·60.0` = 420 on NTSC.
pub const REQUEST_DEBOUNCE: i32 = 420;

impl Music {
    pub fn new(present: [bool; 15]) -> Self { Music { present, pending_track: -1, pending_stinger: -1, ..Default::default() } }

    fn player(&mut self, p: PlayerId) -> &mut Player {
        match p {
            PlayerId::Main => &mut self.main,
            PlayerId::Transition => &mut self.transition,
        }
    }

    fn has(&self, k: i32) -> bool { usize::try_from(k).ok().and_then(|k| self.present.get(k)).copied().unwrap_or(false) }

    /// `music_start_track(track, flags, vol)` (0x279fa8).
    pub fn start_track(&mut self, track: i16, flags: u16, vol: i16, cmds: &mut Vec<StreamCommand>) {
        if self.main.handle != 0 || !self.has(track as i32) { return; }
        self.main = Player { handle: -1, track, vol, flags, state: 1, ..self.main };
        cmds.push(StreamCommand::Play { player: PlayerId::Main, track: track as usize, vol: vol as i32, group: 1, queue: 0, flags: 0x20, cb: StreamCallback::Start });
    }

    /// `music_preseek_track` (0x279ed0): like the start, paused (flags 0x21).
    pub fn preseek_track(&mut self, track: i16, flags: u16, vol: i16, cmds: &mut Vec<StreamCommand>) {
        if self.main.handle != 0 || !self.has(track as i32) { return; }
        self.main = Player { handle: -1, track, vol, flags, state: 1, ..self.main };
        cmds.push(StreamCommand::Play { player: PlayerId::Main, track: track as usize, vol: vol as i32, group: 1, queue: 0, flags: 0x21, cb: StreamCallback::Preseek });
    }

    /// `music_StartTrackBody` (0x27a080): queue `track + 1` behind the running stream.
    fn start_track_body(&mut self, track: i16, flags: u16, vol: i16, cmds: &mut Vec<StreamCommand>) {
        let m = self.main;
        if m.state == 9 || m.handle == 0 || m.handle == -1 || !self.has(track as i32 + 1) { return; }
        self.main = Player { track, flags, state: 9, vol, ..m };
        let f = if flags & 1 != 0 { 0x24 } else { 0x20 };
        cmds.push(StreamCommand::Play { player: PlayerId::Main, track: track as usize + 1, vol: vol as i32, group: 1, queue: m.handle, flags: f, cb: StreamCallback::Body });
    }

    /// `music_Transition(track, stinger, flags, vol)` (0x27a168).
    fn transition_to(&mut self, track: i16, stinger: i16, flags: u16, vol: i16, cmds: &mut Vec<StreamCommand>) -> bool {
        if self.transition.handle != 0 || !self.has(stinger as i32) { return false; }
        self.transition = Player { handle: -1, track, vol, flags, state: 1, stinger: 1, remaining: 48000 };
        cmds.push(StreamCommand::Play { player: PlayerId::Transition, track: stinger as usize, vol: vol as i32, group: 1, queue: 0, flags: 0x20, cb: StreamCallback::Transition });
        true
    }

    /// `FUN_0027a248(track, stinger)`: a music box request. Taken at once when both players are idle,
    /// otherwise remembered for `music_Update`.
    pub fn request(&mut self, track: i16, stinger: i16) {
        let differs = self.main.track != track || self.pending_track != -1;
        if differs && self.main.state == 0 && self.transition.state == 0 {
            self.main.track = track;
            return;
        }
        if differs {
            self.pending_track = track as i8;
            self.pending_stinger = stinger as i8;
        }
    }

    /// Apply one callback.
    pub fn reply(&mut self, r: StreamReply) {
        let v = r.value;
        let phase = self.phase;
        let p = self.player(r.player);
        let mut new_phase = None;
        let mut retry = None;
        match r.cb {
            StreamCallback::Start => {
                p.handle = v;
                if v == 0 { p.state = 0 } else if p.state == 1 { p.state = 8 }
            }
            StreamCallback::Body => {
                if v < 0 { p.handle = v; }
                if v == 0 {
                    p.state = 0;
                } else if p.state == 9 {
                    p.state = 4;
                    if p.stinger != 0 { new_phase = Some(1); }
                }
            }
            StreamCallback::Preseek => {
                p.handle = v;
                if v == 0 {
                    // The game preseeks the main track again at once.
                    retry = Some((p.track, p.flags, p.vol));
                } else if p.state == 1 {
                    p.state = 2;
                }
            }
            StreamCallback::Transition => {
                p.handle = v;
                if v == 0 {
                    p.state = 0;
                } else if p.state == 1 {
                    p.state = 4;
                    if p.stinger != 0 { new_phase = Some(1); }
                }
            }
            StreamCallback::Buffered => {
                if v != 0 && p.state == 2 { p.state = 3; }
            }
            StreamCallback::Remaining => {
                p.remaining = v;
                if p.stinger != 0 && phase == 1 && v != 0 {
                    self.phase = 2;
                    self.fade_total = v;
                    self.fade_quarter = v / 4;
                }
            }
            StreamCallback::Alive => {
                if p.handle == -1 {
                    p.handle = v;
                    if v == 0 { p.state = 7; }
                }
            }
        }
        if let Some(ph) = new_phase { self.phase = ph; }
        if let Some((t, f, vol)) = retry {
            let mut cmds = Vec::new();
            self.preseek_track(t, f, vol, &mut cmds);
            self.deferred.extend(cmds);
        }
    }

    /// `music_UpdateStream` (0x27a4a0, Lombyte `music_update_stream`) for one player. Paused ([`Music::paused`]): the
    /// stream is paused once (state bit 15; the voices themselves are held by the audio layer's music pause) and the
    /// update ends there, so no fade, stop, continue or query runs under the pause; unpaused, the bit is cleared.
    fn update_stream(&mut self, id: PlayerId, cmds: &mut Vec<StreamCommand>) {
        let paused = self.paused;
        let p = self.player(id);
        let s = p.state;
        if s == 9 || p.handle == 0 || p.handle == -1 {
            if s == 7 || p.handle == 0 { p.state = 0; }
            return;
        }
        if s == 5 {
            cmds.push(StreamCommand::Stop { handle: p.handle });
            p.state = 6;
        }
        if paused {
            p.state |= 0x8000;
        } else if p.state & 0x8000 != 0 {
            p.state ^= 0x8000;
        }
        if p.state & 0x8000 != 0 { return; }
        let s = p.state;
        if matches!(s, 1 | 8 | 9) { return; }
        if s == 2 || s == 3 {
            if s == 2 { cmds.push(StreamCommand::Query { player: id, handle: p.handle, cb: StreamCallback::Buffered }); }
        } else {
            let h = p.handle;
            p.handle = -1;
            cmds.push(StreamCommand::Query { player: id, handle: h, cb: StreamCallback::Remaining });
            cmds.push(StreamCommand::Query { player: id, handle: h, cb: StreamCallback::Alive });
        }
    }

    /// `music_Update` (0x27a688), once per frame from `sound_update`. Dialogue (0x151720) is not ported.
    pub fn update(&mut self, cmds: &mut Vec<StreamCommand>) {
        cmds.append(&mut self.deferred);
        let m = self.main;
        if m.state & 0x8000 == 0 {
            if m.handle == 0 && m.flags & 1 != 0 && self.pending_track == -1 {
                self.start_track(m.track, m.flags, m.vol, cmds);
            } else if m.state != 9 && m.handle != 0 && m.handle != -1 && m.state == 8 {
                self.start_track_body(m.track, m.flags, m.vol, cmds);
            }
        }
        if fast_dec_timer(&mut self.timer) && self.pending_track != -1 && self.phase == 0 {
            if self.main.track == self.pending_track as i16 {
                self.pending_track = -1;
            } else {
                let (t, s, f, v) = (self.pending_track as i16, self.pending_stinger as i16, self.main.flags, self.main.vol);
                // Without a stinger nothing is started here (the new track waits for the main player to go idle).
                if s == -1 || self.transition_to(t, s, f, v, cmds) { self.timer = REQUEST_DEBOUNCE; }
            }
        }
        let (m, t) = (self.main, self.transition);
        if m.state != 9 && m.handle != -1 && t.state & 0x8000 == 0 && m.state & 0x8000 == 0 {
            match self.phase {
                2 => {
                    let f = self.fade_quarter.max(1);
                    let v = (m.vol as i32 * (f - (self.fade_total - t.remaining))) / f;
                    if v < 1 || ((t.state != 4 || t.stinger == 0) && t.handle == 0) || m.handle == 0 {
                        self.phase = 3;
                        self.main.state = 5;
                    } else if m.state != 9 {
                        self.main.handle = -1;
                        cmds.push(StreamCommand::SetVolume { player: PlayerId::Main, handle: m.handle, vol: v });
                    }
                }
                3 => {
                    if m.state == 0 && (t.stinger != 0 || t.handle == 0) {
                        let track = self.pending_track as i16;
                        self.preseek_track(track, m.flags, m.vol, cmds);
                        self.phase = 4;
                        self.pending_track = -1;
                    }
                }
                4 => {
                    if m.state == 3 && (t.remaining < self.fade_quarter || t.state != 4 || (t.stinger == 0 && t.handle == 0)) {
                        cmds.push(StreamCommand::Continue { handle: m.handle });
                        self.phase = 5;
                        self.main.state = 8;
                    }
                }
                5 if t.state != 4 || t.stinger == 0 => self.phase = 0,
                _ => {}
            }
        }
        self.update_stream(PlayerId::Main, cmds);
        self.update_stream(PlayerId::Transition, cmds);
    }

    /// One frame: the music boxes (hero position), then `music_Update`. The task-level entry point.
    pub fn tick(&mut self, hero_pos: [f32; 3], boxes: &mut [MusicBox]) -> Vec<StreamCommand> {
        for b in boxes.iter_mut() { b.update(hero_pos, self); }
        let mut cmds = Vec::new();
        self.update(&mut cmds);
        cmds
    }
}

// ---------------------------------------------------------------------------------------------------
// Music boxes

/// A sound instance of class 6 (`0x31a128`): leaving the box through its local +x side requests
/// `tracks[1]` with `stingers[1]`, through any other side `tracks[0]` / `stingers[0]`; it arms while the hero
/// is inside (|local x|, |y|, |z| ≤ 1). Pvars `{s32 track_a, s32 track_b, s32 armed, s16 stinger_a,
/// s16 stinger_b}`; a −1 stinger takes the other one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicBox {
    pub instance: usize,
    pub center: [f32; 3],
    pub inverse: [[f32; 4]; 3],
    pub tracks: [i32; 2],
    pub armed: i32,
    pub stingers: [i16; 2],
}

impl MusicBox {
    pub fn from_instance(index: usize, s: &SoundInstance, pvars: &[u8]) -> Option<Self> {
        if s.o_class != 6 || pvars.len() < 16 { return None; }
        let w = |k: usize| i32::from_le_bytes(pvars[4 * k..4 * k + 4].try_into().unwrap());
        let h = |o: usize| i16::from_le_bytes([pvars[o], pvars[o + 1]]);
        Some(MusicBox { instance: index, center: s.position(), inverse: s.inverse, tracks: [w(0), w(1)], armed: w(2), stingers: [h(0xc), h(0xe)] })
    }

    pub fn update(&mut self, hero: [f32; 3], music: &mut Music) {
        if self.stingers[0] == -1 && self.stingers[1] == -1 && self.tracks == [-1, -1] { return; }
        let v = [hero[0] - self.center[0], hero[1] - self.center[1], hero[2] - self.center[2]];
        let r = &self.inverse;
        let local: [f32; 3] = std::array::from_fn(|k| v[0] * r[0][k] + v[1] * r[1][k] + v[2] * r[2][k]);
        if local.iter().all(|c| c.abs() <= 1.0) {
            self.armed = 1;
            return;
        }
        if self.armed == 0 { return; }
        if self.stingers[1] == -1 { self.stingers[1] = self.stingers[0]; }
        if self.stingers[0] == -1 { self.stingers[0] = self.stingers[1]; }
        if local[0] > 0.0 {
            music.request(self.tracks[1] as i16, self.stingers[1]);
        } else {
            music.request(self.tracks[0] as i16, self.stingers[0]);
        }
        self.armed = 0;
    }
}

// ---------------------------------------------------------------------------------------------------
// IOP: VAG stream player

#[derive(Clone, Debug)]
struct Stream {
    voice: usize,
    generation: u32,
    group: u8,
    /// A preseeked stream waits for `Continue` before its key-on.
    pending: Option<StreamPart>,
    pitch: u16,
    /// The voice registers and ungrouped volume chosen at `Play` (restored by `Continue`).
    vol: [u16; 2],
    basevol: (i16, i16),
}

/// Stream voices use a fast linear attack and release (989snd's stream ADSR is not reversed).
pub const STREAM_ADSR1: u16 = 0x00ff;
pub const STREAM_ADSR2: u16 = 0x1fc0;

/// The IOP stream player.
#[derive(Clone, Debug, Default)]
pub struct Streams {
    streams: BTreeMap<i32, Stream>,
    next: i32,
}

impl Streams {
    fn volume(vm: &VoiceManager, vol: i32, group: u8) -> ([u16; 2], (i16, i16)) {
        let v = ((127 * vol) >> 10).clamp(0, 127);
        let base = vm.make_volume(127, 0, v, 0, 127, 0);
        (vm.voice_registers(base, group as usize), base)
    }

    fn alive(&self, spu: &Spu, h: i32) -> bool {
        self.streams.get(&h).is_some_and(|s| s.pending.is_some() || (spu.voices[s.voice].generation == s.generation && spu.voices[s.voice].active()))
    }

    /// Runs one stream command; returns the callback value.
    pub fn execute(&mut self, cmd: &StreamCommand, tracks: &[Option<Arc<MusicTrack>>; 15], vm: &mut VoiceManager, spu: &mut Spu) -> Option<StreamReply> {
        match *cmd {
            StreamCommand::Play { player, track, vol, group, queue, flags, cb } => {
                let Some(t) = tracks.get(track).and_then(|t| t.as_ref()) else { return Some(StreamReply { player, cb, value: 0 }) };
                let part = StreamPart { data: Arc::clone(&t.body), looped: flags & 4 != 0 };
                if queue > 0 {
                    // Queue behind a running stream. The body callback keeps the handle it has (it only takes an
                    // error value), so a stream that already ended must fail here (0: the player goes idle and
                    // restarts the track), never start a second, untracked music stream.
                    if !self.alive(spu, queue) {
                        self.streams.remove(&queue);
                        return Some(StreamReply { player, cb, value: 0 });
                    }
                    let s = self.streams.get_mut(&queue).unwrap();
                    match &mut s.pending {
                        Some(_) => {}
                        None => spu.voices[s.voice].queue_part(part),
                    }
                    return Some(StreamReply { player, cb, value: queue });
                }
                let Some(i) = vm.allocate(spu, group as usize, 127, 0) else { return Some(StreamReply { player, cb, value: 0 }) };
                self.next += 1;
                let h = self.next;
                let pitch = vag::rate_to_pitch(t.rate);
                let (regs, base) = Self::volume(vm, vol, group);
                let mut s = Stream { voice: i, generation: 0, group, pending: None, pitch, vol: regs, basevol: base };
                if flags & 1 != 0 {
                    s.pending = Some(part);
                } else {
                    spu.voices[i].key_on(VoiceData::Stream { cur: part, queue: Default::default() }, 0, pitch, STREAM_ADSR1, STREAM_ADSR2);
                }
                spu.voices[i].vol = regs;
                s.generation = spu.voices[i].generation;
                vm.voices[i] = VoiceSlot { owner: VoiceUse::Stream { handle: h as u32 }, priority: 127, group, basevol: base, start_tick: 0, generation: s.generation };
                self.streams.insert(h, s);
                Some(StreamReply { player, cb, value: h })
            }
            StreamCommand::Continue { handle } => {
                if let Some(s) = self.streams.get_mut(&handle) {
                    if let Some(part) = s.pending.take() {
                        spu.voices[s.voice].key_on(VoiceData::Stream { cur: part, queue: Default::default() }, 0, s.pitch, STREAM_ADSR1, STREAM_ADSR2);
                        spu.voices[s.voice].vol = s.vol;
                        s.generation = spu.voices[s.voice].generation;
                        vm.voices[s.voice] = VoiceSlot { owner: VoiceUse::Stream { handle: handle as u32 }, priority: 127, group: s.group, basevol: s.basevol, start_tick: 0, generation: s.generation };
                    }
                }
                None
            }
            StreamCommand::Stop { handle } => {
                if let Some(s) = self.streams.remove(&handle) {
                    if spu.voices[s.voice].generation == s.generation { spu.voices[s.voice].key_off(); }
                    if vm.voices[s.voice].owner == (VoiceUse::Stream { handle: handle as u32 }) { vm.voices[s.voice].owner = VoiceUse::Free; }
                }
                None
            }
            StreamCommand::SetVolume { player, handle, vol } => {
                let alive = self.alive(spu, handle);
                if let (true, Some(s)) = (alive, self.streams.get_mut(&handle)) {
                    let (regs, base) = Self::volume(vm, vol, s.group);
                    s.vol = regs;
                    s.basevol = base;
                    spu.voices[s.voice].vol = regs;
                    vm.voices[s.voice].basevol = base;
                }
                Some(StreamReply { player, cb: StreamCallback::Alive, value: if alive { handle } else { 0 } })
            }
            StreamCommand::Query { player, handle, cb } => {
                let alive = self.alive(spu, handle);
                if !alive {
                    if let Some(s) = self.streams.remove(&handle) {
                        if vm.voices[s.voice].owner == (VoiceUse::Stream { handle: handle as u32 }) && vm.voices[s.voice].generation == s.generation {
                            vm.voices[s.voice].owner = VoiceUse::Free;
                        }
                    }
                }
                let value = match cb {
                    StreamCallback::Remaining => self.streams.get(&handle).map_or(0, |s| {
                        let src = if s.pending.is_some() { 0 } else { spu.voices[s.voice].stream_remaining() };
                        (src as u64 * 0x1000 / s.pitch.max(1) as u64) as i32
                    }),
                    _ => if alive { handle } else { 0 },
                };
                Some(StreamReply { player, cb, value })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SPU_VOICES;

    fn present() -> [bool; 15] { std::array::from_fn(|k| k < 9) }

    #[test]
    fn level_start_queues_the_loop() {
        let mut m = Music::new(present());
        let mut cmds = Vec::new();
        m.start_track(0, 1, 0x400, &mut cmds);
        assert_eq!(cmds, vec![StreamCommand::Play { player: PlayerId::Main, track: 0, vol: 0x400, group: 1, queue: 0, flags: 0x20, cb: StreamCallback::Start }]);
        // Before the handle comes back music_Update does nothing.
        let mut c2 = Vec::new();
        m.update(&mut c2);
        assert!(c2.is_empty());
        m.reply(StreamReply { player: PlayerId::Main, cb: StreamCallback::Start, value: 7 });
        assert_eq!(m.main.state, 8);
        let mut c3 = Vec::new();
        m.update(&mut c3);
        assert_eq!(c3, vec![StreamCommand::Play { player: PlayerId::Main, track: 1, vol: 0x400, group: 1, queue: 7, flags: 0x24, cb: StreamCallback::Body }]);
        assert_eq!(m.main.state, 9);
        m.reply(StreamReply { player: PlayerId::Main, cb: StreamCallback::Body, value: 7 });
        assert_eq!((m.main.state, m.main.handle), (4, 7));
        // Playing: every frame asks for the time remaining and liveness.
        let mut c4 = Vec::new();
        m.update(&mut c4);
        assert_eq!(c4.len(), 2);
        assert_eq!(m.main.handle, -1);
        m.reply(StreamReply { player: PlayerId::Main, cb: StreamCallback::Alive, value: 7 });
        assert_eq!(m.main.handle, 7);
    }

    #[test]
    fn requests_debounce_and_transition() {
        let mut m = Music::new(present());
        m.main = Player { handle: 7, track: 0, vol: 0x400, flags: 1, state: 4, ..Default::default() };
        m.request(4, 8);
        assert_eq!((m.pending_track, m.pending_stinger), (4, 8));
        let mut cmds = Vec::new();
        m.update(&mut cmds);
        assert!(cmds.iter().any(|c| matches!(c, StreamCommand::Play { player: PlayerId::Transition, track: 8, .. })));
        assert_eq!(m.timer, REQUEST_DEBOUNCE);
        // Stinger starts: phase 1, then the time remaining arms the fade (T, T/4).
        m.reply(StreamReply { player: PlayerId::Transition, cb: StreamCallback::Transition, value: 9 });
        assert_eq!((m.transition.state, m.phase), (4, 1));
        m.reply(StreamReply { player: PlayerId::Transition, cb: StreamCallback::Remaining, value: 288_000 });
        assert_eq!((m.phase, m.fade_total, m.fade_quarter), (2, 288_000, 72_000));
        // Half way through the first quarter: half volume.
        m.transition.remaining = 288_000 - 36_000;
        m.main.handle = 7;
        let mut c = Vec::new();
        m.update(&mut c);
        assert!(c.contains(&StreamCommand::SetVolume { player: PlayerId::Main, handle: 7, vol: 0x200 }));
        // Idle players: a request just sets the track.
        let mut idle = Music::new(present());
        idle.request(2, 6);
        assert_eq!((idle.main.track, idle.pending_track), (2, -1));
    }

    /// A preseeked stream has no envelope yet, but its voice is reserved: tones must not take it, and
    /// `Continue` restores the stream's ownership and volume.
    #[test]
    fn preseeked_stream_keeps_its_voice() {
        let mut body = vec![0u8; 32];
        body[1] = 0;
        body[16 + 1] = 1;
        let track = Arc::new(MusicTrack { name: "t".into(), rate: 48000, body: Arc::from(body) });
        let tracks: [Option<Arc<MusicTrack>>; 15] = std::array::from_fn(|k| (k == 0).then(|| track.clone()));
        let mut vm = VoiceManager::default();
        let mut spu = Spu::new(Arc::from(Vec::new()));
        let mut st = Streams::default();
        let play = StreamCommand::Play { player: PlayerId::Main, track: 0, vol: 0x400, group: 1, queue: 0, flags: 0x21, cb: StreamCallback::Preseek };
        let r = st.execute(&play, &tracks, &mut vm, &mut spu).unwrap();
        assert_eq!(r.value, 1);
        let voice = st.streams[&r.value].voice;
        assert!(!spu.voices[voice].active());
        assert_eq!(vm.voices[voice].owner, VoiceUse::Stream { handle: 1 });
        let (vol, basevol) = (spu.voices[voice].vol, vm.voices[voice].basevol);
        // Fill every other voice with a tone; an allocator that treats the inactive preseek as free takes it.
        let tone: Arc<[u8]> = Arc::from(&[0u8; 16][..]);
        for i in 0..SPU_VOICES {
            if i == voice { continue; }
            spu.voices[i].key_on(VoiceData::Ram(Arc::clone(&tone)), 0, 0x1000, 0x00ff, 0x1fc0);
            vm.voices[i] = VoiceSlot { owner: VoiceUse::Tone { handle: i as u32 + 100 }, priority: 0, group: 0, basevol: (0, 0), start_tick: 0, generation: spu.voices[i].generation };
        }
        let taken = vm.allocate(&mut spu, 0, 0, 0);
        assert_ne!(taken, Some(voice));
        assert_eq!(vm.voices[voice].owner, VoiceUse::Stream { handle: 1 });
        st.execute(&StreamCommand::Continue { handle: r.value }, &tracks, &mut vm, &mut spu);
        assert!(spu.voices[voice].active());
        assert_eq!(spu.voices[voice].vol, vol);
        assert_eq!(vm.voices[voice], VoiceSlot { owner: VoiceUse::Stream { handle: 1 }, priority: 127, group: 1, basevol, start_tick: 0, generation: spu.voices[voice].generation });
        // Once the stream's natural end is observed, the voice can be used again.
        for _ in 0..200 { spu.mix(); }
        assert!(!spu.voices[voice].active());
        let r = st.execute(&StreamCommand::Query { player: PlayerId::Main, handle: r.value, cb: StreamCallback::Alive }, &tracks, &mut vm, &mut spu).unwrap();
        assert_eq!(r.value, 0);
        assert_eq!(vm.voices[voice].owner, VoiceUse::Free);
    }

    /// Queueing the body behind a stream that already ended fails (0) instead of starting a second stream the
    /// player never tracks; the player then goes idle and restarts the track (one music stream throughout).
    #[test]
    fn body_behind_an_ended_stream_fails() {
        // A 2-frame track (ends after 56 samples).
        let mut body = vec![0u8; 32];
        body[1] = 0;
        body[16 + 1] = 1;
        let track = Arc::new(MusicTrack { name: "t".into(), rate: 48000, body: Arc::from(body) });
        let tracks: [Option<Arc<MusicTrack>>; 15] = std::array::from_fn(|k| (k < 2).then(|| track.clone()));
        let mut vm = VoiceManager::default();
        let mut spu = Spu::new(Arc::from(Vec::new()));
        let mut st = Streams::default();
        let play = |queue, flags, cb| StreamCommand::Play { player: PlayerId::Main, track: 0, vol: 0x400, group: 1, queue, flags, cb };
        let r = st.execute(&play(0, 0x20, StreamCallback::Start), &tracks, &mut vm, &mut spu).unwrap();
        assert_eq!(r.value, 1);
        let music = |spu: &Spu| spu.voices.iter().filter(|v| v.active()).count();
        assert_eq!(music(&spu), 1);
        for _ in 0..200 { spu.mix(); }
        assert_eq!(music(&spu), 0);
        let r = st.execute(&play(1, 0x24, StreamCallback::Body), &tracks, &mut vm, &mut spu).unwrap();
        assert_eq!(r.value, 0);
        assert_eq!(music(&spu), 0);
        // The EE side: body failed → idle → the track restarts (one new stream).
        let mut m = Music::new(std::array::from_fn(|k| k < 2));
        m.main = Player { handle: 1, track: 0, vol: 0x400, flags: 1, state: 9, ..Default::default() };
        m.reply(r);
        let mut cmds = Vec::new();
        m.update(&mut cmds);
        for c in &cmds { if let Some(r) = st.execute(c, &tracks, &mut vm, &mut spu) { m.reply(r); } }
        let mut cmds = Vec::new();
        m.update(&mut cmds);
        assert_eq!(cmds.iter().filter(|c| matches!(c, StreamCommand::Play { queue: 0, .. })).count(), 1, "{cmds:?}");
    }
}
