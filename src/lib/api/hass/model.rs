//! Normalized Home Assistant entity reading.
//!
//! HASS's `state` field is *always a string*, regardless of the
//! underlying entity type — a number sensor reports `"72.4"`, a
//! binary sensor reports `"on"`/`"off"`, a door reports `"open"`/
//! `"closed"`. The renderer decides how to display it; this struct
//! just carries the raw string plus the relevant attributes.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct HassEntity {
    /// Raw state string as HASS reports it. `"72.4"`, `"on"`,
    /// `"unavailable"`, etc.
    pub state: String,
    /// `attributes.unit_of_measurement`, if any. `"°F"`, `"%"`, `"kWh"`.
    pub unit: Option<String>,
    /// Display label — caller-supplied override, or HASS's
    /// `attributes.friendly_name`, or the bare entity id when neither
    /// is available.
    pub label: String,
    /// When HASS most recently observed a state change. Used to render
    /// "updated 12s ago" on the footer.
    pub last_changed: DateTime<Utc>,
}

impl HassEntity {
    /// Whole-second age (rounded down) since the last state change.
    /// Capped at `u32::MAX`; for a fresh entity will be 0.
    pub fn age_seconds(&self, now: DateTime<Utc>) -> u32 {
        let secs = (now - self.last_changed).num_seconds().max(0);
        secs.try_into().unwrap_or(u32::MAX)
    }

    /// True when the state string looks numeric (parses as f64) —
    /// used by the renderer to decide between "right-align with unit"
    /// vs "center as text".
    pub fn is_numeric(&self) -> bool {
        self.state.trim().parse::<f64>().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(t: DateTime<Utc>) -> HassEntity {
        HassEntity {
            state: "72.4".into(),
            unit: Some("°F".into()),
            label: "Kitchen".into(),
            last_changed: t,
        }
    }

    #[test]
    fn age_seconds_floor_to_whole_seconds() {
        let t0 = Utc.with_ymd_and_hms(2026, 5, 25, 0, 0, 0).unwrap();
        let e = at(t0);
        assert_eq!(e.age_seconds(t0), 0);
        assert_eq!(e.age_seconds(t0 + chrono::Duration::seconds(45)), 45);
        assert_eq!(e.age_seconds(t0 + chrono::Duration::seconds(3_600)), 3_600);
    }

    #[test]
    fn age_seconds_clamps_negative_to_zero() {
        let t0 = Utc.with_ymd_and_hms(2026, 5, 25, 0, 0, 0).unwrap();
        // Clock went backwards (clock skew / NTP correction)
        assert_eq!(at(t0).age_seconds(t0 - chrono::Duration::minutes(5)), 0);
    }

    #[test]
    fn is_numeric_picks_up_floats_and_ints() {
        for s in ["72.4", "0", "-3", "1.5e2", "  18 "] {
            let e = HassEntity {
                state: s.into(),
                ..at(Utc::now())
            };
            assert!(e.is_numeric(), "expected `{s}` to count as numeric");
        }
    }

    #[test]
    fn is_numeric_rejects_text_states() {
        for s in ["on", "off", "open", "unavailable", "unknown", ""] {
            let e = HassEntity {
                state: s.into(),
                ..at(Utc::now())
            };
            assert!(!e.is_numeric(), "expected `{s}` to count as non-numeric");
        }
    }
}
