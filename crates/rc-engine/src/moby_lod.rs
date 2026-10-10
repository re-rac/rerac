//! MobyProc's per-frame decisions for one moby, replayed on the CPU: the draw-distance / near / frustum
//! sphere culls, the LOD pick, the distance alpha fade and the metal ("shine") pass gate, plus the shine
//! pass's sphere-map basis E. Boot `fun_00211808` (level01 `MobyProc` 0x26a7a0); addresses below are boot
//! addresses. Notes: docs/plan/moby_skinning_lighting.md §9, docs/plan/moby_render_notes.md §7.
//!
//! Per moby (0x211b60..0x211ea0), with v = camera-space bounding-sphere centre and r its radius, both in
//! integer units (game units × 1024), after the occlusion test (crate::moby_render::update_moby_occlusion):
//! * dd = min(moby+0x32 (s16), [`DRAW_DISTANCE_CAP`]) (`pminw` with 0x15ff30); culled when
//!   `(itof(dd << 10) − r) − (v.z − r) < 0` (beyond the draw distance; the radius cancels), when
//!   `n − (v.z + r) ≥ 0` (n = 32, wholly in front of the near plane), or when
//!   `tan·v.z − (|v.x|, |v.y|) − r·k) < 0` (wholly outside a side plane; vf20 = 0x18cdb0, vf22 = 0x18cee0).
//! * fade = min(ftoi0((dd·1024 − r) − (v.z − r)) >> 7, 0x80): the last 16 units of the draw distance fade
//!   linearly. Ambient α lane (job +0xb0, the vertex alpha) = (fade · moby+0x23) >> 7 (0x211cdc..0x211d60).
//!   While fade < 0x80, mode bit 3 is set and the moby's packets are drawn with TEST_1 = 0x5360b + 0x80 −
//!   0x600 = **0x5308b** (AREF 0x08) instead of 0x5360b (0x21242c), restored after the last packet (0x212488).
//! * LOD (0x211e18..0x211e9c): low when `clamp(ftoi0(v.z − r), 0, 0x30000) > lod_trans << 10`, lod_trans =
//!   moby+0x72 = class byte 0xe (`InitMobyInstance`). No hysteresis. The clamp makes lod_trans ≥ 192
//!   (0xff on every class without a low LOD) mean "never". High: packets 0..class[4], job joint count
//!   class[8]; low: packets class[4]..class[4]+class[5], joint count **class[9]**, so `MobyAnimEval` fills
//!   the first class[9] palette slots only (joint count 0: the identity at slot 0), and the low-LOD packets
//!   only reference those slots (checked on the disc by the test below).
//! * Shine (0x212968): when moby+0x73 ≠ 0 (0x18 for a class with metal packets), m = (moby+0x73 << 10) −
//!   ftoi0(v.z − r) (not clamped); m ≤ 0 → no metal pass; else shine alpha = min(m >> 7, 0x80): full within
//!   8 units, fading out at 24 units.
//! * Glow list (level01 0x26b890..0x26b8f8, entered from 0x26b2a0 when moby+0x34 & 0x10): for the drawn LOD's packets
//!   from index class[0xa] (high) / class[0xb] (low) to the end of that list, MobyProc appends a 16-byte record
//!   {moby+0x90 with byte 3 replaced by the vertex alpha (job +0xbc), the packet entries, the job's chain slots, the
//!   packet count} to SPR 0x3400..0x3800, copied to 0x1ac680 (byte length gp−0x6c08) at the end (0x26b4c8).
//!   `DrawMobysCleanUp` (0x264d68) then runs `fun_002116b8` (0x26a650) after the skin/light pass: every transfer
//!   vertex colour of those packets (packet byte 0xf of them) is overwritten with that word. So a glow packet is
//!   drawn unlit, in the moby's glow colour, at the moby's vertex alpha, with the moby's own GS state.
//!   `InitMobyInstance` (0x263488) sets mode 0x10 and +0x90 = class +0x40 when the class's glow word is non-zero;
//!   updates may rewrite +0x90 (the vendor's pulse, the floor switch). Port: [`glow_word`], moby.wgsl.
//!
//! The bounding sphere is moby+0x00 as `fun_0020def8` builds it: the current sequence's sphere (A/B lerped
//! by t while blending), times the scale moby+0x2c on all four lanes, rotated by the rows and added to the
//! position × 1024 (0x20df6c..0x20df90). The camera is the Bevy camera (crate::game_camera: `game_eye`,
//! `game_rows`), so the tests use the port's float camera, not the game's matrix bits: thresholds agree to
//! float noise.

