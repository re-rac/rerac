//! The memory-card pages of the page menu: the Save page (Options → Save), the Load page (Options → Load and the front
//! end's Load Game), the New Game page (the front end) and the challenge-mode page (after the credits), their overwrite
//! confirmation pages, the slot list and the slot's planet label, and the end-of-game page's choice (challenge mode or
//! the time warp). Spec: docs/plan/progression.md `## saves`; the card itself is [`crate::memcard`]; the dialogs of mode 4
//! are [`crate::menus::freeze`].
//!
//! The records are the disc's (L01): Save 0x1b5b48 (confirm 0x1b6af8), Load 0x1b5bd0, the front end's New Game 0x1b9208
//! (confirm 0x1b93b8) and Load Game 0x1b9180, challenge mode 0x1b9440 (confirm 0x1b9518). Every slot widget keeps its
//! state in its own record: +0x40 the cursor, +0x4c the save step (0 idle, 1 save now, 2 done), +0x30 flag 0x2000 =
//! challenge mode.
//!
//! ## Coverage
//! | address | what | port |
//! |---|---|---|
//! | 0x296920 / 0x296960 (boot 0x222f18 / 0x222f58) | the slot widgets' enter / leave: a stream buffer for the whole save (+0x48), step 0 | [`slots_enter`] (the buffer is the card's) |
//! | 0x296990 `SavingDataMenu` | focus only; step 0 → 1 when the confirm page 0x1b6af8 was left with "yes"; step 1: `fun_002269c0` (capture + whole save into the slot), busy 0x1ba298 with "Saving Data" 0x4fb5; busy and the card idle: an error → flag 0x80 + `mode_freezeInit(3, page)`, else the slot's preview; Start/Select/R3 close; △ parent (or close); card not ready (status ∉ {1, 0x10}) → back to the parent; card idle and type 2: Up/Down move the cursor 0..4 (0x15ee34), ✕ on an empty slot saves at once, else the confirm page (0x1ba244 = 0, 0x1b6bc0 = slot); cursor move → sound 1 | [`save_update`] |
//! | 0x296ce0 `LoadingDataMenu` | busy and idle: an error → flag 0x100 + the dialog; else checked 0x13d384 = 1, the mixer from the loaded options, `FUN_002a29a0(level)` (the saved level, no story trip: 0x13e05a = 0); keys as above on 0x13cb04; ✕ on a used slot: sound 0, card 0, slot, request 0xd, busy "Loading Data" 0x4fb6 | [`load_update`] |
//! | 0x296fc0 `SavingDataMenu2` | the New Game / challenge-mode page: step 1 after the confirm page 0x1b93b8 / 0x1b9518; step 1 → `fun_00226a70` (new game into the slot) or `fun_00226b08` (challenge, flag 0x2000); busy and idle: error → flag 0x80 + dialog; else the preview and `FUN_002a29a0(0)` with the story trip 0x13e05a = 1; ✕ empty slot → save, used slot → the confirm page (0x1ba244 = 1 / 2); ○: continue without saving (0x13d384 = 0, flags &= ~6, the new-game reset, level 0, 0x13e05a = 1) | [`new_game_update`] |
//! | 0x2973e8 (boot 0x2239e0) | the slot list: 5 rows from y 4, step 0x4b (PAL 0x54); the cursor row (card idle) framed 0x8020ffff + navy, every row 0x80303030; idle and type 2: "EMPTY" 21015 or the play time `%02d:%02d` (hours ≤ 99) after icon 0xe99e·3, times completed (icon ·4, ≤ 99) when ≠ 0, the bolts (`%d` / `%d,%03d` / `%d,%03d,%03d`, ≤ 9999999, icon 0x754f·15), the save date `%02x/%02x/%02x` (month, day, year; icon 0xe99e·2); the cursor row yellow | [`slots_draw`] |
//! | 0x2967a0 (boot 0x222d98) | the focused slot's level and planet names (0x1c22c0 + 12·level, +4) at y 4 / 0x14, or "EMPTY" centred | [`slot_info_draw`] |
//! | 0x2954c0 | the confirm page: card not ready → parent; ○ → 0x1ba244 = 0, parent, page +0x84 = 1; △ → parent, +0x84 = 0 | [`confirm_update`] |
//! | 0x295450 / 0x295490 | its enter (+0x84 = 0, a stream buffer) / leave | [`confirm_enter`] |
//! | 0x295558 | its text: 0x1ba244 0..2 → 20403, 3 → 20405, else "(error)"; small font, centred lines, centred block, shadow at +1 | [`confirm_draw`] |
//! | 0x2937d0 / 0x293d50 flag 0x100 | the slot's picture (the planet of the saved level) and, while the card works, "Reading Memory Card (PS2)" 20417 or the busy message | `pages::image_update` / `pages::image_draw` |
//! | 0x295c98 `process_global_state_flags` (the choice: `media::stats_update`) | ✕ the time warp's state: `memcard_RestoreGame(0x1ba250)` keeping level 18's gold bolts 0x14bf08 and skill point 0x13d425, game beaten 0x15eea0 = 1 (then the engine: `memcard_Save(0, −1)`, the checkpoint records cleared, `FadeToBlack(16)`, the death flag); ○ without a card: `fun_00226b08(−1)` ([`MemCard::challenge_save`]), level 0, 0x13e05a = 1 | [`time_warp`] |
//! | 0x28e600 actions 4 / 5 | Save / Load / New Game / Load Game: sound 0; card ready → the page; else flag 2 / 4 and `mode_freezeInit(3, page)` | `PageMenu::list_update` |
//! | 0x28e600 action 9 | the language 0x15ed88 = arg (no sound) | `PageMenu::list_update` ([`super::MenuOut::language`]) |
//! | 0x28dbb8 | the front end's Options list: 0x1b8dc0 (NTSC: Language, Sound) or 0x1b8d90 (PAL: + Video) | [`front_options_enter`] |
//! | 0x28c990 (boot 0x2192a8) | a changed card (flag 1) holds the menu in `mode_freezeInit(3, page)` | `PageMenu::tick` |

