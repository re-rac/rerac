//! The throw gloves (docs/plan/hero_gameplay.md §13), headless: the shared glove update (the Bomb Glove's `0x2d8330`
//! and its copies) throwing each glove's object, the ammo, the objects' behaviour and their effect on creatures of two
//! levels (Novalis 577 / 572, Rilgar 866). Harness as `hero_morph.rs` (the level as the engine ticks it; `GiveItem(id,
//! equip)` on a first arrival's game state). Every run twice: identical. Skipped when `extracted/` is absent; the numbers
//! are distilled from the runs.


use rc_formats::moby_anim::{parse_sequence, parse_sequences, MobyAnimClass, MobySequence};
use rc_formats::save_game::{ChunkTables, ItemTables, SaveGameLump};
use rc_formats::{collision, gadget, gameplay, level, moby_spawn};
use rc_game::game_state::{GameState, SessionState};
use rc_game::hero::anim::RatchetAnim;
use rc_game::hero::items::{ItemClass, ItemData, ItemDef, WeaponDef, HERO_LISTS};
use rc_game::hero::Hero;
use rc_game::moby_runtime::{mode, MobyTable, Seq0Info};
use rc_game::moby_update::scheduler::{self, class_info, load_level_mobys, Scheduler};
use rc_game::moby_update::services::{ServiceHits, SharedServices, World};
use rc_game::moby_update::{ClassTable, Services};
use rc_game::pad::{button, PadInput};
use rc_game::particles::Particles;
use rc_game::rng::Rng;
use rc_game::tick::{Game, GameOptions, TickHooks};
use std::sync::Arc;

struct Lv {
    level: u32,
    reactions: std::collections::HashMap<i16, rc_game::moby_update::creature::react::Table>,
    mesh: collision::Collision,
    instances: Vec<gameplay::MobyInstance>,
    pvars: Vec<Option<Vec<u8>>>,
    tests: Vec<moby_spawn::SpawnTest>,
    splines: Vec<Vec<[f32; 4]>>,
    gp: Vec<u8>,
    classes: ClassTable,
    spawnable: usize,
    death_z: f32,
    coll_blobs: Vec<(i32, rc_formats::moby_collision::MobyCollision)>,
    ratchet: MobyAnimClass,
    seconds: Vec<Vec<u8>>,
    items: ItemData,
    weapon_defs: Vec<WeaponDef>,
    tables: ItemTables,
    state: GameState,
    session: SessionState,
}

fn load() -> Option<Lv> { load_level(1) }

