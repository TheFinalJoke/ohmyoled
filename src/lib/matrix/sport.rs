//! Sport renderer — pure-Rust port of `src/python/ohmyoled/matrix/sport/sportmatrix.py`.
//!
//! Three horizontal bands on the 64×32 panel:
//!
//! ```text
//!   ┌──────────────────────────────────────────┐
//!   │ scrolling-home-name     VS    scrolling-away-name │ (top:    64×8,  rows 0..8)
//!   ├──────────────────────────────────────────┤
//!   │ ▢home-logo▢   status / score    ▢away-logo▢       │ (middle: 64×16, rows 9..25)
//!   ├──────────────────────────────────────────┤
//!   │ scrolling 1. Team  2. Team  3. Team ...           │ (bottom: 64×8, rows 26..32)
//!   └──────────────────────────────────────────┘
//! ```
//!
//! Logos are fetched on first render via `reqwest`, decoded with the `image`
//! crate, resized to 16×16 with `Lanczos3`, and cached in-memory for the life
//! of the renderer.

use crate::api::http::shared_client;
use crate::api::sport::model::{
    GameStatus, HomeOrAway, NextGame, SportData, SportKind, TeamSide,
};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::Local;
use image::imageops::FilterType;
use image::{Rgb, RgbImage};
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;
const LOGO_SIZE: u32 = 16;

const SCROLL_FRAMES: u32 = 600;
const SCROLL_TICK: Duration = Duration::from_millis(50);
const FIRST_FRAME_DWELL: Duration = Duration::from_secs(3);
const FINAL_DWELL: Duration = Duration::from_secs(15);

const STANDINGS_COLOR: Color = Color { r: 156, g: 163, b: 173 };

#[derive(Debug, Clone)]
pub struct SportFonts {
    pub body: PathBuf,
    pub big: PathBuf,
}

impl Default for SportFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
            big: "/usr/share/fonts/04b24.otf".into(),
        }
    }
}

pub struct SportMatrix {
    body_font: Font,
    big_font: Font,
    logo_cache: HashMap<String, RgbImage>,
}

impl SportMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(SportFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(SportFonts::default()).await
    }

    pub fn with_fonts(paths: SportFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
            big_font: Font::load_ttf(&paths.big, 14.0)?,
            logo_cache: HashMap::new(),
        })
    }

    pub async fn with_fonts_async(paths: SportFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Fetch + decode + resize + cache a team's logo. Idempotent.
    ///
    /// Failures log a warning and leave the cache miss in place — the renderer
    /// just draws an empty slot for that team's logo.
    async fn ensure_logo(&mut self, side: &TeamSide) {
        if self.logo_cache.contains_key(&side.name) {
            return;
        }
        let Some(url) = &side.logo_url else { return };
        match fetch_logo(url).await {
            Ok(logo) => {
                self.logo_cache.insert(side.name.clone(), logo);
            }
            Err(e) => log::warn!("sport: logo fetch failed for {} ({url}): {e}", side.name),
        }
    }
}

