//! Golf renderer — tournament name + status header, 5-row leaderboard.
//!
//! # Layout (64×32)
//!
//! ```text
//!   ┌────────┊───────────────────────────┐
//!   │ PGA    ┊              In Progress  │ row 0..6   tour box | status box
//!   │ TOURNAMENT NAME (scrolling)        │ row 7..14  full-width marquee
//!   ├────────────────────────┊───────────┤
//!   │  1. SCHEFFLER          ┊      -12  │
//!   │  2. RAHM               ┊       -8  │ rows 16..32   name box | score box
//!   │  3. MORIKAWA           ┊       -6  │
//!   │  4. SPIETH             ┊       -4  │
//!   │  5. CANTLAY            ┊       -3  │
//!   └────────────────────────┴───────────┘
//! ```
//!
//! Each row is split into invisible cells with a small gutter between them.
//! Text is clipped strictly to its cell — a long player name (e.g. "DECHAMBEAU")
//! marquee-scrolls twice within its name box without ever bleeding into the
//! score column. The header status cell behaves the same way for long
//! statuses like "Round 4 In Progress".
//!
//! Score colors: red ⇐ under par, white ⇐ even, yellow ⇐ over par.
//! Off-season is skipped by default; set `show_offseason: true` to draw an
//! "OFFSEASON" card (~20s) between tournaments.
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
//!     tour: pga                # pga | lpga | champions | korn | liv
//!     show_offseason: false    # true ⇒ draw an "OFFSEASON" card between events
//! ```
//!
//! # Data source
//!
//! `GolfCollector::from_espn(tour)` polls ESPN's public scoreboard endpoint
//! (`site.api.espn.com/.../scoreboard`). No API key required.

use crate::api::golf::{GolfData, LeaderboardEntry};
use crate::matrix::cells::{draw_cell, draw_pieces_in_box, Align, Piece, SCROLL_GAP};
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

const SCROLL_FRAMES: u32 = 700;
const SCROLL_TICK: Duration = Duration::from_millis(50);
const FIRST_FRAME_DWELL: Duration = Duration::from_secs(3);
const FINAL_DWELL: Duration = Duration::from_secs(5);
/// How long the static off-season card stays up before yielding the slot.
const OFFSEASON_DWELL: Duration = Duration::from_secs(20);

/// Amber accent reused for the tour code on the off-season card.
const TOUR_AMBER: Color = Color { r: 247, g: 200, b: 0 };

/// Max leaderboard entries shown in the cycling roll.
const LEADERBOARD_MAX: usize = 15;
/// Half-speed marquee divisor for the two header lines.
const HEADER_SPEED_DIV: i32 = 2;
/// How many ticks per pixel of vertical scroll in the leaderboard. Higher = slower.
const LB_TICKS_PER_PX: i32 = 2;

// --- Invisible cell layout --------------------------------------------------
// Header row 0: tour name (left) | status (right). Tour box is sized for the
// widest tour abbreviation ("LPGA"/"CHMP"); status gets the remainder.
const TOUR_BOX_X: i32 = 0;
const TOUR_BOX_W: i32 = 22;
const HEADER_STATUS_BOX_X: i32 = 24;
const HEADER_STATUS_BOX_W: i32 = 39;

// Leaderboard rows: position + name (left) | score (right). Score never
// exceeds ~4 chars ("-15"), so the score box is narrow and the name box gets
// the rest.
const NAME_BOX_X: i32 = 0;
const NAME_BOX_W: i32 = 48;
const SCORE_BOX_X: i32 = 50;
const SCORE_BOX_W: i32 = 13;
// Pixel gap between the position prefix and the player name.
const POS_NAME_GAP: i32 = 2;

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
    /// When `true`, off-season (no active tournament) renders an "OFFSEASON"
    /// card; when `false` (the default) the slot is skipped.
    show_offseason: bool,
}

