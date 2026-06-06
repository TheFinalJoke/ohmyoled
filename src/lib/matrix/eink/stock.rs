//! E-paper stock renderer — symbol, big price, signed change, day range.
//!
//! Static e-paper counterpart to [`crate::matrix::stock::StockMatrix`]. The
//! LED tile colors the change green/red; on the monochrome panel direction is
//! carried by the signed number inside an outline badge. Composed white-on-black;
//! the display inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `stock` section shape:
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
//! Data source: whichever stock provider the section selects (same collector
//! as the LED tile).

use crate::api::stock::model::StockQuote;
use crate::matrix::eink::layout::{
    badge, badge_width, big_value_centered, fit_text, footer, header_band, margin, scaled_px, stat_row,
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
            big: Font::load_ttf(&paths.body, scaled_px(120.0, h))?,
            change: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkStockFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the quote at `w × h`.
    pub fn frame(&self, data: &StockQuote, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let name = fit_text(&self.label, &data.name.to_uppercase(), wi / 2 - 2 * m);
        let content_top = header_band(&mut img, &self.title, &self.label, m, &data.symbol, Some(&name), fg);
        let _ = content_top;

        // Big price hero.
        let hero_base = hi * 42 / 100 + self.big.ascent() / 2;
        if data.current.is_finite() {
            big_value_centered(&mut img, &self.big, &self.label, cx, hero_base, fg, &format!("{:.2}", data.current), "");
        } else {
            big_value_centered(&mut img, &self.big, &self.label, cx, hero_base, fg, "—", "");
        }

        // Signed change in an outline badge — the +/- carries direction on B/W.
        let change = format!("{:+.2}   {:+.2}%", data.dollar_change(), data.percent_change());
        let bx = cx - badge_width(&self.change, &change) / 2;
        badge(&mut img, &self.change, bx, hero_base + m, &change, fg, false);

        // Day range / open.
        let stats = vec![
            format!("OPEN {:.2}", data.open),
            format!("HIGH {:.2}", data.high),
            format!("LOW {:.2}", data.low),
        ];
        let stat_y = hi - m - (self.label.height() - self.label.ascent());
        stat_row(&mut img, &self.label, stat_y, fg, &stats);

        footer(&mut img, &self.foot, fg, &format!("prev close {:.2}", data.previous_close));
        img
    }
}

#[async_trait]
impl EinkRenderer for EinkStockMatrix {
    type Data = StockQuote;

    fn id(&self) -> &'static str {
        "stock"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &StockQuote) -> Result<(), RenderError> {
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

    fn repo_fonts() -> EinkStockFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkStockFonts { body: base.join("04B_03B_.TTF") }
    }

    fn sample(current: f64) -> StockQuote {
        StockQuote {
            api: StockApiSource::Finnhub,
            symbol: "AAPL".into(),
            name: "Apple Inc".into(),
            open: 188.0,
            current,
            high: 192.4,
            low: 187.1,
            previous_close: 190.0,
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
    fn adapts_to_smaller_panel() {
        let r = EinkStockMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&sample(191.2), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
