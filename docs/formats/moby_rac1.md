# RAC1 moby class format

*An independent format specification for the "moby" (animated character / dynamic object) class of **Ratchet & Clank** (2002, PS2; Wrench game id `rac` / RAC1), reconstructed by reading the Wrench source tree at `/Users/aslanhud/Globals/wrench` (GPL; used as reference only, no source text reproduced). Every claim is attributed to the Wrench file it was derived from. Fields whose meaning Wrench does not know are marked **unknown** and their byte layout is given anyway.*

---

## 0. Scope, conventions, and where the data lives

### 0.1 Conventions used throughout

| Convention | Value |
| --- | --- |
| Endianness | Little endian (PS2 EE). |
| Word sizes | `u8`/`s8` 1 byte, `u16`/`s16` 2, `u32`/`s32` 4, `f32` 4. |
| `Vec3f` | 3 × `f32` = 12 bytes (`basic_types.h`). |
| `Vec4f` | 4 × `f32` = 16 bytes (`basic_types.h`). |
| `Mat3` | 3 × `Vec4f` = 48 bytes — three rows of four floats (`basic_types.h`). |
| `Mat4` | 4 × `Vec4f` = 64 bytes (`basic_types.h`). |
| `MobyVec4` | 4 × `s16` = 8 bytes (`moby_packet.h`). |
| Quadword (qw) | 16 bytes. |
| Pointers | Unless stated otherwise, every "pointer" inside a moby class is a **byte offset relative to the start of the class blob** (i.e. relative to the class header at offset 0). Wrench expresses this as `absolute_offset - class_header_ofs` when writing and `src.subbuf(field)` when reading (`moby_low.cpp`). A few fields are instead **quadword** offsets (`bangles`, `corncob`) — noted individually. |
| Zero pointer | A pointer field of 0 means "absent". Wrench tests every optional pointer against 0 before following it (`moby_low.cpp::read_class`). |
| Position units | Packed vertex/collision coordinates are fixed point with **1024 units per world unit**; vertices are additionally multiplied by the class `scale` (`moby_vertex.cpp::unpack_vertices` uses `scale / 1024`, `moby_low.cpp::read_moby_collision` uses `/ 1024`). |
| Axes | Z-up, as elsewhere in the R&C engine. |
| Alignment | The class blob itself is 64-byte (0x40) aligned inside the asset WAD; Wrench asserts `class_header_ofs % 0x40 == 0` when writing (`moby_low.cpp::write_class`) and packs the asset with alignment `0x40` (`level_classes.cpp::pack_moby_classes`). |

### 0.2 Where moby class blobs live

* **Level moby classes.** Each entry of the moby class table in the level core index (`MobyClassEntry { s32 offset_in_asset_wad; s32 o_class; s32 unknown_8; s32 unknown_c; u8 textures[16] }`, `level_core.h`) points at one uncompressed class blob inside the decompressed level `core_data`. `offset_in_asset_wad == 0` means the class has no geometry blob at all, and Wrench simply skips unpacking it (`level_classes.cpp::unpack_moby_classes`). The blob's extent is derived from the surrounding block boundaries (`level_core_block_range`), i.e. **the class blob carries no size field of its own**.
* **RAC1 gadget (weapon) classes.** Listed separately by `RacGadgetHeader { s32 offset_in_asset_wad; s32 class_number; s32 compressed_size; s32 pad }` (`level_core.h`); each is an individually WAD-compressed blob in the asset WAD, and after decompression it is *the same* "phat" moby class format (Wrench unpacks it with the same `FMT_MOBY_CLASS_PHAT` hint — `level_core.cpp`). A gadget must also have a normal `MobyClassEntry` for its textures; Wrench errors out if one is missing.
* **Textures.** The 16-byte `textures[16]` array in `MobyClassEntry` indexes the moby `TextureEntry` table (`header.moby_textures`), and those indices are what the in-mesh AD-GIF blocks refer to (§2.6, §6).
* **Mesh-only variants.** Armor/wrench classes in the globals WADs use a cut-down header (`MobyArmorHeader`, §1.4) containing only the mesh section (`moby_low.h`, `armor_wad.cpp`).

### 0.3 The two header dialects

Wrench models the format with an enum `MobyFormat { RAC1, RAC2, RAC3DL }` (`moby_vertex.h`). The mapping (`moby_low.cpp::read_class`):

| Game | Format |
| --- | --- |
| `Game::RAC` (RAC1) | `RAC1` |
| `Game::GC` (R&C2) | `RAC2`, **unless** header byte 0x0b is non-zero, in which case the class is actually in the old RAC1 layout (`force_rac1_format`). Wrench notes this happens for some mobies in the R&C2 Insomniac Museum. |
| `Game::UYA`, `Game::DL` | `RAC3DL` |

So the RAC1 ("phat"/old) layout is detected by *game*, and in R&C2 by a sniff of byte 0x0b. The format flag affects **only** the packet vertex-table header (§2.7) — the class header itself is a fixed 0x48 bytes in every game (`moby_low.h`, `static_assert(sizeof(MobyClassHeader) == 0x48)`), but several header *fields* are reinterpreted per game (§1.2).

### 0.4 Gadget classes (RAC1) — verified against the game and the disc

*From the game code (level01.elf = the in-game engine, boot ELF addresses in brackets) and the retail data; not from Wrench. Loader: `crates/rc-formats/src/gadget.rs`, golden test `gadgets_for_every_level` (first against the retired C++ `rc_extract gadget` dump, now against the committed snapshot hashes).*

**Layout.** Core index header `+0x80 gadget_count`, `+0x84 gadget_offset` (index-relative) → `gadget_count` × `GadgetEntry { s32 offset_in_asset_wad; s32 class_number /* o_class */; s32 compressed_size; s32 pad /* 0 */ }`. Each entry is one WAD stream at `core_data[offset .. offset + compressed_size]` (the decompressed core data; the stream's header repeats `compressed_size`); the streams are the last blocks of the core data. Each decompresses to an ordinary moby class blob (§1–§6, same parser). Every gadget o_class also has a normal moby class table entry with `offset_in_asset_wad = 0`, and its `textures[16]` map the class's GS texture slots into the **moby** texture table exactly as for any other class (no separate gadget texture table; `LevelMobyClass::texture_table_index` applies). On the disc: 21 entries on each of the 19 levels, always the same 21 classes {71, 157, 163, 168, 175, 176, 177, 180, 185, 188, 190, 192, 208, 229, 454, 483, 562, 585, 619, 849, 1251} with byte-identical decompressed blobs across levels (21 distinct blobs); per level 358 packets, 31,478 vertices, 27,137 high-LOD triangles; each class uses exactly one texture slot (slot 0). Largest decompressed class 0x16340 bytes, under the game's 0x18000 buffer.

**Registration.** `FUN_00258128` (L01 0x258128, the level core loader) copies the table into per-gadget arrays: o_class (0x1b0040), pointer to the compressed stream (0x1b00a0), compressed size (0x1b0100), the 16 texture slots of the class's moby table entry (0x1b0160, found through the o_class → class-slot byte map 0x198040), count at 0x160008. A class without a geometry blob gets no moby class slot pointer, so its per-class sound remap (the table at core header `+0x70`) is parked per gadget at 0x1b02e0 (16 × s16). Nothing is decompressed at level load.

**On-demand decompression.** `select_world_object_resource_tables(o_class, buffer)` (L01 0x259788 [0x204a40]) looks the o_class up in 0x1b0040, WAD-decompresses the stream into one of two alternating 0x18000-byte buffers (`0x174290 + n·0x18000`, `n` toggled when `buffer = −1`), stores the buffer as the class's moby class slot (`0x197780[slot]`), relocates it with the moby texture table and the 16 slots (`fun_00203338`), and writes the parked sound remap into the class's sound defs. `LoadHandGadget` (L01 0x297d70 [0x224368]) calls it when the hand item changes, then spawns the moby (`fun_00225490` → `CreateMoby`).

**Items and attachment.** The game's gadget definition table (L01 0x179f4c [0x1863dc], 37 × 0x4c bytes, indexed by item id; these addresses are the `+0x0c` field of records that start at 0x179f40 [0x1863d0], see the correction below) holds `+0x00 s32 attach` (index of one of Ratchet's joint lists, §3.3), `+0x04 s32 o_class`, `+0x0c s32` (non-zero ⇒ normalise the attached matrix, see below). Hero struct slots: hand item (`hero+0x54`, item id in 0x1ba1a0), head (`+0x50`, 0x1ba1a8), feet (`+0x58`/`+0x5c`, 0x1ba1a4), back (`+0x4c`, 0x1ba1ac). Only **hand** items come from the gadget table; everything worn on the body is a normal moby class present on every level:

| Item | o_class | Slot | Joint list → joint (Ratchet, class 0) | Update (L01 [boot]) |
| --- | --- | --- | --- | --- |
| 8 wrench | **71** (gadget table) | hand | list 0 → **joint 56** | 0x298578 [0x224b70] |
| other hand items (9–27, 30–32, 36) | the other 20 gadget classes (and 439) | hand | def `+0x00`: 0 → 56, 1 → 55, 2 → 99, 6 → 54 | same |
| 1 (default back item, Clank — inferred) | **601** | back | list 5 → joint 5 | 0x2989c8 [0x224fc0] |
| 2, 3, 4 | 607, 608, 609 | back | list 5 → joint 5 | same |
| 5, 6, 7 | 433, 1289, 1290 | head | list 4 → joint 7 | 0x298730 [0x224d28] |
| 28, 29 | 173, 195 (one moby per foot) | feet | lists 2 / 3 → joints 99 / 93 | 0x298820 [0x224e18] |

Class 601 is taken to be Clank on Ratchet's back because it is item 1 of the back slot (Clank transforms into items 2–4), has 33 joints, 18 sequences and Ratchet's scale 0.1458. Flag-driven extras spawned by `LoadHandGadget`: 407 (flag 0x13d4e3, list 30 → joint 5) is the **Persuader** (item 35), 614 (0x13d4e1) the **Map-o-Matic** (33) and 618 (0x13d4e2) the **Bolt Grabber** (34) (list 29 → joint 3; `HeroItemsCreate` 0x22f3c0 creates each while owned, at those lists); up to 8 × 479 (count 0x141347, `fun_00225180`) are the Drone Device's drones.

