//! Every level headless (docs/plan/level_generalisation.md, "Guards"): the level's mobys as the loader creates them
//! on a first visit, its class table through `LevelPorts` (the level's `lvl.vtbl` against the ports' reference
//! code), the scheduler's load pass, then N ticks of the whole game tick (pad → mobys → hero with the wrench and his
//! level's item definitions → particles → camera) with a scripted pad (walk, turn, jump, swing the wrench), with no
//! panic. Reports per level the ticks run, where Ratchet ended, and the classes placed with an update the port
//! does not have (instances per class). Skipped without `extracted/`.
//!
//! `RC_SMOKE_TICKS` (default 600) sets N for [`all_levels_load_spawn_and_tick`]; [`gemlik_three_thousand_ticks`]
//! runs 3,000 on Gemlik (13), the validation level.

use rc_formats::level_overlay::LevelOverlay;
use rc_formats::moby_anim::{parse_sequence, parse_sequences, MobyAnimClass, MobySequence};
use rc_formats::{collision, gadget, gameplay, moby_spawn};
use rc_game::hero::anim::RatchetAnim;
use rc_game::hero::items::{ItemClass, ItemData, ItemDef, HERO_LISTS};
use rc_game::hero::Hero;
use rc_game::moby_runtime::{mode, ClassInfo, MobyTable, Seq0Info};
use rc_game::moby_update::scheduler::{class_info, load_level_mobys, Scheduler};
use rc_game::moby_update::services::{SharedServices, World};
use rc_game::moby_update::{ClassTable, Services};
use rc_game::pad::{button, PadInput};
use rc_game::particles::Particles;
use rc_game::rng::Rng;
use rc_game::tick::{Game, GameOptions, TickHooks};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// The ports of the common-classes pass (605, 832, 604 / 1818 / 1633, 258, 830).
pub const COMMON_PASS: [rc_game::moby_update::classes::ClassUpdate; 7] = {
    use rc_game::moby_update::classes::ClassUpdate as U;
    [U::BuriedBolts, U::RcRange, U::MouseHouse, U::Mouse, U::MouseShot, U::ActivationZone, U::FloorSwitch]
};

fn overlay(level: u32) -> Option<Arc<LevelOverlay>> { crate::common::overlay(level) }

/// What a level's run did.
#[derive(Debug)]
pub struct Outcome {
    pub level: u32,
    pub ticks: u32,
    pub hero_start: [f32; 3],
    pub hero_end: [f32; 3],
    pub hero_state: i32,
    /// The mobys the loader created and those the scheduler runs a port for.
    pub statics: usize,
    pub ported_statics: usize,
    /// Placed (created) instances per class whose level update is not ported: class → (instances, update address).
    pub unported: BTreeMap<i16, (usize, u32)>,
    /// Placed instances per class that run a port (class → instances).
    pub ported: BTreeMap<i16, usize>,
    /// Of those, the instances of the common classes ported in the common-classes pass ([`COMMON_PASS`]): the
    /// drop in unported instances that pass made.
    pub common_pass: usize,
    /// Mobys the scheduler created during the run, and the moby sounds queued.
    pub spawned: usize,
    pub sounds: usize,
}

/// The pad: walk forward, turn, jump every 90 ticks, swing the wrench every 70 (an out-of-bounds fall ends the run).
fn input(t: u32) -> PadInput {
    let mut p = PadInput::neutral();
    let phase = (t / 240) % 4;
    let (x, y) = match phase { 0 => (0.0, -1.0), 1 => (0.7, -0.7), 2 => (0.0, -1.0), _ => (-0.7, -0.7) };
    if t % 600 < 520 { p = p.stick(x, y); }
    if t % 90 == 45 { p = p.press(button::CROSS); }
    if t % 70 == 20 { p = p.press(button::SQUARE); }
    p
}

struct CellHits<'a, 'b> {
    svc: &'a std::cell::RefCell<&'b mut Services>,
    classes: &'a ClassTable,
    coll: &'a collision::Collision,
}

impl rc_game::hero::items::HitSink for CellHits<'_, '_> {
    fn sphere(&mut self, table: &mut MobyTable, r: rc_game::ps2v::Pf, centre: rc_game::hero::physics::V4, flags: u32, ignore: Option<usize>, tmpl: &rc_game::moby_update::services::HitTemplate) -> Option<usize> {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::services::ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.sphere(table, r, centre, flags, ignore, tmpl)
    }
    fn line(&mut self, table: &mut MobyTable, a: rc_game::hero::physics::V4, b: rc_game::hero::physics::V4, flags: u32, ignore: Option<usize>, tmpl: &rc_game::moby_update::services::HitTemplate) -> Option<Option<usize>> {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::services::ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.line(table, a, b, flags, ignore, tmpl)
    }
    fn deliver(&mut self, table: &mut MobyTable, target: usize, tmpl: &rc_game::moby_update::services::HitTemplate) {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::services::deliver_hit_in(table, &mut s.hits, target, tmpl);
    }
    fn create_moby(&mut self, table: &mut MobyTable, o_class: i16, counter: u64) -> Option<usize> {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::classes::bomb::create_from_hero(table, &mut s, self.classes, o_class, counter)
    }
    fn delete_moby(&mut self, table: &mut MobyTable, id: usize, counter: u64) {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::classes::bomb::delete_from_hero(table, &mut s, id, counter);
    }
}

