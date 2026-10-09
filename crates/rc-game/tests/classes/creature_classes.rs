//! The creature-layer class units (docs/plan/creatures.md §9, gaps.md G-ENM-009): each unit resolves to its port on
//! its levels and runs headless on one of them (movement, its attack on Ratchet, a weapon hit, the side effects its
//! coverage table marks ported). Skipped when `extracted/` is absent.
//!
//! `cargo xtask test-job --test classes --filter creature_classes:: --nocapture` prints the per-unit survey.

use rc_formats::level_overlay::LevelOverlay;
use rc_formats::moby_anim::{parse_sequences, MobyAnimClass};
use rc_formats::{collision, gadget, gameplay, moby_spawn};
use rc_game::hero::Hero;
use rc_game::moby_runtime::{mode, MobyId, MobyTable};
use rc_game::moby_update::classes::units::{self, hop_gunner as hg, horny_toad as vc};
use rc_game::moby_update::classes::{ClassUpdate, LevelPorts};
use rc_game::moby_update::creature::react;
use rc_game::moby_update::scheduler::{class_info, load_level_mobys, Scheduler};
use rc_game::moby_update::services::{pvar as p, HitTemplate, World};
use rc_game::moby_update::{ClassTable, Services};
use rc_game::ps2v::Pf;
use rc_game::rng::Rng;
use std::collections::HashMap;
use std::sync::Arc;

fn overlay(level: u32) -> Option<Arc<LevelOverlay>> { crate::common::overlay(level) }

fn ports(level: u32) -> Option<LevelPorts> { crate::common::ports(level, &[]) }

fn unit(name: &str) -> ClassUpdate {
    let i = units::PORTS.iter().position(|u| u.unit == name).unwrap();
    ClassUpdate::Unit(i as u16)
}

/// (unit, class, the levels whose table runs it, created instances on them).
const EXPECTED: &[(&str, i16, &[u32], usize)] = &[("U25", 749, &[0, 18], 106), ("U287", 1023, &[8, 9], 40), ("U287 shot", 1292, &[8, 9], 0)];

/// Every unit's classes resolve to the unit on its levels (and to nothing else), with the census's instance counts.
#[test]
fn units_resolve_on_their_levels() {
    let Some(_) = overlay(1) else { eprintln!("skipped: no extracted/"); return };
    let mut created: HashMap<(&str, i16), usize> = HashMap::new();
    for level in 0..19u32 {
        let p = ports(level).unwrap();
        let gp = rc_formats::test_data::gameplay(level).unwrap();
        let inst = gameplay::parse_moby_instances(&gp).unwrap();
        let tests = moby_spawn::loader_spawns(&inst, &mut moby_spawn::SpawnSave::default());
        for &(name, oc, levels, _) in EXPECTED {
            let u = unit(name);
            if levels.contains(&level) {
                assert_eq!(p.get(oc), Some(u), "level {level:02} class {oc}");
                assert!(p.needs_joint_lists(oc) || units::PORTS.iter().all(|x| x.unit != name || x.joints.is_empty()));
                let n = inst.iter().zip(&tests).filter(|(m, t)| m.o_class as i16 == oc && t.spawn).count();
                *created.entry((name, oc)).or_default() += n;
            } else if p.in_table(oc) {
                assert_ne!(p.get(oc), Some(u), "level {level:02} class {oc}");
            }
        }
    }
    eprintln!("created instances now ported: {created:?}");
    for &(name, oc, _, n) in EXPECTED { assert_eq!(created[&(name, oc)], n, "{name} class {oc}"); }
}

/// The reaction tables reversed on other levels are found by code identity on every level that has them.
#[test]
fn reaction_tables_resolve() {
    let Some(ov1) = overlay(1) else { eprintln!("skipped"); return };
    for level in 0..19u32 {
        let t = react::tables_from_overlays(&overlay(level).unwrap(), &ov1, &overlay);
        let want = [0, 18].contains(&level);
        assert_eq!(t.get(&749) == Some(&react::Table::Veldin749), want, "level {level:02}: {t:?}");
        // The level-01 tables still resolve as before.
        if level == 1 { assert_eq!(t.get(&577), Some(&react::Table::Critter)); }
    }
}

// ---------------------------------------------------------------------------------------------------
// Headless levels (the path_classes harness, with Ratchet's moby placed and the reaction tables)

#[derive(Default)]
pub(crate) struct RecSink {
    pub(crate) slots: Vec<MobyId>,
}

impl rc_game::moby_update::services::SoundSink for RecSink {
    fn play_class_sound(&mut self, ev: &rc_game::moby_update::services::SoundEvent, _rng: &mut Rng) -> i32 {
        self.slots.push(ev.moby);
        self.slots.len() as i32 - 1
    }
    fn alive(&self, slot: i32, moby: MobyId) -> bool { usize::try_from(slot).ok().and_then(|s| self.slots.get(s)) == Some(&moby) }
}

pub(crate) struct Lv {
    pub(crate) sink: RecSink,
    pub(crate) table: MobyTable,
    pub(crate) classes: ClassTable,
    pub(crate) svc: Services,
    pub(crate) sched: Scheduler,
    pub(crate) rng: Rng,
    pub(crate) mesh: collision::Collision,
    pub(crate) counter: u64,
    pub(crate) missions: rc_game::moby_update::services::LevelMissions,
    pub(crate) hero_idx: MobyId,
    pub(crate) particles: rc_game::particles::Particles,
}

