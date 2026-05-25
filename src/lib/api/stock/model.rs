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
}

impl HistorySeries {
    /// Compute the series from raw closes; min/max scan once at
    /// construction time. Filters out NaN samples — some providers
    /// pad missing intervals with nulls that deserialize into NaN.
    pub fn from_closes(raw: Vec<f64>) -> Self {
        let closes: Vec<f64> = raw.into_iter().filter(|v| v.is_finite()).collect();
        let (low, high) = closes
            .iter()
            .copied()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        // An empty series yields ±inf extrema — collapse to (0, 0) so
        // the renderer's autoscale doesn't divide by ±inf.
        let (low, high) = if closes.is_empty() {
            (0.0, 0.0)
        } else {
            (low, high)
        };
        Self { closes, low, high }
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
    fn history_series_direction_and_percent() {
        let up = HistorySeries::from_closes(vec![100.0, 110.0]);
        assert_eq!(up.direction(), Direction::Up);
        assert!((up.percent_change() - 10.0).abs() < 1e-9);

        let down = HistorySeries::from_closes(vec![100.0, 80.0]);
        assert_eq!(down.direction(), Direction::Down);
        assert!((down.percent_change() + 20.0).abs() < 1e-9);
    }
}
