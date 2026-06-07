//! E-paper ISS renderer — a stippled world map with the station plotted on it.
//!
//! Static e-paper counterpart to [`crate::matrix::iss::IssMatrix`]. The whole
//! panel is a world map (equirectangular), continents drawn as a light stipple,
//! with the ISS marked at its sub-satellite point and a strip of telemetry
//! (altitude / velocity / ground distance) beneath. An `OVERHEAD` badge pops
//! next to the marker during a pass. Composed white-on-black; the display
//! inverts to black ink.
//!
//! The land model is a compact union of lat/lon ellipses (plus an Antarctic
//! band) — no map asset or network needed; it renders as a recognisable,
//! stylised world.
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
use crate::matrix::eink::layout::{badge, badge_width, fill_rect, footer, header_band, margin, scaled_px, stat_row};
use crate::matrix::eink::worldmap;
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_circle, draw_line, draw_text, Font};
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
    label: Font,
    marker: Font,
    badge: Font,
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
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            marker: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
            badge: Font::load_ttf(&paths.body, scaled_px(24.0, h))?,
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

        let vis = data.visibility.to_uppercase();
        let content_top = header_band(&mut img, &self.title, &self.label, m, "ISS TRACKER", Some(&vis), fg);

        // Reserve a telemetry strip at the bottom; the map fills the rest.
        let stat_block = self.label.height() + self.foot.height() + 2 * m;
        let map_top = content_top + m / 2;
        let map_bottom = hi - stat_block;
        let avail_w = wi - 2 * m;
        let avail_h = (map_bottom - map_top).max(20);
        // Keep the equirectangular 2:1 aspect, centered in the available band.
        let (mw, mh) = if avail_w <= avail_h * 2 {
            (avail_w, avail_w / 2)
        } else {
            (avail_h * 2, avail_h)
        };
        let mx = (wi - mw) / 2;
        let my = map_top + (avail_h - mh) / 2;

        // Map frame + stippled land.
        worldmap::draw(&mut img, mx, my, mw, mh, fg);

        // ── Plot the ISS at its sub-satellite point ─────────────────────
        let (px, py) = worldmap::project(data.lat as f32, data.lon as f32, mx, my, mw, mh);
        // Faint crosshair across the map for the current lat/lon.
        draw_line(&mut img, mx, py, mx + mw, py, fg);
        draw_line(&mut img, px, my, px, my + mh, fg);
        // Marker: solid dot clear of the stipple, ringed.
        fill_rect(&mut img, px - 8, py - 8, 17, 17, Color::BLACK);
        fill_rect(&mut img, px - 3, py - 3, 7, 7, fg);
        draw_circle(&mut img, px, py, 8, fg);
        // Label, kept inside the map.
        let lab_x = if px + 12 + self.marker.text_width("ISS") < mx + mw { px + 12 } else { px - 12 - self.marker.text_width("ISS") };
        draw_text(&mut img, &self.marker, lab_x, py + self.marker.ascent() / 2, fg, "ISS");
        if data.overhead {
            let bx = (px - badge_width(&self.badge, "OVERHEAD") / 2).clamp(mx + 2, mx + mw - badge_width(&self.badge, "OVERHEAD") - 2);
            let by = (py + 14).min(my + mh - self.badge.height() - 2);
            badge(&mut img, &self.badge, bx, by, "OVERHEAD", fg, true);
        }

        // ── Telemetry strip ─────────────────────────────────────────────
        let stats = vec![
            format!("ALT {:.0} km", data.altitude_km),
            format!("VEL {:.2} km/s", data.velocity_kms),
            format!("DIST {} km", data.ground_distance_km),
        ];
        stat_row(&mut img, &self.label, map_bottom + m + self.label.ascent(), fg, &stats);
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
        // The station moves ~7.6 km/s, so refresh faster during a pass.
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
        assert!(lit > 200, "expected a populated map, got {lit} lit px");
    }

    #[test]
    fn position_changes_the_frame() {
        let r = EinkIssMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let mut other = distant();
        other.lat = -33.0;
        other.lon = 151.0; // over Australia
        let a = r.frame(&distant(), 800, 480);
        let b = r.frame(&other, 800, 480);
        assert_ne!(a.into_raw(), b.into_raw(), "a different sub-point should move the marker");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkIssMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&distant(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
