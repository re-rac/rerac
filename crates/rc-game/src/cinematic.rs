//! The in-level cinematic layer: the calls the moby classes make into the camera, the hero and the scene / movie
//! players while they stage a cutaway (docs/plan/cutscenes.md §3–§5). The game has **no script system** for these:
//! each class calls the same few engine functions directly from its update:
//!
//! | game function | here | effect |
//! |---|---|---|
//! | `CameraScript(pos, euler, mode, ticks, collide)` 0x316ef8 | [`camera_script`] | the script camera (type 5) switched in ([`crate::follow_camera::script`]) |
//! | `0x316dd0` / `0x316e28` | [`camera_targets`] | its target position / Euler |
//! | `0x316e88(a, b)` | [`camera_curve`] | mode 3's distance curve |
//! | level02 `0x2f8a18(mode, ticks)` | [`script_mode`] | the script camera's mode and timer (the launch tube's ride) |
//! | `CameraScript2(kind)` 0x317070 | [`camera_script2`] | back to the follow camera (cut or blend) |
//! | `0x15f404 = 1 / 0` | [`letterbox`] | the letterbox bars (`DrawScreenFade` 0x21b7d8) and the HUD hidden (`HudDraw` 0x24fb50); the creature layer's `Globals::cutscene` *is* this global |
//! | `SetState(s, play)` 0x23cf98 | [`hero_state`] | Ratchet's state (0x72: held, no control) |
//! | `HeroTeleport(pos, euler, state, reset_cam)` 0x2368e0 | [`hero_teleport`] | Ratchet placed, motion cleared, state set |
//! | `DialogStreamStart(k)` 0x2ac330 | [`start_scene`] | a mode-2 scene (the engine's scene player) |
//! | `DialogStreamUpdate(n)` 0x2acf50 | [`start_movie`] | an in-level PSS movie (`StartPssMovie` 0x2ad0c0, mode 1) |
//! | `FadeToBlack(n)` 0x21b438 | [`fade_to_black`] | n blocking black-quad frames over the last image |
//! | `ShowBanner(msg, t)` 0x2789e0 / `ShowPlanetBanner(p)` 0x277c38 | [`show_banner`] / [`show_planet_banner`] | the HUD banner ([`Cinematic::banner`]) |
//! | `UnlockPlanet(p)` 0x2756d0 | [`unlock_planet`] | the saved game's planet bits and map order (+ banner) |
//! | `memcard_Save(0, −1)` | [`save`] | the engine's `memcard_Save` on the native card (`rc-engine` `saves`) |
//! | `EnterSlideshowMode` 0x2ad558 / `PlayMovieB(n)` 0x2ad050 / `EnterMenuMode(kind)` 0x28bf50 | [`enter_slideshow`] / [`play_movie_b`] / [`enter_menu_mode`] | the credits (mode 7), a transitions-table movie (mode 1), the page menu (mode 3) |
//! | `0x2a29a0(dest)` | [`leave_level`] | leave the level for `dest` (`DoSpaceTransition`, `crate::travel`) |
//! | `SetMissionDone(m)` 0x265080 | [`set_mission_done`] | the level's mission byte done (the live bytes and the saved game) |
//! | `0x317d88` / `0x317aa0` / `0x317e70` | [`camera_type6`] | the type-6 camera's switch / tracking / hand-back (the Visibomb) |
//! | `0x313628` / `0x313690` | [`follow_distance`] / [`follow_pivot_height`] | the follow camera's distance / pivot-height targets set by a class (the Pokitaru boats) |
//! | `0x313af0` / `0x3136c8` | [`follow_turn_toward`] / [`follow_look_height`] | the follow camera turned toward a direction / its look height (the collapsing platform 701's look, `0x2f9000`) |
//! | `0x16735c = moby` | [`focus_moby`] | the follow camera's scripted focus moby (its auto-yaw, `0x3111d8`) |
//! | a class-18 record's +0x34 / +0x38, level18 `0x306338` | [`focus_record`] / [`focus_suppress`] | the moby focus record's distance / pivot height (the boss 1422's tweak) and its leave word +0x50 |
//! | `0x15f3fc = f` (gameplay) | [`set_fade`] | the full-screen fade to black a class writes in game mode 0 (`DrawWorld` 0x21a1b8 draws black at `min(f, 1)·128` at the end of the frame; the cutaway machine 1157, the boss 1422) |
//!
//! The camera calls are queued here and applied by the tick right after the moby loop ([`Cinematic::calls`],
//! `crate::tick`); the hero calls go through the hero-block channel ([`crate::moby_update::services::HeroFields`]);
//! the scene / movie / game-state requests are for the engine ([`Cinematic::requests`]). The creature layer's
//! `ScriptRequest`s (gunship 688) are turned into the same calls ([`from_creature`]).

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::ScriptRequest;
use crate::moby_update::services::{HeroCall, HeroPose, World};

