//! The 14 menu-frame mobys of the page menu (class 0x472), their animation and the panel rects taken
//! from their corner joints. Spec: docs/plan/menus.md §3 ("First tick", "Transitions", "Render") and §7.
//!
//! **Class.** 0x472 = o_class 1138, an ordinary moby class in every level's core (moby class table,
//! `moby_class/1138`): 5 joints (root + the four corners), one high- and one low-LOD packet, 216
//! sequences (0..13 one frame, the rest 7). Its joint lists 0..3 end on joints 2, 1, 4, 3.
//!
//! **Spawn** (`FUN_0028c128`, the first tick, only when a page is set): `SpawnHandGadgetMoby(0x472)` 14
//! times into `0x1ba310[14]` — `CreateMoby` (anim state as `init_moby_instance`, scale moby+0x2c = the class
//! scale), +0x32 = 0xff, +0x30 = 0xff, +0x20 = 0, +0x31 = 1, `MobyBuildMatrix`, `pack_render_command_fields(m,
//! 0x202020, 0xe, 0xe, 0)` (moby+0x38: light sets 14 / 14, no cross-fade, ambient 0x20 0x20 0x20); mode &=
//! ~2; update = `MenuMobyUpdate` 0x309898; position = the menu camera position 0x167240 (just set to
//! (256, 256, 64) by `fun_00218d10`), rotation (0, 0, 0); `hard_cut(page.seq[i], frame_count − 1)`.
//!
//! **Per tick** (`MobyUpdateLoop` 0x2793d8 from `PageMenuUpdate`, after the widget updates): `MobyAnimAdvance`,
//! then `MenuMobyUpdate`: once the sequence wraps (+0x70 & 2) t = 0 if the speed is > 0 else 1.0 and the
//! speed = 0 (so a forward play stops on the last frame, a backward one on frame 0); then `fun_0023a318`:
//! `MobyGetBoneMatrix(m, 4, lists {0, 1, 2, 3} at 0x161fe0, pvar)` (joint translations of the lists'
//! last joints, `fun_002106f8`, scaled, rotated, placed) into pvar +0x00..+0x30, and the lengths of
//! corner 1 − corner 0 / corner 2 − corner 0 into pvar +0x40 / +0x44 (not read by the menu).
//!
//! **Rect** (`PageMenuDraw`): corners 0 and 3 through `MobyScreenRect` 0x2ada30 with the menu camera state
//! ([`MenuProjection`]).

use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, V4};
use rc_formats::moby_light;
use rc_formats::tfrag_light::ps2;

/// The frame class (`SpawnHandGadgetMoby(0x472)` in `FUN_0028c128`).
pub const O_CLASS: i32 = 0x472;
pub const SLOTS: usize = 14;
/// `fun_00218d10`: the menu camera position 0x167240, also every frame moby's position (moby+0x10).
pub const CAMERA_POS: [f32; 3] = [256.0, 256.0, 64.0];
/// moby+0x38..0x3b (`pack_render_command_fields(m, 0x202020, 0xe, 0xe, 0)`): light set 14 twice, no
/// cross-fade; moby+0x3c.. ambient.
pub const LIGHT_WORD: u32 = 0x0e0e;
pub const AMBIENT: [u8; 3] = [0x20, 0x20, 0x20];
/// The light set the menu rewrites (0x180340 + 14·0x40 = 0x1806c0).
pub const LIGHT_SET: usize = 14;
/// `FUN_0028c128`: set 14 = { colour A = the quadword at 0x160290, direction A = 1.0 · the xyz at
/// 0x160280 (`VecScale`, w kept), colour B = direction B = 0 }.
pub const LIGHT_DIR_ADDR: u32 = 0x160280;
pub const LIGHT_COLOR_ADDR: u32 = 0x160290;
/// 0x161fe0: the four joint-list ids `fun_0023a318` evaluates (0, 1, 2, 3 on the disc).
pub const CORNER_LISTS_ADDR: u32 = 0x161fe0;

