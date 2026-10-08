//! `class-census`: the static census of the unported moby classes (docs/plan/class_census.md).
//!
//! For every level 00–18 it reads the overlay (`extracted/levels/NN/overlay.bin`), the class table (`lvl.vtbl`), the
//! port's class registry ([`LevelPorts`]) and the placed moby instances (placed = in the gameplay instance list,
//! created = the loader's spawn test on a first visit, as `all_levels_smoke` counts them). It builds the level's
//! static call graph and, from the update function of every class the port does not run, the class's **unit**: the
//! update plus every function only it reaches (a callee joins the unit when all its callers are in the unit). The
//! callees left outside are the unit's **shared functions** (boot ELF, engine / level code other code also calls).
//!
//! Edges: `jal` (call), `j` out of the function (tail), a code address formed with `lui`/`%lo` (ptr: a callback or
//! a state function handed on), a `.data` table of function starts formed the same way (table), and the fall-through
//! into the next start when a function does not end in `jr ra` / `j` (fall). `jalr` sites in the unit are counted as
//! unresolved indirect calls.
//!
//! **Direct reads of engine state** (data): a unit's own code that forms a data address with `lui`/`%lo` (the boot
//! ELF's globals below the overlay base, or overlay data) depends on that global. Only the globals the tag table
//! names (`G:<address>` rows: the state a system owns, e.g. the ship moby `0x140940` of the ship-combat mode) count
//! as a dependency; every boot global an unported unit touches is listed in `globals.tsv` for the next tagging pass.
//!
//! Identity across levels uses the port's code identity ([`rc_formats::level_overlay::mask`], the `Relocation`
//! masking): a shared function is keyed by its level-01 copy (`L01:addr`), its boot address (`boot:addr`), or, when
//! level 01 has no copy, by the hash of its masked code (`H:hash`, with a representative level address). Units are
//! grouped across levels by the hash of their update's masked code (one class-port unit, as `LevelPorts` groups).
//!
//! Outputs (tab-separated, `work/census/` by default): `classes.tsv`, `units.tsv`, `shared.tsv`, `globals.tsv`, and,
//! when a system tag table is given (`tools/ghidra/names/census_systems.tsv`), `systems.tsv` and `unit_systems.tsv`.

use anyhow::{Context, Result};
use rc_formats::level_overlay::{address_refs, mask, LevelOverlay, Relocation, TEXT_SECTION};
use rc_game::moby_update::classes::LevelPorts;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Boot ELF code is below the overlay base.
const OVERLAY_BASE: u32 = 0x15ef00;
/// The boot ELF's load address: a `lui`/`%lo` constant below it is not an address.
const BOOT_BASE: u32 = 0x10_0000;
const JR_RA: u32 = 0x03e0_0008;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Edge {
    Call,
    Tail,
    Ptr,
    Table,
    Fall,
    /// A data address the unit's own code forms (a global it reads or writes), not a call.
    Data,
}

impl Edge {
    fn tag(self) -> &'static str {
        match self {
            Edge::Call => "call",
            Edge::Tail => "tail",
            Edge::Ptr => "ptr",
            Edge::Table => "table",
            Edge::Fall => "fall",
            Edge::Data => "data",
        }
    }
}

/// A level's static call graph.
pub struct Graph {
    pub starts: BTreeSet<u32>,
    /// Function → (callee, edge kind).
    pub callees: HashMap<u32, BTreeSet<(u32, Edge)>>,
    pub callers: HashMap<u32, BTreeSet<u32>>,
    /// Function → `jalr` sites (not `jr ra`).
    pub jalr: HashMap<u32, usize>,
    pub extent: HashMap<u32, usize>,
    /// Function → the data keys it forms: `G:<address>` for a global it reads or writes (outside `.text`, at or
    /// above the boot ELF's 0x100000), `G:<address>=<value>` for a loaded global compared with a constant.
    pub data: HashMap<u32, BTreeSet<String>>,
    /// The starts with evidence of their own (a `jal` target, a class-table update, or right after a `jr ra` +
    /// delay slot), as opposed to a code address only formed with `lui`/`%lo`.
    pub strong: BTreeSet<u32>,
}

fn words_of(s: &rc_formats::font::OverlaySection) -> Vec<u32> { s.data.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect() }

/// The function starts exactly as `LevelOverlay` finds them (its set is private; same rules).
fn starts_of(ov: &LevelOverlay) -> (u32, u32, BTreeSet<u32>) {
    let Some(t) = ov.sections.get(TEXT_SECTION) else { return (0, 0, BTreeSet::new()) };
    let text = words_of(t);
    let (ts, te) = (t.dest, t.dest + 4 * text.len() as u32);
    let mut s: BTreeSet<u32> = text.iter().filter(|&&w| w >> 26 == 3).map(|&w| (w & 0x03ff_ffff) << 2).filter(|&a| a >= ts && a < te).collect();
    s.extend(ov.vtbl().iter().map(|e| e.update).filter(|&a| a >= ts && a < te));
    for p in 2..text.len() {
        let w = text[p];
        if text[p - 2] == JR_RA && w >> 16 == 0x27bd && w & 0x8000 != 0 { s.insert(ts + 4 * p as u32); }
    }
    s.extend(address_refs(&text).into_iter().map(|(_, a)| a).filter(|&a| a >= ts && a < te && a % 4 == 0));
    (ts, te, s)
}

