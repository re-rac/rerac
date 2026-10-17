# Audio: banks, VAG/ADPCM, the sound API, music, Novalis start

Addresses are **level01.elf** unless marked "boot" (SCUS_971.99). The EE-side 989snd RPC stub lives only in
the boot ELF (0x12da28–0x12f178) and the levels call it directly. Confidence tags: **V** = checked against
disc data or code here, **H** = read from the decompile, **M** = inferred, **U** = unknown.
Reference implementation of the same Sony library: OpenGOAL `game/sound/989snd` + `game/sound/common` (ISC).

## 1. Stack

- **IOP modules** (global `irx` lump, table of 24 `{u32 off, u32 size}` at 0, V): 14 sio2man v2.05, 15 mcman,
  16 mcserv, 17 Dbc_Manager, 18 sio2d, 19 ds2u, 20 IOP_stash_daemon, **21 Sound_Device_Library (libsd) v3.03**,
  **22 989snd_Library v2.09** ("989snd (c)2000, 2001 SCEA", banner "NO MIDI VERSION! No MIDI, AME, or basic
  VAG sounds"). Entries 0–13 (not 3, 9) are WAD-compressed 0xD0000-byte non-IRX blobs (images?). The IRX is
  not a custom driver: a stock 989snd built for SFX blocks and VAG streams only (V).
- **IOPRP243.IMG** (disc root, sector 966, 264,449 bytes, V) only holds kernel modules: LOADCORE, SIFCMD,
  SIFMAN, THREADMAN, IOMAN, MODLOAD, FILEIO, CDVDMAN, CDVDFSV, LOADFILE, TIMEMANI, ROMDRV, EESYNC, SYSCLIB,
  STDIO. No sound code.
- **EE→IOP** (boot, H): commands are batched by `snd_SendIOPCommandNoWait` 0x12e6e0 and flushed each frame
  by `snd_FlushSoundCommands` 0x12dc80. Command ids: 0x09 SetMasterVolume(group, vol), 0x10 AutoReverb,
  0x11 PlaySoundVolPanPMPB(bank, sound, vol, pan, pm, pb, cb, data), 0x15 StopSound(h),
  0x19 SoundIsStillPlaying_CB, 0x21 SetSoundParams_CB(h, mask, vol, pan, pm, pb, cb, data),
  0x2a InitVAGStreamingEx, 0x2c PlayVAGStreamByLocEx, 0x4e SetGroupVoiceRange(group, first, last),
  0x50 SetReverbEx(core, type, depth, delay, feedback). Banks: `snd_BankLoadFromEE_CB` 0x12e088.
  The port does not need the RPC layer. It needs what the IOP does: the SFX-block player and the SPU2 voices.

## 2. Formats

### 2.1 `sound_bank` lump (level data WAD +0x08, and global +0x14e0) — V
989snd bank file: `u32 type=3, u32 nchunks=2, {u32 off, u32 size}[2]`. Chunk 0 = SFX block, chunk 1 = raw SPU
ADPCM (no headers). Tone sample offsets are relative to chunk 1. Novalis: block (0x18, 0x6bbc), samples
(0x6bd4, 0x19d3b0). Global: block (0x18, 0x140c), samples (0x1424, 0x240e0).

`SBlk` header (block-relative, little-endian):

| off | type | field | Novalis / global |
|---|---|---|---|
| 0x00 | char[4] | `SBlk` | |
| 0x04 | u32 | version | 1 / 1 (grains are the 0x28-byte v1 form) |
| 0x08 | u32 | flags | 4 / 4 (0x100 names, 0x200 userdata: both absent) |
| 0x0c | u32 | bank id | "DAW\0" / 0 |
| 0x10 | s8 + pad[5] | bank num | 0 |
| 0x16 | s16 ×3 | sounds, grains, vags | 290, 601, 261 / 10, 124, 18 |
| 0x1c | u32 | first sound | 0x3c / 0x34 (**header length varies**; use this, not a fixed size) |
| 0x20 | u32 | first grain | 0xdd4 / 0xac |
| 0x24 | u32 ×4 | vags-in-SR, vag data size, SRAM alloc size, next block | 0, 0x19d3b0, 0x19d3b0, 0 |

Sound record (12 bytes at first_sound): `s8 vol, s8 vol_group, s16 pan, s8 n_grains, s8 instance_limit,
u16 flags (1 = loop), u32 first_grain` (byte offset from first_grain). Novalis: vol 127 on 284 sounds; groups
0 (274) and 4 (16); 30 looped; no instance limits.
Grain v1 (0x28 bytes): `u32 type, s32 delay (240 Hz ticks), 32 bytes data`. Type 1/9 = **tone**:
`s8 priority, s8 vol, s8 center_note, s8 center_fine, s16 pan, s8 map_lo, s8 map_hi, s8 pb_lo, s8 pb_hi,
u16 adsr1, u16 adsr2, u16 flags (1 = to reverb), u32 sample_offset, u32 reserved`. Negative vol/pan select
registers (−1..−4 per sound, −5 random, ≤−6 global). Other types are the script VM: 4 LFO, 20–43
(loop/stop/random play/delay/pitch bend/registers/markers/key-off). Novalis uses 382 tones and 21 other grain
types. Tone priorities: 90, 100, 0. Level tones route to reverb (flags 1), global tones are dry.

### 2.2 Sound definitions and the remap (core index +0x70) — V/H
Game-side record `SoundDef` (0x20), used for both level sounds and per-class sounds (class +0x0d count,
class +0x28 pointer):

| off | type | field |
|---|---|---|
| 0x00 / 0x04 | f32 | near, far (distance) |
| 0x08 / 0x0c | s32 | volume at far, volume at near (0x400 = unity; up to 0x800 seen) |
| 0x10 / 0x14 | s32 | pitch-bend range [lo, hi], randomised per play |
| 0x18 | u8 | loop (must match the play flag 4, else the play is refused) |
| 0x19 | u8 | bit0 squared falloff · bit1 no occlusion · bit2 not halved above water · bit3 no underwater pitch drop |
| 0x1a | u16 | index → **bank sound id** after load |
| 0x1c | u32 | bank handle (written at load, 0x15ed5c) |

Remap block (index-relative): `s16 defs_off, s16 defs_count, s16 map_off, s16 map_count`, then one
`{s16 off, s16 count}` per moby class **in core-index class order**, each pointing to `count × {u16 bank_id,
u16 0}`. The loader (0x258128, lines 260–350) copies the level defs to a 64-byte-aligned heap array
(`0x15f5f4`, count `0x15f5f0`), replaces each def's +0x1a with `map[idx].bank_id` (0xffff and a warning if
idx ≥ map_count), then for each loaded class writes `entry[j].bank_id` into `class_defs[j].+0x1a` (warning
0x209800 when counts differ). Classes without a blob (gadgets) park up to 15 ids at 0x1b02e0 + 0x20·k.
Novalis: 54 level defs over 22 map entries (bank 0–21), 212 class entries covering bank 22–289, 68 classes
with defs and all counts match, 21 ids are 0xffff (Ratchet's unused slots). Level def roles: 0–1 moby-attached
(0x2a1770, `idx < 0x15f574 = 2`), 2–3 ambient, **4–19 footsteps** (0x2a1898:
`idx = tbl_0x1bdca0[level] + surface·4 + foot·2 + variant + 2`, Novalis tbl = 2), 20 underwater loop, 21–53
sound-instance emitters. `moby_sound_remap_offset` (+0xb4, 0x9c00) is **not read** by the RAC1 loader.

### 2.3 VAG files (music, scene speech, help/vendor/space audio) — V
Header 0x30 bytes, **big-endian**: `"VAGp"`, version 0x20, pad, data size, sample rate (44100 music, 44056
speech), 12 bytes 0, char[16] name (`L01_Enemy_Loop`). Body = mono SPU ADPCM. The last two frames have
flags 1 (end) then 7 (pad). No stereo interleave: decoding 001.bin as mono shows no discontinuity at any
0x100–0x8000 boundary. Size = 0x30 + data size (toc.cpp).

### 2.4 ADPCM decoder (SPU2) — V on the data, H on the rounding
16-byte frame: `b0 = shift | filter<<4`, `b1 = flags`, 14 data bytes = 28 nibbles, **low nibble first**.
```
K0 = [0, 60, 115,  98, 122]      K1 = [0, 0, -52, -55, -60]     // /64
for each nibble n:  s = (i16)(n << 12) >> shift
                    s += (K0[f]*h1 + K1[f]*h2 + 32) >> 6       // PCSX2/DuckStation form
                    s = clamp(s, -32768, 32767); h2 = h1; h1 = s
```
OpenGOAL shifts the two terms separately without +32. The two forms differ by ≤1 LSB, and nothing on the
disc can decide between them. Use the PCSX2 form. Flags: bit0 end, bit1 repeat, bit2 loop start. A voice
latches LSA at a bit2 frame. After decoding a bit0 frame it jumps to LSA if bit1, else stops (ADSR off). On
the disc: shift 0–12 and filter 0–4 only, so the invalid cases never occur. History starts at 0 on key-on.
There is no skip: every sample starts with an all-zero frame, which decodes to 28 zeros (261/261). Novalis bank: 230
one-shots (`…, 1, 7`; 229/230 are followed directly by the next sample), 31 loops (`6, 2…, 3`).
**Sample rate:** SPU pitch 0x1000 = 48 kHz. SFX tones start at note 60, fine 0. The pitch is
`sceSdNote2Pitch(|center_note|, center_fine, 60 + bend, …)` (OpenGOAL `util.cpp`, exact table). When
`center_note ≥ 0` (PS1-style) it is then ×44100/48000. So center −74/66 ≈ 22.0 kHz and +72/0 = 22.05 kHz.
Pitch bend uses the tone's pb_lo/pb_hi semitones over ±0x8000. Pitch mod is in 1/128 semitone. Voices use
the SPU2 4-tap Gaussian interpolation and ADSR from adsr1/adsr2 (OpenGOAL `voice.cpp` / `envelope.cpp`).

## 3. The EE sound API (game layer)

### 3.1 Slots — H
30 logical sounds at **0x13e5c0 + i·0x70** (boot BSS): +0 989snd handle (−1 pending), +4 state (0 free,
7 start pending, 1 playing, 4 release requested, 6 stopping), +5 play flags, +8 def*, +0xc bank id,
+0xe class-sound index (0xffff), +0x10 volume scale (0x400 = 1), +0x14 pitch bend, +0x18 owner moby,
+0x1c owner sound instance, +0x20 position, +0x30 local offset, +0x40 occlusion ring index,
+0x44 u8[36] occlusion ring. **Allocation** `fun_0022d7f0` (0x2a13a0): slots 0–25 first-free. Slots 0–29
are open to the hero (0x1413d0), 0x1403e0 and class 0x472 (1138). Refused (−1): loop flag ≠ def loop, or the
initial distance volume < 0x20. Pitch bend = `lo + rand() % (hi − lo)`, where `rand` is 0x26c930:
`(rand() >> 16 & 0x7fff) % n`.
Play flags: 0x01 2-D (no pan or doppler), 0x04 loop, 0x08 don't follow the owner, 0x10 fixed volume (no
distance), 0x20 no doppler, 0x40 follow the owner at a rotated local offset (moby +0xc0 matrix).
No owner and no position: flags |= 0x11, and the sound sits at the listener.

