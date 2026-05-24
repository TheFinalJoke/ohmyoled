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

use super::model::{StockApiSource, StockQuote};
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
