//! Fog zones (environment transitions), the underwater test and the fog selection of `UpdateFog`.
//! Spec: docs/plan/world_animation.md §5 (zones) and §6 (underwater); level01 addresses.
//!
//! Per camera update (`0x20eca8`, after `UpdateAllCameras`): the underwater test `0x20e9f0`
//! ([`UnderwaterState::update`]), then the fog zones `fun_001ee4b0` (level copy 0x2102b8, [`update`]),
//! which writes the **level fog globals** 0x15f444..0x15f454 ([`FogGlobals`]) directly. At the end of every
//! frame render `UpdateFog` (0x218d70, boot 0x1f2588) copies either those globals or, while underwater, the
//! alternate set 0x161204..0x161214 into the view context and sets the particle far `0x16017c`
//! ([`update_fog`]); `UpdateViewContext` then recomputes the fog terms and `SetTfragDists` from them.
//! Nothing restores the globals when the camera leaves a zone.
//!
//! Arithmetic: the zone lookup `0x26bbc0` is VU0 macro code (`vsub`, `vmul`, `vadd`, the `vmula/vmadda`
//! matrix chain, `vclipw`), the lerp and the underwater test are EE FPU (`mul.s`, `add.s`, `sub.s`,
//! `cvt.w.s`, `c.lt.s`). Both run here on PS2 float bit patterns ([`crate::ps2v`]: round toward zero,
//! no denormals), in the game's operation order. The colour lerp is integer (`mult`, `sra 8`).

use crate::collision_query::{coll_line, QueryFlags};
use crate::ps2v::{add, mul, neg, sub, Pf, F, ONE};
use rc_formats::collision::Collision;
use rc_formats::gameplay::{FogZone, FogZones};

/// The level fog globals (0x15f444..0x15f454), the values `fun_001e9b10` (level init) copies from the
/// level settings and `fun_001ee4b0` overwrites inside a zone. Distances in integer units (game units
/// × 1024) along the view axis; intensities are the GS F value (255 = no fog).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogGlobals {
    /// 0x15f444..0x15f446: FOGCOL r, g, b.
    pub color: [u8; 3],
    /// 0x15f448 / 0x15f44c.
    pub near_dist: f32,
    pub far_dist: f32,
    /// 0x15f450 / 0x15f454.
    pub near_intensity: f32,
    pub far_intensity: f32,
}

/// The fog block `DoSpaceTransition` 0x2a68f8 writes into the view context (0x16d0d8..0x16d0f8) as it starts:
/// FOGCOL (0, 0, 0x10), near 0, far 524288 (512 units), intensities 255 → 128. The transition's own loops (the
/// cards, the movies, the flight) never run `UpdateFog`, so they draw with it until the next level's entry.
pub const TRANSITION_FOG: FogGlobals = FogGlobals { color: [0, 0, 0x10], near_dist: 0.0, far_dist: 524_288.0, near_intensity: 255.0, far_intensity: 128.0 };

/// The flight's fog: `EnterSpaceLoadingLoop` 0x2a5868 resets the view context (`InitViewContext` 0x219448: near 0,
/// far 524288, intensities 255 → 0; FOGCOL untouched, the transition's) and its loop never runs `UpdateFog`.
pub const FLIGHT_FOG: FogGlobals = FogGlobals { color: [0, 0, 0x10], near_dist: 0.0, far_dist: 524_288.0, near_intensity: 255.0, far_intensity: 0.0 };

/// `0x16017c` (gp−0x6a84): the global particle far in 20.12 fixed point: 500 units normally ...
pub const PARTICLE_FAR12: i32 = 0x1f_4000;
/// ... and 64 units while underwater (`UpdateFog`).
pub const UNDERWATER_PARTICLE_FAR12: i32 = 0x4_0000;

/// The alternate fog and the full-screen tint used while underwater (0x161200..0x161214).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnderwaterLook {
    /// 0x161200..0x161203: tint RGBAQ of the full-screen sprites `DrawDebugProfiler` draws with
    /// ALPHA_1 = 0x8000000044 (`Cd + (Cs − Cd)·A/128`).
    pub tint: [u8; 4],
    /// 0x161204..0x161214: the alternate fog `UpdateFog` uses while underwater.
    pub fog: FogGlobals,
}

