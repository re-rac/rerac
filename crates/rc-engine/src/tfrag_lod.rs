//! Tfrag LOD as the game does it: the per-frame, per-tfrag draw mode of `TfragProc` (boot 0x233fb0), the
//! VU1 morph constants of `SetTfragDists` (boot 0x233068), and the level data the LOD path needs
//! (block header, mip chains). Spec: docs/formats/tfrag_rac1.md §3b, §2.5.1; the shader side is in
//! `assets/shaders/tfrag.wgsl`, the mesh/storage layout in `tfrag_render.rs`.
//!
//! Each frame `update_tfrag_modes` replays `TfragProc` on the CPU for every tfrag (1004 on Novalis) and
//! writes one u32 per tfrag into a storage buffer that every tfrag material binds. The occlusion test
//! (crate::occlusion) comes first, as in `TfragProc`; only tfrags that pass it reach `tfrag_proc`.
//! 0 = culled, 2 = clipping path (LOD 0, no morph), otherwise the MSCAL address of the VU1 entry
//! (6 / 8 / 0xa / 0xe / 0x10 / 0x14, `TfragDrawMode::mscal`). The vertex shader drops the triangles of the
//! lists the mode does not draw and runs the morph/collapse passes of the chosen entry.
//!
//! Environment: `RC_LOD=0` forces every drawn tfrag to MSCAL 0x14 (LOD 0, no morph; culling unchanged);
//! `RC_LOD_TINT=1` tints LOD 1 red, LOD 2 blue and the clipping path green; `RC_NOVSYNC=1` presents without
//! vsync so `fps:` measures headroom.

use crate::game_camera::{self, LevelFog, NEAR, FAR, UNITS};
use crate::level_load::LoadedLevel;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bevy::render::storage::ShaderBuffer;
use bevy::window::{PresentMode, PrimaryWindow};
use rc_formats::level::LevelCore;
use rc_formats::texture::{self, Texture};
use rc_formats::occlusion::OcclBits;
use rc_formats::tfrag::{self, Tfrag, TfragBlockHeader};

/// Draw-mode codes in the per-tfrag storage buffer (besides the MSCAL addresses 6..0x14).
pub const MODE_CULLED: u32 = 0;
pub const MODE_CLIP: u32 = 2;

/// Level data for the LOD path, loaded next to the tfrags.
pub struct TfragLodData {
    /// Tfrags block header (`unknown_8` = the LOD base distance L).
    pub block: TfragBlockHeader,
    /// Full mip chain per `LevelCore::tfrag_textures` index (`None` where the entry has no pixels).
    pub mips: Vec<Option<Vec<Texture>>>,
}

/// Reads the block header and decodes every tfrag texture's mip chain (`texture::decode_tfrag_mip_levels`).
pub fn load(core: &LevelCore, core_data: &[u8], gs_ram: &[u8]) -> Result<TfragLodData> {
    let block = tfrag::parse_tfrag_block_header(tfrag::tfrag_block(core, core_data)?)?;
    let mips = core
        .tfrag_textures
        .iter()
        .map(|e| {
            if e.width <= 0 || e.height <= 0 || e.data_offset < 0 || e.palette < 0 { return Ok(None); }
            texture::decode_tfrag_mip_levels(core, core_data, gs_ram, e).map(Some)
        })
        .collect::<rc_formats::buf::Result<Vec<_>>>()
        .context("decoding tfrag mip levels")?;
    Ok(TfragLodData { block, mips })
}

