//! Weather renderer — pure-Rust port of `src/python/ohmyoled/matrix/weathermatrix.py`.
//!
//! Two scrolling screens, each ~100 frames at 0.05s + a long dwell. Screen 1
//! shows temp/feels-like/high/low + weather icon + location/conditions. Screen 2
//! shows humidity/wind/sunrise/sunset + icon + location.
//!
//! # Layout (64×32) — screen 1
//!
//! ```text
//!   ┌─────────────────────── (location scrolls) ─────── icon ───┐
//!   │ T:70F          ┊ R:68F                              [☀]   │ row y_top=8
//!   │ H:75F          ┊      L:55F                               │ row y_top=18
//!   │ Conditions: Partly Cloudy (scrolls)                       │ row y_top=26
//!   └────────────────┴──────────────────────────────────────────┘
//! ```
//!
//! Each temperature row is split into two invisible cells (with a tiny
//! barrier between them) so extreme values like `100F` or `-12F` marquee
//! **within** their cell and never bleed into the adjacent one. Screen 2
//! swaps the temp column for humidity / wind / sunrise / sunset; the
//! humidity row reuses the same cell pattern. Temp color: blue ⇐ cold,
//! white ⇐ mild, red ⇐ hot.
//!
//! # Config
//!
//! Weather is a list — each entry maps one provider to one rendered slot,
//! so multi-location displays are just multiple entries.
//!
//! ```yaml
//! weather:
//!   - run: true
//!     api: openweather              # openweather | nws | accuweather | pirate
//!     api_key: YOUR_KEY             # not required for nws
//!     current_location: true
//!     current_location_api_key: YOUR_IPINFO_TOKEN  # needed if current_location
//!     city: "Boston, MA"            # alternative to current_location
//!     weather_format: imperial      # imperial | metric
//! ```
//!
//! # Data sources
//!
//! - `openweather` → OneCall 3.0 (`api.openweathermap.org`)
//! - `nws` → US-only, no API key (`api.weather.gov`, two-step: points then forecast)
//! - `accuweather` → current conditions + 5-day forecast
//! - `pirate` → Pirate Weather drop-in for Dark Sky
//!
//! All providers normalize to the same `Weather` shape; icon table is shared
//! via OWM codes (NWS condition strings are mapped to OWM-equivalent codes).
//! Refresh interval: 10 minutes.

use crate::api::weather::model::{Weather, WindDirection};
use crate::matrix::cells::{draw_pieces_in_box, overflow_period, Align, Piece};
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

const SCROLL_FRAMES: u32 = 100;
const SCROLL_TICK: Duration = Duration::from_millis(50);
const FIRST_FRAME_DWELL: Duration = Duration::from_secs(3);
const SCREEN1_DWELL: Duration = Duration::from_secs(25);
const SCREEN2_DWELL: Duration = Duration::from_secs(30);

// --- Invisible cell layout --------------------------------------------------
// Row 1 (temp/humidity, y_top=8) lives alongside the weather icon at x≥50, so
// the two cells share x=[0, 49) with a 1-px gutter at the midline. Row 2
// (high/low, y_top=18) has no icon, but its cells start at the SAME x as row
// 1's so the H/L labels sit directly under the T/R labels above them. The
// right cell on row 2 then extends further (no icon to leave room for), which
// gives extreme values like -12F more marquee headroom.
const ROW1_LEFT_X: i32 = 0;
const ROW1_BOX_W: i32 = 24;
const ROW1_RIGHT_X: i32 = 25;
const ROW1_RIGHT_W: i32 = 24;

const ROW2_LEFT_X: i32 = 0;
const ROW2_BOX_W: i32 = 24;
const ROW2_RIGHT_X: i32 = 25;
const ROW2_RIGHT_W: i32 = 39;

/// Paths to the three fonts the renderer needs.
///
/// The defaults match what `src/sh/install.sh` lays down in `/usr/share/fonts/`.
/// `temp` is the pixel font used for temperature and humidity readouts — BMmini
/// at 8pt gives clean digits with breathing room around the trailing `F`, where
/// the previous `retro_computer` 7pt jammed glyphs together.
#[derive(Debug, Clone)]
pub struct WeatherFonts {
    pub body: PathBuf,
    pub icon: PathBuf,
    pub temp: PathBuf,
}