impl Default for UnderwaterLook {
    /// The Novalis overlay's static bytes (0x161200 = 18 30 90 40 18 30 90 00, then 0, 32768.0, 255.0,
    /// 48.0); class 751's init writes the same distances/intensities. Other levels keep their own
    /// alternate set in their overlay (not ported: this default is used everywhere).
    fn default() -> Self {
        UnderwaterLook {
            tint: [0x18, 0x30, 0x90, 0x40],
            fog: FogGlobals { color: [0x18, 0x30, 0x90], near_dist: 0.0, far_dist: 32768.0, near_intensity: 255.0, far_intensity: 48.0 },
        }
    }
}

impl UnderwaterLook {
    /// Class 751's per-tick colour choice (0x2fd0e8, after the zone activation): `zone` = the highest
    /// active ripple zone. Zones 1–3: fog (0, 0x28, 0x30), tint (0, 0x28, 0x30, 0x40) = 50 %; any other
    /// active zone: fog (0x18, 0x30, 0x90), tint (0x18, 0x30, 0x90, 0x30) = 37.5 %. No active zone
    /// (`None`) leaves both unchanged.
    pub fn set_from_ripple_zone(&mut self, zone: Option<usize>) {
        match zone {
            Some(1..=3) => {
                self.fog.color = [0, 0x28, 0x30];
                self.tint = [0, 0x28, 0x30, 0x40];
            }
            Some(_) => {
                self.fog.color = [0x18, 0x30, 0x90];
                self.tint = [0x18, 0x30, 0x90, 0x30];
            }
            None => {}
        }
    }
}

/// `UpdateFog` (0x218d70): the fog the view context gets and the particle far, from the level globals
/// or, when the underwater flag 0x167494 is set, the alternate set.
pub fn update_fog(level: &FogGlobals, underwater: bool, look: &UnderwaterLook) -> (FogGlobals, i32) {
    if underwater { (look.fog, UNDERWATER_PARTICLE_FAR12) } else { (*level, PARTICLE_FAR12) }
}

const HALF: F = 0x3f00_0000;
const K255: F = 0x437f_0000;
const K1024: F = 0x4480_0000;

/// `0x26bbc0(p)`: the zone the point is in and the blend weight `t`.
///
/// The circles are walked in order; the first whose 2-D test `(dx² + dy²) − w` has the sign bit set (w
/// = r², z ignored) decides: `l = p·inv` (`vmulax/vmadday/vmaddaz/vmaddw` with p.w = 1) is clipped with
/// `vclipw.xyz l, vf0` (|w| = 1); any lane outside returns `None` **without trying later circles**.
/// Otherwise `t = 0.5·(l.x + 1)` (`vaddw.x`, `vmul.x`). The game's loop also tests the slot just past the
/// last circle (index `count`, stale memory that is zero on a fresh boot and then never hits: 0 − 0 is
/// +0); that slot is not modelled.
pub fn lookup(zones: &FogZones, pos: [f32; 3]) -> Option<(usize, f32)> {
    let p = pos.map(|v| Pf::f(v).0);
    for (i, c) in zones.circles.iter().enumerate() {
        let c = c.map(|v| Pf::f(v).0);
        let (dx, dy) = (sub(p[0], c[0]), sub(p[1], c[1]));
        let d2 = add(mul(dx, dx), mul(dy, dy)); // vmul.xyz, vaddy.x
        if !neg(sub(d2, c[3])) { continue; } // vsub.x vf15, vf13, vf14 (= 0 + c.w)
        let m = zones.zones.get(i)?.inverse.map(|r| r.map(|v| Pf::f(v).0));
        let lane = |k: usize| add(add(add(mul(m[0][k], p[0]), mul(m[1][k], p[1])), mul(m[2][k], p[2])), mul(m[3][k], ONE));
        let l = [lane(0), lane(1), lane(2)];
        // vclipw: +x if l > +1, −x if l < −1 (per lane).
        if l.iter().any(|&v| Pf(v) > Pf(ONE) || Pf(v) < Pf(ONE ^ 0x8000_0000)) { return None; }
        return Some((i, crate::ps2v::to_f32(mul(HALF, add(l[0], ONE)))));
    }
    None
}