impl Graph {
    pub fn build(ov: &LevelOverlay) -> Graph {
        let (ts, te, starts) = starts_of(ov);
        // A code address formed with `lui`/`%lo` is taken as a function only with other evidence of a start (a
        // `jal` target, a class-table update, or right after a `jr ra` + delay slot): constants such as the colour
        // 0x202020 also look like `.text` addresses.
        let text = ov.sections.get(TEXT_SECTION).map(words_of).unwrap_or_default();
        let at = |a: u32| text.get(((a - ts) / 4) as usize).copied().unwrap_or(0);
        let mut strong: BTreeSet<u32> = text.iter().filter(|&&w| w >> 26 == 3).map(|&w| (w & 0x03ff_ffff) << 2).collect();
        strong.extend(ov.vtbl().iter().map(|e| e.update));
        let is_fn = |a: u32| strong.contains(&a) || (a >= ts + 8 && at(a - 8) == JR_RA);
        let strong_starts: BTreeSet<u32> = starts.iter().copied().filter(|&a| is_fn(a)).collect();
        let mut g = Graph { starts: starts.clone(), callees: HashMap::new(), callers: HashMap::new(), jalr: HashMap::new(), extent: HashMap::new(), data: HashMap::new(), strong: strong_starts };
        for &f in &starts {
            let Some(n) = ov.extent(f) else { continue };
            let Some(code) = ov.code(f, n) else { continue };
            g.extent.insert(f, n);
            let end = f + 4 * n as u32;
            let mut out: BTreeSet<(u32, Edge)> = BTreeSet::new();
            let mut jalr = 0;
            for &w in code {
                match w >> 26 {
                    3 => { out.insert(((w & 0x03ff_ffff) << 2, Edge::Call)); }
                    2 => {
                        let a = (w & 0x03ff_ffff) << 2;
                        if a < f || a >= end { out.insert((a, Edge::Tail)); }
                    }
                    0 if w & 63 == 9 => jalr += 1,
                    _ => {}
                }
            }
            let mut data: BTreeSet<String> = data_refs(code, ts, te);
            for (_, a) in address_refs(code) {
                if a >= ts && a < te {
                    if (a < f || a >= end) && starts.contains(&a) && is_fn(a) { out.insert((a, Edge::Ptr)); }
                    continue;
                }
                if a >= BOOT_BASE { data.insert(format!("G:{a:08x}")); }
                if a >= OVERLAY_BASE && a % 4 == 0 {
                    // A table of function starts (entries inside this function are a switch's jump table).
                    let mut p = a;
                    for _ in 0..256 {
                        let Some(v) = ov.u32(p) else { break };
                        if !(v >= ts && v < te && starts.contains(&v) && is_fn(v)) { break; }
                        if v < f || v >= end { out.insert((v, Edge::Table)); }
                        p += 4;
                    }
                }
            }
            // Fall-through: the extent (trailing padding `nop`s dropped) does not end in `jr ra`, `j` or `b` + delay
            // slot.
            let m = code.iter().rposition(|&w| w != 0).map_or(0, |i| i + 1);
            let terminal = |w: u32| w == JR_RA || w >> 26 == 2 || w >> 16 == 0x1000;
            // Only into a start that has no evidence of its own (a code address formed with `lui`/`%lo` inside a
            // function): a real function start is never fallen into (a loop's backward branch can end a function).
            if m >= 1 && !terminal(code[m - 1]) && !(m >= 2 && terminal(code[m - 2])) && end < te && starts.contains(&end) && !is_fn(end) {
                out.insert((end, Edge::Fall));
            }
            if jalr > 0 { g.jalr.insert(f, jalr); }
            if !data.is_empty() { g.data.insert(f, data); }
            for &(c, _) in &out { g.callers.entry(c).or_default().insert(f); }
            g.callees.insert(f, out);
        }
        g
    }

    /// The unit of `root`: the functions only it reaches, and the callees left outside (with their edge kinds).
    pub fn unit(&self, root: u32) -> (BTreeSet<u32>, BTreeMap<u32, BTreeSet<Edge>>) {
        let mut u: BTreeSet<u32> = BTreeSet::from([root]);
        loop {
            let mut added = false;
            let cands: Vec<u32> = u.iter().flat_map(|f| self.callees.get(f).into_iter().flatten().map(|&(c, _)| c)).filter(|c| !u.contains(c) && *c >= OVERLAY_BASE).collect();
            for c in cands {
                if u.contains(&c) { continue; }
                let callers = self.callers.get(&c);
                if callers.is_some_and(|cs| cs.iter().all(|x| u.contains(x) || *x == c)) {
                    u.insert(c);
                    added = true;
                }
            }
            if !added { break; }
        }
        let mut leaves: BTreeMap<u32, BTreeSet<Edge>> = BTreeMap::new();
        for f in &u {
            for &(c, e) in self.callees.get(f).into_iter().flatten() {
                if !u.contains(&c) { leaves.entry(c).or_default().insert(e); }
            }
        }
        (u, leaves)
    }
}

/// The data keys `code` forms (`Graph::data`): a `lui` (+ `addiu` / `ori`) constant and every load / store whose
/// base register holds such a constant (`base + offset`: a field of a global block, e.g. the hero state `0x1413d4`
/// read as `0x13f350 + 0x2084`), and a loaded global compared with a constant (`xori` with an immediate, or a
/// `beq` / `bne` against a register holding one: the hero state against 0x32, `G:001413d4=00000032`). A linear pass
/// with one register file; a call clears the caller-saved registers, a branch is not followed (a register set on one
/// arm and read after the join is a rare extra reference, never a missed one on straight code). Addresses inside
/// `.text` (`ts..te`) and below the boot ELF are dropped.
fn data_refs(code: &[u32], ts: u32, te: u32) -> BTreeSet<String> {
    #[derive(Clone, Copy)]
    enum R {
        Const(u32),
        Load(u32),
    }
    let mut regs: [Option<R>; 32] = [None; 32];
    let mut out = BTreeSet::new();
    let ok = |a: u32| a >= BOOT_BASE && !(a >= ts && a < te);
    let mut call_pending = false;
    for &w in code {
        let (op, rs, rt, rd) = (w >> 26, (w >> 21 & 31) as usize, (w >> 16 & 31) as usize, (w >> 11 & 31) as usize);
        let imm = (w & 0xffff) as u16 as i16 as i32;
        let was_call = call_pending;
        call_pending = false;
        regs[0] = Some(R::Const(0));
        let cst = |r: Option<R>| if let Some(R::Const(c)) = r { Some(c) } else { None };
        let ld = |r: Option<R>| if let Some(R::Load(a)) = r { Some(a) } else { None };
        match op {
            0x0f => regs[rt] = Some(R::Const((w & 0xffff) << 16)),
            0x09 | 0x19 => regs[rt] = cst(regs[rs]).map(|b| R::Const(b.wrapping_add(imm as u32))),
            0x0d => regs[rt] = cst(regs[rs]).map(|b| R::Const(b | (w & 0xffff))),
            0x0e => {
                if let Some(a) = ld(regs[rs]) { out.insert(format!("G:{a:08x}={:08x}", w & 0xffff)); }
                regs[rt] = None;
            }
            0x04 | 0x05 => {
                if let (Some(a), Some(c)) = (ld(regs[rs]), cst(regs[rt])) { out.insert(format!("G:{a:08x}={c:08x}")); }
                if let (Some(a), Some(c)) = (ld(regs[rt]), cst(regs[rs])) { out.insert(format!("G:{a:08x}={c:08x}")); }
            }
            // Loads and stores: lb..lwu, lq / sq, lwc1 / swc1, ldc2 / sdc2 (lqc2 / sqc2), ld / sd.
            0x20..=0x2e | 0x1e | 0x1f | 0x31 | 0x39 | 0x36 | 0x3e | 0x37 | 0x3f => {
                let a = cst(regs[rs]).map(|b| b.wrapping_add(imm as u32)).filter(|&a| ok(a));
                if let Some(a) = a { out.insert(format!("G:{a:08x}")); }
                if matches!(op, 0x20..=0x27 | 0x1e | 0x37) { regs[rt] = a.map(R::Load); }
            }
            0x03 => { regs[31] = None; call_pending = true; }
            0x00 => {
                if w & 63 == 9 { regs[rd] = None; call_pending = true; } else if w & 63 != 8 { regs[rd] = None; }
            }
            0x1c => regs[rd] = None,
            0x11 | 0x12 => { if rs < 4 { regs[rt] = None; } }
            0x08 | 0x0a..=0x0c | 0x18 => regs[rt] = None,
            _ => {}
        }
        if was_call {
            // The delay slot has run: the callee clobbers everything but s0..s7, gp, sp, fp.
            for r in (1..16).chain(24..28).chain([31]) { regs[r] = None; }
        }
    }
    out
}

