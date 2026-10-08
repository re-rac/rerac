//! Game mode 6: the ship's take-off and landing scenes, the fly-away, the flight between planets (level01
//! `GameStateUpdate` 0x2a4080 with its entries; docs/plan/cutscenes_transitions.md §4.1–4.2, menus.md §5).
//!
//! One [`ShipMode::frame`] = one 60 Hz frame. The engine runs the world part of `GameStateUpdate` (the moby loop,
//! the level callbacks, the hero update 0x228870 (Ratchet sits in state 100, hidden), the particles, the sound step,
//! the counter: the gameplay tick in its scene form, without its `CameraUpdate`) on the frames
//! [`SpaceFrame::world_runs`] says, then calls `frame` with a [`World`] on the same table; sub 3's `CameraUpdate` runs
//! after the body (`rc-engine` `travel_render::fly_away_camera`), so the camera targets the body sets reach the camera in
//! the same tick, as in `GameStateUpdate`. The blocking `FadeToBlack(n)` calls inside the game's entries are frames of
//! their own here ([`Phase::Hold`]: the world frozen, the black quads accumulating over the last image).
//!
//! The same player plays the weapon demo scenes ([`SUB_DEMO`]: `VendorModeUpdate` 0x2b03b8 substate 3 in the vendor's
//! frame; [`ShipMode::demo_start`], [`DemoFrame`]).
//!
//! | address | what | here |
//! |---|---|---|
//! | `ShipTakeOff` 0x2a27c8 | HP < 1 → 1; 0x15f5d8 = 1; `music_Pause(0)`; the speech stream stopped (0x15172a ∈ {6, 7} → 5); `sound_update`; the help box killed `FUN_002258b0`; `EnterShipMode(0)`; 0x13e05c = 0 | [`ShipMode::take_off`] |
//! | `EnterShipMode(sub)` 0x2a24b8 | 0x13e054 = −1, 0x13e038 = 0, 0x15f5d8 = 1, the scene state cleared (0x16cce0, 0x17c7c0, 0x17c800), 0x13e050 = sub, mode 6, fade 1.0, white 0x15f400 = 0, `SetState(100, 2)`, 0x1413f5 = 1, `FUN_002486c0` (the hero and his items hidden), the ship mode \|= 1, every live moby of class 74 / 203 mode \|= 0x80; the lump: take-off `ship + 1`, stream `ship + 6` (level 10, ship 1, item 6 not owned: lump 0, stream 0xb; level 14, ship 2: lump 8, stream 0xe), `FUN_00259628(lump, ticks(6))`; landing `ship + 5`, stream `ship + 3` (`ship` when 0x13e05c: the level-start landing; level 10 / ship 1 / no item 6: lump 4, stream 10 / 9; level 14 / ship 2: lump 9, stream 0xd / 0xc), fade `ticks(6)` unless 0x13e05c; 0x1516ec = 40000 + stream; chunk 0; the stream waited for | [`ShipMode::enter`] |
//! | `FUN_00259628(lump, fade)` | the lump read (`anim_looking_thing_2[lump]`, NTSC 0x138e68 / PAL 0x138eb8), `FadeToBlack(fade)` when ≠ 0 (blocking), the chunk table (≤ 70) | [`ShipMode::enter`] (the lump comes loaded: `SpaceLump`) |
//! | `ShipTravelTo(dest)` 0x2a2848 | 0x15f5d8 = 1, mode 6, fade 1, white 0, 0x13e050 = 0, 0x13e054 = −1; dest = level → `EnterShipMode(8)`, then the scene jumps to `ticks(0x160570[ship])` (240) at once; else `memcard_Save(0, dest)`, 0x13e054 = −1, sub 3, 0x15f5c0 = dest | [`ShipMode::travel_to`] |
//! | `0x2a29a0(p)` | 0x15f5c0 = p, 0x15f5d8 = 1, 0x15f570 = 1 (leave the level) | `cinematic::EngineRequest::LeaveLevel` (`crate::moby_update::story::level_exit`) |
//! | `ShipLandingStart` 0x2a29c0 | 0x13e05c = 1, 0x13e05a = 0; level 1 with planet 3 locked (0x13dd43), level 0, level 14 with 0x13d3f0 = 0, or level > 0x13: mode 0 at once; else mode 6, sub 0 then `EnterShipMode(8)` | [`ShipMode::landing_start`] |
//! | 0x2a4080 subs 0 / 8 | 0x13e054++; fade −= 0.125 (≥ 0); scene / chunk tick +1; skip = 0x13e058, or (✕ / △ pressed, tick > `ticks(30)`, fade 0) and (take-off: tick < `ticks(558 − 30)`; landing: tick < end − `ticks(30)`); a skip stops the stream; take-off → tick = `ticks(558)`, its chunk parsed, chunk tick = tick % 96, `FadeToBlack(4)` (none with 0x13e058, which is cleared), fade 1.0; landing → tick = end | [`ShipMode::scene_tick`] |
//! | | the camera: the record of the chunk tick, ship-local: eye `rows·e + pos`, angle z + the ship's yaw; tan(hfov/2) the record's; the mirror cheat 0x15edb4: `FastVecCross(left, up, forward)` | [`ShipMode::scene_tick`] (`scene_player::scene_camera`) |
//! | | each actor: frame f = chunk tick >> 1, f + 1, t = (tick & 1)·0.5 (1.0 on an odd tick of a cut record); position = the lerp of its track, ship-local; +0x71 = 0xff; yaw = the ship's; +0x52 / +0x53 the class's streamed slot; `MobyBuildMatrix` (the streamed sequence's bounding sphere) | [`ShipMode::scene_tick`] (the poses: [`SpaceFrame::actors`]; the table mobys written; the streamed class: [`ShipMode::actor_anim_for`]) |
//! | | class 0: +0x7f (its shadow) = take-off: 0 on level 10 without item 6, else 0x18 until `ticks(62)`; landing: 0x18 past `ticks(360)` (level 10: only with item 6), else 0; class 10: take-off 0x18 until `ticks(350)`, landing 0x18 past `ticks(524)`; +0x7f ≠ 0 → `FUN_0026f0e0`; class 10 → `FUN_00228bc0`; class 0 → `FUN_0024a1d0` and, on level 10 with item 6 or on level 13, the helmet: `CreateMoby(0x509)` once (+0x32 = 0x40, mode \|= 0x806, Ratchet's light, +0x73 = 0x18), `FUN_0024a310` (on Ratchet's joint 4) | +0x7f [`ShipMode::scene_tick`]; `FUN_0026f0e0` and the actors' shadows: [`SpaceFrame::scene_actors`] (`rc-engine` `shadow_render`); `FUN_00228bc0`: [`clank_actor`] (its glow sprite: NOT ported, G-REN-005); `FUN_0024a1d0`: [`ShipMode::ratchet_actor`] (its `FUN_0024a0e8`: NOT ported, G-CUT-009); the helmet: [`ShipMode::helmet`] |
//! | | the ship classes 531..533: glass `RegisterDrawCallback2(0x2a70a8)`, shadow `RegisterDrawCallback(0x2a2130)`; take-off: the glow pulse `cos(((tick & 0x3f) − 0x20)·π/32)·80 + 0x78` until `ticks(360)`; to `ticks(504)`: `min(0xff, (tick − 360)·1.25)` (533: no red), past `ticks(420)` the real ship's flames (+0xbc = (tick − 420)·1.2, +0xb2 = 0), past `ticks(464)` white += 0.025 (≤ 1); then white −= 0.025 (≥ 0), glow 0xa0a0a0; landing: until `ticks(240)` the exhaust (`fun_0022f5b0(−4·dt)`, `(−3·dt)` until `ticks(200)`), the pulse, flames (+0xbc = 0x32, +0xb2 = 10); to `ticks(360)`: the real ship's flames dying (+0xbc = (300 − tick)·1.5 until `ticks(300)`), glow `(360 − tick)·1.25`; then the pulse | [`ship_fx`] |
//! | | the end (tick ≥ end): stream stopped, tan 0.63, the actors deleted (their class slots freed) and the helmet, the ship shown (mode &= ~1), classes 74 / 203 &= ~0x80; take-off: `EnterMenuMode(0)` with kind 0xe (the planet page), 0x15f5d8 = 1; landing: mode 0, `music_Unpause`, 0x15f5d8 = 1, 0x1413f5 = 0, `FUN_002487a8` (shown), then the level-start landing `FUN_0024a3d8` (a point 0.75 behind him, `GroundHeight` from 0.5 up; within 0.3: placed there and `0x249580(yaw, point, 1)`, the walk-to) or `HeroTeleport(0x13e090, 0x13e0a0, 0, 1)` | [`ShipMode::scene_end`] |
//! | 0x2a4080 sub 3 | the first tick: `music_Pause(0)`, the stream 40015 + ship, `FadeToBlack(ticks(12))`, the ship's sequences 1 → 2 at t 0, +0x72 = 0xff, +0x94 = 0; with a path: the riders `CreateMoby(0)` / `CreateMoby(10)` (+0x32 = 0x1ff, +0x72 = 0xff, +0x94 = 0, mode \|= 6, the hero's light); `CameraScript(0x167240, 0x167250, 1, 0, 0)`; the path's points get their turn rates as w (normalised to 20° at the sharpest), the first segment's length, the start (ship) and end (point 0) of the approach, its pitch and yaw | [`ShipMode::fly_away_start`] |
//! | | each tick: the camera eases between the two cuboids (`FUN_00270830(1, 0.666·dt², 0.666·dt², 0.5·dt)`), `0x316dd0` / `0x316e28`; until `ticks(150)` the ship eases to the path's start (`(1 − cos(π·tick/120))/2`, 1 from `ticks(120)`): position, pitch, yaw, its blend +0x54, fade −0.125, the exhaust `fun_0022f5b0(−3.75·dt)` while < 1; then speed += 0.8 (≤ 100), ✕ / △ or the last 6 points start the fade (0.0625), ships 0 / 1 flames (0x32, 10), t += speed·dt / segment; past the end or the fade at 1: 0x15f5d8 = 1, 0x15f570 = 1 (leave); else the path pose (`0x277d40`, its roll into +0x40), a trail sample, `RegisterDrawCallback(0x2a2d28)`, the fade +0.0625 once started; the riders on their seats (`rows·0x1be200 / 0x1be230 + pos`, the ship's rotation) | [`ShipMode::fly_away_tick`] |
//! | `EnterSpaceLoadingLoop` 0x2a5868 | sub 4, 0x13e054 = −1, fade 1, white 0, 0x1413d0 = 0, 0x15f5d8 = 0, the memory slots / VRAM heads / view context reset (n/a: the engine's), the `transition` lump read and decompressed (`rc_formats::transition`): its GS image, its classes 531..533 (byte-identical to the `spaceships` files' the engine draws), the chrome map TEX0 (identical to the levels'), `LoadSky` (the flight sky), the FX bank (`GetEffectTex` during the flight), the particle textures (n/a: no particle pass in the flight's draw), the planet picture `[dest]` (0x160640) and the caption `[19 + 19·max(lang − 1, 0) + dest]` (0x160648); `FadeToBlack(ticks(12))`; variant v = `rand() >> 16 & 3` (v 4 and 0x13e08c = 2 for dest 0 or dest 1 with planet 3 locked), the count 0, the trail reset (head / count 0), 0x1605b0 = 0x1605a0 (16 bytes: −370, −280, 280, 0), the scene state cleared, variant v's chunk table, chunk 0, the flight's sound bank (`snd_bank_load_from_ee_cb`, its handle 0x15ed5c; the loop waits for it) | [`ShipMode::flight_start`] (the lump's data: `rc-engine` `flight_render`) |
//! | `SpaceLoadingLoop` 0x2a33b0 (sub 4) | `FUN_002ab920` (the draw lists empty); fade −= 0.25 (≥ 0); ticks +1; tick 1 (not the first-arrival flights): `snd_play_sound_vol_pan_pmpb(0x15ed5c, v, 0x400, 0, 0, 0)` (the bank's sound v); chunk roll at > 0x5f; at the end: the actors deleted (their class slots freed); while the load runs (0x15ee48 < 3; not the first-arrival flights) the count 0x13e08c = 0; count > 1 → done (0x15ee4a = 0, 0x15f5d8 = 1, no draw this frame); count 0 → v = (v + `(rand() >> 16) % 3` + 1) & 3, else v = 4; the trail reset, count +1, 0x1605b0 = 0x1605a0, the scene state cleared, the next variant's chunk 0; the camera (`build_object_rotation_matrix`, tan ≥ 0x1be0c8[v]); v 4: 0x1605b0 += (end − tick)/end · 0x1605c0; the shell translation 0x160520 = 0x1be060[v]·(tick − ticks(120))·20·f (f = (end − tick)/end in v 4, else 1); 0x1604c4 = 0x1be0b0[v] (the sky's turn); per actor two half-ticks (k = 0, 1; t = (tick & 1)·0.5 + k·0.25): the position lerp, +0x52 / +0x53 = 2, frames 0 / 0, `update_moby_animation_state` (both frame buffers at sequence 2 frame 0), the first 0x20 payload bytes of streamed frames f and f + 1 copied into it (joints 0..3: f + 1's stay), +0x72 = +0x71 = 0xff, +0x32 = 0x1ff, `MobyBuildMatrix`, +0x52 / +0x53 = the streamed slot, `MobyAttachToJoint(moby, 1 / 2)` (joint lists 1 and 2: the engines) → a trail sample (head +1 & 31, count ≤ 32); v 4 on the first-arrival flights: the actor hidden (mode \| 1), the trail count 0; v 4 in the last 56 ticks: scale = class scale·(end − tick)/56, the head alpha of the three ships' trail colours 0x1bdfb0 = end − tick, else 0x38; `IncrementTickCounter` | [`ShipMode::flight_tick`] (the pose: [`flight_sequence`]) |
//! | `VendorStartWeaponDemo` 0x2ae7f8 (scene part) | the frame: origin = vendor rows · (4, 0, 0) (gp−0x5b10) + its position, z + 2, `GroundHeight(0.5)`; Euler (0, 0, yaw + π) → 0x1ca9c0; the scene state cleared; fade 0, white 0; `FUN_002594e0(unknown_1530[0x1ca4a0[item]])` (its `FadeToBlack(4)`); 0x1516ec = 0x2734 (`vendor_audio[36]`); chunk 0; the stream waited for | [`DemoFrame::of_moby`], [`ShipMode::demo_start`] (the vendor's part: `menus::vendor`, `rc-engine` `interact_render`) |
//! | `VendorModeUpdate` substate 3 | the world part (moby loop, level callbacks, hero update, particles); chunk / scene tick + 1; fade + 0.1 (≤ 1) past `end − 12`, else − 0.125 (≥ 0); before the end: chunk roll (≥ 0x60), the camera in the demo's frame (record tan; eye = rows·e + origin, Euler z + yaw), each actor (frames, t, cut, position in the frame, +0x71 = 0xff, yaw, `MobyBuildMatrix`, +0x7f → `FUN_0026f0e0`, class 0 → `FUN_0024a1d0`), `UpdateAllPointLights`, `IncrementTickCounter`; at the end: fade 0, tan 0.63, the actors deleted, `VendorExit(1)` (or `FUN_002aecf0` for the item offer's demo `FUN_002aea70`: NOT ported, G-UI-006) | [`ShipMode::demo_tick`] ([`SpaceEvent::DemoDone`]) |
//! | `DrawWorldPaused` 0x2a3b90 (sub 4's draw, `dispatch_game_state_update` 0x2a57f0) | tan ≥ 0.63 for the sky only; `DrawSkyShells` 0x29f260 (per shell k < count: euler (0, y_k, 0x1604c4 + z_k), rows ·s_k, translation 0x160520: [`FLIGHT_SHELLS`]); v 4: the planet picture `fun_0022e8c8` 0x2a31d0 (one `FastDrawQuadReal`, corners 0x1bdfd0·0x1be010[level] + 0x1605b0, ST 0x1bdd30, RGBA 0x80808080, ALPHA 0x44, TEST 0x31801: Z neither tested nor written); `DrawMobys` (the actors); not the first-arrival flights: the trail of actor 0 (`fun_0022e420`, FX 0); v 4 past tick 0x3c: the caption `fun_0022ea08` 0x2a3310 (`DrawTexturedQuad(0x20, H − 0x58, 0x100, 0x20)`, alpha min((tick − 0x3c)·2, 0x80)); not the first-arrival flights: the canopy glass `ShipDrawCallback` 0x2a70a8 of actor 0 (FX 1 in sub 4); the card busy (0x13d364 > 2 or 0x13d36c ≥ 0): the card icon FX 2 at (0x2c, H − 0x60, 64, 64) and FX 3 turning once per 55 vsyncs at (76, H − 64) 17×17 (`fun_00200600`); the black fade | [`FlightDraw`] (drawn by `rc-engine` `flight_render`) |

