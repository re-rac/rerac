//! The per-level class registry (`moby_update::classes::LevelPorts`) on the 19 level overlays: the level's class
//! table decides which classes run a port (docs/plan/level_generalisation.md C1). Skipped without `extracted/`.

use rc_formats::level_overlay::LevelOverlay;
use rc_game::moby_update::classes::{for_class, ClassUpdate, LevelPorts};
use std::sync::Arc;

fn overlay(level: u32) -> Option<Arc<LevelOverlay>> { crate::common::overlay(level) }

pub fn ports(level: u32) -> Option<LevelPorts> { crate::common::ports(level, &[0x2bd100, 0x2fd0e8]) }

#[test]
fn novalis_ports_are_the_class_number_registry() {
    let (Some(ov), Some(p), Some(gp)) = (overlay(1), ports(1), rc_formats::test_data::gameplay(1)) else { eprintln!("skipped: no extracted/"); return };
    let placed: std::collections::HashSet<i16> = rc_formats::gameplay::parse_moby_instances(&gp).unwrap().iter().map(|m| m.o_class as i16).collect();
    for e in ov.vtbl() {
        let oc = e.o_class as i16;
        if for_class(oc).is_none() && ov.code(e.update, 2).is_some_and(rc_game::moby_update::classes::units::empty::is_empty) {
            // An update that is `jr ra; nop` and no port of its own (433, 1289: worn head items; 531: a ship; 618):
            // the empty-update unit, as on every level (`units::empty`).
            assert_eq!(p.get(oc).map(|u| u.address()), Some(rc_game::moby_update::classes::units::empty::UPDATE_FN), "level 01 class {oc}");
            continue;
        }
        if oc == 731 {
            // The third mission NPC class: the table runs MissionNpcUpdate for it too (not placed on Novalis).
            assert!(!placed.contains(&oc));
            assert_eq!(p.get(oc), Some(ClassUpdate::MissionNpc));
            continue;
        }
        if oc == 1143 {
            // The Gadgetron logo (the vendor's hologram, created, not placed): its update is the same code as Kalebo's
            // placed logos, the unit port `units::hologram_logo` (U514), found by its code on every level.
            assert!(!placed.contains(&oc));
            let row = rc_game::moby_update::classes::units::PORTS.iter().position(|u| u.unit == "U514 1143").unwrap() as u16;
            assert_eq!(p.get(oc), Some(ClassUpdate::Unit(row)));
            continue;
        }
        if oc == 530 {
            // Ratchet's Veldin ship (not placed on Novalis): level01 0x2ecde0 is the same code as level00's, the unit
            // port `units::veldin_ship` (U24), found by its code.
            assert!(!placed.contains(&oc));
            let row = rc_game::moby_update::classes::units::PORTS.iter().position(|u| u.unit == "U24 530").unwrap() as u16;
            assert_eq!(p.get(oc), Some(ClassUpdate::Unit(row)));
            continue;
        }
        assert_eq!(p.get(oc), for_class(oc), "level 01 class {oc} (update {:#x})", e.update);
    }
}

#[test]
fn every_level_runs_the_ports_its_class_table_names() {
    let Some(_) = overlay(1) else { eprintln!("skipped: no extracted/"); return };
    for level in 0..19 {
        let ov = overlay(level).unwrap();
        let p = ports(level).unwrap();
        let mut lost = Vec::new();
        let mut gained = Vec::new();
        for e in ov.vtbl() {
            let oc = e.o_class as i16;
            match (for_class(oc), p.get(oc)) {
                (Some(a), None) => lost.push((oc, a, format!("{:#x}", e.update))),
                (None, Some(b)) => gained.push((oc, b)),
                (Some(a), Some(b)) => assert_eq!(a, b, "level {level:02} class {oc}"),
                (None, None) => {}
            }
        }
        eprintln!("level {level:02}: {} classes, {} unported; by number but other code here {lost:?}; same code, other class number {gained:?}", ov.vtbl().len(), p.unported().len());
    }
    // Gemlik: its body pieces run FxGroupUpdate, the vendor, bombs and teleporters their ports.
    let p = ports(13).unwrap();
    for oc in [1733, 1734, 1735, 1801, 1802, 1803, 1804] { assert_eq!(p.get(oc), Some(ClassUpdate::FxPiece), "class {oc}"); }
    // The class-27 particle emitters (the engine's external update) are Novalis's own; the ripple manager 751 is a
    // water port (`rc_game::water::managers`), also only on Novalis.
    for level in 0..19 {
        let p = ports(level).unwrap();
        assert_eq!(p.external(27), (level == 1).then_some(0x2bd100), "level {level:02}");
        assert_eq!(p.in_table(751) && p.get(751) == Some(ClassUpdate::Water(0)), level == 1, "level {level:02}");
    }
    for (oc, u) in [(11, ClassUpdate::Vendor), (121, ClassUpdate::Bomb), (1135, ClassUpdate::TeleporterPad), (500, ClassUpdate::Crate), (758, ClassUpdate::SwingTarget), (803, ClassUpdate::SwingTarget)] {
        assert_eq!(p.get(oc), Some(u), "class {oc}");
    }
}
