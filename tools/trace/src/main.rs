//! `rc-trace`: compare our reimplementation against the PS2's EE memory. Doc: docs/plan/trace_harness.md,
//! tools/trace/README.md, docs/workflows/pcsx2.md. Dev tool: output goes to `work/trace/`, never `extracted/`.

use anyhow::{bail, Context, Result};
use rc_trace::ee::{self, EeSource, Savestate};
use rc_trace::tfrag_light_cmp::{self as tlc, LevelInputs};
use rc_trace::novalis_spawn as ns;
use rc_trace::spawn_facts::{self as sf, SpawnFacts};
use rc_trace::{default_extracted, pine, trace_out_dir, write_output};
use std::path::{Path, PathBuf};

const USAGE: &str = "\
rc-trace - PCSX2 ground-truth harness (docs/plan/trace_harness.md, docs/workflows/pcsx2.md)

Where things go: generated output (dumps, reports, CSVs) defaults to work/trace/ (RC_WORK overrides
work/); nothing is ever written into extracted/. Personal material lives in ~/PS2/ratchet1/ (RC_PERSONAL
overrides it): kept savestates in savestates/, hero recordings in traces/.

EE memory source (for the commands that read RAM), one of:
  --state <file.p2s | dir | latest | NAME>   PCSX2 savestate; 'latest' = newest SCUS-97199*.p2s in
                                      ~/Library/Application Support/PCSX2/sstates; NAME = a kept
                                      savestate ~/PS2/ratchet1/savestates/NAME.p2s (see save-state)
  --ee <eeMemory.bin>                 raw 32 MiB EE RAM dump
  --pine [slot]                       live read over PCSX2's PINE socket (default slot 28011)

