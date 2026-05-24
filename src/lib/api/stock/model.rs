//! Normalized stock quote — what the collector hands to the renderer.

use serde::{Deserialize, Serialize};

/// Which API produced this data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StockApiSource {
    Finnhub,
    Coingecko,
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
}
