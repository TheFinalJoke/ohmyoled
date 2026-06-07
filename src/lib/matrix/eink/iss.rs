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
use crate::matrix::eink::layout::{
    badge, badge_width, fill_rect, footer, header_band, margin, rect, scaled_px, stat_row,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_circle, draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Continents as a union of lat/lon ellipses `(center_lat, center_lon,
/// semi_lat, semi_lon)`. Stylised, not survey-grade — enough to read as the
/// world. Antarctica is handled separately as a southern band.
const LAND: &[(f32, f32, f32, f32)] = &[
    // North America
    (50.0, -100.0, 18.0, 30.0),
    (62.0, -105.0, 12.0, 33.0),
    (64.0, -78.0, 10.0, 22.0),
    (24.0, -103.0, 10.0, 9.0),
    (12.0, -85.0, 7.0, 8.0),
    // Greenland
    (72.0, -42.0, 8.0, 14.0),
    // South America
    (-6.0, -58.0, 13.0, 14.0),
    (-30.0, -65.0, 18.0, 9.0),
    // Europe
    (50.0, 16.0, 11.0, 22.0),
    (60.0, 28.0, 10.0, 27.0),
    (54.0, -2.0, 5.0, 4.0),
    // Africa
    (17.0, 13.0, 16.0, 19.0),
    (2.0, 22.0, 11.0, 14.0),
    (-18.0, 25.0, 14.0, 12.0),
    // Asia
    (58.0, 95.0, 18.0, 58.0),
    (40.0, 80.0, 13.0, 38.0),
    (28.0, 80.0, 9.0, 12.0),
    (15.0, 78.0, 8.0, 8.0),
    (30.0, 112.0, 12.0, 20.0),
    (25.0, 45.0, 11.0, 13.0),
    (38.0, 140.0, 8.0, 4.0),
    // SE Asia / Indonesia
    (4.0, 110.0, 9.0, 20.0),
    // Australia
    (-25.0, 134.0, 12.0, 21.0),
];

/// Is `(lat, lon)` over land in the stylised model?
fn is_land(lat: f32, lon: f32) -> bool {
    if lat < -62.0 {
        return true; // Antarctica
    }
    LAND.iter().any(|&(clat, clon, alat, alon)| {
        let dl = (lat - clat) / alat;
        let dn = (lon - clon) / alon;
        dl * dl + dn * dn <= 1.0
    })
}

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
        rect(&mut img, mx, my, mw, mh, fg);
        let step = (mh / 44).clamp(4, 8);
        let mut gy = my + step / 2;
        while gy < my + mh {
            let lat = 90.0 - (gy - my) as f32 / mh as f32 * 180.0;
            let mut gx = mx + step / 2;
            while gx < mx + mw {
                let lon = (gx - mx) as f32 / mw as f32 * 360.0 - 180.0;
                if is_land(lat, lon) {
                    fill_rect(&mut img, gx, gy, 2, 2, fg);
                }
                gx += step;
            }
            gy += step;
        }

        // ── Plot the ISS at its sub-satellite point ─────────────────────
        let lat = data.lat.clamp(-90.0, 90.0) as f32;
        let lon = data.lon.clamp(-180.0, 180.0) as f32;
        let px = mx + ((lon + 180.0) / 360.0 * mw as f32) as i32;
        let py = my + ((90.0 - lat) / 180.0 * mh as f32) as i32;
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
    fn land_model_hits_known_points() {
        assert!(is_land(40.0, -100.0), "central US is land");
        assert!(is_land(0.0, 20.0), "central Africa is land");
        assert!(is_land(-80.0, 0.0), "Antarctica is land");
        assert!(!is_land(0.0, -140.0), "mid-Pacific is ocean");
        assert!(!is_land(40.0, -40.0), "mid-Atlantic is ocean");
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
