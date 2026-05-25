//! Normalized launch data — what the collector hands to the renderer.
//!
//! LL2's `status.abbrev` field uses a small fixed vocabulary; we
//! normalize it into our own enum so the renderer's match arms stay
//! exhaustive even if LL2 adds new states.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct UpcomingLaunch {
    /// Launch service provider name, e.g. `"SpaceX"`,
    /// `"China Aerospace Science and Technology Corporation"`. The
    /// renderer truncates to fit; we keep the full string here.
    pub provider: String,
    /// Rocket vehicle name, e.g. `"Falcon 9 Block 5"`, `"Long March 2F/G"`.
    /// Sourced from `rocket.configuration.name`.
    pub vehicle: String,
    /// Mission name, e.g. `"Starlink Group 8-5"`. Empty when LL2 hasn't
    /// populated the mission record yet (sometimes the case weeks out).
    pub mission: String,
    /// "No Earlier Than" launch time — the official scheduled liftoff.
    /// May change as the launch approaches; the collector re-polls every
    /// 30 minutes to keep this fresh enough for a countdown display.
    pub launch_at: DateTime<Utc>,
    /// Coarse launch status from LL2's `status.abbrev`.
    pub status: LaunchStatus,
    /// `pad.location.country_code` — `"USA"`, `"CHN"`, etc. Useful for
    /// callers that want to flag domestic vs. international launches.
    pub country_code: String,
}

/// Coarse launch status. LL2 emits more states (TBC, Partial Failure,
/// On Hold variants) but for an at-a-glance tile we collapse them into
/// these six.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchStatus {
    /// "Go for Launch" — countdown is active.
    Go,
    /// Scheduled but not yet committed ("TBD" / "TBC").
    Scheduled,
    /// On hold / scrubbed for this attempt.
    Hold,
    /// Currently lifting off ("In Flight").
    InFlight,
    /// Past launch — succeeded.
    Success,
    /// Past launch — failed (partial or total).
    Failure,
}

impl LaunchStatus {
    /// Map an LL2 `status.abbrev` to our enum. Unknown values land on
    /// `Scheduled` — best guess for a future record.
    pub fn from_abbrev(abbrev: &str) -> Self {
        match abbrev {
            "Go" => Self::Go,
            "TBD" | "TBC" => Self::Scheduled,
            "Hold" => Self::Hold,
            "In Flight" => Self::InFlight,
            "Success" => Self::Success,
            "Failure" | "Partial Failure" => Self::Failure,
            _ => Self::Scheduled,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Success | Self::Failure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_abbrev_mapping_covers_common_values() {
        assert_eq!(LaunchStatus::from_abbrev("Go"), LaunchStatus::Go);
        assert_eq!(LaunchStatus::from_abbrev("TBD"), LaunchStatus::Scheduled);
        assert_eq!(LaunchStatus::from_abbrev("TBC"), LaunchStatus::Scheduled);
        assert_eq!(LaunchStatus::from_abbrev("Hold"), LaunchStatus::Hold);
        assert_eq!(LaunchStatus::from_abbrev("In Flight"), LaunchStatus::InFlight);
        assert_eq!(LaunchStatus::from_abbrev("Success"), LaunchStatus::Success);
        assert_eq!(LaunchStatus::from_abbrev("Failure"), LaunchStatus::Failure);
        assert_eq!(LaunchStatus::from_abbrev("Partial Failure"), LaunchStatus::Failure);
    }

    #[test]
    fn unknown_status_defaults_to_scheduled() {
        assert_eq!(LaunchStatus::from_abbrev(""), LaunchStatus::Scheduled);
        assert_eq!(LaunchStatus::from_abbrev("Glorp"), LaunchStatus::Scheduled);
    }

    #[test]
    fn is_terminal_only_past_outcomes() {
        assert!(LaunchStatus::Success.is_terminal());
        assert!(LaunchStatus::Failure.is_terminal());
        assert!(!LaunchStatus::Go.is_terminal());
        assert!(!LaunchStatus::Scheduled.is_terminal());
        assert!(!LaunchStatus::Hold.is_terminal());
        assert!(!LaunchStatus::InFlight.is_terminal());
    }
}
