//! The game's own camera projection and fog, as the EE builds them for the VU1 tfrag program.
//!
//! Source: `InitViewContext` (boot 0x1f2c60), `UpdateViewContext` (0x1f2d98), the camera matrix
//! builder `fun_001f2260`, `DrawTfrag` (0x2333a8) and `UpdateFog` (0x1f2588); the derivation and every
//! constant is in docs/plan/game_camera_fog.md. The fly camera still moves the Bevy transform; only
//! the view → clip mapping, the depth mapping and the fog are the game's.
//!
//! Camera space (game): x right, y down (GS screen Y), z forward, in integer units (game units × 1024,
//! the unit tfrag positions are stored in). Bevy view space is x right, y up, -z forward in game units,
//! so `x_game = x_view`, `y_game = -y_view`, `z_game = -1024 z_view`.

use bevy::camera::{CameraProjection, SubCameraView};
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

/// NTSC GS draw buffer (`SetupFS_AA_buffer(0x200, 0x1a0, ...)` in `SetPalMode`, render_pipeline.md §4).
pub const SCREEN_W: f32 = 512.0;
pub const SCREEN_H: f32 = 416.0;
/// Position units per game unit (`DrawTfrag` translates by `-Camera.pos × 1024`).
pub const UNITS: f32 = 1024.0;
/// `0x18cda0` near = 32.0 and `0x18cda4` far = 745472.0 (integer units), set by `InitViewContext`.
pub const NEAR: f32 = 32.0;
pub const FAR: f32 = 745472.0;
/// `0x18cdb0` = 0x3f2147ae (0.63): tangent of the horizontal half field of view.
pub const TAN_HALF_FOV_X: f32 = 0.63;
/// `UpdateViewContext`: vertical tangent = horizontal × 0.775 on NTSC (0x3f466666), × 0.756 on PAL.
pub const NTSC_Y_RATIO: f32 = 0.775;
/// Z lane of the projection (`fVar5` = 0xcafffbe0) and the post-divide Z offset (qw661.z = 0x4afffc20).
pub const Z_SCALE: f32 = -8388080.0;
pub const Z_OFFSET: f32 = 8388112.0;
/// qw661.xy: GS primitive coordinates are centred on 2048 (XYOFFSET = (2048 − 256, 2048 − 208)).
pub const XY_CENTRE: f32 = 2048.0;
/// qw656.w: added to the fog lane to set ADC (bit 15 after `ftoi4`) for guard-band-clipped triangles.
pub const ADC_ADD: f32 = 3072.0;

/// Which camera moves the main view's transform; both feed the same projection and fog.
/// Absent (`RC_PLAY=0`) = the fly camera only. `Play` = the game's follow camera (crate::play_camera; the fly
/// camera is frozen), `Fly` = the debug fly camera while the game keeps ticking. Tab switches (crate::gameplay).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraSource {
    Fly,
    Play,
}

/// Letterboxes the camera to 512:416 and, on `P`, prints the camera as the game would see it.
pub struct GameCameraPlugin {
    pub fog: LevelFog,
}

/// The fog every renderer draws with this frame: the view-context copy `UpdateFog` makes (boot 0x1f2588,
/// level01 0x218d70) of the level fog globals, or of the alternate set while underwater, and the
/// particle far it sets. Starts as the level settings (level init calls `UpdateFog`); `crate::fog_state`
/// rewrites it every frame (zones, underwater) and pushes it into every material's fog uniform, the tie
/// fog values, the tfrag LOD constants (`SetTfragDists`) and the particle cull.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct GameFog {
    /// View-context fog (0x16d0d8..0x16d0f8).
    pub fog: LevelFog,
    /// The material uniform built from `fog` (`TfragFog::new`, honours `RC_FOG=0`).
    pub uniform: TfragFog,
    /// `0x16017c`: global particle far, 20.12 fixed point (500 units, 64 underwater).
    pub particle_far12: i32,
}

impl GameFog {
    pub fn new(fog: LevelFog, particle_far12: i32) -> Self { GameFog { fog, uniform: TfragFog::new(&fog), particle_far12 } }
}

/// The frame's fog ([`GameFog::uniform`]) in one storage buffer every world material binds (tfrags, ties, shrubs,
/// shrub billboards, mobys and their metal pass: binding 2): a fog change rewrites these 32 bytes instead of every
/// material (a material change makes Bevy rebuild its bind group, thousands of them per frame during a fog
/// zone's cross-fade).
static FOG_BUFFER: std::sync::OnceLock<Handle<bevy::render::storage::ShaderBuffer>> = std::sync::OnceLock::new();