use super::ship::{self, Trail};
use super::{ship_index, to_world, ShipGlobals, LANDING_REVISIT_TICK, RIDER_CLANK, RIDER_RATCHET, SHIP_CLASSES, TAKEOFF_SKIP_TICK, TRAIL_A, TRAIL_B};
use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{add_rot, atan, sub_rot, DT, DT2};
use crate::moby_update::services::World;
use crate::scene_player::{fade_to_black_coverage, SceneCamera};
use rc_formats::scene::Scene;
use std::sync::Arc;

/// Mode-6 substates (0x13e050).
pub const SUB_TAKEOFF: i32 = 0;
pub const SUB_FLYAWAY: i32 = 3;
pub const SUB_FLIGHT: i32 = 4;
pub const SUB_LANDING: i32 = 8;
/// A weapon demo scene (port-only id): the vendor's substate 3 (`VendorModeUpdate` 0x2b03b8, game mode 5) and the
/// demo-only entry `FUN_002aea70` (the item offer 0x2e1ac0) play a space scene with this player in a moby's frame.
pub const SUB_DEMO: i32 = 0x103;
/// The weapon demo's stream: 0x1516ec = 0x2734 (`vendor_audio[36]`).
pub const DEMO_STREAM: i32 = 0x2734;
/// `vendor_audio[id − 10000]`.
pub const VENDOR_AUDIO_BASE: i32 = 10000;
/// `VendorStartWeaponDemo`'s scene frame offset in the vendor's frame (gp−0x5b10 = 0x1610f0, every overlay) and its
/// yaw added to the vendor's (π).
pub const VENDOR_DEMO_OFFSET: [f32; 3] = [4.0, 0.0, 0.0];
pub const VENDOR_DEMO_YAW: f32 = std::f32::consts::PI;
/// Chunk ticks (NTSC 0x60, PAL 0x50).
pub const CHUNK_TICKS: i32 = 0x60;
/// The space-audio stream ids are 40000 + k (`space_audio[k]`).
pub const STREAM_BASE: i32 = 40000;
/// The fly-away's stream: 0x9c4f + ship = 40015 + ship.
pub const FLYAWAY_STREAM: i32 = 0x9c4f - STREAM_BASE;
/// The helmet class `CreateMoby(0x509)` (level 10 with item 6, level 13).
pub const HELMET_CLASS: i16 = 0x509;
/// Item 6 (0x13d4c6, the O2 Mask): level 10's special lumps without it.
pub const ITEM_O2_MASK: usize = 6;
/// Classes 74 / 203 (mode bit 0x80 while mode 6 runs: the mine and the decoy).
pub const HIDDEN_CLASSES: [i16; 2] = [0x4a, 0xcb];
/// The gameplay tan(hfov/2) the end restores.
pub const GAMEPLAY_TAN: f32 = 0.63;
/// 0x1be0c8[v]: the flight's smallest tan(hfov/2) per variant.
pub const FLIGHT_MIN_TAN: [f32; 5] = [0.3, 0.3, 0.3, 0.3, 0.63];
/// The flight variants (0..=3 random, 4 the planet approach).
pub const FLIGHT_VARIANTS: usize = 5;

/// A space-scene lump (`anim_looking_thing_2[k]`, the take-off / landing) or a flight variant's chunk table.
#[derive(Clone, Debug)]
pub struct SpaceLump {
    pub index: usize,
    pub scene: Arc<Scene>,
}

/// The lump and stream `EnterShipMode(sub)` picks.
pub fn lump_and_stream(sub: i32, level: i32, ship: i16, o2_mask: bool, level_start: bool) -> (usize, i32) {
    let s = ship as i32;
    if sub == SUB_TAKEOFF {
        if level == 10 && s == 1 && !o2_mask { return (0, 0xb); }
        if level == 14 && s == 2 { return (8, 0xe); }
        ((s + 1) as usize, s + 6)
    } else {
        let stream = if level_start { s } else { s + 3 };
        if level == 10 && s == 1 && !o2_mask { return (4, if level_start { 9 } else { 10 }); }
        if level == 14 && s == 2 { return (9, if level_start { 0xc } else { 0xd }); }
        ((s + 5) as usize, stream)
    }
}

/// The blocking parts of the entries, as frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Not in mode 6.
    Idle,
    /// `FadeToBlack(n)` step k (the world frozen), then `then`.
    Hold { n: u32, k: u32, then: After },
    /// The stream wait (one vsync: the buffered state is not traced [L]).
    Wait,
    /// `GameStateUpdate` frames.
    Run,
}

/// What follows a hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum After {
    /// The stream wait, then the scene frames.
    Scene,
    /// The scene goes on (the skip's `FadeToBlack(4)`).
    Resume,
    /// The fly-away's first tick goes on after its `FadeToBlack(ticks(12))`.
    FlyAwaySetup,
    /// The flight starts.
    Flight,
}

/// Audio requests of mode 6 (for `crate::audio::scene`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceAudio {
    /// `music_Pause(0)`.
    PauseMusic,
    /// `music_Unpause`.
    UnpauseMusic,
    /// The dialogue player streams `space_audio[k]` (id 40000 + k).
    Stream(i32),
    /// 0x15172a ∈ {6, 7} → 5: the stream stopped.
    StopStream,
    /// The dialogue player streams `vendor_audio[k]` (id 10000 + k): the weapon demo's sound.
    VendorStream(i32),
    /// `EnterSpaceLoadingLoop`: the flight's sound bank loaded (`snd_bank_load_from_ee_cb`; the engine swaps the
    /// sound data to it: `DoSpaceTransition` stopped every sound and the music before).
    FlightBank,
    /// `SpaceLoadingLoop` tick 1: `snd_play_sound_vol_pan_pmpb(flight bank, v, 0x400, 0, 0, 0)`.
    FlightSound(i32),
}

/// What the engine does after a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceEvent {
    /// The take-off ended: `EnterMenuMode(0)` with kind 0xe (the planet page).
    PlanetPage,
    /// The landing ended: mode 0.
    Landed,
    /// The fly-away ended: 0x15f570 = 1 with 0x15f5c0 = `dest` (`DoSpaceTransition`).
    Leave { dest: i32 },
    /// The flight is over (0x15f5d8 = 1): the transition's loop ends once the level is loaded.
    FlightDone,
    /// A weapon demo ended (its actors deleted, the fade 0): the caller's exit (`VendorExit(1)` / `FUN_002aecf0`).
    DemoDone,
}

/// The frame a weapon demo scene plays in (0x1ca9c0 rows, 0x1caa00 origin, 0x1ca9f8 yaw).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DemoFrame {
    pub rows: [[f32; 4]; 4],
    pub origin: [f32; 4],
    pub yaw: f32,
}

impl DemoFrame {
    /// `VendorStartWeaponDemo` 0x2ae7f8 (and `FUN_002aea70` with its own offset and turn): origin = the moby's rows ·
    /// `offset` + its position, z + 2, then z = `GroundHeight(0.5, origin, 0)`; rotation (0, 0, the moby's yaw + `yaw_add`)
    /// → the rows (`fun_001fa030`).
    pub fn of_moby(w: &World, id: MobyId, offset: [f32; 3], yaw_add: f32) -> DemoFrame {
        let m = w.m(id);
        let o = to_world(&m.rows, m.position, offset);
        let mut origin = [o[0], o[1], o[2] + 2.0, m.position[3]];
        let v = origin.map(crate::ps2v::Pf::f);
        origin[2] = w.ground_height(crate::ps2v::Pf::f(0.5), v, 0).to_f32();
        let yaw = add_rot(m.rotation[2], yaw_add);
        let r = rc_formats::moby_light::rotation_rows([0.0, 0.0, yaw]).map(|r| r.map(f32::from_bits));
        DemoFrame { rows: [r[0], r[1], r[2], [0.0, 0.0, 0.0, 1.0]], origin, yaw }
    }
}

