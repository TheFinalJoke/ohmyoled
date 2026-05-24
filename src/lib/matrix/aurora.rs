//! Aurora renderer — single-tile geomagnetic Kp readout for the 64×32
//! panel.
//!
//! ```text
//!  ┌─────── steady state ───────┐    ┌─────── alert mode ─────────┐
//!  │      Kp                    │    │      Kp                    │
//!  │                            │    │                            │
//!  │       3                    │    │       6                    │
//!  │                            │    │                            │
//!  │  ███▒▒▒▒▒▒                 │    │  ██████▒▒▒                 │
//!  │                            │    │   AURORA LIKELY            │
//!  └────────────────────────────┘    └────────────────────────────┘
//! ```
//!
//! - **Headline digit** is the rounded Kp index (0-9), drawn big and
//!   color-coded green (0–3) / amber (4) / violet (5–6) / red (7–9) to
//!   match NOAA's storm scale.
//! - **9-block bar** at the bottom shows the same value as a horizontal
//!   gauge — each block painted in the band-color of its position so the
//!   ramp green→amber→violet→red is visible at a glance.
//! - **Alert banner** ("AURORA LIKELY") appears only when
//!   `kp >= alert_threshold`. Cyan and slightly dimmer than the headline
//!   so the digit stays dominant.
//!
//! # Config
//!
//! ```yaml
//! aurora:
//!   run: true
//!   alert_threshold: 5      # 1–9; defaults to 5 (NOAA G1 minor storm)
//! ```
//!
//! # Data source
//!
//! `AuroraCollector::from_swpc` — NOAA Space Weather Prediction Center
//! 1-minute planetary K-index feed, no auth, 5-minute refresh.

use crate::api::aurora::AuroraReading;
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use image::{Rgb, RgbImage};
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

// Storm-scale palette, lifted from NOAA's published G-scale colors:
// quiet (green) → unsettled (amber) → minor/moderate storm (violet) →
// strong/severe/extreme (red).
const QUIET: Color = Color { r: 0, g: 220, b: 60 };
const UNSETTLED: Color = Color { r: 255, g: 200, b: 0 };
const STORM: Color = Color { r: 200, g: 60, b: 255 };
const SEVERE: Color = Color { r: 255, g: 30, b: 30 };
const LABEL: Color = Color { r: 200, g: 200, b: 200 };
const ALERT_BANNER: Color = Color { r: 0, g: 220, b: 255 };
const BAR_OFF: Color = Color { r: 40, g: 40, b: 40 };

#[derive(Debug, Clone)]
pub struct AuroraFonts {
    pub label: PathBuf,
    pub big: PathBuf,
}

impl Default for AuroraFonts {
    fn default() -> Self {
        Self {
            label: "/usr/share/fonts/04B_03B_.TTF".into(),
            big: "/usr/share/fonts/04b24.otf".into(),
        }
    }
}

pub struct AuroraMatrix {
    label_font: Font,
    big_font: Font,
}

impl AuroraMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(AuroraFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(AuroraFonts::default()).await
    }

    pub fn with_fonts(paths: AuroraFonts) -> Result<Self, String> {
        Ok(Self {
            label_font: Font::load_ttf(&paths.label, 8.0)?,
            // 04b24 at 18pt yields a digit ~14 px tall — fills the middle
            // of the panel without crowding the bar or the alert banner.
            big_font: Font::load_ttf(&paths.big, 18.0)?,
        })
    }

    pub async fn with_fonts_async(paths: AuroraFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    pub fn frame(&self, data: &AuroraReading) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);

        // "Kp" label, top-left.
        let label = "Kp";
        let label_y = self.label_font.ascent();
        draw_text(&mut img, &self.label_font, 1, label_y, LABEL, label);

        // Huge centered digit.
        let digit = format!("{}", data.kp);
        let digit_w = self.big_font.text_width(&digit);
        let digit_x = ((PANEL_W as i32 - digit_w) / 2).max(0);
        // Place the digit's baseline so it sits in rows ~7..=20.
        let digit_y = 20;
        let digit_color = kp_color(data.kp);
        draw_text(&mut img, &self.big_font, digit_x, digit_y, digit_color, &digit);

        // 9-block scale bar, centered, rows 22..=24 (3 tall).
        draw_kp_bar(&mut img, data.kp, 22);

        // Alert banner — only when alerting. Renders in the bottom row.
        if data.alert {
            let banner = "AURORA LIKELY";
            let banner_w = self.label_font.text_width(banner);
            let banner_x = ((PANEL_W as i32 - banner_w) / 2).max(0);
            let banner_y = (PANEL_H as i32) - 1;
            draw_text(&mut img, &self.label_font, banner_x, banner_y, ALERT_BANNER, banner);
        }

        img
    }
}