/// The shared fog buffer ([`FOG_BUFFER`]) for a new material.
pub fn fog_buffer() -> Handle<bevy::render::storage::ShaderBuffer> { FOG_BUFFER.get().cloned().expect("GameCameraPlugin creates the fog buffer") }

/// The fog (colour, params), then the graphics options the world shaders read (crate::graphics; every world material
/// binds this buffer): `tex_mode.x` the texture option for `sample_world` (0 Original, 1 Smooth, 2 Sharp), `.y` the
/// Detail distance factor (1 = the game's; the shrub and billboard fade distance).
fn fog_bytes(f: &TfragFog, g: &crate::graphics::GraphicsSettings) -> Vec<u8> {
    use crate::graphics::GfxOption;
    let filter = [g.textures.index() as f32, g.detail.lod_scale(), 0.0, 0.0];
    f.color.to_array().iter().chain(&f.params.to_array()).chain(&filter).flat_map(|v| v.to_le_bytes()).collect()
}

/// Rewrites the shared fog buffer when the frame's fog or the texture option changes.
fn upload_fog(fog: Res<GameFog>, gfx: Res<crate::graphics::GraphicsSettings>, mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>) {
    if !fog.is_changed() && !gfx.is_changed() { return; }
    if let Some(h) = FOG_BUFFER.get() { crate::asset_write::set_buffer(&mut buffers, h, &fog_bytes(&fog.uniform, &gfx)); }
}

impl Plugin for GameCameraPlugin {
    fn build(&self, app: &mut App) {
        let (k656, k661) = self.fog.vu_constants();
        println!(
            "fog: FOGCOL {:?}, F {} at depth {} .. F {} at depth {} (integer units); VU qw656 {k656:?}, qw661 {k661:?}{}",
            self.fog.color, self.fog.near_intensity, self.fog.near_dist, self.fog.far_intensity, self.fog.far_dist,
            if std::env::var("RC_FOG").is_ok_and(|v| v.trim() == "0") { " (disabled by RC_FOG=0)" } else { "" }
        );
        let fog = GameFog::new(self.fog, rc_game::fog_zones::PARTICLE_FAR12);
        let buf = bevy::render::storage::ShaderBuffer::new(&fog_bytes(&fog.uniform, &crate::graphics::GraphicsSettings::default()), bevy::asset::RenderAssetUsages::default());
        let h = app.world_mut().resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>().add(buf);
        let _ = FOG_BUFFER.set(h);
        app.insert_resource(fog).add_systems(Update, print_game_camera).add_systems(PostUpdate, upload_fog.before(bevy::asset::AssetEventSystems));
    }
}

/// `P`: the current view as an `RC_CAM` value plus the matrix `DrawTfrag` would upload to VU qw 5..8.
/// Main camera only: the sky camera copies its transform, so it would print the same line again.
fn print_game_camera(
    keys: Res<ButtonInput<KeyCode>>,
    fog: Res<GameFog>,
    cams: Query<&Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>,
) {
    if !keys.just_pressed(KeyCode::KeyP) { return; }
    for t in &cams {
        let (eye, rows) = (game_eye(t), game_rows(t));
        let target = eye + rows[0] * 10.0;
        println!(
            "RC_CAM={:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
            eye.x, eye.y, eye.z, target.x, target.y, target.z
        );
        for (k, row) in vu_tfrag_matrix(&GameProjection::default(), &fog.fog, eye, rows).iter().enumerate() {
            println!("  VU qw{}: {:?}", 5 + k, row);
        }
    }
}

/// The game's perspective: `pixel = 2048 + (256 / tan_x)·x/z`, `2048 + (208 / tan_y)·y/z`, and the
/// 24-bit GS Z = `Z_OFFSET + Z_SCALE·((f+n)/(f−n) − 2fn/((f−n)·z))` (near → 0xfffff8, far → 32, ZTST GEQUAL),
/// expressed as a Bevy reverse-Z clip matrix (depth = Z / 2^24, larger = nearer, same ordering).
#[derive(Clone, Copy, Debug)]
pub struct GameProjection {
    pub tan_x: f32,
    pub tan_y: f32,
    /// Integer units.
    pub near: f32,
    pub far: f32,
}

