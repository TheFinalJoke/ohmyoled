//! E-paper ISS renderer — ground distance, or an OVERHEAD alert during a pass.
//!
//! Static e-paper counterpart to [`crate::matrix::iss::IssMatrix`]. Two modes:
//! a big ground-distance readout, and — when the station is overhead — a
//! high-contrast filled OVERHEAD banner promoting altitude/velocity. Composed
//! white-on-black; the display inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `iss` section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     iss:
//!       run: true
//!       lat: 40.71
//!       lon: -74.0
//! ```
//!
//! Data source: where-the-iss-at (same collector as the LED tile).

use crate::api::iss::model::IssState;
use crate::matrix::eink::layout::{
    badge, badge_width, big_value_centered, center_text, footer, header_band, margin, scaled_px, stat_row,
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

/// Font paths for the e-paper ISS renderer.
pub struct EinkIssFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkIssFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper ISS tracker renderer.
pub struct EinkIssMatrix {
    title: Font,
    big: Font,
    unit: Font,
    label: Font,
    banner: Font,
    foot: Font,
}

impl EinkIssMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkIssFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkIssFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkIssFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            big: Font::load_ttf(&paths.body, scaled_px(120.0, h))?,
            unit: Font::load_ttf(&paths.body, scaled_px(40.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            banner: Font::load_ttf(&paths.body, scaled_px(40.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkIssFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the tracker at `w × h`.
    pub fn frame(&self, data: &IssState, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let vis = data.visibility.to_uppercase();
        let content_top = header_band(&mut img, &self.title, &self.label, m, "ISS", Some(&vis), fg);

        if data.overhead {
            // High-contrast banner = the alert (replaces the LED magenta).
            let bx = cx - badge_width(&self.banner, "OVERHEAD") / 2;
            badge(&mut img, &self.banner, bx, content_top, "OVERHEAD", fg, true);
            // Promote altitude to the hero.
            let hero_base = hi * 52 / 100 + self.big.ascent() / 2;
            center_text(&mut img, &self.label, cx, hero_base - self.big.ascent() - m, fg, "ALTITUDE");
            big_value_centered(&mut img, &self.big, &self.unit, cx, hero_base, fg, &format!("{:.0}", data.altitude_km), "km");
            let stats = vec![
                format!("VEL {:.2} km/s", data.velocity_kms),
                format!("DIST {} km", data.ground_distance_km),
            ];
            stat_row(&mut img, &self.label, hero_base + self.label.height() + m, fg, &stats);
        } else {
            let hero_base = hi * 48 / 100 + self.big.ascent() / 2;
            center_text(&mut img, &self.label, cx, content_top + self.label.ascent(), fg, "GROUND DISTANCE");
            big_value_centered(&mut img, &self.big, &self.unit, cx, hero_base, fg, &format!("{}", data.ground_distance_km), "km");
            let stats = vec![
                format!("ALT {:.0} km", data.altitude_km),
                format!("VEL {:.2} km/s", data.velocity_kms),
            ];
            stat_row(&mut img, &self.label, hero_base + self.label.height() + m, fg, &stats);
        }

        footer(&mut img, &self.foot, fg, &format!("{:.2}, {:.2}", data.lat, data.lon));
        img
    }
}

#[async_trait]
impl EinkRenderer for EinkIssMatrix {
    type Data = IssState;

    fn id(&self) -> &'static str {
        "iss"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &IssState) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        // Refresh faster during a pass — ground distance moves quickly.
        let dwell = if data.overhead { 15 } else { 60 };
        tokio::time::sleep(Duration::from_secs(dwell)).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_fonts() -> EinkIssFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkIssFonts { body: base.join("04B_03B_.TTF") }
    }

    fn distant() -> IssState {
        IssState {
            ground_distance_km: 1247,
            overhead: false,
            lat: 23.5,
            lon: -50.1,
            altitude_km: 421.0,
            velocity_kms: 7.66,
            visibility: "daylight".into(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkIssMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&distant(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated tracker, got {lit} lit px");
    }

    #[test]
    fn overhead_differs_from_distant() {
        let r = EinkIssMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let mut overhead = distant();
        overhead.overhead = true;
        overhead.ground_distance_km = 120;
        let a = r.frame(&distant(), 800, 480);
        let b = r.frame(&overhead, 800, 480);
        assert_ne!(a.into_raw(), b.into_raw(), "overhead banner should change the frame");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkIssMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&distant(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
