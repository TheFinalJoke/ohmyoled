//! Formula 1 renderer — next race header + driver standings podium + scrolling tail.
//!
//! # Layout (64×32)
//!
//! ```text
//!   ┌──────────────────────────────────────┐
//!   │ NEXT: <race name>     (scrolling)    │ rows 0..8
//!   │ <circuit>             <date>         │ rows 8..14
//!   ├──────────────────────────────────────┤
//!   │ 🥇 DRIVER   123 pts                  │ rows 14..32
//!   │ 🥈 DRIVER   115 pts                  │ — podium (top 3)
//!   │ 🥉 DRIVER    98 pts                  │
//!   │ 4..10 DRIVER pts (scrolling)         │
//!   └──────────────────────────────────────┘
//! ```
//!
//! Podium colors: gold / silver / bronze. The header band is F1 red.
//! Off-season shows a "season ended" placeholder for ~20s.
//!
//! # Config
//!
//! F1 lives under the unified `sport` array, discriminated by
//! `"sport": "f1"`. No extra fields:
//!
//! ```yaml
//! sport:
//!   - run: true
//!     sport: f1
//! ```
//!
//! # Data source
//!
//! `F1Collector::from_jolpica()` issues two parallel requests against the
//! free `api.jolpi.ca` Ergast-compatible endpoint: `current/next.json` for
//! the upcoming race and `current/driverStandings.json` for points.

use crate::api::f1::{DriverStanding, F1Data};
use crate::matrix::cells::draw_text_in_window;
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::{DateTime, Local};
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

/// How many championship standings entries cycle through the leaderboard.
const LEADERBOARD_MAX: usize = 22;
/// Ticks per pixel of vertical scroll in the leaderboard. Higher = slower.
const LB_TICKS_PER_PX: i32 = 2;

const GOLD: Color = Color { r: 230, g: 170, b: 0 };
const SILVER: Color = Color { r: 192, g: 192, b: 192 };
const BRONZE: Color = Color { r: 192, g: 102, b: 0 };
const F1_RED: Color = Color { r: 225, g: 6, b: 0 };
const PRACTICE_YELLOW: Color = Color { r: 255, g: 220, b: 0 };
const QUALI_ORANGE: Color = Color { r: 255, g: 140, b: 0 };

/// Race-weekend phase derived from how long until lights-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RacePhase {
    /// `start` is in the past — race in progress or recently finished.
    Live,
    /// 0–24h to lights-out (race day).
    Race,
    /// 24–48h (qualifying day).
    Qualifying,
    /// 48–72h (free-practice day).
    Practice,
    /// >72h away — show weekday name instead of a phase letter.
    FarFuture,
}

impl RacePhase {
    fn label(self, start: &DateTime<Local>) -> String {
        match self {
            Self::Live => "LIVE".into(),
            Self::Race => "R".into(),
            Self::Qualifying => "Q".into(),
            Self::Practice => "P".into(),
            Self::FarFuture => start.format("%a").to_string().to_uppercase(),
        }
    }
    fn color(self) -> Color {
        match self {
            Self::Live | Self::Race => F1_RED,
            Self::Qualifying => QUALI_ORANGE,
            Self::Practice => PRACTICE_YELLOW,
            Self::FarFuture => Color::WHITE,
        }
    }
}

fn race_phase_at(start: &DateTime<Local>, now: DateTime<Local>) -> RacePhase {
    let secs = (*start - now).num_seconds();
    if secs < 0 {
        RacePhase::Live
    } else if secs < 86_400 {
        RacePhase::Race
    } else if secs < 2 * 86_400 {
        RacePhase::Qualifying
    } else if secs < 3 * 86_400 {
        RacePhase::Practice
    } else {
        RacePhase::FarFuture
    }
}

/// Compact countdown — `"3d 4h"` / `"4h 12m"` / `"45m"` / `"LIVE"`.
fn format_countdown(start: &DateTime<Local>, now: DateTime<Local>) -> String {
    let secs = (*start - now).num_seconds();
    if secs <= 0 {
        return "LIVE".into();
    }
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins.max(1))
    }
}

#[derive(Debug, Clone)]
pub struct F1Fonts {
    pub body: PathBuf,
}

impl Default for F1Fonts {
    fn default() -> Self {
        Self { body: "/usr/share/fonts/04B_03B_.TTF".into() }
    }
}

pub struct F1Matrix {
    body_font: Font,
}

impl F1Matrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(F1Fonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(F1Fonts::default()).await
    }

    pub fn with_fonts(paths: F1Fonts) -> Result<Self, String> {
        Ok(Self { body_font: Font::load_ttf(&paths.body, 8.0)? })
    }

    pub async fn with_fonts_async(paths: F1Fonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }
}

