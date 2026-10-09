//! Moby animation: sequences/keyframes on the disc and the game's evaluator that turns them into the
//! joint palette `F_j = P_j · S_j` the skinning uses. Spec: docs/plan/moby_animation.md (layout §1,
//! instance fields §2, `MobyAnimAdvance` §3, `MobyAnimEval` §6).
//!
//! Ported routines (boot ELF `SCUS_971.99`):
//! * [`advance`] = `fun_0020d580` (L01 0x265260), the per-tick step: `t += speed·rate`, the
//!   [0.99609, 1.00391] snap to 1, the leftover carry `(t − 1)/r_old · r_new`, wrap at the end of the
//!   sequence (flag 2), backwards play. EE FPU ops (`add.s`, `madd.s`, `div.s`, `mul.s`).
//! * [`evaluate`] = `fun_0020e0e0` (L01 0x265dc0) without the runtime pose layers (+0x60/+0x64 lists,
//!   null for every moby without special update code): channel decode into the scratchpad joint
//!   records, the three sparse-record lists (inherited scale, translation, post-scale — the last one
//!   with the game's re-append bug, see [`post_scale_list`]), plain lerp or nlerp with hemisphere flip,
//!   the local matrix rows `R(q)ᵀ`, inherited scale, the parent chain through the `common_trans` +0xc
//!   SPR address, post-scale after the chain, then `P·S` with the class skeleton.
//!
//! Arithmetic: everything runs on the PS2 float model ([`crate::tfrag_light::ps2`]: truncating
//! multiplies/adds, the VU adder's operand pre-truncation, `rsqrt` = truncated sqrt then divide),
//! in the exact instruction order of the disassembly (every `ACC` chain, every `(q+q)·q` product).
//! Reason: the palette feeds `ftoi0` of the skinned positions and the 1/128-grid colour sums, where
//! the last bit can flip an output integer; plain f32 (round-to-nearest) differs in the last bit on
//! most operations. The cost (a few µs per joint) is paid once per distinct pose per 60 Hz tick.
//! Values are carried as raw f32 bit patterns ([`V4`]).

use crate::buf::{invalid, Buf, Result};
use crate::moby::MobyClass;
use crate::tfrag_light::ps2;
use bytemuck::{Pod, Zeroable};

/// Four VU lanes as raw f32 bits.
pub type V4 = [u32; 4];
/// A matrix as four VU rows (row i = image of axis i, row 3 = translation; `M·v = r0·x + r1·y + r2·z + r3`).
pub type Rows = [[f32; 4]; 4];

const ONE: u32 = ps2::ONE;
const NEG_ONE: u32 = 0xbf80_0000;
/// SPR base the `common_trans` parent words point into (`0x70000000 + 0x40·parent`).
const SPR: u32 = 0x7000_0000;
/// SPR joint records 0..0x1bff (the ≤ 0x6f-joint limit; the skeleton is DMA'd to 0x1c00).
const SPR_RECORDS: usize = 0x70;
/// Identity rows.
pub const IDENTITY: Rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

// ---------------------------------------------------------------------------------------------------
// Disc layout (§1)

/// Sequence header (0x1c bytes), class-relative (or `ratchet_seq` blob-relative) offset in the
/// class's pointer list at 0x48.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MobySequenceHeader {
    /// 0x00: bounding sphere (centre, radius), packed model units; `fun_0020e098` lerps A/B by t.
    pub sphere: [f32; 4],
    /// 0x10: frame count.
    pub frame_count: u8,
    /// 0x11: looping sound id, 0xff = none (→ moby+0x7c).
    pub loop_sound: u8,
    /// 0x12: trigger count (→ moby+0x7e).
    pub trigger_count: u8,
    /// 0x13: always 0 on the disc.
    pub pad: u8,
    /// 0x14: trigger-data pointer (non-zero in 4 sequences on the disc); not read by the animation code (the level04
    /// leg walker reads it: [`gait_records`]).
    pub trigger_data: u32,
    /// 0x18: rate override: if ≠ 0 it replaces every frame's rate in `MobyAnimAdvance`.
    pub rate_override: f32,
}
const _: () = assert!(std::mem::size_of::<MobySequenceHeader>() == 0x1c);

/// Frame (keyframe) header, 16 bytes; the payload of `qwc` quadwords follows.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MobyFrameHeader {
    /// 0x0: t increment per tick for the interval from this key to the next (= 8 / Δtime).
    pub rate: f32,
    /// 0x4: time of this key in 1/8 ticks.
    pub time: i16,
    /// 0x6: payload quadwords = `(trans_offset + 8·trans_count + 15) >> 4`.
    pub qwc: u16,
    /// 0x8: quaternion bytes = 8·joint count (= offset of the scale records).
    pub quat_bytes: u16,
    /// 0xa: scale record count (not read by the game, which walks from `quat_bytes` to `trans_offset`).
    pub scale_count: u16,
    /// 0xc: offset of the translation records (= `quat_bytes` + 8·scale count).
    pub trans_offset: u16,
    /// 0xe: translation record count.
    pub trans_count: u16,
}
const _: () = assert!(std::mem::size_of::<MobyFrameHeader>() == 0x10);

/// Scale record: `u16 sx, sy, sz` in 4.12 (value / 4096), joint, flags (bit 7 = inherited).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct ScaleRec {
    pub scale: [u16; 3],
    pub joint: u8,
    pub flags: u8,
}

impl ScaleRec {
    /// Flag 0x80: the scale enters the local matrix (children see it); else it scales only this joint's
    /// final P_j after the chain. The game tests the sign of the 64-bit record (`bltz`).
    pub fn inherited(&self) -> bool { self.flags & 0x80 != 0 }
    /// The value as `pextlh` / `psrlw 13` / `vitof15.xyz` produce it (= u16 / 4096, exact).
    pub fn value(&self) -> [f32; 3] { self.scale.map(|s| s as f32 / 4096.0) }
}

/// Translation record: `s16 tx, ty, tz` in packed model units (`vitof0`), joint (read with `lb`), pad.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct TransRec {
    pub trans: [i16; 3],
    pub joint: u8,
    pub pad: u8,
}

/// One keyframe: the header, one quaternion per joint (s16 x, y, z, w in 1.15) and the sparse scale
/// and translation records in disc order (the order matters for the post-scale list quirk).
#[derive(Clone, Debug, PartialEq)]
pub struct MobyFrame {
    pub header: MobyFrameHeader,
    pub quats: Vec<[i16; 4]>,
    pub scales: Vec<ScaleRec>,
    pub trans: Vec<TransRec>,
    /// The `qwc`·16 payload bytes as the game DMAs them (quaternions are read from here by joint index,
    /// so a class joint count larger than the frame's reads the following bytes, like the game).
    pub payload: Vec<u8>,
}

impl MobyFrame {
    /// Quaternion of joint `j` as `lq`/`ld` + `pextlh`/`psraw 16` read it from the payload (0 past the end).
    pub fn quat_at(&self, j: usize) -> [i16; 4] {
        let mut q = [0i16; 4];
        for (k, v) in q.iter_mut().enumerate() {
            let o = 8 * j + 2 * k;
            if o + 2 <= self.payload.len() { *v = i16::from_le_bytes([self.payload[o], self.payload[o + 1]]); }
        }
        q
    }
}

/// One animation sequence.
#[derive(Clone, Debug, PartialEq)]
pub struct MobySequence {
    pub header: MobySequenceHeader,
    pub frames: Vec<MobyFrame>,
    /// Trigger words after the frame pointers: lo16 = sound id, hi16 = time in 1/16 of a key interval
    /// units (`index·16 + ⌊16·t⌋`). Not played by the port.
    pub triggers: Vec<u32>,
}

/// Quaternion component `s16 / 32768` (`vitof15`, exact).
pub fn quat_f32(q: [i16; 4]) -> [f32; 4] { q.map(|c| c as f32 / 32768.0) }

/// Parses a sequence at `offset` in `base`; frame pointers are relative to `base` (the class blob for
/// class sequences, the sequence blob itself for `ratchet_seq`).
pub fn parse_sequence(base: &[u8], offset: usize) -> Result<MobySequence> {
    let b = Buf(base);
    let header: MobySequenceHeader = b.pod(offset, "moby sequence header")?;
    let fc = header.frame_count as usize;
    let ptrs: Vec<u32> = b.pod_slice(offset + 0x1c, fc, "moby frame pointers")?;
    let triggers: Vec<u32> = b.pod_slice(offset + 0x1c + 4 * fc, header.trigger_count as usize, "moby sequence triggers")?;
    let mut frames = Vec::with_capacity(fc);
    for &p in &ptrs {
        if p >> 28 != 0 { return invalid(format!("moby sequence: frame pointer {p:#x} has a non-zero top nibble")); }
        frames.push(parse_frame(b, p as usize)?);
    }
    Ok(MobySequence { header, frames, triggers })
}

fn parse_frame(b: Buf, at: usize) -> Result<MobyFrame> {
    let h: MobyFrameHeader = b.pod(at, "moby frame header")?;
    let (qb, to, tc) = (h.quat_bytes as usize, h.trans_offset as usize, h.trans_count as usize);
    if !qb.is_multiple_of(8) || to < qb || !(to - qb).is_multiple_of(8) {
        return invalid(format!("moby frame at {at:#x}: quat bytes {qb:#x} / translation offset {to:#x} inconsistent"));
    }
    if (to - qb) / 8 != h.scale_count as usize { return invalid(format!("moby frame at {at:#x}: scale count mismatch")); }
    if h.qwc as usize != (to + 8 * tc + 15) >> 4 { return invalid(format!("moby frame at {at:#x}: qwc mismatch")); }
    let payload = b.sub(at + 0x10, h.qwc as usize * 16, "moby frame payload")?;
    Ok(MobyFrame {
        header: h,
        quats: payload.pod_slice(0, qb / 8, "moby frame quaternions")?,
        scales: payload.pod_slice(qb, (to - qb) / 8, "moby frame scale records")?,
        trans: payload.pod_slice(to, tc, "moby frame translation records")?,
        payload: payload.bytes().to_vec(),
    })
}

/// Sequences of a class from its pointer list at 0x48 (`MobyClass::sequence_pointers`); `None` for
/// empty (0) slots. Frame pointers are class-relative.
pub fn parse_sequences(class_blob: &[u8], class: &MobyClass) -> Result<Vec<Option<MobySequence>>> {
    class
        .sequence_pointers
        .iter()
        .map(|&p| if p <= 0 { Ok(None) } else { parse_sequence(class_blob, p as usize).map(Some) })
        .collect()
}

