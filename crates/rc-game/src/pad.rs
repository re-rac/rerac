//! The controller: libpad2 bytes → the game's `PAD` record (level01 `0x13c940`), exactly as
//! `UpdatePad__FR3PAD` (0x27bb30) and `ProcessPadInput__FR3PADPUci` (0x27bd90) build it, plus the
//! input-history queries the hero code uses (`0x27b940`, `0x27b9e0`, `0x27ba60`).
//! Spec: `docs/plan/player_controller.md` §1.
//!
//! **Exactness.** Every float step runs on the PS2 FPU/VU model ([`crate::ps2v::Pf`]): the axis
//! normalisation `cvt.s.w(d − 48) / cvt.s.w(76)`, the stick length (VU `vsqrt` of `x² + y²`,
//! `fun_001f9b20`) and angle ([`fast_arctan`], the VU0 polynomial of `FastArcTan__Fff` 0x2217c0 with its
//! octant table at 0x1c28c0) and the flick test (`FastDiffRots` 0x222100). Bit-exact by construction.
//!
//! **Not modelled:** the libpad2 connection state machine (mode/actuator setup of `UpdatePad`; the port
//! is handed either a data block or "disconnected"), the auto-repeat of PAD+0x34d..0x34f (0 in gameplay)
//! and the actuator bytes.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use crate::ps2v::Pf;

/// Button bits of `held`/`pressed`/`released` (`((b0 << 8) | b1) ^ 0xffff`, standard SCE layout).
pub mod button {
    pub const L2: u32 = 0x1;
    pub const R2: u32 = 0x2;
    pub const L1: u32 = 0x4;
    pub const R1: u32 = 0x8;
    pub const TRIANGLE: u32 = 0x10;
    pub const CIRCLE: u32 = 0x20;
    /// ✕: jump.
    pub const CROSS: u32 = 0x40;
    /// □: wrench.
    pub const SQUARE: u32 = 0x80;
    pub const SELECT: u32 = 0x100;
    pub const L3: u32 = 0x200;
    pub const R3: u32 = 0x400;
    pub const START: u32 = 0x800;
    pub const UP: u32 = 0x1000;
    pub const RIGHT: u32 = 0x2000;
    pub const DOWN: u32 = 0x4000;
    pub const LEFT: u32 = 0x8000;
    /// "Flick" (pressed mask only): the stick was slammed out or swung more than 55° (see [`super::PadState::process`]).
    pub const FLICK: u32 = 0x10000;
    /// Crouch = `held & (R1 | R2)`.
    pub const CROUCH: u32 = R1 | R2;
    /// Strafe = `held & (L1 | L2)`.
    pub const STRAFE: u32 = L1 | L2;
}

/// History depth of the pressed/angle/length rings (PAD+0x1e0/+0x258/+0x2d0).
pub const HISTORY: usize = 30;

const F0_9: Pf = Pf::b(0x3f66_6666); // 0.9
const F0_25: Pf = Pf::b(0x3e80_0000); // 0.25
const F55DEG: Pf = Pf::b(0x3f75_bcd0); // 0.9599311 rad (0x27c2f0 region: `lui 0x3f75; ori 0xbcd0`)
const PI: Pf = Pf::b(0x4049_0fdb);
/// 1/255 = 0.003921569 (pressure scale).
const INV255: Pf = Pf::b(0x3b80_8081);

