//! Earthquake module — top-magnitude event from the USGS feed.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod model;
pub mod usgs;

pub use model::{QuakeEvent, QuakeFeed, QuakeStatus};

use usgs::{UsgsConfig, UsgsProvider};

pub enum QuakeSource {
    Usgs(UsgsProvider),
}

impl QuakeSource {
    pub async fn poll(&self) -> Result<QuakeStatus, ApiError> {
        match self {
            Self::Usgs(c) => c.poll().await,
        }
    }
}

pub struct QuakeCollector {
    source: QuakeSource,
}

impl QuakeCollector {
    pub fn from_usgs(cfg: UsgsConfig) -> Result<Self, ApiError> {
        Ok(Self {
            source: QuakeSource::Usgs(UsgsProvider::new(cfg)?),
        })
    }
}

#[async_trait]
impl Collector for QuakeCollector {
    type Output = QuakeStatus;

    fn id(&self) -> &'static str {
        "quake"
    }

    fn refresh_interval(&self) -> Duration {
        // USGS regenerates the summary feeds every ~minute, but a 5-minute
        // poll is plenty for a tile that exists to flag "did something
        // interesting happen?" rather than tick second-by-second.
        Duration::from_secs(300)
    }

    async fn poll(&self) -> Result<QuakeStatus, ApiError> {
        self.source.poll().await
    }
}
