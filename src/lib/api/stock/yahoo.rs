//! Yahoo Finance unofficial chart endpoint — the only free, no-auth
//! source for historical equity closes after Finnhub's `/candle`
//! moved to a paid tier in 2024.
//!
//! Endpoint:
//! - `GET https://query1.finance.yahoo.com/v8/finance/chart/<SYMBOL>?range=<R>&interval=<I>`
//!
//! Used ranges → intervals: `1d`/`5m`, `1mo`/`1d`, `1y`/`1wk`. The
//! response shape is identical across them — a `chart.result[0]` with
//! `timestamp[]` and `indicators.quote[0].close[]` parallel arrays.
//!
//! This is an *unofficial* endpoint. Mitigations:
//! - Parse defensively: each per-range fetch errors independently so
//!   one missing window doesn't poison the others.
//! - The collector returns the last successfully populated snapshot;
//!   poll failures log and skip rather than blanking the panel.

use super::model::{HistorySeries, StockApiSource, StockHistory};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct YahooConfig {
    /// Ticker symbol — passed unmodified to Yahoo (case-sensitive on
    /// their side; "AAPL" works, "aapl" 404s).
    pub symbol: String,
}

pub struct YahooProvider {
    symbol: String,
}

/// Yahoo `range`/`interval` pairings tuned for the chart's three
/// windows. Wider ranges use coarser intervals so the response stays
/// under ~100 samples — plenty for a 62-pixel-wide graph.
const RANGE_DAY: (&str, &str) = ("1d", "5m");
const RANGE_MONTH: (&str, &str) = ("1mo", "1d");
const RANGE_YEAR: (&str, &str) = ("1y", "1wk");

impl YahooProvider {
    pub fn new(cfg: YahooConfig) -> Result<Self, ApiError> {
        if cfg.symbol.trim().is_empty() {
            return Err(ApiError::Config("yahoo: symbol missing".to_string()));
        }
        Ok(Self {
            // Yahoo is case-sensitive on the URL — preserve whatever
            // the user gave us instead of forcing upper/lower.
            symbol: cfg.symbol.trim().to_string(),
        })
    }

    /// Fetch all three windows in sequence. Each window is independent
    /// — a 404 on the year endpoint shouldn't blank the day endpoint.
    pub async fn poll_history(&self) -> Result<StockHistory, ApiError> {
        let day = self.fetch(RANGE_DAY).await;
        let month = self.fetch(RANGE_MONTH).await;
        let year = self.fetch(RANGE_YEAR).await;

        // Decide `current` / `previous_close` from whichever window
        // resolved successfully. The 1D window's last close is the
        // freshest live price Yahoo will give us via this endpoint;
        // its `previous_close` is supplied directly in the meta block.
        let (current, previous_close) = match &day {
            Ok(parsed) => (parsed.last_close, parsed.previous_close.unwrap_or(parsed.last_close)),
            Err(_) => (0.0, 0.0),
        };

        // If every window failed we surface the day error so the
        // operator sees the most relevant message.
        if day.is_err() && month.is_err() && year.is_err() {
            return Err(day.err().unwrap());
        }

        Ok(StockHistory {
            api: StockApiSource::Yahoo,
            symbol: self.symbol.to_uppercase(),
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

    async fn fetch(&self, range_interval: (&str, &str)) -> Result<ParsedWindow, ApiError> {
        let (range, interval) = range_interval;
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval={}",
            self.symbol, range, interval
        );
        let raw: RawResponse = get_json(&url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "yahoo",
            msg: e.to_string(),
        })?;
        parse_window(raw).ok_or_else(|| ApiError::Provider {
            provider: "yahoo",
            msg: format!("missing chart payload for range={range} interval={interval}"),
        })
    }
}

#[derive(Debug)]
struct ParsedWindow {
    closes: Vec<f64>,
    last_close: f64,
    previous_close: Option<f64>,
}

/// Pure: pull the closes out of a parsed Yahoo response. Returns
/// `None` if the chart block is empty or absent — Yahoo serves this
/// shape for delisted/unknown symbols.
fn parse_window(raw: RawResponse) -> Option<ParsedWindow> {
    let result = raw.chart.result.into_iter().next()?;
    let quote = result.indicators.quote.into_iter().next()?;
    let closes: Vec<f64> = quote
        .close
        .into_iter()
        .flatten() // drop null gaps
        .filter(|v| v.is_finite())
        .collect();
    if closes.is_empty() {
        return None;
    }
    let last_close = *closes.last().unwrap();
    Some(ParsedWindow {
        closes,
        last_close,
        previous_close: result.meta.previous_close,
    })
}

#[derive(Debug, Deserialize)]
struct RawResponse {
    chart: Chart,
}
#[derive(Debug, Deserialize)]
struct Chart {
    #[serde(default)]
    result: Vec<ChartResult>,
}
#[derive(Debug, Deserialize)]
struct ChartResult {
    meta: Meta,
    indicators: Indicators,
}
#[derive(Debug, Deserialize)]
struct Meta {
    #[serde(default, rename = "previousClose")]
    previous_close: Option<f64>,
}
#[derive(Debug, Deserialize)]
struct Indicators {
    #[serde(default)]
    quote: Vec<QuoteBlock>,
}
#[derive(Debug, Deserialize)]
struct QuoteBlock {
    #[serde(default)]
    close: Vec<Option<f64>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed but shape-accurate fixture from Yahoo's chart endpoint.
    // Real responses also carry open/high/low/volume arrays and a
    // longer meta block — we only deserialize what we need.
    const FIXTURE: &str = r#"{
        "chart": {
            "result": [
                {
                    "meta": {
                        "currency": "USD",
                        "symbol": "AAPL",
                        "previousClose": 181.10
                    },
                    "indicators": {
                        "quote": [
                            {
                                "close": [180.5, 181.2, null, 182.0, 182.34]
                            }
                        ]
                    }
                }
            ],
            "error": null
        }
    }"#;

    #[test]
    fn parses_real_response_shape() {
        let raw: RawResponse = serde_json::from_str(FIXTURE).expect("deserialize");
        let parsed = parse_window(raw).expect("window");
        // Null entries are dropped — 5 input samples become 4.
        assert_eq!(parsed.closes.len(), 4);
        assert_eq!(parsed.last_close, 182.34);
        assert_eq!(parsed.previous_close, Some(181.10));
    }

    #[test]
    fn empty_chart_returns_none() {
        let raw: RawResponse = serde_json::from_str(r#"{"chart":{"result":[]}}"#).unwrap();
        assert!(parse_window(raw).is_none());
    }

    #[test]
    fn quote_block_with_only_nulls_returns_none() {
        let raw: RawResponse = serde_json::from_str(
            r#"{"chart":{"result":[{"meta":{},"indicators":{"quote":[{"close":[null,null]}]}}]}}"#,
        )
        .unwrap();
        assert!(parse_window(raw).is_none(), "all-null close array should be treated as empty");
    }

    #[test]
    fn rejects_blank_symbol() {
        let r = YahooProvider::new(YahooConfig { symbol: "   ".into() });
        assert!(matches!(r, Err(ApiError::Config(_))));
    }
}
