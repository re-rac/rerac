//! **The armed fly-by** (camera class 19; level10: hook `0x2f65f8`, init `0x2f5b58`, update `0x2f5f18`, pre `0x2f66c0`
//! (empty), the region test `0x2f5a78`; copies on levels 13, 14 and 15). A scripted camera a class arms (`arm`: the
//! record's +0x38 = 1, +0x39 = 0; level10 `0x2f5a50`, level13 `0x316f20`, level14 `0x314168`): the screen fades to
//! black, the camera jumps to the start of its path (+0x20) looking along a second path (+0x24, or at a moby +0x28 with
//! an offset), the letterbox comes up and the screen fades back in; the camera flies the path (its own speed, or the
//! path's length over +0x3a ticks), the field of view keyed along it; at the path's end, the timer's or when the class
//! ends it (`end`: +0x39 = 1 while it is current; level13 `0x316f48`, level14 `0x314190`) the screen fades to black
//! once more and the follow camera comes back with a cut while the fade clears. Ratchet is held (state 0x72) through it
//! unless +0x4c. Spec: docs/plan/player_controller.md §15 "Class 19".
//!
//! **System or not.** One class (a `camvtbl` row); level 10's code and its copies ([`super::level::CameraPorts`]
//! compares the four functions and the region test). The fade (0x15f3fc), the letterbox (0x15f404), the view's tangent
//! (0x16cf70) and Ratchet's `SetState` are engine words the camera writes: [`FlybyOut`], which the tick applies after
//! the camera update.
//!
//! **The record** (pvar): the header (its shapes +0x08 / +0x0c / +0x10 / +0x14 the region); +0x20 the camera's path,
//! +0x24 the look path, +0x28 a moby to look at (−1: the path), +0x2c..+0x34 the offset from it (through the camera's
//! rows), +0x38 armed, +0x39 done, +0x3a its ticks (s16; 0: the path's end), +0x3c its speed (u/s; from the ticks when
//! set), +0x40 keep flying after the end, +0x44 / +0x48 Ratchet's state / group it needs (−1: any), +0x4c no hold.
//!
//! | address | call / branch | port |
//! |---|---|---|
//! | 0x2f65f8 | done → −1; armed and the best not class 19 → 1; else the best not releasing and not below in priority → 0; the region test (`0x2f5a78`: the cuboid, cylinder, sphere or path of the header holding Ratchet's feet; no shape: armed and not done), state / group → 1 | [`Camera::flyby_hook`] |
//! | 0x2f5b58 | +0x38 = 1; D+0x40..0x54 = 0, D+0x56 = `ticks(+0x3a)`; the FOV keys D+0x58 (point index, −1) / D+0x68 (degrees, the current FOV when > 180) from the camera path's points with w > 0; D+0x88 = 2·atan(0x16cf70); the camera path's w = its chords, D+0x48 its length; +0x3a > 0 → +0x3c = length / ticks · 60; the look path's chords and length D+0x4c; D+0x00 / +0x04 / +0x20 / +0x24 = 0.03 / 0.2; the previous camera's rows and position; not +0x4c → `SetState(0x72, 1)` | [`Camera::flyby_init`] |
//! | 0x2f5f18 phase 0 | the fade + 0.05 until 1 → the letterbox, phase 1, the camera at the path's first point facing the look path's first point (or the moby + offset) | [`Camera::flyby_update`] |
//! | phase 1 | the fade − 0.05 (to 0); along the path at +0x3c·dt (`0x250318` = `crate::spline::advance`); the timer D+0x56; the end (path, timer, +0x39; with +0x40 the timer below `ticks(20)`) → phase 2; the FOV eased between its keys (`0x24a8d0` = `CosInterp`) into 0x16cf70; the look along the look path by the share travelled, or the moby + offset; rows from the look and −gravity | `flyby_update` |
//! | phase 2 | the fade + 0.05 to 1 → phase 3: the FOV back, +0x39 = 1, the fade-out rate 0x167358 = 0.05, +0x7e = 4, the letterbox off, not +0x4c → `SetState(0, 1)`; +0x40 keeps flying | `flyby_update` |
//! | phase 3 | nothing (the follow camera takes over: +0x7e = 4, a cut) | `flyby_update` |
//! | 0x2f66c0 | pre hook: empty | n/a |
//! | `FUN_001ea298` / `0x1ea290` | debug draws of the look point and path | n/a |
//!
//! Native `f32`.

