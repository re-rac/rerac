# Moby animation: keyframes → joint palette

How a moby's animation sequence becomes the joint palette `F_j = P_j · S_j` (SPR `0x70000000 + 0x40·j`) that VU0 104691 skins with (docs/plan/moby_skinning_lighting.md). Addresses are boot ELF `SCUS_971.99`; "L01" = the level01.elf copy (the in-game engine). The sources are the Ghidra disassembly of the hand-written routines (their decompiler output is unusable), the Ghidra C of the plain-C helpers, and Python scans of every moby class and `ratchet_seq` blob in `extracted/levels/*/core/` (all 19 levels). No `vcallms` is involved: the animation code does all its math with inline VU0 macro ops and never calls the 28259 trig/Euler routines. The trig table that `process_moby_anim_data` copies to SPR 0x3800 is used for normals, not for animation.

## Summary

- **Q1 Disc layout.** A sequence has a 0x1c-byte header (bounding sphere, frame count, loop-sound id, trigger count, trigger list pointer, **f32 rate override**), then frame pointers and trigger words. A frame has a 16-byte header (**f32 rate, s16 time×8 ticks**, qwc, and the offsets and counts of three channels) followed by: one **quaternion per joint as 4×s16 (x,y,z,w) in 1.15 fixed point** (all four components stored, none rebuilt); sparse **scale records** (3×u16 in 4.12, joint byte, flag byte whose bit 7 means "inherited"); and sparse **translation records** (3×s16 in packed model units, joint byte). Joints without a record use scale 1 and the class `common_trans` rest translation. The "special" (Ratchet) format does not occur in RAC1: every frame on the disc is regular. Verified.
- **Q2 Evaluation.** Each tick, `MobyAnimAdvance` (`fun_0020d580`) does `t += speed·rate`. When t reaches 1 (t ≥ 0.99609 snaps to exactly 1) it steps to the next keyframe and wraps at the end of the sequence. `MobyAnimEval` (`fun_0020e0e0`) lerps the two keyframes A and B by t. Quaternions are either **plain-lerped with no normalisation** (A, B consecutive in the same sequence) or **nlerped with a hemisphere flip**. Scale and translation are lerped. The local matrix uses rows `R(q)ᵀ`, inherited scale multiplies the rows, and the translation goes in row 3. A sequence change blends over N ticks from the current keyframe or from a re-encoded snapshot of the current pose (`fun_00212f90`). Verified; the tick rate itself (60 Hz) is inferred.
- **Q3 Hierarchy.** `P_j = P_parent · L_j` (column-vector convention, VU `M·v = r0·x + r1·y + r2·z + r3`). The parent comes from the `common_trans` word at +0xc, which is the **SPR address** `0x70000000 + 0x40·parent`; 0 marks the root. Non-inherited scale is applied to `P_j` after the whole chain. Then `F_j = P_j · S_j`. The instance transform is not in the palette: it enters through M_vu1 in MobyProc. Units are packed model units (world × 1024 / scale), the same as the vertices. Verified: on 5 classes, seq 0 frame 0 gives `P_j·S_j = I` to 1e-4 with this convention, and the transposed convention fails.
- **Q4 Instance fields.** +0x50 frame A index, +0x51 frame B index, +0x52 seq A (0xff = snapshot), +0x53 seq B, **+0x54 f32 t**, +0x58 f32 speed, +0x5c f32 rate, +0x60 pose-layer list, +0x64 joint-modifier list, +0x68/+0x6c frame A/B pointers, +0x70 event flags. At spawn: seq 0, frame 0, speed 1, rate 1. With no update function a moby loops **sequence 0** forever; static classes (1 sequence, ≤ 1 frame) get speed 0 and mode 0x40. Verified.
- **Q5 Worked example.** Class 66 (Novalis instance 295, a 1-joint spinner: 15 frames, rate 0.125 = 8 ticks per key, one turn every 120 ticks) and class 747 (2 joints: scale and translation records plus the parent chain). See §7. Verified.

## 1. On-disc layout (class-relative offsets; Ratchet's `ratchet_seq/NNN` blobs are the same format, blob-relative)

**Sequence header** (class+0x48 holds `seq_count` s32 pointers; 0 = empty):

| off | type | meaning (game use) |
|---|---|---|
| 0x00 | vec4 f32 | bounding sphere (centre, radius), packed units; `fun_0020e098` lerps the A/B spheres by t for culling |
| 0x10 | u8 | frame count |
| 0x11 | u8 | looping sound id, 0xff = none (→ moby+0x7c; Wrench calls it "sound_count") |
| 0x12 | u8 | trigger count (→ moby+0x7e) |
| 0x13 | u8 | always 0 on the disc |
| 0x14 | u32 | trigger-data pointer (non-zero in 4 sequences on the whole disc); not read by the code below |
| 0x18 | **f32** | **rate override**: if ≠ 0 it replaces every frame's rate (Wrench's "animation_info") |
| 0x1c | u32[fc] | frame pointers (top nibble 0 in every sequence on the disc) |
| … | u32[trig] | triggers: lo16 = sound id, hi16 = time in the same ×16 units as below (note only) |

