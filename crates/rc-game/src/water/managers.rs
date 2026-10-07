//! The water classes: the ripple managers of every level that has the ripple module, and level 05's water plane.
//! They are class updates (`ClassUpdate::Water(i)`, the index into [`PORTS`]) found per level by code identity
//! (`LevelPorts`), each ported once from the level that has it (addresses are that level's):
//!
//! | port | classes (level) | reference code | what |
//! |---|---|---|---|
//! | 0 | 751 (01) | level01 `0x2fd0e8` | one manager, 21 patches in 7 camera zones ([`super::RippleSim::tick_with`]) |
//! | 1 | 831 (05) | level05 `0x30ca80` | one moby per patch (48): water level = moby z; raise / lower on group commands; the sewer flood |
//! | 2 | 982 (05) | level05 `0x318a68` | the flat water plane `0x1612dc..ec` (and no diving) while Ratchet is in its cuboid |
//! | 3 | 902, 941 … 971 (07) | level07 `0x30c508` | one moby per patch (36): raise / lower on group commands (the Hydrodisplacer pools) |
//! | 4 | 1158 (11) | level11 `0x30e088` | one moby per patch (20, built from the mobys): the tide between 132.36 and 137 |
//! | 5 | 19 (12) | level12 `0x2bf140` | one moby per patch (17, built from the mobys): rises by 5 on a command; sets the hero's water level near it |
//! | 6 | 1263, 1393 (13) | level13 `0x309e88` | one moby per patch (22): the level swings between two heights (cos) |
//!
//! **The patch managers** (ports 1, 3–6) are copies of one template: every instance owns patch `idx` of the level's
//! table; its update writes its z into the patch (+0x08, the water level) and the patch's sub-block mask (+0x1e) from a
//! view test (`FUN_00275690(48 or 64, moby)` = `FastBSphereCheck` of the moby's bounding sphere: the patch is simulated
//! and drawn only while its moby is in view), and adds a random drop (`randi(200) == 0`, radius 1, additive) to its
//! patch. The instance of patch 0 does the module's init (`RipplePatchesInit`, the constants, the light, the bounds,
//! the neighbour links, and on 07 / 13 two drops per patch), and every tick the clock `RippleSimClock` and the draw
//! callback registration. Their moby's collision (class +0x10: surface-0 faces at the moby's z) is the water Ratchet
//! swims in, so the level moves with the moby.
//!
//! **Additions 2026-10-01 (G-REN-008)**:
//!
//! | address | what | port |
//! |---|---|---|
//! | 751 `0x2fd0e8` | the tick's phases in the game's order: zones + random drops, the clock, `RegisterDrawCallback(0x2fd0c0)`, the drip (zone 0 / 6: timer, `randi(5)` point of 0x1fa650, x / y + `randf(±0.15)`, `0x2ffcd0(0.2, point)`, `rand_range(300, 0x4b0)`), zone 5's foam | `update_751` (`RippleSim::tick_zones` / `drip_due` / `tick_mist`, `units::drip::spawn`) |
//! | level13 `0x309e88` head | the vehicle 0x140940 ≠ 0 with class (+0xa6) 0x45, its state not 0xfe / 0xfd, and Ratchet in state 0x32 → return (nothing this tick, not even the patch tail) | `update_gemlik` (`Services::vehicle`, written by the ship 69's mount) |
//! | 831 state 3 | the flood button: Ratchet standing (`air_ticks` 0) on a class-0x33e moby (830, `FloorSwitch`) within 10 | `update_rilgar` (ported 2026-09-28; the button is the floor switch port) |
//! | raise / lower | the managers take the group commands 8 / 4 (`scheduler::group_cmd` / `group_state`); the senders: the Hydrodisplacer's use (`crate::hero::hydrodisplacer`: the pad's linked manager +0xbc = 8 / 4, 2026-10-01), Umbris's timed switches 886 (`units::timed_switch`) | receivers here |
//!
//! **Arithmetic.** Native `f32` for the manager logic (spring, heights, distances); the ripple module keeps its
//! existing PS2-float port ([`super::RippleSim`], one of the diagnosis layers kept as is).

use super::world::WaterWorld;
use super::RippleSim;
use crate::moby_runtime::MobyId;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::turn::spring;
use crate::moby_update::scheduler::{group_cmd, group_ids as group, group_state};
use crate::moby_update::services::{pvar, World};
use crate::ps2v::Pf;

/// One water class port.
#[derive(Clone, Copy, Debug)]
pub struct Port {
    pub name: &'static str,
    /// The level whose overlay holds [`Port::func`].
    pub level: u32,
    /// The update function (the reference level's address).
    pub func: u32,
    /// The classes the reference level's class table runs it for.
    pub classes: &'static [i16],
    /// The patch table (label, count) the manager hands `RipplePatchesInit`.
    pub patches: Option<(u32, usize)>,
    /// The per-patch sub-block masks (label) and their stride.
    pub masks: Option<u32>,
    pub mask_stride: u32,
    /// Other data read (label, words): `.lit` globals and tables.
    pub words: &'static [(u32, usize)],
}

pub const RIPPLE_751: usize = 0;
pub const RILGAR: usize = 1;
pub const PLANE: usize = 2;
pub const UMBRIS: usize = 3;
pub const POKITARU: usize = 4;
pub const HOVEN: usize = 5;
pub const GEMLIK: usize = 6;

