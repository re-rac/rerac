//! Audio: the EE sound manager, the 989snd sound player, the music state machine and a software SPU2, run
//! at the game's rates (docs/plan/audio.md). One [`AudioSystem`] owns everything:
//!
//! * [`voices`]: the EE side of `sound_update` (level01 0x2a0638): 30 logical slots, distance volume, pan,
//!   doppler, underwater, the 36-sample occlusion ring, master groups; the sound-instance emitters; and the
//!   989snd voice manager (SPU voice allocation, `MakeVolume`, group volumes).
//! * [`grain_vm`]: the 989snd SFX block player: block-sound handlers, the grain VM at 240 Hz, tones, LFOs.
//! * [`music`]: `music_Update` (0x27a688) and friends, the music trigger boxes, and the IOP VAG stream players.
//! * this module: the SPU2 core (ADPCM voices with the 4-tap Gaussian interpolation and the ADSR state
//!   machine), the 48 kHz stereo mix, and the frame driver.
//!
//! Timing: [`AudioSystem::tick`] runs one 60 Hz game frame (the EE part) and then renders exactly 800 output
//! samples (48000 / 60) on the IOP/SPU side, ticking the 989snd handlers every 200 samples (240 Hz). EE → IOP
//! commands issued in frame N are executed at the start of frame N's 800 samples; their callbacks reach the EE
//! at the start of frame N+1, like the RPC round trip. Everything is integer or fixed-order float arithmetic,
//! so the output is a pure function of the per-frame inputs (listener, hero position).
//!
//! * [`reverb`]: the reverb zones (reverb boxes, env sample points, the checkpoint copy, `sound_update`'s reverb
//!   commands) and the native reverb effect the mix sends the "to reverb" tones through.
//!
//! Not modelled: the two SPU2 cores' separate mixing stages (the reverb takes the reverb-flagged voices of every
//! core), noise voices, voice pitch modulation (PMON), the 989snd RPC layer itself.

pub mod class_sounds;
pub mod grain_vm;
pub mod music;
pub mod reverb;
pub mod scene;
pub mod voices;

use rc_formats::collision::Collision;
use rc_formats::sound_bank::{Bank, EnvSamplePoint, LevelSounds, SoundInstance};
use rc_formats::vag::{self, History, FLAG_END, FLAG_LOOP_START, FLAG_REPEAT};
use std::collections::VecDeque;
use std::sync::Arc;

/// Output rate: the SPU2 runs at 48 kHz.
pub const OUTPUT_RATE: u32 = 48_000;
/// Output samples per 60 Hz game frame.
pub const SAMPLES_PER_FRAME: usize = 800;
/// Output samples per 989snd tick (240 Hz).
pub const SAMPLES_PER_IOP_TICK: usize = 200;
/// SPU2 voices (two cores of 24).
pub const SPU_VOICES: usize = 48;

// ---------------------------------------------------------------------------------------------------
// ADSR

/// The SPU2 envelope generator (psx-spx "SPU ADSR", PCSX2 / OpenGOAL `common/envelope.cpp`, whose state
/// machine and step rule this mirrors): ADSR1/ADSR2 bit fields select, per phase, linear or exponential
/// steps of `step << max(0, 11 − shift)` every `2^max(0, shift − 11)` samples, with the exponential
/// increase slowed ×4 above 0x6000 and the exponential decrease scaled by the level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Adsr {
    pub adsr1: u16,
    pub adsr2: u16,
    pub phase: Phase,
    /// 0..=0x7fff.
    pub level: i32,
    counter: u32,
    shift: u8,
    step: i8,
    exp: bool,
    decrease: bool,
    target: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    Attack,
    Decay,
    Sustain,
    Release,
    #[default]
    Stopped,
}

impl Adsr {
    fn reg(&self) -> u32 { (self.adsr2 as u32) << 16 | self.adsr1 as u32 }

    fn update_settings(&mut self) {
        let r = self.reg();
        let bits = |pos: u32, n: u32| (r >> pos) & ((1 << n) - 1);
        match self.phase {
            Phase::Attack => {
                self.exp = bits(15, 1) != 0;
                self.decrease = false;
                self.shift = bits(10, 5) as u8;
                self.step = 7 - bits(8, 2) as i8;
                self.target = 0x7fff;
            }
            Phase::Decay => {
                self.exp = true;
                self.decrease = true;
                self.shift = bits(4, 4) as u8;
                self.step = -8;
                self.target = ((bits(0, 4) + 1) << 11) as i32;
            }
            Phase::Sustain => {
                self.exp = bits(31, 1) != 0;
                self.decrease = bits(30, 1) != 0;
                self.shift = bits(24, 5) as u8;
                let s = bits(22, 2) as i8;
                self.step = if self.decrease { -8 + s } else { 7 - s };
                self.target = 0;
            }
            Phase::Release => {
                self.exp = bits(21, 1) != 0;
                self.decrease = true;
                self.shift = bits(16, 5) as u8;
                self.step = -8;
                self.target = 0;
            }
            Phase::Stopped => {}
        }
    }

    /// Key on: attack from level 0.
    pub fn attack(&mut self) {
        self.phase = Phase::Attack;
        self.level = 0;
        self.counter = 0;
        self.update_settings();
    }

    /// Key off: release from the current level.
    pub fn release(&mut self) {
        if self.phase == Phase::Stopped { return; }
        self.phase = Phase::Release;
        self.counter = 0;
        self.update_settings();
    }

    /// End of a non-repeating sample: silent at once.
    pub fn stop(&mut self) {
        self.phase = Phase::Stopped;
        self.level = 0;
    }

    fn step(&mut self) {
        let mut c_step: u32 = 0x80_0000;
        let s = self.shift as i32 - 11;
        if s > 0 { c_step >>= s; }
        let mut step = ((self.step as i32) << (11 - self.shift as i32).max(0)) as i16 as i32;
        if self.exp {
            if !self.decrease && self.level > 0x6000 { c_step >>= 2; }
            if self.decrease { step = (step * self.level) >> 15; }
        }
        self.counter += c_step;
        if self.counter >= 0x80_0000 {
            self.counter = 0;
            self.level = (self.level + step).clamp(0, 0x7fff);
        }
    }

