//! The game tick in the engine: the ported on-foot hero, pad and follow camera (`rc_game::tick`,
//! docs/plan/player_controller.md §10 and "Engine wiring") driving Ratchet's instance and the view.
//!
//! * **Load** (`setup`, first `PreUpdate`), the level loader and load pass of `rc_game::moby_update`
//!   (docs/plan/moby_update_catalogue.md "In the port"): the class table (slot = index in the level core's
//!   class list, update function = the Rust port's level-table address or an external update's), the loader's
//!   **spawn test** on every instance (`rc_formats::moby_spawn::loader_spawns` with the level's save bytes from
//!   [`Persistent`]: mission bytes, killed bits; the rejected instances are not created and their entities are
//!   hidden), the static mobys from `scheduler::load_level_mobys` (the created instances in order: moby index =
//!   `0x1acc00[instance]`, pvar moby links remapped), `MobyTable::new(statics, spawnable count)` (gameplay moby
//!   section +4), the ship via `CreateMoby` in the first dynamic slot (hidden, mode |= 3, while the mission NPC's
//!   mission is open: the first arrival); `Game::new` (**`srand(1234)`**, hero init: Ratchet ground-snapped,
//!   mode 2, the fidget-timer draw; the camera snapped behind); the services (level, splines, the groups of
//!   gameplay +0x48 through instance → moby); then the **load pass** `Scheduler::load_pass` at counter 0 on the
//!   game's stream (the class-27 emitters see no view: culled, as the game's zero view before the first render),
//!   and the load's `0x15f5cc++` (`Game::finish_load`: the first tick runs at counter 1): the ported classes'
//!   state-0 inits (bolts, crates, grass), the external updates (class-27 emitters 0x2bd100 of
//!   crate::particle_render; the water managers are class ports, `rc_game::water::managers`), one `MobyAnimAdvance` of every
//!   non-mode-2 moby. crate::moby_spawn stays the source for the unported classes only (459, 572-family, 577,
//!   666, 730/790, 750, 1818, the ship): per moby the anim / position / mode come either from the table (a
//!   class with a ported update) or from its `SpawnState`, never both. Ratchet's animation is `RatchetAnim`
//!   on his class (sequences = the level's `ratchet_seq` table).
//! * **Tick** (`FixedUpdate`, 60 Hz): `Game::tick_with_sound` = pad → free-slot pass → mobys (hook: `Scheduler::tick`
//!   with the game camera of the previous tick, the level collision, the moby collision of `Services`, the
//!   particles, the render view of the last frame for `FastBSphereCheck`, the external updates) → hero (its
//!   queries test the mobys: `TickHooks::world` = `SharedServices`, docs/plan/collision_queries.md §7; then
//!   `MobyBuildMatrix` on Ratchet, re-registering him in the moby grid) → particles (hook: `UpdateParts` on the
//!   game's stream with the tick's camera as 0x167240, then the glints `FUN_00220928`) → camera (its queries
//!   test the mobys too; a crate blocking its line gets a hit) → sound step → counter; the sky stars draw
//!   after it (render phase). **One `rand` stream** (`Game::rng`): load pass → per tick the mobys in run order
//!   (incl. the 751 drops, the class-27 spawns and the class sounds' pitch bends) → hero → particles → camera →
//!   sound → counter → sky stars. Ticks per
//!   rendered frame follow the main loop's rule (`rc_game::tick::ticks_for_frame`: at most 2, the second only
//!   when the frame took longer than one field); in frame-exact mode exactly 1 per update.
//! * **Moby rendering after each tick.** Static instances of a ported class: position, rows, scale, light
//!   block (for the new rows / ambient) and hide (deleted or mode & 0x81) go to their `MobyInst` record and
//!   MobyProc inputs (`MobyOcclusion::drive`), the anim state and snapshot to their `MobyAnim` instance
//!   (`MobyAnim::drive`, never advanced there). Mobys the scheduler creates (dropped bolts, debris, flashes,
//!   icons) live in the dynamic slots and are drawn as extra instances ([`DynMobys`]: one record and palette
//!   range per dynamic slot, entities spawned per (slot, class) on first use and shown / hidden by the slot's
//!   state and MobyProc's draw-distance / near / frustum culls; high LOD, no fade). +0x31 is written back
//!   after drawing (`write_visible`): 1 when drawn, 0 on every skip (occlusion, culls, hidden, spawn-hidden).
//!   The moby sounds go to `rc_game::audio` through the moby loop's `SoundSink`
//!   (`audio::class_sounds::ClassSoundSink`: slot and pitch-bend draw at the moby's place in the loop, listener =
//!   the previous tick's camera) and are also counted per (class, index). The bolt counter goes to
//!   [`Persistent`] after each tick.
//! * **Sound step** (crate::audio_out): after the camera, before the counter increment, on the game's stream
//!   (`Game::tick_with_hero_sounds`, `audio::class_sounds::sound_step`): `sound_update` (the occlusion origin's 3
//!   draws every tick, the pitch bends of the sound instances' plays) and the frame's 800 samples. Ratchet's own
//!   sounds play inside the hero update (`class_sounds::HeroClassSounds`: his animation triggers right after his
//!   advance, his hurt / death voices after the transitions). Without audio (`RC_AUDIO=0`, or no sound data) the
//!   tick runs without a sound layer.
//! * **Idle and back items**: the hero gets the back items' classes (pack 607 and Clank 601,
//!   `Hero::set_back_classes`: created on the first hero update, then advanced and driven by the idle code:
//!   the back table, Clank's fidgets and blink), the level (`idle.level`, 0x15ed84) and, before every tick,
//!   the tick counter (`idle.counter`, 0x15f5cc); crate::moby_attach draws the back items from `Hero::back`.
//! * **After each tick**: Ratchet's moby (+0x10 position, +0xc0 rows, written back by the hero) is drawn
//!   by an extra moby instance of his class (crate::moby_render "Extra" block: a record + palette of its
//!   own, re-lit with his light word / ambient for the new rows); his gameplay-instance entities are hidden
//!   (`SpawnHidden`), since the static record cannot move and the static occlusion word of his spawn cell
//!   would hide him elsewhere. His `RatchetAnim` state and snapshot are written into his `MobyAnim` instance
//!   (with `skip_advance`, so the generic `MobyAnimAdvance` never touches him: mode 2), and his rows /
//!   position into crate::moby_attach, which then places the wrench, pack and Clank on the new pose (the
//!   pack's and Clank's animation from `Hero::back`).
//!   The camera goes to crate::play_camera.
//! * **Hits and death** (`rc_game::hero::damage`, docs/plan/hero_states.md P2): the tick hands Ratchet's hit
//!   message to the hero (the moby hit log, `MobySystem::hit_message`); after the tick the damage events go to
//!   the game state (hits 0x15eea8 / 0x13df88[level], deaths 0x15eeac / 0x13dfd8[level], the killer's mission
//!   deaths `LevelMissions::hero_death`; a death clears bit 31 of the help records, `0x225938`). **Respawn on the
//!   game's death flag** 0x141401 (`Hero::fell_out`, raised by the death sequence 0x2319b0 at the end of every
//!   death state, or by x/y outside 2..1022), never on entering a state: no catch-up tick, `FadeToBlack(16)` and the
//!   disc reload's black ([`RELOAD_BLACK`], `scene_render::FadeHold`), then on the next tick the death reload in the
//!   game's order: the mobys ([`death_reload`]), the hero side ([`respawn`]: the slot items kept but the headgear,
//!   the checkpoint record's place, light and body), the checkpoint's visit records, the load pass, the music from
//!   the checkpoint's track. `R` respawns on demand (the hero side only, e.g. out of a frozen unported state).
//!
//! * **Game state** (both modes, before the app runs): the persistent state and the session of a direct
//!   boot into this level, from the port of docs/plan/game_state.md (`rc_game::game_state`): new game from
//!   the disc's save template, Veldin's level start and Clank init, then for level N ≥ 1 the transition and
//!   N's level start (Novalis: HP 4/4, wrench held, bomb glove owned with 10 ammo, quick select [10, 0, …]).
//!   Published as the [`Persistent`] and [`Session`] resources (menus, HUD, scheduler); the tick reads its
//!   options (`Options::game_options`: mirror, camera yaw/pitch sense and rate), the mirrored-animation
//!   option and the HP (hero +0x15f8). Respawns redo the hero init (`SessionState::hero_init`).
//!
//! Environment: `RC_PLAY=0` disables all of this (fly camera only); `RC_PLAY_SCRIPT` feeds scripted pad input
//! (crate::input_map::Script); `RC_PLAY_TRACE=1` prints one line per tick (state, position, anim, camera, the
//! `rand` state, the moby loop's run count, bolts, free slots, dynamic mobys drawn / live, particles, grass on
//! sequence 1); `RC_DEBUG_HIT=moby@tick,...` delivers a hit (flags 0x10000, damage 1: breaks a crate) to those
//! mobys before the moby loop of those ticks (debug; the wrench itself now hits through the hero's hit sink);
//! `hero@tick` hits Ratchet (flags 1, damage 1, no attacker: the hit intake's knockback straight back, one HP).
//! The trace line ends with `| hp <health> inv <0x13f510>`.
//! * **Hand items and melee** (docs/plan/player_controller.md §12): the tick runs with `Game::item_data` (item
//!   definitions from the overlay's item table, the gadget classes, Ratchet's joint lists), the hand-swap globals
//!   synced with `Session::temp_hand` (the quick-select request) and `Persistent` (`equipped[0]`, `last_hand_item`,
//!   `wrench_held`), and a hit sink on the services' hit log ([`CellHits`]); after the tick the HUD's weapon slot
//!   ([`HeldWeapon`]) is derived from the held item.
//!
//! `RC_PLAY_FLY=1` starts on the fly camera (`RC_CAM`) with the game ticking (Tab switches as usual).

use crate::determinism::Deterministic;
use crate::fly_cam::FlyCam;
use crate::game_camera::CameraSource;
use crate::input_map::{self, PadFrame, Script};
use crate::moby_anim::MobyAnim;
use crate::moby_attach::{AttachedTo, MobyAttach};
use crate::moby_render::{self, ExtraMobys, MobyMaterial, MobyOcclusion};
use crate::moby_spawn::{MobySpawn, SpawnHidden};
use crate::particle_render::ParticleSim;
use crate::play_camera::{self, PlayView};
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::collision::Collision;
use rc_formats::save_game::{ChunkTables, ItemTables, SaveGameLump};
use rc_game::game_state::{GameState, SessionState};
use rc_formats::moby_anim::{self, MobyAnimClass};
use rc_formats::moby_light::{self as light, V4};
use rc_game::follow_camera::CameraView;
use rc_game::hero::anim::RatchetAnim;
use rc_game::hero::damage::DamageEvent;
use rc_game::hero::items::{HitSink, ItemClass, ItemData, ItemDef, ItemGlobals, HERO_LISTS};
use rc_game::audio::class_sounds::{self, ClassSoundSink, HeroClassSounds};
use rc_game::hero::{Hero, HeroTick};
use rc_game::moby_runtime::{Moby, MobyId, MobyTable, Seq0Info};
use rc_game::moby_update::scheduler::{self, Scheduler};
use rc_game::moby_update::services::{ExternalUpdates, HitTemplate, LevelMissions, ServiceHits, SharedServices, World};
use rc_game::moby_update::{ClassData, ClassTable, Services};
use rc_game::pad::PadInput;
use rc_game::particles::{type06, BSphereView, Particles};
use rc_game::rng::Rng;
use rc_game::tick::{ticks_for_frame, Game, GameOptions, TickHooks, FIELD_RCNT1};
use std::cell::RefCell;
use std::collections::HashMap;

/// Level-table address of the class-27 emitter update (Novalis only), crate::particle_render.
pub(crate) const EMITTER_UPDATE: u32 = 0x2bd100;

/// `RC_GIVE_ITEMS=<id>,...` (debug): the item ids to own from the start (decimal or `0x` hex; ids outside the
/// item table are ignored).
fn give_items() -> Option<Vec<usize>> {
    let v = std::env::var("RC_GIVE_ITEMS").ok()?;
    let ids: Vec<usize> = v
        .split(',')
        .filter_map(|t| { let t = t.trim(); t.strip_prefix("0x").map_or_else(|| t.parse().ok(), |h| usize::from_str_radix(h, 16).ok()) })
        .filter(|&i| i < rc_formats::save_game::ITEM_COUNT)
        .collect();
    (!ids.is_empty()).then_some(ids)
}

/// `RC_PLAY` (default on; `0` = fly camera only).
pub fn enabled() -> bool { !std::env::var("RC_PLAY").is_ok_and(|v| v.trim() == "0") }

/// The boot's game state: a direct boot into level `index` (`load_game_state`) with the debug grants applied. Built
/// once, before the boot's level load (its spawn bits, [`level_spawn_save`]), and handed to [`GameplayPlugin`].
pub fn boot_state(root: &std::path::Path, index: u32) -> Option<(GameState, SessionState)> {
    let (mut gs, mut sess) = match load_game_state(root, index) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("game state: not built ({e:#}); the tick uses the boot option defaults");
            return None;
        }
    };
    // RC_GIVE_HYDROPACK=1: own the Hydro-Pack (item 4) from the start (debug).
    if std::env::var("RC_GIVE_HYDROPACK").is_ok_and(|v| v.trim() == "1") {
        gs.global.owned[rc_game::hero::swim::ITEM_HYDRO_PACK] = 1;
        println!("game state: RC_GIVE_HYDROPACK=1: Hydro-Pack owned");
    }
    // RC_GIVE_ITEMS=<id>,<id>,...: own those items (decimal or 0x hex ids, docs/plan/gadgets.md §6) from the
    // start (debug, rc_game::inventory::debug_grant); unless RC_GIVE_ITEMS_EQUIP=0, the last back, feet
    // and head item among them are the saved items of their slots (equipped[3 / 1 / 2]: Clank wears the
    // pack, Ratchet the boots / head item), and the last hand item is requested into the hand. Without the
    // variable nothing is granted: the state is the game's.
    // RC_GIVE_BOLTS=<n>: start with n bolts (debug; e.g. to buy at the vendor).
    if let Some(n) = std::env::var("RC_GIVE_BOLTS").ok().and_then(|v| v.trim().parse::<i32>().ok()) {
        gs.global.bolts = n;
        println!("game state: RC_GIVE_BOLTS: {n} bolts");
    }
    // RC_UNLOCK_PLANETS=<p>,<p>,...: those planets unlocked (`UnlockPlanet` 0x2756d0: 0x13dd40 and the map order
    // 0x13d510) from the start (debug): the ship's planet page lists them (rc_game::travel).
    if let Ok(v) = std::env::var("RC_UNLOCK_PLANETS") {
        let ps: Vec<usize> = v.split(',').filter_map(|t| t.trim().parse().ok()).filter(|&p: &usize| p < 20).collect();
        for &p in &ps { gs.unlock_planet(p); }
        println!("game state: RC_UNLOCK_PLANETS: planets {ps:?} unlocked (map order {:?})", gs.global.map_order);
    }
    // RC_GAME_BEATEN=1: the game-beaten flag (save chunk 31) set (debug): the pause menu's Goodies entry (0x2917d8).
    if std::env::var("RC_GAME_BEATEN").is_ok_and(|v| v.trim() == "1") {
        gs.global.game_beaten = 1;
        println!("game state: RC_GAME_BEATEN=1: Goodies on the pause menu");
    }
    if let Some(ids) = give_items() {
        use rc_game::inventory::{debug_grant, GrantEquip};
        // With the ammo `GiveItem` grants (the item record's +0x12), so a given weapon can fire.
        let tables = crate::disc_source::read_path(root, &root.join("boot/SCUS_971.99")).ok()
            .zip(crate::disc_source::level_file(root, index, "overlay.bin").ok())
            .and_then(|(elf, ov)| ItemTables::load(&elf, &ov).ok());
        let mode = if std::env::var("RC_GIVE_ITEMS_EQUIP").is_ok_and(|v| v.trim() == "0") { GrantEquip::None } else { GrantEquip::LastPerSlot };
        let [hand, feet, head, back] = debug_grant(&mut gs, &mut sess, &ids, tables.as_ref(), mode);
        println!("game state: RC_GIVE_ITEMS: items {ids:?} owned ({mode:?}): saved back {back:?} / feet {feet:?} / head {head:?}, hand request {hand:?}");
    }
    let g = &gs.global;
    println!(
        "game state: direct boot into level {index:02} (new game → Veldin start → transition): level {}, HP {}/{}, wrench held {}, \
         equipped {:?}, quick select {:?}, bolts {}; options {:?}",
        g.level, sess.hp, g.max_hp, g.wrench_held, &g.equipped[..1], g.quick_select, g.bolts, gs.options()
    );
    Some((gs, sess))
}

/// The save bits the loader's spawn test reads for `level` (`scheduler::spawn_save` of its level section; a level
/// never visited, or no game state: all zero). The one source for the renderer's level load (crate::moby_spawn,
/// through crate::level_switch) and the game's moby table, so both create the same instances.
pub fn level_spawn_save(state: Option<&GameState>, level: u32) -> rc_formats::moby_spawn::SpawnSave {
    state.and_then(|s| s.levels.get(level as usize)).map(scheduler::spawn_save).unwrap_or_default()
}

/// The game tick; `boot` is the boot's game state ([`boot_state`]), taken by `build`.
pub struct GameplayPlugin {
    pub boot: std::sync::Mutex<Option<(GameState, SessionState)>>,
}

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        if let Some((gs, sess)) = self.boot.lock().unwrap_or_else(|e| e.into_inner()).take() {
            app.insert_resource(Persistent(gs)).insert_resource(Session(sess));
        }
        if !enabled() {
            println!("gameplay: RC_PLAY=0: fly camera only, no game tick");
            return;
        }
        let script = std::env::var("RC_PLAY_SCRIPT").ok().filter(|s| !s.trim().is_empty()).map(|s| match Script::parse(&s) {
            Ok(sc) => sc,
            Err(e) => panic!("RC_PLAY_SCRIPT: {e}"),
        });
        println!("{}", input_map::CONTROLS);
        if let Some(s) = &script { println!("gameplay: RC_PLAY_SCRIPT drives the pad for ticks 0..={} (keyboard and gamepad ignored)", s.end()); }
        // RC_PLAY_FLY=1: the game ticks but the view starts on the fly camera (RC_CAM), for looking at mobys.
        let fly = std::env::var("RC_PLAY_FLY").is_ok_and(|v| v.trim() == "1");
        // A runtime level change (crate::level_switch): the old level's play state dropped; `setup` builds the new one.
        app.add_systems(
            crate::level_switch::LevelUnload,
            (
                crate::level_switch::remove::<Play>,
                despawn_lod_parents,
                crate::level_switch::remove::<AmmoTable>,
                crate::level_switch::remove::<HeldWeapon>,
                crate::level_switch::remove::<PlayView>,
                crate::level_switch::reset::<TickBudget>,
            ),
        );
        app.insert_resource(if fly { CameraSource::Fly } else { CameraSource::Play })
            .insert_resource(PlayScript(script))
            .init_resource::<PadFrame>()
            .init_resource::<TickBudget>()
            .add_systems(
                PreUpdate,
                (setup, input_map::sample_system, set_budget, keys).chain().after(bevy::input::InputSystems),
            )
            .add_systems(FixedUpdate, tick.in_set(GameTick).before(crate::particle_render::tick))
            .add_systems(Update, (show_reloaded, debug_dump, respawn_probe))
            .add_systems(PostUpdate, ratchet_visibility.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate))
            .add_systems(RunFixedMainLoop, play_camera::apply.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop))
            .add_systems(
                PostUpdate,
                (
                    upload.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate).before(bevy::asset::AssetEventSystems),
                    write_visible.after(moby_render::update_moby_occlusion).after(upload),
                    caster_order.before(moby_render::update_moby_occlusion),
                ),
            );
    }
}

#[derive(Resource)]
struct PlayScript(Option<Script>);

/// The parent of one of Ratchet's LOD sets (`Play::ratchet_lods`; no mesh, so despawned here at a level change).
#[derive(Component)]
struct RatchetLod;

fn despawn_lod_parents(mut commands: Commands, q: Query<Entity, With<RatchetLod>>) {
    for e in &q { commands.entity(e).despawn(); }
}

/// The set of the gameplay tick system (crate::menu_render runs its frame after it).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameTick;

/// The persistent game state (what the memory card saves), `rc_game::game_state`.
#[derive(Resource)]
pub struct Persistent(pub GameState);

/// The session-only state (hero HP, temporary items, Clank hidden, tick scale).
#[derive(Resource)]
pub struct Session(pub SessionState);

/// The HUD's weapon slot input after the last tick (`Inputs::weapon`, rule 0x24f9c0): the held item
/// (0x140408) with its ammo and max ammo when its item record has an ammo HUD and the hero is on foot.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct HeldWeapon(pub Option<(u16, i32, i32)>);

/// Per item: has an ammo HUD (record +8 ≠ 0) and the max ammo (record +0xe), from the item records; and the item
/// definitions' weapon fields (`0x22ee08`), by id.
#[derive(Resource, Clone, Debug, Default)]
struct AmmoTable(Vec<(bool, u16)>, Vec<rc_game::hero::items::WeaponDef>);

/// The hand-item data, the ammo table (uses ammo, max) and the weapon fields of the item definitions.
type ItemTablesOut = (ItemData, Vec<(bool, u16)>, Vec<rc_game::hero::items::WeaponDef>, Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<(i16, Vec<u8>)>);

