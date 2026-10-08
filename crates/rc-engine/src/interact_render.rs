//! The "use" system in the engine (`rc_game::moby_update::interact`, docs/plan/interaction.md): the level's talk
//! tables and text installed into the moby services at load, the hand-offs the classes request (the vendor's mode
//! 5, a talker's scene or movie) consumed after each gameplay tick, the context prompt fed to the HUD (slot 12), and
//! game mode 5 itself (`rc_game::menus::vendor`) driven once per main-loop frame from crate::menu_render.
//!
//! * **Load** ([`install`], called by crate::gameplay's setup before the load pass): `TalkTables::load` on the level
//!   overlay (node lists 0x1b1af8, level ranges 0x1c4938, price records 0x1c4530), the level's messages and
//!   language, the talk slots of the created mobys (instance +0x74 → `0x179638`, the loader's
//!   `MobyUnknown74Hook`), the save values the conditions read.
//! * **After a tick** ([`after_tick`], mode 0): `OpenVendor` → mode 5 (the vendor screen, [`VendorRt`]); the
//!   talkers' `Scene` / `Movie` hand-offs stay for crate::scene_render (which plays the scene, or skips the movie,
//!   and at the end places Ratchet and signals `Interact::scene_ended`); the rest are logged. The prompt goes to the
//!   HUD ([`crate::hud_render::HudFeed`]).
//! * **Mode 5** ([`vendor_frame`], docs/plan/interaction.md §9): `UpdatePad` (scripted pad by main-loop frame, as
//!   mode 3), `Vendor::frame`; its animation requests on the vendor moby (seq 2 unfold at half speed, seq 3 open,
//!   seq 4 fold backwards, seq 1) and, in the menu, the vendor's `MobyAnimAdvance` (the moby loop advances it while
//!   the world runs); the world tick of substates 0 / 2 (crate::gameplay runs its tick in the scene form: moby loop,
//!   particles, counter; Ratchet is in state 100) requested through [`crate::menu_render::MenuMode::world_tick`];
//!   the class sounds into the audio system, and `sound_update` itself on the frames without a world tick (the
//!   game's `VendorModeUpdate` calls it every frame: without it no vendor sound would start); the music paused
//!   between the open and the exit; the salesman's voice lines (`vendor_audio`) on the dialogue voice; the camera held
//!   on the vendor's front ([`rc_game::menus::vendor::vendor_camera_at`]); the screens placed on the vendor's monitor
//!   joints ([`VendorRt::draw`], drawn by crate::vendor_render); the HUD's bolt counter pinned and its ammo slot; on
//!   the exit: mode 0, the vendor's state 1, Ratchet teleported 3.5 in front of it (`HeroTeleport`, state 0) and the
//!   follow camera blending back (`CameraScript2(2)`), both through the next tick's class channels.
//!
//! Environment: `RC_INTERACT_TRACE=1` prints the prompt owner changes, every hand-off, the vendor's substates, sounds
//! and voice lines.

use crate::gameplay::Play;
use crate::play_camera::PlayView;
use bevy::prelude::*;
use rc_formats::moby_anim::{self, MobyAnimClass};
use rc_formats::moby_light::V4;
use rc_formats::save_game::ItemTables;
use rc_game::follow_camera::CameraView;
use rc_game::game_state::GameState;
use rc_game::menus::mode::{Mode, ModeState};
use rc_game::menus::vendor::screens::{self, FxDraw, Placed, Quad, View};
use rc_game::menus::vendor::{self as vendor, vendor_camera_at, Vendor, VendorAnim, VendorClasses, VendorOut, VendorScene, VendorTables};
use rc_game::menus::{MenuAssets, MenuDraw, MenuInput, Overlay};
use rc_game::moby_runtime::MobyId;
use rc_game::moby_update::interact::{Handoff, TalkTables};
use rc_game::moby_update::services::{ClassData, SoundEvent};
use rc_game::moby_update::Services;
use rc_game::ps2v::Pf;
use std::sync::Arc;

/// The level overlay (`levels/NN/overlay.bin`).
fn overlay() -> anyhow::Result<(Vec<u8>, Overlay)> {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let bytes = crate::disc_source::level_file(&root, index, "overlay.bin")?;
    let ov = Overlay::parse(&bytes)?;
    Ok((bytes, ov))
}

/// The load step (module docs): tables, text, talk slots, save values.
pub fn install(svc: &mut Services, lv: &crate::level_load::LoadedLevel, instance_to_moby: &[Option<MobyId>], state: Option<&GameState>) {
    let it = &mut svc.interact;
    match overlay().map(|(_, ov)| TalkTables::load(&ov)) {
        Ok(Some(t)) => {
            let n = t.tables.iter().filter(|x| !x.is_empty()).count();
            it.tables = Arc::new(t);
            println!("interact: talk tables: {n} node lists, level ranges {:?}", it.tables.base);
        }
        Ok(None) => eprintln!("interact: talk tables not found in the overlay: NPC talk disabled"),
        Err(e) => eprintln!("interact: overlay not read ({e:#}): NPC talk disabled"),
    }
    if let Some(h) = lv.hud.as_ref() {
        it.messages = Arc::new(h.messages.clone());
        it.lang = h.lang;
    }
    let slots: Vec<(MobyId, i32)> = lv
        .mobys
        .instances
        .iter()
        .enumerate()
        .filter(|(_, i)| i.unknown_74 != -1)
        .filter_map(|(k, i)| Some((instance_to_moby.get(k).copied().flatten()?, i.unknown_74)))
        .collect();
    println!("interact: {} talk slots (instance +0x74): {:?}", slots.len(), slots);
    it.register_talk_mobys(slots);
    if let Some(gs) = state { it.sync_game(gs); }
}