    /// One sample.
    pub fn run(&mut self) {
        if self.phase == Phase::Stopped { return; }
        self.step();
        if self.phase == Phase::Sustain { return; }
        if (!self.decrease && self.level >= self.target) || (self.decrease && self.level <= self.target) {
            self.phase = match self.phase {
                Phase::Attack => Phase::Decay,
                Phase::Decay => Phase::Sustain,
                Phase::Release => Phase::Stopped,
                p => p,
            };
            self.update_settings();
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// Gaussian interpolation

/// The SPU's 512-entry Gaussian interpolation table (psx-spx "SPU Gaussian Interpolation"; PCSX2 and
/// DuckStation carry the same numbers, OpenGOAL `common/interp_table.inc` as 256 × 4 rows
/// `{g[255−i], g[511−i], g[256+i], g[i]}`). A hardware constant, not disc data. Each row sums to ~0x7f80.
#[rustfmt::skip]
pub static GAUSS: [i16; 512] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 3, 3,
    3, 4, 4, 5, 5, 6, 7, 7, 8, 9, 9, 10, 11, 12, 13, 14,
    15, 16, 17, 18, 19, 21, 22, 24, 25, 27, 28, 30, 32, 33, 35, 37,
    39, 41, 44, 46, 48, 51, 53, 56, 58, 61, 64, 67, 70, 73, 77, 80,
    84, 87, 91, 95, 99, 103, 107, 111, 116, 120, 125, 130, 135, 140, 145, 150,
    156, 161, 167, 173, 179, 186, 192, 199, 205, 212, 219, 227, 234, 242, 250, 257,
    266, 274, 283, 291, 300, 309, 319, 328, 338, 348, 358, 369, 379, 390, 401, 412,
    424, 436, 448, 460, 473, 485, 498, 512, 525, 539, 553, 567, 582, 597, 612, 627,
    643, 659, 675, 692, 708, 726, 743, 761, 779, 797, 816, 835, 854, 874, 894, 914,
    935, 956, 977, 999, 1020, 1043, 1066, 1089, 1112, 1136, 1160, 1184, 1209, 1234, 1260, 1286,
    1312, 1339, 1366, 1394, 1422, 1450, 1479, 1508, 1537, 1567, 1598, 1628, 1660, 1691, 1723, 1756,
    1789, 1822, 1856, 1890, 1924, 1959, 1995, 2031, 2067, 2104, 2141, 2179, 2217, 2256, 2295, 2334,
    2374, 2415, 2456, 2497, 2539, 2582, 2624, 2668, 2712, 2756, 2801, 2846, 2892, 2938, 2985, 3032,
    3079, 3128, 3176, 3225, 3275, 3325, 3376, 3427, 3479, 3531, 3584, 3637, 3691, 3745, 3799, 3855,
    3910, 3967, 4023, 4081, 4138, 4197, 4255, 4315, 4374, 4435, 4495, 4557, 4619, 4681, 4744, 4807,
    4871, 4935, 5000, 5065, 5131, 5197, 5264, 5332, 5399, 5468, 5536, 5606, 5676, 5746, 5817, 5888,
    5959, 6032, 6104, 6177, 6251, 6325, 6400, 6475, 6550, 6626, 6702, 6779, 6856, 6934, 7012, 7091,
    7170, 7249, 7329, 7409, 7490, 7571, 7653, 7735, 7817, 7900, 7983, 8066, 8150, 8234, 8319, 8404,
    8489, 8575, 8661, 8748, 8834, 8922, 9009, 9097, 9185, 9273, 9362, 9451, 9541, 9630, 9720, 9811,
    9901, 9992, 10083, 10174, 10266, 10358, 10450, 10542, 10635, 10727, 10820, 10913, 11007, 11100, 11194, 11288,
    11382, 11476, 11571, 11665, 11760, 11855, 11950, 12045, 12140, 12236, 12331, 12427, 12522, 12618, 12714, 12809,
    12905, 13001, 13097, 13193, 13289, 13385, 13481, 13577, 13673, 13769, 13865, 13961, 14056, 14152, 14248, 14343,
    14439, 14534, 14630, 14725, 14820, 14915, 15010, 15104, 15199, 15293, 15387, 15481, 15575, 15669, 15762, 15855,
    15948, 16041, 16133, 16226, 16317, 16409, 16500, 16592, 16682, 16773, 16863, 16953, 17042, 17131, 17220, 17308,
    17396, 17484, 17571, 17658, 17744, 17830, 17916, 18001, 18086, 18170, 18254, 18337, 18420, 18502, 18584, 18665,
    18746, 18826, 18905, 18985, 19063, 19141, 19219, 19295, 19372, 19447, 19522, 19597, 19671, 19744, 19816, 19888,
    19959, 20030, 20100, 20169, 20238, 20306, 20373, 20439, 20505, 20570, 20634, 20698, 20760, 20822, 20884, 20944,
    21004, 21063, 21121, 21178, 21235, 21290, 21345, 21399, 21452, 21505, 21556, 21607, 21657, 21706, 21754, 21801,
    21848, 21893, 21938, 21982, 22025, 22066, 22107, 22148, 22187, 22225, 22262, 22299, 22334, 22369, 22402, 22435,
    22467, 22498, 22527, 22556, 22584, 22611, 22637, 22662, 22686, 22709, 22731, 22752, 22772, 22791, 22809, 22826,
    22842, 22857, 22872, 22885, 22897, 22908, 22918, 22927, 22935, 22942, 22948, 22953, 22957, 22960, 22962, 22963,
];

/// `out = Σ (g · s) >> 15` over the four samples `s[idx−3..=idx]` with `i = (counter >> 4) & 0xff`, each
/// product shifted separately as the hardware does.
#[inline]
pub fn gauss_interpolate(i: usize, s: [i32; 4]) -> i32 {
    ((GAUSS[0xff - i] as i32 * s[0]) >> 15)
        + ((GAUSS[0x1ff - i] as i32 * s[1]) >> 15)
        + ((GAUSS[0x100 + i] as i32 * s[2]) >> 15)
        + ((GAUSS[i] as i32 * s[3]) >> 15)
}

// ---------------------------------------------------------------------------------------------------
// SPU voices

/// One queued VAG stream part: a body up to and including its end frame.
#[derive(Clone, Debug)]
pub struct StreamPart {
    pub data: Arc<[u8]>,
    /// Repeat this part when it ends and nothing is queued behind it (stream flag 4).
    pub looped: bool,
}

/// Where a voice reads its ADPCM frames.
#[derive(Clone, Debug)]
pub enum VoiceData {
    /// Sound RAM (the level bank's sample chunk), with the hardware loop rules: LSA latches at a
    /// loop-start frame; after an end frame the voice jumps to LSA if the repeat flag is set, else it stops
    /// (ADSR level 0 at once, psx-spx "Loop End without Repeat").
    Ram(Arc<[u8]>),
    /// A 989snd VAG stream: after the end frame of the current part the next queued part starts with the
    /// decoder history carried over (the stream player feeds one voice from a ring buffer), the last part
    /// repeats when looped, otherwise the voice stops. The file flags are otherwise ignored (inferred).
    Stream { cur: StreamPart, queue: VecDeque<StreamPart> },
}

impl Default for VoiceData {
    fn default() -> Self { VoiceData::Ram(Arc::from(Vec::new())) }
}

/// One SPU2 voice: ADPCM decode at the current address, 4-tap Gaussian interpolation at the pitch step
/// (psx-spx / PCSX2: `counter += min(pitch, 0x3fff)`, 12 fraction bits, the interpolation over the current
/// sample and the three before it, the last three of the previous block at a block start, zeros at key-on),
/// the envelope, and the left/right volume registers (sweep never enabled by 989snd: level = reg << 1).
/// OpenGOAL's `Voice::Run` interpolates over the next four decoded samples instead, three samples earlier.
#[derive(Clone, Debug, Default)]
pub struct SpuVoice {
    pub adsr: Adsr,
    pub pitch: u16,
    /// VOLL / VOLR registers (0..0x3fff as 989snd writes them).
    pub vol: [u16; 2],
    /// Bumped on every key-on, so owners can tell a reused voice from theirs.
    pub generation: u32,
    data: VoiceData,
    nax: usize,
    lsa: usize,
    flags: u8,
    block: [i16; vag::FRAME_SAMPLES],
    prev: [i16; 3],
    hist: History,
    counter: u32,
    /// Frames played in the current stream part (for the time remaining).
    part_frame: usize,
    /// The voice's output also goes to the reverb (989snd sets it for a tone with flag 1; cleared at key-on).
    pub reverb: bool,
}

impl SpuVoice {
    fn frame(&self, at: usize) -> Option<&[u8]> {
        let d: &[u8] = match &self.data {
            VoiceData::Ram(r) => r,
            VoiceData::Stream { cur, .. } => &cur.data,
        };
        d.get(at..at + vag::FRAME_BYTES)
    }

    fn load_block(&mut self) {
        let Some(f) = self.frame(self.nax) else {
            // Past the data (a malformed sample): silence.
            self.adsr.stop();
            return;
        };
        let f: [u8; 16] = f.try_into().unwrap();
        self.flags = f[1];
        self.block = vag::decode_frame(&f, &mut self.hist, vag::Variant::Rounded);
        if self.flags & FLAG_LOOP_START != 0 { self.lsa = self.nax; }
    }

    /// Key on at byte address `start` of `data`.
    pub fn key_on(&mut self, data: VoiceData, start: usize, pitch: u16, adsr1: u16, adsr2: u16) {
        self.data = data;
        self.nax = start;
        self.lsa = start;
        self.hist = History::default();
        self.prev = [0; 3];
        self.counter = 0;
        self.part_frame = 0;
        self.pitch = pitch;
        self.adsr.adsr1 = adsr1;
        self.adsr.adsr2 = adsr2;
        self.generation = self.generation.wrapping_add(1);
        self.reverb = false;
        self.load_block();
        self.adsr.attack();
    }

    pub fn key_off(&mut self) { self.adsr.release(); }

    /// Silence at once (voice stolen, stream stopped).
    pub fn stop(&mut self) { self.adsr.stop(); }

    pub fn active(&self) -> bool { self.adsr.phase != Phase::Stopped }

    /// Queue a stream part behind the current one.
    pub fn queue_part(&mut self, part: StreamPart) {
        if let VoiceData::Stream { queue, .. } = &mut self.data { queue.push_back(part); }
    }

    /// Stream: source samples left in the current part (0 for a RAM voice).
    pub fn stream_remaining(&self) -> usize {
        match &self.data {
            VoiceData::Stream { cur, .. } => {
                let frames = cur.data.len() / vag::FRAME_BYTES;
                (frames.saturating_sub(self.part_frame)) * vag::FRAME_SAMPLES - (self.counter >> 12) as usize
            }
            VoiceData::Ram(_) => 0,
        }
    }

    fn next_block(&mut self) {
        self.prev = [self.block[25], self.block[26], self.block[27]];
        if self.flags & FLAG_END != 0 {
            match &mut self.data {
                VoiceData::Ram(_) => {
                    self.nax = self.lsa;
                    if self.flags & FLAG_REPEAT == 0 {
                        self.adsr.stop();
                        return;
                    }
                }
                VoiceData::Stream { cur, queue } => {
                    if let Some(next) = queue.pop_front() {
                        *cur = next;
                    } else if !cur.looped {
                        self.adsr.stop();
                        return;
                    }
                    self.nax = 0;
                    self.lsa = 0;
                    self.part_frame = 0;
                }
            }
        } else {
            self.nax += vag::FRAME_BYTES;
            self.part_frame += 1;
        }
        self.load_block();
    }

    /// One output sample: (left, right) after envelope and volume.
    pub fn run(&mut self) -> (i32, i32) {
        if self.adsr.phase == Phase::Stopped { return (0, 0); }
        let idx = (self.counter >> 12) as usize;
        let at = |k: isize| -> i32 { if k >= 0 { self.block[k as usize] as i32 } else { self.prev[(3 + k) as usize] as i32 } };
        let k = idx as isize;
        let s = gauss_interpolate(((self.counter >> 4) & 0xff) as usize, [at(k - 3), at(k - 2), at(k - 1), at(k)]) as i16 as i32;
        let sample = ((s * self.adsr.level) >> 15) as i16 as i32;
        let level = |reg: u16| (reg << 1) as i16 as i32;
        let out = (((sample * level(self.vol[0])) >> 15) as i16 as i32, ((sample * level(self.vol[1])) >> 15) as i16 as i32);
        self.counter += self.pitch.min(0x3fff) as u32;
        while (self.counter >> 12) as usize >= vag::FRAME_SAMPLES {
            self.counter -= (vag::FRAME_SAMPLES as u32) << 12;
            self.next_block();
            if self.adsr.phase == Phase::Stopped { break; }
        }
        self.adsr.run();
        out
    }
}

/// The SPU2: 48 voices summed, the reverb-flagged ones also into the reverb effect whose wet output joins the sum
/// ([`reverb::ReverbFx`]), then the master volume (0x3fff, as 989snd leaves it) and a clamp to 16 bits.
/// The hardware saturates at each core's mix stage; here the sum is exact and clamped once (inferred;
/// only matters when the mix clips).
#[derive(Clone, Debug)]
pub struct Spu {
    pub voices: Vec<SpuVoice>,
    pub master: [u16; 2],
    /// Sound RAM: the level bank's sample chunk.
    pub ram: Arc<[u8]>,
    pub reverb: reverb::ReverbFx,
}

impl Spu {
    pub fn new(ram: Arc<[u8]>) -> Self { Spu { voices: vec![SpuVoice::default(); SPU_VOICES], master: [0x3fff; 2], ram, reverb: reverb::ReverbFx::default() } }

    pub fn mix(&mut self) -> [i16; 2] {
        let (mut l, mut r) = (0i32, 0i32);
        let mut send = [0i32; 2];
        for v in &mut self.voices {
            let (a, b) = v.run();
            l += a;
            r += b;
            if v.reverb {
                send[0] += a;
                send[1] += b;
            }
        }
        if self.reverb.active() {
            let [wl, wr] = self.reverb.run(send);
            l += wl;
            r += wr;
        }
        [master_out(l, self.master[0]), master_out(r, self.master[1])]
    }
}

/// The master volume stage of [`Spu::mix`] on the exact sum `x` of the voices and the reverb: `x · (reg << 1)`
/// (a signed 16-bit level) `>> 15`, clamped to 16 bits. The product is taken in 64 bits: three or more loud voices
/// (the R.Y.N.O.'s salvo: seven missiles with their class sound and blasts) take the 32-bit product past `i32`,
/// which panicked in the dev build (overflow check) and wrapped into a full-scale click in the release build.
pub fn master_out(x: i32, reg: u16) -> i16 {
    ((x as i64 * (reg << 1) as i16 as i64) >> 15).clamp(i16::MIN as i64, i16::MAX as i64) as i16
}

// ---------------------------------------------------------------------------------------------------
// The frame driver

use music::{Music, MusicBox, MusicTrack, StreamCommand, StreamReply, Streams};
use voices::{Emitters, Listener, SndCommand, SndReply, SoundSlots, UpdateCtx};

/// Everything the audio system reads from the level.
#[derive(Clone)]
pub struct LevelAudio {
    pub bank: Arc<Bank>,
    pub sounds: LevelSounds,
    pub instances: Vec<SoundInstance>,
    /// Pvar block per sound instance.
    pub instance_pvars: Vec<Option<Vec<u8>>>,
    /// The level header's music table, loaded (track k = `music/k`).
    pub music: [Option<Arc<MusicTrack>>; 15],
    pub env_points: Vec<EnvSamplePoint>,
    /// For the occlusion rays; None = never occluded.
    pub collision: Option<Arc<Collision>>,
}

impl LevelAudio {
    /// Builds the level's audio data from its lumps: `sound_bank`, the raw core index with its parsed header
    /// and the decompressed core data (defs + remap), the decompressed gameplay file (sound instances, pvars,
    /// env points), the level header (music table) and the music VAGs by table slot. The collision mesh is
    /// optional (occlusion).
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        sound_bank: &[u8],
        core_index: &[u8],
        core: &rc_formats::level::LevelCore,
        core_data: &[u8],
        gameplay: &[u8],
        level_header: &[u8],
        music_files: &[Option<Vec<u8>>; 15],
        collision: Option<Collision>,
    ) -> rc_formats::buf::Result<Self> {
        use rc_formats::sound_bank as sb;
        let bank = Arc::new(sb::parse_bank(sound_bank)?);
        let mut sounds = sb::parse_level_sounds(core_index, core, core_data)?;
        // The hand items' (gadget classes') defs from their own blobs, as the game applies them when one is loaded.
        let gadgets = rc_formats::gadget::parse_gadget_classes(core, core_data)?;
        let blobs: Vec<(i32, &[u8])> = gadgets.iter().map(|g| (g.moby.o_class, g.blob.as_slice())).collect();
        sb::apply_gadget_defs(&mut sounds, &blobs)?;
        let instances = sb::parse_sound_instances(gameplay)?;
        let instance_pvars = instances.iter().map(|s| sb::pvar_block(gameplay, s.pvar_index)).collect::<rc_formats::buf::Result<Vec<_>>>()?;
        let table = sb::music_table(level_header)?;
        let mut music: [Option<Arc<MusicTrack>>; 15] = Default::default();
        for (k, m) in music.iter_mut().enumerate() {
            if table[k] == 0 { continue; }
            if let Some(bytes) = &music_files[k] { *m = Some(Arc::new(MusicTrack::from_vag(bytes)?)); }
        }
        let env_points = sb::parse_env_sample_points(gameplay)?;
        Ok(LevelAudio { bank, sounds, instances, instance_pvars, music, env_points, collision: collision.map(Arc::new) })
    }
}

/// One logged class sound play: `(tick, sound class, class sound index, flags, slot)`.
pub type ClassPlay = (u64, i16, i32, u32, i32);

/// Per-frame input from the game.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameInput {
    pub listener: Listener,
    /// The hero position 0x13f3d0 (music boxes test it).
    pub hero_pos: [f32; 3],
}

