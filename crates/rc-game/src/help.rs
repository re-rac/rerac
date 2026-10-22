//! The help / hint message system (HelpDesk boxes): `Help_Request` 0x225818, `Help_Update` 0x225bd0 (boot
//! 0x1fde90), `Help_ComputeSize` 0x225a98, the kill `FUN_002258b0`, the suspend / resume `FUN_00225a28` /
//! `FUN_00225a88` (and its close `FUN_00225790`), the help log `FUN_00226a70` with its id table 0x1798d0
//! (`fun_001fecc8`), the help records 0x141968 (chunk 16) and their bump. The draw is the HUD's
//! (`crate::hud`, `DrawHud2D_B` 0x2266c0) from [`Help::bx`]. Spec: docs/plan/hud_text.md §3.4 and "Help system";
//! addresses are level01.elf.
//!
//! **A system, one copy.** Every level runs this same code (boot-hash-matched); what differs per level are the
//! *callers* (each one its class's or the hero's own code: docs/plan/menus.md §10) and the level text. The state
//! block 0x179890..0x1798cc is overlay .bss, so it starts zeroed on every level load: help is **off until 120 ticks
//! after the first direction input** of the level (0x1798c0 / 0x1798c4).
//!
//! **Callers** test the box themselves (`box idle && request == −1`: [`Help::idle`]) and call [`Help::request`];
//! the request is refused unless the box is idle, nothing is pending, the dialogue player is free (0x151720 = 0,
//! 0x1516ec = −1) and the record's count is not 0xffff; an accepted request is logged ([`Help::log_append`]).
//! Ported callers: the Novalis director 1341 (`units::help_director`), the hero's `0x228498` ([`hero_hints`]),
//! the Pyrocitor `0x2cde98` and the Tesla Claw `0x2ce448` (through [`HeroHelp`]).
//!
//! **The voice** (`help_audio`, message +8 `audio` + 30000): the box drives the dialogue player 0x151720 (music_Update
//! 0x27a688's request 0x1516ec, `PlayDialogue` 0x279cd8 → `fun_002156d8`: the stream `help_audio[lang·150 + n]`,
//! started at volume 0 when the HelpDesk voice option 0x15ee1c is off). [`Voice`] is that player as the help code
//! sees it; the engine loads / plays / stops the VAG on its [`VoiceCmd`]s and reports the loaded length.
//!
//! **Native.** Plain integers; the VAG's length gives the playing time [L: the IOP stream latency is not modelled,
//! the stream is ready on the tick after its request as the vendor's salesman lines].

use crate::game_state::{GameState, HelpRec};
use crate::hud::text as wtext;
use rc_formats::font::GlyphTable;
use rc_formats::strings::Message;
use std::sync::Arc;

/// Help records (chunk 16, 0x141968 + 8·rec).
pub const HELP_RECORDS: usize = 148;
/// Move records (chunk 17, 0x141848 + 8·rec).
pub const MOVE_RECORDS: usize = 36;
/// Gadget-help records (chunk 18, 0x141720 + 8·item).
pub const GADGET_RECORDS: usize = 37;
/// The help log (chunk 1010, 0x141e08) and the id table 0x1798d0 (`fun_001fecc8` scans 0x96 entries).
pub const LOG_LEN: usize = 150;
/// The level-01 address of the log's id table `{s16 help message id, s16 log title id}[150]` (`fun_001fecc8`).
pub const LOG_IDS: u32 = 0x1798d0;
/// Help voice stream ids: `PlayDialogue` ids 30000..39999 (`fun_002156d8`).
pub const VOICE_BASE: i32 = 30000;
/// `help_audio` entries per language (TOC field 0x1ab8 = 0x139638, `lang·600 + n·4`).
pub const VOICE_PER_LANGUAGE: i32 = 150;
/// Qwark's boss lines (Gemlik's ship 388): `PlayDialogue` ids 50000..59999 (`fun_00215518`): the stream
/// `qwark_boss_audio[(id − 50000)·6 + lang]` (the TOC's 0x138a80, 0x18 bytes an id).
pub const QWARK_BASE: i32 = 50000;
/// `qwark_boss_audio` entries per line (one per language).
pub const QWARK_LANGUAGES: i32 = 6;
/// The level's own lines (Blarg's queen 1051 and its escape 1108, the only level with any): `PlayDialogue` ids
/// 20000..29999 (`fun_002157d0`): the stream `bindata[id − 20000]` of the loaded level's header (0x13a508, 8 bytes a
/// slot), no language column (the classes pick the language's line).
pub const LEVEL_BASE: i32 = 20000;

/// The extracted file `PlayDialogue` 0x279cd8 streams for dialogue id `id` in language `lang` on level `level`: ≥ 60000
/// `post_credits_audio[id − 60000]` (`fun_00215440`), 50000.. `qwark_boss_audio` ([`QWARK_BASE`]), 30000..
/// `help_audio[lang·150 + id − 30000]` (`fun_002156d8`), 20000.. the loaded level's `bindata[id − 20000]`
/// ([`LEVEL_BASE`]); None for the ranges with their own players (scenes, the vendor, space).
pub fn dialogue_stream(id: i32, lang: u32, level: u32) -> Option<String> {
    match id {
        60000.. => Some(format!("global/post_credits_audio/{:03}.bin", id - 60000)),
        QWARK_BASE..=59999 => Some(format!("global/qwark_boss_audio/{:03}.bin", (id - QWARK_BASE) * QWARK_LANGUAGES + lang as i32)),
        VOICE_BASE..=39999 => Some(format!("global/help_audio/{:03}.bin", lang as i32 * VOICE_PER_LANGUAGE + id - VOICE_BASE)),
        LEVEL_BASE..=29999 => Some(format!("levels/{level:02}/bindata/{:03}.bin", id - LEVEL_BASE)),
        _ => None,
    }
}