/// VU1 constants qw666..669 exactly as `SetTfragDists` computes them (f32, same operation order), plus the
/// w slope the VU's w lane has per raw unit of camera depth (the fog slope, `0x18cdec`).
///
/// `cf20` (`UpdateViewContext`) = (If − In) / ((Df − Dn) · (1/1024)), `fi = Di · cf20`,
/// `a = 1 / (f0 − f1)`, `b = 1 / (f1 − f2)`:
/// qw666 = (a/2, −a, 0, f0), qw667 = (b/2, −b, 0, f1), qw668 = (−f1·a/2, f0·a, 0, 0), qw669 = (−f2·b/2, f1·b, 0, 0).
/// The VU then does `t.xy = clamp(w·qw66x.xy + qw66y.xy, 0, (0.5, 1))` = (u/2, 1 − u) and collapses when both
/// parents have `w < qw66x.w`.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct TfragLodUniform {
    pub k666: Vec4,
    pub k667: Vec4,
    pub k668: Vec4,
    pub k669: Vec4,
    /// x = VU w per raw unit of camera depth, y = 1 to tint by LOD (`RC_LOD_TINT=1`), z = near (32, Q = n / z).
    pub misc: Vec4,
    /// Per batch: y = MXL (highest mip level). (TEX1 K comes per vertex.)
    pub tex: Vec4,
}

impl TfragLodUniform {
    pub fn new(block: &TfragBlockHeader, fog: &LevelFog) -> Self {
        let tint = std::env::var("RC_LOD_TINT").is_ok_and(|v| v.trim() == "1");
        let mut u = TfragLodUniform { misc: Vec4::new(0.0, if tint { 1.0 } else { 0.0 }, NEAR, 0.0), ..default() };
        u.set_fog(block, fog);
        u
    }

    /// `UpdateViewContext` → `SetTfragDists` for the current view-context fog: rewrites qw666..669 and the
    /// w slope, keeps the tint flag and the batch's `tex`. The game reruns this every frame (`UpdateFog` at
    /// the end of each frame render), so the LOD distances follow the fog zones and the underwater fog.
    pub fn set_fog(&mut self, block: &TfragBlockHeader, fog: &LevelFog) {
        let [d0, d1, d2] = block.lod_distances();
        let cf20 = (fog.far_intensity - fog.near_intensity) / ((fog.far_dist - fog.near_dist) * 0.0009765625);
        let (f0, f1, f2) = (d0 * cf20, d1 * cf20, d2 * cf20);
        let a = 1.0 / (f0 - f1);
        let b = 1.0 / (f1 - f2);
        self.k666 = Vec4::new(a * 0.5, -a, 0.0, f0);
        self.k667 = Vec4::new(b * 0.5, -b, 0.0, f1);
        self.k668 = Vec4::new(f1 * a * -0.5, f0 * a, 0.0, 0.0);
        self.k669 = Vec4::new(f2 * b * -0.5, f1 * b, 0.0, 0.0);
        self.misc.x = fog.vu_terms().1;
    }
}

/// Every frame after `crate::fog_state`: the tfrag materials' LOD constants from the frame's fog
/// ([`TfragLodUniform::set_fog`]), written only where they changed.
fn update_lod_constants(
    fog: Res<crate::game_camera::GameFog>,
    state: Option<Res<TfragLodState>>,
    mut materials: ResMut<Assets<crate::tfrag_render::TfragMaterial>>,
) {
    let Some(state) = state else { return };
    let stale: Vec<_> = materials
        .iter()
        .filter(|(_, m)| {
            let mut want = m.lod;
            want.set_fog(&state.block, &fog.fog);
            want != m.lod
        })
        .map(|(id, _)| id)
        .collect();
    for id in stale {
        if let Some(mut m) = materials.get_mut(id) {
            let mut lod = m.lod;
            lod.set_fog(&state.block, &fog.fog);
            m.lod = lod;
        }
    }
}

/// Per-tfrag data `TfragProc` reads: bounding sphere (raw units), the 8 clip-box corners (absolute raw
/// units, s16 × 64), `base_only`, and the tfrag itself for `Tfrag::draw_mode`.
struct TfragCullInfo {
    sphere: [f32; 4],
    corners: [Vec3; 8],
}

