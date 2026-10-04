//! One gameplay tick in the game's order (level01 `FUN_002aba68`, called from the main loop after
//! `UpdatePad`): pad → mobys → hero `0x228870` → particles → camera `0x20eca8` → tick counter `0x15f5cc`++.
//! Spec: `docs/plan/player_controller.md` §0.
//!
//! The moby updates and the particles are callbacks (their ports live elsewhere); the free-slot pass
//! 0x263300 runs before them as in the game. Point lights and the help system are not modelled.
//!
//! **One `rand` stream** ([`Game::rng`], `srand(1234)` at level init): the moby hook gets it (the scheduler's
//! classes and the external updates draw from it in run order), then the hero, then the particle hook
//! (`UpdateParts`); the camera draws nothing; then the sound step; the render phase (sky stars) draws after the
//! counter increment.
//!
//! **Moby collision** ([`MobySystem`], the moby system's state outside the table: grid, class blobs, pose cache,
//! hit log): the hero's queries read the table as the moby loop left it ([`Env::mobys`], Ratchet ignored); the
//! write-back's `MobyBuildMatrix(Ratchet)` (HeroSyncMoby 0x229f20) rebuilds his bounding sphere and grid cells;
//! the camera's queries read the table after that ([`CamInput::mobys`]) and a crate blocking its line gets a hit.
//!
//! **Catch-up rule** (main loop `entry` 0x259c40): after a rendered frame, if RCNT1 counted more than one
//! 60 Hz field (0x2580), the game runs one extra `UpdatePad` + gameplay tick — at most one. The caller
//! decides how many ticks to run per rendered frame; [`ticks_for_frame`] is the game's rule.
//!
//! **Sound** ([`Game::tick_with_sound`]): the sound step runs after the camera and before the counter
//! increment on the same stream (the occlusion origin's 3 draws every tick, the pitch bends of new plays);
//! the moby loop's class sounds reach the audio layer through its `SoundSink` during the moby hook.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use crate::follow_camera::{CamInput, Camera, CameraOptions, CameraView};
use crate::hero::items::{items_update, HitSink, ItemData, ItemEnv, ItemGlobals, NoHits};
use crate::hero::{hero_update_with_sounds, AnimCtl, Env, Hero, HeroSounds, HeroTick, NoHeroSounds};
use crate::collision_query::OwnedScene;
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::services::{HitRecord, HitTemplate};
use crate::pad::PadState;
use crate::ps2v::Pf;
use crate::rng::Rng;
use rc_formats::collision::Collision;

/// RCNT1 counts in one NTSC field (the main loop's catch-up threshold).
pub const FIELD_RCNT1: u32 = 0x2580;

/// The main loop's rule: 2 ticks when the frame took longer than one field (RCNT1 > 0x2580), else 1.
pub fn ticks_for_frame(rcnt1_elapsed: u32) -> u32 { if rcnt1_elapsed > FIELD_RCNT1 { 2 } else { 1 } }

/// Game-wide options the tick reads.
#[derive(Clone, Copy, Debug, Default)]
pub struct GameOptions {
    /// 0x15edb4: mirrored controls.
    pub mirror: bool,
    pub camera: CameraOptions,
    /// The cheat bytes 0x15edb0 (`crate::cheats`); the hero reads them as [`Hero::cheats`].
    pub cheats: crate::cheats::Cheats,
    /// 0x15eea0 ‖ 0x15ee20: the game beaten or completed (the cheat entry `0x2285a0` runs).
    pub cheat_entry: bool,
    /// **Port-only**: the Port Options strafe (`crate::hero::strafe`; off by default).
    pub strafe: bool,
}

/// The gameplay state one tick advances.
pub struct Game {
    pub pad: PadState,
    pub mobys: MobyTable,
    /// Ratchet's moby in [`Game::mobys`] (`+0xa6 == 0`).
    pub hero_moby: MobyId,
    pub hero: Hero,
    pub camera: Camera,
    pub rng: Rng,
    /// `0x15f5cc`: gameplay tick counter.
    pub counter: u64,
    pub options: GameOptions,
    /// `0x15f638`: the level's death height (gameplay header +0x28).
    pub death_z: Pf,
    /// The item definitions and classes of the hand items (None: no hand item is created, as before the
    /// melee port).
    pub item_data: Option<ItemData>,
    /// The hand-swap globals outside the hero block (the engine syncs them with the saved game / session).
    pub item_globals: ItemGlobals,
    /// The tick in its mode-2 form (the in-engine scene frame, `CutsceneModeUpdate` 0x2aca80): the moby loop, the
    /// hero, the particles, the sound step and the counter run, but never `CameraUpdate` 0x20eca8 (and its crate hit),
    /// whose springs resume where they were on the first mode-0 tick, nor the free-slot pass `MobyFreeSlotBookkeeping`
    /// 0x263300 (only `InLevelFrameUpdate` runs it).
    pub camera_paused: bool,
    /// The level's grind paths (gameplay section 0x74) the hero rides (`hero::boots`; empty: no rails).
    pub grind_paths: std::sync::Arc<Vec<rc_formats::volumes::GrindPath>>,
    /// The cheat entry's move patterns 0x179b80 (`crate::cheats::CheatTables::patterns`; empty: none match).
    pub cheat_patterns: std::sync::Arc<Vec<Vec<u8>>>,
    /// The cheat the entry `0x2285a0` toggled this tick (its byte already flipped in [`GameOptions::cheats`]); the engine
    /// writes 0x15edb0 / 0x15edc0 and shows the banner.
    pub cheat_toggled: Option<crate::cheats::Toggled>,
}

/// The moby hook: `(table, hero, rng, camera, collision, counter)`.
pub type MobyHook<'a> = dyn FnMut(&mut MobyTable, &Hero, &mut Rng, &CameraView, &Collision, u64) + 'a;