/// Level 05 (831) labels.
mod l05 {
    pub const PATCHES: u32 = 0x1d_6880;
    pub const MASKS: u32 = 0x20_b380;
    /// Flood stage durations (ticks) and rise speeds (u/s) as used, and their unscaled originals.
    pub const TIMERS: u32 = 0x20_b3e0;
    pub const SPEEDS: u32 = 0x20_b3f8;
    pub const TIMERS0: u32 = 0x20_b410;
    pub const SPEEDS0: u32 = 0x20_b428;
    /// `$gp` globals: the mask override (patch, mask), the drown count, the flood stage, the stage-5 height, the level,
    /// the reached flag, the spring acceleration / speed factors.
    pub const G_OVR: u32 = 0x16_1d10;
    pub const G_OVR_MASK: u32 = 0x16_1d14;
    pub const G_DROWN: u32 = 0x16_1d1c;
    pub const G_STAGE: u32 = 0x16_1d20;
    pub const G_THRESH: u32 = 0x16_1d24;
    pub const G_LEVEL: u32 = 0x16_1d28;
    pub const G_REACHED: u32 = 0x16_1d2c;
    pub const G_SPRING_A: u32 = 0x16_1d30;
    pub const G_SPRING_V: u32 = 0x16_1d34;
}

/// Level 07 labels.
mod l07 {
    pub const PATCHES: u32 = 0x1d_ce40;
    pub const MASKS: u32 = 0x20_4680;
    pub const G_OVR: u32 = 0x16_1b30;
    pub const G_OVR_MASK: u32 = 0x16_1b34;
    /// Clears two owned-item words when set (0 on the disc, no writer in the level's code).
    pub const G_FLAG: u32 = 0x16_1b38;
}

/// Level 11 / 12 / 13 labels.
mod l11 {
    pub const PATCHES: u32 = 0x1d_a880;
    pub const G_COUNT: u32 = 0x16_1f64;
}
mod l12 {
    pub const PATCHES: u32 = 0x1c_bf40;
    pub const G_COUNT: u32 = 0x16_1388;
}
mod l13 {
    pub const PATCHES: u32 = 0x1d_9bc0;
    pub const MASKS: u32 = 0x1f_1e20;
    pub const G_OVR: u32 = 0x16_1ef0;
    pub const G_OVR_MASK: u32 = 0x16_1ef4;
}

pub const PORTS: [Port; 7] = [
    Port { name: "751 ripple zones", level: 1, func: 0x2f_d0e8, classes: &[751], patches: None, masks: None, mask_stride: 4, words: &[] },
    Port {
        name: "831 flood water",
        level: 5,
        func: 0x30_ca80,
        classes: &[831],
        patches: Some((l05::PATCHES, 48)),
        masks: Some(l05::MASKS),
        mask_stride: 2,
        words: &[
            (l05::TIMERS, 6),
            (l05::SPEEDS, 6),
            (l05::TIMERS0, 6),
            (l05::SPEEDS0, 6),
            (l05::G_OVR, 1),
            (l05::G_OVR_MASK, 1),
            (l05::G_DROWN, 1),
            (l05::G_STAGE, 1),
            (l05::G_THRESH, 1),
            (l05::G_LEVEL, 1),
            (l05::G_REACHED, 1),
            (l05::G_SPRING_A, 1),
            (l05::G_SPRING_V, 1),
        ],
    },
    Port { name: "982 water plane", level: 5, func: 0x31_8a68, classes: &[982], patches: None, masks: None, mask_stride: 2, words: &[] },
    Port {
        name: "902 pools",
        level: 7,
        func: 0x30_c508,
        classes: &[902, 941, 943, 944, 948, 949, 950, 951, 952, 953, 954, 955, 958, 961, 962, 967, 968, 969, 970, 971],
        patches: Some((l07::PATCHES, 36)),
        masks: Some(l07::MASKS),
        mask_stride: 2,
        words: &[(l07::G_OVR, 1), (l07::G_OVR_MASK, 1), (l07::G_FLAG, 1)],
    },
    Port { name: "1158 tide", level: 11, func: 0x30_e088, classes: &[1158], patches: Some((l11::PATCHES, 20)), masks: None, mask_stride: 2, words: &[(l11::G_COUNT, 1)] },
    Port { name: "19 rising water", level: 12, func: 0x2b_f140, classes: &[19], patches: Some((l12::PATCHES, 17)), masks: None, mask_stride: 2, words: &[(l12::G_COUNT, 1)] },
    Port {
        name: "1263 swinging water",
        level: 13,
        func: 0x30_9e88,
        classes: &[1263, 1393],
        patches: Some((l13::PATCHES, 22)),
        masks: Some(l13::MASKS),
        mask_stride: 2,
        words: &[(l13::G_OVR, 1), (l13::G_OVR_MASK, 1)],
    },
];

/// Port indices as the `ClassUpdate::Water` payload.
pub fn ids() -> impl Iterator<Item = u8> { 0..PORTS.len() as u8 }

