//! E-paper earthquake renderer — latest significant event, or a quiet card.
//!
//! Static e-paper counterpart to [`crate::matrix::quake::QuakeMatrix`]. For an
//! event: a big magnitude, a severity badge (filled only for major), the place,
//! and depth/felt stats. When the feed is quiet it renders a "NO QUAKES" card
//! so the panel never goes stale. Composed white-on-black; the display inverts.
//!
//! Severity is never color-only (the panel is monochrome): the magnitude number
//! is always shown, and the tier badge is outlined for MINOR/MODERATE, filled
//! (high-contrast) only for MAJOR (≥6).
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `quake` section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     quake:
//!       run: true
//!       feed: significant_day
//! ```
//!
//! Data source: USGS feeds (same collector as the LED tile).

use crate::api::quake::model::{QuakeEvent, QuakeStatus};
use crate::matrix::eink::layout::{
    badge, badge_width, big_value_centered, center_text, footer, header_band, margin, scaled_px, stat_row,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use chrono::Utc;
use image::RgbImage;
use ohmyoled_matrix::graphics::Font;
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper quake renderer.
pub struct EinkQuakeFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkQuakeFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper earthquake renderer.
pub struct EinkQuakeMatrix {
    title: Font,
    big: Font,
    tier: Font,
    place: Font,
    label: Font,
    foot: Font,
}

impl EinkQuakeMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkQuakeFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkQuakeFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkQuakeFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            big: Font::load_ttf(&paths.body, scaled_px(120.0, h))?,
            tier: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            place: Font::load_ttf(&paths.body, scaled_px(26.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkQuakeFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the screen at `w × h`.
    pub fn frame(&self, data: &QuakeStatus, w: u32, h: u32) -> RgbImage {
        match data {
            QuakeStatus::Event(ev) => self.frame_event(ev, w, h),
            QuakeStatus::Quiet => self.frame_quiet(w, h),
        }
    }

    fn frame_event(&self, ev: &QuakeEvent, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let age = age_text(ev.age_minutes(Utc::now()));
        header_band(&mut img, &self.title, &self.label, m, "QUAKE", Some(&age), fg);

        // Big magnitude hero.
        let hero_base = hi * 44 / 100 + self.big.ascent() / 2;
        big_value_centered(&mut img, &self.big, &self.label, cx, hero_base, fg, &format!("M {:.1}", ev.magnitude), "");

        // Severity tier badge — filled only for MAJOR.
        let (tier, filled) = if ev.magnitude >= 6.0 {
            ("MAJOR", true)
        } else if ev.magnitude >= 4.0 {
            ("MODERATE", false)
        } else {
            ("MINOR", false)
        };
        let tier_y = hero_base + m;
        let bx = cx - badge_width(&self.tier, tier) / 2;
        badge(&mut img, &self.tier, bx, tier_y, tier, fg, filled);

        // Place line (USGS title carries "M x.x - PLACE").
        let place_y = tier_y + self.tier.height() + m;
        center_text(&mut img, &self.place, cx, place_y + self.place.ascent(), fg, &ev.title);

        // Depth / felt stats.
        let felt = match ev.felt {
            Some(n) => format!("FELT {n}"),
            None => "FELT —".into(),
        };
        let stats = vec![format!("DEPTH {:.0} km", ev.depth_km), felt];
        let stat_y = hi - m - (self.label.height() - self.label.ascent());
        stat_row(&mut img, &self.label, stat_y, fg, &stats);

        footer(&mut img, &self.foot, fg, &ev.origin.format("%b %-d %H:%M UTC").to_string());
        img
    }

    fn frame_quiet(&self, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        header_band(&mut img, &self.title, &self.label, m, "QUAKE", None, fg);
        let bx = cx - badge_width(&self.big, "NO QUAKES") / 2;
        badge(&mut img, &self.big, bx, hi / 2 - self.big.height() / 2, "NO QUAKES", fg, false);
        img
    }
}

/// "12m ago" under an hour, "3h ago" beyond.
fn age_text(mins: u32) -> String {
    if mins < 60 {
        format!("{mins}m ago")
    } else {
        format!("{}h ago", mins / 60)
    }
}

#[async_trait]
impl EinkRenderer for EinkQuakeMatrix {
    type Data = QuakeStatus;

    fn id(&self) -> &'static str {
        "quake"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &QuakeStatus) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_fonts() -> EinkQuakeFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkQuakeFonts { body: base.join("04B_03B_.TTF") }
    }

    fn event() -> QuakeStatus {
        QuakeStatus::Event(QuakeEvent {
            magnitude: 6.2,
            title: "M 6.2 - OFF EAST COAST OF HONSHU, JAPAN".into(),
            origin: Utc::now() - chrono::Duration::minutes(14),
            depth_km: 24.0,
            felt: Some(482),
        })
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkQuakeMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&event(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated event, got {lit} lit px");
    }

    #[test]
    fn quiet_differs_from_event_and_renders() {
        let r = EinkQuakeMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let ev = r.frame(&event(), 800, 480);
        let quiet = r.frame(&QuakeStatus::Quiet, 800, 480);
        assert!(quiet.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 200, "quiet card must render");
        assert_ne!(ev.into_raw(), quiet.into_raw(), "quiet vs event should differ");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkQuakeMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&event(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