/// The particle hook: `(hero, camera, rng, counter)`.
pub type PartsHook<'a> = dyn FnMut(&Hero, &CameraView, &mut Rng, u64) + 'a;

/// The sound step (`sound_update` 0x2a0638 and the class sounds of the tick, e.g.
/// `audio::class_sounds::sound_step`): `(table, hero, camera, rng, counter)` with this tick's camera, after
/// the camera update and before the counter increment.
pub type SoundHook<'a> = dyn FnMut(&MobyTable, &Hero, &CameraView, &mut Rng, u64) + 'a;

/// The moby system's state the hero and the camera reach outside the table (`moby_update::Services`: the grid
/// 0x19bc60, the class collision blobs, the pose cache and snapshots, the hit log), e.g.
/// `moby_update::services::SharedServices`.
pub trait MobySystem {
    /// What the collision kernels read of `table` now (a snapshot; None: no moby collision, world only).
    fn scene(&mut self, table: &MobyTable) -> Option<OwnedScene>;
    /// `MobyBuildMatrix` 0x265bd8 on moby `id`: matrix, bounding sphere, grid re-registration.
    fn build_matrix(&mut self, table: &mut MobyTable, id: MobyId);
    /// `FUN_0026e968(target, tmpl)`: deliver a hit.
    fn deliver_hit(&mut self, table: &mut MobyTable, target: MobyId, tmpl: &HitTemplate);
    /// The level's water-height tables for the hero's ground probe (`0x26ed38`: the class-751 ripple patches;
    /// None: the water faces' own heights).
    fn water(&self) -> Option<&dyn crate::hero::swim::WaterQuery> { None }
    /// The hit message of moby `target` (`MobyGetHitMessage`'s lookup: the record of slot `+0xa4` when it is
    /// for `target`; None: no record). The hero's hit intake reads Ratchet's (docs/plan/hero_states.md P2).
    fn hit_message(&self, _table: &MobyTable, _target: MobyId) -> Option<HitRecord> { None }
    /// The hero-block fields the moby loop's class updates wrote this tick (`moby_update::services::HeroFields`;
    /// None: none), applied to the hero right after the moby loop.
    fn take_hero_writes(&mut self) -> Option<crate::moby_update::services::HeroFields> { None }
    /// The points of the level's spline `i` (`0x1b0930[i]`; None: none), for the hero's spline wall
    /// (`Hero::wall_spline`).
    fn spline(&self, _i: usize) -> Option<Vec<[f32; 4]>> { None }
    /// The camera shake requests the moby loop's class updates made this tick (`World::shake_camera`), in order;
    /// the tick stores them into the camera's shake records right after the moby loop.
    fn take_camera_shakes(&mut self) -> Vec<crate::follow_camera::ShakeRequest> { Vec::new() }
    /// The cinematic camera calls the moby loop made this tick (`CameraScript` / `CameraScript2` …,
    /// [`crate::cinematic`]), in order; the tick applies them to the camera right after the moby loop.
    fn take_cinematic(&mut self) -> Vec<crate::cinematic::CinematicCall> { Vec::new() }
    /// The moby loop's run list for the camera at `camera` (the `0x15ffe4` chain the hero's Swingshot target
    /// searches walk: `moby_update::scheduler::build_active_list`); None: every live moby in table order.
    fn run_list(&self, _table: &MobyTable, _camera: crate::hero::physics::V4) -> Option<Vec<MobyId>> { None }
    /// The level's volume sections (the cuboids the Swingshot targets' records name); None: none.
    fn volumes(&self) -> Option<std::sync::Arc<rc_formats::volumes::Volumes>> { None }
    /// The moby ids of group `g` (`0x1abcc0[g]`, list order; none for `g < 0`): the Swingshot camera's group look.
    fn group(&self, _g: i8) -> Vec<MobyId> { Vec::new() }
    /// `CreateMoby(o_class)` 0x263390 outside the moby loop (the camera moby 1007, `crate::follow_camera::camera_moby`);
    /// None: no slot, no class, or no moby world.
    fn create_moby(&mut self, _table: &mut MobyTable, _o_class: i16, _counter: u64) -> Option<MobyId> { None }
    /// `DeleteMoby` 0x2636c0 outside the moby loop (the camera moby).
    fn delete_moby(&mut self, _table: &mut MobyTable, _id: MobyId, _counter: u64) {}
    /// The game mode 0x15f5c4 as the moby loop left it (the hero code's tests of it: the disguise's timer in
    /// `HeroTickStateTimer`, `FUN_00230770`; `crate::hero::hologuise`). 0: gameplay.
    fn game_mode(&self) -> i32 { 0 }
    /// The Hoverboard's view of the world around board `board` (`crate::hero::hoverboard::BoardWorld`; None: none).
    fn board_world(&self, _table: &MobyTable, _board: MobyId) -> Option<crate::hero::hoverboard::BoardWorld> { None }
    /// The Hoverboard hero code's stores (`crate::hero::hoverboard::BoardCmd`), applied by the next moby loop.
    fn queue_board(&mut self, _cmds: Vec<crate::hero::hoverboard::BoardCmd>) {}
    /// The global fade 0x15f3fc (the moby loop's `cinematic::Cinematic::fade`) and its store.
    fn fade(&self) -> f32 { 0.0 }
    fn set_fade(&mut self, _v: f32) {}
}

