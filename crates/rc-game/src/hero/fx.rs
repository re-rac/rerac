//! **Hero polish — the hero's effects on the rest of the game**: the requests the hero code makes of systems it does
//! not own (the camera, the particle system, the sound layer's delayed voices), queued during the hero update in the
//! game's order and handed over by the tick (`crate::tick`) or the particle hook. One mechanism for every hero state:
//! a state asks through the helpers here and never talks to the camera or the particle pool itself.
//!
//! * **Camera shake** ([`HeroFx::shakes`], [`shake`]): the hero code's stores into the camera's shake records
//!   0x167260 / 0x167270 (`crate::follow_camera::Shake`), e.g. the Thruster stomp's landing (0.2 along up for 40
//!   ticks, level01 0x2390a0). The tick hands them to `Camera::request_shake` right after the hero update; the
//!   camera update of the same tick applies them, as in the game.
//! * **Particles** ([`HeroFx::parts`], [`spark`], [`dust`], [`bubbles`], [`sparkle_burst`]): the hero code calls the particle
//!   spawners inside its update, and the spawners' random draws land there. The port makes those draws at the
//!   game's point (so the one `rand` stream stays in step) and queues the spawn with them; the particle hook creates
//!   the queued records, in order, right before `UpdateParts` ([`create_particles`]). Nothing else creates a
//!   particle between the hero update and `UpdateParts` (the hand items' wall / ground sparks are not ported), so the
//!   pool slots and the update order are the game's. One difference: the game makes a spawner's own draws only when
//!   `CreatePart` found a free record; the port always makes them (the pool never fills in play: 2048 records).
//! * **Delayed voices** ([`HeroFx::voices`], `0x236810` / `0x236860`): the 8-entry queue at 0x141528 (`{active,
//!   sound, timer, flags}`); SetState queues the surfacing gasps (7 / 8) there, and right after the transitions
//!   (mode 0) `0x236860` counts every entry's timer down and plays `0x236738(sound, flags)` when it runs out
//!   ([`flush`]). The swim code's own voices (`SwimEvent::Sound` / `Voice`, which it records instead of playing)
//!   go through the same flush, at the same point.
//! * **Footsteps** (`HeroFootstepSound` 0x227e48 → `PlayFootstepSound` 0x2a1898, [`HeroSounds::footstep`]): the level def
//!   `tbl[level] + class·4 + foot·2 + variant + 2` (docs/plan/audio.md "Sound paths"), class = the footstep class of the
//!   ground under the feet 0x14063d (collision type bits 5–6, set by the ground probe), variant 1 with the Magneboots
//!   worn. Walking and running play them at fixed key times of sequences 3 / 4 ([`walk_footsteps`], `FUN_00227e90`,
//!   after the hand item's update); the landing of the fall (0x242930 case 6 / 0x2d, on the ground) plays both feet
//!   ([`HeroFx::footsteps`], queued in the transitions and played by [`flush`]). On Hoven the wrapper `0x228e58`
//!   also spawns a type-70 flat decal at the foot ([`footprint`]): the snow prints.

use super::swim::SwimEvent;
use super::{Hero, HeroSounds};
use crate::collision_query::MobyScene;
use crate::follow_camera::{ShakeAxis, ShakeRequest};
use crate::moby_runtime::Moby;
use crate::particles::{type02, type04, type12, type25, type34, type35, type45, type46, type47, type53, type70, Particles};
use crate::ps2v::Pf;
use crate::rng::Rng;
use rc_formats::collision::Collision;

/// A particle spawn the hero code made this tick, with its spawner's random draws already made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PartSpawn {
    /// `PartType25Spawn(pos, vel, variant)` 0x2825f8 (the grind / cable sparks; vel.w = its gravity); `size` =
    /// the spawner's `randf(5000, 30000)`.
    Spark { pos: [f32; 4], vel: [f32; 4], variant: bool, size: f32 },
    /// `PartType47Spawn(size, pos, vel)` 0x286cb0 (the sinking floor's sand); `alpha` = `randi(8)`, `rot` =
    /// `randi(0x100)`, the spawner's two draws.
    Dust { size: f32, pos: [f32; 4], vel: [f32; 4], alpha: u8, rot: u8 },
    /// `PartType34Spawn(size, level, pos, vel)` 0x2840e0 (bubbles), with the pop level resolved at the call and the
    /// spawner's six draws.
    Bubble { size: f32, level: f32, pos: [f32; 4], vel: [f32; 3], draws: type34::Draws },
    /// `PartType53Spawn(s12, s13, s14, pos, life, rgba, a3, t0, vel)` 0x287328 with `a3` 0 / 1 (no draw): the
    /// sparkle (the cable grab's burst `0x2a7e20`).
    Sparkle { s12: f32, s13: f32, s14: f32, pos: [f32; 4], life: i32, rgba: u32, b8: u8, t0: i8, vel: [f32; 3] },
    /// `PartType12Spawn(len, pos, vel, flags)` 0x280138: a Pyrocitor flame (or its glow puff), with its draws.
    Flame { len: f32, pos: [f32; 4], vel: [f32; 4], flags: u8, draws: crate::particles::type12::Draws },
    /// `PartType02Spawn` 0x27dc98: a trail blob (the Pyrocitor's embers); `rng` = the stream at its rotation draw
    /// (made by the caller at the game's point).
    Blob { spawn: crate::particles::type02::Spawn, rng: Rng },
    /// `PartType45Spawn(size, growth, pos, &0x13f640, −1)` 0x286780: a flat ring on the hero's water level (the
    /// splash's rings, the treading wake); `rng` = the stream at the spawner's first draw (its four draws were made
    /// at the call: [`reserve`]).
    Ring45 { size: f32, growth: f32, pos: [f32; 4], rng: Rng },
    /// `PartType46Spawn(size, spin, pos, vel, &0x13f640)` 0x286a68: a spreading ring riding the hero's water level
    /// (the wading / swimming bow rings); `rng` as for [`PartSpawn::Ring45`] (two draws).
    Ring46 { size: f32, spin: f32, pos: [f32; 4], vel: [f32; 4], rng: Rng },
    /// `PartType35Spawn(pos, vel, kind, life)` 0x2845a8: a water drop; `rng` as for [`PartSpawn::Ring45`] (two draws).
    Drop35 { pos: [f32; 4], vel: [f32; 4], kind: i32, life: i32, rng: Rng },
    /// `PartType27Spawn(size, pos, vel, rgba, life)` 0x282d80: a gun spark (the Blaster's muzzle); `rot` = the spawner's
    /// `rand()` (made at the call).
    Spark27 { size: f32, pos: [f32; 4], vel: [f32; 4], rgba: u32, life: i32, rot: u8 },
    /// `PartType26Spawn(size, moby, rgba, life, −1)` 0x282b00: a glow riding a moby the hero code created (the Blaster
    /// shot), at `at`; `rng` = the stream at the spawner's `rand()` (made at the call).
    Glow26 { size: f32, moby: usize, rgba: u32, life: i32, at: [f32; 3], rng: Rng },
    /// Spawner 0x28a3f0 of type 72 for moby `moby`'s pointer slot `slot` (the Blaster shot's trail; the record is
    /// linked in `Particles::links`), with the spawner's draws and the shot's size / colour writes after it.
    Trail72 { moby: usize, slot: u8, pos: [f32; 4], draws: crate::particles::type72::Draws, size: f32, rgba: u32 },
    /// `PartType23Spawn(jitter, lo, hi, size, pos, spin, vel, rgba)` 0x282060 then the caller's patch (the Blaster
    /// shot's impact smoke `0x2e2a18`: timer `life`, phase 2 fading from `alpha`, ALPHA 0x44 when `normal`); `rng` = the
    /// stream at the spawner's first draw (its five draws were made at the call).
    Puff23 { jitter: f32, lo: f32, hi: f32, size: f32, pos: [f32; 4], spin: i32, vel: [f32; 4], rgba: u32, life: i32, alpha: u8, normal: bool, rng: Rng },
    /// Spawner 0x286450 of type 44 (the Devastator's muzzle smoke); `rot` = its `randi(0xff)` (made at the call).
    Smoke44 { spawn: crate::particles::type44::Spawn, rot: u8 },
    /// `PartType21Spawn(size, pos, vel, c1, c2, life, split)` 0x281c10 (the Devastator's muzzle sparks); `rot` = its
    /// `rand()` (made at the call).
    Spark21 { size: f32, pos: [f32; 4], vel: [f32; 4], c1: u32, c2: u32, life: i32, split: i16, rot: u8 },
    /// `PartType28Spawn(spread, pos, rows)` 0x282ef0: a foot mote (`0x22c5c0`, `super::pose`); `rng` = the stream at
    /// the spawner's first draw (its 7 / 9 draws were made at the call: [`reserve`]).
    Mote28 { spread: f32, pos: [f32; 4], rows: Option<[[f32; 3]; 3]>, rng: Rng },
    /// `PartType78Spawn(s1, s2, pos, life, rgba, mode, spin, vel, target)` 0x28ad08: a Morph-o-Ray beam spark
    /// (`super::morph_ray`; modes 0 / 1: no draw); `target_word` = the target's record +0x10.
    Spark78 { s1: f32, s2: f32, pos: [f32; 4], life: i16, rgba: u32, mode: i32, spin: u8, vel: [f32; 3], target: Option<usize>, target_word: u32 },
    /// `PartType04Spawn` 0x27e538: a fire puff (the burns' fire, [`burn_fire`] / [`burn_bounce_fire`]); `rng` = the
    /// stream at its rotation draw (made at the call).
    Fire04 { spawn: type04::Spawn, rng: Rng },
    /// `PartType19Spawn(pos, vel)` 0x281748: a streak (the wrench's hit burst, [`wrench_burst`]); `rng` = the stream at
    /// its `randi(6)` (made at the call).
    Streak19 { pos: [f32; 4], vel: [f32; 4], rng: Rng },
    /// Level 12 `0x28d510` (the same code as level00 `0x274cf8`): a flat decal, the Hoven snow footprints
    /// ([`footprint`]); `frame` 1 is the left foot's texture, 0 the right.
    Footprint { angle: f32, pos: [f32; 4], frame: usize },
}