use super::class_cam::{rows_about, set_len};
use super::{fsub, CamInput, Camera};
use crate::hero::physics::to_f32x3;
use crate::spline::{advance, Cursor};

/// The camera class.
pub const CLASS_FLYBY: i32 = 19;
/// The level-10 functions of class 19 ([`super::level::CameraPorts::from_overlays`] looks for their copies).
pub const FLYBY_FNS: [u32; 4] = [0x2f_65f8, 0x2f_5b58, 0x2f_5f18, 0x2f_66c0];
pub const FLYBY_HELPERS: [u32; 1] = [0x2f_5a78];

/// The record's words (module doc).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlybyRec {
    pub cam_path: i32,
    pub look_path: i32,
    pub moby: i32,
    pub offset: [f32; 3],
    pub armed: u8,
    pub done: u8,
    pub ticks: i16,
    pub speed: f32,
    pub keep: i32,
    pub state: i32,
    pub group: i32,
    pub no_hold: i32,
}

impl FlybyRec {
    pub fn parse(p: &[u8]) -> Option<FlybyRec> {
        let w = |o: usize| p.get(o..o + 4).map(|b| [b[0], b[1], b[2], b[3]]);
        let i = |o: usize| w(o).map(i32::from_le_bytes);
        let f = |o: usize| w(o).map(f32::from_le_bytes);
        Some(FlybyRec {
            cam_path: i(0x20)?,
            look_path: i(0x24)?,
            moby: i(0x28)?,
            offset: [f(0x2c)?, f(0x30)?, f(0x34)?],
            armed: *p.get(0x38)?,
            done: *p.get(0x39)?,
            ticks: i16::from_le_bytes([*p.get(0x3a)?, *p.get(0x3b)?]),
            speed: f(0x3c)?,
            keep: i(0x40)?,
            state: i(0x44)?,
            group: i(0x48)?,
            no_hold: i(0x4c)?,
        })
    }
}

/// Class 19's D-block words.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlybyState {
    /// D+0x40 / +0x44: the cursor on the camera path (segment, units along it).
    pub cursor: Cursor,
    /// D+0x48 / +0x4c: the camera and look paths' lengths; D+0x50 the distance flown.
    pub cam_len: f32,
    pub look_len: f32,
    pub flown: f32,
    /// D+0x54: the phase; D+0x56: the timer.
    pub phase: i16,
    pub timer: i16,
    /// D+0x58 / +0x68: the FOV keys (point index, −1 past the last) and their angles in degrees.
    pub keys: [i16; 8],
    pub degrees: [f32; 8],
    /// D+0x88: the FOV the fly-by found (radians).
    pub base_fov: f32,
    /// The camera path with its chords in w (the game writes them into the level's path), and the look path's.
    pub cam_path: Vec<[f32; 4]>,
    pub look_path: Vec<[f32; 4]>,
}

/// The engine words class 19 writes this tick (module doc), applied by the tick after the camera update.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlybyOut {
    /// 0x15f3fc.
    pub fade: Option<f32>,
    /// 0x15f404.
    pub letterbox: Option<bool>,
    /// 0x16cf70 (`UpdateViewContext` after it).
    pub tan: Option<f32>,
    /// `SetState(state, 1)` on Ratchet (made before the next hero update).
    pub hero_state: Option<i32>,
    /// 0x167358: the fade-out rate (`fun_001eda60` takes it off the fade each frame until 0).
    pub fade_rate: Option<f32>,
    /// The race cameras' stores into the moby world and the hero block (classes 20 / 21, [`super::race`]), made
    /// before the next moby loop.
    pub race: super::race::RaceOut,
}