use super::{Data, MenuOut, PageMenu};
use crate::game_state::GameState;
use crate::hud::{text as wtext, Draw, Rot};
use crate::memcard::{MemCard, SaveCapture, SLOTS, TYPE_PS2};
use crate::menus::{text, MenuAssets, MenuDraw, MenuInput, MenuSound, Overlay};
use crate::pad::button;
use rc_formats::font::Font;
use std::collections::BTreeMap;

/// Callbacks by address (level01).
pub mod func {
    pub const SAVE_UPDATE: u32 = 0x296990;
    pub const LOAD_UPDATE: u32 = 0x296ce0;
    pub const NEW_GAME_UPDATE: u32 = 0x296fc0;
    pub const SLOTS_DRAW: u32 = 0x2973e8;
    pub const SLOTS_ENTER: u32 = 0x296920;
    pub const SLOTS_LEAVE: u32 = 0x296960;
    pub const SLOT_INFO_DRAW: u32 = 0x2967a0;
    pub const CONFIRM_UPDATE: u32 = 0x2954c0;
    pub const CONFIRM_DRAW: u32 = 0x295558;
    pub const CONFIRM_ENTER: u32 = 0x295450;
    pub const CONFIRM_LEAVE: u32 = 0x295490;
    pub const FRONT_OPTIONS_ENTER: u32 = 0x28dbb8;
}

/// Pages the code names (level01).
pub mod page {
    pub const SAVE: u32 = 0x1b5b48;
    pub const LOAD: u32 = 0x1b5bd0;
    pub const SAVE_CONFIRM: u32 = 0x1b6af8;
    pub const NEW_GAME: u32 = 0x1b9208;
    pub const NEW_GAME_CONFIRM: u32 = 0x1b93b8;
    pub const LOAD_GAME: u32 = 0x1b9180;
    pub const CHALLENGE: u32 = 0x1b9440;
    pub const CHALLENGE_CONFIRM: u32 = 0x1b9518;
    /// The pages the menu's load walks besides the trees (every confirm page is named by code only).
    pub const ROOTS: [u32; 8] = [SAVE, LOAD, SAVE_CONFIRM, NEW_GAME, NEW_GAME_CONFIRM, LOAD_GAME, CHALLENGE, CHALLENGE_CONFIRM];
}