/// The `PAD` record fields the game logic reads (offsets from PAD = level01 `0x13c940`).
#[derive(Clone, Debug)]
pub struct PadState {
    /// +0x100 / +0x104: right stick X/Y (after the dead zone; up and left negative).
    pub rx: Pf,
    pub ry: Pf,
    /// +0x108 / +0x10c (0x13ca48 / 0x13ca4c): **left stick** X/Y.
    pub lx: Pf,
    pub ly: Pf,
    /// +0x110..+0x13c: the 12 button pressures / 255 (libpad2 bytes 6..17: R, L, U, D, △, ○, ✕, □, L1, R1, L2, R2).
    pub pressure: [Pf; 12],
    /// +0x140..+0x17c: copy of +0x100..+0x13c before the mirror option is applied a second time (i.e.
    /// un-mirrored sticks).
    pub analog_copy: [Pf; 16],
    /// +0x1a0: held = buttons | stick-direction bits (lx < 0 → 0x8000, lx > 0 → 0x2000, ly < 0 → 0x1000, ly > 0 → 0x4000).
    pub held: u32,
    /// +0x1a4: pressed = held & !prev (+ [`button::FLICK`]).
    pub pressed: u32,
    /// +0x1a8: released = prev & !held.
    pub released: u32,
    /// +0x1ac: previous tick's held (from +0x348).
    pub prev_held: u32,
    /// +0x1b0 (0x13caf0): raw buttons only (the hero's d-pad fallback).
    pub raw: u32,
    /// +0x1b4 / +0x1b8: `!prev_held & raw` / `!held & prev_raw`.
    pub raw_pressed: u32,
    pub raw_released: u32,
    /// +0x1bc: previous raw.
    pub prev_raw: u32,
    /// +0x1c0 / +0x1c4 / +0x1c8: held / pressed / released with the mirror swap undone.
    pub held_unmirrored: u32,
    pub pressed_unmirrored: u32,
    pub released_unmirrored: u32,
    /// +0x1cc: one-shot input lock (1: clear sticks and ✕/△/○/d-pad-up-down bits; 2: keep only Start/Select).
    pub lock: i32,
    /// +0x1d0: no button held; +0x1d4: no d-pad/stick direction held.
    pub nothing_held: bool,
    pub no_direction: bool,
    /// +0x1d8: the left stick is off centre.
    pub stick_active: bool,
    /// +0x1e0: pressed-mask history.
    pub hist_pressed: [u32; HISTORY],
    /// +0x258: left-stick angle history (`FastArcTan(lx, ly)`).
    pub hist_angle: [Pf; HISTORY],
    /// +0x2d0: left-stick length history.
    pub hist_len: [Pf; HISTORY],
    /// +0x18e (s16): next history slot; +0x190: filled entries (≤ 30).
    pub hist_index: i16,
    pub hist_count: i32,
    /// +0x348: held, kept for the next tick's `prev_held`.
    pub held_store: u32,
}

impl Default for PadState {
    fn default() -> Self {
        PadState {
            rx: Pf::ZERO, ry: Pf::ZERO, lx: Pf::ZERO, ly: Pf::ZERO,
            pressure: [Pf::ZERO; 12], analog_copy: [Pf::ZERO; 16],
            held: 0, pressed: 0, released: 0, prev_held: 0, raw: 0, raw_pressed: 0, raw_released: 0, prev_raw: 0,
            held_unmirrored: 0, pressed_unmirrored: 0, released_unmirrored: 0, lock: 0,
            nothing_held: true, no_direction: true, stick_active: false,
            hist_pressed: [0; HISTORY], hist_angle: [Pf::ZERO; HISTORY], hist_len: [Pf::ZERO; HISTORY],
            hist_index: 0, hist_count: 0, held_store: 0,
        }
    }
}

/// One axis byte → float (`ProcessPadInput` loop at 0x27be40): `d = |b − 127|`; `d < 48 → 0` (the
/// field keeps the 0 written before the loop); else `min(cvt(d − 48) / cvt(76), 1)`, negated when `b < 127`.
pub fn axis(b: u8) -> Pf {
    let d = (b as i32 - 0x7f).abs();
    if d <= 0x2f { return Pf::ZERO; }
    let v = Pf::from_i32(d - 0x30) / Pf::from_i32(0x4c);
    let v = if Pf::ONE < v { Pf::ONE } else { v };
    if b < 0x7f { -v } else { v }
}

/// `fun_001f9b20` (0x221318): 2-D length on VU0: `vmul.xyz; vaddy.x` → `x² + y²`, `vsqrt`, `vaddq.x vf1, vf0, Q`.
pub fn len2(x: Pf, y: Pf) -> Pf { Pf::ZERO + ((x * x) + (y * y)).sqrt() }