/// Counters for reports and tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioStats {
    pub frames: u64,
    pub samples: u64,
    pub plays: u64,
    /// `PlayClassSound` requests ([`class_sounds`]) and how many got a slot.
    pub class_sounds: u64,
    pub class_slots: u64,
    pub max_voices: usize,
    pub max_slots: usize,
}

/// The whole audio side of the game (see the module docs).
pub struct AudioSystem {
    pub data: LevelAudio,
    pub slots: SoundSlots,
    pub emitters: Emitters,
    pub music: Music,
    pub boxes: Vec<MusicBox>,
    pub snd: grain_vm::Snd989,
    pub streams: Streams,
    pub spu: Spu,
    /// A private `rand` stream (`srand(1234)`) for the standalone frames ([`AudioSystem::tick`] /
    /// [`AudioSystem::game_frame`], e.g. without a game tick). With the game running, the sound code draws from
    /// the game's one stream instead ([`AudioSystem::game_frame_with`] at the tick's sound step,
    /// [`class_sounds`] during the moby loop).
    pub rng: crate::rng::Rng,
    /// Option volumes (boot data: sfx 0x400, music 0x2cc).
    pub sfx_option: i32,
    pub music_option: i32,
    /// The Sound options page is open (`SoundOptionsMenu` 0x290538): group 2 (0x13e5a0) plays at the music volume
    /// while it is, back to sfx·8/10 on its leave 0x290508 (the engine sets it from the page menu).
    pub sound_page: bool,
    /// The game mode 0x15f5c4 as `sound_update` reads it (the engine copies the game's).
    pub game_mode: i32,
    pub stats: AudioStats,
    /// Mode-2 scene audio: speech voice, music pause / resume, cutscene volumes ([`scene`]).
    pub scene: scene::SceneAudio,
    /// The class sounds played since the caller last drained it, when logging (the engine's `RC_AUDIO_TRACE`):
    /// `(tick, sound class, class sound index, flags, slot)`; None: not logged.
    pub play_log: Option<Vec<ClassPlay>>,
    /// The EE's reverb request 0x13e5b0.. ([`reverb`]).
    pub reverb: reverb::ReverbRequest,
    /// The level's reverb boxes (sound instance class 3), in instance order.
    pub reverb_boxes: Vec<reverb::ReverbBox>,
    /// The reverb commands sent, when logging (`RC_AUDIO_TRACE`): `(frame, command)`.
    pub reverb_log: Option<Vec<(u64, SndCommand)>>,
    /// The underwater flag 0x167494 as the camera update last left it (the engine's `UnderwaterTest` 0x20e9f0,
    /// crate::fog_zones::UnderwaterState): the tick's sound step puts it into the listener
    /// ([`class_sounds::sound_step`]).
    pub underwater: bool,
    /// The checkpoint record's music track 0x1bc304 ([`AudioSystem::checkpoint_saved`]); None: no record this visit.
    pub checkpoint_track: Option<i16>,
    started: bool,
    /// The level start's `music_start_track(0, 1, 0x400)` done ([`AudioSystem::level_music_start`]).
    music_started: bool,
    snd_cmds: Vec<SndCommand>,
    stream_cmds: Vec<StreamCommand>,
    snd_replies: Vec<SndReply>,
    stream_replies: Vec<StreamReply>,
}