/// Data the code names (level01): the front end's Options lists, the challenge mode's kept items.
pub mod data {
    pub const FRONT_OPTIONS_NTSC: u32 = 0x1b8dc0;
    pub const FRONT_OPTIONS_PAL: u32 = 0x1b8d90;
    /// `0x1ba120`: the items whose owned flag challenge mode keeps (s32, −1 terminated).
    pub const CHALLENGE_ITEMS: u32 = 0x1ba120;
}

/// Messages.
pub const MSG_SAVING: i32 = 0x4fb5;
pub const MSG_LOADING: i32 = 0x4fb6;
pub const MSG_READING: i32 = 0x4fb9;
pub const MSG_OVERWRITE: i32 = 0x4fb3;
pub const MSG_EMPTY: i32 = 0x5217;
/// The slot list's icons.
pub const MENU_ICON: u16 = 0xe99e;
pub const BOLT_ICON: u16 = 0x754f;
/// Colours.
const YELLOW: u32 = 0x8020_ffff;
const LIGHT_BLUE: u32 = 0x80ff_a888;
const GREY: u32 = 0x8030_3030;

/// The memory-card pages' globals and the card.
#[derive(Clone, Debug)]
pub struct SaveMenu {
    /// The card (the engine keeps it and moves it in while the menu runs, as the map state).
    pub card: MemCard,
    /// 0x1ba298 / 0x1ba29c: a save or load is running, and its message.
    pub busy: bool,
    pub busy_msg: i32,
    /// 0x1ba244: the confirm page's question (0 save, 1 new game, 2 challenge, 3 saving).
    pub confirm_kind: i32,
    /// 0x1b6bc0: the slot the confirm page asks about.
    pub confirm_slot: i32,
    /// Page +0x84: the confirm page was left with "yes".
    pub confirmed: BTreeMap<u32, bool>,
    /// 0x15ee34: the last slot (shared by the pages, kept for the session).
    pub last_slot: i32,
    /// The clock the next save stamps (the engine's `sceCdReadClock`).
    pub clock: [u8; 8],
    /// The level ranges 0x1c4938 (landmark capture).
    pub landmark_base: Vec<i32>,
    /// 0x1ba250: the ending buffer (the whole save class 1750 makes before the last boss), for the time warp.
    pub ending: Option<Vec<u8>>,
    /// The front end's Options lists (NTSC, PAL) and the challenge mode's kept items (read at load).
    pub front_options: [Vec<super::Item>; 2],
    pub challenge_items: Vec<usize>,
    /// The new-game reset `load_and_initialize_level_chunk`: the disc template (from the card's lump).
    pub template: Option<Vec<u8>>,
}

impl Default for SaveMenu {
    fn default() -> Self {
        SaveMenu {
            card: MemCard::absent(),
            busy: false,
            busy_msg: 0,
            confirm_kind: 0,
            confirm_slot: 0,
            confirmed: BTreeMap::new(),
            last_slot: 0,
            clock: [0; 8],
            landmark_base: Vec::new(),
            ending: None,
            front_options: [Vec::new(), Vec::new()],
            challenge_items: Vec::new(),
            template: None,
        }
    }
}

impl SaveMenu {
    /// Reads the front end's lists and the challenge items from the overlay.
    pub fn read(ov: &Overlay) -> SaveMenu {
        let items = |a: u32| super::read_items(ov, ov.at(a));
        let mut challenge_items = Vec::new();
        let base = ov.at(data::CHALLENGE_ITEMS);
        for k in 0..64u32 {
            let Some(v) = ov.i32(base + 4 * k) else { break };
            if v == -1 { break; }
            if let Ok(i) = usize::try_from(v) { challenge_items.push(i); }
        }
        SaveMenu { front_options: [items(data::FRONT_OPTIONS_NTSC), items(data::FRONT_OPTIONS_PAL)], challenge_items, ..Default::default() }
    }
}

