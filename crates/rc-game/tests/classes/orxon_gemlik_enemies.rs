//! The remaining ready enemy units of W3 (docs/plan/creatures.md §11): Orxon's 1202 / 1199 / 1196 (level 10) and
//! Gemlik's turret 29 with its rider 36 and shot 1238 (level 13). The level harness is `creature_classes`'s.

use rc_formats::{gameplay, moby_spawn};

/// The pvars of the created instances (survey).
#[test]
#[ignore = "survey: prints, asserts nothing; run with --ignored --nocapture"]
fn survey_w3_pvars() {
    for (level, oc) in [(10u32, 1196i16), (10, 1199), (10, 1202), (13, 29)] {
        let Some(gp) = rc_formats::test_data::gameplay(level) else { eprintln!("skipped: no extracted/"); return };
        let inst = gameplay::parse_moby_instances(&gp).unwrap();
        let tests = moby_spawn::loader_spawns(&inst, &mut moby_spawn::SpawnSave::default());
        let spawned: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
        let pvars = gameplay::parse_pvars_spawned(&gp, &spawned).unwrap();
        let mut n = 0;
        for (i, m) in inst.iter().enumerate() {
            if m.o_class as i16 != oc || !spawned[i] { continue; }
            n += 1;
            let p = usize::try_from(m.pvar_index).ok().and_then(|k| pvars.get(k)).and_then(|b| b.as_deref()).unwrap_or(&[]);
            eprintln!("L{level:02} class {oc} inst {i} pos {:?} rot {:?} scale {} group {} pvars {}", m.position, m.rotation, m.scale, m.group, p.len());
            if n > 4 && !(oc == 1202 && [690usize, 700, 723].contains(&i)) { continue; }
            for (k, c) in p.chunks(16).enumerate() {
                if (k < 0x10 && oc != 29) || (oc == 1202 && k < 0x19) { continue; }
                let w: Vec<String> = c.chunks(4).map(|b| { let u = u32::from_le_bytes([b[0], b[1], b.get(2).copied().unwrap_or(0), b.get(3).copied().unwrap_or(0)]); let f = f32::from_bits(u); if f.is_finite() && f.abs() > 1e-4 && f.abs() < 1e6 { format!("{u:08x}({f})") } else { format!("{u:08x}") } }).collect();
                eprintln!("  +{:03x}: {}", k * 16, w.join(" "));
            }
        }
        eprintln!("L{level:02} class {oc}: {n} created");
    }
    if let Some(gp) = rc_formats::test_data::gameplay(10) {
        let inst = gameplay::parse_moby_instances(&gp).unwrap();
        let tests = moby_spawn::loader_spawns(&inst, &mut moby_spawn::SpawnSave::default());
        let spawned: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
        let pvars = gameplay::parse_pvars_spawned(&gp, &spawned).unwrap();
        for (i, m) in inst.iter().enumerate() {
            if ![1199, 1196].contains(&(m.o_class as i16)) { continue; }
            let p = usize::try_from(m.pvar_index).ok().and_then(|k| pvars.get(k)).and_then(|b| b.as_deref()).unwrap_or(&[]);
            let w = |o: usize| if p.len() >= o + 4 { i32::from_le_bytes(p[o..o + 4].try_into().unwrap()) } else { -99 };
            eprintln!("inst {i} class {} spawned {} sid {} pos {:?} grp {} 150 {} 160 {:x} {:x} 174 {} 178 {} 17c {} 188 {} 18c {} 194 {}", m.o_class, spawned[i], m.spawn_id, m.position, m.group, w(0x150), w(0x160), w(0x164), w(0x174), f32::from_bits(w(0x178) as u32), w(0x17c), f32::from_bits(w(0x188) as u32), w(0x18c), w(0x194));
        }
        for i in [0x28busize, 0x28c, 0x28d, 0x292, 0x293, 0x296] { if let Some(m) = inst.get(i) { eprintln!("moby {i:#x}: class {} spawned {} pos {:?} grp {}", m.o_class, spawned[i], m.position, m.group); } }
    }
    for (level, oc) in [(10u32, 1196i16), (10, 1199), (10, 1202), (13, 29), (13, 36), (13, 1238)] {
        let Ok(b) = std::fs::read(rc_formats::test_data::level_dir(level).join("overlay.bin")) else { return };
        let ov = rc_formats::level_overlay::LevelOverlay::parse(&b).unwrap();
        for e in ov.vtbl().iter().filter(|e| e.o_class as i16 == oc) {
            let slots: Vec<String> = (0..6).map(|k| format!("{:#x}", ov.u32(e.w8 + 4 * k).unwrap_or(0))).collect();
            eprintln!("L{level:02} class {oc}: update {:#x} table {:#x} slots {}", e.update, e.w8, slots.join(" "));
        }
    }
}

use crate::creature_classes::{hero_at, load, wrench, Lv};
use rc_game::hero::Hero;
use rc_game::moby_runtime::MobyId;
use rc_game::moby_update::classes::units::{self, gemlik_turret as gt, orxon_brawler as ob, orxon_flyers as of};
use rc_game::moby_update::classes::ClassUpdate;
use rc_game::moby_update::creature::react;
use rc_game::moby_update::services::{pvar as p, HitTemplate};
use rc_game::ps2v::Pf;
use std::collections::HashMap;