/// A moby the hero code created this tick (`CreateMoby` inside the hero update), with the creator's draws already made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MobySpawn {
    /// `FUN_002ff768(size, pos)`: the splash moby 775; `angle` = its `rand_angle` (the one draw).
    Splash { size: f32, pos: [f32; 4], angle: f32 },
    /// `FUN_002c9da0(side)`: a Thruster-Pack flame, class 0xa7 (`HeroItemsCreate`; no draws).
    ThrusterFlame { side: i32 },
    /// `HeroTickStateTimer`'s `CreateMoby(0x27a)`: the Hologuise disguise at the hero (`pos` = 0x13f3d0, `rot` =
    /// 0x13f3e0), then `SwitchCharacter(3, 0x53, moby)` (`super::hologuise`).
    Disguise { pos: [f32; 4], rot: [f32; 4] },
    /// The sinking death 0x7f's ice (`super::damage`'s module doc): created once, then faded in on the hero moby.
    Ice,
}

/// Ratchet's moby as the last write-back left it (`MobyBuildMatrix` of the previous tick): what `FUN_002645a8(Ratchet,
/// list, out)` reads besides his pose (the rows +0xc0, the position, the scale +0x2c and the joint-modifier list +0x64).
/// Captured at the start of every hero update ([`begin`]). Prints as `..` (a copy of the moby, not hero state).
#[derive(Clone, Default, PartialEq)]
pub struct JointFrame {
    pub rows: [[f32; 4]; 3],
    pub position: [f32; 4],
    pub scale: f32,
    pub mods: Vec<rc_formats::moby_anim::JointModifier>,
}

impl std::fmt::Debug for JointFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("..") }
}

/// The joint lists the hero's effects read (class header `joints`, the first byte list of each: root-to-joint
/// chains, by list index), as the engine loaded them: Ratchet's ([`Hero::set_joint_chains`]) and the back packs'
/// with their class scale ([`Hero::set_pack_joint_lists`]: the Hydro-Pack's jets). Prints as `..` (level data).
#[derive(Clone, Default, PartialEq)]
pub struct JointData {
    pub hero: std::sync::Arc<Vec<Vec<u8>>>,
    /// `(o_class, class scale +0x24, lists)` of the pack classes.
    pub packs: std::sync::Arc<Vec<PackLists>>,
    /// The other bodies' classes' joint lists (Clank 0x57, Giant Clank 0x1a3: `super::bodies::BodyJoints`): what
    /// `0x2645a8` reads while a body is the hero moby.
    pub bodies: std::sync::Arc<Vec<super::bodies::BodyJoints>>,
    /// The hand item classes' joint lists' modifier targets `(o_class, target per list)` (the second byte list's first
    /// entry, `rc_formats::moby_anim::list_target`; 0xff none): the joints the hand records and the Metal Detector's
    /// node act on (`super::gadgets::hand_modifiers`).
    pub items: std::sync::Arc<Vec<(i16, Vec<u8>)>>,
}

/// One back pack class's joint lists: `(o_class, class scale +0x24, first byte lists)`.
pub type PackLists = (i16, f32, Vec<Vec<u8>>);

impl std::fmt::Debug for JointData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("..") }
}

/// The back pack moby's placement as `HeroItemsAttach` 0x22fec0 left it at the end of the last hero update ([`end`]):
/// the attach matrix `W` of Ratchet's joint list 5 (the back items' attach word, item definitions 2..4 +4), position
/// `W.r3`, rows `W.r0..r2` with their columns normalised (`FUN_00271030`). Prints as `..`.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct BackFrame {
    pub rows: [[f32; 4]; 3],
    pub position: [f32; 4],
}

impl std::fmt::Debug for BackFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("..") }
}

/// The back items' attach list (`HERO_LISTS[5]`: the attach word 5 of item definitions 2, 3 and 4).
pub const BACK_LIST: usize = 5;

/// One entry of the delayed-voice queue 0x141528 (8 bytes: s16 active, sound, timer, flags).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueuedVoice {
    pub active: bool,
    pub sound: i16,
    pub timer: i16,
    pub flags: i16,
}

/// The hero's queued effects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeroFx {
    /// Camera shake requests of this tick, in order (the last one of an axis wins, as the stores do); drained by the
    /// tick.
    pub shakes: Vec<ShakeRequest>,
    /// Particle spawns of this tick, in the game's order (cleared at the start of the hero update; created by the
    /// particle hook: [`create_particles`]).
    pub parts: Vec<PartSpawn>,
    /// 0x141528: the delayed voices.
    pub voices: [QueuedVoice; 8],
    /// The hand item's class sounds of this tick (`PlayClassSound(index, 0, item)` in its update: the wrench's hit),
    /// played by the tick right after the item's update (`super::gadgets::flush_item_sounds`).
    pub item_sounds: Vec<i32>,
    /// Ratchet's own sounds the hand item's update makes (`0x236738` voices, the thrown wrench's whoosh loop in slot
    /// 0x14156c and its release), played with the item sounds (`super::gadgets::flush_item_sounds`).
    pub item_voices: Vec<super::packs::SoundCmd>,
    /// The hand item's looping class sounds' slots, by channel: [`LOOP_FLAME`] the item update's own loop (the
    /// Pyrocitor's flame +0x4a), [`LOOP_HUM`] the Tesla Claw's hum (+0x48), [`LOOP_CLICK`] the Blaster's empty click (its pvar +0x0c,
    /// kept while it plays), [`LOOP_SEQ`] the item's sequence loop sound (moby +0x7d: the Blaster's firing sequence 4).
    pub item_loops: [Option<i32>; 6],
    /// Whether each [`HeroFx::item_loops`] slot was still playing at the last flush (`SoundIsAlive` as the next
    /// update reads it: the Taunter's whistle).
    pub item_loop_alive: [bool; 6],
    /// The class sound [`LOOP_SEQ`]'s slot plays (the item's +0x7c when it was started).
    pub seq_loop: Option<i32>,
    /// Footsteps the transitions played this tick (`HeroFootstepSound(class, foot, 1)` of the landing), in order:
    /// `(class, foot)`, played by [`flush`] right after the transitions.
    pub footsteps: Vec<(u8, u8)>,
    /// The length of `swim.events` when this tick's hero update started (the swim events after it are this tick's;
    /// the engine drains the list after the tick).
    pub swim_mark: usize,
    /// Mobys the hero code created this tick, in order (cleared at the start of the hero update; created by the tick
    /// right after it, before anything else can take a moby slot: [`create_mobys`]).
    pub mobys: Vec<MobySpawn>,
    /// Ratchet's moby for the joint points ([`joint_point`]), captured by [`begin`].
    pub frame: JointFrame,
    /// Ratchet's and the packs' joint lists ([`joint_point`], [`pack_point`]).
    pub joints: JointData,
    /// The back pack's placement ([`end`], [`pack_point`]).
    pub back: BackFrame,
    /// The after-image records 0x1409c0 (Ratchet) and 0x140b00 (the thrown wrench): `crate::afterimage`.
    pub trails: crate::afterimage::Trails,
    /// `CameraResetBehindHero` asked for by a hero-side `HeroTeleport` ([`super::Hero::teleport`]); the tick applies it.
    pub camera_reset: bool,
    /// Ratchet's anim keys and key time as the hero update left them ([`end`]): what the moby loop of the next tick
    /// reads from his moby (+0x50..+0x55) and 0x13fdf8 (the Thruster flames' test, `crate::moby_update::classes::thruster_flame`).
    pub view: super::AnimView,
}

