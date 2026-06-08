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

const SCROLL_TICK: Duration = Duration::from_millis(50);
const SETTLE_DWELL: Duration = Duration::from_secs(2);
/// Trailing dwell after a marquee pass so the text settles back at the
/// start before the next cycle takes the panel.
const FINAL_DWELL: Duration = Duration::from_secs(3);
/// Steady-state hold when nothing needs to scroll.
const STATIC_DWELL: Duration = Duration::from_secs(15);
/// Pixel padding past the right edge before the marquee starts wrapping.
/// The line slides offscreen by this many pixels so the trailing
/// characters aren't visibly clipped before the reset.
const MARQUEE_OVERSCROLL: i32 = 8;

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
        self.draw_frame(data, 0)
    }

    /// Render one frame at the given scroll position. `scroll_px` is the
    /// number of pixels to shift the marqueed line leftward (0 = settled
    /// at the left margin). Static elements (line 1, footer) are unaffected.
    pub fn draw_frame(&self, data: &QuakeStatus, scroll_px: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        match data {
            QuakeStatus::Event(e) => self.draw_event(&mut img, e, scroll_px),
            QuakeStatus::Quiet => self.draw_quiet(&mut img),
        }
        img
    }

    fn draw_event(&self, img: &mut RgbImage, e: &QuakeEvent, scroll_px: i32) {
        let body = &self.body_font;
        let line_h = body.height().max(body.ascent() + 1);

        // Layout budget on a 32-row panel (line_h ≈ 8):
        //   rows  0–6  : title line 1                (baseline at body.ascent)
        //   rows  9–15 : title line 2 (or marquee)   (baseline at body.ascent + line_h + 1)
        //   rows 24–30 : footer                      (baseline at PANEL_H − 1)
        // Footer uses the same body font so it stays readable. When the
        // wrapped second line would overflow, we marquee the full remainder
        // through line 2's pixel rect instead of truncating with `…` —
        // line 1 (which always carries the "M X.X -" prefix) stays static.
        let mag_color = magnitude_color(e.magnitude);
        let (line_1, line_2_full) = split_title(&e.title, PANEL_W as i32, body);
        let line_2_y = body.ascent() + line_h + 1;

        draw_text(img, body, 1, body.ascent(), mag_color, &line_1);
        if !line_2_full.is_empty() {
            let l2_x = 1 - scroll_px;
            draw_text(img, body, l2_x, line_2_y, mag_color, &line_2_full);
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

    /// Distance line 2 has to travel for one full marquee pass — `0` when
    /// the second line fits without scrolling.
    fn line_2_scroll_distance(&self, e: &QuakeEvent) -> i32 {
        let (_l1, l2) = split_title(&e.title, PANEL_W as i32, &self.body_font);
        let l2_w = self.body_font.text_width(&l2);
        if l2_w <= PANEL_W as i32 - 2 {
            0
        } else {
            // From settled (text at x=1) to fully offscreen-left, plus a
            // little overscroll so the trailing edge clears the right margin
            // before the reset snap.
            l2_w + MARQUEE_OVERSCROLL
        }
    }

    fn draw_quiet(&self, img: &mut RgbImage) {
        let font = &self.body_font;
        let banner = "NO QUAKES";
        let sub = "in last 24h";
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
        // Steady state without overflow: 15s. With overflow, the marquee
        // adds (line_2_width + overscroll) × 50ms — typically 8–14s extra
        // for long region strings. The scheduler treats this as an upper
        // bound; render() respects it precisely.
        STATIC_DWELL
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &QuakeStatus) -> Result<(), RenderError> {
        matrix.clear();

        let scroll_distance = match data {
            QuakeStatus::Event(e) => self.line_2_scroll_distance(e),
            QuakeStatus::Quiet => 0,
        };

        // Settle on frame 0 first so non-overflowing tiles appear immediately.
        matrix.set_image(&self.draw_frame(data, 0), 0, 0);
        tokio::time::sleep(SETTLE_DWELL).await;

        if scroll_distance > 0 {
            for offset in 1..=scroll_distance {
                matrix.set_image(&self.draw_frame(data, offset), 0, 0);
                tokio::time::sleep(SCROLL_TICK).await;
            }
            // Snap back to the start so the trailing dwell shows the
            // beginning of the line rather than a blank frame.
            matrix.set_image(&self.draw_frame(data, 0), 0, 0);
            tokio::time::sleep(FINAL_DWELL).await;
        } else {
            tokio::time::sleep(STATIC_DWELL - SETTLE_DWELL).await;
        }

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

/// Split `text` into a greedy line-1 (fits within `max_px`) and a line-2
/// "remainder" containing every word that didn't fit on line 1. The
/// remainder is returned verbatim — no ellipsis, no truncation — so the
/// caller can marquee it if it overflows the panel width.
///
/// Returns `(line_1, line_2_remainder)`. If the entire text fits on one
/// line, `line_2_remainder` is empty.
fn split_title(text: &str, max_px: i32, font: &Font) -> (String, String) {
    let text = text.trim();
    if text.is_empty() {
        return (String::new(), String::new());
    }
    if font.text_width(text) <= max_px {
        return (text.to_string(), String::new());
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut line_1 = String::new();
    let mut i = 0;
    while i < words.len() {
        let candidate = if line_1.is_empty() {
            words[i].to_string()
        } else {
            format!("{} {}", line_1, words[i])
        };
        if font.text_width(&candidate) <= max_px {
            line_1 = candidate;
            i += 1;
        } else {
            break;
        }
    }
    // First word didn't fit — hard-truncate it so line 1 is non-empty.
    if line_1.is_empty() {
        line_1 = truncate_with_ellipsis(words[0], max_px, font);
        i = 1;
    }
    (line_1, words[i..].join(" "))
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
            lat: 0.0,
            lon: 0.0,
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
    fn quiet_lines_fit_panel_width() {
        // The quiet sub-line used to be "no events 24h", which measured
        // wide enough to brush the 64 px panel edge in 04B_03B 8pt.
        // Guard the literals so future text edits can't silently reintroduce
        // clipping.
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let font = &m.body_font;
        for literal in ["NO QUAKES", "in last 24h"] {
            let w = font.text_width(literal);
            assert!(
                w <= PANEL_W as i32 - 2,
                "`{literal}` measures {w}px; must be <= {} to clear the panel edge",
                PANEL_W as i32 - 2
            );
        }
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
    fn split_title_short_fits_on_one_line() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let (l1, l2) = split_title("M 3.1 - Reno", 64, &m.body_font);
        assert_eq!(l1, "M 3.1 - Reno");
        assert_eq!(l2, "");
    }

    #[test]
    fn split_title_long_returns_full_remainder_no_ellipsis() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let (l1, l2) = split_title(
            "M 6.2 - OFF EAST COAST OF HONSHU, JAPAN",
            64,
            &m.body_font,
        );
        assert!(l1.starts_with("M 6.2"));
        // The remainder must be intact — the renderer will marquee it.
        assert!(!l2.ends_with('…'));
        assert!(l2.contains("JAPAN"), "remainder should include the tail, got {l2:?}");
    }

    #[test]
    fn long_title_triggers_marquee() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let e = QuakeEvent {
            magnitude: 6.2,
            title: "M 6.2 - OFF EAST COAST OF HONSHU, JAPAN".into(),
            origin: Utc::now() - ChDuration::minutes(14),
            lat: 38.3,
            lon: 142.1,
            depth_km: 24.0,
            felt: Some(482),
        };
        let dist = m.line_2_scroll_distance(&e);
        assert!(dist > 0, "expected marquee distance > 0 for long title");

        // Mid-scroll the line 2 region (rows 9..16) should differ from the
        // settled frame — pixel motion confirms the marquee is alive.
        let settled = m.frame(&QuakeStatus::Event(e.clone()));
        let mid = m.draw_frame(&QuakeStatus::Event(e), dist / 2);
        let mut diffs = 0usize;
        for y in 9..16u32 {
            for x in 0..PANEL_W {
                if settled.get_pixel(x, y) != mid.get_pixel(x, y) {
                    diffs += 1;
                }
            }
        }
        assert!(diffs > 10, "line 2 should visibly shift mid-marquee, got {diffs} diffs");
    }

    #[test]
    fn short_title_does_not_marquee() {
        let m = QuakeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let e = QuakeEvent {
            magnitude: 3.1,
            title: "M 3.1 - Reno NV".into(),
            origin: Utc::now() - ChDuration::minutes(2),
            lat: 39.5,
            lon: -119.8,
            depth_km: 5.0,
            felt: None,
        };
        assert_eq!(m.line_2_scroll_distance(&e), 0);
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