fn load_level(lvl: u32) -> Option<Lv> {
    let root = rc_formats::test_data::root();
    let dir = root.join(format!("levels/{lvl:02}"));
    let data = rc_formats::test_data::core_data(lvl)?;
    let idx = std::fs::read(dir.join("core_index.bin")).ok()?;
    let gp = rc_formats::test_data::gameplay(lvl)?;
    let settings = rc_formats::test_data::gameplay_section(lvl, "level_settings")?;
    let core = level::parse_level_core(&idx, data.len()).unwrap();
    let mesh = collision::parse_collision(&core, &data).unwrap();
    let instances = gameplay::parse_moby_instances(&gp).unwrap();
    let tests = moby_spawn::loader_spawns(&instances, &mut moby_spawn::SpawnSave::default());
    let spawned: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
    let pvars = gameplay::parse_pvars_spawned(&gp, &spawned).unwrap();
    let splines = gameplay::parse_splines(&gp).unwrap();
    let rd = |o: usize| i32::from_le_bytes(gp[o..o + 4].try_into().unwrap());
    let spawnable = rd(rd(0x44) as usize + 4) as usize;
    let death_z = f32::from_le_bytes(settings[0x28..0x2c].try_into().unwrap());
    let mut classes = ClassTable::default();
    for (slot, e) in core.moby_classes.iter().enumerate() {
        let oc = e.o_class as i16;
        let parsed = rc_formats::test_data::core_block(lvl, &format!("moby_class/{:04}", e.o_class)).and_then(|b| rc_formats::moby::parse_moby_class(&b).ok().map(|c| (b, c)));
        if let Some((blob, c)) = parsed {
            let anim = MobyAnimClass::new(&c, parse_sequences(&blob, &c).unwrap_or_default());
            let mut info = class_info(&c, slot as u8, scheduler::port_update_fn(oc));
            info.seq0 = anim.sequence(0).map(|q| Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
            classes.classes.insert(oc, (info, Some(anim)));
        } else {
            let info = rc_game::moby_runtime::ClassInfo { slot: slot as u8, no_header: true, update_fn: scheduler::port_update_fn(oc), ..Default::default() };
            classes.classes.entry(oc).or_insert((info, None));
        }
    }
    let coll_blobs = rc_formats::moby_collision::parse_level(&core, &data).unwrap();
    let ratchet_blob = rc_formats::test_data::core_block(lvl, "moby_class/0000")?;
    let class = rc_formats::moby::parse_moby_class(&ratchet_blob).unwrap();
    let seqs: Vec<Option<MobySequence>> = (0..256)
        .map(|i| rc_formats::test_data::core_block(lvl, &format!("ratchet_seq/{i:03}")).and_then(|b| parse_sequence(&b, 0).ok()))
        .collect();
    let ratchet = MobyAnimClass::new(&class, seqs);
    let seconds = (0..64).map(|l| gadget::joint_list(&ratchet_blob, &class.header, l).map(|(_, b)| b).unwrap_or_default()).collect();
    let elf = std::fs::read(root.join("boot/SCUS_971.99")).ok()?;
    let ov0 = std::fs::read(root.join("levels/00/overlay.bin")).ok()?;
    let ov1 = std::fs::read(dir.join("overlay.bin")).ok()?;
    let tables = ItemTables::load(&elf, &ov1).unwrap();
    let sections = rc_formats::font::parse_overlay_sections(&ov1).unwrap();
    let (n, sz) = (rc_formats::save_game::ITEM_COUNT, rc_formats::save_game::ITEM_DEF_SIZE);
    let raw = rc_formats::font::read_overlay(&sections, tables.item_defs_addr, n * sz).unwrap().to_vec();
    let w = |i: usize, o: usize| i32::from_le_bytes(raw[i * sz + o..i * sz + o + 4].try_into().unwrap());
    let defs = (0..n).map(|i| ItemDef { slot: w(i, 8), attach: w(i, 0xc), o_class: w(i, 0x10), b18: raw[i * sz + 0x18] }).collect();
    let weapon_defs = (0..n).map(|i| WeaponDef { w18: w(i, 0x18), anims: [w(i, 0x24), w(i, 0x28), w(i, 0x2c)], w30: w(i, 0x30) }).collect();
    let hero_chains = HERO_LISTS.iter().map(|&l| gadget::joint_list(&ratchet_blob, &class.header, l).map(|(a, _)| a).unwrap_or_default()).collect();
    let item_classes = gadget::parse_gadget_classes(&core, &data)
        .unwrap()
        .iter()
        .map(|g| {
            let c = &g.moby.class;
            let chains = (0..16).map_while(|l| gadget::joint_list(&g.blob, &c.header, l).ok().map(|(a, _)| a)).collect();
            ItemClass { o_class: g.moby.o_class as i16, anim: MobyAnimClass::new(c, parse_sequences(&g.blob, c).unwrap_or_default()), scale: c.header.scale, chains }
        })
        .collect();
    let items = ItemData { defs, hero_chains, classes: item_classes };
    let lump = SaveGameLump::parse(&std::fs::read(root.join("global/save_game.bin")).ok()?).unwrap();
    let ct = ChunkTables::from_boot_elf(&elf).unwrap();
    let mut state = GameState::new_game(ct, &lump.template).unwrap();
    let mut session = SessionState::default();
    state.apply_level_start(0, &ItemTables::load(&elf, &ov0).unwrap(), &mut session);
    state.on_veldin_clank_init(&mut session);
    state.apply_transition(lvl as i32);
    state.apply_level_start(lvl as i32, &tables, &mut session);
    let ov_target = rc_formats::level_overlay::LevelOverlay::parse(&ov1).ok()?;
    let ov_ref = rc_formats::level_overlay::LevelOverlay::parse(&std::fs::read(root.join("levels/01/overlay.bin")).ok()?).ok()?;
    let reactions = rc_game::moby_update::creature::react::tables_from_overlay(&ov_target, &ov_ref);
    Some(Lv { level: lvl, reactions, mesh, instances, pvars, tests, splines, gp: gp.to_vec(), classes, spawnable, death_z, coll_blobs, ratchet, seconds, items, weapon_defs, tables, state, session })
}

/// The hero's hit sink over the moby loop's services, as the engine's `CellHits` (with the guns' probe and class type).
struct CellHits<'a, 'b> {
    svc: &'a std::cell::RefCell<&'b mut Services>,
    classes: &'a ClassTable,
    coll: &'a collision::Collision,
    parts: &'a std::cell::RefCell<&'b mut Particles>,
}

impl CellHits<'_, '_> {
    fn with<R>(&mut self, f: impl FnOnce(&mut ServiceHits) -> R) -> R {
        let mut s = self.svc.borrow_mut();
        f(&mut ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) })
    }
}

type V4 = rc_game::hero::physics::V4;
type Tmpl = rc_game::moby_update::services::HitTemplate;