/// `FastArcTan__Fff(a, b)` (level01 0x2217c0, boot 0x1f9e90) = atan2(b, a) in (−π, π]: the ratio
/// `x = (min − max) / (min + max)` of |a|, |b| (FPU), the odd polynomial to x¹⁵ on VU0 (coefficients at
/// 0x1c28a0/0x1c28b0, accumulated `((((((c0x + c1x³) + c2x⁵) + c3x⁷) + c4x⁹) + …) + c7x¹⁵` with each
/// term as `1·term`), then `(π/4 + p)·k + o` from the octant table at 0x1c28c0 indexed by
/// sign(|a| − |b|), sign(b), sign(a). Returns 0 when both are 0.
pub fn fast_arctan(a: Pf, b: Pf) -> Pf {
    const C: [u32; 8] = [0x3f7f_fff5, 0xbeaa_a61c, 0x3e4c_40a6, 0xbe0e_6c63, 0x3dc5_77df, 0xbd65_01c4, 0x3cb3_1652, 0xbb84_d7e7];
    // (k, o) per octant: offset 8·sign(|a|−|b|) + 16·sign(b) + 32·sign(a).
    const T: [(u32, u32); 8] = [
        (0x3f80_0000, 0), (0xbf80_0000, 0x3fc9_0fdb), (0xbf80_0000, 0), (0x3f80_0000, 0xbfc9_0fdb),
        (0xbf80_0000, 0x4049_0fdb), (0x3f80_0000, 0x3fc9_0fdb), (0x3f80_0000, 0xc049_0fdb), (0xbf80_0000, 0xbfc9_0fdb),
    ];
    let (mut f1, mut f2) = (a.abs(), b.abs());
    let d = f1 - f2;
    if !d.sign() { f1 = b.abs(); f2 = a.abs(); }
    if f2 <= Pf::ZERO { return Pf::ZERO; }
    let x = (f1 - f2) / (f1 + f2);
    let i = (d.sign() as usize) | (b.sign() as usize) << 1 | (a.sign() as usize) << 2;
    let (k, o) = (Pf(T[i].0), Pf(T[i].1));
    // vf1 = (x, x, x, x) (`qmtc2`, `vaddx.yz vf0`, `vmulx.w vf0`: every lane exactly x), then
    // vf1 = (x, x·x², x·x⁴, (x·x⁴)·x²) and vf2 = vf1·x⁸ with x² = x·x, x⁴ = x²·x², x⁸ = x⁴·x⁴.
    let x2 = x * x;
    let x4 = x2 * x2;
    let x8 = x4 * x4;
    let x5 = x * x4;
    let v1 = [x, x * x2, x5, x5 * x2];
    let t1 = [0, 1, 2, 3].map(|k| v1[k] * Pf(C[k]));
    let t2 = [0, 1, 2, 3].map(|k| (v1[k] * x8) * Pf(C[4 + k]));
    let mut acc = t1[0] + t1[1];
    for t in [t1[2], t1[3], t2[0], t2[1], t2[2], t2[3]] { acc = acc + Pf::ONE * t; }
    ((Pf::b(0x3f49_0fdb) + acc) * k) + o
}

/// `FastDiffRots__Fff` (0x222100): `|a − b|`, and `2π − that` when it is ≥ π.
pub fn fast_diff_rots(a: Pf, b: Pf) -> Pf {
    let d = (a - b).abs();
    if d < PI { d } else { (PI + PI) - d }
}

/// Swap LEFT (0x8000) and RIGHT (0x2000) as the mirror option does.
fn mirror_lr(m: u32) -> u32 {
    if m & 0x8000 != 0 { m & !0x8000 | 0x2000 } else if m & 0x2000 != 0 { m & !0x2000 | 0x8000 } else { m }
}

impl PadState {
    /// **Port-only**: this pad as if the buttons of `mask` were never pressed (every held / pressed / released
    /// word, their raw and un-mirrored copies, the pressed history and their pressures). The hero sees this
    /// view while the Port Options strafe owns L2 / R2 (`crate::hero::strafe`).
    pub fn without(&self, mask: u32) -> PadState {
        let mut p = self.clone();
        for w in [
            &mut p.held, &mut p.pressed, &mut p.released, &mut p.prev_held, &mut p.raw, &mut p.raw_pressed,
            &mut p.raw_released, &mut p.prev_raw, &mut p.held_unmirrored, &mut p.pressed_unmirrored,
            &mut p.released_unmirrored, &mut p.held_store,
        ] {
            *w &= !mask;
        }
        for h in p.hist_pressed.iter_mut() { *h &= !mask; }
        // The pressure bytes 6..17 are R, L, U, D, △, ○, ✕, □, L1, R1, L2, R2.
        if mask & button::L2 != 0 { p.pressure[10] = Pf::ZERO; }
        if mask & button::R2 != 0 { p.pressure[11] = Pf::ZERO; }
        p.nothing_held = p.held == 0;
        p
    }