/// `(*moby+0x74)(moby)` of water port `port`. Without the level's water data (`Services::water` not loaded: unit
/// tests, `RC_WATER`-less set-ups) nothing runs, no state changes and nothing is drawn from the stream.
pub fn update(w: &mut World, id: MobyId, port: u8) {
    if w.svc.water.data.is_none() { return; }
    match port as usize {
        RIPPLE_751 => update_751(w, id),
        RILGAR => update_rilgar(w, id),
        PLANE => update_plane(w, id),
        UMBRIS => update_umbris(w, id),
        POKITARU => update_pokitaru(w, id),
        HOVEN => update_hoven(w, id),
        GEMLIK => update_gemlik(w, id),
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------------
// Shared pieces

const DT: f32 = 1.0 / 60.0;
const DT2: f32 = 1.0 / 3600.0;

fn water<'a>(w: &'a mut World) -> &'a mut WaterWorld { &mut w.svc.water }

/// `FUN_00275690(far, m)`: `FastBSphereCheck(far, bsphere · 1/1024) ≠ −1`. No view (the load pass, before the first
/// render) is out of view.
fn in_view(w: &World, id: MobyId, far: f32) -> bool {
    let m = w.m(id);
    w.view.is_some_and(|v| !v.culled(far, m.bsphere.map(|x| x * (1.0 / 1024.0))))
}

/// `Approach(target, step, &x)` 0x270728.
fn approach(x: &mut f32, target: f32, step: f32) {
    let d = target - *x;
    *x += d.clamp(-step, step);
}

fn pi(w: &World, id: MobyId, o: usize) -> i32 { pvar::i32(&w.m(id).pvars, o) }
fn pf(w: &World, id: MobyId, o: usize) -> f32 { pvar::ff(&w.m(id).pvars, o) }
fn set_pi(w: &mut World, id: MobyId, o: usize, x: i32) { pvar::set_i32(&mut w.mm(id).pvars, o, x) }
fn set_pf(w: &mut World, id: MobyId, o: usize, x: f32) { pvar::set_ff(&mut w.mm(id).pvars, o, x) }
fn z(w: &World, id: MobyId) -> f32 { w.m(id).position[2] }
fn set_z(w: &mut World, id: MobyId, v: f32) { w.mm(id).position[2] = v; }

/// The patch record `idx` of the module (None: no module yet, or out of range).
fn patch<'a>(w: &'a mut World, idx: i32) -> Option<&'a mut super::Patch> {
    let i = usize::try_from(idx).ok()?;
    water(w).sim.as_mut()?.patches.get_mut(i)
}

/// The patch record `idx` before the module's init (the stored table, which the patch managers' mobys already write:
/// the init keeps +0x08 and +0x1e).
fn record<'a>(w: &'a mut World, idx: i32) -> Option<&'a mut rc_formats::water::PatchRecord> {
    let i = usize::try_from(idx).ok()?;
    water(w).records.get_mut(i)
}

/// Record +0x08 (the water level) of patch `idx`.
fn set_z_at(w: &mut World, idx: i32, z: f32) {
    if let Some(p) = patch(w, idx) { p.set_z(z) } else if let Some(r) = record(w, idx) { r.centre[2] = z }
}

/// Record +0x1e (the active sub-blocks) of patch `idx`.
fn set_mask(w: &mut World, idx: i32, m: u16) {
    if let Some(p) = patch(w, idx) { p.mask = m } else if let Some(r) = record(w, idx) { r.mask = m }
}

/// Record +0x1c / +0x1d (FIX env, water) of patch `idx`.
fn set_fix(w: &mut World, idx: i32, fix: [u8; 2]) {
    if let Some(p) = patch(w, idx) {
        (p.rec.fix_env, p.rec.fix_water) = (fix[0], fix[1]);
    } else if let Some(r) = record(w, idx) {
        (r.fix_env, r.fix_water) = (fix[0], fix[1]);
    }
}

/// A manager's global (its stored value until written), as u32 / i32 / f32.
fn g(w: &World, port: usize, label: u32) -> u32 { w.svc.water.global(port, label) }
fn gi(w: &World, port: usize, label: u32) -> i32 { g(w, port, label) as i32 }
fn gf(w: &World, port: usize, label: u32) -> f32 { f32::from_bits(g(w, port, label)) }
fn set_g(w: &mut World, label: u32, v: u32) { water(w).set_global(label, v) }

/// The stored sub-block mask of patch `idx` (after an override: [`mask_override`]).
fn mask_of(w: &World, port: usize, idx: i32) -> u16 {
    let Some(pm) = PORTS[port].masks else { return 0xffff };
    let label = pm + PORTS[port].mask_stride * idx.max(0) as u32;
    if let Some(&v) = w.svc.water.globals.get(&label) { return v as u16; }
    w.svc.water.data.as_ref().and_then(|d| d.port(port)).and_then(|p| p.masks.get(idx.max(0) as usize)).copied().unwrap_or(0)
}

/// The tail's override `if idx == G_OVR: patch mask = masks[idx] = G_OVR_MASK` (the `.lit` pair; never written by the
/// level's code).
fn mask_override(w: &mut World, port: usize, idx: i32, ovr: u32, ovr_mask: u32) {
    if gi(w, port, ovr) != idx { return; }
    let m = g(w, port, ovr_mask) as u16;
    set_mask(w, idx, m);
    if let Some(pm) = PORTS[port].masks { set_g(w, pm + PORTS[port].mask_stride * idx.max(0) as u32, m as u32); }
}

/// The random drop of a patch manager's tick: `randi(odds) == 0` → `RippleDisturb(x ± randf(−4, 4), y ± randf(−4, 4),
/// 1, amp, &patch[idx], 1, additive)` (x drawn first).
fn random_drop(w: &mut World, idx: i32, odds: i32, amp: f32) {
    if w.rng.randi(odds) != 0 { return; }
    let rx = Pf(w.rng.randf_bits(0xc080_0000, 0x4080_0000));
    let ry = Pf(w.rng.randf_bits(0xc080_0000, 0x4080_0000));
    let Ok(i) = usize::try_from(idx) else { return };
    let Some(sim) = water(w).sim.as_mut() else { return };
    let Some(p) = sim.patches.get(i) else { return };
    let (px, py) = (p.centre[0], p.centre[1]);
    sim.disturb(px + rx, py + ry, Pf::ONE, Pf::f(amp), i..i + 1, true);
}

/// The clock and the draw callback of the patch-0 instance: `RippleSimClock(table, n)`, `RegisterDrawCallback`.
fn clock_and_register(w: &mut World, id: MobyId) {
    if let Some(sim) = water(w).sim.as_mut() { sim.clock(); }
    w.svc.draw_callbacks.register(Callback::RipplePatches, id);
}