/// Callbacks for the subsystems ported elsewhere.
pub struct TickHooks<'a> {
    /// The moby update loop (0x279470 …, `moby_update::Scheduler::tick`), after the free-slot pass: the table,
    /// the hero block (last tick's), the game's one `rand` stream, the camera as the previous tick's camera
    /// update left it (0x167240 …), the level collision and the tick counter.
    pub mobys: &'a mut MobyHook<'a>,
    /// `UpdateParts` (particles), between the hero and the camera: the hero, the camera as the previous tick's
    /// camera update left it (0x167240, read by the type-11 sparks), the game's `rand` stream, the tick counter.
    pub particles: &'a mut PartsHook<'a>,
    /// The moby collision for the hero and the camera (None: their queries are world only, Ratchet's grid
    /// registration stays as loaded, the camera hits nothing).
    pub world: Option<&'a mut dyn MobySystem>,
}

/// What one tick did.
#[derive(Clone, Copy, Debug)]
pub struct TickReport {
    pub hero: HeroTick,
    pub camera: CameraView,
    /// The 30-tick blocked reset of the camera fired this tick.
    pub camera_reset: bool,
}

impl Game {
    /// A game on `coll` in the loader's order (`InitLevelRenderGlobals` 0x255958): `srand(1234)` (its line
    /// 79, before the instance loop), then the hero init `HeroInit` 0x226b70 on its moby (index `hero_moby` in
    /// `mobys`, ground-snapped by [`Hero::init_from_moby`], which takes the stream's first draw for the fidget
    /// timer), and the camera snapped behind it. The tick counter is 0 until [`Game::finish_load`].
    pub fn new(coll: &Collision, mobys: MobyTable, hero_moby: MobyId, options: GameOptions, death_z: f32) -> Game {
        let mut rng = Rng::new();
        rng.srand(crate::rng::LEVEL_SEED);
        Game::with_rng(coll, mobys, hero_moby, options, death_z, rng)
    }

    /// [`Game::new`] continuing the stream `rng` (a respawn's hero init draws from the running stream).
    pub fn with_rng(coll: &Collision, mut mobys: MobyTable, hero_moby: MobyId, options: GameOptions, death_z: f32, mut rng: Rng) -> Game {
        let hero = Hero::init_from_moby(&mut mobys.mobys[hero_moby], coll, &mut rng);
        let pad = PadState::default();
        let camera = Camera::new(&CamInput { hero: &hero, pad: &pad, coll, mobys: None, hero_moby: None }, options.camera);
        Game { pad, mobys, hero_moby, hero, camera, rng, counter: 0, options, death_z: Pf::f(death_z), item_data: None, item_globals: ItemGlobals::default(), camera_paused: false, grind_paths: Default::default(), cheat_patterns: Default::default(), cheat_toggled: None }
    }

    /// The end of the level load (`LoadLevelCoreData` 0x258128): `0x15f5cc++` right after
    /// `MobyLoadTimeUpdatePass`, so the load pass runs at counter 0 and the first gameplay tick sees 1. Call it
    /// once, after the caller's `Scheduler::load_pass`.
    pub fn finish_load(&mut self) { self.counter += 1; }

    /// One gameplay tick: `UpdatePad` with `pad_data` (None = disconnected), then the tick body. The hero's
    /// attacks hit nothing ([`Game::tick_with_hits`] takes a sink).
    pub fn tick(&mut self, pad_data: Option<&[u8]>, coll: &Collision, anim: &mut dyn AnimCtl, hooks: &mut TickHooks) -> TickReport {
        self.tick_with_hits(pad_data, coll, anim, hooks, &mut NoHits)
    }

    /// [`Game::tick`] with the hit sink the hero's attacks (the wrench) deliver to.
    pub fn tick_with_hits(&mut self, pad_data: Option<&[u8]>, coll: &Collision, anim: &mut dyn AnimCtl, hooks: &mut TickHooks, hits: &mut dyn HitSink) -> TickReport {
        self.tick_with_sound(pad_data, coll, anim, hooks, hits, None)
    }

    /// [`Game::tick_with_hits`] with the sound step `sound` (None: no sound layer) after the camera and before
    /// the counter increment, in game modes 0 and 2 alike (`sound_update` runs in both). The hero's own sounds
    /// are not played ([`Game::tick_with_hero_sounds`]).
    pub fn tick_with_sound(
        &mut self,
        pad_data: Option<&[u8]>,
        coll: &Collision,
        anim: &mut dyn AnimCtl,
        hooks: &mut TickHooks,
        hits: &mut dyn HitSink,
        sound: Option<&mut SoundHook>,
    ) -> TickReport {
        self.tick_with_hero_sounds(pad_data, coll, anim, hooks, hits, sound, &mut NoHeroSounds)
    }