/// An actor's class animation with its streamed slot: (the chunk it was set for, the class, the slot, the flight's
/// streamed frame the slot was last patched from).
pub type ActorAnim = (usize, Arc<rc_formats::moby_anim::MobyAnimClass>, u8, Option<usize>);

/// One actor's pose this frame (ship-local transform applied).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorPose {
    pub actor: usize,
    /// The class as created (actor 0 of class 533 → the ship's class in mode 6).
    pub class: i32,
    pub chunk: usize,
    pub frame_a: u8,
    pub frame_b: u8,
    pub t: f32,
    pub position: [f32; 3],
    /// Euler z (+0x48 = the ship's yaw in the scenes; the flight's actors keep theirs).
    pub yaw: f32,
    /// The table moby (0x16ce58[k]).
    pub moby: Option<MobyId>,
    /// The flight's shrink (moby+0x2c) in the last 56 ticks.
    pub scale: Option<f32>,
    /// Hidden (mode & 1: the flight's first-arrival approach).
    pub hidden: bool,
    /// The glow word (+0x90) when the table moby glows (mode & 0x10: Clank's `FUN_00228bc0`).
    pub glow: Option<u32>,
    /// The flight's pose ([`flight_sequence`]): the streamed frame whose joints 0..3 replace those of the class's
    /// sequence 2 frame 0 (frames a / b then 0).
    pub head: Option<u8>,
}

/// The pose of a flight actor (`SpaceLoadingLoop` 0x2a33b0): with +0x52 / +0x53 = 2 and +0x50 / +0x51 = 0,
/// `update_moby_animation_state` points both frame buffers (+0x68, +0x6c) at the class's sequence 2 frame 0, and the two
/// `FastMemCopy(buffer + 0x10, streamed + 0x10, 0x20)` write the first 0x20 payload bytes (the quaternions of joints
/// 0..3) of streamed frame f, then of f + 1, into that one frame: f + 1's stay. Everything else (the other joints, the
/// scale and translation records, the header) is sequence 2 frame 0's: the canopy stays shut. `MobyBuildMatrix`,
/// `MobyAttachToJoint` (`MobyAnimEvalChain` reads +0x68 / +0x6c) and the draw all see this frame. A one-frame sequence
/// (sequence 2's header); None when the class has no sequence 2 frame. Past the stream's end: its last frame [L].
pub fn flight_sequence(anim: &rc_formats::moby_anim::MobyAnimClass, streamed: &rc_formats::moby_anim::MobySequence, f: usize) -> Option<rc_formats::moby_anim::MobySequence> {
    let base = anim.sequences.get(2)?.as_ref()?;
    let mut frame = base.frames.first()?.clone();
    if let Some(src) = streamed.frames.get(f + 1).or(streamed.frames.last()) {
        let n = 0x20.min(frame.payload.len()).min(src.payload.len());
        frame.payload[..n].copy_from_slice(&src.payload[..n]);
        for j in 0..frame.quats.len().min(4) { frame.quats[j] = frame.quat_at(j); }
    }
    Some(rc_formats::moby_anim::MobySequence { header: base.header, frames: vec![frame], triggers: Vec::new() })
}

/// One frame's output.
#[derive(Clone, Debug, Default)]
pub struct SpaceFrame {
    /// The gameplay tick ran this frame (the engine runs it before [`ShipMode::frame`]).
    pub world_runs: bool,
    /// The scene camera (subs 0 / 8 / 4); None: the play camera (sub 3, the holds keep the last image).
    pub camera: Option<SceneCamera>,
    /// Black coverage over this frame (0x15f3fc, or a hold's accumulated quads).
    pub black: f32,
    /// The white quad 0x15f400.
    pub white: f32,
    pub actors: Vec<ActorPose>,
    pub audio: Vec<SpaceAudio>,
    pub event: Option<SpaceEvent>,
    /// The flight's scenery (sub 4: `DrawWorldPaused`).
    pub flight: Option<FlightDraw>,
    /// Subs 0 / 8: the actors' table mobys with their class animation (streamed slot): their shadows (+0x7f ≠ 0:
    /// `FUN_0026f0e0`, then `MobyProc`'s shadow pass) are cast by the engine's shadow system.
    pub scene_actors: Vec<(MobyId, Arc<rc_formats::moby_anim::MobyAnimClass>)>,
}

/// What `DrawWorldPaused` 0x2a3b90 draws besides the actors in a flight frame (module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlightDraw {
    pub variant: usize,
    /// 0x1604c4 (`0x1be0b0[v]`): the sky shells' z turn ([`flight_shell`]).
    pub sky_turn: f32,
    /// 0x160520: the sky shells' translation (sky units).
    pub shell: [f32; 4],
    /// tan(hfov/2) of the sky draw (the camera's, at least [`FLIGHT_SKY_TAN`]).
    pub sky_tan: f32,
    /// v 4: the planet picture's corners (world; ST 0x1bdd30, RGBA 0x80808080).
    pub planet: Option<[[f32; 3]; 4]>,
    /// v 4 past tick 0x3c: the caption's alpha (0: not drawn).
    pub caption_alpha: u32,
    /// The destination (the picture and the caption are its).
    pub dest: i32,
    /// Not the first-arrival flights: actor 0's trail (FX 0).
    pub trail: bool,
    /// Not the first-arrival flights: actor 0's canopy glass, its class and the joint-list-0 matrix (FX 1).
    pub glass: Option<(i16, [[f32; 4]; 4])>,
}

/// One flight actor class's skeleton for `MobyAttachToJoint`: the class animation and its joint lists (class header
/// +0x1c; the engine reads them from the `spaceships` class blob).
#[derive(Clone, Debug)]
pub struct FlightRig {
    pub anim: rc_formats::moby_anim::MobyAnimClass,
    pub lists: Vec<Vec<u8>>,
    /// The class header's scale (+0x2c of the created moby): the transition lump's classes 531..533 are the flight's
    /// own, so the ship has its scale on a level whose class table lacks the class (Orxon's ship is 532).
    pub scale: f32,
}

/// The mode-6 state (0x13e050.., the scene globals 0x16cd.. while mode 6 uses them, 0x15f3fc / 0x15f400).
#[derive(Clone, Debug)]
pub struct ShipMode {
    pub phase: Phase,
    pub sub: i32,
    /// 0x13e054.
    pub sub_tick: i16,
    pub lump: Option<SpaceLump>,
    /// 0x16cd14 / 0x16cd1c / 0x16cd18.
    pub tick: i32,
    pub chunk: usize,
    pub chunk_tick: i32,
    /// 0x15f3fc / 0x15f400.
    pub fade: f32,
    pub white: f32,
    /// 0x16ce58[k] and the classes they were created with.
    pub actors: Vec<(Option<MobyId>, i32)>,
    /// 0x13e038: the helmet (not created: G-LVL-010).
    pub helmet: Option<MobyId>,
    /// 0x13e05c: the landing at the level start.
    pub level_start: bool,
    /// 0x15f5c0.
    pub dest: i32,
    pub last_camera: Option<SceneCamera>,
    pub last_actors: Vec<ActorPose>,
    /// The fly-away (0x13e040.. / 0x13e06c..).
    pub fly: FlyAway,
    /// The flight (0x13e088 / 0x13e08c, 0x1605b0, 0x160520).
    pub flight: Flight,
    /// 0x1516ec: the stream the entry set, started once its `FadeToBlack` is over (the dialogue player's wait for the
    /// buffered state, then `continue_audio_stream_if_ready`).
    pub stream: Option<i32>,
    /// Per actor: its class animation with the streamed slot, for the chunk it was set for ([`ShipMode::actor_joint`]).
    /// The flight's: the streamed frame its slot was last patched from ([`flight_sequence`]).
    pub actor_anim: Vec<Option<ActorAnim>>,
    /// 0x17c7c0 attached: the Ratchet actor's head manipulator of this scene (`FUN_0024a1d0`; the entry clears it).
    pub head_attached: bool,
    /// A weapon demo's frame ([`SUB_DEMO`]).
    pub demo: Option<DemoFrame>,
}

impl Default for ShipMode {
    fn default() -> Self {
        ShipMode {
            phase: Phase::Idle,
            sub: 0,
            sub_tick: -1,
            lump: None,
            tick: 0,
            chunk: 0,
            chunk_tick: 0,
            fade: 0.0,
            white: 0.0,
            actors: Vec::new(),
            helmet: None,
            level_start: false,
            dest: 0,
            last_camera: None,
            last_actors: Vec::new(),
            fly: FlyAway::default(),
            flight: Flight::default(),
            stream: None,
            actor_anim: Vec::new(),
            head_attached: false,
            demo: None,
        }
    }
}

/// The fly-away's state.
#[derive(Clone, Debug, Default)]
pub struct FlyAway {
    /// 0x13e040 / 0x13e044: Ratchet and Clank on their seats.
    pub riders: [Option<MobyId>; 2],
    /// 0x13e06c: the position along the path (points); 0x13e070: the speed; 0x13e074: the first segment's length.
    pub t: f32,
    pub speed: f32,
    pub segment: f32,
    /// 0x13e078 / 0x13e07c: the camera's ease and its velocity.
    pub cam_t: f32,
    pub cam_v: f32,
    /// 0x13e0b0 / 0x13e0c0: the approach's start (the ship) and end (point 0).
    pub from: [f32; 4],
    pub to: [f32; 4],
    /// 0x13e0d0..: the ship's rotation at the start; 0x13e0e4 / 0x13e0e8: the pitch / yaw of the first segment.
    pub rot_from: [f32; 4],
    pub pitch_to: f32,
    pub yaw_to: f32,
    /// The path with its points' w rewritten to turn rates (the game writes them into the level's path).
    pub path: Vec<[u32; 4]>,
    /// The setup ran (after the first tick's hold).
    pub ready: bool,
}

/// The flight's state.
#[derive(Clone, Debug, Default)]
pub struct Flight {
    /// 0x13e088: the variant (0..=3 random, 4 the approach); 0x13e08c: the count.
    pub variant: usize,
    pub count: i32,
    /// The five variants' scenes (`transition` header[0x14 + v]).
    pub variants: Vec<Option<Arc<Scene>>>,
    /// 0x1605b0: the planet picture's offset (variant 4 moves it toward the camera).
    pub planet: [f32; 4],
    /// 0x160520: the shell translation.
    pub shell: [f32; 4],
    /// The level is loaded (`FUN_00257dc8`'s state ≥ 3: the loader state the end of a variant reads).
    pub loaded: bool,
    /// The first-arrival flights (dest 0, or dest 1 with planet 3 locked).
    pub first_arrival: bool,
    /// 0x15ed84 during the flight: the destination.
    pub dest: i32,
    /// 0x1604c4: the sky's turn.
    pub turn: f32,
    /// The actors' skeletons by class (`MobyAttachToJoint`); a class without one uses the ship's trail points [L].
    pub rigs: std::collections::HashMap<i32, FlightRig>,
}

/// `fun_001f9a40(t, out, a, b)`: a + (b − a)·t.
fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] { std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t) }

impl ShipMode {
    pub fn active(&self) -> bool { self.phase != Phase::Idle }

    fn mirror(&self, g: &mut ShipGlobals) {
        g.sub = self.sub;
        g.tick = self.sub_tick;
    }

    /// `ShipTakeOff` 0x2a27c8 (its audio and the help kill; the HP clamp and the hero hide are the caller's World
    /// writes here).
    pub fn take_off(&mut self, w: &mut World, lump: SpaceLump, out: &mut SpaceFrame) {
        if w.hero.health < 1 { w.hero_fields_mut().health = 1; }
        out.audio.push(SpaceAudio::PauseMusic);
        out.audio.push(SpaceAudio::StopStream);
        w.svc.help.kill();
        self.enter(w, SUB_TAKEOFF, lump, out);
        self.level_start = false;
    }