/// [`HeroFx::item_loops`] channels.
pub const LOOP_FLAME: usize = 0;
pub const LOOP_CLICK: usize = 1;
pub const LOOP_SEQ: usize = 2;
pub const LOOP_HUM: usize = 3;
/// The Suck Cannon's suction (class sound 2, its pvar +0x04) and the Taunter's whistle (its pvar +0x04): their own
/// channel (channel 0 is released every tick by the Pyrocitor's `item_gone` when another item is in the hand).
pub const LOOP_ITEM: usize = 4;
/// The Morph-o-Ray's beam hum (class sound 0 looping; the game keeps its slot in its own global 0x1617f8): its own
/// channel, so no other item's release can cut it.
pub const LOOP_MORPH: usize = 5;

/// A camera shake request (the writer's stores into 0x167260 for [`ShakeAxis::Up`], 0x167270 for
/// [`ShakeAxis::Forward`]): `amp` units for `ticks` ticks.
pub fn shake(h: &mut Hero, axis: ShakeAxis, amp: f32, ticks: i32) { h.fx.shakes.push(ShakeRequest { axis, amp, ticks }); }

/// `PartType25Spawn(pos, vel, variant)` 0x2825f8 from the hero code: the spawner's size draw `randf(5000, 30000)`
/// now, the record at the particle hook.
pub fn spark(h: &mut Hero, rng: &mut Rng, pos: [f32; 4], vel: [f32; 4], variant: bool) {
    let size = rng.randf(5000.0, 30000.0);
    h.fx.parts.push(PartSpawn::Spark { pos, vel, variant, size });
}

/// `PartType47Spawn(size, pos, vel)` 0x286cb0 from the hero code: the spawner's draws `randi(8)` (alpha) and
/// `randi(0x100)` (rotation) now, the record at the particle hook.
pub fn dust(h: &mut Hero, rng: &mut Rng, size: f32, pos: [f32; 4], vel: [f32; 4]) {
    let alpha = rng.randi(8) as u8;
    let rot = rng.randi(0x100) as u8;
    h.fx.parts.push(PartSpawn::Dust { size, pos, vel, alpha, rot });
}

/// `0x22b140(n, 0)`: `n` bubbles around the hero ([`bubbles_at`] mode 0).
pub fn bubbles(h: &mut Hero, rng: &mut Rng, n: i32) { bubbles_at(h, rng, None, n, 0) }

/// `0x22b140(n, mode)` (level01, read from the disassembly): `n` type-34 bubbles. Mode 0 around the hero's feet
/// (0x13f3d0 + (`randf(±0.1)`, `randf(±0.1)`, `randf(−0.5, 0.1)`): three draws), mode 1 at Ratchet's joint lists 0 / 0xe
/// alternately (the first bubble at 0), mode 2 at his lists 0x17 / 0x16 ([`joint_point`]; no draws). Each drifts with
/// 0.7 of the hero's displacement 0x13f450 (1.15 in 0x34) ± `randf(±0.4·dt)` sideways and `randf(−dt, 0)` down, size
/// `randf(4200, 7350)`, then `PartType34Spawn(size, −1, pos, vel)`: the −1 pops it at the water level 0x13f640 (the
/// hero's z + 0.4 in the sinking floor 0x31), and the spawner's six draws. `anim` = Ratchet's pose for modes 1 / 2
/// (None: the moby's origin).
pub fn bubbles_at(h: &mut Hero, rng: &mut Rng, anim: Option<&dyn super::AnimCtl>, n: i32, mode: i32) {
    let dt = 1.0 / 60.0;
    let p0 = crate::hero::physics::to_f32x3(h.pos);
    let d = crate::hero::physics::to_f32x3(h.disp);
    let k = if h.state == 0x34 { f32::from_bits(0x3f93_3333) } else { f32::from_bits(0x3f33_3333) };
    let level = if h.state == 0x31 { p0[2] + 0.4 } else { h.water_level.to_f32() };
    for i in 0..n.max(0) {
        let pos = match mode {
            0 => {
                let x = p0[0] + rng.randf(-0.1, 0.1);
                let y = p0[1] + rng.randf(-0.1, 0.1);
                let z = p0[2] + rng.randf(-0.5, 0.1);
                [x, y, z, 0.0]
            }
            _ => {
                let list = match (mode, i & 1) { (1, 0) => 0, (1, _) => 0xe, (_, 0) => 0x17, _ => 0x16 };
                match anim {
                    Some(a) => joint_point(h, a, list),
                    None => h.fx.frame.position,
                }
            }
        };
        let vx = d[0] * k + rng.randf(dt * -0.4, dt * 0.4);
        let vy = d[1] * k + rng.randf(dt * -0.4, dt * 0.4);
        let vz = d[2] * k + rng.randf(-dt, dt * 0.0);
        let size = rng.randf(4200.0, 7350.0);
        let draws = type34::Draws::draw(rng);
        h.fx.parts.push(PartSpawn::Bubble { size, level, pos, vel: [vx, vy, vz], draws });
    }
}

/// `0x2a7e20(point, n)` (L00; the cable grab): `n` pairs of type-53 sparkles flying off `point` (a random direction
/// 20°..88° up at 0.05..0.1 u/tick, spin ±1), a coloured and a white one per pair. Each pair first jitters `point`
/// itself by `randf_sym(0, 0.15)` per axis (the game writes the jitter into its argument, the hand point 0x13f930,
/// cumulatively). Draws per pair: `randf(0.1, 2)`, `randi(2)`, `rand_angle`, `randf(0.349, 1.536)`,
/// `randf(0.05, 0.1)`, three `randf_sym` (the spawns draw nothing).
pub fn sparkle_burst(h: &mut Hero, rng: &mut Rng, point: &mut [f32; 3], n: i32) {
    for _ in 0..n.max(0) {
        let s = rng.randf(0.1, 2.0);
        let i2 = rng.randi(2);
        let spin = if i2 != 0 { i2 } else { -1 };
        let a = rng.rand_angle();
        let b = rng.randf(f32::from_bits(0x3eb2_b8c2), f32::from_bits(0x3fc4_9809));
        let l = rng.randf(f32::from_bits(0x3d4c_cccd), 0.1);
        // FUN_00262c50(len, a, b): (cos a·l·cos b, sin a·l·cos b, sin b·l).
        let vel = [a.cos() * l * b.cos(), a.sin() * l * b.cos(), b.sin() * l];
        for p in point.iter_mut() { *p += rng.randf_sym(0.0, 0.15); }
        let pos = [point[0], point[1], point[2], 0.0];
        let life = crate::hero::physics::ticks(30);
        let s14 = f32::from_bits(0x3b44_9ba6);
        h.fx.parts.push(PartSpawn::Sparkle { s12: s * 0.1, s13: s, s14, pos, life, rgba: 0x7f20_7f7f, b8: 0, t0: spin as i8, vel });
        h.fx.parts.push(PartSpawn::Sparkle { s12: s * 0.07, s13: s * 0.7, s14, pos, life, rgba: 0x7f7f_7f7f, b8: 0x20, t0: -spin as i8, vel });
    }
}

/// One fire puff of the burns: `base` / `growth` = `trunc(randf(4200, 7350) / 1000)` / `trunc(randf(21000,
/// 31500.01) / 1000)` (drawn first), colour `c1` → 0x70f0 over `ticks(40)`, additive.
fn fire_sizes(rng: &mut Rng) -> (i16, i16) {
    let base = (rng.randf(4200.0, 7350.0) / 1000.0) as i16;
    let growth = (rng.randf(21000.0, f32::from_bits(0x46f6_1801)) / 1000.0) as i16;
    (base, growth)
}

fn fire_puff(h: &mut Hero, rng: &mut Rng, pos: [f32; 4], vel: [f32; 4], c1: u32, base: i16, growth: i16) {
    let spawn = type04::Spawn { pos, vel, c1, c2: 0x70f0, life: crate::hero::physics::ticks(0x28), base, growth, additive: true };
    let at = reserve(rng, 1);
    h.fx.parts.push(PartSpawn::Fire04 { spawn, rng: at });
}