impl rc_game::hero::items::HitSink for CellHits<'_, '_> {
    fn sphere(&mut self, table: &mut MobyTable, r: rc_game::ps2v::Pf, centre: V4, flags: u32, ignore: Option<usize>, tmpl: &Tmpl) -> Option<usize> { self.with(|h| h.sphere(table, r, centre, flags, ignore, tmpl)) }
    fn line(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<usize>, tmpl: &Tmpl) -> Option<Option<usize>> { self.with(|h| h.line(table, a, b, flags, ignore, tmpl)) }
    fn deliver(&mut self, table: &mut MobyTable, target: usize, tmpl: &Tmpl) {
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
    fn probe(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<usize>) -> Option<Option<[f32; 3]>> { self.with(|h| h.probe(table, a, b, flags, ignore)) }
    fn probe_moby(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<usize>) -> Option<Option<rc_game::hero::items::Probe>> { self.with(|h| h.probe_moby(table, a, b, flags, ignore)) }
    fn class_type(&self, o_class: i16) -> Option<u8> { use rc_game::moby_update::services::ClassData; self.classes.info(o_class).map(|i| i.ty) }
    fn light_alloc(&mut self, l: rc_game::point_lights::PointLight) -> i32 { self.with(|h| h.light_alloc(l)) }
    fn light_get(&mut self, slot: i32) -> Option<rc_game::point_lights::PointLight> { self.with(|h| h.light_get(slot)) }
    fn light_set(&mut self, slot: i32, l: rc_game::point_lights::PointLight) { self.with(|h| h.light_set(slot, l)) }
    fn light_free(&mut self, slot: i32) { self.with(|h| h.light_free(slot)) }
    fn world(&mut self, table: &mut MobyTable, hero: &Hero, rng: &mut Rng, counter: u64, f: &mut dyn FnMut(&mut World)) -> bool {
        let mut s = self.svc.borrow_mut();
        let mut p = self.parts.borrow_mut();
        let mut w = World::new(table, hero, rng, self.classes, &mut s, counter);
        w.coll = Some(self.coll);
        w.particles = Some(&mut **p);
        f(&mut w);
        true
    }
}

/// One tick of a run.
#[derive(Clone, Debug, PartialEq)]
struct Row {
    state: i32,
    pos: [f32; 3],
    /// The hand item: id, its state (+0x20), its key B sequence.
    hand: (i32, u8, u8),
    /// The ammo of the item given, the throws so far, the object in the glove.
    ammo: i32,
    throws: u32,
    held: Option<usize>,
    /// The watched mobys: (state, position, class).
    watched: Vec<(u8, [f32; 3], i16)>,
    /// Ratchet's health.
    health: f32,
    /// The gloves' objects alive: (id, class, state, position, a pvar word of the class (the decoy's health, the mine's
    /// and the drone's target + 1), the ambient's red (the decoy's hit flash)).
    objects: Vec<(usize, i16, u8, [f32; 3], f32, u8)>,
    /// The reticles registered this tick.
    reticles: usize,
    /// The objects' class sounds of this tick (class, index, flags), the hero-side sounds ([`FakeSounds::heard`]).
    sounds: Vec<(i16, i32, u32)>,
    heard: Vec<(i16, i32)>,
    /// The particle spawns so far of the types in [`PARTS`], the explosion lights alive, the camera shake's timer.
    parts: [u64; 9],
    lights: usize,
    shake: i32,
    /// Bolts (classes 13..16) alive; mines the Taunter lures (+0x78).
    bolts: usize,
    lured: usize,
    rng: u32,
}

/// What a run gives and watches.
struct Setup<'a> {
    item: i32,
    at: ([f32; 3], f32),
    /// Moby instances to watch (their states).
    watch: &'a [usize],
    /// How long a hand-item class sound plays (the stand-in sound layer's `SoundIsAlive`; 0: no sound layer).
    sound_ticks: u64,
    /// A tougher record for the watched mobys after the load (health, the s16 +0x04): the other levels' big creatures
    /// (level 07's 871: 3 / 3, level 13's 111: 9 / 9, level 10's 1229: 20 / 20).
    record: Option<(f32, i16)>,
    /// A second item given too and asked for (the quick-select request 0x141408) at a tick.
    then: Option<(u32, i32)>,
    /// Lose the glove's object on the tick before the throw's trigger (the throw state 0x23 at tick 15): the case-3
    /// branch without an object.
    lose_held: bool,
}

/// A stand-in sound layer: every hand-item class sound plays for `ticks` ticks (the Taunter's whistles, the Suck
/// Cannon's suction), so `SoundIsAlive` answers as an audio layer would; nothing is heard.
struct FakeSounds {
    ticks: u64,
    now: u64,
    started: Vec<u64>,
    /// The hero-side sounds of this tick: the hand item's class sounds (class, index) and Ratchet's voices (0, index).
    heard: Vec<(i16, i32)>,
}

impl rc_game::hero::HeroSounds for FakeSounds {
    fn anim_advanced(&mut self, _: &rc_game::moby_runtime::Moby, _: &rc_game::hero::anim::AnimView, _: &rc_game::hero::anim::AnimView, _: &mut Rng) {}
    fn voice(&mut self, _: &rc_game::moby_runtime::Moby, index: i32, _: u32, _: &mut Rng) -> i32 {
        self.heard.push((0, index));
        -1
    }
    fn item_sound(&mut self, o_class: i16, _: [f32; 3], index: i32, _: u32, _: &mut Rng) -> i32 {
        self.heard.push((o_class, index));
        if self.ticks == 0 { return -1; }
        self.started.push(self.now);
        self.started.len() as i32 - 1
    }
    fn alive(&mut self, slot: i32) -> bool { self.started.get(slot as usize).is_some_and(|&t| self.now < t + self.ticks) }
}

fn give(lv: &Lv, item: i32) -> (GameState, SessionState) {
    let (mut gs, mut session) = (lv.state.clone(), lv.session);
    gs.give_item(item as usize, true, &lv.tables, &mut session);
    // Buying the Drone Device launches its drones (the vendor 0x2af7e8: 0x141345 = 1).
    if item == 24 { session.drone = true; }
    (gs, session)
}

