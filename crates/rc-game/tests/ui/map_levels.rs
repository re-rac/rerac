//! The in-game map on the disc's data (docs/plan/menus.md §13), headless: the 38 map files, the overlay tables on all
//! 19 levels, the fog mask's making / writing / packing, the zone rules, and the map page (compose, draw, keys, pan and
//! zoom, the Map-o-Matic switch). Skipped when `extracted/` is absent.

use rc_formats::save_game::{ChunkTables, ItemTables, SaveGameLump};
use rc_game::game_state::{GameState, SessionState};
use rc_game::map::{self, predicates, FogInput, MapFile, MapState, Mask, Tables};
use rc_game::menus::pause::map_page::{self, HeroMark, Loader};
use rc_game::menus::pause::{MenuEnv, PageMenu};
use rc_game::menus::{MenuDraw, MenuInput, Overlay, QuadTex};
use rc_game::pad::button;
use rc_game::ps2v::Pf;
use std::sync::Arc;

fn root() -> std::path::PathBuf { rc_formats::test_data::root() }

fn map_file(i: usize) -> Option<MapFile> {
    let raw = std::fs::read(root().join(format!("global/unknown_0820/{i:03}.bin"))).ok()?;
    let b = if rc_formats::wad::is_wad(&raw) { rc_formats::wad::decompress(&raw).ok()? } else { raw };
    MapFile::parse(b)
}

fn overlay(l: u32) -> Option<Vec<u8>> { std::fs::read(root().join(format!("levels/{l:02}/overlay.bin"))).ok() }

fn tables(l: u32) -> Option<Tables> {
    let (t, r) = (overlay(l)?, overlay(1)?);
    Tables::read(&Overlay::relocated(&t, &r).ok()?)
}

/// Every map file parses; the zone tiles decode to exactly their 1024 pixels; the Map-o-Matic set keeps the plain
/// set's zones and runs (only the pictures differ).
#[test]
fn map_files_parse() {
    let Some(f1) = map_file(1) else { eprintln!("skipped: no extracted/"); return };
    for i in 0..38 {
        let f = map_file(i).unwrap_or_else(|| panic!("map file {i}"));
        for t in 0..256 {
            let d = &f.bytes[f.hdr[0] + 8..f.hdr[1]];
            let e = |k: usize| u16::from_le_bytes([d[2 * k], d[2 * k + 1]]) as usize;
            let (start, end) = (if t == 0 { 0x200 } else { e(t - 1) }, e(t));
            let runs = (end - start) * 2 / 3;
            let mut n = 0;
            for k in 0..runs {
                let p = 3 * k;
                let (b0, b1) = (d[start + p / 2], d[start + p / 2 + 1]);
                let len = if p & 1 == 0 { ((b1 as usize & 0xf) << 4) | (b0 >> 4) as usize } else { b1 as usize };
                n += if len == 0 { 256 } else { len };
            }
            assert_eq!(n, 1024, "file {i} tile {t}");
        }
        if i >= map::LEVELS {
            let p = map_file(i - map::LEVELS).unwrap();
            assert_eq!(f.runs(), p.runs(), "file {i}: runs");
            assert_eq!(Mask::initial(&f), Mask::initial(&p), "file {i}: zones");
        }
    }
    // Novalis: most of the map is in a zone; its pictures are 512 × 512 with a palette.
    let m = Mask::initial(&f1);
    assert!(m.fogged_count() > 40_000 && m.fogged_count() < 200_000, "{m:?}");
    assert_eq!(f1.pif(0).map(|p| (p.width, p.height)), Some((128, 128)));
}