fn hash_words(w: &[u32]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    w.hash(&mut h);
    h.finish()
}

/// Masked-code hash of the function at `f` over its extent.
fn code_hash(ov: &LevelOverlay, f: u32) -> Option<u64> { Some(hash_words(&mask(ov.code(f, ov.extent(f)?)?))) }

/// `tools/ghidra/names/clusters.tsv` (overlay_diff.py's relocation-tolerant hash): (level, address) → (cluster hash,
/// its level-01 member if exactly one, the number of programs).
struct Clusters(HashMap<(u32, u32), (String, Option<u32>, u32)>);

impl Clusters {
    fn load(path: &Path) -> Clusters {
        let mut m = HashMap::new();
        let Ok(s) = std::fs::read_to_string(path) else { return Clusters(m) };
        for l in s.lines().skip(1) {
            let c: Vec<&str> = l.split('\t').collect();
            if c.len() < 5 { continue; }
            let members: Vec<(Option<u32>, u32)> = c[4]
                .split(' ')
                .filter_map(|x| {
                    let (p, a) = x.split_once(':')?;
                    let a = u32::from_str_radix(a, 16).ok()?;
                    let lv = p.strip_prefix("level").and_then(|r| r.strip_suffix(".elf")).and_then(|n| n.parse().ok());
                    Some((lv, a))
                })
                .collect();
            let l01: Vec<u32> = members.iter().filter(|m| m.0 == Some(1)).map(|m| m.1).collect();
            let one = if l01.len() == 1 { Some(l01[0]) } else { None };
            let n: u32 = c[1].parse().unwrap_or(0);
            for (lv, a) in members {
                if let Some(lv) = lv { m.insert((lv, a), (c[0][..12.min(c[0].len())].to_string(), one, n)); }
            }
        }
        Clusters(m)
    }
}

/// Class updates the port runs outside the class registry (level-01 addresses): the particle emitter the scheduler runs
/// through the engine's external hook, and the water strips whose update only queues the strip draw (or fills its
/// table, or moves its z) that `rc-engine`'s water renderer does.
const OUTSIDE: [(u32, &str); 5] = [
    (0x2bd100, "Emitter"),
    (0x2f6128, "WaterStrip"),
    (0x2f6180, "WaterStrip"),
    (0x2feb58, "WaterStrip"),
    (0x309c98, "WaterStrip"),
];

/// A level function's identity across levels: boot address, level-01 copy (the port's code identity, then the
/// cluster table), the cluster (`C:`), else the masked-code hash (`H:`).
/// The level-01 copy must be a function start there, and for a short function (under 8 words, whose masked words
/// can also match inside another function) have the same extent.
fn key_of(level: u32, f: u32, ov: &LevelOverlay, to_l01: &Relocation, cl: &Clusters, l01: &Graph) -> (String, bool) {
    if f < OVERLAY_BASE { return (format!("boot:{f:08x}"), true); }
    if level == 1 { return (format!("L01:{f:08x}"), true); }
    let n = ov.extent(f).unwrap_or(0);
    if let Some(a) = to_l01.func(f).filter(|a| l01.starts.contains(a) && (n >= 8 || l01.extent.get(a) == Some(&n))) {
        return (format!("L01:{a:08x}"), true);
    }
    if let Some((h, one, _)) = cl.0.get(&(level, f)) {
        if let Some(a) = one { return (format!("L01:{a:08x}"), true); }
        return (format!("C:{h}"), false);
    }
    (format!("H:{:016x}", code_hash(ov, f).unwrap_or(0)), false)
}

/// Function names from the decompiler export (`work/decomp/<program>/index.tsv`).
pub fn names(work: &Path, program: &str) -> HashMap<u32, String> {
    let Ok(s) = std::fs::read_to_string(work.join("decomp").join(program).join("index.tsv")) else { return HashMap::new() };
    s.lines().filter_map(|l| { let mut it = l.split('\t'); Some((u32::from_str_radix(it.next()?, 16).ok()?, it.next()?.to_string())) }).collect()
}

/// Hex tokens cited in the repo: token value → (files under crates/, files under docs/). Lower-case `0x…` (5–8 digits)
/// and `fun_XXXXXXXX`.
fn repo_refs(root: &Path) -> HashMap<u32, (BTreeSet<String>, BTreeSet<String>)> {
    let mut out: HashMap<u32, (BTreeSet<String>, BTreeSet<String>)> = HashMap::new();
    fn walk(d: &Path, ext: &str, files: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, ext, files); } else if p.extension().is_some_and(|x| x == ext) { files.push(p); }
        }
    }
    for (dir, ext, is_doc) in [("crates", "rs", false), ("docs", "md", true)] {
        let mut files = Vec::new();
        walk(&root.join(dir), ext, &mut files);
        for p in files {
            let Ok(s) = std::fs::read_to_string(&p) else { continue };
            let s = s.to_ascii_lowercase();
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().into_owned();
            let b = s.as_bytes();
            let mut i = 0;
            while i + 2 < b.len() {
                let (skip, start) = if b[i] == b'0' && b[i + 1] == b'x' { (2, true) } else if b[i..].starts_with(b"fun_") { (4, true) } else { (0, false) };
                if start && (i == 0 || !b[i - 1].is_ascii_alphanumeric()) {
                    let j0 = i + skip;
                    let mut j = j0;
                    while j < b.len() && b[j].is_ascii_hexdigit() { j += 1; }
                    let n = j - j0;
                    if (5..=8).contains(&n) && (j == b.len() || !b[j].is_ascii_alphanumeric()) {
                        if let Ok(v) = u32::from_str_radix(&s[j0..j], 16) {
                            let e = out.entry(v).or_default();
                            if is_doc { e.1.insert(rel.clone()); } else { e.0.insert(rel.clone()); }
                        }
                    }
                    i = j.max(i + 1);
                    continue;
                }
                i += 1;
            }
        }
    }
    out
}