/// The engine words class 19 reads, given by the tick before the camera update.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamEngine {
    /// 0x15f3fc.
    pub fade: f32,
    /// 0x16cf70.
    pub tan: f32,
}

impl Default for CamEngine {
    fn default() -> Self { CamEngine { fade: 0.0, tan: 0.63 } }
}

/// The level path `i` with each point's w its chord to the next (the last point's to the first) and the length of the
/// first n − 1 chords (`0x2f5b58`'s loops).
pub(super) fn chords(v: Option<&rc_formats::volumes::Volumes>, i: i32) -> (Vec<[f32; 4]>, f32) {
    let Some(src) = usize::try_from(i).ok().and_then(|i| v.and_then(|v| v.paths.get(i))) else { return (Vec::new(), 0.0) };
    let n = src.len();
    let mut out = src.clone();
    let mut len = 0.0;
    for k in 0..n {
        let (a, b) = (src[k], src[(k + 1) % n]);
        let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        out[k][3] = d;
        if k + 1 < n { len += d; }
    }
    (out, len)
}

/// The game's `fun_001f96f8` (`ticks(n)`, NTSC: n).
fn ticks(n: i32) -> i32 { crate::hero::physics::ticks(n) }

impl Camera {
    pub(super) fn flyby_rec(&self, slot: usize) -> Option<FlybyRec> { self.level_cams.slots.get(slot).and_then(|s| s.flyby) }
    pub(super) fn flyby_rec_mut(&mut self, slot: usize) -> Option<&mut FlybyRec> { self.level_cams.slots.get_mut(slot).and_then(|s| s.flyby.as_mut()) }

    /// A class's arming of the fly-by record `slot` (`0x2f5a50` and its copies: +0x38 = 1, +0x39 = 0).
    pub fn flyby_arm(&mut self, slot: usize) {
        if let Some(r) = self.flyby_rec_mut(slot) {
            r.armed = 1;
            r.done = 0;
        }
    }

    /// A class ending the fly-by record `slot` (`0x316f48` / `0x314190`: +0x39 = 1 while class 19 is current).
    pub fn flyby_end(&mut self, slot: usize) {
        if self.current_class() != CLASS_FLYBY { return; }
        if let Some(r) = self.flyby_rec_mut(slot) { r.done = 1; }
    }

    /// `0x2f65f8` for slot `i` (module doc): −1 no, 1 yes, 0 go on. `best_class` / `best` the best so far.
    pub(super) fn flyby_hook(&self, i: usize, best_class: i32, best: super::level::Best, inp: &CamInput) -> i32 {
        let Some(r) = self.flyby_rec(i) else { return 0 };
        if r.done != 0 { return -1; }
        if r.armed != 0 && best_class != CLASS_FLYBY { return 1; }
        let mine = self.level_cams.slots[i].header.priority;
        if !best.releasing && mine <= best.priority { return 0; }
        let h = inp.hero;
        if !self.flyby_region(i, inp) { return 0; }
        if 0 <= r.group && h.group != r.group { return 0; }
        if 0 <= r.state && h.state != r.state { return 0; }
        1
    }

    /// `0x2f5a78`: the header's cuboid, cylinder, sphere or path (the first one set) holds Ratchet's feet; without
    /// shapes: armed and not done.
    pub(super) fn flyby_region(&self, i: usize, inp: &CamInput) -> bool {
        use crate::moby_update::triggers::{point_in_cuboid, point_in_cylinder, point_in_path, point_in_sphere};
        let s = &self.level_cams.slots[i];
        let hd = s.header;
        let p = to_f32x3(inp.hero.pos);
        let Some(v) = self.level_cams.shapes() else { return false };
        if 0 <= hd.cuboid { return point_in_cuboid(v, p, hd.cuboid); }
        if 0 <= hd.cylinder { return point_in_cylinder(v, p, hd.cylinder); }
        if 0 <= hd.sphere { return point_in_sphere(v, p, hd.sphere); }
        if 0 <= hd.path { return point_in_path(v, p, hd.path); }
        s.flyby.is_some_and(|r| r.armed != 0 && r.done == 0)
    }

