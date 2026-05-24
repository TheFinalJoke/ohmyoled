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
fn pick_top_event(raw: FeatureCollection) -> QuakeStatus {
    let top = raw
        .features
        .into_iter()
        .filter_map(|f| {
            let mag = f.properties.mag?;
            let depth = f
                .geometry
                .as_ref()
                .and_then(|g| g.coordinates.get(2).copied())
                .unwrap_or(0.0) as f32;
            let origin = f.properties.time.and_then(parse_unix_ms)?;
            Some(QuakeEvent {
                magnitude: mag as f32,
                place: f.properties.place.unwrap_or_default(),
                origin,
                depth_km: depth,
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
    /// USGS expresses event time as milliseconds since the Unix epoch.
    #[serde(default)]
    time: Option<i64>,
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
        "metadata": {"generated": 1700000000000, "title": "USGS Significant Earthquakes, Past Day", "count": 2},
        "features": [
            {
                "type": "Feature",
                "properties": {
                    "mag": 5.8,
                    "place": "73km SSE of Yelizovo, Russia",
                    "time": 1700000000000,
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
                    "time": 1700001000000
                },
                "geometry": { "type": "Point", "coordinates": [142.1, 38.3, 12.0] },
                "id": "usefgh"
            }
        ]
    }"#;

    const EMPTY_FIXTURE: &str = r#"{
        "type": "FeatureCollection",
        "metadata": {"generated": 1700000000000, "title": "USGS Significant Earthquakes, Past Day", "count": 0},
        "features": []
    }"#;

    #[test]
    fn picks_highest_magnitude_event() {
        let raw: FeatureCollection = serde_json::from_str(FIXTURE).expect("deserialize");
        match pick_top_event(raw) {
            QuakeStatus::Event(e) => {
                assert!((e.magnitude - 6.2).abs() < 1e-3);
                assert!(e.place.contains("HONSHU"));
                assert!((e.depth_km - 12.0).abs() < 1e-3);
            }
            QuakeStatus::Quiet => panic!("expected an event, got Quiet"),
        }
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
                assert_eq!(e.place, "y");
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
