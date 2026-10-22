//! **Port-only** (the disc's game has no vibration: its "Controller vibration" page is not linked and no actuator call
//! is, docs/plan/game_state.md): controller rumble and adaptive-trigger feel from what happens in the game, sent to a
//! DualSense (crate::dualsense).
//!
//! **Rumble.** Each game tick reads the state the game already keeps; an event starts a short pulse ([`PULSE_TICKS`])
//! on the two motors:
//! * a wrench swing hitting something (`Melee::hit` rising, once per swing: the jump attack's hit too);
//! * Ratchet losing health (the strongest);
//! * a weapon firing (any ammo count going down): a kick, full strength for the R.Y.N.O.;
//! * a landing after [`LANDING_AIR`] ticks in the air, stronger the longer the fall;
//! * a camera shake starting (`follow_camera::Shake`: explosions, collapsing platforms, the jet-pack stomp), at
//!   `sqrt(amp / 0.4)` so small shakes are still felt. A long shake ([`LONG_SHAKE`] ticks or more: rumbling rocks)
//!   also rumbles along its fading envelope.
//!
//! A new event while a pulse runs cuts the motors for [`GAP_TICKS`] first, so back-to-back hits are each felt instead of
//! merging into one buzz. Pulses count game ticks, so they stop with the game: while a menu holds the tick
//! (crate::menu_render) the motors are off. A change of the vibration settings plays a short pulse at the new strength.
//!
//! **R2 as the fire button and its feel.** With [`ControllerSettings::fire_r2`] the pad's R2 is read as ○ (fire;
//! crate::input_map::Pads), leaving R1 to crouch (the game reads R1 and R2 as one crouch button). With
//! [`ControllerSettings::triggers`] too, R2 takes the held weapon's feel ([`weapon_feel`]) while the game ticks, the
//! weapon is in hand and it has ammo (or uses none, [`NO_AMMO`]); otherwise the trigger moves freely. R2 then fires
//! only once pulled to where that effect is felt ([`FirePoint`]), not at its first touch. The feel follows when the
//! game's shots come out: the streams and the R.Y.N.O. buzz while they fire and stop with them ([`Burst`]: the
//! R.Y.N.O.'s salvo is its item's state 4, seven missiles on one ammo), until R2 is let go; the Blaster kicks on each
//! shot's own tick ([`Feel::Kick`]).
//!
//! **Settings** ([`ControllerSettings`], Port Options "Vibration", "Vibration strength", "Fire button" and "Trigger
//! effects", `rc_game::menus::pause::port`): kept in the port settings file (`vibration = on|off`,
//! `vibration_strength = low|medium|high`, `fire_button = circle|r2`, `trigger_effects = on|off`); `RC_VIBRATION=0|1`
//! overrides the first at start.

use bevy::prelude::*;

use crate::dualsense::{DualSense, Trigger};
use crate::gameplay::{Persistent, Play};

/// How strongly the motors run, as the Port Options row's values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strength {
    Low,
    Medium,
    High,
}

impl Strength {
    pub const ALL: [Strength; 3] = [Strength::Low, Strength::Medium, Strength::High];
    const KEYS: [&'static str; 3] = ["low", "medium", "high"];

    pub fn index(self) -> u8 { Self::ALL.iter().position(|&s| s == self).unwrap_or(1) as u8 }
    pub fn from_index(i: u8) -> Strength { Self::ALL.get(i as usize).copied().unwrap_or(Strength::Medium) }
    fn key(self) -> &'static str { Self::KEYS[self.index() as usize] }

    /// The factor on every motor value.
    fn scale(self) -> f32 {
        match self {
            Strength::Low => 0.4,
            Strength::Medium => 0.7,
            Strength::High => 1.0,
        }
    }
}

/// The controller's Port Options rows (module docs).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControllerSettings {
    /// Rumble on.
    pub enabled: bool,
    pub strength: Strength,
    /// R2 fires (read as ○).
    pub fire_r2: bool,
    /// R2 takes the held weapon's effect (with `fire_r2`).
    pub triggers: bool,
    /// The button icons in text (crate::button_icons).
    pub icons: crate::button_icons::IconStyle,
}

