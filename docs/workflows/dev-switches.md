# Workflow: engine switches (command line, settings file, environment)

How to steer a `rerac` run: its two command-line options, the settings file, and the `RC_*` environment switches.
All of them work the same under `cargo dev` (`cargo dev -- <options>`) and on a built `rerac`.

## Command line

| Option | Effect |
|---|---|
| `--data-dir <folder>` (also `--data-dir=<folder>`) | The game data folder; how the launcher starts the game. Selection order and exit codes: `docs/workflows/game-data.md` "How the engine finds its data" |
| `--version-json` | Prints the launcher contract line (`{"name":"rerac","version":"0.2.0","game":"rac1","data_format":1}`) and exits without touching any data |

## Settings file

Port settings (MSAA, the "Port Options" page) live in `~/Library/Application Support/rerac/settings.toml` (macOS),
`$XDG_CONFIG_HOME/rerac/settings.toml` (else `~/.config/rerac/`, Linux) or `%APPDATA%\rerac\settings.toml`
(Windows). When that file is missing, a pre-rename `randcrw/settings.toml` there is copied over once on the first start
(the old one is kept). `RC_SETTINGS_FILE=<path>`
picks another file; `RC_SETTINGS_FILE=0` (or empty) disables it. Frame-exact runs (`RC_DETERMINISTIC=1`,
`RC_SCREENSHOT_FRAME`, `RC_DUMP_FRAMES`) neither read nor write the default file; an explicit `RC_SETTINGS_FILE` still
applies to them (`crates/rc-engine/src/render_settings.rs`).

## Environment switches

Boolean switches are on with `1` (or off with `0` where the default is on).

This table lists the switches for everyday development. The engine reads more (trace, dump and survey switches added
by individual ports, e.g. `RC_AUDIO_TRACE`, `RC_MOVIE_TRACE`, `RC_SCREENSHOT_FRAME`); each is documented in the
module that reads it: `grep -rn '"RC_' crates/` lists them all.

