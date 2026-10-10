//! Tie LOD as the game does it: `TieProc` (boot 0x235be8, level01 0x2a9a90) replayed on the CPU every
//! frame for every tie instance, and the per-instance VU1 quadword 4 it uploads (the fat-vertex morph
//! factor and the colour weights of programs 13507 / 224979). Spec: docs/formats/tie_rac1.md §3.5; the
//! shader side is in `assets/shaders/tie.wgsl`, the meshes and material in `tie_render.rs`.
//!
//! Per instance and frame (`update_tie_lods`) one `vec4<u32>` goes into a storage buffer every tie
//! material binds (index = instance index = `MeshTag`). The occlusion test (tie +0x18/+0x19 against the
//! frame mask, crate::occlusion) comes first; an occluded instance gets the culled record:
//! x = LOD (0..2, [`LOD_CULLED`] = not drawn) | F << 8 (the GS fog value `TieProc` puts in qw 5.w),
//! y = k (f32 bits), z = qw4.w = 256·k, w = qw4.z = 256 − 256·k (the VU colour weights, f32 bits).
//! The vertex shader drops the vertices of the other LODs, draws fat vertices at `position + k·delta`
//! and blends their colour with the VU's float arithmetic (`vu_fat_color`).
//!
//! **CPU culling.** An instance `TieProc` does not draw (occluded or culled) also gets `Visibility::Hidden` on
//! its part entities, written only when its state changes, so Bevy does not queue it and the GPU does not
//! run its vertices (all three LODs) just to drop them. The vertex shader's `LOD_CULLED` test stays as the
//! backstop; the per-instance rules are unchanged. The written state takes effect the same frame (the system
//! runs before `VisibilitySystems::VisibilityPropagate`).
//!
//! Environment: `RC_TIE_LOD=0` forces every drawn instance to LOD 0 with k = 0 (culling unchanged);
//! `RC_TIE_LOD_TINT=1` tints LOD 1 red and LOD 2 blue; `RC_TIE_CPU_CULL=0` leaves culled instances visible
//! to Bevy (the vertex shader alone drops them, as before).

use crate::game_camera::{self, LevelFog, NEAR, UNITS};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::occlusion::OcclBits;
use rc_formats::tfrag_light::ps2;

/// LOD code of an instance `TieProc` does not draw this frame.
pub const LOD_CULLED: u32 = 3;
/// 256.0: the colour-weight scale (`lui at, 0x4380`).
const F256: u32 = 0x4380_0000;

/// What `TieProc` decides for one drawn instance: the packet list (`lod`) and VU1 qw4 = (k, −, z, w).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TieLodPick {
    pub lod: u32,
    /// Morph factor (qw4.x): fat vertices are drawn at `position + k·delta`.
    pub k: f32,
    /// qw4.w = 256·k: weight of the fat vertex's averaged second/third colour.
    pub w: f32,
    /// qw4.z = 256 − 256·k (VU adder: w + z lies in [256, 256 + 2^-16)): weight of its first colour.
    pub z: f32,
}

impl TieLodPick {
    /// The static paths store qw4 = (0, 0, 256.0, 0) (`por at, zero, zero; lui at, 0x4380; prot3w at, at`).
    fn fixed(lod: u32) -> Self { TieLodPick { lod, k: 0.0, w: 0.0, z: 256.0 } }
    /// The morph paths: `vdiv Q`, qw4 = (Q, 0, 256 − 256·Q, 256·Q) (`vmulq.w`, `vaddq.x`, `vsubw.z`, VU0 FMAC).
    fn morph(lod: u32, num: u32, den: u32) -> Self {
        let q = ps2::div(num, den);
        let w = ps2::mul(F256, q);
        let z = ps2::sub(F256, w);
        TieLodPick { lod, k: f32::from_bits(q), w: f32::from_bits(w), z: f32::from_bits(z) }
    }
}

