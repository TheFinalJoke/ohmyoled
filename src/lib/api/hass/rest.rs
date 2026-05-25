//! Home Assistant REST API provider — bearer-auth GET against a local
//! HASS instance.
//!
//! Endpoint:
//! - `GET <base_url>/api/states/<entity_id>`
//!   Header: `Authorization: Bearer <long_lived_access_token>`
//!
//! HASS's `state` field is always a string regardless of the underlying
//! entity type — numeric sensors report `"72.4"`, binary sensors
//! report `"on"`/`"off"`, etc. We pass it through verbatim and let
//! the renderer decide how to display it.

use super::model::HassEntity;
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct RestConfig {
    /// Base URL of the HASS instance, e.g. `http://homeassistant.local:8123`.
    /// No trailing slash — the collector appends `/api/states/...`.
    pub base_url: String,
    /// HASS long-lived access token (Profile → Security → Long-Lived
    /// Access Tokens).
    pub token: String,
    /// Entity id, e.g. `sensor.kitchen_temp`, `binary_sensor.garage_door`.
    pub entity_id: String,
    /// Optional display label override. When `None`, the renderer falls
    /// back to `attributes.friendly_name` (or the bare entity id if HASS
    /// has no friendly name either).
    pub label_override: Option<String>,
}

pub struct RestProvider {
    base_url: String,
    auth_header: String,
    entity_id: String,
    label_override: Option<String>,
}

impl RestProvider {
    pub fn new(cfg: RestConfig) -> Result<Self, ApiError> {
        if cfg.base_url.is_empty() {
            return Err(ApiError::Config("hass: base_url missing".into()));
        }
        if cfg.token.is_empty() {
            return Err(ApiError::Config("hass: token missing".into()));
        }
        if cfg.entity_id.is_empty() {
            return Err(ApiError::Config("hass: entity_id missing".into()));
        }
        Ok(Self {
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            auth_header: format!("Bearer {}", cfg.token),
            entity_id: cfg.entity_id,
            label_override: cfg.label_override.filter(|s| !s.is_empty()),
        })
    }

    pub async fn poll(&self) -> Result<HassEntity, ApiError> {
        let url = format!("{}/api/states/{}", self.base_url, self.entity_id);
        let raw: RawState = get_json(&url, &[("Authorization", self.auth_header.as_str())])
            .await
            .map_err(|e| ApiError::Provider {
                provider: "hass",
                msg: e.to_string(),
            })?;
        Ok(entity_from_raw(raw, self.label_override.as_deref(), &self.entity_id))
    }
}

/// Pure mapping from a parsed REST response to our `HassEntity`. Split
/// out so the test suite can exercise it without a live server.
fn entity_from_raw(raw: RawState, label_override: Option<&str>, entity_id: &str) -> HassEntity {
    let label = label_override
        .map(str::to_string)
        .or_else(|| {
            raw.attributes
                .as_ref()
                .and_then(|a| a.friendly_name.clone())
        })
        .unwrap_or_else(|| entity_id.to_string());
    let unit = raw
        .attributes
        .as_ref()
        .and_then(|a| a.unit_of_measurement.clone())
        .filter(|u| !u.is_empty());
    let last_changed = raw
        .last_changed
        .as_deref()
        .and_then(parse_iso)
        .unwrap_or_else(Utc::now);
    HassEntity {
        state: raw.state.unwrap_or_default(),
        unit,
        label,
        last_changed,
    }
}

fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[derive(Debug, Deserialize)]
struct RawState {
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    last_changed: Option<String>,
    #[serde(default)]
    attributes: Option<RawAttributes>,
}

#[derive(Debug, Deserialize)]
struct RawAttributes {
    #[serde(default)]
    unit_of_measurement: Option<String>,
    #[serde(default)]
    friendly_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const NUMERIC_FIXTURE: &str = r#"{
        "entity_id": "sensor.kitchen_temp",
        "state": "72.4",
        "attributes": {
            "unit_of_measurement": "°F",
            "friendly_name": "Kitchen Temperature",
            "device_class": "temperature"
        },
        "last_changed": "2026-05-25T03:42:11.000000+00:00",
        "last_updated": "2026-05-25T03:42:11.000000+00:00"
    }"#;

    const BINARY_FIXTURE: &str = r#"{
        "entity_id": "binary_sensor.garage_door",
        "state": "open",
        "attributes": {
            "friendly_name": "Garage Door",
            "device_class": "door"
        },
        "last_changed": "2026-05-25T03:30:00.000000+00:00"
    }"#;

    const SPARSE_FIXTURE: &str = r#"{
        "entity_id": "sensor.unknown_thing",
        "state": "unavailable"
    }"#;

    #[test]
    fn parses_numeric_entity() {
        let raw: RawState = serde_json::from_str(NUMERIC_FIXTURE).unwrap();
        let e = entity_from_raw(raw, None, "sensor.kitchen_temp");
        assert_eq!(e.state, "72.4");
        assert_eq!(e.unit.as_deref(), Some("°F"));
        assert_eq!(e.label, "Kitchen Temperature");
        assert!(e.is_numeric());
    }

    #[test]
    fn parses_binary_entity_with_no_unit() {
        let raw: RawState = serde_json::from_str(BINARY_FIXTURE).unwrap();
        let e = entity_from_raw(raw, None, "binary_sensor.garage_door");
        assert_eq!(e.state, "open");
        assert!(e.unit.is_none());
        assert!(!e.is_numeric());
    }

    #[test]
    fn label_override_wins_over_friendly_name() {
        let raw: RawState = serde_json::from_str(NUMERIC_FIXTURE).unwrap();
        let e = entity_from_raw(raw, Some("KITCHEN"), "sensor.kitchen_temp");
        assert_eq!(e.label, "KITCHEN");
    }

    #[test]
    fn label_falls_back_to_entity_id_when_nothing_else() {
        let raw: RawState = serde_json::from_str(SPARSE_FIXTURE).unwrap();
        let e = entity_from_raw(raw, None, "sensor.unknown_thing");
        assert_eq!(e.label, "sensor.unknown_thing");
        assert_eq!(e.state, "unavailable");
    }

    #[test]
    fn rest_provider_rejects_missing_config() {
        let make = |b: &str, t: &str, e: &str| RestConfig {
            base_url: b.into(),
            token: t.into(),
            entity_id: e.into(),
            label_override: None,
        };
        assert!(matches!(
            RestProvider::new(make("", "x", "sensor.y")),
            Err(ApiError::Config(_))
        ));
        assert!(matches!(
            RestProvider::new(make("http://x", "", "sensor.y")),
            Err(ApiError::Config(_))
        ));
        assert!(matches!(
            RestProvider::new(make("http://x", "y", "")),
            Err(ApiError::Config(_))
        ));
    }

    #[test]
    fn rest_provider_strips_trailing_slash_from_base_url() {
        let p = RestProvider::new(RestConfig {
            base_url: "http://hass.local:8123/".into(),
            token: "abc".into(),
            entity_id: "sensor.x".into(),
            label_override: None,
        })
        .unwrap();
        assert_eq!(p.base_url, "http://hass.local:8123");
    }

    #[test]
    fn empty_label_override_is_treated_as_none() {
        let p = RestProvider::new(RestConfig {
            base_url: "http://hass.local:8123".into(),
            token: "abc".into(),
            entity_id: "sensor.x".into(),
            label_override: Some("".into()),
        })
        .unwrap();
        assert!(p.label_override.is_none());
    }
}