/// `load_and_initialize_level_chunk` (L01 0x25f640): the template restored (`RestoreGame`), level 0.
pub fn reset_game(gs: &mut GameState, template: Option<&[u8]>) {
    let Some(t) = template else { return };
    if let Ok(n) = GameState::new_game(gs.tables.clone(), t) { *gs = n; }
}

fn raw(m: &PageMenu, w: u32, i: usize) -> i32 { m.widgets.get(&w).map_or(0, |x| x.raw[i] as i32) }
fn set_raw(m: &mut PageMenu, w: u32, i: usize, v: i32) {
    if let Some(x) = m.widgets.get_mut(&w) { x.raw[i] = v as u32; }
}
fn focus(m: &PageMenu) -> u32 { m.pages.get(&m.current).map_or(0, |p| p.focus) }
fn parent(m: &PageMenu) -> u32 { m.pages.get(&m.current).map_or(0, |p| p.parent) }

/// The slot widgets' enter `fun_00222f18`: step 0 (the stream buffer is the card's whole-save buffer).
pub fn slots_enter(m: &mut PageMenu, w: u32) { set_raw(m, w, 7, 0); }

/// The slot's preview after a save: `*(slot·0x1c + 0x13d2b0..)` = level, bolts, completes, elapsed, clock.
fn note_preview(card: &mut MemCard, gs: &GameState) {
    if let Some(p) = usize::try_from(card.card.slot).ok().and_then(|s| card.card.previews.get_mut(s)) {
        p.bolts = gs.global.bolts;
        p.level = gs.global.level;
        p.elapsed = gs.global.elapsed;
        p.clock = gs.global.save_clock;
        p.completes = gs.global.completes;
    }
}

/// The keys every slot page runs first: Start/Select/R3 close (1), △ the parent or close (−1). `Some(ret)` = done.
fn page_keys(m: &mut PageMenu, inp: &MenuInput) -> Option<i32> {
    if inp.pressed_u & 0xd00 != 0 && !m.no_close { return Some(1); }
    if inp.pressed_u & button::TRIANGLE != 0 {
        let p = parent(m);
        if p != 0 {
            m.target = p;
        } else if !m.no_close {
            return Some(-1);
        }
    }
    None
}

/// The card gate: status ∉ {1, 0x10} → back to the parent; busy or not a PS2 card → nothing this frame.
fn card_gate(m: &mut PageMenu) -> bool {
    if !m.saves.card.ready() {
        m.target = parent(m);
        return false;
    }
    m.saves.card.idle() && m.saves.card.card.ty == TYPE_PS2
}

/// Writes the slot a confirm page asks about into its slot list's cursor (0x1b6bc0 is the SAVE_CONFIRM page's
/// slot widget 0x1b6b80 + 0x40; the new-game / challenge confirm pages share their parent's slot widget, whose
/// cursor already is the slot). The slot list draw highlights that cursor ([`slots_draw`]).
fn set_confirm_slot(m: &mut PageMenu, page: u32, slot: i32) {
    let ws = m.pages.get(&page).map_or([0; 14], |p| p.widgets);
    for w in ws {
        if w != 0 && m.widgets.get(&w).is_some_and(|wd| wd.draw == func::SLOTS_DRAW) { set_raw(m, w, 4, slot); }
    }
}

/// The cursor keys (0x15ee34 shared): Up above 0, Down below 4.
fn cursor_keys(m: &mut PageMenu, w: u32, keys: u32) -> i32 {
    let last = m.saves.last_slot;
    let mut c = last;
    if keys & button::UP != 0 && last != 0 { c = last - 1; }
    if keys & button::DOWN != 0 && c < SLOTS as i32 - 1 { c += 1; }
    m.saves.last_slot = c;
    set_raw(m, w, 4, c);
    c
}

fn keys_of(m: &PageMenu, w: u32, inp: &MenuInput, pressed: u32) -> u32 { if raw(m, w, 0) & 1 != 0 { inp.raw_pressed } else { pressed } }

