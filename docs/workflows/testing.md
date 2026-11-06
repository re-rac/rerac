# Workflow: testing (tiers, areas, the test audit)

What runs when, which tests belong to which area, and the audit of every test in the workspace (2026-09-29). All
commands run from the repo root with `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`. Tests that need game data
skip themselves when `extracted/` is absent.

**No test relies on the user's personal files** (§10): not the disc image, savestates, PCSX2 traces, screen
recordings, the settings file or session output in `work/`, not even "skip if missing". Tests read `extracted/`
through the data root and committed fixtures; findings from personal material reach them only as a few distilled
numbers. `guards::no_test_or_product_file_references_personal_paths` enforces it.

## 1. Commands that share the dev build

**Tests run only through `cargo xtask test-*`.** Never run `cargo test` or `cargo nextest` directly: there is no
Cargo test alias, and the xtask commands pass the one flag set that shares the dev Bevy build
(`--workspace --features rc-engine/dev`). Compile checks use the `.cargo/config.toml` aliases:

| What | Command |
|---|---|
| All unit tests (every crate's `src/` tests; rc-engine's live in the `rerac` bin) | `cargo xtask test-quick` |
| One crate's unit tests | `cargo xtask test-quick <crate>` (e.g. `rc-formats`) |
| A job's tier (§2.1) | `cargo xtask test-job <area…> [shared]` |
| One or more integration binaries alone (the binaries of §3) | `cargo xtask test-job --test <binary> [--test <binary> …]` |
| One former test file | `cargo xtask test-job --test <binary> --filter <module>::` |
| One test | `cargo xtask test-job --test <binary> --filter <module>::<test_fn> --exact`, or a substring: `cargo xtask test-job --test hero --filter novalis_hero_digest` |
| Ignored surveys, with their output | add `--ignored --nocapture` (e.g. `cargo xtask test-job --test classes --filter creature_classes:: --ignored --nocapture`) |
| Type check of everything, with tests | `cargo check-all` |
| Lints | `cargo clippy-all` |
| Full suite | `cargo xtask test-full` |

The `cargo xtask test-*` commands set `RC_AUDIO=0` (other variables such as `RC_SNAPSHOT_WRITE=1` pass through) and
run through cargo-nextest when it is installed, else through `cargo test --workspace --features rc-engine/dev` (§8).
`--filter`, `--exact`, `--ignored` and `--nocapture` mean the same under both runners. The integration binaries are one
per area (§3), each with one module per former test file, so a test's path is `<module>::<test_fn>`, e.g.
`hero_novalis::novalis_hero_digest` in the binary `hero`.

Why no by-hand runs: `cargo test -p rc-game --lib` resolved other features and recompiled rc-formats and rc-game from
scratch (32 s, measured 2026-09-29) next to the shared copy; plain `cargo test --workspace` rebuilds Bevy statically;
and the old `cargo test-all` alias silently ran the whole workspace when given `-p <crate>` (removed 2026-09-29).
`cargo xtask test-quick` was up to date in 0.4 s after a full run.

## 2. Tiers

Two tiers.

### 2.1 Per job

Each brief names its areas, for example "areas: weapons, classes". The job runs:

1. **Unit tests:** `cargo xtask test-quick` (`--lib --bins`), every crate. The whole run takes about 5 s, and it is the only
   cheap way to cover a dependency change, so it is not split by area.
2. **The area groups** of §3, for each area the job touched.
3. **The shared-code guard set**, when the job touched shared code (§2.3):
   - Ratchet's NO_IDLE digest. **A change detector, not a correctness check:** it pins the port's current hero
     behaviour, which is known to be imperfect, so a match says "nothing moved", never "the movement is right". It
     must be byte-identical to the baseline, unless the brief changes hero behaviour on purpose. Movement correctness
     will come from distilled PCSX2 values in the (deferred) hero feel pass (docs/plan/hero_feel_pass.md). The test
     runs with the package as its working directory, so give it absolute paths:

     `cargo xtask test-job … shared` does this (it writes `work/test-results/digest_job.txt` with
     `RC_HERO_DIGEST` and `RC_HERO_DIGEST_NO_IDLE=1` and compares it byte for byte with the baseline).

     The baseline `work/test-results/hero_digest_no_idle.txt` is local: git-ignored (`/work/`), never committed,
     and written only by `cargo xtask digest-baseline`, which prints what changed. Run it only when a human has decided a digest change is
     intended, or when there is no baseline yet (a job takes one before its first edit). `test-job shared` and
     `test-full` only compare with it, and fail on a mismatch. A test that rewrote its own reference on every run
     would bake regressions in.
   - `all_levels_smoke::` in `world` (also part of `test-job shared`): 19 levels × 600 ticks and Gemlik × 3000 (about 26 s).
4. **`cargo check-all`** once at the end, and `cargo clippy-all` for the job's files (as before).

The report lists every command run and its result.

### 2.2 Full suite

`cargo xtask test-full` runs only:
- on merge to main;
- for a big shared-code commit, when the coordinator decides it.

The coordinator runs it through the commit agent and writes `work/test-results/latest.txt` (time, HEAD,
`git status --short`, per-suite counts, failures). The same run compares the NO_IDLE digest with the baseline (§2.1)
and fails on a mismatch; it never rewrites the baseline. The last full run
(2026-09-29 02:52, before the merge) took about 9 min of test time (520 s summed over the suites, before
`creature_classes` was added); `water_levels` alone took 176 s. After the merge job (§5) the full suite under nextest
takes 139 s (1167 run, 26 ignored).

### 2.3 What counts as shared code

Shared code is code that every level or every tick runs through. A change to it can break areas the job did not name.
- **rc-game:** `tick.rs`, `lib.rs`, `hero.rs` and the hero core (`hero/{physics,states,common,anim,ground,walk,jump,air,damage}.rs`), `pad.rs`, `follow_camera*`, `collision_query*`, `rng.rs`, `ps2v.rs`, `moby_runtime.rs`, `moby_update.rs`, `moby_update/{scheduler,services,interact,triggers}.rs`, `moby_update/classes/mod.rs` (the `LevelPorts` registry), `particles.rs` (the dispatcher).
- **rc-formats:** the level loaders every level uses (`level`, `gameplay`, `moby`, `collision`, `wad`, `level_overlay`, `test_data`).
- **rc-engine:** the level load and the main loop. These tests do not reach rc-engine; there, the screenshot rules of the brief are the check.

## 3. Area map (per-job groups)

The unit tests of every area run in step 1 of §2.1. The table lists each area's integration binary and its modules
(the former test files, merged 2026-09-29, §5): `cargo xtask test-job --test <binary>`, or add
`--filter <module>::` for one module. `cargo xtask test-job <area…>` runs the unit tests plus the listed binaries; its area table is `AREAS`
in `tools/xtask/src/test.rs`, which this table documents (area names and aliases are the same).

Aliases: creatures and mobys → `classes`; levels, collision and water → `world`; menus, HUD, map, save and vendor → `ui`;
movies → `video`; render and input → `engine`; fx → `particles`; guards and layout → `repo`.
Wall times: the 2026-09-29 full run before the merge, summed over the old binaries; for `classes` and `world` also
the merged binary under `cargo test` (per-binary caches, §5).

