//! Player-facing render settings, applied to the world cameras at start and whenever they change.
//!
//! **Multisampling.** The GS has no multisampling: every world pass draws one sample per pixel, so the default
//! is [`Msaa::Off`] (faithful, and cheaper: no 4× colour/depth load/store, writeback and resolve per pass). The
//! samples come from the Anti-aliasing option (crate::graphics: Original and Off draw one; `RC_MSAA=0|2|4|8` there);
//! the option changes [`RenderSettings::msaa`] at run time, and [`apply`] then sets it on the main and
//! sky cameras together (they share the depth buffer: the main camera loads what the sky pass cleared).
//! Changing it re-specialises every world pipeline once (a hitch of the first frame after the change).
//! `RC_MSAA_SWITCH=<frame>:<samples>` changes the setting at that frame (a test of the run-time path).
//!
//! **Device support** ([`SupportedMsaa`]): not every GPU can multisample every format at every count (Apple M-series:
//! 1, 2, 4 but not 8 for `Rgba8UnormSrgb` / `Depth32Float`; WebGPU guarantees only 1 and 4). At start the counts the
//! adapter supports for every attachment the world cameras multisample ([`WORLD_FORMATS`]: the main colour texture and
//! the depth buffer) are intersected, from `RenderAdapter::get_texture_format_features` when the device has
//! `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` (Bevy requests it by default), else the formats' WebGPU guarantee. A
//! requested count outside that set — from `RC_MSAA`, the settings file, `RC_MSAA_SWITCH` or the menu — becomes the
//! highest supported count below it ([`SupportedMsaa::clamp`], one warning line); a stored value never crashes the
//! start and the file is not rewritten then (only a change on the Port Options page writes it, and that page offers
//! supported counts only).
//!
//! **Persistence** (port-only; the "Port Options" page, `rc_game::menus::pause::port`): the options are kept in the
//! port settings file ([`load_key`] / [`save_key`]; the Anti-aliasing option's own key: crate::graphics). The file is plain `key = value` text (std only): `~/Library/Application Support/rerac/
//! settings.toml` on macOS, `$XDG_CONFIG_HOME` (or `~/.config`) `/rerac/settings.toml` elsewhere,
//! `%APPDATA%\rerac\settings.toml` on Windows. When that file is missing and the pre-rename `randcrw/settings.toml`
//! exists, the old file is copied over once and kept ([`migrate_legacy`]). `RC_SETTINGS_FILE=<path>` picks another
//! file and `RC_SETTINGS_FILE=0` (or empty) disables it. Frame-exact / deterministic runs neither read nor write the
//! default file (their output must be a function of the environment only); an explicit `RC_SETTINGS_FILE`
//! still applies to them. Unknown keys and comments are kept when the file is rewritten.
//!
//! Cameras that render elsewhere keep their own setting: the HUD camera (offscreen 512×416, `Msaa::Off`) and
//! the menu layer's 3D camera (its own image, `Msaa::Off`). The consumers of the main target's output
//! (HUD composite UI node, underwater tint and scene fade passes, the menu snapshot copy, the frame-exact
//! capture) all read the resolved single-sample texture, so they work in every mode.

use bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, WgpuFeatures};
use bevy::render::renderer::{RenderAdapter, RenderDevice};
use bevy::render::view::Msaa;

/// A camera that draws the 3D world into the primary target (the main camera and the sky camera).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct WorldCamera;

/// Render settings a menu can change at run time.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderSettings {
    /// Multisampling of the world cameras (default [`Msaa::Off`], like the GS).
    pub msaa: Msaa,
}

impl Default for RenderSettings {
    fn default() -> Self { RenderSettings { msaa: Msaa::Off } }
}

impl RenderSettings {
    /// The start settings: the Anti-aliasing option's sample count (crate::graphics: the port settings file, then
    /// `RC_GFX_ANTI_ALIASING` / `RC_MSAA` over it).
    pub fn startup() -> (Self, &'static str) {
        let aa = crate::graphics::GraphicsSettings::startup().aa;
        let source = if std::env::var_os("RC_MSAA").is_some() { "RC_MSAA" } else { "settings" };
        (RenderSettings { msaa: msaa_from_samples(aa.samples()) }, source)
    }
}

/// 2, 4 and 8 samples; anything else is off.
pub fn msaa_from_samples(n: u32) -> Msaa {
    match n {
        2 => Msaa::Sample2,
        4 => Msaa::Sample4,
        8 => Msaa::Sample8,
        _ => Msaa::Off,
    }
}

