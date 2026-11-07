//! DualSense (PS5) and DualSense Edge pads read straight from their HID reports, over USB or Bluetooth.
//!
//! Why not through Bevy's gamepads (gilrs, Windows.Gaming.Input on Windows): over Bluetooth a DualSense that Steam,
//! DualSenseX, DS4Windows or an SDL game has talked to switches to its extended input report (0x31), which is vendor
//! defined in the pad's HID descriptor; Windows.Gaming.Input then reads nothing at all from it until the pad is power
//! cycled. A virtual pad from those tools showed up as the second gamepad, after the silent DualSense. Reading the
//! reports here works in every mode: USB report 0x01, Bluetooth simple report 0x01 and Bluetooth extended report 0x31.
//!
//! **Rumble and the R2 trigger's effect** (crate::haptics sets them, [`DualSense::set_rumble`],
//! [`DualSense::set_trigger`]): the reader thread sends the pad's output report (USB 0x02, Bluetooth 0x31 with its
//! CRC-32) whenever they change, through a second handle so a blocked read never holds a write up. The report sets the
//! two "compatible vibration" motors (valid flags 0x01 | 0x02) and the right trigger's effect (0x04, [`Trigger`]); the
//! lightbar, LEDs and the left trigger are left alone. Dropping [`RumbleStop`] (the app's end) stops the motors and
//! frees the trigger.
//!
//! A thread per process scans for a pad every second, then reads its reports into [`DualSense`] until it disconnects.
//! While a pad is read here, Bevy's gamepad for the same pad is skipped (crate::input_map::Pads), so it is not read
//! twice. Off in frame-exact mode (crate::determinism), like every live device. `RC_DUALSENSE=0` turns it off;
//! `RC_DUALSENSE_TRACE=1` prints each report's decoded state.
//!
//! Report layouts (after the report id; hid-playstation.c, SDL_hidapi_ps5.c): sticks LX LY RX RY (0 = up/left,
//! 128 = centre); in the full report (USB 0x01, Bluetooth 0x31 after one more byte) the triggers, a counter, then the
//! buttons at +7; in the Bluetooth simple report the buttons at +4, then the triggers. Buttons: byte 0 = d-pad hat in
//! the low nibble (0 = up, clockwise, 8 = released), 0x10 square, 0x20 cross, 0x40 circle, 0x80 triangle; byte 1 = L1,
//! R1, L2, R2, Create, Options, L3, R3 from bit 0; byte 2 = PS (0x01), touchpad click (0x02).

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use rc_game::pad::button;

use crate::input_map::PadDevice;

/// Sony's USB vendor id and the DualSense / DualSense Edge product ids.
pub const VENDOR: u16 = 0x054c;
pub const PRODUCTS: [u16; 2] = [0x0ce6, 0x0df2];

/// Stick values within this of the centre read as centred (Bevy's default gamepad dead zone), so a resting stick
/// leaves the keyboard's stick alone (crate::input_map::sample).
const DEAD_ZONE: f32 = 0.05;

/// An adaptive trigger effect: the 11 bytes of the output report's trigger block (the pad's legacy modes, as SDL's
/// test program and DualSense-Windows use them; positions and forces 0..=255).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trigger(pub [u8; 11]);

