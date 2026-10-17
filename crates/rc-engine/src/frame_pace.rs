//! **Port-only: frame pacing.** The game runs at 60 Hz (one logic tick per NTSC field) and the port draws no frame
//! between two ticks (nothing is interpolated), so a frame drawn without a tick is the previous picture again, and a
//! frame that runs two ticks skips one: either shows as a stutter. This module keeps the ticks at 60 a second and
//! draws at most [`FrameLimit`] frames a second (the Port Options' "FPS limit"; default 60, one tick per frame).
//!
//! * **Ticks** (`First`, before Bevy's time update): each frame advances `TimeUpdateStrategy::FixedTimesteps(n)`
//!   ticks, `n` from the real time since the last frame start counted in ticks with a remainder that carries over:
//!   while the limiter runs at 60 frames a second or fewer, `n` rounds (a frame a little early or late is still one
//!   tick; one that arrives more than half a tick early still gets its tick, the remainder kept within ±½), so the
//!   usual frame is exactly one tick (two at 30) and only a real hiccup catches up; above 60 (and without the limiter)
//!   `n` is the plain floor of the elapsed ticks, so the frames between two ticks repeat the picture. At most
//!   [`MAX_STEPS`] a frame: time lost in a longer stall (a level load) is dropped.
//! * **Limiter** (`Last`): the frame waits for its deadline. The period follows the primary monitor: at ~the limit
//!   none (vsync paces the loop; a second clock beside it would beat against the display and drop a frame now and
//!   then); at a multiple of ~the limit (60 on 120 Hz ProMotion or 240 Hz) that many refreshes, so the frames stay on
//!   the display's own rhythm; otherwise 1/limit s (60 on a 144 Hz screen without variable refresh still shows some
//!   frames longer than others). Vsync stays on, so a limit above the display's rate gives the display's rate.
//!   The wait sleeps to 0.3 ms before the deadline, then yields until it (the yield loop costs CPU: keep it short).
//! * **Unlimited**: no limiter, and the window presents without vsync.
//!
//! The limit is kept in the port settings file (`fps_limit = 30|60|120|144|unlimited`, crate::render_settings).
//! `RC_FRAME_CAP=0` (or `RC_NOVSYNC=1`, which measures headroom) turns the limiter off whatever the limit; frame-exact
//! runs (`crate::determinism`) keep their own one tick per update, without the limiter unless `RC_FRAME_CAP=1`
//! (then at 60).

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{Monitor, PresentMode, PrimaryMonitor, PrimaryWindow};
use std::time::{Duration, Instant};

/// Ticks a frame may run at most (a longer stall's time is dropped).
pub const MAX_STEPS: u32 = 4;
/// The tick rate and period (1/60 s).
const TICK_HZ: f64 = crate::determinism::TICK_HZ;
const TICK: f64 = 1.0 / TICK_HZ;

/// The most frames drawn a second (the Port Options' "FPS limit" row, `rc_game::menus::pause::port::Setting::FrameLimit`).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameLimit {
    Fps30,
    #[default]
    Fps60,
    Fps120,
    Fps144,
    Unlimited,
}

impl FrameLimit {
    /// In the row's value order.
    pub const ALL: [FrameLimit; 5] = [FrameLimit::Fps30, FrameLimit::Fps60, FrameLimit::Fps120, FrameLimit::Fps144, FrameLimit::Unlimited];
    const KEY: &str = "fps_limit";

    pub fn index(self) -> u8 { Self::ALL.iter().position(|&l| l == self).unwrap_or(1) as u8 }
    pub fn from_index(v: u8) -> Self { Self::ALL.get(v as usize).copied().unwrap_or_default() }

