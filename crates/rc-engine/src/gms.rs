//! **Port-only**: the Gadgetron Movement System (GMS), the game's lore name for its controllers: Port Options →
//! Controller opens a page listing every input device (`rc_game::menus::pause::port`, the GMS page) and lets the player
//! lock the game to one of them.
//!
//! **Devices** ([`Gms::devices`], rebuilt every frame): the keyboard and mouse (always), a DualSense read from its HID
//! reports (crate::dualsense) and each of Bevy's gamepads under its own name (an Xbox pad, a PDP pad, a DualShock 4; a
//! DualSense's Bevy gamepad is left out while the pad is read directly, as crate::input_map::Pads does). The device that
//! last gave input (a key or button pressed, a stick pushed past half way, R2 pulled) is the **Active** one.
//!
//! **Lock** ([`Gms::lock`]): ✕ on a row reads only that device (Locked, the others Off); ✕ on it again goes back to all
//! of them. While the locked device is unplugged the lock does not apply, so the game never ends up without input. The
//! lock lasts for the run (devices are not identified across runs).
//!
//! **The HelpDesk note** ([`HELP_TEXT`]): on Veldin (level 0) the help box shows once what the Gadgetron Movement
//! System is and where its options are, as a text-only message (no voice: the game's lines are recordings) added to the
//! level text under [`HELP_MESSAGE`] and requested without a help record (`rc_game::help::Help::request_port`). It
//! counts as shown once the box displays it; the port settings file keeps `gms_help = shown`.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use rc_game::menus::pause::port::{gms_status, GmsRow};

use crate::dualsense::DualSense;
use crate::gameplay::Play;

/// The note's id in the level text (positive, as the help box takes only those; past the disc's ids 0..=21500).
pub const HELP_MESSAGE: i32 = 0x7f00_0001;
/// The note (roxas997's words, from the ReRAC Discord).
pub const HELP_TEXT: &[u8] = b"The Gadgetron Movement System is what is used to move Ratchet and Clank on their intergalactic adventure. \
If you like, you may change your options on the Port Options screen.";
/// Game ticks into Veldin before the note may show (past the arrival's own help).
const HELP_AFTER_TICKS: u64 = 600;
/// `RC_GMS_TRACE=1`: the note's conditions every 120 ticks.
static TRACE: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var("RC_GMS_TRACE").is_ok_and(|v| v.trim() == "1"));
/// The longest device name shown (the row's status needs the rest of the width).
const NAME_MAX: usize = 24;

/// One input device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeviceKey {
    Keyboard,
    /// The DualSense read from its HID reports.
    DualSense,
    /// One of Bevy's gamepads.
    Gamepad(Entity),
}

/// The devices, the one last used, the lock and the note's state.
#[derive(Resource, Debug, Default)]
pub struct Gms {
    pub devices: Vec<(DeviceKey, Vec<u8>)>,
    pub active: Option<DeviceKey>,
    pub lock: Option<DeviceKey>,
    pub help_shown: bool,
}

impl Gms {
    /// The lock while its device is connected.
    fn effective_lock(&self) -> Option<DeviceKey> { self.lock.filter(|l| self.devices.iter().any(|(k, _)| k == l)) }

    /// Whether input from `k` reaches the game.
    pub fn allows(&self, k: DeviceKey) -> bool { self.effective_lock().is_none_or(|l| l == k) }

    /// The GMS page's rows: each device and its status.
    pub fn rows(&self) -> Vec<GmsRow> {
        let lock = self.effective_lock();
        self.devices
            .iter()
            .map(|(k, name)| GmsRow {
                name: name.clone(),
                status: match lock {
                    Some(l) if l == *k => gms_status::LOCKED,
                    Some(_) => gms_status::OFF,
                    None if self.active == Some(*k) => gms_status::ACTIVE,
                    None => gms_status::AVAILABLE,
                },
            })
            .collect()
    }

    /// ✕ on row `i`: lock that device, or free it when it is the one locked.
    pub fn pick(&mut self, i: usize) {
        let Some((k, _)) = self.devices.get(i) else { return };
        self.lock = if self.effective_lock() == Some(*k) { None } else { Some(*k) };
    }
}