| fn | purpose |
|---|---|
| 0x2a1618 `fun_0022da68(i, flags, moby)` | class sound i of the moby (triggers, most gameplay) |
| 0x2a16c0 `(i, flags, moby, o_class)` | class sound looked up by o_class |
| 0x2a1770 `(i, flags, moby)` / 0x2a1808 `(i, flags, sndinst, vol)` | level def at a moby / at sound instance +0x40 |
| 0x2a1898 `(surface, foot, variant, flags, moby)` | footsteps |
| 0x2a1968 / 0x2a1988 | set slot volume / pitch bend |
| 0x2a12f0 `(moby, slot)` alive? · 0x2a1348 release · 0x2a1b08 stop all | |

**Animation.** Trigger words (lo16 = class sound, hi16 = time) call `fun_0022da68(id, 0, moby)` from
`MobyAnimAdvance` (0x265260). Loop sound: moby +0x7c = class sound, +0x7d = slot. Boot 0x20c940 (called
from the advance) plays with flag 4 when +0x7d = 0xff. It releases the slot when the class index changed,
and forgets the slot when another owner took it. A loop out of range is refused, so it is retried on each
refresh (H).

### 3.2 Per-frame update `sound_update` 0x2a0638 — H
Order: reverb commands → the listener → master volumes → per slot, mark → per slot, occlusion → send →
`music_Update` → flush. The **listener is the camera** (pos 0x167240, rotation 0x167450), not the hero.
- Listener velocity: 4-entry ring of camera positions. It averages the last ≤3 deltas and stops at a delta
  ≥ 60·`0x15ed6c` (teleport).
- Owner tracking (unless flag 8): pos = moby +0x10 with **z + 1.0**, or offset·rot + pos for flag 0x40.
  Velocity = new − old. A deleted owner (state −2/−3) is dropped, and its loops stop.
- **Distance volume** 0x2a02e0: `d = |pos − cam|`; d ≤ near → vnear; d ≥ far → vfar; else
  `vfar + trunc((far−d)·f32(vnear−vfar)/(far−near))`, or with squared terms when def bit0.
  `vol = (atten · slot.vol) >> 10` (arith. rounding toward 0). Below 0x20 both: loops stop, one-shots continue.
- **Pan** 0x2a0418: `v = Rᵀ(pos − cam)`, `k = clamp(|v.xy| − 1, 0, 1)`,
  `pan = trunc(−FastArcTan(v.x, v.y)·180·k·0.31830987)` degrees. It fades to centre inside 1 unit.
- **Doppler**: `dot = normalize(cam − pos)·(src_vel − listener_vel)`,
  `pm = (trunc(dot·300) · 0x5f4) / 0x2e5` (boot 0x12f1a0). A source approaching the listener raises the pitch.
- **Underwater** (`0x167494` ≠ 0): pm −= 0x5f4 (≈ −1 octave) unless def bit3. Volume /2 when
  pos.z > water height `0x13f640`, unless def bit2. Music group ×3/5.
- **Occlusion** (def bit1 clear): a 36-sample ring of `CollLine_Fix(origin, cam + 0.75·(pos − cam) (then
  0x1f9c90 64.0), mask 0x82, ignore owner)`. Origins are jittered listener points (0x2a0190: random
  0.5–6.0 around the camera, pulled to 0.75 of any hit). A new sound fills all 36 from 6 points. A running
  sound re-tests one ring entry every 2nd frame (loops) or every 4th frame (one-shots), phased by slot
  index. n occluded:
  n ≥ 36 → vol 0; n > 18 → `vol = (36 − n)·vol / 18`.
- Send: state 7 → `PlaySoundVolPanPMPB(def.bank, def.id, vol, pan, pm, pb)`, → state 1. Else
  `SetSoundParams(h, mask, …)` with mask 1 vol | 0x10 pb (if pb ≠ 0) | 6 pan | 8 pm. Release: `StopSound`,
  then state 6, and the still-playing callback frees the slot. Paused game: only liveness is polled.
- **Master volume groups** (init 0x2a04b8, options sfx `0x15edf0`, music `0x15edec`): 0 = sfx·8/10,
  1 = music, 2 = sfx·8/10, 3 = sfx·7/10, 4 = sfx·7/10, 5 = sfx. Cutscene (`0x15f5c4 == 2`): group 1 = 0,
  groups 0 and 3 halved. Voice ranges: groups 1, 2 and 4 → SPU voices 0x18–0x2f. Mono flag from
  `0x15ede8`. Streams use group 1 (music) and 2 (dialogue). Bank sounds use groups 0 and 4.
- **989snd side** (OpenGOAL, same library, M for RAC1): `v = clamp((sfx.vol·vol) >> 10, 0, 127)`; per tone
  `MakeVolume` = `127·258·v/127·tone_vol/127` panned by the 181-entry constant-power table (0° centre,
  +90° right); group scale `(x·master/0x400)² / 0x7ffe`, then >> 1 into the SPU voice. The grain VM ticks at
  240 Hz. SPU2 has 48 voices. The **voice-steal policy is not reversed** (U): proposal below.