    /// `EnterShipMode(sub)` 0x2a24b8 with its lump already read (module docs).
    pub fn enter(&mut self, w: &mut World, sub: i32, lump: SpaceLump, out: &mut SpaceFrame) {
        self.sub_tick = -1;
        self.helmet = None;
        self.actor_anim.clear();
        // The scene state 0x16cce0 / 0x17c7c0 / 0x17c800 cleared: the actors' manipulator records.
        self.head_attached = false;
        self.sub = sub;
        self.tick = 0;
        self.chunk = 0;
        self.chunk_tick = 0;
        self.actors.clear();
        self.last_actors.clear();
        self.last_camera = None;
        self.fade = 1.0;
        self.white = 0.0;
        w.svc.game_mode = 6;
        crate::cinematic::hero_state(w, crate::scene_player::HERO_SCENE_STATE, true);
        // 0x1413f5 = 1 after the SetState (which clears it): Ratchet's held objects hide (the Bomb Glove's bomb).
        w.hero_fields_mut().hero_hidden = Some(1);
        if let Some(id) = w.svc.travel.moby { w.mm(id).mode |= 1; }
        for m in w.table.mobys.iter_mut() {
            if m.state & 0x80 == 0 && HIDDEN_CLASSES.contains(&m.o_class) { m.mode |= 0x80; }
        }
        let ship = w.svc.travel.ship;
        let level = w.svc.level as i32;
        let o2 = w.hero.owned.0.get(ITEM_O2_MASK).is_some_and(|&b| b != 0);
        let (_, stream) = lump_and_stream(sub, level, ship, o2, self.level_start);
        let fade = if sub == SUB_TAKEOFF || !self.level_start { w.ticks(6) as u32 } else { 0 };
        self.lump = Some(lump);
        self.parse_chunk(w, 0);
        let _ = out;
        self.stream = Some(stream);
        self.phase = if fade > 0 { Phase::Hold { n: fade, k: 0, then: After::Scene } } else { Phase::Wait };
        self.mirror(&mut w.svc.travel);
    }

    /// `ShipTravelTo(dest)` 0x2a2848 (the save `memcard_Save(0, dest)` is the caller's: `EngineRequest::Save`).
    /// Returns whether the destination is the current planet (the revisit landing).
    pub fn travel_to(&mut self, w: &mut World, dest: i32, landing: Option<SpaceLump>, out: &mut SpaceFrame) -> bool {
        w.svc.game_mode = 6;
        self.fade = 1.0;
        self.white = 0.0;
        self.sub = 0;
        self.sub_tick = -1;
        if w.svc.level as i32 == dest {
            if let Some(l) = landing {
                self.level_start = false;
                self.enter(w, SUB_LANDING, l, out);
                let s = ship_index(w.svc.travel.ship);
                self.tick = w.ticks(LANDING_REVISIT_TICK[s]);
                self.chunk = (self.tick / CHUNK_TICKS) as usize;
                self.parse_chunk(w, self.chunk);
                self.chunk_tick = self.tick % CHUNK_TICKS;
            }
            return true;
        }
        self.sub_tick = -1;
        self.sub = SUB_FLYAWAY;
        self.dest = dest;
        self.fly = FlyAway::default();
        self.phase = Phase::Run;
        self.mirror(&mut w.svc.travel);
        false
    }

    /// `ShipLandingStart` 0x2a29c0, the level's `entry` after its init (module docs): Some(mode 6 runs) or None (mode 0).
    pub fn landing_start(&mut self, w: &mut World, lump: Option<SpaceLump>, flag_13d3f0: bool, out: &mut SpaceFrame) -> bool {
        self.level_start = true;
        w.svc.travel.reset_trip = false;
        let level = w.svc.level as i32;
        let p3 = w.svc.interact.game.planet_unlocked.get(3).is_some_and(|&b| b != 0);
        if (level == 1 && !p3) || level == 0 || (level == 14 && !flag_13d3f0) || level > 0x13 {
            w.svc.game_mode = 0;
            self.phase = Phase::Idle;
            return false;
        }
        let Some(l) = lump else {
            w.svc.game_mode = 0;
            self.phase = Phase::Idle;
            return false;
        };
        self.sub_tick = -1;
        w.svc.game_mode = 6;
        self.fade = 1.0;
        self.sub = 0;
        self.enter(w, SUB_LANDING, l, out);
        true
    }

    /// `parse_space_scene_chunk(k)` / `FUN_00259288` in mode 6: the chunk's actors created on first sight (actor 0 of
    /// class 533 → the ship's class; +0x32 = 0x1ff, +0x72 = 0xff, mode \|= 6 (the port adds the hidden bit: it draws
    /// them itself), +0x94 = 0, the hero's light words or 0x38383800000000, +0x73 = 0x18 when class +6).
    fn parse_chunk(&mut self, w: &mut World, k: usize) {
        self.chunk = k;
        self.chunk_tick = 0;
        let Some(lump) = self.lump.clone() else { return };
        let Some(chunk) = lump.scene.chunks.get(k) else { return };
        let ship = ship_index(w.svc.travel.ship);
        for (i, a) in chunk.actors.iter().enumerate() {
            if self.actors.get(i).is_some_and(|x| x.0.is_some()) { continue; }
            let class = if i == 0 && a.class == 533 { SHIP_CLASSES[ship] } else { a.class };
            let id = w.create_moby(class as i16);
            if let Some(id) = id {
                let hl = w.hero_moby.and_then(|h| w.table.mobys.get(h)).map(|h| (h.light, h.ambient));
                let b06 = w.classes.info(class as i16).map_or(0, |i| i.b06);
                let rig_scale = self.flight.rigs.get(&class).filter(|_| w.classes.info(class as i16).is_none()).map(|r| r.scale);
                let m = w.mm(id);
                if b06 != 0 { m.b73 = 0x18; }
                m.draw_dist = 0x1ff;
                m.b72 = 0xff;
                m.mode |= 6 | 1;
                m.has_collision = false;
                if let Some(sc) = rig_scale { m.scale = sc; }
                match hl {
                    Some((l, a)) => (m.light, m.ambient) = (l, a),
                    None => (m.light, m.ambient) = (0, [0x38, 0x38, 0x38, 0]),
                }
            }
            if self.actors.len() <= i { self.actors.resize(i + 1, (None, 0)); }
            self.actors[i] = (id, class);
        }
    }

    /// One frame of mode 6 after the engine's world part (module docs).
    pub fn frame(&mut self, w: &mut World, pressed: u32, world_ran: bool) -> SpaceFrame {
        let mut out = SpaceFrame { world_runs: world_ran, ..Default::default() };
        match self.phase {
            Phase::Idle => {}
            Phase::Hold { n, k, then } => {
                out.black = fade_to_black_coverage(n, k.min(n - 1));
                out.camera = self.last_camera;
                out.actors = self.last_actors.clone();
                self.phase = if k + 1 < n { Phase::Hold { n, k: k + 1, then } } else {
                    match then {
                        After::Scene => Phase::Wait,
                        After::Resume | After::Flight => Phase::Run,
                        After::FlyAwaySetup => {
                            if let Some(k) = self.stream.take() { out.audio.push(SpaceAudio::Stream(k)); }
                            self.fly_away_setup(w);
                            Phase::Run
                        }
                    }
                };
            }
            Phase::Wait => {
                out.black = 1.0;
                if let Some(k) = self.stream.take() {
                    out.audio.push(if self.sub == SUB_DEMO { SpaceAudio::VendorStream(k - VENDOR_AUDIO_BASE) } else { SpaceAudio::Stream(k) });
                }
                self.phase = Phase::Run;
                // The demo's first frame shows its fade 0 (set before the lump's `FadeToBlack(4)`).
                if self.sub == SUB_DEMO { out.black = self.fade; }
            }
            Phase::Run => match self.sub {
                SUB_TAKEOFF | SUB_LANDING => self.scene_tick(w, pressed, &mut out),
                SUB_DEMO => self.demo_tick(w, &mut out),
                SUB_FLYAWAY => self.fly_away_tick(w, pressed, &mut out),
                SUB_FLIGHT => self.flight_tick(w, &mut out),
                _ => {}
            },
        }
        if out.black == 0.0 && self.phase == Phase::Run && self.sub != SUB_FLIGHT { out.black = self.fade.clamp(0.0, 1.0); }
        if self.phase == Phase::Run && self.sub == SUB_FLIGHT { out.black = self.fade.clamp(0.0, 1.0); }
        out.white = self.white;
        self.mirror(&mut w.svc.travel);
        out
    }

    /// `VendorStartWeaponDemo` 0x2ae7f8's scene part (and `FUN_002aea70`'s): the scene state cleared (0x16cce0,
    /// 0x17c7c0), the fade 0x15f3fc = 0 and the white 0, the lump read (`FUN_002594e0`: `unknown_1530[scene]`, its
    /// `FadeToBlack(4)`), the stream 0x1516ec = `stream`, chunk 0 parsed, the stream waited for and started; then
    /// [`ShipMode::frame`] plays it ([`SUB_DEMO`]). The game mode stays the caller's (5).
    pub fn demo_start(&mut self, w: &mut World, lump: SpaceLump, frame: DemoFrame, stream: i32) {
        self.sub = SUB_DEMO;
        self.sub_tick = -1;
        self.helmet = None;
        self.tick = 0;
        self.chunk = 0;
        self.chunk_tick = 0;
        self.actors.clear();
        self.actor_anim.clear();
        self.last_actors.clear();
        self.last_camera = None;
        self.head_attached = false;
        self.fade = 0.0;
        self.white = 0.0;
        self.demo = Some(frame);
        self.level_start = false;
        self.lump = Some(lump);
        self.parse_chunk(w, 0);
        self.stream = Some(stream);
        self.phase = Phase::Hold { n: 4, k: 0, then: After::Scene };
    }

    /// `VendorModeUpdate` substate 3 (module docs of [`SUB_DEMO`]): after the world part (the gameplay tick's scene
    /// form) the chunk and scene ticks + 1; the fade: past `end − 12` + 0.1 (≤ 1), else − 0.125 (≥ 0); before the end the
    /// chunk roll (≥ 0x60), the camera of the record (its tan; eye and Euler z in the demo's frame), the actors (frames,
    /// t = (chunk tick & 1)·0.5, 1.0 on an odd tick of a cut record; position in the demo's frame, +0x71 = 0xff, yaw =
    /// the frame's, `MobyBuildMatrix`, +0x7f ≠ 0 → `FUN_0026f0e0`, class 0 → `FUN_0024a1d0`), `UpdateAllPointLights`
    /// (the engine's world lights), `IncrementTickCounter`; at the end the fade 0, tan 0.63, the actors deleted (their
    /// class slots freed) and [`SpaceEvent::DemoDone`].
    fn demo_tick(&mut self, w: &mut World, out: &mut SpaceFrame) {
        let (Some(lump), Some(fr)) = (self.lump.clone(), self.demo) else { return };
        let scene = &lump.scene;
        self.sub_tick = self.sub_tick.wrapping_add(1);
        self.chunk_tick += 1;
        self.tick += 1;
        let end = scene.end_tick();
        if end - 0xc < self.tick {
            self.fade += 0.1;
            if 1.0 < self.fade { self.fade = 1.0; }
        } else {
            self.fade -= 0.125;
            if self.fade < 0.0 { self.fade = 0.0; }
        }
        if end <= self.tick {
            self.fade = 0.0;
            for (id, _) in std::mem::take(&mut self.actors) {
                if let Some(id) = id { w.delete_moby(id); }
            }
            if let Some(h) = self.helmet.take() { w.delete_moby(h); }
            self.last_actors.clear();
            self.last_camera = None;
            self.demo = None;
            out.camera = None;
            out.event = Some(SpaceEvent::DemoDone);
            self.phase = Phase::Idle;
            return;
        }
        if CHUNK_TICKS <= self.chunk_tick {
            let c = self.chunk + 1;
            self.parse_chunk(w, c);
        }
        let Some(chunk) = scene.chunks.get(self.chunk) else { return };
        let Some(rec) = chunk.camera.get(self.chunk_tick as usize).copied() else { return };
        let mut r = rec;
        r.eye = to_world(&fr.rows, fr.origin, rec.eye);
        r.angles[2] = add_rot(rec.angles[2], fr.yaw);
        let cam = crate::scene_player::scene_camera(&r);
        self.last_camera = Some(cam);
        out.camera = Some(cam);
        let cut = rec.is_cut();
        let f = (self.chunk_tick >> 1) as usize;
        let mut poses = Vec::new();
        for (k, a) in chunk.actors.iter().enumerate() {
            let (id, class) = self.actors.get(k).copied().unwrap_or((None, a.class));
            let mut t = (self.chunk_tick & 1) as f32 * 0.5;
            if cut && self.chunk_tick & 1 != 0 { t = 1.0; }
            let p0 = a.positions.get(f).copied().unwrap_or([0.0; 4]);
            let p1 = a.positions.get(f + 1).copied().unwrap_or(p0);
            let local = [p0[0] * (1.0 - t) + p1[0] * t, p0[1] * (1.0 - t) + p1[1] * t, p0[2] * (1.0 - t) + p1[2] * t];
            let pos = to_world(&fr.rows, fr.origin, local);
            if let Some(id) = id {
                let m = w.mm(id);
                (m.position[0], m.position[1], m.position[2]) = (pos[0], pos[1], pos[2]);
                m.b71 = 0xff;
                m.rotation[2] = fr.yaw;
                m.anim.frame_a = f as u8;
                m.anim.frame_b = (f + 1) as u8;
                m.anim.t = t;
                match self.actor_anim_for(w, k, class, Some(&a.sequence)) {
                    Some((anim, slot)) => {
                        let m = w.mm(id);
                        (m.anim.seq_a, m.anim.seq_b) = (slot, slot);
                        build_with(w, id, Some(&anim));
                        out.scene_actors.push((id, anim));
                    }
                    None => w.build_matrix(id),
                }
                if class == 0 { self.ratchet_actor(w, id); }
            }
            let glow = id.and_then(|i| (w.m(i).mode & 0x10 != 0).then(|| w.m(i).glow));
            poses.push(ActorPose { actor: k, class, chunk: self.chunk, frame_a: f as u8, frame_b: (f + 1) as u8, t, position: pos, yaw: fr.yaw, moby: id, scale: None, hidden: false, glow, head: None });
        }
        self.last_actors = poses.clone();
        out.actors = poses;
    }