/// A name as the row shows it: ASCII, cut at a word to [`NAME_MAX`] with "...".
fn row_name(name: &str) -> Vec<u8> {
    let ascii: String = name.chars().map(|c| if c.is_ascii() && !c.is_ascii_control() { c } else { ' ' }).collect();
    let ascii = ascii.trim();
    if ascii.len() <= NAME_MAX { return ascii.as_bytes().to_vec(); }
    let cut = &ascii[..NAME_MAX - 3];
    let cut = if ascii.as_bytes()[NAME_MAX - 3] == b' ' { cut } else { cut.rsplit_once(' ').map_or(cut, |(a, _)| a) };
    format!("{}...", cut.trim_end()).into_bytes()
}

/// Each frame, before the pads are sampled: the devices, and the one that gave input.
pub fn track(
    mut gms: ResMut<Gms>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    gamepads: Query<(Entity, &Gamepad, Option<&Name>)>,
    dualsense: Option<Res<DualSense>>,
    mut ds_held: Local<u32>,
) {
    let ds = dualsense.as_ref().and_then(|d| d.get());
    let mut devices = vec![(DeviceKey::Keyboard, b"Keyboard and mouse".to_vec())];
    let mut used = (keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some()).then_some(DeviceKey::Keyboard);
    if let Some(p) = ds {
        devices.push((DeviceKey::DualSense, b"DualSense".to_vec()));
        let pressed = p.buttons & !*ds_held != 0;
        if pressed || p.left.length() > 0.5 || p.right.length() > 0.5 || p.r2 > 0.3 { used = Some(DeviceKey::DualSense); }
        *ds_held = p.buttons;
    }
    for (e, g, name) in &gamepads {
        if ds.is_some() && crate::dualsense::is_dualsense(g) { continue; }
        let n = devices.len();
        devices.push((DeviceKey::Gamepad(e), row_name(name.map_or(&format!("Controller {n}"), |n| n.as_str()))));
        if g.get_just_pressed().next().is_some() || g.left_stick().length() > 0.5 || g.right_stick().length() > 0.5 {
            used = Some(DeviceKey::Gamepad(e));
        }
    }
    if let Some(k) = used { gms.active = Some(k); }
    if gms.active.is_some_and(|a| !devices.iter().any(|(k, _)| *k == a)) { gms.active = None; }
    gms.devices = devices;
}

/// The keyboard's input as the game reads it: none while another device is locked.
pub fn keyboard<'a>(
    gms: Option<&Gms>,
    keys: &'a ButtonInput<KeyCode>,
    mouse: &'a ButtonInput<MouseButton>,
    motion: &'a AccumulatedMouseMotion,
) -> (&'a ButtonInput<KeyCode>, &'a ButtonInput<MouseButton>, &'a AccumulatedMouseMotion) {
    static NO_KEYS: std::sync::LazyLock<ButtonInput<KeyCode>> = std::sync::LazyLock::new(ButtonInput::default);
    static NO_MOUSE: std::sync::LazyLock<ButtonInput<MouseButton>> = std::sync::LazyLock::new(ButtonInput::default);
    static NO_MOTION: std::sync::LazyLock<AccumulatedMouseMotion> = std::sync::LazyLock::new(AccumulatedMouseMotion::default);
    if gms.is_none_or(|g| g.allows(DeviceKey::Keyboard)) { (keys, mouse, motion) } else { (&NO_KEYS, &NO_MOUSE, &NO_MOTION) }
}

/// The note in the level text (crate::gameplay's help setup).
pub fn add_help_message(messages: &mut Vec<rc_formats::strings::Message>) {
    if messages.iter().any(|m| m.id == HELP_MESSAGE) { return; }
    messages.push(rc_formats::strings::Message { id: HELP_MESSAGE, text: HELP_TEXT.to_vec(), help_audio: -1 });
}

