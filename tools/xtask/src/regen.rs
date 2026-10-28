//! `cargo xtask regen-data`: rebuild the dev game data folder from the user's disc image with the dev build of
//! `rerac-extract`, without ever leaving the user without data.
//!
//! 1. The disc: `--iso` or `RC_ISO`; when missing, print how to set `RC_ISO` once (fish, bash/zsh, PowerShell).
//! 2. The folder (`--data-dir`, else `RC_EXTRACTED`, else `<repo>/extracted`) may hold only what the extractor writes
//!    (the Tier 0 table's files, `extract-info.json`, `cache/`, `exports/`) and the retired C++ extractor's known
//!    leftovers ([`classify::LEFTOVER_KINDS`], listed with counts and deleted). Anything else stops the run unless
//!    `--force`.
//! 3. Extract into a staging folder (`work/data-staging` for `<repo>/extracted`, else a hidden sibling), verify it,
//!    then rename the old folder aside, rename the new one in and delete the old one. A failure before the swap
//!    leaves the old folder untouched.

use crate::classify::{self, Report, LEFTOVER_KINDS};
use crate::{cargo, home_dir, repo_root, strip_verbatim};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    iso: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    force: bool,
    check: bool,
}

fn parse(argv: &[OsString]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = argv.iter();
    while let Some(arg) = it.next() {
        let s = arg.to_str().ok_or_else(|| format!("unexpected argument {arg:?}"))?;
        let (flag, inline) = match s.split_once('=') { Some((f, v)) if f.starts_with("--") => (f, Some(OsString::from(v))), _ => (s, None) };
        let mut value = |name: &str| match inline.clone().or_else(|| it.next().cloned()) {
            Some(v) if !v.is_empty() => Ok(PathBuf::from(v)),
            _ => Err(format!("{name} needs a value")),
        };
        match flag {
            "--iso" => a.iso = Some(value("--iso")?),
            "--data-dir" => a.data_dir = Some(value("--data-dir")?),
            "--force" if inline.is_none() => a.force = true,
            "--check" if inline.is_none() => a.check = true,
            _ => return Err(format!("unknown option {s}")),
        }
    }
    Ok(a)
}

fn env_path(name: &str) -> Option<PathBuf> { std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from) }

fn absolute(p: &Path) -> PathBuf {
    let p = if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(p) };
    if let Ok(c) = p.canonicalize() { return strip_verbatim(c); }
    match (p.parent().and_then(|d| d.canonicalize().ok()).map(strip_verbatim), p.file_name()) {
        (Some(d), Some(n)) => d.join(n),
        _ => p,
    }
}

/// Why `d` must never be replaced, even with `--force` (it would delete something that is not a data folder).
fn unsafe_target(d: &Path, repo: &Path, home: Option<&Path>) -> Option<String> {
    if d.parent().is_none() { return Some("it is a filesystem root".into()); }
    if home.is_some_and(|h| h.starts_with(d)) { return Some("it is your home folder or contains it".into()); }
    if repo.starts_with(d) { return Some("it is the repository or contains it".into()); }
    if d.starts_with(repo) && d != repo.join("extracted") && !d.starts_with(repo.join("work")) {
        return Some("inside the repository only extracted/ or a folder under work/ can be a data folder".into());
    }
    if d.join(".git").exists() || d.join("Cargo.toml").exists() { return Some("it holds .git or Cargo.toml".into()); }
    None
}

/// The staging folder and the aside name for the old folder: under `work/` for `<repo>/extracted` (git-ignored, same
/// filesystem), else hidden siblings of `d` (same filesystem, so both renames are atomic).
fn side_dirs(d: &Path, repo: &Path) -> (PathBuf, PathBuf) {
    if d == repo.join("extracted") {
        return (repo.join("work/data-staging"), repo.join("work/data-old"));
    }
    let parent = d.parent().unwrap_or(Path::new("."));
    let name = d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "data".into());
    (parent.join(format!(".{name}.xtask-staging")), parent.join(format!(".{name}.xtask-old")))
}

fn gib(b: u64) -> String {
    if b >= 1 << 30 { format!("{:.2} GiB", b as f64 / (1u64 << 30) as f64) } else { format!("{:.1} MiB", b as f64 / (1u64 << 20) as f64) }
}

fn count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) { out.push(','); }
        out.push(c);
    }
    out
}