Commands:
  compare-tfrag-light <source> [--level 01] [--extracted DIR] [--csv FILE] [--max-diffs N]
      Lights every tfrag with rc_formats::tfrag_light and compares with the RGBA blocks in RAM.
      Exit status 0 = all equal, 2 = mismatches, 1 = error.
  compare-novalis-spawn <source> [--extracted DIR] [--no-jump] [--ticks N] [--max-diffs N] [--out FILE]
      A savestate taken on Novalis after the landing, standing at the spawn: game-state chunks, hero block,
      follow camera, fog globals, rand state + tick counter, static moby table, tie/shrub lit colours,
      each against the port (rc_game run headless for the tick counter's ticks). Report text to
      work/trace/novalis_spawn_report.txt (--out), moby mismatches to work/trace/novalis_spawn_mobys.csv.
      --no-jump: no scripted ✕ (default: one ✕ tap at the tick the idle counter 0x160ff0 points at).
      --press-ticks N: the scripted ✕ is held N ticks, ending at the idle counter's tick (default 2: the savestate's landing timers match a 2-3 tick press).
      --cutscene-ticks N: the first N port ticks run in game mode 2 (default: ticks − frames in mode 0).
      --load-pre-draws N: extra rand() draws right before the load pass (diagnostic; HeroInit's own draw is in).
      --load-emitters-visible: the load pass's class-27 emitters see the camera (diagnostic: the pre-fix port,
      one particle each; the port's rule is the game's unbuilt view, which culls them).
      --no-audio: run the port without the sound layer (diagnostic: no sound draws on the game stream).
  port-load-pass --out FILE [--load-pre-draws N] [--load-emitters-visible]
      Diagnostic: the port's static mobys right after its load pass, as CSV.
  distill-spawn <source> [--extracted DIR] [--out FILE]
      Distils a Novalis spawn savestate into the committed, numbers-only fixture the test compares the port
      with (default tools/trace/tests/fixtures/novalis_spawn.tsv; the one output that is source, not work/).
      Runs the fixture checks and the savestate checks side by side and refuses to write if they disagree.
  compare-tie-shrub-light <source> [--extracted DIR] [--max-diffs N]
      Only the tie/shrub lit-colour part (level 01).
  save-state NAME [--from FILE.p2s | latest] [--force]
      Keeps a PCSX2 savestate: copies the newest one (or --from) to ~/PS2/ratchet1/savestates/NAME.p2s.
  peek <dis ADDR [N] | words SPEC.. | inst CLASS|-INDEX[,..] [--short] | cuboid I.. | spline I.. | vtbl [CLASS..]> LEVEL
      Porting aids, read-only: the level overlay disassembled from ADDR (hex, N instructions), words at gp offsets
      (`-5264` or the decompiler's `ad9c`) or absolute addresses (`@1ba5d0`), moby instances with their pvar words,
      cuboids, splines, the class table. Usage: `peek dis 03 2cca00 80`.
  disc-check [--iso IMAGE] [--extracted DIR] [--extract-into DIR]
      The checks that need your disc image (no test reads it; docs/workflows/testing.md §10): the
      disc reader against extracted/ on every level, the save_game lump and SYSTEM.CNF, and the extractor's
      size/SHA-1 table against the committed one. --extract-into also extracts twice (full, --ntsc-only; ~7 GiB)
      into that scratch folder and checks the file set and hashes. Default image: RC_ISO, else the ISO in the
      personal folder. Use --release. Exit status 0 = all checks pass, 1 = a check failed.
  overlay-diff [--cite DIR]... [--fn L01:ADDR]... [--callees N] [--name NAME] [--out DIR] [--max-lines N]
      The masked function x level diff (docs/workflows/ghidra.md, masked overlay diff): the rows are the
      level-01 (or level-00) functions cited in the .rs files under each --cite DIR plus each --fn, and their
      overlay callees N calls deep; every level's counterpart is compared after masking the relocated fields.
      Default: --cite crates/rc-game/src/hero --callees 1 --name hero. Writes NAME.tsv, NAME_matrix.txt and
      NAME_diffs.txt (only the differing instructions) to work/overlay_diff/ (--out).
  class-census [--out DIR] [--tags FILE] [--extracted DIR]
      Static census of the unported moby classes on levels 00-18 (docs/plan/class_census.md): per class the
      placed / created instances, its class-port unit (update + private callees, grouped across levels by
      code identity) and the shared functions it calls. Writes classes.tsv, units.tsv, shared.tsv and, with
      the tag table (default tools/ghidra/names/census_systems.tsv), unit_systems.tsv + systems.tsv to
      work/census/ (--out).
  death-census [--out DIR] [--extracted DIR]
      For every placed or ported moby class on levels 00-18: its call path to SetDeathBits and the functions it
      reaches that form a persistent death-bit or visit-table address (docs/plan/game_state.md §6.1). Writes
      deaths.tsv to work/census/ (--out).
  info --state <p2s>                  list savestate entries and version
  dump-ee <source> [--out FILE] [--entry NAME]   write EE RAM (or any savestate entry) to FILE
                                      (default work/trace/<savestate name>_ee.bin)
  find <source> --bytes HEX [--align N] [--max N]   list EE addresses holding the bytes
  read <source> --addr HEX --len N    hex-dump EE memory
  pine-info [slot]                    print PCSX2 version / game / status over PINE
  pine-savestate <state-slot> [--slot N]   ask PCSX2 to save to a state slot (like pressing F1)
  pine-write --addr HEX --bytes HEX [--slot N]   write bytes (e.g. 01ff) to EE memory over PINE (dev pokes)
  synth [--level 01] [--unlit] --out FILE [--base HEX]   write a synthetic 'loaded level' EE image

Hero feel pass (docs/plan/hero_feel_pass.md):
  record [--pine [slot]] [--out FILE] [--seconds N] [--savestate N] [--poll-us N] [--quiet]
      Live per-tick recorder of Ratchet over PINE (read-only). One batched PINE read per poll; each new tick
      is written once (TSV with a documented header). Default file ~/PS2/ratchet1/traces/hero_<UTC>.tsv.
      Stops on Enter or after --seconds. --savestate N first asks PCSX2 to save state slot N (off by default).
  replay-hero --trace FILE [--extracted DIR] [--level N] [--start TICK] [--ticks N] [--tol X]
              [--camera-at-hero] [--out-port FILE] [--report FILE] [--states N]
      Runs the port's hero headless on the recorded level from the first standing sample, fed the recorded
      pad bytes, and diffs per tick: first divergence, per-field max / mean error, per-jump table (PCSX2 vs
      port), state sequences side by side. Writes work/trace/<trace name>.port.tsv and .report.txt (full curves).
  hero-jumps --trace FILE        the per-jump table of one trace
  hero-snap <source>             one hero sample from a savestate / EE dump / PINE (checks the addresses)
";

struct Args(Vec<String>);

impl Args {
    fn flag(&mut self, name: &str) -> bool {
        match self.0.iter().position(|a| a == name) {
            Some(i) => { self.0.remove(i); true }
            None => false,
        }
    }
    fn opt(&mut self, name: &str) -> Result<Option<String>> {
        let Some(i) = self.0.iter().position(|a| a == name) else { return Ok(None) };
        if i + 1 >= self.0.len() { bail!("{name} needs a value"); }
        let v = self.0.remove(i + 1);
        self.0.remove(i);
        Ok(Some(v))
    }
    /// `--pine` with an optional numeric slot after it.
    fn pine(&mut self) -> Option<u16> {
        let i = self.0.iter().position(|a| a == "--pine")?;
        self.0.remove(i);
        if let Some(s) = self.0.get(i).and_then(|s| s.parse().ok()) { self.0.remove(i); Some(s) } else { Some(pine::DEFAULT_SLOT) }
    }
    fn source(&mut self) -> Result<EeSource> {
        if let Some(s) = self.opt("--state")? { return Ok(EeSource::State(ee::resolve_state(&s)?)); }
        if let Some(p) = self.opt("--ee")? { return Ok(EeSource::Raw(p.into())); }
        if let Some(slot) = self.pine() { return Ok(EeSource::Pine(slot)); }
        bail!("need an EE memory source: --state, --ee or --pine\n\n{USAGE}")
    }
    fn level(&mut self) -> Result<u32> { Ok(self.opt("--level")?.map(|l| l.parse()).transpose().context("--level")?.unwrap_or(1)) }
    fn extracted(&mut self) -> Result<PathBuf> { Ok(self.opt("--extracted")?.map(PathBuf::from).unwrap_or_else(default_extracted)) }
    fn done(&self) -> Result<()> {
        if !self.0.is_empty() { bail!("unexpected arguments: {:?}\n\n{USAGE}", self.0); }
        Ok(())
    }
}

fn hex_u32(s: &str) -> Result<u32> { u32::from_str_radix(s.trim_start_matches("0x"), 16).with_context(|| format!("bad hex {s:?}")) }

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let mut a = Args(std::env::args().skip(1).collect());
    if a.0.is_empty() || a.flag("--help") || a.flag("-h") {
        print!("{USAGE}");
        return Ok(0);
    }
    let cmd = a.0.remove(0);
    match cmd.as_str() {
        "class-census" => {
            let out = a.opt("--out")?.map(PathBuf::from).unwrap_or_else(|| rc_trace::work_dir().join("census"));
            let tags = a.opt("--tags")?.map(PathBuf::from).unwrap_or_else(|| rc_trace::repo_root().join("tools/ghidra/names/census_systems.tsv"));
            let extracted = a.extracted()?;
            a.done()?;
            let levels: Vec<u32> = (0..19).collect();
            let s = rc_trace::class_census::run(&extracted, &rc_trace::repo_root(), &rc_trace::work_dir(), &out, Some(&tags), &levels)?;
            println!(
                "class census: {} unported class-levels ({} placed, {} created instances), {} class-port units, {} shared functions -> {}",
                s.classes, s.placed, s.created, s.units, s.shared, s.out.display()
            );
            Ok(0)
        }
        "death-census" => {
            let out = a.opt("--out")?.map(PathBuf::from).unwrap_or_else(|| rc_trace::work_dir().join("census"));
            let extracted = a.extracted()?;
            a.done()?;
            let n = rc_trace::class_census::deaths(&extracted, &rc_trace::repo_root(), &rc_trace::work_dir(), &out)?;
            println!("death census: {n} classes -> {}", out.join("deaths.tsv").display());
            Ok(0)
        }
        "overlay-diff" => {
            let mut cite = Vec::new();
            while let Some(d) = a.opt("--cite")? { cite.push(PathBuf::from(d)); }
            let mut fns = Vec::new();
            while let Some(f) = a.opt("--fn")? {
                let (l, x) = f.split_once(':').context("--fn LNN:ADDR")?;
                let l: u32 = l.trim_start_matches('L').parse().context("--fn level")?;
                if l > 1 { bail!("--fn: the reference is level 00 or 01"); }
                fns.push((l, u32::from_str_radix(x.trim_start_matches("0x"), 16).context("--fn address")?));
            }
            let defaults = cite.is_empty() && fns.is_empty();
            if defaults { cite.push(PathBuf::from("crates/rc-game/src/hero")); }
            let callee_depth = a.opt("--callees")?.map(|v| v.parse()).transpose()?.unwrap_or(if defaults { 1 } else { 0 });
            let name = a.opt("--name")?.unwrap_or_else(|| if defaults { "hero".into() } else { "functions".into() });
            let out = a.opt("--out")?.map(PathBuf::from).unwrap_or_else(|| rc_trace::work_dir().join("overlay_diff"));
            let max_lines = a.opt("--max-lines")?.map(|v| v.parse()).transpose()?.unwrap_or(24);
            let extracted = a.extracted()?;
            a.done()?;
            let o = rc_trace::overlay_diff::Options { cite, fns, callee_depth, name, out, max_lines };
            let s = rc_trace::overlay_diff::run(&extracted, &rc_trace::work_dir(), &rc_trace::repo_root(), &o)?;
            println!("overlay-diff: {} rows, cells {:?}, {} rows with own differences -> {}", s.rows, s.counts, s.own_diff_rows, s.out.display());
            Ok(0)
        }
        "peek" => {
            let extracted = a.extracted()?;
            let short = a.flag("--short");
            if a.0.len() < 2 { bail!("peek <dis|words|inst|cuboid|spline|vtbl> LEVEL ..."); }
            let what = a.0.remove(0);
            let level: u32 = a.0.remove(0).parse().context("peek level")?;
            let rest = std::mem::take(&mut a.0);
            let hex = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).context("hex address");
            let nums = |v: &[String]| -> Result<Vec<i32>> { v.iter().flat_map(|s| s.split(',')).map(|x| x.parse::<i32>().context("number")).collect() };
            use rc_trace::level_peek as lp;
            match what.as_str() {
                "dis" => lp::dis(&extracted, level, hex(rest.first().context("address")?)?, rest.get(1).map_or(Ok(40), |n| n.parse()).context("count")?)?,
                "words" => lp::words(&extracted, level, &rest)?,
                "inst" => lp::instances(level, &nums(&rest)?, short)?,
                "cuboid" => lp::cuboids(level, &nums(&rest)?.into_iter().map(|x| x as usize).collect::<Vec<_>>())?,
                "spline" => lp::splines(level, &nums(&rest)?.into_iter().map(|x| x as usize).collect::<Vec<_>>())?,
                "vtbl" => lp::vtbl(&extracted, level, &nums(&rest)?)?,
                _ => bail!("peek: unknown {what}"),
            }
            Ok(0)
        }
        "disc-check" => {
            let iso = match a.opt("--iso")? {
                Some(p) => PathBuf::from(p),
                None => std::env::var_os("RC_ISO").filter(|v| !v.is_empty()).map(PathBuf::from)
                    .unwrap_or_else(|| rc_trace::personal_dir().join("Ratchet & Clank (USA) (En,Fr,De,Es,It).iso")),
            };
            let extracted = a.extracted()?;
            let into = a.opt("--extract-into")?.map(PathBuf::from);
            a.done()?;
            rc_trace::disc_check::run(&iso, &extracted, into.as_deref())?;
            println!("disc-check: all checks passed");
            Ok(0)
        }
        "compare-tfrag-light" => compare_tfrag_light(a),
        "compare-novalis-spawn" => compare_novalis_spawn(a),
        "distill-spawn" => distill_spawn(a),
        "save-state" => save_state(a),
        "port-load-pass" => {
            // Diagnostic: the port's static mobys right after its load pass (no savestate needed).
            let extracted = a.extracted()?;
            let pre: u32 = a.opt("--load-pre-draws")?.map(|s| s.parse()).transpose()?.unwrap_or(0);
            let visible = a.flag("--load-emitters-visible");
            let out = PathBuf::from(a.opt("--out")?.context("--out required")?);
            a.done()?;
            let lv = rc_trace::port_sim::LevelData::load(&extracted, 1)?;
            let sim = rc_trace::port_sim::PortSim::with_options(&lv, &rc_trace::port_sim::SimOptions { load_pre_draws: pre, load_emitters_visible: visible, ..Default::default() })?;
            let mut s = String::from("index,instance,o_class,state,mode,x,y,z,rx,ry,rz,pvar55,seq_b,frame_b\n");
            for (i, m) in sim.game.mobys.mobys.iter().enumerate().take(sim.n_static) {
                s += &format!("{i},{},{},{},{},{},{},{},{},{},{},{},{},{}\n", sim.moby_to_instance[i], m.o_class, m.state, m.mode, m.position[0], m.position[1], m.position[2],
                    m.rotation[0], m.rotation[1], m.rotation[2], m.pvars.get(0x55).copied().unwrap_or(0), m.anim.seq_b, m.anim.frame_b);
            }
            write_output(&out, s)?;
            println!("wrote {} (load pass: {} draws, rng {:#010x})", out.display(), sim.load_draws, sim.game.rng.state);
            Ok(0)
        }
        "compare-tie-shrub-light" => {
            let extracted = a.extracted()?;
            let max: usize = a.opt("--max-diffs")?.map(|s| s.parse()).transpose()?.unwrap_or(20);
            let src = a.source()?;
            a.done()?;
            let img = src.load()?;
            let mut out = ns::Out::default();
            ns::check_tie_shrub(&img, &extracted, &mut out, max)?;
            Ok(if out.tallies.iter().all(|t| t.1 == t.2) { 0 } else { 2 })
        }
        "info" => {
            let p = ee::resolve_state(&a.opt("--state")?.context("--state required")?)?;
            a.done()?;
            let s = Savestate::open(&p)?;
            println!("{}", p.display());
            if let Ok(v) = s.version() { println!("  save version {:#010x} (major {:#06x}), PCSX2 {}", v.save_version, v.save_version >> 16, v.build); }
            for e in &s.archive.entries {
                println!("  {:<34} method {:>2}  {:>10} -> {:>10} bytes", e.name, e.method, e.compressed_size, e.size);
            }
            Ok(0)
        }
        "dump-ee" => {
            let out = a.opt("--out")?.map(PathBuf::from);
            let entry = a.opt("--entry")?;
            let src = a.source()?;
            a.done()?;
            let out = out.unwrap_or_else(|| {
                let stem = match &src { EeSource::State(p) | EeSource::Raw(p) => p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(), EeSource::Pine(_) => "pine".into() };
                trace_out_dir().join(match &entry { Some(e) => format!("{stem}_{e}"), None => format!("{stem}_ee.bin") })
            });
            let bytes = match (entry, &src) {
                (Some(e), EeSource::State(p)) => Savestate::open(p)?.archive.read(&e)?,
                (Some(_), _) => bail!("--entry needs --state"),
                (None, s) => s.load()?.ram,
            };
            write_output(&out, &bytes)?;
            println!("wrote {} ({} bytes)", out.display(), bytes.len());
            Ok(0)
        }
        "find" => {
            let hex = a.opt("--bytes")?.context("--bytes required")?;
            let align: usize = a.opt("--align")?.map(|s| s.parse()).transpose()?.unwrap_or(2);
            let max: usize = a.opt("--max")?.map(|s| s.parse()).transpose()?.unwrap_or(64);
            let src = a.source()?;
            a.done()?;
            let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if !clean.len().is_multiple_of(2) || clean.is_empty() { bail!("--bytes needs an even number of hex digits"); }
            let pat: Vec<u8> = (0..clean.len()).step_by(2).map(|i| u8::from_str_radix(&clean[i..i + 2], 16).unwrap()).collect();
            let img = src.load()?;
            let hits = img.find(&pat, align);
            println!("{} hit(s) for {} bytes (align {align})", hits.len(), pat.len());
            for h in hits.iter().take(max) { println!("  {h:#010x}"); }
            Ok(0)
        }
        "read" => {
            let addr = hex_u32(&a.opt("--addr")?.context("--addr required")?)?;
            let len: usize = a.opt("--len")?.map(|s| s.parse()).transpose()?.unwrap_or(0x40);
            let src = a.source()?;
            a.done()?;
            let img = src.load()?;
            for (k, row) in img.bytes(addr, len)?.chunks(16).enumerate() {
                let hex: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
                println!("{:#010x}: {}", addr as usize + 16 * k, hex.join(" "));
            }
            Ok(0)
        }
        "pine-info" => {
            let slot = a.0.first().and_then(|s| s.parse().ok()).unwrap_or(pine::DEFAULT_SLOT);
            let mut c = pine::Pine::connect(slot)?;
            println!("socket  {}", pine::socket_path(slot).display());
            println!("version {}", c.version()?);
            println!("status  {}", c.status_name());
            println!("game    {} | {}", c.game_id().unwrap_or_default(), c.title().unwrap_or_default());
            Ok(0)
        }
        "pine-write" => {
            let slot: u16 = a.opt("--slot")?.map(|s| s.parse()).transpose()?.unwrap_or(pine::DEFAULT_SLOT);
            let addr = hex_u32(&a.opt("--addr")?.context("--addr required")?)?;
            let hex = a.opt("--bytes")?.context("--bytes required")?;
            a.done()?;
            if !hex.len().is_multiple_of(2) { bail!("--bytes: an even number of hex digits"); }
            let bytes = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).with_context(|| format!("bad hex {hex:?}"))).collect::<Result<Vec<_>>>()?;
            pine::Pine::connect(slot)?.write(addr, &bytes)?;
            println!("wrote {} bytes at {addr:#x}", bytes.len());
            Ok(0)
        }
        "pine-savestate" => {
            let slot: u16 = a.opt("--slot")?.map(|s| s.parse()).transpose()?.unwrap_or(pine::DEFAULT_SLOT);
            let state: u8 = a.0.first().context("state slot number required")?.parse()?;
            pine::Pine::connect(slot)?.save_state(state)?;
            println!("requested save to state slot {state}; the file appears in {}", ee::pcsx2_sstates_dir().unwrap_or_default().display());
            Ok(0)
        }
        "synth" => {
            let level = a.level()?;
            let extracted = a.extracted()?;
            let unlit = a.flag("--unlit");
            let base = a.opt("--base")?.map(|s| hex_u32(&s)).transpose()?.unwrap_or(0x0040_0000);
            let out = PathBuf::from(a.opt("--out")?.context("--out required")?);
            a.done()?;
            let lvl = LevelInputs::load(&extracted, level)?;
            let img = tlc::synthesize(&lvl, base, !unlit);
            write_output(&out, &img.ram)?;
            println!("wrote {} ({})", out.display(), img.source);
            Ok(0)
        }
        "record" => record(a),
        "replay-hero" => replay_hero(a),
        "hero-jumps" => {
            let t = rc_trace::hero_trace::Trace::read(&PathBuf::from(a.opt("--trace")?.context("--trace required")?))?;
            a.done()?;
            let j = rc_trace::hero_analysis::segment(&t.samples);
            print!("{}", rc_trace::hero_analysis::jumps_table(&rc_trace::hero_analysis::jump_stats(&t.samples, &j)));
            Ok(0)
        }
        "hero-snap" => {
            let src = a.source()?;
            a.done()?;
            let img = src.load()?;
            let mem: &dyn rc_trace::hero_trace::Mem = &img;
            let moby = img.u32(rc_trace::hero_trace::MOBY_PTR)?;
            let level = img.u32(rc_trace::hero_trace::LEVEL)? as i32;
            let hero = rc_trace::hero_trace::hero_pos(mem).context("hero position")?;
            let cam = rc_trace::hero_trace::locate_camera(mem, level, hero);
            let s = rc_trace::hero_trace::sample_from(mem, moby, cam)?;
            println!("{}: level {level}, moby {moby:#x}, camera {}", img.source, cam.map_or("not found".into(), |c| format!("{c:#x}")));
            let row = s.row();
            for (f, v) in rc_trace::hero_trace::FIELDS.iter().zip(row.split('\t')) { println!("  {:<14} {:<24} {}", f.name, f.addr, v); }
            Ok(0)
        }
        _ => bail!("unknown command {cmd:?}\n\n{USAGE}"),
    }
}