/// A screen as crate::vendor_render draws it this frame: which, where, its content (target pixels) and static.
#[derive(Clone, Debug)]
pub struct ScreenDraw {
    pub s: usize,
    pub placed: Placed,
    pub content: Vec<MenuDraw>,
    pub fx: Vec<FxDraw>,
    /// Drawn with TEX1_1 = 1 (point sampling): the ticker, the first screen after the hologram cone
    /// (`DrawWorld_Mode5` 0x2b4020: cone 0x2b3cc8 → `FastDrawQuadReal` TEX1_1 = its quad's +0x80 = 1; screens 1..5
    /// follow the item's and the salesman's moby draws, whose packets set bilinear TEX1). No cone on the PDA's remote
    /// vendor (0x1ca980): then the vendor moby's TEX1 holds.
    pub nearest: bool,
}

/// What mode 5 draws this frame (written by [`vendor_frame`], drawn by crate::vendor_render).
#[derive(Clone, Debug, Default)]
pub struct VendorDraw {
    /// The vendor moby (the cone and the screens hang on it), its position and rows.
    pub vendor: Option<(MobyId, [f32; 3], [[f32; 4]; 4])>,
    /// The camera the screens were projected with.
    pub view: Option<View>,
    pub screens: Vec<ScreenDraw>,
    /// The glass quads over screens 1..5 (world corners).
    pub glass: Vec<Quad>,
    /// The item hologram, the item panel's and the salesman's mobys, the popup.
    pub scene: VendorScene,
    /// The cone's V scroll 0x161184 (menu only).
    pub cone: Option<f32>,
}

/// The vendor mode's engine state.
#[derive(Resource, Default)]
pub struct VendorRt {
    tables: Option<VendorTables>,
    items: Option<ItemTables>,
    loaded: bool,
    pub vendor: Option<Vendor>,
    moby: Option<MobyId>,
    /// The vendor's joint lists 0..23 (the screens' corners) and the popup's 0..2.
    vendor_lists: Vec<Vec<u8>>,
    popup_lists: Vec<Vec<u8>>,
    trace: bool,
    last_owner: i32,
    /// The cone's scroll 0x161184 (`VendorDrawHologramCone`: +0.01 per draw, wrapped at 1).
    cone_scroll: f32,
    pub draw: VendorDraw,
    /// The HUD language (the salesman's lines).
    lang: u32,
    /// Wall-clock time of the open (`RC_INTERACT_TRACE` timing).
    opened_at: Option<std::time::Instant>,
    /// The PDA's remote vendor (`OpenVendorMenu(0)`): its own class-11 moby (deleted by `VendorExit`) and the view the
    /// first render sets (the camera's position, rows identity: looking along +x; the Euler restored at the leave).
    remote_moby: bool,
    remote_cam: Option<CameraView>,
}

/// The class blob of `o_class` in the level core (joint lists).
pub(crate) fn class_blob(o_class: i32) -> anyhow::Result<Vec<u8>> {
    // The level core, decompressed once per level for every blob asked for (the store's copy; a runtime level change,
    // crate::level_switch, makes it the next level's).
    type Core = (u32, Option<(std::sync::Arc<[u8]>, rc_formats::level::LevelCore)>);
    static CORE: std::sync::Mutex<Option<Core>> = std::sync::Mutex::new(None);
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let mut g = CORE.lock().unwrap_or_else(|e| e.into_inner());
    if g.as_ref().map(|c| c.0) != Some(index) {
        let load = || -> Option<(std::sync::Arc<[u8]>, rc_formats::level::LevelCore)> {
            let data = rc_data::level_core_data(&root, index).ok()?;
            let core = rc_formats::level::parse_level_core(&crate::disc_source::level_file(&root, index, "core_index.bin").ok()?, data.len()).ok()?;
            Some((data, core))
        };
        *g = Some((index, load()));
    }
    let (data, core) = g.as_ref().and_then(|c| c.1.as_ref()).ok_or_else(|| anyhow::anyhow!("level core not read"))?;
    let name = format!("moby_class/{o_class:04}");
    let blk = core.blocks.iter().find(|b| b.name == name).ok_or_else(|| anyhow::anyhow!("no {name}"))?;
    Ok(data.get(blk.offset..blk.offset + blk.size).ok_or_else(|| anyhow::anyhow!("{name} out of range"))?.to_vec())
}

/// The first byte list of joint lists `0..n` of a class blob (the lists `MobyGetBoneMatrix` evaluates).
fn joint_lists(blob: &[u8], n: usize) -> Vec<Vec<u8>> {
    let Ok(c) = rc_formats::moby::parse_moby_class(blob) else { return Vec::new() };
    (0..n).map_while(|l| rc_formats::gadget::joint_list(blob, &c.header, l).ok().map(|(a, _)| a)).collect()
}

