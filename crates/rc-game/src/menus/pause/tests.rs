use super::*;
use crate::game_state::GameState;
use rc_formats::save_game::ChunkTables;

const OPTIONS: u32 = 0x1b4dd0;
const SOUND: u32 = 0x1b5a18;
const OPT_LIST: u32 = 0x1b4f78;
const SOUND_W: u32 = 0x1b5af8;
const PLIST: u32 = 0x1b6738;
const ROOT_W: [u32; 7] = [0x1b2b38, 0x1b2b88, 0x1b2bd8, 0x1b2c28, 0x1b2c78, 0x1b2cc8, 0x1b2d18];

fn gs() -> GameState { GameState::zeroed(ChunkTables { global: vec![], level: vec![] }) }

fn list(addr: u32, flags: u32, items: Vec<Item>, above: u32, below: u32, enter: u32, draw: u32) -> Widget {
    let data = Data::List(List { flags, items_addr: 0, items, above, below, cursor: 0, scroll: 0 });
    Widget { addr, update: func::LIST_UPDATE, draw, enter, leave: 0, dflags: 0, moby: 0, rect: [0, 0, 160, 30], raw: [0; 8], data }
}

fn item(label: i16, action: i16, arg: u32) -> Item { Item { label, action, arg, sublabel: 0, hl: 0 } }

fn pg(addr: u32, kind: i32, parent: u32, focus: u32, widgets: &[(usize, u32)], seq0: i32) -> Page {
    let mut w = [0u32; 14];
    for &(i, a) in widgets { w[i] = a; }
    Page { addr, seqs: std::array::from_fn(|i| seq0 + i as i32), parent, kind, focus, widgets: w, pending: 0 }
}

/// A synthetic tree with the disc's shape: the root's 7 one-item lists (Options → an Options page whose list
/// leads to the Sound page) and the planet select page.
fn menu() -> PageMenu {
    let mut m = PageMenu {
        consts: MenuConsts::default(),
        addrs: Addrs::default(),
        pages: BTreeMap::new(),
        widgets: BTreeMap::new(),
        kind: 0,
        current: 0,
        target: 0,
        post: 0,
        trans: 0,
        ticks: 0,
        prev: 0,
        goodies: false,
        no_close: false,
        arg: 0,
        return_page: 0,
        return_kind: 0,
        dest: 0,
        mobys: [(0, false); 14],
        stub_calls: BTreeMap::new(),
        active: false,
        frames: None,
        enter_b13f4: 0,
        level_names: (0..20u32).map(|l| (20153 + l, 20172 + l)).collect(),
        planet_points: vec![[0; 4]; 19],
        name_dy: 17,
        equip: [0; 4],
        items: None,
        unusable_head: false,
        unusable_back: false,
        view: super::gadgets::GadgetsView::default(),
        view_model: None,
        text_swap: 0,
        label_tables: BTreeMap::new(),
        help_items: Default::default(),
        movie_lists: Vec::new(),
        log_ids: Vec::new(),
        lang: 0,
        ammo_records: Vec::new(),
        gold_spin: None,
        map: Default::default(),
        media: Default::default(),
        saves: Default::default(),
    };
    let labels = [20194, 20195, 20196, 20197, 20198, 20199, 20418];
    for (i, &a) in ROOT_W.iter().enumerate() {
        let above = if i == 0 { 0 } else { ROOT_W[i - 1] };
        let below = if i == 5 { 0 } else if i == 6 { ROOT_W[0] } else { ROOT_W[i + 1] };
        let above = if i == 6 { ROOT_W[5] } else { above };
        let arg = if i == 5 { OPTIONS } else { 0x1b3000 + i as u32 };
        let enter = if i < 3 { func::ROOT_ENTER } else { 0 };
        m.widgets.insert(a, list(a, lf::LARGE, vec![item(labels[i], 3, arg)], above, below, enter, func::LIST_DRAW));
    }
    let root_ws: Vec<(usize, u32)> = ROOT_W.iter().copied().enumerate().collect();
    m.pages.insert(page::ROOT, pg(page::ROOT, 2, 0, ROOT_W[0], &root_ws, 14));
    let opts = vec![item(20254, 3, 0x1b5040), item(20258, 3, SOUND), item(20262, 3, 0x1b6060)];
    m.widgets.insert(OPT_LIST, list(OPT_LIST, lf::WRAP, opts, 0, 0, 0, func::LIST_DRAW));
    m.pages.insert(OPTIONS, pg(OPTIONS, 8, page::ROOT, OPT_LIST, &[(3, OPT_LIST)], 67));
    m.widgets.insert(SOUND_W, Widget { addr: SOUND_W, update: func::SOUND_UPDATE, draw: func::SOUND_DRAW, enter: 0, leave: 0, dflags: 0, moby: 0, rect: [0; 4], raw: [0; 8], data: Data::Sound { cursor: 0 } });
    m.pages.insert(SOUND, pg(SOUND, 0x20, OPTIONS, SOUND_W, &[(3, SOUND_W)], 118));
    m.widgets.insert(PLIST, list(PLIST, 0x139, vec![], 0, 0, func::PLANET_LIST_ENTER, func::PLANET_LIST_DRAW));
    m.pages.insert(page::PLANET_SELECT, pg(page::PLANET_SELECT, 0xf, 0, PLIST, &[(1, PLIST)], 99));
    m
}