fn compare_tfrag_light(mut a: Args) -> Result<i32> {
    let level = a.level()?;
    let extracted = a.extracted()?;
    let csv = a.opt("--csv")?.map(PathBuf::from).unwrap_or_else(|| trace_out_dir().join(format!("level{level:02}_tfrag_light_mismatches.csv")));
    let max_diffs: usize = a.opt("--max-diffs")?.map(|s| s.parse()).transpose()?.unwrap_or(20);
    let src = a.source()?;
    a.done()?;

    let lvl = LevelInputs::load(&extracted, level)?;
    let img = src.load()?;
    println!("level {level:02}: {} tfrags, {} directional light sets (disc); EE image: {}", lvl.tfrags.len(), lvl.bank.count, img.source);

    let loc = tlc::locate(&img, &lvl)?;
    println!(
        "tfrag header table at EE {:#010x} (block {:#010x}); {} candidate(s) for tfrag 0's bsphere, {}/{} headers equal on load-invariant bytes",
        loc.table, loc.table - lvl.table_offset as u32, loc.candidates, loc.headers_matching, lvl.tfrags.len()
    );
    println!("  data pointers relocated to absolute: {}/{} (still disc-relative: {})", loc.relocated, lvl.tfrags.len(), loc.unrelocated);
    if let Some((g, v, implied, ok)) = loc.core_ptr_check {
        println!("  level global {g:#x} (core-data base) = {v:#010x} -> tfrags block {implied:#010x}: {}", if ok { "agrees with the search" } else { "DISAGREES with the search" });
    }
    let vol: Vec<String> = loc.volatile_diffs.iter().filter(|d| d.2 > 0).map(|(o, n, c)| format!("{n}(0x{o:02x}) x{c}")).collect();
    if !vol.is_empty() { println!("  header bytes changed at run time vs disc: {}", vol.join(", ")); }
    if loc.relocated * 2 < lvl.tfrags.len() {
        println!("  WARNING: most data pointers are not relocated; the level may not have finished loading");
    }

    let bank = tlc::locate_bank(&img, &lvl);
    match &bank {
        Some(b) => println!("directional light bank at EE {:#010x} ({}): {}", b.addr, b.how, if b.equals_disc { "equals the disc gameplay bank" } else { "DIFFERS from the disc gameplay bank" }),
        None => println!("directional light bank: not found (tfrags with point lights will be skipped)"),
    }
    let points = bank.as_ref().map(|b| &b.points);

    println!("\nours (disc light bank) vs RAM:");
    let r = tlc::compare(&img, &lvl, &loc, &lvl.bank, points)?;
    r.print(max_diffs);
    if !r.diffs.is_empty() {
        r.write_csv(&csv)?;
        println!("  all {} mismatching vertices written to {}", r.diffs.len(), csv.display());
    }
    let mut ok = r.all_equal();
    if let Some(b) = bank.as_ref().filter(|b| !b.equals_disc) {
        println!("\nours (light bank read from RAM) vs RAM:");
        let r2 = tlc::compare(&img, &lvl, &loc, &b.bank, points)?;
        r2.print(max_diffs);
        ok = r2.all_equal();
    }
    println!("\nresult: {}", if ok { "MATCH - every lit vertex byte equals the game's" } else { "MISMATCH" });
    Ok(if ok { 0 } else { 2 })
}

