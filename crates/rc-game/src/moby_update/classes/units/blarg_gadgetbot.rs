//! **Clank's gadgetbots, class 857** (level06 `0x2f0040` with its draw callback `0x2f37d0`; 4 placed (level 10's 31 run
//! the same class at `0x2d4078`); census U224; the name is descriptive [L]). Small robots that follow Clank and obey
//! his commands. Each starts asleep in a glass bubble (class 302, with a marker 303 under it that pulses while it
//! sleeps), shown only while Ratchet is Clank; a hit (mask 0x10000) shatters the bubble and wakes the bot. Within 16
//! (xy) of Ratchet's feet and 1.5 in height, a command (0x141610, `Bodies::command`) sends a bot that is following,
//! waiting, attacking or going to a pad to follow (1 → 3), wait (2 → 4), attack (3 → 5) or the nearest pad (4 → 7).
//! They walk with the shared walker, route round the walls of their arena path through its waypoint graph, and stop at
//! doors Ratchet went through until those open. Attacking bots share out the creatures near them (each creature gets as
//! many as its target record asks for, spread ±50° round it) and strike in their animation's key window. A bot reaching
//! a pad merges into it (the pad 1302 counts it down), a hit knocks it flying back home. Each glows (yellow; following
//! green, attacking red, to a pad blue) with three glow sprites on its joints, and its head grows with the cheat "Clank
//! has a large noggin" (0x15edb3). Read from the level06 decomp, its instance pvars and gp words. Native `f32`.
//!
//! **Pvars** (0x390, [`pv`]): +0x00 the wander record ([`walker::wr`]: its home is the bot's anchor), +0x30 the walker
//! record `J` ([`walker`]), +0x44 the turn spring's velocity, +0x80 the knockback record `K` ([`knock`]), +0xe0 home,
//! +0xf0 the progress check point, +0x100 the head manipulator, +0x140 / +0x150 / +0x160 joint lists 1..3's points,
//! +0x170 the bubble (302), +0x174 the target, +0x178 the next target, +0x17c the approach angle offset,
//! +0x184 the follow ticks, +0x188 the distance the bot got stuck at, +0x18c the tick of the last share-out, +0x190
//! (s16) a stuck flag (never set), +0x192 (s16) the candidate count, +0x194 a timer, +0x198 (s16) engaged, +0x19a
//! (s16) a share-out ran, +0x19c the moby under it, +0x1a0 / +0x1c0 the eight best candidates and their scores, +0x1e0
//! / +0x220 sixteen arena paths and their waypoint graphs, +0x27c the arena (−1 none), +0x284 the push-out spring's
//! speed, +0x288 the ground spring's speed, +0x28c the glow phase, +0x290 the glow colour, +0x294 the address of the
//! once-a-frame word, +0x2a0 / +0x2c0 eight doors and their cuboids, +0x2e0 / +0x2f0 this and last tick's position,
//! +0x300 the ledge limits, +0x304 the pad it merges into, +0x308 the head scale, +0x30c the group it searches (−1:
//! the run list), +0x310 the arena count. The port keeps moby pointers as index + 1.
//!
//! | address | what | port |
//! |---|---|---|
//! | top | d = xy distance to Ratchet (0x13f3d0), dz = \|z − his ground point z (0x13f5f8)\|; state ≠ 1: the hits (`0x2f3640`); within 16 and 1.5 in height in states 2..7: the listener count 0x17ec84 + 1 (`Globals::bot_listeners`, the command menu's gate), the command | [`update`], [`command`] |
//! | top | the cheat 0x15edb3: the head manipulator on list 0 (`AttachManipulator`), its scale `Approach`ed to 1 (state 1) or 2.3 by 0.1 a tick; off: detached | [`head`] (`manip`) |
//! | 0 | head 1; z = `GroundHeight(0.5)`; the marker 303 (`0x2d8b98`: drawn, owned by the bot); positions; its mission done (0x14c050) → deleted; the bubble 302 (`0x2d8510`: update / draw distance 0x60, targetable); home and anchor; wander steps 2·dt / 150°·dt; the walker (radius 300, step-up 0.51, ledges +0x300, top 3.1·dt, acc 0.02, brake 0.3, arrive 0.2, no gravity, ticks(4)); the arena count; glow phase `rand_angle`; → 1, collision off | [`init`] (`walker`) |
//! | 1 | the bubble whole (state < 2): bot and bubble shown and targetable with Clank (0x1413f4; the bubble's collision on), hidden for Ratchet; the glow; else each arena's graph (`0x270b68`) and the arena it sees (`0x270900`); radius 300 (375 with the cheat); shown, targetable, collision on → 3 | [`wake`] (`region::graph_init`, `region::visible_nodes`) |
//! | 2 | sequence 1; the wander (`0x2701c0`: 0.35 up, 0.25 down) | [`update`] (`walker::wander`) |
//! | 3 | off level 6: the arena both stand in; a closed door 0x3f7 whose cuboid holds Ratchet's moby but not the bot → 4; then +0xbc: 0 start (offset `randf_sym(8°, 20°)`); 1 walk (route, turn 300°·dt, top speed by distance, the step beyond 0.8, every 8th tick a 0.4 shove at other mobys, the stuck check every 14 ticks, close / stuck / 1.75 below → 2); 2 wait near (back to 0 beyond 1.8 of the anchor) | [`follow`], [`route`] |
//! | 4 | sequence 0; faces Ratchet | [`face`] |
//! | 5 | off level 6: the arena; +0xbc 0 start; 1 the share-out (`0x2f2250`) once a tick, chase the target (sequence 5, top speed 4.5·dt, ±offset unless it crosses a wall), a clear 0.3 line 0.4 ahead and 0.4 up hitting it → 2; 2 sequence 2 (frame 8), key 12..15 → a 2.0 hit (flags 0x10000); wrapped → 3 (30 ticks); 3 wait, then 1 or 4; 4 wait 60 ticks for a new target | [`attack`], [`share_out`], [`score`] |
//! | 7 | +0xbc 0: the nearest station (`0x2f2910`: 0x3b1 / 0x400 / 0x515 / 0x516 within 12 and 0.2 in height, not finished); 1: walk to it (its position, a pad's point one row ahead) | [`to_pad`], [`nearest_station`] |
//! | 8 | spiral into the pad (4·dt a tick, its yaw at 360°·dt), shrink within 1 with a glow (type 26 on list 3); at it: the pad's count − 1 and its sound (1 at 0), deleted | [`merge`] |
//! | 9 | the flight (`0x26b4a8`); landed or out: the death explosion (0.5, 13), home, 4, nine sparkle bursts | [`update`] (`knock::update`, `fx::death_explosion`) |
//! | `0x2f32f0` | the run list (or group +0x30c): a pad (0x400 / 0x516, state 7 only) not finished within 1 → 8; a dock (0x15f / 0x3b1 / 0x515) with a link: its slot + 1, sparkles, the bot placed round the link at slot·60°, → 4 | [`stations`], [`dock`] |
//! | tail | state ≠ 1: the walker settle (speed 0), the push-out eased by a spring (at least 0.25, at most its speed); not docked: six 0.51-up sphere pushes taking only the steep ones (over 30° from vertical), a ground step back beyond the ledge limit; gravity 7·dt² (cap 8·dt) onto `GroundHeight`, a spring up onto it; out of the world or 5 under home → home, 4 | [`settle`] |
//! | tail | the moby under it; a line down (1 up, 2 down) onto a 0x409, another bot or a crate → back; drawn: the shadow within 17 of the camera (+0x7f 0xd), the glow (`0x2f38c8`); the positions | [`update`], [`glow`] |
//! | `0x2f3640` | not 9 and not in sequence 2: a hit (mask 0x330001) not from Ratchet's moby or a bot: the flight (gravity 4/ticks(80)², arc 2 / 0.5, sequence 4, keys 10 / 20, air speed 2·dt), sound 2, → 9; the hit slot cleared | [`hit`] (`knock`) |
//! | `0x2f38c8` | grouped: colour by state, pulsing ×(1 + 0.5 sin) at 170°·dt, the glow tweened 0.1 toward it; joint lists 1..3; once a frame the draw | [`glow`] |
//! | `0x2f37d0` | each drawn bot of the group: three glow sprites (0.057, 0.114 with the cheat; pull 0.08) | [`glow_quads`] |
//!
//! | `0x2d85a0` (302) | the hit slot (0x10000) read and cleared; 0 → 1; 1: the dormant pulse glow, a hit shatters it, drawn → the draw; 2: the byte timer out → deleted | [`bubble_update`] |
//! | `0x2d8178` | the shatter: sound 0, six puffs (type 13), ten glass shards along the hit, hidden, 2, `ticks(15)` | [`shatter`] (`blarg_glass::shard_from`) |
//! | `0x2d8738` | the bubble's mesh (level06 0x1d3290..): culled quads, a reflected sphere map within 16 of the camera | [`bubble_quads`] |
//! | `0x2d8c20` (303) | the pulse while its bot sleeps, else eased to 0x80464646 | [`marker_update`] |
//!
//! [L] The sparkles' velocity starts from stack leftovers in the game (the dock's second burst from the first's last
//! point); the port starts from zero. The dock branch's result is the leftover return register (taken as found). The
//! moby under the bot is the last ground probe's (the game's last collision output).
use super::{FxQuad, FxQuads, GlowQuad};
use crate::moby_runtime::{mode, MobyId, MobyTable};
use crate::moby_update::creature::{self as c, attack, fx, knock, projectile, region, turn, walker, V};
use crate::moby_update::manip;
use crate::moby_update::scheduler::{self, GroupWalk};
use crate::moby_update::services::{pf, pv as v4, pvar as p, HitTemplate, Services, World};
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 6;
pub const UPDATE_FN: u32 = 0x2f_0040;
pub const DRAW_FN: u32 = 0x2f_37d0;
pub const CLASSES: [i16; 1] = [857];
/// Level 10's copy (`0x2d4078`, census U351, 31 placed; its glow `0x2d7808`): the same code and callees (checked
/// function by function); the code identity of the class tables does not pair them, so it has its own row.
pub const ORXON_LEVEL: u32 = 10;
pub const ORXON_FN: u32 = 0x2d_4078;
pub const BUBBLE_FN: u32 = 0x2d_85a0;
pub const BUBBLE_DRAW_FN: u32 = 0x2d_8738;
pub const BUBBLE_CLASSES: [i16; 1] = [302];
pub const MARKER_FN: u32 = 0x2d_8c20;
pub const MARKER_CLASSES: [i16; 1] = [303];

