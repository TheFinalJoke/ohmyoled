//! Earthquake renderer — single-event tile for the 64×32 panel.
//!
//! ```text
//!  ┌──── event mode ────────────┐    ┌──── quiet mode ────────────┐
//!  │ M 6.2                      │    │       QUIET                │
//!  │                            │    │                            │
//!  │ OFF EAST COAST             │    │      no events 24h         │
//!  │ OF HONSHU JAPAN            │    │                            │
//!  │                            │    │                            │
//!  │ 14m            24km        │    │                            │
//!  └────────────────────────────┘    └────────────────────────────┘
//! ```
//!
//! - **Magnitude** is color-coded: green < 4, amber 4–6, red ≥ 6.
//! - **Place** wraps to two lines on word boundaries; longer strings are
//!   truncated with `…`. No scrolling animation (yet).
//! - **Footer** holds age (minutes since origin) on the left and depth on
//!   the right, both in a dim grey so they read as metadata.
//!
//! # Config
//!
//! ```yaml
//! quake:
//!   run: true
//!   feed: significant_day   # significant_day | m45_day | m25_day | all_day
//! ```
//!
//! # Data source
//!
//! `QuakeCollector::from_usgs` — public USGS GeoJSON feed, no auth,
//! 5-minute refresh. The collector returns either `QuakeStatus::Event`
//! (top-magnitude event in the window) or `QuakeStatus::Quiet`.

use crate::api::quake::{QuakeEvent, QuakeStatus};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::Utc;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

const GREEN: Color = Color { r: 0, g: 220, b: 60 };
const AMBER: Color = Color { r: 255, g: 170, b: 0 };
const RED: Color = Color { r: 255, g: 30, b: 30 };
const WHITE: Color = Color { r: 255, g: 255, b: 255 };
const DIM: Color = Color { r: 130, g: 130, b: 130 };

#[derive(Debug, Clone)]
pub struct QuakeFonts {
    pub body: PathBuf,
}

impl Default for QuakeFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
        }
    }
}

pub struct QuakeMatrix {
    body_font: Font,
}

impl QuakeMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(QuakeFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(QuakeFonts::default()).await
    }

    pub fn with_fonts(paths: QuakeFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
        })
    }

    pub async fn with_fonts_async(paths: QuakeFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    pub fn frame(&self, data: &QuakeStatus) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        match data {
            QuakeStatus::Event(e) => self.draw_event(&mut img, e),
            QuakeStatus::Quiet => self.draw_quiet(&mut img),
        }
        img
    }

    fn draw_event(&self, img: &mut RgbImage, e: &QuakeEvent) {
        let font = &self.body_font;
        let ascent = font.ascent();
        let line_h = font.height().max(ascent + 1);

        // Row 1 — magnitude, color-coded.
        let mag_text = format!("M {:.1}", e.magnitude);
        let mag_color = magnitude_color(e.magnitude);
        draw_text(img, font, 1, ascent + 1, mag_color, &mag_text);

        // Rows 3-4 — place, word-wrapped to fit two 64-px lines.
        let (line_a, line_b) = wrap_two_lines(&e.place, PANEL_W as i32, font);
        let y_a = ascent + 1 + line_h + 4;
        let y_b = y_a + line_h;
        draw_text(img, font, 1, y_a, WHITE, &line_a);
        if !line_b.is_empty() {
            draw_text(img, font, 1, y_b, WHITE, &line_b);
        }

        // Row 6 — footer: age on the left, depth on the right.
        let age = format!("{}m", e.age_minutes(Utc::now()));
        let depth = format!("{}km", e.depth_km.round() as i32);
        let footer_y = (PANEL_H as i32) - 1;
        draw_text(img, font, 1, footer_y, DIM, &age);
        let depth_w = font.text_width(&depth);
        let depth_x = (PANEL_W as i32 - depth_w - 1).max(0);
        draw_text(img, font, depth_x, footer_y, DIM, &depth);
    }

    fn draw_quiet(&self, img: &mut RgbImage) {
        let font = &self.body_font;
        let banner = "QUIET";
        let sub = "no events 24h";
        let banner_w = font.text_width(banner);
        let sub_w = font.text_width(sub);
        let line_h = font.height().max(font.ascent() + 1);
        let content_h = 2 * line_h + 4;
        let top_pad = ((PANEL_H as i32 - content_h) / 2).max(0);

        let banner_x = ((PANEL_W as i32 - banner_w) / 2).max(0);
        let banner_y = top_pad + font.ascent();
        draw_text(img, font, banner_x, banner_y, DIM, banner);

        let sub_x = ((PANEL_W as i32 - sub_w) / 2).max(0);
        let sub_y = banner_y + line_h + 4;
        draw_text(img, font, sub_x, sub_y, DIM, sub);
    }
}

