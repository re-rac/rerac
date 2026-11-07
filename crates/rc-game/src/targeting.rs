//! **Targeting: the target list, the aim searches, the throw's landing preview and the ground reticle** (level01;
//! docs/plan/hero_gameplay.md §8).
//!
//! **One target list for everything.** `BuildActiveMobyList` 0x265548 builds, with the run list, the null-terminated
//! list `0x1abe80`: every moby of the run list whose mode has 0x1000 ("targetable"), in the run list's order
//! ([`target_list`]). Each target carries a **target record** (`FUN_002711f8`: with mode 0x20 the first word of the
//! pvar block; the creatures' damage record, `crate::moby_update::creature::header`): `+0x00` health, `+0x10` the
//! height of the aim point above the moby's position ([`aim_height`]), `+0x0a` / `+0x38..+0x3a` bytes other
//! searches read (a radius ×0.125, the head look's range, priority and height).
//!
//! **Per-weapon searches over it** (verified from the level01 decompiler output and disassembly): every reader of
//! `0x1abe80` has its own selection code with its own constants — there is no shared "auto-aim" routine:
//!
//! | reader | what it picks | rules |
//! |---|---|---|
//! | `0x2d8330` Bomb Glove (class 192, item 10) | the throw's aim point ([`aim_search`], [`BOMB_GLOVE`]) | range 15 (2D, from the launch point), within 45° of the reference yaw (first the facing's) or 90° within 4, elevation < 55°, at most 2 above the launch point, a clear line from the camera (flags 6: the world and the mobys' meshes); greedy: each accepted target becomes the reference (its yaw, its distance) for the rest of the list |
//! | `0x2351d0` → `0x22e238` / `0x22dff0` melee aim assist (wrench swings, the Comet-Strike, the glove throw 0x23 without a glove target) | the swing's facing | range 11, cone 50° (`0x2351d0(11, 50°, −1)`), score `d + diff·d` (+7 for a record flag), health ≠ 0 |
//! | `0x22c080` `HeroScanTargets` | the head-look target | every 7th tick; range = record `+0x38`, 110° cone, 60° pitch, score `d + diff·d` (−1.5 for the current one), record `+0x39` priority, a clear line from the head |
//! | `0x2c7d68` (the Devastator 11, class 157; `crate::hero::devastator::search`) | the missile's aim and lock | creatures (class type 5); within 2.5 a hard lock (60° / 45°), else the cone 10° (+40° gold) narrowed by the record's radius byte `+0x0a` ([`cone_miss`]), a clear camera line; a creature no missile holds becomes the lock |
//! | `0x2ca310` (the Blaster 15, class 168; `crate::hero::blaster::search`) | the shot's aim | range 20 (3D), the cone 9° narrowed by the radius byte ([`cone_miss`]), clear camera and muzzle lines (a blocked one ends the search), the yaw taken within 10° |
//! | `0x2e4bb8` (the R.Y.N.O. 23, class 454; `crate::hero::ryno::search`) | the salvo's targets | creatures with a record and health ≥ 0, drawn (or not yet taken this salvo), within 97° of the camera's yaw and elevation and 80 (the salvo's re-targets: 180°, 100), score `d/5` within 5 else `yaw²·pitch²·d + d`, a clear line (flags 2) |
//! | `0x2cf138` (the Tesla Claw 19, class 177; `crate::hero::tesla::build`) | the beam's target(s) | creatures with a record within 15 (2D) of the claw, 32° of the camera's yaw and 45° of Ratchet's; score `1.5·(32° − off)/32° + (15 − d)/15`, the best two (the second for the gold claw) |
//! | `0x2c5778` (the gold Devastator missile's re-target) | | not ported (gold not mirrored) |
//! | `0x2d2018` (the Morph-o-Ray 21, class 185; `crate::hero::morph_ray::search`) | the beam's target | creatures with a record, not 0x58e / 0x452; within 2.5 (60° of Ratchet, 2 in height) at once, else within 8 in the 10° cone ([`cone_miss`]), the line clear or blocked by the target only |
//! | `0x2bfe40` (the Mine Glove's mine, class 74; `crate::moby_update::classes::mine`) | the mine's seek | armed and landed: any target within 1.5 (3-D) sets it off; else the nearest (xy) non-crate within `4 + radius/8` (×3 lured), less than 2 above or below |
//! | `0x2d49f8`, `0x30d308` | others | their own ranges and cones (not ported) |
//!
//! So the system here is one list and one record reader shared by every weapon, plus a search per weapon: the Bomb
//! Glove's as a data row ([`AimRules`]), the guns' as their own functions (their rules differ in kind, not only in
//! constants), sharing the cone test [`cone_miss`] (the Blaster and the Devastator) and the record readers.
//!
//! **The ground reticle: the bomb's, the mine's and the decoy's.** The bomb (class 121) of the glove (class 0xc0) runs
//! the landing preview `0x2c2be0` every tick from its update `0x2c3300` ([`arc_landing`]): from the launch point (held)
//! or its position (flying), its velocity stepped 10 ticks at a time under gravity 11 u/s² (`p += v; v.z −= 11·dt²`), a
//! line test per step (`CollLine_Fix(.., 0x10, bomb)`) for at most 300 ticks (the fuse when flying). A world face ends
//! it at the hit; a moby's collision primitive **snaps** it to that moby's position (the hero and the glove only after
//! 10 ticks; a primitive only while the arc is less than 2 above its start); water only while falling. The point is
//! pulled 5 % toward the camera ([`pull_toward`]) and the draw callback `0x2c23c0` is registered on list 1
//! ([`Reticles`]): two 2×2 quads in the plane of the hit face's normal ([`reticle_quads`]), FX textures 0x11 and 0x12
//! at 0x80808080, ALPHA 0x48 (additive), turning once every 180 ticks in opposite directions. **Copies** (2026-09-28,
//! docs/plan/hero_gameplay.md §13; corrects §8's "the Bomb Glove's only"): the mine 74's preview `0x2bfa78` (gravity
//! 9.8, 120 ticks held, only faces end it: [`Prims::Ignore`]) with the draw `0x2bf420`, the decoy 203's `0x2d9760` and
//! the Glove of Doom's canister's `0x2de0a8` (gravity 9, 300 ticks, primitives snapped without the height rule) with
//! the draw `0x2d8e28` — the same quads and constants ([`BOMB_RETICLE`]). Both preview while held: their create leaves
//! the velocity 0 (the glove's aim fills it), so the first update (state 0) has nothing to preview and state 1 draws
//! every tick; only once the glove holds a new object does a thrown one's preview stop (its `+0x46` / `+0x3a`).
//!
//! Native `f32` throughout (`atan2` for `FastArcTan`).