/// A camera call of this tick's moby loop, applied to the camera by the tick in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CinematicCall {
    /// `CameraScript(pos, euler, mode, ticks, collide)`.
    CameraScript { pos: [f32; 3], euler: [f32; 3], mode: u8, ticks: i32, collide: bool },
    /// `0x316dd0` (position) / `0x316e28` (Euler).
    CameraTargets { pos: Option<[f32; 3]>, euler: Option<[f32; 3]> },
    /// `0x316e88(a, b)`: mode 3's distance curve (D+0x124 / D+0x128).
    CameraCurve { a: f32, b: f32 },
    /// `0x312b40`: the script camera's springs, (k, damping, max) for its position and its Euler.
    ScriptSprings { pos: [f32; 3], euler: [f32; 3] },
    /// Level02 `0x2f8a18(mode, ticks)`: the script camera's mode and timer.
    ScriptMode { mode: u8, ticks: i32 },
    /// `CameraScript2(kind)` on level `level`.
    CameraRelease { kind: u8, level: u32 },
    /// `HeroTeleport(…, reset_cam = 1)`'s `CameraResetBehindHero` 0x20ee80 (after the hero calls of the tick).
    CameraResetBehindHero,
    /// A creature script's `SetState` (the gunship queues it with its camera call): run with the hero calls.
    HeroState { state: i32, play: bool },
    /// The type-6 camera's switch `0x317d88`, tracking `0x317aa0` and hand-back `0x317e70` (the Visibomb's missile:
    /// [`crate::follow_camera::type6`]).
    Type6(crate::follow_camera::type6::Call),
    /// A Swingshot target's call into the follow camera's look-up hint (`0x2eb3d0` / `0x2eb4c0`:
    /// [`crate::follow_camera::swing::LookHint`]).
    LookHint(crate::follow_camera::swing::HintCall),
    /// The follow camera's distance setter `0x313628(d, rate, base)` called from a class update
    /// ([`crate::follow_camera::Camera::set_distance`]: a no-op unless the follow camera is current).
    FollowDistance { dist: f32, rate: f32, base: bool },
    /// The follow camera's pivot-height setter `0x313690(h, rate)` called from a class update
    /// ([`crate::follow_camera::Camera::set_pivot_height`]).
    FollowPivotHeight { h: f32, rate: f32 },
    /// The follow camera's turn `0x313af0(rate, tolerance, dir)` called from a class update
    /// ([`crate::follow_camera::Camera::turn_toward`]).
    FollowTurnToward { rate: f32, tolerance: f32, dir: [f32; 3] },
    /// The follow camera's turn toward a point `0x313b48(rate, tolerance, point)` called from a class update
    /// ([`crate::follow_camera::Camera::turn_toward_point`]).
    FollowTurnTowardPoint { rate: f32, tolerance: f32, point: [f32; 3] },
    /// The follow camera's look-height setter `0x3136c8(h, rate, add)` called from a class update
    /// ([`crate::follow_camera::Camera::set_look_height`]).
    FollowLookHeight { h: f32, rate: f32, add: bool },
    /// A class's store into 0x16735c, the follow camera's scripted focus moby
    /// ([`crate::follow_camera::Camera::set_focus_moby`]).
    FocusMoby(Option<MobyId>),
    /// `0x313768(k, d)` / `0x313858()` / `0x313820()` / `0x313740(v)`: the follow camera's horizontal spring, stick off,
    /// look from the smoothed target, leash ([`crate::follow_camera::Camera::set_h_spring`] …).
    FollowHSpring { k: f32, d: f32 },
    FollowStickOff,
    FollowLookSmoothed,
    FollowLeash(i16),
    /// The follow camera's row blend from its forward ([`crate::follow_camera::Camera::row_blend`]).
    FollowRowBlend,
    /// A class's store `0x167360 = t` (the ticks since the right stick moved, which ease the focus turn in; Hoven's
    /// drones 326 write 0 with their focus store): [`crate::follow_camera::Camera::focus_ticks`].
    FocusTicks(i32),
    /// A class's stores into a class-18 camera record's pvar block (`0x15ef50 + i·0x20` +0x1c): +0x34 the distance,
    /// +0x38 the pivot height ([`focus_record`]).
    FocusRecord { record: usize, distance: f32, pivot: f32 },
    /// `0x306338(slot)` (level18): a class-18 slot's record +0x50 = 1 (the region leaves this tick) ([`focus_suppress`]).
    FocusSuppress(usize),
    /// A class arming the fly-by camera record `slot` (camera class 19: level10 `0x2f5a50`, level13 `0x316f20`, level14
    /// `0x314168`, level15 `0x2f78b8`; [`crate::follow_camera::Camera::flyby_arm`]) or ending it (level13 `0x316f48`,
    /// level14 `0x314190`; [`crate::follow_camera::Camera::flyby_end`]).
    FlybyArm(usize),
    FlybyEnd(usize),
    /// `0x3135b8(m)` (level14): the Swingshot camera, when current (class 7), looks along moby `m` (D+0x80 = 1, D+0x84 =
    /// m: its yaw, `yaw`), its look height and height 2, distance 7 ([`swing_follow`]).
    SwingFollow { yaw: f32 },
    /// [`CinematicCall::CameraScript`] unless the script camera is already up ([`camera_script_unless_script`]).
    CameraScriptUnlessScript { pos: [f32; 3], euler: [f32; 3], mode: u8, ticks: i32, collide: bool },
}