pub(crate) fn load(level: u32) -> Option<Lv> {
    let core = rc_formats::test_data::core(level)?;
    let gp = rc_formats::test_data::gameplay(level)?;
    let ports = ports(level)?;
    let mesh = collision::parse_collision(&core.core, &core.data).unwrap();
    let instances = gameplay::parse_moby_instances(&gp).unwrap();
    let tests = moby_spawn::loader_spawns(&instances, &mut moby_spawn::SpawnSave::default());
    let spawned: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
    let pvars = gameplay::parse_pvars_spawned(&gp, &spawned).unwrap();
    let rd = |o: usize| i32::from_le_bytes(gp[o..o + 4].try_into().unwrap());
    let spawnable = rd(rd(0x44) as usize + 4).max(1) as usize;
    let mut classes = ClassTable::default();
    let mut joints = HashMap::new();
    for (slot, e) in core.core.moby_classes.iter().enumerate() {
        let oc = e.o_class as i16;
        let parsed = core.block(&format!("moby_class/{:04}", e.o_class)).and_then(|b| rc_formats::moby::parse_moby_class(b).ok().map(|c| (b, c)));
        if let Some((blob, c)) = parsed {
            let anim = MobyAnimClass::new(&c, parse_sequences(blob, &c).unwrap_or_default());
            let mut info = class_info(&c, slot as u8, ports.update_fn(oc));
            info.seq0 = anim.sequence(0).map(|q| rc_game::moby_runtime::Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
            if ports.needs_joint_lists(oc) {
                joints.insert(oc, (0..16).map_while(|l| gadget::joint_list(blob, &c.header, l).ok().map(|(a, _)| a)).collect());
            }
            classes.classes.insert(oc, (info, Some(anim)));
        } else {
            let info = rc_game::moby_runtime::ClassInfo { slot: slot as u8, no_header: true, update_fn: ports.update_fn(oc), ..Default::default() };
            classes.classes.entry(oc).or_insert((info, None));
        }
    }
    // Ratchet's class (not in the level's class list: the engine loads it with the hero) with his sequences, so his
    // moby's collision is posed (standing, sequence 0) as the hero update leaves it.
    if let Some(blob) = rc_formats::test_data::core_block(level, "moby_class/0000") {
        let c = rc_formats::moby::parse_moby_class(&blob).unwrap();
        let seqs: Vec<Option<rc_formats::moby_anim::MobySequence>> = (0..256).map(|i| rc_formats::test_data::core_block(level, &format!("ratchet_seq/{i:03}")).and_then(|b| rc_formats::moby_anim::parse_sequence(&b, 0).ok())).collect();
        let anim = MobyAnimClass::new(&c, seqs);
        let e = classes.classes.entry(0).or_insert((rc_game::moby_runtime::ClassInfo::default(), None));
        e.1 = Some(anim);
    }
    let statics = load_level_mobys(&instances, &mut classes, &pvars, &tests);
    let mut table = MobyTable::new(statics.mobys.clone(), spawnable);
    let hero_idx = table.mobys.iter().position(|m| m.o_class == 0)?;
    table.mobys[hero_idx].mode |= mode::NO_UPDATE;
    let mut svc = Services::new();
    svc.level = level;
    svc.set_splines(&gameplay::parse_splines(&gp).unwrap());
    svc.groups = statics.groups(&gp);
    svc.set_moby_collision(rc_formats::moby_collision::parse_level(&core.core, &core.data).unwrap());
    svc.set_volumes(rc_formats::volumes::parse_volumes(&gp).unwrap());
    svc.joint_lists = joints;
    svc.creatures.react.tables = react::tables_from_overlays(&*overlay(level)?, &*overlay(1)?, &overlay);
    svc.build_grid(&mut table);
    Some(Lv { sink: RecSink::default(), table, classes, svc, sched: Scheduler::new(), rng: Rng::new(), mesh, counter: 0, missions: rc_game::moby_update::services::LevelMissions::fresh_load(level, [0xff; 16]), hero_idx, particles: rc_game::particles::Particles::new(None, Vec::new()) })
}

impl Lv {
    pub(crate) fn world<'a>(&'a mut self, hero: &'a Hero) -> World<'a> {
        let mut w = World::new(&mut self.table, hero, &mut self.rng, &self.classes, &mut self.svc, self.counter);
        w.coll = Some(&self.mesh);
        w.sound = Some(&mut self.sink);
        w.camera = hero.pos;
        w.missions = &self.missions;
        w.particles = Some(&mut self.particles);
        w
    }
    pub(crate) fn load_pass(&mut self, hero: &Hero) {
        self.place_hero(hero);
        let mut sched = std::mem::take(&mut self.sched);
        { let mut w = self.world(hero); sched.load_pass(&mut w); }
        self.sched = sched;
    }
    /// Ratchet's moby follows the hero block (the hero update's job in the engine).
    fn place_hero(&mut self, hero: &Hero) {
        let h = &mut self.table.mobys[self.hero_idx];
        h.position = hero.pos.map(|x| f32::from_bits(x.0));
        h.position[3] = 1.0;
        self.svc.build_matrix_in(&mut self.table, &self.classes, self.hero_idx);
    }
    pub(crate) fn tick(&mut self, hero: &Hero) {
        self.counter += 1;
        let c = self.counter;
        self.place_hero(hero);
        self.table.free_slot_pass(c);
        let mut sched = std::mem::take(&mut self.sched);
        { let mut w = self.world(hero); sched.tick(&mut w); }
        self.sched = sched;
        // `start_scene` stores game mode 2 at once (DialogStreamStart); the harness plays no scenes, so a requested
        // scene "never starts" and mode 2 is reset after the tick, as the engine does for a scene request it cannot
        // play (crate::scene_render's mode-2 reset). Without it a level whose arrival scene fires on tick 0 (Gemlik)
        // would stay in scene mode and its classes would only run their mode-2 branches.
        if self.svc.game_mode == 2 && self.svc.cinematic.requests.iter().any(|r| matches!(r, rc_game::cinematic::EngineRequest::StartScene { .. })) {
            self.svc.game_mode = 0;
        }
    }
    pub(crate) fn of_class(&self, oc: i16) -> Vec<MobyId> { self.table.mobys.iter().enumerate().filter(|(_, m)| m.o_class == oc && m.state < 0x80).map(|(i, _)| i).collect() }
    /// The hit records the log holds for `target`.
    pub(crate) fn hits_on(&self, target: MobyId) -> Vec<rc_game::moby_update::services::HitRecord> { self.svc.hits.records.iter().filter(|r| r.target == target && r.attacker.is_some()).copied().collect() }
    pub(crate) fn hit(&mut self, hero: &Hero, id: MobyId, t: &HitTemplate) {
        let mut w = self.world(hero);
        w.deliver_hit(id, t);
    }
}

pub(crate) fn hero_at(p: [f32; 3]) -> Hero {
    let mut h = Hero::new();
    h.pos = rc_game::hero::physics::v4(p[0], p[1], p[2]);
    h.body_point = rc_game::hero::physics::v4(p[0], p[1], p[2] + 0.7);
    h.shadow_point = h.pos;
    h
}

/// The wrench's hit (`0x2be1c0`: flags 0x10000, type 0 / 1, class 0x47, damage 1, the exact push along `dir`).
pub(crate) fn wrench(hero: MobyId, dir: [f32; 2]) -> HitTemplate {
    HitTemplate { dir: [Pf::f(dir[0]), Pf::f(dir[1]), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(hero), flags: 0x1_0000, b18: 0, b19: 1, h1a: 0x47, damage: Pf::ONE, w20: 1 }
}

/// The pvars of the created instances of `oc` on `level`, as words (survey).
#[test]
#[ignore = "survey: prints, asserts nothing; run with --ignored --nocapture"]
fn survey_pvars() {
    for (level, oc) in [(0u32, 749i16), (18, 749), (8, 1023), (4, 340), (8, 333), (4, 217), (3, 578)] {
        let Some(gp) = rc_formats::test_data::gameplay(level) else { eprintln!("skipped: no extracted/"); return };
        let inst = gameplay::parse_moby_instances(&gp).unwrap();
        let tests = moby_spawn::loader_spawns(&inst, &mut moby_spawn::SpawnSave::default());
        let spawned: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
        let pvars = gameplay::parse_pvars_spawned(&gp, &spawned).unwrap();
        let mut n = 0;
        for (i, m) in inst.iter().enumerate() {
            if m.o_class as i16 != oc || !spawned[i] { continue; }
            n += 1;
            if n > 3 { continue; }
            let p = usize::try_from(m.pvar_index).ok().and_then(|k| pvars.get(k)).and_then(|b| b.as_deref()).unwrap_or(&[]);
            eprintln!("L{level:02} class {oc} inst {i} pos {:?} rot {:?} scale {} upd {} group? pvars {} bytes", m.position, m.rotation, m.scale, m.update_distance, p.len());
            for (k, c) in p.chunks(16).enumerate() {
                let w: Vec<String> = c.chunks(4).map(|b| { let u = u32::from_le_bytes([b[0], b[1], b.get(2).copied().unwrap_or(0), b.get(3).copied().unwrap_or(0)]); let f = f32::from_bits(u); if f.is_finite() && f.abs() > 1e-4 && f.abs() < 1e6 { format!("{u:08x}({f})") } else { format!("{u:08x}") } }).collect();
                eprintln!("  +{:03x}: {}", k * 16, w.join(" "));
            }
        }
        eprintln!("L{level:02} class {oc}: {n} created");
    }
}

/// The class-table entries (update, reaction table and its six slots) of the unit classes (survey).
#[test]
#[ignore = "survey: prints, asserts nothing; run with --ignored --nocapture"]
fn survey_tables() {
    for (level, oc) in [(0u32, 749i16), (18, 749), (8, 1023), (9, 1023), (8, 1292), (8, 1025), (9, 1292), (9, 1025), (4, 340), (8, 333), (4, 217), (3, 578)] {
        let Ok(b) = std::fs::read(rc_formats::test_data::level_dir(level).join("overlay.bin")) else { eprintln!("skipped"); return };
        let ov = rc_formats::level_overlay::LevelOverlay::parse(&b).unwrap();
        for e in ov.vtbl().iter().filter(|e| e.o_class as i16 == oc) {
            let slots: Vec<String> = (0..6).map(|k| format!("{:#x}", ov.u32(e.w8 + 4 * k).unwrap_or(0))).collect();
            eprintln!("L{level:02} class {oc}: update {:#x} table {:#x} slots {}", e.update, e.w8, slots.join(" "));
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// U25: the Veldin horny toads 749 (levels 00, 18)

/// A headless run: the per-tick (state, position, sequence) rows of one moby, the level after, the hero.
type Run = (Vec<(u8, [f32; 3], u8)>, Lv, Hero);

/// Horny toad #143 (level 00, area path 0) and a point in front of it.
const V143: MobyId = 143;

fn critter_state(lv: &Lv, id: MobyId) -> (u8, [f32; 3], u8) {
    let m = &lv.table.mobys[id];
    (m.state, [m.position[0], m.position[1], m.position[2]], m.anim.seq_b)
}

#[test]
fn horny_toads_wander_around_home_on_the_ground() {
    for level in [0u32, 18] {
        let Some(mut lv) = load(level) else { eprintln!("skipped"); return };
        let ids = lv.of_class(749);
        assert_eq!(ids.len(), if level == 0 { 16 } else { 90 });
        // Ratchet 30 away from the first group: within the update distance, outside the search range (24).
        let a = lv.table.mobys[ids[0]].position;
        let hero = hero_at([a[0] + 30.0, a[1], a[2]]);
        lv.load_pass(&hero);
        let group: Vec<MobyId> = ids.iter().copied().filter(|&i| p::i32(&lv.table.mobys[i].pvars, vc::pv::AREA) == p::i32(&lv.table.mobys[ids[0]].pvars, vc::pv::AREA)).collect();
        let start: Vec<[f32; 4]> = group.iter().map(|&i| lv.table.mobys[i].position).collect();
        for _ in 0..600 { lv.tick(&hero); }
        let mut moved = 0;
        let want_scale = units::class_scale(&lv.world(&hero), 749) * vc::k::SCALE;
        for (k, &i) in group.iter().enumerate() {
            let m = lv.table.mobys[i].clone();
            assert_eq!(m.state, vc::st::WANDER, "#{i}");
            assert_eq!(m.anim.seq_b, 4, "#{i} walks (sequence 4)");
            assert!((0.92..=1.08).contains(&m.anim.speed), "#{i} anim speed {}", m.anim.speed);
            assert!((m.scale - want_scale).abs() < 1e-6);
            let home = p::v4f(&m.pvars, vc::pv::WANDER);
            let d = ((m.position[0] - home[0]).powi(2) + (m.position[1] - home[1]).powi(2)).sqrt();
            let leash = p::ff(&m.pvars, vc::pv::WANDER + 0x1c);
            assert!(leash == 1.5 || leash == 0.75);
            assert!(d < leash + 2.5, "#{i} strayed {d} from home (leash {leash})");
            let w = lv.world(&hero);
            let g = rc_game::moby_update::creature::ground::ground(&w, m.position, 0.5, 0);
            assert!(g.hit && (m.position[2] - g.z).abs() < 0.05, "#{i} off the ground: z {} ground {}", m.position[2], g.z);
            if (m.position[0] - start[k][0]).abs() + (m.position[1] - start[k][1]).abs() > 0.1 { moved += 1; }
        }
        eprintln!("L{level:02}: {} critters in area {}, {moved} moved in 10 s", group.len(), p::i32(&lv.table.mobys[ids[0]].pvars, vc::pv::AREA));
        assert!(moved * 2 >= group.len(), "most of them wander");
        let snd: Vec<(i32, u32)> = lv.svc.sounds.iter().filter(|s| lv.table.mobys[s.moby].o_class == 749).map(|s| (s.index, s.flags)).collect();
        eprintln!("L{level:02}: wander sounds (the sequences' triggers) {:?}", &snd[..snd.len().min(8)]);
    }
}

/// Runs critter #143 (level 00) with Ratchet 2.5 in front of it; returns the per-tick rows and the level.
fn bite_run() -> Option<Run> {
    let mut lv = load(0)?;
    let a = lv.table.mobys[V143].position;
    let hero = hero_at([a[0] - 2.5, a[1], a[2]]);
    lv.load_pass(&hero);
    let mut rows = Vec::new();
    for _ in 0..240 {
        lv.tick(&hero);
        rows.push(critter_state(&lv, V143));
    }
    Some((rows, lv, hero))
}

#[test]
fn horny_toad_goes_for_ratchet_and_bites() {
    let Some((rows, lv, _)) = bite_run() else { eprintln!("skipped"); return };
    let states: Vec<u8> = rows.iter().map(|r| r.0).collect();
    let first = |s: u8| states.iter().position(|&x| x == s);
    eprintln!("#143 states: go {:?}, bite {:?}", first(vc::st::GO), first(vc::st::BITE));
    assert!(first(vc::st::GO).is_some() && first(vc::st::BITE).is_some(), "{states:?}");
    assert!(first(vc::st::GO) < first(vc::st::BITE));
    let m = &lv.table.mobys[V143];
    assert_eq!(p::i32(&m.pvars, vc::pv::TGT_KIND), 0, "Ratchet is the target");
    assert_eq!(p::i32(&m.pvars, vc::pv::TGT_MOBY), lv.hero_idx as i32 + 1);
    // The bite: a hit record for Ratchet's moby from the critter (flags 1, damage 1, type 0 / 1, its class, the exact
    // push along its facing with z 1).
    let bites = lv.hits_on(lv.hero_idx);
    eprintln!("#143 bit Ratchet {} times: {:?}", bites.len(), bites.first());
    let b = bites.iter().find(|r| r.attacker == Some(V143)).expect("no bite on Ratchet's moby");
    assert_eq!((b.flags, b.damage, b.b28, b.b29, b.h2a), (1, Pf::ONE, 0, 1, 749));
    assert_eq!(b.dir[2], Pf::ONE);
    assert_eq!(b.dir[3], Pf::b(0x45af_df66));
}

#[test]
fn horny_toad_bite_run_is_deterministic() {
    let (Some((a, ..)), Some((b, ..))) = (bite_run(), bite_run()) else { eprintln!("skipped"); return };
    assert_eq!(a, b);
}

#[test]
fn a_wrench_hit_kills_the_horny_toad() {
    let Some(mut lv) = load(0) else { eprintln!("skipped"); return };
    let a = lv.table.mobys[V143].position;
    let hero = hero_at([a[0] - 20.0, a[1], a[2]]);
    lv.load_pass(&hero);
    for _ in 0..3 { lv.tick(&hero); }
    let bolts0: usize = (13..=16).map(|c| lv.of_class(c).len()).sum();
    let parts0 = lv.svc.fx.part_spawns.get(&11).copied().unwrap_or(0);
    let lights0 = lv.svc.creatures.lights;
    let t = wrench(lv.hero_idx, [1.0, 0.0]);
    lv.hit(&hero, V143, &t);
    lv.tick(&hero);
    let m = &lv.table.mobys[V143];
    assert_eq!(m.state, vc::st::DYING, "the death flight");
    assert_eq!(m.mode & mode::TARGETABLE, 0, "untargetable");
    assert_eq!(p::ff(&m.pvars, vc::pv::D), 0.0, "health 1 − 1");
    assert!([6, 7].contains(&m.anim.seq_b), "flight sequence {}", m.anim.seq_b);
    assert!(lv.svc.save.death.contains(&(0, m.spawn_id)), "SetDeathBits");
    let bolts: usize = (13..=16).map(|c| lv.of_class(c).len()).sum();
    eprintln!("#143 b4 {}: {} bolts dropped", m.b4, bolts - bolts0);
    if m.b4 > 0 { assert!(bolts > bolts0, "the bolts"); }
    let x0 = m.position[0];
    let mut end = None;
    for k in 0..400 {
        lv.tick(&hero);
        if lv.table.mobys[V143].state >= 0xfd { end = Some(k); break; }
    }
    let k = end.expect("the flight never ended");
    let snd = lv.svc.sounds.iter().filter(|s| s.moby == V143).map(|s| s.index).collect::<Vec<_>>();
    let parts = lv.svc.fx.part_spawns.get(&11).copied().unwrap_or(0) - parts0;
    let lights = lv.svc.creatures.lights;
    eprintln!("#143 flew {k} ticks from x {x0}; sounds {snd:?}; {parts} type-11 sparks; lights {lights0} → {lights}");
    assert!(snd.contains(&7), "class sound 7 at the end");
    assert_eq!(parts, 3, "the small explosion's three spark pairs");
    assert_eq!(lights, lights0 + 1, "the explosion light 639 (radius 10)");
}

#[test]
fn the_taunter_lures_the_horny_toad_into_its_chase() {
    let Some(mut lv) = load(0) else { eprintln!("skipped"); return };
    let a = lv.table.mobys[V143].position;
    let hero = hero_at([a[0] + 30.0, a[1], a[2]]);
    lv.load_pass(&hero);
    lv.tick(&hero);
    assert_eq!(lv.table.mobys[V143].state, vc::st::WANDER);
    // The Taunter writes its moby into the damage record's +0x18 (react::lure).
    let lure = { let w = lv.world(&hero); react::lure(&w, V143).unwrap() };
    assert_eq!(lure, vc::pv::LURE);
    p::set_i32(&mut lv.table.mobys[V143].pvars, lure, 1);
    lv.tick(&hero);
    let m = &lv.table.mobys[V143];
    let t = p::i16(&m.pvars, vc::pv::ALERT_T);
    assert!((179..=240).contains(&t), "alert timer {t}");
    assert_eq!(p::i32(&m.pvars, lure), 0, "the lure is taken");
    let s = m.state;
    lv.tick(&hero);
    assert!(s == vc::st::CHASE || lv.table.mobys[V143].state == vc::st::CHASE, "the chase");
}

#[test]
fn the_suck_cannon_takes_the_horny_toad() {
    let Some(mut lv) = load(0) else { eprintln!("skipped"); return };
    let a = lv.table.mobys[V143].position;
    let hero = hero_at([a[0] + 30.0, a[1], a[2]]);
    lv.load_pass(&hero);
    lv.tick(&hero);
    {
        let mut w = lv.world(&hero);
        assert_eq!(react::table(&w, V143), Some(react::Table::Veldin749));
        assert_eq!(react::wrappers(&w, V143), Some(react::VELDIN_749));
        assert_eq!(react::record(&w, V143), Some(vc::pv::SUCK));
        // A pull in progress (record state 1), then slot +0x0c (let go): held state 9, the record falls (7); the class's
        // own update runs the carried update 0x305260 until it lands.
        rc_game::moby_update::creature::set_pi16(&mut w, V143, vc::pv::SUCK + react::rec::STATE, 1);
        react::slot_let_go(&mut w, V143);
        assert_eq!(w.m(V143).state, vc::st::HELD);
    }
    let mut back = None;
    for k in 0..600 {
        lv.tick(&hero);
        if lv.table.mobys[V143].state != vc::st::HELD { back = Some(k); break; }
    }
    let k = back.expect("never let go");
    let m = &lv.table.mobys[V143];
    eprintln!("#143 let go: back in state {} after {k} ticks", m.state);
    assert_eq!(m.state, vc::st::WANDER);
    assert_eq!(p::i16(&m.pvars, vc::pv::SUCK + react::rec::STATE), 0);
}

/// A class with a reaction table reversed on its level: the table found, a pull let go puts it in its held state, its
/// own update runs the carried update until it lands and leaves `back`.
fn sucked_and_let_go(level: u32, oc: i16, table: react::Table, record: usize, held: u8, back: u8) {
    let Some(mut lv) = load(level) else { eprintln!("skipped"); return };
    let b = lv.of_class(oc)[0];
    let a = lv.table.mobys[b].position;
    let hero = hero_at([a[0] - 30.0, a[1], a[2]]);
    lv.load_pass(&hero);
    lv.tick(&hero);
    {
        let mut w = lv.world(&hero);
        assert_eq!(react::table(&w, b), Some(table));
        assert_eq!(react::record(&w, b), Some(record));
        rc_game::moby_update::creature::set_pi16(&mut w, b, record + react::rec::STATE, 1);
        react::slot_let_go(&mut w, b);
        assert_eq!(w.m(b).state, held);
    }
    let k = (0..600).find(|_| { lv.tick(&hero); lv.table.mobys[b].state != held }).expect("never let go");
    let m = &lv.table.mobys[b];
    eprintln!("{oc} #{b} let go: back in state {} after {k} ticks", m.state);
    assert_eq!(m.state, back);
    assert_eq!(p::i16(&m.pvars, record + react::rec::STATE), 0);
}

/// Eudora's brawler 340 (level04 0x1dd11c, [`react::EUDORA_340`]): held in 9, the record at +0x180.
#[test]
fn the_suck_cannon_takes_eudoras_brawler() { sucked_and_let_go(4, 340, react::Table::Eudora340, 0x180, 9, 0); }

/// The Fleet's crew 1382 (level17 0x1e7854, [`react::FLEET_1382`]): held in 0xc, the record at +0x60.
#[test]
fn the_suck_cannon_takes_the_fleets_crew() { sucked_and_let_go(17, 1382, react::Table::Fleet1382, 0x60, 0xc, 0); }

/// Veldin's hopper 1906 (level18 0x1f34c8, [`react::VELDIN_1906`]): held in 7, the record at +0xd0.
#[test]
fn the_suck_cannon_takes_veldins_hopper() { sucked_and_let_go(18, 1906, react::Table::Veldin1906, 0xd0, 7, 2); }

#[test]
fn a_knocked_horny_toad_presses_its_floor_switch() {
    let Some(mut lv) = load(18) else { eprintln!("skipped"); return };
    let c = 388;
    let sw = p::i32(&lv.table.mobys[c].pvars, vc::pv::SWITCH) as MobyId;
    assert_eq!(lv.table.mobys[sw].o_class, 830);
    let at = lv.table.mobys[sw].position;
    let hero = hero_at([at[0] + 30.0, at[1], at[2]]);
    lv.load_pass(&hero);
    lv.tick(&hero);
    assert_eq!(lv.table.mobys[sw].state, 1, "the switch is armed");
    // Knocked (state 0xb) onto the plate: pressed at the tail of its update, and forgotten.
    lv.table.mobys[c].state = vc::st::KNOCKED;
    lv.table.mobys[c].position = [at[0] + 0.3, at[1], at[2] + 0.5, 1.0];
    lv.tick(&hero);
    assert_eq!(lv.table.mobys[sw].state, 2, "pressed");
    assert_eq!(lv.table.mobys[sw].glow, rc_game::moby_update::classes::floor_switch::PRESSED_GLOW);
    assert_eq!(p::i32(&lv.table.mobys[c].pvars, vc::pv::SWITCH), -1);
    assert!(lv.svc.sounds.iter().any(|s| s.moby == sw && s.index == 0), "the switch's sound");
    assert!(lv.svc.save.killed.contains_key(&lv.table.mobys[sw].spawn_id));
}

// ---------------------------------------------------------------------------------------------------
// U287: the hopping gunners 1023 (levels 08, 09) and their shots 1292

/// Gunner #806 (level 08): Ratchet `d` in front of its hop path's first point; the per-tick (state, pos, seq) rows.
fn gunner_run(d: f32, ticks: usize) -> Option<Run> {
    let mut lv = load(8)?;
    let g = lv.of_class(1023)[0];
    let hop = p::i32(&lv.table.mobys[g].pvars, hg::pv::HOP) as usize;
    let a = lv.svc.splines[hop][0].map(f32::from_bits);
    let hero = hero_at([a[0] + d, a[1], a[2]]);
    lv.load_pass(&hero);
    let mut rows = Vec::new();
    for _ in 0..ticks {
        lv.tick(&hero);
        rows.push(critter_state(&lv, g));
    }
    Some((rows, lv, hero))
}

#[test]
fn gunner_hops_between_its_points_and_fires_bursts_at_ratchet() {
    let Some((rows, mut lv, hero)) = gunner_run(8.0, 600) else { eprintln!("skipped"); return };
    let g = lv.of_class(1023)[0];
    let hop = p::i32(&lv.table.mobys[g].pvars, hg::pv::HOP) as usize;
    let pts: Vec<[f32; 4]> = lv.svc.splines[hop].iter().map(|q| q.map(f32::from_bits)).collect();
    assert_eq!(pts.len(), 2);
    let states: Vec<u8> = rows.iter().map(|r| r.0).collect();
    let hops = states.windows(2).filter(|w| w[0] != hg::st::HOP && w[1] == hg::st::HOP).count() + (states[0] == hg::st::HOP) as usize;
    let bursts = states.windows(2).filter(|w| w[0] != hg::st::FIRE && w[1] == hg::st::FIRE).count();
    eprintln!("gunner #{g}: {hops} hops, {bursts} bursts in 10 s");
    assert!(hops >= 3 && bursts >= 3, "{states:?}");
    // Each landing is on one of the two hop points; the arc rises up to 2 above the line between them.
    let top = rows.iter().filter(|r| r.0 == hg::st::HOP).map(|r| r.1[2]).fold(f32::MIN, f32::max);
    let base = pts[0][2].max(pts[1][2]);
    eprintln!("the arc's top {top} over the points' {base}");
    assert!(top > base + 1.0 && top <= base + 2.01, "arc top {top}");
    for r in rows.iter().filter(|r| r.0 == hg::st::FIRE) {
        let near = pts.iter().any(|q| ((r.1[0] - q[0]).powi(2) + (r.1[1] - q[1]).powi(2)).sqrt() < 0.6);
        assert!(near, "fires away from its hop points: {:?}", r.1);
    }
    // The gun rides joint 4 (class 0x401, rows kept).
    let guns = lv.of_class(hg::GUN_CLASS);
    let gun = (p::i32(&lv.table.mobys[g].pvars, hg::pv::GUN) - 1) as MobyId;
    assert!(guns.contains(&gun));
    let jp = lv.world(&hero).joint_point(g, 4);
    let gp = lv.table.mobys[gun].position;
    assert!((gp[0] - jp[0]).abs() < 0.5 && (gp[1] - jp[1]).abs() < 0.5, "gun at {gp:?}, joint 4 at {jp:?}");
    assert_ne!(lv.table.mobys[gun].mode & mode::KEEP_ROWS, 0);
    // The shots hit Ratchet: records for his moby with the shots' flags 0x10001, damage 1, pushing along their flight.
    let shot_hits: Vec<_> = lv.hits_on(lv.hero_idx).into_iter().filter(|h| h.flags == 0x1_0001).collect();
    eprintln!("{} shot hits on Ratchet; {} shots in flight", shot_hits.len(), lv.of_class(1292).len());
    assert!(!shot_hits.is_empty());
    assert!(shot_hits.iter().all(|h| h.damage == Pf::ONE));
    // A shot's own life: flying at 20 u/s (the burst's drop 1/n), a type-4 trail puff a tick, a small explosion at its end.
    let parts0 = lv.svc.fx.part_spawns.get(&4).copied().unwrap_or(0);
    let muzzle = lv.world(&hero).joint_point(g, 4);
    let hi = lv.hero_idx;
    let s = { let mut w = lv.world(&hero); hg::spawn_shot(&mut w, [0.0, 20.0 / 60.0, -20.0 / 60.0 / 2.0, 0.0], [muzzle[0], muzzle[1], muzzle[2] + 30.0, 1.0], g, Some(hi), 180, 3).unwrap() };
    let p0 = lv.table.mobys[s].position;
    lv.tick(&hero);
    let p1 = lv.table.mobys[s].position;
    assert!((p1[1] - p0[1] - 20.0 / 60.0).abs() < 1e-4, "{p0:?} → {p1:?}");
    assert_eq!(lv.table.mobys[s].o_class, 1292);
    assert!(lv.svc.fx.part_spawns.get(&4).copied().unwrap_or(0) > parts0, "the trail");
    p::set_i32(&mut lv.table.mobys[s].pvars, hg::shot::LIFE, 1);
    let lights0 = lv.svc.creatures.lights;
    lv.tick(&hero);
    lv.tick(&hero);
    assert!(lv.table.mobys[s].state >= 0xfd || lv.table.mobys[s].o_class != 1292, "the shot ended");
    assert!(lv.svc.creatures.lights > lights0, "its explosion light");
}

#[test]
fn gunner_run_is_deterministic() {
    let (Some((a, ..)), Some((b, ..))) = (gunner_run(8.0, 300), gunner_run(8.0, 300)) else { eprintln!("skipped"); return };
    assert_eq!(a, b);
}

#[test]
fn gunner_clubs_ratchet_up_close() {
    let Some((rows, lv, _)) = gunner_run(2.0, 300) else { eprintln!("skipped"); return };
    let g = lv.of_class(1023)[0];
    assert!(rows.iter().any(|r| r.0 == hg::st::CLUB), "never clubbed: {:?}", rows.iter().map(|r| r.0).collect::<Vec<_>>());
    let club = lv.hits_on(lv.hero_idx).into_iter().filter(|h| h.flags == 1 && h.attacker == Some(g)).count();
    eprintln!("gunner #{g}: {club} club hits on Ratchet");
    assert!(club > 0);
}

#[test]
fn the_wrench_knocks_the_gunner_back_then_kills_it() {
    let Some((_, mut lv, _)) = gunner_run(20.0, 30) else { eprintln!("skipped"); return };
    let g = lv.of_class(1023)[0];
    let hero = hero_at([1.0, 1.0, 1.0]);
    let at = lv.table.mobys[g].position;
    let hero = Hero { pos: rc_game::hero::physics::v4(at[0] + 20.0, at[1], at[2]), ..hero };
    let gun = (p::i32(&lv.table.mobys[g].pvars, hg::pv::GUN) - 1) as MobyId;
    let t = wrench(lv.hero_idx, [0.0, 1.0]);
    lv.hit(&hero, g, &t);
    lv.tick(&hero);
    let m = &lv.table.mobys[g];
    assert_eq!(m.state, hg::st::KNOCKED, "the first hit knocks back (reaction 3)");
    assert_eq!(p::ff(&m.pvars, hg::pv::D), 1.0);
    assert_eq!(m.anim.seq_b, 8);
    // Wait out the cooldown (kind 1: 15 ticks) and hit again: health 0 → the death flight.
    for _ in 0..40 { lv.tick(&hero); }
    let bolts0: usize = (13..=16).map(|c| lv.of_class(c).len()).sum();
    lv.hit(&hero, g, &t);
    lv.tick(&hero);
    let m = &lv.table.mobys[g];
    assert_eq!(m.state, hg::st::DYING, "the second hit kills");
    assert_eq!(m.mode & mode::TARGETABLE, 0);
    assert_eq!(m.anim.seq_b, 0xc);
    // `BoltBurst(m, 2, 3, 0, −1)`: flags 0 returns at once in the game (no bolts); the bolts come with SetDeathBits.
    let bolts: usize = (13..=16).map(|c| lv.of_class(c).len()).sum();
    assert_eq!(bolts, bolts0, "BoltBurst with flags 0 drops nothing");
    let b4 = lv.table.mobys[g].b4;
    let pieces0: usize = hg::PIECES.iter().map(|&c| lv.of_class(c).len()).sum();
    let mut end = None;
    for k in 0..400 {
        lv.tick(&hero);
        if lv.table.mobys[g].state >= 0xfd { end = Some(k); break; }
    }
    eprintln!("gunner #{g} died after {end:?} ticks of flight");
    assert!(end.is_some());
    let pieces: usize = hg::PIECES.iter().map(|&c| lv.of_class(c).len()).sum();
    assert_eq!(pieces, pieces0 + 3, "the three pieces 1692–1694");
    assert!(lv.table.mobys[gun].state >= 0xfd || lv.table.mobys[gun].o_class != hg::GUN_CLASS, "the gun goes with it");
    assert!(lv.svc.sounds.iter().any(|s| s.moby == g && s.index == 0), "the death explosion's class sound 0");
    assert!(lv.svc.save.death.contains(&(8, lv.table.mobys[g].spawn_id)), "SetDeathBits");
    let bolts: usize = (13..=16).map(|c| lv.of_class(c).len()).sum();
    eprintln!("gunner #{g}: +0xb4 = {b4}, {} bolts dropped at the end", bolts - bolts0);
    if b4 > 0 { assert!(bolts > bolts0, "SetDeathBits's bolts"); }
}

#[test]
fn the_morph_ray_keep_byte_and_the_taunter_act_on_the_gunner() {
    let Some((_, mut lv, hero)) = gunner_run(20.0, 30) else { eprintln!("skipped"); return };
    let g = lv.of_class(1023)[0];
    // The lure: 6 more search range for the alert.
    let base = p::ff(&lv.table.mobys[g].pvars, hg::pv::BASE_RANGE);
    p::set_i32(&mut lv.table.mobys[g].pvars, hg::pv::LURE, 1);
    lv.tick(&hero);
    let m = &lv.table.mobys[g];
    assert_eq!(p::ff(&m.pvars, hg::pv::RANGE), base + 6.0);
    assert!((170..=240).contains(&p::i16(&m.pvars, hg::pv::ALERT_T)));
    // The Morph-o-Ray's keep byte 2: deleted with its gun.
    let gun = (p::i32(&lv.table.mobys[g].pvars, hg::pv::GUN) - 1) as MobyId;
    lv.table.mobys[g].pvars[hg::pv::MORPH_KEEP] = 2;
    lv.tick(&hero);
    assert!(lv.table.mobys[g].state >= 0xfd);
    assert!(lv.table.mobys[gun].state >= 0xfd);
}



/// A Kerwan trooper 574 knocked by a flat push (the jump attack's shockwave: facing·2, z 0, damage 2, the exact
/// marker) lands and recovers: the flight's drag stops it horizontally, then gravity still brings it down
/// (`0x221460` keeps z when it zeroes xy; it used to hang in the air in state 0xa).
#[test]
fn trooper_lands_after_a_flat_push() {
    let Some(mut lv) = load(3) else { eprintln!("skipped: no extracted/"); return };
    let t = lv.of_class(574)[0];
    let p = lv.table.mobys[t].position;
    let hero = hero_at([p[0] - 2.0, p[1], p[2]]);
    lv.load_pass(&hero);
    for _ in 0..30 { lv.tick(&hero); }
    let h = lv.hero_idx;
    let tmpl = HitTemplate { dir: [Pf::f(2.0), Pf::ZERO, Pf::ZERO, Pf::b(0x45af_df66)], attacker: Some(h), flags: 0x1_0000, b18: 0, b19: 1, h1a: 0x47, damage: Pf::f(2.0), w20: 1 };
    lv.hit(&hero, t, &tmpl);
    lv.tick(&hero);
    assert_eq!(lv.table.mobys[t].state, 0xa, "knocked");
    let left = (0..200).find(|_| {
        lv.tick(&hero);
        lv.table.mobys[t].state != 0xa
    });
    assert!(left.is_some(), "still knocked after 200 ticks at {:?}", lv.table.mobys[t].position);
}