/// The 0x50-byte record a sequence header's +0x14 points to (class-relative), as words: the gait record of the level04
/// leg walker (`rc_game::moby_update::creature::legs`: +0x00 the sequence id, +0x04 the stride, +0x18 / +0x1c and
/// +0x28..+0x4c foot-contact key ranges; +0x08..+0x10 and +0x20..+0x3c are written by its set-up). Per sequence of the
/// class's pointer list with a non-zero pointer: (sequence index, record).
pub fn gait_records(class_blob: &[u8], class: &MobyClass) -> Vec<(u8, [u32; 20])> {
    let b = Buf(class_blob);
    class
        .sequence_pointers
        .iter()
        .enumerate()
        .filter(|(_, &p)| p > 0)
        .filter_map(|(i, &p)| {
            let h: MobySequenceHeader = b.pod(p as usize, "moby sequence header").ok()?;
            if h.trigger_data == 0 { return None; }
            let w: Vec<u32> = b.pod_slice(h.trigger_data as usize, 20, "gait record").ok()?;
            Some((i as u8, w.try_into().ok()?))
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------------
// Class data the evaluator needs

/// What `MobyAnimEval` reads from a class: the joint count of the high LOD (class byte 8), the skeleton
/// (inverse bind) matrices, `common_trans` (rest translation + parent SPR word) and the sequences.
#[derive(Clone, Debug)]
pub struct MobyAnimClass {
    pub joint_count: usize,
    /// Class `skeleton`, rows as stored (only the xyz of each row is read). Identity when absent.
    pub skeleton: Vec<Rows>,
    /// `common_trans[j].xyz`, the rest local translation (packed units).
    pub rest: Vec<[f32; 3]>,
    /// `common_trans[j]` word at +0xc: 0 for the root, else `0x70000000 + 0x40·parent`.
    pub parent_word: Vec<u32>,
    pub sequences: Vec<Option<MobySequence>>,
}

impl MobyAnimClass {
    pub fn new(class: &MobyClass, sequences: Vec<Option<MobySequence>>) -> Self {
        let jc = class.header.joint_count as usize;
        let sk = &class.skeleton;
        MobyAnimClass {
            joint_count: jc,
            skeleton: (0..jc).map(|j| sk.matrices.get(j).copied().unwrap_or(IDENTITY)).collect(),
            rest: (0..jc).map(|j| sk.trans.get(j).map(|t| t.vector).unwrap_or_default()).collect(),
            parent_word: (0..jc).map(|j| sk.trans.get(j).map(|t| t.parent_offset as u32 | (t.seventy as u32) << 16).unwrap_or(0)).collect(),
            sequences,
        }
    }

    pub fn sequence(&self, seq: u8) -> Option<&MobySequence> { self.sequences.get(seq as usize)?.as_ref() }
    pub fn frame(&self, seq: u8, index: u8) -> Option<&MobyFrame> { self.sequence(seq)?.frames.get(index as usize) }

    /// Parent joint from the SPR word (None for 0 = root, or a word that is not a joint record address).
    pub fn parent(&self, j: usize) -> Option<usize> { spr_record(*self.parent_word.get(j)?) }
}

fn spr_record(word: u32) -> Option<usize> {
    if word < SPR || !(word - SPR).is_multiple_of(0x40) { return None; }
    let i = ((word - SPR) / 0x40) as usize;
    (i < SPR_RECORDS).then_some(i)
}

// ---------------------------------------------------------------------------------------------------
// Runtime state and `MobyAnimAdvance` (§2, §3)

/// The animation fields of the runtime moby (moby+0x50..0x70, 0x7e).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimState {
    /// +0x52 / +0x50: sequence and frame index of key A (seq 0xff = snapshot slot, not ported).
    pub seq_a: u8,
    pub frame_a: u8,
    /// +0x53 / +0x51: key B.
    pub seq_b: u8,
    pub frame_b: u8,
    /// +0x54: blend A → B.
    pub t: f32,
    /// +0x58: speed multiplier (0 freezes, negative plays backwards).
    pub speed: f32,
    /// +0x5c: t increment per tick for the current interval.
    pub rate: f32,
    /// +0x70: set per tick: bit 0 = crossed a key, bit 1 = wrapped.
    pub flags: u8,
    /// +0x7e: trigger count of the current sequence (updated when a transition lands).
    pub trigger_count: u8,
    /// Mode bit 0x40: the update loop skips the advance entirely.
    pub skip_advance: bool,
}

impl AnimState {
    /// `init_moby_instance` (0x20c5f0): zeroed struct (seq 0, frames 0/0, t = 0), speed = rate = 1. A
    /// class with exactly one sequence of ≤ 1 frame gets speed 0, and mode 0x40 when that sequence's
    /// loop-sound byte has bit 7 set.
    pub fn spawn(class: &MobyAnimClass) -> AnimState {
        let mut s = AnimState { seq_a: 0, frame_a: 0, seq_b: 0, frame_b: 0, t: 0.0, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false };
        if class.sequences.len() == 1 {
            if let Some(q) = &class.sequences[0] {
                if q.header.frame_count <= 1 {
                    s.speed = 0.0;
                    s.skip_advance = q.header.loop_sound & 0x80 != 0;
                }
            }
        }
        s
    }
}

/// `fun_0020d580` MobyAnimAdvance: one 60 Hz tick. The sound trigger ([`advance_trigger`]) and the loop-sound
/// refresh are the caller's (they need the sound system: `rc_game::moby_update::anim_sound`). Returns false (state unchanged but `flags` cleared, as in the game) when the rate or the
/// speed is 0; also stops without touching the state when a sequence the step needs is missing (the game
/// would read through a null pointer).
pub fn advance(s: &mut AnimState, class: &MobyAnimClass) -> bool {
    let (sp, r) = (s.speed.to_bits(), s.rate.to_bits());
    s.flags = 0;
    if r == 0 { return false; }
    // Same sequence: adda.s ACC = 0 + t; madd.s t' = ACC + s·r. Transition: add.s t' = t + r.
    let mut t = if s.seq_a == s.seq_b { ps2::add(ps2::add(0, s.t.to_bits()), ps2::mul(sp, r)) } else { ps2::add(s.t.to_bits(), r) };
    if sp == 0 { return false; }
    let ti = t as i32;
    // Snap window: 0x3f7f0000 ≤ bits ≤ 0x3f808000 (signed compare; negatives never snap).
    let snap = (0x3f7f_0000..=0x3f80_8000).contains(&ti);
    let stepping = snap || ti > ONE as i32 || ti < 0;
    if !stepping {
        s.t = f32::from_bits(t);
        return true;
    }
    let Some(seq) = class.sequence(s.seq_b) else { return false };
    let fc = seq.header.frame_count as i32;
    let override_bits = seq.header.rate_override.to_bits();
    let mut r = r;
    let mut flags = 0u8;
    if snap || ti > ONE as i32 {
        if snap { t = ONE; }
        // Forward: repeat while t' > 1.0 (bits compare, no snap inside the loop).
        loop {
            t = ps2::sub(t, ONE);
            let new_a = s.frame_b;
            let mut new_b = s.frame_b as i32 + 1;
            if s.seq_a != s.seq_b {
                s.trigger_count = seq.header.trigger_count;
                s.seq_a = s.seq_b;
            }
            t = ps2::div(t, r);
            flags |= 1;
            if fc - new_b <= 0 { new_b = 0; flags |= 2; }
            // Rate: the sequence override, else the new key A's own rate (= the old B frame).
            r = if override_bits != 0 { override_bits } else {
                match seq.frames.get(new_a as usize) { Some(f) => f.header.rate.to_bits(), None => return false }
            };
            t = ps2::mul(t, r);
            s.frame_a = new_a;
            s.frame_b = new_b as u8;
            s.rate = f32::from_bits(r);
            s.t = f32::from_bits(t);
            if (t as i32) <= ONE as i32 { break; }
        }
    } else {
        // Backward: repeat while t' < 0 (sign bit, so −0.0 counts).
        loop {
            t = ps2::div(t, r);
            let new_b = s.frame_a;
            let mut new_a = s.frame_a as i32 - 1;
            flags |= 1;
            if new_a < 0 { new_a = fc - 1; flags |= 2; }
            r = if override_bits != 0 { override_bits } else {
                match seq.frames.get(new_a as usize) { Some(f) => f.header.rate.to_bits(), None => return false }
            };
            t = ps2::mul(t, r);
            s.frame_a = new_a as u8;
            s.frame_b = new_b;
            t = ps2::add(t, ONE);
            s.rate = f32::from_bits(r);
            s.t = f32::from_bits(t);
            if t & ps2::SIGN == 0 { break; }
        }
    }
    s.flags = flags;
    true
}

// ---------------------------------------------------------------------------------------------------
// Sequence changes (§4) and the pose snapshot

/// Key-A sequence id meaning "the snapshot frame" (`fun_00212f90` stores 0xff in +0x52 and the snapshot
/// slot in +0x50; `update_moby_animation_state` then points frame A at `0x1aabc0 + slot·0x800`). The port
/// keeps the snapshot frame next to the state and passes it to [`evaluate_with_snapshot`].
pub const SNAPSHOT_SEQ: u8 = 0xff;

/// `fun_00212ed8` (L01 0x26c5a8), the hard cut: A = B = `seq`; A.index = `frame` if `frame` < frame
/// count, else count − 1; B.index = A.index + 1 clamped to count − 1 (not wrapped; count 0 gives 0);
/// rate = frame A's own rate word (the sequence's rate override is not consulted here); flag bit 1
/// cleared; t and speed kept. Returns false, changing nothing, when the class has no sequence `seq` (the
/// game would dereference a null pointer).
pub fn hard_cut(s: &mut AnimState, class: &MobyAnimClass, seq: u8, frame: i32) -> bool {
    let Some(q) = class.sequence(seq) else { return false };
    let fc = q.header.frame_count as i32;
    let a = if frame < fc { frame } else { fc - 1 } as u8;
    let mut b = a.wrapping_add(1);
    if fc - 1 < b as i32 { b = (fc - 1) as u8; }
    if fc <= b as i32 { b = 0; }
    let Some(fa) = q.frames.get(a as usize) else { return false };
    s.seq_a = seq;
    s.frame_a = a;
    s.seq_b = seq;
    s.frame_b = b;
    s.rate = fa.header.rate;
    s.flags &= !2;
    // update_moby_animation_state (0x263718): +0x7e = key A's trigger count (+0x7c, the loop sound: the moby's).
    s.trigger_count = q.header.trigger_count;
    true
}

/// `fun_00212f90` (L01 0x26c660), the blend `(seq, frame, blend_ticks)`: B.index = `frame` clamped to the
/// target's frame count − 1. If t > 0.025 (the pose layers +0x60/+0x64 are null in the port), the current
/// pose is re-encoded into a snapshot frame ([`snapshot`], `fun_0020ede8` with flags 0x300 = no layers),
/// which becomes key A (`seq_a` = [`SNAPSHOT_SEQ`]); otherwise key A is left as it is (possibly another
/// sequence). Then seq B = `seq`, speed = 1, t = 0, flag bit 1 cleared and rate = 1 / `blend_ticks`
/// (`cvt.s.w` then `div.s`; 0 ticks gives the FPU's +MAX, i.e. the next advance lands on the target at
/// once, one key interval already started). +0x7e (trigger count) is set as `update_moby_animation_state` does; the
/// loop-sound byte +0x7c lives on the moby ([`loop_sound_of`]). `snapshot` holds
/// the moby's current snapshot frame (read when key A is one) and receives the new one. Returns false,
/// changing nothing, when the class has no sequence `seq`.
pub fn set_sequence(s: &mut AnimState, class: &MobyAnimClass, seq: u8, frame: i32, blend_ticks: i32, snapshot_frame: &mut Option<MobyFrame>) -> bool {
    set_sequence_ex(s, class, seq, frame, blend_ticks, snapshot_frame, false)
}

/// `MobyAnimBlendEx` 0x26c7a8 (boot `0x2130d8`): [`set_sequence`] whose snapshot is also taken when no blend runs
/// with `force` (its flag 4). Its flags 1 / 2 (the snapshot's 0x100 / 0x200: skip the pose-layer lists) change nothing
/// here (the port's snapshots carry no layers). [L] The frame is clamped as the plain blend does (the game passes it
/// through; every caller passes 0).
pub fn set_sequence_ex(s: &mut AnimState, class: &MobyAnimClass, seq: u8, frame: i32, blend_ticks: i32, snapshot_frame: &mut Option<MobyFrame>, force: bool) -> bool {
    let Some(q) = class.sequence(seq) else { return false };
    let fc = q.header.frame_count as i32;
    let b = if frame < fc { frame } else { fc - 1 } as u8;
    if s.t > 0.025 || force {
        // find_or_allocate_id_slot: the port always has a slot (the game falls back to "A unchanged").
        if let Some(f) = snapshot(class, s, snapshot_frame.as_ref()) {
            *snapshot_frame = Some(f);
            s.seq_a = SNAPSHOT_SEQ;
            s.frame_a = 0;
        }
    }
    s.frame_b = b;
    s.seq_b = seq;
    // update_moby_animation_state (0x263718): +0x7e = key A's trigger count, 0 for a snapshot key.
    s.trigger_count = if s.seq_a == SNAPSHOT_SEQ { 0 } else { class.sequence(s.seq_a).map_or(0, |a| a.header.trigger_count) };
    s.speed = 1.0;
    s.t = 0.0;
    s.flags &= !2;
    s.rate = f32::from_bits(ps2::div(ONE, (blend_ticks as f32).to_bits()));
    true
}

/// The sound trigger of one `MobyAnimAdvance` (0x265260, its tail): with key A and key B on the same sequence
/// before the advance (`before`) and a trigger count +0x7e, the first trigger word of the sequence (lo16 class
/// sound, hi16 time in 1/16 frames, unsigned) with `16·frame_a₀ + trunc(16·t₀) < time ≤ 16·frame_a₁ + trunc(16·t₁)`
/// (`after` = the state the advance left) fires: the game plays `PlayClassSound(sound, 0, moby)` and returns (no
/// loop-sound refresh that tick). A tick whose advance landed a transition (key A ≠ key B before) never fires.
pub fn advance_trigger(before: &AnimState, after: &AnimState, class: &MobyAnimClass) -> Option<u16> {
    if before.seq_a != before.seq_b || before.trigger_count == 0 { return None; }
    let seq = class.sequence(before.seq_b)?;
    let hi = after.frame_a as i32 * 16 + (after.t * 16.0) as i32;
    let lo = before.frame_a as i32 * 16 + (before.t * 16.0) as i32;
    if hi - lo <= 0 { return None; }
    seq.triggers.iter().take(before.trigger_count as usize).find(|&&w| {
        let time = (w >> 16) as i32;
        lo < time && time <= hi
    }).map(|&w| w as u16)
}

/// Whether a `MobyAnimAdvance` landed a transition (key A ≠ key B before, equal after): the game then returns
/// before the trigger check and the loop-sound refresh (it sets its +0x7c word to 0xffff for the rest of the call).
pub fn advance_landed(before: &AnimState, after: &AnimState) -> bool { before.seq_a != before.seq_b && after.seq_a == after.seq_b }

/// The loop-sound byte a sequence change leaves in moby +0x7c: `update_moby_animation_state` takes key A's
/// (0xff for a snapshot key), and the blend `fun_00212f90` then stores key B's; both equal key B's sequence
/// header +0x11 after a hard cut (A = B) or a blend. 0xff when the sequence is missing.
pub fn loop_sound_of(class: &MobyAnimClass, seq: u8) -> u8 { class.sequence(seq).map_or(0xff, |q| q.header.loop_sound) }

/// VU `ftoiN`: `x · 2^n` truncated toward zero, saturated to the s32 range (exponent 0 = 0; the PS2's
/// exponent-255 values are ordinary huge numbers and saturate).
fn ftoi(x: u32, n: i32) -> i32 {
    let e = ((x >> 23) & 0xff) as i32;
    if e == 0 { return 0; }
    let neg = x & ps2::SIGN != 0;
    if e == 255 { return if neg { i32::MIN } else { i32::MAX }; }
    let v = (f32::from_bits(x & !ps2::SIGN) as f64) * (2f64).powi(n);
    let v = if neg { -v.trunc() } else { v.trunc() };
    v.clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

/// `fun_0020ede8` (boot; L01 0x2666c8 region) with flags 0x300 as `fun_00212f90` calls it: re-encodes the
/// current local pose as a keyframe (the "snapshot" blends start from). It decodes and interpolates on its
/// own, not through `MobyAnimEval`:
/// * joint records: +0x00 = +0x10 = (1, 1, 1, 0); +0x20 = `common_trans[j]` (w = parent word),
///   +0x30 = its xyz · 1.0 (w 0);
/// * t bits = 0: scale records → +0x10 (w = (flag byte >> 6) − 1), translations → +0x20 (parent kept),
///   quaternions → +0x00;
/// * two keys (u = 1 − t): A scales → +0x00 (w = −4 marks "listed"), A translations → +0x20 (+0x3c = −4),
///   B scales → +0x10, B translations → +0x30, each joint appended to its list unless A listed it (the −4
///   test; a joint repeated in B is appended again). Every listed scale becomes `A·u + B·t` in +0x10 and
///   every listed translation `A·u + B·t` in +0x20, in list order, with the next record's operands loaded
///   before the current store (a repeated joint uses the value from before the previous pass only when
///   adjacent). Quaternions: consecutive keys → `A·u + B·t` unnormalised; otherwise `a = A·u, b = B·t`,
///   `d = ((a·b).y + (a·b).x) + (a·b).z + (a·b).w` on these scaled values, `q = d < 0 ? a − b : a + b`,
///   `q *= rsqrt(((x² + y²) + z²) + w²)`;
/// * no pose layers (flags 0x200 / 0x100 skip the +0x60 / +0x64 lists);
/// * encode, per joint: quaternion `vftoi15` clamped to [−0x8000, 0x7fff]; scale `vftoi15 >> 3` (logical)
///   clamped to [0, 0xffff], written as a record (joint, flag `(w & 0x80) ^ 0x80`) only when xyz ≠ 0x1000;
///   translation `vftoi0` (low 16 bits) written only when it differs from `vftoi0(common_trans)`.
///
/// The frame's rate and time are 0 (never read: a snapshot is always key A of a transition). Scale flags:
/// 0x80 = inherited, 0 = post-scale, as on the disc. `snap` is the current snapshot when key A is one.
/// None when a key the state needs is missing (the game would read garbage).
pub fn snapshot(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>) -> Option<MobyFrame> {
    use ps2::{add, mul, sub};
    let jc = class.joint_count;
    let fa = if s.seq_a == SNAPSHOT_SEQ { snap? } else { class.frame(s.seq_a, s.frame_a)? };
    let t = s.t.to_bits();
    let fb = if t != 0 { Some(class.frame(s.seq_b, s.frame_b)?) } else { None };
    const N: usize = 256;
    let unit = [ONE, ONE, ONE, 0];
    let mut rec = [[[0u32; 4]; 4]; N];
    for (j, r) in rec.iter_mut().enumerate().take(jc.min(N)) {
        let c = bits3w(class.rest[j], class.parent_word[j]);
        *r = [unit, unit, c, [mul(c[0], ONE), mul(c[1], ONE), mul(c[2], ONE), 0]];
    }
    let flag_word = |flags: u8| ((flags >> 6) as u32).wrapping_sub(1);
    let scale_v = |r: &ScaleRec| r.value().map(f32::to_bits);
    let trans_v = |r: &TransRec| r.trans.map(|c| (c as f32).to_bits());
    let q_i15 = |q: [i16; 4]| q.map(|c| (c as f32 / 32768.0).to_bits());
    const LISTED: u32 = (-4i32) as u32;
    match fb {
        None => {
            for r in &fa.scales {
                let v = scale_v(r);
                rec[r.joint as usize][1] = [v[0], v[1], v[2], flag_word(r.flags)];
            }
            for r in &fa.trans {
                let j = r.joint as usize;
                let v = trans_v(r);
                rec[j][2] = [v[0], v[1], v[2], rec[j][2][3]];
            }
            for (j, r) in rec.iter_mut().enumerate().take(jc.min(N)) { r[0] = q_i15(fa.quat_at(j)); }
        }
        Some(fb) => {
            let u = sub(ONE, t);
            let (mut sl, mut tl) = (Vec::new(), Vec::new());
            for r in &fa.scales {
                let j = r.joint as usize;
                let v = scale_v(r);
                sl.push(j);
                rec[j][1][3] = flag_word(r.flags);
                rec[j][0] = [v[0], v[1], v[2], LISTED];
            }
            for r in &fa.trans {
                let j = r.joint as usize;
                let v = trans_v(r);
                tl.push(j);
                rec[j][2] = [v[0], v[1], v[2], rec[j][2][3]];
                rec[j][3][3] = LISTED;
            }
            for r in &fb.scales {
                let j = r.joint as usize;
                let v = scale_v(r);
                if rec[j][0][3] != LISTED { sl.push(j); }
                rec[j][1] = [v[0], v[1], v[2], flag_word(r.flags)];
            }
            for r in &fb.trans {
                let j = r.joint as usize;
                let v = trans_v(r);
                if rec[j][3][3] != LISTED { tl.push(j); }
                rec[j][3] = [v[0], v[1], v[2], ONE];
            }
            // The pipelined interpolation loops (0x20f1a8 / 0x20f1f8): operands of entry i+1 are loaded
            // before entry i is stored.
            // Slots a (A) and b (B); the result goes to `dst` with that slot's w word (flag / parent) kept.
            let pass = |rec: &mut [[V4; 4]; N], list: &[usize], a: usize, b: usize, dst: usize| {
                let Some(&first) = list.first() else { return };
                let (mut va, mut vb) = (rec[first][a], rec[first][b]);
                for (i, &j) in list.iter().enumerate() {
                    let r = lanes(|k| if k < 3 { add(mul(va[k], u), mul(vb[k], t)) } else { 0 });
                    let w = rec[j][dst][3];
                    if let Some(&n) = list.get(i + 1) { va = rec[n][a]; vb = rec[n][b]; }
                    rec[j][dst] = [r[0], r[1], r[2], w];
                }
            };
            pass(&mut rec, &sl, 0, 1, 1);
            pass(&mut rec, &tl, 2, 3, 2);
            let plain = consecutive(s);
            for (j, r) in rec.iter_mut().enumerate().take(jc.min(N)) {
                let (qa, qb) = (quat_bits(fa.quat_at(j)), quat_bits(fb.quat_at(j)));
                r[0] = if plain {
                    lerp(qa, qb, u, t, 4, qa)
                } else {
                    let a = lanes(|k| mul(qa[k], u));
                    let b = lanes(|k| mul(qb[k], t));
                    let p = lanes(|k| mul(a[k], b[k]));
                    let d = add(add(add(p[1], p[0]), mul(ONE, p[2])), mul(ONE, p[3]));
                    let q = if d & ps2::SIGN == 0 { lanes(|k| add(a[k], b[k])) } else { lanes(|k| sub(a[k], b[k])) };
                    let sq = lanes(|k| mul(q[k], q[k]));
                    let n = add(add(add(sq[0], sq[1]), mul(ONE, sq[2])), mul(ONE, sq[3]));
                    let qq = ps2::rsqrt(ONE, n);
                    lanes(|k| mul(q[k], qq))
                };
            }
        }
    }
    // Encode (0x20f468..0x20f610).
    let mut payload = Vec::new();
    let (mut scales, mut trans) = (Vec::new(), Vec::new());
    for (j, r) in rec.iter().enumerate().take(jc.min(N)) {
        let q = r[0].map(|c| ftoi(c, 15).clamp(-0x8000, 0x7fff) as i16);
        for c in q { payload.extend_from_slice(&c.to_le_bytes()); }
        let sc = [0, 1, 2].map(|k| (((ftoi(r[1][k], 15) as u32) >> 3) as i32).clamp(0, 0xffff) as u16);
        if sc != [0x1000; 3] {
            scales.push(ScaleRec { scale: sc, joint: j as u8, flags: ((r[1][3] & 0x80) ^ 0x80) as u8 });
        }
        let tv = [0, 1, 2].map(|k| ftoi(r[2][k], 0) as i16);
        let rest = [0, 1, 2].map(|k| ftoi(class.rest[j][k].to_bits(), 0) as i16);
        if tv != rest { trans.push(TransRec { trans: tv, joint: j as u8, pad: 0 }); }
    }
    let quat_bytes = 8 * jc.min(N);
    for r in &scales { payload.extend_from_slice(bytemuck::bytes_of(r)); }
    for r in &trans { payload.extend_from_slice(bytemuck::bytes_of(r)); }
    payload.extend_from_slice(&1u64.to_le_bytes()); // the terminator word the encoder appends
    let qwc = ((quat_bytes + 8 * scales.len() + 8 * trans.len() + 8) >> 4) as u16;
    payload.resize(qwc as usize * 16, 0);
    let header = MobyFrameHeader {
        rate: 0.0,
        time: 0,
        qwc,
        quat_bytes: quat_bytes as u16,
        scale_count: scales.len() as u16,
        trans_offset: (quat_bytes + 8 * scales.len()) as u16,
        trans_count: trans.len() as u16,
    };
    let quats = (0..jc.min(N)).map(|j| { let o = 8 * j; [0, 1, 2, 3].map(|k| i16::from_le_bytes([payload[o + 2 * k], payload[o + 2 * k + 1]])) }).collect();
    Some(MobyFrame { header, quats, scales, trans, payload })
}

// ---------------------------------------------------------------------------------------------------
// `MobyAnimEval` (§6)

fn lanes(f: impl Fn(usize) -> u32) -> V4 { [f(0), f(1), f(2), f(3)] }
fn bits4(v: [f32; 4]) -> V4 { v.map(f32::to_bits) }
fn bits3w(v: [f32; 3], w: u32) -> V4 { [v[0].to_bits(), v[1].to_bits(), v[2].to_bits(), w] }
/// `vmula ACC, a, u; vmadd r, b, t` on the first `n` lanes; other lanes keep `keep`.
fn lerp(a: V4, b: V4, u: u32, t: u32, n: usize, keep: V4) -> V4 {
    lanes(|k| if k < n { ps2::add(ps2::mul(a[k], u), ps2::mul(b[k], t)) } else { keep[k] })
}
/// `vmulax ACC, m0, v.x; vmadday ACC, m1, v.y; vmaddz r, m2, v.z` (all four lanes).
fn mat3(m0: V4, m1: V4, m2: V4, v: V4) -> V4 {
    lanes(|k| ps2::add(ps2::add(ps2::mul(m0[k], v[0]), ps2::mul(m1[k], v[1])), ps2::mul(m2[k], v[2])))
}
/// `... vmaddaz ACC, m2, v.z; vmaddw r, m3, vf0.w`.
fn mat4_point(m: &[V4; 4], v: V4) -> V4 {
    lanes(|k| ps2::add(ps2::add(ps2::add(ps2::mul(m[0][k], v[0]), ps2::mul(m[1][k], v[1])), ps2::mul(m[2][k], v[2])), ps2::mul(m[3][k], ONE)))
}
fn quat_bits(q: [i16; 4]) -> V4 { bits4(quat_f32(q)) }
fn scale_bits(r: &ScaleRec) -> [f32; 3] { r.value() }
fn trans_value(r: &TransRec) -> [f32; 3] { r.trans.map(|c| c as f32) }
const UNIT_SCALE: V4 = [ONE, ONE, ONE, 0];
const ID4: [V4; 4] = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

/// The post-scale (non-inherited) list of the two-key path, exactly as `fun_0020e0e0` links it: A
/// records are appended in disc order (each append writes the link into the previous node, then the
/// record store overwrites the new node's own link word with the raw record bits). B records are
/// appended when the *B slot's* w word is still 0, which is always true on first sight of a joint in B,
/// so a joint already on the list from A is appended again and the node after it in the A chain is cut
/// off. The terminator zeroes the last node's link. Returns the joints reachable from the head, in walk
/// order (only membership matters: each node scales only its own joint).
pub fn post_scale_list(a: &[u8], b: &[u8]) -> Vec<u8> {
    #[derive(Clone, Copy, PartialEq)]
    enum Link { Zero, Raw, To(u8) }
    let mut head = Link::Zero;
    let mut link = [Link::Zero; 256];
    let mut last: Option<u8> = None;
    let set = |head: &mut Link, link: &mut [Link; 256], last: Option<u8>, v: Link| match last {
        None => *head = v,
        Some(n) => link[n as usize] = v,
    };
    for &j in a {
        set(&mut head, &mut link, last, Link::To(j));
        last = Some(j);
        link[j as usize] = Link::Raw; // sqc2 of the record: w = (joint | flags << 8) << 3 ≠ 0
    }
    let mut b_slot_written = [false; 256];
    for &j in b {
        let was = b_slot_written[j as usize];
        b_slot_written[j as usize] = true;
        if !was {
            set(&mut head, &mut link, last, Link::To(j));
            last = Some(j);
        }
    }
    set(&mut head, &mut link, last, Link::Zero);
    let mut out = Vec::new();
    let mut cur = head;
    while let Link::To(j) = cur {
        out.push(j);
        cur = link[j as usize];
        if out.len() > 512 { break; } // cannot happen (see module test), guards a corrupted list
    }
    out
}

/// Whether A and B are consecutive keys of one sequence (MobyProc job+0x12 = 0x400): plain lerp.
pub fn consecutive(s: &AnimState) -> bool { s.seq_a == s.seq_b && s.frame_b as u32 == s.frame_a as u32 + 1 }

/// `fun_0020e0e0` MobyAnimEval: the joint palette `F_j = P_j·S_j` for the state (one matrix per class
/// joint; a single identity for a class without joints; identity rows for every joint when key A (or, for
/// t ≠ 0, key B) does not exist, e.g. classes with joints but no sequence 0).
pub fn evaluate(class: &MobyAnimClass, s: &AnimState) -> Vec<Rows> { evaluate_with_snapshot(class, s, None) }

/// [`evaluate`] for a state whose key A may be a snapshot frame (`seq_a` = [`SNAPSHOT_SEQ`], see
/// [`set_sequence`]): `snap` is then read as key A, exactly like a disc frame (MobyProc DMAs frame A from
/// the snapshot slot). Without a snapshot such a state evaluates to identity rows.
pub fn evaluate_with_snapshot(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>) -> Vec<Rows> {
    evaluate_layered(class, s, snap, &[])
}

/// A runtime pose layer of the `+0x60` list (§6.4; the 0x40-byte nodes at 0x18efc0 that `FUN_00263e08` links in,
/// e.g. Ratchet's weapon arm): its own two keys of the moby's class, `t`, the weight `+8`, and the joints it covers
/// (the second byte list of the class joint list the node was made for, `joint_list(..).1`).
#[derive(Clone, Copy, Debug)]
pub struct PoseLayer<'a> {
    pub joints: &'a [u8],
    pub seq_a: u8,
    pub frame_a: u8,
    pub seq_b: u8,
    pub frame_b: u8,
    pub t: f32,
    pub weight: f32,
    /// The node's `+0x18` retarget class and its key flags `+6` / `+7` (None: both keys are the moby's class's).
    pub alt: Option<AltKeys<'a>>,
}

/// A pose layer whose keys come from another, animation-only class (`FUN_00264220(node, seq, frame, buffer)`, called by
/// `FUN_00263f70` for a key whose flag `+6` / `+7` is set): the key's frame of `class` is rebuilt in the moby's
/// class's layout, joint `k` of `class` becoming the moby's joint `map[k]` (the byte table after the retarget class's
/// joint-list word, class `+0x1c` + 4, [`retarget_map`]; the other joints zero), its scale and translation records
/// copied as they are (they carry the moby's joint numbers). Ratchet's glove-holding layers use it (classes 1 / 2).
#[derive(Clone, Copy, Debug)]
pub struct AltKeys<'a> {
    pub class: &'a MobyAnimClass,
    pub map: &'a [u8],
    /// `+6` / `+7`: key A / key B is read from `class`.
    pub a: bool,
    pub b: bool,
}

/// The retarget table of an animation-only class (`FUN_00264220` reads `*(class +0x1c) + 4`): one target joint per
/// joint of the class (`joint_count` bytes after the word at the class's `joints` offset). None when it is out of the
/// blob or the class has no joints.
pub fn retarget_map(blob: &[u8], header: &crate::moby::MobyClassHeader) -> Option<Vec<u8>> {
    let base = usize::try_from(header.joints).ok().filter(|&b| b > 0)? + 4;
    blob.get(base..base + header.joint_count as usize).map(<[u8]>::to_vec)
}

/// One key of a pose layer: its frame, and for a retargeted key ([`AltKeys`]) the map from the frame's joints.
struct LayerKey<'f> {
    f: &'f MobyFrame,
    map: Option<&'f [u8]>,
}