/// The half-angle tangents (x, y) of a camera's projection (the game's 0.63, 0.63·0.775 for any other projection):
/// `UpdateViewContext`'s frustum planes. In 16:9 x carries the Hor+ factor and y stays the game's (crate::display),
/// so every cull and LOD test takes both from here rather than deriving y from x.
pub fn projection_tans(p: Option<&Projection>) -> (f32, f32) {
    let d = GameProjection::default();
    match p {
        Some(Projection::Custom(c)) => c.get::<GameProjection>().map_or((d.tan_x, d.tan_y), |g| (g.tan_x, g.tan_y)),
        _ => (d.tan_x, d.tan_y),
    }
}

/// The game's default tangents (x, y).
#[cfg(test)]
pub fn default_tans() -> (f32, f32) {
    let d = GameProjection::default();
    (d.tan_x, d.tan_y)
}

impl Default for GameProjection {
    fn default() -> Self {
        GameProjection { tan_x: TAN_HALF_FOV_X, tan_y: TAN_HALF_FOV_X * NTSC_Y_RATIO, near: NEAR, far: FAR }
    }
}

impl GameProjection {
    /// GS Z as `a + b / d` for Bevy view depth `d` in game units, from the EE's matrix entries
    /// (`0x18cde8` = (f+n)/(n(f−n))·Zs, `0x18cdf8` = −2nf/(n(f−n))·Zs) and Q = n / z.
    /// f64 because `Z_OFFSET + n·A` cancels to about −688 (the VU does it in f32 per vertex).
    fn z_affine(&self) -> (f64, f64) {
        let (n, f, zs) = (self.near as f64, self.far as f64, Z_SCALE as f64);
        let a_m = (f + n) / (n * (f - n)) * zs;
        let c_m = (n * -2.0 * f) / (n * (f - n)) * zs;
        (n * a_m + Z_OFFSET as f64, n * c_m / UNITS as f64)
    }

    /// GS primitive coordinates (X, Y in pixels around 2048, Z as the 24-bit value) of a point given in
    /// game camera space (x right, y down, z forward) in integer units. Mirrors `fun_001f2070`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn gs_xyz(&self, cam: Vec3) -> Vec3 {
        let q = self.near / cam.z;
        let (a, b) = self.z_affine();
        Vec3::new(
            XY_CENTRE + cam.x * (SCREEN_W * 0.5) / (self.tan_x * self.near) * q,
            XY_CENTRE + cam.y * (SCREEN_H * 0.5) / (self.tan_y * self.near) * q,
            (a + b * UNITS as f64 / cam.z as f64) as f32,
        )
    }
}

impl CameraProjection for GameProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        // clip = (x / tan_x, y / tan_y, (a·d + b) / 2^24, d) with d = −z_view (game units).
        let (a, b) = self.z_affine();
        let s = 1.0 / 16_777_216.0;
        Mat4::from_cols(
            Vec4::new(1.0 / self.tan_x, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0 / self.tan_y, 0.0, 0.0),
            Vec4::new(0.0, 0.0, (-a * s) as f32, -1.0),
            Vec4::new(0.0, 0.0, (b * s) as f32, 0.0),
        )
    }

    fn get_clip_from_view_for_sub(&self, sub: &SubCameraView) -> Mat4 {
        let full = sub.full_size.as_vec2();
        let (lo, hi) = (sub.offset, sub.offset + sub.size.as_vec2());
        // Sub-rectangle in NDC (y up), then remap it to [-1, 1].
        let (l, r) = (-1.0 + 2.0 * lo.x / full.x, -1.0 + 2.0 * hi.x / full.x);
        let (t, bm) = (1.0 - 2.0 * lo.y / full.y, 1.0 - 2.0 * hi.y / full.y);
        let (kx, ky) = (2.0 / (r - l), 2.0 / (t - bm));
        let (cx, cy) = ((r + l) * 0.5, (t + bm) * 0.5);
        let crop = Mat4::from_cols(
            Vec4::new(kx, 0.0, 0.0, 0.0),
            Vec4::new(0.0, ky, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(-kx * cx, -ky * cy, 0.0, 1.0),
        );
        crop * self.get_clip_from_view()
    }

    /// The frustum is the 512×416 draw buffer's regardless of the target's size: the target is the game frame
    /// (crate::display), always of that aspect.
    fn update(&mut self, _width: f32, _height: f32) {}

    fn far(&self) -> f32 { self.far / UNITS }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        let (n, f) = (z_near.abs(), z_far.abs());
        let c = |d: f32, z: f32, sx: f32, sy: f32| Vec3A::new(sx * d * self.tan_x, sy * d * self.tan_y, z);
        [
            c(n, z_near, 1.0, -1.0),
            c(n, z_near, 1.0, 1.0),
            c(n, z_near, -1.0, 1.0),
            c(n, z_near, -1.0, -1.0),
            c(f, z_far, 1.0, -1.0),
            c(f, z_far, 1.0, 1.0),
            c(f, z_far, -1.0, 1.0),
            c(f, z_far, -1.0, -1.0),
        ]
    }
}

