//! The Morph-o-Ray and its chicken (docs/plan/hero_gameplay.md §12), headless: the beam, the meter, the morph of
//! creatures of three classes on two levels (Novalis 577 / 572, Rilgar 866), the chicken's flight from Ratchet, its
//! burst and feathers, the Suck Cannon taking a chicken through its reaction table. Harness as
//! `hero_reactive_novalis.rs` (the level as the engine ticks it; `GiveItem(id, equip)` on a first arrival's game state).
//! Every run twice: identical. Skipped when `extracted/` is absent; the numbers are distilled from the runs.

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
    out: u8,
    /// The watched mobys: (state, position, class).
    watched: Vec<(u8, [f32; 3], i16)>,
    /// The Morph-o-Ray: firing, target, meter, HUD value, beam drawn this tick, morphs so far, light slot.
    firing: bool,
    target: Option<usize>,
    meter: f32,
    hud: i32,
    beam: bool,
    morphs: Vec<usize>,
    light: i32,
    /// The chickens alive (id, state, position, suck record state) and the feathers alive.
    chickens: Vec<(usize, u8, [f32; 3], i16)>,
    feathers: usize,
    beam_quads: usize,
    /// The beam's tip relative to the muzzle, and the aim (yaw, pitch).
    tip: [f32; 3],
    aim: (f32, f32),
    looping: bool,
    rng: u32,
    /// Live type-78 records (the beam's homing sparks) after the tick's particle update.
    sparks: usize,
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
}

/// A stand-in sound layer: every hand-item class sound plays for `ticks` ticks (the Taunter's whistles, the Suck
/// Cannon's suction), so `SoundIsAlive` answers as an audio layer would; nothing is heard.
struct FakeSounds {
    ticks: u64,
    now: u64,
    started: Vec<u64>,
}

impl rc_game::hero::HeroSounds for FakeSounds {
    fn anim_advanced(&mut self, _: &rc_game::moby_runtime::Moby, _: &rc_game::hero::anim::AnimView, _: &rc_game::hero::anim::AnimView, _: &mut Rng) {}
    fn item_sound(&mut self, _: i16, _: [f32; 3], _: i32, _: u32, _: &mut Rng) -> i32 {
        if self.ticks == 0 { return -1; }
        self.started.push(self.now);
        self.started.len() as i32 - 1
    }
    fn alive(&mut self, slot: i32) -> bool { self.started.get(slot as usize).is_some_and(|&t| self.now < t + self.ticks) }
}

fn give(lv: &Lv, item: i32) -> (GameState, SessionState) {
    let (mut gs, mut session) = (lv.state.clone(), lv.session);
    gs.give_item(item as usize, true, &lv.tables, &mut session);
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
    let mut fake = FakeSounds { ticks: s.sound_ticks, now: 0, started: Vec::new() };
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
        let r = game.tick_with_hero_sounds(Some(&input(t).bytes()), &lv.mesh, &mut anim.ctl(&lv.ratchet), &mut hooks, &mut hits, None, &mut fake);
        assert_eq!(r.hero, rc_game::hero::HeroTick::Ran, "hero stopped in state {:#x} at tick {t}", game.hero.state);
        let h = &game.hero;
        let tick = game.counter - 1;
        let m = &h.weapons.reactive.morph;
        let chickens = game.mobys.mobys.iter().enumerate().filter(|(_, x)| x.o_class == 270 && x.state < 0xfd)
            .map(|(i, x)| (i, x.state, [x.position[0], x.position[1], x.position[2]], if x.pvars.len() >= 0x14a { i16::from_le_bytes([x.pvars[0x148], x.pvars[0x149]]) } else { -1 })).collect();
        let cam = game.camera.out.pos_f32();
        rows.push(Row {
            state: h.state,
            pos: h.position(),
            hand: h.items.slot.item.as_ref().map_or((h.items.slot.id, 0xff, 0xff), |x| (h.items.slot.id, x.mstate, x.anim.seq_b)),
            out: h.f13f8,
            watched: watch.iter().map(|&i| { let x = &game.mobys.mobys[i]; (x.state, [x.position[0], x.position[1], x.position[2]], x.o_class) }).collect(),
            firing: m.firing,
            target: m.target,
            meter: m.meter,
            hud: m.hud_value,
            beam: m.beam.drawn == Some(tick),
            morphs: m.morphs.clone(),
            light: m.light,
            chickens,
            feathers: game.mobys.mobys.iter().filter(|x| x.o_class == 428 && x.state < 0xfd).count(),
            tip: std::array::from_fn(|k| m.beam.pts[11][k] - m.beam.pts[0][k]),
            aim: (m.yaw, m.pitch),
            beam_quads: if m.beam.drawn == Some(tick) { m.beam.quads(cam).len() } else { 0 },
            looping: h.fx.item_loop_alive[rc_game::hero::fx::LOOP_MORPH],
            rng: game.rng.state,
            sparks: parts_cell.borrow().live_by_type()[78] as usize,
        });
    }
    rows
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
    if std::env::var("RC_MORPH_DUMP").is_err() { return; }
    for (t, r) in rows.iter().enumerate() { eprintln!("{t:4} {r:?}"); }
}

