//! Weather renderer — six-screen suite with condition-driven background
//! animation, animated icons, and conditional screen skipping based on
//! provider data availability.
//!
//! # Screen suite (~90–120 s cycle)
//!
//! Screens are assembled per-poll. Three are unconditional (hero / horizon /
//! comfort); three are skipped when the underlying data isn't available:
//!
//! 1. **Hero (stat strip)** — giant temperature on the left, animated
//!    condition icon in the middle, and a 3-row stat column on the right
//!    (feels-like / high / low). A condition-particle background drifts
//!    across the whole panel.
//! 2. **Horizon (day-length bar)** — big countdown to the next solar event,
//!    secondary daylight stats, and a horizontal day timeline at the bottom
//!    with a sun/moon marker that advances ~1 px per minute.
//! 3. **Comfort** — large "Feels Nn°" headline plus four stacked
//!    label+value+bar rows (humidity, UV, precip-chance, wind).
//! 4. **UV gauge** *(skipped if `current.uv` is `None`, e.g. NWS)* — giant
//!    UV number with rotating sun rays, color-banded scale with current-
//!    value marker, sun-safety hint, burn-time estimate.
//! 5. **3-day forecast strip** *(skipped if `daily` is empty)* — three cells
//!    each carrying weekday / icon / hi / lo / precip-% mini-bar.
//! 6. **Precip-chance bars** *(skipped if `hourly` is empty or all zeros)* —
//!    12 vertical bars for the next 12 hours of precip probability, with
//!    y-axis percent ticks and x-axis hour labels.
//!
//! # Config
//!
//! ```yaml
//! weather:
//!   - run: true
//!     api: openweather              # openweather | nws | accuweather | pirate
//!     api_key: YOUR_KEY             # not required for nws
//!     current_location: true
//!     current_location_api_key: YOUR_IPINFO_TOKEN
//!     city: "Boston, MA"            # alternative to current_location
//!     weather_format: imperial      # imperial | metric
//!     animation: subtle             # off | subtle | prominent (default subtle)
//! ```
//!
//! # Data sources
//!
//! - `openweather` → OneCall 3.0 (`api.openweathermap.org`)
//! - `nws` → US-only, no API key (`api.weather.gov`)
//! - `accuweather` → current + 5-day + 12-hour endpoints
//! - `pirate` → Pirate Weather drop-in for Dark Sky
//!
//! All providers normalize to the same `Weather` shape. Refresh: 10 min.

use crate::api::weather::model::{
    DailyForecast, HourlyForecast, Weather, WeatherIcon, WindDirection,
};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::{Datelike, Local, Timelike};
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

/// Frame display time. 50 ms matches the panel's natural refresh budget.
const FRAME_TICK: Duration = Duration::from_millis(50);
/// Dwell per screen after the initial settle.
const SCREEN_DWELL: Duration = Duration::from_secs(15);
/// First-frame pause so the new screen registers before animation starts.
const FIRST_FRAME_DWELL: Duration = Duration::from_millis(400);

/// How much of the hero background the condition animation should occupy.
///
/// Wire format is lowercase (`off`/`subtle`/`prominent`). Defaults to
/// `subtle` — sparse particles that never crowd the temp/icon.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WeatherAnimationMode {
    Off,
    #[default]
    Subtle,
    Prominent,
}

/// Paths to the fonts the renderer needs.
///
/// `temp` reuses the BMmini path; renderer derives both the giant-temp
/// (16pt) and the mid-headline (14pt) sizes from it. Everything else is
/// 04B_03B at 8pt for labels and weathericons at three sizes for glyphs.
/// `small` is the 4×6 BDF bitmap font used in the 3-day forecast cells
/// where every pixel of vertical space counts.
#[derive(Debug, Clone)]
pub struct WeatherFonts {
    pub body: PathBuf,
    pub icon: PathBuf,
    pub temp: PathBuf,
    pub small: PathBuf,
}

impl Default for WeatherFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
            icon: "/usr/share/fonts/weathericons.ttf".into(),
            temp: "/usr/share/fonts/BMmini.TTF".into(),
            small: "/usr/share/fonts/4x6.bdf".into(),
        }
    }
}

/// Which screen is currently rendering. The set is assembled per-cycle so
/// screens whose data is missing are skipped automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Hero,
    Horizon,
    Comfort,
    Uv,
    Forecast3,
    Precip,
}

/// Decide which screens to render this cycle.
///
/// The three structural screens (hero, horizon, comfort) are always shown.
/// The data-driven ones drop out when their backing field is empty:
///
/// - **UV** — `current.uv` is `None` (NWS never provides it).
/// - **3-day strip** — `daily` is empty (provider didn't return forecast
///   data or the tier doesn't include it).
/// - **Precip bars** — `hourly` is empty *or* every entry is 0% (clear
///   forecast — nothing to plot).
pub fn build_screens(data: &Weather) -> Vec<Screen> {
    let mut s = vec![Screen::Hero, Screen::Horizon, Screen::Comfort];
    if data.current.uv.is_some() {
        s.push(Screen::Uv);
    }
    if !data.daily.is_empty() {
        s.push(Screen::Forecast3);
    }
    if data.hourly.iter().any(|h| h.precipitation_chance > 0) {
        s.push(Screen::Precip);
    }
    s
}

/// Weather renderer state. Holds the loaded fonts and the animation mode.
pub struct WeatherMatrix {
    body_font: Font,
    /// BMmini 14pt — hero temp, countdowns, comfort headline, UV number.
    mid_font: Font,
    /// Weathericons 11pt — hero condition icon + sun/moon marker on the
    /// horizon bar.
    icon_md_font: Font,
    /// 4×6 BDF bitmap — forecast-cell weekday and hi/lo digits.
    small_font: Font,
    animation: WeatherAnimationMode,
}

impl WeatherMatrix {
    /// Build with default font paths and the default animation mode (subtle).
    pub fn new() -> Result<Self, String> {
        Self::with_fonts_and_animation(WeatherFonts::default(), WeatherAnimationMode::default())
    }