/// `force_help_message(5, 0)`: the owner id the box holds the context prompt with.
pub const PROMPT_OWNER: i32 = 5;
/// `PlayLevelSoundAtMoby(0, 1, 0)`: the opening sound (`crate::audio::class_sounds::level_sound::HELP_OPEN`).
pub const OPEN_SOUND: (i32, u32) = (crate::audio::class_sounds::level_sound::HELP_OPEN, 1);
/// Frame buffer height 0x13e504 (NTSC).
const SCREEN_H: i32 = crate::hud::SCREEN_H;

/// `ScaleTicks` (0x220e30; NTSC: identity + 0.5 truncated).
fn ticks(n: i32) -> i32 { crate::hud::scale_ticks(n) }

/// The records the help code reads and bumps: a mirror of chunks 16 / 17 ([`Help::sync_in`] / [`Help::sync_out`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Records {
    /// 0x141968.
    pub help: [HelpRec; HELP_RECORDS],
    /// 0x141848.
    pub moves: [HelpRec; MOVE_RECORDS],
    /// 0x141720 (read only here: the directors' "item not used for a while" tests).
    pub gadget: [HelpRec; GADGET_RECORDS],
}

impl Default for Records {
    fn default() -> Self { Records { help: [HelpRec::default(); HELP_RECORDS], moves: [HelpRec::default(); MOVE_RECORDS], gadget: [HelpRec::default(); GADGET_RECORDS] } }
}

impl Records {
    pub fn of(gs: &GameState) -> Records { Records { help: gs.global.help, moves: gs.global.move_help, gadget: gs.global.gadget_help } }
    /// `help[rec].count` as the code reads it (`lh` / `lhu`); out of range: 0xffff (refused).
    pub fn help_count(&self, rec: i32) -> u16 { usize::try_from(rec).ok().and_then(|r| self.help.get(r)).map_or(0xffff, |r| r.count) }
}

/// The record bump inlined everywhere (`count++` unless 0xffff, `time = max(time, ScaleTicks(play) / 600)`, `mask |= 1
/// << level | 0x80000000`): `crate::hero::melee::bump_record`.
pub fn bump(r: &mut HelpRec, level: i32, play_time: i32) { crate::hero::melee::bump_record(r, level, ticks(play_time)); }

/// The same without the count (the box closing in state 8 with no message, the Tesla Claw's short shots).
pub fn touch(r: &mut HelpRec, level: i32, play_time: i32) {
    let t = ticks(play_time) / 600;
    if (r.time as i32) < t { r.time = t as u16; }
    r.mask |= (1u32 << (level as u32 & 31)) | 0x8000_0000;
}

/// The box as the draw 0x2266c0 reads it (0x179890..0x1798ac, 0x1798c0 and the text option 0x15ee1d).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelpBox {
    /// 0x179890: 0 idle, 1 open, 2 logo, 3 grow, 4 text fade-in, 5 hold, 6 text fade-out, 7 shrink, 8 suspended.
    pub state: u8,
    /// 0x179894: ticks in the state (incremented before the state's rule).
    pub t: i32,
    /// 0x1798b0: index of the shown message in the level text (`Help_FindIndex`), None = −1.
    pub index: Option<usize>,
    /// 0x179898 / 0x17989c: full half-size; 0x1798a0 / 0x1798a4: centre.
    pub half_w: i32,
    pub half_h: i32,
    pub cx: i32,
    pub cy: i32,
    /// 0x1798a8 / 0x1798ac: the current half-size, written by the draw (state 7 shrinks from it).
    pub cur_w: i32,
    pub cur_h: i32,
    /// 0x1798c0: help enabled.
    pub enabled: bool,
    /// 0x15ee1d / 0x15ee1c: the HelpDesk text / voice options (the draw needs one of them, and the text for any
    /// frame).
    pub text_on: bool,
    pub voice_on: bool,
}

impl HelpBox {
    /// Everything but the draw-owned current size (the HUD keeps its own).
    pub fn copy_logic_from(&mut self, o: &HelpBox) {
        let (w, h) = (self.cur_w, self.cur_h);
        *self = o.clone();
        self.cur_w = w;
        self.cur_h = h;
    }
}

/// The dialogue player 0x151720 as the help code reads and drives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice {
    /// 0x1516ec: the requested stream id (−1 none).
    pub request: i32,
    /// 0x151720 ≠ 0: a stream is loading or playing.
    pub busy: bool,
    /// 0x151724 (s16): the id of the stream last started.
    pub track: i32,
    /// 0x15172a: 0 never used, 1 requested, 3 buffered (paused, ready), 4 playing, 5 stop, 7 ended / stopped.
    pub state: u16,
    /// Port: ticks of the line left while playing (its VAG length).
    pub left: i32,
    prev_track: i32,
}

impl Default for Voice {
    fn default() -> Self { Voice { request: -1, busy: false, track: 0, state: 0, left: 0, prev_track: 0 } }
}

impl Voice {
    /// `iVar == sRam00151724 − 30000`: the player's stream is the voice of help audio `audio`.
    pub fn is(&self, audio: i32) -> bool { audio != -1 && audio == self.track - VOICE_BASE }
    /// `1 < (u16)(0x15172a − 6)`: not already stopping / ended → state 5.
    fn stop(&mut self) {
        if self.state.wrapping_sub(6) > 1 { self.state = 5; }
    }
}

