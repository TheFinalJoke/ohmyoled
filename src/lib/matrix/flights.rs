//! Flights renderer — radar (left) + ranked traffic list (right) for the
//! 64×32 panel.
//!
//! ```text
//!  ┌─── active airspace ──────────┐    ┌──── empty airspace ──────────┐
//!  │      N      ┊ DAL2451     12 │    │      N      ┊                │
//!  │   ▪ _|_     ┊ UAL989      28 │    │     _|_     ┊                │
//!  │  ---+--- ▪  ┊ JBU42       44 │    │  ---+---    ┊      NO AC     │
//!  │     |    ▪  ┊ AAL117      55 │    │     |       ┊                │
//!  │     | ▪     ┊                │    │     |       ┊                │
//!  └─────────────┴────────────────┘    └─────────────┴────────────────┘
//! ```
//!
//! - **Radar** (left, x=1..30): ring + crosshair + N marker. The sweep
//!   line rotates clockwise; dots pulse white as it passes.
//! - **Dots**: the four ranked aircraft are 2×2 colored blocks; their
//!   colors match the corresponding row in the right-side list, so a
//!   yellow dot is the yellow row, etc. Extras past rank 4 stay as
//!   single dim-grey pixels — "other traffic" present but not called
//!   out.
//! - **List** (right, x=33..63): up to four entries `CALL  DD` ordered
//!   closest → farthest. Distance is right-aligned to the panel edge;
//!   a long callsign that wouldn't fit in its row marquees left so the
//!   full identifier rolls past over time. The palette is ordered
//!   warm→cool, so the color itself also encodes distance (yellow=
//!   closest, orange, magenta, cyan=farthest tracked).
//! - **Quiet mode**: scope still animates, list area shows a centered
//!   "NO AC" badge.
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