/// The salesman's class (level class 12) with the ten sequences of `vendor.bin` (`FUN_002ade20`: WAD-compressed, 16
/// offsets, each a sequence whose pointers are relative to its start).
fn salesman_class(play: &Play) -> anyhow::Result<MobyAnimClass> {
    let root = crate::level_load::extracted_root();
    let raw = crate::disc_source::read(&root, "global/vendor.bin")?;
    let d = rc_formats::wad::decompress(&raw)?;
    let blob = class_blob(vendor::salesman::CLASS as i32)?;
    let c = rc_formats::moby::parse_moby_class(&blob)?;
    let mut seqs = Vec::new();
    for i in 0..16usize {
        let o = u32::from_le_bytes(d.get(4 * i..4 * i + 4).ok_or_else(|| anyhow::anyhow!("vendor.bin header"))?.try_into()?) as usize;
        if o == 0 {
            if i < c.header.sequence_count as usize { seqs.push(None); }
            continue;
        }
        seqs.push(Some(moby_anim::parse_sequence(d.get(o..).ok_or_else(|| anyhow::anyhow!("vendor.bin offset {o:#x}"))?, 0)?));
    }
    let _ = play;
    Ok(MobyAnimClass::new(&c, seqs))
}

impl VendorRt {
    /// The placement tables (the approach beam uses them while no vendor is open).
    pub fn layout(&self) -> Option<rc_game::menus::vendor::layout::VendorLayout> { self.tables.as_ref().and_then(|t| t.layout.clone()) }

    fn load(&mut self, play: &Play, assets: Option<&MenuAssets>) {
        if self.loaded { return; }
        self.loaded = true;
        self.trace = std::env::var("RC_INTERACT_TRACE").is_ok_and(|v| v.trim() == "1");
        let root = crate::level_load::extracted_root();
        let r = (|| -> anyhow::Result<(ItemTables, VendorTables)> {
            let (bytes, ov) = overlay()?;
            let elf = crate::disc_source::read_path(&root, &root.join("boot/SCUS_971.99"))?;
            let items = ItemTables::load(&elf, &bytes)?;
            // The placement tables by their level-01 labels (the menus' relocated overlay maps them on other levels).
            let ov = assets.map(|a| a.overlay.clone()).unwrap_or(ov);
            let mut vt = VendorTables::load(&ov, items.item_defs_addr, play.svc.interact.tables.shop.clone())
                .ok_or_else(|| anyhow::anyhow!("vendor tables not found"))?;
            let salesman = salesman_class(play).map_err(|e| eprintln!("interact: no salesman ({e:#})")).ok();
            let popup = play.classes.anim(vendor::POPUP_CLASS).cloned();
            vt.classes = Arc::new(VendorClasses { popup, salesman });
            Ok((items, vt))
        })();
        match r {
            Ok((i, t)) => {
                println!(
                    "interact: vendor tables: {} ticker lines, item names {:?}, layout {}, salesman {}, popup {}",
                    t.ticker.len(), &t.name[10..20], t.layout.is_some(), t.classes.salesman.is_some(), t.classes.popup.is_some()
                );
                self.items = Some(i);
                self.tables = Some(t);
            }
            Err(e) => eprintln!("interact: no vendor ({e:#})"),
        }
        self.vendor_lists = class_blob(vendor::VENDOR_CLASS as i32).map(|b| joint_lists(&b, 24)).unwrap_or_default();
        self.popup_lists = class_blob(vendor::POPUP_CLASS as i32).map(|b| joint_lists(&b, 3)).unwrap_or_default();
        println!("interact: vendor joint lists {}, popup joint lists {}", self.vendor_lists.len(), self.popup_lists.len());
    }
}

/// The class sounds of the vendor (`PlayClassSound(n, 0, vendor)`) into the audio system.
fn play_sounds(play: &mut Play, audio: Option<&mut crate::audio_out::AudioOut>, moby: Option<MobyId>, sounds: &[u8], listener: &rc_game::audio::voices::Listener) {
    let Some(id) = moby else { return };
    let Some(m) = play.game.mobys.mobys.get(id) else { return };
    let ev = |i: u8| SoundEvent { index: i as i32, flags: 0, moby: id, o_class: m.o_class, sound_class: m.o_class, pos: [m.position[0], m.position[1], m.position[2]], tick: play.game.counter };
    let evs: Vec<SoundEvent> = sounds.iter().map(|&i| ev(i)).collect();
    if let Some(a) = audio {
        for e in &evs { a.system().play_class_sound(e, None, listener, &mut play.game.rng); }
    }
}

/// Every frame, whatever the mode: a `PromptRelease` of the last tick empties the prompt's slot at once. The ship's △
/// releases it in the tick whose take-off leaves mode 0 before [`after_tick`] would run.
pub fn feed_release(play: &Play, feed: &mut crate::hud_render::HudFeed) {
    let released = play.svc.interact.prompt.released;
    if feed.prompt_released != released {
        feed.prompt_released = released;
        feed.prompt = false;
    }
}