use crate::game_camera::{NEAR, UNITS};
use bevy::prelude::*;
use rc_formats::moby_anim::{AnimState, MobyAnimClass};

/// `0x15ff30` (gp−0x6cd0), `pminw`'d with moby+0x32; the level init `fun_001e9b10` sets it to 500.
pub const DRAW_DISTANCE_CAP: i32 = 500;
/// moby+0x73 of a class with metal packets (`InitMobyInstance` 0x20c5f0: class byte 6 ≠ 0 → 0x18).
pub const SHINE_DISTANCE: u8 = 0x18;
/// `lui a0, 0x3` (0x211e1c): the LOD depth is clamped to [0, 0x30000] before the compare.
pub const LOD_DEPTH_MAX: i32 = 0x30000;
/// TEST_1 of a fading moby: `0x5360b + (t6 << 4) − 0x600` with t6 = 8 (0x212428..0x212434).
pub const TEST_1_FADE: u64 = 0x5360b + (8 << 4) - 0x600;
/// AREF of that word (0x08).
pub const AREF_FADE: u8 = crate::gs_state::Test1::z_write_aref(TEST_1_FADE);

/// What one moby needs from its runtime struct.
#[derive(Clone, Copy, Debug)]
pub struct ProcInput {
    /// moby+0x10, game units.
    pub position: [f32; 3],
    /// moby+0xc0..0xe0 (row i = image of model axis i).
    pub rows: [[f32; 3]; 3],
    /// moby+0x2c.
    pub scale: f32,
    /// moby+0x32 (the gameplay record's draw distance as s16), before the cap.
    pub draw_distance: i32,
    /// moby+0x72.
    pub lod_trans: u8,
    /// moby+0x73 (0 = no metal pass).
    pub shine_distance: u8,
    /// moby+0x23.
    pub alpha: u8,
}

/// MobyProc's result for a drawn moby.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcPick {
    pub low_lod: bool,
    /// Vertex alpha (the ambient α lane).
    pub alpha: u8,
    /// Distance fade < 0x80: TEST_1 AREF 0x08 for the moby's packets.
    pub fading: bool,
    /// Shine alpha of the metal pass; 0 = no metal pass.
    pub shine: u8,
}

/// Why a moby is not drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cull {
    DrawDistance,
    Near,
    Frustum,
}

/// `ftoi0`: truncation toward zero, saturating like the VU.
fn ftoi0(x: f32) -> i32 { x.clamp(-2147483648.0, 2147483520.0) as i32 }

/// The sequence sphere `fun_0020def8` starts from (packed model units): key A's sequence, or while
/// blending (seq A ≠ seq B) `(B·t + A) − A·t` (`vmulax / vmaddaw / vmsubx`). A snapshot key A (0xff, the
/// table at 0x1b2c00) is not ported: key B's sphere is used. `fallback` = the class header sphere, for a
/// class whose sequence is missing.
pub fn seq_sphere(class: &MobyAnimClass, st: &AnimState, fallback: [f32; 4]) -> [f32; 4] {
    let s = |q: u8| class.sequence(q).map(|s| s.header.sphere).unwrap_or(fallback);
    if st.seq_a == st.seq_b || st.seq_a == 0xff { return s(st.seq_b); }
    let (a, b, t) = (s(st.seq_a), s(st.seq_b), st.t);
    std::array::from_fn(|k| (b[k] * t + a[k]) - a[k] * t)
}

/// moby+0x00: world bounding sphere in integer units (centre × 1024, radius × 1024).
pub fn world_sphere(inp: &ProcInput, seq: [f32; 4]) -> [f32; 4] {
    let c = seq.map(|v| v * inp.scale);
    let [r0, r1, r2] = inp.rows;
    let p = inp.position.map(|v| v * UNITS);
    let centre: [f32; 3] = std::array::from_fn(|k| ((r0[k] * c[0] + r1[k] * c[1]) + r2[k] * c[2]) + p[k]);
    [centre[0], centre[1], centre[2], c[3]]
}