/// What the engine does for the dialogue player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceCmd {
    /// Load stream `id` (≥ 30000: `help_audio[lang·150 + id − 30000]`) and answer [`Help::voice_loaded`].
    Load { id: i32 },
    /// `continue_audio_stream_if_ready` 0x279e78: start the loaded line (volume 0 when `audible` is false).
    Play { audible: bool },
    /// State 5: stop it.
    Stop,
}

/// Per-tick values `Help_Update` reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HelpInputs {
    /// 0x15f5c4 when `Help_Update` runs.
    pub mode: i32,
    /// 0x13cae0 held (stick directions included) and 0x13cae4 pressed.
    pub held: u32,
    pub pressed: u32,
    /// 0x15eea4, 0x15ed84.
    pub play_time: i32,
    pub level: i32,
    /// 0x15ee1d / 0x15ee1c.
    pub text_on: bool,
    pub voice_on: bool,
}

/// The level text the box sizes and voices from.
#[derive(Clone, Debug, Default)]
pub struct HelpText {
    pub messages: Arc<Vec<Message>>,
    /// The small font's glyphs (FX texture 2's table).
    pub small: Option<GlyphTable>,
    /// 0x15ed88: the language (the voice line's `help_audio` bank).
    pub lang: u32,
}

/// A voice line's playing time in ticks from its VAG header (`VAGp`, data size at +0x0c and sample rate at +0x10, big
/// endian; 28 samples per 16-byte block), rounded up. None when it is not a VAG.
pub fn vag_ticks(vag: &[u8]) -> Option<i32> {
    if vag.len() < 0x30 || &vag[..4] != b"VAGp" { return None; }
    let be = |o: usize| u32::from_be_bytes([vag[o], vag[o + 1], vag[o + 2], vag[o + 3]]) as u64;
    let (size, rate) = (be(0x0c), be(0x10));
    if rate == 0 { return None; }
    let samples = size / 16 * 28;
    Some(((samples * 60).div_ceil(rate)) as i32)
}

/// What the tick asks of others.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelpOut {
    /// `PlayLevelSoundAtMoby(index, flags, 0)` calls.
    pub sounds: Vec<(i32, u32)>,
    pub voice: Vec<VoiceCmd>,
    /// `force_help_message(5, 0)` was called this tick.
    pub force_prompt: bool,
}

/// The help system (0x179890..0x1798cc, the records, the log, the dialogue player's view).
#[derive(Clone, Debug)]
pub struct Help {
    pub bx: HelpBox,
    /// 0x1798b4: the requested message id (−1 none); 0x1798b8: its record.
    pub request: i32,
    pub rec: i32,
    /// 0x1798c4: ticks since the first direction input (0: none yet).
    pub first_input: i32,
    /// 0x1798c8: suspended (`FUN_00225a28`); 0x1798ca: the box was reopened after it; 0x1798cc: the reopen timer.
    pub hold: bool,
    pub reopened: u16,
    pub reopen_timer: i32,
    pub records: Records,
    /// 0x141e08 / 0x15ee30: the help log (log-table indices) and its length.
    pub log: [u8; LOG_LEN],
    pub log_pos: i32,
    /// 0x1798d0: `{help message id, log title id}` per log-table index (read from the overlay).
    pub log_ids: Arc<Vec<(i16, i16)>>,
    pub voice: Voice,
    pub text: HelpText,
    /// The level (0x15ed84) and play time (0x15eea4) of the tick, for the bumps.
    pub level: i32,
    pub play_time: i32,
    pub out: HelpOut,
    /// A camera option reversed (the low byte of 0x15eddc or 0x15ede0 is 0): the Aridia director 1324's camera hint.
    pub cam_reversed: bool,
    /// 0x15eddc (camera up / down: 1 normal, 0 reversed) as the classes read it (Hoven's turret 1267 negates the
    /// stick's y when it is 0).
    pub cam_pitch_word: u32,
}

impl Default for Help {
    fn default() -> Self {
        Help {
            bx: HelpBox { text_on: true, voice_on: true, ..HelpBox::default() },
            request: -1,
            rec: 0,
            first_input: 0,
            hold: false,
            reopened: 0,
            reopen_timer: 0,
            records: Records::default(),
            log: [0; LOG_LEN],
            log_pos: 1,
            log_ids: Arc::new(Vec::new()),
            voice: Voice::default(),
            text: HelpText::default(),
            level: 0,
            play_time: 0,
            out: HelpOut::default(),
            cam_reversed: false,
            cam_pitch_word: 1,
        }
    }
}

/// The id table 0x1798d0 of the level's overlay (`label` → this level's address through `at`).
pub fn read_log_ids(ov: &crate::menus::Overlay) -> Vec<(i16, i16)> {
    let a = ov.at(LOG_IDS);
    (0..LOG_LEN as u32).map_while(|i| Some((ov.i16(a + 4 * i)?, ov.i16(a + 4 * i + 2)?))).collect()
}

/// The id table from the level's overlay lump `target`, found through the code that forms it in the level-01 overlay
/// `reference` (`fun_001fecc8`'s `lui`/`%lo`: `rc_formats::level_overlay::Relocation`); level 01 itself or no
/// reference: read at 0x1798d0. Empty when it cannot be read.
pub fn load_log_ids(target: &[u8], reference: Option<&[u8]>) -> Vec<(i16, i16)> {
    use rc_formats::level_overlay::{LevelOverlay, Relocation};
    let Ok(ov) = crate::menus::Overlay::parse(target) else { return Vec::new() };
    let addr = match (reference, LevelOverlay::parse(target)) {
        (Some(r), Ok(t)) => match LevelOverlay::parse(r) {
            Ok(r) => Relocation::new(&r, &t).data(LOG_IDS).unwrap_or(LOG_IDS),
            Err(_) => LOG_IDS,
        },
        _ => LOG_IDS,
    };
    (0..LOG_LEN as u32).map_while(|i| Some((ov.i16(addr + 4 * i)?, ov.i16(addr + 4 * i + 2)?))).collect()
}