/// `TieProc` pass 2 (boot 0x236288..0x2364c8) for an instance at view depth `depth` (= max(0, z) of its
/// bounding-sphere centre, world units) and the class `near/mid/far_dist` (header 0x10, `lqc2 vf16`):
/// `vf2 = dists − depth`, then by sign bit
/// * near − depth ≥ 0: LOD 0, k = 0;
/// * far − depth < 0: LOD 2, k = 0;
/// * mid − depth ≥ 0: LOD 0, k = (depth − near) / (mid − near);
/// * otherwise: LOD 1, k = (depth − mid) / (far − mid).
pub fn tie_lod(depth: f32, dists: [f32; 3]) -> TieLodPick {
    let [near, mid, far] = dists.map(f32::to_bits);
    let d = depth.to_bits();
    let (dn, dm, df) = (ps2::sub(near, d), ps2::sub(mid, d), ps2::sub(far, d));
    let neg = |x: u32| x & ps2::SIGN != 0;
    if !neg(dn) { return TieLodPick::fixed(0); }
    if neg(df) { return TieLodPick::fixed(2); }
    if !neg(dm) { return TieLodPick::morph(0, ps2::sub(0, dn), ps2::sub(mid, near)); }
    TieLodPick::morph(1, ps2::sub(0, dm), ps2::sub(far, mid))
}

/// `TieProc` pass 1 (boot 0x235f1c..0x236040) on the camera-space bounding sphere `p` (x = right,
/// y = down, z = forward, world units) with radius `r` and draw distance `dist` (instance distance,
/// already `pminw`ed with the 720.0 cap): `None` when culled, else the depth pass 2 uses, max(0, z)
/// (`vmax.z vf1, vf0, vf2` then `vmr32.xy`).
/// * `dist` = 0 (`beq v0, zero`), `(dist − r) − (z − r) < 0` (beyond the draw distance) or
///   `n/1024 − (z + r) ≥ 0` (wholly in front of the near plane): culled;
/// * wholly outside a side plane, `tan·z − (|x| − r·kf) < 0` (x and y; vf20 = 0x18cdb0, vf22 = 0x18cee0,
///   as for tfrags): culled.
///
/// The occlusion bits (+0x18/+0x19 against the mask copy at 0x70003000, level01 0x2a9dd8..0x2a9e2c) are
/// tested before this, in `update_tie_lods`. Not modelled: the guard-band box test that sends an instance
/// to the clipping program 224979 (same LOD and qw4, the GPU clips instead).
pub fn tie_cull(p: Vec3, r: f32, dist: f32, tans: (f32, f32)) -> Option<f32> {
    if dist.to_bits() == 0 { return None; }
    if (dist - r) - (p.z - r) < 0.0 || NEAR / UNITS - (p.z + r) >= 0.0 { return None; }
    let (tx, ty) = tans;
    let (kx, ky) = ((1.0 + tx * tx).sqrt(), (1.0 + ty * ty).sqrt());
    if tx * p.z - (p.x.abs() - r * kx) < 0.0 || ty * p.z - (p.y.abs() - r * ky) < 0.0 { return None; }
    Some(p.z.max(0.0))
}

/// The per-instance GS fog value: `vadda.y ACC, vf18, vf0; vmaddx.y vf4, vf1, vf18` =
/// `cf24 + depth·cf20`, `vminibcz` with In, `vmaxy.w` with If (vf18 = view context 0x18cf20 =
/// (cf20, cf24, In, If), `UpdateViewContext`), then `ftoi4` into XYZF2 (truncated).
pub fn fog_value(depth: f32, fog: &LevelFog) -> u32 {
    let [dn, df, i_n, i_f] = [fog.near_dist, fog.far_dist, fog.near_intensity, fog.far_intensity].map(f32::to_bits);
    let range = ps2::sub(df, dn);
    let cf20 = ps2::div(ps2::sub(i_f, i_n), ps2::mul(range, 0x3a80_0000));
    let cf24 = ps2::sub(i_n, ps2::mul(ps2::mul(dn, 0x3a80_0000), cf20));
    let f = ps2::max(ps2::min(ps2::add(cf24, ps2::mul(depth.to_bits(), cf20)), i_n), i_f);
    f32::from_bits(f).clamp(0.0, 255.0) as u32
}