const BOT: i16 = 0x359;
const PAD: i16 = 0x516;
const PAD_B: i16 = 0x400;
const DOCK: i16 = 0x515;
const DOCK_B: i16 = 0x15f;
const DOCK_C: i16 = 0x3b1;
const DOOR: i16 = 0x3f7;
const SOLID: i16 = 0x409;
const BUBBLE: i16 = 0x12e;
const MARKER: i16 = 0x12f;
const DEG: f32 = 0.017_453_292;
const TURN: f32 = 300.0 * DEG;
const TURN_SLOW: f32 = 200.0 * DEG;
const PVARS: usize = 0x390;

/// Pvar offsets (module doc).
pub mod pv {
    pub const ANCHOR: usize = 0x00;
    pub const WANDER_STEP: usize = 0x14;
    pub const WANDER_TURN: usize = 0x18;
    pub const J: usize = 0x30;
    pub const TURN_V: usize = 0x44;
    pub const K: usize = 0x80;
    pub const HOME: usize = 0xe0;
    pub const CHECK: usize = 0xf0;
    pub const HEAD: usize = 0x100;
    pub const JOINTS: usize = 0x140;
    pub const BUBBLE: usize = 0x170;
    pub const TARGET: usize = 0x174;
    pub const NEXT: usize = 0x178;
    pub const OFFSET: usize = 0x17c;
    pub const TICKS: usize = 0x184;
    pub const STUCK_D: usize = 0x188;
    pub const STAMP: usize = 0x18c;
    pub const STUCK: usize = 0x190;
    pub const N: usize = 0x192;
    pub const TIMER: usize = 0x194;
    pub const ENGAGED: usize = 0x198;
    pub const SHARED: usize = 0x19a;
    pub const UNDER: usize = 0x19c;
    pub const CANDS: usize = 0x1a0;
    pub const SCORES: usize = 0x1c0;
    pub const PATHS: usize = 0x1e0;
    pub const GRAPHS: usize = 0x220;
    pub const REGION: usize = 0x27c;
    pub const PUSH_V: usize = 0x284;
    pub const GROUND_V: usize = 0x288;
    pub const GLOW_PH: usize = 0x28c;
    pub const GLOW: usize = 0x290;
    pub const WORD: usize = 0x294;
    pub const DOORS: usize = 0x2a0;
    pub const CUBOIDS: usize = 0x2c0;
    pub const PREV: usize = 0x2e0;
    pub const PREV2: usize = 0x2f0;
    pub const LEDGE: usize = 0x300;
    pub const PAD: usize = 0x304;
    pub const SCALE: usize = 0x308;
    pub const GROUP: usize = 0x30c;
    pub const REGIONS: usize = 0x310;
}

/// Walker record words (`J` + offset, [`walker`]).
mod j {
    pub const RADIUS: usize = 0x00;
    pub const UP: usize = 0x04;
    pub const LEDGE_UP: usize = 0x08;
    pub const LEDGE_DOWN: usize = 0x0c;
    pub const SPEED: usize = 0x10;
    pub const VZ: usize = 0x18;
    pub const ACC: usize = 0x1c;
    pub const BRAKE: usize = 0x20;
    pub const TOP: usize = 0x24;
    pub const W10: usize = 0x28;
    pub const ARRIVE: usize = 0x2c;
    pub const FLAGS: usize = 0x38;
    pub const W15: usize = 0x3c;
    pub const W16: usize = 0x40;
    pub const W17: usize = 0x44;
    pub const W18: usize = 0x48;
}

fn jf(w: &World, id: MobyId, o: usize) -> f32 { c::pf(w, id, pv::J + o) }
fn set_jf(w: &mut World, id: MobyId, o: usize, x: f32) { c::set_pf(w, id, pv::J + o, x) }

/// A moby pointer pvar (index + 1, 0 none).
fn link(w: &World, id: MobyId, o: usize) -> Option<MobyId> { usize::try_from(c::pi32(w, id, o) - 1).ok().filter(|&m| m < w.table.mobys.len()) }
fn set_link(w: &mut World, id: MobyId, o: usize, m: Option<MobyId>) { c::set_pi32(w, id, o, m.map_or(0, |m| m as i32 + 1)) }
/// A moby table index (−1 none).
fn index(w: &World, i: i32) -> Option<MobyId> { usize::try_from(i).ok().filter(|&m| m < w.table.mobys.len()) }
fn deleted(w: &World, m: MobyId) -> bool { matches!(w.m(m).state, 0xfd | 0xfe) }

/// `if (+0x53 != s) MobyAnimBlend(m, s, frame, ticks(7))`.
fn blend(w: &mut World, id: MobyId, s: u8, frame: i32) {
    let t = w.ticks(7);
    c::blend_to(w, id, s, frame, t);
}
fn cmd(w: &World, id: MobyId) -> u8 { w.m(id).cmd }
fn set_cmd(w: &mut World, id: MobyId, v: u8) { w.mm(id).cmd = v; }
fn set_state(w: &mut World, id: MobyId, s: u8) { w.mm(id).state = s; }

fn hero_ground_z(w: &World) -> f32 { f32::from_bits(w.hero.ground_point[2].0) }
fn heading(from: V, to: V) -> f32 { c::atan(to[0] - from[0], to[1] - from[1]) }
fn spring_turn(w: &mut World, id: MobyId, h: f32, acc: f32, max: f32) { turn::spring_turn2_pvar(w, id, h, acc * c::SPEED, 0.3 * c::SPEED, max * c::DT, pv::TURN_V); }
fn clank_cheat(svc: &Services) -> bool { svc.cheats.on(crate::cheats::slot::CLANK) }

fn path_of(w: &World, id: MobyId, i: usize) -> Option<usize> { usize::try_from(c::pi32(w, id, pv::PATHS + 4 * i)).ok() }
fn graph_of(w: &World, id: MobyId, i: usize) -> usize { usize::try_from(c::pi32(w, id, pv::GRAPHS + 4 * i)).unwrap_or(usize::MAX) }
fn region_of(w: &World, id: MobyId) -> Option<usize> { usize::try_from(c::pi32(w, id, pv::REGION)).ok().filter(|&r| r < 16) }

/// `GroundHeight(0.5, p, 0)` with the moby the line hit (the game's last collision output).
fn ground(w: &World, p: V) -> (f32, Option<MobyId>) {
    let a = [p[0], p[1], p[2] + 0.5, p[3]];
    let b = [p[0], p[1], 0.01, p[3]];
    match w.coll_line(v4(a), v4(b), 2, None) {
        Some(o) => (o.point[2], o.moby.filter(|_| 0 < o.kind)),
        None => (0.0, None),
    }
}