**Frame header** (16 bytes; payload at +0x10, DMA'd to SPR as `qwc` quadwords):

| off | type | meaning |
|---|---|---|
| 0x0 | f32 | rate: t increment per tick for the interval from this frame to the next one (used when seq+0x18 = 0). Always equals `8/Δtime` |
| 0x4 | s16 | time of this key in 1/8 ticks (= 1/16 of a 30 Hz frame); read by the frame finder L01 `FUN_00263870` and by `compute_interpolated_record_value` 0x20c9e0 (×1/16) |
| 0x6 | u16 | payload qwc = `(off_trans + 8·n_trans + 15) >> 4` |
| 0x8 | u16 | quaternion bytes = 8·joint_count (= offset of the scale records) |
| 0xa | u16 | scale record count (the game does not read it: it walks up to 0xc) |
| 0xc | u16 | offset of the translation records (= 0x8 + 8·scale_count) |
| 0xe | u16 | translation record count |

**Payload** (all little-endian, 8-byte records):
- `quat[j]`: `s16 x, y, z, w`; value = s16 / 32768 (`vitof15`). |q| lies in [0.99994, 1.0] for 3.0 M joint keys; identity is `(0,0,0,0x7fff)`.
- scale record: `u16 sx, sy, sz` (value = u16 / 4096, from `pextlh`/`psrlw 13`/`vitof15`), `u8 joint`, `u8 flags`. Flags **0x80 = inherited**: the scale goes into the local matrix and so reaches the children (117 k records). **0x00 = post-scale**: it scales only this joint's final P_j (32 k records).
- translation record: `s16 tx, ty, tz` (value = integer, `vitof0`), `u8 joint` (read with `lb`), `u8 0`. It replaces `common_trans[joint].xyz` for this key.

**`common_trans[j]`** (16 bytes): `f32 x, y, z` is the joint's **rest local translation** (relative to its parent; used whenever the key has no translation record for the joint). `u32` at +0xc is **0 for the root, otherwise `0x70000000 + 0x40·parent`**, a ready-made SPR pointer to the parent's pose matrix. The Rust loader's `parent_offset`/`seventy` split reads as `(0x40·parent, 0x7000)`. Confidence: verified (disassembly + data).

The game's own encoder confirms the layout. `fun_0020ede8` (snapshot, §4) writes this exact format: quats via `vftoi15` clamped to [−0x8000, 0x7fff]; scale records only where the scale ≠ (0x1000 ×3), with the byte 7 flag; translation records only where the value differs from the rest translation.

Disc invariants, all 19 levels: every header field is consistent with the others (109,156 frames); rate·Δtime = 8 on all 98,411 key pairs; adjacent keys of a sequence always have q_k·q_{k+1} ≥ 0 (2.94 M pairs), whereas the wrap pair (last → first) has 1,170 negative dots. Classes 0001/0002 (20 joints, no skeleton) have translation records with joint indices 52–86: they drive another skeleton (Ratchet-related, not reversed), so a port must bounds-check the joint index. Class 0 (Ratchet, 111 joints) has only empty sequence slots, and its sequences come from `core/ratchet_seq` (88 blobs on Novalis, all regular format, 111 joints). Confidence: verified.

## 2. Runtime instance fields (moby struct, 0x100 bytes)

| off | type | meaning |
|---|---|---|
| 0x50 / 0x51 | u8 | frame index of key A / key B |
| 0x52 / 0x53 | u8 | sequence of A / B; A = 0xff means "snapshot slot 0x50" (frame = `0x1aabc0 + slot·0x800`, L01 `0x18f140`) |
| 0x54 | f32 | **t**, blend between A and B (0 = A) |
| 0x58 | f32 | speed multiplier (1.0; 0 freezes; negative plays backwards) |
| 0x5c | f32 | rate = t increment per tick for the current interval |
| 0x60 | ptr | pose-layer list (§6.4) → job+0x08 |
| 0x64 | ptr | joint-modifier list (§6.4) → job+0x0c |
| 0x68 / 0x6c | ptr | frame header A / B (`update_moby_animation_state` 0x20c880 = L01 0x263718 recomputes them from 0x50–0x53) |
| 0x70 | u8 | set per tick: bit 0 = crossed a key, bit 1 = wrapped (sequence finished); gameplay polls bit 1 |
| 0x7c / 0x7d / 0x7e | u8 | loop sound id, voice slot, trigger count |
| 0xa5 | u8 | last real sequence before a snapshot |
| 0xf0 | vec4 | animated bounding sphere (`fun_0020e098`) |

**Spawn** (`init_moby_instance` 0x20c5f0): the struct is zeroed (seq 0, frames 0/0, t = 0), speed = rate = 1.0, then `update_moby_animation_state`. If the class has exactly 1 sequence and it has ≤ 1 frame: speed = 0, and when its loop-sound byte has bit 7 set (0xff = none), mode |= 0x40 (skip the advance entirely). Classes with joints but no sequence 0 (38 class blobs on the disc, 19 of them Ratchet's class 0) return early with null frame pointers. The first tick gives t = 1 → key 0 → 1 with the frame's rate. From then on the engine **loops sequence 0**. Gameplay update functions (moby+0x74, called right after the advance) switch sequences through §4. Confidence: verified; which classes have update functions that override seq 0 is unknown (the table at 0x1b3580 is filled per level).

## 3. `fun_0020d580` MobyAnimAdvance (L01 0x265260; called per moby per frame by the update loops L01 0x2792d0 / 0x2793d8 unless mode & 0x40, before the update fn and `fun_0020def8`)

With s = +0x58, r = +0x5c and t = +0x54: if r's bits are 0 or s's bits are 0, it returns and changes nothing. Otherwise `t' = t + s·r` (`adda`/`madd`) when seq A = seq B, else `t' = t + r` (a transition ignores speed). Then:
- **Snap:** if t' bits ∈ [0x3f7f0000, 0x3f808000] (t' ∈ [0.99609, 1.00391]), t' = 1.0 exactly.
- **Forward** (t' > 1 or snapped): repeat { `t' = (t' − 1)/r` (leftover ticks); A ← B (index, pointer; if seq A ≠ seq B then seq A = seq B, +0x7e = seq.trigcount and this tick's trigger check is skipped); B.index = A.index + 1, and **if B.index ≥ frame_count then B.index = 0 and flag |= 2**; r = seq+0x18, or if that is 0, `*(f32*)frameA` (the new A's rate); `t' = t'·r`; store r → +0x5c, t' → +0x54 } while t' > 1.0 (bits > 0x3f800000, no snap inside the loop). flag |= 1.
- **Backward** (t' < 0): the mirror image: `t' = t'/r`; B ← A; A.index −= 1 wrapping to frame_count − 1 (flag |= 2); r from seq+0x18 or the new A; `t' = t'·r + 1`; repeat while t' < 0.
- Else store t'.
- Triggers (note): when seq A = seq B, the first trigger whose time falls in `(old_idx·16 + ⌊16·t_old⌋, new_idx·16 + ⌊16·t_new⌋]` plays its sound (`fun_0022da68`). The loop-sound voice is refreshed through `fun_0020c940`.

Timing: one call per game frame. `rate·Δtime = 8` everywhere, and the typical rates 0.5/0.25 (2/4 ticks per key) imply 30 Hz authored keys played at a 60 Hz tick. Confidence: algorithm verified; 60 Hz inferred (the main-loop pacing was not traced).

## 4. Setting a sequence (L01 addresses in brackets)

- `fun_00212ed8(moby, seq, frame)` [0x26c5a8], **cut**: A = B = seq; A.index = min(frame, fc−1); B.index = A+1, clamped to fc−1 and wrapping to 0 if ≥ fc; rate = A's frame rate (seq+0x18 is ignored here); flag &= ~2; t is kept.
- `fun_00212f90(moby, seq, frame, blend_ticks)` [0x26c660], **blend**: if t > 0.025 or +0x60 ≠ 0 or +0x64 ≠ 0, then allocate slot k (`find_or_allocate_id_slot` 0x20cc18), `fun_0020ede8(moby, k|0x300)` re-encodes the current evaluated local pose (layers included) as a frame at `0x1aabc0 + k·0x800`, copies the sphere +0xf0 to `0x197180 + 16k`, and sets +0xa5 = seq A, seq A = 0xff, A.index = k. Then B = (seq, min(frame, fc−1)), **t = 0, speed = 1, rate = 1/blend_ticks**, +0x7c = seq.loop_sound. So the transition lerps from the current key A (or the snapshot) to the target key in `blend_ticks` ticks, then carries on from the target.
- `fun_002130d8(moby, seq, frame, blend, flags)` [0x26c7a8]: the same, with flags bit 2 forcing the snapshot and bits 0/1 → 0x100/0x200 passed to the snapshot.

Confidence: verified (Ghidra C of plain-C functions). The snapshot's quantisation is verified from its store path; its internal evaluation is inferred to match §6.

## 5. MobyProc → job record (0x211e18–0x211f44)

job+0x04 = t (+0x54), +0x08 = +0x60, +0x0c = +0x64, +0x10 = joint count (**class[8] high LOD, class[9] low LOD**; many classes have class[9] = 0, i.e. an identity palette for the low LOD). +0x12 = **0x400 if seq A = seq B and B.index = A.index + 1** (consecutive keys; this selects the plain-lerp path). The 0x400 is written only when the joint count ≠ 0. +0x18 = class `common_trans`, +0x1c = class `skeleton`. For X = A at +0x20 / B at +0x30: `+0 = frame+0x10` (payload), `+4 = u16 qwc`, `+8 = u32 frame+8`, `+0xc = u32 frame+0xc`. Confidence: verified.

## 6. `fun_0020e0e0` MobyAnimEval(job, scratch) (L01 0x265dc0)

`scratch` is the main-memory vertex-cache buffer (second argument of `moby_anim_proc`). Joint count 0 → identity at SPR 0, return.

**6.1 DMA / SPR.** A D9 (toSPR) chain built at scratch+0x1fc0: `REF common_trans, qwc = jc`; `REF A payload`; `REFE B payload` (when t's bits are 0 the A tag is turned into REFE and B is not fetched) → SPR 0x2000 onwards. Completion is polled through −1 sentinels in the last word of each region. Joint record in SPR at `0x40·j`: +0x00 quat (or inherited scale A, before the quats are written), +0x10 inherited scale (w ≠ 0 = present), +0x20 translation A (w = parent word), +0x30 translation B (w = list link). Post-scale records live at `scratch+0x10+0x20·j` (+0 = A, +0x10 = B). List heads are at scratch+0x0 (inherited scale), +0x4 (translation), +0xc (post-scale). Init: +0x00, +0x10 = (1,1,1,0), +0x20 = +0x30 = common_trans[j].

**6.2 Channels** (A, then B): each scale record → a post-scale list record or the SPR record (+0 for A, +0x10 for B), linked on first touch. Each translation record → xyz of +0x20 (A) / +0x30 (B), with the w word preserved.

**6.3 Interpolation** (u = 1 − t, computed as `vf4.x − t`; VU order: `ACC = a·u; r = ACC + b·t`):
- inherited scale (listed joints): `s = s_A·u + s_B·t`, w = 1; translation (listed): `t = t_A·u + t_B·t`.
- **quaternion, job+0x12 = 0x400:** `q = q_A·u + q_B·t`, **not normalised** (no hemisphere test; the disc guarantees dot ≥ 0 on these pairs).
- **quaternion, otherwise:** `d = ((a.x·b.x + a.y·b.y) + a.z·b.z) + a.w·b.w`; `q = q_A·u + (d ≥ 0 ? q_B : −q_B)·t`; `q *= rsqrt(((q.x² + q.y²) + q.z²) + q.w²)` (`vrsqrt Q, vf0.w, …`). The sign test is on the float's sign bit, so −0.0 counts as negative.
- t = 0 (bits): single-key path. Quats are copied raw (1.15 → float, unnormalised), inherited scale is stored directly in +0x10 (w = raw record bits ≠ 0), translations go to +0x20, and there is no lerp.
- **Quirk (post-scale list, two-key path only):** the B loop tests the record's *B-slot* w (always 0) instead of "already linked". So every B record is appended again, and an A-only joint that sat after the first re-appended joint falls off the list. That joint gets **no** post-scale for that interval. This affects 167 key pairs on the disc (e.g. L00 class 186 seq 7, class 608 seq 8). To be exact, replicate the list: final list = the A records up to the first joint that is also in B, then the B records in order. Joints not on the list keep scale 1. The inherited-scale and translation lists do this test correctly (true set union). Confidence: verified in disassembly; visual effect not checked.

**6.4 Runtime layers** (null for mobys without special update code; applied after 6.3, before the matrices):
- `+0x60` list, node: `u16 n @+2, f32 w @+8, u32 entries @+0x10, next @+0x1c`. The entries are n × 0x30 bytes (quat, scale, translation; the w word of the translation qw = target SPR joint record) and are DMA'd to SPR 0x2000. Blend: q = nlerp_flip(q, q_e, w) as above (applied to the pre-scaled vectors), s, t lerped by w. A missing scale flag is taken from the entry.
- `+0x64` list, node: `u8 mode @+3, target @+4, next @+8, f32 w @+0xc, quat @+0x10, scale @+0x20, trans @+0x30`. Mode 0: `q ← q ⊗ q_m` (vector part `a.w·b + b.w·a + a×b`, scalar `a.w·b.w − a·b`, a = current q, not normalised), `s ← s∘s_m` (w = 1, so the scale is then present), `t ← t + t_m`. Mode ≠ 0: a weighted nlerp as for layers. Confidence: verified (math). Target joint = the node's class joint list's second byte list, first entry (`AttachManipulator` 0x264370). MobyProc keeps the scale's presence word (`vmul.xyz`) and skips targets past the joint count, but its own local-matrix build (0x268690..0x26876c) scales the rows by the record's scale without testing the word, so a node's scale on a joint without a scale record still shows (records start at (1, 1, 1)); `MobyAnimEvalChain` marks the scale present. Users and the port: hero_gameplay.md §7 (`evaluate_posed`, `evaluate_chains_posed`, `Moby::joint_mods`).

**6.5 Local matrix and chain** (0x20eb2c, joints in index order, parents first). With q = (x, y, z, w) unnormalised (every product is `(q_a + q_a)·q_b`):

    r0 = (1 − 2y² − 2z²,  2xy − 2zw,  2xz + 2yw)
    r1 = (2xy + 2zw,  1 − 2x² − 2z²,  2yz − 2xw)
    r2 = (2xz − 2yw,  2yz + 2xw,  1 − 2x² − 2y²)       // w lanes 0

These rows are the images of the local axes, so as a column matrix the rotation is `R(q)ᵀ = R(q̄)`. The game's rotation for q is the standard rotation of the conjugate: q = (0, −sin θ/2, 0, cos θ/2) gives the same rows as the 28259 Euler `+θ` about y. The subtraction order is `1 − 2yy − 2zz` (`vsuby`, then `vsubz`). If +0x1c ≠ 0 (inherited scale present): `r0 *= s.x, r1 *= s.y, r2 *= s.z`. `r3 = (t.xyz, 1)`. With `p` = the +0x2c word (0 → identity):

    P_j.r_i = Pp.r0·r_i.x + Pp.r1·r_i.y + Pp.r2·r_i.z          (i = 0..2)
    P_j.r3  = Pp.r0·t.x + Pp.r1·t.y + Pp.r2·t.z + Pp.r3

i.e. `P_j = P_parent · [R(q)ᵀ·diag(s) | t]`, written over the joint record.

**6.6 Post-scale** (0x20ec38), for each post-scale list joint: `s = s_A·u + s_B·t` (or s_A when t = 0); `P_j.r0 *= s.x, r1 *= s.y, r2 *= s.z`. This runs after the whole chain, so children do not see it.

**6.7 Palette** (0x20ed18; the skeleton was DMA'd to SPR 0x1c00): `F.r_i = P.r0·S.r_i.x + P.r1·S.r_i.y + P.r2·S.r_i.z` (i < 3), `F.r3 = P.r0·S.r3.x + P.r1·S.r3.y + P.r2·S.r3.z + P.r3`. **The w column of S is never read**: the disc stores other values there (e.g. 831.39, 1.6e29). The ≤ 0x6f-joint limit keeps SPR 0..0x1bff clear of the skeleton at 0x1c00. Confidence: verified.

## 7. Verification and worked examples (Python over the level 01 core blocks `moby_class/NNNN`, then split by the retired C++ extractor)

**Convention check.** For classes 11, 608, 724, 754 and 1365, seq 0 frame 0 run through §6 gives `max |P_j·S_j − I|` ≤ 2e-4 (rotation) and < 1 unit (translation). With the rows transposed the error is 1.1–2.0 and hundreds to 14,834 units. The median joint-origin error against the bind pose (from S⁻¹) is 0.4–1.0 units vs 3,000–12,000 for classes 724/725/754. This pins down the quaternion→matrix formula, the pre-multiplied parent chain, the parent word and packed units.

**Class 66** (Novalis instance 295 at (250.4, 135.3, 61.4), instance scale 0.761; class scale 0.2496; 1 joint, 1 sequence). The sequence is at 0x50: fc = 15, loop sound 0xff, rate override 0.125. `skeleton` = 0x350 with rows (0.5, 0, −0.866), (0, 1, 0), (0.866, 0, 0.5), r3 = 0. `common_trans` = 0x390 = (0,0,0), word 0.
- frame 0 @0xb0, header `0000003e 0000 0100 0800 0000 0800 0000`: rate 0.125, time 0, qwc 1, quat bytes 8, 0 scale, trans offset 8, 0 trans. Joint 0 `00000000 0000ff7f` → q = (0, 0, 0, 0.99997) → rows = I → P = I, **F_0 = S_0**.
- frame 1 @0xd0 (time 0x40 = 64 = 8 ticks): `000064e5 0000337d` → (0, −6812, 0, 32051) → q = (0, −0.207886, 0, 0.978119), |q| = 0.999967. Rows r0 = (0.913567, 0, −0.406674), r2 = (0.406674, 0, 0.913567), i.e. +24° about y (cos 24° = 0.913545). F_0 = rows (0.104594, 0, 0.994509), (0, 1, 0), (−0.994509, 0, 0.104594).
- mid-key 0→1, t = 0.5 (0x400 path): q = (0, −0.103943, 0, 0.989044), **|q| = 0.99449, not normalised**, r0.x = 0.978392 (a unit q would give cos 12° = 0.978148).
- wrap 14→0, t = 0.5: q14 = (0, −6812, 0, −32051), dot with q0 = −0.97809 < 0 → flip → nlerp = (0, −0.104519, 0, −0.994523) = 348°. Frames step by 24°; one turn = 15 × 8 = 120 ticks.

**Class 747** (3 instances; 2 joints, seq 0: fc 61, rate 0.25, trigger count 2). `common_trans`: j0 = (0, 0, 831.386), word 0; j1 = (0, 0, 21119.06), word **0x70000000** (parent 0). Skeleton r3 of j0 = (0, 0, −831.386), of j1 = (0, 0, −27067.32). Frame 1 @0x1a0 (header `(0.25, 32, qwc 2, 16, 1, 24, 1)`): both quats identity, scale record `0010 0010 d60c 00 00` → (1, 1, 0.80225), joint 0, **post-scale**, translation record `0000 0000 3852 01 00` → (0, 0, 21048), joint 1. Result: P_0 = diag(1, 1, 0.80225) with translation (0, 0, 831.39); P_1 translation = (0, 0, 21879.39) (the child ignores the post-scale); **F_0.r3 = (0, 0, 164.41)**, F_0.r2 = (0, 0, 0.80225); **F_1.r3 = (0, 0, −5187.94)**, F_1 rotation = I.

## 8. Port plan

**Loader (`rc-formats`, `moby.rs`, new `moby_anim.rs`):**
1. `MobySequence { sphere: [f32; 4], frame_count, loop_sound: u8, triggers: Vec<(u16 sound, u16 time)>, rate_override: f32, frames: Vec<MobyFrame> }`, parsed from class+0x48 (and `ratchet_seq/NNN` for class 0, whose pointers are relative to the blob). Reject a non-zero top nibble.
2. `MobyFrame { rate: f32, time: i16, quats: Vec<[i16; 4]>, scales: Vec<{[u16; 3], joint: u8, inherit: bool}>, trans: Vec<{[i16; 3], joint: u8}> }` in disc order (the order matters for the §6.3 quirk). Validate the header invariants from §1 and keep the rest of the payload.
3. Rename `MobyTrans::vector` → rest translation; expose `parent()` from the word (0 → none, else `(w − 0x70000000)/0x40`).

**Runtime per moby per tick:** state `{seqA, idxA, seqB, idxB, t, speed, rate, flags}`, initialised as in §2; `advance()` = §3 exactly (the snap window, the leftover carry, wrap flag 2); the blend API = §4 (the snapshot can hold the evaluated f32 local pose re-quantised as the game does: `ftoi15` with clamping for quats, ×4096 for scale, `ftoi0` for translation). **Per frame:** `evaluate()` = §6.2–6.7 on the CPU (≤ 111 joints; cheap), then upload F (≤ 111 mat4) for the vertex-shader skinning from moby_skinning_lighting.md. Use the job's joint count (class[8] or class[9] per LOD). For bit-exactness run the math on the PS2 float model already used by `moby_light` (VU macro ops: separate multiply and add roundings, `ACC` chains in the listed order, `rsqrt`).

**Unit tests:**
- Header invariants on all 19 levels (0 mismatches; rate·Δtime = 8).
- Class 66 frame 1 → the rows and F_0 above; t = 0.5 plain lerp → |q| = 0.99449; the 14→0 wrap takes the flip.
- Class 747 frame 1 → F_0.r3.z = 164.41, F_1.r3.z = −5187.94.
- `P·S ≈ I` for classes 11/608/724/754/1365 at seq 0 frame 0 (tolerance 1e-3 rotation / 1 unit).
- `advance()` from spawn: tick 1 → (idxA, idxB, t) = (0, 1, 0); class 66 cycles back to key 0 after 120 ticks with flag 2 set.
- The post-scale list quirk on L00 class 608 seq 8, keys 1→2 (only joint 0 is scaled).

**Renderer default:** every instance plays seq 0 in a loop at speed 1 (or stays static where mode 0x40 would be set). Per-class update scripts, the pose layers (+0x60/+0x64), the triggers and sounds come later.

## Open

- The main-loop tick rate and frame skipping (assumed 60 Hz, fixed step).
- Which update functions override seq 0 at spawn.
- Who fills the +0x60/+0x64 lists.
- What classes 0001/0002 animate (foreign joint indices).
- `fun_0020e098`'s exact sphere rule.
- The trigger-data block at seq+0x14.

## In the port (2026-09-26)

**Formats** (`crates/rc-formats/src/moby_anim.rs`): `MobySequenceHeader` (0x1c) / `MobyFrameHeader` (16) / `ScaleRec` / `TransRec` as Pod, `parse_sequence(base, offset)` and `parse_sequences(class_blob, &MobyClass) -> Vec<Option<MobySequence>>` (None = empty slot). Frames keep their raw payload so quaternions are read by class joint index like the game. `MobyAnimClass` holds joint count (class byte 8), skeleton, rest translations and the raw `common_trans` parent words (the loader's `MobySkeleton::parent` treats word 0 and `0x70000000` alike; the evaluator uses the full word). `AnimState::spawn` = §2, `advance` = §3 (triggers and loop sound not played), `evaluate` = §6.2–6.7 without the +0x60/+0x64 layers. The evaluator emulates the SPR joint records (so a parent read sees whatever the record holds), links the post-scale list exactly as the code does (`post_scale_list`; see below), and runs every operation on the PS2 float model (`tfrag_light::ps2`) in disassembly order: `(q+q)·q` products, `1 − yy − zz` subtraction order, `ACC` chains, the nlerp dot `((y·y' + x·x') + 1·zz') + 1·ww'`, `rsqrt` = truncated sqrt then divide. Reason: the palette feeds `ftoi0` positions and 1/128-grid colour sums.

Corrections found while porting (checked in the disassembly of 0x20e0e0 and against the data):
- **Post-scale list, exact rule.** The re-append bug is not "A up to the first shared joint, then B in order". Each append writes the link into the previous tail; A nodes' own link words are overwritten with raw record bits; every B record is appended (B-slot w is 0 on first sight); the terminator zeroes the last tail. The walk is therefore: A records up to and including the first A joint that also has a B record, then the B records that come *after that joint in B order*. Example: A = [1, 0], B = [0, 1] keeps only joint 1. L00 class 608 seq 8 keys 1→2 (A = [0, 1, 4, 13], B = [0]) keeps only joint 0 (golden test).
- **§7 class 66 F_0 signs.** With the §6.7 formula (0x20ed18) and the quoted P and S rows, frame 1 gives F_0 rows (0.104594, 0, **−**0.994509), (0, 1, 0), (**+**0.994509, 0, 0.104594); §7 quotes r0.z / r2.x with the opposite signs.
- **§7 bind-pose tolerance.** Class 754 gives max |P·S − I| = 2.35e-4 (f64 on the same data: 2.349e-4), not ≤ 2e-4; the other four classes are 1.1–1.7e-4. Translations < 0.9 units.
- **Key pairs.** 98,406 of 98,411 consecutive key pairs have rate·Δtime = 8; the other 5 have Δtime = 0 (and rate = +inf bits).
- Joint bytes of translation records are read with `lb` (signed); scale joints with `lbu`. Records whose joint is ≥ the class joint count are ignored for the matrices (they only touch unused SPR records), but post-scale records still take part in the list linking.

**Engine** (`crates/rc-engine/src/moby_anim.rs`, `moby_render.rs`, `moby_light.rs`, `assets/shaders/moby.wgsl`):
- Every placed instance spawns with `AnimState::spawn` and plays sequence 0 in a loop. Ticks run in `FixedUpdate` with `Time<Fixed>` = 60 Hz (the game's rate is inferred, §3). Ratchet (class 0: 134 empty slots) gets the level's `ratchet_seq` table as his slots (slot = sequence id, inferred), so he plays `ratchet_seq/000`, an idle.
- After each tick the palette is evaluated once per distinct (class, seq/frame A, seq/frame B, t bits). Instances of a class that spawned together share the pose: 65 evaluations per tick on Novalis, 3.2 ms per tick in the dev build. The result is copied into each instance's slots of one storage buffer (8,463 `mat4x4` on Novalis, column i = F row i).
- GPU: one mesh per (class, texture) in packed game-axis units with a `Uint32x4` skin word (az | el | count, joints, 10-bit weights, multiplier RGBA). One entity per (instance, texture) with `MeshTag` = instance index (≤ 197 instanced draws for 1,712 entities), and `NoFrustumCulling` because poses leave the bind AABB. Per-instance `MobyInst` storage record (208 bytes): model = `game_to_bevy · [s/1024·R | p]`, light rows / colours / −|K| / ambient from the bit-exact `moby_lights`, palette base, colour mode. The boot-ELF normal table goes to the shader as a 256 × vec2 storage array. The shader blends the matrices (w/256), truncates and s16-wraps the position, and lights with c = A + ⌊128·S·rsqrt(|n′|²)⌋ plus the EE pack.
- Drift of the GPU formula (`RC_MOBY_LIGHT_CHECK=1`, f32 replica vs the bit-exact CPU pass, identity palette): **0 of 315,913 vertices differ** on Novalis (all 380 distinct colour sets). On screen, `RC_ANIM=0` is pixel-identical between the GPU path and `RC_MOBY_CPU_LIGHT=1` (a close view of Ratchet and the props around him), and to the pre-animation build at the `RC_CAM=152,126,66,164,140,61` view.
- Environment: `RC_ANIM=0` (identity palette), `RC_MOBY_CPU_LIGHT=1` (bit-exact CPU colours, bind pose only), `RC_MOBY_LIGHT_CHECK=1`.

**Not ported yet** (as of 2026-09-26; all of it ported since — blends `hero/anim.rs` / `moby_anim`, the class updates `rc_game::moby_update`, pose layers and joint modifiers, `anim_sound`, the sequence sphere, low LOD `moby_lod.rs`, the metal pass; leftovers: G-HERO-009, G-CLS-019, G-REN-018)**:** sequence changes and blends (`fun_00212ed8` / `fun_00212f90` / `fun_002130d8`, the snapshot re-encode `fun_0020ede8`, seq A = 0xff); the per-class update functions that override sequence 0; pose layers (+0x60 / +0x64); sound triggers and loop sounds; the animated bounding sphere (`fun_0020e098`, needed for culling); the low LOD and its joint count (class byte 9); the metal pass's skinning (entries 0x111/0x121/0x12d).

## 9. Manipulators on class mobys (W3 lane 1, 2026-09-30; G-HERO-009 class side)

**System or not: a shared system.** [H: disassembly of L01 0x264370, 0x2643e8, 0x2777d8, 0x221e38, 0x2bb128; L16
0x2e1088; L05 0x30c0a8; L04 0x2cdda0; the census `units.tsv` call lists] Evidence:
* **One list, one link pair.** `AttachManipulator` 0x264370 (Lombyte exact boot match 0x20cb10; one copy per overlay)
  and `DetachManipulator` 0x2643e8 are the only writers of moby +0x64 (§6.4, hero_gameplay.md §7). In level01 alone 20
  functions call `AttachManipulator` (Ratchet's joint records 0x227050, his blink 0x227590, Clank 0x2278c0, the
  hand / feet items, the big-head cheats 0x278720 / 0x2fac80 / 0x2fb4b8 / 0x2ff028, the vendor 0x2bb128, the vendor
  menu 0x2aee20, the NPC look-at 0x2777d8, 0x2f2280, 0x309990); the census finds it in 60 more units' class code on the
  other levels. Every pose evaluation reads the list (MobyProc 0x268b00, the chain evaluator 0x268ee8).
* **Shared record writers.** `FUN_00221e38(a, out, axis)` writes a rotation quaternion into a node's +0x10 (called
  directly by the vendor, 1143, 823, the Visibomb missile 172 and 20 more units, and by `FUN_0026ee30` = Euler x ⊗ y ⊗ z).
  `FUN_002777d8(k, d, moby, rec, list)` (f12, f13, a0, a1, a2) is **the NPC look-at record update**, one function in
  every overlay called by 40 units: a 0x80-byte record whose first 0x40 bytes are the node, +0x40 Euler angles, +0x50
  their spring velocities, +0x60 this tick's target angles, +0x70 the scale, +0x78 the moby it is linked into.
* **Per-owner parts** stay per class: where the record lives (the owner's pvars), which list, which angles.
* **The gap in the port** was data, not code: `Services::joint_lists` kept only a list's first byte list (the joint
  points of `0x2645a8`); the node's target (`AttachManipulator`: `pb[pb[0] + 4]`, the second byte list's first entry)
  was loaded for Ratchet only (`Hero::joint_targets`).

**Step 0 (reuse plan).** Built on: `rc_formats::moby_anim::{JointModifier, list_target, evaluate_posed}` (the node
and the evaluator), `Moby::joint_mods` (the list, read by the dynamic-moby palette, the shadows and the attachments),
`hero::idle::{axis_quat, euler_quat}` (0x221e38 / 0x26ee30 in native `f32`), `hero::physics::turn_spring` (0x270b58),
`class_joint_lists_where` (the loader of `Services::joint_lists`), `units::PORTS` (the unit rows). New: the list's
node keys on the moby (`Moby::joint_mod_keys`, parallel to `joint_mods`: which owner record a node is), the loaded
targets (`Services::joint_targets`), `moby_update::manip` (attach / detach / the record writers), the static-moby
palette reading `joint_mods` (it evaluated without them). Retired: `rc-engine` `vendor_render`'s own logo draw.

**In the port** (`rc_game::moby_update::manip`; coverage tables in its module doc and in each consumer's):
* `attach` / `detach` / `sync` / `set_axis` (`FUN_00221e38` into the record) / `look` (`FUN_002777d8`): the record's
  bytes at the game's pvar offsets, the node mirrored into the target's `Moby::joint_mods` under its key
  (`Moby::joint_mod_keys`). The loader fills `Services::joint_targets` beside `Services::joint_lists` for the classes
  `LevelPorts::needs_joint_lists` names (the vendor's hologram through `ClassUpdate::reads_joints_of`).
* The static-moby palette (`rc-engine` `moby_anim::MobyAnim::mods`, driven from the table) and `World::joint_point`
  (`MobyAnimEvalChain` applies +0x64) read the list; the dynamic-moby palette already did.
* Consumers: the vendor 11 → its hologram 1143 (lists 0 / 1, z spin; `rc-engine` `vendor_render` draws the beam only,
  the logo is the dynamic moby with its metal pass); the logo 1143's own update (U514, every level: its list 1 against
  its yaw); the talking NPC 774's head look-at (two `look` records, glances and Ratchet); the searchlights 823 (U180:
  list 0 tilt, the beam callback 0x30c220 through `Callback::UnitQuads`); the floats 481 (U155: three `look` records as
  spinning parts); the Visibomb missile 172's fins (lists 0 / 1, `0x221e38` axis 1).
* First-tick note [H]: a `look` record whose +0x70 is 0 in the placed data (the NPC 774's are) is active on its first
  update with scale 0 (the node shrinks the joint for that tick), as the game's code does.
* Tests: `manip::tests` (3), `vendor::tests`, `talking_npc::tests::head_looks_at_ratchet_and_glances`,
  `hologram_logo`, `sweep_light`, `spinner_float`, `visibomb::tests::fins_tilt_against_the_steering`; on the level data
  `tests/classes/manipulators.rs` (6: the loaded targets, Novalis' vendor and NPC, Kalebo's logos, Rilgar's lights,
  Eudora's floats). Frames (two runs identical): the Novalis vendor with its chrome hologram above it; the Water Pump
  Worker after the talk scene, his head toward Ratchet.
* Left (gaps G-HERO-009): U540 347 (17) and U197 447 / 920 (05, 07, 13) are cheap now (only this blocked them); the
  units that call `0x2777d8` behind other blockers (the big-head cheat, G-SAV-006: 638, 238, 623, 1382, 44, 574, 427,
  717, 631, 556; G-CLS-026: 221, 1356; the `memcard_Save` NPCs, G-SAV-002); `MobyAnimBlendEx` 0x26c7a8, `0x27b9c0`,
  `0x2b52a8` (census `anim`: 4 units / 65 created, all behind the cheat or the save).
