//! Home Assistant module — display any HASS entity's state.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod model;
pub mod rest;

pub use model::HassEntity;

use rest::{RestConfig, RestProvider};

pub enum HassSource {
    Rest(RestProvider),
}

impl HassSource {
    pub async fn poll(&self) -> Result<HassEntity, ApiError> {
        match self {
            Self::Rest(c) => c.poll().await,
        }
    }
}

pub struct HassCollector {
    source: HassSource,
}

impl HassCollector {
    pub fn from_rest(cfg: RestConfig) -> Result<Self, ApiError> {
        Ok(Self {
            source: HassSource::Rest(RestProvider::new(cfg)?),
        })
    }
}

#[async_trait]
impl Collector for HassCollector {
    type Output = HassEntity;

    fn id(&self) -> &'static str {
        "hass"
    }

    fn refresh_interval(&self) -> Duration {
        // 30 s matches typical HASS internal-entity tick rates. Local
        // network so rate limits aren't a concern; the tradeoff is "how
        // stale can the panel feel" — 30 s feels live for sensors and
        // is plenty quick for binary state changes.
        Duration::from_secs(30)
    }

    async fn poll(&self) -> Result<HassEntity, ApiError> {
        self.source.poll().await
    }
}