/// What the engine has to do for the moby loop (outside the gameplay tick).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineRequest {
    /// `DialogStreamStart(k)`: scene k of this level (mode 2). `arrival`: the first-arrival trigger set global flag
    /// 0x13d397 with it.
    StartScene { scene: usize, arrival: bool },
    /// `DialogStreamUpdate(n)` → `StartPssMovie(mpegs[2 + n] NTSC / [21 + n] PAL)`.
    StartMovie { movie: i32 },
    /// `SetMissionDone(m)` (`0x14c050 + level·16 + m = 0xff`).
    MissionDone { mission: u8 },
    /// `FadeToBlack(n)` 0x21b438 called from a class in gameplay (the gold bolt's pickup: `ticks(10)`): the game draws
    /// n black quads over the last image, one per vsync, inside the tick (blocking), then the tick goes on. The engine
    /// holds the next `frames` frames (tick suspended, the last view) under the growing coverage.
    FadeToBlack { frames: i32 },
    /// `FUN_002a2450` / `FUN_002a2480`: the ship moby 0x13e030 hidden (mode |= 3, no collision) / shown.
    ShipHidden(bool),
    /// `memcard_Save(0, −1)` (`rc-engine` `saves::memcard_save`).
    Save,
    /// Class 1750's `FUN_00281fa8(0x1dfc10)` (level18 0x2fad08): `MakeWholeSave` into the ending buffer 0x1ba250 once
    /// (the time warp's `memcard_RestoreGame` reads it; `rc-engine` `saves::store_ending`).
    EndingSave,
    /// `EnterSlideshowMode` 0x2ad558 (game mode 7: the credits, `crate::slideshow`) called from a class (the boss 1422's
    /// ending): the engine runs the mode from the next frame (`FadeToBlack(8)` first).
    Slideshow,
    /// `PlayMovieB(n)` 0x2ad050 (`StartPssMovie(mpegs[40 + n])`, the transitions table 0x1394b8, NTSC) from a class (the
    /// boss 1422: 11, the ending movie).
    MovieB { movie: i32 },
    /// `EnterMenuMode(kind)` 0x28bf50 (Lombyte `PauseAllSounds`; level18 0x278118) from a class: the page menu opened with
    /// `kind` (the boss 1422: 0x21, the end-of-game page 0x1b7670 with no close keys). Was filed as a sound call
    /// (G-AUD-012): the function pauses SFX group 0x1d and the music because it enters the menu.
    EnterMenu { kind: i32 },
    /// The item-movie player of levels 02 / 08 / 16 (level02 `0x298c68`, level08 `0x2a1880`, level16 `0x2923d0`):
    /// `StartPssMovie(mpegs[64 + n])` (NTSC; PAL `[67 + n]`) in the language 0x15ed88 (Aridia's surfer 786: 2)
    /// ([`item_movie`]).
    ItemMovie { movie: i32 },
    /// `0x2a29a0(dest)` (`0x15f5c0 = dest`, `0x15f5d8 = 1`, `0x15f570 = 1`): the level's main loop ends and
    /// `DoSpaceTransition` takes the game to `dest` (Veldin's Clank 834, class 436; `crate::travel`). The engine's one
    /// level change (`rc-engine` `travel_render`).
    LeaveLevel { dest: i32 },
    /// `memcard_Save(0, pretend)` from a class (Umbris' director 436: `memcard_Save(0, 8)`, the save made as if on
    /// Batalia) ([`save_as`]).
    SaveAs { pretend: i32 },
}

/// The cinematic layer's state in the moby services.
#[derive(Clone, Debug, Default)]
pub struct Cinematic {
    /// The camera calls of this tick's moby loop.
    pub calls: Vec<CinematicCall>,
    /// The engine requests, drained by the engine after the tick.
    pub requests: Vec<EngineRequest>,
    /// Global flag 0x13d397 (the Novalis first-arrival scene was started), mirrored from the saved game by the
    /// engine before each tick and set by the mission NPC.
    pub arrival_seen: bool,
    /// Every creature-layer request the tick has taken ([`take_calls`]), oldest first, the last [`LOG_LEN`] kept:
    /// the record of the hand-off for reports and tests.
    pub creature_log: std::collections::VecDeque<ScriptRequest>,
    /// The running mode-2 scene as the moby loop reads it (0x16cd10 / 0x16cd14 / 0x16ce58), published by the engine's
    /// scene player before each scene tick's moby loop; None outside scenes.
    pub scene: Option<crate::scene_player::SceneState>,
    /// 0x162070: the point the cutscene FX driver 1546 last read on the arrival ship (its trail's reference).
    pub fx_ship_point: [f32; 4],
    /// The banner buffer's last `ShowBanner(msg, ticks)` of the moby loop (0x179598 / 0x15f640: one banner, the last
    /// call wins), for the HUD; `seq` counts the calls.
    pub banner: BannerCall,
    /// 0x15f3fc as the gameplay classes write it (the fade to black over the world, 0..1; drawn by the engine at
    /// `trunc(min(f, 1)·128)` black when > 0, `DrawWorld` 0x21a1b8). The scene player keeps its own copy in mode 2 and
    /// ends it at 0, so a scene start clears this one ([`start_scene`]) [L].
    pub fade: f32,
    /// 0x15f400 as the gameplay classes write it (the white quad over the world, 0..1, drawn like [`Cinematic::fade`];
    /// Blarg's escape 1108 whites out the exploding station).
    pub white: f32,
    /// 0x1ba25c: the level moby an In-Level Movies replay hides while its scene runs (`MenuSetPostAction` case 6 from the
    /// hook table 0x179638; `DialogStreamStart` sets its mode bit 1, `FUN_002ac608` clears it and the word).
    pub replay_hide: Option<crate::moby_runtime::MobyId>,
}