/// The level's hand-item data (item definitions from the overlay, the gadget classes, Ratchet's joint lists)
/// for `rc_game::hero::items`: the definitions at the overlay's item table (L01 0x179f40, found through
/// `GiveItem`), the classes from the gadget table (decompressed as `select_world_object_resource_tables`
/// would), their joint lists; with the ammo table (uses ammo, max) and the weapon fields of the definitions.
fn item_data(lv: &crate::level_load::LoadedLevel) -> anyhow::Result<ItemTablesOut> {
    use anyhow::Context;
    use rc_formats::gadget;
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let elf = crate::disc_source::read_path(&root, &root.join("boot/SCUS_971.99")).context("boot ELF")?;
    let overlay = crate::disc_source::level_file(&root, index, "overlay.bin").context("level overlay")?;
    let tables = ItemTables::load(&elf, &overlay)?;
    let sections = rc_formats::font::parse_overlay_sections(&overlay)?;
    let n = rc_formats::save_game::ITEM_COUNT;
    let sz = rc_formats::save_game::ITEM_DEF_SIZE;
    let raw = rc_formats::font::read_overlay(&sections, tables.item_defs_addr, n * sz).context("item definitions")?;
    let w = |i: usize, o: usize| i32::from_le_bytes(raw[i * sz + o..i * sz + o + 4].try_into().unwrap());
    let defs = (0..n).map(|i| ItemDef { slot: w(i, 8), attach: w(i, 0xc), o_class: w(i, 0x10), b18: raw[i * sz + 0x18] }).collect();
    // The weapon draw's fields (+0x18 word, the three sequences +0x24..+0x2c, +0x30): `0x22ee08`.
    let weapon_defs = (0..n).map(|i| rc_game::hero::items::WeaponDef { w18: w(i, 0x18), anims: [w(i, 0x24), w(i, 0x28), w(i, 0x2c)], w30: w(i, 0x30) }).collect();
    let (ratchet_blob, gadgets) = crate::moby_attach::load_blobs()?;
    let rc = lv.mobys.classes.iter().find(|c| c.o_class == gadget::RATCHET_O_CLASS).context("no class 0")?;
    let hero_chains = HERO_LISTS.iter().map(|&l| gadget::joint_list(&ratchet_blob, &rc.class.header, l).map(|(a, _)| a).unwrap_or_default()).collect();
    // The second byte lists of Ratchet's joint lists: the weapon arm layers' joints (`FUN_00263e08`, lists 12 / 13)
    // and the joint-modifier nodes' targets (`AttachManipulator`, their first entries).
    let seconds: Vec<Vec<u8>> = (0..64).map(|l| gadget::joint_list(&ratchet_blob, &rc.class.header, l).map(|(_, b)| b).unwrap_or_default()).collect();
    // The first byte lists (root-to-joint chains) of all his lists: the joint points of the hero's effects (the water
    // bubbles at his hands, feet and mouth: rc_game::hero::fx::joint_point).
    let firsts: Vec<Vec<u8>> = (0..64).map(|l| gadget::joint_list(&ratchet_blob, &rc.class.header, l).map(|(a, _)| a).unwrap_or_default()).collect();
    let mut classes = Vec::new();
    // The gadget classes' joint-list targets (the hand records and the Metal Detector's head node act on them:
    // rc_game::hero::gadgets::hand_modifiers).
    let mut item_targets = Vec::new();
    for g in &gadgets {
        let c = &g.moby.class;
        let seqs = rc_formats::moby_anim::parse_sequences(&g.blob, c).with_context(|| format!("gadget {} sequences", g.moby.o_class))?;
        let chains = (0..16).map_while(|l| gadget::joint_list(&g.blob, &c.header, l).ok().map(|(a, _)| a)).collect();
        let targets = (0..16).map_while(|l| gadget::joint_list(&g.blob, &c.header, l).ok().map(|(_, b)| rc_formats::moby_anim::list_target(&b).unwrap_or(0xff))).collect();
        item_targets.push((g.moby.o_class as i16, targets));
        classes.push(ItemClass { o_class: g.moby.o_class as i16, anim: rc_formats::moby_anim::MobyAnimClass::new(c, seqs), scale: c.header.scale, chains });
    }
    let ammo = tables.records.iter().map(|r| (r.has_ammo(), u16::from_le_bytes([r.0[0xe], r.0[0xf]]))).collect();
    Ok((ItemData { defs, hero_chains, classes }, ammo, weapon_defs, seconds, firsts, item_targets))
}

/// The level's water the hero's ground probe reads (`SetWaterLevel` 0x26ed38: the ripple module's active patches,
/// then the flat plane; `Services::water`, borrowed per query: the moby hook runs the managers earlier in the tick).
struct HeroWater<'w, 'r>(&'w RefCell<&'r mut Services>);

impl rc_game::hero::swim::WaterQuery for HeroWater<'_, '_> {
    fn water_height(&self, p: [f32; 3]) -> Option<f32> { self.0.borrow().water.water_height(p) }
}

/// The moby system hook plus the water tables.
struct HeroWorld<'a, 'b, 'w, 'r> {
    world: SharedServices<'a, 'b>,
    water: HeroWater<'w, 'r>,
}

impl rc_game::tick::MobySystem for HeroWorld<'_, '_, '_, '_> {
    fn scene(&mut self, table: &MobyTable) -> Option<rc_game::collision_query::OwnedScene> { self.world.scene(table) }
    fn build_matrix(&mut self, table: &mut MobyTable, id: MobyId) { self.world.build_matrix(table, id) }
    fn deliver_hit(&mut self, table: &mut MobyTable, target: MobyId, tmpl: &HitTemplate) { self.world.deliver_hit(table, target, tmpl) }
    fn water(&self) -> Option<&dyn rc_game::hero::swim::WaterQuery> { Some(&self.water) }
    fn hit_message(&self, table: &MobyTable, target: MobyId) -> Option<rc_game::moby_update::services::HitRecord> { self.world.hit_message(table, target) }
    fn take_hero_writes(&mut self) -> Option<rc_game::moby_update::services::HeroFields> { self.world.take_hero_writes() }
    fn spline(&self, i: usize) -> Option<Vec<[f32; 4]>> { self.world.spline(i) }
    fn take_camera_shakes(&mut self) -> Vec<rc_game::follow_camera::ShakeRequest> { self.world.take_camera_shakes() }
    fn take_cinematic(&mut self) -> Vec<rc_game::cinematic::CinematicCall> { self.world.take_cinematic() }
    fn run_list(&self, table: &MobyTable, camera: [rc_game::ps2v::Pf; 4]) -> Option<Vec<MobyId>> { self.world.run_list(table, camera) }
    fn volumes(&self) -> Option<std::sync::Arc<rc_formats::volumes::Volumes>> { self.world.volumes() }
    fn group(&self, g: i8) -> Vec<MobyId> { self.world.group(g) }
    fn create_moby(&mut self, table: &mut MobyTable, o_class: i16, counter: u64) -> Option<MobyId> { self.world.create_moby(table, o_class, counter) }
    fn delete_moby(&mut self, table: &mut MobyTable, id: MobyId, counter: u64) { self.world.delete_moby(table, id, counter) }
    fn game_mode(&self) -> i32 { self.world.game_mode() }
    fn board_world(&self, table: &MobyTable, board: MobyId) -> Option<rc_game::hero::hoverboard::BoardWorld> { self.world.board_world(table, board) }
    fn queue_board(&mut self, cmds: Vec<rc_game::hero::hoverboard::BoardCmd>) { self.world.queue_board(cmds) }
    fn view_tan(&self) -> f32 { self.world.view_tan() }
    fn set_view_tan(&mut self, v: f32) { self.world.set_view_tan(v) }
    fn set_letterbox(&mut self, on: bool) { self.world.set_letterbox(on) }
    fn queue_hero_state(&mut self, state: i32) { self.world.queue_hero_state(state) }
    fn queue_race(&mut self, out: rc_game::follow_camera::race::RaceOut) { self.world.queue_race(out) }
    fn fade(&self) -> f32 { self.world.fade() }
    fn set_fade(&mut self, v: f32) { self.world.set_fade(v) }
}

/// The hero's hit sink: the moby system's hit log and moby collision (borrowed per call: the moby hook borrows
/// the services too) and the level collision.
struct CellHits<'a, 'b, 'c> {
    svc: &'a RefCell<&'b mut Services>,
    classes: &'a dyn ClassData,
    coll: &'a Collision,
    /// The particles, for the hand items' calls into the moby world (the Suck Cannon's vortex strands).
    parts: Option<&'a RefCell<Option<&'b mut ParticleSim>>>,
    /// What else the moby loop's world has, for the hand items' calls into class code (the Suck Cannon's vacuum:
    /// `bolt::start_fly`'s pickup sound, `pickup::collect`'s `AddAmmo` and sound): the item tables (with Ratchet's
    /// ammo), the missions, the sound layer and the listener the hero update hears with, Ratchet's moby.
    items: Option<&'a rc_game::moby_update::classes::pickup::ItemTables>,
    missions: Option<&'a LevelMissions>,
    audio: Option<(&'a RefCell<Option<&'c mut crate::audio_out::AudioOut>>, rc_game::audio::voices::Listener, MobyId)>,
}

impl HitSink for CellHits<'_, '_, '_> {
    fn sphere(&mut self, table: &mut MobyTable, r: rc_game::ps2v::Pf, centre: [rc_game::ps2v::Pf; 4], flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<MobyId> {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.sphere(table, r, centre, flags, ignore, tmpl)
    }
    fn line(&mut self, table: &mut MobyTable, a: [rc_game::ps2v::Pf; 4], b: [rc_game::ps2v::Pf; 4], flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<Option<MobyId>> {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.line(table, a, b, flags, ignore, tmpl)
    }
    fn deliver(&mut self, table: &mut MobyTable, target: MobyId, tmpl: &HitTemplate) {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::services::deliver_hit_in(table, &mut s.hits, target, tmpl);
    }
    fn create_moby(&mut self, table: &mut MobyTable, o_class: i16, counter: u64) -> Option<MobyId> {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::classes::bomb::create_from_hero(table, &mut s, self.classes, o_class, counter)
    }
    fn delete_moby(&mut self, table: &mut MobyTable, id: MobyId, counter: u64) {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::classes::bomb::delete_from_hero(table, &mut s, id, counter);
    }
    // The line probe and the point lights of the hand items (the Pyrocitor): the services' own.
    fn probe(&mut self, table: &mut MobyTable, a: [rc_game::ps2v::Pf; 4], b: [rc_game::ps2v::Pf; 4], flags: u32, ignore: Option<MobyId>) -> Option<Option<[f32; 3]>> {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.probe(table, a, b, flags, ignore)
    }
    // The guns' rays and class filter (rc_game::hero::{blaster, devastator, ryno, tesla}).
    fn probe_moby(&mut self, table: &mut MobyTable, a: [rc_game::ps2v::Pf; 4], b: [rc_game::ps2v::Pf; 4], flags: u32, ignore: Option<MobyId>) -> Option<Option<rc_game::hero::items::Probe>> {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.probe_moby(table, a, b, flags, ignore)
    }
    fn class_type(&self, o_class: i16) -> Option<u8> { self.classes.info(o_class).map(|i| i.ty) }
    fn light_alloc(&mut self, l: rc_game::point_lights::PointLight) -> i32 {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.light_alloc(l)
    }
    fn light_get(&mut self, slot: i32) -> Option<rc_game::point_lights::PointLight> {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.light_get(slot)
    }
    fn light_set(&mut self, slot: i32, l: rc_game::point_lights::PointLight) {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.light_set(slot, l)
    }
    fn light_free(&mut self, slot: i32) {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.light_free(slot)
    }
    // The hand items' calls into class code (rc_game::moby_update::creature::react: the Suck Cannon, the Taunter).
    fn world(&mut self, table: &mut MobyTable, hero: &rc_game::hero::Hero, rng: &mut rc_game::rng::Rng, counter: u64, f: &mut dyn FnMut(&mut rc_game::moby_update::services::World)) -> bool {
        let mut s = self.svc.borrow_mut();
        let mut p = self.parts.map(|c| c.borrow_mut());
        let inv = self.items.map(|i| i.clone().with_hero(hero));
        let mut a = self.audio.map(|(c, l, h)| (c.borrow_mut(), l, h));
        let mut sink = a.as_mut().and_then(|(r, l, h)| r.as_deref_mut().map(|o| ClassSoundSink { audio: o.system(), listener: *l, hero: Some(*h) }));
        let mut w = rc_game::moby_update::services::World::new(table, hero, rng, self.classes, &mut s, counter);
        w.coll = Some(self.coll);
        w.particles = p.as_deref_mut().and_then(|p| p.as_deref_mut()).map(|p| &mut p.sys);
        if let Some(i) = &inv { w.inventory = i; }
        if let Some(m) = self.missions { w.missions = m; }
        w.sound = sink.as_mut().map(|s| s as &mut dyn rc_game::moby_update::services::SoundSink);
        f(&mut w);
        true
    }
}

/// The level's moby class collision blobs (class header +0x10), from the core (re-read here: the level loader
/// keeps no raw class blobs).
fn moby_collision_blobs(index: u32) -> anyhow::Result<Vec<(i32, rc_formats::moby_collision::MobyCollision)>> {
    use anyhow::Context;
    let root = crate::level_load::extracted_root();
    let idx = crate::disc_source::level_file(&root, index, "core_index.bin").context("core index")?;
    let data = rc_data::level_core_data(&root, index).context("decompressing core data")?;
    let core = rc_formats::level::parse_level_core(&idx, data.len()).context("parsing core index")?;
    Ok(rc_formats::moby_collision::parse_level(&core, &data)?)
}

/// The collision blob of spaceship class `o_class` (530..=533) from the global `spaceships` file (entry o_class − 530).
fn spaceship_collision(o_class: i32) -> anyhow::Result<Option<rc_formats::moby_collision::MobyCollision>> {
    anyhow::ensure!((530..=533).contains(&o_class), "class {o_class} is not a spaceship class");
    let root = crate::level_load::extracted_root();
    let file = crate::disc_source::read(&root, &format!("global/spaceships/{:03}.bin", o_class - 530))?;
    let s = rc_formats::moby_spawn::parse_spaceship(&file)?;
    Ok(rc_formats::moby_collision::MobyCollision::of_class(s.class)?)
}

/// The hand-swap globals from the saved game and the session (0x141408 request, 0x141660 saved hand item,
/// 0x15ed8c previous, 0x15ed90 wrench flag).
fn item_globals(state: Option<&GameState>, session: Option<&SessionState>) -> ItemGlobals {
    let mut g = ItemGlobals { wrench_flag: 1, ..Default::default() };
    if let Some(gs) = state {
        g.saved = gs.global.equipped[0];
        g.previous = gs.global.last_hand_item;
        g.wrench_flag = gs.global.wrench_held;
    }
    if let Some(s) = session {
        g.request = s.temp_hand;
        g.drone = s.drone;
    }
    g
}

/// The state of a direct boot into level `index`: new game, Veldin's level start and Clank init, then
/// (index ≥ 1) the transition and this level's start (docs/plan/game_state.md §4).
fn load_game_state(root: &std::path::Path, index: u32) -> anyhow::Result<(GameState, SessionState)> {
    use anyhow::Context;
    let elf = crate::disc_source::read_path(root, &root.join("boot/SCUS_971.99")).context("boot ELF")?;
    let lump = SaveGameLump::parse(&crate::disc_source::read(root, "global/save_game.bin").context("save_game lump")?)?;
    let items = |l: u32| -> anyhow::Result<ItemTables> {
        Ok(ItemTables::load(&elf, &crate::disc_source::level_file(root, l, "overlay.bin").with_context(|| format!("level {l:02} overlay"))?)?)
    };
    let mut gs = GameState::new_game(ChunkTables::from_boot_elf(&elf)?, &lump.template)?;
    let mut sess = SessionState::default();
    gs.apply_level_start(0, &items(0)?, &mut sess);
    if index >= 1 {
        gs.on_veldin_clank_init(&mut sess);
        gs.apply_transition(index as i32);
        gs.apply_level_start(index as i32, &items(index)?, &mut sess);
    }
    Ok((gs, sess))
}

/// Gameplay ticks still allowed this rendered frame (the main loop's catch-up rule).
#[derive(Resource, Default)]
struct TickBudget(u32);

/// The black frames between the death fade and the respawn: the PS2's `LoadLevelCoreData(0, 1)` re-reads the level
/// from the disc with the screen black (measured on the original: 0.70 s after the 16-frame fade; the port's reload
/// takes a frame, so the black is held for this many frames).
pub const RELOAD_BLACK: u32 = 42;

/// The running game and Ratchet's draw state.
#[derive(Resource)]
pub struct Play {
    pub game: Game,
    pub ratchet: RatchetAnim,
    /// The body moby's hero animation while a body is the hero moby (`rc_game::hero::bodies`, bound through
    /// `rc_game::hero::anim::HeroAnimCtl`).
    pub body_anim: Option<rc_game::hero::anim::BodyAnim>,
    /// Ratchet's moby (`0x1acc00[his gameplay instance]`) and animation instance.
    hero_id: MobyId,
    hero_k: Option<usize>,
    /// His class (index into `LevelMobys::classes` / `anim`), moby+0x2c scale, palette slots.
    class: usize,
    scale: f32,
    slots: u32,
    light_word: u32,
    ambient: [u8; 3],
    /// His moby as the level places it, for respawns.
    spawn_moby: Moby,
    extra: ExtraMobys,
    entities: Vec<Entity>,
    /// The parents of his high- and low-LOD entities (all in `entities` with the metal ones): MobyProc's LOD pick
    /// shows one set; `ratchet_low` is empty without a low LOD.
    ratchet_lods: [Entity; 2],
    ratchet_has_low: bool,
    ratchet_lod_low: bool,
    uploaded: Option<u64>,
    /// The moby loop: scheduler state, the class table (shared with the hero's per-tick collision scene), the
    /// services it owns.
    pub sched: Scheduler,
    pub classes: std::sync::Arc<ClassTable>,
    pub svc: Services,
    /// Static mobys of a class with a ported update (drawn from the table) with their gameplay instance and
    /// `MobyAnim` instance, and
    /// the light block inputs they were last lit with.
    driven: Vec<(MobyId, usize, Option<usize>)>,
    /// The other static mobys with their gameplay instance and what they were loaded with ([`StaticSig`]): one that
    /// another class moves, turns, hides or deletes (Blarg's button 1118 turning its doors 1145 open) joins
    /// [`Self::driven`] from then on ([`promote_statics`]).
    watched: Vec<(MobyId, usize, StaticSig)>,
    /// Static moby → gameplay instance (the renderer's records, entities and occlusion are per instance).
    moby_to_instance: Vec<usize>,
    /// The level's mission state (0x15fc88, 0x14c050, the deaths 0x14ee90): kept across respawns (the death
    /// reload keeps the deaths; the death sequence bumps the killer's mission, `DamageEvent::Died`).
    pub missions: LevelMissions,
    lit: HashMap<MobyId, ([u32; 9], [u8; 3], u32)>,
    /// The loader's ship: (its table id, its gameplay-instance index), drawn by the static path.
    ship: Option<(MobyId, usize)>,
    /// Class-27 emitter moby → `Particles::owners` index.
    emitters: HashMap<MobyId, usize>,
    level: u32,
    dynamic: DynMobys,
    /// Moby sounds queued so far (drained from `Services::sounds` every tick), per (class, index).
    sounds: HashMap<(i16, i32), u64>,
    /// `RC_DEBUG_HIT=moby@tick,...` (`hero@tick` = Ratchet): hits delivered to those mobys at those ticks.
    debug_hits: Vec<(MobyId, u64)>,
    /// Option 0x15edb5 (mirrored animation), for respawns.
    mirror_anim: bool,
    /// The hand-item data (also given to every respawned `Game`).
    item_data: Option<ItemData>,
    /// The back items' classes (the packs by back item id, Clank 601), given to every (respawned) hero.
    back_classes: Option<BackPacks>,
    trace: bool,
    respawn: bool,
    /// The death flag 0x141401 was set: the fade and the reload's black run (`scene_render::FadeHold`), the reload
    /// itself ([`death_reload`], [`respawn`]) on the first tick after them.
    pub death_pending: bool,
    frozen_hint: bool,
    /// Ratchet's entities hidden (his moby's mode bit 1: the first-person view hides him, `HeroSyncMoby`).
    ratchet_hidden: bool,
    /// Dev builds: the noclip position (N toggles; [`noclip_step`]).
    noclip: Option<[f32; 3]>,
    /// His caster draws' order as last set (crate::moby_render `queue_late`; None: as spawned, late).
    ratchet_late: Option<bool>,
    /// The instances a death reload made spawn that the level's load did not (their entities still hidden): shown by
    /// [`show_reloaded`].
    reload_shown: Vec<usize>,
    /// The weapon arm layers' joints (Ratchet's joint lists 12 / 13, `rc_game::hero::weapons::ARM_LISTS`).
    arm_joints: [Vec<u8>; 2],
    /// The help voice line loaded for the dialogue player (rc_game::help::VoiceCmd::Load), played on its `Play`.
    help_vag: Option<std::sync::Arc<[u8]>>,
    /// `RC_HUD_HELP=<id>`: the help message forced at the first tick (dev switch; the first-input gate skipped).
    help_debug: Option<i32>,
}

impl Play {
    /// The loader's ship in the moby table (`0x13e030`; crate::scene_render hides / shows it for the mission NPC).
    pub fn ship_moby(&self) -> Option<MobyId> { self.ship.map(|s| s.0) }
}

/// The loaded level's ported class updates (`rc_game::moby_update::classes::LevelPorts`, docs/plan/level_generalisation.md
/// C1): the level's class table `lvl.vtbl` matched against the ports' reference functions (the level-01 overlay, level
/// 03's for the swing target), with the engine's external updates (emitter, ripple manager). Built once per level;
/// without the overlays, every class by number.
pub fn level_ports() -> &'static rc_game::moby_update::classes::LevelPorts {
    use rc_formats::level_overlay::LevelOverlay;
    use rc_game::moby_update::classes::LevelPorts;
    // One table per level (crate::level_switch: the loaded level changes at run time).
    static PORTS: crate::level_load::PerLevel<LevelPorts> = [const { std::sync::OnceLock::new() }; 20];
    crate::level_load::per_level(&PORTS).get_or_init(|| {
        let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
        let overlay = |l: u32| -> Option<std::sync::Arc<LevelOverlay>> {
            let b = crate::disc_source::level_file(&root, l, "overlay.bin").ok()?;
            LevelOverlay::parse(&b).ok().map(std::sync::Arc::new)
        };
        let Some(target) = overlay(index) else {
            eprintln!("gameplay: level {index:02} overlay not read: class ports by class number");
            return LevelPorts::by_class_number();
        };
        LevelPorts::from_overlays(&target, &overlay, &[EMITTER_UPDATE])
    })
}

