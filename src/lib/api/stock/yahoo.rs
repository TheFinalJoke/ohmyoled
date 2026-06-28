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

use super::model::{HistorySeries, StockApiSource, StockHistory, TradingSession};
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
            // The 1D window carries per-sample timestamps + session
            // bounds so the renderer can lay it out by time of day.
            day: day
                .map(|p| HistorySeries::from_samples(p.closes, p.times, p.session))
                .unwrap_or_else(|_| HistorySeries::from_closes(vec![])),
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
    /// Epoch-second timestamp per surviving close, aligned with
    /// `closes`. Empty when Yahoo omitted the `timestamp` array (it's
    /// present on the intraday range, absent on some delisted symbols).
    times: Vec<i64>,
    /// Regular trading session for the day the bars belong to. `None`
    /// when the meta block lacks the offset or trading-period fields.
    session: Option<TradingSession>,
    last_close: f64,
    previous_close: Option<f64>,
}

/// Pure: pull the closes (and, when present, their timestamps) out of a
/// parsed Yahoo response. Returns `None` if the chart block is empty or
/// absent — Yahoo serves that shape for delisted/unknown symbols.
fn parse_window(raw: RawResponse) -> Option<ParsedWindow> {
    let result = raw.chart.result.into_iter().next()?;
    let timestamps = result.timestamp;
    let quote = result.indicators.quote.into_iter().next()?;

    // Walk closes and timestamps together so dropping a null close also
    // drops its timestamp — the two arrays must stay index-aligned.
    let have_times = timestamps.len() == quote.close.len();
    let mut closes = Vec::with_capacity(quote.close.len());
    let mut times = Vec::with_capacity(quote.close.len());
    for (i, c) in quote.close.into_iter().enumerate() {
        if let Some(v) = c {
            if v.is_finite() {
                closes.push(v);
                if have_times {
                    times.push(timestamps[i]);
                }
            }
        }
    }
    if closes.is_empty() {
        return None;
    }
    let last_close = *closes.last().unwrap();

    // Anchor the session to the calendar day of the most recent bar.
    let session = times
        .last()
        .and_then(|&last_ts| session_for_day(&result.meta, last_ts));

    Some(ParsedWindow {
        closes,
        times,
        session,
        last_close,
        previous_close: result.meta.previous_close,
    })
}

