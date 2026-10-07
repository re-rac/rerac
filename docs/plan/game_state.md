# Game state, new game and memory-card saves (RAC1, SCUS_971.99)

Reversed 2026-09-26 from the boot ELF (`work/decomp/SCUS_971.99`), level01/level00 overlays and the disc's
`save_game` lump (global ToC +0x10). Addresses are EE addresses. `L01 0x…` = level01 overlay, `[0x…]` = boot copy.
gp = 0x166c00 (`gp−0x7e7c` = 0x15ed84). Confidence: **H** read directly from code/data and cross-checked,
**M** inferred from a few uses, **L** guess. Chunk names were cross-checked against Wrench's `savegame.wtf`
(format reference only); where code disagrees, the code wins (noted).

## 0. Summary

* The whole persistent state is a **list of (pointer, size, id) descriptors**, one list for global data (47
  chunks, boot 0x1a04c0 / L01 0x184a40) and one for per-level data (11 chunks × 20 level slots, boot 0x1a07c0 /
  L01 0x184d40; entry `ptr + level·size`). The tables are byte-identical in the boot ELF and all 19 overlays. [H]
* Everything the tables point at lives in `core.data`/`core.bss`/`core.lit` (0x12f480..0x15eee0), which level
  overlays never overwrite, so the state survives level loads without copying. [H]
* **New game = restore the disc template save** (`save_game.bin` +0x8534, 0xea08 bytes, checksums valid) and set
  level 0. The template equals the ELF's initial data except `level = −1` (empty-slot marker). [H]
* Save file = `save%d.bin` (5 slots) in `/BASCUS-97199RATCHET/`: 8-byte header + global section + 20 level sections,
  each section `{u32 size, u32 crc16, tagged chunks…, {−1, 0}}`. CRC reproduced bit-exactly (§3.3). [H]
* Hero position is **not** saved: every level load spawns Ratchet at the level's uid-0 moby. [H]

## 1. Global chunks (table 0x1a04c0; all saved; order = file order)