/// `FUN_00262c50(len, a, b)`: (cos a·l·cos b, sin a·l·cos b, sin b·l).
fn polar(l: f32, a: f32, b: f32) -> [f32; 4] { [a.cos() * l * b.cos(), a.sin() * l * b.cos(), b.sin() * l, 0.0] }

/// L00 `0x217450`: the burn death's fire (every tick of 0x7c; once at the lava sinking death 0x7b's entry on level 15):
/// ten puffs (colour 0x7000c0f0) around Ratchet within `randf(0.05, radius + 0.2)`, at the height `floor` (Ratchet's
/// own off level 6), rising at `randf(0.15, 0.7)·dt` 20°..80° up. Draws per puff: the sizes, the speed, the angle, the
/// elevation, the distance, the spawner's rotation.
pub fn burn_fire(h: &mut Hero, rng: &mut Rng, floor: f32) {
    let dt = crate::hero::physics::DT.to_f32();
    for _ in 0..10 {
        let (base, growth) = fire_sizes(rng);
        let speed = rng.randf(dt * 0.15, dt * 0.7);
        let p = super::physics::to_f32x3(h.pos);
        let z = if h.idle.level != 6 { floor } else { p[2] };
        let a = rng.rand_angle();
        let b = rng.randf(f32::from_bits(0x3eb2_b8c2), f32::from_bits(0x3fb2_b8c2));
        let r = rng.randf(f32::from_bits(0x3d4c_cccd), h.cap_radius.to_f32() + 0.2);
        let vel = polar(speed, a, b);
        let pos = [p[0] + a.cos() * r, p[1] + a.sin() * r, z, h.pos[3].to_f32()];
        fire_puff(h, rng, pos, vel, 0x7000_c0f0, base, growth);
    }
}

/// L00 `0x209ec8`: the burn bounce's effects while 0x3c rises (the physics after the jump's): every 2..3 ticks
/// (0x13f7ee) a spark (type 25) off his side (+90°) at `randf(0.2, 0.7)·dt`, falling at 8·dt²; every 3..7 ticks
/// (0x13f7ec) a ring of 57 fire puffs in every direction at `randf(0.15, 0.57)·dt`, their alpha 240 → 32 over his first
/// 40 ticks in the state (0x13f4e8), at most 160.
pub fn burn_bounce_fire(h: &mut Hero, rng: &mut Rng) {
    use crate::hero::physics::{ticks, DT, DT2};
    let (dt, dt2) = (DT.to_f32(), DT2.to_f32());
    let fire = |t: &mut i16| { if *t == 0 { return true; } *t -= 1; *t == 0 };
    if fire(&mut h.damage.burn_spark) {
        let y = crate::moby_update::creature::add_rot(h.rot[2].to_f32(), std::f32::consts::FRAC_PI_2);
        let s = rng.randf(dt * 0.2, dt * 0.7);
        let pos = h.pos.map(|x| x.to_f32());
        spark(h, rng, pos, [y.cos() * s, y.sin() * s, 0.0, dt2 * -8.0], false);
        h.damage.burn_spark = rng.rand_range(2, 3) as i16;
    }
    if fire(&mut h.damage.burn_ring) {
        let f = ((ticks(0x28) - h.timer) as f32 / ticks(0x28) as f32).clamp(0.0, 1.0);
        let alpha = ((f * 240.0) as i32).clamp(0x20, 0xa0) as u32;
        for _ in 0..57 {
            let (base, growth) = fire_sizes(rng);
            let speed = rng.randf(dt * 0.15, dt * 0.57);
            let a = rng.rand_angle();
            let b = rng.rand_angle();
            let pos = h.pos.map(|x| x.to_f32());
            fire_puff(h, rng, pos, polar(speed, a, b), alpha << 24 | 0xc0f0, base, growth);
        }
        h.damage.burn_ring = rng.rand_range(ticks(3), ticks(7)) as i16;
    }
}

/// `0x2bdd20(item)`: the wrench's hit burst at `point` (0x1742e0, the last line test's): 20 streaks (type 19) and 10
/// sparks (type 21: size 20000, colours 0x4f007fff → 0x1fffffff, `rand_range(ticks(10), ticks(20))` ticks), their
/// directions `randf_sym(0, π)` off the elevation from the point to the camera `cam` (0x167240), negated, for both
/// angles; the streaks 0.2 a tick from `randf(1, 3)` of that out, the sparks 0.05 a tick from the point jittered by
/// `randf_sym(0, 0.1)` per axis. Its tail's sparkles (`0x2bdb18(point, 10)` when the wrench's s16 pvar +0x78 is 0x2b) never
/// run: nothing in the level code writes that word, and a created moby's pvars start at 0.
pub fn wrench_burst(h: &mut Hero, rng: &mut Rng, point: [f32; 3], cam: [f32; 3]) {
    use crate::hero::physics::ticks;
    use crate::moby_update::creature::add_rot;
    use std::f32::consts::PI;
    let d = [cam[0] - point[0], cam[1] - point[1], cam[2] - point[2]];
    let xy = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let up = -crate::moby_update::creature::atan(xy, d[2]);
    let w = h.pos[3].to_f32();
    for _ in 0..20 {
        let a = add_rot(rng.randf_sym(0.0, PI), up);
        let b = add_rot(rng.randf_sym(0.0, PI), up);
        let v = polar(0.2, b, a);
        let s = rng.randf(1.0, 3.0);
        let pos = [v[0] * s + point[0], v[1] * s + point[1], v[2] * s + point[2], w];
        let at = reserve(rng, 1);
        h.fx.parts.push(PartSpawn::Streak19 { pos, vel: v, rng: at });
    }
    for _ in 0..10 {
        let pos = [point[0] + rng.randf_sym(0.0, 0.1), point[1] + rng.randf_sym(0.0, 0.1), point[2] + rng.randf_sym(0.0, 0.1), w];
        let a = add_rot(rng.randf_sym(0.0, PI), up);
        let b = add_rot(rng.randf_sym(0.0, PI), up);
        let v = polar(f32::from_bits(0x3d4c_cccd), b, a);
        let life = rng.rand_range(ticks(10), ticks(0x14));
        let rot = rng.rand() as u8;
        h.fx.parts.push(PartSpawn::Spark21 { size: 20000.0, pos, vel: v, c1: 0x4f00_7fff, c2: 0x1fff_ffff, life, split: 1, rot });
    }
}

/// `0x2bdb18(point, n)`: the wrench's hit sparkles, `n` pairs of type-53 sparkles (`PartType53Spawn` 0x287328, modes 0 / 1:
/// no draw in the spawner). Each pair: size `s = randf(0.1, 2)`, spin side `randi(2)` (0 → −1), velocity
/// `polar(randf(0.05, 0.1), rand_angle, randf(20°, 88°))`, then the point (the caller's, moved in place: the jitter adds
/// up) `+= randf_sym(0, 0.15)` per axis; the first (0.1·s, s, z deceleration 0.003, `ticks(30)`, 0x7f207f7f, mode 0, the
/// side), the second (0.07·s, 0.7·s, …, 0x7f7f7f7f, mode 1, the other side).
pub fn wrench_sparkles(h: &mut Hero, rng: &mut Rng, mut point: [f32; 3], n: i32) {
    let w = h.pos[3].to_f32();
    for _ in 0..n.max(0) {
        let s = rng.randf(0.1, 2.0);
        let r = rng.randi(2);
        let mut side = if r != 0 { r } else { -1 } as i8;
        let a = rng.rand_angle();
        let p = rng.randf(f32::from_bits(0x3eb2_b8c2), f32::from_bits(0x3fc4_9809));
        let v = polar(rng.randf(0.05, 0.1), a, p);
        for c in &mut point { *c += rng.randf_sym(0.0, 0.15); }
        let pos = [point[0], point[1], point[2], w];
        let life = crate::hero::physics::ticks(30);
        let vel = [v[0], v[1], v[2]];
        let s14 = f32::from_bits(0x3b44_9ba6);
        h.fx.parts.push(PartSpawn::Sparkle { s12: s * 0.1, s13: s, s14, pos, life, rgba: 0x7f20_7f7f, b8: 0, t0: side, vel });
        side = -side;
        h.fx.parts.push(PartSpawn::Sparkle { s12: s * f32::from_bits(0x3d8f_5c29), s13: s * f32::from_bits(0x3f33_3333), s14, pos, life, rgba: 0x7f7f_7f7f, b8: 1, t0: side, vel });
    }
}