impl AudioSystem {
    pub fn new(data: LevelAudio) -> Self {
        let emitters = Emitters::new(data.instances.clone(), &data.instance_pvars);
        let boxes = data
            .instances
            .iter()
            .enumerate()
            .filter_map(|(i, s)| MusicBox::from_instance(i, s, data.instance_pvars.get(i)?.as_deref()?))
            .collect();
        let music = Music::new(std::array::from_fn(|k| data.music[k].is_some()));
        let reverb_boxes = data
            .instances
            .iter()
            .enumerate()
            .filter_map(|(i, s)| reverb::ReverbBox::from_instance(i, s, data.instance_pvars.get(i)?.as_deref()?))
            .collect();
        let spu = Spu::new(Arc::from(data.bank.samples.as_slice()));
        let mut rng = crate::rng::Rng::new();
        rng.srand(crate::rng::LEVEL_SEED);
        AudioSystem {
            data,
            slots: SoundSlots::new(),
            emitters,
            music,
            boxes,
            snd: grain_vm::Snd989::new(),
            streams: Streams::default(),
            spu,
            rng,
            sfx_option: 0x400,
            music_option: 0x2cc,
            sound_page: false,
            game_mode: 0,
            stats: AudioStats::default(),
            scene: scene::SceneAudio::default(),
            play_log: None,
            reverb: reverb::ReverbRequest::default(),
            reverb_boxes,
            reverb_log: None,
            underwater: false,
            checkpoint_track: None,
            started: false,
            music_started: false,
            snd_cmds: Vec::new(),
            stream_cmds: Vec::new(),
            snd_replies: Vec::new(),
            stream_replies: Vec::new(),
        }
    }

    /// One 60 Hz frame: the EE update, then 800 output samples appended to `out`.
    pub fn tick(&mut self, input: &FrameInput, out: &mut Vec<[i16; 2]>) {
        self.game_frame(input);
        self.render(SAMPLES_PER_FRAME, out);
    }

    /// One 60 Hz frame on the game's stream: [`AudioSystem::game_frame_with`], then 800 output samples appended
    /// to `out`.
    pub fn tick_with(&mut self, input: &FrameInput, frame: u32, rng: &mut crate::rng::Rng, owner_pos: &dyn Fn(u32) -> Option<[f32; 3]>, out: &mut Vec<[i16; 2]>) {
        self.game_frame_with(input, frame, rng, owner_pos);
        self.render(SAMPLES_PER_FRAME, out);
    }

    /// The EE part of a standalone frame ([`AudioSystem::game_frame_with`] on the private stream
    /// [`AudioSystem::rng`], the frame count as the counter, no moby owners).
    pub fn game_frame(&mut self, input: &FrameInput) {
        let mut rng = self.rng;
        let frame = self.stats.frames as u32 + 1;
        self.game_frame_with(input, frame, &mut rng, &|_| None);
        self.rng = rng;
    }

    /// The EE part of a frame: level start (`entry` 0x259c40: `music_start_track(0, 1, 0x400)`), the
    /// sound-instance updates, then `sound_update` with `music_Update` inside it. `frame` is the tick counter
    /// 0x15f5cc (phases the occlusion re-tests), `rng` the stream the sound code draws from (the game's one
    /// stream at the tick's sound step: the occlusion origin every frame = 3 draws, the 6-origin batch for a new
    /// occluded sound, the pitch-bend draw per play), `owner_pos` resolves a slot owner (a moby id) to its
    /// position (+0x10; None = deleted).
    pub fn game_frame_with(&mut self, input: &FrameInput, frame: u32, rng: &mut crate::rng::Rng, owner_pos: &dyn Fn(u32) -> Option<[f32; 3]>) {
        self.stats.frames += 1;
        let l = input.listener;
        if !self.started {
            self.started = true;
            self.level_music_start();
            // `GameStateUpdate`'s level-start `HeroTeleport(0x13e090, …)`: the env sample point near the spawn.
            self.hero_teleported(input.hero_pos);
        }
        self.scene_frame();
        self.emitters.update(&mut self.slots, &self.data.sounds, &l, rng);
        // The reverb boxes (sound instance class 3) test the hero position 0x13f3d0.
        for b in &mut self.reverb_boxes { b.update(input.hero_pos, &mut self.reverb); }
        self.sound_update_with(input, frame, rng, owner_pos);
    }

