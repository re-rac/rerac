//! The classes G-HERO-002's scripted hero states unblocked (hero_states.md "Scripted control"; class_census.md): the
//! kill cuboids 1039 (U216, levels 06, 08, 13, 18) and Batalia's circling fighters 438 (U274, level 08). Each resolves
//! to its unit port on its levels and runs headless on the level data, covering every side-effect row its coverage
//! table marks ported. Skipped when `extracted/` is absent.
//!
//! `cargo xtask test-job --test classes --filter scripted_consumers:: --nocapture` prints the survey.

use rc_formats::level_overlay::LevelOverlay;
use rc_formats::moby_anim::{parse_sequences, MobyAnimClass};
use rc_formats::{collision, gameplay, moby_spawn};
use rc_game::cinematic::EngineRequest;
use rc_game::hero::Hero;
use rc_game::moby_runtime::{mode, MobyId, MobyTable};
use rc_game::moby_update::classes::units::{self, batalia_fighter as bf, kill_volume as kv};
use rc_game::moby_update::classes::{ClassUpdate, LevelPorts};
use rc_game::moby_update::scheduler::{class_info, load_level_mobys, Scheduler};
use rc_game::moby_update::services::{pvar as p, HeroCall, HitTemplate, World};
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
const EXPECTED: &[(&str, i16, &[u32], usize)] = &[("U216 1039", 1039, &[6, 8, 13, 18], 4), ("U274 438", 438, &[8], 25)];

#[test]
fn scripted_consumers_resolve_on_their_levels() {
    let Some(_) = overlay(1) else { eprintln!("skipped: no extracted/"); return };
    let mut created: HashMap<i16, usize> = HashMap::new();
    for level in 0..19u32 {
        let p = ports(level).unwrap();
        let gp = rc_formats::test_data::gameplay(level).unwrap();
        let inst = gameplay::parse_moby_instances(&gp).unwrap();
        let tests = moby_spawn::loader_spawns(&inst, &mut moby_spawn::SpawnSave::default());
        for &(name, oc, levels, _) in EXPECTED {
            let u = unit(name);
            if levels.contains(&level) {
                assert_eq!(p.get(oc), Some(u), "level {level:02} class {oc}");
                *created.entry(oc).or_default() += inst.iter().zip(&tests).filter(|(m, t)| m.o_class as i16 == oc && t.spawn).count();
            } else if p.in_table(oc) {
                assert_ne!(p.get(oc), Some(u), "level {level:02} class {oc}");
            }
        }
    }
    eprintln!("created instances now ported: {created:?}");
    for &(_, oc, _, n) in EXPECTED { assert_eq!(created[&oc], n, "class {oc}"); }
}

// ---------------------------------------------------------------------------------------------------
// Headless levels

struct Lv {
    table: MobyTable,
    classes: ClassTable,
    svc: Services,
    sched: Scheduler,
    rng: Rng,
    mesh: collision::Collision,
    counter: u64,
    missions: rc_game::moby_update::services::LevelMissions,
}