/// Everything `update_tfrag_modes` needs, plus the buffer it writes.
#[derive(Resource)]
pub struct TfragLodState {
    pub modes: Handle<ShaderBuffer>,
    tfrags: Vec<TfragCullInfo>,
    draw: Vec<Tfrag>,
    /// Per tfrag: the load-time occlusion word (header 0x3a/0x3b, `crate::occlusion`).
    occl: Vec<OcclBits>,
    thresholds: [i32; 3],
    /// The block header (`unknown_8` = L) for `SetTfragDists`.
    block: TfragBlockHeader,
    force_lod0: bool,
    last_hist: [usize; 8],
    /// What follows the draw modes in the buffer: one u32 per tfrag with its point-light nibble list, then the
    /// point-light bank (crate::world_lights; tfrag.wgsl reads both).
    tail: Vec<u8>,
}

impl TfragLodState {
    /// crate::world_lights: the tfrags' nibble lists and the bank bytes, into the buffer after the modes.
    pub fn set_point_lights(&mut self, lists: &[u16], bank: &[u8], buffers: &mut Assets<ShaderBuffer>) {
        let n = self.tfrags.len().max(1);
        for (i, &l) in lists.iter().take(n).enumerate() { self.tail[i * 4..i * 4 + 4].copy_from_slice(&(l as u32).to_le_bytes()); }
        self.tail[n * 4..n * 4 + bank.len()].copy_from_slice(bank);
        // Only a changed tail marks the buffer (crate::asset_write: every get_mut is an upload).
        let same = buffers.get(&self.modes).and_then(|b| b.data.as_deref()).is_some_and(|d| d.len() >= self.tail.len() && d[d.len() - self.tail.len()..] == self.tail[..]);
        if same { return; }
        if let Some(mut buf) = buffers.get_mut(&self.modes) {
            if let Some(d) = buf.data.as_mut() {
                let at = d.len() - self.tail.len();
                d[at..].copy_from_slice(&self.tail);
            }
        }
    }

    pub fn new(level: &LoadedLevel, buffers: &mut Assets<ShaderBuffer>) -> Self {
        let n = level.tfrags.len().max(1);
        // MAIN_WORLD too: the system rewrites it every frame (same size, so Bevy writes the GPU buffer in place
        // and the material bind groups stay valid).
        // After the n modes: n empty nibble lists (0xffff) and an empty point-light bank (crate::world_lights).
        let mut tail = vec![0u8; n * 4 + crate::world_lights::BANK_BYTES];
        for i in 0..n { tail[i * 4..i * 4 + 4].copy_from_slice(&0xffffu32.to_le_bytes()); }
        let mut init = vec![0u8; n * 4];
        init.extend_from_slice(&tail);
        let modes = buffers.add(ShaderBuffer::new(&init, RenderAssetUsages::default()));
        let tfrags = level
            .tfrags
            .iter()
            .map(|t| TfragCullInfo {
                sphere: t.header.bsphere,
                corners: t.cube.map(|c| Vec3::new(c[0] as f32 * 64.0, c[1] as f32 * 64.0, c[2] as f32 * 64.0)),
            })
            .collect();
        // `draw_mode` only needs the header; keep header-only copies.
        let draw = level.tfrags.iter().map(|t| Tfrag { header: t.header, ..Default::default() }).collect();
        TfragLodState {
            modes,
            tfrags,
            draw,
            occl: level.occlusion.objects.tfrag.clone(),
            thresholds: level.tfrag_lod.block.lod_thresholds_raw(),
            block: level.tfrag_lod.block,
            force_lod0: std::env::var("RC_LOD").is_ok_and(|v| v.trim() == "0"),
            last_hist: [0; 8],
            tail,
        }
    }
}

pub struct TfragLodPlugin;

impl Plugin for TfragLodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, no_vsync)
            .add_systems(crate::level_switch::LevelUnload, crate::level_switch::remove::<TfragLodState>)
            .add_systems(PostUpdate, (update_tfrag_modes.after(crate::occlusion::OcclusionSet), update_lod_constants.after(crate::fog_state::FogSet)));
    }
}

