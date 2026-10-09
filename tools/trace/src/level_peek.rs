//! `peek`: quick looks into one level's extracted data for porting work (docs/workflows/ghidra.md): the overlay's code
//! disassembled ([`crate::mips`]), words at `$gp`-relative or absolute addresses, the moby instances of some classes
//! with their pvars, cuboids and splines. Read-only; prints to stdout.

use anyhow::{Context, Result};
use rc_formats::level_overlay::LevelOverlay;
use std::path::Path;

/// `$gp` as the boot ELF sets it (the same on every level).
pub const GP: u32 = 0x166c00;

fn overlay(extracted: &Path, level: u32) -> Result<LevelOverlay> {
    let p = extracted.join(format!("levels/{level:02}/overlay.bin"));
    let b = std::fs::read(&p).with_context(|| format!("reading {}", p.display()))?;
    LevelOverlay::parse(&b).map_err(|e| anyhow::anyhow!("parsing {}: {e:?}", p.display()))
}

fn gameplay(level: u32) -> Result<std::sync::Arc<Vec<u8>>> { rc_formats::test_data::gameplay(level).context("gameplay (extracted/ missing?)") }

/// `n` instructions from `addr` (stops at the end of the text).
pub fn dis(extracted: &Path, level: u32, addr: u32, n: usize) -> Result<()> {
    let ov = overlay(extracted, level)?;
    for i in 0..n {
        let pc = addr + 4 * i as u32;
        let Some(w) = ov.u32(pc) else { break };
        let t = crate::mips::disasm(w, pc);
        // lui: the float the upper half would make (constants are loaded this way).
        let f = if w >> 26 == 0xf { format!("   (f:{})", f32::from_bits((w & 0xffff) << 16)) } else { String::new() };
        println!("{pc:08x}  {w:08x}  {t}{f}");
    }
    Ok(())
}

/// The words at `$gp`-relative offsets (`-0x5264`, `ad9c` = gp−0x5264 as the decompiler's `Gpffffad9c`) or absolute
/// addresses (`@0x1ba5d0`), as hex, int and float.
pub fn words(extracted: &Path, level: u32, specs: &[String]) -> Result<()> {
    let ov = overlay(extracted, level)?;
    for s in specs {
        let a = if let Some(x) = s.strip_prefix('@') {
            u32::from_str_radix(x.trim_start_matches("0x"), 16)?
        } else if let Some(x) = s.strip_prefix('-') {
            GP - u32::from_str_radix(x.trim_start_matches("0x"), 16)?
        } else {
            // A decompiler suffix: 0xffff0000 | x, i.e. gp − (0x10000 − x).
            GP.wrapping_sub(0x1_0000 - u32::from_str_radix(s.trim_start_matches("0x"), 16)?)
        };
        match ov.u32(a) {
            Some(v) => println!("{s}  @{a:#x}  {v:#010x}  i={}  f={}", v as i32, f32::from_bits(v)),
            None => println!("{s}  @{a:#x}  (not in the overlay: boot or bss)"),
        }
    }
    Ok(())
}

/// The moby instances of `classes` (negative numbers: instance indices) with their pvar words.
pub fn instances(level: u32, classes: &[i32], short: bool) -> Result<()> {
    use rc_formats::gameplay::{parse_moby_instances, parse_pvars};
    let g = gameplay(level)?;
    let inst = parse_moby_instances(&g)?;
    let pv = parse_pvars(&g)?;
    for (i, m) in inst.iter().enumerate() {
        if !classes.contains(&m.o_class) && !classes.contains(&-(i as i32)) { continue; }
        println!("#{i} class {} pos {:?} rot {:?} group {} uid {} pvar {} flags {:#x} mission {}", m.o_class, m.position, m.rotation, m.group, m.spawn_id, m.pvar_index, m.spawn_flags, m.unknown_4);
        if let Some(p) = m.pvar(&pv) {
            let w: Vec<String> = p
                .chunks(4)
                .enumerate()
                .map(|(k, c)| {
                    let v = u32::from_le_bytes(c.try_into().unwrap_or([0; 4]));
                    let looks_float = (0x3000_0000..0x5000_0000).contains(&v) || (0xb000_0000..0xd000_0000).contains(&v);
                    if looks_float { format!("+{:02x}={v:x}({})", k * 4, f32::from_bits(v)) } else { format!("+{:02x}={v:x}({})", k * 4, v as i32) }
                })
                .collect();
            let n = if short { w.len().min(24) } else { w.len() };
            for row in w[..n].chunks(8) { println!("   {}", row.join(" ")); }
        }
    }
    Ok(())
}

/// Cuboids by index: centre and rows.
pub fn cuboids(level: u32, idx: &[usize]) -> Result<()> {
    let v = rc_formats::volumes::parse_volumes(&gameplay(level)?)?;
    for &i in idx {
        match v.cuboids.get(i) {
            Some(s) => println!("cuboid {i}: centre {:?} r0 {:?} r1 {:?} r2 {:?}", s.matrix[3], s.matrix[0], s.matrix[1], s.matrix[2]),
            None => println!("cuboid {i}: none ({} cuboids)", v.cuboids.len()),
        }
    }
    Ok(())
}

/// Splines by index: the point count and the points.
pub fn splines(level: u32, idx: &[usize]) -> Result<()> {
    let sp = rc_formats::gameplay::parse_splines(&gameplay(level)?)?;
    for &i in idx {
        match sp.get(i) {
            Some(s) => {
                println!("spline {i}: {} points", s.len());
                for (k, p) in s.iter().enumerate() { println!("   {k}: {p:?}"); }
            }
            None => println!("spline {i}: none ({} splines)", sp.len()),
        }
    }
    Ok(())
}

/// The class table: each class's update function.
pub fn vtbl(extracted: &Path, level: u32, classes: &[i32]) -> Result<()> {
    let ov = overlay(extracted, level)?;
    for e in ov.vtbl() {
        if !(classes.is_empty() || classes.contains(&{ e.o_class })) { continue; }
        println!("class {} update {:#x} react {:#x}", e.o_class, e.update, e.w8);
        if !classes.is_empty() && e.w8 != 0 {
            let slots: Vec<String> = (0..6).map(|k| ov.u32(e.w8 + 4 * k).map_or("?".into(), |v| format!("{v:#x}"))).collect();
            println!("  slots {}", slots.join(" "));
        }
    }
    Ok(())
}