fn compare_novalis_spawn(mut a: Args) -> Result<i32> {
    let extracted = a.extracted()?;
    let no_jump = a.flag("--no-jump");
    let ticks: Option<u64> = a.opt("--ticks")?.map(|s| s.parse()).transpose().context("--ticks")?;
    let max: usize = a.opt("--max-diffs")?.map(|s| s.parse()).transpose()?.unwrap_or(20);
    let pre_draws: u32 = a.opt("--load-pre-draws")?.map(|s| s.parse()).transpose().context("--load-pre-draws")?.unwrap_or(0);
    let emitters_visible = a.flag("--load-emitters-visible");
    let no_audio = a.flag("--no-audio");
    let hold: u64 = a.opt("--press-ticks")?.map(|s| s.parse()).transpose().context("--press-ticks")?.unwrap_or(2);
    let cutscene: Option<u64> = a.opt("--cutscene-ticks")?.map(|s| s.parse()).transpose().context("--cutscene-ticks")?;
    let report = a.opt("--out")?.map(PathBuf::from).unwrap_or_else(|| trace_out_dir().join("novalis_spawn_report.txt"));
    let src = a.source()?;
    a.done()?;
    let img = src.load()?;
    let mut out = ns::Out::default();
    let level = img.u32(ns::LEVEL)?;
    if level != 1 { bail!("level global 0x15ed84 = {level}: this savestate is not on Novalis"); }
    let mut tl = ns::Timeline::read(&img)?;
    if let Some(t) = ticks { tl.ticks = t; }
    let vsync = img.u32(ns::VSYNC)?;
    let play = img.u32(ns::PLAY_TIME)?;
    println!("EE image: {}", img.source);
    println!(
        "timeline: tick counter 0x15f5cc = {} (port ticks {}), mode 0x15f5c4 = {} for {} frames, idle counter 0x160ff0 = {} (last pad input at port tick {:?}), vsync 0x15f3f8 = {vsync}, play time 0x15eea4 = {play}",
        tl.tick_counter, tl.ticks, tl.mode, tl.mode_frames, tl.idle, tl.press_at
    );
    let gs = ns::port_game_state(&extracted)?;
    ns::check_game_state(&img, &gs, &mut out)?;
    let lv = rc_trace::port_sim::LevelData::load(&extracted, 1)?;
    let t0 = std::time::Instant::now();
    let opt = rc_trace::port_sim::SimOptions { load_pre_draws: pre_draws, load_emitters_visible: emitters_visible, cutscene_ticks: cutscene.unwrap_or(tl.ticks.saturating_sub(tl.mode_frames.max(0) as u64)), no_audio };
    println!("port options: {opt:?}");
    let hold = if no_jump { 0 } else { hold };
    let sim = ns::run_port_hold(&lv, &tl, hold, &opt)?;
    println!("\n(port: load pass + {} ticks in {:.1} s{})", tl.ticks, t0.elapsed().as_secs_f64(), if no_jump { String::new() } else { format!(", ✕ held {hold} tick(s) ending at port tick {:?}", tl.press_at) });
    ns::check_hero(&img, &sim, &mut out)?;
    ns::check_camera(&img, &sim, &mut out)?;
    ns::check_fog(&img, &lv, &sim, &mut out)?;
    ns::check_rng(&img, &sim, &tl, &mut out)?;
    ns::check_mobys(&img, &sim, &mut out, &trace_out_dir().join("novalis_spawn_mobys.csv"))?;
    ns::check_tie_shrub(&img, &extracted, &mut out, max)?;
    println!("\n== summary (matching / total)");
    for (name, ok, total) in &out.tallies {
        println!("  {:<46} {:>7} / {:<7} {}", name, ok, total, if ok == total { "ok" } else { "MISMATCH" });
    }
    write_output(&report, &out.text)?;
    println!("report written to {}", report.display());
    Ok(if out.tallies.iter().all(|t| t.1 == t.2) { 0 } else { 2 })
}