/// `RC_NOVSYNC=1`: present without vsync so `fps:` measures headroom instead of the display rate.
fn no_vsync(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if !std::env::var("RC_NOVSYNC").is_ok_and(|v| v.trim() == "1") { return; }
    for mut w in &mut windows { w.present_mode = PresentMode::AutoNoVsync; }
}

/// VU CLIP judgement of one vertex against |w| (bits: +x, −x, +y, −y, +z, −z).
fn clip_flags(c: Vec4) -> u32 {
    let w = c.w.abs();
    (c.x > w) as u32 | ((c.x < -w) as u32) << 1 | ((c.y > w) as u32) << 2 | ((c.y < -w) as u32) << 3
        | ((c.z > w) as u32) << 4 | ((c.z < -w) as u32) << 5
}

/// `TfragProc`'s decision for one tfrag. `cam` = camera position × 1024 (vf28), `rows` = the view
/// matrix 0x186f40 as the three camera axes (x = −left, y = −up, z = forward) in game coordinates.
///
/// Boot 0x2341dc..0x2342d0 (sphere) and 0x2345c0..0x234790 (box), in f32:
/// * `x, y, z` = camera-space sphere centre, `far = z + r`, `near = z − r`;
/// * cull if `512000 − near < 0` (0x160ec0, set by the level init 0x1e9b10) or `n − far ≥ 0`, or the sphere is wholly
///   outside a side plane: `tan·z − |x| + r·k < 0` (x and y; `tan` = 0x18cdb0/4, `k` = 0x18cee0/4 = 1/cos(atan tan));
/// * the sphere is wholly inside when `tan·z − |x| − r·k ≥ 0` (x and y), `far < f` and `near ≥ n`
///   (vf27 = 0x18cef0 = (f, n)): then the LOD rule (`Tfrag::draw_mode`);
/// * otherwise the 8 box corners are transformed by 0x187040 (the view frustum as a clip volume, w = z/n) and
///   CLIP-tested twice, unscaled and with xy × 0.25 (0x160e60, i.e. the 4× guard band): all corners outside
///   one unscaled plane = cull; any corner outside the scaled volume = clipping path (VU1 0x107028, LOD 0, no
///   morph); else the LOD rule.
///
/// The occlusion test (hdr 0x3a/0x3b against the mask copy at 0x70003b00, level01 0x2a80b8..0x2a80e4) runs
/// before this, in `update_tfrag_modes`; an occluded tfrag never reaches these tests.
fn tfrag_proc(info: &TfragCullInfo, t: &Tfrag, cam: Vec3, rows: [Vec3; 3], thresholds: [i32; 3], tans: (f32, f32)) -> u32 {
    let (tx, ty) = tans;
    let (kx, ky) = ((1.0 + tx * tx).sqrt(), (1.0 + ty * ty).sqrt());
    let [sx, sy, sz, r] = info.sphere;
    let d = Vec3::new(sx, sy, sz) - cam;
    let p = Vec3::new(rows[0].dot(d), rows[1].dot(d), rows[2].dot(d));
    let (far, near) = (r + p.z, -r + p.z);
    if 512000.0 - near < 0.0 || NEAR - far >= 0.0 { return MODE_CULLED; }
    let (ax, ay) = (p.x.abs(), p.y.abs());
    if tx * p.z - (ax - r * kx) < 0.0 || ty * p.z - (ay - r * ky) < 0.0 { return MODE_CULLED; }
    let inside = tx * p.z - (ax + r * kx) >= 0.0 && ty * p.z - (ay + r * ky) >= 0.0 && far - FAR < 0.0 && near - NEAR >= 0.0;
    if !inside {
        let (mut all_out, mut any_guard) = (0x3fu32, 0u32);
        let (a, b) = ((FAR + NEAR) / (NEAR * (FAR - NEAR)), (NEAR * -2.0 * FAR) / (NEAR * (FAR - NEAR)));
        for c in &info.corners {
            let d = *c - cam;
            let v = Vec3::new(rows[0].dot(d), rows[1].dot(d), rows[2].dot(d));
            let clip = Vec4::new(v.x * (1.0 / (tx * NEAR)), v.y * (1.0 / (ty * NEAR)), v.z * a + b, v.z * (1.0 / NEAR));
            all_out &= clip_flags(clip);
            any_guard |= clip_flags(Vec4::new(clip.x * 0.25, clip.y * 0.25, clip.z, clip.w));
        }
        if all_out != 0 { return MODE_CULLED; }
        if any_guard != 0 { return MODE_CLIP; }
    }
    t.draw_mode(p.z, thresholds).mscal() as u32
}