impl LayerKey<'_> {
    /// The key's quaternion of the moby's joint `j` (a retargeted key: its class's joint `k` with `map[k] = j`, else 0).
    fn quat(&self, j: usize) -> [i16; 4] {
        match self.map {
            None => self.f.quat_at(j),
            Some(m) => m.iter().position(|&x| x as usize == j).map_or([0; 4], |k| self.f.quat_at(k)),
        }
    }
}

/// The layer's own local values for joint `j` (`FUN_00267770`'s decode, §6.2/§6.3 on the layer's keys): the quaternion,
/// the inherited scale when a key carries one for `j`, and the translation (the rest translation when neither key
/// has one: the decode starts from the class's rest pose).
fn layer_local(class: &MobyAnimClass, l: &PoseLayer, j: usize) -> Option<(V4, Option<[f32; 3]>, [f32; 3])> {
    let rest = *class.rest.get(j)?;
    let key = |seq: u8, frame: u8, alt: bool| -> Option<LayerKey> {
        match l.alt {
            Some(a) if alt => a.class.frame(seq, frame).map(|f| LayerKey { f, map: Some(a.map) }),
            _ => class.frame(seq, frame).map(|f| LayerKey { f, map: None }),
        }
    };
    let fa = key(l.seq_a, l.frame_a, l.alt.is_some_and(|a| a.a))?;
    let t = l.t.to_bits();
    let fb = if t != 0 { key(l.seq_b, l.frame_b, l.alt.is_some_and(|a| a.b)) } else { None };
    let u = ps2::sub(ONE, t);
    let find_s = |k: &LayerKey| k.f.scales.iter().find(|r| r.joint as usize == j && r.inherited()).map(scale_bits);
    let find_t = |k: &LayerKey| k.f.trans.iter().find(|r| r.joint as i8 >= 0 && r.joint as usize == j).map(trans_value);
    let lerp3 = |a: [f32; 3], b: [f32; 3]| -> [f32; 3] { let v = lerp(bits3w(a, 0), bits3w(b, 0), u, t, 3, [0; 4]); [0, 1, 2].map(|k| f32::from_bits(v[k])) };
    match fb {
        None => Some((quat_bits(fa.quat(j)), find_s(&fa), find_t(&fa).unwrap_or(rest))),
        Some(fb) => {
            let (qa, qb) = (quat_bits(fa.quat(j)), quat_bits(fb.quat(j)));
            let plain = l.seq_a == l.seq_b && l.frame_b as u32 == l.frame_a as u32 + 1;
            let q = if plain { lerp(qa, qb, u, t, 4, qa) } else { nlerp_flip(qa, qb, u, t) };
            let sc = match (find_s(&fa), find_s(&fb)) { (Some(a), Some(b)) => Some(lerp3(a, b)), (a, b) => a.or(b) };
            let tr = lerp3(find_t(&fa).unwrap_or(rest), find_t(&fb).unwrap_or(rest));
            Some((q, sc, tr))
        }
    }
}