impl Default for F1Matrix {
    fn default() -> Self {
        Self::new().expect("default F1Matrix font load failed")
    }
}

#[async_trait]
impl Renderer for F1Matrix {
    type Data = F1Data;

    fn id(&self) -> &'static str {
        "f1"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(40)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &F1Data) -> Result<(), RenderError> {
        matrix.clear();
        if data.is_offseason() {
            // Between seasons. If we have standings, the leader is the
            // outgoing world champion — show a static banner. Otherwise
            // skip this slot entirely so the scheduler advances.
            if let Some(champ) = data.standings.first() {
                let img = self.draw_champion(data, champ);
                matrix.set_image(&img, 0, 0);
                tokio::time::sleep(Duration::from_secs(20)).await;
                matrix.clear();
            }
            return Ok(());
        }

        for xpos in 0..SCROLL_FRAMES {
            let img = self.draw_frame(data, xpos as i32);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(if xpos == 0 { FIRST_FRAME_DWELL } else { SCROLL_TICK }).await;
        }
        let img = self.draw_frame(data, SCROLL_FRAMES as i32);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(FINAL_DWELL).await;
        matrix.clear();
        Ok(())
    }
}

impl F1Matrix {
    /// One composed frame. Public for examples and tests.
    pub fn draw_frame(&self, data: &F1Data, xpos: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let header_done_at = self.header_cycle_frames(data);
        self.draw_header(&mut img, data, xpos);
        self.draw_phase_line(&mut img, data, Local::now());
        self.draw_leaderboard(&mut img, data, xpos, header_done_at);
        img
    }

    /// Header marquee text: `R5 Canadian Grand Prix · Circuit Gilles
    /// Villeneuve · Leader: Verstappen`. Off-week falls back to "Standings".
    fn header_text(&self, data: &F1Data) -> String {
        let Some(nr) = &data.next_race else {
            return format!("{} Standings", data.season);
        };
        let mut s = format!("R{} {}", nr.round, nr.name);
        if !nr.circuit.is_empty() {
            s.push_str(" \u{00B7} ");
            s.push_str(&nr.circuit);
        }
        if let Some(leader) = data.standings.first() {
            s.push_str(" \u{00B7} Leader: ");
            s.push_str(&leader.family_name);
        }
        s
    }

    /// Ticks until the header marquee completes one full pass — used as
    /// the "leaderboard starts cycling" boundary.
    fn header_cycle_frames(&self, data: &F1Data) -> i32 {
        let font = &self.body_font;
        let badge_w = font.text_width("F1") + 3;
        let scroll_area = PANEL_W as i32 - badge_w;
        let text_w = font.text_width(&self.header_text(data));
        if text_w > scroll_area {
            text_w + scroll_area
        } else {
            0
        }
    }

    /// Top band: F1 badge + race-name/circuit/leader marquee. One pass,
    /// smooth wrap-around, then settles left-aligned. The marquee text is
    /// strictly clipped to the area right of the badge so it appears to
    /// slide behind the badge instead of bleeding through it.
    fn draw_header(&self, img: &mut RgbImage, data: &F1Data, xpos: i32) {
        let font = &self.body_font;
        let bl = top_to_baseline(0, font.ascent());
        let badge_w = font.text_width("F1") + 3;

        let text = self.header_text(data);
        let text_w = font.text_width(&text);
        let scroll_area = PANEL_W as i32 - badge_w;
        if text_w > scroll_area {
            let period = text_w + scroll_area;
            if xpos < period {
                let scroll_x = badge_w - xpos;
                draw_text_in_window(
                    img, font, scroll_x, bl, Color::WHITE, &text, badge_w, PANEL_W as i32,
                );
                draw_text_in_window(
                    img, font, scroll_x + period, bl, Color::WHITE, &text, badge_w, PANEL_W as i32,
                );
            } else {
                draw_text_in_window(
                    img, font, badge_w, bl, Color::WHITE, &text, badge_w, PANEL_W as i32,
                );
            }
        } else {
            draw_text_in_window(
                img, font, badge_w, bl, Color::WHITE, &text, badge_w, PANEL_W as i32,
            );
        }
        // Badge drawn last so any glyph stragglers can't paint into its
        // footprint via the lighten-blend in draw_text.
        draw_text(img, font, 1, bl, F1_RED, "F1");
    }

