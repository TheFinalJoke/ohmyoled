//! E-paper stock-chart renderer — three stacked sparklines (1D / 1M / 1Y).
//!
//! Static e-paper counterpart to [`crate::matrix::stock_chart::StockChartMatrix`].
//! The LED tile cycles one period at a time; the still 800×480 panel shows all
//! three windows stacked, each autoscaled independently. Composed white-on-black;
//! the display inverts to black ink.
//!
//! # Config
//!
//! Built automatically alongside an `eink.modules.stock` tile that sets
//! `chart: true` (same trigger as the LED tile):
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
//!       chart: true
//! ```
//!
//! Data source: Yahoo Finance / CoinGecko history (same collector as the LED tile).

use crate::api::stock::model::{HistorySeries, StockHistory};
use crate::matrix::eink::layout::{
    badge, badge_width, footer, header_band, margin, right_text, scaled_px, sparkline,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper stock-chart renderer.
pub struct EinkStockChartFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkStockChartFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper stock-chart renderer.
pub struct EinkStockChartMatrix {
    title: Font,
    label: Font,
    pct: Font,
    foot: Font,
}

impl EinkStockChartMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkStockChartFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkStockChartFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkStockChartFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(34.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(24.0, h))?,
            pct: Font::load_ttf(&paths.body, scaled_px(20.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkStockChartFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the three-window chart at `w × h`.
    pub fn frame(&self, data: &StockHistory, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);

        let head = format!("{:+.2} ({:+.2}%)", data.dollar_change(), data.percent_change());
        let content_top = header_band(&mut img, &self.title, &self.pct, m, &data.symbol, Some(&head), fg);
        let body_bottom = hi - m - self.foot.height();

        let rows: [(&str, &HistorySeries); 3] =
            [("1D", &data.day), ("1M", &data.month), ("1Y", &data.year)];
        let row_h = (body_bottom - content_top) / 3;
        let gutter = wi / 9;
        for (i, (code, series)) in rows.iter().enumerate() {
            let top = content_top + row_h * i as i32;
            if i > 0 {
                draw_line(&mut img, m, top, wi - m, top, fg);
            }
            // Left gutter: period code + its % change (sign carries direction).
            draw_text(&mut img, &self.label, m, top + row_h / 2, fg, code);
            let pct = format!("{:+.1}%", series.percent_change());
            draw_text(&mut img, &self.pct, m, top + row_h / 2 + self.label.height(), fg, &pct);

            let plot_x = m + gutter;
            let plot_y = top + m / 2;
            let plot_h = row_h - m;
            if series.is_empty() {
                let plot_w = wi - m - plot_x;
                let bx = plot_x + plot_w / 2 - badge_width(&self.pct, "NO DATA") / 2;
                badge(&mut img, &self.pct, bx, plot_y + plot_h / 2 - self.pct.height() / 2, "NO DATA", fg, false);
            } else {
                // Reserve a right column for the period high/low labels.
                let hl_w = self.pct.text_width(&format!("{:.0}", series.high)).max(self.pct.text_width(&format!("{:.0}", series.low)));
                let plot_w = wi - m - plot_x - hl_w - m / 2;
                let pts: Vec<f32> = series.closes.iter().map(|&v| v as f32).collect();
                sparkline(&mut img, plot_x, plot_y, plot_w, plot_h, &pts, fg);
                // High at the top-right, low at the bottom-right of the plot.
                right_text(&mut img, &self.pct, wi - m, plot_y + self.pct.ascent(), fg, &format!("{:.0}", series.high));
                right_text(&mut img, &self.pct, wi - m, plot_y + plot_h - self.pct.height() + self.pct.ascent(), fg, &format!("{:.0}", series.low));
            }
        }

        footer(&mut img, &self.foot, fg, &format!("{}  now {:.2}", data.symbol, data.current));
        img
    }
}

#[async_trait]
impl EinkRenderer for EinkStockChartMatrix {
    type Data = StockHistory;

    fn id(&self) -> &'static str {
        "stock_chart"
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
    use crate::api::stock::model::StockApiSource;

    fn repo_fonts() -> EinkStockChartFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkStockChartFonts { body: base.join("04B_03B_.TTF") }
    }

    fn series(n: usize, base: f64, slope: f64) -> HistorySeries {
        HistorySeries::from_closes((0..n).map(|i| base + i as f64 * slope).collect())
    }

    fn sample() -> StockHistory {
        StockHistory {
            api: StockApiSource::Yahoo,
            symbol: "AAPL".into(),
            current: 191.2,
            previous_close: 190.0,
            day: series(24, 188.0, 0.15),
            month: series(30, 180.0, 0.4),
            year: series(52, 150.0, 0.8),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkStockChartMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&sample(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated chart, got {lit} lit px");
    }

    #[test]
    fn empty_window_renders_no_data() {
        let r = EinkStockChartMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let mut s = sample();
        s.day = HistorySeries::from_closes(vec![]);
        let img = r.frame(&s, 800, 480);
        assert!(img.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 200, "empty window still renders");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkStockChartMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&sample(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
