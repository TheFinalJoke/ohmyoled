//! E-paper stock renderer — big price, signed change, and today's chart.
//!
//! Static e-paper counterpart to [`crate::matrix::stock::StockMatrix`], enriched
//! with an intraday (1D) sparkline. Consumes [`StockHistory`] (the same data the
//! `stock_chart` tile uses) so the live price and today's shape live on one
//! screen. The LED tile colors the change green/red; on the monochrome panel
//! direction is the signed number + the chart's own slope. Composed
//! white-on-black; the display inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `stock` section shape. The
//! intraday history is sourced like the chart tile (Yahoo for equities,
//! CoinGecko for coins), so the live price tracks that provider:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     stock:
//!       run: true
//!       api: finnhub
//!       api_key: "REPLACE_ME_FINNHUB_KEY"
//!       symbol: AAPL
//! ```
//!
//! Data source: Yahoo Finance / CoinGecko history (same collector as `stock_chart`).

use crate::api::stock::model::StockHistory;
use crate::matrix::eink::layout::{
    badge, badge_width, big_value_centered, fit_text, footer, header_band, margin, scaled_px, sparkline, stat_row,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::Font;
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper stock renderer.
pub struct EinkStockFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkStockFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper stock quote renderer.
pub struct EinkStockMatrix {
    title: Font,
    big: Font,
    change: Font,
    label: Font,
    foot: Font,
}

impl EinkStockMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkStockFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkStockFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkStockFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(34.0, h))?,
            big: Font::load_ttf(&paths.body, scaled_px(96.0, h))?,
            change: Font::load_ttf(&paths.body, scaled_px(28.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkStockFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the quote + today's chart at `w × h`.
    pub fn frame(&self, data: &StockHistory, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        header_band(&mut img, &self.title, &self.label, m, &data.symbol, Some("TODAY"), fg);

        // Big current price hero (upper third).
        let hero_base = hi * 30 / 100 + self.big.ascent() / 2;
        if data.current.is_finite() {
            big_value_centered(&mut img, &self.big, &self.label, cx, hero_base, fg, &format!("${:.2}", data.current), "");
        } else {
            big_value_centered(&mut img, &self.big, &self.label, cx, hero_base, fg, "—", "");
        }

        // Signed change in an outline badge — the +/- carries direction on B/W.
        let dc = data.dollar_change();
        let sign = if dc < 0.0 { "-" } else { "+" };
        let change = format!("{sign}${:.2}   {:+.2}%", dc.abs(), data.percent_change());
        let bx = cx - badge_width(&self.change, &change) / 2;
        badge(&mut img, &self.change, bx, hero_base + m, &change, fg, false);

        // ── Today's (1D) chart ──────────────────────────────────────────
        let chart_top = hero_base + m + self.change.height() + m;
        let stat_y = hi - m - (self.label.height() - self.label.ascent());
        let chart_bottom = stat_y - self.label.height() - m;
        if data.day.is_empty() {
            let bx = cx - badge_width(&self.label, "NO 1D DATA") / 2;
            badge(&mut img, &self.label, bx, (chart_top + chart_bottom) / 2, "NO 1D DATA", fg, false);
        } else {
            let pts: Vec<f32> = data.day.closes.iter().map(|&v| v as f32).collect();
            sparkline(&mut img, m, chart_top, wi - 2 * m, (chart_bottom - chart_top).max(8), &pts, fg);
        }

        // Day range from the intraday series.
        let stats = vec![
            format!("H ${:.2}", data.day.high),
            format!("L ${:.2}", data.day.low),
        ];
        stat_row(&mut img, &self.label, stat_y, fg, &stats);

        let prev = fit_text(&self.foot, &format!("prev close ${:.2}", data.previous_close), wi - 2 * m);
        footer(&mut img, &self.foot, fg, &prev);
        img
    }
}

#[async_trait]
impl EinkRenderer for EinkStockMatrix {
    type Data = StockHistory;

    fn id(&self) -> &'static str {
        "stock"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &StockHistory) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::stock::model::{HistorySeries, StockApiSource};

    fn repo_fonts() -> EinkStockFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkStockFonts { body: base.join("04B_03B_.TTF") }
    }

    fn series(n: usize, base: f64, slope: f64) -> HistorySeries {
        HistorySeries::from_closes((0..n).map(|i| base + i as f64 * slope).collect())
    }

    fn sample(current: f64) -> StockHistory {
        StockHistory {
            api: StockApiSource::Yahoo,
            symbol: "AAPL".into(),
            current,
            previous_close: 190.0,
            day: series(26, 188.0, 0.15),
            month: series(30, 180.0, 0.4),
            year: series(52, 150.0, 0.8),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkStockMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&sample(191.2), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated quote, got {lit} lit px");
    }

    #[test]
    fn up_and_down_differ() {
        let r = EinkStockMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let up = r.frame(&sample(195.0), 800, 480);
        let down = r.frame(&sample(185.0), 800, 480);
        assert_ne!(up.into_raw(), down.into_raw(), "up vs down change should differ");
    }

    #[test]
    fn empty_day_renders_badge() {
        let r = EinkStockMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let mut s = sample(191.2);
        s.day = HistorySeries::from_closes(vec![]);
        let img = r.frame(&s, 800, 480);
        assert!(img.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 200, "empty day still renders");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkStockMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&sample(191.2), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