fn press(bits: u32) -> MenuInput { MenuInput { pressed_u: bits, raw_pressed: bits, held_u: bits, connected: true, ..Default::default() } }
fn hold(bits: u32) -> MenuInput { MenuInput { held_u: bits, connected: true, ..Default::default() } }
const NONE: MenuInput = MenuInput {
    held: 0, pressed: 0, raw_pressed: 0, raw_released: 0, held_u: 0, pressed_u: 0,
    stick_x: crate::ps2v::Pf::ZERO, stick_y: crate::ps2v::Pf::ZERO, stick_active: false, connected: true, sticks: [crate::ps2v::Pf::ZERO; 4],
};

fn open(m: &mut PageMenu, g: &mut GameState, kind: i32) -> Vec<MenuOut> {
    m.enter(kind, g);
    (0..13).map(|_| m.tick(&NONE, g, &MenuEnv::default())).collect()
}

fn list_of(m: &PageMenu, w: u32) -> &List {
    match &m.widgets[&w].data {
        Data::List(l) => l,
        _ => panic!("not a list"),
    }
}

#[test]
fn open_transition_is_12_raw_ticks_with_the_seqs_reversed() {
    let (mut m, mut g) = (menu(), gs());
    m.enter(0, &g);
    let o = m.tick(&NONE, &mut g, &MenuEnv::default());
    assert_eq!(o.sounds, vec![MenuSound::Open]);
    assert_eq!((m.kind, m.trans, m.current), (1, 12, 0));
    assert!(m.mobys.iter().all(|&(_, back)| back), "opening plays the page's seqs reversed");
    assert_eq!(m.mobys[0].0, 14);
    for t in 2..=12 {
        m.tick(&NONE, &mut g, &MenuEnv::default());
        assert_eq!(m.kind, 1, "tick {t}");
    }
    let o = m.tick(&NONE, &mut g, &MenuEnv::default());
    assert_eq!((m.kind, m.current, o.entered), (2, page::ROOT, Some(page::ROOT)));
    // ✕ on Options (after moving the focus there) → forward transition to the Options page.
    m.pages.get_mut(&page::ROOT).unwrap().focus = ROOT_W[5];
    let o = m.tick(&press(button::CROSS), &mut g, &MenuEnv::default());
    assert_eq!(m.target, OPTIONS);
    assert!(o.sounds.is_empty());
    let o = m.tick(&NONE, &mut g, &MenuEnv::default());
    assert_eq!(o.sounds, vec![MenuSound::PageChange]);
    assert!(m.mobys.iter().all(|&(_, back)| !back) && m.mobys[0].0 == 67, "forward: the target's seqs from frame 0");
    for _ in 0..12 { m.tick(&NONE, &mut g, &MenuEnv::default()); }
    assert_eq!((m.kind, m.current), (8, OPTIONS));
    // △ → back to the parent: the current page's seqs reversed.
    m.tick(&press(button::TRIANGLE), &mut g, &MenuEnv::default());
    let o = m.tick(&NONE, &mut g, &MenuEnv::default());
    assert_eq!(o.sounds, vec![MenuSound::PageChange]);
    assert!(m.mobys.iter().all(|&(_, back)| back) && m.mobys[0].0 == 67);
}