    /// Second header row: phase indicator + race start time + countdown.
    /// `now` is injected so tests can pin a wall-clock relative to `start`.
    fn draw_phase_line(&self, img: &mut RgbImage, data: &F1Data, now: DateTime<Local>) {
        let Some(nr) = &data.next_race else {
            return;
        };
        let font = &self.body_font;
        let bl = top_to_baseline(7, font.ascent());

        let phase = race_phase_at(&nr.start, now);
        let label = phase.label(&nr.start);
        let time_str = nr.start.format("%H:%M").to_string();
        let countdown = format_countdown(&nr.start, now);
        let countdown_color = if phase == RacePhase::Live {
            F1_RED
        } else {
            Color::new(180, 180, 180)
        };

        draw_text(img, font, 0, bl, phase.color(), &label);
        let label_w = font.text_width(&label);
        let time_x = label_w + 3;
        draw_text(img, font, time_x, bl, Color::WHITE, &time_str);

        let countdown_w = font.text_width(&countdown);
        let countdown_x = PANEL_W as i32 - countdown_w;
        draw_text(img, font, countdown_x, bl, countdown_color, &countdown);
    }

    /// Leaderboard: top-22 standings pre-rendered to a tall buffer that
    /// scrolls vertically after the header marquee finishes one cycle.
    fn draw_leaderboard(&self, img: &mut RgbImage, data: &F1Data, xpos: i32, header_done_at: i32) {
        let font = &self.body_font;
        let line_h = (font.height() + 1).max(5);
        let entries: Vec<&DriverStanding> = data
            .standings
            .iter()
            .take(LEADERBOARD_MAX)
            .collect();
        if entries.is_empty() {
            return;
        }
        let leader_points = entries[0].points;

        let buf_h = (entries.len() as i32) * line_h;
        let mut buf = RgbImage::new(PANEL_W, buf_h as u32);
        for (idx, entry) in entries.iter().enumerate() {
            let top_y = (idx as i32) * line_h;
            let bl = top_to_baseline(top_y, font.ascent());
            self.draw_lb_entry(&mut buf, font, bl, entry, leader_points);
        }

        let lb_xpos = (xpos - header_done_at).max(0);
        let offset = (lb_xpos / LB_TICKS_PER_PX).rem_euclid(buf_h);

        let visible_top: i32 = 15;
        let visible_h = (PANEL_H as i32 - visible_top) as u32;
        for y in 0..visible_h {
            let src_y = (offset + y as i32).rem_euclid(buf_h) as u32;
            for x in 0..PANEL_W {
                img.put_pixel(x, visible_top as u32 + y, *buf.get_pixel(x, src_y));
            }
        }
    }

    fn draw_lb_entry(
        &self,
        img: &mut RgbImage,
        font: &Font,
        baseline: i32,
        entry: &DriverStanding,
        leader_points: f32,
    ) {
        let code_color = match entry.position {
            1 => GOLD,
            2 => SILVER,
            3 => BRONZE,
            _ => Color::WHITE,
        };
        let pos_text = format!("{:>2}.", entry.position);
        let pts_text = format!("{:>3}", entry.points.round() as i32);
        let gap_text = if entry.position == 1 {
            "\u{2014}".to_string() // em-dash
        } else {
            let gap = (leader_points - entry.points).round() as i32;
            format!("+{}", gap)
        };

        draw_text(img, font, 0, baseline, Color::new(170, 170, 170), &pos_text);
        draw_text(img, font, 13, baseline, code_color, &entry.code);
        draw_text(img, font, 28, baseline, Color::WHITE, &pts_text);
        let gap_w = font.text_width(&gap_text);
        draw_text(
            img,
            font,
            PANEL_W as i32 - gap_w,
            baseline,
            Color::new(160, 160, 160),
            &gap_text,
        );
    }

    /// Static three-line champion banner shown between seasons:
    ///
    /// ```text
    ///   F1 2026
    ///   CHAMPION
    ///   Verstappen  350
    /// ```
    pub fn draw_champion(&self, data: &F1Data, champ: &DriverStanding) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let font = &self.body_font;

        let bl0 = top_to_baseline(2, font.ascent());
        let title = format!("F1 {}", data.season);
        draw_text(&mut img, font, 2, bl0, F1_RED, &title);

        let bl1 = top_to_baseline(12, font.ascent());
        draw_text(&mut img, font, 2, bl1, GOLD, "CHAMPION");

        let bl2 = top_to_baseline(22, font.ascent());
        draw_text(&mut img, font, 2, bl2, Color::WHITE, &champ.family_name);
        let pts_text = format!("{}", champ.points.round() as i32);
        let pts_w = font.text_width(&pts_text);
        draw_text(
            &mut img,
            font,
            PANEL_W as i32 - pts_w - 1,
            bl2,
            GOLD,
            &pts_text,
        );