/// What the frame mobys need from the class 0x472 (loaded by the engine from the level core).
#[derive(Clone, Debug)]
pub struct FrameClass {
    pub anim: MobyAnimClass,
    /// The first byte list of each joint list named at 0x161fe0 (root-to-corner chains).
    pub chains: [Vec<u8>; 4],
    /// Class header scale = moby+0x2c.
    pub scale: f32,
}

/// One frame moby: its animation state and the pvar fields the menu reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMoby {
    pub state: AnimState,
    /// pvar +0x00, +0x10, +0x20, +0x30: the corner points (world, game units, as raw bits).
    pub corners: [V4; 4],
    /// pvar +0x50..+0x5c written by the draw: x, y (the `MobyScreenRect` corner + 1), w, h.
    pub rect: [i32; 4],
    /// pvar +0x48 / +0x4c: the panel's noise burst (`fun_00223e28`, crate::menus::screen_static).
    pub noise: crate::menus::screen_static::PanelStatic,
    /// Port-only: the world z each corner was moved by ([`FrameMobys::stretch`]); the engine moves the corner joints
    /// of the drawn moby by the same amount.
    pub lift: [f32; 4],
}

/// The menu camera state `MobyScreenRect` projects with: `fun_00218d10` (position (256, 256, 64), rows
/// 0x167450 = identity), `UpdateViewContext` 0x219580 with the view-context defaults of `InitViewContext`
/// (near 32, draw buffer 512 × 416) and FOV 0x16cf70 = 0.63 (0x3f2147ae, set by `FUN_0028c128`), and
/// `BuildRotationViewProj` 0x218a48 (0x16c4ec = 0: the identity rows, not the Euler angles).
///
/// With V = the view rows of the identity camera ((−0, −0, 1), (−1, −0, 0), (−0, −1, 0)) the product
/// 0x167200 = `sceVu0MulMatrix(P, V)` has rows (P2, −P0, −P1, P3) up to signed zeros, P = 0x16d000 with rows
/// 0 and 1 scaled by 0x16d080 / 0x16d084 (= 4); so `fun_001f9d20` gives x = −P0.x·y, y = −P1.y·z,
/// w = P2.w·x (every other product is 0 and the adds with ±0 are exact).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuProjection {
    /// −(0x16d000.x · 0x16d080), −(0x16d014 · 0x16d084), 0x16d02c (= 1 / near).
    pub neg_p0x: u32,
    pub neg_p1y: u32,
    pub p2w: u32,
    /// 0x16d050 / 0x16d054 (= 0x16d0c8 / 0x16d0cc: 1024, 832).
    pub s050: u32,
    pub s054: u32,
    /// `(float)` 0x13e508 / 0x13e50c: 256, 208.
    pub cx: u32,
    pub cy: u32,
}

impl MenuProjection {
    /// NTSC (`UpdateViewContext`: vertical tangent = 0.775 × horizontal; PAL 0.756).
    pub fn new(pal: bool) -> Self {
        use ps2::{div, mul};
        let f = |x: f32| x.to_bits();
        let (w, h) = (512.0f32, 416.0f32);
        // InitViewContext: 0x16d0c0 = w·0.5, 0x16d0c4 = h·0.5, 0x16d0c8 = 0x16d0c0·4, 0x16d0cc = h·0.5·4.
        let (c0, c4) = (mul(f(w), f(0.5)), mul(f(h), f(0.5)));
        let (c8, cc) = (mul(c0, f(4.0)), mul(mul(f(h), f(0.5)), f(4.0)));
        let (near, tx) = (f(32.0), 0x3f21_47ae);
        let ty = mul(tx, if pal { 0x3f41_8937 } else { 0x3f46_6666 });
        // 0x16cf80 = c0 / (tx·n), 0x16cf94 = c4 / (ty·n); 0x16d000 = 0x16cf80 / c8, 0x16d014 = 0x16cf94 / cc;
        // 0x16cfec = 1 / n (copied to 0x16d02c).
        let (d0x, d1y) = (div(div(c0, mul(tx, near)), c8), div(div(c4, mul(ty, near)), cc));
        let (d080, d084) = (div(c8, c0), div(cc, c4));
        MenuProjection {
            neg_p0x: mul(d0x, d080) ^ ps2::SIGN,
            neg_p1y: mul(d1y, d084) ^ ps2::SIGN,
            p2w: div(ps2::ONE, near),
            s050: c8,
            s054: cc,
            cx: f(256.0),
            cy: f(208.0),
        }
    }

