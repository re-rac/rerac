//! **The Hologuise** (item 31 = 0x1f; its hand moby is class 483 = 0x1e3, the Drone Device's, which has no update) and
//! **the disguise it puts on**: body 3 of the hero (the moby class 0x27a, states 0x53..0x59: [`super::bodies::disguise`]),
//! on the shared body system ([`super::bodies`]: `SwitchCharacter` / leaving the body). Read from the level01 decompiler
//! output: the weapon check `HeroPdaGadget` 0x240ed8 case 0x1f, `UpdateWrenchSelected` 0x2307e0 (request 0x1f), the gate
//! `FUN_00230770`, `HeroTickStateTimer` 0x23c710 (the 18-tick timer), the squash `0x229158` and its table 0x179c58,
//! `HeroUpdateAlt` 0x228000 (body 3: the tint, the glow points, the way out), the hero's draw callback `0x229440` (body
//! 3: three glow quads), the SetState prologue (a state outside the disguise's leaves it).
//!
//! **How it plays.** Choosing the Hologuise (the quick select's request 0x141408 = 0x1f) or ○ with it saved in hand
//! starts the 18-tick timer 0x14162e with Ratchet's voice 0x18 and the squash (0x14162c: Ratchet's moby squeezed
//! across, 1 → 1.5 → 0.01, by the table); when the timer runs out the disguise moby 0x27a is made where he stands and the
//! hero switches into it (`SwitchCharacter(3, 0x53, moby)`): it walks, falls, takes hits and dies with its own states,
//! glows (its colour pulses teal every 170 ticks; 90 / 50 ticks and amber when the guards' alert 0x141630 is 1 / 2) and
//! draws three glow quads at its joint lists 0..2. □, another hand item chosen, or (Magneboots owned) standing on a
//! magnetic floor starts the timer again with voice 0x19 (the squash as well); when it runs out the hero leaves the
//! body (`0x231450`: the disguise moby deleted, Ratchet back where it stood), and □ held within 18 ticks takes the wrench
//! out (0x1413f6). Any `SetState` to a state outside the disguise's (0x53..0x59, 0x65..0x67, 0, 1) leaves it at once.
//!
//! **Coverage** (`address | what | status`):
//!
//! | address | what | status |
//! |---|---|---|
//! | 0x230770 | the gate: not in states 101..103 (`0x2495d0`), game mode 0x15f5c4 = 0, body 0x1413f4 = 0, movement group < 3 or 4, not in the water 0x140634 | ported ([`can_disguise`]) |
//! | 0x240ed8 case 0x1f | the gate, ○ (the slot's fire mask) pressed within `ticks(7)`, the saved hand item 0x141660 = 0x1f, the hand moby of class 0x1e3, the timer 0 → voice 0x18, squash 1, timer 0x12; then the epilogue | ported ([`fire`]) |
//! | 0x2307e0 request 0x1f | the gate: the timer 0 → voice 0x18, timer 0x12, squash 1; target = 0x1f, 0x15ed8c = the saved item, saved = 0x1f, a swap; the request cleared either way | ported (`items::update_hand_selected` → [`request`]) |
//! | 0x23c710 | the timer running and reaching 0 (`FastDecTimer_s16`), health ≠ 0, game mode 0: body 3 → `0x231450` (leave) and □ pressed within `ticks(18)` → 0x1413f6 = 1; else `CreateMoby(0x27a)` at the hero (+0x10 = 0x13f3d0, +0x40 = 0x13f3e0), +0x32 = 0x40, +0x31 = 1, +0x90 = 0, +0x38 = the hero moby's, `MobyBuildMatrix`, `SwitchCharacter(3, 0x53, moby)` | ported ([`Disguise::due`]: the leave right after the move in the body's update, `bodies::body_update`; the moby by `fx::MobySpawn::Disguise` and the switch through `Bodies::restore` at the start of the next tick [L: one tick later than the game's call inside the move]) |
//! | 0x229158 | squash ≠ 0: +1; the table 0x179c58 at the new index; −1 → 0; else the hero moby's rows +0xc0 / +0xd0 scaled; body 0: every item slot's mobys (0x1403e0 / 0x1403e4, the first one twice) scaled likewise (the hand's factor capped at 1) and pulled toward the body point 0x13f420 in x / y | ported ([`squash`], [`squash_hand`]: the hero moby and the hand item; the other slots' mobys are the items `rc-engine` moby_attach places, which get [`Disguise::factor`] after its attach matrices) |
//! | HeroUpdateAlt body 3 | cheat 0x15edb1: record 30's scale (0x17c04c) = 1.9; the pulse (period `ticks(170)`, 90 with alert 1, 50 with alert 2): s = sin(2π·t − π), colour (r, g, b) = (50 + 20s, 180 + 70s, 20 + 10s) (alert 2: (180 + 70s, (50 + 20s)/2, 20 + 10s)), alpha 0x80; +0x90 = `FastTweenColor(0.1, +0x90, colour)`, mode \|= 0x10; joint lists 0..2's points → 0x1410a0..; 0x141634 = alpha (0x30, or timer/18·48 while it runs) \| the glow's rgb; `0x229400` (the hero's draw callback, once a frame); 0x141630 = 0; the request 0 or 0x1f: cleared, and on a magnetic floor (0x140637) grounded with the Magneboots owned and the timer 0: voice 0x19, squash, timer; another request with the timer 0: the same | ported ([`body_update`]) |
//! | 0x229440 body 3 | three glow quads `0x2781d0(0.2, 0.08, 0x1410a0 + 16k, 0x141634)` | ported ([`Disguise::glow_points`], `Callback::DisguiseGlow`, drawn by `rc-engine` fx_draw) |
//! | SetState prologue | body 3 and the new state outside 0x53..0x59, 0x65..0x67, 0, 1: `0x22cd18` → leave the body; body 0: 0x141630 = 0 | ported (`states::set_state`) |
//! | 0x2405a0 | `SetState(0)` in a body: body 0 → `SetState(0, 1)`, body 3 → `SetState(0x53, 1)` | ported (`bodies::body_idle`) |
//! | the guards' alert 0x141630 | written by the guards 44 (L15 `0x2979d8`, L17 `0x29f8d8`: 1 suspicious, 2 alarmed) | ported (`classes::units::quartu_guard`'s tail through `HeroFields::disguise_alert`) |
//! | `0x236738` in body 3 | Ratchet's moby placed at the hero, then `PlayClassSound(index, flags, Ratchet)` (every hero voice of the body's update) | ported ([`DisguiseVoices`], [`queue_voices`]: played by the tick after the body's update [L: after the update instead of at the call]) |
//! | sounds, particles, lights, stats, bolts, save flags | Ratchet's voices 0x18 / 0x19; no particle, light, stat, bolt or save write | — |
//!
//! **Native.** Plain `f32`.