- **Reverb**: env sample point +0x20..0x27 (depth, type, delay, feedback, enable) → `SetReverbEx(2, …)`.
  Sound-instance class 3 boxes (0x319f18) blend depth by box x and switch off on exit through −x.
  libsd types: 3 STUDIO_B, 4 STUDIO_C, 9 PIPE.

### 3.3 Sound instances (gameplay section 0x0c) — H
Update `sndinst+4` from table 0x20c580 `{class, fn}` (0x2a19a8). Pvars `{def, min_s, max_s, timer, slot}`.
The retrigger timer is `round((min + rand()%(max−min))·scale)·60` ticks, and 0/0 means "replay as soon as
the last one ends".
0 `0x3197a0` sphere: plays when |cam − pos| < range; releases beyond range + 1.
1 `0x319928` box with volume by depth. 2 `0x319cc8` box one-shot. 3 reverb box. 5 `0x31a078`
underwater loop at the camera (flags 0x15). 6 `0x31a128` music box (§4).

## 4. Music and streams — H (state names M)
- `InitVAGStreamingEx(4, 0xf000, 0, 1)` (0x2796f8). Track table = level header `music[15]` sectors, in memory
  at **0x13a628**, so track k = `levels/NN/music/k`. Tracks come in pairs `(Start, Loop)`.
- `music_start_track(k, flags, vol)` 0x279fa8: stream `table[k]` (flags 0x20 one-shot, group 1). The handle
  callback 0x27af20 sets state 8. `music_Update` then calls `StartTrackBody` 0x27a080, which **queues
  `table[k+1]` behind the running stream**, with stream flags 0x24 (looping) when `flags & 1`. So Start
  plays once and Loop repeats seamlessly.
- Players: main 0x151704, transition 0x15173c, dialogue 0x151720. Layout: +0 handle, +4 track, +6 vol,
  +8 flags, +0xa state (0 idle, 1 requested, 2 started, 3 buffered, 4, 5 stop, 6 stopping, 7 ended,
  8 queue body, 9 body queued, bit 15 paused), +0xc pause timer.
- Changes: `FUN_0027a248(track, stinger)` requests (0x1516f2, 0x1516f3). `music_Update` 0x27a688 acts when
  the timer 0x1516fc expires, then reloads it with `round(7·scale)·60` = **420 ticks**.
  `music_Transition` 0x27a168 plays the stinger on the transition player. The time-remaining callback
  0x27afb8 arms the fade: T = stinger remaining, F = T/4. The main track fades linearly,
  `vol·(F − (T − t))/F`, over the stinger's first quarter. Then stop → preseek the next track (paused) →
  continue when the stinger has < F left → body queued. Stream time units U.
- Level start: `entry` 0x259c40 sets track 0 and calls `music_start_track(track, 1, 0x400)`. The env
  sample point nearest the hero (0x264e98) sets the track from its +0x28 while music is idle.
- Dialogue (0x279cd8, id < 10000): `scene sounds` at `0x13a664 + id·0x250 + language·4`, group 2.

## 5. Novalis start (level 01)
- **Music** (V data, H logic): env point 0 (249.3, 131.3, 56.1) music = 0 → **track 0 `L01_Enemy_Start`**
  (3.8 s), then **`L01_Enemy_Loop`** (111.4 s) looping, vol 0x400, group 1. Music boxes (class 6) switch
  to 2 (`L01_Cave`) and 4 (`L01_Waterworks`) with stingers 6/7/8 (`L01_Transition1..3`, 6.0 s each).
  Instances 45 (0→4, stingers 8/7) and 46–48 (0→2, 8/6). The global `music` lump is a copy of
  `L01_Enemy_Loop`.
- **Reverb**: env point type 4 (STUDIO_C), depth 1500, delay 0, feedback 0 (enabled). Reverb box inst 30
  (type 3, depth 6000) is 9.8 from spawn.
- **Ambient emitters** in range of the spawn (Ratchet inst 0 at (162.5, 136.4, 60.5); the camera is a few
  units behind him):

| inst | class | dist | range | level def → bank | behaviour |
|---|---|---|---|---|---|
| 4 | 0 sphere | 14.4 | 24 | 23 → 21 | random-ambience script (markers, random delay/pitch), vol 2048→0 over 24, replays when done |
| 9 | 0 sphere | 17.9 | 24 | 25 → 2 | looped sample (32,004 samples ≈ 1.9 s), vol 512, reverb |
| 0 | 0 sphere | 19.1 | 24 | 21 → 21 | as inst 4 |
| 44 | 5 | 4.7 | — | 20 → 20 | underwater LFO loop; only while `0x167494` ≠ 0 (not at start) |

- **Moby loop sounds** in def range at spawn (only while the moby is active): class 688 gunship
  inst 694 (37.8, def 0 → bank 220, far 128, vol 1331, flag bit0); class 660 Blarg flyers 677/679/681
  (38–43, bank 214, far 100, vol 1228). Class 705 spinners (bank 227) are ≥ 72.8 away.
- **Ship** (class 531, `FUN_002a1c40`): **no sound defs, no sequence loop sound, no sound calls**. The
  landing is the intro scene's VAG speech/stream (`scene00/sound`, 44,056 Hz), which is not traced here.
  Ratchet: class 0 has 64 defs (bank 22–64). Footsteps come from level defs 4–19 (bank 4–19).

## 6. Port plan
**Formats (`rc-formats::audio`)**
1. `SoundBankFile` (chunk table), `SfxBlock` (header via first_sound/first_grain, sounds, v1 grains,
   typed `Tone`), `SoundDef` + `SoundRemap::apply(level_defs, class_defs)`, `VagHeader` (BE) + body.
2. `adpcm::decode_frame(frame, &mut hist) -> [i16; 28]`, plus a `VoiceCursor` that follows LSA/end flags.
3. Tests, all 19 levels + global (no oracle, so check structure and signal):
   - `first_grain + n_grains·0x28 == block size`; every tone offset < sample size; frames start at offset 0
     mod 16.
   - Every sample starts with a zero frame. One-shots end `1, 7` and tile to the next sample (Novalis
     229/230). Loops have exactly one 6 before the 3.
   - Shift ≤ 12 and filter ≤ 4. Remap counts equal class +0x0d, and every id < n_sounds.
   - Decoded bank samples: RMS 500–20,000 (Novalis 640–15,681), clipped < 0.01 % (14 of 2.95 M), zero
     crossings ≤ 0.9/sample.
   - Music `001`: 4,911,004 samples, RMS ≈ 7,729, no jump at interleave boundaries.
   - Loop start/end sample indices are multiples of 28.

**Game (`rc-game::sound`, deterministic, trace-checkable)**
4. The 30-slot manager exactly as §3.1–3.2: allocation rules, flag semantics, integer volume, pan and
   doppler, underwater and cutscene rules, the 36-sample occlusion ring (uses `CollLine_Fix`, already
   ported), the master groups, the game RNG draws in the same order.
   It outputs a per-frame command list `{Play, SetParams(mask), Stop}`, the same as the RPC batch, so a
   PCSX2 trace of `0x13e5c0` and of the command buffer can check it.
5. Sound instances (§3.3), animation triggers and loop sounds, footsteps, music state machine (§4), reverb
   selection.

**Engine (`rc-audio`)**
6. A software 989snd + SPU2 at 48 kHz. Ports:
   - the SFX grain VM (240 Hz), with `MakeVolume` and the pan table;
   - voices with ADPCM, ADSR, pitch, 4-tap Gaussian interpolation and group volumes;
   - a VAG stream player with queueing and pause;
   - a simple reverb send (the libsd presets are not reversed).
   Voice stealing (proposal, U): take a free voice in the group range, else the lowest-priority, oldest voice
   with priority ≤ the new tone's.
7. **Output: a raw sample sink, not Bevy's per-sound playback.** Wrap the mixer as one `rodio::Source`
   played through `bevy_audio`. That adds no dependency and does not change the build profile. Bevy's
   per-sound API cannot apply per-voice ADSR, pitch modulation, grain timing or 240 Hz parameter updates.
   Exact SPU mixing is not achievable. Which sounds play, when, at what 989 volume, pan and pitch can match
   exactly, and that is what the traces check.

## 7. Unknowns
- 989snd voice allocation and stealing, stream time units, and the exact SPU2 reverb (not disassembled; the
  IRX is available as entry 22).