impl Help {
    /// The records, log, play time, level and options from the saved game (before the tick).
    pub fn sync_in(&mut self, gs: &GameState) {
        self.records = Records::of(gs);
        self.log = gs.global.help_log;
        self.log_pos = gs.global.help_log_pos;
        self.level = gs.global.level;
        self.play_time = gs.global.play_time;
        self.bx.text_on = gs.global.helpdesk_text != 0;
        self.bx.voice_on = gs.global.helpdesk_voice != 0;
        self.cam_reversed = gs.global.cam_pitch_normal & 0xff == 0 || gs.global.cam_yaw_normal & 0xff == 0;
        self.cam_pitch_word = gs.global.cam_pitch_normal as u32;
    }

    /// The records and the log back into the saved game (after the tick).
    pub fn sync_out(&self, gs: &mut GameState) {
        let g = &mut gs.global;
        if g.help != self.records.help { g.help = self.records.help; }
        if g.move_help != self.records.moves { g.move_help = self.records.moves; }
        if g.gadget_help != self.records.gadget { g.gadget_help = self.records.gadget; }
        if g.help_log != self.log { g.help_log = self.log; }
        if g.help_log_pos != self.log_pos { g.help_log_pos = self.log_pos; }
    }

    /// What the callers test before a request: the box idle (0x179890 = 0) and nothing pending (0x1798b4 = −1).
    pub fn idle(&self) -> bool { self.bx.state == 0 && self.request == -1 }

    /// `Help_Request(msg, rec)` 0x225818.
    pub fn request(&mut self, msg: i32, rec: i32) -> bool {
        if self.bx.state != 0 || self.request != -1 || self.voice.busy || self.voice.request != -1 { return false; }
        if self.records.help_count(rec) == 0xffff { return false; }
        self.request = msg;
        self.rec = rec;
        self.log_append(msg);
        true
    }

    /// **Port-only**: [`Help::request`] for a message the port adds to the level text (rc-engine's Gadgetron
    /// Movement System note), with no help record: none is checked or bumped (rec −1), and it is not in the help log's
    /// id table, so it is not logged. The same conditions as the game's request otherwise.
    pub fn request_port(&mut self, msg: i32) -> bool {
        if self.bx.state != 0 || self.request != -1 || self.voice.busy || self.voice.request != -1 { return false; }
        self.request = msg;
        self.rec = -1;
        true
    }

    /// `fun_001fecc8(msg, 0, 0)`: the log-table index of help message `msg`.
    pub fn log_index(&self, msg: i32) -> Option<usize> { self.log_ids.iter().position(|&(m, _)| m as i32 == msg as i16 as i32) }

    /// `FUN_00226a70(msg)`: appends the message's log index, or moves it to the end when it is already logged
    /// (entry 0 included: the log starts as {0} with length 1, the HelpDesk welcome).
    pub fn log_append(&mut self, msg: i32) {
        let Some(idx) = self.log_index(msg) else { return };
        let idx = idx as u8;
        let pos = self.log_pos;
        let mut i = 0i32;
        let mut at = pos;
        let found_first = self.log[0] == idx;
        if !found_first {
            if pos < 1 {
                self.put(at, idx);
                return;
            }
            i = 1;
            while (i as usize) < LOG_LEN && self.log[i as usize] != idx && i < pos { i += 1; }
        }
        if i < pos {
            while i < pos - 1 {
                self.log[i as usize] = self.log[i as usize + 1];
                i += 1;
            }
            at = pos - 1;
            if let Some(b) = self.log.get_mut(at as usize) { *b = 0; }
        }
        self.put(at, idx);
    }

    fn put(&mut self, at: i32, idx: u8) {
        if let Some(b) = usize::try_from(at).ok().and_then(|a| self.log.get_mut(a)) { *b = idx; }
        self.log_pos = at + 1;
    }

    /// The logged messages, oldest first (log indices → `(help message id, title id)`).
    pub fn logged(&self) -> Vec<(i16, i16)> {
        self.log.iter().take(self.log_pos.clamp(0, LOG_LEN as i32) as usize).filter_map(|&i| self.log_ids.get(i as usize).copied()).collect()
    }

    /// `FUN_002258b0`: the box closed at once (a scene, a movie, the vendor, the ship, a gold-weapon upgrade start);
    /// its voice is stopped; no record bump.
    pub fn kill(&mut self) {
        if self.bx.state == 0 { return; }
        if self.voice.is(self.audio()) { self.voice.stop(); }
        self.bx.state = 0;
        self.bx.t = 0;
        self.bx.index = None;
        self.request = -1;
    }

    /// `continue_audio_stream_if_ready` 0x279e78 from a class (Qwark's ship 388's taunts): a line buffered in the
    /// player (busy, 0x15172a = 3) starts (state 4). Returns whether it did.
    pub fn continue_stream(&mut self) -> bool {
        if !self.voice.busy || self.voice.state != 3 { return false; }
        self.voice.state = 4;
        self.out.voice.push(VoiceCmd::Play { audible: true });
        true
    }

