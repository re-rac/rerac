# Level logic: is there a "level scripting" system? (RAC1 NTSC)

Investigation of 2026-09-28 (gap G-LVL-006). Addresses are level01 unless marked `L<nn>` (that level's overlay);
engine functions sit at other addresses in every overlay and were found there by code identity
(`rc_formats::level_overlay::Relocation`, the same matcher `classes::LevelPorts` uses). Confidence: **[H]** read in
the code and checked over the 19 overlays, **[M]** read in the code only, **[L]** inference.

## 1. Answer

**There is no level-scripting system.** No script VM, no bytecode or event stream, no script data in the level
files, no "director" mechanism shared between levels. The per-level logic (arrival scenes, missions, tutorial
hints, story beats) is **ordinary moby class code**: each level's overlay compiles its own update functions for
its own classes, placed once or a few times in the level, and those functions call a small set of **ordinary
engine helpers** (scene start, movie start, `SetMissionDone`, `Help_Request`, `CameraScript`, `HeroTeleport`,
`UnlockPlanet`, banners, the save, the checkpoint record, `PointInCuboid`) and read / write a few **saved-game
fields** (global flags, the help / move records, the mission bytes). What looks like a "script" is a `switch` on
the moby's state byte (+0x20) over its pvars (cuboid indices, moby links, timers).

Evidence [H unless noted]:
* **No interpreter.** No function in the boot ELF or the level01 overlay reads a command stream (names and bodies of
  `work/decomp/{SCUS_971.99,level01.elf}`: the only "command" code is SIF / sound / GS / IPU transport). The
  "director" classes read nothing but their own pvars (e.g. 1342 below: pvar words 0, 3, 6, 9, 0xa, 0xb, 0xc, 0xd,
  0xe, 0xf, 0x10, 0x11, 0x1c, 0x1d, 0x1e, 0x1f are cuboid indices, 0x14..0x16 counters / a timer).
* **No script data.** The gameplay file's 36 section pointers are all tables with a known reader (`rc_formats::
  gameplay::SECTION_NAMES`: instances, pvars, groups, volumes, paths, lights, cameras, help strings, …); the scenes
  carry no command / event track either (cutscenes.md §7.1, checked byte by byte on all 138 scenes).
* **The logic is per-level code.** Census (§3): 41 placed classes call `Help_Request` on 18 levels, 39 of them with
  update code that has no level-01 copy; the story classes that start scenes (1353 on Gemlik, 1190 on L04, 436 on
  L07, 1419 / 1446 on L15, …) are each unique to their overlay. The two Novalis "directors" are too:
  `HelpHintDirectorUpdate` 0x30acb8 (class 1341) and Kerwan's 1342 (L03 0x2df520) are different code (masked
  compare: not the same function), and `MissionNpcUpdate` 0x2fad68 (730 / 790) exists only on level 01
  (clusters.tsv 2c0aee80). Two script classes are compiled into two overlays each (the same code): 439 (L05
  0x2f87a8 = L16 0x2c7788) and 1179 (L11 0x30ee00 = L15 0x2d9100; it tests `0x15ed84 == 0xb` itself). That is the
  game's own per-level compilation, found by code identity like every other class.
* **The shared part is the engine helpers and a few generic classes**, the same object code in every overlay:
  the scene player, the movie player, the script camera, the hold 0x72, the help box, `SetMissionDone`, the
  checkpoint record, `PointInCuboid`; and the generic classes camera trigger 737, checkpoint 805, infobot 750,
  talker 774, gold bolt 1134, bolt crank 280, gold-weapon offer 304 (all ported, `classes/`).

So the "director / script moby mechanism" is the moby loop plus the class registry: a level's director runs when its
`lvl.vtbl` entry is a ported function (`classes::LevelPorts`), exactly like a crate or an enemy. Porting a level's
logic = porting its classes (G-CLS-001, `docs/plan/class_census.md`), against the helpers below.

## 2. The helpers the per-level classes call

| helper (level01) | effect | placed callers (classes on levels) | port |
|---|---|---|---|
| `DialogStreamStart(k)` 0x2ac330 | mode-2 scene k | 44 classes, 15 levels | `cinematic::start_scene` → `scene_render` (cutscenes.md §2, §7) |
| `DialogStreamUpdate(n)` 0x2acf50 | in-level movie `mpegs[2+n]` | 28, 11 | `cinematic::start_movie` → `movie_render` (§5) |
| `SetMissionDone(m)` 0x265080 | `*(0x14c050 + L·16 + m) = 0xff` (m ≠ 0xff) | 63, 18 | `cinematic::set_mission_done` (§5 here) |
| `CameraScript` 0x316ef8 / `CameraScript2` 0x317070, `0x15f404` | cutaway camera, letterbox | 63, 18 | `cinematic` + `follow_camera::script` (cutscenes.md §3) |
| `HeroTeleport` 0x2368e0 (+ hold 0x72) | place and hold Ratchet | 69, 18 | `cinematic::hero_teleport`, `hero::scripted` |
| `UnlockPlanet` 0x2756d0 / `ShowPlanetBanner` 0x277c38 | planet bits, map order, banner | 20, 13 | `cinematic::unlock_planet` / `show_planet_banner` |
| `ShowBanner` 0x2789e0 | HUD banner | 50, 18 | `cinematic::show_banner` |
| `memcard_Save(0, −1)` 0x261448 | save | 75, 18 | `cinematic::save` (logged; G-SAV-002) |
| checkpoint record `FUN_0029ac10(point, euler)` | respawn record, kills made permanent | 54, 18 | `classes::checkpoint::record` |
| `PointInCuboid` 0x274820 | the trigger test (cuboid index in a pvar) | 86, 19 | `moby_update::triggers::point_in_cuboid`, `World::in_cuboid` |
| `GiveItem` 0x275760 | item to the player | 19, 12 | NPC item gifts through the talk system / item offers only (no class caller ported) |
| `Help_Request(msg, rec)` 0x225818 | help box request (§4) | 41, 18 | `Help::request` (G-UI-017 system ported; directors §8) |
| help / move record bump (inline code, §4) | `{count, time/600, level mask}` | 44 classes read / write chunks 16 / 17 / 18 | records typed in `GameState`; bump = `hero::melee::bump_record`; class writers: the directors of §8 (`units/hints.rs`) |
| global flags 0x13d388[128] | cross-level story state (0x13d397 Novalis arrival, 0x13d3ec..ee Gemlik, 0x13d3a0 / a2 Kerwan) | 78, 19 | read through `TalkGame::flags` / `Cinematic::arrival_seen`; no general class writer (G-SAV-010) |
| `try_set_help_message` 0x278f58 / `force_help_message` 0x279000 | the △ prompt | 23 / 9, 14 / 8 | `Interact::try_prompt` / `force_prompt` (interaction.md) |
| `FUN_0026e0e0(group, cmd)` / `MobyGroupCount` 0x26e008 | moby group commands / counts | 17 / 6, 6 / 4 | count ported (`enemy_spawner::group_count`); commands G-CLS-023 |

Counts: placed classes (≥ 1 instance on the disc) per level whose update, or a private helper only it calls, calls
the helper; from the census probe over the 19 overlays (vtbl updates, `jal` targets, `lui`/`%lo` and `$gp` data
references). "Unique" code (no level-01 copy) is the majority for every helper except the generic classes' ones.

## 3. The per-level logic classes (census)

Placed classes that start scenes (S) or movies (V), set missions (D), unlock planets (P), request help (H), stage
cutaways (C) or give items (I); r / s = reads or writes help / move records, f = global flags, W = save. `*` = the
same code as a level-01 class (generic, mostly ported). Update address in that level's overlay.

| L | classes |
|---|---|
| 00 | 834 0x2d9dc8 [SVf] (Veldin's Clank; its init rule is ported: `GameState::on_veldin_clank_init`); 1413 0x2e0988 [Hrs] (ported §8) |
| 01 | 280* [DCf]; 730* / 790* 0x2fad68 [SVDPfW] (ported: `mission_npc`); 750* [SVDPW]; 774* [DPW]; 805* [D]; 1341* 0x30acb8 [Hrs] (help-hint director; ported) |
| 02 | 713 0x2ddc88 [HCrs]; 786 0x2e0dc0 [HDIrW]; 788 0x2e1950 [HDIW]; 805* [D]; 1005 0x2ea210 [HSCIW]; 1324 0x2ee890 [Hrsf] (ported §8) |
| 03 | 750* [SVDPW]; 805* [D]; 890 0x2da870 [DIW]; 909 0x2db558 [DIW]; 1342 0x2df520 [Hrsf] (§4; ported §8) |
| 04 | 280* [DCf]; 805* [D]; 1120 0x2e1a10 [SIW]; 1190 0x2e3078 [SVDPfW]; 1343 0x2e4418 [Hrsf] (ported §8) |
| 05 | 439 0x2f87a8 [Hrsf]; 805* [D]; 919 0x317470 [PW]; 1347 0x31bf40 [Hrsf] (ported §8) |
| 06 | 750* [SVDPW]; 805* [D]; 1016 0x2f4638 [HSCIW]; 1028 0x2f55a0 [HDCW]; 1035 0x2f6470 [Hr]; 1061 0x2fc640 [Sf]; 1105 0x301070 [HSVDCIfW]; 1109 0x302578 [HSVDPCrfW]; 1302 0x307d30 [D]; 1348 0x3083b0 [Hrf] (ported §8) |
| 07 | 436 0x2f5ba0 [SVPfW]; 750* [SVDPW]; 805* [D]; 886 0x30bf90 [D]; 1106 0x314150 [D] |
| 08 | 253 0x2d42d8 [Hrf]; 280* [DCf]; 805* [D]; 1130 0x302ce8 [DPfW]; 1144 0x305270 [DPW]; 1283 0x3065d8 [DIW]; 1349 0x307540 [Hrf] (ported §8) |
| 09 | 805* [D]; 1000 0x300888 [Hr] (ported §8); 1290* 0x308818 [HSDIrW] |
| 10 | 18 0x298668 [SIW]; 750* [SVDPW]; 805* [D]; 1229 0x2e4a88 [Hr]; 1302 0x2e7cd8 [D]; 1326 0x2e8358 [SfW]; 1344 0x2e85b8 [Hrf] (ported §8) |
| 11 | 23 0x2cb668 [HW]; 90 0x2d0710 [HIW]; 114 0x2d0fa8 [HDIfW]; 298 0x2f0e40 [HDrfW]; 805* [D]; 1179 0x30ee00 [Hr]; 1242 0x313290 [HDCrs] |
| 12 | 282 0x2e6c48 [DfW]; 326 0x2e9f68 [Hr]; 328 0x2eb570 [DIW]; 422 0x2ed280 [Hr] (ported §8); 805* [D]; 886 0x2ff838 [D]; 1267 0x303540 [SVDPCW]; 1274 0x3069d0 [S]; 1345 0x3081b0 [D]; 1404 0x3093c8 [Sf] |
| 13 | 69 0x2bb068 [DC]; 304* / 1456..1465* [SVfW]; 558 0x2f3778 [Hr] (ported §8); 805* [D]; 1353 0x30b628 [SVPfW] (§4) |
| 14 | 684 0x2ed280 [Sf] (also hides the ship: `FUN_002a2450`); 805* [D]; 851 0x2fba20 [DIW]; 924 0x2fefe0 [DPW]; 1354 0x305758 [HSDIW] |
| 15 | 805* [D]; 1179 0x2d9100 [Hr]; 1388 0x2ea748 [SIW]; 1419 0x2eb4c0 [SVDPW]; 1446 0x2ec760 [SVDPfW]; 1451 0x2ed068 [S]; 1469 0x2ed398 [Hrf] |
| 16 | 439 0x2c7788 [Hrsf]; 805* [D]; 1377 0x2e3190 [HDIfW]; 1455 0x2e6808 [HIrW] |
| 17 | 750* [SVDPW]; 805* [D]; 1379 0x2ed018 [DC]; 1428 0x2f1790 [SfW]; 1470 0x2f26d0 [Hr] |
| 18 | 586 0x2d7b20 [Hrf]; 644 0x2df608 [HC]; 805* [D]; 1422 0x2f2bf0 [SDC]; 1899 0x2fb868 [S] |

Kinds, by what the code does [M]: **help directors** (one per level, no geometry: 1413, 1341, 1324, 1342, 1343,
1347 / 439, 1348, 1349, 1000, 1344, 422, 558, 1469, 1470, 586 …): cuboid / timer / hero-state tests → `Help_Request`
and record bumps; **story directors** (1353, 1190, 436, 1267, 1419, 1446, 1422 …): a state machine over global flags
and mission bytes → scenes, movies, `UnlockPlanet`, save; **mission NPCs / item givers** (786, 788, 890, 909, 1120,
851, 1283, 1377 …): `GiveItem` + `SetMissionDone` + checkpoint record + save; **cutaway stagers** (1028, 1105, 1242,
644, 69 …): `CameraScript` + `HeroTeleport`. Which systems each still needs is the census' job
(`docs/plan/class_census.md`).

## 4. The three examples, read

**Novalis 1341 `HelpHintDirectorUpdate` 0x30acb8 [M].** Pvars: +0x20 / +0x24 / +0x30 / +0x38 / +0x3c cuboids,
+0x18 timer, +0x28 / +0x2c / +0x34 counters. "Idle" below = help box idle (0x179890 = 0) and no pending request
(0x1798b4 = −1); "group ok" = hero group 0x1413dc < 2 or 9. Each tick: within `ticks(400)` of the load, 0x4e27 (rec
0x51) when move record 0x1418a8 is used, 0x1418a0 is not, the play time is less than `ticks(0x44c)` past 0x1418a8's
time and help record 0x51 was never shown (mask ≥ 0); 0x4e28 (rec 0x72) the same way from move record 0x141930; in
game mode 0 the planet hints 1000 / 1001 / 1002 (recs 4 / 5 / 6) by the planet bits 0x13dd42 / 0x13dd43 and those
records' counts; cuboid +0x3c, group ok, idle, more than `ticks(0x34bc0)` (one hour) of play past move record 9's
time and record 0x43 without this level's bit → 0x3ef; +0x38 → 0x3ec (rec 0x40, while record 0x40 is clear and
0x15f68c ≤ 2), then in hero state 1 → 0x3ed (rec 0x41); +0x30 with 0x1413e0 = 0x3b near (< 3) a class-280 crank whose
first pvar ≠ 1.0 → counter +0x34, 0x3ee on the 3rd and 6th time; hero group 0x12 in +0x20 / +0x24 for more than
`ticks(240)` / `ticks(360)` → 0x3f0 (rec 0x44) and a bump of move record 0x1418f8 / 0x141900; group 0x11 sets help
record 0x44's count to 0xffff (never again); within 10 of a class-806 nanotech cluster, records 1 / 2 / 0x1418b0
clear: below full health → 1 (and a 0x1418b0 bump), at full health and `FUN_00275690(255, cluster)` → 2. Help ids are
level message ids; the second argument is the help-record index (0x141968 + 8·rec).

**Kerwan 1342 (L03 0x2df520) [M].** State 0: counters +0x50 / +0x54 = 0, update distance 0xff → 1. State 1, per
tick (the help calls only while idle): cuboid +0x78 (group ok) sets global flag 0x13d3a2; while it is clear, cuboids
+0x00 / +0x0c / +0x18 → 3000 / 0xbb9 / 0xbbb (recs 0x14 / 0x15 / 0x17, while never shown); +0x30 entered → move record
0x141860 set once; +0x24 entered outside group ok → counter +0x50 (60-tick timer +0x58), every 6th → 0xbba; +0x28 more
than one hour of play past move record 9 → 0xbbc; +0x3c with no creature (class type 5) within 8, item 0xf owned
with `FUN_00223668(0xf)` ≥ 0x15 and records clear → 20000; hero state 0x1e → move record 0x1418e0 bumped every tick;
+0x38 → 0x3ec / 0x3ed (as 1341); +0x40 → 0xbc0; game mode 0 with item flag 0x13d4f4 → 0xbbd; +0x34 → move record
0x1418c0 set once; the Swingshot miss flag 0x13fcd8 counted (+0x54) → 0xbbe after 3 while 0x1418c0 is clear;
**hero state 0x74 (the cable) → move record 0x1418c8 set once** (the "cable used" stat); +0x2c in hero group 6 with
the wrench (0x140408 = 8) while 0x1418c8 is clear → **0xbbf** (rec 0x1b: the cable help); +0x44 → 0x4e2f; +0x7c with
two or more owned items of a kind (0x179bc8 table word 0, not 8 / 0x18) and 0x1418e8 clear → 0x4e25; +0x70 → global
flag 0x13d3a0; +0x74 → **skill point 0x13d40b** (jingle `allocate_voice_for_bank_entry(1, 0, 0)`, banner 0x53d6).

**Gemlik 1353 (L13 0x30b628) [M].** State 0 → 1 in game mode 0; 1: global flag 0x13d3ec clear → set, **scene 0**
(the arrival; `RC_SCENE=+0`) → 2 (waits while mode 2) → 3 → 4 (flag already set: → 3 at once); 4: flag 0x13d3ed clear
and Ratchet in cuboid P[0] → set, moby P[5] hidden (mode |= 0x41), scene 1 → 5 (flag set: → 6; P[0] = −1: → 7; P[3]
deleted once 4 is left); 5, after the scene: P[5] shown, moby P[4] moved to cuboid P[2]'s centre and put in its state 2
(its pvar +0x7c and the fade 0x15f3fc = 0.99), Ratchet `HeroTeleport(P[2], state 0)` → 6 → 7; 7: flag 0x13d3ee clear,
waits for moby P[5] (class 388) to die → P[4] hidden, `HeroTeleport(P[2], 0x72)`, flag set, moby P[8] hidden,
`FUN_00271070(0, 5)`, scene 2 → 8: movie 0xe → 9: scene 3 → 10: P[4] shown, `UnlockPlanet(0xe)`, P[8] shown and
`FUN_00298cb0(P[8])`, the landing point 0x13e090 = (464.66, 580.68, 316.72), yaw 0x13e0a8 = 2.77, letterbox off →
0xb: once P[4] (class 69) is dead, save → 0xc → 0xd: P[4]'s mission done in the load copy 0x15fc88 → P[8] deleted
→ 0xe hidden. A story state machine of Gemlik's own, as 730 / 790 is Novalis'.

**The help / move records** (chunks 16 / 17 / 18, `GameState::global.{help, move_help, gadget_help}`, typed
`HelpRec {count, time, mask}`) are the "stats": one bump rule everywhere (`count++` unless 0xffff, `time = max(time,
ScaleTicks(play time) / 600)`, `mask |= 1 << level | 0x80000000`), inlined by the hero (melee, jump, quick select),
the directors and `Help_Update` 0x225bd0 (the shown message's record, when its box closes). `Help_Request(msg, rec)`
0x225818 refuses unless the box is idle, no voice line / stream plays (0x151720 = 0, 0x1516ec = −1) and `help[rec].count
≠ 0xffff`; it stores (msg, rec) and appends msg to the help log (chunk 1010 / 1011, `FUN_00226a70`). The directors
gate on `count == 0` (never closed) or `mask ≥ 0` (never shown on any level) themselves.

## 5. Which system owns what (the map for the ports)

| level logic piece | system (owner) | status |
|---|---|---|
| the director / story / NPC classes themselves | ordinary class ports (`classes::LevelPorts`, the census) | Novalis' 730 / 790 ported; the rest G-CLS-001 |
| scenes, movies, their start / end | scene player, movie player (`scene_player`, `scene_render`, `movie_render`) | ported; started via `cinematic::start_scene` / `start_movie` |
| cutaways | script camera + hold 0x72 + letterbox (`cinematic`, `follow_camera::script`, `hero::scripted`); generic trigger 737 | ported |
| trigger volumes | each class's own `PointInCuboid` tests (`triggers.rs`); activation zones 258 | ported |
| mission bytes | game state chunk 3004 + `LevelMissions`; writer `cinematic::set_mission_done` | ported (this pass: one writer) |
| checkpoint record | `classes::checkpoint::record` | ported |
| help box | HUD (`hud::Help`) | ported; class requests, gate, record bump, voices, log: G-UI-017 |
| help / move records ("stats") | game state chunks 16 / 17 / 18; bump `hero::melee::bump_record` | typed; class writers G-SAV-009 |
| global flags, skill points, planets | game state chunks 5, 8, 14 (`TalkGame` mirrors, `UnlockPlanet`) | read-only mirrors; a general class writer G-SAV-010; skill points G-SAV-007 |
| NPC talk, prompts, item gifts | interaction system (`interact`, talk tables) | ported |
| moby group commands | G-CLS-023 | open |
| level camera volumes | G-HERO-027 | open |
| save | G-SAV-002 | logged only |

## 6. Triage resolutions

* **G-TRI-002 `SetMissionDone` vs `GameWrite`** [H]. The game has one writer, `SetMissionDone` 0x265080, a single byte
  store into the live save bytes that every class reads; nothing else. The port had two paths: the cinematic request
  (`EngineRequest::MissionDone`: mission NPC, infobot, crank), which the engine applies to both the live bytes
  (`LevelMissions::done`) and the saved game, and the talker's `GameWrite::MissionDone`, which wrote the saved game
  only (the classes kept reading the old live byte until the next load), and the checkpoint had none (counted as
  unported). Resolved: **`cinematic::set_mission_done`** is the one call; the checkpoint 805 (all 19 levels) and the
  talker 774 use it, `GameWrite::MissionDone` is gone; the talker's checkpoint cuboid +0x4c (`FUN_0029ac10(centre,
  Euler)`, 0x2ff118 state 2) is wired too. Deviation kept [L]: the engine applies the byte after the tick, so a class
  later in the same moby loop reads it one tick late (no such reader on the ported levels).
* **G-TRI-003 hero state 0x72** [H]: ported (`rc-game/src/hero/scripted.rs`: entry, idle physics, no transitions;
  used by the camera trigger, the gunship, the gold bolt, the crank). hero_states.md §1.2's "–" is stale.

## 7. In the port (2026-09-28)

* `rc-game/src/cinematic.rs`: `set_mission_done` (the one `SetMissionDone`).
* `classes/checkpoint.rs`: `SetMissionDone(+0xb0)` on taking (was counted as unported); unit test
  `taking_sets_its_mission_once` (armed, taken once, record set, no second request while inside).
* `classes/talking_npc.rs`: `set_mission_done` + the checkpoint record of cuboid +0x4c; unit test
  `scene_end_sets_mission_and_checkpoint` (open mission → one request + record; done → nothing).
* `classes/{mission_npc, infobot, bolt_crank}.rs`: the same call instead of their own request push.
* `moby_update/interact.rs`: `GameWrite::MissionDone` removed (no writer left).
* Nothing else was built: no scripting layer, by the finding above. The census probe was a temporary test (deleted;
  its output was kept in the agent scratchpad `scripting/census.txt`).
* Checked: the class unit tests above; `cutscene_novalis`, `interaction_vendor`, `gold_bolt_infobot_novalis`,
  `bolt_crank_novalis` pass; `novalis_hero_digest` with `RC_HERO_DIGEST_NO_IDLE=1` byte-identical before / after.

## 8. W2 lane 2 (2026-09-29): the quick wins and the help directors

### 8.1 Quick wins: system or not

| gap | finding (evidence) | ported | test |
|---|---|---|---|
| G-AUD-010 voice handoff | **Not a shared routine**: 838 (`0x30dc68`) and 855 (`0x3150f0`) each inline their own hand-off (838 tests the owner's state 1, loops sound 0 with flags 4 and follows; 855 tests state 2, plays sound 0 with flags 0xd (no follow) and places the voice 2.5 along its row 0). What they share is one **store**: the slot record's owner +0x18 and position +0x20 (`0x13e5c0 + slot·0x70`). | the store once: `SoundSink::hand_over` (the audio port's `ClassSoundSink` rewrites the slot), `World::hand_over_sound` / `sound_owner`; consumer 838 `units/laser_fence.rs` (U183, 26 on 05) with its bars callback `0x30de90` (`Callback::UnitQuads`, `units::fx_quads`, rc-engine fx_draw: FX 0x13, additive, ten bars per lit fence of the group). 855 (U189, 7) is census-cheap now: a G-CLS-027 port | `laser_fence::tests::the_nearest_fence_takes_the_loop_and_the_switch_turns_it_off` (every side effect: loop start, hand-off owner + position, power-down sound 1, collision off, state 2, one callback per tick, the quads); `quick_wins::laser_fences_rilgar_hand_one_voice_along` (Rilgar data: 26 fences, one group, one shared slot word, no second play) |
| G-AUD-007 `0x27eca0` | **Not a system**: level04's own copy of `PlayLevelSoundAtMoby` 0x2a1770 (the same masked code; the census missed it because the overlay has the function twice). Its only caller, the 466 family's `0x2ca420`, uses it for the skill point jingle (G-SAV-007). | nothing new: `World::play_level_sound`; re-tagged `has` | `quick_wins::eudora_level_sound_is_play_level_sound_at_moby` |
| G-REN-020 `DrawSpriteHelper_A` 0x21e340 | **Not a system**: the VU1 billboard program's set-up (view·projection 0x167140, guard band 0x1671c0, fog words, GIF tags, `0x2b4c28`); no game state. rc-engine's fx material does the same per prim. The callers' own draws (e.g. U532's `0x2f5690`: strips through `0x1faf00`) are their class ports. | re-tagged `has` (cited in `rc-engine/src/fx_draw.rs`); its 4 units / 16 instances are census-cheap | census re-run |
| G-LVL-008 height grid | **A small engine read**: `LoadLevelCoreData` hands core +0xa4 to `0x2530f0` (0x15fc98 width, 0x15fc9c rows, 0x15fca0 / a4 low / high, 0x15fca8 cells); `0x278020` reads `(high − low)(1 − cell/255) + low` at `width·(int)y + (int)x`. Consumers: 1400 (twice: its update and `SpawnImpactSparks` 0x2780b0). | `rc_formats::level::HeightGrid` (512 × 512 on 08 / 12 / 14); re-tagged. 1400 stays blocked by G-PRT-001 (type 01) and `0x272cb8` | `quick_wins::height_grid_on_its_three_levels` |

### 8.2 The help directors ported

Ordinary class ports (`units::PORTS`, rows "U… help"), each with its full coverage table in its module doc. The shared
inline patterns (the cuboid test, "group ok", "idle", the record bump, the arm, the 18-second reminder, the crank
tries) are written once in `units/hints.rs`: the game repeats them byte for byte in each director, so this is a
coding convenience, not a game system.

| class (level) | module | ported side effects | NOT ported (gap) |
|---|---|---|---|
| 1413 (00) | `help_veldin` | 6 hints (3, 0x4e2a, 5, 4, 0, 1 / 2 by health and the watched crates) | — |
| 1324 (02) | `help_aridia` | 10 hints; flag 0x14; record writes `H[0x10]`, `H[0x12]`, `H[0x76]` := 0xffff; pos copy; dune-jump state | the skill point 0x13d409 (G-SAV-007) |
| 1342 (03) | `help_kerwan` | 14 hints incl. the cable help 0xbbf; flags 0x1a, 0x18; stats `M[3]`, `M[15]`, `M[16]` (cable used), `M[19]`; the Swingshot miss flag taken (`HeroFields::swing_help_clear`) | the skill point 0x13d40b (G-SAV-007) |
| 1343 (04) | `help_eudora` | 9 hints incl. the crank tries; flags 0x1c..0x1f; stats `M[17]`, `M[24]`, `M[25]` | — |
| 1347 (05) | `help_rilgar` | 9 hints; stats `H[0x25]`, `H[0x5e]`; `H[0x24]` / `H[0x44]` retired; `M[29]` reset; the swim / dive counters; pos copies | — (the pad activation `0x30c928` is `floor_switch::press`) |
| 1348 (06) | `help_blarg` | 5 hints (the crossed records 0x2e / 0x2f kept); `H[0x28]` retired; flag 0x25 | — |
| 1349 (08) | `help_batalia` | 4 hints; flag 0x2f; `H[0x70]` retired on magnetic walls; grind state | the skill point 0x13d413 (G-SAV-007) |
| 1000 (09) | `help_gaspar` | the hint 9001 and its record (count on item 11, time / mask every tick) | the skill point 0x13d416 and its platform counter 0x1613c8 (G-SAV-007) |
| 1344 (10) | `help_orxon` | 7 hints; the level word 0x161f38 (read by `0x2dfc38`, unported) | — |
| 422 (12) | `help_hoven` | 3 hints; stat `H[0x6e]` from the hover counters | the skill point 0x13d41c (G-SAV-007) |
| 558 (13) | `help_gemlik` | the hint 13000; stat `H[0x6f]` | the airless flag 0x14161b (G-HERO-032) |

Every request goes through `Help::request` (refused while a box is up, a voice plays or the record is 0xffff), is
logged, and shows through the ported box (voice `30000 + help_audio`, record bump on close). Tests:
`help_directors::*` (11, one per director on its level's data: trigger → message and record, logged; the gates that
stop repeats; the flags and stats written; the message in the level text, its voice stream and the bump on close).
Frames: three levels (Veldin's welcome from 1413, Gaspar's 9001 from 1000, Orxon's 10000 from 1344), identical
across two runs.

### 8.3 Not ported this pass (filed)

* The save-only story / NPC classes (1120, 1190, 1290, 18, 1326, 23, 1354, 1388, 1419, 1750): besides
  `memcard_Save` (logged, G-SAV-002) they write items, bolts and health directly and drive the dialogue helpers:
  G-CLS-029.
* 439 (05, 16) and 1469 (15): hero-state branches (G-HERO-002 / G-HERO-005); 1179 (11, 15) and 586 (18): pad-like
  classes with the help 0x2b01 / 0x2b02 reminders, per-instance save bytes (G-SAV-003) and draw callbacks (G-CLS-027).
* 855 (the spouts): census-cheap now, G-CLS-027.