use crate::moby_runtime::{mode, Moby, MobyId, MobyTable};
use crate::moby_update::creature::{add_rot, atan, diff_rots};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Mode bit 0x20: the pvar block starts with the header pointers (the target record first).
const PVAR_HEADER: u16 = 0x20;

/// `FUN_002711f8(moby)`: the offset of the target record in the pvars (mode 0x20 and a non-zero first word).
pub fn record(m: &Moby) -> Option<usize> {
    if m.mode & PVAR_HEADER == 0 || m.pvars.len() < 4 { return None; }
    let v = u32::from_le_bytes(m.pvars[0..4].try_into().ok()?) as usize;
    (v != 0 && v + 0x14 <= m.pvars.len()).then_some(v)
}

/// The missiles' `record +0x1e |= 0x80` (the target learns a missile is on it: the Devastator's `0x2c5440` and the
/// R.Y.N.O.'s `0x2e5738`). [`record`] only guarantees the record's first 0x14 bytes: a record that ends before
/// +0x20 is left alone instead of indexing past the pvars.
pub fn mark_missile(m: &mut Moby) {
    let Some(r) = record(m) else { return };
    let Some(b) = m.pvars.get_mut(r + 0x1e..r + 0x20) else { return };
    let f = u16::from_le_bytes([b[0], b[1]]) | 0x80;
    b.copy_from_slice(&f.to_le_bytes());
}

/// The record's `+0x10`: the aim point's height above the moby's position.
pub fn aim_height(m: &Moby) -> Option<f32> {
    let r = record(m)?;
    Some(f32::from_le_bytes(m.pvars[r + 0x10..r + 0x14].try_into().ok()?))
}

/// `0x1abe80`: the targetable mobys (mode 0x1000) of the run list, in its order.
pub fn target_list(table: &MobyTable, run_list: &[MobyId]) -> Vec<MobyId> {
    run_list.iter().copied().filter(|&id| table.mobys.get(id).is_some_and(|m| m.mode & mode::TARGETABLE != 0)).collect()
}

/// One weapon's aim search (the constants of its update).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimRules {
    /// Candidates closer than this (2D, from the launch point) are taken; each accepted one lowers it to its distance.
    pub range: f32,
    /// Accepted within `cone` of the reference yaw, or within `near_cone` when closer than `near`.
    pub cone: f32,
    pub near: f32,
    pub near_cone: f32,
    /// The aim point's elevation above the launch point (`atan2(dz, d)`) must be below this, and `dz` below `max_rise`.
    pub max_elevation: f32,
    pub max_rise: f32,
    /// `CollLine_Fix` flags of the line of sight from the camera to the aim point (6: the world mesh only).
    pub los_flags: u32,
}

/// The Bomb Glove's search (`0x2d8330`).
pub const BOMB_GLOVE: AimRules = AimRules {
    range: 15.0,
    cone: f32::from_bits(0x3f49_0fdb),
    near: 4.0,
    near_cone: f32::from_bits(0x3fc9_0fdb),
    max_elevation: f32::from_bits(0x3f75_bcd6),
    max_rise: 2.0,
    los_flags: 6,
};

/// A search's result: the target (`0x13fda0` for the glove), its aim point and the yaw to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimHit {
    pub id: MobyId,
    pub point: [f32; 3],
    pub yaw: f32,
}

/// The aim search of `rules` over the target list `list`, from `from` (the launch point) with the reference yaw
/// `yaw` (the Bomb Glove's: the facing): per target (live: state < 0x80, targetable, with a record) the aim point `position + (0, 0,
/// record +0x10)`; accepted when inside the cones and the range, low enough, and `clear(point)` (the line of sight);
/// the accepted target's yaw and distance become the reference for the targets after it (greedy, as the game).
pub fn aim_search(rules: &AimRules, table: &MobyTable, list: &[MobyId], from: [f32; 3], yaw: f32, mut clear: impl FnMut([f32; 3]) -> bool) -> Option<AimHit> {
    let (mut best, mut reference, mut out) = (rules.range, yaw, None);
    for &id in list {
        let Some(m) = table.mobys.get(id) else { continue };
        if m.state >= 0x80 || m.mode & mode::TARGETABLE == 0 { continue; }
        let Some(h) = aim_height(m) else { continue };
        let p = [m.position[0], m.position[1], m.position[2] + h];
        let d = ((p[0] - from[0]).powi(2) + (p[1] - from[1]).powi(2)).sqrt();
        let ty = atan(p[0] - from[0], p[1] - from[1]);
        let dz = p[2] - from[2];
        let elevation = atan(d, dz);
        let diff = diff_rots(reference, ty);
        let in_cone = diff < rules.cone || (d < rules.near && diff < rules.near_cone);
        if in_cone && d < best && elevation < rules.max_elevation && dz < rules.max_rise && clear(p) {
            out = Some(AimHit { id, point: p, yaw: ty });
            reference = ty;
            best = d;
        }
    }
    out
}