    /// `sound_update` (0x2a0638) alone, as the page menu's frame runs it (`SceneController` 0x28c990 after
    /// `MobyUpdateLoop`, and once more on the close): the replies, the reverb command, the slots and `music_Update`;
    /// no sound-instance update (0x2a19a8 runs from the level update 0x256810 only) and no reverb boxes.
    pub fn sound_update_with(&mut self, input: &FrameInput, frame: u32, rng: &mut crate::rng::Rng, owner_pos: &dyn Fn(u32) -> Option<[f32; 3]>) {
        let l = input.listener;
        for r in self.snd_replies.drain(..) { self.slots.apply_reply(r); }
        for r in self.stream_replies.drain(..) { self.music.reply(r); }
        let collision = self.data.collision.clone();
        let mut ctx = UpdateCtx {
            listener: &l,
            collision: collision.as_deref(),
            rng,
            frame,
            owner_pos,
            sfx_option: self.sfx_option,
            music_option: self.music_option,
            sound_page: self.sound_page,
            cutscene: self.scene.cutscene,
            game_mode: self.game_mode,
        };
        // The head of `sound_update`: the reverb command of this frame's changes, before the slots' commands.
        if let Some(c) = self.reverb.command() {
            if let Some(log) = self.reverb_log.as_mut() { log.push((self.stats.frames, c)); }
            self.snd_cmds.push(c);
        }
        let cmds = self.slots.update(&mut ctx);
        self.stats.plays += cmds.iter().filter(|c| matches!(c, SndCommand::Play { .. })).count() as u64;
        self.snd_cmds.extend(cmds);
        self.music_resume_countdown();
        // `music_Pause` / `music_Unpause` (the page menu, the vendor, a scene): the players' fade bit.
        self.music.paused = self.scene.music_paused;
        let more = self.music.tick(input.hero_pos, &mut self.boxes);
        self.stream_cmds.extend(more);
        self.stats.max_slots = self.stats.max_slots.max(self.slots.slots.iter().filter(|s| s.state != voices::state::FREE).count());
    }

    /// The IOP / SPU part: run the queued commands (their replies reach the EE next frame), then `n` samples
    /// with a 989snd tick every 200.
    pub fn render(&mut self, n: usize, out: &mut Vec<[i16; 2]>) {
        let bank = Arc::clone(&self.data.bank);
        for c in std::mem::take(&mut self.snd_cmds) {
            match c {
                SndCommand::MasterVolume { group, vol } => self.snd.set_master_volume(group as usize, vol, &mut self.spu),
                SndCommand::Play { slot, sound, vol, pan, pm, pb } => {
                    let h = self.snd.play_sound(&bank, &mut self.spu, sound, vol, pan, pm, pb);
                    self.snd_replies.push(SndReply { slot, handle: h as i32, play: true });
                }
                SndCommand::SetParams { slot, handle, mask, vol, pan, pm, pb } => {
                    let h = self.snd.set_params(&bank, &mut self.spu, handle, mask, vol, pan, pm, pb);
                    self.snd_replies.push(SndReply { slot, handle: h, play: false });
                }
                SndCommand::Stop { handle } => self.snd.stop_sound(&mut self.spu, handle),
                SndCommand::Poll { slot, handle } => self.snd_replies.push(SndReply { slot, handle: self.snd.still_playing(handle), play: false }),
                SndCommand::SetReverb { kind, depth, delay, feedback } => self.spu.reverb.set(kind, depth, delay, feedback),
                SndCommand::AutoReverb { depth, delta, .. } => self.spu.reverb.auto(depth, delta),
            }
        }
        for c in std::mem::take(&mut self.stream_cmds) {
            if let Some(r) = self.streams.execute(&c, &self.data.music, &mut self.snd.vm, &mut self.spu) { self.stream_replies.push(r); }
        }
        // A scene's music pause holds a stream these commands keyed too.
        if self.scene.music_paused { self.pause_music(); }
        out.reserve(n);
        for _ in 0..n {
            if self.stats.samples.is_multiple_of(SAMPLES_PER_IOP_TICK as u64) { self.snd.tick(&bank, &mut self.spu); }
            out.push(self.spu.mix());
            self.stats.samples += 1;
        }
        self.stats.max_voices = self.stats.max_voices.max(self.spu.voices.iter().filter(|v| v.active()).count());
    }

    /// `StartPssMovie` 0x2ad0c0 / `MovieModeUpdate` 0x2ad498 (a PSS movie, game mode 1): `sound_StopAllSounds`
    /// (989snd stops every sound, every sound slot is freed) and `music_Stop` (the music players and streams are
    /// reset; a pending music-box track becomes the current track). The port reaches that state by rebuilding the
    /// audio state from the level data, keeping the options, the counters, the IOP random stream and the play log;
    /// the sound instances' timers restart with it [L: the game keeps the emitters' own pvars]. The level start's
    /// automatic `music_start_track(0)` is not repeated: [`AudioSystem::movie_exit`] restarts the track.
    pub fn movie_stop(&mut self) {
        let track = if self.music.pending_track != -1 { self.music.pending_track as i16 } else { self.music.main.track };
        let mut fresh = AudioSystem::new(self.data.clone());
        fresh.sfx_option = self.sfx_option;
        fresh.music_option = self.music_option;
        fresh.stats = self.stats;
        fresh.rng = self.rng;
        fresh.snd.rng = self.snd.rng;
        fresh.snd.tick = self.snd.tick;
        fresh.snd.vm.mono = self.snd.vm.mono;
        fresh.play_log = self.play_log.take();
        fresh.reverb_log = self.reverb_log.take();
        fresh.underwater = self.underwater;
        fresh.checkpoint_track = self.checkpoint_track;
        fresh.spu.reverb.enabled = self.spu.reverb.enabled;
        // The reverb request and the boxes' pvars are the game's (kept); the effect is switched off (dirty bit 8).
        fresh.reverb = self.reverb;
        fresh.reverb.dirty |= reverb::dirty::OFF;
        fresh.reverb_boxes = std::mem::take(&mut self.reverb_boxes);
        fresh.started = true;
        fresh.music_started = true;
        fresh.music.main.track = track;
        *self = fresh;
    }

    /// The level start's `music_start_track(0x151708 = 0, 1, 0x400)`: `entry` 0x259c40 makes it before its main loop,
    /// so before any class update. The port's sound step comes after the moby loop, so a class's first-tick
    /// `MusicRequestTrack` (Orxon's Clank section, class 22: `(2, 4)`) makes it first ([`class_sounds`]): the request
    /// then finds the main player busy and becomes a transition, as in the game, instead of being overwritten by track 0.
    pub fn level_music_start(&mut self) {
        if self.music_started { return; }
        self.music_started = true;
        self.music.start_track(0, 1, 0x400, &mut self.stream_cmds);
    }

    /// `MovieExitToGameplay` 0x2ad2b8: `music_start_track(0x151708, 1, 0x400)`, the current track from the start, and
    /// the reverb resent (dirty bit 0x10).
    pub fn movie_exit(&mut self) {
        let track = self.music.main.track;
        self.music.start_track(track, 1, 0x400, &mut self.stream_cmds);
        self.reverb.dirty |= reverb::dirty::RESEND;
    }

    /// `HeroTeleport` 0x2368e0's `EnvNearestSamplePoint(hero)` 0x264e98, audio part: the env sample point nearest to
    /// `pos` within 8 units sets the reverb (when its +0x27 is set) and, while the music is idle (main player state 0,
    /// phase 0, no pending track), the track the music starts next (+0x28 when ≥ 0). Nothing without such a point.
    pub fn hero_teleported(&mut self, pos: [f32; 3]) {
        let Some(i) = reverb::nearest_env_point(&self.data.env_points, pos) else { return };
        let p = self.data.env_points[i];
        self.reverb.apply_env_point(&p);
        let m = &mut self.music;
        if p.music_track >= 0 && m.main.state == 0 && m.phase == 0 && m.pending_track == -1 { m.main.track = p.music_track as i16; }
    }

