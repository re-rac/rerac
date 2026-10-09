//! Package P5 (docs/plan/hero_states.md §3): the Magneboots on Orxon's magnetic floors (level 10, surface 2),
//! headless on the level's world mesh. Skipped when `extracted/` is absent.

use rc_formats::moby_anim::{parse_sequence, parse_sequences, MobyAnimClass, MobySequence};
use rc_formats::{collision, gadget, level};
use rc_game::collision_query::{coll_line, QueryFlags};
use rc_game::hero::anim::RatchetAnim;
use rc_game::hero::boots::MAGNEBOOTS;
use rc_game::hero::items::{ItemClass, ItemData, ItemDef, HERO_LISTS};
use rc_game::moby_runtime::{ClassInfo, Moby, MobyTable};
use rc_game::pad::{button, PadInput};
use rc_game::tick::{Game, GameOptions, TickHooks};

struct Lv {
    mesh: collision::Collision,
    ratchet: MobyAnimClass,
    /// Ratchet's class scale (the hand point's joint matrices).
    scale: f32,
    death_z: f32,
    /// The wrench (item 8 = class 71 on list 0), as the engine builds it from the item definitions.
    items: ItemData,
}

fn level_data(n: usize) -> Option<Lv> {
    let d = rc_formats::test_data::root().join(format!("levels/{n:02}"));
    let data = rc_formats::test_data::core_data(n as u32)?;
    let idx = std::fs::read(d.join("core_index.bin")).ok()?;
    let settings = rc_formats::test_data::gameplay_section(n as u32, "level_settings")?;
    let core = level::parse_level_core(&idx, data.len()).ok()?;
    let mesh = collision::parse_collision(&core, &data).ok()?;
    let death_z = f32::from_le_bytes(settings[0x28..0x2c].try_into().unwrap());
    let blob = rc_formats::test_data::core_block(n as u32, "moby_class/0000")?;
    let class = rc_formats::moby::parse_moby_class(&blob).ok()?;
    let seqs: Vec<Option<MobySequence>> = (0..256)
        .map(|i| rc_formats::test_data::core_block(n as u32, &format!("ratchet_seq/{i:03}")).and_then(|b| parse_sequence(&b, 0).ok()))
        .collect();
    let mut defs = vec![ItemDef::default(); 37];
    defs[8] = ItemDef { slot: 0, attach: 0, o_class: 71, b18: 0 };
    let hero_chains = HERO_LISTS.iter().map(|&l| gadget::joint_list(&blob, &class.header, l).map(|(a, _)| a).unwrap_or_default()).collect();
    let classes = gadget::parse_gadget_classes(&core, &data)
        .ok()?
        .iter()
        .map(|g| {
            let c = &g.moby.class;
            let chains = (0..16).map_while(|l| gadget::joint_list(&g.blob, &c.header, l).ok().map(|(a, _)| a)).collect();
            ItemClass { o_class: g.moby.o_class as i16, anim: MobyAnimClass::new(c, parse_sequences(&g.blob, c).unwrap_or_default()), scale: c.header.scale, chains }
        })
        .collect();
    Some(Lv { mesh, ratchet: MobyAnimClass::new(&class, seqs), scale: class.header.scale, death_z, items: ItemData { defs, hero_chains, classes } })
}

/// A clear, near-flat surface-2 face (a line from 3 above hits it, normal within 25° of up) with surface-2 ground
/// 2 ahead along one of the four axes too: the point and that direction's yaw.
fn flat_magnet_floor(mesh: &collision::Collision) -> Option<([f32; 3], f32)> {
    let hit = |p: [f32; 3]| coll_line(mesh, [p[0], p[1], p[2] + 3.0], [p[0], p[1], p[2] - 0.5], QueryFlags(2));
    let flat = |o: &rc_game::collision_query::CollOutput| 0.9 < o.normal[2] / (o.normal[0].hypot(o.normal[1]).hypot(o.normal[2]));
    for c in &mesh.cells {
        for f in &c.faces {
            if f.surface & 0x1f != 2 { continue; }
            let v = f.v.map(|k| c.vertices[k as usize]);
            let p = [(v[0][0] + v[1][0] + v[2][0]) / 3.0, (v[0][1] + v[1][1] + v[2][1]) / 3.0, (v[0][2] + v[1][2] + v[2][2]) / 3.0];
            let Some(h) = hit(p) else { continue };
            if h.surface_id() != 2 || (h.point[2] - p[2]).abs() > 0.01 || !flat(&h) { continue; }
            for k in 0..4 {
                let y = k as f32 * std::f32::consts::FRAC_PI_2;
                let q = [p[0] + 2.0 * y.cos(), p[1] + 2.0 * y.sin(), p[2]];
                if hit(q).is_some_and(|o| o.surface_id() == 2 && (o.point[2] - p[2]).abs() < 0.3 && flat(&o)) { return Some((p, y)); }
            }
        }
    }
    None
}