impl Trigger {
    /// No effect: the trigger moves freely.
    pub const OFF: Trigger = Trigger([0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    /// Resistance from `start` to the end of the pull.
    pub const fn resistance(start: u8, force: u8) -> Trigger { Trigger([0x01, start, force, 0, 0, 0, 0, 0, 0, 0, 0]) }
    /// Vibration past `start` at `freq` (Hz) and `amp`.
    pub const fn vibration(freq: u8, amp: u8, start: u8) -> Trigger { Trigger([0x06, freq, amp, start, 0, 0, 0, 0, 0, 0, 0]) }
    /// The pad's "weapon" effect (0x25, Nielk1's trigger effect notes): a wall from zone `start` (2..=7) to zone `end`
    /// (`start`+1..=8) of the trigger's ten that snaps when pulled through, at `strength` 1..=8: a gun's trigger break,
    /// a sniper's at full strength.
    pub const fn weapon(start: u8, end: u8, strength: u8) -> Trigger {
        let zones = (1u16 << start) | (1u16 << end);
        Trigger([0x25, zones as u8, (zones >> 8) as u8, strength.saturating_sub(1), 0, 0, 0, 0, 0, 0, 0])
    }

    /// The weapon effect's end zone (where it snaps), `None` for another effect.
    pub fn weapon_end(self) -> Option<u8> {
        (self.0[0] == 0x25).then(|| 15 - u16::from_le_bytes([self.0[1], self.0[2]]).leading_zeros() as u8)
    }
}

impl Default for Trigger {
    fn default() -> Self { Trigger::OFF }
}

/// What the reader thread and the app share: the pad's latest state, the wanted rumble and trigger effect, and the
/// pad's output.
#[derive(Default)]
struct Shared {
    pad: Mutex<Option<PadDevice>>,
    /// The wanted motor strengths: heavy (left) motor in the high byte, light (right) motor in the low byte.
    rumble: AtomicU16,
    /// The wanted effect on R2.
    trigger: Mutex<Trigger>,
    #[cfg(windows)]
    out: Mutex<Option<hid::Output>>,
}

/// The connected pad's latest state, `None` without one, and its rumble.
#[derive(Resource, Clone, Default)]
pub struct DualSense(Arc<Shared>);

impl DualSense {
    pub fn get(&self) -> Option<PadDevice> { *self.0.pad.lock().unwrap_or_else(|e| e.into_inner()) }

    /// The motor strengths to run at (0 = off, 255 = full): `heavy` the left (low-frequency) motor, `light` the right.
    pub fn set_rumble(&self, heavy: u8, light: u8) { self.0.rumble.store(u16::from_be_bytes([heavy, light]), Ordering::Relaxed); }

    /// The effect R2 should have.
    pub fn set_trigger(&self, t: Trigger) { *self.0.trigger.lock().unwrap_or_else(|e| e.into_inner()) = t; }
}

/// Stops the motors and frees R2 when the app ends (its resources drop after `App::run`).
#[derive(Resource)]
pub struct RumbleStop(DualSense);

impl Drop for RumbleStop {
    fn drop(&mut self) {
        self.0.set_rumble(0, 0);
        self.0.set_trigger(Trigger::OFF);
        #[cfg(windows)]
        if let Some(o) = self.0 .0.out.lock().unwrap_or_else(|e| e.into_inner()).as_mut() { o.send(0, 0, Trigger::OFF); }
    }
}

pub struct DualSensePlugin;

impl Plugin for DualSensePlugin {
    fn build(&self, app: &mut App) {
        let state = DualSense::default();
        if std::env::var("RC_DUALSENSE").map(|v| v.trim() == "0").unwrap_or(false) || crate::determinism::deterministic() {
            app.insert_resource(state);
            return;
        }
        app.insert_resource(RumbleStop(state.clone()));
        #[cfg(windows)]
        {
            let shared = state.0.clone();
            let trace = std::env::var("RC_DUALSENSE_TRACE").map(|v| v.trim() == "1").unwrap_or(false);
            let spawned = std::thread::Builder::new().name("dualsense".into()).spawn(move || hid::run(&shared, trace));
            if let Err(e) = spawned { println!("dualsense: no reader thread: {e}"); }
        }
        app.insert_resource(state);
    }
}

/// One stick byte (0 = up/left, 255 = down/right) → Bevy's convention, [−1, 1] with y up applied by the caller.
fn stick(b: u8) -> f32 {
    let v = (b as f32 - 127.5) / 127.5;
    if v.abs() < DEAD_ZONE { 0.0 } else { v.clamp(-1.0, 1.0) }
}

/// The libpad2 bits of the d-pad hat (0 = up, clockwise in eighths, anything above 7 = released).
fn hat(h: u8) -> u32 {
    match h & 0x0f {
        0 => button::UP,
        1 => button::UP | button::RIGHT,
        2 => button::RIGHT,
        3 => button::DOWN | button::RIGHT,
        4 => button::DOWN,
        5 => button::DOWN | button::LEFT,
        6 => button::LEFT,
        7 => button::UP | button::LEFT,
        _ => 0,
    }
}

/// One input report (report id first) → the pad, or `None` for a report that carries no input state.
/// `bluetooth`: the pad is connected over Bluetooth (its report 0x01 is the simple one).
pub fn parse(report: &[u8], bluetooth: bool) -> Option<PadDevice> {
    // Offsets of the sticks, the buttons and R2's travel (the full report: after L2's, right after the sticks; the
    // Bluetooth simple report: after L2's, after the buttons).
    let (sticks, buttons, r2) = match (report.first()?, bluetooth) {
        (0x01, false) => (1, 8, 6),
        (0x01, true) => (1, 5, 9),
        (0x31, true) => (2, 9, 7),
        _ => return None,
    };
    let s = report.get(sticks..sticks + 4)?;
    let r2 = *report.get(r2)? as f32 / 255.0;
    let b = report.get(buttons..buttons + 3)?;
    let mut m = hat(b[0]);
    for (bit, pad) in [(0x10, button::SQUARE), (0x20, button::CROSS), (0x40, button::CIRCLE), (0x80, button::TRIANGLE)] {
        if b[0] & bit != 0 { m |= pad; }
    }
    for (bit, pad) in [
        (0x01, button::L1),
        (0x02, button::R1),
        (0x04, button::L2),
        (0x08, button::R2),
        (0x10, button::SELECT),
        (0x20, button::START),
        (0x40, button::L3),
        (0x80, button::R3),
    ] {
        if b[1] & bit != 0 { m |= pad; }
    }
    // The touchpad click is the remasters' Select (the map); the PS button has no PS2 counterpart.
    if b[2] & 0x02 != 0 { m |= button::SELECT; }
    Some(PadDevice {
        left: Vec2::new(stick(s[0]), -stick(s[1])),
        right: Vec2::new(stick(s[2]), -stick(s[3])),
        buttons: m,
        r2,
    })
}

/// The output report that runs the motors (`len`: the pad's output report length, report id included), `None` for a
/// length too short for the DualSense's (USB 0x02: 48, Bluetooth 0x31: 78). Windows wants every write as long as the
/// pad's longest output report (over Bluetooth 547 bytes, report 0x39), so the report is padded with zeros to `len`,
/// as hidapi does. The common part (hid-playstation.c `dualsense_output_report_common`, SDL's `DS5EffectsState_t`):
/// valid flags 0 / 1, the right (light) and left (heavy) motors, then at +10 the right trigger's effect.
pub fn output_report(len: usize, bluetooth: bool, seq: u8, heavy: u8, light: u8, trigger: Trigger) -> Option<Vec<u8>> {
    let mut r = vec![0u8; len];
    let common = if bluetooth {
        if len < 78 { return None; }
        r[0] = 0x31;
        r[1] = (seq & 0xf) << 4;
        // DS_OUTPUT_TAG.
        r[2] = 0x10;
        3
    } else {
        if len < 48 { return None; }
        r[0] = 0x02;
        1
    };
    // DS_OUTPUT_VALID_FLAG0_COMPATIBLE_VIBRATION | DS_OUTPUT_VALID_FLAG0_HAPTICS_SELECT | the right trigger's effect.
    r[common] = 0x07;
    r[common + 2] = light;
    r[common + 3] = heavy;
    r[common + 10..common + 21].copy_from_slice(&trigger.0);
    if bluetooth {
        // CRC-32 over the Bluetooth output header byte 0xa2 and the report, little-endian in the last four bytes.
        let crc = crc32(&[&[0xa2], &r[..74]]);
        r[74..78].copy_from_slice(&crc.to_le_bytes());
    }
    Some(r)
}

/// CRC-32 (IEEE, zlib's) over the parts in order.
fn crc32(parts: &[&[u8]]) -> u32 {
    let mut c = !0u32;
    for &b in parts.iter().flat_map(|p| p.iter()) {
        c ^= b as u32;
        for _ in 0..8 { c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 }; }
    }
    !c
}

#[cfg(windows)]
mod hid {
    //! The Windows HID calls: cfgmgr32 lists the HID interfaces, hid.dll reads ids and report sizes, and the reports
    //! come through `ReadFile` (std's `File::read`).
    use std::fs::{File, OpenOptions};
    use std::io::Read;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::io::Write;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{output_report, parse, Shared, Trigger, PRODUCTS, VENDOR};

