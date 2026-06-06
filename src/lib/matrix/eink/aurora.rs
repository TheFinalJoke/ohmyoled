//! E-paper aurora renderer — the Kp index, a 9-step gauge, and an alert badge.
//!
//! Static e-paper counterpart to [`crate::matrix::aurora::AuroraMatrix`]: one
//! big Kp digit, an [`hbar`](crate::matrix::eink::layout::hbar) gauge with nine
//! ticks (the LED's 9-block bar), and a filled "AURORA LIKELY" badge when the
//! storm threshold is crossed. Composed white-on-black; the display inverts.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `aurora` section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     aurora:
//!       run: true
//!       alert_threshold: 5
//! ```
//!
//! Data source: NOAA SWPC (same collector as the LED tile).

use crate::api::aurora::model::AuroraReading;
use crate::matrix::eink::layout::{
    badge, badge_width, big_value_centered, center_text, footer, header_band, hbar, margin, scaled_px,
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

/// Font paths for the e-paper aurora renderer.
pub struct EinkAuroraFonts {
    /// Body/label font (04B_03B TTF).
    pub body: PathBuf,
    /// Heavy font for the big Kp digit (04b24 OTF).
    pub big: PathBuf,
}

impl Default for EinkAuroraFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
            big: Path::new(FONT_DIR).join("04b24.otf"),
        }
    }
}

/// Static e-paper aurora dashboard renderer.
pub struct EinkAuroraMatrix {
    title: Font,
    big: Font,
    label: Font,
    banner: Font,
    foot: Font,
}

impl EinkAuroraMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkAuroraFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkAuroraFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkAuroraFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            big: Font::load_ttf(&paths.big, scaled_px(150.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            banner: Font::load_ttf(&paths.body, scaled_px(34.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkAuroraFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the dashboard at `w × h`.
    pub fn frame(&self, data: &AuroraReading, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let content_top = header_band(&mut img, &self.title, &self.label, m, "AURORA", Some(&data.kp_text), fg);

        // Alert banner (filled = high contrast) only above the threshold.
        if data.alert {
            let bx = cx - badge_width(&self.banner, "AURORA LIKELY") / 2;
            badge(&mut img, &self.banner, bx, content_top, "AURORA LIKELY", fg, true);
        } else {
            center_text(&mut img, &self.label, cx, content_top + self.label.ascent(), fg, "QUIET SKIES");
        }

        // Big Kp digit hero.
        center_text(&mut img, &self.label, cx, hi * 36 / 100, fg, "Kp INDEX");
        let hero_base = hi * 56 / 100 + self.big.ascent() / 2;
        big_value_centered(&mut img, &self.big, &self.label, cx, hero_base, fg, &format!("{}", data.kp), "");

        // Nine-step gauge below the hero.
        let bar_w = wi - 2 * m;
        let bar_h = (hi / 18).max(8);
        let bar_y = hero_base + m;
        hbar(&mut img, m, bar_y, bar_w, bar_h, data.kp as f32 / 9.0, 9, fg);

        footer(&mut img, &self.foot, fg, &format!("updated {}", data.sampled_at.format("%H:%M UTC")));
        img
    }
}

#[async_trait]
impl EinkRenderer for EinkAuroraMatrix {
    type Data = AuroraReading;

    fn id(&self) -> &'static str {
        "aurora"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &AuroraReading) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn repo_fonts() -> EinkAuroraFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkAuroraFonts {
            body: base.join("04B_03B_.TTF"),
            big: base.join("04b24.otf"),
        }
    }

    fn reading(kp: u8, alert: bool) -> AuroraReading {
        AuroraReading {
            kp,
            kp_index: kp as f32,
            kp_text: format!("{kp}Z"),
            alert,
            sampled_at: Utc::now(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkAuroraMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&reading(6, true), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated dashboard, got {lit} lit px");
    }

    #[test]
    fn alert_differs_from_quiet() {
        let r = EinkAuroraMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let a = r.frame(&reading(2, false), 800, 480);
        let b = r.frame(&reading(8, true), 800, 480);
        assert_ne!(a.into_raw(), b.into_raw(), "alert banner + Kp should change the frame");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkAuroraMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&reading(4, false), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