use crate::api::flights::{FlightInfo, FlightSnapshot};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use image::{Rgb, RgbImage};
use ohmyoled_matrix::graphics::{draw_circle, draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

/// Radar geometry — left half of the panel, leaving x=32..63 for the
/// list. Center sits one pixel above mid-height so a 14-pixel radius
/// fits cleanly inside y=1..29 without clipping the ring.
const CX: i32 = 15;
const CY: i32 = 15;
const RADAR_R: i32 = 14;

/// Right-side list pane.
const LIST_X: i32 = 33;
const LIST_W: i32 = PANEL_W as i32 - LIST_X;
const LIST_ROWS: usize = 4;
const ROW_H: i32 = 8;

// Scope furniture.
const RING: Color = Color { r: 0, g: 60, b: 30 };
const CROSSHAIR: Color = Color { r: 25, g: 35, b: 25 };
const N_MARKER: Color = Color { r: 0, g: 180, b: 80 };
const SWEEP_HEAD: Color = Color { r: 80, g: 255, b: 120 };
const PING: Color = Color { r: 255, g: 255, b: 255 };
/// Separator between radar and list — a thin dim vertical line so the
/// two halves visually decouple without a heavy divider.
const DIVIDER: Color = Color { r: 30, g: 30, b: 30 };
const QUIET: Color = Color { r: 180, g: 200, b: 180 };

/// Per-rank palette — index 0 is the closest aircraft, index 3 is the
/// fourth-closest. The colors are spread around the wheel so adjacent
/// ranks read as different rows, and the ordering is warm→cool so the
/// color itself hints at how far away each plane is. Aircraft past
/// rank 3 (still inside `radius_km` but not in the list) use
/// `DOT_EXTRA` — dim grey "other traffic".
const RANK_COLORS: [Color; LIST_ROWS] = [
    Color { r: 255, g: 230, b: 90 },  // closest — warm yellow
    Color { r: 255, g: 130, b: 0 },   // orange
    Color { r: 230, g: 80, b: 220 },  // magenta
    Color { r: 0, g: 200, b: 255 },   // farthest tracked — cool cyan
];
const DOT_EXTRA: Color = Color { r: 100, g: 100, b: 100 };

const FRAME_TICK: Duration = Duration::from_millis(50);
/// Bearing degrees the sweep advances per frame. 4° × 20 fps = 80°/s,
/// so one full rotation takes ~4.5 s.
const SWEEP_STEP_DEG: f32 = 4.0;
/// How many "trail" pings to draw behind the sweep head. Each is one
/// `SWEEP_STEP_DEG` further behind and dimmer than the last.
const TRAIL_STEPS: u32 = 4;
/// Angular distance behind the sweep within which an aircraft dot
/// brightens toward white.
const PING_ARC_DEG: f32 = 22.0;
/// Renderer cycle length — ~2.6 sweep rotations, comparable to the
/// other modules' dwell times.
const CYCLE: Duration = Duration::from_secs(12);

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
        self.draw_frame(data, 0.0, 0)
    }

    pub fn draw_frame(&self, data: &FlightSnapshot, sweep_deg: f32, scroll_phase: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        self.draw_scope(&mut img);
        self.draw_sweep(&mut img, sweep_deg);
        self.draw_aircraft(&mut img, data, sweep_deg);
        self.draw_divider(&mut img);
        self.draw_list(&mut img, data, scroll_phase);
        img
    }

    /// Ring + crosshair + N marker — static scope furniture on the
    /// left half of the panel.
    fn draw_scope(&self, img: &mut RgbImage) {
        draw_line(img, CX - RADAR_R, CY, CX + RADAR_R, CY, CROSSHAIR);
        draw_line(img, CX, CY - RADAR_R, CX, CY + RADAR_R, CROSSHAIR);
        draw_circle(img, CX, CY, RADAR_R, RING);
        // Center pip — user position.
        put(img, CX, CY, RING);
        // "N" tucked just inside the top of the ring. Baseline at
        // CY - RADAR_R + 6 puts the glyph between the ring and the
        // first crosshair tick.
        draw_text(img, &self.body_font, CX - 1, CY - RADAR_R + 6, N_MARKER, "N");
    }

    /// Sweep head + a short fading trail behind it.
    fn draw_sweep(&self, img: &mut RgbImage, sweep_deg: f32) {
        for step in 0..=TRAIL_STEPS {
            let bearing = sweep_deg - (step as f32) * SWEEP_STEP_DEG;
            let factor = 1.0 - (step as f32) * (0.85 / (TRAIL_STEPS as f32));
            let color = dim(SWEEP_HEAD, factor);
            let (tx, ty) = bearing_to_screen(bearing, RADAR_R as f32);
            draw_line(img, CX, CY, CX + tx, CY + ty, color);
        }
    }

    /// One dot per aircraft. The first `LIST_ROWS` (the entries in the
    /// right-side list) get a 2×2 rank-colored block. Extras past rank
    /// 4 stay as a single dim grey pixel — "other traffic" that's
    /// present but not called out.
    fn draw_aircraft(&self, img: &mut RgbImage, data: &FlightSnapshot, sweep_deg: f32) {
        // Fall back to a 1 km scale on a bad config — the validator
        // guards this but the renderer shouldn't divide by zero.
        let scale = data.radius_km.max(1.0);
        let max_r = (RADAR_R - 1) as f32;

        for (i, f) in data.nearby.iter().enumerate() {
            let r_px = ((f.distance_km / scale) * max_r).clamp(0.0, max_r);
            let (dx, dy) = bearing_to_screen(f.bearing_deg, r_px);
            let x = CX + dx;
            let y = CY + dy;
            let base = rank_color(i);
            // Ping when the sweep just rolled past this bearing.
            let delta = angular_delta_behind(sweep_deg, f.bearing_deg);
            let color = if delta < PING_ARC_DEG {
                let t = 1.0 - delta / PING_ARC_DEG;
                blend(base, PING, t)
            } else {
                base
            };

            if i < LIST_ROWS {
                put_block_2x2(img, x, y, color);
            } else {
                put(img, x, y, color);
            }
        }
    }

    /// Thin vertical divider between radar and list — keeps the two
    /// halves visually distinct without a heavy line.
    fn draw_divider(&self, img: &mut RgbImage) {
        for y in 0..PANEL_H as i32 {
            put(img, LIST_X - 2, y, DIVIDER);
        }
    }

    /// Right-side list: up to `LIST_ROWS` ranked entries (`CALL  DD`),
    /// each in the same color as its dot on the radar. A callsign that
    /// overflows its row's callsign slot marquees left — the
    /// distance and divider stay put. Drops a centered "NO AC" badge
    /// instead when the airspace is empty.
    fn draw_list(&self, img: &mut RgbImage, data: &FlightSnapshot, scroll_phase: u32) {
        let body = &self.body_font;
        if data.nearby.is_empty() {
            let badge = "NO AC";
            let bw = body.text_width(badge);
            let bx = LIST_X + (LIST_W - bw) / 2;
            let by = PANEL_H as i32 / 2 + 3;
            draw_text(img, body, bx, by, QUIET, badge);
            return;
        }

        // Per-row layout: [callsign (scrolling if too long)][gap][dist]
        // with distance right-aligned to the panel edge.
        const GAP: i32 = 1;
        for (i, f) in data.nearby.iter().take(LIST_ROWS).enumerate() {
            let color = rank_color(i);
            let row_top = (i as i32) * ROW_H;
            let baseline = row_top + body.ascent();

            let dist_txt = format!("{}", f.distance_km.round().max(0.0) as i32);
            let dist_w = body.text_width(&dist_txt);
            let max_call_w = LIST_W - dist_w - GAP;

            // Distance first — right-aligned, fixed position so the
            // scrolling callsign doesn't disturb it.
            let dist_x = PANEL_W as i32 - dist_w;
            draw_text(img, body, dist_x, baseline, color, &dist_txt);

            let call = display_name(f);
            draw_scrolling_text(
                img,
                body,
                &call,
                LIST_X,
                row_top,
                max_call_w,
                color,
                scroll_phase,
            );
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
        CYCLE
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &FlightSnapshot) -> Result<(), RenderError> {
        matrix.clear();
        let frames = (CYCLE.as_millis() / FRAME_TICK.as_millis()) as u32;
        let mut sweep_deg = 0.0_f32;
        for frame_idx in 0..frames {
            matrix.set_image(&self.draw_frame(data, sweep_deg, frame_idx), 0, 0);
            sweep_deg = (sweep_deg + SWEEP_STEP_DEG).rem_euclid(360.0);
            tokio::time::sleep(FRAME_TICK).await;
        }
        matrix.clear();
        Ok(())
    }
}

