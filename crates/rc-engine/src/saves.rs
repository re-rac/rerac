//! The memory card, the saves and the front end in the engine (docs/plan/progression.md `## saves`; the logic is
//! `rc_game::memcard`, `rc_game::menus::freeze`, `rc_game::menus::pause::saves` and `rc_game::frontend`).
//!
//! * **The card** ([`with_card`]): one `MemCard` for the process (a static, as the game's 0x13d290 block), on the native
//!   card folder `rc_game::memcard::save_root`: `RC_SAVE_DIR=<path>` (`0` or empty: no card), else
//!   `<config base>/rerac/memcard/` (macOS `~/Library/Application Support/rerac/memcard`, beside the port settings);
//!   deterministic runs have no card unless `RC_SAVE_DIR` is set. The directory name comes from the disc's SYSTEM.CNF
//!   (`/BASCUS-97199RATCHET`), the icon files and the blank save from the `save_game` lump.
//! * **Every main-loop frame** ([`card_frame`], `FixedUpdate` after the menus' frame): `memcard_Update` then the card
//!   monitor, as `entry` 0x259c40 / the boot loop run them after the mode's update, in every mode. While the page menu
//!   runs, the card is moved into it for its tick (crate::menu_render, as the map state).
//! * **`memcard_Save`** ([`memcard_save`]): the classes' `EngineRequest::Save` (`memcard_Save(0, −1)`; crate::scene_render),
//!   the save notice's ✕ on level 1 and the time warp; the capture: the clock (host UTC as BCD), the landmark hooks
//!   (`Interact::talk_slots` → moby x, y, angle; the level ranges `TalkTables::base`) and the map mask (`MapState::pack`).
//!   `ShipTravelTo`'s `memcard_Save(0, dest)` is the travel lane's call of [`memcard_save`].
//! * **The ending buffer** ([`store_ending`]): class 1750's whole save before the last boss (0x1ba250), read by the time
//!   warp ([`take_ending`]).
//! * **The level exits** (`FUN_002a29a0(level)`): New Game / Load / the dialogs' new game without a card go through the
//!   one level change (`media_render::request_level_exit` → `EngineRequest::LeaveLevel`, the travel lane's), with
//!   0x13e05a (`ShipGlobals::reset_trip`): 1 for a new game's story trip, 0 for a loaded game.
//! * **The front end** (the default start; [`front_end_requested`], [`FrontEndRt`]): the boot flow of `rc_game::frontend` run by crate::menu_render
//!   over the loaded level (the world hidden behind the menu layer cleared to opaque black, the tick suspended): the card
//!   check, the logos movie, the still, the title with its logo, PRESS START, the attract movies and the main menu (the
//!   level's copy of the front-end page tree, kind 0x2d). The title world (the space flight behind the title) is drawn by
//!   crate::title_world in the title phase ([`title_world_shown`]: the menu layer is then clear instead of black). New Game /
//!   Load Game end it with a level exit, which the travel lane's level change loads.

use rc_formats::save_game::SaveGameLump;
use rc_game::game_state::GameState;
use rc_game::memcard::{CardFs, MemCard, SaveCapture};
use std::sync::Mutex;

static CARD: Mutex<Option<MemCard>> = Mutex::new(None);
static ENDING: Mutex<Option<Vec<u8>>> = Mutex::new(None);
static FRONT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Runs `f` on the card (None before setup or while the page menu holds it).
pub fn with_card<R>(f: impl FnOnce(&mut MemCard) -> R) -> Option<R> { CARD.lock().unwrap_or_else(|e| e.into_inner()).as_mut().map(f) }

/// Moves the card out (into the page menu for its tick); [`put_card`] puts it back.
pub fn take_card() -> Option<MemCard> { CARD.lock().unwrap_or_else(|e| e.into_inner()).take() }
pub fn put_card(c: MemCard) { *CARD.lock().unwrap_or_else(|e| e.into_inner()) = Some(c); }

/// Class 1750's `FUN_00281fa8(0x1dfc10)`: the whole save into the ending buffer, once (0x1ba7d0 on level 18 = 0).
pub fn store_ending(gs: &GameState) {
    let mut e = ENDING.lock().unwrap_or_else(|e| e.into_inner());
    if e.is_none() {
        *e = Some(gs.to_save_file().to_bytes());
        println!("saves: the ending buffer (MakeWholeSave before the last boss, class 1750)");
    }
}
/// The ending buffer (the time warp's `memcard_RestoreGame(0x1ba250)`).
pub fn take_ending() -> Option<Vec<u8>> { ENDING.lock().unwrap_or_else(|e| e.into_inner()).clone() }