/// The level's item definitions (the overlay's table, found through `GiveItem`), gadget classes and Ratchet's
/// joint chains, as the engine builds them.
fn items(level: u32, core: &rc_formats::test_data::Core, ratchet_blob: &[u8], ratchet: &rc_formats::moby::MobyClass) -> ItemData {
    let root = rc_formats::test_data::root();
    let elf = std::fs::read(root.join("boot/SCUS_971.99")).expect("boot ELF");
    let ov_bytes = std::fs::read(rc_formats::test_data::level_dir(level).join("overlay.bin")).unwrap();
    let tables = rc_formats::save_game::ItemTables::load(&elf, &ov_bytes).unwrap_or_else(|e| panic!("level {level:02} item tables: {e}"));
    let sections = rc_formats::font::parse_overlay_sections(&ov_bytes).unwrap();
    let (n, sz) = (rc_formats::save_game::ITEM_COUNT, rc_formats::save_game::ITEM_DEF_SIZE);
    let raw = rc_formats::font::read_overlay(&sections, tables.item_defs_addr, n * sz).expect("item definitions");
    let w = |i: usize, o: usize| i32::from_le_bytes(raw[i * sz + o..i * sz + o + 4].try_into().unwrap());
    let defs = (0..n).map(|i| ItemDef { slot: w(i, 8), attach: w(i, 0xc), o_class: w(i, 0x10), b18: raw[i * sz + 0x18] }).collect();
    let hero_chains = HERO_LISTS.iter().map(|&l| gadget::joint_list(ratchet_blob, &ratchet.header, l).map(|(a, _)| a).unwrap_or_default()).collect();
    let classes = gadget::parse_gadget_classes(&core.core, &core.data)
        .unwrap()
        .iter()
        .map(|g| {
            let c = &g.moby.class;
            let chains = (0..16).map_while(|l| gadget::joint_list(&g.blob, &c.header, l).ok().map(|(a, _)| a)).collect();
            ItemClass { o_class: g.moby.o_class as i16, anim: MobyAnimClass::new(c, parse_sequences(&g.blob, c).unwrap_or_default()), scale: c.header.scale, chains }
        })
        .collect();
    ItemData { defs, hero_chains, classes }
}

