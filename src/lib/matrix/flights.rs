//! Flights renderer — closest-aircraft tile for the 64×32 panel.
//!
//! ```text
//!  ┌─── active airspace ────────┐    ┌──── empty airspace ────────┐
//!  │ AC 7 NEAR                  │    │      QUIET SKIES           │
//!  │                            │    │                            │
//!  │ DAL2451                    │    │      no flights            │
//!  │                            │    │      in range              │
//!  │ FL320  • 12km SW           │    │                            │
//!  └────────────────────────────┘    └────────────────────────────┘
//! ```
//!
//! - **Header**: `AC N NEAR` — count of aircraft within the configured
//!   radius. White; the "AC" stays put as a label anchor.
//! - **Callsign**: cyan, top-aligned. Falls back to ICAO24 hex when
//!   the transponder hasn't broadcast a callsign yet. Marquees when
//!   the rendered text overflows 64 px (rare — most callsigns fit).
//! - **Footer**: dim grey, flight level / distance / bearing. `FL000`
//!   suppressed when the aircraft is `on_ground` — shows `GND` instead.
//!
//! # Config
//!
//! ```yaml
//! flights:
//!   run: true
//!   lat: 40.7128
//!   lon: -74.0060
//!   radius_km: 80
//! ```
//!
//! # Data source
//!
//! `FlightsCollector::from_opensky` — anonymous bbox query against
//! `https://opensky-network.org/api/states/all`, post-filtered by
//! Haversine distance to enforce a circular radius. 60 s refresh.

use crate::api::flights::{bearing_octant, FlightInfo, FlightSnapshot};
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

const HEADER: Color = Color { r: 255, g: 255, b: 255 };
const CALLSIGN: Color = Color { r: 0, g: 220, b: 255 };
const FOOTER: Color = Color { r: 150, g: 150, b: 150 };
const QUIET: Color = Color { r: 180, g: 180, b: 180 };

const SCROLL_TICK: Duration = Duration::from_millis(50);
const SETTLE_DWELL: Duration = Duration::from_secs(2);
const FINAL_DWELL: Duration = Duration::from_secs(3);
const STATIC_DWELL: Duration = Duration::from_secs(12);
/// Extra pixels the callsign slides past the right edge during a marquee
/// pass — keeps the trailing edge from snapping mid-letter.
const MARQUEE_OVERSCROLL: i32 = 8;

#[derive(Debug, Clone)]
pub struct FlightsFonts {
    pub body: PathBuf,
}

impl Default for FlightsFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
        }
    }
}

pub struct FlightsMatrix {
    body_font: Font,
}

impl FlightsMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(FlightsFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(FlightsFonts::default()).await
    }

    pub fn with_fonts(paths: FlightsFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
        })
    }

    pub async fn with_fonts_async(paths: FlightsFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    pub fn frame(&self, data: &FlightSnapshot) -> RgbImage {
        self.draw_frame(data, 0)
    }

    /// Render one frame. `scroll_px` only affects the callsign row — the
    /// header and footer are static so the eye can lock on them while
    /// the long callsign (if any) marquees past.
    pub fn draw_frame(&self, data: &FlightSnapshot, scroll_px: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        if let Some(closest) = &data.closest {
            self.draw_active(&mut img, data.count, closest, scroll_px);
        } else {
            self.draw_quiet(&mut img);
        }
        img
    }

    fn draw_active(&self, img: &mut RgbImage, count: usize, f: &FlightInfo, scroll_px: i32) {
        let body = &self.body_font;
        let line_h = body.height().max(body.ascent() + 1);

        // Row 1: "AC N NEAR" header, white.
        let header = format!("AC {} NEAR", count);
        draw_text(img, body, 1, body.ascent(), HEADER, &header);

        // Row 2: callsign in cyan, marquees if it'd overflow. The display
        // name is the callsign when present; fall back to the ICAO24 hex
        // so the row never goes blank for a known aircraft.
        let display = display_name(f);
        let callsign_y = body.ascent() + line_h + 2;
        let callsign_x = 1 - scroll_px;
        draw_text(img, body, callsign_x, callsign_y, CALLSIGN, &display);

        // Row 3 (bottom): "FL320 • 12km SW" footer. On the ground,
        // FL000 would be misleading — show "GND" instead.
        let level = if f.on_ground {
            "GND".to_string()
        } else {
            format!("FL{:03}", f.altitude_ft / 100)
        };
        let dist = format!("{}km", f.distance_km.round().max(0.0) as i32);
        let dir = bearing_octant(f.bearing_deg);
        let footer = format!("{level} {dist} {dir}");
        let footer_y = (PANEL_H as i32) - 1;
        draw_text(img, body, 1, footer_y, FOOTER, &footer);
    }

    fn draw_quiet(&self, img: &mut RgbImage) {
        let body = &self.body_font;
        let line_h = body.height().max(body.ascent() + 1);
        let banner = "QUIET SKIES";
        let sub_a = "no flights";
        let sub_b = "in range";

        let banner_w = body.text_width(banner);
        let sub_a_w = body.text_width(sub_a);
        let sub_b_w = body.text_width(sub_b);

        let content = 3 * line_h + 4;
        let top_pad = ((PANEL_H as i32 - content) / 2).max(0);
        let banner_y = top_pad + body.ascent();
        let sub_a_y = banner_y + line_h + 4;
        let sub_b_y = sub_a_y + line_h;

        draw_text(img, body, ((PANEL_W as i32 - banner_w) / 2).max(0), banner_y, QUIET, banner);
        draw_text(img, body, ((PANEL_W as i32 - sub_a_w) / 2).max(0), sub_a_y, FOOTER, sub_a);
        draw_text(img, body, ((PANEL_W as i32 - sub_b_w) / 2).max(0), sub_b_y, FOOTER, sub_b);
    }

    /// Marquee distance for the callsign row, in pixels. 0 when the
    /// callsign fits without scrolling.
    fn callsign_scroll_distance(&self, f: &FlightInfo) -> i32 {
        let display = display_name(f);
        let w = self.body_font.text_width(&display);
        if w <= PANEL_W as i32 - 2 {
            0
        } else {
            w + MARQUEE_OVERSCROLL
        }
    }
}