/// The camera's `Projection` component.
pub fn game_projection() -> Projection { Projection::custom(GameProjection::default()) }

/// The u32 at byte `off` of the level-settings section (gameplay file pointer 0, wad_layouts_rac1.md §3.3).
fn level_settings_word(gameplay: &[u8], off: usize) -> anyhow::Result<u32> {
    let u32_at = |o: usize| -> anyhow::Result<u32> {
        let b = gameplay.get(o..o + 4).ok_or_else(|| anyhow::anyhow!("gameplay file too short for level settings"))?;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    };
    let base = u32_at(0)? as usize;
    anyhow::ensure!(base != 0, "gameplay file has no level-settings section");
    u32_at(base + off)
}

/// Frame-buffer clear colour of a level, as the GS writes it (display-encoded bytes).
///
/// `fun_001e9b10` (level init, boot 0x1e9b10) copies level settings +0x00/+0x04/+0x08 (s32 r, g, b)
/// to 0x18cf3c.. and calls `set_background_color` (0x1fb280), which stores
/// `r | g << 8 | b << 16 | 0x80000000` into 0x152078. That word is the RGBAQ data of the packet at
/// 0x152040 built by `setup_fs_aa_buffer` (0x1fa978): A+D TEST_1 = 0x30000 (ZTE, ZTST = ALWAYS), then
/// PRIM = 0x106 (SPRITE, FST, no ABE/TME), RGBAQ, then 16 untextured sprites of 32 × H pixels with Z = 0
/// covering the whole 512 × 416 draw buffer. `append_gif_transfer_packet` (0x1fb368) puts it at the start of
/// every frame's chain (first call of the frame render `draw_debug_profiler`, 0x1f39d0), so the game clears
/// colour to this and Z to 0 (farthest under ZTST GEQUAL) before the sky. The clear is skipped when
/// `0x16045c` (sky header) != 0, sky header +0x04 (`clear_screen`, shrub_sky_rac1.md §2.2) == 0,
/// `uGpffff8834 & 1`, `0x18a2b8 != 0` and `0x18c34c == 0`: the same gates under which `transition_draw_sky`
/// draws the sky, and `update_sky_effects` zeroes `clear_screen` every frame, so while the sky is drawn the
/// game does not clear at all (the sky dome covers the frame). The port reproduces that in
/// `sky_render` (sky camera clears with this colour on the first frame or every frame, per level).
/// Start-of-level and space transitions call `set_background_color(0, 0, 0)`.
/// All 19 levels store r, g, b in 0..=255, so the OR does not bleed between channels.
pub fn level_background(gameplay: &[u8]) -> anyhow::Result<[u8; 3]> {
    let w = |o: usize| level_settings_word(gameplay, o);
    let rgbaq = w(0x00)? | w(0x04)? << 8 | w(0x08)? << 16 | 0x8000_0000;
    Ok([rgbaq as u8, (rgbaq >> 8) as u8, (rgbaq >> 16) as u8])
}

/// Fog fields of the gameplay file's level-settings section (pointer 0 of the gameplay file), exactly
/// as `fun_001e9b10` (level init, boot 0x1e9b10) copies them to 0x15f484..0x15f494 for `UpdateFog`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelFog {
    /// FOGCOL: low byte of the s32 r, g, b at +0x0c/+0x10/+0x14.
    pub color: [u8; 3],
    /// +0x18 / +0x1c, integer units (depth along the view axis).
    pub near_dist: f32,
    pub far_dist: f32,
    /// +0x20 / +0x24: GS fog coefficient F at those depths (255 = no fog).
    pub near_intensity: f32,
    pub far_intensity: f32,
}

impl LevelFog {
    /// `gameplay` is the decompressed gameplay file (`gameplay_ntsc.bin` after WAD decompression).
    pub fn parse(gameplay: &[u8]) -> anyhow::Result<Self> {
        let w = |o: usize| level_settings_word(gameplay, o);
        let f = |o: usize| w(o).map(f32::from_bits);
        Ok(LevelFog {
            color: [w(0x0c)? as u8, w(0x10)? as u8, w(0x14)? as u8],
            near_dist: f(0x18)?,
            far_dist: f(0x1c)?,
            near_intensity: f(0x20)?,
            far_intensity: f(0x24)?,
        })
    }