| id | addr | size | meaning | default | conf |
|---|---|---|---|---|---|
| 0 | 0x15ed84 | 4 | current level index (s32) | 0 (template −1) | H |
| 1 | 0x15ed98 | 4 | bolts (s32) | 0 | H |
| 2 | 0x15ee20 | 4 | times completed / challenge-mode count (NG+ does +1) | 0 | H |
| 3 | 0x15ee24 | 4 | elapsed time, 60 Hz ticks (PAL adds 10 per 50 frames, `0x226e08`; +180 per level load) | 0 | H |
| 4 | 0x15ee98 | 8 | last-save time, `sceCdCLOCK` (BCD) from `sceCdReadClock` + local-time fix | 0 | H |
| 5 | 0x13d388 | 128 | global flags u8[128]: [4] premium nanotech, [5] ultra nanotech, [8] Veldin Clank dialog done (§4), others per level scripts | 0 | H [4,5,8] / L rest |
| 7 | 0x15edb0 | 12 | cheats active u8[12]; [4] 0x15edb4 = mirror (pad/rotation), [5] 0x15edb5 = mirrored anim swap | 0 | M |
| 8 | 0x13d408 | 32 | skill points u8[32] (30 used; `count_nonzero_up_to_30`) | 0 | M |
| 9 | 0x13d428 | 148 | ammo s32[37] by item id | 0 | H |
| 10 | 0x13d4c0 | 37 | **item owned** u8[37] (gates use; e.g. [2] heli, [4] …, [27] metal detector = HUD bolt alert 0x13d4db) | 0 | H |
| 11 | 0x13d4e8 | 37 | item ever acquired u8[37] (set with owned; also set for gold re-grant) | 0 | H |
| 12 | 0x15edd0 | 12 | vendor stock u8[12]: low 6 bits item, 0x40 = owned (sells ammo), 0xff empty | 0xff×12 | H |
| 13 | 0x141ea0 | 32 | quick-select s32[8] (item ids, 0 = empty) | 0 | H |
| 19 | 0x15eda0 | 4 | **max HP** (4; 5 premium, 8 ultra nanotech) | 4 | H |
| 14 | 0x13dd40 | 20 | **planet unlocked** u8[20] (infobot / arrival, `0x2756d0`) → ship destinations | 0 | H |
| 20 | 0x13d510 | 80 | galaxy-map order s32[20]: planet ids in unlock order | 0 | H |
| 15 | 0x13d5b0 | 1936 | map landmarks 121×{f32 x, f32 y, f32 rot, u32 flags}, copied from mobys at save (`0x208770`) | 0 | M |
| 16 | 0x141968 | 1184 | help-message records 148×{u16 count, u16 time/600, u32 level mask\|0x80000000} | 0 | M |
| 17 | 0x141848 | 288 | misc records 36× same layout (hero moves etc.) | 0 | M |
| 18 | 0x141720 | 296 | gadget-help records 37× same layout | 0 | M |
| 21 | 0x15ed8c | 4 | last hand item (`0x2307e0`) | 0 | M |
| 22 | 0x15ed90 | 4 | wrench held (hand shows item 8) | 1 | H |
| 23 | 0x15ed94 | 4 | thruster pack last (back shows item 3 instead of 2) | 0 | H |
| 24 | 0x15ed9c | 4 | unused (no reference) | 0 | H |
| 25 | 0x15ede0 | 4 | camera left/right: 1 Normal, 0 Reversed (yaw input sign) | 1 | H |
| 26 | 0x15eddc | 4 | camera up/down: 1 Normal, 0 Reversed (pitch input sign) | 1 | H |
| 27 | 0x15ede4 | 4 | camera rotation speed index 0/1/2 | 1 | H |
| 28 | 0x15ee1c | 1 | HelpDesk voice | 1 | H |
| 29 | 0x15ee1d | 1 | HelpDesk text | 1 | H |
| 30 | 0x13e520 | 40 | gold weapon owned u8[40] | 0 | M |
| 31 | 0x15eea0 | 4 | game beaten (enables cutscene skip / cheat entry) | 0 | M |
| 32 | 0x141660 | 28 | equipped s32[7]: hand, feet, head, back, 3 unused | 0 | H (0-3) |
| 33 | 0x15ee40 | 1 | subtitles | 0 | H |
| 34 | 0x15ede8 | 4 | stereo (1) / mono | 1 | H |
| 35 | 0x15edec | 4 | music volume 0..0x400 | 0x2cc | H |
| 36 | 0x15edf0 | 4 | effects volume 0..0x400 | 0x400 | H |
| 37 | 0x15edc0 | 12 | cheats ever activated u8[12] | 0 | M |
| 1000-1002 | 0x13dd70 / 0x13de08 / 0x13dea0 | 148 each | ammo bought / picked up / used s32[37] | 0 | M |
| 1003 | 0x15eea4 | 4 | play time, game ticks (++ each gameplay tick, `0x2ab960`) | 0 | H |
| 1004 / 1005 | 0x15eea8 / 0x15eeac | 4 | total hits taken / deaths | 0 | H |
| 1008 / 1009 | 0x15ee28 / 0x15ee2c | 4 | bolt-rate window ticks / bolts in window (`0x275988` rate) | 0 | M |
| 1010 / 1011 | 0x141e08 / 0x15ee30 | 150 / 4 | help log u8[150] / log position | 0 / 1 | M |

Item ids (help text, gadget table): 2 heli-pack, 3 thruster, 4 hydro, 5 sonic summoner, 6 O2 mask, 7 pilot's helmet,
8 wrench, 9 suck cannon, 10 bomb glove, 11 devastator, 12 swingshot, 13 visibomb, 14 taunter, 15 blaster,
16 pyrocitor, 17 mine glove, 18 walloper, 19 tesla claw, 20 glove of doom, 21 morph-o-ray, 22 hydrodisplacer,
23 RYNO, 24 drone, 25 decoy, 26 trespasser, 27 metal detector, 28 magneboots, 29 grindboots, 30 hoverboard,
31 hologuise, 32 PDA, 33 map-o-matic, 34 bolt grabber, 35 persuader. [M: via help texts; slot types from 0x179f48]

**Session-only (not saved)**: language 0x15ed88 (from the PS2 system config in `InitOnce`), PAL flag 0x15ed80/0x15ee90,
the whole hero block 0x13f350..0x141660 incl. HP 0x1415f8 (zeroed and HP = max HP at every level start,
`FUN_00226b70`), temporary items 0x141408/0x141424/0x141430, destination 0x15f600, game mode 0x15f5c4. [H]