**Attach rule** (`fun_0020cca8`, L01 0x264508 [0x20cca8], called every frame from the item's update function with Ratchet's moby and the list index):
1. `fun_00210850` [0x210850] marks the joints of the list's **first** byte list (a root-to-joint chain) and evaluates only them with `fun_002109b8` [0x2109b8]; the result is the pose matrix `P_j` of the chain's last joint `j` (docs/plan/moby_animation.md; the partial evaluator never loads the class skeleton `+0x14`, so it is `P_j`, not `P_j·S_j` — inferred from the disassembly).
2. `P_j.r3 *= ratchet.scale / 1024` (moby `+0x2c`, the class scale 0.1458).
3. `W.r_i = Σ_k P_j.r_i[k] · R.r_k` for all four rows, with `R` = Ratchet's rotation rows (moby `+0xc0`, three rows, `w` row 0), then `W.r3 += ratchet.position` (moby `+0x10`). In the column-vector convention of moby_animation.md: `W = [R | pos] · diag-scale-translation(P_j)`.
4. The item moby's position (`+0x10`) = `W.r3`, its rotation rows (`+0xc0`) = `W.r0..r2` (`fun_001fa2b8`). Body items and hand items with def `+0x0c ≠ 0` then normalise the three columns (`FUN_00271030`); the **wrench does not** (def `+0x0c = 0`), so joint 56's scale keys reach it. The item keeps its own class scale (`CreateMoby` copies class `+0x24` to moby `+0x2c`; wrench 71's scale applies to its own vertices).

No extra offset matrix is applied: the item's model origin sits at the joint.

**Correction: two item systems (2026-09-26, from level01.elf).** Everything above about `LoadHandGadget` is right about that function, but `LoadHandGadget` is not the gameplay path: its only reference is a function-pointer record at 0x1b3318 (`{LoadHandGadget, 0x291800, 0x297ad0, 0x297cc0, 2}`) reached from the mode tables at 0x1b2db0 / 0x1b3280 of the scene/cinematic controller `FUN_0028c990` (`0x1ba170` states), which uses the item ids 0x1ba1a0..0x1ba1ac copied from the save by `FUN_0028c128`. The playing hero's items come from the hero update `FUN_00228870` → `FUN_0022a940` (attach matrices) → `FUN_00231268` → `FUN_0022f3c0` (create) + `FUN_0022fec0` (attach, advance):
* The item definition records start at **0x179f40** (boot copy 0x1863d0, zero in the file; not 0x179f48 or 0x179f4c: nearly every reference, e.g. `GiveItem` 0x275760, loads the base 0x179f40): `+0x00` name text id (20039 "Blaster"…), `+0x04` second text id, `+0x08` slot (0 hand, 1 feet, 2 head, 3 back, 4/5 the flag-driven extras), `+0x0c` attach word, `+0x10` o_class, `+0x14` second o_class (boots: the other foot), `+0x1c` 1 for the body items, `+0x38` u16 icon id (60000 + id). The fields named above as `+0x00 / +0x04 / +0x0c` are these `+0x0c / +0x10 / +0x18`.
* Slots are 0x50-byte records at 0x1403e0 + 0x50·slot: `+0x00`/`+0x04` moby pointers, `+0x28` item id, `+0x1a` "detached" flag. The item ids come from `0x141424` (temporary item) / the save (`0x141660` hand, `0x141664` feet, `0x141668` head, `0x14166c` back) with defaults: hand **8 (wrench)** while `0x15ed90` ≠ 0 (1 in the boot ELF's data; `FUN_002307e0` sets it to 1 when the wrench is selected, 0 otherwise); back pack = `0x141430` → `0x14166c` → **2 (class 607)**, 3 (608) when `0x15ed94`; **Clank 601** as a second back moby, always (`CreateMoby(def[1].o_class)`), both hidden (mode \|= 0x41) while `0x141628` ≠ 0; head and feet only when their id is non-zero. `InitLevelRenderGlobals` → `FUN_00226b70` sets an empty saved hand weapon to 10 (bomb glove), which is what the hand shows once `0x15ed90` is cleared. A created wrench gets `fun_00212f90(moby, 1, 0, 1)`; every item copies Ratchet's moby+0x38..0x3f (light sets, cross-fade, ambient) at creation and every frame.
* Attach matrices: `FUN_0022a940` calls `fun_002646d0(ratchet, 9, {0, 1, 2, 3, 4, 5, 6, 29, 30} (0x208c70), 0x13fe10)`: one `fun_00210850` over the 9 lists, then per list `W = A·P` with `A = [rows; (0,0,0,1)]`, `W.r3.xyz *= scale/1024`, `+= position` (rotation before the scale, unlike `fun_0020cca8`). Item `i` uses `0x13fe10 + 0x40·def.attach`, so attach words 7 / 8 mean lists 29 / 30.
* `FUN_0022fec0` per item (slot order 0..6, pack before Clank): position = `W.r3`; if the item is not a glove (10, 17, 20, 25), head item (5–7) or boot (28, 29): `MobyAnimAdvance` of its own state; rows = `W.r0..r2`; then, for those same items, **normalise the columns** (`FUN_00271030`) — so in gameplay the wrench **is** normalised (record `+0x18`, read as 0x179f58 + 0x4c·id, only selects an Euler-angle route when `iGpffff8a90` ≠ 0); mode \|= 6 (off the generic update list, rows not rebuilt from angles). Gloves / head / boots instead get `FUN_0022a9c8(table, frame, class, extra, ratchet)`: a single keyframe built from Ratchet's decoded local pose (`FUN_00269938`) — item joint 0 identity, item joint k+1 = Ratchet joint `table[k]` (quaternion ×32768, inherited scale ×4096, translation when it differs from the item's rest), tables 0x17aa40 (hand: 55..70), 0x15f6a8 / 0x17aa88 (head), 0x17aad0 / 0x17aae8 (feet) read from `table + 4`. Clank's antenna glow (class 1204) hangs from Clank's list 6.
* Idle sequences (`FUN_00242930` state 0): a back moby whose sequence wrapped and is not on 1 blends to 1 over 7 ticks; on a random 110..270-tick timer `FUN_002473e0` picks a fidget from the pack table (heli 0x17c070: pack / Clank sequences (4, 11), (3, 10), (7, 14) for hero animation 0).

**`fun_002109b8` vs `fun_0020e0e0`** (verified from the disassembly, ported as `rc_formats::moby_anim::evaluate_chains`): same channel decode, list building, lerp / nlerp and quaternion rows, but non-inherited (post) scale records are skipped, the scale / translation records are read at payload byte `8 · class joint count`, the plain-lerp test is `((B.index − A.index) & 0xfffe) + (A.seq − B.seq) == 0`, quaternions are decoded only below the count, the chain runs over the marked joints only and there is no `P·S` step. So the attachment sees `P_j` — the joint's frame in model space — not the skinning matrix `F_j = P_j·S_j`, which maps the bind pose's model space to the posed one.

**In the port** (2026-09-26, `crates/rc-engine/src/moby_attach.rs`, `rc_formats::moby_anim::{evaluate_chains, attach_matrix, normalise_columns}`): on the level's class-0 instance, the wrench (71, from the gadget table, sequence 1 via a one-tick blend at creation), the back pack 607 and Clank 601 (both from sequence 0, back to 1 with a 7-tick blend when they wrap), each on its own `AnimState`, advanced once per 60 Hz tick after Ratchet's advance (`FixedPostUpdate`); per tick the 9 hero lists are evaluated from Ratchet's state, `W` built on the PS2 float model, the item's rows normalised; its palette (own state) and `MobyInst` record (own rows, class scale, Ratchet's light word and ambient) go to a record / palette buffer of its own (`moby_render::ExtraMobys`). Unit tests: `evaluate_chain` equals the full evaluator's `P_j` bit for bit (identity skeleton; synthetic class with a post-scale, and Ratchet's hand chain on 400 ticks of sequence 0). Not ported: the fidgets (RNG), gloves / head / boots (`FUN_0022a9c8`), the antenna glow, the hero state machine. `RC_ATTACH=0` disables.

---

## 1. The moby class header

### 1.1 File layout

Order in which Wrench writes a class, which matches the games' layout (`moby_low.cpp::write_class`, corroborated by `docs/moby_renderer.md` "File Layout"):

1. Class header (0x48 bytes)
2. Animation sequence pointer list (`sequence_count` × `s32`)
3. Zero padding up to `header_end_offset`
4. Bangles block (16-byte aligned)
5. Corncob block (16-byte aligned; **R&C2+ only**)
6. Animation sequences
7. Zero padding up to `packet_table_offset`
8. Packet (submesh) table: high-LOD entries, then low-LOD entries, then metal entries, then bangle entries
9. Collision data
10. Shadow data, immediately followed by the skeleton
11. Common transforms (`common_trans`)
12. Joints
13. Sound definitions
14. High-LOD packet data (VIF lists + vertex tables)
15. Low-LOD packet data
16. Metal packet data
17. Bangle packet data
18. Team palettes (**UYA/DL only**)
19. GIF usage table

### 1.2 Header fields (offset 0x00, size 0x48)

From `packed_struct(MobyClassHeader, …)` in `moby_low.h`, plus the interpretation logic in `moby_low.cpp::read_class` / `write_class`. The "RAC1 meaning" column is authoritative for this document; differences are called out.

| Off | Size | Type | Name | RAC1 meaning | Differences in GC / UYA / DL |
| --- | --- | --- | --- | --- | --- |
| 0x00 | 4 | `s32` | `packet_table_offset` | Pointer to the packet (submesh) table. **0 ⇒ the class has no mesh at all**; Wrench then sets `has_packet_table = false` and reads no packets. | same |
| 0x04 | 1 | `u8` | `high_lod_count` | Number of high-LOD packets; the table starts with these. | same |
| 0x05 | 1 | `u8` | `low_lod_count` | Number of low-LOD packets; they follow the high-LOD entries in the same table. | same |
| 0x06 | 1 | `u8` | `metal_count` | Number of metal/multipass packets (§6). | same |
| 0x07 | 1 | `u8` | `metal_begin` | Index (in packet-table entries) of the first metal packet. Wrench always writes `high_lod_count + low_lod_count`. | same |
| 0x08 | 1 | `u8` | `joint_count` | Number of skeleton joints. Drives the sizes of the `skeleton`, `common_trans` and per-frame joint arrays. Wrench enforces **max 0x6f (111)** when writing. | same |
| 0x09 | 1 | `u8` | `unknown_9` | **Low-LOD joint count** (the game's `MobyProc` uses byte 8 with the high LOD and byte 9 with the low LOD; `docs/plan/moby_skinning_lighting.md` §2). Verified: every low-LOD vertex's joints are `< max(byte 9, 1)`. Wrench preserves it verbatim without a meaning. | same |
| 0x0a | 1 | `u8` | `rac1_byte_a` | **unknown**; RAC1-only. Wrench only writes it when the format is RAC1 and otherwise leaves it 0. | Not written for RAC2/RAC3DL. |
| 0x0b | 1 | `u8` | `rac12_byte_b` / `rac3dl_team_textures` | **unknown** in RAC1 (`rac1_byte_b`), preserved verbatim. | **R&C2:** 0 ⇒ true RAC2 format, non-zero ⇒ this class uses the old RAC1 layout. **UYA/DL:** team-palette descriptor — low nibble = palettes per texture, high nibble = number of team textures (§6.3). |
| 0x0c | 1 | `u8` | `sequence_count` | Number of animation sequences; the pointer list starts at 0x48. | same |
| 0x0d | 1 | `u8` | `sound_count` | Number of `MobySoundDef` records at `sound_defs`. Wrench enforces < 256. | same |
| 0x0e | 1 | `u8` | `lod_trans` | LOD switch distance: the game draws the low LOD when the moby's depth exceeds `lod_trans * 1024` (`docs/plan/moby_skinning_lighting.md` §2; Wrench's placeholder builder uses 0x20). | same |
| 0x0f | 1 | `u8` | `shadow` | Number of 16-byte shadow entries. The shadow block sits **immediately before** the skeleton, i.e. at `skeleton - shadow*16` (`read_class`). Contents **unknown** (kept as an opaque byte blob). | same |
| 0x10 | 4 | `s32` | `collision` | Pointer to moby collision data (§5). 0 ⇒ none. | same |
| 0x14 | 4 | `s32` | `skeleton` | Pointer to `joint_count` bind/bone matrices (§3). 0 ⇒ no skeleton *and* no shadow block is read. | **DL:** matrices are `Mat3` (0x30) instead of `Mat4` (0x40). |
| 0x18 | 4 | `s32` | `common_trans` | Pointer to `joint_count` × `MobyTrans` (§3.2). 0 ⇒ absent. | same |
| 0x1c | 4 | `s32` | `joints` | Pointer to the joint-list structure (§3.3). **Wrench follows this unconditionally**, without a zero check. | same |
| 0x20 | 4 | `s32` | `gif_usage` | Pointer to the GIF usage table (§6.2). 0 ⇒ none. Also serves as the *end* marker for the team-palette region in UYA/DL. | same |
| 0x24 | 4 | `f32` | `scale` | Global scale multiplier; counteracts the 16-bit quantisation of vertex positions. World position = `packed * (scale / 1024)`. | same |
| 0x28 | 4 | `s32` | `sound_defs` | Pointer to `sound_count` × `MobySoundDef` (§1.3). | same |
| 0x2c | 1 | `u8` | `bangles` | **Quadword** offset of the bangles block (`bangles * 0x10` bytes from the class header). 0 ⇒ no bangles. | same |
| 0x2d | 1 | `u8` | `mip_dist` | Mipmap distance (`docs/moby_renderer.md`); Wrench's placeholder builder uses 8. | same |
| 0x2e | 2 | `s16` | `corncob` / `rac1_short_2e` | **RAC1: not a corncob pointer.** Wrench stores it verbatim as `rac1_short_2e` and never dereferences it — its meaning is **unknown** for RAC1. | **R&C2+:** quadword pointer to the "corn" system data (`corncob * 0x10`), §1.5. |
| 0x30 | 16 | `Vec4f` | `bounding_sphere` | Centre `(x,y,z)` and radius `w` of the sphere enclosing the moby. | same |
| 0x40 | 4 | `s32` | `glow_rgba` | Glow colour (§6.4): RGBA bytes, R low. Non-zero → `InitMobyInstance` sets moby mode 0x10 and +0x90 = this word; MobyProc then draws the glow packets (below) in +0x90's RGB with byte 3 replaced by the vertex alpha (`docs/plan/moby_skinning_lighting.md` §10). | same |
| 0x44 | 2 | `s16` | `mode_bits` | Class-level bit flags; **individual bits unknown** at class level. (Note: the *instance* `mode_bits` field is a different, 32-bit field — §7.) | same |
| 0x46 | 1 | `u8` | `type` | **unknown** class type tag. | same |
| 0x47 | 1 | `u8` | `mode_bits2` | More bit flags; **unknown**. | same |

Immediately after the header, at **0x48**, comes the sequence pointer list: `sequence_count` × `s32`, each a class-relative pointer to a sequence header, `0` meaning "this sequence slot is empty" (`moby_animation.cpp::read_moby_sequences`).

**Derived quantity — `header_end_offset`.** Wrench computes where the header region ends as the minimum of: the first non-zero sequence pointer, `bangles * 0x10`, and (non-RAC1) `corncob * 0x10`; defaulting to 0x48 (`read_class`). It writes zero padding out to that offset so that a rebuilt class is byte-identical. This is metadata about the original file, not a stored field.

**Documentation discrepancy.** `docs/moby_renderer.md`'s class-header table lists `sequence_count` at 0x8 and omits 0x9–0xc. The struct in `moby_low.h` (used by the actual reader/writer and validated by the round-trip test in `moby_class.cpp::test_moby_class_core`) puts `joint_count` at 0x8 and `sequence_count` at 0xc. Trust the struct.

### 1.3 Sound definitions (`MobySoundDef`, 0x20 bytes each)

`moby_low.h`. `sound_count` records at `sound_defs`.

| Off | Type | Name | Notes |
| --- | --- | --- | --- |
| 0x00 | `f32` | `min_range` | |
| 0x04 | `f32` | `max_range` | |
| 0x08 | `s32` | `min_volume` | |
| 0x0c | `s32` | `max_volume` | |
| 0x10 | `s32` | `min_pitch` | |
| 0x14 | `s32` | `max_pitch` | |
| 0x18 | `u8` | `loop` | |
| 0x19 | `u8` | `flags` | **bits unknown** |
| 0x1a | `s16` | `index` | sound index within the bank |
| 0x1c | `s32` | `bank_index` | |

Wrench writes this block 16-byte aligned and only if `sound_count > 0`.

### 1.4 Mesh-only ("armor") header

`MobyArmorHeader`, 0x10 bytes (`moby_low.h`), used by armor/wrench assets instead of the full 0x48 header:

| Off | Type | Name |
| --- | --- | --- |
| 0x00 | `MobyMeshInfo` (4 × `u8`) | `high_lod_count`, `low_lod_count`, `metal_count`, `metal_begin` |
| 0x04 | `s32` | `packet_table_offset` |
| 0x08 | `s32` | `gif_usage` |
| 0x0c | `s32` | `pad` |

Scale and "is animated" are **not stored** here; Wrench has to be told them via a build hint (e.g. `meshonly,0.145833328,true` for RAC1 armor — `armor_wad.cpp`). In RAC1 the wrench meshes use scale 1.

### 1.5 Corncob (R&C2+ only; listed for contrast)

`MobyCornCobHeader` = 16 × `u8`, each a quadword offset (relative to the corncob header) of a "kernel", `0xff` = slot empty. A kernel is a `Vec4f` followed by `MobyVec4` vertices; the vertex count is stashed in the `w` component of the **first vertex** (byte `kernel + 0x16`), and a kernel whose leading `Vec4f` is all zero has no vertices (`moby_low.cpp::read_moby_corncob`). **RAC1 does not use this** — field 0x2e is an opaque `s16` there.

---

## 2. Submesh ("packet") format

A moby mesh is split into *packets* small enough to fit VU1's 16 KiB of data memory. Each packet is a VIF command list (geometry topology + texture state, DMA'd to VU1) plus a vertex table (processed on the EE core and uploaded to VU1 by the EE). Wrench's types: `MobyPacket { VertexTable vertex_table; SharedVifData vif; std::vector<MobyTexCoord> sts; }` (`moby_packet.h`).

### 2.1 Packet table

`packet_table_offset` points at an array of `MobyPacketEntry`, 0x10 bytes each (`moby_packet.h`), ordered: `high_lod_count` entries, then `low_lod_count`, then (starting at index `metal_begin`) `metal_count`, then one run per bangle (`moby_low.cpp::read_moby_mesh_section`, `read_moby_bangles`).

| Off | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | `u32` | `vif_list_offset` | Class-relative pointer to the VIF command list. |
| 0x4 | `u16` | `vif_list_size` | Size of the VIF list in **16-byte units**. |
| 0x6 | `u16` | `vif_list_texture_unpack_offset` | Offset of the third UNPACK within the list (0 ⇒ no third UNPACK). Wrench computes it as `(end_of_list - ad_gif_unpack_vifcode_pos + 4) / 0x10`. |
| 0x8 | `u32` | `vertex_offset` | Class-relative pointer to the vertex table header. |
| 0xc | `u8` | `vertex_data_size` | Total size of the vertex data **including** its header, in 16-byte units. |
| 0xd | `u8` | `unknown_d` | Redundant: always `(0xf + transfer_vertex_count * 6) / 0x10`. Wrench *verifies* this on read. |
| 0xe | `u8` | `unknown_e` | Redundant: always `(3 + transfer_vertex_count) / 4`. Verified on read. |
| 0xf | `u8` | `transfer_vertex_count` | Number of vertices sent to VU1; must equal the vertex-table header's own `transfer_vertex_count`. |

### 2.2 VIF command list — command order

Read with the generic VIF decoder (`read_vif_command_list` / `filter_vif_unpacks`, `core/vif.h`; the tfrag spec covers the decoder). The list contains **two or three UNPACKs**, possibly with NOPs between them (`moby_packet.cpp`, `docs/moby_renderer.md`). Wrench indexes the *filtered* unpack list, so intervening non-UNPACK commands are irrelevant.

| # | Present in | Content | VIFcode written by Wrench |
| --- | --- | --- | --- |
| 1 | regular packets only | ST (texture coordinate) array | `UNPACK`, `vnvl = V2_16`, signed, `flg = USE_VIF1_TOPS` (double-buffered), `addr = 0xc2` quadwords, `num = sts.size()` |
| 2 | always | Index buffer, preceded by a 4-byte header | `UNPACK`, `vnvl = V4_8`, signed, `USE_VIF1_TOPS`, `addr = 0x12d` quadwords, `num = index_bytes / 4` |
| 3 | only if the packet changes texture | AD-GIF blocks + extra ("super secret") indices | `UNPACK`, `vnvl = V4_32`, signed, `USE_VIF1_TOPS`, `addr = 0x12d + num_of_unpack2`, `num = textures * 4` |

**Metal packets have no ST unpack**: for them the index buffer is unpack #1 and the AD-GIF block is unpack #2 (`moby_packet.cpp::read_metal_packets`).

The third UNPACK's VIFcode is deliberately placed at `offset % 0x10 == 0xc`, so the 16-byte-aligned quadword containing it starts 0xc bytes earlier; that earlier address is what the GIF usage table records (§6.2) (`write_shared_moby_vif_packets`).

### 2.3 ST (texture coordinate) unpack

Array of `MobyTexCoord { s16 s; s16 t; }` (4 bytes, `moby_packet.h`). Values are **VU 12-bit fixed point**: `float = s16 / 4096.0` (`vu_fixed12_to_float`, `core/vif.h`, applied in `moby_high.cpp::recover_packets`). The ST array is indexed by the same index as the VU1 vertex buffer, and it is **longer than the in-file vertex count**: the extra entries at the tail correspond to the duplicate vertices (§2.8).

### 2.4 Index buffer

Header `MobyIndexHeader` (4 bytes, `moby_packet.h`) followed immediately by signed 1-byte indices:

| Off | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | `u8` | `unknown_0` | **unknown**; Wrench round-trips it as `index_header_first_byte` (its own mesh builder writes 0). |
| 0x1 | `u8` | `texture_unpack_offset_quadwords` | Quadword offset of the third UNPACK's data relative to the decompressed index buffer in VU memory. Written as `index_bytes / 4` when textures exist, else 0. |
| 0x2 | `s8` | `secret_index` | The **first** "secret index" (see below). |
| 0x3 | `u8` | `pad` | Must be 0 (Wrench verifies). |

Index semantics (`moby_high.cpp::recover_packets`, `docs/moby_renderer.md`):

* Indices are **1-based** into the VU1 vertex buffer; subtract 1 to get a zero-based vertex.
* Only the low 7 bits are the index. **Bit 7 (0x80) suppresses the GS drawing kick** for that vertex — i.e. it is a "skip this triangle" flag, used instead of degenerate triangles to break tristrips. Wrench's reconstruction treats a byte whose value is `<= 0` (sign bit set) as "restart-ish": if the *next* byte is also `<= 0` it starts a new triangle strip, otherwise it emits a zero-area triangle (duplicating the previous index) to encode a strip restart/swap.
* **An index byte of 0 is an escape.** It means: copy 0x40 bytes (one AD-GIF block, §2.6) from the third UNPACK into the output GS packet, then take the next *secret index* as the actual vertex index. The triangle ending on that vertex is not drawn (the restart bit is effectively always set). The first secret index lives in the index header; subsequent ones are stuffed into padding inside the AD-GIF blocks.
* **Terminator.** When an index byte of 0 is reached and the corresponding secret index is also 0, the packet is finished. Because the VU1 microprogram keeps several vertices in flight, the reconstruction then drops the **last three** indices it emitted — they would never have been kicked to the GS. Wrench's own builder emits the trailer `1, 1, 1, 0` for exactly this reason (`moby_high.cpp::build_packets`). Verified: all 26,785 retail packets end with `1, 1, 1, 0` (with the secret-index stride of §2.6); not dropping them would draw a spurious triangle `(last, second-to-last, vertex 0)` per packet.
* **Winding.** Triangles come out in strip order and their facing is **not** consistent (sorting by strip parity agrees with the vertex normals only ~65% of the time), and the GS does not cull, so draw moby triangles double-sided.
* When a secret index is used as a real index, Wrench derives it as `secret_index - 0x80` (i.e. the stored byte has the no-kick bit set).

### 2.5 VU1 memory budget (useful invariants)

From the tristrip constraints Wrench uses to re-split meshes (`moby_high.cpp::setup_moby_constraints`):

* VU1 vertex/ST buffer: **0xc2 quadwords**, 2 quadwords per vertex (position + colour).
* Total unpacked data per packet: **0x164 quadwords**, with a fixed cost of `(0x12d + 1 + 1) * 16` bytes (index-buffer base address + index header + microprogram termination indices), 4 bytes per index (rounded up to a multiple of 16), and 4 quadwords per AD-GIF block.

### 2.6 AD-GIF texture blocks

The third UNPACK is an array of `MobyTexturePrimitive`, **0x40 bytes** each (`moby_packet.h`). Each is four GIF A+D quadwords (`GifAdData12 { s32 data_lo; s32 data_hi; u8 address; u8 pad_9; u16 pad_a; }`, `gif.h`) whose trailing 4 bytes are reused to smuggle indices:

| Off | Field | GS register (`address`) | Notes |
| --- | --- | --- | --- |
| 0x00 | `d1_tex1_1` | `TEX1_1` (0x14) | LOD/filter state. Wrench's builder writes `data_lo = 0xff92`, `data_hi = 4`, `pad_a = 0x41a0`. |
| 0x0c | `super_secret_index_1` | — | Extra index read by the VU1 microcode. |
| 0x10 | `d2_clamp_1` | `CLAMP_1` (0x08) | Wrap mode. |
| 0x1c | `super_secret_index_2` | — | |
| 0x20 | `d3_tex0_1` | `TEX0_1` (0x06) | **`data_lo` carries the texture index** (see below). |
| 0x2c | `super_secret_index_3` | — | |
| 0x30 | `d4_miptbp1_1` | `MIPTBP1_1` (0x34) | Mip base pointers. |
| 0x3c | `super_secret_index_4` | — | |

**Correction (verified on the retail disc, 2026-09-26).** There is one extra secret index per AD-GIF block, but they are taken from **successive quadwords** of the unpack, not from successive blocks: secret index *k* (k ≥ 1) is byte `0xc` of quadword *k−1* (`(k−1) * 0x10 + 0xc`), so the four slots of block 0 hold secret indices 1–4, block 1's slots hold 5–8, and so on; only the first *block-count* quadwords are read. This is what Wrench's `texture_data.read<s8>(i * 0x10 + 0xc)` does (the loop variable counts blocks but the stride is a quadword); an earlier reading of it as "slot 0 of each block" was wrong, and the C++ loader had that bug until 2026-09-26 (it silently ended 668 packets early). With the quadword stride, every one of the 26,785 packets on the disc (levels 0–18, including metal) uses exactly as many texture switches as it has AD-GIF blocks and ends on the trailer `1, 1, 1` followed by the terminating 0.

**Texture state carries across packets.** The first packet of each list (high LOD, low LOD, metal) always starts with a 0 escape (a texture switch); later packets often do not, and then draw with the texture left by the previous packet of the same list (TEX0 is GS state; the strip GIF tag only writes ST/RGBAQ/XYZF2). A loader must carry the current texture index from packet to packet within a list.

`d3_tex0_1.data_lo` is the **moby texture index** (an index into the class's `textures[16]` array) or one of the special values (`moby_packet.h`):

| Value | Meaning |
| --- | --- |
| ≥ 0 | index into the class texture list |
| −1 | `MOBY_TEX_NONE` — untextured |
| −2 | `MOBY_TEX_CHROME` |
| −3 | `MOBY_TEX_GLASS` |

**Validation rules Wrench enforces:** a *regular* packet's texture index must be ≥ −1 (so chrome/glass are illegal there); a *metal* packet's must be exactly −2 or −3 (`moby_packet.cpp`).

### 2.7 Vertex table header — the RAC1 difference

This is the one place where the RAC1 layout differs structurally (`moby_vertex.cpp`).

**RAC1 (`RacVertexTableHeader`, 0x20 bytes, all `u32`):**

| Off | Type | Name |
| --- | --- | --- |
| 0x00 | `u32` | `matrix_transfer_count` |
| 0x04 | `u32` | `two_way_blend_vertex_count` |
| 0x08 | `u32` | `three_way_blend_vertex_count` |
| 0x0c | `u32` | `main_vertex_count` |
| 0x10 | `u32` | `duplicate_vertex_count` |
| 0x14 | `u32` | `transfer_vertex_count` |
| 0x18 | `u32` | `vertex_table_offset` |
| 0x1c | `u32` | `unknown_e` |

**GC/UYA/DL (`GcUyaDlVertexTableHeader`, 0x10 bytes, all `u16`):** identical field order at half the width (0x0, 0x2, 0x4, 0x6, 0x8, 0xa, 0xc, 0xe).

Field meanings (`moby_vertex.cpp`, `docs/moby_renderer.md`):

* `matrix_transfer_count` — number of `MobyMatrixTransfer` records that follow the header; these are the matrices pushed into VU0 memory **before** the per-vertex loop.
* `two_way_blend_vertex_count`, `three_way_blend_vertex_count`, `main_vertex_count` — the three vertex "types", stored in exactly that order in the vertex array. Their sum is the **in-file vertex count**.
* `duplicate_vertex_count` — number of duplicate-vertex entries.
* `transfer_vertex_count` — `two_way + three_way + main + duplicate`; must match the packet-table entry's field 0xf (verified).
* `vertex_table_offset` — byte offset of the vertex array **relative to the vertex table header**. Wrench verifies `vertex_table_offset / 0x10 <= vertex_data_size`.
* `unknown_e` — **this field's type differs by game**. In GC/UYA/DL it is an opaque `u16` (`docs/moby_renderer.md` guesses "glow_rgba"). **In RAC1 it is an offset**: `header_offset + unknown_e` is the start of a trailing blob of size `vertex_data_size * 0x10 - unknown_e`, which Wrench keeps as opaque `unknown_e_data` and rewrites verbatim. It is also used to locate the end of the epilogue vertices (§2.9). **The blob is the per-vertex lighting multiplier** (`docs/plan/moby_skinning_lighting.md` §4): 4 bytes (R, G, B, A) per *transfer* vertex, 0x80 = 1.0, zero-padded to 16 bytes (size = `align16(4 * transfer_vertex_count)`, verified on every packet). The EE driver multiplies the lit vertex colour by it: `rgba = min(255, (floor(128 c) * m) >> 7)`. Every multiplier byte on the disc is **0x80** (8.8 M bytes), every padding byte 0.

Layout after the header:

1. `matrix_transfer_count` × `MobyMatrixTransfer` (2 bytes each).
2. Alignment: Wrench advances to the next multiple of 4, then to the next multiple of 8 (`if (ofs % 4) ofs += 2; if (ofs % 8) ofs += 4;`) — i.e. **the duplicate list is 8-byte aligned**. On write it simply pads to 8.
3. `duplicate_vertex_count` × `u16`.
4. Pad to 0x10, then the vertex array at `vertex_table_offset`: `two_way + three_way + main` × 0x10 bytes, plus 0–6 **epilogue** vertices (§2.9).
5. (RAC1 only) the `unknown_e_data` blob.

### 2.8 Duplicate vertices

Each duplicate entry is a `u16` whose value is `index << 7`; Wrench reads it as `dupe >> 7` (`moby_vertex.cpp`). The value indexes the VU1 "vertex cache" — a 512-entry intermediate buffer the game keeps at the end of the VU1 command buffer (`moby_high.cpp` models it as `Opt<Vertex> vertex_cache[512]`), addressed by the 9-bit vertex ID (§2.9). A duplicate vertex is appended to the packet's vertex list as a **copy of a previously seen vertex (position, normal and skin weights), but with a fresh ST pair** taken from the ST array at the position past the real vertices. This is how the format shares a position between two UV islands, and it is why the ST array is longer than the vertex array. Wrench errors ("bad duplicate vertex") if the referenced cache slot was never written — note the cache persists **across packets**.

### 2.9 Vertex record (`MobyVertex`, 0x10 bytes)

One union of three interpretations plus an "epilogue" view (`moby_vertex.h`). Bytes 0x8–0xf are common to all three types:

| Off | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x8 | `u8` | `normal_angle_azimuth` | Normal azimuth, 256 steps = 2π |
| 0x9 | `u8` | `normal_angle_elevation` | Normal elevation, 256 steps = 2π |
| 0xa | `s16` | `x` | position X, fixed point |
| 0xc | `s16` | `y` | position Y |
| 0xe | `s16` | `z` | position Z |

Positions: `world = packed * (scale / 1024)`.

Normals are **spherical coordinates, not the ISO convention**; the game copies a 256-entry `(cos, sin)` table (boot ELF 0x165500) to scratchpad 0x3800 and does the conversion on VU0. **Corrected decode** (`docs/plan/moby_skinning_lighting.md` §4–§5, from the VU0 program 104691 and its EE driver):

```
a = azimuth   * (2π/256)      // byte 0x8
e = elevation * (2π/256)      // byte 0x9
nx = cos(a) * cos(e)
ny = sin(a) * cos(e)
nz = sin(e)
```

so `a = 0, e = 0` is **+x** and `a = 64 (90°), e = 0` is **+y**. Wrench's decode (`moby_vertex.cpp::unpack_vertices`) has **x and y swapped** (`nx = sin(az)cos(el)`, `ny = cos(az)cos(el)`), and so did this section before 2026-09-26. Independent checks: the table's entry *a* equals `(cos a, sin a)` to within 3.5e-7 (so lane x is the cosine), and of all 96 axis/sign/byte-order conventions, this one correlates best with the triangle winding on the disc. The game uses the normal unnormalised after skinning (`n' = M·n`) and normalises after the light sum. Wrench's encode is `azimuth = atan2(nx, ny) * 128/π`, `elevation = asin(nz) * 128/π` (swap the `atan2` arguments to match the corrected decode); when `elevation == 0x40` (straight up, azimuth irrelevant) Insomniac's exporter adds 0x80 to the azimuth, which Wrench replicates.

Bytes 0x0–0x7 depend on the vertex type. Bytes 0x0–0x1 are always a `low_halfword` whose **bits 0–8 are a 9-bit vertex ID** (see the ID quirk below); bits 9–15 are type-dependent. The union member `register_names` records that bytes 0x0–0x3 land in VU register `vf3` (x,y,z,w) and 0x4–0x7 in `vf4` (x,y,z,w).

**Type 1 — two-way blend** (first `two_way_blend_vertex_count` vertices):

| Off | Name | Meaning |
| --- | --- | --- |
| 0x0 | `low_halfword` | bits 0–8 vertex ID; **bits 9–15 = `spr_joint_index`** (scratchpad joint to transfer) |
| 0x2 | `vu0_matrix_load_addr_1` | VU0 address of first matrix to blend |
| 0x3 | `vu0_matrix_load_addr_2` | VU0 address of second matrix |
| 0x4 | `weight_1` | first weight (0–255) |
| 0x5 | `weight_2` | second weight |
| 0x6 | `vu0_transferred_matrix_store_addr` | where to store the scratchpad-transferred matrix (`0xf4` = don't) |
| 0x7 | `vu0_blended_matrix_store_addr` | where to store the newly blended matrix (`0xf4` = don't cache it) |

**Type 2 — three-way blend** (next `three_way_blend_vertex_count`):

| Off | Name | Meaning |
| --- | --- | --- |
| 0x0 | `low_halfword` | bits 0–8 vertex ID; **bits 9–15 = third matrix load address ÷ 2** (Wrench multiplies by 2 to recover it) |
| 0x2 | `vu0_matrix_load_addr_1` | |
| 0x3 | `vu0_matrix_load_addr_2` | |
| 0x4 | `weight_1` | |
| 0x5 | `weight_2` | |
| 0x6 | `weight_3` | |
| 0x7 | `vu0_blended_matrix_store_addr` | `0xf4` = don't cache |

Note a three-way-blend vertex **cannot** carry a matrix transfer (there is no room for `spr_joint_index`).

**Type 3 — regular / no blend** (remaining `main_vertex_count`):

| Off | Name | Meaning |
| --- | --- | --- |
| 0x0 | `low_halfword` | bits 0–8 vertex ID; **bits 9–15 = `spr_joint_index`** |
| 0x2 | `vu0_matrix_load_addr` | matrix to use; the load happens *after* the store |
| 0x3 | `vu0_transferred_matrix_store_addr` | `0xf4` = don't transfer |
| 0x4–0x7 | `unused_4`..`unused_7` | unused (Wrench's packer stashes the packet index in `unused_5`) |

**The 9-bit ID quirk.** The ID stored in vertex *v[i]* is not its own — it is the ID of vertex *v[i−7]* (the VU1 program has 7 vertices in flight). Consequently:

* On read, Wrench copies `vertices[i].id → vertices[i-7].id` for all `i ≥ 7`.
* The last up-to-7 IDs live **past the end** of the real vertex array, in 0–6 *epilogue* vertices (padding vertices whose only meaningful content is the 9-bit ID). The epilogue vertex count is derived, not stored: for **RAC1** it is `(unknown_e - vertex_table_offset) / 0x10 - in_file_vertex_count`; for GC/UYA/DL it is `vertex_data_size - vertex_table_offset/0x10 - in_file_vertex_count`. Wrench verifies it is **< 7**.
* If the epilogue vertices run out, the remaining IDs are packed into the **second half of the last epilogue vertex**, viewed as `epilogue { u16 vertex_index; u8 unused_2; u8 unused_3; u16 vertex_indices[6]; }` — i.e. six more `u16` IDs at bytes 0x4–0xf. There is always **at least one** epilogue vertex.
* When writing, Wrench emits epilogue vertices until `vertices.size() % 4 == 2` and then one final vertex carrying the leftover IDs — a rule reverse-engineered from Insomniac's exporter.

The ID is what indexes the 512-entry VU1 vertex cache used by duplicate vertices (§2.8).

### 2.10 Matrix transfer / joint cache scheme

`MobyMatrixTransfer` is 2 bytes (`moby_vertex.h`):

| Off | Type | Name |
| --- | --- | --- |
| 0x0 | `u8` | `spr_joint_index` — index into the scratchpad array of world-space joint matrices |
| 0x1 | `u8` | `vu0_dest_addr` — destination address in VU0 data memory |

VU0 data memory is modelled as 64 matrix slots; an address is **always a multiple of 4** (4 quadwords per matrix) and slot = `addr / 4`. Wrench verifies the alignment and errors on an unaligned address (`moby_skinning.cpp`). The sentinel **`0xf4` means "no store"** (it is past the 0..0xf3 usable range).

The game's blending (VU0 program 104691 entry 0, `docs/plan/moby_skinning_lighting.md` §5; `docs/moby_renderer.md` and `moby_skinning.cpp::read_skin_attributes` agree except for the weight divisor). Byte numbers are offsets in the 16-byte vertex record; `SPR[j]` is joint-palette matrix *j*:

```
for each packet:
    for each pre-loop transfer:  VU0mem[t.addr/4] = SPR[t.joint]
    for each two-way vertex v:
        VU0mem[b6/4] = SPR[b1 >> 1]                              // store before load
        m = (VU0mem[b2/4]*b4 + VU0mem[b3/4]*b5) / 256
        VU0mem[b7/4] = m
    for each three-way vertex v:                                  // no transfer
        m = (VU0mem[b2/4]*b4 + VU0mem[b3/4]*b5 + VU0mem[(b1 & 0xfe)/4]*b6) / 256
        VU0mem[b7/4] = m
    for each regular vertex v:
        VU0mem[b3/4] = SPR[b1 >> 1]                              // store before load
        m = VU0mem[b2/4]
```

**Weights are /256, not /255**: the VU0 code does `itof12(w) * 16`, and every 2- and 3-way vertex on the disc has weights summing to exactly 256 (308,135 two-way and 51,665 three-way resolved vertices, levels 0–18).

Key consequences:

* **VU0 memory persists across packets.** Insomniac's exporter exploits this: a matrix blended in packet *n* can be *used* (as a "regular" vertex's single load) in packet *n+1*. So "no blend" does **not** mean "not animated".
* Transfers are **scheduled ahead of time**: a matrix needed by packet *n* is often transferred by a *vertex of packet n−1* (using the spare `vu0_transferred_matrix_store_addr` slot of a regular vertex), with the pre-loop transfer list as the overflow path (`moby_skinning.cpp::schedule_matrix_transfers`).
* Wrench validates that a vertex never loads from and stores to the same VU0 address in the same loop iteration ("Insomniac's exporter never does this"), and that inputs to a blend are themselves unblended (`count < 2`).
* For a **non-animated** mesh, a pre-loop transfer of `spr_joint_index == 0` is interpreted as loading the identity ("blend shape") matrix (`prepare_skin_matrices`).
* Wrench's own allocator reserves VU0 addresses `0 .. max_joints_per_packet*4` for transferred (unblended) matrices and `max_joints_per_packet*4 .. 0xf4` for blended ones, and fails if the first blend address would reach 0xf4.

Recovered per-vertex skin attributes are `{count, joints[3], weights[3]}` with count 1/2/3 and weights summing to 256 (a single joint has weight 256). **Verified on the whole disc** (the Rust and C++ loaders both resolve the slot machine at load time and reject any violation): all 2,968 classes resolve with every VU0 address a multiple of 4, no load from an unwritten slot or from `0xf4`+ (the write-only sink and constant area), no blend whose input is itself a blend, and all joints below the LOD's joint count (byte 8 high LOD, byte 9 low LOD). State is reset at the start of each LOD list; nothing on the disc relies on slot contents left by another list or moby.

### 2.11 Metal packets

Separate packet type (§6.1). Their vertex table header is `MobyMetalVertexTableHeader`, 0x10 bytes (`moby_vertex.cpp`; field meanings from `docs/plan/moby_skinning_lighting.md` §6):

| Off | Type | Name |
| --- | --- | --- |
| 0x0 | `s32` | `vertex_count` |
| 0x4 | `s32` | `unknown_4` — output offset of the positions in the EE driver's output buffer |
| 0x8 | `s32` | `unknown_8` — output offset of the colours |
| 0xc | `s32` | `unknown_c` — total output bytes (the sphere-map ST goes at 0) |

Vertices start at `header + 0x10`: `MetalVertex`, 0x10 bytes (**corrected layout**, from the EE driver `fun_001ee650` and VU0 entries 0x111/0x121/0x12D):

| Off | Type | Name |
| --- | --- | --- |
| 0x0 | `s16` | `x` |
| 0x2 | `s16` | `y` |
| 0x4 | `s16` | `z` |
| 0x6 | `u8` | `normal_azimuth` (decode as §2.9) |
| 0x7 | `u8` | `normal_elevation` |
| 0x8 | 3 × `u8` | `joint[3]` — joint-palette indices, read **directly** from the palette (no VU0 slot cache) |
| 0xb | `u8` | `count` — ≤ 1: a single joint (byte 0x8); 2 or 3: that many joints |
| 0xc | 3 × `u8` | `weight[3]` — /256; for count 2/3 they sum to exactly 256 (verified on every metal vertex) |
| 0xf | `u8` | pad |

Metal packets share the index-buffer and AD-GIF machinery, use no ST unpack, and their texture index must be chrome (−2) or glass (−3).

### 2.12 How Wrench reconstructs a mesh

`moby_high.cpp::recover_packets`, per packet, with state carried **across packets**:

1. `unpack_vertices` → positions (scaled), normals (spherical → Cartesian), skin attributes (by replaying the VU0 blend machine), and the 9-bit vertex ID.
2. Write every vertex into `vertex_cache[id & 0x1ff]`, and attach ST *j* from the ST array to vertex *j*.
3. Append the duplicate vertices: copy from `vertex_cache[dupe]`, then attach the ST entry at index `vertices.size()`.
4. Walk the index bytes, building triangle-strip primitives: 0 ⇒ AD-GIF/texture switch + secret index (and advance `ad_gif_index`); 0 with secret index 0 ⇒ end of packet (drop the last three indices); a byte with the sign bit set whose successor also has it ⇒ start a new strip; a lone sign-bit byte ⇒ emit a zero-area triangle (strip restart). The material of each primitive is the current texture index (which is allowed to be −1/−2/−3 and is mapped to the "none"/"chrome"/"glass" materials later).
5. `merge_packets` concatenates packets (offsetting indices) and deduplicates vertices.

---

## 3. Skeleton

### 3.1 `skeleton` table — bind matrices

`header.skeleton` points at `joint_count` matrices (`moby_low.cpp::read_class`):

* **RAC1 / R&C2 / UYA:** `Mat4`, 0x40 bytes each — four `Vec4f` rows.
* **DL only:** `Mat3`, 0x30 bytes each; Wrench synthesises the fourth row as `{m0.w, m1.w, m2.w, 0}` (i.e. DL packs the translation into the `w` components of the three rows).

Interpretation (`moby_low.cpp::recover_moby_joints`): the stored matrix is treated as the **bind matrix**; the *inverse bind matrix* is built from `inverse(mat3(stored))` for the rotation part plus `stored[3] * (scale / 1024)` for the translation. So the translation column is in the same 1024-units-per-world-unit fixed-ish space as vertices, scaled by the class `scale`. A "tip" vector (for editor bone display) is computed as `-translation * inverse_mat3`, falling back to `(0,0,0.001)` if degenerate.

The **shadow** block (`shadow` × 16 bytes) sits immediately before this table and is written immediately before it (`write_class`). Contents **unknown**.

### 3.2 `common_trans` table — hierarchy

`header.common_trans` points at `joint_count` × `MobyTrans`, 16 bytes (`moby_low.h`):

| Off | Size | Type | Name | Meaning |
| --- | --- | --- | --- | --- |
| 0x0 | 12 | `Vec3f` | `vector` | Per-joint translation / rest offset (**exact role unknown**; Wrench stores it but does not use it in joint recovery). |
| 0xc | 2 | `u16` | `parent_offset` | Encoded parent. For RAC1/2/3/DL: **parent joint index = `parent_offset / 0x40`** — i.e. it is a *byte offset into the `skeleton` table* (0x40 = one `Mat4`). Joint 0 always has parent −1 (root). |
| 0xe | 2 | `u16` | `seventy` | **unknown**; named after its usual value (0x70). |

Wrench also documents an alternative encoding for a later game ("RC4"): `parent_offset & ~0x80` is a **direct** joint index, `0x7f` means "no parent", and the high bit is set for unknown reasons; the format is detected dynamically. **RAC1 uses the `/0x40` form.**

Wrench asserts `skeleton` and `common_trans` have the same length (both `joint_count`), and that a parent index is always less than the number of joints already added (so the table is in topological order).

### 3.3 `joints` table — per-joint index lists

`header.joints` points at (`moby_low.cpp::read_moby_joints` / `write_moby_joints`):

```
s32  list_count
s32  list_pointer[list_count]      // class-relative, each 4-byte aligned
```

and each list is:

```
s16  thing_one_count
s16  thing_two_count
u8   thing_one[thing_one_count]
u8   thing_two[thing_two_count]
u8   terminator = 0xff             // Wrench verifies this
```

The **semantics of `thing_one` / `thing_two` are unknown**; they are byte lists (plausibly joint-index sets used by the animation/IK or matrix-upload code). Note this table's `list_count` is *independent* of the header's `joint_count`.

Two quirks worth recording: (a) `read_class` follows `header.joints` **without checking it for zero**, so a class with `joints == 0` would be misparsed by Wrench; (b) `write_class` always writes this table, even when empty. In practice Wrench also treats "has joints list non-empty" as its test for "this class is animated" when exporting (`moby_class.cpp::unpack_phat_class` uses `data.animation.joints.size() > 0`), whereas the renderer path uses `joint_count > 0` (`moby_low.cpp::recover_moby_class`) — an internal inconsistency in Wrench, not in the format.

---

## 4. Animation

### 4.1 Sequence table

At class offset **0x48**: `sequence_count` × `s32`, each a class-relative pointer to a sequence header; **0 = empty slot** (`moby_animation.cpp::read_moby_sequences`). Wrench enforces `sequence_count < 256` on write.

### 4.2 Sequence header (`MobySequenceHeader`, 0x1c bytes)

`moby_animation.h`:

| Off | Size | Type | Name | Meaning |
| --- | --- | --- | --- | --- |
| 0x00 | 16 | `Vec4f` | `bounding_sphere` | Per-sequence bounding sphere (centre + radius). |
| 0x10 | 1 | `u8` | `frame_count` | Number of frames. |
| 0x11 | 1 | `u8` | `sound_count` | Number of sounds for this sequence (Wrench's default/"none" value is 0xff). |
| 0x12 | 1 | `u8` | `trigger_count` | Number of entries in the trigger list. |
| 0x13 | 1 | `u8` | `unknown_13` | **unknown**. |
| 0x14 | 4 | `u32` | `triggers` | Pointer to a `MobyTriggerData` block (0 = none). **RAC1: relative to the class header.** GC/UYA: relative to the sequence header. (Wrench branches on `Game::RAC` in both the reader and the writer.) |
| 0x18 | 4 | `u32` | `animation_info` | **unknown**; stored verbatim. |

Then, contiguously:

```
0x1c                          : s32 frame_pointer[frame_count]
0x1c + frame_count*4          : u32 trigger_list[trigger_count]
(then, only if "special")      : special sequence data — §4.5
```

**Frame pointer encoding.** Low 28 bits = class-relative offset of the frame; **top nibble = flag**. If *any* frame pointer in the sequence has a non-zero top nibble, the whole sequence uses the "special" frame format; Wrench writes the flag as `0xf0000000`. Otherwise all frames use the regular format.

`MobyTriggerData` (0x20 bytes, `moby_animation.h`) is eight `u32`s at 0x00, 0x04, 0x08, 0x0c, 0x10, 0x14, 0x18, 0x1c — **all unknown**. For RAC1 Wrench pads to 0x10 before writing it.

### 4.3 Regular frame format

Header `Rac123MobyFrameHeader`, 0x10 bytes (`moby_animation.h`) — the same for RAC1, R&C2 and UYA:

| Off | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | `f32` | `unknown_0` | **unknown** (a float — plausibly a duration/speed). |
| 0x4 | `u16` | `unknown_4` | **unknown**. |
| 0x6 | `u16` | `data_size_qwords` | Size of the payload after the header, in quadwords: `((joint_count + thing_1_count + thing_2_count) * 8)` rounded up to a multiple of 0x10, divided by 0x10. |
| 0x8 | `u16` | `joint_data_size` | `joint_count * 8` — bytes of joint data. |
| 0xa | `u16` | `thing_1_count` | Count of 8-byte records in part 1. |
| 0xc | `u16` | `unknown_c` | **unknown**. |
| 0xe | `u16` | `thing_2_count` | Count of 8-byte records in part 2. |

Payload, starting at `frame + 0x10`:

| Region | Size | Content |
| --- | --- | --- |
| joint data | `joint_count × 8` | One **64-bit word per joint**. |
| thing 1 | `thing_1_count × 8` | 64-bit words, **unknown**. |
| thing 2 | `thing_2_count × 8` | 64-bit words, **unknown**. |

**Joint transform encoding: unknown.** Wrench reads the per-joint data as opaque `u64` values and never decodes them. What can be stated with confidence from the layout alone: each joint gets exactly **8 bytes per frame**, so the pose is a heavily bit-packed rotation (+ possibly scale/translation) — e.g. a packed quaternion or Euler triple — but the field boundaries, bit widths and scales are **not established by Wrench**. Likewise `thing_1` / `thing_2` are 8-byte records of unknown purpose (their counts vary per frame, so they are plausibly sparse per-joint overrides, translation channels, or IK/trigger data).

### 4.4 Special ("Ratchet") frame format

Used "for Ratchet and a handful of other mobies" (`moby_animation.cpp`). It is a **delta/variable-length** encoding: the per-frame part sizes are stored once per *sequence*, not per frame.

Sequence-level special block, written immediately after the trigger list:

| Off (from block start) | Size | Content |
| --- | --- | --- |
| 0x0 | 4 | **Packed part offsets** `u32`: bits **0–9** = `second_part_ofs`, bits **10–20** = `third_part_ofs`, bits **21–31** = `fourth_part_ofs`. Wrench adds 4 to each when reading (offsets are measured from the frame's part-1 start, i.e. `frame + 4`), and on write stores `first_part_size`, `first+second`, `first+second+third`. Limits enforced: part 2 offset ≤ 0x3ff, parts 3 and 4 offsets ≤ 0x7ff. |
| 0x4 | `joint_count × 6` | **Per-joint data: 3 × `u16` per joint** (`special.joint_data`). Contents **unknown** — plausibly a per-joint base pose or per-channel scale. (Wrench pads to 2-byte alignment before writing.) |
| 0x4 + `joint_count*6` | 1 | `u8 thing_1_count` |
| +1 | 1 | `u8 thing_2_count` |
| +2 | `thing_1_count × 8` | `u64` records, **unknown** |
| … | `thing_2_count × 8` | `u64` records, **unknown** |

Per-frame special data (at the frame pointer, masked with `0x0fffffff`):

| Off | Size | Name | Notes |
| --- | --- | --- | --- |
| 0x0 | 2 | `inverse_unknown_0` | `u16`, **unknown** (name suggests a reciprocal of the regular format's `unknown_0`). |
| 0x2 | 2 | `unknown_4` | `u16`, **unknown**. |
| 0x4 | `second_part_ofs − 4` | first part | bytes, **unknown** |
| `second_part_ofs` | `third_part_ofs − second_part_ofs` | second part | bytes, **unknown** |
| `third_part_ofs` | `fourth_part_ofs − third_part_ofs` | third part | bytes, **unknown** |
| `fourth_part_ofs` | `ceil(joint_count / 8)` | fourth part | **a bitfield with one bit per joint** — almost certainly "this joint changed this frame" flags. |
| then | variable | fifth part 1 | `thing_1_count` variable-length groups (see below) |
| then | variable | fifth part 2 | `thing_2_count` variable-length groups |

**Fifth-part group encoding** (`read_fifth_part` in `moby_animation.cpp`): each group starts with one flag byte; three 2-bit fields select how many payload bytes follow, with the value **3 meaning 0**:

```
n1 = bits 0-1 of flag;  if (n1 == 3) n1 = 0
n2 = bits 2-3 of flag;  if (n2 == 3) n2 = 0
n3 = bits 4-5 of flag;  if (n3 == 3) n3 = 0
group = flag byte + n1 bytes + n2 bytes + n3 bytes
```

Bits 6–7 of the flag byte are **unused/unknown**. The three counts strongly suggest three channels (X/Y/Z of a translation or an Euler triple) each stored with 0, 1 or 2 bytes of delta — but Wrench does not interpret them, it merely copies the bytes and re-emits them.

**Note on Wrench's writer:** it takes the part sizes from **frame 0** of the sequence and assumes all frames share them, which is consistent with the sizes being stored once per sequence.

### 4.5 Deadlocked sequences (for contrast)

DL sequences are not decoded at all: Wrench reads `s32 data_ofs` at `seq + 0x1c`, then a `DeadlockedMobySequenceDataHeader { u8 unknown_0; u8 spr_dma_qwc; u8 unknown_2; u8 unknown_3; u32 unknown_4; u32 unknown_8; u32 unknown_c }` at `seq + data_ofs`, and keeps `data_ofs + spr_dma_qwc*16` bytes as an opaque blob (`moby_animation.cpp`). RAC1 does **not** use this.

---

## 5. Collision data inside the class

`header.collision` (0x10) points at a block whose header is `MobyCollisionHeader`, 0x10 bytes (`moby_low.h`):

| Off | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | `u16` | `unknown_0` | **unknown** |
| 0x2 | `u16` | `unknown_2` | **unknown** |
| 0x4 | `s32` | `first_part_size` | size in **bytes** |
| 0x8 | `s32` | `third_part_size` | size in **bytes** |
| 0xc | `s32` | `second_part_size` | size in **bytes**; must be a multiple of 8 (Wrench verifies) |

Payload, starting at `collision + 0x10`, in this order (note the header lists part 3's size *before* part 2's, but the data order is 1, 2, 3):

| Region | Size | Content |
| --- | --- | --- |
| first part | `first_part_size` | opaque bytes — **unknown** (plausibly the mesh/face/sector structure) |
| second part | `second_part_size` | **vertex array**: 4 × `s16` per vertex (8 bytes) = `{x, y, z, pad}`; `pad` is written as 0. Coordinates are **divided by 1024** to get world units (so this array is *not* multiplied by the class `scale`). |
| third part | `third_part_size` | opaque bytes — **unknown** |

Total block size is therefore `0x10 + first + second + third`, and Wrench pads to 0x10 before writing it (`moby_low.cpp::read_moby_collision` / `write_moby_collision`).

Wrench does **not** interpret this as a hitbox/sphere set; the only decoded content is the vertex list. There is no separate hitbox structure in the class header — the `bounding_sphere` at 0x30 is the only sphere, and it is a culling sphere.

---

## 6. Multipass/metal, chrome/glass, team palettes, GIF usage

### 6.1 Metal (multipass) submeshes

`metal_count` packets starting at packet-table index `metal_begin` (= `high_lod_count + low_lod_count`). They are a **second pass** over the same silhouette that produces the shiny metallic effect (`docs/moby_renderer.md`). Structural differences from a regular packet (`moby_packet.cpp`, `moby_vertex.cpp`):

* No ST unpack (no UV data at all — the reflection coordinates are generated).
* Vertex table is `MobyMetalVertexTableHeader` + `MetalVertex[]` (§2.11), with the normal and up to three joints/weights inline and **no** matrix-transfer / duplicate / epilogue machinery — the count is an explicit `s32`. The VU0 shine entries also output a sphere-map ST, `(E·n' + 1)/2` (`docs/plan/moby_skinning_lighting.md` §5–§6).
* Their AD-GIF texture index must be **−2 (chrome)** or **−3 (glass)**; regular packets are forbidden from using those values.
* They are excluded from the GIF usage table: Wrench passes a null `gif_usage` pointer when writing metal packets.

Chrome and glass are resolved against level-global data, not the class texture list: the level core header has `chrome_map_texture` / `chrome_map_palette` fields (`level_core.cpp`). On export, texture indices −1/−2/−3 become materials named `none`, `chrome`, `glass` (`moby_class.cpp::handle_special_materials`; `moby_low.cpp::recover_moby_class` additionally generates `mat_N`, `chrome_N` and `glass_N` material sets per texture).

### 6.2 GIF usage table

`header.gif_usage` points at an array of `MobyGifUsage`, 16 bytes (`moby_vertex.h`):

| Off | Size | Field | Meaning |
| --- | --- | --- | --- |
| 0x0 | 12 | `u8 texture_indices[12]` | The texture index of each AD-GIF block in one packet's third UNPACK, in order; unused slots are `0xff`. Max 12 per entry (Wrench asserts). |
| 0xc | 4 | `u32 offset_and_terminator` | Class-relative offset of the AD-GIF unpack (specifically `vifcode_position − 0xc`, i.e. the containing quadword), **OR'd with 0x80000000 on the last entry of the table**. |

One entry per regular (non-metal) packet that has an AD-GIF unpack, generated in packet order (high LOD, low LOD, then bangles). Its purpose is to let the game find and patch every TEX0 register in the class at load time, once the textures have been assigned GS RAM addresses. Wrench refuses to write team palettes for a class with an empty GIF usage table.

### 6.3 Team palettes (UYA/DL only; documented for completeness)

Header byte **0x0b** doubles as `rac3dl_team_textures`: `palettes_per_texture = byte & 0x0f`, `texture_count = (byte & 0xf0) >> 4` (Wrench enforces both ≤ 15 and `palettes_per_texture != 0`). The palettes are `palettes_per_texture * texture_count` blocks of **1024 bytes** (256 × `u32`) located **immediately before the GIF usage table**, i.e. palette *i* counted from the end is at `gif_usage - i*1024`. Wrench's writer emits 16 zero bytes, then the palettes, then the GIF usage table.

**RAC1 has no team palettes** — byte 0x0b is the unknown `rac1_byte_b` there, and Wrench gates the whole palette path on `game == UYA || game == DL`. (The read side of the palette loop is currently commented out in `moby_low.cpp`, so Wrench does not actually round-trip palette *contents*, only the descriptor byte.)

### 6.4 Glow

`glow_rgba` at header 0x40 (`s32`) is the glow colour (RGBA, R low; byte 3 unused by the draw). RAC1 header bytes **0x0a / 0x0b** are the index of the first **glow packet** of the high / low LOD list (0xff or ≥ the count: none; MobyProc level01 0x26b8b0 `lbu v1, 0xa(class + lod)`); the packets from there to the end of the list are drawn unlit in the moby's glow colour while the moby has mode bit 0x10 (`docs/plan/moby_skinning_lighting.md` §10). `docs/moby_renderer.md` also speculates that the GC/UYA/DL vertex-table field at 0xe is a `glow_rgba`, but the code treats it as opaque, and in RAC1 that slot is an offset instead (§2.7). There is no separate "glow submesh" list in the class header.

### 6.5 Bangles

Bangles are optional detachable/attachable sub-meshes (helmets, add-ons) that reuse the main packet table. `header.bangles` is a **quadword** offset; the block is (`moby_packet.h`, `moby_low.cpp::read_moby_bangles` / `write_moby_bangles`):

| Off (from block start) | Size | Content |
| --- | --- | --- |
| 0x00 | 4 | `MobyBangleHeader { u8 packet_begin; u8 packet_count; u8 unknown_2; u8 unknown_3 }` — Wrench writes this all-zero and never reads it; fields 2 and 3 are **unknown**. |
| 0x04 | 60 | **15** × `MobyBangleIndices { u8 high_lod_packet_begin; u8 high_lod_packet_count; u8 low_lod_packet_begin; u8 low_lod_packet_count }`. A slot with `high_lod_packet_begin == 0` has no high-LOD mesh; same for low. Slots are fixed (always 15), so bangle *n* is slot *n*. |
| 0x40 | 16 per bangle | Two `MobyVec4` (2 × 8 bytes) per bangle — **meaning unknown** (`bangle.vectors[0..1]`); plausibly an attachment offset/orientation or a bounding volume. |

The packet begin/count values are **indices into the class's packet table**, so bangle packets live at `packet_table_offset + begin * 0x10` and are decoded exactly like main packets. Wrench appends bangle packet-table entries after the metal entries.

Two quirks in Wrench's implementation (potentially bugs rather than format facts): the reader indexes the vector array by *slot* (0–14, including empty slots) while the writer packs vectors consecutively per *present* bangle; and the writer allocates space for 15 index records plus `bangles.size()*16` bytes of vectors.

---

## 7. RAC1 moby instance record in the gameplay file

### 7.1 Gameplay header slots relevant to mobies

The RAC1 gameplay file begins with a table of `s32` block pointers; a pointer of 0 means the block is absent (`gameplay.cpp::read_gameplay`). Moby-related slots (`gameplay.cpp`, `RAC_GAMEPLAY_BLOCKS`; cross-checked against the table in `docs/gameplay.md`):

| Header offset | Block |
| --- | --- |
| 0x40 | moby classes |
| 0x44 | moby instances |
| 0x48 | moby groups |
| 0x4c | shared data |
| 0x50 | pvar moby-link fixup table |
| 0x54 | pvar table |
| 0x58 | pvar data |
| 0x5c | relative pvar pointer fixup table |

(For contrast: GC/UYA use 0x48/0x4c/0x50/0x54/0x58/0x5c/0x60/0x64 and DL uses 0x2c/0x30/0x34/0x38/0x3c/0x40/0x44/0x48 for the same eight blocks.)

Blocks are written 16-byte aligned (`write_gameplay`).

### 7.2 Moby instances block

```
0x00  s32 static_count            // number of instance records that follow
0x04  s32 spawnable_moby_count    // headroom for runtime-spawned mobies (Wrench's default: 400)
0x08  s32 pad[2]
0x10  RacMobyInstance record[static_count]
```

(`MobyBlockHeader` and `RacMobyBlock` in `gameplay_impl_classes.inl`.)

### 7.3 `RacMobyInstance` — 0x78 bytes

`gameplay_impl_classes.inl`, `static_assert(sizeof(RacMobyInstance) == 0x78)`. Wrench verifies `size == 0x78` on read.

| Off | Size | Type | Name | Meaning |
| --- | --- | --- | --- | --- |
| 0x00 | 4 | `s32` | `size` | Self-size; always 0x78 in RAC1. |
| 0x04 | 4 | `s32` | `unknown_4` | **unknown** (`rac1_unknown_4`) |
| 0x08 | 4 | `s32` | `unknown_8` | **unknown** |
| 0x0c | 4 | `s32` | `unknown_c` | **unknown** |
| 0x10 | 4 | `s32` | `unknown_10` | **unknown** |
| 0x14 | 4 | `s32` | `unknown_14` | **unknown** |
| 0x18 | 4 | `s32` | `o_class` | Moby class number (matches `MobyClassEntry::o_class`). |
| 0x1c | 4 | `f32` | `scale` | Uniform scale. |
| 0x20 | 4 | `f32` | `draw_distance` | **`f32` in RAC1** (it is an `s32` in GC/UYA/DL). |
| 0x24 | 4 | `s32` | `update_distance` | Distance beyond which the update function stops running. |
| 0x28 | 4 | `s32` | `unused_28` | Constant **32**; Wrench always writes 32. |
| 0x2c | 4 | `s32` | `unused_2c` | Constant **64**; Wrench always writes 64. |
| 0x30 | 12 | `Vec3f` | `position` | World position. |
| 0x3c | 12 | `Vec3f` | `rotation` | **Euler angles in radians.** Wrench composes `T · S · Rz · Ry · Rx` (`instance.cpp::set_from_pos_rot_scale`), i.e. rotate about X first, then Y, then Z; angles are wrapped to (−π, π]. |
| 0x48 | 4 | `s32` | `group` | Index into the moby groups block, or **−1** for "no group". This field is *derived* on write from the groups' member lists, and Wrench errors if one instance appears in two groups. |
| 0x4c | 4 | `s32` | `is_rooted` | Boolean. |
| 0x50 | 4 | `f32` | `rooted_distance` | Companion to `is_rooted`. |
| 0x54 | 4 | `s32` | `unknown_54` | **unknown** (`rac1_unknown_54`) |
| 0x58 | 4 | `s32` | `pvar_index` | Index into the pvar table, or −1 for none. |
| 0x5c | 4 | `s32` | `occlusion` | **0 = precompute occlusion for this instance** (comment in `gameplay_impl_classes.inl`). |
| 0x60 | 4 | `s32` | `mode_bits` | Instance mode flags. Only one bit is identified: **0x20 = `MOBY_MB1_HAS_SUB_VARS`** — the instance's pvars begin with a sub-vars header (`instance.h`, used by `pvar.cpp`). All other bits **unknown**. |
| 0x64 | 12 | `Rgb96` | `colour` | Three `s32` channels (r at 0x64, g at 0x68, b at 0x6c), each 0–255; Wrench converts to/from float via `/255` and `round(·*255)` (`gameplay_impl_common.inl`). Semantically a tint/light colour (the GC/UYA field is literally named `light_colour`). |
| 0x70 | 4 | `s32` | `light` | Light index. |
| 0x74 | 4 | `s32` | `unknown_74` | **unknown** (`rac1_unknown_74`) |

**Fields RAC1 does *not* have** (present from R&C2 on): `mission`, `uid`, `bolts`. In GC/UYA the record is **0x88** bytes with `mission` at 0x04, `uid` at 0x10, `bolts` at 0x14; in DL it is **0x70** bytes with `mission` 0x04, `uid` 0x08, `bolts` 0x0c. The RAC1 record has *five* leading unknown `s32`s (0x04–0x14) where the later games put mission/uid/bolts and four unknowns. A plausible reading is that some of RAC1's unknown_4..14 are the precursors of those fields, but Wrench does not claim so.

**Instance IDs.** There is no UID in the RAC1 record; Wrench synthesises instance IDs from the array index (`instance.set_id_value(index++)`), and `mobylink` references elsewhere (groups, pvars) are resolved against that index (`gameplay_convert.cpp`).

`Rgb96` is `{ s32 r; s32 g; s32 b; }` — note that `gameplay_impl_common.inl` declares its `b` member with the comment offset `0xc`, which is a typo for 0x8; the struct is 12 bytes and the surrounding `static_assert`s on the instance sizes confirm it.

### 7.4 Moby classes list

A flat table of class numbers actually used by the level (`ClassBlock` in `gameplay_impl_classes.inl`):

```
0x00  s32 count
0x04  s32 o_class[count]
```

Wrench reads it verbatim for mobies (unlike the tie/shrub class lists, which it regenerates from the instance arrays on write).

### 7.5 Moby groups

`GroupBlock` (`gameplay_impl_classes.inl`):

```
0x00  s32 group_count
0x04  s32 data_size          // size in bytes of the member data region
0x08  s32 pad[2]
0x10  s32 pointer[group_count]   // byte offset into the member region, or -1 for an empty group
      ... padded to 0x10 ...
      u16 members[data_size / 2]
```

A group's member list starts at `member_region + pointer` and is a run of `u16`s: **bits 0–14 are the moby instance index, bit 15 (0x8000) marks the last member**. A pointer of −1 (or any negative value) means the group has no members. Wrench pads the member region to 0x10 and computes `data_size` from the padded end.

### 7.6 Pvars

Four cooperating blocks (`gameplay_impl_classes.inl`, `gameplay.h`, `pvar.cpp`).

**Pvar table** (header 0x54): an array of `PvarTableEntry { s32 offset; s32 size }`. The array length is **not stored** — Wrench derives it as `1 + max(pvar_index)` over all moby, camera and sound instances. Each entry describes a slice of the pvar data block.

**Pvar data** (header 0x58): one opaque byte blob; its length is likewise derived as `max(entry.offset + entry.size)`.

**Moby-link fixup table** (header 0x50) and **relative-pointer fixup table** (header 0x5c): both arrays of `PvarFixupEntry { s32 pvar_index; u32 offset }`, terminated by an entry whose `pvar_index < 0`. Wrench writes the terminator as two `s32`s of −1.

* The **moby-link** table lists every 4-byte field inside a pvar block that holds a moby instance index, so the game can turn it into a pointer at load time. Wrench only emits an entry when the stored link value is > −1.
* The **relative-pointer** table lists every 4-byte field holding an offset that must be relocated into an absolute pointer at load time — this is how the "sub vars" pointers work.

**Sub vars.** If an instance's `mode_bits & 0x20` is set, its pvar block starts with a fixed header of pointers to sub-structures. For **RAC1** the header is **0x20 bytes**, with eight `s32` pointer slots at 0x00–0x1c; Wrench's reconstructed names and sizes (`RAC_PVAR_SUB_VARS`, `pvar.cpp`) are:

| Slot | Struct name (Wrench) | Size |
| --- | --- | --- |
| 0x00 | `RacVars00` | 0x40 |
| 0x04 | `RacVars04` | unknown |
| 0x08 | `RacVars08` | 0x40 |
| 0x0c | `RacVars0c` | 0x10 |
| 0x10 | `RacVars10` | 0x60 |
| 0x14 | `RacVars14` | 0xb0 |
| 0x18 | `RacVars18` | 0x50 |
| 0x1c | (unused) | — |

These names are placeholders; only the *sizes and offsets* are established. (For contrast R&C2's 0x20-byte header names slot 0x00 `TargetVars` (0x30), 0x10 `ReactVars`, 0x1c `MoveVars_V2`; UYA's header is 0x30 and DL's 0x50.) Wrench also enforces that all instances of the same moby class have pvars of the **same size**, and that their sub-vars headers are byte-identical.

Type recovery for the remainder of a pvar block is heuristic: at each 4-byte offset Wrench checks, in order, for a sub-vars struct start, a moby link, a relative/shared pointer, and otherwise emits a placeholder `int unknown_XX` (`pvar.cpp::generate_moby_pvar_types`). Pvar structs are named `update<o_class>` for mobies (`camera<n>` / `sound<n>` for the other two pvar-bearing instance types).

### 7.7 Shared data

Header 0x4c (`SharedDataBlock`, `gameplay_impl_classes.inl`):

```
0x00  s32 data_size        // bytes of shared data (16-byte aligned on write)
0x04  s32 pointer_count
0x08  s32 unused_8[2]
0x10  u8  shared_data[data_size]
      SharedDataEntry table[pointer_count]     // immediately after the data (16-byte aligned on write)
```

`SharedDataEntry` is 8 bytes (`gameplay.h`):

| Off | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | `u16` | `pvar_index` | Which pvar block contains the pointer. |
| 0x2 | `u16` | `pointer_offset` | Byte offset of the pointer field inside that pvar block. |
| 0x4 | `s32` | `shared_data_offset` | Byte offset into the shared-data region that the pointer should be made to point at. |

This is a third fixup mechanism: several instances' pvars can point into one shared region (e.g. shared state for a group of mobies). Wrench slices the region into "shared data instances" at the distinct `shared_data_offset` values found in the table.

---

## 8. Wrench's export algorithm (for cross-checking an implementation)

**Class → glTF** (`moby_class.cpp::unpack_phat_class`):

1. Dump the raw blob as a binary asset (so nothing is lost).
2. `MOBY::read_class(buffer, game)` — parse the whole class per §1–§6.
3. `animated = !animation.joints.empty()`.
4. `MOBY::recover_packets(high_lod, …)` → glTF meshes, then `merge_packets` → one node named `moby`; likewise `low_lod` → `moby_low_lod`, and each bangle → `bangle_N` / `bangle_N_low_lod`.
5. Emit up to 16 materials from the class texture list, plus the special `none` / `chrome` / `glass` materials for texture indices −1/−2/−3, plus an `error` material for out-of-range indices.
6. Write `mesh.glb`.

Note: metal packets, collision, animation, sequences, sound defs, corncob, shadow and the joints list are **not** exported to glTF — they survive only via the binary core asset. `recover_moby_class` (the editor-facing COLLADA path) additionally recovers the joint hierarchy, but its mesh recovery and `recover_moby_joints` body are currently commented out / early-returning in this checkout, and `build_moby_class` / `pack_vertices` / `write_packets`'s scheduling half are likewise stubbed. **Only the binary reader/writer and the mesh-only packer are fully live.**

**Round-trip self-test** (`moby_class.cpp::test_moby_class_core`) — a good acceptance test for any reimplementation:

1. `read_class(src)` then `write_class(dest)`.
2. Strip trailing padding from the source to a 0x40 boundary.
3. Diff bytes 0x00–0x50 (the header + sequence pointers) and 0x50–end separately.
4. Independently: `recover_packets → merge_packets → split_packets` for both LOD levels and assert the per-packet meshes are equal and the packet counts match.

Because this test demands **byte-exact** reproduction, the odd rules in this spec (the `% 4 == 2` epilogue-vertex rule, the 8-byte duplicate-list alignment, the `0x10`-aligned AD-GIF placement at `% 0x10 == 0xc`, the `header_end_offset` zero padding, the `unused_28 = 32` / `unused_2c = 64` constants) are load-bearing, not cosmetic.

**Mesh-only packing** (`pack_mesh_only_class`) shows the intended build pipeline: read glTF → map materials → `effective_materials(SURFACE | WRAP_MODE)` → `split_packets` (honouring the VU1 constraints of §2.5) → `build_packets` → `write_mesh_only_class`.

---

## 8b. Verified on the retail NTSC-U disc (2026-09-26)

The retired C++ reference (`src/core/moby.cpp`) implemented §1.2, §2.1–§2.11 (positions, normals, ids, duplicates, index walk, ad-gif texture indices, VU0 slot machine, metal vertices, multiplier blob), §3.1–§3.2 and §7.3. `rc_extract moby --level N` parses **every moby class of all 19 levels** (2,968 class blobs; 96–183 per level) with zero failures: the redundant packet-entry fields always match, every epilogue count is 1–6, every index stream terminates on a zero secret index after the `1, 1, 1` trailer, the 9-bit vertex cache carried across packets resolves every duplicate vertex, and the skinning invariants of §2.10 hold. Instances whose class has no geometry blob (offset 0) are 10–120 per level. Bind-pose vertices lie within ~1.25× the class bounding sphere for ~97% of high-LOD vertices; the outliers need checking against the game's culling code (the sphere may be for a different rest pose). Animation decoding (§4) is not implemented yet.

Totals, levels 0–18: 2,968 classes; 22,227 high-LOD, 3,435 low-LOD and 1,123 metal packets; 2,300,789 vertices (300,722 of them duplicates); 1,933,983 high-LOD, 261,607 low-LOD and 94,406 metal triangles; resolved skins with 1/2/3 joints: 1,940,989 / 308,135 / 51,665.

Corrections found while porting (all now in both loaders): the secret-index stride (§2.6; the C++ loader silently truncated 668 packets), the flush trailer being drawn (§2.4), the texture carried across packets (§2.6), the normal axis order (§2.9), the weight divisor (§2.10), the metal vertex layout (§2.11) and the `unknown_e` multiplier blob (§2.7). Sources: `docs/plan/moby_skinning_lighting.md` (decomp of the EE driver and VU0 program 104691) and disc-wide scans.

### Rust loader

`crates/rc-formats/src/moby.rs` (`parse_moby_class`, `parse_level_mobys`, `moby_triangles`, `moby_normal`) mirrors the retired C++ loader. `rc_extract moby` wrote `extracted/levels/NN/moby_dump.bin` (every parsed field and every resolved vertex/triangle record), and the golden test `mobys_for_every_level` in `crates/rc-formats/tests/formats/golden.rs` parses all 19 levels from the compressed core data and required every section of every packet of every class to be byte-identical to that dump (now: to the committed snapshot hashes of the same sections). It also checks the game's trig table against `moby_normal`. The loader exposes, per vertex: packed position (world = packed · scale / 1024), normal angles, the 9-bit cache id, the resolved skin `{count, joints[3], weights[3] /256}` and the ST (4.12); per triangle: three vertex indices and the texture index (class texture slot, or −1/−2/−3).

## 9. Unknowns — consolidated

**Class header:** semantics of `lod_trans` (0x0e); the contents of the shadow block; `rac1_short_2e` (0x2e, the slot that becomes the corncob pointer in later games); every bit of `mode_bits` (0x44), `type` (0x46) and `mode_bits2` (0x47). `MobyClassEntry::unknown_8` / `unknown_c` in the core index are also unknown.

**Packets:** index-header byte 0x00; the secret-index slots past the first *block-count* quadwords of the AD-GIF unpack (never read); the GC/UYA/DL vertex-table field 0xe (speculatively `glow_rgba`); the meaning of the `unused_4`..`unused_7` bytes in regular vertices; whether anything writes the RGBA multiplier blob at runtime.

**Skeleton:** the role of `MobyTrans::vector`; `MobyTrans::seventy`; the contents and purpose of the per-joint `thing_one` / `thing_two` byte lists, and why their outer list count is independent of `joint_count`.

**Animation — the largest gap:** the **joint transform encoding is not decoded at all**. The regular format gives exactly 8 bytes per joint per frame, but the bit layout (quaternion vs Euler, component widths, scale factors, whether translation/scale are present) is unknown. Also unknown: `Rac123MobyFrameHeader` fields `unknown_0` (f32), `unknown_4`, `unknown_c`; the `thing_1` / `thing_2` 8-byte records in both the regular and special formats; the special format's `inverse_unknown_0` and `unknown_4`; the special sequence's 3 × `u16` per-joint data; the special frame's first/second/third parts; the exact meaning of the per-joint bit in the fourth part (strongly implied to be a "changed" flag); the three 2-bit channel widths in the fifth-part groups and bits 6–7 of their flag byte; `MobySequenceHeader::unknown_13` and `animation_info`; and all eight `u32`s of `MobyTriggerData`.

**Collision:** `MobyCollisionHeader` fields `unknown_0` / `unknown_2`; the entire first and third parts (only the middle vertex array is decoded); how the parts reference each other; whether hitbox information exists here at all.

**Bangles:** `MobyBangleHeader` entirely (Wrench writes it zeroed and never reads it), including its `packet_begin` / `packet_count` which appear redundant with the per-slot indices; the meaning of the two `MobyVec4`s per bangle.

**Corncob (R&C2+):** what the "corn" system is; why the vertex count is hidden in the first vertex's `w`.

**Instances:** RAC1 `unknown_4`, `unknown_8`, `unknown_c`, `unknown_10`, `unknown_14`, `unknown_54`, `unknown_74`; all `mode_bits` except 0x20 (`HAS_SUB_VARS`); the precise semantics of `occlusion` beyond "0 = precompute"; why `unused_28`/`unused_2c` are fixed at 32/64.

**Pvars:** the real names and layouts of the seven RAC1 sub-vars structs (`RacVars00`…`RacVars18`), including the size of `RacVars04`; everything about a pvar block outside the recognised sub-vars/link/pointer offsets.

**Team palettes:** not applicable to RAC1, and even for UYA/DL Wrench currently round-trips only the descriptor byte, not the palette contents.

---

### Source map

| Fact area | Wrench file(s) |
| --- | --- |
| Class header struct, file layout, read/write order, bangles, corncob, collision, joints table, joint recovery, mesh-section assembly | `src/engine/moby_low.h`, `src/engine/moby_low.cpp` |
| Packet table, VIF list structure, index header, AD-GIF blocks, metal packets, bangle structs, GIF usage emission | `src/engine/moby_packet.h`, `src/engine/moby_packet.cpp` |
| Vertex table headers (RAC1 vs GC/UYA/DL), `MobyVertex` layouts, normal/position packing, epilogue-vertex/ID quirk, duplicate vertices, metal vertices, `MobyGifUsage`, `MobyMatrixTransfer`, `MobyFormat` | `src/engine/moby_vertex.h`, `src/engine/moby_vertex.cpp` |
| VU0 blend replay, matrix allocation/liveness/scheduling, 0xf4 sentinel, alignment rules | `src/engine/moby_skinning.h`, `src/engine/moby_skinning.cpp` |
| Sequence table/header, regular and special frame formats, trigger data, DL sequences | `src/engine/moby_animation.h`, `src/engine/moby_animation.cpp` |
| Mesh reconstruction from indices, tristrip constraints / VU1 budget, packet splitting | `src/engine/moby_high.h`, `src/engine/moby_high.cpp` |
| `Vec3f`/`Vec4f`/`Mat3`/`Mat4`; GIF A+D register addresses and `GifAdData12`; VIF unpack modes and 12-bit fixed point | `src/engine/basic_types.h`, `src/engine/gif.h`, `src/core/vif.h` |
| Class-table unpack/pack, 0x40 alignment, texture index wiring | `src/wrenchbuild/level/level_classes.cpp` |
| Asset-level export/import pipeline, special/invalid materials, round-trip test | `src/wrenchbuild/classes/moby_class.cpp` |
| `MobyClassEntry`, `RacGadgetHeader`, gadget unpacking, moby GS stash, chrome map | `src/wrenchbuild/level/level_core.h`, `src/wrenchbuild/level/level_core.cpp` |
| Mesh-only (armor/wrench) classes and their scale hints | `src/wrenchbuild/globals/armor_wad.cpp` |
| Gameplay block tables per game, block read/write driver | `src/instancemgr/gameplay.h`, `src/instancemgr/gameplay.cpp` |
| `RacMobyInstance`, moby block header, class list, groups, pvar table/data/fixups, shared data | `src/instancemgr/gameplay_impl_classes.inl` |
| Transform/rotation semantics, `Rgb96`, table/instance block helpers | `src/instancemgr/gameplay_impl_common.inl`, `src/instancemgr/instance.cpp` |
| Instance field list, `MOBY_MB1_HAS_SUB_VARS` | `src/instancemgr/instance_schema.wtf`, `src/instancemgr/instance.h`, `docs/instance_reference.md` |
| Pvar sub-vars specs, fixup/type recovery, shared data slicing | `src/instancemgr/pvar.h`, `src/instancemgr/pvar.cpp` |
| Prose descriptions of the renderer, blending pseudocode, header/packet/vertex tables | `docs/moby_renderer.md`, `docs/gameplay.md` |