/// One colour lane of a fat vertex exactly as VU1 computes it (program 13507 L25..L39, lines 564–700;
/// 224979 lines 588–670): the palette lanes are `0x4b000000 + byte` (V4_8 USN with STMOD 1, STROW
/// 2^23, boot 0x1de8e0), `mulay/maddy` (× 0.5, `addi.y vf27, vf00, I`) averages the second and third
/// slots, `mulaw / maddz` weights `avg·w + c0·z`, every step truncated; the GS takes the low byte
/// (PACKED RGBAQ). `w`, `z` are the qw4 bits of [`TieLodPick`]. The shader's `vu_fat_lane` is this
/// with [`vu_mul`] / [`vu_add`].
#[cfg_attr(not(test), allow(dead_code))] // CPU reference for tie.wgsl, exercised by the tests
pub fn vu_fat_color(c0: u8, c1: u8, c2: u8, w: u32, z: u32) -> u8 {
    let pal = |c: u8| 0x4b00_0000 | c as u32;
    let avg = ps2::add(ps2::mul(pal(c1), 0x3f00_0000), ps2::mul(pal(c2), 0x3f00_0000));
    ps2::add(ps2::mul(avg, w), ps2::mul(pal(c0), z)) as u8
}

/// `ps2::mul` for non-negative operands in 32-bit integer arithmetic (the WGSL `vu_mul`, line for line).
#[cfg_attr(not(test), allow(dead_code))] // CPU reference for tie.wgsl, exercised by the tests
pub fn vu_mul(a: u32, b: u32) -> u32 {
    let (ea, eb) = (((a >> 23) & 0xff) as i32, ((b >> 23) & 0xff) as i32);
    if ea == 0 || eb == 0 { return 0; }
    let (ma, mb) = ((a & 0x7f_ffff) | 0x80_0000, (b & 0x7f_ffff) | 0x80_0000);
    let (ah, al, bh, bl) = (ma >> 12, ma & 0xfff, mb >> 12, mb & 0xfff);
    let mid = ah * bl + al * bh;
    let low = ((mid & 0xfff) << 12) + al * bl;
    let top = ah * bh + (mid >> 12) + (low >> 24); // floor(ma·mb / 2^24)
    let (m, e) = if top >= 0x80_0000 { (top, ea + eb - 126) } else { ((top << 1) | ((low >> 23) & 1), ea + eb - 127) };
    if e < 1 { return 0; }
    if e > 255 { return 0x7fff_ffff; }
    ((e as u32) << 23) | (m & 0x7f_ffff)
}

/// `ps2::add` for non-negative operands (the WGSL `vu_add`).
#[cfg_attr(not(test), allow(dead_code))] // CPU reference for tie.wgsl, exercised by the tests
pub fn vu_add(a: u32, b: u32) -> u32 {
    let (ea, eb) = ((a >> 23) as i32, (b >> 23) as i32);
    if ea == 0 { return b; }
    if eb == 0 { return a; }
    let (hi, lo, eh, el) = if ea >= eb { (a, b, ea, eb) } else { (b, a, eb, ea) };
    let d = (eh - el) as u32;
    if d >= 25 { return hi; }
    let s = ((hi & 0x7f_ffff) | 0x80_0000) + (((lo & 0x7f_ffff) | 0x80_0000) >> d);
    if s >= 0x100_0000 { return (((eh + 1) as u32) << 23) | ((s >> 1) & 0x7f_ffff); }
    ((eh as u32) << 23) | (s & 0x7f_ffff)
}

/// Per-instance inputs of the replay.
#[derive(Clone, Copy, Debug)]
pub struct TieLodInput {
    /// World bounding-sphere centre (game coordinates) and radius (`tie_light::instance_centre`).
    pub sphere: [f32; 4],
    /// Draw distance after the 720.0 cap; 0 = never drawn (also used for instances without a class).
    pub dist: f32,
    /// Class `near/mid/far_dist`.
    pub dists: [f32; 3],
}

