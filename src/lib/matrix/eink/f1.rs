//! E-paper Formula 1 renderer — next race + countdown, or an off-season
//! champion card, plus the driver standings table.
//!
//! Static e-paper counterpart to [`crate::matrix::f1::F1Matrix`]. The countdown
//! is recomputed at render time. Off-season (no next race) shows the reigning
//! champion + final standings rather than blanking. Podium gold/silver/bronze
//! becomes fill-state: P1 filled badge, P2–3 outline, rest plain. Composed
//! white-on-black; the display inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules.sport` array as an `f1`-tagged entry:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     sport:
//!       - sport: f1
//!         run: true
//! ```
//!
//! Data source: Jolpica/Ergast (same collector as the LED tile).

use crate::api::f1::{F1Data, NextRace};
use crate::matrix::eink::layout::{
    badge, badge_width, center_text, fit_text, header_band, margin, right_text, scaled_px,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Countdown text from `start` relative to `now` — coarse, like the LED tile.
fn format_countdown(start: &DateTime<Local>, now: DateTime<Local>) -> String {
    let secs = (*start - now).num_seconds();
    if secs <= 0 {
        return "LIVE".into();
    }
    let (days, hours, mins) = (secs / 86_400, (secs % 86_400) / 3_600, (secs % 3_600) / 60);
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{}m", mins.max(1))
    }
}

/// Font paths for the e-paper F1 renderer.
pub struct EinkF1Fonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkF1Fonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper Formula 1 renderer.
pub struct EinkF1Matrix {
    title: Font,
    race: Font,
    label: Font,
    countdown: Font,
    row: Font,
}