impl ControllerSettings {
    /// The port settings file, `RC_VIBRATION=0|1` over its vibration; by default rumble on at medium strength, ○ fires,
    /// trigger effects on (felt once R2 fires), the disc's button icons.
    pub fn startup() -> Self {
        let key = |k: &str| crate::render_settings::load_key(k).map(|v| v.trim().to_string());
        let strength = key("vibration_strength").and_then(|v| Strength::KEYS.iter().position(|k| *k == v)).map_or(Strength::Medium, |i| Strength::ALL[i]);
        let enabled = match std::env::var("RC_VIBRATION") {
            Ok(v) => v.trim() != "0",
            Err(_) => key("vibration").is_none_or(|v| v != "off"),
        };
        let fire_r2 = key("fire_button").is_some_and(|v| v == "r2");
        let triggers = key("trigger_effects").is_none_or(|v| v != "off");
        let icons = key("button_icons").and_then(|v| crate::button_icons::IconStyle::from_key(&v)).unwrap_or_default();
        ControllerSettings { enabled, strength, fire_r2, triggers, icons }
    }

    /// Writes the settings into the port settings file.
    pub fn save(&self) {
        let on = |b: bool| if b { "on" } else { "off" };
        crate::render_settings::save_key("vibration", on(self.enabled));
        crate::render_settings::save_key("vibration_strength", self.strength.key());
        crate::render_settings::save_key("fire_button", if self.fire_r2 { "r2" } else { "circle" });
        crate::render_settings::save_key("trigger_effects", on(self.triggers));
        crate::render_settings::save_key("button_icons", self.icons.key());
    }
}

/// Ticks in the air before a landing is felt (a third of a second: a jump, not a step down).
const LANDING_AIR: i32 = 20;
/// A pulse's length in game ticks (100 ms).
const PULSE_TICKS: i32 = 6;
/// Ticks the motors stop before a pulse that interrupts another, so each one is felt.
const GAP_TICKS: i32 = 2;
/// A camera shake this long or longer (ticks) also rumbles while it lasts.
const LONG_SHAKE: i32 = 60;
/// Frames of the pulse a settings change plays.
const PREVIEW_FRAMES: u32 = 18;

/// What one game tick saw.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Sample {
    health: i32,
    ammo: i64,
    air: i32,
    hit: bool,
    /// Each shake record's timer.
    shake_timers: [i32; 2],
    /// The hand item (8: the wrench).
    item: i32,
}

/// The R.Y.N.O.'s item id: its shots kick hardest.
const RYNO: i32 = 23;

/// One tick's events (`a` the previous tick, `b` this one): the pulse they start (heavy, light), each in 0..=1, or none.
/// `shake_amps`: this tick's shake amplitudes.
fn tick_pulse(a: &Sample, b: &Sample, shake_amps: [f32; 2]) -> Option<(f32, f32)> {
    let mut p: Option<(f32, f32)> = None;
    let mut add = |heavy: f32, light: f32| {
        let (h, l) = p.unwrap_or((0.0, 0.0));
        p = Some((h.max(heavy), l.max(light)));
    };
    if b.health < a.health { add(1.0, 0.7); }
    if b.hit && !a.hit { add(0.55, 0.85); }
    if b.ammo < a.ammo {
        if b.item == RYNO { add(1.0, 1.0) } else { add(0.25, 0.6) }
    }
    if a.air >= LANDING_AIR && b.air == 0 { add((a.air as f32 / 60.0).clamp(0.3, 0.7), 0.0); }
    for ((after, before), amp) in b.shake_timers.iter().zip(a.shake_timers).zip(shake_amps) {
        // A request sets the timer anew; otherwise it only counts down.
        if *after > before {
            let s = shake_strength(amp);
            add(s, s * 0.4);
        }
    }
    p
}

