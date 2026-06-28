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
pub mod yahoo;

pub use model::{Direction, HistorySeries, StockApiSource, StockHistory, StockQuote, TradingSession};

use coingecko::{CoingeckoConfig, CoingeckoProvider};
use finnhub::{FinnhubConfig, FinnhubProvider};
use yahoo::{YahooConfig, YahooProvider};

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

/// Provider dispatch for the chart-mode (historical) collector.
/// Yahoo for equity tickers, CoinGecko for crypto — matches the
/// `api` setting on `StockSection`. Each provider's `poll_history`
/// returns the same normalized `StockHistory` shape.
pub enum HistorySource {
    Yahoo(YahooProvider),
    Coingecko(CoingeckoProvider),
}

impl HistorySource {
    pub async fn poll(&self) -> Result<StockHistory, ApiError> {
        match self {
            Self::Yahoo(p) => p.poll_history().await,
            Self::Coingecko(p) => p.poll_history().await,
        }
    }
}

/// Top-level collector for the chart tile. Same Module/Renderer
/// pattern as `StockCollector` — keeps the live and chart paths
/// independent so a flaky Yahoo response can't blank the live price.
pub struct StockHistoryCollector {
    source: HistorySource,
    refresh: Duration,
}

impl StockHistoryCollector {
    pub fn new(source: HistorySource) -> Self {
        // 5 minutes — Yahoo and CoinGecko both serve daily/weekly
        // candles that change at most once per minute (the 1D 5-min
        // bar). Three fetches per refresh × 5 min ≫ rate-limit ceiling
        // for both providers.
        Self::new_with_refresh(source, Duration::from_secs(300))
    }

    pub fn new_with_refresh(source: HistorySource, refresh: Duration) -> Self {
        Self { source, refresh }
    }

    pub fn from_yahoo(cfg: YahooConfig) -> Result<Self, ApiError> {
        Ok(Self::new(HistorySource::Yahoo(YahooProvider::new(cfg)?)))
    }

    pub fn from_coingecko(cfg: CoingeckoConfig) -> Result<Self, ApiError> {
        Ok(Self::new(HistorySource::Coingecko(CoingeckoProvider::new(cfg)?)))
    }
}

#[async_trait]
impl Collector for StockHistoryCollector {
    type Output = StockHistory;

    fn id(&self) -> &'static str {
        "stock_chart"
    }

    fn refresh_interval(&self) -> Duration {
        self.refresh
    }

    async fn poll(&self) -> Result<StockHistory, ApiError> {
        self.source.poll().await
    }
}
