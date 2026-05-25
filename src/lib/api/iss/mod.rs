//! ISS tracker — distance from the user's location to the International
//! Space Station, plus an "overhead now" flag.
//!
//! Single provider today (wheretheiss.at); the enum shape leaves room for
//! adding NORAD/TLE-based predictors later.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod model;
pub mod wheretheiss;

pub use model::IssState;

use wheretheiss::{WhereTheIssConfig, WhereTheIssProvider};

pub enum IssSource {
    WhereTheIss(WhereTheIssProvider),
}

impl IssSource {
    pub async fn poll(&self) -> Result<IssState, ApiError> {
        match self {
            Self::WhereTheIss(c) => c.poll().await,
        }
    }
}

pub struct IssCollector {
    source: IssSource,
}

impl IssCollector {
    pub fn from_wheretheiss(cfg: WhereTheIssConfig) -> Result<Self, ApiError> {
        Ok(Self {
            source: IssSource::WhereTheIss(WhereTheIssProvider::new(cfg)?),
        })
    }
}

#[async_trait]
impl Collector for IssCollector {
    type Output = IssState;

    fn id(&self) -> &'static str {
        "iss"
    }

    fn refresh_interval(&self) -> Duration {
        // ISS travels ~7.66 km/s — a 30s sample buys us ~230 km of position
        // movement, fine for a "where is it right now" display without
        // hammering the (free, anonymous) API.
        Duration::from_secs(30)
    }

    async fn poll(&self) -> Result<IssState, ApiError> {
        self.source.poll().await
    }
}

/// Great-circle distance between two `(lat, lon)` points in km, mean Earth
/// radius. The classic Haversine — 10 lines of math, no geo crate.
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6371.0; // mean Earth radius (km)
    let to_rad = std::f64::consts::PI / 180.0;
    let (lat1r, lat2r) = (lat1 * to_rad, lat2 * to_rad);
    let dlat = (lat2 - lat1) * to_rad;
    let dlon = (lon2 - lon1) * to_rad;
    let a = (dlat / 2.0).sin().powi(2)
        + lat1r.cos() * lat2r.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    R * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haversine_known_distance() {
        // NYC (40.7128, -74.0060) to London (51.5074, -0.1278) ≈ 5570 km.
        let d = haversine_km(40.7128, -74.0060, 51.5074, -0.1278);
        assert!((d - 5570.0).abs() < 10.0, "got {d}");
    }

    #[test]
    fn haversine_same_point_is_zero() {
        let d = haversine_km(40.0, -74.0, 40.0, -74.0);
        assert!(d.abs() < 1e-6, "got {d}");
    }

    #[test]
    fn haversine_symmetry() {
        let a = haversine_km(0.0, 0.0, 10.0, 10.0);
        let b = haversine_km(10.0, 10.0, 0.0, 0.0);
        assert!((a - b).abs() < 1e-9);
    }
}