    /// `UpdatePad__FR3PAD` (0x27bb30) for one tick. `data` = the libpad2 read (`sceScfPad2Read`: bytes 0/1
    /// buttons active-low, 2..5 = rx, ry, lx, ly, 6..17 pressures) or `None` when the pad is not
    /// connected / not in a readable state (→ `ClearPadInput`). `mirror` = option byte 0x15edb4.
    pub fn update(&mut self, data: Option<&[u8]>, mirror: bool) {
        self.held = 0;
        self.prev_raw = self.raw;
        self.raw = 0;
        self.prev_held = self.held_store;
        match data {
            Some(d) => self.process(d, mirror),
            None => self.clear(),
        }
    }

    /// `ClearPadInput__FR3PAD` (0x27bd28).
    pub fn clear(&mut self) {
        self.raw = 0;
        self.no_direction = true;
        self.held = 0;
        self.pressed = 0;
        self.released = 0;
        self.nothing_held = true;
        self.raw_pressed = 0;
        self.raw_released = 0;
        self.held_unmirrored = 0;
        self.pressed_unmirrored = 0;
        self.released_unmirrored = 0;
        self.stick_active = false;
        self.rx = Pf::ZERO; self.ry = Pf::ZERO; self.lx = Pf::ZERO; self.ly = Pf::ZERO;
        self.pressure = [Pf::ZERO; 12];
        self.analog_copy = [Pf::ZERO; 16];
    }

    /// `ProcessPadInput__FR3PADPUci` (0x27bd90): buttons, axes (only when the block has more than 5 bytes),
    /// pressures (more than 17), the mirror option, the stick-direction bits, pressed/released masks,
    /// the lock modes, the 30-deep histories and the flick bit.
    pub fn process(&mut self, d: &[u8], mirror: bool) {
        let b = (((d[0] as u32) << 8) | d[1] as u32) ^ 0xffff;
        self.raw = b;
        self.held = b;
        let mut an = [Pf::ZERO; 16];
        if d.len() > 5 {
            for (k, v) in an.iter_mut().take(4).enumerate() { *v = axis(d[2 + k]); }
        }
        if d.len() > 0x11 {
            for k in 0..12 { an[4 + k] = Pf::from_i32(d[6 + k] as i32) * INV255; }
        }
        if mirror {
            an[2] = -an[2];
            an[0] = -an[0];
            self.held = mirror_lr(self.held);
            self.raw = self.held;
        }
        self.analog_copy = an;
        [self.rx, self.ry, self.lx, self.ly] = [an[0], an[1], an[2], an[3]];
        self.pressure.copy_from_slice(&an[4..]);
        self.stick_active = !(self.lx == Pf::ZERO && self.ly == Pf::ZERO);
        if self.lx < Pf::ZERO { self.held |= 0x8000; }
        if Pf::ZERO < self.lx { self.held |= 0x2000; }
        if self.ly < Pf::ZERO { self.held |= 0x1000; }
        if Pf::ZERO < self.ly { self.held |= 0x4000; }
        let held = self.held;
        let pressed = !self.prev_held & held;
        let released = !held & self.prev_held;
        self.no_direction = held & 0xf000 == 0;
        self.nothing_held = held == 0;
        self.raw_pressed = !self.prev_held & self.raw;
        self.raw_released = !held & self.prev_raw;
        self.pressed_unmirrored = pressed;
        self.released_unmirrored = released;
        self.pressed = pressed;
        self.released = released;
        self.held_unmirrored = held;
        self.held_store = held;
        if mirror {
            self.analog_copy[2] = -self.analog_copy[2];
            self.analog_copy[0] = -self.analog_copy[0];
            self.held_unmirrored = mirror_lr(self.held_unmirrored);
            self.pressed_unmirrored = mirror_lr(self.pressed_unmirrored);
            self.released_unmirrored = mirror_lr(self.released_unmirrored);
        }
        if self.lock == 1 {
            self.held &= 0xffff_afcf;
            self.pressed &= 0xffff_afcf;
            self.released &= 0xffff_afcf;
            self.rx = Pf::ZERO;
            self.ry = Pf::ZERO;
            self.lock = 0;
        }
        if self.lock == 2 {
            self.held &= 0x900;
            self.pressed &= 0x900;
            self.released &= 0x900;
            self.raw &= 0x900;
            self.no_direction = true;
            self.lx = Pf::ZERO;
            self.ly = Pf::ZERO;
            self.lock = 0;
        }
        // Histories and the flick bit.
        let len = len2(self.lx, self.ly);
        let ang = fast_arctan(self.lx, self.ly);
        let idx = self.hist_index as i32;
        let at = |i: i32| ((idx - i + 0x1e) % 0x1e) as usize;
        self.hist_len[idx as usize] = len;
        self.hist_angle[idx as usize] = ang;
        if F0_9 < len {
            // Slammed out: longer than 0.9 now, below 0.25 within the last 3 ticks (a > 0.9 on the way stops the search).
            for i in 1..4 {
                let v = self.hist_len[at(i)];
                if F0_9 < v { break; }
                if v < F0_25 { self.pressed |= button::FLICK; break; }
            }
            if self.pressed & button::FLICK == 0 {
                // Swung: the angle moved by more than 55° within the last 4 ticks.
                for i in 1..5 {
                    if F55DEG < fast_diff_rots(self.hist_angle[at(i)], ang) {
                        self.pressed |= button::FLICK;
                        break;
                    }
                }
            }
        }
        self.hist_pressed[idx as usize] = self.pressed;
        self.hist_count += 1;
        self.hist_index = ((self.hist_index as i32 + 1) % 0x1e) as i16;
        if self.hist_count > 0x1e { self.hist_count = 0x1e; }
    }