/// A point inside area path `path` (a 2-D polygon, `region::point_in_polygon`) near `near` (a 0.5 grid over its
/// bounding box), with `near`'s z.
fn inside(lv: &mut Lv, path: usize, near: [f32; 4]) -> Option<[f32; 3]> {
    let pts: Vec<[f32; 4]> = lv.svc.splines[path].iter().map(|q| q.map(f32::from_bits)).collect();
    let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |a, q| (a.0.min(q[0]), a.1.max(q[0])));
    let (y0, y1) = pts.iter().fold((f32::MAX, f32::MIN), |a, q| (a.0.min(q[1]), a.1.max(q[1])));
    let hero = hero_at([0.0, 0.0, -100.0]);
    let w = lv.world(&hero);
    let mut best: Option<([f32; 3], f32)> = None;
    let mut x = x0;
    while x <= x1 {
        let mut y = y0;
        while y <= y1 {
            if rc_game::moby_update::creature::region::point_in_polygon(&w, path, [x, y, near[2], 1.0]) {
                let d = (x - near[0]).powi(2) + (y - near[1]).powi(2);
                if best.is_none_or(|b| d < b.1) { best = Some(([x, y, near[2]], d)); }
            }
            y += 0.5;
        }
        x += 0.5;
    }
    best.map(|b| b.0)
}