/// Level06 `0x2f0040` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < PVARS { w.mm(id).pvars.resize(PVARS, 0); }
    let start = c::pos(w, id);
    let hero = super::hero_pos(w);
    let d = c::dist2(start, hero);
    let dz = (start[2] - hero_ground_z(w)).abs();
    if w.m(id).state != 1 {
        hit(w, id);
        if d < 16.0 && (start[2] - hero[2]).abs() < 1.5 { command(w, id); }
    }
    head(w, id);
    let state_now = w.m(id).state;
    match state_now {
        0 => {
            if !init(w, id) { return; }
        }
        1 => {
            if !wake(w, id) { return; }
        }
        2 => {
            blend(w, id, 1, 0);
            walker::wander(w, id, f32::from_bits(0x3eb3_3333), 0.25, pv::ANCHOR);
        }
        3 => follow(w, id, d, dz),
        4 => face(w, id),
        5 => attack(w, id, start),
        7 => to_pad(w, id, start),
        8 => {
            merge(w, id);
            return;
        }
        9 => {
            let r = knock::update(w, id, pv::K);
            if r & 0x41 != 0 {
                let p0 = c::pos(w, id);
                fx::death_explosion(w, 0.5, 13.0, Some(id), p0, -1);
                if super::blarg_bot_pad::trace() { eprintln!("bots: tick {}: bot {id} knocked down at {:?}: back home to wait", w.counter, [p0[0], p0[1], p0[2]]); }
                let home = c::pv4(w, id, pv::HOME);
                c::set_pos(w, id, home);
                set_state(w, id, 4);
                let mut v = [0.0; 4];
                sparkles(w, home, &mut v);
                return;
            }
        }
        _ => {}
    }
    let docked = stations(w, id, w.m(id).state != 7);
    if docked == 2 { return; }
    let mut under = None;
    if w.m(id).state != 1 {
        let (out, u) = settle(w, id, start, docked);
        under = u;
        if out { return; }
    }
    set_link(w, id, pv::UNDER, under);
    // A line down from 1 above to 2 below (at least 0.5): onto a 0x409, another bot or a crate → back.
    let p0 = c::pos(w, id);
    let a = [p0[0], p0[1], p0[2] + 1.0, p0[3]];
    let b = [p0[0], p0[1], (p0[2] - 2.0).max(0.5), p0[3]];
    if let Some(m) = w.coll_line(v4(a), v4(b), 0, Some(id)).and_then(|o| o.moby) {
        let oc = w.m(m).o_class;
        if oc == SOLID || oc == BOT || crate::moby_update::classes::crate_::is_crate(w, Some(m)) { c::set_pos(w, id, start); }
    }
    if w.m(id).visible != 0 {
        let cam = w.camera_point();
        if c::dist3(c::pos(w, id), [cam[0], cam[1], cam[2], 0.0]) < 17.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0xd;
        }
        if w.m(id).visible != 0 { glow(w, id); }
    }
    let prev = c::pv4(w, id, pv::PREV);
    c::set_pv4(w, id, pv::PREV2, prev);
    let p1 = c::pos(w, id);
    c::set_pv4(w, id, pv::PREV, p1);
}

/// The command 0x141610 (module doc): only bots in states 2..7.
fn command(w: &mut World, id: MobyId) {
    if !(2..=7).contains(&w.m(id).state) { return; }
    w.svc.units.bot_listeners += 1;
    let state_now = w.m(id).state;
    match w.hero.bodies.command {
        1 if state_now != 3 => {
            set_state(w, id, 3);
            set_cmd(w, id, 0);
        }
        2 => set_state(w, id, 4),
        3 if state_now != 5 => {
            set_state(w, id, 5);
            set_cmd(w, id, 0);
        }
        4 => {
            set_cmd(w, id, 0);
            set_state(w, id, 7);
        }
        _ => {}
    }
}

/// The head manipulator with the cheat 0x15edb3 (module doc).
fn head(w: &mut World, id: MobyId) {
    if !clank_cheat(w.svc) {
        if manip::attached(w, id, pv::HEAD) { manip::detach(w, id, id, pv::HEAD); }
        return;
    }
    if !manip::attached(w, id, pv::HEAD) { manip::attach(w, id, 0, id, pv::HEAD); }
    let t = if w.m(id).state == 1 { 1.0 } else { f32::from_bits(0x4013_3333) };
    let mut s = c::pf(w, id, pv::SCALE);
    turn::approach(t, c::SPEED * 0.1, &mut s);
    c::set_pf(w, id, pv::SCALE, s);
    for k in 0..3 { c::set_pf(w, id, pv::HEAD + manip::rec::SCALE + 4 * k, s); }
    manip::sync(w, id, id, pv::HEAD);
}

/// `0x2d8b98` / `0x2d8510`: `CreateMoby(class)` at the bot (its yaw, light words), owned by it.
fn spawn_beside(w: &mut World, id: MobyId, class: i16) -> Option<MobyId> {
    let b = w.create_moby(class)?;
    let (rot, light, amb, pos) = { let m = w.m(id); (m.rotation[2], m.light, m.ambient, m.position) };
    let bubble = class == BUBBLE;
    let m = w.mm(b);
    m.update_dist = if bubble { 0x60 } else { 0 };
    m.draw_dist = 0x60;
    m.visible = 1;
    m.rotation[0] = 0.0;
    m.rotation[1] = 0.0;
    m.rotation[2] = rot;
    m.light = light;
    m.ambient = amb;
    m.position = pos;
    if bubble { m.mode |= mode::TARGETABLE; } else { m.parent = Some(id); }
    w.build_matrix(b);
    Some(b)
}

/// State 0 (module doc). False when the bot was deleted.
fn init(w: &mut World, id: MobyId) -> bool {
    c::set_pf(w, id, pv::SCALE, 1.0);
    let (z, _) = ground(w, c::pos(w, id));
    w.mm(id).position[2] = z;
    spawn_beside(w, id, MARKER);
    let p0 = c::pos(w, id);
    c::set_pv4(w, id, pv::PREV2, p0);
    c::set_pv4(w, id, pv::PREV, p0);
    let mission = w.m(id).mission as i32;
    if super::hints::mission_done(w, mission) {
        w.delete_moby(id);
        return false;
    }
    let comp = spawn_beside(w, id, BUBBLE);
    set_link(w, id, pv::BUBBLE, comp);
    let (z, _) = ground(w, c::pos(w, id));
    w.mm(id).position[2] = z;
    let p0 = c::pos(w, id);
    c::set_pv4(w, id, pv::ANCHOR, p0);
    c::set_pv4(w, id, pv::HOME, p0);
    let ledge = c::pf(w, id, pv::LEDGE);
    c::set_pf(w, id, pv::WANDER_STEP, c::DT + c::DT);
    c::set_pf(w, id, pv::WANDER_TURN, 150.0 * DEG * c::DT);
    c::set_pi32(w, id, pv::J + j::RADIUS, 300);
    set_jf(w, id, j::UP, f32::from_bits(0x3f02_8f5c));
    set_jf(w, id, j::LEDGE_DOWN, ledge);
    set_jf(w, id, j::TOP, c::DT * 3.0 + c::DT * 0.1);
    set_jf(w, id, j::ACC, c::SPEED * 0.02);
    set_jf(w, id, j::W10, f32::from_bits(0x3f32_b8c2));
    set_jf(w, id, j::ARRIVE, f32::from_bits(0x3e4c_cccd));
    set_jf(w, id, j::W15, c::SPEED * 0.03);
    set_jf(w, id, j::W16, c::SPEED * 0.3);
    set_jf(w, id, j::W17, std::f32::consts::PI * c::DT);
    set_jf(w, id, j::LEDGE_UP, ledge);
    set_jf(w, id, j::BRAKE, c::SPEED * 0.3);
    let t4 = w.ticks(4);
    c::set_pi32(w, id, pv::J + j::W18, t4);
    let f = c::pi32(w, id, pv::J + j::FLAGS) | 1;
    c::set_pi32(w, id, pv::J + j::FLAGS, f);
    let n = (0..16).filter(|&i| path_of(w, id, i).is_some()).count() as i32;
    c::set_pi32(w, id, pv::REGIONS, c::pi32(w, id, pv::REGIONS) + n);
    let a = w.rng.rand_angle();
    c::set_pf(w, id, pv::GLOW_PH, a);
    c::set_pi32(w, id, pv::REGION, -1);
    set_state(w, id, 1);
    w.mm(id).has_collision = false;
    true
}

/// State 1 (module doc). False when it returns before the tail.
fn wake(w: &mut World, id: MobyId) -> bool {
    if let Some(comp) = link(w, id, pv::BUBBLE) {
        if !deleted(w, comp) && w.m(comp).state < 2 {
            let coll = w.classes.info(w.m(comp).o_class).is_some_and(|i| i.has_collision);
            let clank = w.body() != 0;
            for m in [id, comp] {
                let mm = w.mm(m);
                if clank { mm.mode = (mm.mode & !0x41) | mode::TARGETABLE; } else { mm.mode = (mm.mode | 0x41) & !mode::TARGETABLE; }
            }
            w.mm(comp).has_collision = clank && coll;
            if w.m(id).visible != 0 { glow(w, id); }
            return false;
        }
    }
    if 0 < c::pi32(w, id, pv::REGIONS) {
        let pos = c::pos(w, id);
        for i in 0..16 {
            let Some(path) = path_of(w, id, i) else { continue };
            let graph = graph_of(w, id, i);
            region::graph_init(w, &[path], graph);
            if region::visible_nodes(w, 0.0, &[path], graph, pos) != 0 { c::set_pi32(w, id, pv::REGION, i as i32); }
        }
    }
    // gp−0x4f20 (0.27 / 0.35, the lifted point of the settle) is a dead store [L].
    c::set_pi32(w, id, pv::J + j::RADIUS, if clank_cheat(w.svc) { 0x177 } else { 300 });
    let m = w.mm(id);
    m.mode = (m.mode & !0x41) | mode::TARGETABLE;
    let oc = m.o_class;
    let coll = w.classes.info(oc).is_some_and(|i| i.has_collision);
    w.mm(id).has_collision = coll;
    set_state(w, id, 3);
    true
}

/// Off level 6 (states 3 and 5): the arena that holds both Ratchet and the bot (−1 none).
fn shared_region(w: &mut World, id: MobyId) {
    if w.svc.level == 6 { return; }
    c::set_pi32(w, id, pv::REGION, -1);
    if c::pi32(w, id, pv::REGIONS) <= 1 { return; }
    let (hero, pos) = (super::hero_pos(w), c::pos(w, id));
    for i in 0..16 {
        let Some(path) = path_of(w, id, i) else { continue };
        if region::point_in_polygon(w, path, hero) && region::point_in_polygon(w, path, pos) { c::set_pi32(w, id, pv::REGION, i as i32); }
    }
}