    /// `MobyScreenRect(a, b, &w, &h, &x, &y)` 0x2ada30 for corner points `a`, `b` (world, game units);
    /// returns [x, y, w, h].
    pub fn screen_rect(&self, a: V4, b: V4) -> [i32; 4] {
        use ps2::{add, div, mul, sub};
        let cam = CAMERA_POS.map(f32::to_bits);
        let k1024 = 0x4480_0000;
        // VecSub (vsub.xyz) and VecScale by 1024 (vmulx.xyz); w = 1; fun_001f9d20 with 0x167200.
        let proj = |p: V4| -> (u32, u32, u32) {
            let d = [0, 1, 2].map(|l| mul(sub(p[l], cam[l]), k1024));
            (mul(self.neg_p0x, d[1]), mul(self.neg_p1y, d[2]), mul(self.p2w, d[0]))
        };
        let ((ax, ay, aw), (bx, by, bw)) = (proj(a), proj(b));
        let ia = div(ps2::ONE, aw);
        let (ax, ay) = (mul(ax, ia), mul(ay, ia));
        let ib = div(ps2::ONE, bw);
        let (bx, by) = (mul(bx, ib), mul(by, ib));
        let (ax, ay, bx, by) = (mul(ax, self.s050), mul(ay, self.s054), mul(bx, self.s050), mul(by, self.s054));
        let q = 0x3e80_0000; // 0.25
        [
            cvt_w(add(mul(ax, q), self.cx)),
            cvt_w(add(mul(ay, q), self.cy)),
            cvt_w(mul(sub(bx, ax), q)),
            cvt_w(mul(sub(by, ay), q)),
        ]
    }
}

/// EE `cvt.w.s`: truncation toward zero, saturating (the FPU has no other rounding mode).
pub fn cvt_w(x: u32) -> i32 {
    let e = ((x >> 23) & 0xff) as i32;
    if e < 127 { return 0; }
    if e >= 127 + 31 { return if x & ps2::SIGN != 0 { i32::MIN } else { i32::MAX }; }
    let m = ((x & 0x7f_ffff) | 0x80_0000) as i64;
    let v = if e >= 150 { m << (e - 150) } else { m >> (150 - e) };
    (if x & ps2::SIGN != 0 { -v } else { v }) as i32
}

/// The 14 frame mobys (`0x1ba310[14]`, None when freed or never spawned) and the class they use.
#[derive(Clone, Debug)]
pub struct FrameMobys {
    pub class: FrameClass,
    /// `MobyBuildMatrix` rows for rotation (0, 0, 0) (moby+0xc0..).
    pub rows: [V4; 3],
    pub proj: MenuProjection,
    pub slots: Option<[FrameMoby; SLOTS]>,
    /// Port-only: per slot, the screen rows `[top, bottom]` its panel is stretched to (the Port Options list), reached
    /// as the slot's sequence reaches its last frame; None = the game's panel.
    pub stretch: [Option<[i32; 2]>; SLOTS],
}

impl FrameMobys {
    pub fn new(class: FrameClass, pal: bool) -> Self {
        FrameMobys { class, rows: moby_light::rotation_rows([0.0; 3]), proj: MenuProjection::new(pal), slots: None, stretch: [None; SLOTS] }
    }