    /// [`Game::tick_with_sound`] with the hero's sounds (`hero_update_with_sounds`: the class sound of Ratchet's
    /// animation trigger right after his advance, the hurt / death voices after the transitions) played at
    /// their points inside the hero update through `hero_sounds`.
    #[allow(clippy::too_many_arguments)]
    pub fn tick_with_hero_sounds(
        &mut self,
        pad_data: Option<&[u8]>,
        coll: &Collision,
        anim: &mut dyn AnimCtl,
        hooks: &mut TickHooks,
        hits: &mut dyn HitSink,
        sound: Option<&mut SoundHook>,
        hero_sounds: &mut dyn HeroSounds,
    ) -> TickReport {
        self.pad.update(pad_data, self.options.mirror);
        self.hero.cheats = self.options.cheats;
        self.cheat_toggled = None;
        if !self.camera_paused { self.mobys.free_slot_pass(self.counter); }
        // The globals the moby loop reads outside the moby system: this tick's pad, the last camera update's Euler,
        // Ratchet's anim fields after his last update (moby_update::services::LoopGlobals).
        let e = self.camera.out.euler;
        self.hero.loop_in = crate::moby_update::services::LoopGlobals {
            pad: self.pad.clone(),
            cam_euler: [e[0].to_f32(), e[1].to_f32(), e[2].to_f32()],
            anim: anim.view(),
            cam_pos: self.camera.current_pos(),
        };
        (hooks.mobys)(&mut self.mobys, &self.hero, &mut self.rng, &self.camera.out, coll, self.counter);
        // The classes' stores into the hero block (the flow 679's push, the lift's lockouts …) land before the hero
        // update, as in the game (moby_update::services::HeroFields).
        let hero_writes = hooks.world.as_deref_mut().and_then(|w| w.take_hero_writes());
        if let Some(f) = &hero_writes { f.apply(&mut self.hero); }
        // A class's store of the death height 0x15f638 (Kalebo's race host 1455).
        if let Some(z) = hero_writes.as_ref().and_then(|f| f.death_z) { self.death_z = Pf::f(z); }
        // Their grind path radius stores (Veldin's rail chooser 582): the hero's rails.
        for (i, r) in hero_writes.iter().flat_map(|f| f.rail_radius.iter().flatten()) {
            if let Some(g) = std::sync::Arc::make_mut(&mut self.grind_paths).get_mut(*i as usize) { g.bsphere[3] = *r; }
        }
        // Their camera shakes (stores into 0x167260 / 0x167270; the camera update at the end of the tick applies them).
        for r in hooks.world.as_deref_mut().map(|w| w.take_camera_shakes()).unwrap_or_default() { self.camera.request_shake(r); }
        // 0x15f5c4 for the hero code that tests it (the disguise's timer and gate: crate::hero::hologuise).
        self.hero.gadgets.game_mode = hooks.world.as_deref().map_or(0, |w| w.game_mode());
        // `0x1413d0`: the hero moby (Ratchet's, or the body moby while a body is in: crate::hero::bodies).
        let hero_moby = Some(self.hero.hero_moby(self.hero_moby));
        // Their cinematic camera calls (CameraScript, CameraScript2, HeroTeleport's camera reset: crate::cinematic).
        let cine = hooks.world.as_deref_mut().map(|w| w.take_cinematic()).unwrap_or_default();
        crate::cinematic::apply_camera_calls(&mut self.camera, &cine, &CamInput { hero: &self.hero, pad: &self.pad, coll, mobys: None, hero_moby });
        // Ratchet's hit message as the moby loop left it (the hit intake 0x231580 and the hurt entries read it).
        self.hero.damage.hit = hooks.world.as_deref().and_then(|w| w.hit_message(&self.mobys, self.hero.hero_moby(self.hero_moby))).map(|r| hero_hit(&self.mobys, &r));
        // The hero's queries see the table as the moby loop left it (a snapshot: the hero holds its own moby).
        let scene = hooks.world.as_deref_mut().and_then(|w| w.scene(&self.mobys));
        // The carriers' platform blocks as the moby loop left them (`HeroPlatformUpdate`, hero::platform).
        let mut carriers = crate::hero::platform::Carriers::collect(&self.mobys, self.hero.hero_moby(self.hero_moby));
        carriers.grind = self.grind_paths.clone();
        // The Swingshot targets of the moby loop's run list (hero::swingshot, the weapon check's searches).
        carriers.targets = self.swing_targets(hooks.world.as_deref());
        // The target list 0x1abe80 for the melee aim search (hero::melee::aim_search) and the head's look target
        // (`HeroScanTargets`, hero::pose: every tick, with or without a hand item).
        carriers.melee = crate::hero::melee::melee_targets(&self.mobys, &self.target_list(hooks.world.as_deref()));
        // The ground moby's class and pose (the Hydrodisplacer's poses 0x38..0x3a read the pad's: hero::hydrodisplacer).
        carriers.ground = self.hero.ground_moby.and_then(|g| self.mobys.mobys.get(g).map(|m| (g, m.o_class, [m.position[0], m.position[1], m.position[2]], m.rotation[2])));
        // The Hoverboard's board, its paths, racers and pickups (hero::hoverboard).
        carriers.board = self.hero.board.moby.and_then(|b| hooks.world.as_deref().and_then(|w| w.board_world(&self.mobys, b)));
        // The spline a class named for the capsule pass's spline wall (Pokitaru's 361: hero::physics::spline_wall).
        let wall = self.hero.wall_spline as u16 as usize;
        carriers.wall = (wall != 0).then(|| hooks.world.as_deref().and_then(|w| w.spline(wall)).map(|p| (wall, p))).flatten();
        // The weapon's target (0x13fda0) where the moby loop left it (SetState 0x23 aims at it).
        crate::hero::weapons::refresh_aim(&mut self.hero, &self.mobys);
        // Port-only: the Port Options strafe (crate::hero::strafe): while it owns L2 / R2 the hero sees the pad without
        // them (the item updates keep the whole pad) and reads the strafe as `Hero::strafe`.
        self.hero.strafe_mode = self.options.strafe;
        let strafe_pad = crate::hero::strafe::prepare(&mut self.hero, &self.pad);
        let hero_pad = strafe_pad.as_ref().unwrap_or(&self.pad);
        let hero_tick = {
            let mobys = scene.as_ref().map(OwnedScene::scene);
            let view = self.camera.out;
            let env = Env {
                coll,
                pad: hero_pad,
                cam_yaw: view.yaw(),
                cam_rows: view.rows,
                mirror: self.options.mirror,
                death_z: self.death_z,
                mobys: mobys.as_ref(),
                hero_moby,
                water: hooks.world.as_deref().and_then(|w| w.water()),
                world: Some(&carriers),
            };
            // The classes' calls into the hero code (SetState / SetAnim: the bolt crank; SwitchCharacter / leaving a body:
            // crate::hero::bodies), with the hero's context.
            // A `SwitchCharacter`'s item slots' pass `0x231088` (crate::hero::items::slot_pass) with the item environment.
            {
                let switching = self.hero.bodies.restore.is_some() || hero_writes.as_ref().is_some_and(|f| f.calls.iter().flatten().any(|c| matches!(c, crate::moby_update::services::HeroCall::SwitchCharacter { .. })));
                let targets = if switching && self.hero.items.slot.item.is_some() { self.target_list(hooks.world.as_deref()) } else { Vec::new() };
                let cam = (self.camera.out.pos_f32(), self.camera.out.rows_f32());
                let (data, pad, counter, ratchet) = (self.item_data.as_ref(), &self.pad, self.counter, self.hero_moby);
                let (globals, mobys) = (&mut self.item_globals, &mut self.mobys);
                let mut pass = |h: &mut crate::hero::Hero, a: &mut dyn AnimCtl, r: &mut crate::rng::Rng| {
                    let Some(data) = data else { return };
                    let ienv = ItemEnv { data, pad, frame: counter as i32, hero_moby: ratchet, coll: Some(coll), camera: Some((cam.0, cam.1[0])), camera_up: Some(cam.1[2]), targets: &targets };
                    crate::hero::items::slot_pass(h, globals, mobys, &*a, r, &ienv, &mut *hits);
                };
                // The death reload's switch back into the checkpoint's body (`0x29adc8`, crate::hero::bodies).
                if let Some((m, st, b)) = self.hero.bodies.restore.take() {
                    crate::hero::bodies::switch_character_with(&mut self.hero, &mut crate::hero::states::Ctx { env: &env, anim: &mut *anim, rng: &mut self.rng, voice: None }, m, st, b, Some(&mut pass));
                }
                if let Some(f) = &hero_writes { f.run_calls_with(&mut self.hero, &mut crate::hero::states::Ctx { env: &env, anim: &mut *anim, rng: &mut self.rng, voice: None }, Some(&mut pass)); }
            }
            crate::cinematic::run_hero_calls(&mut self.hero, &cine, &mut crate::hero::states::Ctx { env: &env, anim: &mut *anim, rng: &mut self.rng, voice: None });
            // A switch made the body the hero moby: its commands to the table (Ratchet hidden, the body's mode bits),
            // and the hit message the intake reads is the new hero moby's.
            if !self.hero.bodies.cmds.is_empty() {
                crate::hero::bodies::apply_cmds(&mut self.hero, &mut self.mobys, self.hero_moby, hits, self.counter);
                let now = self.hero.hero_moby(self.hero_moby);
                if Some(now) != hero_moby {
                    self.hero.damage.hit = hooks.world.as_deref().and_then(|w| w.hit_message(&self.mobys, now)).map(|r| hero_hit(&self.mobys, &r));
                }
            }
            // 0x141660 / 0x141408 for the hero code outside the slot loop (the Hologuise's weapon check, the disguise's way
            // out: crate::hero::hologuise); a change of the request is written back after the hero update.
            (self.hero.gadgets.hand_saved, self.hero.gadgets.hand_request) = (self.item_globals.saved, self.item_globals.request);
            let moby = &mut self.mobys.mobys[self.hero.hero_moby(self.hero_moby)];
            hero_update_with_sounds(&mut self.hero, moby, &env, anim, &mut self.rng, hero_sounds)
        };
        let hm = self.hero.hero_moby(self.hero_moby);
        let hero_moby = Some(hm);
        drop(scene);
        // The hand request as the hero update left it (the disguise's update clears 0x141408: crate::hero::hologuise).
        if self.hero.gadgets.hand_request != self.item_globals.request { self.item_globals.request = self.hero.gadgets.hand_request; }
        // The mobys the hero update created (CreateMoby inside it: the water splash 775), before anything else can take a
        // slot, with their MobyBuildMatrix (hero::fx::create_mobys).
        if !self.hero.fx.mobys.is_empty() {
            for id in crate::hero::fx::create_mobys(&mut self.hero, &mut self.mobys, self.hero_moby, hits, self.counter) {
                if let Some(w) = hooks.world.as_deref_mut() { w.build_matrix(&mut self.mobys, id); }
            }
        }
        // The Hoverboard: the board carried under Ratchet's feet (level05 0x24bdc0's tail) and the hero code's stores into
        // other mobys and the game state, applied by the next moby loop (hero::hoverboard).
        if let (Some((pos, rows)), Some(b)) = (self.hero.board.carry.take(), self.hero.board.moby) {
            if let Some(m) = self.mobys.mobys.get_mut(b) {
                m.position = pos;
                m.rows[..3].copy_from_slice(&rows);
            }
        }
        // The time trial's meter (the HUD element's update `0x262b58`, which is gameplay: hero::hoverboard::meter_tick).
        crate::hero::hoverboard::meter_tick(&mut self.hero);
        let board_cmds = std::mem::take(&mut self.hero.board.cmds);
        if !board_cmds.is_empty() {
            if let Some(w) = hooks.world.as_deref_mut() { w.queue_board(board_cmds); }
        }
        // The hero's camera shakes (the stomp's landing, …: its stores into 0x167260 / 0x167270 during the update).
        for r in std::mem::take(&mut self.hero.fx.shakes) { self.camera.request_shake(r); }
        // HeroSyncMoby 0x229f20 (0x22a110 in a body): the hero moby's hit slot +0xa4 = 0xff (the message is consumed by
        // this update).
        if hero_tick != HeroTick::OutOfBounds { self.mobys.mobys[hm].hit_slot = 0xff; }
        self.hero.damage.hit = None;
        if hero_tick == HeroTick::Ran {
            // The write-back's MobyBuildMatrix(hero moby) (HeroSyncMoby 0x229f20 / 0x22a110): bounding sphere from its
            // animation fields (+0x50..0x54, the moby's own in the game), matrix, grid re-registration.
            let v = anim.view();
            let a = &mut self.mobys.mobys[hm].anim;
            (a.seq_a, a.seq_b, a.frame_a, a.frame_b, a.t) = (v.seq_a, v.seq_b, v.frame_a, v.frame_b, v.t);
            if let Some(w) = hooks.world.as_deref_mut() { w.build_matrix(&mut self.mobys, hm); }
        }
        // The rest of `HeroUpdateAlt` (a body): the glow, Clank's antenna glow and rotor, Giant Clank's pilot, the
        // body's hits; and the commands its update or a class's leave made (crate::hero::bodies).
        if self.hero.mode != 0 || !self.hero.bodies.cmds.is_empty() {
            crate::hero::bodies::after_update(&mut self.hero, &mut self.mobys, self.hero_moby, &mut *anim, hits, hero_sounds, &mut self.rng, self.counter);
            if self.hero.mode != 0 {
                if let Some(w) = hooks.world.as_deref_mut() { w.build_matrix(&mut self.mobys, self.hero_moby); }
            }
        }
        // The disguise's transitions' `UpdateWrenchSelected(0)` (□ in body 3: crate::hero::bodies::disguise), with the item
        // environment, right after the body's update.
        if std::mem::take(&mut self.hero.gadgets.wrench_select) {
            if let Some(data) = self.item_data.as_ref() {
                let ienv = ItemEnv { data, pad: &self.pad, frame: self.counter as i32, hero_moby: self.hero_moby, coll: Some(coll), camera: Some((self.camera.out.pos_f32(), self.camera.out.rows_f32()[0])), camera_up: Some(self.camera.out.rows_f32()[2]), targets: &[] };
                crate::hero::items::update_hand_selected(&mut self.hero, &mut self.item_globals, &mut self.rng, &ienv);
            }
        }
        // The board weapon pickup's hand switch (crate::hero::hoverboard): 0x1413fc = 0, `UpdateWrenchSelected(0)`,
        // 0x1413fc = 1, then the request 0x24 again (the pickup's last store).
        if std::mem::take(&mut self.hero.gadgets.board_select) {
            if let Some(data) = self.item_data.as_ref() {
                let ienv = ItemEnv { data, pad: &self.pad, frame: self.counter as i32, hero_moby: self.hero_moby, coll: Some(coll), camera: Some((self.camera.out.pos_f32(), self.camera.out.rows_f32()[0])), camera_up: Some(self.camera.out.rows_f32()[2]), targets: &[] };
                self.hero.items.f13fc = 0;
                crate::hero::items::update_hand_selected(&mut self.hero, &mut self.item_globals, &mut self.rng, &ienv);
                self.hero.items.f13fc = 1;
                self.item_globals.request = crate::hero::hoverboard::WEAPON_ITEM;
            }
        }
        // Ratchet's class sounds a body state played on his moby (Clank's burn 0x7d: `PlayClassSound(9, 0, Ratchet)`).
        for (index, flags) in std::mem::take(&mut self.hero.bodies.ratchet_sounds) {
            hero_sounds.voice(&self.mobys.mobys[self.hero_moby], index, flags, &mut self.rng);
        }
        // The hits the pack states queued in their physics (the stomp's descent, the Thruster long jump's crates).
        if !self.hero.packs.hits.is_empty() { crate::hero::packs::deliver_hits(&mut self.hero, &mut self.mobys, self.hero_moby, hits); }
        // HeroItemsUpdate 0x231268 (hand slot): create, attach, the swap, the item's update (the wrench's hit). Not in a body
        // (`HeroUpdateAlt` has no item pass).
        if hero_tick == HeroTick::Ran && self.hero.mode == 0 {
            if let Some(data) = self.item_data.as_ref() {
                // 0x1abe80, the targetable mobys of the moby loop's run list (crate::targeting), for the items' aim searches.
                let targets = if self.hero.items.slot.item.is_some() { self.target_list(hooks.world.as_deref()) } else { Vec::new() };
                let ienv = ItemEnv { data, pad: &self.pad, frame: self.counter as i32, hero_moby: self.hero_moby, coll: Some(coll), camera: Some((self.camera.out.pos_f32(), self.camera.out.rows_f32()[0])), camera_up: Some(self.camera.out.rows_f32()[2]), targets: &targets };
                items_update(&mut self.hero, &mut self.item_globals, &mut self.mobys, &*anim, &mut self.rng, &ienv, hits);
                // The item updates' writes into the pad's released mask 0x13cae8 (the Suck Cannon's put-away with L1 / L2
                // held: hero::gadgets::Gadgets::released_or).
                self.pad.released |= std::mem::take(&mut self.hero.gadgets.released_or);
                // The slot loop's item update that needs the hero's context (the Swingshot's hook: SetState, the
                // collision lines), at the same point of the frame (hero::gadgets).
                if self.hero.gadgets.pending.is_some() || !self.hero.gadgets.calls.is_empty() || self.hero.swing.item.alive || self.hero.weapons.deferred.is_some() || self.hero.weapons.pending_draw || self.hero.weapons.pending_idle || self.hero.weapons.pending_anim.is_some() {
                    let scene = hooks.world.as_deref_mut().and_then(|w| w.scene(&self.mobys));
                    let mobys = scene.as_ref().map(OwnedScene::scene);
                    let view = self.camera.out;
                    // The strafe's view of the pad as it is now (the released mask above changed it).
                    let strafe_pad = crate::hero::strafe::owns_buttons(&self.hero).then(|| self.pad.without(crate::hero::strafe::BUTTONS));
                    let env = Env {
                        coll,
                        pad: strafe_pad.as_ref().unwrap_or(&self.pad),
                        cam_yaw: view.yaw(),
                        cam_rows: view.rows,
                        mirror: self.options.mirror,
                        death_z: self.death_z,
                        mobys: mobys.as_ref(),
                        hero_moby,
                        water: hooks.world.as_deref().and_then(|w| w.water()),
                        world: Some(&carriers),
                    };
                    let mut c = crate::hero::states::Ctx { env: &env, anim: &mut *anim, rng: &mut self.rng, voice: None };
                    crate::hero::gadgets::after_items(&mut self.hero, &mut c, data);
                }
                // The hand item's class sounds (the wrench's hit, the Swingshot's fire / hit / pull), right after its
                // update.
                crate::hero::gadgets::flush_item_sounds(&mut self.hero, &self.mobys.mobys[self.hero_moby], hero_sounds, &mut self.rng);
            }
            // `0x229348`: the hero's fade of 0x15f3fc (the Hoverboard's wrong-way respawn).
            if self.hero.board.fade.on {
                if let Some(w) = hooks.world.as_deref_mut() {
                    let mut v = w.fade();
                    self.hero.board.fade.step(&mut v);
                    w.set_fade(v);
                }
            }
            // `0x229158` (after HeroItemsUpdate): the Hologuise's squash on Ratchet's moby and the hand item.
            crate::hero::hologuise::squash(&mut self.hero, &mut self.mobys.mobys[self.hero_moby]);
            crate::hero::hologuise::squash_hand(&mut self.hero);
            // FUN_00227e90: the walk / run footsteps (after HeroItemsUpdate in 0x228870).
            crate::hero::fx::walk_footsteps(&mut self.hero, &self.mobys.mobys[self.hero_moby], &anim.view(), hero_sounds, &mut self.rng);
            // FUN_002285a0: the cheat entry (the last moves against the patterns; crate::cheats).
            let h = &self.hero;
            let inp = crate::cheats::MoveInput { state_ticks: h.timer, group: h.group, state: h.state, flip_sector: h.jump.kind7a0, combo: h.melee.combo, mirror: self.options.cheats.on(crate::cheats::slot::MIRROR) };
            let mut active = self.options.cheats.0;
            if let Some(t) = self.hero.cheat_moves.step(self.options.cheat_entry, &inp, &self.cheat_patterns, &mut active) {
                self.options.cheats.0 = active;
                self.hero.cheats = self.options.cheats;
                self.cheat_toggled = Some(t);
            }
        }
        // The camera calls the hand items' updates made through the moby world (the Visibomb's launch switches the
        // type-6 camera in, `0x317d88`), applied before this tick's camera update as the game's direct calls are.
        let mut cine = hooks.world.as_deref_mut().map(|w| w.take_cinematic()).unwrap_or_default();
        // A hero-side `HeroTeleport`'s camera reset (the Hoverboard's respawns: hero::Hero::teleport).
        if std::mem::take(&mut self.hero.fx.camera_reset) { cine.insert(0, crate::cinematic::CinematicCall::CameraResetBehindHero); }
        if !cine.is_empty() { crate::cinematic::apply_camera_calls(&mut self.camera, &cine, &CamInput { hero: &self.hero, pad: &self.pad, coll, mobys: None, hero_moby }); }
        (hooks.particles)(&self.hero, &self.camera.out, &mut self.rng, self.counter);
        let resets = self.camera.resets;
        if self.camera_paused {
            if let Some(f) = sound { f(&self.mobys, &self.hero, &self.camera.out, &mut self.rng, self.counter); }
            self.counter += 1;
            return TickReport { hero: hero_tick, camera: self.camera.out, camera_reset: false };
        }
        // The Swingshot camera's look along the swung-on target's moby group (`0x3182c8`, levels 14 / 7 / 9): the
        // target's group byte +0x21, position and its group's members as the camera update reads them.
        self.camera.world.swing_group = match (self.hero.state, self.hero.swing.on) {
            (0x2c, Some(on)) => self.mobys.mobys.get(on).map(|m| {
                let p3 = |p: [f32; 4]| [p[0], p[1], p[2]];
                let ids = hooks.world.as_deref().map(|w| w.group(m.group)).unwrap_or_default();
                let members = ids
                    .iter()
                    .filter_map(|&id| self.mobys.mobys.get(id).map(|x| crate::follow_camera::swing::GroupMember { id, class: x.o_class, pos: p3(x.position) }))
                    .collect();
                crate::follow_camera::swing::SwingGroup { group: m.group, on_pos: p3(m.position), members }
            }),
            _ => None,
        };
        // The follow camera's focus scan candidates (`0x3111d8`): mobys whose target record's byte +0x0d is set.
        let me = self.hero_moby;
        self.camera.world.focus = self
            .mobys
            .mobys
            .iter()
            .enumerate()
            .filter(|&(id, m)| id != me && crate::moby_update::triggers::pvar_record(m).is_some_and(|o| m.pvars[o + 0xd] != 0))
            .map(|(id, _)| id)
            .collect();
        // The class-18 regions' mobys and groups (`0x2fb9c8`: their moby or their group's first live member).
        {
            let (mut mobys, mut groups) = (std::collections::BTreeMap::new(), std::collections::BTreeMap::new());
            let mut add = |id: usize| {
                if let Some(m) = self.mobys.mobys.get(id) {
                    mobys.insert(id, crate::follow_camera::focus::CamMoby { state: m.state, pos: [m.position[0], m.position[1], m.position[2]] });
                }
            };
            for f in self.camera.level_cams.slots.iter().filter_map(|s| s.focus) {
                if f.group < 0 {
                    if let Ok(id) = usize::try_from(f.moby) { add(id); }
                } else if let Ok(g) = i8::try_from(f.group) {
                    let ids = hooks.world.as_deref().map(|w| w.group(g)).unwrap_or_default();
                    for &id in &ids { add(id); }
                    groups.insert(f.group, ids);
                }
            }
            // The follow camera's scripted focus moby (0x16735c, `0x3111d8`): its state and position.
            if let Some(id) = self.camera.focus_moby { add(id); }
            self.camera.world.mobys = mobys;
            self.camera.world.groups = groups;
        }
        // The camera's queries see the table after the hero's write-back (Ratchet re-registered).
        let scene = hooks.world.as_deref_mut().and_then(|w| w.scene(&self.mobys));
        let camera = {
            let mobys = scene.as_ref().map(OwnedScene::scene);
            self.camera.update(&CamInput { hero: &self.hero, pad: &self.pad, coll, mobys: mobys.as_ref(), hero_moby })
        };
        drop(scene);
        // `Camera_handleCollWithHero`'s camera moby (class 1007): created while the follow camera is current (+0x30 =
        // 0xff, mode |= 0x41, at the camera, `MobyBuildMatrix`), deleted otherwise (crate::follow_camera::camera_moby).
        if let (Some(call), Some(w)) = (self.camera.cam_moby_call.take(), hooks.world.as_deref_mut()) {
            match call {
                crate::follow_camera::camera_moby::Call::Create { pos } => {
                    let id = w.create_moby(&mut self.mobys, crate::follow_camera::camera_moby::CLASS, self.counter);
                    if let Some(id) = id {
                        let m = &mut self.mobys.mobys[id];
                        m.update_dist = 0xff;
                        m.mode |= 0x41;
                        m.position = [pos[0], pos[1], pos[2], m.position[3]];
                        w.build_matrix(&mut self.mobys, id);
                    }
                    self.camera.camera_moby_created(id);
                }
                crate::follow_camera::camera_moby::Call::Delete(id) => w.delete_moby(&mut self.mobys, id, self.counter),
            }
        }
        // The first-person camera's store of 0x1413f5 (`0x316330`; every SetState clears it).
        if self.camera.first_person_flag() { self.hero.f13f5 = 1; }
        // A crate blocking the camera line (0x312ef8): FUN_0026e808(20.0, tmpl, Ratchet, 0x800000, dir), +0x18 /
        // +0x19 = 3, +0x1a = Ratchet's class, then FUN_0026e968.
        if let (Some((id, dir)), Some(w)) = (self.camera.hit, hooks.world.as_deref_mut()) {
            let h1a = self.mobys.mobys[self.hero_moby].o_class as u16;
            let tmpl = HitTemplate { dir, attacker: hero_moby, flags: 0x80_0000, b18: 3, b19: 3, h1a, damage: Pf::b(0x41a0_0000), w20: 1 };
            w.deliver_hit(&mut self.mobys, id, &tmpl);
        }
        // The sound step with this tick's camera (the listener 0x167240).
        if let Some(f) = sound { f(&self.mobys, &self.hero, &self.camera.out, &mut self.rng, self.counter); }
        self.counter += 1;
        TickReport { hero: hero_tick, camera, camera_reset: self.camera.resets != resets }
    }
}