/// [`evaluate_with_snapshot`] with the runtime pose layers of the `+0x60` list applied after the interpolation and
/// before the matrices (§6.4; `MobyAnimEvalChain` 0x268ee8 does the same for chains: [`evaluate_chains_layered`]):
/// for each joint a layer covers, `q = nlerp_flip(q, q_layer, 1 − w, w)`, the translation lerped by `w` toward the
/// layer's, and the inherited scale where the layer's keys carry one.
pub fn evaluate_layered(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>, layers: &[PoseLayer]) -> Vec<Rows> {
    evaluate_posed(class, s, snap, layers, &[])
}

/// A node of a moby's runtime **joint-modifier list** (moby `+0x64`, §6.4): what `AttachManipulator` 0x264370 links
/// in front of the list for one of the class's joint lists, and what the owner rewrites every tick (Ratchet's head
/// look and idle joint records 0x17ab00, his eyelids 0x140080, an NPC's head turn, …). `joint` is the joint the node
/// acts on: the first entry of the second byte list of that joint list (`pb[pb[0] + 4]`, [`list_target`]).
///
/// | node | field |
/// |---|---|
/// | +0x03 | `mode`: 0 composes (`q ← q ⊗ quat`, inherited scale `∘ scale`, translation `+ trans`), else blends toward the node by `weight` |
/// | +0x04 | target joint record |
/// | +0x0c | `weight` (mode ≠ 0) |
/// | +0x10 / +0x20 / +0x30 | `quat`, `scale`, `trans` |
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointModifier {
    pub joint: u8,
    pub mode: u8,
    pub weight: f32,
    pub quat: [f32; 4],
    pub scale: [f32; 3],
    pub trans: [f32; 3],
}

impl JointModifier {
    /// A composing node (mode 0) with the identity rotation, unit scale and no translation: what `AttachManipulator`
    /// leaves before the owner writes its values.
    pub fn compose(joint: u8) -> JointModifier { JointModifier { joint, mode: 0, weight: 0.0, quat: [0.0, 0.0, 0.0, 1.0], scale: [1.0; 3], trans: [0.0; 3] } }
}

/// The joint a joint-modifier (or pose-layer) node made for a class joint list acts on: the first entry of the list's
/// second byte list (`AttachManipulator` 0x264370: `pb[pb[0] + 4]`, `pb` = the list, `pb[0]` = the low byte of its
/// first count). `second` = [`crate::gadget::joint_list`]`(..).1`.
pub fn list_target(second: &[u8]) -> Option<u8> { second.first().copied() }

/// The Hamilton product `a ⊗ b` (`fun_001fa3c0` 0x221d78 and the evaluator's composing modifier): xyz =
/// `a.w·b + b.w·a + a × b`, w = `a.w·b.w − a·b`.
pub fn quat_product(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + b[3] * a[0] + (a[1] * b[2] - a[2] * b[1]),
        a[3] * b[1] + b[3] * a[1] + (a[2] * b[0] - a[0] * b[2]),
        a[3] * b[2] + b[3] * a[2] + (a[0] * b[1] - a[1] * b[0]),
        a[3] * b[3] - (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]),
    ]
}

/// The joint-modifier list (§6.4) on the local records `[q, inherited scale, translation, _]`, in list order (head
/// first), after the pose layers and before the matrices (`MobyProc` 0x267fc0 at 0x268b00, `MobyAnimEvalChain`
/// 0x268ee8). Native `f32` arithmetic on the record values (the game's rules, not its VU0 rounding):
/// * a node whose joint is not below `limit` is skipped (`MobyProc`: the class's joint count);
/// * mode 0: `q ← q ⊗ q_m` (not normalised), scale xyz `← s ∘ s_m`, translation xyz `← t + t_m`; the scale's
///   presence word is kept (a joint without an inherited scale ignores `s_m`);
/// * mode ≠ 0, `w` = the node's weight: `a = q·(1 − w)`, `b = q_m·w`, `q ← normalise(a ± b)` (− when `a·b < 0`),
///   scale and translation xyz `← x·(1 − w) + x_m·w`;
/// * `mark_scale` (the chain evaluator): the scale's presence word is set afterwards (`sw 1, +0x1c`).
fn apply_modifiers(mods: &[JointModifier], rec: &mut [[V4; 4]], limit: usize, mark_scale: bool) {
    let f = |v: V4| v.map(f32::from_bits);
    for m in mods {
        let j = m.joint as usize;
        if j >= limit.min(rec.len()) { continue; }
        let (q, s, t) = (f(rec[j][0]), f(rec[j][1]), f(rec[j][2]));
        let (nq, ns, nt) = if m.mode == 0 {
            (quat_product(q, m.quat), [0, 1, 2].map(|k| s[k] * m.scale[k]), [0, 1, 2].map(|k| t[k] + m.trans[k]))
        } else {
            let (w, u) = (m.weight, 1.0 - m.weight);
            let a = q.map(|x| x * u);
            let b = m.quat.map(|x| x * w);
            let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
            let v: [f32; 4] = std::array::from_fn(|k| if d < 0.0 { a[k] - b[k] } else { a[k] + b[k] });
            let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3]).sqrt();
            (v.map(|x| x / n), [0, 1, 2].map(|k| s[k] * u + m.scale[k] * w), [0, 1, 2].map(|k| t[k] * u + m.trans[k] * w))
        };
        rec[j][0] = nq.map(f32::to_bits);
        let sw = if mark_scale { 1 } else { rec[j][1][3] };
        rec[j][1] = [ns[0].to_bits(), ns[1].to_bits(), ns[2].to_bits(), sw];
        rec[j][2] = [nt[0].to_bits(), nt[1].to_bits(), nt[2].to_bits(), rec[j][2][3]];
    }
}

