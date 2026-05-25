//! ISS renderer — two-mode 64×32 layout: a quiet "distance to ISS" tile
//! when the station is below the horizon, and a brighter "OVERHEAD"
//! banner when the user is inside the ISS visibility footprint.
//!
//! ```text
//!  ┌──────── normal mode ────────┐    ┌─────── overhead mode ───────┐
//!  │ ISS                         │    │       OVERHEAD              │
//!  │                             │    │                             │
//!  │      1247                   │    │      alt 421 km             │
//!  │       km                    │    │      vel 7.7 km/s           │
//!  └─────────────────────────────┘    └─────────────────────────────┘
//! ```
//!
//! Colors: cyan for the steady-state tile; magenta for the overhead banner.
//! The label/footer rows stay white so they read at a glance.
//!
//! # Config
//!
//! ```yaml
//! iss:
//!   run: true
//!   lat: 40.7128            # user latitude, decimal degrees
//!   lon: -74.0060           # user longitude, decimal degrees
//! ```
//!
//! Distance is great-circle (Haversine) from the configured `(lat, lon)`
//! to the ISS sub-point.
//!
//! # Data source
//!
//! `IssCollector::from_wheretheiss` — public `https://api.wheretheiss.at`
//! endpoint, no auth, 30 s refresh. The `footprint` field from the API
//! drives the overhead check.

use crate::api::iss::IssState;
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

const CYAN: Color = Color { r: 0, g: 200, b: 255 };
const MAGENTA: Color = Color { r: 255, g: 0, b: 200 };
const LABEL: Color = Color { r: 200, g: 200, b: 200 };

#[derive(Debug, Clone)]
pub struct IssFonts {
    pub body: PathBuf,
}

impl Default for IssFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
        }
    }
}

pub struct IssMatrix {
    label_font: Font,
    big_font: Font,
}

impl IssMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(IssFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(IssFonts::default()).await
    }

    pub fn with_fonts(paths: IssFonts) -> Result<Self, String> {
        Ok(Self {
            label_font: Font::load_ttf(&paths.body, 8.0)?,
            big_font: Font::load_ttf(&paths.body, 13.0)?,
        })
    }

    pub async fn with_fonts_async(paths: IssFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    pub fn frame(&self, data: &IssState) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        if data.overhead {
            self.draw_overhead(&mut img, data);
        } else {
            self.draw_distance(&mut img, data);
        }
        img
    }

    fn draw_distance(&self, img: &mut RgbImage, data: &IssState) {
        // Top: "ISS" label, left-justified with a 1-px inset.
        let label = "ISS";
        let label_baseline = self.label_font.ascent() + 1;
        draw_text(img, &self.label_font, 1, label_baseline, LABEL, label);

        // Center: big distance number with thousands separator.
        let big = format_with_commas(data.ground_distance_km);
        let big_w = self.big_font.text_width(&big);
        let big_x = ((PANEL_W as i32 - big_w) / 2).max(0);
        // Place the big number's baseline around y=22 to clear the label row
        // above and leave space for the "km" footer below.
        let big_y = 22;
        draw_text(img, &self.big_font, big_x, big_y, CYAN, &big);

        // Footer: "km" unit, centered, just below the big number.
        let unit = "km";
        let unit_w = self.label_font.text_width(unit);
        let unit_x = ((PANEL_W as i32 - unit_w) / 2).max(0);
        let unit_y = (PANEL_H as i32) - 2;
        draw_text(img, &self.label_font, unit_x, unit_y, LABEL, unit);
    }

    fn draw_overhead(&self, img: &mut RgbImage, data: &IssState) {
        // Top: centered "OVERHEAD" banner in magenta.
        let banner = "OVERHEAD";
        let banner_w = self.label_font.text_width(banner);
        let banner_x = ((PANEL_W as i32 - banner_w) / 2).max(0);
        let banner_y = self.label_font.ascent() + 1;
        draw_text(img, &self.label_font, banner_x, banner_y, MAGENTA, banner);

        // Two info rows: altitude (km) and velocity (km/s), both centered.
        let alt = format!("alt {} km", data.altitude_km.round() as i32);
        let vel = format!("vel {:.1} km/s", data.velocity_kms);

        let line_h = self.label_font.height().max(self.label_font.ascent() + 2);
        let alt_w = self.label_font.text_width(&alt);
        let alt_x = ((PANEL_W as i32 - alt_w) / 2).max(0);
        let alt_y = banner_y + line_h + 4;
        draw_text(img, &self.label_font, alt_x, alt_y, CYAN, &alt);

        let vel_w = self.label_font.text_width(&vel);
        let vel_x = ((PANEL_W as i32 - vel_w) / 2).max(0);
        let vel_y = alt_y + line_h + 2;
        draw_text(img, &self.label_font, vel_x, vel_y, LABEL, &vel);
    }
}