/// The overlay tables (transforms 0x182c90, zones 0x183020, pans 0x184370, the predicate slots 0x184410) are the same
/// data in all 19 overlays; the port's predicate list has a function exactly where the game's table does.
#[test]
fn tables_on_every_level() {
    let Some(t1) = tables(1) else { eprintln!("skipped: no extracted/"); return };
    // Novalis' transform from its reference points (61.2, 202.6) → (77.5, 189), (291, 59.5) → (454.5, 420).
    let t = t1.transforms[1];
    let (u, v) = t.to_map(61.2, 202.6);
    assert!((u * 512.0 - 77.5).abs() < 0.01 && (v * 512.0 - 189.0).abs() < 0.01, "{u} {v}");
    assert_eq!(t1.zones[1][7], map::Zone { z_min: 39.5, z_max: 42.5, flags: 0x102, arg: 0 });
    let r = overlay(1).unwrap();
    let ov1 = Overlay::parse(&r).unwrap();
    for l in 0..19u32 {
        let Some(t) = tables(l) else { continue };
        assert_eq!(t.zones, t1.zones, "level {l}");
        assert_eq!(t.pans, t1.pans, "level {l}");
        let ov = Overlay::relocated(&overlay(l).unwrap(), &r).unwrap();
        let base = ov.at(map::PREDICATES);
        for lv in 0..19usize {
            for k in 0..8usize {
                let p = ov.u32(base + 0x20 * lv as u32 + 4 * k as u32).unwrap();
                assert_eq!(p != 0, predicates::address(lv, k).is_some(), "overlay {l}: level {lv} predicate {k}");
                if l == 1 { assert_eq!(Some(p).filter(|&p| p != 0), predicates::address(lv, k)); }
            }
        }
    }
    // The mission and marker lists: the same records in every overlay (the callbacks are code addresses of each
    // overlay, compared by presence).
    let t1m = map_page::MapTables::read(&ov1);
    for l in 0..19u32 {
        let Some(o) = overlay(l) else { continue };
        let t = map_page::MapTables::read(&Overlay::relocated(&o, &r).unwrap());
        assert_eq!(t.markers, t1m.markers, "overlay {l} markers");
        assert_eq!(t.sizes, t1m.sizes, "overlay {l} sizes");
        assert_eq!(t.prices, t1m.prices, "overlay {l} prices");
        type Key = (i16, i16, u16, bool, u32, [i16; 2]);
        let strip = |ms: &Vec<Vec<map_page::Mission>>| -> Vec<Vec<Key>> {
            ms.iter().map(|v| v.iter().map(|m| (m.name, m.desc, m.flags, m.cb != 0, m.arg, [m.req[0].0, m.req[1].0])).collect()).collect()
        };
        assert_eq!(strip(&t.missions), strip(&t1m.missions), "overlay {l} missions");
    }
    // Every callback the level-01 lists name is ported.
    for m in t1m.missions.iter().flatten() {
        for a in [m.cb].into_iter().chain(m.req.iter().filter(|r| r.0 == 7).map(|r| r.1)).filter(|&a| a != 0) {
            assert!((0x262b40..0x262da0).contains(&a), "callback {a:#x}");
        }
    }
}

fn state(level: u32, owned_mapomatic: bool, saved: &[u8]) -> Option<MapState> {
    let t = tables(level)?;
    Some(MapState::enter(&t, level as i32, owned_mapomatic, map_file(map::file_index(level as usize, owned_mapomatic)), saved))
}

fn fog_at(s: &MapState, x: f32, y: f32, z: f32) -> FogInput { FogInput { pos: [x, y, z], level: s.level, alt: false, group: 0, f0634: 0, magnetic: 0 } }

/// `FUN_0025c4f8` on Novalis: at the landing point the circle of radius 16 about the hero's pixel + 1 opens (only zone
/// pixels, only inside the brush), nothing beyond; a second call reveals nothing new; walking on reveals more.
#[test]
fn fog_writer_reveals_the_brush() {
    let Some(mut s) = state(1, false, &[]) else { eprintln!("skipped: no extracted/"); return };
    let before = s.mask.clone();
    let (x, y, z) = (162.5, 136.4, 60.5);
    let n = s.reveal(&fog_at(&s, x, y, z), &[0; 128]);
    assert!(n > 300, "{n}");
    let (u, v) = s.transforms[1].to_map(x, y);
    let (hx, hy) = map::pixel(u, v);
    let (cx, cy) = (hx + 1, hy + 1);
    for py in 0..512i32 {
        for px in 0..512i32 {
            let (a, b) = (before.get(px as usize, py as usize), s.mask.get(px as usize, py as usize));
            if a == b { continue; }
            assert!(a && !b, "only reveals");
            let (bx, by) = (px - cx + 16, py - cy + 16);
            assert!((0..32).contains(&bx) && (0..32).contains(&by) && map::brush_open(bx as usize, by as usize), "({px}, {py}) outside the brush");
        }
    }
    assert_eq!(s.reveal(&fog_at(&s, x, y, z), &[0; 128]), 0);
    assert!(s.reveal(&fog_at(&s, x + 12.0, y, z), &[0; 128]) > 0);
    // Movement group 22 / state 50 are the caller's skip (engine); off the map nothing happens.
    assert_eq!(s.reveal(&fog_at(&s, -5000.0, -5000.0, z), &[0; 128]), 0);
}