impl Default for SportMatrix {
    fn default() -> Self {
        Self::new().expect("default SportMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for SportMatrix {
    type Data = SportData;

    fn id(&self) -> &'static str {
        "sport"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(45)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &SportData) -> Result<(), RenderError> {
        matrix.clear();

        if data.is_offseason() {
            let img = self.draw_offseason(data.sport);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(Duration::from_secs(30)).await;
            matrix.clear();
            return Ok(());
        }

        if let Some(ng) = &data.next_game {
            self.ensure_logo(&ng.home).await;
            self.ensure_logo(&ng.away).await;
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

// ---------------------------------------------------------------------------
// Drawing — pure functions on a fresh RgbImage so frames are unit-testable.
// ---------------------------------------------------------------------------

impl SportMatrix {
    /// Render one composed frame at the given scroll offset.
    pub fn draw_frame(&self, data: &SportData, xpos: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        if let Some(ng) = &data.next_game {
            self.draw_top(&mut img, ng, xpos);
            self.draw_middle(&mut img, ng);
        }
        self.draw_bottom(&mut img, data, xpos);
        img
    }

    /// Render the offseason placeholder ("NBA / Offseason"). Public for
    /// examples and visual-regression checks.
    pub fn draw_offseason(&self, sport: SportKind) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let baseline_1 = top_to_baseline(2, self.big_font.ascent());
        let baseline_2 = top_to_baseline(17, self.big_font.ascent());
        draw_text(&mut img, &self.big_font, 4, baseline_1, Color::WHITE, sport.display_name());
        draw_text(&mut img, &self.big_font, 4, baseline_2, Color::WHITE, "Offseason");
        img
    }

    /// Top band: scrolling home name (left half) + "VS" + scrolling away name (right half).
    fn draw_top(&self, img: &mut RgbImage, ng: &NextGame, xpos: i32) {
        let font = &self.body_font;
        let baseline = top_to_baseline(0, font.ascent());

        let home_name = &ng.home.name;
        let away_name = &ng.away.name;

        // Names slide left at one-pixel-per-tick. The slot is 22px wide; once a
        // name fully scrolls off, reset by modulo so it loops.
        let home_x = -((xpos as i32) % (home_name.chars().count() as i32 * 4 + 22).max(1));
        let away_x = 44 - ((xpos as i32) % (away_name.chars().count() as i32 * 4 + 22).max(1));

        draw_text(img, font, home_x, baseline, Color::WHITE, home_name);
        draw_text(img, font, 28, baseline, Color::WHITE, "VS");
        draw_text(img, font, away_x, baseline, Color::WHITE, away_name);
    }

    /// Middle band: home logo (16×16 at x=0), away logo (16×16 at x=48), and
    /// status/score/date text in the gap.
    fn draw_middle(&self, img: &mut RgbImage, ng: &NextGame) {
        if let Some(logo) = self.logo_cache.get(&ng.home.name) {
            paste(img, logo, 0, 9);
        }
        if let Some(logo) = self.logo_cache.get(&ng.away.name) {
            paste(img, logo, 48, 9);
        }

        let lines = middle_text_lines(ng);
        let font = &self.body_font;
        let line_h = font.height();
        // Center the two-line block vertically in rows 9..25 (16px tall).
        let block_h = line_h * 2 + 1;
        let block_top = 9 + ((16 - block_h) / 2).max(0);
        for (i, line) in lines.iter().enumerate() {
            let y = top_to_baseline(block_top + (i as i32) * (line_h + 1), font.ascent());
            let color = middle_line_color(ng, i);
            draw_text(img, font, 17, y, color, line);
        }
    }

    /// Bottom band: scrolling "1. Team  2. Team ..." (or our team's record if
    /// standings empty).
    fn draw_bottom(&self, img: &mut RgbImage, data: &SportData, xpos: i32) {
        let font = &self.body_font;
        let baseline = top_to_baseline(25, font.ascent());

        let text = if data.standings.is_empty() {
            format!("{} ({}) {}", data.team_name, data.record, data.sport.display_name())
        } else {
            let mut parts: Vec<String> = data
                .standings
                .iter()
                .map(|e| format!("{}. {}", e.position, e.team_name))
                .collect();
            parts.push(format!("({})", data.record));
            parts.join("  ")
        };

        // Text width ≈ 4px/char for 04B_03B; loop scroll modulo total width.
        let total = (text.chars().count() as i32) * 4 + (PANEL_W as i32);
        let scroll_x = -((xpos % total.max(1)) as i32);
        draw_text(img, font, scroll_x, baseline, STANDINGS_COLOR, &text);
    }
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

fn top_to_baseline(top_y: i32, ascent: i32) -> i32 {
    top_y + ascent
}

/// Compose `src` onto `dst` at `(x, y)`, clipping to dst bounds.
fn paste(dst: &mut RgbImage, src: &RgbImage, x: i32, y: i32) {
    let dst_w = dst.width() as i32;
    let dst_h = dst.height() as i32;
    for sy in 0..src.height() {
        let dy = y + sy as i32;
        if dy < 0 || dy >= dst_h {
            continue;
        }
        for sx in 0..src.width() {
            let dx = x + sx as i32;
            if dx < 0 || dx >= dst_w {
                continue;
            }
            let p = src.get_pixel(sx, sy);
            if p.0 == [0, 0, 0] {
                continue;
            }
            dst.put_pixel(dx as u32, dy as u32, *p);
        }
    }
}

/// Two-line middle text — top row is status/date, bottom row is score or weekday.
fn middle_text_lines(ng: &NextGame) -> [String; 2] {
    match ng.status {
        GameStatus::InProgress | GameStatus::Final => {
            let (us, them) = match ng.our_side {
                HomeOrAway::Home => (&ng.home, &ng.away),
                HomeOrAway::Away => (&ng.away, &ng.home),
            };
            let status_label = match ng.status {
                GameStatus::InProgress => "Live",
                GameStatus::Final => "Final",
                _ => "",
            };
            let score = format!(
                "{}-{}",
                us.score.unwrap_or(0),
                them.score.unwrap_or(0)
            );
            [status_label.to_string(), score]
        }
        GameStatus::Postponed => ["Postp.".to_string(), String::new()],
        GameStatus::Scheduled => {
            // Show the date on top and the local weekday/time below.
            let date = ng.start.with_timezone(&Local).format("%m-%d").to_string();
            let time = ng.start.with_timezone(&Local).format("%H:%M").to_string();
            [date, time]
        }
    }
}

/// Line 0 (status/date) is white; line 1 (score) tinted green/red on win/loss.
fn middle_line_color(ng: &NextGame, line_idx: usize) -> Color {
    if line_idx == 0 {
        return Color::WHITE;
    }
    if !matches!(ng.status, GameStatus::Final) {
        return Color::WHITE;
    }
    let (us, them) = match ng.our_side {
        HomeOrAway::Home => (&ng.home, &ng.away),
        HomeOrAway::Away => (&ng.away, &ng.home),
    };
    match (us.score, them.score) {
        (Some(u), Some(t)) if u > t => Color::new(0, 255, 0),
        (Some(u), Some(t)) if u < t => Color::new(255, 0, 0),
        _ => Color::WHITE,
    }
}

/// HTTP fetch + image decode + resize to `LOGO_SIZE × LOGO_SIZE`.
async fn fetch_logo(url: &str) -> Result<RgbImage, String> {
    let bytes = shared_client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("http: {e}"))?
        .error_for_status()
        .map_err(|e| format!("http status: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("body: {e}"))?;
    let owned = bytes.to_vec();
    tokio::task::spawn_blocking(move || decode_and_resize(&owned))
        .await
        .map_err(|e| format!("decode task panicked: {e}"))?
}

fn decode_and_resize(bytes: &[u8]) -> Result<RgbImage, String> {
    let dyn_img = image::load_from_memory(bytes).map_err(|e| format!("decode: {e}"))?;
    let resized = dyn_img.resize_exact(LOGO_SIZE, LOGO_SIZE, FilterType::Lanczos3);
    let rgba = resized.into_rgba8();

    // Composite onto black to avoid PIL-style alpha → grey edges.
    let mut out = RgbImage::new(LOGO_SIZE, LOGO_SIZE);
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as f32 / 255.0;
        let r = (p[0] as f32 * a).round() as u8;
        let g = (p[1] as f32 * a).round() as u8;
        let b = (p[2] as f32 * a).round() as u8;
        out.put_pixel(x, y, Rgb([r, g, b]));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sport::model::{SportApiSource, StandingsEntry};
    use chrono::TimeZone;
    use std::path::PathBuf;

    fn repo_fonts() -> SportFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        SportFonts {
            body: repo.join("04B_03B_.TTF"),
            big: repo.join("04b24.otf"),
        }
    }

    fn make_data(with_game: bool, with_standings: bool, status: GameStatus) -> SportData {
        let next_game = if with_game {
            Some(NextGame {
                start: Local.with_ymd_and_hms(2024, 6, 15, 19, 30, 0).unwrap(),
                status,
                home: TeamSide {
                    name: "Boston Celtics".into(),
                    abbreviation: "BOS".into(),
                    logo_url: None,
                    score: Some(108),
                },
                away: TeamSide {
                    name: "Philadelphia 76ers".into(),
                    abbreviation: "PHI".into(),
                    logo_url: None,
                    score: Some(100),
                },
                our_side: HomeOrAway::Home,
            })
        } else {
            None
        };
        let standings = if with_standings {
            (1..=5)
                .map(|i| StandingsEntry { position: i, team_name: format!("Team{i}") })
                .collect()
        } else {
            vec![]
        };
        SportData {
            api: SportApiSource::Espn,
            sport: SportKind::Basketball,
            team_name: "Boston Celtics".into(),
            record: "56-26".into(),
            next_game,
            standings,
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = SportMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img = m.draw_frame(&make_data(true, true, GameStatus::Final), 0);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }

    #[test]
    fn offseason_renders_two_line_placeholder() {
        let m = SportMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let data = make_data(false, false, GameStatus::Scheduled);
        assert!(data.is_offseason());
        let img = m.draw_offseason(data.sport);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 20, "offseason frame should have visible text, got {lit}");
    }

    #[test]
    fn middle_lines_scheduled_vs_final() {
        let s = make_data(true, true, GameStatus::Scheduled);
        let l = middle_text_lines(s.next_game.as_ref().unwrap());
        assert!(l[0].contains('-')); // date "06-15"
        assert!(l[1].contains(':')); // time "19:30"

        let f = make_data(true, true, GameStatus::Final);
        let l = middle_text_lines(f.next_game.as_ref().unwrap());
        assert_eq!(l[0], "Final");
        assert_eq!(l[1], "108-100");
    }

    #[test]
    fn paste_respects_bounds() {
        let mut dst = RgbImage::new(64, 32);
        let mut src = RgbImage::new(20, 20);
        for x in 0..20 {
            for y in 0..20 {
                src.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        // Paste with negative offset and extending past the right edge.
        paste(&mut dst, &src, -5, 25);
        // Pixel at (0, 25) should be lit (src x=5, y=0 maps to dst x=0, y=25).
        assert_ne!(dst.get_pixel(0, 25).0, [0, 0, 0]);
        // Pixel at (14, 31) should be lit. (15..20 are off the right/bottom.)
        assert_ne!(dst.get_pixel(14, 31).0, [0, 0, 0]);
    }

    #[test]
    fn middle_line_color_paints_win_green_loss_red() {
        let mut ng = make_data(true, true, GameStatus::Final).next_game.unwrap();
        // We're home, scored 108, opp scored 100 — line 1 should be green.
        assert_eq!(middle_line_color(&ng, 1), Color::new(0, 255, 0));

        // Flip the score → red.
        ng.home.score = Some(90);
        assert_eq!(middle_line_color(&ng, 1), Color::new(255, 0, 0));

        // In progress → always white.
        ng.status = GameStatus::InProgress;
        assert_eq!(middle_line_color(&ng, 1), Color::WHITE);
    }
}