    /// `GameStateUpdate` subs 0 / 8 (module docs).
    fn scene_tick(&mut self, w: &mut World, pressed: u32, out: &mut SpaceFrame) {
        let Some(lump) = self.lump.clone() else { return };
        let scene = &lump.scene;
        self.sub_tick = self.sub_tick.wrapping_add(1);
        self.fade -= 0.125;
        self.chunk_tick += 1;
        self.tick += 1;
        if self.fade < 0.0 { self.fade = 0.0; }
        let end = scene.end_tick();
        let ship = ship_index(w.svc.travel.ship);
        let mut skip = w.svc.travel.autoskip;
        if pressed & 0x50 != 0 && w.ticks(30) < self.tick && self.fade == 0.0 {
            if self.sub == SUB_TAKEOFF && self.tick < w.ticks(TAKEOFF_SKIP_TICK[ship] - 30) { skip = true; }
            if self.sub == SUB_LANDING && self.tick < end - w.ticks(30) { skip = true; }
        }
        if skip {
            out.audio.push(SpaceAudio::StopStream);
            if self.sub == SUB_TAKEOFF {
                self.tick = w.ticks(TAKEOFF_SKIP_TICK[ship]);
                let c = (self.tick / CHUNK_TICKS) as usize;
                self.parse_chunk(w, c);
                self.chunk_tick = self.tick % CHUNK_TICKS;
                let auto = w.svc.travel.autoskip;
                w.svc.travel.autoskip = false;
                self.fade = 1.0;
                if !auto {
                    // FadeToBlack(4) inside the update: four black quads over the last image (this frame shows the first),
                    // then the update goes on from the skip tick on the next frame [L: the rest of this tick's camera and
                    // actor update is not drawn: the game draws it under fade 1.0, i.e. black].
                    self.phase = Phase::Hold { n: 4, k: 1, then: After::Resume };
                    out.black = fade_to_black_coverage(4, 0);
                    out.camera = self.last_camera;
                    out.actors = self.last_actors.clone();
                    return;
                }
            } else {
                self.tick = end;
            }
        }
        if self.tick >= end {
            self.scene_end(w, out);
            return;
        }
        if self.chunk_tick >= CHUNK_TICKS {
            let c = self.chunk + 1;
            self.parse_chunk(w, c);
        }
        let Some(chunk) = scene.chunks.get(self.chunk) else { return };
        let Some(rec) = chunk.camera.get(self.chunk_tick as usize).copied() else { return };
        let Some(sid) = w.svc.travel.moby else { return };
        let (spos, srows, syaw) = { let m = w.m(sid); (m.position, m.rows, m.rotation[2]) };
        // Ship-local camera: the eye through the ship's rows, the z angle + its yaw.
        let mut r = rec;
        r.eye = to_world(&srows, spos, rec.eye);
        r.angles[2] = add_rot(rec.angles[2], syaw);
        let mut cam = crate::scene_player::scene_camera(&r);
        // The mirror cheat 0x15edb4: `FastVecCross(left, up, forward)` (the left row = up × forward).
        if w.svc.cheats.on(crate::cheats::slot::MIRROR) {
            let [f, _, u] = cam.rows;
            cam.rows[1] = [u[1] * f[2] - u[2] * f[1], u[2] * f[0] - u[0] * f[2], u[0] * f[1] - u[1] * f[0]];
        }
        self.last_camera = Some(cam);
        out.camera = Some(cam);
        let cut = rec.is_cut();
        let f = (self.chunk_tick >> 1) as usize;
        let mut poses = Vec::new();
        for (k, a) in chunk.actors.iter().enumerate() {
            let (id, class) = self.actors.get(k).copied().unwrap_or((None, a.class));
            let mut t = (self.chunk_tick & 1) as f32 * 0.5;
            if cut && self.chunk_tick & 1 != 0 { t = 1.0; }
            let p0 = a.positions.get(f).copied().unwrap_or([0.0; 4]);
            let p1 = a.positions.get(f + 1).copied().unwrap_or(p0);
            let local = [p0[0] * (1.0 - t) + p1[0] * t, p0[1] * (1.0 - t) + p1[1] * t, p0[2] * (1.0 - t) + p1[2] * t];
            let pos = to_world(&srows, spos, local);
            if let Some(id) = id {
                let m = w.mm(id);
                (m.position[0], m.position[1], m.position[2]) = (pos[0], pos[1], pos[2]);
                m.b71 = 0xff;
                m.rotation[2] = syaw;
                m.anim.frame_a = f as u8;
                m.anim.frame_b = (f + 1) as u8;
                m.anim.t = t;
                // +0x52 / +0x53: the class's streamed slot (`FUN_00259288`); MobyBuildMatrix (the rows from the ship's yaw,
                // the bounding sphere of the streamed sequence the flames and the shadow read).
                match self.actor_anim_for(w, k, class, Some(&a.sequence)) {
                    Some((anim, slot)) => {
                        let m = w.mm(id);
                        (m.anim.seq_a, m.anim.seq_b) = (slot, slot);
                        build_with(w, id, Some(&anim));
                        out.scene_actors.push((id, anim));
                    }
                    None => w.build_matrix(id),
                }
                self.actor_fx(w, id, class, k, &a.sequence);
            }
            poses.push(ActorPose { actor: k, class, chunk: self.chunk, frame_a: f as u8, frame_b: (f + 1) as u8, t, position: pos, yaw: syaw, moby: id, scale: None, hidden: false, glow: id.and_then(|i| (w.m(i).mode & 0x10 != 0).then(|| w.m(i).glow)), head: None });
        }
        self.last_actors = poses.clone();
        out.actors = poses;
    }

    /// The per-class part of the actor loop (module docs).
    fn actor_fx(&mut self, w: &mut World, id: MobyId, class: i32, k: usize, seq: &rc_formats::moby_anim::MobySequence) {
        let tick = self.tick;
        let level = w.svc.level;
        let o2 = w.hero.owned.0.get(ITEM_O2_MASK).is_some_and(|&b| b != 0);
        let takeoff = self.sub == SUB_TAKEOFF;
        match class {
            0 => {
                let b7f = if takeoff {
                    if (level == 10 && !o2) || w.ticks(0x3e) < tick { 0 } else { 0x18 }
                } else if w.ticks(0x168) < tick {
                    if level == 10 && !o2 { 0 } else { 0x18 }
                } else {
                    0
                };
                w.mm(id).b7f = b7f;
                self.ratchet_actor(w, id);
                // On level 10 with the O2 mask, and on level 13: the helmet on Ratchet's head.
                if (level == 10 && o2) || level == 13 { self.helmet(w, id, k, class, seq); }
            }
            10 => {
                let b7f = if takeoff {
                    if tick <= w.ticks(0x15e) { 0x18 } else { 0 }
                } else if w.ticks(0x20c) < tick {
                    0x18
                } else {
                    0
                };
                w.mm(id).b7f = b7f;
                clank_actor(w, id);
            }
            c if (531..=533).contains(&c) => ship_fx(self, w, id, k),
            _ => {}
        }
    }

    /// `FUN_0024a1d0` on the Ratchet actor: the head manipulator (record 0x17c7c0, joint list 0x18) attached once per
    /// scene with Ratchet's head scale 0x15ee14 (at most 1.33 in game mode 6); on levels 2, 9, 11 and 13 the actors of
    /// classes 0x1b1 / 0x50a / 0x509 take their class scale × that scale. `FUN_0024a0e8`'s part (game modes 2 / 6:
    /// 0x509 / 0x50a actors mode \| 0x800, the hero's worn items attached to and shown on this actor,
    /// `HeroComputeAttachMatrices` / `HeroItemsAttach` / `FUN_00249ea0`) is NOT ported (G-CUT-009).
    fn ratchet_actor(&mut self, w: &mut World, id: MobyId) {
        let mut s = w.hero.idle.rec17_scale;
        if w.svc.game_mode == 6 && 1.33 < s { s = 1.33; }
        if !self.head_attached {
            self.head_attached = true;
            match crate::moby_update::manip::target_joint(w, id, 0x18) {
                Some(joint) => {
                    let node = rc_formats::moby_anim::JointModifier { joint, mode: 0, weight: 0.0, quat: [0.0, 0.0, 0.0, 1.0], scale: [s; 3], trans: [0.0; 3] };
                    let m = w.mm(id);
                    if m.joint_mod_keys.len() != m.joint_mods.len() { m.joint_mod_keys.resize(m.joint_mods.len(), u32::MAX); }
                    m.joint_mods.insert(0, node);
                    m.joint_mod_keys.insert(0, ACTOR_HEAD_KEY);
                }
                None => w.svc.unported("mode-6 Ratchet actor: joint list 0x18 not loaded"),
            }
        }
        if matches!(w.svc.level, 2 | 9 | 11 | 13) {
            for (a, _) in self.actors.clone() {
                let Some(a) = a else { continue };
                let oc = w.m(a).o_class;
                if matches!(oc, 0x1b1 | 0x50a | 0x509) {
                    let base = w.classes.info(oc).map_or(1.0, |i| i.scale);
                    w.mm(a).scale = base * s;
                }
            }
        }
    }