    /// `PressedWithin(mask, n, *ago)` 0x27b940: was any bit of `mask` pressed within the last
    /// `min(n, count)` ticks, this tick included? Returns the age (0 = this tick) of the newest press.
    pub fn pressed_within(&self, mask: u32, n: i32) -> Option<i32> {
        let m = n.min(self.hist_count);
        (0..m).find(|&i| self.hist_pressed[self.slot_ago(i)] & mask != 0)
    }

    /// `Combo(a, b, n, gap)` 0x27b9e0: both pressed within the last `n` ticks, strictly less than `gap`
    /// ticks apart.
    pub fn combo(&self, a: u32, b: u32, n: i32, gap: i32) -> bool {
        let (Some(ta), Some(tb)) = (self.pressed_within(a, n), self.pressed_within(b, n)) else { return false };
        (ta - tb).abs() < gap
    }

    /// `StickLenAgo(n)` 0x27ba60: the stick length `min(n, count) − 1` ticks ago (n = 1: this tick).
    pub fn stick_len_ago(&self, n: i32) -> Pf { self.hist_len[self.slot_ago(n.min(self.hist_count) - 1)] }

    /// The history slot `i` ticks back (0 = this tick's entry).
    fn slot_ago(&self, i: i32) -> usize { ((self.hist_index as i32 - 1 - i).rem_euclid(0x1e)) as usize }
}

/// Test/engine helper: the 18 libpad2 bytes of one read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadInput {
    /// Buttons held (active high here; stored active-low in the block).
    pub buttons: u16,
    /// Stick bytes: 127 = centre, 0 = up/left, 255 = down/right.
    pub rx: u8,
    pub ry: u8,
    pub lx: u8,
    pub ly: u8,
    /// Pressure bytes (R, L, U, D, △, ○, ✕, □, L1, R1, L2, R2).
    pub pressure: [u8; 12],
}

impl Default for PadInput {
    fn default() -> Self { PadInput::neutral() }
}

impl PadInput {
    /// Sticks centred, nothing pressed.
    pub const fn neutral() -> Self { PadInput { buttons: 0, rx: 0x7f, ry: 0x7f, lx: 0x7f, ly: 0x7f, pressure: [0; 12] } }

    /// A stick value in [−1, 1] (per axis; up/left negative) → the byte that decodes to it: `127 ± (48 + 76·|v|)`,
    /// rounded to the nearest byte; |v| < 0.0066 gives the centre.
    pub fn axis_byte(v: f32) -> u8 {
        let d = (48.0 + 76.0 * v.abs().min(1.0)).round() as i32;
        if v.abs() < 0.5 / 76.0 { return 0x7f; }
        (if v < 0.0 { 127 - d } else { 127 + d }).clamp(0, 255) as u8
    }

    /// Left stick (x right, y down; so "forward" is `stick(0.0, -1.0)`).
    pub fn stick(mut self, x: f32, y: f32) -> Self {
        self.lx = Self::axis_byte(x);
        self.ly = Self::axis_byte(y);
        self
    }

    /// Right stick.
    pub fn rstick(mut self, x: f32, y: f32) -> Self {
        self.rx = Self::axis_byte(x);
        self.ry = Self::axis_byte(y);
        self
    }