/// (state, gravity mode, 0x13f658, position) per tick of Ratchet on Orxon at `at`, facing +x.
fn run(lv: &Lv, at: [f32; 3], yaw: f32, boots: bool, ticks: u32, input: impl Fn(u32) -> PadInput) -> Vec<(i32, u8, i16, [f32; 3])> {
    let mut out = Vec::new();
    run_with(lv, at, yaw, boots, false, ticks, input, |g| out.push((g.hero.state, g.hero.gravity_mode, g.hero.f658, g.hero.position())));
    out
}

/// [`run`] with `rec` called after each tick; `wrench`: the wrench in the hand.
#[allow(clippy::too_many_arguments)]
fn run_with(lv: &Lv, at: [f32; 3], yaw: f32, boots: bool, wrench: bool, ticks: u32, input: impl Fn(u32) -> PadInput, mut rec: impl FnMut(&Game)) {
    let class = ClassInfo { scale: lv.scale, ..Default::default() };
    let mut m = Moby::init_instance(0, 0, Some(&class));
    m.position = [at[0], at[1], at[2], 1.0];
    m.rotation = [0.0, 0.0, yaw, 0.0];
    let mut g = Game::new(&lv.mesh, MobyTable::new(vec![m], 16), 0, GameOptions::default(), lv.death_z);
    g.finish_load();
    g.hero.idle.level = 10;
    if boots { g.hero.grant_items(&[MAGNEBOOTS]); }
    if wrench {
        g.item_data = Some(lv.items.clone());
        g.item_globals.wrench_flag = 1;
    }
    let mut anim = RatchetAnim::new(&lv.ratchet);
    let mut mobys = |_: &mut MobyTable, _: &rc_game::hero::Hero, _: &mut rc_game::rng::Rng, _: &rc_game::follow_camera::CameraView, _: &collision::Collision, _: u64| {};
    let mut parts = |_: &rc_game::hero::Hero, _: &rc_game::follow_camera::CameraView, _: &mut rc_game::rng::Rng, _: u64| {};
    let mut hooks = TickHooks { mobys: &mut mobys, particles: &mut parts, world: None };
    for t in 0..ticks {
        let b = input(t).bytes();
        g.tick(Some(&b), &lv.mesh, &mut anim.ctl(&lv.ratchet), &mut hooks);
        rec(&g);
    }
}

/// On a magnetic walkway with the Magneboots: idle is gravity mode 1 with 0x13f658, the walk is 0x3f (up to
/// 3.5 u/s); without the boots the same floor is plain ground (mode 0, the walk 2).
#[test]
fn orxon_magnetic_walkway() {
    let Some(lv) = level_data(10) else { eprintln!("skipped: no extracted/levels/10"); return };
    let (p, yaw) = flat_magnet_floor(&lv.mesh).expect("a flat magnetic floor on Orxon");
    let walk = |t: u32| if (40..70).contains(&t) { PadInput::neutral().stick(0.0, -1.0) } else { PadInput::neutral() };
    let at = [p[0], p[1], p[2] + 0.2];
    let recs = run(&lv, at, yaw, true, 200, walk);
    let mut s: Vec<(usize, i32)> = Vec::new();
    for (t, r) in recs.iter().enumerate() { if s.last().map(|x| x.1) != Some(r.0) { s.push((t, r.0)); } }
    eprintln!("Orxon magnet floor at {p:?}: yaw {yaw}: states {s:x?}, end {:?}", recs.last().unwrap().3);
    assert_eq!((recs[30].0, recs[30].1, recs[30].2), (0, 1, 1), "idle on the magnetic floor: mode 1");
    assert!(recs[42..70].iter().all(|r| r.0 == 0x3f && r.1 == 1), "the Magneboots walk: {s:x?}");
    // No stick after 30 ticks of 0x3f: idle at once (and gravity mode 1 again).
    assert_eq!((recs[71].0, recs[75].1), (0, 1), "{s:x?}");
    let d = ((recs[70].3[0] - recs[40].3[0]).powi(2) + (recs[70].3[1] - recs[40].3[1]).powi(2)).sqrt();
    assert!(d > 0.6, "walked {d}");
    let again = run(&lv, at, yaw, true, 200, walk);
    assert!(recs.iter().zip(&again).all(|(a, b)| a == b), "deterministic");
    let plain = run(&lv, at, yaw, false, 200, walk);
    assert!(plain.iter().all(|r| r.1 == 0 && r.2 == 0 && r.0 != 0x3f));
    assert!(plain[42..70].iter().any(|r| r.0 == 2));
}