fn update_tfrag_modes(
    state: Option<ResMut<TfragLodState>>,
    occl: Option<ResMut<crate::occlusion::OcclusionFrame>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    cams: Query<(&Transform, Option<&Projection>), With<Camera3d>>,
    time: Res<Time>,
    mut last_print: Local<f32>,
) {
    let (Some(mut state), Some((cam, proj))) = (state, cams.iter().next()) else { return };
    // `UpdateViewContext`'s planes (0x18cdb0 / 0x18cee0) from the view's tangent: a flown ship widens it.
    let tans = game_camera::projection_tans(proj);
    let (eye, [fwd, left, up]) = (game_camera::game_eye(cam), game_camera::game_rows(cam));
    let rows = [-left, -up, fwd];
    let cam_raw = eye * UNITS;
    let mut hist = [0usize; 8];
    let mut occluded = 0;
    let mask = occl.as_ref().map(|o| o.mask);
    let modes: Vec<u32> = state
        .tfrags
        .iter()
        .zip(&state.draw)
        .zip(&state.occl)
        .map(|((info, t), bits)| {
            // TfragProc: `mask[hdr 0x3b] & hdr 0x3a == 0 → next`, before the sphere and box tests.
            if mask.as_ref().is_some_and(|m| !bits.visible(m)) {
                occluded += 1;
                hist[0] += 1;
                return MODE_CULLED;
            }
            let mut m = tfrag_proc(info, t, cam_raw, rows, state.thresholds, tans);
            if state.force_lod0 && m != MODE_CULLED { m = 0x14; }
            hist[match m { 0 => 0, 2 => 1, 6 => 2, 8 => 3, 0xa => 4, 0xe => 5, 0x10 => 6, _ => 7 }] += 1;
            m
        })
        .collect();
    let mut d = bytemuck_u32(&modes);
    if d.is_empty() { d.resize(4, 0); }
    d.extend_from_slice(&state.tail);
    crate::asset_write::set_buffer(&mut buffers, &state.modes, &d);
    if let Some(mut o) = occl {
        o.tfrags = crate::occlusion::CullCounts { occluded, culled: hist[0] - occluded, drawn: modes.len() - hist[0] };
    }
    if hist != state.last_hist && time.elapsed_secs() - *last_print >= 1.0 {
        *last_print = time.elapsed_secs();
        state.last_hist = hist;
        println!("tfrag modes: culled {} ({occluded} occluded), clip {}, LOD2 {}, 0x8 {}, 0xa {}, 0xe {}, 0x10 {}, 0x14 {}", hist[0], hist[1], hist[2], hist[3], hist[4], hist[5], hist[6], hist[7]);
    }
}

fn bytemuck_u32(v: &[u32]) -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() }

#[cfg(test)]
mod tests {
    use super::*;

    const NOVALIS_FOG: LevelFog =
        LevelFog { color: [105, 127, 180], near_dist: 0.0, far_dist: 245760.0, near_intensity: 255.0, far_intensity: 102.0 };