/// The shared part of the patch managers' inits: `RipplePatchesInit(table, n)` with the level's records and the
/// module constants, light and bounds ([`RippleSim::module_init`]), the module's FIX globals and the underwater fog
/// the init writes (far 32768, intensities 255 / 48, near 0; level 05 its own).
fn module_init(w: &mut World, port: usize, zero_masks: bool, fix: [u8; 2]) -> bool {
    let Some(data) = w.svc.water.data.clone() else { return false };
    let (Some(m), Some(_)) = (data.module.as_ref(), data.port(port)) else { return false };
    let mut records = w.svc.water.records.clone();
    if zero_masks { for r in &mut records { r.mask = 0; } }
    let sim = RippleSim::module_init(&records, m, data.light_xy);
    let ww = water(w);
    ww.sim = Some(sim);
    ww.module_fix = fix;
    ww.look.fog.far_dist = 32768.0;
    ww.look.fog.near_intensity = 255.0;
    ww.look.fog.far_intensity = 48.0;
    ww.look.fog.near_dist = 0.0;
    true
}

/// The patch link pass of levels 07 / 11 / 12 / 13 (`0x30cd48` on 07): links of patch `i` reset to 0xff, then every
/// other patch 16 units away along an edge (±0.1) or a diagonal becomes the matching neighbour.
fn link(w: &mut World, i: i32) {
    let Some(sim) = water(w).sim.as_mut() else { return };
    let Ok(i) = usize::try_from(i) else { return };
    if i >= sim.patches.len() { return; }
    sim.patches[i].rec.links = [0xff; 8];
    let c = |p: &super::Patch| [p.rec.centre[0], p.rec.centre[1]];
    let me = c(&sim.patches[i]);
    let n = sim.patches.len();
    let near = |v: f32| v.abs() < 0.1;
    for j in 0..n {
        if j == i { continue; }
        let o = c(&sim.patches[j]);
        let (dx, dy) = (me[0] - o[0], me[1] - o[1]);
        let l = &mut sim.patches[i].rec.links;
        let j8 = j as u8;
        if near(dx) {
            if near(dy - 16.0) { l[0] = j8; }
            if near(dy + 16.0) { l[2] = j8; }
        } else if near(dy) {
            if near(dx - 16.0) { l[1] = j8; }
            if near(dx + 16.0) { l[3] = j8; }
        } else if near(dx + 16.0) {
            if near(dy - 16.0) { l[5] = j8; }
            if near(dy + 16.0) { l[7] = j8; }
        } else if near(dx - 16.0) {
            if near(dy - 16.0) { l[4] = j8; }
            if near(dy + 16.0) { l[6] = j8; }
        }
    }
}

/// Level 05's link pass (`0x30d6a0`), an earlier version: no reset, and the diagonal branches test x twice, so every
/// patch 16 units along x (and off the row) becomes the +0x11 (x − 16) or +0x12 (x + 16) corner.
fn link_05(w: &mut World, i: i32) {
    let Some(sim) = water(w).sim.as_mut() else { return };
    let Ok(i) = usize::try_from(i) else { return };
    if i >= sim.patches.len() { return; }
    let me = [sim.patches[i].rec.centre[0], sim.patches[i].rec.centre[1]];
    let near = |v: f32| v.abs() < 0.1;
    for j in 0..sim.patches.len() {
        if j == i { continue; }
        let o = [sim.patches[j].rec.centre[0], sim.patches[j].rec.centre[1]];
        let (dx, dy) = (me[0] - o[0], me[1] - o[1]);
        let l = &mut sim.patches[i].rec.links;
        let j8 = j as u8;
        if near(dx) {
            if near(dy - 16.0) { l[0] = j8; }
            if near(dy + 16.0) { l[2] = j8; }
        } else if near(dy) {
            if near(dx - 16.0) { l[1] = j8; }
            if near(dx + 16.0) { l[3] = j8; }
        } else if near(dx - 16.0) {
            l[5] = j8;
        } else if near(dx + 16.0) {
            l[6] = j8;
        }
    }
}

/// Every group member of class `831` follows moby `id` (`0x30d5c0`): z = `id`'s z (keeping its offset from `id`'s
/// low level when not flooding and the lows differ), and its patch's level with it.
fn sync_05(w: &mut World, id: MobyId) {
    let (zm, low, flood) = (z(w, id), pf(w, id, 4), pi(w, id, 0xc));
    for o in group(w, w.m(id).group) {
        if w.table.mobys.get(o).is_none_or(|m| m.o_class != 0x33f) { continue; }
        let olow = pf(w, o, 4);
        let nz = if flood != 0 || olow == low { zm } else { olow + (zm - low) };
        set_z(w, o, nz);
        let idx = pi(w, o, 0x14);
        set_z_at(w, idx, nz);
    }
}

/// Every group member follows moby `id`'s z, and its patch (`idx` at pvar `+o`) with it (`0x30e690` on 11,
/// `0x2bf6d0` on 12).
fn sync_all(w: &mut World, id: MobyId, o: usize) {
    let zm = z(w, id);
    for m in group(w, w.m(id).group) {
        if w.table.mobys.get(m).is_none() { continue; }
        set_z(w, m, zm);
        let idx = pi(w, m, o);
        set_z_at(w, idx, zm);
    }
}

// ---------------------------------------------------------------------------------------------------
// 751 (level 01)