/// The way toward `to` (states 3, 5): straight without an arena; else `LineOfSightTest(0.1)` through its graph (`to`
/// when it finds nothing), and the bot kept 0.17 off its walls while that stays inside it.
fn route(w: &mut World, id: MobyId, to: V) -> V {
    let Some(r) = region_of(w, id) else { return to };
    let Some(path) = path_of(w, id, r) else { return to };
    let graph = graph_of(w, id, r);
    let pos = c::pos(w, id);
    let way = region::line_of_sight(w, 0.1, &[path], graph, pos, to).unwrap_or(to);
    let q = region::push_out(w, f32::from_bits(0x3e2e_147b), path, pos).unwrap_or(pos);
    if region::point_in_polygon(w, path, q) { c::set_pos(w, id, q); }
    way
}

/// A closed door Ratchet's moby went through (its cuboid holds him, not the bot).
fn door_shut_behind(w: &World, id: MobyId) -> bool {
    let Some(hero) = w.hero_moby.map(|m| w.m(m).position) else { return false };
    let pos = c::pos(w, id);
    for i in 0..8 {
        let (d, q) = (c::pi32(w, id, pv::DOORS + 4 * i), c::pi32(w, id, pv::CUBOIDS + 4 * i));
        if d == -1 || q == -1 { return false; }
        let Some(door) = index(w, d) else { continue };
        if w.m(door).o_class == DOOR && w.m(door).state != 4 && w.in_cuboid([hero[0], hero[1], hero[2]], q) && !w.in_cuboid([pos[0], pos[1], pos[2]], q) { return true; }
    }
    false
}

/// State 3 (module doc): `d` / `dz` are the top's distance to Ratchet and height over his ground point.
fn follow(w: &mut World, id: MobyId, d: f32, dz: f32) {
    shared_region(w, id);
    if door_shut_behind(w, id) {
        set_state(w, id, 4);
        return;
    }
    let hero = super::hero_pos(w);
    match cmd(w, id) {
        0 | 1 => {
            if cmd(w, id) == 0 {
                set_cmd(w, id, 1);
                c::set_pi32(w, id, pv::TICKS, 0);
                c::set_pi16(w, id, pv::STUCK, 0);
                let p0 = c::pos(w, id);
                c::set_pv4(w, id, pv::CHECK, p0);
                let o = w.rng.randf_sym(f32::from_bits(0x3e0e_fa35), f32::from_bits(0x3eb2_b8c2));
                c::set_pf(w, id, pv::OFFSET, o);
            }
            blend(w, id, 1, 0);
            let saved = c::pos(w, id);
            let way = route(w, id, hero);
            let pos = c::pos(w, id);
            let h = c::add_rot(heading(pos, way), c::pf(w, id, pv::OFFSET));
            spring_turn(w, id, h, 0.07, TURN);
            let mut top = jf(w, id, j::TOP);
            if 2.3 < d {
                turn::approach(c::DT * 3.0 * 1.35, c::DT2, &mut top);
            } else if d < 1.2 {
                turn::approach(c::DT * 3.0 * 0.77, c::DT2 + c::DT2, &mut top);
            } else {
                turn::approach(c::DT * 3.0 * 0.95, c::DT2, &mut top);
            }
            top = top.min(c::DT * 3.0 * 1.35);
            set_jf(w, id, j::TOP, top);
            let mut r = 0;
            if 0.8 < d {
                let mut out = [0.0; 4];
                r = walker::step(w, id, pv::J, 1.0, hero, &mut out);
            }
            if (w.counter & 7) == (super::kalebo_traffic::slot_phase(id) & 7) {
                let p0 = c::pos(w, id);
                attack::sphere_hit(w, 0.4, 1.0, 1.0, id, p0, 0x100_0000, 0, 1, 0);
            }
            let pos = c::pos(w, id);
            let d2 = c::dist2(pos, hero);
            let dz2 = (pos[2] - hero_ground_z(w)).abs();
            if let (_, Some(m)) = ground(w, pos) {
                if w.m(m).o_class == DOCK_C { c::set_pos(w, id, saved); }
            }
            let mut stuck = false;
            if r & 1 != 0 && c::pi32(w, id, pv::TICKS) % 14 == 0 {
                let pos = c::pos(w, id);
                stuck = c::dist2(c::pv4(w, id, pv::CHECK), pos) < jf(w, id, j::SPEED);
                if stuck { c::set_pf(w, id, pv::STUCK_D, c::dist2(hero, pos)); }
                c::set_pv4(w, id, pv::CHECK, pos);
            }
            if r & 0x20 != 0 {
                set_state(w, id, 4);
                c::set_pos(w, id, saved);
            }
            if d2 < 0.8 || stuck || 1.75 < dz2 {
                let p0 = c::pos(w, id);
                c::set_pv4(w, id, pv::ANCHOR, p0);
                set_cmd(w, id, 2);
            }
        }
        2 => {
            let d2 = c::dist2(c::pv4(w, id, pv::ANCHOR), hero);
            let far = if c::pi16(w, id, pv::STUCK) == 0 { 1.8 < d2 } else { c::pf(w, id, pv::STUCK_D) + 0.5 < d2 };
            if far && dz < 1.75 { set_cmd(w, id, 0); }
            blend(w, id, 0, 0);
            let pos = c::pos(w, id);
            spring_turn(w, id, heading(pos, hero), 0.05, TURN_SLOW);
        }
        _ => {}
    }
    c::set_pi32(w, id, pv::TICKS, c::pi32(w, id, pv::TICKS) + 1);
}

/// State 4: sequence 0, facing Ratchet.
fn face(w: &mut World, id: MobyId) {
    blend(w, id, 0, 0);
    let (pos, hero) = (c::pos(w, id), super::hero_pos(w));
    spring_turn(w, id, heading(pos, hero), 0.05, TURN_SLOW);
}

/// State 5 (module doc); `start` is the position at the top of the update.
fn attack(w: &mut World, id: MobyId, start: V) {
    shared_region(w, id);
    if cmd(w, id) == 0 {
        set_cmd(w, id, 1);
        set_link(w, id, pv::TARGET, None);
        c::set_pi16(w, id, pv::ENGAGED, 0);
        c::set_pi16(w, id, pv::SHARED, 0);
        let o = w.rng.randf_sym(f32::from_bits(0x3e0e_fa35), f32::from_bits(0x3eb2_b8c2));
        c::set_pf(w, id, pv::OFFSET, o);
        c::set_pi32(w, id, pv::STAMP, w.counter as i32);
    }
    let now = w.counter as i32;
    match cmd(w, id) {
        1 => {
            if c::pi32(w, id, pv::STAMP) < now { share_out(w); }
            let Some(t) = link(w, id, pv::TARGET) else {
                if c::pi16(w, id, pv::SHARED) != 0 {
                    c::set_pi16(w, id, pv::SHARED, 0);
                    let t60 = w.ticks(60);
                    c::set_pi32(w, id, pv::TIMER, t60);
                    set_cmd(w, id, 4);
                }
                return;
            };
            blend(w, id, 5, 0);
            let tp = w.m(t).position;
            let way = route(w, id, tp);
            let pos = c::pos(w, id);
            let h = c::add_rot(heading(pos, way), c::pf(w, id, pv::OFFSET));
            let (cs, sn) = c::cs(h);
            let probe = [pos[0] + cs * 3.0, pos[1] + sn * 3.0, pos[2], pos[3]];
            if let Some(path) = region_of(w, id).and_then(|r| path_of(w, id, r)) {
                if region::crosses(w, path, pos, probe) {
                    let mut o = c::pf(w, id, pv::OFFSET);
                    turn::approach_rot(0.0, c::DT * 90.0 * DEG, &mut o);
                    c::set_pf(w, id, pv::OFFSET, o);
                }
            }
            spring_turn(w, id, h, 0.07, TURN);
            set_jf(w, id, j::TOP, c::DT * 4.5);
            let mut out = [0.0; 4];
            let r = walker::step(w, id, pv::J, 1.0, tp, &mut out);
            if r & 0x20 != 0 {
                set_state(w, id, 4);
                c::set_pos(w, id, start);
            }
            let pos = c::pos(w, id);
            if c::dist2(pos, tp) < 1.5 {
                let yaw = c::yaw(w, id);
                let (cs, sn) = c::cs(yaw);
                let near = [pos[0] + cs * 0.4, pos[1] + sn * 0.4, pos[2] + 0.4, pos[3]];
                let far = [near[0] + cs * 0.3, near[1] + sn * 0.3, near[2], near[3]];
                if let Some(o) = w.coll_line(v4(far), v4(near), 0, Some(id)) {
                    if o.moby == Some(t) { set_cmd(w, id, 2); }
                }
            }
        }
        2 => {
            blend(w, id, 2, 8);
            if let Some(t) = link(w, id, pv::TARGET) {
                let k = c::ground::key_time(w, id);
                if (12.0..=15.0).contains(&k) {
                    let (cs, sn) = c::cs(c::yaw(w, id));
                    let tmpl = HitTemplate { dir: [pf(cs), pf(sn), Pf::ZERO, Pf::ZERO], attacker: Some(id), flags: 0x1_0000, b18: 0, b19: 0, h1a: 0, damage: pf(2.0), w20: 1 };
                    w.deliver_hit(t, &tmpl);
                }
            }
            if w.m(id).anim.flags & 2 != 0 {
                c::set_pi16(w, id, pv::SHARED, 0);
                let t30 = w.ticks(30);
                c::set_pi32(w, id, pv::TIMER, t30);
                set_cmd(w, id, 3);
            }
        }
        3 => {
            blend(w, id, 0, 0);
            let none = link(w, id, pv::TARGET).is_none();
            if c::pi16(w, id, pv::SHARED) != 0 && none {
                c::set_pi16(w, id, pv::SHARED, 0);
                let t60 = w.ticks(60);
                c::set_pi32(w, id, pv::TIMER, t60);
                set_cmd(w, id, 4);
            }
            if c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0 && link(w, id, pv::TARGET).is_some() { set_cmd(w, id, 1); }
        }
        4 => {
            if c::pi32(w, id, pv::STAMP) < now { share_out(w); }
            blend(w, id, 0, 0);
            let expired = c::dec_timer_pvar_i32(w, id, pv::TIMER) != 0;
            let shared = c::pi16(w, id, pv::SHARED) != 0;
            let target = link(w, id, pv::TARGET).is_some();
            if expired && shared && !target {
                set_state(w, id, 3);
                set_cmd(w, id, 0);
            } else if target {
                set_cmd(w, id, 1);
            } else if shared {
                c::set_pi16(w, id, pv::SHARED, 0);
            }
        }
        _ => {}
    }
}