/// The attachments the world cameras multisample: the main colour texture (an LDR camera on the window: the
/// swapchain format normalised to `Rgba8UnormSrgb`, Bevy's `main_texture_sampled`) and the 3D depth buffer
/// (`Depth32Float`, `view_depth_texture`, shared by the sky and main cameras). No prepass / HDR / OIT targets.
pub const WORLD_FORMATS: [TextureFormat; 2] = [TextureFormat::Rgba8UnormSrgb, CORE_3D_DEPTH_FORMAT];

/// The sample counts the world cameras can use on this device (ascending; always contains 1 = off).
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct SupportedMsaa(pub Vec<u32>);

impl Default for SupportedMsaa {
    /// What WebGPU guarantees for every renderable format (1 and 4).
    fn default() -> Self { SupportedMsaa(vec![1, 4]) }
}

impl SupportedMsaa {
    /// The counts supported for every [`WORLD_FORMATS`] format: the adapter's own format features when the device
    /// was created with `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` (only then may it use them), else the WebGPU
    /// guarantee for the device's features. Without a renderer: [`Default`].
    pub fn detect(adapter: Option<&RenderAdapter>, device: Option<&RenderDevice>) -> (Self, &'static str) {
        let (Some(adapter), Some(device)) = (adapter, device) else { return (Self::default(), "no renderer: WebGPU guarantee") };
        let features = device.features();
        let specific = features.contains(WgpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        let counts = [1u32, 2, 4, 8]
            .into_iter()
            .filter(|&n| {
                WORLD_FORMATS.iter().all(|&f| {
                    let flags = if specific { adapter.get_texture_format_features(f).flags } else { f.guaranteed_format_features(features).flags };
                    flags.sample_count_supported(n)
                })
            })
            .collect();
        (SupportedMsaa(counts), if specific { "adapter format features" } else { "WebGPU guarantee" })
    }

    pub fn contains(&self, m: Msaa) -> bool { self.0.contains(&m.samples()) }

    /// `m` if supported, else the highest supported count below it (off at worst).
    pub fn clamp(&self, m: Msaa) -> Msaa {
        let n = self.0.iter().copied().filter(|&n| n <= m.samples()).max().unwrap_or(1);
        msaa_from_samples(n)
    }

    /// `m` clamped ([`Self::clamp`]), with a warning line naming `source` when it changed.
    pub fn clamp_warn(&self, m: Msaa, source: &str) -> Msaa {
        let c = self.clamp(m);
        if c != m {
            warn!(
                "render settings: MSAA {} samples ({source}) is not supported by this GPU for {:?} (supported: {:?}); using {}",
                m.samples(), WORLD_FORMATS, self.0, c.samples()
            );
        }
        c
    }
}

/// The port settings file (module docs), or None when disabled.
pub fn settings_path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Some(v) = std::env::var_os("RC_SETTINGS_FILE") {
        return (!v.is_empty() && v != "0").then(|| PathBuf::from(v));
    }
    if crate::determinism::deterministic() { return None; }
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        var("HOME")?.join("Library/Application Support")
    } else if cfg!(windows) {
        var("APPDATA")?
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config")))?
    };
    // Once per process: the first lookup copies a pre-rename file over (later ones find the new file anyway).
    static MIGRATED: std::sync::Once = std::sync::Once::new();
    MIGRATED.call_once(|| match migrate_legacy(&base) {
        Ok(Some((old, new))) => println!("render settings: copied {} to {} (the old file is kept)", old.display(), new.display()),
        Ok(None) => {}
        Err(e) => eprintln!("render settings: could not copy the old settings file: {e}"),
    });
    Some(settings_file_in(&base))
}

/// The engine's own settings folder name under the per-OS config base dir.
const SETTINGS_DIR: &str = "rerac";
/// The folder name before the rename to ReRAC (read once by [`migrate_legacy`], never written).
const LEGACY_SETTINGS_DIR: &str = "randcrw";
/// The files the engine itself keeps in that folder.
const SETTINGS_FILE: &str = "settings.toml";

/// The settings file under a per-OS config base dir (`<base>/rerac/settings.toml`).
fn settings_file_in(base: &std::path::Path) -> std::path::PathBuf { base.join(SETTINGS_DIR).join(SETTINGS_FILE) }