/// Camera rows as the VU uses them (vf25..vf27 = 0x186f40): camera space x right, y down, z forward.
pub fn camera_rows(fwd: Vec3, left: Vec3, up: Vec3) -> [Vec3; 3] { [-left, -up, fwd] }

/// `vmulax / vmadday / vmaddz` with the camera rows: (right·v, down·v, fwd·v).
fn to_camera(rows: &[Vec3; 3], v: [f32; 3]) -> [f32; 3] { rows.map(|r| (r.x * v[0] + r.y * v[1]) + r.z * v[2]) }

/// Camera-space centre of an integer-unit sphere for the eye (game units).
pub fn view_centre(sphere: [f32; 4], eye: Vec3, rows: &[Vec3; 3]) -> [f32; 3] {
    let cam = [eye.x * UNITS, eye.y * UNITS, eye.z * UNITS];
    to_camera(rows, [sphere[0] - cam[0], sphere[1] - cam[1], sphere[2] - cam[2]])
}

/// The per-moby tests of MobyProc after the occlusion test (module doc); `v` = camera-space centre, `r` =
/// radius, both integer units; the game's 4:3 view.
#[cfg(test)]
pub fn moby_proc(v: [f32; 3], r: f32, inp: &ProcInput) -> Result<ProcPick, Cull> { moby_proc_view(v, r, inp, crate::game_camera::default_tans()) }

/// MobyProc with the view's half-angle tangents `tans` (x, y; `UpdateViewContext` writes the frustum planes vf20 /
/// vf22 from them: a flown ship's wider view, or the 16:9 frame, culls less at the sides).
pub fn moby_proc_view(v: [f32; 3], r: f32, inp: &ProcInput, tans: (f32, f32)) -> Result<ProcPick, Cull> {
    // The cap 0x15fff0: 500, 0x90 in the Visibomb's view (crate::visibomb_view::SHORT_FAR).
    let cap = if crate::visibomb_view::SHORT_FAR.load(std::sync::atomic::Ordering::Relaxed) { crate::visibomb_view::SHORT_MOBY_CAP } else { DRAW_DISTANCE_CAP };
    let dd = inp.draw_distance.min(cap);
    // vf19.y = itof0(dd << 10) − r; vf1.xy = (vz + r, vz − r); vf3 = vf19 − vf1 (vf19.x = n).
    let far = ((dd.wrapping_shl(10)) as f32 - r) - (v[2] - r);
    let near = NEAR - (v[2] + r);
    if far.is_sign_negative() { return Err(Cull::DrawDistance); }
    if !near.is_sign_negative() { return Err(Cull::Near); }
    let (tx, ty) = tans;
    let (kx, ky) = ((1.0 + tx * tx).sqrt(), (1.0 + ty * ty).sqrt());
    if (tx * v[2] - (v[0].abs() - r * kx)).is_sign_negative() || (ty * v[2] - (v[1].abs() - r * ky)).is_sign_negative() {
        return Err(Cull::Frustum);
    }
    let depth = ftoi0(v[2] - r);
    let low_lod = low_lod(depth, inp.lod_trans);
    let fade = ((ftoi0(far) as u32) >> 7).min(0x80);
    let alpha = ((fade * inp.alpha as u32) >> 7) as u8;
    Ok(ProcPick { low_lod, alpha, fading: fade < 0x80, shine: shine_alpha(depth, inp.shine_distance) })
}

/// The Detail distance option's factor on the LOD switch depth (crate::graphics; f32 bits, 1.0 = the game's): drawing
/// only — the port evaluates every joint whatever LOD is drawn, and no game code reads the pick.
static LOD_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x3f80_0000);

pub fn set_lod_scale(s: f32) { LOD_SCALE.store(s.to_bits(), std::sync::atomic::Ordering::Relaxed); }

/// The LOD pick: low past `lod_trans << 10` (× the Detail factor) of the clamped depth.
fn low_lod(depth: i32, lod_trans: u8) -> bool {
    let d = depth.clamp(0, LOD_DEPTH_MAX);
    let s = f32::from_bits(LOD_SCALE.load(std::sync::atomic::Ordering::Relaxed));
    if s == 1.0 { d - ((lod_trans as i32) << 10) > 0 } else { d as f32 > ((lod_trans as i32) << 10) as f32 * s }
}