/// The particle hook's part: create the hero's queued spawns of this tick, in order (before `UpdateParts`).
pub fn create_particles(h: &Hero, sys: &mut Particles) {
    sys.hero = crate::hero::physics::to_f32x3(h.pos);
    sys.hero_body = crate::hero::physics::to_f32x3(h.body_point);
    let gold = h.weapons.gold[super::pyrocitor::PYROCITOR as usize];
    sys.gold = gold;
    sys.hero_plat = crate::hero::physics::to_f32x3(h.plat_applied);
    // 0x13f5e0, the gravity direction type 78 homes around.
    sys.gravity = crate::hero::physics::to_f32x3(h.gravity_dir);
    // The object in the glove follows the hand point in the hero update, after the moby loop wrote its anchor: the
    // Doom canister's type-32 glows (`0x283ea8`) read its position +0x10 in `UpdateParts`, after the hero.
    if let Some((b, p)) = h.weapons.glove.placed {
        if let Some(a) = sys.anchors.get_mut(&b) { *a = p; }
    }
    for s in &h.fx.parts { create_one(sys, s, gold); }
}

/// One queued spawn's record(s) (the hero's hook, and the moby updates that describe their spawns the same way: the
/// Blaster shot's impact).
pub fn create_one(sys: &mut Particles, s: &PartSpawn, gold: u8) {
    match *s {
        PartSpawn::Spark { pos, vel, variant, size } => { type25::spawn(sys, pos, vel, variant, size); }
        PartSpawn::Dust { size, pos, vel, alpha, rot } => { type47::spawn(sys, size, pos, vel, alpha, rot); }
        PartSpawn::Bubble { size, level, pos, vel, draws } => { type34::spawn(sys, size, level, pos, vel, &draws); }
        PartSpawn::Sparkle { s12, s13, s14, pos, life, rgba, b8, t0, vel } => {
            type53::spawn(sys, s12.to_bits(), s13.to_bits(), s14.to_bits(), pos.map(f32::to_bits), life, rgba, b8, t0, vel.map(f32::to_bits));
        }
        PartSpawn::Flame { len, pos, vel, flags, draws } => { type12::spawn(sys, len, pos, vel, flags, gold, &draws); }
        PartSpawn::Blob { spawn, mut rng } => { type02::spawn(sys, &mut rng, &spawn); }
        PartSpawn::Fire04 { spawn, mut rng } => { type04::spawn(sys, &mut rng, &spawn); }
        PartSpawn::Streak19 { pos, vel, mut rng } => { crate::particles::type19::spawn19(sys, &mut rng, pos, vel); }
        PartSpawn::Ring45 { size, growth, pos, mut rng } => { type45::spawn45_on(sys, &mut rng, size, growth, pos, type45::HERO_WATER_LEVEL, u32::MAX); }
        PartSpawn::Ring46 { size, spin, pos, vel, mut rng } => { type46::spawn_on(sys, &mut rng, size, spin, pos, vel, type45::HERO_WATER_LEVEL); }
        PartSpawn::Drop35 { pos, vel, kind, life, mut rng } => { type35::spawn(sys, &mut rng, pos, vel, kind, life); }
        PartSpawn::Spark27 { size, pos, vel, rgba, life, rot } => { crate::particles::type27::spawn(sys, size, pos, vel, rgba, life, rot); }
        PartSpawn::Glow26 { size, moby, rgba, life, at, mut rng } => { crate::particles::type26::spawn(sys, &mut rng, size, moby, rgba, life, -1, at); }
        PartSpawn::Trail72 { moby, slot, pos, draws, size, rgba } => {
            if let Some(i) = crate::particles::type72::spawn(sys, pos, &draws) {
                let r = &mut sys.pool.recs[i];
                crate::particles::rec::set_ff(r, 0xc, size);
                crate::particles::rec::set_u32(r, 4, rgba);
                sys.links.insert((moby, slot), i);
            }
        }
        PartSpawn::Smoke44 { spawn, rot } => { crate::particles::type44::spawn(sys, &spawn, rot); }
        PartSpawn::Spark21 { size, pos, vel, c1, c2, life, split, rot } => { crate::particles::type21::spawn(sys, size, pos, vel, c1, c2, life, split, rot); }
        PartSpawn::Mote28 { spread, pos, rows, mut rng } => { crate::particles::type28::spawn(sys, &mut rng, spread, pos, rows); }
        PartSpawn::Spark78 { s1, s2, pos, life, rgba, mode, spin, vel, target, target_word } => {
            // The beam's modes 0 / 1 draw nothing (the spawner's `randi(255)` is for the other modes).
            let mut rng = Rng::new();
            crate::particles::type78::spawn(sys, &mut rng, crate::particles::type78::Spawn { s1, s2, pos, life, rgba, mode, spin, vel, target, target_word });
        }
        PartSpawn::Puff23 { jitter, lo, hi, size, pos, spin, vel, rgba, life, alpha, normal, mut rng } => {
            if let Some(i) = crate::particles::type23::spawn(sys, &mut rng, jitter, lo, hi, size, pos, spin, vel, rgba) {
                let r = &mut sys.pool.recs[i];
                crate::particles::rec::set_i16(r, 0xa, life as i16);
                crate::particles::rec::set_u32(r, 0x24, 2);
                r[0x2a] = alpha;
                r[0x2b] = life as u8;
                if normal { r[3] = 0x44; }
            }
        }
        PartSpawn::Footprint { angle, pos, frame } => { type70::spawn(sys, angle, pos, frame); }
    }
}

/// `0x236810(sound, delay, flags)`: the first free entry of the queue (none free: dropped).
pub fn queue_voice(h: &mut Hero, sound: i16, delay: i16, flags: i16) {
    if let Some(e) = h.fx.voices.iter_mut().find(|e| !e.active) {
        *e = QueuedVoice { active: true, sound, timer: delay, flags };
    }
}

/// Start of the hero update: this tick's particle and moby spawns begin empty; the swim events so far are old ones;
/// Ratchet's moby as the last write-back left it is kept for the joint points.
pub(super) fn begin(h: &mut Hero, moby: &Moby) {
    h.fx.parts.clear();
    h.fx.mobys.clear();
    h.fx.swim_mark = h.swim.events.len();
    let f = &mut h.fx.frame;
    f.rows = [moby.rows[0], moby.rows[1], moby.rows[2]];
    f.position = moby.position;
    f.scale = moby.scale;
    f.mods.clone_from(&moby.joint_mods);
}

/// `FUN_002645a8(Ratchet, list, out)` 0x2645a8 from the hero code: the world point of the last joint of Ratchet's joint
/// list `list` in his current pose (this tick's advance, the weapon-arm layers and the joint modifiers applied:
/// `AnimCtl::eval_chains_with`), placed by his moby as the last write-back left it ([`JointFrame`]). The same
/// formula as [`crate::moby_update::services::World::joint_point`] for the table's mobys ([`joint_world_point`]).
/// Without the list or animation data the point is the moby's origin (as there). Native `f32`.
pub fn joint_point(h: &Hero, anim: &dyn super::AnimCtl, list: usize) -> [f32; 4] {
    joint_point_in(h, anim, &h.fx.frame, list)
}

/// [`joint_point`] at an explicit moby frame. The calls before the write-back (the landing's footprints, the foot
/// motes) pass [`HeroFx::frame`]; the walk's footprints run after it and pass the moby's current rows and position
/// (`MobyJointPoint` 0x268508 reads the moby).
fn joint_point_in(h: &Hero, anim: &dyn super::AnimCtl, f: &JointFrame, list: usize) -> [f32; 4] {
    // While a body is the hero moby, its class's lists (super::bodies).
    let chains = h.body_joints().map_or(&*h.fx.joints.hero, |b| &b.chains);
    let chain = chains.get(list).filter(|c| !c.is_empty());
    let t = chain
        .and_then(|c| anim.eval_chains_with(&[c.as_slice()], &h.weapons.layers, &f.mods).into_iter().next())
        .map_or([0.0, 0.0, 0.0, 1.0], |p| p[3]);
    joint_world_point(&f.rows, f.position, f.scale, t)
}