    #[repr(C)]
    #[derive(Default)]
    struct Guid(u32, u16, u16, [u8; 8]);

    #[repr(C)]
    #[derive(Default)]
    struct Attributes {
        size: u32,
        vendor: u16,
        product: u16,
        version: u16,
    }

    /// HIDP_CAPS: usage, usage page, the three report lengths, then 17 reserved and 10 count fields.
    #[repr(C)]
    struct Caps {
        usage: u16,
        usage_page: u16,
        input_len: u16,
        output_len: u16,
        feature_len: u16,
        rest: [u16; 27],
    }

    const HIDP_STATUS_SUCCESS: i32 = 0x0011_0000;

    #[link(name = "hid")]
    unsafe extern "system" {
        fn HidD_GetHidGuid(guid: *mut Guid);
        fn HidD_GetAttributes(device: *mut core::ffi::c_void, attributes: *mut Attributes) -> u8;
        fn HidD_GetPreparsedData(device: *mut core::ffi::c_void, data: *mut isize) -> u8;
        fn HidD_FreePreparsedData(data: isize) -> u8;
        fn HidP_GetCaps(data: isize, caps: *mut Caps) -> i32;
    }

    #[link(name = "cfgmgr32")]
    unsafe extern "system" {
        fn CM_Get_Device_Interface_List_SizeW(len: *mut u32, class: *const Guid, device: *const u16, flags: u32) -> u32;
        fn CM_Get_Device_Interface_ListW(class: *const Guid, device: *const u16, buffer: *mut u16, len: u32, flags: u32) -> u32;
    }

