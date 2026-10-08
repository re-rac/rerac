//! The in-game HUD: the 13-slot element system, health / bolts / weapon elements, the banner, the help box and
//! the game's word-wrap layout (`FontPrintWindow`). Spec: docs/plan/hud_text.md §2–§4; addresses are level01.elf.
//!
//! [`HudState::tick`] runs one 60 Hz game tick (the HUD update loop 0x24f880; the help box's `Help_Update` 0x225bd0 is
//! `crate::help`'s, the HUD draws its box: [`HudState::set_help`]) and returns
//! the frame's draw calls in the game's order: slots 0..12 (`HudDraw` 0x24fb50), the banner, the help box
//! (0x2266c0). A [`Draw`] is one call of the game's 2D layer (`HudSprite` family, `FontPrint`,
//! `FontPrintWindow`, `DrawUIFrame`, `DrawTexturedQuad`); the engine turns them into GS primitives.
//!
//! **Slots** (13 × 0x90 at 0x17e0d0). `queue_animation_update` (0x24ad98) stores an element request as
//! *pending* when any argument differs from the last request (and then zeroes the slot's timer and ramps); the
//! loop applies it (`apply_pending_animation` 0x24aea8: copy, call the element's init) once the slot's
//! visibility counter (+0x6c) has come down to −6. Every tick the loop: bumps the timer (+0x7c) to at least
//! `ScaleTicks(10)` for a persistent element (flag 0x10); decrements it; counts +0x6c up to 30 while it is
//! ≥ 1 after the decrement, else down to −6; applies a pending element; runs the element's update. Updates ramp
//! **slide** (+0x70) 0→steps, then **alpha** (+0x71) 0→steps, one step per tick while `timer ≥ ScaleTicks(5)`,
//! and back (alpha first, then slide, then +0x6c = −6) below it. Draws use `s = slide/steps`, `f = alpha/steps`.
//!
//! **Constants** are the overlay's small-data values (gp = 0x166c00) and the boot ELF's defaults: rate factor
//! 0x15ed68 = 1.0, PAL flag 0x15ed80 = 0, max HP 0x15eda0 = 4, help text / voice options 0x15ee1d / 0x15ee1c = 1.
//!
//! **A system: the slot machine is general; consumers only queue.** Every element is a request (the [`Request`]: slot,
//! flags, icon, element, max); game code records its calls in [`calls::Calls`] (`Services::hud`, the classes) or the
//! engine's `HudFeed::calls` (the menus), the HUD replays them ([`HudState::apply_calls`]); callers the port only sees
//! through the state (the hero's) are derived per tick ([`HudState::tick`]'s `frame_calls`). The generic pieces
//! (`0x24b418` init, `0x24b318` value + size, `0x24b538` / `0x24b7c0` updates, `0x24b108` / `0x24b170` placement) are
//! shared by every meter. A boss is `Calls::boss_meter(key, value, max)` each tick; a new element is a variant of
//! [`Element`] with its init / update / draw. Not a system: the per-level elements (level 02's counters `0x23c990`, the
//! races' slots 0 / 5 / 7 of levels 05 / 16) are their levels' own code (filed with their consumers).
//!
//! **Coverage (G-UI-011, 2026-10-01; level01 unless noted)**
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x24ad98 `queue_animation_update` | compare (data, max, icon, flags, init, update, draw), new handle, pending, timer / ramps 0, flag 0x20 applies at once | [`HudState::queue_req`] |
//! | 0x24ad98 | game mode 0x15f5c4 = 5: only slots 0 / 2 take a request (others return 0) | `queue_req` (`Inputs::mode`) |
//! | 0x24afa0 `fun_001ff480` | release a handle: the empty request (max 0) in its slot | [`HudState::release`] |
//! | 0x24b090 / 0x24b4b0 | set flags / keep up by handle | `set_flags` / `keep_up`; by request: [`Call::SetFlags`] / [`Call::KeepUp`] |
//! | 0x24b020 `load_animation_definition`, 0x24f6b0 icon animation | icon index, anim mode, frame +0x44 | n/a: every icon these elements use has anim mode 0 (one frame) or none (0xffff), and no ported draw reads +0x44 |
//! | 0x24b318 | value = min(*data, max) (no data: 99999); the size grows with max's digits by the anchor bits | `init_value` |
//! | 0x24b418 / 0x24b458 | generic init (offset 0, timer `ScaleTicks(180) + 30`; 0x24b458 also 32 × 32) | `init_generic` / `init_alert` |
//! | 0x24b538 | value from data, timer 180 on a change, the shown value counting (√ step, byte accumulator) once in (+0x6c > 23), the ramp (no −6) | `update_counting`, `ramp_generic` |
//! | 0x24b7c0 | as 0x24b538, the shown value jumps | `update_immediate` |
//! | 0x24b108 / 0x24b170 | placement from the static anchor / size / bits; the slide tables 0x17e820 / 0x17e880 by +0x6c | `align`, `place` |
//! | 0x24c878 | the prompt's update = 0x24b538 (was a ramp with −6) | `update_slots` |
//! | 0x24c898 best-time lines | levels 5 / 16 within 5 of the start: banner y + 100, best time / score banners (msgs 0x531b / 0x531c) | `draw_best_times` (the records 0x15ee58.. are written by the races: `Inputs::race_best_*`, G-LVL-007) |
//! | 0x24b900 / 0x24baf0 / 0x24bc40 | oxygen meter: bubbles init (rand order x, y, vx, vy ×3), step (every other, bounces), the fill 0x80684c2f, blink ≤ ⅓ (45 of 60 VSyncs) + Ratchet's sound 11 at each lit start, tank `HudSpriteRot90`, bubbles `fun_00200080` below the level | `bubbles_init`, `bubbles_step`, `draw_tank(…, true)`; the sound: `sounds` → engine |
//! | 0x24c1a8 / 0x24c398 / 0x24c4e8 | Morph-o-Ray meter (same bubbles; 0x24b7c0; no fill / blink; all bubbles size 12) | `draw_tank(…, false)` |
//! | 0x24ce30 / 0x24ce50 / 0x24ced8 | Suck Cannon count: max 5 / 10 gold, timer held while in hand on foot else ≤ 30; black, fill 0x80829e00 from the bottom, black lines per held, tank | `update_suck`, `draw_suck` |
//! | 0x24b458 / 0x24ee20 / 0x24eed8 | bolt alert: flag-driven ramp; nothing in body 2 / state 0x32 / slide 0; bar, spinning bolt, pulsing "!" | `init_alert`, `update_alert`, `draw_alert` |
//! | 0x227d90 `HudBoltAlertShow` | queues element 7 with the Metal Detector owned and a cache within 20 | `classes::buried_bolts::alert_frame` → `Services::hud` |
//! | 0x24f9c0 body 2 | Giant Clank's energy: `0x24f248` (sub-rect bar `221·shown/200 + 27`, frame, beam light) | `update_weapon_request`, `draw_giant` |
//! | 0x24f9c0 `iGpffff8d68` | weapon show skipped after `FUN_0024fb00` until `update_resource_counter` | n/a: the port's only reset is the vendor's open, whose own slot-0 requests (`HudFeed::weapon`) drive slot 0 until its exit |
//! | 0x231348 leave body 2 | `FUN_0024b090(0x140984, 0)` | `frame_calls` (body ≠ 2 → flags 0) |
//! | level18 0x23cd60 (06 0x249738, 07, 13) | boss meter: n segments (3; 6 on 07 / 13; 7 on 18), level 13 at the top, the squeezed ends, red → yellow → green fill | [`Element::Boss`], `draw_boss`; consumers: `Calls::boss_meter` (level06 0x2f9a28 class 1051, level18 0x2f7288 for 1422: G-CLS-001) |
//! | 0x226fa8 `HeroTakeDamage` → 0x24a498 | health queued on damage only (a gain is shown by the element already in slot 1) | `frame_calls` (health went down) |
//! | 0x2aba68 L3 | outside 0x1d / 0x32: health + bolts, keep-up 180 | `frame_calls` → `show_health_bolts` |
//! | 0x242930 △ / 0x28c6c8 `PageMenuClose` | health + bolts, keep-up 180 | engine `menu_render` → `Call::ShowHealthBolts` |
//! | 0x2406b0 | oxygen meter queued every tick under water (group 0x11) without the O2 mask | `frame_calls` |
//! | 0x303000 | Suck Cannon meter queued every tick of its update | `frame_calls` (`Inputs::suck_active`) |
//! | 0x2d2450 | Morph meter queued on a new target, released when the ray stops | `frame_calls` (`Inputs::morph_*`) |
//! | `randf` 0x26c9c8 in the bubble inits | the game's one stream | NOT ported: a HUD-local newlib stream (G-UI-011) |
//! | 0x27b028 `NpcTalkUpdate` | bolt counter queued and kept up `ScaleTicks(60)` for NPC nodes of kind 1 / 6 | `interact::talk_update_at` → `Services::hud` |
//! | 0x2bc4f0 `CollectBolt` | the bolt counter queued | `classes::bolt` → `Services::hud` (and the count's change, `frame_calls`) |
//! | 0x24fb50 second banner line 0x1795e8 (countdown > 1000) | | NOT ported (G-UI-011) |
//! | level05 0x24cee8 / level16 0x21e398 race HUD, slots 5 / 7 (`0x2661a0` init, `0x2661e8` update, draws `0x266320` / `0x266710`) | lap and place, time (and the score once the race is won), on the board only; level 16's board-weapon count box (0x238a30) | [`Element::RaceLap`] / [`Element::RaceTime`] (queued by the board's 0x6b entry: `crate::hero::hoverboard`) |
//! | level05 race HUD slot 0 \| 0x10 (`0x262ae8` / `0x262b58` / `0x262f50`, data 0x13fbb4) | the boost meter, the trick-combo texts, the wrong-way warning (time trials) | [`Element::RaceMeter`] (its game part: `crate::hero::hoverboard::meter_tick`) |
//! | level02 0x2e1fb8 counters (slots 5 / 7, icons 2000 / 2001, `0x23c990` / `0x23ccc8` / `0x23cda8`) | | NOT ported (G-UI-011, level 02's own element) |

use rc_formats::font::{measure_text_width, Font, GlyphTable};
use rc_formats::hud::{Hud, IconEntry};
use rc_formats::strings::{self, Message};
use std::collections::HashMap;

pub mod calls;
mod meters;

pub use calls::{Call, Calls};

/// Frame buffer size (NTSC, `0x13e500` / `0x13e504` at run time).
pub const SCREEN_W: i32 = 512;
pub const SCREEN_H: i32 = 416;
/// Element slots.
pub const SLOTS: usize = 13;

/// `ScaleTicks` (0x220e30): `(int)(n · rate + 0.5)`, rate = 1.0 on NTSC.
pub fn scale_ticks(n: i32) -> i32 { (n as f32 * 1.0 + 0.5) as i32 }

/// `FastTweenColor` (0x2221a8): per byte `a·(1−t) + b·t` on VU0 floats, truncated.
pub fn tween_color(t: f32, a: u32, b: u32) -> u32 {
    let mut out = 0u32;
    for k in 0..4 {
        let (ca, cb) = (((a >> (8 * k)) & 0xff) as f32, ((b >> (8 * k)) & 0xff) as f32);
        out |= ((ca * (1.0 - t) + cb * t) as i32 as u32 & 0xff) << (8 * k);
    }
    out
}

/// Static slot anchors (+0x50 / +0x54, level01 .data 0x17e0d0; never written). Only x is read by the elements
/// ported here; each element has its own y.
pub const ANCHORS: [(i32, i32); SLOTS] = [
    (20, 15), (256, 32), (492, 15), (20, 208), (492, 208), (20, 376), (256, 376), (492, 376),
    (492, 208), (492, 208), (492, 208), (492, 208), (142, 50),
];

// Small-data constants (gp-relative, level01 overlay section 0).
/// gp−0x7410 0x15f7f0: health ramp steps.
const HEALTH_STEPS: i32 = 8;
/// 0x15f7f8 / 0x17e8e0 / 0x17e8f8: orb offsets for 4, 5 and 8 orbs.
const ORBS_4: [(i32, i32); 4] = [(-80, 0), (-48, 0), (-16, 0), (16, 0)];
const ORBS_5: [(i32, i32); 5] = [(-64, 0), (-32, 0), (0, 0), (-48, 32), (-16, 32)];
const ORBS_8: [(i32, i32); 8] = [(-80, 0), (-48, 0), (-16, 0), (16, 0), (-80, 32), (-48, 32), (-16, 32), (16, 32)];
/// 0x15f808 health y base (+18 NTSC, +10 PAL), 0x15f80c second bar row, 0x15f810 bar y offset,
/// 0x15f81c bar tile size, 0x15f820 health bar alpha.
const HEALTH_Y: i32 = 14;
const BAR_ROW_DY: i32 = 16;
const BAR_DY: i32 = -16;
const BAR_TILE: i32 = 32;
const HEALTH_BAR_ALPHA: i32 = 20;
/// 0x15f824 / 0x15f828 bolt ramp steps, 0x15f82c / 0x15f830 text colour tween, 0x15f834 / 0x15f838 text offset,
/// 0x15f83c.. separator offsets ("'" / "."), 0x15f84c digit width.
const BOLT_SLIDE_STEPS: i32 = 8;
const BOLT_ALPHA_STEPS: i32 = 8;
const TEXT_FROM: u32 = 0x00e0_8060;
const TEXT_TO: u32 = 0x80e0_8060;
const BOLT_TEXT_D: (i32, i32) = (-32, 8);
const SEP_APOSTROPHE_D: (i32, i32) = (-30, 21);
const SEP_DOT_D: (i32, i32) = (-29, 10);
const DIGIT_W: i32 = 13;
/// 0x15f91c / 0x15f920 weapon ramp steps, 0x15f924 / 0x15f928 bar length (max ammo < 100 / ≥ 100),
/// 0x15f934 / 0x15f938 empty-ammo colour tween, 0x15f93c / 0x15f940 text offset.
const WEAPON_STEPS: i32 = 8;
const WEAPON_LEN: [i32; 2] = [70, 95];
const EMPTY_FROM: u32 = 0x0020_2080;
const EMPTY_TO: u32 = 0x8020_2080;
const WEAPON_TEXT_D: (i32, i32) = (25, 8);
/// Element y on NTSC (the code picks 10 on PAL).
const ELEMENT_Y: i32 = 18;

/// Icon ids (hud_text.md §1.3).
pub mod icon {
    pub const HEALTH_SLOT: u16 = 30005;
    pub const ORB: u16 = 30006;
    pub const BOLT_SLOT: u16 = 30030;
    pub const BOLT: u16 = 30031;
    pub const BAR: u16 = 30080;
    pub const ITEM_BASE: u16 = 60000;
}