    /// How far slot state `s` is through its sequence (0 on frame 0, 1 on the last frame; 1 for a one-frame sequence).
    fn progress(&self, s: &AnimState) -> f32 {
        let Some(last) = self.last_frame(s.seq_a as i32).filter(|&l| l > 0) else { return 1.0 };
        let pos = s.frame_a as f32 + s.t * (s.frame_b as f32 - s.frame_a as f32);
        (pos / last as f32).clamp(0.0, 1.0)
    }

    /// Frame count − 1 of sequence `seq` (`*(class + 0x48 + 4·seq)` +0x10, minus 1), None when missing.
    fn last_frame(&self, seq: i32) -> Option<i32> {
        let s = self.class.anim.sequence(u8::try_from(seq).ok()?)?;
        Some(s.header.frame_count as i32 - 1)
    }

    /// `fun_00212ed8` on slot `i` (a missing sequence leaves the state as it is).
    fn cut(&mut self, i: usize, seq: i32, frame: i32) {
        let Some(slots) = self.slots.as_mut() else { return };
        if let Ok(q) = u8::try_from(seq) { moby_anim::hard_cut(&mut slots[i].state, &self.class.anim, q, frame); }
    }

    /// The spawn of `FUN_0028c128` for the page's seqs: every moby on its sequence's last frame.
    pub fn spawn(&mut self, seqs: &[i32; SLOTS]) {
        let state = AnimState::spawn(&self.class.anim);
        self.slots = Some([FrameMoby { state, corners: [[0; 4]; 4], rect: [0; 4], noise: Default::default(), lift: [0.0; 4] }; SLOTS]);
        for (i, &s) in seqs.iter().enumerate() {
            let last = self.last_frame(s).unwrap_or(0);
            self.cut(i, s, last);
        }
    }

    /// The transition's moby step (`PageMenuUpdate` 0x28c990): `back` = `hard_cut(seq, last)` at speed
    /// −1.0 (moby+0x58), forward = `hard_cut(seq, 0)` at +1.0.
    pub fn start(&mut self, i: usize, seq: i32, back: bool) {
        let frame = if back { self.last_frame(seq).unwrap_or(0) } else { 0 };
        self.cut(i, seq, frame);
        if let Some(slots) = self.slots.as_mut() { slots[i].state.speed = if back { -1.0 } else { 1.0 }; }
    }

    /// One `MobyUpdateLoop` for the frame mobys: advance, `MenuMobyUpdate`, corners.
    pub fn update(&mut self) {
        let Some(mut slots) = self.slots.take() else { return };
        let chains: [&[u8]; 4] = std::array::from_fn(|k| self.class.chains[k].as_slice());
        for (i, m) in slots.iter_mut().enumerate() {
            if !m.state.skip_advance { moby_anim::advance(&mut m.state, &self.class.anim); }
            if m.state.flags & 2 != 0 {
                m.state.t = if m.state.speed > 0.0 { 0.0 } else { 1.0 };
                m.state.speed = 0.0;
            }
            let p = moby_anim::joint_translations(&self.class.anim, &m.state, None, &chains);
            let w = moby_anim::bone_points(&p, &self.rows, CAMERA_POS, self.class.scale);
            m.corners = std::array::from_fn(|k| w[k]);
            m.lift = [0.0; 4];
            if let Some(target) = self.stretch[i] { self.stretch_corners(m, target); }
        }
        self.slots = Some(slots);
    }

    /// Port-only: moves slot `m`'s top corners up and its bottom corners down (world z, the screen's vertical) so its
    /// rect spans the rows `[top, bottom]` on the sequence's last frame, in proportion to [`Self::progress`] before.
    fn stretch_corners(&self, m: &mut FrameMoby, [top, bottom]: [i32; 2]) {
        let prog = self.progress(&m.state);
        if prog <= 0.0 { return; }
        let z = |c: V4| f32::from_bits(c[2]);
        let [_, y, _, h] = self.proj.screen_rect(m.corners[0], m.corners[3]);
        // Screen rows per world unit of z at the corners' depth (all four share it).
        let mut probe = m.corners[0];
        probe[2] = (z(probe) - 10.0).to_bits();
        let k = self.proj.screen_rect(m.corners[0], probe)[3] as f32 / 10.0;
        if k <= 0.0 { return; }
        let up = (y + 1 - top) as f32 * prog / k;
        let down = (bottom - (y + 1 + h)) as f32 * prog / k;
        let mid = (z(m.corners[0]) + z(m.corners[3])) * 0.5;
        for (c, l) in m.corners.iter_mut().zip(&mut m.lift) {
            *l = if z(*c) > mid { up } else { -down };
            c[2] = (z(*c) + *l).to_bits();
        }
    }

