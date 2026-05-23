//! Finnhub provider — direct HTTP via our shared `reqwest` client.
//!
//! Previously used the [`finnhub`](https://crates.io/crates/finnhub) crate,
//! but that crate's transitive `reqwest` dep is built with `default_features
//! = true`, which enables `native-tls` and drags in `openssl-sys` — a
//! showstopper for the aarch64/armv7 cross-compile. We only consume two
//! endpoints, so a direct call is both smaller and TLS-stack-agnostic.
//!
//! Endpoints:
//! - `https://finnhub.io/api/v1/quote?symbol=...&token=...`         → price quote
//! - `https://finnhub.io/api/v1/stock/profile2?symbol=...&token=...` → company name

use super::model::{StockApiSource, StockQuote};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct FinnhubConfig {
    pub api_key: String,
    pub symbol: String,
}

pub struct FinnhubProvider {
    api_key: String,
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
            api_key: cfg.api_key,
            symbol: cfg.symbol.to_uppercase(),
        })
    }

    pub async fn poll(&self) -> Result<StockQuote, ApiError> {
        let quote_url = format!(
            "https://finnhub.io/api/v1/quote?symbol={}&token={}",
            self.symbol, self.api_key
        );
        let profile_url = format!(
            "https://finnhub.io/api/v1/stock/profile2?symbol={}&token={}",
            self.symbol, self.api_key
        );

        let (quote_res, profile_res) = tokio::join!(
            get_json::<QuoteRaw>(&quote_url, &[]),
            get_json::<ProfileRaw>(&profile_url, &[]),
        );

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
            open: quote.o,
            current: quote.c,
            high: quote.h,
            low: quote.l,
            previous_close: quote.pc,
        })
    }
}

// Finnhub's quote endpoint uses one-letter field names — current `c`,
// high `h`, low `l`, open `o`, previous close `pc`.
#[derive(Debug, Deserialize)]
struct QuoteRaw {
    #[serde(default)]
    c: f64,
    #[serde(default)]
    h: f64,
    #[serde(default)]
    l: f64,
    #[serde(default)]
    o: f64,
    #[serde(default)]
    pc: f64,
}

// `profile2` returns the human-readable company name. We don't use the
// other fields (industry, market cap, logo, etc.) but the upstream may
// return them, so leave the struct permissive.
#[derive(Debug, Deserialize)]
struct ProfileRaw {
    #[serde(default)]
    name: Option<String>,
}