/// One class on one level.
struct ClassRow {
    level: u32,
    o_class: i32,
    placed: usize,
    created: usize,
    update: u32,
    port: String,
    unit: usize,
}

#[derive(Default)]
struct Unit {
    /// (level, update address in that level).
    copies: BTreeSet<(u32, u32)>,
    l01: Option<u32>,
    classes: BTreeSet<(u32, i32)>,
    placed: usize,
    created: usize,
    private_fns: usize,
    private_words: usize,
    jalr: usize,
    /// Shared key → edge kinds.
    leaves: BTreeMap<String, BTreeSet<Edge>>,
    ported: bool,
}

#[derive(Default)]
struct Shared {
    /// (level, address) of every copy seen.
    sites: BTreeSet<(u32, u32)>,
    units: BTreeSet<usize>,
    in_ported_unit: bool,
    /// Direct callees (keys) of the representative copy.
    callees: BTreeSet<String>,
    words: usize,
}

/// A tag row of `census_systems.tsv`.
#[derive(Clone, Debug, Default)]
struct Tag {
    name: String,
    system: String,
    status: String,
    conf: String,
}

/// The tag table: key → tag. Keys: `L01:` / `boot:` / `C:` / `H:` functions, `G:<address>` globals.
fn tags(path: &Path) -> HashMap<String, Tag> {
    let Ok(s) = std::fs::read_to_string(path) else { return HashMap::new() };
    s.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            if c.len() < 4 || c[0] == "key" { return None; }
            Some((c[0].to_string(), Tag { name: c[1].to_string(), system: c[2].to_string(), status: c[3].to_string(), conf: c.get(4).unwrap_or(&"").to_string() }))
        })
        .collect()
}

/// A system's tally: (units, classes as (level, class), created instances, blocking function → units, status).
type SystemTally = (BTreeSet<usize>, BTreeSet<(u32, i32)>, usize, BTreeMap<String, usize>, String);

pub struct Summary {
    pub classes: usize,
    pub placed: usize,
    pub created: usize,
    pub units: usize,
    pub shared: usize,
    pub out: PathBuf,
}