/// `fun_001ee4b0`'s write for one zone and weight (called only when `zone.flags & 2`): EE FPU, in the
/// game's order. `i = cvt.w.s(t·255)`, colour byte `(c2·i + c1·(255 − i)) >> 8`; with `u = 1 − t`,
/// `near/far dist = (d2·t + d1·u)·1024`, `near/far F = 255 − (k2·t + k1·u)·255` (side 1 = t 0, side 2 = t 1).
pub fn apply(g: &mut FogGlobals, zone: &FogZone, t: f32) {
    let t = Pf::f(t);
    let k255 = Pf::b(K255);
    let i = (t * k255).to_i32();
    let u = Pf::ONE - t;
    let [s1, s2] = zone.side;
    let lerp = |a: f32, b: f32| Pf::f(b) * t + Pf::f(a) * u;
    g.far_intensity = (k255 - lerp(s1.far_density, s2.far_density) * k255).to_f32();
    g.near_intensity = (k255 - lerp(s1.near_density, s2.near_density) * k255).to_f32();
    g.far_dist = (lerp(s1.far_dist, s2.far_dist) * Pf::b(K1024)).to_f32();
    g.near_dist = (lerp(s1.near_dist, s2.near_dist) * Pf::b(K1024)).to_f32();
    let (c1, c2) = (zone.fog_color[0] as i32, zone.fog_color[1] as i32);
    for (k, out) in g.color.iter_mut().enumerate() {
        let (a, b) = ((c1 >> (8 * k)) & 0xff, (c2 >> (8 * k)) & 0xff);
        *out = ((b * i + a * (0xff - i)) >> 8) as u8;
    }
}

/// `fun_001ee4b0(camera)`: lookup, then [`apply`] when the zone has flags & 2. Returns the zone and t
/// that were hit (whatever the flags), for callers that also drive the hero cross-fade.
pub fn update(g: &mut FogGlobals, zones: &FogZones, camera: [f32; 3]) -> Option<(usize, f32)> {
    let (i, t) = lookup(zones, camera)?;
    let z = &zones.zones[i];
    if z.lerps_fog() { apply(g, z, t); }
    Some((i, t))
}

/// `0x26be04`'s zone part (flags & 1), for the hero moby only: the value for hero +0x38
/// (`light1 | light2 << 8 | i << 16`, the moby light-bank cross-fade) and hero +0x80 (colour). The
/// colour is the MMI sequence `pmulth` (per byte `c·w` as 16 bits), `paddub` (byte-wise saturating add of
/// the two products, no carry between bytes), `psrlh 8`: `min(255, (c1·(255 − i) >> 8) + (c2·i >> 8))`
/// per byte, all four bytes. Hook: nothing calls this until the hero is ported (world_animation.md §5).
pub fn hero_light(zone: &FogZone, t: f32) -> Option<(u32, [u8; 4])> {
    if !zone.lerps_hero_light() { return None; }
    let i = (Pf::f(t) * Pf::b(K255)).to_i32();
    let (c1, c2) = (zone.hero_color[0].to_le_bytes(), zone.hero_color[1].to_le_bytes());
    let colour = std::array::from_fn(|k| {
        let (p1, p2) = ((c1[k] as i32 * (0xff - i)) as u32 & 0xffff, (c2[k] as i32 * i) as u32 & 0xffff);
        ((p1 >> 8) + (p2 >> 8)).min(255) as u8
    });
    let word = (zone.hero_light[0] as u32 & 0xff) | (zone.hero_light[1] as u32 & 0xff) << 8 | (i as u32) << 16;
    Some((word, colour))
}