    /// The VU weight t = clamp(w·qw666.xy + qw668.xy) is (u/2, 1 − u) with u = (depth − D1) / (D0 − D1), and the
    /// collapse threshold qw666.w is D0 in w units (likewise qw667/669 with D2..D1).
    #[test]
    fn vu_weights_match_world_depth_rule() {
        let block = TfragBlockHeader { table_offset: 0x10, tfrag_count: 1, unknown_8: 15.0, unknown_c: 0 };
        let k = TfragLodUniform::new(&block, &NOVALIS_FOG);
        let slope = k.misc.x;
        let t = |depth_units: f32, s: Vec4, i: Vec4| {
            let w = depth_units * 1024.0 * slope;
            (Vec2::new(w * s.x + i.x, w * s.y + i.y)).clamp(Vec2::ZERO, Vec2::new(0.5, 1.0))
        };
        for (d, u01, u0) in [(20.0, 0.0, 0.0), (30.0, 0.0, 0.0), (45.0, 0.0, 0.5), (60.0, 0.0, 1.0), (75.0, 0.5, 1.0), (90.0, 1.0, 1.0), (120.0, 1.0, 1.0)] {
            let a = t(d, k.k666, k.k668);
            let b = t(d, k.k667, k.k669);
            assert!((a - Vec2::new(u01 / 2.0, 1.0 - u01)).abs().max_element() < 1e-4, "LOD-01 at {d}: {a}");
            assert!((b - Vec2::new(u0 / 2.0, 1.0 - u0)).abs().max_element() < 1e-4, "LOD-0 at {d}: {b}");
        }
        // w decreases with depth, so "w < threshold" = "deeper than D".
        assert!((k.k666.w / (1024.0 * slope) - 90.0).abs() < 1e-3 && (k.k667.w / (1024.0 * slope) - 60.0).abs() < 1e-3);
    }

    #[test]
    fn tfrag_proc_culls_and_selects() {
        let th = TfragBlockHeader { table_offset: 0, tfrag_count: 1, unknown_8: 15.0, unknown_c: 0 }.lod_thresholds_raw();
        let rows = [Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0), Vec3::X]; // camera looks along +X
        let at = |x: f32, y: f32, r: f32| {
            let c = Vec3::new(x, y, 0.0) * 1024.0;
            let corners = std::array::from_fn(|i| c + Vec3::new(
                if i & 1 == 0 { -r } else { r }, if i & 2 == 0 { -r } else { r }, if i & 4 == 0 { -r } else { r }) * 1024.0);
            TfragCullInfo { sphere: [c.x, c.y, c.z, r * 1024.0], corners }
        };
        let m = |info: TfragCullInfo| {
            let mut t = Tfrag::default();
            t.header.bsphere = info.sphere;
            tfrag_proc(&info, &t, Vec3::ZERO, rows, th, crate::game_camera::default_tans())
        };
        assert_eq!(m(at(20.0, 0.0, 4.0)), 0x14);   // far 24 < D2 = 30
        assert_eq!(m(at(40.0, 0.0, 4.0)), 0x10);
        assert_eq!(m(at(58.0, 0.0, 4.0)), 0xe);    // straddles D1 = 60
        assert_eq!(m(at(70.0, 0.0, 4.0)), 0xa);
        assert_eq!(m(at(88.0, 0.0, 4.0)), 8);      // straddles D0 = 90
        assert_eq!(m(at(120.0, 0.0, 4.0)), 6);
        assert_eq!(m(at(-40.0, 0.0, 4.0)), MODE_CULLED); // behind
        assert_eq!(m(at(600.0, 0.0, 4.0)), MODE_CULLED); // near > 500 units
        assert_eq!(m(at(40.0, 200.0, 4.0)), MODE_CULLED); // far outside the side plane
        assert_eq!(m(at(2.0, 0.0, 4.0)), MODE_CLIP);  // crosses the near plane
    }
}
