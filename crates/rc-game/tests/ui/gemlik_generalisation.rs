//! Gemlik Base (level 13), the level-generalisation validation level (docs/plan/level_generalisation.md): what was
//! Novalis-only and now reads Gemlik's own overlay. Skipped without `extracted/`.
//!
//! * Menus (M1–M7): the page tree, callbacks, planet tables, quick-select block and d-pad defaults, the frame's
//!   corner lists and the Port Options page, through `menus::Overlay::relocated` against level 01.
//! * Class ports (C1): Gemlik's body pieces run `FxGroupUpdate` (its class table names classes Novalis does not use);
//!   asserted in `level_ports.rs::every_level_runs_the_ports_its_class_table_names`.

use rc_formats::level_overlay::LevelOverlay;
use rc_formats::save_game::{ChunkTables, SaveGameLump};
use rc_game::game_state::GameState;
use rc_game::menus::pause::{frame, media, Data, PageMenu};
use rc_game::menus::quick_select::{QsConsts, QsTables};
use rc_game::menus::Overlay;
use rc_game::moby_update::classes::LevelPorts;
use std::sync::Arc;

fn bytes(level: u32) -> Option<Vec<u8>> { std::fs::read(rc_formats::test_data::level_dir(level).join("overlay.bin")).ok() }

fn menus(level: u32) -> Option<Overlay> { Some(Overlay::relocated(&bytes(level)?, &bytes(1)?).unwrap()) }

#[test]
fn gemlik_pause_menu_is_novalis_tree_at_gemlik_addresses() {
    let (Some(n), Some(g)) = (menus(1), menus(13)) else { eprintln!("skipped: no extracted/"); return };
    let (mn, mg) = (PageMenu::load(&n).unwrap(), PageMenu::load(&g).expect("Gemlik page menu"));
    eprintln!("Novalis {} pages / {} widgets, Gemlik {} / {}; Gemlik root {:#x}", mn.pages.len(), mn.widgets.len(), mg.pages.len(), mg.widgets.len(), mg.addrs.root);
    assert_eq!((mg.pages.len(), mg.widgets.len()), (mn.pages.len(), mn.widgets.len()));
    assert_ne!(mg.addrs.root, mn.addrs.root, "Gemlik's .data is laid out differently");
    // Every widget of the Gemlik tree dispatches on the same labels as its Novalis twin.
    for (wn, wg) in mn.widgets.values().zip(mg.widgets.values()) {
        assert_eq!((wn.update, wn.draw, wn.enter, wn.leave), (wg.update, wg.draw, wg.enter, wg.leave), "widget {:#x} / {:#x}", wn.addr, wg.addr);
        assert_eq!(std::mem::discriminant(&wn.data), std::mem::discriminant(&wg.data));
    }
    // The level / planet names and the galaxy points are engine tables (same values, other addresses).
    assert_eq!((&mg.level_names, &mg.planet_points, mg.name_dy, mg.consts), (&mn.level_names, &mn.planet_points, mn.name_dy, mn.consts));
    // The Port Options page attaches to Gemlik's Options page.
    let (mut an, mut ag) = (mn.clone(), mg.clone());
    assert!(an.install_port_page(&n) && ag.install_port_page(&g));
    // Quick select: the gp block and the d-pad defaults.
    assert_eq!(QsConsts::load(&g), QsConsts::load(&n));
    let (tn, tg) = (QsTables::load(&n).unwrap(), QsTables::load(&g).unwrap());
    assert_eq!((tn.entries, tn.defaults), (tg.entries, tg.defaults));
    // The frame mobys' corner joint lists (0x161fe0 on Novalis).
    let ids = |o: &Overlay| (0..4u32).map(|k| o.i32(o.at(frame::CORNER_LISTS_ADDR) + 4 * k)).collect::<Vec<_>>();
    assert_eq!(ids(&g), vec![Some(0), Some(1), Some(2), Some(3)]);
    assert_eq!(ids(&n), ids(&g));
}

#[test]
fn every_level_loads_the_pause_menu() {
    let Some(n) = menus(1) else { eprintln!("skipped: no extracted/"); return };
    let mn = PageMenu::load(&n).unwrap();
    for level in 0..19 {
        let ov = menus(level).unwrap();
        let m = PageMenu::load(&ov).unwrap_or_else(|| panic!("level {level:02}: no page menu"));
        assert_eq!((m.pages.len(), m.widgets.len()), (mn.pages.len(), mn.widgets.len()), "level {level:02}");
        let mut m2 = m.clone();
        assert!(m2.install_port_page(&ov), "level {level:02}: Port Options page");
        assert!(QsTables::load(&ov).is_some() && QsConsts::load(&ov).is_some(), "level {level:02}: quick select");
    }
}