    /// The device paths of the present HID interfaces.
    fn hid_paths() -> Vec<String> {
        let mut guid = Guid::default();
        let mut len = 0u32;
        // SAFETY: each call writes only the GUID / length / buffer it is given, the buffer `len` UTF-16 units long.
        let list = unsafe {
            HidD_GetHidGuid(&mut guid);
            if CM_Get_Device_Interface_List_SizeW(&mut len, &guid, std::ptr::null(), 0) != 0 || len == 0 { return Vec::new(); }
            let mut buf = vec![0u16; len as usize];
            if CM_Get_Device_Interface_ListW(&guid, std::ptr::null(), buf.as_mut_ptr(), len, 0) != 0 { return Vec::new(); }
            buf
        };
        list.split(|&c| c == 0).filter(|p| !p.is_empty()).map(String::from_utf16_lossy).collect()
    }

    /// A DualSense's path by its name, before opening anything: USB `vid_054c&pid_0ce6`, Bluetooth
    /// `_vid&0002054c_pid&0ce6`.
    fn named_like_dualsense(path: &str) -> bool {
        let p = path.to_ascii_lowercase();
        (p.contains(&format!("{VENDOR:04x}_pid")) || p.contains(&format!("{VENDOR:04x}&pid")))
            && PRODUCTS.iter().any(|id| p.contains(&format!("pid&{id:04x}")) || p.contains(&format!("pid_{id:04x}")))
    }

    struct Pad {
        file: File,
        input_len: usize,
        output_len: usize,
        bluetooth: bool,
    }

    /// The pad's output: its own handle (a write on the reading handle would wait for the blocked read), the report's
    /// length, and the Bluetooth report's sequence number.
    pub struct Output {
        file: File,
        len: usize,
        bluetooth: bool,
        seq: u8,
    }

    impl Output {
        /// Sends the motor strengths and R2's effect; false when the write fails (the pad is gone).
        pub fn send(&mut self, heavy: u8, light: u8, trigger: Trigger) -> bool {
            let Some(r) = output_report(self.len, self.bluetooth, self.seq, heavy, light, trigger) else { return false };
            self.seq = (self.seq + 1) & 0xf;
            self.file.write_all(&r).is_ok()
        }
    }

    /// Shared read / write: other programs (Steam, DualSenseX) may hold the pad open too.
    fn open_file(path: &str) -> Option<File> { OpenOptions::new().read(true).write(true).share_mode(0x1 | 0x2).open(path).ok() }

