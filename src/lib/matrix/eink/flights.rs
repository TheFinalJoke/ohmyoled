//! E-paper flight-radar renderer — a static polar radar + a ranked list.
//!
//! Static e-paper counterpart to [`crate::matrix::flights::FlightsMatrix`]. The
//! LED tile animates a sweeping radar; on the still panel we draw the radar
//! once (range rings + crosshair + N) and plot each aircraft by bearing and
//! distance, with the nearest ringed. Composed white-on-black; the display
//! inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `flights` section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     flights:
//!       run: true
//!       lat: 40.71
//!       lon: -74.0
//!       radius_km: 80
//! ```
//!
//! Data source: OpenSky (same collector as the LED tile).

use crate::api::flights::model::{bearing_octant, FlightSnapshot};
use crate::matrix::eink::layout::{badge, badge_width, fit_text, footer, header_band, margin, scaled_px};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_circle, draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper flights renderer.
pub struct EinkFlightsFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkFlightsFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper flight-radar renderer.
pub struct EinkFlightsMatrix {
    title: Font,
    list: Font,
    label: Font,
    foot: Font,
}

impl EinkFlightsMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkFlightsFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkFlightsFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkFlightsFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            list: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkFlightsFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the radar + list at `w × h`.
    pub fn frame(&self, data: &FlightSnapshot, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);

        let right = format!("{} IN {:.0}km", data.count, data.radius_km);
        let content_top = header_band(&mut img, &self.title, &self.label, m, "FLIGHTS", Some(&right), fg);
        let body_bottom = hi - m - self.foot.height();

        // ── Radar in the left half ───────────────────────────────────────
        let cx = wi / 4;
        let cy = (content_top + body_bottom) / 2;
        let radius = ((wi / 4 - m).min((body_bottom - content_top) / 2 - m)).max(10);
        self.draw_radar(&mut img, cx, cy, radius, data, fg);

        // ── Ranked list in the right half ────────────────────────────────
        let lx = wi / 2 + m;
        if data.nearby.is_empty() {
            let bx = wi * 3 / 4 - badge_width(&self.list, "NO AIRCRAFT") / 2;
            badge(&mut img, &self.list, bx, cy - self.list.height() / 2, "NO AIRCRAFT", fg, false);
        } else {
            draw_text(&mut img, &self.label, lx, content_top + self.label.ascent(), fg, "NEAREST");
            let mut y = content_top + self.label.height() + m / 2;
            for ac in data.nearby.iter().take(6) {
                let line = format!("{}  {:.0}km {}", ac.label(), ac.distance_km, bearing_octant(ac.bearing_deg));
                let line = fit_text(&self.list, &line, wi / 2 - 2 * m);
                draw_text(&mut img, &self.list, lx, y + self.list.ascent(), fg, &line);
                y += self.list.height() + m / 3;
            }
        }

        // Footer: closest readout (airline-decoded + heading octant).
        if let Some(c) = &data.closest {
            let alt = if c.on_ground { "GND".to_string() } else { format!("FL{:03}", c.altitude_ft / 100) };
            let hdg = c.heading_deg.map(|h| format!("  hdg {}", bearing_octant(h))).unwrap_or_default();
            footer(&mut img, &self.foot, fg, &format!("closest {}  {:.0}km  {alt}{hdg}", c.label(), c.distance_km));
        }
        img
    }

    /// Draw range rings, crosshair, the N marker, and the aircraft dots.
    fn draw_radar(&self, img: &mut RgbImage, cx: i32, cy: i32, r: i32, data: &FlightSnapshot, fg: Color) {
        for k in 1..=3 {
            draw_circle(img, cx, cy, r * k / 3, fg);
        }
        draw_line(img, cx - r, cy, cx + r, cy, fg);
        draw_line(img, cx, cy - r, cx, cy + r, fg);
        crate::matrix::eink::layout::center_text(img, &self.label, cx, cy - r - self.label.height() / 3, fg, "N");

        let radius_km = data.radius_km.max(1.0);
        for (i, ac) in data.nearby.iter().enumerate() {
            let frac = (ac.distance_km / radius_km).min(1.0);
            let rr = r as f32 * frac;
            let theta = ac.bearing_deg.to_radians();
            let px = cx + (rr * theta.sin()).round() as i32;
            let py = cy - (rr * theta.cos()).round() as i32;
            // 3px dot.
            crate::matrix::eink::layout::fill_rect(img, px - 1, py - 1, 3, 3, fg);
            // Heading vector — a short line in the aircraft's direction of
            // travel (north-up, clockwise), so you see where it's going.
            if let Some(h) = ac.heading_deg {
                let hr = h.to_radians();
                let len = (r / 6).max(6) as f32;
                let ex = px + (len * hr.sin()).round() as i32;
                let ey = py - (len * hr.cos()).round() as i32;
                draw_line(img, px, py, ex, ey, fg);
            }
            // Ring the nearest (first) so it reads without color.
            if i == 0 {
                draw_circle(img, px, py, (r / 12).max(3), fg);
            }
        }
    }
}

#[async_trait]
impl EinkRenderer for EinkFlightsMatrix {
    type Data = FlightSnapshot;

    fn id(&self) -> &'static str {
        "flights"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &FlightSnapshot) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::flights::model::FlightInfo;

    fn repo_fonts() -> EinkFlightsFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkFlightsFonts { body: base.join("04B_03B_.TTF") }
    }

    fn ac(call: &str, dist: f32, bearing: f32) -> FlightInfo {
        FlightInfo {
            callsign: call.into(),
            icao24: "a1b2c3".into(),
            altitude_ft: 34_000,
            on_ground: false,
            distance_km: dist,
            bearing_deg: bearing,
            ground_speed_kt: Some(420),
            heading_deg: Some(bearing),
            country: "United States".into(),
        }
    }

    fn busy() -> FlightSnapshot {
        let nearby = vec![ac("UAL123", 8.0, 45.0), ac("DAL456", 22.0, 210.0), ac("SWA789", 51.0, 300.0)];
        FlightSnapshot {
            count: nearby.len(),
            closest: Some(nearby[0].clone()),
            nearby,
            radius_km: 80.0,
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkFlightsMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&busy(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated radar, got {lit} lit px");
    }

    #[test]
    fn empty_renders_radar_and_badge() {
        let r = EinkFlightsMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let empty = FlightSnapshot { count: 0, closest: None, nearby: vec![], radius_km: 80.0 };
        let img = r.frame(&empty, 800, 480);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "empty airspace should still draw the radar + badge");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkFlightsMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&busy(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