    /// Frames a second (None: unlimited).
    pub fn hz(self) -> Option<f64> {
        match self {
            FrameLimit::Fps30 => Some(30.0),
            FrameLimit::Fps60 => Some(60.0),
            FrameLimit::Fps120 => Some(120.0),
            FrameLimit::Fps144 => Some(144.0),
            FrameLimit::Unlimited => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            FrameLimit::Fps30 => "30",
            FrameLimit::Fps60 => "60",
            FrameLimit::Fps120 => "120",
            FrameLimit::Fps144 => "144",
            FrameLimit::Unlimited => "unlimited",
        }
    }

    fn parse(s: &str) -> Option<Self> { Self::ALL.into_iter().find(|l| l.key().eq_ignore_ascii_case(s.trim())) }

    /// The start value: the port settings file's, else 60.
    pub fn startup() -> Self { crate::render_settings::load_key(Self::KEY).as_deref().and_then(Self::parse).unwrap_or_default() }

    /// Writes the setting into the port settings file.
    pub fn save(self) { crate::render_settings::save_key(Self::KEY, self.key()); }
}

pub struct FramePacePlugin;

#[derive(Resource)]
struct Pace {
    limit: bool,
    /// The last frame's start (the ticks' clock).
    last: Option<Instant>,
    /// Elapsed ticks not yet run.
    acc: f64,
    /// The limiter's next deadline and its period.
    next: Option<Instant>,
    period: Option<Duration>,
}

fn env_is(name: &str, v: &str) -> bool { std::env::var(name).is_ok_and(|x| x.trim() == v) }

impl Plugin for FramePacePlugin {
    fn build(&self, app: &mut App) {
        if crate::determinism::deterministic() {
            // Frame-exact runs keep their one tick per update; `RC_FRAME_CAP=1` still limits them to 60 frames a second
            // (a capture run otherwise renders as fast as the machine allows).
            app.insert_resource(FrameLimit::Fps60);
            if env_is("RC_FRAME_CAP", "1") {
                app.insert_resource(Pace { limit: true, last: None, acc: 0.0, next: None, period: None }).add_systems(Last, wait);
            }
            return;
        }
        let limit = !env_is("RC_FRAME_CAP", "0") && !env_is("RC_NOVSYNC", "1");
        let fps = FrameLimit::startup();
        if fps != FrameLimit::default() { println!("frame pace: FPS limit {} (settings file)", fps.key()); }
        app.insert_resource(fps)
            .insert_resource(Pace { limit, last: None, acc: 0.0, next: None, period: None })
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
            .add_systems(First, steps.before(bevy::time::TimeSystems));
        if limit { app.add_systems(Last, (present_mode, wait)); }
    }
}

/// This frame's ticks (module docs).
fn steps(mut pace: ResMut<Pace>, mut strategy: ResMut<TimeUpdateStrategy>, fps: Res<FrameLimit>) {
    let now = Instant::now();
    let Some(last) = pace.last.replace(now) else {
        *strategy = TimeUpdateStrategy::FixedTimesteps(1);
        return;
    };
    pace.acc += (now - last).as_secs_f64() / TICK;
    let round = pace.limit && fps.hz().is_some_and(|h| h <= TICK_HZ);
    let n = if round {
        let mut n = pace.acc.round().max(0.0);
        if n == 0.0 && pace.acc > 0.25 { n = 1.0; }
        n
    } else {
        pace.acc.floor()
    };
    let n = n.min(MAX_STEPS as f64);
    pace.acc -= n;
    pace.acc = if round { pace.acc.clamp(-0.5, 0.5) } else { pace.acc.clamp(0.0, 1.0) };
    *strategy = TimeUpdateStrategy::FixedTimesteps(n as u32);
}

/// The limiter's period for a limit of `hz` frames a second on a monitor of `mhz` millihertz (module docs): None at
/// ~`hz`.
fn period_for(mhz: Option<u32>, hz: f64) -> Option<Duration> {
    let own = Some(Duration::from_secs_f64(1.0 / hz));
    let Some(refresh) = mhz.map(|m| m as f64 / 1000.0).filter(|&h| h > 1.0) else { return own };
    let k = (refresh / hz).round();
    if k >= 1.0 && (refresh / k - hz).abs() < hz * 0.025 {
        if k == 1.0 { return None; }
        return Some(Duration::from_secs_f64(k / refresh));
    }
    own
}