#[test]
fn highlight_timer_and_focus_moves() {
    let (mut m, mut g) = (menu(), gs());
    open(&mut m, &mut g, 0);
    let s = 10;
    for _ in 0..30 { m.tick(&NONE, &mut g, &MenuEnv::default()); }
    let hl0 = list_of(&m, ROOT_W[0]).items[0].hl;
    assert_eq!(hl0, 31, "+1 per tick while focused (the enter tick included), no clamp");
    assert_eq!(hl_colour(hl0 as i32, s, LIGHT_BLUE, YELLOW), YELLOW);
    assert_eq!(hl_colour(0, s, LIGHT_BLUE, YELLOW), LIGHT_BLUE);
    assert_eq!(hl_colour(5, s, LIGHT_BLUE, YELLOW), tween(0.5, LIGHT_BLUE, YELLOW));
    // Down on a one-item list: the page's pending focus = below (Gadgets), applied the same tick, sound 1.
    let o = m.tick(&press(button::DOWN), &mut g, &MenuEnv::default());
    assert_eq!(o.sounds, vec![MenuSound::Cursor]);
    assert_eq!(m.pages[&page::ROOT].focus, ROOT_W[1]);
    // The old item was still focused during that tick's update (+1); then min(t, 10) − 1 per tick.
    let seq: Vec<i16> = (0..12).map(|_| {
        m.tick(&NONE, &mut g, &MenuEnv::default());
        list_of(&m, ROOT_W[0]).items[0].hl
    }).collect();
    assert_eq!(seq, vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 0]);
    // Up from Gadgets → Weapons again; Up from Weapons (Goodies locked) → Options.
    m.tick(&press(button::UP), &mut g, &MenuEnv::default());
    assert_eq!(m.pages[&page::ROOT].focus, ROOT_W[0]);
    m.tick(&press(button::UP), &mut g, &MenuEnv::default());
    assert_eq!(m.pages[&page::ROOT].focus, ROOT_W[5]);
}

#[test]
fn goodies_wiring_both_ways() {
    let (mut m, mut g) = (menu(), gs());
    m.enter(0, &g);
    assert!(!m.goodies);
    assert_eq!((list_of(&m, wiring::WEAPONS).above, list_of(&m, wiring::OPTIONS).below), (wiring::OPTIONS, wiring::WEAPONS));
    g.global.game_beaten = 1;
    m.enter(0, &g);
    assert!(m.goodies);
    assert_eq!((list_of(&m, wiring::WEAPONS).above, list_of(&m, wiring::OPTIONS).below), (wiring::GOODIES, wiring::GOODIES));
    g.global.game_beaten = 0;
    g.global.completes = 1;
    m.enter(0, &g);
    assert!(m.goodies, "challenge mode counts too");
}

#[test]
fn root_enter_disables_the_first_three_when_0x1413f4_is_1() {
    let (mut m, mut g) = (menu(), gs());
    m.enter(0, &g);
    let env = MenuEnv { b13f4: 1, ..Default::default() };
    for _ in 0..13 { m.tick(&NONE, &mut g, &env); }
    let actions: Vec<i16> = ROOT_W.iter().map(|&w| list_of(&m, w).items[0].action).collect();
    assert_eq!(actions, vec![0, 0, 0, 3, 3, 3, 3]);
    // ✕ on a disabled entry does nothing.
    m.tick(&press(button::CROSS), &mut g, &env);
    assert_eq!(m.target, 0);
}