/// `0x2f2148(bot, m, kind)`: xy distance + 10·the bot's turn to it + 3·the camera's (0x167558); +10 when `m` was not
/// drawn, +100 for kind 9.
fn score(w: &World, bot: MobyId, m: MobyId, kind: i32) -> f32 {
    let (a, b) = (w.m(bot).position, w.m(m).position);
    let h = heading(a, b);
    let mut s = c::dist2(a, b) + c::diff_rots(w.m(bot).rotation[2], h) * 10.0 + c::diff_rots(w.camera_yaw, h) * 3.0;
    if w.m(m).visible == 0 { s += 10.0; }
    if kind == 9 { s += 100.0; }
    s
}

/// `0x2f2250`: the share-out of the creatures (class type 5) among the attacking bots without a live target (module
/// doc): each bot's eight best (score, within 12 or 5 once engaged, 1 in height, a clear 0.25-up line), the targets by
/// vote (8 − rank), each taking up to its target record's count (+0x04) of the free bots with the best score for it,
/// those spread ±(n − 1)·15° (at most ±50°); every bot: the tick stamp, its target, shared, engaged with one.
fn share_out(w: &mut World) {
    let list = super::hints::run_list(w);
    let mut bots: Vec<MobyId> = Vec::new();
    for &m in &list {
        if bots.len() == 50 { break; }
        let mm = w.m(m);
        if mm.o_class != BOT || mm.state != 5 || mm.cmd == 2 || mm.pvars.len() < PVARS { continue; }
        if link(w, m, pv::TARGET).is_none_or(|t| deleted(w, t)) { bots.push(m); }
    }
    let creatures: Vec<MobyId> = list.iter().copied().filter(|&m| w.m(m).has_class && w.classes.info(w.m(m).o_class).is_some_and(|i| i.ty == 5)).collect();
    for &b in &bots {
        let mut cands: Vec<(MobyId, f32)> = Vec::new();
        let reach = if c::pi16(w, b, pv::ENGAGED) != 0 { 5.0 } else { 12.0 };
        for &m in &creatures {
            let (bp, mp) = (w.m(b).position, w.m(m).position);
            if reach < c::dist2(bp, mp) || 1.0 < (bp[2] - mp[2]).abs() { continue; }
            let a = [bp[0], bp[1], bp[2] + 0.25, bp[3]];
            let e = [mp[0], mp[1], mp[2] + 0.25, mp[3]];
            if w.coll_line(v4(a), v4(e), 2, Some(b)).is_some_and(|o| o.moby != Some(m)) { continue; }
            let s = score(w, b, m, 5);
            let at = cands.iter().position(|&(_, x)| s < x).unwrap_or(cands.len());
            if at < 8 {
                cands.insert(at, (m, s));
                cands.truncate(8);
            }
        }
        for (k, &(m, s)) in cands.iter().enumerate() {
            c::set_pi32(w, b, pv::CANDS + 4 * k, m as i32 + 1);
            c::set_pf(w, b, pv::SCORES + 4 * k, s);
        }
        c::set_pi16(w, b, pv::N, cands.len() as i16);
    }
    let mut targets: Vec<(MobyId, i8)> = Vec::new();
    for &b in &bots {
        for k in 0..c::pi16(w, b, pv::N) as usize {
            let Some(m) = link(w, b, pv::CANDS + 4 * k) else { continue };
            if 50 <= targets.len() { continue; }
            let vote = 8 - k as i8;
            match targets.iter_mut().find(|(t, _)| *t == m) {
                Some(e) => e.1 = e.1.wrapping_add(vote),
                None => targets.push((m, vote)),
            }
        }
    }
    for &b in &bots { set_link(w, b, pv::NEXT, None); }
    for _ in 0..targets.len() {
        let mut pick = 0;
        let mut best = 0i8;
        for (i, &(_, v)) in targets.iter().enumerate() {
            if best < v {
                best = v;
                pick = i;
            }
        }
        targets[pick].1 = -1;
        let t = targets[pick].0;
        let want = crate::moby_update::triggers::pvar_record(w.m(t)).map_or(1, |o| p::i16(&w.m(t).pvars, o + 4) as i32);
        let mut taken: Vec<MobyId> = Vec::new();
        while (taken.len() as i32) < want {
            let mut best_s = 9_999_999.0f32;
            let mut who = None;
            for &b in &bots {
                if link(w, b, pv::NEXT).is_some() { continue; }
                for k in 0..c::pi16(w, b, pv::N) as usize {
                    if link(w, b, pv::CANDS + 4 * k) != Some(t) { continue; }
                    let s = c::pf(w, b, pv::SCORES + 4 * k);
                    if s < best_s {
                        best_s = s;
                        who = Some(b);
                    }
                }
            }
            let Some(b) = who else { break };
            set_link(w, b, pv::NEXT, Some(t));
            taken.push(b);
        }
        if !taken.is_empty() {
            let n1 = (taken.len() - 1) as f32;
            let span = (n1 * f32::from_bits(0x3f06_0a92)).min(f32::from_bits(0x3fdf_66f3));
            let step = if 1 < taken.len() { span / n1 } else { 0.0 };
            for (k, &b) in taken.iter().enumerate() { c::set_pf(w, b, pv::OFFSET, c::add_rot(span * -0.5, k as f32 * step)); }
        }
    }
    let now = w.counter as i32;
    for &b in &bots {
        c::set_pi32(w, b, pv::STAMP, now);
        let next = c::pi32(w, b, pv::NEXT);
        c::set_pi32(w, b, pv::TARGET, next);
        c::set_pi16(w, b, pv::SHARED, 1);
        if next != 0 { c::set_pi16(w, b, pv::ENGAGED, 1); }
    }
}

/// The run list, or the members of group +0x30c.
fn search_list(w: &World, id: MobyId) -> Vec<MobyId> {
    match c::pi32(w, id, pv::GROUP) {
        -1 => super::hints::run_list(w),
        g => scheduler::group_walk(w, g, GroupWalk::Alive),
    }
}

/// Is `m` a station a bot can still use (a pad not finished, a dock with a free link)?
fn open_station(w: &World, m: MobyId) -> bool {
    let mm = w.m(m);
    match mm.o_class {
        PAD | PAD_B => mm.state != 2,
        DOCK => mm.pvars.len() >= 4 && p::i32(&mm.pvars, 0) != -1,
        DOCK_C => true,
        _ => false,
    }
}

/// `0x2f2910`: the nearest station within 12 (xy) and 0.2 in height (module doc) → +0x174; shared.
fn nearest_station(w: &mut World, id: MobyId) {
    set_link(w, id, pv::TARGET, None);
    let pos = c::pos(w, id);
    let mut best = 12.0;
    for m in search_list(w, id) {
        let mp = w.m(m).position;
        let d = c::dist2(pos, mp);
        if best < d || 0.2 < (pos[2] - mp[2]).abs() || !open_station(w, m) { continue; }
        best = d;
        set_link(w, id, pv::TARGET, Some(m));
    }
    c::set_pi16(w, id, pv::SHARED, 1);
}

/// A station's point: a dock's position, a pad's one row ahead.
fn station_point(w: &World, m: MobyId) -> V {
    let mm = w.m(m);
    if mm.o_class == DOCK || mm.o_class == DOCK_B { mm.position } else { c::add(mm.position, mm.rows[0]) }
}

