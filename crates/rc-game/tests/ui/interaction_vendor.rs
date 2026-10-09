//! The "use" system and the Gadgetron vendor on Novalis, headless (docs/plan/interaction.md): the level's mobys
//! through the loader and the scheduler (the vendor class 11, the gold-weapon offers 304 and the talking NPC 774 on
//! the talk system), the context prompt, the hand-off to game mode 5, and the vendor screen's purchase flow
//! (`rc_game::menus::vendor`) on the game state of a first Novalis arrival. Skipped without `extracted/`.
//!
//! Script (as the engine check, docs/plan/interaction.md §7): Ratchet placed 3 units in front of the vendor, facing
//! it; tick 100 △ opens the vendor; in mode 5, frame 80 ✕ (the Pyrocitor, the middle entry) and frame 100 ✕
//! ("Purchase?" yes); frame 140 △ leaves; back in mode 0 the prompt returns.

use rc_formats::save_game::{ChunkTables, ItemTables, SaveGameLump};
use rc_formats::{collision, gameplay, level, strings};
use rc_game::game_state::{GameState, SessionState};
use rc_game::hero::anim::RatchetAnim;
use rc_game::hero::Hero;
use rc_game::hud::HudAssets;
use rc_game::menus::vendor::{Vendor, VendorClasses, VendorOut, VendorTables};
use rc_game::menus::{MenuAssets, MenuInput, Overlay};
use rc_game::moby_runtime::{mode, MobyTable, Seq0Info};
use rc_game::moby_update::interact::{owner, Handoff, TalkTables};
use rc_game::moby_update::scheduler::{self, class_info, load_static_mobys, Groups, Scheduler};
use rc_game::moby_update::services::{ClassData, SharedServices, World};
use rc_game::moby_update::{ClassTable, Services};
use rc_game::pad::{button, PadInput, PadState};
use rc_game::rng::Rng;
use rc_game::tick::{Game, GameOptions, TickHooks};
use std::sync::Arc;

/// Ratchet 3 units in front of the vendor (169.95, 140.44, 60; yaw −2.2089), facing it.
const HERO_AT: [f32; 4] = [168.16, 138.03, 60.0, 0.9327];

struct Data {
    mesh: collision::Collision,
    instances: Vec<gameplay::MobyInstance>,
    pvars: Vec<Option<Vec<u8>>>,
    splines: Vec<Vec<[f32; 4]>>,
    gp: Vec<u8>,
    classes: ClassTable,
    spawnable: usize,
    death_z: f32,
    coll_blobs: Vec<(i32, rc_formats::moby_collision::MobyCollision)>,
    ratchet: rc_formats::moby_anim::MobyAnimClass,
    overlay: Overlay,
    items: ItemTables,
    messages: Vec<strings::Message>,
    glyphs: [rc_formats::font::GlyphTable; 3],
    state: GameState,
    session: SessionState,
}

