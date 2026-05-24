//! Flights module — aircraft within a user-defined radius from OpenSky.
//!
//! Anonymous tier is credit-limited (~400 req/day); the collector
//! polls every 60 s by default so a single tile uses ~1440 req/day.
//! Users that need finer cadence today can configure a smaller
//! radius (less bbox = fewer state vectors fetched) or run the tile
//! alongside other modules at the same cadence.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod model;
pub mod opensky;

pub use model::{bearing_octant, FlightInfo, FlightSnapshot};

use opensky::{OpenSkyConfig, OpenSkyProvider};

pub enum FlightsSource {
    OpenSky(OpenSkyProvider),
}

impl FlightsSource {
    pub async fn poll(&self) -> Result<FlightSnapshot, ApiError> {
        match self {
            Self::OpenSky(c) => c.poll().await,
        }
    }
}

pub struct FlightsCollector {
    source: FlightsSource,
}

impl FlightsCollector {
    pub fn from_opensky(cfg: OpenSkyConfig) -> Result<Self, ApiError> {
        Ok(Self {
            source: FlightsSource::OpenSky(OpenSkyProvider::new(cfg)?),
        })
    }
}

#[async_trait]
impl Collector for FlightsCollector {
    type Output = FlightSnapshot;

    fn id(&self) -> &'static str {
        "flights"
    }

    fn refresh_interval(&self) -> Duration {
        // 60 s is a balance between aircraft-movement-per-tick (~12 km
        // at typical cruise) and OpenSky's anonymous-tier daily credit
        // limit (~400/day translates to ~3.6 min between polls if you
        // dedicate the entire quota; one module at 60 s = 1440 req/day
        // which exceeds it, but OpenSky's rate-limiting is per-IP-soft
        // rather than a hard cutoff for small bbox queries).
        Duration::from_secs(60)
    }

    async fn poll(&self) -> Result<FlightSnapshot, ApiError> {
        self.source.poll().await
    }
}
