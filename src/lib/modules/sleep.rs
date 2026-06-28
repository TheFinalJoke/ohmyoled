//! Sleep mode — a schedule-driven global gate that blanks both panels and
//! pauses all API polling during configured "asleep" windows.
//!
//! ## How it fits together
//!
//! A single process-wide [`watch::channel<bool>`] is the gate (mirroring the
//! `SHUTDOWN`/`EINK_ACTIVE` atomics in [`scheduler`](crate::modules::scheduler),
//! but `watch` so consumers can *await* a wake transition rather than spin):
//!
//! - The per-module background poll loop ([`background_poll`](crate::modules))
//!   checks [`is_sleeping`] and parks on [`wait_until_awake`] instead of
//!   hitting the network.
//! - Both schedulers ([`scheduler::run`](crate::modules::scheduler::run) and
//!   [`run_eink`](crate::modules::scheduler::run_eink)) blank the panel once on
//!   entering sleep and park on [`wait_until_awake`] until the wake transition.
//!
//! A supervisor task re-evaluates the [`SleepSchedule`] every 30 s (cron is
//! minute-resolution) and publishes state changes. Evaluation is *stateless*
//! (`is_asleep(now)` recomputed from scratch), so it survives clock jumps from
//! NTP / a Docker host suspend without drifting.

use std::sync::OnceLock;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveTime};
use croner::Cron;
use std::str::FromStr;
use tokio::sync::watch;

use crate::modules::registry::{SleepConfig, SleepWindow};

/// Re-evaluation cadence for the supervisor. Cron resolves to the minute, so a
/// sub-minute tick is plenty to catch every transition promptly.
const TICK: Duration = Duration::from_secs(30);

/// Process-wide gate. `None` until [`init`] runs (sleep disabled / not
/// configured), in which case the daemon is always considered awake.
static GATE: OnceLock<watch::Sender<bool>> = OnceLock::new();

/// Whether the daemon is currently in a sleep window. `false` when sleep mode
/// is disabled or unconfigured.
pub fn is_sleeping() -> bool {
    GATE.get().map(|tx| *tx.borrow()).unwrap_or(false)
}

/// Await until the gate reports awake. Returns immediately when sleep mode is
/// disabled (no gate) or already awake; otherwise parks until the supervisor
/// flips the gate back to `false` (or the gate is torn down).
pub async fn wait_until_awake() {
    if let Some(tx) = GATE.get() {
        let mut rx = tx.subscribe();
        while *rx.borrow_and_update() {
            if rx.changed().await.is_err() {
                return; // sender gone — treat as awake so we never wedge.
            }
        }
    }
}

/// Parse the config into a [`SleepSchedule`], seed the gate with the current
/// state, and spawn the supervisor.
///
/// Call this **before** building the registries so the spawned poll tasks see
/// the gate immediately and don't fire a poll during a cold-start sleep window.
/// A malformed schedule logs an error and leaves sleep disabled (gate unset, so
/// the daemon behaves exactly as it did before).
pub fn init(cfg: &SleepConfig) {
    let schedule = match SleepSchedule::from_config(cfg) {
        Ok(s) => s,
        Err(e) => {
            log::error!("sleep: invalid schedule, sleep mode disabled: {e}");
            return;
        }
    };
    if schedule.is_empty() {
        log::warn!("sleep: enabled but no schedule shapes configured; sleep mode is a no-op");
        return;
    }
    let initial = schedule.is_asleep(Local::now());
    let (tx, _rx) = watch::channel(initial);
    if GATE.set(tx).is_err() {
        log::warn!("sleep: already initialized; ignoring duplicate init");
        return;
    }
    log::info!(
        "sleep mode enabled (currently {})",
        if initial { "asleep — panels off" } else { "awake" }
    );
    tokio::spawn(supervisor(schedule));
}

/// Re-evaluate the schedule on a fixed cadence and publish transitions.
async fn supervisor(schedule: SleepSchedule) {
    loop {
        tokio::time::sleep(TICK).await;
        let asleep = schedule.is_asleep(Local::now());
        if let Some(tx) = GATE.get() {
            if *tx.borrow() != asleep {
                log::info!(
                    "sleep mode: {}",
                    if asleep { "entering sleep — panels off" } else { "waking" }
                );
                let _ = tx.send(asleep);
            }
        }
    }
}