/// The help system's part of the tick (rc_game::help): the hero's help calls and record writes (the Pyrocitor, the
/// Tesla Claw, the first-person look), the hero update's own hints `0x228498` (its normal update only), then
/// `Help_Update` with `force_help_message(5, 0)` (`InLevelFrameUpdate` only: the scene / vendor frames run just the
/// dialogue player's step), its opening sound and the voice line (`help_audio[lang·150 + n]`), and the records and
/// log back into the saved game.
fn help_frame(p: &mut Play, report: &rc_game::tick::TickReport, other_frame: bool, gs: Option<&mut GameState>, audio: Option<&mut crate::audio_out::AudioOut>) {
    use rc_game::help::{self, HelpInputs, VoiceCmd};
    help::apply_hero(&mut p.svc.help, &mut p.game.hero.help);
    let h = &p.game.hero;
    if !other_frame && h.mode == 0 && matches!(report.hero, HeroTick::Ran) {
        let types: Vec<i32> = p.item_data.as_ref().map_or_else(Vec::new, |d| d.defs.iter().map(|d| d.slot).collect());
        let counter = p.game.counter as i32 - 1;
        for (msg, rec) in help::hero_hints(h.state, h.group, &h.owned.0, &types, counter, &p.svc.help.records) { p.svc.help.request(msg, rec); }
    }
    if let Some(id) = p.help_debug.take() {
        p.svc.help.bx.enabled = true;
        let rec = p.svc.help.log_index(id).unwrap_or(0) as i32;
        p.svc.help.request = -1;
        p.svc.help.request(id, rec);
    }
    if other_frame {
        p.svc.help.voice_frame();
    } else {
        let (text_on, voice_on) = (p.svc.help.bx.text_on, p.svc.help.bx.voice_on);
        let inp = HelpInputs { mode: p.svc.game_mode, held: p.game.pad.held, pressed: p.game.pad.pressed, play_time: p.svc.help.play_time, level: p.level as i32, text_on, voice_on };
        help::tick(&mut p.svc, &inp);
    }
    let out = std::mem::take(&mut p.svc.help.out);
    let mut audio = audio;
    if let Some(a) = audio.as_deref_mut() {
        let listener = rc_game::audio::class_sounds::listener_of(&p.game.camera.out);
        let mut rng = rc_game::rng::Rng::new();
        for &(index, flags) in &out.sounds { a.system().play_level_sound_at_moby(index, flags, None, None, &listener, &mut rng, p.game.counter); }
    }
    for cmd in out.voice {
        match cmd {
            VoiceCmd::Load { id } => {
                // `PlayDialogue`'s stream (the help lines, Qwark's boss lines, the level's lines: rc_game::help::dialogue_stream).
                let file = help::dialogue_stream(id, p.svc.help.text.lang, p.svc.level);
                let root = crate::level_load::extracted_root();
                let vag = file.as_ref().and_then(|f| crate::disc_source::read(&root, f).ok());
                let len = vag.as_deref().and_then(help::vag_ticks);
                println!("help: tick {}: voice line {id} ({}): {}", p.game.counter, file.as_deref().unwrap_or("no stream"), len.map_or("missing".to_string(), |t| format!("{t} ticks")));
                p.help_vag = vag.filter(|_| len.is_some()).map(std::sync::Arc::from);
                p.svc.help.voice_loaded(len);
            }
            VoiceCmd::Play { audible } => {
                if let (Some(vag), Some(a), true) = (p.help_vag.clone(), audio.as_deref_mut(), audible) {
                    a.system().scene_command(rc_game::audio::scene::SceneAudioCmd::Speech { vag });
                }
            }
            VoiceCmd::Stop => {
                // Only a help line still on the speech voice in gameplay (a scene's own speech replaces it).
                if p.help_vag.take().is_some() && p.svc.game_mode == 0 {
                    if let Some(a) = audio.as_deref_mut() { a.system().scene_command(rc_game::audio::scene::SceneAudioCmd::StopSpeech); }
                }
            }
        }
    }
    if let Some(gs) = gs {
        p.svc.help.sync_out(gs);
        // The persistent death bits and the bolt-drop slots, written into the save's level chunks as the game's writes are.
        p.svc.sync_save(gs);
    }
}

/// The cheat entry's move patterns 0x179b80 (`rc_game::cheats::CheatTables`) from the level's overlay (relocated against
/// level 01's); twelve empty patterns when the overlay cannot be read (no cheat matches).
fn cheat_patterns() -> Vec<Vec<u8>> {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let reference = crate::disc_source::level_file(&root, 1, "overlay.bin");
    let ov = crate::disc_source::level_file(&root, index, "overlay.bin").ok().and_then(|b| match &reference {
        Ok(r) => rc_game::menus::Overlay::relocated(&b, r).ok(),
        Err(_) => rc_game::menus::Overlay::parse(&b).ok(),
    });
    match ov {
        Some(ov) => rc_game::cheats::CheatTables::read(&ov).patterns,
        None => vec![Vec::new(); rc_game::cheats::SLOTS],
    }
}

/// The help system's level data (rc_game::help): the level text and the small font's glyphs (the box sizing), and the
/// help log's id table 0x1798d0 from the level's overlay (relocated against level 01's).
fn help_setup(svc: &mut Services, lv: &crate::level_load::LoadedLevel, index: u32) {
    if let Some(h) = lv.hud.as_ref() {
        svc.help.text = rc_game::help::HelpText { messages: std::sync::Arc::new(h.messages.clone()), small: Some(h.glyphs[rc_formats::font::Font::Small as usize]), lang: h.lang };
    }
    let root = crate::level_load::extracted_root();
    let (Ok(target), reference) = (crate::disc_source::level_file(&root, index, "overlay.bin"), crate::disc_source::level_file(&root, 1, "overlay.bin").ok()) else {
        eprintln!("gameplay: overlay not read: no help log table");
        return;
    };
    svc.help.log_ids = std::sync::Arc::new(rc_game::help::load_log_ids(&target, reference.as_deref()));
}

/// The level entry's map part (`FUN_0025a4c0`, rc_game::map): the overlay's tables, the level's map file (the
/// Map-o-Matic set when owned[33]) and its fog mask (the saved chunk 3002, else the zone tiles).
fn map_setup(svc: &mut Services, index: u32, gs: Option<&GameState>) {
    let root = crate::level_load::extracted_root();
    let (Ok(target), Ok(reference)) = (crate::disc_source::level_file(&root, index, "overlay.bin"), crate::disc_source::level_file(&root, 1, "overlay.bin")) else {
        eprintln!("gameplay: overlay not read: no map");
        return;
    };
    let Some(tables) = rc_game::menus::Overlay::relocated(&target, &reference).ok().and_then(|ov| rc_game::map::Tables::read(&ov)) else {
        eprintln!("gameplay: map tables not found: no map");
        return;
    };
    let owned = gs.is_some_and(|g| g.global.owned[rc_game::map::MAP_O_MATIC] != 0);
    let file = map_file(rc_game::map::file_index(index as usize, owned));
    let saved = gs.and_then(|g| g.levels.get(index as usize)).map_or(Vec::new(), |l| l.map_mask.to_vec());
    svc.map = rc_game::map::MapState::enter(&tables, index as i32, owned, file, &saved);
    println!(
        "gameplay: map {} (file {}), {} of {} pixels fogged",
        if svc.map.file.is_some() { "loaded" } else { "missing" },
        rc_game::map::file_index(index as usize, owned),
        svc.map.mask.fogged_count(),
        rc_game::map::SIZE * rc_game::map::SIZE
    );
}

/// Map file `index` of the global TOC field 0x820 (`global/unknown_0820/NNN.bin`), WAD-decompressed.
pub fn map_file(index: usize) -> Option<rc_game::map::MapFile> {
    let root = crate::level_load::extracted_root();
    let f = rc_formats::disc::RAC1_GLOBAL_FIELDS.iter().find(|f| f.offset as u32 == rc_game::map::FIELD)?;
    let raw = crate::disc_source::read(&root, &format!("global/{}/{index:03}.bin", f.name)).ok()?;
    let bytes = if rc_formats::wad::is_wad(&raw) { rc_formats::wad::decompress(&raw).ok()? } else { raw.to_vec() };
    rc_game::map::MapFile::parse(bytes)
}

/// The fog writer after the hero update (`FUN_0025c4f8` from 0x228a1c unless movement group 22 or state 50, and
/// always from the alternative update 0x228088 of the other player modes).
fn map_tick(p: &mut Play, report: &rc_game::tick::TickReport, gs: Option<&GameState>) {
    if !matches!(report.hero, HeroTick::Ran) { return; }
    let h = &p.game.hero;
    if h.mode == 0 && (h.group == 22 || h.state == 50) { return; }
    let inp = rc_game::map::FogInput {
        pos: [h.pos[0].to_f32(), h.pos[1].to_f32(), h.pos[2].to_f32()],
        level: p.level as i32,
        alt: p.svc.map.alt,
        group: h.group,
        f0634: h.f0634,
        magnetic: h.f658,
    };
    let flags = gs.map_or([0u8; 128], |g| g.global.flags);
    p.svc.map.reveal(&inp, &flags);
}

/// The loaded level's ported camera classes (`rc_game::follow_camera::level::CameraPorts`: the level's `lvl.camvtbl`
/// against the reference levels' code). Built once per process; the default (level 01's) without the overlays.
pub fn camera_ports() -> &'static rc_game::follow_camera::level::CameraPorts {
    use rc_formats::level_overlay::LevelOverlay;
    static P: crate::level_load::PerLevel<rc_game::follow_camera::level::CameraPorts> = [const { std::sync::OnceLock::new() }; 20];
    crate::level_load::per_level(&P).get_or_init(|| {
        let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
        let overlay = |l: u32| -> Option<LevelOverlay> { LevelOverlay::parse(&crate::disc_source::level_file(&root, l, "overlay.bin").ok()?).ok() };
        match (overlay(index), overlay(1)) {
            (Some(t), Some(_)) => rc_game::follow_camera::level::CameraPorts::from_overlays(&t, &|l| overlay(l).map(std::sync::Arc::new)),
            _ => Default::default(),
        }
    })
}

/// The loaded level's ported class reaction tables (`rc_game::moby_update::creature::react::tables_from_overlays`: the
/// level's `lvl.vtbl` third words against level 01's tables and the ones reversed on other levels). Built once per process; empty without the overlays (the
/// class-number fallback).
pub fn level_reactions() -> &'static std::collections::HashMap<i16, rc_game::moby_update::creature::react::Table> {
    use rc_formats::level_overlay::LevelOverlay;
    static T: crate::level_load::PerLevel<std::collections::HashMap<i16, rc_game::moby_update::creature::react::Table>> = [const { std::sync::OnceLock::new() }; 20];
    crate::level_load::per_level(&T).get_or_init(|| {
        let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
        let overlay = |l: u32| -> Option<LevelOverlay> { LevelOverlay::parse(&crate::disc_source::level_file(&root, l, "overlay.bin").ok()?).ok() };
        let reference = |l: u32| overlay(l).map(std::sync::Arc::new);
        match (overlay(index), overlay(1)) {
            (Some(t), Some(r)) => rc_game::moby_update::creature::react::tables_from_overlays(&t, &r, &reference),
            _ => Default::default(),
        }
    })
}

/// The external updates' level-table address for `o_class` (`ExternalUpdates::update_fn`, also used to build
/// the class table before the externals exist): the class's table entry is the emitter 0x2bd100.
fn external_update_fn(o_class: i16) -> Option<u32> {
    match level_ports().external(o_class) {
        Some(EMITTER_UPDATE) => Some(EMITTER_UPDATE),
        _ => None,
    }
}

/// Class updates ported outside `rc_game::moby_update`, run by the scheduler at their place in the moby order
/// on the game's stream: the class-27 emitters (`type06::emitter_update_live` 0x2bd100 on the emitter's live
/// moby, which the Blarg flyers 660 move every tick, into the world's particles, with the view of the last rendered
/// frame). (The water managers are class ports: `rc_game::water::managers`.)
struct Externals<'a> {
    level: u32,
    emitters: &'a HashMap<MobyId, usize>,
    view: Option<&'a BSphereView>,
}

impl ExternalUpdates for Externals<'_> {
    fn update_fn(&self, o_class: i16) -> Option<u32> { external_update_fn(o_class) }

    fn update(&mut self, addr: u32, id: MobyId, table: &mut MobyTable, rng: &mut Rng, _camera: [rc_game::ps2v::Pf; 4], _counter: u64, particles: Option<&mut Particles>) {
        if addr == EMITTER_UPDATE {
            if let (Some(p), Some(&o)) = (particles, self.emitters.get(&id)) { type06::emitter_update_live(p, rng, o, self.view, self.level, &table.mobys[id]); }
        }
    }
}

/// The class table of the loaded level: every class of the core's class list (slot = its index there,
/// `MobyClassesRelocate`), its `InitMobyInstance` view (a class without a blob: no header), the update address
/// (a Rust port, an external update, else none → mode 2) and its sequences. A class loaded from elsewhere (the ship from the spaceships file)
/// gets the next slot (inferred).
fn class_table(lv: &crate::level_load::LoadedLevel, ext: &dyn Fn(i16) -> Option<u32>) -> ClassTable {
    let m = &lv.mobys;
    let mut t = ClassTable::default();
    let mut slot_of: HashMap<i32, usize> = lv.moby_class_order.iter().enumerate().rev().map(|(s, &o)| (o, s)).collect();
    let mut next = lv.moby_class_order.len();
    for (c, a) in m.classes.iter().zip(&m.anim) {
        let slot = *slot_of.entry(c.o_class).or_insert_with(|| { next += 1; next - 1 });
        let oc = c.o_class as i16;
        let mut info = scheduler::class_info(&c.class, slot as u8, level_ports().update_fn(oc).or_else(|| ext(oc)));
        info.seq0 = a.sequence(0).map(|q| Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
        t.classes.insert(oc, (info, Some(a.clone())));
    }
    // Slots without a class blob (no header, e.g. the class-27 emitters and 751): InitMobyInstance keeps the
    // slot's update function and sets mode |= 5.
    for (slot, &o) in lv.moby_class_order.iter().enumerate() {
        let oc = o as i16;
        t.classes.entry(oc).or_insert_with(|| {
            let info = rc_game::moby_runtime::ClassInfo { slot: slot as u8, no_header: true, update_fn: level_ports().update_fn(oc).or_else(|| ext(oc)), ..Default::default() };
            (info, None)
        });
    }
    t
}

/// The joint lists of the classes whose update reads joint points (`rc_game::moby_update::classes::needs_joint_lists`,
/// `Services::joint_lists`), from their blobs in the level core (as `menu_render::load_frame_class`).
/// With them, per class, each list's manipulator target joint (`Services::joint_targets`, `rc_game::moby_update::manip`).
type JointLists = (HashMap<i16, Vec<Vec<u8>>>, HashMap<i16, Vec<u8>>);
fn class_joint_lists(lv: &crate::level_load::LoadedLevel) -> anyhow::Result<JointLists> {
    let both = class_joint_data_where(lv, |o| level_ports().needs_joint_lists(o))?;
    let targets = both.iter().map(|(o, l)| (*o, l.iter().map(|(_, s)| rc_formats::moby_anim::list_target(s).unwrap_or(0xff)).collect())).collect();
    Ok((both.into_iter().map(|(o, l)| (o, l.into_iter().map(|(a, _)| a).collect())).collect(), targets))
}

/// [`class_joint_lists`] for the level's classes `want` picks.
/// Ratchet's glove-holding classes 1 / 2 (`rc_game::hero::anim::HoldClass`, `0x22e660`) from the level's core blocks;
/// None where one is missing (those layers are then not drawn).
fn hold_classes() -> [Option<std::sync::Arc<rc_game::hero::anim::HoldClass>>; 2] {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let Ok(data) = rc_data::level_core_data(&root, index) else { return [None, None] };
    let Some(core) = crate::disc_source::level_file(&root, index, "core_index.bin").ok().and_then(|i| rc_formats::level::parse_level_core(&i, data.len()).ok()) else { return [None, None] };
    rc_game::hero::anim::HOLD_CLASSES.map(|oc| {
        let blk = core.blocks.iter().find(|b| b.name == format!("moby_class/{oc:04}"))?;
        let c = rc_game::hero::anim::HoldClass::parse(data.get(blk.offset..blk.offset + blk.size)?);
        if c.is_none() { eprintln!("gameplay: holding class {oc} does not parse: no glove-holding layer on its arm"); }
        c.map(std::sync::Arc::new)
    })
}

fn class_joint_lists_where(lv: &crate::level_load::LoadedLevel, want: impl Fn(i16) -> bool) -> anyhow::Result<HashMap<i16, Vec<Vec<u8>>>> {
    Ok(class_joint_data_where(lv, want)?.into_iter().map(|(o, l)| (o, l.into_iter().map(|(a, _)| a).collect())).collect())
}

/// Per class `want` picks: each joint list's two byte lists (`rc_formats::gadget::joint_list`).
/// A class's joint lists: (first byte list, second byte list) per list.
type ClassLists = HashMap<i16, Vec<(Vec<u8>, Vec<u8>)>>;
fn class_joint_data_where(lv: &crate::level_load::LoadedLevel, want: impl Fn(i16) -> bool) -> anyhow::Result<ClassLists> {
    use anyhow::{anyhow, Context};
    let wanted: Vec<&rc_formats::moby::LevelMobyClass> = lv.mobys.classes.iter().filter(|c| want(c.o_class as i16)).collect();
    let mut out = HashMap::new();
    if wanted.is_empty() { return Ok(out); }
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let read = |name: &str| crate::disc_source::level_file(&root, index, name);
    let data = rc_data::level_core_data(&root, index).context("decompressing core_data")?;
    let core = rc_formats::level::parse_level_core(&read("core_index.bin")?, data.len()).context("parsing core index")?;
    for c in wanted {
        let name = format!("moby_class/{:04}", c.o_class);
        let blk = core.blocks.iter().find(|b| b.name == name).ok_or_else(|| anyhow!("no {name} block"))?;
        let blob = data.get(blk.offset..blk.offset + blk.size).ok_or_else(|| anyhow!("{name} out of range"))?;
        let lists = (0..256).map_while(|l| rc_formats::gadget::joint_list(blob, &c.class.header, l).ok()).collect();
        out.insert(c.o_class as i16, lists);
    }
    Ok(out)
}

/// The gait records (`rc_formats::moby_anim::gait_records`) of the level's classes `want` picks, by (class, sequence).
fn class_gaits(lv: &crate::level_load::LoadedLevel, want: impl Fn(i16) -> bool) -> anyhow::Result<HashMap<(i16, u8), [u32; 20]>> {
    use anyhow::Context;
    let wanted: Vec<&rc_formats::moby::LevelMobyClass> = lv.mobys.classes.iter().filter(|c| want(c.o_class as i16)).collect();
    let mut out = HashMap::new();
    if wanted.is_empty() { return Ok(out); }
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let data = rc_data::level_core_data(&root, index).context("decompressing core_data")?;
    let core = rc_formats::level::parse_level_core(&crate::disc_source::level_file(&root, index, "core_index.bin")?, data.len()).context("parsing core index")?;
    for c in wanted {
        let name = format!("moby_class/{:04}", c.o_class);
        let Some(blob) = core.blocks.iter().find(|b| b.name == name).and_then(|b| data.get(b.offset..b.offset + b.size)) else { continue };
        for (seq, rec) in rc_formats::moby_anim::gait_records(blob, &c.class) { out.insert((c.o_class as i16, seq), rec); }
    }
    Ok(out)
}

/// What the renderer draws a static moby with: rows, position, scale, mode, state and alpha (as bits).
type StaticSig = ([u32; 16], u16, u8, u8);

fn static_sig(m: &Moby) -> StaticSig {
    let mut v = [0u32; 16];
    for i in 0..3 { for j in 0..3 { v[i * 3 + j] = m.rows[i][j].to_bits(); } }
    for j in 0..3 { v[9 + j] = m.position[j].to_bits(); }
    v[12] = m.scale.to_bits();
    (v, m.mode, m.state, m.alpha)
}

/// The static mobys not in `driven`, with their instance and their loaded [`StaticSig`].
fn watched_statics(table: &MobyTable, moby_to_instance: &[usize], driven: &[(MobyId, usize, Option<usize>)]) -> Vec<(MobyId, usize, StaticSig)> {
    let ids: std::collections::HashSet<MobyId> = driven.iter().map(|d| d.0).collect();
    (0..table.first_dynamic)
        .filter(|id| !ids.contains(id))
        .filter_map(|id| Some((id, *moby_to_instance.get(id)?, static_sig(table.mobys.get(id)?))))
        .collect()
}

/// The game draws every moby from its table entry each frame; the renderer draws only [`Play::driven`] from the table
/// (the others keep their loaded records). A watched static whose entry changed (another class moved, turned, hid or
/// deleted it) is driven from now on: its placement and visibility, not its animation (no ported update advances
/// it; the renderer keeps playing it).
fn promote_statics(p: &mut Play) {
    let table = &p.game.mobys;
    let mut moved = Vec::new();
    p.watched.retain(|&(id, ii, sig)| {
        let changed = table.mobys.get(id).is_some_and(|m| static_sig(m) != sig);
        if changed { moved.push((id, ii, None)); }
        !changed
    });
    p.driven.extend(moved);
}

fn rows_bits(rows: &[[f32; 4]; 4]) -> [V4; 3] { [0, 1, 2].map(|i| rows[i].map(f32::to_bits)) }
fn rows3(rows: &[[f32; 4]; 4]) -> [[f32; 3]; 3] { [0, 1, 2].map(|i| [rows[i][0], rows[i][1], rows[i][2]]) }
fn pos3(m: &Moby) -> [f32; 3] { [m.position[0], m.position[1], m.position[2]] }

/// `(gameplay +0x44 section) + 4`: the spawnable moby count = the dynamic slots.
fn spawnable_count(gameplay: &[u8]) -> usize {
    let rd = |o: usize| gameplay.get(o..o + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap()));
    rd(rc_formats::gameplay::MOBY_INSTANCES_POINTER).and_then(|sec| rd(sec as usize + 4)).map_or(0, |n| n.max(0) as usize)
}