fn run(lv: &Lv, s: &Setup, input: &dyn Fn(u32) -> PadInput, ticks: u32) -> Vec<Row> {
    let (mut gs, mut session) = give(lv, s.item);
    if let Some((_, second)) = s.then {
        gs.give_item(second as usize, false, &lv.tables, &mut session);
    }
    let classes = Arc::new(ClassTable { classes: lv.classes.classes.clone() });
    let mut ct = ClassTable { classes: lv.classes.classes.clone() };
    let statics = load_level_mobys(&lv.instances, &mut ct, &lv.pvars, &lv.tests);
    let watch: Vec<usize> = s.watch.iter().map(|w| statics.moby_to_instance.iter().position(|&i| i == *w).expect("watched moby")).collect();
    let mut mobys = statics.mobys.clone();
    let hero_idx = mobys.iter().position(|m| m.o_class == 0).expect("Ratchet");
    let (p, yaw) = s.at;
    mobys[hero_idx].position = [p[0], p[1], p[2], 1.0];
    mobys[hero_idx].rotation = [0.0, 0.0, yaw, 0.0];
    let mut table = MobyTable::new(mobys, lv.spawnable);
    table.mobys[hero_idx].mode |= mode::NO_UPDATE;
    let mut game = Game::new(&lv.mesh, table, hero_idx, GameOptions::default(), lv.death_z);
    game.hero.idle.level = lv.level as _;
    game.hero.set_joint_targets(&lv.seconds);
    game.item_data = Some(lv.items.clone());
    game.hero.weapons.defs = lv.weapon_defs.clone();
    game.hero.owned.0 = gs.global.owned;
    game.hero.weapons.ammo = gs.global.ammo;
    for (i, r) in lv.tables.records.iter().enumerate() { game.hero.weapons.uses_ammo[i] = r.has_ammo(); }
    game.item_globals.wrench_flag = 1;
    game.item_globals.request = session.temp_hand;
    game.item_globals.saved = session.temp_hand;
    game.item_globals.drone = session.drone;
    let mut svc = Services::new();
    svc.level = lv.level;
    svc.creatures.react.tables = lv.reactions.clone();
    svc.set_splines(&lv.splines);
    svc.groups = statics.groups(&lv.gp);
    svc.set_moby_collision(lv.coll_blobs.clone());
    svc.build_grid(&mut game.mobys);
    let mut particles = Particles::new(None, Vec::new());
    let mut sched = Scheduler::new();
    {
        let hero: Hero = game.hero.clone();
        let mut w = World::new(&mut game.mobys, &hero, &mut game.rng, &*classes, &mut svc, 0);
        w.camera = game.camera.out.pos;
        w.coll = Some(&lv.mesh);
        w.particles = Some(&mut particles);
        sched.load_pass(&mut w);
    }
    game.finish_load();
    if let Some((h, s4)) = s.record {
        for &i in &watch {
            let m = &mut game.mobys.mobys[i];
            if let Some(r) = rc_game::targeting::record(m) {
                m.pvars[r..r + 4].copy_from_slice(&h.to_le_bytes());
                m.pvars[r + 4..r + 6].copy_from_slice(&s4.to_le_bytes());
            }
        }
    }
    let mut anim = RatchetAnim::new(&lv.ratchet);
    anim.layer_joints = rc_game::hero::anim::LAYER_LISTS.map(|l| lv.seconds[l as usize].clone());
    let svc_cell = std::cell::RefCell::new(&mut svc);
    let parts_cell = std::cell::RefCell::new(&mut particles);
    let mut rows = Vec::new();
    let mut fake = FakeSounds { ticks: s.sound_ticks, now: 0, started: Vec::new(), heard: Vec::new() };
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
        let mut parts = |hero: &Hero, _: &rc_game::follow_camera::CameraView, rng: &mut Rng, _: u64| {
            let mut p = parts_cell.borrow_mut();
            rc_game::hero::fx::create_particles(hero, &mut p);
            p.update_parts(rng);
        };
        let mut world = SharedServices { svc: &svc_cell, classes: classes.clone() };
        let mut hooks = TickHooks { mobys: &mut mobys, particles: &mut parts, world: Some(&mut world) };
        game.hero.idle.counter = game.counter as i32;
        if let Some((at, second)) = s.then { if t == at { game.item_globals.request = second; } }
        // The render's +0x31 ("drawn last frame", MobyProc's; the R.Y.N.O.'s search reads it): the watched mobys are
        // in view in these runs.
        for &i in &watch { if game.mobys.mobys[i].state < 0x80 { game.mobys.mobys[i].visible = 1; } }
        let mut hits = CellHits { svc: &svc_cell, classes: &classes, coll: &lv.mesh, parts: &parts_cell };
        fake.now = game.counter;
        fake.heard.clear();
        let sounds0 = svc_cell.borrow().sounds.len();
        let r = game.tick_with_hero_sounds(Some(&input(t).bytes()), &lv.mesh, &mut anim.ctl(&lv.ratchet), &mut hooks, &mut hits, None, &mut fake);
        assert_eq!(r.hero, rc_game::hero::HeroTick::Ran, "hero stopped in state {:#x} at tick {t}", game.hero.state);
        let h = &game.hero;
        let tick = game.counter - 1;
        if s.lose_held && game.hero.state == 0x23 && game.hero.timer == 15 {
            if let Some(o) = game.hero.weapons.glove.held {
                let mut sv = svc_cell.borrow_mut();
                rc_game::moby_update::classes::bomb::delete_from_hero(&mut game.mobys, &mut sv, o, game.counter);
            }
        }
        let objects = game.mobys.mobys.iter().enumerate().filter(|(_, x)| OBJECTS.contains(&x.o_class) && x.state < 0xfd)
            .map(|(i, x)| (i, x.o_class, x.state, [x.position[0], x.position[1], x.position[2]], object_word(x), x.ambient[0])).collect();
        rows.push(Row {
            state: h.state,
            pos: h.position(),
            hand: h.items.slot.item.as_ref().map_or((h.items.slot.id, 0xff, 0xff), |x| (h.items.slot.id, x.mstate, x.anim.seq_b)),
            ammo: h.weapons.ammo[s.item as usize],
            throws: h.weapons.throws,
            held: h.weapons.glove.held,
            health: h.health as f32,
            watched: watch.iter().map(|&i| { let x = &game.mobys.mobys[i]; (x.state, [x.position[0], x.position[1], x.position[2]], x.o_class) }).collect(),
            objects,
            reticles: svc_cell.borrow().reticles.of_tick(tick).len(),
            sounds: svc_cell.borrow().sounds[sounds0..].iter().filter(|e| OBJECTS.contains(&e.o_class)).map(|e| (e.o_class, e.index, e.flags)).collect(),
            heard: fake.heard.clone(),
            parts: PARTS.map(|t| svc_cell.borrow().fx.part_spawns.get(&t).copied().unwrap_or(0)),
            lights: game.mobys.mobys.iter().filter(|x| x.o_class == rc_game::moby_update::creature::fx::LIGHT_CLASS && x.state < 0xfd).count(),
            shake: game.camera.shake.iter().map(|k| k.timer).max().unwrap_or(0),
            bolts: game.mobys.mobys.iter().filter(|x| (13..=16).contains(&x.o_class) && x.state < 0xfd).count(),
            lured: game.mobys.mobys.iter().filter(|x| x.o_class == 74 && x.state < 0xfd && x.pvars.len() >= 0x7c && x.pvars[0x78..0x7c] != [0; 4]).count(),
            rng: game.rng.state,
        });
    }
    rows
}