/// The rename to ReRAC: when `<base>/rerac/settings.toml` is missing and `<base>/randcrw/settings.toml` exists,
/// copies it (never moves, modifies or deletes the old one). Returns `(old, new)` when it copied. Only the engine's
/// own file is copied, not the whole folder: on macOS the same `Application Support/<name>/` folder is also the
/// launcher's data root (game data, versions), which is the launcher's to migrate.
pub fn migrate_legacy(base: &std::path::Path) -> std::io::Result<Option<(std::path::PathBuf, std::path::PathBuf)>> {
    let (old, new) = (base.join(LEGACY_SETTINGS_DIR).join(SETTINGS_FILE), settings_file_in(base));
    if new.exists() || !old.is_file() { return Ok(None); }
    if let Some(dir) = new.parent() { std::fs::create_dir_all(dir)?; }
    std::fs::copy(&old, &new)?;
    Ok(Some((old, new)))
}

/// The value of `key` in `key = value` text (the last one wins; `#` comments, quotes around the value allowed).
fn read_key<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.split('#').next().unwrap_or("").trim().trim_matches('"'))
        .next_back()
}

/// `text` with `key = value` set (the existing line replaced in place, else appended).
fn write_key(text: &str, key: &str, value: &str) -> String {
    let mut found = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| match l.split_once('=') {
            Some((k, _)) if k.trim() == key && !l.trim_start().starts_with('#') => {
                found = true;
                format!("{key} = {value}")
            }
            _ => l.to_string(),
        })
        .collect();
    if out.is_empty() { out.push("# ReRAC port settings (Port Options page; RC_GFX_* switches override at start)".into()); }
    if !found { out.push(format!("{key} = {value}")); }
    out.join("\n") + "\n"
}

/// The value of `key` in the port settings file (None: no file, or no such key).
pub fn load_key(key: &str) -> Option<String> {
    let text = std::fs::read_to_string(settings_path()?).ok()?;
    read_key(&text, key).map(str::to_string)
}

/// Writes `key = value` into the port settings file (other lines kept; a no-op when the file is disabled); errors
/// are reported, not fatal.
pub fn save_key(key: &str, value: &str) {
    let Some(path) = settings_path() else { return };
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    let new = write_key(&old, key, value);
    let res = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, new));
    match res {
        Ok(()) => println!("render settings: saved {key} = {value} to {}", path.display()),
        Err(e) => eprintln!("render settings: could not write {}: {e}", path.display()),
    }
}

pub struct RenderSettingsPlugin;

impl Plugin for RenderSettingsPlugin {
    fn build(&self, app: &mut App) {
        let (settings, source) = RenderSettings::startup();
        if settings.msaa != Msaa::Off { println!("render settings: MSAA {} samples requested ({source})", settings.msaa.samples()); }
        // Clamped to the device in `finish` (the renderer exists by then); `apply` re-checks every change.
        app.insert_resource(settings).insert_resource(StartSource(source)).add_systems(PostUpdate, apply);
        let switch = std::env::var("RC_MSAA_SWITCH").ok().and_then(|v| {
            let (f, n) = v.trim().split_once(':')?;
            Some((f.parse::<u64>().ok()?, msaa_from_samples(n.parse().ok()?)))
        });
        if let Some((frame, msaa)) = switch {
            app.add_systems(Update, move |f: Res<crate::determinism::FrameNumber>, mut s: ResMut<RenderSettings>| {
                if f.0 == frame {
                    println!("render settings: frame {frame}: MSAA {} samples (RC_MSAA_SWITCH)", msaa.samples());
                    s.msaa = msaa;
                }
            });
        }
    }

    /// After `RenderPlugin::finish` (added before this plugin), which puts the adapter and device into the main world.
    fn finish(&self, app: &mut App) {
        let world = app.world_mut();
        let (supported, how) = SupportedMsaa::detect(world.get_resource::<RenderAdapter>(), world.get_resource::<RenderDevice>());
        let source = world.remove_resource::<StartSource>().map_or("start", |s| s.0);
        let mut settings = world.resource_mut::<RenderSettings>();
        if settings.msaa != Msaa::Off {
            println!("render settings: supported MSAA sample counts {:?} ({how})", supported.0);
            // Not written back to the settings file: only a change on the Port Options page saves.
            let clamped = supported.clamp_warn(settings.msaa, source);
            if clamped != settings.msaa { settings.msaa = clamped; }
        }
        world.insert_resource(supported);
    }
}