/// The loader's moby table: the spawn test on the instances before the ship (`moby_spawn::loader_spawns` with
/// the level's save bytes from `state`), the created ones as the static mobys (`load_level_mobys`, pvar moby
/// links through the instance → moby map), the spawnable count of dynamic slots, the ship created in the first
/// of them (hidden while the level's mission NPC has its mission open: `moby_spawn::ship_hidden_on_arrival`).
/// Returns the table, the statics' maps and the ship's id.
fn moby_table(lv: &crate::level_load::LoadedLevel, classes: &mut ClassTable, ship: Option<usize>, state: Option<&GameState>, level: u32, visit: Option<&rc_game::moby_update::services::SaveBits>, arrival_missions: Option<[u8; 16]>) -> anyhow::Result<(MobyTable, scheduler::LevelStatics, Option<MobyId>)> {
    use rc_formats::moby_spawn as ms;
    let m = &lv.mobys;
    let n_inst = ship.unwrap_or(m.instances.len());
    let insts = &m.instances[..n_inst];
    let mut save = level_spawn_save(state, level);
    // The death reload (G-CLS-030): this visit's bits as the loader reads them again, `0x1ba950` (the death bits),
    // `0x1bbb04` (the per-id flags: the collected placed bolts) and the persistent death bits written this visit.
    if let Some(v) = visit { scheduler::add_visit_bits(&mut save, v, level); }
    // ...and the mission bytes the spawn test reads, 0x15fc88: copied from the save's 0x14c050 only by a full load
    // (`LoadLevelCoreData(1, 0)`; the death's `(0, 1)` skips the copy), so the reload sees the missions as they were
    // at the arrival; `SetMissionDone` writes 0x14c050 alone.
    if let Some(m) = arrival_missions { save.missions = m; }
    let tests = ms::loader_spawns(insts, &mut save.clone());
    // RC_TRACE_RESPAWN=1 (debug): the death reload's spawn test for Kerwan's troopers 574 (the train riders and the rest).
    if visit.is_some() && std::env::var("RC_TRACE_RESPAWN").is_ok_and(|v| v.trim() == "1") {
        println!("respawn trace: death reload: mission bytes 0x15fc88 {:02x?}", save.missions);
        for (k, (i, t)) in insts.iter().zip(&tests).enumerate().filter(|(_, (i, _))| i.o_class == 574) {
            let id = i.spawn_id;
            let killed = usize::try_from(id).ok().and_then(|b| save.killed.get(b >> 3)).is_some_and(|b| b >> (id & 7) & 1 != 0);
            println!(
                "respawn trace:   instance {k} 574 spawn id {id} flags {:#x} mission {} -> {} (never-again {:?}, death this visit {}, killed ever {killed})",
                i.spawn_flags, i.unknown_4, if t.spawn { "created" } else { "NOT created" }, save.id_flags.get(&id), save.visit_death.contains(&id)
            );
        }
    }
    let spawned: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
    let pvars = rc_formats::gameplay::parse_pvars_spawned(&lv.gameplay, &spawned)?;
    let statics = scheduler::load_level_mobys(insts, classes, &pvars, &tests);
    let mut table = MobyTable::new(statics.mobys.clone(), spawnable_count(&lv.gameplay).max(1));
    let mut ship_id = None;
    if let Some(s) = ship {
        let inst = &m.instances[s];
        let info = classes.info(inst.o_class as i16);
        if let Some(id) = table.create(inst.o_class as i16, info.as_ref(), 0) {
            let mo = &mut table.mobys[id];
            mo.position = [inst.position[0], inst.position[1], inst.position[2], 0.0];
            mo.rotation = [inst.rotation[0], inst.rotation[1], inst.rotation[2], 0.0];
            for (k, r) in crate::moby_light::instance_rows(inst).iter().enumerate() { mo.rows[k] = r.map(f32::from_bits); }
            // InitLevelRenderGlobals 0x255958: +0x74 = ShipUpdate (the class port), +0x32 = 0xff, +0x30 = 0x10, mode &= ~2,
            // `hard_cut(ship, 1, 0)`.
            mo.draw_dist = 0xff;
            mo.update_dist = 0x10;
            mo.mode &= !2;
            (mo.anim.seq_a, mo.anim.seq_b, mo.anim.frame_a, mo.anim.frame_b, mo.anim.t) = (1, 1, 0, 0, 0.0);
            let _ = (inst.draw_distance, inst.update_distance);
            if ms::ship_hidden_on_arrival(insts, &tests, &save.missions) {
                // MissionNpcUpdate state 1 → FUN_002a2450: mode |= 3, +0x94 = 0.
                mo.mode |= 3;
                mo.has_collision = false;
            }
            ship_id = Some(id);
        }
    }
    Ok((table, statics, ship_id))
}

/// The dynamic slots drawn as extra instances (see the module doc): record `s` and palette range
/// `s·pal_slots` belong to table slot `first_dynamic + s`.
pub struct DynMobys {
    extra: ExtraMobys,
    pal_slots: u32,
    /// o_class → index into `LevelMobys::classes`.
    class_ix: HashMap<i16, usize>,
    /// The class index each slot shows.
    shown: Vec<Option<usize>>,
    records: Vec<u8>,
    palette: Vec<u8>,
    /// +0x31 per slot (the stand-in MobyProc's result of the last upload).
    visible: Vec<u8>,
    /// Live / drawn this frame (for the trace).
    live: usize,
    drawn: usize,
}

impl DynMobys {
    fn new(lv: &crate::level_load::LoadedLevel, slots: usize, buffers: &mut Assets<ShaderBuffer>) -> Self {
        let m = &lv.mobys;
        // Palette matrices per slot: the most any class other than Ratchet needs.
        let pal_slots = m.classes.iter().zip(&m.anim).filter(|(c, _)| c.o_class != 0)
            .map(|(c, a)| (a.joint_count as u32).max(ExtraMobys::max_skinned_joint(c) as u32 + 1)).max().unwrap_or(1).max(1);
        let records = vec![0; slots.max(1) * moby_render::EXTRA_RECORD_SIZE];
        let palette = crate::moby_anim::identity_palette(slots.max(1) as u32 * pal_slots);
        let extra = ExtraMobys::new(lv, records.clone(), palette.clone(), buffers);
        DynMobys {
            extra,
            pal_slots,
            class_ix: m.classes.iter().enumerate().map(|(i, c)| (c.o_class as i16, i)).collect(),
            shown: vec![None; slots],
            records,
            palette,
            visible: vec![0; slots],
            live: 0,
            drawn: 0,
        }
    }
}

/// The gameplay instances' moby entities (not the attachments).
type GameplayEntities<'w, 's> = Query<'w, 's, (Entity, &'static MeshTag), (With<MeshMaterial3d<MobyMaterial>>, Without<AttachedTo>)>;

#[allow(clippy::too_many_arguments)]
fn setup(
    mut done: Local<bool>,
    generation: Res<crate::level_switch::LevelGeneration>,
    mut commands: Commands,
    level: Res<crate::Level>,
    spawn: Option<Res<MobySpawn>>,
    state: Option<Res<Persistent>>,
    session: Option<Res<Session>>,
    occl: Option<ResMut<MobyOcclusion>>,
    mut anim: Option<ResMut<MobyAnim>>,
    mut particles: Option<ResMut<ParticleSim>>,
    gameplay_entities: GameplayEntities,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    travel: Option<Res<crate::travel_render::Travel>>,
) {
    // Once per level (crate::level_switch: a runtime level change runs the set-up again).
    if generation.is_changed() { *done = false; }
    if *done { return; }
    let Some(mut occl) = occl else { return };
    *done = true;
    let lv = &level.0;
    let m = &lv.mobys;
    let Some(coll) = lv.collision.as_ref() else {
        eprintln!("gameplay: no collision mesh: no game tick");
        return;
    };
    let Some(hero_ii) = m.instances.iter().zip(&m.placed).position(|(i, p)| i.o_class == 0 && p.is_some()) else {
        eprintln!("gameplay: no placed class-0 (Ratchet) instance on this level: no game tick");
        return;
    };
    let placed = m.placed[hero_ii].unwrap();
    let class = &m.anim[placed.class];
    let level_index = lv.particles.level;

    // The loader: class table, static mobys, dynamic slots, the ship.
    let mut classes = class_table(lv, &external_update_fn);
    let ship_ii = spawn.as_ref().and_then(|s| s.ship);
    let (mut table, statics, ship_id) = match moby_table(lv, &mut classes, ship_ii, state.as_ref().map(|s| &s.0), level_index, None, None) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("gameplay: moby table not built ({e:#}): no game tick");
            return;
        }
    };
    let Some(hero_id) = statics.instance_to_moby.get(hero_ii).copied().flatten() else {
        eprintln!("gameplay: Ratchet's instance {hero_ii} was not created by the spawn test: no game tick");
        return;
    };
    if table.hero() != Some(hero_id) { warn!("gameplay: MobyTable::hero() = {:?}, using moby {hero_id} (instance {hero_ii})", table.hero()); }
    // RC_HERO_AT=x,y,z[,yaw] (debug): Ratchet's moby placed there before the hero init (which ground-snaps it); the boot's
    // level only (a level reached by travel starts where the game puts him).
    if let Some(v) = std::env::var("RC_HERO_AT").ok().filter(|_| generation.0 == 0).map(|v| v.split(',').filter_map(|t| t.trim().parse::<f32>().ok()).collect::<Vec<_>>()) {
        if v.len() >= 3 {
            let m = &mut table.mobys[hero_id];
            m.position = [v[0], v[1], v[2], m.position[3]];
            if let Some(&y) = v.get(3) { m.rotation[2] = y; }
            println!("gameplay: RC_HERO_AT: Ratchet placed at {:?}, yaw {}", &v[..3], m.rotation[2]);
        }
    }
    let spawn_moby = table.mobys[hero_id].clone();
    let opts = state.as_ref().map(|s| s.0.options());
    let options = opts.map_or(GameOptions::default(), |o| o.game_options());
    // srand(1234), hero init (ground snap, mode 2, the fidget-timer draw), the camera behind him.
    let mut game = Game::new(coll, table, hero_id, options, lv.death_z);
    if let Some(s) = &session { game.hero.health = s.0.hp; }
    // The hand items (wrench, bomb glove, …): created by the hero update from the first tick on.
    let mut arm_joints: [Vec<u8>; 2] = Default::default();
    let item_data = match item_data(lv) {
        Ok((d, ammo, weapon_defs, seconds, firsts, item_targets)) => {
            game.hero.set_item_joint_targets(item_targets);
            arm_joints = rc_game::hero::weapons::ARM_LISTS.map(|l| seconds.get(l as usize).cloned().unwrap_or_default());
            game.hero.set_joint_targets(&seconds);
            game.hero.set_joint_chains(firsts);
            // The back packs' joint lists and class scales (the Hydro-Pack's jets: rc_game::hero::fx::pack_point).
            let packs: Vec<i16> = [2, 3, 4].iter().map(|&id| d.def(id).o_class as i16).filter(|&o| o > 0).collect();
            match class_joint_lists_where(lv, |o| packs.contains(&o)) {
                Ok(m) => game.hero.set_pack_joint_lists(
                    m.into_iter().filter_map(|(o, l)| lv.mobys.classes.iter().find(|c| c.o_class as i16 == o).map(|c| (o, c.class.header.scale, l))).collect(),
                ),
                Err(e) => eprintln!("gameplay: no back pack joint lists ({e:#}): the Hydro-Pack's jets start at the pack's origin"),
            }
            println!(
                "gameplay: hand items: {} item definitions, {} gadget classes (wrench 71: {}, bomb glove def {:?})",
                d.defs.len(), d.classes.len(), d.class(71).is_some(), d.defs.get(10)
            );
            commands.insert_resource(AmmoTable(ammo, weapon_defs));
            Some(d)
        }
        Err(e) => {
            eprintln!("gameplay: no hand items ({e:#}): the wrench is not created and □ does nothing");
            None
        }
    };
    game.item_data = item_data.clone();
    commands.insert_resource(HeldWeapon::default());
    let mut ratchet = RatchetAnim::new(class);
    ratchet.arm_joints = arm_joints.clone();
    // The glove-holding layers' classes 1 / 2 (rc_game::hero::weapons::hold_update).
    ratchet.hold = hold_classes();
    let mirror_anim = opts.is_some_and(|o| o.mirror_anim);
    ratchet.mirror = mirror_anim;
    // The back items (pack 607 of back item 2, Clank 601) and the level for the idle code.
    let back_classes = back_classes(lv, item_data.as_ref());
    match &back_classes {
        None => eprintln!("gameplay: no pack / Clank (601) classes on this level: no back items (no Clank fidgets)"),
        Some((packs, _, heads, _)) => println!(
            "gameplay: back packs (item, class) {:?} and Clank 601; head items {:?}",
            packs.iter().map(|p| (p.0, p.1)).collect::<Vec<_>>(),
            heads.iter().map(|p| (p.0, p.1)).collect::<Vec<_>>()
        ),
    }
    hero_level_setup(&mut game.hero, back_classes.as_ref(), level_index);
    // HeroInit 0x226b70: unless the level-13 ship was adopted (0x160540), 0x13e090 / 0x13e0a0 = Ratchet's position and
    // rotation after the init: where the landing scene's end puts him back (rc_game::travel).
    let landing_spot = {
        let h = &game.hero;
        let p = [h.pos[0].to_f32(), h.pos[1].to_f32(), h.pos[2].to_f32(), h.pos[3].to_f32()];
        (p, [h.rot[0].to_f32(), h.rot[1].to_f32(), h.rot[2].to_f32(), h.rot[3].to_f32()])
    };

    // The moby loop's services and the load pass (counter 0) on the game's stream.
    let mut svc = Services::new();
    svc.level = level_index;
    svc.death_z = lv.death_z;
    // `entry` 0x259c40 stores game mode 6 before the load pass (`LoadLevelCoreData`): the classes' first update sees
    // it (Umbris' director 436 waits in state 0 for mode 0, so its arrival scene follows the landing).
    if travel.as_ref().is_some_and(|t| t.entry_follows()) { svc.game_mode = 6; }
    // The ship block (rc_game::travel): the loader's ship 0x13e030, its index 0x13e056, the fly-away's path and camera
    // cuboids (level settings +0x3c..+0x44), Ratchet's landing spot; the ship's joint lists (the canopy glass).
    svc.travel = rc_game::travel::ShipGlobals {
        moby: ship_id,
        ship: crate::moby_spawn::ship_index_for(level_index) as i16,
        setup: rc_game::travel::FlyAwaySetup::parse(&lv.gameplay),
        landing_spot: Some(landing_spot),
        ..Default::default()
    };
    // The help system (rc_game::help): the level text and small font it sizes with, the help log's id table.
    help_setup(&mut svc, lv, level_index);
    // The map system's level entry (rc_game::map, FUN_0025a4c0): tables, the level's map file, its fog mask.
    map_setup(&mut svc, level_index, state.as_ref().map(|s| &s.0));
    // The class reaction tables of the level's `lvl.vtbl` (rc_game::moby_update::creature::react).
    svc.creatures.react.tables = level_reactions().clone();
    // The level's water (rc_game::water::world: the ripple managers' tables, the module, the underwater look).
    if let Some(d) = lv.water.data.clone() { svc.water = rc_game::water::world::WaterWorld::new(d); }
    // The level fog globals 0x15f444.. as the moby code reads them (crate::fog_state keeps the copy current).
    svc.water.fog = Some(lv.fog.globals());
    // Level 18's pool meshes (rc_game units::veldin_pool, read with the draw-callback tables).
    svc.units.veldin_pools = lv.water.fx.veldin_pools.clone();
    svc.units.wave_meshes = lv.water.fx.wave_meshes.clone();
    svc.units.blarg_glass = lv.water.fx.blarg_glass.clone();
    svc.units.blarg_bubble = lv.water.fx.blarg_bubble.clone();
    if let Ok(sp) = rc_formats::gameplay::parse_splines(&lv.gameplay) { svc.set_splines(&sp); }
    svc.pvar_shared = rc_formats::gameplay::parse_pvar_shared_data(&lv.gameplay).unwrap_or_default();
    // The volume sections (cuboids, spheres, cylinders, pills, paths, grind paths) for the trigger tests.
    match rc_formats::volumes::parse_volumes(&lv.gameplay) {
        Ok(v) => {
            // The grind paths are also the hero's rails and cables (hero::boots).
            game.grind_paths = std::sync::Arc::new(v.grind_paths.clone());
            svc.set_volumes(v)
        }
        Err(e) => eprintln!("gameplay: no trigger volumes ({e}): every volume test is false"),
    }
    // The level's camera records (rc_game::follow_camera::level: the slots, the class-17 regions) on its shapes.
    // RC_LEVEL_CAMERAS=0: without them (the follow camera alone, as before the level camera system).
    let use_cams = std::env::var("RC_LEVEL_CAMERAS").map_or(true, |v| v != "0");
    match rc_formats::cameras::parse_level_cameras(&lv.gameplay) {
        Ok(_) if !use_cams => println!("gameplay: RC_LEVEL_CAMERAS=0: the level's camera records not loaded"),
        Ok(mut c) => {
            // The loader's moby-link fixups on the camera blocks (class 18's moby) through this load's instance map.
            if let Err(e) = rc_formats::cameras::remap_moby_links(&mut c, &lv.gameplay, &|i| statics.instance_to_moby.get(i).copied().flatten()) {
                eprintln!("gameplay: camera moby links not remapped ({e})");
            }
            let lc = rc_game::follow_camera::level::LevelCameras::new(level_index, &c, Some(svc.volumes.clone()), *camera_ports());
            // The records' classes and class-18 distance / pivot words for the moby loop (the boss 1422's camera tweak).
            svc.camera_classes = c.iter().map(|x| x.record.class).collect();
            svc.camera_focus = lc.slots.iter().map(|s| s.focus.as_ref().map_or([0.0; 2], |f| [f.distance, f.pivot_height])).collect();
            println!("gameplay: {} camera records, {} class-17 regions (ported: {})", c.len(), lc.slots.iter().filter(|s| s.region.is_some()).count(), lc.ports.region);
            game.camera.set_level(lc);
        }
        Err(e) => eprintln!("gameplay: no camera records ({e}): the follow camera only"),
    }
    match class_joint_lists(lv) {
        Ok((j, t)) => (svc.joint_lists, svc.joint_targets) = (j, t),
        Err(e) => eprintln!("gameplay: no class joint lists ({e:#}): the Blarg flyers' exhaust sits at the flyer origin"),
    }
    match class_gaits(lv, |o| level_ports().needs_joint_lists(o)) {
        Ok(g) => svc.gaits = g,
        Err(e) => eprintln!("gameplay: no gait records ({e:#}): the leg walkers stand still"),
    }
    // The other bodies' classes (Clank 0x57, Giant Clank 0x1a3, rc_game::hero::bodies): their joint lists for the hero's
    // joint points and joint modifiers, and for the moby world's joint matrices (Clank's antenna glow and rotor).
    match class_joint_data_where(lv, |o| o == rc_game::hero::bodies::CLANK_CLASS || o == rc_game::hero::bodies::GIANT_CLASS || o == rc_game::hero::bodies::DISGUISE_CLASS) {
        Ok(m) => {
            let mut bodies = Vec::new();
            for (o, l) in m {
                let targets: Vec<u8> = l.iter().map(|(_, s)| rc_formats::moby_anim::list_target(s).unwrap_or(0xff)).collect();
                let chains: Vec<Vec<u8>> = l.into_iter().map(|(a, _)| a).collect();
                svc.joint_lists.entry(o).or_insert_with(|| chains.clone());
                svc.joint_targets.entry(o).or_insert_with(|| targets.clone());
                bodies.push(rc_game::hero::bodies::BodyJoints { o_class: o, chains, targets });
            }
            game.hero.set_body_joints(bodies);
        }
        Err(e) => eprintln!("gameplay: no body joint lists ({e:#}): Clank / Giant Clank's joint points sit at the body's origin"),
    }
    if let Some(id) = ship_id {
        let oc = game.mobys.mobys[id].o_class;
        match crate::travel_render::ship_joint_lists(oc) {
            Some(l) => { svc.joint_lists.insert(oc, l); }
            None => eprintln!("gameplay: ship class {oc}: no joint lists (the canopy glass at the ship's origin)"),
        }
        // InitLevelRenderGlobals: the ship's +0x38 = the hero's light words (after the hero init).
        let (light, ambient) = (game.mobys.mobys[hero_id].light, game.mobys.mobys[hero_id].ambient);
        let mo = &mut game.mobys.mobys[id];
        (mo.light, mo.ambient) = (light, ambient);
    }
    let n_static = game.mobys.first_dynamic;
    svc.groups = statics.groups(&lv.gameplay);
    // The "use" system: talk tables, text, talk slots, save values (crate::interact_render; before the load pass).
    crate::interact_render::install(&mut svc, lv, &statics.instance_to_moby, state.as_ref().map(|s| &s.0));
    if let Some(gs) = &state {
        svc.counters.bolts = gs.0.global.bolts;
        // 0x15ee20 (the challenge-gated pads 1135 and gold-weapon offers read it) and 0x13e520.
        svc.counters.times_completed = gs.0.global.completes;
        svc.counters.gold_weapons = gs.0.global.gold_weapons.to_vec();
        if let Some(l) = gs.0.levels.get(level_index as usize) {
            svc.counters.level_bolts[level_index as usize % 20] = l.bolts;
            // 0x14d592 + L·0x100 + k·4: the bolts each spawner slot already dropped and were collected (chunk 3006): a
            // dropper whose slot is short of its total drops the rest (`SetDeathBits`), not the whole count again.
            for (k, d) in l.bolt_drops.iter().enumerate() {
                if d.collected != 0 { svc.counters.spawner_bolts.insert((level_index, k as u8), d.collected); }
            }
        }
    }
    // The spawner slots the loader's spawn test hands out (`FUN_0029ab50` writes them into 0x14d590 + L·0x100: chunk 3006's
    // `first`), for `Services::sync_save`.
    {
        let n = ship_ii.unwrap_or(lv.mobys.instances.len());
        let mut s = level_spawn_save(state.as_ref().map(|s| &s.0), level_index);
        let _ = rc_formats::moby_spawn::loader_spawns(&lv.mobys.instances[..n], &mut s);
        svc.counters.spawner_first = s.spawner.to_vec();
    }
    // Moby collision: the class blobs and the loader's grid registrations (MobyBuildMatrix per instance).
    match moby_collision_blobs(level_index) {
        Ok(mut b) => {
            // The loader's ship from the global `spaceships` file (crate::moby_spawn) brings its class's blob too.
            if let Some(oc) = ship_id.map(|id| game.mobys.mobys[id].o_class as i32).filter(|oc| !b.iter().any(|(c, _)| c == oc)) {
                match spaceship_collision(oc) {
                    Ok(Some(c)) => b.push((oc, c)),
                    Ok(None) => {}
                    Err(e) => eprintln!("gameplay: the ship class {oc}'s collision not loaded: {e:#}"),
                }
            }
            svc.set_moby_collision(b)
        }
        Err(e) => eprintln!("gameplay: no moby collision ({e:#}): the collision queries see no mobys"),
    }
    svc.build_grid(&mut game.mobys);
    println!(
        "gameplay: moby collision: {} class blobs, {} grid entries",
        svc.coll_classes.len(), svc.grid.cells.iter().map(Vec::len).sum::<usize>()
    );
    // Class-27 emitters: owner k belongs to gameplay instance `o.instance`, i.e. to moby 0x1acc00[o.instance].
    let emitters: HashMap<MobyId, usize> = particles.as_ref().map(|p| {
        p.sys.owners.iter().enumerate().filter_map(|(k, o)| Some((statics.instance_to_moby.get(o.instance).copied().flatten()?, k))).collect()
    }).unwrap_or_default();
    // The level's mission state: 0x15fc88 / 0x14c050 from the save, the per-mission deaths 0x14ee90 cleared
    // (a fresh load; the ammo crates 501 read them in the load pass).
    let level_save = state.as_ref().and_then(|s| s.0.levels.get(level_index as usize)).map(|l| l.missions).unwrap_or_default();
    let missions = LevelMissions::fresh_load(level_index, level_save);
    let mut sched = Scheduler::new();
    let rng0 = game.rng;
    // Ratchet's owned items and ammo from the save before the load pass (the tick copies them in again every frame).
    if let Some(gs) = &state {
        game.hero.owned.0 = gs.0.global.owned;
        game.hero.weapons.ammo = gs.0.global.ammo;
    }
    let n_load = {
        let hero = game.hero.clone();
        let inv = load_inventory(&svc, item_data.as_ref(), state.as_ref().map(|s| s.0.global.vendor), &hero);
        // view None: the view before the level's first render (all zero: FastBSphereCheck culls, type06).
        let mut ext = Externals { level: level_index, emitters: &emitters, view: None };
        let mut w = World::new(&mut game.mobys, &hero, &mut game.rng, &classes, &mut svc, game.counter);
        w.inventory = &inv;
        w.camera = game.camera.out.pos;
        w.coll = Some(coll);
        w.particles = particles.as_deref_mut().map(|p| &mut p.sys);
        w.external = Some(&mut ext);
        w.missions = &missions;
        sched.load_pass(&mut w)
    };
    // LoadLevelCoreData: 0x15f5cc++ after the load pass.
    game.finish_load();
    if let Some(p) = particles.as_mut() { p.external = true; }

    // Ratchet as an extra instance of his class; the gameplay-instance entities are hidden.
    let slots = (class.joint_count as u32).max(ExtraMobys::max_skinned_joint(&m.classes[placed.class]) as u32 + 1).max(1);
    let mut extra = ExtraMobys::new(lv, vec![0; moby_render::EXTRA_RECORD_SIZE], crate::moby_anim::identity_palette(slots), &mut buffers);
    let hm = &game.mobys.mobys[hero_id];
    let t = Transform::from_matrix(moby_render::extra_model(rows3(&hm.rows), placed.scale, [hm.position[0], hm.position[1], hm.position[2]]));
    let (ratchet_high, ratchet_metal) = extra.spawn_split(&mut commands, lv, &m.classes[placed.class], 0, t, "Ratchet (play)", &mut meshes, &mut images, &mut materials);
    let ratchet_low = extra.spawn_low(&mut commands, lv, &m.classes[placed.class], 0, t, "Ratchet (play)", &mut meshes, &mut images, &mut materials);
    let entities: Vec<Entity> = ratchet_high.iter().chain(&ratchet_metal).chain(&ratchet_low).copied().collect();
    for &e in &entities { commands.entity(e).insert(RatchetMesh); }
    // The LOD sets under one parent each (identity transforms): the pick toggles the parents, so the scene / vendor /
    // travel hides, which set the entities themselves, keep working.
    let ratchet_lods = [
        commands.spawn((RatchetLod, Transform::IDENTITY, Visibility::Inherited, Name::new("Ratchet LOD high"))).add_children(&ratchet_high).id(),
        commands.spawn((RatchetLod, Transform::IDENTITY, Visibility::Hidden, Name::new("Ratchet LOD low"))).add_children(&ratchet_low).id(),
    ];
    let mut hidden = 0;
    for (e, tag) in &gameplay_entities {
        if tag.0 as usize == hero_ii && !entities.contains(&e) {
            commands.entity(e).insert((SpawnHidden, Visibility::Hidden));
            hidden += 1;
        }
    }
    // Instances the loader did not create (spawn test): never drawn.
    let pending: std::collections::HashSet<usize> = statics.pending.iter().copied().collect();
    let mut unspawned = 0;
    for (e, tag) in &gameplay_entities {
        if pending.contains(&(tag.0 as usize)) {
            commands.entity(e).insert((SpawnHidden, Visibility::Hidden));
            unspawned += 1;
        }
    }
    if let Some(a) = anim.as_mut() {
        for &ii in &statics.pending { if let Some(k) = occl.anim_index(ii) { a.hidden[k] = true; a.pending[k] = None; } }
    }
    let hero_k = occl.anim_index(hero_ii);
    if let (Some(a), Some(k)) = (anim.as_mut(), hero_k) { a.pending[k] = None; }

    // The static mobys the table drives (a class with a Rust port), and the dynamic slots.
    let driven: Vec<(MobyId, usize, Option<usize>)> = (0..n_static)
        .filter(|&id| {
            let m = &game.mobys.mobys[id];
            // The body mobys are driven by the hero code while they are the hero (rc_game::hero::bodies).
            m.update_fn.and_then(scheduler::ported).is_some() || m.o_class == rc_game::hero::bodies::CLANK_CLASS || m.o_class == rc_game::hero::bodies::GIANT_CLASS
        })
        .map(|id| { let ii = statics.moby_to_instance[id]; (id, ii, occl.anim_index(ii)) })
        .collect();
    // The ship (`ShipUpdate`, rc_game::travel::ship) is driven from the table like the ported statics: its update, the
    // mode-6 scenes and the fly-away move, hide and animate it (its instance is the one crate::moby_spawn appended).
    let watched = watched_statics(&game.mobys, &statics.moby_to_instance, &driven);
    let mut driven = driven;
    if let (Some(id), Some(ii)) = (ship_id, ship_ii) {
        driven.push((id, ii, occl.anim_index(ii)));
        // Hidden here; the occlusion pass shows the one group it picks (`MobyOcclusion::reshow`).
        for (e, tag) in &gameplay_entities {
            if tag.0 as usize == ii { commands.entity(e).remove::<SpawnHidden>().insert(Visibility::Hidden); }
        }
        occl.reshow(ii);
        if let (Some(a), Some(k)) = (anim.as_mut(), occl.anim_index(ii)) { (a.hidden[k], a.pending[k]) = (false, None); }
    }
    let dynamic = DynMobys::new(lv, game.mobys.mobys.len() - n_static, &mut buffers);

    let inst = &m.instances[hero_ii];
    let h = &game.hero;
    let mut per_class: std::collections::BTreeMap<i16, usize> = std::collections::BTreeMap::new();
    for &(id, _, _) in &driven { *per_class.entry(game.mobys.mobys[id].o_class).or_default() += 1; }
    println!(
        "gameplay: Ratchet = instance {hero_ii} = moby {hero_id} (class slot {}), ground-snapped to {:.3?} yaw {:.4}, fidget timer {}; death z {}; \
         moby table {} static ({} instances not created by the spawn test, {unspawned} entities hidden) + {} dynamic slots (spawnable count; ship {:?}{}); \
         {hidden} gameplay entities hidden, drawn as an extra instance ({} entities, {slots} palette slots)",
        placed.class, h.position(), h.yaw().to_f32(), h.fidget_timer, lv.death_z, n_static, statics.pending.len(), game.mobys.mobys.len() - n_static, ship_id,
        if ship_id.is_some_and(|id| game.mobys.mobys[id].mode & 1 != 0) { ", hidden by the mission NPC" } else { "" }, entities.len()
    );
    println!(
        "gameplay: moby load pass: {n_load} mobys run, rng {:#010x} -> {:#010x}, tick counter now {}; scheduler-driven statics per class {per_class:?}; \
         {} groups; {} class-27 emitters, ripple module {}; dynamic slots drawn as extras ({} palette matrices each)",
        rng0.state, game.rng.state, game.counter, svc.groups.lists.iter().flatten().count(), emitters.len(),
        if svc.water.sim.is_some() { "initialised on the game stream" } else { "none" }, dynamic.pal_slots
    );
    commands.insert_resource(PlayView { view: game.camera.out, tan_half_fov: play_camera::GAME_TAN_HALF_FOV });
    let mut play = Play {
        game,
        ratchet,
        body_anim: None,
        hero_id,
        hero_k,
        class: placed.class,
        scale: placed.scale,
        slots,
        light_word: inst.light_word(),
        ambient: inst.ambient_rgb(),
        spawn_moby,
        extra,
        entities,
        ratchet_lods,
        ratchet_has_low: !ratchet_low.is_empty(),
        ratchet_lod_low: false,
        uploaded: None,
        sched,
        classes: std::sync::Arc::new(classes),
        svc,
        driven,
        watched,
        moby_to_instance: statics.moby_to_instance.clone(),
        missions,
        lit: HashMap::new(),
        ship: ship_id.zip(ship_ii),
        emitters,
        level: level_index,
        dynamic,
        sounds: HashMap::new(),
        ratchet_hidden: false,
        noclip: None,
        ratchet_late: None,
        reload_shown: Vec::new(),
        arm_joints,
        help_vag: None,
        help_debug: std::env::var("RC_HUD_HELP").ok().and_then(|v| v.trim().parse().ok()),
        debug_hits: std::env::var("RC_DEBUG_HIT").ok().map(|v| {
            v.split(',').filter_map(|h| {
                let (a, b) = h.trim().split_once('@')?;
                let id = if a.trim() == "hero" { hero_id } else { a.parse().ok()? };
                Some((id, b.parse().ok()?))
            }).collect()
        }).unwrap_or_default(),
        mirror_anim,
        item_data,
        back_classes,
        trace: std::env::var("RC_PLAY_TRACE").is_ok_and(|v| v.trim() == "1"),
        respawn: false,
        death_pending: false,
        frozen_hint: false,
    };
    publish_anim(&mut play, anim.as_deref_mut());
    drive_statics(&mut play, lv, &mut occl, anim.as_deref_mut());
    commands.insert_resource(play);
}

