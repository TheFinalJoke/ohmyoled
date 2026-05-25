//! CoinGecko provider — crypto quotes that map into the same `StockQuote`
//! shape as Finnhub stocks, so `StockMatrix` can render them unchanged.
//!
//! Endpoint:
//! - `https://api.coingecko.com/api/v3/coins/<id>` → coin detail incl. 24h
//!   high/low/% change. Free tier, no API key.
//!
//! The config carries the **CoinGecko coin id** (e.g. `"bitcoin"`,
//! `"ethereum"`) in the `symbol` field — the renderer-facing symbol
//! (e.g. `"BTC"`) is populated from the API response.

use super::model::{HistorySeries, StockApiSource, StockHistory, StockQuote};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct CoingeckoConfig {
    /// CoinGecko coin id, e.g. `"bitcoin"`.
    pub coin_id: String,
}

pub struct CoingeckoProvider {
    coin_id: String,
}

impl CoingeckoProvider {
    pub fn new(cfg: CoingeckoConfig) -> Result<Self, ApiError> {
        if cfg.coin_id.is_empty() {
            return Err(ApiError::Config("coingecko: coin_id missing".to_string()));
        }
        Ok(Self {
            coin_id: cfg.coin_id.to_lowercase(),
        })
    }

    pub async fn poll(&self) -> Result<StockQuote, ApiError> {
        let url = format!(
            "https://api.coingecko.com/api/v3/coins/{}?localization=false&tickers=false&community_data=false&developer_data=false&sparkline=false&market_data=true",
            self.coin_id
        );

        let raw: CoinDetailRaw = get_json(&url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "coingecko",
            msg: e.to_string(),
        })?;

        let current = raw.market_data.current_price.usd;
        let high = raw.market_data.high_24h.usd;
        let low = raw.market_data.low_24h.usd;
        let pct = raw.market_data.price_change_percentage_24h.unwrap_or(0.0);
        let previous_close = derive_previous_close(current, pct);

        Ok(StockQuote {
            api: StockApiSource::Coingecko,
            symbol: raw.symbol.to_uppercase(),
            name: raw.name,
            open: previous_close,
            current,
            high,
            low,
            previous_close,
        })
    }

    /// Fetch the three chart windows (1d / 30d / 365d) from
    /// `/coins/{id}/market_chart`. The endpoint returns
    /// `[[ts_ms, price], …]` arrays — we pull just the price column.
    /// Each window is independent so a 404 on the year window doesn't
    /// blank the day window.
    pub async fn poll_history(&self) -> Result<StockHistory, ApiError> {
        let day = self.fetch_history(1).await;
        let month = self.fetch_history(30).await;
        let year = self.fetch_history(365).await;

        // `current` and `previous_close` derive from the 1-day window:
        // last sample is freshest, first sample is ~24h ago.
        let (current, previous_close, symbol) = match &day {
            Ok(p) => (p.last_close, p.first_close, p.symbol.clone()),
            Err(_) => (0.0, 0.0, self.coin_id.to_uppercase()),
        };

        if day.is_err() && month.is_err() && year.is_err() {
            return Err(day.err().unwrap());
        }

        Ok(StockHistory {
            api: StockApiSource::Coingecko,
            symbol,
            current,
            previous_close,
            day: day.map(|p| HistorySeries::from_closes(p.closes)).unwrap_or_else(|_| HistorySeries::from_closes(vec![])),
            month: month
                .map(|p| HistorySeries::from_closes(p.closes))
                .unwrap_or_else(|_| HistorySeries::from_closes(vec![])),
            year: year
                .map(|p| HistorySeries::from_closes(p.closes))
                .unwrap_or_else(|_| HistorySeries::from_closes(vec![])),
        })
    }

    async fn fetch_history(&self, days: u32) -> Result<ParsedHistory, ApiError> {
        let url = format!(
            "https://api.coingecko.com/api/v3/coins/{}/market_chart?vs_currency=usd&days={}",
            self.coin_id, days
        );
        let raw: MarketChartRaw = get_json(&url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "coingecko",
            msg: e.to_string(),
        })?;
        parse_market_chart(raw, &self.coin_id).ok_or_else(|| ApiError::Provider {
            provider: "coingecko",
            msg: format!("market_chart days={days} returned no usable prices"),
        })
    }
}

#[derive(Debug)]
struct ParsedHistory {
    closes: Vec<f64>,
    first_close: f64,
    last_close: f64,
    symbol: String,
}

