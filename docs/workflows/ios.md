# Workflow: iOS (iPhone / iPad)

Tools: `tools/ios/build.sh`, `tools/ios/run-simulator.sh`, `.github/workflows/ios.yml`. Port-only code:
`crates/rc-engine/src/ios.rs` (start-up, first-run disc setup, audio session), `touch_pad.rs` (touch controls),
`gc_pad.rs` (Bluetooth / MFi controllers through GameController). Everything is behind `cfg(target_os = "ios")`.

| Task | Command |
|---|---|
| iPhone build (unsigned `.ipa` for Sideloadly / AltStore) | `tools/ios/build.sh device` → `dist/ios/ReRAC.ipa` |
| Simulator build | `tools/ios/build.sh simulator` → `dist/ios-sim/ReRAC.app` |
| Run in the Simulator (first run copies the ISO) | `tools/ios/run-simulator.sh /path/to/disc.iso`, then `tools/ios/run-simulator.sh` |

Needs macOS with Xcode and rustup (the script adds the `aarch64-apple-ios` / `aarch64-apple-ios-sim` targets). The
app is a plain bundle around the `rerac` executable (no Xcode project): `tools/ios/Info.plist`, the shaders in
`assets/` next to the executable (`main.rs` `asset_dir`) and the icon from `docs/assets/rerac-logo.png`. Env:
`BUNDLE_ID` (default `com.rerac.ios`), `PROFILE` (default `release`), `SIGN_IDENTITY` (optional codesign identity).
CI builds both and uploads `ReRAC-ipa` and `ReRAC-simulator`; its smoke test boots the app in an iPhone 16 Pro Max
simulator with no disc and checks it reaches the setup screen.

## What the port does

* **Start-up** (`ios.rs`): an iOS app has no launcher, so it does the launcher's part itself. The data folder is
  `Library/Application Support/rerac/data` in the sandbox (`RC_DATA_DIR`); settings and the memory card live next to
  it. Without extracted data it runs a small setup app: it waits for a `.iso` in `Documents/` (the folder the Files
  app shows as *On My iPhone › ReRAC*), extracts and verifies it with `rc_extract` on a worker thread (the same
  stages as `rerac-extract extract`), shows the progress and asks for a restart (one event loop per process).
  Defaults: full screen, 720p render resolution (`RC_RESOLUTION`).
* **Audio session**: the shared `AVAudioSession` gets the Playback category. cpal leaves the session alone, and the
  default category (SoloAmbient) is muted by the Ring / Silent switch, so without this the game is silent whenever
  the phone is in silent mode.
* **Touch controls** (`touch_pad.rs`): floating left stick (left half), camera drag (right half), ✕ ○ □ △,
  L1/L2/R1/R2, d-pad, SELECT and START; hidden while a controller is connected.
* **Controllers** (`gc_pad.rs`): DualSense, DualShock 4, Xbox and MFi through GameController.

## Installing on an iPhone (free Apple ID)

Tested on 14 October 2026: iPhone 16 Pro Max on iOS 26.7.1, MacBook Pro M3 on macOS 14.5, Sideloadly 0.70.1. The
app installs, extracts the disc and plays the opening movie with sound; gameplay performance and controls on the
device are not checked yet.

1. Get `ReRAC.ipa` (the CI artifact `ReRAC-ipa`, or `tools/ios/build.sh device`).
2. Connect the iPhone by cable, unlock it and tap **Trust**. With a macOS / Xcode older than the phone's iOS, Xcode
   may never see the phone; Sideloadly still does.
3. In Sideloadly, **drag the `.ipa` onto the window**. The IPA button's file picker can hang Sideloadly when Desktop,
   Documents or Downloads are slow to answer (e.g. iCloud Drive). Pick the iPhone, enter the Apple ID, set
   *Advanced options › Anisette Authentication* to **Remote** (with the local mode the login can fail with
   `Login failed (-22406): Enter the correct password` even with the right password) and press **Start**. On iOS 26
   Sideloadly changes the bundle id (`com.rerac.ios.<suffix>`).
4. On the iPhone: *Settings › Privacy & Security › Developer Mode* (then restart), and trust the Apple ID in
   *Settings › General › VPN & Device Management*.
5. Open ReRAC once, then copy the `.iso` into its folder (Finder › the iPhone › Files › ReRAC, or the Files app).
   **Do not open the app while it copies** and keep the phone unlocked, or the copy stops halfway. From a terminal
   with libimobiledevice: `afcclient --container <bundle id> put disc.iso /Documents/disc.iso` (`--documents` is
   refused with *Permission denied* on iOS 26). Leave free at least twice the ISO's size for the extraction.
6. Open ReRAC: it extracts the disc (a few minutes) and asks for a restart. The ISO can be deleted afterwards.
7. A free Apple ID signature expires after 7 days: press Start in Sideloadly again with the same `.ipa` (the data,
   saves and settings are kept). Free accounts are also limited to 3 sideloaded apps and 10 new app ids per week.
