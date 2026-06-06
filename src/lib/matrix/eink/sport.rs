//! E-paper team-sport renderer — a static scoreboard + standings table.
//!
//! Static e-paper counterpart to [`crate::matrix::sport::SportMatrix`]. The LED
//! tile scrolls names and standings and animates; the tall panel shows the
//! whole scoreboard at once: both teams (abbreviation badges + full names), a
//! big centered score/status, and a standings table. Off-season renders a card
//! rather than blanking. Win/lose color becomes the score position + a status
//! badge. Composed white-on-black; the display inverts to black ink.
//!
//! Logos: the LED tile fetches and caches raster team logos. The e-paper tile
//! keeps `frame()` pure and offline by drawing boxed team **abbreviations**
//! instead — legible at a glance and free of async logo I/O.
//!
//! # Config
//!
//! Lives under the `eink.modules.sport` array as a team-sport-tagged entry:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     sport:
//!       - sport: basketball
//!         run: true
//!         team_logo: { name: "76ers", shorthand: "PHI" }
//! ```
//!
//! Data source: ESPN (same collector as the LED tile).

use crate::api::sport::model::{GameStatus, NextGame, SportData};
use crate::matrix::eink::layout::{
    badge, badge_width, center_text, fit_text, header_band, margin, rect, right_text, scaled_px,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper sport renderer.
pub struct EinkSportFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkSportFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper team-sport scoreboard renderer.
pub struct EinkSportMatrix {
    title: Font,
    abbr: Font,
    name: Font,
    score: Font,
    status: Font,
    row: Font,
}

impl EinkSportMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkSportFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkSportFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkSportFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            abbr: Font::load_ttf(&paths.body, scaled_px(34.0, h))?,
            name: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            score: Font::load_ttf(&paths.body, scaled_px(96.0, h))?,
            status: Font::load_ttf(&paths.body, scaled_px(26.0, h))?,
            row: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkSportFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the scoreboard at `w × h`.
    pub fn frame(&self, data: &SportData, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let content_top = header_band(&mut img, &self.title, &self.row, m, data.sport.display_name(), Some(&data.record), fg);

        let standings_top = if data.standings.is_empty() { hi } else { hi - hi * 40 / 100 };

        match &data.next_game {
            Some(game) => self.draw_scoreboard(&mut img, game, content_top, standings_top, fg),
            None => {
                let label = if data.standings.is_empty() {
                    format!("{} OFF-SEASON", data.sport.display_name())
                } else {
                    "NO UPCOMING GAME".to_string()
                };
                // The abbreviation font is large but still fits these labels.
                let bx = cx - badge_width(&self.abbr, &label) / 2;
                let by = (content_top + standings_top) / 2 - self.abbr.height() / 2;
                badge(&mut img, &self.abbr, bx, by, &label, fg, true);
            }
        }

        if !data.standings.is_empty() {
            ohmyoled_matrix::graphics::draw_line(&mut img, m, standings_top, wi - m, standings_top, fg);
            self.draw_standings(&mut img, data, standings_top + m / 2, fg);
        }
        img
    }

    fn draw_scoreboard(&self, img: &mut RgbImage, game: &NextGame, top: i32, bottom: i32, fg: Color) {
        let wi = img.width() as i32;
        let m = margin(img.width());
        let band_h = bottom - top;
        let home_cx = wi / 5;
        let away_cx = wi * 4 / 5;
        let cx = wi / 2;

        // Team abbreviation badges + names.
        let box_w = (wi / 6).min(band_h / 3);
        let box_top = top + m;
        for (tcx, side) in [(home_cx, &game.home), (away_cx, &game.away)] {
            rect(img, tcx - box_w / 2, box_top, box_w, box_w, fg);
            center_text(img, &self.abbr, tcx, box_top + box_w / 2 + self.abbr.ascent() / 2, fg, &side.abbreviation);
            let name = fit_text(&self.name, &side.name.to_uppercase(), wi / 3 - m);
            center_text(img, &self.name, tcx, box_top + box_w + self.name.ascent() + m / 2, fg, &name);
        }

        // Center: status badge + score (or date/time for a scheduled game).
        let (status_text, filled) = match game.status {
            GameStatus::Final => ("FINAL", true),
            GameStatus::InProgress => ("LIVE", true),
            GameStatus::Postponed => ("PPD", true),
            GameStatus::Scheduled => ("SCHEDULED", false),
        };
        let sb_x = cx - badge_width(&self.status, status_text) / 2;
        badge(img, &self.status, sb_x, box_top, status_text, fg, filled);

        let mid_base = top + band_h / 2 + self.score.ascent() / 2;
        match game.status {
            GameStatus::Final | GameStatus::InProgress => {
                let hs = game.home.score.unwrap_or(0);
                let as_ = game.away.score.unwrap_or(0);
                center_text(img, &self.score, cx, mid_base, fg, &format!("{hs}-{as_}"));
            }
            GameStatus::Scheduled | GameStatus::Postponed => {
                let date = game.start.format("%a %b %-d").to_string();
                let time = game.start.format("%-I:%M %p").to_string();
                center_text(img, &self.status, cx, mid_base - self.status.height(), fg, &date);
                center_text(img, &self.status, cx, mid_base, fg, &time);
            }
        }
    }

    fn draw_standings(&self, img: &mut RgbImage, data: &SportData, top: i32, fg: Color) {
        let wi = img.width() as i32;
        let hi = img.height() as i32;
        let m = margin(img.width());
        let pos_col = wi / 14;
        let name_x = m + pos_col + m;
        let row_h = self.row.height() + m / 3;
        let rows = (((hi - m) - top) / row_h).clamp(1, 10) as usize;
        for (i, e) in data.standings.iter().take(rows).enumerate() {
            let base = top + row_h * i as i32 + self.row.ascent();
            let ours = e.team_name.eq_ignore_ascii_case(&data.team_name);
            if ours {
                badge(img, &self.row, m, top + row_h * i as i32, &format!("{}", e.position), fg, true);
            } else {
                right_text(img, &self.row, m + pos_col, base, fg, &format!("{}", e.position));
            }
            let name = fit_text(&self.row, &e.team_name, wi - name_x - m);
            draw_text(img, &self.row, name_x, base, fg, &name);
        }
    }
}