/// `0x26bcf4(pos, &d2)`: the static point light whose 2-D circle holds `pos` (the grid cell `(y >> 4)·64 + (x >> 4)`
/// of the section, its list walked in order; VU: `d = p − light`, `d2 = dx² + dy²`, inside when `d2 − light.w` is
/// negative): the light's index and `d2`. A cell without a list, an empty list or a position off the grid: none.
pub fn static_light(s: &rc_formats::gameplay::StaticLights, pos: [f32; 3]) -> Option<(usize, f32)> {
    let (cx, cy) = ((pos[0] as i32 as u32) >> 4, (pos[1] as i32 as u32) >> 4);
    let cell = *s.grid.get((cy.checked_mul(64)?.checked_add(cx)?) as usize)? as usize;
    if cell == 0 || !cell.is_multiple_of(4) { return None; }
    let n = *s.grid.get(cell / 4)? as usize;
    let p = pos.map(|v| Pf::f(v).0);
    for k in 0..n {
        let i = *s.grid.get(cell / 4 + 1 + k)? as usize;
        let l = s.lights.get(i)?;
        let c = l.pos.map(|v| Pf::f(v).0);
        let (dx, dy) = (sub(p[0], c[0]), sub(p[1], c[1]));
        let d2 = add(mul(dx, dx), mul(dy, dy));
        if neg(sub(d2, c[3])) { return Some((i, crate::ps2v::to_f32(d2))); }
    }
    None
}

/// `HeroEnvLighting` 0x26be04 on the hero moby at `pos`: a fog zone holding him with flags & 1 sets his colour +0x80
/// (`base`) and light word +0x38 ([`hero_light`]); then his ambient +0x3c = +0x80, plus (bytewise saturating, all four
/// bytes) the static light holding him ([`static_light`]) at `(255 − cvt(d2 / r2 · 255))`·colour >> 8.
pub fn hero_env_lighting(zones: Option<&FogZones>, lights: &rc_formats::gameplay::StaticLights, pos: [f32; 3], base: &mut [u8; 4], light: &mut u32, ambient: &mut [u8; 4]) {
    if let Some((i, t)) = zones.and_then(|z| lookup(z, pos)) {
        if let Some((word, colour)) = zones.and_then(|z| z.zones.get(i)).and_then(|z| hero_light(z, t)) {
            *base = colour;
            *light = word;
        }
    }
    let Some((i, d2)) = static_light(lights, pos) else {
        *ambient = *base;
        return;
    };
    let l = lights.lights[i];
    let k = ((Pf::f(d2) / Pf::f(l.pos[3])) * Pf::b(K255)).to_i32();
    let a = 0xff - k;
    let c = l.rgba.to_le_bytes();
    *ambient = std::array::from_fn(|j| {
        let add = ((c[j] as i32 * a) as u32 & 0xffff) >> 8;
        (base[j] as u32 + (add & 0xff)).min(255) as u8
    });
}

/// The underwater flag `0x167494` and its per-camera-update test `0x20e9f0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnderwaterState {
    pub flag: bool,
}

/// `0x3f400000`, `0x3c23d70a`, `0x3d23d70a`.
const K075: F = 0x3f40_0000;
const K001: F = 0x3c23_d70a;
const K004: F = 0x3d23_d70a;