#[test]
#[ignore = "survey: prints, asserts nothing; run with --ignored --nocapture"]
fn survey_w3_level() {
    for level in [10u32, 13] {
        let Some(mut lv) = load(level) else { eprintln!("skipped"); return };
        let hero = hero_at([0.0, 0.0, -100.0]);
        lv.load_pass(&hero);
        for oc in [1196i16, 1199, 1202, 29] {
            for id in lv.of_class(oc) {
                let m = &lv.table.mobys[id];
                let q = &m.pvars;
                let w = |o: usize| p::i32(q, o);
                if oc == 29 {
                    let c = p::i32(q, 0x50);
                    let cc = lv.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c).map(|s| s.centre());
                    eprintln!("L{level} 29 #{id} pos {:?} yaw {} cuboid {c} centre {cc:?} cone {}", m.position, m.rotation[2], p::ff(q, 0x6c));
                } else if oc == 1202 {
                    eprintln!("L{level} 1202 #{id} pos {:?} grp {} area {} watch {} idle {} ronly {}", m.position, m.group, w(0x1b0), w(0x1bc), w(0x1b4), w(0x1dc));
                } else {
                    eprintln!("L{level} {oc} #{id} pos {:?} grp {} path {} wake {:?} mode {} area {} warea {} cuboid {}", m.position, m.group, w(0x150), [w(0x160), w(0x164), w(0x168)], w(0x174), w(0x17c), w(0x18c), w(0x194));
                }
            }
        }
        if level == 10 {
            for (path, id, dx) in [(147usize, 647usize, 0.0f32), (6, 629, 6.0), (111, 622, 0.0), (21, 668, 5.0), (25, 626, 0.0)] {
                let q = lv.table.mobys[id].position;
                eprintln!("inside {path} near #{id}: {:?}", inside(&mut lv, path, [q[0] + dx, q[1], q[2], 1.0]));
            }
            for i in [0x28busize, 0x28c, 0x28d, 0x292, 0x293, 0x296, 0x2a4, 0x2b1, 0x2b2] {
                let m = &lv.table.mobys[i];
                eprintln!("moby {i:#x}: class {} grp {} state {}", m.o_class, m.group, m.state);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// Resolution

fn unit(name: &str) -> ClassUpdate {
    let i = units::PORTS.iter().position(|u| u.unit == name).unwrap_or_else(|| panic!("no unit {name}"));
    ClassUpdate::Unit(i as u16)
}

/// (unit row, class, the levels whose table runs it, created instances on them).
const EXPECTED: &[(&str, i16, &[u32], usize)] = &[
    ("U335 1196", 1196, &[10], 7),
    ("U336 1199", 1199, &[10], 15),
    ("U337 1202", 1202, &[10], 57),
    ("U407 29", 29, &[13], 9),
    ("U407 36", 36, &[13], 0),
    ("U407 1238", 1238, &[13], 0),
];

/// Every unit's classes resolve to the unit on its levels only (code identity), with the census's instance counts;
/// the Orxon and Gemlik classes have no Suck Cannon reaction table (the levels' shared no-op tables).
#[test]
fn w3_units_resolve_on_their_levels() {
    let Some(ov1) = crate::common::overlay(1) else { eprintln!("skipped: no extracted/"); return };
    let mut created: HashMap<(&str, i16), usize> = HashMap::new();
    for level in 0..19u32 {
        let pt = crate::common::ports(level, &[]).unwrap();
        let gp = rc_formats::test_data::gameplay(level).unwrap();
        let inst = gameplay::parse_moby_instances(&gp).unwrap();
        let tests = moby_spawn::loader_spawns(&inst, &mut moby_spawn::SpawnSave::default());
        for &(name, oc, levels, _) in EXPECTED {
            let u = unit(name);
            if levels.contains(&level) {
                assert_eq!(pt.get(oc), Some(u), "level {level:02} class {oc}");
                let n = inst.iter().zip(&tests).filter(|(m, t)| m.o_class as i16 == oc && t.spawn).count();
                *created.entry((name, oc)).or_default() += n;
            } else if pt.in_table(oc) {
                assert_ne!(pt.get(oc), Some(u), "level {level:02} class {oc}");
            }
        }
        let t = react::tables_from_overlays(&crate::common::overlay(level).unwrap(), &ov1, &crate::common::overlay);
        for oc in [1196i16, 1199, 1202, 29, 36, 1238] {
            if (level == 10 && [1196, 1199, 1202].contains(&oc)) || (level == 13 && [29, 36, 1238].contains(&oc)) {
                assert_eq!(t.get(&oc), None, "level {level:02} class {oc}: no reaction table");
            }
        }
    }
    eprintln!("created instances now ported: {created:?}");
    for &(name, oc, _, n) in EXPECTED { assert_eq!(created.get(&(name, oc)).copied().unwrap_or(0), n, "{name} class {oc}"); }
}

// ---------------------------------------------------------------------------------------------------
// Harness bits

/// Level `level` after its load pass with Ratchet at `at`.
fn level_at(level: u32, at: [f32; 3]) -> Option<(Lv, Hero)> {
    let mut lv = load(level)?;
    let hero = hero_at(at);
    lv.load_pass(&hero);
    Some((lv, hero))
}

/// `n` ticks with `ids` marked drawn before each (as MobyProc would for mobys on screen), recording each one's states.
fn run(lv: &mut Lv, hero: &Hero, ids: &[MobyId], n: usize) -> Vec<Vec<u8>> {
    let mut seen = vec![Vec::new(); ids.len()];
    for _ in 0..n {
        for &i in ids { if lv.table.mobys[i].state < 0x80 { lv.table.mobys[i].visible = 1; } }
        lv.tick(hero);
        for (k, &i) in ids.iter().enumerate() {
            let s = lv.table.mobys[i].state;
            if seen[k].last() != Some(&s) { seen[k].push(s); }
        }
    }
    seen
}

fn hits_from(lv: &Lv, target: MobyId, class: i16) -> Vec<rc_game::moby_update::services::HitRecord> {
    lv.hits_on(target).into_iter().filter(|h| h.attacker.is_some_and(|a| lv.table.mobys[a].o_class == class)).collect()
}

fn sounds(lv: &Lv, id: MobyId, index: i32) -> usize { lv.svc.sounds.iter().filter(|e| e.moby == id && e.index == index).count() }
fn parts(lv: &Lv, ty: u8) -> u64 { lv.svc.fx.part_spawns.get(&ty).copied().unwrap_or(0) }

// ---------------------------------------------------------------------------------------------------
// U334: the path scout 1196 (#622: path 2, waiting area 111, wakes 1202 #653 / #652 / #651)

/// A scout run: the level, Ratchet, the scout's states and its wake calls (its state, the three brawlers').
type ScoutRun = (Lv, Hero, Vec<Vec<u8>>, Vec<(u8, [u8; 3])>);

fn scout_run(ticks: usize) -> Option<ScoutRun> {
    let (mut lv, _) = level_at(10, [0.0, 0.0, -100.0])?;
    let near = lv.table.mobys[622].position;
    let at = inside(&mut lv, 111, near)?;
    let hero = hero_at([at[0], at[1], 35.0]);
    let mut seen = vec![Vec::new()];
    let mut wakes = Vec::new();
    for _ in 0..ticks {
        let before = lv.table.mobys[622].state;
        let r = run(&mut lv, &hero, &[622], 1);
        if seen[0].last() != r[0].last() { seen[0].push(r[0][0]); }
        if before == of::scout::CALL && lv.table.mobys[622].state != of::scout::CALL {
            wakes.push((lv.table.mobys[622].state, [651, 652, 653].map(|i| lv.table.mobys[i].state)));
        }
    }
    Some((lv, hero, seen, wakes))
}

/// Ratchet in its waiting area: it wakes (2), rises to home + 2 (4), flies its path (5); at the nodes with a w it
/// wakes the brawlers' groups of its +0x160 mobys (6: the sleepers 1 → 2); the path's end → it hovers about the
/// ground point there (10 ⇄ 9). It never attacks Ratchet.
#[test]
fn scout_wakes_flies_its_path_and_wakes_the_brawlers() {
    let Some((lv, _, seen, wakes)) = scout_run(1500) else { eprintln!("skipped: no extracted/"); return };
    eprintln!("scout states {:?}; after its calls (state, #651..#653) {wakes:?}", seen[0]);
    let s = &seen[0];
    for x in [of::scout::WAKE, of::scout::RISE, of::scout::PATH, of::scout::CALL, of::scout::HOVER, of::scout::PICK] { assert!(s.contains(&x), "{x} in {s:?}"); }
    assert!(!wakes.is_empty(), "a call");
    assert!(wakes.iter().any(|(_, st)| st.iter().any(|&x| x != 1)), "a brawler of the called groups woke");
    assert!(hits_from(&lv, lv.hero_idx, 1196).is_empty(), "no attack");
    let m = &lv.table.mobys[622];
    let home = p::v4f(&m.pvars, of::pv::HOME);
    assert!(((m.position[0] - home[0]).powi(2) + (m.position[1] - home[1]).powi(2)).sqrt() < 4.0, "hovers near the path's end");
}

#[test]
fn scout_run_is_deterministic() {
    let Some((a, _, sa, wa)) = scout_run(700) else { eprintln!("skipped"); return };
    let (b, _, sb, wb) = scout_run(700).unwrap();
    assert_eq!((sa, wa), (sb, wb));
    for i in [622usize, 651, 652, 653] { assert_eq!(a.table.mobys[i].position.map(f32::to_bits), b.table.mobys[i].position.map(f32::to_bits)); }
}

/// A weaker hit: the flash (0xfa) and the cooldown; the wrench kills it: 0xb (untargetable, blend 5), the flight,
/// then `SetDeathBits` (flags 0x200: the save bit and bolts), the piece explosion (its light) and the delete. With
/// Ratchet in the help director's cuboid (the level word) the kill takes the skill-point branch (G-SAV-007: counted).
#[test]
fn weapons_hurt_and_kill_a_scout() {
    let Some((mut lv, hero)) = level_at(10, [217.0, 262.0, 35.0]) else { eprintln!("skipped: no extracted/"); return };
    let h = lv.hero_idx;
    let mut t = wrench(h, [1.0, 0.0]);
    t.damage = Pf::f(0.5);
    lv.hit(&hero, 622, &t);
    lv.tick(&hero);
    let m = &lv.table.mobys[622];
    assert_eq!((p::ff(&m.pvars, of::pv::D), m.pvars[of::pv::FLASH + 7]), (0.5, 0xfa));
    assert!(p::i16(&m.pvars, of::pv::COOLDOWN) > 50);
    // The help director 1344 writes the word each tick (Ratchet in its cuboid): out of the way, the test sets it.
    for d in lv.of_class(1344) { lv.world(&hero).delete_moby(d); }
    lv.svc.units.set_word(units::help_orxon::AIR_WORD, 1);
    for _ in 0..60 { lv.tick(&hero); }
    lv.hit(&hero, 622, &wrench(h, [1.0, 0.0]));
    lv.tick(&hero);
    let m = &lv.table.mobys[622];
    assert_eq!(m.state, of::scout::DYING);
    assert_eq!(m.mode & rc_game::moby_runtime::mode::TARGETABLE, 0);
    // Skill point 0x13d418 (index 0x10) through `story::award_skill_point` (ported by the story lane).
    assert_eq!(lv.svc.interact.game.skill_points.get(0x10).copied(), Some(1));
    let lights0 = lv.svc.creatures.lights;
    for _ in 0..240 {
        lv.tick(&hero);
        if lv.table.mobys[622].state >= 0x80 { break; }
    }
    assert!(lv.table.mobys[622].state >= 0x80, "deleted after the flight");
    assert!(lv.svc.save.death.contains(&(10, lv.table.mobys[622].spawn_id)), "the save bit");
    assert!(lv.svc.creatures.lights > lights0, "the explosion's light");
}

// ---------------------------------------------------------------------------------------------------
// U335: the swoop flyers 1199 (#629 / #630: group 3, area 6; #631 / #632: the cuboid 1 flyers)

fn swoop_run(ticks: usize) -> Option<(Lv, Hero, Vec<Vec<u8>>)> {
    let (mut lv, _) = level_at(10, [0.0, 0.0, -100.0])?;
    let home = lv.table.mobys[629].position;
    let at = inside(&mut lv, 6, [home[0] + 6.0, home[1], 35.0, 1.0])?;
    let hero = hero_at([at[0], at[1], 35.0]);
    let seen = run(&mut lv, &hero, &[629, 630], ticks);
    Some((lv, hero, seen))
}

/// Ratchet in their area: one winds up (6), dives (7; class sound 0 at the end) and bites (8): hit records on his moby
/// (flags 1, damage 1, class 1199, the exact push); the group's +0xbc delays the other; they go back (5) and hover.
#[test]
fn swoop_flyers_dive_and_bite_ratchet() {
    let Some((lv, _, seen)) = swoop_run(1200) else { eprintln!("skipped: no extracted/"); return };
    eprintln!("states {seen:?}");
    assert!(seen.iter().any(|s| s.windows(3).any(|w| w == [of::swoop::WIND_UP, of::swoop::DIVE, of::swoop::BITE])), "{seen:?}");
    let hits = hits_from(&lv, lv.hero_idx, 1199);
    eprintln!("bites: {}", hits.len());
    assert!(!hits.is_empty() && hits.iter().all(|h| h.flags == 1 && h.damage.to_f32() == 1.0 && h.b29 == 1 && h.h2a == 1199 && h.dir[3] == Pf::b(0x45af_df66)));
    assert!(sounds(&lv, 629, 0) + sounds(&lv, 630, 0) > 0, "the dive's sound");
}

#[test]
fn swoop_run_is_deterministic() {
    let Some((a, _, sa)) = swoop_run(500) else { eprintln!("skipped"); return };
    let (b, _, sb) = swoop_run(500).unwrap();
    assert_eq!(sa, sb);
    for i in [629usize, 630] { assert_eq!(a.table.mobys[i].position.map(f32::to_bits), b.table.mobys[i].position.map(f32::to_bits)); }
}

/// The cuboid flyers: Ratchet in cuboid 1: they charge at the cuboid's height (10), bite (0xb: records flags 0x10001,
/// damage 1) and pull back (0xc), then hover again (9).
#[test]
fn cuboid_flyers_charge_and_bite_ratchet() {
    let Some((lv0, _)) = level_at(10, [0.0, 0.0, -100.0]) else { eprintln!("skipped: no extracted/"); return };
    let c = lv0.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, 1).unwrap().centre();
    drop(lv0);
    let (mut lv, _) = level_at(10, [c[0], c[1], c[2]]).unwrap();
    let hero = hero_at([c[0], c[1], c[2]]);
    let seen = run(&mut lv, &hero, &[631, 632], 900);
    eprintln!("cuboid {c:?}: states {seen:?}");
    assert!(seen.iter().any(|s| s.windows(2).any(|w| w == [of::swoop::CHARGE, of::swoop::CUBOID_BITE])), "{seen:?}");
    let hits = hits_from(&lv, lv.hero_idx, 1199);
    assert!(!hits.is_empty() && hits.iter().all(|h| h.flags == 0x1_0001 && h.damage.to_f32() == 1.0));
    assert!(seen.iter().any(|s| s.contains(&of::swoop::PULL_BACK)));
}

/// Its own class's hit is ignored; a weaker one flashes it; the wrench kills it: 0xd (class sound 2), the flight,
/// `SetDeathBits`, the piece explosion, deleted.
#[test]
fn weapons_hurt_and_kill_a_swoop_flyer() {
    let Some((mut lv, hero)) = level_at(10, [240.0, 250.0, 35.0]) else { eprintln!("skipped: no extracted/"); return };
    let mut own = wrench(630, [1.0, 0.0]);
    own.h1a = 1199;
    lv.hit(&hero, 629, &own);
    lv.tick(&hero);
    assert_eq!(p::ff(&lv.table.mobys[629].pvars, of::pv::D), 1.0, "its own class's hit does nothing");
    let h = lv.hero_idx;
    for _ in 0..60 { lv.tick(&hero); }
    let mut t = wrench(h, [1.0, 0.0]);
    t.damage = Pf::f(0.25);
    lv.hit(&hero, 629, &t);
    lv.tick(&hero);
    assert_eq!(lv.table.mobys[629].pvars[of::pv::FLASH + 7], 0xfa);
    for _ in 0..60 { lv.tick(&hero); }
    lv.hit(&hero, 629, &wrench(h, [1.0, 0.0]));
    lv.tick(&hero);
    assert_eq!(lv.table.mobys[629].state, of::swoop::DYING);
    assert_eq!(sounds(&lv, 629, 2), 1);
    for _ in 0..300 {
        lv.tick(&hero);
        if lv.table.mobys[629].state >= 0x80 { break; }
    }
    assert!(lv.table.mobys[629].state >= 0x80);
    assert!(lv.svc.save.death.contains(&(10, lv.table.mobys[629].spawn_id)));
}

// ---------------------------------------------------------------------------------------------------
// U336: the brawlers 1202 (group 10: #668..#673, area 21)

const PACK: [MobyId; 6] = [668, 669, 670, 671, 672, 673];

fn pack_run(ticks: usize) -> Option<(Lv, Hero, Vec<Vec<u8>>)> {
    let (mut lv, _) = level_at(10, [0.0, 0.0, -100.0])?;
    let at = inside(&mut lv, 21, [276.0, 272.0, 40.0, 1.0])?;
    let hero = hero_at([at[0], at[1], 40.0]);
    lv.tick(&hero);
    p::set_i32(&mut lv.table.mobys[668].pvars, ob::pv::LURE, 1);
    let seen = run(&mut lv, &hero, &PACK, ticks);
    Some((lv, hero, seen))
}

/// The Taunter's lure on one brawler: its group wakes (1 → 2), they run at Ratchet spread about him (5: each takes a
/// different side), swing (7, sequence 6 / 7): hit records on his moby (flags 0x10001, damage 3.1, type 0 / 1, class
/// 1202, the exact push).
#[test]
fn brawlers_wake_run_spread_and_swing_at_ratchet() {
    let Some((lv, hero, seen)) = pack_run(900) else { eprintln!("skipped: no extracted/"); return };
    eprintln!("states {seen:?}");
    assert!(seen.iter().all(|s| s.contains(&ob::st::WAKE)), "the group wakes");
    assert!(seen.iter().filter(|s| s.contains(&ob::st::SWING)).count() >= 2, "{seen:?}");
    let hits = hits_from(&lv, lv.hero_idx, 1202);
    eprintln!("swings that hit: {}", hits.len());
    assert!(!hits.is_empty() && hits.iter().all(|h| h.flags == 0x1_0001 && h.damage == Pf::b(0x4046_6666) && (h.b28, h.b29, h.h2a) == (0, 1, 1202)));
    let hp = hero.pos.map(|x| x.to_f32());
    let mut angles: Vec<f32> = PACK.iter().map(|&i| { let q = lv.table.mobys[i].position; (q[1] - hp[1]).atan2(q[0] - hp[0]) }).collect();
    angles.sort_by(f32::total_cmp);
    eprintln!("angles about Ratchet {angles:?}");
    assert!(angles.windows(2).any(|w| w[1] - w[0] > 0.3), "spread out");
}

#[test]
fn brawler_run_is_deterministic() {
    let Some((a, _, sa)) = pack_run(400) else { eprintln!("skipped"); return };
    let (b, _, sb) = pack_run(400).unwrap();
    assert_eq!(sa, sb);
    for i in PACK { assert_eq!(a.table.mobys[i].position.map(f32::to_bits), b.table.mobys[i].position.map(f32::to_bits)); }
}

/// The wrench knocks one back (10: the arc, flash 0xfa, cooldown 60, the group woken) and it counter-swings (7) on
/// landing; a heavy hit kills: 0xb, then the death: the scorch (type 52), the sparks (160 type-2 blobs), the piece
/// explosion's light, the six pieces (0x775, 0x77a–0x77c, 0x623, 0x624), `SetDeathBits` and the delete.
#[test]
fn weapons_knock_back_and_kill_a_brawler() {
    let Some((mut lv, _)) = level_at(10, [0.0, 0.0, -100.0]) else { eprintln!("skipped: no extracted/"); return };
    let at = inside(&mut lv, 21, [273.0, 279.0, 40.0, 1.0]).unwrap();
    let hero = hero_at([at[0], at[1], 40.0]);
    lv.tick(&hero);
    let h = lv.hero_idx;
    lv.hit(&hero, 670, &wrench(h, [0.0, 1.0]));
    lv.tick(&hero);
    let m = &lv.table.mobys[670];
    eprintln!("after the wrench: state {} hp {}", m.state, p::ff(&m.pvars, ob::pv::D));
    assert_eq!(m.state, ob::st::KNOCKED);
    assert_eq!(m.pvars[ob::pv::FLASH + 7], 0xfa);
    assert!(PACK.iter().filter(|&&i| i != 670).any(|&i| lv.table.mobys[i].state != ob::st::SLEEP), "the group woke");
    let seen = run(&mut lv, &hero, &[670], 120);
    eprintln!("then {seen:?}");
    assert!(seen[0].contains(&ob::st::SWING), "the counter-swing");
    let (p52, p02, lights) = (parts(&lv, 52), parts(&lv, 2), lv.svc.creatures.lights);
    let pieces0: usize = [0x775, 0x77a, 0x77b, 0x77c, 0x623, 0x624].iter().map(|&c| lv.of_class(c).len()).sum();
    let mut t = wrench(h, [0.0, 1.0]);
    t.damage = Pf::f(10.0);
    t.b18 = 1;
    lv.hit(&hero, 671, &t);
    lv.tick(&hero);
    assert!(lv.table.mobys[671].state == ob::st::DEAD || lv.table.mobys[671].state >= 0x80);
    lv.tick(&hero);
    assert!(lv.table.mobys[671].state >= 0x80, "exploded");
    let pieces: usize = [0x775, 0x77a, 0x77b, 0x77c, 0x623, 0x624].iter().map(|&c| lv.of_class(c).len()).sum();
    eprintln!("pieces {pieces0} -> {pieces}, type 52 +{}, type 2 +{}", parts(&lv, 52) - p52, parts(&lv, 2) - p02);
    assert_eq!(pieces - pieces0, 6);
    assert_eq!(parts(&lv, 52) - p52, 1);
    assert_eq!(parts(&lv, 2) - p02, 160);
    assert!(lv.svc.creatures.lights > lights);
    assert!(lv.svc.save.death.contains(&(10, lv.table.mobys[671].spawn_id)));
}

/// The Morph-o-Ray aimed at a sleeping brawler (hand item 0x15, its target) wakes it and its group.
#[test]
fn the_morph_ray_aim_wakes_a_brawler_group() {
    let Some((mut lv, mut hero)) = level_at(10, [276.0, 300.0, 40.0]) else { eprintln!("skipped: no extracted/"); return };
    lv.tick(&hero);
    assert!(PACK.iter().all(|&i| lv.table.mobys[i].state == ob::st::SLEEP));
    hero.items.slot.id = rc_game::hero::morph_ray::MORPH;
    hero.weapons.reactive.morph.target = Some(669);
    lv.tick(&hero);
    assert!(PACK.iter().all(|&i| lv.table.mobys[i].state != ob::st::SLEEP), "{:?}", PACK.map(|i| lv.table.mobys[i].state));
}

/// Falling (0xc) with the ground at or below it: the death (pieces, `SetDeathBits`, deleted).
#[test]
fn a_falling_brawler_dies() {
    let Some((mut lv, hero)) = level_at(10, [276.0, 300.0, 40.0]) else { eprintln!("skipped: no extracted/"); return };
    lv.tick(&hero);
    lv.table.mobys[672].state = ob::st::FALL;
    lv.table.mobys[672].position[2] += 3.0;
    lv.tick(&hero);
    eprintln!("state {} pos {:?}", lv.table.mobys[672].state, lv.table.mobys[672].position);
    assert!(lv.table.mobys[672].state >= 0x80);
    assert_eq!(lv.of_class(0x775).len(), 1);
    assert!(lv.svc.save.death.contains(&(10, lv.table.mobys[672].spawn_id)));
}

/// Falling (0xc) into a hazard surface (`CollType` > 1 and ≠ 3, within the probe's 0.5 above it): the death flight
/// (0xb: flash 0xfa, not targetable, `SetDeathBits(m, 0, −1)`, sequence 0xd), then the death's explosion.
#[test]
fn a_brawler_falling_into_a_hazard_takes_the_death_flight() {
    let Some((mut lv, hero)) = level_at(10, [276.0, 300.0, 40.0]) else { eprintln!("skipped: no extracted/"); return };
    lv.tick(&hero);
    let mut spot = None;
    'find: for x in (0..256).map(|k| 4.0 * k as f32) {
        for y in (0..256).map(|k| 4.0 * k as f32) {
            let w = lv.world(&hero);
            let g = rc_game::moby_update::creature::ground::ground(&w, [x, y, 500.0, 1.0], 0.5, 0);
            if g.hit && 1 < g.surface && g.surface != 3 { spot = Some((x, y, g.z, g.surface)); break 'find; }
        }
    }
    let Some((x, y, z, surface)) = spot else { panic!("no hazard surface on level 10") };
    eprintln!("hazard surface {surface} at ({x}, {y}, {z})");
    let b = 672;
    lv.table.mobys[b].state = ob::st::FALL;
    lv.table.mobys[b].position = [x, y, z - 0.25, 1.0];
    for o in 0..0xc { lv.table.mobys[b].pvars[ob::pv::K + o] = 0; }
    lv.tick(&hero);
    let m = &lv.table.mobys[b];
    eprintln!("state {} flash {:#x} mode {:#x}", m.state, m.pvars[ob::pv::FLASH + 7], m.mode);
    assert_eq!(m.state, ob::st::DEAD, "the death flight");
    assert_eq!(m.pvars[ob::pv::FLASH + 7], 0xfa);
    assert_eq!(m.mode & rc_game::moby_runtime::mode::TARGETABLE, 0);
    assert!(lv.svc.save.death.contains(&(10, m.spawn_id)));
    for _ in 0..300 { lv.tick(&hero); if lv.table.mobys[b].state >= 0x80 { break; } }
    assert!(lv.table.mobys[b].state >= 0x80, "then the death");
}

// ---------------------------------------------------------------------------------------------------
// U406: Gemlik's turret 29 (#9: home yaw π, cuboid 0), its rider 36 and shot 1238

fn turret_run(ticks: usize) -> Option<(Lv, Hero, Vec<Vec<u8>>)> {
    let (lv0, _) = level_at(13, [0.0, 0.0, -100.0])?;
    let c = lv0.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, 0)?.centre();
    let at = [c[0], c[1], lv0.table.mobys[9].position[2]];
    drop(lv0);
    let (mut lv, hero) = level_at(13, at)?;
    let seen = run(&mut lv, &hero, &[9], ticks);
    Some((lv, hero, seen))
}

/// The first update creates its rider (36) at the turret; Ratchet in its cuboid in front: it tracks him (3) and fires
/// every 2 ticks: shots 1238 whose hits on his moby are records (flags 0x10001, damage 1, type 1 / 1, class 1238)
/// with class sound 0 and the five type-27 sparks; the rider rides (its rows are the turret's).
#[test]
fn turret_tracks_and_fires_at_ratchet() {
    let Some((lv, _, seen)) = turret_run(240) else { eprintln!("skipped: no extracted/"); return };
    eprintln!("states {seen:?}");
    for i in lv.of_class(1238).into_iter().take(3) { let m = &lv.table.mobys[i]; eprintln!("shot #{i} pos {:?} vel {:?} life {}", m.position, p::v4f(&m.pvars, 0), p::i32(&m.pvars, 0x2c)); }
    eprintln!("shots alive {} hero moby at {:?}", lv.of_class(1238).len(), lv.table.mobys[lv.hero_idx].position);
    assert!(seen[0].contains(&gt::st::FIRE));
    let r = (p::i32(&lv.table.mobys[9].pvars, gt::pv::RIDER) - 1) as usize;
    assert_eq!(lv.table.mobys[r].o_class, 36);
    assert_eq!(lv.table.mobys[r].rows[0].map(f32::to_bits), lv.table.mobys[9].rows[0].map(f32::to_bits));
    let hits = hits_from(&lv, lv.hero_idx, 1238);
    eprintln!("shot hits {}; shot sounds {}; type 27 {}", hits.len(), lv.svc.sounds.iter().filter(|e| e.o_class == 1238 && e.index == 0).count(), parts(&lv, 27));
    assert!(!hits.is_empty() && hits.iter().all(|h| h.flags == 0x1_0001 && h.damage.to_f32() == 1.0 && (h.b28, h.b29) == (1, 1)));
    assert!(lv.svc.sounds.iter().any(|e| e.o_class == 1238 && e.index == 0));
}

#[test]
fn turret_run_is_deterministic() {
    let Some((a, _, sa)) = turret_run(120) else { eprintln!("skipped"); return };
    let (b, _, sb) = turret_run(120).unwrap();
    assert_eq!(sa, sb);
    assert_eq!(a.table.mobys[9].rotation.map(f32::to_bits), b.table.mobys[9].rotation.map(f32::to_bits));
    assert_eq!(a.of_class(1238).len(), b.of_class(1238).len());
}

/// With a drone out (0x141346, slot 0x141370) a shot within 2 of Ratchet is taken: the drone gets its record, the
/// shot stops and fizzles five ticks later (three type-51 sparks, deleted).
#[test]
fn a_drone_takes_the_turret_shot() {
    let Some((mut lv, hero, _)) = turret_run(4) else { eprintln!("skipped: no extracted/"); return };
    let drone = 10;
    lv.svc.drones.count = 1;
    lv.svc.drones.slots[0] = Some(drone);
    let p51 = parts(&lv, 51);
    for _ in 0..30 { lv.tick(&hero); }
    let h = lv.hits_on(drone);
    eprintln!("drone records {h:?}");
    assert!(h.iter().any(|r| r.flags == 0x1_0001 && r.h2a == 1238));
    assert!(parts(&lv, 51) >= p51 + 3);
}

/// Splash (flag 0x800000) from the front half does nothing; from behind it kills the rider: flash, `SetDeathBits`,
/// sequence 3 (2), the turret → 4: its blast and death bits, deleted; the rider bursts when its sequence ends.
#[test]
fn the_rider_shields_the_front_and_its_death_blows_up_the_turret() {
    let Some((mut lv, _)) = level_at(13, [560.0, 418.0, 304.0]) else { eprintln!("skipped: no extracted/"); return };
    let hero = hero_at([560.0, 418.0, 304.0]);
    lv.tick(&hero);
    lv.tick(&hero);
    let r = (p::i32(&lv.table.mobys[9].pvars, gt::pv::RIDER) - 1) as usize;
    let h = lv.hero_idx;
    let splash = HitTemplate { dir: [Pf::ZERO; 4], attacker: Some(h), flags: 0x81_0001, b18: 2, b19: 1, h1a: 0, damage: Pf::ONE, w20: 0x81_0001 };
    lv.hit(&hero, r, &splash);
    lv.tick(&hero);
    // Reaction 1 (column 0) zeroes the health as in the game, but the shielded hit's out5 = 1 keeps it alive.
    assert_eq!(lv.table.mobys[r].state, 1, "shielded");
    assert_ne!(lv.table.mobys[9].state, gt::st::DEAD);
    let hero2 = hero_at([600.0, 418.0, 304.0]);
    for _ in 0..70 { lv.tick(&hero2); }
    lv.hit(&hero2, r, &splash);
    lv.tick(&hero2);
    assert_eq!(lv.table.mobys[r].state, 2, "the rider falls");
    assert_eq!(lv.table.mobys[9].state, gt::st::DEAD);
    let lights = lv.svc.creatures.lights;
    lv.tick(&hero2);
    assert!(lv.table.mobys[9].state >= 0x80, "the turret blew up");
    assert!(lv.svc.save.death.contains(&(13, lv.table.mobys[9].spawn_id)));
    assert!(lv.svc.creatures.lights > lights);
    for _ in 0..200 { lv.tick(&hero2); if lv.table.mobys[r].state >= 0x80 { break; } }
    assert!(lv.table.mobys[r].state >= 0x80, "the rider burst");
}

/// A turret whose rider is gone blows up a second later (5 → 4).
#[test]
fn a_turret_without_its_rider_blows_up() {
    let Some((mut lv, hero)) = level_at(13, [0.0, 0.0, -100.0]) else { eprintln!("skipped: no extracted/"); return };
    lv.tick(&hero);
    let r = (p::i32(&lv.table.mobys[10].pvars, gt::pv::RIDER) - 1) as usize;
    lv.world(&hero).delete_moby(r);
    let seen = run(&mut lv, &hero, &[10], 5);
    assert_eq!(&seen[0][..2], &[gt::st::ORPHAN, gt::st::DEAD]);
    assert!(lv.table.mobys[10].state >= 0x80);
}

// ---------------------------------------------------------------------------------------------------
// U475: Gemlik's stompers 1262, the shockwave ring

/// A stomper's ring rolls out for its whole life (`ticks(150)`, fading from 20 left) and is drawn: the points keep
/// their own fade (w) — the game's VecSub / VecAdd write x, y, z only, and the centre's w is the ring's timer
/// (+0x1fc), so taking it in faded the ring to nothing on its second tick (only the sparks were left).
#[test]
fn stomper_ring_rolls_out_and_is_drawn() {
    use rc_game::moby_update::classes::units::gemlik_robot as gr;
    let Some(mut lv) = load(13) else { eprintln!("skipped: no extracted/"); return };
    let b = lv.of_class(1262)[0];
    let (p, y) = (lv.table.mobys[b].position, lv.table.mobys[b].rotation[2]);
    let hero = hero_at([p[0] + y.cos() * 7.0, p[1] + y.sin() * 7.0, p[2] + 0.1]);
    lv.load_pass(&hero);
    let row = units::row(gr::REFERENCE_LEVEL, gr::DRAW_FN).unwrap();
    let fade = |lv: &Lv, j: usize| p::ff(&lv.table.mobys[b].pvars, 0x210 + 0x10 * j + 0xc);
    let (mut launched, mut lit) = (None, 0);
    for t in 0..300 {
        lv.tick(&hero);
        let pv = &lv.table.mobys[b].pvars;
        if pv.len() < 0x440 || p::ff(pv, 0x1fc) == 0.0 { continue; }
        launched.get_or_insert(t);
        if (1..15).all(|j| fade(&lv, j) == 1.0) { lit += 1; }
        let q = units::fx_quads(&lv.table, &lv.svc, row, b).expect("the ring's quads");
        assert!((0..16).all(|j| (0.0..=1.0).contains(&fade(&lv, j))), "tick {t}: fades out of 0..1");
        if t < launched.unwrap() + 100 { assert!(q.quads.iter().any(|q| q.rgba.iter().any(|c| c >> 24 != 0)), "tick {t}: the ring drawn"); }
    }
    eprintln!("stomper #{b}: ring from tick {launched:?}, fully live {lit} ticks");
    assert!(launched.is_some() && lit >= 120, "the ring live {lit} ticks");
}