/// The particle types the rows count: the decoy's pop (5), the smoke / fire puffs (16), the drones' trails (55), the
/// beam explosion's sparks / streaks / puffs (11 / 15 / 8), the water's bubbles / drops / scorch (34 / 35 / 64).
const PARTS: [u8; 9] = [5, 16, 55, 11, 15, 8, 34, 35, 64];

/// The classes the gloves throw (the decoys 203 / 1900, the mine 74) and the Drone Device's drones 479.
const OBJECTS: [i16; 4] = [203, 1900, 74, 479];

/// The object's word the rows show: the decoy's health (+0x48), the mine's target (+0x54: the moby + 1).
fn object_word(m: &rc_game::moby_runtime::Moby) -> f32 {
    match m.o_class {
        203 | 1900 if m.pvars.len() >= 0x4c => f32::from_le_bytes(m.pvars[0x48..0x4c].try_into().unwrap()),
        74 if m.pvars.len() >= 0x58 => i32::from_le_bytes(m.pvars[0x54..0x58].try_into().unwrap()) as f32,
        479 if m.pvars.len() >= 0x34 => i32::from_le_bytes(m.pvars[0x30..0x34].try_into().unwrap()) as f32,
        _ => 0.0,
    }
}

/// The instance of the table's moby `id` (the moby loader's order).
#[allow(dead_code)]
fn instance_of(lv: &Lv, id: usize) -> usize {
    let mut ct = ClassTable { classes: lv.classes.classes.clone() };
    let statics = load_level_mobys(&lv.instances, &mut ct, &lv.pvars, &lv.tests);
    statics.moby_to_instance[id]
}

/// A place `d` units from instance `inst` in direction `from`, facing it.
fn facing(lv: &Lv, inst: usize, d: f32, from: f32) -> ([f32; 3], f32) {
    let c = lv.instances[inst].position;
    let p = [c[0] + d * from.cos(), c[1] + d * from.sin(), c[2] + 0.5];
    (p, (c[1] - p[1]).atan2(c[0] - p[0]))
}

fn dump(rows: &[Row]) {
    if std::env::var("RC_GLOVES_DUMP").is_err() { return; }
    for (t, r) in rows.iter().enumerate() { eprintln!("{t:4} {r:?}"); }
}

fn tap(at: &[u32]) -> impl Fn(u32) -> PadInput + '_ { move |t| if at.contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() } }

/// The first instance of class `class` (the gameplay file's order).
#[allow(dead_code)]
fn first_of(lv: &Lv, class: i16, skip: usize) -> usize {
    lv.instances.iter().enumerate().filter(|(i, m)| m.o_class as i16 == class && lv.tests.get(*i).is_some_and(|t| t.spawn)).nth(skip).map(|(i, _)| i).expect("instance")
}

/// The states an object went through.
fn states_of(rows: &[Row], id: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for r in rows { if let Some(o) = r.objects.iter().find(|o| o.0 == id) { if v.last() != Some(&o.2) { v.push(o.2); } } }
    v
}

/// The first object thrown (held at the throw, then out of the glove).
fn thrown(rows: &[Row]) -> Option<(usize, usize)> {
    let t = rows.iter().position(|r| r.throws > 0)?;
    let id = rows[t - 1].held?;
    Some((t, id))
}

/// Novalis, the Decoy Glove (item 25, class 562) 7 units east of critter group 2 (instance 592), facing it: ○ throws
/// (the throw state 0x23, one ammo); the decoy 203 flies (state 2), lands and inflates (3) about 3 units ahead; a
/// critter lands by it and bites it (its health 4 → 0, one bite per 30 ticks) instead of Ratchet, who is not hurt while
/// it stands, until it bursts (deleted); then the critter comes for Ratchet.
#[test]
fn novalis_decoy_lures_critters() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    assert_eq!(lv.items.defs[25].o_class, 562, "item 25 is the Decoy Glove (class 562)");
    let b = instance_of(&lv, 592);
    let at = facing(&lv, b, 7.0, 0.0);
    let s = Setup { item: 25, at, watch: &[b], sound_ticks: 0, record: None, then: None, lose_held: false };
    let input = tap(&[40]);
    let rows = run(&lv, &s, &input, 520);
    dump(&rows);
    let (t, d) = thrown(&rows).expect("never threw");
    assert!(rows[t - 16..t].iter().all(|r| r.state == 0x23), "thrown from the throw state");
    assert_eq!(rows[t].ammo + 1, rows[30].ammo, "one ammo a throw");
    let st = states_of(&rows, d);
    assert_eq!(&st[..4], &[0, 1, 2, 3], "held, flew, landed: {st:?}");
    let life: Vec<(usize, [f32; 3], f32)> = rows.iter().enumerate().filter_map(|(i, r)| r.objects.iter().find(|o| o.0 == d && o.2 == 3).map(|o| (i, o.3, o.4))).collect();
    let (landed, spot, _) = life[0];
    let gone = life.last().unwrap().0 + 1;
    eprintln!("decoy {d}: thrown {t}, landed {landed} at {spot:?}, gone {gone}");
    let bites: Vec<f32> = life.windows(2).filter(|w| w[1].2 < w[0].2).map(|w| w[1].2).collect();
    assert_eq!(bites, [3.0, 2.0, 1.0, 0.0], "four bites seen before the burst");
    assert!(rows[gone].objects.iter().all(|o| o.0 != d), "burst");
    let near = rows[landed..gone].iter().any(|r| { let c = r.watched[0].1; ((c[0] - spot[0]).powi(2) + (c[1] - spot[1]).powi(2)).sqrt() < 2.0 });
    assert!(near, "the critter came to the decoy");
    assert!(rows[landed..gone].iter().all(|r| r.health == 4.0), "Ratchet untouched while it stands");
    assert!(rows[gone..].iter().any(|r| r.health < 4.0), "then the critter goes for Ratchet");
    // Side effects: the throw voice 0x1a; the bite's class sound 0 and the red flash; the burst's sound 2 (its
    // SpawnBeamExplosion), its light and the camera shake; the next decoy in the glove at once.
    assert!(rows[t].heard.contains(&(0, 0x1a)), "the throw voice");
    assert!(rows[t].held.is_some() && rows[t].held != Some(d), "a new decoy in the glove at once");
    let bite_sounds = rows[landed..gone].iter().flat_map(|r| r.sounds.iter()).filter(|s| **s == (203, 0, 0)).count();
    assert!(bite_sounds >= 4, "a class sound 0 a bite ({bite_sounds})");
    let base = rows[landed].objects.iter().find(|o| o.0 == d).unwrap().5;
    assert!(rows[landed..gone].iter().any(|r| r.objects.iter().any(|o| o.0 == d && o.5 > base)), "the red flash");
    assert!(rows[gone].sounds.contains(&(203, 2, 0)), "the burst's sound 2: {:?}", rows[gone].sounds);
    assert!(rows[gone].lights > rows[gone - 2].lights, "the burst's light");
    assert!(rows[gone].shake > 0, "the burst's shake");
    assert_eq!(rows, run(&lv, &s, &input, 520), "deterministic");
}