/// The buffer `update_tie_lods` writes and what it needs.
#[derive(Resource)]
pub struct TieLodState {
    pub buffer: Handle<ShaderBuffer>,
    inputs: Vec<TieLodInput>,
    /// Per instance (gameplay order): the load-time occlusion word, runtime tie +0x18.
    occl: Vec<OcclBits>,
    /// The fog `fog_value` uses; `crate::fog_state` keeps it equal to the frame's `GameFog`.
    pub(crate) fog: LevelFog,
    force_lod0: bool,
    last_hist: [usize; 6],
    /// Per instance: its part entities (none for instances that are never drawn).
    entities: Vec<Vec<Entity>>,
    /// Per instance: whether its entities are currently shown (`Visibility::Inherited`).
    shown: Vec<bool>,
    /// `RC_TIE_CPU_CULL` (default on).
    cpu_cull: bool,
    /// What follows the per-instance words in the buffer: one `vec4<u32>` per instance whose x is its point-light
    /// nibble list, then the point-light bank (16 `vec4`; crate::world_lights; tie.wgsl reads both).
    tail: Vec<u8>,
}

impl TieLodState {
    pub fn new(inputs: Vec<TieLodInput>, occl: Vec<OcclBits>, fog: LevelFog, buffers: &mut Assets<ShaderBuffer>) -> Self {
        assert_eq!(inputs.len(), occl.len(), "one occlusion word per tie instance");
        let init: Vec<u32> = inputs.iter().flat_map(|_| [LOD_CULLED, 0, 0, 0]).collect();
        // After the instance words: every nibble list empty (0xffff), an empty bank (crate::world_lights).
        let mut tail = vec![0u8; inputs.len() * 16 + crate::world_lights::BANK_BYTES];
        for i in 0..inputs.len() { tail[i * 16..i * 16 + 4].copy_from_slice(&0xffffu32.to_le_bytes()); }
        let mut data = u32_bytes(&init);
        data.extend_from_slice(&tail);
        // MAIN_WORLD too: rewritten every frame at the same size (see tfrag_lod).
        let buffer = buffers.add(ShaderBuffer::new(&data, RenderAssetUsages::default()));
        let force_lod0 = std::env::var("RC_TIE_LOD").is_ok_and(|v| v.trim() == "0");
        let cpu_cull = !std::env::var("RC_TIE_CPU_CULL").is_ok_and(|v| v.trim() == "0");
        let n = inputs.len();
        TieLodState { buffer, inputs, occl, fog, force_lod0, last_hist: [usize::MAX; 6], entities: vec![Vec::new(); n], shown: vec![true; n], cpu_cull, tail }
    }

    /// crate::world_lights: the ties' nibble lists and the bank bytes, into the buffer after the instance words.
    pub fn set_point_lights(&mut self, lists: &[u16], bank: &[u8], buffers: &mut Assets<ShaderBuffer>) {
        let n = self.inputs.len();
        for (i, &l) in lists.iter().take(n).enumerate() { self.tail[i * 16..i * 16 + 4].copy_from_slice(&(l as u32).to_le_bytes()); }
        self.tail[n * 16..n * 16 + bank.len()].copy_from_slice(bank);
        // Only a changed tail marks the buffer (crate::asset_write: every get_mut is an upload).
        let same = buffers.get(&self.buffer).and_then(|b| b.data.as_deref()).is_some_and(|d| d.len() >= self.tail.len() && d[d.len() - self.tail.len()..] == self.tail[..]);
        if same { return; }
        if let Some(mut buf) = buffers.get_mut(&self.buffer) {
            if let Some(d) = buf.data.as_mut() {
                let at = d.len() - self.tail.len();
                d[at..].copy_from_slice(&self.tail);
            }
        }
    }

    /// Instance `ii`'s part entities (spawned visible).
    pub fn set_entities(&mut self, ii: usize, entities: Vec<Entity>) {
        if let Some(e) = self.entities.get_mut(ii) { *e = entities; }
    }
}

/// `RC_TIE_LOD_TINT=1`.
pub fn tint_enabled() -> bool { std::env::var("RC_TIE_LOD_TINT").is_ok_and(|v| v.trim() == "1") }