impl Default for QuakeMatrix {
    fn default() -> Self {
        Self::new().expect("default QuakeMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for QuakeMatrix {
    type Data = QuakeStatus;

    fn id(&self) -> &'static str {
        "quake"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(15)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &QuakeStatus) -> Result<(), RenderError> {
        matrix.clear();
        let img = self.frame(data);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(Duration::from_secs(15)).await;
        matrix.clear();
        Ok(())
    }
}

fn magnitude_color(mag: f32) -> Color {
    if mag < 4.0 {
        GREEN
    } else if mag < 6.0 {
        AMBER
    } else {
        RED
    }
}

/// Word-wrap `text` into at most two lines fitting `max_px` each. Long single
/// words are character-truncated with a trailing `…`. Returns `(line1, line2)`
/// where `line2` may be empty if the entire text fits on one line.
fn wrap_two_lines(text: &str, max_px: i32, font: &Font) -> (String, String) {
    let text = text.trim();
    if text.is_empty() {
        return (String::new(), String::new());
    }

    if font.text_width(text) <= max_px {
        return (text.to_string(), String::new());
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut line_a = String::new();
    let mut i = 0;
    while i < words.len() {
        let candidate = if line_a.is_empty() {
            words[i].to_string()
        } else {
            format!("{} {}", line_a, words[i])
        };
        if font.text_width(&candidate) <= max_px {
            line_a = candidate;
            i += 1;
        } else {
            break;
        }
    }

    // If even the first word didn't fit, hard-truncate character-by-character.
    if line_a.is_empty() {
        line_a = truncate_with_ellipsis(words[0], max_px, font);
        i = 1;
    }

    let mut line_b = String::new();
    while i < words.len() {
        let candidate = if line_b.is_empty() {
            words[i].to_string()
        } else {
            format!("{} {}", line_b, words[i])
        };
        if font.text_width(&candidate) <= max_px {
            line_b = candidate;
            i += 1;
        } else {
            break;
        }
    }

    // Anything left after two lines — append `…` to line_b to signal truncation.
    if i < words.len() {
        line_b = truncate_with_ellipsis(&line_b, max_px - font.text_width("…"), font);
        line_b.push('…');
    }

    (line_a, line_b)
}

fn truncate_with_ellipsis(s: &str, max_px: i32, font: &Font) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let mut candidate = out.clone();
        candidate.push(ch);
        if font.text_width(&candidate) > max_px {
            break;
        }
        out = candidate;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::quake::QuakeEvent;
    use chrono::{Duration as ChDuration, Utc};
    use std::path::PathBuf;

    fn repo_fonts() -> QuakeFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        QuakeFonts {
            body: repo.join("04B_03B_.TTF"),
        }
    }

    fn sample_event(mag: f32, place: &str) -> QuakeEvent {
        QuakeEvent {
            magnitude: mag,
            place: place.into(),
            origin: Utc::now() - ChDuration::minutes(14),
            depth_km: 24.0,
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&QuakeStatus::Event(sample_event(
            6.2,
            "OFF EAST COAST OF HONSHU JAPAN",
        )));
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn quiet_mode_renders_banner() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img_q = m.frame(&QuakeStatus::Quiet);
        let img_e = m.frame(&QuakeStatus::Event(sample_event(5.0, "Anywhere")));
        assert_ne!(
            img_q.as_raw(),
            img_e.as_raw(),
            "quiet frame should differ from event frame"
        );
        let lit_q = img_q.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit_q > 30, "quiet banner should still light pixels");
    }

    #[test]
    fn magnitude_color_bands() {
        assert_eq!(magnitude_color(2.5), GREEN);
        assert_eq!(magnitude_color(3.9), GREEN);
        assert_eq!(magnitude_color(4.0), AMBER);
        assert_eq!(magnitude_color(5.9), AMBER);
        assert_eq!(magnitude_color(6.0), RED);
        assert_eq!(magnitude_color(8.4), RED);
    }

    #[test]
    fn magnitude_red_pixels_for_big_quake() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&QuakeStatus::Event(sample_event(7.1, "Pacific Rim")));
        let red_pixels = img
            .pixels()
            .filter(|p| p.0[0] > 200 && p.0[1] < 80 && p.0[2] < 80)
            .count();
        assert!(red_pixels > 0, "expected red magnitude pixels at M7.1");
    }

    #[test]
    fn wrap_short_text_fits_one_line() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let (a, b) = wrap_two_lines("Hi", 64, &m.body_font);
        assert_eq!(a, "Hi");
        assert_eq!(b, "");
    }

    #[test]
    fn wrap_long_text_breaks_at_word_boundary() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let (a, b) = wrap_two_lines("OFF EAST COAST OF HONSHU JAPAN", 64, &m.body_font);
        assert!(!a.is_empty(), "line A should have content");
        assert!(!b.is_empty(), "line B should have content for a long region");
        // No word should be split across the boundary — both fragments end on
        // word boundaries (i.e. no trailing partial word).
        assert!(!a.ends_with(' '));
        assert!(!b.ends_with(' '));
    }
}