    /// `FUN_00225a28`: suspend (the Visibomb's flight). The box closes (`FUN_00225790`) and reopens its message
    /// `ScaleTicks(60)` ticks after [`Help::resume`].
    pub fn suspend(&mut self) {
        if self.hold { return; }
        self.hold = true;
        self.reopen_timer = ticks(0x3c);
        match self.bx.state {
            0 => self.request = -1,
            1..=3 => {
                self.bx.state = 7;
                self.bx.t = 0;
            }
            4 => {
                self.bx.state = 6;
                self.bx.t = 4 - self.bx.t;
            }
            5 => {
                self.bx.state = 6;
                self.bx.t = 0;
            }
            _ => {}
        }
        if self.bx.state == 0 { self.bx.state = 8; }
    }

    /// `FUN_00225a88`.
    pub fn resume(&mut self) { self.hold = false; }

    /// The shown message's help-audio index (message +8; −1 none).
    fn audio(&self) -> i32 { self.bx.index.and_then(|i| self.text.messages.get(i)).map_or(-1, |m| m.help_audio) }

    fn bump_rec(&mut self) {
        let (l, p) = (self.level, self.play_time);
        if let Some(r) = usize::try_from(self.rec).ok().and_then(|r| self.records.help.get_mut(r)) { bump(r, l, p); }
    }

    fn touch_rec(&mut self) {
        let (l, p) = (self.level, self.play_time);
        if let Some(r) = usize::try_from(self.rec).ok().and_then(|r| self.records.help.get_mut(r)) { touch(r, l, p); }
    }

    /// The engine's answer to [`VoiceCmd::Load`]: the line's length in ticks, None when the stream does not
    /// exist (`fun_002156d8` starts nothing: the player stays free and 0x151724 keeps its old id).
    pub fn voice_loaded(&mut self, ticks_len: Option<i32>) {
        match ticks_len {
            Some(n) => {
                self.voice.state = 3;
                self.voice.left = n.max(1);
            }
            None => {
                self.voice.busy = false;
                self.voice.state = 0;
                self.voice.track = self.voice.prev_track;
            }
        }
    }

    /// `music_Update` 0x27a688's dialogue part, every game frame (before `Help_Update` in a gameplay tick; also in the
    /// scene / vendor frames, where `Help_Update` does not run): the request 0x1516ec starts the stream when the player
    /// is free, else stops the running one; a stop (state 5) is carried out; the stream runs to its end.
    pub fn voice_frame(&mut self) {
        let v = &mut self.voice;
        // A line that ended (or was stopped) stays in state 7 with the player free until the next request (the
        // stop tests `(u16)(state − 6) > 1` leave it alone).
        match v.state {
            4 => {
                v.left -= 1;
                if v.left <= 0 {
                    v.state = 7;
                    v.busy = false;
                }
            }
            5 => {
                self.out.voice.push(VoiceCmd::Stop);
                v.state = 7;
                v.busy = false;
            }
            _ => {}
        }
        if v.request >= 0 {
            if !v.busy {
                v.prev_track = v.track;
                v.busy = true;
                v.state = 1;
                v.track = v.request;
                self.out.voice.push(VoiceCmd::Load { id: v.request });
                v.request = -1;
            } else {
                v.stop();
            }
        }
    }

    /// `Help_ComputeSize` 0x225a98: state 1, the opening sound (text or voice option on), the text measured in the
    /// small font (window y 240..480, x 44..468, anchor 256, y 360, line 16, flags 7) and the box placed.
    fn compute_size(&mut self) {
        self.bx.state = 1;
        self.bx.t = 0;
        if self.bx.text_on || self.bx.voice_on { self.out.sounds.push(OPEN_SOUND); }
        let text = self.bx.index.and_then(|i| self.text.messages.get(i)).map_or(&[][..], |m| &m.text[..]);
        let mut win = window(0x168, 7);
        if let Some(g) = &self.text.small { wtext::layout(&mut win, text, -1, g, true); }
        let hh = (win.height >> 1) as i32;
        let b = &mut self.bx;
        b.cy = SCREEN_H - 0x3c;
        b.half_h = hh + 5;
        b.half_w = (win.max_width >> 1) as i32 + 10;
        b.cx = 0x100;
        b.cur_w = 8;
        b.cur_h = 8;
        if SCREEN_H - 12 < b.cy + b.half_h { b.cy = SCREEN_H - (hh + 0x11); }
    }