    /// `0x2f5b58` (module doc): `prev` the camera being left.
    pub(super) fn flyby_init(&mut self, prev: ([[f32; 3]; 3], [f32; 3])) {
        let Some(r) = self.flyby_begin(prev, 1, true) else { return };
        if r.no_hold == 0 { self.flyby_out.hero_state = Some(0x72); }
    }

    /// The init shared with class 20's `0x314f00`: the record's +0x38 = `armed`, the D words, the FOV keys (`substitute`:
    /// a key above 180° takes the current FOV, class 19 only), the paths' chords and lengths, the speed from the ticks,
    /// the previous camera's rows and position. Returns the record as stored.
    pub(super) fn flyby_begin(&mut self, prev: ([[f32; 3]; 3], [f32; 3]), armed: u8, substitute: bool) -> Option<FlybyRec> {
        let slot = self.class_cam.slot;
        let mut r = self.flyby_rec(slot)?;
        r.armed = armed;
        let vols = self.level_cams.shapes_arc();
        let (cam_path, cam_len) = chords(vols.as_deref(), r.cam_path);
        let (look_path, look_len) = chords(vols.as_deref(), r.look_path);
        let tan = self.engine.tan;
        let base_fov = tan.atan() * 2.0;
        let mut st = FlybyState { cam_len, look_len, base_fov, keys: [-1; 8], timer: ticks(r.ticks as i32) as i16, ..FlybyState::default() };
        let mut k = 0;
        for (j, q) in self.flyby_source(r.cam_path).iter().enumerate() {
            if 0.0 < q[3] && k < 8 {
                st.degrees[k] = if substitute && 180.0 < q[3] { base_fov * 57.295_776 } else { q[3] };
                st.keys[k] = j as i16;
                k += 1;
            }
        }
        if 0 < r.ticks { r.speed = (cam_len / ticks(r.ticks as i32) as f32) * 60.0; }
        st.cam_path = cam_path;
        st.look_path = look_path;
        self.class_cam.flyby = st;
        if let Some(rm) = self.flyby_rec_mut(slot) { *rm = r; }
        self.class_cam.rows = prev.0;
        self.class_cam.pos = prev.1;
        Some(r)
    }

    /// The camera path's points as the level loaded them (their w the FOV keys before the chords overwrite them).
    fn flyby_source(&self, path: i32) -> Vec<[f32; 4]> {
        let v = self.level_cams.shapes_arc();
        usize::try_from(path).ok().and_then(|i| v.as_deref().and_then(|v| v.paths.get(i).cloned())).unwrap_or_default()
    }

    /// The look point: the moby +0x28 plus the offset through the camera's rows, or `path_pt`.
    pub(super) fn flyby_look(&self, r: &FlybyRec, path_pt: [f32; 3]) -> [f32; 3] {
        let Ok(id) = usize::try_from(r.moby) else { return path_pt };
        let Some(m) = self.world.mobys.get(&id) else { return path_pt };
        let rows = self.class_cam.rows;
        let o = r.offset;
        let v: [f32; 3] = std::array::from_fn(|k| rows[0][k] * o[0] + rows[1][k] * o[1] + rows[2][k] * o[2]);
        [m.pos[0] + v[0], m.pos[1] + v[1], m.pos[2] + v[2]]
    }

    /// The rows toward `look` about −gravity (`FastVecCross` pair, [`rows_about`]).
    pub(super) fn flyby_face(&mut self, look: [f32; 3], inp: &CamInput) {
        let g = to_f32x3(inp.hero.gravity_dir);
        let up = [-g[0], -g[1], -g[2]];
        let fwd = set_len(fsub(look, self.class_cam.pos), 1.0);
        self.class_cam.rows = rows_about(fwd, up);
    }

