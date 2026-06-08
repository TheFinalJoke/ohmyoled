//! USGS earthquake feed provider — free, public, no-auth GeoJSON.
//!
//! Endpoint family:
//! - `https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/<feed>.geojson`
//!
//! where `<feed>` is `significant_day | 4.5_day | 2.5_day | all_day`.
//!
//! The response is a standard GeoJSON FeatureCollection. We only care
//! about a few fields per feature: `properties.{mag, place, time}` and
//! `geometry.coordinates[2]` (depth in km — the third tuple element).
//! Sort by magnitude descending; take the top entry.

use super::model::{QuakeEvent, QuakeFeed, QuakeStatus};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct UsgsConfig {
    pub feed: QuakeFeed,
}

pub struct UsgsProvider {
    feed: QuakeFeed,
}

impl UsgsProvider {
    pub fn new(cfg: UsgsConfig) -> Result<Self, ApiError> {
        Ok(Self { feed: cfg.feed })
    }

    pub async fn poll(&self) -> Result<QuakeStatus, ApiError> {
        let url = format!(
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/{}.geojson",
            self.feed.slug()
        );
        let raw: FeatureCollection = get_json(&url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "usgs",
            msg: e.to_string(),
        })?;

        Ok(pick_top_event(raw))
    }
}

