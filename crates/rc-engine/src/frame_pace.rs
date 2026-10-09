//! **Port-only: frame pacing.** The game runs at 60 Hz (one logic tick per NTSC field) and the port draws no frame
//! between two ticks (nothing is interpolated), so a frame drawn without a tick is the previous picture again, and a
//! frame that runs two ticks skips one: either shows as a stutter. This module keeps one tick per drawn frame and
//! draws at most 60 frames a second.
//!
//! * **Ticks** (`First`, before Bevy's time update): each frame advances `TimeUpdateStrategy::FixedTimesteps(n)`
//!   ticks, `n` from the real time since the last frame start counted in ticks with a remainder that carries over:
//!   while the limiter runs, `n` rounds (a frame a little early or late is still one tick; one that arrives
//!   more than half a tick early still gets its tick, the remainder kept within ±½), so the usual frame is exactly one
//!   tick and only a real hiccup catches up; without the limiter `n` is the plain floor of the elapsed ticks. At
//!   most [`MAX_STEPS`] a frame: time lost in a longer stall (a level load) is dropped.
//! * **Limiter** (`Last`): the frame waits for its deadline. The period follows the primary monitor: at ~60 Hz none
//!   (vsync paces the loop; a second clock beside it would beat against the display and drop a frame now and then);
//!   at a multiple of ~60 Hz (120 Hz ProMotion, 240 Hz) that many refreshes, so the frames stay on the display's own
//!   rhythm; otherwise 1/60 s (a 144 Hz screen without variable refresh still shows some frames longer than others).
//!   The wait sleeps to 0.3 ms before the deadline, then yields until it (the yield loop costs CPU: keep it short).
//!
//! `RC_FRAME_CAP=0` (or `RC_NOVSYNC=1`, which measures headroom) turns the limiter off; frame-exact runs
//! (`crate::determinism`) keep their own one tick per update, without the limiter unless `RC_FRAME_CAP=1`.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{Monitor, PrimaryMonitor};
use std::time::{Duration, Instant};

/// Ticks a frame may run at most (a longer stall's time is dropped).
pub const MAX_STEPS: u32 = 4;
/// The tick period (1/60 s).
const TICK: f64 = 1.0 / crate::determinism::TICK_HZ;

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
            if env_is("RC_FRAME_CAP", "1") {
                app.insert_resource(Pace { limit: true, last: None, acc: 0.0, next: None, period: None }).add_systems(Last, wait);
            }
            return;
        }
        let limit = !env_is("RC_FRAME_CAP", "0") && !env_is("RC_NOVSYNC", "1");
        app.insert_resource(Pace { limit, last: None, acc: 0.0, next: None, period: None })
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
            .add_systems(First, steps.before(bevy::time::TimeSystems));
        if limit { app.add_systems(Last, wait); }
    }
}

/// This frame's ticks (module docs).
fn steps(mut pace: ResMut<Pace>, mut strategy: ResMut<TimeUpdateStrategy>) {
    let now = Instant::now();
    let Some(last) = pace.last.replace(now) else {
        *strategy = TimeUpdateStrategy::FixedTimesteps(1);
        return;
    };
    pace.acc += (now - last).as_secs_f64() / TICK;
    let n = if pace.limit {
        let mut n = pace.acc.round().max(0.0);
        if n == 0.0 && pace.acc > 0.25 { n = 1.0; }
        n
    } else {
        pace.acc.floor()
    };
    let n = n.min(MAX_STEPS as f64);
    pace.acc -= n;
    pace.acc = if pace.limit { pace.acc.clamp(-0.5, 0.5) } else { pace.acc.clamp(0.0, 1.0) };
    *strategy = TimeUpdateStrategy::FixedTimesteps(n as u32);
}

/// The limiter's period for a monitor of `mhz` millihertz (module docs): None at ~60 Hz.
fn period_for(mhz: Option<u32>) -> Option<Duration> {
    let hz = mhz.map(|m| m as f64 / 1000.0).filter(|&h| h > 1.0);
    let Some(hz) = hz else { return Some(Duration::from_secs_f64(TICK)) };
    let k = (hz / 60.0).round();
    if k >= 1.0 && (hz / k - 60.0).abs() < 1.5 {
        if k == 1.0 { return None; }
        return Some(Duration::from_secs_f64(k / hz));
    }
    Some(Duration::from_secs_f64(TICK))
}

/// The limiter (module docs).
fn wait(mut pace: ResMut<Pace>, monitors: Query<&Monitor, With<PrimaryMonitor>>) {
    let period = period_for(monitors.iter().next().and_then(|m| m.refresh_rate_millihertz));
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
        assert_eq!(period_for(Some(60_000)), None);
        assert_eq!(period_for(Some(59_940)), None);
        let p = |mhz| period_for(Some(mhz)).unwrap().as_secs_f64();
        assert!((p(120_000) - 2.0 / 120.0).abs() < 1e-9);
        assert!((p(119_880) - 2.0 / 119.88).abs() < 1e-9);
        assert!((p(240_000) - 4.0 / 240.0).abs() < 1e-9);
        assert!((p(144_000) - TICK).abs() < 1e-9);
        assert!((period_for(None).unwrap().as_secs_f64() - TICK).abs() < 1e-9);
    }
}
