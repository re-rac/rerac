# Progression: the game flow from power-on to the credits

One section per lane of batch 6 (2026-10-01): **`## saves`** (boot → title → main menu, the memory card, saves and
loads, challenge mode), **`## travel`** (the ship, planet travel, the runtime level change), **`## story`** (every
level's story drivers: scenes, help, items, unlocks, missions), **`## media`** (front-end movies, the slideshow,
credits, Goodies, cheats). The save block itself is docs/plan/game_state.md.

## saves

Boot → title → main menu → game, the memory card, the save points and the slot format. Code: `rc_game::memcard`
(card driver, monitor, saves), `rc_game::menus::freeze` (mode 4), `rc_game::menus::pause::saves` (the card pages),
`rc_game::frontend` (the boot flow), `rc_formats::frontend` (the title world's pictures, the boot pictures),
`rc-engine` `saves` + `menu_render` (the glue). The save block itself is game_state.md.

### Boot → title → main menu (boot `startlevel` 0x1e9658, `transition_do_transition` 0x1eb798)

1. `startlevel`: front-end flag 0x15f5e8 = 1, `InitOnce`; the card check `fun_00209168` (0 ok, 1 no / not a PS2 card, 2
   no save folder and < 350 KB): while ≠ 0 a full-screen warning (IRX lump `global/irx.bin` +0x10 / +0x40 + 8·language,
   raw 512×416) until fixed or, after 10 vsyncs, any button; `FadeToBlack(10)` if shown.
2. The logos movie `mpegs[0]` (PAL [1]), `FadeToBlack(ticks(18))`, the loading still (IRX +0x00) while the sound bank and
   the title world (`global/unknown_14e8.bin`) load, at least ticks(180) in all.
3. `transition_do_transition`: the new-game reset (template, level 0), the title world, then per vsync: `memcard_Update`,
   the card monitor, `UpdatePad`, the update `fun_001eb0a8`, the draw.
   * **Title (mode 0)**: fade from black (−0.0625 / vsync); after ticks(60) the logo (256×128 at (0xec, 0x10)) fades in
     to alpha 0x40; after ticks(120) PRESS START (FX 4 + language − 1, 192×96 at (0xa0, H − 0x50)) pulses `cos(…)·32 +
     0x60` with a 60-vsync period; Start or ✕ opens the main menu. 1500 idle vsyncs → attract movie `mpegs[80 + i]` (i
     cycles from `rand() % 4`), then the title starts over.
   * **Main menu (mode 3)**: the page menu kind 0x2d, page 0x1b8b48 (L01; boot 0x1d45c8): **New Game** (action 4 →
     0x1b9208), **Load Game** (5 → 0x1b9180), **Options** (3 → 0x1b8d08: Language 0x1b8ec0 — English, French,
     German, Spanish, Italian; Sound 0x1b8e38; PAL adds Video). △ on the root closes the menu: back to the title. The
     logo and PRESS START fade out (−0x10 per vsync).
   * **Card dialog (mode 4)**: see below.
4. A page asks for a level (`initialize_global_state_entry`, L01 `FUN_002a29a0`): the loop ends, `DoSpaceTransition`
   loads it (0x13e05a = 1 for a new game: the story trip; 0 for a load). Port: `EngineRequest::LeaveLevel` + 
   `ShipGlobals::reset_trip` (the travel lane's level change).

Port: the engine's default start runs this flow over the loaded level (the world hidden behind the menu layer cleared to opaque
black, the tick suspended). Not ported: the title world (the space flight behind the title, its mobys and particles,
the title music) — G-SAV-012; the runtime language switch's text reload — G-UI-020.

### The memory card

The card is a folder (`<config base>/rerac/memcard/BASCUS-97199RATCHET/`; `RC_SAVE_DIR`, the settings key `memcard_dir` or a
`memcard/` folder beside the executable override it, docs/workflows/dev-switches.md) holding the files the
game writes: `icon.sys`, `static.ico`, `save0..4.bin`, `BASCUS-97199RATCHET` (a 0x3c04-byte reservation, zero-filled
here). `memcard_Update` runs one libmc step per frame on it (the native calls complete at once). The **card monitor**
(status 0x15eeb0, flags 0x15eeb4) drives what the menus may do: status 1 (checked, auto-save on) and 16 (data found)
let the Save / Load / New Game pages open; otherwise the action asks (flag 2 save / 4 load) and the **card dialog**
(`mode_freezeInit(3, page)`) walks the statuses: checking (10/11) → data found (16: the page opens) or no data (12/13:
"No Ratchet and Clank save data … create a save file?" ○ Yes / △ No → creating (14/15) → 16), unformatted (5–8),
not enough space (19), failures (17, 18, 20, 21). In the front end the "continue without saving" variants (23, 24, 4,
19) start a new game without a card on ✕.

### Saves (every `memcard_Save` / whole save)

| caller | call | what |
|---|---|---|
| New Game page 0x296fc0 | `fun_00226a70(slot)` | the new-game reset into the slot (whole save), level 0 with the story trip; ○ = continue without saving |
| Save page 0x296990 (Options → Save) | `fun_002269c0(slot)` | capture + whole save into the slot (overwrite asks: page 0x1b6af8) |
| Load pages 0x296ce0 | state 0xd | the slot restored (`RestoreData` per section), auto-save on, the saved level without the story trip |
| challenge page (end page ○, card ready) 0x296fc0 flag 0x2000 | `fun_00226b08(slot)` | the challenge reset (gold weapons, the 0x1ba120 items' owned flags, ammo, gold bolts, quick select, vendor, skill points, bolts kept; nanotech max HP; HelpDesk off; completes + 1) saved whole, level 0 |
| end page ✕ "Timewarp" | `memcard_RestoreGame(0x1ba250)` + `memcard_Save(0, −1)` | back to the class-1750 save before the last boss (level 18's gold bolts and skill point 0x1d kept), game beaten, reload |
| class 1750 (level 18) | `MakeWholeSave(0x1ba250)` once | the ending buffer |
| every class `memcard_Save(0, −1)` (gold bolt 1134, infobot 750, item offers, mission NPCs, talking NPCs, Pokitaru's commando, Quartu's Giant Clank mission, 0x30a6d0, …) | incremental | clock, landmarks, the current level's map mask; global + the current level's section; with auto-save on only |
| save notice (mode 4 kind 5, L00 0x297f78) | — / `memcard_Save(0, −1)` on level 1 | shown once (global flag 0x10) on the first level ≠ 0 after auto-save turned on |
| `ShipTravelTo` | `memcard_Save(0, dest)` | the travel lane's (`saves::memcard_save(play, gs, false, dest)`) |

### Slot format

`save%d.bin` = game_state.md §3.2 (8-byte header, global section 0x1530, 20 level sections 0xaa4; CRC-16 0x1f45). The
slot list shows per slot (from the global section's chunks 0..4): the play time `hh:mm` (chunk 3 / 216000 ticks per
hour), times completed (when ≠ 0), the bolts, the save date `mm/dd/yy` (chunk 4, BCD; the port stamps UTC), and on the
side the saved level's location / planet name and planet picture (`planets[level]`). An empty slot (level −1) shows
"EMPTY".

## travel

The ship, game mode 6, `DoSpaceTransition` and the runtime level change. Code: `rc_game::travel` (`ship`, `space`,
`transition`, `cards`), `rc-engine` `level_switch.rs` (the one level change), `travel_render.rs` (mode 6 and the
transition in the engine); the ship's draws in `fx_draw.rs`. Addresses are level01; the code and its tables are
identical on all 19 overlays (masked overlay-diff 2026-10-01), so there is one port.

**The one level-change path.** The game leaves a level one way: `0x15f5c0 = dest; 0x15f570 = 1`, the level's main
loop (`entry` 0x259c40) ends, the boot runs `DoSpaceTransition` 0x2a68f8, which plays its cards / movies / flight
and loads `dest`, then the new overlay's `entry` (level init, `ShipLandingStart`). In the port every caller ends in
`travel_render::request_leave(dest)`: `rc_game::cinematic::EngineRequest::LeaveLevel` (a class's `0x2a29a0(p)`, the
menus' level exits through `media_render::request_level_exit`: Quit Game −1, New Game, Load Game, the end page's
challenge restart 0), and the fly-away's end. The load itself is `level_switch` (a worker thread; the swap despawns
the old level, drops its per-level state through `LevelUnload`, inserts the new level's resources and runs the level
start's schedules again: `LevelStartup` / `LevelPostStartup` / the lazy set-ups keyed on `LevelGeneration`). The boot
loads its level through the same `load_bundle` and schedules. Kept across a change: the saved game (`Persistent`),
the session, the ship index 0x13e056, the options, the menu mode, the movie player, the audio output.

| way the game changes level | what the port does |
|---|---|
| the ship: △ → take-off → planet page → fly-away (`ShipTravelTo`, sub 3) → leave | mode 6 → `request_leave(dest)` |
| a class's `0x2a29a0(p)` (Veldin's Clank 834, class 436: the story trips; ported by the story lane) | `EngineRequest::LeaveLevel` |
| Quit Game (Options, ○): −1 | `LeaveLevel(-1)` → the front end over the reloaded level (`saves::set_front_end_active`) |
| New Game / Load Game / the card dialogs' new game (`UpdateModeFreeze`, `LoadingDataMenu`, `SavingDataMenu2`) | `LeaveLevel(0 / saved level)` with 0x13e05a (`ShipGlobals::reset_trip`; no reader but `ShipLandingStart`'s clear) |
| the end page's challenge restart (`process_global_state_flags` 0x295c98: `FUN_002a29a0(0)`) | `LeaveLevel(0)` |
| the ship to the planet it is on (`ShipTravelTo(level)`) | **not** a level change: the landing scene from tick 240 |
| a death, a fall, `HeroCall::Reload` (0x141401) | **not** a level change: `LoadLevelCoreData(0, 1)` inside the same `entry` loop (the port respawns; the mobys' re-creation is G-CLS-030) |
| the boot (`startlevel` 0x1e9658: 0x13e05a = 1, level −1, `do_space_transition`) | the engine's start: the front end by default, or straight into `RC_LEVEL` (development) |

**Mode 6** (`rc_game::travel::space::ShipMode`, one call per 60 Hz frame after the world part of `GameStateUpdate`,
which is the gameplay tick: scene form for subs 0 / 8, with the follow camera for sub 3; the blocking `FadeToBlack`s
are frames of their own):

```
mode 0 ──△ at the hatch (ShipUpdate: 0x15f630)──► ShipTakeOff ─► EnterShipMode(0): hold ticks(6), stream ship+6
   sub 0  take-off scene (lump ship+1, 598 ticks; skip ✕/△ → tick 558, FadeToBlack(4)) ─► end: EnterMenuMode(0), kind 0xe
   mode 3 planet page ─► close (post 2) ─► ShipTravelTo(dest)
        dest = level ─► EnterShipMode(8) at tick 240 ─► sub 8 landing ─► HeroTeleport(landing spot) ─► mode 0
        else memcard_Save(0, dest) ─► sub 3 fly-away: hold ticks(12), stream 40015+ship, riders, CameraScript;
             120 ticks ease to the path, accelerate (0.8/tick ≤ 100), trail, fade ─► 0x15f570 = 1 ─► DoSpaceTransition
DoSpaceTransition ─► steps (below) ─► flight (sub 4) ─► load done ─► new level `entry`
entry ─► ShipLandingStart: level 0 / first Novalis / level 14 without 0x13d3f0 ─► mode 0
                           else EnterShipMode(8) (no fade, stream ship) ─► sub 8 landing ─► the walk-back ─► mode 0
```

**Transition steps** (`rc_game::travel::transition::plan`; cards `space_plates[lang]`, movies `mpegs[40 + n]` NTSC):

| branch | steps |
|---|---|
| dest < 0 | fade 6, level = −1, the front end |
| dest 0, visited[0] = 0 (new game) | fade 6; cards 0+1 (240), movie 0; card 2 (180), movie 1; level = 0; cards 3+4 (240, loading), movie 2; enter |
| 0 → 1, visited[1] = 0 | fade 6; cards 5+6 (240), movies 3, 4; card 7 (180), movie 5; visited[0] = 2, level = 1; card 8 (240, loading); enter |
| otherwise, in order | dest 4 first: fade 12, cards 9+10, movie 6; leaving 7 (visited[7] ≠ 2, planet 8 unlocked): fade 12, card 11, movie 7; dest 13 first: fade 12, cards 12+13, movie 8; leaving 14 (visited[14] ≠ 2, planet 15): fade 12, card 14, movie 9; dest 16 first: fade 12, cards 15+16, movie 10; then visited[from] = 2 (not 7 without planet 8, not 14 without 15), level = dest, the flight (until the load is done, then one more random variant and the approach), enter |

**The flight** (`SpaceLoadingLoop`): variants 0–3 of the `transition` lump (random: `rand() >> 16 & 3`, then `+ rand() %
3 + 1`), 4 = the planet approach (the only one for the first-arrival flights, dest 0 or dest 1 with planet 3 locked,
which never fly in practice: their branches use the loading cards). While the load runs the count resets; after it,
one more variant and the approach. The actor (class 533 → the ship's class), its camera and the flight's scenery
(`DrawWorldPaused`: the lump's sky with its turned, scaled and translated shells, the planet picture and the caption
of the approach, the trail of the actor's engine joints, the canopy glass, the card icon, the flight's sound bank and
sound v; `rc_formats::transition`, `rc-engine` `flight_render`) are ported.

**The space-scene player** (`rc_game::travel::space::ShipMode`) also plays the vendor's weapon demos (`SUB_DEMO`:
`unknown_1530[scene]` in the vendor's frame, `VendorExit(1)` at the end) and, as `rc_game::travel::title`, the title
world's loop behind the front end (`rc-engine` `title_world`: the title lump drawn through the level renderers on
layers of its own).

**Planets and ships.** The planet page lists the unlocked planets (0x13dd40, in the map order 0x13d510: chunks 14 /
20, written by `UnlockPlanet` 0x2756d0 and the level start's own unlock); every listed planet is selectable; the
current planet lands again. The ship index (0x13e056) for the destination: 0; 1 when planet 8 is unlocked or dest > 7;
2 when planet 14 is unlocked or dest > 13 (classes 531 / 532 / 533 and their extra classes 535 / 536 / 537 from the
`spaceships` file).

**Saves.** `ShipTravelTo` saves before the fly-away: `memcard_Save(0, dest)` (`saves::memcard_save(play, gs, false,
dest)`: the global section with level = dest and visited[dest] raised to 1). There is no save on arrival.

Not ported (gaps.md): G-CUT-009 (the hero's worn items on the scene's Ratchet actor, `FUN_0024a0e8`), G-LVL-002
(level 13's adopted ship 0x2a2360), G-CLS-030 (the death reload's moby re-creation), G-UI-006 (the item offer's demo
entry `FUN_002aea70`).

## story

Every level's story drivers (batch 6, lane `story`): what triggers them, what they do (scene, movie, help, item,
planet, mission, flag, save) and in what order the story runs. There is no script system (level_scripting.md §1):
each driver is ordinary class code on the engine helpers. Built from the class census (`work/census`, the units
calling the story helpers), the decomp of every class (`work/decomp/levelNN.elf`) and the ported classes' module docs.
Status: **P** ported (the module), **—** not ported (gap). Code: `rc_game::moby_update::story` (the shared services),
`classes/units/*_story.rs` (this batch's drivers), `classes/units/story_npc.rs` (the NPC template's inline patterns).

### The shared story services (`rc_game::moby_update::story`)

| game | port | notes |
|---|---|---|
| global flags 0x13d388[128] (chunk 5) | `story::flag` / `set_flag` (→ `interact::set_global_flag`: the mirror + `GameWrite::Flag`) | G-SAV-010: one read / write view; every class of this batch uses it; `Cinematic::arrival_seen` (flag 0x0f) stays the travel lane's arrival mirror |
| skill points 0x13d408[32] (chunk 8) | `story::skill_point` / `award_skill_point` (the byte, `PlayLevelSoundAtMoby(1, 0, 0)`, `ShowBanner(0x53d6, −1)`; `GameWrite::SkillPoint`, mirror `TalkGame::skill_points`) | G-SAV-007: every census site wired (table below) |
| items: `GiveItem` 0x275760 | `interact::give_item` (banner, acquired mirror, `GameWrite::GiveItem`) | unchanged; the direct stores (Pokitaru's 23 / 298) are `story::set_owned` / `set_acquired` (`GameWrite::Owned` / `Acquired`) |
| max health 0x15eda0 (chunk 19), health 0x1415f8, bolts 0x15ed98 | `story::set_max_hp` (`GameWrite::MaxHp`), `set_health` (`HeroFields::health`), `add_bolts` (`GameCounters::bolts`) | G-SAV-005: the Nanotech seller 1326 |
| `UnlockPlanet` / `ShowPlanetBanner` / `SetMissionDone` / scenes / movies / saves | `cinematic::{unlock_planet, show_planet_banner, set_mission_done, start_scene, start_movie, item_movie, save, save_as}` | `start_scene` now stores game mode 2 at once (DialogStreamStart's 0x15f5c4 = 2); `item_movie` = the level item-movie player (`mpegs[64 + n]`, levels 02 / 08 / 16); `save_as(p)` = `memcard_Save(0, p)` |
| the level exit `0x2a29a0(p)` | `story::level_exit` → `cinematic::leave_level` (the travel lane's one path) | Veldin's Clank 834, Umbris' 436 |
| the made-permanent stores (killed / collected bytes, death bits) | `story::kill_record`, `story::death_bits` | the same tests as `SetDeathBits` |
| the help / move / gadget records (chunks 16 / 17 / 18) | `Help::records` (+ `units::hints::{bump, remind, …}`), written back by `Help::sync_out` (now with the gadget records) | G-SAV-009: the melee stats' move / gadget bumps are now copied into the help system's records before its write-back (they were lost) |
| hero-block stores | `HeroFields::{head_request (0x141410), clank_hidden (0x141628), set_entry_pose (0x141050 / 0x141060), hero_hidden (0x1413f5)}` | Gaspar's helmet, Veldin's Clank, the race girl, Gemlik / Quartu |

**Skill points wired** (G-SAV-007; index = 0x13d408 + k): 0 the Blarg flyers / the gunship (Novalis), 1 Aridia's dune
jump (1324), 2 the path ships' third kill on Aridia (1212), 3 Kerwan's cuboid +0x74 (1342), 4 Kerwan's air traffic 795,
8 the Morph-o-Ray on Rilgar's class 625, 0xb Batalia's director cuboid (1349), 0xc the fighters 438 from the turret,
0xe Gaspar's tethered-platform word (1000 + 1182–1189), 0xf the path ships from the turret on Gaspar, 0x10 / 0x11
Orxon's scouts / brawlers (1196 / 1202), 0x14 Hoven's group (422), 0x16 Gemlik's breakables (1805). Not wired (their
classes are unported): Eudora's 466 family (6, 0x13d40e), Batalia's 253 (0x13d415), Kalebo's race 1455 (0x13d422), the
cheat entry (media lane).

### The planet chain (who unlocks what)

| planet | unlocked by | status |
|---|---|---|
| 1 Novalis | the trip from Veldin: Veldin's Clank 834 (`0x2a29a0(1)`) → level start | P (`veldin_story`) |
| 2 Aridia | Novalis' plumber 774 (the Infobot sale, talk node 4) | P (`talking_npc`) |
| 3 Kerwan | Novalis' mission NPC 730 / 790 | P (`mission_npc`) |
| 4 Eudora | Kerwan's infobot 750 (pvar +4 = 4) | P (`infobot`) |
| 5 Rilgar | Blarg's infobots 750 (two, +4 = 5); Blarg's shuttle 1109 also | P (infobot); 1109 — |
| 6 Blarg | Eudora's informant 1190 (talk node 2) | P (`eudora_story`) |
| 7 Umbris | Rilgar's bouncer 919 (talk node 5) | P (`rilgar_story`) |
| 8 Batalia | Umbris' director 436 (the exit; and the trip there) and its infobot | P (`umbris_story`, `infobot`) |
| 9 Gaspar | Batalia's deserter 1144 (talk node 4) | P (`batalia_story`) |
| 10 Orxon | Batalia's commando 1130 (talk node 3) | P (`batalia_story`) |
| 11 Pokitaru, 12 Hoven | Orxon's infobots 750 | P (`infobot`) |
| 13 Gemlik | Hoven's turret 1267 (its target, the carrier 1274, destroyed) | P (`hoven_turret`, `hoven_carrier`; 2026-10-01 minigames lane) |
| 14 Oltanis | Gemlik's director 1353 (Qwark's escape), after Qwark's ship 388 falls to Gemlik's ship 69 | P (`gemlik_story`; the fight: `gemlik_ship`, `qwark_ship`, 2026-10-01) |
| 15 Quartu | Oltanis' scrap merchant 924 (talk node 4) | P (`oltanis_story`) |
| 16 Kalebo III | Quartu's Giant Clank mission 1446 | P (`quartu_giant_mission`) |
| 17 Drek's fleet | Quartu's broadcast director 1419 | P (`quartu_story`) |
| 18 Veldin | the fleet's infobot 750 (+4 = 18) | P (`infobot`) |

### Per level (story order)

Abbreviations: S scene, V movie (`DialogStreamUpdate`), IV item movie, H help, I item, P planet, D mission done, F flag,
W save, C checkpoint, X level exit.

| level | driver (class, update) | trigger → effect | status |
|---|---|---|---|
| 00 Veldin | Clank 834 `0x2d9dc8` | first update: Clank hidden on Ratchet's back, F 8; talk (radius 255 → 3 / 16 in cuboid +0x60); a node past 3 played → X 1 | P `veldin_story` |
| | help director 1413 | hints | P `help_veldin` |
| 01 Novalis | arrival / mission NPC 730 / 790, plumber 774, infobot 750 (stand-in), crank 280, checkpoints 805, director 1341 | S 5 (arrival, F 0x0f), the mission (S 3 / 4, P 3, W), the plumber's Infobot (P 2), hints | P (earlier batches) |
| 02 Aridia | surfer 786 `0x2e0dc0` | talk: D + C; node 2 → IV 2 → I 5 (Sonic Summoner), W; H 2004 near him | P `aridia_story` |
| | agent 788 `0x2e1950` | the shark race (counters +0x14c / +0x150 from the sharks / nests) → his talk node 2 → H 2000, I 30 (Hoverboard), D, C, W | P `aridia_story` |
| | item scene 1005 `0x2ea210` | touch → S 4 → I 26 (Trespasser), banner 2015, H 2009, camera glide, W | P `aridia_story` |
| | 713 `0x2ddc88` | cuboid hints (H 2013 / rec 0x48) and a hero teleport (cutaway) | — G-CLS-033 |
| | help director 1324 | hints, skill point 1 | P `help_aridia` |
| 03 Kerwan | Helga 890 `0x2da870` | talk: D, C, the course moby 1012 opened; node 2 → I 12 (Swingshot), banner 3016, W | P `kerwan_story` (the visit record: G-SAV-003) |
| | 909 `0x2db558` | talk: D, C; node 2 → I 2 (Heli-Pack), banner 3015, the gate 997 opened, C, W, teleport | P `kerwan_story` |
| | infobot 750 | P 4 | P |
| | director 1342, train 822, air traffic 795 | hints, skill points 3 / 4, the train ride | P |
| 04 Eudora | Suck Cannon 1120 `0x2e1a10` | touch → S 2 → I 9, C, W | P `eudora_story` |
| | informant 1190 `0x2e3078` | talk node 2 → P 6, D, W | P `eudora_story` |
| | director 1343, cranks 280 | hints | P |
| | the 466 family | skill point 6 (0x13d40e, 10 kills) | — (class unported: G-CLS-001) |
| 05 Rilgar | race girl 918 `0x316ab8` | talk node 3 → the race (start / finish teleports, music); race won (node 4) → F 0, banner 5013, W | P `rilgar_story` |
| | bouncer 919 `0x317470` | talk node 5 → P 7, W | P `rilgar_story` |
| | salesman 925 `0x3180a0` | talk nodes 3 / 4 → I 23 (R.Y.N.O.), banner 5014, W | P `rilgar_story` |
| | hoverboard 439 `0x2f87a8` (also 16) | mount (hero state 0x6b), H 5007 | — (hero state 0x6b: G-HERO-002) |
| | director 1347 | hints | P `help_rilgar` |
| 06 Blarg | scientist 1105 `0x301070` | creatures of his group asleep → talk: D; node 2 → I 29 (Grindboots), W, H 6005, camera glide | P `blarg_story` |
| | item scene 1016 (= 1005's code) | touch → S 2 → I 22 (Hydrodisplacer), W | P `aridia_story` |
| | infobots 750 ×2 | P 5 | P |
| | Clank's lab 1028 `0x2f55a0` | Clank with the Hydrodisplacer → the drain cutaway (the water levels), D, H 6006, banner 6012, W | — G-CLS-033 |
| | shuttle 1109 `0x302578` | the shuttle rides (menu, spline flight), the final ride → S 12, P 5, D, C; S 5 / V 5, banner, W | — G-CLS-032 |
| | 1035 (help 6004), 1302 (D) | | — G-CLS-033 |
| | Clank's station 1061, director 1348 | the body switch, hints | P |
| 07 Umbris | director 436 `0x2f5ba0` | S 0 (F 0x28); the lair (F 0x29): S 1 / S 2, the fight; the exit (F 0x2a): S 3, V 8, S 4 → P 8, the map revealed, W (as Batalia), X 8 | P `umbris_story` |
| | infobot 750 | P 8 | P |
| | 1106 `0x314150` (1753 lines) | D, C, talked | — G-CLS-033 |
| 08 Batalia | commando 1130 `0x302ce8` | talk (F 0x2c, moves) → node 3 → P 10 (banner), D, C, W | P `batalia_story` |
| | deserter 1144 `0x305270` | near: D; talk node 4 → P 9, W | P `batalia_story` |
| | turret host 1283 `0x3065d8` | talk node 2 → C, mounted on the turret; node 3 → IV 1 → D, I 27 (Metal Detector), W; node 4 → death reload | P `batalia_story` |
| | 253 `0x2d42d8` | H 8004, F 0x2e, skill point 0xd | — G-CLS-033 |
| | director 1349, fighters 438 | hints, skill points 0xb / 0xc | P |
| 09 Gaspar | Pilot's Helmet 1290 (level01 `0x30a6d0`) | touch → S P[0] → I 7 worn, H 9000, D, W | P `gaspar_story` |
| | director 1000, platforms 1182–1189, path ships 1212 | hint 9001, skill points 0xe / 0xf | P |
| 10 Orxon | Magneboots 18 `0x298668` | touch → S 2 → I 28, C, W | P `orxon_story` |
| | Nanotech seller 1326 `0x2e8358` | △ → F 4 / 5, max health 5 / 8, bolts −4000 / −30000, S 0 / 1, W | P `orxon_story` (G-SAV-005) |
| | infobots 750 ×2 | P 11, P 12 | P |
| | Clank section 22, ship boarding (travel) | the body switch; scenes 4 / 5 / 6, movie 11 | P |
| | 1229 (help 10005), 1302 (D) | | — G-CLS-033 |
| 11 Pokitaru | commando 114 | the boats, I 6 (O2 Mask), D | P `pokitaru_commando` |
| | O2 Mask prop 23 `0x2cb668` | state 2 (another class) + touch → owned / acquired 6, W, H 11000 | P `pokitaru_story` |
| | Thruster-Pack giver 90 `0x2d0710` | talk in cuboid → node 2 → I 3, C, W, H 11003 | P `pokitaru_story` |
| | Persuader giver 298 `0x2f0e40` | H 11011 reminder; near: D, C; node 2 → owned / acquired 35, F 1 = 0, banner 11014, W, H 11001 | P `pokitaru_story` |
| | jet plane 1242 `0x313290`, 1179 (help) | D, H 11002 / 11004 | — G-CLS-032 / G-CLS-033 |
| 12 Hoven | 282 `0x2e6c48` | talk (F 1 clear) → F 1, banner 12007, F 0x61, D, C, W | P `hoven_story` |
| | Hydro-Pack giver 328 `0x2eb570` | talk: D, C; node 2 → I 4, banner 12006, W | P `hoven_story` |
| | scene triggers 1404 `0x3093c8` | cuboid → F 0x5c + n, S n | P `hoven_story` |
| | turret 1267 `0x303540` | the turret game → D, V 13, **P 13**, S 1, W | P `hoven_turret` (the game: `hoven_carrier` 1274, `hoven_drone` 326 / 409, `turret_shell` 458, `gun_shot` 184, `drone_rider` 1371) |
| | 1274 (S 7), 326 (help 12002) | the carrier's death → S 7; the drones' hint 12002 | P `hoven_carrier`, `hoven_drone` |
| | 1345 (D) | | — G-CLS-033 |
| | director 422 | hints, skill point 0x14 | P |
| 13 Gemlik | director 1353 `0x30b628` | S 0 (F 0x64); the ambush (F 0x65): S 1, the vehicle 69; Qwark's escape (F 0x66): S 2, V 14, S 3 → P 14, the landing spot, W | P `gemlik_story` (the ship adoption: G-LVL-002) |
| | vehicle 69 `0x2bb068` | the ride (state 0x32), D, C | — G-CLS-032 |
| | gold-weapon offers 304, director 558, breakables 1805 | gold weapons, hint, skill point 0x16 | P |
| 14 Oltanis | Qwark 851 `0x2fba20` | cuboid: D, C, the course on; nodes 3 / 4 → I 32 (PDA), W | P `oltanis_story` |
| | scrap merchant 924 `0x2fefe0` | cuboid: D, C; node 4 → P 15, W | P `oltanis_story` |
| | Morph-o-Ray 1354 `0x305758` | touch → I 21, S 6 → D, W, banner, H 14000 | P `oltanis_story` |
| | 684 `0x2ed280` | S 0 (F 0x68, the ship hidden), then the lightning | — G-CLS-033 |
| 15 Quartu | Bolt Grabber 1388 `0x2ea748` | touch → S 5 → I 34, W | P `quartu_story` |
| | broadcast 1419 `0x2eb4c0` | cuboid + moby P[2] in state 3 → S 3, D; V 17; S 4 → P 17, C, W | P `quartu_story` (the captions: G-UI-025) |
| | Giant Clank mission 1446, pads 1451 | P 16 | P |
| | help director 1469 `0x2ed398` | the sentry / Hologuise hints (15000..15002, 15006, 15007), the Giant Clank hint 15004 (F 0x6f), the Hydro-Pack hints 15005 / 20014, F 0x6e | P `quartu_story` (the disguise's branches wait on body 3: G-WPN-006) |
| | 1179 (pads, help 11009 / 11010) | | — G-CLS-033 |
| 16 Kalebo III | Map-O-Matic giver 1377 `0x2e3190` | talk → teleport, H 16002, I 33, D, C, W; F 0x70 / 0x71 | P `kalebo_story` |
| | race host 1455 `0x2e6808` | the grind race; node 4 → IV 0 → I 31 (Hologuise), W; skill point 0x1a | P `kalebo_race` (the race itself: G-LVL-007) |
| | 439 | the hoverboard | — (G-HERO-002) |
| 17 Drek's fleet | item scene 1428 `0x2f1790` | touch → S 1 → F 0x78, F 2, banner 17002, W | P `fleet_story` |
| | infobot 750 | P 18 | P |
| | vehicle 1379 `0x2ed018`, 1470 (help) | D, C | — G-CLS-032 / G-CLS-033 |
| 18 Veldin | boss 1422, pads 586, cutaway 644, Giant Clank pad 1899 | the finale (slideshow, ending movie) | P |
| | 1750 `0x2fad08` | the ending save | P (`ending_save`, saves lane) |

### What a new driver needs

Each remaining driver is per-class code (no script system): a module under `classes/units/`, a `UnitPort` row (the
census unit, the reference level and update), and the calls above. NPCs reuse `talking_npc::look_at_layout` with a
`LookLayout` row (records, glance / seen / timer offsets, eye height, the pitch / yaw factors, the timer width, the
gate sequences) and `story_npc::{idle_anim, talk_radius, place_grounded, attach_child}`; item pickups reuse
`gold_bolt::{glow_init, item_glow}` (the infobot's glow, same code); the actors' big-head cheat is
`manip::scene_big_head(w, &[classes], list, scale)` at the top of the update.

## media

Front-end movies, the credits slideshow, the end of the game, Goodies and cheats (batch 6, lane `media`). Code:
`rc_game::cheats`, `rc_game::slideshow`, `rc_game::movie_player`, `rc_game::menus::pause::media`, `rc-engine`
`media_render` / `movie_render` / `mirror_render` / `menu_render` (the glue), `rc_game::moby_update::manip` (the
big-head nodes). Each module's doc has its coverage table.

### Movies outside gameplay

| entry | file | language | skip (0x15eed8) | after |
|---|---|---|---|---|
| `DialogStreamUpdate(n)` 0x2acf50 (classes, talkers; the Cinematics page's action 7) | `mpegs[2 + n]` | 0x15ed88 | 0 (Start + L1 L2 R1 R2; Start alone once beaten / completed / on level 0); from the menu 2 (any button) | `MovieExitToGameplay`: mode 0, the level track restarted, the talker refreshed, 0x15eed8 = 2 → 0 |
| `PlayMovieB(n)` 0x2ad050 (the Cinematics page's action 8; the boss 1422's `PlayMovieB(11)` = the ending) | `mpegs[40 + n]` | 0x15ed88 | from the menu 1 (Start alone) | the same |
| `PlayMovieC(n)` 0x2acfe8 (the Commercials' and Making Of's action 10) | `mpegs[70 + n]` | 0 | 1 | the same |
| boot `fun_001e9488(i)` (the title's attract loop, the `saves` lane's title calls `movie_render::play_attract(i)`) | `mpegs[80 + i]` | 0 | 2 (set by the call, 0 after) | `FadeToBlack(ticks(12))` in, `FadeToBlack(4)` out, no music restart, back to the title |

The page menu's post-actions (`PageMenuUpdate` 0x28c990 after its two closing ticks): 3 / 4 / 6 (movie), 5 (scene: the
In-Level Movies list's action 6 = `DialogStreamStart(arg & 0xffff)` of this level), 7 (the slideshow: Credits' action
11): the old 0x15eed8 into 0x1ba2a0, the new one (3: 2; 4 / 5 / 6: 1; 7: unchanged), `FadeToBlack(ticks(16))` over the
menu image, mode 0, the action, `snd_ContinueAllSoundsInGroup(0x1d)` and `music_Unpause`. The list item set the return
page 0x1ba260 / kind 0x1ba264: back in mode 0, `InLevelFrameUpdate` reopens the page menu on that page before the
tick (`EnterMenuMode(0)`), restoring 0x15eed8 from 0x1ba2a0.

### The end of the game (level 18, the boss 1422)

Scene 5 → (mode 0) `EnterSlideshowMode` (the credits) → (mode 0) `PlayMovieB(11)` (`mpegs[51]`) → (mode 0)
`EnterMenuMode(0x21)` (Lombyte `PauseAllSounds`; it pauses SFX group 0x1d and the music because it enters the menu): the
end page 0x1b7670 (kind 0x22, no close keys): the Helpdesk girl (W0, not ported: G-CUT-008), "You've got:" the gold
bolts (of 40), gold weapons (of 10) and skill points (of 30) with check boxes (W1), the hints "○ Start new game with
current weapons and bolts" / "✕ Timewarp to before you defeated Drek." (W3). ○: with a memory card (0x15eeb0 ∈ {1, 0x10})
the save page 0x1b9440 (the challenge-mode save, `saves` lane), else challenge mode (`fun_00226b08(−1)`) and the level
exit to Veldin (0x15f5c0 = 0, 0x15f570 = 1, 0x13e05a = 1). ✕: `memcard_RestoreGame(0x1ba250)` keeping level 18's gold
bolts (0x14bf08) and the skill point 0x13d425, 0x15eea0 = 1 (the game beaten), `memcard_Save`, the checkpoint records
cleared, `FadeToBlack(16)`, 0x141401 = 1, the menu closed. The port hands both choices to the `saves` lane
(`media_render::take_end_choice`); the level exit goes through `EngineRequest::LeaveLevel`.

### The credits slideshow (mode 7)

`EnterSlideshowMode` 0x2ad558: `FadeToBlack(8)`, mode 7, fade 0x15f3fc = 1.0, picture 0 shown, picture 1 loading
(`credits_images_ntsc`, 20 raw 512×416 pictures; index i loads `i % 20`). Each frame: the fade −0.05; the picture
times 0x174070[i] − 36 start a 32-tick column dissolve (pixels x % 32 = counter copied), the next picture is read at
tick 36 of it; the US build jumps from picture 17 to 28 (time 8760) and from 37 to 40 (time 12980); picture 43's time
(14564) ends the mode, as Start does. The render: the shown picture, the black fade, then the 183 text entries
0x173500 (`{msg, x, y, start, duration, align, join}`: shown from start to start + duration, the bar fading in over 16
ticks and out over 8, the text over 16 / 8 ticks later, in the regular font with a black shadow). The exit: `FadeToBlack(12)`,
mode 0, fade 0. Music: none started (the level's keeps playing, `sound_update` every frame).

### Goodies (`EnterMenuMode` → `FUN_0029aa60`)

Goodies is in the root page (and wired below Weapons / above Options) once the game is beaten (0x15eea0) or completed
(0x15ee20) (or 0x1ba268, no writer found). Entries: Skill Points (page), Credits (the slideshow), Cheats (page), Cinematics
(page: 29 replays, actions 7 / 8), In-Level Movies (this level's scenes, action 6), Sketchbook (≥ 15 skill points; 30
pictures, ✕ / ○ page), Epilogue (all 30 skill points; 12 pictures per language, → / ✕ next, ← / ○ previous), Making Of
(all 10 gold weapons: `PlayMovieC(3)`), Commercials (all 10 gold weapons; page of three `PlayMovieC(0..2)`). A locked
entry is action 2 ("????", the denied sound) and its description says what unlocks it (0x4fd9, 0x4fda, 0x4fdb).

### Cheats (`rc_game::cheats`)

| cheat | byte | move code to enter it (game beaten / completed; on foot) | effect |
|---|---|---|---|
| Actors have oversized craniums | 0x15edb0 | 1 2 3 3 4 5 4 6 | NPC heads ×2.75 / scene actors ×2.1 / 2.75 / 2.5, the mouse ×2.1 |
| Ratchet has a big head | 0x15edb1 | 3 3 3 7 10 8 | record 17 → 1.57 |
| Trippy contrails | 0x15edb2 | 9 ×10 5 6 | the vehicles' contrails 16 wide instead of 5 (their classes are not ported) |
| Clank has a large noggin | 0x15edb3 | 3 6 4 5 6 1 2 7 | 0x15ee18 → 1.8, record 18 / 25, Giant Clank's 28 (×1.4), the antenna glow 3.1, the camera eye 1.2 |
| Levels are mirrored | 0x15edb4 | 1 1 1 1 12 13 14 6 2 2 2 2 5 7 | the world drawn mirrored, the pad's left / right swapped |
| Health gives invincibility at max | 0x15edb6 | 4 4 4 4 3 7 3 7 4 4 4 4 | a nanotech cluster taken at full health gives `ticks(600)` of invulnerability |
| Enemies have massive domes | 0x15edb7 | 10 3 3 3 10 3 3 3 10 3 3 3 7 | the enemies' head manipulators (2.1 .. 3.6) |

Move codes (`0x2285a0`, on the tick the state is ticks(15) old): 1 / 2 / 3 the side flip left / right / the back flip
(state 0xb; 1 ↔ 2 mirrored), 4 the comet strike (0x15), 5 the double jump (0xe), 6 the wrench jump attack (0x14), 9 the
wall jump (0x11), 10 the Heli-Pack long jump (0xa), 11 the Thruster-Pack stomp (0x22), 12 + the combo row in the wrench
combo (0x13: 12, 13, 14 = its three swings); held ticks(60): 7 the crouch (4), 8 the Heli-Pack glide (8). A match toggles the byte, marks it ever activated (chunk 37: the Cheats
page lists only those) and shows "Cheat Enabled" / "Cheat Disabled". The Cheats page (Goodies) toggles them. The pause
menu's debug code entry `MenuInput` 0x298f80 (hold R2 + L1, 20 presses of the d-pad, □ and ○, matched against the
table 0x1ba020) gives items, unlocks planets, sets global flags 0..5 or awards skill points.

### Requests the classes make (`rc_game::cinematic`)

`EnterSlideshowMode`, `PlayMovieB(n)`, `EnterMenuMode(kind)` set the moby loop's mode word at once (7 / 1 / 3) and are
taken right after the tick (`media_render::take_class_requests`); the engine's mode follows from the next frame.