    /// The helmet `CreateMoby(0x509)` (0x13e038, once per scene: +0x32 = 0x40, mode \| 0x806, the actor's light words,
    /// +0x73 = 0x18 when its class +6) and `FUN_0024a310(actor, helmet)`: placed on the actor's joint list 4
    /// (`MobyAttachToJoint`: its point and rows, the bounding sphere), posed from the actor's head joints 8 and 10
    /// (`HeroItemPoseFromRatchet(0x15f6a8, …)`: `hero::worn::pose_from_host`, key A the synthesized frame, t 0).
    fn helmet(&mut self, w: &mut World, actor: MobyId, k: usize, class: i32, seq: &rc_formats::moby_anim::MobySequence) {
        if self.helmet.is_none() {
            let Some(h) = w.create_moby(HELMET_CLASS) else { return };
            let (light, ambient) = { let a = w.m(actor); (a.light, a.ambient) };
            let b06 = w.classes.info(HELMET_CLASS).map_or(0, |i| i.b06);
            let m = w.mm(h);
            m.draw_dist = 0x40;
            m.mode |= 0x806;
            (m.light, m.ambient) = (light, ambient);
            if b06 != 0 { m.b73 = 0x18; }
            m.has_collision = false;
            self.helmet = Some(h);
        }
        let Some(h) = self.helmet else { return };
        let Some(mtx) = self.actor_joint(w, k, class, Some(seq), actor, crate::hero::worn::HEAD_ATTACH) else { return };
        {
            let m = w.mm(h);
            (m.position[0], m.position[1], m.position[2]) = (mtx[3][0], mtx[3][1], mtx[3][2]);
            m.rows[..3].copy_from_slice(&mtx[..3]);
        }
        let Some((anim, slot)) = self.actor_anim_for(w, k, class, Some(seq)) else { return };
        let am = w.m(actor);
        let host = rc_formats::moby_anim::AnimState { seq_a: slot, frame_a: am.anim.frame_a, seq_b: slot, frame_b: am.anim.frame_b, t: am.anim.t, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: true };
        let frame = w.classes.anim(HELMET_CLASS).and_then(|item| crate::hero::worn::pose_from_host(&anim, &host, None, item, &crate::hero::worn::HEAD_JOINTS, 0));
        if w.svc.snapshots.len() <= h { w.svc.snapshots.resize(h + 1, None); }
        w.svc.snapshots[h] = frame;
        // moby_anim_sphere_lerp (the bounding sphere of the placed helmet, its +0x52 / +0x53 0), then +0x50..+0x54 = 0 and
        // the frame pointers at the synthesized frame (here: key A the snapshot slot, t 0).
        {
            let m = w.mm(h);
            (m.anim.seq_a, m.anim.frame_a, m.anim.seq_b, m.anim.frame_b, m.anim.t) = (0, 0, 0, 0, 0.0);
        }
        crate::moby_update::creature::react::sphere_lerp(w, h);
        let st = crate::hero::worn::posed_state();
        let m = w.mm(h);
        (m.anim.seq_a, m.anim.frame_a, m.anim.seq_b, m.anim.frame_b, m.anim.t) = (st.seq_a, st.frame_a, st.seq_b, st.frame_b, st.t);
    }

    /// The end of a take-off / landing scene (module docs).
    fn scene_end(&mut self, w: &mut World, out: &mut SpaceFrame) {
        out.audio.push(SpaceAudio::StopStream);
        for (id, _) in std::mem::take(&mut self.actors) {
            if let Some(id) = id { w.delete_moby(id); }
        }
        if let Some(h) = self.helmet.take() { w.delete_moby(h); }
        if let Some(id) = w.svc.travel.moby { w.mm(id).mode &= !1; }
        for m in w.table.mobys.iter_mut() {
            if HIDDEN_CLASSES.contains(&m.o_class) { m.mode &= !0x80; }
        }
        self.last_actors.clear();
        self.last_camera = None;
        out.camera = None;
        if self.sub != SUB_TAKEOFF {
            w.svc.game_mode = 0;
            out.audio.push(SpaceAudio::UnpauseMusic);
            if self.level_start {
                walk_back(w);
            } else if let Some((pos, rot)) = w.svc.travel.landing_spot {
                crate::cinematic::hero_teleport(w, [pos[0], pos[1], pos[2]], [rot[0], rot[1], rot[2]], 0, true);
            }
            out.event = Some(SpaceEvent::Landed);
        } else {
            out.event = Some(SpaceEvent::PlanetPage);
        }
        self.phase = Phase::Idle;
        self.fade = 0.0;
    }

    /// Sub 3, the first tick (module docs): the stream and the hold; the setup runs after it.
    fn fly_away_tick(&mut self, w: &mut World, pressed: u32, out: &mut SpaceFrame) {
        self.sub_tick = self.sub_tick.wrapping_add(1);
        if self.sub_tick == 0 && !self.fly.ready {
            out.audio.push(SpaceAudio::PauseMusic);
            self.stream = Some(FLYAWAY_STREAM + w.svc.travel.ship as i32);
            self.phase = Phase::Hold { n: w.ticks(12) as u32, k: 0, then: After::FlyAwaySetup };
            out.black = self.fade.clamp(0.0, 1.0);
            return;
        }
        self.fly_away_body(w, pressed, out);
    }