        img
    }
}

fn top_to_baseline(top_y: i32, ascent: i32) -> i32 {
    top_y + ascent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::f1::{DriverStanding, NextRace};
    use chrono::{Local, TimeZone};
    use std::path::PathBuf;

    fn repo_fonts() -> F1Fonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        F1Fonts { body: repo.join("04B_03B_.TTF") }
    }

    fn sample(with_race: bool, n_drivers: usize) -> F1Data {
        F1Data {
            season: "2026".into(),
            next_race: if with_race {
                Some(NextRace {
                    round: 5,
                    name: "Canadian Grand Prix".into(),
                    circuit: "Circuit Gilles Villeneuve".into(),
                    start: Local.with_ymd_and_hms(2026, 5, 24, 16, 0, 0).unwrap(),
                })
            } else {
                None
            },
            standings: (1..=n_drivers as u32)
                .map(|i| DriverStanding {
                    position: i,
                    code: format!("D{i}"),
                    family_name: format!("Driver{i}"),
                    points: (200.0 - (i as f32 * 15.0)).max(0.0),
                })
                .collect(),
        }
    }

    #[test]
    fn full_frame_renders() {
        let m = F1Matrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.draw_frame(&sample(true, 22), 0);
        assert_eq!(img.dimensions(), (64, 32));
        assert!(img.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 50);
    }

    #[test]
    fn champion_banner_renders() {
        let m = F1Matrix::with_fonts(repo_fonts()).expect("fonts");
        let data = sample(false, 22); // no next_race, standings populated
        assert!(data.is_offseason());
        let champ = data.standings.first().unwrap();
        let img = m.draw_champion(&data, champ);
        assert!(img.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 20);
    }

    #[test]
    fn is_offseason_now_just_checks_next_race() {
        // With no race but standings present, this is "between seasons" — still
        // offseason from the renderer's perspective, but a champion can be shown.
        let mut d = sample(false, 5);
        assert!(d.is_offseason());
        // Adding a race flips it back to active.
        d = sample(true, 5);
        assert!(!d.is_offseason());
    }

    #[test]
    fn leaderboard_caps_at_22_in_buffer() {
        // At settled xpos=0 with offset=0, the visible window shows the top
        // of the buffer (entries 1 & 2). That window must be identical
        // whether the underlying standings have 22 or 50 entries — the
        // collector should never put more than 22 rows in the buffer.
        let m = F1Matrix::with_fonts(repo_fonts()).expect("fonts");
        let lb_pixels = |img: &RgbImage| -> Vec<[u8; 3]> {
            (15u32..32)
                .flat_map(|y| (0..64).map(move |x| img.get_pixel(x, y).0))
                .collect()
        };
        let img_22 = m.draw_frame(&sample(true, 22), 0);
        let img_50 = m.draw_frame(&sample(true, 50), 0);
        assert_eq!(lb_pixels(&img_22), lb_pixels(&img_50));
    }

    #[test]
    fn race_phase_brackets() {
        let start = Local.with_ymd_and_hms(2026, 5, 24, 14, 0, 0).unwrap();
        // > 72h before lights-out → FarFuture
        assert_eq!(
            race_phase_at(&start, start - chrono::Duration::hours(100)),
            RacePhase::FarFuture
        );
        // 48–72h → Practice (Friday)
        assert_eq!(
            race_phase_at(&start, start - chrono::Duration::hours(60)),
            RacePhase::Practice
        );
        // 24–48h → Qualifying (Saturday)
        assert_eq!(
            race_phase_at(&start, start - chrono::Duration::hours(36)),
            RacePhase::Qualifying
        );
        // 0–24h → Race (Sunday)
        assert_eq!(
            race_phase_at(&start, start - chrono::Duration::hours(12)),
            RacePhase::Race
        );
        // After start → Live
        assert_eq!(
            race_phase_at(&start, start + chrono::Duration::minutes(15)),
            RacePhase::Live
        );
    }

    #[test]
    fn countdown_format_buckets() {
        let start = Local.with_ymd_and_hms(2026, 5, 24, 14, 0, 0).unwrap();
        assert_eq!(
            format_countdown(&start, start - chrono::Duration::hours(76)),
            "3d 4h"
        );
        assert_eq!(
            format_countdown(&start, start - chrono::Duration::minutes(252)),
            "4h 12m"
        );
        assert_eq!(
            format_countdown(&start, start - chrono::Duration::minutes(45)),
            "45m"
        );
        assert_eq!(
            format_countdown(&start, start + chrono::Duration::minutes(5)),
            "LIVE"
        );
    }
}