    /// Async constructor — loads fonts on a tokio worker thread.
    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_and_animation_async(
            WeatherFonts::default(),
            WeatherAnimationMode::default(),
        )
        .await
    }

    /// Async constructor used by the registry, which knows the user's
    /// configured animation mode.
    pub async fn new_with_animation_async(
        animation: WeatherAnimationMode,
    ) -> Result<Self, String> {
        Self::with_fonts_and_animation_async(WeatherFonts::default(), animation).await
    }

    /// Build with caller-supplied font paths + animation mode.
    pub fn with_fonts_and_animation(
        paths: WeatherFonts,
        animation: WeatherAnimationMode,
    ) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
            mid_font: Font::load_ttf(&paths.temp, 14.0)?,
            icon_md_font: Font::load_ttf(&paths.icon, 11.0)?,
            small_font: Font::load_bdf(&paths.small)?,
            animation,
        })
    }

    /// Async variant of [`Self::with_fonts_and_animation`].
    pub async fn with_fonts_and_animation_async(
        paths: WeatherFonts,
        animation: WeatherAnimationMode,
    ) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts_and_animation(paths, animation))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Back-compat shim used by tests and existing callers that don't care
    /// about animation mode.
    pub fn with_fonts(paths: WeatherFonts) -> Result<Self, String> {
        Self::with_fonts_and_animation(paths, WeatherAnimationMode::default())
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
        // Worst case is 6 screens × 20 s/screen = 120 s. Actual cycle
        // shortens when data-driven screens are skipped.
        Duration::from_secs(120)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &Weather) -> Result<(), RenderError> {
        let screens = build_screens(data);
        let dwell_frames = (SCREEN_DWELL.as_millis() / FRAME_TICK.as_millis()) as u32;
        // Frames-since-screen-entry drives per-screen animations (bar
        // fills, icon stagger). Global frame counter drives background
        // particles so motion stays continuous across screen transitions.
        let mut global_frame: u32 = 0;

        for screen in &screens {
            matrix.clear();
            for entry_frame in 0..dwell_frames {
                let img = self.render_screen(*screen, data, entry_frame, global_frame);
                matrix.set_image(&img, 0, 0);
                tokio::time::sleep(if entry_frame == 0 {
                    FIRST_FRAME_DWELL
                } else {
                    FRAME_TICK
                })
                .await;
                global_frame += 1;
            }
        }
        matrix.clear();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Drawing — pure functions on a fresh RgbImage. No matrix interaction, so
// these are unit-testable.
// ---------------------------------------------------------------------------

impl WeatherMatrix {
    /// Dispatch by screen kind to the appropriate frame helper.
    pub fn render_screen(
        &self,
        screen: Screen,
        data: &Weather,
        entry_frame: u32,
        global_frame: u32,
    ) -> RgbImage {
        match screen {
            Screen::Hero => self.frame_hero_stats(data, global_frame),
            Screen::Horizon => self.frame_horizon(data, global_frame),
            Screen::Comfort => self.frame_comfort(data, entry_frame),
            Screen::Uv => self.frame_uv(data, global_frame),
            Screen::Forecast3 => self.frame_forecast_strip(data, entry_frame),
            Screen::Precip => self.frame_precip_bars(data, entry_frame),
        }
    }

    // --- Screen 1: Hero stat strip -------------------------------------------

    /// Hero with location strip on top, big temp + animated icon + stat
    /// column in the middle, and a condition + wind footer.
    pub fn frame_hero_stats(&self, data: &Weather, frame: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        // Particles draw under everything so glyphs sit on top.
        render_animation(&mut img, data.current.icon, frame, self.animation);

        // y=0..5 — location anchored to the left edge.
        let loc_baseline = top_to_baseline(1, self.body_font.ascent());
        draw_text(
            &mut img,
            &self.body_font,
            2,
            loc_baseline,
            Color::new(0, 254, 0),
            &data.location_name,
        );

        // y=10..23 — temp on the left (dropped below the location strip).
        self.draw_hero_temp(&mut img, data);
        // Animated condition icon, pulled in next to the temp.
        self.draw_animated_icon(&mut img, data.current.icon, frame, 24, 12);

        // y=4..21 — stat column on the right (x=44..63).
        self.draw_hero_stats_col(&mut img, data);

        // y=23 — horizontal divider, lifted to make room for the footer.
        for x in 4..PANEL_W as i32 - 4 {
            put_safe(&mut img, x, 23, Color::new(60, 60, 60));
        }

        // y=24..29 — condition word only. Wind lives on the comfort
        // screen now and isn't repeated here.
        self.draw_centered(&mut img, &self.body_font, &data.current.conditions, 24, Color::WHITE);

        img
    }

    fn draw_hero_temp(&self, img: &mut RgbImage, data: &Weather) {
        // Use the mid (14pt) font so the digits leave room for the icon
        // immediately to their right instead of crowding into it.
        let font = &self.mid_font;
        let temp = data.current.temp.round() as i32;
        let digits = format!("{temp}");
        let digits_w = font.text_width(&digits);
        // Left-anchored at x=2 — the icon now sits flush to the right of the
        // degree mark.
        let left = 2;
        let baseline = top_to_baseline(11, font.ascent());
        draw_text(img, font, left, baseline, temp_color(data.current.temp), &digits);
        // Manual degree mark — 04B_03B's U+00B0 doesn't render usably.
        // Pulled flush against the digits (was `+ 1`).
        draw_degree(img, left + digits_w - 1, 11, Color::WHITE);
    }

    fn draw_hero_stats_col(&self, img: &mut RgbImage, data: &Weather) {
        let body = &self.body_font;
        let right_edge: i32 = PANEL_W as i32 - 2;

        // FL sits on the header row so it visually pairs with the city
        // strip; Hi / Lo flank the big temperature on the right.
        let lines = [
            (format!("FL {}", data.current.feels_like.round() as i32), Color::new(220, 220, 220), 1),
            (format!("Hi {}", data.forecast.today_high.round() as i32), Color::new(255, 150, 40), 10),
            (format!("Lo {}", data.forecast.today_low.round() as i32), Color::new(100, 180, 255), 17),
        ];
        for (text, color, y_top) in &lines {
            let w = body.text_width(text);
            let x = right_edge - w - 3; // leave room for the degree mark
            // Clear a black background under the stat so a long city name
            // sits *behind* it (put_safe uses a lighten blend, so without
            // this clear the city pixels would bleed through the labels).
            clear_rect(img, x - 1, *y_top, right_edge + 2, *y_top + body.ascent() + 1);
            let baseline = top_to_baseline(*y_top, body.ascent());
            draw_text(img, body, x, baseline, *color, text);
            draw_degree(img, x + w + 1, *y_top, *color);
        }
    }

    // --- Screen 2: Horizon day-length bar ------------------------------------

    /// Day-length horizon screen: title + big countdown + horizon timeline.
    pub fn frame_horizon(&self, data: &Weather, frame: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);

        // y=0..4 — title "May 24 . 14:35" centered.
        let now = Local::now();
        let title = format!("{} . {}", now.format("%b %d"), now.format("%H:%M"));
        self.draw_centered(&mut img, &self.body_font, &title, 1, Color::new(180, 180, 200));

        // y=6..21 — countdown to next solar event, split into a small
        // label and a big value so the whole "Sunrise in 5h 02m" string
        // doesn't overflow the 64 px panel at mid_font.
        let sunrise = data.forecast.sunrise;
        let sunset = data.forecast.sunset;
        let in_day = now >= sunrise && now < sunset;
        let (event_name, remaining_secs, accent) = if in_day {
            ("Sunset in", (sunset - now).num_seconds().max(0), Color::new(255, 130, 0))
        } else if now < sunrise {
            ("Sunrise in", (sunrise - now).num_seconds().max(0), Color::new(255, 200, 0))
        } else {
            let tomorrow = sunrise + chrono::Duration::days(1);
            ("Sunrise in", (tomorrow - now).num_seconds().max(0), Color::new(255, 200, 0))
        };
        let h = remaining_secs / 3600;
        let m = (remaining_secs % 3600) / 60;

        // Two-line countdown — single-line "Sunset in 2h 14m" kissed both
        // panel edges at body_font and risked clipping; splitting puts both
        // halves safely centered within the 64 px width.
        self.draw_centered(&mut img, &self.body_font, event_name, 10, accent);
        let value = format!("{h}h {m:02}m");
        self.draw_centered(&mut img, &self.body_font, &value, 17, Color::WHITE);

        // y=24..27 — horizon timeline.
        self.draw_horizon_bar(&mut img, data, frame);

        // y=29..31 — sunrise / sunset times anchored at corners.
        let sunrise_str = sunrise.format("%H:%M").to_string();
        let sunset_str = sunset.format("%H:%M").to_string();
        let baseline = top_to_baseline(26, self.body_font.ascent());
        draw_text(&mut img, &self.body_font, 1, baseline, Color::new(255, 200, 0), &sunrise_str);
        let sunset_w = self.body_font.text_width(&sunset_str);
        draw_text(
            &mut img,
            &self.body_font,
            PANEL_W as i32 - sunset_w - 1,
            baseline,
            Color::new(255, 130, 0),
            &sunset_str,
        );

        img
    }

    fn draw_horizon_bar(&self, img: &mut RgbImage, data: &Weather, frame: u32) {
        const BAR_Y: i32 = 24;
        const LEFT: i32 = 2;
        const RIGHT: i32 = PANEL_W as i32 - 2;

        let sunrise = data.forecast.sunrise;
        let sunset = data.forecast.sunset;
        let now = Local::now();
        let total = (sunset - sunrise).num_seconds().max(1) as f32;
        let elapsed = (now - sunrise).num_seconds() as f32;
        let in_day = (0.0..=total).contains(&elapsed);
        let t = (elapsed / total).clamp(0.0, 1.0);

        // Twilight band ends.
        let twilight_w: i32 = 8;
        let day_left = LEFT + twilight_w;
        let day_right = RIGHT - twilight_w;

        // Pre-dawn twilight (left band): dim blue gradient.
        for x in LEFT..day_left {
            put_safe(img, x, BAR_Y, Color::new(40, 40, 80));
        }
        // Day band: lighter horizon.
        for x in day_left..=day_right {
            put_safe(img, x, BAR_Y, Color::new(120, 120, 180));
        }
        // Dusk twilight (right band).
        for x in (day_right + 1)..=RIGHT {
            put_safe(img, x, BAR_Y, Color::new(40, 40, 80));
        }

        // Marker — a tiny 2 px tick just above the bar. Warm yellow by
        // day, pale moonlight blue at night.
        let marker_x = day_left + ((day_right - day_left) as f32 * t).round() as i32;
        let color = if in_day {
            Color::new(255, 220, 80)
        } else {
            Color::new(180, 200, 255)
        };
        put_safe(img, marker_x, BAR_Y - 1, color);
        put_safe(img, marker_x, BAR_Y - 2, color);
        let _ = frame; // marker no longer animates
    }

    // --- Screen 3: Comfort ---------------------------------------------------

    /// Comfort screen: big feels-like headline + four stacked bars (humidity,
    /// UV, precip-chance, wind). Bars animate 0 → value over the first ~600 ms.
    pub fn frame_comfort(&self, data: &Weather, entry_frame: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);

        // Four full-panel gauges on an 8-row pitch — label+value on the
        // first row, full-width 2 px bar two rows below. Title and
        // feels-like headline are intentionally gone; the gauges *are*
        // the screen.
        let fill_t = bar_entry_t(entry_frame);
        let hum = data.current.humidity.min(100);
        self.draw_comfort_bar(
            &mut img, 0, "Humidity", &format!("{hum}%"),
            hum as f32 / 100.0, fill_t, Color::new(7, 250, 246),
        );
        let uv = data.current.uv.unwrap_or(0.0).max(0.0);
        let uv_t = (uv / 11.0).min(1.0);
        self.draw_comfort_bar(
            &mut img, 8, "UV", &format!("{}", uv.round() as i32),
            uv_t, fill_t, uv_color(uv),
        );
        let pr = data.current.precipitation_chance.min(100);
        self.draw_comfort_bar(
            &mut img, 16, "Rain", &format!("{pr}%"),
            pr as f32 / 100.0, fill_t, Color::new(120, 200, 255),
        );
        // Wind: bar fills relative to a 30 mph reference (typical strong wind).
        let wind = data.current.wind_speed.max(0.0);
        let wind_dir = data
            .current
            .wind_direction_deg
            .map(|d| WindDirection::from_degrees(d).as_str())
            .unwrap_or("---");
        let wind_t = (wind / 30.0).min(1.0);
        self.draw_comfort_bar(
            &mut img, 24, "Wind", &format!("{} mph {wind_dir}", wind.round() as i32),
            wind_t, fill_t, Color::new(201, 1, 253),
        );

        img
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_comfort_bar(
        &self,
        img: &mut RgbImage,
        y_top: i32,
        label: &str,
        value: &str,
        fraction: f32,
        entry_t: f32,
        color: Color,
    ) {
        let body = &self.body_font;
        let baseline = top_to_baseline(y_top, body.ascent());
        // Label + value both drawn in the gauge color so each row reads
        // as one visually-grouped unit (instead of "grey label, colored
        // value" which looked washed out on the panel).
        let label_w = body.text_width(label);
        draw_text(img, body, 2, baseline, color, label);
        draw_text(img, body, 2 + label_w + 3, baseline, color, value);

        // Full-panel-width 2 px bar tucked right under the baseline so the
        // bar sits at the bottom of its 8 px cell — at the old `+1` gap
        // the bar collided with the *next* row's text (most visibly the
        // Wind label clipping the Rain bar).
        let bar_x0 = 0;
        let bar_x1 = PANEL_W as i32;
        let bar_w = bar_x1 - bar_x0;
        let filled = (bar_w as f32 * fraction * entry_t) as i32;
        let bar_y = y_top + body.ascent();
        for by in bar_y..(bar_y + 2) {
            for x in bar_x0..bar_x1 {
                put_safe(img, x, by, Color::new(40, 40, 60));
            }
            for x in bar_x0..(bar_x0 + filled).min(bar_x1) {
                put_safe(img, x, by, color);
            }
        }
    }

    // --- Screen 4: UV gauge --------------------------------------------------

    /// UV gauge: big number with rotating rays, color band with marker,
    /// safety hint, burn-time estimate.
    pub fn frame_uv(&self, data: &Weather, frame: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let uv = data.current.uv.unwrap_or(0.0).max(0.0);

        // Title.
        let title = "UV INDEX";
        self.draw_centered(&mut img, &self.body_font, title, 1, Color::new(180, 180, 200));

        // Big number with rotating rays. Anchor cx/cy to the digit's
        // own math midpoint (not the panel center) so the rays orbit
        // the number rather than sitting beside/below it.
        const NUM_TOP: i32 = 10;
        let label = format!("{}", uv.round() as i32);
        let num_w = self.mid_font.text_width(&label);
        let left = (PANEL_W as i32 - num_w) / 2;
        let baseline = top_to_baseline(NUM_TOP, self.mid_font.ascent());
        let color = uv_color(uv);
        draw_text(&mut img, &self.mid_font, left, baseline, color, &label);
        let cx = left + num_w / 2;
        // Orbit center tracks the *visual* midpoint of the digit's ink,
        // not the font's ascent-box midpoint — BMmini's ascent box sits
        // ~1 px above the actual rasterized digit, so the legacy
        // `NUM_TOP + ascent / 2` rays appeared slightly off-axis.
        let cy = baseline + self.mid_font.text_v_center_from_baseline(&label);
        let num_y0 = NUM_TOP;
        let num_y1 = NUM_TOP + self.mid_font.ascent();
        let num_x0 = left;
        let num_x1 = left + num_w;
        for i in 0..6 {
            let base = i as f32 * std::f32::consts::TAU / 6.0;
            let theta = base + (frame as f32 * 0.08);
            // Tight 2-px ring (r=7..9) keeps the rays hugging the
            // digit; skip any ray pixel that lands inside the digit's
            // bbox so the number stays cleanly readable.
            for r in 7..9 {
                let x = cx + (r as f32 * theta.cos()).round() as i32;
                let y = cy + (r as f32 * theta.sin()).round() as i32;
                if x >= num_x0 && x < num_x1 && y >= num_y0 && y < num_y1 {
                    continue;
                }
                put_safe(&mut img, x, y, color);
            }
        }

        // Color band scale with marker. The band sits 2 rows below the
        // digit so the rotating-ray orbit's bottom half (at y=cy+r ~ 22..23
        // for the BMmini 14pt digit) stays visible instead of being
        // overpainted — without this, the rays read as a top-only halo
        // instead of orbiting the digit.
        const BAND_Y: i32 = 24;
        const BAND_L: i32 = 4;
        const BAND_R: i32 = PANEL_W as i32 - 4;
        let band_w = (BAND_R - BAND_L) as f32;
        for x in BAND_L..BAND_R {
            let t = (x - BAND_L) as f32 / band_w;
            put_safe(&mut img, x, BAND_Y, uv_color(t * 11.0));
            put_safe(&mut img, x, BAND_Y + 1, uv_color(t * 11.0));
        }
        let marker_x = BAND_L + ((uv / 11.0).min(1.0) * band_w) as i32;
        put_safe(&mut img, marker_x - 1, BAND_Y - 1, Color::WHITE);
        put_safe(&mut img, marker_x, BAND_Y - 1, Color::WHITE);
        put_safe(&mut img, marker_x + 1, BAND_Y - 1, Color::WHITE);

        // Footer at y=26 — action hint + burn-time on a single row.
        // The earlier layout stacked two body_font rows at y=26 and
        // y=29; the font's visible glyph height is ~5 px so the rows
        // overlapped and rendered unreadably. WHO-style burn estimate:
        // 200 / UV minutes.
        let burn_min = if uv > 0.5 { (200.0 / uv).round() as i32 } else { 0 };
        let footer = if burn_min == 0 || burn_min >= 999 {
            uv_hint(uv).to_string()
        } else {
            format!("{} {burn_min}m", uv_hint(uv))
        };
        self.draw_centered(&mut img, &self.body_font, &footer, 26, Color::new(220, 220, 220));

        img
    }

    // --- Screen 5: 3-day forecast strip --------------------------------------

    /// 3-day strip: weekday / icon / hi / lo / precip mini-bar per cell.
    pub fn frame_forecast_strip(&self, data: &Weather, entry_frame: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);

        // Title.
        self.draw_centered(&mut img, &self.body_font, "3-DAY OUTLOOK", 1, Color::new(180, 180, 200));

        let cells = data.daily.iter().take(3).collect::<Vec<_>>();
        if cells.is_empty() {
            return img;
        }
        let cell_w = PANEL_W as i32 / cells.len() as i32;

        // Stagger icon fade-in left → right.
        let stagger_per_cell: u32 = 2; // frames
        for (i, day) in cells.iter().enumerate() {
            let cx = i as i32 * cell_w + cell_w / 2;
            let visible_after = i as u32 * stagger_per_cell;
            if entry_frame < visible_after {
                continue;
            }
            self.draw_forecast_cell(&mut img, day, cx);
        }
        img
    }

    fn draw_forecast_cell(&self, img: &mut RgbImage, day: &DailyForecast, cx: i32) {
        let small = &self.small_font;

        // Weekday letters (4×6 BDF, y=8..13).
        let weekday = match day.date.weekday() {
            chrono::Weekday::Mon => "MON",
            chrono::Weekday::Tue => "TUE",
            chrono::Weekday::Wed => "WED",
            chrono::Weekday::Thu => "THU",
            chrono::Weekday::Fri => "FRI",
            chrono::Weekday::Sat => "SAT",
            chrono::Weekday::Sun => "SUN",
        };
        let w = small.text_width(weekday);
        let baseline = top_to_baseline(8, small.ascent());
        draw_text(img, small, cx - w / 2, baseline, Color::new(220, 220, 220), weekday);

        // Icon (weathericons 11pt, y=14..24).
        let glyph = day.icon.glyph.to_string();
        let icon_w = self.icon_md_font.text_width(&glyph);
        let icon_baseline = top_to_baseline(14, self.icon_md_font.ascent());
        draw_text(img, &self.icon_md_font, cx - icon_w / 2, icon_baseline, day.icon.color, &glyph);

        // Hi / lo side-by-side (4×6 BDF, y=25..30). Stacking them at this
        // size would force the lo glyph past the bottom of the panel, so
        // hi goes left-of-center, lo goes right-of-center with a 2 px gap.
        let hi = format!("{}", day.high.round() as i32);
        let lo = format!("{}", day.low.round() as i32);
        let hi_w = small.text_width(&hi);
        let temp_baseline = top_to_baseline(25, small.ascent());
        draw_text(img, small, cx - hi_w - 1, temp_baseline, Color::new(255, 150, 40), &hi);
        draw_text(img, small, cx + 1, temp_baseline, Color::new(100, 180, 255), &lo);

        // Precip mini-bar (y=31) under each cell.
        let bar_left = cx - 4;
        let bar_right = cx + 4;
        let filled = bar_left
            + (((bar_right - bar_left) as u32 * day.precipitation_chance / 100) as i32);
        for x in bar_left..bar_right {
            put_safe(img, x, 30, Color::new(30, 50, 70));
        }
        for x in bar_left..filled {
            put_safe(img, x, 30, Color::new(120, 200, 255));
        }
    }

    // --- Screen 6: Precip-chance bars ---------------------------------------

    /// 12 vertical bars showing the next 12 hours of precipitation chance.
    pub fn frame_precip_bars(&self, data: &Weather, entry_frame: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);

        // Title with peak callout. The peak time is rendered with a
        // single-char meridian ("12p", "3p") — the long form blew past
        // the 64 px panel and clipped on the right.
        let bars: Vec<&HourlyForecast> = data.hourly.iter().take(12).collect();
        let peak = bars
            .iter()
            .max_by_key(|h| h.precipitation_chance)
            .map(|h| {
                let t = h.time.format("%-I%P").to_string();
                let short = if t.len() > 3 { t[..t.len() - 1].to_string() } else { t };
                (h.precipitation_chance, short)
            })
            .unwrap_or((0, "—".into()));
        let title = format!("RAIN {}%@{}", peak.0, peak.1);
        self.draw_centered(&mut img, &self.body_font, &title, 1, Color::new(180, 180, 200));

        // Plot area.
        const BASELINE_Y: i32 = 25;
        const TOP_Y: i32 = 8;
        const BAR_LEFT: i32 = 12;
        let plot_w = PANEL_W as i32 - BAR_LEFT - 2;
        let bar_count = bars.len().max(1) as i32;
        let bar_pitch = plot_w / bar_count;
        let bar_w = (bar_pitch - 1).max(1);

        // y-axis dim ticks at 0/25/50/75/100.
        for (pct, y_top) in [(100, TOP_Y), (50, (TOP_Y + BASELINE_Y) / 2)] {
            let label = format!("{pct}");
            let baseline = top_to_baseline(y_top, self.body_font.ascent());
            draw_text(&mut img, &self.body_font, 2, baseline, Color::new(120, 120, 120), &label);
            for x in BAR_LEFT..(PANEL_W as i32 - 2) {
                if x % 2 == 0 {
                    put_safe(&mut img, x, y_top + self.body_font.ascent() / 2, Color::new(40, 40, 50));
                }
            }
        }
        // Baseline.
        for x in BAR_LEFT..(PANEL_W as i32 - 1) {
            put_safe(&mut img, x, BASELINE_Y, Color::new(80, 80, 100));
        }

        // Bars (animated rise on entry).
        let fill_t = bar_entry_t(entry_frame);
        let max_h = BASELINE_Y - TOP_Y;
        for (i, h) in bars.iter().enumerate() {
            let bar_x = BAR_LEFT + i as i32 * bar_pitch;
            let chance = h.precipitation_chance.min(100);
            let target_h = (max_h * chance as i32) / 100;
            let drawn_h = (target_h as f32 * fill_t).round() as i32;
            let color = if chance >= 60 {
                Color::new(80, 200, 255)
            } else if chance >= 30 {
                Color::new(120, 180, 255)
            } else {
                Color::new(80, 120, 180)
            };
            for y in (BASELINE_Y - drawn_h)..BASELINE_Y {
                for x in bar_x..(bar_x + bar_w) {
                    put_safe(&mut img, x, y, color);
                }
            }
        }

        // x-axis hour labels — every 3rd bar in the 4×6 BDF font. Body
        // font "12p" was 13 px wide in a 12 px slot, so labels overlapped
        // into each other ("smushed"). The bitmap font drops it to 12 px
        // exactly, leaving a 1 px gap between adjacent labels.
        for (i, h) in bars.iter().enumerate() {
            if i % 3 != 0 {
                continue;
            }
            let bar_x = BAR_LEFT + i as i32 * bar_pitch;
            let hour = h.time.format("%-I%P").to_string();
            // Clip the meridian to a single char for space.
            let short = if hour.len() > 3 { &hour[..hour.len() - 1] } else { &hour };
            let baseline = top_to_baseline(26, self.small_font.ascent());
            draw_text(&mut img, &self.small_font, bar_x, baseline, Color::new(160, 160, 160), short);
        }

        img
    }

    // --- Shared helpers -----------------------------------------------------

    fn draw_centered(&self, img: &mut RgbImage, font: &Font, text: &str, y_top: i32, color: Color) {
        let w = font.text_width(text);
        let x = ((PANEL_W as i32 - w) / 2).max(0);
        let baseline = top_to_baseline(y_top, font.ascent());
        draw_text(img, font, x, baseline, color, text);
    }

    /// Draw the condition icon plus per-condition animation overlays
    /// (rotating sun rays, drifting cloud, periodic lightning flash,
    /// micro-snow inside the icon bbox).
    fn draw_animated_icon(&self, img: &mut RgbImage, icon: WeatherIcon, frame: u32, x_left: i32, y_top: i32) {
        let glyph = icon.glyph.to_string();
        let font = &self.icon_md_font;
        let baseline = top_to_baseline(y_top, font.ascent());

        // Per-code animation offset / overlay decisions before drawing the glyph.
        let (x_off, y_off, overlay) = match icon.owm_code {
            // Sunny — rotating rays around the glyph center.
            800 => (0, 0, IconOverlay::SunRays),
            // Clouds — drift the glyph horizontally on a sine wave.
            801..=899 => (((frame as f32 / 14.0).sin() * 2.0) as i32, 0, IconOverlay::None),
            // Thunderstorm — random flash every ~50 frames lasting 2 frames.
            200..=299 => (0, 0, if (frame % 50) < 2 { IconOverlay::Flash } else { IconOverlay::None }),
            // Snow — micro snowflakes inside the icon bbox.
            600..=699 => (0, 0, IconOverlay::SnowDust),
            _ => (0, 0, IconOverlay::None),
        };

        draw_text(img, font, x_left + x_off, baseline + y_off, icon.color, &glyph);

        let glyph_w = font.text_width(&glyph);
        let cx = x_left + glyph_w / 2;
        // Rendered TTF glyphs land roughly between y_top and y_top + ascent,
        // so the visual midpoint is ascent/2 below the top. Using this here
        // means overlays (sun rays, snow dust) ring the icon evenly instead
        // of sitting below it.
        let cy = y_top + font.ascent() / 2;
        match overlay {
            IconOverlay::SunRays => {
                // Radius tuned to hug the glyph — at radius 6 the rays don't
                // bleed into the temp digits or stat column.
                for i in 0..8 {
                    let base = i as f32 * std::f32::consts::TAU / 8.0;
                    let theta = base + (frame as f32 * 0.06);
                    let x = cx + (6.0 * theta.cos()).round() as i32;
                    let y = cy + (6.0 * theta.sin()).round() as i32;
                    put_safe(img, x, y, Color::new(255, 220, 80));
                }
            }
            IconOverlay::Flash => {
                // Brighten the icon bbox briefly.
                for y in y_top..(y_top + 13) {
                    for x in x_left..(x_left + glyph_w) {
                        put_safe(img, x, y, Color::new(255, 255, 200));
                    }
                }
            }
            IconOverlay::SnowDust => {
                for i in 0..3 {
                    let px = x_left + (((frame + i * 11) as i32 + i as i32 * 3) % glyph_w.max(1));
                    let py = y_top + (((frame / 3 + i * 5) as i32) % 12);
                    put_safe(img, px, py, Color::new(220, 220, 255));
                }
            }
            IconOverlay::None => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum IconOverlay {
    None,
    SunRays,
    Flash,
    SnowDust,
}

// ---------------------------------------------------------------------------
// Module-level helpers.
// ---------------------------------------------------------------------------

/// PIL `ImageDraw.text((x, y), ...)` interprets `y` as the *top* of the glyph;
/// our `draw_text` takes a baseline. This converts top-y → baseline-y.
fn top_to_baseline(top_y: i32, ascent: i32) -> i32 {
    top_y + ascent
}

/// Draw a degree-sign mark as a 2×2 filled square at `(x, y)`.
fn draw_degree(img: &mut RgbImage, x: i32, y: i32, color: Color) {
    put_safe(img, x, y, color);
    put_safe(img, x + 1, y, color);
    put_safe(img, x, y + 1, color);
    put_safe(img, x + 1, y + 1, color);
}

/// Hard-overwrite a rectangle to black, clipping to the panel. Used to
/// punch a background under text that needs to occlude (not blend with) the
/// pixels already drawn beneath it — `put_safe`'s lighten composite cannot
/// darken existing pixels on its own.
fn clear_rect(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32) {
    let xa = x0.max(0);
    let ya = y0.max(0);
    let xb = x1.min(PANEL_W as i32);
    let yb = y1.min(PANEL_H as i32);
    for y in ya..yb {
        for x in xa..xb {
            img.put_pixel(x as u32, y as u32, image::Rgb([0, 0, 0]));
        }
    }
}

/// Lighten-composite a single pixel, dropping anything outside the panel.
fn put_safe(img: &mut RgbImage, x: i32, y: i32, color: Color) {
    if x < 0 || y < 0 || x >= PANEL_W as i32 || y >= PANEL_H as i32 {
        return;
    }
    let existing = img.get_pixel(x as u32, y as u32).0;
    img.put_pixel(
        x as u32,
        y as u32,
        image::Rgb([
            existing[0].max(color.r),
            existing[1].max(color.g),
            existing[2].max(color.b),
        ]),
    );
}

/// Temperature → color, six bands tuned to read intuitively at a glance.
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

/// UV-index → WHO color band.
fn uv_color(uv: f32) -> Color {
    match uv.round() as i32 {
        i if i <= 2 => Color::new(80, 200, 80),
        3..=5 => Color::new(255, 215, 0),
        6..=7 => Color::new(255, 140, 30),
        8..=10 => Color::new(230, 60, 60),
        _ => Color::new(180, 100, 220),
    }
}

/// Short single-word action for the UV footer. Kept terse so it can
/// share a 64-px row with the burn-time estimate.
fn uv_hint(uv: f32) -> &'static str {
    match uv.round() as i32 {
        i if i <= 2 => "safe",
        3..=5 => "shade",
        6..=7 => "cover",
        8..=10 => "limit sun",
        _ => "avoid sun",
    }
}

/// Easing fraction for the bar-fill animation on screen entry. Returns 1.0
/// after the first ~600 ms (12 frames at 50 ms tick).
fn bar_entry_t(entry_frame: u32) -> f32 {
    const FRAMES: u32 = 12;
    if entry_frame >= FRAMES {
        1.0
    } else {
        // Ease-out (1 - (1-t)^2).
        let t = entry_frame as f32 / FRAMES as f32;
        1.0 - (1.0 - t).powi(2)
    }
}

// ---------------------------------------------------------------------------
// Background-animation engine (unchanged from the prior renderer aside from
// internal cleanups; particles are still owm_code-driven).
// ---------------------------------------------------------------------------

/// Compose the per-frame background animation onto `img` based on the
/// resolved condition icon. The icon (not just its `owm_code`) carries the
/// accent color so day/night variants that share an OWM code — e.g. SUNNY
/// vs CLEAR_NIGHT, both code 800 — render in their own palette.
fn render_animation(img: &mut RgbImage, icon: WeatherIcon, frame: u32, mode: WeatherAnimationMode) {
    if matches!(mode, WeatherAnimationMode::Off) {
        return;
    }
    let intensity = match mode {
        WeatherAnimationMode::Subtle => 1,
        WeatherAnimationMode::Prominent => 3,
        WeatherAnimationMode::Off => return,
    };

    match icon.owm_code {
        200..=299 => {
            render_rain(img, frame, 6 * intensity, Color::new(120, 180, 255));
            if (frame + (icon.owm_code as u32 % 7)) % 80 < 2 {
                flash(img, Color::new(120, 120, 200));
            }
        }
        300..=399 => render_rain(img, frame, 3 * intensity, Color::new(140, 200, 255)),
        500..=599 => render_rain(img, frame, 5 * intensity, Color::new(120, 180, 255)),
        600..=699 => render_snow(img, frame, 5 * intensity),
        700..=799 => render_fog(img, frame, 2 * intensity),
        800 => render_clear_pulse(img, frame, intensity, icon.color),
        801..=899 => render_clouds(img, frame, 3 * intensity),
        _ => {}
    }
}

fn render_rain(img: &mut RgbImage, frame: u32, count: u32, color: Color) {
    for i in 0..count {
        let col = ((i * 11 + 5) % PANEL_W) as i32;
        let phase = (i * 7) % PANEL_H;
        let y = ((frame + phase) % PANEL_H) as i32;
        put_safe(img, col, y, color);
        put_safe(img, col, y - 1, dim(color, 2));
    }
}

fn render_snow(img: &mut RgbImage, frame: u32, count: u32) {
    let color = Color::new(220, 220, 255);
    for i in 0..count {
        let base_col = ((i * 13 + 3) % PANEL_W) as i32;
        let drift = ((frame as f32 / 14.0) + i as f32).sin();
        let col = base_col + (drift * 2.0) as i32;
        let phase = (i * 5) % (PANEL_H * 3);
        let y = ((frame + phase) / 3 % PANEL_H) as i32;
        put_safe(img, col, y, color);
    }
}

fn render_fog(img: &mut RgbImage, frame: u32, count: u32) {
    let color = Color::new(120, 120, 130);
    for i in 0..count {
        let row = match i % 3 {
            0 => 3,
            1 => 15,
            _ => 24,
        };
        let off = (frame + i * 11) % (PANEL_W * 2);
        for dx in 0..6 {
            let x = (off as i32 + dx) % PANEL_W as i32;
            put_safe(img, x, row, color);
        }
    }
}

fn render_clear_pulse(img: &mut RgbImage, frame: u32, intensity: u32, accent: Color) {
    let phase = (frame % 60) as f32 / 60.0;
    let amp = (phase * std::f32::consts::TAU).sin().abs();
    let brightness = (40 + intensity * 60).min(180) as f32 * amp / 255.0;
    // Twinkles inherit the icon's accent color so the sun pulses warm and
    // the moon pulses cool — no hardcoded orange that bleeds into night.
    let color = Color::new(
        (accent.r as f32 * brightness) as u8,
        (accent.g as f32 * brightness) as u8,
        (accent.b as f32 * brightness) as u8,
    );
    let offset = (frame / 8) as i32;
    for (i, base) in [(2, 2), (60, 4), (4, 28), (58, 26), (10, 6), (54, 24)]
        .iter()
        .enumerate()
    {
        if i as u32 >= 2 + intensity * 2 {
            break;
        }
        let x = (base.0 + (i as i32 + offset) % 3) % PANEL_W as i32;
        let y = base.1;
        put_safe(img, x, y, color);
    }
}

fn render_clouds(img: &mut RgbImage, frame: u32, count: u32) {
    let color = Color::new(140, 140, 160);
    for i in 0..count {
        let row = 3 + (i as i32 % 3) * 2;
        let speed = 1 + (i % 3);
        let off = (frame * speed + i * 17) % (PANEL_W * 2);
        let x = off as i32 - PANEL_W as i32;
        put_safe(img, x, row, color);
        put_safe(img, x + 1, row, color);
        put_safe(img, x + 2, row, color);
    }
}

fn flash(img: &mut RgbImage, color: Color) {
    for y in 0..PANEL_H as i32 {
        for x in 0..PANEL_W as i32 {
            put_safe(img, x, y, color);
        }
    }
}

fn dim(c: Color, factor: u8) -> Color {
    Color::new(c.r / factor, c.g / factor, c.b / factor)
}

// Silence unused-import warning when only `Timelike` becomes idle in some
// configurations — the trait is needed for `chrono::Local::now().hour()`
// style calls in future helpers.
const _: fn() = || {
    let _ = Local::now().hour();
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::weather::model::{
        CurrentWeather, DailyForecast, DayForecast, HourlyForecast, Weather, WeatherApiSource,
    };
    use chrono::{Local, NaiveDate, TimeZone};
    use std::path::PathBuf;

    fn repo_fonts() -> WeatherFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        WeatherFonts {
            body: repo.join("04B_03B_.TTF"),
            icon: repo.join("weathericons.ttf"),
            temp: repo.join("BMmini.TTF"),
            small: repo.join("4x6.bdf"),
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
            hourly: (0..6)
                .map(|i| HourlyForecast {
                    time: now + chrono::Duration::hours(i),
                    temp: 65.0 + i as f32,
                    precipitation_chance: (i as u32 * 15).min(90),
                })
                .collect(),
            daily: (1..=5)
                .map(|i| DailyForecast {
                    date: NaiveDate::from_ymd_opt(2024, 6, 15 + i).unwrap(),
                    high: 70.0 + i as f32,
                    low: 55.0 + i as f32,
                    icon: if i % 2 == 0 {
                        crate::api::weather::icon_table::PARTLY_CLOUDY
                    } else {
                        crate::api::weather::icon_table::SUNNY
                    },
                    precipitation_chance: (i as u32 * 10).min(90),
                })
                .collect(),
        }
    }

    fn matrix_subtle() -> WeatherMatrix {
        WeatherMatrix::with_fonts_and_animation(repo_fonts(), WeatherAnimationMode::Subtle)
            .expect("fonts load")
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

    fn lit_pixel_count(img: &RgbImage) -> usize {
        img.pixels().filter(|p| p.0 != [0, 0, 0]).count()
    }

    #[test]
    fn hero_stats_draws_pixels() {
        let m = matrix_subtle();
        let img = m.frame_hero_stats(&sample_weather(), 0);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        assert!(lit_pixel_count(&img) > 80, "hero should fill the panel");
    }

    #[test]
    fn horizon_draws_pixels() {
        let m = matrix_subtle();
        let img = m.frame_horizon(&sample_weather(), 0);
        assert!(lit_pixel_count(&img) > 60);
    }

    #[test]
    fn comfort_draws_pixels_after_bar_fill() {
        let m = matrix_subtle();
        // entry_frame >> 12 means bars are fully filled.
        let img = m.frame_comfort(&sample_weather(), 20);
        assert!(lit_pixel_count(&img) > 80);
    }

    #[test]
    fn uv_draws_pixels() {
        let m = matrix_subtle();
        let img = m.frame_uv(&sample_weather(), 5);
        assert!(lit_pixel_count(&img) > 60);
    }

    #[test]
    fn uv_rays_visible_below_digit() {
        // Regression: the band+marker used to sit immediately below
        // the digit (BAND_Y=22), overpainting the bottom half of the
        // ray orbit (cy=15 + r=7..8 lands at y=22..23). The user saw
        // rays only above the digit, reading as a halo not an orbit.
        // BAND_Y=24 leaves room for the orbit's bottom rays at y=21..22.
        let m = matrix_subtle();
        let w = sample_weather();
        // Sweep a full rotation to be sure SOME frame draws a bottom
        // ray in the gap between digit ink and marker.
        let mut bottom_lit = 0u32;
        for frame in 0..80u32 {
            let img = m.frame_uv(&w, frame);
            // Below the digit's ascent box (y >= 21), above the marker
            // row (y < 23). Exclude the digit's own column footprint
            // so we count rays, not the digit's tail.
            for y in 21..23u32 {
                for x in 0..PANEL_W {
                    let p = img.get_pixel(x, y).0;
                    if p == [0, 0, 0] || p == [255, 255, 255] {
                        continue;
                    }
                    bottom_lit += 1;
                }
            }
        }
        assert!(
            bottom_lit > 20,
            "expected visible ray pixels below the digit across a full rotation, got {bottom_lit}"
        );
    }

    #[test]
    fn uv_ray_orbit_is_centered_on_digit_ink() {
        // Regression: the orbit used to anchor at `NUM_TOP + ascent/2`
        // — BMmini's ascent box sits ~1 px above the actual rendered
        // digit ink, so the rays appeared off-axis. Verify the helper
        // that frame_uv now uses (`text_v_center_from_baseline`)
        // tracks the digit's *rendered* vertical centroid within 1 px.
        let m = matrix_subtle();

        for label in ["0", "5", "9", "11"] {
            // Render the digit in isolation at a known baseline and
            // measure the vertical centroid of its non-black pixels.
            let baseline = 20i32;
            let mut img = RgbImage::new(PANEL_W, PANEL_H);
            draw_text(&mut img, &m.mid_font, 24, baseline, Color::WHITE, label);

            let (mut sum_y, mut count) = (0.0f64, 0u32);
            for y in 0..PANEL_H as i32 {
                for x in 0..PANEL_W as i32 {
                    if img.get_pixel(x as u32, y as u32).0 != [0, 0, 0] {
                        sum_y += y as f64;
                        count += 1;
                    }
                }
            }
            assert!(count > 0, "digit `{label}` must render some pixels");
            let ink_centroid = sum_y / count as f64;
            let helper_cy =
                baseline as f64 + m.mid_font.text_v_center_from_baseline(label) as f64;
            assert!(
                (helper_cy - ink_centroid).abs() <= 1.0,
                "for `{label}`: helper returned cy={helper_cy:.2}, ink centroid {ink_centroid:.2}"
            );
        }
    }

    #[test]
    fn forecast_strip_draws_pixels() {
        let m = matrix_subtle();
        // Long enough for all cells to be visible.
        let img = m.frame_forecast_strip(&sample_weather(), 30);
        assert!(lit_pixel_count(&img) > 80);
    }

    #[test]
    fn precip_bars_draws_pixels() {
        let m = matrix_subtle();
        let img = m.frame_precip_bars(&sample_weather(), 20);
        assert!(lit_pixel_count(&img) > 50);
    }

    #[test]
    fn build_screens_includes_all_when_data_complete() {
        let w = sample_weather();
        assert_eq!(
            build_screens(&w),
            vec![
                Screen::Hero,
                Screen::Horizon,
                Screen::Comfort,
                Screen::Uv,
                Screen::Forecast3,
                Screen::Precip,
            ]
        );
    }

    #[test]
    fn build_screens_skips_when_data_missing() {
        let mut w = sample_weather();
        w.current.uv = None; // simulates NWS
        w.daily.clear(); // simulates missing daily forecast
        w.hourly.clear(); // simulates missing hourly forecast
        assert_eq!(
            build_screens(&w),
            vec![Screen::Hero, Screen::Horizon, Screen::Comfort]
        );
    }

    #[test]
    fn build_screens_skips_precip_when_all_zero() {
        let mut w = sample_weather();
        for h in &mut w.hourly {
            h.precipitation_chance = 0;
        }
        // UV and daily still present.
        assert_eq!(
            build_screens(&w),
            vec![Screen::Hero, Screen::Horizon, Screen::Comfort, Screen::Uv, Screen::Forecast3]
        );
    }

    #[test]
    fn comfort_bar_animation_advances_with_entry_frame() {
        // The bars draw a dim empty track first and overpaint the filled
        // portion in full color, so the lit-pixel count is identical
        // between early and late frames. Compare total brightness instead.
        let m = matrix_subtle();
        let brightness = |img: &RgbImage| -> u64 {
            img.pixels()
                .map(|p| p.0[0] as u64 + p.0[1] as u64 + p.0[2] as u64)
                .sum()
        };
        let early = m.frame_comfort(&sample_weather(), 0);
        let late = m.frame_comfort(&sample_weather(), 20);
        assert!(brightness(&late) > brightness(&early));
    }

    #[test]
    fn forecast_strip_staggers_in() {
        let m = matrix_subtle();
        // At entry_frame 0 only the first cell shows; by 30 all do.
        let early = m.frame_forecast_strip(&sample_weather(), 0);
        let late = m.frame_forecast_strip(&sample_weather(), 30);
        assert!(lit_pixel_count(&late) > lit_pixel_count(&early));
    }

    #[test]
    fn precip_bars_rise_with_entry_frame() {
        let m = matrix_subtle();
        let early = m.frame_precip_bars(&sample_weather(), 0);
        let late = m.frame_precip_bars(&sample_weather(), 20);
        assert!(lit_pixel_count(&late) > lit_pixel_count(&early));
    }

    #[test]
    fn extreme_temps_render_without_panicking() {
        let m = matrix_subtle();
        for t in [-12.0_f32, 100.0, 110.0] {
            let mut w = sample_weather();
            w.current.temp = t;
            let _ = m.frame_hero_stats(&w, 0);
        }
    }

    #[test]
    fn animation_off_yields_fewer_pixels_than_subtle() {
        let off = WeatherMatrix::with_fonts_and_animation(repo_fonts(), WeatherAnimationMode::Off)
            .expect("fonts");
        let subtle = matrix_subtle();
        let mut w = sample_weather();
        w.current.icon = crate::api::weather::icon_table::RAIN;
        let count = |m: &WeatherMatrix| -> usize {
            (0..10).map(|f| lit_pixel_count(&m.frame_hero_stats(&w, f))).sum()
        };
        assert!(count(&subtle) > count(&off));
    }

    #[test]
    fn animation_is_deterministic_per_frame() {
        let m = matrix_subtle();
        let mut w = sample_weather();
        w.current.icon = crate::api::weather::icon_table::RAIN;
        let img_a = m.frame_hero_stats(&w, 7);
        let img_b = m.frame_hero_stats(&w, 7);
        assert_eq!(img_a.as_raw(), img_b.as_raw());
    }
}