/// The back items' classes: `(back item id, o_class, class)` per pack, and Clank; the head items' (5..7, their
/// put-away animation: rc_game::hero::worn), and Clank's class scale.
type BackPacks = (Vec<(i32, i16, MobyAnimClass)>, MobyAnimClass, rc_game::hero::worn::HeadClasses, Option<f32>);

/// The back items' anim classes on this level: the pack moby of each back item 2 / 3 / 4 (the item definitions'
/// class +0x10: Heli-Pack 607, Thruster-Pack 608, Hydro-Pack 609; those values without the definitions) that the
/// level has, and Clank (item 1's 601). None without the pack of item 2 or Clank.
fn back_classes(lv: &crate::level_load::LoadedLevel, items: Option<&ItemData>) -> Option<BackPacks> {
    let m = &lv.mobys;
    let anim = |o: i32| m.classes.iter().position(|c| c.o_class == o).map(|ci| m.anim[ci].clone());
    let class_of = |id: i32, fallback: i32| items.map(|d| d.def(id).o_class).filter(|&o| o > 0).unwrap_or(fallback);
    let packs: Vec<(i32, i16, MobyAnimClass)> =
        [(2, 607), (3, 608), (4, 609)].into_iter().filter_map(|(id, o)| { let o = class_of(id, o); Some((id, o as i16, anim(o)?)) }).collect();
    if !packs.iter().any(|p| p.0 == 2) { return None; }
    let heads = [(5, 433), (6, 1289), (7, 1290)].into_iter().filter_map(|(id, o)| { let o = class_of(id, o); Some((id, o as i16, anim(o)?)) }).collect();
    Some((packs, anim(class_of(1, 601))?, heads, clank_scale(lv, items)))
}

/// Clank's class scale (item 1's class, 601 without the definitions): his moby's +0x2c (`rc_game::hero::pose`'s record 18).
fn clank_scale(lv: &crate::level_load::LoadedLevel, items: Option<&ItemData>) -> Option<f32> {
    let o = items.map(|d| d.def(1).o_class).filter(|&o| o > 0).unwrap_or(601);
    lv.mobys.classes.iter().find(|c| c.o_class == o).map(|c| c.class.header.scale)
}

/// What the hero code needs from the level after `HeroInit` (load and respawn): the back items' classes and
/// the level index 0x15ed84.
fn hero_level_setup(hero: &mut Hero, back: Option<&BackPacks>, level: u32) {
    if let Some((packs, clank, heads, clank_scale)) = back {
        hero.set_back_packs(packs.clone(), clank.clone());
        hero.set_head_classes(heads.clone());
        // Clank's +0x2c for record 18's sway (rc_game::hero::pose).
        if let Some(s) = clank_scale { hero.set_clank_scale(*s); }
    }
    hero.idle.level = level as i32;
}

/// MobyProc's deferral of the scheduler-driven static mobys (crate::shadow_render's casters of the last frame, or
/// mode 0x800): their caster draws after the shadow pass ([`moby_render::CasterTwin`]).
fn caster_order(play: Option<Res<Play>>, shadows: Option<Res<crate::shadow_render::ShadowVolumes>>, occl: Option<ResMut<MobyOcclusion>>) {
    let (Some(p), Some(mut occl)) = (play, occl) else { return };
    let table = &p.game.mobys;
    for &(id, ii, _) in &p.driven {
        let Some(m) = table.mobys.get(id) else { continue };
        occl.set_late(ii, shadows.as_ref().is_some_and(|s| s.deferred.contains(&id)) || m.mode & 0x800 != 0);
    }
}

/// The table → the renderer for the scheduler-driven static mobys (module doc): placement, light block (for
/// changed rows / ambient / light word), hide, anim state and snapshot.
fn drive_statics(p: &mut Play, lv: &crate::level_load::LoadedLevel, occl: &mut MobyOcclusion, mut anim: Option<&mut MobyAnim>) {
    promote_statics(p);
    let table = &p.game.mobys;
    let lighting = lv.mobys.lighting.as_ref();
    for &(id, ii, k) in &p.driven {
        let m = &table.mobys[id];
        // +0x23 alpha 0 (the Blarg flyers' altitude fade above z 175) draws nothing; the partial fade (0 < a <
        // 0x80) is not rendered.
        let hidden = m.state >= 0x80 || m.mode & 0x81 != 0 || m.alpha == 0;
        let rows = rows_bits(&m.rows);
        let key = (std::array::from_fn(|i| rows[i / 3][i % 3]), [m.ambient[0], m.ambient[1], m.ambient[2]], m.light);
        let lights = match lighting {
            Some(l) if !hidden && p.lit.get(&id) != Some(&key) => {
                p.lit.insert(id, key);
                Some(light::moby_lights(&rows, &l.bank, m.light, key.1, 0x80))
            }
            _ => None,
        };
        // A run-time class swap (rc_game::moby_update::class_swap): the instance draws its live class.
        if let Some(&ci) = p.dynamic.class_ix.get(&m.o_class) {
            if occl.set_class(ii, ci) {
                if let (Some(a), Some(k)) = (anim.as_deref_mut(), k) { a.set_class(k, ci); }
            }
        }
        occl.drive(ii, pos3(m), rows3(&m.rows), m.scale, lights.as_ref(), hidden);
        // Moby +0x36, the occlusion word MobyProc tests: the game keeps 0 for "the loader's word" (the renderer
        // resolved it per instance) and writes 0x7f80 for a creature revived or moved elsewhere, a thrown crate, …
        occl.set_occlusion_word(ii, m.occlusion);
        occl.look(ii, moby_render::MobyLook { alpha: m.alpha, mode: m.mode, glow: m.glow, shine_distance: m.b73, draw_dist: Some(m.draw_dist), late: None });
        if let (Some(a), Some(k)) = (anim.as_deref_mut(), k) {
            a.drive(k, m.anim, p.svc.snapshots.get(id).and_then(|s| s.as_ref()), &m.joint_mods);
        }
    }
}

/// The main loop's catch-up rule for this rendered frame: RCNT1 counts 0x2580 per 60 Hz field.
fn set_budget(det: Res<Deterministic>, time: Res<Time<Real>>, mut budget: ResMut<TickBudget>) {
    let rcnt1 = (time.delta_secs_f64() * 60.0 * FIELD_RCNT1 as f64) as u32;
    budget.0 = if det.0 { 1 } else { ticks_for_frame(rcnt1) };
}

/// Tab: fly ↔ game camera (the fly camera resumes from the current view). R: respawn.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut source: ResMut<CameraSource>,
    play: Option<ResMut<Play>>,
    mut cams: Query<(&Transform, &mut FlyCam)>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        *source = match *source {
            CameraSource::Play => {
                for (t, mut c) in &mut cams {
                    let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
                    (c.yaw, c.pitch) = (yaw, pitch);
                }
                println!("camera: fly (Tab: back to the game camera; the pad is neutral meanwhile)");
                CameraSource::Fly
            }
            CameraSource::Fly => {
                println!("camera: game");
                CameraSource::Play
            }
        };
    }
    let mut play = play;
    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(p) = play.as_mut() { p.respawn = true; }
    }
    // Dev builds: N toggles the hero noclip.
    if cfg!(feature = "dev") && keys.just_pressed(KeyCode::KeyN) {
        if let Some(p) = play.as_mut() {
            p.noclip = match p.noclip {
                Some(_) => None,
                None => Some(p.game.hero.pos.map(|v| v.to_f32())[..3].try_into().unwrap()),
            };
            println!("noclip: {} (stick: move, R1 / ✕: up, L1: down, R2: fast)", if p.noclip.is_some() { "on" } else { "off" });
        }
    }
}

/// Dev builds: the noclip's move from the pad (the stick along the camera's view, R1 or ✕ up, L1 down, R2 fast), the
/// hero placed there after the tick with no velocity, so neither collision nor gravity holds him.
fn noclip_step(p: &mut Play, pad: &PadInput) {
    use rc_game::pad::button;
    let Some(mut q) = p.noclip else { return };
    let axis = |b: u8| ((b as f32 - 127.5) / 127.5).clamp(-1.0, 1.0);
    let (sx, sy) = (axis(pad.lx), -axis(pad.ly));
    let held = |b: u32| pad.buttons as u32 & b != 0;
    let cam = p.game.camera.out.pos_f32();
    let (dx, dy) = (q[0] - cam[0], q[1] - cam[1]);
    let l = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (fx, fy) = (dx / l, dy / l);
    let speed = if held(button::R2) { 0.6 } else { 0.2 };
    let up = (held(button::R1) || held(button::CROSS)) as i32 as f32 - held(button::L1) as i32 as f32;
    q[0] += (fx * sy + fy * sx) * speed;
    q[1] += (fy * sy - fx * sx) * speed;
    q[2] += up * speed;
    p.noclip = Some(q);
    let h = &mut p.game.hero;
    h.pos = rc_game::hero::physics::from_f32x3(q);
    h.vel = rc_game::hero::physics::V0;
    let hm = h.hero_moby(p.hero_id);
    let h = &mut p.game.hero;
    h.write_back(&mut p.game.mobys.mobys[hm]);
}

/// Ratchet's animation into his `MobyAnim` instance (never advanced there: `skip_advance`).
fn publish_anim(p: &mut Play, anim: Option<&mut MobyAnim>) {
    let (Some(a), Some(k)) = (anim, p.hero_k) else { return };
    let mut s = p.ratchet.state;
    s.skip_advance = true;
    a.instances[k].state = s;
    a.snapshots[k] = p.ratchet.snapshot.clone();
}

/// The item tables a load pass reads (the tick's, `w.inventory`): the price records, the vendor list 0x15edd0, the item
/// slot types and Ratchet's owned items and ammo (0x13d4c0 / 0x13d428). The classes' inits test them there (Helga
/// leaves once the Swingshot is owned); without them every item read as not owned.
fn load_inventory(svc: &rc_game::moby_update::services::Services, item_data: Option<&ItemData>, vendor: Option<[u8; 12]>, hero: &Hero) -> rc_game::moby_update::classes::pickup::ItemTables {
    let slots: Vec<i32> = item_data.map_or_else(Vec::new, |d| d.defs.iter().map(|d| d.slot).collect());
    rc_game::moby_update::classes::pickup::ItemTables::new(&svc.interact.tables.shop.records, vendor.unwrap_or([0xff; 12])).with_slots(&slots).with_hero(hero)
}