/// The second half of `FUN_002645a8`, for any moby: a joint's pose translation `t` (`P.r3`, class units) to the world:
/// `q = t.xyz · scale/1024` (w kept), `r = rows · q` (row 3 = (0, 0, 0, 1)), `out.xyz = r.xyz + position`, `out.w = r.w`.
pub fn joint_world_point(rows: &[[f32; 4]; 3], position: [f32; 4], scale: f32, t: [f32; 4]) -> [f32; 4] {
    let k = scale * (1.0 / 1024.0);
    let q = [t[0] * k, t[1] * k, t[2] * k, t[3]];
    let v: [f32; 4] = std::array::from_fn(|l| rows[0][l] * q[0] + rows[1][l] * q[1] + rows[2][l] * q[2] + if l == 3 { q[3] } else { 0.0 });
    [v[0] + position[0], v[1] + position[1], v[2] + position[2], v[3]]
}

/// End of the hero update, after the back items' update: `HeroItemsAttach`'s placement of the back pack from Ratchet's
/// joint list [`BACK_LIST`] in his pose now and his moby as the write-back left it ([`BackFrame`]).
pub(super) fn end(h: &mut Hero, moby: &Moby, anim: &dyn super::AnimCtl) {
    h.fx.view = anim.view();
    let Some(chain) = h.fx.joints.hero.get(BACK_LIST).filter(|c| !c.is_empty()) else { return };
    let Some(p) = anim.eval_chains_with(&[chain.as_slice()], &h.weapons.layers, &moby.joint_mods).into_iter().next() else { return };
    let r = &moby.rows;
    let mut rows: [[f32; 4]; 3] = std::array::from_fn(|i| std::array::from_fn(|l| r[0][l] * p[i][0] + r[1][l] * p[i][1] + r[2][l] * p[i][2]));
    for c in 0..3 {
        let n = (rows[0][c] * rows[0][c] + rows[1][c] * rows[1][c] + rows[2][c] * rows[2][c]).sqrt();
        let q = if n == 0.0 { 0.0 } else { 1.0 / n };
        for row in rows.iter_mut() { row[c] *= q; }
    }
    let rr = [moby.rows[0], moby.rows[1], moby.rows[2]];
    h.fx.back = BackFrame { rows, position: joint_world_point(&rr, moby.position, moby.scale, p[3]) };
}

/// `FUN_002645a8(pack, list, out)` on the back pack moby (0x1404d0): its joint list `list` in its current pose, placed
/// by [`BackFrame`] at its class scale. None without a pack, its class data or the list.
pub fn pack_point(h: &Hero, list: usize) -> Option<[f32; 4]> {
    let b = h.back.as_ref()?;
    let class = b.classes.pack(b.pack_item).filter(|c| c.0 == b.pack_o_class)?.1;
    let (_, scale, lists) = h.fx.joints.packs.iter().find(|p| p.0 == b.pack_o_class)?;
    let chain = lists.get(list).filter(|c| !c.is_empty())?;
    let t = rc_formats::moby_anim::evaluate_chains(class, &b.pack.anim, b.pack.snapshot.as_ref(), &[chain.as_slice()]).into_iter().next()?[3];
    Some(joint_world_point(&h.fx.back.rows, h.fx.back.position, *scale, t))
}

/// `MatrixMulVec3(v, v, Ratchet+0xc0)`: `v` in Ratchet's moby frame ([`JointFrame::rows`]) to the world (no position).
pub fn moby_dir(h: &Hero, v: [f32; 3]) -> [f32; 3] {
    let r = &h.fx.frame.rows;
    std::array::from_fn(|l| r[0][l] * v[0] + r[1][l] * v[1] + r[2][l] * v[2])
}

/// The stream as a queued spawner will draw from it: the state now, and `n` draws skipped (each of the spawner's
/// helpers — `randi`, `randf`, `rand_angle`, `rand` — takes exactly one `rand()`), so the draws after the call see
/// the stream the game's do.
pub fn reserve(rng: &mut Rng, n: usize) -> Rng {
    let at = *rng;
    for _ in 0..n { rng.rand(); }
    at
}

/// The tick's part, right after the hero update: create the mobys the hero code asked for this tick, in order
/// (`CreateMoby` through the hit sink: `super::items::HitSink::create_moby`), and fill them as their creators do.
/// Returns the new mobys (the tick builds their matrices). `light` = Ratchet's light word.
#[allow(clippy::too_many_arguments)]
pub fn create_mobys(
    h: &mut Hero,
    table: &mut crate::moby_runtime::MobyTable,
    hero_moby: usize,
    hits: &mut dyn super::items::HitSink,
    sounds: &mut dyn HeroSounds,
    rng: &mut Rng,
    counter: u64,
) -> Vec<usize> {
    let mut out = Vec::new();
    let light = table.mobys.get(hero_moby).map(|m| m.light);
    for s in std::mem::take(&mut h.fx.mobys) {
        match s {
            MobySpawn::Splash { size, pos, angle } => {
                let Some(id) = hits.create_moby(table, crate::moby_update::classes::splash::CLASS, counter) else { continue };
                crate::moby_update::classes::splash::fill(&mut table.mobys[id], light, size, pos, angle);
                out.push(id);
            }
            MobySpawn::ThrusterFlame { side } => {
                use crate::moby_update::classes::thruster_flame as tf;
                let Some(id) = hits.create_moby(table, tf::CLASS, counter) else { continue };
                tf::fill(&mut table.mobys[id], side);
                out.push(id);
            }
            MobySpawn::Disguise { pos, rot } => {
                let class = super::bodies::DISGUISE_CLASS;
                let Some(id) = hits.create_moby(table, class, counter) else { continue };
                let hero_light = table.mobys.get(h.hero_moby(hero_moby)).map(|m| (m.light, m.ambient));
                let m = &mut table.mobys[id];
                m.position = pos;
                m.rotation = rot;
                m.draw_dist = 0x40;
                m.visible = 1;
                m.glow = 0;
                if let Some((l, a)) = hero_light { (m.light, m.ambient) = (l, a); }
                h.bodies.disguise = Some(id);
                // `SwitchCharacter(3, 0x53, moby)`: made by the next tick before the classes' calls (`Bodies::restore`).
                h.bodies.restore = Some((super::bodies::body::DISGUISE, super::hologuise::IDLE, super::bodies::BodyMoby { id, o_class: class, anim: m.anim }));
                out.push(id);
            }
            MobySpawn::Ice => {
                let class = super::damage::ICE_CLASS;
                let id = match h.damage.ice {
                    Some(id) => id,
                    None => {
                        let Some(id) = hits.create_moby(table, class, counter) else { continue };
                        let m = &mut table.mobys[id];
                        m.position = h.pos.map(|v| v.to_f32());
                        m.rotation = h.rot.map(|v| v.to_f32());
                        m.update_dist = 0;
                        m.visible = 1;
                        m.draw_dist = 0x40;
                        m.alpha = 0;
                        m.mode |= 0x800;
                        let p = m.position;
                        h.damage.ice = Some(id);
                        sounds.item_sound(class, [p[0], p[1], p[2]], 0, 0, rng);
                        id
                    }
                };
                let Some((pos, rot)) = table.mobys.get(h.hero_moby(hero_moby)).map(|m| (m.position, m.rotation)) else { continue };
                let Some(m) = table.mobys.get_mut(id) else { continue };
                m.alpha = m.alpha.saturating_add(5).min(0x80);
                (m.position, m.rotation) = (pos, rot);
                out.push(id);
            }
        }
    }
    out
}

/// After the transitions (mode 0): this tick's swim voices (`0x236738(id, 0)` played, `0x236810` queued), then
/// `0x236860`: every queue entry's timer counts down (`FastDecTimer__FRs`), and an entry whose timer runs out this
/// tick is freed and its voice played.
pub(super) fn flush(h: &mut Hero, moby: &Moby, sounds: &mut dyn HeroSounds, rng: &mut Rng) {
    for (class, foot) in std::mem::take(&mut h.fx.footsteps) { footstep(h, moby, class, foot, sounds, rng); }
    let new: Vec<SwimEvent> = h.swim.events.get(h.fx.swim_mark..).map(|e| e.to_vec()).unwrap_or_default();
    h.fx.swim_mark = h.swim.events.len();
    for e in new {
        match e {
            SwimEvent::Sound(id) => { sounds.voice(moby, id, 0, rng); }
            SwimEvent::Voice(id, delay) => queue_voice(h, id as i16, delay as i16, 0),
            _ => {}
        }
    }
    if h.mode != 0 { return; }
    voice_queue(h, moby, sounds, rng);
}