/// `ShowBanner(msg, ticks)` 0x2789e0 as the HUD takes it (`HudState::show_banner_msg`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BannerCall {
    pub seq: u32,
    /// The level message id (`msg_string`).
    pub msg: i32,
    /// The countdown 0x15f640 (`ticks(180)` for `ShowBanner(msg, −1)`).
    pub ticks: i32,
}

/// `ShowPlanetBanner(p)` 0x277c38: the message of planet `clamp(p, 0, 18)` from the table at level01 0x20a0c0 (the
/// same ids in every overlay), shown `ticks(1180)`: "Infobot for Planet Aridia acquired" (1009), … .
pub const PLANET_BANNERS: [i32; 19] = [0, 6, 1009, 1010, 3014, 6011, 4006, 5012, 7000, 8007, 8008, 10009, 10010, 12005, 13001, 14001, 15009, 15010, 17001];
/// `ShowPlanetBanner`'s countdown (`ticks(0x49c)`).
pub const PLANET_BANNER_TICKS: i32 = 0x49c;

/// Entries kept in [`Cinematic::creature_log`].
pub const LOG_LEN: usize = 64;

/// `CameraScript(pos, euler, mode, ticks, collide)`. Its store `0x167240 = pos` is seen at once by the rest of the
/// moby loop (mode 3 keeps the current camera): Blarg's queen 1051 reads the camera back in the same tick to hold its
/// cutaway there, and with last tick's position it held the follow camera's spot, looking 30° up past the queen.
pub fn camera_script(w: &mut World, pos: [f32; 3], euler: [f32; 3], mode: u8, ticks: i32, collide: bool) {
    w.svc.cinematic.calls.push(CinematicCall::CameraScript { pos, euler, mode, ticks, collide });
    if mode != crate::follow_camera::script::MODE_ORBIT {
        w.camera = [crate::ps2v::Pf::f(pos[0]), crate::ps2v::Pf::f(pos[1]), crate::ps2v::Pf::f(pos[2]), crate::ps2v::Pf::ONE];
    }
}

/// `0x316dd0` / `0x316e28`.
pub fn camera_targets(w: &mut World, pos: Option<[f32; 3]>, euler: Option<[f32; 3]>) {
    w.svc.cinematic.calls.push(CinematicCall::CameraTargets { pos, euler });
}

/// `0x312b40(d_p, k_p, max_p, d_e, k_e, max_e)` (level06; the arguments in the game's order).
pub fn script_springs(w: &mut World, a: [f32; 6]) {
    w.svc.cinematic.calls.push(CinematicCall::ScriptSprings { pos: [a[1], a[0], a[2]], euler: [a[4], a[3], a[5]] });
}

/// Level02 `0x2f8a18(mode, ticks)`: the script camera's mode and timer
/// ([`crate::follow_camera::Camera::camera_script_mode`]).
pub fn script_mode(w: &mut World, mode: u8, ticks: i32) { w.svc.cinematic.calls.push(CinematicCall::ScriptMode { mode, ticks }); }

/// `0x316e88(a, b)`.
pub fn camera_curve(w: &mut World, a: f32, b: f32) { w.svc.cinematic.calls.push(CinematicCall::CameraCurve { a, b }); }

/// `if (0x167400 && 0x167400->type ≠ 5) CameraScript(pos, euler, mode, ticks, collide)`: the script camera switched in
/// unless it is already the current camera (the item scene 1005's glide, level02 `0x2ea578`): decided when the tick
/// applies the camera calls ([`CinematicCall::CameraScriptUnlessScript`]) [L: the game tests it in the class update;
/// no camera switch happens between the two].
pub fn camera_script_unless_script(w: &mut World, pos: [f32; 3], euler: [f32; 3], mode: u8, ticks: i32, collide: bool) {
    w.svc.cinematic.calls.push(CinematicCall::CameraScriptUnlessScript { pos, euler, mode, ticks, collide });
}

/// The level item-movie player `0x298c68(n)` ([`EngineRequest::ItemMovie`]): with Ratchet's health at 0 the death
/// sequence instead; the help box closed (`StartPssMovie`).
pub fn item_movie(w: &mut World, movie: i32) {
    if movie < 0 { return; }
    if w.hero.health == 0 {
        w.hero_fields_mut().call(HeroCall::Death);
        return;
    }
    w.svc.help.kill();
    w.svc.cinematic.requests.push(EngineRequest::ItemMovie { movie });
}

/// `CameraScript2(kind)`.
pub fn camera_script2(w: &mut World, kind: u8) {
    let level = w.svc.level;
    w.svc.cinematic.calls.push(CinematicCall::CameraRelease { kind, level });
}