#[async_trait]
impl EinkRenderer for EinkSportMatrix {
    type Data = SportData;

    fn id(&self) -> &'static str {
        "sport"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &SportData) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sport::model::{HomeOrAway, SportApiSource, SportKind, StandingsEntry, TeamSide};
    use chrono::Local;

    fn repo_fonts() -> EinkSportFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkSportFonts { body: base.join("04B_03B_.TTF") }
    }

    fn side(name: &str, abbr: &str, score: Option<i32>) -> TeamSide {
        TeamSide { name: name.into(), abbreviation: abbr.into(), logo_url: None, score }
    }

    fn standings() -> Vec<StandingsEntry> {
        vec![
            StandingsEntry { position: 1, team_name: "Celtics".into() },
            StandingsEntry { position: 2, team_name: "76ers".into() },
            StandingsEntry { position: 3, team_name: "Knicks".into() },
        ]
    }

    fn live() -> SportData {
        SportData {
            api: SportApiSource::Espn,
            sport: SportKind::Basketball,
            team_name: "76ers".into(),
            record: "41-21".into(),
            next_game: Some(NextGame {
                start: Local::now(),
                status: GameStatus::InProgress,
                home: side("76ers", "PHI", Some(88)),
                away: side("Celtics", "BOS", Some(81)),
                our_side: HomeOrAway::Home,
            }),
            standings: standings(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkSportMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let img = r.frame(&live(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated scoreboard, got {lit} lit px");
    }

    #[test]
    fn offseason_renders() {
        let r = EinkSportMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let off = SportData {
            api: SportApiSource::Espn,
            sport: SportKind::Basketball,
            team_name: "76ers".into(),
            record: "—".into(),
            next_game: None,
            standings: vec![],
        };
        assert!(off.is_offseason());
        let img = r.frame(&off, 800, 480);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "off-season must still render a card");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkSportMatrix::with_fonts(repo_fonts(), (400, 300)).unwrap();
        let img = r.frame(&live(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
