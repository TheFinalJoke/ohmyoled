//! Sleep-mode tile for the config builder — the top-level `sleep` block.
//!
//! Unlike the module tiles, `sleep` is **target-independent and top-level**:
//! it never nests under `eink.modules`, there is at most one instance, and it
//! carries `enabled` instead of `run`. The assembly layer
//! (`tui::preview::sleep_value`) handles that placement; this module owns the
//! schema + the cron/window validation so a wizard-written schedule can't be
//! rejected by the daemon at startup (`SleepSchedule::from_config` applies the
//! same rules).
//!
//! The cron-anchored `windows` list (objects of `{at, for_mins}`) doesn't fit
//! the flat form engine, so the wizard doesn't edit it — a loaded config's
//! `windows` are preserved verbatim across a round-trip instead.

use crate::createjson::tui::field::{FieldDef, FieldKind};
use chrono::NaiveTime;
use croner::Cron;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// The top-level `sleep` block, mirroring `registry::SleepConfig`. `windows`
/// stays an opaque `Value` list: the wizard preserves it rather than editing it.
#[derive(Debug, Serialize, Deserialize)]
pub struct SleepOptions {
    pub enabled: bool,
    #[serde(default)]
    pub sleep: Option<String>,
    #[serde(default)]
    pub wake: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub windows: Vec<serde_json::Value>,
}

impl Default for SleepOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            sleep: Some("0 22 * * *".into()),
            wake: Some("0 7 * * *".into()),
            start: None,
            end: None,
            windows: Vec::new(),
        }
    }
}

/// Field schema. `id` MUST match the serde field name — the value is
/// round-tripped through [`SleepOptions`] on save, so a typo'd id is dropped.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "enabled",
            "Enabled",
            "Master switch — off keeps the schedule but never sleeps.",
            FieldKind::Bool { default: true },
        ),
        FieldDef::new(
            "sleep",
            "Sleep cron",
            "Cron that ENTERS sleep, e.g. '0 22 * * *' (pairs with wake; blank = unused).",
            FieldKind::OptionalText { default: "0 22 * * *" },
        ),
        FieldDef::new(
            "wake",
            "Wake cron",
            "Cron that WAKES, e.g. '0 7 * * *' (pairs with sleep cron).",
            FieldKind::OptionalText { default: "0 7 * * *" },
        ),
        FieldDef::new(
            "start",
            "Window start (HH:MM)",
            "Plain clock window start, e.g. '22:00' (pairs with end; wraps midnight).",
            FieldKind::OptionalText { default: "" },
        ),
        FieldDef::new(
            "end",
            "Window end (HH:MM)",
            "Plain clock window end, e.g. '07:00' (pairs with start).",
            FieldKind::OptionalText { default: "" },
        ),
    ]
}

/// A set optional string — `None`, blank, and the legacy `"null"` literal all
/// count as unset (matching `null_string_as_none` in the daemon's parser).
fn set_str(v: &Option<String>) -> Option<&str> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"))
}

/// Validate the schedule the same way `SleepSchedule::from_config` does:
/// `sleep`/`wake` must pair and parse as cron; `start`/`end` must pair and
/// parse as `HH:MM`. Returns `(field id, message)` errors for the form UI.
pub fn validate(opts: &SleepOptions) -> Vec<(String, String)> {
    let mut errs = Vec::new();

    let mut check_cron = |id: &str, expr: Option<&str>| {
        if let Some(e) = expr {
            if let Err(err) = Cron::from_str(e) {
                errs.push((id.to_string(), format!("invalid cron '{e}': {err}")));
            }
        }
    };
    let (sleep, wake) = (set_str(&opts.sleep), set_str(&opts.wake));
    check_cron("sleep", sleep);
    check_cron("wake", wake);
    if sleep.is_some() != wake.is_some() {
        errs.push((
            "sleep".to_string(),
            "sleep and wake crons must both be set (or both blank)".to_string(),
        ));
    }

    let mut check_time = |id: &str, t: Option<&str>| {
        if let Some(s) = t {
            if NaiveTime::parse_from_str(s, "%H:%M").is_err() {
                errs.push((id.to_string(), format!("'{s}' is not HH:MM")));
            }
        }
    };
    let (start, end) = (set_str(&opts.start), set_str(&opts.end));
    check_time("start", start);
    check_time("end", end);
    if start.is_some() != end.is_some() {
        errs.push((
            "start".to_string(),
            "start and end must both be set (or both blank)".to_string(),
        ));
    }

    errs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> SleepOptions {
        SleepOptions {
            enabled: true,
            sleep: None,
            wake: None,
            start: None,
            end: None,
            windows: Vec::new(),
        }
    }

    #[test]
    fn default_cron_pair_validates() {
        assert!(validate(&SleepOptions::default()).is_empty());
    }

    #[test]
    fn half_pairs_are_errors() {
        let half = SleepOptions { sleep: Some("0 22 * * *".into()), ..opts() };
        assert!(!validate(&half).is_empty());
        let half = SleepOptions { start: Some("22:00".into()), ..opts() };
        assert!(!validate(&half).is_empty());
    }

    #[test]
    fn malformed_cron_and_time_are_errors() {
        let bad = SleepOptions {
            sleep: Some("not a cron".into()),
            wake: Some("0 7 * * *".into()),
            ..opts()
        };
        assert!(validate(&bad).iter().any(|(id, _)| id == "sleep"));
        let bad = SleepOptions {
            start: Some("25:99".into()),
            end: Some("07:00".into()),
            ..opts()
        };
        assert!(validate(&bad).iter().any(|(id, _)| id == "start"));
    }

    #[test]
    fn null_literal_and_blank_count_as_unset() {
        let v = SleepOptions {
            sleep: Some("null".into()),
            wake: Some("".into()),
            ..opts()
        };
        assert!(validate(&v).is_empty(), "legacy 'null' strings are unset");
    }

    #[test]
    fn empty_schedule_is_valid() {
        // The daemon treats an all-blank schedule as a no-op, not an error.
        assert!(validate(&opts()).is_empty());
    }
}
