//! E-paper time renderer — a large ticking clock + date screen.
//!
//! The e-paper counterpart to [`crate::matrix::time::TimeMatrix`]. While the
//! time module owns the panel it **ticks every second**: `HH:MM` is drawn big
//! with smaller live `SS`, and the analog dial has a sweeping second hand. The
//! first frame is a clean clear+draw; each subsequent second uses the panel's
//! fast partial refresh (no white flash) via [`EinkDisplay::show_fast`].
//! Numerals are drawn big in the project pixel font, which stays crisp at large
//! sizes on a monochrome panel.
//!
//! Reuses the shared `draw_*` primitives and the same `TimeFormat` (12h/24h)
//! as the LED tile. Composed white-on-black; the display inverts to
//! black-ink-on-white.
//!
//! # Config
//!
//! Lives under the independent `eink` display block, reusing the `time`
//! section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     time:
//!       run: true
//!       color: [255, 255, 255]     # ignored on B/W; kept for config parity
//!       time_format: "12h"         # "12h" | "24h"
//! ```

use crate::matrix::eink::layout::{center_text, fill_rect, scaled_px};
use crate::matrix::eink_renderer::{sleep_to_next_second, EinkRenderer};
use crate::matrix::error::RenderError;
use crate::matrix::time::{TimeFormat, TimeSnapshot};
use async_trait::async_trait;
use chrono::{DateTime, Local, Timelike};
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_circle, draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper time renderer.
pub struct EinkTimeFonts {
    /// The pixel font used for every line at different sizes.
    pub body: PathBuf,
}

impl Default for EinkTimeFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper clock renderer.
///
/// Fonts are sized for the panel passed at construction (see [`scaled_px`]),
/// so the clock reads the same on a 400×300 4.2" or an 800×480 7.5" sheet.
pub struct EinkTimeMatrix {
    clock: Font,
    meridiem: Font,
    /// Smaller font for the live seconds beside `HH:MM`.
    seconds: Font,
    date: Font,
    weekday: Font,
    format: TimeFormat,
}

impl EinkTimeMatrix {
    /// Sync constructor (useful for tests). `dims` is the target panel size.
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkTimeFonts::default(), dims)
    }

    /// Async constructor used by the registry.
    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkTimeFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkTimeFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            clock: Font::load_ttf(&paths.body, scaled_px(136.0, h))?,
            meridiem: Font::load_ttf(&paths.body, scaled_px(36.0, h))?,
            seconds: Font::load_ttf(&paths.body, scaled_px(48.0, h))?,
            date: Font::load_ttf(&paths.body, scaled_px(40.0, h))?,
            weekday: Font::load_ttf(&paths.body, scaled_px(34.0, h))?,
            format: TimeFormat::default(),
        })
    }

    pub async fn with_fonts_async(paths: EinkTimeFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Override the clock format (12h vs 24h).
    pub fn with_format(mut self, format: TimeFormat) -> Self {
        self.format = format;
        self
    }

    /// Compose the clock screen at `w × h`.
    pub fn frame(&self, now: DateTime<Local>, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let cx = wi / 2;

        // Clock string without seconds — e-paper updates per minute.
        let (clock_str, meridiem) = match self.format {
            TimeFormat::Twelve => (
                now.format("%-I:%M").to_string(),
                now.format("%p").to_string(),
            ),
            TimeFormat::TwentyFour => (now.format("%H:%M").to_string(), String::new()),
        };

        // ── Weekday across the top ──────────────────────────────────────
        let weekday = now.format("%A").to_string().to_uppercase();
        let wk_y = hi / 8 + self.weekday.ascent() / 2;
        center_text(&mut img, &self.weekday, cx, wk_y, fg, &weekday);
        let rule_y = wk_y + self.weekday.height() / 2 + 6;
        draw_line(&mut img, wi / 5, rule_y, wi - wi / 5, rule_y, fg);

        let date_y = hi - hi / 8;
        let band_cy = (rule_y + date_y - self.date.height()) / 2;

        // ── Analog clock on the left ────────────────────────────────────
        let gap = wi / 24;
        let acx = wi * 22 / 100;
        let radius = (wi * 15 / 100).min((date_y - self.date.height() - rule_y) / 2 - 4).max(20);
        self.draw_analog(&mut img, acx, band_cy, radius, now, fg);

        // ── Digital clock centered in the space to the right of the dial ─
        // A small right-hand column holds the live seconds, with the meridiem
        // (12h only) riding above them.
        let sec_str = now.format("%S").to_string();
        let clock_w = self.clock.text_width(&clock_str);
        let mer_gap = self.clock.height() / 16;
        let mer_w = if meridiem.is_empty() {
            0
        } else {
            self.meridiem.text_width(&meridiem)
        };
        let sec_w = self.seconds.text_width(&sec_str);
        let col_w = mer_w.max(sec_w);
        let dial_right = acx + radius + radius / 4; // clear of the rays
        let region_right = wi - wi / 24;
        let dcx = (dial_right + gap + region_right) / 2;
        let clock_x = dcx - (clock_w + mer_gap + col_w) / 2;
        let clock_base = band_cy - self.clock.text_v_center_from_baseline(&clock_str);
        draw_text(&mut img, &self.clock, clock_x, clock_base, fg, &clock_str);
        // Seconds sit at the big digits' baseline; meridiem rides above them.
        let col_x = clock_x + clock_w + mer_gap;
        draw_text(&mut img, &self.seconds, col_x, clock_base, fg, &sec_str);
        if !meridiem.is_empty() {
            let my = clock_base - self.clock.ascent() + self.clock.ascent() / 4 + self.meridiem.ascent();
            draw_text(&mut img, &self.meridiem, col_x, my, fg, &meridiem);
        }

        // ── Full date across the bottom ─────────────────────────────────
        let date = now.format("%B %-d, %Y").to_string();
        center_text(&mut img, &self.date, cx, date_y, fg, &date);

        img
    }

    /// Draw an analog clock face (ring, hour ticks, hour + minute hands).
    fn draw_analog(&self, img: &mut RgbImage, cx: i32, cy: i32, r: i32, now: DateTime<Local>, fg: Color) {
        use std::f32::consts::TAU;
        draw_circle(img, cx, cy, r, fg);
        draw_circle(img, cx, cy, r - 1, fg);
        let rf = r as f32;
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU;
            let (s, c) = (a.sin(), a.cos());
            // Quarter ticks longer than the rest.
            let inner = if k % 3 == 0 { 0.80 } else { 0.88 };
            draw_line(
                img,
                cx + (rf * inner * s) as i32,
                cy - (rf * inner * c) as i32,
                cx + (rf * 0.97 * s) as i32,
                cy - (rf * 0.97 * c) as i32,
                fg,
            );
        }
        let (h, m, s) = (now.hour() % 12, now.minute(), now.second());
        let hr_a = (h as f32 + m as f32 / 60.0) / 12.0 * TAU;
        let min_a = m as f32 / 60.0 * TAU;
        let sec_a = s as f32 / 60.0 * TAU;
        self.hand(img, cx, cy, hr_a, rf * 0.5, 2, fg);
        self.hand(img, cx, cy, min_a, rf * 0.82, 1, fg);
        // Thin, long second hand sweeping to near the rim.
        self.hand(img, cx, cy, sec_a, rf * 0.9, 0, fg);
        fill_rect(img, cx - 2, cy - 2, 5, 5, fg);
    }

    /// Draw a clock hand from the center at `angle` (radians, clockwise from
    /// 12 o'clock), `len` px long, `thick` px of extra width each side.
    #[allow(clippy::too_many_arguments)]
    fn hand(&self, img: &mut RgbImage, cx: i32, cy: i32, angle: f32, len: f32, thick: i32, fg: Color) {
        let (s, c) = (angle.sin(), angle.cos());
        let (ex, ey) = (cx + (len * s) as i32, cy - (len * c) as i32);
        for t in -thick..=thick {
            let (ox, oy) = ((t as f32 * c) as i32, (t as f32 * s) as i32);
            draw_line(img, cx + ox, cy + oy, ex + ox, ey + oy, fg);
        }
    }
}