/// After a mode-0 tick: the hand-offs and the prompt (module docs).
#[allow(clippy::too_many_arguments)]
pub fn after_tick(
    vr: &mut VendorRt,
    play: &mut Play,
    gs: &mut GameState,
    mode: &mut ModeState,
    frame: u64,
    feed: Option<&mut crate::hud_render::HudFeed>,
    audio: Option<&mut crate::audio_out::AudioOut>,
    assets: Option<&MenuAssets>,
) {
    vr.load(play, assets);
    let it = &mut play.svc.interact;
    if vr.trace && it.prompt.owner != vr.last_owner {
        println!("interact: frame {frame}: prompt owner {} → {} (msg {})", vr.last_owner, it.prompt.owner, it.prompt.msg);
    }
    vr.last_owner = it.prompt.owner;
    vr.lang = it.lang;
    let mut feed = feed;
    if let Some(f) = feed.as_deref_mut() {
        f.prompt = it.prompt_hud || it.talk_shown || it.class_shown;
        if f.prompt_text != it.prompt.text { f.prompt_text = it.prompt.text.clone(); }
    }
    let mut audio = audio;
    let handoffs = std::mem::take(&mut play.svc.interact.handoffs);
    let mut keep = Vec::new();
    for h in handoffs {
        println!("interact: frame {frame}: hand-off {h:?}");
        match h {
            Handoff::OpenVendor { vendor } => {
                let Some(t) = vr.tables.clone() else { continue };
                let mut out = VendorOut::default();
                let mut v = Vendor::open(t, gs, vendor.is_none(), &mut out);
                // OpenVendorMenu(0) (the PDA): `CreateMoby(0xb)` at the camera + (3.8, 0, −1.5) (gp−0x5ba0, 0x161068), +0x32
                // = 0x40, rotation (0, 0, π), `MobyBuildMatrix`, state 3; the view: rows identity at the camera.
                let vendor = match vendor {
                    Some(id) => Some(id),
                    None => {
                        let id = remote_vendor(play);
                        vr.remote_moby = id.is_some();
                        let pos = play.game.camera.out.pos;
                        let one = |k: usize| std::array::from_fn::<Pf, 4, _>(|i| if i == k { Pf::f(1.0) } else { Pf::ZERO });
                        vr.remote_cam = Some(CameraView { pos, euler: [Pf::ZERO; 4], rows: [one(0), one(1), one(2)], class: 0 });
                        id
                    }
                };
                v.lang = vr.lang as i32;
                println!(
                    "interact: frame {frame}: OpenVendorMenu: mode 5, {} entries {:?}, selection {}",
                    v.items.len(), v.items.iter().map(|e| (e.item, e.ammo)).collect::<Vec<_>>(), v.sel
                );
                // The four arm manipulators 0x166300 on the vendor's joint lists 0x14..0x17 (the monitors laid out for
                // fewer than 8 entries).
                if let Some(id) = vendor { arms(play, id, Some(v.items.len())); }
                vr.vendor = Some(v);
                vr.moby = vendor;
                // FUN_0024fb00: the HUD's slots emptied (the prompt goes at once); the bolt counter is pinned again.
                if let Some(f) = feed.as_deref_mut() {
                    f.reset = f.reset.wrapping_add(1);
                    f.prompt = false;
                }
                vr.cone_scroll = 0.0;
                vr.opened_at = Some(std::time::Instant::now());
                apply_anim(play, vendor, &out.anim);
                // `snd_PauseAllSoundsInGroup(0x1d)`, `music_Pause(0)` (before the open sound, as `OpenVendorMenu`); the
                // world ticks of mode 5 are the scene form (no camera update).
                if let Some(a) = audio.as_deref_mut() { a.system().menu_open(); }
                let listener = rc_game::audio::class_sounds::listener_of(&play.game.camera.out);
                play_sounds(play, audio.as_deref_mut(), vendor, &out.sounds, &listener);
                play.game.camera_paused = true;
                mode.set(Mode::Vendor);
            }
            // crate::scene_render's.
            h @ (Handoff::Scene { .. } | Handoff::Movie { .. }) => keep.push(h),
            Handoff::ShipMenu | Handoff::GoldUpgrade { .. } => {}
        }
    }
    play.svc.interact.handoffs = keep;
}

/// The vendor's animation requests on its moby (`hard_cut`, `MobyAnimBlend`, +0x58).
fn apply_anim(play: &mut Play, moby: Option<MobyId>, reqs: &[VendorAnim]) {
    let Some(id) = moby else { return };
    let classes = play.classes.clone();
    let Some(class) = classes.anim(vendor::VENDOR_CLASS) else { return };
    if play.svc.snapshots.len() <= id { play.svc.snapshots.resize(id + 1, None); }
    let m = &mut play.game.mobys.mobys[id];
    for r in reqs {
        match *r {
            VendorAnim::HardCut { seq, frame } => {
                moby_anim::hard_cut(&mut m.anim, class, seq, frame);
            }
            VendorAnim::Blend { seq, frame, ticks } => {
                if moby_anim::set_sequence(&mut m.anim, class, seq, frame, ticks, &mut play.svc.snapshots[id]) {
                    rc_game::moby_update::anim_sound::after_sequence_change(m, class);
                }
            }
            VendorAnim::Speed(s) => m.anim.speed = s,
        }
    }
}

/// The vendor camera as a game view (eye = rows · (3.8, 0, 1.5) + position, yaw + π).
fn camera_view(m: &rc_game::moby_runtime::Moby, offset: [f32; 3]) -> CameraView {
    let (eye, rows) = vendor_camera_at(offset, [m.position[0], m.position[1], m.position[2]], m.rows, m.rotation[2]);
    let v = |a: [f32; 3]| [Pf::f(a[0]), Pf::f(a[1]), Pf::f(a[2]), Pf::ZERO];
    let yaw = rows[0][1].atan2(rows[0][0]);
    CameraView { pos: v(eye), euler: [Pf::ZERO, Pf::ZERO, Pf::f(yaw), Pf::ZERO], rows: [v(rows[0]), v(rows[1]), v(rows[2])], class: 0 }
}

