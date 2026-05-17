//! Golf renderer — tournament name + status header, 5-row leaderboard.
//!
//! # Layout (64×32)
//!
//! ```text
//!   ┌──────────────────────────────────────┐
//!   │ TOURNAMENT NAME (scrolling)          │ rows 0..8
//!   │ status: round / cut / final          │ rows 8..14
//!   ├──────────────────────────────────────┤
//!   │ 1  PLAYER       -12                  │
//!   │ 2  PLAYER        -8                  │ rows 14..32
//!   │ 3  PLAYER        -6                  │ — top 5 leaderboard
//!   │ 4  PLAYER        -4                  │
//!   │ 5  PLAYER        -3                  │
//!   └──────────────────────────────────────┘
//! ```
//!
//! Score colors: red ⇐ under par, white ⇐ even, yellow ⇐ over par.
//! Off-season shows a two-line placeholder for ~20s.
//!
//! # Config
//!
//! Golf lives under the unified `sport` array, discriminated by
//! `"sport": "golf"`. The `tour` field selects which ESPN scoreboard to
//! pull from; default is `pga`.
//!
//! ```yaml
//! sport:
//!   - run: true
//!     sport: golf
//!     tour: pga          # pga | lpga | champions | korn | liv
//! ```
//!
//! # Data source
//!
//! `GolfCollector::from_espn(tour)` polls ESPN's public scoreboard endpoint
//! (`site.api.espn.com/.../scoreboard`). No API key required.

use crate::api::golf::{GolfData, LeaderboardEntry};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

const SCROLL_FRAMES: u32 = 400;
const SCROLL_TICK: Duration = Duration::from_millis(50);
const FIRST_FRAME_DWELL: Duration = Duration::from_secs(3);
const FINAL_DWELL: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct GolfFonts {
    pub body: PathBuf,
}

impl Default for GolfFonts {
    fn default() -> Self {
        Self { body: "/usr/share/fonts/04B_03B_.TTF".into() }
    }
}

pub struct GolfMatrix {
    body_font: Font,
}

impl GolfMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(GolfFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(GolfFonts::default()).await
    }

    pub fn with_fonts(paths: GolfFonts) -> Result<Self, String> {
        Ok(Self { body_font: Font::load_ttf(&paths.body, 8.0)? })
    }

    pub async fn with_fonts_async(paths: GolfFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }
}

impl Default for GolfMatrix {
    fn default() -> Self {
        Self::new().expect("default GolfMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for GolfMatrix {
    type Data = GolfData;

    fn id(&self) -> &'static str {
        "golf"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(35)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &GolfData) -> Result<(), RenderError> {
        matrix.clear();
        if data.is_offseason() {
            let img = self.draw_offseason(data);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(Duration::from_secs(20)).await;
            matrix.clear();
            return Ok(());
        }

        for xpos in 0..SCROLL_FRAMES {
            let img = self.draw_frame(data, xpos as i32);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(if xpos == 0 { FIRST_FRAME_DWELL } else { SCROLL_TICK }).await;
        }
        let img = self.draw_frame(data, 0);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(FINAL_DWELL).await;
        matrix.clear();
        Ok(())
    }
}

impl GolfMatrix {
    /// One composed frame. Public for examples and tests.
    pub fn draw_frame(&self, data: &GolfData, xpos: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        self.draw_header(&mut img, data, xpos);
        self.draw_leaderboard(&mut img, data);
        img
    }

    /// Two-line header: tour code + status (line 0) and tournament name (line 1, scrolling).
    fn draw_header(&self, img: &mut RgbImage, data: &GolfData, xpos: i32) {
        let font = &self.body_font;
        let bl0 = top_to_baseline(0, font.ascent());
        let bl1 = top_to_baseline(7, font.ascent());

        let header_left = data.tour.display_name();
        draw_text(img, font, 1, bl0, Color::new(247, 200, 0), header_left);

        let status_color = match data.status.to_lowercase().as_str() {
            s if s.contains("progress") => Color::new(0, 255, 0),
            s if s.contains("final") || s.contains("complete") => Color::WHITE,
            _ => Color::new(180, 180, 180),
        };
        let status_x = 16;
        draw_text(img, font, status_x, bl0, status_color, &data.status);

        // Tournament name scrolls along row 1 if long.
        let total = (data.event_name.chars().count() as i32) * 4 + (PANEL_W as i32);
        let scroll_x = -(xpos % total.max(1));
        draw_text(img, font, scroll_x, bl1, Color::WHITE, &data.event_name);
    }