impl Game {
    /// `0x1abe80` as the moby loop built it: the targetable mobys of its run list for the camera it ran with (the
    /// previous tick's), in order (`crate::targeting::target_list`); without a moby system, the table's order.
    fn target_list(&self, world: Option<&dyn MobySystem>) -> Vec<crate::moby_runtime::MobyId> {
        let order = world.and_then(|w| w.run_list(&self.mobys, self.camera.out.pos)).unwrap_or_else(|| {
            self.mobys.mobys.iter().enumerate().take_while(|(_, m)| m.state != crate::moby_runtime::state::END).map(|(i, _)| i).collect()
        });
        crate::targeting::target_list(&self.mobys, &order)
    }

    /// The Swingshot targets the hero sees this tick: the target mobys of the moby loop's run list (in its order)
    /// with their records and cuboids, and the camera as the previous tick's update left it.
    fn swing_targets(&self, world: Option<&dyn MobySystem>) -> crate::hero::swingshot::Targets {
        use crate::hero::swingshot::{Targets, PULL_CLASS, SWING_CLASS};
        let view = self.camera.out;
        let (cam, yaw, pitch) = (view.pos_f32(), view.yaw().to_f32(), view.euler[1].to_f32());
        if !self.mobys.mobys.iter().any(|m| m.o_class == PULL_CLASS || m.o_class == SWING_CLASS) { return Targets { camera: cam, cam_yaw: yaw, cam_pitch: pitch, ..Targets::default() }; }
        let order = world.and_then(|w| w.run_list(&self.mobys, view.pos)).unwrap_or_else(|| {
            self.mobys.mobys.iter().enumerate().take_while(|(_, m)| m.state != crate::moby_runtime::state::END).map(|(i, _)| i).collect()
        });
        let volumes = world.and_then(|w| w.volumes());
        Targets::collect(&self.mobys, &order, volumes.as_deref(), cam, yaw, pitch)
    }
}

/// A hit record for the hero, with what the hero code reads of the attacker (class +0xa6, position +0x10,
/// mission +0xb0) out of the table.
fn hero_hit(table: &MobyTable, r: &HitRecord) -> crate::hero::damage::HeroHit {
    let attacker = r.attacker.and_then(|id| table.mobys.get(id).map(|m| crate::hero::damage::Attacker {
        id,
        o_class: m.o_class,
        pos: [m.position[0], m.position[1], m.position[2]],
        mission: m.mission,
    }));
    crate::hero::damage::HeroHit {
        attacker,
        flags: r.flags,
        b28: r.b28,
        damage: r.damage.to_f32(),
        w30: r.w30,
        dir: [r.dir[0].to_f32(), r.dir[1].to_f32(), r.dir[2].to_f32(), r.dir[3].to_f32()],
    }
}