/// `0x15f404 = on` (the letterbox and HUD flag; the same global the creature layer's `cutscene` field holds).
pub fn letterbox(w: &mut World, on: bool) { w.svc.creatures.cutscene = on; }

/// `SetState(state, play)` on Ratchet.
pub fn hero_state(w: &mut World, state: i32, play: bool) { w.hero_fields_mut().call(HeroCall::SetState { id: state, play }); }

/// `HeroTeleport(pos, euler, state, reset_cam)` 0x2368e0: position and yaw stored, the motion block cleared, then the
/// hero side's [`crate::hero::Hero::teleport`] at them (`HeroCall::Teleport`: airborne, **the platform carry dropped**,
/// the weapon put away, `SetState(state, 1)` unless −1, the ground probe), the underwater flag 0x167494 = Ratchet's
/// group is 0x11 (under water), and with `reset_cam` the follow camera reset behind him. Only the yaw of the Euler is
/// kept (the port's hero block has no pitch / roll of his own; every Novalis caller passes 0 for them). Without the
/// carry drop a teleport off a moving carrier was carried again by the carrier's move of that tick (Kerwan's train
/// arrival threw Ratchet off the map).
pub fn hero_teleport(w: &mut World, pos: [f32; 3], euler: [f32; 3], state: i32, reset_cam: bool) {
    let f = w.hero_fields_mut();
    f.clear_motion();
    f.pose = Some(HeroPose { pos, yaw: euler[2], target_yaw: euler[2] });
    f.call(HeroCall::Teleport { state });
    // `0x167494 = (0x1413dc == 0x11)` after the SetState (the group of the state it sets: applied with the camera).
    w.svc.water.underwater_store = Some((w.counter, crate::water::world::UnderwaterStore::HeroGroup));
    if reset_cam {
        w.svc.cinematic.calls.push(CinematicCall::CameraResetBehindHero);
        if let Some(s) = w.sound.as_deref_mut() { s.camera_reset_at(pos); }
    }
    // `EnvNearestSamplePoint(hero)`: the env sample point near the destination (reverb, music track; crate::audio).
    if let Some(s) = w.sound.as_deref_mut() { s.hero_teleported(pos); }
}

/// A call into the type-6 camera (`0x317d88` / `0x317aa0` / `0x317e70`).
pub fn camera_type6(w: &mut World, c: crate::follow_camera::type6::Call) { w.svc.cinematic.calls.push(CinematicCall::Type6(c)); }

/// `0x313628(d, rate, base)` from a class (the Pokitaru boats 1075 past their camera node).
pub fn follow_distance(w: &mut World, dist: f32, rate: f32, base: bool) { w.svc.cinematic.calls.push(CinematicCall::FollowDistance { dist, rate, base }); }

/// `0x313690(h, rate)` from a class.
pub fn follow_pivot_height(w: &mut World, h: f32, rate: f32) { w.svc.cinematic.calls.push(CinematicCall::FollowPivotHeight { h, rate }); }

/// `0x313af0(rate, tolerance, dir)` from a class (the collapsing platform 701's look).
pub fn follow_turn_toward(w: &mut World, rate: f32, tolerance: f32, dir: [f32; 3]) {
    w.svc.cinematic.calls.push(CinematicCall::FollowTurnToward { rate, tolerance, dir });
}

/// `0x313b48(rate, tolerance, point)` from a class (the Fleet's help director 1470's look at its cuboids).
pub fn follow_turn_toward_point(w: &mut World, rate: f32, tolerance: f32, point: [f32; 3]) {
    w.svc.cinematic.calls.push(CinematicCall::FollowTurnTowardPoint { rate, tolerance, point });
}

/// `0x3136c8(h, rate, add)` from a class.
pub fn follow_look_height(w: &mut World, h: f32, rate: f32, add: bool) { w.svc.cinematic.calls.push(CinematicCall::FollowLookHeight { h, rate, add }); }

/// The store `0x16735c = moby` (0: `None`) from a class: the follow camera's scripted focus moby.
pub fn focus_moby(w: &mut World, moby: Option<MobyId>) { w.svc.cinematic.calls.push(CinematicCall::FocusMoby(moby)); }

/// `0x3135b8(m)` from a class ([`CinematicCall::SwingFollow`]): the moby's yaw as it stands after its update.
pub fn swing_follow(w: &mut World, m: MobyId) {
    let yaw = w.m(m).rotation[2];
    w.svc.cinematic.calls.push(CinematicCall::SwingFollow { yaw });
}

/// The store `0x167360 = t` from a class ([`CinematicCall::FocusTicks`]).
pub fn focus_ticks(w: &mut World, t: i32) { w.svc.cinematic.calls.push(CinematicCall::FocusTicks(t)); }

/// The stores `+0x34 = distance`, `+0x38 = pivot` into class-18 camera record `record`'s block (the boss 1422's camera
/// tweak `0x2f7288`); the moby loop's mirror ([`crate::moby_update::Services::camera_focus`]) is updated at once.
pub fn focus_record(w: &mut World, record: usize, distance: f32, pivot: f32) {
    if let Some(f) = w.svc.camera_focus.get_mut(record) { *f = [distance, pivot]; }
    w.svc.cinematic.calls.push(CinematicCall::FocusRecord { record, distance, pivot });
}