/// `FUN_00277b50(len, yaw, pitch, out)`: `len·(cos yaw·cos pitch, sin yaw·cos pitch, sin pitch)`.
pub fn polar(len: f32, yaw: f32, pitch: f32) -> [f32; 3] { [yaw.cos() * len * pitch.cos(), yaw.sin() * len * pitch.cos(), pitch.sin() * len] }

/// The guns' cone test (the Blaster's search `0x2ca310`, the Devastator's `0x2c7d68`, the same code with their own
/// constants): the point `dist` along the aim `(yaw, pitch)` from `from` and the target's aim point `p` subtend
/// `2·asin(|chord| / 2·dist)`; above `threshold`, a target with a record has its radius byte `radius` (×0.125) taken
/// off (`− asin(r / dist)`, not below 0, only when `r < dist`). Compared with `threshold` by the caller.
pub fn cone_miss(dist: f32, yaw: f32, pitch: f32, from: [f32; 3], p: [f32; 3], radius: Option<u8>, threshold: f32) -> f32 {
    let a = polar(dist, yaw, pitch);
    let q = [a[0] + from[0], a[1] + from[1], a[2] + from[2]];
    let chord = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2)).sqrt();
    let mut ang = (chord / (dist + dist)).clamp(-1.0, 1.0).asin() * 2.0;
    if threshold < ang {
        if let Some(b) = radius {
            let r = b as f32 * 0.125;
            if r < dist {
                ang -= (r / dist).clamp(-1.0, 1.0).asin();
                if ang < 0.0 { ang = 0.0; }
            }
        }
    }
    ang
}

/// The record's radius byte `+0x0a` (the guns' cone tests).
pub fn record_radius(m: &Moby) -> Option<u8> { record(m).and_then(|r| m.pvars.get(r + 0x0a).copied()) }

/// The record's first word `+0x00` as a float (the creatures' health: the R.Y.N.O. and the Tesla Claw read it).
pub fn record_health(m: &Moby) -> Option<f32> { record(m).map(|r| f32::from_le_bytes(m.pvars[r..r + 4].try_into().unwrap())) }

/// A moby's aim point: its position raised by the record's `+0x10` (0.5 without a record: the guns' default).
pub fn aim_point(m: &Moby) -> [f32; 3] { [m.position[0], m.position[1], m.position[2] + aim_height(m).unwrap_or(0.5)] }

// ---------------------------------------------------------------------------------------------------------------
// The screen markers (0x1694c0: `FUN_0020fb60` registers, `FUN_0020fc40` draws in `DrawWorld`)

/// One screen marker (`FUN_0020fb60(size, angle, step, owner, rgba, pos, fx, mode, …)`, 0x30 bytes at 0x1694c0 + 0x30·i):
/// FX texture `fx` drawn as a 2D sprite of `40·size` pixels (`FUN_0020f838`), turned by `angle`, at the screen
/// projection of `at` (None: the screen centre 0x13e508 / 0x13e50c = (256, 208)), vertex colour `rgba` (R low),
/// ALPHA 0x44 (`DrawWorld` sets it right before). The guns' markers use `mode == -1` with FX 0x23 / 0x26 / 0x27, which
/// the register turns into draw mode 2 (one sprite anchored at its centre).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    pub size: f32,
    pub angle: f32,
    pub rgba: u32,
    pub at: Option<[f32; 3]>,
    pub fx: usize,
}

/// The marker list of one tick (at most three; the draw empties it, a new tick's first register drops the old ones;
/// Ratchet's hold state 0x72 empties it at the draw: not drawn).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Markers {
    pub tick: u64,
    pub list: Vec<Marker>,
}

impl Markers {
    /// `FUN_0020fb60`: refused in game mode 2 (the callers here never run in it) and past three entries.
    pub fn register(&mut self, tick: u64, m: Marker) {
        if self.tick != tick {
            self.tick = tick;
            self.list.clear();
        }
        if self.list.len() < 3 { self.list.push(m); }
    }
    pub fn of_tick(&self, tick: u64) -> &[Marker] { if self.tick == tick { &self.list } else { &[] } }
}

/// The game's screen (`UpdateViewContext`: 512 × 416, centre (256, 208), tangents 0.63 and 0.63·0.775 on NTSC).
pub const SCREEN: [f32; 2] = [512.0, 416.0];

/// A class's screen-space sprite of one frame (a draw callback's `0x203a38` sprite at the projection of a world point:
/// the Veldin boss beam's flash 1898 `0x2fb690`): centred on `at`'s projection, `half` game pixels each way, FX `fx`,
/// texel UVs per corner, one colour; blended (PRIM 0x154), drawn with the world's 2D overlays (`rc-engine`
/// marker_render).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenSprite {
    /// The tick that registered it.
    pub tick: u64,
    pub at: [f32; 3],
    pub half: f32,
    pub fx: u16,
    pub uv: [[i32; 2]; 4],
    pub rgba: u32,
}