    fn open(path: &str) -> Option<Pad> {
        let file = open_file(path)?;
        let h = file.as_raw_handle();
        let mut attr = Attributes { size: size_of::<Attributes>() as u32, ..Default::default() };
        let mut data = 0isize;
        let mut caps = Caps { usage: 0, usage_page: 0, input_len: 0, output_len: 0, feature_len: 0, rest: [0; 27] };
        // SAFETY: the handle is open for the calls; each writes only the struct it is given; the preparsed data is
        // freed once the caps are read.
        let ok = unsafe {
            HidD_GetAttributes(h, &mut attr) != 0
                && HidD_GetPreparsedData(h, &mut data) != 0
                && {
                    let r = HidP_GetCaps(data, &mut caps);
                    HidD_FreePreparsedData(data);
                    r == HIDP_STATUS_SUCCESS
                }
        };
        // The game pad collection (generic desktop page 1, usage 5), not the pad's other interfaces (audio over USB).
        if !ok || attr.vendor != VENDOR || !PRODUCTS.contains(&attr.product) || caps.usage_page != 1 || caps.usage != 5 {
            return None;
        }
        // USB input reports are 64 bytes; over Bluetooth the descriptor's longest is 78 (report 0x31).
        let input_len = caps.input_len as usize;
        Some(Pad { file, input_len, output_len: caps.output_len as usize, bluetooth: input_len != 64 })
    }

    /// Scans, reads, and rescans after a disconnect, for the life of the process. Each report read is also when the
    /// rumble is sent, if it changed (the pad reports every few milliseconds).
    pub fn run(shared: &Arc<Shared>, trace: bool) {
        let set = |v: Option<super::PadDevice>| *shared.pad.lock().unwrap_or_else(|e| e.into_inner()) = v;
        let set_out = |o: Option<Output>| *shared.out.lock().unwrap_or_else(|e| e.into_inner()) = o;
        loop {
            let found = hid_paths().into_iter().filter(|p| named_like_dualsense(p)).find_map(|p| open(&p).map(|pad| (pad, p)));
            let Some((mut pad, path)) = found else {
                std::thread::sleep(Duration::from_secs(1));
                continue;
            };
            println!("dualsense: connected over {}", if pad.bluetooth { "Bluetooth" } else { "USB" });
            set_out(open_file(&path).map(|file| Output { file, len: pad.output_len, bluetooth: pad.bluetooth, seq: 0 }));
            // The rumble and trigger effect last sent and when: a running motor is sent again every second, in case a write
            // was lost. Nothing is sent before the first rumble or effect: over Bluetooth an output report switches the pad
            // to its extended reports, which other programs reading it through Windows.Gaming.Input then miss until it is
            // turned off and on.
            let mut sent = (0u16, Trigger::OFF, Instant::now());
            let mut warned = false;
            let mut buf = vec![0u8; pad.input_len.max(1)];
            let mut reported = false;
            while let Ok(n) = pad.file.read(&mut buf) {
                if n == 0 { break; }
                let Some(state) = parse(&buf[..n], pad.bluetooth) else { continue };
                if !reported {
                    println!("dualsense: reading report {:#04x}", buf[0]);
                    reported = true;
                }
                if trace { println!("dualsense: {state:?}"); }
                set(Some(state));
                let want = shared.rumble.load(Ordering::Relaxed);
                let trigger = *shared.trigger.lock().unwrap_or_else(|e| e.into_inner());
                if want != sent.0 || trigger != sent.1 || (want != 0 && sent.2.elapsed() > Duration::from_secs(1)) {
                    let [heavy, light] = want.to_be_bytes();
                    let ok = shared.out.lock().unwrap_or_else(|e| e.into_inner()).as_mut().is_some_and(|o| o.send(heavy, light, trigger));
                    if !ok && !warned {
                        println!("dualsense: rumble not sent (output report length {})", pad.output_len);
                        warned = true;
                    }
                    sent = (want, trigger, Instant::now());
                }
            }
            set(None);
            set_out(None);
            println!("dualsense: disconnected");
        }
    }

    #[cfg(test)]
    mod tests {
        use super::named_like_dualsense;

