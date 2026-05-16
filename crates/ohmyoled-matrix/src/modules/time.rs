//! Pure-Rust replacement for the old Python `TimeMatrix`.
//!
//! Renders the current date and clock onto a 64×32 `RgbImage` using a BDF font
//! and pushes the result to the supplied `RGBMatrix`. The original Python
//! version used `PIL.ImageFont.truetype("NotoMono", 10)`; this module uses the
//! crate's BDF text renderer so we stay free of PIL/freetype dependencies.

use crate::graphics::{draw_text, Font};
use crate::{Color, RGBMatrix};
use chrono::Local;
use image::RgbImage;
use std::path::Path;
use std::time::Duration;

/// Default font shipped with this repo at `fonts/4x6.bdf` and installed to
/// `/usr/share/fonts/4x6.bdf` by the build scripts.
pub const DEFAULT_FONT_PATH: &str = "/usr/share/fonts/4x6.bdf";

/// 12-hour vs 24-hour clock format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFormat {
    Twelve,
    TwentyFour,
}

impl Default for TimeFormat {
    fn default() -> Self {
        Self::Twelve
    }
}

impl TimeFormat {
    fn strftime(self) -> &'static str {
        match self {
            Self::Twelve => "%I:%M:%S",
            Self::TwentyFour => "%H:%M:%S",
        }
    }
}

/// Renderer for the time module.
///
/// # Example
/// ```no_run
/// use ohmyoled_matrix::{RGBMatrix, MatrixOptions, Color};
/// use ohmyoled_matrix::modules::TimeMatrix;
/// use std::time::Duration;
///
/// let mut matrix = RGBMatrix::test(MatrixOptions::default());
/// let time = TimeMatrix::new(Color::WHITE, None).unwrap();
/// time.run(&mut matrix, 30, Duration::from_secs(1));
/// ```
pub struct TimeMatrix {
    color: Color,
    font: Font,
    format: TimeFormat,
}

impl TimeMatrix {
    /// Build a new `TimeMatrix`.
    ///
    /// `font_path = None` uses `DEFAULT_FONT_PATH`.
    pub fn new(color: Color, font_path: Option<&Path>) -> Result<Self, String> {
        let path = font_path.unwrap_or(Path::new(DEFAULT_FONT_PATH));
        let mut font = Font::new();
        font.load_font(path)?;
        Ok(Self {
            color,
            font,
            format: TimeFormat::default(),
        })
    }

    /// Override the clock format (12h vs 24h).
    pub fn with_format(mut self, format: TimeFormat) -> Self {
        self.format = format;
        self
    }

    /// Render one frame containing the current date + time.
    pub fn frame(&self) -> RgbImage {
        let now = Local::now();
        let date = now.format("%m/%d/%Y").to_string();
        let clock = now.format(self.format.strftime()).to_string();

        let mut img = RgbImage::new(64, 32);
        let ascent = self.font.ascent();
        draw_text(&mut img, &self.font, 3, 5 + ascent, self.color, &date);
        draw_text(&mut img, &self.font, 8, 16 + ascent, self.color, &clock);
        img
    }

    /// Drive the matrix for `frames` ticks at `interval` per tick.
    ///
    /// Mirrors the old Python `TimeMatrix.render` loop (30 frames, 1s interval).
    pub fn run(&self, matrix: &mut RGBMatrix, frames: usize, interval: Duration) {
        for _ in 0..frames {
            let img = self.frame();
            matrix.set_image(&img, 0, 0);
            std::thread::sleep(interval);
        }
    }

    /// Async variant of `run` that yields between frames via `tokio::time::sleep`.
    ///
    /// Available with the `tokio` feature; required when calling from within an
    /// async runtime so we don't block the executor.
    #[cfg(feature = "tokio")]
    pub async fn run_async(&self, matrix: &mut RGBMatrix, frames: usize, interval: Duration) {
        for _ in 0..frames {
            let img = self.frame();
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_font() -> PathBuf {
        // tests run with CWD = crate dir; walk up to repo root and grab the BDF
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("..");
        p.push("..");
        p.push("fonts");
        p.push("4x6.bdf");
        p
    }

    #[test]
    fn frame_has_correct_dimensions() {
        let tm = TimeMatrix::new(Color::WHITE, Some(&repo_font())).unwrap();
        let img = tm.frame();
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn frame_draws_some_pixels() {
        let tm = TimeMatrix::new(Color::WHITE, Some(&repo_font())).unwrap();
        let img = tm.frame();
        let any_lit = img.pixels().any(|p| p.0 != [0, 0, 0]);
        assert!(any_lit, "expected at least one non-black pixel in time frame");
    }

    #[test]
    fn twelve_vs_twentyfour_format() {
        assert_eq!(TimeFormat::Twelve.strftime(), "%I:%M:%S");
        assert_eq!(TimeFormat::TwentyFour.strftime(), "%H:%M:%S");
    }

    #[test]
    fn missing_font_returns_err() {
        let bogus = Path::new("/nonexistent/path/to/font.bdf");
        assert!(TimeMatrix::new(Color::WHITE, Some(bogus)).is_err());
    }
}