fn rows_bits(r: &[[f32; 4]; 4]) -> [V4; 3] { [0, 1, 2].map(|i| r[i].map(f32::to_bits)) }

/// World points of a moby's joint lists (`MobyGetBoneMatrix`: the lists' last joints' pose translations × scale /
/// 1024, turned by its rows, plus its position).
fn joint_points(class: &MobyAnimClass, state: &moby_anim::AnimState, snap: Option<&moby_anim::MobyFrame>, lists: &[Vec<u8>], rows: &[[f32; 4]; 4], pos: [f32; 3], scale: f32) -> Vec<[f32; 3]> {
    joint_points_posed(class, state, snap, lists, rows, pos, scale, &[])
}

/// [`joint_points`] with the moby's joint-modifier list (+0x64): the vendor's arm manipulators move its monitors.
#[allow(clippy::too_many_arguments)]
fn joint_points_posed(class: &MobyAnimClass, state: &moby_anim::AnimState, snap: Option<&moby_anim::MobyFrame>, lists: &[Vec<u8>], rows: &[[f32; 4]; 4], pos: [f32; 3], scale: f32, mods: &[moby_anim::JointModifier]) -> Vec<[f32; 3]> {
    let chains: Vec<&[u8]> = lists.iter().map(|l| l.as_slice()).collect();
    if chains.is_empty() || chains.iter().any(|c| c.is_empty()) { return Vec::new(); }
    let p: Vec<[u32; 4]> = moby_anim::evaluate_chains_posed(class, state, snap, &chains, &[], mods).iter().map(|r| r[3].map(f32::to_bits)).collect();
    moby_anim::bone_points(&p, &rows_bits(rows), pos, scale).iter().map(|v| [f32::from_bits(v[0]), f32::from_bits(v[1]), f32::from_bits(v[2])]).collect()
}

/// `OpenVendorMenu(0)`'s remote vendor moby (module docs of `rc_game::hero::pda`): class 11 at the camera + (3.8, 0, −1.5),
/// +0x32 = 0x40, rotation (0, 0, π), its matrix built, state 3. None when the class is not on the level or the table is
/// full (the game would crash on a null moby).
fn remote_vendor(play: &mut Play) -> Option<MobyId> {
    let classes = play.classes.clone();
    let cam = play.game.camera.out.pos;
    let mut w = rc_game::moby_update::services::World::new(&mut play.game.mobys, &play.game.hero, &mut play.game.rng, &*classes, &mut play.svc, play.game.counter);
    classes.info(vendor::VENDOR_CLASS)?;
    let id = w.create_moby(vendor::VENDOR_CLASS)?;
    {
        let m = w.mm(id);
        m.draw_dist = 0x40;
        let pw = m.position[3];
        m.position = [cam[0].to_f32() + 3.8, cam[1].to_f32(), cam[2].to_f32() - 1.5, pw];
        m.rotation = [0.0, 0.0, std::f32::consts::PI, m.rotation[3]];
        m.state = 3;
    }
    w.build_matrix(id);
    Some(id)
}

/// The owner key of the vendor's arm manipulators 0x166300 (`rc_game::moby_update::manip::key` with an owner no table
/// moby has).
const ARMS_OWNER: usize = 0xfffe;

/// `OpenVendorMenu`'s `AttachManipulator(vendor, 0x14 + k, 0x166300 + 0x40·k)` (with `entries`:
/// `rc_game::menus::vendor::arm_manipulators`), or `DetachManipulator` of the four (None).
fn arms(play: &mut Play, vendor_id: rc_game::moby_runtime::MobyId, entries: Option<usize>) {
    use rc_game::moby_update::manip;
    let targets = play.svc.joint_targets.get(&vendor::VENDOR_CLASS).cloned().unwrap_or_default();
    let Some(m) = play.game.mobys.mobys.get_mut(vendor_id) else { return };
    if m.joint_mod_keys.len() != m.joint_mods.len() { m.joint_mod_keys.resize(m.joint_mods.len(), u32::MAX); }
    for k in 0..4usize {
        let key = manip::key(ARMS_OWNER, 0x40 * k);
        if let Some(i) = m.joint_mod_keys.iter().position(|&x| x == key) {
            m.joint_mod_keys.remove(i);
            m.joint_mods.remove(i);
        }
    }
    let Some(n) = entries else { return };
    for (k, (list, trans)) in vendor::arm_manipulators(n).into_iter().enumerate() {
        let Some(&joint) = targets.get(list as usize).filter(|&&j| j != 0xff) else { continue };
        let node = moby_anim::JointModifier { joint, mode: 0, weight: 0.0, quat: [0.0, 0.0, 0.0, 1.0], scale: [1.0; 3], trans };
        m.joint_mods.insert(0, node);
        m.joint_mod_keys.insert(0, manip::key(ARMS_OWNER, 0x40 * k));
    }
}

/// One EE audio frame of mode 5 (`sound_update` at the end of `VendorModeUpdate`) when no world tick ran it.
fn sound_frame(play: &mut Play, audio: &mut crate::audio_out::AudioOut, listener: rc_game::audio::voices::Listener) {
    let game = &mut play.game;
    let h = game.hero.pos;
    let input = rc_game::audio::FrameInput { listener, hero_pos: [h[0].to_f32(), h[1].to_f32(), h[2].to_f32()] };
    let counter = game.counter as u32;
    let table = &game.mobys;
    let (sys, buf) = audio.parts();
    buf.clear();
    // sound_update only: the sound instances (0x2a19a8) run from the level update, which mode 5 does not run.
    sys.sound_update_with(&input, counter, &mut game.rng, &|id| rc_game::audio::class_sounds::owner_position(table, id));
    sys.render(rc_game::audio::SAMPLES_PER_FRAME, buf);
    audio.push_frame();
}