/// State 7 (module doc); `start` is the position at the top of the update.
fn to_pad(w: &mut World, id: MobyId, start: V) {
    match cmd(w, id) {
        0 => {
            set_cmd(w, id, 1);
            nearest_station(w, id);
        }
        1 => {
            let Some(t) = link(w, id, pv::TARGET) else {
                if c::pi16(w, id, pv::SHARED) != 0 {
                    set_cmd(w, id, 0);
                    set_state(w, id, 3);
                }
                return;
            };
            blend(w, id, 1, 0);
            let tp = station_point(w, t);
            let pos = c::pos(w, id);
            spring_turn(w, id, heading(pos, tp), 0.07, TURN);
            let mut out = [0.0; 4];
            if walker::step(w, id, pv::J, 1.0, tp, &mut out) & 0x20 != 0 {
                set_state(w, id, 4);
                c::set_pos(w, id, start);
            }
        }
        _ => {}
    }
}

/// State 8: into the pad +0x304 (module doc).
fn merge(w: &mut World, id: MobyId) {
    let Some(pad) = link(w, id, pv::PAD) else {
        w.delete_moby(id);
        return;
    };
    let (pp, pyaw) = (w.m(pad).position, w.m(pad).rotation[2]);
    let pos = c::pos(w, id);
    let mut a = heading(pp, pos);
    let d = c::dist2(pp, pos);
    turn::approach_rot(pyaw, c::DT * std::f32::consts::TAU, &mut a);
    let d = d - c::DT * 4.0;
    if 0.0 < d {
        let (cs, sn) = c::cs(a);
        let m = w.mm(id);
        m.position[0] = cs * d + pp[0];
        m.position[1] = sn * d + pp[1];
        if d < 1.0 {
            let oc = w.m(id).o_class;
            w.mm(id).scale = super::class_scale(w, oc) * d;
            let life = w.ticks(2);
            projectile::part26_joint(w, (1.0 - d) * 0.5 * 210_000.0, id, 0x3000_407f, life, 3);
        }
        if w.m(id).visible != 0 { glow(w, id); }
        let m = w.mm(id);
        m.has_collision = false;
        m.mode &= !mode::TARGETABLE;
        return;
    }
    if w.m(pad).pvars.len() >= 0x18 {
        let left = c::pi32(w, pad, 0x14);
        if 0 < left { c::set_pi32(w, pad, 0x14, left - 1); }
        if super::blarg_bot_pad::trace() { eprintln!("bots: tick {}: bot {id} merged into pad {pad}: count left {}", w.counter, (left - 1).max(0)); }
        let snd = if c::pi32(w, pad, 0x14) == 0 { 1 } else { 0 };
        w.play_sound_as(snd, 0, pad, PAD);
    }
    w.delete_moby(id);
}

/// `0x2f32f0(m, not_seeking)`: the stations the bot touches (module doc). 2 merging into a pad, 1 docked, 0 none.
fn stations(w: &mut World, id: MobyId, not_seeking: bool) -> i32 {
    let mut result = 0;
    for m in search_list(w, id) {
        if dock(w, id, m, not_seeking, &mut result) { break; }
    }
    result
}

/// `0x2f2c28(m, P, other, not_seeking, &result)`: one station (module doc). True stops the scan.
fn dock(w: &mut World, id: MobyId, m: MobyId, not_seeking: bool, result: &mut i32) -> bool {
    let oc = w.m(m).o_class;
    let at = station_point(w, m);
    let pos = c::pos(w, id);
    if oc == PAD || oc == PAD_B {
        if !not_seeking && w.m(m).state != 2 && c::dist2(pos, at) < 1.0 {
            set_state(w, id, 8);
            set_link(w, id, pv::PAD, Some(m));
            *result = 2;
        }
        return false;
    }
    if !matches!(oc, DOCK | DOCK_B | DOCK_C) || 1.0 <= c::dist2(pos, at) || w.m(m).pvars.len() < 8 { return false; }
    let link_i = c::pi32(w, m, 0);
    if link_i < 0 { return false; }
    let slot = c::pi32(w, m, 4);
    c::set_pi32(w, m, 4, slot + 1);
    if oc == DOCK { w.play_sound_as(0, 0, m, DOCK); }
    let mut v = [0.0; 4];
    let last = sparkles(w, pos, &mut v);
    if let Some(l) = index(w, link_i) {
        let (lp, ly) = (w.m(l).position, w.m(l).rotation[2]);
        let a = c::add_rot(ly, crate::moby_update::services::fl(crate::moby_update::services::normalize_angle(pf(slot as f32 * 60.0 * DEG))));
        let p1 = [lp[0] + a.cos(), lp[1] + a.sin(), lp[2], lp[3] + 0.0];
        c::set_pos(w, id, p1);
    }
    let p1 = c::pos(w, id);
    c::set_pv4(w, id, pv::PREV2, p1);
    c::set_pv4(w, id, pv::PREV, p1);
    let mut v = last;
    sparkles(w, p1, &mut v);
    set_state(w, id, 4);
    *result = 1;
    if 1 < c::pi32(w, id, pv::REGIONS) {
        for i in 0..16 {
            let Some(path) = path_of(w, id, i) else { continue };
            let graph = graph_of(w, id, i);
            if region::visible_nodes(w, 0.0, &[path], graph, p1) != 0 { c::set_pi32(w, id, pv::REGION, i as i32); }
        }
    }
    true
}

/// Nine type-53 sparkle bursts round `at` (the dock's, the knockback's end): `s = randf(0.1, 1)`, a colour of three
/// `rand_range(0x40, 0xff)`, an offset (`rand_angle` cos · `randf(0, 0.25)`, sin · the same, `randf(0, 0.5)` up); `v`
/// scaled to `randf(0, dt)` with z `randf(2.5·dt, 5·dt)`; (0.07s, 0.7s, 15·dt², life `ticks(rand_range(15, 30))`, spin
/// ±1 by `randi(2)`). Returns the last point.
fn sparkles(w: &mut World, at: V, v: &mut V) -> V {
    let mut q = at;
    for _ in 0..9 {
        let s = w.rng.randf(0.1, 1.0);
        let r = w.rng.rand_range(0x40, 0xff) as u32;
        let g = w.rng.rand_range(0x40, 0xff) as u32;
        let b = w.rng.rand_range(0x40, 0xff) as u32;
        let a = w.rng.rand_angle();
        let x = a.cos() * w.rng.randf(0.0, 0.25);
        let a = w.rng.rand_angle();
        let y = a.sin() * w.rng.randf(0.0, 0.25);
        let z = w.rng.randf(0.0, 0.5);
        q = [x + at[0], y + at[1], z + at[2], at[3]];
        let l = w.rng.randf(0.0, c::DT);
        *v = c::set_len3(*v, l);
        v[2] = w.rng.randf(c::DT * 2.5, c::DT * 5.0);
        let n = w.rng.rand_range(15, 30);
        let life = w.ticks(n);
        let spin: i8 = if w.rng.randi(2) == 0 { -1 } else { 1 };
        w.part53(pf(s * 0.07), pf(s * 0.7), pf(c::DT2 * 15.0), v4(q), life, (b << 16) | (g << 8) | 0x7f00_0000 | r, 0, spin, v4(*v));
    }
    q
}

/// The tail's settle (states ≠ 1, module doc). Returns (out of the world → home and 4, the moby under it).
fn settle(w: &mut World, id: MobyId, start: V, docked: i32) -> (bool, Option<MobyId>) {
    let before = c::pos(w, id);
    let (speed, top) = (jf(w, id, j::SPEED), jf(w, id, j::TOP));
    set_jf(w, id, j::SPEED, 0.0);
    set_jf(w, id, j::TOP, 0.0);
    let mut out = [0.0; 4];
    walker::step(w, id, pv::J, 1.0, before, &mut out);
    set_jf(w, id, j::SPEED, speed);
    set_jf(w, id, j::TOP, top);
    w.mm(id).position[2] = before[2];
    let now = c::pos(w, id);
    if 0.005 < c::dist3(before, now) {
        let mut push = c::sub(now, before);
        let l = c::len3(push);
        if l < 0.25 { push = c::set_len3(push, 0.25); }
        c::set_pos(w, id, before);
        let mut x = -l;
        let mut v = c::pf(w, id, pv::PUSH_V);
        turn::spring(0.0, c::SPEED * 0.025, c::SPEED * 0.3, c::DT * 3.0, &mut x, &mut v);
        let l2 = c::len3(push);
        if l2 < v { v = l2; }
        c::set_pf(w, id, pv::PUSH_V, v);
        if v < l2 { push = c::set_len3(push, v); }
        let p1 = c::add(before, push);
        c::set_pos(w, id, p1);
    } else {
        c::set_pf(w, id, pv::PUSH_V, 0.0);
        c::set_pos(w, id, before);
    }
    let mut under = None;
    let floor = if w.m(id).state == 9 {
        c::pf(w, id, pv::HOME + 8)
    } else {
        if docked == 0 {
            let z0 = c::pos(w, id)[2];
            let r = c::pi32(w, id, pv::J + j::RADIUS) as f32 * (1.0 / 1024.0);
            let up = jf(w, id, j::UP);
            for _ in 0..6 {
                let p0 = c::pos(w, id);
                let centre = [p0[0], p0[1], p0[2] + up, p0[3]];
                let Some(o) = w.coll_sphere(v4(centre), pf(r), 6, Some(id)) else { break };
                let n = [o.normal[0], o.normal[1], o.normal[2], 0.0];
                if f32::from_bits(0x3f06_0a92) < c::atan(n[2], c::len2(n)) {
                    if let Some(pc) = o.pushed_centre { c::set_pos(w, id, [pc[0], pc[1], pc[2] - up, p0[3]]); }
                }
            }
            w.mm(id).position[2] = z0;
            let (g, _) = ground(w, c::pos(w, id));
            if c::pf(w, id, pv::J + j::LEDGE_DOWN) <= (start[2] - g).abs() {
                c::set_pos(w, id, start);
                c::set_pf(w, id, pv::PUSH_V, 0.0);
            }
        }
        let vz = (jf(w, id, j::VZ) - c::DT2 * 7.0).max(c::DT * -8.0);
        set_jf(w, id, j::VZ, vz);
        let z = c::pos(w, id)[2];
        let p0 = c::pos(w, id);
        let (g, u) = ground(w, [p0[0], p0[1], z + 0.25, p0[3]]);
        under = u;
        if z < g {
            let mut zz = z;
            let mut gv = c::pf(w, id, pv::GROUND_V);
            turn::spring(g, c::SPEED * 0.04, c::SPEED * 0.3, c::DT * 1.5, &mut zz, &mut gv);
            c::set_pf(w, id, pv::GROUND_V, gv);
            w.mm(id).position[2] = zz;
            set_jf(w, id, j::VZ, 0.0);
        } else if g < z {
            w.mm(id).position[2] = (z + vz).max(g);
        }
        c::pf(w, id, pv::HOME + 8)
    };
    let p0 = c::pos(w, id);
    if p0[2] < floor - 5.0 || !projectile::in_world(p0) {
        let home = c::pv4(w, id, pv::HOME);
        c::set_pos(w, id, home);
        set_state(w, id, 4);
        return (true, under);
    }
    (false, under)
}