/// The icon and frame-size tables the HUD code reads (`GetIconFrame`, `HudFrame`), the glyph tables and the
/// level's messages.
#[derive(Clone, Debug)]
pub struct HudAssets {
    pub icons: Vec<IconEntry>,
    /// Texture size of every frame (power of two), (0, 0) when unknown.
    pub frame_sizes: Vec<(i32, i32)>,
    pub glyphs: [GlyphTable; 3],
    pub messages: Vec<Message>,
}

impl HudAssets {
    pub fn new(hud: &Hud, glyphs: [GlyphTable; 3], messages: Vec<Message>) -> Self {
        let frame_sizes = (0..hud.frames.len()).map(|i| hud.frame_size(i).map_or((0, 0), |(w, h)| (w as i32, h as i32))).collect();
        HudAssets { icons: hud.icons.clone(), frame_sizes, glyphs, messages }
    }

    /// `GetIconFrame__Fii` with every bank resident (rc_formats::hud::Hud::icon_frame).
    pub fn icon_frame(&self, id: u16, k: i32) -> usize {
        let i = self.icons.iter().position(|e| e.id == 0xffff || e.id == id).unwrap_or(self.icons.len().saturating_sub(1));
        let Some(e) = self.icons.get(i) else { return 0 };
        if e.id == 0xffff || k >= e.frame_count as i32 { return 0; }
        (e.first_frame as i32 + k).max(0) as usize
    }

    fn size(&self, frame: usize) -> (i32, i32) { self.frame_sizes.get(frame).copied().unwrap_or((0, 0)) }

    fn glyphs(&self, font: Font) -> &GlyphTable { &self.glyphs[font as usize] }

    /// `FUN_0021cc90`-style width of `text` (to the NUL) in `font`.
    pub fn text_width(&self, font: Font, text: &[u8]) -> i32 { measure_text_width(text, -1, self.glyphs(font)) }
}

/// Which `HudSprite` variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rot {
    /// 0x2500e0: SPRITE, UV (0,0)..(tw,th).
    None,
    /// 0x250468: TRISTRIP with the texture rotated 180° (right bar caps).
    R180,
    /// 0x2506c8: TRISTRIP with the texture rotated 90°.
    R90,
}

/// One call of the game's 2D layer, in draw order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Draw {
    /// `HudSprite` (frame, x, y, w, h, alpha): the whole frame texture stretched over `w×h`, RGBAQ =
    /// `alpha << 24 | 0x7f7f7f` (alpha may exceed 0x80).
    Sprite { frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32, rot: Rot },
    /// `FontPrint(x, y, rgba, text, -1)` in `font` (right / centre wrappers already applied to `x`).
    Text { font: Font, x: i32, y: i32, rgba: u32, text: Vec<u8> },
    /// `FontPrintWindow(window, rgba, text, -1)` in `font`.
    TextWindow { font: Font, window: text::Window, rgba: u32, text: Vec<u8> },
    /// `DrawUIFrame(top, bottom, left, right, alpha)` (0x21c958).
    UiFrame { top: i32, bottom: i32, left: i32, right: i32, alpha: i32 },
    /// `DrawTexturedQuad(x, y, w, h, u, v, tw, th, rgba, GetEffectTex(fx))` (0x21be90).
    FxQuad { fx: usize, x: i32, y: i32, w: i32, h: i32, u: i32, v: i32, tw: i32, th: i32, rgba: u32 },
    /// `fun_00200080` (0x250928): `HudSprite` with the corner and size in 1/16 pixels (the tank meters' bubbles).
    Sprite16 { frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32 },
    /// `fun_00200e08(x0, y0, x1, y1, rgba, 1)` (0x2516b0): an untextured blended SPRITE between two corners in 1/16
    /// pixels (sent as `v + OF − 8`, the sprites' convention; the whole-pixel form `(…, 0)` is `16·v − 8` here).
    Rect16 { x0: i32, y0: i32, x1: i32, y1: i32, rgba: u32 },
    /// `HudSpriteSubRect` (0x2502c8): the first `w × h` texels of the frame at their own size (UV (0,0)..(w,h)).
    SpriteSub { frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32 },
}

/// What a slot shows (the init / update / draw callbacks + data pointer of `queue_animation_update`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Element {
    #[default]
    Empty,
    /// 0x24e1b8 / 0x24e238 / 0x24e418, data 0x1415f8 (HP), max 8.
    Health,
    /// 0x24e8d0 / 0x24e908 / 0x24ea00, data 0x15ed98 (bolts), max 9999999.
    Bolts,
    /// 0x24f368 / 0x2519c0 / 0x24f3b0, data 0x13d428 + 4·item (ammo), max = item table +6.
    Weapon { item: u16 },
    /// The context prompt, slot 12: init 0x24c828, update 0x24c878 (the generic ramp 0x24b538), draw 0x24c898
    /// (Lombyte `HudRaceTimerDraw`), no data (docs/plan/interaction.md §2).
    Prompt,
    /// The oxygen meter, slot 4: init 0x24b900, update 0x24baf0, draw 0x24bc40, data 0x1415f0, max 10000, icon 30015.
    Oxygen,
    /// The Morph-o-Ray's meter, slot 4: init 0x24c1a8, update 0x24c398, draw 0x24c4e8, data gp−0x54e4, max 10000,
    /// icon 30003.
    Morph,
    /// The Suck Cannon's held count, slot 4: init 0x24ce30, update 0x24ce50, draw 0x24ced8, data 0x1413c8, max 5,
    /// icon 30015.
    SuckCannon,
    /// The nearby-bolt alert, slot 7: init 0x24b458, update 0x24ee20, draw 0x24eed8, data 0x141398, max 1, icon 30010.
    BoltAlert,
    /// Giant Clank's energy, slot 0 | 0x10 (`HudWeaponShow` in body 2): init 0x24b418, update 0x24b538, draw 0x24f248,
    /// data 0x140980, max 200.
    GiantEnergy,
    /// The boss meter, slot 6 | 0x10: init 0x24b418, update 0x24b538, the boss draw (level18 0x23cd60), data the
    /// consumer's word published under `key` ([`calls::Calls::boss_meter`]).
    Boss { key: u32 },
    /// The race's lap and place, slot 5 (levels 5 / 16): init 0x2661a0, update 0x2661e8, draw level05 0x266320, no data.
    RaceLap,
    /// The race's time (and the time trial's score), slot 7: init 0x2661a0, update 0x2661e8, draw level05 0x266710.
    RaceTime,
    /// The time trial's boost meter, slot 0 | 0x10 (levels 5 / 16): init level05 0x262ae8, update 0x262b58 (its game part:
    /// `crate::hero::hoverboard::meter_tick`), draw 0x262f50; data 0x13fbb4 (the fuel), max ticks(17)·60.
    RaceMeter,
}

/// A request: the arguments `queue_animation_update` compares (the element stands for init / update / draw / data).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    /// The slot (`slot_flags & 0xf`) and the flags (`& 0xfff0`: 0x10 persistent, 0x20 applied at once when the slot's
    /// flags also have it).
    pub slot: u8,
    pub flags: u32,
    pub icon: u16,
    pub element: Element,
    pub max: i32,
}

impl Request {
    /// The time trial's meter: `queue_animation_update(0x10, 0xffff, 0x262ae8, 0x262b58, 0x262f50, 0x13fbb4, ticks(17)·60)`
    /// with `t17` = ticks(17).
    pub fn race_meter(t17: i32) -> Request { Request { slot: 0, flags: 0x10, icon: 0xffff, element: Element::RaceMeter, max: (t17 as f32 * 60.0) as i32 } }

    /// `slot | flags` as the game passes it.
    pub fn new(slot_flags: u32, icon: u16, element: Element, max: i32) -> Request {
        Request { slot: (slot_flags & 0xf) as u8, flags: slot_flags & 0xfff0, icon, element, max }
    }

    /// The arguments the game compares (not the slot).
    fn same(&self, o: &Request) -> bool { (self.flags, self.icon, self.element, self.max) == (o.flags, o.icon, o.element, o.max) }

    /// `HudShowHealth` 0x24a498: `queue_animation_update(1, 0x7535, …, 0x1415f8, 8)`.
    pub const HEALTH: Request = Request { slot: 1, flags: 0, icon: icon::HEALTH_SLOT, element: Element::Health, max: 8 };
    /// The bolt counter's request of every caller: `queue_animation_update(2, 0x754e, …, 0x15ed98, 9999999)`.
    pub const BOLTS: Request = Request { slot: 2, flags: 0, icon: icon::BOLT_SLOT, element: Element::Bolts, max: 9_999_999 };
    /// The oxygen meter's (`0x2406b0`).
    pub const OXYGEN: Request = Request { slot: 4, flags: 0, icon: meters::ICON_TANK, element: Element::Oxygen, max: 10000 };
    /// The Morph-o-Ray's (`FUN_002d2450`).
    pub const MORPH: Request = Request { slot: 4, flags: 0, icon: meters::ICON_MORPH, element: Element::Morph, max: 10000 };
    /// The Suck Cannon's (`0x303000`).
    pub const SUCK_CANNON: Request = Request { slot: 4, flags: 0, icon: meters::ICON_TANK, element: Element::SuckCannon, max: 5 };
    /// `PromptTick` 0x278eb8's slot-12 request (the context prompt).
    pub const PROMPT: Request = Request { slot: 12, flags: 0, icon: 0, element: Element::Prompt, max: 0 };
    /// The bolt alert's (`HudBoltAlertShow` 0x227d90).
    pub const BOLT_ALERT: Request = Request { slot: 7, flags: 0, icon: meters::ICON_ALERT, element: Element::BoltAlert, max: 1 };
    /// Giant Clank's energy (`HudWeaponShow` 0x24f9c0 in body 2).
    pub const GIANT: Request = Request { slot: 0, flags: 0x10, icon: 0xffff, element: Element::GiantEnergy, max: 200 };
}

/// One HUD slot (the fields of the 0x90-byte record the ported elements use).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// +0x04 (0x10 = persistent).
    pub flags: u32,
    /// +0x00: the element's icon id.
    pub icon: u16,
    /// +0x08.
    pub max: i32,
    pub element: Element,
    /// +0x20..+0x38: the last request, +0x68: not applied yet.
    last: Request,
    pending: bool,
    /// +0x64: request handle.
    pub handle: u32,
    /// +0x48 / +0x4a: `HudFrame` offset.
    pub offset: (i32, i32),
    /// +0x6c: −6 = fully hidden (a pending element may be applied), up to 30 while shown.
    pub counter: i32,
    /// +0x70 / +0x71.
    pub slide: i32,
    pub alpha: i32,
    /// +0x74 shown value, +0x78 current value.
    pub shown: i32,
    pub value: i32,
    /// +0x7c.
    pub timer: i32,
    /// +0x58 / +0x5c: the element's size (static per slot, [`SIZES`]; the inits and draws write it).
    pub size: (i32, i32),
    /// +0x73: the counting update's step accumulator (0x24b538).
    pub acc: u8,
}

const EMPTY_REQUEST: Request = Request { slot: 0, flags: 0, icon: 0xffff, element: Element::Empty, max: 1 };

impl Default for Slot {
    /// After the level-start reset `FUN_0024af10`: the empty element applied, timer 0, +0x6c = −6.
    fn default() -> Self {
        Slot {
            flags: 0, icon: 0xffff, max: 1, element: Element::Empty, last: EMPTY_REQUEST, pending: false, handle: 0,
            offset: (0, 0), counter: -6, slide: 0, alpha: 0, shown: 0, value: 0, timer: 0, size: (0, 0), acc: 0,
        }
    }
}

/// The slots' static data (+0x58 / +0x5c size, +0x60 anchor bits: 1 top, 2 bottom, 4 left, 8 right; level01 .data
/// 0x17e0d0, the same in every overlay).
pub const SIZES: [(i32, i32); SLOTS] = [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (180, 66)];
pub const ANCHOR_BITS: [u32; SLOTS] = [5, 1, 9, 4, 8, 6, 2, 0xa, 9, 9, 9, 9, 0];

/// `FUN_0024b170`'s slide tables (24 f32 by +0x6c): 0x17e820 while the timer runs (the slide in, overshooting), 0x17e880
/// at timer 0 (the slide out).
const SLIDE_SHOWN: [u32; 24] = [
    0x3f800000, 0x3f752503, 0x3f68a3d7, 0x3f5a707e, 0x3f4a8544, 0x3f38e065, 0x3f257f73, 0x3f1059c9, 0x3ef2b9b2, 0x3ec0e51d, 0x3e8b47a6, 0x3e25c811,
    0x3d589ce5, 0xbd23c750, 0xbdbb213e, 0xbd9cb35f, 0xbc0e043a, 0x3cf1a7e3, 0xbbae31d7, 0xba1bb6aa, 0xbad38cda, 0x3abfa094, 0x3a0c825a, 0xb929de8b,
];
const SLIDE_HIDDEN: [u32; 24] = [
    0x3f800000, 0x3f75272d, 0x3f68b0f2, 0x3f5a9ccb, 0x3f4afa72, 0x3f39ece5, 0x3f27ad4b, 0x3f148c93, 0x3f00f29d, 0x3edab64a, 0x3eb49de0, 0x3e90b4c0,
    0x3e6001d6, 0x3e26b65b, 0x3ded527e, 0x3da09460, 0x3d4d2d45, 0x3cf5a31a, 0x3c8895d1, 0x3c0bb906, 0x3b820e63, 0x3ad9945b, 0x3a217b0f, 0x3950aaa8,
];

/// Banner (`ShowBanner` 0x2789e0, drawn by 0x24fb50 through 0x251b88).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Banner {
    /// 0x15f640.
    pub countdown: i32,
    /// 0x15f644, 0..0x80.
    pub alpha: i32,
    /// 0x15f648.
    pub y: i32,
    /// 0x179598.
    pub text: Vec<u8>,
}

/// Help box text colour (small font, `0x80ffa888` with the state's alpha).
const HELP_COLOUR: u32 = 0x00ff_a888;

