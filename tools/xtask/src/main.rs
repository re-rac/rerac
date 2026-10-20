//! `cargo xtask <command>`: one short command per dev chore (alias in `.cargo/config.toml`). Dev only, never ships;
//! std only. Doc: `tools/xtask/README.md`.

mod classify;
mod regen;
mod sweep;
mod test;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const HELP: &str = "\
cargo xtask <command> [options]    dev chores, one command each (tools/xtask/README.md)

commands:
  regen-data [--iso <image>] [--data-dir <dir>] [--force] [--check]
      Rebuild the dev game data folder (extracted/, or RC_EXTRACTED) from your disc image with the dev build of
      rerac-extract: extract into a staging folder, verify it, then swap it in and delete the old folder.
      The retired C++ extractor's leftovers are listed and removed; any unknown file stops it unless --force.
        --iso <image>     the disc image (default: RC_ISO)
        --data-dir <dir>  the folder to rebuild (default: RC_EXTRACTED, else <repo>/extracted)
        --force           also delete unknown files
        --check           only list what the folder holds; change nothing (no disc needed)
  package [--no-build]
      Release packaging: runs tools/package/package.sh (dist/rerac-<version>-<os>-<arch>/ and its .zip).
  sweep [--limit <size>] [--dry-run]
      Keep target/ under a size limit (default 30GB; sizes like 30GB, 20G, 500MB, 2GiB) with cargo-sweep: the
      least recently used build units go first (never incremental/; `cargo clean` is the full reset). Refuses while
      another cargo, rustc or cargo-nextest runs. Prints target/ before and after, and what went. Without cargo-sweep
      it prints `cargo install cargo-sweep --locked` and exits 0. test-full runs it at the end (a failure only warns).
        --dry-run         only show what it would remove

test tiers (docs/workflows/testing.md §1-2): the only way to run tests; never `cargo test` / `cargo nextest` by hand.
RC_AUDIO=0; cargo-nextest when installed, else `cargo test --workspace --features rc-engine/dev`.
  test-quick [crate]
      Unit tests only: every crate's lib and bin tests (`--lib --bins`, about 5 s). With nextest a crate name runs
      just that crate's (without nextest every crate runs: a single one would rebuild).
  test-job <area>... [--test <binary>]...
      The per-job tier: the unit tests plus the integration binaries of each area. Areas (aliases):
        hero, weapons, classes (creatures, mobys), world (levels, collision, water), ui (menus, hud, map, save,
        vendor), audio, formats, data, extract, video (movies), engine (render, input: unit tests only), particles
        (fx: the ported classes' particle consumers), trace,
        repo (guards, layout), shared (for shared code: the all-levels smoke, and the NO_IDLE hero digest
        compared with work/test-results/hero_digest_no_idle.txt).
      --test <binary>     also run this integration binary whole (repeatable); with no area, only the binaries
                          (no unit tests, no digest). Binaries: hero weapons classes world ui formats data extract
                          movies trace guards.
  test-full
      The full suite (on merge to main, or when the coordinator asks), then the NO_IDLE hero digest compared
      with the baseline; a mismatch fails. It never rewrites the baseline. Then `sweep` (a failure only warns).
  digest-baseline
      The only command that writes the NO_IDLE hero digest baseline, and prints what changed: for a human who
      has decided a digest change is intended, or before a job's first edit when there is none.
    Targeting options of test-quick and test-job (the same meaning under both runners):
      --filter <name>     only tests whose path contains <name> (repeatable: any), e.g. hero_novalis:: or a fn name;
                          a filtered test-job skips the digest
      --exact             the filters are full paths, <module>::<test_fn>
      --ignored           only the ignored tests (surveys, printers); a filtered test-job skips the digest
      --nocapture         show the tests' output
    Options of the test commands:
      --cargo-test        use `cargo test --workspace --features rc-engine/dev` even when nextest is installed
      -- <args>           passed on as is: to nextest, or to the test binaries under cargo test
  help
      This list.";

/// The repository root (this crate lives at `tools/xtask`).
pub fn repo_root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    p.canonicalize().map(strip_verbatim).unwrap_or(p)
}

/// `p` without the `\\?\` prefix `canonicalize` adds on Windows (Git Bash cannot open such paths).
pub fn strip_verbatim(p: PathBuf) -> PathBuf {
    match p.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
        _ => p,
    }
}

/// The user's home folder: `HOME`, else `USERPROFILE` on Windows (where `HOME` is usually unset).
pub fn home_dir() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    var("HOME").or_else(|| if cfg!(windows) { var("USERPROFILE") } else { None })
}

/// The `cargo` that ran us (`CARGO` is set by `cargo run`), else `cargo` from `PATH`.
pub fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn main() -> ExitCode {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (cmd, rest) = match argv.split_first() {
        Some((c, r)) => (c.to_string_lossy().into_owned(), r),
        None => ("help".to_string(), &argv[..]),
    };
    match cmd.as_str() {
        "regen-data" => regen::run(rest),
        "package" => package(rest),
        "test-quick" => test::quick(rest),
        "test-job" => test::job(rest),
        "test-full" => test::full(rest),
        "digest-baseline" => test::baseline(rest),
        "sweep" => sweep::main(rest),
        "help" | "--help" | "-h" => { println!("{HELP}"); ExitCode::SUCCESS }
        other => {
            eprintln!("error: unknown command {other:?}\n\n{HELP}");
            ExitCode::from(2)
        }
    }
}

/// `cargo xtask package [args]`: `tools/package/package.sh [args]`, from the repo root.
fn package(args: &[OsString]) -> ExitCode {
    let root = repo_root();
    let bash = bash();
    let status = Command::new(&bash).arg("tools/package/package.sh").args(args).current_dir(&root).status();
    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().and_then(|c| u8::try_from(c).ok()).unwrap_or(1)),
        Err(e) => { eprintln!("error: cannot run {} tools/package/package.sh: {e}", bash.display()); ExitCode::FAILURE }
    }
}

/// The bash that runs `package.sh`. On Windows a bare `bash` can resolve to WSL's, so Git for Windows' is preferred.
fn bash() -> PathBuf {
    #[cfg(windows)]
    if let Some(b) = git_bash() { return b; }
    "bash".into()
}

/// Git for Windows' bash: `git --exec-path` is `<git>/mingw64/libexec/git-core`, bash is `<git>/bin/bash.exe`.
#[cfg(windows)]
fn git_bash() -> Option<PathBuf> {
    let out = Command::new("git").arg("--exec-path").output().ok()?;
    let exec = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    let b = exec.ancestors().nth(3)?.join("bin/bash.exe");
    b.is_file().then_some(b)
}
