//! Pi-hole v5 admin-API provider.
//!
//! Endpoint:
//! - `GET <base_url>/admin/api.php?summaryRaw[&auth=<token>]`
//!
//! `summaryRaw` returns native numeric types (vs the bare `summary`
//! variant which returns formatted strings — `"34.2"` vs `34.2`).
//!
//! Auth on v5's `summary` endpoint is *optional*: many home installs
//! expose summary stats without a token. The `auth` query parameter
//! is the legacy app-password mechanism; supply it if your install
//! has the "block API access" toggle enabled.

use super::model::PiholeSummary;
use crate::api::error::ApiError;
use crate::api::http::get_json;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct V5Config {
    /// Pi-hole base URL, e.g. `http://pi.hole` or `http://192.168.1.2`.
    /// No trailing slash needed.
    pub base_url: String,
    /// Optional Pi-hole admin token (the SHA-256 hash from
    /// `Settings → API/Web interface → Show API token`). `None` works
    /// for installs that haven't restricted the summary endpoint.
    pub token: Option<String>,
}

pub struct V5Provider {
    base_url: String,
    token: Option<String>,
}

impl V5Provider {
    pub fn new(cfg: V5Config) -> Result<Self, ApiError> {
        if cfg.base_url.is_empty() {
            return Err(ApiError::Config("pihole: base_url missing".into()));
        }
        Ok(Self {
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            token: cfg.token.filter(|t| !t.is_empty()),
        })
    }

    pub async fn poll(&self) -> Result<PiholeSummary, ApiError> {
        let url = match &self.token {
            Some(t) => format!("{}/admin/api.php?summaryRaw&auth={}", self.base_url, t),
            None => format!("{}/admin/api.php?summaryRaw", self.base_url),
        };
        let raw: RawSummary = get_json(&url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "pihole-v5",
            msg: e.to_string(),
        })?;
        Ok(summary_from_raw(raw))
    }
}

fn summary_from_raw(raw: RawSummary) -> PiholeSummary {
    PiholeSummary {
        percent_blocked: raw.ads_percentage_today.clamp(0.0, 100.0),
        queries_today: raw.dns_queries_today,
        blocked_today: raw.ads_blocked_today,
        unique_clients: raw.unique_clients,
    }
}

#[derive(Debug, Deserialize)]
struct RawSummary {
    #[serde(default)]
    dns_queries_today: u32,
    #[serde(default)]
    ads_blocked_today: u32,
    #[serde(default)]
    ads_percentage_today: f32,
    #[serde(default)]
    unique_clients: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "domains_being_blocked": 134567,
        "dns_queries_today": 12348,
        "ads_blocked_today": 4221,
        "ads_percentage_today": 34.18,
        "unique_domains": 521,
        "queries_forwarded": 4012,
        "queries_cached": 4115,
        "clients_ever_seen": 18,
        "unique_clients": 12,
        "dns_queries_all_types": 12500,
        "reply_NODATA": 100,
        "reply_NXDOMAIN": 50,
        "reply_CNAME": 200,
        "reply_IP": 1000,
        "privacy_level": 0,
        "status": "enabled",
        "gravity_last_updated": {"file_exists": true, "absolute": 1700000000, "relative": {"days": 1, "hours": 2, "minutes": 30}}
    }"#;

    const QUIET_FIXTURE: &str = r#"{
        "dns_queries_today": 0,
        "ads_blocked_today": 0,
        "ads_percentage_today": 0.0,
        "unique_clients": 0
    }"#;

    #[test]
    fn parses_typical_v5_summary() {
        let raw: RawSummary = serde_json::from_str(FIXTURE).unwrap();
        let s = summary_from_raw(raw);
        assert_eq!(s.queries_today, 12_348);
        assert_eq!(s.blocked_today, 4_221);
        assert!((s.percent_blocked - 34.18).abs() < 0.01);
        assert_eq!(s.unique_clients, 12);
    }

    #[test]
    fn handles_zero_queries_quiet_install() {
        let raw: RawSummary = serde_json::from_str(QUIET_FIXTURE).unwrap();
        let s = summary_from_raw(raw);
        assert_eq!(s.queries_today, 0);
        assert_eq!(s.blocked_today, 0);
        assert_eq!(s.percent_blocked, 0.0);
    }

    #[test]
    fn clamps_percent_to_valid_range() {
        // Defensive — Pi-hole shouldn't emit out-of-range, but if it
        // does we don't want to feed -5% or 137% to the renderer.
        let mut raw: RawSummary = serde_json::from_str(QUIET_FIXTURE).unwrap();
        raw.ads_percentage_today = 137.0;
        assert_eq!(summary_from_raw(raw).percent_blocked, 100.0);
        let mut raw2: RawSummary = serde_json::from_str(QUIET_FIXTURE).unwrap();
        raw2.ads_percentage_today = -5.0;
        assert_eq!(summary_from_raw(raw2).percent_blocked, 0.0);
    }

    #[test]
    fn rejects_empty_base_url() {
        assert!(matches!(
            V5Provider::new(V5Config {
                base_url: "".into(),
                token: None,
            }),
            Err(ApiError::Config(_))
        ));
    }

    #[test]
    fn strips_trailing_slash_from_base_url() {
        let p = V5Provider::new(V5Config {
            base_url: "http://pi.hole/".into(),
            token: None,
        })
        .unwrap();
        assert_eq!(p.base_url, "http://pi.hole");
    }

    #[test]
    fn empty_token_string_treated_as_none() {
        let p = V5Provider::new(V5Config {
            base_url: "http://pi.hole".into(),
            token: Some("".into()),
        })
        .unwrap();
        assert!(p.token.is_none());
    }
}