use super::physics::ticks;
use super::states::Ctx;
use super::Hero;

pub const HOLOGUISE: i32 = 0x1f;
/// The Hologuise's hand moby class (0x1e3 = 483, shared with the Drone Device).
pub const ITEM_CLASS: i16 = 0x1e3;
/// The timer's length (0x12) and the voices of putting on / taking off the disguise.
pub const TIMER: i16 = 0x12;
pub const VOICE_ON: i32 = 0x18;
pub const VOICE_OFF: i32 = 0x19;
/// The disguise's first state.
pub const IDLE: i32 = 0x53;

/// The squash table 0x179c58 (read at the incremented index; −1 ends it).
pub const SQUASH: [f32; 44] = [
    0.0, 1.0, 1.2, 1.35, 1.5, 1.35, 1.2, 1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.01, 0.01, 0.01, 0.1, 0.2, 0.3, 0.4, 0.5,
    0.6, 0.7, 0.8, 0.9, 1.0, 1.2, 1.35, 1.4, 1.45, 1.4, 1.35, 1.3, 1.25, 1.2, 1.15, 1.1, 1.05, 1.0, -1.0,
];

/// What the timer's end asks for (made where the hero has the context: [`Disguise::due`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Due {
    /// Make the disguise moby and switch into it.
    Enter,
    /// Leave the body; `wrench`: □ was pressed within 18 ticks (0x1413f6 = 1 after it).
    Leave { wrench: bool },
}