/// One instance's record: (LOD | F << 8, k, w, z). `scale`: the Detail distance option on the class LOD distances
/// (crate::graphics; 1 = the game's, the draw distance is never scaled).
fn record(inp: &TieLodInput, eye: Vec3, rows: [Vec3; 3], fog: &LevelFog, force_lod0: bool, tans: (f32, f32), scale: f32) -> ([u32; 4], usize) {
    let [x, y, z, r] = inp.sphere;
    let d = Vec3::new(x, y, z) - eye;
    let p = Vec3::new(rows[0].dot(d), rows[1].dot(d), rows[2].dot(d));
    // The cap 0x160fe0 (720, already in `inp.dist`): 144 in the Visibomb's view (crate::visibomb_view::SHORT_FAR).
    let dist = if crate::visibomb_view::SHORT_FAR.load(std::sync::atomic::Ordering::Relaxed) { inp.dist.min(crate::visibomb_view::SHORT_TIE_CAP) } else { inp.dist };
    let Some(depth) = tie_cull(p, r, dist, tans) else { return ([LOD_CULLED, 0, 0, 0], 0) };
    let dists = if scale == 1.0 { inp.dists } else { inp.dists.map(|d| d * scale) };
    let pick = if force_lod0 { TieLodPick::fixed(0) } else { tie_lod(depth, dists) };
    let bin = match (pick.lod, pick.w != 0.0) { (0, false) => 1, (0, true) => 2, (1, _) => 3, _ => 4 };
    ([pick.lod | fog_value(depth, fog) << 8, pick.k.to_bits(), pick.w.to_bits(), pick.z.to_bits()], bin)
}

pub fn update_tie_lods(
    state: Option<ResMut<TieLodState>>,
    occl: Option<ResMut<crate::occlusion::OcclusionFrame>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    cams: Query<(&Transform, Option<&Projection>), With<Camera3d>>,
    (time, gfx): (Res<Time>, Res<crate::graphics::GraphicsSettings>),
    mut last_print: Local<f32>,
    mut vis: Query<&mut Visibility>,
) {
    let scale = gfx.detail.lod_scale();
    let (Some(mut state), Some((cam, proj))) = (state, cams.iter().next()) else { return };
    let tans = game_camera::projection_tans(proj);
    let (eye, [fwd, left, up]) = (game_camera::game_eye(cam), game_camera::game_rows(cam));
    let rows = [-left, -up, fwd];
    // hist: occluded, culled, LOD0, LOD0 morphing, LOD1, LOD2.
    let mut hist = [0usize; 6];
    let mut words = Vec::with_capacity(state.inputs.len() * 4);
    let mask = occl.as_ref().map(|o| o.mask);
    for (inp, bits) in state.inputs.iter().zip(&state.occl) {
        // TieProc: `mask[+0x19] & +0x18 == 0 → next`, before the draw-distance and sphere tests.
        if mask.as_ref().is_some_and(|m| !bits.visible(m)) {
            hist[0] += 1;
            words.extend_from_slice(&[LOD_CULLED, 0, 0, 0]);
            continue;
        }
        let (rec, bin) = record(inp, eye, rows, &state.fog, state.force_lod0, tans, scale);
        hist[bin + 1] += 1;
        words.extend_from_slice(&rec);
    }
    // Show / hide the instances whose drawn state changed (x & 0xff is the LOD, LOD_CULLED = not drawn).
    if state.cpu_cull {
        let st = &mut *state;
        for (ii, rec) in words.as_chunks::<4>().0.iter().enumerate() {
            let drawn = rec[0] & 0xff != LOD_CULLED;
            if drawn == st.shown[ii] || st.entities[ii].is_empty() { continue; }
            st.shown[ii] = drawn;
            let want = if drawn { Visibility::Inherited } else { Visibility::Hidden };
            for &e in &st.entities[ii] {
                if let Ok(mut v) = vis.get_mut(e) { *v = want; }
            }
        }
    }
    if let Some(mut o) = occl {
        o.ties = crate::occlusion::CullCounts { occluded: hist[0], culled: hist[1], drawn: hist[2..].iter().sum() };
    }
    let mut d = u32_bytes(&words);
    d.extend_from_slice(&state.tail);
    crate::asset_write::set_buffer(&mut buffers, &state.buffer, &d);
    if hist != state.last_hist && time.elapsed_secs() - *last_print >= 1.0 {
        *last_print = time.elapsed_secs();
        state.last_hist = hist;
        println!(
            "tie LODs: occluded {}, culled {}, LOD0 {} + {} morphing, LOD1 {} (morphing), LOD2 {}",
            hist[0], hist[1], hist[2], hist[3], hist[4], hist[5]
        );
    }
}