    /// Hold `mask` (a [`button`] bit set; digital, pressure 255 for the pressure buttons).
    pub fn press(mut self, mask: u32) -> Self {
        self.buttons |= mask as u16;
        const ORDER: [u32; 12] = [button::RIGHT, button::LEFT, button::UP, button::DOWN, button::TRIANGLE, button::CIRCLE, button::CROSS, button::SQUARE, button::L1, button::R1, button::L2, button::R2];
        for (k, b) in ORDER.iter().enumerate() {
            if mask & b != 0 { self.pressure[k] = 0xff; }
        }
        self
    }

    /// The libpad2 data block (`((b0 << 8) | b1) ^ 0xffff` = buttons).
    pub fn bytes(&self) -> [u8; 18] {
        let b = !self.buttons;
        let mut o = [0u8; 18];
        o[0] = (b >> 8) as u8;
        o[1] = b as u8;
        o[2] = self.rx;
        o[3] = self.ry;
        o[4] = self.lx;
        o[5] = self.ly;
        o[6..].copy_from_slice(&self.pressure);
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_dead_zone_and_scale() {
        assert_eq!(axis(127).0, 0);
        assert_eq!(axis(80).0, 0); // d = 47
        assert_eq!(axis(0), Pf::b(0xbf80_0000)); // d = 127 → 79/76 clamped to 1, negated
        assert_eq!(axis(251), Pf::ONE); // d = 124 → 76/76
        // 200: d = 73 → 25/76 = 0.32894737 (div.s truncated)
        assert_eq!(axis(200).0, 0x3ea8_6bca);
        assert!((axis(200).to_f32() - 25.0 / 76.0).abs() < 1e-7);
        assert_eq!(axis(54), -axis(200));
    }

    #[test]
    fn arctan_matches_atan2() {
        for &(a, b) in &[(1.0f32, 0.0f32), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0), (1.0, 1.0), (-0.3, 0.8), (0.5, -0.25), (-0.7, -0.7), (0.9, -1.0)] {
            let r = fast_arctan(Pf::f(a), Pf::f(b)).to_f32();
            assert!((r - b.atan2(a)).abs() < 2e-5, "atan2({b}, {a}) = {} vs {r}", b.atan2(a));
        }
        assert_eq!(fast_arctan(Pf::ZERO, Pf::ZERO).0, 0);
        // The polynomial does not cancel π/4 exactly: atan2(0, 1) is 2^-22·0.75, not 0 (a property of the game's routine).
        assert_eq!(fast_arctan(Pf::ONE, Pf::ZERO).0, 0x3440_0000);
    }

    #[test]
    fn buttons_pressed_released_and_stick_bits() {
        let mut p = PadState::default();
        let a = PadInput::neutral().press(button::CROSS).stick(0.0, -1.0);
        p.update(Some(&a.bytes()), false);
        assert_eq!(p.held, button::CROSS | button::UP);
        assert_eq!(p.pressed & 0xffff, button::CROSS | button::UP);
        assert_eq!(p.raw, button::CROSS);
        assert_eq!(p.ly, Pf::b(0xbf80_0000));
        p.update(Some(&a.bytes()), false);
        assert_eq!(p.pressed & 0xffff, 0);
        p.update(Some(&PadInput::neutral().bytes()), false);
        assert_eq!(p.released, button::CROSS | button::UP);
        assert_eq!(p.pressed_within(button::CROSS, 7), Some(2));
        assert_eq!(p.pressed_within(button::CROSS, 2), None);
    }

    #[test]
    fn flick_from_rest() {
        let mut p = PadState::default();
        p.update(Some(&PadInput::neutral().bytes()), false);
        p.update(Some(&PadInput::neutral().stick(1.0, 0.0).bytes()), false);
        assert_ne!(p.pressed & button::FLICK, 0);
        p.update(Some(&PadInput::neutral().stick(1.0, 0.0).bytes()), false);
        assert_eq!(p.pressed & button::FLICK, 0);
        // Swing from right to up (90°) at full length: angle flick.
        p.update(Some(&PadInput::neutral().stick(0.0, -1.0).bytes()), false);
        assert_ne!(p.pressed & button::FLICK, 0);
    }

    #[test]
    fn mirror_swaps_left_right() {
        let mut p = PadState::default();
        p.update(Some(&PadInput::neutral().press(button::LEFT).stick(1.0, 0.0).bytes()), true);
        assert_eq!(p.lx, Pf::b(0xbf80_0000));
        assert_eq!(p.raw, button::RIGHT);
        assert_eq!(p.held, button::RIGHT | button::LEFT); // raw right + stick-left bit
    }
}
