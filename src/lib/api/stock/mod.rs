//! Stock collector — Finnhub for equities and CoinGecko for crypto, both
//! normalized to the same [`StockQuote`] so the renderer doesn't care which
//! one produced the data.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod coingecko;
pub mod finnhub;
pub mod model;

pub use model::{Direction, StockApiSource, StockQuote};

use coingecko::{CoingeckoConfig, CoingeckoProvider};
use finnhub::{FinnhubConfig, FinnhubProvider};

/// Which provider this collector is using.
pub enum StockSource {
    Finnhub(FinnhubProvider),
    Coingecko(CoingeckoProvider),
}

impl StockSource {
    pub async fn poll(&self) -> Result<StockQuote, ApiError> {
        match self {
            Self::Finnhub(c) => c.poll().await,
            Self::Coingecko(c) => c.poll().await,
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
        // Default: market-hours cadence. Finnhub free tier is 60/min; 30s
        // per symbol stays comfortably under that.
        Self::new_with_refresh(source, Duration::from_secs(30))
    }

    pub fn new_with_refresh(source: StockSource, refresh: Duration) -> Self {
        Self { source, refresh }
    }

    pub fn from_finnhub(cfg: FinnhubConfig) -> Result<Self, ApiError> {
        Ok(Self::new(StockSource::Finnhub(FinnhubProvider::new(cfg)?)))
    }

    pub fn from_coingecko(cfg: CoingeckoConfig) -> Result<Self, ApiError> {
        // CoinGecko's public tier rate-limits ~5–15 req/min — give each
        // coin its own 60s budget so a few entries don't trip the limit.
        Ok(Self::new_with_refresh(
            StockSource::Coingecko(CoingeckoProvider::new(cfg)?),
            Duration::from_secs(60),
        ))
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