/// Per-tick game values the HUD elements read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inputs {
    /// 0x1415f8.
    pub hp: i32,
    /// 0x15eda0 (4 at start, up to 8).
    pub max_hp: i32,
    /// 0x15ed98.
    pub bolts: i32,
    /// Held item with an ammo HUD, its ammo and max ammo; `None` when the held item has none. The game's rule
    /// (0x24f9c0): shown iff item table 0x1c4538[item] (stride 0x18) has a non-zero s16 at +0 and the player
    /// state 0x1413f4 is 0; the wrench (item 8) has 0 there, so holding it shows nothing.
    pub weapon: Option<(u16, i32, i32)>,
    /// 0x15ed88 (rc_formats::strings::lang).
    pub lang: u32,
    /// Ratchet's state 0x1413d4: mounted (0x32, `crate::hero::scripted::MOUNTED`: a turret or vehicle holds him) the
    /// health and bolt draws draw nothing (0x24e418 / 0x24ea00 test it first).
    pub hero_state: i32,
    /// The body word 0x1413f4 (`crate::hero::bodies`): Giant Clank (2) draws no health; Clank (1) shows as many orbs as
    /// 0x1415fc (`clank_max`, Clank's health when the switch took him: 0x24e418).
    pub body: u8,
    pub clank_max: i32,
    /// The tick counter 0x15f5cc (the bolt alert's spin and pulse).
    pub tick: u64,
    /// The level 0x15ed84 (the boss meter's segments, the race starts of the prompt's best-time lines).
    pub level: i32,
    /// The game mode 0x15f5c4 (5: `queue_animation_update` takes only slots 0 and 2).
    pub mode: i32,
    /// The pad's pressed mask 0x13cae4 (L3 0x200: `InLevelFrameUpdate` shows health and bolts).
    pub pressed: u32,
    /// Ratchet's position 0x13f3d0 (the race starts).
    pub hero_pos: [f32; 3],
    /// The movement group 0x1413dc (0x11: under water, the oxygen meter's caller `0x2406b0`).
    pub group: i32,
    /// The O2 mask is owned (item 0x13d4c6): no air drain and no oxygen meter.
    pub o2_mask: bool,
    /// 0x1415f0: the air left (0..10000, `crate::hero::swim::Swim::oxygen`).
    pub oxygen: i32,
    /// 0x140408: the hand item (9: the Suck Cannon's timer hold, 0x24ce50).
    pub held_item: i32,
    /// The Suck Cannon's update `0x303000` ran this tick (it queues its meter every tick); 0x1413c8 its held count;
    /// 0x13e529 the gold cannon.
    pub suck_active: bool,
    pub suck_held: i32,
    pub suck_gold: bool,
    /// The Morph-o-Ray's meter (`MorphRay::hud`: the request +0x34 is held; the target; gp−0x54e4 the value).
    pub morph_hud: bool,
    pub morph_target: i32,
    pub morph_value: i32,
    /// 0x140980: Giant Clank's energy; 0x140986: his beam lockout.
    pub energy: i32,
    pub beam_lock: i16,
    /// 0x141398: a buried bolt cache within 20 (the bolt alert's flag, `classes::buried_bolts`).
    pub bolt_alert: bool,
    /// 0x15ee58 / 0x15ee5c (Blackwater City, Kalebo III) the best race times in ticks and 0x15ee68 / 0x15ee6c the best
    /// scores: written by the races (G-LVL-007), 0 until they are ported.
    pub race_best_time: [i32; 2],
    pub race_best_score: [i32; 2],
    /// The Hoverboard's race (`crate::hero::hoverboard`): the lap 0x13fbea, the place 0x13fc10, the race ticks 0x13fbe4,
    /// the score 0x13fbf8; global flag 0 (0x13d388: Rilgar's race won once); PAL 0x15ed80.
    pub race_lap: i32,
    pub race_place: i32,
    pub race_ticks: i32,
    pub race_score: i32,
    pub race_won: bool,
    /// 0x13fc1e: the board weapons held (Kalebo III's count box in the lap bar).
    pub race_weapons: u8,
    pub pal: bool,
    /// The time trial's meter (`crate::hero::hoverboard::MeterView`; its pulse reads [`Inputs::tick`]).
    pub race_meter: crate::hero::hoverboard::MeterView,
}

impl Default for Inputs {
    /// Before the first frame reads the game state: 4 / 4, no bolts, no slot, nothing under way.
    fn default() -> Self {
        Inputs {
            hp: 4,
            max_hp: 4,
            bolts: 0,
            weapon: None,
            lang: 0,
            hero_state: 0,
            body: 0,
            clank_max: 4,
            tick: 0,
            level: 1,
            mode: 0,
            pressed: 0,
            hero_pos: [0.0; 3],
            group: 0,
            o2_mask: false,
            oxygen: 10000,
            held_item: 0,
            suck_active: false,
            suck_held: 0,
            suck_gold: false,
            morph_hud: false,
            morph_target: -1,
            morph_value: 0,
            energy: 0,
            beam_lock: 0,
            bolt_alert: false,
            race_best_time: [0; 2],
            race_best_score: [0; 2],
            race_lap: 0,
            race_weapons: 0,
            race_place: 0,
            race_ticks: 0,
            race_score: 0,
            race_won: false,
            pal: false,
            race_meter: Default::default(),
        }
    }
}

/// The HUD.
#[derive(Clone, Debug)]
pub struct HudState {
    pub assets: HudAssets,
    pub slots: [Slot; SLOTS],
    /// 0x17e958: next request handle.
    next_handle: u32,
    /// 0x17e95c: show every slot (bumps timers like flag 0x10).
    pub show_all: bool,
    /// 0x15fab0: health animation shorts {spin tick, glow, glow velocity, glow acceleration}.
    pub health_anim: [i16; 4],
    /// 0x15fab8: bolt spin tick.
    pub bolt_anim: i16,
    /// 0x15f970 / 0x15f96c: the weapon slot's request handle and item.
    weapon_handle: Option<u32>,
    /// The context prompt this tick (`PromptTick` 0x278eb8 / `NpcTalkUpdate`: an owner holds a message) and the
    /// text buffer 0x17e9b0 the draw prints ([`HudState::set_prompt`]).
    pub prompt_show: bool,
    pub prompt_text: Vec<u8>,
    /// The bolt counter kept up (`queue_animation_update(0x12, …)`: flag 0x10) while the vendor is open.
    pub bolts_pinned: bool,
    bolts_pin_handle: Option<u32>,
    pub banner: Banner,
    /// The help box as `crate::help` (the system: `Help_Update` 0x225bd0 runs in the game tick) last left it; the HUD
    /// draws it (0x2266c0) and owns only its current size 0x1798a8 / 0x1798ac ([`HudState::set_help`]).
    pub help: crate::help::HelpBox,
    inputs: Inputs,
    last: Option<(i32, i32)>,
    /// The values behind the consumers' data keys ([`Call::Data`]: the boss meters' words).
    pub words: HashMap<u32, i32>,
    /// 0x17eae0: the slot-4 tank meters' bubbles {x, y, vx, vy}.
    pub bubbles: [[f32; 4]; meters::BUBBLES],
    /// The bubbles' draws (`randf` 0x26c9c8): a newlib stream of the HUD's own [L] (the game's is the one stream
    /// `crate::rng`; the HUD runs at the engine's frame cadence, so its draws are not interleaved with the tick's).
    pub rng: crate::rng::Rng,
    /// The VSync counter 0x15f3f8 (one per HUD tick here [L]): the bubbles' alternation, the low-air blink.
    pub vsync: u32,
    /// Ratchet's class sounds the HUD played this tick (`0x236738(n, 0)`: the low-air warning 11), for the engine.
    pub sounds: Vec<i32>,
    /// `0x140984`: Giant Clank's energy request is up (`HudWeaponShow`, released by the body's leave).
    giant_up: bool,
    /// The Morph-o-Ray's request as last seen (up, target).
    morph_last: (bool, i32),
    /// Port-only: the game pixels the frame extends past the 512-wide screen on each side (16:9; 0 at 4:3, the TV's).
    /// The slots anchored left or right ([`ANCHOR_BITS`] 4 / 8) move out by it, to the frame's edges ([`Self::anchor`]).
    pub side_extra: i32,
}

impl HudState {
    pub fn new(assets: HudAssets) -> Self {
        HudState {
            assets,
            slots: [Slot::default(); SLOTS],
            next_handle: 0,
            show_all: false,
            health_anim: [0; 4],
            bolt_anim: 0,
            weapon_handle: None,
            prompt_show: false,
            prompt_text: Vec::new(),
            bolts_pinned: false,
            bolts_pin_handle: None,
            banner: Banner { y: 100, ..Default::default() },
            help: crate::help::HelpBox::default(),
            inputs: Inputs::default(),
            last: None,
            words: HashMap::new(),
            bubbles: [[0.0; 4]; meters::BUBBLES],
            rng: crate::rng::Rng::new(),
            vsync: 0,
            sounds: Vec::new(),
            giant_up: false,
            side_extra: 0,
            morph_last: (false, -1),
        }
        .with_sizes()
    }

    fn with_sizes(mut self) -> Self {
        for (s, &z) in self.slots.iter_mut().zip(SIZES.iter()) { s.size = z; }
        self
    }

    /// `queue_animation_update(slot | flags, icon, init, update, draw, data, max)` → the request handle. In game mode 5
    /// (0x15f5c4, the vendor) only slots 0 and 2 take a request (the others return 0 and keep theirs).
    pub fn queue(&mut self, slot_flags: u32, icon: u16, element: Element, max: i32) -> u32 { self.queue_req(Request::new(slot_flags, icon, element, max)) }

    /// [`HudState::queue`] with a [`Request`].
    pub fn queue_req(&mut self, req: Request) -> u32 {
        let slot = req.slot as usize & 0xf;
        if slot >= SLOTS { return 0; }
        if self.inputs.mode == 5 && slot != 0 && slot != 2 { return 0; }
        let s = &mut self.slots[slot];
        if s.last.same(&req) { return s.handle; }
        s.last = req;
        s.handle = self.next_handle;
        self.next_handle += 1;
        s.pending = true;
        s.timer = 0;
        s.slide = 0;
        s.alpha = 0;
        if req.flags & s.flags & 0x20 != 0 { self.apply(slot); }
        self.slots[slot].handle
    }

    /// The handle of the slot whose last request is `r` (the handle the game's caller of that request holds).
    fn handle_of(&self, r: &Request) -> Option<u32> {
        let s = self.slots.get(r.slot as usize)?;
        (s.last.same(r) && s.last.element != Element::Empty).then_some(s.handle)
    }

    /// `fun_001ff480(handle)` 0x24afa0: the slot holding `handle` gets the empty request (icon 0xffff, max 0), its
    /// visible timer +0x7c = 0 and its counter +0x6c = −6, so the empty element replaces it at once (docs/plan/menus.md
    /// "Close"). Waiting for the element to hide instead never ends under flag 0x10, which keeps the timer up (the
    /// boss meters). True when a slot held it.
    pub fn release(&mut self, handle: u32) -> bool {
        let Some(i) = self.slots.iter().position(|s| s.handle == handle && s.last.element != Element::Empty) else { return false };
        self.queue(i as u32, 0xffff, Element::Empty, 0);
        self.slots[i].counter = -6;
        self.apply(i);
        true
    }

    /// `PromptRelease` 0x279070's slot part: the prompt's slot gets the empty request at once ([`HudState::release`]),
    /// whether or not the HUD ticks this frame (the take-off hides it, the planet page freezes it). True when it was up.
    pub fn release_prompt(&mut self) -> bool {
        self.prompt_show = false;
        self.handle_of(&Request::PROMPT).is_some_and(|h| self.release(h))
    }

    /// This frame's draws without a tick (on a copy: the draws advance the banner and help box).
    pub fn draws_now(&self) -> Vec<Draw> {
        let mut c = self.clone();
        let mut out = Vec::new();
        c.draw(&mut out);
        out
    }

    /// The calls the game made since the last HUD tick ([`calls`]), in order.
    pub fn apply_calls(&mut self, calls: &[Call]) {
        for c in calls {
            match *c {
                Call::Queue(r) => {
                    self.queue_req(r);
                }
                Call::KeepUp(r, n) => {
                    if let Some(h) = self.handle_of(&r) { self.keep_up(h, n); }
                }
                Call::SetFlags(r, f) => {
                    if let Some(h) = self.handle_of(&r) { self.set_flags(h, f); }
                }
                Call::Release(r) => {
                    if let Some(h) = self.handle_of(&r) { self.release(h); }
                }
                Call::ShowHealthBolts(n) => self.show_health_bolts(n),
                Call::Data(k, v) => {
                    self.words.insert(k, v);
                }
            }
        }
    }

    /// `HudShowHealth()` + `FUN_0024b4b0(h, n)`, the bolt counter's request + `FUN_0024b4b0(h, n)` (the keep-up does
    /// nothing while the request is still pending: the element's own init timer then shows it).
    pub fn show_health_bolts(&mut self, n: i32) {
        let h = self.queue_req(Request::HEALTH);
        self.keep_up(h, n);
        let h = self.queue_req(Request::BOLTS);
        self.keep_up(h, n);
    }

    /// `FUN_0024b090(handle, flags)`: replace the flags of the slot holding `handle`.
    pub fn set_flags(&mut self, handle: u32, flags: u32) {
        if let Some(s) = self.slots.iter_mut().find(|s| s.handle == handle) {
            s.last.flags = flags;
            if !s.pending { s.flags = flags; }
        }
    }

    /// `apply_pending_animation` + the element's init.
    fn apply(&mut self, slot: usize) {
        let s = &mut self.slots[slot];
        let r = s.last;
        s.icon = r.icon;
        s.flags = r.flags;
        s.element = r.element;
        s.max = r.max;
        s.pending = false;
        match r.element {
            Element::Empty => {}
            Element::Health => {
                // 0x24e1b8.
                s.timer = scale_ticks(180) + 30;
                s.offset = (32, 0);
                self.init_value(slot);
                self.health_anim = [0, 0, 0, 1];
            }
            Element::Bolts => {
                // 0x24e8d0 → 0x24f368.
                s.timer = scale_ticks(120) + 30;
                self.init_value(slot);
                self.bolt_anim = 0;
            }
            Element::Weapon { .. } => {
                s.timer = scale_ticks(120) + 30;
                self.init_value(slot);
            }
            Element::Prompt => {
                // 0x24c828: offset 0, timer ScaleTicks(10) + 30, size 32×32.
                s.offset = (0, 0);
                s.timer = scale_ticks(10) + 30;
                s.size = (0x20, 0x20);
                self.init_value(slot);
            }
            Element::Oxygen | Element::Morph => {
                // 0x24b900 / 0x24c1a8: 0x24b418, then the bubbles.
                self.init_generic(slot);
                self.bubbles_init();
            }
            // 0x24ce30 (= 0x24b418), 0x24b418.
            Element::SuckCannon | Element::GiantEnergy | Element::Boss { .. } => self.init_generic(slot),
            Element::BoltAlert => self.init_alert(slot),
            Element::RaceMeter => {
                // 0x262ae8: offset 0, timer ticks(180) + 30, the value (0x24b318); the last combos: the game side's.
                s.offset = (0, 0);
                s.timer = scale_ticks(0xb4) + 0x1e;
                self.init_value(slot);
            }
            Element::RaceLap | Element::RaceTime => {
                // 0x2661a0: timer ticks(180) + 30, size 0x80 × 0x80, offset 0.
                s.timer = scale_ticks(0xb4) + 0x1e;
                s.size = (0x80, 0x80);
                s.offset = (0, 0);
            }
        }
    }

    /// `FUN_0024b418`: offset 0, timer `ScaleTicks(180) + 30`, the value (0x24b318).
    fn init_generic(&mut self, slot: usize) {
        let s = &mut self.slots[slot];
        s.offset = (0, 0);
        s.timer = scale_ticks(180) + 30;
        self.init_value(slot);
    }

