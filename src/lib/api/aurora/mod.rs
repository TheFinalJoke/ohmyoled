//! Aurora module — NOAA SWPC planetary Kp index, with an alert flag
//! when the threshold (configurable, default 5) is met or exceeded.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod model;
pub mod swpc;

pub use model::AuroraReading;

use swpc::{SwpcConfig, SwpcProvider};

pub enum AuroraSource {
    Swpc(SwpcProvider),
}

impl AuroraSource {
    pub async fn poll(&self) -> Result<AuroraReading, ApiError> {
        match self {
            Self::Swpc(c) => c.poll().await,
        }
    }
}

pub struct AuroraCollector {
    source: AuroraSource,
}

impl AuroraCollector {
    pub fn from_swpc(cfg: SwpcConfig) -> Result<Self, ApiError> {
        Ok(Self {
            source: AuroraSource::Swpc(SwpcProvider::new(cfg)?),
        })
    }
}

#[async_trait]
impl Collector for AuroraCollector {
    type Output = AuroraReading;

    fn id(&self) -> &'static str {
        "aurora"
    }

    fn refresh_interval(&self) -> Duration {
        // SWPC updates the 1-minute series every minute, but the Kp index
        // barely moves on minute scales — 5 minutes is plenty for a panel
        // tile and stays well clear of any conceivable rate limit.
        Duration::from_secs(300)
    }

    async fn poll(&self) -> Result<AuroraReading, ApiError> {
        self.source.poll().await
    }
}