/// Color for the aircraft at rank `i` (0 = closest). Past the visible
/// list, falls back to `DOT_EXTRA` so unranked traffic still shows on
/// the scope without competing with the four ranked dots.
fn rank_color(i: usize) -> Color {
    RANK_COLORS.get(i).copied().unwrap_or(DOT_EXTRA)
}

/// Bearing in degrees CW from north → screen-delta in pixels. North
/// (β=0) is `-y` (up); east (β=90) is `+x`.
fn bearing_to_screen(bearing_deg: f32, radius_px: f32) -> (i32, i32) {
    let rad = bearing_deg.to_radians();
    let dx = (radius_px * rad.sin()).round() as i32;
    let dy = (-radius_px * rad.cos()).round() as i32;
    (dx, dy)
}

/// Smallest signed angular distance the head has rotated *past* a
/// target bearing, in [0, 360).
fn angular_delta_behind(sweep_deg: f32, target_deg: f32) -> f32 {
    let d = (sweep_deg - target_deg).rem_euclid(360.0);
    if d < 0.0 {
        d + 360.0
    } else {
        d
    }
}

/// Per-channel multiply; saturates at 255.
fn dim(c: Color, factor: f32) -> Color {
    let f = factor.clamp(0.0, 1.0);
    Color {
        r: (c.r as f32 * f).round() as u8,
        g: (c.g as f32 * f).round() as u8,
        b: (c.b as f32 * f).round() as u8,
    }
}