/// One mode-5 frame (module docs). Returns the frame's draws (the fades) in `draws`.
#[allow(clippy::too_many_arguments)]
pub fn vendor_frame(
    vr: &mut VendorRt,
    play: &mut Play,
    gs: &mut GameState,
    sess: &mut rc_game::game_state::SessionState,
    mm: &mut crate::menu_render::MenuMode,
    input: MenuInput,
    assets: &MenuAssets,
    frame: u64,
    view: Option<&mut PlayView>,
    feed: Option<&mut crate::hud_render::HudFeed>,
    audio: Option<&mut crate::audio_out::AudioOut>,
    draws: &mut Vec<MenuDraw>,
) {
    let (Some(v), Some(items)) = (vr.vendor.as_mut(), vr.items.as_ref()) else {
        mm.state.set(Mode::Gameplay);
        mm.world_tick = false;
        return;
    };
    let mut audio = audio;
    let (sub0, pre0) = (v.sub, v.pre_fade);
    let ticked = std::mem::take(&mut mm.world_ticked);
    let bolts0 = gs.global.bolts;
    let mut out = v.frame(&input, gs, items, sess, assets, &mut play.game.rng);
    // VendorStartWeaponDemo 0x2ae7f8: the demo scene plays in the vendor's frame (the space-scene player,
    // crate::travel_render, `rc_game::travel::space::SUB_DEMO`), the hand request 0x141408 = the bought weapon (0 for the
    // Drone Device 0x18); substate 3 ends with `VendorExit(1)` when the demo is over.
    let mut demo_exit = false;
    if let Some(d) = out.weapon_demo.take() {
        println!("interact: frame {frame}: VendorStartWeaponDemo: item {} scene unknown_1530[{}], stream {}", d.item, d.scene, d.stream);
        sess.temp_hand = if d.item == 0x18 { 0 } else { d.item as i32 };
        match vr.moby {
            Some(id) => {
                // `CameraScript2(2)` first (the scene's camera then drives the view).
                let level = play.svc.level;
                play.svc.cinematic.calls.push(rc_game::cinematic::CinematicCall::CameraRelease { kind: 2, level });
                crate::travel_render::request_weapon_demo(d.scene, id);
            }
            None => {
                out.exit = true;
                out.sounds.push(vendor::sound::CLOSED);
            }
        }
    }
    if v.sub == 3 && crate::travel_render::take_weapon_demo_done() {
        out.exit = true;
        out.stop_voice = true;
        demo_exit = true;
    }
    // DetachManipulator(vendor, 0x166300 + 0x40·k): the arms back.
    if out.detach_arms {
        if let Some(id) = vr.moby { arms(play, id, None); }
    }
    // VendorExit: 0x15172a ∉ {6, 7} → 5: the salesman's line stops.
    if out.stop_voice && out.exit {
        if let Some(a) = audio.as_deref_mut() { a.system().scene_command(rc_game::audio::scene::SceneAudioCmd::StopSpeech); }
    }
    apply_anim(play, vr.moby, &out.anim);
    let classes = play.classes.clone();
    // The menu's MobyAnimAdvance of the vendor (the moby loop does it while the world runs).
    if out.advance_vendor {
        if let (Some(id), Some(c)) = (vr.moby, classes.anim(vendor::VENDOR_CLASS)) { moby_anim::advance(&mut play.game.mobys.mobys[id].anim, c); }
    }
    if vr.trace && (v.sub != sub0 || (pre0 > 0 && v.pre_fade == 0) || out.exit) {
        let ms = vr.opened_at.map_or(0.0, |t| t.elapsed().as_secs_f64() * 1e3);
        println!("interact: frame {frame}: vendor substate {} (t {}, fade {:.2}, pre-fade {}, world {}, exit {}) {ms:.0} ms after △", v.sub, v.t, v.fade, v.pre_fade, v.world_runs(), out.exit);
    }
    let layout_cam = v.tables.layout.as_ref().map_or([3.8, 0.0, 1.5], |l| l.camera);
    let vm = vr.moby.and_then(|id| play.game.mobys.mobys.get(id).cloned());
    // The camera: cut to the vendor's front once the open's fade is black, held there (the script camera's targets).
    let cam = if vr.remote_moby { vr.remote_cam } else { vm.as_ref().map(|m| camera_view(m, layout_cam)) };
    if v.pre_fade == 0 {
        if let (Some(view), Some(c)) = (view, cam) { view.view = c; }
    }
    let listener = cam.map(|c| rc_game::audio::class_sounds::listener_of(&c)).unwrap_or_else(|| rc_game::audio::class_sounds::listener_of(&play.game.camera.out));
    v.fades(draws);
    // The screens (menu render), the glass quads, the scene mobys.
    let mut d = VendorDraw::default();
    if let (Some(m), Some(id), Some(c)) = (vm.as_ref(), vr.moby, cam) {
        let pos = [m.position[0], m.position[1], m.position[2]];
        d.vendor = Some((id, pos, m.rows));
        let rows_f = c.rows_f32();
        let view_g = View::game(c.pos_f32(), rows_f);
        d.view = Some(view_g);
        let snap = play.svc.snapshots.get(id).and_then(|s| s.as_ref());
        let pts = classes.anim(vendor::VENDOR_CLASS).map(|cl| joint_points_posed(cl, &m.anim, snap, &vr.vendor_lists, &m.rows, pos, m.scale, &m.joint_mods)).unwrap_or_default();
        let joint = |j: usize| pts.get(j).copied().unwrap_or(pos);
        let layout = v.tables.layout.clone();
        if let Some(l) = layout.as_ref().filter(|_| pts.len() >= 23) {
            if v.screens_shown() {
                let f = v.power();
                let placed: Vec<Option<Placed>> = (0..6).map(|s| screens::place(l, s, &joint, &view_g, f)).collect();
                // The popup (screen 6): its joints 0..2, margins 0.03 / 0.04, shrink 0.06 / 0.04 (FUN_002b3a40) [M: the list
                // ids are lost in the decompile].
                let popup_place = match (v.popup.as_ref(), v.tables.classes.popup.as_ref()) {
                    (Some(p), Some(pc)) => {
                        let rows = vendor::euler_rows([m.rotation[0], m.rotation[1], m.rotation[2]]);
                        let r4 = [[rows[0][0], rows[0][1], rows[0][2], 0.0], [rows[1][0], rows[1][1], rows[1][2], 0.0], [rows[2][0], rows[2][1], rows[2][2], 0.0], [0.0; 4]];
                        let scale = classes.info(vendor::POPUP_CLASS).map_or(1.0, |i| i.scale);
                        let pp = joint_points(pc, &p.anim, p.snapshot.as_ref(), &vr.popup_lists, &r4, pos, scale);
                        (pp.len() >= 3).then(|| {
                            let q = Quad::new([pp[0], pp[1], pp[2]], [0.03, 0.04, 0.03, 0.02]);
                            view_g.rect(q.c, q.far()).map(|r| Placed { full: r, drawn: r, quad: q })
                        }).flatten()
                    }
                    _ => None,
                };
                let size = |p: &Option<Placed>| p.map_or((0.0, 0.0), |p| (p.drawn[2], p.drawn[3]));
                let mut sizes = [(0.0, 0.0); 7];
                for s in 0..6 { sizes[s] = size(&placed[s]); }
                sizes[6] = size(&popup_place);
                let fx = v.render_screens(&sizes, &mut play.game.rng, assets);
                for (s, p) in placed.iter().enumerate() {
                    let Some(p) = p else { continue };
                    let (w, h) = p.texel_size();
                    let nearest = s == screens::TICKER && !v.remote;
                    d.screens.push(ScreenDraw { s, placed: *p, content: v.screen_content(s, (w, h), assets, gs, frame as u32), fx: fx[s].clone(), nearest });
                }
                if let Some(p) = popup_place {
                    let (w, h) = p.texel_size();
                    d.screens.push(ScreenDraw { s: screens::POPUP, placed: p, content: v.screen_content(screens::POPUP, (w, h), assets, gs, frame as u32), fx: fx[6].clone(), nearest: false });
                }
                // The cone's V scroll (+0.01 per draw); the remote vendor has no cone (`DrawWorld_Mode5`: 0x1ca980 = 0 only).
                if !v.remote {
                    vr.cone_scroll += 0.01;
                    if vr.cone_scroll > 1.0 { vr.cone_scroll -= 1.0; }
                    d.cone = Some(vr.cone_scroll);
                }
            }
            if v.screens_shown() || v.glass_only() {
                d.glass = (1..6).map(|s| screens::glass(l, s, &joint)).collect();
            }
        }
        if v.screens_shown() { d.scene = v.scene(pos, &m.rows, [m.rotation[0], m.rotation[1], m.rotation[2]], gs); }
    }
    vr.draw = d;
    if let Some(f) = feed {
        f.prompt = false;
        f.bolts_pinned = !out.exit;
        f.weapon = Some(v.hud_ammo(gs));
    }
    if let Some(p) = out.purchase {
        println!("interact: frame {frame}: purchase item {} ({}) for {} bolts, {} unit(s): bolts {} → {}, owned {}, quick select {:?}",
            p.0, if p.1 { "ammo" } else { "weapon" }, p.2, p.3, bolts0, gs.global.bolts, gs.global.owned[p.0], gs.global.quick_select);
    }
    if !out.sounds.is_empty() && vr.trace { println!("interact: frame {frame}: vendor sounds {:?}", out.sounds); }
    // The moby services' bolt copy is what the next tick writes back (gameplay::tick).
    play.svc.counters.bolts = gs.global.bolts;
    let moby = vr.moby;
    play_sounds(play, audio.as_deref_mut(), moby, &out.sounds, &listener);
    if let Some(id) = out.voice {
        let n = id - vendor::salesman::VOICE_BASE;
        let root = crate::level_load::extracted_root();
        match crate::disc_source::read(&root, &format!("global/vendor_audio/{n:03}.bin")) {
            Ok(b) => {
                if vr.trace { println!("interact: frame {frame}: salesman line {id} (vendor_audio {n:03})"); }
                if let Some(a) = audio.as_deref_mut() { a.system().scene_command(rc_game::audio::scene::SceneAudioCmd::Speech { vag: Arc::from(b) }); }
            }
            Err(e) => eprintln!("interact: salesman line {id}: {e:#}"),
        }
    }
    // sound_update of this frame (the world tick's sound step made it when the world ran; FadeToBlack's frames have none).
    if !ticked && pre0 == 0 {
        if let Some(a) = audio.as_deref_mut() { sound_frame(play, a, listener); }
    }
    let world_next = !out.exit && v.world_runs();
    if out.exit {
        // VendorExit 0x2ae660: mode 0, the HUD slots released, the vendor's state 1, Ratchet teleported in front of the
        // vendor facing it (state 0), CameraScript2(2) from the vendor view, music_Unpause.
        if let Some(id) = moby { rc_game::moby_update::classes::vendor::on_exit(&mut play.game.mobys, id); }
        if vr.remote_moby {
            // The remote vendor: `SetState(0, 1)` where he stands, no camera script, the moby deleted (0x1ca980 ≠ 0).
            let mut f = rc_game::moby_update::services::HeroFields::of(&play.game.hero);
            f.call(rc_game::moby_update::services::HeroCall::SetState { id: 0, play: true });
            play.svc.hero_writes = Some((play.game.counter, f));
            if let Some(id) = moby {
                let c = play.game.counter;
                play.game.mobys.delete(id, c);
            }
            vr.remote_moby = false;
            vr.remote_cam = None;
            vr.moby = None;
        } else if let (Some(m), Some(c)) = (vm.as_ref(), cam) {
            let exit = v.tables.layout.as_ref().map_or([3.5, 0.0, 0.0], |l| l.exit);
            let pos = vendor::to_world(&m.rows, [m.position[0], m.position[1], m.position[2]], exit);
            let yaw = rc_game::moby_update::interact::add_rot(m.rotation[2], std::f32::consts::PI);
            let mut f = rc_game::moby_update::services::HeroFields::of(&play.game.hero);
            f.clear_motion();
            f.pose = Some(rc_game::moby_update::services::HeroPose { pos, yaw, target_yaw: yaw });
            f.call(rc_game::moby_update::services::HeroCall::SetState { id: 0, play: true });
            play.svc.hero_writes = Some((play.game.counter, f));
            let level = play.svc.level;
            let calls = &mut play.svc.cinematic.calls;
            if demo_exit {
                // VendorExit(1) after a weapon demo: `HeroTeleport(…, 0, 1)`: the follow camera reset behind Ratchet.
                calls.push(rc_game::cinematic::CinematicCall::CameraResetBehindHero);
            } else {
                calls.push(rc_game::cinematic::CinematicCall::CameraScript { pos: c.pos_f32(), euler: [0.0, 0.0, c.euler[2].to_f32()], mode: 1, ticks: 0, collide: false });
                calls.push(rc_game::cinematic::CinematicCall::CameraRelease { kind: 2, level });
            }
            if let Some(a) = audio.as_deref_mut() { a.system().hero_teleported(pos); }
        }
        // `VendorExit`: `snd_ContinueAllSoundsInGroup(0x1d)` (0x2ae7c0), `music_Unpause`.
        if let Some(a) = audio { a.system().menu_close(true); }
        play.game.camera_paused = false;
        println!("interact: frame {frame}: VendorExit: mode 0 (bolts {}, ammo {:?})", gs.global.bolts, &gs.global.ammo[10..20]);
        vr.vendor = None;
        vr.draw = VendorDraw::default();
        mm.state.set(Mode::Gameplay);
    }
    mm.world_tick = world_next;
}