/// Where the start value came from (for the clamp warning in `finish`).
#[derive(Resource)]
struct StartSource(&'static str);

/// Writes the setting onto every world camera when it changed, and onto world cameras as they are spawned; a value
/// the device cannot multisample (any writer) is first clamped ([`SupportedMsaa::clamp`]).
pub fn apply(
    mut settings: ResMut<RenderSettings>,
    supported: Option<Res<SupportedMsaa>>,
    mut cams: Query<(Ref<WorldCamera>, &mut Msaa)>,
    mut commands: Commands,
    missing: Query<Entity, (With<WorldCamera>, Without<Msaa>)>,
) {
    let changed = settings.is_changed();
    if changed {
        let s = supported.as_deref().cloned().unwrap_or_default();
        if !s.contains(settings.msaa) { settings.msaa = s.clamp_warn(settings.msaa, "run-time change"); }
    }
    let want = settings.msaa;
    for e in &missing { commands.entity(e).insert(want); }
    for (marker, mut msaa) in &mut cams {
        if (changed || marker.is_added()) && *msaa != want { *msaa = want; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples() {
        assert_eq!(msaa_from_samples(0), Msaa::Off);
        assert_eq!(msaa_from_samples(1), Msaa::Off);
        assert_eq!(msaa_from_samples(4), Msaa::Sample4);
        assert_eq!(msaa_from_samples(8), Msaa::Sample8);
        assert_eq!(RenderSettings::default().msaa, Msaa::Off);
    }

    #[test]
    fn clamp_to_supported() {
        let apple = SupportedMsaa(vec![1, 2, 4]);
        assert_eq!(apple.clamp(Msaa::Sample8), Msaa::Sample4);
        assert_eq!(apple.clamp(Msaa::Sample2), Msaa::Sample2);
        assert_eq!(apple.clamp(Msaa::Off), Msaa::Off);
        let webgpu = SupportedMsaa::default();
        assert_eq!(webgpu.clamp(Msaa::Sample2), Msaa::Off);
        assert_eq!(webgpu.clamp(Msaa::Sample8), Msaa::Sample4);
        assert!(!webgpu.contains(Msaa::Sample8) && webgpu.contains(Msaa::Off));
        assert_eq!(SupportedMsaa(vec![1]).clamp(Msaa::Sample4), Msaa::Off);
        assert_eq!(SupportedMsaa::detect(None, None).0, webgpu);
    }

    #[test]
    fn legacy_settings_are_copied_once() {
        let base = std::env::temp_dir().join(format!("rerac-settings-migration-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (old_dir, new_dir) = (base.join("randcrw"), base.join("rerac"));
        let (old, new) = (old_dir.join("settings.toml"), new_dir.join("settings.toml"));
        // Nothing to copy: no files are created.
        assert!(migrate_legacy(&base).unwrap().is_none());
        assert!(!new_dir.exists());
        std::fs::create_dir_all(old_dir.join("games")).unwrap();
        std::fs::write(&old, "msaa = 4\n").unwrap();
        std::fs::write(old_dir.join("games/big.bin"), "launcher data").unwrap();
        // The new folder may already exist (the launcher's data root on macOS): the file is still copied.
        std::fs::create_dir_all(&new_dir).unwrap();
        assert_eq!(migrate_legacy(&base).unwrap(), Some((old.clone(), new.clone())));
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "msaa = 4\n");
        assert!(!new_dir.join("games").exists(), "only the engine's own file is copied");
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "msaa = 4\n", "the old file is kept");
        // The new file exists now: it is never overwritten from the old one.
        std::fs::write(&new, "msaa = 8\n").unwrap();
        assert!(migrate_legacy(&base).unwrap().is_none());
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "msaa = 8\n");
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "msaa = 4\n", "the old file is never modified");
        assert_eq!(settings_file_in(&base), new);
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn settings_text() {
        assert_eq!(read_key("# x\nmsaa = 4\n", "msaa"), Some("4"));
        assert_eq!(read_key("msaa=\"8\" # hi\nother = 1", "msaa"), Some("8"));
        assert_eq!(read_key("other = 1", "msaa"), None);
        assert_eq!(write_key("# c\nother = 1\nmsaa = 2\n", "msaa", "4"), "# c\nother = 1\nmsaa = 4\n");
        let fresh = write_key("", "msaa", "8");
        assert!(fresh.starts_with('#') && fresh.ends_with("msaa = 8\n"));
        assert_eq!(read_key(&fresh, "msaa"), Some("8"));
    }
}