fn load(level: u32) -> Option<Lv> {
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
    for (slot, e) in core.core.moby_classes.iter().enumerate() {
        let oc = e.o_class as i16;
        let parsed = core.block(&format!("moby_class/{:04}", e.o_class)).and_then(|b| rc_formats::moby::parse_moby_class(b).ok().map(|c| (b, c)));
        if let Some((blob, c)) = parsed {
            let anim = MobyAnimClass::new(&c, parse_sequences(blob, &c).unwrap_or_default());
            let mut info = class_info(&c, slot as u8, ports.update_fn(oc));
            info.seq0 = anim.sequence(0).map(|q| rc_game::moby_runtime::Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
            classes.classes.insert(oc, (info, Some(anim)));
        } else {
            let info = rc_game::moby_runtime::ClassInfo { slot: slot as u8, no_header: true, update_fn: ports.update_fn(oc), ..Default::default() };
            classes.classes.entry(oc).or_insert((info, None));
        }
    }
    let statics = load_level_mobys(&instances, &mut classes, &pvars, &tests);
    let mut table = MobyTable::new(statics.mobys.clone(), spawnable);
    if let Some(h) = table.mobys.iter().position(|m| m.o_class == 0) { table.mobys[h].mode |= mode::NO_UPDATE; }
    let mut svc = Services::new();
    svc.level = level;
    svc.set_splines(&gameplay::parse_splines(&gp).unwrap());
    svc.groups = statics.groups(&gp);
    svc.set_moby_collision(rc_formats::moby_collision::parse_level(&core.core, &core.data).unwrap());
    svc.set_volumes(rc_formats::volumes::parse_volumes(&gp).unwrap());
    svc.build_grid(&mut table);
    Some(Lv { table, classes, svc, sched: Scheduler::new(), rng: Rng::new(), mesh, counter: 0, missions: rc_game::moby_update::services::LevelMissions::fresh_load(level, [0xff; 16]) })
}

impl Lv {
    fn world<'a>(&'a mut self, hero: &'a Hero) -> World<'a> {
        let mut w = World::new(&mut self.table, hero, &mut self.rng, &self.classes, &mut self.svc, self.counter);
        w.coll = Some(&self.mesh);
        w.camera = hero.pos;
        w.missions = &self.missions;
        w
    }
    fn load_pass(&mut self, hero: &Hero) {
        let mut sched = std::mem::take(&mut self.sched);
        {
            let mut w = self.world(hero);
            sched.load_pass(&mut w);
        }
        self.sched = sched;
    }
    /// One moby loop; the hero calls and engine requests of this tick only.
    fn tick(&mut self, hero: &Hero) {
        self.counter += 1;
        self.svc.hero_writes = None;
        self.svc.cinematic.requests.clear();
        let c = self.counter;
        self.table.free_slot_pass(c);
        let mut sched = std::mem::take(&mut self.sched);
        {
            let mut w = self.world(hero);
            sched.tick(&mut w);
        }
        self.sched = sched;
    }
    fn of_class(&self, oc: i16) -> Vec<MobyId> { self.table.mobys.iter().enumerate().filter(|(_, m)| m.o_class == oc && m.state < 0x80).map(|(i, _)| i).collect() }
    fn calls(&self) -> Vec<HeroCall> { self.svc.hero_writes.as_ref().map(|(_, f)| f.calls.iter().flatten().copied().collect()).unwrap_or_default() }
    fn cuboid_centre(&self, i: i32) -> Option<[f32; 3]> { self.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, i).map(|s| s.centre()) }
}

fn hero_at(p: [f32; 3]) -> Hero {
    let mut h = Hero::new();
    h.pos = rc_game::hero::physics::v4(p[0], p[1], p[2]);
    h
}

// ---------------------------------------------------------------------------------------------------
// U216: the kill cuboids 1039

fn fades(lv: &Lv) -> Vec<i32> { lv.svc.cinematic.requests.iter().filter_map(|r| if let EngineRequest::FadeToBlack { frames } = r { Some(*frames) } else { None }).collect() }