/// The corners of a [`ScreenSprite`] centred on `c` in the order its draw sends them: (x + h, y − h), (x − h, y − h),
/// (x + h, y + h), (x − h, y + h).
pub fn screen_sprite_corners(c: [f32; 2], half: f32) -> [[f32; 2]; 4] {
    [[c[0] + half, c[1] - half], [c[0] - half, c[1] - half], [c[0] + half, c[1] + half], [c[0] - half, c[1] + half]]
}
pub const SCREEN_TAN: [f32; 2] = [0.63, 0.63 * 0.775];

/// `FUN_00218838`'s projection of a world point for a camera at `eye` with rows (forward, left, up), in screen pixels
/// (None behind the eye): camera x = −left (right), y = −up (down), z = forward.
pub fn project(eye: [f32; 3], rows: [[f32; 3]; 3], p: [f32; 3]) -> Option<[f32; 2]> { project_tan(eye, rows, p, SCREEN_TAN[0]) }

/// [`project`] with the view's horizontal half-angle tangent `tan_x` (0x16cf70, `Services::view_tan_x`: Gemlik's ship
/// widens it; the vertical one is `tan_x · 0.775`).
pub fn project_tan(eye: [f32; 3], rows: [[f32; 3]; 3], p: [f32; 3], tan_x: f32) -> Option<[f32; 2]> {
    let d = [p[0] - eye[0], p[1] - eye[1], p[2] - eye[2]];
    let dot = |a: [f32; 3]| a[0] * d[0] + a[1] * d[1] + a[2] * d[2];
    let (z, x, y) = (dot(rows[0]), -dot(rows[1]), -dot(rows[2]));
    if z <= 0.0 { return None; }
    let tan_y = tan_x * (SCREEN_TAN[1] / SCREEN_TAN[0]);
    Some([SCREEN[0] * 0.5 + x / z * (SCREEN[0] * 0.5 / tan_x), SCREEN[1] * 0.5 + y / z * (SCREEN[1] * 0.5 / tan_y)])
}

/// `FUN_0020f838` draw mode 2 (`fun_001f5ab0(x, y, w, w, angle, 0.5, 0.5, 0x3f, 0x3f, tex, …)`): the sprite's corners in
/// GS strip order (v0 v1 v2 v3) around the screen point `c`, side `40·size`, turned by `angle`; texel UVs 1..63 (the
/// `<< 4` of 0x3f with the 0x10 start).
pub fn marker_corners(m: &Marker, c: [f32; 2]) -> [[f32; 2]; 4] {
    let w = m.size * 40.0;
    let (s, co) = m.angle.sin_cos();
    // fun_001f5ab0: A = h·(sin, cos), B = w·(cos, −sin), anchor (0.5, 0.5): v0 = P + A/2 − B/2, v1 = P + A/2 + B/2,
    // v2 = P − A/2 − B/2, v3 = P − A/2 + B/2.
    let a = [w * s, w * co];
    let b = [w * co, -w * s];
    let at = |ka: f32, kb: f32| [c[0] + a[0] * ka + b[0] * kb, c[1] + a[1] * ka + b[1] * kb];
    [at(0.5, -0.5), at(0.5, 0.5), at(-0.5, -0.5), at(-0.5, 0.5)]
}

/// The corners' texel UVs in [`marker_corners`]' order.
pub const MARKER_UV: [[i32; 2]; 4] = [[1, 1], [63, 1], [1, 63], [63, 63]];

// ---------------------------------------------------------------------------------------------------------------
// The landing preview (0x2c2be0)

/// What a line test found (`CollLine_Fix` +0x18 moby, +0x1c kind, +0x20 point, +0x40 normal, `CollType`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHit {
    pub point: [f32; 3],
    /// The hit face's normal (any length; zero for a primitive).
    pub normal: [f32; 3],
    pub moby: Option<MobyId>,
    /// +0x1c: > 0 for a triangle (world or moby mesh), ≤ 0 for a moby's collision primitive.
    pub kind: i32,
    /// `CollType` 0x2151d8 (0 = water, −1 for a primitive).
    pub surface: i32,
}

/// Where the preview landed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    /// The landing point (a face hit), or the snapped moby's position.
    pub point: [f32; 3],
    /// The normal of the line test that ended it.
    pub normal: [f32; 3],
    /// The moby it snapped to (a collision-primitive hit).
    pub snapped: Option<MobyId>,
    /// The moby the collision output names once the search ends (`coll_hit.moby`): a face's moby; for a snap the ground
    /// line's (None on the world), or the snapped one when that line met nothing. The Decoy / Doom previews' modes test
    /// it.
    pub hit: Option<MobyId>,
}

/// The preview's inputs.
#[derive(Clone, Copy, Debug)]
pub struct ArcStart {
    /// The start (the launch point when held, the bomb's position when flying) and the velocity per tick.
    pub from: [f32; 3],
    pub vel: [f32; 3],
    /// The tick budget (300 held, the fuse left when flying).
    pub budget: i32,
    /// Gravity per tick² (11·dt²).
    pub gravity: f32,
    /// The first test's start (Ratchet's position at the start's height) and what it ignores (Ratchet's moby).
    pub first: [f32; 3],
    pub first_ignore: Option<MobyId>,
    /// What the stepped tests ignore (the bomb).
    pub ignore: Option<MobyId>,
    /// Mobys a primitive hit only counts on after 10 ticks of the budget (Ratchet, the glove).
    pub own: [Option<MobyId>; 2],
    /// The gold glove while walking (0x13e52a and state 1): after the first test the velocity's xy ×1.5, z ÷1.5.
    pub gold: bool,
    /// What a moby's collision primitive does to the arc (the copies differ: [`Prims`]).
    pub prims: Prims,
    /// Whether the first test (Ratchet → the start) can end the search (the Bomb and Decoy / Doom previews; the
    /// Mine Glove's records its point but always steps on).
    pub first_test: bool,
}