fn record(mut a: Args) -> Result<i32> {
    use rc_trace::hero_record as hr;
    let slot = a.pine().unwrap_or(pine::DEFAULT_SLOT);
    let out = a.opt("--out")?.map(PathBuf::from).unwrap_or_else(|| hr::default_dir().join(format!("hero_{}.tsv", hr::utc_stamp())));
    let seconds: Option<f64> = a.opt("--seconds")?.map(|s| s.parse()).transpose().context("--seconds")?;
    let savestate: Option<u8> = a.opt("--savestate")?.map(|s| s.parse()).transpose().context("--savestate")?;
    let poll_us: u64 = a.opt("--poll-us")?.map(|s| s.parse()).transpose().context("--poll-us")?.unwrap_or(250);
    let quiet = a.flag("--quiet");
    a.done()?;
    let sum = hr::record(&hr::RecordOptions { slot, out, seconds, savestate, poll_us, quiet })?;
    Ok(if sum.rows == 0 { 2 } else { 0 })
}

fn replay_hero(mut a: Args) -> Result<i32> {
    use rc_trace::hero_analysis as ha;
    use rc_trace::hero_replay as hrp;
    use rc_trace::hero_trace::Trace;
    let path = PathBuf::from(a.opt("--trace")?.context("--trace required")?);
    let extracted = a.extracted()?;
    let level: Option<u32> = a.opt("--level")?.map(|s| s.parse()).transpose().context("--level")?;
    let start_tick: Option<u32> = a.opt("--start")?.map(|s| s.parse()).transpose().context("--start")?;
    let ticks: Option<u32> = a.opt("--ticks")?.map(|s| s.parse()).transpose().context("--ticks")?;
    let tol: f64 = a.opt("--tol")?.map(|s| s.parse()).transpose().context("--tol")?.unwrap_or(1e-3);
    let max_states: usize = a.opt("--states")?.map(|s| s.parse()).transpose().context("--states")?.unwrap_or(80);
    let camera_at_hero = a.flag("--camera-at-hero");
    // Generated output goes to work/trace/, not next to the recording (~/PS2/ratchet1/traces/ is personal).
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "hero".into());
    let out_port = a.opt("--out-port")?.map(PathBuf::from).unwrap_or_else(|| trace_out_dir().join(format!("{stem}.port.tsv")));
    let report = a.opt("--report")?.map(PathBuf::from).unwrap_or_else(|| trace_out_dir().join(format!("{stem}.report.txt")));
    a.done()?;
    let trace = Trace::read(&path)?;
    if trace.samples.is_empty() { bail!("{} has no samples", path.display()); }
    let start = match start_tick {
        Some(t) => Some(trace.samples.iter().position(|s| s.tick >= t).context("--start beyond the trace")?),
        None => None,
    };
    let lv_n = level.unwrap_or(trace.samples[start.unwrap_or(0)].level.max(0) as u32);
    let lv = hrp::HeroLevel::load(&extracted, lv_n)?;
    let t0 = std::time::Instant::now();
    let r = hrp::replay(&lv, &trace, &hrp::ReplayOptions { start, ticks, camera_at_hero })?;
    let rec = &trace.samples[r.start..];
    let rec = &rec[..rec.len().min(r.samples.len())];
    let mut port = Trace { meta: trace.meta.clone(), samples: r.samples.clone() };
    port.set_meta("source", "rc-trace replay-hero (port)");
    port.set_meta("replayed_from", path.display().to_string());
    port.write(&out_port)?;
    let names = ("pcsx2", "port");
    let mut head = format!("replay of {} on level {lv_n}: {} ticks from tick {} (sample {}) in {:.2} s; {} missed ticks filled with the next pad\n",
        path.display(), r.samples.len().saturating_sub(1), rec[0].tick, r.start, t0.elapsed().as_secs_f64(), r.filled);
    for n in &r.notes { head += &format!("note: {n}\n"); }
    // The start sample is the placement, not a result: the diff starts at the first replayed tick.
    let d = ha::diff(&rec[1..], &r.samples[1..], tol);
    let ja = ha::jump_stats(rec, &ha::segment(rec));
    let jb = ha::jump_stats(&r.samples, &ha::segment(&r.samples));
    let body = format!(
        "\n== diff (tolerance {tol} on floats, integers exact)\n{}\n== jumps (k-th vs k-th)\n{}\n== states side by side\n",
        ha::diff_text(&d, names), ha::jumps_compare(&ja, &jb, names)
    );
    print!("{head}{body}{}", ha::states_side_by_side(rec, &r.samples, names, max_states));
    let full = format!("{head}{body}{}\n== curves\n{}", ha::states_side_by_side(rec, &r.samples, names, usize::MAX), ha::curves_compare(&ja, &jb, names));
    write_output(&report, full)?;
    println!("\nport trace: {}\nreport (with the per-tick curves): {}", out_port.display(), report.display());
    Ok(if d.first.is_none() { 0 } else { 2 })
}