fn kill_cuboids_on(level: u32) {
    let Some(mut lv) = load(level) else { eprintln!("skipped"); return };
    let far = hero_at([1.0, 1.0, 1.0]);
    lv.load_pass(&far);
    lv.tick(&far);
    let ids = lv.of_class(1039);
    assert_eq!(ids.len(), 1, "level {level:02}");
    let k = ids[0];
    assert_eq!((lv.table.mobys[k].state, lv.table.mobys[k].update_dist), (1, 0xff), "level {level:02}: state 0 → 1");
    assert!(lv.calls().is_empty() && fades(&lv).is_empty(), "far away: nothing");
    let list = |lv: &Lv, base: usize| (0..32).map(|i| p::i32(&lv.table.mobys[k].pvars, base + 4 * i)).filter(|&c| c >= 0).collect::<Vec<i32>>();
    let (falls, deaths) = (list(&lv, 0), list(&lv, 0x80));
    eprintln!("level {level:02}: fall cuboids {falls:?}, fade cuboids {deaths:?}");
    assert!(!falls.is_empty() || !deaths.is_empty(), "level {level:02}: no cuboid");
    let in_fade = |lv: &Lv, q: [f32; 3]| deaths.iter().any(|&c| lv.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c).is_some_and(|_| rc_game::moby_update::triggers::point_in_cuboid(&lv.svc.volumes, q, c)));
    for &c in &falls {
        let q = lv.cuboid_centre(c).expect("cuboid");
        if in_fade(&lv, q) { continue; }
        // Ratchet (body 0) inside: SetState(0x77, 1), once however many cuboids hold him.
        let mut h = hero_at(q);
        lv.tick(&h);
        assert_eq!(lv.calls(), vec![HeroCall::SetState { id: kv::DEATH_FALL, play: true }], "level {level:02} cuboid {c}");
        assert!(fades(&lv).is_empty());
        // Already falling: nothing.
        h.state = kv::DEATH_FALL;
        lv.tick(&h);
        assert!(lv.calls().is_empty(), "level {level:02}: already in 0x77");
        // Another body: the fade and the death sequence instead.
        h.state = 0;
        h.mode = 1;
        lv.tick(&h);
        assert_eq!(fades(&lv), vec![10]);
        assert_eq!(lv.calls(), vec![HeroCall::Death]);
        // Mounted (0x32) with no ship moby (0x140940 = 0: the turret, or any level without a flown ship): still on.
        let mut m = hero_at(q);
        m.state = rc_game::hero::scripted::MOUNTED;
        lv.tick(&m);
        assert_eq!(lv.calls(), vec![HeroCall::SetState { id: kv::DEATH_FALL, play: true }], "level {level:02}: on without a ship");
        // Mounted with the ridden vehicle's record naming a live ship of class 0x45 (Gemlik's 69, 0x140940 / 0x140944):
        // off — the ship flies through the level's kill cuboids.
        let ship = (0..lv.table.mobys.len()).find(|&i| i != k && lv.table.mobys[i].state < 0xfe).expect("a live moby");
        lv.svc.vehicle.moby = Some(ship);
        lv.svc.vehicle.class = kv::SHIP_CLASS;
        lv.tick(&m);
        assert!(lv.calls().is_empty() && fades(&lv).is_empty(), "level {level:02}: off while flying a 0x45 ship");
        // Any other class (Hoven's turret 1267, Batalia's 440): on again.
        lv.svc.vehicle.class = kv::SHIP_CLASS + 1;
        lv.tick(&m);
        assert_eq!(lv.calls(), vec![HeroCall::SetState { id: kv::DEATH_FALL, play: true }], "level {level:02}: on for another class");
        lv.svc.vehicle.moby = None;
        lv.svc.vehicle.class = 0;
    }
    for &c in &deaths {
        let q = lv.cuboid_centre(c).expect("cuboid");
        lv.tick(&hero_at(q));
        assert!(!fades(&lv).is_empty(), "level {level:02} fade cuboid {c}");
        assert!(fades(&lv).iter().all(|&f| f == 10));
        assert_eq!(lv.calls().iter().filter(|c| **c == HeroCall::Death).count(), fades(&lv).len(), "one death sequence per fade");
    }
}

#[test]
fn kill_cuboids_blarg() { kill_cuboids_on(6) }
#[test]
fn kill_cuboids_batalia() { kill_cuboids_on(8) }
#[test]
fn kill_cuboids_gemlik() { kill_cuboids_on(13) }
#[test]
fn kill_cuboids_veldin2() { kill_cuboids_on(18) }

