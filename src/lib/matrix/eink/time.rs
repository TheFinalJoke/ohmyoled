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

use crate::matrix::eink::layout::{center_text, scaled_px};
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
/// Fonts are sized for the panel passed at construction (see [`scaled_px`]),
/// so the clock reads the same on a 400×300 4.2" or an 800×480 7.5" sheet.
pub struct EinkTimeMatrix {
    clock: Font,
    meridiem: Font,
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
            clock: Font::load_ttf(&paths.body, scaled_px(150.0, h))?,
            meridiem: Font::load_ttf(&paths.body, scaled_px(34.0, h))?,
            date: Font::load_ttf(&paths.body, scaled_px(34.0, h))?,
            weekday: Font::load_ttf(&paths.body, scaled_px(28.0, h))?,
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
    fn adapts_to_smaller_panel() {
        let r = EinkTimeMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(at(9, 5), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