/// `0x306338(slot)`: a class-18 record's +0x50 = 1 for this tick.
pub fn focus_suppress(w: &mut World, record: usize) { w.svc.cinematic.calls.push(CinematicCall::FocusSuppress(record)); }

/// The fly-by camera record `slot` armed / ended from a class (camera class 19, [`CinematicCall::FlybyArm`]); a
/// negative record (none) is ignored.
pub fn flyby_arm(w: &mut World, slot: i32) { if let Ok(s) = usize::try_from(slot) { w.svc.cinematic.calls.push(CinematicCall::FlybyArm(s)); } }
pub fn flyby_end(w: &mut World, slot: i32) { if let Ok(s) = usize::try_from(slot) { w.svc.cinematic.calls.push(CinematicCall::FlybyEnd(s)); } }

/// A Swingshot target's hint call (`0x2eb3d0` reset / `0x2eb4c0` offer), applied to the camera before its update.
pub fn look_hint(w: &mut World, c: crate::follow_camera::swing::HintCall) { w.svc.cinematic.calls.push(CinematicCall::LookHint(c)); }

/// `DialogStreamStart(k)` (level01 0x2ac330; the same code on other levels, e.g. level18 0x2983e8): with Ratchet's
/// health 0x1415f8 at 0 the death sequence (`0x2319b0`) runs instead of the scene.
pub fn start_scene(w: &mut World, scene: usize, arrival: bool) {
    if w.hero.health == 0 {
        w.hero_fields_mut().call(HeroCall::Death);
        return;
    }
    // DialogStreamStart sets 0x15f3fc = 1.0 and the scene ramps it to 0 (the scene player's own copy).
    w.svc.cinematic.fade = 0.0;
    // DialogStreamStart 0x2ac330 closes the help box at once (`FUN_002258b0`).
    w.svc.help.kill();
    w.svc.cinematic.requests.push(EngineRequest::StartScene { scene, arrival });
    // DialogStreamStart stores game mode 2 (0x15f5c4) at once: the rest of this moby loop reads it (Umbris' director
    // 436 falls from state 10 into 0xb in the same tick and waits there on it).
    w.svc.game_mode = 2;
    scene_hero(w);
}

/// `DialogStreamStart`'s `hero_set_state(100, 2)` inside the asking tick: queued on the hero-call channel, which runs
/// after the moby loop and before the hero update, so the hero update of this tick is already the scene body's and
/// reads no pad (the port used to freeze him on the next frame only: one tick of play under the scene's start, a jump or
/// a swing that carried into the scene). His hiding (0x1413f5, the held objects) waits for the start fade's black: the
/// game's asking frame is not drawn and its fade runs over the last image shown, with him in it (`rc-engine`
/// scene_render).
pub fn scene_hero(w: &mut World) {
    w.hero_fields_mut().call(HeroCall::SetState { id: crate::scene_player::HERO_SCENE_STATE, play: true });
}

/// `DialogStreamUpdate(n)`.
pub fn start_movie(w: &mut World, movie: i32) {
    // StartPssMovie 0x2ad0c0 closes the help box (`FUN_002258b0`).
    w.svc.help.kill();
    w.svc.cinematic.requests.push(EngineRequest::StartMovie { movie });
    // StartPssMovie stores game mode 1 (0x15f5c4) at once: the rest of this moby loop reads it (a talker's
    // `NpcTalkUpdate` does nothing outside mode 0); `MovieExitToGameplay` sets 0 again (crate `rc-engine` movie_render).
    w.svc.game_mode = 1;
}

/// `0x15f3fc = f` from a class update in gameplay ([`Cinematic::fade`]).
pub fn set_fade(w: &mut World, f: f32) { w.svc.cinematic.fade = f; }

/// `0x15f400 = f` from a class update in gameplay ([`Cinematic::white`]).
pub fn set_white(w: &mut World, f: f32) { w.svc.cinematic.white = f; }

/// `FadeToBlack(n)` from a class (see [`EngineRequest::FadeToBlack`]).
pub fn fade_to_black(w: &mut World, frames: i32) { w.svc.cinematic.requests.push(EngineRequest::FadeToBlack { frames }); }

/// `EnterSlideshowMode` 0x2ad558 from a class ([`EngineRequest::Slideshow`]): the game mode is 7 at once.
pub fn enter_slideshow(w: &mut World) {
    w.svc.game_mode = 7;
    w.svc.cinematic.requests.push(EngineRequest::Slideshow);
}

/// `PlayMovieB(n)` 0x2ad050 from a class ([`EngineRequest::MovieB`]): `n < 0` does nothing; else `StartPssMovie` (the
/// help box closed, `FUN_002258b0`; the game mode 1 at once).
pub fn play_movie_b(w: &mut World, movie: i32) {
    if movie < 0 { return; }
    w.svc.help.kill();
    w.svc.game_mode = 1;
    w.svc.cinematic.requests.push(EngineRequest::MovieB { movie });
}

/// `EnterMenuMode(kind)` 0x28bf50 from a class ([`EngineRequest::EnterMenu`]): the game mode is 3 at once (a later class
/// of this moby loop reads 3).
pub fn enter_menu_mode(w: &mut World, kind: i32) {
    w.svc.game_mode = 3;
    w.svc.cinematic.requests.push(EngineRequest::EnterMenu { kind });
}