/// Runs the census over the levels in `levels`.
pub fn run(extracted: &Path, repo: &Path, work: &Path, out: &Path, tag_file: Option<&Path>, levels: &[u32]) -> Result<Summary> {
    let load = |l: u32| -> Option<Arc<LevelOverlay>> {
        let b = std::fs::read(extracted.join(format!("levels/{l:02}/overlay.bin"))).ok()?;
        LevelOverlay::parse(&b).ok().map(Arc::new)
    };
    let mut cache: HashMap<u32, Option<Arc<LevelOverlay>>> = HashMap::new();
    for l in 0..19 { cache.insert(l, load(l)); }
    let reference = |l: u32| cache.get(&l).cloned().flatten();
    let l01 = reference(1).context("level 01 overlay")?;
    let l01_names = names(work, "level01.elf");
    let boot_names = names(work, "SCUS_971.99");
    let clusters = Clusters::load(&repo.join("tools/ghidra/names/clusters.tsv"));
    let l01_graph = Graph::build(&l01);
    let tagmap = tag_file.map(tags).unwrap_or_default();

    let mut rows: Vec<ClassRow> = Vec::new();
    let mut units: Vec<Unit> = Vec::new();
    let mut unit_by_hash: HashMap<(u64, bool), usize> = HashMap::new();
    let mut shared: BTreeMap<String, Shared> = BTreeMap::new();
    // Boot global key → (units whose own code forms it, one is ported).
    let mut globals: BTreeMap<String, (BTreeSet<usize>, bool)> = BTreeMap::new();
    let mut level_names: HashMap<u32, HashMap<u32, String>> = HashMap::new();

    for &level in levels {
        let Some(ov) = reference(level) else { eprintln!("level {level:02}: no overlay"); continue };
        let ports = LevelPorts::from_overlays(&ov, &reference, &OUTSIDE.map(|(a, _)| a));
        let gp = rc_formats::test_data::gameplay(level).with_context(|| format!("level {level:02} gameplay"))?;
        let instances = rc_formats::gameplay::parse_moby_instances(&gp).with_context(|| format!("level {level:02} instances"))?;
        let tests = rc_formats::moby_spawn::loader_spawns(&instances, &mut rc_formats::moby_spawn::SpawnSave::default());
        let mut placed: BTreeMap<i32, (usize, usize)> = BTreeMap::new();
        for (i, t) in instances.iter().zip(&tests) {
            if i.o_class == 0 { continue; }
            let e = placed.entry(i.o_class).or_default();
            e.0 += 1;
            if t.spawn { e.1 += 1; }
        }
        let graph = Graph::build(&ov);
        let to_l01 = Relocation::new(&ov, &l01);
        let lnames = names(work, &format!("level{level:02}.elf"));
        let mut update_classes: BTreeMap<u32, Vec<i32>> = BTreeMap::new();
        for e in ov.vtbl() {
            if e.update == 0 || e.o_class == 0 { continue; }
            update_classes.entry(e.update).or_default().push(e.o_class);
        }
        for (&update, cls) in &update_classes {
            let ported: Vec<String> = cls
                .iter()
                .filter_map(|&c| {
                    let c = c as i16;
                    ports.get(c).map(|u| format!("{u:?}")).or_else(|| ports.external(c).and_then(|a| OUTSIDE.iter().find(|o| o.0 == a)).map(|o| o.1.to_string()))
                })
                .collect();
            let is_ported = !ported.is_empty();
            let (p, _c): (usize, usize) = cls.iter().map(|c| placed.get(c).copied().unwrap_or_default()).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
            if p == 0 && !is_ported { continue; }
            let (private, leaves) = graph.unit(update);
            let h = code_hash(&ov, update).unwrap_or(0);
            let uid = *unit_by_hash.entry((h, is_ported)).or_insert_with(|| {
                units.push(Unit { ported: is_ported, ..Default::default() });
                units.len() - 1
            });
            let u = &mut units[uid];
            if u.copies.is_empty() {
                u.private_fns = private.len();
                u.private_words = private.iter().map(|f| graph.extent.get(f).copied().unwrap_or(0)).sum();
                u.jalr = private.iter().map(|f| graph.jalr.get(f).copied().unwrap_or(0)).sum();
            }
            u.copies.insert((level, update));
            if u.l01.is_none() { u.l01 = if level == 1 { Some(update) } else { to_l01.func(update) }; }
            for &c in cls {
                let (pl, cr) = placed.get(&c).copied().unwrap_or_default();
                u.classes.insert((level, c));
                u.placed += pl;
                u.created += cr;
                rows.push(ClassRow { level, o_class: c, placed: pl, created: cr, update, port: ported.first().cloned().unwrap_or_default(), unit: uid });
            }
            for (&leaf, kinds) in &leaves {
                let (key, _) = key_of(level, leaf, &ov, &to_l01, &clusters, &l01_graph);
                u.leaves.entry(key.clone()).or_default().extend(kinds.iter().copied());
                let s = shared.entry(key.clone()).or_default();
                s.sites.insert((level, leaf));
                s.units.insert(uid);
                if is_ported { s.in_ported_unit = true; }
                if s.callees.is_empty() && leaf >= OVERLAY_BASE {
                    s.words = graph.extent.get(&leaf).copied().unwrap_or(0);
                    for &(c, _) in graph.callees.get(&leaf).into_iter().flatten() {
                        s.callees.insert(key_of(level, c, &ov, &to_l01, &clusters, &l01_graph).0);
                    }
                }
            }
            // The globals the unit's own code forms: a tagged one (`G:`) is a dependency like a call. A class
            // override (`X:<class>`: state the census cannot see, with its reason in the table) counts the same way.
            let mut data: BTreeSet<String> = private.iter().flat_map(|f| graph.data.get(f).into_iter().flatten().cloned()).collect();
            data.extend(cls.iter().map(|c| format!("X:{c}")).filter(|k| tagmap.contains_key(k)));
            for key in &data {
                let a = key.get(2..10).and_then(|h| u32::from_str_radix(h, 16).ok()).unwrap_or(0);
                if key.starts_with("G:") && a < OVERLAY_BASE {
                    let e = globals.entry(key.clone()).or_default();
                    e.0.insert(uid);
                    e.1 |= is_ported;
                }
                if !tagmap.contains_key(key) { continue; }
                u.leaves.entry(key.clone()).or_default().insert(Edge::Data);
                let s = shared.entry(key.clone()).or_default();
                s.sites.insert((level, a));
                s.units.insert(uid);
                if is_ported { s.in_ported_unit = true; }
            }
            if is_ported {
                // The functions a ported unit owns count as ported too (by their level-01 key).
                for &f in &private {
                    let (key, _) = key_of(level, f, &ov, &to_l01, &clusters, &l01_graph);
                    shared.entry(key).or_default().in_ported_unit = true;
                }
            }
        }
        level_names.insert(level, lnames);
    }

    let refs = repo_refs(repo);
    let name_of = |key: &str, s: Option<&Shared>| -> String {
        if key.starts_with("G:") || key.starts_with("X:") { return tagmap.get(key).map(|t| t.name.clone()).unwrap_or_default(); }
        if let Some(a) = key.strip_prefix("L01:").and_then(|h| u32::from_str_radix(h, 16).ok()) {
            return l01_names.get(&a).cloned().unwrap_or_default();
        }
        if let Some(a) = key.strip_prefix("boot:").and_then(|h| u32::from_str_radix(h, 16).ok()) {
            return boot_names.get(&a).cloned().unwrap_or_default();
        }
        s.and_then(|s| s.sites.iter().find_map(|(l, a)| level_names.get(l).and_then(|m| m.get(a)).cloned())).unwrap_or_default()
    };
    let addr_of = |key: &str| -> Option<u32> { key.strip_prefix("L01:").or_else(|| key.strip_prefix("boot:")).or_else(|| key.strip_prefix("G:").map(|h| &h[..8.min(h.len())])).and_then(|h| u32::from_str_radix(h, 16).ok()) };

    std::fs::create_dir_all(out)?;
    // classes.tsv
    let mut t = String::from("level\to_class\tplaced\tcreated\tupdate\tunit\tport\n");
    for r in &rows { t += &format!("{:02}\t{}\t{}\t{}\t{:08x}\tU{}\t{}\n", r.level, r.o_class, r.placed, r.created, r.update, r.unit, r.port); }
    crate::write_output(&out.join("classes.tsv"), t)?;
    // units.tsv (unported only, by created instances)
    let mut order: Vec<usize> = (0..units.len()).filter(|&i| !units[i].ported).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(units[i].created), std::cmp::Reverse(units[i].placed), i));
    let mut t = String::from("unit\tcreated\tplaced\tn_classes\tlevels\tclasses\tupdates\tl01_copy\tprivate_fns\tprivate_words\tjalr\tn_shared\tshared\n");
    for &i in &order {
        let u = &units[i];
        let levels: BTreeSet<u32> = u.copies.iter().map(|c| c.0).collect();
        let classes: BTreeSet<i32> = u.classes.iter().map(|c| c.1).collect();
        t += &format!(
            "U{i}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            u.created,
            u.placed,
            classes.len(),
            levels.iter().map(|l| format!("{l:02}")).collect::<Vec<_>>().join(","),
            classes.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","),
            u.copies.iter().map(|(l, a)| format!("L{l:02}:{a:06x}")).collect::<Vec<_>>().join(","),
            u.l01.map(|a| format!("{a:06x}")).unwrap_or_default(),
            u.private_fns,
            u.private_words,
            u.jalr,
            u.leaves.len(),
            u.leaves.iter().map(|(k, e)| format!("{k}{}", if e.contains(&Edge::Call) || e.contains(&Edge::Tail) { String::new() } else { format!("[{}]", e.iter().map(|x| x.tag()).collect::<Vec<_>>().join("+")) })).collect::<Vec<_>>().join(";")
        );
    }
    crate::write_output(&out.join("units.tsv"), t)?;
    // shared.tsv (functions unported units call)
    let mut srow: Vec<(&String, &Shared, usize, usize)> = shared
        .iter()
        .filter(|(_, s)| s.units.iter().any(|&u| !units[u].ported))
        .map(|(k, s)| {
            let un: Vec<usize> = s.units.iter().copied().filter(|&u| !units[u].ported).collect();
            (k, s, un.len(), un.iter().map(|&u| units[u].created).sum::<usize>())
        })
        .collect();
    srow.sort_by_key(|r| (std::cmp::Reverse(r.3), std::cmp::Reverse(r.2), r.0.clone()));
    let mut t = String::from("key\tname\tunported_units\tcreated\tported_unit_uses\tcrates_refs\tdocs_refs\twords\trep_site\tn_levels\tsystem\tstatus\tconf\tcrates_files\tdocs_files\tcallees\n");
    for (k, s, n, c) in &srow {
        let (cr, dr) = addr_of(k).and_then(|a| refs.get(&a)).map(|(c, d)| (c.len(), d.len())).unwrap_or((0, 0));
        let files = |doc: bool| -> String {
            addr_of(k).and_then(|a| refs.get(&a)).map(|(c, d)| (if doc { d } else { c }).iter().take(4).map(|f| f.trim_start_matches("crates/").trim_start_matches("docs/plan/").to_string()).collect::<Vec<_>>().join(",")).unwrap_or_default()
        };
        let nlev = s.sites.iter().map(|x| x.0).collect::<BTreeSet<_>>().len();
        let tag = tagmap.get(*k);
        let rep = s.sites.iter().next().map(|(l, a)| format!("L{l:02}:{a:06x}")).unwrap_or_default();
        t += &format!(
            "{k}\t{}\t{n}\t{c}\t{}\t{cr}\t{dr}\t{}\t{rep}\t{nlev}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            name_of(k, Some(s)),
            s.in_ported_unit as u8,
            s.words,
            tag.map(|t| t.system.as_str()).unwrap_or(""),
            tag.map(|t| t.status.as_str()).unwrap_or(""),
            tag.map(|t| t.conf.as_str()).unwrap_or(""),
            files(false),
            files(true),
            s.callees.iter().cloned().collect::<Vec<_>>().join(";")
        );
    }
    crate::write_output(&out.join("shared.tsv"), t)?;
    // globals.tsv (boot globals the unported units' own code forms, by the instances of those units)
    let mut grow: Vec<(&String, Vec<usize>, bool)> = globals
        .iter()
        .map(|(k, (us, p))| (k, us.iter().copied().filter(|&u| !units[u].ported).collect::<Vec<_>>(), *p))
        .filter(|r| !r.1.is_empty())
        .collect();
    grow.sort_by_key(|r| (std::cmp::Reverse(r.1.iter().map(|&u| units[u].created).sum::<usize>()), r.0.clone()));
    let mut t = String::from("global\tunported_units\tcreated\tin_ported_unit\tcrates_refs\tsystem\tstatus\tunits\n");
    for (k, us, p) in &grow {
        let tag = tagmap.get(*k);
        t += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            &k[2..],
            us.len(),
            us.iter().map(|&u| units[u].created).sum::<usize>(),
            *p as u8,
            addr_of(k).and_then(|a| refs.get(&a)).map(|(c, _)| c.len()).unwrap_or(0),
            tag.map(|t| t.system.as_str()).unwrap_or(""),
            tag.map(|t| t.status.as_str()).unwrap_or(""),
            us.iter().map(|u| format!("U{u}")).collect::<Vec<_>>().join(",")
        );
    }
    crate::write_output(&out.join("globals.tsv"), t)?;

    // With a tag table: per-unit systems and the system ranking.
    if !tagmap.is_empty() {
        let mut t = String::from("unit\tcreated\tclasses\tlevels\tverdict\tmissing\tpartly\thas\tfamily\tjalr\tblocking_functions\tuntagged\n");
        let mut sys: BTreeMap<String, SystemTally> = BTreeMap::new();
        for &i in &order {
            let u = &units[i];
            let mut by: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
            let mut untagged = Vec::new();
            for k in u.leaves.keys() {
                match tagmap.get(k) {
                    Some(Tag { system, status, .. }) => {
                        by.entry(status.as_str()).or_default().insert(system.clone());
                        if status == "missing" || status == "partly" {
                            let e = sys.entry(system.clone()).or_default();
                            e.0.insert(i);
                            e.1.extend(u.classes.iter().copied());
                            *e.3.entry(k.clone()).or_default() += 1;
                        }
                    }
                    None => untagged.push(k.clone()),
                }
            }
            let need = |s: &str| by.get(s).map(|v| v.iter().cloned().collect::<Vec<_>>().join(",")).unwrap_or_default();
            let levels: BTreeSet<u32> = u.copies.iter().map(|c| c.0).collect();
            let classes: BTreeSet<i32> = u.classes.iter().map(|c| c.1).collect();
            let verdict = if by.contains_key("missing") { "missing" } else if by.contains_key("partly") { "partly" } else if !untagged.is_empty() { "unknown" } else { "cheap" };
            let blocking: Vec<&String> = u.leaves.keys().filter(|k| tagmap.get(*k).is_some_and(|t| t.status == "missing" || t.status == "partly")).collect();
            t += &format!(
                "U{i}\t{}\t{}\t{}\t{verdict}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                u.created,
                classes.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","),
                levels.iter().map(|l| format!("{l:02}")).collect::<Vec<_>>().join(","),
                need("missing"),
                need("partly"),
                need("has"),
                need("family"),
                u.jalr,
                blocking.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(";"),
                untagged.join(";")
            );
        }
        crate::write_output(&out.join("unit_systems.tsv"), t)?;
        for (k, e) in sys.iter_mut() {
            e.2 = e.0.iter().map(|&u| units[u].created).sum();
            // The system's status: partly when the port has any of its functions, else missing.
            let has = tagmap.values().any(|t| &t.system == k && t.status != "missing");
            e.4 = if has { "partly".into() } else { "missing".into() };
        }
        let mut ranked: Vec<_> = sys.iter().collect();
        ranked.sort_by_key(|(k, e)| (std::cmp::Reverse(e.2), (*k).clone()));
        let mut t = String::from("system\tstatus\tunits\tclass_levels\tcreated\texample_classes\tkey_functions\n");
        for (k, e) in ranked {
            let mut ex: Vec<(usize, usize)> = e.0.iter().map(|&u| (units[u].created, u)).collect();
            ex.sort_by(|a, b| b.cmp(a));
            let exs: Vec<String> = ex.iter().take(4).map(|&(c, u)| format!("{} ({c})", units[u].classes.iter().map(|x| x.1).collect::<BTreeSet<_>>().iter().take(3).map(|x| x.to_string()).collect::<Vec<_>>().join("/"))).collect();
            let mut fns: Vec<(&String, &usize)> = e.3.iter().collect();
            fns.sort_by(|a, b| b.1.cmp(a.1));
            t += &format!("{k}\t{}\t{}\t{}\t{}\t{}\t{}\n", e.4, e.0.len(), e.1.len(), e.2, exs.join("; "), fns.iter().take(6).map(|(f, n)| { let nm = name_of(f, shared.get(*f)); format!("{f}{}×{n}", if nm.is_empty() { String::new() } else { format!(" {nm}") }) }).collect::<Vec<_>>().join(" "));
        }
        crate::write_output(&out.join("systems.tsv"), t)?;
    }

    let un: Vec<&ClassRow> = rows.iter().filter(|r| r.port.is_empty()).collect();
    Ok(Summary {
        classes: un.len(),
        placed: un.iter().map(|r| r.placed).sum(),
        created: un.iter().map(|r| r.created).sum(),
        units: order.len(),
        shared: srow.len(),
        out: out.to_path_buf(),
    })
}


