//! Finnhub provider — wraps the [`finnhub`](https://crates.io/crates/finnhub)
//! crate (reqwest 0.12, async, 96% endpoint coverage).
//!
//! Fans out `stock().quote(symbol)` + `stock().company_profile(symbol)` in
//! parallel via `tokio::join!`, mirroring the old Python `SQuote.run()` flow.

use super::model::{StockApiSource, StockQuote};
use crate::api::error::ApiError;
use finnhub::FinnhubClient;

#[derive(Debug, Clone)]
pub struct FinnhubConfig {
    pub api_key: String,
    pub symbol: String,
}

pub struct FinnhubProvider {
    client: FinnhubClient,
    symbol: String,
}

impl FinnhubProvider {
    pub fn new(cfg: FinnhubConfig) -> Result<Self, ApiError> {
        if cfg.api_key.is_empty() {
            return Err(ApiError::Config("finnhub: api_key missing".to_string()));
        }
        if cfg.symbol.is_empty() {
            return Err(ApiError::Config("finnhub: symbol missing".to_string()));
        }
        Ok(Self {
            client: FinnhubClient::new(cfg.api_key),
            symbol: cfg.symbol.to_uppercase(),
        })
    }

    pub async fn poll(&self) -> Result<StockQuote, ApiError> {
        let stock = self.client.stock();
        let (quote_res, profile_res) =
            tokio::join!(stock.quote(&self.symbol), stock.company_profile(&self.symbol));

        let quote = quote_res.map_err(|e| ApiError::Provider {
            provider: "finnhub",
            msg: format!("quote: {e}"),
        })?;
        let profile = profile_res.map_err(|e| ApiError::Provider {
            provider: "finnhub",
            msg: format!("profile: {e}"),
        })?;

        Ok(StockQuote {
            api: StockApiSource::Finnhub,
            symbol: self.symbol.clone(),
            name: profile.name.unwrap_or_else(|| self.symbol.clone()),
            open: quote.open,
            current: quote.current_price,
            high: quote.high,
            low: quote.low,
            previous_close: quote.previous_close,
        })
    }
}