fn u32_bytes(v: &[u32]) -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() }

#[cfg(test)]
mod tests {
    use super::*;

    const D: [f32; 3] = [10.0, 20.0, 30.0];

    fn close(a: f32, b: f32) -> bool { (a - b).abs() < 1e-5 }

    #[test]
    fn lod_boundaries() {
        let p = |d: f32| tie_lod(d, D);
        assert_eq!(p(0.0), TieLodPick::fixed(0));
        assert_eq!(p(10.0), TieLodPick::fixed(0)); // near − depth = +0: not negative
        let a = p(12.5);
        assert!(a.lod == 0 && close(a.k, 0.25) && close(a.w, 64.0) && close(a.z, 192.0), "{a:?}");
        let b = p(20.0); // mid − depth = +0: still LOD 0, fully morphed
        assert!(b.lod == 0 && b.k == 1.0 && b.w == 256.0 && b.z == 0.0, "{b:?}");
        let c = p(20.5);
        assert!(c.lod == 1 && close(c.k, 0.05), "{c:?}");
        let e = p(30.0); // far − depth = +0: LOD 1 at k = 1, not LOD 2
        assert!(e.lod == 1 && e.k == 1.0, "{e:?}");
        assert_eq!(p(30.01), TieLodPick::fixed(2));
        assert_eq!(p(719.0), TieLodPick::fixed(2));
        // k stays in (0, 1] on the morph paths.
        for i in 1..4000 {
            let t = tie_lod(10.0 + i as f32 * 0.005, D);
            assert!(t.k > 0.0 && t.k <= 1.0 && t.w + t.z > 255.99 && t.w + t.z < 256.01, "{t:?}");
        }
        // 10/20/1024 classes (the other common Novalis set) morph LOD 1 over 20..1024.
        let f = tie_lod(522.0, [10.0, 20.0, 1024.0]);
        assert!(f.lod == 1 && close(f.k, 0.5), "{f:?}");
        // The tests run in the game's order: far < depth wins over depth <= mid.
        assert_eq!(tie_lod(15.0, [10.0, 20.0, 12.0]), TieLodPick::fixed(2));
    }

    #[test]
    fn culling() {
        assert_eq!(tie_cull(Vec3::new(0.0, 0.0, 50.0), 5.0, 720.0, crate::game_camera::default_tans()), Some(50.0));
        assert_eq!(tie_cull(Vec3::new(0.0, 0.0, 50.0), 5.0, 0.0, crate::game_camera::default_tans()), None);
        assert_eq!(tie_cull(Vec3::new(0.0, 0.0, 50.0), 5.0, 49.0, crate::game_camera::default_tans()), None); // centre beyond the draw distance
        assert_eq!(tie_cull(Vec3::new(0.0, 0.0, 50.0), 5.0, 50.0, crate::game_camera::default_tans()), Some(50.0));
        assert_eq!(tie_cull(Vec3::new(0.0, 0.0, -10.0), 5.0, 720.0, crate::game_camera::default_tans()), None); // behind
        assert_eq!(tie_cull(Vec3::new(0.0, 0.0, -2.0), 5.0, 720.0, crate::game_camera::default_tans()), Some(0.0)); // straddles the camera: depth 0
        assert_eq!(tie_cull(Vec3::new(100.0, 0.0, 50.0), 5.0, 720.0, crate::game_camera::default_tans()), None); // right of the frustum
        assert!(tie_cull(Vec3::new(33.0, 0.0, 50.0), 5.0, 720.0, crate::game_camera::default_tans()).is_some()); // overlaps the side plane
    }