    /// `FUN_0024b4b0(handle, n)`: the slot holding `handle`, when its request is applied, keeps its timer at ≥ n;
    /// false when there is no such slot or it is still pending.
    pub fn keep_up(&mut self, handle: u32, n: i32) -> bool {
        // (The port's handles start at 0 like the empty slots' default: only a slot holding an element matches.)
        let Some(s) = self.slots.iter_mut().find(|s| s.handle == handle && !s.pending && s.element != Element::Empty) else { return false };
        s.timer = s.timer.max(n);
        true
    }

    /// `FUN_0024fb00` → `FUN_0024af10` (`OpenVendorMenu`, the page menus): every slot emptied at once (no ramp), the
    /// requests the elements keep re-made by their owners.
    pub fn reset_slots(&mut self) {
        // (+0x58 / +0x5c are not reset: the static sizes stay as the inits left them.)
        let sizes = self.slots.map(|s| s.size);
        self.slots = [Slot::default(); SLOTS];
        for (s, z) in self.slots.iter_mut().zip(sizes) { s.size = z; }
        self.giant_up = false;
        self.weapon_handle = None;
        self.bolts_pin_handle = None;
        self.prompt_show = false;
    }

    /// The context prompt of this tick and its text (the engine: `moby_update::interact`).
    pub fn set_prompt(&mut self, show: bool, text: &[u8]) {
        self.prompt_show = show;
        if self.prompt_text != text { self.prompt_text = text.to_vec(); }
    }

    /// The element's data word (`*data`).
    fn data(&self, e: Element) -> Option<i32> {
        let i = &self.inputs;
        Some(match e {
            Element::Empty | Element::Prompt | Element::RaceLap | Element::RaceTime => return None,
            Element::Health => i.hp,
            Element::Bolts => i.bolts,
            Element::Weapon { .. } => i.weapon.map_or(0, |w| w.1),
            Element::Oxygen => i.oxygen,
            Element::Morph => i.morph_value,
            Element::SuckCannon => i.suck_held,
            Element::BoltAlert => i.bolt_alert as i32,
            Element::GiantEnergy => i.energy,
            Element::RaceMeter => i.race_meter.fuel,
            Element::Boss { key } => self.words.get(&key).copied().unwrap_or(0),
        })
    }

    /// `FUN_0024b318`: value = min(*data, max), shown = value (no data → 99999); then the size for the value's digits
    /// (n = the digits of max − 1): a slot anchored left or right only grows `12·(n + 1)` taller and at least 14 wide,
    /// any other `14·(n + 1)` wider and at least 12 tall (the ported draws set their size before using it).
    fn init_value(&mut self, slot: usize) {
        let v = self.data(self.slots[slot].element);
        let s = &mut self.slots[slot];
        s.value = match v {
            Some(v) => v.min(s.max),
            None => 99999,
        };
        s.shown = s.value;
        let mut n = 0;
        let mut m = s.max;
        while m > 9 {
            n += 1;
            m /= 10;
        }
        let bits = ANCHOR_BITS[slot];
        if bits & 3 == 0 && bits & 0xc != 0 {
            s.size.1 += (n + 1) * 0xc;
            if s.size.0 < 0xe { s.size.0 = 0xe; }
        } else {
            if s.size.1 < 0xc { s.size.1 = 0xc; }
            s.size.0 += (n + 1) * 0xe;
        }
    }

    /// `FUN_0024b108`: the slot's place from its anchor: centred vertically unless anchored top or bottom (`y −= h/2`),
    /// `x −= w` anchored right, `x −= w/2` centred, unchanged anchored left.
    fn align(&self, i: usize) -> (i32, i32) {
        let (mut x, mut y) = self.anchor(i);
        let (w, h) = self.slots[i].size;
        let bits = ANCHOR_BITS[i];
        if bits & 1 == 0 && bits & 2 == 0 { y -= h >> 1; }
        if bits & 4 == 0 { x -= if bits & 8 == 0 { w >> 1 } else { w }; }
        (x, y)
    }

    /// The slot's static anchor ([`ANCHORS`]), moved out by [`Self::side_extra`] when anchored left (4) or right (8).
    fn anchor(&self, i: usize) -> (i32, i32) {
        let (x, y) = ANCHORS[i];
        let bits = ANCHOR_BITS[i];
        let dx = if bits & 4 != 0 { -self.side_extra } else if bits & 8 != 0 { self.side_extra } else { 0 };
        (x + dx, y)
    }

    /// `FUN_0024b108` then `FUN_0024b170(slot, &x, &y, +0x6c, 0)`: the slide off its edge, `f = table[clamp(+0x6c, 0,
    /// 23)]` (0x17e820 while the timer runs, 0x17e880 at 0): top `y −= ⌊f·(h + 52) + ½⌋`, bottom `y +=`, else left
    /// `x −= ⌊f·(w + 20) + ½⌋`, right `x +=` (truncated toward 0).
    fn place(&self, i: usize) -> (i32, i32) {
        let (mut x, mut y) = self.align(i);
        let s = &self.slots[i];
        let k = s.counter.clamp(0, 0x17) as usize;
        let f = f32::from_bits(if s.timer != 0 { SLIDE_SHOWN[k] } else { SLIDE_HIDDEN[k] });
        let (w, h) = s.size;
        let bits = ANCHOR_BITS[i];
        let v = |n: i32| (f * (n as f32 + if bits & 3 != 0 { 52.0 } else { 20.0 }) + 0.5) as i32;
        if bits & 1 != 0 {
            y -= v(h);
        } else if bits & 2 != 0 {
            y += v(h);
        } else if bits & 4 != 0 {
            x -= v(w);
        } else if bits & 8 != 0 {
            x += v(w);
        }
        (x, y)
    }

    /// The help box of the game tick (`crate::help::Help::bx`), before this tick's draw.
    pub fn set_help(&mut self, b: &crate::help::HelpBox) { self.help.copy_logic_from(b); }

    /// `ShowBanner(msg, ticks)` with the text already formatted; `ticks` defaults to `ScaleTicks(180)`, as does −1 (the
    /// game's `ShowBanner` 0x2789e0 turns −1 into `ticks(0xb4)`: kept as −1 the countdown would never reach 0).
    pub fn show_banner(&mut self, text: &[u8], ticks: Option<i32>) {
        self.banner.text = text.to_vec();
        self.banner.countdown = match ticks {
            None | Some(-1) => scale_ticks(180),
            Some(t) => t,
        };
    }

    /// `ShowBanner(id, ticks)` 0x2789e0: the level message `id` (`msg_string`) for `ticks` (the gold bolt's "Gold Bolt
    /// Acquired", `ShowPlanetBanner`'s planet messages).
    pub fn show_banner_msg(&mut self, id: i32, ticks: i32) {
        let t = strings::lookup(&self.assets.messages, id).to_vec();
        self.show_banner(&t, Some(ticks));
    }

    /// `ShowBannerf(id, n, −1)` 0x278a50: the level message `id` with its `%d` replaced by `n` (`sprintf` into
    /// 0x179598), 180 ticks (the ammo pickups' "+n" banner).
    pub fn show_bannerf(&mut self, id: i32, n: i32) {
        let t = strings::lookup(&self.assets.messages, id).to_vec();
        let text = match t.windows(2).position(|w| w == b"%d") {
            Some(i) => [&t[..i], n.to_string().as_bytes(), &t[i + 2..]].concat(),
            None => t,
        };
        self.show_banner(&text, None);
    }

    /// One game tick: the callers that arm elements, `Help_Update`, the slot loop; then the frame's draws.
    pub fn tick(&mut self, inputs: Inputs) -> Vec<Draw> {
        self.inputs = inputs;
        self.sounds.clear();
        self.vsync = self.vsync.wrapping_add(1);
        self.frame_calls();
        // PromptTick 0x278eb8's slot part (and NpcTalkUpdate's): slot 12 requested, or kept up for 10 ticks.
        if self.prompt_show {
            let h = self.queue_req(Request::PROMPT);
            self.keep_up(h, scale_ticks(10));
        }
        // The vendor's bolt counter (OpenVendorMenu: slot 2 | 0x10; VendorExit: flags 0).
        match (self.bolts_pinned, self.bolts_pin_handle) {
            (true, None) => self.bolts_pin_handle = Some(self.queue(0x12, icon::BOLT_SLOT, Element::Bolts, 9_999_999)),
            (false, Some(h)) => {
                self.set_flags(h, 0);
                self.bolts_pin_handle = None;
            }
            _ => {}
        }
        self.update_slots();
        let mut out = Vec::new();
        self.draw(&mut out);
        out
    }

    /// The callers the port sees through the game state instead of a call (their code is the hero's, which reaches no
    /// call channel), each at the call's condition:
    /// * `HeroTakeDamage` 0x226fa8 → `HudShowHealth`: the health went down (a gain re-arms nothing: the element, once in
    ///   slot 1, shows the change itself, 0x24e238). The bolts' callers (`CollectBolt` 0x2bc4f0, the vendor, …): the
    ///   count changed.
    /// * `InLevelFrameUpdate` 0x2aba68: L3 pressed (0x13cae4 & 0x200) outside states 0x1d / 0x32 → health and bolts kept
    ///   up `ScaleTicks(180)`.
    /// * `0x2406b0` (the swim checks): in the under-water group 0x11 without the O2 mask, the oxygen meter every tick.
    /// * `0x303000` (the Suck Cannon's update): its meter every tick it runs.
    /// * `FUN_002d2450` (the Morph-o-Ray): a new target queues its meter; the request released (`fun_001ff480(+0x34)`)
    ///   when it stops firing.
    /// * The body's leave `0x231348`: Giant Clank's energy request `FUN_0024b090(0x140984, 0)`.
    fn frame_calls(&mut self) {
        let i = self.inputs;
        if let Some((hp, bolts)) = self.last {
            if i.hp < hp { self.queue_req(Request::HEALTH); }
            if i.bolts != bolts { self.queue_req(Request::BOLTS); }
        }
        self.last = Some((i.hp, i.bolts));
        if i.pressed & crate::pad::button::L3 != 0 && i.hero_state != 0x1d && i.hero_state != crate::hero::scripted::MOUNTED { self.show_health_bolts(scale_ticks(180)); }
        if i.group == 0x11 && !i.o2_mask { self.queue_req(Request::OXYGEN); }
        if i.suck_active { self.queue_req(Request::SUCK_CANNON); }
        let (was, target) = self.morph_last;
        if i.morph_hud && (!was || (i.morph_target != -1 && i.morph_target != target)) {
            self.queue_req(Request::MORPH);
        } else if !i.morph_hud && was {
            if let Some(h) = self.handle_of(&Request::MORPH) { self.release(h); }
        }
        self.morph_last = (i.morph_hud, if i.morph_target != -1 { i.morph_target } else { target });
        if i.body != 2 && self.giant_up {
            if let Some(h) = self.handle_of(&Request::GIANT) { self.set_flags(h, 0); }
            self.giant_up = false;
        }
    }

    /// 0x24f880 (with 0x24f9c0 first).
    fn update_slots(&mut self) {
        self.update_weapon_request();
        for i in 0..SLOTS {
            let s = &mut self.slots[i];
            if s.flags & 0x10 != 0 || self.show_all { s.timer = s.timer.max(scale_ticks(10)); }
            let shown = s.timer >= 1 && {
                s.timer -= 1;
                s.timer >= 1
            };
            if shown {
                if s.counter < 30 { s.counter += 1; }
            } else if s.counter > -6 {
                s.counter -= 1;
            }
            if s.pending && s.counter == -6 { self.apply(i); }
            match self.slots[i].element {
                Element::Empty => {}
                Element::Health => self.update_health(i),
                Element::Bolts => self.update_bolts(i),
                Element::Weapon { .. } => self.update_weapon(i),
                // 0x24c878 = 0x24b538 (no data: the ramp only).
                Element::Prompt | Element::GiantEnergy | Element::Boss { .. } => self.update_counting(i),
                Element::Oxygen => {
                    // 0x24baf0.
                    self.update_counting(i);
                    self.bubbles_step();
                }
                Element::Morph => {
                    // 0x24c398.
                    self.update_immediate(i);
                    self.bubbles_step();
                }
                Element::SuckCannon => self.update_suck(i),
                Element::BoltAlert => self.update_alert(i),
                Element::RaceMeter => {
                    // 0x262b58's slot part: the shown value approaches the fuel by 6 (`Approach`, truncated).
                    let fuel = self.data(Element::RaceMeter).unwrap_or(0) as f32;
                    let s = &mut self.slots[i];
                    // 0x262f50 writes the size before placing: 256 × 64.
                    s.size = (0x100, 0x40);
                    let mut v = s.shown as f32;
                    let d = fuel - v;
                    if d.abs() <= 6.0 { v = fuel; } else if 0.0 < d { v += 6.0; } else { v -= 6.0; }
                    s.shown = v as i32;
                }
                Element::RaceLap | Element::RaceTime => {
                    // 0x2661e8: on the board (group 0x16) timer ticks(30), +0x6c = 5; else +0x6c = −6.
                    let on = self.inputs.group == 0x16;
                    let s = &mut self.slots[i];
                    if on {
                        s.timer = scale_ticks(0x1e);
                        s.counter = 5;
                    } else {
                        s.counter = -6;
                    }
                }
            }
        }
    }

    /// 0x24f9c0: keep the weapon element requested while the held item has an ammo HUD (on foot, body 0), release it
    /// otherwise; as Giant Clank (body 2) slot 0 shows his energy instead (`0x140984`).
    fn update_weapon_request(&mut self) {
        let weapon = if self.inputs.body != 0 { None } else { self.inputs.weapon };
        match weapon {
            Some((item, _, max)) => {
                self.weapon_handle = Some(self.queue(0x10, icon::ITEM_BASE.wrapping_add(item), Element::Weapon { item }, max));
            }
            None => {
                if let Some(h) = self.weapon_handle.take() { self.set_flags(h, 0); }
                if self.inputs.body == 2 {
                    self.queue_req(Request::GIANT);
                    self.giant_up = true;
                }
            }
        }
    }

    /// `FUN_0024b538`, the counting update: value = min(max(*data, 0), max); while the shown value differs, the timer is
    /// `ScaleTicks(180)` and, once the slot is in (+0x6c > 23), the shown value counts toward it: a step of
    /// `max(⌊5·√(d/25)⌋, ⌊2d/10⌋)` clamped to 1..121 is added to the accumulator +0x73 (a byte) and once that passes 2,
    /// the shown value moves by `acc / 2` and the accumulator keeps the remainder. Then the generic ramp.
    fn update_counting(&mut self, i: usize) {
        let v = self.data(self.slots[i].element);
        let s = &mut self.slots[i];
        if let Some(v) = v { s.value = v.max(0).min(s.max); }
        if s.shown != s.value {
            s.timer = scale_ticks(180);
            if s.counter > 0x17 {
                let d = (s.shown - s.value).abs();
                if d != 0 {
                    let mut n = ((d as f32 / 25.0).sqrt() * 5.0) as i32;
                    let q = d * scale_ticks(2) / scale_ticks(10);
                    if n < q { n = q; }
                    n = n.clamp(1, 0x79);
                    let sum = s.acc as i32 + n;
                    s.acc = sum as u8;
                    if scale_ticks(2) < (sum & 0xff) {
                        let k = s.acc as i32 / scale_ticks(2);
                        s.shown += if s.value < s.shown { -k } else { k };
                        s.acc = s.acc.wrapping_sub((k * scale_ticks(2)) as u8);
                    }
                }
            }
        }
        Self::ramp_generic(s);
    }