    /// Top 5 of the leaderboard, one row per line.
    fn draw_leaderboard(&self, img: &mut RgbImage, data: &GolfData) {
        let font = &self.body_font;
        let line_h = font.height() + 1;
        for (idx, entry) in data.leaderboard.iter().take(5).enumerate() {
            let top_y = 16 + (idx as i32) * line_h.max(5);
            if top_y + font.ascent() >= PANEL_H as i32 + font.ascent() {
                break;
            }
            let bl = top_to_baseline(top_y, font.ascent());
            self.draw_entry(img, font, bl, entry);
        }
    }

    fn draw_entry(&self, img: &mut RgbImage, font: &Font, baseline: i32, entry: &LeaderboardEntry) {
        let pos_text = format!("{:>2}.", entry.position);
        let pos_x = draw_text(img, font, 0, baseline, Color::new(200, 200, 200), &pos_text);
        let _ = draw_text(img, font, pos_x + 2, baseline, Color::WHITE, &entry.player_short);
        let color = score_color(&entry.score);
        // Right-align the score in the last 12px.
        let score_x = (PANEL_W as i32) - 1 - (entry.score.chars().count() as i32) * 4;
        draw_text(img, font, score_x, baseline, color, &entry.score);
    }

    pub fn draw_offseason(&self, data: &GolfData) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let font = &self.body_font;
        let bl0 = top_to_baseline(8, font.ascent());
        let bl1 = top_to_baseline(18, font.ascent());
        draw_text(&mut img, font, 2, bl0, Color::WHITE, data.tour.display_name());
        draw_text(&mut img, font, 2, bl1, Color::WHITE, "No event");
        img
    }
}

fn top_to_baseline(top_y: i32, ascent: i32) -> i32 {
    top_y + ascent
}

fn score_color(score: &str) -> Color {
    let s = score.trim();
    if let Some(rest) = s.strip_prefix('-') {
        if rest.parse::<i32>().is_ok() {
            return Color::new(0, 255, 0);
        }
    }
    if s.starts_with('+') {
        return Color::new(255, 80, 80);
    }
    if s.eq_ignore_ascii_case("e") {
        return Color::WHITE;
    }
    Color::WHITE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::golf::GolfTour;
    use std::path::PathBuf;

    fn repo_fonts() -> GolfFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        GolfFonts { body: repo.join("04B_03B_.TTF") }
    }

    fn sample(n: usize) -> GolfData {
        GolfData {
            tour: GolfTour::Pga,
            event_name: "PGA Championship".into(),
            status: "In Progress".into(),
            leaderboard: (1..=n as u32)
                .map(|i| LeaderboardEntry {
                    position: i,
                    player_short: format!("P{i}"),
                    score: if i == 1 { "-6".into() } else if i == 2 { "+2".into() } else { "E".into() },
                })
                .collect(),
        }
    }

    #[test]
    fn frame_paints_pixels() {
        let m = GolfMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.draw_frame(&sample(5), 0);
        assert_eq!(img.dimensions(), (64, 32));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }

    #[test]
    fn offseason_renders() {
        let m = GolfMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.draw_offseason(&sample(0));
        assert!(img.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 10);
    }

    #[test]
    fn score_colors() {
        assert_eq!(score_color("-6"), Color::new(0, 255, 0));
        assert_eq!(score_color("+2"), Color::new(255, 80, 80));
        assert_eq!(score_color("E"), Color::WHITE);
        assert_eq!(score_color("e"), Color::WHITE);
    }

    #[test]
    fn only_top_5_drawn() {
        let m = GolfMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img_5 = m.draw_frame(&sample(5), 0);
        let img_50 = m.draw_frame(&sample(50), 0);
        let lit5 = img_5.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        let lit50 = img_50.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert_eq!(lit5, lit50, "leaderboard must clip to top 5");
    }
}