/// [`evaluate_layered`] with the moby's runtime joint-modifier list (`+0x64`, [`JointModifier`], head first) applied
/// after the pose layers (`MobyProc` 0x267fc0): the full pose of a moby with both runtime lists.
pub fn evaluate_posed(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>, layers: &[PoseLayer], mods: &[JointModifier]) -> Vec<Rows> {
    let jc = class.joint_count;
    if jc == 0 { return vec![IDENTITY]; }
    let t = s.t.to_bits();
    let fa = if s.seq_a == SNAPSHOT_SEQ { snap } else { class.frame(s.seq_a, s.frame_a) };
    let Some(fa) = fa else { return vec![IDENTITY; jc] };
    let fb = if t != 0 { match class.frame(s.seq_b, s.frame_b) { Some(f) => Some(f), None => return vec![IDENTITY; jc] } } else { None };
    let u = ps2::sub(ONE, t); // vsuby.x vf6, vf4 (1), vf7 (t)

    // SPR joint records: [0] quat (or inherited scale A before the quats), [1] inherited scale (B
    // slot; w ≠ 0 = present), [2] translation A (w = parent word), [3] translation B.
    let mut rec = [[[0u32; 4]; 4]; SPR_RECORDS];
    let mut post_a = [UNIT_SCALE; 256];
    let mut post_b = [UNIT_SCALE; 256];
    for (j, r) in rec.iter_mut().enumerate().take(jc) {
        let rest = bits3w(class.rest[j], class.parent_word[j]);
        *r = [UNIT_SCALE, UNIT_SCALE, rest, [rest[0], rest[1], rest[2], 0]];
    }
    let valid = |j: usize| j < jc && j < SPR_RECORDS;
    let post_list: Vec<u8>;

    match fb {
        None => {
            // Single-key path (t bits = 0): no lerp, quats raw (unnormalised).
            let mut listed = Vec::new();
            for sr in &fa.scales {
                let j = sr.joint as usize;
                let v = scale_bits(sr);
                if sr.inherited() {
                    // Stored straight into +0x10 with w = the raw record bits (never 0).
                    if valid(j) { rec[j][1] = bits3w(v, ((sr.joint as u32) | (sr.flags as u32) << 8) << 3); }
                } else {
                    post_a[j] = bits3w(v, 0);
                    listed.push(sr.joint);
                }
            }
            post_list = listed;
            for tr in &fa.trans {
                let j = tr.joint as i8;
                if j >= 0 && valid(j as usize) {
                    let v = trans_value(tr);
                    let w = rec[j as usize][2][3];
                    rec[j as usize][2] = bits3w(v, w);
                }
            }
            for (j, r) in rec.iter_mut().enumerate().take(jc) { r[0] = quat_bits(fa.quat_at(j)); }
        }
        Some(fb) => {
            // Two-key path. Channels of A, then B (§6.2).
            let mut inh = [false; 256];
            let mut tr_listed = [false; 256];
            let (mut post_a_order, mut post_b_order) = (Vec::new(), Vec::new());
            for sr in &fa.scales {
                let j = sr.joint as usize;
                let v = scale_bits(sr);
                if sr.inherited() {
                    if valid(j) { rec[j][0] = bits3w(v, 0); inh[j] = true; }
                } else {
                    post_a[j] = bits3w(v, 0);
                    post_a_order.push(sr.joint);
                }
            }
            for tr in &fa.trans {
                let j = tr.joint as i8;
                if j >= 0 && valid(j as usize) {
                    let w = rec[j as usize][2][3];
                    rec[j as usize][2] = bits3w(trans_value(tr), w);
                    tr_listed[j as usize] = true;
                }
            }
            for sr in &fb.scales {
                let j = sr.joint as usize;
                let v = scale_bits(sr);
                if sr.inherited() {
                    // Linked only when the A slot's w word is 0 (not yet on the list): a true union.
                    if valid(j) { rec[j][1] = bits3w(v, 0); inh[j] = true; }
                } else {
                    post_b[j] = bits3w(v, 0);
                    post_b_order.push(sr.joint);
                }
            }
            for tr in &fb.trans {
                let j = tr.joint as i8;
                if j >= 0 && valid(j as usize) {
                    let w = rec[j as usize][3][3];
                    rec[j as usize][3] = bits3w(trans_value(tr), w);
                    tr_listed[j as usize] = true; // link word test = "already linked or the tail": union
                }
            }
            post_list = post_scale_list(&post_a_order, &post_b_order);

            // Interpolation (§6.3): inherited scale → +0x10 with w = 1; translation → +0x20 (parent kept).
            for j in 0..jc.min(SPR_RECORDS) {
                if inh[j] { rec[j][1] = lerp(rec[j][0], rec[j][1], u, t, 3, [0, 0, 0, ONE]); }
                if tr_listed[j] { rec[j][2] = lerp(rec[j][2], rec[j][3], u, t, 3, rec[j][2]); }
            }
            let plain = consecutive(s);
            for (j, r) in rec.iter_mut().enumerate().take(jc) {
                let (qa, qb) = (quat_bits(fa.quat_at(j)), quat_bits(fb.quat_at(j)));
                r[0] = if plain { lerp(qa, qb, u, t, 4, qa) } else { nlerp_flip(qa, qb, u, t) };
            }
        }
    }

    apply_layers(class, layers, &mut rec[..jc.min(SPR_RECORDS)]);
    apply_modifiers(mods, &mut rec[..jc.min(SPR_RECORDS)], jc, false);

    // Local matrices and the chain (§6.5), joints in index order. `MobyProc`'s own build (0x268690..0x26876c) scales
    // the rows by the record's scale without testing its presence word: a joint without a scale record holds (1, 1, 1)
    // (the init), so only a modifier's or layer's scale on such a joint shows the difference (Ratchet's feet records at
    // 0.01 under the boots).
    for j in 0..jc.min(SPR_RECORDS) {
        let [q, sc, tr, _] = rec[j];
        let mut rows = quat_rows(q);
        for (i, row) in rows.iter_mut().enumerate() {
            for c in row.iter_mut().take(3) { *c = ps2::mul(*c, sc[i]); }
        }
        let pp: [V4; 4] = if tr[3] == 0 { ID4 } else {
            match spr_record(tr[3]) { Some(p) => rec[p], None => ID4 }
        };
        let r3 = [ps2::add(0, tr[0]), ps2::add(0, tr[1]), ps2::add(0, tr[2]), ONE];
        rec[j] = [
            mat3(pp[0], pp[1], pp[2], rows[0]),
            mat3(pp[0], pp[1], pp[2], rows[1]),
            mat3(pp[0], pp[1], pp[2], rows[2]),
            mat4_point(&pp, r3),
        ];
    }

    // Post-scale (§6.6), after the whole chain: children do not see it.
    for &j in &post_list {
        let j = j as usize;
        if !valid(j) { continue; }
        let sv = if t == 0 { post_a[j] } else { lerp(post_a[j], post_b[j], u, t, 3, post_a[j]) };
        for (row, &k) in rec[j].iter_mut().zip(&sv[..3]) {
            for c in row.iter_mut().take(3) { *c = ps2::mul(*c, k); }
        }
    }

    // Palette (§6.7): F.r_i = P0·S_i.x + P1·S_i.y + P2·S_i.z; F.r3 adds P3. S's w column is never read.
    (0..jc.min(SPR_RECORDS))
        .map(|j| {
            let p = rec[j];
            let sk = class.skeleton[j].map(bits4);
            let f = [
                mat3(p[0], p[1], p[2], sk[0]),
                mat3(p[0], p[1], p[2], sk[1]),
                mat3(p[0], p[1], p[2], sk[2]),
                mat4_point(&p, sk[3]),
            ];
            f.map(|r| r.map(f32::from_bits))
        })
        .collect()
}

/// The runtime pose layers (§6.4) on the local records `[q, inherited scale, translation, _]`, in list order.
fn apply_layers(class: &MobyAnimClass, layers: &[PoseLayer], rec: &mut [[V4; 4]]) {
    for l in layers {
        if l.weight <= 0.0 || l.weight.is_nan() { continue; }
        let (w, uw) = (l.weight.to_bits(), ps2::sub(ONE, l.weight.to_bits()));
        for &j in l.joints {
            let j = j as usize;
            if j >= rec.len() { continue; }
            let Some((q, sc, tr)) = layer_local(class, l, j) else { continue };
            rec[j][0] = nlerp_flip(rec[j][0], q, uw, w);
            if let Some(v) = sc {
                let cur = if rec[j][1][3] != 0 { rec[j][1] } else { [ONE, ONE, ONE, ONE] };
                rec[j][1] = lerp(cur, bits3w(v, ONE), uw, w, 3, [0, 0, 0, ONE]);
            }
            rec[j][2] = lerp(rec[j][2], bits3w(tr, 0), uw, w, 3, rec[j][2]);
        }
    }
}

/// Quaternion rows as 0x20eb2c builds them (every product is `(q_a + q_a)·q_b`; w lanes 0).
pub fn quat_rows(q: V4) -> [V4; 3] {
    use ps2::{add, mul, sub};
    let [x, y, z, w] = q;
    let (x2, y2, z2) = (add(x, x), add(y, y), add(z, z));
    let (xw2, yw2, zw2) = (mul(x2, w), mul(y2, w), mul(z2, w));
    let (xx2, yx2, zx2) = (mul(x2, x), mul(y2, x), mul(z2, x));
    let (yy2, zy2) = (mul(y2, y), mul(z2, y));
    let zz2 = mul(z2, z);
    [
        [sub(sub(ONE, yy2), zz2), sub(yx2, zw2), add(zx2, yw2), 0],
        [add(add(0, zw2), yx2), sub(sub(ONE, xx2), zz2), sub(zy2, xw2), 0],
        [add(sub(0, yw2), zx2), add(add(0, xw2), zy2), sub(sub(ONE, xx2), yy2), 0],
    ]
}

