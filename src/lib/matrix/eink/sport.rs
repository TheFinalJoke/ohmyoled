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

use crate::api::http::shared_client;
use crate::api::sport::model::{GameStatus, NextGame, SportData, TeamSide};
use crate::matrix::eink::layout::{
    badge, badge_width, center_text, fit_text, header_band, margin, rect, right_text, scaled_px,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::imageops::FilterType;
use image::RgbImage;
use std::collections::HashMap;
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
    /// Fetched + resized team logos, keyed by team name. Populated lazily in
    /// `render`; `frame` draws from it (abbreviation box on a miss).
    logo_cache: HashMap<String, RgbImage>,
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
            logo_cache: HashMap::new(),
        })
    }

    /// Fetch + decode + resize a team's logo into the cache (idempotent). A
    /// miss or failure leaves the renderer to draw the abbreviation box.
    async fn ensure_logo(&mut self, side: &TeamSide) {
        if self.logo_cache.contains_key(&side.name) {
            return;
        }
        let Some(url) = side.logo_url.clone() else { return };
        match fetch_logo(&url).await {
            Ok(logo) => {
                self.logo_cache.insert(side.name.clone(), logo);
            }
            Err(e) => log::warn!("eink sport: logo fetch failed for {} ({url}): {e}", side.name),
        }
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

        let standings_top = if data.standings.is_empty() { hi } else { hi - hi * 50 / 100 };

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
        // Everything hangs off the band's vertical center so the scoreboard
        // fills the space rather than hugging the top.
        let score_cy = top + band_h / 2;

        // Team crest (logo if fetched, else an abbreviation box) + name,
        // vertically centered as a unit on the score line.
        let box_w = (wi / 6).min(band_h * 42 / 100);
        let unit_h = box_w + m / 2 + self.name.height();
        let box_top = score_cy - unit_h / 2;
        for (tcx, side) in [(home_cx, &game.home), (away_cx, &game.away)] {
            self.draw_crest(img, tcx, box_top, box_w, side, fg);
            let name = fit_text(&self.name, &side.name.to_uppercase(), wi / 3 - m);
            center_text(img, &self.name, tcx, box_top + box_w + m / 2 + self.name.ascent(), fg, &name);
        }

        // Status badge above the centered score.
        let (status_text, filled) = match game.status {
            GameStatus::Final => ("FINAL", true),
            GameStatus::InProgress => ("LIVE", true),
            GameStatus::Postponed => ("PPD", true),
            GameStatus::Scheduled => ("SCHEDULED", false),
        };
        let sb_x = cx - badge_width(&self.status, status_text) / 2;
        let sb_y = score_cy - self.score.height() / 2 - self.status.height() - m / 2;
        badge(img, &self.status, sb_x, sb_y, status_text, fg, filled);

        match game.status {
            GameStatus::Final | GameStatus::InProgress => {
                let hs = game.home.score.unwrap_or(0);
                let as_ = game.away.score.unwrap_or(0);
                let s = format!("{hs}-{as_}");
                let base = score_cy - self.score.text_v_center_from_baseline(&s);
                center_text(img, &self.score, cx, base, fg, &s);
            }
            GameStatus::Scheduled | GameStatus::Postponed => {
                let date = game.start.format("%a %b %-d").to_string();
                let time = game.start.format("%-I:%M %p").to_string();
                center_text(img, &self.status, cx, score_cy, fg, &date);
                center_text(img, &self.status, cx, score_cy + self.status.height(), fg, &time);
            }
        }
    }

    /// Draw a team crest into the `box_w` square at `(cx - box_w/2, box_top)`:
    /// the fetched logo as a B/W silhouette when available, else an outlined
    /// box with the abbreviation.
    fn draw_crest(&self, img: &mut RgbImage, cx: i32, box_top: i32, box_w: i32, side: &TeamSide, fg: Color) {
        let bx = cx - box_w / 2;
        if let Some(logo) = self.logo_cache.get(&side.name) {
            draw_logo_silhouette(img, logo, bx, box_top, box_w, fg);
        } else {
            rect(img, bx, box_top, box_w, box_w, fg);
            center_text(img, &self.abbr, cx, box_top + box_w / 2 + self.abbr.ascent() / 2, fg, &side.abbreviation);
        }
    }

    /// Two-column standings (1.. on the left, the next batch on the right) so
    /// the bottom band fills the panel width.
    fn draw_standings(&self, img: &mut RgbImage, data: &SportData, top: i32, fg: Color) {
        let wi = img.width() as i32;
        let hi = img.height() as i32;
        let m = margin(img.width());
        let pos_col = wi / 22;
        let row_h = self.row.height() + m / 3;
        let per_col = (((hi - m) - top) / row_h).clamp(1, 10) as usize;
        let col_w = (wi - 2 * m) / 2;
        for (i, e) in data.standings.iter().take(per_col * 2).enumerate() {
            let (col, r) = (i / per_col, i % per_col);
            let cx0 = m + (col as i32) * col_w;
            let base = top + row_h * r as i32 + self.row.ascent();
            let name_x = cx0 + pos_col + m;
            let ours = e.team_name.eq_ignore_ascii_case(&data.team_name);
            if ours {
                badge(img, &self.row, cx0, top + row_h * r as i32, &format!("{}", e.position), fg, true);
            } else {
                right_text(img, &self.row, cx0 + pos_col, base, fg, &format!("{}", e.position));
            }
            let name = fit_text(&self.row, &e.team_name, cx0 + col_w - name_x - m);
            draw_text(img, &self.row, name_x, base, fg, &name);
        }
    }
}

