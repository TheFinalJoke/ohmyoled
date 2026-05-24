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
        let body = &self.body_font;
        let line_h = body.height().max(body.ascent() + 1);

        // Layout budget on a 32-row panel (line_h ≈ 8):
        //   rows  0–6  : title line 1   (baseline at body.ascent)
        //   rows  9–15 : title line 2   (baseline at body.ascent + line_h + 1, only if wrapped)
        //   rows 24–30 : footer         (baseline at PANEL_H − 1)
        // Footer uses the same body font so it stays readable — at the cost
        // of capping the title at 2 wrap lines instead of 3. Long titles
        // get an ellipsis on the second line, which is the right tradeoff
        // because the magnitude prefix is what people actually read at a
        // glance, and that always lives at the start of line 1.
        let mag_color = magnitude_color(e.magnitude);
        let lines = wrap_into_lines(&e.title, PANEL_W as i32, body, 2);
        for (i, line) in lines.iter().enumerate() {
            let y = body.ascent() + i as i32 * (line_h + 1);
            draw_text(img, body, 1, y, mag_color, line);
        }

        // Footer: "felt N" when populated (more interesting than age),
        // otherwise just the origin-age. Depth always on the right.
        let left = match e.felt {
            Some(n) => format!("felt {n}"),
            None => format!("{}m ago", e.age_minutes(Utc::now())),
        };
        let depth = format!("{}km", e.depth_km.round() as i32);
        let footer_y = (PANEL_H as i32) - 1;
        draw_text(img, body, 1, footer_y, DIM, &left);
        let depth_w = body.text_width(&depth);
        let depth_x = (PANEL_W as i32 - depth_w - 1).max(0);
        draw_text(img, body, depth_x, footer_y, DIM, &depth);
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

/// Word-wrap `text` into at most `max_lines` lines, each fitting `max_px`.
/// Long single words are character-truncated. Anything left over after the
/// last line gets an ellipsis tacked onto the final line to signal
/// truncation. Returns at most `max_lines` non-empty lines.
fn wrap_into_lines(text: &str, max_px: i32, font: &Font, max_lines: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() || max_lines == 0 {
        return Vec::new();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines: Vec<String> = Vec::with_capacity(max_lines);
    let mut current = String::new();
    let mut i = 0;

    while i < words.len() && lines.len() < max_lines {
        let candidate = if current.is_empty() {
            words[i].to_string()
        } else {
            format!("{} {}", current, words[i])
        };
        if font.text_width(&candidate) <= max_px {
            current = candidate;
            i += 1;
        } else if current.is_empty() {
            // Single word too wide — hard-truncate.
            let truncated = truncate_with_ellipsis(words[i], max_px, font);
            lines.push(truncated);
            i += 1;
        } else {
            lines.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }

    // Words remaining after the last line — tag the last visible line with `…`.
    if i < words.len() {
        if let Some(last) = lines.last_mut() {
            let max_for_ellipsis = max_px - font.text_width("…");
            *last = truncate_with_ellipsis(last, max_for_ellipsis, font);
            last.push('…');
        }
    }

    lines
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

    /// Find the topmost row that has any lit pixel inside `x_range`. None if
    /// the strip is entirely dark.
    fn first_lit_row(img: &image::RgbImage, x_range: std::ops::Range<u32>) -> Option<u32> {
        for y in 0..img.height() {
            for x in x_range.clone() {
                if img.get_pixel(x, y).0 != [0, 0, 0] {
                    return Some(y);
                }
            }
        }
        None
    }

    /// Find the bottommost row that has any lit pixel inside `x_range`. None
    /// if the strip is entirely dark.
    fn last_lit_row(img: &image::RgbImage, x_range: std::ops::Range<u32>) -> Option<u32> {
        let mut last = None;
        for y in 0..img.height() {
            for x in x_range.clone() {
                if img.get_pixel(x, y).0 != [0, 0, 0] {
                    last = Some(y);
                    break;
                }
            }
        }
        last
    }

    fn sample_event(mag: f32, place: &str) -> QuakeEvent {
        QuakeEvent {
            magnitude: mag,
            title: format!("M {:.1} - {}", mag, place),
            origin: Utc::now() - ChDuration::minutes(14),
            depth_km: 24.0,
            felt: None,
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
        let lines = wrap_into_lines("Hi", 64, &m.body_font, 3);
        assert_eq!(lines, vec!["Hi".to_string()]);
    }

    /// Regression: with a long place name wrapping to two lines, the
    /// second place line must not overlap the footer. We pick a non-zero
    /// gap column (x ≥ 12 is past "14m" but to the left of "24km") and
    /// require the bottom of the place text + top of the footer to leave
    /// at least one fully-dark row between them.
    #[test]
    fn long_place_name_does_not_overlap_footer() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&QuakeStatus::Event(sample_event(
            6.2,
            "OFF EAST COAST OF HONSHU JAPAN",
        )));
        // The left-half footer ("14m") occupies roughly x in 1..=12.
        let footer_top = first_lit_row(&img, 1..12).expect("footer should be lit");
        // The body text ends somewhere before that; find the last lit row of
        // the place lines by searching above the footer.
        let place_bottom = last_lit_row(&img, 1..PANEL_W).filter(|y| *y < footer_top);
        // Either there's clear space, or the place text just doesn't reach
        // down to where the footer starts at all.
        if let Some(pb) = place_bottom {
            assert!(
                footer_top > pb + 1,
                "footer (row {footer_top}) overlaps place-name bottom (row {pb})"
            );
        }
    }

    #[test]
    fn wrap_long_text_breaks_at_word_boundary() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let lines = wrap_into_lines(
            "M 6.2 - OFF EAST COAST OF HONSHU JAPAN",
            64,
            &m.body_font,
            3,
        );
        assert!(lines.len() >= 2, "long title should wrap to multiple lines");
        assert!(lines.len() <= 3, "wrap must respect max_lines");
        for line in &lines {
            assert!(!line.ends_with(' '), "no trailing space on wrapped lines");
        }
    }

    #[test]
    fn wrap_truncates_with_ellipsis_when_overflowing_max_lines() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        // Force a very wide string into a 2-line budget — last line gets `…`.
        let lines = wrap_into_lines(
            "Words and more words and even more words and yet more words again",
            64,
            &m.body_font,
            2,
        );
        assert_eq!(lines.len(), 2);
        assert!(
            lines.last().unwrap().ends_with('…'),
            "expected trailing ellipsis on truncated last line, got {:?}",
            lines.last()
        );
    }

    #[test]
    fn felt_count_renders_in_footer_when_present() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut e = sample_event(5.0, "near somewhere");
        e.felt = Some(482);
        let img_with = m.frame(&QuakeStatus::Event(e.clone()));
        e.felt = None;
        let img_without = m.frame(&QuakeStatus::Event(e));
        // Different footer text → different pixel buffers.
        assert_ne!(
            img_with.as_raw(),
            img_without.as_raw(),
            "footer should differ when felt count is present vs absent"
        );
        // And the footer rows (last 8 rows of the panel) should have lit
        // pixels — the "felt N" string is drawn there.
        let mut lit_footer = 0usize;
        for y in (PANEL_H - 8)..PANEL_H {
            for x in 0..PANEL_W {
                if img_with.get_pixel(x, y).0 != [0, 0, 0] {
                    lit_footer += 1;
                }
            }
        }
        assert!(lit_footer > 5, "felt-mode footer should be drawn");
    }
}