/// The disguise's hero-block fields.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Disguise {
    /// 0x14162c (u16): the squash's index (0: off).
    pub squash: u16,
    /// 0x14162e (s16): the timer.
    pub timer: i16,
    /// The timer's end of this tick, for the hero update ([`Due`]).
    pub due: Option<Due>,
    /// 0x141630 (s16): the guards' alert (0 calm, 1, 2), cleared by the body's update and by SetState in body 0.
    pub alert: i16,
    /// 0x141634: the glow quads' colour.
    pub tint: u32,
    /// 0x1410a0 / 0x1410b0 / 0x1410c0: the body's joint lists 0..2's points (the glow quads).
    pub glow_points: [[f32; 3]; 3],
    /// The disguise moby's glow word +0x90 as the tween last left it.
    pub glow: u32,
    /// The squash factor of this tick (None: no squash), for the tick's item pass ([`squash_hand`]) and for the
    /// items `rc-engine` places and draws (the back, feet, head and extra slots, kept until the next tick's [`squash`]).
    pub factor: Option<f32>,
}

/// `FUN_00230770`.
pub fn can_disguise(h: &Hero) -> bool {
    let menu = (101..=103).contains(&h.state);
    !menu && h.gadgets.game_mode == 0 && h.mode == 0 && (h.group < 3 || h.group == 4) && h.f0634 == 0
}

/// The hero's sounds in body 3: every `0x236738` voice is queued (Ratchet's class sound on Ratchet's moby, which the game
/// first moves to the hero 0x13f3d0), everything else goes through.
pub struct DisguiseVoices<'a> {
    pub inner: &'a mut dyn super::HeroSounds,
    pub queued: Vec<(i32, u32)>,
}

impl super::HeroSounds for DisguiseVoices<'_> {
    fn anim_advanced(&mut self, moby: &crate::moby_runtime::Moby, before: &super::AnimView, after: &super::AnimView, rng: &mut crate::rng::Rng) { self.inner.anim_advanced(moby, before, after, rng) }
    fn voice(&mut self, _moby: &crate::moby_runtime::Moby, index: i32, flags: u32, _rng: &mut crate::rng::Rng) -> i32 {
        self.queued.push((index, flags));
        -1
    }
    fn release(&mut self, moby: &crate::moby_runtime::Moby, slot: i32) { self.inner.release(moby, slot) }
    fn item_sound(&mut self, o_class: i16, pos: [f32; 3], index: i32, flags: u32, rng: &mut crate::rng::Rng) -> i32 { self.inner.item_sound(o_class, pos, index, flags, rng) }
    fn footstep(&mut self, moby: &crate::moby_runtime::Moby, level: i32, class: u8, foot: u8, variant: u8, rng: &mut crate::rng::Rng) -> i32 { self.inner.footstep(moby, level, class, foot, variant, rng) }
    fn alive(&mut self, slot: i32) -> bool { self.inner.alive(slot) }
    fn moby_sound(&mut self, id: crate::moby_runtime::MobyId, o_class: i16, pos: [f32; 3], index: i32, flags: u32, rng: &mut crate::rng::Rng) -> i32 { self.inner.moby_sound(id, o_class, pos, index, flags, rng) }
    fn set_pitch_bend(&mut self, slot: i32, pb: i32) { self.inner.set_pitch_bend(slot, pb) }
}

/// The voices of a body-3 update, played by the tick on Ratchet's moby placed at the hero (`0x236738`'s body-3 branch:
/// Ratchet's +0x10 = 0x13f3d0, then `PlayClassSound(index, flags, Ratchet)`).
pub fn queue_voices(h: &mut Hero, voices: Vec<(i32, u32)>) {
    if voices.is_empty() { return; }
    let p = [h.pos[0].to_f32(), h.pos[1].to_f32(), h.pos[2].to_f32(), h.pos[3].to_f32()];
    h.bodies.cmds.push(super::bodies::BodyCmd::RatchetPlace(p));
    h.bodies.ratchet_sounds.extend(voices);
}