impl UnderwaterState {
    /// `0x20e9f0` with the line query and the water height as callbacks.
    ///
    /// `cast(p0, p1)` = `CollLine_Fix(p0, p1, 0x12)` → (hit point, surface id `0x2151d8`). The segment runs
    /// from `cam.z + 0.75` to `cam.z − 0.75` (FPU). Up to 6 casts: a hit whose surface id is not 0 restarts
    /// from its hit point with `z − 0.01`, same end. On surface 0 (water): `h = water_height(hit)` (`0x26ed38`:
    /// the ripple-patch height when an active patch contains the point, else the flat plane when enabled,
    /// else the hit point's own z, which is what `None` gives here) and `flag = cam.z < h + 0.04`
    /// (`c.lt.s`). No hit, or six non-water hits, leaves the flag **unchanged**.
    ///
    /// The clears for camera mode 6 (`[0x167280]+0x86`) and the game mode `0x15f5c4 != 0`, and the other writers
    /// (`HeroTeleport` 0x2368e0, the surface jump `0x2406b0`, the Visibomb's end `0x2cb788`) are the caller's
    /// (rc-engine `fog_state`; `crate::water::world::UnderwaterStore`).
    pub fn update(
        &mut self,
        cam: [f32; 3],
        mut cast: impl FnMut([f32; 3], [f32; 3]) -> Option<([f32; 3], i32)>,
        mut water_height: impl FnMut([f32; 3]) -> Option<f32>,
    ) {
        let cz = Pf::f(cam[2]);
        let mut p0 = [cam[0], cam[1], (cz + Pf::b(K075)).to_f32()];
        let p1 = [cam[0], cam[1], (cz - Pf::b(K075)).to_f32()];
        for _ in 0..6 {
            let Some((hit, surface)) = cast(p0, p1) else { return };
            if surface == 0 {
                let h = water_height(hit).unwrap_or(hit[2]);
                self.flag = cz < Pf::f(h) + Pf::b(K004);
                return;
            }
            p0 = [hit[0], hit[1], (Pf::f(hit[2]) - Pf::b(K001)).to_f32()];
        }
    }

    /// [`Self::update`] against the level collision mesh (`CollLine_Fix`, flags 0x12 = two-sided | skip
    /// moby primitives).
    pub fn update_with_mesh(&mut self, mesh: &Collision, cam: [f32; 3], water_height: impl FnMut([f32; 3]) -> Option<f32>) {
        self.update(
            cam,
            |a, b| coll_line(mesh, a, b, QueryFlags(0x12)).map(|h| (h.point, h.surface_id())),
            water_height,
        );
    }

