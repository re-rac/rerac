# tools/xtask: one short command per dev chore

Dev only, never ships. Package `xtask` (workspace member at `tools/xtask`), run through the Cargo alias in
`.cargo/config.toml` (`xtask = "run -q -p xtask --"`), so every command is `cargo xtask <command>`. Std only, no
dependencies: it shells out to `cargo` and to `tools/package/package.sh`, so building it compiles only this crate.

| Command | What it does |
|---|---|
| `cargo xtask help` | Lists the commands (also `cargo xtask` alone). |
| `cargo xtask regen-data` | Rebuilds the dev game data folder `extracted/` from your disc image (below). |
| `cargo xtask package [--no-build]` | Runs `tools/package/package.sh` with the same arguments (`docs/workflows/release.md`). |
| `cargo xtask test-quick [crate]` | Unit tests of every crate (`--lib --bins`); with nextest, a crate name runs only its. |
| `cargo xtask test-job <area…>` | A job's test tier: the unit tests plus each area's integration binaries (below). |
| `cargo xtask test-job --test <binary>…` | One or more integration binaries alone (no unit tests, no digest). |
| `cargo xtask test-full` | The full suite, then the NO_IDLE hero digest compared with the baseline (a mismatch fails; never rewritten), then `sweep` (a failure only warns). |
| `cargo xtask digest-baseline` | The only writer of the digest baseline `work/test-results/hero_digest_no_idle.txt`; prints what changed. Only when a digest change is intended. |
| `cargo xtask sweep [--limit <size>] [--dry-run]` | Keeps `target/` under a limit (default 30 GB) with cargo-sweep, least recently used build units first; refuses while a build runs; `test-full` runs it at the end (below). |

## The test commands

The tiers of `docs/workflows/testing.md` §2 (the doc has the policy; this is the mechanics). They are the only way to
run tests in this repo: there is no Cargo test alias, and `cargo test` / `cargo nextest` are never run by hand.
Every test command:
* sets `RC_AUDIO=0` and runs from the repo root (other variables, e.g. `RC_SNAPSHOT_WRITE=1`, pass through);
* uses cargo-nextest when `cargo nextest` answers (settings: `.config/nextest.toml`), with
  `--workspace --features rc-engine/dev` so the dev Bevy build is shared; else `cargo test` with the same flags
  (`NEXTEST` and `CARGO_TEST` in `src/test.rs`). `--cargo-test` forces `cargo test`;
* passes the arguments after `--` on: to nextest (e.g. `--no-capture`), or to the test binaries under `cargo test`;
* runs every step even when one fails, and exits 1 if any failed.

Targeting (`test-quick` and `test-job`; the same meaning under both runners):