/// `0x236860`: the delayed-voice queue's countdown (mode 0 after the transitions; `HeroUpdateAlt` calls it in the bodies:
/// super::bodies).
pub(super) fn voice_queue(h: &mut Hero, moby: &Moby, sounds: &mut dyn HeroSounds, rng: &mut Rng) {
    for k in 0..h.fx.voices.len() {
        let e = &mut h.fx.voices[k];
        // FastDecTimer: 1 at 0 (no change), else t = max(t, 1) − 1, 2 when it reaches 0.
        if e.timer == 0 { continue; }
        e.timer = e.timer.max(1) - 1;
        if e.timer > 0 { continue; }
        e.active = false;
        let (sound, flags) = (e.sound, e.flags);
        sounds.voice(moby, sound as i32, flags as u16 as u32, rng);
    }
}

/// `HeroFootstepSound(class, foot, 1)` (0x227e48): the variant is 1 when the feet item 0x140430 is the Magneboots
/// model (class 0xad: [`Hero::magneboots_on`], the feet slot), then `PlayFootstepSound(class, foot, variant, 0,
/// Ratchet)` 0x2a1898 on the level `0x15ed84`.
pub fn footstep(h: &Hero, moby: &Moby, class: u8, foot: u8, sounds: &mut dyn HeroSounds, rng: &mut Rng) -> i32 {
    let variant = h.magneboots_on() as u8;
    sounds.footstep(moby, h.idle.level, class, foot, variant, rng)
}

/// `FUN_00227e90` (in `0x228870` after `HeroItemsUpdate`, mode 0): the walk / run footsteps. Unless Ratchet is in state
/// 2 for fewer than 15 ticks: in state 2 at exactly tick 22 the left foot; then on sequence 3 (walk) the key times 49.5
/// (left) and 17.0 (right), on sequence 4 (run) 12.5 (left) and 1.0 (right), when passed this tick (`0x231f18`) and no
/// blend runs (0x13fdec). `anim` = Ratchet's animation after this tick's advance; `moby` = his moby after the
/// write-back (the level-12 footprints' joint frame).
pub fn walk_footsteps(h: &mut Hero, moby: &Moby, anim: &dyn super::AnimCtl, coll: &Collision, mobys: Option<&MobyScene<'_>>, sounds: &mut dyn HeroSounds, rng: &mut Rng) {
    use super::physics::ticks;
    if h.mode != 0 { return; }
    if h.state == 2 && h.timer < ticks(0xf) { return; }
    let view = anim.view();
    let class = h.footstep;
    let mut step = |h: &mut Hero, foot: u8| {
        footstep(h, moby, class, foot, sounds, rng);
        if let Some(p) = footprint(h, anim, Some(moby), coll, mobys, foot) { h.fx.parts.push(p); }
    };
    if h.state == 2 && h.timer == ticks(0x16) { step(h, 0); }
    let passed = |f: f32| super::comet::passed(&view, f);
    let keys: &[(f32, u8)] = match view.seq_b {
        3 => &[(49.5, 0), (17.0, 1)],
        4 => &[(12.5, 0), (1.0, 1)],
        _ => &[],
    };
    if view.blending() { return; }
    for &(f, foot) in keys {
        if passed(f) { step(h, foot); }
    }
}

/// The two-foot pair at a state change (`0x228e58` with its sound flag clear): the fall landing and level 12's
/// idle → walk entry. Both feet in order, at the pre-write-back frame ([`HeroFx::frame`]).
pub(super) fn footprints(h: &mut Hero, anim: &dyn super::AnimCtl, env: &super::physics::Env) {
    for foot in [0, 1] {
        if let Some(p) = footprint(h, anim, None, env.coll, env.mobys, foot) { h.fx.parts.push(p); }
    }
}

/// The feet' joint lists (`MobyJointPoint(hero, 0x16 + foot)`; the foot motes' lists 22 / 23).
const FOOT_LISTS: [usize; 2] = [0x16, 0x17];

/// The level-12 tail of the footstep wrapper `0x228e58`: a snow footprint decal.
///
/// The wrapper plays `PlayFootstepSound` (when its variant flag is set) and then, on Hoven only, with footstep
/// class 0 (0x14063d) and level ground under the feet (0x13f64c = 0), takes the foot's joint point
/// (`MobyJointPoint(hero, 0x16 + foot)`), moves it 0.15 ahead along Ratchet's yaw (`FastCos` on x, `FastSin` on
/// y), snaps it with `GroundHeight(0.5)` (`0x26e618`: a line from z + 0.5 down to 0.01) and raises it 0.04. The
/// decal's frame is 1 for the left foot, 0 for the right. No RNG. `moby` is the walk's current moby (its call
/// point is after the write-back); `None` uses [`HeroFx::frame`] (the landing, before the write-back).
fn footprint(h: &Hero, anim: &dyn super::AnimCtl, moby: Option<&Moby>, coll: &Collision, mobys: Option<&MobyScene<'_>>, foot: u8) -> Option<PartSpawn> {
    if h.mode != 0 || h.idle.level != 12 || h.footstep != 0 || h.ground_moby.is_some() { return None; }
    let list = *FOOT_LISTS.get(usize::from(foot))?;
    let frame = moby.map_or_else(|| h.fx.frame.clone(), moby_frame);
    let mut pos = joint_point_in(h, anim, &frame, list);
    let (cos, sin) = (super::physics::fast_cos(h.rot[2]), super::physics::fast_sin(h.rot[2]));
    let ahead = Pf::b(0x3e19_999a); // 0.15
    pos[0] = (Pf::f(pos[0]) + cos * ahead).to_f32();
    pos[1] = (Pf::f(pos[1]) + sin * ahead).to_f32();
    let at = [Pf::f(pos[0]), Pf::f(pos[1]), Pf::f(pos[2]), Pf::ZERO];
    let z = super::physics::ground_height(coll, mobys, Pf::b(0x3f00_0000), at, 0) + Pf::b(0x3d23_d70a);
    Some(PartSpawn::Footprint { angle: h.rot[2].to_f32(), pos: [at[0], at[1], z, Pf::ZERO].map(Pf::to_f32), frame: usize::from(foot == 0) })
}