- The global bank's (10 sounds) owner and handle, and entries 0–13 of the `irx` lump.
- Semantics of play flag 0x08, and the level copy of boot 0x20c940.
- The intro cutscene audio on first arrival (`scene00`), and help voice-overs (help message +0xc `vag`).

## In the port (2026-09-26)

Code: `rc_formats::{sound_bank, vag}` (first verified against the C++ oracle's `rc_extract sound`, retired 2026-09-27), `rc_game::audio`
(`audio.rs` SPU2 + frame driver, `audio/voices.rs` EE slots + emitters + 989snd voice manager,
`audio/grain_vm.rs` 989snd block player, `audio/music.rs` music EE + IOP streams), `rc-engine/src/audio_out.rs`
(bevy_audio output). Tags as above; **T** = checked by a test here.

**Formats (T, golden).** `sound_banks_for_every_level` (`crates/rc-formats/tests/formats/golden.rs`) serialises, per level
and for the global bank, the bank header, sounds, grains, sample extents, both decodes of every sample, remapped
level defs, map, per-class ids and defs and the music table, and checks each section against the committed
snapshot hashes (`data/loader_snapshots.tsv`; generated while byte-identical to the retired C++ `rc_extract sound`
dump); it proves a changed PCM bit or def byte reaches the snapshot. Totals: 19 levels + global, 5392 sounds, 10590 grains, 4806
samples, 56,633,696 PCM samples, 567 level defs, 5550 class ids. Corrections to §2 found on the way:
- Loop samples come in two shapes: `0, 6, 2…, 3` (293) and `0, 2…, 6, 2…, 3` (297, repeat-flagged frames before
  the loop start); one-shots are `0…, 1` padded with a `7` frame (4216). Every sample starts with a zero frame and
  the distinct tone offsets equal the header's vag count on every bank.
- Sample RMS over all banks is 197..18,024 (not ≥ 500); 68 clipped of 56.6 M.
- The OpenGOAL decoder variant differs by up to 2 LSB per prediction (it floors both terms), and the error feeds
  the history: 41.6 M of the 56.6 M samples differ.
- Remap: a level def index of −1 (levels 0, 11, 14) passes the loader's signed compare and reads the u16 before
  the map (0 on the disc); the per-class copy writes header-count entries, walking past the class's remap list
  when the header count is larger (level 10 class 1229: 12 defs, 6 ids). Both are reproduced.
- `sceSdNote2Pitch`'s table equals `trunc(0x8000·2^(k/12))` / `trunc(0x8000·2^(k/1536))` and the 140 u16 in the
  libsd IRX (decompressed `global/irx.bin` 0x7e100) (V). The 989snd pan table in the IRX (0x990a4) is
  `trunc(0x3fff·(cos, sin)(k/2°))` except entry 1 = (0x3ffe, 0xb6) (the formula and OpenGOAL give 0x8e); the
  port uses the disc value (V). The LFO sine is `trunc(32767·cos(2πi/2048))` (equal to OpenGOAL's table).

**EE (H, unit tests).** `SoundSlots` follows `fun_0022d7f0` / `sound_update` statement by statement:
first-free search over 26 (30 for privileged owners) slots, loop-flag check, initial-volume refusal, pitch bend
`lo + randi(hi − lo)`; per frame the listener ring (primed with the first camera position, see below), master
groups, owner follow, the distance law on PS2 floats, `vol·scale/1024` toward zero, the underwater halving, the
loop cut below 0x20, pan (`fwd·p`, `left·p`: the send loop transposes the camera rows 0x167450 first with
`fun_001fa2d8`, so v is camera-space), doppler `(trunc(dot·300)·0x5f4)/0x2e5`, the occlusion ring (six origins
per frame for new sounds, one re-test per 2nd/4th frame, `36 − n` law), and the command batch. Disassembled for
this: the play callback 0x2a1bc8 (`handle = h`; 0 frees the slot; state 1 → **2**) and the liveness callback
0x2a1c10 (`handle = h`; 0 frees). Two quirks kept: a slot without an owner uses the stack variable of the last
listener step as its velocity, minus the listener velocity once per such slot; and the EE pan of −1 / −2 degrees
reaches 989snd as `PAN_RESET` / `PAN_DONT_CHANGE`. The camera ring is zeroed at level init; the port starts at
the first gameplay frame and primes it with that camera position (M: the game keeps running `sound_update`
through the load and intro frames, otherwise the first frame would read 0 → camera as one huge step and every
static emitter would start with a wild doppler). Emitters: class 0 (`0x3197a0`, `FastDecTimer`, retrigger
`(min + randi(max − min))·60`, release beyond range + 1) and class 5 (underwater loop). Classes 1, 2, 3 are
listed as unported.

**989snd (M, mirrors OpenGOAL 989snd).** Handlers, grains 1/4/20–44, `MakeVolume`, `AdjustVolToGroup`,
`PitchBend` + `PS1Note2Pitch`, LFOs; v1 grains carry `s16 param[4]` inline and RAND_DELAY's modulus as a plain s32.
The Novalis ambience scripts (bank 21 for defs 21/23; bank 2 for def 25; bank 20 for the underwater loop) use
markers, GOTO_RANDOM_MARKER, RAND_DELAY, RAND_PB, SET_REGISTER(_RAND), DEC/ADD/TEST_REGISTER, LOOP_START/END,
RAND_PLAY, WAIT_FOR_ALL_VOICES, STOP, LFO: all ported; unported types (2, 3, 5–8, noise / reverb-only tones)
are counted (none occur). The IOP `rand()` is not reversed: a newlib-style LCG seeded 1 (M). SPU voice
allocation (U, the proposal of §6): free voice in the group range (groups 1/2/4 → 0x18–0x2f, others 0–47),
else the lowest-priority, oldest tone with priority ≤ the new one, else drop; streams are never taken.
OpenGOAL has no voice limit. `SetSoundParams` mask: 1 vol, 2/4 pan, 8 pm, 0x10 pb (the EE's bits; the IOP's
reading of 4 vs 2 is inferred).

**Music (H for the EE, M for the IOP).** `music_Update`, start / preseek / body / transition, the request
`FUN_0027a248`, `music_UpdateStream`, and the seven callbacks, disassembled here: start 0x27af20 (`handle = h`;
0 → idle; requested → 8), body 0x27aec8 (stores h only when negative; 0 → idle; 9 → 4, and phase 1 when +0x10),
preseek 0x27ae18 (0 → preseek again; 1 → 2), transition 0x27ae78 (1 → 4, phase 1), buffered 0x27ad88 (2 → 3),
remaining 0x27afb8 (`+0x18 = t`; phase 1 → 2 with T = t, F = t/4), liveness 0x27af60 (0 → state 7). Without a
stinger the timer only re-arms (the decompile starts nothing then; Novalis always has one). Music boxes
(`0x31a128`) test the **hero** position 0x13f3d0 against the box's inverse rows and request on leaving
(+x side → pvar[1] with the s16 at +0xe, else pvar[0] with +0xc). IOP stream player (all M): one SPU voice at
`rate·0x1000/48000` (3763 for 44.1 kHz), queued parts continue with the decoder history, the last part loops
with flag 4, ADSR 0x00ff/0x1fc0, volume `MakeVolume(127, 0, min(127, 127·vol >> 10), 0, 127, 0)` → group →
`>> 1`, time remaining in 48 kHz samples. A unit test shows the Start → Loop hand-over is sample-identical to
one voice playing the two bodies back to back.

**SPU2 (M).** ADPCM rounded form; Gaussian interpolation over the current and three previous samples (psx-spx /
PCSX2 / DuckStation; OpenGOAL looks ahead instead, three samples earlier) with the 512-entry table; pitch
counter `+= min(pitch, 0x3fff)`; ADSR state machine and step rule as OpenGOAL `envelope.cpp` (= psx-spx);
volume registers without sweep (level = reg << 1); RAM loop rules (LSA latched at a loop-start frame, end without
repeat → level 0); voices summed exactly, master 0x3fff, one clamp. Not modelled: reverb (level tones are
flagged for it; Novalis STUDIO_C depth 1500), per-core mixing and saturation, noise, PMON.

**Timing.** One `AudioSystem::tick` per 60 Hz game tick: EE (level start `music_start_track(0, 1, 0x400)` on the
first frame, emitters, `sound_update` with the previous frame's replies applied first, music boxes,
`music_Update`), then 800 samples, the queued commands executed first (their replies reach the next frame), a
989snd tick every 200 samples. Deterministic: two renders are identical (unit test and engine WAVs).

**Engine.** Bevy 0.19's default features include `audio` (bevy_audio + rodio 0.22 + cpal, already in
`Cargo.lock`); `audio_out.rs` registers a `Decodable` asset whose decoder reads a ring buffer that the game tick
fills (50 ms prefill, 0.25 s cap). Listener = the main camera (the follow camera when gameplay runs); hero =
Ratchet's 0x13f3d0 from crate::gameplay, else the camera. `RC_AUDIO=0`, `RC_AUDIO_WAV=path` (10 s WAV),
`RC_AUDIO_MUSIC` / `RC_AUDIO_SFX` (option volumes 0..1024; defaults 716 / 1024 from the boot data).
The underwater flag was wired on 2026-09-28 (below).

**Novalis start, measured** (10 s, offline test at the spawn and `cargo dev` frame-exact): music from sample 82
(the VAG's zero frame), per-second RMS 550–960 (music at 0x400 in group 1 = 716 → voice level ≈ 0.244); the
Start → Loop boundary at 3.841 s has no gap; three emitters start at once (instances 0, 4, 9: bank 21 twice, bank
2); effects alone: RMS ≈ 24 from the instance-9 loop until the bank-21 script's first `RAND_DELAY` (1800 +
rand % 2400 ticks at 240 Hz ≥ 7.5 s) plays its first ambience tone (RMS 200–350).

## The sound layer on the game stream (2026-09-28)

The sound code no longer draws from a private `srand(1234)` copy while the game runs. Everything below is on
`Game::rng` (the one stream), at the game's points in the tick (docs/plan/trace_results_novalis.md "Second
savestate", difference 2):

* **Sound step** (`rc_game::tick::Game::tick_with_sound`, `SoundHook`): after the camera update, before
  `0x15f5cc++`, in modes 0 and 2. `audio::class_sounds::sound_step` first plays Ratchet's animation-trigger
  class sounds, then runs `AudioSystem::tick_with`: the EE frame (`game_frame_with`: emitters, `sound_update`
  with the occlusion origin's 3 draws every tick, the 6-origin batch per new occluded sound, the pitch-bend
  `randi` per play), then its 800 samples. The listener is this tick's game camera (0x167240 / 0x167450..). The
  frame phase is the tick counter. Slot owners are resolved to moby +0x10 through the moby table; a deleted
  moby (state 0xfd / 0xfe) drops the owner and stops its loops.
* **Moby class sounds** (`PlayClassSound` 0x2a1618 / 0x2a16c0): `ClassSoundSink` implements the moby loop's
  `SoundSink`. `AudioSystem::play_class_sound` allocates the slot at once (owner = the moby, privileged for
  Ratchet and class 0x472; +0xe = the class-sound index), so the pitch-bend draw lands in the moby order. Its
  listener is the previous tick's camera. `Services::sounds` still records every call (the engine counts them
  per class and index).