/// The death sequence the kill cuboids call is the hero's (`0x2319b0`): the deaths counted, the reload flag set.
#[test]
fn the_death_call_runs_the_death_sequence() {
    let mut h = Hero::new();
    let mut f = rc_game::moby_update::services::HeroFields::of(&h);
    f.call(HeroCall::Death);
    let coll = collision::Collision::default();
    let pad = rc_game::pad::PadState::default();
    let env = rc_game::hero::Env { coll: &coll, pad: &pad, cam_yaw: Pf::ZERO, cam_rows: [[Pf::ZERO; 4]; 3], mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
    let mut anim = rc_game::hero::RecordingAnim::default();
    let mut rng = Rng::new();
    let mut c = rc_game::hero::states::Ctx { env: &env, anim: &mut anim, rng: &mut rng, voice: None };
    f.run_calls(&mut h, &mut c);
    assert_eq!(h.fell_out, 1);
    assert!(matches!(h.damage.events.as_slice(), [rc_game::hero::damage::DamageEvent::Died { .. }]), "{:?}", h.damage.events);
}

// ---------------------------------------------------------------------------------------------------
// U274: Batalia's fighters 438

/// State 0 → 1 on the first update (the load pass): the step from the path's first segment at 45 u/s, t scaled to
/// points, the height offset within ±5; each update after, the fighter sits on the path's pose at t (0.5 × its
/// pitch) plus the offset and t advances by the step (wrapping at the count). Drawn, it leaves one type-2 trail blob
/// a tick.
#[test]
fn fighters_circle_their_paths_with_a_trail() {
    let Some(mut lv) = load(8) else { eprintln!("skipped"); return };
    let far = hero_at([1.0, 1.0, 1.0]);
    let ids = lv.of_class(438);
    assert_eq!(ids.len(), 25);
    let frac: Vec<f32> = ids.iter().map(|&i| p::ff(&lv.table.mobys[i].pvars, 4)).collect();
    lv.load_pass(&far);
    let mut t_before = Vec::new();
    for (n, &i) in ids.iter().enumerate() {
        let m = &lv.table.mobys[i];
        assert_eq!((m.state, m.update_dist), (1, 0xff), "fighter {i}");
        let pts = lv.svc.splines[p::i32(&m.pvars, 0) as usize].clone();
        let q = |k: usize| pts[k].map(f32::from_bits);
        let d = ((q(0)[0] - q(1)[0]).powi(2) + (q(0)[1] - q(1)[1]).powi(2) + (q(0)[2] - q(1)[2]).powi(2)).sqrt();
        let step = p::ff(&m.pvars, 8);
        assert_eq!(step, bf::SPEED * (1.0 / 60.0) / d, "fighter {i}");
        assert!(p::ff(&m.pvars, 0xc).abs() <= 5.0);
        let t = frac[n] * pts.len() as f32 + step;
        assert_eq!(p::ff(&m.pvars, 4), if (pts.len() as f32) < t { t - pts.len() as f32 } else { t }, "fighter {i}: the first update's step");
        t_before.push(p::ff(&m.pvars, 4));
    }
    lv.tick(&far);
    for (n, &i) in ids.iter().enumerate() {
        let m = &lv.table.mobys[i];
        let pts = lv.svc.splines[p::i32(&m.pvars, 0) as usize].clone();
        let (step, off, t) = (p::ff(&m.pvars, 8), p::ff(&m.pvars, 0xc), t_before[n]);
        let (pos, rot) = rc_game::path::pose(&pts, true, t, true);
        assert_eq!(m.position[..2], pos[..2], "fighter {i}: on its path");
        assert_eq!(m.position[2], pos[2] + off);
        assert_eq!(m.rotation[1], rot[1] * 0.5);
        let want = t + step;
        assert_eq!(p::ff(&m.pvars, 4), if (pts.len() as f32) < want { want - pts.len() as f32 } else { want });
    }
    // Drawn: one blob each.
    for &i in &ids { lv.table.mobys[i].visible = 1; }
    let before = lv.svc.fx.part_spawns.get(&2).copied().unwrap_or(0);
    lv.run_all(&far, &ids);
    assert_eq!(lv.svc.fx.part_spawns.get(&2).copied().unwrap_or(0) - before, 25, "a trail blob per drawn fighter");
    // A long run: they keep circling, none leaves state 1.
    for _ in 0..600 { lv.tick(&far); }
    assert!(ids.iter().all(|&i| lv.table.mobys[i].state == 1));
}

impl Lv {
    fn run_all(&mut self, hero: &Hero, ids: &[MobyId]) {
        let u = unit("U274 438");
        for &i in ids {
            let mut w = self.world(hero);
            rc_game::moby_update::classes::dispatch(u, &mut w, i);
        }
    }
    fn shoot(&mut self, id: MobyId, flags: u32) {
        let hero = Hero::new();
        let mut w = self.world(&hero);
        let t = HitTemplate { dir: [Pf::ZERO; 4], attacker: None, flags, b18: 0, b19: 0, h1a: 0, damage: Pf::ONE, w20: 0 };
        w.deliver_hit(id, &t);
    }
}

/// A hit with flag 0x10000 (whatever Ratchet's state: the mounted skill point is G-SAV-007's) blows it up in the air
/// (the blast: 20 streaks, 40 spark pairs, 16 puffs, the debris), sets its fall velocity to this tick's motion plus
/// 2·dt up, and it tumbles down (15·dt² gravity, the pitch along the velocity) until it crashes into the world (a
/// second blast) or drops below z 10, and is deleted. A hit without the flag only drops the record.
#[test]
fn a_shot_fighter_blows_up_and_falls() {
    let Some(mut lv) = load(8) else { eprintln!("skipped"); return };
    let far = hero_at([1.0, 1.0, 1.0]);
    lv.load_pass(&far);
    for _ in 0..3 { lv.tick(&far); }
    let ids = lv.of_class(438);
    let (a, b) = (ids[0], ids[1]);
    // Without the flag: dropped.
    lv.shoot(b, 0x20);
    lv.run_all(&far, &[b]);
    assert_eq!((lv.table.mobys[b].state, lv.table.mobys[b].hit_slot), (1, 0xff));
    // With it, Ratchet mounted.
    let mut mounted = hero_at([1.0, 1.0, 1.0]);
    mounted.state = rc_game::hero::scripted::MOUNTED;
    let streaks = lv.svc.fx.part_spawns.get(&15).copied().unwrap_or(0);
    let old = lv.table.mobys[a].position;
    lv.shoot(a, bf::SHOT_FLAG);
    lv.run_all(&mounted, &[a]);
    let m = &lv.table.mobys[a];
    assert_eq!((m.state, m.hit_slot), (2, 0xff));
    let v = p::v4f(&m.pvars, 0x10);
    let d = [m.position[0] - old[0], m.position[1] - old[1], m.position[2] - old[2] + 2.0 / 60.0];
    assert_eq!(v[..3], d, "this tick's motion plus 2·dt up");
    assert_eq!(lv.svc.fx.part_spawns.get(&15).copied().unwrap_or(0) - streaks, 20, "the blast's streaks");
    assert!(lv.svc.sounds.iter().all(|e| e.moby != a), "no sound (the skill point's jingle is G-SAV-007's)");
    // The fall.
    let mut vz = v[2];
    let mut ticks = 0;
    while lv.table.mobys[a].state < 0x80 && ticks < 2000 {
        let p0 = lv.table.mobys[a].position;
        lv.run_all(&far, &[a]);
        ticks += 1;
        let m = &lv.table.mobys[a];
        if m.state >= 0x80 { break; }
        vz -= (1.0f32 / 60.0) * (1.0 / 60.0) * 15.0;
        let v = p::v4f(&m.pvars, 0x10);
        assert_eq!(v[2], vz);
        assert_eq!(m.position[2], p0[2] + vz);
        assert_eq!(m.rotation[1], -(v[2]).atan2((v[0] * v[0] + v[1] * v[1]).sqrt()));
    }
    let crashed = lv.svc.fx.part_spawns.get(&15).copied().unwrap_or(0) - streaks == 40;
    eprintln!("fighter {a}: deleted after {ticks} ticks of falling; crashed into the world: {crashed}");
    assert!(lv.table.mobys[a].state >= 0x80, "deleted");
}

/// The collision word follows the 4-unit edge rule each tick.
#[test]
fn fighter_collision_is_off_near_the_origin_planes() {
    let Some(mut lv) = load(8) else { eprintln!("skipped"); return };
    let far = hero_at([1.0, 1.0, 1.0]);
    lv.load_pass(&far);
    lv.tick(&far);
    let a = lv.of_class(438)[0];
    let coll = lv.classes.classes.get(&438).is_some_and(|(i, _)| i.has_collision);
    assert_eq!(lv.table.mobys[a].has_collision, coll, "high above the level: the class's collision");
    lv.table.mobys[a].state = 7;
    lv.table.mobys[a].position[2] = 3.0;
    lv.run_all(&far, &[a]);
    assert!(!lv.table.mobys[a].has_collision, "z < 4: off");
    lv.table.mobys[a].position[2] = 50.0;
    lv.run_all(&far, &[a]);
    assert_eq!(lv.table.mobys[a].has_collision, coll);
}