| Area | Modules (former test files) | Binary | Unit-test modules (in the lib) | Wall time |
|---|---|---|---|---|
| hero | `hero_boots_grind` `hero_boots_magnet` `hero_cable_kerwan` `hero_damage` `hero_followups` `hero_ledge_novalis` `hero_novalis` `hero_pack_swap_novalis` `hero_packs_novalis` `hero_platform_novalis` `hero_surfaces` `hero_swingshot_levels` | `hero` | `hero::*` except the weapon modules, `follow_camera`, `pad`, `afterimage` | ~8 s |
| weapons | `hero_doom` `hero_gameplay_novalis` `hero_gloves` `hero_guns_novalis` `hero_morph` `hero_pyrocitor_novalis` `hero_reactive_novalis` `hero_targeting_novalis` `hero_visibomb` `hero_weapons4` `hero_weapons_novalis` | `weapons` | `hero::{blaster,comet,devastator,guns,melee,morph_ray,pyrocitor,ryno,suck_cannon,suck_vortex,tesla,walloper,weapons,…}`, `targeting` | ~8 s |
| classes | `bolt_crank_novalis` `breakables_levels` `cheap_classes_a` `cheap_classes_b` `common_classes_levels` `creature_classes` `creatures_enemies_novalis` `creatures_novalis` `explosion_survey` `gold_bolt_infobot_novalis` `moby_update_novalis` `path_classes` | `classes` | `moby_update::*` (classes, creature, units), `moby_runtime`, `path`, `spline` | ~200 s before; 14 s merged |
| world | `all_levels_smoke` `cutscene_novalis` `level_ports` `moby_collision_novalis` `novalis_collision` `novalis_world` `sea_levels` `shadow_volume_novalis` `water_levels` | `world` | `particles::*`, `water::*`, `shadows::*`, `collision_query`, `fog_zones`, `point_lights`, `sky_stars`, `cinematic`, `rng`, `ps2v` | ~290 s before; 32 s merged |
| ui | `gadgets_novalis` `game_state_novalis` `gemlik_generalisation` `help_novalis` `interaction_vendor` `map_levels` `pause_pages_novalis` `reverb_conformance` `sound_conformance` | `ui` | `menus::*`, `map::*`, `hud`, `help`, `game_state`, `inventory`, `movie_player`, `scene_player` | ~8 s |
| audio | `reverb_conformance` `sound_conformance` (in `ui`), plus `weapons -- --exact hero_guns_novalis::novalis_ryno_salvo_sounds_mix_without_overflow` | in `ui`: `--test ui -- reverb_conformance:: sound_conformance::` | `audio::*` | ~3 s |
| formats | `golden` `level_overlay_disc` `moby_anim_golden` `moby_collision_disc` `moby_shadow_disc` `occlusion_frames` `scene_coverage` `tfrag_light_golden` (rc-formats) | `formats` | rc-formats lib | ~6 s |
| data | `lifecycle` `roundtrip` (rc-data) | `data` | rc-data lib | ~3 s |
| extract (extractor, launcher side) | `synthetic` (rc-extract; the checks against a real disc are `rc-trace disc-check`, §10) | `extract` | rc-extract lib and bin | <1 s |
| video | `movies` (rc-video) | `movies` | rc-video lib | <1 s |
| engine (render, input, engine glue) | none | — | the `rerac` bin's tests | — |
| particles (alias fx; added 2026-09-29) | `particle_consumers` (in `classes`: the ported classes' particle types living their lives) | in `classes`: `--test classes --filter particle_consumers::` | `particles::*` (listed under world; every job runs the unit tests) | ~5 s |
| trace tools | `hero_replay_selfcheck` `novalis_spawn` `synthetic` (tools/trace) | `trace` | rc-trace lib | ~2 s |
| repo layout (new crates, dependencies, top-level files) | `guards` (tools/repo-checks) | `guards` | — | <1 s |
| shared code (§2.3; area `shared`) | the guard set of §2.1: `hero -- --exact hero_novalis::novalis_hero_digest` (digest compare) and `world -- all_levels_smoke::` | — | — | ~27 s |

A brief with "areas: weapons, classes" runs `cargo xtask test-job weapons classes`: the unit tests plus
`--test weapons --test classes`. The areas `engine` (unit tests only) and `repo` (`guards`) and the aliases are in
`cargo xtask help`.

## 4. A separate canary set?

No (decided 2026-09-29). The unit tests plus the shared-code guard set (`cargo xtask test-job shared`: digest and
`all_levels_smoke`, about 30 s) are the canary. The coordinator runs them once more when it commits a batch of
several jobs without the full suite; that catches cross-area breakage that each job's own areas miss. A hand-picked
list would drift out of date with every new port.

## 5. The merge (done 2026-09-29)

The 70 integration binaries are now 11. Each group is `tests/<group>/main.rs` with one `mod` per former file, so every
test keeps its name as `<file>::<test_fn>` (the two repeated names, `units_resolve_on_their_levels` and `survey`, stay
apart). rc-video's `movies.rs` and repo-checks' `guards.rs` were already one binary each and stay where they are.

| Crate | Before | Binaries now |
|---|---|---|
| rc-game | 53 | 5: `hero`, `weapons`, `classes`, `world`, `ui` (audio is inside `ui`) |
| rc-formats | 8 | 1: `formats` (with `formats/snapshot/mod.rs`, the snapshot table writer) |
| rc-data | 2 | 1: `data` |
| rc-extract | 2 | 1: `extract` |
| rc-video | 1 | 1: `movies` (unchanged) |
| repo-checks | 1 | 1: `guards` (unchanged) |
| tools/trace | 3 | 1: `trace` |

What changed besides the moves (no test function or assertion changed):
- **Per-process caches** in `crates/rc-game/tests/common/mod.rs` (`#[path = "../common/mod.rs"] mod common;` in
  `classes` and `world`): the parsed level overlays, the per-level `LevelPorts` (keyed by level and external list),
  and the per-level `LevelWaterData`. The files' own `overlay` / `ports` / `water_data` helpers now return clones of
  the cached values; four in-body `LevelPorts::from_overlays(overlay(level), overlay, &[])` calls
  (`gold_bolt_infobot_novalis`, `sea_levels` ×2, `all_levels_smoke::run_level`) call the same cache. Checked: the
  printed per-level output of `gold_bolt_infobot_novalis` and `sea_levels` is byte-identical before and after, and
  the NO_IDLE hero digest is identical.
- **The four no-assert surveys are `#[ignore]`** (§6); `--ignored` still runs them.
- **Paths:** the fixture `include_str!` of `shadow_volume_novalis` (`../fixtures/`), repo-checks' `KNOWN_OFFENDERS`,
  and the `tests/<file>.rs` / `--test <file>` references in the sources, docs and README now name the merged paths.
- **Heat:** a merged binary runs all its tests on every core under `cargo test`; cap it with `RUST_TEST_THREADS=4`.
  nextest is capped at 8 threads (§8).

Measured (M-series laptop, 2026-09-29, the dev build shared through the aliases):

| | Before | After |
|---|---|---|
| Integration binaries | 70 | 11 |
| The test build (`--no-run`, workspace, dev features) after touching `crates/rc-game/src/lib.rs` (warm) | 20.4 s | 10.4 s |
| The integration binaries on disk (executables + `.dSYM`) | 2.2 GB | 0.46 GB |
| `du -sh target` | 11 GB | 13 GB, of which 3.2 GB are the 68 stale old binaries and their incremental state (`cargo clean` drops them); about 9.8 GB without them |
| `classes` + `world` under `cargo test` | ~490 s summed (200 + 290) | 14 s + 32 s |
| `water_levels::water_inventory_all_levels` | 167 s | 48.5 s (nextest, under full-suite load) |
| Full suite | ~520 s summed test time (`cargo test`, 87 suites, run one after another) | 139 s wall (`cargo xtask test-full`, nextest, 8 threads) |

Under nextest every test is its own process, so the caches only save the repeated parses inside one test; the
13 tests that scan all 19 levels each still build their own `LevelPorts` (30–52 s each under load). Under `cargo test`
they share one build per binary: `classes` and `world` then take 46 s together, so a `cargo xtask test-full --cargo-test` run is
estimated at about 90 s with far less CPU (not measured: the job allowed one full run).

## 6. Slow tests and tests that look wrong

**Slow** (over 10 s each; all others are under ~3 s). "Before": the old binaries under `cargo test`. "Nextest": the
2026-09-29 full run after the merge, one process per test with 8 running at once, so the times include contention;
under `cargo test` the merged binaries share one `LevelPorts` build per level (§5).

| Test | Before | Nextest |
|---|---|---|
| `water_levels::water_inventory_all_levels` | 167 s | 48.5 s (`water_data` cached per level) |
| `level_ports::every_level_runs_the_ports_its_class_table_names` | ~45 s | 34.9 s |
| `breakables_levels::every_placed_breakable_breaks_on_its_level` and `every_breakable_class_resolves_its_data_on_every_level` | ~25–48 s | 40.0 s, 39.3 s |
| the four `units_resolve_on_their_levels` | 22–24 s each | 37–39 s each |
| `common_classes_levels::common_classes_resolve_on_their_levels` | ~25 s | 39.2 s |
| `gold_bolt_infobot_novalis::registered_on_the_levels_with_identical_code` | ~22 s | 36.9 s |
| `sea_levels::sea_inventory_all_levels` and `sea_ports_run_on_their_levels` | ~20–44 s | 52.1 s, 30.0 s |
| `all_levels_smoke::all_levels_load_spawn_and_tick` | ~26 s | 40.0 s |
| `water_levels::managers_run_on_their_levels` | 12.7 s | 21.7 s |

**Look wrong or weak.** No flaky or order-dependent test was seen: every test is headless, the determinism checks pass
run to run, and the merged binaries (all tests in one process under `cargo test`) and nextest (one process per test)
give the same results.
- `breakables_levels::novalis_breaks_every_breakable_kind`: `assert!((4..=7).contains(&got) || got > 0)`. The range
  half is vacuous; only `got > 0` is checked.
- `hero_surfaces::novalis_flow_surface_is_the_sinking_floor`: the doc says the flow class 679 "is not ported". It is,
  since `hero_followups`. The test is still valid, because its harness runs no mobys; only the doc is stale.
- `guards::top_level_holds_only_the_agreed_folders` fails while a user screen recording sits in the repo root. This is
  expected, but it turns every full run red.

**Surveys that assert nothing** (`#[ignore]` since 2026-09-29; run them with `--ignored --nocapture`):
- `creature_classes::survey_pvars`
- `creature_classes::survey_tables`
- `gemlik_generalisation::gemlik_content_gap` (it builds Gemlik's ports, ~1 s)
- `moby_lod::novalis_metal_classes` (rc-engine)

`bolt_crank_novalis::{probe_placement, write_engine_script}` return at once unless their environment variable is set.

## 7. The audit

### 7.1 Method

Each row carries four fields.
- **Pin kind.** What the test holds fixed:
  - **game-checked:** behaviour on real game data, with its address when the test or its doc cites one;
  - **regression:** a bug it once caught, or a refactor guard;
  - **determinism:** a second identical run;
  - **data/format:** a loader, snapshot or table invariant;
  - **exists/resolves:** registry, resolve and "loads" smoke checks;
  - **repo guard;**
  - **port logic:** the port's own code on a synthetic fixture. The module cites the decomp; the test does not;
  - **survey / dev helper:** asserts nothing.

  "+ determinism" marks a behaviour test that also asserts a second identical run (84 tests).
- **Fixture.** In brackets: a level (L01 = Novalis), "all levels", "disc data" or synthetic.
- **Runtime.** Estimated from the code (levels loaded, `LevelPorts` builds, ticks), calibrated on the suite times of
  the 2026-09-29 full run. The bracketed times were measured one binary at a time with
  `RUSTC_BOOTSTRAP=1 <test binary> --test-threads=1 -Z unstable-options --report-time`, which needs no rebuild; this
  was done for `creature_classes`, `cheap_classes_a` and `water_levels`.
  - **fast:** under 1 s.
  - **medium:** 1–10 s. Usually one level's `LevelPorts` (~1.1 s), or a 19-level parse.
  - **slow:** over 10 s. The 19-level `LevelPorts` scans.

  Unit tests are all fast: each crate's lib runs in under 1.5 s.
- **Group.** The merge group of §5. For unit tests it is `lib:<area>`, the area whose module the test lives in.

### 7.2 Counts

| | Before | After |
|---|---|---|
| Tests (`#[test]` fns) | 1188 | 1186 |
| Integration binaries | 70 | 70; 11 after the merge job (§5) |

| Crate | Unit before → after | Integration before → after | Binaries | `#[ignore]` |
|---|---|---|---|---|
| rc-game | 564 → 564 | 286 → 284 | 53 (5 after the merge) | 17 (20 after the merge job: 3 surveys) |
| rc-formats | 147 → 147 | 34 → 34 | 8 | 0 |
| rc-engine | 67 → 67 | 0 | 0 | 0 (1 after the merge job: 1 survey) |
| rc-data | 5 → 5 | 6 → 6 | 2 | 0 |
| rc-extract | 24 → 24 | 5 → 5 | 2 | 3 |
| rc-video | 8 → 8 | 3 → 3 | 1 | 2 |
| tools/trace | 21 → 21 | 5 → 5 | 3 | 0 |
| tools/repo-checks | 0 | 5 → 5 | 1 | 0 |
| tools/xtask | 8 → 8 | 0 | 0 | 0 |

Pin kinds after the drops (1186 tests):

| Pin kind | Tests |
|---|---|
| port logic | 643 |
| game-checked | 370 |
| data/format | 99 |
| exists/resolves | 25 |
| survey (ignored) | 22 |
| determinism | 9 (plus 84 behaviour tests that also assert it) |
| regression | 7 |
| repo guard | 5 |
| survey (no assert) | 4 (ignored since the merge job: 26 ignored in all) |
| dev helper | 2 |

### 7.3 Dropped (2)

- **`gemlik_generalisation::gemlik_class_table_runs_the_body_pieces`.** A duplicate: it built Gemlik's `LevelPorts`
  and asserted that classes 1733–1735 and 1801–1804 run `FxPiece`.
  `level_ports::every_level_runs_the_ports_its_class_table_names` asserts the same seven classes, with the same
  `FxPiece` check, on the same L13 overlay.
- **`hero_surfaces::surface_ids_per_level`.** A superseded survey: it printed `surfaces(n)` for all 19 levels and
  asserted nothing. `hero_surfaces::every_level_handles_its_own_surfaces` runs the same `surfaces(n)` on every level
  and asserts on its result.

### 7.3a Moved out of the suite (4, 2026-09-29)

They read the user's disc image (`RC_ISO`, else the ISO in the personal folder), which no test may do (§10). The same
checks, unchanged, are the dev command `cargo run --release -p rc-trace -- disc-check`:
- `formats golden::disc_matches_extracted_for_every_level` (check 1);
- `ui game_state_novalis::disc_save_game_lump_matches_extracted` (check 2);
- `extract golden::builtin_table_matches_the_disc` (check 3; was `#[ignore]`);
- `extract golden::extract_matches_the_committed_table` (check 4, with `--extract-into <scratch dir>`; was `#[ignore]`).

The tables below are the audit as taken, minus these four rows.

### 7.4 Every test

### rc-game: integration tests (285)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `all_levels_smoke::all_levels_load_spawn_and_tick` | exists/resolves: all levels load spawn and tick (all levels) | slow | world | keep |
| `all_levels_smoke::gemlik_three_thousand_ticks` | exists/resolves: Gemlik Base (13), the validation level (L13) | medium | world | keep |
| `bolt_crank_novalis::probe_placement` | dev helper: probe placement (disc data) | fast | classes | keep; no-op unless `RC_CRANK_PROBE` (dev helper) |
| `bolt_crank_novalis::novalis_crank_opens_the_doors` | game-checked: Holding □ at the bolt latches Ratchet (0x3b, the latch sequence) (L01) | fast | classes | keep |
| `bolt_crank_novalis::novalis_crank_unwinds_when_let_go` | game-checked: Let go early (□ again after 60 ticks), the crank unwinds and the doors close again (L01) | fast | classes | keep |
| `bolt_crank_novalis::novalis_crank_deterministic` | determinism: Two runs are identical tick for tick. (L01) | fast | classes | keep |
| `bolt_crank_novalis::cranks_on_levels_1_4_8` | exists/resolves: The crank registered on every level that has one (1, 4, 8), with the same update. (all levels) | fast | classes | keep |
| `bolt_crank_novalis::write_engine_script` | dev helper: `RC_CRANK_SCRIPT=<file>` (disc data) | fast | classes | keep; no-op unless `RC_CRANK_SCRIPT` (dev helper) |
| `breakables_levels::every_breakable_class_resolves_its_data_on_every_level` | exists/resolves: every breakable class resolves its data on every level (all levels) | slow | classes | keep |
| `breakables_levels::novalis_breaks_every_breakable_kind` | game-checked: novalis breaks every breakable kind (L01) | medium | classes | keep |
| `breakables_levels::every_placed_breakable_breaks_on_its_level` | game-checked: Every level that places a template breakable (all levels) + determinism | slow | classes | keep |
| `cheap_classes_a::units_resolve_on_their_levels` | exists/resolves: Every unit's classes resolve to the unit on its levels, with the census's created counts, and a unit's port runs on… (all levels) | slow (21.9 s) | classes | keep |
| `cheap_classes_a::lamps_rilgar_pulse_and_register_one_glow_per_group` | game-checked: lamps rilgar pulse and register one glow per group (L05) | medium | classes | keep |
| `cheap_classes_a::empty_update_classes_are_updated_not_parked` | game-checked: empty update classes are updated not parked (disc data) | medium | classes | keep |
| `cheap_classes_a::loose_pieces_batalia_rest_then_tumble_fade_and_go` | game-checked: loose pieces batalia rest then tumble fade and go (L08) | medium | classes | keep |
| `cheap_classes_a::conveyors_hoven_carry_their_riders` | game-checked: conveyors hoven carry their riders (L12) | medium | classes | keep |
| `cheap_classes_a::timed_switches_umbris_press_and_time_out` | game-checked: timed switches umbris press and time out (L07) | medium | classes | keep |
| `cheap_classes_a::linked_movers_gemlik_follow_their_link` | game-checked: linked movers gemlik follow their link (L13) | medium | classes | keep |
| `cheap_classes_a::vents_kalebo_blow_trail_blobs` | game-checked: vents kalebo blow trail blobs (L16) | medium | classes | keep |
| `cheap_classes_a::rail_mines_batalia_blow_on_a_hit_or_a_grind` | game-checked: rail mines batalia blow on a hit or a grind (L08) | medium | classes | keep |
| `cheap_classes_a::rising_blocks_rilgar_rise_on_their_link` | game-checked: rising blocks rilgar rise on their link (L05) | medium | classes | keep |
| `cheap_classes_a::bobbing_blocks_kalebo_bob_in_step` | game-checked: bobbing blocks kalebo bob in step (L16) | medium | classes | keep |
| `cheap_classes_a::markers_kerwan_are_deleted_by_their_first_update` | game-checked: markers kerwan are deleted by their first update (L03) | medium | classes | keep |
| `cheap_classes_a::smoke_emitters_batalia_puff_on_their_period` | game-checked: smoke emitters batalia puff on their period (L08) | medium | classes | keep |
| `cheap_classes_a::hydro_pads_rilgar_glow_under_ratchet` | game-checked: hydro pads rilgar glow under ratchet (L05) | medium | classes | keep |
| `cheap_classes_a::sliders_kalebo_slide_to_their_target` | game-checked: sliders kalebo slide to their target (L16) | medium | classes | keep |
| `cheap_classes_b::units_resolve_on_their_levels` | exists/resolves: Every unit's classes resolve to the unit on its levels (and to nothing else), with the census's instance counts (all levels) | slow | classes | keep |
| `cheap_classes_b::asteroids_idle_then_split_on_a_hit` | game-checked: asteroids idle then split on a hit [`0x330000`] (disc data) | medium | classes | keep |
| `cheap_classes_b::chain_links_find_their_neighbours_and_break_in_turn` | game-checked: chain links find their neighbours and break in turn (disc data) | medium | classes | keep |
| `cheap_classes_b::barricade_group_flies_apart_on_a_hit` | game-checked: barricade group flies apart on a hit (disc data) | medium | classes | keep |
| `cheap_classes_b::tethered_platform_breaks_and_sinks_into_the_lava` | game-checked: tethered platform breaks and sinks into the lava [`0x13d3c0`] (disc data) | medium | classes | keep |
| `cheap_classes_b::explosive_tank_takes_hits_then_explodes` | game-checked: explosive tank takes hits then explodes (disc data) | medium | classes | keep |
| `cheap_classes_b::fleet_door_opens_for_ratchet_and_shuts_after` | game-checked: fleet door opens for ratchet and shuts after (L17) | medium | classes | keep |
| `cheap_classes_b::small_units_on_their_levels` | game-checked: small units on their levels (all levels) | medium | classes | keep |
| `cheap_classes_b::frame_cameras` | survey (ignored): The frame cameras (disc data) | ignored | classes | keep |
| `cheap_classes_b::tiny_units_on_their_levels` | game-checked: U559 hides its props, U493 deletes its mobys, U484 blows bubbles, U456 initialises its fields. (all levels) | medium | classes | keep |
| `cheap_classes_b::class_table_survey` | survey (ignored): Diagnostic (all levels) | ignored | classes | keep |
| `common_classes_levels::common_classes_resolve_on_their_levels` | exists/resolves: Every newly registered class resolves to its port on each of its levels (and on no other level's table entry with… (all levels) | slow | classes | keep |
| `common_classes_levels::buried_bolts_novalis` | game-checked: buried bolts novalis (L01) | medium | classes | keep |
| `common_classes_levels::buried_bolts_gemlik` | game-checked: buried bolts gemlik (L13) | medium | classes | keep |
| `common_classes_levels::rc_range_novalis` | game-checked: rc range novalis (L01) | medium | classes | keep |
| `common_classes_levels::rc_range_gemlik` | game-checked: rc range gemlik (L13) | medium | classes | keep |
| `common_classes_levels::activation_zones_rilgar` | game-checked: activation zones rilgar (L05) | medium | classes | keep |
| `common_classes_levels::activation_zones_gemlik` | game-checked: activation zones gemlik (L13) | medium | classes | keep |
| `common_classes_levels::summoner_mouse_novalis` | game-checked: summoner mouse novalis (L01) | medium | classes | keep |
| `common_classes_levels::summoner_mouse_rilgar` | game-checked: summoner mouse rilgar (L05) | medium | classes | keep |
| `common_classes_levels::floor_switches_rilgar` | game-checked: floor switches rilgar (L05) | medium | classes | keep |
| `common_classes_levels::floor_switches_other` | game-checked: floor switches other (disc data) | medium | classes | keep |
| `creature_classes::units_resolve_on_their_levels` | exists/resolves: Every unit's classes resolve to the unit on its levels (and to nothing else), with the census's instance counts. (all levels) | slow (23.5 s) | classes | keep |
| `creature_classes::reaction_tables_resolve` | exists/resolves: The reaction tables reversed on other levels are found by code identity on every level that has them. (all levels) | medium (2.3 s) | classes | keep |
| `creature_classes::survey_pvars` | survey (no assert): The pvars of the created instances of `oc` on `level`, as words (survey). (disc data) | medium | classes | ignored (survey, 2026-09-29); run with `--ignored` |
| `creature_classes::survey_tables` | survey (no assert): The class-table entries (update, reaction table and its six slots) of the unit classes (survey). (disc data) | medium | classes | ignored (survey, 2026-09-29); run with `--ignored` |
| `creature_classes::horny_toads_wander_around_home_on_the_ground` | game-checked: horny toads wander around home on the ground (disc data) | medium (2.6 s) | classes | keep |
| `creature_classes::horny_toad_goes_for_ratchet_and_bites` | game-checked: horny toad goes for ratchet and bites (disc data) | medium | classes | keep |
| `creature_classes::horny_toad_bite_run_is_deterministic` | determinism: horny toad bite run is deterministic (disc data) | medium (2.2 s) | classes | keep |
| `creature_classes::a_wrench_hit_kills_the_horny_toad` | game-checked: a wrench hit kills the horny toad (disc data) | medium | classes | keep |
| `creature_classes::the_taunter_lures_the_horny_toad_into_its_chase` | game-checked: the taunter lures the horny toad into its chase (disc data) | medium | classes | keep |
| `creature_classes::the_suck_cannon_takes_the_horny_toad` | game-checked: the suck cannon takes the horny toad [`0x305260`] (disc data) | medium | classes | keep |
| `creature_classes::a_knocked_horny_toad_presses_its_floor_switch` | game-checked: a knocked horny toad presses its floor switch (disc data) | medium | classes | keep |
| `creature_classes::gunner_hops_between_its_points_and_fires_bursts_at_ratchet` | game-checked: gunner hops between its points and fires bursts at ratchet (disc data) | medium | classes | keep |
| `creature_classes::gunner_run_is_deterministic` | determinism: gunner run is deterministic (disc data) | medium (2.7 s) | classes | keep |
| `creature_classes::gunner_clubs_ratchet_up_close` | game-checked: gunner clubs ratchet up close (disc data) | medium | classes | keep |
| `creature_classes::the_wrench_knocks_the_gunner_back_then_kills_it` | game-checked: the wrench knocks the gunner back then kills it (disc data) | medium | classes | keep |
| `creature_classes::the_morph_ray_keep_byte_and_the_taunter_act_on_the_gunner` | game-checked: the morph ray keep byte and the taunter act on the gunner (disc data) | medium | classes | keep |
| `creatures_enemies_novalis::novalis_dropship_brings_three_troopers` | game-checked: The dropship 666 (instance 687) flies in as soon as Ratchet is in cuboid 27 (the landing pad), carries troopers 309… (L01) + determinism | fast | classes | keep |
| `creatures_enemies_novalis::novalis_trooper_drops_in_fires_and_hurts_ratchet` | game-checked: novalis trooper drops in fires and hurts ratchet (L01) + determinism | fast | classes | keep |
| `creatures_enemies_novalis::novalis_trooper_dies_to_the_wrench` | game-checked: The wrench (L01) + determinism | fast | classes | keep |
| `creatures_enemies_novalis::novalis_trooper_dies_to_the_bomb` | game-checked: The Bomb Glove (L01) + determinism | fast | classes | keep |
| `creatures_enemies_novalis::novalis_gunship_bridge_fly_by` | game-checked: The bridge fly-by [`0x15f404`] (L01) + determinism | fast | classes | keep |
| `creatures_enemies_novalis::novalis_gunship_bombards_the_town` | game-checked: The bombardment (L01) | fast | classes | keep |
| `creatures_novalis::novalis_critters_notice_and_bite_ratchet` | game-checked: novalis critters notice and bite ratchet (L01) + determinism | fast | classes | keep |
| `creatures_novalis::novalis_critters_die_from_the_wrench_and_drop_bolts` | game-checked: novalis critters die from the wrench and drop bolts (L01) + determinism | fast | classes | keep |
| `creatures_novalis::novalis_amoeboids_chase_strike_split_and_die` | game-checked: novalis amoeboids chase strike split and die (L01) + determinism | fast | classes | keep |
| `creatures_novalis::creatures_survey` | survey (ignored): Survey helper (L01) | ignored | classes | keep |
| `cutscene_novalis::arrival_scene_is_asked_for_in_the_first_tick` | game-checked: The mission NPC asks for the arrival scene in the first gameplay tick (counter 1), once, and hides the ship. (disc data) | fast | world | keep |
| `cutscene_novalis::drop_in_cutaway` | game-checked: Drop-in cutaway (disc data) | fast | world | keep |
| `cutscene_novalis::bridge_cutaway_after_the_mission` | game-checked: The door cutaway (disc data) | fast | world | keep |
| `cutscene_novalis::gunship_fly_by_over_the_bridge` | game-checked: Gunship 695 over the bridge (disc data) | fast | world | keep |
| `cutscene_novalis::cutscenes_are_deterministic` | determinism: Two runs of the drop-in and bridge sequence are identical tick for tick. (disc data) | fast | world | keep |
| `cutscene_novalis::survey` | survey (ignored): Survey of the cinematic users placed on Novalis (instances, pvars, cuboids) (L01) | ignored | world | keep |
| `explosion_survey::explosion_classes_survey` | survey (ignored): explosion classes survey (disc data) | ignored | classes | keep |
| `gadgets_novalis::first_arrival_owns_only_the_bomb_glove` | game-checked: A first arrival owns only the Bomb Glove (given by Novalis' level start with its quick-select slot), has it saved in… [`0x13d4c2`] (L01) | fast | ui | keep |
| `gadgets_novalis::gadgets_page_equips_the_packs` | game-checked: The Gadgets page from the pause menu with both packs owned [`0x141414`, `0x242930`] (disc data) | fast | ui | keep |
| `gadgets_novalis::gadgets_page_feet_and_head` | game-checked: Feet and head items through the page [`0x14140c`, `0x141410`] (disc data) | fast | ui | keep |
| `game_state_novalis::first_novalis_arrival` | game-checked: first novalis arrival (L01) | fast | ui | keep |
| `gemlik_generalisation::gemlik_class_table_runs_the_body_pieces` | exists/resolves: Gemlik (13) body pieces 1733–1735, 1801–1804 run FxGroupUpdate (L13) | medium | ui | **drop**: duplicate of `level_ports::every_level_runs_the_ports_its_class_table_names` (same seven classes, same `FxPiece` assert on L13) |
| `gemlik_generalisation::gemlik_pause_menu_is_novalis_tree_at_gemlik_addresses` | game-checked: gemlik pause menu is novalis tree at gemlik addresses [`0x161fe0`] (L01+L13) | medium | ui | keep |
| `gemlik_generalisation::every_level_loads_the_pause_menu` | exists/resolves: every level loads the pause menu (all levels) | medium | ui | keep |
| `gemlik_generalisation::gemlik_content_gap` | survey (no assert): Gemlik's content gap (a queue for later, not a check) (L13) | medium | ui | ignored (survey, 2026-09-29); run with `--ignored` |
| `gold_bolt_infobot_novalis::gold_bolt_pickup_orbit_camera` | game-checked: gold bolt pickup orbit camera (disc data) | medium | classes | keep |
| `gold_bolt_infobot_novalis::gold_bolt_pickup_cuboid_camera` | game-checked: gold bolt pickup cuboid camera (disc data) | fast | classes | keep |
| `gold_bolt_infobot_novalis::gold_bolt_is_deterministic` | determinism: gold bolt is deterministic (disc data) | fast | classes | keep |
| `gold_bolt_infobot_novalis::infobot_bought_from_the_water_pump_worker` | game-checked: infobot bought from the water pump worker (disc data) | fast | classes | keep |
| `gold_bolt_infobot_novalis::novalis_infobot_instance_is_inert` | exists/resolves: Novalis' placed infobot (instance 874) is the scene actors' stand-in (L01) | fast | classes | keep |
| `gold_bolt_infobot_novalis::survey` | survey (ignored): Survey (dev) (all levels) | ignored | classes | keep |
| `gold_bolt_infobot_novalis::registered_on_the_levels_with_identical_code` | exists/resolves: The per-level registry (`LevelPorts`, the level's own class table matched against the level-01 functions) runs the… (all levels) | slow | classes | keep |
| `help_novalis::log_table_on_every_level` | exists/resolves: log table on every level (all levels) | medium | ui | keep |
| `help_novalis::director_look_and_map_hints_in_its_cuboids` | game-checked: director look and map hints in its cuboids (disc data) | fast | ui | keep |
| `help_novalis::infobot_hint_box_on_the_level_text` | game-checked: infobot hint box on the level text (disc data) | fast | ui | keep |
| `help_novalis::print_cuboids` | survey (ignored): Prints the director's cuboid centres (`cargo xtask test-job --test ui --filter help_novalis::print_cuboids --ignored --nocapture`)… (L01) | ignored | ui | keep |
| `hero_boots_grind::spline_follower_on_every_levels_grind_paths` | game-checked: The data the spline follower relies on, on every level (all levels) | fast | hero | keep |
| `hero_boots_grind::oltanis_grind_jump_and_rail_switch` | game-checked: Oltanis (level 14) (L14) | fast | hero | keep |
| `hero_boots_grind::kalebo_long_grind` | game-checked: Kalebo III (level 16) (L16) + determinism | fast | hero | keep |
| `hero_boots_magnet::orxon_magnetic_walkway` | game-checked: On a magnetic walkway with the Magneboots [`0x13f658`] (L10) | fast | hero | keep |
| `hero_cable_kerwan::kerwan_cables_jump_catch_slide_drop` | game-checked: Each cable (L03) | fast | hero | keep |
| `hero_cable_kerwan::kerwan_cable_reach_is_its_own` | game-checked: Kerwan's own path search reaches 1.7 beside a cable, level 4's (level 00's) 0.9 [`0x205830`, `0x20cd08`] (L03) | fast | hero | keep |
| `hero_damage::take_damage_takes_at_most_one` | game-checked: take damage takes at most one (disc data) | fast | hero | keep |
| `hero_damage::hit_knocks_back_and_hurts` | game-checked: A contact hit from an enemy in front (disc data) | fast | hero | keep |
| `hero_damage::hazard_hit_kills_and_raises_the_death_flag` | game-checked: The hazard classes 0x4eb / 0x558 kill at once (0x80, health 0, no push) [`0x141401`] (disc data) | fast | hero | keep |
| `hero_damage::death_at_zero_health` | game-checked: The last health point (disc data) | fast | hero | keep |
| `hero_damage::death_fall_voice_spin_and_flag` | game-checked: Below the level's death height (and more than 2 above the ground) (disc data) | fast | hero | keep |
| `hero_damage::look_stance_enters_and_leaves_on_its_buttons` | game-checked: L1 enters the look stance and keeps it while held (disc data) | fast | hero | keep |
| `hero_damage::walk_to_point` | game-checked: Walk to a point (0x65 → 0x67), the scripted walk the vendors and the ship use. (disc data) | fast | hero | keep |
| `hero_doom::novalis_bots_blow_up_a_critter` | game-checked: Novalis, the Glove of Doom (item 20, class 229) 7 units east of critter group 2 (moby 592), facing it (L01) + determinism | fast | weapons | keep |
| `hero_doom::rilgar_bot_blows_up_an_amoeboid` | game-checked: Rilgar, the small amoeboid 866 (instance 1262) 6 units ahead (L05) + determinism | fast | weapons | keep |
| `hero_doom::novalis_glove_of_doom_clicks_without_a_canister` | game-checked: The Glove of Doom with its canister lost before the trigger (the throw state 0x23 at tick 16) (L01) + determinism | fast | weapons | keep |
| `hero_doom::novalis_bot_blast_breaks_a_crate` | game-checked: Novalis crate 376, the canister thrown from 3 units [`0x26e968`] (L01) + determinism | fast | weapons | keep |
| `hero_followups::flow_chutes_explore` | survey (ignored): Prints where the flow splines of levels 1, 5, 8, 15 run over the sinking floor (`--ignored --nocapture`). (disc data) | ignored | hero | keep |
| `hero_followups::novalis_flow_carries_ratchet_down_the_chute` | game-checked: Novalis' chute (spline 43 of the flow 679) [`0x13f530`] (L01) | fast | hero | keep |
| `hero_followups::flow_chutes_other_levels` | game-checked: The flow on the other levels that have it (Rilgar 5, Blarg 8, Orxon 15 (L05+L06+L10) | fast | hero | keep |
| `hero_followups::novalis_ledge_camera_swings_behind` | game-checked: The camera swings behind a hanging Ratchet (`CamType0HeroStateTweaks` 0x3111d8's ledge branch) [`0x1415d4`, `0x3111d8`] (L01) | fast | hero | keep |
| `hero_followups::moby_ledge_flag_from_the_pvar_record` | game-checked: Probe B's moby ledge flag through `HeroWorld` (disc data) | fast | hero | keep |
| `hero_followups::hero_on_moby_ledge_branch` | game-checked: `HeroOnMoby` 0x277fb8 [`0x13f848`, `0x277fb8`] (disc data) | fast | hero | keep |
| `hero_followups::flow_registered_for_its_levels` | exists/resolves: The flow's registration (all levels) | fast | hero | keep |
| `hero_gameplay_novalis::crate_survey` | survey (ignored): Survey helper (L01) | ignored | weapons | keep |
| `hero_gameplay_novalis::novalis_bolt_crate_pays_bolts` | game-checked: novalis bolt crate pays bolts (L01) + determinism | fast | weapons | keep |
| `hero_gameplay_novalis::novalis_nanotech_crate_heals_up_to_max` | game-checked: novalis nanotech crate heals up to max (L01) + determinism | fast | weapons | keep |
| `hero_gameplay_novalis::novalis_ammo_crate_fills_the_glove_up_to_max` | game-checked: novalis ammo crate fills the glove up to max (L01) + determinism | fast | weapons | keep |
| `hero_gameplay_novalis::novalis_running_throw_uses_the_arm_layer` | game-checked: Running with the Bomb Glove, ○ at tick 70 (L01) + determinism | fast | weapons | keep |
| `hero_gloves::novalis_decoy_lures_critters` | game-checked: Novalis, the Decoy Glove (item 25, class 562) 7 units east of critter group 2 (instance 592), facing it (L01) + determinism | fast | weapons | keep |
| `hero_gloves::rilgar_decoy_lures_an_amoeboid` | game-checked: Rilgar (level 05), a small amoeboid 866 (instance 1262) [`0x274df8`] (L05) + determinism | fast | weapons | keep |
| `hero_gloves::novalis_mine_seeks_a_critter` | game-checked: Novalis, the Mine Glove (item 17, class 190) 7 units east of critter group 2 (L01) + determinism | fast | weapons | keep |
| `hero_gloves::rilgar_mine_takes_an_amoeboid` | game-checked: Rilgar (level 05), a mine thrown at the small amoeboid 866 (instance 1262) 6 units away (L05) + determinism | fast | weapons | keep |
| `hero_gloves::novalis_drones_guard_ratchet` | game-checked: Novalis, the Drone Device (item 24, class 483) given as the vendor gives it (0x141345) [`0x141345`] (L01) + determinism | fast | weapons | keep |
| `hero_gloves::rilgar_drones_strike_an_amoeboid` | game-checked: Rilgar (level 05), the drones launched 4 units from the small amoeboid 866 (instance 1262) (L05) + determinism | fast | weapons | keep |
| `hero_gloves::novalis_mine_breaks_a_crate` | game-checked: Novalis, crates and the mine (the filter sets) (L01) + determinism | fast | weapons | keep |
| `hero_gloves::novalis_taunter_lures_a_mine` | game-checked: Novalis, the Taunter lures a mine (its second loop over the mines' list 0x1b0c30) [`0x1b0c30`] (L01) + determinism | fast | weapons | keep |
| `hero_gloves::novalis_decoy_glove_clicks_without_a_decoy` | game-checked: The throw with the object lost (the shared update's case 3 without one) (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::blaster_given_with_its_ammo_and_defs` | exists/resolves: blaster given with its ammo and defs (disc data) | fast | weapons | keep |
| `hero_guns_novalis::novalis_blaster_standing_breaks_a_crate` | game-checked: Standing at the crate [`0x22e660`] (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_blaster_running_arm_layer` | game-checked: Running (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_blaster_targets_and_hits_a_critter` | game-checked: The critters in the pit (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_blaster_first_person` | game-checked: First person (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::ryno_given_with_its_ammo_and_defs` | exists/resolves: ryno given with its ammo and defs (disc data) | fast | weapons | keep |
| `hero_guns_novalis::novalis_ryno_salvo_hits_a_critter` | game-checked: At the critters (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_ryno_running` | game-checked: Running at the spawn (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_ryno_first_person` | game-checked: First person (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::devastator_given_with_its_ammo_and_defs` | exists/resolves: devastator given with its ammo and defs (disc data) | fast | weapons | keep |
| `hero_guns_novalis::novalis_devastator_locks_and_blows_up_a_critter` | game-checked: At the critters (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_devastator_running_arm_layer` | game-checked: Running (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_devastator_first_person` | game-checked: First person (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::tesla_given_with_its_ammo_and_defs` | exists/resolves: tesla given with its ammo and defs (disc data) | fast | weapons | keep |
| `hero_guns_novalis::novalis_tesla_beam_hits_a_critter` | game-checked: At the critters (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_tesla_beam_stops_at_a_crate` | game-checked: Standing at the crate (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_holding_layers_per_item` | game-checked: Which items hold with which arms (def +0x18 → 0x1413fb) [`0x1413fb`] (L01) | fast | weapons | keep |
| `hero_guns_novalis::novalis_weapon_steady_while_running` | game-checked: The weapon stays level in his hands while he runs [`0x2c7d68`, `0x2ca610`] (L01) | fast | weapons | keep |
| `hero_guns_novalis::novalis_standing_fire_then_run` | game-checked: Firing standing, then pushing the stick while still firing (the user's recording 16.47.33 of the original) [`0x22eca0`, `0x242858`] (L01) + determinism | fast | weapons | keep |
| `hero_guns_novalis::novalis_ryno_salvo_sounds_mix_without_overflow` | regression: Regression (the R.Y.N.O. crash, 2026-09-28) (L01) | fast | weapons | keep |
| `hero_guns_novalis::novalis_swap_releases_the_holding_layers` | game-checked: `UpdateWrenchSelected` 0x2307e0 sets +0x34 on both holding nodes when it commits a swap [`0x1413fb`, `0x2307e0`] (L01) | fast | weapons | keep |
| `hero_guns_novalis::novalis_swap_take_out_sounds` | game-checked: The swap's sounds are the new item's own [`0x22f3c0`, `0x2305e8`] (L01) | fast | weapons | keep |
| `hero_ledge_novalis::novalis_try_script` | survey (ignored): Prints the per-tick trace of the script in `LEDGE_SCRIPT_TRY` (a manual helper, `--ignored --nocapture`). (L01) | ignored | hero | keep |
| `hero_ledge_novalis::novalis_find_a_ledge` | survey (ignored): Search (prints the scripts that reach a hang (L01) | ignored | hero | keep |
| `hero_ledge_novalis::novalis_find_a_ledge_below` | survey (ignored): Two-leg search (`--ignored --nocapture`) (L01) | ignored | hero | keep |
| `hero_ledge_novalis::novalis_ledge_grab_and_climb` | game-checked: novalis ledge grab and climb (L01) | fast | hero | keep |
| `hero_ledge_novalis::novalis_digest_scripts_enter_no_ledge_state` | regression: The scripts of `hero_novalis.rs::novalis_hero_digest` (the refactor guard) with the ledges ported (L01) | fast | hero | keep |
| `hero_ledge_novalis::novalis_scan_ledges` | survey (ignored): Geometric prefilter + simulation (prints ledges within 60 units of the spawn (L01) | ignored | hero | keep |
| `hero_morph::novalis_morphs_a_critter` | game-checked: Novalis, a critter 577 in the pit (L01) + determinism | fast | weapons | keep |
| `hero_morph::novalis_morphs_a_big_amoeboid` | game-checked: Novalis, a big amoeboid 572 (group 22) given level 07's 871 record (health 3, scale 3 (L01+L05) + determinism | fast | weapons | keep |
| `hero_morph::rilgar_morphs_a_small_amoeboid` | game-checked: Rilgar (level 05), a small amoeboid 866 (all levels) + determinism | fast | weapons | keep |
| `hero_morph::novalis_suck_cannon_swallows_a_chicken` | game-checked: Novalis [`0x2e0a28`] (L01) + determinism | fast | weapons | keep |
| `hero_novalis::novalis_idle_run_jump_and_camera` | game-checked: novalis idle run jump and camera (L01) | fast | hero | keep |
| `hero_novalis::novalis_fidget_sequences_match_the_savestates` | game-checked: The two idle fidgets' timing on Ratchet's own sequences against the savestates (docs/plan/trace_results_novalis.md… (L01) | fast | hero | keep |
| `hero_novalis::novalis_idle_fidgets_and_draws` | game-checked: Idle from the load pass on Novalis with the back items (pack 607, Clank 601) (L01) | fast | hero | keep |
| `hero_novalis::novalis_walk_into_the_lake_swim_and_dive` | game-checked: novalis walk into the lake swim and dive (L01) | fast | hero | keep |
| `hero_novalis::novalis_hero_digest` | regression: Refactor guard (docs/plan/hero_states.md "Restructure") [`0x13fc40`, `0x13fc54`] (L01) | fast | hero | keep; the shared-code guard (compare with `RC_HERO_DIGEST` + `RC_HERO_DIGEST_NO_IDLE=1`) |
| `hero_pack_swap_novalis::back_swap_sequence_into_and_out_of_the_lake` | game-checked: Into the lake and back out with the Heli-Pack on the back and the Hydro-Pack owned (disc data) + determinism | fast | hero | keep |
| `hero_pack_swap_novalis::thruster_long_jump_after_images` | game-checked: The Thruster-Pack long jump's after-images (Ratchet's record 0x1409c0) [`0x1409c0`] (disc data) | fast | hero | keep |
| `hero_packs_novalis::novalis_packs_explore` | survey (ignored): Prints the states / heights of a script (`--ignored --nocapture` (L01) | ignored | hero | keep |
| `hero_packs_novalis::novalis_heli_glide` | game-checked: novalis heli glide (L01) | fast | hero | keep |
| `hero_packs_novalis::novalis_long_jumps` | game-checked: novalis long jumps (L01) | fast | hero | keep |
| `hero_packs_novalis::novalis_thruster_stomp` | game-checked: novalis thruster stomp (L01) | fast | hero | keep |
| `hero_packs_novalis::novalis_hydro_pack_on_the_back_in_the_lake` | game-checked: The lake with the Hydro-Pack owned (L01) | fast | hero | keep |
| `hero_packs_novalis::novalis_packs_deterministic` | determinism: Two identical runs give identical records (no hidden state across runs). (L01) | fast | hero | keep |
| `hero_platform_novalis::novalis_lift_carries_ratchet_down` | regression: The lift carries Ratchet down the cliff (L01) | fast | hero | keep |
| `hero_platform_novalis::novalis_lift_is_deterministic` | determinism: Determinism (L01) | fast | hero | keep |
| `hero_platform_novalis::novalis_steep_bank_holds_him_at_its_foot` | game-checked: Off the lift at the bottom, walking into the meadow's bank (faces of 55°–68° at (178..181, 157, 42)) (L01) | fast | hero | keep |
| `hero_pyrocitor_novalis::the_purchase_puts_the_pyrocitor_in_hand_with_ammo` | game-checked: the purchase puts the pyrocitor in hand with ammo (disc data) | fast | weapons | keep |
| `hero_pyrocitor_novalis::novalis_pyrocitor_standing` | game-checked: novalis pyrocitor standing (L01) + determinism | fast | weapons | keep |
| `hero_pyrocitor_novalis::novalis_pyrocitor_running_breaks_a_crate` | game-checked: novalis pyrocitor running breaks a crate (L01) + determinism | fast | weapons | keep |
| `hero_pyrocitor_novalis::novalis_pyrocitor_first_person` | game-checked: novalis pyrocitor first person (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_suck_cannon_pulls_and_fires_a_critter` | game-checked: Novalis, the critters 577 in the pit [`0x2efc60`, `0x302bd0`] (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::rilgar_suck_cannon_takes_a_small_amoeboid` | game-checked: Rilgar (level 05), a small amoeboid 866 awake by the river (L05) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_taunter_lures_the_critters` | game-checked: Novalis, the critters in the pit (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_taunter_knocks_a_crate` | game-checked: The crates below the spawn plateau (the table's mobys 375, 377, 376 (class 500) and 533 (501)), all within 12 in… (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_suck_cannon_holds_continuously` | game-checked: Novalis, the pit (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::rilgar_suck_cannon_far_reach_and_two_in_sequence` | game-checked: Rilgar, two small amoeboids in a line in front of Ratchet (L05) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_taunter_lures_for_the_whole_hold` | game-checked: The Taunter held for 240 ticks (the stand-in sound layer plays each whistle for 90) (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_suck_cannon_leaves_crates` | game-checked: The pull's filter (`0x302bd0` [`0x2732b8`, `0x302bd0`] (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_suck_cannon_fired_critter_breaks_a_crate` | game-checked: What the Suck Cannon does to a crate in the game (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_bolt_pickup_sound_on_a_normal_pickup` | game-checked: The normal pickup plays the same sound through the same code (`FUN_002bcb90` from the bolt's idle, in the moby loop) (L01) + determinism | fast | weapons | keep |
| `hero_reactive_novalis::novalis_suck_cannon_vacuums_an_ammo_pickup` | game-checked: The ammo crate 552 (class 511) broken next to Ratchet drops Bomb Glove ammo (he holds 10 of 40) [`0x2db850`, `0x307a50`] (L01) + determinism | fast | weapons | keep |
| `hero_surfaces::surface_ids_per_level` | survey (no assert): prints each level's world / moby surface ids (all levels) | medium | hero | **drop**: superseded; `hero_surfaces::every_level_handles_its_own_surfaces` runs the same `surfaces(n)` on every level and asserts on it |
| `hero_surfaces::every_level_handles_its_own_surfaces` | game-checked: Every surface id a level's collision uses is handled by that level's own reaction, or by no level's (0xa, 0x1f (all levels) | medium | hero | keep |
| `hero_surfaces::aridia_quicksand_sink_jump_out_and_fade` | game-checked: Aridia (level 2) quicksand, surface 3: the sink rates and the fade 1.2 deep, with Aridia's capsule flags 0x324 (L02) | fast | hero | keep |
| `hero_surfaces::deadly_liquid_pass_levels_have_no_such_faces` | data/format: levels 6 and 14 (pass surface 0xd) have no surface-0xd face [`0x22a1d0`, `0x224db0`] (L06, L14) | fast | hero | keep |
| `hero_surfaces::slippery_floor_is_the_slide` | game-checked: A slippery floor (surface 7, levels 12 and 14) (L01) | fast | hero | keep |
| `hero_surfaces::novalis_flow_surface_is_the_sinking_floor` | game-checked: Novalis' flow chutes (surface 4 [`0x13f530`] (L01) | fast | hero | keep; doc says the flow 679 "is not ported" — stale since `hero_followups` ported it |
| `hero_swingshot_levels::pull_on_real_targets` | game-checked: On Aridia and Kerwan (L02+L03) + determinism | medium | hero | keep |
| `hero_swingshot_levels::swing_on_real_targets` | game-checked: On Aridia and Kerwan (L02+L03) + determinism | medium | hero | keep |
| `hero_targeting_novalis::targeting_survey` | survey (ignored): Survey (L01) | ignored | weapons | keep |
| `hero_targeting_novalis::novalis_reticle_snaps_to_a_critter_and_the_bomb_lands_on_it` | game-checked: The glove's search picks a critter (0x13fda0 [`0x13fda0`] (L01) + determinism | fast | weapons | keep |
| `hero_targeting_novalis::critter_ground_survey` | survey (ignored): Survey (disc data) | ignored | weapons | keep |
| `hero_visibomb::visibomb_is_item_13_with_its_classes` | exists/resolves: The item table 0x179f40 gives item 13 the gun class 163 [`0x179f40`, `0x2cbda8`] (disc data) | fast | weapons | keep |
| `hero_visibomb::novalis_launch` | game-checked: Standing at the spawn [`0x15f30c`, `0x15f608`] (L01) + determinism | fast | weapons | keep |
| `hero_visibomb::novalis_explosion_breaks_a_crate` | game-checked: Launched 8 units from crate 376 and dived onto it [`0x2cb968`, `0x317e70`] (L01) + determinism | fast | weapons | keep |
| `hero_visibomb::novalis_range_limit_ends_the_flight` | game-checked: Pulled up with ✕ held from the spawn [`0x2cb788`] (L01) + determinism | fast | weapons | keep |
| `hero_visibomb::novalis_steering_and_circle` | game-checked: The stick steers from the 6th tick of the flight (L01) + determinism | fast | weapons | keep |
| `hero_visibomb::novalis_explosion_hits_a_critter` | game-checked: At a critter 577 in the pit (L01) + determinism | fast | weapons | keep |
| `hero_visibomb::rilgar_launch_detonate_and_hand_back` | game-checked: On Rilgar (level 5 [`0x16cc30`] (L05) + determinism | fast | weapons | keep |
| `hero_visibomb::novalis_holding_layers_and_first_person_launch` | game-checked: Held, the Visibomb's def +0x18 = 2 makes both glove-holding layers (lists 12 and 13) [`0x167240`] (L01) + determinism | fast | weapons | keep |
| `hero_weapons4::novalis_walloper_lunges_into_an_amoeboid` | game-checked: Novalis (L01) + determinism | fast | weapons | keep |
| `hero_weapons4::novalis_walloper_lunge_timing_in_the_open` | game-checked: Novalis (L01) + determinism | fast | weapons | keep |
| `hero_weapons4::novalis_walloper_breaks_a_crate` | game-checked: Novalis (L01) + determinism | fast | weapons | keep |
| `hero_weapons4::rilgar_walloper_hits_a_small_amoeboid` | game-checked: Rilgar (level 05) (L05) + determinism | fast | weapons | keep |
| `hero_weapons4::novalis_ryno_state_3` | game-checked: The R.Y.N.O.'s update state 3 (0x2e5264 [`0x2c4c20`, `0x2e5264`] (L01) + determinism | fast | weapons | keep |
| `hero_weapons4::novalis_ryno_state_3_released` | game-checked: State 3 without L1 / L2 held goes back to state 2 at once (the crosshair drawn that tick). (L01) | fast | weapons | keep |
| `hero_weapons4::novalis_flyer_kill_blast` | regression: The Blarg flyer 660's kill (a hit 0x800000 (L01+L06) + determinism | fast | weapons | keep |
| `hero_weapons4::novalis_pyrocitor_burn_puffs` | game-checked: The Pyrocitor's hits are the resolver's type 5 (the burning kind 4) [`0x271258`] (L01) + determinism | fast | weapons | keep |
| `hero_weapons_novalis::weapons_survey` | survey (ignored): Survey helper (L01) | ignored | weapons | keep |
| `hero_weapons_novalis::novalis_comet_strike_breaks_a_crate_and_the_wrench_returns` | game-checked: novalis comet strike breaks a crate and the wrench returns (L01) + determinism | fast | weapons | keep |
| `hero_weapons_novalis::novalis_bomb_breaks_a_crate` | game-checked: novalis bomb breaks a crate (L01) + determinism | fast | weapons | keep |
| `hero_weapons_novalis::novalis_first_person_enter_aim_and_exit` | game-checked: novalis first person enter aim and exit [`0x1413f5`] (L01) + determinism | fast | weapons | keep |
| `hero_weapons_novalis::novalis_first_person_comet_strike` | game-checked: novalis first person comet strike (L01) | fast | weapons | keep |
| `interaction_vendor::novalis_vendor_prompt_open_buy_close` | game-checked: novalis vendor prompt open buy close [`0x2ba9c0`] (L01) | fast | ui | keep |
| `interaction_vendor::novalis_vendor_cannot_afford` | game-checked: novalis vendor cannot afford (L01) | fast | ui | keep |
| `interaction_vendor::novalis_vendor_is_deterministic` | determinism: Determinism (L01) | fast | ui | keep |
| `level_ports::novalis_ports_are_the_class_number_registry` | exists/resolves: novalis ports are the class number registry (L01) | medium | world | keep |
| `level_ports::every_level_runs_the_ports_its_class_table_names` | exists/resolves: every level runs the ports its class table names [`0x2bd100`] (all levels) | slow | world | keep |
| `map_levels::map_files_parse` | exists/resolves: Every map file parses (disc data) | medium | ui | keep |
| `map_levels::tables_on_every_level` | game-checked: The overlay tables (transforms 0x182c90, zones 0x183020, pans 0x184370, the predicate slots 0x184410) are the same… [`0x182c90`, `0x183020`] (all levels) | medium | ui | keep |
| `map_levels::fog_writer_reveals_the_brush` | game-checked: `FUN_0025c4f8` on Novalis (L01) | fast | ui | keep |
| `map_levels::zone_rules` | game-checked: The zone rules (L01) | fast | ui | keep |
| `map_levels::pack_and_unpack` | game-checked: The saved mask (disc data) | fast | ui | keep |
| `map_levels::map_page_on_novalis` | game-checked: The map page on Novalis [`0x1b3998`] (L01) | fast | ui | keep |
| `moby_collision_novalis::novalis_line_onto_crate_344` | game-checked: novalis line onto crate 344 (L01) | fast | world | keep |
| `moby_collision_novalis::novalis_bolt_settles_on_a_crate` | game-checked: novalis bolt settles on a crate (L01) | fast | world | keep |
| `moby_update_novalis::novalis_scheduler_300_ticks` | game-checked: novalis scheduler 300 ticks (L01) | fast | classes | keep |
| `novalis_collision::novalis_vertical_rays_find_ground` | game-checked: novalis vertical rays find ground (L01) | fast | world | keep |
| `novalis_collision::novalis_line_cell_walks_are_connected` | game-checked: Cell walks of long rays stay consistent (L01) | fast | world | keep |
| `novalis_world::novalis_fire_fields_init_like_the_game` | game-checked: novalis fire fields init like the game [`0x161390`, `0x2ba658`] (L01) | fast | world | keep |
| `novalis_world::novalis_fire_field_flames_cycle_every_j_plus_3_seconds` | game-checked: novalis fire field flames cycle every j plus 3 seconds (L01) | fast | world | keep |
| `novalis_world::fire_fields_on_levels_0_and_14_init` | game-checked: fire fields on levels 0 and 14 init (disc data) | fast | world | keep |
| `novalis_world::novalis_waterfall_foam_spawns_rings_and_mist` | game-checked: novalis waterfall foam spawns rings and mist (L01) + determinism | fast | world | keep |
| `novalis_world::novalis_props_move_like_the_game` | game-checked: novalis props move like the game (L01) + determinism | fast | world | keep |
| `novalis_world::novalis_breakables_burst_into_rock_bits` | game-checked: novalis breakables burst into rock bits (L01) | fast | world | keep |
| `novalis_world::novalis_collapsing_platform_falls_once_ratchet_is_on_it` | game-checked: novalis collapsing platform falls once ratchet is on it (L01) | fast | world | keep |
| `path_classes::units_resolve_on_their_levels` | exists/resolves: Every unit's classes resolve to the unit on its levels (and to nothing else), with the census's instance counts. (all levels) | slow | classes | keep |
| `path_classes::gliders_ride_their_paths_above_the_ground_and_flap` | game-checked: gliders ride their paths above the ground and flap (disc data) | medium | classes | keep |
| `path_classes::rail_car_runs_its_path_when_ratchet_grinds_in_its_cuboid` | game-checked: rail car runs its path when ratchet grinds in its cuboid (disc data) | medium | classes | keep |
| `path_classes::air_traffic_groups_spread_along_their_paths_and_keep_their_spacing` | game-checked: air traffic groups spread along their paths and keep their spacing (disc data) | medium | classes | keep |
| `pause_pages_novalis::quick_select_assigns_slots_and_writes_them_back` | game-checked: Quick Select (0x2901a8 / 0x2903d0 / 0x290468) [`0x141ea0`, `0x2901a8`] (disc data) | fast | ui | keep |
| `pause_pages_novalis::weapons_page_ammo_text_and_model` | game-checked: The Weapons page [`0x2919a0`, `0x292b60`] (disc data) | fast | ui | keep |
| `pause_pages_novalis::items_page_gold_bolts_and_picture` | game-checked: The Items page (disc data) | fast | ui | keep |
| `pause_pages_novalis::help_page_streamed_image_states` | game-checked: The Help page's streamed picture (0x2937d0 / 0x293d50) through its buffer states [`0x2937d0`, `0x293d50`] (disc data) | fast | ui | keep |
| `pause_pages_novalis::help_log_page_lists_the_log_and_shows_the_message` | game-checked: Help / Help Log [`0x290b70`, `0x290c00`] (disc data) | fast | ui | keep |
| `pause_pages_novalis::help_controls_page_two_pictures` | game-checked: Help / Controls (0x294050 / 0x294198) [`0x294050`, `0x294198`] (disc data) | fast | ui | keep |
| `pause_pages_novalis::help_moves_table_follows_the_heli_pack` | game-checked: Help / Moves (0x2904a0) [`0x2904a0`] (disc data) | fast | ui | keep |
| `pause_pages_novalis::help_weapons_icon_list` | game-checked: Help / Weapons (0x295000, icon list 0x28d818) [`0x28d818`, `0x295000`] (disc data) | fast | ui | keep |
| `pause_pages_novalis::goodies_skill_points_and_movies` | game-checked: Goodies [`0x1b8aa8`, `0x2917d8`] (disc data) | fast | ui | keep |
| `reverb_conformance::reverb_boxes_on_every_level` | game-checked: reverb boxes on every level (all levels) | medium | ui | keep |
| `reverb_conformance::reverb_in_the_mix_on_every_level` | game-checked: reverb in the mix on every level (all levels) | fast | ui | keep |
| `reverb_conformance::env_points_on_every_level` | game-checked: env points on every level (all levels) | fast | ui | keep |
| `reverb_conformance::level_sounds_on_every_level` | game-checked: level sounds on every level (all levels) | medium | ui | keep |
| `reverb_conformance::level_sound_call_sites_in_every_overlay` | game-checked: level sound call sites in every overlay (all levels) | fast | ui | keep |
| `sea_levels::sea_inventory_all_levels` | game-checked: sea inventory all levels (all levels) | slow | world | keep |
| `sea_levels::sea_ports_run_on_their_levels` | game-checked: sea ports run on their levels (all levels) | slow | world | keep |
| `sea_levels::pokitaru_ocean_needs_the_camera_above_its_floor` | game-checked: The ocean does not register while the camera is at or below pvar 8 (level 11 (L11) | medium | world | keep |
| `shadow_volume_novalis::ratchet_posed_list_matches_the_game` | game-checked: ratchet posed list matches the game (disc data) | fast | world | keep |
| `shadow_volume_novalis::directions_and_slab_match_the_game` | game-checked: Directions 0 and 1 from Ratchet's light (light word 0 [`0x1af000`, `0x1af010`] (disc data) | fast | world | keep |
| `sound_conformance::every_animation_sound_and_footstep_resolves_and_plays` | exists/resolves: every animation sound and footstep resolves and plays [`0x15f5f0`] (all levels) | medium | ui | keep |
| `sound_conformance::footstep_constants_in_every_overlay` | exists/resolves: The footsteps' code constants [`0x15f574`] (all levels) | fast | ui | keep |
| `water_levels::water_inventory_all_levels` | game-checked: water inventory all levels (all levels) | slow (167 s before the merge job) | world | keep; `water_data` is now cached per level (§5) |
| `water_levels::managers_run_on_their_levels` | game-checked: Runs every level with a patch manager for 400 ticks with the camera looking at the manager's patch 0 (all levels) | slow (12.7 s) | world | keep |
| `water_levels::novalis_751_port_is_the_ripple_module` | game-checked: The 751 port against the ripple module alone on the same stream (L01) | fast (0.9 s) | world | keep |
| `water_levels::rilgar_swim_and_wade` | game-checked: Rilgar (05) (L05) | medium (7.4 s) | world | keep |
| `water_levels::pokitaru_swim_and_wade` | game-checked: Pokitaru (11) (L11) | medium (7.0 s) | world | keep |
| `water_levels::pokitaru_tide_pool_swim` | game-checked: Pokitaru's tide pools [`0x26e618`] (L11) | medium (4.0 s) | world | keep |

### rc-game: unit tests (564)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `afterimage::thruster_trail_places_and_fades` | port logic: The Thruster long jump's trail | fast | lib:hero | keep |
| `afterimage::start_is_once_and_kill_ends` | game-checked: `0x277400` on an active record keeps it [`0x277400`, `0x277740`] | fast | lib:hero | keep |
| `audio::gauss_rows_sum_to_unity` | port logic: gauss rows sum to unity | fast | lib:audio | keep |
| `audio::adsr_phases` | port logic: adsr phases | fast | lib:audio | keep |
| `audio::loud_mix_clamps_without_overflow` | regression: Regression (the R.Y.N.O. crash, 2026-09-28) | fast | lib:audio | keep |
| `audio::voice_plays_one_shots_and_loops` | game-checked: voice plays one shots and loops | fast | lib:audio | keep |
| `audio::gadget_class_sounds_on_every_level` | data/format: Every level's gadget classes (the hand items) get their class sound defs from their blobs with the parked remap (all levels) | fast | lib:audio | keep |
| `audio::novalis_start_renders_music_and_ambience` | port logic: Novalis from `extracted/` (skipped when absent) (L01) + determinism | fast | lib:audio | keep |
| `audio::stream_queue_is_seamless` | game-checked: The Start → Loop hand-over is sample-exact | fast | lib:audio | keep |
| `audio::wav_header` | port logic: wav header | fast | lib:audio | keep |
| `audio/class_sounds::trigger_fires_once_in_its_window` | port logic: trigger fires once in its window | fast | lib:audio | keep |
| `audio/grain_vm::lfo_sine_table` | port logic: lfo sine table | fast | lib:audio | keep |
| `audio/grain_vm::tone_pitches` | port logic: tone pitches | fast | lib:audio | keep |
| `audio/music::level_start_queues_the_loop` | port logic: level start queues the loop | fast | lib:audio | keep |
| `audio/music::requests_debounce_and_transition` | port logic: requests debounce and transition | fast | lib:audio | keep |
| `audio/music::body_behind_an_ended_stream_fails` | port logic: Queueing the body behind a stream that already ended fails (0) instead of starting a second stream the player never… | fast | lib:audio | keep |
| `audio/reverb::box_ramps_and_exits` | port logic: box ramps and exits | fast | lib:audio | keep |
| `audio/reverb::dirty_priorities_and_checkpoint` | port logic: dirty priorities and checkpoint | fast | lib:audio | keep |
| `audio/reverb::env_point_radius` | port logic: env point radius | fast | lib:audio | keep |
| `audio/reverb::presets_decay_at_their_rt60` | port logic: The decay | fast | lib:audio | keep |
| `audio/reverb::off_is_silent_and_glide_is_linear` | port logic: off is silent and glide is linear | fast | lib:audio | keep |
| `audio/scene::inbox_keeps_order` | port logic: inbox keeps order | fast | lib:audio | keep |
| `audio/scene::scene_commands_keep_one_music_instance` | port logic: The scene command path on Novalis (extracted data (L01) | fast | lib:audio | keep |
| `audio/scene::menu_open_holds_music_and_world_sounds_and_plays_menu_sounds` | game-checked: The page menu's audio (`EnterMenuMode` 0x28bf50 / `PageMenuUpdate` 0x28c990's close) on Novalis [`0x28bf50`, `0x28c990`] (L01) | fast | lib:audio | keep |
| `audio/voices::distance_law` | port logic: distance law | fast | lib:audio | keep |
| `audio/voices::pan_directions` | port logic: pan directions | fast | lib:audio | keep |
| `audio/voices::volumes_and_pan_table` | port logic: volumes and pan table | fast | lib:audio | keep |
| `audio/voices::slot_rules` | port logic: slot rules | fast | lib:audio | keep |
| `audio/voices::timer` | port logic: timer | fast | lib:audio | keep |
| `cinematic::gunship_requests_map_to_the_game_calls` | game-checked: gunship requests map to the game calls | fast | lib:world | keep |
| `collision_query/tests::line_hit_and_miss` | port logic: line hit and miss | fast | lib:world | keep |
| `collision_query/tests::one_sided_and_two_sided` | port logic: one sided and two sided | fast | lib:world | keep |
| `collision_query/tests::exclusion_flags` | port logic: exclusion flags | fast | lib:world | keep |
| `collision_query/tests::quad_split_is_v0v1v2_then_v0v2v3` | port logic: quad split is v0v1v2 then v0v2v3 | fast | lib:world | keep |
| `collision_query/tests::edge_tests_are_inclusive` | port logic: edge tests are inclusive | fast | lib:world | keep |
| `collision_query/tests::nearest_hit_is_strictly_closer_across_cells` | port logic: nearest hit is strictly closer across cells | fast | lib:world | keep |
| `collision_query/tests::walk_stops_once_the_next_cell_starts_behind_the_hit` | port logic: walk stops once the next cell starts behind the hit | fast | lib:world | keep |
| `collision_query/tests::cell_walk_order_on_a_diagonal` | port logic: cell walk order on a diagonal | fast | lib:world | keep |
| `collision_query/tests::sphere_hit_and_push_out` | port logic: sphere hit and push out | fast | lib:world | keep |
| `collision_query/tests::sphere_takes_the_closest_face` | port logic: sphere takes the closest face | fast | lib:world | keep |
| `collision_query/tests::capsule_floor_wall_and_height` | port logic: capsule floor wall and height | fast | lib:world | keep |
| `collision_query/tests::moby_grid_registration` | port logic: moby grid registration | fast | lib:world | keep |
| `collision_query/tests::moby_mesh_line_hits_top_face_after_the_world` | port logic: moby mesh line hits top face after the world | fast | lib:world | keep |
| `collision_query/tests::moby_mesh_blocks_sphere_and_capsule` | port logic: moby mesh blocks sphere and capsule | fast | lib:world | keep |
| `collision_query/tests::moby_sphere_primitive_line_sphere_capsule` | port logic: moby sphere primitive line sphere capsule | fast | lib:world | keep |
| `collision_query/tests::moby_joint_capsule_and_pose_cache` | port logic: moby joint capsule and pose cache | fast | lib:world | keep |
| `collision_query/tests::hero_groups_sphere` | port logic: hero groups sphere | fast | lib:world | keep |
| `fog_zones::t_at_the_cuboid_faces` | port logic: t at the cuboid faces | fast | lib:world | keep |
| `fog_zones::apply_and_no_restore` | port logic: apply and no restore | fast | lib:world | keep |
| `fog_zones::hero_light_crossfade` | port logic: hero light crossfade | fast | lib:world | keep |
| `fog_zones::underwater_hysteresis` | port logic: underwater hysteresis | fast | lib:world | keep |
| `fog_zones::update_fog_selects_the_alternate_set` | game-checked: update fog selects the alternate set [`0x1f4000`] | fast | lib:world | keep |
| `fog_zones::novalis_zone_3_step` | game-checked: Novalis disc (L01) | fast | lib:world | keep |
| `follow_camera::asin_and_rot` | port logic: asin and rot | fast | lib:hero | keep |
| `follow_camera::first_person_enter_turn_exit` | game-checked: The look stance (camera mode 4) [`0x1413f5`] | fast | lib:hero | keep |
| `follow_camera::first_person_cut_when_far_off` | port logic: Leaving within the blend-in with the views 80° apart cuts back without a blend. | fast | lib:hero | keep |
| `follow_camera::init_snap_behind_the_hero` | game-checked: Init snap 0x311890 behind a hero facing +x [`0x311890`] | fast | lib:hero | keep |
| `follow_camera::right_stick_yaw` | port logic: Standing still, the camera stays put | fast | lib:hero | keep |
| `follow_camera::shake_envelope` | port logic: The stomp's shake (0.2 along up for 40 ticks) | fast | lib:hero | keep |
| `follow_camera/script::euler_rows_forward_is_yaw_pitch` | port logic: euler rows forward is yaw pitch | fast | lib:hero | keep |
| `follow_camera/script::spring_mode_converges_and_tracks_a_moving_target` | port logic: spring mode converges and tracks a moving target | fast | lib:hero | keep |
| `follow_camera/script::orbit_mode_swings_about_the_hero_and_holds_the_targets` | port logic: orbit mode swings about the hero and holds the targets | fast | lib:hero | keep |
| `follow_camera/script::lerp_mode_lands_on_the_target_when_the_timer_runs_out` | port logic: lerp mode lands on the target when the timer runs out | fast | lib:hero | keep |
| `follow_camera/type6::switch_places_the_camera_behind_the_target` | port logic: The switch places the camera on the missile (0.01 behind the centre, 15° up) looking along its yaw, pitched down 20°… | fast | lib:hero | keep |
| `follow_camera/type6::tracking_follows_a_turning_target` | port logic: In flight the yaw follows the target's yaw on the 0.07 spring and the pitch offset eases 5 % a tick toward its pitch | fast | lib:hero | keep |
| `follow_camera/type6::orbit_after_the_flight` | port logic: The first call after the flight re-places the camera 6 from the centre (elevation = the pitch offset, no pitch… | fast | lib:hero | keep |
| `follow_camera/type6::hand_back_cuts_when_ratchet_is_far_or_off_the_view` | port logic: hand back cuts when ratchet is far or off the view | fast | lib:hero | keep |
| `game_state::typed_raw_and_pads_round_trip` | port logic: typed raw and pads round trip | fast | lib:ui | keep |
| `game_state::restore_rules` | port logic: restore rules | fast | lib:ui | keep |
| `game_state::transition_rules` | port logic: transition rules | fast | lib:ui | keep |
| `game_state::scale_ticks_ntsc_is_identity` | port logic: scale ticks ntsc is identity | fast | lib:ui | keep |
| `help::request_gate_and_log` | port logic: request gate and log | fast | lib:ui | keep |
| `help::log_moves_a_repeat_to_the_end` | port logic: log moves a repeat to the end | fast | lib:ui | keep |
| `help::states_timings_and_the_record_bump_on_close` | port logic: states timings and the record bump on close | fast | lib:ui | keep |
| `help::triangle_skips_with_a_bump_and_prompt_is_forced` | port logic: triangle skips with a bump and prompt is forced | fast | lib:ui | keep |
| `help::first_input_gate_and_mode` | port logic: first input gate and mode | fast | lib:ui | keep |
| `help::voice_request_wait_play_and_stop` | game-checked: voice request wait play and stop [`0x1516ec`] | fast | lib:ui | keep |
| `help::suspend_closes_and_reopens_after_resume` | game-checked: suspend closes and reopens after resume [`0x1798ca`] | fast | lib:ui | keep |
| `help::hero_hints_rules` | port logic: hero hints rules | fast | lib:ui | keep |
| `hero::idle_on_flat_ground_stays_put` | port logic: idle on flat ground stays put | fast | lib:hero | keep |
| `hero::run_ramp_and_brake` | port logic: run ramp and brake | fast | lib:hero | keep |
| `hero::jump_heights_match_the_model` | port logic: The §4.4 model numbers (T counted from the first tick in the jump state | fast | lib:hero | keep |
| `hero::fall_speed_is_capped_by_the_move_clamp` | port logic: Falling | fast | lib:hero | keep |
| `hero::idle_momentum_decays_in_28_ticks` | port logic: idle momentum decays in 28 ticks | fast | lib:hero | keep |
| `hero::double_jump_window` | port logic: double jump window | fast | lib:hero | keep |
| `hero::jump_buffer_in_idle_is_7_ticks` | port logic: jump buffer in idle is 7 ticks | fast | lib:hero | keep |
| `hero::unported_state_freezes` | port logic: unported state freezes | fast | lib:hero | keep |
| `hero::crouch_and_flip` | port logic: crouch and flip | fast | lib:hero | keep |
| `hero::jump_from_rest` | port logic: jump from rest | fast | lib:hero | keep |
| `hero/anim::loop_exit_jumps_to_the_key` | game-checked: The loop 6..21 holds key B inside the range [`0x13fe04`, `0x247d18`] | fast | lib:hero | keep |
| `hero/blaster::search_rules` | game-checked: 0x2ca310 [`0x2ca310`] | fast | lib:weapons | keep |
| `hero/blaster::shot_aims_at_the_target` | port logic: The shot's direction | fast | lib:weapons | keep |
| `hero/boots/tests::gravity_mode_rule_is_the_games` | game-checked: gravity mode rule is the games | fast | lib:hero | keep |
| `hero/boots/tests::grind_along_a_rail` | port logic: No rail contact without the Grind Boots or off a rail level | fast | lib:hero | keep |
| `hero/boots/tests::grind_slopes` | port logic: Downhill the slope term adds speed (and the downhill lean anim), uphill it takes it away. | fast | lib:hero | keep |
| `hero/boots/tests::grind_jump_lands_on_the_rail` | port logic: ✕ on the rail | fast | lib:hero | keep |
| `hero/boots/tests::rail_switch_to_the_side` | port logic: ✕ with the stick to the side | fast | lib:hero | keep |
| `hero/boots/tests::off_the_end_of_the_rail` | port logic: The end of an open rail | fast | lib:hero | keep |
| `hero/boots/tests::grind_wrench` | port logic: □: the grind wrench 0x2b (anim 0x4e, the swing's hit sphere queued) and back to 0x28 after the row's idle frame. | fast | lib:hero | keep |
| `hero/boots/tests::grind_hurt` | port logic: A hit on the rail | fast | lib:hero | keep |
| `hero/boots/tests::magneboots_walk_and_hop` | game-checked: The Magneboots on a magnetic floor (surface 2, Orxon's rules) [`0x13f658`] (L10) | fast | lib:hero | keep |
| `hero/boots/tests::magneboots_hold_a_steep_floor` | port logic: A 60° magnetic slope | fast | lib:hero | keep |
| `hero/boots/tests::cable_slide` | port logic: The cable 0x74 (a cable level) | fast | lib:hero | keep |
| `hero/boots/tests::a_rising_jump_catches_the_cable` | port logic: A rising jump catches a cable whose hands come within 0.7 of it (Kerwan's platform (L03) | fast | lib:hero | keep |
| `hero/boots/tests::square_is_the_wrench_grab_not_the_jump_attack` | game-checked: □ pressed at the cable reaches 1.2 below it instead of 0.7 (0x13cae4 & 0x80 in `0x20d330`), and the weapon check… [`0x13cae4`, `0x20d330`] | fast | lib:hero | keep |
| `hero/boots/tests::ride_to_the_end_no_jump_off` | game-checked: On the cable [`0x13f534`] | fast | lib:hero | keep |
| `hero/comet::crouch_square_throws_at_frame_33` | port logic: Crouch + □ with the wrench | fast | lib:weapons | keep |
| `hero/comet::flight_out_and_back` | port logic: The flight | fast | lib:weapons | keep |
| `hero/comet::first_person_orientation_and_spin_by_pitch` | game-checked: The first-person throw's orientation and spin at 0°, 30°, 60° and 84° up, against the game's formula [`0x167254`, `0x2721f0`] | fast | lib:weapons | keep |
| `hero/comet::world_z_step_differs_when_pitched` | game-checked: The Euler-z step the port used before (a spin about the world z axis) matches the game only while the wrench is level | fast | lib:weapons | keep |
| `hero/comet::look_stance_throw` | port logic: The look stance after 20 ticks | fast | lib:weapons | keep |
| `hero/crank::latch_conditions` | game-checked: The latch | fast | lib:hero | keep |
| `hero/crank::release_after_60_ticks` | port logic: Release | fast | lib:hero | keep |
| `hero/crank::entry_state` | port logic: The entry | fast | lib:hero | keep |
| `hero/crank::turning_speed_ring_and_anims` | port logic: Turning | fast | lib:hero | keep |
| `hero/crank::no_backwards_turning` | port logic: Pushing against his facing never turns the crank backwards | fast | lib:hero | keep |
| `hero/crank::drop_and_latch_wrap` | port logic: Off the ground he drops with 9.8 u/s² onto it | fast | lib:hero | keep |
| `hero/crank::bolt_follows_hexagon` | port logic: The bolt follows 15° ahead of him, and its hexagon steps by 60° instead of spinning back. | fast | lib:hero | keep |
| `hero/devastator::early_distance_accumulates` | port logic: early distance accumulates | fast | lib:weapons | keep |
| `hero/fx::walk_and_landing_footsteps` | game-checked: `FUN_00227e90` | fast | lib:hero | keep |
| `hero/fx::delayed_voices_play_when_their_timers_run_out` | port logic: The surfacing gasps | fast | lib:hero | keep |
| `hero/fx::ledge_climb_voice_with_the_thruster` | game-checked: The ledge climb's voice [`0x1404f8`, `0x236738`] | fast | lib:hero | keep |
| `hero/fx::full_queue_drops` | port logic: full queue drops | fast | lib:hero | keep |
| `hero/guns::aims_and_reflections` | port logic: aims and reflections | fast | lib:weapons | keep |
| `hero/idle::modifier_list_order_and_blink` | port logic: The joint-modifier list | fast | lib:hero | keep |
| `hero/idle::euler_node_quaternions` | game-checked: `FUN_00221e38` turns by −a about its axis | fast | lib:hero | keep |
| `hero/idle::running_lean_targets` | port logic: `HeroLean` while running (state 2, substate 1) | fast | lib:hero | keep |
| `hero/idle::fidget_chance_and_cooldowns` | port logic: The fidget chance on its own | fast | lib:hero | keep |
| `hero/idle::cooldown_blocks_the_chance_draw` | port logic: Short fidgets | fast | lib:hero | keep |
| `hero/idle::draw_count_per_idle_tick` | port logic: Draws per idle tick with every consumer live | fast | lib:hero | keep |
| `hero/idle::set_state_blink_period` | port logic: SetState(0) sets the blink period and clears the look | fast | lib:hero | keep |
| `hero/idle::back_table_pick` | port logic: back table pick | fast | lib:hero | keep |
| `hero/ledge::probes_see_the_step` | port logic: probes see the step | fast | lib:hero | keep |
| `hero/ledge::grab_from_a_jump_and_hang` | port logic: grab from a jump and hang | fast | lib:hero | keep |
| `hero/ledge::shimmy_both_ways_and_stop_at_the_ends` | port logic: shimmy both ways and stop at the ends | fast | lib:hero | keep |
| `hero/ledge::climb_up` | port logic: climb up | fast | lib:hero | keep |
| `hero/ledge::drop_off` | port logic: drop off | fast | lib:hero | keep |
| `hero/ledge::wall_jump_off_a_tall_wall` | port logic: wall jump off a tall wall | fast | lib:hero | keep |
| `hero/melee::square_starts_the_combo_and_chains_in_the_window` | port logic: square starts the combo and chains in the window | fast | lib:weapons | keep |
| `hero/melee::a_late_press_does_not_chain_and_the_combo_ends_in_idle` | port logic: a late press does not chain and the combo ends in idle | fast | lib:weapons | keep |
| `hero/melee::third_swing_wraps_to_the_first` | port logic: third swing wraps to the first | fast | lib:weapons | keep |
| `hero/melee::hit_frames` | port logic: hit frames | fast | lib:weapons | keep |
| `hero/melee::square_in_the_air_is_the_jump_attack` | port logic: square in the air is the jump attack | fast | lib:weapons | keep |
| `hero/melee::swap_to_the_glove_and_back` | port logic: swap to the glove and back | fast | lib:weapons | keep |
| `hero/morph_ray::beam_lays_straight_then_bends_to_the_aim` | port logic: beam lays straight then bends to the aim | fast | lib:weapons | keep |
| `hero/morph_ray::pulses_run_out_and_fade` | port logic: pulses run out and fade | fast | lib:weapons | keep |
| `hero/morph_ray::quads_cull_back_faces_and_fade_the_far_ring` | port logic: quads cull back faces and fade the far ring | fast | lib:weapons | keep |
| `hero/packs/tests::crouch_jumps_by_pack` | port logic: crouch jumps by pack | fast | lib:hero | keep |
| `hero/packs/tests::heli_high_jump_lifts` | port logic: The Heli high jump | fast | lib:hero | keep |
| `hero/packs/tests::thruster_high_jump_bursts` | port logic: The Thruster high jump | fast | lib:hero | keep |
| `hero/packs/tests::heli_long_jump_distance_and_landing` | port logic: The Heli long jump | fast | lib:hero | keep |
| `hero/packs/tests::thruster_long_jump_speed` | port logic: The Thruster long jump | fast | lib:hero | keep |
| `hero/packs/tests::glide_sink_rates_and_release` | port logic: The glide | fast | lib:hero | keep |
| `hero/packs/tests::glide_lands_into_idle` | port logic: The glide's landing | fast | lib:hero | keep |
| `hero/packs/tests::stomp` | port logic: The stomp | fast | lib:hero | keep |
| `hero/packs/tests::hover_double_tap` | game-checked: The Thruster hover [`0x13f654`] | fast | lib:hero | keep |
| `hero/packs/tests::long_jump_into_a_wall_rebounds` | port logic: The long jumps into a wall | fast | lib:hero | keep |
| `hero/packs/tests::pack_loops_start_and_stop` | port logic: The hero's looping sound slots | fast | lib:hero | keep |
| `hero/packs/tests::double_jump_boost_by_pack` | port logic: The double jump's boost with the Thruster-Pack on the back is larger (×1.25 | fast | lib:hero | keep |
| `hero/physics::approach_and_spring` | port logic: approach and spring | fast | lib:hero | keep |
| `hero/physics::rotations_wrap` | port logic: rotations wrap | fast | lib:hero | keep |
| `hero/physics::turn_spring_converges` | port logic: turn spring converges | fast | lib:hero | keep |
| `hero/platform::idle_rider_is_pinned` | game-checked: Idle on a moving platform [`0x13f490`] | fast | lib:hero | keep |
| `hero/platform::walking_rider_gets_the_move` | game-checked: Walking (group 1) [`0x13f4a0`] | fast | lib:hero | keep |
| `hero/platform::jumping_off_decays_the_push` | port logic: Jumping off | fast | lib:hero | keep |
| `hero/platform::drop_beyond_two_units_and_snap` | port logic: The attachment drops when the pinned point is more than 2 units away (a teleport, a push), a correction below 1e-4… | fast | lib:hero | keep |
| `hero/platform::turning_carrier_turns_the_rider` | game-checked: A carrier that turns (none on the disc [`0x13f44c`] | fast | lib:hero | keep |
| `hero/platform::not_a_carrier` | port logic: Not a carrier (no block, or no world) | fast | lib:hero | keep |
| `hero/platform::euler_round_trips` | port logic: The rotations | fast | lib:hero | keep |
| `hero/pyrocitor::launch_speed_rule` | game-checked: `0x270830` from rest [`0x270830`] | fast | lib:weapons | keep |
| `hero/registry::implemented_set_unchanged` | port logic: The restructure is behaviour-neutral | fast | lib:hero | keep |
| `hero/registry::stubs_own_no_ported_state` | port logic: Every module's ported states route to the module's own code (no state is ported in a stub module). | fast | lib:hero | keep |
| `hero/ryno::search_scores_and_filters` | game-checked: 0x2e4bb8 [`0x2e4bb8`] | fast | lib:weapons | keep |
| `hero/scripted::visibomb_flight_holds_ratchet_with_the_gun_out` | port logic: `SetState(100, 2)` (a scene) | fast | lib:hero | keep |
| `hero/scripted::scene_body_holds_ratchet_still` | port logic: scene body holds ratchet still | fast | lib:hero | keep |
| `hero/suck_cannon::groups` | port logic: groups | fast | lib:weapons | keep |
| `hero/suck_vortex::circle_spring_rotation` | port logic: circle spring rotation | fast | lib:weapons | keep |
| `hero/suck_vortex::vacuum_reach` | port logic: vacuum reach | fast | lib:weapons | keep |
| `hero/surface::level_rules` | port logic: level rules | fast | lib:hero | keep |
| `hero/surface::pass_surface_per_level` | port logic: the per-level pass-through surface table and flags [`0x233940`, `0x248268`] | fast | lib:hero | keep |
| `hero/surface::reaction_flags_per_level` | port logic: The flags of the reaction follow the level's rules | fast | lib:hero | keep |
| `hero/surface::steep_ramp_slides_him_down` | port logic: A slope steeper than 50° is not ground | fast | lib:hero | keep |
| `hero/surface::slippery_ramp_slides_him_down` | port logic: A slippery ramp (surface 7, level 0) | fast | lib:hero | keep |
| `hero/surface::flat_ice_walk_and_stop` | port logic: Flat ice | fast | lib:hero | keep |
| `hero/surface::sinking_floor_state` | game-checked: The sinking floor (surface 4, level 1) [`0x13f530`] | fast | lib:hero | keep |
| `hero/surface::sinking_liquid_states` | game-checked: Sinking liquid (surface 3, level 0) over a floor 3 below [`0x1409b0`] | fast | lib:hero | keep |
| `hero/swim/effects::splash_draws_and_queue` | game-checked: `0x22b3a8(3, n, 1)` [`0x22b3a8`] | fast | lib:hero | keep |
| `hero/swim/effects::wake_counter` | game-checked: The counters [`0x22ac40`] | fast | lib:hero | keep |
| `hero/swim/tests::fall_into_deep_water_floats_on_the_surface` | port logic: fall into deep water floats on the surface | fast | lib:hero | keep |
| `hero/swim/tests::surface_swim_speed_and_stop` | port logic: surface swim speed and stop | fast | lib:hero | keep |
| `hero/swim/tests::no_pack_dives_slowly_and_r1_does_nothing` | port logic: no pack dives slowly and r1 does nothing | fast | lib:hero | keep |
| `hero/swim/tests::hydro_pack_dive_thrust_and_surface` | game-checked: hydro pack dive thrust and surface [`0x236520`] | fast | lib:hero | keep |
| `hero/swim/tests::out_of_air_drowns_unless_masked` | port logic: out of air drowns unless masked | fast | lib:hero | keep |
| `hero/swim/tests::jump_out_of_deep_water` | port logic: jump out of deep water | fast | lib:hero | keep |
| `hero/swim/tests::swim_up_a_ramp_wades_out` | port logic: swim up a ramp wades out | fast | lib:hero | keep |
| `hero/swim/tests::water_level_comes_from_the_water_tables` | port logic: The ground probe takes the water level from the level's tables (a ripple patch) when they cover the hit. | fast | lib:hero | keep |
| `hero/swim/tests::entry_splash_and_rand_ledger` | game-checked: The fall into deep water [`0x22b3a8`, `0x22bdd0`] | fast | lib:hero | keep |
| `hero/swim/tests::treading_water_leaves_a_wake` | game-checked: Treading water [`0x22ac40`] | fast | lib:hero | keep |
| `hero/swim/tests::surface_swim_bow_rings` | game-checked: Swimming on the surface at full stick [`0x22ad38`, `0x22af48`] | fast | lib:hero | keep |
| `hero/swim/tests::water_jump_splashes_at_tick_20` | game-checked: The deep-water jump 0x12 [`0x22b3a8`] | fast | lib:hero | keep |
| `hero/swim/tests::wading_and_shallow_water_splash` | game-checked: Wading (0.5 deep, state 0x73) and ankle-deep water (0.19 deep, state 2 in water) [`0x22b140`, `0x22b3a8`] | fast | lib:hero | keep |
| `hero/swingshot/tests::pull_search_rules` | game-checked: The pull search [`0x13fcb4`] | fast | lib:hero | keep |
| `hero/swingshot/tests::swing_search_rules` | port logic: The swing search | fast | lib:hero | keep |
| `hero/swingshot/tests::pull_fire_fly_release` | port logic: ○ at a pull target | fast | lib:hero | keep |
| `hero/swingshot/tests::pull_arrives_by_itself` | port logic: Held all the way | fast | lib:hero | keep |
| `hero/swingshot/tests::arrive_pendulum` | port logic: Mode ≠ 0 (record +0x08) | fast | lib:hero | keep |
| `hero/swingshot/tests::fire_ends_without_the_swingshot` | port logic: The hand item is no longer the Swingshot | fast | lib:hero | keep |
| `hero/swingshot/tests::swing_rope_release` | port logic: ○ at a swing target | fast | lib:hero | keep |
| `hero/swingshot/tests::swing_needs_ten_ticks` | port logic: ○ tapped | fast | lib:hero | keep |
| `hero/swingshot/tests::record_parse` | data/format: The record parser reads a disc record (Aridia's first swing target). (L02) | fast | lib:hero | keep |
| `hero/tesla::strips_face_the_eye` | port logic: The core strip | fast | lib:weapons | keep |
| `hero/tesla::reset_lays_the_chain_straight` | port logic: reset lays the chain straight | fast | lib:weapons | keep |
| `hero/walloper::melee_aim_search_scores` | game-checked: `0x22e238` / `0x22dff0` [`0x22dff0`, `0x22e238`] | fast | lib:weapons | keep |
| `hero/walloper::dec_timer_semantics` | port logic: dec timer semantics | fast | lib:weapons | keep |
| `hero/walloper::arc_geometry_and_draw` | port logic: The arcs | fast | lib:weapons | keep |
| `hero/weapons::launch_velocity_arcs` | game-checked: `0x2d80f0` aimed 8.5 ahead at the same height [`0x2d80f0`] | fast | lib:weapons | keep |
| `hero/weapons::ammo` | port logic: ammo | fast | lib:weapons | keep |
| `hero/weapons::circle_throws` | port logic: ○ standing | fast | lib:weapons | keep |
| `hero/weapons::holding_layers` | game-checked: `0x22e660` [`0x1413fb`, `0x1413ff`] | fast | lib:weapons | keep |
| `hero/weapons::circle_while_running_draws_the_arm` | game-checked: ○ while running [`0x13f50c`, `0x1413f8`] | fast | lib:weapons | keep |
| `hero/worn::grindboots_follow_the_grind_group` | port logic: Grinding with the Grindboots owned puts them on (saved), leaving the grind group restores what was worn before (nothing) | fast | lib:hero | keep |
| `hero/worn::magneboots_and_water` | port logic: Magnetic floor → Magneboots (the surface reaction's gravity test then sees them) | fast | lib:hero | keep |
| `hero/worn::o2_mask_under_water` | port logic: Under water the O2 Mask goes on without being saved | fast | lib:hero | keep |
| `hero/worn::requests_equip_and_unequip` | port logic: A menu request (the Gadgets page's close) swaps the slot like any request | fast | lib:hero | keep |
| `hud::prompt_slot_12_shows_while_requested` | game-checked: The context prompt (slot 12, 0x24c898) [`0x24c898`] | fast | lib:ui | keep |
| `hud::nothing_on_screen_until_something_changes` | port logic: nothing on screen until something changes | fast | lib:ui | keep |
| `hud::bolt_ramp_timings` | port logic: First pickup | fast | lib:ui | keep |
| `hud::bolt_visible_90_ticks_after_a_change` | port logic: Later changes show the counter for 90 ticks (the update's ScaleTicks(90)). | fast | lib:ui | keep |
| `hud::health_shows_120_ticks_after_damage_and_always_at_one_hp` | port logic: health shows 120 ticks after damage and always at one hp | fast | lib:ui | keep |
| `hud::bolt_separators_english_and_german` | port logic: bolt separators english and german | fast | lib:ui | keep |
| `hud::wrench_hides_the_weapon_slot_and_ammo_weapon_persists` | port logic: wrench hides the weapon slot and ammo weapon persists | fast | lib:ui | keep |
| `hud::help_box_draws_from_the_help_system` | game-checked: The help box of the game tick (crate::help) drawn by the HUD: sized with the small font, centred at (256, H − 60),… | fast | lib:ui | keep |
| `hud::window_wrap_balances_short_last_lines` | game-checked: window wrap balances short last lines | fast | lib:ui | keep |
| `inventory::select_rules_per_slot` | port logic: select rules per slot | fast | lib:ui | keep |
| `inventory::close_requests_only_changed_slots` | port logic: close requests only changed slots | fast | lib:ui | keep |
| `map/predicates::predicates_match_the_game_code` | game-checked: Every predicate against its game code [`0x140634`, `0x25cc08`] (all levels) | fast | lib:ui | keep |
| `menus/mode::raw_round_trip_and_tick_rule` | port logic: raw round trip and tick rule | fast | lib:ui | keep |
| `menus/mode::frames_in_mode_counts_and_resets` | port logic: frames in mode counts and resets | fast | lib:ui | keep |
| `menus/pause/frame::cvt_truncates_toward_zero` | port logic: cvt truncates toward zero | fast | lib:ui | keep |
| `menus/pause/frame::menu_projection_matches_the_pinhole` | game-checked: The projection is the game's pixel mapping | fast | lib:ui | keep |
| `menus/pause/frame::root_page_panels_from_the_disc` | data/format: Class 1138 on Novalis (L01) | fast | lib:ui | keep |
| `menus/pause/gadgets::cursor_cell_item` | port logic: cursor cell item | fast | lib:ui | keep |
| `menus/pause/gadgets::angle_wraps` | port logic: angle wraps | fast | lib:ui | keep |
| `menus/pause/map_page::conditions` | game-checked: `fun_0020baf0`'s condition types on a bare state, each flipped by its own field. [`0x262c68`] | fast | lib:ui | keep |
| `menus/pause/map_page::callbacks` | game-checked: The 14 mission callbacks (0x262b40..0x262d78), each against its fields. [`0x262b40`, `0x262b68`] | fast | lib:ui | keep |
| `menus/pause/map_page::mission_status_rules` | game-checked: `FUN_00262760` [`0x262c78`] | fast | lib:ui | keep |
| `menus/pause/port::cycle_skips_unsupported_values` | port logic: cycle skips unsupported values | fast | lib:ui | keep |
| `menus/pause/tests::open_transition_is_12_raw_ticks_with_the_seqs_reversed` | port logic: open transition is 12 raw ticks with the seqs reversed | fast | lib:ui | keep |
| `menus/pause/tests::highlight_timer_and_focus_moves` | port logic: highlight timer and focus moves | fast | lib:ui | keep |
| `menus/pause/tests::goodies_wiring_both_ways` | port logic: goodies wiring both ways | fast | lib:ui | keep |
| `menus/pause/tests::root_enter_disables_the_first_three_when_0x1413f4_is_1` | port logic: root enter disables the first three when 0x1413f4 is 1 | fast | lib:ui | keep |
| `menus/pause/tests::close_needs_10_menu_ticks_then_2_ticks_of_kind_0x14` | port logic: close needs 10 menu ticks then 2 ticks of kind 0x14 | fast | lib:ui | keep |
| `menus/pause/tests::sound_slider_full_range_takes_342_ticks` | port logic: sound slider full range takes 342 ticks | fast | lib:ui | keep |
| `menus/pause/tests::planet_list_in_acquisition_order` | game-checked: planet list in acquisition order [`0x184894`] | fast | lib:ui | keep |
| `menus/pause/tests::blink_cadence_22_on_8_off` | port logic: blink cadence 22 on 8 off | fast | lib:ui | keep |
| `menus/pause/tests::overlay_page_tree` | game-checked: The disc's page tree (skipped without `extracted/`). [`0x15ede4`, `0x15ee1c`] (disc data) | fast | lib:ui | keep |
| `menus/pause/tests::port_page_from_the_disc` | game-checked: port page from the disc [`0x1b5160`] (disc data) | fast | lib:ui | keep |
| `menus/quick_select::layout_positions_for_8_slots` | port logic: layout positions for 8 slots | fast | lib:ui | keep |
| `menus/quick_select::sector_edges_27_and_63_degrees_with_hysteresis` | port logic: sector edges 27 and 63 degrees with hysteresis | fast | lib:ui | keep |
| `menus/quick_select::dpad_chains_from_none_and_each_slot` | port logic: dpad chains from none and each slot | fast | lib:ui | keep |
| `menus/quick_select::tap_closes_without_equip` | port logic: tap closes without equip | fast | lib:ui | keep |
| `menus/quick_select::hold_then_release_equips_and_locks_after_15_ticks` | port logic: hold then release equips and locks after 15 ticks | fast | lib:ui | keep |
| `menus/quick_select::double_tap_window_20_20` | port logic: double tap window 20 20 | fast | lib:ui | keep |
| `menus/quick_select::overlay_tables_match_level01` | port logic: The overlay's tables and constants are the values above (skipped without `extracted/`). (disc data) | fast | lib:ui | keep |
| `menus/quick_select::name_split_rules` | port logic: name split rules | fast | lib:ui | keep |
| `menus/quick_select::draw_emits_ring_icons_cursor_and_text` | port logic: draw emits ring icons cursor and text | fast | lib:ui | keep |
| `menus/screen_static::burst_fades_in_holds_and_fades_out` | port logic: burst fades in holds and fades out | fast | lib:ui | keep |
| `menus/screen_static::panel_draws_vignette_noise_lines_and_glass` | port logic: panel draws vignette noise lines and glass | fast | lib:ui | keep |
| `menus/screen_static::a_burst_ends_after_128_frames` | port logic: a burst ends after 128 frames | fast | lib:ui | keep |
| `menus/vendor/tests::list_is_stock_then_owned_ammo` | port logic: list is stock then owned ammo | fast | lib:ui | keep |
| `menus/vendor/tests::price_text_formats` | port logic: price text formats | fast | lib:ui | keep |
| `menus/vendor/tests::vendor_camera_faces_the_vendor` | port logic: vendor camera faces the vendor | fast | lib:ui | keep |
| `menus/vendor/tests::euler_rows_match_the_moby_matrix` | port logic: euler rows match the moby matrix | fast | lib:ui | keep |
| `menus/vendor/tests::screen_quad_insets_and_shrinks_about_its_centre` | port logic: screen quad insets and shrinks about its centre | fast | lib:ui | keep |
| `menus/vendor/tests::projection_centres_the_view_axis` | port logic: projection centres the view axis | fast | lib:ui | keep |
| `menus/vendor/tests::ticker_static_always_runs_and_other_screens_burst` | game-checked: ticker static always runs and other screens burst [`0x221110`] | fast | lib:ui | keep |
| `menus/vendor/tests::scan_bar_fades_in_and_out` | port logic: scan bar fades in and out | fast | lib:ui | keep |
| `menus/vendor/tests::power_factor_follows_the_counters` | port logic: power factor follows the counters | fast | lib:ui | keep |
| `menus/vendor/tests::ticker_glyphs_are_half_bright_textured_quads` | game-checked: ticker glyphs are half bright textured quads | fast | lib:ui | keep |
| `moby_runtime::create_reuses_slots_two_ticks_after_delete` | port logic: create reuses slots two ticks after delete | fast | lib:classes | keep |
| `moby_runtime::a_slot_without_header_keeps_its_update` | game-checked: a slot without header keeps its update [`0x2bd100`] | fast | lib:classes | keep |
| `moby_runtime::free_slot_pass_counts_reusable_and_trailing_slots` | port logic: free slot pass counts reusable and trailing slots | fast | lib:classes | keep |
| `moby_update::active_rule_camera_distance_visible_groups_and_slot_order` | port logic: active rule camera distance visible groups and slot order | fast | lib:classes | keep |
| `moby_update::bolt_flies_to_the_hero_and_is_collected` | port logic: bolt flies to the hero and is collected | fast | lib:classes | keep |
| `moby_update::crate_break_drops_bolts_with_the_game_draws` | game-checked: crate break drops bolts with the game draws | fast | lib:classes | keep |
| `moby_update::tnt_fuse_runs_179_updates_with_four_beeps` | port logic: tnt fuse runs 179 updates with four beeps | fast | lib:classes | keep |
| `moby_update::grass_bends_when_the_hero_runs_through_and_returns` | port logic: grass bends when the hero runs through and returns | fast | lib:classes | keep |
| `moby_update/anim_sound::trigger_window_and_transitions` | port logic: trigger window and transitions | fast | lib:classes | keep |
| `moby_update/anim_sound::init_and_sequence_change_set_the_loop_byte` | port logic: init and sequence change set the loop byte | fast | lib:classes | keep |
| `moby_update/classes/activation_zone::rules_follow_the_zone` | port logic: rules follow the zone | fast | lib:classes | keep |
| `moby_update/classes/blaster_shot::trail_curve_runs_from_0_to_1` | port logic: trail curve runs from 0 to 1 | fast | lib:classes | keep |
| `moby_update/classes/bolt_crank::latch_turn_release_unwind` | port logic: Init, the 30-tick wait, the latch (SetState 0x3b queued, state 3, the angle from the progress), then a release… | fast | lib:classes | keep |
| `moby_update/classes/bolt_crank::done_sinks_and_persists` | port logic: Done | fast | lib:classes | keep |
| `moby_update/classes/bolt_crank::latch_refusals` | port logic: No latch while an enemy (a targetable moby) is within 3.25 of the bolt, nor with an undone rearm. | fast | lib:classes | keep |
| `moby_update/classes/bolt_crank::rotator_follows_progress` | port logic: The rotator | fast | lib:classes | keep |
| `moby_update/classes/bomb::fuse_explosion_and_fireballs` | port logic: A released bomb whose fuse runs out in the air | fast | lib:classes | keep |
| `moby_update/classes/bomb::dry_explosion_rand_stream_matches_the_game_ledger` | game-checked: The rand stream through a bomb's flight and dry explosion, tick by tick, against a ledger of the draws the game's… [`0x2c35c8`, `0x2c4c20`] | fast | lib:classes | keep |
| `moby_update/classes/bomb::blast_glove_row` | game-checked: The Bomb Glove's row (`0x2c3300`, k = 1) [`0x2c3300`] | fast | lib:classes | keep |
| `moby_update/classes/bomb::blast_suck_cannon_row` | game-checked: The Suck Cannon burst's row (`0x304798`) [`0x270f48`, `0x304798`] | fast | lib:classes | keep |
| `moby_update/classes/bomb::blast_tank_row` | game-checked: Gemlik's tank row (level13 `0x3073c8` / `0x30c138`) [`0x3073c8`, `0x30c138`] (L13) | fast | lib:classes | keep |
| `moby_update/classes/bomb_water::bomb_sinks_onto_a_shallow_floor` | port logic: Shallow water (floor 0.75 under the surface) | fast | lib:classes | keep |
| `moby_update/classes/bomb_water::bomb_fuse_ends_just_under_the_surface` | port logic: The fuse runs out 10 ticks after the entry | fast | lib:classes | keep |
| `moby_update/classes/bomb_water::bomb_bursts_in_deep_water` | port logic: Deep water (floor 6.75 under the surface) + determinism | fast | lib:classes | keep |
| `moby_update/classes/breakables::recipes_are_distinct_and_their_piece_ops_are_sane` | port logic: recipes are distinct and their piece ops are sane | fast | lib:classes | keep |
| `moby_update/classes/breakables::every_port_address_names_one_port` | port logic: every port address names one port | fast | lib:classes | keep |
| `moby_update/classes/breakables::add_rot_wraps_once` | port logic: add rot wraps once | fast | lib:classes | keep |
| `moby_update/classes/buried_bolts::nibbles_high_then_low` | port logic: nibbles high then low | fast | lib:classes | keep |
| `moby_update/classes/checkpoint::taking_sets_its_mission_once` | port logic: Entering the cuboid takes the checkpoint once | fast | lib:classes | keep |
| `moby_update/classes/debris::registry_maps_every_debris_and_flash_class` | port logic: registry maps every debris and flash class | fast | lib:classes | keep |
| `moby_update/classes/debris::level_table_lists_the_shared_classes` | game-checked: The level01 class table (read from the level 01 overlay when `extracted/` is present) maps exactly [`CLASSES`] to… [`0x2c22a8`, `0x2c5218`] (L01) | fast | lib:classes | keep |
| `moby_update/classes/debris::piece_falls_bounces_fades_and_is_deleted` | port logic: piece falls bounces fades and is deleted | fast | lib:classes | keep |
| `moby_update/classes/debris::piece_without_bounces_falls_through` | port logic: piece without bounces falls through | fast | lib:classes | keep |
| `moby_update/classes/debris::scale_range_and_spin_come_from_the_stream` | port logic: scale range and spin come from the stream | fast | lib:classes | keep |
| `moby_update/classes/debris::flash_grows_fades_and_is_deleted` | port logic: flash grows fades and is deleted | fast | lib:classes | keep |
| `moby_update/classes/debris::two_runs_are_identical` | port logic: two runs are identical | fast | lib:classes | keep |
| `moby_update/classes/decoy::pops_on_a_pop_surface` | game-checked: `0x2d9f90` / `0x2d90a8` [`0x2d90a8`, `0x2d9f90`] | fast | lib:classes | keep |
| `moby_update/classes/decoy::slides_from_under_ratchet` | game-checked: `0x2d9d08` [`0x13f532`, `0x2d9d08`] | fast | lib:classes | keep |
| `moby_update/classes/doom_bot::drops_out_and_walks` | port logic: State 0 → 0xe → walking | fast | lib:classes | keep |
| `moby_update/classes/doom_bot::lives_sixty_seconds` | port logic: The lifetime | fast | lib:classes | keep |
| `moby_update/classes/doom_bot::explodes_when_hit` | game-checked: `MobyGetHitMessage(0x800001)` | fast | lib:classes | keep |
| `moby_update/classes/doom_bot::explodes_on_a_pop_surface` | game-checked: `0x2d5b30` [`0x2d5b30`] | fast | lib:classes | keep |
| `moby_update/classes/doom_bot::runs_out_of_patience` | port logic: Patience | fast | lib:classes | keep |
| `moby_update/classes/doom_bot::lob_apex` | game-checked: `0x26e3e8` [`0x26e3e8`] | fast | lib:classes | keep |
| `moby_update/classes/doom_canister::opens_and_lets_out_four_bots` | game-checked: `0x2de650` states 2 → 3 → 4 [`0x2de650`] | fast | lib:classes | keep |
| `moby_update/classes/doom_canister::pops_on_a_pop_surface` | game-checked: `0x2ddce0` [`0x2ddce0`] | fast | lib:classes | keep |
| `moby_update/classes/doom_canister::waits_while_eight_bots_are_out` | port logic: State 3 with 8 bots alive | fast | lib:classes | keep |
| `moby_update/classes/drone::stuck_out_of_view_jumps_behind_the_camera` | game-checked: State 2 stuck (the move's wall bit [`0x167240`, `0x167450`] | fast | lib:classes | keep |
| `moby_update/classes/fire_field::element_range_is_start_plus_both_counts` | port logic: element range is start plus both counts | fast | lib:classes | keep |
| `moby_update/classes/fire_field::cuboid_point_is_row_vector_form` | port logic: cuboid point is row vector form | fast | lib:classes | keep |
| `moby_update/classes/floor_switch::press_opens_the_path` | port logic: press opens the path | fast | lib:classes | keep |
| `moby_update/classes/flow::measure_writes_segment_lengths` | port logic: The init's measure | fast | lib:classes | keep |
| `moby_update/classes/flow::flow_pushes_the_sinking_hero_along` | port logic: Init measures the spline and sets the update distance | fast | lib:classes | keep |
| `moby_update/classes/flow::level1_speed_by_length_left` | port logic: Level 1's speed by the length left | fast | lib:classes | keep |
| `moby_update/classes/flyer::hermite_hits_the_ends_and_the_tangents` | port logic: hermite hits the ends and the tangents | fast | lib:classes | keep |
| `moby_update/classes/flyer::spring_turn_limits_the_step_to_the_error` | port logic: spring turn limits the step to the error | fast | lib:classes | keep |
| `moby_update/classes/flyer::wraps` | port logic: wraps | fast | lib:classes | keep |
| `moby_update/classes/gold_bolt::timer_counts_down_like_fast_dec_timer` | port logic: timer counts down like fast dec timer | fast | lib:classes | keep |
| `moby_update/classes/gold_bolt::levels_are_all_but_veldin` | port logic: levels are all but veldin (L00) | fast | lib:classes | keep |
| `moby_update/classes/hinged_bridge::raised_pitch_is_36_degrees` | port logic: raised pitch is 36 degrees | fast | lib:classes | keep |
| `moby_update/classes/infobot::aim_is_two_units_ahead_on_an_even_path` | port logic: aim is two units ahead on an even path | fast | lib:classes | keep |
| `moby_update/classes/infobot::levels_are_the_cluster` | port logic: levels are the cluster | fast | lib:classes | keep |
| `moby_update/classes/mine::explodes_in_water` | port logic: A mine thrown into water (the pool's water at 12 over a floor at 8) | fast | lib:classes | keep |
| `moby_update/classes/missile::intercepts` | port logic: A still target | fast | lib:classes | keep |
| `moby_update/classes/mouse::house_opens_and_closes` | port logic: The house opens on command 3, stays open, closes on 1. | fast | lib:classes | keep |
| `moby_update/classes/mouse::mouse_home` | port logic: Without a house the mouse deletes itself | fast | lib:classes | keep |
| `moby_update/classes/path_platform::waits_until_called_then_travels_and_arrives` | port logic: waits until called then travels and arrives | fast | lib:classes | keep |
| `moby_update/classes/path_platform::ridden_after_rearm` | port logic: ridden after rearm | fast | lib:classes | keep |
| `moby_update/classes/path_platform::pauses_above_ratchet_while_descending` | port logic: pauses above ratchet while descending | fast | lib:classes | keep |
| `moby_update/classes/path_platform::activation_cuboid_latches_into_the_death_bits` | port logic: activation cuboid latches into the death bits | fast | lib:classes | keep |
| `moby_update/classes/path_platform::novalis_lift` | port logic: Novalis (from `extracted/`) (L01) | fast | lib:classes | keep |
| `moby_update/classes/path_platform::appear_cuboid_hides_until_entered` | port logic: appear cuboid hides until entered | fast | lib:classes | keep |
| `moby_update/classes/pickup::cubic_runs_from_b_to_c` | port logic: cubic runs from b to c | fast | lib:classes | keep |
| `moby_update/classes/pickup::rotate_turns_right_handed` | port logic: rotate turns right handed | fast | lib:classes | keep |
| `moby_update/classes/pickup::item_tables_read_the_records` | port logic: item tables read the records | fast | lib:classes | keep |
| `moby_update/classes/pickup::collect_only_when_the_ammo_rises` | game-checked: `0x2db850` [`0x2db850`] | fast | lib:classes | keep |
| `moby_update/classes/pyro_glow::wrap_keeps_small_angles` | port logic: wrap keeps small angles | fast | lib:classes | keep |
| `moby_update/classes/rc_range::idle_without_the_missile` | port logic: idle without the missile | fast | lib:classes | keep |
| `moby_update/classes/rc_range::static_grid` | game-checked: `0x302438` with the counter at 45 [`0x302438`] | fast | lib:classes | keep |
| `moby_update/classes/swing_target::turn_accel_reaches_the_target_and_stops` | port logic: turn accel reaches the target and stops | fast | lib:classes | keep |
| `moby_update/classes/swing_target::registry_maps_both_target_classes` | port logic: registry maps both target classes | fast | lib:classes | keep |
| `moby_update/classes/talking_npc::scene_end_sets_mission_and_checkpoint` | game-checked: State 2 after the talker's scene | fast | lib:classes | keep |
| `moby_update/classes/thruster_flame::long_jump_states_and_exhaust_rate` | game-checked: `0x2c9e00` through a Thruster long jump [`0x2c9e00`] | fast | lib:classes | keep |
| `moby_update/classes/thruster_flame::hover_and_glide_rates` | game-checked: The hover 0x81 | fast | lib:classes | keep |
| `moby_update/classes/thruster_flame::idle_flame_waits` | port logic: Not thrusting (a plain jump that is descending) | fast | lib:classes | keep |
| `moby_update/classes/thruster_flame::frame_is_orthonormal_along_the_nozzle` | port logic: The frame | fast | lib:classes | keep |
| `moby_update/classes/units/asteroid::init_reset_and_idle` | port logic: init reset and idle | fast | lib:classes | keep |
| `moby_update/classes/units/bob_block::bobs_on_the_tick_counter_and_rests_in_cutscenes` | port logic: bobs on the tick counter and rests in cutscenes | fast | lib:classes | keep |
| `moby_update/classes/units/chain_link::break_runs_down_the_chain` | port logic: break runs down the chain | fast | lib:classes | keep |
| `moby_update/classes/units/chain_link::search_takes_the_nearest_within_half_a_unit` | port logic: search takes the nearest within half a unit | fast | lib:classes | keep |
| `moby_update/classes/units/conveyor::one_way_and_reversing_belts_carry_riders` | port logic: one way and reversing belts carry riders | fast | lib:classes | keep |
| `moby_update/classes/units/grind_mine::a_grinding_ratchet_or_a_hit_sets_it_off` | port logic: a grinding ratchet or a hit sets it off | fast | lib:classes | keep |
| `moby_update/classes/units/help_director::infobot_hints_by_planet_bits_and_records` | port logic: infobot hints by planet bits and records | fast | lib:classes | keep |
| `moby_update/classes/units/help_director::early_move_hint_window` | port logic: early move hint window | fast | lib:classes | keep |
| `moby_update/classes/units/help_director::swim_group_0x11_retires_the_swim_hint` | port logic: swim group 0x11 retires the swim hint | fast | lib:classes | keep |
| `moby_update/classes/units/hydro_pad::glow_fades_in_under_ratchet` | port logic: glow fades in under ratchet | fast | lib:classes | keep |
| `moby_update/classes/units/lamp::pulse_colour_and_one_callback_per_group` | port logic: pulse colour and one callback per group | fast | lib:classes | keep |
| `moby_update/classes/units/linked_mover::opens_while_the_link_is_away_and_closes_on_state_b` | port logic: opens while the link is away and closes on state b | fast | lib:classes | keep |
| `moby_update/classes/units/loose_piece::rests_then_flies_fades_and_drops_out` | port logic: rests then flies fades and drops out | fast | lib:classes | keep |
| `moby_update/classes/units/rising_block::rises_on_the_links_command` | port logic: rises on the links command | fast | lib:classes | keep |
| `moby_update/classes/units/slider::slides_to_the_target_fraction` | port logic: slides to the target fraction | fast | lib:classes | keep |
| `moby_update/classes/units/smoke_emitter::one_blob_per_period` | port logic: one blob per period | fast | lib:classes | keep |
| `moby_update/classes/units/timed_switch::three_presses_win_and_a_timeout_resets` | port logic: three presses win and a timeout resets | fast | lib:classes | keep |
| `moby_update/classes/units/vent::steam_and_sparks_spawn_trail_blobs` | port logic: steam and sparks spawn trail blobs | fast | lib:classes | keep |
| `moby_update/classes/visibomb::pitch_bend_is_the_games_expression` | game-checked: pitch bend is the games expression | fast | lib:classes | keep |
| `moby_update/classes/visibomb::view_writes_count` | port logic: view writes count | fast | lib:classes | keep |
| `moby_update/creature/fx::flash_kinds` | game-checked: §F: the two flash spawners are one code [`0x2c20e0`, `0x309a68`] | fast | lib:classes | keep |
| `moby_update/creature/fx::spark_burst_death_row` | game-checked: §C `0x273f50` [`0x273f50`] | fast | lib:classes | keep |
| `moby_update/creature/fx::spark_burst_piece_row_swaps_red_and_green` | game-checked: §C `0x2742a8` (divergence fixed 2026-09-29) [`0x270f48`, `0x270fa8`] | fast | lib:classes | keep |
| `moby_update/creature/fx::spark_burst_shot_row` | game-checked: §C `0x2fa1b0` (the path enemies' glob) [`0x2fa1b0`] | fast | lib:classes | keep |
| `moby_update/creature/fx::beam_flashes_carry_the_base_vector` | port logic: §A `SpawnBeamExplosion` (divergence fixed 2026-09-29) | fast | lib:classes | keep |
| `moby_update/creature/fx::crash_beam_throws_sixty_fireballs` | regression: §A the Novalis crash (`0x30c190`, divergence fixed 2026-09-29) [`0x30c190`] (L01) | fast | lib:classes | keep |
| `moby_update/creature/knock::knocked_into_water_splashes_once` | game-checked: A knocked creature crossing a water face (surface 0) makes the splash 775 at the water's height (`FUN_002ff768(3,… | fast | lib:classes | keep |
| `moby_update/creature/react::quaternion_round_trip_and_slerp_ends` | port logic: quaternion round trip and slerp ends | fast | lib:classes | keep |
| `moby_update/creature/react::swallow_and_fire_take_the_cannons_rows` | game-checked: `0x302840` with `gp−0x4d60` (the swallow) and `gp−0x4d50` (the fire) [`0x302840`] | fast | lib:classes | keep |
| `moby_update/creature/react::burst_carries_the_base_velocity` | game-checked: `0x304798` [`0x304798`] | fast | lib:classes | keep |
| `moby_update/creature/react::tables_by_class_number` | port logic: tables by class number | fast | lib:classes | keep |
| `moby_update/creature/tests::rotations_wrap_like_the_game` | game-checked: rotations wrap like the game | fast | lib:classes | keep |
| `moby_update/creature/tests::turn_toward_stops_on_the_target` | game-checked: turn toward stops on the target | fast | lib:classes | keep |
| `moby_update/creature/tests::rate_slot_takes_the_last_free_slot` | port logic: rate slot takes the last free slot | fast | lib:classes | keep |
| `moby_update/creature/tests::critter_hovers_with_three_draws_per_pick` | port logic: critter hovers with three draws per pick | fast | lib:classes | keep |
| `moby_update/creature/tests::critter_lands_walks_and_bites_ratchet` | port logic: critter lands walks and bites ratchet | fast | lib:classes | keep |
| `moby_update/creature/tests::a_hit_kills_the_critter_with_pieces_explosion_and_bolts` | port logic: a hit kills the critter with pieces explosion and bolts | fast | lib:classes | keep |
| `moby_update/creature/tests::resolver_cools_down_repeated_hits_of_one_kind` | game-checked: resolver cools down repeated hits of one kind | fast | lib:classes | keep |
| `moby_update/creature/tests::flash_ramps_to_red_and_back` | port logic: flash ramps to red and back | fast | lib:classes | keep |
| `moby_update/creature/tests::region_walls_and_waypoints` | port logic: region walls and waypoints | fast | lib:classes | keep |
| `moby_update/creature/tests::spring_turn_is_spring_turn2_on_any_angle` | port logic: spring turn is spring turn2 on any angle | fast | lib:classes | keep |
| `moby_update/creature/tests::projectile_parts_draw_like_the_spawners` | port logic: projectile parts draw like the spawners | fast | lib:classes | keep |
| `moby_update/creature/tests::projectile_sweep_hits_the_floor_and_misses_the_air` | port logic: projectile sweep hits the floor and misses the air | fast | lib:classes | keep |
| `moby_update/creature/tests::the_morph_spawns_a_chicken_and_deletes_the_target` | port logic: the morph spawns a chicken and deletes the target | fast | lib:classes | keep |
| `moby_update/creature/tests::the_chicken_runs_from_ratchet` | port logic: the chicken runs from ratchet | fast | lib:classes | keep |
| `moby_update/creature/tests::a_hit_bursts_the_chicken_into_feathers` | port logic: a hit bursts the chicken into feathers | fast | lib:classes | keep |
| `moby_update/creature/tests::the_gold_chicken_is_a_tough_decoy` | port logic: the gold chicken is a tough decoy | fast | lib:classes | keep |
| `moby_update/creature/tests::a_displaced_chicken_goes_when_out_of_view` | port logic: a displaced chicken goes when out of view | fast | lib:classes | keep |
| `moby_update/creature/tests::move_ground_walks_a_floor_and_refuses_a_step` | port logic: move ground walks a floor and refuses a step | fast | lib:classes | keep |
| `moby_update/creature/tests::lerp_rot_goes_the_short_way` | port logic: lerp rot goes the short way | fast | lib:classes | keep |
| `moby_update/creature/tests::wander_picks_turns_and_steps_on_the_ground` | port logic: wander picks turns and steps on the ground | fast | lib:classes | keep |
| `moby_update/creature/tests::wander_heads_home_past_the_leash_and_shies_from_ratchet` | port logic: wander heads home past the leash and shies from ratchet | fast | lib:classes | keep |
| `moby_update/creature/tests::the_sphere_hit_template_pushes_along_the_facing` | port logic: the sphere hit template pushes along the facing | fast | lib:classes | keep |
| `moby_update/interact::lease_is_two_ticks_and_owner_keyed` | port logic: lease is two ticks and owner keyed | fast | lib:classes | keep |
| `moby_update/interact::vendor_rule_distance_height_facing_and_state` | port logic: vendor rule distance height facing and state | fast | lib:classes | keep |
| `moby_update/interact::talk_rule_range_and_both_facings` | port logic: talk rule range and both facings | fast | lib:classes | keep |
| `moby_update/interact::diff_rots_wraps` | port logic: diff rots wraps | fast | lib:classes | keep |
| `moby_update/interact::spendable_gold_bolts` | port logic: spendable gold bolts | fast | lib:classes | keep |
| `moby_update/interact::node_parse` | port logic: node parse | fast | lib:classes | keep |
| `moby_update/services::rows_euler_inverts_euler_rows` | game-checked: `0x2721f0` inverts `0x221980` (R = Rz·Ry·Rx) for /y/ < π/2 [`0x221980`, `0x2721f0`] | fast | lib:classes | keep |
| `moby_update/services::rows_euler_quat_round_trip` | game-checked: The bolts' settle reads a tumbled bolt's rows back as a quaternion (`0x26ee30(0x2721f0(rows))`) [`0x26ee30`, `0x2721f0`] | fast | lib:classes | keep |
| `moby_update/services::part11_is_type11_spawn_draw_for_draw` | port logic: `World::part11` is `type11::spawn` (plus the spawn counters) | fast | lib:classes | keep |
| `moby_update/triggers::cuboid_faces_are_inclusive` | port logic: cuboid faces are inclusive | fast | lib:classes | keep |
| `moby_update/triggers::cylinder_sphere_and_path` | port logic: cylinder sphere and path | fast | lib:classes | keep |
| `moby_update/triggers::camera_grid_sphere_test` | port logic: camera grid sphere test | fast | lib:classes | keep |
| `moby_update/triggers::carry_block_round_trip` | port logic: carry block round trip | fast | lib:classes | keep |
| `moby_update/triggers::novalis_cuboids` | port logic: Novalis (from `extracted/`) (L01) | fast | lib:classes | keep |
| `movie_player::skip_rule` | port logic: skip rule | fast | lib:ui | keep |
| `movie_player::timeline_played_through` | port logic: timeline played through | fast | lib:ui | keep |
| `movie_player::audio_clock` | port logic: 30 fps | fast | lib:ui | keep |
| `movie_player::timeline_skipped` | port logic: timeline skipped | fast | lib:ui | keep |
| `pad::axis_dead_zone_and_scale` | port logic: axis dead zone and scale | fast | lib:hero | keep |
| `pad::arctan_matches_atan2` | game-checked: arctan matches atan2 | fast | lib:hero | keep |
| `pad::buttons_pressed_released_and_stick_bits` | port logic: buttons pressed released and stick bits | fast | lib:hero | keep |
| `pad::flick_from_rest` | port logic: flick from rest | fast | lib:hero | keep |
| `pad::mirror_swaps_left_right` | port logic: mirror swaps left right | fast | lib:hero | keep |
| `particles::allocator_takes_lowest_free_and_tracks_hw` | port logic: allocator takes lowest free and tracks hw | fast | lib:world | keep |
| `particles::allocator_scan_crosses_full_bytes_and_reports_full` | port logic: allocator scan crosses full bytes and reports full | fast | lib:world | keep |
| `particles::create_clears_only_the_first_half` | port logic: create clears only the first half | fast | lib:world | keep |
| `particles::update_parts_kills_unported_types_and_counts_them` | port logic: update parts kills unported types and counts them | fast | lib:world | keep |
| `particles::dec_timer_and_ticks` | port logic: dec timer and ticks | fast | lib:world | keep |
| `particles::bsphere_check_depth_and_sides` | port logic: bsphere check depth and sides | fast | lib:world | keep |
| `particles/type02::pack_round_trip_and_scale` | port logic: pack round trip and scale | fast | lib:world | keep |
| `particles/type02::three_phases_then_death` | port logic: three phases then death | fast | lib:world | keep |
| `particles/type04::grows_slows_and_fades` | port logic: grows slows and fades | fast | lib:world | keep |
| `particles/type06::live_owner_follows_the_moby` | port logic: live owner follows the moby | fast | lib:world | keep |
| `particles/type06::emitter_spawns_one_per_tick_with_novalis_pvars` | port logic: emitter spawns one per tick with novalis pvars (L01) | fast | lib:world | keep |
| `particles/type06::particle_lives_150_ticks_and_follows_the_pvars` | game-checked: particle lives 150 ticks and follows the pvars | fast | lib:world | keep |
| `particles/type06::clamp_length_rescales_only_when_too_long` | port logic: clamp length rescales only when too long | fast | lib:world | keep |
| `particles/type06::add_rotations_wraps_once` | port logic: add rotations wraps once | fast | lib:world | keep |
| `particles/type06::culled_emitter_does_not_draw_random_numbers` | port logic: culled emitter does not draw random numbers | fast | lib:world | keep |
| `particles/type06::load_pass_view_culls_on_novalis_only` | port logic: load pass view culls on novalis only (L01) | fast | lib:world | keep |
| `particles/type08::grows_and_fades_out_in_the_last_six_ticks` | port logic: grows and fades out in the last six ticks | fast | lib:world | keep |
| `particles/type11::spawner_writes_the_record_and_draws_five` | port logic: spawner writes the record and draws five | fast | lib:world | keep |
| `particles/type11::generation_0_spark_splits_into_five_on_the_fourth_update` | port logic: generation 0 spark splits into five on the fourth update | fast | lib:world | keep |
| `particles/type11::generation_2_spark_turns_to_smoke_and_dies` | port logic: generation 2 spark turns to smoke and dies | fast | lib:world | keep |
| `particles/type11::curve_size_starts_at_zero_and_grows` | game-checked: curve size starts at zero and grows | fast | lib:world | keep |
| `particles/type12::flies_its_length_then_stops` | port logic: A flame launched at 16 u/s with 12 units to go keeps its speed, then slows to rest at its end, and rises 1 u/s. | fast | lib:world | keep |
| `particles/type12::life_size_and_bounds` | port logic: The life counts down (twice as fast at rest) | fast | lib:world | keep |
| `particles/type12::spawn_draw_order_and_record` | port logic: spawn draw order and record | fast | lib:world | keep |
| `particles/type13::dust_fades_in_then_out_over_40_updates_without_rng` | port logic: dust fades in then out over 40 updates without rng | fast | lib:world | keep |
| `particles/type15::split_streak_spawns_children_on_odd_ticks` | port logic: split streak spawns children on odd ticks | fast | lib:world | keep |
| `particles/type16::falls_fades_and_dies_on_the_floor` | port logic: falls fades and dies on the floor | fast | lib:world | keep |
| `particles/type16::kind_one_leaves_rising_smoke` | port logic: kind one leaves rising smoke | fast | lib:world | keep |
| `particles/type16::fades_near_the_camera` | port logic: fades near the camera | fast | lib:world | keep |
| `particles/type19::a_streak_moves_and_fades` | port logic: a streak moves and fades | fast | lib:world | keep |
| `particles/type19::a_joint_ribbon_trails_its_joint` | port logic: a joint ribbon trails its joint | fast | lib:world | keep |
| `particles/type21::shrinks_tweens_and_splits_on_odd_ticks` | port logic: shrinks tweens and splits on odd ticks | fast | lib:world | keep |
| `particles/type22::grows_rises_and_fades_over_its_life` | port logic: grows rises and fades over its life | fast | lib:world | keep |
| `particles/type23::fades_in_then_out_and_grows` | port logic: fades in then out and grows | fast | lib:world | keep |
| `particles/type25::spark_cools_falls_and_dies_on_the_timer` | port logic: A spark with the grind's velocity in the open air | fast | lib:world | keep |
| `particles/type25::variant_swaps_the_rates` | port logic: The variant cools G slowly and R fast (and keeps the negated channels). | fast | lib:world | keep |
| `particles/type26::follows_its_moby_fades_and_dies_with_it` | port logic: follows its moby fades and dies with it | fast | lib:world | keep |
| `particles/type27::moves_fades_and_dies` | port logic: Moves by its velocity, fades linearly from the colour's alpha, dies with its timer. | fast | lib:world | keep |
| `particles/type32::pulses_follows_and_dies` | port logic: The pulse | fast | lib:world | keep |
| `particles/type34::bubble_rises_and_pops_at_the_surface` | port logic: A bubble deep under the surface rises (its vertical speed reaching 1.3 u/s), grows to its cap and draws nothing | fast | lib:world | keep |
| `particles/type35::a_kind_zero_drop_falls_then_fades_below_its_start` | port logic: a kind zero drop falls then fades below its start | fast | lib:world | keep |
| `particles/type35::a_kind_three_drop_rings_on_the_water` | port logic: a kind three drop rings on the water | fast | lib:world | keep |
| `particles/type44::drifts_grows_fades` | port logic: drifts grows fades | fast | lib:world | keep |
| `particles/type45::rings_fade_one_alpha_step_per_period` | port logic: rings fade one alpha step per period | fast | lib:world | keep |
| `particles/type45::rings_ride_the_hero_water_level` | game-checked: The hero's water-level pointer [`0x13f640`] | fast | lib:world | keep |
| `particles/type46::spreads_and_fades_one_step_a_tick_after_five` | port logic: spreads and fades one step a tick after five | fast | lib:world | keep |
| `particles/type47::puff_fades_in_22_updates` | port logic: Alpha 0x28 + 3 = 0x2b | fast | lib:world | keep |
| `particles/type52::flat_drip_spreads` | port logic: flat drip spreads | fast | lib:world | keep |
| `particles/type53::sparkle_shrinks_and_fades_over_25_updates` | port logic: sparkle shrinks and fades over 25 updates | fast | lib:world | keep |
| `particles/type53::sparkle_dies_outside_the_level_box` | port logic: sparkle dies outside the level box | fast | lib:world | keep |
| `particles/type56::puff_greys_out` | port logic: Grey 0x60 + 2·k fades by 4 a tick | fast | lib:world | keep |
| `particles/type57::ring_lives_128_updates` | port logic: 128 updates, the alpha rising to 64 at t = 64 and falling back to 0. | fast | lib:world | keep |
| `particles/type59::follows_the_hero_and_dies_on_its_timer` | port logic: A two-tick sparkle attached to the hero follows him and dies on its second update. | fast | lib:world | keep |
| `particles/type60::glint_fades_over_its_life` | port logic: A swing target's glint | fast | lib:world | keep |
| `particles/type62::puffs_fade_orbs_stay_anchored_follow` | port logic: A trail puff (kind 0, 20 ticks) | fast | lib:world | keep |
| `particles/type64::falls_and_bursts_into_a_streak` | port logic: falls and bursts into a streak | fast | lib:world | keep |
| `particles/type72::spins_and_lives_while_it_has_a_size` | port logic: spins and lives while it has a size | fast | lib:world | keep |
| `path::pose_open_path` | port logic: A straight-then-up open path | fast | lib:classes | keep |
| `path::pose_closed_path` | port logic: A closed square | fast | lib:classes | keep |
| `path::push_from_walls_moves_a_near_point` | port logic: A wall along x from 0 to 10 (w ≠ 0 on both points) and a gap segment after it. | fast | lib:classes | keep |
| `path::nearest_at_distance_picks_the_first_best` | port logic: nearest at distance picks the first best | fast | lib:classes | keep |
| `point_lights::first_free_slot_and_load_gate` | port logic: first free slot and load gate | fast | lib:world | keep |
| `point_lights::nibble_lists_fill_from_the_low_end_and_compact` | port logic: nibble lists fill from the low end and compact | fast | lib:world | keep |
| `point_lights::overlap_is_strict` | port logic: overlap is strict | fast | lib:world | keep |
| `point_lights::attach_margin_move_threshold_and_free` | port logic: attach margin move threshold and free | fast | lib:world | keep |
| `point_lights::list_capacity_is_shared_ties_first` | port logic: list capacity is shared ties first | fast | lib:world | keep |
| `ps2v::ftoi0_truncates_toward_zero` | port logic: ftoi0 truncates toward zero | fast | lib:world | keep |
| `ps2v::cross_and_dots` | port logic: cross and dots | fast | lib:world | keep |
| `rng::first_ten_after_level_seed_follow_newlib` | port logic: first ten after level seed follow newlib | fast | lib:world | keep |
| `rng::helpers_on_level_seed` | port logic: Every helper on seed 1234 | fast | lib:world | keep |
| `scene_player::start_sequence_fade_black_and_speech_lead` | port logic: start sequence fade black and speech lead | fast | lib:ui | keep |
| `scene_player::fade_clears_in_three_ticks` | port logic: fade clears in three ticks | fast | lib:ui | keep |
| `scene_player::chunk_rollover_camera_continuity_and_actor_lerp` | port logic: chunk rollover camera continuity and actor lerp | fast | lib:ui | keep |
| `scene_player::end_fade_and_hand_back` | port logic: end fade and hand back | fast | lib:ui | keep |
| `scene_player::skip_gating` | port logic: skip gating | fast | lib:ui | keep |
| `scene_player::camera_rows_are_rz_ry_rx_columns` | port logic: camera rows are rz ry rx columns | fast | lib:ui | keep |
| `scene_player::ecossin_matches_libm` | port logic: ecossin matches libm | fast | lib:ui | keep |
| `shadows::direction_zero_leans_along_the_light` | game-checked: direction zero leans along the light [`0x1af000`] | fast | lib:world | keep |
| `shadows::light_dir_cross_fades_set_a_into_b` | port logic: light dir cross fades set a into b | fast | lib:world | keep |
| `shadows::hero_slab_from_the_seed_and_the_four_probes` | game-checked: hero slab from the seed and the four probes [`0x1af010`] | fast | lib:world | keep |
| `shadows::hero_pitch_slides_under_him_in_the_air` | port logic: hero pitch slides under him in the air | fast | lib:world | keep |
| `shadows::class_slab_rules` | port logic: class slab rules | fast | lib:world | keep |
| `shadows/volume::size_by_distance` | game-checked: size by distance | fast | lib:world | keep |
| `shadows/volume::budget_stops_after_the_area_is_passed` | port logic: budget stops after the area is passed | fast | lib:world | keep |
| `shadows/volume::posing_scales_points_and_radius` | port logic: posing scales points and radius | fast | lib:world | keep |
| `shadows/volume::dir_frame_is_orthonormal` | port logic: dir frame is orthonormal | fast | lib:world | keep |
| `shadows/volume::sphere_outline_is_an_ngon_turning_about_the_direction` | port logic: sphere outline is an ngon turning about the direction | fast | lib:world | keep |
| `shadows/volume::capsule_outline_arcs_and_tangents` | port logic: capsule outline arcs and tangents | fast | lib:world | keep |
| `shadows/volume::prisms_are_closed_outward_and_clipped_to_the_slab` | port logic: prisms are closed outward and clipped to the slab | fast | lib:world | keep |
| `shadows/volume::game_asin` | port logic: game asin | fast | lib:world | keep |
| `sky_stars::counts_per_level` | port logic: Counts per level (kinds 0..3) and the levels without stars. | fast | lib:world | keep |
| `sky_stars::stream_use_matches_the_generators` | port logic: `rand` draws per record | fast | lib:world | keep |
| `sky_stars::deterministic_given_a_seed` | determinism: Same seed → identical records and stream | fast | lib:world | keep |
| `sky_stars::generic_records` | port logic: Record contents | fast | lib:world | keep |
| `sky_stars::twinkle_is_per_frame_noise` | port logic: Kind 1 twinkle | fast | lib:world | keep |
| `sky_stars::fixed_star_pulse_period` | port logic: Kind 2 (06) | fast | lib:world | keep |
| `sky_stars::moving_star_lap_and_blink` | port logic: Kind 0 | fast | lib:world | keep |
| `spline::step_index_clamps_or_wraps` | port logic: step index clamps or wraps | fast | lib:classes | keep |
| `spline::advance_straight` | port logic: A straight open spline | fast | lib:classes | keep |
| `spline::advance_closed_loop` | port logic: A closed square | fast | lib:classes | keep |
| `spline::nearest_straight` | port logic: The nearest point on a straight spline, inside and outside the vertical band. | fast | lib:classes | keep |
| `spline::nearest_and_ride_curved` | port logic: A curved (quarter-circle) spline | fast | lib:classes | keep |
| `targeting::missile_mark_is_bounded` | port logic: The missiles' mark | fast | lib:weapons | keep |
| `targeting::the_list_and_the_record` | port logic: the list and the record | fast | lib:weapons | keep |
| `targeting::bomb_glove_search_rules` | port logic: The glove's cones | fast | lib:weapons | keep |
| `targeting::arc_lands_on_the_ground` | port logic: Flat ground at z = 0 | fast | lib:weapons | keep |
| `targeting::arc_snaps_to_a_moby` | port logic: A moby's primitive on the arc snaps the point to the moby's position | fast | lib:weapons | keep |
| `targeting::reticle_placement` | port logic: The quads lie in the plane of the normal (up | fast | lib:weapons | keep |
| `targeting::pulled_toward_the_camera_and_registered_per_tick` | port logic: pulled toward the camera and registered per tick | fast | lib:weapons | keep |
| `water::scroll_wraps_each_lane_once` | port logic: scroll wraps each lane once | fast | lib:world | keep |
| `water::wobble_groups_and_values` | port logic: wobble groups and values | fast | lib:world | keep |
| `water::bob_period_and_range` | port logic: bob period and range | fast | lib:world | keep |
| `water::init_761_rungs_alternate` | port logic: init 761 rungs alternate | fast | lib:world | keep |
| `water::wave_step_on_a_3x3_impulse` | port logic: wave step on a 3x3 impulse | fast | lib:world | keep |
| `water::damping_threshold` | port logic: damping threshold | fast | lib:world | keep |
| `water::interpolation_ticks_0_to_8` | port logic: interpolation ticks 0 to 8 | fast | lib:world | keep |
| `water::disturb_and_height_query` | port logic: disturb and height query | fast | lib:world | keep |
| `water::flat_water_is_grey_139` | port logic: flat water is grey 139 | fast | lib:world | keep |
| `water::halo_mirror_between_linked_patches` | port logic: halo mirror between linked patches | fast | lib:world | keep |
| `water::foam_scroll_wraps_at_minus_8` | port logic: foam scroll wraps at minus 8 | fast | lib:world | keep |
| `water/sea::wrap_tests_each_bound_once` | port logic: wrap tests each bound once | fast | lib:world | keep |
| `water/sea::bob_is_the_games_sine` | game-checked: bob is the games sine | fast | lib:world | keep |
| `water/sea::hoven_rescale_sets_rgb_and_halves_alpha` | port logic: hoven rescale sets rgb and halves alpha (L12) | fast | lib:world | keep |
| `water/sea::ports_are_unique` | port logic: ports are unique | fast | lib:world | keep |

### rc-formats: integration tests (33)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `golden::toc_level_table` | data/format: toc level table (disc data) | fast | formats | keep |
| `golden::wad_decompression_for_every_lump` | data/format: wad decompression for every lump (disc data) | medium | formats | keep |
| `golden::core_index_blocks_tile_the_data` | data/format: core index blocks tile the data (all levels) | medium | formats | keep |
| `golden::gameplay_sections_for_every_level` | data/format: The gameplay file's pointer-table sections (`gameplay::sections`), which tests read `level_settings` through. (all levels) | medium | formats | keep |
| `golden::textures_for_every_level` | data/format: Every level texture (tfrag, moby, tie, shrub tables and shrub billboards) decoded to RGBA, in table order (all levels) | medium | formats | keep |
| `golden::tfrags_for_every_level` | data/format: tfrags for every level (all levels) | medium | formats | keep |
| `golden::mobys_for_every_level` | data/format: mobys for every level [`0x165500`] (all levels) | medium | formats | keep |
| `golden::ties_for_every_level` | data/format: ties for every level (all levels) | medium | formats | keep |
| `golden::sky_for_every_level` | data/format: sky for every level (all levels) | medium | formats | keep |
| `golden::shrubs_for_every_level` | data/format: shrubs for every level (all levels) | medium | formats | keep |
| `golden::tfrag_texture_mip_chains` | data/format: Every tfrag texture is square, and every mip level `decode_tfrag_mip_levels` finds (`ty` levels (all levels) | medium | formats | keep |
| `golden::tfrag_lod_links_are_well_formed` | data/format: The LOD linkage the renderer's morph / collapse passes rely on (tfrag_rac1.md §3b.3, §3b.6), on every retail tfrag (all levels) | medium | formats | keep |
| `golden::instance_classes_and_tie_draw_distances` | data/format: Instance records against their classes on every level (all levels) | medium | formats | keep |
| `golden::collision_for_every_level` | data/format: collision for every level (all levels) | medium | formats | keep |
| `golden::occlusion_for_every_level` | data/format: occlusion for every level (all levels) | medium | formats | keep |
| `golden::gadgets_for_every_level` | data/format: Gadget classes (docs/formats/moby_rac1.md 0.4) (all levels) | medium | formats | keep |
| `golden::particle_textures_for_every_level` | data/format: Particle and FX textures (all levels) | medium | formats | keep |
| `golden::sound_banks_for_every_level` | data/format: Sound banks [`0x209800`] (all levels) | medium | formats | keep |
| `golden::hud_fonts_and_strings_for_every_level` | data/format: HUD tables, every decoded frame, the glyph tables (found through the font wrappers) and the English messages, for… [`0x1c35d0`, `0x1c3970`] (all levels) | medium | formats | keep |
| `golden::scenes_for_every_level` | data/format: Scene tables and chunks (`rc_formats::scene`) for all 19 levels, NTSC and PAL: the Rust path (level header →… (all levels) | medium | formats | keep |
| `level_overlay_disc::every_overlay_has_the_seven_sections_and_a_class_table` | data/format: every overlay has the seven sections and a class table [`0x15ef00`] (all levels) | medium | formats | keep |
| `level_overlay_disc::relocation_finds_level13_copies_of_level01_code_and_data` | game-checked: relocation finds level13 copies of level01 code and data [`0x161e08`, `0x161fe0`] (disc data) | fast | formats | keep |
| `moby_anim_golden::every_sequence_parses_with_consistent_headers` | data/format: every sequence parses with consistent headers (all levels) | medium | formats | keep |
| `moby_anim_golden::class66_spinner_matches_the_worked_example` | game-checked: class66 spinner matches the worked example [`0x20ed18`] (disc data) | medium | formats | keep |
| `moby_anim_golden::class747_frame1_matches_the_worked_example` | game-checked: class747 frame1 matches the worked example (disc data) | medium | formats | keep |
| `moby_anim_golden::bind_pose_palette_is_identity` | port logic: bind pose palette is identity (disc data) | medium | formats | keep |
| `moby_anim_golden::post_scale_quirk_on_level00_class608` | port logic: post scale quirk on level00 class608 (disc data) | medium | formats | keep |
| `moby_collision_disc::every_level_moby_collision_blob_parses` | data/format: every level moby collision blob parses (all levels) | medium | formats | keep |
| `moby_shadow_disc::every_level_shadow_block_parses` | data/format: every level shadow block parses (all levels) | medium | formats | keep |
| `occlusion_frames::novalis_frame_masks` | game-checked: novalis frame masks (L01) | fast | formats | keep |
| `scene_coverage::every_scene_on_the_disc_is_understood` | game-checked: every scene on the disc is understood (all levels) | medium | formats | keep |
| `tfrag_light_golden::normal_table_boot_copy_matches_level01_overlay` | data/format: normal table boot copy matches level01 overlay [`0x166500`, `0x2a8e40`] (disc data) | medium | formats | keep |
| `tfrag_light_golden::light_records_match_tfrag_layout_in_every_level` | data/format: light records match tfrag layout in every level (all levels) | medium | formats | keep |

### rc-formats: unit tests (147)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `collision::packed_vertex_decode_is_signed_fixed_point` | data/format: packed vertex decode is signed fixed point | fast | lib:formats | keep |
| `collision::cell_walk_and_triangles` | port logic: cell walk and triangles | fast | lib:formats | keep |
| `collision::rejects_corrupt_leaves` | port logic: rejects corrupt leaves | fast | lib:formats | keep |
| `disc::reads_toc_boot_elf_and_level_lumps_from_a_synthetic_disc` | data/format: reads toc boot elf and level lumps from a synthetic disc | fast | lib:formats | keep |
| `disc::global_lumps_and_archive_plan_follow_the_cpp_unpack_rules` | data/format: global lumps and archive plan follow the cpp unpack rules | fast | lib:formats | keep |
| `font::call_pattern_finds_tables_and_sign_extends_lo` | data/format: call pattern finds tables and sign extends lo [`0x100000`, `0x1c0000`] | fast | lib:formats | keep |
| `font::measure_counts_nonzero_advances` | port logic: measure counts nonzero advances | fast | lib:formats | keep |
| `gameplay::parses_synthetic_section` | data/format: parses synthetic section | fast | lib:formats | keep |
| `gameplay::pvar_fixups_as_the_loader` | port logic: pvar fixups as the loader | fast | lib:formats | keep |
| `gameplay::splines_and_ship_placement` | port logic: splines and ship placement | fast | lib:formats | keep |
| `gameplay::fog_zones_synthetic` | game-checked: fog zones synthetic [`0x261919`, `0x282828`] | fast | lib:formats | keep |
| `gameplay::fog_zones_novalis_disc` | game-checked: Novalis (level 01, NTSC gameplay) [`0x261919`, `0x282828`] (L01) | fast | lib:formats | keep |
| `hud::parse_lookup_and_decode` | data/format: parse lookup and decode | fast | lib:formats | keep |
| `iso9660::reads_a_synthetic_2048_byte_image` | port logic: reads a synthetic 2048 byte image | fast | lib:formats | keep |
| `iso9660::reads_a_synthetic_raw_2352_byte_image` | port logic: reads a synthetic raw 2352 byte image | fast | lib:formats | keep |
| `iso9660::rejects_images_without_a_pvd` | port logic: rejects images without a pvd | fast | lib:formats | keep |
| `level_overlay::masking_follows_lui_through_moves_and_keeps_constants` | game-checked: masking follows lui through moves and keeps constants [`0x1742c0`, `0x1af000`] | fast | lib:formats | keep |
| `moby::normal_axis_convention` | port logic: normal axis convention | fast | lib:formats | keep |
| `moby::vertex_cache_ids_shift_by_seven` | port logic: vertex cache ids shift by seven | fast | lib:formats | keep |
| `moby::slot_machine_resolves_weights_summing_to_256` | port logic: slot machine resolves weights summing to 256 | fast | lib:formats | keep |
| `moby::index_walk_secret_indices_and_trailer` | port logic: index walk secret indices and trailer | fast | lib:formats | keep |
| `moby_anim::class66_quaternion_decode_and_rows` | data/format: class66 quaternion decode and rows | fast | lib:formats | keep |
| `moby_anim::plain_lerp_is_not_normalised_and_wrap_flips` | port logic: plain lerp is not normalised and wrap flips | fast | lib:formats | keep |
| `moby_anim::class747_frame1_post_scale_and_chain` | port logic: class747 frame1 post scale and chain | fast | lib:formats | keep |
| `moby_anim::advance_from_spawn_and_wrap` | port logic: advance from spawn and wrap | fast | lib:formats | keep |
| `moby_anim::snap_window` | port logic: snap window | fast | lib:formats | keep |
| `moby_anim::post_scale_list_drops_a_only_joints` | port logic: post scale list drops a only joints | fast | lib:formats | keep |
| `moby_anim::hard_cut_clamps_and_takes_the_frame_rate` | port logic: hard cut clamps and takes the frame rate | fast | lib:formats | keep |
| `moby_anim::blend_with_zero_ticks_lands_on_the_next_advance` | port logic: blend with zero ticks lands on the next advance | fast | lib:formats | keep |
| `moby_anim::snapshot_of_a_key_re_encodes_it_exactly` | data/format: snapshot of a key re encodes it exactly | fast | lib:formats | keep |
| `moby_anim::blend_from_mid_key_starts_from_the_snapshot` | data/format: blend from mid key starts from the snapshot | fast | lib:formats | keep |
| `moby_anim::chain_equals_full_pose_without_skeleton_and_post_scale` | game-checked: chain equals full pose without skeleton and post scale | fast | lib:formats | keep |
| `moby_anim::attach_matrix_and_normalise` | port logic: attach matrix and normalise | fast | lib:formats | keep |
| `moby_anim::ratchet_hand_chain_matches_full_evaluator` | port logic: Ratchet (class 0) on Novalis with his `ratchet_seq` sequences (L01) | fast | lib:formats | keep |
| `moby_anim::compose_mode` | port logic: Mode 0 composes | fast | lib:formats | keep |
| `moby_anim::blend_mode_and_limit` | port logic: Mode 1 blends by the weight | fast | lib:formats | keep |
| `moby_anim::list_target_is_the_second_lists_first_joint` | port logic: list target is the second lists first joint | fast | lib:formats | keep |
| `moby_collision::parses_sections_and_primitive_fields` | data/format: parses sections and primitive fields | fast | lib:formats | keep |
| `moby_collision::rejects_bad_lists_and_indices` | port logic: rejects bad lists and indices | fast | lib:formats | keep |
| `moby_light::sine_polynomial_matches_sin_and_folds` | port logic: sine polynomial matches sin and folds | fast | lib:formats | keep |
| `moby_light::rotation_convention_is_rz_ry_rx` | port logic: rotation convention is rz ry rx | fast | lib:formats | keep |
| `moby_light::lighting_of_up_facing_vertex` | port logic: lighting of up facing vertex | fast | lib:formats | keep |
| `moby_light::rotation_moves_light_into_model_space` | port logic: rotation moves light into model space | fast | lib:formats | keep |
| `moby_light::pack_saturates_like_the_ee` | port logic: pack saturates like the ee | fast | lib:formats | keep |
| `moby_shadow::parses_spheres_and_capsules_until_last` | data/format: parses spheres and capsules until last | fast | lib:formats | keep |
| `moby_shadow::block_sits_before_the_skeleton` | port logic: block sits before the skeleton | fast | lib:formats | keep |
| `moby_spawn::plain_classes_play_sequence_0_advanced_once` | port logic: plain classes play sequence 0 advanced once | fast | lib:formats | keep |
| `moby_spawn::enemy_459_moves_to_its_path_and_hides_in_the_air` | port logic: enemy 459 moves to its path and hides in the air | fast | lib:formats | keep |
| `moby_spawn::amoeboids_big_walk_small_hide_underground` | data/format: amoeboids big walk small hide underground | fast | lib:formats | keep |
| `moby_spawn::critter_577_sequence_4_or_hover` | port logic: critter 577 sequence 4 or hover | fast | lib:formats | keep |
| `moby_spawn::dropship_npcs_infobot_mouse` | port logic: dropship npcs infobot mouse | fast | lib:formats | keep |
| `moby_spawn::ship_index_follows_the_destination_planet` | port logic: ship index follows the destination planet | fast | lib:formats | keep |
| `moby_spawn::spaceship_file_layout` | data/format: spaceship file layout | fast | lib:formats | keep |
| `moby_spawn::ship_is_cut_to_sequence_1_then_advanced` | port logic: ship is cut to sequence 1 then advanced | fast | lib:formats | keep |
| `moby_spawn::spawn_test_follows_the_loader_branches` | game-checked: spawn test follows the loader branches [`0x1bbb04`] | fast | lib:formats | keep |
| `moby_spawn::ship_is_hidden_while_the_mission_npc_mission_is_open` | port logic: ship is hidden while the mission npc mission is open | fast | lib:formats | keep |
| `occlusion::tree_walk_and_cell_mapping` | port logic: tree walk and cell mapping | fast | lib:formats | keep |
| `occlusion::bits_and_fallbacks` | data/format: bits and fallbacks | fast | lib:formats | keep |
| `occlusion::mapping_resolution_rules` | port logic: mapping resolution rules | fast | lib:formats | keep |
| `particle_tex::part_defs_null_rule_and_runs` | port logic: part defs null rule and runs | fast | lib:formats | keep |
| `particle_tex::parse_part_defs_reads_header_offsets_and_blob` | data/format: parse part defs reads header offsets and blob | fast | lib:formats | keep |
| `particle_tex::runtime_words_pack_address_csa_and_log2` | port logic: runtime words pack address csa and log2 | fast | lib:formats | keep |
| `particle_tex::bank_texture_uses_its_own_clut` | port logic: bank texture uses its own clut | fast | lib:formats | keep |
| `pif::parses_a_synthetic_pif` | data/format: parses a synthetic pif | fast | lib:formats | keep |
| `pss::demux_synthetic_stream` | data/format: A synthetic program stream | fast | lib:formats | keep |
| `pss::audio_deinterleave_and_decode` | data/format: De-interleaving | fast | lib:formats | keep |
| `pss::real_movies_demux` | data/format: Real movies (Tier 0 `global/mpegs`) (L01) | fast | lib:formats | keep |
| `save_game::crc_matches_the_textbook_form_and_its_limits` | data/format: crc matches the textbook form and its limits | fast | lib:formats | keep |
| `save_game::crc_known_vector` | data/format: crc known vector | fast | lib:formats | keep |
| `save_game::section_round_trip_with_pads_and_terminator` | data/format: section round trip with pads and terminator | fast | lib:formats | keep |
| `save_game::section_size_is_get_data_size` | port logic: section size is get data size | fast | lib:formats | keep |
| `save_game::incremental_write_touches_global_and_one_level_only` | port logic: incremental write touches global and one level only | fast | lib:formats | keep |
| `save_game::tables_and_template_from_the_disc` | data/format: Descriptor tables, template sizes / CRCs / round trip, and "template = boot ELF initial data". Skipped without… (disc data) | fast | lib:formats | keep |
| `save_game::item_tables_from_every_overlay` | data/format: The item tables load from every overlay [`0x179ac0`, `0x179f40`] (all levels) | fast | lib:formats | keep |
| `save_game::card_dir_from_system_cnf` | port logic: card dir from system cnf | fast | lib:formats | keep |
| `scene::parses_a_synthetic_chunk` | data/format: parses a synthetic chunk | fast | lib:formats | keep |
| `scene::scene_camera_and_chunk_rollover` | port logic: scene camera and chunk rollover | fast | lib:formats | keep |
| `scene::region_table_slices_the_region_file` | data/format: region table slices the region file | fast | lib:formats | keep |
| `sea::liquid_grid_record_layout` | data/format: The grid state record's fields at their offsets, the block tables behind its pointers. | fast | lib:formats | keep |
| `sha1::standard_vectors` | port logic: FIPS 180-2 appendix A / NIST CAVP short-message vectors. | fast | lib:formats | keep |
| `sha1::padding_boundaries_and_incremental_updates_agree` | port logic: padding boundaries and incremental updates agree | fast | lib:formats | keep |
| `shrub::walker_follows_vu1_rules` | port logic: walker follows vu1 rules | fast | lib:formats | keep |
| `shrub::stop_bit_rules` | port logic: stop bit rules | fast | lib:formats | keep |
| `shrub::texture_state_carries_across_packets` | port logic: texture state carries across packets | fast | lib:formats | keep |
| `shrub::malformed_packets_are_rejected` | port logic: malformed packets are rejected | fast | lib:formats | keep |
| `shrub::ad_gif_conversion_is_the_tfrag_rule` | port logic: ad gif conversion is the tfrag rule | fast | lib:formats | keep |
| `shrub::billboard_registers_and_quad` | game-checked: billboard registers and quad | fast | lib:formats | keep |
| `shrub::fade_rules` | port logic: fade rules | fast | lib:formats | keep |
| `shrub::sway` | game-checked: sway [`0x100000`] | fast | lib:formats | keep |
| `shrub::instance_helpers` | port logic: instance helpers | fast | lib:formats | keep |
| `shrub_light::vu1_quirk_addresses` | port logic: vu1 quirk addresses | fast | lib:formats | keep |
| `shrub_light::normals_decode_by_32768` | data/format: normals decode by 32768 | fast | lib:formats | keep |
| `shrub_light::one_light_and_back_factor` | port logic: one light and back factor | fast | lib:formats | keep |
| `shrub_light::clamp_is_243_and_alpha_is_0x80` | port logic: clamp is 243 and alpha is 0x80 | fast | lib:formats | keep |
| `shrub_light::blend_scales_xyz_and_sums_back_factors` | port logic: blend scales xyz and sums back factors | fast | lib:formats | keep |
| `shrub_light::light_follows_rotation_mirror_and_ignores_scale` | port logic: light follows rotation mirror and ignores scale | fast | lib:formats | keep |
| `shrub_light::point_lights_merge_at_the_bounding_sphere_centre` | port logic: point lights merge at the bounding sphere centre | fast | lib:formats | keep |
| `shrub_light::average_is_integer_mean` | port logic: average is integer mean | fast | lib:formats | keep |
| `shrub_light::lights_every_retail_instance` | data/format: Lights every shrub instance of every level (skipped without `extracted/`) (all levels) | fast | lib:formats | keep |
| `sky::parses_shells_and_walks_faces` | data/format: parses shells and walks faces | fast | lib:formats | keep |
| `sky::decodes_textures_with_clut_order_and_alpha` | data/format: decodes textures with clut order and alpha | fast | lib:formats | keep |
| `sky::rejects_bad_data` | port logic: rejects bad data | fast | lib:formats | keep |
| `sound_bank::parses_a_synthetic_bank` | data/format: parses a synthetic bank | fast | lib:formats | keep |
| `sound_bank::resolve_sound_by_owner` | port logic: resolve sound by owner | fast | lib:formats | keep |
| `strings::parse_and_lookup_first_match` | data/format: parse and lookup first match | fast | lib:formats | keep |
| `texture::clut_index_swaps_middle_blocks` | port logic: clut index swaps middle blocks | fast | lib:formats | keep |
| `texture::scale_alpha_maps_0x80_to_opaque` | port logic: scale alpha maps 0x80 to opaque | fast | lib:formats | keep |
| `texture::decode_indexed8_applies_clut_order_and_alpha` | data/format: decode indexed8 applies clut order and alpha | fast | lib:formats | keep |
| `tfrag::ad_gif_fields_decode` | data/format: ad gif fields decode | fast | lib:formats | keep |
| `tfrag::ad_gif_gs_registers_match_init` | port logic: ad gif gs registers match init | fast | lib:formats | keep |
| `tfrag::kick_records_with_z_switch_texture` | port logic: kick records with z switch texture | fast | lib:formats | keep |
| `tfrag::texture_sphere_decode` | data/format: texture sphere decode | fast | lib:formats | keep |
| `tfrag::lod_distances_and_draw_mode` | port logic: lod distances and draw mode | fast | lib:formats | keep |
| `tfrag::lod_links_follow_parent_arrays` | port logic: lod links follow parent arrays | fast | lib:formats | keep |
| `tfrag_light::ps2_arithmetic_truncates` | port logic: ps2 arithmetic truncates | fast | lib:formats | keep |
| `tfrag_light::normal_decode` | data/format: normal decode | fast | lib:formats | keep |
| `tfrag_light::one_light_evaluation` | port logic: one light evaluation | fast | lib:formats | keep |
| `tfrag_light::point_light_falloff` | port logic: point light falloff | fast | lib:formats | keep |
| `tie::walker_follows_vu1_rules` | port logic: walker follows vu1 rules | fast | lib:formats | keep |
| `tie::winding_flag_flips_parity` | port logic: winding flag flips parity | fast | lib:formats | keep |
| `tie::dinky_double_phase_uses_marker_plus_four` | port logic: dinky double phase uses marker plus four | fast | lib:formats | keep |
| `tie::malformed_packets_are_rejected` | port logic: malformed packets are rejected | fast | lib:formats | keep |
| `tie::instance_helpers` | port logic: instance helpers | fast | lib:formats | keep |
| `tie::novalis_fat_vertices_morph_onto_the_next_lod` | game-checked: Novalis (skipped without `extracted/`) (L01) | fast | lib:formats | keep |
| `tie::novalis_draw_distances_are_integers_up_to_720` | data/format: Novalis (skipped without `extracted/`) (L01) | fast | lib:formats | keep |
| `tie_light::identity_instance_matches_the_tfrag_rules` | port logic: identity instance matches the tfrag rules | fast | lib:formats | keep |
| `tie_light::clamp_is_243_not_255` | port logic: clamp is 243 not 255 | fast | lib:formats | keep |
| `tie_light::light_follows_the_instance_rotation_and_ignores_scale` | port logic: light follows the instance rotation and ignores scale | fast | lib:formats | keep |
| `tie_light::blended_set_and_unused_set` | port logic: blended set and unused set | fast | lib:formats | keep |
| `tie_light::point_lights_merge_into_the_third_light` | port logic: point lights merge into the third light | fast | lib:formats | keep |
| `tie_light::lights_every_retail_instance` | data/format: Lights every tie instance of every level (skipped without `extracted/`) (all levels) | fast | lib:formats | keep |
| `vag::zero_frame_decodes_to_zeros_and_low_nibble_is_first` | data/format: zero frame decodes to zeros and low nibble is first | fast | lib:formats | keep |
| `vag::rounded_and_opengoal_variants` | data/format: rounded and opengoal variants | fast | lib:formats | keep |
| `vag::extent_follows_flags` | port logic: extent follows flags | fast | lib:formats | keep |
| `vag::note_table_and_pitch` | data/format: note table and pitch | fast | lib:formats | keep |
| `vag::vag_header` | data/format: vag header | fast | lib:formats | keep |
| `vif::decodes_code_fields` | data/format: decodes code fields | fast | lib:formats | keep |
| `vif::unpack_sizes_and_num_zero` | port logic: unpack sizes and num zero | fast | lib:formats | keep |
| `vif::non_unpack_payloads` | port logic: non unpack payloads | fast | lib:formats | keep |
| `vif::truncated_payload_is_an_error_and_trailing_bytes_are_ignored` | port logic: truncated payload is an error and trailing bytes are ignored | fast | lib:formats | keep |
| `volumes::synthetic_sections` | port logic: A gameplay file with one cuboid (centre (10, 20, 5), half sizes (4, 2, 3)), no spheres, two cylinders and one pill… | fast | lib:formats | keep |
| `volumes::all_levels_disc` | data/format: Every level of the disc (from `extracted/`) (all levels) | fast | lib:formats | keep |
| `wad::literal_then_little_match` | game-checked: literal then little match | fast | lib:formats | keep |
| `wad::medium_match_with_little_literal` | game-checked: medium match with little literal | fast | lib:formats | keep |
| `wad::dummy_packet_and_pad` | game-checked: dummy packet and pad | fast | lib:formats | keep |
| `water::overlay_walk_and_read` | port logic: overlay walk and read | fast | lib:formats | keep |
| `water::novalis_water_tables` | data/format: The Novalis tables match docs/plan/world_animation.md and every FX texture they name exists. [`0x1cad1c`] (L01) | fast | lib:formats | keep |

### rc-engine: unit tests (68)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `disc_source::version_json_is_the_contract_line` | port logic: version json is the contract line | fast | lib:engine | keep |
| `disc_source::arguments` | port logic: arguments | fast | lib:engine | keep |
| `disc_source::resolution_order` | port logic: resolution order (disc data) | fast | lib:engine | keep |
| `disc_source::folder_checks` | port logic: folder checks | fast | lib:engine | keep |
| `disc_source::reads_are_relative_to_the_root` | port logic: reads are relative to the root | fast | lib:engine | keep |
| `fx_draw::glow_quad_faces_the_camera` | game-checked: `0x2781d0` [`0x2781d0`] | fast | lib:engine | keep |
| `fx_draw::vendor_glow_points_sit_on_the_antennas` | port logic: The vendor's glow points | fast | lib:engine | keep |
| `fx_draw::strips_and_quads_make_the_game_triangles` | game-checked: strips and quads make the game triangles | fast | lib:engine | keep |
| `fx_draw::wrap_is_the_vu_remainder` | port logic: wrap is the vu remainder | fast | lib:engine | keep |
| `game_camera::level_background_reads_settings_rgb` | port logic: `set_background_color` packing of level settings +0x00/04/08 (Novalis values), section at pointer 0. (L01) | fast | lib:engine | keep |
| `game_camera::bevy_projection_matches_vu_pipeline` | port logic: The Bevy clip matrix, the EE/VU pipeline (M, qw656, qw661) and `gs_xyz` agree. | fast | lib:engine | keep |
| `gs_state::test_1_words` | port logic: test 1 words | fast | lib:engine | keep |
| `gs_state::draw_split` | port logic: draw split | fast | lib:engine | keep |
| `hud_render::sprite_variants_map_uv_like_the_gs_packets` | port logic: sprite variants map uv like the gs packets | fast | lib:engine | keep |
| `hud_render::repeat_and_nearest_reach_the_vertex_flags` | port logic: repeat and nearest reach the vertex flags | fast | lib:engine | keep |
| `hud_render::atlas_packs_without_overlap` | port logic: atlas packs without overlap | fast | lib:engine | keep |
| `input_map::script_ranges_and_actions` | port logic: script ranges and actions | fast | lib:engine | keep |
| `input_map::device_encoding_decodes_like_a_pad` | game-checked: device encoding decodes like a pad | fast | lib:engine | keep |
| `moby_attach::one_hand_item_per_equipped_item` | game-checked: The 21 gadget classes of every level (docs/formats/moby_rac1.md §0.4) [`0x1413ff`] (all levels) | fast | lib:engine | keep |
| `moby_attach::absent_items_stay_hidden_after_a_show_again` | port logic: A swap (Bomb Glove 192 → Hologuise / Drone 483 → Pyrocitor 176 → wrench 71), then the vendor's or a scene's… | fast | lib:engine | keep |
| `moby_attach::the_hologuise_squash_scales_the_attached_items` | game-checked: `0x229158`'s item loop scales and pulls Clank once and the slot's first moby twice, the attach matrices taken before the squash; the hand item, the hook and the antenna glow are left alone [`0x229158`] | fast | lib:engine | keep |
| `moby_lod::glow_word_of_mode_and_colour` | port logic: The glow word | fast | lib:engine | keep |
| `moby_lod::test_1_fade_word` | game-checked: test 1 fade word | fast | lib:engine | keep |
| `moby_lod::draw_distance_fade_and_lod` | game-checked: draw distance fade and lod | fast | lib:engine | keep |
| `moby_lod::shine_alpha` | port logic: shine alpha | fast | lib:engine | keep |
| `moby_lod::shine_basis_straight_ahead_is_the_camera_rotation` | port logic: shine basis straight ahead is the camera rotation | fast | lib:engine | keep |
| `moby_lod::low_lod_uses_the_first_low_joint_count_slots` | game-checked: The low-LOD joint mapping (all levels) | fast | lib:engine | keep |
| `moby_lod::novalis_metal_classes` | survey (no assert): Novalis metal classes, their packets' textures (−2 chrome / −3 glass) and the TEX1/CLAMP words of their ad-gifs, for… (L01) | fast | lib:engine | ignored (survey, 2026-09-29); run with `--ignored` |
| `moby_render::moby_blend_pick_and_draws` | port logic: MobyProc's per-moby choice | fast | lib:engine | keep |
| `moby_render::point_light_merge_weights` | port logic: The point-light merge | fast | lib:engine | keep |
| `moby_render::glow_parts_blend_their_soft_edge_on_display_bytes` | port logic: A glow part's soft edge (the TEST_1 fail half) blends on display bytes | fast | lib:engine | keep |
| `moby_render::record_placement_round_trips` | port logic: The record's placement comes back from its model, at the item's own scale (not its class's). | fast | lib:engine | keep |
| `moby_render::glow_packets_are_flagged` | game-checked: The glow packets (class byte 0xa on) of the vendor 11 and the floor switch 830 are their own parts with every vertex… (disc data) | fast | lib:engine | keep |
| `moby_render::moby_cross_batch_coplanar_overlaps` | game-checked: Measurement for the within-class draw order question (gs_state.rs "Packet order") (disc data) | fast | lib:engine | keep |
| `moby_spawn::segment_hits_the_front_face_only` | port logic: segment hits the front face only | fast | lib:engine | keep |
| `movie_render::fade_alpha_matches_display_bytes` | port logic: fade alpha matches display bytes | fast | lib:engine | keep |
| `movie_render::requests_map_to_the_ntsc_files` | port logic: requests map to the ntsc files | fast | lib:engine | keep |
| `play_camera::transform_round_trips_through_game_rows` | port logic: transform round trips through game rows | fast | lib:engine | keep |
| `render_settings::samples` | port logic: samples | fast | lib:engine | keep |
| `render_settings::clamp_to_supported` | port logic: clamp to supported | fast | lib:engine | keep |
| `render_settings::legacy_settings_are_copied_once` | port logic: legacy settings are copied once | fast | lib:engine | keep |
| `render_settings::settings_text` | port logic: settings text | fast | lib:engine | keep |
| `scene_render::letterbox_bars` | port logic: `DrawScreenFade` | fast | lib:engine | keep |
| `scene_render::subtitle_box_geometry` | game-checked: `fun_001f4be0` | fast | lib:engine | keep |
| `screen_canvas::target_view_maps_texels_to_the_rectangle` | port logic: target view maps texels to the rectangle | fast | lib:engine | keep |
| `screen_canvas::viewport_letterboxes` | port logic: viewport letterboxes | fast | lib:engine | keep |
| `sea_render::tween_is_per_byte_and_truncates` | port logic: tween is per byte and truncates | fast | lib:engine | keep |
| `sea_render::grid_image_blends_two_frames_by_the_tick` | port logic: grid image blends two frames by the tick | fast | lib:engine | keep |
| `shadow_render::factor_matches_the_gs_bytes` | port logic: The resolve's linear multiply against the GS byte `Cd − ⌈Cd/4⌉` (ALPHA_1 0x2000000064 | fast | lib:engine | keep |
| `shadow_render::shader_uses_the_same_factor` | port logic: shader uses the same factor | fast | lib:engine | keep |
| `sky_render::theta_matches_dispatch` | game-checked: θ = (c & mask)·2π/(mask + 1) − π, as level01 0x29ee10 computes it for shell 4 (mask 0xffff). [`0x29ee10`] | fast | lib:engine | keep |
| `sky_render::rotation_is_ccw_about_game_up` | port logic: Rz(θ) row-vector form | fast | lib:engine | keep |
| `sky_stars::overlay_sections` | port logic: Two sections | fast | lib:engine | keep |
| `text_render::font_print_rules` | port logic: font print rules | fast | lib:engine | keep |
| `text_render::ui_frame_rects` | port logic: ui frame rects | fast | lib:engine | keep |
| `tfrag_lod::vu_weights_match_world_depth_rule` | port logic: The VU weight t = clamp(w·qw666.xy + qw668.xy) is (u/2, 1 − u) with u = (depth − D1) / (D0 − D1), and the collapse… | fast | lib:engine | keep |
| `tfrag_lod::tfrag_proc_culls_and_selects` | port logic: tfrag proc culls and selects | fast | lib:engine | keep |
| `thruster_render::tables_on_every_level` | data/format: The callback's tables are found through the relocation on all 19 levels and are the same data everywhere (all levels) | fast | lib:engine | keep |
| `thruster_render::scroll_wraps` | port logic: The ST scroll | fast | lib:engine | keep |
| `thruster_render::flame_draws_in_its_frame` | port logic: The draws of a burning flame along −x | fast | lib:engine | keep |
| `tie_lod::lod_boundaries` | game-checked: lod boundaries | fast | lib:engine | keep |
| `tie_lod::culling` | port logic: culling | fast | lib:engine | keep |
| `tie_lod::fog_matches_the_tfrag_line` | port logic: fog matches the tfrag line | fast | lib:engine | keep |
| `tie_lod::integer_vu_ops_match_the_float_model` | port logic: integer vu ops match the float model | fast | lib:engine | keep |
| `tie_lod::fat_colour_blend` | port logic: fat colour blend | fast | lib:engine | keep |
| `travel_render::only_a_gold_demo_actor_takes_the_heros_light` | port logic: `update_item_tint_from_hero` in the gold upgrade's demo (`FUN_002aea70`): the gold actor takes light set 12 from the hero moby; every other actor keeps its own light word | fast | lib:engine | keep |
| `vendor_render::the_static_tiles_its_textures` | game-checked: the static tiles its textures | fast | lib:engine | keep |
| `visibomb_view::fog_swap_and_restore_in_order` | game-checked: fog swap and restore in order | fast | lib:engine | keep |
| `visibomb_view::band_records_by_level` | port logic: band records by level | fast | lib:engine | keep |

### rc-data: integration tests (6)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `lifecycle::lazy_build_then_once_per_process_then_read_from_disk` | game-checked: lazy build then once per process then read from disk (disc data) | fast | data | keep |
| `lifecycle::stale_stamp_triggers_a_rebuild` | game-checked: stale stamp triggers a rebuild (disc data) | fast | data | keep |
| `lifecycle::corrupt_lump_is_detected_and_rebuilt` | data/format: corrupt lump is detected and rebuilt (disc data) | fast | data | keep |
| `lifecycle::unwritable_cache_falls_back_to_memory` | data/format: unwritable cache falls back to memory (disc data) | fast | data | keep |
| `lifecycle::ensure_keeps_good_lumps_and_rebuilds_the_rest` | data/format: ensure keeps good lumps and rebuilds the rest (disc data) | fast | data | keep |
| `roundtrip::every_cached_lump_equals_fresh_decompression_for_all_19_levels` | data/format: every cached lump equals fresh decompression for all 19 levels (all levels) | medium | data | keep |

### rc-data: unit tests (5)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `cache::stamp_round_trips_and_compares` | data/format: stamp round trips and compares | fast | lib:data | keep |
| `cache::trailer_round_trips` | data/format: trailer round trips | fast | lib:data | keep |
| `cache::paths` | port logic: paths | fast | lib:data | keep |
| `xxh64::reference_vectors` | port logic: reference vectors | fast | lib:data | keep |
| `xxh64::every_byte_matters` | port logic: every byte matters | fast | lib:data | keep |

### rc-extract: integration tests (3)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `synthetic::error_codes_for_bad_images` | game-checked: error codes for bad images (disc data) | fast | extract | keep |
| `synthetic::identify_extract_verify_on_a_synthetic_rc1_disc` | game-checked: identify extract verify on a synthetic rc1 disc (disc data) | fast | extract | keep |
| `synthetic::binary_json_lines_and_exit_codes` | port logic: The binary (disc data) | fast | extract | keep |

### rc-extract: unit tests (24)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `archive::pal_copies` | port logic: pal copies | fast | lib:extract | keep |
| `archive::table_join` | data/format: table join | fast | lib:extract | keep |
| `build_db::serials_regions_and_games` | port logic: serials regions and games | fast | lib:extract | keep |
| `build_db::builtin_table_parses_and_matches_its_build` | data/format: builtin table parses and matches its build | fast | lib:extract | keep |
| `build_db::table_round_trip_and_errors` | data/format: table round trip and errors | fast | lib:extract | keep |
| `export/data::relative_uris_climb_to_the_export_root` | port logic: relative uris climb to the export root | fast | lib:extract | keep |
| `export/geometry::euler_order_is_x_then_y_then_z` | port logic: euler order is x then y then z | fast | lib:extract | keep |
| `export/geometry::orient_turns_triangles_against_their_normals` | port logic: orient turns triangles against their normals | fast | lib:extract | keep |
| `export/geometry::ps2_colours_map_0x80_to_one` | port logic: ps2 colours map 0x80 to one | fast | lib:extract | keep |
| `export/gltf::a_written_document_validates_and_bounds_are_checked` | port logic: a written document validates and bounds are checked | fast | lib:extract | keep |
| `export/gltf::skins_and_animations_are_checked` | port logic: skins and animations are checked | fast | lib:extract | keep |
| `export/jsonv::writes_and_reads_back_nested_values` | game-checked: writes and reads back nested values | fast | lib:extract | keep |
| `export/level_test::export_level_01_fully` | survey (ignored): export level 01 fully (disc data) | ignored | lib:extract | keep |
| `export/mod::kinds_parse_lists_and_all` | data/format: kinds parse lists and all | fast | lib:extract | keep |
| `export/models::decompose_rebuilds_rotation_translation_and_scale` | game-checked: decompose rebuilds rotation translation and scale | fast | lib:extract | keep |
| `export/png::checksums_match_reference_values` | port logic: checksums match reference values | fast | lib:extract | keep |
| `export/png::deflate_round_trips_through_the_reader` | data/format: deflate round trips through the reader | fast | lib:extract | keep |
| `export/png::indexed_and_rgba_pngs_decode_to_their_pixels` | data/format: indexed and rgba pngs decode to their pixels | fast | lib:extract | keep |
| `export/wav::header_fields_samples_and_loop_round_trip` | data/format: header fields samples and loop round trip | fast | lib:extract | keep |
| `identify::cnf_values` | port logic: cnf values | fast | lib:extract | keep |
| `json::objects_round_trip_with_escapes` | data/format: objects round trip with escapes | fast | lib:extract | keep |
| `main::parses_the_contract_command_lines` | data/format: parses the contract command lines | fast | lib:extract | keep |
| `main::rejects_bad_command_lines` | port logic: rejects bad command lines | fast | lib:extract | keep |
| `space::reports_something_plausible_for_the_temp_dir` | port logic: reports something plausible for the temp dir | fast | lib:extract | keep |

### rc-video: integration tests (3)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `movies::holofilm_start` | game-checked: holofilm start [`0x285900`] (disc data) | fast | video | keep |
| `movies::every_movie_decodes` | survey (ignored): every movie decodes (disc data) | ignored | video | keep |
| `movies::dump_yuv` | survey (ignored): dump yuv (disc data) | ignored | video | keep |

### rc-video: unit tests (8)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `bits::reads_msb_first_and_pads_with_zeros` | port logic: reads msb first and pads with zeros | fast | lib:video | keep |
| `bits::finds_start_codes` | port logic: finds start codes | fast | lib:video | keep |
| `idct::ieee_1180_accuracy` | port logic: ieee 1180 accuracy | fast | lib:video | keep |
| `movie::bt601_limited_range` | port logic: bt601 limited range | fast | lib:video | keep |
| `movie::resample_keeps_48k_and_scales_length` | port logic: resample keeps 48k and scales length | fast | lib:video | keep |
| `mpeg2::synthetic_display_order_and_samples` | port logic: A synthetic 32×16 stream (2 macroblocks, one slice per picture) | fast | lib:video | keep |
| `vlc::tables_are_consistent` | data/format: The tables are prefix codes (the build panics otherwise) with the expected sizes and code spaces | fast | lib:video | keep |
| `vlc::decodes_codes` | data/format: decodes codes | fast | lib:video | keep |

### trace: integration tests (5)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `hero_replay_selfcheck::replay_of_the_ports_own_trace_has_no_divergence` | port logic: replay of the ports own trace has no divergence (disc data) | fast | trace | keep |
| `novalis_spawn::novalis_spawn_state_checks` | game-checked: novalis spawn state checks (L01) | medium | trace | keep |
| `novalis_spawn::fixture_round_trips` | data/format: The fixture parses and writes back to the same facts (no personal data needed). (disc data) | fast | trace | keep |
| `synthetic::locator_and_compare_on_synthetic_novalis` | game-checked: locator and compare on synthetic novalis (L01) | fast | trace | keep |
| `synthetic::compare_through_a_savestate_file` | game-checked: The same lit image packed as a PCSX2 v2.8.2-style savestate (stored version id, zstd RAM). (disc data) | fast | trace | keep |

### trace: unit tests (21)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `class_census::data_refs_sees_lo_loads_base_offsets_and_compares` | game-checked: data refs sees lo loads base offsets and compares [`0x13f350`, `0x140000`] | fast | lib:trace | keep |
| `class_census::data_refs_drops_text_addresses_small_constants_and_clobbered_bases` | port logic: data refs drops text addresses small constants and clobbered bases | fast | lib:trace | keep |
| `ee::address_mirrors` | port logic: address mirrors | fast | lib:trace | keep |
| `ee::find_respects_alignment` | port logic: find respects alignment | fast | lib:trace | keep |
| `ee::savestate_container` | game-checked: A synthetic savestate laid out like PCSX2 v2.8.2 writes it | fast | lib:trace | keep |
| `hero_analysis::segments_chained_jumps` | port logic: segments chained jumps | fast | lib:trace | keep |
| `hero_analysis::ignores_short_drops_and_handles_trace_ending_in_air` | port logic: ignores short drops and handles trace ending in air | fast | lib:trace | keep |
| `hero_analysis::diff_reports_first_divergence_and_errors` | game-checked: diff reports first divergence and errors | fast | lib:trace | keep |
| `hero_analysis::angle_errors_wrap` | port logic: angle errors wrap | fast | lib:trace | keep |
| `hero_record::plan_brackets_the_counter_and_splits` | game-checked: plan brackets the counter and splits [`0x13f3d0`, `0x13f3d4`] | fast | lib:trace | keep |
| `hero_record::utc_stamp_shape` | port logic: utc stamp shape | fast | lib:trace | keep |
| `hero_trace::format_round_trip_is_bit_exact` | data/format: format round trip is bit exact | fast | lib:trace | keep |
| `hero_trace::format_tolerates_reordered_unknown_and_missing_columns` | port logic: format tolerates reordered unknown and missing columns | fast | lib:trace | keep |
| `hero_trace::pad_bytes_rebuild_the_decode` | data/format: The PAD record written by the port's `ProcessPadInput` for a pad read → the rebuilt bytes decode the same. | fast | lib:trace | keep |
| `hero_trace::camera_record_check` | game-checked: camera record check [`0x160000`, `0x160400`] | fast | lib:trace | keep |
| `pine::batched_read_framing` | game-checked: A fake PINE server answering MsgRead64 with the address itself, to check framing and batching. | fast | lib:trace | keep |
| `pine::read_many_framing` | game-checked: `read_many` [`0x13f3d0`] | fast | lib:trace | keep |
| `port_sim::lcg_distance_counts_steps` | port logic: lcg distance counts steps | fast | lib:trace | keep |
| `zip::crc32_known_value` | data/format: crc32 known value | fast | lib:trace | keep |
| `zip::roundtrip_all_methods` | data/format: roundtrip all methods | fast | lib:trace | keep |
| `zip::corrupt_data_is_detected` | port logic: corrupt data is detected | fast | lib:trace | keep |

### repo-checks: integration tests (6)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `guards::product_crates_do_not_depend_on_tools` | repo guard: Rule (disc data) | fast | repo | keep |
| `guards::manifest_scan_catches_a_tool_path` | repo guard: The manifest scan itself (disc data) | fast | repo | keep |
| `guards::no_test_or_product_file_references_personal_paths` | repo guard: no test relies on personal files; no allow-list (§10; renamed 2026-09-29) | fast | repo | keep |
| `guards::guard_scans_crates_and_tool_tests` | repo guard: the scanned folders (added 2026-09-29) | fast | repo | keep |
| `guards::personal_patterns_match_segments_only` | repo guard: personal patterns match segments only (disc data) | fast | repo | keep |
| `guards::top_level_holds_only_the_agreed_folders` | repo guard: Rule (disc data) | fast | repo | keep; fails while a user `.mov` sits in the repo root (expected) |

### xtask: unit tests (8)

| test | pins | runtime | group | verdict |
|---|---|---|---|---|
| `classify::extractor_output_is_recognised` | port logic: extractor output is recognised | fast | lib:xtask | keep |
| `classify::cpp_leftovers_are_known` | port logic: cpp leftovers are known | fast | lib:xtask | keep |
| `classify::anything_else_is_unknown` | game-checked: anything else is unknown | fast | lib:xtask | keep |
| `classify::the_committed_table_loads` | port logic: the committed table loads | fast | lib:xtask | keep |
| `regen::parses_flags` | port logic: parses flags | fast | lib:xtask | keep |
| `regen::never_replaces_the_repo_home_or_root` | port logic: never replaces the repo home or root (disc data) | fast | lib:xtask | keep |
| `regen::staging_sits_on_the_same_filesystem` | port logic: staging sits on the same filesystem (disc data) | fast | lib:xtask | keep |
| `regen::formats_numbers` | port logic: formats numbers | fast | lib:xtask | keep |

## 8. cargo-nextest and the xtask tier commands

**Install** (once): `cargo install cargo-nextest --locked` (0.9.146 on 2026-09-29). It lands in `~/.cargo/bin`
(`CARGO_HOME`), which is not on this machine's `PATH`; `cargo nextest …` still works because cargo looks in
`$CARGO_HOME/bin` for subcommands. Only calling `cargo-nextest` directly needs the folder on `PATH` (fish:
`fish_add_path ~/.cargo/bin`; zsh: `export PATH="$HOME/.cargo/bin:$PATH"` in `~/.zprofile`).

**Never by hand:** the xtask commands call `cargo nextest run --workspace --features rc-engine/dev …` (the flags
that reuse the dev (dynamic) Bevy build) with the right target flags and filterset; every targeted need has an option
(§1), so neither `cargo nextest` nor `cargo test` is run directly.

**Settings:** `.config/nextest.toml`, profile `default`:
- `fail-fast = false`;
- a test is flagged SLOW after 30 s (and every 30 s after); nothing is killed;
- `test-threads = 8`, the same cap as the build's `jobs = 8`;
- the final summary lists the slow and failed tests again;
- test group `loader-snapshots` (one at a time): the rc-formats `golden::` tests, which rewrite one shared file
  (`crates/rc-formats/data/loader_snapshots.tsv`) under `RC_SNAPSHOT_WRITE=1`. Under `cargo test` a mutex serialises
  them; nextest's one process per test defeats the mutex. No other test shares a file or an environment variable:
  the temp folders carry the process id and the test name, no test calls `set_var`, and the `RC_*` variables are
  only read.

**Doctests:** none (every lib's `Doc-tests` runs 0), so the nextest runs miss nothing. When one appears, add
a `cargo test --workspace --features rc-engine/dev --doc` step to `test-full` (`full_steps` in `tools/xtask/src/test.rs`).

**Determinism:** the shared guard set (`cargo xtask test-job shared`, 851 tests plus the digest) was run twice under
nextest on 2026-09-29: all passed both times, and the two NO_IDLE digests are byte-identical to each other and to
the baseline taken before the merge.

**The commands** (`cargo xtask help`; `tools/xtask/README.md`):

| Command | Runs |
|---|---|
| `cargo xtask test-quick [crate]` | the unit tests (`--lib --bins`); with nextest a crate name narrows it (`-E package(<crate>)`) |
| `cargo xtask test-job <area…>` | the unit tests and the areas' binaries of §3 in one nextest run (a filterset for the partial areas `audio` and `shared`); with `shared`, then the digest compare |
| `cargo xtask test-job --test <binary>…` | those integration binaries alone (`--test <binary>`; no unit tests, no digest); next to areas, added to them |
| `cargo xtask test-full` | the whole suite with `--no-fail-fast`, then the digest compared with the baseline (a mismatch fails), then `cargo xtask sweep` (§9; only warns) |
| `cargo xtask digest-baseline` | the only writer of the baseline; prints what changed. Run it only when a human has decided the digest change is intended |

Targeting options of `test-quick` and `test-job`: `--filter <name>` (a test-path substring, repeatable; nextest
`-E test(<name>)` ANDed with the selection, libtest's positional filter), `--exact` (full paths: `test(=…)`,
libtest `--exact`), `--ignored` (`--run-ignored only`, libtest `--ignored`), `--nocapture` (`--no-capture`). A
filtered or `--ignored` `test-job shared` skips the digest. Under `--cargo-test` a filter replaces a partial area's own
module filters (libtest ORs filters), so it applies to that whole binary.

Each sets `RC_AUDIO=0`; `--cargo-test` uses `cargo test --workspace --features rc-engine/dev` instead of nextest;
arguments after `--` go to nextest as is (or to the test binaries under `cargo test`). The runner choice matters for the level scans: see the end of §5.

## 9. Keeping target/ small

`target/` grows with every feature set, profile and stale test binary (13–16 GB on 2026-09-29; the static-Bevy
mistakes of §1 roughly double it). `cargo xtask sweep` keeps it under a limit, **30 GB by default**
(`--limit 20G`, `--limit 500MB`, `--limit 2GiB`; G/GB are decimal), with cargo-sweep
(`cargo install cargo-sweep --locked`, 0.8.0 on 2026-09-29; like nextest it lands in `~/.cargo/bin` and `cargo sweep`
finds it there).

* **What goes:** while `target/` holds more file bytes than the limit, the least recently used build units, oldest
  first: a unit is one `.fingerprint/<name>-<hash>` with its `deps/` and `build/` files, and "used" is the newest access
  time of its fingerprint files (cargo reads them on every build that includes the unit). What the dev loop and the
  tests used last stays, so `cargo dev` does not rebuild Bevy. It never touches `incremental/` (not tracked by
  fingerprints; about half of `target/`) or unhashed outputs such as `target/debug/rerac`.
* **Safety:** it refuses (exit 1) while another `cargo`, `rustc` or `cargo-nextest` runs, so it never deletes files
  under a running build; the `cargo` running the xtask itself does not count. Without cargo-sweep it prints the install
  line and exits 0.
* **Output:** `target/` before and after, and the units removed. `--dry-run` shows what it would remove.
* **When it runs:** at the end of `cargo xtask test-full` (a refusal or failure there only warns; the test result
  stands), after the coordinator's commit, and by hand whenever.
* **Full reset:** `cargo clean` (then the next `cargo dev` rebuilds everything, Bevy included: minutes, not seconds).
  It is also the only way to drop a big stale `incremental/`.

## 10. Personal files: none in tests (2026-09-29)

**The rule.** No test relies on the user's personal files, not even optionally or "skip if missing". Personal means:
anything in the personal folder (the disc image, savestates, PCSX2 traces), screen recordings, the user's settings
file, anything in `work/` that came from the user's own sessions, and any other file that exists only on the user's
machine. Tests may read `extracted/` through the data root (`rc_formats::test_data::root`, `RC_EXTRACTED`) like the rest
of the suite, and committed fixtures. Findings from personal material reach tests only as small distilled values: a
handful of numbers inline, or a tiny committed fixture (e.g. `crates/rc-game/tests/fixtures/shadow_ratchet_novalis_idle.tsv`).
No gameplay logs, trace dumps or savestate excerpts are committed. Provenance comments name the source in words
("distilled from the user's PCSX2 savestate of 2026-09-26"), never with a path.

**The guard.** `tools/repo-checks` (`guards::no_test_or_product_file_references_personal_paths`) scans every file under
`crates/` and every tool's `tests/` folder for the personal patterns (`.p2s`, `~/PS2`, `PS2/ratchet1`, `work/`,
`/traces/`, `RC_PERSONAL`), and test files additionally for the disc-image and home-folder hooks (`RC_ISO`, `HOME`,
`home_dir`). It has no allow-list. Tool sources (`tools/*/src`) are not scanned: tools read personal material on
purpose (`rc-trace` savestates and recordings, `xtask regen-data` the disc image).

**Disc checks (by hand).** What needs the disc image is a dev command, not a test:

```
cargo run --release -p rc-trace -- disc-check [--iso IMAGE] [--extracted DIR] [--extract-into SCRATCH]
```

1. the disc reader (`rc_formats::disc`) against `extracted/` on all 19 levels (every lump, the audio/scene lumps, the
   TOC, the boot ELF), with per-level read times;
2. the disc's `save_game` lump against `extracted/global/save_game.bin`, and the card folder `SYSTEM.CNF` names;
3. the extractor's size/SHA-1 table hashed from the image against `crates/rc-extract/data/scus_971_99.tsv` (run it after
   regenerating the table with `rerac-extract table`);
4. with `--extract-into`: a full and an `--ntsc-only` extraction (~7 GiB; delete afterwards) hold exactly the table's
   files and verify.

The image defaults to `RC_ISO`, else the ISO in the personal folder. Run it after a change to `rc_formats::disc`,
`iso9660` or the extractor.

## 11. The loader golden tests and the build flags

**Golden tests.** `crates/rc-formats/tests/formats/golden.rs` compares what the Rust loaders produce for all 19
levels with `crates/rc-formats/data/loader_snapshots.tsv` (per test, level and section: item count, byte count,
SHA-1). The table was generated while the output was byte-identical to the C++ reference extractor, retired on
2026-09-27 (`docs/plan/decisions.md`). After an intended loader change, `RC_SNAPSHOT_WRITE=1 cargo xtask test-job
--test formats --filter golden::` rewrites the rows of the tests that ran.

**One Bevy build.** The `cargo xtask test-*` commands pass `--workspace --features rc-engine/dev`, which enables
Bevy's `dynamic_linking` like `cargo dev`, so the tests reuse the one Bevy build `cargo dev` made
(plain `cargo test --workspace` would compile a second, static Bevy). `cargo check-all` and `cargo clippy-all`
(aliases in `.cargo/config.toml`) add `--all-targets` with the same features; they share one Bevy check build
(metadata only, which a test build cannot reuse). Release builds and `tools/package` stay static (never
`--features dev`).