* **Ratchet's animation triggers** (`RatchetAnimAdvance` 0x247d48): if key A equals key B before the advance,
  the first trigger with `16·frame_a₀ + trunc(16·t₀) < time ≤ 16·frame_a₁ + trunc(16·t₁)` plays its class
  sound with flags 0 (`class_sounds::ratchet_trigger`), at the game's point: `class_sounds::HeroClassSounds` is
  the hero's `HeroSounds` (`Game::tick_with_hero_sounds` → `hero_update_with_sounds`), called right after his
  advance, before the back items' draw and his physics (owner: his moby as the last write-back left it; listener:
  the previous tick's camera). His voices `0x236738` (the hurt / death voices of `hero::damage`) go through the
  same object (`HeroSounds::voice`). No Ratchet sequence has a loop sound (header +0x11 = 0xff for all 256).
* **Not routed**: Clank (601) and the back packs (607–609) have 0 class sound defs on every level (header
  +0x0d), so their triggers never play or draw. The hand items' triggers (wrench 71, …) and footsteps
  (0x2a1898) are routed since 2026-09-27 ("Sound paths" below), with every moby's triggers and loop sounds.
* **Engine** (crate::audio_out, crate::gameplay): the gameplay tick borrows `AudioOut` for both hooks. The sound
  step pushes the frame to the output ring and to the `RC_AUDIO_WAV` capture. When a rendered frame's ticks did
  not run (menu, catch-up), `run_audio` fills in IOP-only frames: 989snd and the SPU run, with no EE update and
  no draw. `RC_PLAY=0` keeps the standalone path (private stream, listener = main camera). `RC_AUDIO=0` runs
  the tick without a sound layer, so the stream then differs.
* **Headless** (`rc-trace` `port_sim`): the same sink, hero sounds and sound step, with the level's
  sound data from `extracted/` (the 989snd / SPU side rendered too, since its replies free the slots).
  `compare-novalis-spawn --no-audio` gives the old stream for comparison.