/// A collision primitive (a moby's sphere / cylinder, `CollOutput +0x1c ≤ 0`) on the arc, per copy of the preview.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prims {
    /// Snap to the moby (Ratchet / the glove only after 10 ticks of the budget); `max_rise`: only while the arc is
    /// less than that above its start (the bomb's `0x2c2be0`: 2; the decoy's `0x2d9760` and the Glove of Doom's
    /// `0x2de0a8`: no limit).
    Snap { max_rise: Option<f32> },
    /// Stepped over: only a face ends the arc (the mine's `0x2bfa78`).
    Ignore,
}

/// `0x2c2be0`'s search: the first test from Ratchet to the start, then the stepped arc (module doc). `line(a, b,
/// ignore)` is `CollLine_Fix(a, b, 0x10, ignore, 0)`; `position(id)` a moby's position; `ground(a, b)` the normal and
/// the moby (a moby mesh's; None for the world) of the face `CollLine_Fix(a, b, 2, 0, 0)` hits. A snap calls `GroundHeight(0.5, position + (0, 0, 0.5))`,
/// whose height the preview drops but whose line (from 1 above the moby down to z = 0.01) leaves its face's normal
/// in the collision output the preview then stores: a snapped reticle lies on the ground under the moby (the
/// primitive's `hit − centre` when that line meets nothing). None: the budget ran out (no reticle). Pure: the caller
/// keeps the lock bookkeeping and registers the draw.
pub fn arc_landing(
    s: &ArcStart,
    mut line: impl FnMut([f32; 3], [f32; 3], Option<MobyId>) -> Option<LineHit>,
    position: impl Fn(MobyId) -> [f32; 3],
    mut ground: impl FnMut([f32; 3], [f32; 3]) -> Option<([f32; 3], Option<MobyId>)>,
) -> Option<Landing> {
    let mut snap = |m: MobyId, h: &LineHit| {
        let p = position(m);
        match ground([p[0], p[1], p[2] + 1.0], [p[0], p[1], 0.01]) {
            Some((normal, under)) => Landing { point: p, normal, snapped: Some(m), hit: under },
            None => Landing { point: p, normal: h.normal, snapped: Some(m), hit: Some(m) },
        }
    };
    let own = |m: MobyId| s.own.contains(&Some(m));
    let ticks_300 = 300;
    let ticks_10 = 10;
    let mut n = s.budget;
    let (mut p, mut v) = (s.from, s.vel);
    // The first test (Ratchet → the start): a face, or a primitive (Ratchet / the glove only once 10 ticks are gone).
    let first = if s.first_test { line(s.first, p, s.first_ignore) } else { None };
    if let Some(h) = first {
        if h.surface != 0 || 0.0 < v[2] {
            let found = match h.moby {
                _ if h.kind > 0 => Some(Landing { point: h.point, normal: h.normal, snapped: None, hit: h.moby }),
                None => None,
                Some(m) if !own(m) || n < ticks_300 - ticks_10 => Some(snap(m, &h)),
                Some(_) => None,
            };
            // A found point with an empty budget draws nothing (`if n == 0 return 0`).
            if found.is_some() { return found.filter(|_| n != 0); }
        }
    }
    if s.gold { v = [v[0] * 1.5, v[1] * 1.5, v[2] / 1.5]; }
    let start_z = p[2];
    loop {
        // FastDecTimer: done when it reaches 0 (or was 0).
        if n == 0 { return None; }
        n = n.max(1) - 1;
        if n < 1 { return None; }
        let prev = p;
        for _ in 0..10 {
            p = [p[0] + v[0], p[1] + v[1], p[2] + v[2]];
            v[2] -= s.gravity;
        }
        let Some(h) = line(prev, p, s.ignore) else { continue };
        if h.surface == 0 && 0.0 < v[2] { continue; }
        if h.kind > 0 { return Some(Landing { point: h.point, normal: h.normal, snapped: None, hit: h.moby }); }
        let Some(m) = h.moby else { continue };
        let Prims::Snap { max_rise } = s.prims else { continue };
        if max_rise.is_some_and(|r| r <= p[2] - start_z) { continue; }
        if own(m) && !(n < ticks_300 - ticks_10) { continue; }
        return Some(snap(m, &h));
    }
}

/// `0x2c2be0`'s tail: the reticle point pulled 5 % toward the camera (`cam + (p − cam)·0.95`).
pub fn pull_toward(camera: [f32; 3], p: [f32; 3]) -> [f32; 3] {
    let k = f32::from_bits(0x3f73_3333);
    [camera[0] + (p[0] - camera[0]) * k, camera[1] + (p[1] - camera[1]) * k, camera[2] + (p[2] - camera[2]) * k]
}

// ---------------------------------------------------------------------------------------------------------------
// The reticle (draw callback 0x2c23c0)

/// How a reticle looks (the draw callback's constants).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReticleStyle {
    /// The FX textures of the two quads (`GetEffectTex`).
    pub fx: [usize; 2],
    /// Half the quad's side.
    pub half: f32,
    /// Ticks per turn (the second quad turns the other way).
    pub spin_ticks: u64,
    /// Vertex colour (GS bytes, 0x80 = 1.0).
    pub rgba: u32,
    /// ALPHA 0x48 (`Cs·As + Cd`).
    pub additive: bool,
}

