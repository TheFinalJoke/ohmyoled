//! NOAA SWPC planetary K-index provider — public, no-auth time series.
//!
//! Endpoint:
//! - `https://services.swpc.noaa.gov/json/planetary_k_index_1m.json`
//!
//! Returns the full day's 1-minute Kp samples as an array. We only care
//! about the most recent entry. The Kp index runs 0..=9; values ≥ 5
//! are minor geomagnetic-storm territory and the threshold above which
//! aurora becomes visible from mid-latitudes.

use super::model::AuroraReading;
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct SwpcConfig {
    /// Kp value at or above which `alert=true`. SWPC defines G1 (minor
    /// storm) at Kp ≥ 5; lower thresholds (3, 4) catch high-latitude
    /// viewers, higher ones (6+) are for "aurora visible far south".
    pub alert_threshold: u8,
}

pub struct SwpcProvider {
    alert_threshold: u8,
}

impl SwpcProvider {
    pub fn new(cfg: SwpcConfig) -> Result<Self, ApiError> {
        if !(1..=9).contains(&cfg.alert_threshold) {
            return Err(ApiError::Config(format!(
                "aurora: alert_threshold {} outside [1, 9]",
                cfg.alert_threshold
            )));
        }
        Ok(Self {
            alert_threshold: cfg.alert_threshold,
        })
    }

    pub async fn poll(&self) -> Result<AuroraReading, ApiError> {
        let url = "https://services.swpc.noaa.gov/json/planetary_k_index_1m.json";
        let series: Vec<SwpcEntry> = get_json(url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "swpc",
            msg: e.to_string(),
        })?;
        let last = series.into_iter().last().ok_or_else(|| ApiError::Provider {
            provider: "swpc",
            msg: "empty Kp series".to_string(),
        })?;
        Ok(reading_from_entry(last, self.alert_threshold))
    }
}

fn reading_from_entry(e: SwpcEntry, alert_threshold: u8) -> AuroraReading {
    let kp_index = e.kp_index;
    // Rounded display value — SWPC's `kp_index` can be fractional.
    let kp = kp_index.round().clamp(0.0, 9.0) as u8;
    AuroraReading {
        kp,
        kp_index,
        kp_text: e.kp,
        alert: kp >= alert_threshold,
        sampled_at: parse_swpc_time(&e.time_tag).unwrap_or_else(Utc::now),
    }
}

/// SWPC timestamps look like `"2026-05-24T18:33:00"` — no timezone, all
/// UTC by convention. Parse via the naive route then attach UTC.
fn parse_swpc_time(s: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|nd| Utc.from_utc_datetime(&nd))
}

#[derive(Debug, Deserialize)]
struct SwpcEntry {
    #[serde(default)]
    time_tag: String,
    /// SWPC sometimes serializes this as an integer, sometimes as a float
    /// (e.g. `0` vs `4.67`). Either deserializes into f32 cleanly.
    #[serde(default)]
    kp_index: f32,
    /// Quadrant-form: `"0Z"`, `"4-"`, `"3+"`, ... Always present.
    #[serde(default)]
    kp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"[
        {"time_tag": "2026-05-24T12:36:00", "kp_index": 0,    "estimated_kp": 0.0,  "kp": "0Z"},
        {"time_tag": "2026-05-24T18:00:00", "kp_index": 4.67, "estimated_kp": 5.0,  "kp": "4-"},
        {"time_tag": "2026-05-24T18:33:00", "kp_index": 6.2,  "estimated_kp": 6.33, "kp": "6+"}
    ]"#;

    #[test]
    fn picks_most_recent_entry() {
        let series: Vec<SwpcEntry> = serde_json::from_str(FIXTURE).unwrap();
        let last = series.into_iter().last().unwrap();
        let r = reading_from_entry(last, 5);
        assert_eq!(r.kp, 6); // 6.2 rounds to 6
        assert!((r.kp_index - 6.2).abs() < 1e-3);
        assert_eq!(r.kp_text, "6+");
        assert!(r.alert);
    }

    #[test]
    fn below_threshold_is_not_alert() {
        let series: Vec<SwpcEntry> = serde_json::from_str(FIXTURE).unwrap();
        // Use the middle entry — kp_index 4.67 → 5, equal to threshold 5.
        let middle = series.into_iter().nth(1).unwrap();
        let r = reading_from_entry(middle, 5);
        assert_eq!(r.kp, 5);
        assert!(r.alert, "kp==threshold should still alert");

        let series2: Vec<SwpcEntry> = serde_json::from_str(FIXTURE).unwrap();
        let middle2 = series2.into_iter().nth(1).unwrap();
        let r2 = reading_from_entry(middle2, 6);
        assert!(!r2.alert, "kp=5 should NOT alert with threshold=6");
    }

    #[test]
    fn parses_integer_kp_index_value() {
        // First fixture entry has kp_index: 0 (int). Must not blow up the
        // f32 deserializer.
        let series: Vec<SwpcEntry> = serde_json::from_str(FIXTURE).unwrap();
        let first = series.into_iter().next().unwrap();
        let r = reading_from_entry(first, 5);
        assert_eq!(r.kp, 0);
        assert_eq!(r.kp_text, "0Z");
        assert!(!r.alert);
    }

    #[test]
    fn parses_naive_utc_timestamp() {
        let t = parse_swpc_time("2026-05-24T18:33:00").unwrap();
        assert_eq!(t.timezone(), Utc);
        assert_eq!(t.format("%H:%M").to_string(), "18:33");
    }

    #[test]
    fn rejects_invalid_threshold() {
        for bad in [0u8, 10, 200] {
            let err = SwpcProvider::new(SwpcConfig { alert_threshold: bad });
            assert!(matches!(err, Err(ApiError::Config(_))), "threshold={bad} should reject");
        }
    }
}