impl Default for WeatherFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
            icon: "/usr/share/fonts/weathericons.ttf".into(),
            temp: "/usr/share/fonts/BMmini.TTF".into(),
        }
    }
}

/// Weather renderer state. Owns the four TTF fonts.
pub struct WeatherMatrix {
    body_font: Font,
    /// Inline weather glyph next to wind on screen 2 — kept small so it fits
    /// in the tight vertical space between the wind text and the sunrise row.
    body_icon_font: Font,
    /// Bigger version of the same icon font, used for the prominent
    /// top-right corner icon on screens 1 and 2.
    corner_icon_font: Font,
    big_icon_font: Font,
    temp_font: Font,
}

impl WeatherMatrix {
    /// Build with default font paths (`/usr/share/fonts/...`).
    ///
    /// Synchronous — prefer [`Self::new_async`] inside a tokio runtime.
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(WeatherFonts::default())
    }

    /// Async constructor — loads the three TTF fonts on a tokio worker thread.
    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(WeatherFonts::default()).await
    }

    /// Build with caller-supplied font paths.
    pub fn with_fonts(paths: WeatherFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
            // 9pt for the inline wind glyph (tight vertical clearance);
            // 13pt for the prominent top-right corner icon;
            // 11pt for the sunrise/sunset glyphs.
            body_icon_font: Font::load_ttf(&paths.icon, 9.0)?,
            corner_icon_font: Font::load_ttf(&paths.icon, 13.0)?,
            big_icon_font: Font::load_ttf(&paths.icon, 11.0)?,
            // BMmini at 8pt — pixel font with a visible gap before the trailing
            // F. Keep the file in sync with install.sh.
            temp_font: Font::load_ttf(&paths.temp, 8.0)?,
        })
    }

    /// Async variant of [`Self::with_fonts`].
    pub async fn with_fonts_async(paths: WeatherFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }
}