/// Files and bytes under `dir` (and, separately, `*.lump` files: the engine cache's lumps).
fn tally(dir: &Path) -> (u64, u64, u64) {
    let (mut files, mut bytes, mut lumps) = (0, 0, 0);
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() { stack.push(e.path()); continue; }
            files += 1;
            bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
            if e.path().extension().is_some_and(|x| x == "lump") { lumps += 1; }
        }
    }
    (files, bytes, lumps)
}

/// The disc images in `~/PS2/ratchet1/`, to suggest a value for `RC_ISO`.
fn iso_suggestion() -> String {
    let found = home_dir().map(|h| h.join("PS2/ratchet1")).and_then(|d| std::fs::read_dir(d).ok()).and_then(|rd| {
        let mut isos: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso"))).collect();
        isos.sort();
        (isos.len() == 1).then(|| isos.remove(0))
    });
    found.map(|p| p.display().to_string()).unwrap_or_else(|| "/path/to/your disc.iso".into())
}

fn print_missing_iso() {
    let iso = iso_suggestion();
    eprintln!(
        "error: regen-data needs your disc image: pass --iso <image>, or set RC_ISO once.\n\
         \n  fish (applies at once, to every fish session, and persists):\n      set -Ux RC_ISO \"{iso}\"\n\
         \n  zsh (then open a new terminal):\n      echo 'export RC_ISO=\"{iso}\"' >> ~/.zshrc\n\
         \n  bash (then open a new terminal):\n      echo 'export RC_ISO=\"{iso}\"' >> ~/.bashrc\n\
         \n  PowerShell on Windows (then open a new terminal):\n      [Environment]::SetEnvironmentVariable('RC_ISO', '{iso}', 'User')\n\
         \nthen run: cargo xtask regen-data"
    );
}

fn print_report(d: &Path, r: &Report) {
    println!("regen-data: data folder {}", d.display());
    println!("  written by rerac-extract:            {:>7} files  {:>10}", count(r.extractor.files), gib(r.extractor.bytes));
    let lt = r.leftover_total();
    if lt.files > 0 {
        println!("  known C++ extractor leftovers:       {:>7} files  {:>10}  (stale; deleted with the old folder)", count(lt.files), gib(lt.bytes));
        for (kind, t) in LEFTOVER_KINDS.iter().zip(&r.leftovers).filter(|(_, t)| t.files > 0) {
            println!("      {:<33}{:>7} files  {:>10}", kind, count(t.files), gib(t.bytes));
        }
    }
    if !r.unknown.is_empty() {
        println!("  unknown (not written by the extractor): {} files  {}", count(r.unknown_tally.files), gib(r.unknown_tally.bytes));
        for p in r.unknown.iter().take(20) { println!("      {p}"); }
        if r.unknown.len() > 20 { println!("      … and {} more", r.unknown.len() - 20); }
    }
}