    /// The main loop's death reload (`entry` on the death flag 0x141401): `sound_StopAllSounds` and `music_Stop`
    /// ([`AudioSystem::movie_stop`]'s state), the reload (`FUN_00244110(0, 1)`; with a checkpoint record `0x29adc8` puts
    /// its reverb and its music track back), then `music_start_track(0x151708, 1, 0x400)`: that track from the start.
    pub fn death_reload(&mut self, checkpoint: bool) {
        self.movie_stop();
        if checkpoint {
            self.checkpoint_restored();
            // 0x29adc8: 0x151708 = the record's track 0x1bc304.
            if let Some(t) = self.checkpoint_track { self.music.main.track = t; }
        }
        let track = self.music.main.track;
        self.music.start_track(track, 1, 0x400, &mut self.stream_cmds);
    }

    /// The checkpoint record (`0x29ac10`): the reverb request saved, and the music track 0x1bc304 (the pending track
    /// 0x1516f2 when one is set, else the current 0x151708).
    pub fn checkpoint_saved(&mut self) {
        self.reverb.checkpoint();
        let m = &self.music;
        self.checkpoint_track = Some(if m.pending_track != -1 { m.pending_track as i16 } else { m.main.track });
    }

    /// The death reload's placement (`0x29adc8`): the saved reverb back (resent at the next `sound_update`).
    pub fn checkpoint_restored(&mut self) { self.reverb.restore(); }

    /// The movie sound as the SPU outputs it: 989snd's movie player (`snd_init_movie_sound(…, vol 0x400, pan 0,
    /// group 5, …)`) plays the left and right channels on two voices; each gets the stream-voice volume law
    /// (`MakeVolume(127, pan, 127, 0, 127, 0)`, group 5 = the sfx option 0x13e5ac, `>> 1`) with the left voice
    /// panned hard left and the right one hard right [L: the IOP movie player is not reversed], no envelope, then
    /// the SPU master volume and the 16-bit clamp as [`Spu::mix`]. `pcm` is 48 kHz stereo; appends to `out`.
    pub fn mix_movie(&self, pcm: &[[i16; 2]], out: &mut Vec<[i16; 2]>) {
        let mut vm = self.snd.vm.clone();
        vm.master[5] = self.sfx_option;
        vm.duck[5] = 0x10000;
        let regs = [vm.voice_registers(vm.make_volume(127, 270, 127, 0, 127, 0), 5), vm.voice_registers(vm.make_volume(127, 90, 127, 0, 127, 0), 5)];
        let level = |reg: u16| (reg << 1) as i16 as i32;
        let voice = |s: i16, reg: u16| ((s as i32 * level(reg)) >> 15) as i16 as i32;
        let master = |x: i32, reg: u16| ((x * (reg << 1) as i16 as i32) >> 15).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        out.extend(pcm.iter().map(|&[l, r]| {
            let (sl, sr) = (voice(l, regs[0][0]) + voice(r, regs[1][0]), voice(l, regs[0][1]) + voice(r, regs[1][1]));
            [master(sl, self.spu.master[0]), master(sr, self.spu.master[1])]
        }));
    }
}