/// The HUD's feed when no vendor is open (the prompt only).
pub fn feed_idle(feed: &mut crate::hud_render::HudFeed) {
    feed.bolts_pinned = false;
    feed.weapon = None;
}

/// Marks Ratchet's entities and his items hidden while the vendor is open.
#[derive(Component)]
pub struct VendorHidden;

/// `OpenVendorMenu`'s `0x1413f5 = 1` / `FUN_002486c0`: Ratchet and his items are hidden while the vendor is open (from
/// the camera cut on), shown again by `VendorExit` (`FUN_002487a8`). The tick that would do it through `HeroSyncMoby`
/// is the scene form in mode 5, so the entities are hidden here (as crate::scene_render does for mode 2).
#[allow(clippy::type_complexity)]
pub fn hide_hero(
    vr: Res<VendorRt>,
    mut commands: Commands,
    hero: Query<(Entity, &Name), (With<MeshMaterial3d<crate::moby_render::MobyMaterial>>, Without<crate::moby_attach::AttachedTo>, Without<VendorHidden>)>,
    items: Query<Entity, (With<crate::moby_attach::AttachedTo>, Without<VendorHidden>)>,
    mut hidden: Query<(Entity, &mut Visibility, Has<crate::gameplay::RatchetMesh>), With<VendorHidden>>,
) {
    let want = vr.vendor.as_ref().is_some_and(|v| v.pre_fade == 0);
    if want {
        for (e, name) in &hero {
            if name.as_str().starts_with("Ratchet (play)") { commands.entity(e).insert((VendorHidden, Visibility::Hidden)); }
        }
        for e in &items { commands.entity(e).insert((VendorHidden, Visibility::Hidden)); }
        for (_, mut v, _) in &mut hidden { if *v != Visibility::Hidden { *v = Visibility::Hidden; } }
    } else {
        // Ratchet's own meshes go back to crate::gameplay's visibility sync; his items are shown again.
        for (e, _, ratchet) in &hidden {
            if ratchet { commands.entity(e).remove::<VendorHidden>(); } else { commands.entity(e).remove::<VendorHidden>().insert(Visibility::Inherited); }
        }
    }
}