pub fn run(argv: &[OsString]) -> ExitCode {
    let a = match parse(argv) {
        Ok(a) => a,
        Err(m) => { eprintln!("error: {m} (see cargo xtask help)"); return ExitCode::from(2); }
    };
    let t_all = Instant::now();
    let repo = repo_root();

    // 1. The disc image.
    let iso = a.iso.clone().or_else(|| env_path("RC_ISO"));
    let iso = match (iso, a.check) {
        (Some(p), _) => {
            let p = absolute(&p);
            if !p.is_file() { eprintln!("error: the disc image {} does not exist", p.display()); return ExitCode::from(2); }
            Some(p)
        }
        (None, true) => None,
        (None, false) => { print_missing_iso(); return ExitCode::from(2); }
    };

    // 2. The data folder and what it holds.
    let d = absolute(&a.data_dir.clone().or_else(|| env_path("RC_EXTRACTED")).unwrap_or_else(|| repo.join("extracted")));
    if let Some(why) = unsafe_target(&d, &repo, home_dir().map(|h| absolute(&h)).as_deref()) {
        eprintln!("error: refusing to replace {}: {why}", d.display());
        return ExitCode::FAILURE;
    }
    if d.exists() && !d.is_dir() { eprintln!("error: {} is not a folder", d.display()); return ExitCode::FAILURE; }
    let (staging, old) = side_dirs(&d, &repo);

    // A run killed during the swap: the old folder is aside and the new one not in place yet.
    if !a.check && old.exists() {
        if d.exists() {
            println!("regen-data: deleting {} (the folder a previous run replaced)", old.display());
            if let Err(e) = std::fs::remove_dir_all(&old) { eprintln!("error: cannot delete {}: {e}", old.display()); return ExitCode::FAILURE; }
        } else {
            println!("regen-data: restoring {} from {} (a previous run stopped mid-swap)", d.display(), old.display());
            if let Err(e) = std::fs::rename(&old, &d) { eprintln!("error: cannot restore {}: {e}", d.display()); return ExitCode::FAILURE; }
        }
    }

    let tier0 = match classify::load_tier0(&repo.join("crates/rc-extract/data")) {
        Ok(t) if !t.is_empty() => t,
        Ok(_) => { eprintln!("error: no Tier 0 table in crates/rc-extract/data"); return ExitCode::FAILURE; }
        Err(e) => { eprintln!("error: cannot read crates/rc-extract/data/*.tsv: {e}"); return ExitCode::FAILURE; }
    };
    let before = if d.exists() {
        match classify::scan(&d, &tier0) {
            Ok(r) => { print_report(&d, &r); Some(r) }
            Err(e) => { eprintln!("error: cannot read {}: {e}", d.display()); return ExitCode::FAILURE; }
        }
    } else {
        println!("regen-data: data folder {} (does not exist yet)", d.display());
        None
    };
    let unknown = before.as_ref().map_or(0, |r| r.unknown.len());
    if a.check {
        println!("regen-data --check: nothing changed.{}", if unknown > 0 { " A real run would stop at the unknown files above (or delete them with --force)." } else { "" });
        return ExitCode::SUCCESS;
    }
    if unknown > 0 {
        if !a.force {
            eprintln!(
                "error: {} holds {} unknown file(s) (listed above). regen-data replaces the whole folder, so it stops here.\n\
                 Move them out (generated dev output goes to work/, personal material to ~/PS2/ratchet1/), or pass --force to delete them.",
                d.display(), unknown
            );
            return ExitCode::FAILURE;
        }
        println!("regen-data: --force: the {unknown} unknown file(s) will be deleted with the old folder");
    }
    let iso = iso.expect("checked above");

    // 3. Build the dev extractor, extract into staging, verify.
    if staging.exists() {
        println!("regen-data: deleting the stale staging folder {}", staging.display());
        if let Err(e) = std::fs::remove_dir_all(&staging) { eprintln!("error: cannot delete {}: {e}", staging.display()); return ExitCode::FAILURE; }
    }
    if let Some(p) = staging.parent() {
        if let Err(e) = std::fs::create_dir_all(p) { eprintln!("error: cannot create {}: {e}", p.display()); return ExitCode::FAILURE; }
    }
    let fail = |what: &str| {
        let _ = std::fs::remove_dir_all(&staging);
        eprintln!("error: {what}; {} is unchanged (the staging folder was deleted)", d.display());
        ExitCode::FAILURE
    };
    println!("regen-data: building rerac-extract (dev profile)");
    let t = Instant::now();
    match cargo().args(["build", "-q", "-p", "rc-extract"]).current_dir(&repo).status() {
        Ok(s) if s.success() => {}
        Ok(s) => return fail(&format!("cargo build -p rc-extract failed ({s})")),
        Err(e) => return fail(&format!("cannot run cargo: {e}")),
    }
    let t_build = t.elapsed().as_secs_f64();
    let extractor = |args: &[&std::ffi::OsStr]| cargo().args(["run", "-q", "-p", "rc-extract", "--"]).args(args).current_dir(&repo).status();

    println!("regen-data: extracting {} into {}", iso.display(), staging.display());
    let t = Instant::now();
    match extractor(&["extract".as_ref(), "--iso".as_ref(), iso.as_os_str(), "--out".as_ref(), staging.as_os_str()]) {
        Ok(s) if s.success() => {}
        Ok(s) => return fail(&format!("rerac-extract extract failed ({s})")),
        Err(e) => return fail(&format!("cannot run cargo: {e}")),
    }
    let t_extract = t.elapsed().as_secs_f64();

    println!("regen-data: verifying {}", staging.display());
    let t = Instant::now();
    match extractor(&["verify".as_ref(), "--out".as_ref(), staging.as_os_str()]) {
        Ok(s) if s.success() => {}
        Ok(s) => return fail(&format!("rerac-extract verify failed ({s})")),
        Err(e) => return fail(&format!("cannot run cargo: {e}")),
    }
    let t_verify = t.elapsed().as_secs_f64();

    // 4. Swap: old folder aside, new one in, old one deleted.
    let t = Instant::now();
    let had_old = d.exists();
    if had_old {
        if let Err(e) = std::fs::rename(&d, &old) { return fail(&format!("cannot move {} aside to {}: {e}", d.display(), old.display())); }
    } else if let Some(p) = d.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if let Err(e) = std::fs::rename(&staging, &d) {
        if had_old { let _ = std::fs::rename(&old, &d); }
        return fail(&format!("cannot move {} to {}: {e}", staging.display(), d.display()));
    }
    if had_old {
        if let Err(e) = std::fs::remove_dir_all(&old) {
            eprintln!("warning: the new data is in place, but the old folder {} could not be deleted: {e}", old.display());
        }
    }
    let t_swap = t.elapsed().as_secs_f64();

    // 5. Summary.
    let (files, bytes, _) = tally(&d);
    let (cfiles, cbytes, lumps) = tally(&d.join("cache/v1"));
    println!("\nregen-data: done: {}", d.display());
    println!("  data:     {} files, {} (Tier 0 archive, extract-info.json, engine cache)", count(files), gib(bytes));
    println!("  cache:    cache/v1: {} lumps, {} files, {}", count(lumps), count(cfiles), gib(cbytes));
    println!(
        "  time:     extract + prepare {t_extract:.1} s, verify {t_verify:.1} s, build {t_build:.1} s, swap + delete {t_swap:.1} s; total {:.1} s",
        t_all.elapsed().as_secs_f64()
    );
    if let Some(r) = before {
        let lt = r.leftover_total();
        println!(
            "  removed:  the old folder: {} extractor files, {} C++ leftovers ({}){}",
            count(r.extractor.files), count(lt.files), gib(lt.bytes),
            if r.unknown.is_empty() { String::new() } else { format!(", {} unknown files (--force)", count(r.unknown.len() as u64)) }
        );
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Result<Args, String> { parse(&s.iter().map(OsString::from).collect::<Vec<_>>()) }

    #[test]
    fn parses_flags() {
        assert_eq!(args(&[]).unwrap(), Args::default());
        let a = args(&["--iso", "/a b.iso", "--data-dir=/d", "--force", "--check"]).unwrap();
        assert_eq!(a, Args { iso: Some("/a b.iso".into()), data_dir: Some("/d".into()), force: true, check: true });
        for bad in [&["--iso"][..], &["--bogus"], &["--force=1"], &["--data-dir="], &["extract"]] {
            assert!(args(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn never_replaces_the_repo_home_or_root() {
        let repo = Path::new("/u/me/Repos/checkout");
        let home = Some(Path::new("/u/me"));
        assert_eq!(unsafe_target(&repo.join("extracted"), repo, home), None);
        assert_eq!(unsafe_target(&repo.join("work/scratch-data"), repo, home), None);
        assert_eq!(unsafe_target(Path::new("/u/me/data/rc"), repo, home), None);
        for bad in ["/", "/u", "/u/me", "/u/me/Repos", "/u/me/Repos/checkout", "/u/me/Repos/checkout/docs", "/u/me/Repos/checkout/crates/rc-extract"] {
            assert!(unsafe_target(Path::new(bad), repo, home).is_some(), "{bad}");
        }
    }

    #[test]
    fn drops_the_windows_verbatim_prefix() {
        assert_eq!(strip_verbatim(r"\\?\C:\r\extracted".into()), PathBuf::from(r"C:\r\extracted"));
        assert_eq!(strip_verbatim(r"\\?\UNC\srv\share\r".into()), PathBuf::from(r"\\?\UNC\srv\share\r"));
        assert_eq!(strip_verbatim("/r/extracted".into()), PathBuf::from("/r/extracted"));
    }

    #[test]
    fn staging_sits_on_the_same_filesystem() {
        let repo = Path::new("/r");
        assert_eq!(side_dirs(Path::new("/r/extracted"), repo), (PathBuf::from("/r/work/data-staging"), PathBuf::from("/r/work/data-old")));
        assert_eq!(side_dirs(Path::new("/x/data"), repo), (PathBuf::from("/x/.data.xtask-staging"), PathBuf::from("/x/.data.xtask-old")));
    }

    #[test]
    fn formats_numbers() {
        assert_eq!((count(0), count(999), count(1000), count(20641), count(1234567)), ("0".into(), "999".into(), "1,000".into(), "20,641".into(), "1,234,567".into()));
        assert_eq!((gib(1 << 20), gib(3 << 30)), ("1.0 MiB".into(), "3.00 GiB".into()));
    }
}