/// `SetDeathBits` (level 01), the save- and visit-bit store of a dying moby (docs/plan/game_state.md §6.1).
const SET_DEATH_BITS: u32 = 0x26c250;
/// The persistent death bits `0x14c190 + level·0x100` (save chunk 3005), all 19 levels.
const PERSISTENT: std::ops::Range<u32> = 0x14c190..0x14c190 + 19 * 0x100;
/// The level overlay's visit tables (death bits 0x1ba950 .. the checkpoint copy's end 0x1bc310, level-01 addresses)
/// and their offset in every overlay (the 0xc60 clears of `FUN_0029abc0`).
const VISIT: std::ops::Range<u32> = 0x1ba950..0x1bc310;
const VISIT_OFF: [i32; 19] = [-0x480, 0, 0x180, -0x380, -0x300, 0, 0x300, -0x400, 0x380, -0x300, 0, 0x580, 0, -0x180, 0x280, 0x180, 0, 0x480, 0x580];

/// Whether the address completed at word `i` is stored through: word `i` is a store, or it forms a register that a
/// store within the next 24 words uses as its base (directly or through an `addu` / `daddu` with an index). A short
/// straight-line scan; a register written by anything else drops out.
fn stores_through(code: &[u32], i: usize) -> bool {
    let store = |op: u32| matches!(op, 0x28 | 0x29 | 0x2b | 0x3f | 0x1f);
    let w = code[i];
    if store(w >> 26) { return true; }
    let mut regs: u32 = 1 << (w >> 16 & 31);
    for &w in code.iter().skip(i + 1).take(24) {
        let (op, rs, rt, rd) = (w >> 26, w >> 21 & 31, w >> 16 & 31, w >> 11 & 31);
        if store(op) && regs & 1 << rs != 0 { return true; }
        match op {
            0 if matches!(w & 63, 0x21 | 0x2d) => {
                if regs & (1 << rs | 1 << rt) != 0 { regs |= 1 << rd } else { regs &= !(1 << rd) }
            }
            0 => regs &= !(1 << rd),
            0x09 | 0x19 => if regs & 1 << rs != 0 { regs |= 1 << rt } else { regs &= !(1 << rt) },
            0x20..=0x27 | 0x37 | 0x0a..=0x0f => regs &= !(1 << rt),
            _ => {}
        }
        if regs == 0 { break; }
    }
    false
}

