//! Launch Library 2 provider — public space-launch feed.
//!
//! Endpoint:
//! - `https://ll.thespacedevs.com/2.2.0/launch/upcoming/?limit=10`
//!
//! Anonymous rate limit is ~15 req/hr per API docs; the collector polls
//! every 30 minutes so a single tile uses ~2 req/hr.
//!
//! We ask for the next 10 launches and pick the first whose `net` is
//! actually in the future — LL2's "upcoming" endpoint occasionally
//! includes launches whose `net` has already passed (recently lifted
//! off, hasn't been moved into status `In Flight` / `Success` yet).
//! An optional `agency_filter` whitelist (case-insensitive substring
//! match against `provider.name`) lets users narrow to e.g. SpaceX-only.

use super::model::{LaunchStatus, UpcomingLaunch};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Default)]
pub struct LldevConfig {
    /// Case-insensitive substring whitelist applied to provider name.
    /// Empty = show everything.
    pub agency_filter: Vec<String>,
}

pub struct LldevProvider {
    agency_filter: Vec<String>,
}

impl LldevProvider {
    pub fn new(cfg: LldevConfig) -> Result<Self, ApiError> {
        Ok(Self {
            agency_filter: cfg
                .agency_filter
                .into_iter()
                .map(|s| s.to_lowercase())
                .collect(),
        })
    }

    pub async fn poll(&self) -> Result<UpcomingLaunch, ApiError> {
        // Pull 10 results so the now-filter has something to chew on
        // even if LL2's first entry has just lifted off.
        let url = "https://ll.thespacedevs.com/2.2.0/launch/upcoming/?limit=10";
        let raw: ListResponse = get_json(url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "lldev",
            msg: e.to_string(),
        })?;
        pick_next(raw, &self.agency_filter, Utc::now()).ok_or_else(|| ApiError::Provider {
            provider: "lldev",
            msg: "no future launches in the next 10 results".to_string(),
        })
    }
}

/// Pure pick logic, split for testing: walk the result list and return
/// the first entry whose `net` is in the future *and* (if `filter` is
/// non-empty) whose provider matches one of the filter substrings.
fn pick_next(
    raw: ListResponse,
    filter: &[String],
    now: DateTime<Utc>,
) -> Option<UpcomingLaunch> {
    for r in raw.results {
        let launch_at = r.net.as_deref().and_then(parse_iso)?;
        if launch_at <= now {
            continue;
        }
        let provider = r
            .launch_service_provider
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_default();
        if !filter.is_empty() {
            let p_lower = provider.to_lowercase();
            if !filter.iter().any(|sub| p_lower.contains(sub)) {
                continue;
            }
        }
        let vehicle = r
            .rocket
            .as_ref()
            .and_then(|r| r.configuration.as_ref())
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let mission = r
            .mission
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let status = r
            .status
            .as_ref()
            .map(|s| LaunchStatus::from_abbrev(&s.abbrev))
            .unwrap_or(LaunchStatus::Scheduled);
        let country_code = r
            .pad
            .as_ref()
            .and_then(|p| p.location.as_ref())
            .map(|l| l.country_code.clone())
            .unwrap_or_default();
        return Some(UpcomingLaunch {
            provider,
            vehicle,
            mission,
            launch_at,
            status,
            country_code,
        });
    }
    None
}

fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    #[serde(default)]
    results: Vec<RawLaunch>,
}

#[derive(Debug, Deserialize)]
struct RawLaunch {
    #[serde(default)]
    net: Option<String>,
    #[serde(default)]
    status: Option<RawStatus>,
    #[serde(default)]
    launch_service_provider: Option<RawProvider>,
    #[serde(default)]
    rocket: Option<RawRocket>,
    #[serde(default)]
    mission: Option<RawMission>,
    #[serde(default)]
    pad: Option<RawPad>,
}

#[derive(Debug, Deserialize)]
struct RawStatus {
    #[serde(default)]
    abbrev: String,
}

#[derive(Debug, Deserialize)]
struct RawProvider {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawRocket {
    #[serde(default)]
    configuration: Option<RawRocketConfig>,
}

#[derive(Debug, Deserialize)]
struct RawRocketConfig {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawMission {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawPad {
    #[serde(default)]
    location: Option<RawLocation>,
}

#[derive(Debug, Deserialize)]
struct RawLocation {
    #[serde(default)]
    country_code: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const FIXTURE: &str = r#"{
        "count": 200,
        "results": [
            {
                "id": "old-one",
                "name": "Falcon 9 | Past",
                "net": "2024-01-01T00:00:00Z",
                "status": {"abbrev": "Success"},
                "launch_service_provider": {"name": "SpaceX"},
                "rocket": {"configuration": {"name": "Falcon 9"}},
                "mission": {"name": "Past Mission"},
                "pad": {"location": {"country_code": "USA"}}
            },
            {
                "id": "next",
                "name": "Falcon 9 | Starlink 8-5",
                "net": "2030-06-01T14:23:00Z",
                "status": {"abbrev": "Go"},
                "launch_service_provider": {"name": "SpaceX"},
                "rocket": {"configuration": {"name": "Falcon 9 Block 5"}},
                "mission": {"name": "Starlink Group 8-5"},
                "pad": {"location": {"country_code": "USA"}}
            },
            {
                "id": "chinese",
                "name": "Long March | Tianhe",
                "net": "2030-07-15T03:00:00Z",
                "status": {"abbrev": "TBD"},
                "launch_service_provider": {"name": "China Aerospace"},
                "rocket": {"configuration": {"name": "Long March 5B"}},
                "mission": {"name": "Tianhe Module"},
                "pad": {"location": {"country_code": "CHN"}}
            }
        ]
    }"#;

    fn raw() -> ListResponse {
        serde_json::from_str(FIXTURE).expect("fixture parse")
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 25, 0, 0, 0).unwrap()
    }

    #[test]
    fn skips_past_net_returns_next_future_launch() {
        let next = pick_next(raw(), &[], now()).expect("future launch");
        assert_eq!(next.vehicle, "Falcon 9 Block 5");
        assert_eq!(next.provider, "SpaceX");
        assert_eq!(next.mission, "Starlink Group 8-5");
        assert_eq!(next.status, LaunchStatus::Go);
        assert_eq!(next.country_code, "USA");
    }

    #[test]
    fn agency_filter_narrows_results() {
        // Filter to "china" — should return the third entry, not the
        // first (which is SpaceX).
        let next = pick_next(raw(), &["china".to_string()], now()).expect("filtered launch");
        assert_eq!(next.provider, "China Aerospace");
        assert_eq!(next.country_code, "CHN");
    }

    #[test]
    fn filter_no_match_returns_none() {
        let none = pick_next(raw(), &["roscosmos".to_string()], now());
        assert!(none.is_none());
    }

    #[test]
    fn returns_none_when_all_results_are_past() {
        // All results in the past relative to `now`.
        let future_now = Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap();
        let none = pick_next(raw(), &[], future_now);
        assert!(none.is_none());
    }

    #[test]
    fn handles_missing_mission_and_provider_gracefully() {
        let json = r#"{
            "results": [{
                "id": "sparse",
                "net": "2030-01-01T00:00:00Z",
                "status": {"abbrev": "TBD"}
            }]
        }"#;
        let r: ListResponse = serde_json::from_str(json).unwrap();
        let next = pick_next(r, &[], now()).expect("sparse launch");
        assert_eq!(next.provider, "");
        assert_eq!(next.vehicle, "");
        assert_eq!(next.mission, "");
        assert_eq!(next.status, LaunchStatus::Scheduled);
    }
}