/// The death reload's moby side (`LoadLevelCoreData(0, 1)` after the death flag 0x141401, G-CLS-030): the loader's
/// instance loop and spawn test run again, now with this visit's bits (`scheduler::add_visit_bits`: the death bits
/// 0x1ba950, the per-id flags 0x1bbb04, the death bits set this visit), so every placed moby restarts from its record
/// (crates, enemies, switches back) and the ones killed or collected this visit stay gone; the dynamic slots are freed
/// (the hero's items, bodies, shots, effects); the ship is created again; the load pass runs. The table is compacted
/// as the game's is, so the engine's maps follow it: Ratchet's moby, the ship, the driven statics, the class-27
/// emitters, and the instances whose spawn changed are hidden / shown. [`respawn`] then makes the hero side.
#[allow(clippy::too_many_arguments)]
fn death_reload(p: &mut Play, lv: &crate::level_load::LoadedLevel, state: Option<&GameState>, occl: Option<&mut MobyOcclusion>, anim: Option<&mut MobyAnim>, particles: Option<&mut ParticleSim>) {
    // The class table as the load builds it (the loader ORs each instance's mode bits into it again).
    let mut classes = class_table(lv, &external_update_fn);
    let ship_ii = p.ship.map(|(_, ii)| ii);
    let (table, statics, ship_id) = match moby_table(lv, &mut classes, ship_ii, state, p.level, Some(&p.svc.save), Some(p.missions.slot)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("gameplay: death reload: moby table not rebuilt ({e:#}): the old table stays");
            return;
        }
    };
    let Some(hero_ii) = p.moby_to_instance.get(p.hero_id).copied() else { return };
    let Some(hero_id) = statics.instance_to_moby.get(hero_ii).copied().flatten() else {
        eprintln!("gameplay: death reload: Ratchet's instance not created: the old table stays");
        return;
    };
    // The instances whose spawn changed since the last load (the renderer draws per instance).
    let was: std::collections::HashSet<usize> = p.moby_to_instance.iter().copied().collect();
    let now: std::collections::HashSet<usize> = statics.moby_to_instance.iter().copied().collect();
    let mut occl = occl;
    let mut anim = anim;
    for &ii in was.difference(&now) {
        if let Some(o) = occl.as_deref_mut() { o.drive(ii, [0.0; 3], [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], 1.0, None, true); }
        if let (Some(a), Some(k)) = (anim.as_deref_mut(), occl.as_deref().and_then(|o| o.anim_index(ii))) { a.hidden[k] = true; }
    }
    let shown: Vec<usize> = now.difference(&was).copied().collect();
    p.game.mobys = table;
    p.classes = std::sync::Arc::new(classes);
    p.hero_id = hero_id;
    p.spawn_moby = p.game.mobys.mobys[hero_id].clone();
    p.ship = ship_id.zip(ship_ii);
    // 0x13e030: the loader's new ship (`ShipUpdate` runs on it alone: its glass, its hatch prompt). The compacted table
    // moves its id once a placed moby is gone; the old id left the ship without its update after a death.
    p.svc.travel.moby = ship_id;
    p.moby_to_instance = statics.moby_to_instance.clone();
    let n_static = p.game.mobys.first_dynamic;
    let mut driven: Vec<(MobyId, usize, Option<usize>)> = (0..n_static)
        .filter(|&id| {
            let m = &p.game.mobys.mobys[id];
            m.update_fn.and_then(scheduler::ported).is_some() || m.o_class == rc_game::hero::bodies::CLANK_CLASS || m.o_class == rc_game::hero::bodies::GIANT_CLASS
        })
        .map(|id| { let ii = statics.moby_to_instance[id]; (id, ii, occl.as_deref().and_then(|o| o.anim_index(ii))) })
        .collect();
    p.watched = watched_statics(&p.game.mobys, &statics.moby_to_instance, &driven);
    if let (Some(id), Some(ii)) = (ship_id, ship_ii) { driven.push((id, ii, occl.as_deref().and_then(|o| o.anim_index(ii)))); }
    p.driven = driven;
    p.lit.clear();
    // The emitters (owner k of gameplay instance i) by the new instance → moby map.
    if let Some(ps) = particles.as_deref() {
        p.emitters = ps.sys.owners.iter().enumerate().filter_map(|(k, o)| Some((statics.instance_to_moby.get(o.instance).copied().flatten()?, k))).collect();
    }
    // `InitLevelRenderGlobals(0)` empties the particle pool (high water −1, free hint 0): the old visit's particles are
    // gone (kept, the nanotech orbs of the freed clusters stayed in the air, still and see-through). The moby anchors
    // go with them (the ids are the old table's).
    if let Some(ps) = particles {
        ps.sys.pool.level_init();
        ps.sys.anchors.clear();
        ps.sys.anchor_scales.clear();
        ps.sys.joint_anchors.clear();
    }
    // `InitLevelRenderGlobals(0)` reads the gameplay file from the disc again and decompresses it: the paths, the
    // shared pvar data and the volumes come back as recorded (classes edit them in place: Kerwan's infobot carries its
    // ride cuboid with the train, the train moves its arrival cuboid; kept, every death shifted the train's riders).
    if let Ok(sp) = rc_formats::gameplay::parse_splines(&lv.gameplay) { p.svc.set_splines(&sp); }
    p.svc.pvar_shared = rc_formats::gameplay::parse_pvar_shared_data(&lv.gameplay).unwrap_or_default();
    if let Ok(v) = rc_formats::volumes::parse_volumes(&lv.gameplay) {
        p.game.grind_paths = std::sync::Arc::new(v.grind_paths.clone());
        p.svc.set_volumes(v);
    }
    // Its camera records too, their moby links through this reload's instance map (the compacted table: the ids shift
    // once a placed moby is gone), then `0x20ef58` (`set_level`: the camera's level state cleared), as the load.
    if std::env::var("RC_LEVEL_CAMERAS").map_or(true, |v| v != "0") {
        if let Ok(mut c) = rc_formats::cameras::parse_level_cameras(&lv.gameplay) {
            if let Err(e) = rc_formats::cameras::remap_moby_links(&mut c, &lv.gameplay, &|i| statics.instance_to_moby.get(i).copied().flatten()) {
                eprintln!("gameplay: death reload: camera moby links not remapped ({e})");
            }
            let lc = rc_game::follow_camera::level::LevelCameras::new(p.level, &c, Some(p.svc.volumes.clone()), *camera_ports());
            p.svc.camera_classes = c.iter().map(|x| x.record.class).collect();
            p.svc.camera_focus = lc.slots.iter().map(|s| s.focus.as_ref().map_or([0.0; 2], |f| [f.distance, f.pivot_height])).collect();
            p.game.camera.set_level(lc);
        }
    }
    // The nanotech master 0x15f63c (the cluster that turns the orb directions and counts the clusters on their way):
    // the clusters are dynamic mobys the reload frees, and the compacted table moves their slots once a placed moby is
    // gone; a stale master left every new cluster still and its heal check reading another moby (nothing could be
    // picked up). The first cluster of the load pass takes it again, as the game's same-slot reload gives it.
    p.svc.pickups.master = None;
    // `InitLevelRenderGlobals` reads the level fog 0x15f444..0x15f454 from the gameplay file again (`UpdateFog`) and
    // zeroes the camera block 0x167100..0x1674a0 (the underwater flag 0x167494): a class's fog store (Rilgar's sewer
    // fog 841, the Quartu shock fog) does not outlive a death, and the load pass's classes save the level's fog.
    let tick = p.game.counter;
    p.svc.water.store_fog(tick, lv.fog.globals());
    p.svc.water.underwater_store = Some((tick, rc_game::water::world::UnderwaterStore::Off));
    // `InitLevelRenderGlobals` also zeroes 0x13f350..0x141660 (`FastMemSet(0x13f350, 0, 0x2310)`): of the words the
    // port keeps in the services, the Visibomb's missile 0x141330, the drones' block 0x141344..0x141388 and the
    // buried bolt cache's nearest 0x141390..98 (the others there are the hero's, rebuilt by the respawn).
    p.svc.visibomb.missile = None;
    p.svc.drones = Default::default();
    let b = &mut p.svc.buried;
    (b.nearest, b.distance, b.alert, b.seen) = (None, 0.0, false, None);
    // The services that hold the old table's ids or per-visit moby state.
    p.svc.groups = statics.groups(&lv.gameplay);
    p.svc.hits = Default::default();
    // The "use" system's talk slots and save values by the new instance → moby map (before the load pass, as the load).
    crate::interact_render::install(&mut p.svc, lv, &statics.instance_to_moby, state);
    p.svc.build_grid(&mut p.game.mobys);
    p.reload_shown = shown;
    println!(
        "gameplay: death reload: {} static mobys ({} gone this visit, {} back)",
        n_static, was.difference(&now).count(), p.reload_shown.len()
    );
}

/// The death reload's end (`LoadLevelCoreData` after `0x29adc8`): the talk cooldown 0x179590 = 0, the load pass on the
/// respawned hero (counter 0, `InitLevelRenderGlobals` reset it), then `0x15f5cc++`.
fn reload_load_pass(p: &mut Play, coll: &rc_formats::collision::Collision, particles: Option<&mut ParticleSim>, vendor: Option<[u8; 12]>) {
    p.svc.interact.cooldown = 0;
    let mut sched = Scheduler::new();
    let n_load = {
        let mut hero = p.game.hero.clone();
        // `0x29adc8` switches into the checkpoint's body before this pass; the port's switch is made by the next tick
        // (`Bodies::restore`), so the pass sees the hero as the game has him: in the body (0x1413f4, 0x1413d0). Else the
        // classes take the body for Ratchet's: Blarg's Clank station hides Clank and drops his collision.
        if let Some((body, _, b)) = hero.bodies.restore {
            hero.mode = body;
            hero.bodies.moby = Some(b.id);
            hero.bodies.class = b.o_class;
        }
        let inv = load_inventory(&p.svc, p.item_data.as_ref(), vendor, &hero);
        let mut ext = Externals { level: p.level, emitters: &p.emitters, view: None };
        let mut w = World::new(&mut p.game.mobys, &hero, &mut p.game.rng, &*p.classes, &mut p.svc, p.game.counter);
        w.inventory = &inv;
        w.camera = p.game.camera.out.pos;
        w.coll = Some(coll);
        w.particles = particles.map(|ps| &mut ps.sys);
        w.external = Some(&mut ext);
        w.missions = &p.missions;
        sched.load_pass(&mut w)
    };
    p.sched = sched;
    p.game.finish_load();
    println!("gameplay: death reload: {n_load} run by the load pass");
}

/// Ratchet's own mesh entities (his moby drawn as an extra instance): their visibility is [`ratchet_visibility`]'s.
#[derive(Component)]
pub(crate) struct RatchetMesh;

/// Ratchet's meshes follow his moby's hidden bit (mode 1: first person, a body in, `HeroSyncMoby` 0x229f20 → 0x2486c0)
/// every frame, written directly, so no other writer's leftover sticks (a scene's, travel's or the vendor's show at
/// their end: they leave his meshes to this). Meshes those hides hold are skipped while they hold them.
#[allow(clippy::type_complexity)]
fn ratchet_visibility(
    play: Option<ResMut<Play>>,
    mut q: Query<(&mut Visibility, Has<crate::scene_render::SceneHidden>, Has<crate::travel_render::TravelHidden>, Has<crate::interact_render::VendorHidden>), With<RatchetMesh>>,
) {
    let Some(mut p) = play else { return };
    let Some(m) = p.game.mobys.mobys.get(p.hero_id) else { return };
    let hidden = m.mode & rc_game::moby_runtime::mode::HIDDEN != 0;
    p.ratchet_hidden = hidden;
    let want = if hidden { Visibility::Hidden } else { Visibility::Inherited };
    for (mut v, scene, travel, vendor) in &mut q {
        if scene || travel || vendor { continue; }
        if *v != want { *v = want; }
    }
}

/// Debug: F9 prints the hero, the engine's own Ratchet draw and every moby within 12 units of the hero (id, class,
/// state, mode bits, collision, position) with the visibility of its generic moby meshes.
/// F10 (debug): the respawn state, in the format of the PCSX2 probe `work/scratch/respawn_probe.py` (the original's
/// memory over PINE), so the two can be compared line by line: the mission bytes (the arrival copy 0x15fc88, the save's
/// 0x14c050), the checkpoint, this visit's kills 0x1baea4, the never-again bytes 0x1bbb04, the visit death bits
/// 0x1ba950, the persistent death bits 0x14c190 and the bolt-drop slots 0x14d590 (spawn id / collected).
/// `RC_RESPAWN_PROBE_AT=<tick>` prints it once at that gameplay tick too (headless runs).
fn respawn_probe(keys: Res<ButtonInput<KeyCode>>, play: Option<Res<Play>>, state: Option<Res<Persistent>>, mut done: Local<Option<u64>>) {
    static AT: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    let at = *AT.get_or_init(|| std::env::var("RC_RESPAWN_PROBE_AT").ok().and_then(|v| v.trim().parse().ok()));
    let tick = play.as_ref().map(|p| p.game.counter);
    let timed = at.is_some() && tick == at && *done != at;
    if !keys.just_pressed(KeyCode::F10) && !timed { return; }
    if timed { *done = at; }
    let Some(p) = play else { println!("respawn probe: no gameplay"); return };
    let l = p.level;
    let s = &p.svc.save;
    let lv = state.as_ref().and_then(|g| g.0.levels.get(l as usize));
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ");
    let list = |v: Vec<String>| if v.is_empty() { "-".to_string() } else { v.join(" ") };
    let bytes = |m: &HashMap<i16, u8>| {
        let mut v: Vec<_> = m.iter().filter(|&(_, &b)| b != 0).map(|(&k, &b)| (k, b)).collect();
        v.sort();
        list(v.into_iter().map(|(k, b)| format!("{k}={b}")).collect())
    };
    let mut visit: Vec<i16> = s.death_level.iter().copied().collect();
    visit.sort();
    let mut persistent: std::collections::BTreeSet<i32> = s.death.iter().filter(|(dl, _)| *dl == l).map(|&(_, id)| id as i32).collect();
    if let Some(lv) = lv {
        for i in 0..0x800 { if lv.killed.get(i >> 3).is_some_and(|b| b >> (i & 7) & 1 != 0) { persistent.insert(i as i32); } }
    }
    println!("== respawn probe (port) tick {}  level {l}", p.game.counter);
    println!("missions arrival: {}", hex(&p.missions.slot));
    println!("missions save:    {}", lv.map_or("-".to_string(), |lv| hex(&lv.missions)));
    println!("checkpoint: {}", s.checkpoint.is_some() as u8);
    println!("killed this visit: {}", bytes(&s.killed));
    println!("never again: {}", bytes(&s.collected));
    println!("visit death bits: {}", list(visit.iter().map(|i| i.to_string()).collect()));
    println!("persistent bits: {}", list(persistent.iter().map(|i| i.to_string()).collect()));
    let slots = lv.map(|lv| lv.bolt_drops.iter().enumerate().filter(|(_, d)| d.first != 0).map(|(k, d)| format!("{k}:{}/{}", d.first - 1, d.collected)).collect()).unwrap_or_default();
    println!("bolt slots: {}", list(slots));
}

fn debug_dump(
    keys: Res<ButtonInput<KeyCode>>,
    play: Option<Res<Play>>,
    meshes: Query<(&MeshTag, &InheritedVisibility), With<MeshMaterial3d<MobyMaterial>>>,
    vis: Query<&InheritedVisibility>,
) {
    if !keys.just_pressed(KeyCode::F9) { return; }
    let Some(p) = play else { println!("dump: no gameplay"); return };
    let h = &p.game.hero;
    let hp = h.pos.map(|v| v.to_f32());
    println!(
        "dump: tick {} hero mode {} state {:#x} pos ({:.2}, {:.2}, {:.2}) body {:?} hero moby {} ratchet moby {}",
        p.game.counter, h.mode, h.state, hp[0], hp[1], hp[2], h.bodies.moby, h.hero_moby(p.hero_id), p.hero_id
    );
    let shown = p.entities.iter().filter(|&&e| vis.get(e).is_ok_and(|v| v.get())).count();
    println!("dump: engine Ratchet draw: cached hidden {}, {} of {} entities visible", p.ratchet_hidden, shown, p.entities.len());
    let mut visible: std::collections::HashMap<u32, (usize, usize)> = Default::default();
    for (tag, v) in &meshes {
        let e = visible.entry(tag.0).or_default();
        e.1 += 1;
        if v.get() { e.0 += 1; }
    }
    for (id, m) in p.game.mobys.mobys.iter().enumerate() {
        if m.is_deleted() { continue; }
        let d = ((m.position[0] - hp[0]).powi(2) + (m.position[1] - hp[1]).powi(2) + (m.position[2] - hp[2]).powi(2)).sqrt();
        if d > 12.0 { continue; }
        let (sv, st) = visible.get(&(id as u32)).copied().unwrap_or((0, 0));
        println!(
            "dump:   moby {id:4} class {:5} state {:#04x} mode {:#06x} coll {} pos ({:.2}, {:.2}, {:.2}) dist {:.1} meshes {sv}/{st} visible",
            m.o_class, m.state, m.mode, m.has_collision, m.position[0], m.position[1], m.position[2], d
        );
    }
}

/// The entities of instances a death reload made spawn (their level load had hidden them: `SpawnHidden`).
fn show_reloaded(play: Option<ResMut<Play>>, mut commands: Commands, gameplay_entities: GameplayEntities, mut occl: Option<ResMut<MobyOcclusion>>, mut anim: Option<ResMut<MobyAnim>>) {
    let Some(mut p) = play else { return };
    if p.reload_shown.is_empty() { return; }
    let shown = std::mem::take(&mut p.reload_shown);
    for (e, tag) in &gameplay_entities {
        if shown.contains(&(tag.0 as usize)) { commands.entity(e).remove::<SpawnHidden>().insert(Visibility::Hidden); }
    }
    for &ii in &shown {
        if let Some(o) = occl.as_deref_mut() { o.reshow(ii); }
        if let (Some(a), Some(k)) = (anim.as_deref_mut(), occl.as_deref().and_then(|o| o.anim_index(ii))) { a.hidden[k] = false; }
    }
}

/// The respawn: the death reload's hero side (`InitLevelRenderGlobals` and `0x29adc8` inside `LoadLevelCoreData(0,
/// 1)`), also the R key's. In the game's order:
/// * the hero init `0x226b70`: the slot item ids (+0x28 of the slots at 0x140408 + 0x50·k: hand, feet, head, back)
///   survive the clear of the hero block 0x13f350..0x141660 (the slot states and the requests 0x141408.. do not); an
///   empty saved hand item 0x141660 becomes the bomb glove; HP = max HP;
/// * `FUN_00226e90`: the saved head item 0x141668 = 0, so the headgear does not come back (the saved hand, feet and
///   back items lie outside the cleared block: the weapon, the boots and the pack are made again by the first hero
///   update); the no-air flag 0x14161b and the state 0x1413d4 = 0 (the fresh hero's);
/// * the camera init and the level's camera slots (0x20ef58); the tick counter 0x15f5cc = 0;
/// * with a checkpoint record, `0x29adc8`: the record's position and Euler angles (also Ratchet's moby's matrix),
///   Ratchet's moby's light word and ambient, the body (made by the next tick) and the camera snapped behind him.
///
/// The RNG continues. Stores the dead hero's tick queued for the next tick are dropped (the game's land on the old
/// hero block before the clear).
fn respawn(p: &mut Play, coll: &rc_formats::collision::Collision, class: &MobyAnimClass, death_z: f32, state: Option<&mut GameState>, session: Option<&mut SessionState>) {
    let mut table = std::mem::take(&mut p.game.mobys);
    table.mobys[p.hero_id] = p.spawn_moby.clone();
    let (rng, options) = (p.game.rng, p.game.options);
    let old = &p.game.hero;
    let ids = (old.items.slot.id, old.feet_slot.id, old.head_slot.id, old.back_slot.slot.id);
    // The hero init on the running stream (its fidget-timer draw included).
    let mut g = Game::with_rng(coll, table, p.hero_id, options, death_z, rng);
    g.item_data = p.item_data.clone();
    g.grind_paths = p.game.grind_paths.clone();
    hero_level_setup(&mut g.hero, p.back_classes.as_ref(), p.level);
    g.hero.joint_targets = p.game.hero.joint_targets.clone();
    // Ratchet's and the packs' joint lists (level data the hero's effects read).
    g.hero.fx.joints = p.game.hero.fx.joints.clone();
    (g.hero.items.slot.id, g.hero.feet_slot.id, g.hero.head_slot.id, g.hero.back_slot.slot.id) = ids;
    if let (Some(gs), Some(s)) = (state, session) {
        s.hero_init(gs.global.max_hp);
        g.hero.health = s.hp;
        let e = &mut gs.global.equipped;
        if e[0] == 0 { e[0] = rc_game::game_state::item::BOMB_GLOVE as i32; }
        e[2] = 0;
    }
    if let Some(cp) = p.svc.save.checkpoint {
        use rc_game::hero::physics::{euler_rows, from_f32x3};
        g.hero.pos = from_f32x3(cp.pos);
        g.hero.rot = from_f32x3(cp.rot);
        g.hero.rows = euler_rows(g.hero.rot);
        // `0x221960(moby + 0xc0, 0x13f3e0)`: Ratchet's moby's matrix from the record's Euler angles (the camera
        // reset below snaps behind it; the target yaw 0x13f4d0 stays the hero init's).
        g.hero.write_back(&mut g.mobys.mobys[p.hero_id]);
        if let Some((light, ambient)) = p.svc.save.checkpoint_light {
            let m = &mut g.mobys.mobys[p.hero_id];
            (m.light, m.ambient) = (light, ambient);
        }
        let cam = rc_game::follow_camera::CamInput { hero: &g.hero, pad: &g.pad, coll, mobys: None, hero_moby: None };
        g.camera = rc_game::follow_camera::Camera::new(&cam, options.camera);
    }
    g.camera.set_level(p.game.camera.level_cams.restarted());
    p.game = g;
    p.svc.hero_writes = None;
    // The switch back into the checkpoint's body, made by the next tick; the body moby's animation binding is
    // dropped with the old hero.
    p.body_anim = None;
    if p.svc.save.checkpoint.is_some() {
        let (body, st) = p.svc.save.checkpoint_body;
        rc_game::hero::bodies::restore_from_checkpoint(&mut p.game.hero, &p.game.mobys, body, st);
    }
    let hold = std::mem::take(&mut p.ratchet.hold);
    p.ratchet = RatchetAnim::new(class);
    p.ratchet.arm_joints = p.arm_joints.clone();
    p.ratchet.hold = hold;
    p.ratchet.mirror = p.mirror_anim;
    p.frozen_hint = false;
}