    /// `FUN_00298f38` on every slot (`PageMenuClose`).
    pub fn free(&mut self) { self.slots = None; }

    /// `MobyScreenRect(corner 0, corner 3)` of slot `i` → pvar +0x50.. = (x + 1, y + 1, w, h); returned.
    pub fn place(&mut self, i: usize) -> Option<[i32; 4]> {
        let proj = self.proj;
        let m = &mut self.slots.as_mut()?[i];
        let [x, y, w, h] = proj.screen_rect(m.corners[0], m.corners[3]);
        m.rect = [x + 1, y + 1, w, h];
        Some(m.rect)
    }

    pub fn present(&self) -> bool { self.slots.is_some() }

    /// `fun_00223e28(0x1ba310[i])` on slot `i`'s pvar rect (crate::menus::screen_static::PanelStatic::draw).
    pub fn panel_static(&mut self, i: usize, pal: bool, vignette: (usize, (i32, i32)), rng: &mut crate::rng::Rng, out: &mut Vec<crate::menus::screen_static::StaticDraw>) {
        let Some(m) = self.slots.as_mut().map(|s| &mut s[i]) else { return };
        m.noise.draw(m.rect, pal, vignette, rng, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cvt_truncates_toward_zero() {
        for (x, want) in [(0.0f32, 0), (0.99, 0), (1.0, 1), (-1.5, -1), (255.99, 255), (-0.3, 0), (3e9, i32::MAX), (-3e9, i32::MIN)] {
            assert_eq!(cvt_w(x.to_bits()), want, "{x}");
        }
    }

    /// The projection is the game's pixel mapping: x = 256 + 256/(0.63·z)·(−y), y = 208 + 208/(0.48825·z)·(−z').
    #[test]
    fn menu_projection_matches_the_pinhole() {
        let p = MenuProjection::new(false);
        let at = |dy: f32, dz: f32, dx: f32| -> V4 { [256.0 + dx, 256.0 + dy, 64.0 + dz, 1.0].map(f32::to_bits) };
        let [x, y, w, h] = p.screen_rect(at(1.0, 1.0, 5.0), at(-1.0, -1.0, 5.0));
        let fx = 256.0 / (0.63 * 5.0);
        let fy = 208.0 / (0.63 * 0.775 * 5.0);
        assert_eq!((x, y), ((256.0 - fx) as i32, (208.0 - fy) as i32));
        assert!((w - (2.0 * fx) as i32).abs() <= 1 && (h - (2.0 * fy) as i32).abs() <= 1, "{w} {h}");
    }

    fn disc_class() -> Option<(FrameClass, crate::menus::Overlay)> {
        let root = rc_formats::test_data::root().join("levels/01");
        let blob = rc_formats::test_data::core_block(1, "moby_class/1138")?;
        let ov = crate::menus::Overlay::parse(&std::fs::read(root.join("overlay.bin")).ok()?).ok()?;
        let class = rc_formats::moby::parse_moby_class(&blob).ok()?;
        let anim = MobyAnimClass::new(&class, moby_anim::parse_sequences(&blob, &class).ok()?);
        let chains = std::array::from_fn(|k| {
            let id = ov.i32(CORNER_LISTS_ADDR + 4 * k as u32).unwrap() as usize;
            rc_formats::gadget::joint_list(&blob, &class.header, id).unwrap().0
        });
        Some((FrameClass { anim, chains, scale: class.header.scale }, ov))
    }

    /// Port-only: the Port Options pages' frames (Camera's, the front end's Sound) with the list panel stretched to
    /// `port::LIST_ROWS` once settled, growing with the open animation; the other panels as the game places them.
    #[test]
    fn port_list_panel_stretches_from_the_disc() {
        use super::super::port;
        let Some((class, ov)) = disc_class() else { return };
        let m = crate::menus::pause::PageMenu::load(&ov).unwrap();
        for (model, page) in [(port::MODEL_MANY, port::PAGE), (0x1b8e38, port::FRONT_PAGE)] {
            let seqs = m.pages[&model].seqs;
            let run = |stretch: bool| {
                let mut f = FrameMobys::new(class.clone(), false);
                f.spawn(&seqs);
                if stretch { f.stretch = port::panel_stretch(page); }
                for (i, &s) in seqs.iter().enumerate() { f.start(i, s, false); }
                let mut heights = Vec::new();
                for _ in 0..60 {
                    f.update();
                    heights.push(f.place(port::LIST_SLOT).unwrap()[3]);
                }
                ((0..SLOTS).map(|i| f.place(i).unwrap()).collect::<Vec<_>>(), heights)
            };
            let ((game, game_heights), (ported, heights)) = (run(false), run(true));
            let [_, y, _, h] = ported[port::LIST_SLOT];
            assert!((y - port::LIST_ROWS[0]).abs() <= 1 && (y + h - port::LIST_ROWS[1]).abs() <= 1, "{model:#x}: {:?}", ported[port::LIST_SLOT]);
            assert_eq!(ported[port::LIST_SLOT][0], game[port::LIST_SLOT][0]);
            assert_eq!(ported[port::LIST_SLOT][2], game[port::LIST_SLOT][2]);
            for i in (0..SLOTS).filter(|&i| i != port::LIST_SLOT) { assert_eq!(ported[i], game[i], "{model:#x} slot {i}"); }
            // From the parent page's panel (the game's own animation), never shorter than the game's on the way.
            assert!(heights.iter().zip(&game_heights).all(|(a, b)| a >= b), "{model:#x}: {heights:?} vs {game_heights:?}");        }
    }

    /// Class 1138 on Novalis: the root page's settled panels (skipped without `extracted/`).
    #[test]
    fn root_page_panels_from_the_disc() {
        let Some((class, ov)) = disc_class() else { return };
        assert_eq!(class.chains.iter().map(|c| *c.last().unwrap()).collect::<Vec<_>>(), vec![2, 1, 4, 3]);
        let mut f = FrameMobys::new(class, false);
        let seqs: [i32; SLOTS] = std::array::from_fn(|i| ov.i32(super::super::page::ROOT + 4 * i as u32).unwrap());
        assert_eq!(seqs, [14, 15, 16, 17, 18, 19, 20, 7, 8, 9, 10, 11, 12, 13]);
        f.spawn(&seqs);
        for (i, &s) in seqs.iter().enumerate() { f.start(i, s, true); }
        let mut rects = Vec::new();
        for t in 0..120 {
            f.update();
            rects = (0..SLOTS).map(|i| f.place(i).unwrap()).collect();
            if t % 10 == 0 || t < 12 { println!("tick {t}: {:?}", &rects[..8]); }
        }
        println!("settled: {rects:?}");
        // The seven root panels, stacked (the 7-frame seqs 14..20 stop on frame 0); seqs 7..13 (one frame,
        // rate 0: the advance never steps, so their speed stays −1) sit off screen.
        let st = f.slots.unwrap();
        assert!(st[..7].iter().all(|m| m.state.speed == 0.0 && m.state.frame_b == 0 && m.state.t == 1.0), "the open animations stop on frame 0 (key B at t = 1)");
        assert_eq!(rects[..7].iter().map(|r| r[1]).collect::<Vec<_>>(), vec![44, 93, 144, 193, 242, 292, 342]);
        assert!(rects[..7].iter().all(|r| (r[0], r[2], r[3]) == (155, 203, 35)));
    }
}