    /// The rest of the first tick after `FadeToBlack(ticks(12))` (module docs).
    fn fly_away_setup(&mut self, w: &mut World) {
        self.fly.ready = true;
        let Some(sid) = w.svc.travel.moby else { return };
        {
            let m = w.mm(sid);
            m.anim.seq_a = 1;
            m.anim.seq_b = 2;
            m.anim.t = 0.0;
            m.b72 = 0xff;
            m.has_collision = false;
        }
        let setup = w.svc.travel.setup;
        if setup.path >= 0 {
            let hl = w.hero_moby.and_then(|h| w.table.mobys.get(h)).map(|h| (h.light, h.ambient));
            for (k, class) in [0i16, 10].into_iter().enumerate() {
                let id = w.create_moby(class);
                if let Some(id) = id {
                    let m = w.mm(id);
                    m.draw_dist = 0x1ff;
                    m.b72 = 0xff;
                    m.has_collision = false;
                    m.mode |= 6;
                    if let Some((l, a)) = hl { (m.light, m.ambient) = (l, a); }
                }
                self.fly.riders[k] = id;
            }
        }
        // CameraScript(0x167240, 0x167250, 1, 0, 0): the script camera from the current view.
        let cam = w.camera_point();
        let e = w.hero.loop_in.cam_euler;
        crate::cinematic::camera_script(w, cam, e, 1, 0, false);
        self.fly.cam_t = 0.0;
        self.fly.cam_v = 0.0;
        if setup.path < 0 { return; }
        let Some(src) = w.svc.splines.get(setup.path as usize).cloned() else { return };
        let mut pts = src;
        let n = pts.len();
        if n < 2 { return; }
        let p = |pts: &Vec<[u32; 4]>, i: usize| pts[i].map(f32::from_bits);
        let heading = |a: [f32; 4], b: [f32; 4]| atan(b[0] - a[0], b[1] - a[1]);
        pts[0][3] = 0f32.to_bits();
        let mut max = 0.0f32;
        for i in 1..n.saturating_sub(1) {
            let a = heading(p(&pts, i - 1), p(&pts, i));
            let b = heading(p(&pts, i), p(&pts, i + 1));
            let d = sub_rot(b, a);
            pts[i][3] = d.to_bits();
            if max < d.abs() { max = d.abs(); }
        }
        let a = heading(p(&pts, n - 2), p(&pts, n - 1));
        let b = heading(p(&pts, n - 1), p(&pts, 0));
        pts[n - 1][3] = sub_rot(b, a).to_bits();
        let k = 0.349_065_84 / max;
        for q in pts.iter_mut() { q[3] = (f32::from_bits(q[3]) * k).to_bits(); }
        let (p0, p1) = (p(&pts, 0), p(&pts, 1));
        self.fly.segment = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2) + (p1[2] - p0[2]).powi(2)).sqrt();
        self.fly.t = 0.0;
        self.fly.speed = 0.0;
        w.svc.travel.trail.head = 0;
        w.svc.travel.trail.count = 0;
        let m = w.m(sid);
        self.fly.from = m.position;
        self.fly.to = p0;
        self.fly.rot_from = m.rotation;
        let d2 = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt();
        self.fly.pitch_to = atan(d2, p0[2] - p1[2]);
        self.fly.yaw_to = atan(p1[0] - p0[0], p1[1] - p0[1]);
        self.fly.path = pts;
    }

    /// Sub 3 after the setup (module docs).
    fn fly_away_body(&mut self, w: &mut World, pressed: u32, out: &mut SpaceFrame) {
        let setup = w.svc.travel.setup;
        if setup.cam_a >= 0 && setup.cam_b >= 0 {
            crate::moby_update::creature::turn::spring(1.0, DT2 * 0.666, DT2 * 0.666, DT * 0.5, &mut self.fly.cam_t, &mut self.fly.cam_v);
            let shape = |c: i32| w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c).map(|s| (s.centre(), s.euler));
            if let (Some((pa, ea)), Some((pb, eb))) = (shape(setup.cam_a), shape(setup.cam_b)) {
                let f = self.fly.cam_t;
                let pos = [pa[0] + (pb[0] - pa[0]) * f, pa[1] + (pb[1] - pa[1]) * f, pa[2] + (pb[2] - pa[2]) * f];
                let euler = [0.0, add_rot(sub_rot(eb[1], ea[1]) * f, ea[1]), add_rot(sub_rot(eb[2], ea[2]) * f, ea[2])];
                crate::cinematic::camera_targets(w, Some(pos), Some(euler));
            }
        }
        let Some(sid) = w.svc.travel.moby else { return };
        if setup.path >= 0 {
            let st = self.sub_tick as i32;
            if st < w.ticks(0x96) {
                let mut f = (1.0 - (st as f32 * (std::f32::consts::PI / w.ticks(0x78) as f32)).cos()) * 0.5;
                if w.ticks(0x78) <= st { f = 1.0; }
                let (from, to, r0) = (self.fly.from, self.fly.to, self.fly.rot_from);
                let (pitch_to, yaw_to) = (self.fly.pitch_to, self.fly.yaw_to);
                {
                    let m = w.mm(sid);
                    m.anim.t = f;
                    m.position = lerp4(from, to, f);
                    m.rotation[0] = 0.0;
                    m.rotation[1] = add_rot(r0[1], sub_rot(pitch_to, r0[1]) * f);
                    m.rotation[2] = add_rot(r0[2], sub_rot(yaw_to, r0[2]) * f);
                }
                if self.fade > 0.0 {
                    self.fade -= 0.125;
                    if self.fade < 0.0 { self.fade = 0.0; }
                }
                if f < 1.0 { ship::exhaust(w, DT * -3.75, sid); }
            } else {
                self.fly.speed += 0.8;
                if 100.0 < self.fly.speed { self.fly.speed = 100.0; }
                let n = self.fly.path.len() as f32;
                if pressed & 0x50 != 0 && self.fade < 0.0625 { self.fade = 0.0625; }
                if n - 6.0 < self.fly.t && self.fade < 0.0625 { self.fade = 0.0625; }
                if w.svc.travel.ship < 2 {
                    let m = w.mm(sid);
                    m.cmd = 0x32;
                    m.spawn_id = 10;
                    ship::register_flames(w.svc, sid);
                }
                self.fly.t += (self.fly.speed * DT) / self.fly.segment;
                if n - 1.0 < self.fly.t || 1.0 <= self.fade {
                    out.event = Some(SpaceEvent::Leave { dest: self.dest });
                    self.phase = Phase::Idle;
                } else {
                    let (pos, rot) = crate::path::pose(&self.fly.path, true, self.fly.t, true);
                    {
                        let m = w.mm(sid);
                        m.position = pos;
                        m.rotation = rot;
                        m.rotation[0] = m.rotation[3];
                    }
                    w.build_matrix(sid);
                    let s = ship_index(w.svc.travel.ship);
                    let (rows, p) = (w.m(sid).rows, w.m(sid).position);
                    let (a, b) = (to_world(&rows, p, TRAIL_A[s]), to_world(&rows, p, TRAIL_B[s]));
                    w.svc.travel.trail.push(a, b);
                    w.svc.draw_callbacks.register(crate::moby_update::classes::draw_callbacks::Callback::ShipTrail, sid);
                    if 0.0 < self.fade {
                        self.fade += 0.0625;
                        if 1.0 < self.fade { self.fade = 1.0; }
                    }
                }
            }
            w.build_matrix(sid);
            let s = ship_index(w.svc.travel.ship);
            let (rows, p, rot) = (w.m(sid).rows, w.m(sid).position, w.m(sid).rotation);
            for (k, seat) in [RIDER_RATCHET[s], RIDER_CLANK[s]].into_iter().enumerate() {
                let Some(r) = self.fly.riders[k] else { continue };
                let q = to_world(&rows, p, seat);
                {
                    let m = w.mm(r);
                    m.rotation = rot;
                    (m.position[0], m.position[1], m.position[2]) = (q[0], q[1], q[2]);
                }
                w.build_matrix(r);
            }
        }
    }

    /// `EnterSpaceLoadingLoop` 0x2a5868 with the variants' scenes loaded (module docs). `rand` = the game's stream;
    /// `dest` = 0x15ed84 (the destination), `rigs` = the actor classes' skeletons. The flight's sound bank:
    /// [`SpaceAudio::FlightBank`] in `out`.
    pub fn flight_start(&mut self, w: &mut World, variants: Vec<Option<Arc<Scene>>>, first_arrival: bool, dest: i32, rigs: std::collections::HashMap<i32, FlightRig>, out: &mut SpaceFrame) {
        self.sub = SUB_FLIGHT;
        self.sub_tick = -1;
        self.fade = 1.0;
        self.white = 0.0;
        let v = (w.rng.rand() >> 16 & 3) as usize;
        self.flight = Flight {
            variant: v,
            count: 0,
            variants,
            planet: PLANET_START,
            shell: [0.0; 4],
            loaded: false,
            first_arrival,
            dest,
            turn: FLIGHT_SKY_TURN[v],
            rigs,
        };
        if first_arrival {
            self.flight.count = 2;
            self.flight.variant = 4;
        }
        w.svc.travel.trail = Trail::default();
        self.actors.clear();
        self.start_variant(w);
        out.audio.push(SpaceAudio::FlightBank);
        self.phase = Phase::Hold { n: w.ticks(12) as u32, k: 0, then: After::Flight };
    }

    fn start_variant(&mut self, w: &mut World) {
        let v = self.flight.variant.min(FLIGHT_VARIANTS - 1);
        self.lump = self.flight.variants.get(v).cloned().flatten().map(|scene| SpaceLump { index: v, scene });
        self.tick = 0;
        self.chunk = 0;
        self.chunk_tick = 0;
        self.actor_anim.clear();
        self.parse_chunk(w, 0);
    }

    /// Actor `k`'s class animation with its streamed slot (`FUN_00259288`: the class's sequence count) pointed at the
    /// chunk's sequence `seq`, re-pointed when the chunk changes; the class from the flight's skeletons, else the level's.
    fn actor_anim_for(&mut self, w: &World, k: usize, class: i32, seq: Option<&rc_formats::moby_anim::MobySequence>) -> Option<(Arc<rc_formats::moby_anim::MobyAnimClass>, u8)> {
        if self.actor_anim.len() <= k { self.actor_anim.resize(k + 1, None); }
        let fresh = !matches!(&self.actor_anim[k], Some((c, _, _, _)) if *c == self.chunk);
        if fresh {
            let base = match self.flight.rigs.get(&class) {
                Some(r) => r.anim.clone(),
                None => w.classes.anim(class as i16)?.clone(),
            };
            let mut anim = base;
            let slot = anim.sequences.len() as u8;
            anim.sequences.push(seq.cloned());
            self.actor_anim[k] = Some((self.chunk, Arc::new(anim), slot, None));
        }
        self.actor_anim[k].as_ref().map(|(_, a, s, _)| (a.clone(), *s))
    }

    /// [`Self::actor_anim_for`] with the slot holding the flight's pose of streamed frame `f` ([`flight_sequence`]).
    fn flight_anim_for(&mut self, w: &World, k: usize, class: i32, seq: &rc_formats::moby_anim::MobySequence, f: usize) -> Option<(Arc<rc_formats::moby_anim::MobyAnimClass>, u8)> {
        self.actor_anim_for(w, k, class, Some(seq))?;
        let (_, anim, slot, key) = self.actor_anim.get_mut(k)?.as_mut()?;
        if *key != Some(f) {
            let patched = flight_sequence(anim, seq, f);
            if let Some(q) = Arc::make_mut(anim).sequences.get_mut(*slot as usize) { *q = patched; }
            *key = Some(f);
        }
        Some((anim.clone(), *slot))
    }

    /// `MobyAttachToJoint(moby, list)` of actor `k` (class `class`, table moby `id`) in its current pose: the joint
    /// list's world matrix (row 3 = the point). None without the class's skeleton or the list.
    fn actor_joint(&mut self, w: &World, k: usize, class: i32, seq: Option<&rc_formats::moby_anim::MobySequence>, id: MobyId, list: usize) -> Option<[[f32; 4]; 4]> {
        use rc_formats::moby_anim::{attach_matrix, evaluate_chains_posed, AnimState};
        let chain = match self.flight.rigs.get(&class) {
            Some(r) => r.lists.get(list).cloned(),
            None => w.svc.joint_lists.get(&(class as i16)).and_then(|l| l.get(list)).cloned(),
        }
        .filter(|c| !c.is_empty())?;
        let (anim, slot) = self.actor_anim_for(w, k, class, seq)?;
        let m = w.m(id);
        let s = AnimState { seq_a: slot, frame_a: m.anim.frame_a, seq_b: slot, frame_b: m.anim.frame_b, t: m.anim.t, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: true };
        let p = evaluate_chains_posed(&anim, &s, None, &[&chain[..]], &[], &m.joint_mods)[0];
        let rows = [m.rows[0], m.rows[1], m.rows[2]].map(|r| r.map(f32::to_bits));
        Some(attach_matrix(&p, &rows, [m.position[0], m.position[1], m.position[2]], m.scale))
    }

    /// `SpaceLoadingLoop` 0x2a33b0 (module docs). Returns the flight's end through `out.event`, the scenery the draw
    /// `DrawWorldPaused` reads through `out.flight`.
    fn flight_tick(&mut self, w: &mut World, out: &mut SpaceFrame) {
        // FUN_002ab920: the frame's draw-callback lists start empty (no moby loop runs in the flight).
        w.svc.draw_callbacks.list1.clear();
        w.svc.draw_callbacks.list2.clear();
        self.fade -= 0.25;
        self.chunk_tick += 1;
        self.tick += 1;
        if self.fade < 0.0 { self.fade = 0.0; }
        // Tick 1 of every variant (not the first-arrival flights): the bank's sound v.
        if self.tick == 1 && !self.flight.first_arrival { out.audio.push(SpaceAudio::FlightSound(self.flight.variant as i32)); }
        let Some(mut lump) = self.lump.clone() else {
            out.event = Some(SpaceEvent::FlightDone);
            self.phase = Phase::Idle;
            return;
        };
        let mut end = lump.scene.end_tick();
        if self.tick < end {
            if 0x5f < self.chunk_tick { let c = self.chunk + 1; self.parse_chunk(w, c); }
        } else {
            for (id, _) in std::mem::take(&mut self.actors) {
                if let Some(id) = id { w.delete_moby(id); }
            }
            if !self.flight.first_arrival && !self.flight.loaded { self.flight.count = 0; }
            if 1 < self.flight.count {
                out.event = Some(SpaceEvent::FlightDone);
                self.phase = Phase::Idle;
                return;
            }
            if self.flight.count == 0 {
                let r = w.rng.rand() >> 16;
                self.flight.variant = (self.flight.variant + (r % 3) as usize + 1) & 3;
            } else {
                self.flight.variant = 4;
            }
            w.svc.travel.trail.head = 0;
            w.svc.travel.trail.count = 0;
            self.flight.planet = PLANET_START;
            self.flight.count += 1;
            self.start_variant(w);
            let Some(l) = self.lump.clone() else { return };
            lump = l;
            end = lump.scene.end_tick();
        }
        let Some(chunk) = lump.scene.chunks.get(self.chunk) else { return };
        let Some(rec) = chunk.camera.get(self.chunk_tick.clamp(0, i32::MAX) as usize).copied() else { return };
        let v = self.flight.variant.min(FLIGHT_VARIANTS - 1);
        let mut cam = crate::scene_player::scene_camera(&rec);
        if cam.tan_half_fov < FLIGHT_MIN_TAN[v] { cam.tan_half_fov = FLIGHT_MIN_TAN[v]; }
        self.last_camera = Some(cam);
        out.camera = Some(cam);
        let mut f = 1.0;
        if v == 4 {
            f = (end - self.tick) as f32 / end as f32;
            for (p, d) in self.flight.planet.iter_mut().zip(PLANET_STEP) { *p += f * d; }
        }
        let s = (self.tick - w.ticks(0x78)) as f32 * 20.0 * f;
        let dir = FLIGHT_SHELL[v];
        self.flight.shell = [dir[0] * s, dir[1] * s, dir[2] * s, dir[3] * s];
        self.flight.turn = FLIGHT_SKY_TURN[v];
        let fr = (self.chunk_tick >> 1) as usize;
        let mut poses = Vec::new();
        let ship = ship_index(w.svc.travel.ship);
        for (k, a) in chunk.actors.iter().enumerate() {
            let (id, class) = self.actors.get(k).copied().unwrap_or((None, a.class));
            let mut pose = None;
            for half in 0..2 {
                let t = (self.chunk_tick & 1) as f32 * 0.5 + half as f32 * 0.25;
                let p0 = a.positions.get(fr).copied().unwrap_or([0.0; 4]);
                let p1 = a.positions.get(fr + 1).copied().unwrap_or(p0);
                let pos = [p0[0] * (1.0 - t) + p1[0] * t, p0[1] * (1.0 - t) + p1[1] * t, p0[2] * (1.0 - t) + p1[2] * t];
                let mut hidden = false;
                let mut scale = None;
                if let Some(id) = id {
                    let m = w.mm(id);
                    (m.position[0], m.position[1], m.position[2]) = (pos[0], pos[1], pos[2]);
                    // +0x50 / +0x51 = 0: both frame buffers at the slot's one frame (`flight_sequence`).
                    m.anim.frame_a = 0;
                    m.anim.frame_b = 0;
                    m.anim.t = t;
                    m.b72 = 0xff;
                    m.b71 = 0xff;
                    m.draw_dist = 0x1ff;
                    // +0x52 / +0x53 = 2 for `update_moby_animation_state` and `MobyBuildMatrix` (the bounding sphere of
                    // the class's sequence 2), then the streamed slot.
                    (m.anim.seq_a, m.anim.seq_b) = (2, 2);
                    w.build_matrix(id);
                    if let Some((_, slot)) = self.flight_anim_for(w, k, class, &a.sequence, fr) {
                        let m = w.mm(id);
                        (m.anim.seq_a, m.anim.seq_b) = (slot, slot);
                    }
                    // MobyAttachToJoint(moby, 1 / 2): joint lists 1 and 2 (the engines) of this pose → a trail sample.
                    let j1 = self.actor_joint(w, k, class, Some(&a.sequence), id, 1);
                    let j2 = self.actor_joint(w, k, class, Some(&a.sequence), id, 2);
                    let (ta, tb) = match (j1, j2) {
                        (Some(a), Some(b)) => ([a[3][0], a[3][1], a[3][2]], [b[3][0], b[3][1], b[3][2]]),
                        // [L] No skeleton for the class: the ship's own trail points in the actor's frame.
                        _ => {
                            let m = w.m(id);
                            (to_world(&m.rows, m.position, super::TRAIL_A[ship]), to_world(&m.rows, m.position, super::TRAIL_B[ship]))
                        }
                    };
                    w.svc.travel.trail.push(ta, tb);
                    if v == 4 && self.flight.first_arrival {
                        w.mm(id).mode |= 1;
                        hidden = true;
                        w.svc.travel.trail.count = 0;
                    }
                    if v == 4 {
                        if end - 0x38 < self.tick {
                            let base = w.classes.info(class as i16).map_or(1.0, |i| i.scale);
                            let sc = base * (end - self.tick) as f32 * 0.017_857_144;
                            w.mm(id).scale = sc;
                            scale = Some(sc);
                            let a = ((end - self.tick) as u32) << 24;
                            for c in w.svc.travel.trail.colours.iter_mut() { c[0] = c[0] & 0xff_ffff | a; }
                        } else {
                            for c in w.svc.travel.trail.colours.iter_mut() { c[0] = c[0] & 0xff_ffff | 0x3800_0000; }
                        }
                    }
                }
                pose = Some(ActorPose { actor: k, class, chunk: self.chunk, frame_a: fr as u8, frame_b: (fr + 1) as u8, t, position: pos, yaw: id.map_or(0.0, |i| w.m(i).rotation[2]), moby: id, scale, hidden, glow: None, head: Some(fr as u8) });
            }
            if let Some(p) = pose { poses.push(p); }
        }
        // The draw (`DrawWorldPaused`): the trail and the glass of actor 0 on the flights that are not a first arrival.
        let first = self.actors.first().and_then(|a| a.0).map(|id| (id, self.actors[0].1));
        let shown = !self.flight.first_arrival;
        let glass = match first.filter(|_| shown) {
            Some((id, class)) => {
                let seq = chunk.actors.first().map(|a| &a.sequence);
                let o_class = w.m(id).o_class;
                self.actor_joint(w, 0, class, seq, id, 0).map(|m| (o_class, m))
            }
            None => None,
        };
        let dest = self.flight.dest;
        let planet = (v == 4).then(|| {
            let sc = usize::try_from(dest).ok().and_then(|d| PLANET_SCALE.get(d)).copied().unwrap_or(1.0);
            let o = self.flight.planet;
            PLANET_CORNERS.map(|c| [c[0] * sc + o[0], c[1] * sc + o[1], c[2] * sc + o[2]])
        });
        let caption_alpha = if v == 4 && 0x3c < self.tick { ((self.tick - 0x3c) * 2).min(0x80) as u32 } else { 0 };
        out.flight = Some(FlightDraw {
            variant: v,
            sky_turn: self.flight.turn,
            shell: self.flight.shell,
            sky_tan: cam.tan_half_fov.max(FLIGHT_SKY_TAN),
            planet,
            caption_alpha,
            dest,
            trail: shown,
            glass,
        });
        self.last_actors = poses.clone();
        out.actors = poses;
    }
}