/// The Save page's slot list `SavingDataMenu` (0x296990).
pub fn save_update(m: &mut PageMenu, w: u32, inp: &MenuInput, gs: &mut GameState, out: &mut MenuOut) -> i32 {
    if focus(m) != w { return 0; }
    if raw(m, w, 7) == 0 && m.prev == m.addrs_saves(page::SAVE_CONFIRM) && m.saves.confirmed.get(&m.addrs_saves(page::SAVE_CONFIRM)).copied().unwrap_or(false) {
        set_raw(m, w, 7, 1);
    }
    if raw(m, w, 7) == 1 {
        let cap = capture(m);
        let slot = raw(m, w, 4);
        m.saves.card.save_whole(gs, slot, &cap);
        m.saves.busy_msg = MSG_SAVING;
        m.saves.busy = true;
    }
    set_raw(m, w, 7, 2);
    let before = raw(m, w, 4);
    if m.saves.busy {
        if !m.saves.card.idle() { return 0; }
        m.saves.busy = false;
        if m.saves.card.error != 0 {
            m.saves.card.flags |= 0x80;
            out.freeze = Some(m.current);
            return 0;
        }
        note_preview(&mut m.saves.card, gs);
    }
    if let Some(r) = page_keys(m, inp) { return r; }
    if !card_gate(m) { return 0; }
    let keys = keys_of(m, w, inp, inp.pressed);
    let c = cursor_keys(m, w, keys);
    if keys & button::CROSS != 0 && m.saves.card.card.ty == TYPE_PS2 {
        if m.saves.card.card.previews[c as usize].level == -1 {
            set_raw(m, w, 7, 1);
        } else {
            m.saves.confirm_kind = 0;
            m.target = m.addrs_saves(page::SAVE_CONFIRM);
            m.saves.confirm_slot = raw(m, w, 4);
            set_confirm_slot(m, m.target, m.saves.confirm_slot);
        }
    }
    if raw(m, w, 4) != before { out.sounds.push(MenuSound::Cursor); }
    0
}

/// The Load pages' slot list `LoadingDataMenu` (0x296ce0).
pub fn load_update(m: &mut PageMenu, w: u32, inp: &MenuInput, gs: &mut GameState, out: &mut MenuOut) -> i32 {
    let before = raw(m, w, 4);
    if m.saves.busy {
        if !m.saves.card.idle() { return 0; }
        m.saves.busy = false;
        if m.saves.card.error != 0 {
            m.saves.card.flags |= 0x100;
            out.freeze = Some(m.current);
            return 0;
        }
        m.saves.card.checked = true;
        // 0x13e598.. = the mixer from the loaded options (sfx·8/10, music, sfx·7/10, sfx).
        out.sound_settings = true;
        out.level_exit = Some(gs.global.level);
        out.story = Some(false);
    }
    if let Some(r) = page_keys(m, inp) { return r; }
    if !card_gate(m) { return 0; }
    let keys = keys_of(m, w, inp, inp.pressed_u);
    let c = cursor_keys(m, w, keys);
    if keys & button::CROSS != 0 && m.saves.card.card.ty == TYPE_PS2 && m.saves.card.card.previews[c as usize].level >= 0 {
        out.sounds.push(MenuSound::Confirm);
        m.saves.card.load(raw(m, w, 4));
        m.saves.busy = true;
        m.saves.busy_msg = MSG_LOADING;
    }
    if raw(m, w, 4) != before { out.sounds.push(MenuSound::Cursor); }
    0
}