/// `distill-spawn`: the savestate's spawn facts as the committed fixture, after checking that the fixture
/// checks give the same tallies as the savestate checks on this very image.
fn distill_spawn(mut a: Args) -> Result<i32> {
    let extracted = a.extracted()?;
    let out = a.opt("--out")?.map(PathBuf::from).unwrap_or_else(rc_trace::spawn_fixture);
    let src = a.source()?;
    a.done()?;
    let img = src.load()?;
    let level = img.u32(ns::LEVEL)?;
    if level != 1 { bail!("level global 0x15ed84 = {level}: this savestate is not on Novalis"); }
    let facts = SpawnFacts::from_ee(&img, &extracted)?;
    let header = format!(
        "Novalis spawn facts, distilled by `cargo run -p rc-trace -- distill-spawn` from {}.\n\
         Compared with the port by tools/trace/tests/trace/novalis_spawn.rs (needs only extracted/). Doc: docs/workflows/pcsx2.md.",
        img.source.rsplit('/').next().unwrap_or("")
    );
    let text = facts.to_tsv(&header);
    let back = SpawnFacts::parse_tsv(&text)?;
    if back != facts { bail!("the TSV does not round-trip (a writer/parser bug)"); }
    // The same checks, on the facts and on the EE image.
    let fx = sf::check_all(&back, &extracted)?;
    let mut ee_out = ns::Out::default();
    ns::check_tie_shrub(&img, &extracted, &mut ee_out, 0)?;
    let gs = ns::port_game_state(&extracted)?;
    ns::check_game_state(&img, &gs, &mut ee_out)?;
    let tl = ns::Timeline::read(&img)?;
    let lv = rc_trace::port_sim::LevelData::load(&extracted, 1)?;
    let opt = rc_trace::port_sim::SimOptions { cutscene_ticks: tl.ticks.saturating_sub(tl.mode_frames.max(0) as u64), ..Default::default() };
    let sim = ns::run_port_hold(&lv, &tl, 2, &opt)?;
    ns::check_rng(&img, &sim, &tl, &mut ee_out)?;
    ns::check_fog(&img, &lv, &sim, &mut ee_out)?;
    ns::check_mobys(&img, &sim, &mut ee_out, &trace_out_dir().join("novalis_spawn_mobys.csv"))?;
    println!("\n== fixture checks vs savestate checks (matching / total)");
    for (name, ok, total) in &fx.tallies {
        let e = ee_out.tallies.iter().find(|t| &t.0 == name).map_or("-".to_string(), |t| format!("{} / {}", t.1, t.2));
        println!("  {name:<46} fixture {ok:>6} / {total:<6} savestate {e}");
    }
    let bad = sf::disagreements(&fx, &ee_out);
    if !bad.is_empty() { bail!("fixture and savestate checks disagree: {bad:?}"); }
    write_output(&out, &text)?;
    println!("wrote {} ({} bytes: {} chunks, {} moby slots, {} ties, {} shrubs)", out.display(), text.len(), facts.chunks.len(), facts.mobys.len(), facts.ties.len(), facts.shrubs.len());
    Ok(0)
}