## 2. Per-level chunks (table 0x1a07c0; entry = `ptr + L·size`, L = 0..19; 19 levels used)

| id | addr (L=0) | size | meaning | conf |
|---|---|---|---|---|
| 3001 | 0x13dd58 | 1 | visited: 0 no, 1 visited (set by level `entry`), 2 left/completed (`DoSpaceTransition`) | H |
| 3002 | 0x141ec0 | 0x800 | compressed per-level moby/map state (`0x207b08` packs 0x1a0104 → 0x800 via `0x1fa860`; zero-filled if 0x1a0118 = 0; map screen reads it) | M |
| 3003 | 0x14bec0 | 4 | gold bolts collected u8[4] (class 1134; counts per level at L01 0x1c4e08 = 0,3,4,3,1,2,2,2,2,2,2,1,2,1,4,2,2,2,3 = 40) | H |
| 3004 | 0x14c050 | 16 | mission bytes: 0xff = done (`0x265080`, moby+0xb0 index) | H |
| 3005 | 0x14c190 | 0x100 | killed bitset by spawn id (moby+0xb2) — no respawn (`0x26c250`) | H |
| 3006 | 0x14d590 | 0x100 | 64×{s16, s16 bolts collected from drop group moby+0xb1} (bolt pickup `0x2bc4f0`, crate drop scaling) | M |
| 3007 | 0x141680 | 8 | {u16 entries, u16 time/600, u32 level mask\|0x80000000} (level `entry`) | M |
| 3008 | 0x14bf10 | 16 | metal-detector (buried bolt) bytes (`0x2f21c0`/`0x2f2eb8`) | L |
| 4000 / 4001 / 4002 | 0x13df38 / 0x13df88 / 0x13dfd8 | 4 | bolts collected / hits / deaths on this level | H |

## 3. Memory-card format

### 3.1 Files [H]
Directory `/BA` + `SCUS-97199` + `RATCHET` built by `memcard_GetName` 0x209030 from the SYSTEM.CNF boot line
(`BOOT2 = cdrom0:\SCUS_971.99;1`, chars 16..26; `BE…` when char 18 is `E`). Created by `memcard_Update`
0x2093d8 state 10 when ≥ 0x15e (350) free clusters:
* `icon.sys` 0x3c4 bytes (title "Ratchet & Clank" full-width SJIS, break at 32; all three icons `static.ico`) and
  `static.ico` 0x8158 bytes — both copied from `save_game.bin` (header: `+0 icon.sys off 0x18, +4 size 0x3c4,
  +8 ico off 0x3dc, +0xc size 0x8158, +0x10 template off 0x8534, +0x14 size 0xea08`).
* `save0.bin`..`save4.bin`, each written with the template (all empty, level −1).
* `BASCUS-97199RATCHET` (same name as the dir): 0x3c04 bytes (NTSC; 0x3c00 PAL) of raw RAM from 0x15ed84 — never
  read back; a space reservation. [M on purpose]

### 3.2 `save%d.bin` layout [H]
```
+0   u32 global_size  (= GetDataSize(global) = 0x1530)     +4 u32 level_size (= 0xaa4)
+8   global section (0x1530 bytes)   then 20 × level section (0xaa4 bytes)   total 0xea08
section: u32 data_size (= section − 8), u32 crc16(data), data = chunks, then {s32 −1, u32 0}
chunk:   s32 id, u32 size, bytes[size], zero pad to 4
```
Global chunks 0..4 sit at fixed offsets (level +0x10, bolts +0x1c, completes +0x28, elapsed +0x34, clock +0x40),
which `memcard_RestoreInfo` 0x20ae60 uses for the slot preview (0x13d2b0 + slot·0x1c: level, bolts, completes,
elapsed, clock; per card 0xb8). `GetDataSize` 0x20ac88 = 8 + Σ(align4(size) + 8) + 8. No version field: the
size pair is the version check.

### 3.3 Checksum `memcard_Checksum` 0x20acc0 [H, verified on the template: 0x9ad4 global, 0xdce3 level]
CRC-16, MSB first, init 0x8320 (the code seeds 0xedb88320; only the low 16 bits matter), poly 0x1f45, no final
xor, over `data_size` bytes; returns 0 when length > 0x1800. `TestChecksum` fails when the stored CRC is 0 or differs.