    /// `FUN_0024b7c0`: as [`HudState::update_counting`] but the shown value jumps to the value once the slot is in.
    fn update_immediate(&mut self, i: usize) {
        let v = self.data(self.slots[i].element);
        let s = &mut self.slots[i];
        if let Some(v) = v { s.value = v.max(0).min(s.max); }
        if s.shown != s.value {
            s.timer = scale_ticks(180);
            if s.counter > 0x17 { s.shown = s.value; }
        }
        Self::ramp_generic(s);
    }

    /// The ramp of 0x24b538 / 0x24b7c0: below `ScaleTicks(5)` of timer, +0x6c = 1 while alpha or slide is up and alpha
    /// then slide step down (+0x6c is left to the loop once both are 0); otherwise slide then alpha step up to
    /// `ScaleTicks(8)`.
    fn ramp_generic(s: &mut Slot) {
        if s.timer < scale_ticks(5) {
            if s.alpha != 0 || s.slide != 0 { s.counter = 1; }
            if s.alpha != 0 {
                s.alpha -= 1;
            } else if s.slide != 0 {
                s.slide -= 1;
            }
        } else if s.slide < scale_ticks(8) {
            s.slide += 1;
        } else if s.alpha < scale_ticks(8) {
            s.alpha += 1;
        }
    }

    fn ramp(s: &mut Slot, slide_steps: i32, alpha_steps: i32, down: bool) {
        if down {
            s.counter = 1;
            if s.alpha != 0 {
                s.alpha -= 1;
            } else if s.slide != 0 {
                s.slide -= 1;
            } else {
                s.counter = -6;
            }
        } else if s.slide < slide_steps {
            s.slide += 1;
        } else if s.alpha < alpha_steps {
            s.alpha += 1;
        }
    }

    /// 0x24e238.
    fn update_health(&mut self, i: usize) {
        let hp = self.inputs.hp.max(0);
        let p = &mut self.health_anim;
        let acc = p[2].wrapping_add(p[3]);
        p[1] = p[1].wrapping_add(p[2]);
        p[2] = acc;
        p[0] = (p[0] + 1) % 60;
        p[2] = acc.clamp(-60, 60);
        if p[1] >= 0x800 && p[2] > 0 { p[3] = -1; }
        if p[1] >= 0x1000 { p[1] = 0xfff; }
        if p[1] < 0x7ff && p[2] < 0 { p[3] = 1; }
        if p[1] < 0 { p[1] = 0; }
        let s = &mut self.slots[i];
        s.value = hp.min(s.max);
        if s.shown != s.value || s.shown == 1 {
            s.timer = scale_ticks(120);
            s.shown = s.value;
        }
        let down = s.timer < scale_ticks(5);
        Self::ramp(s, HEALTH_STEPS, HEALTH_STEPS, down);
    }

    /// 0x24e908.
    fn update_bolts(&mut self, i: usize) {
        self.bolt_anim = (self.bolt_anim + 1) % 60;
        let bolts = self.inputs.bolts;
        let s = &mut self.slots[i];
        if s.shown != bolts {
            s.shown = bolts;
            s.timer = scale_ticks(90);
        }
        let down = s.timer < scale_ticks(5);
        Self::ramp(s, BOLT_SLIDE_STEPS, BOLT_ALPHA_STEPS, down);
    }

    /// 0x2519c0.
    fn update_weapon(&mut self, i: usize) {
        let ammo = self.data(self.slots[i].element).unwrap_or(0);
        let s = &mut self.slots[i];
        s.shown = if s.max < ammo { s.max } else { ammo.max(0) };
        let down = s.timer < scale_ticks(5);
        if !down { s.timer = scale_ticks(5); }
        Self::ramp(s, WEAPON_STEPS, WEAPON_STEPS, down);
    }

    fn help_text(&self) -> &[u8] { self.help.index.and_then(|i| self.assets.messages.get(i)).map_or(&[][..], |m| &m.text) }

    /// `HudDraw` 0x24fb50 (slots, banner) then the help box 0x2266c0.
    fn draw(&mut self, out: &mut Vec<Draw>) {
        for i in 0..SLOTS {
            match self.slots[i].element {
                Element::Empty => {}
                Element::Health => self.draw_health(i, out),
                Element::Bolts => self.draw_bolts(i, out),
                Element::Weapon { .. } => self.draw_weapon(i, out),
                Element::Prompt => self.draw_prompt(i, out),
                Element::Oxygen => self.draw_tank(i, true, out),
                Element::Morph => self.draw_tank(i, false, out),
                Element::SuckCannon => self.draw_suck(i, out),
                Element::BoltAlert => self.draw_alert(i, out),
                Element::GiantEnergy => self.draw_giant(i, out),
                Element::Boss { .. } => self.draw_boss(i, out),
                Element::RaceLap => self.draw_race_lap(i, out),
                Element::RaceTime => self.draw_race_time(i, out),
                Element::RaceMeter => self.draw_race_meter(i, out),
            }
        }
        self.draw_banner(out);
        self.draw_help(out);
    }

    fn fractions(s: &Slot, slide_steps: i32, alpha_steps: i32) -> (f32, f32) {
        ((s.slide as f32 / slide_steps as f32).clamp(0.0, 1.0), (s.alpha as f32 / alpha_steps as f32).clamp(0.0, 1.0))
    }

    /// `HudFrame` 0x24fd28: slot offset, then flags 1 centre, 2 half size, 4 double size.
    #[allow(clippy::too_many_arguments)]
    fn hud_frame(&self, slot: usize, frame: usize, x: i32, y: i32, flags: u32, alpha: i32, out: &mut Vec<Draw>) {
        let (ox, oy) = self.slots[slot].offset;
        let (mut x, mut y) = (x + ox, y + oy);
        let (w0, h0) = self.assets.size(frame);
        let (hw, hh) = (w0 >> 1, h0 >> 1);
        let (mut w, mut h) = (w0, h0);
        if flags & 1 != 0 {
            x -= hw;
            y -= hh;
        }
        if flags & 2 != 0 {
            x += w0 >> 2;
            y += h0 >> 2;
            w = hw;
            h = hh;
        }
        if flags & 4 != 0 {
            x -= hw;
            y -= hh;
            w <<= 1;
            h <<= 1;
        }
        out.push(Draw::Sprite { frame, x, y, w, h, alpha, rot: Rot::None });
    }

    fn sprite(frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32, rot: Rot) -> Draw { Draw::Sprite { frame, x, y, w, h, alpha, rot } }

    /// `font_print_right` (regular font): `x − width`.
    fn text_right(&self, x: i32, y: i32, rgba: u32, text: &[u8], out: &mut Vec<Draw>) {
        let w = self.assets.text_width(Font::Regular, text);
        out.push(Draw::Text { font: Font::Regular, x: x - w, y, rgba, text: text.to_vec() });
    }

    /// 0x24e418 (nothing as Giant Clank, body 2, or while Ratchet is mounted, state 0x32; as Clank, body 1, the orbs of
    /// Clank's 0x1415fc).
    fn draw_health(&self, i: usize, out: &mut Vec<Draw>) {
        let s = &self.slots[i];
        if self.inputs.body == 2 || self.inputs.hero_state == crate::hero::scripted::MOUNTED || s.slide == 0 { return; }
        let x = self.anchor(i).0;
        let y = HEALTH_Y + ELEMENT_Y;
        let (sf, f) = Self::fractions(s, HEALTH_STEPS, HEALTH_STEPS);
        let a_orb = (f * 128.0) as i32;
        let bar_a = (HEALTH_BAR_ALPHA as f32 * sf) as i32;
        let p = &self.health_anim;
        let mut fr = (p[0] as i32) >> 1;
        if s.value == 0 { fr = 30; }
        let max_hp = if self.inputs.body == 1 { self.inputs.clank_max } else { self.inputs.max_hp };
        let (orbs, hw, hw2): (&[(i32, i32)], i32, i32) = match max_hp {
            8 => (&ORBS_8, (sf * 48.0) as i32, (sf * 48.0) as i32),
            5 => (&ORBS_5, (sf * 32.0) as i32, (sf * 16.0) as i32),
            _ => (&ORBS_4, (sf * 48.0) as i32, 0),
        };
        let cap = self.assets.icon_frame(icon::BAR, 1);
        let mid = self.assets.icon_frame(icon::BAR, 0);
        let mut yb = y + BAR_DY;
        for (k, half) in [hw, hw2].into_iter().enumerate() {
            if k == 1 {
                if half == 0 { break; }
                yb += BAR_ROW_DY;
            }
            out.push(Self::sprite(cap, x + half, yb, BAR_TILE, BAR_TILE, bar_a, Rot::R180));
            out.push(Self::sprite(mid, x - half, yb, half << 1, BAR_TILE, bar_a, Rot::None));
            out.push(Self::sprite(cap, x - half - BAR_TILE, yb, BAR_TILE, BAR_TILE, bar_a, Rot::None));
        }
        let glow_a = ((p[1] >> 4) as f32 * f) as i32;
        for (n, &(ox, oy)) in orbs.iter().enumerate().take(max_hp.max(0) as usize) {
            self.hud_frame(i, self.assets.icon_frame(icon::ORB, fr), x + ox, y + oy, 1, a_orb, out);
            if fr != 30 {
                self.hud_frame(i, self.assets.icon_frame(icon::ORB, 30), x + ox, y + oy, 1, a_orb, out);
                out.push(Self::sprite(self.assets.icon_frame(icon::ORB, 31), x + ox + 14, y + oy - 17, 34, 34, glow_a, Rot::None));
                fr = if s.value == n as i32 + 1 { 30 } else { (fr + 6) % 30 };
            }
        }
    }

    /// 0x24ea00 (nothing while Ratchet is mounted, state 0x32).
    fn draw_bolts(&self, i: usize, out: &mut Vec<Draw>) {
        let s = &self.slots[i];
        if self.inputs.hero_state == crate::hero::scripted::MOUNTED || s.slide == 0 { return; }
        let y = ELEMENT_Y;
        let x = self.anchor(i).0;
        let (sf, f) = Self::fractions(s, BOLT_SLIDE_STEPS, BOLT_ALPHA_STEPS);
        let bar_a = ((sf * 128.0) as i32 as f32 * 0.7) as i32;
        let bolts = self.inputs.bolts;
        let mut digits = 1;
        let mut v = bolts;
        while v > 9 {
            digits += 1;
            v /= 10;
        }
        let w = ((DIGIT_W * digits) as f32 * sf) as i32;
        let left = x - 0x1c - w;
        let (cap, mid) = (self.assets.icon_frame(icon::BAR, 1), self.assets.icon_frame(icon::BAR, 0));
        out.push(Self::sprite(cap, x - 12, y, 32, 32, bar_a, Rot::R180));
        out.push(Self::sprite(mid, left, y, w + 16, 32, bar_a, Rot::None));
        out.push(Self::sprite(cap, left - 32, y, 32, 32, bar_a, Rot::None));
        self.hud_frame(i, self.assets.icon_frame(icon::BOLT, (self.bolt_anim as i32) >> 1), x - 32, y, 0, 0x80, out);
        let colour = tween_color(f, TEXT_FROM, TEXT_TO);
        let shadow = tween_color(f, 0, 0x8000_0000);
        let number = bolts.to_string().into_bytes();
        let (tx, ty) = (x + BOLT_TEXT_D.0, y + BOLT_TEXT_D.1);
        self.text_right(tx + 1, ty + 1, shadow, &number, out);
        self.text_right(tx, ty, colour, &number, out);
        let german_italian = self.inputs.lang == strings::lang::GERMAN || self.inputs.lang == strings::lang::ITALIAN;
        let (sep, (dx, dy)): (&[u8], _) = if german_italian { (b".", SEP_DOT_D) } else { (b"'", SEP_APOSTROPHE_D) };
        let mut k = 3;
        while k < digits {
            self.text_right(x + dx - DIGIT_W * k + 2, y + dy + 2, shadow, sep, out);
            self.text_right(x + dx - DIGIT_W * k, y + dy, colour, sep, out);
            k += 3;
        }
    }

    /// 0x24c898 (Lombyte `HudRaceTimerDraw`): the prompt text 0x17e9b0 in a bar frame, centred on x 256, at
    /// y = 0x15f770 (32) + 18 (NTSC; 10 PAL). Alpha `trunc(128·slide/8)`, text colour `A << 24 | 0x40f040`
    /// (green), regular font. A byte 0x01 splits it into two lines (frame 54 tall, second line 19 lower). Then the
    /// best-time lines ([`HudState::draw_best_times`]).
    fn draw_prompt(&mut self, i: usize, out: &mut Vec<Draw>) {
        self.draw_prompt_box(i, out);
        self.draw_best_times(i, out);
    }

    /// The best-time lines of 0x24c898: on level 5 (Blackwater City) or 16 (Kalebo III) with Ratchet within 5 of the race
    /// start ((294.5, 225.4, 100) / (115.7, 287, 119.5)), the banner moves to y + 100 (0x15f648) and, with a best time
    /// (0x15ee58 + 4·(level 16)), `DrawBanner(256, y + 40, colour, "%s: %d:%02d:%02d")` of message 0x531b and the time
    /// (3600 ticks a minute, 60 a second, hundredths), then with a best score (0x15ee68 + …) `"%s: %d"` of 0x531c at
    /// y + 72, both in the prompt's colour.
    fn draw_best_times(&mut self, i: usize, out: &mut Vec<Draw>) {
        let level = self.inputs.level;
        if level != 5 && level != 0x10 { return; }
        let start: [f32; 3] = if level == 5 { [f32::from_bits(0x4393_4000), f32::from_bits(0x4361_6666), 100.0] } else { [f32::from_bits(0x42e7_6666), f32::from_bits(0x438f_8000), f32::from_bits(0x42ef_0000)] };
        let p = self.inputs.hero_pos;
        let d = ((p[0] - start[0]).powi(2) + (p[1] - start[1]).powi(2) + (p[2] - start[2]).powi(2)).sqrt();
        if d >= 5.0 { return; }
        let y = 32 + ELEMENT_Y;
        self.banner.y = y + 100;
        let k = (level == 0x10) as usize;
        let (time, score) = (self.inputs.race_best_time[k], self.inputs.race_best_score[k]);
        if time == 0 { return; }
        let f = (self.slots[i].slide as f32 / scale_ticks(8) as f32).clamp(0.0, 1.0);
        let colour = ((f * 128.0) as i32 as u32) << 24 | 0x0040_f040;
        let minute = 0xe10;
        let second = minute / 0x3c;
        let rest = time % minute;
        let secs = rest / second;
        let label = |id: i32| strings::lookup(&self.assets.messages, id).to_vec();
        let mut t = label(0x531b);
        t.extend_from_slice(format!(": {}:{:02}:{:02}", time / minute, secs, ((rest - secs * second) * 100) / second).as_bytes());
        self.draw_banner_text(0x100, y + 0x28, colour, t, out);
        if score != 0 {
            let mut t = label(0x531c);
            t.extend_from_slice(format!(": {score}").as_bytes());
            self.draw_banner_text(0x100, y + 0x48, colour, t, out);
        }
    }