/// Linear interpolate from `a` toward `b` by `t` ∈ [0, 1].
fn blend(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| -> u8 {
        let fx = x as f32;
        let fy = y as f32;
        (fx + (fy - fx) * t).round().clamp(0.0, 255.0) as u8
    };
    Color {
        r: lerp(a.r, b.r),
        g: lerp(a.g, b.g),
        b: lerp(a.b, b.b),
    }
}

/// Bounds-checked single-pixel plot. Out-of-range coords drop silently.
fn put(img: &mut RgbImage, x: i32, y: i32, c: Color) {
    if x >= 0 && x < img.width() as i32 && y >= 0 && y < img.height() as i32 {
        img.put_pixel(x as u32, y as u32, Rgb([c.r, c.g, c.b]));
    }
}

/// 2×2 block anchored at `(x, y)` as its top-left corner. Uses `put`
/// per pixel so out-of-range corners drop silently.
fn put_block_2x2(img: &mut RgbImage, x: i32, y: i32, c: Color) {
    put(img, x, y, c);
    put(img, x + 1, y, c);
    put(img, x, y + 1, c);
    put(img, x + 1, y + 1, c);
}

/// Pixel gap between repeats of a marqueeing text — the head of the
/// second copy follows this many empty columns after the tail of the
/// first, so the loop wrap looks intentional instead of joined.
const MARQUEE_GAP_PX: i32 = 6;
/// How many full passes of the text scroll past before the row
/// settles back to the head. Two passes lets the eye verify a
/// callsign it read on the first one; after that, freezing at the
/// head keeps the panel calm for the remainder of the cycle.
const MARQUEE_PASSES: i32 = 2;
/// Frames per scroll step. 1 = 1 px/frame (~20 px/sec on the 20 fps
/// panel); raising this slows the marquee proportionally so callsigns
/// are easier to read.
const SCROLL_FRAMES_PER_STEP: u32 = 2;

/// Draw `text` at `(dest_x, row_top)` clipped to `max_w` pixels. If
/// the rendered width fits, draws once; otherwise marquees left at
/// 1 px per `scroll_phase` step. Renders into a small temp image and
/// blits the visible window so nothing bleeds past `max_w` into
/// neighboring cells (radar, distance, divider).
#[allow(clippy::too_many_arguments)]
fn draw_scrolling_text(
    img: &mut RgbImage,
    font: &Font,
    text: &str,
    dest_x: i32,
    row_top: i32,
    max_w: i32,
    color: Color,
    scroll_phase: u32,
) {
    let text_w = font.text_width(text);
    if text_w <= max_w {
        draw_text(img, font, dest_x, row_top + font.ascent(), color, text);
        return;
    }
    let cycle = text_w + MARQUEE_GAP_PX;
    // Scroll for `MARQUEE_PASSES` full loops, then hold at offset 0
    // (head of text) for the rest of the render cycle. The hold is
    // what makes the row legible after the second pass — without it
    // the panel would never stop moving.
    let total_scroll = cycle * MARQUEE_PASSES;
    // Quantise the incoming frame counter so each scroll step lasts
    // `SCROLL_FRAMES_PER_STEP` frames — the marquee moves at
    // (1 / SCROLL_FRAMES_PER_STEP) px per frame.
    let phase = (scroll_phase / SCROLL_FRAMES_PER_STEP) as i32;
    let offset = if phase >= total_scroll {
        0
    } else {
        phase.rem_euclid(cycle)
    };
    // Render two consecutive copies of the text so a window that
    // straddles the wrap point still reads continuously.
    let temp_w = (cycle * 2) as u32;
    let row_h = ROW_H as u32;
    let mut temp = RgbImage::new(temp_w, row_h);
    let temp_baseline = font.ascent();
    draw_text(&mut temp, font, 0, temp_baseline, color, text);
    draw_text(&mut temp, font, cycle, temp_baseline, color, text);

    // Blit the [offset .. offset + max_w] slice of `temp` to the
    // panel. Only non-black pixels are copied so we don't erase the
    // radar or divider where the row leaves them visible.
    for x in 0..max_w {
        let src_x = offset + x;
        if src_x < 0 || src_x as u32 >= temp_w {
            continue;
        }
        for y in 0..row_h as i32 {
            let src = temp.get_pixel(src_x as u32, y as u32);
            if src.0 == [0, 0, 0] {
                continue;
            }
            let dx = dest_x + x;
            let dy = row_top + y;
            if dx >= 0 && dx < img.width() as i32 && dy >= 0 && dy < img.height() as i32 {
                img.put_pixel(dx as u32, dy as u32, *src);
            }
        }
    }
}