/// On Veldin, once: request the note while the help box is free and enabled, and mark it shown once the box shows it.
fn help_note(mut gms: ResMut<Gms>, play: Option<ResMut<Play>>, mode: Option<Res<crate::menu_render::MenuMode>>) {
    if gms.help_shown { return; }
    let Some(mut play) = play else { return };
    let help = &mut play.svc.help;
    let shown = help.bx.state != 0 && help.bx.index.and_then(|i| help.text.messages.get(i)).is_some_and(|m| m.id == HELP_MESSAGE);
    if shown {
        gms.help_shown = true;
        crate::render_settings::save_key("gms_help", "shown");
        println!("gms: the HelpDesk note shown");
        return;
    }
    let ticking = mode.as_ref().is_none_or(|m| m.state.mode.advances_tick());
    if *TRACE && play.game.counter % 120 == 0 {
        let h = &play.svc.help;
        println!(
            "gms: tick {}: level {}, ticking {ticking}, held {:#x}, help enabled {}, idle {}, box state {}, note in text {}",
            play.game.counter, play.svc.level, play.game.pad.held, h.bx.enabled, h.idle(), h.bx.state, h.text.messages.iter().any(|m| m.id == HELP_MESSAGE)
        );
    }
    if play.svc.level != 0 || !ticking || play.game.counter < HELP_AFTER_TICKS { return; }
    let help = &mut play.svc.help;
    if help.bx.enabled && help.idle() && help.text.messages.iter().any(|m| m.id == HELP_MESSAGE) { help.request_port(HELP_MESSAGE); }
}

pub struct GmsPlugin;

impl Plugin for GmsPlugin {
    fn build(&self, app: &mut App) {
        let help_shown = crate::render_settings::load_key("gms_help").is_some_and(|v| v.trim() == "shown") || crate::determinism::deterministic();
        app.insert_resource(Gms { help_shown, ..default() })
            .add_systems(PreUpdate, track.after(bevy::input::InputSystems).before(crate::input_map::sample_system))
            .add_systems(Update, help_note);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gms() -> Gms {
        let pad = DeviceKey::Gamepad(Entity::PLACEHOLDER);
        Gms { devices: vec![(DeviceKey::Keyboard, b"Keyboard and mouse".to_vec()), (DeviceKey::DualSense, b"DualSense".to_vec()), (pad, b"Xbox Controller".to_vec())], active: Some(DeviceKey::DualSense), ..default() }
    }

    #[test]
    fn rows_lock_and_free() {
        let mut g = gms();
        let st = |g: &Gms| g.rows().iter().map(|r| r.status).collect::<Vec<_>>();
        assert_eq!(st(&g), vec![gms_status::AVAILABLE, gms_status::ACTIVE, gms_status::AVAILABLE]);
        assert!(g.allows(DeviceKey::Keyboard) && g.allows(DeviceKey::DualSense));
        // ✕ on the Xbox pad: only it is read.
        g.pick(2);
        assert_eq!(st(&g), vec![gms_status::OFF, gms_status::OFF, gms_status::LOCKED]);
        assert!(!g.allows(DeviceKey::Keyboard) && !g.allows(DeviceKey::DualSense));
        // Unplugged: the lock does not apply.
        let saved = g.devices.pop();
        assert!(g.allows(DeviceKey::Keyboard));
        g.devices.extend(saved);
        // ✕ on it again: all of them.
        g.pick(2);
        assert_eq!(g.lock, None);
        assert!(g.allows(DeviceKey::Keyboard));
    }

    #[test]
    fn names_fit_the_row() {
        assert_eq!(row_name("Xbox Controller"), b"Xbox Controller");
        assert_eq!(row_name("PDP Wired Controller for Xbox Series X"), b"PDP Wired Controller...");
        assert_eq!(row_name("Wireless Gamepad Controller XYZ"), b"Wireless Gamepad...");
        assert!(row_name("PDP Wired Controller for Xbox Series X").len() <= NAME_MAX);
    }

    #[test]
    fn the_note_is_added_once() {
        let mut m = Vec::new();
        add_help_message(&mut m);
        add_help_message(&mut m);
        assert_eq!(m.len(), 1);
        assert_eq!((m[0].id, m[0].help_audio), (HELP_MESSAGE, -1));
    }
}