    /// `UpdateViewContext`'s fog terms, in f32 like the EE: (`0x160b30` = qw656.x = Q numerator,
    /// `0x18cdec` = w-lane slope of the projection, `0x18cf14` = qw661.w).
    pub fn vu_terms(&self) -> (f32, f32, f32) {
        let range = self.far_dist - self.near_dist;
        let q_num = ((self.far_intensity - self.near_intensity) * NEAR) / range;
        let slope = (1.0 / NEAR) * q_num;
        let offset = (self.near_intensity * self.far_dist - self.far_intensity * self.near_dist) / range;
        (q_num, slope, offset)
    }

    /// The level fog globals (0x15f444..) these values are a copy of, for `rc_game::fog_zones`.
    pub fn globals(&self) -> rc_game::fog_zones::FogGlobals {
        rc_game::fog_zones::FogGlobals {
            color: self.color,
            near_dist: self.near_dist,
            far_dist: self.far_dist,
            near_intensity: self.near_intensity,
            far_intensity: self.far_intensity,
        }
    }

    /// `UpdateFog`'s copy of the globals into the view context.
    pub fn from_globals(g: &rc_game::fog_zones::FogGlobals) -> Self {
        LevelFog { color: g.color, near_dist: g.near_dist, far_dist: g.far_dist, near_intensity: g.near_intensity, far_intensity: g.far_intensity }
    }

    /// VU1 constants qw656 and qw661 as the EE uploads them for tfrags (0x1de750 / 0x1de7a0).
    pub fn vu_constants(&self) -> ([f32; 4], [f32; 4]) {
        let (q_num, _, offset) = self.vu_terms();
        ([q_num, self.far_intensity, self.near_intensity, ADC_ADD], [XY_CENTRE, XY_CENTRE, Z_OFFSET, offset])
    }
}

/// Fog uniform of the tfrag material (binding 2). The vertex shader computes
/// `F = trunc(clamp(depth_int·slope + offset, lo, hi))` like the VU's w lane, the fragment shader
/// blends `FOGCOL + (C − FOGCOL)·F/255` like the GS with FGE = 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct TfragFog {
    /// FOGCOL as display-encoded 0..1 RGB; w = 1 enables fog (`RC_FOG=0` sets 0).
    pub color: Vec4,
    /// (slope per integer unit of depth, offset, lower clamp qw656.y, upper clamp qw656.z).
    pub params: Vec4,
}

impl TfragFog {
    pub fn new(fog: &LevelFog) -> Self {
        let enabled = std::env::var("RC_FOG").map(|v| v.trim() != "0").unwrap_or(true);
        let (_, slope, offset) = fog.vu_terms();
        let [r, g, b] = fog.color.map(|c| c as f32 / 255.0);
        TfragFog {
            color: Vec4::new(r, g, b, if enabled { 1.0 } else { 0.0 }),
            params: Vec4::new(slope, offset, fog.far_intensity, fog.near_intensity),
        }
    }
}

/// The 4×4 matrix `DrawTfrag` uploads to VU qw 5..8 for a camera at `eye` (game units) whose rotation
/// rows are `rot` = (forward, left, up), returned as those four quadwords (qw k multiplies coordinate k,
/// qw 8 is the translation row). `Translate(−eye·1024)`, then `0x186f40` (camera x = −left, y = −up,
/// z = forward), then the projection `0x18cdc0`. For comparing against a PCSX2 VU1 dump; not drawn with.
pub fn vu_tfrag_matrix(proj: &GameProjection, fog: &LevelFog, eye: Vec3, rot: [Vec3; 3]) -> [[f32; 4]; 4] {
    let (q_num, _, _) = fog.vu_terms();
    let (n, f) = (proj.near, proj.far);
    let fs = n * (f - n);
    // Column-vector form of 0x18cdc0: x' = sx·x, y' = sy·y, z' = A·z + C, w' = (q_num / n)·z.
    let p = Mat4::from_cols(
        Vec4::new(SCREEN_W * 0.5 / (proj.tan_x * n), 0.0, 0.0, 0.0),
        Vec4::new(0.0, SCREEN_H * 0.5 / (proj.tan_y * n), 0.0, 0.0),
        Vec4::new(0.0, 0.0, ((f + n) / fs) * Z_SCALE, (1.0 / n) * q_num),
        Vec4::new(0.0, 0.0, ((n * -2.0 * f) / fs) * Z_SCALE, 0.0),
    );
    let [fwd, left, up] = rot;
    let v = Mat4::from_cols((-left).extend(0.0), (-up).extend(0.0), fwd.extend(0.0), Vec4::W).transpose();
    let t = Mat4::from_translation(-eye * UNITS);
    (p * v * t).to_cols_array_2d()
}