/// Pure: pull the price column out of CoinGecko's `[[ts, price], …]`
/// array. Returns `None` for empty / null-only series. The display
/// symbol is the coin id uppercased — CoinGecko doesn't return a
/// short symbol on the market_chart endpoint, and we don't want to
/// pay a second `/coins/{id}` round-trip just for the label.
fn parse_market_chart(raw: MarketChartRaw, coin_id: &str) -> Option<ParsedHistory> {
    let closes: Vec<f64> = raw
        .prices
        .into_iter()
        .filter_map(|pair| pair.into_iter().nth(1))
        .filter(|v| v.is_finite())
        .collect();
    if closes.is_empty() {
        return None;
    }
    let first = *closes.first().unwrap();
    let last = *closes.last().unwrap();
    Some(ParsedHistory {
        closes,
        first_close: first,
        last_close: last,
        symbol: coin_id.to_uppercase(),
    })
}

#[derive(Debug, Deserialize)]
struct MarketChartRaw {
    #[serde(default)]
    prices: Vec<Vec<f64>>,
}

fn derive_previous_close(current: f64, pct_24h: f64) -> f64 {
    let denom = 1.0 + pct_24h / 100.0;
    if denom.abs() < f64::EPSILON {
        current
    } else {
        current / denom
    }
}

#[derive(Debug, Deserialize)]
struct CoinDetailRaw {
    #[serde(default)]
    symbol: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    market_data: MarketData,
}

#[derive(Debug, Default, Deserialize)]
struct MarketData {
    #[serde(default)]
    current_price: PriceMap,
    #[serde(default)]
    high_24h: PriceMap,
    #[serde(default)]
    low_24h: PriceMap,
    #[serde(default)]
    price_change_percentage_24h: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct PriceMap {
    #[serde(default)]
    usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "id": "bitcoin",
        "symbol": "btc",
        "name": "Bitcoin",
        "market_data": {
            "current_price": {"usd": 43521.0, "eur": 39800.0},
            "high_24h":      {"usd": 44100.0},
            "low_24h":       {"usd": 42950.0},
            "price_change_percentage_24h": 1.24
        }
    }"#;

    #[test]
    fn parses_real_response_shape() {
        let raw: CoinDetailRaw = serde_json::from_str(FIXTURE).expect("deserialize");
        assert_eq!(raw.symbol, "btc");
        assert_eq!(raw.name, "Bitcoin");
        assert_eq!(raw.market_data.current_price.usd, 43521.0);
        assert_eq!(raw.market_data.high_24h.usd, 44100.0);
        assert_eq!(raw.market_data.low_24h.usd, 42950.0);
        assert_eq!(raw.market_data.price_change_percentage_24h, Some(1.24));
    }

    #[test]
    fn derives_previous_close_from_pct() {
        // 43521 / 1.0124 ≈ 42987.949
        let pc = derive_previous_close(43521.0, 1.24);
        assert!((pc - 42987.9494).abs() < 0.01, "got {pc}");
    }

    #[test]
    fn derive_previous_close_guards_minus_100() {
        // pct == -100 would zero the denominator; we fall back to `current`.
        assert_eq!(derive_previous_close(50.0, -100.0), 50.0);
    }

    const HISTORY_FIXTURE: &str = r#"{
        "prices": [
            [1700000000000, 42950.10],
            [1700000300000, 43100.55],
            [1700000600000, 43521.00]
        ],
        "market_caps": [],
        "total_volumes": []
    }"#;

    #[test]
    fn parses_market_chart_into_history() {
        let raw: MarketChartRaw = serde_json::from_str(HISTORY_FIXTURE).expect("deserialize");
        let parsed = parse_market_chart(raw, "bitcoin").expect("history");
        assert_eq!(parsed.closes, vec![42950.10, 43100.55, 43521.00]);
        assert_eq!(parsed.first_close, 42950.10);
        assert_eq!(parsed.last_close, 43521.00);
        assert_eq!(parsed.symbol, "BITCOIN");
    }

    #[test]
    fn empty_prices_returns_none() {
        let raw: MarketChartRaw = serde_json::from_str(r#"{"prices":[]}"#).unwrap();
        assert!(parse_market_chart(raw, "bitcoin").is_none());
    }

    #[test]
    fn percent_change_round_trips_through_stock_quote() {
        let raw: CoinDetailRaw = serde_json::from_str(FIXTURE).unwrap();
        let current = raw.market_data.current_price.usd;
        let pct = raw.market_data.price_change_percentage_24h.unwrap();
        let prev = derive_previous_close(current, pct);
        let q = StockQuote {
            api: StockApiSource::Coingecko,
            symbol: raw.symbol.to_uppercase(),
            name: raw.name,
            open: prev,
            current,
            high: raw.market_data.high_24h.usd,
            low: raw.market_data.low_24h.usd,
            previous_close: prev,
        };
        // Should reproduce the input 24h % to within rounding.
        assert!(
            (q.percent_change() - pct).abs() < 1e-6,
            "round-trip pct mismatch: got {} expected {pct}",
            q.percent_change()
        );
    }
}