    #[test]
    fn fog_matches_the_tfrag_line() {
        // Novalis: F = 255 at depth 0 down to 102 at 240 units, linear in depth.
        let fog = LevelFog { color: [105, 127, 180], near_dist: 0.0, far_dist: 245760.0, near_intensity: 255.0, far_intensity: 102.0 };
        assert_eq!(fog_value(0.0, &fog), 255);
        assert_eq!(fog_value(120.0, &fog), 178); // 255 − 76.5, truncated
        assert_eq!(fog_value(240.0, &fog), 102);
        assert_eq!(fog_value(700.0, &fog), 102);
    }

    #[test]
    fn integer_vu_ops_match_the_float_model() {
        let mut s = 0x1234_5678u32;
        let mut rnd = || { s ^= s << 13; s ^= s >> 17; s ^= s << 5; s };
        for i in 0..200_000 {
            let (mut a, mut b) = (rnd() & 0x7fff_ffff, rnd() & 0x7fff_ffff);
            if i % 3 == 0 { a = 0x4b00_0000 | (a & 0xff); b = (b & 0x007f_ffff) | 0x4300_0000; } // palette × weight range
            if i % 7 == 0 { b = 0; }
            if (a >> 23) & 0xff < 64 || (b >> 23) & 0xff < 64 || (a >> 23) & 0xff > 190 || (b >> 23) & 0xff > 190 { continue; }
            assert_eq!(vu_mul(a, b), ps2::mul(a, b), "mul {a:08x} {b:08x}");
            assert_eq!(vu_add(a, b), ps2::add(a, b), "add {a:08x} {b:08x}");
        }
        let lane = |c0: u8, c1: u8, c2: u8, w: u32, z: u32| {
            let pal = |c: u8| 0x4b00_0000 | c as u32;
            let avg = vu_add(vu_mul(pal(c1), 0x3f00_0000), vu_mul(pal(c2), 0x3f00_0000));
            vu_add(vu_mul(avg, w), vu_mul(pal(c0), z)) as u8
        };
        for depth in [10.0f32, 10.001, 11.0, 13.3, 15.0, 17.77, 19.99, 20.0] {
            let p = tie_lod(depth, D);
            for (c0, c1, c2) in [(0u8, 0u8, 0u8), (7, 200, 13), (128, 128, 128), (255, 0, 255), (60, 61, 62)] {
                assert_eq!(lane(c0, c1, c2, p.w.to_bits(), p.z.to_bits()), vu_fat_color(c0, c1, c2, p.w.to_bits(), p.z.to_bits()));
            }
        }
    }

    #[test]
    fn fat_colour_blend() {
        let at = |depth: f32| { let p = tie_lod(depth, D); (p.w.to_bits(), p.z.to_bits()) };
        // k = 0 (static LOD) gives the first slot exactly; k = 1 the truncated average of the other two.
        for (c0, c1, c2) in [(0u8, 0u8, 0u8), (7, 200, 13), (255, 254, 255), (128, 0, 1)] {
            let (w, z) = at(5.0);
            assert_eq!(vu_fat_color(c0, c1, c2, w, z), c0);
            let (w, z) = at(20.0);
            assert_eq!(vu_fat_color(c0, c1, c2, w, z), ((c1 as u32 + c2 as u32) / 2) as u8);
        }
        // Halfway: 0.5·c0 + 0.5·avg, truncated (possibly one lower from the truncated products).
        let (w, z) = at(15.0);
        let v = vu_fat_color(100, 200, 200, w, z);
        assert!(v == 150 || v == 149, "{v}");
        // 256 − 256k goes through the VU adder, which drops the bits of 256k more than one below 256's LSB
        // before subtracting: w + z is never below 256 (up to 2^-16 above), so a lane never falls under
        // 2^31 (where the low byte would wrap) and black stays black.
        for depth in [10.001f32, 10.3, 12.345, 19.999] {
            let (w, z) = at(depth);
            let sum = f64::from(f32::from_bits(w)) + f64::from(f32::from_bits(z));
            assert!((256.0..256.0 + 1.0 / 65536.0).contains(&sum), "{depth}: {sum}");
            assert_eq!(vu_fat_color(0, 0, 0, w, z), 0);
            // The truncated products can cost one step: white may dim to 254 while morphing.
            assert!(matches!(vu_fat_color(255, 255, 255, w, z), 254 | 255));
        }
    }
}
