# tools/trace: PCSX2 ground-truth harness (`rc-trace`)

Dev only, never ships. Package name `rc-trace` (workspace member at `tools/trace`), so every command is
`cargo run -p rc-trace -- <command>`; `--help` lists them all. Workflow: `docs/workflows/pcsx2.md`. Design, addresses
and triage: `docs/plan/trace_harness.md`; results: `docs/plan/trace_results_novalis.md`; hero feel pass:
`docs/plan/hero_feel_pass.md`.

## Purpose

Reads the PS2's EE memory (a PCSX2 savestate, a raw dump, or the live game over PINE) and compares it with what the
port computes from the same game data: tfrag / tie / shrub lighting, the Novalis spawn state (game-state chunks, hero,
camera, fog, `rand` stream, moby table), and per-tick hero recordings replayed through the port.

## Commands (the main ones)

| Task | Command |
|---|---|
| Keep the newest PCSX2 savestate as `NAME` | `cargo run -p rc-trace -- save-state NAME` |
| Compare tfrag lighting | `cargo run -p rc-trace -- compare-tfrag-light --state NAME --level 01` |
| Compare the Novalis spawn state | `cargo run -p rc-trace -- compare-novalis-spawn --state NAME` |
| Dump EE RAM | `cargo run -p rc-trace -- dump-ee --state NAME` |
| Rewrite the spawn test fixture | `cargo run -p rc-trace -- distill-spawn --state NAME` |
| Record the hero over PINE | `cargo run --release -p rc-trace -- record --seconds 90` |
| Replay a recording through the port | `cargo run --release -p rc-trace -- replay-hero --trace FILE` |
| Check the disc reader and the extractor against your disc image (the checks no test may run; docs/workflows/testing.md §10) | `cargo run --release -p rc-trace -- disc-check [--extract-into DIR]` |
| Census of the unported moby classes and the shared systems they call (no PCSX2; reads `extracted/`, tags `tools/ghidra/names/census_systems.tsv`; docs/plan/class_census.md) | `cargo run -p rc-trace -- class-census` → `work/census/` |
| Which classes record a death, and how: the path to `SetDeathBits` and the death-table writers per class and level (no PCSX2; docs/plan/game_state.md §6.1) | `cargo run -p rc-trace -- death-census` → `work/census/deaths.tsv` |
| The masked function × level diff: does a function (the hero's by default) differ between the level overlays, and how (no PCSX2; reads `extracted/`; docs/workflows/ghidra.md "Masked overlay diff") | `cargo run -p rc-trace -- overlay-diff` → `work/overlay_diff/` |

## Inputs and outputs

* **Game data:** `extracted/` (`RC_EXTRACTED` or `--extracted` overrides). Read only: the tool refuses to write there.
* **Personal material:** `~/PS2/ratchet1/` (`RC_PERSONAL` overrides): kept savestates in `savestates/` (`--state NAME`
  resolves `savestates/NAME.p2s`), hero recordings in `traces/` (`record` writes new ones there). `--state latest`
  reads PCSX2's own folder `~/Library/Application Support/PCSX2/sstates`.
* **Generated output:** `work/trace/` (`RC_WORK` overrides `work/`): EE dumps (`<name>_ee.bin`), reports, CSVs,
  replay traces.
* **Committed fixture:** `tests/fixtures/novalis_spawn.tsv`, the Novalis spawn savestate distilled to numbers
  (counters, fog, light bank, per-chunk and per-palette hashes, moby slot / class / spawn id / state). Written only
  by `distill-spawn`, which first checks that the fixture checks and the savestate checks give the same tallies.

## Tests

`cargo xtask test-job trace` (the unit tests plus the `trace` binary). They need only `extracted/` (skipped without it) and the committed fixture; no savestate,
dump, recording or disc image (no test relies on personal files; `tools/repo-checks` scans `tests/`).
