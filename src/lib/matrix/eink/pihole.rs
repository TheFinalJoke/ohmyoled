//! E-paper Pi-hole renderer — a big block-rate hero + the day's query stats.
//!
//! The static e-paper counterpart to [`crate::matrix::pihole::PiholeMatrix`]:
//! one large "percent blocked" number, a gauge echoing it, and a row of the
//! day's totals. Composed white-on-black; the display inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `pihole` section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     pihole:
//!       run: true
//!       base_url: "http://pi.hole"
//!       token: "REPLACE_ME_PIHOLE_TOKEN"
//! ```
//!
//! Data source: the Pi-hole admin API (same collector as the LED tile).

use crate::api::pihole::model::PiholeSummary;
use crate::matrix::eink::layout::{
    big_value_centered, center_text, hbar, header_band, margin, scaled_px, stat_row,
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

/// Font paths for the e-paper Pi-hole renderer.
pub struct EinkPiholeFonts {
    /// Body/label/stat font (04B_03B TTF).
    pub body: PathBuf,
    /// Heavy font for the big percentage (04b24 OTF).
    pub big: PathBuf,
}

impl Default for EinkPiholeFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
            big: Path::new(FONT_DIR).join("04b24.otf"),
        }
    }
}

/// Static e-paper Pi-hole dashboard renderer.
pub struct EinkPiholeMatrix {
    title: Font,
    big: Font,
    unit: Font,
    label: Font,
}

impl EinkPiholeMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkPiholeFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkPiholeFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkPiholeFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            big: Font::load_ttf(&paths.big, scaled_px(120.0, h))?,
            unit: Font::load_ttf(&paths.body, scaled_px(42.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkPiholeFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the dashboard at `w × h`.
    pub fn frame(&self, data: &PiholeSummary, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let content_top = header_band(&mut img, &self.title, &self.label, m, "PI-HOLE", Some("TODAY"), fg);

        // Caption under the header.
        center_text(&mut img, &self.label, cx, content_top + self.label.ascent(), fg, "BLOCKED");

        // Big percentage hero, vertically around 42% of the panel.
        let hero_base = hi * 42 / 100 + self.big.ascent() / 2;
        let pct = format!("{:.1}", data.percent_blocked);
        big_value_centered(&mut img, &self.big, &self.unit, cx, hero_base, fg, &pct, "%");

        // Gauge echoing the percentage.
        let bar_w = wi - 2 * m;
        let bar_h = (hi / 22).max(6);
        let bar_y = hero_base + m;
        hbar(&mut img, m, bar_y, bar_w, bar_h, data.percent_blocked / 100.0, 10, fg);

        // Stat row of the day's totals near the bottom.
        let stats = vec![
            format!("QUERIES {}", compact(data.queries_today)),
            format!("BLOCKED {}", compact(data.blocked_today)),
            format!("CLIENTS {}", data.unique_clients),
        ];
        let stat_y = hi - m - (self.label.height() - self.label.ascent());
        stat_row(&mut img, &self.label, stat_y, fg, &stats);

        img
    }
}

/// Compact large counts: `12_348 -> "12.3k"`, `2_100_000 -> "2.1M"`.
fn compact(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f32 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f32 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[async_trait]
impl EinkRenderer for EinkPiholeMatrix {
    type Data = PiholeSummary;

    fn id(&self) -> &'static str {
        "pihole"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &PiholeSummary) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_fonts() -> EinkPiholeFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkPiholeFonts {
            body: base.join("04B_03B_.TTF"),
            big: base.join("04b24.otf"),
        }
    }

    fn sample() -> PiholeSummary {
        PiholeSummary {
            percent_blocked: 34.2,
            queries_today: 12_348,
            blocked_today: 4_221,
            unique_clients: 12,
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkPiholeMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&sample(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated dashboard, got {lit} lit px");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkPiholeMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&sample(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