fn load() -> Option<Data> {
    use rc_formats::moby_anim::{parse_sequence, parse_sequences, MobyAnimClass, MobySequence};
    let root = rc_formats::test_data::root();
    let dir = root.join("levels/01");
    let data = rc_formats::test_data::core_data(1)?;
    let idx = std::fs::read(dir.join("core_index.bin")).ok()?;
    let gp = rc_formats::test_data::gameplay(1)?;
    let settings = rc_formats::test_data::gameplay_section(1, "level_settings")?;
    let core = level::parse_level_core(&idx, data.len()).unwrap();
    let mesh = collision::parse_collision(&core, &data).unwrap();
    let instances = gameplay::parse_moby_instances(&gp).unwrap();
    let pvars = gameplay::parse_pvars(&gp).unwrap();
    let splines = gameplay::parse_splines(&gp).unwrap();
    let rd = |o: usize| i32::from_le_bytes(gp[o..o + 4].try_into().unwrap());
    let spawnable = rd(rd(0x44) as usize + 4) as usize;
    let death_z = f32::from_le_bytes(settings[0x28..0x2c].try_into().unwrap());
    let mut classes = ClassTable::default();
    for (slot, e) in core.moby_classes.iter().enumerate() {
        let oc = e.o_class as i16;
        let Some(blob) = rc_formats::test_data::core_block(1, &format!("moby_class/{:04}", e.o_class)) else { continue };
        let Ok(c) = rc_formats::moby::parse_moby_class(&blob) else { continue };
        let anim = MobyAnimClass::new(&c, parse_sequences(&blob, &c).unwrap_or_default());
        let mut info = class_info(&c, slot as u8, scheduler::port_update_fn(oc));
        info.seq0 = anim.sequence(0).map(|q| Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
        classes.classes.insert(oc, (info, Some(anim)));
    }
    let coll_blobs = rc_formats::moby_collision::parse_level(&core, &data).unwrap();
    let blob = rc_formats::test_data::core_block(1, "moby_class/0000")?;
    let class = rc_formats::moby::parse_moby_class(&blob).unwrap();
    let seqs: Vec<Option<MobySequence>> = (0..256)
        .map(|i| rc_formats::test_data::core_block(1, &format!("ratchet_seq/{i:03}")).and_then(|b| parse_sequence(&b, 0).ok()))
        .collect();
    let ratchet = MobyAnimClass::new(&class, seqs);
    // The game state of a first Novalis arrival (game_state.md §4), with 5000 bolts.
    let elf = std::fs::read(root.join("boot/SCUS_971.99")).ok()?;
    let lump = SaveGameLump::parse(&std::fs::read(root.join("global/save_game.bin")).ok()?).unwrap();
    let ov0 = std::fs::read(root.join("levels/00/overlay.bin")).ok()?;
    let ov1 = std::fs::read(dir.join("overlay.bin")).ok()?;
    let tables = ChunkTables::from_boot_elf(&elf).unwrap();
    let mut state = GameState::new_game(tables, &lump.template).unwrap();
    let mut session = SessionState::default();
    state.apply_level_start(0, &ItemTables::load(&elf, &ov0).unwrap(), &mut session);
    state.on_veldin_clank_init(&mut session);
    state.apply_transition(1);
    let items = ItemTables::load(&elf, &ov1).unwrap();
    state.apply_level_start(1, &items, &mut session);
    state.global.bolts = 5000;
    let messages = strings::parse_strings(&gp, 0).unwrap();
    let (glyphs, _) = rc_formats::font::parse_glyph_tables(&ov1).unwrap();
    let overlay = Overlay::parse(&ov1).unwrap();
    Some(Data { mesh, instances, pvars, splines, gp: gp.to_vec(), classes, spawnable, death_z, coll_blobs, ratchet, overlay, items, messages, glyphs, state, session })
}

/// One frame's record: (tick counter, game mode 0 / 5, prompt owner, vendor class state, bolts, vendor substate, the
/// vendor moby's sequence B, Ratchet's state, the world ran this frame).
type Rec = (u64, u8, i32, u8, i32, i8, u8, i32, bool);

struct Run {
    recs: Vec<Rec>,
    handoffs: Vec<(usize, Handoff)>,
    purchases: Vec<(usize, (usize, bool, i32, i32))>,
    sounds: Vec<(usize, Vec<u8>)>,
    state: GameState,
    talk_slots: usize,
    /// Frames on which the salesman started a voice line (stream id).
    voices: Vec<(usize, i32)>,
    /// Mode-0 frames whose tick registered the vendor's draw callback `0x2ba9c0` (beam and glow points) on list 2.
    beam_frames: Vec<usize>,
    /// `VendorStartWeaponDemo` requests: (frame, item, demo scene).
    demos: Vec<(usize, usize, i32)>,
}

/// Runs `frames` main-loop frames: gameplay ticks in mode 0 (pad `tick_input(frame)`), vendor frames in mode 5
/// (pad `vendor_input(frame since the open)`).
fn run(d: &Data, frames: usize, bolts: i32) -> Run {
    let classes = Arc::new(ClassTable { classes: d.classes.classes.clone() });
    let mut ct = ClassTable { classes: d.classes.classes.clone() };
    let mut statics = load_static_mobys(&d.instances, &mut ct, &d.pvars);
    let hero_idx = statics.iter().position(|m| m.o_class == 0).expect("Ratchet");
    let vendor = statics.iter().position(|m| m.o_class == 11).expect("the vendor 11");
    statics[hero_idx].position = [HERO_AT[0], HERO_AT[1], HERO_AT[2], 0.0];
    statics[hero_idx].rotation[2] = HERO_AT[3];
    let mut table = MobyTable::new(statics, d.spawnable);
    table.mobys[hero_idx].mode |= mode::NO_UPDATE;
    let mut game = Game::new(&d.mesh, table, hero_idx, GameOptions::default(), d.death_z);
    game.hero.idle.level = 1;
    let mut svc = Services::new();
    svc.level = 1;
    svc.set_splines(&d.splines);
    svc.groups = Groups::parse(&d.gp, &|i| (i < d.instances.len()).then_some(i));
    svc.set_moby_collision(d.coll_blobs.clone());
    svc.build_grid(&mut game.mobys);
    let mut gs = d.state.clone();
    gs.global.bolts = bolts;
    let mut session = d.session;
    svc.counters.bolts = gs.global.bolts;
    svc.counters.gold_weapons = gs.global.gold_weapons.to_vec();
    // The "use" system's load step (the engine's interact_render::install).
    let tables = TalkTables::load(&d.overlay).expect("talk tables");
    let shop = tables.shop.clone();
    svc.interact.tables = Arc::new(tables);
    svc.interact.messages = Arc::new(d.messages.clone());
    svc.interact.register_talk_mobys(d.instances.iter().enumerate().map(|(i, x)| (i, x.unknown_74)));
    let talk_slots = svc.interact.talk_slots.len();
    svc.interact.sync_game(&gs);
    let mut vt = VendorTables::load(&d.overlay, d.items.item_defs_addr, shop).expect("vendor tables");
    // The popup's class (its animation times the buy flow); the salesman's sequences are the engine's (vendor.bin).
    vt.classes = Arc::new(VendorClasses { popup: classes.anim(0x471).cloned(), salesman: None });
    assert!(vt.layout.is_some(), "the vendor's placement tables");
    let assets = MenuAssets::new(HudAssets { icons: Vec::new(), frame_sizes: Vec::new(), glyphs: d.glyphs, messages: d.messages.clone() }, d.overlay.clone());
    let mut sched = Scheduler::new();
    {
        let hero: Hero = game.hero.clone();
        let mut w = World::new(&mut game.mobys, &hero, &mut game.rng, &*classes, &mut svc, 0);
        w.camera = game.camera.out.pos;
        w.coll = Some(&d.mesh);
        sched.load_pass(&mut w);
    }
    game.finish_load();
    let mut anim = RatchetAnim::new(&d.ratchet);
    let svc_cell = std::cell::RefCell::new(&mut svc);
    let mut out = Run { recs: Vec::new(), handoffs: Vec::new(), purchases: Vec::new(), sounds: Vec::new(), state: GameState::zeroed(ChunkTables { global: Vec::new(), level: Vec::new() }), talk_slots, voices: Vec::new(), beam_frames: Vec::new(), demos: Vec::new() };
    let mut v: Option<(Vendor, usize)> = None;
    let mut pad = PadState::default();
    for f in 0..frames {
        // The mode-0 tick, or mode 5's world update (substates 0 / 2: the tick in its scene form, no camera update).
        let world = v.as_ref().is_some_and(|(vend, _)| vend.world_runs());
        if v.is_none() || world {
            let input = if f == 100 { PadInput::neutral().press(button::TRIANGLE) } else { PadInput::neutral() };
            game.hero.owned.0 = gs.global.owned;
            game.camera_paused = world;
            svc_cell.borrow_mut().interact.sync_game(&gs);
            let classes_ref: &ClassTable = &classes;
            let mut mobys = |table: &mut MobyTable, hero: &Hero, rng: &mut Rng, cam: &rc_game::follow_camera::CameraView, coll: &collision::Collision, counter: u64| {
                let mut s = svc_cell.borrow_mut();
                let mut w = World::new(table, hero, rng, classes_ref, &mut s, counter);
                w.camera = cam.pos;
                w.coll = Some(coll);
                w.svc.interact.begin_tick(hero.loop_in.pad.pressed, hero.state);
                sched.tick(&mut w);
            };
            let mut parts = |_: &Hero, _: &rc_game::follow_camera::CameraView, _: &mut Rng, _: u64| {};
            let mut wd = SharedServices { svc: &svc_cell, classes: classes.clone() };
            let mut hooks = TickHooks { mobys: &mut mobys, particles: &mut parts, world: Some(&mut wd) };
            game.hero.idle.counter = game.counter as i32;
            game.tick(Some(&input.bytes()), &d.mesh, &mut anim.ctl(&d.ratchet), &mut hooks);
            let mut s = svc_cell.borrow_mut();
            if s.draw_callbacks.list2.contains(&(rc_game::moby_update::classes::draw_callbacks::Callback::VendorBeam, vendor)) { out.beam_frames.push(f); }
            s.interact.apply_writes(&mut gs);
            if !world { gs.global.bolts = s.counters.bolts; }
            for h in std::mem::take(&mut s.interact.handoffs) {
                if let Handoff::OpenVendor { vendor: vm } = h {
                    let mut o = VendorOut::default();
                    let mut vend = Vendor::open(vt.clone(), &gs, vm.is_none(), &mut o);
                    // The engine's anim requests: hard cut to seq 2 at half speed.
                    apply(&mut game, &classes, &mut s, vendor, &o.anim);
                    vend.lang = 0;
                    v = Some((vend, f));
                    out.sounds.push((f, o.sounds));
                }
                out.handoffs.push((f, h));
            }
            pad = game.pad.clone();
            if !world {
                let sb = game.mobys.mobys[vendor].anim.seq_b;
                out.recs.push((game.counter, 0, s.interact.prompt.owner, game.mobys.mobys[vendor].state, gs.global.bolts, -1, sb, game.hero.state, false));
                continue;
            }
        }
        if let Some((vend, opened)) = v.as_mut() {
            // Mode 5: UpdatePad, then VendorModeUpdate.
            let k = f - *opened;
            let input = match k {
                80 | 100 => PadInput::neutral().press(button::CROSS),
                140 => PadInput::neutral().press(button::TRIANGLE),
                _ => PadInput::neutral(),
            };
            pad.update(Some(&input.bytes()), false);
            let o = vend.frame(&MenuInput::from_pad(&pad, true), &mut gs, &d.items, &mut session, &assets, &mut game.rng);
            apply(&mut game, &classes, &mut svc_cell.borrow_mut(), vendor, &o.anim);
            if o.advance_vendor {
                if let Some(c) = classes.anim(11) { rc_formats::moby_anim::advance(&mut game.mobys.mobys[vendor].anim, c); }
            }
            if vend.screens_shown() { vend.render_screens(&[(100.0, 50.0); 7], &mut game.rng, &assets); }
            if let Some(id) = o.voice { out.voices.push((f, id)); }
            svc_cell.borrow_mut().counters.bolts = gs.global.bolts;
            if let Some(p) = o.purchase { out.purchases.push((f, p)); }
            // The weapon demo (substate 3) is the engine's space-scene player; the harness has none, so the demo ends on
            // the frame after its request, as `take_weapon_demo_done` would report it: `VendorExit(1)`.
            if let Some(dm) = o.weapon_demo { out.demos.push((f, dm.item, dm.scene)); }
            let demo_done = vend.sub == 3 && out.demos.last().is_some_and(|&(df, _, _)| df < f);
            if !o.sounds.is_empty() { out.sounds.push((f, o.sounds.clone())); }
            let sub = vend.sub as i8;
            let sb = game.mobys.mobys[vendor].anim.seq_b;
            out.recs.push((game.counter, 5, svc_cell.borrow().interact.prompt.owner, game.mobys.mobys[vendor].state, gs.global.bolts, sub, sb, game.hero.state, world));
            if o.exit || demo_done {
                rc_game::moby_update::classes::vendor::on_exit(&mut game.mobys, vendor);
                // VendorExit's HeroTeleport (the engine's): 3.5 in front of the vendor, facing it, state 0.
                let vm = &game.mobys.mobys[vendor];
                let pos = rc_game::menus::vendor::to_world(&vm.rows, [vm.position[0], vm.position[1], vm.position[2]], [3.5, 0.0, 0.0]);
                let yaw = rc_game::moby_update::interact::add_rot(vm.rotation[2], std::f32::consts::PI);
                let mut hf = rc_game::moby_update::services::HeroFields::of(&game.hero);
                hf.clear_motion();
                hf.pose = Some(rc_game::moby_update::services::HeroPose { pos, yaw, target_yaw: yaw });
                hf.call(rc_game::moby_update::services::HeroCall::SetState { id: 0, play: true });
                svc_cell.borrow_mut().hero_writes = Some((game.counter, hf));
                game.camera_paused = false;
                v = None;
            }
            continue;
        }
    }
    out.state = gs;
    out
}
/// The vendor's animation requests on its moby (the engine's crate::interact_render::apply_anim).
fn apply(game: &mut Game, classes: &ClassTable, svc: &mut Services, id: usize, reqs: &[rc_game::menus::vendor::VendorAnim]) {
    use rc_game::menus::vendor::VendorAnim;
    let Some(c) = classes.anim(11) else { return };
    if svc.snapshots.len() <= id { svc.snapshots.resize(id + 1, None); }
    let m = &mut game.mobys.mobys[id];
    for r in reqs {
        match *r {
            VendorAnim::HardCut { seq, frame } => { rc_formats::moby_anim::hard_cut(&mut m.anim, c, seq, frame); }
            VendorAnim::Blend { seq, frame, ticks } => { rc_formats::moby_anim::set_sequence(&mut m.anim, c, seq, frame, ticks, &mut svc.snapshots[id]); }
            VendorAnim::Speed(x) => m.anim.speed = x,
        }
    }
}

#[test]
fn novalis_vendor_prompt_open_buy_close() {
    let Some(d) = load() else { eprintln!("skipped: no extracted/"); return; };
    let r = run(&d, 420, 5000);
    eprintln!("talk slots {}; hand-offs {:?}; purchases {:?}; sounds {:?}; voices {:?}", r.talk_slots, r.handoffs, r.purchases, r.sounds, r.voices);
    // Novalis' talk slots: the ten gold-weapon offers and the Water Pump Worker.
    assert_eq!(r.talk_slots, 11);
    // The prompt: the vendor owns the lease from the first near check on (the class goes near on a tick with
    // counter & 7 = 0, then the rule holds), and the vendor goes near (state 2, seq 1).
    let first = r.recs.iter().position(|x| x.2 == owner::VENDOR).expect("no prompt");
    assert!(first < 20, "prompt from frame {first}");
    assert!(r.recs[first..100].iter().all(|x| x.2 == owner::VENDOR && x.3 == 2 && x.6 == 1), "prompt held until △");
    // States 1 and 2 register the beam / glow-point callback every tick (`RegisterDrawCallback2(0x2ba9c0, vendor)`, the
    // △ tick included: the tail runs before the state change); state 3 (the menu) does not.
    assert!((0..=100).all(|f| r.beam_frames.contains(&f)), "{:?}", r.beam_frames);
    assert!(!r.beam_frames.iter().any(|f| (101..240).contains(f)), "state 3 (the menu's world frames) draws no beam");
    // △ at frame 100 → OpenVendorMenu, the vendor's state 3 and its unfold (seq 2), Ratchet in state 100, mode 5.
    assert!(matches!(r.handoffs.as_slice(), [(100, Handoff::OpenVendor { vendor: Some(_) })]), "{:?}", r.handoffs);
    assert_eq!((r.recs[100].3, r.recs[100].6, r.recs[100].7), (3, 2, 100));
    assert_eq!(r.recs[101].1, 5);
    assert_eq!(r.sounds[0], (100, vec![3]), "open: sound 3");
    // FadeToBlack(4): frames 101..104 run nothing; then 40 frames of substate 0 with the world running (the tick
    // counter advances), the vendor unfolding on seq 2; at the 40th, seq 3 and sound 4 (screens on).
    assert!(r.recs[101..105].iter().all(|x| !x.8 && x.5 == 0));
    assert!(r.recs[105..145].iter().all(|x| x.8 && x.5 == 0 || x.5 == 1), "world frames");
    assert_eq!(r.recs[105..144].iter().filter(|x| x.8).count(), 39);
    assert_eq!(r.recs[144].0, r.recs[104].0 + 40, "40 ticks of world update during the fly-in");
    assert!(r.sounds.iter().any(|(f, s)| *f == 144 && s == &vec![4]), "{:?}", r.sounds);
    assert_eq!((r.recs[144].5, r.recs[144].6), (1, 3), "the menu on the open sequence");
    assert!(r.recs[145..240].iter().all(|x| !x.8 && x.0 == r.recs[144].0), "no world update in the menu");
    // ✕ at menu frame 80 (frame 180): sound 0 and the popup; ✕ at 200 confirms: the popup blends back over 8 ticks and
    // folds up (seq 0 backwards), then the purchase: the Pyrocitor for 2500 (sound 7).
    assert!(r.sounds.iter().any(|(f, s)| *f == 180 && s == &vec![0]), "{:?}", r.sounds);
    assert_eq!(r.purchases.len(), 1, "{:?}", r.purchases);
    let (pf, p) = r.purchases[0];
    assert_eq!(p, (16, false, 2500, 1));
    assert!((208..=220).contains(&pf), "purchase at frame {pf}");
    assert!(r.sounds.iter().any(|(f, s)| *f == pf && s == &vec![7]), "purchase sound 7");
    let g = &r.state.global;
    assert_eq!(g.bolts, 2500);
    assert_eq!(g.owned[16], 1);
    assert_eq!(g.quick_select[..2], [10, 16]);
    assert!(g.vendor.contains(&(16 | 0x40)), "{:?}", g.vendor);
    assert!(g.ammo[16] > 0, "the Pyrocitor comes with ammo");
    // △ at menu frame 140 (frame 240): sound 5, the screens power off for 8 frames, then the fold (seq 4) and 40 world
    // frames, then VendorExit: sound 6, the vendor's state 1, mode 0.
    assert!(r.sounds.iter().any(|(f, s)| *f == 240 && s == &vec![5]), "{:?}", r.sounds);
    let leave = r.recs.iter().position(|x| x.5 == 2).expect("substate 2");
    assert_eq!(leave, 247, "8 frames of power-off (the △ frame counts)");
    assert_eq!(r.recs[leave].6, 4, "the fold");
    // 40 frames of substate 2 after the frame that entered it; then, a weapon bought that has a demo scene
    // (0x1ca4a0[16]), `VendorStartWeaponDemo` (substate 3) instead of the exit's sound 6, and `VendorExit(1)` at the
    // demo's end (the harness ends it on the next frame).
    assert_eq!(r.demos.len(), 1, "{:?}", r.demos);
    let (df, item, _scene) = r.demos[0];
    assert_eq!((df, item), (leave + 40, 16), "the Pyrocitor's demo after 40 frames of substate 2");
    let back = r.recs.iter().rposition(|x| x.1 == 5).unwrap() + 1;
    assert_eq!(back, leave + 42, "the demo's frame, then VendorExit(1)");
    assert!(!r.sounds.iter().any(|(f, s)| *f >= leave && s == &vec![6]), "no sound 6 on the demo's exit {:?}", r.sounds);
    assert_eq!(r.recs[back].1, 0);
    assert_eq!(r.recs[back].7, 0, "Ratchet back in state 0 on the first tick after the exit");
    assert!(r.recs[back..].iter().any(|x| x.2 == owner::VENDOR), "the prompt returns after the exit");
    assert!(r.recs[back..].iter().all(|x| x.4 == 2500), "the bolts stay spent: the tick writes back the services' copy");
    // Open 44 frames (4 + 40) to the menu; close 48 (8 + 40).
    eprintln!("timing: open → menu {} frames, △ → mode 0 {} frames", 144 - 100, back - 240);
}

#[test]
fn novalis_vendor_cannot_afford() {
    let Some(d) = load() else { eprintln!("skipped: no extracted/"); return; };
    let r = run(&d, 300, 1000);
    // ✕ on the Pyrocitor (2500) with 1000 bolts: select (0), the popup says why with the denied sound (2); ✕ again closes
    // it: no purchase, sound 0 again, the bolts unchanged.
    assert!(r.purchases.is_empty(), "{:?}", r.purchases);
    let s: Vec<u8> = r.sounds.iter().filter(|(f, _)| (180..240).contains(f)).flat_map(|(_, s)| s.clone()).collect();
    assert_eq!(s, vec![0, 2, 0], "{:?}", r.sounds);
    assert_eq!(r.state.global.bolts, 1000);
}

/// Determinism: two headless runs give the same per-frame records and the same end state.
#[test]
fn novalis_vendor_is_deterministic() {
    let Some(d) = load() else { eprintln!("skipped: no extracted/"); return; };
    let (a, b) = (run(&d, 300, 5000), run(&d, 300, 5000));
    assert_eq!(a.recs, b.recs);
    assert_eq!(a.purchases, b.purchases);
    assert_eq!(a.state.global.quick_select, b.state.global.quick_select);
}

/// Every level's overlay yields the price records, also the levels without NPC talk (Umbris, Gaspar: no
/// `NpcTalkRegister`): the vendor's ammo list reads its prices there.
#[test]
fn every_level_loads_the_price_records() {
    let root = rc_formats::test_data::root();
    let mut missing = Vec::new();
    for lv in 0..19 {
        let Ok(bytes) = std::fs::read(root.join(format!("levels/{lv:02}/overlay.bin"))) else { return };
        let ov = Overlay::parse(&bytes).unwrap();
        match TalkTables::load(&ov) {
            Some(t) if t.shop.ammo_price(15) == 1 => {}
            _ => missing.push(lv),
        }
    }
    assert!(missing.is_empty(), "no price records on levels {missing:?}");
}