impl Default for IssMatrix {
    fn default() -> Self {
        Self::new().expect("default IssMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for IssMatrix {
    type Data = IssState;

    fn id(&self) -> &'static str {
        "iss"
    }

    fn cycle_duration(&self) -> Duration {
        // Hold the tile long enough to read comfortably; extend a little
        // when overhead so the user has time to react ("go outside!").
        Duration::from_secs(12)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &IssState) -> Result<(), RenderError> {
        matrix.clear();
        let img = self.frame(data);
        matrix.set_image(&img, 0, 0);
        let hold = if data.overhead {
            Duration::from_secs(20)
        } else {
            Duration::from_secs(12)
        };
        tokio::time::sleep(hold).await;
        matrix.clear();
        Ok(())
    }
}

/// Render an integer with comma thousands separators (e.g. `1247 → "1,247"`).
fn format_with_commas(n: u32) -> String {
    // Build right-to-left, inserting a comma every third digit, then reverse.
    let mut s: Vec<char> = Vec::new();
    for (i, c) in n.to_string().chars().rev().enumerate() {
        if i != 0 && i % 3 == 0 {
            s.push(',');
        }
        s.push(c);
    }
    s.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_fonts() -> IssFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        IssFonts {
            body: repo.join("04B_03B_.TTF"),
        }
    }

    fn sample_distance(km: u32) -> IssState {
        IssState {
            ground_distance_km: km,
            overhead: false,
            lat: 40.0,
            lon: -74.0,
            altitude_km: 421.0,
            velocity_kms: 7.66,
            visibility: "daylight".into(),
        }
    }

    fn sample_overhead() -> IssState {
        IssState {
            ground_distance_km: 250,
            overhead: true,
            lat: 40.0,
            lon: -74.0,
            altitude_km: 421.0,
            velocity_kms: 7.66,
            visibility: "daylight".into(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = IssMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img = m.frame(&sample_distance(1247));
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 60, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn overhead_mode_renders_different_pixels_than_distance_mode() {
        let m = IssMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img_d = m.frame(&sample_distance(1247));
        let img_o = m.frame(&sample_overhead());
        // Compare full pixel buffers — overhead mode has a different layout
        // (banner + two info lines vs. label + big number + km).
        assert_ne!(
            img_d.as_raw(),
            img_o.as_raw(),
            "overhead frame should differ from distance frame"
        );
    }

    #[test]
    fn overhead_mode_lights_magenta() {
        let m = IssMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img = m.frame(&sample_overhead());
        // The banner pixels are magenta-ish (high R, low G, high B). Any pixel
        // with R>200 and B>150 and G<100 counts as a banner pixel.
        let magenta_pixels = img
            .pixels()
            .filter(|p| p.0[0] > 200 && p.0[1] < 100 && p.0[2] > 150)
            .count();
        assert!(magenta_pixels > 0, "expected magenta banner pixels");
    }

    #[test]
    fn format_with_commas_basics() {
        assert_eq!(format_with_commas(0), "0");
        assert_eq!(format_with_commas(7), "7");
        assert_eq!(format_with_commas(42), "42");
        assert_eq!(format_with_commas(999), "999");
        assert_eq!(format_with_commas(1000), "1,000");
        assert_eq!(format_with_commas(1247), "1,247");
        assert_eq!(format_with_commas(12_345), "12,345");
        assert_eq!(format_with_commas(123_456), "123,456");
        assert_eq!(format_with_commas(1_234_567), "1,234,567");
    }
}