/// A parsed sleep schedule. Built once from [`SleepConfig`]; [`is_asleep`] is
/// pure and cheap so the supervisor can call it on every tick.
///
/// [`is_asleep`]: SleepSchedule::is_asleep
pub struct SleepSchedule {
    /// `(enter, wake)` cron pair — asleep while the next event is a wake.
    cron_pair: Option<(Cron, Cron)>,
    /// `[start, end)` clock window, wrapping past midnight when `start > end`.
    clock_window: Option<(NaiveTime, NaiveTime)>,
    /// Cron-anchored windows: asleep for `for_mins` after each firing.
    windows: Vec<(Cron, chrono::Duration)>,
}

impl SleepSchedule {
    /// Parse and validate every configured shape. Returns an error (and the
    /// caller disables sleep) if any cron / time string is malformed.
    pub fn from_config(cfg: &SleepConfig) -> Result<Self, String> {
        let parse_cron = |label: &str, expr: &str| -> Result<Cron, String> {
            Cron::from_str(expr).map_err(|e| format!("{label} cron '{expr}': {e}"))
        };

        let cron_pair = match (&cfg.sleep, &cfg.wake) {
            (Some(s), Some(w)) => Some((parse_cron("sleep", s)?, parse_cron("wake", w)?)),
            (Some(_), None) | (None, Some(_)) => {
                return Err("`sleep` and `wake` must both be set (cron pair)".to_string());
            }
            (None, None) => None,
        };

        let parse_time = |label: &str, s: &str| -> Result<NaiveTime, String> {
            NaiveTime::parse_from_str(s, "%H:%M")
                .map_err(|e| format!("{label} time '{s}' (expected HH:MM): {e}"))
        };
        let clock_window = match (&cfg.start, &cfg.end) {
            (Some(s), Some(e)) => Some((parse_time("start", s)?, parse_time("end", e)?)),
            (Some(_), None) | (None, Some(_)) => {
                return Err("`start` and `end` must both be set (HH:MM window)".to_string());
            }
            (None, None) => None,
        };

        let windows = cfg
            .windows
            .iter()
            .map(|w: &SleepWindow| {
                let cron = parse_cron("window", &w.at)?;
                Ok((cron, chrono::Duration::minutes(w.for_mins as i64)))
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(Self { cron_pair, clock_window, windows })
    }

    /// True when nothing is configured (sleep mode would be a no-op).
    pub fn is_empty(&self) -> bool {
        self.cron_pair.is_none() && self.clock_window.is_none() && self.windows.is_empty()
    }

    /// Whether `now` falls inside any configured sleep window.
    pub fn is_asleep(&self, now: DateTime<Local>) -> bool {
        self.cron_pair_asleep(now) || self.clock_window_asleep(now) || self.windows_asleep(now)
    }

    /// Cron pair: the next event being a *wake* means we're inside a sleep
    /// period. A cron error (no occurrence in croner's search horizon) is
    /// treated as "not asleep" so a flaky expression can't pin the panel off.
    fn cron_pair_asleep(&self, now: DateTime<Local>) -> bool {
        let Some((enter, wake)) = &self.cron_pair else { return false };
        match (
            enter.find_next_occurrence(&now, false),
            wake.find_next_occurrence(&now, false),
        ) {
            (Ok(next_enter), Ok(next_wake)) => next_wake < next_enter,
            _ => false,
        }
    }

    /// HH:MM window, with overnight wrap when `start > end`.
    fn clock_window_asleep(&self, now: DateTime<Local>) -> bool {
        let Some((start, end)) = &self.clock_window else { return false };
        let t = now.time();
        if start <= end {
            t >= *start && t < *end
        } else {
            // Wraps midnight, e.g. 22:00..07:00.
            t >= *start || t < *end
        }
    }

    /// Cron windows: asleep if the most recent firing is still within its
    /// `for_mins` span.
    fn windows_asleep(&self, now: DateTime<Local>) -> bool {
        self.windows.iter().any(|(cron, dur)| {
            match cron.find_previous_occurrence(&now, true) {
                Ok(last) => now < last + *dur,
                Err(_) => false,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use crate::modules::registry::SleepConfig;

    fn at(h: u32, m: u32) -> DateTime<Local> {
        // 2026-06-15 is a Monday; fixed so tests never touch the wall clock.
        Local.with_ymd_and_hms(2026, 6, 15, h, m, 0).unwrap()
    }

    fn cfg() -> SleepConfig {
        SleepConfig::default()
    }

    #[test]
    fn overnight_clock_window_wraps_midnight() {
        let s = SleepSchedule::from_config(&SleepConfig {
            enabled: true,
            start: Some("22:00".into()),
            end: Some("07:00".into()),
            ..cfg()
        })
        .unwrap();
        assert!(s.is_asleep(at(23, 0)), "23:00 is inside 22:00-07:00");
        assert!(s.is_asleep(at(2, 30)), "02:30 is inside the wrapped window");
        assert!(s.is_asleep(at(22, 0)), "start is inclusive");
        assert!(!s.is_asleep(at(7, 0)), "end is exclusive");
        assert!(!s.is_asleep(at(12, 0)), "noon is awake");
    }

    #[test]
    fn same_day_clock_window() {
        let s = SleepSchedule::from_config(&SleepConfig {
            enabled: true,
            start: Some("13:00".into()),
            end: Some("14:00".into()),
            ..cfg()
        })
        .unwrap();
        assert!(s.is_asleep(at(13, 30)));
        assert!(!s.is_asleep(at(12, 59)));
        assert!(!s.is_asleep(at(14, 0)));
    }

    #[test]
    fn cron_pair_across_boundary() {
        let s = SleepSchedule::from_config(&SleepConfig {
            enabled: true,
            sleep: Some("0 22 * * *".into()),
            wake: Some("0 7 * * *".into()),
            ..cfg()
        })
        .unwrap();
        // At 23:00 the next event is the 07:00 wake → asleep.
        assert!(s.is_asleep(at(23, 0)));
        assert!(s.is_asleep(at(3, 0)));
        // At 12:00 the next event is the 22:00 sleep → awake.
        assert!(!s.is_asleep(at(12, 0)));
        assert!(!s.is_asleep(at(7, 30)));
    }

    #[test]
    fn cron_window_duration() {
        let s = SleepSchedule::from_config(&SleepConfig {
            enabled: true,
            windows: vec![SleepWindow { at: "0 13 * * *".into(), for_mins: 60 }],
            ..cfg()
        })
        .unwrap();
        assert!(s.is_asleep(at(13, 0)), "fires at 13:00");
        assert!(s.is_asleep(at(13, 59)), "still within the 60-min window");
        assert!(!s.is_asleep(at(14, 0)), "window has elapsed");
        assert!(!s.is_asleep(at(12, 59)), "before the window");
    }

    #[test]
    fn shapes_are_ored_together() {
        let s = SleepSchedule::from_config(&SleepConfig {
            enabled: true,
            start: Some("22:00".into()),
            end: Some("07:00".into()),
            windows: vec![SleepWindow { at: "0 13 * * *".into(), for_mins: 30 }],
            ..cfg()
        })
        .unwrap();
        assert!(s.is_asleep(at(23, 0)), "matched by clock window");
        assert!(s.is_asleep(at(13, 15)), "matched by cron window");
        assert!(!s.is_asleep(at(15, 0)), "matched by neither");
    }

    #[test]
    fn empty_schedule_is_never_asleep() {
        let s = SleepSchedule::from_config(&SleepConfig { enabled: true, ..cfg() }).unwrap();
        assert!(s.is_empty());
        assert!(!s.is_asleep(at(3, 0)));
    }

    #[test]
    fn half_configured_pair_is_an_error() {
        let r = SleepSchedule::from_config(&SleepConfig {
            enabled: true,
            sleep: Some("0 22 * * *".into()),
            ..cfg()
        });
        assert!(r.is_err());
    }

    #[test]
    fn malformed_cron_is_an_error() {
        let r = SleepSchedule::from_config(&SleepConfig {
            enabled: true,
            sleep: Some("not a cron".into()),
            wake: Some("0 7 * * *".into()),
            ..cfg()
        });
        assert!(r.is_err());
    }
}