/// `save-state NAME`: keep a PCSX2 savestate under ~/PS2/ratchet1/savestates/NAME.p2s.
fn save_state(mut a: Args) -> Result<i32> {
    let from = a.opt("--from")?;
    let force = a.flag("--force");
    let name = if a.0.is_empty() { bail!("save-state needs a NAME\n\n{USAGE}") } else { a.0.remove(0) };
    a.done()?;
    if name.contains('/') || name.is_empty() { bail!("NAME is a file name, not a path: {name:?}"); }
    let src = match from.as_deref() {
        None | Some("latest") => ee::newest_state(&ee::pcsx2_sstates_dir().context("HOME not set")?)?,
        Some(p) => PathBuf::from(p),
    };
    Savestate::open(&src).with_context(|| format!("{} is not a readable savestate", src.display()))?;
    let dst = rc_trace::savestates_dir().join(if name.ends_with(".p2s") { name.clone() } else { format!("{name}.p2s") });
    if dst.exists() && !force { bail!("{} exists (--force to replace it)", dst.display()); }
    copy_state(&src, &dst)?;
    println!("kept {} as {}", src.display(), dst.display());
    Ok(0)
}

fn copy_state(src: &Path, dst: &Path) -> Result<()> {
    if let Some(d) = dst.parent() { std::fs::create_dir_all(d)?; }
    std::fs::copy(src, dst).with_context(|| format!("copying {} to {}", src.display(), dst.display()))?;
    Ok(())
}