fn update_751(w: &mut World, id: MobyId) {
    let Some(data) = w.svc.water.data.clone() else { return };
    let Some((t, zones)) = data.z751.as_ref() else { return };
    if w.m(id).state == 0 {
        // State 0 (the load pass): the init, then state 1, update distance 0xff, z + 0.5.
        let sim = RippleSim::new(t, zones.clone(), data.light_xy, w.rng);
        let ww = water(w);
        ww.sim = Some(sim);
        (ww.look.fog.far_dist, ww.look.fog.near_intensity, ww.look.fog.far_intensity, ww.look.fog.near_dist) = (32768.0, 255.0, 48.0, 0.0);
        let m = w.mm(id);
        m.state = 1;
        m.update_dist = 0xff;
        m.position[2] += 0.5;
        return;
    }
    let cam = [w.camera[0].to_f32(), w.camera[1].to_f32(), w.camera[2].to_f32()];
    let counter = w.counter;
    let Some(sim) = w.svc.water.sim.as_mut() else { return };
    let mut info = sim.tick_zones(cam, &data.cuboids, w.rng);
    // The underwater colours of the highest active zone.
    w.svc.water.look.set_from_ripple_zone(usize::try_from(info.zone).ok());
    w.svc.draw_callbacks.register(Callback::RipplePatches, id);
    // The drip 787 (zone 0 or 6): its point, `0x2ffcd0(0.2, point)`, the timer re-armed.
    let due = w.svc.water.sim.as_mut().and_then(|s| s.drip_due(info.zone, w.rng));
    if let Some(at) = due {
        crate::moby_update::classes::units::drip::spawn(w, crate::moby_update::classes::units::drip::SIZE, at);
        let t = w.rng.rand_range(300, 0x4b0);
        if let Some(s) = w.svc.water.sim.as_mut() { s.drip_timer = t; }
        info.drips += 1;
    }
    if let Some(s) = w.svc.water.sim.as_mut() { s.tick_mist(&mut info, w.rng, counter, w.particles.as_deref_mut()); }
}

// ---------------------------------------------------------------------------------------------------
// 831 (level 05)

fn update_rilgar(w: &mut World, id: MobyId) {
    use l05::*;
    const P: usize = RILGAR;
    let idx = pi(w, id, 0x14);
    let state = w.m(id).state;
    match state {
        0 => {
            if idx == 0 {
                set_g(w, G_LEVEL, z(w, id).to_bits());
                set_g(w, G_STAGE, 0);
                set_g(w, G_REACHED, 0);
                if !module_init(w, P, false, [0x1c, 0x40]) { return; }
                let ww = water(w);
                (ww.look.fog.color, ww.look.fog.far_dist, ww.look.fog.far_intensity) = ([0, 0x0b, 0x3e], 20480.0, 26.0);
                let m0 = mask_of(w, P, 0);
                set_mask(w, 0, m0);
                let n = w.svc.water.sim.as_ref().map_or(0, |s| s.patches.len());
                for k in 0..n as i32 { link_05(w, k); }
            }
            w.mm(id).draw_dist = 0;
            w.mm(id).mode |= 1;
            if pi(w, id, 0xc) == 0 {
                if pi(w, id, 0x10) < 0 {
                    let zz = z(w, id);
                    set_pf(w, id, 4, zz);
                    set_pf(w, id, 0, zz + 4.5);
                    if pi(w, id, 8) != 0 {
                        w.mm(id).state = 1;
                        set_z(w, id, zz + 4.5);
                    } else {
                        w.mm(id).state = 2;
                        set_z(w, id, zz);
                    }
                } else {
                    w.mm(id).state = 2;
                    let lo = pf(w, id, 4);
                    set_z(w, id, lo);
                }
            } else {
                if pi(w, id, 0x10) >= 0 {
                    w.mm(id).update_dist = 0xff;
                    let drowns = gi(w, P, G_DROWN);
                    if drowns > 1 {
                        let f = ((drowns - 1) as f32 * 0.014_285_714 + 1.0).min(1.4);
                        for k in 0..6 {
                            let t0 = gi(w, P, TIMERS0 + 4 * k) as f32;
                            let s0 = gf(w, P, SPEEDS0 + 4 * k);
                            set_g(w, TIMERS + 4 * k, ((t0 * f) as i32) as u32);
                            set_g(w, SPEEDS + 4 * k, (s0 / f).to_bits());
                        }
                    }
                }
                let done = w.mission_done(w.svc.level, w.m(id).mission) == 0xff;
                if done {
                    w.mm(id).state = 7;
                    set_z(w, id, 44.0);
                } else {
                    w.mm(id).state = 3;
                    let t = w.ticks(60);
                    set_pi(w, id, 0x18, t);
                    if z(w, id) < pf(w, id, 4) { set_mask(w, idx, 0); }
                }
            }
            if idx == 47 { w.mm(id).has_collision = false; }
        }
        1 | 2 => {
            let (bit, next, target_off, reply) = if state == 1 { (8u8, 2u8, 0usize, 2u8) } else { (4, 1, 4, 1) };
            if w.m(id).cmd & bit == 0 {
                if state == 1 {
                    w.mm(id).cmd = reply;
                } else if pi(w, id, 0x10) >= 0 && w.mission_done(w.svc.level, w.m(id).mission) == 0xff {
                    // The flood's mission done: the water stays up.
                    w.mm(id).state = 1;
                    let hi = pf(w, id, 0);
                    set_z(w, id, hi);
                }
            } else {
                set_pi(w, id, 0x24, 1);
                let gr = w.m(id).group;
                group_state(w, gr, next);
            }
            if pi(w, id, 0x24) != 0 {
                let a = gf(w, P, G_SPRING_A) * DT2;
                let v = gf(w, P, G_SPRING_V) * DT;
                let target = pf(w, id, target_off);
                let (mut zz, mut vel) = (z(w, id), pf(w, id, 0x20));
                spring(target, a, a, v, &mut zz, &mut vel);
                set_z(w, id, zz);
                set_pf(w, id, 0x20, vel);
                sync_05(w, id);
            }
            if state == 2 { w.mm(id).cmd = reply; }
            random_drop(w, idx, 200, -0.05);
        }
        3 => {
            let hero = w.hero.position();
            let mp = w.m(id).position;
            let d = ((mp[0] - hero[0]).powi(2) + (mp[1] - hero[1]).powi(2)).sqrt();
            if d < 16.0 && mp[2] > hero[2] && w.hero.group == 0x11 {
                let hf = w.hero_fields();
                if hf.water_level < mp[2] { w.hero_fields_mut().water_level = mp[2]; }
            }
            let on_button = w.hero.ground_moby.and_then(|g| w.table.mobys.get(g)).is_some_and(|g| g.o_class == 0x33e);
            if pi(w, id, 0x10) >= 0 && d < 10.0 && on_button && w.hero.air_ticks == 0 {
                set_pi(w, id, 0x1c, 0);
                w.mm(id).state = 8;
                set_g(w, G_STAGE, 0);
                let t = w.ticks(gi(w, P, TIMERS));
                set_pi(w, id, 0x18, t);
            }
            random_drop(w, idx, 50, -0.01);
        }
        7 => {
            if pi(w, id, 0x10) < 0 {
                if pf(w, id, 0) < z(w, id) {
                    set_mask(w, idx, 0);
                    w.delete_moby(id);
                    return;
                }
            } else if w.hero.state == 0x6a && w.hero.timer == 2 {
                set_g(w, G_DROWN, (gi(w, P, G_DROWN) + 1) as u32);
            }
        }
        8 => {
            let mut t = pi(w, id, 0x18);
            if crate::moby_update::creature::dec_timer_i32(&mut t) != 0 && gi(w, P, G_STAGE) < 5 {
                let s = gi(w, P, G_STAGE) + 1;
                set_g(w, G_STAGE, s as u32);
                t = w.ticks(gi(w, P, TIMERS + 4 * s as u32));
            }
            set_pi(w, id, 0x18, t);
            let s = gi(w, P, G_STAGE).clamp(0, 5) as u32;
            let nz = z(w, id) + gf(w, P, SPEEDS + 4 * s) * DT;
            set_z(w, id, nz);
            sync_05(w, id);
            set_g(w, G_LEVEL, nz.to_bits());
            let hero = w.hero.position();
            let d = ((211.0 - hero[0]).powi(2) + (338.0 - hero[1]).powi(2) + (25.0 - hero[2]).powi(2)).sqrt();
            if d < 6.0 && gi(w, P, G_REACHED) == 0 {
                set_g(w, G_REACHED, 1);
                if gf(w, P, G_THRESH) < nz { set_g(w, G_STAGE, 5); }
            }
            if pi(w, id, 0x10) >= 0 && w.hero.state == 0x6a && w.hero.timer == 2 {
                set_g(w, G_DROWN, (gi(w, P, G_DROWN) + 1) as u32);
            }
            if nz >= 44.0 {
                let gr = w.m(id).group;
                group_state(w, gr, 7);
            }
        }
        _ => {}
    }
    // The tail.
    if idx < 0 { return; }
    let zz = z(w, id);
    set_z_at(w, idx, zz);
    let st = w.m(id).state;
    let active = in_view(w, id, 48.0) && (!(3..=7).contains(&st) || (pf(w, id, 4) < zz && zz < pf(w, id, 0)));
    let m = if active { mask_of(w, P, idx) } else { 0 };
    set_mask(w, idx, m);
    if idx == 0 { clock_and_register(w, id); }
    mask_override(w, P, idx, G_OVR, G_OVR_MASK);
}

