//! The 989snd SFX block player (IOP): one block-sound handler per playing sound, its grain script run at
//! 240 Hz, tones keyed onto SPU voices, LFOs, and the handle API the EE's commands use.
//!
//! Behavioural reference: OpenGOAL `game/sound/989snd/blocksound_handler.cpp`, `sfxgrain.cpp`, `lfo.cpp`,
//! `vagvoice.cpp`, `util.cpp` (ISC; the same Sony library, Jak's build). Mirrored: handler construction
//! (`play vol = sfx.vol·vol >> 10`, the first grains run at once while the countdown ≤ 0), `Tick` (LFOs,
//! countdown, grains), `DoGrain` (the RAND_PLAY / PLAY_CYCLE skip window), `SetVolPan` / `SetPMod` /
//! `SetPBend`, the tone grain (register / random volume and pan, `MakeVolume`, `PitchBend` +
//! `PS1Note2Pitch`), the control grains 20–44 and the LFO shapes. RAC1's v1 grains carry their parameters
//! inline (`s16 param[4]`, RAND_DELAY's modulus as a plain s32).
//!
//! Not ported (counted in [`Snd989::unimplemented`], with the reason): XREF (2, 3), child sounds (5, 6),
//! plugin messages (7), BRANCH (8), noise tones (flag 8), reverb-only tones (flag 0x10). None of them occur in
//! the RAC1 banks (grain types used: 1, 4, 20–43; tone flags 8 and 0x10 are never set). Tone flag 1 ("to reverb",
//! every level-bank tone) marks the voice's reverb send ([`super::reverb`]).
//!
//! Randomness: 989snd calls the IOP C library `rand()`; which generator the RAC1 IRX links is not reversed.
//! [`IopRng`] is the newlib-style LCG seeded with 1 (inferred), so the output is deterministic.

use super::voices::{VoiceManager, VoiceUse};
use super::{Spu, VoiceData};
use rc_formats::sound_bank::{grain, Bank, Grain, Tone};
use rc_formats::vag;
use std::collections::BTreeMap;
use std::sync::Arc;

/// 989snd's `rand()` stand-in (see the module docs): `s = s·0x41c64e6d + 0x3039`, returns bits 16..30.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IopRng(pub u32);

impl Default for IopRng {
    fn default() -> Self { IopRng(1) }
}

impl IopRng {
    pub fn rand(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        ((self.0 >> 16) & 0x7fff) as i32
    }
}

/// `PAN_RESET` / `PAN_DONT_CHANGE` / `VOLUME_DONT_CHANGE` (989snd sentinels). A pan the EE computes as −1
/// or −2 degrees therefore means "reset" / "keep" to 989snd, as on the PS2.
pub const PAN_RESET: i32 = -1;
pub const PAN_DONT_CHANGE: i32 = -2;
pub const VOLUME_DONT_CHANGE: i32 = 0x7fff_ffff;

/// LFO tracker (OpenGOAL `LFOTracker`). Targets: 1 volume, 2 pan, 3 pitch mod, 4 pitch bend.
#[derive(Clone, Copy, Debug, Default)]
struct Lfo {
    shape: u8,
    target: u8,
    setup_flags: u8,
    depth: i32,
    next_step: i32,
    step_size: u32,
    state_hold1: i32,
    state_hold2: i32,
    range: i32,
    tick: u32,
}

/// The LFO sine: `trunc(32767·cos(2πi/2048))`, equal to OpenGOAL's `gLFO_sine` for all 2048 entries.
fn lfo_sine(i: usize) -> i32 { (32767.0 * (2.0 * std::f64::consts::PI * i as f64 / 2048.0).cos()) as i32 }

#[derive(Clone, Copy, Debug)]
struct VoiceRef {
    spu: usize,
    tone: Tone,
    g_vol: i32,
    g_pan: i32,
}