/// nlerp with hemisphere flip (0x20e5ac): `d = ((a.y·b.y + a.x·b.x) + 1·a.z·b.z) + 1·a.w·b.w`; if d's
/// sign bit is set use −b; `q = a·u + b·t`; `q *= rsqrt(((x² + y²) + 1·z²) + 1·w²)`.
pub fn nlerp_flip(a: V4, b: V4, u: u32, t: u32) -> V4 {
    use ps2::{add, mul};
    let p = lanes(|k| mul(a[k], b[k]));
    let nb = lanes(|k| mul(b[k], NEG_ONE));
    let d = add(add(add(p[1], p[0]), mul(ONE, p[2])), mul(ONE, p[3]));
    let q = if d & ps2::SIGN == 0 { lerp(a, b, u, t, 4, a) } else { lerp(a, nb, u, t, 4, a) };
    let sq = lanes(|k| mul(q[k], q[k]));
    let n = add(add(add(sq[0], sq[1]), mul(ONE, sq[2])), mul(ONE, sq[3]));
    let qq = ps2::rsqrt(ONE, n);
    lanes(|k| mul(q[k], qq))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(x: f32) -> u32 { x.to_bits() }
    fn q4(v: V4) -> [f32; 4] { v.map(f32::from_bits) }
    fn close(a: f32, b: f32, tol: f32) -> bool { (a - b).abs() <= tol }

    /// Class 66, frame 1, joint 0: the doc's 8 bytes `000064e5 0000337d`.
    const Q66_F1: [u8; 8] = [0x00, 0x00, 0x64, 0xe5, 0x00, 0x00, 0x33, 0x7d];
    const Q_ID: [i16; 4] = [0, 0, 0, 0x7fff];

    fn q66_f1() -> [i16; 4] { bytemuck::pod_read_unaligned(&Q66_F1) }

    #[test]
    fn class66_quaternion_decode_and_rows() {
        let q = q66_f1();
        assert_eq!(q, [0, -6812, 0, 32051]);
        let v = quat_f32(q);
        assert!(close(v[1], -0.207886, 1e-6) && close(v[3], 0.978119, 1e-6) && v[0] == 0.0 && v[2] == 0.0, "{v:?}");
        let r = quat_rows(bits4(v)).map(q4);
        assert!(close(r[0][0], 0.913567, 2e-6) && close(r[0][2], -0.406674, 2e-6), "{:?}", r[0]);
        assert!(close(r[2][0], 0.406674, 2e-6) && close(r[2][2], 0.913567, 2e-6), "{:?}", r[2]);
        // Identity quaternion (0x7fff = 0.99997) gives exact identity rows.
        assert_eq!(quat_rows(quat_bits(Q_ID)), [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0]]);
    }

    #[test]
    fn plain_lerp_is_not_normalised_and_wrap_flips() {
        let (a, b) = (quat_bits(Q_ID), quat_bits(q66_f1()));
        let (u, t) = (f(0.5), f(0.5));
        let q = q4(lerp(a, b, u, t, 4, a));
        let n = (q.iter().map(|x| x * x).sum::<f32>()).sqrt();
        assert!(close(q[1], -0.103943, 1e-6) && close(q[3], 0.989044, 1e-5), "{q:?}");
        assert!(close(n, 0.99449, 1e-5), "|q| = {n}");
        assert!(close(q4(quat_rows(bits4(q))[0])[0], 0.978392, 2e-6));
        // Wrap 14 → 0: q14 = (0, −6812, 0, −32051), dot with q0 < 0 → flip → (0, −0.104519, 0, −0.994523).
        let q14 = quat_bits([0, -6812, 0, -32051]);
        let w = q4(nlerp_flip(q14, a, u, t));
        assert!(close(w[1], -0.104519, 2e-6) && close(w[3], -0.994523, 2e-6) && w[0] == 0.0, "{w:?}");
    }

    fn frame(quats: Vec<[i16; 4]>, scales: Vec<ScaleRec>, trans: Vec<TransRec>, rate: f32) -> MobyFrame {
        let mut payload: Vec<u8> = bytemuck::cast_slice(&quats).to_vec();
        payload.extend_from_slice(bytemuck::cast_slice(&scales));
        let to = payload.len();
        payload.extend_from_slice(bytemuck::cast_slice(&trans));
        while !payload.len().is_multiple_of(16) { payload.push(0); }
        let header = MobyFrameHeader {
            rate, time: 0, qwc: (payload.len() / 16) as u16, quat_bytes: (quats.len() * 8) as u16,
            scale_count: scales.len() as u16, trans_offset: to as u16, trans_count: trans.len() as u16,
        };
        MobyFrame { header, quats, scales, trans, payload }
    }

    /// Class 747's numbers from the doc: 2 joints, j1's parent is j0, frame 1 has a post-scale on j0
    /// and a translation on j1.
    fn class747() -> MobyAnimClass {
        let mut s0 = IDENTITY;
        s0[3] = [0.0, 0.0, -831.386, 1.0];
        let mut s1 = IDENTITY;
        s1[3] = [0.0, 0.0, -27067.32, 1.0];
        let f0 = frame(vec![Q_ID, Q_ID], vec![], vec![], 0.25);
        let f1 = frame(
            vec![Q_ID, Q_ID],
            vec![ScaleRec { scale: [0x1000, 0x1000, 0x0cd6], joint: 0, flags: 0 }],
            vec![TransRec { trans: [0, 0, 21048], joint: 1, pad: 0 }],
            0.25,
        );
        let seq = MobySequence {
            header: MobySequenceHeader { frame_count: 2, loop_sound: 0xff, rate_override: 0.0, ..Default::default() },
            frames: vec![f0, f1],
            triggers: vec![],
        };
        MobyAnimClass {
            joint_count: 2,
            skeleton: vec![s0, s1],
            rest: vec![[0.0, 0.0, 831.386], [0.0, 0.0, 21119.06]],
            parent_word: vec![0, 0x7000_0000],
            sequences: vec![Some(seq)],
        }
    }

    #[test]
    fn class747_frame1_post_scale_and_chain() {
        let c = class747();
        let st = AnimState { seq_a: 0, frame_a: 1, seq_b: 0, frame_b: 1, t: 0.0, ..AnimState::spawn(&c) };
        let f = evaluate(&c, &st);
        assert!(close(f[0][2][2], 0.80225, 1e-4), "{:?}", f[0]);
        assert!(close(f[0][3][2], 164.41, 1e-2), "F0 z = {}", f[0][3][2]);
        assert!(close(f[1][3][2], -5187.94, 1e-2), "F1 z = {}", f[1][3][2]);
        assert_eq!(f[1][0][..3], [1.0, 0.0, 0.0]);
        assert_eq!(f[1][2][..3], [0.0, 0.0, 1.0], "the child ignores the parent's post-scale");
    }

    #[test]
    fn advance_from_spawn_and_wrap() {
        // 15 keys, rate override 0.125 (8 ticks per key): one cycle = 120 ticks, like class 66.
        let frames = (0..15).map(|_| frame(vec![Q_ID], vec![], vec![], 0.125)).collect();
        let seq = MobySequence { header: MobySequenceHeader { frame_count: 15, loop_sound: 0xff, rate_override: 0.125, ..Default::default() }, frames, triggers: vec![] };
        let c = MobyAnimClass { joint_count: 1, skeleton: vec![IDENTITY], rest: vec![[0.0; 3]], parent_word: vec![0], sequences: vec![Some(seq)] };
        let mut s = AnimState::spawn(&c);
        assert_eq!(s.speed, 1.0);
        advance(&mut s, &c);
        assert_eq!((s.frame_a, s.frame_b, s.t, s.rate, s.flags), (0, 1, 0.0, 0.125, 1));
        let mut wraps = 0;
        for tick in 2..=121 {
            advance(&mut s, &c);
            if s.flags & 2 != 0 { wraps += 1; assert_eq!(tick, 1 + 14 * 8, "wrap when B steps past the last key"); }
        }
        // After 120 more ticks: back at key 0 → 1 with t = 0.
        assert_eq!((s.frame_a, s.frame_b, s.t), (0, 1, 0.0));
        assert_eq!(wraps, 1);
        // Backwards: speed −1 from (0 → 1, t = 0) steps to (14 → 0).
        s.speed = -1.0;
        advance(&mut s, &c);
        assert_eq!((s.frame_a, s.frame_b, s.flags), (14, 0, 3));
        assert!(close(s.t, 0.875, 1e-6), "{}", s.t);
        // Static class: one sequence with ≤ 1 frame and no loop sound → speed 0 and mode 0x40.
        let st = MobySequence { header: MobySequenceHeader { frame_count: 1, loop_sound: 0xff, ..Default::default() }, frames: vec![frame(vec![Q_ID], vec![], vec![], 1.0)], triggers: vec![] };
        let sc = MobyAnimClass { sequences: vec![Some(st)], ..c.clone() };
        let s = AnimState::spawn(&sc);
        assert!(s.speed == 0.0 && s.skip_advance);
    }

    #[test]
    fn snap_window() {
        let frames = (0..3).map(|_| frame(vec![Q_ID], vec![], vec![], 0.3)).collect();
        let seq = MobySequence { header: MobySequenceHeader { frame_count: 3, ..Default::default() }, frames, triggers: vec![] };
        let c = MobyAnimClass { joint_count: 1, skeleton: vec![IDENTITY], rest: vec![[0.0; 3]], parent_word: vec![0], sequences: vec![Some(seq)] };
        // t' = 0.9975 lies in the snap window → exactly one key step with t = 0.
        let mut s = AnimState { seq_a: 0, frame_a: 0, seq_b: 0, frame_b: 1, t: 0.6975, speed: 1.0, rate: 0.3, flags: 0, trigger_count: 0, skip_advance: false };
        advance(&mut s, &c);
        assert_eq!((s.frame_a, s.frame_b, s.t), (1, 2, 0.0));
        // Below the window: no step.
        let mut s2 = AnimState { t: 0.6, frame_a: 0, frame_b: 1, ..s };
        advance(&mut s2, &c);
        assert_eq!((s2.frame_a, s2.flags), (0, 0));
    }

    #[test]
    fn post_scale_list_drops_a_only_joints() {
        // Same joints in A and B: full set.
        assert_eq!(post_scale_list(&[0, 1, 2], &[0, 1, 2]), [0, 1, 2]);
        // Disjoint: A then B (true union).
        assert_eq!(post_scale_list(&[0, 1], &[2, 3]), [0, 1, 2, 3]);
        // A = [0, 1, 3], B = [0, 2]: the re-append of 0 cuts 1 and 3 off.
        assert_eq!(post_scale_list(&[0, 1, 3], &[0, 2]), [0, 2]);
        // A = [1, 0], B = [0, 1]: 1's link is overwritten last with the terminator → only 1 remains.
        assert_eq!(post_scale_list(&[1, 0], &[0, 1]), [1]);
        // Only A or only B.
        assert_eq!(post_scale_list(&[4, 5], &[]), [4, 5]);
        assert_eq!(post_scale_list(&[], &[7]), [7]);
    }

    /// class747 plus a second sequence (3 keys, rate 0.5; joint 0 turned 24° about y in key 2).
    fn class747_two_seqs() -> MobyAnimClass {
        let mut c = class747();
        let q24 = bytemuck::pod_read_unaligned::<[i16; 4]>(&Q66_F1);
        let frames = vec![
            frame(vec![Q_ID, Q_ID], vec![], vec![], 0.5),
            frame(vec![Q_ID, Q_ID], vec![], vec![], 0.5),
            frame(vec![q24, Q_ID], vec![], vec![TransRec { trans: [0, 0, 21000], joint: 1, pad: 0 }], 0.75),
        ];
        c.sequences.push(Some(MobySequence { header: MobySequenceHeader { frame_count: 3, loop_sound: 0xff, ..Default::default() }, frames, triggers: vec![] }));
        c
    }

    #[test]
    fn hard_cut_clamps_and_takes_the_frame_rate() {
        let c = class747_two_seqs();
        let mut s = AnimState::spawn(&c);
        s.t = 0.3;
        s.flags = 3;
        assert!(hard_cut(&mut s, &c, 1, 0));
        assert_eq!((s.seq_a, s.frame_a, s.seq_b, s.frame_b, s.rate, s.t, s.flags), (1, 0, 1, 1, 0.5, 0.3, 1));
        // Past the end: A = last key, B clamped to it (no wrap).
        assert!(hard_cut(&mut s, &c, 1, 7));
        assert_eq!((s.frame_a, s.frame_b, s.rate), (2, 2, 0.75));
        assert!(!hard_cut(&mut s, &c, 5, 0));
    }

    #[test]
    fn blend_with_zero_ticks_lands_on_the_next_advance() {
        let c = class747_two_seqs();
        let mut s = AnimState::spawn(&c);
        advance(&mut s, &c); // the load pass: (0,0) -> (0,1), t = 0
        let mut snap = None;
        assert!(set_sequence(&mut s, &c, 1, 0, 0, &mut snap));
        assert!(snap.is_none(), "t = 0: no snapshot, key A kept");
        assert_eq!((s.seq_a, s.frame_a, s.seq_b, s.frame_b, s.t, s.speed), (0, 0, 1, 0, 0.0, 1.0));
        assert_eq!(s.rate.to_bits(), ps2::MAX, "1 / 0 on the FPU");
        advance(&mut s, &c);
        assert_eq!((s.seq_a, s.frame_a, s.seq_b, s.frame_b, s.t, s.rate), (1, 0, 1, 1, 0.5, 0.5));
        // Five ticks: rate 1/5, t reaches 1 after five advances (then the target's own rate).
        let mut s = AnimState::spawn(&c);
        advance(&mut s, &c);
        set_sequence(&mut s, &c, 1, 0, 5, &mut snap);
        for _ in 0..4 { advance(&mut s, &c); assert_eq!(s.seq_a, 0); }
        advance(&mut s, &c);
        assert_eq!((s.seq_a, s.frame_a, s.frame_b), (1, 0, 1));
    }

    #[test]
    fn snapshot_of_a_key_re_encodes_it_exactly() {
        let c = class747_two_seqs();
        let st = AnimState { seq_a: 0, frame_a: 1, seq_b: 0, frame_b: 0, t: 0.0, ..AnimState::spawn(&c) };
        let f = snapshot(&c, &st, None).unwrap();
        let disc = c.frame(0, 1).unwrap();
        assert_eq!(f.quats, disc.quats);
        assert_eq!(f.scales, disc.scales, "post-scale flag 0 kept");
        assert_eq!(f.trans, disc.trans);
        let snap_state = AnimState { seq_a: SNAPSHOT_SEQ, frame_a: 0, ..st };
        assert_eq!(evaluate_with_snapshot(&c, &snap_state, Some(&f)), evaluate(&c, &st));
        assert_eq!(evaluate(&c, &snap_state), vec![IDENTITY; 2], "no snapshot: identity");
    }

    #[test]
    fn blend_from_mid_key_starts_from_the_snapshot() {
        let c = class747_two_seqs();
        // Mid-interval of seq 0 (0 -> 1, t = 0.5): j0 post-scale halfway, j1 translation halfway.
        let mut s = AnimState { seq_a: 0, frame_a: 0, seq_b: 0, frame_b: 1, t: 0.5, rate: 0.25, ..AnimState::spawn(&c) };
        let before = evaluate(&c, &s);
        let mut snap = None;
        assert!(set_sequence(&mut s, &c, 1, 2, 4, &mut snap));
        let f = snap.clone().expect("t > 0.025 takes a snapshot");
        assert_eq!((s.seq_a, s.frame_a, s.seq_b, s.frame_b, s.t, s.rate), (SNAPSHOT_SEQ, 0, 1, 2, 0.0, 0.25));
        assert_eq!(f.scales.len(), 1);
        assert_eq!((f.scales[0].joint, f.scales[0].flags), (0, 0));
        assert_eq!(f.trans.len(), 1);
        assert_eq!((f.trans[0].joint, f.trans[0].trans[2]), (1, 21083)); // vftoi0(21083.53)
        let after = evaluate_with_snapshot(&c, &s, Some(&f));
        for (a, b) in before.iter().zip(&after) {
            for (ra, rb) in a.iter().zip(b) {
                for k in 0..3 { assert!(close(ra[k], rb[k], if ra[3] == 1.0 { 1.0 } else { 1e-3 }), "{before:?} vs {after:?}"); }
            }
        }
        // Halfway through the blend the snapshot (key A) and seq 1 key 2 (key B) are nlerped.
        advance(&mut s, &c);
        advance(&mut s, &c);
        assert_eq!((s.seq_a, s.t), (SNAPSHOT_SEQ, 0.5));
        let mid = evaluate_with_snapshot(&c, &s, Some(&f));
        let target = evaluate(&c, &AnimState { seq_a: 1, frame_a: 2, seq_b: 1, frame_b: 2, t: 0.0, ..s });
        assert!(mid[0][0][2] < 0.0 && mid[0][0][2] > target[0][0][2] && close(mid[0][0][0], 0.978148, 1e-3), "12 of 24 degrees: {:?} {:?}", mid[0][0], target[0][0]);
        // A second snapshot during the blend reads the first one as key A.
        let mut s2 = s;
        let mut snap2 = Some(f.clone());
        assert!(set_sequence(&mut s2, &c, 0, 0, 3, &mut snap2));
        assert_ne!(snap2.as_ref(), Some(&f));
    }
}

// ===================================================================================================
// Attachments: the partial pose `fun_00210850` / `fun_002109b8` and the attach matrices `fun_002646d0`
// (docs/formats/moby_rac1.md §0.4). Used by rc-engine `moby_attach` for the items on Ratchet.

/// SPR records the partial evaluator can address with a joint byte below 0x80 (its mark buffer is 0x80
/// bytes; the port ignores joint bytes >= 0x80, which would land in the DMA'd frame data).
const CHAIN_RECORDS: usize = 0x80;

/// A frame's scale and translation records as `fun_002109b8` reads them: `scale_count` (+0xa) then
/// `trans_count` (+0xe) consecutive 8-byte records from payload byte `8 · class joint count` (it does not
/// use the frame's `quat_bytes` / `trans_offset`). Bytes past the payload read as 0.
fn chain_records(f: &MobyFrame, jc: usize) -> (Vec<u64>, Vec<u64>) {
    let rd = |o: usize| -> u64 { u64::from_le_bytes(std::array::from_fn(|k| f.payload.get(o + k).copied().unwrap_or(0))) };
    let (ns, nt) = (f.header.scale_count as usize, f.header.trans_count as usize);
    let base = 8 * jc;
    ((0..ns).map(|i| rd(base + 8 * i)).collect(), (0..nt).map(|i| rd(base + 8 * (ns + i))).collect())
}

/// Record lanes: `pextlh` / `psrlw 13` / `vitof15` (scale, u16 / 4096) or `psraw 16` / `vitof0`
/// (translation, s16); the joint byte (+6) and the flag word `(record >> 62) − 1` (non-zero for scale
/// flags 0x80 / 0xc0).
fn rec_scale(r: u64) -> [u32; 3] { [0, 1, 2].map(|k| (((r >> (16 * k)) & 0xffff) as f32 / 4096.0).to_bits()) }
fn rec_trans(r: u64) -> [u32; 3] { [0, 1, 2].map(|k| (((r >> (16 * k)) & 0xffff) as u16 as i16 as f32).to_bits()) }
fn rec_joint(r: u64) -> usize { ((r >> 48) & 0xff) as usize }
fn rec_inherited(r: u64) -> bool { (r as i64) < 0 }
fn rec_flag_word(r: u64) -> u32 { ((r >> 62) as u32).wrapping_sub(1) }