/// Composite a fetched logo into the `box_w` square as a high-contrast B/W
/// silhouette: source pixels that are dark or saturated (the logo's marks)
/// become foreground ink; near-white background is left clear.
fn draw_logo_silhouette(img: &mut RgbImage, logo: &RgbImage, bx: i32, by: i32, box_w: i32, fg: Color) {
    let n = logo.width().max(1);
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    let px = image::Rgb([fg.r, fg.g, fg.b]);
    for ty in 0..box_w {
        for tx in 0..box_w {
            let sx = (tx as u32 * n / box_w as u32).min(n - 1);
            let sy = (ty as u32 * n / box_w as u32).min(logo.height() - 1);
            let p = logo.get_pixel(sx, sy).0;
            let luma = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
            let sat = *p.iter().max().unwrap() as i32 - *p.iter().min().unwrap() as i32;
            if luma < 200 || sat > 40 {
                let (gx, gy) = (bx + tx, by + ty);
                if gx >= 0 && gx < iw && gy >= 0 && gy < ih {
                    img.put_pixel(gx as u32, gy as u32, px);
                }
            }
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
        // Fetch both teams' logos (cached) before composing.
        if let Some(game) = &data.next_game {
            let (home, away) = (game.home.clone(), game.away.clone());
            self.ensure_logo(&home).await;
            self.ensure_logo(&away).await;
        }
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

/// HTTP fetch + decode + resize a logo to `LOGO_PX` square, composited onto a
/// white background (so transparency reads as background for the silhouette).
async fn fetch_logo(url: &str) -> Result<RgbImage, String> {
    const LOGO_PX: u32 = 120;
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
    tokio::task::spawn_blocking(move || {
        let dyn_img = image::load_from_memory(&owned).map_err(|e| format!("decode: {e}"))?;
        let rgba = dyn_img.resize_exact(LOGO_PX, LOGO_PX, FilterType::Lanczos3).to_rgba8();
        let mut out = RgbImage::from_pixel(LOGO_PX, LOGO_PX, image::Rgb([255, 255, 255]));
        for (x, y, p) in rgba.enumerate_pixels() {
            let a = p[3] as f32 / 255.0;
            let blend = |c: u8| (c as f32 * a + 255.0 * (1.0 - a)).round() as u8;
            out.put_pixel(x, y, image::Rgb([blend(p[0]), blend(p[1]), blend(p[2])]));
        }
        Ok(out)
    })
    .await
    .map_err(|e| format!("decode task panicked: {e}"))?
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