    /// `Help_Update` 0x225bd0. The caller runs the dialogue player's frame first ([`Help::voice_frame`]) and
    /// `force_help_message(5, 0)` when [`HelpOut::force_prompt`] is set ([`tick`]).
    pub fn update(&mut self, inp: &HelpInputs) {
        self.level = inp.level;
        self.play_time = inp.play_time;
        self.bx.text_on = inp.text_on;
        self.bx.voice_on = inp.voice_on;
        if !self.bx.enabled {
            if self.first_input == 0 && inp.held & 0xf000 != 0 { self.first_input = 1; }
            if self.first_input != 0 {
                self.first_input += 1;
                if ticks(0x78) <= self.first_input { self.bx.enabled = true; }
            }
        }
        if inp.mode != 0 || !self.bx.enabled {
            self.bx.state = 0;
            self.bx.t = 0;
            self.request = -1;
            return;
        }
        let tri = inp.pressed & crate::pad::button::TRIANGLE != 0;
        self.bx.t += 1;
        let audio = self.audio();
        match self.bx.state {
            0 => {
                if self.request >= 0 {
                    let idx = rc_formats::strings::find_index(&self.text.messages, self.request);
                    self.bx.index = idx;
                    self.request = -1;
                    if idx.is_some() { self.compute_size(); }
                }
            }
            1 => {
                self.out.force_prompt = true;
                if audio >= 0 && !self.voice.busy && self.voice.request == -1 { self.voice.request = audio + VOICE_BASE; }
                if tri {
                    self.bump_rec();
                    self.bx.t = 8 - self.bx.t;
                    self.bx.state = 7;
                } else if self.bx.t >= 6 {
                    self.bx.state = 2;
                    self.bx.t = 0;
                }
            }
            2 => {
                self.out.force_prompt = true;
                if tri {
                    self.bump_rec();
                    self.set(7);
                } else if self.bx.t >= ticks(0x18) {
                    // The line still loading waits (unless the player reports it buffered).
                    let waiting = self.voice.state != 3 && audio != -1 && self.voice.is(audio);
                    if !waiting { self.set(3); }
                }
            }
            3 => {
                self.out.force_prompt = true;
                if tri {
                    self.bump_rec();
                    self.set(7);
                } else if self.bx.t >= 8 {
                    self.set(4);
                }
            }
            4 => {
                self.out.force_prompt = true;
                if tri {
                    self.bump_rec();
                    self.bx.state = 6;
                    self.bx.t = 4 - self.bx.t;
                } else if self.bx.t >= 4 {
                    if self.voice.is(audio) && self.voice.state == 3 {
                        // continue_audio_stream_if_ready 0x279e78.
                        self.voice.state = 4;
                        self.out.voice.push(VoiceCmd::Play { audible: self.bx.voice_on });
                    }
                    self.set(5);
                }
            }
            5 => {
                self.out.force_prompt = true;
                let playing = self.voice.is(audio) && (self.voice.busy || self.voice.request != -1);
                if (self.bx.t >= ticks(0x1a4) && !playing) || tri {
                    self.bump_rec();
                    self.set(6);
                }
            }
            6 => {
                if !(self.bx.text_on && self.bx.t < 4 && !tri) { self.set(7); }
            }
            7 => {
                self.out.force_prompt = true;
                if self.voice.is(audio) { self.voice.stop(); }
                if self.bx.t >= 8 {
                    if self.hold {
                        self.bx.state = 8;
                    } else {
                        self.bx.state = 0;
                        self.bx.index = None;
                    }
                    self.bx.t = 0;
                }
            }
            8 => {
                if self.hold { return; }
                if self.bx.index.is_some() {
                    if self.reopened == 0 {
                        if crate::moby_update::creature::dec_timer_i32(&mut self.reopen_timer) == 0 { return; }
                        self.compute_size();
                        self.reopened += 1;
                        return;
                    }
                    self.bump_rec();
                } else {
                    self.touch_rec();
                }
                self.bx.index = None;
                self.bx.state = 0;
                self.reopened = 0;
            }
            _ => {}
        }
    }

    fn set(&mut self, state: u8) {
        self.bx.state = state;
        self.bx.t = 0;
    }
}

/// The help box text window (`FontSetWindow(0xf0, 0x1e0, 0x2c, 0x1d4, 0x100, y, 0x10)`).
pub fn window(y: i32, flags: u16) -> wtext::Window { wtext::Window::new(0xf0, 0x1e0, 0x2c, 0x1d4, 0x100, y as i16, 0x10, flags) }

/// `Help_Update` with its `force_help_message(5, 0)` on the context prompt (the call inside `InLevelFrameUpdate`
/// 0x2aba68, after `HudUpdate`).
pub fn tick(svc: &mut crate::moby_update::Services, inp: &HelpInputs) {
    svc.help.out.force_prompt = false;
    svc.help.voice_frame();
    svc.help.update(inp);
    if svc.help.out.force_prompt { svc.interact.force_prompt(PROMPT_OWNER, 0); }
}

// ---------------------------------------------------------------------------------------------------
// The hero's help callers

/// The hero-side help state: the hero block bytes the Pyrocitor / Tesla Claw count short uses with (0x141404 /
/// 0x141405), the first-person look timer and count (gp 0x15f688 / 0x15f68c, read by the Novalis director), the
/// records as the tick started (mirrored in by the engine), and the requests / record writes of the hero update,
/// applied after it in order ([`apply_hero`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeroHelp {
    pub records: Records,
    /// 0x141404: Pyrocitor taps shorter than `ScaleTicks(30)`.
    pub pyro_taps: u8,
    /// 0x141405: Tesla Claw shots shorter than `ScaleTicks(30)` in a row.
    pub tesla_taps: u8,
    /// 0x15f688: the first-person look timer (`ScaleTicks(60)` on entering state 1, −1 once a look was counted).
    pub look_timer: i32,
    /// 0x15f68c: first-person looks counted.
    pub looks: i32,
    pub out: Vec<HeroHelpOut>,
}

/// One help call or record write of the hero update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroHelpOut {
    Request { msg: i32, rec: i32 },
    /// A move record bumped (count, time, mask).
    BumpMove(usize),
    /// A move record's time and mask only.
    TouchMove(usize),
}

/// The hero's help calls and record writes of the tick into `help` (their order is the hero update's).
pub fn apply_hero(help: &mut Help, hero: &mut HeroHelp) {
    let (l, p) = (help.level, help.play_time);
    for o in std::mem::take(&mut hero.out) {
        match o {
            HeroHelpOut::Request { msg, rec } => {
                help.request(msg, rec);
            }
            HeroHelpOut::BumpMove(i) => {
                if let Some(r) = help.records.moves.get_mut(i) { bump(r, l, p); }
            }
            HeroHelpOut::TouchMove(i) => {
                if let Some(r) = help.records.moves.get_mut(i) { touch(r, l, p); }
            }
        }
    }
}

/// `FUN_0022dea8`: in the water (group 0x11 / 0x12) or in state 0x6a / 0x82 / 0x76 / 0x75.
pub fn hero_in_water(state: i32, group: i32) -> bool { (group - 0x11) as u32 <= 1 || matches!(state, 0x6a | 0x82 | 0x76 | 0x75) }