    fn draw_prompt_box(&self, i: usize, out: &mut Vec<Draw>) {
        let s = &self.slots[i];
        let y = 32 + ELEMENT_Y;
        let f = (s.slide as f32 / scale_ticks(8) as f32).clamp(0.0, 1.0);
        let a = (f * 128.0) as i32;
        let colour = (a as u32) << 24 | 0x0040_f040;
        let text = &self.prompt_text;
        let (mid, cap) = (self.assets.icon_frame(icon::BAR, 0), self.assets.icon_frame(icon::BAR, 1));
        let width = |t: &[u8]| self.assets.text_width(Font::Regular, t);
        let centred = |t: &[u8], y: i32, out: &mut Vec<Draw>| out.push(Draw::Text { font: Font::Regular, x: 0x100 - (width(t) >> 1), y, rgba: colour, text: t.to_vec() });
        // The scan stops at the first byte ≤ 1 from index 1 on.
        let mut k = 0;
        if text.first().is_some_and(|&c| c > 1) {
            k = 1;
            while k < 0x80 && text.get(k).is_some_and(|&c| c > 1) { k += 1; }
        }
        if text.get(k) == Some(&1) {
            let (l1, l2) = (&text[..k], &text[k + 1..]);
            let (w1, w2) = (width(l1), width(l2));
            let (x1, x2) = (0xe0 - (w1 >> 1), 0xe0 - (w2 >> 1));
            let (x, w) = if x2 <= x1 { (x2, w2) } else { (x1, w1) };
            out.push(Self::sprite(mid, x + 0x20, y, w, 0x36, a, Rot::None));
            out.push(Self::sprite(cap, x, y, 0x20, 0x36, a, Rot::None));
            out.push(Self::sprite(cap, x + w + 0x20, y, 0x20, 0x36, a, Rot::R180));
            centred(l1, y + 8, out);
            centred(l2, y + 0x1b, out);
        } else {
            let w = width(text);
            let x = 0xe0 - (w >> 1);
            out.push(Self::sprite(cap, x, y, 0x20, 0x20, a, Rot::None));
            out.push(Self::sprite(mid, 0x100 - (w >> 1), y, w, 0x20, a, Rot::None));
            out.push(Self::sprite(cap, x + w + 0x20, y, 0x20, 0x20, a, Rot::R180));
            centred(text, y + 8, out);
        }
    }

    /// 0x24f3b0. The middle bar's width is `x + len` (x = the anchor, 20) and the right cap follows it, so the
    /// bar is `x` pixels longer than `len` would suggest; that is what the code draws. In a wide frame the slot moves
    /// out by [`Self::side_extra`] but that width term stays the game's 20 (port-only, crate `rc-engine` display).
    fn draw_weapon(&self, i: usize, out: &mut Vec<Draw>) {
        let s = &self.slots[i];
        if s.slide == 0 { return; }
        let y = ELEMENT_Y;
        let x = self.anchor(i).0;
        let x0 = ANCHORS[i].0;
        let (sf, f) = Self::fractions(s, WEAPON_STEPS, WEAPON_STEPS);
        let a = (sf * 128.0) as i32;
        let bar_a = (a as f32 * 0.7) as i32;
        let len = WEAPON_LEN[(s.max > 99) as usize];
        let lw = (len as f32 * sf) as i32;
        let (cap, mid) = (self.assets.icon_frame(icon::BAR, 1), self.assets.icon_frame(icon::BAR, 0));
        out.push(Self::sprite(cap, x - 0x1c, y, 32, 32, bar_a, Rot::None));
        out.push(Self::sprite(mid, x + 4, y, x0 + lw, 32, bar_a, Rot::None));
        out.push(Self::sprite(cap, x + 4 + x0 + lw, y, 32, 32, bar_a, Rot::R180));
        self.hud_frame(i, self.assets.icon_frame(s.icon, 3), x, y, 0, a, out);
        let text = format!("{}/{}", s.shown, s.max).into_bytes();
        let colour = if s.shown != 0 { tween_color(f, TEXT_FROM, TEXT_TO) } else { tween_color(f, EMPTY_FROM, EMPTY_TO) };
        let shadow = tween_color(f, 0, 0x8000_0000);
        let (tx, ty) = (x + WEAPON_TEXT_D.0 + len, y + WEAPON_TEXT_D.1);
        self.text_right(tx + 1, ty + 1, shadow, &text, out);
        self.text_right(tx, ty, colour, &text, out);
    }

    /// The banner part of 0x24fb50 (ramp ±0x80/ScaleTicks(8) per tick) and its draw 0x251b88. Not drawn (and not
    /// counted down) while the game mode 0x15f5c4 is not 0 (a scene, a movie); a countdown reaching 1000 drops to 0:
    /// the long banners (`ShowPlanetBanner`'s `ticks(0x49c)`) show for their first 180 ticks.
    fn draw_banner(&mut self, out: &mut Vec<Draw>) {
        let mode = self.inputs.mode;
        let b = &mut self.banner;
        if (b.countdown == 0 && b.alpha == 0) || mode != 0 {
            b.y = 100;
            return;
        }
        let step = 0x80 / scale_ticks(8);
        if b.countdown == 0 {
            b.alpha -= step;
            if b.alpha < 0 { b.alpha = 0; }
        } else {
            b.alpha = (b.alpha + step).min(0x80);
        }
        let colour = (b.alpha as u32) << 24 | 0x00f0_f0f0;
        let (x, y, text) = (0x100, b.y, b.text.clone());
        if b.countdown != 0 { b.countdown -= 1; }
        if b.countdown == 1000 { b.countdown = 0; }
        self.draw_banner_text(x, y, colour, text, out);
    }

    /// `DrawBanner` 0x251b88 (x, y, colour, text): shadow, bar frame behind the text (alpha ≤ 0x50), text.
    fn draw_banner_text(&self, x: i32, y: i32, colour: u32, text: Vec<u8>, out: &mut Vec<Draw>) {
        let a = ((colour as i32) >> 24).min(0x50);
        let w = self.assets.text_width(Font::Large, &text);
        let left = (x + 1) - (w >> 1);
        out.push(Draw::Text { font: Font::Large, x: left, y: y + 1, rgba: colour & 0xff00_0000, text: text.clone() });
        self.stretch_frame(left - 0x20, y - 8, (x - (left - 0x20)) * 2, 0x20, a, out);
        out.push(Draw::Text { font: Font::Large, x: x - (w >> 1), y, rgba: colour, text });
    }

    /// Level05 0x262f50: on the board, the trick texts by the meter's phase (1 in the air / 2 crashed: the spin in degrees
    /// and "xN Poses" centred at (350, 27) / (350, 47), faded in over 10 ticks, coloured by the fade (red when crashed);
    /// 3 landed: the combo's score and name (msg 0x5092 + combo) or "trick + N Flips" (msgs 0x517e + trick) as banners at
    /// (250, 101) / (250, 125)); the meter bar (icon 0x7558: frame 1 cut at `shown·221/max + 27`, frame 0 over it, 256 × 64
    /// at the slot's place); the wrong-way banner 0x531a at (256, 180) pulsing once a second past ticks(120) the wrong way.
    fn draw_race_meter(&self, i: usize, out: &mut Vec<Draw>) {
        if self.inputs.group != 0x16 { return; }
        let v = self.inputs.race_meter;
        let m = v.meter;
        let label = |id: i32| strings::lookup(&self.assets.messages, id).to_vec();
        let width = |t: &[u8]| self.assets.text_width(Font::Large, t);
        let centred = |x: i32, y: i32, rgba: u32, t: &[u8], out: &mut Vec<Draw>| out.push(Draw::Text { font: Font::Large, x: x - (width(t) >> 1), y, rgba, text: t.to_vec() });
        if m.phase != 0 {
            let f = (m.t as f32 / 10.0).min(1.0);
            let n = scale_ticks(0x14);
            let base = if m.phase == 1 || m.phase == 3 { tween_color(m.fade as f32 / n as f32, 0x00e0_8060, 0x00ff_b080) } else { 0x0000_00ff };
            let alpha = ((f * 128.0) as i32 as u32) << 24;
            let c = base.wrapping_add(alpha);
            if m.phase == 1 || m.phase == 2 {
                let t = format!("{}", m.flips * 0x168).into_bytes();
                centred(0x15f, 0x1c, alpha, &t, out);
                centred(0x15e, 0x1b, c, &t, out);
                let t = format!("x{} Poses", m.kinds).into_bytes();
                centred(0x15f, 0x30, alpha, &t, out);
                centred(0x15e, 0x2f, c, &t, out);
            } else {
                let mut combo = -1;
                if m.kinds == 3 {
                    combo = 0;
                    if m.used[3] != 0 { combo = if m.used[2] == 0 { 1 } else if m.used[1] != 0 { 3 } else { 2 }; }
                } else if m.kinds == 4 {
                    combo = 4;
                }
                if 0 <= combo {
                    let tier = if 4 <= m.flips { 2 } else { (2 <= m.flips) as i32 };
                    self.draw_banner_text(0xfa, 0x7d - 0x18, c, format!("{}", m.combo).into_bytes(), out);
                    self.draw_banner_text(0xfa, 0x7d, c, label(combo + tier * 5 + 0x5092), out);
                } else if 0 < m.kinds {
                    let tricks: Vec<i32> = (0..4).filter(|&k| m.used[k as usize] != 0).take(m.kinds as usize).collect();
                    let name = |k: usize| tricks.get(k).map_or(Vec::new(), |&t| label(t + 0x517e));
                    self.draw_banner_text(0xfa, 0x7d - 0x18, c, format!("{}", m.combo).into_bytes(), out);
                    let mut t = name(0);
                    if m.kinds == 1 {
                        t.extend_from_slice(format!(" + {} Flips", m.flips).as_bytes());
                    } else {
                        t.extend_from_slice(b" + ");
                        t.extend_from_slice(&name(1));
                        t.extend_from_slice(format!(" + {} Flips", m.flips).as_bytes());
                    }
                    self.draw_banner_text(0xfa, 0x7d, c, t, out);
                }
            }
        }
        let (x, y) = self.place(i);
        let s = &self.slots[i];
        let w = if s.max == 0 { 0 } else { (s.shown * 0xdd) / s.max + 0x1b };
        out.push(Draw::SpriteSub { frame: self.assets.icon_frame(0x7558, 1), x, y, w, h: 0x40, alpha: 0x80 });
        out.push(Self::sprite(self.assets.icon_frame(0x7558, 0), x, y, 0x100, 0x40, 0x80, Rot::None));
        if scale_ticks(0x78) < v.wrong_way as i32 {
            let n = scale_ticks(0x3c);
            let f = (self.inputs.tick % n.max(1) as u64) as f32 / n as f32;
            // The game's literals 6.28318 / 3.14159 (0x40c90fd0 / 0x40490fd0).
            let k = (f * f32::from_bits(0x40c9_0fd0) - f32::from_bits(0x4049_0fd0)).sin() * 0.5 + 0.5;
            let c = tween_color(k, 0x8020_2080, 0x8020_20c0);
            let t = label(0x531a);
            self.draw_banner_text(0x101, 0xb5, 0x8000_0000, t.clone(), out);
            self.draw_banner_text(0x100, 0xb4, c, t, out);
        }
    }

    /// Level05 0x266320 (level16 0x238a30): the race's lap and place on a bar across the bottom (`FontPrintLarge`, each
    /// with a black shadow one pixel off): "Lap: n/3" from x 30 (the lap + 1, at most 3), "Place: nst / nd / rd / th"
    /// ending at x 480. On level 16 (0x15ed84) also the board weapons' box ([`Self::draw_race_weapons`]).
    fn draw_race_lap(&self, i: usize, out: &mut Vec<Draw>) {
        if self.slots[i].counter < 1 { return; }
        let inp = &self.inputs;
        let pad = if inp.pal { 10 } else { 0x12 };
        let frame_y = SCREEN_H - (0x14 + pad);
        let y = SCREEN_H - (0xc + pad);
        self.stretch_frame(0, frame_y, 0x208, 0x20, 0x60, out);
        let label = |id: i32| strings::lookup(&self.assets.messages, id).to_vec();
        let width = |t: &[u8]| self.assets.text_width(Font::Large, t);
        let text = |x: i32, y: i32, rgba: u32, t: &[u8], out: &mut Vec<Draw>| out.push(Draw::Text { font: Font::Large, x, y, rgba, text: t.to_vec() });
        let (c1, c2) = (0x80e0_8060u32, 0x80ff_b080u32);
        let x = 0x1e;
        let mut t = label(0x523f);
        t.extend_from_slice(b": ");
        text(x - 1, y + 1, 0x8000_0000, &t, out);
        text(x, y, c1, &t, out);
        let w = width(&t);
        let lap = format!(" {}/3", (inp.race_lap + 1).min(3)).into_bytes();
        text(x + w - 1, y + 2, 0x8000_0000, &lap, out);
        text(x + w, y, c2, &lap, out);
        let place = match inp.race_place {
            1 => " 1st".to_string(),
            2 => " 2nd".to_string(),
            3 => " 3rd".to_string(),
            n => format!(" {n}th"),
        };
        let place = place.into_bytes();
        let px = 0x1e0 - width(&place);
        text(px + 1, y + 1, 0x8000_0000, &place, out);
        text(px, y, c2, &place, out);
        let mut l = b" ".to_vec();
        l.extend_from_slice(&label(0x5240));
        l.push(b':');
        let lx = px - width(&l);
        text(lx + 1, y + 1, 0x8000_0000, &l, out);
        text(lx, y, c1, &l, out);
        if inp.level == 16 { self.draw_race_weapons(out); }
    }