/// `fun_00210850(moby, n, lists, out)`: the pose matrix `P_j` (joint local → packed model space, no
/// skeleton factor) of the **last** joint of each chain, where a chain is the first byte list of one of
/// the class's joint lists (spec 3.3, `gadget::joint_list`). It marks every joint of every chain in a
/// 0x80-byte buffer, sets the evaluation count to (largest last joint) + 1 and runs `fun_002109b8`, which
/// is `MobyAnimEval` (`fun_0020e0e0`, [`evaluate`]) restricted to that range and to the marked joints:
/// * channel decode and interpolation as in `fun_0020e0e0` (quaternions only for joints below the count),
///   but **non-inherited ("post") scale records are skipped** entirely, and the scale / translation
///   records are read from payload byte `8 · class joint count`;
/// * the plain-lerp test is `((B.index − A.index) & 0xfffe) + (A.seq − B.seq) == 0` (so equal key indices
///   also lerp plainly), not [`consecutive`];
/// * the chain runs over the marked joints in index order (`P_j = P_parent · L_j`, parent from the
///   `common_trans` SPR word); unmarked joints keep their decoded channel records, so a chain whose parent
///   is not marked would multiply with them, as the game does;
/// * no `P·S` step: the result is `P_j`, not the skinning palette `F_j = P_j · S_j` (`S_j` maps the bind
///   pose's model space to joint space; an attachment needs the joint's own frame in model space, which is
///   `P_j`).
///
/// `snap` is key A when `seq_a` is [`SNAPSHOT_SEQ`], as in [`evaluate_with_snapshot`]. Identity rows for
/// a class without joints, a missing key or an empty chain. The +0x60 pose layers: [`evaluate_chains_layered`];
/// with the +0x64 joint modifiers as well: [`evaluate_chains_posed`].
pub fn evaluate_chains(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>, chains: &[&[u8]]) -> Vec<Rows> {
    evaluate_chains_layered(class, s, snap, chains, &[])
}

/// [`evaluate_chains`] with the moby's +0x60 pose layers blended in after the interpolation (`MobyAnimEvalChain`
/// 0x268ee8 walks the list like `MobyProc`), e.g. Ratchet's hand under the weapon arm.
pub fn evaluate_chains_layered(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>, chains: &[&[u8]], layers: &[PoseLayer]) -> Vec<Rows> {
    evaluate_chains_posed(class, s, snap, chains, layers, &[])
}

/// [`evaluate_chains_layered`] with the moby's joint-modifier list (`+0x64`) after the layers, as
/// `MobyAnimEvalChain` 0x268ee8 applies it: every node (no joint-count test), and the modified joint's inherited
/// scale is marked present afterwards.
pub fn evaluate_chains_posed(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>, chains: &[&[u8]], layers: &[PoseLayer], mods: &[JointModifier]) -> Vec<Rows> {
    let fail = || vec![IDENTITY; chains.len()];
    let jc = class.joint_count;
    if jc == 0 || chains.iter().any(|c| c.is_empty()) { return fail(); }
    // fun_00210850: marks, the last joint of each chain, count = max(last) + 1 (pmaxw over the last joints).
    let mut mark = [false; CHAIN_RECORDS];
    let mut max_last = 0usize;
    for c in chains {
        for &j in c.iter() { if (j as usize) < CHAIN_RECORDS { mark[j as usize] = true; } }
        max_last = max_last.max(*c.last().unwrap() as usize);
    }
    let count = (max_last + 1).min(CHAIN_RECORDS);
    let t = s.t.to_bits();
    let fa = if s.seq_a == SNAPSHOT_SEQ { snap } else { class.frame(s.seq_a, s.frame_a) };
    let Some(fa) = fa else { return fail() };
    let fb = if t != 0 { match class.frame(s.seq_b, s.frame_b) { Some(f) => Some(f), None => return fail() } } else { None };
    let u = ps2::sub(ONE, t); // vsub.x vf6, vf4 (1), vf7 (t)

    // SPR records: [0] quat (A inherited scale before the quats; w = scale-list link), [1] inherited scale
    // (B slot; w = flag word), [2] translation A (w = parent word), [3] translation B (w = translation-list
    // link). Links are record numbers + 1 (the game stores SPR addresses; record 0's is non-zero too).
    let mut rec = [[[0u32; 4]; 4]; CHAIN_RECORDS];
    for (j, r) in rec.iter_mut().enumerate().take(jc.min(CHAIN_RECORDS)) {
        let c = bits3w(class.rest[j], class.parent_word[j]);
        *r = [UNIT_SCALE, UNIT_SCALE, c, [c[0], c[1], c[2], ps2::mul(c[3], 0)]];
    }
    let (sa, ta) = chain_records(fa, jc);
    match fb {
        None => {
            for &r in &sa {
                let j = rec_joint(r);
                if !rec_inherited(r) || j >= CHAIN_RECORDS { continue; }
                let v = rec_scale(r);
                rec[j][1] = [v[0], v[1], v[2], rec_flag_word(r)];
            }
            for &r in &ta {
                let j = rec_joint(r);
                if j >= CHAIN_RECORDS { continue; }
                let v = rec_trans(r);
                rec[j][3] = [0, 0, 0, ONE];
                rec[j][2] = [v[0], v[1], v[2], rec[j][2][3]];
            }
            for (j, r) in rec.iter_mut().enumerate().take(count) { r[0] = quat_bits(fa.quat_at(j)); }
        }
        Some(fb) => {
            let (sb, tb) = chain_records(fb, jc);
            // Linked lists as the game builds them: scale list head 0x2000 / links in [0].w, translation
            // list head 0x2004 / links in [3].w. `None` = the head word.
            let (mut s_head, mut t_head) = (0u32, 0u32);
            let link = |rec: &mut [[V4; 4]; CHAIN_RECORDS], head: &mut u32, at: Option<usize>, slot: usize, v: u32| match at {
                None => *head = v,
                Some(p) => rec[p][slot][3] = v,
            };
            let mut s_tail: Option<usize> = None;
            for &r in &sa {
                let j = rec_joint(r);
                if !rec_inherited(r) || j >= CHAIN_RECORDS { continue; }
                let v = rec_scale(r);
                link(&mut rec, &mut s_head, s_tail, 0, j as u32 + 1);
                s_tail = Some(j);
                rec[j][1][3] = rec_flag_word(r);
                rec[j][0] = [v[0], v[1], v[2], 0];
            }
            let mut t_tail: Option<usize> = None;
            for &r in &ta {
                let j = rec_joint(r);
                if j >= CHAIN_RECORDS { continue; }
                let v = rec_trans(r);
                let parent = rec[j][2][3];
                link(&mut rec, &mut t_head, t_tail, 3, j as u32 + 1);
                t_tail = Some(j);
                rec[j][2] = [v[0], v[1], v[2], parent];
            }
            link(&mut rec, &mut t_head, t_tail, 3, 0);
            for &r in &sb {
                let j = rec_joint(r);
                if !rec_inherited(r) || j >= CHAIN_RECORDS { continue; }
                let listed = rec[j][0][3];
                let v = rec_scale(r);
                rec[j][1] = [v[0], v[1], v[2], rec_flag_word(r)];
                if listed != 0 { continue; }
                link(&mut rec, &mut s_head, s_tail, 0, j as u32 + 1);
                s_tail = Some(j);
                rec[j][0][3] = 0;
            }
            for &r in &tb {
                let j = rec_joint(r);
                if j >= CHAIN_RECORDS { continue; }
                let listed = rec[j][3][3];
                let v = rec_trans(r);
                rec[j][3] = [v[0], v[1], v[2], listed];
                if listed != 0 { continue; }
                link(&mut rec, &mut t_head, t_tail, 3, j as u32 + 1);
                t_tail = Some(j);
                rec[j][3][3] = 0;
            }
            // Interpolation along the lists (a repeated joint can cut a list short or be visited twice).
            let mut guard = 0;
            let mut cur = s_head;
            while cur != 0 && guard < 4 * CHAIN_RECORDS {
                let j = cur as usize - 1;
                let next = rec[j][0][3];
                rec[j][1] = lerp(rec[j][0], rec[j][1], u, t, 3, rec[j][1]);
                cur = next;
                guard += 1;
            }
            let mut cur = t_head;
            while cur != 0 && guard < 8 * CHAIN_RECORDS {
                let j = cur as usize - 1;
                let next = rec[j][3][3];
                rec[j][2] = lerp(rec[j][2], rec[j][3], u, t, 3, rec[j][2]);
                cur = next;
                guard += 1;
            }
            let plain = ((s.frame_b as i32 - s.frame_a as i32) & 0xfffe) + (s.seq_a as i32 - s.seq_b as i32) == 0;
            for (j, r) in rec.iter_mut().enumerate().take(count) {
                let (qa, qb) = (quat_bits(fa.quat_at(j)), quat_bits(fb.quat_at(j)));
                r[0] = if plain { lerp(qa, qb, u, t, 4, qa) } else { nlerp_flip(qa, qb, u, t) };
            }
        }
    }

    apply_layers(class, layers, &mut rec[..jc.min(CHAIN_RECORDS)]);
    apply_modifiers(mods, &mut rec[..jc.min(CHAIN_RECORDS)], CHAIN_RECORDS, true);

    // The chain over the marked joints below the count (0x2112d4).
    for j in (0..count).filter(|&j| mark[j]) {
        let [q, sc, tr, _] = rec[j];
        let mut rows = quat_rows(q);
        if sc[3] != 0 {
            for (i, row) in rows.iter_mut().enumerate() {
                for c in row.iter_mut().take(3) { *c = ps2::mul(*c, sc[i]); }
            }
        }
        let pp: [V4; 4] = if tr[3] == 0 { ID4 } else { match spr_record(tr[3]) { Some(p) if p < CHAIN_RECORDS => rec[p], _ => ID4 } };
        let r3 = [ps2::add(0, tr[0]), ps2::add(0, tr[1]), ps2::add(0, tr[2]), ONE];
        rec[j] = [
            mat3(pp[0], pp[1], pp[2], rows[0]),
            mat3(pp[0], pp[1], pp[2], rows[1]),
            mat3(pp[0], pp[1], pp[2], rows[2]),
            mat4_point(&pp, r3),
        ];
    }
    chains.iter().map(|c| rec[(*c.last().unwrap() as usize).min(CHAIN_RECORDS - 1)].map(|r| r.map(f32::from_bits))).collect()
}

/// [`evaluate_chains`] for one chain (`fun_0020cca8`'s `fun_00210850(moby, 1, &list, out)`).
pub fn evaluate_chain(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>, chain: &[u8]) -> Rows {
    evaluate_chains(class, s, snap, &[chain])[0]
}

/// `fun_002646d0` (L01; `FUN_0022a940` calls it every frame for Ratchet with the 9 lists
/// {0, 1, 2, 3, 4, 5, 6, 29, 30} into 0x13fe10 + 0x40·i) for one list: from the host's pose matrix `p`
/// ([`evaluate_chains`]), rotation rows `rows` (moby+0xc0, w lanes as stored), position (moby+0x10) and
/// scale (moby+0x2c):
/// 1. `W.r_k = A0·p_k.x + A1·p_k.y + A2·p_k.z + A3·p_k.w` for all four rows (`fun_001fa378`, one VU0
///    ACC chain per row), with `A = [rows; (0, 0, 0, 1)]` (`fun_001fa298`);
/// 2. `W.r3.xyz *= scale · (1/1024)` (`mul.s` then `vmulx.xyz`);
/// 3. `W.r3.xyz += position` (`vadd.xyz`).
///
/// (`fun_0020cca8`, the single-list version the other callers use, scales `p.r3` before the rotation;
/// same value, different rounding.)
pub fn attach_matrix(p: &Rows, rows: &[V4; 3], position: [f32; 3], scale: f32) -> Rows {
    use ps2::{add, mul};
    let a: [V4; 4] = [rows[0], rows[1], rows[2], [0, 0, 0, ONE]];
    let pb = p.map(bits4);
    let mut w: [V4; 4] = pb.map(|r| lanes(|l| add(add(add(mul(a[0][l], r[0]), mul(a[1][l], r[1])), mul(a[2][l], r[2])), mul(a[3][l], r[3]))));
    let s = mul(scale.to_bits(), 0x3a80_0000); // 1/1024
    for l in 0..3 {
        w[3][l] = mul(w[3][l], s);
        w[3][l] = add(w[3][l], position[l].to_bits());
    }
    w.map(|r| r.map(f32::from_bits))
}

/// `fun_002106f8(moby, n, list_ids, out)` (boot 0x2106f8, L01 0x268c28): the same marking and partial
/// evaluation as `fun_00210850` ([`evaluate_chains`]: the first byte list of each joint list, one shared
/// mark buffer, `fun_002109b8`), but it copies only **row 3** (SPR record +0x30, xyzw) of each list's last
/// joint: the joint's translation `P_j.r3` in packed model units. `MobyGetBoneMatrix` (L01 0x264630)
/// calls it; the menu-frame mobys (class 0x472) use it for their four corner joints.
pub fn joint_translations(class: &MobyAnimClass, s: &AnimState, snap: Option<&MobyFrame>, chains: &[&[u8]]) -> Vec<V4> {
    evaluate_chains(class, s, snap, chains).iter().map(|r| bits4(r[3])).collect()
}

/// `MobyGetBoneMatrix(moby, n, list_ids, out)` (L01 0x264630) after [`joint_translations`], per point `p`:
/// `k = mul.s(moby+0x2c, 1/1024)`; `p.xyz *= k` (`vmulx.xyz`, 0x221210, w kept); `p = r0·p.x + r1·p.y +
/// r2·p.z + vf0·p.w` (0x2215e0: `vmulax / vmadday / vmaddaz / vmaddw`, all four lanes, rows as stored at
/// moby+0xc0); `p.xyz += moby+0x10` (`vadd.xyz`, 0x221188). World points (game units), w as computed.
pub fn bone_points(points: &[V4], rows: &[V4; 3], position: [f32; 3], scale: f32) -> Vec<V4> {
    use ps2::{add, mul};
    let k = mul(scale.to_bits(), 0x3a80_0000);
    let vf0: V4 = [0, 0, 0, ONE];
    points
        .iter()
        .map(|p| {
            let p = [mul(p[0], k), mul(p[1], k), mul(p[2], k), p[3]];
            let r = lanes(|l| add(add(add(mul(rows[0][l], p[0]), mul(rows[1][l], p[1])), mul(rows[2][l], p[2])), mul(vf0[l], p[3])));
            [add(r[0], position[0].to_bits()), add(r[1], position[1].to_bits()), add(r[2], position[2].to_bits()), r[3]]
        })
        .collect()
}