/// `ftoi0(v.z − r)`: the depth the LOD and shine tests use.
pub fn sphere_depth(v: [f32; 3], r: f32) -> i32 { ftoi0(v[2] - r) }

/// The shine gate (0x212968): 0 = no metal pass, else min(((dist << 10) − depth) >> 7, 0x80).
pub fn shine_alpha(depth: i32, shine_distance: u8) -> u8 {
    if shine_distance == 0 { return 0; }
    let m = ((shine_distance as i32) << 10).wrapping_sub(depth);
    if m <= 0 { 0 } else { ((m as u32) >> 7).min(0x80) as u8 }
}

/// The shine pass's sphere-map basis E (MobyProc 0x212998..0x212ae8, stored at the head of the job's metal
/// record, loaded into VU0 vf22..vf24 by `fun_001ee650` 0x1ef730): with d = camera-space unit vector from
/// the eye to the sphere centre and m_i = camera-space moby row i,
/// `A(m) = (m.x, d.z·m.y − d.y·m.z, d.y·m.y + d.z·m.z)`, `B(u) = (d.z·u.x − d.x·u.z, u.y, d.x·u.x + d.z·u.z)`,
/// E_i = B(A(m_i)). VU0 104691 (0x132..0x147) then takes e = E_0·n'.x + E_1·n'.y + E_2·n'.z for the
/// skinned model-space normal n' and ST = (ftoi12(e.xy + 1)) >> 1 (EE `psraw 1`).
pub fn shine_basis(sphere: [f32; 4], eye: Vec3, cam: &[Vec3; 3], rows: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let c = [eye.x * UNITS, eye.y * UNITS, eye.z * UNITS];
    let d0 = [sphere[0] - c[0], sphere[1] - c[1], sphere[2] - c[2]];
    let len2 = (d0[0] * d0[0] + d0[1] * d0[1]) + d0[2] * d0[2];
    let q = 1.0 / len2.sqrt();
    let d = to_camera(cam, d0.map(|v| v * q));
    rows.map(|r| {
        let m = to_camera(cam, r);
        let a = [m[0], d[2] * m[1] + -d[1] * m[2], d[1] * m[1] + d[2] * m[2]];
        [d[2] * a[0] + -d[0] * a[2], a[1], d[0] * a[0] + d[2] * a[2]]
    })
}

/// `RC_MOBY_LOD=0`: no LOD, draw-distance / near / frustum culls or fade (every drawn moby high LOD, α 0x80).
pub fn lod_enabled() -> bool { !std::env::var("RC_MOBY_LOD").is_ok_and(|v| v.trim() == "0") }
/// `RC_MOBY_LOD_TINT=1`: low-LOD instances drawn red.
pub fn tint_enabled() -> bool { std::env::var("RC_MOBY_LOD_TINT").is_ok_and(|v| v.trim() == "1") }
/// `RC_MOBY_METAL=0`: no metal (shine) pass.
pub fn metal_enabled() -> bool { !std::env::var("RC_MOBY_METAL").is_ok_and(|v| v.trim() == "0") }

/// One `MobyLod` record of moby.wgsl / moby_metal.wgsl: misc (vertex alpha, flags, shine alpha, glow word), E rows.
pub const LOD_RECORD_SIZE: usize = 64;
/// `MobyLod.misc.y` bit: tint this instance (low LOD with `RC_MOBY_LOD_TINT=1`).
pub const FLAG_TINT: u32 = 1;
/// `MobyLod.misc.w` bit: the moby is on the glow list (mode 0x10); bits 0..23 are its glow RGB (moby+0x90).
pub const GLOW_ON: u32 = 1 << 24;

/// The `MobyLod` glow word of a moby with mode bits `mode` (+0x34) and glow colour `glow` (+0x90): [`GLOW_ON`] | RGB
/// when mode bit 0x10 is set (MobyProc 0x26b2a0), else 0 (its glow packets keep their lit colours). Byte 3 of +0x90
/// is not used: the glow pass replaces it with the vertex alpha (module doc).
pub fn glow_word(mode: u16, glow: u32) -> u32 { if mode & 0x10 != 0 && glow_enabled() { GLOW_ON | (glow & 0x00ff_ffff) } else { 0 } }