impl Default for FlightsMatrix {
    fn default() -> Self {
        Self::new().expect("default FlightsMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for FlightsMatrix {
    type Data = FlightSnapshot;

    fn id(&self) -> &'static str {
        "flights"
    }

    fn cycle_duration(&self) -> Duration {
        STATIC_DWELL
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &FlightSnapshot) -> Result<(), RenderError> {
        matrix.clear();

        let scroll = match &data.closest {
            Some(f) => self.callsign_scroll_distance(f),
            None => 0,
        };

        // Settled frame first.
        matrix.set_image(&self.draw_frame(data, 0), 0, 0);
        tokio::time::sleep(SETTLE_DWELL).await;

        if scroll > 0 {
            for offset in 1..=scroll {
                matrix.set_image(&self.draw_frame(data, offset), 0, 0);
                tokio::time::sleep(SCROLL_TICK).await;
            }
            // Snap back to the start so the trailing dwell shows the head.
            matrix.set_image(&self.draw_frame(data, 0), 0, 0);
            tokio::time::sleep(FINAL_DWELL).await;
        } else {
            tokio::time::sleep(STATIC_DWELL - SETTLE_DWELL).await;
        }

        matrix.clear();
        Ok(())
    }
}

/// Choose what to render on the callsign row — prefer the broadcast
/// callsign, fall back to the ICAO24 hex (always present), and never
/// leave the row blank.
fn display_name(f: &FlightInfo) -> String {
    if !f.callsign.is_empty() {
        f.callsign.clone()
    } else if !f.icao24.is_empty() {
        f.icao24.to_uppercase()
    } else {
        "—".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_fonts() -> FlightsFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        FlightsFonts {
            body: repo.join("04B_03B_.TTF"),
        }
    }

    fn sample_flight() -> FlightInfo {
        FlightInfo {
            callsign: "DAL2451".into(),
            icao24: "abc123".into(),
            altitude_ft: 32000,
            on_ground: false,
            distance_km: 12.4,
            bearing_deg: 225.0,
            ground_speed_kt: Some(450),
            country: "United States".into(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let snap = FlightSnapshot {
            count: 7,
            closest: Some(sample_flight()),
        };
        let img = m.frame(&snap);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn quiet_mode_differs_from_active() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let active = m.frame(&FlightSnapshot {
            count: 5,
            closest: Some(sample_flight()),
        });
        let quiet = m.frame(&FlightSnapshot {
            count: 0,
            closest: None,
        });
        assert_ne!(active.as_raw(), quiet.as_raw());
        // The quiet banner should light cyan/dim grey pixels in the middle
        // rows where "QUIET SKIES" would be drawn.
        let mid_lit: usize = (10..22u32)
            .flat_map(|y| (0..PANEL_W).map(move |x| (x, y)))
            .filter(|&(x, y)| quiet.get_pixel(x, y).0 != [0, 0, 0])
            .count();
        assert!(mid_lit > 20, "quiet banner should light middle rows");
    }

    #[test]
    fn on_ground_shows_gnd_not_fl000() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut f = sample_flight();
        f.on_ground = true;
        f.altitude_ft = 0;
        // Snapshot rendered "on the ground" should differ from the same
        // flight rendered in the air (FL320 vs GND footer text).
        let on_ground = m.frame(&FlightSnapshot { count: 1, closest: Some(f.clone()) });
        f.on_ground = false;
        f.altitude_ft = 32000;
        let in_air = m.frame(&FlightSnapshot { count: 1, closest: Some(f) });
        assert_ne!(on_ground.as_raw(), in_air.as_raw(), "GND footer should differ from FLnnn");
    }

    #[test]
    fn long_callsign_triggers_scroll() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut f = sample_flight();
        f.callsign = "ABCDEFGHIJKLMNOP".into(); // way wider than 64 px
        let dist = m.callsign_scroll_distance(&f);
        assert!(dist > 0, "long callsign should marquee");
        // Mid-marquee, the callsign row pixels should differ from settled.
        let snap = FlightSnapshot { count: 3, closest: Some(f) };
        let settled = m.frame(&snap);
        let mid = m.draw_frame(&snap, dist / 2);
        let row_diff: usize = (8..18u32)
            .flat_map(|y| (0..PANEL_W).map(move |x| (x, y)))
            .filter(|&(x, y)| settled.get_pixel(x, y) != mid.get_pixel(x, y))
            .count();
        assert!(row_diff > 10, "callsign should visibly shift mid-marquee");
    }

    #[test]
    fn short_callsign_does_not_scroll() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let f = sample_flight();
        assert_eq!(m.callsign_scroll_distance(&f), 0);
    }

    #[test]
    fn empty_callsign_falls_back_to_icao() {
        let mut f = sample_flight();
        f.callsign = "".into();
        assert_eq!(display_name(&f), "ABC123", "ICAO24 hex uppercased as fallback");
    }
}