/// One playing sound (OpenGOAL `BlockSoundHandler`).
#[derive(Clone, Debug)]
struct Handler {
    sound: usize,
    group: u8,
    done: bool,
    paused: bool,
    next_grain: i32,
    countdown: i32,
    grains_to_play: u32,
    grains_to_skip: u32,
    skip_grains: bool,
    voices: Vec<VoiceRef>,
    orig_volume: i32,
    cur_volume: i32,
    cur_pan: i32,
    cur_pm: i32,
    cur_pb: i32,
    app_volume: i32,
    app_pan: i32,
    app_pm: i32,
    app_pb: i32,
    lfo_volume: i32,
    lfo_pan: i32,
    lfo_pm: i32,
    lfo_pb: i32,
    registers: [i8; 4],
    lfo: [Lfo; 4],
}

/// `PitchBend(tone, pb, pm, note, fine)`: `v = note·128 + fine + pm`, plus `pb_high·(pb << 7)/0x7fff` or
/// `pb_low·(pb << 7)/0x8000`; returns `(v / 128, v % 128)`.
pub fn pitch_bend(tone: &Tone, pb: i32, pm: i32, note: i32, fine: i32) -> (i16, i16) {
    let base = (note << 7) + fine + pm;
    let v = if pb >= 0 { tone.pb_high as i32 * (pb << 7) / 0x7fff + base } else { tone.pb_low as i32 * (pb << 7) / 0x8000 + base };
    ((v / 128) as i16, (v % 128) as i16)
}

/// The SPU pitch of a tone at the handler's bend and modulation (start note 60, fine 0).
pub fn tone_pitch(tone: &Tone, pb: i32, pm: i32) -> u16 {
    let (n, f) = pitch_bend(tone, pb, pm, 60, 0);
    vag::ps1_note_to_pitch(tone.center_note, tone.center_fine, n, f)
}

/// The 989snd side: handlers by handle, the voice manager, the 240 Hz tick.
#[derive(Clone, Debug)]
pub struct Snd989 {
    pub vm: VoiceManager,
    handlers: BTreeMap<u32, Handler>,
    next_handle: u32,
    /// 240 Hz ticks since start.
    pub tick: u64,
    pub rng: IopRng,
    /// The global registers (`g_block_reg`).
    block_reg: [i8; 32],
    /// Grain types / tone kinds met but not ported, with counts (see the module docs).
    pub unimplemented: BTreeMap<&'static str, u32>,
    pub tones_started: u64,
    /// The groups the last `snd_PauseAllSoundsInGroup` named, until continued ([`Snd989::set_groups_paused`]).
    pub paused_groups: u32,
    /// RAND_PLAY's "previous" and PLAY_CYCLE's "index", which 989snd writes back into the grain itself (param[2], the
    /// bank's copy): they carry over from one play of the sound to the next, so a PLAY_CYCLE sound alternates its
    /// options. Keyed by (sound, grain); a new bank (a new level) starts again from the file's values.
    grain_state: BTreeMap<(usize, usize), i16>,
}

impl Default for Snd989 {
    fn default() -> Self {
        Snd989 { vm: VoiceManager::default(), handlers: BTreeMap::new(), next_handle: 1, tick: 0, rng: IopRng::default(), block_reg: [0; 32], unimplemented: BTreeMap::new(), tones_started: 0, paused_groups: 0, grain_state: BTreeMap::new() }
    }
}

fn wrap360(mut p: i32) -> i32 {
    while p >= 360 { p -= 360; }
    while p < 0 { p += 360; }
    p
}

impl Snd989 {
    pub fn new() -> Self { Self::default() }

    /// Handles still playing.
    pub fn playing(&self) -> usize { self.handlers.len() }