/// The New Game / challenge-mode slot list `SavingDataMenu2` (0x296fc0).
pub fn new_game_update(m: &mut PageMenu, w: u32, inp: &MenuInput, gs: &mut GameState, out: &mut MenuOut) -> i32 {
    if focus(m) != w { return 0; }
    let before = raw(m, w, 4);
    let challenge = raw(m, w, 0) & 0x2000 != 0;
    if raw(m, w, 7) == 0 {
        let (ng, ch) = (m.addrs_saves(page::NEW_GAME_CONFIRM), m.addrs_saves(page::CHALLENGE_CONFIRM));
        let yes = |p: u32| m.saves.confirmed.get(&p).copied().unwrap_or(false);
        if (m.prev == ng && yes(ng)) || (m.prev == ch && yes(ch)) { set_raw(m, w, 7, 1); }
    }
    if raw(m, w, 7) == 1 {
        let slot = raw(m, w, 4);
        let (clock, template) = (m.saves.clock, m.saves.template.clone());
        if !challenge {
            m.saves.card.new_game_save(gs, slot, clock, |g| reset_game(g, template.as_deref()));
        } else {
            let keep = m.saves.challenge_items.clone();
            m.saves.card.challenge_save(gs, slot, clock, &keep, |g| reset_game(g, template.as_deref()));
        }
        m.saves.busy = true;
        m.saves.busy_msg = MSG_SAVING;
    }
    set_raw(m, w, 7, 2);
    if m.saves.busy {
        if !m.saves.card.idle() { return 0; }
        m.saves.busy = false;
        if m.saves.card.error != 0 {
            m.saves.card.flags |= 0x80;
            out.freeze = Some(m.current);
            return 0;
        }
        note_preview(&mut m.saves.card, gs);
        out.level_exit = Some(0);
        out.story = Some(true);
    }
    if let Some(r) = page_keys(m, inp) { return r; }
    if !card_gate(m) { return 0; }
    let keys = keys_of(m, w, inp, inp.pressed);
    let c = cursor_keys(m, w, keys);
    if keys & button::CROSS == 0 || m.saves.card.card.ty != TYPE_PS2 {
        if keys & button::CIRCLE != 0 {
            // Continue without saving.
            m.saves.card.checked = false;
            m.saves.card.flags &= !6;
            let t = m.saves.template.clone();
            reset_game(gs, t.as_deref());
            out.level_exit = Some(0);
        out.story = Some(true);
        }
    } else if m.saves.card.card.previews[c as usize].level == -1 {
        set_raw(m, w, 7, 1);
    } else {
        m.target = m.addrs_saves(if challenge { page::CHALLENGE_CONFIRM } else { page::NEW_GAME_CONFIRM });
        m.saves.confirm_kind = if challenge { 2 } else { 1 };
        m.saves.confirm_slot = raw(m, w, 4);
    }
    if raw(m, w, 4) != before { out.sounds.push(MenuSound::Cursor); }
    0
}

/// The confirm page's update (0x2954c0).
pub fn confirm_update(m: &mut PageMenu, inp: &MenuInput) -> i32 {
    let p = parent(m);
    if !m.saves.card.ready() {
        m.target = p;
        return 0;
    }
    let cur = m.current;
    if inp.pressed & button::CIRCLE != 0 {
        m.saves.confirm_kind = 0;
        m.target = p;
        m.saves.confirmed.insert(cur, true);
    } else if inp.pressed & button::TRIANGLE != 0 {
        m.target = p;
        m.saves.confirmed.insert(cur, false);
    }
    0
}

/// The confirm page's enter (0x295450): +0x84 = 0.
pub fn confirm_enter(m: &mut PageMenu) {
    let cur = if m.current != 0 { m.current } else { m.target };
    m.saves.confirmed.insert(cur, false);
}

/// The window the confirm text and the busy text use (`font_print_window_small`, centred lines, measured, centred
/// vertically, a black copy at +1 under the light blue).
fn small_window(out: &mut Vec<MenuDraw>, a: &MenuAssets, ww: i32, wh: i32, t: &[u8]) {
    let glyphs = &a.hud.glyphs[Font::Small as usize];
    let mut win = wtext::Window::new(1, (wh + 1) as i16, 1, (ww + 1) as i16, (ww >> 1) as i16, 5, 0x10, wtext::CENTRE_LINES | wtext::MEASURE_ONLY);
    wtext::layout(&mut win, t, -1, glyphs, true);
    win.y_start = ((wh - win.height as i32) >> 1) as i16;
    win.flags ^= wtext::MEASURE_ONLY;
    out.push(MenuDraw::Hud(Draw::TextWindow { font: Font::Small, window: win, rgba: 0x8000_0000, text: t.to_vec() }));
    let mut w2 = win;
    w2.x_min -= 1;
    w2.y_min -= 1;
    w2.x_max -= 1;
    w2.y_max -= 1;
    w2.x_anchor -= 1;
    w2.y_start -= 1;
    out.push(MenuDraw::Hud(Draw::TextWindow { font: Font::Small, window: w2, rgba: LIGHT_BLUE, text: t.to_vec() }));
}