/// A shake amplitude's strength (module docs).
fn shake_strength(amp: f32) -> f32 { (amp.abs() / 0.4).sqrt().min(1.0) }

/// A long shake's continuous rumble on the heavy motor: its envelope `(timer / max)²` while it runs.
fn long_shake_level(s: &rc_game::follow_camera::Shake) -> f32 {
    if s.timer <= 0 || s.max < LONG_SHAKE { return 0.0; }
    let f = s.timer as f32 / s.max as f32;
    0.6 * shake_strength(s.amp.to_f32()) * f * f
}

/// Ticks without spending ammo, with R2 held past the fire point, that end a stream's burst (the Pyrocitor's and Tesla
/// Claw's held fire spends it much more often).
const BURST_GAP: i32 = 20;
/// R2 travel under which the trigger counts as let go (a new pull may buzz again).
const RELEASED: f32 = 0.1;

/// A [`Feel::Burst`] weapon's buzz through one pull: on while R2 is pulled toward the shot and while the weapon fires,
/// off once it has not fired for `gap` ticks (the R.Y.N.O.'s salvo is over) until R2 is let go.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Burst {
    quiet: i32,
    done: bool,
}

impl Burst {
    /// `ticks` game ticks with R2 at `r2`, the weapon firing at `point`, `firing`: whether it fired in them. True while
    /// the buzz runs.
    fn step(&mut self, r2: f32, point: f32, firing: bool, ticks: i32, gap: i32) -> bool {
        if r2 < point {
            self.quiet = 0;
            if r2 < RELEASED { self.done = false; }
        } else if firing {
            self.quiet = 0;
        } else {
            self.quiet += ticks;
            if self.quiet > gap { self.done = true; }
        }
        !self.done
    }
}

/// The pulse running: its strengths, the ticks left, and the gap ticks before it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Pulse {
    heavy: f32,
    light: f32,
    left: i32,
    gap: i32,
}

impl Pulse {
    /// A new pulse; one already running is cut by a gap first.
    fn start(&mut self, (heavy, light): (f32, f32)) {
        self.gap = if self.left > 0 { GAP_TICKS } else { 0 };
        (self.heavy, self.light, self.left) = (heavy, light, PULSE_TICKS);
    }

    /// One game tick passes.
    fn step(&mut self) {
        if self.gap > 0 { self.gap -= 1 } else if self.left > 0 { self.left -= 1 }
    }

    /// The motors now.
    fn level(&self) -> (f32, f32) { if self.gap == 0 && self.left > 0 { (self.heavy, self.light) } else { (0.0, 0.0) } }
}

/// What the last frame saw, and the pulse running.
#[derive(Default)]
struct Haptics {
    counter: Option<u64>,
    last: Sample,
    pulse: Pulse,
    preview: u32,
    settings: Option<ControllerSettings>,
    burst: Burst,
    /// A [`Feel::Burst`] weapon's buzz is off: its burst is over ([`Burst::step`] of the last ticks).
    buzz_off: bool,
    /// Ticks left of a [`Feel::Kick`] weapon's kick.
    kick: i32,
}

fn motor(v: f32, scale: f32) -> u8 { (v.clamp(0.0, 1.0) * scale * 255.0).round() as u8 }

/// How far R2 is pulled (0..1) before it fires (crate::input_map::PadDevice::fire_on_r2): where the held weapon's
/// effect is felt, so the shot follows it. 0 without an effect (the least travel fires).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct FirePoint(pub f32);

/// The fire point of an effect: past a weapon effect's snap, a little into a resistance, well after a vibration starts
/// ([`VIBRATION_LEAD`]: the trigger's motor needs a moment to build up, and a shot right at its start came first).
pub fn fire_point(t: Trigger) -> f32 {
    // The weapon effect: just past the zone where it snaps (zone k of ten at about k / 9 of the travel).
    if let Some(end) = t.weapon_end() { return ((end as f32 + 0.6) / 9.0).min(0.92); }
    let [mode, a, _, c, ..] = t.0;
    let at = match mode {
        0x01 => a as f32 + 40.0,
        0x06 => c as f32 + VIBRATION_LEAD,
        _ => return 0.0,
    };
    (at / 255.0).min(0.92)
}

