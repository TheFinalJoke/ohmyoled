//! Pure-Rust replacement for the old Python `TimeMatrix`.
//!
//! Renders the current date and clock onto a 64×32 `RgbImage` using a BDF font
//! and pushes the result to the supplied `RGBMatrix`. Acts as the canonical
//! example of the [`crate::api::Collector`] + [`crate::matrix::Renderer`]
//! contract: the data type bridging the two is [`TimeSnapshot`].

use crate::api::error::ApiError;
use crate::api::Collector;
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::Path;
use std::time::Duration;

/// Default font shipped with this repo at `fonts/4x6.bdf` and installed to
/// `/usr/share/fonts/4x6.bdf` by the build scripts.
pub const DEFAULT_FONT_PATH: &str = "/usr/share/fonts/4x6.bdf";

/// 12-hour vs 24-hour clock format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeFormat {
    #[default]
    Twelve,
    TwentyFour,
}

impl TimeFormat {
    fn strftime(self) -> &'static str {
        match self {
            Self::Twelve => "%I:%M:%S",
            Self::TwentyFour => "%H:%M:%S",
        }
    }
}

/// What the time collector hands to the renderer each tick.
#[derive(Debug, Clone)]
pub struct TimeSnapshot {
    pub now: DateTime<Local>,
}

/// Collector for the time module — trivial; just reads the system clock.
pub struct TimeCollector;

impl TimeCollector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimeCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Collector for TimeCollector {
    type Output = TimeSnapshot;

    fn id(&self) -> &'static str {
        "time"
    }

    fn refresh_interval(&self) -> Duration {
        Duration::from_secs(1)
    }

    async fn poll(&self) -> Result<TimeSnapshot, ApiError> {
        Ok(TimeSnapshot { now: Local::now() })
    }
}

/// Renderer for the time module.
///
/// # Example
/// ```no_run
/// use ohmyoled_matrix::{RGBMatrix, MatrixOptions, Color};
/// use oledlib::matrix::TimeMatrix;
///
/// let mut matrix = RGBMatrix::test(MatrixOptions::default());
/// let _time = TimeMatrix::new(Color::WHITE, None).unwrap();
/// ```
pub struct TimeMatrix {
    color: Color,
    font: Font,
    format: TimeFormat,
    /// Number of frames per `render()` call.
    pub frames: usize,
    /// Delay between frames.
    pub interval: Duration,
}

impl TimeMatrix {
    /// Build a new `TimeMatrix` synchronously.
    ///
    /// `font_path = None` uses `DEFAULT_FONT_PATH`. Prefer [`Self::new_async`]
    /// from inside a tokio runtime so font I/O doesn't block.
    pub fn new(color: Color, font_path: Option<&Path>) -> Result<Self, String> {
        let path = font_path.unwrap_or(Path::new(DEFAULT_FONT_PATH));
        let mut font = Font::new();
        font.load_font(path)?;
        Ok(Self {
            color,
            font,
            format: TimeFormat::default(),
            frames: 30,
            interval: Duration::from_secs(1),
        })
    }

    /// Async constructor — moves the blocking font load onto a tokio worker thread.
    pub async fn new_async(color: Color, font_path: Option<&Path>) -> Result<Self, String> {
        let owned = font_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| Path::new(DEFAULT_FONT_PATH).to_path_buf());
        tokio::task::spawn_blocking(move || Self::new(color, Some(&owned)))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Override the clock format (12h vs 24h).
    pub fn with_format(mut self, format: TimeFormat) -> Self {
        self.format = format;
        self
    }

    /// Render one frame containing the supplied moment.
    pub fn frame(&self, now: DateTime<Local>) -> RgbImage {
        let date = now.format("%m/%d/%Y").to_string();
        let clock = now.format(self.format.strftime()).to_string();

        let mut img = RgbImage::new(64, 32);
        let ascent = self.font.ascent();
        draw_text(&mut img, &self.font, 3, 5 + ascent, self.color, &date);
        draw_text(&mut img, &self.font, 8, 16 + ascent, self.color, &clock);
        img
    }
}

#[async_trait]
impl Renderer for TimeMatrix {
    type Data = TimeSnapshot;

    fn id(&self) -> &'static str {
        "time"
    }

    fn cycle_duration(&self) -> Duration {
        self.interval * self.frames as u32
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &TimeSnapshot) -> Result<(), RenderError> {
        // Refresh the displayed time each frame; `data.now` is the snapshot
        // captured at poll() time but we keep the frame-rate refresh so the
        // seconds tick. The collector polls every second, so this is a clean
        // cycle where each tick uses the *current* clock for that frame.
        let _ = data;
        for _ in 0..self.frames {
            let img = self.frame(Local::now());
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(self.interval).await;
        }
        matrix.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_font() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("fonts");
        p.push("4x6.bdf");
        p
    }

    #[test]
    fn frame_has_correct_dimensions() {
        let tm = TimeMatrix::new(Color::WHITE, Some(&repo_font())).unwrap();
        let img = tm.frame(Local::now());
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn frame_draws_some_pixels() {
        let tm = TimeMatrix::new(Color::WHITE, Some(&repo_font())).unwrap();
        let img = tm.frame(Local::now());
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