/// The confirm page's text (0x295558): return 2.
pub fn confirm_draw(m: &PageMenu, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 2 };
    let k = m.saves.confirm_kind;
    let t: Vec<u8> = match k {
        0..=2 => a.msg(MSG_OVERWRITE).to_vec(),
        3 => a.msg(MSG_SAVING).to_vec(),
        _ => b"(error)".to_vec(),
    };
    small_window(out, a, wd.rect[2], wd.rect[3], &t);
    2
}

/// The busy text of the slot pages' picture widget (0x293d50, flag 0x100, no picture yet): only while the card works,
/// "Reading Memory Card (PS2)" or the running save / load's message. Returns the draw's value.
pub fn busy_draw(m: &PageMenu, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 2 };
    let c = &m.saves.card;
    if c.card.ty != TYPE_PS2 || c.idle() { return 2; }
    let id = if m.saves.busy { m.saves.busy_msg } else { MSG_READING };
    small_window(out, a, wd.rect[2], wd.rect[3], a.msg(id));
    2
}

/// `FontPrintSmall`.
fn small(out: &mut Vec<MenuDraw>, x: i32, y: i32, rgba: u32, t: &[u8]) { text(out, Font::Small, x, y, rgba, t); }

fn rect(out: &mut Vec<MenuDraw>, x0: i32, y0: i32, x1: i32, y1: i32, rgba: u32) { out.push(MenuDraw::Rect { x0, y0, x1, y1, rgba }); }

/// The slot list's draw (0x2973e8): return 2.
pub fn slots_draw(m: &PageMenu, w: u32, a: &MenuAssets, pal: bool, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 2 };
    let ww = wd.rect[2];
    let cursor = wd.raw[4] as i32;
    let c = &m.saves.card;
    let idle = c.idle();
    let navy = m.consts.navy;
    let mut y = 4;
    for i in 0..SLOTS as i32 {
        let x1 = ww - 3;
        if idle && cursor == i {
            rect(out, 0, y - 4, ww, y + 0x34, YELLOW);
            rect(out, 3, y - 1, x1, y + 0x31, navy);
        }
        rect(out, 3, y - 1, x1, y + 0x31, GREY);
        if idle && c.card.ty == TYPE_PS2 {
            let p = c.card.previews[i as usize];
            if p.level == -1 {
                let t = a.msg(MSG_EMPTY);
                small(out, ww / 2 - (a.width(Font::Small, t) >> 1), y + 0x10, LIGHT_BLUE, t);
            } else {
                let hours = p.elapsed / 0x34bc0;
                let col = if cursor == i { YELLOW } else { LIGHT_BLUE };
                let time = format!("{:02}:{:02}", hours.min(99), p.elapsed / 0xe10 - hours * 60);
                out.push(MenuDraw::Hud(Draw::Sprite { frame: a.frame(MENU_ICON, 3), x: 4, y, w: 0x10, h: 0x10, alpha: 0x80, rot: Rot::None }));
                small(out, 0x16, y, col, time.as_bytes());
                if p.completes != 0 {
                    out.push(MenuDraw::Hud(Draw::Sprite { frame: a.frame(MENU_ICON, 4), x: 0x4e, y, w: 0x10, h: 0x10, alpha: 0x80, rot: Rot::None }));
                    small(out, 0x60, y, col, p.completes.min(99).to_string().as_bytes());
                }
                let b = p.bolts.min(9_999_999);
                let bolts = if b < 1000 {
                    format!("{b}")
                } else if b < 1_000_000 {
                    format!("{},{:03}", b / 1000, b % 1000)
                } else {
                    format!("{},{:03},{:03}", b / 1_000_000, (b % 1_000_000) / 1000, b % 1000)
                };
                out.push(MenuDraw::Hud(Draw::Sprite { frame: a.frame(BOLT_ICON, 0xf), x: 4, y: y + 0x10, w: 0x10, h: 0x10, alpha: 0x80, rot: Rot::None }));
                small(out, 0x16, y + 0x10, col, bolts.as_bytes());
                let date = format!("{:02x}/{:02x}/{:02x}", p.clock[6], p.clock[5], p.clock[7]);
                out.push(MenuDraw::Hud(Draw::Sprite { frame: a.frame(MENU_ICON, 2), x: 4, y: y + 0x20, w: 0x10, h: 0x10, alpha: 0x80, rot: Rot::None }));
                small(out, 0x16, y + 0x20, col, date.as_bytes());
            }
        }
        y += if pal { 0x54 } else { 0x4b };
    }
    2
}