### 3.4 Save paths
* `memcard_Save(force, level)` 0x20b178 (L01 0x261448): stamps the clock, stores landmarks and packs level state,
  then if a card/slot is active: optionally pretends `level` (sets 0x15ed84 and visited = 1 temporarily),
  updates the slot preview, `PrepData` global → 0x14eed0 and current level → 0x1506d0, queues state 0x10:
  open, seek 8, write global, seek `level·level_size`, write that one level section. **Incremental**: other level
  sections on the card are untouched. Callers: vendor purchase, mission NPC/infobot pickup, ship take-off
  (`memcard_Save(0, dest)`), game end.
* Whole saves (`MakeWholeSave` 0x20abb0 into buffer 0x1d5cd0, state 0x13 writes all 0xea08): Save menu
  `0x2269c0`, New Game into slot `0x226a70`, challenge mode `0x226b08`.

### 3.5 Load path [H]
State 0xd/0xe: open `save%d.bin`, read 8, check sizes (> 0x1800 / > 0x1000 traps), read global → 0x14eed0 →
`RestoreData` 0x20af20, then 20 × read level section → 0x1506d0 → `RestoreData(…, L, table)`. `RestoreData`: bad
CRC → 1 error, nothing copied; else chunks matched **by id**; size equal → copy; saved smaller → copy saved size;
saved larger → copy table size; unknown ids and size-sum mismatch count as errors (0x13d33c[card]).
Then `LoadingDataMenu` 0x2232d8 → `initialize_global_state_entry(0x15ed84)` → `DoSpaceTransition` loads the
level; hero at the level spawn.

## 4. New game, and the state on first arrival at Novalis

**Reset** `load_and_initialize_level_chunk` 0x209370: read the `save_game` lump, `RestoreGame` 0x209298 on the
template, `level = 0`. Run at boot (`transition_do_transition`), by New Game (`0x226a70`, then a whole save to the
slot with preview level 0) and on quit. Defaults are §1's column (all zero except: max HP 4, wrench held 1,
camera 1/1/1, HelpDesk voice/text 1, stereo 1, music 0x2cc, effects 0x400, help-log pos 1, vendor 0xff×12). [H]
**Challenge mode** `0x226b08` (after the game): reset, then restores gold weapons, owned flags of items
9-11,13-21,23-25,32-35 (list 0x1d5ba0), ammo, all gold bolts, quick-select (unowned → 0), vendor stock, skill
points and bolts; completes +1; max HP 5/8 if premium/ultra nanotech flags were set; HelpDesk voice/text = 0. [H]

**Every level start** (L01 `entry` 0x259c40 → `FUN_00251c30`, `FUN_00251da0`, `FUN_00226b70`): [H]
1. If bomb glove not owned: `GiveItem(10, equip)` (L01 0x275760: owned/acquired = 1, ammo = max(ammo, 10) from item
   table 0x1c4538+0xa, first empty quick-select, hand 0x141408), `vendor[0] = 0x4a`, equipped hand 0x141660 = 10.
2. Vendor stock pruned of items no level sells, then this level's item added (`0x1c4318[L]`: 0,16,0,15,20,17,14,0,
   11,18,13,25,24,0,19 → Novalis adds 16 pyrocitor, unowned).
3. If level ≠ 0: unlock planet L (`0x13dd40[L] = 1`, append to 0x13d510). Elapsed += 180.
4. Hero block cleared; HP = max HP; spawn at the uid-0 moby; empty hand → 10.
5. `visited[L] = 1` if 0; record 3007 updated. Level 1 with Kerwan (planet 3) still locked skips the fly-in intro
   (mode 0 at once, music unpaused) — the first-arrival case. [M]