/// Ratchet's moby as [`JointFrame`] (the walk's call point is after the write-back).
fn moby_frame(m: &Moby) -> JointFrame {
    JointFrame { rows: [m.rows[0], m.rows[1], m.rows[2]], position: m.position, scale: m.scale, mods: m.joint_mods.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Log(Vec<(i32, u32)>);
    impl HeroSounds for Log {
        fn anim_advanced(&mut self, _: &Moby, _: &super::super::AnimView, _: &super::super::AnimView, _: &mut Rng) {}
        fn voice(&mut self, _: &Moby, index: i32, flags: u32, _: &mut Rng) -> i32 {
            self.0.push((index, flags));
            0
        }
        fn footstep(&mut self, _: &Moby, level: i32, class: u8, foot: u8, variant: u8, _: &mut Rng) -> i32 {
            self.0.push((1000 * level + 100 * class as i32 + 10 * foot as i32 + variant as i32, 0xf));
            0
        }
    }

    fn anim(seq_a: u8, seq_b: u8, frame: f32, step: f32) -> super::super::RecordingAnim {
        let mut a = super::super::RecordingAnim::default();
        a.v.seq_a = seq_a;
        a.v.seq_b = seq_b;
        a.v.frame = frame;
        a.v.frame_step = step;
        a
    }

    /// `FUN_00227e90`: the run (sequence 4) steps at key times 12.5 (left) and 1.0 (right) when passed this tick, none
    /// during a blend or in the first 15 ticks of state 2; the landing's two steps from the queue.
    #[test]
    fn walk_and_landing_footsteps() {
        let mut h = Hero::new();
        h.state = 2;
        h.timer = 40;
        h.footstep = 1;
        h.idle.level = 1;
        let moby = Moby::zeroed();
        let coll = super::super::testkit::floor(0.0, 100, 106, 100, 106);
        let mut rng = Rng::new();
        let mut log = Log(Vec::new());
        walk_footsteps(&mut h, &moby, &anim(4, 4, 12.6, 0.3), &coll, None, &mut log, &mut rng);
        walk_footsteps(&mut h, &moby, &anim(4, 4, 12.9, 0.3), &coll, None, &mut log, &mut rng);
        walk_footsteps(&mut h, &moby, &anim(4, 4, 1.2, 0.3), &coll, None, &mut log, &mut rng);
        assert_eq!(log.0, vec![(1100, 0xf), (1110, 0xf)]);
        walk_footsteps(&mut h, &moby, &anim(0xff, 4, 12.6, 0.3), &coll, None, &mut log, &mut rng);
        h.timer = 10;
        walk_footsteps(&mut h, &moby, &anim(4, 4, 12.6, 0.3), &coll, None, &mut log, &mut rng);
        assert_eq!(log.0.len(), 2);
        h.fx.footsteps.extend([(0, 0), (0, 1)]);
        flush(&mut h, &moby, &mut log, &mut rng);
        assert_eq!(&log.0[2..], &[(1000, 0xf), (1010, 0xf)]);
    }

    /// The surfacing gasps: queued twice (27 and 40 ticks), played when each timer runs out; the splash voice at
    /// once.
    #[test]
    fn delayed_voices_play_when_their_timers_run_out() {
        let mut h = Hero::new();
        let moby = Moby::zeroed();
        let mut rng = Rng::new();
        let mut log = Log(Vec::new());
        begin(&mut h, &moby);
        h.swim.events.push(SwimEvent::Sound(3));
        h.swim.events.push(SwimEvent::Voice(7, 27));
        h.swim.events.push(SwimEvent::Voice(7, 40));
        flush(&mut h, &moby, &mut log, &mut rng);
        assert_eq!(log.0, vec![(3, 0)]);
        let mut at = Vec::new();
        for t in 1..60 {
            begin(&mut h, &moby);
            let n = log.0.len();
            flush(&mut h, &moby, &mut log, &mut rng);
            if log.0.len() > n { at.push((t, log.0[n])); }
        }
        // Counted down once on the queueing tick too: 26 / 39 ticks later.
        assert_eq!(at, vec![(26, (7, 0)), (39, (7, 0))]);
        assert!(h.fx.voices.iter().all(|e| !e.active));
    }

    /// The ledge climb's voice: 0x1c with the Thruster-Pack as the back item (0x1404f8 = 3) and Clank shown, on the
    /// tick the anim passes frame 10 (`0x236738(4, 0)`), queued for the physics' flush; none with the Heli-Pack.
    #[test]
    fn ledge_climb_voice_with_the_thruster() {
        use super::super::packs::SoundCmd;
        let coll = super::super::testkit::floor(100.0, 100, 106, 100, 106);
        let pad = crate::pad::PadState::default();
        let (cam_rows, cam_yaw) = super::super::testkit::cam_x();
        let env = super::super::Env { coll: &coll, pad: &pad, cam_yaw, cam_rows, mirror: false, death_z: crate::ps2v::Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
        for (back, frame, want) in [(3, 10.5, true), (3, 11.5, false), (2, 10.5, false)] {
            let mut h = Hero::spawn([410.0, 410.0, 102.0], 0.0);
            h.state = 0x1c;
            h.back_slot.slot.id = back;
            let mut anim = super::super::RecordingAnim::default();
            anim.v.frame = frame;
            anim.v.frame_step = 1.0;
            super::super::ledge::physics(&mut h, &env, &mut anim, &mut Rng::new());
            assert_eq!(h.packs.sounds.contains(&SoundCmd::Voice { index: 4, flags: 0 }), want, "back {back} frame {frame}");
        }
    }

    /// Hoven's walk: each step spawns the type-70 decal at the foot joint, 0.15 ahead along the yaw, 0.04 above
    /// the ground, with the left / right texture frame. Nothing on another level, for a footstep class other than
    /// 0, or while the ground is a moby.
    #[test]
    fn hoven_walk_leaves_snow_footprints() {
        let coll = super::super::testkit::floor(100.0, 20, 40, 20, 40);
        let mut h = Hero::spawn([100.0, 100.0, 102.0], 0.0);
        h.state = 2;
        h.timer = 40;
        h.footstep = 0;
        h.idle.level = 12;
        h.ground_moby = None;
        let mut moby = Moby::zeroed();
        moby.position = h.pos.map(Pf::to_f32);
        moby.scale = 1024.0;
        moby.rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        let mut log = Log(Vec::new());
        let mut rng = Rng::new();
        let step = |h: &mut Hero, log: &mut Log, rng: &mut Rng| {
            walk_footsteps(h, &moby, &anim(4, 4, 12.6, 0.3), &coll, None, log, rng);
        };
        step(&mut h, &mut log, &mut rng);
        assert_eq!(log.0, vec![(12000, 0xf)]);
        let PartSpawn::Footprint { angle, pos, frame } = h.fx.parts[0] else { panic!("no footprint: {:?}", h.fx.parts) };
        assert_eq!((angle, frame), (0.0, 1), "left foot, frame 1");
        assert!((pos[0] - 100.15).abs() < 1e-4 && (pos[1] - 100.0).abs() < 1e-4 && (pos[2] - 100.04).abs() < 1e-4, "{pos:?}");
        h.idle.level = 11;
        h.fx.parts.clear();
        step(&mut h, &mut log, &mut rng);
        assert!(h.fx.parts.is_empty(), "no footprints on another level");
        h.idle.level = 12;
        h.footstep = 1;
        h.fx.parts.clear();
        step(&mut h, &mut log, &mut rng);
        assert!(h.fx.parts.is_empty(), "no footprints for footstep class 1");
        h.footstep = 0;
        h.ground_moby = Some(0);
        h.fx.parts.clear();
        step(&mut h, &mut log, &mut rng);
        assert!(h.fx.parts.is_empty(), "no footprints on a moby");
    }

    /// The landing's two queued steps spawn both decals, with the frames 1 (left) / 0 (right).
    #[test]
    fn hoven_landing_leaves_two_snow_footprints() {
        let coll = super::super::testkit::floor(100.0, 20, 40, 20, 40);
        let mut h = Hero::spawn([100.0, 100.0, 102.0], 0.0);
        h.footstep = 0;
        h.idle.level = 12;
        let mut moby = Moby::zeroed();
        moby.position = h.pos.map(Pf::to_f32);
        moby.scale = 1024.0;
        moby.rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        begin(&mut h, &moby);
        let pad = crate::pad::PadState::default();
        let (cam_rows, cam_yaw) = super::super::testkit::cam_x();
        let env = super::super::Env { coll: &coll, pad: &pad, cam_yaw, cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
        footprints(&mut h, &anim(4, 4, 0.0, 0.0), &env);
        let frames: Vec<usize> = h.fx.parts.iter().filter_map(|p| match *p { PartSpawn::Footprint { frame, .. } => Some(frame), _ => None }).collect();
        assert_eq!(frames, vec![1, 0]);
    }

    /// Level 12's idle → walk transition (`tr_idle`, transitions case 0) spawns both feet's decals before the entry
    /// animation, with the frames 1 (left) / 0 (right).
    #[test]
    fn hoven_idle_to_walk_leaves_snow_footprints() {
        let coll = super::super::testkit::floor(100.0, 20, 40, 20, 40);
        let mut h = Hero::spawn([100.0, 100.0, 102.0], 0.0);
        h.idle.level = 12;
        h.footstep = 0;
        h.stick_mag = Pf::ONE;
        let mut moby = Moby::zeroed();
        moby.position = h.pos.map(Pf::to_f32);
        moby.scale = 1024.0;
        moby.rows = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        begin(&mut h, &moby);
        let pad = crate::pad::PadState::default();
        let (cam_rows, cam_yaw) = super::super::testkit::cam_x();
        let env = super::super::Env { coll: &coll, pad: &pad, cam_yaw, cam_rows, mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
        let mut anim = anim(0, 0, 0.0, 0.0);
        let mut rng = Rng::new();
        let mut c = super::super::states::Ctx { env: &env, anim: &mut anim, rng: &mut rng, voice: None };
        h.tr_idle(&mut c);
        assert_eq!(h.state, 2, "entered the walk");
        let frames: Vec<usize> = h.fx.parts.iter().filter_map(|p| match *p { PartSpawn::Footprint { frame, .. } => Some(frame), _ => None }).collect();
        assert_eq!(frames, vec![1, 0]);
    }

    /// The queued spawn reaches the pool as the game's type-70 record.
    #[test]
    fn footprint_spawn_creates_a_type70_record() {
        let mut sys = Particles::new(None, Vec::new());
        create_one(&mut sys, &PartSpawn::Footprint { angle: 0.0, pos: [1.0, 2.0, 3.0, 1.0], frame: 1 }, 0);
        assert_eq!(sys.pool.count, 1);
        let r = &sys.pool.recs[0];
        assert_eq!((r[0], r[1], r[3], r[9]), (type70::TYPE, 1, 0x44, 0x22));
    }

    #[test]
    fn full_queue_drops() {
        let mut h = Hero::new();
        for k in 0..9 { queue_voice(&mut h, k, 5, 0); }
        assert_eq!(h.fx.voices.iter().map(|e| e.sound).collect::<Vec<_>>(), (0..8).collect::<Vec<_>>());
    }
}