/// Rilgar (level 05), a small amoeboid 866 (instance 1262): the decoy thrown between them; the amoeboid's target search
/// (with its arena polygon, `0x274df8`) takes the decoy and it strikes it.
#[test]
fn rilgar_decoy_lures_an_amoeboid() {
    let Some(lv) = load_level(5) else { eprintln!("skipped: no extracted/"); return };
    let at = facing(&lv, 1262, 6.0, -std::f32::consts::FRAC_PI_2);
    let s = Setup { item: 25, at, watch: &[1262], sound_ticks: 0, record: None, then: None, lose_held: false };
    let input = tap(&[30]);
    let rows = run(&lv, &s, &input, 600);
    dump(&rows);
    let (t, d) = thrown(&rows).expect("never threw");
    let st = states_of(&rows, d);
    eprintln!("decoy {d} thrown {t}: {st:?}");
    let hp: Vec<f32> = rows.iter().filter_map(|r| r.objects.iter().find(|o| o.0 == d).map(|o| o.4)).collect();
    assert_eq!(&st[..4], &[0, 1, 2, 3], "held, flew, landed: {st:?}");
    assert!(hp.iter().any(|&h| h <= 2.0), "struck twice: {hp:?}");
    assert!(rows.iter().all(|r| r.health == 4.0), "Ratchet untouched");
    let spot = rows.iter().find_map(|r| r.objects.iter().find(|o| o.0 == d && o.2 == 3).map(|o| o.3)).unwrap();
    assert!(rows.iter().any(|r| { let c = r.watched[0].1; ((c[0] - spot[0]).powi(2) + (c[1] - spot[1]).powi(2)).sqrt() < 1.5 }), "the amoeboid came to it");
    assert_eq!(rows, run(&lv, &s, &input, 600), "deterministic");
}

/// A mine's run: the states it went through, the target it sought (the moby), whether it went off (deleted).
fn mine_story(rows: &[Row], m: usize) -> (Vec<u8>, Option<usize>, Option<usize>) {
    let st = states_of(rows, m);
    let target = rows.iter().find_map(|r| r.objects.iter().find(|o| o.0 == m && o.4 > 0.0).map(|o| o.4 as usize - 1));
    let alive = rows.iter().rposition(|r| r.objects.iter().any(|o| o.0 == m));
    let gone = alive.filter(|&t| t + 1 < rows.len()).map(|t| t + 1);
    (st, target, gone)
}