fn hold(from: u32, to: u32) -> impl Fn(u32) -> PadInput { move |t| if (from..=to).contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() } }

/// The first instance of class `class` (the gameplay file's order).
#[allow(dead_code)]
fn first_of(lv: &Lv, class: i16, skip: usize) -> usize {
    lv.instances.iter().enumerate().filter(|(i, m)| m.o_class as i16 == class && lv.tests.get(*i).is_some_and(|t| t.spawn)).nth(skip).map(|(i, _)| i).expect("instance")
}

/// Novalis, a critter 577 in the pit (4 units away: the Water Pump Worker's head look draws from the stream and
/// moves the critters; 2026-09-30): ○ held fires the beam at it, the meter runs down (the critter's record: an
/// instant morph), the critter is replaced by a chicken 270 at its place (deleted, its bolts dropped), which runs from
/// Ratchet; letting go stops the beam and fades the light.
#[test]
fn novalis_morphs_a_critter() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    assert_eq!(lv.items.defs[21].o_class, 185, "item 21 is the Morph-o-Ray (class 185)");
    assert_eq!(lv.items.defs[21].b18, 1, "holding class 1");
    let b = instance_of(&lv, 592);
    let at = facing(&lv, b, 4.0, std::f32::consts::PI);
    let s = Setup { item: 21, at, watch: &[b], sound_ticks: 0, record: None, then: None };
    let input = hold(40, 300);
    let rows = run(&lv, &s, &input, 420);
    dump(&rows);
    let first = rows.iter().position(|r| r.firing).expect("never fired");
    assert!(rows[first..=300].iter().all(|r| r.firing && r.beam), "the beam every tick of the hold");
    let morph = rows.iter().position(|r| !r.morphs.is_empty()).expect("no morph");
    assert!(rows[morph].watched[0].0 >= 0xfd, "the critter deleted");
    let ch = rows[morph].morphs[0];
    assert!(rows[morph].chickens.iter().any(|c| c.0 == ch));
    let p0 = rows[morph].chickens.iter().find(|c| c.0 == ch).unwrap().2;
    let later = rows[420 - 1].chickens.iter().find(|c| c.0 == ch).expect("the chicken lives");
    assert!(seen_states(&rows, ch).len() >= 2, "states {:?}", seen_states(&rows, ch));
    let _ = (p0, later);
    assert!(!rows[330].firing && !rows[330].beam && rows[330].light == -1, "stopped and the light faded");
    // The beam's sparks (type 78, `0x2d2d08` → `PartType78Spawn`): pairs one tick in two while firing, homing on the
    // target while it lives; after the firing they run out (life at most 8 / 0.2 × 1.2 × 3 = 144 ticks).
    assert!(rows[first + 10..=300].iter().filter(|r| r.sparks > 0).count() > 200, "sparks while firing");
    assert!(rows[first + 10..=morph].iter().any(|r| r.sparks >= 6), "pairs pile up while homing: {:?}", rows[morph].sparks);
    assert_eq!(rows[419].sparks, 0, "run out");
    assert_eq!(rows, run(&lv, &s, &input, 420), "deterministic");
}

/// Moving does not stop the Morph-o-Ray. The game's gate is the movement group 0x1413dc, not the state 0x1413d4
/// (decompiled 0x2bc7e8 / level01 0x2d2450: `(u32)control_mode < 2 || control_mode == 4 || == 5 || == 0xc`): walking
/// or running (state 2, group 1) and the jumps (group 4) fire like standing still. The port used to test the state, so
/// only states 0 / 1 (idle / look stance) passed: the weapon had to be fired from a standstill.
#[test]
fn fires_while_walking() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let b = instance_of(&lv, 592);
    let at = facing(&lv, b, 12.0, std::f32::consts::PI);
    let s = Setup { item: 21, at, watch: &[b], sound_ticks: 0, record: None, then: None };
    let input = |t: u32| if t >= 40 { PadInput::neutral().press(button::CIRCLE).stick(0.0, -1.0) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 120);
    dump(&rows);
    let walking: Vec<_> = rows.iter().filter(|r| r.state == 2).collect();
    assert!(!walking.is_empty(), "never walked: {:?}", rows.iter().map(|r| r.state).collect::<Vec<_>>());
    assert!(walking.iter().all(|r| r.firing && r.beam), "fired every tick of the walk");
}