/// The tick's HUD weapon slot, the ammo table and the sound layer (one system parameter).
type HudAudio<'w> = (Option<ResMut<'w, HeldWeapon>>, Option<Res<'w, AmmoTable>>, Option<ResMut<'w, crate::audio_out::AudioOut>>);

type MainCamera<'w, 's> = Query<'w, 's, &'static Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>;

/// One 60 Hz gameplay tick (see the module docs). Skipped in the modes whose update does not run the game
/// tick (crate::menu_render: `Mode::advances_tick`, e.g. the mode-3 menus).
#[allow(clippy::too_many_arguments)]
fn tick(
    play: Option<ResMut<Play>>,
    level: Res<crate::Level>,
    pad: Res<PadFrame>,
    script: Res<PlayScript>,
    source: Res<CameraSource>,
    mut budget: ResMut<TickBudget>,
    mut anim: Option<ResMut<MobyAnim>>,
    mut attach: Option<ResMut<MobyAttach>>,
    mut particles: Option<ResMut<ParticleSim>>,
    cams: MainCamera,
    mut view: Option<ResMut<PlayView>>,
    mut state: Option<ResMut<Persistent>>,
    mut session: Option<ResMut<Session>>,
    mut mode: Option<ResMut<crate::menu_render::MenuMode>>,
    mut occl: Option<ResMut<MobyOcclusion>>,
    (mut held, ammo, mut audio): HudAudio,
) {
    let Some(mut play) = play else { return };
    // Mode 5's world frames (crate::interact_render: substates 0 / 2) run one tick in the scene form.
    let world_frame = mode.as_ref().is_some_and(|m| !m.state.mode.advances_tick() && m.world_tick);
    if mode.as_ref().is_some_and(|m| !m.state.mode.advances_tick()) && !world_frame { return; }
    if budget.0 == 0 { return; }
    if world_frame {
        if let Some(m) = mode.as_mut() {
            m.world_tick = false;
            m.world_ticked = true;
        }
    }
    budget.0 -= 1;
    let lv = &level.0;
    let Some(coll) = lv.collision.as_ref() else { return };
    let p = &mut *play;
    let class = &lv.mobys.anim[p.class];
    if std::mem::take(&mut p.respawn) {
        let counter = p.game.counter;
        respawn(p, coll, class, lv.death_z, state.as_deref_mut().map(|s| &mut s.0), session.as_deref_mut().map(|s| &mut s.0));
        p.game.counter = counter;
        println!("gameplay: respawned at the uid-0 moby (R)");
    }
    // The death reload (`LoadLevelCoreData(0, 1)`), once the fade and the reload's black ran (`scene_render::FadeHold`):
    // the mobys, the hero side, the checkpoint's visit records (`0x29b080`), the load pass; then the main loop's
    // `music_start_track` with the checkpoint's track.
    if std::mem::take(&mut p.death_pending) {
        let checkpoint = p.svc.save.checkpoint.is_some();
        death_reload(p, lv, state.as_deref().map(|s| &s.0), occl.as_deref_mut(), anim.as_deref_mut(), particles.as_deref_mut());
        // `0x29adc8`'s copy of the checkpoint area 0x1bb6b0..0x1bc310 over 0x1baa50..0x1bb6b0 (zeroed without a
        // checkpoint): the live visit records 0x1baaa0 become the checkpoint's 0x1bb700, and this visit's kills 0x1baea4
        // become a copy of the never-again bytes 0x1bbb04. Kills a checkpoint did not promote before the death are
        // forgotten (PCSX2: Kerwan's train troopers killed on the ride, 0x1baea4 = 5 → 0 after the respawn, so the
        // station's checkpoint taken again never makes them permanent).
        let save = &mut p.svc.save;
        if checkpoint {
            save.visit = save.checkpoint_visit.clone();
            save.killed = save.collected.clone();
        } else {
            save.visit.clear();
            save.killed.clear();
        }
        respawn(p, coll, class, lv.death_z, state.as_deref_mut().map(|s| &mut s.0), session.as_deref_mut().map(|s| &mut s.0));
        if checkpoint { rc_game::moby_update::visit::restore(&mut p.game.mobys, &p.svc.save.checkpoint_visit); }
        reload_load_pass(p, coll, particles.as_deref_mut(), state.as_deref().map(|s| s.0.global.vendor));
        if let Some(a) = audio.as_deref_mut() { a.system().death_reload(checkpoint); }
        println!("gameplay: death reload done: respawned at {}", if checkpoint { "the checkpoint" } else { "the uid-0 moby" });
    }

    let input = match &script.0 {
        // Indexed by the main-loop frame (= the gameplay tick index, counter − 1 after the load's increment,
        // until a menu pauses the tick).
        Some(s) => s.at(mode.as_ref().map_or(p.game.counter.saturating_sub(1), |m| m.loop_frame)),
        None if *source == CameraSource::Play => pad.0,
        None => PadInput::neutral(),
    };
    // Dev builds: the noclip takes the pad (the tick sees it neutral; noclip_step moves the hero after it).
    let noclip_pad = p.noclip.map(|_| input);
    let input = if noclip_pad.is_some() { PadInput::neutral() } else { input };
    // The view of the last rendered frame (0x16d140, `FastBSphereCheck`): the main camera as last drawn.
    let tans = particles.as_deref().map_or_else(|| crate::particle_render::view_tans(None), |s| s.view_tan);
    let view_cull = cams.iter().next().map(|t| crate::particle_render::bsphere_view(t, tans));
    let level_index = p.level;
    // The scene form of the tick (crate::scene_render: mode 2).
    let scene_frame = p.game.camera_paused;
    // The item tables the pickups read (rc_game::moby_update::classes::pickup::ItemTables): the price records, the
    // vendor list 0x15edd0; the owned items and ammo come from Ratchet's mirrors inside the moby hook.
    let slots: Vec<i32> = p.item_data.as_ref().map_or_else(Vec::new, |d| d.defs.iter().map(|d| d.slot).collect());
    let item_base = rc_game::moby_update::classes::pickup::ItemTables::new(&p.svc.interact.tables.shop.records, state.as_deref().map_or([0xff; 12], |s| s.0.global.vendor)).with_slots(&slots);
    let parts_cell = RefCell::new(particles.as_deref_mut());
    let svc_cell = RefCell::new(&mut p.svc);
    let (sched, classes_arc, emitters, missions) = (&mut p.sched, &p.classes, &p.emitters, &p.missions);
    let classes: &ClassTable = classes_arc;
    let debug_hits = &p.debug_hits;
    // The sound layer (crate::audio_out): the moby loop's class sounds and the tick's sound step.
    let audio_cell = RefCell::new(audio.as_deref_mut());
    let hero_id = p.hero_id;
    let mut n_active = 0usize;
    let mut mobys = |table: &mut MobyTable, hero: &Hero, rng: &mut Rng, cam: &CameraView, coll: &Collision, counter: u64| {
        let mut parts = parts_cell.borrow_mut();
        let mut svc = svc_cell.borrow_mut();
        let mut audio_ref = audio_cell.borrow_mut();
        let mut sink = audio_ref.as_deref_mut().map(|a| ClassSoundSink { audio: a.system(), listener: class_sounds::listener_of(cam), hero: Some(hero_id) });
        let mut ext = Externals { level: level_index, emitters, view: view_cull.as_ref() };
        let inv = item_base.clone().with_hero(hero);
        let mut w = World::new(table, hero, rng, classes, &mut svc, counter);
        w.inventory = &inv;
        w.sound = sink.as_mut().map(|s| s as &mut dyn rc_game::moby_update::services::SoundSink);
        w.camera = cam.pos;
        w.camera_yaw = cam.yaw().to_f32();
        w.camera_rows = cam.rows_f32();
        w.camera_class = cam.class;
        w.coll = Some(coll);
        w.particles = parts.as_deref_mut().map(|p| &mut p.sys);
        w.view = view_cull.as_ref();
        w.external = Some(&mut ext);
        w.missions = missions;
        // RC_DEBUG_HIT: a wrench-like hit (flags 0x10000, damage 1) delivered before the moby loop of tick index
        // N (counter N + 1: the load pass ran at 0); to Ratchet an enemy-contact-like hit (flags 1: the hero's
        // hit intake takes it; no attacker, so he is pushed straight back).
        for &(id, _) in debug_hits.iter().filter(|h| h.1 + 1 == counter) {
            let flags = if id == hero_id { 1 } else { 0x1_0000 };
            w.deliver_hit(id, &HitTemplate { flags, damage: rc_game::ps2v::Pf::ONE, ..Default::default() });
        }
        // PromptTick 0x278eb8 and this tick's pad for the "use" system (moby_update::interact), before the loop.
        w.svc.interact.begin_tick(hero.loop_in.pad.pressed, hero.state);
        n_active = sched.tick(&mut w);
        // RC_UNPORTED=1: the calls the moby loop met without a port so far (`Services::unported`), every 600 ticks.
        if counter.is_multiple_of(600) && std::env::var("RC_UNPORTED").is_ok_and(|v| v == "1") {
            eprintln!("unported after tick {counter}: {:?}", w.svc.fx.unported);
        }
    };
    let mut parts = |hero: &Hero, cam: &CameraView, rng: &mut Rng, counter: u64| {
        if let Some(sim) = parts_cell.borrow_mut().as_deref_mut() {
            sim.sys.counter = counter;
            // 0x167240 as the previous tick's camera update left it (the type-11 sparks read it), and the game's
            // one rand stream.
            sim.sys.camera = [cam.pos[0].0, cam.pos[1].0, cam.pos[2].0];
            sim.sys.cam_yaw = cam.yaw().to_f32();
            // 0x13f640, the water level (type 35's drops land on it).
            sim.sys.water_z = hero.water_level.to_f32();
            // The particles the hero update spawned (sparks, sand, …: rc_game::hero::fx), in its order.
            rc_game::hero::fx::create_particles(hero, &mut sim.sys);
            crate::particle_render::update_parts(sim, Some(rng));
        }
        // FUN_00220928, right after UpdateParts (InLevelFrameUpdate only: not in the mode-2 scene frame).
        if !scene_frame { svc_cell.borrow_mut().glints.update(); }
    };
    // The moby collision the hero and the camera query (and Ratchet's MobyBuildMatrix, the camera's crate hit).
    let mut world = HeroWorld { world: SharedServices { svc: &svc_cell, classes: classes_arc.clone() }, water: HeroWater(&svc_cell) };
    let mut hooks = TickHooks { mobys: &mut mobys, particles: &mut parts, world: Some(&mut world) };
    // The hand-swap globals in from the saved game / session (the quick-select ring writes the request).
    p.game.item_globals = item_globals(state.as_deref().map(|s| &s.0), session.as_deref().map(|s| &s.0));
    // The item table 0x13d4c0 (the hero's owned mirror) and the back slot's globals: the saved back item 0x14166c
    // (equipped[3]), 0x15ed94, the request 0x141414 (the session's temp back item) and Clank hidden 0x141628.
    // The ammo table 0x13d428 and the items' "uses ammo" (records +8) and weapon fields (definitions) for the weapons.
    if let Some(a) = ammo.as_deref() {
        for (i, &(has, _)) in a.0.iter().enumerate().take(p.game.hero.weapons.uses_ammo.len()) { p.game.hero.weapons.uses_ammo[i] = has; }
        if p.game.hero.weapons.defs.is_empty() { p.game.hero.weapons.defs = a.1.clone(); }
    }
    // The Taunter's whistle cycle from its class's sound defs (rc_game::hero::taunter::cycle_of).
    if p.game.hero.weapons.reactive.taunter.cycle.is_empty() {
        if let Some(a) = audio_cell.borrow_mut().as_deref_mut() {
            if let Some(c) = a.system().data.sounds.classes.iter().find(|c| c.o_class == rc_game::hero::taunter::CLASS as i32) {
                p.game.hero.weapons.reactive.taunter.cycle = rc_game::hero::taunter::cycle_of(&c.defs);
            }
        }
    }
    if let Some(gs) = state.as_deref() {
        p.game.hero.weapons.ammo = gs.0.global.ammo;
        p.game.hero.owned.0 = gs.0.global.owned;
        // The gold weapons 0x13e520 (by item id) as the hero code reads them (`Weapons::gold`; G-WPN-009).
        for (d, v) in p.game.hero.weapons.gold.iter_mut().zip(gs.0.global.gold_weapons.iter()) { *d = *v; }
        {
            // The moby side's view of the same table: the item offers and the hit resolver (`Counters::gold_weapons`), the
            // gold Suck Cannon 0x13e529 (item 9: ten slots) and the gold Morph-o-Ray 0x13e535 (item 21: the gold chickens).
            let mut svc = svc_cell.borrow_mut();
            let g = &gs.0.global.gold_weapons;
            if svc.counters.gold_weapons.as_slice() != &g[..] { svc.counters.gold_weapons = g.to_vec(); }
            svc.creatures.react.gold = g.get(9).is_some_and(|&b| b != 0);
            svc.creatures.react.gold_morph = g.get(21).copied().unwrap_or(0);
        }
        svc_cell.borrow_mut().interact.sync_game(&gs.0);
        // The cheat bytes 0x15edb0 (rc_game::cheats) for the tick and the hero (GameOptions) and the moby loop
        // (Services::cheats), and the options the game reads every tick (the mirror 0x15edb4, the camera's).
        let g = &gs.0.global;
        let mut o = gs.0.options().game_options();
        o.cheats = rc_game::cheats::Cheats(g.cheats_active);
        o.cheat_entry = g.game_beaten != 0 || g.completes != 0;
        p.game.options = o;
        p.game.camera.opts = o.camera;
        svc_cell.borrow_mut().cheats = o.cheats;
        if p.game.cheat_patterns.is_empty() { p.game.cheat_patterns = std::sync::Arc::new(cheat_patterns()); }
        // The help records, log, play time and options (rc_game::help), and the hero's copy of the records.
        svc_cell.borrow_mut().help.sync_in(&gs.0);
        p.game.hero.help.records = svc_cell.borrow().help.records.clone();
        // Max health 0x15eda0 (the nanotech orbs heal up to it) and the bolt grabber 0x13d4e2 (item 34: the pickup
        // volume 12 / 4.5).
        svc_cell.borrow_mut().counters.max_hp = gs.0.global.max_hp;
        svc_cell.borrow_mut().bolt_grabber = gs.0.global.owned[34] != 0;
        p.game.hero.back_slot.slot.saved = gs.0.global.equipped[3];
        p.game.hero.back_slot.thruster_last = gs.0.global.thruster_last;
        // The feet / head slots' saved items 0x141664 / 0x141668 (rc_game::inventory).
        p.game.hero.feet_slot.saved = gs.0.global.equipped[1];
        p.game.hero.head_slot.saved = gs.0.global.equipped[2];
    }
    if let Some(s) = session.as_deref() {
        p.game.hero.feet_slot.request = s.0.temp_feet;
        p.game.hero.head_slot.request = s.0.temp_head;
        p.game.hero.back_slot.slot.request = s.0.temp_back;
        p.game.hero.back_slot.clank_hidden = s.0.clank_hidden;
    }
    let mut hits = CellHits { svc: &svc_cell, classes, coll, parts: Some(&parts_cell), items: Some(&item_base), missions: Some(missions), audio: Some((&audio_cell, class_sounds::listener_of(&p.game.camera.out), hero_id)) };
    // The sound step (after the camera, before the counter increment).
    let mut sound = |table: &MobyTable, hero: &Hero, cam: &CameraView, rng: &mut Rng, counter: u64| {
        let mut audio_ref = audio_cell.borrow_mut();
        let Some(out) = audio_ref.as_deref_mut() else { return };
        let (sys, buf) = out.parts();
        class_sounds::sound_step(sys, table, hero, cam, rng, counter, buf);
        out.push_frame();
    };
    let has_audio = audio_cell.borrow().is_some();
    // idle.counter: 0x15f5cc as the hero update reads it (the counter before this tick's increment).
    p.game.hero.idle.counter = p.game.counter as i32;
    // Ratchet's own sounds (his animation triggers, his voices) inside the hero update; the listener is the
    // camera the hero update sees (the previous tick's).
    // The other bodies' animation classes by o_class (rc_game::hero::bodies).
    let body_class = |o: i16| lv.mobys.classes.iter().position(|c| c.o_class as i16 == o).map(|ci| &lv.mobys.anim[ci]);
    let mut hero_sounds = HeroClassSounds {
        audio: || std::cell::RefMut::filter_map(audio_cell.borrow_mut(), |a| a.as_deref_mut().map(|o| o.system())).ok(),
        class,
        body_classes: Some(&body_class),
        listener: class_sounds::listener_of(&p.game.camera.out),
        // `0x1413d0`, the sounds' owner: the body moby while a body is in (Clank's command voices and steps follow
        // him, not Ratchet's moby left behind).
        hero: p.game.hero.hero_moby(hero_id),
        counter: p.game.counter,
    };
    // The hero's animation: Ratchet's, or the body moby's while a body is the hero moby (rc_game::hero::bodies).
    let mut anim_ctl = rc_game::hero::anim::HeroAnimCtl { ratchet: p.ratchet.ctl(class), body: &mut p.body_anim, classes: &body_class };
    let sound = if has_audio { Some(&mut sound as &mut rc_game::tick::SoundHook) } else { None };
    let report = p.game.tick_with_hero_sounds(Some(&input.bytes()), coll, &mut anim_ctl, &mut hooks, &mut hits, sound, &mut hero_sounds);
    if let Some(pad) = noclip_pad { noclip_step(p, &pad); }
    // … and back out.
    let g = p.game.item_globals;
    if let Some(gs) = state.as_mut() {
        let gl = &mut gs.0.global;
        if gl.equipped[0] != g.saved { gl.equipped[0] = g.saved; }
        if gl.last_hand_item != g.previous { gl.last_hand_item = g.previous; }
        if gl.wrench_held != g.wrench_flag { gl.wrench_held = g.wrench_flag; }
    }
    if let Some(s) = session.as_mut() {
        if s.0.temp_hand != g.request { s.0.temp_hand = g.request; }
        if s.0.drone != g.drone { s.0.drone = g.drone; }
        let b = p.game.hero.back_slot.slot.request;
        if s.0.temp_back != b { s.0.temp_back = b; }
        let (f, h) = (p.game.hero.feet_slot.request, p.game.hero.head_slot.request);
        if s.0.temp_feet != f { s.0.temp_feet = f; }
        if s.0.temp_head != h { s.0.temp_head = h; }
        // 0x141628 as a class stored it (Veldin's Clank 834: `HeroFields::clank_hidden`).
        let ch = p.game.hero.back_slot.clank_hidden;
        if s.0.clank_hidden != ch { s.0.clank_hidden = ch; }
    }
    if let Some(gs) = state.as_mut() {
        let (saved, last) = (p.game.hero.back_slot.slot.saved, p.game.hero.back_slot.thruster_last);
        let gl = &mut gs.0.global;
        let (fs, hs) = (p.game.hero.feet_slot.saved, p.game.hero.head_slot.saved);
        if gl.equipped[1] != fs { gl.equipped[1] = fs; }
        if gl.equipped[2] != hs { gl.equipped[2] = hs; }
        if gl.equipped[3] != saved { gl.equipped[3] = saved; }
        if gl.thruster_last != last { gl.thruster_last = last; }
    }

    // The ammo the weapons used (0x249450: the ammo and the ammo-used stat 0x13dea0).
    if let Some(gs) = state.as_mut() {
        let w = &mut p.game.hero.weapons;
        if gs.0.global.ammo != w.ammo { gs.0.global.ammo = w.ammo; }
        if w.used.iter().any(|&u| u != 0) {
            for (t, u) in gs.0.global.ammo_used.iter_mut().zip(w.used.iter_mut()) { *t += std::mem::take(u); }
        }
        // … and picked up (the ammo pickups 0x2db028: 0x13de08).
        if w.picked.iter().any(|&u| u != 0) {
            for (t, u) in gs.0.global.ammo_picked_up.iter_mut().zip(w.picked.iter_mut()) { *t += std::mem::take(u); }
        }
    }
    // The melee entries' stats records (SetState 0x23cf98: 0x1416c0 = levels[8] 3007, misc 0/1, gadget 17).
    if p.game.hero.melee.entered != [0; 3] {
        if let (Some(gs), Some(s)) = (state.as_mut(), session.as_ref()) {
            rc_game::hero::melee::apply_melee_stats(&mut p.game.hero.melee, &mut gs.0, &s.0);
            // The help system's copy of the records (written back by `Help::sync_out` below) takes the bumped move /
            // gadget records, so the write-back keeps them (G-SAV-009).
            let r = &mut p.svc.help.records;
            r.moves[0] = gs.0.global.move_help[0];
            r.moves[1] = gs.0.global.move_help[1];
            r.gadget[17] = gs.0.global.gadget_help[17];
        }
    }
    // The cheat entry's toggle (`0x2285a0`): 0x15edc0[i] = 1, 0x15edb0[i] flipped, `ShowBanner(0x4fbe / 0x4fbf, −1)`.
    if let Some(t) = p.game.cheat_toggled.take() {
        if let Some(gs) = state.as_mut() {
            gs.0.global.cheats_ever[t.slot] = 1;
            gs.0.global.cheats_active[t.slot] = t.on as u8;
        }
        rc_game::cinematic::banner_call(&mut p.svc.cinematic, t.banner(), rc_game::hud::scale_ticks(rc_game::cheats::BANNER_TICKS));
        println!("cheats: tick {}: cheat {} {} (the move entry 0x2285a0)", p.game.counter, t.slot, if t.on { "enabled" } else { "disabled" });
    }
    // The help system (rc_game::help), then the play time 0x15eea4 (`FUN_002ab960` at the tick's end).
    help_frame(p, &report, scene_frame || world_frame, state.as_deref_mut().map(|s| &mut s.0), audio_cell.borrow_mut().as_deref_mut());
    map_tick(p, &report, state.as_deref().map(|s| &s.0));
    if let Some(gs) = state.as_mut() { gs.0.global.play_time += 1; }
    // Moby sounds: queued and counted (no moby-sound entry in rc_game::audio yet).
    for ev in p.svc.sounds.drain(..) { *p.sounds.entry((ev.o_class, ev.index)).or_default() += 1; }
    // The talk system's saved-game writes (moby_update::interact::GameWrite).
    if let Some(gs) = state.as_mut() {
        for w in p.svc.interact.apply_writes(&mut gs.0) {
            println!("interact: tick {}: game write {w:?}", p.game.counter);
            // A class's `GiveItem(item, equip)` (0x275760; Pokitaru's commando 114): the item tables from the disc.
            if let rc_game::moby_update::interact::GameWrite::GiveItem { item, equip } = w {
                let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
                let tables = crate::disc_source::read_path(&root, &root.join("boot/SCUS_971.99")).ok()
                    .zip(crate::disc_source::level_file(&root, index, "overlay.bin").ok())
                    .and_then(|(elf, ov)| ItemTables::load(&elf, &ov).ok());
                match (tables, session.as_mut()) {
                    (Some(t), Some(s)) if item < rc_formats::save_game::ITEM_COUNT => gs.0.give_item(item, equip, &t, &mut s.0),
                    _ => println!("interact: GiveItem({item}) not applied (no item tables or session)"),
                }
            }
        }
    }
    // The bolt counter (0x15ed98, 0x13df38[level]) into the persistent state the HUD and menus read.
    if let Some(gs) = state.as_mut() {
        let c = &p.svc.counters;
        if gs.0.global.bolts != c.bolts { gs.0.global.bolts = c.bolts; }
        if let Some(l) = gs.0.levels.get_mut(level_index as usize) {
            let lb = c.level_bolts[level_index as usize % 20];
            if l.bolts != lb { l.bolts = lb; }
        }
    }
    if let Some(o) = occl.as_deref_mut() { drive_statics(p, lv, o, anim.as_deref_mut()); }

    if p.trace {
        let h = &p.game.hero;
        let s = &p.ratchet.state;
        println!(
            "tick {:5}: state {:#04x} pos {:.4?} yaw {:+.4} air {:3} | anim {}:{} -> {}:{} t {:.3} | cam {:.3?} | {:?} | rng {:#010x} \
             mobys {n_active} bolts {} free {} dyn {}/{} parts {} grass seq1 {} | hand {}:{} {:?} combo {} hit {} fr {:.2} tip {:.3?} | back {} | snd {} | hp {} inv {} | shake {:.4} {:.4}",
            p.game.counter - 2, h.state, h.position(), h.yaw().to_f32(), h.air_ticks, s.seq_a, s.frame_a, s.seq_b, s.frame_b, s.t,
            report.camera.pos_f32(), report.hero, p.game.rng.state, p.svc.counters.bolts, p.game.mobys.free_slots,
            p.dynamic.drawn, p.dynamic.live, particles.as_ref().map_or(0, |s| s.sys.pool.count),
            p.game.mobys.mobys.iter().filter(|m| matches!(m.o_class, 724 | 725) && m.anim.seq_b == 1).count(),
            h.items.slot.id, h.items.slot.state, h.items.slot.item.as_ref().map(|m| (m.anim.seq_b, m.anim.frame_b, m.position)),
            h.melee.combo, h.melee.hit, p.ratchet.frame.to_f32(), rc_game::hero::physics::to_f32x3(h.melee.tip),
            h.back.as_ref().map_or("-".to_string(), |b| format!("pack {}:{} clank {}:{}", b.pack.anim.seq_b, b.pack.anim.frame_b, b.clank.anim.seq_b, b.clank.anim.frame_b)),
            audio_cell.borrow().as_ref().map_or("-".to_string(), |a| { let st = a.stats(); format!("{} plays, class {}/{}", st.plays, st.class_slots, st.class_sounds) }),
            h.health, h.f510, p.game.camera.shake[0].offset.to_f32(), p.game.camera.shake[1].offset.to_f32()
        );
    }
    // The swim's ripple disturbances (`RippleDisturb` 0x2b82a8 on every patch; it draws nothing, so after the tick is
    // the game's order for the 751 update of the next tick). The splashes, rings, drops and bubbles are the hero's
    // particles and splash mobys (rc_game::hero::swim::effects), the voices play in the hero update.
    for e in std::mem::take(&mut p.game.hero.swim.events) {
        if let rc_game::hero::swim::SwimEvent::Ripple { x, y, r, amp } = e {
            p.svc.water.disturb(x, y, r, amp, false);
        }
        // The surface jump's store `0x167494 = 0` (`0x2406b0`), made in the hero update after the moby loop (so it
        // overrides a HeroTeleport store of the same tick, as in the game); the camera applies it before its test.
        if e == rc_game::hero::swim::SwimEvent::UnderwaterOff {
            p.svc.water.underwater_store = Some((p.game.counter.wrapping_sub(1), rc_game::water::world::UnderwaterStore::Off));
        }
    }
    // Hits taken and deaths (rc_game::hero::damage): the game state's counters (0x15eea8 / 0x13df88[level],
    // 0x15eeac / 0x13dfd8[level]) and the killer's mission deaths (0x14ee90, `LevelMissions::hero_death`).
    for e in std::mem::take(&mut p.game.hero.damage.events) {
        let gs = state.as_mut().map(|g| &mut g.0);
        match e {
            DamageEvent::Hit => {
                if let Some(gs) = gs {
                    gs.global.total_hits += 1;
                    if let Some(l) = gs.levels.get_mut(level_index as usize) { l.hits += 1; }
                }
            }
            DamageEvent::Died { killer_mission, killer_class } => {
                if let Some(gs) = gs {
                    gs.global.total_deaths += 1;
                    if let Some(l) = gs.levels.get_mut(level_index as usize) { l.deaths += 1; }
                    // 0x225938: bit 31 of the 148 help records' masks (0x14196c + 8·k) cleared, so their help plays again.
                    for r in gs.global.help.iter_mut() { r.mask &= 0x7fff_ffff; }
                }
                for r in p.svc.help.records.help.iter_mut() { r.mask &= 0x7fff_ffff; }
                p.missions.hero_death(killer_mission);
                println!("gameplay: tick {}: Ratchet died in state {:#x} (killer class {killer_class:?}, mission {killer_mission:?})", p.game.counter, p.game.hero.state);
            }
        }
    }
    if let HeroTick::Unimplemented(s) = report.hero {
        if !std::mem::replace(&mut p.frozen_hint, true) { println!("gameplay: hero frozen in unported state {s:#x}; R respawns"); }
    }
    // The death flag 0x141401: the death sequence 0x2319b0 and the bounds check (x / y outside 2..1022) both call
    // `FadeToBlack(16)` and set it; the main loop runs no catch-up tick, skips `DrawWorld` and reloads at the end of
    // the frame (the disc read keeps the screen black, [`RELOAD_BLACK`]).
    if p.game.hero.fell_out != 0 && !p.death_pending {
        let (at, st) = (p.game.hero.position(), p.game.hero.state);
        budget.0 = 0;
        p.death_pending = true;
        p.svc.cinematic.requests.push(rc_game::cinematic::EngineRequest::FadeToBlack { frames: rc_game::menus::scale_ticks(0x10) });
        println!("gameplay: tick {}: death flag 0x141401 (state {st:#x} at {at:.2?}): fade to black, then the reload", p.game.counter);
    }

    publish_anim(p, anim.as_deref_mut());
    let hm = &p.game.mobys.mobys[p.hero_id];
    if let Some(a) = attach.as_mut() { a.set_host(rows_bits(&hm.rows), [hm.position[0], hm.position[1], hm.position[2]]); }
    if let Some(v) = view.as_mut() {
        v.view = p.game.camera.out;
        // 0x16cf70 as the tick's classes left it (`InitViewContext`'s 0.63 unless a vehicle changed it).
        v.tan_half_fov = p.svc.view_tan_x;
    }
    if let Some(s) = session.as_mut() { s.0.hp = p.game.hero.health; }
    // The HUD's weapon slot (0x24f9c0): the held item 0x140408 when it has an ammo HUD and the hero is on foot.
    if let Some(hw) = held.as_mut() {
        let id = p.game.hero.items.slot.id;
        let w = ammo.as_ref().and_then(|a| a.0.get(id.max(0) as usize).copied()).filter(|&(has, _)| has && id > 0 && p.game.hero.mode == 0)
            .map(|(_, max)| (id as u16, state.as_ref().and_then(|s| s.0.global.ammo.get(id as usize).copied()).unwrap_or(0), max as i32));
        if hw.0 != w { hw.0 = w; }
    }
}

