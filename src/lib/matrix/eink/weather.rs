//! E-paper weather renderer — a static, full-resolution monochrome dashboard.
//!
//! The same [`Weather`] data the LED weather tile animates, laid out for a
//! large e-paper sheet: big current temperature + condition icon, a stats row
//! (feels-like / humidity / wind / UV), today's high-low, and a multi-day
//! forecast strip. No scrolling, no animation — one composed screen that the
//! panel holds until the next refresh.
//!
//! Reuses the shared `draw_*` primitives and TTF fonts, so glyphs match the
//! rest of the project. Everything is drawn white-on-black; the display
//! inverts to black-ink-on-white when it pushes the frame.
//!
//! # Config
//!
//! Lives under the independent `eink` display block (see the registry), e.g.:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   model: "4in2"
//!   modules:
//!     weather:
//!       run: true
//!       api: nws
//!       current_location: true
//! ```
//!
//! Data source: whichever weather provider the section selects (same
//! collectors as the LED tile).

use crate::api::weather::model::{Weather, WindDirection};
use crate::matrix::eink::layout::{center_text, scaled_px, sparkline, stat_row};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_circle, draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default font directory laid down by the install scripts.
const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper weather renderer.
pub struct EinkWeatherFonts {
    /// Body/numeric text font (04B_03B TTF).
    pub body: PathBuf,
    /// Weather-icons glyph font.
    pub icon: PathBuf,
}

impl Default for EinkWeatherFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
            icon: Path::new(FONT_DIR).join("weathericons.ttf"),
        }
    }
}

/// Static e-paper weather dashboard renderer.
///
/// Fonts are sized for the panel passed at construction (see [`scaled_px`]),
/// so the dashboard reads the same on a 400×300 4.2" or an 800×480 7.5" sheet.
pub struct EinkWeatherMatrix {
    big: Font,
    title: Font,
    label: Font,
    small: Font,
    unit: Font,
    icon_big: Font,
    icon_small: Font,
}