/// The Bomb Glove's (`0x2c23c0`; the mine's `0x2bf420` and the decoy's `0x2d8e28` are copies): FX 0x11 / 0x12, 2×2,
/// 180 ticks a turn, 0x80808080, additive.
pub const BOMB_RETICLE: ReticleStyle = ReticleStyle { fx: [0x11, 0x12], half: 1.0, spin_ticks: 180, rgba: 0x8080_8080, additive: true };

/// One registered reticle: the point and normal the callback reads (the bomb's pvars +0x10 / +0x20).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reticle {
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub style: &'static ReticleStyle,
}

/// The reticle draw callbacks (`RegisterDrawCallback(0x2c23c0, bomb)`, list 1) of tick `tick`, in registration order.
/// A registration for a new tick drops the older ones (the game clears the lists at the start of each tick).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reticles {
    pub tick: u64,
    pub list: Vec<Reticle>,
}

impl Reticles {
    /// Registers `r` for tick `tick` (64 entries per list, as `RegisterDrawCallback`).
    pub fn register(&mut self, tick: u64, r: Reticle) {
        if self.tick != tick {
            self.tick = tick;
            self.list.clear();
        }
        if self.list.len() < 64 { self.list.push(r); }
    }
    /// The reticles registered in tick `tick` (none when that tick registered none).
    pub fn of_tick(&self, tick: u64) -> &[Reticle] { if self.tick == tick { &self.list } else { &[] } }
}

/// Euler rows (`fun_001fa050`, the moby convention).
fn euler(e: [f32; 3]) -> [[f32; 3]; 3] {
    let r = rc_formats::moby_light::rotation_rows(e);
    r.map(|row| [f32::from_bits(row[0]), f32::from_bits(row[1]), f32::from_bits(row[2])])
}

/// `FastNormalizeAngle`: into [−π, π).
fn normalize(a: f32) -> f32 { (a + PI).rem_euclid(TAU) - PI }

/// `0x2c23c0` for frame counter `counter` (0x15f5cc): the two quads (FX index, corners in GS strip order with the ST
/// `(0,1), (1,1), (0,0), (1,0)`). Each quad is `(±half, 0, ±half)` turned by `(0, ±spin, 0)` then by the Euler angles
/// `(π/2 − atan2(n.y, √(n.x² + n.z²)), atan2(n.x, n.z), 0)` of the normal (which take the quad's y axis onto it), at
/// the point; `spin = 2π·(counter mod 180)/180`, negated for the second quad.
pub fn reticle_quads(r: &Reticle, counter: u64) -> [(usize, [[f32; 3]; 4]); 2] {
    let n = r.normal;
    let a = add_rot(-atan((n[0] * n[0] + n[2] * n[2]).sqrt(), n[1]), FRAC_PI_2);
    let b = atan(n[2], n[0]);
    let e = euler([a, b, 0.0]);
    let period = r.style.spin_ticks.max(1);
    let spin = normalize(((counter % period) as f32 * TAU) / period as f32);
    let h = r.style.half;
    let corners = [[-h, 0.0, h], [h, 0.0, h], [-h, 0.0, -h], [h, 0.0, -h]];
    let quad = |k: usize| {
        let s = euler([0.0, if k & 1 != 0 { -spin } else { spin }, 0.0]);
        // fun_001fa378(out, E, S): out_i = Σ_k S_i[k]·E_k (the spin first, then the Euler rotation).
        let m: [[f32; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|c| s[i][0] * e[0][c] + s[i][1] * e[1][c] + s[i][2] * e[2][c]));
        let pts = corners.map(|v| std::array::from_fn(|c| v[0] * m[0][c] + v[1] * m[1][c] + v[2] * m[2][c] + r.point[c]));
        (r.style.fx[k], pts)
    };
    [quad(0), quad(1)]
}