/// The timer started with a voice (`0x236738(0x18 / 0x19, 0)`, 0x14162c = 1, 0x14162e = 0x12), unless it runs. The
/// voice: played at once with the transitions' player, else queued: in body 0 with the item sounds (played after the
/// slot loop), in a body with Ratchet's class sounds the tick plays after the body's update.
fn start(h: &mut Hero, voice: i32, c: Option<&mut Ctx>) {
    if h.gadgets.disguise.timer != 0 { return; }
    let played = c.is_some_and(|c| c.voice(voice, 0));
    if !played {
        if h.mode == super::bodies::body::DISGUISE {
            queue_voices(h, vec![(voice, 0)]);
        } else if h.mode != 0 {
            h.bodies.ratchet_sounds.push((voice, 0));
        } else {
            h.fx.item_voices.push(super::packs::SoundCmd::Voice { index: voice, flags: 0 });
        }
    }
    h.gadgets.disguise.squash = 1;
    h.gadgets.disguise.timer = TIMER;
}

/// The disguise's way out from its transitions (□, `0x242930` cases 0x53 / 0x54): voice 0x19, squash and timer when
/// the timer is not running.
pub fn way_out(h: &mut Hero) { start(h, VOICE_OFF, None); }

/// `HeroPdaGadget` case 0x1f.
pub fn fire(h: &mut Hero, c: &mut Ctx) {
    if !can_disguise(h) { return; }
    if c.env.pad.pressed_within(h.items.slot.fire_mask, ticks(7)).is_none() { return; }
    let class_ok = h.items.slot.item.as_ref().is_some_and(|m| m.o_class == ITEM_CLASS);
    if h.gadgets.hand_saved != HOLOGUISE || !class_ok || h.gadgets.disguise.timer != 0 { return; }
    start(h, VOICE_ON, Some(c));
}

/// `UpdateWrenchSelected(0)`'s request 0x1f: true when the swap to the Hologuise starts (the caller sets the target,
/// the previous and the saved items). The request is cleared by the caller either way.
pub fn request(h: &mut Hero) -> bool {
    if !can_disguise(h) { return false; }
    start(h, VOICE_ON, None);
    true
}

/// `HeroTickStateTimer`'s part (the move's post step): the timer counted down; its end this tick → [`Disguise::due`].
pub fn tick_timer(h: &mut Hero, square: bool) {
    let d = &mut h.gadgets.disguise;
    if d.timer == 0 { return; }
    if super::idle::dec_timer_s16(&mut d.timer) == 0 { return; }
    if h.health == 0 || h.gadgets.game_mode != 0 { return; }
    h.gadgets.disguise.due = Some(if h.mode == super::bodies::body::DISGUISE { Due::Leave { wrench: square } } else { Due::Enter });
}

/// The timer's end in body 0: the disguise moby asked for (made by the tick right after the hero update,
/// `fx::create_mobys`, which then queues the switch).
pub fn enter(h: &mut Hero) {
    if h.gadgets.disguise.due != Some(Due::Enter) { return; }
    h.gadgets.disguise.due = None;
    // The new moby's +0x90 = 0: the tint's tween starts from it.
    h.gadgets.disguise.glow = 0;
    let pos = super::physics::to_f32x3(h.pos);
    let rot = [h.rot[0].to_f32(), h.rot[1].to_f32(), h.rot[2].to_f32()];
    h.fx.mobys.push(super::fx::MobySpawn::Disguise { pos: [pos[0], pos[1], pos[2], h.pos[3].to_f32()], rot: [rot[0], rot[1], rot[2], h.rot[3].to_f32()] });
}

/// The timer's end in body 3: `0x231450` and the wrench's request flag.
pub fn leave(h: &mut Hero, c: &mut Ctx) {
    let Some(Due::Leave { wrench }) = h.gadgets.disguise.due else { return };
    h.gadgets.disguise.due = None;
    let mode = h.gadgets.game_mode;
    super::bodies::leave_body(h, c, mode);
    if wrench { h.items.f13f6 = 1; }
}

