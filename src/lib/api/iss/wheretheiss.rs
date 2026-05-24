//! wheretheiss.at provider — public, no-auth API returning current ISS
//! position, altitude, velocity, visibility, and visibility footprint.
//!
//! Endpoint:
//! - `https://api.wheretheiss.at/v1/satellites/25544` (25544 is the
//!   ISS's NORAD catalog id; the endpoint also accepts other satellites).
//!
//! "Overhead" is determined by comparing the user-to-ISS great-circle
//! distance to the ISS's instantaneous visibility footprint (the diameter,
//! in km, of the area where ISS sits above the horizon right now).

use super::model::IssState;
use crate::api::error::ApiError;
use crate::api::http::get_json;
use crate::api::iss::haversine_km;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct WhereTheIssConfig {
    /// User latitude (degrees, -90..90).
    pub user_lat: f64,
    /// User longitude (degrees, -180..180).
    pub user_lon: f64,
}

pub struct WhereTheIssProvider {
    user_lat: f64,
    user_lon: f64,
}

impl WhereTheIssProvider {
    pub fn new(cfg: WhereTheIssConfig) -> Result<Self, ApiError> {
        if !(-90.0..=90.0).contains(&cfg.user_lat) {
            return Err(ApiError::Config(format!(
                "iss: user_lat {} outside [-90, 90]",
                cfg.user_lat
            )));
        }
        if !(-180.0..=180.0).contains(&cfg.user_lon) {
            return Err(ApiError::Config(format!(
                "iss: user_lon {} outside [-180, 180]",
                cfg.user_lon
            )));
        }
        Ok(Self {
            user_lat: cfg.user_lat,
            user_lon: cfg.user_lon,
        })
    }

    pub async fn poll(&self) -> Result<IssState, ApiError> {
        let url = "https://api.wheretheiss.at/v1/satellites/25544";
        let raw: WhereTheIssRaw = get_json(url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "wheretheiss",
            msg: e.to_string(),
        })?;

        let ground_distance = haversine_km(self.user_lat, self.user_lon, raw.latitude, raw.longitude);
        // The ISS visibility footprint is the diameter (km) of the area where
        // the ISS is above the horizon. Overhead-for-the-user reduces to:
        // ground distance < radius (== diameter / 2).
        let overhead = ground_distance < raw.footprint / 2.0;

        Ok(IssState {
            ground_distance_km: ground_distance.round().max(0.0) as u32,
            overhead,
            lat: raw.latitude,
            lon: raw.longitude,
            altitude_km: raw.altitude,
            velocity_kms: raw.velocity / 3600.0, // API returns km/h; convert to km/s
            visibility: raw.visibility,
        })
    }
}

#[derive(Debug, Deserialize)]
struct WhereTheIssRaw {
    #[serde(default)]
    latitude: f64,
    #[serde(default)]
    longitude: f64,
    #[serde(default)]
    altitude: f64,
    /// API returns km/h here, despite the name "velocity".
    #[serde(default)]
    velocity: f64,
    /// Diameter (km) of the area where the ISS is above the horizon now.
    #[serde(default)]
    footprint: f64,
    #[serde(default)]
    visibility: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "name": "iss",
        "id": 25544,
        "latitude": 40.7128,
        "longitude": -74.0060,
        "altitude": 421.5,
        "velocity": 27580.0,
        "visibility": "daylight",
        "footprint": 4521.3,
        "timestamp": 1700000000,
        "daynum": 2459903.5,
        "solar_lat": 0.0,
        "solar_lon": 0.0,
        "units": "kilometers"
    }"#;

    #[test]
    fn parses_real_response_shape() {
        let raw: WhereTheIssRaw = serde_json::from_str(FIXTURE).expect("deserialize");
        assert!((raw.latitude - 40.7128).abs() < 1e-6);
        assert!((raw.altitude - 421.5).abs() < 1e-6);
        assert!((raw.footprint - 4521.3).abs() < 1e-6);
        assert_eq!(raw.visibility, "daylight");
    }

    #[test]
    fn overhead_when_user_under_iss() {
        // ISS directly over user → distance 0 → must be inside the footprint.
        let raw: WhereTheIssRaw = serde_json::from_str(FIXTURE).unwrap();
        let dist = haversine_km(40.7128, -74.0060, raw.latitude, raw.longitude);
        assert!(dist < raw.footprint / 2.0);
    }

    #[test]
    fn not_overhead_when_user_on_other_side_of_earth() {
        // ISS at NYC; user in Sydney (≈ 15,990 km away — far outside the ~2260 km radius).
        let raw: WhereTheIssRaw = serde_json::from_str(FIXTURE).unwrap();
        let dist = haversine_km(-33.8688, 151.2093, raw.latitude, raw.longitude);
        assert!(dist > raw.footprint / 2.0);
    }

    #[test]
    fn rejects_out_of_range_lat() {
        let err = WhereTheIssProvider::new(WhereTheIssConfig {
            user_lat: 91.0,
            user_lon: 0.0,
        });
        assert!(matches!(err, Err(ApiError::Config(_))));
    }

    #[test]
    fn velocity_converts_km_per_hour_to_km_per_second() {
        // 27580 km/h ≈ 7.66 km/s
        let kms = 27580.0_f64 / 3600.0;
        assert!((kms - 7.661).abs() < 0.01);
    }
}