/// Novalis, the Mine Glove (item 17, class 190) 9 units east of critter group 2 (7 before the Water Pump Worker's head
/// look drew from the stream, 2026-09-30): ○ throws a mine 74 (0x23, one ammo,
/// the glove reticle while it is held); it lobs, lands (armed at once: the arming timer is cleared on the ground), and
/// the critter coming down for Ratchet is sought (0x10, hopping at it) and blown up (its death flight 99).
#[test]
fn novalis_mine_seeks_a_critter() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    assert_eq!(lv.items.defs[17].o_class, 190, "item 17 is the Mine Glove (class 190)");
    let b = instance_of(&lv, 592);
    let at = facing(&lv, b, 9.0, 0.0);
    let s = Setup { item: 17, at, watch: &[b], sound_ticks: 0, record: None, then: None, lose_held: false };
    let input = tap(&[40]);
    let rows = run(&lv, &s, &input, 400);
    dump(&rows);
    let (t, m) = thrown(&rows).expect("never threw");
    assert_eq!(rows[t].ammo + 1, rows[30].ammo, "one ammo a throw");
    assert!(rows[10..t].iter().all(|r| r.reticles == 1), "the reticle while held");
    let (st, target, gone) = mine_story(&rows, m);
    eprintln!("mine {m} thrown {t}: states {st:?}, target {target:?}, gone {gone:?}");
    assert!(st.contains(&(1 | 4)) && st.iter().any(|&x| x & 0x12 == 0x12), "landed, armed, seeking: {st:?}");
    let g = gone.expect("never went off");
    assert!(target.is_some(), "sought a target");
    assert!(rows[g..].iter().any(|r| r.watched[0].0 == 99), "the critter killed");
    // Side effects: the throw voice; no new mine for 9 ticks after the throw (the 10-tick delay counts in that tick); the seek's class sound 0 and loop 1
    // (flags 4); the explosion's sound 2, `trunc(dcam · 15)` type-16 puffs on the ground (the count scales with the xy
    // camera distance: ~146 here), the light and the camera shake.
    assert!(rows[t].heard.contains(&(0, 0x1a)), "the throw voice");
    let next = rows[t..].iter().position(|r| r.held.is_some()).expect("no new mine") ;
    assert_eq!(next, 9, "the next mine 9 ticks after the throw (10, less the throw tick's count)");
    let all: Vec<(i16, i32, u32)> = rows.iter().flat_map(|r| r.sounds.iter().copied()).collect();
    assert!(all.contains(&(74, 0, 0)) && all.contains(&(74, 1, 4)), "the seek's sounds: {all:?}");
    assert!(rows[g].sounds.contains(&(74, 2, 0)), "the explosion's sound 2");
    let puffs = rows[g].parts[1] - rows[g - 1].parts[1];
    let beam = rows[g].parts[3] - rows[g - 1].parts[3];
    assert!(puffs == 146 || beam > 0, "ground puffs {puffs} or the air's beam explosion");
    assert!(rows[g].lights > rows[g - 1].lights && rows[g].shake > 0, "the light and the shake");
    assert_eq!(rows, run(&lv, &s, &input, 400), "deterministic");
}

/// Rilgar (level 05), a mine thrown at the small amoeboid 866 (instance 1262) 6 units away: it lands within 1.5 of it,
/// arms and goes off at once (the proximity search's 1.5 rule), deleting the amoeboid.
#[test]
fn rilgar_mine_takes_an_amoeboid() {
    let Some(lv) = load_level(5) else { eprintln!("skipped: no extracted/"); return };
    let at = facing(&lv, 1262, 6.0, -std::f32::consts::FRAC_PI_2);
    let s = Setup { item: 17, at, watch: &[1262], sound_ticks: 0, record: None, then: None, lose_held: false };
    let input = tap(&[30]);
    let rows = run(&lv, &s, &input, 700);
    dump(&rows);
    let (t, m) = thrown(&rows).expect("never threw");
    let (st, target, gone) = mine_story(&rows, m);
    eprintln!("mine {m} thrown {t}: states {st:?}, target {target:?}, gone {gone:?}");
    assert_eq!(st, [0, 1, 1 | 4], "held, out, landed");
    let g = gone.expect("never went off");
    assert!(rows[g].watched[0].0 >= 0xfd, "the amoeboid gone with it");
    assert!(rows.iter().all(|r| r.health == 4.0), "Ratchet untouched");
    assert_eq!(rows, run(&lv, &s, &input, 700), "deterministic");
}

/// The Drone Device's run: (the tick the drones came, their count then, the ammo before and then, the drones that took
/// a target: (drone, target moby, the tick), the watched creature's death tick).
/// (drone, target moby, tick) of each drone that took a target.
type Took = Vec<(usize, usize, usize)>;

fn drone_story(lv: &Lv, rows: &[Row]) -> (usize, usize, i32, i32, Took) {
    let drones = |r: &Row| r.objects.iter().filter(|o| o.1 == 479).count();
    let first = rows.iter().position(|r| drones(r) > 0).expect("no drones");
    let before = give(lv, 24).0.global.ammo[24];
    let mut took: Took = Vec::new();
    for (t, r) in rows.iter().enumerate() {
        for o in r.objects.iter().filter(|o| o.1 == 479 && o.2 == 3) {
            if took.iter().all(|x| x.0 != o.0) { took.push((o.0, o.4 as usize - 1, t)); }
        }
    }
    (first, drones(&rows[first]), before, rows[first].ammo, took)
}

/// Novalis, the Drone Device (item 24, class 483) given as the vendor gives it (0x141345): the hand swap launches six
/// drones 479 at once (one ammo) which orbit Ratchet (state 2); the critter coming for him is taken by one (state 3),
/// struck (its death flight 99) and the drone blows up with it; the others keep orbiting him.
#[test]
fn novalis_drones_guard_ratchet() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    assert_eq!(lv.items.defs[24].o_class, 483, "item 24 is the Drone Device (class 483)");
    let b = instance_of(&lv, 592);
    let at = facing(&lv, b, 7.0, 0.0);
    let s = Setup { item: 24, at, watch: &[b], sound_ticks: 0, record: None, then: None, lose_held: false };
    let input = |_: u32| PadInput::neutral();
    let rows = run(&lv, &s, &input, 300);
    dump(&rows);
    let (first, n, before, after, took) = drone_story(&lv, &rows);
    eprintln!("drones from {first}: {n}; ammo {before} → {after}; took {took:?}");
    assert_eq!((first, n, after + 1), (0, 6, before), "six drones at once, one ammo");
    let (d, target, t) = took[0];
    let died = rows.iter().position(|r| r.watched[0].0 == 99).expect("the critter never died");
    assert!(t < died, "taken ({t}) before it died ({died})");
    assert!(rows[died + 40].objects.iter().all(|o| o.0 != d), "the drone {d} went with it (target {target})");
    let orbiting = rows[died + 40].objects.iter().filter(|o| o.1 == 479 && o.2 == 2).count();
    assert_eq!(orbiting, 5, "the others orbit");
    // Side effects: the two type-55 trails a drone (records); the assignment's class sound (0 / 2 / 3) on the updating
    // drone; the strike's SpawnBeamExplosion (sound 1, the light, the shake).
    assert_eq!(rows[5].parts[2], 12, "two trails a drone");
    assert!(rows[t].sounds.iter().any(|s| s.0 == 479 && matches!(s.1, 0 | 2 | 3)), "the assignment's sound: {:?}", rows[t].sounds);
    let blew = rows.iter().position(|r| r.objects.iter().any(|o| o.0 == d && o.2 == 4)).expect("the drone never blew up");
    assert!(rows[blew].sounds.contains(&(479, 1, 0)), "the strike's sound 1: {:?}", rows[blew].sounds);
    assert!(rows[blew].lights > rows[blew - 1].lights && rows[blew].shake > 0, "the light and the shake");
    assert_eq!(rows, run(&lv, &s, &input, 300), "deterministic");
}