**Veldin → Novalis** (`DoSpaceTransition` 0x231ff0, branch `level 0 → 1, visited[1] == 0`): space cutscenes 5-8,
`visited[0] = 2`, level = 1. So a fresh first arrival has: bolts as collected on Veldin; max HP 4 and HP 4; owned
wrench (always) + bomb glove (ammo 10 + Veldin pickups); quick-select [10]; vendor [0x4a, 0x10, 0xff…];
planet 1 unlocked (map order [1]); visited = {0: 2, 1: 1}; global flag 8 set; no gold bolts, skill points or
missions unless done on Veldin. Veldin's own bolts/crates/kills sit in level-0 sections and do not matter on
Novalis. [H for the code path; the bolt and ammo numbers need a PCSX2 dump]

**Clank**: there is **no** "has Clank" save flag. The hero item code (`FUN_0022f3c0`) always creates item 1's class
(601, Clank) as a second back moby plus the back item (`0x141430` → `0x14166c` → 2, or 3 if 0x15ed94). Both are
hidden (mode |= 0x41) while hero s16 0x141628 ≠ 0. Only Veldin's class 834 (L00 `0x2d9dc8`, HelpDesk-style dialog)
sets 0x141628 = 1 at spawn and sets global flag 8. The hero-block clear resets it, so **Clank shows on the back
on Novalis**. Heli/thruster use is gated separately by owned[2]/[3]. [H for code; M for the class-834 story role]

## 5. Options and their effect on ported code

| option | var | values | effect | port |
|---|---|---|---|---|
| Left/Right movement | 0x15ede0 | 0 Reversed, 1 Normal | `0x313b88`: rx sign | `CameraOptions::yaw_normal` [H] |
| Up/Down movement | 0x15eddc | 0 Reversed, 1 Normal | `0x313b88`/`0x316330`: ry sign | `pitch_normal` [H] |
| Rotation speed | 0x15ede4 | 0 Slow, 1 Medium, 2 Fast | `0x314e00`: yaw rate = f32 table 0x162228 {1.0°, 1.3°, 1.6°}/tick | `YAW_RATES` [H] |
| HelpDesk voice / text | 0x15ee1c / 0x15ee1d | 0/1 | help box drawn only with text on (`0x2266c0`, `Help_Update`) | HUD todo [H] |
| Subtitles | 0x15ee40 | 0/1 | menu item 0x1b5868 | cutscene todo [M] |
| Sound | 0x15ede8, 0x15edec, 0x15edf0 | stereo; 0..0x400, step 3 | master groups (audio.md) | audio todo [H] |
| Mirror (cheat) | 0x15edb4 / 0x15edb5 | 0/1 | pad L/R swap, anim swap | `pad.rs` [H] |
| Vibration | — | — | the page at ~0x1b58d0 ("Controller vibration" 20385) has a **null** variable and is not linked from the options list 0x1b4f18 (HelpDesk, Save, Load, Sound, Camera, Subtitles, Quit); no libvib actuator call is linked (only `sceVibGetProfile`). Not implemented in this build. | none [M] |

Menu items are `{u32 text id, u32 *var, u32 text for value 0, value 1, …}` (e.g. 0x1b5fc0). [H]

## 6. Levels and the ship menu