/// A 16-bit stereo 48 kHz RIFF/WAVE file of `samples`.
pub fn wav_bytes(samples: &[[i16; 2]]) -> Vec<u8> {
    let data_len = (samples.len() * 4) as u32;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend(b"RIFF");
    w.extend((36 + data_len).to_le_bytes());
    w.extend(b"WAVEfmt ");
    w.extend(16u32.to_le_bytes());
    w.extend(1u16.to_le_bytes());
    w.extend(2u16.to_le_bytes());
    w.extend(OUTPUT_RATE.to_le_bytes());
    w.extend((OUTPUT_RATE * 4).to_le_bytes());
    w.extend(4u16.to_le_bytes());
    w.extend(16u16.to_le_bytes());
    w.extend(b"data");
    w.extend(data_len.to_le_bytes());
    for s in samples {
        w.extend(s[0].to_le_bytes());
        w.extend(s[1].to_le_bytes());
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauss_rows_sum_to_unity() {
        for i in 0..256 {
            let s = GAUSS[0xff - i] as i32 + GAUSS[0x1ff - i] as i32 + GAUSS[0x100 + i] as i32 + GAUSS[i] as i32;
            assert!((0x7f7f..=0x7f81).contains(&s), "row {i}: {s:#x}");
        }
        assert_eq!(GAUSS[511], 0x59b3);
        // A constant signal stays (almost) constant through the filter.
        assert_eq!(gauss_interpolate(0, [10000; 4]), gauss_interpolate(255, [10000; 4]));
    }

    #[test]
    fn adsr_phases() {
        let mut a = Adsr { adsr1: 0x80ff, adsr2: 0x9fc0, ..Default::default() };
        a.attack();
        // Exponential attack, step +7 at shift 0: 7 << 11 per sample up to 0x6000, then every 4th sample:
        // 0x3800, 0x7000, (4 samples) 0x7fff → decay; sustain level 15 (0x8000) is met at once → sustain.
        a.run();
        a.run();
        assert_eq!(a.level, 0x7000);
        for _ in 0..4 { a.run(); }
        assert_eq!((a.level, a.phase), (0x7fff, Phase::Decay));
        a.run();
        assert_eq!(a.phase, Phase::Sustain);
        a.release();
        // Release shift 0, linear −8 << 11 per sample.
        a.run();
        assert_eq!(a.level, 0x7fff - 0x4000);
        a.run();
        a.run();
        assert_eq!(a.phase, Phase::Stopped);
        // Slow linear attack: shift 20 → one step (+7 << 0... = 7) every 2^9 samples.
        let mut b = Adsr { adsr1: 20 << 10, adsr2: 0, ..Default::default() };
        b.attack();
        for _ in 0..512 { b.run(); }
        assert_eq!(b.level, 7);
    }

    /// Regression (the R.Y.N.O. crash, 2026-09-28): many loud voices at once — the mix clamps, it does not overflow
    /// (the 32-bit master product panicked in the dev build past about two full-scale voices).
    #[test]
    fn loud_mix_clamps_without_overflow() {
        assert_eq!(master_out(48 * 32767, 0x3fff), i16::MAX);
        assert_eq!(master_out(-48 * 32768, 0x3fff), i16::MIN);
        assert_eq!(master_out(1000, 0x3fff), ((1000 * 0x7ffe) >> 15) as i16, "the level below the clamp is unchanged");
        let mut ram = Vec::new();
        ram.extend(frame(6, 0x77));
        ram.extend(frame(2, 0x77));
        ram.extend(frame(3, 0x77));
        let ram: Arc<[u8]> = Arc::from(ram);
        let mut spu = Spu::new(ram.clone());
        for v in spu.voices.iter_mut().take(12) {
            v.key_on(VoiceData::Ram(ram.clone()), 0, 0x1000, 0x00ff, 0x1fc0);
            v.vol = [0x3fff, 0x3fff];
        }
        let out: Vec<[i16; 2]> = (0..400).map(|_| spu.mix()).collect();
        assert!(out.iter().any(|s| s[0] == i16::MAX || s[0] == i16::MIN), "twelve loud voices clip");
    }

    fn frame(flags: u8, v: u8) -> [u8; 16] {
        let mut f = [v; 16];
        f[0] = 0;
        f[1] = flags;
        f
    }

    #[test]
    fn voice_plays_one_shots_and_loops() {
        // One-shot: zero frame, a loud frame, end. At pitch 0x1000 (one sample per output sample) the voice
        // stops after 3 · 28 samples.
        let mut ram = Vec::new();
        ram.extend(frame(0, 0));
        ram.extend(frame(0, 0x11));
        ram.extend(frame(1, 0x11));
        ram.extend(frame(7, 0));
        let loop_at = ram.len();
        ram.extend(frame(6, 0));
        ram.extend(frame(2, 0x33));
        ram.extend(frame(3, 0x33));
        let ram: Arc<[u8]> = Arc::from(ram);
        let mut v = SpuVoice::default();
        v.key_on(VoiceData::Ram(ram.clone()), 0, 0x1000, 0x00ff, 0x1fc0);
        v.vol = [0x3fff, 0x3fff];
        let out: Vec<i32> = (0..120).map(|_| v.run().0).collect();
        assert!(out[..28].iter().all(|&x| x == 0));
        assert!(out[40..80].iter().any(|&x| x != 0));
        assert!(!v.active());
        assert!(out[85..].iter().all(|&x| x == 0));
        // Loop: keeps playing past its end, jumping back to the loop-start frame.
        let mut v = SpuVoice::default();
        v.key_on(VoiceData::Ram(ram), loop_at, 0x1000, 0x00ff, 0x1fc0);
        v.vol = [0x3fff, 0x3fff];
        let out: Vec<i32> = (0..28 * 10).map(|_| v.run().0).collect();
        assert!(v.active());
        assert!(out[28 * 7..].iter().any(|&x| x != 0));
        // Streams: two parts, the second repeating.
        let a = StreamPart { data: Arc::from(&[frame(0, 0x11), frame(1, 0x11)].concat()[..]), looped: false };
        let b = StreamPart { data: Arc::from(&[frame(0, 0x22), frame(1, 0x22)].concat()[..]), looped: true };
        let mut v = SpuVoice::default();
        v.key_on(VoiceData::Stream { cur: a, queue: VecDeque::from([b]) }, 0, 0x1000, 0x00ff, 0x1fc0);
        assert_eq!(v.stream_remaining(), 56);
        for _ in 0..56 * 5 { v.run(); }
        assert!(v.active());
        assert!(v.stream_remaining() <= 56);
    }

    /// Every level's gadget classes (the hand items) get their class sound defs from their blobs with the parked
    /// remap: the wrench on every level, the Swingshot (0xd0) wherever it is in the gadget table, every remapped id
    /// a sound of the bank (skipped without `extracted/`).
    #[test]
    fn gadget_class_sounds_on_every_level() {
        use rc_formats::sound_bank as sb;
        let mut seen = 0;
        for level in 0..19 {
            let root = rc_formats::test_data::root().join(format!("levels/{level:02}"));
            let Ok(bank) = std::fs::read(root.join("sound_bank.bin")) else { continue };
            let rd = |n: &str| std::fs::read(root.join(n)).unwrap();
            let idx = rd("core_index.bin");
            let data = rc_formats::test_data::core_data(level).unwrap();
            let core = rc_formats::level::parse_level_core(&idx, data.len()).unwrap();
            let bank = sb::parse_bank(&bank).unwrap();
            let mut sounds = sb::parse_level_sounds(&idx, &core, &data).unwrap();
            let gadgets = rc_formats::gadget::parse_gadget_classes(&core, &data).unwrap();
            let before = sounds.def(sb::SoundOwner::Class(71), 0).is_some();
            let blobs: Vec<(i32, &[u8])> = gadgets.iter().map(|g| (g.moby.o_class, g.blob.as_slice())).collect();
            sb::apply_gadget_defs(&mut sounds, &blobs).unwrap();
            assert!(!before, "level {level}: the wrench had defs before");
            let n_sounds = bank.sounds.len();
            let mut report = Vec::new();
            for g in &gadgets {
                let Some(c) = sounds.classes.iter().find(|c| c.o_class == g.moby.o_class) else { continue };
                for (j, d) in c.defs.iter().enumerate().take(c.bank_ids.len().min(15)) {
                    assert!((d.index as usize) < n_sounds, "level {level} class {} def {j}: id {} of {n_sounds}", c.o_class, d.index);
                }
                report.push((c.o_class, c.defs.len()));
            }
            assert!(sounds.def(sb::SoundOwner::Class(71), 0).is_some(), "level {level}: no wrench sound");
            if gadgets.iter().any(|g| g.moby.o_class == 0xd0) { assert!(sounds.def(sb::SoundOwner::Class(0xd0), 2).is_some(), "level {level}: Swingshot"); }
            eprintln!("level {level}: gadget classes (o_class, defs) {report:?}");
            seen += 1;
        }
        eprintln!("{seen} levels checked");
    }

    /// Novalis from `extracted/` (skipped when absent): 10 s at the spawn, listener a few units behind
    /// Ratchet looking along +y. Music must start at once, the looped emitter (instance 9) must play, and two
    /// runs must be sample-identical.
    #[test]
    fn novalis_start_renders_music_and_ambience() {
        let _lock = scene::TEST_INBOX.lock().unwrap_or_else(|e| e.into_inner());
        let root = rc_formats::test_data::root().join("levels/01");
        let Ok(bank) = std::fs::read(root.join("sound_bank.bin")) else { eprintln!("skipped: no extracted/"); return };
        let rd = |n: &str| std::fs::read(root.join(n)).unwrap();
        let idx = rd("core_index.bin");
        let data = rc_formats::test_data::core_data(1).unwrap();
        let core = rc_formats::level::parse_level_core(&idx, data.len()).unwrap();
        let gameplay = rc_formats::test_data::gameplay(1).unwrap();
        let music: [Option<Vec<u8>>; 15] = std::array::from_fn(|k| std::fs::read(root.join(format!("music/{k:03}.bin"))).ok());
        let coll = rc_formats::collision::parse_collision(&core, &data).ok();
        let audio = LevelAudio::from_parts(&bank, &idx, &core, &data, &gameplay, &rd("level_header.bin"), &music, coll).unwrap();
        assert_eq!(audio.music[0].as_ref().unwrap().name, "L01_Enemy_Start");
        let listener = Listener { pos: [162.5, 130.0, 63.0], rows: [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], ..Default::default() };
        let input = FrameInput { listener, hero_pos: [162.5, 136.4, 60.5] };
        let run_with = |frames: usize, music_option: i32| {
            let mut sys = AudioSystem::new(audio.clone());
            sys.music_option = music_option;
            let mut out = Vec::new();
            for _ in 0..frames { sys.tick(&input, &mut out); }
            (out, sys)
        };
        let run = |frames: usize| run_with(frames, 0x2cc);
        let (a, sys) = run(600);
        let (b, _) = run(600);
        // Effects alone (music option 0): the looped emitter (instance 9, bank sound 2) from the first frames.
        let (fx, _) = run_with(600, 0);
        assert_eq!(a.len(), 480_000);
        assert!(a == b, "not deterministic");
        let rms = |s: &[[i16; 2]]| (s.iter().map(|x| (x[0] as f64).powi(2) + (x[1] as f64).powi(2)).sum::<f64>() / (2 * s.len()).max(1) as f64).sqrt();
        let per_second: Vec<f64> = a.chunks(48_000).map(rms).collect();
        eprintln!("novalis 10 s: RMS per second {per_second:.0?}; stats {:?}; voices active {}; 989 handlers {}; unimplemented {:?}",
                  sys.stats, sys.spu.voices.iter().filter(|v| v.active()).count(), sys.snd.playing(), sys.snd.unimplemented);
        let fx_second: Vec<f64> = fx.chunks(48_000).map(rms).collect();
        eprintln!("novalis 10 s, effects only: RMS per second {fx_second:.1?}");
        assert!(fx_second.iter().all(|&r| r > 1.0), "no ambience");
        assert!(per_second.iter().all(|&r| r > 100.0), "silent second");
        // Music from the start: the first 50 ms are not silent (the VAG's leading zero frame is 28 samples).
        assert!(rms(&a[..2400]) > 1.0);
        // The emitters at the spawn started (instance 9's loop at least).
        assert!(sys.stats.plays >= 1, "no ambience played");
    }

    /// Orxon's first visit: the Clank section (class 22) hands the hero to Clank on the first tick and requests
    /// `MusicRequestTrack(2, 4)` from the moby loop, before the tick's sound step. The level start's track 0 has
    /// already started (`entry` 0x259c40 starts it before its main loop), so the request becomes a transition: the
    /// stinger, then Clank's track 2 ("Clank's Second Foray"), not Ratchet's track 0.
    #[test]
    fn orxon_clank_first_tick_request_plays_clanks_track() {
        let _lock = scene::TEST_INBOX.lock().unwrap_or_else(|e| e.into_inner());
        let root = rc_formats::test_data::root().join("levels/10");
        let Ok(bank) = std::fs::read(root.join("sound_bank.bin")) else { eprintln!("skipped: no extracted/"); return };
        let rd = |n: &str| std::fs::read(root.join(n)).unwrap();
        let idx = rd("core_index.bin");
        let data = rc_formats::test_data::core_data(10).unwrap();
        let core = rc_formats::level::parse_level_core(&idx, data.len()).unwrap();
        let gameplay = rc_formats::test_data::gameplay(10).unwrap();
        let music: [Option<Vec<u8>>; 15] = std::array::from_fn(|k| std::fs::read(root.join(format!("music/{k:03}.bin"))).ok());
        let coll = rc_formats::collision::parse_collision(&core, &data).ok();
        let audio = LevelAudio::from_parts(&bank, &idx, &core, &data, &gameplay, &rd("level_header.bin"), &music, coll).unwrap();
        let input = FrameInput { listener: Listener::default(), hero_pos: [0.0; 3] };
        // The old order (the request before the level start): the idle player takes track 2, then the start
        // overwrites it with track 0 — Ratchet's music for Clank.
        let mut old = AudioSystem::new(audio.clone());
        old.music.request(2, 4);
        let mut out = Vec::new();
        for _ in 0..600 { old.tick(&input, &mut out); }
        assert_eq!(old.music.main.track, 0);
        let mut sys = AudioSystem::new(audio);
        // The first tick: the class request (through the class sound sink's path), then the sound step.
        sys.level_music_start();
        sys.music.request(2, 4);
        let mut out = Vec::new();
        let mut tracks = Vec::new();
        for _ in 0..1200 {
            sys.tick(&input, &mut out);
            tracks.push(sys.music.main.track);
        }
        assert_eq!(sys.music.main.track, 2, "main track after 20 s (tracks seen {:?})", { let mut t = tracks.clone(); t.dedup(); t });
        assert!(sys.music.main.state != 0, "the main player plays");
    }

    /// Blarg's escape 1108 at the end of the tube ride: `MusicRequestTrack(6, 9)` while the level's track plays (the
    /// forum's "self-destruct music never starts"): the stinger, then track 6 — also when a scene paused the music at
    /// any point of an earlier transition (the shuttle's (4, 8)).
    #[test]
    fn blarg_escape_request_plays_track_6() {
        let _lock = scene::TEST_INBOX.lock().unwrap_or_else(|e| e.into_inner());
        let root = rc_formats::test_data::root().join("levels/06");
        let Ok(bank) = std::fs::read(root.join("sound_bank.bin")) else { eprintln!("skipped: no extracted/"); return };
        let rd = |n: &str| std::fs::read(root.join(n)).unwrap();
        let idx = rd("core_index.bin");
        let data = rc_formats::test_data::core_data(6).unwrap();
        let core = rc_formats::level::parse_level_core(&idx, data.len()).unwrap();
        let gameplay = rc_formats::test_data::gameplay(6).unwrap();
        let music: [Option<Vec<u8>>; 15] = std::array::from_fn(|k| std::fs::read(root.join(format!("music/{k:03}.bin"))).ok());
        let coll = rc_formats::collision::parse_collision(&core, &data).ok();
        let audio = LevelAudio::from_parts(&bank, &idx, &core, &data, &gameplay, &rd("level_header.bin"), &music, coll).unwrap();
        let input = FrameInput { listener: Listener::default(), hero_pos: [0.0; 3] };
        let level_audio = audio.clone();
        let mut sys = AudioSystem::new(audio);
        // A scene pausing the music at points of an earlier transition (the shuttle's (4, 8)), then the escape's request.
        for pause_at in [0usize, 1, 3, 30, 91, 95, 150, 280, 400] {
            let mut t = AudioSystem::new(level_audio.clone());
            t.level_music_start();
            let mut out = Vec::new();
            for _ in 0..600 { t.tick(&input, &mut out); }
            t.music.request(4, 8);
            for _ in 0..pause_at { t.tick(&input, &mut out); }
            t.scene_command(scene::SceneAudioCmd::PauseMusic);
            for _ in 0..300 { t.tick(&input, &mut out); }
            t.scene_command(scene::SceneAudioCmd::ResumeMusic { after: 30 });
            for _ in 0..900 { t.tick(&input, &mut out); }
            let before = (t.music.main.track, t.music.main.state, t.music.phase, t.music.pending_track);
            t.music.request(6, 9);
            for _ in 0..1800 { t.tick(&input, &mut out); }
            assert_eq!((t.music.main.track, t.music.phase, t.music.pending_track), (6, 0, -1), "a pause {pause_at} ticks into the (4, 8) transition (before the escape {before:?})");
            assert!(t.music.main.state != 0);
        }
        sys.level_music_start();
        let mut out = Vec::new();
        for _ in 0..600 { sys.tick(&input, &mut out); }
        sys.music.request(6, 9);
        let mut seen = Vec::new();
        for f in 0..1800 {
            sys.tick(&input, &mut out);
            let m = &sys.music;
            let k = (m.main.track, m.main.state, m.transition.state, m.phase, m.pending_track);
            if seen.last().map(|x: &(i32, (i16, u16, u16, i16, i8))| x.1) != Some(k) { seen.push((f, k)); }
        }
        assert_eq!(sys.music.main.track, 6, "{seen:?}");
        assert!(sys.music.main.state != 0, "{seen:?}");
    }

    /// The Start → Loop hand-over is sample-exact: a stream voice playing part A then part B outputs the same
    /// samples as one RAM voice playing A's frames (end flag cleared) followed by B's, at the music pitch.
    #[test]
    fn stream_queue_is_seamless() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        for k in 0..6u8 {
            let mut f = [0u8; 16];
            f[0] = (k % 4) << 4 | 3;
            for (i, x) in f[2..].iter_mut().enumerate() { *x = k.wrapping_mul(37).wrapping_add((i as u8).wrapping_mul(11)) ^ 0x5a; }
            let mut g = f;
            g[0] = ((k + 1) % 5) << 4 | 2;
            a.extend(f);
            b.extend(g);
        }
        a[5 * 16 + 1] = 1; // end frames
        b[5 * 16 + 1] = 1;
        let mut joined = a.clone();
        joined[5 * 16 + 1] = 0;
        joined.extend(&b);
        let pitch = rc_formats::vag::rate_to_pitch(44100);
        let mut s = SpuVoice::default();
        let (pa, pb) = (StreamPart { data: Arc::from(&a[..]), looped: false }, StreamPart { data: Arc::from(&b[..]), looped: false });
        s.key_on(VoiceData::Stream { cur: pa, queue: VecDeque::from([pb]) }, 0, pitch, 0x00ff, 0x1fc0);
        let mut r = SpuVoice::default();
        r.key_on(VoiceData::Ram(Arc::from(joined)), 0, pitch, 0x00ff, 0x1fc0);
        for v in [&mut s, &mut r] { v.vol = [0x3fff, 0x3fff]; }
        let n = 12 * 28 * 4096 / pitch as usize - 4;
        let xs: Vec<(i32, i32)> = (0..n).map(|_| s.run()).collect();
        let ys: Vec<(i32, i32)> = (0..n).map(|_| r.run()).collect();
        assert_eq!(xs, ys);
        assert!(xs.iter().filter(|x| x.0 != 0).count() > n / 2);
    }

    #[test]
    fn wav_header() {
        let w = wav_bytes(&[[1, -1], [2, -2]]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len(), 44 + 8);
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 48000);
        assert_eq!(i16::from_le_bytes([w[46], w[47]]), -1);
    }
}
