//! Pi-hole module — DNS blocking stats from a local Pi-hole instance.
//!
//! Only the legacy v5 admin API is implemented today. v6 changed the
//! auth model significantly (session-token POST → header on each
//! subsequent call) and is worth adding when it stops being volatile —
//! the `PiholeSource` enum leaves room for a `V6` variant alongside.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod model;
pub mod v5;

pub use model::PiholeSummary;

use v5::{V5Config, V5Provider};

pub enum PiholeSource {
    V5(V5Provider),
}

impl PiholeSource {
    pub async fn poll(&self) -> Result<PiholeSummary, ApiError> {
        match self {
            Self::V5(c) => c.poll().await,
        }
    }
}

pub struct PiholeCollector {
    source: PiholeSource,
}

impl PiholeCollector {
    pub fn from_v5(cfg: V5Config) -> Result<Self, ApiError> {
        Ok(Self {
            source: PiholeSource::V5(V5Provider::new(cfg)?),
        })
    }
}

#[async_trait]
impl Collector for PiholeCollector {
    type Output = PiholeSummary;

    fn id(&self) -> &'static str {
        "pihole"
    }

    fn refresh_interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn poll(&self) -> Result<PiholeSummary, ApiError> {
        self.source.poll().await
    }
}