**Measured** (`compare-novalis-spawn`, idle window = slot 2 − slot 1, 1072 ticks): the sound step draws
3.00 per tick (3216 in the window; 6 plays and 3 class sounds over the whole run, no 6-origin batch in the
window). The port's window draws are 35 827 = 33.42 per tick with sound and 32 739 = 30.54 without, against
the game's 56 072 = 52.31. The remaining gap of 18.9 per tick is the flyer exhaust emitters, the gunship
volley, the 760 foam and critters (trace_results_novalis.md, differences 1 and 3), plus the flyer loop sounds
(slots 1 and 5 in the game, whose pitch bends the port does not draw: the flyer sound code is not ported).
Engine, frame-exact, `RC_SCENE=0`: the crate script's pitch-bend draws (crate break, bolt pickups: 10 class
sounds by tick 439) shift the stream. With the sound layer the break spawns 14 dynamic mobys (15 with
`RC_AUDIO=0`) and 16 bolts are collected (17 with `RC_AUDIO=0`). The first stream difference is at tick 10
(Clank's first fidget row). Ratchet's path, the break tick (341) and the jump onto crate 410 (z 41.0000) are
unchanged. Two idle runs give identical traces, PNGs and WAVs.

### Scene audio on the game tick (fix, 2026-09-26)
* **Regression.** After the sound layer moved onto the gameplay tick, a running scene (mode 2) suspends the tick
  (`scene_render`: `GameTick.run_if(!running)`), so no EE audio frame ran for the whole scene: `run_audio` filled
  the scene frames with IOP-only frames. The scene inbox (`PauseMusic`, `Speech`) was never drained and
  `music_Update` never took the start callback, so track 0 played on unpaused with no speech and ended without
  its Loop body. On the first tick after the scene every request applied at once, `StartTrackBody` queued track 1
  behind the ended stream, and `Streams::execute` then started it as a **new** stream whose handle the body
  callback ignores (it only takes an error value): the player saw its own handle dead, went idle and restarted
  track 0, while the orphaned Loop stream played on. Two copies of the theme.
* **Fix.** (1) `audio_out::scene_sound` (FixedUpdate, after the scene frame and the suspended tick) runs
  `AudioSystem::scene_frame_with` once per scene frame: the scene requests apply at once (the game issues them from
  the blocking start / end code), the EE frame runs when `SceneTick::world_runs` (`CutsceneModeUpdate`: game
  stream, game counter, listener = the scene camera, or the play camera before the first scene tick), the blocking
  fades / vsync waits get IOP-only frames. (2) Queueing a body behind a stream that is no longer alive replies 0
  (the player goes idle and restarts the track) instead of starting an untracked stream. (3) The music pause holds
  every group-1 stream voice until the resume, also one keyed meanwhile, and a held voice whose registers get
  rewritten (the normal group volumes coming back when the cutscene volumes end) records the new values and stays
  silent; before, the held voice output its frozen sample as a DC offset (±441) for the 30 resume ticks.
* **Measured** (engine, frame-exact, scene 5, 40 s WAV; `RC_AUDIO_TRACE=1` logs music voices playing / held once
  a second): music held from output frame 2 to 1563 (full − `RC_AUDIO_MUSIC=0` is exactly 0 there), the speech
  envelope matches `speech/05_en.bin` decoded (correlation 0.994, starting at frame 9 = the `Speech` frame),
  music resumes at frame 1564 (31st gameplay tick after the end frame) with one voice from then on; the
  music-only signal matches Start+Loop decoded at one constant lag (798 samples = the frame it played before the
  pause) through the Start → Loop hand-over (window correlations 0.98–0.998). `RC_SCENE=0`: one music voice from
  the level start, the hand-over at 3.84 s at a constant lag. Two scene runs give identical WAVs and PNGs.
* The EE frames of a scene draw from the game's stream (3 per frame for the occlusion origin), as the game's
  mode-2 `sound_update` does; the engine's scene does not run the mobys, so the stream after a scene still differs
  from the game's.

## The hero's loops, item sounds and delayed voices (2026-09-26, hero polish)

* **Gadget class sound defs.** A hand item's class (wrench 71, Swingshot 0xd0, every weapon of the gadget table) has no
  blob in the level's class table, so `parse_level_sounds` leaves its defs empty; the loader parks the first
  `min(count, 15)` ids of its remap list at 0x1b02e0 (`LoadLevelCoreData` 0x258128) and `select_world_object_resource_tables`
  0x259788 writes them into the blob's defs (+0x28, count +0x0d) when the gadget loads. `sound_bank::apply_gadget_defs`
  does the same for every gadget blob (`LevelAudio::from_parts`); the golden (snapshot) output of `parse_level_sounds` is
  unchanged. Test `audio::tests::gadget_class_sounds_on_every_level` (21 gadget classes on each of the 19 levels).
* **Item sounds** (`PlayClassSound(i, 0, item)` in the item's update: the Swingshot's fire 0 / hit 1 / pull 2, the
  wrench's hit `FUN_002bda88()` = 0 here): `HeroSounds::item_sound`, played by the tick right after the item update
  (`hero::gadgets::flush_item_sounds`). The slot's owner is Ratchet's moby (the item is not in the moby table; it is in
  his hand), privileged as the game's 0x1403e0 owner.
* **Loops** in the hero's slots 0x141568 + 4n (`packs::loop_sound`): the grind / cable (n 0, class sound 0), besides
  the packs (3 / 4) and the sinking floor (surface.rs, n 2). **Voices**: the ledge climb's (4, Thruster only), the cable
  grab's (0xd), the swim's `SwimEvent::Sound` (3 / 0x11), and the delayed-voice queue 0x141528 (`0x236810` /
  `0x236860`: the surfacing gasps 7 / 8), all through `HeroSounds::voice` (`hero::fx::flush` after the transitions).
* **Position of the draws.** The loop / voice / item sounds' pitch-bend draws are made where the port plays them (after
  the physics, after the transitions, after the item update), not inline at the game's call; within those steps the
  other draws of the step come first. Only the first tick of a loop and the one-shots are affected.
* **`RC_AUDIO_TRACE=1`** now also logs each class sound play (`AudioSystem::play_log`): tick, class, index, flags, slot.
  Measured: Oltanis grind, tick 6 `class sound 0 of class 0, flags 0x4 -> slot 0`, effects-only RMS ≈ 1700 from 0.1 s on
  (`scratchpad/hero_polish/grind_sfx.wav`); Aridia ○ at a swing target, tick 61 `class sound 0 of class 208` and tick
  69 `class sound 1 of class 208`.

## In-level cinematics and scene hand-offs (2026-09-27, docs/plan/cutscenes.md)

* The kind-B cutaways (camera trigger 737, gunship 688, bolt crank 280) do not touch the sound: the game stays in
  mode 0, the mobys' and the hero's sounds go on, the master volumes stay the gameplay ones.
* Scenes are now started by the classes' requests (the mission NPC's arrival scene 5, its scenes 3 / 4, the talkers'
  hand-offs); the audio path is the one above. **Check** (engine, frame-exact, arrival scene, 28 s WAV,
  `RC_AUDIO_TRACE=1`): the trigger on app frame 3 as before; the music voice held from the start to frame 1560,
  speech on until ≈ frame 1500, music back at frame 1564 with one voice; the WAV matches the previous scene capture
  (`hero_polish/novalis_scene.wav`) second by second at correlation ≥ 0.99 through the speech (identical for 0–2 s;
  the small differences are the level's class sounds before the scene); two runs give identical WAVs and PNGs.
* Movies (mode 1): `StartPssMovie` stops all sounds and the music; `MovieExitToGameplay` restarts the level track
  (`music_start_track(0x151708, 1, 0x400)`). Ported (2026-09-27, docs/plan/cutscenes.md §5): `AudioSystem::movie_stop`
  (the audio state rebuilt from the level data: every slot free, every voice and stream stopped, music players reset,
  a pending box track made current; options, counters, IOP random stream and play log kept; the emitters' timers
  restart [L]), `AudioSystem::mix_movie` (the movie's two SPU voices: stream law at vol 0x400, hard left / right,
  group 5 = sfx option, then the SPU master: unity gain at the defaults [L]) and `AudioSystem::movie_exit`; the
  engine pushes one 800-sample frame of movie audio per movie vsync (silence in the fades).


## Sound paths: how the game triggers every sound (2026-09-27)

The user heard the jump, the enemy shots, the Bomb Glove, the ambience and the music, but no footsteps, no enemy hit /
reload / jetpack sounds and no flyer engines. The reason is structural: the game has **several shared trigger paths**,
and only the ones whose calls sit in ported class or hero code worked. The rest are driven by the level data through
shared engine code that had not been ported. Addresses level01 (boot equivalents in `tools/ghidra/names/clusters.tsv`).

| # | Path | Shared function(s) | Data it reads | Port status |
|---|---|---|---|---|
| 1 | Explicit class-code calls | `PlayClassSound` 0x2a1618, `PlayClassSoundByClass` 0x2a16c0, `SoundIsAlive` 0x2a12f0, `release_voice_slot` 0x2a1348 | class sound defs (class +0x28, count +0x0d, remapped at load) | worked: every ported class calls `World::play_sound` (crates, bolts, platforms, the troopers' glob 722, the gunship shells, the explosions' sound of `SpawnBeamExplosion` 0x273310 / `0x273f50`) |
| 2 | Hero code | voices `0x236738`, loops `0x236798`, `HeroStatePhysics` 0x2370b8, `HeroPdaGadget` 0x240ed8, the hand item's update (wrench `0x2be1c0`, Swingshot) | Ratchet's class defs (class 0), the gadget defs parked at 0x1b02e0 | worked (`HeroSounds`) |
| 3 | **Animation triggers** | `MobyAnimAdvance` 0x265260 (its tail); Ratchet's own `RatchetAnimAdvance` 0x247d48 | sequence trigger words (`lo16` class sound, `hi16` time in 1/16 frames), count +0x12 → moby +0x7e | Ratchet: worked (his jump / land / swing voices). **Every other moby: not ported → now general** (`moby_update::anim_sound`): trooper steps / reload / hit reactions, critter steps and squeaks, amoeboids, NPCs; the hand item's triggers (wrench whooshes) too |
| 4 | **Sequence loop sounds** (engines, jetpacks, hums) | `MobyAnimAdvance` → `FUN_002637d8`; set by `update_moby_animation_state` 0x263718 (`hard_cut` 0x26c5a8, `InitMobyInstance` 0x263488) and `MobyAnimBlend` 0x26c660 | sequence header +0x11 → moby +0x7c (class sound), +0x7d (voice slot) | **not ported → now general**: the Blarg flyers' (660) and gunship's (688) engines, the troopers' jetpack (459 seq 8), spinners 705, amoeboid 572/865/866 loops. Pitch / volume / pan / doppler by distance and speed are `sound_update`'s (§3.2, already ported), since the slot follows its owner moby |
| 5 | Hit / damage sounds | none in the shared hit resolver (`MobyGetHitMessage` 0x26f320, the damage records 0x26f378): the **victim** sounds through its hit-reaction sequences (path 3); the **attacker** plays its own: the wrench `FUN_002bda88` picks class sound 1 when the hit moby's damage record (`FUN_002711f8`, pvar +0 pointer) has +9 = 1, else 0 | the creature pvar header | victim: via path 3 (now); wrench selection **was a constant 0 → now read from the hit moby** (`melee::wrench_hit_sound`; Novalis troopers and critters have +9 = 0, so they get sound 0, as in the game) |
| 6 | **Footsteps by surface** | `HeroFootstepSound` 0x227e48 → `PlayFootstepSound` 0x2a1898; callers `FUN_00227e90` (in `0x228870` after `HeroItemsUpdate`: walk seq 3 at key times 49.5 / 17.0, run seq 4 at 12.5 / 1.0, the 22nd tick of state 2) and the landing of the fall (`HeroStateTransitions` case 6 / 0x2d) | level def `tbl[level] + class·4 + foot·2 + variant + 0x15f574` (table `g_footstep_level_base` 0x1bdca0 = `[0,2,6,1,0,2,2,0,2,2,1,7,0,0,4,0,0,0,0]`, identical in all 19 overlays; 0x15f574 = 2), class = collision type bits 5–6 of the ground (`CollSoundClass` 0x215208 via the ground probe → 0x14063d), variant 1 with the Magneboots model on the feet | **never ported → now** (`hero::fx::walk_footsteps`, `fx::footstep`, `AudioSystem::play_footstep`). Ratchet's walk / run sequences carry no triggers: path 6 is the only source of steps |
| 7 | Level defs at a moby / at the listener | `PlayLevelSoundAtMoby` 0x2a1770 (defs < 0x15f574) | level defs 0–1 | **ported 2026-09-28** ("Reverb zones and level sounds"): the help box's opening sound (def 0) wired; the skill point jingle (def 1) is callable from class code (`World::play_level_sound`), its callers (kills, weapon `0x2d2450`, cheat entry) are not ported |
| 8 | Sound instances | `SoundInstanceUpdate` 0x2a19a8, table 0x20c580 `{0 sphere 0x3197a0, 1 box volume 0x319928, 2 box one-shot 0x319cc8, 3 reverb box 0x319f18, 5 underwater 0x31a078, 6 music box 0x31a128}` | gameplay section 0x0c + pvars | 0, 5, 6 worked; **1 and 2 now ported** (Novalis has 2 and 8 of them, most levels use them); 3 ported 2026-09-28 with the reverb |
| 9 | Music and scene streams | `music_Update` 0x27a688, scene speech | level header music table, scene sounds | worked (§4, "Scene audio on the game tick") |

**Now general** (one implementation each, data-driven, every class, every level):
* `rc_formats::moby_anim::{advance_trigger, advance_landed, loop_sound_of}`; `set_sequence` / `hard_cut` set +0x7e as
  `update_moby_animation_state` does.
* `rc_game::moby_update::anim_sound`: `advance(w, id)` = `MobyAnimAdvance` with its trigger (`PlayClassSound(s, 0, m)`, then
  no refresh that tick), the landing rule (a tick that lands a blend does neither) and the loop refresh `refresh_loop` =
  `FUN_002637d8` (play with flags 4 when +0x7d = 0xff; forget the slot when its owner changed; release it when it plays
  another class sound); the refresh of a moby without a slot runs on its phase `(address >> 8 & 3) == (0x15f5cc & 3)` =
  `index & 3` (array at 0x1e9a480 on Novalis); `init` / `after_sequence_change` keep +0x7c. The scheduler, the dropship's
  carried troopers, `World::anim_blend`, `create_moby` and the load all go through it. `SoundSink::slot_owner` gives the
  refresh the slot's owner (+0x18, zeroed on free: callbacks 0x2a1bc8 / 0x2a1c10 disassembled) and class-sound index.
* The hand item's advance (`hero::items::advance_item`): its triggers queue with the item's sounds.
* The hero's own advance (`RatchetAnimAdvance` 0x247d48, `hero::anim`): its loop-sound tail (+0x7c set by `SetAnim`
  from the target sequence header, +0x7d refreshed after the trigger with no phase gate) is `AnimCtl::refresh_loop_sound`.
  The only sequence in the game with one is Clank's glide (class 0x57 sequence 0xe, class sound 0x10): his helicopter
  hum while he holds ✕ in the air.