impl Default for AuroraMatrix {
    fn default() -> Self {
        Self::new().expect("default AuroraMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for AuroraMatrix {
    type Data = AuroraReading;

    fn id(&self) -> &'static str {
        "aurora"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(10)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &AuroraReading) -> Result<(), RenderError> {
        matrix.clear();
        let img = self.frame(data);
        matrix.set_image(&img, 0, 0);
        let hold = if data.alert {
            // Lean longer on alert so the viewer notices the banner.
            Duration::from_secs(15)
        } else {
            Duration::from_secs(10)
        };
        tokio::time::sleep(hold).await;
        matrix.clear();
        Ok(())
    }
}

/// Color band for a given Kp value. Matches the NOAA G-scale convention.
pub fn kp_color(kp: u8) -> Color {
    match kp {
        0..=3 => QUIET,
        4 => UNSETTLED,
        5..=6 => STORM,
        _ => SEVERE,
    }
}

/// Draw the 9-block Kp gauge at `top_y`. The bar is 3 rows tall and
/// horizontally centered. Each block is 5 px wide + 1 px gap; lit blocks
/// take their band-color, unlit blocks render in dim grey so the scale
/// is always visible (vs. a "fill from left" with empty cells).
fn draw_kp_bar(img: &mut RgbImage, kp: u8, top_y: i32) {
    const BLOCK_W: i32 = 5;
    const GAP: i32 = 1;
    const N: u8 = 9;
    let total_w = N as i32 * BLOCK_W + (N as i32 - 1) * GAP;
    let start_x = (PANEL_W as i32 - total_w) / 2;
    let bar_h = 3;

    for i in 0..N {
        let x0 = start_x + i as i32 * (BLOCK_W + GAP);
        let color = if i < kp {
            // Lit: use the per-position color so the gauge gradient is
            // visible even when not all blocks are filled. Position 0..=2
            // is green, 3 amber, 4..=5 storm, 6+ severe — same banding as
            // the headline digit.
            kp_color(i + 1)
        } else {
            BAR_OFF
        };
        fill_rect(img, x0, top_y, BLOCK_W, bar_h, color);
    }
}

fn fill_rect(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, color: Color) {
    let rgb = Rgb([color.r, color.g, color.b]);
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < 0 || py < 0 {
                continue;
            }
            let (px, py) = (px as u32, py as u32);
            if px < img.width() && py < img.height() {
                img.put_pixel(px, py, rgb);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn repo_fonts() -> AuroraFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        AuroraFonts {
            label: repo.join("04B_03B_.TTF"),
            big: repo.join("04b24.otf"),
        }
    }

    fn sample(kp: u8, alert: bool) -> AuroraReading {
        AuroraReading {
            kp,
            kp_index: kp as f32,
            kp_text: format!("{kp}Z"),
            alert,
            sampled_at: Utc::now(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&sample(3, false));
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn alert_mode_draws_banner_pixels() {
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        let q = m.frame(&sample(3, false));
        let a = m.frame(&sample(6, true));
        // Different pixel buffers — alert mode adds the banner.
        assert_ne!(q.as_raw(), a.as_raw());
        // Cyan banner pixels (high G+B, low R) should appear in alert mode.
        let cyan = a
            .pixels()
            .filter(|p| p.0[0] < 80 && p.0[1] > 180 && p.0[2] > 200)
            .count();
        assert!(cyan > 0, "alert frame should have cyan banner pixels");
    }

    #[test]
    fn kp_color_bands() {
        assert_eq!(kp_color(0), QUIET);
        assert_eq!(kp_color(3), QUIET);
        assert_eq!(kp_color(4), UNSETTLED);
        assert_eq!(kp_color(5), STORM);
        assert_eq!(kp_color(6), STORM);
        assert_eq!(kp_color(7), SEVERE);
        assert_eq!(kp_color(9), SEVERE);
    }

    #[test]
    fn kp_bar_lit_block_count_matches_value() {
        // For each Kp value, exactly that many bar blocks should be "lit"
        // (non-BAR_OFF). Count non-grey pixels in the bar's vertical
        // strip and divide by the per-block pixel count.
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        for kp in 0..=9u8 {
            let img = m.frame(&sample(kp, false));
            let mut lit_blocks = 0u8;
            // Bar lives at rows 22..=24, 5×3 px blocks, 1 px gaps.
            // Sample the center column of each block to detect lit vs off.
            let start_x = (64 - (9 * 5 + 8)) / 2; // total_w = 53, start_x = 5
            for i in 0..9 {
                let cx = start_x + i * (5 + 1) + 2;
                let p = img.get_pixel(cx as u32, 23);
                if p.0 != [BAR_OFF.r, BAR_OFF.g, BAR_OFF.b] && p.0 != [0, 0, 0] {
                    lit_blocks += 1;
                }
            }
            assert_eq!(
                lit_blocks, kp,
                "kp={kp}: expected {kp} lit blocks, got {lit_blocks}"
            );
        }
    }

    #[test]
    fn higher_kp_uses_redder_digit() {
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        // Kp=8 should produce strongly-red digit pixels somewhere.
        let img = m.frame(&sample(8, true));
        let red_pixels = img
            .pixels()
            .filter(|p| p.0[0] > 200 && p.0[1] < 80 && p.0[2] < 80)
            .count();
        assert!(red_pixels > 5, "expected red digit pixels at Kp=8, got {red_pixels}");
    }
}