/// Ratchet's record (model matrix, light block for the current rows) and palette after a tick, then the
/// dynamic mobys ([`upload_dynamic`]).
#[allow(clippy::too_many_arguments)]
fn upload(
    play: Option<ResMut<Play>>,
    level: Res<crate::Level>,
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut transforms: Query<&mut Transform, Without<Camera3d>>,
    cams: MainCamera,
    projs: Query<&Projection, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut point_lights: ResMut<moby_render::PointLightFrame>,
    shadows: Option<Res<crate::shadow_render::ShadowVolumes>>,
) {
    let Some(mut p) = play else { return };
    if p.uploaded == Some(p.game.counter) { return; }
    let none = std::collections::HashSet::new();
    let deferred = shadows.as_deref().map_or(&none, |s| &s.deferred);
    // The point-light bank after the tick (MobyProc merges it into the mobys' third light, moby_render).
    let lights: Vec<_> = p.svc.point_lights.active().copied().collect();
    if point_lights.0 != lights { point_lights.0 = lights.clone(); }
    p.uploaded = Some(p.game.counter);
    let lv = &level.0;
    let class = &lv.mobys.anim[p.class];
    let mut palette = crate::moby_anim::identity_palette(p.slots);
    // His pose with the weapon arm's pose layers (rc_game::hero::weapons, the moby +0x60 list) and his joint
    // modifiers (head look, lean, eyelids: the moby +0x64 list, rc_game::hero::idle) over it.
    let hm = &p.game.mobys.mobys[p.hero_id];
    let layers = rc_game::hero::anim::pose_layers_with(&p.game.hero.weapons.layers, &p.arm_joints, &p.ratchet.hold);
    let f = moby_anim::evaluate_posed(class, &p.ratchet.state, p.ratchet.snapshot.as_ref(), &layers, &hm.joint_mods);
    for (k, b) in f.iter().take(p.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() { palette[k] = b; }
    let rows = rows_bits(&hm.rows);
    let lights = lv.mobys.lighting.as_ref().map(|l| light::moby_lights(&rows, &l.bank, p.light_word, p.ambient, 0x80));
    let model = moby_render::extra_model(rows3(&hm.rows), p.scale, [hm.position[0], hm.position[1], hm.position[2]]);
    let mut record = moby_render::extra_record(&model, lights.as_ref(), 0);
    moby_render::write_point_light(&mut record, moby_render::point_light_merge(&point_lights.0, sphere_centre(hm), &rows3(&hm.rows)));
    let t = Transform::from_matrix(model);
    for &e in &p.entities {
        if let Ok(mut tr) = transforms.get_mut(e) { *tr = t; }
    }
    // MobyProc's deferral (crate::shadow_render): his draws after the shadow pass only while he casts.
    let late = deferred.contains(&p.hero_id) || hm.mode & 0x800 != 0;
    if p.ratchet_late != Some(late) {
        p.ratchet_late = Some(late);
        moby_render::queue_late(&mut commands, p.entities.clone(), late);
    }
    let cam = cams.iter().next().map(|t| {
        let [fwd, left, up] = crate::game_camera::game_rows(t);
        (crate::game_camera::game_eye(t), crate::moby_lod::camera_rows(fwd, left, up))
    });
    let tans = crate::game_camera::projection_tans(projs.iter().next());
    // MobyProc's LOD pick (crate::moby_lod: low past his class's lod_trans 32, about 44 units of view depth to his
    // sphere's centre). [L] his culls and distance fade are not applied (he is always near the camera in play).
    let hm = &p.game.mobys.mobys[p.hero_id];
    let low = p.ratchet_has_low
        && cam.is_some_and(|(eye, rows)| {
            let h = &lv.mobys.classes[p.class].class.header;
            let inp = crate::moby_lod::ProcInput { position: pos3(hm), rows: rows3(&hm.rows), scale: hm.scale, draw_distance: i32::MAX, lod_trans: h.lod_trans, shine_distance: 0, alpha: 0x80 };
            let sphere = crate::moby_lod::world_sphere(&inp, crate::moby_lod::seq_sphere(class, &p.ratchet.state, h.bsphere));
            let v = crate::moby_lod::view_centre(sphere, eye, &rows);
            crate::moby_lod::moby_proc_view(v, sphere[3], &inp, tans).is_ok_and(|q| q.low_lod)
        });
    if low != p.ratchet_lod_low {
        p.ratchet_lod_low = low;
        let vis = |on: bool| if on { Visibility::Inherited } else { Visibility::Hidden };
        commands.entity(p.ratchet_lods[0]).insert(vis(!low));
        commands.entity(p.ratchet_lods[1]).insert(vis(low));
    }
    if let Some(mut buf) = buffers.get_mut(&p.extra.palette) { buf.data = Some(palette); }
    if let Some(mut buf) = buffers.get_mut(&p.extra.instances) { buf.data = Some(record); }
    upload_dynamic(&mut p, lv, cam, tans, &mut commands, &mut buffers, &mut meshes, &mut images, &mut materials, &point_lights.0, deferred);
}

/// moby+0x00 (the sphere centre, integer units) in game units; the position when the moby has no sphere.
fn sphere_centre(m: &Moby) -> [f32; 3] {
    if m.bsphere[3] == 0.0 { return pos3(m); }
    [m.bsphere[0] / 1024.0, m.bsphere[1] / 1024.0, m.bsphere[2] / 1024.0]
}

/// The mobys in the dynamic slots (module doc): per slot, drawn when live (state < 0x80), not hidden
/// (mode & 0x81) and passing MobyProc's draw-distance / near / frustum culls (`moby_lod::moby_proc` on the
/// sequence sphere); its record (model, light block for its rows / light word / ambient) and palette are
/// rewritten, its (slot, class) entities spawned on first use and shown, the others hidden. The ship's slot
/// is drawn by the static path.
#[allow(clippy::too_many_arguments)]
fn upload_dynamic(
    p: &mut Play,
    lv: &crate::level_load::LoadedLevel,
    cam: Option<(Vec3, [Vec3; 3])>,
    tans: (f32, f32),
    commands: &mut Commands,
    buffers: &mut Assets<ShaderBuffer>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    point_lights: &[rc_game::point_lights::PointLight],
    deferred: &std::collections::HashSet<usize>,
) {
    let m = &lv.mobys;
    let table = &p.game.mobys;
    let first = table.first_dynamic;
    let d = &mut p.dynamic;
    let ship = p.ship.map(|s| s.0);
    let (mut rec_changed, mut pal_changed) = (false, false);
    let (mut live, mut drawn) = (0, 0);
    let pal_bytes = d.pal_slots as usize * 64;
    for slot in 0..d.shown.len() {
        let id = first + slot;
        let Some(mo) = table.mobys.get(id) else { break };
        if Some(id) == ship { continue; }
        let ci = (mo.state < 0x80).then(|| d.class_ix.get(&mo.o_class).copied()).flatten();
        if ci.is_some() { live += 1; }
        // MobyProc: the culls, then the vertex alpha (fade × +0x23), the blend (moby_render::MobyBlend) and the shine
        // gate (+0x73) with its sphere-map basis.
        let pick = ci.and_then(|ci| {
            if mo.mode & 0x81 != 0 { return None; }
            let Some((eye, rows)) = cam else { return Some((ci, mo.alpha, false, 0, [[0.0; 3]; 3])) };
            let c = &m.classes[ci].class.header;
            let inp = crate::moby_lod::ProcInput {
                position: pos3(mo),
                rows: rows3(&mo.rows),
                scale: mo.scale,
                draw_distance: mo.draw_dist as i32,
                lod_trans: c.lod_trans,
                shine_distance: mo.b73,
                alpha: mo.alpha,
            };
            let sphere = crate::moby_lod::world_sphere(&inp, crate::moby_lod::seq_sphere(&m.anim[ci], &mo.anim, c.bsphere));
            let v = crate::moby_lod::view_centre(sphere, eye, &rows);
            crate::moby_lod::moby_proc_view(v, sphere[3], &inp, tans).ok().map(|p| {
                let e = if p.shine > 0 { crate::moby_lod::shine_basis(sphere, eye, &rows, &inp.rows) } else { [[0.0; 3]; 3] };
                (ci, p.alpha, p.fading, p.shine, e)
            })
        });
        let ci = pick.map(|p| p.0);
        d.visible[slot] = ci.is_some() as u8;
        d.shown[slot] = ci;
        let model = moby_render::extra_model(rows3(&mo.rows), mo.scale, pos3(mo));
        let look = pick.map(|(ci, alpha, fading, shine, e)| (&m.classes[ci], moby_render::SlotLook { model, alpha, fading, mode: mo.mode, glow: mo.glow, shine, e, late: deferred.contains(&id) || mo.mode & 0x800 != 0 }));
        d.extra.show_slot(commands, lv, slot as u32, look, meshes, images, materials, buffers);
        let Some(ci) = ci else { continue };
        drawn += 1;
        let rows = rows_bits(&mo.rows);
        let lights = m.lighting.as_ref().map(|l| light::moby_lights(&rows, &l.bank, mo.light, [mo.ambient[0], mo.ambient[1], mo.ambient[2]], 0x80));
        let mut rec = moby_render::extra_record(&model, lights.as_ref(), slot as u32 * d.pal_slots);
        moby_render::write_point_light(&mut rec, moby_render::point_light_merge(point_lights, sphere_centre(mo), &rows3(&mo.rows)));
        // The pilot flame's class texture scroll (rc_game::moby_update::classes::pyro_glow, +0x04) [L: kept per moby; the
        // game's is the class's, stepped by each flame].
        if mo.o_class == rc_game::moby_update::classes::pyro_glow::CLASS {
            let o = rc_game::hero::pyrocitor::glow_pv::SCROLL;
            if let Some(b) = mo.pvars.get(o..o + 4) { moby_render::set_uv_scroll(&mut rec, 0, i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i16); }
        }
        let at = slot * moby_render::EXTRA_RECORD_SIZE;
        if d.records[at..at + rec.len()] != rec[..] {
            d.records[at..at + rec.len()].copy_from_slice(&rec);
            rec_changed = true;
        }
        let snap = p.svc.snapshots.get(id).and_then(|s| s.as_ref());
        let f = moby_anim::evaluate_posed(&m.anim[ci], &mo.anim, snap, &[], &mo.joint_mods);
        let bytes: Vec<u8> = f.iter().take(d.pal_slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).collect();
        let at = slot * pal_bytes;
        if d.palette[at..at + bytes.len()] != bytes[..] {
            d.palette[at..at + bytes.len()].copy_from_slice(&bytes);
            pal_changed = true;
        }
    }
    (d.live, d.drawn) = (live, drawn);
    if rec_changed {
        if let Some(mut buf) = buffers.get_mut(&d.extra.instances) { buf.data = Some(d.records.clone()); }
    }
    if pal_changed {
        if let Some(mut buf) = buffers.get_mut(&d.extra.palette) { buf.data = Some(d.palette.clone()); }
    }
}

/// +0x31 after drawing (MobyProc writes it every frame): 1 for a moby drawn this frame, 0 on every skip
/// (occlusion, the draw-distance / near / frustum culls, hidden by its update, spawn-hidden, no geometry).
/// The moby loop of the next tick reads it (`fun_0020d868`'s activity rule).
fn write_visible(play: Option<ResMut<Play>>, occl: Option<Res<MobyOcclusion>>, anim: Option<Res<MobyAnim>>) {
    let (Some(mut p), Some(occl)) = (play, occl) else { return };
    let p = &mut *p;
    let hidden = |ii: usize| -> bool { occl.anim_index(ii).zip(anim.as_ref()).is_some_and(|(k, a)| a.hidden[k]) };
    let drawn = |ii: usize| -> u8 { (occl.anim_index(ii).is_some() && occl.drawn(ii) && !hidden(ii)) as u8 };
    let t = &mut p.game.mobys;
    let first = t.first_dynamic;
    for (id, m) in t.mobys[..first].iter_mut().enumerate() { m.visible = p.moby_to_instance.get(id).map_or(0, |&ii| drawn(ii)); }
    // Ratchet is drawn by his extra instance.
    t.mobys[p.hero_id].visible = 1;
    for (slot, &v) in p.dynamic.visible.iter().enumerate() {
        if let Some(m) = t.mobys.get_mut(first + slot) { m.visible = v; }
    }
    if let Some((id, ii)) = p.ship { t.mobys[id].visible = drawn(ii); }
}
