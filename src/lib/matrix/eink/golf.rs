//! E-paper golf renderer — a static leaderboard table (no scroll).
//!
//! Static e-paper counterpart to [`crate::matrix::golf::GolfMatrix`]. The LED
//! tile vertically scrolls five rows; the tall panel shows the top ~12 at once.
//! Off-season (no active tournament) renders a card rather than going blank.
//! Score sign (`-6` / `E` / `+2`) carries under/over par on the monochrome
//! panel. Composed white-on-black; the display inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules.sport` array as a `golf`-tagged entry:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     sport:
//!       - sport: golf
//!         run: true
//!         tour: pga
//! ```
//!
//! Data source: ESPN golf (same collector as the LED tile).

use crate::api::golf::{GolfData, GolfTour};
use crate::matrix::eink::layout::{
    badge, badge_width, fit_text, header_band, margin, right_text, scaled_px,
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

/// Font paths for the e-paper golf renderer.
pub struct EinkGolfFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkGolfFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper golf leaderboard renderer.
pub struct EinkGolfMatrix {
    title: Font,
    event: Font,
    row: Font,
    tour: GolfTour,
    /// When `true`, off-season renders an "OFF-SEASON" card; when `false` (the
    /// default) the tile yields its slot (no repaint).
    show_offseason: bool,
}

impl EinkGolfMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkGolfFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkGolfFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkGolfFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            event: Font::load_ttf(&paths.body, scaled_px(32.0, h))?,
            row: Font::load_ttf(&paths.body, scaled_px(24.0, h))?,
            tour: GolfTour::Pga,
            show_offseason: false,
        })
    }

    pub async fn with_fonts_async(paths: EinkGolfFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Set the tour shown in the header (defaults to PGA).
    pub fn with_tour(mut self, tour: GolfTour) -> Self {
        self.tour = tour;
        self
    }

    /// Enable (or disable) the off-season card. Builder-style for the registry.
    pub fn with_offseason(mut self, show: bool) -> Self {
        self.show_offseason = show;
        self
    }

    /// Compose the leaderboard at `w × h`.
    pub fn frame(&self, data: &GolfData, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let status = if data.status.trim().is_empty() { "—" } else { data.status.trim() };
        let content_top = header_band(&mut img, &self.title, &self.row, m, self.tour.display_name(), Some(status), fg);

        // Event name, full width (no scroll).
        let event = fit_text(&self.event, &data.event_name.to_uppercase(), wi - 2 * m);
        let event_y = content_top + self.event.ascent();
        crate::matrix::eink::layout::center_text(&mut img, &self.event, cx, event_y, fg, &event);
        let table_top = content_top + self.event.height() + m;

        if data.leaderboard.is_empty() {
            // Off-season card: a centered "OFF-SEASON" badge over a quieter
            // "NO ACTIVE TOURNAMENT" subtitle, below the tour/event header.
            let label = "OFF-SEASON";
            let bx = cx - badge_width(&self.event, label) / 2;
            let by = (table_top + hi) / 2 - self.event.height();
            badge(&mut img, &self.event, bx, by, label, fg, true);
            crate::matrix::eink::layout::center_text(
                &mut img,
                &self.row,
                cx,
                by + self.event.height() + m + self.row.ascent(),
                fg,
                "NO ACTIVE TOURNAMENT",
            );
            return img;
        }

        // Leaderboard table: position | player | score.
        let pos_col = wi / 10;
        let name_x = m + pos_col + m;
        let row_h = self.row.height() + m / 2;
        let rows = (((hi - m) - table_top) / row_h).clamp(1, 14) as usize;
        for (i, e) in data.leaderboard.iter().take(rows).enumerate() {
            let base = table_top + row_h * i as i32 + self.row.ascent();
            if e.position == 1 {
                badge(&mut img, &self.row, m, table_top + row_h * i as i32, &format!("{}", e.position), fg, true);
            } else {
                right_text(&mut img, &self.row, m + pos_col, base, fg, &format!("{}", e.position));
            }
            let name = fit_text(&self.row, &e.player_short, wi - name_x - m - self.row.text_width(&e.score) - m);
            draw_text(&mut img, &self.row, name_x, base, fg, &name);
            right_text(&mut img, &self.row, wi - m, base, fg, &e.score);
        }
        img
    }
}

#[async_trait]
impl EinkRenderer for EinkGolfMatrix {
    type Data = GolfData;

    fn id(&self) -> &'static str {
        "golf"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &GolfData) -> Result<(), RenderError> {
        // Off-season + opted out: don't repaint — yield the slot.
        if data.is_offseason() && !self.show_offseason {
            return Ok(());
        }
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::golf::LeaderboardEntry;

    fn repo_fonts() -> EinkGolfFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkGolfFonts { body: base.join("04B_03B_.TTF") }
    }

    fn entry(pos: u32, name: &str, score: &str) -> LeaderboardEntry {
        LeaderboardEntry {
            position: pos,
            player_short: name.into(),
            score: score.into(),
        }
    }

    fn live() -> GolfData {
        GolfData {
            tour: GolfTour::Pga,
            event_name: "The Masters".into(),
            status: "Round 3".into(),
            leaderboard: vec![
                entry(1, "S. Scheffler", "-12"),
                entry(2, "R. McIlroy", "-9"),
                entry(3, "J. Rahm", "-7"),
                entry(4, "C. Morikawa", "E"),
                entry(5, "T. Finau", "+3"),
            ],
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkGolfMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap().with_tour(GolfTour::Pga);
        let img = r.frame(&live(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated leaderboard, got {lit} lit px");
    }

    #[test]
    fn offseason_renders() {
        let r = EinkGolfMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let off = GolfData {
            tour: GolfTour::Pga,
            event_name: "Next: Players Championship".into(),
            status: String::new(),
            leaderboard: vec![],
        };
        assert!(off.is_offseason());
        let img = r.frame(&off, 800, 480);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "off-season must still render a card");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkGolfMatrix::with_fonts(repo_fonts(), (400, 300)).unwrap();
        let img = r.frame(&live(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