/// The present mode for the limit: vsync, except when unlimited.
fn present_mode(fps: Res<FrameLimit>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if !fps.is_changed() { return; }
    let want = if *fps == FrameLimit::Unlimited { PresentMode::AutoNoVsync } else { PresentMode::Fifo };
    for mut w in &mut windows {
        if w.present_mode != want { w.present_mode = want; }
    }
}

/// The limiter (module docs).
fn wait(mut pace: ResMut<Pace>, fps: Res<FrameLimit>, monitors: Query<&Monitor, With<PrimaryMonitor>>) {
    let mhz = monitors.iter().next().and_then(|m| m.refresh_rate_millihertz);
    let period = fps.hz().and_then(|hz| period_for(mhz, hz));
    if period != pace.period {
        pace.period = period;
        pace.next = None;
    }
    let Some(period) = period else { return };
    let now = Instant::now();
    // A deadline more than a period behind (a stall) starts the rhythm again from now.
    let deadline = match pace.next {
        Some(d) if d + period > now => d,
        _ => now,
    };
    let spin = Duration::from_micros(300);
    if deadline > now + spin { std::thread::sleep(deadline - now - spin); }
    while Instant::now() < deadline { std::thread::yield_now(); }
    pace.next = Some(deadline + period);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_period_follows_the_display() {
        assert_eq!(period_for(Some(60_000), 60.0), None);
        assert_eq!(period_for(Some(59_940), 60.0), None);
        let p = |mhz, hz| period_for(Some(mhz), hz).unwrap().as_secs_f64();
        assert!((p(120_000, 60.0) - 2.0 / 120.0).abs() < 1e-9);
        assert!((p(119_880, 60.0) - 2.0 / 119.88).abs() < 1e-9);
        assert!((p(240_000, 60.0) - 4.0 / 240.0).abs() < 1e-9);
        assert!((p(144_000, 60.0) - TICK).abs() < 1e-9);
        assert!((period_for(None, 60.0).unwrap().as_secs_f64() - TICK).abs() < 1e-9);
    }

    #[test]
    fn the_period_follows_the_limit() {
        // 30 on a 60 Hz screen: every other refresh; 144 on a 144 Hz screen: vsync alone; 120 on 240 Hz: two refreshes.
        assert!((period_for(Some(60_000), 30.0).unwrap().as_secs_f64() - 2.0 / 60.0).abs() < 1e-9);
        assert_eq!(period_for(Some(144_000), 144.0), None);
        assert_eq!(period_for(Some(143_998), 144.0), None);
        assert!((period_for(Some(240_000), 120.0).unwrap().as_secs_f64() - 2.0 / 240.0).abs() < 1e-9);
        // No match: the limit's own period (144 on 60 Hz: vsync holds it to 60 anyway).
        assert!((period_for(Some(165_000), 144.0).unwrap().as_secs_f64() - 1.0 / 144.0).abs() < 1e-9);
        assert!((period_for(None, 120.0).unwrap().as_secs_f64() - 1.0 / 120.0).abs() < 1e-9);
    }

    #[test]
    fn limit_values() {
        assert_eq!(FrameLimit::default().index(), 1);
        for (k, l) in FrameLimit::ALL.into_iter().enumerate() {
            assert_eq!((l.index() as usize, FrameLimit::from_index(k as u8)), (k, l));
            assert_eq!(FrameLimit::parse(l.key()), Some(l));
        }
        assert_eq!(FrameLimit::parse(" Unlimited "), Some(FrameLimit::Unlimited));
        assert_eq!(FrameLimit::parse("90"), None);
        assert_eq!(FrameLimit::from_index(9), FrameLimit::Fps60);
        assert_eq!(FrameLimit::Unlimited.hz(), None);
    }
}
