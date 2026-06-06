//! E-paper time renderer — a large static clock + date screen.
//!
//! The e-paper counterpart to [`crate::matrix::time::TimeMatrix`]. E-paper
//! refreshes slowly and holds its image, so there's no per-second tick: the
//! screen shows `HH:MM` (plus weekday and full date) and refreshes about once
//! a minute. Numerals are drawn big in the project pixel font, which stays
//! crisp at large sizes on a monochrome panel.
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

use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use crate::matrix::time::{TimeFormat, TimeSnapshot};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

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
/// Font sizes are tuned for a 4.2" (400×300) panel; positions derive from the
/// live `display.width()/height()` so larger panels fill correctly.
pub struct EinkTimeMatrix {
    clock: Font,
    meridiem: Font,
    date: Font,
    weekday: Font,
    format: TimeFormat,
}

impl EinkTimeMatrix {
    /// Sync constructor (useful for tests).
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(EinkTimeFonts::default())
    }

    /// Async constructor used by the registry.
    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(EinkTimeFonts::default()).await
    }

    pub fn with_fonts(paths: EinkTimeFonts) -> Result<Self, String> {
        Ok(Self {
            clock: Font::load_ttf(&paths.body, 92.0)?,
            meridiem: Font::load_ttf(&paths.body, 22.0)?,
            date: Font::load_ttf(&paths.body, 22.0)?,
            weekday: Font::load_ttf(&paths.body, 18.0)?,
            format: TimeFormat::default(),
        })
    }

    pub async fn with_fonts_async(paths: EinkTimeFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
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
        let wk_y = hi / 6 + self.weekday.ascent() / 2;
        center_text(&mut img, &self.weekday, cx, wk_y, fg, &weekday);
        let rule_y = wk_y + self.weekday.height() / 2 + 4;
        draw_line(&mut img, wi / 4, rule_y, wi - wi / 4, rule_y, fg);

        // ── Big clock, vertically centered ──────────────────────────────
        let clock_w = self.clock.text_width(&clock_str);
        let clock_x = cx - clock_w / 2;
        let clock_base = hi / 2 + self.clock.ascent() / 2;
        draw_text(&mut img, &self.clock, clock_x, clock_base, fg, &clock_str);
        // AM/PM tucked to the top-right of the numerals (12h only).
        if !meridiem.is_empty() {
            let mx = clock_x + clock_w + 6;
            let my = clock_base - self.clock.ascent() + self.meridiem.ascent();
            draw_text(&mut img, &self.meridiem, mx, my, fg, &meridiem);
        }

        // ── Full date across the bottom ─────────────────────────────────
        let date = now.format("%B %-d, %Y").to_string();
        let date_y = hi - hi / 6;
        center_text(&mut img, &self.date, cx, date_y, fg, &date);

        img
    }
}

/// Center `text` horizontally on `cx` at baseline `y`.
fn center_text(img: &mut RgbImage, font: &Font, cx: i32, y: i32, color: Color, text: &str) {
    let tw = font.text_width(text);
    draw_text(img, font, cx - tw / 2, y, color, text);
}

#[async_trait]
impl EinkRenderer for EinkTimeMatrix {
    type Data = TimeSnapshot;

    fn id(&self) -> &'static str {
        "time"
    }

    fn cycle_duration(&self) -> Duration {
        // Refresh about once a minute — e-paper is slow and the clock only
        // shows minutes.
        Duration::from_secs(60)
    }

    async fn render(
        &mut self,
        display: &mut EinkDisplay,
        data: &TimeSnapshot,
    ) -> Result<(), RenderError> {
        let _ = data;
        // Sample the clock at render time so the displayed minute is current.
        let img = self.frame(Local::now(), display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
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

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkTimeMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img = r.frame(at(20, 42), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated clock, got {lit} lit px");
    }

    #[test]
    fn format_changes_the_rendered_clock() {
        // 12h ("8:42 PM") and 24h ("20:42") must produce visibly different
        // frames at the same instant — proves the format is actually applied.
        let twelve = EinkTimeMatrix::with_fonts(repo_fonts())
            .unwrap()
            .with_format(TimeFormat::Twelve);
        let twentyfour = EinkTimeMatrix::with_fonts(repo_fonts())
            .unwrap()
            .with_format(TimeFormat::TwentyFour);
        let n = at(20, 42);
        let f12 = twelve.frame(n, 400, 300);
        let f24 = twentyfour.frame(n, 400, 300);
        assert!(f12.pixels().any(|p| p.0 != [0, 0, 0]));
        assert!(f24.pixels().any(|p| p.0 != [0, 0, 0]));
        assert_ne!(f12.into_raw(), f24.into_raw(), "12h vs 24h should differ");
    }

    #[test]
    fn adapts_to_larger_panel() {
        let r = EinkTimeMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img = r.frame(at(9, 5), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
    }
}
