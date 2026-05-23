//! Sport collector — currently ESPN's free unofficial API, structured to take
//! more providers (e.g. league-official APIs) if rate limits become an issue.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod espn;
pub mod model;

pub use model::{
    GameStatus, HomeOrAway, NextGame, SportApiSource, SportData, SportKind, StandingsEntry,
    TeamSide,
};

use espn::{EspnConfig, EspnProvider};

/// Which provider this collector is using.
pub enum SportSource {
    Espn(EspnProvider),
}

impl SportSource {
    pub async fn poll(&self) -> Result<SportData, ApiError> {
        match self {
            Self::Espn(c) => c.poll().await,
        }
    }
}

pub struct SportCollector {
    source: SportSource,
    refresh: Duration,
}

impl SportCollector {
    pub fn new(source: SportSource) -> Self {
        Self {
            source,
            // Game state changes minute-by-minute when in progress; 60s gives
            // near-real-time updates without abusing the free endpoint.
            refresh: Duration::from_secs(60),
        }
    }

    pub fn from_espn(cfg: EspnConfig) -> Self {
        Self::new(SportSource::Espn(EspnProvider::new(cfg)))
    }
}

#[async_trait]
impl Collector for SportCollector {
    type Output = SportData;

    fn id(&self) -> &'static str {
        "sport"
    }

    fn refresh_interval(&self) -> Duration {
        self.refresh
    }

    async fn poll(&self) -> Result<SportData, ApiError> {
        self.source.poll().await
    }
}