/// `0x2a29a0(dest)` from a class ([`EngineRequest::LeaveLevel`]): the main loop leaves after this frame.
pub fn leave_level(w: &mut World, dest: i32) { w.svc.cinematic.requests.push(EngineRequest::LeaveLevel { dest }); }

/// `memcard_Save(0, −1)`: the engine saves to the native card (`rc-engine` `saves::memcard_save`).
pub fn save(w: &mut World) { w.svc.cinematic.requests.push(EngineRequest::Save); }

/// `memcard_Save(0, pretend)` ([`EngineRequest::SaveAs`]).
pub fn save_as(w: &mut World, pretend: i32) { w.svc.cinematic.requests.push(EngineRequest::SaveAs { pretend }); }

/// Class 1750's ending save (`FUN_00281fa8(0x1dfc10)`, level18 0x2fad08).
pub fn ending_save(w: &mut World) { w.svc.cinematic.requests.push(EngineRequest::EndingSave); }

/// `SetMissionDone(m)` 0x265080: `*(0x14c050 + level·16 + m) = 0xff` unless `m` is 0xff (no other effect). The one
/// writer of the mission bytes for every class (checkpoint 805, talker 774, mission NPC 730 / 790, infobot 750, bolt
/// crank 280; the per-level classes of docs/plan/level_scripting.md): the engine applies it after the tick to the live
/// bytes the classes read ([`crate::moby_update::services::LevelMissions::done`]) and to the saved game, so a class
/// later in the same moby loop still reads the old byte (one tick later than the game's direct store) [L: no reader
/// of the same tick depends on it on the ported levels].
pub fn set_mission_done(w: &mut World, mission: u8) {
    if mission != 0xff { w.svc.cinematic.requests.push(EngineRequest::MissionDone { mission }); }
}

/// `ShowBanner(msg, ticks)` 0x2789e0 (`ticks` already scaled; the caller passes `ticks(180)` for −1).
pub fn show_banner(w: &mut World, msg: i32, ticks: i32) { banner_call(&mut w.svc.cinematic, msg, ticks); }

/// `ShowBanner(msg, ticks)` from outside the moby loop (the page menu's code entry `MenuInput` 0x298f80, the hero
/// update's cheat entry `0x2285a0`): the same banner buffer.
pub fn banner_call(c: &mut Cinematic, msg: i32, ticks: i32) {
    let b = &mut c.banner;
    *b = BannerCall { seq: b.seq.wrapping_add(1), msg, ticks };
}

/// `ShowPlanetBanner(p)` 0x277c38.
pub fn show_planet_banner(w: &mut World, planet: i32) {
    let t = w.ticks(PLANET_BANNER_TICKS);
    show_banner(w, PLANET_BANNERS[planet.clamp(0, 18) as usize], t);
}

/// `UnlockPlanet(p)` 0x2756d0: when planet `p` is still locked (0x13dd40[p], the moby loop's mirror of the saved
/// game), it is unlocked and appended to the galaxy-map order 0x13d510 (`GameState::unlock_planet`, through the
/// saved-game write channel), and its banner shown unless `p` is the current level.
pub fn unlock_planet(w: &mut World, planet: i32) {
    let Ok(p) = usize::try_from(planet) else { return };
    if p >= 20 { return; }
    let u = &mut w.svc.interact.game.planet_unlocked;
    if u.len() < 20 { u.resize(20, 0); }
    if u[p] != 0 { return; }
    u[p] = 1;
    w.svc.interact.writes.push(crate::moby_update::interact::GameWrite::UnlockPlanet(p));
    if planet as u32 != w.svc.level { show_planet_banner(w, planet); }
}

/// A creature layer script request as the camera / hero calls the game makes for it (gunship 688:
/// `SetState(0x72, 0)` then `CameraScript(centre, euler, 0, 0, 0)`; at the end `CameraScript2(0)` then
/// `SetState(0, 1)`).
pub fn from_creature(r: &ScriptRequest, level: u32) -> [CinematicCall; 2] {
    match *r {
        ScriptRequest::Start { hero_state, centre, euler, .. } => [
            CinematicCall::HeroState { state: hero_state as i32, play: false },
            CinematicCall::CameraScript { pos: centre, euler, mode: 0, ticks: 0, collide: false },
        ],
        ScriptRequest::End { .. } => [CinematicCall::CameraRelease { kind: 0, level }, CinematicCall::HeroState { state: 0, play: true }],
    }
}

/// This tick's camera calls for the tick: the classes' own, then the creature layer's requests
/// ([`from_creature`]); both queues are emptied.
pub fn take_calls(svc: &mut crate::moby_update::Services) -> Vec<CinematicCall> {
    let mut v = std::mem::take(&mut svc.cinematic.calls);
    let level = svc.level;
    for r in std::mem::take(&mut svc.creatures.scripts) {
        v.extend(from_creature(&r, level));
        let log = &mut svc.cinematic.creature_log;
        if log.len() == LOG_LEN { log.pop_front(); }
        log.push_back(r);
    }
    v
}