| Option | Nextest | `cargo test` |
|---|---|---|
| `test-quick <crate>` | `-E package(<crate>)` | every crate (a single one would rebuild) |
| `test-job --test <binary>` (repeatable) | `--test <binary>` (next to areas also `binary(=<binary>)` in the filterset) | `--test <binary>` |
| `--filter <name>` (repeatable) | `-E test(<name>)`, ANDed with the selection | libtest filter (replaces a partial area's own filters) |
| `--exact` | `test(=<name>)` | `--exact` |
| `--ignored` | `--run-ignored only` | `--ignored` |
| `--nocapture` | `--no-capture` | `--nocapture` |

Examples: `cargo xtask test-quick rc-formats`, `cargo xtask test-job --test weapons`, `cargo xtask test-job --test
hero --filter hero_novalis::novalis_hero_digest --exact`, `cargo xtask test-job --test classes --filter
creature_classes:: --ignored --nocapture`. A filtered or `--ignored` `test-job shared` skips the digest; `test-full`
takes no targeting options.

The areas of `test-job` are the table `AREAS` in `src/test.rs` (testing.md §3 documents it): `hero`, `weapons`,
`classes` (`creatures`, `mobys`), `world` (`levels`, `collision`, `water`), `ui` (`menus`, `hud`, `map`, `save`,
`vendor`), `audio` (the two conformance modules of `ui` and one Ryno test of `weapons`), `formats`, `data`, `extract`,
`video` (`movies`), `engine` (`render`, `input`; unit tests only), `trace`, `repo` (`guards`, `layout`), and `shared`:
the all-levels smoke plus the NO_IDLE hero digest, written to `work/test-results/digest_job.txt` and compared byte
for byte with the baseline (missing baseline: the step fails and says to run `digest-baseline`). With nextest a job is
one run with a filterset; with `cargo test` it is one run for the whole binaries plus one per filtered part.

## `sweep`

```
cargo xtask sweep [--limit <size>] [--dry-run]
```

Runs `cargo sweep --maxsize <limit>B` (cargo-sweep reads a bare number as MiB, so the exact byte count is passed)
on the workspace's `target/`. The limit defaults to 30 GB; sizes take a unit: `30GB`/`30G`, `500MB`/`500M`, `1.5TB`
(decimal, as cargo-sweep's), `2GiB` (binary), `1000B`; a bare number is refused. It prints `target/`'s file bytes
before and after and the build units removed (`--dry-run`: would remove). It refuses (exit 1) while any `cargo`,
`rustc` or `cargo-nextest` other than its own ancestors runs (`ps -axo pid,ppid,comm`), and exits 0 with the install
line (`cargo install cargo-sweep --locked`) when cargo-sweep is missing. `test-full` calls it last and only warns on a
refusal or failure. What it removes and what it never touches: `docs/workflows/testing.md` §9.



```
cargo xtask regen-data [--iso <image>] [--data-dir <dir>] [--force] [--check]
```

* **Inputs:** the disc image (`--iso`, else `RC_ISO`); the Tier 0 table `crates/rc-extract/data/*.tsv`.
* **Output:** the data folder: `--data-dir`, else `RC_EXTRACTED`, else `<repo>/extracted`.

What it does, in order:

1. **The disc.** Without `--iso` or `RC_ISO`, it prints how to set `RC_ISO` once (fish: `set -Ux RC_ISO "…"`; zsh /
   bash: an `export` line in `~/.zshrc` / `~/.bashrc`; PowerShell: a user environment variable) and exits with code 2.
2. **What the folder holds.** Every file is sorted into:
   * **extractor output:** the Tier 0 table's files, `extract-info.json`, `cache/`, `exports/`;
   * **known C++ extractor leftovers,** listed with counts and sizes, then deleted with the old folder: `*.partial`,
     `.dec` copies, `*_dump.bin`, `levels/NN/{core,gameplay,textures}/` splits, `levels/NN/overlay.{elf,txt}`, `vu/`,
     `levels/NN/*.{png,obj}` previews, `.DS_Store` (the overlay ELFs and VU listings were copied to `work/` on
     2026-09-27);
   * **unknown:** anything else. The first 20 are listed and the run stops (exit 1) unless `--force`, which deletes
     them too.
   `--check` stops here and changes nothing (no disc needed). It never replaces `/`, your home folder, the repository
   or a folder holding `.git` / `Cargo.toml`; inside the repo only `extracted/` or a folder under `work/`.
3. **Extract into staging.** It builds `rerac-extract` with the normal dev profile (`cargo build -p rc-extract`,
   never `--release`) and runs `extract` (which ends with `prepare`, the engine cache) into a staging folder on the
   same filesystem: `work/data-staging` for `<repo>/extracted`, else `.<name>.xtask-staging` next to the folder.
4. **Verify** the staging folder (`rerac-extract verify`: every Tier 0 file's size and SHA-1).
5. **Swap.** The old folder is renamed aside (`work/data-old`, or `.<name>.xtask-old`), the new one renamed in, the
   old one deleted. Any failure before the swap deletes the staging folder and leaves the old folder untouched; a run
   killed mid-swap is repaired by the next run (the aside folder is restored or deleted).
6. **Summary:** files, bytes, engine cache lumps and size, and the time of each step.

Timing on the author's Mac (2026-09-27, ISO in the page cache): extract + prepare 4.3 s, verify 3 s, a no-op build
0.1 s, swap 0.2 s; about 8 s in all, plus a few seconds when `rerac-extract` needs rebuilding. The dev profile builds the
workspace crates at `opt-level = 1`, which is enough: the extractor is I/O-bound, as fast as the release build.
It needs free space for the new data (about 4.4 GiB) while the old folder still exists.

Exit codes: 0 done (or `--check`), 1 refused or failed (the old folder unchanged), 2 bad arguments or no disc image.

**Tests:** `cargo xtask test-quick xtask` (argument parsing, the test-tier table and its steps, the sweep's sizes, busy check and missing-tool path, the leftover classifier against the committed Tier 0 table, the
safety guard, staging paths). End to end it was run against scratch folders only, with `--data-dir`.