/// `FUN_00271030` (L01): normalises the three **columns** of rotation rows in place, each with
/// `0x221410`: `n = (x² + y²) + 1·z²`, `v *= rsqrt(1, n)`, or `v = 0` when n's bits are 0.
pub fn normalise_columns(rows: &mut [V4; 3]) {
    use ps2::{add, mul};
    for c in 0..3 {
        let v = [rows[0][c], rows[1][c], rows[2][c]];
        let sq = v.map(|x| mul(x, x));
        let n = add(add(sq[0], sq[1]), mul(ONE, sq[2]));
        let out = if n == 0 { [0; 3] } else { let q = ps2::rsqrt(ONE, n); v.map(|x| mul(x, q)) };
        for (row, o) in rows.iter_mut().zip(out) { row[c] = o; }
    }
}

#[cfg(test)]
mod chain_tests {
    use super::*;

    /// A copy of `c` whose skeleton is identity, so [`evaluate`] returns `P_j` itself (`P·I` is exact on
    /// the PS2 float model: products with 1 and sums with ±0 do not round).
    fn unskinned(c: &MobyAnimClass) -> MobyAnimClass { MobyAnimClass { skeleton: vec![IDENTITY; c.joint_count], ..c.clone() } }

    fn class747() -> MobyAnimClass {
        let q_id: [i16; 4] = [0, 0, 0, 0x7fff];
        let mk = |scales: Vec<ScaleRec>, trans: Vec<TransRec>| {
            let quats = vec![q_id, [0, -6812, 0, 32051]];
            let mut payload: Vec<u8> = bytemuck::cast_slice(&quats).to_vec();
            payload.extend_from_slice(bytemuck::cast_slice(&scales));
            let to = payload.len();
            payload.extend_from_slice(bytemuck::cast_slice(&trans));
            while !payload.len().is_multiple_of(16) { payload.push(0); }
            let header = MobyFrameHeader {
                rate: 0.25, time: 0, qwc: (payload.len() / 16) as u16, quat_bytes: 16,
                scale_count: scales.len() as u16, trans_offset: to as u16, trans_count: trans.len() as u16,
            };
            MobyFrame { header, quats, scales, trans, payload }
        };
        let f0 = mk(vec![], vec![]);
        // Frame 1: a post-scale on j0, an inherited scale on j0 and a translation on j1.
        let f1 = mk(
            vec![ScaleRec { scale: [0x1000, 0x1000, 0x0cd6], joint: 0, flags: 0 }, ScaleRec { scale: [0x1200, 0x1000, 0x1000], joint: 0, flags: 0x80 }],
            vec![TransRec { trans: [0, 0, 21048], joint: 1, pad: 0 }],
        );
        let seq = MobySequence { header: MobySequenceHeader { frame_count: 2, loop_sound: 0xff, ..Default::default() }, frames: vec![f0, f1], triggers: vec![] };
        let mut s1 = IDENTITY;
        s1[3] = [0.0, 0.0, -27067.32, 1.0];
        MobyAnimClass {
            joint_count: 2,
            skeleton: vec![IDENTITY, s1],
            rest: vec![[0.0, 0.0, 831.386], [0.0, 0.0, 21119.06]],
            parent_word: vec![0, 0x7000_0000],
            sequences: vec![Some(seq)],
        }
    }

    #[test]
    fn chain_equals_full_pose_without_skeleton_and_post_scale() {
        let c = unskinned(&class747());
        for (fa, fb, t) in [(1u8, 1u8, 0.0f32), (0, 1, 0.5), (1, 0, 0.25), (1, 1, 0.75)] {
            let st = AnimState { seq_a: 0, frame_a: fa, seq_b: 0, frame_b: fb, t, ..AnimState::spawn(&c) };
            let full = evaluate(&c, &st);
            let p1 = evaluate_chain(&c, &st, None, &[0, 1]);
            if fa == fb && t != 0.0 {
                // Equal key indices: fun_002109b8 lerps plainly, fun_0020e0e0 (`consecutive`) normalises.
                assert_ne!(p1, full[1]);
                assert_eq!(p1[3], full[1][3], "same translation");
                continue;
            }
            // j1's P does not see j0's post-scale (children never do): equal bit for bit.
            assert_eq!(p1, full[1], "state {fa}->{fb} t {t}");
            // j0 itself: the full evaluator applies its post-scale (z row × 0.80225), the chain does not.
            let p0 = evaluate_chain(&c, &st, None, &[0]);
            if fa == 1 && t == 0.0 {
                assert_ne!(p0, full[0]);
                assert_eq!(p0[3], full[0][3], "translation row untouched by the post-scale");
            }
        }
        // Two chains in one call: marks of both, count = max last + 1.
        let st = AnimState { seq_a: 0, frame_a: 0, seq_b: 0, frame_b: 1, t: 0.5, ..AnimState::spawn(&c) };
        let both = evaluate_chains(&c, &st, None, &[&[0], &[0, 1]]);
        assert_eq!(both[1], evaluate_chain(&c, &st, None, &[0, 1]));
    }

    #[test]
    fn attach_matrix_and_normalise() {
        // Identity pose, rotation 90° about z, scale 0.1458333 → r3 = R·p3·s/1024 + pos.
        let mut p = IDENTITY;
        p[3] = [1024.0, 0.0, 0.0, 1.0];
        let rows: [V4; 3] = [[0, ONE, 0, 0], [NEG_ONE, 0, 0, 0], [0, 0, ONE, 0]];
        let w = attach_matrix(&p, &rows, [10.0, 20.0, 30.0], 0.5);
        assert_eq!(w[0][..3], [0.0, 1.0, 0.0]);
        assert_eq!(w[3][..3], [10.0, 20.5, 30.0]);
        let mut r: [V4; 3] = [[f(2.0), 0, 0, 0], [0, f(0.5), 0, 0], [0, 0, 0, 0]];
        normalise_columns(&mut r);
        assert_eq!(r, [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, 0, 0]], "zero column stays zero");
    }
    fn f(x: f32) -> u32 { x.to_bits() }

    /// Ratchet (class 0) on Novalis with his `ratchet_seq` sequences: the hand chain (joint list 0, ending
    /// at joint 56) evaluated by [`evaluate_chain`] equals the full evaluator's `P_56` (skeleton replaced by
    /// identity) bit for bit on every tick of the first 400 of sequence 0 (none of which has a post-scale on
    /// joint 56 or equal key indices). Needs `extracted/levels/01`; skipped without it.
    #[test]
    fn ratchet_hand_chain_matches_full_evaluator() {
        let Some(core) = crate::test_data::core(1) else { eprintln!("skipped: no extracted level 01"); return };
        let blob = core.block("moby_class/0000").unwrap().to_vec();
        let class = crate::moby::parse_moby_class(&blob).unwrap();
        let seqs: Vec<Option<MobySequence>> = (0..256)
            .map(|i| core.block(&format!("ratchet_seq/{i:03}")).and_then(|b| parse_sequence(b, 0).ok()))
            .collect();
        let ac = MobyAnimClass::new(&class, seqs);
        let (chain, _) = crate::gadget::joint_list(&blob, &class.header, crate::gadget::HAND_JOINT_LIST).unwrap();
        assert_eq!(*chain.last().unwrap(), 56);
        let bare = unskinned(&ac);
        let mut s = AnimState::spawn(&ac);
        let mut post56 = 0;
        for _ in 0..400 {
            advance(&mut s, &ac);
            let full = evaluate(&bare, &s);
            let p = evaluate_chain(&ac, &s, None, &chain);
            let post = |f: Option<&MobyFrame>| f.is_some_and(|f| f.scales.iter().any(|r| r.joint == 56 && !r.inherited()));
            if post(ac.frame(s.seq_a, s.frame_a)) || post(ac.frame(s.seq_b, s.frame_b)) { post56 += 1; continue; }
            assert_eq!(p, full[56], "state {s:?}");
        }
        assert_eq!(post56, 0, "joint 56 has no post-scale keys in sequence 0");
        // The spawn state (key 0, t = 0): P_56 (the chain) and F_56 = P_56·S_56 (the palette) share the
        // rotation rows and differ in the translation, which S moves from joint space to bind-pose space.
        let s0 = AnimState::spawn(&ac);
        let (f, p) = (evaluate(&ac, &s0), evaluate_chain(&ac, &s0, None, &chain));
        assert_eq!(p, evaluate(&bare, &s0)[56]);
        assert_ne!(p[3], f[56][3]);
    }
}

#[cfg(test)]
mod modifier_tests {
    use super::*;

    fn rec(q: [f32; 4], s: Option<[f32; 3]>, t: [f32; 3]) -> [V4; 4] {
        let sw = if s.is_some() { ONE } else { 0 };
        let s = s.unwrap_or([1.0; 3]);
        [q.map(f32::to_bits), bits3w(s, sw), bits3w(t, 0x7000_0040), [0; 4]]
    }
    fn q_of(r: &[V4; 4]) -> [f32; 4] { r[0].map(f32::from_bits) }
    fn v3(v: V4) -> [f32; 3] { [0, 1, 2].map(|k| f32::from_bits(v[k])) }
    fn close(a: &[f32], b: &[f32]) -> bool { a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6) }

    /// Mode 0 composes: `q ⊗ q_m` (the node's rotation in the joint's own frame), scale ∘ (only where the joint has
    /// an inherited scale, unless the chain evaluator marks it), translation added; the parent word is kept.
    #[test]
    fn compose_mode() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let rz = [0.0, 0.0, h, h]; // 90° about z
        let rx = [h, 0.0, 0.0, h]; // 90° about x
        let mut r = [rec(rz, None, [1.0, 2.0, 3.0])];
        let m = JointModifier { quat: rx, scale: [0.5; 3], trans: [1.0, 0.0, -1.0], ..JointModifier::compose(0) };
        apply_modifiers(&[m], &mut r, 1, false);
        assert!(close(&q_of(&r[0]), &quat_product(rz, rx)));
        assert!(close(&q_of(&r[0]), &[0.5, 0.5, 0.5, 0.5]));
        assert_eq!(r[0][1][3], 0, "no inherited scale: still absent");
        assert!(close(&v3(r[0][2]), &[2.0, 2.0, 2.0]));
        assert_eq!(r[0][2][3], 0x7000_0040, "parent word kept");
        // With an inherited scale it multiplies; the chain evaluator marks the scale present either way.
        let mut r = [rec(rz, Some([2.0, 2.0, 2.0]), [0.0; 3]), rec(rz, None, [0.0; 3])];
        let m1 = JointModifier { joint: 1, ..m };
        apply_modifiers(&[m, m1], &mut r, 2, true);
        assert!(close(&v3(r[0][1]), &[1.0; 3]) && r[0][1][3] != 0);
        assert!(close(&v3(r[1][1]), &[0.5; 3]) && r[1][1][3] != 0);
        // Identity node: nothing moves.
        let mut r = [rec(rz, None, [1.0, 2.0, 3.0])];
        let before = r;
        apply_modifiers(&[JointModifier::compose(0)], &mut r, 1, false);
        assert!(close(&q_of(&r[0]), &q_of(&before[0])) && r[0][2] == before[0][2]);
    }

    /// Mode 1 blends by the weight: 0 keeps the pose, 1 takes the node's, the hemisphere of the node is flipped to
    /// the pose's; a joint at or past the limit (MobyProc's joint count) is left alone.
    #[test]
    fn blend_mode_and_limit() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let q = [0.0, 0.0, h, h];
        let node = |w: f32, nq: [f32; 4]| JointModifier { joint: 0, mode: 1, weight: w, quat: nq, scale: [0.5, 1.0, 1.0], trans: [10.0, 0.0, 0.0] };
        let mut r = [rec(q, Some([1.0; 3]), [0.0; 3])];
        apply_modifiers(&[node(0.0, [0.0, 0.0, 0.0, 1.0])], &mut r, 1, false);
        assert!(close(&q_of(&r[0]), &q));
        let mut r = [rec(q, Some([1.0; 3]), [0.0; 3])];
        apply_modifiers(&[node(1.0, [0.0, 0.0, 0.0, -1.0])], &mut r, 1, false);
        assert!(close(&q_of(&r[0]), &[0.0, 0.0, 0.0, -1.0]));
        let mut r = [rec(q, Some([1.0; 3]), [0.0; 3])];
        apply_modifiers(&[node(0.5, [0.0, 0.0, 0.0, -1.0])], &mut r, 1, false);
        // a = q/2, b = −id/2, a·b < 0 → a − b: the short way from q to the identity.
        let e = [0.0, 0.0, h, h + 1.0];
        let n = (e[2] * e[2] + e[3] * e[3]).sqrt();
        assert!(close(&q_of(&r[0]), &e.map(|x| x / n)));
        assert!(close(&v3(r[0][1]), &[0.75, 1.0, 1.0]) && close(&v3(r[0][2]), &[5.0, 0.0, 0.0]));
        let mut r = [rec(q, None, [0.0; 3])];
        let before = r;
        apply_modifiers(&[node(1.0, [0.0, 0.0, 0.0, 1.0])], &mut r, 0, false);
        assert_eq!(r, before);
    }

    #[test]
    fn list_target_is_the_second_lists_first_joint() {
        assert_eq!(list_target(&[18, 19]), Some(18));
        assert_eq!(list_target(&[]), None);
    }
}