/// The Goodies descriptions `0x29aa60` rewrites (items 5..8): the level's own description table 0x1b6df8 must be
/// the one the page's label reads, so the unlock texts replace the disc's `0xffffffff` placeholders on every level
/// (the label's id is a level-local address, so it has to be mapped like the other `DATA_LABELS`).
#[test]
fn every_level_resolves_the_goodies_descriptions() {
    let Some(n) = menus(1) else { eprintln!("skipped: no extracted/"); return };
    let root = rc_formats::test_data::root();
    let (elf, save) = (std::fs::read(root.join("boot/SCUS_971.99")).unwrap(), std::fs::read(root.join("global/save_game.bin")).unwrap());
    let template = SaveGameLump::parse(&save).unwrap().template;
    let mut gs = GameState::new_game(ChunkTables::from_boot_elf(&elf).unwrap(), &template).unwrap();
    // 16 skill points: Sketchbook unlocked (0x4fd3), the rest locked (0x4fda / 0x4fdb).
    gs.global.skill_points[..16].fill(1);
    // The Goodies page's description label: stride 4, its id table a list of message ids.
    let description_label = |ov: &Overlay, m: &PageMenu| -> u32 {
        let gp = ov.at(media::label::GOODIES_PAGE);
        m.pages[&gp]
            .widgets
            .iter()
            .filter_map(|w| m.widgets.get(w))
            .find_map(|w| match &w.data {
                Data::Label(l) if l.stride == 4 && ov.u32(l.id).is_some_and(|v| (0x4f00..0x5100).contains(&v)) => Some(l.id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no Goodies description label"))
    };
    let n1 = PageMenu::load(&n).unwrap();
    let l1 = description_label(&n, &n1);
    assert_eq!((l1, n1.media.goodies_texts), (0x1b6df8, 0x1b6df8), "level 01 is the reference layout");
    for level in 0..19 {
        let ov = menus(level).unwrap();
        let mut m = PageMenu::load(&ov).unwrap();
        let label = description_label(&ov, &m);
        assert_eq!(m.media.goodies_texts, label, "level {level:02}: the description table the label reads");
        media::goodies_unlocks(&mut m, &gs);
        let t = m.label_tables.get(&label).unwrap_or_else(|| panic!("level {level:02}: no patched description table"));
        assert_eq!(&t[5..9], &[0x4fd3, 0x4fda, 0x4fdb, 0x4fdb], "level {level:02}");
    }
}

/// Gemlik's content gap (a queue for later, not a check): the placed classes whose update is not ported, with
/// their instances, update address, sequence / joint counts and a kind guess (no class blob → controller / fx;
/// ≥ 8 sequences and ≥ 8 joints → creature; else prop). `cargo xtask test-job --test ui --filter
/// gemlik_generalisation::gemlik_content_gap --ignored --nocapture`.
#[test]
#[ignore = "survey: prints, asserts nothing; run with --ignored --nocapture"]
fn gemlik_content_gap() {
    let (Some(b), Some(gp), Some(core)) = (bytes(13), rc_formats::test_data::gameplay(13), rc_formats::test_data::core(13)) else { eprintln!("skipped: no extracted/"); return };
    let ov = |l: u32| bytes(l).map(|b| Arc::new(LevelOverlay::parse(&b).unwrap()));
    let p = LevelPorts::from_overlays(&LevelOverlay::parse(&b).unwrap(), &ov, &[]);
    let inst = rc_formats::gameplay::parse_moby_instances(&gp).unwrap();
    let mut by: std::collections::BTreeMap<i16, usize> = Default::default();
    for m in &inst { *by.entry(m.o_class as i16).or_default() += 1; }
    let mut total = 0;
    for (oc, n) in by {
        let Some(f) = p.level_update(oc).filter(|&f| f != 0 && p.get(oc).is_none()) else { continue };
        let c = core.block(&format!("moby_class/{oc:04}")).and_then(|b| rc_formats::moby::parse_moby_class(b).ok().map(|c| (rc_formats::moby_anim::parse_sequences(b, &c).map(|s| s.len()).unwrap_or(0), c.header.joint_count)));
        let kind = match c { None => "controller/fx", Some((s, j)) if s >= 8 && j >= 8 => "creature", Some(_) => "prop" };
        eprintln!("class {oc:5}  {n:4} placed  update {f:#08x}  {c:?}  {kind}");
        total += n;
    }
    eprintln!("{total} unported instances");
}