    /// `0x2f5f18` (module doc).
    pub(super) fn flyby_update(&mut self, inp: &CamInput) {
        let slot = self.class_cam.slot;
        let Some(r) = self.flyby_rec(slot) else { return };
        let fade = self.flyby_out.fade.unwrap_or(self.engine.fade);
        match self.class_cam.flyby.phase {
            0 => {
                let f = fade + 0.05;
                self.flyby_out.fade = Some(f);
                if f < 1.0 { return; }
                self.flyby_out.fade = Some(1.0);
                self.flyby_out.letterbox = Some(true);
                let st = &mut self.class_cam.flyby;
                st.phase = 1;
                let start = st.cam_path.first().map_or(self.class_cam.pos, |q| [q[0], q[1], q[2]]);
                let look0 = st.look_path.first().map_or(start, |q| [q[0], q[1], q[2]]);
                self.class_cam.pos = start;
                let look = self.flyby_look(&r, look0);
                self.flyby_face(look, inp);
                return;
            }
            2 => {
                let f = fade + 0.05;
                self.flyby_out.fade = Some(f);
                if 1.0 <= f {
                    self.class_cam.flyby.phase = 3;
                    self.flyby_out.fade = Some(1.0);
                    let half = self.class_cam.flyby.base_fov * 0.5;
                    self.flyby_out.tan = Some(half.sin() / half.cos());
                    if let Some(rm) = self.flyby_rec_mut(slot) { rm.done = 1; }
                    self.flyby_out.fade_rate = Some(0.05);
                    self.class_cam.release = 4;
                    self.flyby_out.letterbox = Some(false);
                    if r.no_hold == 0 { self.flyby_out.hero_state = Some(0); }
                }
                if r.keep == 0 { return; }
            }
            3 => return,
            _ => {
                let f = fade - 0.05;
                self.flyby_out.fade = Some(if f < 0.0 { 0.0 } else { f });
            }
        }
        let old = self.class_cam.pos;
        let dist = r.speed * crate::hero::physics::DT.to_f32();
        let st = &mut self.class_cam.flyby;
        let (p, mut ended) = advance(&st.cam_path, false, dist, &mut st.cursor);
        self.class_cam.pos = p;
        if st.timer != 0 {
            let mut t = st.timer as i32;
            let out = crate::moby_update::creature::dec_timer_i32(&mut t) != 0;
            st.timer = t as i16;
            // Run out, or (with +0x40) under `ticks(20)` left.
            if out || (r.keep != 0 && (st.timer as i32) < ticks(20)) { ended = true; }
        }
        if self.flyby_rec(slot).is_some_and(|r| r.done != 0) { ended = true; }
        if ended {
            self.class_cam.flyby.phase = 2;
            if r.keep == 0 { return; }
        }
        // The FOV between its keys.
        let st = &self.class_cam.flyby;
        let seg = st.cursor.seg;
        for k in 0..7 {
            let (a, b) = (st.keys[k], st.keys[k + 1]);
            if a < 0 || b < 0 { break; }
            let (a, b) = (a as i32, b as i32);
            if !(a <= seg && seg < b) { continue; }
            let chord = |j: i32| st.cam_path.get(j as usize).map_or(0.0, |q| q[3]);
            let before: f32 = (a..seg).map(chord).sum();
            let span: f32 = before + (seg..b).map(chord).sum::<f32>();
            let at = before + st.cursor.t;
            let deg = super::script::cos_interp(st.degrees[k], st.degrees[k + 1], at / span);
            let half = deg * 0.017_453_292 * 0.5;
            self.flyby_out.tan = Some(half.sin() / half.cos());
        }
        // The look.
        let d = ((p[0] - old[0]).powi(2) + (p[1] - old[1]).powi(2) + (p[2] - old[2]).powi(2)).sqrt();
        let st = &mut self.class_cam.flyby;
        st.flown += d;
        let look = if r.moby < 0 {
            let share = if st.cam_len == 0.0 { 0.0 } else { st.flown / st.cam_len * st.look_len };
            let mut c = Cursor::default();
            advance(&st.look_path, false, share, &mut c).0
        } else {
            self.flyby_look(&r, p)
        };
        self.flyby_face(look, inp);
    }
}