// ---------------------------------------------------------------------------------------------------
// 982 (level 05): the flat water plane

fn update_plane(w: &mut World, id: MobyId) {
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.state = 1;
            m.update_dist = 0xff;
            m.mode |= 1;
            m.has_collision = false;
        }
        1 => {
            let Some(data) = w.svc.water.data.clone() else { return };
            let hero = w.hero.position();
            let inside = usize::try_from(pi(w, id, 0)).ok().and_then(|c| data.cuboids.get(c)).is_some_and(|c| super::cuboid_contains(c, hero));
            if !inside {
                water(w).plane.on = false;
                return;
            }
            // Water-current lock: no diving while in the cuboid (0x13f52e = 5).
            w.hero_fields_mut().dive_lock = 5;
            let t = (w.counter % 360) as f32;
            // The game's literal 3.14 (0x4048f5c3), not π.
            #[allow(clippy::approx_constant)]
            let z = (t * 0.017_444_445 - 3.14).sin() * 0.25 + 59.5;
            water(w).plane = super::world::WaterPlane { on: true, centre: hero, z, radius: 48.0 };
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------------
// 902 … (level 07)

fn update_umbris(w: &mut World, id: MobyId) {
    use l07::*;
    const P: usize = UMBRIS;
    if w.m(id).pvars.len() < 0x20 { return; }
    let idx = pi(w, id, 0x14);
    let state = w.m(id).state;
    match state {
        0 => {
            w.mm(id).mode |= 0x81;
            if idx == 0 {
                if !module_init(w, P, false, [0x40, 0x40]) { return; }
                let m0 = mask_of(w, P, 0);
                set_mask(w, 0, m0);
                if let Some(sim) = w.svc.water.sim.as_mut() { sim.init_drops(w.rng); }
                let n = w.svc.water.sim.as_ref().map_or(0, |s| s.patches.len());
                for k in 0..n as i32 { link(w, k); }
            }
            if pi(w, id, 0xc) == 0 {
                w.mm(id).state = if pi(w, id, 8) == 0 { 2 } else { 1 };
            } else {
                w.mm(id).state = 3;
                let t = w.ticks(60);
                set_pi(w, id, 0x18, t);
                if z(w, id) < pf(w, id, 4) { set_mask(w, idx, 0); }
            }
        }
        1 | 2 => {
            let (bit, next_state, reply, target_off) = if state == 1 { (8u8, 2u8, 2u8, 0usize) } else { (4, 1, 1, 4) };
            let gr = w.m(id).group;
            if w.m(id).cmd & bit == 0 {
                // `FUN_00281ef8`: whether this moby leads its group (the group list's first entry).
                let leads = group(w, gr).first() == Some(&id);
                let target = pf(w, id, target_off);
                if w.m(id).cmd == reply || leads {
                    let mut zz = z(w, id);
                    approach(&mut zz, target, DT * 4.0);
                    set_z(w, id, zz);
                    if leads && w.m(id).cmd != reply { group_cmd(w, gr, reply); }
                }
            } else {
                group_state(w, gr, next_state);
            }
            random_drop(w, idx, 200, -0.05);
        }
        3 => {
            let hero = w.hero.position();
            let cub = pi(w, id, 0x10);
            let inside = cub >= 0 && w.svc.water.data.as_ref().and_then(|d| d.cuboids.get(cub as usize)).is_some_and(|c| super::cuboid_contains(c, hero));
            if inside {
                set_pi(w, id, 0x1c, 0);
                let gr = w.m(id).group;
                group_state(w, gr, 4);
            }
        }
        4 => {
            let mut t = pi(w, id, 0x18);
            if crate::moby_update::creature::dec_timer_i32(&mut t) != 0 {
                t = w.ticks(0x708);
                w.mm(id).state = 5;
            }
            set_pi(w, id, 0x18, t);
        }
        5 => {
            let nz = z(w, id) + DT * 0.32;
            set_z(w, id, nz);
            let mut t = pi(w, id, 0x18);
            if crate::moby_update::creature::dec_timer_i32(&mut t) != 0 { w.mm(id).state = 6; }
            set_pi(w, id, 0x18, t);
        }
        6 => {
            let nz = z(w, id) + DT * 0.42;
            set_z(w, id, nz);
            if nz >= 54.0 { w.mm(id).state = 7; }
        }
        7 if pf(w, id, 0) < z(w, id) => {
            set_mask(w, idx, 0);
            w.delete_moby(id);
            return;
        }
        _ => {}
    }
    if idx < 0 { return; }
    let fix = w.svc.water.module_fix;
    let zz = z(w, id);
    set_fix(w, idx, fix);
    set_z_at(w, idx, zz);
    let st = w.m(id).state;
    let active = w.m(id).has_collision && in_view(w, id, 48.0) && (!(3..=7).contains(&st) || (pf(w, id, 4) < zz && zz < pf(w, id, 0)));
    let m = if active { mask_of(w, P, idx) } else { 0 };
    set_mask(w, idx, m);
    if idx == 0 { clock_and_register(w, id); }
    mask_override(w, P, idx, G_OVR, G_OVR_MASK);
}

// ---------------------------------------------------------------------------------------------------
// 1158 (level 11) and 19 (level 12): patches built from the mobys

/// State 0 of the 11 / 12 managers: the next patch index from the counter, the moby's scale doubled, and the patch
/// record from the moby (patch 0's instance runs the module init first, all masks cleared).
fn build_patch(w: &mut World, id: MobyId, port: usize, counter: u32, idx_off: usize, fx: [i32; 2]) -> i32 {
    let idx = gi(w, port, counter);
    set_g(w, counter, (idx + 1) as u32);
    set_pi(w, id, idx_off, idx);
    let oc = w.m(id).o_class;
    if let Some(s) = w.classes.info(oc).map(|c| c.scale) { w.mm(id).scale = s + s; }
    w.mm(id).update_dist = 0xff;
    w.mm(id).mode |= 1;
    if idx == 0 && !module_init(w, port, true, [0x1c, 0x40]) { return idx; }
    let pos = w.m(id).position;
    let consts = w.svc.water.sim.as_ref().map(|s| s.consts);
    if let (Some(p), Some(c)) = (patch(w, idx), consts) {
        p.centre = [Pf::f(pos[0]), Pf::f(pos[1]), Pf::f(pos[2])];
        p.rec.centre = [pos[0], pos[1], pos[2]];
        (p.rec.fx_env, p.rec.fx_water, p.rec.fix_env, p.rec.fix_water) = (fx[0], fx[1], 0x1c, 0x44);
        p.mask = 0xffff;
        p.set_bounds(&c);
    }
    idx
}

fn update_pokitaru(w: &mut World, id: MobyId) {
    use l11::*;
    const P: usize = POKITARU;
    let state = w.m(id).state;
    match state {
        0 => {
            build_patch(w, id, P, G_COUNT, 4, [0x38, 0x39]);
            w.mm(id).state = 1;
        }
        1 => {
            set_g(w, G_COUNT, 0);
            let idx = pi(w, id, 4);
            link(w, idx);
            w.mm(id).state = 2;
        }
        2 | 3 => {
            let (bit, next, reply, target) = if state == 2 { (8u8, 3u8, 2u8, 137.0f32) } else { (4, 2, 1, f32::from_bits(0x4304_5c29)) };
            if w.m(id).cmd & bit != 0 {
                set_pi(w, id, 8, 1);
                let gr = w.m(id).group;
                group_state(w, gr, next);
            }
            w.mm(id).cmd = reply;
            if pi(w, id, 8) != 0 {
                let (mut zz, mut vel) = (z(w, id), pf(w, id, 0));
                spring(target, DT2 * 4.0, DT2 * 4.0, DT * 4.0, &mut zz, &mut vel);
                set_z(w, id, zz);
                set_pf(w, id, 0, vel);
                sync_all(w, id, 4);
            }
            let idx = pi(w, id, 4);
            random_drop(w, idx, 200, -0.03);
        }
        _ => {}
    }
    let idx = pi(w, id, 4);
    if idx < 0 { return; }
    let m = if in_view(w, id, 64.0) { 0xffff } else { 0 };
    set_mask(w, idx, m);
    if idx == 0 { clock_and_register(w, id); }
}

fn update_hoven(w: &mut World, id: MobyId) {
    use l12::*;
    const P: usize = HOVEN;
    if w.m(id).pvars.len() < 0x1c { return; }
    let state = w.m(id).state;
    match state {
        0 => {
            set_pi(w, id, 0, 3);
            let fx = [pi(w, id, 0) + 0x28, pi(w, id, 4) + 0x28];
            build_patch(w, id, P, G_COUNT, 0xc, fx);
            w.mm(id).state = 1;
        }
        1 => {
            set_g(w, G_COUNT, 0);
            let idx = pi(w, id, 0xc);
            link(w, idx);
            w.mm(id).state = 2;
            let zz = z(w, id);
            set_pf(w, id, 0x10, zz);
        }
        2 => {
            if w.m(id).cmd & 4 != 0 {
                set_pi(w, id, 0x14, 1);
                let t = pf(w, id, 0x10) + 5.0;
                set_pf(w, id, 0x10, t);
            }
            w.mm(id).cmd = 1;
            if pi(w, id, 0x14) != 0 {
                let target = pf(w, id, 0x10);
                let (mut zz, mut vel) = (z(w, id), pf(w, id, 8));
                spring(target, DT2 * 4.0, DT2 * 4.0, DT * 4.0, &mut zz, &mut vel);
                set_z(w, id, zz);
                set_pf(w, id, 8, vel);
                sync_all(w, id, 0xc);
            }
            // Ratchet within 8 of the moby in x or in y: his water level is the moby's z.
            let hero = w.hero.position();
            let mp = w.m(id).position;
            if (hero[0] - mp[0]).abs() <= 8.0 || (hero[1] - mp[1]).abs() <= 8.0 { w.hero_fields_mut().water_level = mp[2]; }
            let idx = pi(w, id, 0xc);
            random_drop(w, idx, 200, -0.05);
        }
        _ => {}
    }
    let idx = pi(w, id, 0xc);
    if idx < 0 { return; }
    let mut active = in_view(w, id, 64.0);
    let cub = pi(w, id, 0x18);
    if cub != -1 {
        let cam = [w.camera[0].to_f32(), w.camera[1].to_f32(), w.camera[2].to_f32()];
        let inside = w.svc.water.data.as_ref().and_then(|d| d.cuboids.get(cub as usize)).is_some_and(|c| super::cuboid_contains(c, cam));
        if !inside { active = false; }
    }
    set_mask(w, idx, if active { 0xffff } else { 0 });
    if idx == 0 { clock_and_register(w, id); }
}

// ---------------------------------------------------------------------------------------------------
// 1263 / 1393 (level 13)

fn update_gemlik(w: &mut World, id: MobyId) {
    use l13::*;
    const P: usize = GEMLIK;
    // The ship pause: the vehicle 0x140940 is a class-0x45 moby, alive (state not 0xfe / 0xfd), and Ratchet flies it
    // (state 0x32) → nothing this tick.
    if let Some(v) = w.svc.vehicle.moby.filter(|&v| v < w.table.mobys.len()) {
        let m = w.m(v);
        if m.o_class == 0x45 && m.state != 0xfe && m.state != 0xfd && w.hero.state == 0x32 { return; }
    }
    if w.m(id).pvars.len() < 0x14 { return; }
    let idx = pi(w, id, 0x10);
    match w.m(id).state {
        0 => {
            w.mm(id).mode |= 1;
            let a = pf(w, id, 8) * 0.017_453_292;
            set_pf(w, id, 8, a);
            let r = pf(w, id, 0xc) * 0.017_453_292 * DT;
            set_pf(w, id, 0xc, r);
            if idx == 0 {
                if !module_init(w, P, false, [0x60, 0x40]) { return; }
                w.mm(id).update_dist = 0xff;
                let m0 = mask_of(w, P, 0);
                set_mask(w, 0, m0);
                if let Some(sim) = w.svc.water.sim.as_mut() { sim.init_drops(w.rng); }
                let n = w.svc.water.sim.as_ref().map_or(0, |s| s.patches.len());
                for k in 0..n as i32 { link(w, k); }
            }
            w.mm(id).draw_dist = 0;
            w.mm(id).state = 1;
        }
        1 => {
            let (hi, lo, a) = (pf(w, id, 0), pf(w, id, 4), pf(w, id, 8));
            let half = (hi - lo) * 0.5;
            set_z(w, id, lo + a.cos() * half + half);
            let (hg, hs) = (w.hero.group, w.hero.state);
            if !(matches!(hg, 0x14 | 0x19) || matches!(hs, 0x3d | 0x7b | 0x7c)) {
                let r = pf(w, id, 0xc);
                set_pf(w, id, 8, f32::from_bits(crate::particles::type06::fast_add_rotations(a.to_bits(), r.to_bits())));
            }
            let fix = w.svc.water.module_fix;
            set_fix(w, idx, fix);
            random_drop(w, idx, 200, -0.05);
        }
        _ => {}
    }
    if idx < 0 { return; }
    let zz = z(w, id);
    set_z_at(w, idx, zz);
    let m = if in_view(w, id, 48.0) { mask_of(w, P, idx) } else { 0 };
    set_mask(w, idx, m);
    if idx == 0 { clock_and_register(w, id); }
    mask_override(w, P, idx, G_OVR, G_OVR_MASK);
}