Level ids 0-18 = Veldin, Novalis, Aridia, Kerwan, Eudora, Rilgar, Nebula G34, Umbris, Batalia, Gaspar, Orxon,
Pokitaru, Hoven, Oltanis Orbit, Oltanis, Quartu, Kalebo III, Veldin Orbit, Veldin (finale). Text ids (all_text,
English): planet name 20172 + L (L = 1..18; 20172 = "Planet"), area name 20153 + L (Kyzil Plateau, Tobruk Crater,
Outpost X11, Metropolis, …, Drek's Fleet, Kyzil Plateau). [M: consecutive runs; the lookup code is not found]
Selectable destinations = planets with `0x13dd40[p] ≠ 0` (the pause map cycles them, `0x28fec8`); order on the
galaxy page = 0x13d510. First-visit story flights exist for 0, 0→1, 4, from 7, 13, from 14, 16
(`DoSpaceTransition`). Ship flow and planet page: menus.md §5. Galaxy-map coordinates: **not found** (page
0x1b6508 layout still open). [H for flags; L for coordinates]

## 6.1 What comes back after a death (the death reload) [H]

A death (0x141401 set, game mode 0) runs `LoadLevelCoreData(0, 1)` from the main loop (level01 entry 0x259c40). With
`param_1 = 0` it reads neither the core data nor the save's mission bytes again; `InitLevelRenderGlobals(0)` reads the
gameplay file from the disc again (paths, volumes, camera records, the instance records), zeroes 0x13f350..0x141660,
empties the particle pool and creates the ship; the instance loop's spawn test then decides per placed record whether
it is created again. Three word sets decide it:

| word | written by | read by |
|---|---|---|
| 0x15fc88[16] (the spawn test's mission bytes, `g_spawn_test_table`) | copied from 0x14c050 + L·16 by a full load only | the spawn test (mission-gated records); `SetDeathBits` |
| 0x14c050 + L·16 (the save's mission bytes) | `SetMissionDone` 0x265080 (alone) | `SetDeathBits`; the classes |
| 0x1ba950 (this visit's death bits), 0x14c190 + L·0x100 (the persistent death bits), 0x1baea4[id] | `SetDeathBits` 0x26c250, always | the spawn test (flags 8 / 4); halving the bolt count |
| 0x1bbb04[id] (never spawn again) | `SetDeathBits`, only when the moby's mission is 0xff, or was not done at the arrival (0x15fc88) and is done now (0x14c050) | the spawn test, first |

So, per record (`spawn_test`, rc-formats moby_spawn.rs): flagged 0x1bbb04 → gone; else mission-gated (flags & 3) →
by the **arrival's** mission state (flag 1 before, flag 2 after), regardless of deaths this visit; else flag 8 → gone
once killed this visit; flags & 0xc = 4 → gone once ever killed; else always back. Examples (Kerwan): the train's
troopers (flags 3, mission 3) come back after every death; a nanotech crate (flags 3, mission 1) comes back after a
death until mission 1 is done, after which a broken one stays gone. **Checkpoints** (805) also make kills permanent:
taking one calls `SetMissionDone(its mission)` and then its record 0x29ac10, which turns every kill of this visit
(0x1baea4 ≠ 0) whose mission is done now (0x14c050), or has none, into "never again" (0x1bbb04 and both death bits);
its own mission counts, as `SetMissionDone` writes at once. **A death forgets the kills no checkpoint promoted**:
`0x29adc8` copies the checkpoint area 0x1bb6b0..0x1bc310 over 0x1baa50..0x1bb6b0 (zeroes it without a checkpoint), so
the visit records 0x1baaa0 become the checkpoint's copy 0x1bb700 and the kills 0x1baea4 become a copy of the never-again
bytes 0x1bbb04; the death bits 0x1ba950 / 0x14c190 stay. Measured in PCSX2 over PINE (Kerwan, every mission done; the
visit tables are 0x380 lower in level03): the train troopers' 0x1baea4 = 5 after killing them, 0 after the respawn,
never-again 0 throughout, so they come back after every death (probe: `work/scratch/kerwan_probe.py`). Kerwan's missions: 0 the Heli-Pack checkpoint, 1 the train
station's checkpoint, 2 Helga's course checkpoint, 3 the train's infobot. The port's reload: `rc-engine` gameplay.rs
`death_reload` (G-CLS-030); the same-tick read: `World::mission_done`.

## 7. Port plan — `crates/rc-game/src/game_state.rs`

* **Format in `rc-formats`** (`save_game.rs`): `CHUNKS_GLOBAL: [(id, size); 47]`, `CHUNKS_LEVEL: [(id, size); 11]` as
  const tables in file order, `crc16(&[u8]) -> u16`, `encode_section(chunks) -> Vec<u8>`,
  `decode_section(&[u8]) -> (Vec<(i32, &[u8])>, errors)`, and the `save_game` lump reader (icon.sys, static.ico,
  template). No serde; plain little-endian byte writes.
* **One struct** `GameState { g: Global, lv: [Level; 20] }`: every chunk is a named field with its exact type
  (`bolts: i32`, `ammo: [i32; 37]`, `owned: [u8; 37]`, `vendor: [u8; 12]`, `landmarks: [Landmark; 121]`,
  `help: [HelpRec; 148]`, …; opaque ones as `[u8; N]`, e.g. `map_mask: [u8; 0x800]`). One `const fn` per chunk
  maps id → `&mut [u8]` view via explicit (de)serialisers so `encode(decode(x)) == x` byte for byte.
* **Restore rules** copy `RestoreData` exactly (id match, min/truncate on size mismatch, error count); bad CRC
  keeps defaults for that section.
* `GameState::new_game(&SaveGameLump)` decodes the disc template and sets `level = 0` (defaults come from the
  disc, never hard-coded). `on_level_start(level)` implements §4 steps 1-5; `give_item(id, equip)` = L01 0x275760
  (needs the item tables 0x1c4530 / 0x179f40 / 0x1c4318 from the overlay; `GiveItem` reads `+8` of the first two). `boot_novalis_first_arrival()` =
  `new_game()` + `on_level_start(0)` + Veldin exit (`visited[0] = 2`, flag 8 = 1) + `on_level_start(1)`.
* **Import**: `from_card_file(&[u8])` for `save%d.bin`; a later `ps2mc.rs` reads a PCSX2 `.ps2` image (8 MB,
  512+16-byte pages with ECC, 1 KB clusters, FAT, directory entries) and extracts `/BASCUS-97199RATCHET/save*.bin`.
  Export writes the same bytes, so saves round-trip to real hardware.
* **Tests (golden)**: template sizes 0x1530/0xaa4/0xea08 and CRCs 0x9ad4/0xdce3; decode→encode identity on the
  template; `new_game()` equals the boot ELF data at every chunk address (except level); a PCSX2 dump of a first
  Novalis arrival compared field by field.
* The options (§5) feed `CameraOptions`, the pad mirror and the HUD directly from `GameState`.

## 8. Unknowns

* Exact bolts/ammo/gold bolts on first Novalis arrival (depends on Veldin play): take a PCSX2 dump of
  0x15ed98, 0x13d428, 0x14bec0 at the Novalis spawn.
* Global flags other than 4, 5, 8; cheat slots other than 4/5; skill-point index order; chunk 3002's packed
  format (`0x1fa860` codec and the 0x1a0104 source) and 3006/3008 details; unknown equip slots 4-6.
* Where planet/area names are looked up; galaxy-map positions; class 834's identity on Veldin.
* Why the `BASCUS-97199RATCHET` main file is a RAM dump (harmless; never read).

## In the port (2026-09-26)

**Code.** `crates/rc-formats/src/save_game.rs` (format, tables, lump, item tables), `Disc::save_game_lump`
(+ `toc::global_sector_range` / `SAVE_GAME_FIELD`), `crates/rc-game/src/game_state.rs` (state and rules),
`crates/rc-game/tests/ui/game_state_novalis.rs`. Nothing is copied from the disc: descriptor tables, template,
vendor list and item records come from the boot ELF / `save_game` lump / level overlay at run time.

* `ChunkTables::from_boot_elf` reads both tables (47 global, 11 per level; `GetDataSize` 0x1530 / 0xaa4, file
  0xea08). `SaveGameLump::parse` splits the lump (icon.sys 0x3c4 starting `PS2D`, static.ico 0x8158, template
  0xea08). `crc16`, `Section::{encode, decode}`, `SaveFile::{parse, to_bytes, write_incremental}`,
  `card_dir_name(SYSTEM.CNF)` = `/BASCUS-97199RATCHET`, `save_file_name(slot)`.
* `GameState { tables, global: Global, levels: Vec<LevelState> /*20*/, raw, pads }`: one typed field per chunk
  id (`chunk_struct!`); ids the tables carry but the port does not type (none on retail), or whose table size
  differs from the typed field, stay raw bytes. `restore_section` = `RestoreData` (bad CRC → nothing, match by
  id, copy `min(saved, table)`, error counts); `load_card_file` = `RestoreGame` with the size-pair check;
  `new_game(tables, template)`; `to_save_file` = `MakeWholeSave`; `save_incremental(card, pretend, clock)` =
  `memcard_Save` + state 0x10.
* Rules: `give_item` (0x275760), `unlock_planet` (0x2756d0), `apply_level_start` (0x251da0 → 0x251fe0 →
  unlock if L ≠ 0 → elapsed += 180 → hero init → visited / record 3007, incl. the debug branch for L ≥ 19),
  `apply_transition(dest)` (`DoSpaceTransition`, from = current level), `on_veldin_clank_init` (class 834),
  `options()` / `set_options()` → `Options::game_options()` feeds `tick::GameOptions` / `CameraOptions`.
  `SessionState` holds HP, the hero block's temp items (0x141408 / 0x141410 / 0x141414), 0x141628 and the tick
  scale (0x15ed68, 1.0 NTSC; `fun_001f96f8` in PS2 float arithmetic).

**Verified.** Template CRCs 0x9ad4 / 0xdce3; every section's CRC passes; decode → encode of the template and
`GameState` round trips are byte-identical; template chunk data = boot ELF initial data at every chunk address
for all 20 slots (only level differs: −1 vs 0); template pad bytes are zero. First Novalis arrival (new game →
L0 start → class 834 → transition → L1 start): HP 4/4, wrench held, bomb glove owned/acquired with ammo 10,
equipped hand 10, quick-select [10, 0…], vendor [0x4a, 0x10, 0xff×10], planet 1 unlocked, map order [1],
visited [2, 1, 0…], flag 8, elapsed 360, records 3007 of L0/L1 count 1 with masks 0x80000001 / 0x80000002;
whole save → parse → state identical; incremental save onto the template writes global + level 1 only.

**Corrections / precisions to the sections above.**
* Descriptors are **16 bytes** `{u32 addr, u32 size, s32 id, s32 restore_status}` (the fourth word is
  `RestoreData` scratch: 1 equal, −1 saved smaller, −2 saved larger); the table ends at `addr == 0`.
* The item definition records start at **0x179f40** (slot type at +8 = 0x179f48; L01 addresses) and move per
  overlay (L00 0x179ac0, L05 0x179f20, …); the boot copy (0x1863d0, the base 28 boot `lui/addiu` pairs load) is zero in the file. `ItemTables::load`
  locates them through `GiveItem`'s code (`lui/addiu` of the vendor table, then `lui rC` · `addiu rD,rC,lo` ·
  `lw …,8(rD)`), checked on all 19 overlays. The vendor list (L01 0x1c4318) and the 0x18-byte price records
  (L01 0x1c4530; `GiveItem` reads `+8` s16 "has ammo" and `+0x12` u16 granted ammo) are byte-identical in the
  boot ELF (0x1dfd98 / 0x1dffb0) and every overlay, so they are read from the boot ELF.
* `GiveItem` only adds a vendor slot for items some level sells; the bomb glove is not in the list, so its
  `vendor[0] = 0x4a` comes from 0x251da0 alone. The `GiveItem` temp-hand write (0x141408) is wiped by the
  hero-block clear later in the same level start.
* `memcard_Save(force, pretend)` writes the **current** level's section (saved in 0x13d358 before pretending);
  pretend changes only the global section's level and `visited[pretend]`, which reaches the card only when
  pretend equals the current level.
* Flag 8 is not a transition effect: class 834 sets it (and 0x141628 = 1) on its first update on Veldin.
* Wrench: `owned[8]` is 0 in the template; "wrench held" is chunk 22 (= 1).
* `PrepData` never writes the chunk pad bytes; whole saves go through buffer 0x1d5cd0 (overlay memory), so real
  card files may carry non-zero pads. `Chunk::pad` / `GameState::pads` keep them for byte-exact round trips.

**Approximations / not ported.** `RestoreData`'s last error scan stops at the descriptor whose id equals the
stale word after the terminator (not modelled: every table chunk not restored at its exact size counts). The
vendor searches are bounded to the 12 slots (the game has no bound; unreachable in practice). Not ported: the
3002 packer / codec (0x207b08 / 0x1fa860; the chunk saves as held), landmark capture 0x208770, challenge mode
0x226b08, the slot preview, the RAM-dump file `BASCUS-97199RATCHET`, PCSX2 `.ps2` card import/export, help
sounds and new-planet messages. The Veldin-play-dependent numbers (bolts, extra ammo) still need a PCSX2 dump.

**2026-10-01 (saves lane).** The card, the saves and the loads run natively: `rc_game::memcard` (`memcard_Update`, the
card monitor, `memcard_Save`, the whole saves, the slot previews `memcard_RestoreInfo`) on a card folder, the landmark
capture (`GameState::capture_landmarks`), challenge mode (`GameState::challenge_reset`), the front end and the card
pages: docs/plan/progression.md `## saves`. Left: G-SAV-011 (native leftovers), G-SAV-012 (the title world).