/// `0x2f3640`: the hits (module doc).
fn hit(w: &mut World, id: MobyId) {
    let a = w.m(id).anim;
    if w.m(id).state != 9 && a.seq_a != 2 && a.seq_b != 2 {
        if let Some(h) = w.get_hit(id, 0x33_0001, false) {
            let from_bot = h.attacker.is_some_and(|m| w.m(m).o_class == w.m(id).o_class);
            if h.attacker != w.hero_moby && !from_bot {
                let k = pv::K;
                let t = w.ticks(80);
                c::set_pu8(w, id, k + 0x3d, 0);
                c::set_pi32(w, id, k + knock::k::FLAGS, 9);
                c::set_pf(w, id, k + knock::k::GRAVITY, 4.0 / (t * t) as f32);
                knock::ballistic(w, id, 2.0, 0.5, k);
                let dir = h.dir.map(|x| f32::from_bits(x.0));
                let (mut sp, mut up) = (c::pf(w, id, k + knock::k::SPEED), c::pf(w, id, k + knock::k::UP));
                let ang = knock::aim(dir, &mut sp, &mut up);
                c::set_pf(w, id, k + knock::k::SPEED, sp);
                c::set_pf(w, id, k + knock::k::UP, up);
                knock::start(w, id, k, ang, 4, 5, 2);
                c::set_pf(w, id, k + knock::k::KEY_APEX, 10.0);
                c::set_pf(w, id, k + knock::k::KEY_LAND, 20.0);
                c::set_pf(w, id, k + knock::k::AIR_SPEED, c::DT + c::DT);
                w.play_sound(2, 0, id);
                set_state(w, id, 9);
            }
        }
    }
    w.mm(id).hit_slot = 0xff;
}

/// `0x2f38c8`: the glow (module doc).
fn glow(w: &mut World, id: MobyId) {
    if w.m(id).group == -1 { return; }
    let base: u32 = match w.m(id).state {
        3 | 6 => 0x3228_aa28,
        5 | 9 => 0x3228_28aa,
        7 => 0x32aa_2828,
        _ => 0x3228_aaaa,
    };
    let ph = c::add_rot(c::pf(w, id, pv::GLOW_PH), c::DT * 170.0 * DEG);
    c::set_pf(w, id, pv::GLOW_PH, ph);
    let f = ph.sin() * 0.5 + 1.0;
    let ch = |sh: u32| -> u32 { ((((base >> sh) & 0xff) as f32 * f) as i32).min(0xff) as u32 };
    let target = (base & 0xff00_0000) | (ch(16) << 16) | (ch(8) << 8) | ch(0);
    let g = crate::particles::tween_color(0.1f32.to_bits(), w.m(id).glow, target);
    w.mm(id).glow = g;
    for k in 0..3 {
        let jp = w.joint_point(id, k + 1);
        c::set_pv4(w, id, pv::JOINTS + 0x10 * k, jp);
    }
    c::set_pi32(w, id, pv::GLOW, g as i32);
    let frame = w.counter as i32;
    let word = c::pi32(w, id, pv::WORD);
    if frame != 0 && w.svc.shared_i32(word) != frame {
        w.svc.set_shared_i32(word, frame);
        if let Some(i) = super::row(REFERENCE_LEVEL, DRAW_FN) { w.svc.draw_callbacks.register(crate::moby_update::classes::draw_callbacks::Callback::UnitGlow(i), id); }
    }
}

/// `0x2f37d0` for the group of `id`: three glow sprites on each drawn bot (module doc).
pub fn glow_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Vec<GlowQuad> {
    let Some(bot) = table.mobys.get(id) else { return Vec::new() };
    let size = if clank_cheat(svc) { f32::from_bits(0x3de9_78d5) } else { f32::from_bits(0x3d69_78d5) };
    let Some(Some(list)) = usize::try_from(bot.group).ok().and_then(|g| svc.groups.lists.get(g)) else { return Vec::new() };
    let mut out = Vec::new();
    for m in list.iter().filter_map(|&e| table.mobys.get((e & 0x7fff) as usize)) {
        if m.visible == 0 || m.o_class != bot.o_class || m.pvars.len() < PVARS { continue; }
        let rgba = p::u32(&m.pvars, pv::GLOW);
        for k in 0..3 {
            let q = p::v4f(&m.pvars, pv::JOINTS + 0x10 * k);
            out.push(GlowQuad { size, pull: f32::from_bits(0x3da3_d70a), point: [q[0], q[1], q[2]], rgba });
        }
    }
    out
}

// -------------------------------------------------------------------------------------------------
// The bubble 302 and the marker 303

/// Level06 data of the bubble's mesh ([`Bubble`]); level 10's copy (`0x2bd0e8`, its draw `0x2bd280`) reads the same
/// tables 0x300 lower ([`BUBBLE_MESH_SHIFT`]), with the same gp words (level10 gp−0x53b0..−0x5394).
const BUBBLE_POINTS: u32 = 0x1d_3290;
const BUBBLE_NORMALS: u32 = 0x1d_3d30;
const BUBBLE_ST: u32 = 0x1d_38c0;
const BUBBLE_QUADS: u32 = 0x1d_3620;
const BUBBLE_CENTRES: u32 = 0x1d_40c0;
const BUBBLE_FACES: u32 = 0x1d_3a90;
/// Each level's offset of the mesh tables from level 6's: Orxon's bubbles round its 31 gadgetbots.
const BUBBLE_MESH_SHIFT: [(u32, u32); 2] = [(REFERENCE_LEVEL, 0), (ORXON_LEVEL, 0x300)];
const BUBBLE_N: usize = 0x39;
const BUBBLE_NQ: usize = 0x2a;
/// gp−0x537c the texture, −0x5378 the colour, −0x5374 the length the reflected normals are scaled to; the blend is
/// ALPHA (0, 1, 2, 1) with FIX 0x20 (gp−0x5390..−0x5380), drawn as an alpha blend at 0x20 [L].
const BUBBLE_FX: usize = 0x29;
const BUBBLE_RGBA: u32 = 0x2080_8080;
const BUBBLE_NORMAL_LEN: f32 = 0.1;
/// The bubble's pvars (the port's): the camera position the update saw, for its draw.
const BUBBLE_CAM: usize = 0x00;

/// The bubble's mesh (level06 data): 57 points (w 1) and their normals (w 1: the draw moves them with the bubble
/// too), the ST of the far view, 42 quads of (point, ST) index pairs, and each quad's centre and normal for the cull.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bubble {
    pub points: Vec<[f32; 4]>,
    pub normals: Vec<[f32; 4]>,
    pub st: Vec<[f32; 2]>,
    pub quads: Vec<[(u16, u16); 4]>,
    pub centres: Vec<[f32; 4]>,
    pub faces: Vec<[f32; 4]>,
}

impl Bubble {
    /// Whether `level` has the bubbles (levels 6 and 10).
    pub fn on_level(level: u32) -> bool { BUBBLE_MESH_SHIFT.iter().any(|s| s.0 == level) }