    /// Level16 0x238a30's tail: the board weapons' box: before Rilgar's race is won (global flag 0) at the slot's anchor
    /// x and y 18 (10 on PAL), after it at (110, 55) (gp−0x7240 / −0x723c, beside the time trial's meter); the frame
    /// from x − 24, 110 wide (gp−0x7238), 32 high, alpha 0x60; the icon 0x7558 frame 2 (32 × 32); the count ("%d")
    /// at x + 40 with its shadow, dim red 0x80202080 at 0, else 0x80e08060.
    fn draw_race_weapons(&self, out: &mut Vec<Draw>) {
        let inp = &self.inputs;
        let (x, y) = if inp.race_won { (110, 55) } else { (self.anchor(5).0, if inp.pal { 10 } else { 0x12 }) };
        self.stretch_frame(x - 0x18, y, 110, 0x20, 0x60, out);
        out.push(Self::sprite(self.assets.icon_frame(0x7558, 2), x, y, 0x20, 0x20, 0x80, Rot::None));
        let n = format!("{}", inp.race_weapons).into_bytes();
        out.push(Draw::Text { font: Font::Large, x: x + 0x29, y: y + 8, rgba: 0x8000_0000, text: n.clone() });
        let c = if inp.race_weapons == 0 { 0x8020_2080 } else { 0x80e0_8060 };
        out.push(Draw::Text { font: Font::Large, x: x + 0x28, y: y + 7, rgba: c, text: n });
    }

    /// Level05 0x266710: the race's time "Time:  m:ss:hh" centred on x 256 (3600 ticks a minute, 3000 on PAL); after
    /// the race is won (global flag 0) a bar above it with the score "Score: n".
    fn draw_race_time(&self, i: usize, out: &mut Vec<Draw>) {
        if self.slots[i].counter < 1 { return; }
        let inp = &self.inputs;
        let pad = if inp.pal { 10 } else { 0x12 };
        let label = |id: i32| strings::lookup(&self.assets.messages, id).to_vec();
        let width = |t: &[u8]| self.assets.text_width(Font::Large, t);
        let text = |x: i32, y: i32, rgba: u32, t: &[u8], out: &mut Vec<Draw>| out.push(Draw::Text { font: Font::Large, x, y, rgba, text: t.to_vec() });
        let (c1, c2) = (0x80e0_8060u32, 0x80ff_b080u32);
        let minute = if inp.pal { 3000 } else { 0xe10 };
        let second = minute / 0x3c;
        let (m, rest) = (inp.race_ticks / minute, inp.race_ticks % minute);
        let secs = rest / second;
        if inp.race_won {
            self.stretch_frame(0x8c, SCREEN_H - (0x2e + pad), 0xeb, 0x20, 0x60, out);
            let y = SCREEN_H - (0x26 + pad);
            let mut full = label(0x50a6);
            full.extend_from_slice(format!(": {}", inp.race_score).as_bytes());
            let x = 0x100 - (width(&full) >> 1);
            let mut t = label(0x50a6);
            t.extend_from_slice(b": ");
            text(x + 1, y + 1, 0x8000_0000, &t, out);
            text(x, y, c1, &t, out);
            let w = width(&t);
            let n = format!(" {}", inp.race_score).into_bytes();
            text(x + w + 1, y + 1, 0x8000_0000, &n, out);
            text(x + w, y, c2, &n, out);
        }
        let y = SCREEN_H - (0xc + pad);
        let mut probe = label(0x5241);
        probe.extend_from_slice(b":  0:00:00");
        let x = 0x100 - (width(&probe) >> 1);
        let mut t = label(0x5241);
        t.extend_from_slice(b":  ");
        text(x + 1, y + 1, 0x8000_0000, &t, out);
        text(x, y, c1, &t, out);
        let w = width(&t);
        let n = format!("{}:{:02}:{:02}", m, secs, ((rest - secs * second) * 100) / second).into_bytes();
        text(x + w + 1, y + 1, 0x8000_0000, &n, out);
        text(x + w, y, c2, &n, out);
    }

    /// `draw_stretchable_ui_frame` 0x251ab0: cap, stretched middle, rotated cap.
    pub fn stretch_frame(&self, x: i32, y: i32, w: i32, h: i32, alpha: i32, out: &mut Vec<Draw>) {
        let (mid, cap) = (self.assets.icon_frame(icon::BAR, 0), self.assets.icon_frame(icon::BAR, 1));
        out.push(Self::sprite(cap, x, y, 0x20, h, alpha, Rot::None));
        out.push(Self::sprite(mid, x + 0x20, y, w - 0x40, h, alpha, Rot::None));
        out.push(Self::sprite(cap, x + w - 0x20, y, 0x20, h, alpha, Rot::R180));
    }

    /// 0x2266c0.
    fn draw_help(&mut self, out: &mut Vec<Draw>) {
        let text = self.help_text().to_vec();
        let h = &mut self.help;
        // Every state's frame needs the text option 0x15ee1d (the voice option alone plays the line, draws nothing).
        if h.state == 0 || !h.enabled || !h.text_on { return; }
        let frame = |top, bottom, left, right, alpha| Draw::UiFrame { top, bottom, left, right, alpha };
        let logo = |cx: i32, cy: i32, a: i32| Draw::FxQuad { fx: 4, x: cx - 0x20, y: cy - 0x20, w: 0x40, h: 0x40, u: 0, v: 0, tw: 0x40, th: 0x40, rgba: (a as u32) << 24 | 0x0080_8080 };
        match h.state {
            1 => {
                h.cur_w = h.t * 4 + 8;
                h.cur_h = h.cur_w;
                out.push(frame(h.cy - h.cur_w, h.cy + h.cur_w, h.cx - h.cur_w, h.cx + h.cur_w, 0x60));
            }
            2 => {
                // Help_DrawPrompt 0x2265d8: logo alpha 0x7e in this state.
                h.cur_w = 0x20;
                h.cur_h = 0x20;
                out.push(frame(h.cy - 0x20, h.cy + 0x20, h.cx - 0x20, h.cx + 0x20, 0x60));
                out.push(logo(h.cx, h.cy, 0x7e));
            }
            3 => {
                h.cur_w = (h.half_w - 0x20) * h.t / 8 + 0x20;
                h.cur_h = (h.half_h - 0x20) * h.t / 8 + 0x20;
                out.push(frame(h.cy - h.cur_h, h.cy + h.cur_h, h.cx - h.cur_w, h.cx + h.cur_w, 0x60));
                out.push(logo(h.cx, h.cy, ((8 - h.t) << 4).max(0)));
            }
            4..=6 => {
                h.cur_w = h.half_w;
                h.cur_h = h.half_h;
                out.push(frame(h.cy - h.half_h, h.cy + h.half_h, h.cx - h.half_w, h.cx + h.half_w, 0x60));
                let rgba = match h.state {
                    4 => HELP_COLOUR | (h.t as u32) << 29,
                    6 => HELP_COLOUR | ((4 - h.t) as u32) << 29,
                    _ => 0x80ff_a888,
                };
                out.push(Draw::TextWindow { font: Font::Small, window: crate::help::window(h.cy, 3), rgba, text });
            }
            7 => {
                let hh = h.cur_h - (h.cur_h - 8) * h.t / 8;
                let hw = h.cur_w - (h.cur_w - 8) * h.t / 8;
                out.push(frame(h.cy - hh, h.cy + hh, h.cx - hw, h.cx + hw, (8 - h.t) * 12));
            }
            _ => {}
        }
    }
}

/// `FontPrintWindow`'s layout (0x21db48): line breaking, balancing and placement. The engine draws the lines
/// with `FontPrint` inside the window's scissor; the help box uses it to measure.
pub mod text {
    use rc_formats::font::{measure_text_width, GlyphTable};

    /// Flags (window +0x12).
    pub const CENTRE_LINES: u16 = 1;
    pub const CENTRE_BLOCK: u16 = 2;
    pub const MEASURE_ONLY: u16 = 4;
    /// Sub-pixel float path (`0x21d0f0`); not used by the ported callers, drawn like the integer path.
    pub const FLOAT_POS: u16 = 8;

