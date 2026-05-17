//! Formula 1 renderer — next race header + driver standings podium + scrolling tail.

use crate::api::f1::F1Data;
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

const GOLD: Color = Color { r: 230, g: 170, b: 0 };
const SILVER: Color = Color { r: 192, g: 192, b: 192 };
const BRONZE: Color = Color { r: 192, g: 102, b: 0 };
const F1_RED: Color = Color { r: 225, g: 6, b: 0 };

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
        Duration::from_secs(35)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &F1Data) -> Result<(), RenderError> {
        matrix.clear();
        if data.is_offseason() {
            let img = self.draw_offseason();
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

impl F1Matrix {
    /// One composed frame. Public for examples and tests.
    pub fn draw_frame(&self, data: &F1Data, xpos: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        self.draw_header(&mut img, data, xpos);
        self.draw_podium(&mut img, data);
        self.draw_scroll_tail(&mut img, data, xpos);
        img
    }

    /// Top band: "F1" badge + scrolling next-race name (rows 0..8).
    fn draw_header(&self, img: &mut RgbImage, data: &F1Data, xpos: i32) {
        let font = &self.body_font;
        let bl = top_to_baseline(0, font.ascent());
        draw_text(img, font, 1, bl, F1_RED, "F1");

        let text = match &data.next_race {
            Some(nr) => format!("R{} {}", nr.round, nr.name),
            None => "Standings".to_string(),
        };
        let total = (text.chars().count() as i32) * 4 + 64;
        let scroll_x = 12 - (xpos % total.max(1));
        draw_text(img, font, scroll_x, bl, Color::WHITE, &text);
    }

    /// Middle band: top-3 driver standings as a podium block (rows 9..24).
    fn draw_podium(&self, img: &mut RgbImage, data: &F1Data) {
        let font = &self.body_font;
        for (idx, d) in data.standings.iter().take(3).enumerate() {
            let color = match idx {
                0 => GOLD,
                1 => SILVER,
                _ => BRONZE,
            };
            let top_y = 9 + (idx as i32) * 6;
            let bl = top_to_baseline(top_y, font.ascent());
            let line = format!("{}. {} {:>3}", d.position, d.code, d.points.round() as i32);
            draw_text(img, font, 0, bl, color, &line);
        }
    }

    /// Bottom band: scrolling positions 4..10 + race date (rows 25..32).
    fn draw_scroll_tail(&self, img: &mut RgbImage, data: &F1Data, xpos: i32) {
        let font = &self.body_font;
        let bl = top_to_baseline(25, font.ascent());

        let mut parts: Vec<String> = data
            .standings
            .iter()
            .skip(3)
            .take(7)
            .map(|d| format!("{}.{} {}", d.position, d.code, d.points.round() as i32))
            .collect();
        if let Some(nr) = &data.next_race {
            parts.push(format!("@ {}", nr.start.format("%m-%d %H:%M")));
        }
        let text = parts.join("  ");
        if text.is_empty() {
            return;
        }
        let total = (text.chars().count() as i32) * 4 + (PANEL_W as i32);
        let scroll_x = -(xpos % total.max(1));
        draw_text(img, font, scroll_x, bl, Color::new(160, 160, 160), &text);
    }

    pub fn draw_offseason(&self) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let font = &self.body_font;
        let bl0 = top_to_baseline(8, font.ascent());
        let bl1 = top_to_baseline(18, font.ascent());
        draw_text(&mut img, font, 2, bl0, F1_RED, "F1");
        draw_text(&mut img, font, 2, bl1, Color::WHITE, "Offseason");
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
    fn offseason_render() {
        let m = F1Matrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.draw_offseason();
        assert!(img.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 20);
    }

    #[test]
    fn only_top_3_in_podium() {
        let m = F1Matrix::with_fonts(repo_fonts()).expect("fonts");
        let img_3 = m.draw_frame(&sample(true, 3), 0);
        let img_20 = m.draw_frame(&sample(true, 20), 0);
        // The podium area itself shouldn't grow past 3 rows even with 20 drivers.
        let count_podium = |img: &RgbImage| {
            (9u32..24)
                .flat_map(|y| (0..64).map(move |x| img.get_pixel(x, y).0))
                .filter(|p| *p != [0, 0, 0])
                .count()
        };
        // Top-3 region pixel count should match.
        assert_eq!(count_podium(&img_3), count_podium(&img_20));
    }
}