/// Trigger travel (of 255) a vibrating weapon is pulled past the buzz's start before it fires: about a third of the pull,
/// so the buzz is felt first.
const VIBRATION_LEAD: f32 = 90.0;

/// The weapons without ammo (their item record's max ammo is 0): Suck Cannon, Taunter, Walloper, Morph-o-Ray. Their
/// trigger effect does not wait for ammo.
const NO_AMMO: [i32; 4] = [9, 14, 18, 21];

/// The Blaster's item id: its shots kick R2 one by one.
const BLASTER: i32 = 15;
/// Ticks a shot's kick lasts on R2 (the Blaster fires every 6: `rc_game::hero::blaster`).
const KICK_TICKS: i32 = 3;
/// Ticks without firing that end the R.Y.N.O.'s burst: its salvo state is the firing itself, so only a tick's slack.
const SALVO_GAP: i32 = 3;

/// How R2 feels with a weapon, synced to when its shots come out (module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Feel {
    /// One effect throughout: resistance or a weapon effect's snap.
    Fixed(Trigger),
    /// A buzz while R2 is pulled toward the shot and while the weapon fires, stopping `gap` ticks after it last fired
    /// until R2 is let go ([`Burst`]).
    Burst { buzz: Trigger, gap: i32 },
    /// `idle` at rest, `kick` for [`KICK_TICKS`] on the tick each shot comes out.
    Kick { idle: Trigger, kick: Trigger },
}

impl Feel {
    /// The effect the fire point is taken from.
    fn base(self) -> Trigger {
        match self {
            Feel::Fixed(t) | Feel::Burst { buzz: t, .. } | Feel::Kick { idle: t, .. } => t,
        }
    }
}

/// R2's feel for a weapon (item id; docs/plan/game_state.md §1), `None` for anything else.
pub fn weapon_feel(item: i32) -> Option<Feel> {
    Some(match item {
        // Suck Cannon: the strain of the vacuum.
        9 => Feel::Fixed(Trigger::resistance(40, 180)),
        // Bomb Glove, Visibomb, Mine Glove, Glove of Doom, Decoy Glove: a lob's click.
        10 => Feel::Fixed(Trigger::weapon(2, 4, 5)),
        13 => Feel::Fixed(Trigger::weapon(2, 5, 5)),
        17 | 25 => Feel::Fixed(Trigger::weapon(2, 4, 4)),
        20 => Feel::Fixed(Trigger::weapon(2, 4, 5)),
        // Devastator, Walloper: a sniper's heavy click, a stiff wall deep in the pull that snaps.
        11 | 18 => Feel::Fixed(Trigger::weapon(3, 6, 8)),
        // Taunter, Drone Device: light resistance.
        14 => Feel::Fixed(Trigger::resistance(0, 60)),
        24 => Feel::Fixed(Trigger::resistance(0, 80)),
        // Blaster: light resistance, a sharp kick with each shot (its first comes `ticks(7)` after the draw, then one
        // every `ticks(6)`: a free-running buzz ran ahead of them).
        BLASTER => Feel::Kick { idle: Trigger::resistance(0, 50), kick: Trigger::vibration(40, 230, 0) },
        // The streams buzz from early in the pull, fire about halfway ([`VIBRATION_LEAD`]) and buzz while they spend
        // ammo. Pyrocitor: the flame's hum; Tesla Claw: a fast buzz; Morph-o-Ray (no ammo: while held): a warble.
        16 => Feel::Burst { buzz: Trigger::vibration(30, 60, 20), gap: BURST_GAP },
        19 => Feel::Burst { buzz: Trigger::vibration(40, 90, 20), gap: BURST_GAP },
        21 => Feel::Burst { buzz: Trigger::vibration(20, 50, 30), gap: BURST_GAP },
        // R.Y.N.O.: the heaviest shake, from the start of the pull through its salvo (seven missiles `ticks(9)` apart
        // on one ammo: the item's state 4, `rc_game::hero::ryno`).
        RYNO => Feel::Burst { buzz: Trigger::vibration(10, 255, 20), gap: SALVO_GAP },
        _ => return None,
    })
}