/// `death-census`: for every class of every level (placed or ported), how its update records a death: the call path
/// to `SetDeathBits`, and the functions it reaches that form an address in the persistent death bits (`P`) or the
/// visit tables (`V`, the inline kill / death-bit stores, and the readers of the same tables). Writes `deaths.tsv`.
pub fn deaths(extracted: &Path, repo: &Path, work: &Path, out: &Path) -> Result<usize> {
    let load = |l: u32| -> Option<Arc<LevelOverlay>> {
        let b = std::fs::read(extracted.join(format!("levels/{l:02}/overlay.bin"))).ok()?;
        LevelOverlay::parse(&b).ok().map(Arc::new)
    };
    let mut cache: HashMap<u32, Option<Arc<LevelOverlay>>> = HashMap::new();
    for l in 0..19 { cache.insert(l, load(l)); }
    let reference = |l: u32| cache.get(&l).cloned().flatten();
    let l01 = reference(1).context("level 01 overlay")?;
    let l01_names = names(work, "level01.elf");
    let clusters = Clusters::load(&repo.join("tools/ghidra/names/clusters.tsv"));
    let l01_graph = Graph::build(&l01);
    let mut t = String::from("level\to_class\tplaced\tcreated\tport\tupdate\tset_death_bits\tsites\tinline\n");
    let mut n = 0;
    for level in 0..19u32 {
        let Some(ov) = reference(level) else { continue };
        let ports = LevelPorts::from_overlays(&ov, &reference, &OUTSIDE.map(|(a, _)| a));
        let gp = rc_formats::test_data::gameplay(level).with_context(|| format!("level {level:02} gameplay"))?;
        let instances = rc_formats::gameplay::parse_moby_instances(&gp)?;
        let tests = rc_formats::moby_spawn::loader_spawns(&instances, &mut rc_formats::moby_spawn::SpawnSave::default());
        let mut placed: BTreeMap<i32, (usize, usize)> = BTreeMap::new();
        for (i, s) in instances.iter().zip(&tests) {
            let e = placed.entry(i.o_class).or_default();
            e.0 += 1;
            if s.spawn { e.1 += 1; }
        }
        let graph = Graph::build(&ov);
        let to_l01 = Relocation::new(&ov, &l01);
        let sdb = Relocation::new(&l01, &ov).func(SET_DEATH_BITS);
        let lnames = names(work, &format!("level{level:02}.elf"));
        let name = |f: u32| -> String {
            let (k, _) = key_of(level, f, &ov, &to_l01, &clusters, &l01_graph);
            let nm = k.strip_prefix("L01:").and_then(|h| u32::from_str_radix(h, 16).ok()).and_then(|a| l01_names.get(&a)).or_else(|| lnames.get(&f));
            match nm { Some(nm) if !nm.starts_with("FUN_") => format!("{f:x}({nm})"), _ => format!("{f:x}") }
        };
        let d = VISIT_OFF[level as usize];
        let visit = (VISIT.start as i32 + d) as u32..(VISIT.end as i32 + d) as u32;
        // Functions forming a death-table address.
        let mut inline: HashMap<u32, BTreeSet<&str>> = HashMap::new();
        for &f in &graph.starts {
            let Some(code) = graph.extent.get(&f).and_then(|&k| ov.code(f, k)) else { continue };
            for (i, a) in address_refs(code) {
                let w = if stores_through(code, i) { "w" } else { "r" };
                if PERSISTENT.contains(&a) { inline.entry(f).or_default().insert(if w == "w" { "Pw" } else { "Pr" }); }
                if visit.contains(&a) { inline.entry(f).or_default().insert(if w == "w" { "Vw" } else { "Vr" }); }
            }
        }
        let classes_of_update: BTreeSet<u32> = ov.vtbl().iter().map(|e| e.update).collect();
        for e in ov.vtbl() {
            if e.update == 0 || e.o_class == 0 { continue; }
            let c = e.o_class as i16;
            let port = ports.get(c).map(|u| format!("{u:?}")).unwrap_or_default();
            let (pl, cr) = placed.get(&e.o_class).copied().unwrap_or_default();
            if pl == 0 && port.is_empty() { continue; }
            // Breadth-first over the update's reach (another class's update is not entered).
            let mut parent: HashMap<u32, u32> = HashMap::from([(e.update, e.update)]);
            let mut q = std::collections::VecDeque::from([e.update]);
            while let Some(f) = q.pop_front() {
                for &(c2, _) in graph.callees.get(&f).into_iter().flatten() {
                    if c2 < OVERLAY_BASE || parent.contains_key(&c2) || (classes_of_update.contains(&c2) && c2 != e.update) { continue; }
                    parent.insert(c2, f);
                    if Some(c2) != sdb { q.push_back(c2); }
                }
            }
            let path = |mut f: u32| -> String {
                let mut p = vec![name(f)];
                while parent[&f] != f { f = parent[&f]; p.push(name(f)); }
                p.reverse();
                p.join(" > ")
            };
            let via = sdb.filter(|s| parent.contains_key(s)).map(&path).unwrap_or_default();
            // The `jal SetDeathBits` sites in the reach, by function.
            let jal = sdb.map(|s| 0x0c00_0000 | s >> 2);
            let mut sites: Vec<String> = parent
                .keys()
                .filter_map(|&f| {
                    let n = graph.extent.get(&f).and_then(|&k| ov.code(f, k))?.iter().filter(|&&w| Some(w) == jal).count();
                    (n > 0).then(|| format!("{}×{n}", name(f)))
                })
                .collect();
            sites.sort();
            let mut inl: Vec<String> = parent.keys().filter_map(|f| inline.get(f).map(|k| format!("{}[{}]", path(*f), k.iter().copied().collect::<Vec<_>>().join("")))).collect();
            inl.sort();
            t += &format!("{level:02}\t{}\t{pl}\t{cr}\t{port}\t{:x}\t{via}\t{}\t{}\n", e.o_class, e.update, sites.join(" "), inl.join(" ; "));
            n += 1;
        }
    }
    std::fs::create_dir_all(out)?;
    crate::write_output(&out.join("deaths.tsv"), t)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MIPS words for the data census: `lui`, `addiu`, `lw`, `xori`, `beq`, `jal`.
    fn lui(rt: u32, imm: u32) -> u32 { 0x0f << 26 | rt << 16 | imm }
    fn addiu(rt: u32, rs: u32, imm: i16) -> u32 { 0x09 << 26 | rs << 21 | rt << 16 | (imm as u16 as u32) }
    fn lw(rt: u32, off: i16, base: u32) -> u32 { 0x23 << 26 | base << 21 | rt << 16 | (off as u16 as u32) }
    fn xori(rt: u32, rs: u32, imm: u32) -> u32 { 0x0e << 26 | rs << 21 | rt << 16 | imm }
    fn beq(rs: u32, rt: u32) -> u32 { 0x04 << 26 | rs << 21 | rt << 16 | 4 }
    const V0: u32 = 2;
    const V1: u32 = 3;
    const A0: u32 = 4;
    const TS: u32 = 0x15ef00;
    const TE: u32 = 0x320000;

    #[test]
    fn data_refs_sees_lo_loads_base_offsets_and_compares() {
        // `lw v1, 0x13d4(v0)` with v0 = 0x140000: the hero state; `xori v1, v1, 0x32`: compared with 0x32.
        let code = [lui(V0, 0x14), lw(V1, 0x13d4, V0), xori(V1, V1, 0x32), JR_RA, 0];
        let refs = data_refs(&code, TS, TE);
        assert_eq!(refs, BTreeSet::from(["G:001413d4".to_string(), "G:001413d4=00000032".to_string()]));
        // The hero block base 0x13f350 (`lui` + `addiu`) and the state at +0x2084; `li a0, 0x32; beq v1, a0`.
        let code = [lui(V0, 0x14), addiu(V0, V0, -0xcb0), lw(V1, 0x2084, V0), addiu(A0, 0, 0x32), beq(V1, A0), 0, JR_RA, 0];
        // (The base constant itself, `G:0013f350`, is `address_refs`'s: `Graph::build` unions both.)
        let refs = data_refs(&code, TS, TE);
        assert!(refs.contains("G:001413d4"), "{refs:?}");
        assert!(refs.contains("G:001413d4=00000032"), "{refs:?}");
    }

    #[test]
    fn data_refs_drops_text_addresses_small_constants_and_clobbered_bases() {
        // A `.text` address and a colour constant are not globals.
        let code = [lui(V0, 0x20), addiu(V0, V0, 0x1000), lui(V1, 0x20), addiu(V1, V1, 0x2020), JR_RA, 0];
        assert!(data_refs(&code, TS, TE).is_empty());
        // After a call the base register is unknown: no reference from the load.
        let code = [lui(V0, 0x14), 0x0c00_0000, 0, lw(V1, 0x13d4, V0), JR_RA, 0];
        assert!(data_refs(&code, TS, TE).is_empty());
        // s0 survives the call.
        let code = [lui(16, 0x14), 0x0c00_0000, 0, lw(V1, 0x13d4, 16), JR_RA, 0];
        assert_eq!(data_refs(&code, TS, TE), BTreeSet::from(["G:001413d4".to_string()]));
    }
}