/// `RC_MOBY_GLOW=0`: no glow list (the glow packets keep their lit colours, as before it was ported).
pub fn glow_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| !std::env::var("RC_MOBY_GLOW").is_ok_and(|v| v.trim() == "0"))
}

/// The glow word of a fresh moby of a class with glow colour `class_glow` (class header +0x40): `InitMobyInstance`
/// sets mode 0x10 and +0x90 = that word when it is non-zero.
pub fn class_glow_word(class_glow: i32) -> u32 { if class_glow != 0 { glow_word(0x10, class_glow as u32) } else { 0 } }

pub fn write_lod_record(out: &mut [u8], alpha: u8, flags: u32, shine: u8, glow: u32, e: &[[f32; 3]; 3]) {
    let mut w = |i: usize, v: u32| out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    w(0, alpha as u32);
    w(1, flags);
    w(2, shine as u32);
    w(3, glow);
    for (r, row) in e.iter().enumerate() {
        for (k, v) in row.iter().enumerate() { w(4 + r * 4 + k, v.to_bits()); }
        w(4 + r * 4 + 3, 0);
    }
}

/// The record of a moby drawn as before this module (α 0x80, no metal pass).
pub fn default_lod_records(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n.max(1) * LOD_RECORD_SIZE];
    for rec in v.chunks_mut(LOD_RECORD_SIZE) { write_lod_record(rec, 0x80, 0, 0, 0, &[[0.0; 3]; 3]); }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The glow word: on with mode bit 0x10 only; RGB from +0x90, its byte 3 dropped (the vertex alpha replaces it);
    /// a class with a glow colour starts on, one without off.
    #[test]
    fn glow_word_of_mode_and_colour() {
        assert_eq!(glow_word(0x10, 0x8084_e643), GLOW_ON | 0x84_e643);
        assert_eq!(glow_word(0x210, 0x0020_8020), GLOW_ON | 0x20_8020);
        assert_eq!(glow_word(0x200, 0x8084_e643), 0);
        assert_eq!(class_glow_word(0x808c_8c8cu32 as i32), GLOW_ON | 0x8c_8c8c);
        assert_eq!(class_glow_word(0), 0);
        // Black is a colour: mode 0x10 with +0x90 = 0 draws the glow packets black.
        assert_eq!(glow_word(0x10, 0), GLOW_ON);
    }
    use rc_formats::moby::{LevelMobyClass, MobySubmesh};

    fn inp(dd: i32, lod: u8, shine: u8) -> ProcInput {
        ProcInput { position: [0.0; 3], rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], scale: 1.0, draw_distance: dd, lod_trans: lod, shine_distance: shine, alpha: 0x80 }
    }

    #[test]
    fn test_1_fade_word() {
        assert_eq!(TEST_1_FADE, 0x5308b);
        assert_eq!(AREF_FADE, 0x08);
    }

    #[test]
    fn draw_distance_fade_and_lod() {
        let k = 1024.0;
        let i = inp(64, 32, 0);
        // Well inside: full alpha, high LOD; the LOD compare uses v.z − r.
        assert_eq!(moby_proc([0.0, 0.0, 10.0 * k], 1.0 * k, &i), Ok(ProcPick { low_lod: false, alpha: 0x80, fading: false, shine: 0 }));
        assert!(!moby_proc([0.0, 0.0, 33.0 * k], 1.0 * k, &i).unwrap().low_lod); // v.z − r = 32 units: not > 32
        assert!(moby_proc([0.0, 0.0, 33.0 * k + 1.0], 1.0 * k, &i).unwrap().low_lod);
        // Fade over the last 16 units: 64 − 48 = 16 units = 16384 >> 7 = 0x80 (not fading), then linear.
        assert_eq!(moby_proc([0.0, 0.0, 48.0 * k], 0.0, &i).unwrap().alpha, 0x80);
        let p = moby_proc([0.0, 0.0, 56.0 * k], 0.0, &i).unwrap();
        assert_eq!((p.alpha, p.fading), (0x40, true));
        assert_eq!(moby_proc([0.0, 0.0, 64.0 * k], 0.0, &i).unwrap().alpha, 0);
        assert_eq!(moby_proc([0.0, 0.0, 64.0 * k + 1.0], 0.0, &i), Err(Cull::DrawDistance));
        // The cap: 300 → 500 units max.
        assert!(moby_proc([0.0, 0.0, 499.0 * k], 0.0, &inp(0x7fff, 0xff, 0)).is_ok());
        assert_eq!(moby_proc([0.0, 0.0, 501.0 * k], 0.0, &inp(0x7fff, 0xff, 0)), Err(Cull::DrawDistance));
        // lod_trans ≥ 192 never switches (depth clamp 0x30000).
        assert!(!moby_proc([0.0, 0.0, 499.0 * k], 0.0, &inp(500, 0xff, 0)).unwrap().low_lod);
        assert!(!moby_proc([0.0, 0.0, 499.0 * k], 0.0, &inp(500, 192, 0)).unwrap().low_lod);
        assert!(moby_proc([0.0, 0.0, 499.0 * k], 0.0, &inp(500, 191, 0)).unwrap().low_lod);
        // Near and side planes.
        assert_eq!(moby_proc([0.0, 0.0, 10.0], 20.0, &i), Err(Cull::Near));
        assert!(moby_proc([0.0, 0.0, -5.0 * k], 6.0 * k, &i).is_ok());
        assert_eq!(moby_proc([40.0 * k, 0.0, 50.0 * k], 1.0 * k, &i), Err(Cull::Frustum));
        assert_eq!(moby_proc([0.0, 30.0 * k, 50.0 * k], 1.0 * k, &i), Err(Cull::Frustum));
    }

    #[test]
    fn shine_alpha() {
        let k = 1024.0;
        let i = inp(64, 0xff, SHINE_DISTANCE);
        assert_eq!(moby_proc([0.0, 0.0, 5.0 * k], 1.0 * k, &i).unwrap().shine, 0x80);
        assert_eq!(moby_proc([0.0, 0.0, 9.0 * k], 1.0 * k, &i).unwrap().shine, 0x80); // 24 − 8 = 16 units
        assert_eq!(moby_proc([0.0, 0.0, 17.0 * k], 1.0 * k, &i).unwrap().shine, 0x40);
        assert_eq!(moby_proc([0.0, 0.0, 25.0 * k], 1.0 * k, &i).unwrap().shine, 0);
        assert_eq!(moby_proc([0.0, 0.0, 5.0 * k], 1.0 * k, &inp(64, 0xff, 0)).unwrap().shine, 0);
    }

    #[test]
    fn shine_basis_straight_ahead_is_the_camera_rotation() {
        // Camera at the origin looking down game +y, moby 10 units ahead: d = (0, 0, 1), A = B = identity,
        // E_i = camera-space rows.
        let cam = camera_rows(Vec3::Y, -Vec3::X, Vec3::Z);
        let rows = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let e = shine_basis([0.0, 10240.0, 0.0, 100.0], Vec3::ZERO, &cam, &rows);
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6);
        assert!(close(e[0], [1.0, 0.0, 0.0]) && close(e[1], [0.0, 0.0, 1.0]) && close(e[2], [0.0, -1.0, 0.0]), "{e:?}");
        // Off-axis: both steps use the same unrenormalised d, so the view direction does not land exactly on
        // +z: B(A(d)) = (d.z·d.x − d.x·(d.y² + d.z²), 0, d.x² + d.z·(d.y² + d.z²)) (camera space).
        let e = shine_basis([3000.0, 10240.0, -2000.0, 100.0], Vec3::ZERO, &cam, &rows);
        let w = Vec3::new(3000.0, 10240.0, -2000.0).normalize();
        let v: [f32; 3] = std::array::from_fn(|k| e[0][k] * w.x + e[1][k] * w.y + e[2][k] * w.z);
        let d = Vec3::new(w.x, -w.z, w.y);
        let s = d.y * d.y + d.z * d.z;
        assert!(close(v, [d.z * d.x - d.x * s, 0.0, d.x * d.x + d.z * s]), "{v:?}");
        assert!(v[2] < 0.99, "{v:?}");
    }

    fn level01() -> Option<(rc_formats::level::LevelCore, Vec<u8>, Vec<LevelMobyClass>)> {
        let root = crate::level_load::extracted_root();
        let (Ok(index), Ok(data)) = (crate::disc_source::level_file(&root, 1, "core_index.bin"), crate::disc_source::level_file(&root, 1, "core_data.bin")) else { return None };
        let data = rc_formats::wad::decompress(&data).unwrap();
        let core = rc_formats::level::parse_level_core(&index, data.len()).unwrap();
        let classes = rc_formats::moby::parse_level_mobys(&core, &data).unwrap();
        Some((core, data, classes))
    }

    fn max_palette_slot(list: &[MobySubmesh]) -> Option<u8> {
        list.iter().flat_map(|s| &s.vertices).map(|v| *v.skin.joints[..v.skin.count.max(1) as usize].iter().max().unwrap()).max()
    }

    /// The low-LOD joint mapping: `MobyAnimEval` runs with class[9] joints for the low LOD, so its packets
    /// may only use palette slots below class[9] (slot 0 = identity when class[9] = 0). Measured on every
    /// class of every level on the disc that has a low LOD. Needs the disc data; skipped without it.
    #[test]
    fn low_lod_uses_the_first_low_joint_count_slots() {
        let root = crate::level_load::extracted_root();
        let mut checked = 0;
        for lv in 0..19u32 {
            let (Ok(index), Ok(data)) = (crate::disc_source::level_file(&root, lv, "core_index.bin"), crate::disc_source::level_file(&root, lv, "core_data.bin")) else { continue };
            let data = rc_formats::wad::decompress(&data).unwrap();
            let core = rc_formats::level::parse_level_core(&index, data.len()).unwrap();
            let Ok(classes) = rc_formats::moby::parse_level_mobys(&core, &data) else { continue };
            for c in &classes {
                let h = &c.class.header;
                let Some(max) = max_palette_slot(&c.class.low_lod) else { continue };
                checked += 1;
                assert!((max as u32) < (h.low_lod_joint_count as u32).max(1), "level {lv} class {}: low LOD skins to slot {max}, low joint count {}", c.o_class, h.low_lod_joint_count);
            }
        }
        if checked == 0 { eprintln!("skipped: no level data"); } else { println!("{checked} low-LOD classes checked"); }
    }

    /// Novalis metal classes, their packets' textures (−2 chrome / −3 glass) and the TEX1/CLAMP words of their
    /// ad-gifs, for the notes.
    #[test]
    #[ignore = "survey: prints, asserts nothing; run with --ignored --nocapture"]
    fn novalis_metal_classes() {
        let Some((core, data, classes)) = level01() else { eprintln!("skipped: no level 01 data"); return };
        for c in classes.iter().filter(|c| !c.class.metal.is_empty()) {
            let mut tex = std::collections::BTreeMap::<i32, usize>::new();
            for s in &c.class.metal { for t in &s.triangles { *tex.entry(t.texture).or_default() += 1; } }
            let verts: usize = c.class.metal.iter().map(|s| s.vertices.len()).sum();
            let max_joint = max_palette_slot(&c.class.metal);
            let blk = core.blocks.iter().find(|b| b.name == format!("moby_class/{:04}", c.o_class)).unwrap();
            let blob = &data[blk.offset..blk.offset + blk.size];
            let mut words = std::collections::BTreeSet::new();
            for s in &c.class.metal {
                let e = &s.entry;
                let list = &blob[e.vif_list_offset as usize..e.vif_list_offset as usize + e.vif_list_size as usize * 16];
                let codes = rc_formats::vif::parse_vif(list).unwrap();
                if let Some(ad) = codes.iter().filter(|c| c.is_unpack()).nth(1) {
                    for b in 0..ad.count() as usize / 4 {
                        let w = |q: usize, k: usize| u32::from_le_bytes(ad.data[b * 0x40 + q * 16 + k * 4..][..4].try_into().unwrap());
                        words.insert((w(0, 0), w(0, 1), w(1, 0), w(2, 0)));
                    }
                }
            }
            println!(
                "class {:4}: {} metal packets, {verts} vertices, triangles by texture {tex:?}, max joint {max_joint:?} (joint count {}), ad-gif (TEX1 lo, TEX1 hi, CLAMP lo, TEX0 lo) {words:x?}",
                c.o_class, c.class.metal.len(), c.class.header.joint_count
            );
        }
        let h = &core.header;
        println!("env maps: chrome tex 0x{:x} pal 0x{:x}, glass tex 0x{:x} pal 0x{:x}", h.chrome_map_texture, h.chrome_map_palette, h.glass_map_texture, h.glass_map_palette);
    }
}