/// Pure: from a deserialized FeatureCollection, pick the top-magnitude event
/// (or `Quiet`). Split out so tests can drive it from a `FIXTURE`.
///
/// Non-earthquake events (quarry blasts, nuclear/chemical explosions, mining
/// induced, etc.) are filtered out — USGS catalogues these in the same feed
/// but they're noise for an "interesting seismic activity" tile. `type`
/// missing or empty is treated as `"earthquake"` (the historical default).
fn pick_top_event(raw: FeatureCollection) -> QuakeStatus {
    let top = raw
        .features
        .into_iter()
        .filter(|f| is_earthquake(f.properties.event_type.as_deref()))
        .filter_map(|f| {
            let mag = f.properties.mag?;
            // GeoJSON geometry is `[lon, lat, depth_km]`.
            let coords = f.geometry.as_ref().map(|g| &g.coordinates);
            let lon = coords.and_then(|c| c.first().copied()).unwrap_or(0.0) as f32;
            let lat = coords.and_then(|c| c.get(1).copied()).unwrap_or(0.0) as f32;
            let depth = coords.and_then(|c| c.get(2).copied()).unwrap_or(0.0) as f32;
            let origin = f.properties.time.and_then(parse_unix_ms)?;
            // Prefer the curated title; synthesize a minimal one if USGS
            // omitted it (older revisions of the feed sometimes do).
            let title = f.properties.title.unwrap_or_else(|| {
                let place = f.properties.place.unwrap_or_default();
                format!("M {:.1} - {}", mag, place)
            });
            Some(QuakeEvent {
                magnitude: mag as f32,
                title,
                origin,
                lat,
                lon,
                depth_km: depth,
                felt: f.properties.felt,
            })
        })
        .max_by(|a, b| {
            a.magnitude
                .partial_cmp(&b.magnitude)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    match top {
        Some(e) => QuakeStatus::Event(e),
        None => QuakeStatus::Quiet,
    }
}

fn is_earthquake(t: Option<&str>) -> bool {
    matches!(t, None | Some("") | Some("earthquake"))
}

fn parse_unix_ms(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

#[derive(Debug, Deserialize)]
struct FeatureCollection {
    #[serde(default)]
    features: Vec<Feature>,
}

#[derive(Debug, Deserialize)]
struct Feature {
    #[serde(default)]
    properties: Properties,
    #[serde(default)]
    geometry: Option<Geometry>,
}

#[derive(Debug, Default, Deserialize)]
struct Properties {
    #[serde(default)]
    mag: Option<f64>,
    #[serde(default)]
    place: Option<String>,
    /// Pre-formatted title from USGS, e.g. `"M 6.2 - OFF EAST COAST OF
    /// HONSHU, JAPAN"`. We prefer this over `place` because the curators
    /// tweak punctuation/casing per event.
    #[serde(default)]
    title: Option<String>,
    /// USGS expresses event time as milliseconds since the Unix epoch.
    #[serde(default)]
    time: Option<i64>,
    /// "Did You Feel It?" report count — populated only for events that have
    /// drawn community reports (often `null` for small / remote / offshore).
    #[serde(default)]
    felt: Option<u32>,
    /// Event type: usually `"earthquake"`, but USGS also publishes things
    /// like `"quarry blast"`, `"nuclear explosion"`, `"mine collapse"` here.
    /// Renamed because `type` is a Rust keyword.
    #[serde(default, rename = "type")]
    event_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Geometry {
    /// `[lon, lat, depth_km]`.
    #[serde(default)]
    coordinates: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "type": "FeatureCollection",
        "metadata": {"generated": 1700000000000, "title": "USGS Significant Earthquakes, Past Day", "count": 3},
        "features": [
            {
                "type": "Feature",
                "properties": {
                    "mag": 5.8,
                    "place": "73km SSE of Yelizovo, Russia",
                    "title": "M 5.8 - 73km SSE of Yelizovo, Russia",
                    "time": 1700000000000,
                    "type": "earthquake",
                    "felt": 12,
                    "url": "https://earthquake.usgs.gov/earthquakes/eventpage/usabcd"
                },
                "geometry": { "type": "Point", "coordinates": [157.42, 52.34, 24.5] },
                "id": "usabcd"
            },
            {
                "type": "Feature",
                "properties": {
                    "mag": 6.2,
                    "place": "OFF EAST COAST OF HONSHU, JAPAN",
                    "title": "M 6.2 - OFF EAST COAST OF HONSHU, JAPAN",
                    "time": 1700001000000,
                    "type": "earthquake",
                    "felt": 482
                },
                "geometry": { "type": "Point", "coordinates": [142.1, 38.3, 12.0] },
                "id": "usefgh"
            },
            {
                "type": "Feature",
                "properties": {
                    "mag": 9.9,
                    "place": "Mojave Desert, CA",
                    "title": "M 9.9 - Mojave Desert quarry blast",
                    "time": 1700002000000,
                    "type": "quarry blast"
                },
                "geometry": { "type": "Point", "coordinates": [-117.0, 35.0, 0.0] },
                "id": "ci7000"
            }
        ]
    }"#;

    const EMPTY_FIXTURE: &str = r#"{
        "type": "FeatureCollection",
        "metadata": {"generated": 1700000000000, "title": "USGS Significant Earthquakes, Past Day", "count": 0},
        "features": []
    }"#;

    #[test]
    fn picks_highest_magnitude_earthquake_skipping_blasts() {
        // The fixture's M9.9 quarry blast must NOT win — non-earthquake
        // event types are filtered out before the max-magnitude reduction.
        let raw: FeatureCollection = serde_json::from_str(FIXTURE).expect("deserialize");
        match pick_top_event(raw) {
            QuakeStatus::Event(e) => {
                assert!((e.magnitude - 6.2).abs() < 1e-3);
                assert!(e.title.contains("HONSHU"));
                assert!(e.title.starts_with("M 6.2"));
                assert!((e.depth_km - 12.0).abs() < 1e-3);
                assert_eq!(e.felt, Some(482));
            }
            QuakeStatus::Quiet => panic!("expected an event, got Quiet"),
        }
    }

    #[test]
    fn falls_back_to_synthesized_title_when_missing() {
        let json = r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": { "mag": 4.1, "place": "near Reno, NV", "time": 1700000000000 },
                "geometry": { "type": "Point", "coordinates": [0,0,7.0] }
            }]
        }"#;
        let raw: FeatureCollection = serde_json::from_str(json).expect("deserialize");
        match pick_top_event(raw) {
            QuakeStatus::Event(e) => {
                assert_eq!(e.title, "M 4.1 - near Reno, NV");
                assert_eq!(e.felt, None);
            }
            QuakeStatus::Quiet => panic!("expected an event"),
        }
    }

    #[test]
    fn type_filter_treats_unknown_and_empty_as_earthquake() {
        assert!(is_earthquake(None));
        assert!(is_earthquake(Some("")));
        assert!(is_earthquake(Some("earthquake")));
        assert!(!is_earthquake(Some("quarry blast")));
        assert!(!is_earthquake(Some("nuclear explosion")));
        assert!(!is_earthquake(Some("mine collapse")));
    }

    #[test]
    fn empty_feed_returns_quiet() {
        let raw: FeatureCollection = serde_json::from_str(EMPTY_FIXTURE).expect("deserialize");
        assert!(matches!(pick_top_event(raw), QuakeStatus::Quiet));
    }

    #[test]
    fn missing_magnitude_skips_feature() {
        let json = r#"{
            "type": "FeatureCollection",
            "features": [
                { "type": "Feature", "properties": { "place": "x", "time": 1 }, "geometry": null },
                { "type": "Feature", "properties": { "mag": 3.1, "place": "y", "time": 2 }, "geometry": { "type": "Point", "coordinates": [0,0,5.0] } }
            ]
        }"#;
        let raw: FeatureCollection = serde_json::from_str(json).expect("deserialize");
        match pick_top_event(raw) {
            QuakeStatus::Event(e) => {
                assert!(e.title.ends_with('y'), "title should reflect place 'y', got {:?}", e.title);
                assert!((e.magnitude - 3.1).abs() < 1e-3);
            }
            QuakeStatus::Quiet => panic!("expected the magnitude-3.1 entry"),
        }
    }

    #[test]
    fn feed_slugs_match_usgs_url_pattern() {
        assert_eq!(QuakeFeed::SignificantDay.slug(), "significant_day");
        assert_eq!(QuakeFeed::M45Day.slug(), "4.5_day");
        assert_eq!(QuakeFeed::M25Day.slug(), "2.5_day");
        assert_eq!(QuakeFeed::AllDay.slug(), "all_day");
    }
}