impl Default for WeatherMatrix {
    fn default() -> Self {
        Self::new().expect("default WeatherMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for WeatherMatrix {
    type Data = Weather;

    fn id(&self) -> &'static str {
        "weather"
    }

    fn cycle_duration(&self) -> Duration {
        // Screen 1: 100 * 0.05s + 25s dwell = 30s
        // Screen 2: 100 * 0.05s + 30s dwell = 35s
        // Total ≈ 65s
        Duration::from_secs(65)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &Weather) -> Result<(), RenderError> {
        matrix.clear();

        // Screen 1: temp + icon + scrolling location/conditions.
        let screen1_frames = SCROLL_FRAMES.max(self.screen_one_scroll_frames(data));
        for xpos in 0..screen1_frames {
            let img = self.draw_screen_one(data, xpos as i32);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(if xpos == 0 { FIRST_FRAME_DWELL } else { SCROLL_TICK }).await;
        }
        let img = self.draw_screen_one(data, 0);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(SCREEN1_DWELL).await;
        matrix.clear();

        // Screen 2: humidity + wind + sunrise/sunset + icon + scrolling location.
        let screen2_frames = SCROLL_FRAMES.max(self.screen_two_scroll_frames(data));
        for xpos in 0..screen2_frames {
            let img = self.draw_screen_two(data, xpos as i32);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(if xpos == 0 { FIRST_FRAME_DWELL } else { SCROLL_TICK }).await;
        }
        let img = self.draw_screen_two(data, 0);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(SCREEN2_DWELL).await;
        matrix.clear();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Drawing — pure functions on a fresh RgbImage. No matrix interaction here so
// these are unit-testable.
// ---------------------------------------------------------------------------

impl WeatherMatrix {
    /// Render screen 1 (temp/icon/location) at the given horizontal scroll offset.
    /// Public for use in examples and visual regression checks.
    pub fn draw_screen_one(&self, data: &Weather, xpos: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let frame = xpos.max(0) as u32;
        self.render_temp(&mut img, data, frame);
        self.render_icon(&mut img, data);
        self.render_location(&mut img, data, xpos);
        self.render_conditions(&mut img, data, xpos);
        img
    }

    /// Render screen 2 (humidity/wind/sunrise/sunset) at the given scroll offset.
    pub fn draw_screen_two(&self, data: &Weather, xpos: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let frame = xpos.max(0) as u32;
        self.render_location(&mut img, data, xpos);
        self.render_icon(&mut img, data);
        self.render_humidity(&mut img, data, frame);
        self.render_wind(&mut img, data);
        self.render_time(&mut img, data);
        img
    }

    /// Frames needed for two marquee passes of the widest overflowing cell on
    /// screen 1. Returns 0 when every cell fits — callers `.max(SCROLL_FRAMES)`
    /// it so the location/conditions strip still scrolls regardless.
    pub fn screen_one_scroll_frames(&self, data: &Weather) -> u32 {
        let f = &self.temp_font;
        let widths = [
            (
                f.text_width("T:") + f.text_width(&format!("{}F", data.current.temp.round() as i32)),
                ROW1_BOX_W,
            ),
            (
                f.text_width("R:") + f.text_width(&format!("{}F", data.current.feels_like.round() as i32)),
                ROW1_RIGHT_W,
            ),
            (
                f.text_width("H:") + f.text_width(&format!("{}F", data.forecast.today_high.round() as i32)),
                ROW2_BOX_W,
            ),
            (
                f.text_width("L:") + f.text_width(&format!("{}F", data.forecast.today_low.round() as i32)),
                ROW2_RIGHT_W,
            ),
        ];
        max_double_period(&widths)
    }

    /// Same idea for screen 2 — only the humidity row uses cells today.
    pub fn screen_two_scroll_frames(&self, data: &Weather) -> u32 {
        let f = &self.body_font;
        let widths = [
            (
                f.text_width("H:") + f.text_width(&format!("{}%", data.current.humidity)),
                ROW1_BOX_W,
            ),
            (
                f.text_width("P:") + f.text_width(&format!("{}%", data.current.precipitation_chance)),
                ROW1_RIGHT_W,
            ),
        ];
        max_double_period(&widths)
    }

    fn render_temp(&self, img: &mut RgbImage, data: &Weather, frame: u32) {
        let font = &self.temp_font;
        let baseline_row1 = top_to_baseline(8, font.ascent());
        let baseline_row2 = top_to_baseline(18, font.ascent());

        let temp_str = format!("{}F", data.current.temp.round() as i32);
        let feels_str = format!("{}F", data.current.feels_like.round() as i32);
        let high_str = format!("{}F", data.forecast.today_high.round() as i32);
        let low_str = format!("{}F", data.forecast.today_low.round() as i32);

        // Row 1 (y_top=8): T:<temp> | R:<feels>   (icon overlaps right edge)
        draw_pieces_in_box(
            img,
            font,
            ROW1_LEFT_X,
            ROW1_BOX_W,
            baseline_row1,
            Align::Left,
            &[
                Piece { text: "T:", color: Color::WHITE },
                Piece { text: &temp_str, color: temp_color(data.current.temp) },
            ],
            0,
            frame,
        );
        draw_pieces_in_box(
            img,
            font,
            ROW1_RIGHT_X,
            ROW1_RIGHT_W,
            baseline_row1,
            Align::Left,
            &[
                Piece { text: "R:", color: Color::WHITE },
                Piece { text: &feels_str, color: temp_color(data.current.feels_like) },
            ],
            0,
            frame,
        );

        // Row 2 (y_top=18): H:<high> | L:<low>   (full width, no icon)
        draw_pieces_in_box(
            img,
            font,
            ROW2_LEFT_X,
            ROW2_BOX_W,
            baseline_row2,
            Align::Left,
            &[
                Piece { text: "H:", color: Color::WHITE },
                Piece { text: &high_str, color: temp_color(data.forecast.today_high) },
            ],
            0,
            frame,
        );
        draw_pieces_in_box(
            img,
            font,
            ROW2_RIGHT_X,
            ROW2_RIGHT_W,
            baseline_row2,
            Align::Left,
            &[
                Piece { text: "L:", color: Color::WHITE },
                Piece { text: &low_str, color: temp_color(data.forecast.today_low) },
            ],
            0,
            frame,
        );
    }

    fn render_icon(&self, img: &mut RgbImage, data: &Weather) {
        let glyph = data.current.icon.glyph.to_string();
        let color = data.current.icon.color;
        // Center the glyph in the right-hand strip (x=50..63, 14 px wide).
        let font = &self.corner_icon_font;
        let glyph_w = font.text_width(&glyph);
        let x = 50 + ((14 - glyph_w) / 2).max(0);
        let baseline = top_to_baseline(0, font.ascent());
        draw_text(img, font, x, baseline, color, &glyph);
    }

    fn render_location(&self, img: &mut RgbImage, data: &Weather, xpos: i32) {
        let baseline = top_to_baseline(1, self.body_font.ascent());
        draw_text(
            img,
            &self.body_font,
            -xpos,
            baseline,
            Color::new(0, 254, 0),
            &data.location_name,
        );
    }

    fn render_conditions(&self, img: &mut RgbImage, data: &Weather, xpos: i32) {
        let baseline = top_to_baseline(26, self.body_font.ascent());
        let text = format!("Conditions: {}", data.current.conditions);
        draw_text(img, &self.body_font, -xpos, baseline, Color::WHITE, &text);
    }

    fn render_humidity(&self, img: &mut RgbImage, data: &Weather, frame: u32) {
        let font = &self.body_font;
        let baseline = top_to_baseline(8, font.ascent());
        let humidity_str = format!("{}%", data.current.humidity);
        let precip_str = format!("{}%", data.current.precipitation_chance);
        let cyan = Color::new(7, 250, 246);

        // Same two-cell pattern as the temp row; icon overlaps the right edge.
        draw_pieces_in_box(
            img,
            font,
            ROW1_LEFT_X,
            ROW1_BOX_W,
            baseline,
            Align::Left,
            &[
                Piece { text: "H:", color: Color::WHITE },
                Piece { text: &humidity_str, color: cyan },
            ],
            0,
            frame,
        );
        draw_pieces_in_box(
            img,
            font,
            ROW1_RIGHT_X,
            ROW1_RIGHT_W,
            baseline,
            Align::Left,
            &[
                Piece { text: "P:", color: Color::WHITE },
                Piece { text: &precip_str, color: cyan },
            ],
            0,
            frame,
        );
    }

    fn render_wind(&self, img: &mut RgbImage, data: &Weather) {
        //  wind glyph (9pt).
        let wind_glyph_baseline = top_to_baseline(12, self.body_icon_font.ascent());
        draw_text(
            img,
            &self.body_icon_font,
            1,
            wind_glyph_baseline,
            Color::WHITE,
            "\u{f050}",
        );

        let text_baseline = top_to_baseline(15, self.body_font.ascent());
        let dir_text = data
            .current
            .wind_direction_deg
            .map(|d| WindDirection::from_degrees(d).as_str())
            .unwrap_or("---");
        draw_text(
            img,
            &self.body_font,
            15,
            text_baseline,
            Color::new(201, 1, 253),
            dir_text,
        );
        draw_text(
            img,
            &self.body_font,
            36,
            text_baseline,
            Color::new(201, 1, 253),
            &format!("{}mph", data.current.wind_speed.round() as i32),
        );
    }

    fn render_time(&self, img: &mut RgbImage, data: &Weather) {
        //  sunrise (yellow),  sunset (orange) — 11pt big icons.
        // Sit just above the time row so the icons read as paired with their
        // times; the sunrise glyph's left side bearing extends to x=0, so we
        // start at x=2 to keep its leading edge inside the panel.
        let sunrise_glyph_baseline = top_to_baseline(22, self.big_icon_font.ascent());
        let sunset_glyph_baseline = top_to_baseline(21, self.big_icon_font.ascent());
        draw_text(
            img,
            &self.big_icon_font,
            2,
            sunrise_glyph_baseline,
            Color::new(255, 255, 0),
            "\u{f058}",
        );
        draw_text(
            img,
            &self.big_icon_font,
            35,
            sunset_glyph_baseline,
            Color::new(255, 145, 0),
            "\u{f044}",
        );

        // Sunrise / sunset times beside each glyph.
        let text_baseline = top_to_baseline(23, self.body_font.ascent());
        let sunrise_str = data.forecast.sunrise.format("%H:%M").to_string();
        let sunset_str = data.forecast.sunset.format("%H:%M").to_string();
        draw_text(img, &self.body_font, 7, text_baseline, Color::WHITE, &sunrise_str);
        draw_text(img, &self.body_font, 40, text_baseline, Color::WHITE, &sunset_str);
    }
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// PIL `ImageDraw.text((x, y), ...)` interprets `y` as the *top* of the glyph;
/// our `draw_text` takes a baseline. This converts top-y → baseline-y.
fn top_to_baseline(top_y: i32, ascent: i32) -> i32 {
    top_y + ascent
}

/// Twice the longest marquee period across the given `(unit_w, box_w)` cells,
/// or 0 if everything fits. Used to size the scroll phase so any overflowing
/// cell gets exactly two full passes.
fn max_double_period(cells: &[(i32, i32)]) -> u32 {
    let max_period = cells
        .iter()
        .filter_map(|(w, box_w)| overflow_period(*w, *box_w))
        .max()
        .unwrap_or(0);
    (max_period as u32).saturating_mul(2)
}

/// Temperature → color, six bands tuned to read intuitively at a glance.
///
/// Boundaries (°F):
///   ≥95   deep red       extreme heat
///   75–94 orange         hot
///   60–74 yellow / gold  warm
///   45–59 green          comfortable
///   30–44 cyan           cool
///   <30   blue           cold
fn temp_color(temp: f32) -> Color {
    let t = temp.round() as i32;
    match t {
        n if n >= 95 => Color::new(220, 30, 30),
        75..=94 => Color::new(255, 130, 20),
        60..=74 => Color::new(255, 215, 0),
        45..=59 => Color::new(50, 220, 50),
        30..=44 => Color::new(60, 200, 230),
        _ => Color::new(50, 100, 240),
    }
}

// Icon colors live on the WeatherIcon variants themselves — see icon_table.rs.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::weather::model::{
        CurrentWeather, DayForecast, Weather, WeatherApiSource,
    };
    use chrono::{Local, TimeZone};
    use std::path::PathBuf;

    fn repo_fonts() -> WeatherFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        WeatherFonts {
            body: repo.join("04B_03B_.TTF"),
            icon: repo.join("weathericons.ttf"),
            temp: repo.join("BMmini.TTF"),
        }
    }

    fn sample_weather() -> Weather {
        let now = Local.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        Weather {
            api: WeatherApiSource::OpenWeather,
            lat: 37.7749,
            lon: -122.4194,
            location_name: "San Francisco".into(),
            current: CurrentWeather {
                conditions: "Clear".into(),
                temp: 65.0,
                feels_like: 63.0,
                wind_speed: 8.0,
                humidity: 72,
                precipitation_chance: 10,
                uv: Some(5.2),
                wind_direction_deg: Some(270.0),
                icon: crate::api::weather::icon_table::SUNNY,
            },
            forecast: DayForecast {
                today_high: 70.0,
                today_low: 55.0,
                sunrise: now - chrono::Duration::hours(6),
                sunset: now + chrono::Duration::hours(8),
            },
        }
    }

    #[test]
    fn temp_color_ranges() {
        assert_eq!(temp_color(100.0), Color::new(220, 30, 30), "extreme heat");
        assert_eq!(temp_color(85.0), Color::new(255, 130, 20), "hot");
        assert_eq!(temp_color(68.0), Color::new(255, 215, 0), "warm");
        assert_eq!(temp_color(55.0), Color::new(50, 220, 50), "comfortable");
        assert_eq!(temp_color(38.0), Color::new(60, 200, 230), "cool");
        assert_eq!(temp_color(10.0), Color::new(50, 100, 240), "cold");
    }

    #[test]
    fn temp_color_band_boundaries() {
        // Inclusive lower bounds — temp at the band edge should land in the
        // *higher* band (e.g. 75 is hot, 74 is warm; 95 is extreme, 94 is hot).
        assert_eq!(temp_color(95.0), Color::new(220, 30, 30));
        assert_eq!(temp_color(94.0), Color::new(255, 130, 20));
        assert_eq!(temp_color(75.0), Color::new(255, 130, 20));
        assert_eq!(temp_color(74.0), Color::new(255, 215, 0));
        assert_eq!(temp_color(60.0), Color::new(255, 215, 0));
        assert_eq!(temp_color(59.0), Color::new(50, 220, 50));
        assert_eq!(temp_color(45.0), Color::new(50, 220, 50));
        assert_eq!(temp_color(44.0), Color::new(60, 200, 230));
        assert_eq!(temp_color(30.0), Color::new(60, 200, 230));
        assert_eq!(temp_color(29.0), Color::new(50, 100, 240));
    }

    #[test]
    fn screen_one_draws_pixels() {
        let m = WeatherMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img = m.draw_screen_one(&sample_weather(), 0);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 50, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn screen_two_draws_pixels() {
        let m = WeatherMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img = m.draw_screen_two(&sample_weather(), 0);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 50, "expected substantial lit pixels, got {lit}");
    }

    fn extreme_temps_sample() -> Weather {
        // 100°F current, -12°F low — both 4-char strings that overflow the
        // 24-px row-1 right cell and stress the row-2 cells.
        let mut w = sample_weather();
        w.current.temp = 100.0;
        w.current.feels_like = 105.0;
        w.forecast.today_high = 110.0;
        w.forecast.today_low = -12.0;
        w
    }

    #[test]
    fn normal_temps_do_not_trigger_marquee() {
        let m = WeatherMatrix::with_fonts(repo_fonts()).expect("fonts load");
        assert_eq!(m.screen_one_scroll_frames(&sample_weather()), 0);
        assert_eq!(m.screen_two_scroll_frames(&sample_weather()), 0);
    }

    #[test]
    fn extreme_temps_trigger_double_pass_marquee() {
        let m = WeatherMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let w = extreme_temps_sample();
        let s1 = m.screen_one_scroll_frames(&w);
        assert!(s1 > 0, "expected scroll on extreme temps, got 0");
        // Each overflowing cell has its own period, so we don't try to align
        // them all — just verify that the row-1 right cell *moves* mid-scroll
        // (frame 6 is well inside the first pass for any cell with period >12).
        let row1_right_band = (ROW1_RIGHT_X as u32)..((ROW1_RIGHT_X + ROW1_RIGHT_W) as u32);
        let row1_right_pixels = |img: &RgbImage| -> Vec<[u8; 3]> {
            (8..16u32)
                .flat_map(|y| row1_right_band.clone().map(move |x| (x, y)))
                .map(|(x, y)| img.get_pixel(x, y).0)
                .collect()
        };
        let img0 = m.draw_screen_one(&w, 0);
        let img6 = m.draw_screen_one(&w, 6);
        assert_ne!(
            row1_right_pixels(&img0),
            row1_right_pixels(&img6),
            "row 1 right cell should shift between frame 0 and frame 6 during marquee"
        );
    }

    #[test]
    fn screen_one_gutter_stays_dark_under_overflow() {
        // The 1-px row-1 midline gutter (x=24) and the 2-px row-2 midline gutter
        // (x=30..32) must stay dark even when adjacent cells marquee.
        let m = WeatherMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let w = extreme_temps_sample();
        for frame in [0i32, 3, 11, 25, 40, 80] {
            let img = m.draw_screen_one(&w, frame);
            // Row-1 gutter at x=24, y in temp-row band (y_top=8 → y≈8..16).
            for y in 8..16u32 {
                assert_eq!(
                    img.get_pixel(24, y).0,
                    [0, 0, 0],
                    "row 1 gutter (24,{y}) lit at frame {frame}"
                );
            }
            // Row-2 gutter at x=24, y in H/L band (now aligned with row 1).
            for y in 18..26u32 {
                assert_eq!(
                    img.get_pixel(24, y).0,
                    [0, 0, 0],
                    "row 2 gutter (24,{y}) lit at frame {frame}"
                );
            }
        }
    }

    #[test]
    fn screen_two_humidity_gutter_holds() {
        let m = WeatherMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let mut w = sample_weather();
        // 100% humidity & precipitation overflow the body-font cells.
        w.current.humidity = 100;
        w.current.precipitation_chance = 100;
        for frame in [0i32, 5, 13, 27, 50] {
            let img = m.draw_screen_two(&w, frame);
            for y in 8..16u32 {
                assert_eq!(
                    img.get_pixel(24, y).0,
                    [0, 0, 0],
                    "screen 2 row 1 gutter (24,{y}) lit at frame {frame}"
                );
            }
        }
    }

    #[test]
    fn location_scrolls_off_left_edge() {
        let m = WeatherMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img0 = m.draw_screen_one(&sample_weather(), 0);
        let img_far = m.draw_screen_one(&sample_weather(), 200);
        // At xpos=200 the location text should be entirely off-screen, so the
        // top row (y=1..=8) should have noticeably fewer lit pixels than at xpos=0.
        let count_row = |img: &RgbImage, y_range: std::ops::Range<u32>| {
            y_range
                .flat_map(|y| (0..64).map(move |x| img.get_pixel(x, y).0))
                .filter(|p| *p != [0, 0, 0])
                .count()
        };
        let row_full = count_row(&img0, 1..9);
        let row_scrolled = count_row(&img_far, 1..9);
        assert!(row_full > row_scrolled, "full={row_full} scrolled={row_scrolled}");
    }
}