    /// The window record (shorts, `FontSetWindow` 0x21e120).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Window {
        pub y_min: i16,
        pub y_max: i16,
        pub x_min: i16,
        pub x_max: i16,
        pub x_anchor: i16,
        pub y_start: i16,
        /// Out: widest drawn line.
        pub max_width: i16,
        /// Out: lines × line height.
        pub height: i16,
        pub line_height: i16,
        pub flags: u16,
        /// 1/16-pixel offsets of the float path.
        pub sub_x: i16,
        pub sub_y: i16,
    }

    impl Window {
        /// `FontSetWindow(win, y_min, y_max, x_min, x_max, x_anchor, y_start, line_height, flags)`.
        #[allow(clippy::too_many_arguments)]
        pub fn new(y_min: i16, y_max: i16, x_min: i16, x_max: i16, x_anchor: i16, y_start: i16, line_height: i16, flags: u16) -> Self {
            Window { y_min, y_max, x_min, x_max, x_anchor, y_start, max_width: 0, height: 0, line_height, flags, sub_x: 0, sub_y: 0 }
        }
    }

    /// A line `FontPrintWindow` draws: `count` bytes from `start`, pen at (x, y), starting colour slot
    /// `colour` of the font colour table (0 = the caller's colour).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Line {
        pub start: usize,
        /// Bytes to print (`FontPrint`'s len: ≤ 0 prints to the NUL, like the game).
        pub count: i32,
        pub colour: u8,
        pub x: i32,
        pub y: i32,
        pub width: i32,
    }

    /// Wraps `text` (first `len` bytes, all when `len < 0`), fills `win.max_width` / `win.height` and returns
    /// the lines inside the window's y range (all of them, also with [`MEASURE_ONLY`]). `colour_codes` is the
    /// game's colour-code switch 0x15f45c (1 during play).
    pub fn layout(win: &mut Window, text: &[u8], len: i32, glyphs: &GlyphTable, colour_codes: bool) -> Vec<Line> {
        let at = |i: i32| -> u8 { if i < 0 { 0 } else { text.get(i as usize).copied().unwrap_or(0) } };
        let adv = |c: u8| -> i32 { glyphs.get(c as usize).map_or(0, |g| g.advance as i32) };
        let (x_min, x_max, anchor) = (win.x_min as i32, win.x_max as i32, win.x_anchor as i32);
        let w0 = if win.flags & CENTRE_LINES != 0 { 2 * (x_max - anchor).min(anchor - x_min) } else { x_max - anchor };
        let mut colour = 0i32;
        let (mut final_pass, mut n0, mut last_w, mut w) = (false, 0usize, 0i32, w0);
        // (start, end inclusive, colour slot at the line start)
        let mut lines: Vec<(i32, i32, i32)>;
        loop {
            lines = Vec::new();
            let mut pos = 0i32;
            if len != 0 && at(0) != 0 {
                loop {
                    let start = pos;
                    let line_colour = colour;
                    let mut brk = pos;
                    let mut width = 0i32;
                    if w > 0 {
                        loop {
                            let c = at(pos);
                            if c == 0x20 || c < 0x10 { brk = pos; }
                            if colour_codes && c.wrapping_sub(8) < 8 { colour = c as i32 - 8; }
                            if c < 2 { break; }
                            pos += 1;
                            width += adv(c);
                            if width >= w { break; }
                        }
                    }
                    let mut end = if brk as i16 == start as i16 { pos } else { brk };
                    let e = end;
                    let ce = at(e);
                    if ce == 0x20 || ce < 0x10 { end -= 1; }
                    lines.push((start, end, line_colour));
                    if ce == 0 {
                        last_w = width;
                        break;
                    }
                    pos = e + 1;
                    if pos == len || at(pos) == 0 { break; }
                }
            }
            let n = lines.len();
            if final_pass { break; }
            if n0 == 0 { n0 = n; }
            if n < 2 { break; }
            if n0 < n {
                final_pass = true;
                w = w0;
                continue;
            }
            let third = w / 3;
            w -= 16;
            if last_w >= third { break; }
        }

        let lh = win.line_height as i32;
        let total = lines.len() as i32 * lh;
        win.max_width = 0;
        win.height = total as i16;
        let mut y = win.y_start as i32;
        if win.flags & CENTRE_BLOCK != 0 { y -= total >> 1; }
        let mut out = Vec::new();
        for &(start, end, col) in &lines {
            if !(y + lh < win.y_min as i32 || (win.y_max as i32) < y) {
                let count = end - start + 1;
                let from = text.get(start.max(0) as usize..).unwrap_or(&[]);
                let width = measure_text_width(from, count, glyphs);
                if (win.max_width as i32) < width { win.max_width = width as i16; }
                let x = if win.flags & CENTRE_LINES != 0 { anchor - (width >> 1) } else { anchor };
                out.push(Line { start: start.max(0) as usize, count, colour: col as u8, x, y, width });
            }
            y += lh;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::font::{Glyph, GLYPHS};

    fn glyphs() -> GlyphTable {
        let mut t = [Glyph::default(); GLYPHS];
        for g in &mut t[0x21..0x7b] { g.advance = 10; }
        t[b' ' as usize].advance = 5;
        for d in b'0'..=b'9' { t[d as usize].advance = 13; }
        t[b'\'' as usize].advance = 4;
        t[b'.' as usize].advance = 6;
        t
    }

    fn assets() -> HudAssets {
        let mut icons = vec![IconEntry { id: 0, frame_count: 1, first_frame: 0, ..Default::default() }];
        let mut next = 1u16;
        for (id, n) in [(icon::ORB, 32u16), (icon::BOLT, 30), (icon::BAR, 2), (60010, 5)] {
            icons.push(IconEntry { id, frame_count: n, first_frame: next, ..Default::default() });
            next += n;
        }
        icons.push(IconEntry { id: 0xffff, ..Default::default() });
        let g = glyphs();
        let messages = vec![Message { id: 1000, text: b"Gadgetron \x0cInfobots\x08 give you coordinates for new planets.".to_vec(), help_audio: 4 }];
        HudAssets { icons, frame_sizes: vec![(32, 32); next as usize], glyphs: [g, g, g], messages }
    }

    fn inputs(hp: i32, bolts: i32) -> Inputs { Inputs { hp, bolts, ..Default::default() } }

    /// The context prompt (slot 12, 0x24c898): requested while an owner holds it, slides in over 8 ticks (alpha
    /// 16·slide), one or two lines in a bar frame centred on x 256 at y 50, and fades out once no longer requested.
    #[test]
    fn prompt_slot_12_shows_while_requested() {
        let mut h = HudState::new(assets());
        h.set_prompt(true, b"\x12 Activate");
        let mut d = Vec::new();
        for _ in 0..60 { d = h.tick(inputs(4, 0)); }
        let s = &h.slots[12];
        assert_eq!((s.element, s.slide), (Element::Prompt, 8));
        let text: Vec<_> = d.iter().filter_map(|x| if let Draw::Text { y, rgba, text, .. } = x { Some((*y, *rgba, text.clone())) } else { None }).collect();
        assert_eq!(text, vec![(58, 0x8040_f040, b"\x12 Activate".to_vec())]);
        let sprites = d.iter().filter(|x| matches!(x, Draw::Sprite { y: 50, h: 32, alpha: 128, .. })).count();
        assert_eq!(sprites, 3, "caps and middle");
        // Two lines split at 0x01: frame 54 tall, the second line 19 lower.
        h.set_prompt(true, b"Buy\x01now");
        let d = h.tick(inputs(4, 0));
        let ys: Vec<i32> = d.iter().filter_map(|x| if let Draw::Text { y, .. } = x { Some(*y) } else { None }).collect();
        assert_eq!(ys, vec![58, 77]);
        assert!(d.iter().any(|x| matches!(x, Draw::Sprite { h: 0x36, .. })));
        // No longer requested: the timer (≥ 10) runs out, then alpha and slide ramp down.
        h.set_prompt(false, b"");
        for _ in 0..60 { h.tick(inputs(4, 0)); }
        assert_eq!(h.slots[12].slide, 0);
    }

    /// Mounted (hero state 0x32) the health and bolt draws draw nothing while their slots stay up (0x24e418 /
    /// 0x24ea00); back in any other state they draw again.
    #[test]
    fn mounted_hides_health_and_bolts() {
        let mut h = HudState::new(assets());
        h.tick(inputs(4, 0));
        let mut d = Vec::new();
        for _ in 0..20 { d = h.tick(inputs(3, 1234)); }
        assert!(!d.is_empty(), "shown after the change");
        let up: Vec<Element> = h.slots.iter().filter(|s| s.slide != 0).map(|s| s.element).collect();
        assert!(up.contains(&Element::Health) && up.contains(&Element::Bolts), "{up:?}");
        let mounted = Inputs { hero_state: crate::hero::scripted::MOUNTED, ..inputs(3, 1234) };
        assert!(h.tick(mounted).is_empty(), "mounted: nothing drawn");
        assert!(h.slots.iter().any(|s| s.element == Element::Health && s.slide != 0), "the slot stays up");
        assert!(!h.tick(inputs(3, 1234)).is_empty(), "drawn again");
    }

    /// `HudDraw` 0x24a7f8: a countdown reaching 1000 drops to 0, so `ShowPlanetBanner`'s `ticks(0x49c)` shows for 180
    /// ticks (then fades over 8), not 1180; outside game mode 0 the banner is neither drawn nor counted down.
    #[test]
    fn long_banners_stop_at_1000_and_wait_outside_gameplay() {
        let banner_up = |d: &[Draw]| d.iter().any(|x| matches!(x, Draw::Text { text, .. } if text == b"Kerwan"));
        let mut h = HudState::new(assets());
        h.show_banner(b"Kerwan", Some(0x49c));
        let mut shown = 0;
        for _ in 0..400 { if banner_up(&h.tick(inputs(4, 0))) { shown += 1; } }
        assert!((180..=190).contains(&shown), "shown {shown} ticks");
        let mut h = HudState::new(assets());
        h.show_banner(b"Kerwan", Some(300));
        let mut scene = inputs(4, 0);
        scene.mode = 2;
        for _ in 0..500 { assert!(!banner_up(&h.tick(scene))); }
        assert_eq!(h.banner.countdown, 300, "paused while the mode is not 0");
        // `ShowBanner(msg, −1)` (Blarg's bridge, the skill points): ticks(180), not a countdown that never ends.
        h.show_banner(b"Kerwan", Some(-1));
        assert_eq!(h.banner.countdown, scale_ticks(180));
    }

    /// A boss meter (slot 6, flag 0x10: its timer is kept up) leaves at once on `fun_001ff480`'s release; waiting for it
    /// to hide never ended (the Blarg queen's and Umbris's meters stayed after the fight).
    #[test]
    fn released_boss_meter_leaves() {
        let mut h = HudState::new(assets());
        let r = Request::boss(7, 0x1ef);
        for t in 0..200 {
            h.apply_calls(&[Call::Data(7, 300 - t), Call::Queue(r)]);
            h.tick(inputs(4, 0));
        }
        assert!(!h.tick(inputs(4, 0)).is_empty(), "the meter is up");
        h.apply_calls(&[Call::Data(7, 0), Call::Release(r)]);
        assert_eq!(h.slots[6].element, Element::Empty);
        for _ in 0..10 { assert!(h.tick(inputs(4, 0)).is_empty(), "nothing left on screen"); }
    }

    /// `PromptRelease` (the ship's △) empties the prompt's slot at once, without a tick: the take-off hides the HUD and
    /// the planet page freezes it, so the prompt stayed behind the Galactic Map.
    #[test]
    fn released_prompt_leaves_without_a_tick() {
        let mut h = HudState::new(assets());
        h.set_prompt(true, b"Enter ship");
        for _ in 0..30 { h.tick(inputs(4, 0)); }
        assert!(!h.draws_now().is_empty(), "the prompt is up");
        assert!(h.release_prompt());
        assert_eq!(h.slots[12].element, Element::Empty);
        assert!(h.draws_now().is_empty(), "nothing left on screen");
        for _ in 0..10 { assert!(h.tick(inputs(4, 0)).is_empty(), "and it does not come back"); }
    }

    #[test]
    fn nothing_on_screen_until_something_changes() {
        let mut h = HudState::new(assets());
        for _ in 0..300 { assert!(h.tick(inputs(4, 0)).is_empty()); }
    }

    /// First pickup: the counter is armed (init timer ScaleTicks(120)+30 = 150), slides in over 8 ticks, fades
    /// in over 8, holds, and ramps out (alpha first) once the timer drops below 5.
    #[test]
    fn bolt_ramp_timings() {
        let mut h = HudState::new(assets());
        h.tick(inputs(4, 0));
        let mut seen = Vec::new();
        for k in 0..200 {
            h.tick(inputs(4, if k == 0 { 0 } else { 5 }));
            let s = h.slots[2];
            seen.push((s.slide, s.alpha));
        }
        // Armed on tick 1 (the change): applied the same tick (slot hidden, counter −6), slide 1.
        assert_eq!(seen[1], (1, 0));
        assert_eq!(seen[8], (8, 0));
        assert_eq!(seen[9], (8, 1));
        assert_eq!(seen[16], (8, 8));
        // Timer 150 at arming, −1 per later tick: < 5 from 146 ticks after arming.
        assert_eq!(seen[1 + 145], (8, 8));
        assert_eq!(seen[1 + 146], (8, 7));
        assert_eq!(seen[1 + 153], (8, 0));
        assert_eq!(seen[1 + 154], (7, 0));
        assert_eq!(seen[1 + 161], (0, 0));
        assert_eq!(h.slots[2].counter, -6);
    }

    /// Later changes show the counter for 90 ticks (the update's ScaleTicks(90)).
    #[test]
    fn bolt_visible_90_ticks_after_a_change() {
        let mut h = HudState::new(assets());
        h.tick(inputs(4, 0));
        for _ in 0..200 { h.tick(inputs(4, 1)); }
        assert_eq!((h.slots[2].slide, h.slots[2].alpha), (0, 0));
        let mut frames_with_draws = 0;
        let mut full = Vec::new();
        for k in 0..200 {
            let d = h.tick(inputs(4, 1234));
            if !d.is_empty() { frames_with_draws += 1; }
            if (h.slots[2].slide, h.slots[2].alpha) == (8, 8) { full.push(k); }
        }
        // Change seen at k = 0: timer 90, then 89, 88, …; below 5 from k = 86.
        assert_eq!((full[0], *full.last().unwrap()), (15, 85));
        // Alpha 7 at k = 86 … 0 at 93, slide 7 at 94 … 0 at 101: drawn (slide > 0) for k = 0 ..= 100.
        assert_eq!(frames_with_draws, 101);
    }

    #[test]
    fn health_shows_120_ticks_after_damage_and_always_at_one_hp() {
        let mut h = HudState::new(assets());
        h.tick(inputs(4, 0));
        h.tick(inputs(3, 0));
        // Init: timer ScaleTicks(180) + 30.
        assert_eq!((h.slots[1].element, h.slots[1].timer, h.slots[1].offset), (Element::Health, 210, (32, 0)));
        for _ in 0..400 { h.tick(inputs(3, 0)); }
        assert_eq!(h.slots[1].slide, 0);
        for _ in 0..400 { h.tick(inputs(1, 0)); }
        assert_eq!((h.slots[1].slide, h.slots[1].alpha), (8, 8), "1 HP keeps the timer at 120");
    }

    fn texts(d: &[Draw]) -> Vec<(i32, i32, Vec<u8>)> {
        d.iter().filter_map(|x| match x { Draw::Text { x, y, text, .. } => Some((*x, *y, text.clone())), _ => None }).collect()
    }

    #[test]
    fn bolt_separators_english_and_german() {
        for (lang, sep, (dx, dy)) in [(strings::lang::ENGLISH, b'\'', (-30, 21)), (strings::lang::GERMAN, b'.', (-29, 10))] {
            let mut h = HudState::new(assets());
            h.tick(Inputs { lang, ..inputs(4, 0) });
            let mut d = Vec::new();
            for _ in 0..20 { d = h.tick(Inputs { lang, ..inputs(4, 1_234_567) }); }
            let t = texts(&d);
            // Number: shadow + text, right-aligned at x − 32 (width 7 × 13).
            assert_eq!(t[1], (492 - 32 - 91, 18 + 8, b"1234567".to_vec()));
            assert_eq!(t[0], (492 - 32 - 91 + 1, 18 + 8 + 1, b"1234567".to_vec()));
            // Separators after the 3rd and 6th digit from the right: shadow at +2, +2.
            let w = if sep == b'.' { 6 } else { 4 };
            let seps: Vec<_> = t[2..].to_vec();
            assert_eq!(seps.len(), 4);
            for (j, k) in [3, 6].into_iter().enumerate() {
                assert_eq!(seps[2 * j], (492 + dx - 13 * k + 2 - w, 18 + dy + 2, vec![sep]));
                assert_eq!(seps[2 * j + 1], (492 + dx - 13 * k - w, 18 + dy, vec![sep]));
            }
        }
    }

    #[test]
    fn wrench_hides_the_weapon_slot_and_ammo_weapon_persists() {
        let mut h = HudState::new(assets());
        for _ in 0..50 { h.tick(Inputs { weapon: Some((10, 25, 40)), ..inputs(4, 0) }); }
        assert_eq!((h.slots[0].slide, h.slots[0].alpha, h.slots[0].shown), (8, 8, 25));
        for _ in 0..400 { h.tick(Inputs { weapon: Some((10, 25, 40)), ..inputs(4, 0) }); }
        assert_eq!((h.slots[0].slide, h.slots[0].alpha), (8, 8), "flag 0x10 keeps it up");
        for _ in 0..40 { h.tick(inputs(4, 0)); }
        assert_eq!((h.slots[0].slide, h.slots[0].alpha, h.slots[0].flags), (0, 0, 0));
    }

    /// The help box of the game tick (crate::help) drawn by the HUD: sized with the small font, centred at (256, H − 60),
    /// the grow frame and logo, then the frame and the text window.
    #[test]
    fn help_box_draws_from_the_help_system() {
        let a = assets();
        let mut help = crate::help::Help {
            text: crate::help::HelpText { messages: std::sync::Arc::new(a.messages.clone()), small: Some(a.glyphs[Font::Small as usize]), lang: 0 },
            ..Default::default()
        };
        help.bx.enabled = true;
        help.log_ids = std::sync::Arc::new(vec![(1000, 21107)]);
        assert!(help.request(1000, 4));
        let mut h = HudState::new(a);
        let mut seen = Vec::new();
        for _ in 0..45 {
            help.voice_frame();
            // The engine answers the line's load (its length in ticks).
            if help.out.voice.contains(&crate::help::VoiceCmd::Load { id: 30004 }) { help.voice_loaded(Some(30)); }
            help.out = Default::default();
            help.update(&crate::help::HelpInputs { text_on: true, voice_on: true, ..Default::default() });
            h.set_help(&help.bx);
            let d = h.tick(inputs(4, 0));
            seen.push((help.bx.state, d.iter().any(|x| matches!(x, Draw::FxQuad { fx: 4, .. })), d.iter().any(|x| matches!(x, Draw::TextWindow { .. }))));
        }
        assert_eq!((h.help.cx, h.help.cy), (256, 356));
        assert!(seen.iter().any(|&(s, logo, _)| s == 2 && logo));
        assert!(seen.iter().any(|&(s, _, text)| s == 5 && text));
        // The text option off: nothing is drawn.
        let mut b = help.bx.clone();
        b.text_on = false;
        h.set_help(&b);
        assert!(!h.tick(inputs(4, 0)).iter().any(|x| matches!(x, Draw::TextWindow { .. } | Draw::UiFrame { .. })));
    }

    #[test]
    fn window_wrap_balances_short_last_lines() {
        let g = glyphs();
        // Anchor 100, width 2·min(100, 100) = 200: "aaaa…" words of 5 letters (50 px) + space (5 px).
        let text = b"aaaaa aaaaa aaaaa aaaaa b";
        let mut w = text::Window::new(0, 400, 0, 200, 100, 50, 16, text::CENTRE_LINES);
        let lines = text::layout(&mut w, text, -1, &g, true);
        // Greedy at 200: 3 words + space (165 px), then "aaaa" overflows → break at the space: [3 words] (160 px),
        // [1 word + "b"] (65 px); 65 < 200/3 → retry at 184: same lines, 65 ≥ 184/3 → kept.
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[0].count, lines[1].start, lines[1].count), (17, 18, 7));
        assert_eq!(lines[0].x, 100 - (160 >> 1));
        assert_eq!((w.max_width, w.height), (160, 32));
        // Four words at W = 180: [3 words], [1 word] (50 < 60) → 164 (50 < 54) → 148: [2 words], [2 words].
        let text = b"aaaaa aaaaa aaaaa aaaaa";
        let mut w = text::Window::new(0, 400, 0, 180, 90, 50, 16, text::CENTRE_LINES);
        let lines = text::layout(&mut w, text, -1, &g, true);
        assert_eq!(lines.iter().map(|l| (l.start, l.count, l.width)).collect::<Vec<_>>(), [(0, 11, 105), (12, 11, 105)]);
        // Newline 0x01 forces a break. The colour slot is not reset between balancing passes (s2 is zeroed
        // before the pass loop only): the short last line makes the game re-wrap, and the second pass starts
        // line 0 with the 0x0c seen at the end of the first.
        let mut w = text::Window::new(0, 400, 0, 400, 0, 0, 16, 0);
        let lines = text::layout(&mut w, b"ab\x01\x0ccd", -1, &g, true);
        assert_eq!(lines.iter().map(|l| (l.start, l.count, l.colour)).collect::<Vec<_>>(), [(0, 2, 4), (3, 3, 4)]);
        let mut w = text::Window::new(0, 400, 0, 400, 0, 0, 16, 0);
        let lines = text::layout(&mut w, b"abcdefghijklmn\x01\x0cxy\x08z", -1, &g, true);
        assert_eq!(lines.iter().map(|l| (l.start, l.count, l.colour)).collect::<Vec<_>>(), [(0, 14, 0), (15, 5, 0)]);
    }
}