#[test]
fn close_needs_10_menu_ticks_then_2_ticks_of_kind_0x14() {
    let (mut m, mut g) = (menu(), gs());
    m.enter(0, &g);
    m.tick(&NONE, &mut g, &MenuEnv::default());
    // Start while the transition runs does nothing (no widget update); after it, Start closes.
    for _ in 0..12 { m.tick(&NONE, &mut g, &MenuEnv::default()); }
    let o = m.tick(&press(button::START), &mut g, &MenuEnv::default());
    assert_eq!((m.post, m.kind, m.trans), (1, 0x14, 2));
    assert!(o.exit.is_none());
    assert!(m.tick(&NONE, &mut g, &MenuEnv::default()).exit.is_none());
    assert_eq!(m.tick(&NONE, &mut g, &MenuEnv::default()).exit, Some(PostAction::Resume));
    assert!(!m.active);
    // A close request before 10 menu ticks waits for tick 10.
    let (mut m, mut g) = (menu(), gs());
    m.enter(0, &g);
    m.ticks = 0;
    m.kind = 2;
    m.current = page::ROOT;
    m.tick(&press(button::START), &mut g, &MenuEnv::default());
    assert_eq!((m.post, m.kind), (1, 2));
    for _ in 0..8 { m.tick(&NONE, &mut g, &MenuEnv::default()); }
    assert_eq!((m.ticks, m.kind), (9, 2));
    m.tick(&NONE, &mut g, &MenuEnv::default());
    assert_eq!((m.ticks, m.kind), (10, 0x14), "closes on menu tick 10");
}

#[test]
fn sound_slider_full_range_takes_342_ticks() {
    let (mut m, mut g) = (menu(), gs());
    m.enter(0, &g);
    m.kind = 0x20;
    m.current = SOUND;
    g.global.effects_volume = 0;
    let mut n = 0;
    while g.global.effects_volume < 0x400 {
        m.tick(&hold(button::RIGHT), &mut g, &MenuEnv::default());
        n += 1;
        assert!(n < 1000);
    }
    assert_eq!(n, 342);
    let mut n = 0;
    while g.global.effects_volume > 0 {
        m.tick(&hold(button::LEFT), &mut g, &MenuEnv::default());
        n += 1;
    }
    assert_eq!(n, 342);
    // Down → music row; ✕ on row 2 toggles stereo.
    m.tick(&press(button::DOWN), &mut g, &MenuEnv::default());
    m.tick(&hold(button::RIGHT), &mut g, &MenuEnv::default());
    assert_eq!((g.global.effects_volume, g.global.music_volume), (0, 3));
    m.tick(&press(button::DOWN), &mut g, &MenuEnv::default());
    let o = m.tick(&press(button::CROSS), &mut g, &MenuEnv::default());
    assert_eq!((g.global.stereo, o.sound_settings), (1, true));
    // Up wraps 0 → 2.
    m.tick(&press(button::DOWN), &mut g, &MenuEnv::default());
    m.tick(&press(button::UP), &mut g, &MenuEnv::default());
    assert!(matches!(m.widgets[&SOUND_W].data, Data::Sound { cursor: 2 }));
}

#[test]
fn planet_list_in_acquisition_order() {
    let (mut m, mut g) = (menu(), gs());
    g.global.level = 3;
    g.global.map_order[..4].copy_from_slice(&[1, 3, 4, 2]);
    let outs = open(&mut m, &mut g, 0xe);
    assert_eq!(outs[0].sounds, vec![MenuSound::Open]);
    assert_eq!((m.kind, m.current), (0xf, page::PLANET_SELECT));
    let l = list_of(&m, PLIST);
    let labels: Vec<(i16, i16, i16)> = l.items.iter().map(|it| (it.label, it.sublabel, it.action)).collect();
    assert_eq!(labels, vec![(20154, 20173, 1), (20156, 20175, 1), (20157, 20176, 1), (20155, 20174, 1)]);
    assert_eq!(l.cursor, 1, "the cursor starts on the current planet (dest = level 3)");
    assert!(l.flags & lf::JUMP_SCROLL != 0);
    // Buttons only (flag 1): Down from the pressed-edge mask of the buttons; the cursor writes 0x184894.
    let o = m.tick(&press(button::DOWN), &mut g, &MenuEnv::default());
    assert_eq!((list_of(&m, PLIST).cursor, m.dest), (2, 4));
    assert_eq!(o.sounds, vec![MenuSound::Cursor]);
    // L1 steps back too (flag 0x100); Start closes with dest = the current level and post-action 2.
    m.tick(&press(button::L1), &mut g, &MenuEnv::default());
    assert_eq!(m.dest, 3);
    m.tick(&press(button::START), &mut g, &MenuEnv::default());
    assert_eq!((m.post, m.kind), (2, 0x14));
    m.tick(&NONE, &mut g, &MenuEnv::default());
    assert_eq!(m.tick(&NONE, &mut g, &MenuEnv::default()).exit, Some(PostAction::ShipTravel(3)));
}

