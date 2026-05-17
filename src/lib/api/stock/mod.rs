//! Stock collector — currently Finnhub-only, structured to take more providers.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod finnhub;
pub mod model;

pub use model::{Direction, StockApiSource, StockQuote};

use finnhub::{FinnhubConfig, FinnhubProvider};

/// Which provider this collector is using.
pub enum StockSource {
    Finnhub(FinnhubProvider),
}

impl StockSource {
    pub async fn poll(&self) -> Result<StockQuote, ApiError> {
        match self {
            Self::Finnhub(c) => c.poll().await,
        }
    }
}

/// Top-level stock collector. Bridges [`Collector`] + the provider enum.
pub struct StockCollector {
    source: StockSource,
    refresh: Duration,
}

impl StockCollector {
    pub fn new(source: StockSource) -> Self {
        Self {
            source,
            // Stock data changes minute-to-minute during market hours; poll
            // every 30s. Finnhub free-tier limit is 60/min so we have headroom.
            refresh: Duration::from_secs(30),
        }
    }

    pub fn from_finnhub(cfg: FinnhubConfig) -> Result<Self, ApiError> {
        Ok(Self::new(StockSource::Finnhub(FinnhubProvider::new(cfg)?)))
    }
}

#[async_trait]
impl Collector for StockCollector {
    type Output = StockQuote;

    fn id(&self) -> &'static str {
        "stock"
    }

    fn refresh_interval(&self) -> Duration {
        self.refresh
    }

    async fn poll(&self) -> Result<StockQuote, ApiError> {
        self.source.poll().await
    }
}
