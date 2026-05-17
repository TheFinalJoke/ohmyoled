//! Weather renderer — pure-Rust port of `src/python/ohmyoled/matrix/weathermatrix.py`.
//!
//! Two scrolling screens, each ~100 frames at 0.05s + a long dwell. Screen 1
//! shows temp/feels-like/high/low + weather icon + location/conditions. Screen 2
//! shows humidity/wind/sunrise/sunset + icon + location.
//!
//! # Layout (64×32) — screen 1
//!
//! ```text
//!   ┌─────────────┬─────────┬──────────────┐
//!   │ 72°F        │  icon   │ feels 70°F   │
//!   │ city        │         │ ↑78  ↓64     │
//!   │ scrolling conditions text            │
//!   └──────────────────────────────────────┘
//! ```
//!
//! Screen 2 swaps the temperature column for humidity / wind / sunrise /
//! sunset. Temp color: blue ⇐ cold, white ⇐ mild, red ⇐ hot.
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
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::Local;
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

/// Paths to the three fonts the renderer needs.
///
/// The defaults match what `src/sh/install.sh` lays down in `/usr/share/fonts/`.
#[derive(Debug, Clone)]
pub struct WeatherFonts {
    pub body: PathBuf,
    pub icon: PathBuf,
    pub retro: PathBuf,
}

impl Default for WeatherFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
            icon: "/usr/share/fonts/weathericons.ttf".into(),
            retro: "/usr/share/fonts/retro_computer.ttf".into(),
        }
    }
}

/// Weather renderer state. Owns the three TTF fonts.
pub struct WeatherMatrix {
    body_font: Font,
    body_icon_font: Font,
    big_icon_font: Font,
    retro_font: Font,
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
            // 9pt for inline weather glyphs; 11pt for sunrise/sunset symbols.
            body_icon_font: Font::load_ttf(&paths.icon, 9.0)?,
            big_icon_font: Font::load_ttf(&paths.icon, 11.0)?,
            retro_font: Font::load_ttf(&paths.retro, 7.0)?,
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
        for xpos in 0..SCROLL_FRAMES {
            let img = self.draw_screen_one(data, xpos as i32);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(if xpos == 0 { FIRST_FRAME_DWELL } else { SCROLL_TICK }).await;
        }
        let img = self.draw_screen_one(data, 0);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(SCREEN1_DWELL).await;
        matrix.clear();

        // Screen 2: humidity + wind + sunrise/sunset + icon + scrolling location.
        for xpos in 0..SCROLL_FRAMES {
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
        self.render_temp(&mut img, data);
        self.render_icon(&mut img, data);
        self.render_location(&mut img, data, xpos);
        self.render_conditions(&mut img, data, xpos);
        img
    }

    /// Render screen 2 (humidity/wind/sunrise/sunset) at the given scroll offset.
    pub fn draw_screen_two(&self, data: &Weather, xpos: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        self.render_location(&mut img, data, xpos);
        self.render_icon(&mut img, data);
        self.render_humidity(&mut img, data);
        self.render_wind(&mut img, data);
        self.render_time(&mut img, data);
        img
    }

    fn render_temp(&self, img: &mut RgbImage, data: &Weather) {
        let font = &self.retro_font;
        let baseline = top_to_baseline(8, font.ascent());
        let baseline2 = top_to_baseline(18, font.ascent());

        // T:<temp>F  R:<feels>F
        draw_text(img, font, 0, baseline, Color::WHITE, "T:");
        draw_text(
            img,
            font,
            10,
            baseline,
            temp_color(data.current.temp),
            &format!("{}F", data.current.temp.round() as i32),
        );
        draw_text(img, font, 30, baseline, Color::WHITE, "R:");
        draw_text(
            img,
            font,
            40,
            baseline,
            temp_color(data.current.feels_like),
            &format!("{}F", data.current.feels_like.round() as i32),
        );

        // H:<high>F  L:<low>F
        draw_text(img, font, 1, baseline2, Color::WHITE, "H:");
        draw_text(
            img,
            font,
            10,
            baseline2,
            temp_color(data.forecast.today_high),
            &format!("{}F", data.forecast.today_high.round() as i32),
        );
        draw_text(img, font, 30, baseline2, Color::WHITE, "L:");
        draw_text(
            img,
            font,
            40,
            baseline2,
            temp_color(data.forecast.today_low),
            &format!("{}F", data.forecast.today_low.round() as i32),
        );
    }