#[test]
fn blink_cadence_22_on_8_off() {
    let on: Vec<bool> = (0..60).map(planet_select::blink_on).collect();
    assert!(on[..22].iter().all(|&b| b) && on[22..30].iter().all(|&b| !b));
    assert_eq!(&on[30..60], &on[..30]);
    let mut g = gs();
    g.global.planet_unlocked[2] = 1;
    g.levels[1].visited = 1;
    assert_eq!((planet_select::planet_state(&g, 1), planet_select::planet_state(&g, 2), planet_select::planet_state(&g, 3)), (3, 2, 0));
}

/// The disc's page tree (skipped without `extracted/`).
#[test]
fn overlay_page_tree() {
    let p = rc_formats::test_data::root().join("levels/01/overlay.bin");
    let Ok(bytes) = std::fs::read(&p) else { eprintln!("skipped: no {}", p.display()); return };
    let m = PageMenu::load(&Overlay::parse(&bytes).unwrap()).unwrap();
    let root = &m.pages[&page::ROOT];
    assert_eq!((root.kind, root.parent, root.focus, root.seqs[0], root.seqs[7]), (2, 0, ROOT_W[0], 14, 7));
    let labels: Vec<i16> = ROOT_W.iter().map(|w| list_of(&m, *w).items[0].label).collect();
    assert_eq!(labels, vec![20194, 20195, 20196, 20197, 20198, 20199, 20418]);
    let opts: Vec<(i16, i16)> = list_of(&m, OPT_LIST).items.iter().map(|i| (i.label, i.action)).collect();
    assert_eq!(opts, vec![(20254, 3), (20255, 4), (20256, 5), (20258, 3), (20259, 3), (20260, 3), (20262, 3)]);
    assert_eq!(list_of(&m, OPT_LIST).flags, lf::WRAP);
    assert!(matches!(&m.widgets[&0x1b5160].data, Data::Toggle(t) if t.entries.len() == 2 && t.entries[0].var == 0x15ee1c));
    assert!(matches!(&m.widgets[&0x1b6020].data, Data::Camera(c) if c.entries.len() == 3 && c.entries[2].var == 0x15ede4));
    assert_eq!(m.pages[&SOUND].kind, 0x20);
    assert_eq!(m.pages[&page::PLANET_SELECT].focus, PLIST);
    assert_eq!(list_of(&m, PLIST).flags, 0x139);
    assert_eq!(m.consts, MenuConsts::default());
    assert_eq!((m.level_names[1], m.planet_points[0], m.name_dy), ((20154, 20173), [80, 95, 20, -20], 17));
}