    /// [`update_with_mesh`](Self::update_with_mesh) with the moby pass of `CollLine_Fix` too: flags 0x12 test the moby
    /// triangle meshes after the world (only their primitives are skipped), so the water the patch managers' mobys
    /// carry as collision (levels 05 / 07 / 11 / 12 / 13: surface-0 faces at the moby's z) puts the camera under water
    /// as the world's water faces do.
    pub fn update_with_scene(&mut self, mesh: &Collision, scene: &crate::collision_query::MobyScene, cam: [f32; 3], water_height: impl FnMut([f32; 3]) -> Option<f32>) {
        self.update(
            cam,
            |a, b| crate::collision_query::coll_line_m(mesh, Some(scene), a, b, QueryFlags(0x12), None).map(|h| (h.point, h.surface_id())),
            water_height,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hero_ambient_from_base_and_static_light() {
        use rc_formats::gameplay::{StaticLight, StaticLights};
        // One light at (40, 40), r² = 16, red 0x80 alpha 0; cell (2, 2) lists it.
        let mut grid = vec![0u32; 4096 + 2];
        grid[2 * 64 + 2] = 4096 * 4;
        grid[4096] = 1;
        grid[4097] = 0;
        let lights = StaticLights { lights: vec![StaticLight { pos: [40.0, 40.0, 0.0, 16.0], rgba: 0x0000_0080 }], grid };
        let (mut base, mut light, mut amb) = ([0x30, 0x30, 0x30, 0], 0u32, [0u8; 4]);
        // At the centre: k = 0, a = 255: 0x80·255 >> 8 = 0x7f added to red.
        hero_env_lighting(None, &lights, [40.0, 40.0, 5.0], &mut base, &mut light, &mut amb);
        assert_eq!(amb, [0x30 + 0x7f, 0x30, 0x30, 0]);
        // 2 units off (d² = 4): k = cvt(4/16·255) = 63, a = 192: 0x80·192 >> 8 = 0x60.
        hero_env_lighting(None, &lights, [42.0, 40.0, 5.0], &mut base, &mut light, &mut amb);
        assert_eq!(amb[0], 0x30 + 0x60);
        // Outside the circle: the base.
        hero_env_lighting(None, &lights, [45.0, 40.0, 5.0], &mut base, &mut light, &mut amb);
        assert_eq!(amb, base);
    }
    use rc_formats::gameplay::FogZoneSide;

    fn bytes(v: [u32; 3]) -> [u8; 3] { v.map(|x| x as u8) }

    /// Level-settings fog of Novalis (docs/plan/game_camera_fog.md §5).
    const OUTDOOR: FogGlobals = FogGlobals { color: [105, 127, 180], near_dist: 0.0, far_dist: 245760.0, near_intensity: 255.0, far_intensity: 102.0 };

    /// A cuboid centred at (10, 20, 5) with half sizes (4, 2, 3), local x along world +y: `inv` maps
    /// world → local (row-vector form).
    fn synthetic() -> FogZones {
        // local = ((y − 20)/4, −(x − 10)/2, (z − 5)/3)
        let inverse = [[0.0, -0.5, 0.0, 0.0], [0.25, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0 / 3.0, 0.0], [-5.0, 5.0, -5.0 / 3.0, 1.0]];
        let zone = FogZone {
            inverse,
            flags: 3,
            fog_color: [0x0f0505, 0xb37e69],
            side: [
                FogZoneSide { near_dist: 0.0, near_density: 0.0, far_dist: 50.0, far_density: 0.8 },
                FogZoneSide { near_dist: 0.0, near_density: 0.0, far_dist: 240.0, far_density: 0.6 },
            ],
            hero_color: [0x261919, 0x282828],
            hero_light: [1, 0],
            ..Default::default()
        };
        FogZones { circles: vec![[10.0, 20.0, 5.0, 25.0], [10.0, 20.0, 5.0, 400.0]], zones: vec![zone, FogZone { flags: 2, ..zone }] }
    }

    #[test]
    fn t_at_the_cuboid_faces() {
        let z = synthetic();
        assert_eq!(lookup(&z, [10.0, 16.0, 5.0]), Some((0, 0.0)), "local x = −1 face");
        assert_eq!(lookup(&z, [10.0, 24.0, 5.0]), Some((0, 1.0)), "local x = +1 face");
        assert_eq!(lookup(&z, [10.0, 20.0, 5.0]), Some((0, 0.5)));
        assert_eq!(lookup(&z, [10.0, 17.0, 5.0]), Some((0, 0.125)));
        // Inside circle 0 but outside the cuboid in z: none, and circle 1 (which contains it too) is not tried.
        assert_eq!(lookup(&z, [10.0, 20.0, 8.5]), None);
        // Outside circle 0 (r = 5), inside circle 1 (r = 20): record 1's cuboid (the same) decides.
        assert_eq!(lookup(&z, [10.0, 26.0, 5.0]), None, "local x 1.5 > 1");
        assert_eq!(lookup(&z, [15.5, 20.0, 5.0]), None, "local y −2.75");
        let only_second = FogZones { circles: vec![[100.0, 0.0, 0.0, 1.0], z.circles[1]], zones: z.zones.clone() };
        assert_eq!(lookup(&only_second, [10.0, 22.0, 5.0]), Some((1, 0.75)));
        assert_eq!(lookup(&FogZones::default(), [0.0; 3]), None);
    }

    #[test]
    fn apply_and_no_restore() {
        let z = synthetic();
        let mut g = OUTDOOR;
        // t = 0: side 1 exactly (colour: i = 0 → c1·255 >> 8).
        apply(&mut g, &z.zones[0], 0.0);
        assert_eq!(g.color, bytes([(5 * 255) >> 8, (5 * 255) >> 8, (15 * 255) >> 8]));
        assert_eq!((g.near_dist, g.far_dist, g.near_intensity), (0.0, 51200.0, 255.0));
        assert!((g.far_intensity - 51.0).abs() < 1e-3);
        // t = 1: side 2 (i = 255).
        apply(&mut g, &z.zones[0], 1.0);
        assert_eq!(g.color, bytes([(105 * 255) >> 8, (126 * 255) >> 8, (179 * 255) >> 8]));
        assert_eq!(g.far_dist, 245760.0);
        assert!((g.far_intensity - 102.0).abs() < 1e-3);
        // Leaving every zone restores nothing.
        let before = g;
        assert_eq!(update(&mut g, &z, [100.0, 100.0, 0.0]), None);
        assert_eq!(g, before);
        // flags & 2 clear: lookup hits, the fog is untouched.
        let no_fog = FogZones { circles: z.circles.clone(), zones: vec![FogZone { flags: 1, ..z.zones[0] }; 2] };
        assert_eq!(update(&mut g, &no_fog, [10.0, 20.0, 5.0]), Some((0, 0.5)));
        assert_eq!(g, before);
    }

    #[test]
    fn hero_light_crossfade() {
        let z = synthetic();
        let (word, colour) = hero_light(&z.zones[0], 0.5).unwrap();
        // i = trunc(0.5·255) = 127.
        assert_eq!(word, 1 | 127 << 16, "bank 1 → bank 0 at i = 127");
        // 0x19·128 >> 8 = 12, 0x28·127 >> 8 = 19 → 31; 0x26·128 >> 8 = 19 → 38.
        assert_eq!(colour, [31, 31, 38, 0]);
        assert_eq!(hero_light(&z.zones[1], 0.5), None);
    }

    #[test]
    fn underwater_hysteresis() {
        let mut s = UnderwaterState::default();
        let none = |_: [f32; 3]| None;
        // Water surface at z = 10 (surface id 0).
        let water = |a: [f32; 3], b: [f32; 3]| (a[2] >= 10.0 && b[2] <= 10.0).then_some(([a[0], a[1], 10.0], 0));
        s.update([0.0, 0.0, 9.5], water, none);
        assert!(s.flag, "0.5 below the surface");
        // Far from the surface (no hit within ±0.75): unchanged, in both states.
        s.update([0.0, 0.0, 5.0], water, none);
        assert!(s.flag);
        s.update([0.0, 0.0, 10.5], water, none);
        assert!(!s.flag, "above: z ≥ h + 0.04");
        s.update([0.0, 0.0, 5.0], water, none);
        assert!(!s.flag, "unchanged on no hit");
        // Just below h + 0.04 counts as underwater; the height callback wins over the hit z.
        s.update([0.0, 0.0, 10.03], water, none);
        assert!(s.flag);
        s.update([0.0, 0.0, 10.03], water, |_| Some(9.9));
        assert!(!s.flag);
        // A non-water face first: the cast restarts 0.01 below it and then finds the water.
        let mut calls = Vec::new();
        let layered = |a: [f32; 3], b: [f32; 3]| {
            calls.push(a[2]);
            if a[2] >= 10.2 && b[2] <= 10.2 { Some(([0.0, 0.0, 10.2], 5)) } else if a[2] >= 10.0 && b[2] <= 10.0 { Some(([0.0, 0.0, 10.0], 0)) } else { None }
        };
        s.update([0.0, 0.0, 9.6], layered, none);
        assert!(s.flag);
        assert_eq!(calls.len(), 2);
        assert!((calls[1] - 10.19).abs() < 1e-5);
        // Six non-water hits: unchanged.
        let mut n = 0;
        s.update([0.0, 0.0, 9.6], |a, _| { n += 1; Some((a, 3)) }, none);
        assert!(s.flag && n == 6);
    }

    #[test]
    fn update_fog_selects_the_alternate_set() {
        let mut look = UnderwaterLook::default();
        assert_eq!(update_fog(&OUTDOOR, false, &look), (OUTDOOR, 0x1f4000));
        let (f, far) = update_fog(&OUTDOOR, true, &look);
        assert_eq!((f.color, f.far_dist, f.far_intensity, far), ([0x18, 0x30, 0x90], 32768.0, 48.0, 0x40000));
        look.set_from_ripple_zone(Some(2));
        assert_eq!((look.fog.color, look.tint), ([0, 40, 48], [0, 40, 48, 0x40]));
        look.set_from_ripple_zone(None);
        assert_eq!(look.tint, [0, 40, 48, 0x40], "no active zone: unchanged");
        look.set_from_ripple_zone(Some(5));
        assert_eq!((look.fog.color, look.tint), ([24, 48, 144], [24, 48, 144, 0x30]));
    }

    /// Novalis disc: stepping into zone 3 (a cave mouth, flags 3) from the outdoor values.
    #[test]
    fn novalis_zone_3_step() {
        let path = rc_formats::test_data::root().join("levels/01/gameplay_ntsc.bin");
        let Ok(wad) = std::fs::read(&path) else { eprintln!("skipped: no {}", path.display()); return };
        let gp = rc_formats::wad::decompress(&wad).unwrap();
        let zones = rc_formats::gameplay::parse_fog_zones(&gp).unwrap();
        let z3 = zones.zones[3];
        // World point at local (lx, 0, 0): p = (l − row3)·inv⁻¹ (3×3 part inverted in f64).
        let world = |lx: f64| -> [f32; 3] {
            let m = z3.inverse.map(|r| r.map(|v| v as f64));
            let a = [[m[0][0], m[0][1], m[0][2]], [m[1][0], m[1][1], m[1][2]], [m[2][0], m[2][1], m[2][2]]];
            let b = [lx - m[3][0], -m[3][1], -m[3][2]];
            // Solve x·A = b, i.e. Aᵀ x = b (Cramer).
            let at = |r: usize, c: usize| a[c][r];
            let det3 = |f: &dyn Fn(usize, usize) -> f64| {
                f(0, 0) * (f(1, 1) * f(2, 2) - f(1, 2) * f(2, 1)) - f(0, 1) * (f(1, 0) * f(2, 2) - f(1, 2) * f(2, 0))
                    + f(0, 2) * (f(1, 0) * f(2, 1) - f(1, 1) * f(2, 0))
            };
            let d = det3(&at);
            std::array::from_fn(|k| (det3(&|r, c| if c == k { b[r] } else { at(r, c) }) / d) as f32)
        };
        // Near the t = 0 face: the cave side (flags 3: fog and hero).
        let p = world(-0.98);
        let (i, t) = lookup(&zones, p).expect("inside zone 3");
        assert_eq!(i, 3, "zone 3 is the first circle containing {p:?}");
        assert!((t - 0.01).abs() < 1e-3, "t {t}");
        let mut g = OUTDOOR;
        update(&mut g, &zones, p);
        // i = trunc(0.01·255) = 2: colour (0·253 + 105·2) >> 8 = 0, (44·253 + 126·2) >> 8 = 44, (120·253 + 179·2) >> 8 = 119.
        assert_eq!(g.color, [0, 44, 119]);
        assert!((g.near_dist / 1024.0 - 15.0 * 0.99).abs() < 0.05, "near {}", g.near_dist / 1024.0);
        assert!((g.far_dist / 1024.0 - (100.0 * 0.99 + 240.0 * 0.01)).abs() < 0.1, "far {}", g.far_dist / 1024.0);
        assert!((g.near_intensity - 255.0).abs() < 1e-3);
        assert!((g.far_intensity - (255.0 - 255.0 * (0.4 * 0.99 + 0.6 * 0.01))).abs() < 0.1, "far F {}", g.far_intensity);
        // Midpoint and the outdoor face.
        update(&mut g, &zones, world(0.0));
        assert_eq!(g.color, bytes([(105 * 127) >> 8, (44 * 128 + 126 * 127) >> 8, (120 * 128 + 179 * 127) >> 8]));
        assert!((g.far_dist / 1024.0 - 170.0).abs() < 0.05);
        update(&mut g, &zones, world(0.99));
        assert!((g.far_dist / 1024.0 - 239.3).abs() < 0.05, "t = 0.995: far {}", g.far_dist / 1024.0);
        assert!(g.far_intensity > 102.0 && g.far_intensity < 103.5);
        // Stepping back out of the circle keeps the last values.
        let last = g;
        update(&mut g, &zones, [200.0, 300.0, 60.0]);
        assert_eq!(g, last);
        // Hero cross-fade for the same record: bank 2 → 0.
        assert_eq!(hero_light(&z3, 0.5).unwrap().0, 2 | 127 << 16);
    }
}