/// Each frame: the new ticks' events, then the motors (off while no tick runs, unless a settings preview plays) and
/// R2's effect.
fn haptics_frame(
    mut h: Local<Haptics>,
    settings: Res<ControllerSettings>,
    pad: Option<Res<DualSense>>,
    play: Option<Res<Play>>,
    gs: Option<Res<Persistent>>,
    mode: Option<Res<crate::menu_render::MenuMode>>,
    mut fire: ResMut<FirePoint>,
) {
    let Some(pad) = pad else { return };
    let vib = |s: &ControllerSettings| (s.enabled, s.strength);
    if h.settings.is_some_and(|s| vib(&s) != vib(&settings)) && settings.enabled { h.preview = PREVIEW_FRAMES; }
    h.settings = Some(*settings);
    let ticking = mode.as_ref().is_none_or(|m| m.state.mode.advances_tick());
    let mut long_shake = 0.0f32;
    let mut trigger = Trigger::OFF;
    // The held weapon's fire point (kept while its buzz is off or a kick plays).
    let mut point = 0.0f32;
    if let (Some(play), true) = (play.as_ref(), ticking) {
        let game = &play.game;
        let global = gs.as_ref().map(|g| &g.0.global);
        // The hand item: the wrench while it shows, else the equipped one.
        let item = global.map_or(8, |g| if g.wrench_held != 0 { 8 } else { g.equipped[0] });
        let now = Sample {
            health: game.hero.health,
            ammo: global.map_or(0, |g| g.ammo.iter().map(|&a| a.max(0) as i64).sum()),
            air: game.hero.air_ticks as i32,
            hit: game.hero.melee.hit != 0,
            shake_timers: [game.camera.shake[0].timer, game.camera.shake[1].timer],
            item,
        };
        // The ticks this frame ran, and whether a shot came in them.
        let (mut ticks, mut shot) = (0, false);
        match h.counter {
            Some(c) if game.counter > c => {
                let amps = [game.camera.shake[0].amp.to_f32(), game.camera.shake[1].amp.to_f32()];
                let last = h.last;
                ticks = (game.counter - c).min(60) as i32;
                shot = now.ammo < last.ammo;
                for _ in 0..ticks { h.pulse.step(); }
                if let Some(p) = tick_pulse(&last, &now, amps) { h.pulse.start(p); }
            }
            // The first tick seen, or the level was reloaded (counter back to 0): nothing to compare with yet.
            Some(c) if game.counter == c => {}
            _ => h.pulse = Pulse::default(),
        }
        (h.counter, h.last) = (Some(game.counter), now);
        long_shake = game.camera.shake.iter().map(long_shake_level).fold(0.0, f32::max);
        // R2's feel: the weapon in hand (not the wrench), while it has ammo (or uses none).
        let feel = global.filter(|_| settings.fire_r2 && settings.triggers).and_then(|g| {
            let ammo = usize::try_from(item).ok().and_then(|i| g.ammo.get(i)).copied().unwrap_or(0);
            (ammo > 0 || NO_AMMO.contains(&item)).then(|| weapon_feel(item)).flatten()
        });
        let r2 = pad.get().map_or(0.0, |p| p.r2);
        trigger = feel.map_or(Trigger::OFF, Feel::base);
        point = fire_point(trigger);
        match feel {
            Some(Feel::Burst { buzz, gap }) => {
                if ticks > 0 {
                    // Firing: the R.Y.N.O.'s salvo running (its item in state 4), a stream spending ammo, the
                    // Morph-o-Ray (no ammo) held past its point.
                    let firing = if item == RYNO {
                        game.hero.items.slot.id == RYNO && game.hero.items.slot.item.as_ref().is_some_and(|m| m.mstate == 4)
                    } else {
                        shot || (NO_AMMO.contains(&item) && r2 >= point)
                    };
                    h.buzz_off = !h.burst.step(r2, point, firing, ticks, gap);
                }
                if h.buzz_off { trigger = Trigger::OFF } else { trigger = buzz }
                h.kick = 0;
            }
            Some(Feel::Kick { kick, .. }) => {
                // The kick starts on the tick the shot's ammo is spent: the shot's own spawn tick.
                h.kick = if shot { KICK_TICKS } else { (h.kick - ticks).max(0) };
                if h.kick > 0 { trigger = kick; }
                (h.burst, h.buzz_off) = (Burst::default(), false);
            }
            _ => (h.burst, h.buzz_off, h.kick) = (Burst::default(), false, 0),
        }
    }
    let scale = settings.strength.scale();
    let (heavy, light) = if h.preview > 0 {
        h.preview -= 1;
        let f = h.preview as f32 / PREVIEW_FRAMES as f32;
        (0.9 * f, 0.6 * f)
    } else if !settings.enabled || !ticking {
        (0.0, 0.0)
    } else {
        let (ph, pl) = h.pulse.level();
        (ph.max(long_shake), pl.max(long_shake * 0.5))
    };
    pad.set_rumble(motor(heavy, scale), motor(light, scale));
    pad.set_trigger(trigger);
    if fire.0 != point { fire.0 = point; }
}