/// The zone rules: Novalis zone 7 (z 39.5..42.5, flags 0x102 = not riding + predicate 0 `side(211, 219, 297, 249)`),
/// zone 14 (z 0..5000, flag 4 = movement group 0x10 only); the z range of zone 2 (59.5..62).
#[test]
fn zone_rules() {
    let Some(s) = state(1, false, &[]) else { eprintln!("skipped: no extracted/"); return };
    let flags = [0u8; 128];
    let mut inp = fog_at(&s, 0.0, 0.0, 40.0);
    // Predicate 0 holds on the negative side of (211, 219) → (297, 249): e.g. pixel (250, 100), not (250, 300).
    let open = s.zones_open(&inp, &flags, 250, 100);
    assert!(open[7] && !open[14] && !open[2]);
    assert!(!s.zones_open(&inp, &flags, 250, 300)[7], "predicate 0 fails");
    inp.group = 0x11;
    assert!(!s.zones_open(&inp, &flags, 250, 100)[7], "riding");
    inp.group = 0x10;
    assert!(s.zones_open(&inp, &flags, 250, 100)[14]);
    inp.pos[2] = 60.0;
    assert!(s.zones_open(&inp, &flags, 250, 100)[2] && !s.zones_open(&inp, &flags, 250, 100)[7]);
    assert!(s.zones_open(&inp, &flags, 0, 0)[0], "zone 0 always");
}

/// The saved mask: the run coding round-trips every pixel of the run table (the rest reads back clear); a mask too
/// irregular for 0x800 bytes falls back to the 4 × 4 downsample (bit 0 of byte 0 clear, bit 1 set), which reads back
/// as blocks; the level entry takes a saved mask back.
#[test]
fn pack_and_unpack() {
    let (Some(f), Some(mut s)) = (map_file(1), state(1, false, &[])) else { eprintln!("skipped: no extracted/"); return };
    for k in 0..8 { s.reveal(&fog_at(&s, 120.0 + 12.0 * k as f32, 136.4, 60.5), &[0; 128]); }
    let (saved, n) = s.mask.pack(f.runs());
    assert!(n > 0 && saved[0] & 1 == 1, "run coding: {n} bytes");
    assert_eq!(Mask::unpack(&saved, f.runs()), s.mask, "every fogged pixel is in the run table");
    let again = state(1, false, &saved).unwrap();
    assert_eq!(again.mask, s.mask);
    // A checkerboard over the zones does not fit.
    let mut noisy = Mask::initial(&f);
    for y in 0..512 {
        for x in 0..512 {
            if (x + y) % 2 == 0 && noisy.get(x, y) { noisy.set(x, y, false); }
        }
    }
    let (coarse, n) = noisy.pack(f.runs());
    assert_eq!((n, coarse[0] & 3), (-1, 2));
    let back = Mask::unpack(&coarse, f.runs());
    for (by, bx) in [(10usize, 3usize), (60, 60), (127, 127)] {
        let v = back.get(bx * 4, by * 4);
        for dy in 0..4 {
            for dx in 0..4 { assert_eq!(back.get(bx * 4 + dx, by * 4 + dy), v); }
        }
    }
    // No map: all zero.
    assert_eq!(MapState::default().pack(), [0; map::SAVE_BYTES]);
}

struct Disc {
    elf: Vec<u8>,
    ov0: Vec<u8>,
    ov1: Vec<u8>,
    template: Vec<u8>,
}

fn first_arrival() -> Option<(GameState, Disc)> {
    let d = Disc {
        elf: std::fs::read(root().join("boot/SCUS_971.99")).ok()?,
        ov0: overlay(0)?,
        ov1: overlay(1)?,
        template: SaveGameLump::parse(&std::fs::read(root().join("global/save_game.bin")).ok()?).ok()?.template,
    };
    let t1 = ItemTables::load(&d.elf, &d.ov1).unwrap();
    let mut gs = GameState::new_game(ChunkTables::from_boot_elf(&d.elf).unwrap(), &d.template).unwrap();
    let mut s = SessionState::default();
    gs.apply_level_start(0, &ItemTables::load(&d.elf, &d.ov0).unwrap(), &mut s);
    gs.on_veldin_clank_init(&mut s);
    gs.apply_transition(1);
    gs.apply_level_start(1, &t1, &mut s);
    Some((gs, d))
}