/// Orxon's magnetic spiral (x 300..315, y 229..251, z 44..72): the walk climbs the bend onto the wall and over the
/// ceiling, and standing still at the bends he stays put. The capsule's sphere sits 0.6 up the hero's own z axis
/// in gravity modes 1 / 2 (`HeroCapsulePasses` 0x233940 → `0x248ea8`); lifted along the world z, on the 54° bend
/// it dug into the floor, the push-out held him 0.05 above it (walking in place in the air) and slid him along
/// when idle.
#[test]
fn orxon_magnetic_spiral() {
    let Some(lv) = level_data(10) else { eprintln!("skipped: no extracted/levels/10"); return };
    let at = [307.0, 240.0, 57.0];
    let yaw = 1.139_874_6;
    let walk = |stop: u32| move |t: u32| if (30..stop).contains(&t) { PadInput::neutral().stick(0.0, -1.0) } else { PadInput::neutral() };
    let recs = run(&lv, at, yaw, true, 760, walk(760));
    let top = recs.iter().map(|r| r.3[2]).fold(f32::MIN, f32::max);
    eprintln!("spiral: top z {top}, end {:?}", recs.last().unwrap().3);
    assert!(top > 71.5, "over the ceiling: top z {top}");
    assert!(recs[40..].iter().all(|r| r.0 == 0x3f && r.1 == 1), "the Magneboots walk throughout");
    // Stopped at the 54° bend, on the wall and on the ceiling: no drift.
    for stop in [230, 300, 440, 760] {
        let recs = run(&lv, at, yaw, true, stop + 300, walk(stop));
        let (a, b) = (recs[stop as usize + 60].3, recs.last().unwrap().3);
        let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        assert!(d < 0.05, "stopped at tick {stop}: drift {d}");
        assert_eq!((recs.last().unwrap().0, recs.last().unwrap().1), (0, 1));
    }
}

/// The Comet-Strike (crouch + □) on the spiral's wall and ceiling: Ratchet stays put on the Magneboots (gravity
/// along −normal, `0x248b68`) and the wrench flies along his tilted facing (`0x248da0`: the model x axis), out
/// from the surface's plane by no more than it is in the upright case; it comes back to the hand.
#[test]
fn orxon_comet_strike_on_the_wall_and_ceiling() {
    let Some(lv) = level_data(10) else { eprintln!("skipped: no extracted/levels/10"); return };
    let at = [307.0, 240.0, 57.0];
    let yaw = 1.139_874_6;
    for stop in [440u32, 790] {
        let input = move |t: u32| {
            if (30..stop).contains(&t) {
                PadInput::neutral().stick(0.0, -1.0)
            } else if (stop + 10..stop + 20).contains(&t) {
                PadInput::neutral().press(button::R1)
            } else if (stop + 20..stop + 23).contains(&t) {
                PadInput::neutral().press(button::R1 | button::SQUARE)
            } else {
                PadInput::neutral()
            }
        };
        // (state, gravity mode, position, ground normal, the wrench's moby state and position)
        let mut recs = Vec::new();
        run_with(&lv, at, yaw, true, true, stop + 260, input, |g| {
            let w = g.hero.items.slot.item.as_ref().map_or((0, [0.0; 3]), |m| (m.mstate, [m.position[0], m.position[1], m.position[2]]));
            let n = g.hero.ground_normal.map(|v| v.to_f32());
            recs.push((g.hero.state, g.hero.gravity_mode, g.hero.position(), [n[0], n[1], n[2]], w.0, w.1));
        });
        let s = stop as usize;
        let entered = recs[s..].iter().position(|r| r.0 == 0x15).unwrap_or_else(|| panic!("stop {stop}: no comet strike")) + s;
        let out = recs[entered..].iter().position(|r| r.4 == 10).unwrap_or_else(|| panic!("stop {stop}: the wrench was never thrown")) + entered;
        let back = recs[out..].iter().position(|r| r.4 == 11).expect("never turned back") + out;
        let caught = recs[back..].iter().position(|r| r.4 == 0).expect("never caught") + back;
        let p0 = recs[entered].2;
        let n = recs[entered].3;
        for r in &recs[entered..] {
            assert_eq!(r.1, 1, "stop {stop}: left the Magneboots");
            let d = ((r.2[0] - p0[0]).powi(2) + (r.2[1] - p0[1]).powi(2) + (r.2[2] - p0[2]).powi(2)).sqrt();
            assert!(d < 0.1, "stop {stop}: Ratchet slid {d}");
        }
        // Out along the surface: its height over Ratchet's plane stays small; it went away ≥ 2 units (on the
        // ceiling the spiral's curve ahead bounces it back early).
        let mut far = 0.0f32;
        for r in &recs[out..caught] {
            let d = [r.5[0] - p0[0], r.5[1] - p0[1], r.5[2] - p0[2]];
            let h = d[0] * n[0] + d[1] * n[1] + d[2] * n[2];
            assert!((-0.5..2.0).contains(&h), "stop {stop}: the wrench {h} off the plane");
            far = far.max((d[0] * d[0] + d[1] * d[1] + d[2] * d[2] - h * h).sqrt());
        }
        assert!(far > 2.0, "stop {stop}: the wrench went only {far}");
        eprintln!("stop {stop}: n {n:?}, out {out} back {back} caught {caught}, far {far}");
    }
}