/// Rilgar (level 05), the drones launched 4 units from the small amoeboid 866 (instance 1262): one takes it and strikes
/// it (the amoeboid deleted).
#[test]
fn rilgar_drones_strike_an_amoeboid() {
    let Some(lv) = load_level(5) else { eprintln!("skipped: no extracted/"); return };
    let at = facing(&lv, 1262, 3.5, -std::f32::consts::FRAC_PI_2);
    let s = Setup { item: 24, at, watch: &[1262], sound_ticks: 0, record: None, then: None, lose_held: false };
    let input = |_: u32| PadInput::neutral();
    let rows = run(&lv, &s, &input, 300);
    dump(&rows);
    let (first, n, before, after, took) = drone_story(&lv, &rows);
    eprintln!("drones from {first}: {n}; ammo {before} → {after}; took {took:?}");
    assert_eq!((n, after + 1), (6, before));
    assert!(!took.is_empty(), "no drone took a target");
    assert!(rows.iter().any(|r| r.watched[0].0 >= 0xfd), "the amoeboid struck");
    assert_eq!(rows, run(&lv, &s, &input, 300), "deterministic");
}

/// Novalis, crates and the mine (the filter sets): a mine thrown at crate 376 (3 units ahead) meets it on its way —
/// a crate on the path (`FUN_00273278`: classes 500..540) sets it off and the path's template (damage 3) and the
/// area hit break it (its bolts fly); crates are never sought.
#[test]
fn novalis_mine_breaks_a_crate() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let at = facing(&lv, 376, 3.0, std::f32::consts::PI);
    let s = Setup { item: 17, at, watch: &[376], sound_ticks: 0, record: None, then: None, lose_held: false };
    let input = tap(&[40]);
    let rows = run(&lv, &s, &input, 200);
    dump(&rows);
    let (t, m) = thrown(&rows).expect("never threw");
    let (st, target, gone) = mine_story(&rows, m);
    eprintln!("mine {m} thrown {t}: states {st:?}, target {target:?}, gone {gone:?}");
    assert_eq!(st, [0, 1], "it went off on its way");
    let g = gone.expect("never went off");
    assert!(target.is_none(), "crates are not sought");
    assert!(rows[g..].iter().any(|r| r.watched[0].0 >= 0xfd), "the crate broken");
    assert!(rows[g..].iter().any(|r| r.bolts > rows[g - 1].bolts), "its bolts");
    assert_eq!(rows, run(&lv, &s, &input, 200), "deterministic");
}

/// Novalis, the Taunter lures a mine (its second loop over the mines' list 0x1b0c30): a mine thrown and landed, then
/// the Taunter (asked for at tick 120) whistling toward it sets its +0x78 (the mine triples its seek reach while lured).
#[test]
fn novalis_taunter_lures_a_mine() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let c = lv.instances[376].position;
    let at = ([c[0], c[1] + 9.0, c[2]], -std::f32::consts::FRAC_PI_2);
    let s = Setup { item: 17, at, watch: &[376], sound_ticks: 30, record: None, then: Some((120, 14)), lose_held: false };
    let input = |t: u32| if t == 40 || (200..=260).contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 280);
    dump(&rows);
    let (_, m) = thrown(&rows).expect("never threw");
    assert!(rows[190].objects.iter().any(|o| o.0 == m && o.2 & 4 != 0), "the mine landed");
    assert_eq!(rows[190].hand.0, 14, "the Taunter in hand");
    assert!(rows[..200].iter().all(|r| r.lured == 0) && rows[200..].iter().any(|r| r.lured == 1), "lured by the whistle");
    assert_eq!(rows, run(&lv, &s, &input, 280), "deterministic");
}

/// The throw with the object lost (the shared update's case 3 without one): the Decoy Glove clicks (the glove's class
/// sound 0, `fun_0022da68(0, 0, glove)`), throws nothing and still goes to state 4 with its lockout; the ammo was used
/// by the trigger; a new decoy comes with the tail.
#[test]
fn novalis_decoy_glove_clicks_without_a_decoy() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let b = instance_of(&lv, 592);
    let at = facing(&lv, b, 7.0, 0.0);
    let s = Setup { item: 25, at, watch: &[b], sound_ticks: 0, record: None, then: None, lose_held: true };
    let input = tap(&[40]);
    let rows = run(&lv, &s, &input, 80);
    dump(&rows);
    let t = rows.iter().position(|r| r.ammo < rows[30].ammo).expect("no trigger");
    assert!(rows[t].heard.contains(&(562, 0)), "the click: {:?}", rows[t].heard);
    assert_eq!(rows[t].throws, 0, "nothing thrown");
    assert_eq!(rows[t].hand.1, 4, "state 4");
    assert!(rows[t].held.is_some(), "a new decoy from the tail");
    assert!(rows[t..].iter().all(|r| r.objects.iter().all(|o| o.2 <= 1)), "no decoy out");
    assert_eq!(rows, run(&lv, &s, &input, 80), "deterministic");
}
