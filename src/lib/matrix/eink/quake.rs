//! E-paper earthquake renderer — magnitude + a world map with the epicenter.
//!
//! Static e-paper counterpart to [`crate::matrix::quake::QuakeMatrix`]. For an
//! event: a magnitude readout, a severity badge, a magnitude gauge, and depth/
//! felt stats on the left, with the **epicenter plotted on a world map** (seismic
//! rings sized by magnitude) on the right. A quiet feed renders a "NO QUAKES"
//! card so the panel never goes stale. Composed white-on-black; the display
//! inverts to black ink.
//!
//! Severity is never color-only (the panel is monochrome): the magnitude number
//! is always shown, the gauge fills proportionally, and the tier badge is
//! outlined for MINOR/MODERATE, filled (high-contrast) only for MAJOR (≥6).
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
    badge, badge_width, center_text, fill_rect, fit_text, footer, hbar, header_band, margin, scaled_px,
};
use crate::matrix::eink::worldmap;
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use chrono::Utc;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_circle, Font};
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
            big: Font::load_ttf(&paths.body, scaled_px(72.0, h))?,
            tier: Font::load_ttf(&paths.body, scaled_px(26.0, h))?,
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

        let age = age_text(ev.age_minutes(Utc::now()));
        let content_top = header_band(&mut img, &self.title, &self.label, m, "QUAKE", Some(&age), fg);

        // Place (the curated USGS title) along the bottom; time in the footer.
        let footer_top = hi - m - self.label.height() - self.foot.height() - m / 2;
        let place = fit_text(&self.label, &ev.title.to_uppercase(), wi - 2 * m);
        center_text(&mut img, &self.label, wi / 2, footer_top + self.label.ascent(), fg, &place);
        footer(&mut img, &self.foot, fg, &ev.origin.format("%b %-d  %H:%M UTC").to_string());

        // ── Left column: magnitude, tier, gauge, depth/felt ─────────────
        let col_w = wi * 36 / 100;
        let col_cx = (m + col_w) / 2;
        let mag_base = content_top + m + self.big.ascent();
        center_text(&mut img, &self.big, col_cx, mag_base, fg, &format!("M {:.1}", ev.magnitude));

        let (tier, filled) = if ev.magnitude >= 6.0 {
            ("MAJOR", true)
        } else if ev.magnitude >= 4.0 {
            ("MODERATE", false)
        } else {
            ("MINOR", false)
        };
        let tier_y = mag_base + m;
        badge(&mut img, &self.tier, col_cx - badge_width(&self.tier, tier) / 2, tier_y, tier, fg, filled);

        // Magnitude gauge (0..9) — fills proportionally with unit ticks.
        let gauge_y = tier_y + self.tier.height() + m;
        let gauge_w = col_w - 2 * m;
        hbar(&mut img, m, gauge_y, gauge_w, (hi / 24).max(8), (ev.magnitude / 9.0).clamp(0.0, 1.0), 9, fg);

        // Depth + felt.
        let depth_y = gauge_y + (hi / 24).max(8) + m + self.label.ascent();
        center_text(&mut img, &self.label, col_cx, depth_y, fg, &format!("DEPTH {:.0} km", ev.depth_km));
        let felt = match ev.felt {
            Some(n) => format!("FELT {n}"),
            None => "FELT —".into(),
        };
        center_text(&mut img, &self.label, col_cx, depth_y + self.label.height() + m / 2, fg, &felt);

        // ── Right: world map with the epicenter ─────────────────────────
        let map_x = col_w + m;
        let map_y = content_top + m / 2;
        let map_w = wi - m - map_x;
        let map_h = (footer_top - m / 2 - map_y).max(20);
        worldmap::draw(&mut img, map_x, map_y, map_w, map_h, fg);
        let (px, py) = worldmap::project(ev.lat, ev.lon, map_x, map_y, map_w, map_h);
        self.draw_epicenter(&mut img, px, py, ev.magnitude, map_h, fg);

        img
    }

    /// Concentric "seismic" rings around the epicenter, sized by magnitude.
    fn draw_epicenter(&self, img: &mut RgbImage, px: i32, py: i32, mag: f32, map_h: i32, fg: Color) {
        let max_r = (map_h / 4).max(10);
        let outer = (4.0 + mag * 3.0).round() as i32;
        let outer = outer.clamp(6, max_r);
        // Knock out a clear disc so the marker stands clear of the land stipple.
        fill_rect(img, px - outer - 1, py - outer - 1, 2 * outer + 3, 2 * outer + 3, Color::BLACK);
        for k in 1..=3 {
            draw_circle(img, px, py, outer * k / 3, fg);
        }
        fill_rect(img, px - 2, py - 2, 5, 5, fg);
    }

    fn frame_quiet(&self, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);

        let content_top = header_band(&mut img, &self.title, &self.label, m, "QUAKE", None, fg);
        let map_x = m;
        let map_y = content_top + m / 2;
        let map_w = wi - 2 * m;
        let map_h = (hi - m - map_y).max(20);
        worldmap::draw(&mut img, map_x, map_y, map_w, map_h, fg);
        let label = "NO RECENT QUAKES";
        let bw = badge_width(&self.tier, label);
        let bh = self.tier.height() * 3 / 2;
        let bx = wi / 2 - bw / 2;
        let by = hi / 2 - bh / 2;
        // Clear the land stipple behind the badge so it reads cleanly.
        fill_rect(&mut img, bx - m / 2, by - m / 2, bw + m, bh + m, Color::BLACK);
        badge(&mut img, &self.tier, bx, by, label, fg, false);
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
            lat: 38.3,
            lon: 142.1,
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
    fn epicenter_moves_with_coordinates() {
        let r = EinkQuakeMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let a = r.frame(&event(), 800, 480);
        let mut chile = event();
        if let QuakeStatus::Event(e) = &mut chile {
            e.lat = -33.0;
            e.lon = -72.0;
        }
        let b = r.frame(&chile, 800, 480);
        assert_ne!(a.into_raw(), b.into_raw(), "a different epicenter should move the marker");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkQuakeMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&event(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