/// The corners' texture coordinates, in [`reticle_quads`]' order.
pub const RETICLE_ST: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::Moby;

    fn target(pos: [f32; 3], h: f32) -> Moby {
        let mut m = Moby { position: [pos[0], pos[1], pos[2], 1.0], mode: mode::TARGETABLE | PVAR_HEADER, pvars: vec![0; 0x80], ..Moby::default() };
        m.pvars[0..4].copy_from_slice(&0x20u32.to_le_bytes());
        m.pvars[0x30..0x34].copy_from_slice(&h.to_le_bytes());
        m
    }

    fn table(ms: Vec<Moby>) -> MobyTable { MobyTable::new(ms, 0) }

    /// The missiles' mark: +0x1e |= 0x80 on a full record; a record cut short after +0x14 is left alone (no panic).
    #[test]
    fn missile_mark_is_bounded() {
        let mut m = target([0.0; 3], 0.5);
        mark_missile(&mut m);
        assert_eq!(m.pvars[0x20 + 0x1e], 0x80);
        let mut short = target([0.0; 3], 0.5);
        short.pvars.truncate(0x20 + 0x14);
        mark_missile(&mut short);
        assert_eq!(short.pvars.len(), 0x34);
    }

    #[test]
    fn the_list_and_the_record() {
        let mut a = target([1.0, 0.0, 0.0], 0.5);
        let b = { let mut m = target([2.0, 0.0, 0.0], 0.5); m.mode &= !mode::TARGETABLE; m };
        let t = table(vec![a.clone(), b]);
        assert_eq!(target_list(&t, &[1, 0]), vec![0]);
        assert_eq!(aim_height(&a), Some(0.5));
        a.mode &= !PVAR_HEADER;
        assert_eq!(aim_height(&a), None);
    }

    /// The glove's cones: 45° anywhere in range, 90° within 4; the range 15; the rise 2 and the 55° elevation; the
    /// line of sight; the closest accepted wins and becomes the reference.
    #[test]
    fn bomb_glove_search_rules() {
        let from = [100.0, 100.0, 50.0];
        let r = &BOMB_GLOVE;
        let at = |d: f32, deg: f32, dz: f32| [from[0] + d * deg.to_radians().cos(), from[1] + d * deg.to_radians().sin(), from[2] + dz];
        let one = |p: [f32; 3]| { let t = table(vec![target(p, 0.0)]); aim_search(r, &t, &[0], from, 0.0, |_| true).map(|h| h.id) };
        assert_eq!(one(at(10.0, 40.0, 0.0)), Some(0));
        assert_eq!(one(at(10.0, 50.0, 0.0)), None, "outside 45° beyond 4");
        assert_eq!(one(at(3.5, 80.0, 0.0)), Some(0), "90° within 4");
        assert_eq!(one(at(3.5, 95.0, 0.0)), None);
        assert_eq!(one(at(14.9, 0.0, 0.0)), Some(0));
        assert_eq!(one(at(15.1, 0.0, 0.0)), None, "range 15");
        assert_eq!(one(at(10.0, 0.0, 1.9)), Some(0));
        assert_eq!(one(at(10.0, 0.0, 2.1)), None, "at most 2 above");
        assert_eq!(one(at(1.0, 0.0, 1.9)), None, "elevation ≥ 55°");
        assert_eq!(one(at(10.0, 0.0, -30.0)), Some(0), "below is fine");
        let t = table(vec![target(at(10.0, 0.0, 0.0), 0.0)]);
        assert_eq!(aim_search(r, &t, &[0], from, 0.0, |_| false), None, "no line of sight");
        // The aim point is the record's height above the moby.
        let t = table(vec![target(at(10.0, 0.0, 0.0), 0.75)]);
        assert!((aim_search(r, &t, &[0], from, 0.0, |_| true).unwrap().point[2] - (from[2] + 0.75)).abs() < 1e-6);
        // Greedy: the closer target in list order wins; a later farther one is refused (range lowered).
        let t = table(vec![target(at(6.0, 30.0, 0.0), 0.0), target(at(9.0, 0.0, 0.0), 0.0), target(at(5.0, 60.0, 0.0), 0.0)]);
        let h = aim_search(r, &t, &[1, 0, 2], from, 0.0, |_| true).unwrap();
        // 1 (9 at 0°) → 0 (6 at 30°, reference now 0°) → 2 (5 at 60°: 30° off the new reference 30°).
        assert_eq!(h.id, 2);
        // Dead or untargetable mobys are skipped.
        let mut dead = target(at(5.0, 0.0, 0.0), 0.0);
        dead.state = 0xfe;
        let t = table(vec![dead]);
        assert_eq!(aim_search(r, &t, &[0], from, 0.0, |_| true), None);
    }

    fn start(from: [f32; 3], vel: [f32; 3]) -> ArcStart {
        ArcStart { from, vel, budget: 300, gravity: (1.0 / 3600.0) * 11.0, first: [from[0] - 0.8, from[1], from[2]], first_ignore: Some(9), ignore: Some(8), own: [Some(9), None], gold: false, prims: Prims::Snap { max_rise: Some(2.0) }, first_test: true }
    }

    /// Flat ground at z = 0: the landing point is where the stepped arc crosses it (the 10-tick steps, the line
    /// between them).
    #[test]
    fn arc_lands_on_the_ground() {
        let s = start([0.0, 0.0, 1.0], [8.5 / 60.0, 0.0, 5.0 / 60.0]);
        let ground = |a: [f32; 3], b: [f32; 3], _: Option<MobyId>| -> Option<LineHit> {
            if a[2] > 0.0 && b[2] <= 0.0 {
                let t = a[2] / (a[2] - b[2]);
                Some(LineHit { point: std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t), normal: [0.0, 0.0, 1.0], moby: None, kind: 0x1000, surface: 1 })
            } else { None }
        };
        let l = arc_landing(&s, ground, |_| [0.0; 3], |_, _| None).unwrap();
        // The analytic landing (continuous arc) of that throw: x = 8.5·t with 1 + 5t − 5.5t² = 0.
        let t = (5.0 + (25.0f32 + 22.0).sqrt()) / 11.0;
        assert!((l.point[0] - 8.5 * t).abs() < 0.15, "{:?} vs {}", l.point, 8.5 * t);
        assert_eq!(l.point[2], 0.0);
        assert_eq!(l.snapped, None);
        // Nothing to land on: the budget runs out, no reticle.
        assert_eq!(arc_landing(&s, |_, _, _| None, |_| [0.0; 3], |_, _| None), None);
    }

    /// A moby's primitive on the arc snaps the point to the moby's position; Ratchet's (own) only after 10 ticks;
    /// water only while falling; a primitive hit high above the start does not count.
    #[test]
    fn arc_snaps_to_a_moby() {
        let s = start([0.0, 0.0, 1.0], [8.5 / 60.0, 0.0, 5.0 / 60.0]);
        let prim = |m: MobyId, x: f32| move |a: [f32; 3], b: [f32; 3], _: Option<MobyId>| -> Option<LineHit> {
            (a[0] < x && x <= b[0]).then_some(LineHit { point: [x, 0.0, 1.0], normal: [0.0; 3], moby: Some(m), kind: -0x10, surface: -1 })
        };
        let pos = |m: MobyId| [m as f32, 1.0, 2.0];
        let l = arc_landing(&s, prim(5, 3.0), pos, |_, _| None).unwrap();
        assert_eq!((l.snapped, l.point, l.normal), (Some(5), [5.0, 1.0, 2.0], [0.0; 3]));
        // The ground line under the snapped moby (from 1 above it down to 0.01) gives the normal.
        let mut asked = None;
        let l = arc_landing(&s, prim(5, 3.0), pos, |a, b| { asked = Some((a, b)); Some(([0.0, 0.1, 1.0], None)) }).unwrap();
        assert_eq!((l.normal, asked), ([0.0, 0.1, 1.0], Some(([5.0, 1.0, 3.0], [5.0, 1.0, 0.01]))));
        // Ratchet's primitive in the first 10 ticks' step: ignored (the arc goes on and never lands).
        assert_eq!(arc_landing(&s, prim(9, 0.5), pos, |_, _| None), None);
        // Water (surface 0) while rising: ignored; while falling: lands.
        let water = |a: [f32; 3], b: [f32; 3], _: Option<MobyId>| -> Option<LineHit> {
            (a[0] < 1.0 && 1.0 <= b[0]).then_some(LineHit { point: [1.0, 0.0, 1.2], normal: [0.0, 0.0, 1.0], moby: None, kind: 0x1000, surface: 0 })
        };
        assert_eq!(arc_landing(&s, water, pos, |_, _| None), None);
        let s2 = start([0.0, 0.0, 1.0], [8.5 / 60.0, 0.0, -1.0 / 60.0]);
        assert!(arc_landing(&s2, water, pos, |_, _| None).is_some());
        // A primitive met more than 2 above the start is passed through.
        let high = start([0.0, 0.0, 1.0], [2.0 / 60.0, 0.0, 30.0 / 60.0]);
        let prim_high = |a: [f32; 3], b: [f32; 3], _: Option<MobyId>| -> Option<LineHit> {
            (a[2] < 4.0 && 4.0 <= b[2]).then_some(LineHit { point: b, normal: [0.0; 3], moby: Some(5), kind: -0x10, surface: -1 })
        };
        assert_eq!(arc_landing(&high, prim_high, pos, |_, _| None), None);
    }

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
    fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
    fn unit(a: [f32; 3]) -> [f32; 3] { let l = dot(a, a).sqrt(); a.map(|x| x / l) }

    /// The quads lie in the plane of the normal (up: flat on the ground; a wall: against it; an unknown zero normal:
    /// flat), centred on the point, 2 across, and turn once per 180 ticks, the second the other way.
    #[test]
    fn reticle_placement() {
        for n in [[0.0, 0.0, 1.0], [0.0, 0.0, 5.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.3, 0.4, 0.8], [-0.6, 0.2, 0.3], [0.0, 0.0, 0.0]] {
            let r = Reticle { point: [10.0, 20.0, 30.0], normal: n, style: &BOMB_RETICLE };
            for c in [0u64, 17, 90, 179] {
                for (k, (fx, q)) in reticle_quads(&r, c).into_iter().enumerate() {
                    assert_eq!(fx, BOMB_RETICLE.fx[k]);
                    let centre: [f32; 3] = std::array::from_fn(|i| (q[0][i] + q[1][i] + q[2][i] + q[3][i]) / 4.0);
                    assert!(dot(sub(centre, r.point), sub(centre, r.point)) < 1e-8);
                    let plane = unit(cross(sub(q[1], q[0]), sub(q[2], q[0])));
                    let want = if n == [0.0; 3] { [0.0, 0.0, 1.0] } else { unit(n) };
                    assert!((dot(plane, want).abs() - 1.0).abs() < 1e-4, "normal {n:?} tick {c}: plane {plane:?}");
                    assert!((dot(sub(q[1], q[0]), sub(q[1], q[0])).sqrt() - 2.0).abs() < 1e-4);
                }
            }
        }
        // The turn: 90 ticks = half a turn; the two quads turn opposite ways.
        let r = Reticle { point: [0.0; 3], normal: [0.0, 0.0, 1.0], style: &BOMB_RETICLE };
        let edge = |c: u64, k: usize| { let q = reticle_quads(&r, c)[k].1; unit(sub(q[1], q[0])) };
        assert!((dot(edge(0, 0), edge(90, 0)) + 1.0).abs() < 1e-4);
        assert!((dot(edge(180, 0), edge(0, 0)) - 1.0).abs() < 1e-4);
        // 30 ticks = 60° each way: 120° apart.
        let (a, b) = (edge(30, 0), edge(30, 1));
        assert!((dot(a, b) + 0.5).abs() < 1e-4, "opposite turns: {}", dot(a, b));
        assert!((dot(edge(0, 0), edge(0, 1)) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn pulled_toward_the_camera_and_registered_per_tick() {
        let p = pull_toward([0.0, 0.0, 10.0], [20.0, 0.0, 0.0]);
        assert!((p[0] - 19.0).abs() < 1e-4 && (p[2] - 0.5).abs() < 1e-4);
        let mut l = Reticles::default();
        let r = Reticle { point: p, normal: [0.0, 0.0, 1.0], style: &BOMB_RETICLE };
        l.register(5, r);
        l.register(5, r);
        assert_eq!(l.of_tick(5).len(), 2);
        assert!(l.of_tick(6).is_empty());
        l.register(7, r);
        assert_eq!((l.of_tick(7).len(), l.of_tick(5).len()), (1, 0));
    }
}