/// The game's camera rows for a Bevy camera transform: forward, left, up in game (Z-up) coordinates.
pub fn game_rows(t: &Transform) -> [Vec3; 3] {
    let to_game = |v: Vec3| Vec3::new(v.x, -v.z, v.y);
    [to_game(*t.forward()), to_game(*t.left()), to_game(*t.up())]
}

/// Game-space eye position for a Bevy transform.
pub fn game_eye(t: &Transform) -> Vec3 { Vec3::new(t.translation.x, -t.translation.z, t.translation.y) }

#[cfg(test)]
mod tests {
    use super::*;

    const NOVALIS: LevelFog =
        LevelFog { color: [105, 127, 180], near_dist: 0.0, far_dist: 245760.0, near_intensity: 255.0, far_intensity: 102.0 };

    /// `set_background_color` packing of level settings +0x00/04/08 (Novalis values), section at pointer 0.
    #[test]
    fn level_background_reads_settings_rgb() {
        let mut g = vec![0u8; 0x40];
        g[0..4].copy_from_slice(&0x10u32.to_le_bytes());
        for (k, v) in [100u32, 255, 255].into_iter().enumerate() { g[0x10 + 4 * k..][..4].copy_from_slice(&v.to_le_bytes()); }
        assert_eq!(level_background(&g).unwrap(), [100, 255, 255]);
    }

    /// The Bevy clip matrix, the EE/VU pipeline (M, qw656, qw661) and `gs_xyz` agree.
    #[test]
    fn bevy_projection_matches_vu_pipeline() {
        let proj = GameProjection::default();
        let clip_from_view = proj.get_clip_from_view();
        let (k656, k661) = NOVALIS.vu_constants();
        // Camera at a Novalis-like spot looking along +X (rows forward, left, up = identity).
        let eye = Vec3::new(150.0, 200.0, 60.0);
        let m = Mat4::from_cols_array_2d(&vu_tfrag_matrix(&proj, &NOVALIS, eye, [Vec3::X, Vec3::Y, Vec3::Z]));
        for cam in [Vec3::new(0.0, 0.0, 40960.0), Vec3::new(-8000.0, 5000.0, 20000.0), Vec3::new(3000.0, -2000.0, 200000.0)] {
            let gs = proj.gs_xyz(cam);
            // Bevy: view = (x, -y, -z) / 1024.
            let clip = clip_from_view * Vec4::new(cam.x, -cam.y, -cam.z, UNITS) / UNITS;
            let ndc = clip.truncate() / clip.w;
            let bevy = Vec3::new(2048.0 + ndc.x * 256.0, 2048.0 - ndc.y * 208.0, ndc.z * 16_777_216.0);
            // VU: world point = eye + camera basis (x = -left, y = -up, z = forward).
            let world = eye * UNITS + Vec3::new(cam.z, -cam.x, -cam.y);
            let p = m * world.extend(1.0);
            let q = k656[0] / p.w;
            let vu = Vec3::new(p.x * q + k661[0], p.y * q + k661[1], p.z * q + k661[2]);
            let f = (p.w + k661[3]).min(k656[2]).max(k656[1]);
            let f_expect = (255.0 + (102.0 - 255.0) * cam.z / 245760.0).clamp(102.0, 255.0);
            assert!((gs.truncate() - bevy.truncate()).abs().max_element() < 1e-3, "{gs} vs bevy {bevy}");
            assert!((gs.truncate() - vu.truncate()).abs().max_element() < 1e-2, "{gs} vs vu {vu}");
            assert!((gs.z - bevy.z).abs() < 2.0 && (gs.z - vu.z).abs() < 4.0, "Z {gs} {bevy} {vu}");
            assert!((f - f_expect).abs() < 1e-2, "F {f} vs {f_expect}");
        }
        // Near plane maps to the top of the 24-bit Z range, far to the bottom.
        assert!((proj.gs_xyz(Vec3::new(0.0, 0.0, NEAR)).z - 16776192.0).abs() < 1.0);
        assert!((proj.gs_xyz(Vec3::new(0.0, 0.0, FAR)).z - 32.0).abs() < 1.0);
    }
}