/// The hero update's `FUN_00228498` (0x228870, after `HeroItemsUpdate`; not in the alternate update 0x228000):
/// the Hydro-Pack hint 20014 (help record 0x78) while it is owned (item 4) and Ratchet is in the water, and every
/// 128 ticks the QuickSelect hint 20006 (record 0x50) once more than 8 owned weapons (item slot type 0, not the
/// wrench 8 or the drone device 0x18) exist and neither record 0x50 nor move record 21 was used. `slot_types` =
/// the item definitions' +8 (0x179f48 + 0x4c·id); `counter` = 0x15f5cc.
pub fn hero_hints(state: i32, group: i32, owned: &[u8], slot_types: &[i32], counter: i32, rec: &Records) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    if rec.help[0x78].count == 0 && owned.get(4).is_some_and(|&o| o != 0) && hero_in_water(state, group) { out.push((0x4e2e, 0x78)); }
    if 0 < counter && counter & 0x7f == 0 && rec.help[0x50].count == 0 && rec.moves[21].count == 0 {
        let n = (0..0x25usize)
            .filter(|&i| slot_types.get(i).is_some_and(|&t| t == 0) && i != 8 && i != 0x18 && owned.get(i).is_some_and(|&o| o != 0))
            .count();
        if 8 < n { out.push((0x4e26, 0x50)); }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn help_with(msgs: Vec<Message>) -> Help {
        let mut h = Help { text: HelpText { messages: Arc::new(msgs), small: None, lang: 0 }, ..Help::default() };
        h.bx.enabled = true;
        h.log_ids = Arc::new(vec![(3, 21106), (0, 21103), (1, 21104), (2, 21105), (1000, 21107), (1001, 21108)]);
        h
    }

    fn msg(id: i32, audio: i32) -> Message { Message { id, text: b"Gadgetron \x0cInfobots\x08 give you coordinates.".to_vec(), help_audio: audio } }

    fn step(h: &mut Help, pressed: u32) -> u8 {
        h.out = HelpOut::default();
        h.voice_frame();
        h.update(&HelpInputs { held: 0, pressed, play_time: 6000, level: 1, text_on: true, voice_on: true, ..Default::default() });
        h.bx.state
    }

    #[test]
    fn request_gate_and_log() {
        let mut h = help_with(vec![msg(1000, -1)]);
        assert!(h.request(1000, 4));
        assert_eq!((h.request, h.rec), (1000, 4));
        assert_eq!(&h.log[..2], &[0, 4]);
        assert_eq!(h.log_pos, 2);
        // Pending → refused.
        assert!(!h.request(1001, 5));
        h.request = -1;
        // Dialogue player busy or a line requested → refused.
        h.voice.busy = true;
        assert!(!h.request(1001, 5));
        h.voice.busy = false;
        h.voice.request = 30004;
        assert!(!h.request(1001, 5));
        h.voice.request = -1;
        // A record set to 0xffff → refused ("never again").
        h.records.help[5].count = 0xffff;
        assert!(!h.request(1001, 5));
        // Box up → refused.
        h.bx.state = 3;
        assert!(!h.request(1000, 4));
    }

    #[test]
    fn log_moves_a_repeat_to_the_end() {
        let mut h = help_with(vec![]);
        for m in [1000, 1001, 0] { h.log_append(m); }
        assert_eq!((&h.log[..4], h.log_pos), (&[0, 4, 5, 1][..], 4));
        h.log_append(1000);
        assert_eq!((&h.log[..4], h.log_pos), (&[0, 5, 1, 4][..], 4));
        // Entry 0 (the welcome) moves too.
        h.log_append(3);
        assert_eq!((&h.log[..4], h.log_pos), (&[5, 1, 4, 0][..], 4));
        // An id the table does not have: nothing.
        h.log_append(4242);
        assert_eq!(h.log_pos, 4);
        assert_eq!(h.logged(), vec![(1001, 21108), (0, 21103), (1000, 21107), (3, 21106)]);
    }

    #[test]
    fn states_timings_and_the_record_bump_on_close() {
        let mut h = help_with(vec![msg(1000, -1)]);
        assert!(h.request(1000, 4));
        let mut states = Vec::new();
        for _ in 0..480 { states.push(step(&mut h, 0)); }
        let first = |s: u8| states.iter().position(|&x| x == s).unwrap();
        // Tick 0 opens (state 1 + the opening sound), 2 after 6, 3 after 24, 4 after 8, 5 after 4, 6 after 420, 7 after 4, 0 after 8.
        assert_eq!([first(1), first(2), first(3), first(4), first(5), first(6), first(7)], [0, 6, 30, 38, 42, 462, 466]);
        assert_eq!(states[474], 0);
        // The record: bumped once when the hold ended (count, time = play/600, mask level 1).
        assert_eq!(h.records.help[4], HelpRec { count: 1, time: 10, mask: 0x8000_0002 });
    }

    #[test]
    fn triangle_skips_with_a_bump_and_prompt_is_forced() {
        let mut h = help_with(vec![msg(1000, -1)]);
        h.request(1000, 4);
        step(&mut h, 0);
        step(&mut h, 0);
        assert!(h.out.force_prompt);
        // △ in state 1 at t = 2: shrink from 8 − t, one bump.
        assert_eq!(step(&mut h, crate::pad::button::TRIANGLE), 7);
        assert_eq!(h.bx.t, 8 - 2);
        assert_eq!(h.records.help[4].count, 1);
        for _ in 0..3 { step(&mut h, 0); }
        assert_eq!(h.bx.state, 0);
        assert_eq!(h.records.help[4].count, 1);
    }

    #[test]
    fn first_input_gate_and_mode() {
        let mut h = help_with(vec![msg(1000, -1)]);
        h.bx.enabled = false;
        h.request(1000, 4);
        // Without input: disabled, the request dropped.
        step(&mut h, 0);
        assert_eq!((h.bx.state, h.request), (0, -1));
        // The first direction input starts the count; enabled once it reaches ScaleTicks(120).
        let mut n = 0;
        h.update(&HelpInputs { held: 0x1000, ..Default::default() });
        n += 1;
        while !h.bx.enabled {
            step(&mut h, 0);
            n += 1;
        }
        assert_eq!(n, 119);
        // Another game mode resets the box and drops the request.
        h.request(1000, 4);
        h.update(&HelpInputs { mode: 3, ..Default::default() });
        assert_eq!((h.bx.state, h.request), (0, -1));
    }

    /// `PlayDialogue`'s ranges: 20000.. the loaded level's `bindata` (Blarg's escape 1108: lines 0..29), no language
    /// column; 30000.. `help_audio` by language.
    #[test]
    fn dialogue_streams_by_range() {
        assert_eq!(dialogue_stream(20000, 2, 6).as_deref(), Some("levels/06/bindata/000.bin"));
        assert_eq!(dialogue_stream(20029, 0, 6).as_deref(), Some("levels/06/bindata/029.bin"));
        assert_eq!(dialogue_stream(30004, 1, 6).as_deref(), Some("global/help_audio/154.bin"));
        assert_eq!(dialogue_stream(19999, 0, 6), None);
    }

    #[test]
    fn voice_request_wait_play_and_stop() {
        let mut h = help_with(vec![msg(1000, 4)]);
        h.request(1000, 4);
        step(&mut h, 0); // → 1
        step(&mut h, 0); // state 1: the line is requested (0x1516ec = 30004)
        assert_eq!(h.voice.request, 30004);
        step(&mut h, 0); // the player starts loading it
        assert_eq!(h.out.voice, vec![VoiceCmd::Load { id: 30004 }]);
        assert!(h.voice.busy && h.voice.is(4));
        // Not loaded yet: state 2 waits past its 24 ticks.
        while h.bx.state < 2 { step(&mut h, 0); }
        for _ in 0..40 { step(&mut h, 0); }
        assert_eq!(h.bx.state, 2);
        h.voice_loaded(Some(600));
        while h.bx.state != 5 { step(&mut h, 0); }
        assert_eq!(h.voice.state, 4);
        // The hold lasts while the line plays (600 ticks > 420).
        let mut n = 0;
        while h.bx.state == 5 {
            step(&mut h, 0);
            n += 1;
        }
        assert!((595..=601).contains(&n), "{n}");
        // A killed box stops its line.
        let mut h = help_with(vec![msg(1000, 4)]);
        h.request(1000, 4);
        for _ in 0..3 { step(&mut h, 0); }
        h.voice_loaded(Some(300));
        h.kill();
        assert_eq!((h.bx.state, h.voice.state, h.request), (0, 5, -1));
        step(&mut h, 0);
        assert!(h.out.voice.contains(&VoiceCmd::Stop));
    }

    #[test]
    fn suspend_closes_and_reopens_after_resume() {
        let mut h = help_with(vec![msg(1000, -1)]);
        h.request(1000, 4);
        for _ in 0..50 { step(&mut h, 0); }
        assert_eq!(h.bx.state, 5);
        h.suspend();
        assert_eq!((h.bx.state, h.reopen_timer), (6, 60));
        for _ in 0..20 { step(&mut h, 0); }
        assert_eq!(h.bx.state, 8);
        h.resume();
        let mut n = 0;
        while h.bx.state == 8 {
            step(&mut h, 0);
            n += 1;
        }
        assert_eq!((n, h.bx.state, h.reopened), (60, 1, 1));
        // The reopened box runs its states and closes normally (state 7 → 0: the hold is off); the suspend's close
        // bumped nothing, the reopened box's hold end bumps once; 0x1798ca stays 1 (only state 8 clears it).
        for _ in 0..480 { step(&mut h, 0); }
        assert_eq!((h.bx.state, h.records.help[4].count, h.reopened), (0, 1, 1));
    }

    #[test]
    fn hero_hints_rules() {
        let mut rec = Records::default();
        let mut owned = [0u8; 37];
        // Slot types (item definitions +8): the packs / helmet / boots are not weapons.
        let mut types = [0i32; 37];
        for i in [2, 3, 4, 5, 6, 7, 28, 29] { types[i] = 3; }
        assert!(hero_hints(1, 0, &owned, &types, 128, &rec).is_empty());
        owned[4] = 1;
        assert_eq!(hero_hints(0x6a, 0, &owned, &types, 5, &rec), vec![(0x4e2e, 0x78)]);
        assert_eq!(hero_hints(1, 0x12, &owned, &types, 5, &rec), vec![(0x4e2e, 0x78)]);
        rec.help[0x78].count = 1;
        assert!(hero_hints(1, 0x12, &owned, &types, 5, &rec).is_empty());
        for i in [9, 10, 11, 13, 14, 15, 16, 17, 8, 0x18] { owned[i] = 1; }
        // 8 weapons besides the wrench and the drone device: not more than 8.
        assert!(hero_hints(1, 0, &owned, &types, 256, &rec).is_empty());
        owned[18] = 1;
        assert_eq!(hero_hints(1, 0, &owned, &types, 256, &rec), vec![(0x4e26, 0x50)]);
        assert!(hero_hints(1, 0, &owned, &types, 255, &rec).is_empty());
        rec.moves[21].count = 1;
        assert!(hero_hints(1, 0, &owned, &types, 256, &rec).is_empty());
    }
}