/// The tick's application of the camera calls (right after the moby loop, before the hero update).
pub fn apply_camera_calls(cam: &mut crate::follow_camera::Camera, calls: &[CinematicCall], inp: &crate::follow_camera::CamInput) {
    for c in calls {
        match *c {
            CinematicCall::CameraScript { pos, euler, mode, ticks, collide } => {
                cam.camera_script(pos, euler, mode, ticks, collide, crate::hero::physics::to_f32x3(inp.hero.pos))
            }
            CinematicCall::CameraScriptUnlessScript { pos, euler, mode, ticks, collide } => {
                if !cam.script_active() { cam.camera_script(pos, euler, mode, ticks, collide, crate::hero::physics::to_f32x3(inp.hero.pos)) }
            }
            CinematicCall::CameraCurve { a, b } => cam.camera_script_curve(a, b),
            CinematicCall::FlybyArm(s) => cam.flyby_arm(s),
            CinematicCall::FlybyEnd(s) => cam.flyby_end(s),
            CinematicCall::ScriptSprings { pos, euler } => cam.camera_script_springs(pos, euler),
            CinematicCall::ScriptMode { mode, ticks } => cam.camera_script_mode(mode, ticks, crate::hero::physics::to_f32x3(inp.hero.pos)),
            CinematicCall::CameraTargets { pos, euler } => cam.camera_script_targets(pos, euler),
            CinematicCall::CameraRelease { kind, level } => cam.camera_script2(kind, level),
            // CameraResetBehindHero 0x20ee80 on the follow camera (the hero's pose is this tick's: the class stores
            // were applied just before).
            CinematicCall::CameraResetBehindHero => cam.reset(inp),
            CinematicCall::HeroState { .. } => {}
            CinematicCall::Type6(c) => cam.type6_call(&c, inp),
            CinematicCall::LookHint(c) => cam.look_hint(c, inp.pad.ry.to_f32()),
            CinematicCall::FollowDistance { dist, rate, base } => cam.set_distance(dist, rate, base),
            CinematicCall::FollowPivotHeight { h, rate } => cam.set_pivot_height(h, rate),
            CinematicCall::FollowTurnToward { rate, tolerance, dir } => cam.turn_toward(rate, tolerance, dir),
            CinematicCall::FollowTurnTowardPoint { rate, tolerance, point } => cam.turn_toward_point(rate, tolerance, point),
            CinematicCall::FollowLookHeight { h, rate, add } => cam.set_look_height(h, rate, add),
            CinematicCall::FollowHSpring { k, d } => cam.set_h_spring(k, d),
            CinematicCall::FollowStickOff => cam.stick_off(),
            CinematicCall::FollowLookSmoothed => cam.look_from_smoothed(),
            CinematicCall::FollowLeash(v) => cam.set_leash(v),
            CinematicCall::FollowRowBlend => cam.row_blend(),
            CinematicCall::FocusMoby(m) => cam.set_focus_moby(m),
            CinematicCall::FocusTicks(t) => cam.focus_ticks = t,
            CinematicCall::FocusRecord { record, distance, pivot } => {
                if let Some(f) = cam.level_cams.slots.get_mut(record).and_then(|s| s.focus.as_mut()) {
                    f.distance = distance;
                    f.pivot_height = pivot;
                }
            }
            CinematicCall::SwingFollow { yaw } => {
                if cam.swing.active {
                    let s = &mut cam.swing;
                    s.follow_yaw = Some(yaw);
                    s.look_h = 2.0;
                    s.height = 2.0;
                    s.dist = 7.0;
                }
            }
            CinematicCall::FocusSuppress(record) => {
                if let Some(f) = cam.level_cams.slots.get_mut(record).and_then(|s| s.focus.as_mut()) { f.suppress = 1; }
            }
        }
    }
}

/// The creature scripts' `SetState` calls, with the hero calls of the tick.
pub fn run_hero_calls(h: &mut crate::hero::Hero, calls: &[CinematicCall], c: &mut crate::hero::states::Ctx) {
    for call in calls {
        if let CinematicCall::HeroState { state, play } = *call { h.set_state(c, state, play); }
    }
}

/// The moby that issued a creature request (reports).
pub fn creature_moby(r: &ScriptRequest) -> MobyId {
    match *r {
        ScriptRequest::Start { moby, .. } | ScriptRequest::End { moby } => moby,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gunship_requests_map_to_the_game_calls() {
        let s = ScriptRequest::Start { moby: 3, hero_state: 0x72, cuboid: 23, centre: [1.0, 2.0, 3.0], euler: [0.0, 0.02, 1.39], ticks: 510 };
        let c = from_creature(&s, 1);
        assert_eq!(c[0], CinematicCall::HeroState { state: 0x72, play: false });
        assert_eq!(c[1], CinematicCall::CameraScript { pos: [1.0, 2.0, 3.0], euler: [0.0, 0.02, 1.39], mode: 0, ticks: 0, collide: false });
        let e = from_creature(&ScriptRequest::End { moby: 3 }, 1);
        assert_eq!(e, [CinematicCall::CameraRelease { kind: 0, level: 1 }, CinematicCall::HeroState { state: 0, play: true }]);
        assert_eq!(creature_moby(&s), 3);
    }
}