    /// Reads the mesh from the level's overlay (None on a level without the bubbles or when a table does not read).
    pub fn parse(ov: &rc_formats::water::Overlay, level: u32) -> Option<std::sync::Arc<Bubble>> {
        let shift = BUBBLE_MESH_SHIFT.iter().find(|s| s.0 == level)?.1;
        let vec4 = |a: u32| -> Option<[f32; 4]> { let a = a - shift; Some([ov.f32(a).ok()?, ov.f32(a + 4).ok()?, ov.f32(a + 8).ok()?, ov.f32(a + 12).ok()?]) };
        let table = |a: u32, n: usize| (0..n as u32).map(|i| vec4(a + 16 * i)).collect::<Option<Vec<_>>>();
        let b = ov.read(BUBBLE_QUADS - shift, 16 * BUBBLE_NQ).ok()?;
        let h = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let quads: Vec<[(u16, u16); 4]> = (0..BUBBLE_NQ).map(|q| std::array::from_fn(|k| (h(16 * q + 4 * k), h(16 * q + 4 * k + 2)))).collect();
        let ns = quads.iter().flatten().map(|c| c.1 as u32 + 1).max()?;
        let st = (0..ns).map(|i| Some([ov.f32(BUBBLE_ST - shift + 8 * i).ok()?, ov.f32(BUBBLE_ST - shift + 8 * i + 4).ok()?])).collect::<Option<Vec<_>>>()?;
        let bubble = Bubble {
            points: table(BUBBLE_POINTS, BUBBLE_N)?,
            normals: table(BUBBLE_NORMALS, BUBBLE_N)?,
            st,
            quads,
            centres: table(BUBBLE_CENTRES, BUBBLE_NQ)?,
            faces: table(BUBBLE_FACES, BUBBLE_NQ)?,
        };
        if bubble.quads.iter().flatten().any(|c| c.0 as usize >= BUBBLE_N) { return None; }
        Some(std::sync::Arc::new(bubble))
    }
}

/// The dormant pulse (302 / 303): `t = (counter % 150) / 150 · 2π` (− 2π past π), `(sin t + 1) / 2`.
fn pulse(counter: u64) -> f32 {
    let mut t = (counter % 150) as f32 / 150.0;
    t = (t + t) * std::f32::consts::PI;
    if std::f32::consts::PI < t { t -= std::f32::consts::TAU; }
    (t.sin() + 1.0) * 0.5
}

/// Level06 `0x2d85a0`: the bubble 302 round a dormant bot. Its hit slot (mask 0x10000) is read and cleared every tick.
/// 0 → 1. 1: the glow pulses (`FastTweenColor(pulse, 0x80464646, 0x80828282)`); a hit shatters it (`0x2d8178`);
/// drawn → the draw `0x2d8738` (`RegisterDrawCallback2`). 2: the byte timer +0xbc out → deleted.
pub fn bubble_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x10 { w.mm(id).pvars.resize(0x10, 0); }
    let hit = w.get_hit(id, 0x1_0000, false);
    w.mm(id).hit_slot = 0xff;
    match w.m(id).state {
        0 => set_state(w, id, 1),
        1 => {
            let f = pulse(w.counter);
            w.mm(id).glow = crate::particles::tween_color(f.to_bits(), 0x8046_4646, 0x8082_8282);
            if let Some(h) = hit { shatter(w, id, h.dir.map(|x| f32::from_bits(x.0))); }
            if w.m(id).visible != 0 {
                let cam = w.camera_point();
                c::set_pv4(w, id, BUBBLE_CAM, [cam[0], cam[1], cam[2], 1.0]);
                if let Some(r) = super::row(REFERENCE_LEVEL, BUBBLE_DRAW_FN) { w.svc.draw_callbacks.register2(crate::moby_update::classes::draw_callbacks::Callback::UnitQuads(r), id); }
            }
        }
        2 => {
            let mut t = w.m(id).cmd;
            let done = crate::moby_update::services::fast_dec_timer_u8(&mut t) != 0;
            w.mm(id).cmd = t;
            if done { w.delete_moby(id); }
        }
        _ => {}
    }
}

/// `0x2d8178(m, hit)`: sound 0; six type-13 puffs 0.3 above it (0.3, 1.01, 1.075 then five at 1.07, 0.05, 50000, 1,
/// 0x40808080); ten glass shards (`0x300c60`, Blarg's window's) from `randf_sym(0, 0.2)` round it and
/// `randf(0.15, 0.55)` up, flying `randf(2, 4.5)·dt` at `rand_angle` plus `randf(0.7, 1.5)·dt` along the hit,
/// `randf(2, 7)·dt` up, their scale ×0.3 for the tick they are made; collision off, 2, hidden, timer `ticks(15)`.
/// (The game's other branch, no hit: away from Ratchet; this class only calls it with one.)
fn shatter(w: &mut World, id: MobyId, dir: V) {
    w.play_sound(0, 0, id);
    let pos = c::pos(w, id);
    let at = [pos[0], pos[1], pos[2] + 0.3, pos[3]];
    for hi in [0x3f89_999a, 0x3f88_f5c3, 0x3f88_f5c3, 0x3f88_f5c3, 0x3f88_f5c3, 0x3f88_f5c3] {
        w.part13(Pf::b(0x3e99_999a), Pf::b(0x3f81_47ae), Pf::b(hi), Pf::b(0x3d4c_cccd), Pf::b(0x4743_5000), v4(at), 1, 0x4080_8080);
    }
    let along = c::atan(dir[0], dir[1]);
    for _ in 0..10 {
        let x = w.rng.randf_sym(0.0, 0.2);
        let y = w.rng.randf_sym(0.0, 0.2);
        let z = w.rng.randf(f32::from_bits(0x3e19_999a), f32::from_bits(0x3f0c_cccd));
        let p0 = c::pos(w, id);
        let q = [p0[0] + x, p0[1] + y, p0[2] + z, p0[3]];
        let a = w.rng.rand_angle();
        let s = w.rng.randf(2.0, 4.5) * c::DT;
        let f = w.rng.randf(c::DT * 0.7, c::DT * 1.5);
        let up = w.rng.randf(2.0, 7.0) * c::DT;
        let v = [a.cos() * s + along.cos() * f, a.sin() * s + along.sin() * f, up, 0.0];
        super::blarg_glass::shard_from(w, q, v, 0.3);
    }
    let t = w.ticks(15);
    let m = w.mm(id);
    m.has_collision = false;
    m.state = 2;
    m.mode |= mode::HIDDEN;
    m.cmd = t as u8;
}

/// Level06 `0x2d8738` for the bubble `id`: its mesh moved with it (rows, position); the quads facing the camera (each
/// centre's direction from the camera against the turned face normal ≤ 0). Within 16 of the camera (x and y) the ST
/// are a sphere map of the view reflected in each point's normal (moved with it and scaled to 0.1: `r = e − 2(n·e)n`,
/// normalised, `st = r.xy / (2√(2(r.z + 1))) + 0.5`), else the mesh's own.
pub fn bubble_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Option<FxQuads> {
    let mesh = svc.units.blarg_bubble.as_ref()?;
    let m = table.mobys.get(id)?;
    if m.pvars.len() < 0x10 { return None; }
    let cam = p::v4f(&m.pvars, BUBBLE_CAM);
    let (r, pos) = (m.rows, m.position);
    let place = |q: [f32; 4]| -> V { std::array::from_fn(|k| if k == 3 { 1.0 } else { r[0][k] * q[0] + r[1][k] * q[1] + r[2][k] * q[2] + pos[k] * q[3] }) };
    let turn = |q: [f32; 4]| -> V { std::array::from_fn(|k| if k == 3 { 0.0 } else { r[0][k] * q[0] + r[1][k] * q[1] + r[2][k] * q[2] }) };
    let near = (cam[0] - pos[0]).abs() < 16.0 && (cam[1] - pos[1]).abs() < 16.0;
    let world: Vec<V> = mesh.points.iter().map(|&q| place(q)).collect();
    let env: Vec<[f32; 2]> = if near {
        world.iter().zip(&mesh.normals).map(|(&wp, &n)| {
            let e = c::set_len3(c::sub(wp, cam), 1.0);
            let n = c::set_len3(place(n), BUBBLE_NORMAL_LEN);
            let d = c::dot3(n, e);
            let rf = c::set_len3(c::sub(e, c::scale(n, d + d)), 1.0);
            let k = ((rf[2] + 1.0) * 2.0).sqrt();
            [rf[0] / (k + k) + 0.5, rf[1] / (k + k) + 0.5]
        }).collect()
    } else {
        Vec::new()
    };
    let mut quads = Vec::new();
    for (qi, q) in mesh.quads.iter().enumerate() {
        let e = c::set_len3(c::sub(place(mesh.centres[qi]), cam), 1.0);
        if 0.0 < c::dot3(e, turn(mesh.faces[qi])) { continue; }
        let corners = q.map(|(v, _)| { let p = world[v as usize]; [p[0], p[1], p[2]] });
        let st = q.map(|(v, s)| if near { env[v as usize] } else { mesh.st.get(s as usize).copied().unwrap_or_default() });
        quads.push(FxQuad { corners, st, rgba: [BUBBLE_RGBA; 4] });
    }
    Some(FxQuads { fx: BUBBLE_FX, additive: false, subtract: false, quads })
}

/// Level06 `0x2d8c20`: the marker 303 under a bot pulses with it while it sleeps (its owner +0xb8 in state < 2:
/// `FastTweenColor(pulse, 0x80464646, 0x80828282)`), else its glow eases 0.1 toward 0x80464646.
pub fn marker_update(w: &mut World, id: MobyId) {
    let dormant = w.m(id).parent.filter(|&o| o < w.table.mobys.len()).is_some_and(|o| w.m(o).state < 2);
    let g = if dormant {
        crate::particles::tween_color(pulse(w.counter).to_bits(), 0x8046_4646, 0x8082_8282)
    } else {
        crate::particles::tween_color(0.1f32.to_bits(), w.m(id).glow, 0x8046_4646)
    };
    w.mm(id).glow = g;
}