/// Prefer broadcast callsign, fall back to ICAO24 hex, never blank.
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

    fn flight(callsign: &str, bearing: f32, dist_km: f32) -> FlightInfo {
        FlightInfo {
            callsign: callsign.into(),
            icao24: "abc123".into(),
            altitude_ft: 32_000,
            on_ground: false,
            distance_km: dist_km,
            bearing_deg: bearing,
            ground_speed_kt: Some(450),
            heading_deg: Some(bearing),
            country: "United States".into(),
        }
    }

    fn snap_with(flights: Vec<FlightInfo>, radius_km: f32) -> FlightSnapshot {
        FlightSnapshot {
            count: flights.len(),
            closest: flights.first().cloned(),
            nearby: flights,
            radius_km,
        }
    }

    /// Right-half pixel rectangle — useful for asserting list content
    /// is or isn't where we expect, independent of the radar.
    fn right_half_lit(img: &RgbImage) -> usize {
        (0..PANEL_H)
            .flat_map(|y| (LIST_X as u32..PANEL_W).map(move |x| (x, y)))
            .filter(|&(x, y)| img.get_pixel(x, y).0 != [0, 0, 0])
            .count()
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let snap = snap_with(
            vec![
                flight("DAL2451", 225.0, 12.4),
                flight("UAL989", 0.0, 28.0),
                flight("JBU42", 90.0, 44.0),
            ],
            80.0,
        );
        let img = m.frame(&snap);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 100, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn quiet_mode_differs_from_active() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let active = m.frame(&snap_with(vec![flight("DAL2451", 225.0, 12.0)], 80.0));
        let quiet = m.frame(&snap_with(vec![], 80.0));
        assert_ne!(active.as_raw(), quiet.as_raw());
        // The NO AC badge should light some pixels in the right pane.
        assert!(right_half_lit(&quiet) > 10, "NO AC badge should light list pane");
    }

    #[test]
    fn empty_callsign_falls_back_to_icao() {
        let mut f = flight("", 90.0, 10.0);
        f.callsign = "".into();
        assert_eq!(display_name(&f), "ABC123", "ICAO24 hex uppercased as fallback");
    }

    #[test]
    fn dots_land_on_the_left_in_expected_quadrants() {
        // Bearing 0 (north) → above center on the left; bearing 90
        // (east) → toward the right edge of the radar (but still left
        // of the divider). Pick sweep angles 180° away so the ping
        // doesn't repaint the dot.
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let north = m.draw_frame(&snap_with(vec![flight("N1", 0.0, 40.0)], 80.0), 180.0, 0);
        let east = m.draw_frame(&snap_with(vec![flight("E1", 90.0, 40.0)], 80.0), 270.0, 0);

        let is_closest = |p: &Rgb<u8>| p.0 == [RANK_COLORS[0].r, RANK_COLORS[0].g, RANK_COLORS[0].b];
        // North dot should sit above CY, within the radar's x-band.
        let north_dot_above = (0..CY as u32)
            .flat_map(|y| (0..LIST_X as u32).map(move |x| (x, y)))
            .any(|(x, y)| is_closest(north.get_pixel(x, y)));
        // East dot should sit to the right of CX but still inside the
        // radar half (x < LIST_X - 2 divider).
        let east_dot_right_of_center = (0..PANEL_H)
            .flat_map(|y| ((CX + 1) as u32..(LIST_X - 2) as u32).map(move |x| (x, y)))
            .any(|(x, y)| is_closest(east.get_pixel(x, y)));
        assert!(north_dot_above, "north flight should plot above center");
        assert!(east_dot_right_of_center, "east flight should plot right of center");
    }

    #[test]
    fn flight_at_radius_edge_lands_inside_ring() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let snap = snap_with(vec![flight("EDGE", 0.0, 80.0)], 80.0);
        let img = m.draw_frame(&snap, 180.0, 0);
        let is_closest = |p: &Rgb<u8>| p.0 == [RANK_COLORS[0].r, RANK_COLORS[0].g, RANK_COLORS[0].b];
        // For a due-north flight at the configured radius, the dot
        // lands at y = CY - (RADAR_R - 1) = 2 inside the top of the ring.
        let target_y = (CY - (RADAR_R - 1)) as u32;
        let on_top_row = (0..LIST_X as u32).any(|x| is_closest(img.get_pixel(x, target_y)));
        assert!(on_top_row, "edge-of-radius dot should sit on the inner ring");
    }

    #[test]
    fn sweep_rotation_changes_frame() {
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let snap = snap_with(vec![flight("DAL2451", 225.0, 30.0)], 80.0);
        let f0 = m.draw_frame(&snap, 0.0, 0);
        let f90 = m.draw_frame(&snap, 90.0, 0);
        assert_ne!(f0.as_raw(), f90.as_raw(), "sweep angle should rotate visible pixels");
    }

    #[test]
    fn list_pane_reflects_flight_data() {
        // Different flight sets should produce different right-half
        // pixels — that's the contract for "the list shows the flights".
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let one = m.frame(&snap_with(vec![flight("DAL2451", 0.0, 10.0)], 80.0));
        let three = m.frame(&snap_with(
            vec![
                flight("DAL2451", 0.0, 10.0),
                flight("UAL989", 90.0, 28.0),
                flight("JBU42", 180.0, 44.0),
            ],
            80.0,
        ));
        // More flights → strictly more lit pixels in the list pane.
        assert!(
            right_half_lit(&three) > right_half_lit(&one),
            "extra flights should add rows to the list pane"
        );
    }

    #[test]
    fn ranked_dots_use_distinct_colors() {
        // Each of the four ranks should produce its palette color on
        // the scope. Pick a sweep far from all four bearings to avoid
        // ping-blending.
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let snap = snap_with(
            vec![
                flight("R0", 0.0, 20.0),
                flight("R1", 90.0, 30.0),
                flight("R2", 180.0, 40.0),
                flight("R3", 270.0, 50.0),
            ],
            80.0,
        );
        // Sweep at 45° — at least 45° from each cardinal, well outside
        // the 22° ping arc.
        let img = m.draw_frame(&snap, 45.0, 0);
        for (i, expected) in RANK_COLORS.iter().enumerate() {
            let needle = [expected.r, expected.g, expected.b];
            let found = img.pixels().any(|p| p.0 == needle);
            assert!(found, "rank {i} color {expected:?} not found on scope");
        }
    }

    #[test]
    fn extra_aircraft_beyond_list_render_as_dim_grey() {
        // The 5th-closest flight isn't in the list, but it should
        // still appear as a `DOT_EXTRA` pixel on the scope.
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut flights = vec![
            flight("R0", 0.0, 10.0),
            flight("R1", 45.0, 20.0),
            flight("R2", 135.0, 30.0),
            flight("R3", 225.0, 40.0),
        ];
        flights.push(flight("R4", 315.0, 50.0));
        let img = m.draw_frame(&snap_with(flights, 80.0), 90.0, 0);
        let needle = [DOT_EXTRA.r, DOT_EXTRA.g, DOT_EXTRA.b];
        assert!(
            img.pixels().any(|p| p.0 == needle),
            "fifth flight should appear as a dim grey dot"
        );
    }

    #[test]
    fn ranked_dot_is_a_2x2_block() {
        // A ranked aircraft's dot occupies four pixels in its palette
        // color. Sweep 180° away from the bearing so the ping doesn't
        // blend the block toward white.
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let only_ranked = snap_with(vec![flight("R0", 180.0, 30.0)], 80.0);
        let img = m.draw_frame(&only_ranked, 0.0, 0);
        let needle = [RANK_COLORS[0].r, RANK_COLORS[0].g, RANK_COLORS[0].b];
        // Restrict to the radar half so the list row's text doesn't
        // skew the count.
        let yellow_on_radar = (0..PANEL_H)
            .flat_map(|y| (0..(LIST_X - 2) as u32).map(move |x| (x, y)))
            .filter(|&(x, y)| img.get_pixel(x, y).0 == needle)
            .count();
        assert_eq!(
            yellow_on_radar, 4,
            "expected exactly the 2×2 block on the radar, got {yellow_on_radar}"
        );
    }

    #[test]
    fn long_callsign_scrolls_in_list_pane() {
        // A callsign wider than the row's text slot should marquee —
        // different scroll phases must shift the visible pixels in the
        // list pane. Short callsigns stay put.
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let snap = snap_with(vec![flight("SUPERCALIFRAGILISTIC", 45.0, 20.0)], 80.0);
        let f0 = m.draw_frame(&snap, 0.0, 0);
        let f10 = m.draw_frame(&snap, 0.0, 10);
        let pane_diff = (LIST_X as u32..PANEL_W)
            .flat_map(|x| (0..PANEL_H).map(move |y| (x, y)))
            .filter(|&(x, y)| f0.get_pixel(x, y) != f10.get_pixel(x, y))
            .count();
        assert!(
            pane_diff > 5,
            "scrolling callsign should shift list-pane pixels, got {pane_diff}"
        );
    }

    #[test]
    fn long_callsign_settles_after_two_passes() {
        // Once the marquee has run twice, the row should stop moving:
        // any two frames captured past the settle point produce
        // identical list-pane pixels.
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let snap = snap_with(vec![flight("SUPERCALIFRAGILISTIC", 45.0, 20.0)], 80.0);
        // 1000 is comfortably past 2 × cycle even for the longest
        // strings the renderer will see (text_width capped by font
        // glyph widths × char count).
        let late_a = m.draw_frame(&snap, 0.0, 1000);
        let late_b = m.draw_frame(&snap, 0.0, 1500);
        let pane_diff = (LIST_X as u32..PANEL_W)
            .flat_map(|x| (0..PANEL_H).map(move |y| (x, y)))
            .filter(|&(x, y)| late_a.get_pixel(x, y) != late_b.get_pixel(x, y))
            .count();
        assert_eq!(
            pane_diff, 0,
            "marquee should settle after two passes; pane still moving ({pane_diff} diff)"
        );
    }

    #[test]
    fn short_callsign_does_not_scroll() {
        // A row that fits doesn't need marquee — scroll phase shouldn't
        // change its rendered pixels.
        let m = FlightsMatrix::with_fonts(repo_fonts()).expect("fonts");
        let snap = snap_with(vec![flight("N12", 45.0, 20.0)], 80.0);
        let f0 = m.draw_frame(&snap, 180.0, 0);
        let f10 = m.draw_frame(&snap, 180.0, 10);
        let pane_diff = (LIST_X as u32..PANEL_W)
            .flat_map(|x| (0..PANEL_H).map(move |y| (x, y)))
            .filter(|&(x, y)| f0.get_pixel(x, y) != f10.get_pixel(x, y))
            .count();
        assert_eq!(
            pane_diff, 0,
            "short callsign should be static across scroll phases"
        );
    }
}