* Footsteps (path 6), the wrench hit selection (path 5), sound instance classes 1 and 2 (path 8).

**Conformance** (`crates/rc-game/tests/ui/sound_conformance.rs`, all 19 levels): every trigger and loop sound of every
sequence of every loaded class (level classes, Ratchet's `ratchet_seq`, the gadgets) resolves to a def with a valid
bank id and, played at the listener through `AudioSystem::play_class_sound`, gets a slot and a live 989snd handle: 4252
references, 4251 played, 1 refused exactly as the game refuses it (level 13 class 83 seq 1: volumes 0..20 < 0x20). Every
footstep def (class, foot, variant) plays, except the water class 3 defs on levels 5, 6, 7, 9, 13, 17, which are looped
or past the defs there and are refused as in the game (class 3 is only produced on levels 1 and 0x12). The footstep
table and 0x15f574 are checked in every overlay.

**The footsteps "regression"** (bisect, worktrees in scratch): no committed version ever played footsteps. The first
commit d815fa9 and HEAD 5746893 give the same effects-only RMS over a scripted 3.7 s run (100 ms windows equal or
within 4 %, no step transients in either); `0x2a1898` has never been called and Ratchet's walk / run sequences
3 / 4 have no triggers. What the user praised on 2026-09-26 (before the first commit) was most likely Ratchet's
trigger sounds 30 / 31 / 39 (the foot plants at the end of the jump, landing and stop sequences), which play in every
commit (inferred: that tree was never committed).

**RNG** (`compare-novalis-spawn` on `novalis_spawn_ee.bin` / `novalis_idle_ee.bin`, window = 1072 ticks): 47.31 → 47.57
draws per tick (game 52.31). The new draws are the loop and trigger plays' pitch bends in the moby loop (at the moby's
advance, before its update) and the 18-draw occlusion batch of each new occluded sound in the sound step (3.02 → 3.22 per
tick): in the window the port starts 14 flyer engine loops as flyers cross the ≈ 84-unit start threshold (squared falloff,
far 100) at the edge of their paths (flyer positions within 2.3 units of RAM, report §f2). The slot table at the two
savestates holds flyer loops in the game too (slots 1 and 5), so these starts are the game's; the "3.00 per tick" of the
game's sound in the attribution table was a model, not a measurement.

**Evidence** (engine, `RC_SCENE=0`, `RC_AUDIO_MUSIC=0`, `RC_AUDIO_TRACE=1`, scratch `sound/before` vs `sound/after`):
running (`30-250:stick 0 -1`): 19 footsteps (level defs 4 / 6: surface class 0, left / right) vs none; the flyers' engine
loop (class 660 sound 0, flags 4) 8–12 starts per run, the gunship's (688) from tick 4, the troopers' jetpack (459 sound 6)
vs none; wrench on a trooper: its hit reactions 459 sounds 3 and 5 and the death explosion's 4 vs only 4; at the trooper
firing position: 459 sounds 0 and 7 next to the glob 722; critters: 577 sounds 0–6 (steps, hit, death) vs only 4. Two
identical runs give identical WAV and PNG. The arrival scene: the music / speech trace (voices playing / held, speech,
player state, resume) is line-for-line identical to before; the mix differs only by the added engine loops during the
first 2 s (correlation ≥ 0.98 through the speech).

**Not done:** the hand items' own loop sounds (gadgets 168 Blaster seq 4, 185 seq 3; weapons not ported), the skill point
callers of path 7, the class-specific flyer sounds of `FlyerPathDriver` for classes 0x33, 0x46a, 0x473, 0x474 (not on
Novalis; not dispatched to the port yet). Reverb boxes, path 7 and the underwater flag: next section.

## Reverb zones and level sounds (2026-09-28)

Addresses level01; the same code sits in every overlay (the reverb box and `FUN_002a1a90` in 13 identical copies, the
others differ only in absolute addresses). Code: `rc_game::audio::reverb` (zones + effect), `AudioSystem::{hero_teleported,
checkpoint_saved, checkpoint_restored, play_level_sound_at_moby}`, `class_sounds::{listener_with_water, level_sound}`.