impl EinkF1Matrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkF1Fonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkF1Fonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkF1Fonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            race: Font::load_ttf(&paths.body, scaled_px(32.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            countdown: Font::load_ttf(&paths.body, scaled_px(58.0, h))?,
            row: Font::load_ttf(&paths.body, scaled_px(24.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkF1Fonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the F1 screen at `w × h`, sampling the countdown at `now`.
    pub fn frame(&self, data: &F1Data, now: DateTime<Local>, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let right = match &data.next_race {
            Some(r) => format!("ROUND {}", r.round),
            None => "OFF-SEASON".to_string(),
        };
        let content_top = header_band(&mut img, &self.title, &self.label, m, &format!("F1 {}", data.season), Some(&right), fg);

        // Standings occupy the bottom; the race/champion banner the top.
        let standings_top = if data.standings.is_empty() { hi } else { hi - hi * 46 / 100 };

        match &data.next_race {
            Some(race) => self.draw_race(&mut img, race, now, content_top, standings_top, fg),
            None => self.draw_offseason(&mut img, data, content_top, standings_top, fg),
        }

        if !data.standings.is_empty() {
            draw_line(&mut img, m, standings_top, wi - m, standings_top, fg);
            self.draw_standings(&mut img, data, standings_top + m / 2, fg);
        }
        let _ = cx;
        img
    }

    fn draw_race(&self, img: &mut RgbImage, race: &NextRace, now: DateTime<Local>, top: i32, bottom: i32, fg: Color) {
        let wi = img.width() as i32;
        let m = margin(img.width());
        let cx = wi / 2;
        let name = fit_text(&self.race, &race.name.to_uppercase(), wi - 2 * m);
        center_text(img, &self.race, cx, top + self.race.ascent() + m / 2, fg, &name);
        let circuit = fit_text(&self.label, &race.circuit, wi - 2 * m);
        center_text(img, &self.label, cx, top + self.race.height() + self.label.ascent() + m / 2, fg, &circuit);
        let line = format_countdown(&race.start, now);
        let cd_base = (top + bottom) / 2 + self.countdown.ascent() / 2;
        center_text(img, &self.countdown, cx, cd_base, fg, &line);
    }

    fn draw_offseason(&self, img: &mut RgbImage, data: &F1Data, top: i32, bottom: i32, fg: Color) {
        let wi = img.width() as i32;
        let cx = wi / 2;
        let mid = (top + bottom) / 2;
        if let Some(champ) = data.standings.first() {
            let label = format!("{} CHAMPION", data.season);
            let bx = cx - badge_width(&self.label, &label) / 2;
            badge(img, &self.label, bx, top + (mid - top) / 2, &label, fg, true);
            center_text(img, &self.race, cx, mid + self.race.ascent(), fg, &format!("{} {}", champ.code, champ.family_name).to_uppercase());
        } else {
            let bx = cx - badge_width(&self.countdown, "OFF-SEASON") / 2;
            badge(img, &self.countdown, bx, mid - self.countdown.height() / 2, "OFF-SEASON", fg, true);
        }
    }

    fn draw_standings(&self, img: &mut RgbImage, data: &F1Data, top: i32, fg: Color) {
        let wi = img.width() as i32;
        let hi = img.height() as i32;
        let m = margin(img.width());
        let pos_col = wi / 12;
        let name_x = m + pos_col + m;
        let row_h = self.row.height() + m / 3;
        let rows = (((hi - m) - top) / row_h).clamp(1, 14) as usize;
        for (i, d) in data.standings.iter().take(rows).enumerate() {
            let base = top + row_h * i as i32 + self.row.ascent();
            // Podium emphasis: P1 filled, P2-3 outline, rest plain.
            match d.position {
                1 => {
                    badge(img, &self.row, m, top + row_h * i as i32, "1", fg, true);
                }
                2 | 3 => {
                    badge(img, &self.row, m, top + row_h * i as i32, &format!("{}", d.position), fg, false);
                }
                _ => right_text(img, &self.row, m + pos_col, base, fg, &format!("{}", d.position)),
            }
            let name = format!("{}  {}", d.code, d.family_name);
            let pts = format!("{:.0}", d.points);
            let name = fit_text(&self.row, &name, wi - name_x - m - self.row.text_width(&pts) - m);
            draw_text(img, &self.row, name_x, base, fg, &name);
            right_text(img, &self.row, wi - m, base, fg, &pts);
        }
    }
}

#[async_trait]
impl EinkRenderer for EinkF1Matrix {
    type Data = F1Data;

    fn id(&self) -> &'static str {
        "f1"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &F1Data) -> Result<(), RenderError> {
        let img = self.frame(data, Local::now(), display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::f1::DriverStanding;
    use chrono::Duration as ChDuration;

    fn repo_fonts() -> EinkF1Fonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkF1Fonts { body: base.join("04B_03B_.TTF") }
    }

    fn driver(pos: u32, code: &str, name: &str, pts: f32) -> DriverStanding {
        DriverStanding { position: pos, code: code.into(), family_name: name.into(), points: pts }
    }

    fn standings() -> Vec<DriverStanding> {
        vec![
            driver(1, "VER", "Verstappen", 314.0),
            driver(2, "NOR", "Norris", 270.0),
            driver(3, "LEC", "Leclerc", 226.0),
            driver(4, "PIA", "Piastri", 197.0),
        ]
    }

    fn in_season() -> F1Data {
        F1Data {
            season: "2025".into(),
            next_race: Some(NextRace {
                round: 7,
                name: "Monaco Grand Prix".into(),
                circuit: "Circuit de Monaco".into(),
                start: Local::now() + ChDuration::days(3),
            }),
            standings: standings(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkF1Matrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let img = r.frame(&in_season(), Local::now(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated screen, got {lit} lit px");
    }

    #[test]
    fn offseason_renders_champion() {
        let r = EinkF1Matrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let off = F1Data { season: "2025".into(), next_race: None, standings: standings() };
        assert!(off.is_offseason());
        let img = r.frame(&off, Local::now(), 800, 480);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "off-season must render the champion card");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkF1Matrix::with_fonts(repo_fonts(), (400, 300)).unwrap();
        let img = r.frame(&in_season(), Local::now(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