#[test]
fn port_page_from_the_disc() {
    use super::port::{self, Setting};
    let p = rc_formats::test_data::root().join("levels/01/overlay.bin");
    let Ok(bytes) = std::fs::read(&p) else { eprintln!("skipped: no {}", p.display()); return };
    let ov = Overlay::parse(&bytes).unwrap();
    let mut m = PageMenu::load(&ov).unwrap();
    let before = (m.pages.clone(), m.widgets.clone());
    assert_eq!(m.port_value(Setting::Msaa), None);
    assert!(m.install_port_page(&ov));
    assert!(m.install_port_page(&ov), "idempotent");
    // Every disc record but the Options list and its description label is untouched.
    for (a, w) in &before.1 {
        if *a != OPT_LIST && *a != port::OPTIONS_LABEL { assert_eq!(&m.widgets[a], w, "{a:#x}"); }
    }
    for (a, pg) in &before.0 { assert_eq!(&m.pages[a], pg, "{a:#x}"); }
    // The entry sits before Quit Game; the description table follows it.
    let opts: Vec<(i16, i16)> = list_of(&m, OPT_LIST).items.iter().map(|i| (i.label, i.action)).collect();
    assert_eq!(opts, vec![(20254, 3), (20255, 4), (20256, 5), (20258, 3), (20259, 3), (20260, 3), (port::text::ENTRY as i16, 3), (20262, 3)]);
    let Data::Label(l) = &m.widgets[&port::OPTIONS_LABEL].data else { panic!() };
    assert_eq!(l.table.as_deref(), Some(&[20273, 20274, 20275, 20277, 20278, 20279, port::text::DESCRIPTION as u32, 20281][..]));
    // The page (three rows): Camera's frames, filler and hint widgets, parent Options.
    let (pg, sub) = (&m.pages[&port::PAGE], m.pages[&port::MODEL_MANY].clone());
    assert_eq!((pg.seqs, pg.parent, pg.focus), (sub.seqs, port::OPTIONS, port::LIST_W));
    assert_eq!((pg.widgets[1], pg.widgets[2], pg.widgets[4]), (sub.widgets[1], sub.widgets[2], sub.widgets[4]));
    assert!(matches!(&m.widgets[&port::TITLE_W].data, Data::Label(t) if t.flags == 0xf && t.id == port::text::TITLE as u32));

    // Pause → Options → Up twice (wrap to Quit Game, then Port Options) → ✕ → the page; ✕ cycles; △ back.
    let mut g = gs();
    let env = MenuEnv::default();
    open(&mut m, &mut g, 0);
    m.pages.get_mut(&page::ROOT).unwrap().focus = ROOT_W[5];
    m.tick(&press(button::CROSS), &mut g, &env);
    for _ in 0..13 { m.tick(&NONE, &mut g, &env); }
    assert_eq!(m.current, OPTIONS);
    m.tick(&press(button::UP), &mut g, &env);
    m.tick(&press(button::UP), &mut g, &env);
    assert_eq!(list_of(&m, OPT_LIST).cursor, 6);
    m.tick(&press(button::CROSS), &mut g, &env);
    let o = m.tick(&NONE, &mut g, &env);
    assert_eq!(o.sounds, vec![MenuSound::PageChange]);
    assert!(m.mobys.iter().all(|&(_, back)| !back) && m.mobys[0].0 == sub.seqs[0]);
    for _ in 0..12 { m.tick(&NONE, &mut g, &env); }
    assert_eq!((m.current, m.kind), (port::PAGE, port::KIND));
    m.set_port_value(Setting::Msaa, 9);
    assert_eq!(m.port_value(Setting::Msaa), Some(3), "clamped");
    m.set_port_value(Setting::Msaa, 1);
    let o = m.tick(&press(button::CROSS), &mut g, &env);
    assert_eq!((o.sounds, m.port_value(Setting::Msaa)), (vec![MenuSound::Confirm], Some(2)));
    m.tick(&press(button::CROSS), &mut g, &env);
    m.tick(&press(button::CROSS), &mut g, &env);
    assert_eq!(m.port_value(Setting::Msaa), Some(0), "wraps after 8x");
    // The shadows row (default on = value 0): Down moves to it, ✕ toggles off and back on.
    assert_eq!(m.port_value(Setting::Shadows), Some(0));
    let o = m.tick(&press(button::DOWN), &mut g, &env);
    assert_eq!(o.sounds, vec![MenuSound::Cursor]);
    m.tick(&press(button::CROSS), &mut g, &env);
    assert_eq!((m.port_value(Setting::Shadows), m.port_value(Setting::Msaa)), (Some(1), Some(0)));
    m.tick(&press(button::CROSS), &mut g, &env);
    assert_eq!(m.port_value(Setting::Shadows), Some(0), "on / off wrap");
    // The strafe row (default off = value 0): Down moves to it, ✕ turns it on; Down stops at the last row.
    assert_eq!(m.port_value(Setting::Strafe), Some(0));
    let o = m.tick(&press(button::DOWN), &mut g, &env);
    assert_eq!(o.sounds, vec![MenuSound::Cursor]);
    let o = m.tick(&press(button::DOWN), &mut g, &env);
    assert!(o.sounds.is_empty(), "last row: no cursor move");
    m.tick(&press(button::CROSS), &mut g, &env);
    assert_eq!((m.port_value(Setting::Strafe), m.port_value(Setting::Shadows)), (Some(1), Some(0)));
    // Its values are the game's own on / off strings (the Subtitles toggle's).
    let Data::Toggle(sub_t) = &m.widgets[&0x1b5160].data else { panic!() };
    assert_eq!((sub_t.entries[0].on, sub_t.entries[0].off), (port::text::ON as u32, port::text::OFF as u32));
    m.tick(&press(button::TRIANGLE), &mut g, &env);
    assert_eq!(m.target, port::OPTIONS);
}