/// The front end runs (the menu layer is opaque black, crate::menu_render).
pub fn front_end_active() -> bool { FRONT.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_front_end_active(v: bool) { FRONT.store(v, std::sync::atomic::Ordering::Relaxed); }

static TITLE_WORLD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The title world is drawn behind the front end (crate::title_world, G-SAV-012): the menu layer is then clear
/// instead of opaque black, as the boot draws the logo, PRESS START and the main menu over its world.
pub fn title_world_shown() -> bool { TITLE_WORLD.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_title_world_shown(v: bool) { TITLE_WORLD.store(v, std::sync::atomic::Ordering::Relaxed); }

/// `sceCdReadClock` (BCD) now: the host's UTC time.
pub fn clock_now() -> [u8; 8] {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    rc_game::memcard::bcd_clock(t)
}

/// The per-OS config base the port settings use (crate::render_settings).
fn config_base() -> Option<std::path::PathBuf> {
    if crate::determinism::deterministic() { return None; }
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(std::path::PathBuf::from);
    if cfg!(target_os = "macos") {
        Some(var("HOME")?.join("Library/Application Support"))
    } else if cfg!(windows) {
        var("APPDATA")
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config")))
    }
}

/// `memcard_Init` + `memcard_GetName` (`InitOnce`): the card on the save root, its directory from SYSTEM.CNF, the lump.
pub fn setup_card() {
    if CARD.lock().unwrap_or_else(|e| e.into_inner()).is_some() { return; }
    let root = crate::level_load::extracted_root();
    let dir = crate::disc_source::read(&root, "boot/SYSTEM.CNF")
        .ok()
        .and_then(|b| rc_formats::save_game::card_dir_name(&b).ok())
        .unwrap_or_else(|| "/BASCUS-97199RATCHET".into());
    let lump = crate::disc_source::read(&root, "global/save_game.bin").ok().and_then(|b| SaveGameLump::parse(&b).ok());
    let save_root = rc_game::memcard::save_root(config_base().as_deref());
    match &save_root {
        Some(p) => println!("saves: memory card folder {} (directory {dir})", p.display()),
        None => println!("saves: no memory card (RC_SAVE_DIR=0 or a deterministic run)"),
    }
    let mut card = MemCard::new(CardFs::new(save_root), dir, lump);
    card.front_end = front_end_active();
    put_card(card);
}

/// One frame of `memcard_Update` and the card monitor (every mode, after the mode's update).
pub fn card_frame(gs: Option<bevy::prelude::ResMut<crate::gameplay::Persistent>>) {
    let Some(mut gs) = gs else { return };
    let gs = &mut gs.0;
    with_card(|c| {
        let before = (c.state, c.status);
        c.update(gs);
        c.monitor();
        if (c.state, c.status) != before && std::env::var("RC_SAVE_TRACE").is_ok_and(|v| v.trim() == "1") {
            println!("saves: card state {:#x} sub {:#x} status {} flags {:#x} error {:#x} slot {}", c.state, c.sub, c.status, c.flags, c.error, c.card.slot);
        }
    });
}

/// The capture `memcard_Save` / the Save page make from the running level.
pub fn capture(play: &mut crate::gameplay::Play) -> SaveCapture {
    let mut hooks: Vec<(i32, (f32, f32, f32))> = play
        .svc
        .interact
        .talk_slots
        .iter()
        .filter_map(|(&id, &k)| {
            let m = play.game.mobys.mobys.get(id)?;
            Some((k, (m.position[0], m.position[1], m.rotation[2])))
        })
        .collect();
    hooks.sort_by_key(|h| h.0);
    SaveCapture { clock: clock_now(), hooks, base: play.svc.interact.tables.base.clone(), map_mask: Some(play.svc.map.pack()) }
}

/// `memcard_Save(force, pretend)` 0x261448 from the running level (module docs). Returns the game's value.
pub fn memcard_save(play: &mut crate::gameplay::Play, gs: &mut GameState, force: bool, pretend: i32) -> bool {
    let cap = capture(play);
    let r = with_card(|c| c.memcard_save(gs, force, pretend, &cap));
    match r {
        Some(q) => {
            println!("saves: memcard_Save({}, {pretend}): {}", force as i32, if q { "queued" } else { "no card / slot or busy" });
            q
        }
        None => {
            // The page menu holds the card this frame (it never does while a class runs): the capture stays in the state.
            rc_game::memcard::capture(gs, &cap);
            false
        }
    }
}

/// The time warp's engine half (`process_global_state_flags` ✕ after its restore): `memcard_Save(0, −1)`, the checkpoint
/// records cleared (`FUN_0029abc0`), `FadeToBlack(ticks(16))`, the death flag 0x141401 (the reload).
pub fn time_warp(play: &mut crate::gameplay::Play, gs: &mut GameState) {
    rc_game::menus::pause::saves::time_warp(gs, take_ending().as_deref());
    memcard_save(play, gs, false, -1);
    // `FUN_0029abc0`: 0x1baa50 / 0x1bb6b0 (0xc60 each: the visit records, this visit's kills, the checkpoint record and
    // its copies, the never-again bytes) and the visit death bits 0x1ba950 zeroed; the persistent bits 0x14c190 stay.
    let death = std::mem::take(&mut play.svc.save.death);
    play.svc.save = rc_game::moby_update::services::SaveBits { death, ..Default::default() };
    play.svc.cinematic.requests.push(rc_game::cinematic::EngineRequest::FadeToBlack { frames: rc_game::menus::scale_ticks(0x10) });
    play.game.hero.fell_out = 1;
    println!("saves: time warp: the ending buffer restored, game beaten, saved, the reload at the level start");
}

/// The front end's engine state (crate::menu_render runs it).
#[derive(bevy::prelude::Resource, Default)]
pub struct FrontEndRt {
    pub fe: Option<rc_game::frontend::FrontEnd>,
    /// The card dialog (mode 4) of the front end.
    pub freeze: Option<rc_game::menus::freeze::Freeze>,
    /// The loaded level's sounds and music were paused for the front end (its first frame).
    pub paused: bool,
}

/// Start in the front end (the boot flow: the card check, the logos, the title, the main menu) instead of the level. The
/// default; `RC_LEVEL` (a development start straight into a level) skips it unless `RC_FRONTEND=1`, and `RC_FRONTEND=0`
/// always skips it.
pub fn front_end_requested() -> bool {
    match std::env::var("RC_FRONTEND").map(|v| v.trim().to_owned()) {
        Ok(v) if v == "1" => true,
        Ok(v) if v == "0" => false,
        _ => std::env::var("RC_LEVEL").is_err(),
    }
}

/// The front end's pictures and its state machine (the card check, the first attract movie `rand() % 4`).
pub fn front_end(lang: u32, rand: i32) -> rc_game::frontend::FrontEnd {
    use rc_formats::frontend::{BootPicture, BootPictures, TitleWad};
    use std::sync::Arc;
    let root = crate::level_load::extracted_root();
    let check = with_card(|c| {
        let dir = c.dir.clone();
        c.fs.boot_check(&dir)
    })
    .unwrap_or(0);
    let mut fe = rc_game::frontend::FrontEnd::new(rand, check);
    fe.lang = lang;
    match crate::disc_source::read(&root, "global/unknown_14e8.bin").map_err(|e| e.to_string()).and_then(|b| TitleWad::parse(&b).map_err(|e| e.to_string())) {
        Ok(t) => {
            fe.press_tex = t.press_start(lang).cloned().map(Arc::new);
            // Every language's PRESS START (a language change in the Options, rc_game::frontend::FrontEnd::set_language).
            fe.press_texs = (0..rc_game::frontend::LANGUAGES).map(|l| t.press_start(l).cloned().map(Arc::new)).collect();
            fe.logo_tex = Some(Arc::new(t.logo));
        }
        Err(e) => eprintln!("saves: the title world lump not read ({e}): no logo / PRESS START"),
    }
    match crate::disc_source::read(&root, "global/irx.bin").map_err(|e| e.to_string()).and_then(|b| BootPictures::parse(&b).map_err(|e| e.to_string())) {
        Ok(p) => {
            fe.still_tex = p.picture(BootPicture::Still { pal: false }).ok().map(Arc::new);
            let warn = if check == 2 { BootPicture::NoSpace { lang } } else { BootPicture::NoCard { lang } };
            fe.warning_tex = p.picture(warn).ok().map(Arc::new);
        }
        Err(e) => eprintln!("saves: the boot pictures not read ({e})"),
    }
    println!("saves: front end: card check {check}, first attract movie {}", fe.attract_next);
    fe
}