**What the game does (H, disassembled where marked).**
* **Request state** 0x13e5b0 owner, 0x13e5b4 s32 depth, 0x13e5b8 type, 0x13e5b9 delay, 0x13e5ba feedback, 0x13e5bb dirty.
  `FUN_002a1a90(owner, type, depth, delay, fb)` marks only what changed: 1 type, 4 depth, 2 delay/feedback.
* **Reverb box** (sound instance class 3, `SndInstReverbBoxUpdate` 0x319f18), pvars `{u8 type, u8 delay, u8 feedback, u8
  inside, s32 depth}`. The **hero** position 0x13f3d0 (not the camera) minus +0x40, through the inverse rows +0x50: inside
  `|x|,|y|,|z| ≤ 1` → depth `trunc(depth·(x+1)·0.5)` (≤ depth), inside = 1. On leaving: `x > 0` → the box's full depth
  (owner 0); else type 0 depth 0 (off). The boxes sit in pairs at cave mouths with +x pointing in (Novalis 29 / 30 studio B
  6000 at the two mouths of the `L01_Cave` music boxes 46 / 47; 36 studio C 5000; 38 / 39 pipe 3072).
* **Env sample points** (section 0x88): `EnvNearestSamplePoint` 0x264e98 (disassembled), called only by `HeroTeleport`
  0x2368e0 (the level start's `HeroTeleport(0x13e090)` in `GameStateUpdate`, vendor exit, ship, camera triggers): the
  nearest point with `d ≤ 8` (strictly nearer wins) sets depth / type / delay / feedback directly with dirty |= 7 when its
  +0x27 is set, and 0x151708 = +0x28 when that is ≥ 0, the main player idle (0x15170e = 0) and 0x1516f0..f2 = (0, 0, −1).
  The Novalis point (249.3, 131.3, 56.1; studio C 1500) is 87 units from the spawn, so it never applies at the start.
* **Commands** at the head of `sound_update`: dirty & 8 → `snd_SetReverbEx(2, 0, 0, 0, 0)`; else & 0x13 →
  `SetReverbEx(2, type, depth, delay, fb)`; else & 4 → `snd_AutoReverb(2, depth, 12, 3)`; dirty = 0. Bit 8: `StartPssMovie`,
  the level unload (`fun_00231608`, boot `0x1e9488`); bit 0x10: `MovieExitToGameplay`, the load's end. Core 2 = `SND_CORE_1`.
  The checkpoint record `0x29ac10` copies depth / type / delay / fb to 0x1bb6ec.., the death reload `0x29adc8` restores them
  with dirty |= 7.
* **libsd types** used on the disc (98 boxes, 15 env points): 1 room 3, 2 studio A 19, 3 studio B 49, 4 studio C 18,
  9 pipe 9 (boxes); points studio B / C and one type 0. Delay / feedback are 0 except level 5 box 14 (127 / 127 on studio C,
  ignored by libsd for that type). No echo / delay type is used.
* **Level sounds** `PlayLevelSoundAtMoby(i, flags, moby)` 0x2a1770: `i < 0x15f574 (= 2)` → `SoundSlotAlloc(level def i,
  flags, moby, 0, 0x400)`, +0xe = i, +0x18 = moby. **Every call site** in the 19 overlays (102, found by the function's
  body and `jal`; test below) is `(0, 1, 0)` = `Help_ComputeSize` 0x225a98 when the help text or voice option is on (19),
  or `(1, 0, 0)` = a skill point (`if !0x13d408[k] { set; PlayLevelSoundAtMoby(1, 0, 0); ShowBanner(0x53d6) }`: Blarg flyer
  and gunship kills on Novalis, weapon update `0x2d2450`, the other levels' classes, and `MenuInput`'s cheat entry) (83).
  Defs 0 and 1 are the same on every level (far 90, vol 1638, no pitch bend, flags 14 / 2): moby 0 → 2-D, fixed volume, no
  `rand` draw. The "menu beeps" are not level sounds.
* **Underwater** 0x167494: `UnderwaterTest` 0x20e9f0 in the camera update (ported in `fog_zones::UnderwaterState`, run
  by crate::fog_state); `sound_update` reads it with the water level 0x13f640 (§3.2).

**In the port.**
* EE: `ReverbRequest` (set / env point / command / checkpoint), `ReverbBox` updated after the emitters with
  `FrameInput::hero_pos`, the command queued before the slots' commands. Level start: `hero_teleported(hero_pos)` on the
  first frame; class teleports: `cinematic::hero_teleport` → `SoundSink::hero_teleported`; checkpoint:
  `checkpoint::record` → `SoundSink::checkpoint_saved`, the engine's death respawn → `checkpoint_restored`. Movies keep the
  request and the boxes' pvars (bit 8 at the stop, 0x10 at the exit). [L] The game keeps the request across a level change
  (0x10 at the load's end resends the previous level's values when no `sound_update` ran between); the port starts each
  level with the reverb off.
* IOP: `SndCommand::{SetReverb, AutoReverb}` → `ReverbFx::{set, auto}`. A new type rebuilds the effect (tail cut, [M] as
  libsd re-inits the work area); the depth is set at once; `AutoReverb` glides linearly over 12 × 200 samples [M: the unit].
* Mix: 989snd sets the voice's reverb send for tone flag 1 (all level-bank tones; global-bank tones and VAG streams are
  dry). [M] The SPU2's per-core routing is not modelled: the send takes reverb-flagged voices whatever core their voice is
  on. Wet = `depth / 0x8000` × the effect, added before the master volume; nothing runs while the type is 0.
* Effect (native, not the SPU engine; hardware_fidelity_layers.md "Result-level reproductions"): per channel four damped
  feedback combs (the preset's same-side and cross-side loop times, and two in between; gains from the preset's decay
  time) into two all-passes (the preset's times and gains), input low-passed at ≈ 11 kHz; the right side detuned by 23
  samples. Character per type, derived from libsd's table: room 0.78 s, studio A 0.93 s, studio B 0.88 s, studio C 2.17 s,
  hall 1.69 s, space 2.91 s, pipe 0.72 s with a 4.2 ms loop (a metallic ring); echo / delay a delay line of
  `(delay + 1)·256` samples with feedback `fb / 128` [M, unused on the disc].
* Underwater: `audio_out::sync_underwater` copies `FogState::underwater_flag()` into `AudioSystem::underwater` before the
  tick; `class_sounds::sound_step` builds the listener with it and the hero's `water_level` (0x13f640). The flag is the
  previous frame's camera update's (crate::fog_state runs after the tick): one tick late [L].
* Level sounds: `AudioSystem::play_level_sound_at_moby` (log class −2), `SoundSink::play_level_sound`,
  `World::play_level_sound`; the HUD's help box queues `(0, 1)` in `HudState::level_sounds`, crate::hud_render plays it.
* `RC_AUDIO_TRACE=1` logs every reverb command and level sound; `RC_REVERB=0` (developer switch) mutes the wet output.

**Evidence** (engine, frame-exact, `RC_SCENE=0`, scratch `reverb/`): Novalis, Ratchet placed in box 30
(`RC_HERO_AT=152.16,134.53,62.8,-2.4426`) walking into the cave (jump, wrench): frame 1 `SetReverbEx type 3 depth 5125`,
the glide to 6000 by frame 36 (out through +x, into the cave); the mix differs from `RC_REVERB=0` by −10…−19 dB (wet)
around every level-bank sound, tails after the steps; two runs identical (WAV and PNG), PNGs identical to `RC_REVERB=0`.
Open ground (`RC_HERO_AT=166,140`): no reverb command, WAV identical with and without the switch. Level 16 box 12 (studio
C): `SetReverbEx type 4 depth 2497`, wet −17…−28 dB. Arrival scene: no reverb at the spawn; WAV identical with / without the
switch and across two runs; music held until the resume as before. `RC_UNDERWATER=1`: underwater loop, pitch drop and the
music at 3/5.

**Tests**: `audio::reverb::tests` (box ramp / exits, dirty priorities, checkpoint, env radius, preset decay, glide, echo
delay); `tests/ui/reverb_conformance.rs` on all 19 levels: 98 boxes parse and behave, the wet signal in the mix (13 levels with
boxes) and none away from them, deterministic; 13 enabled / 2 disabled env points; defs 0 / 1 play with a live handle and
no draw; the 102 call sites.