    fn render_icon(&self, img: &mut RgbImage, data: &Weather) {
        let glyph = data.current.icon.glyph.to_string();
        let color = icon_color(data.current.icon.owm_code, &data.forecast.sunset);
        let baseline = top_to_baseline(0, self.body_icon_font.ascent());
        draw_text(img, &self.body_icon_font, 50, baseline, color, &glyph);
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

    fn render_humidity(&self, img: &mut RgbImage, data: &Weather) {
        let baseline = top_to_baseline(8, self.body_font.ascent());
        draw_text(img, &self.body_font, 2, baseline, Color::WHITE, "H:");
        draw_text(
            img,
            &self.body_font,
            10,
            baseline,
            Color::new(7, 250, 246),
            &format!("{}%", data.current.humidity),
        );
        draw_text(img, &self.body_font, 27, baseline, Color::WHITE, "P:");
        draw_text(
            img,
            &self.body_font,
            34,
            baseline,
            Color::new(7, 250, 246),
            &format!("{}%", data.current.precipitation_chance),
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
        let sun_glyph_baseline = top_to_baseline(18, self.big_icon_font.ascent());
        draw_text(
            img,
            &self.big_icon_font,
            1,
            sun_glyph_baseline,
            Color::new(255, 255, 0),
            "\u{f058}",
        );
        draw_text(
            img,
            &self.big_icon_font,
            35,
            sun_glyph_baseline,
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

/// Temperature → color, mirroring `WeatherMatrix.get_temp_color` (weathermatrix.py:86-96).
fn temp_color(temp: f32) -> Color {
    let t = temp.round() as i32;
    match t {
        n if n >= 100 => Color::new(255, 12, 3),
        70..=99 => Color::new(247, 157, 3),
        40..=69 => Color::new(5, 223, 3),
        20..=39 => Color::new(0, 255, 255),
        _ => Color::new(0, 76, 255),
    }
}

/// Icon color, mirroring `WeatherMatrix.render_icon`'s cascade (weathermatrix.py:131-156).
fn icon_color(owm_code: u16, sunset: &chrono::DateTime<Local>) -> Color {
    match owm_code {
        200..=299 => Color::new(254, 204, 1),   // thunderstorm
        300..=399 => Color::new(220, 220, 220), // drizzle
        500..=599 => Color::new(108, 204, 228), // rain
        600..=699 => Color::WHITE,              // snow
        700..=780 => Color::new(192, 192, 192), // haze/smoke/fog
        800 => {
            if *sunset > Local::now() {
                Color::new(220, 149, 3) // sunny day
            } else {
                Color::WHITE // clear night
            }
        }
        801..=805 => Color::new(220, 220, 220),
        _ => Color::WHITE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::weather::model::{
        CurrentWeather, DayForecast, Weather, WeatherApiSource, WeatherIcon,
    };
    use chrono::TimeZone;
    use std::path::PathBuf;

    fn repo_fonts() -> WeatherFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        WeatherFonts {
            body: repo.join("04B_03B_.TTF"),
            icon: repo.join("weathericons.ttf"),
            retro: repo.join("retro_computer.ttf"),
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
                icon: WeatherIcon {
                    condition: "Sunny",
                    glyph: '\u{f00d}',
                    owm_code: 800,
                },
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
        assert_eq!(temp_color(105.0), Color::new(255, 12, 3));
        assert_eq!(temp_color(85.0), Color::new(247, 157, 3));
        assert_eq!(temp_color(55.0), Color::new(5, 223, 3));
        assert_eq!(temp_color(30.0), Color::new(0, 255, 255));
        assert_eq!(temp_color(10.0), Color::new(0, 76, 255));
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