#[async_trait]
impl EinkRenderer for EinkTimeMatrix {
    type Data = TimeSnapshot;

    fn id(&self) -> &'static str {
        "time"
    }

    fn cycle_duration(&self) -> Duration {
        // How long the clock holds the panel and ticks seconds before yielding
        // to the next module.
        Duration::from_secs(60)
    }

    async fn render(
        &mut self,
        display: &mut EinkDisplay,
        data: &TimeSnapshot,
    ) -> Result<(), RenderError> {
        let _ = data;
        let (w, h) = (display.width(), display.height());
        // Clean baseline: one clear+draw so the screen starts ghost-free.
        display.show(&self.frame(Local::now(), w, h));
        // Then tick once a second using the fast (no-flash) partial refresh.
        let deadline = Instant::now() + self.cycle_duration();
        while Instant::now() < deadline {
            sleep_to_next_second().await;
            display.show_fast(&self.frame(Local::now(), w, h));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn repo_fonts() -> EinkTimeFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkTimeFonts {
            body: base.join("04B_03B_.TTF"),
        }
    }

    fn at(h: u32, m: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 6, 6, h, m, 0).unwrap()
    }

    fn at_s(h: u32, m: u32, s: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 6, 6, h, m, s).unwrap()
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkTimeMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(at(20, 42), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated clock, got {lit} lit px");
    }

    #[test]
    fn format_changes_the_rendered_clock() {
        // 12h ("8:42 PM") and 24h ("20:42") must produce visibly different
        // frames at the same instant — proves the format is actually applied.
        let twelve = EinkTimeMatrix::with_fonts(repo_fonts(), (800, 480))
            .unwrap()
            .with_format(TimeFormat::Twelve);
        let twentyfour = EinkTimeMatrix::with_fonts(repo_fonts(), (800, 480))
            .unwrap()
            .with_format(TimeFormat::TwentyFour);
        let n = at(20, 42);
        let f12 = twelve.frame(n, 800, 480);
        let f24 = twentyfour.frame(n, 800, 480);
        assert!(f12.pixels().any(|p| p.0 != [0, 0, 0]));
        assert!(f24.pixels().any(|p| p.0 != [0, 0, 0]));
        assert_ne!(f12.into_raw(), f24.into_raw(), "12h vs 24h should differ");
    }

    #[test]
    fn seconds_change_the_frame() {
        // The live seconds digits and the analog second hand must make the
        // frame differ second-to-second — proves the per-second tick renders.
        let r = EinkTimeMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let f00 = r.frame(at_s(20, 42, 0), 800, 480);
        let f30 = r.frame(at_s(20, 42, 30), 800, 480);
        assert_ne!(
            f00.into_raw(),
            f30.into_raw(),
            "second 0 vs 30 should render differently"
        );
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkTimeMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(at(9, 5), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