/// Loads `level`, runs its load pass and `ticks` game ticks. None when the data is absent.
pub fn run_level(level: u32, ticks: u32) -> Option<Outcome> {
    let core = rc_formats::test_data::core(level)?;
    let gp = rc_formats::test_data::gameplay(level)?;
    let ports = crate::common::ports(level, &[])?;
    let settings = rc_formats::test_data::gameplay_section(level, "level_settings").expect("level settings");
    let mesh = collision::parse_collision(&core.core, &core.data).unwrap_or_else(|e| panic!("level {level:02} collision: {e}"));
    let instances = gameplay::parse_moby_instances(&gp).unwrap();
    let tests = moby_spawn::loader_spawns(&instances, &mut moby_spawn::SpawnSave::default());
    let spawned_ok: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
    let pvars = gameplay::parse_pvars_spawned(&gp, &spawned_ok).unwrap();
    let splines = gameplay::parse_splines(&gp).unwrap();
    let rd = |o: usize| i32::from_le_bytes(gp[o..o + 4].try_into().unwrap());
    let spawnable = rd(rd(0x44) as usize + 4).max(1) as usize;
    let death_z = f32::from_le_bytes(settings[0x28..0x2c].try_into().unwrap());
    // The class table (slot = the core's class list order), with the level's ports.
    let mut classes = ClassTable::default();
    let mut spheres = HashMap::new();
    let mut joints = HashMap::new();
    let mut gaits = HashMap::new();
    for (slot, e) in core.core.moby_classes.iter().enumerate() {
        let oc = e.o_class as i16;
        let parsed = core.block(&format!("moby_class/{:04}", e.o_class)).and_then(|b| rc_formats::moby::parse_moby_class(b).ok().map(|c| (b, c)));
        if let Some((blob, c)) = parsed {
            let anim = MobyAnimClass::new(&c, parse_sequences(blob, &c).unwrap_or_default());
            let mut info = class_info(&c, slot as u8, ports.update_fn(oc));
            info.seq0 = anim.sequence(0).map(|q| Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
            spheres.insert(oc, c.header.bsphere);
            if ports.get(oc).is_some_and(|u| u.needs_joint_lists()) {
                joints.insert(oc, (0..16).map_while(|l| gadget::joint_list(blob, &c.header, l).ok().map(|(a, _)| a)).collect());
                // The leg walker's gait records (the engine's `class_gaits`): without them every record's speed is 0
                // and the walker's anim speed becomes +inf, which `MobyAnimAdvance` never steps past.
                for (seq, rec) in rc_formats::moby_anim::gait_records(blob, &c) { gaits.insert((oc, seq), rec); }
            }
            classes.classes.insert(oc, (info, Some(anim)));
        } else {
            let info = ClassInfo { slot: slot as u8, no_header: true, update_fn: ports.update_fn(oc), ..Default::default() };
            classes.classes.entry(oc).or_insert((info, None));
        }
    }
    let coll_blobs = rc_formats::moby_collision::parse_level(&core.core, &core.data).unwrap();
    let ratchet_blob = core.block("moby_class/0000")?.to_vec();
    let rclass = rc_formats::moby::parse_moby_class(&ratchet_blob).unwrap();
    let seqs: Vec<Option<MobySequence>> = (0..256).map(|i| core.block(&format!("ratchet_seq/{i:03}")).and_then(|b| parse_sequence(b, 0).ok())).collect();
    let ratchet = MobyAnimClass::new(&rclass, seqs);
    let items = items(level, &core, &ratchet_blob, &rclass);

    let classes = Arc::new(classes);
    let mut ct = ClassTable { classes: classes.classes.clone() };
    let statics = load_level_mobys(&instances, &mut ct, &pvars, &tests);
    let hero_idx = statics.mobys.iter().position(|m| m.o_class == 0)?;
    let mut table = MobyTable::new(statics.mobys.clone(), spawnable);
    table.mobys[hero_idx].mode |= mode::NO_UPDATE;
    let mut game = Game::new(&mesh, table, hero_idx, GameOptions::default(), death_z);
    game.hero.idle.level = level as i32;
    game.item_data = Some(items);
    game.item_globals.wrench_flag = 1;
    let mut svc = Services::new();
    svc.level = level;
    svc.set_splines(&splines);
    svc.pvar_shared = gameplay::parse_pvar_shared_data(&gp).unwrap();
    svc.groups = statics.groups(&gp);
    svc.set_moby_collision(coll_blobs);
    svc.set_volumes(rc_formats::volumes::parse_volumes(&gp).unwrap());
    svc.joint_lists = joints;
    svc.gaits = gaits;
    svc.build_grid(&mut game.mobys);
    svc.creatures.class_spheres = spheres;
    let mut particles = Particles::new(None, Vec::new());
    let mut sched = Scheduler::new();
    {
        let hero: Hero = game.hero.clone();
        let mut w = World::new(&mut game.mobys, &hero, &mut game.rng, &*classes, &mut svc, 0);
        w.camera = game.camera.out.pos;
        w.coll = Some(&mesh);
        w.particles = Some(&mut particles);
        sched.load_pass(&mut w);
    }
    game.finish_load();
    let hero_start = game.hero.position();
    let n_static = game.mobys.first_dynamic;
    let mut anim = RatchetAnim::new(&ratchet);
    let svc_cell = std::cell::RefCell::new(&mut svc);
    let parts_cell = std::cell::RefCell::new(&mut particles);
    let mut spawned = 0;
    let mut seen = std::collections::HashSet::new();
    let mut ran = 0;
    for t in 0..ticks {
        let classes_ref: &ClassTable = &classes;
        let mut mobys = |table: &mut MobyTable, hero: &Hero, rng: &mut Rng, cam: &rc_game::follow_camera::CameraView, coll: &collision::Collision, counter: u64| {
            let mut s = svc_cell.borrow_mut();
            let mut p = parts_cell.borrow_mut();
            let mut w = World::new(table, hero, rng, classes_ref, &mut s, counter);
            w.camera = cam.pos;
            w.coll = Some(coll);
            w.particles = Some(&mut **p);
            sched.tick(&mut w);
        };
        let mut parts = |_: &Hero, _: &rc_game::follow_camera::CameraView, rng: &mut Rng, _: u64| { parts_cell.borrow_mut().update_parts(rng); };
        let mut world = SharedServices { svc: &svc_cell, classes: classes.clone() };
        let mut hooks = TickHooks { mobys: &mut mobys, particles: &mut parts, world: Some(&mut world) };
        game.hero.idle.counter = game.counter as i32;
        let mut hits = CellHits { svc: &svc_cell, classes: &classes, coll: &mesh };
        let r = game.tick_with_hits(Some(&input(t).bytes()), &mesh, &mut anim.ctl(&ratchet), &mut hooks, &mut hits);
        ran = t + 1;
        if r.hero == rc_game::hero::HeroTick::OutOfBounds { break; }
        game.hero.damage.events.clear();
        // MobyProc's +0x31 stand-in (drawn: live, not hidden, within its draw distance of the camera).
        let cam = game.camera.out.pos_f32();
        for m in game.mobys.mobys.iter_mut() {
            if m.state == rc_game::moby_runtime::state::END { break; }
            let d = ((m.position[0] - cam[0]).powi(2) + (m.position[1] - cam[1]).powi(2) + (m.position[2] - cam[2]).powi(2)).sqrt();
            m.visible = (m.state < 0x80 && m.mode & 0x81 == 0 && d < m.draw_dist.max(1) as f32) as u8;
        }
        for (i, m) in game.mobys.mobys.iter().enumerate().skip(n_static) {
            if m.state < 0xfd && seen.insert((i, m.o_class, m.spawn_id)) { spawned += 1; }
        }
    }
    let mut unported = BTreeMap::new();
    let mut ported = BTreeMap::new();
    let mut common_pass = 0;
    for (inst, t) in instances.iter().zip(&tests) {
        if !t.spawn || inst.o_class == 0 { continue; }
        let oc = inst.o_class as i16;
        if let Some(u) = ports.get(oc) {
            *ported.entry(oc).or_insert(0) += 1;
            if COMMON_PASS.contains(&u) { common_pass += 1; }
            continue;
        }
        match ports.level_update(oc) {
            Some(f) if f != 0 => unported.entry(oc).or_insert((0, f)).0 += 1,
            _ => {}
        }
    }
    let sounds = svc_cell.borrow().sounds.len();
    Some(Outcome {
        level,
        ticks: ran,
        hero_start: [hero_start[0], hero_start[1], hero_start[2]],
        hero_end: game.hero.position(),
        hero_state: game.hero.state,
        statics: n_static,
        ported_statics: ported.values().sum(),
        unported,
        ported,
        common_pass,
        spawned,
        sounds,
    })
}

fn ticks() -> u32 { std::env::var("RC_SMOKE_TICKS").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(600) }

#[test]
fn all_levels_load_spawn_and_tick() {
    let Some(_) = overlay(1) else { eprintln!("skipped: no extracted/"); return };
    let n = ticks();
    let mut failed = Vec::new();
    let (mut total_unported, mut total_common) = (0usize, 0usize);
    for level in 0..19 {
        match std::panic::catch_unwind(|| run_level(level, n)) {
            Ok(Some(o)) => {
                let unported_inst: usize = o.unported.values().map(|v| v.0).sum();
                total_unported += unported_inst;
                total_common += o.common_pass;
                eprintln!(
                    "level {level:02}: ok, {} ticks, hero {:?} → {:?} (state {:#x}), {} statics ({} run a port), {} spawned, {} sounds; unported: {} classes, {} instances {:?}",
                    o.ticks, o.hero_start.map(|x| x.round()), o.hero_end.map(|x| x.round()), o.hero_state, o.statics, o.ported_statics, o.spawned, o.sounds,
                    o.unported.len(), unported_inst, o.unported.iter().map(|(c, (k, _))| (*c, *k)).collect::<Vec<_>>()
                );
            }
            Ok(None) => { eprintln!("level {level:02}: data missing"); failed.push(level); }
            Err(e) => {
                let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                eprintln!("level {level:02}: PANIC {msg}");
                failed.push(level);
            }
        }
    }
    eprintln!(
        "all levels: {total_unported} created instances with an unported update; the common-classes pass ported {total_common} (before it: {})",
        total_unported + total_common
    );
    assert!(failed.is_empty(), "levels that did not run: {failed:?}");
}

/// Gemlik Base (13), the validation level: 3,000 ticks, and the ports its class table names.
#[test]
fn gemlik_three_thousand_ticks() {
    let Some(o) = run_level(13, 3000) else { eprintln!("skipped: no extracted/"); return };
    eprintln!("gemlik: {o:?}");
    assert!(o.ticks > 100, "Ratchet fell out at tick {}", o.ticks);
    // Crates, bolts' crates, the gold-weapon offers, teleporters, swing targets and the vendor run their ports.
    for oc in [500i16, 501, 511, 304, 1135, 758, 803, 11] { assert!(o.ported.contains_key(&oc), "class {oc} placed and ported"); }
}
