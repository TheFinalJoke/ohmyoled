//! Normalized stock quote — what the collector hands to the renderer.

use serde::{Deserialize, Serialize};

/// Which API produced this data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StockApiSource {
    Finnhub,
    Coingecko,
    /// Yahoo Finance unofficial chart endpoint — only used by the
    /// `chart: true` path when paired with a Finnhub-style ticker
    /// config (since Finnhub's `/candle` is paid-only).
    Yahoo,
}

/// One company's current trading state plus the metadata needed for display.
#[derive(Debug, Clone)]
pub struct StockQuote {
    pub api: StockApiSource,
    /// Ticker symbol (e.g. `"AAPL"`).
    pub symbol: String,
    /// Human-readable company name (`description.name` in the old Python type).
    pub name: String,
    pub open: f64,
    pub current: f64,
    pub high: f64,
    pub low: f64,
    pub previous_close: f64,
}

impl StockQuote {
    /// Absolute price delta from previous close.
    pub fn dollar_change(&self) -> f64 {
        self.current - self.previous_close
    }

    /// Percentage delta from previous close.
    pub fn percent_change(&self) -> f64 {
        if self.previous_close == 0.0 {
            0.0
        } else {
            (self.dollar_change() / self.previous_close) * 100.0
        }
    }

    /// `Up`, `Down`, or `Flat` — used by the renderer to pick text color.
    pub fn direction(&self) -> Direction {
        if self.current > self.previous_close {
            Direction::Up
        } else if self.current < self.previous_close {
            Direction::Down
        } else {
            Direction::Flat
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Flat,
}

/// Regular trading session bounds (epoch seconds, UTC) for an
/// intraday window. When present alongside per-sample `times`, the
/// renderer anchors the x-axis to `[start, end]` and positions each
/// sample by its real timestamp instead of evenly spacing them — so a
/// half-finished trading day fills only the elapsed half of the axis.
#[derive(Debug, Clone, Copy)]
pub struct TradingSession {
    /// Session open, epoch seconds (e.g. 9:30 AM ET).
    pub start: i64,
    /// Session close, epoch seconds (e.g. 4:00 PM ET).
    pub end: i64,
}

/// One period's worth of historical closes, plus the precomputed
/// extrema. The renderer needs the extrema for autoscaling the graph
/// — keeping them on the series saves the renderer from re-scanning
/// `closes` every frame.
#[derive(Debug, Clone)]
pub struct HistorySeries {
    /// Close prices, chronological — `closes[0]` is the oldest sample,
    /// `closes[last]` is the newest. Empty when the provider returned
    /// no usable data for this period.
    pub closes: Vec<f64>,
    pub low: f64,
    pub high: f64,
    /// Epoch-second timestamp for each close, parallel to `closes`.
    /// Empty unless the provider supplied intraday timestamps (only the
    /// 1D equity window does today) — month/year and crypto leave this
    /// empty and fall back to even spacing.
    pub times: Vec<i64>,
    /// Regular trading session bounds for this window, when known. Only
    /// meaningful together with a fully-populated `times`.
    pub session: Option<TradingSession>,
}

impl HistorySeries {
    /// Compute the extrema for a set of closes. An empty slice yields
    /// `(0.0, 0.0)` so the renderer's autoscale never divides by ±inf.
    fn extrema(closes: &[f64]) -> (f64, f64) {
        if closes.is_empty() {
            return (0.0, 0.0);
        }
        closes
            .iter()
            .copied()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    }

    /// Compute the series from raw closes; min/max scan once at
    /// construction time. Filters out NaN samples — some providers
    /// pad missing intervals with nulls that deserialize into NaN.
    /// Leaves `times`/`session` empty: this is the even-spacing path
    /// used by the month/year windows and by crypto.
    pub fn from_closes(raw: Vec<f64>) -> Self {
        let closes: Vec<f64> = raw.into_iter().filter(|v| v.is_finite()).collect();
        let (low, high) = Self::extrema(&closes);
        Self {
            closes,
            low,
            high,
            times: Vec::new(),
            session: None,
        }
    }

    /// Build an intraday series with per-sample timestamps and the
    /// session bounds. Filters non-finite closes in lockstep with their
    /// timestamps so the two arrays stay aligned. If `raw_times` doesn't
    /// match `raw_closes` in length, the timestamps are dropped and the
    /// series degrades gracefully to even spacing.
    pub fn from_samples(
        raw_closes: Vec<f64>,
        raw_times: Vec<i64>,
        session: Option<TradingSession>,
    ) -> Self {
        let have_times = raw_times.len() == raw_closes.len();
        let mut closes = Vec::with_capacity(raw_closes.len());
        let mut times = Vec::with_capacity(raw_closes.len());
        for (i, v) in raw_closes.into_iter().enumerate() {
            if v.is_finite() {
                closes.push(v);
                if have_times {
                    times.push(raw_times[i]);
                }
            }
        }
        let (low, high) = Self::extrema(&closes);
        // Only keep timestamps when they're fully aligned with closes.
        let times = if times.len() == closes.len() {
            times
        } else {
            Vec::new()
        };
        Self {
            closes,
            low,
            high,
            times,
            session,
        }
    }

    /// Time-anchored view of the series: `(closes, times, session)` when
    /// this window carries aligned per-sample timestamps and session
    /// bounds. `None` for the even-spacing windows (month/year/crypto)
    /// or any degenerate case, which tells the renderer to fall back to
    /// bucketing across the full width.
    pub fn intraday(&self) -> Option<(&[f64], &[i64], TradingSession)> {
        let session = self.session?;
        if self.closes.len() >= 2 && self.times.len() == self.closes.len() {
            Some((&self.closes, &self.times, session))
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.closes.is_empty()
    }

    /// Direction of the series — `Up` if the last close exceeds the
    /// first, `Down` if below, `Flat` for equal or empty.
    pub fn direction(&self) -> Direction {
        match (self.closes.first(), self.closes.last()) {
            (Some(&first), Some(&last)) if last > first => Direction::Up,
            (Some(&first), Some(&last)) if last < first => Direction::Down,
            _ => Direction::Flat,
        }
    }

    /// Percentage change between the first and last sample in the
    /// window. Returns 0 for empty or degenerate series.
    pub fn percent_change(&self) -> f64 {
        match (self.closes.first(), self.closes.last()) {
            (Some(&first), Some(&last)) if first != 0.0 => ((last - first) / first) * 100.0,
            _ => 0.0,
        }
    }
}

/// Three-window historical price snapshot for the chart renderer:
/// one day, one month, one year. Each period autoscales
/// independently so a flat 1D doesn't get drowned out by a volatile
/// 1Y in the same panel.
#[derive(Debug, Clone)]
pub struct StockHistory {
    pub api: StockApiSource,
    pub symbol: String,
    pub current: f64,
    /// Used to color the header `+/- %` for the live (1D-ish) tile —
    /// matches the existing live `StockQuote.previous_close` semantic.
    pub previous_close: f64,
    pub day: HistorySeries,
    pub month: HistorySeries,
    pub year: HistorySeries,
}

impl StockHistory {
    pub fn dollar_change(&self) -> f64 {
        self.current - self.previous_close
    }

    pub fn percent_change(&self) -> f64 {
        if self.previous_close == 0.0 {
            0.0
        } else {
            (self.dollar_change() / self.previous_close) * 100.0
        }
    }

    pub fn direction(&self) -> Direction {
        if self.current > self.previous_close {
            Direction::Up
        } else if self.current < self.previous_close {
            Direction::Down
        } else {
            Direction::Flat
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(current: f64, prev: f64) -> StockQuote {
        StockQuote {
            api: StockApiSource::Finnhub,
            symbol: "TEST".into(),
            name: "Test Co".into(),
            open: prev,
            current,
            high: current + 1.0,
            low: current - 1.0,
            previous_close: prev,
        }
    }

    #[test]
    fn dollar_change_basics() {
        let q = sample(110.0, 100.0);
        assert!((q.dollar_change() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn percent_change_basics() {
        let q = sample(110.0, 100.0);
        assert!((q.percent_change() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn percent_change_zero_prev_is_safe() {
        let q = sample(50.0, 0.0);
        assert_eq!(q.percent_change(), 0.0);
    }

    #[test]
    fn direction_cases() {
        assert_eq!(sample(110.0, 100.0).direction(), Direction::Up);
        assert_eq!(sample(90.0, 100.0).direction(), Direction::Down);
        assert_eq!(sample(100.0, 100.0).direction(), Direction::Flat);
    }

    #[test]
    fn history_series_extrema_skip_non_finite() {
        let s = HistorySeries::from_closes(vec![100.0, f64::NAN, 105.0, 95.0]);
        assert_eq!(s.closes.len(), 3, "NaN should be filtered");
        assert_eq!(s.low, 95.0);
        assert_eq!(s.high, 105.0);
    }

    #[test]
    fn history_series_empty_is_zero_zero() {
        let s = HistorySeries::from_closes(vec![]);
        assert!(s.is_empty());
        assert_eq!(s.low, 0.0);
        assert_eq!(s.high, 0.0);
        assert_eq!(s.direction(), Direction::Flat);
        assert_eq!(s.percent_change(), 0.0);
    }

    #[test]
    fn from_samples_keeps_times_aligned_through_nan_filter() {
        let session = TradingSession { start: 1_000, end: 24_400 };
        let s = HistorySeries::from_samples(
            vec![100.0, f64::NAN, 105.0, 95.0],
            vec![1_000, 1_300, 1_600, 1_900],
            Some(session),
        );
        // NaN sample dropped along with its timestamp.
        assert_eq!(s.closes, vec![100.0, 105.0, 95.0]);
        assert_eq!(s.times, vec![1_000, 1_600, 1_900]);
        let (closes, times, sess) = s.intraday().expect("intraday view");
        assert_eq!(closes.len(), times.len());
        assert_eq!(sess.start, 1_000);
    }

    #[test]
    fn from_samples_drops_times_on_length_mismatch() {
        // A timestamp array that doesn't match closes is discarded — the
        // series degrades to even spacing rather than mis-aligning.
        let s = HistorySeries::from_samples(
            vec![100.0, 105.0, 95.0],
            vec![1_000, 1_300],
            Some(TradingSession { start: 0, end: 1 }),
        );
        assert!(s.times.is_empty());
        assert!(s.intraday().is_none());
    }

    #[test]
    fn from_closes_has_no_intraday_view() {
        let s = HistorySeries::from_closes(vec![100.0, 105.0, 95.0]);
        assert!(s.times.is_empty());
        assert!(s.session.is_none());
        assert!(s.intraday().is_none());
    }

    #[test]
    fn intraday_needs_two_samples() {
        let s = HistorySeries::from_samples(
            vec![100.0],
            vec![1_000],
            Some(TradingSession { start: 0, end: 1 }),
        );
        assert!(s.intraday().is_none(), "single sample can't draw a line");
    }

    #[test]
    fn history_series_direction_and_percent() {
        let up = HistorySeries::from_closes(vec![100.0, 110.0]);
        assert_eq!(up.direction(), Direction::Up);
        assert!((up.percent_change() - 10.0).abs() < 1e-9);

        let down = HistorySeries::from_closes(vec![100.0, 80.0]);
        assert_eq!(down.direction(), Direction::Down);
        assert!((down.percent_change() + 20.0).abs() < 1e-9);
    }
}