        #[test]
        fn paths() {
            assert!(named_like_dualsense(r"\\?\hid#vid_054c&pid_0ce6&mi_03#8&2b5e5c&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"));
            assert!(named_like_dualsense(
                r"\\?\HID#{00001124-0000-1000-8000-00805f9b34fb}_VID&0002054c_PID&0ce6#9&1a2b3c4d&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
            ));
            assert!(!named_like_dualsense(r"\\?\hid#vid_046d&pid_c52b&mi_00#7&1#{4d1e55b2-f16f-11cf-88cb-001111000030}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A USB report 0x01 with the given sticks and button bytes (the triggers and counter zero).
    fn usb(sticks: [u8; 4], b: [u8; 3]) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0x01;
        r[1..5].copy_from_slice(&sticks);
        r[8..11].copy_from_slice(&b);
        r
    }

    #[test]
    fn rumble_reports() {
        assert_eq!(crc32(&[b"123456789"]), 0xcbf4_3926);
        let usb = output_report(48, false, 0, 200, 100, Trigger::OFF).unwrap();
        assert_eq!((usb[0], usb[1], usb[3], usb[4], usb[11]), (0x02, 0x07, 100, 200, 0x05));
        let bt = output_report(78, true, 5, 255, 0, Trigger::weapon(3, 6, 8)).unwrap();
        assert_eq!(&bt[..7], &[0x31, 0x50, 0x10, 0x07, 0x00, 0, 255]);
        assert_eq!(&bt[13..17], &[0x25, 0x48, 0x00, 7]);
        assert_eq!(u32::from_le_bytes(bt[74..].try_into().unwrap()), crc32(&[&[0xa2], &bt[..74]]));
        // Padded to the pad's longest output report, the CRC still over the first 74 bytes.
        let long = output_report(547, true, 5, 255, 0, Trigger::weapon(3, 6, 8)).unwrap();
        assert_eq!((long.len(), &long[..78], long[78..].iter().all(|&b| b == 0)), (547, &bt[..], true));
        assert_eq!(output_report(64, true, 0, 1, 1, Trigger::OFF), None);
        assert_eq!(output_report(40, false, 0, 1, 1, Trigger::OFF), None);
    }

    #[test]
    fn usb_report() {
        let p = parse(&usb([128, 128, 128, 128], [0x08, 0, 0]), false).unwrap();
        assert_eq!(p, PadDevice::default());
        let p = parse(&usb([255, 0, 0, 255], [0x20 | 0x02, 0x01 | 0x20, 0x02]), false).unwrap();
        assert_eq!(p.left, Vec2::new(1.0, 1.0));
        assert_eq!(p.right, Vec2::new(-1.0, -1.0));
        assert_eq!(p.buttons, button::CROSS | button::RIGHT | button::L1 | button::START | button::SELECT);
        let mut r = usb([128, 128, 128, 128], [0x08, 0x08, 0]);
        r[6] = 255;
        assert_eq!(parse(&r, false).unwrap().r2, 1.0);
    }

    #[test]
    fn bluetooth_reports() {
        // Extended 0x31: one byte more before the same layout.
        let mut r = vec![0u8; 78];
        r[0] = 0x31;
        r[2..6].copy_from_slice(&[128, 128, 128, 128]);
        r[9..12].copy_from_slice(&[0x80 | 0x06, 0x40, 0]);
        r[7] = 51;
        let p = parse(&r, true).unwrap();
        assert_eq!((p.buttons, p.r2), (button::TRIANGLE | button::LEFT | button::L3, 0.2));
        // Simple 0x01: the buttons right after the sticks.
        let mut r = vec![0u8; 78];
        r[0] = 0x01;
        r[1..5].copy_from_slice(&[128, 0, 128, 128]);
        r[5..8].copy_from_slice(&[0x10 | 0x08, 0x10, 0]);
        r[9] = 255;
        let p = parse(&r, true).unwrap();
        assert_eq!(p.left, Vec2::new(0.0, 1.0));
        assert_eq!((p.buttons, p.r2), (button::SQUARE | button::SELECT, 1.0));
        // Other reports carry no input.
        assert_eq!(parse(&[0x05, 0, 0], true), None);
    }
}