    /// `snd_PlaySoundVolPanPMPB(bank, sound, vol, pan, pm, pb)`: a new handler (its first grains run now).
    /// Returns the handle, 0 when the sound has no grains or does not exist.
    #[allow(clippy::too_many_arguments)]
    pub fn play_sound(&mut self, bank: &Bank, spu: &mut Spu, sound: u16, vol: i32, pan: i32, pm: i32, pb: i32) -> u32 {
        let Some(sfx) = bank.sounds.get(sound as usize) else { return 0 };
        if sfx.grains.is_empty() { return 0; }
        let sfx_vol = sfx.vol as i32;
        let vol = if vol == VOLUME_DONT_CHANGE { 1024 } else { vol };
        let play_vol = ((sfx_vol * vol) >> 10).min(127);
        let pan = if pan == PAN_RESET || pan == PAN_DONT_CHANGE { sfx.pan as i32 } else { pan };
        let mut h = Handler {
            sound: sound as usize,
            group: sfx.vol_group as u8,
            done: false,
            paused: false,
            next_grain: 0,
            countdown: bank.grains[sfx.grains.start].delay,
            grains_to_play: 0,
            grains_to_skip: 0,
            skip_grains: false,
            voices: Vec::new(),
            orig_volume: sfx_vol,
            cur_volume: play_vol,
            cur_pan: pan,
            cur_pm: pm,
            cur_pb: pb,
            app_volume: vol,
            app_pan: pan,
            app_pm: pm,
            app_pb: pb,
            lfo_volume: 0,
            lfo_pan: 0,
            lfo_pm: 0,
            lfo_pb: 0,
            registers: [0; 4],
            lfo: [Lfo::default(); 4],
        };
        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1).max(1);
        while h.countdown <= 0 && !h.done { self.do_grain(&mut h, handle, bank, spu); }
        self.handlers.insert(handle, h);
        handle
    }

    /// `snd_SoundIsStillPlaying`: the handle, or 0.
    pub fn still_playing(&self, handle: i32) -> i32 { if handle > 0 && self.handlers.contains_key(&(handle as u32)) { handle } else { 0 } }

    /// `snd_StopSound`: key off every voice; the handler ends when they have released.
    pub fn stop_sound(&mut self, spu: &mut Spu, handle: i32) {
        let Some(h) = self.handlers.get_mut(&(handle as u32)) else { return };
        h.done = true;
        for v in &h.voices {
            if self.vm.owns(spu, v.spu, VoiceUse::Tone { handle: handle as u32 }) { spu.voices[v.spu].key_off(); }
        }
    }

    /// `snd_SetSoundParams(handle, mask, vol, pan, pm, pb)`: mask 1 volume, 2/4 pan, 8 pitch mod, 0x10 pitch
    /// bend (the EE's mask bits; the IOP's decoding of 4 vs 2 is inferred). Returns the handle or 0.
    #[allow(clippy::too_many_arguments)]
    pub fn set_params(&mut self, bank: &Bank, spu: &mut Spu, handle: i32, mask: u32, vol: i32, pan: i32, pm: i32, pb: i32) -> i32 {
        let key = handle as u32;
        let Some(mut h) = self.handlers.remove(&key) else { return 0 };
        let vol_arg = if mask & 1 != 0 { vol } else { VOLUME_DONT_CHANGE };
        let pan_arg = if mask & 6 != 0 { pan } else { PAN_DONT_CHANGE };
        if mask & 7 != 0 { self.set_vol_pan(&mut h, key, vol_arg, pan_arg, bank, spu); }
        if mask & 8 != 0 {
            h.app_pm = pm;
            self.update_pitch(&mut h, key, spu);
        }
        if mask & 0x10 != 0 {
            h.app_pb = pb;
            self.update_pitch(&mut h, key, spu);
        }
        self.handlers.insert(key, h);
        handle
    }

    pub fn set_master_volume(&mut self, group: usize, vol: i32, spu: &mut Spu) { self.vm.set_master_volume(group, vol, spu); }

    /// The pause flag of every handler whose group is in `mask` (`snd_PauseAllSoundsInGroup` /
    /// `snd_ContinueAllSoundsInGroup`: OpenGOAL `BlockSoundHandler::Pause` / `Unpause`). Sounds started later play (the
    /// command acts on the handlers that exist).
    pub fn set_groups_paused(&mut self, mask: u32, paused: bool) {
        if paused { self.paused_groups |= mask } else { self.paused_groups &= !mask }
        for h in self.handlers.values_mut() {
            if mask & (1 << (h.group & 31)) != 0 { h.paused = paused; }
        }
    }

    /// One 240 Hz tick of every handler; finished handlers are dropped.
    pub fn tick(&mut self, bank: &Bank, spu: &mut Spu) {
        self.tick += 1;
        let keys: Vec<u32> = self.handlers.keys().copied().collect();
        for key in keys {
            let mut h = self.handlers.remove(&key).unwrap();
            // Voices that ended (envelope stopped, or taken by another tone) leave the handler.
            h.voices.retain(|v| self.vm.owns(spu, v.spu, VoiceUse::Tone { handle: key }));
            if !h.paused { for k in 0..4 { self.lfo_tick(&mut h, k, key, bank, spu); } }
            let finished = h.done && h.voices.is_empty();
            if !finished {
                if !h.paused && !h.done {
                    h.countdown -= 1;
                    while h.countdown <= 0 && !h.done { self.do_grain(&mut h, key, bank, spu); }
                }
                self.handlers.insert(key, h);
            }
        }
    }

    fn reg_value(&mut self, h: &Handler, v: i32) -> i32 {
        if v >= -4 { h.registers[(-v - 1) as usize] as i32 } else if v == -5 { self.rng.rand() % 0x7f } else { self.block_reg[((-v - 6) as usize).min(31)] as i32 }
    }

    fn set_vol_pan(&mut self, h: &mut Handler, key: u32, vol: i32, pan: i32, bank: &Bank, spu: &mut Spu) {
        if vol >= 0 {
            if vol != VOLUME_DONT_CHANGE { h.app_volume = vol; }
        } else {
            h.app_volume = -1024 * vol / 127;
        }
        if pan == PAN_RESET {
            h.app_pan = bank.sounds[h.sound].pan as i32;
        } else if pan != PAN_DONT_CHANGE {
            h.app_pan = pan;
        }
        let new_vol = (((h.app_volume * h.orig_volume) >> 10) + h.lfo_volume).clamp(0, 127);
        let new_pan = wrap360(h.app_pan + h.lfo_pan);
        if new_pan != h.cur_pan || new_vol != h.cur_volume {
            h.cur_volume = new_vol;
            h.cur_pan = new_pan;
            for v in &h.voices {
                if !self.vm.owns(spu, v.spu, VoiceUse::Tone { handle: key }) { continue; }
                let base = self.vm.make_volume(127, 0, h.cur_volume, h.cur_pan, v.g_vol, v.g_pan);
                self.vm.voices[v.spu].basevol = base;
                // A paused sound keeps its voices silent; the continue applies the new volume.
                if !h.paused { spu.voices[v.spu].vol = self.vm.voice_registers(base, h.group as usize); }
            }
        }
    }

    fn update_pitch(&mut self, h: &mut Handler, key: u32, spu: &mut Spu) {
        h.cur_pm = h.app_pm + h.lfo_pm;
        h.cur_pb = (h.app_pb + h.lfo_pb).clamp(i16::MIN as i32, i16::MAX as i32);
        if h.paused { return; }
        for v in &h.voices {
            if self.vm.owns(spu, v.spu, VoiceUse::Tone { handle: key }) { spu.voices[v.spu].pitch = tone_pitch(&v.tone, h.cur_pb, h.cur_pm); }
        }
    }

    fn start_tone(&mut self, h: &mut Handler, key: u32, g: &Grain, bank: &Bank, spu: &mut Spu) {
        let tone = g.tone();
        h.cur_volume = (((h.app_volume * h.orig_volume) >> 10) + h.lfo_volume).clamp(0, 127);
        h.cur_pan = wrap360(h.app_pan + h.lfo_pan);
        if tone.flags & 8 != 0 {
            *self.unimplemented.entry("noise tone").or_default() += 1;
            return;
        }
        if tone.flags & 0x10 != 0 {
            *self.unimplemented.entry("reverb-only tone").or_default() += 1;
            return;
        }
        let mut vol = tone.vol as i32;
        if vol < 0 { vol = self.reg_value(h, vol); }
        let vol = vol.max(0);
        let mut pan = tone.pan as i32;
        if pan < 0 {
            pan = if pan >= -4 { 360 * h.registers[(-pan - 1) as usize] as i32 / 127 } else if pan == -5 { self.rng.rand() % 360 } else { 360 * self.block_reg[((-pan - 6) as usize).min(31)] as i32 / 127 };
        }
        let pan = wrap360(pan);
        let group = h.group as usize;
        let Some(i) = self.vm.allocate(spu, group, tone.priority, self.tick) else {
            self.vm.dropped += 1;
            return;
        };
        let base = self.vm.make_volume(127, 0, h.cur_volume, h.cur_pan, vol, pan);
        let pitch = tone_pitch(&tone, h.cur_pb, h.cur_pm);
        spu.voices[i].key_on(VoiceData::Ram(Arc::clone(&spu.ram)), tone.sample_offset as usize, pitch, tone.adsr1, tone.adsr2);
        spu.voices[i].vol = self.vm.voice_registers(base, group);
        // Tone flag 1: the voice also feeds the reverb (every level-bank tone; super::reverb).
        spu.voices[i].reverb = tone.flags & 1 != 0;
        let gen = spu.voices[i].generation;
        self.vm.voices[i] = super::voices::VoiceSlot { owner: VoiceUse::Tone { handle: key }, priority: tone.priority, group: h.group, basevol: base, start_tick: self.tick, generation: gen };
        h.voices.push(VoiceRef { spu: i, tone, g_vol: vol, g_pan: pan });
        self.tones_started += 1;
        let _ = bank;
    }

    fn find_marker(bank: &Bank, h: &Handler, mark: i16) -> Option<usize> {
        bank.sound_grains(h.sound).iter().position(|g| g.kind == grain::MARKER && g.params()[0] == mark)
    }

    /// `DoGrain`: run grain `next_grain`, apply the skip window, advance, reload the countdown.
    fn do_grain(&mut self, h: &mut Handler, key: u32, bank: &Bank, spu: &mut Spu) {
        let grains = bank.sound_grains(h.sound);
        let gi = h.next_grain as usize;
        let Some(g) = grains.get(gi).copied() else {
            h.done = true;
            return;
        };
        let ret = self.run_grain(h, key, gi, &g, bank, spu);
        if h.skip_grains {
            h.grains_to_play = h.grains_to_play.saturating_sub(1);
            if h.grains_to_play == 0 {
                h.next_grain += h.grains_to_skip as i32;
                h.skip_grains = false;
            }
        }
        h.next_grain += 1;
        if h.next_grain as usize >= grains.len() || h.next_grain < 0 {
            h.done = true;
            return;
        }
        h.countdown = grains[h.next_grain as usize].delay + ret;
    }

    fn run_grain(&mut self, h: &mut Handler, key: u32, gi: usize, g: &Grain, bank: &Bank, spu: &mut Spu) -> i32 {
        let p = g.params();
        let set_reg = |this: &mut Snd989, h: &mut Handler, reg: i16, v: i32| {
            if reg < 0 { this.block_reg[((-reg - 1) as usize).min(31)] = v as i8 } else { h.registers[(reg as usize).min(3)] = v as i8 }
        };
        let get_reg = |this: &Snd989, h: &Handler, reg: i16| -> i32 {
            if reg < 0 { this.block_reg[((-reg - 1) as usize).min(31)] as i32 } else { h.registers[(reg as usize).min(3)] as i32 }
        };
        match g.kind {
            grain::TONE | grain::TONE2 => self.start_tone(h, key, g, bank, spu),
            grain::LFO_SETTINGS => {
                let lp = g.lfo();
                let l = &mut h.lfo[(lp.which_lfo as usize).min(3)];
                l.target = lp.target;
                if l.target != 0 {
                    l.shape = lp.shape;
                    l.setup_flags = lp.flags as u8;
                    l.depth = lp.depth as i16 as i32;
                    l.step_size = lp.step_size;
                    l.state_hold1 = if lp.shape == 2 { lp.duty_cycle as i32 } else { 0 };
                    l.state_hold2 = 0;
                    l.next_step = if l.setup_flags & 2 != 0 { (self.rng.rand() & 0x7ff) << 16 } else { (lp.start_offset as i32) << 16 };
                    // Init: RAND seeds its hold, CalcDepth, then one Tick.
                    if l.shape == 5 {
                        let a = self.rng.rand() & 0x7fff;
                        let b = self.rng.rand() & 1;
                        l.state_hold1 = -a * b;
                        l.state_hold2 = 1;
                    }
                    l.range = match l.target {
                        1 => (bank.sounds[h.sound].vol as i32 * l.depth) >> 10,
                        2 => (180 * l.depth) >> 10,
                        3 => (6096 * l.depth) >> 10,
                        4 => (0x7fff * l.depth) >> 10,
                        _ => 0,
                    };
                    let k = (lp.which_lfo as usize).min(3);
                    self.lfo_tick(h, k, key, bank, spu);
                } else {
                    l.shape = 0;
                }
            }
            grain::CONTROL_NULL | grain::LOOP_START | grain::MARKER | grain::NULL => {}
            grain::LOOP_END => {
                let grains = bank.sound_grains(h.sound);
                match (0..gi).rev().find(|&i| grains[i].kind == grain::LOOP_START) {
                    Some(i) => h.next_grain = i as i32 - 1,
                    None => *self.unimplemented.entry("LOOP_END without LOOP_START").or_default() += 1,
                }
            }
            grain::LOOP_CONTINUE => {
                let grains = bank.sound_grains(h.sound);
                if let Some(i) = (gi + 1..grains.len()).find(|&i| grains[i].kind == grain::LOOP_END) { h.next_grain = i as i32; }
            }
            grain::STOP => h.done = true,
            grain::RAND_PLAY => {
                let (options, count) = (p[0] as i32, p[1] as i32);
                let prev = self.grain_state.get(&(h.sound, gi)).copied().unwrap_or(p[2]) as i32;
                let mut rnd = self.rng.rand() % options.max(1);
                if rnd == prev {
                    rnd += 1;
                    if rnd >= options { rnd = 0; }
                }
                self.grain_state.insert((h.sound, gi), rnd as i16);
                h.next_grain += rnd * count;
                h.grains_to_play = (count + 1) as u32;
                h.grains_to_skip = ((options - 1 - rnd) * count) as u32;
                h.skip_grains = true;
            }
            grain::RAND_DELAY => {
                let amount = g.rand_delay_amount();
                return if amount > 0 { self.rng.rand() % amount } else { 0 };
            }
            grain::RAND_PB => {
                let pb = p[0] as i32;
                let rnd = self.rng.rand();
                h.app_pb = pb * ((0xffff * (rnd % 0x7fff)) / 0x7fff - 0x8000) / 100;
                self.update_pitch(h, key, spu);
            }
            grain::ADD_PB => {
                h.app_pb = (h.cur_pb + 0x7fff * p[0] as i32 / 127).clamp(i16::MIN as i32, i16::MAX as i32);
                self.update_pitch(h, key, spu);
            }
            grain::PB => {
                let pb = p[0] as i32;
                h.app_pb = if pb >= 0 { 0x7fff * pb / 127 } else { -0x8000 * pb / -128 };
                self.update_pitch(h, key, spu);
            }
            grain::SET_REGISTER => set_reg(self, h, p[0], p[1] as i32),
            grain::SET_REGISTER_RAND => {
                let range = (p[2] as i32 - p[1] as i32 + 1).max(1);
                let v = self.rng.rand() % range + p[1] as i32;
                set_reg(self, h, p[0], v);
            }
            grain::INC_REGISTER => {
                let v = (get_reg(self, h, p[0]) + 1).clamp(-128, 127);
                set_reg(self, h, p[0], v);
            }
            grain::DEC_REGISTER => {
                let v = (get_reg(self, h, p[0]) - 1).clamp(-128, 127);
                set_reg(self, h, p[0], v);
            }
            grain::TEST_REGISTER => {
                let (value, action, cmp) = (get_reg(self, h, p[0]), p[1], p[2] as i32);
                let skip = match action {
                    0 => value >= cmp,
                    1 => value != cmp,
                    _ => cmp >= value,
                };
                if skip { h.next_grain += 1; }
            }
            grain::GOTO_MARKER => match Self::find_marker(bank, h, p[0]) {
                Some(i) => h.next_grain = i as i32 - 1,
                None => *self.unimplemented.entry("GOTO_MARKER to a missing marker").or_default() += 1,
            },
            grain::GOTO_RANDOM_MARKER => {
                let range = (p[1] as i32 - p[0] as i32 + 1).max(1);
                let target = (self.rng.rand() % range + p[0] as i32) as i16;
                match Self::find_marker(bank, h, target) {
                    Some(i) => h.next_grain = i as i32 - 1,
                    None => *self.unimplemented.entry("GOTO_RANDOM_MARKER to a missing marker").or_default() += 1,
                }
            }
            grain::WAIT_FOR_ALL_VOICES => {
                h.voices.retain(|v| self.vm.owns(spu, v.spu, VoiceUse::Tone { handle: key }));
                if !h.voices.is_empty() {
                    h.next_grain -= 1;
                    return 1;
                }
            }
            grain::PLAY_CYCLE => {
                let (group_size, group_count) = (p[0] as i32, p[1] as i32);
                let a = self.grain_state.get(&(h.sound, gi)).copied().unwrap_or(p[2]) as i32;
                let mut index = a + 1;
                if index == group_size { index = 0; }
                self.grain_state.insert((h.sound, gi), index as i16);
                h.next_grain += group_count * a;
                h.grains_to_play = (group_count + 1) as u32;
                h.grains_to_skip = ((group_size - 1 - a) * group_count) as u32;
                h.skip_grains = true;
            }
            grain::ADD_REGISTER => {
                let v = (get_reg(self, h, p[1]) + p[0] as i32).clamp(-128, 127);
                set_reg(self, h, p[1], v);
            }
            grain::KEY_OFF_VOICES => {
                for v in &h.voices {
                    if self.vm.owns(spu, v.spu, VoiceUse::Tone { handle: key }) { spu.voices[v.spu].key_off(); }
                }
            }
            grain::KILL_VOICES => {
                for v in &h.voices {
                    if self.vm.owns(spu, v.spu, VoiceUse::Tone { handle: key }) {
                        spu.voices[v.spu].key_off();
                        spu.voices[v.spu].vol = [0, 0];
                    }
                }
            }
            grain::ON_STOP_MARKER => h.next_grain = bank.sound_grains(h.sound).len() as i32 - 1,
            grain::COPY_REGISTER => {
                let v = get_reg(self, h, p[0]);
                set_reg(self, h, p[1], v);
            }
            grain::XREF_ID | grain::XREF_NUM => *self.unimplemented.entry("XREF grain").or_default() += 1,
            grain::START_CHILD_SOUND | grain::STOP_CHILD_SOUND => *self.unimplemented.entry("child sound grain").or_default() += 1,
            grain::PLUGIN_MESSAGE => *self.unimplemented.entry("plugin message grain").or_default() += 1,
            grain::BRANCH => *self.unimplemented.entry("branch grain").or_default() += 1,
            _ => *self.unimplemented.entry("unknown grain type").or_default() += 1,
        }
        0
    }

    /// `LFOTracker::Tick` (every other call does work) + `GetLFO(2)`.
    fn lfo_tick(&mut self, h: &mut Handler, k: usize, key: u32, bank: &Bank, spu: &mut Spu) {
        let l = &mut h.lfo[k];
        l.tick = l.tick.wrapping_add(1);
        if l.target == 0 || l.tick & 1 == 0 { return; }
        let step = (l.next_step >> 16) as usize;
        l.next_step = l.next_step.wrapping_add((2 * l.step_size) as i32);
        if l.next_step > 0x7ff_ffff { l.next_step -= 0x800_0000; }
        let step = step.min(2047);
        let mut v = match l.shape {
            1 => lfo_sine(step),
            2 => if step as i32 >= l.state_hold1 { -32767 } else { 32767 },
            3 => {
                let s = step as i32;
                if s < 512 { 0x7fff * s / 512 } else if s >= 1536 { 0x7fff * (s - 1536) / 512 - 0x7fff } else { 0x7fff - 65534 * (s - 512) / 1024 }
            }
            4 => {
                let s = step as i32;
                if s >= 1024 { 0x7fff * (s - 1024) / 1024 - 0x7fff } else { 0x7fff * s / 1023 }
            }
            5 => {
                if step >= 1024 && l.state_hold2 == 1 {
                    l.state_hold2 = 0;
                    l.state_hold1 = 2 * ((self.rng.rand() & 0x7fff) - 0x3fff);
                } else if step < 1024 && l.state_hold2 == 0 {
                    l.state_hold2 = 1;
                    let a = self.rng.rand() & 0x7fff;
                    let b = self.rng.rand() & 1;
                    l.state_hold1 = -a * b;
                }
                l.state_hold1
            }
            _ => 0,
        };
        if l.setup_flags & 1 != 0 { v = -v; }
        let (target, range) = (l.target, l.range);
        match target {
            1 => {
                let vol = (range * (v - 0x7fff)) >> 16;
                if h.lfo_volume != vol {
                    h.lfo_volume = vol;
                    self.set_vol_pan(h, key, VOLUME_DONT_CHANGE, PAN_DONT_CHANGE, bank, spu);
                }
            }
            2 => {
                let pan = (range * v) >> 15;
                if h.lfo_pan != pan {
                    h.lfo_pan = pan;
                    self.set_vol_pan(h, key, VOLUME_DONT_CHANGE, PAN_DONT_CHANGE, bank, spu);
                }
            }
            3 => {
                let pm = (v * range) >> 15;
                if h.lfo_pm != pm {
                    h.lfo_pm = pm;
                    self.update_pitch(h, key, spu);
                }
            }
            4 => {
                let pb = (v * range) >> 15;
                if h.lfo_pb != pb {
                    h.lfo_pb = pb;
                    self.update_pitch(h, key, spu);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lfo_sine_table() {
        assert_eq!([lfo_sine(0), lfo_sine(1), lfo_sine(2), lfo_sine(3)], [32767, 32766, 32766, 32765]);
        assert_eq!([lfo_sine(510), lfo_sine(511), lfo_sine(512), lfo_sine(513)], [201, 100, 0, -100]);
        assert_eq!(lfo_sine(1024), -32767);
    }

    #[test]
    fn tone_pitches() {
        // Novalis ambience tone: centre −74 / 66 (a PS2-rate sample), played at note 60: ≈ 22.0 kHz.
        let t = Tone { center_note: -74, center_fine: 66, ..Default::default() };
        let p = tone_pitch(&t, 0, 0);
        let hz = p as f64 * 48000.0 / 4096.0;
        assert!((hz - 21_994.0).abs() < 60.0, "{p:#x} = {hz:.0} Hz");
        // PS1-style +72 / 0: 22.05 kHz.
        let t = Tone { center_note: 72, center_fine: 0, ..Default::default() };
        assert_eq!(tone_pitch(&t, 0, 0), (0x800u32 * 44100 / 48000) as u16);
        // A full bend up of pb_high = 2 semitones.
        let t = Tone { center_note: -60, pb_high: 2, ..Default::default() };
        assert_eq!(pitch_bend(&t, 0x7fff, 0, 60, 0), (62, 0));
        assert_eq!(tone_pitch(&t, 0x7fff, 0), vag::note_to_pitch(60, 0, 62, 0));
    }
}