impl EinkWeatherMatrix {
    /// Sync constructor (useful for tests). `dims` is the target panel size.
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkWeatherFonts::default(), dims)
    }

    /// Async constructor used by the registry — font I/O runs on a worker
    /// thread so the executor isn't blocked.
    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkWeatherFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkWeatherFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            big: Font::load_ttf(&paths.body, scaled_px(132.0, h))?,
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            small: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
            unit: Font::load_ttf(&paths.body, scaled_px(42.0, h))?,
            icon_big: Font::load_ttf(&paths.icon, scaled_px(124.0, h))?,
            icon_small: Font::load_ttf(&paths.icon, scaled_px(46.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkWeatherFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the full dashboard at `w × h`.
    pub fn frame(&self, data: &Weather, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let margin = (wi / 32).max(6);

        // ── Header: location (left) + condition (right) ─────────────────
        let header_base = margin + self.title.ascent();
        draw_text(&mut img, &self.title, margin, header_base, fg, &data.location_name);
        let cond = data.current.conditions.to_uppercase();
        let cond_w = self.label.text_width(&cond);
        draw_text(
            &mut img,
            &self.label,
            wi - margin - cond_w,
            margin + self.label.ascent(),
            fg,
            &cond,
        );
        let rule_y = header_base + margin / 2;
        draw_line(&mut img, margin, rule_y, wi - margin, rule_y, fg);

        // The body below the header splits into three stacked bands that fill
        // the panel: a tall hero (big temp + icon), a stats band, and a
        // forecast strip pinned to the bottom edge.
        let has_forecast = !data.daily.is_empty();
        let strip_top = if has_forecast {
            hi - (hi * 38 / 100)
        } else {
            hi
        };

        // ── Stats + hi/lo, stacked just above the forecast strip ─────────
        let wind_dir = data
            .current
            .wind_direction_deg
            .map(|d| WindDirection::from_degrees(d).as_str())
            .unwrap_or("");
        let mut stats = vec![
            format!("FEELS {:.0}F", data.current.feels_like),
            format!("HUM {}%", data.current.humidity),
            format!("WIND {:.0} {}", data.current.wind_speed, wind_dir),
        ];
        if let Some(uv) = data.current.uv {
            stats.push(format!("UV {uv:.0}"));
        }
        let hilo = format!(
            "H {:.0}F    L {:.0}F",
            data.forecast.today_high, data.forecast.today_low
        );
        // hi/lo sits just above the divider; stats one line above that.
        let band_bottom = if has_forecast { strip_top } else { hi };
        let hilo_y = band_bottom - margin - (self.label.height() - self.label.ascent());
        let stats_y = hilo_y - self.label.height() - margin / 2;
        stat_row(&mut img, &self.label, stats_y, fg, &stats);
        draw_text(&mut img, &self.label, margin, hilo_y, fg, &hilo);

        // ── Big current temperature + icon, centered in the hero band ───
        let temp_str = format!("{:.0}", data.current.temp);
        let temp_w = self.big.text_width(&temp_str);
        // Hero band spans the header rule down to the stats; center the
        // numerals' visual midpoint in it so the big glyphs feel anchored.
        let hero_top = rule_y;
        let hero_bottom = stats_y - self.label.ascent();
        let hero_cy = (hero_top + hero_bottom) / 2;
        let temp_base = hero_cy - self.big.text_v_center_from_baseline(&temp_str);
        draw_text(&mut img, &self.big, margin, temp_base, fg, &temp_str);

        // Degree ring + unit, set tight against the digits as a single "°F"
        // group. The ring sits at the cap-height shoulder; the unit letter
        // hangs from the same baseline as the numerals. Radius scales with the
        // big font so it reads right on any panel.
        let deg_r = (self.big.height() / 12).max(3);
        let deg_gap = (self.big.height() / 20).max(2);
        let deg_cx = margin + temp_w + deg_gap + deg_r;
        let deg_cy = temp_base - self.big.ascent() + self.big.ascent() / 5 + deg_r;
        degree_ring(&mut img, deg_cx, deg_cy, deg_r, fg);
        // Unit letter sits beside the ring (not down at the digit baseline) so
        // "°F" reads as one compact group at the top-right shoulder.
        let unit_base = deg_cy + self.unit.ascent() / 2;
        draw_text(&mut img, &self.unit, deg_cx + deg_r + deg_gap, unit_base, fg, "F");

        // Condition icon, right-aligned and vertically centered on the same
        // hero midpoint as the numerals.
        let icon_glyph = data.current.icon.glyph.to_string();
        let icon_w = self.icon_big.text_width(&icon_glyph);
        let icon_x = wi - margin - icon_w;
        let icon_base = hero_cy + self.icon_big.height() / 2 - self.icon_big.height() / 8;
        draw_text(&mut img, &self.icon_big, icon_x, icon_base, fg, &icon_glyph);

        // ── Hourly temp trend + sun times, in the hero's empty center ───
        let cs_x = margin + temp_w + (self.big.height() / 3); // clear of the degree group
        let cs_w = icon_x - cs_x - margin;
        if cs_w > 60 {
            let ccx = cs_x + cs_w / 2;
            if !data.hourly.is_empty() {
                let temps: Vec<f32> = data.hourly.iter().take(24).map(|hr| hr.temp).collect();
                let sh = (hero_bottom - hero_top) * 34 / 100;
                let sy = hero_cy - sh / 2 - self.small.height();
                center_text(&mut img, &self.small, ccx, sy - margin / 3, fg, "NEXT 24H");
                sparkline(&mut img, cs_x, sy, cs_w, sh, &temps, fg);
                let sun_y = sy + sh + self.small.height() + margin / 3;
                let sun = format!(
                    "RISE {}   SET {}",
                    data.forecast.sunrise.format("%-H:%M"),
                    data.forecast.sunset.format("%-H:%M")
                );
                center_text(&mut img, &self.small, ccx, sun_y, fg, &sun);
            } else {
                let sun = format!(
                    "RISE {}   SET {}",
                    data.forecast.sunrise.format("%-H:%M"),
                    data.forecast.sunset.format("%-H:%M")
                );
                center_text(&mut img, &self.small, ccx, hero_cy, fg, &sun);
            }
        }

        // ── Forecast strip filling the bottom band ──────────────────────
        if has_forecast {
            draw_line(&mut img, margin, strip_top, wi - margin, strip_top, fg);
            self.draw_forecast(&mut img, data, margin, strip_top, fg);
        }

        img
    }

    /// Draw the multi-day forecast as evenly spaced columns across the bottom
    /// strip (panel width/height read from `img`).
    fn draw_forecast(&self, img: &mut RgbImage, data: &Weather, margin: i32, top: i32, fg: Color) {
        let days: Vec<_> = data.daily.iter().take(5).collect();
        if days.is_empty() {
            return;
        }
        let wi = img.width() as i32;
        let hi = img.height() as i32;
        let usable = wi - 2 * margin;
        let col_w = usable / days.len() as i32;

        // Spread name (top) / icon (middle) / hi-lo (bottom) across the whole
        // band so the strip fills the bottom of the panel instead of hugging a
        // thin centre line.
        let pad = margin;
        let name_base = top + pad + self.small.ascent();
        let hilo_base = hi - pad - (self.small.height() - self.small.ascent());
        let icon_base = (name_base + hilo_base) / 2 + self.icon_small.height() / 3;

        for (i, day) in days.iter().enumerate() {
            let cx = margin + col_w * i as i32 + col_w / 2;
            let name = day.date.format("%a").to_string().to_uppercase();
            center_text(img, &self.small, cx, name_base, fg, &name);
            let glyph = day.icon.glyph.to_string();
            center_text(img, &self.icon_small, cx, icon_base, fg, &glyph);
            let hi_lo = format!("{:.0}/{:.0}", day.high, day.low);
            center_text(img, &self.small, cx, hilo_base, fg, &hi_lo);
        }
    }
}

/// Draw a small two-pixel-thick ring to stand in for a degree symbol.
fn degree_ring(img: &mut RgbImage, cx: i32, cy: i32, r: i32, color: Color) {
    draw_circle(img, cx, cy, r, color);
    draw_circle(img, cx, cy, r - 1, color);
}

#[async_trait]
impl EinkRenderer for EinkWeatherMatrix {
    type Data = Weather;

    fn id(&self) -> &'static str {
        "weather"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(
        &mut self,
        display: &mut EinkDisplay,
        data: &Weather,
    ) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::weather::icon_table;
    use crate::api::weather::model::{
        CurrentWeather, DailyForecast, DayForecast, Weather, WeatherApiSource,
    };
    use chrono::{Local, NaiveDate, TimeZone};

    fn repo_fonts() -> EinkWeatherFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkWeatherFonts {
            body: base.join("04B_03B_.TTF"),
            icon: base.join("weathericons.ttf"),
        }
    }

    fn sample() -> Weather {
        let now = Local.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        Weather {
            api: WeatherApiSource::OpenWeather,
            lat: 37.77,
            lon: -122.41,
            location_name: "San Francisco".into(),
            current: CurrentWeather {
                conditions: "Clear".into(),
                temp: 68.0,
                feels_like: 66.0,
                wind_speed: 9.0,
                humidity: 72,
                precipitation_chance: 10,
                uv: Some(5.0),
                wind_direction_deg: Some(270.0),
                icon: icon_table::SUNNY,
            },
            forecast: DayForecast {
                today_high: 74.0,
                today_low: 56.0,
                sunrise: now,
                sunset: now,
            },
            hourly: vec![],
            daily: (1..=5)
                .map(|i| DailyForecast {
                    date: NaiveDate::from_ymd_opt(2024, 6, 15 + i).unwrap(),
                    high: 74.0 - i as f32,
                    low: 56.0 - i as f32,
                    icon: icon_table::SUNNY,
                    precipitation_chance: 10,
                })
                .collect(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkWeatherMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&sample(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated dashboard, got {lit} lit px");
    }

    #[test]
    fn frame_adapts_to_smaller_panel() {
        let r = EinkWeatherMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&sample(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200);
    }
}