| Variable | Effect |
|---|---|
| `RC_DATA_DIR` | Game data folder when `--data-dir` is not given (see "Command line") |
| `RC_EXTRACTED` | Development data tree (default `<repo>/extracted`); the engine's fallback when neither `--data-dir` nor `RC_DATA_DIR` is set, and the tests' root |
| `RC_CACHE` | `0`: do not use or write the engine cache `<data>/cache/v1` (decompress in memory, once per process) |
| `RC_PERF_LOG` | `1`: print one line per game-data lump request (engine cache, memory or decompressed; MiB, ms) |
| `RC_AUDIO` | `0`: no audio (the level's sound bank and music are not loaded); the `cargo xtask test-*` commands set it |
| `RC_LEVEL` | Start straight in this level (skips the front end unless `RC_FRONTEND=1`). Without it the game boots into the front end; the level loaded behind it is Novalis (1) |
| `RC_LANDING` | `1`: the boot's level enters as after a level change (`entry`: the music pause / unpause and `ShipLandingStart`'s landing scene); default: the level's start in mode 0 (or `RC_HERO_AT`) |
| `RC_FRONTEND` | The front end (the card check, the logos, the title world, PRESS START, the main menu) is the default start. `0`: skip it and start in the level; `1`: keep it even with `RC_LEVEL` |
| `RC_UNPORTED` | `1`: every 600 ticks, print the calls the moby loop met without a port so far (`Services::unported`, by name and count) |
| `RC_SKIP_LOGOS` | `1`: in the front end, skip the publisher logos movie and go straight to the title |
| `RC_SAVE_DIR` | the memory-card folder (`0` or empty: no card); default `<config>/rerac/memcard`. `RC_SAVE_TRACE=1` logs every card state change |
| `RC_CAM` | Starting camera `ex,ey,ez,tx,ty,tz` (eye and target, game units) |
| `RC_SCREENSHOT` | Save a screenshot to this path, then exit |
| `RC_SCREENSHOT_DELAY` | Seconds before the screenshot (default 3) |
| `RC_DUMP_FRAMES` | `start..end`: save every frame from update `start` to `end` (inclusive) as `frame_NNNNN.png` into `RC_DUMP_DIR` (default `frames`), then exit (dev only; frame-exact, offscreen like `RC_SCREENSHOT_FRAME`) |
| `RC_DUMP_DIR` | Folder for `RC_DUMP_FRAMES` |
| `RC_DUMP_REALTIME` | `1`: `RC_DUMP_FRAMES` in real-time mode (wall-clock ticks as in play, not frame-exact; dev only) |
| `RC_DUMP_TICKS` | `n,m,…`: in frame-exact mode, update k runs the k-th entry's game ticks, cycled (`1,0` = a 120 Hz display, `2` = 30 Hz; dev only, for real-time pacing repros) |
| `RC_NOVSYNC` | `1`: present without vsync, so `fps:` measures headroom |
| `RC_FRAME_CAP` | `0`: no 60 fps limiter (`crate::frame_pace`; also off with `RC_NOVSYNC=1`); ticks then follow the wall clock |
| `RC_RENDER_DIAG` | `1`: Bevy's per-pass render timings (CPU only on Metal) every 5 s, and the number of views rendered |
| `RC_DRAW_STATS` | `1`: every 300 frames, per material type the mesh entities, the visible ones and their distinct materials / meshes, and the active cameras (`crate::draw_stats`) |
| `RC_MOBY_DUMP_AT` | `<tick>`: print the **F9** moby dump once at that gameplay tick (headless runs); `RC_MOBY_DUMP_RADIUS` = its distance from the hero (default 12) |
| `RC_RESPAWN_PROBE_AT` | `<tick>`: print the respawn state once at that gameplay tick, as **F10** does in dev builds (the format of `work/scratch/respawn_probe.py`, which reads the same tables from a running PCSX2 over PINE: compare the two line by line) |
| `RC_TRACE_RESPAWN` | `1`: at every death reload, the spawn test's verdict for each Kerwan trooper (574) with the bits that decided it, and a line whenever a trooper blows up or is deleted (`respawn trace:`) |
| `RC_TRACE_BOTS` | `1`: Clank's gadgetbots and their pads (class 857 / 1302) on stderr: each bot merged into a pad with the count left, each knock-back home, each pad trigger with its count, mission and the class and state of each linked door (a door opens only from state 2) |
| `RC_ASPECT` | `4:3` (default: the original TV picture), `16:10` or `16:9` (Hor+: the view widens, the HUD's side elements move to the frame's edges, the menus stay in the centred 4:3 box); overrides the Port Options value at start (crate::display) |
| `RC_RESOLUTION` | `window` (default: the largest frame of the Aspect ratio that fits the window) or `416`, `720`, `1080`, `1440`, `2160` (a frame that many lines high at the Aspect ratio, scaled to the window); overrides the Port Options value at start (crate::display) |
| `RC_FULLSCREEN` | `1`: borderless fullscreen on the current monitor; `0`: windowed; overrides the Port Options value at start (F11 toggles in game) |
| `RC_FOG` | `0`: disable fog |
| `RC_NO_LIGHT` | `1`: skip the load-time lighting passes (tfrag, tie, shrub, moby); stored colours used |
| `RC_WORLD_LIGHTS` | `0`: point lights (explosions) do not relight tfrags, ties and shrubs |
| `RC_WORLD_LIGHTS_TRACE` | `1`: print, every frame, the frame time, the point-light bank and the listed tfrag / tie / shrub counts |
| `RC_LOD` | `0`: force tfrag LOD 0 |
| `RC_LOD_TINT` | `1`: tint tfrag LOD 1 red, LOD 2 blue, clipping path green |
| `RC_TIE_LOD` | `0`: force tie LOD 0 with morph k = 0 (culling unchanged) |
| `RC_TIE_LOD_TINT` | `1`: tint tie LOD 1 red, LOD 2 blue |
| `RC_SEA` | `0`: do not draw the seas and liquid surfaces of the draw callbacks (crate::sea_render) |
| `RC_NO_TIES` | `1`: do not draw ties |
| `RC_NO_SHRUBS` | `1`: do not draw shrubs |
| `RC_SKY_ROT` | Sky rotation speed in ticks per 60 Hz tick (`0` freezes; default 1) |
| `RC_ANIM` | `0`: disable moby animation (bind pose) |
| `RC_MOBY_CPU_LIGHT` | `1`: use the bit-exact CPU moby lighting (bind pose) instead of the GPU path |
| `RC_MOBY_GLOW` | `0`: no moby glow list (glow packets drawn lit instead of in the moby's glow colour +0x90) |
| `RC_MOBY_LIGHT_CHECK` | `1`: compare GPU vs bit-exact CPU moby lighting at load and report |
| `RC_OCCL` | `0`: occlusion off; `1`: freeze the mask built from the starting camera |
| `RC_OCCL_STATS` | `1`: print occlusion cell and cull counts, at most once a second |
| `RC_GIVE_HYDROPACK` | `1`: own the Hydro-Pack (item 4) from the start (debug; the swim code reads it) |
| `RC_GIVE_ITEMS` | `id,id,…` (decimal or `0x` hex): own those items from the start (debug; `rc_game::inventory::debug_grant`, docs/plan/gadgets.md §6); the last back (2 Heli-Pack, 3 Thruster-Pack, 4 Hydro-Pack), feet (28 Magneboots, 29 Grindboots) and head item (5..7) among them are saved as equipped, and the last hand item (e.g. 12, the Swingshot) is requested into the hand. Unset: the game's own starting state |
| `RC_GIVE_ITEMS_EQUIP` | `0`: `RC_GIVE_ITEMS` only owns the items (nothing equipped; equip them in the pause menu's Gadgets page) |
| `RC_HERO_AT` | `x,y,z[,yaw]`: place Ratchet there at the level load, before the hero init's ground snap (debug; e.g. on a grind rail with `RC_GIVE_ITEMS=29`) |
| `RC_LEVEL_CAMERAS` | `0`: do not load the level's camera records (no class-17 regions: the follow camera alone, as before `rc_game::follow_camera::level`; for before / after comparisons) |
| `RC_GIVE_BOLTS` | `n`: start with n bolts (debug; e.g. to buy at the Gadgetron vendor) |
| `RC_INTERACT_TRACE` | `1`: log the context prompt's owner changes, the "use" hand-offs (vendor, talkers), vendor purchases and sounds (docs/plan/interaction.md) |

Examples:

```
RC_LEVEL=13 cargo dev                                  # start on Gemlik Base
RC_GIVE_ITEMS=2,12,29 RC_GIVE_BOLTS=5000 cargo dev     # Heli-Pack, Swingshot, Grindboots, 5,000 bolts
RC_DUMP_FRAMES=100..160 RC_DUMP_DIR=work/captures/run1 cargo dev
```