fn seen_states(rows: &[Row], ch: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for r in rows { if let Some(c) = r.chickens.iter().find(|c| c.0 == ch) { if v.last() != Some(&c.1) { v.push(c.1); } } }
    v
}

/// The first tick a chicken was made, the chicken, and the ticks the target was held before it.
fn morph_of(rows: &[Row]) -> Option<(usize, usize, usize)> {
    let t = rows.iter().position(|r| !r.morphs.is_empty())?;
    let held = rows[..t].iter().rev().take_while(|r| r.target.is_some()).count();
    Some((t, rows[t].morphs[0], held))
}

/// Novalis, a big amoeboid 572 (group 22) given level 07's 871 record (health 3, scale 3: every Novalis and Rilgar
/// creature has 1 / 1, an instant morph): the beam holds it while the meter runs down 3 a second (60 ticks) with the HUD
/// value rising, then the morph.
#[test]
fn novalis_morphs_a_big_amoeboid() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let a = first_of(&lv, 572, 0);
    let at = facing(&lv, a, 6.0, 0.0);
    let s = Setup { item: 21, at, watch: &[a], sound_ticks: 0, record: Some((3.0, 3)), then: None };
    let input = hold(30, 400);
    let rows = run(&lv, &s, &input, 420);
    dump(&rows);
    let (t, ch, held) = morph_of(&rows).expect("no morph");
    eprintln!("572 morphed at {t} after {held} ticks held; chicken {ch}");
    assert!(held > 10, "held {held}");
    assert!(rows[t].watched[0].0 >= 0xfd);
    assert_eq!(rows, run(&lv, &s, &input, 420), "deterministic");
}

/// Rilgar (level 05), a small amoeboid 866: the morph on a second level (the chicken class 270 runs the same code on
/// every level's table).
#[test]
fn rilgar_morphs_a_small_amoeboid() {
    let Some(lv) = load_level(5) else { eprintln!("skipped: no extracted/"); return };
    let at = facing(&lv, 1262, 5.0, -std::f32::consts::FRAC_PI_2);
    let s = Setup { item: 21, at, watch: &[1262], sound_ticks: 0, record: None, then: None };
    let input = hold(30, 200);
    let rows = run(&lv, &s, &input, 260);
    dump(&rows);
    let (t, ch, held) = morph_of(&rows).expect("no morph");
    eprintln!("866 morphed at {t} after {held} ticks held; chicken {ch}");
    assert!(rows[t].watched[0].0 >= 0xfd);
    assert_eq!(rows, run(&lv, &s, &input, 260), "deterministic");
}

/// Novalis: a critter morphed, then the Suck Cannon (asked for at tick 230) held on the chicken: its reaction table
/// (0x2e0a28..) takes it into its held state 5 and its suck record (pvar +0xe0) runs 1 → 2 → 3 → 4 (swallowed).
#[test]
fn novalis_suck_cannon_swallows_a_chicken() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let b = instance_of(&lv, 592);
    let at = facing(&lv, b, 5.0, std::f32::consts::PI);
    let s = Setup { item: 21, at, watch: &[b], sound_ticks: 0, record: None, then: Some((215, 9)) };
    let input = |t: u32| if (40..=214).contains(&t) || (260..=420).contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 440);
    dump(&rows);
    let (_, ch, _) = morph_of(&rows).expect("no morph");
    let states: Vec<(u8, i16)> = rows.iter().filter_map(|r| r.chickens.iter().find(|c| c.0 == ch).map(|c| (c.1, c.3))).collect();
    let mut recs: Vec<i16> = Vec::new();
    for &(_, r) in &states { if recs.last() != Some(&r) { recs.push(r); } }
    eprintln!("chicken record states {recs:?}");
    assert!(states.iter().any(|&(st, _)| st == 5), "never held");
    assert!(recs.contains(&3) && recs.contains(&4), "record states {recs:?}");
    assert_eq!(rows, run(&lv, &s, &input, 440), "deterministic");
}