/// `DrawSkyShells` 0x29f260 shell `k` (< the sky's shell count): its euler (0, y, `turn` + z) and the scale of its
/// rows (its translation is 0x160520 for all); None past the six the switch knows (scale 1, euler 0 there [L]: the
/// flight sky has six).
pub fn flight_shell(k: usize, turn: f32) -> Option<([f32; 3], f32)> {
    let (y, z, s) = *FLIGHT_SHELLS.get(k)?;
    Some(([0.0, y, add_rot(turn, z)], s))
}

/// The `DrawSkyShells` switch: per shell (y, z offset, row scale). Shell 0 falls through to shell 1's case.
pub const FLIGHT_SHELLS: [(f32, f32, f32); 6] = [(0.0, 0.0, 1.0), (0.0, 0.0, 1.0), (-0.075, -0.15, 1.25), (0.05, 0.125, 1.5), (0.1, -0.05, 1.75), (-0.15, 0.1, 2.0)];
/// 0x1be0b0[v] → 0x1604c4: the flight sky's z turn per variant.
pub const FLIGHT_SKY_TURN: [f32; 5] = [0.0, 0.0, 0.0, 0.0, 3.0];
/// `DrawWorldPaused`: the sky is drawn with tan(hfov/2) at least 0.63.
pub const FLIGHT_SKY_TAN: f32 = 0.63;
/// 0x1605a0 → 0x1605b0 at each variant's start: the planet picture's offset.
pub const PLANET_START: [f32; 4] = [-370.0, -280.0, 280.0, 0.0];
/// 0x1605c0: the planet picture's move per tick in variant 4 (× (end − tick)/end).
pub const PLANET_STEP: [f32; 4] = [0.3, 0.5, 0.0, 0.0];
/// 0x1bdfd0: the planet quad's corners (an upright square in the x/z plane).
pub const PLANET_CORNERS: [[f32; 3]; 4] = [[-80.0, 0.0, 80.0], [80.0, 0.0, 80.0], [-80.0, 0.0, -80.0], [80.0, 0.0, -80.0]];
/// 0x1be010[level]: the planet picture's scale (1.0 past level 18).
pub const PLANET_SCALE: [f32; 19] = [0.8, 1.0, 0.9, 1.4, 0.9, 1.1, 1.3, 0.8, 1.2, 1.5, 1.1, 0.7, 1.3, 1.2, 1.1, 1.0, 1.2, 1.1, 0.8];
/// The caption `fun_0022ea08`: `DrawTexturedQuad(0x20, H − 0x58, 0x100, 0x20, 0, 0, 0x100, 0x20)`.
pub const CAPTION_X: i32 = 0x20;
pub const CAPTION_DY: i32 = 0x58;
pub const CAPTION_W: i32 = 0x100;
pub const CAPTION_H: i32 = 0x20;
/// The card icon: FX 2 at (0x2c, H − 0x60, 0x40, 0x40) and FX 3 turning once per 0x37 vsyncs, centred at
/// (1216/16, H − 64), 272/16 wide (`fun_00200600`).
pub const CARD_ICON: [i32; 4] = [0x2c, 0x60, 0x40, 0x40];
pub const CARD_SPIN_PERIOD: i32 = 0x37;
pub const CARD_SPIN_SIZE: f32 = 272.0 / 16.0;
pub const CARD_SPIN_CENTRE: [i32; 2] = [1216 / 16, 64];

/// `MobyBuildMatrix` of a scene actor with its class animation `anim` (the streamed slot's sequence gives the
/// bounding sphere), then the moby-grid re-insert.
fn build_with(w: &mut World, id: MobyId, anim: Option<&rc_formats::moby_anim::MobyAnimClass>) {
    crate::moby_update::scheduler::rebuild_matrix(&mut w.table.mobys[id], anim);
    Arc::make_mut(&mut w.svc.grid).register(&mut w.table.mobys[id]);
}

/// The key of the Ratchet actor's head manipulator (record 0x17c7c0) in its joint-modifier list.
pub const ACTOR_HEAD_KEY: u32 = 0xffff_ffe0;
/// The key of the Clank actor's noggin manipulator (record 0x17c800).
pub const ACTOR_NOGGIN_KEY: u32 = 0xffff_ffe1;

/// `FUN_00228bc0` on a Clank actor (class 10; the same code runs for classes 419 and 1365 in mode-2 scenes): mode \|=
/// 0x10 (the glow drawn), the glow word +0x90 = s = `fast_sin(2π·(0x15f5cc % ticks(100))/ticks(100) − π)`, `(0x4c +
/// 24s) << 16 \| 0x80000000 \| (0xa2 + 52s) << 8 \| (0x44 + 24s)`; the cheat 0x15edb3 ("Clank has a large noggin"):
/// the manipulator 0x17c800 on joint list 2, scale 0x15ee18, attached once. The glow sprite at joint list 1 (`FUN_00229738
/// (0.047, 0.15, point − 0.007 z, 0x200000c0)`: the hero's glow list 0x141120 drawn by the hero's draw callback
/// 0x229440) is NOT ported (G-REN-005).
fn clank_actor(w: &mut World, id: MobyId) {
    let period = w.ticks(100).max(1);
    let f = (w.counter as i64).rem_euclid(period as i64) as f32 / period as f32;
    let s = ((f + f) * std::f32::consts::PI - std::f32::consts::PI).sin();
    let (a, b) = ((s * 24.0) as i32, (s * 52.0) as i32);
    let m = w.mm(id);
    m.mode |= 0x10;
    m.glow = ((a + 0x4c) as u32) << 16 | 0x8000_0000 | ((b + 0xa2) as u32) << 8 | (a + 0x44) as u32;
    if w.svc.cheats.on(crate::cheats::slot::CLANK) && !w.m(id).joint_mod_keys.contains(&ACTOR_NOGGIN_KEY) {
        match crate::moby_update::manip::target_joint(w, id, 2) {
            Some(joint) => {
                let sc = w.hero.bodies.scale18;
                let node = rc_formats::moby_anim::JointModifier { joint, mode: 0, weight: 0.0, quat: [0.0, 0.0, 0.0, 1.0], scale: [sc; 3], trans: [0.0; 3] };
                let m = w.mm(id);
                if m.joint_mod_keys.len() != m.joint_mods.len() { m.joint_mod_keys.resize(m.joint_mods.len(), u32::MAX); }
                m.joint_mods.insert(0, node);
                m.joint_mod_keys.insert(0, ACTOR_NOGGIN_KEY);
            }
            None => w.svc.unported("mode-6 Clank actor: joint list 2 not loaded"),
        }
    }
}

/// 0x1be060[v]: the flight shell's direction per variant.
pub const FLIGHT_SHELL: [[f32; 4]; 5] = [[0.6, 0.15, 0.0, 1.0], [1.0, 1.0, 0.0, 1.0], [0.9, -0.4, 0.0, 1.0], [-0.3, -0.9, 0.0, 1.0], [0.4, 0.4, 0.0, 1.0]];

/// The ship actor's glow, flames, white flash and exhaust in the take-off / landing scenes (module docs).
fn ship_fx(mode: &mut ShipMode, w: &mut World, id: MobyId, k: usize) {
    use crate::moby_update::classes::draw_callbacks::Callback;
    w.svc.draw_callbacks.register2(Callback::ShipGlass, id);
    // `MobyAttachToJoint(ship, 0)` on the pose the actor plays: its class with the streamed sequence (the class table's
    // has no such slot: the canopy glass sat at the ship's origin, inside the hull).
    let anim = mode.actor_anim.get(k).and_then(|a| a.as_ref()).map(|a| a.1.clone());
    let mat = w.joint_matrix_with(id, 0, anim.as_deref());
    w.svc.draw_callbacks.matrices.insert(id, mat);
    ship::register_shadow(w, id);
    let tick = mode.tick;
    let red_half = w.m(id).o_class == ship::CLASS_533;
    let pulse = |tick: i32| -> u32 {
        let f = (((tick & 0x3f) - 0x20) as f32 * 0.098_174_77).cos();
        let v = ((f * 80.0) as i32 + 0x78) as u32;
        if red_half { v >> 1 | v << 8 | v << 16 } else { v | v << 8 | v << 16 }
    };
    let ramp = |v: u32| -> u32 { if red_half { v << 8 | v << 16 } else { v | v << 8 | v << 16 } };
    let real = w.svc.travel.moby;
    if mode.sub == SUB_TAKEOFF {
        if tick < w.ticks(0x168) {
            w.mm(id).glow = pulse(tick);
        } else if tick < w.ticks(0x1f8) {
            let v = (((tick as f32) - 360.0) * 1.25) as i32;
            w.mm(id).glow = ramp(v.min(0xff) as u32);
            if w.ticks(0x1a4) < tick {
                if let Some(r) = real {
                    let m = w.mm(r);
                    m.cmd = (((tick as f32) - 420.0) * 1.2) as i32 as u8;
                    m.spawn_id = 0;
                    ship::register_flames(w.svc, r);
                }
            }
            if w.ticks(0x1d0) < tick {
                mode.white += 0.025;
                if 1.0 < mode.white { mode.white = 1.0; }
            }
        } else {
            mode.white -= 0.025;
            w.mm(id).glow = 0xa0_a0a0;
            if mode.white < 0.0 { mode.white = 0.0; }
        }
    } else if tick < w.ticks(0xf0) {
        if tick < w.ticks(200) {
            ship::exhaust(w, DT * -4.0, id);
            ship::exhaust(w, DT * -3.0, id);
        }
        w.mm(id).glow = pulse(tick);
        let m = w.mm(id);
        m.cmd = 0x32;
        m.spawn_id = 10;
        ship::register_flames(w.svc, id);
    } else if tick <= w.ticks(0x168) {
        if tick < w.ticks(300) {
            if let Some(r) = real {
                let m = w.mm(r);
                m.cmd = ((300.0 - tick as f32) * 1.5) as i32 as u8;
                m.spawn_id = 0;
                ship::register_flames(w.svc, r);
            }
        }
        let v = ((360.0 - tick as f32) * 1.25) as i32 as u32;
        w.mm(id).glow = ramp(v);
    } else {
        w.mm(id).glow = pulse(tick);
    }
}

/// `FUN_0024a3d8`: the level-start landing puts Ratchet 0.75 behind himself on the ground (when the ground there is
/// within 0.3 of his z) with the walk-to `0x249580(yaw, point, 1)`.
fn walk_back(w: &mut World) {
    use crate::moby_update::services::HeroCall;
    let h = w.hero;
    let pos = [h.pos[0].to_f32(), h.pos[1].to_f32(), h.pos[2].to_f32()];
    let fwd = [h.rows[0][0].to_f32(), h.rows[0][1].to_f32(), h.rows[0][2].to_f32()];
    let yaw = h.rot[2].to_f32();
    let mut p = [pos[0] - 0.75 * fwd[0], pos[1] - 0.75 * fwd[1], pos[2] - 0.75 * fwd[2] + 0.5];
    let v = [p[0], p[1], p[2], 0.0].map(crate::ps2v::Pf::f);
    let g = w.ground_height(crate::ps2v::Pf::ZERO, v, 0).to_f32();
    p[2] = g;
    if (pos[2] - g).abs() <= 0.3 {
        let f = w.hero_fields_mut();
        f.call(HeroCall::WalkTo { point: p, yaw, release: 1 });
        f.pose = Some(crate::moby_update::services::HeroPose { pos: p, yaw, target_yaw: yaw });
    }
}