/// Derive the regular trading session (epoch seconds) for the calendar
/// day containing `anchor_ts`. Yahoo's `currentTradingPeriod.regular`
/// gives the canonical open/close, but its *date* rolls forward to the
/// next session once the market closes — so we take only its
/// **time-of-day** and re-anchor it to the day the bars actually belong
/// to. `gmtoffset` (the exchange's UTC offset, DST-correct for that
/// day) converts between epoch and exchange-local wall clock, so this
/// works for non-US exchanges too without a timezone database.
fn session_for_day(meta: &Meta, anchor_ts: i64) -> Option<TradingSession> {
    let gmt = meta.gmtoffset?;
    let regular = meta.current_trading_period.as_ref()?.regular.as_ref()?;
    let open = regular.start?;
    let close = regular.end?;

    const DAY: i64 = 86_400;
    // Seconds since exchange-local midnight for the canonical open/close.
    let open_tod = (open + gmt).rem_euclid(DAY);
    let close_tod = (close + gmt).rem_euclid(DAY);
    // Exchange-local midnight of the anchor day, back in epoch seconds.
    let local_midnight = (anchor_ts + gmt) - (anchor_ts + gmt).rem_euclid(DAY) - gmt;

    let start = local_midnight + open_tod;
    let mut end = local_midnight + close_tod;
    // Guard against a close-before-open wrap (shouldn't happen for a
    // normal session, but keeps the renderer's span strictly positive).
    if end <= start {
        end += DAY;
    }
    Some(TradingSession { start, end })
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
    /// Parallel to `indicators.quote[0].close` — epoch seconds per bar.
    /// Absent on some payloads, so default to empty.
    #[serde(default)]
    timestamp: Vec<i64>,
    indicators: Indicators,
}
#[derive(Debug, Deserialize)]
struct Meta {
    #[serde(default, rename = "previousClose")]
    previous_close: Option<f64>,
    /// Exchange UTC offset in seconds (DST-correct for the day).
    #[serde(default)]
    gmtoffset: Option<i64>,
    #[serde(default, rename = "currentTradingPeriod")]
    current_trading_period: Option<CurrentTradingPeriod>,
}
#[derive(Debug, Deserialize)]
struct CurrentTradingPeriod {
    #[serde(default)]
    regular: Option<TradingPeriod>,
}
#[derive(Debug, Deserialize)]
struct TradingPeriod {
    #[serde(default)]
    start: Option<i64>,
    #[serde(default)]
    end: Option<i64>,
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
                        "previousClose": 181.10,
                        "gmtoffset": -14400,
                        "currentTradingPeriod": {
                            "regular": { "start": 1718026200, "end": 1718049600 }
                        }
                    },
                    "timestamp": [1718026200, 1718026500, 1718026800, 1718027100, 1718027400],
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
    fn timestamps_drop_in_lockstep_with_null_closes() {
        let raw: RawResponse = serde_json::from_str(FIXTURE).expect("deserialize");
        let parsed = parse_window(raw).expect("window");
        // The 3rd close was null → its timestamp (…26800) is dropped too.
        assert_eq!(parsed.times.len(), parsed.closes.len());
        assert_eq!(
            parsed.times,
            vec![1718026200, 1718026500, 1718027100, 1718027400]
        );
    }

    #[test]
    fn session_anchors_to_the_bars_own_day() {
        // The fixture's gmtoffset is -4h (ET, DST). The regular period's
        // start time-of-day re-anchored to the bar day must equal the
        // first bar (the open). End must be after start.
        let raw: RawResponse = serde_json::from_str(FIXTURE).expect("deserialize");
        let parsed = parse_window(raw).expect("window");
        let s = parsed.session.expect("session derived");
        assert_eq!(s.start, 1718026200, "session start == 9:30 open bar");
        assert!(s.end > s.start);
        // 9:30 → 16:00 is 6.5h = 23_400s.
        assert_eq!(s.end - s.start, 23_400);
    }

    #[test]
    fn session_reanchors_when_trading_period_rolls_forward() {
        // Simulate an after-hours fetch: bars are from one day but
        // currentTradingPeriod.regular has rolled to the *next* session.
        // We should take only its time-of-day and re-anchor to the bars.
        let json = r#"{"chart":{"result":[{
            "meta":{
                "gmtoffset":-14400,
                "currentTradingPeriod":{"regular":{"start":1718112600,"end":1718136000}}
            },
            "timestamp":[1718026200, 1718049600],
            "indicators":{"quote":[{"close":[180.0, 182.0]}]}
        }]}}"#;
        let raw: RawResponse = serde_json::from_str(json).unwrap();
        let parsed = parse_window(raw).expect("window");
        let s = parsed.session.expect("session");
        // Re-anchored to the bar day, the open lands on 9:30 of that day.
        assert_eq!(s.start, 1718026200);
        assert_eq!(s.end - s.start, 23_400);
    }

    #[test]
    fn missing_meta_yields_no_session_but_still_parses() {
        // Closes with no gmtoffset/trading-period: times still captured,
        // session is None, and the renderer falls back to even spacing.
        let json = r#"{"chart":{"result":[{
            "meta":{"previousClose":100.0},
            "timestamp":[1,2,3],
            "indicators":{"quote":[{"close":[100.0,101.0,102.0]}]}
        }]}}"#;
        let raw: RawResponse = serde_json::from_str(json).unwrap();
        let parsed = parse_window(raw).expect("window");
        assert_eq!(parsed.times.len(), 3);
        assert!(parsed.session.is_none());
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