/// `0x229158`: the squash's step, its factor on the hero moby's rows +0xc0 / +0xd0 (`moby`), and the factor kept for the
/// hand item ([`squash_hand`]) and the items `rc-engine` places from the hero moby's joints.
pub fn squash(h: &mut Hero, moby: &mut crate::moby_runtime::Moby) {
    h.gadgets.disguise.factor = None;
    let d = &mut h.gadgets.disguise;
    if d.squash == 0 { return; }
    d.squash = d.squash.wrapping_add(1);
    let f = SQUASH.get(d.squash as usize).copied().unwrap_or(-1.0);
    if f == -1.0 {
        d.squash = 0;
        return;
    }
    for r in 0..2 { for k in 0..3 { moby.rows[r][k] *= f; } }
    if h.mode == 0 { h.gadgets.disguise.factor = Some(f); }
}

/// `0x229158`'s item part in body 0 for the hand item (slot 0's moby, scaled twice by the loop: its factor capped at 1;
/// rows +0xc0 / +0xd0 scaled and its position pulled toward the body point 0x13f420 in x / y). The factor stays in
/// [`Disguise::factor`] for the other item slots, which `rc-engine` places after the tick.
pub fn squash_hand(h: &mut Hero) {
    let Some(f) = h.gadgets.disguise.factor else { return };
    let k = f.min(1.0);
    let bp = super::physics::to_f32x3(h.body_point);
    let Some(it) = h.items.slot.item.as_mut() else { return };
    for _ in 0..2 {
        for r in 0..2 { for c in 0..3 { it.rows[r][c] = (f32::from_bits(it.rows[r][c]) * k).to_bits(); } }
        let d = [it.position[0] - bp[0], it.position[1] - bp[1], 0.0];
        for (p, dc) in it.position.iter_mut().zip(d) { *p = *p - dc + dc * f; }
    }
}

/// `HeroUpdateAlt`'s body-3 part after the write-back (module doc): the cheat's head scale, the pulse into the body's
/// glow, the glow points, the tint, the alert cleared, the way out.
pub fn body_update(h: &mut Hero, anim: &dyn super::AnimCtl, counter: i32) {
    if h.mode != super::bodies::body::DISGUISE { return; }
    if h.cheats.on(crate::cheats::slot::RATCHET) { h.bodies.joints[super::bodies::rec::R30].scale = f32::from_bits(0x3ff3_3333); }
    let d = h.gadgets.disguise;
    let period = ticks(match d.alert { 2 => 0x32, 1 => 0x5a, _ => 0xaa });
    let t = (counter % period.max(1)) as f32 / period as f32;
    let s = ((t + t) * std::f32::consts::PI - std::f32::consts::PI).sin();
    let (a, b, c) = ((s * 20.0) as i32, (s * 70.0) as i32, (s * 10.0) as i32);
    let red = ((a + 0x32) as u32).min(0xff);
    let green = ((b + 0xb4) as u32).min(0xff);
    let blue = ((c + 0x14) as u32) << 16;
    let target = if d.alert == 2 { blue | 0x8000_0000 | ((red as i32 / 2) as u32) << 8 | green } else { blue | 0x8000_0000 | green << 8 | red };
    let glow = crate::hud::tween_color(0.1, d.glow, target);
    h.gadgets.disguise.glow = glow;
    h.bodies.body_glow = Some(glow);
    // 0x2645a8(hero moby, k, 0x1410a0 + 16k): the body's joint lists 0..2.
    for k in 0..3 {
        let p = super::fx::joint_point(h, anim, k);
        h.gadgets.disguise.glow_points[k] = [p[0], p[1], p[2]];
    }
    let alpha = if d.timer != 0 { ((d.timer as f32 / 18.0) * 48.0) as i32 as u32 } else { 0x30 };
    h.gadgets.disguise.tint = alpha << 24 | (glow & 0x00ff_ffff);
    h.gadgets.disguise.alert = 0;
    // The way out: the hand request (0x141408) mirrored in `Gadgets::hand_request` (the tick writes it back).
    let req = h.gadgets.hand_request;
    if req == 0 || req == HOLOGUISE {
        h.gadgets.hand_request = 0;
        if h.f0637 != 0 && h.grounded_ticks != 0 && h.owned.0.get(28).is_some_and(|&b| b != 0) && h.gadgets.disguise.timer == 0 {
            start(h, VOICE_OFF, None);
        }
    } else if h.gadgets.disguise.timer == 0 {
        start(h, VOICE_OFF, None);
    }
}