pub struct HapticsPlugin;

impl Plugin for HapticsPlugin {
    fn build(&self, app: &mut App) {
        let s = ControllerSettings::startup();
        println!(
            "haptics: vibration {}, strength {:?}, fire button {}, trigger effects {} (Port Options; RC_VIBRATION)",
            if s.enabled { "on" } else { "off" },
            s.strength,
            if s.fire_r2 { "R2" } else { "circle" },
            if s.triggers { "on" } else { "off" }
        );
        app.insert_resource(s).init_resource::<FirePoint>().add_systems(PostUpdate, haptics_frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(health: i32, ammo: i64, air: i32, hit: bool) -> Sample { Sample { health, ammo, air, hit, shake_timers: [0, 0], item: 10 } }

    #[test]
    fn events_start_pulses() {
        let base = sample(8, 50, 0, false);
        assert_eq!(tick_pulse(&base, &base, [0.0; 2]), None);
        assert_eq!(tick_pulse(&base, &sample(7, 50, 0, false), [0.0; 2]), Some((1.0, 0.7)));
        assert_eq!(tick_pulse(&base, &sample(8, 50, 0, true), [0.0; 2]), Some((0.55, 0.85)));
        // A hit held over several ticks is one event.
        assert_eq!(tick_pulse(&sample(8, 50, 0, true), &sample(8, 50, 0, true), [0.0; 2]), None);
        assert_eq!(tick_pulse(&base, &sample(8, 49, 0, false), [0.0; 2]), Some((0.25, 0.6)));
        // The R.Y.N.O.'s shot: full strength.
        assert_eq!(tick_pulse(&base, &Sample { ammo: 49, item: RYNO, ..base }, [0.0; 2]), Some((1.0, 1.0)));
        assert_eq!(tick_pulse(&sample(8, 50, LANDING_AIR, false), &base, [0.0; 2]), Some((0.33333334, 0.0)));
        assert_eq!(tick_pulse(&sample(8, 50, LANDING_AIR - 1, false), &base, [0.0; 2]), None);
        // Health or ammo going up (a pickup, a refill) is not felt.
        assert_eq!(tick_pulse(&sample(7, 40, 0, false), &base, [0.0; 2]), None);
        // A shake request: its timer set anew.
        let shaking = Sample { shake_timers: [40, 0], ..base };
        assert_eq!(tick_pulse(&base, &shaking, [0.4, 0.0]), Some((1.0, 0.4)));
        assert_eq!(tick_pulse(&shaking, &Sample { shake_timers: [39, 0], ..base }, [0.4, 0.0]), None);
    }

    #[test]
    fn buzz_ends_with_the_burst() {
        let gap = BURST_GAP;
        let mut b = Burst::default();
        // Pulling toward the shot: it buzzes.
        assert!(b.step(0.3, 0.43, false, 1, gap));
        // Held past the fire point, shots coming: still buzzing, gaps shorter than the burst's included.
        assert!(b.step(0.9, 0.43, true, 1, gap));
        assert!(b.step(0.9, 0.43, false, gap - 5, gap));
        assert!(b.step(0.9, 0.43, true, 1, gap));
        // The burst is over: quiet while R2 stays held, even eased back a little.
        assert!(!b.step(0.9, 0.43, false, gap + 1, gap));
        assert!(!b.step(0.9, 0.43, false, 30, gap));
        assert!(!b.step(0.3, 0.43, false, 1, gap));
        // Let go, then a new pull buzzes again.
        assert!(b.step(0.0, 0.43, false, 1, gap));
        assert!(b.step(0.3, 0.43, false, 1, gap));
    }

    #[test]
    fn ryno_buzzes_through_its_whole_salvo() {
        // Seven missiles `ticks(9)` apart: the salvo state holds about 63 ticks, the buzz with it, then stops.
        let mut b = Burst::default();
        for _ in 0..63 { assert!(b.step(1.0, 0.43, true, 1, SALVO_GAP)); }
        for _ in 0..SALVO_GAP { assert!(b.step(1.0, 0.43, false, 1, SALVO_GAP)); }
        assert!(!b.step(1.0, 0.43, false, 1, SALVO_GAP));
    }

    #[test]
    fn back_to_back_pulses_are_each_felt() {
        let mut p = Pulse::default();
        p.start((1.0, 0.5));
        assert_eq!(p.level(), (1.0, 0.5));
        for _ in 0..3 { p.step(); }
        // A second event mid-pulse: off for the gap, then a full pulse.
        p.start((0.6, 0.6));
        assert_eq!(p.level(), (0.0, 0.0));
        for _ in 0..GAP_TICKS { p.step(); }
        assert_eq!(p.level(), (0.6, 0.6));
        for _ in 0..PULSE_TICKS { p.step(); }
        assert_eq!(p.level(), (0.0, 0.0));
    }

    #[test]
    fn strengths_and_triggers() {
        assert_eq!(motor(1.0, Strength::High.scale()), 255);
        assert_eq!(motor(1.0, Strength::Low.scale()), 102);
        assert_eq!(motor(2.0, 1.0), 255);
        for s in Strength::ALL { assert_eq!(Strength::from_index(s.index()), s); }
        assert_eq!(weapon_feel(8), None);
        // The Blaster: light resistance, a kick per shot.
        assert!(matches!(weapon_feel(BLASTER), Some(Feel::Kick { .. })));
        // The R.Y.N.O. buzzes from 20 and fires well after, at 110 of 255, through its salvo.
        let ryno = weapon_feel(RYNO).unwrap();
        assert_eq!(ryno, Feel::Burst { buzz: Trigger::vibration(10, 255, 20), gap: SALVO_GAP });
        assert_eq!(fire_point(ryno.base()), 110.0 / 255.0);
        for id in [9, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25] {
            let feel = weapon_feel(id).unwrap_or_else(|| panic!("item {id}"));
            // Every weapon fires somewhere in the pull, after its effect starts.
            let p = fire_point(feel.base());
            assert!(p > 0.1 && p <= 0.92, "item {id}: {p}");
        }
        // The Walloper fires past its snap (zone 6 of ten), not before.
        let walloper = weapon_feel(18).unwrap().base();
        assert_eq!(walloper.weapon_end(), Some(6));
        assert_eq!(fire_point(walloper), 6.6 / 9.0);
        // The weapon effect's bytes: zones 3 and 6 (0x48), strength 8 stored as 7.
        assert_eq!(&weapon_feel(11).unwrap().base().0[..4], &[0x25, 0x48, 0x00, 7]);
        assert_eq!(fire_point(Trigger::OFF), 0.0);
    }
}