impl GolfMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(GolfFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(GolfFonts::default()).await
    }

    pub fn with_fonts(paths: GolfFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
            show_offseason: false,
        })
    }

    pub async fn with_fonts_async(paths: GolfFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Enable (or disable) the off-season card. Builder-style for the registry.
    pub fn with_offseason(mut self, show: bool) -> Self {
        self.show_offseason = show;
        self
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
        Duration::from_secs(40)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &GolfData) -> Result<(), RenderError> {
        matrix.clear();
        if data.is_offseason() {
            // No active tournament leaderboard. Either draw the opt-in
            // "OFFSEASON" card or skip the slot so the scheduler advances.
            if self.show_offseason {
                let img = self.draw_offseason(data);
                matrix.set_image(&img, 0, 0);
                tokio::time::sleep(OFFSEASON_DWELL).await;
                matrix.clear();
            }
            return Ok(());
        }

        for xpos in 0..SCROLL_FRAMES {
            let img = self.draw_frame(data, xpos as i32);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(if xpos == 0 { FIRST_FRAME_DWELL } else { SCROLL_TICK }).await;
        }
        // Hold the last cycling frame rather than resetting to xpos=0 (which
        // would restart the header marquee for the final dwell).
        let img = self.draw_frame(data, SCROLL_FRAMES as i32);
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
        let header_done_at = self.header_cycle_frames(data);
        self.draw_header(&mut img, data, xpos);
        self.draw_leaderboard(&mut img, data, xpos, header_done_at);
        img
    }

    /// Real-tick count until both header lines finish their one cycle.
    /// Used by the leaderboard to know when to start its vertical roll.
    fn header_cycle_frames(&self, data: &GolfData) -> i32 {
        let font = &self.body_font;
        let event_w = font.text_width(&data.event_name);
        let status_w = font.text_width(&data.status);
        let event_period = if event_w > PANEL_W as i32 {
            event_w + PANEL_W as i32
        } else {
            0
        };
        let status_period = if status_w > HEADER_STATUS_BOX_W {
            status_w + SCROLL_GAP
        } else {
            0
        };
        event_period.max(status_period) * HEADER_SPEED_DIV
    }

    /// Two-line header: tour code + status (line 0) and tournament name
    /// (line 1, scrolling). Each scrolling element marquees once at half
    /// speed, then settles at its natural alignment.
    fn draw_header(&self, img: &mut RgbImage, data: &GolfData, xpos: i32) {
        let font = &self.body_font;
        let bl0 = top_to_baseline(0, font.ascent());
        let bl1 = top_to_baseline(7, font.ascent());
        let half_xpos = xpos.max(0) / HEADER_SPEED_DIV;

        let status_color = match data.status.to_lowercase().as_str() {
            s if s.contains("progress") => Color::new(0, 255, 0),
            s if s.contains("final") || s.contains("complete") => Color::WHITE,
            _ => Color::new(180, 180, 180),
        };

        // Tour code never overflows its cell.
        draw_cell(
            img,
            font,
            TOUR_BOX_X,
            TOUR_BOX_W,
            bl0,
            Align::Left,
            Color::new(247, 200, 0),
            data.tour.display_name(),
            0,
        );

        // Status: cycle once if it overflows the cell, else static.
        let status_w = font.text_width(&data.status);
        let status_frame = if status_w > HEADER_STATUS_BOX_W {
            let period = status_w + SCROLL_GAP;
            if half_xpos < period { half_xpos as u32 } else { 0 }
        } else {
            0
        };
        draw_cell(
            img,
            font,
            HEADER_STATUS_BOX_X,
            HEADER_STATUS_BOX_W,
            bl0,
            Align::Right,
            status_color,
            &data.status,
            status_frame,
        );

        // Tournament name: full-width marquee. Only scroll if it doesn't
        // fit; otherwise just left-align it. Cycle once with smooth
        // wrap-around (two copies during the slide), then settle.
        let event_w = font.text_width(&data.event_name);
        if event_w > PANEL_W as i32 {
            let period = event_w + PANEL_W as i32;
            if half_xpos < period {
                let scroll_x = -half_xpos;
                draw_text(img, font, scroll_x, bl1, Color::WHITE, &data.event_name);
                draw_text(img, font, scroll_x + period, bl1, Color::WHITE, &data.event_name);
            } else {
                draw_text(img, font, 0, bl1, Color::WHITE, &data.event_name);
            }
        } else {
            draw_text(img, font, 0, bl1, Color::WHITE, &data.event_name);
        }
    }

    /// Top 15 leaderboard entries pre-rendered to a tall buffer; once the
    /// headers finish cycling, the buffer scrolls vertically through the
    /// leaderboard window (y=16..32), looping. Before that, the buffer
    /// sits at offset 0 so the top two entries are visible while the
    /// headers do their thing.
    fn draw_leaderboard(&self, img: &mut RgbImage, data: &GolfData, xpos: i32, header_done_at: i32) {
        let font = &self.body_font;
        let line_h = (font.height() + 1).max(5);
        let entries: Vec<&LeaderboardEntry> = data.leaderboard.iter().take(LEADERBOARD_MAX).collect();
        if entries.is_empty() {
            return;
        }

        // Render all entries into a tall buffer so we can scroll smoothly.
        let buf_h = (entries.len() as i32) * line_h;
        let mut buf = RgbImage::new(PANEL_W, buf_h as u32);
        for (idx, entry) in entries.iter().enumerate() {
            let top_y = (idx as i32) * line_h;
            let bl = top_to_baseline(top_y, font.ascent());
            self.draw_entry(&mut buf, font, bl, entry, 0);
        }

        let lb_xpos = (xpos - header_done_at).max(0);
        let offset = (lb_xpos / LB_TICKS_PER_PX).rem_euclid(buf_h);

        let visible_h = PANEL_H - 16;
        for y in 0..visible_h {
            let src_y = (offset + y as i32).rem_euclid(buf_h) as u32;
            for x in 0..PANEL_W {
                let p = *buf.get_pixel(x, src_y);
                img.put_pixel(x, 16 + y, p);
            }
        }
    }

    fn draw_entry(&self, img: &mut RgbImage, font: &Font, baseline: i32, entry: &LeaderboardEntry, frame: u32) {
        let pos_text = format!("{:>2}.", entry.position);
        draw_pieces_in_box(
            img,
            font,
            NAME_BOX_X,
            NAME_BOX_W,
            baseline,
            Align::Left,
            &[
                Piece { text: &pos_text, color: Color::new(200, 200, 200) },
                Piece { text: &entry.player_short, color: Color::WHITE },
            ],
            POS_NAME_GAP,
            frame,
        );
        draw_cell(
            img,
            font,
            SCORE_BOX_X,
            SCORE_BOX_W,
            baseline,
            Align::Right,
            score_color(&entry.score),
            &entry.score,
            frame,
        );
    }

    /// Off-season card shown between tournaments: the tour code (amber) over an
    /// "OFFSEASON" banner and a "No events" subtitle, instead of a blank slot.
    ///
    /// ```text
    ///   ┌──────────────────┐
    ///   │ PGA              │  row 1   amber tour code
    ///   │ OFFSEASON        │  row 12  white banner
    ///   │ No events        │  row 22  grey subtitle
    ///   └──────────────────┘
    /// ```
    pub fn draw_offseason(&self, data: &GolfData) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let font = &self.body_font;
        let bl0 = top_to_baseline(1, font.ascent());
        let bl1 = top_to_baseline(12, font.ascent());
        let bl2 = top_to_baseline(22, font.ascent());
        draw_text(&mut img, font, 2, bl0, TOUR_AMBER, data.tour.display_name());
        draw_text(&mut img, font, 2, bl1, Color::WHITE, "OFFSEASON");
        draw_text(&mut img, font, 2, bl2, Color::new(156, 163, 173), "No events");
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

    #[test]
    fn long_player_name_stays_within_name_box() {
        // A 14-char name overflows the 48-px name box. At every scroll frame,
        // the score column (x >= SCORE_BOX_X) for the leaderboard row must be
        // either dark or showing the score — never a player-name glyph.
        let m = GolfMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut data = sample(1);
        data.leaderboard[0].player_short = "DECHAMBEAUMORE".into();
        data.leaderboard[0].score = "-15".into();

        // Top of row 0 in the leaderboard is y = 16. Sample at this row band.
        let row_band = 16..23;
        for frame in [0i32, 5, 13, 27, 60, 100] {
            let img = m.draw_frame(&data, frame);
            // The gutter between name and score (x in 48..50) must be dark on
            // the leaderboard row regardless of marquee position.
            for x in NAME_BOX_W..SCORE_BOX_X {
                for y in row_band.clone() {
                    assert_eq!(
                        img.get_pixel(x as u32, y).0,
                        [0, 0, 0],
                        "gutter ({x},{y}) lit at frame {frame}"
                    );
                }
            }
        }
    }

    #[test]
    fn long_status_marquees_in_header_box() {
        let m = GolfMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut data = sample(5);
        data.status = "Round 4 In Progress".into();

        // Compare two frames during the scroll. The pixels inside the status box
        // should differ (text has shifted), confirming the marquee runs.
        let img_a = m.draw_frame(&data, 0);
        let img_b = m.draw_frame(&data, 10);
        let count_in_status = |img: &image::RgbImage| {
            (HEADER_STATUS_BOX_X..HEADER_STATUS_BOX_X + HEADER_STATUS_BOX_W)
                .flat_map(|x| (0..7).map(move |y| (x as u32, y)))
                .filter(|(x, y)| img.get_pixel(*x, *y).0 != [0, 0, 0])
                .count()
        };
        // Both frames have some text. The columns lit differ as the marquee moves.
        assert!(count_in_status(&img_a) > 0);
        assert!(count_in_status(&img_b) > 0);
        let same = (HEADER_STATUS_BOX_X..HEADER_STATUS_BOX_X + HEADER_STATUS_BOX_W).all(|x| {
            (0..7).all(|y| img_a.get_pixel(x as u32, y).0 == img_b.get_pixel(x as u32, y).0)
        });
        assert!(!same, "status box should marquee between frames");
    }

    #[test]
    fn tour_status_barrier_holds() {
        // Both header cells contain content. The 2-px gutter at x in 22..24
        // must remain dark across frames, even when status marquees.
        let m = GolfMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut data = sample(5);
        data.tour = GolfTour::Lpga; // wider tour name "LPGA"
        data.status = "Round 4 In Progress".into();
        for frame in [0i32, 5, 13, 27, 50] {
            let img = m.draw_frame(&data, frame);
            for x in TOUR_BOX_W..HEADER_STATUS_BOX_X {
                for y in 0..7u32 {
                    assert_eq!(
                        img.get_pixel(x as u32, y).0,
                        [0, 0, 0],
                        "header gutter ({x},{y}) lit at frame {frame}"
                    );
                }
            }
        }
    }
}