fn frame(m: &mut PageMenu, gs: &mut GameState, pressed: u32, sticks: [f32; 4], f: &mut u32) {
    let inp = MenuInput { pressed, pressed_u: pressed, raw_pressed: pressed, connected: true, sticks: sticks.map(Pf::f), ..Default::default() };
    let env = MenuEnv { vsync: *f, b13f4: 0, pal: false };
    *f += 1;
    m.tick(&inp, gs, &env);
}

/// The map page on Novalis: Select opens kind 10 → page 0x1b3998; the first update composes the current level with the
/// live mask (the revealed circle shows the revealed picture); the draw has the grid, the picture and the hero arrow;
/// the sticks pan and zoom within the limits; R1 steps to the next unlocked planet and composes it (the zone tiles'
/// mask: not visited); the Map-o-Matic picked up switches the current level's set at the next open.
#[test]
fn map_page_on_novalis() {
    let Some((mut gs, d)) = first_arrival() else { eprintln!("skipped: no extracted/"); return };
    let ov = Overlay::parse(&d.ov1).unwrap();
    let Some(mut m) = PageMenu::load(&ov) else { return };
    // The frame mobys place the panels (as the engine: class 1138 from the level core).
    let blob = rc_formats::test_data::core_block(1, "moby_class/1138").unwrap();
    let class = rc_formats::moby::parse_moby_class(&blob).unwrap();
    let anim = rc_formats::moby_anim::MobyAnimClass::new(&class, rc_formats::moby_anim::parse_sequences(&blob, &class).unwrap());
    use rc_game::menus::pause::frame::{FrameClass, FrameMobys, CORNER_LISTS_ADDR};
    let chains = std::array::from_fn(|k| rc_formats::gadget::joint_list(&blob, &class.header, ov.i32(CORNER_LISTS_ADDR + 4 * k as u32).unwrap() as usize).unwrap().0);
    m.frames = Some(FrameMobys::new(FrameClass { anim, chains, scale: class.header.scale }, false));
    let mut st = state(1, false, &[]).unwrap();
    st.reveal(&fog_at(&st, 162.5, 136.4, 60.5), &[0; 128]);
    let live = st.mask.clone();
    m.map.state = st;
    m.map.loader = Loader(Some(Arc::new(map_file)));
    m.map.hero = HeroMark { pos: [162.5, 136.4, 60.5], yaw: 0.5, group: 0 };
    m.enter(10, &gs);
    let mut f = 0;
    for _ in 0..16 { frame(&mut m, &mut gs, 0, [0.0; 4], &mut f); }
    assert_eq!((m.current, m.map.shown, m.map.available), (0x1b3998, 1, true));
    let pic = m.map.picture.clone().expect("the composed picture");
    let file = map_file(1).unwrap();
    let pal = rc_formats::hud::decode_indexed8_raw(&map::compose(&file, &live), 512, 512, file.palette()).unwrap();
    assert_eq!(pic.rgba, pal.rgba, "composed with the live mask");
    // FUN_00262da0(level, 1) at the open: the view on the hero, Novalis' markers shown by their missions' status.
    let (u, v) = m.map.state.transforms[1].to_map(162.5, 136.4);
    assert_eq!(m.map.state.view[1].pan, [((u * 4096.0) as i32) << 16, ((v * 4096.0) as i32) << 16]);
    assert_eq!(m.map.markers.len(), 4);
    for k in &m.map.markers {
        let want = k.link == -1 || m.map.missions.get(k.link as usize).is_some_and(|x| x.status == 1);
        assert_eq!(k.shown, want, "{k:?}");
    }
    assert!(m.map.markers.iter().any(|k| k.shown), "an open mission on the first arrival");
    let a = rc_game::menus::MenuAssets::new(rc_game::hud::HudAssets { icons: Vec::new(), frame_sizes: Vec::new(), glyphs: rc_formats::font::parse_glyph_tables(&d.ov1).unwrap().0, messages: Vec::new() }, ov.clone());
    let mut draws = Vec::new();
    m.draw(&a, &gs, &MenuEnv::default(), &mut rc_game::rng::Rng::new(), &mut draws);
    let quads: Vec<&QuadTex> = draws.iter().filter_map(|d| match d { MenuDraw::Quad { tex, .. } => Some(tex), _ => None }).collect();
    let images = quads.iter().filter(|t| matches!(t, QuadTex::Image(_))).count();
    assert_eq!(images, 4, "the map picture and the globe's three layers");
    // The globe's scroll moves 1/16 texel a frame (GS UV precision), not a whole texel every 16 frames.
    let globe_u = |vsync: u32| -> Vec<i32> {
        let mut out = Vec::new();
        for &w in m.widgets.keys() { rc_game::menus::pause::map_page::globe_draw(&m, w, vsync, &mut out); }
        out.iter().filter_map(|d| match d { MenuDraw::Quad { uv, uv16: true, repeat: true, .. } => Some(uv[0][0]), _ => None }).collect()
    };
    let (u0, u1) = (globe_u(100), globe_u(101));
    assert!(!u0.is_empty(), "the planet layer scrolls");
    assert_eq!(u1.iter().zip(&u0).map(|(b, a)| b - a).collect::<Vec<_>>(), vec![1; u0.len()], "{u0:?} {u1:?}");
    let shown = m.map.markers.iter().filter(|k| k.shown && k.icon != 0).count();
    assert!(quads.iter().filter(|t| matches!(t, QuadTex::Frame(_))).count() >= 2 + shown, "grid, markers and arrow");
    let labelled = m.map.markers.iter().filter(|k| k.shown && k.label != 0).count();
    let frames = draws.iter().filter(|d| matches!(d, MenuDraw::Hud(rc_game::hud::Draw::UiFrame { .. }))).count();
    assert_eq!(frames, labelled, "a label box per shown labelled marker");
    // Pan / zoom: the left stick moves the view, the right stick zooms, within the limits.
    let v0 = m.map.state.view[1];
    frame(&mut m, &mut gs, 0, [0.0, -1.0, 1.0, 0.0], &mut f);
    let v1 = m.map.state.view[1];
    assert!((v1.zoom - v0.zoom * 1.02).abs() < 1e-6 && v1.pan[0] > v0.pan[0], "{v0:?} {v1:?}");
    for _ in 0..400 { frame(&mut m, &mut gs, 0, [0.0, -1.0, 1.0, 1.0], &mut f); }
    let v = m.map.state.view[1];
    assert_eq!((v.zoom, v.pan[0]), (4.0, 0x1000_0000));
    assert_eq!(v.pan[1], 0x1000_0000 - (1280.0f32 / 4.0) as i32 * 0x8000);
    // R1: the next planet in the unlock order, composed from its zone tiles (not visited).
    gs.global.map_order[..3].copy_from_slice(&[1, 2, 0]);
    frame(&mut m, &mut gs, button::R1, [0.0; 4], &mut f);
    assert_eq!(m.map.shown, 2);
    let f2 = map_file(2).unwrap();
    let want = rc_formats::hud::decode_indexed8_raw(&map::compose(&f2, &Mask::initial(&f2)), 512, 512, f2.palette()).unwrap();
    assert_eq!(m.map.picture.as_ref().unwrap().rgba, want.rgba);
    // Close (Select), take the Map-o-Matic, open again: the current level's picture from file 20.
    frame(&mut m, &mut gs, button::SELECT, [0.0; 4], &mut f);
    for _ in 0..4 { frame(&mut m, &mut gs, 0, [0.0; 4], &mut f); }
    assert!(!m.active);
    gs.global.owned[map::MAP_O_MATIC] = 1;
    m.enter(10, &gs);
    for _ in 0..16 { frame(&mut m, &mut gs, 0, [0.0; 4], &mut f); }
    assert!(m.map.state.map_o_matic);
    let f20 = map_file(20).unwrap();
    let want = rc_formats::hud::decode_indexed8_raw(&map::compose(&f20, &live), 512, 512, f20.palette()).unwrap();
    assert_eq!((m.map.shown, &m.map.picture.as_ref().unwrap().rgba), (1, &want.rgba));
    let _ = map_page::GRID_ICON;
}