/// The focused slot's planet label (0x2967a0): return 2.
pub fn slot_info_draw(m: &PageMenu, w: u32, a: &MenuAssets, out: &mut Vec<MenuDraw>) -> u32 {
    let Some(wd) = m.widgets.get(&w) else { return 2 };
    let (ww, wh) = (wd.rect[2], wd.rect[3]);
    let cursor = m.widgets.get(&focus(m)).map_or(0, |f| f.raw[4] as i32).clamp(0, SLOTS as i32 - 1);
    let c = &m.saves.card;
    if !(c.idle() && c.card.ty == TYPE_PS2) { return 2; }
    let level = c.card.previews[cursor as usize].level;
    let centre = |out: &mut Vec<MenuDraw>, y: i32, t: &[u8]| {
        let tw = a.width(Font::Regular, t);
        text(out, Font::Regular, ww / 2 - (tw >> 1), y, LIGHT_BLUE, t);
    };
    if level == -1 {
        centre(out, wh / 2 - 8, a.msg(MSG_EMPTY));
    } else if let Some(&(loc, planet)) = usize::try_from(level).ok().and_then(|l| m.level_names.get(l)) {
        centre(out, 4, a.msg(loc as i32));
        centre(out, 0x14, a.msg(planet as i32));
    }
    2
}

/// The front end's Options list enter (0x28dbb8): the NTSC or PAL item list.
pub fn front_options_enter(m: &mut PageMenu, w: u32, pal: bool) {
    let items = m.saves.front_options[pal as usize].clone();
    if let Some(Data::List(l)) = m.widgets.get_mut(&w).map(|x| &mut x.data) { l.items = items; }
}

/// The end page's ✕ "time warp" (`process_global_state_flags` 0x295c98, the choice itself is
/// `media::stats_update`): level 18's gold bolts (0x14bf08) and skill point 0x13d425 kept across `memcard_RestoreGame`
/// of the ending buffer 0x1ba250 (the class-1750 save), then game beaten 0x15eea0 = 1. The caller then runs
/// `memcard_Save(0, −1)`, clears the checkpoint records (`FUN_0029abc0`), `FadeToBlack(ticks(16))` and raises the death
/// flag 0x141401 (the reload at the checkpoint).
pub fn time_warp(gs: &mut GameState, ending: Option<&[u8]>) {
    let gold18 = gs.levels.get(18).map(|l| l.gold_bolts);
    let skill = gs.global.skill_points[0x1d];
    // memcard_RestoreGame: nothing when the header sizes differ (or no buffer was made).
    if let Some(b) = ending { let _ = gs.load_card_file(b); }
    gs.global.skill_points[0x1d] = skill;
    if let (Some(g), Some(l)) = (gold18, gs.levels.get_mut(18)) { l.gold_bolts = g; }
    gs.global.game_beaten = 1;
}

/// `memcard_Save` / `fun_002269c0`'s capture from the menu: the clock, the hooks the map page has (0x179638 of the
/// current level), the map state's packed mask.
fn capture(m: &mut PageMenu) -> SaveCapture {
    let mut hooks: Vec<(i32, (f32, f32, f32))> = m.map.hooks.iter().map(|(&k, &v)| (k, v)).collect();
    hooks.sort_by_key(|h| h.0);
    let map_mask = Some(m.map.state.pack());
    SaveCapture { clock: m.saves.clock, hooks, base: m.saves.landmark_base.clone(), map_mask }
}
