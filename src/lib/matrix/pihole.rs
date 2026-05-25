//! Pi-hole renderer — DNS block stats tile for the 64×32 panel.
//!
//! ```text
//!  ┌──────────────────────────────┐
//!  │ PIHOLE                       │   small label, emerald
//!  │                              │
//!  │       34.2%                  │   BIG percent number, color
//!  │                              │   intensity-coded
//!  │  12.3k Q   4.2k BLK          │   compact stat footer, dim
//!  └──────────────────────────────┘
//! ```
//!
//! - **Label** (top): "PIHOLE" in emerald — the visual anchor.
//! - **Percent** (centered, big): today's `% blocked` in the same
//!   emerald, brightness scaled with intensity (bright when >30%,
//!   dim when <10%).
//! - **Footer** (bottom): compact `Nk Q   Nk BLK` showing total
//!   queries and total blocks. Uses k-suffix formatting so big
//!   numbers fit the 64-px row.
//!
//! # Config
//!
//! ```yaml
//! pihole:
//!   run: true
//!   base_url: http://pi.hole
//!   token: null               # optional v5 admin token; null = unauth
//! ```
//!
//! # Data source
//!
//! `PiholeCollector::from_v5` — `<base_url>/admin/api.php?summaryRaw`
//! against a local Pi-hole v5 install. 30 s refresh.

use crate::api::pihole::PiholeSummary;
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

// Pi-hole brand-ish emerald, scaled to three intensity tiers.
const EMERALD_BRIGHT: Color = Color { r: 50, g: 220, b: 120 };
const EMERALD_MED: Color = Color { r: 30, g: 160, b: 90 };
const EMERALD_DIM: Color = Color { r: 20, g: 100, b: 60 };
const LABEL: Color = Color { r: 200, g: 200, b: 200 };
const FOOTER: Color = Color { r: 130, g: 130, b: 130 };

// Same hardcoded-y discipline as launch/hass — guarantees a visible
// gap between the big percent row and the stat footer.
const LABEL_Y: i32 = 5;
const PERCENT_Y: i32 = 21;
const FOOTER_Y: i32 = 31;

#[derive(Debug, Clone)]
pub struct PiholeFonts {
    pub body: PathBuf,
    pub big: PathBuf,
}

impl Default for PiholeFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
            big: "/usr/share/fonts/04b24.otf".into(),
        }
    }
}

pub struct PiholeMatrix {
    body_font: Font,
    big_font: Font,
}

impl PiholeMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(PiholeFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(PiholeFonts::default()).await
    }

    pub fn with_fonts(paths: PiholeFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
            // 12 px gives a percent number ~9-10 tall — the visual focal
            // point without crowding the label above or the footer below.
            big_font: Font::load_ttf(&paths.big, 12.0)?,
        })
    }

    pub async fn with_fonts_async(paths: PiholeFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    pub fn frame(&self, data: &PiholeSummary) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let body = &self.body_font;
        let big = &self.big_font;

        // Label, top-left.
        draw_text(&mut img, body, 1, LABEL_Y, LABEL, "PIHOLE");

        // Big percent, centered, color tier from intensity.
        let percent_text = format_percent(data.percent_blocked);
        let percent_w = big.text_width(&percent_text);
        let percent_x = ((PANEL_W as i32 - percent_w) / 2).max(0);
        let color = percent_color(data.percent_blocked);
        draw_text(&mut img, big, percent_x, PERCENT_Y, color, &percent_text);

        // Footer: compact "12.3k Q   4.2k BLK".
        let footer_text = format!(
            "{} Q   {} BLK",
            short_count(data.queries_today),
            short_count(data.blocked_today)
        );
        let footer_w = body.text_width(&footer_text);
        let footer_x = ((PANEL_W as i32 - footer_w) / 2).max(0);
        draw_text(&mut img, body, footer_x, FOOTER_Y, FOOTER, &footer_text);

        img
    }
}

impl Default for PiholeMatrix {
    fn default() -> Self {
        Self::new().expect("default PiholeMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for PiholeMatrix {
    type Data = PiholeSummary;

    fn id(&self) -> &'static str {
        "pihole"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(10)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &PiholeSummary) -> Result<(), RenderError> {
        matrix.clear();
        let img = self.frame(data);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(Duration::from_secs(10)).await;
        matrix.clear();
        Ok(())
    }
}

/// Drop the decimal at 100% — `100%` reads cleaner than `100.0%`.
/// Everything else carries one decimal place. Always 4-5 chars wide.
fn format_percent(pct: f32) -> String {
    let pct = pct.clamp(0.0, 100.0);
    if pct >= 99.95 {
        "100%".to_string()
    } else {
        format!("{pct:.1}%")
    }
}

/// `12348` → `"12.3k"`, `4221` → `"4.2k"`, `847` → `"847"`. Keeps the
/// footer compact enough to fit "N Q   N BLK" in 64 px.
fn short_count(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f32 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{}k", n / 1_000)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f32 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Color tier from blocked-percent intensity. Mirrors the storm-band
/// pattern aurora uses — a clear "this is doing a lot" signal vs.
/// "this is quiet".
fn percent_color(pct: f32) -> Color {
    if pct >= 30.0 {
        EMERALD_BRIGHT
    } else if pct >= 10.0 {
        EMERALD_MED
    } else {
        EMERALD_DIM
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_fonts() -> PiholeFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        PiholeFonts {
            body: repo.join("04B_03B_.TTF"),
            big: repo.join("04b24.otf"),
        }
    }

    fn sample(pct: f32, q: u32, blk: u32) -> PiholeSummary {
        PiholeSummary {
            percent_blocked: pct,
            queries_today: q,
            blocked_today: blk,
            unique_clients: 12,
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = PiholeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&sample(34.2, 12_348, 4_221));
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn quiet_install_renders_without_panic() {
        let m = PiholeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&sample(0.0, 0, 0));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 30, "quiet install should still draw label + 0%");
    }

    #[test]
    fn percent_color_bands() {
        assert_eq!(percent_color(0.0), EMERALD_DIM);
        assert_eq!(percent_color(9.9), EMERALD_DIM);
        assert_eq!(percent_color(10.0), EMERALD_MED);
        assert_eq!(percent_color(29.9), EMERALD_MED);
        assert_eq!(percent_color(30.0), EMERALD_BRIGHT);
        assert_eq!(percent_color(100.0), EMERALD_BRIGHT);
    }

    #[test]
    fn format_percent_compact_above_100() {
        // 99.95 rounds to 100 — special-cased so we render "100%" not "100.0%".
        assert_eq!(format_percent(99.95), "100%");
        assert_eq!(format_percent(100.0), "100%");
        assert_eq!(format_percent(34.2), "34.2%");
        assert_eq!(format_percent(7.4), "7.4%");
        assert_eq!(format_percent(0.0), "0.0%");
        assert_eq!(format_percent(-5.0), "0.0%");
        assert_eq!(format_percent(137.0), "100%");
    }

    #[test]
    fn short_count_thresholds() {
        assert_eq!(short_count(0), "0");
        assert_eq!(short_count(847), "847");
        assert_eq!(short_count(999), "999");
        assert_eq!(short_count(1_000), "1.0k");
        assert_eq!(short_count(4_221), "4.2k");
        assert_eq!(short_count(9_999), "10.0k"); // rounding boundary — close enough
        assert_eq!(short_count(12_348), "12k");
        // 999_999 / 1_000 (integer div) = 999, so "999k" — not quite "1.0M".
        assert_eq!(short_count(999_999), "999k");
        assert_eq!(short_count(1_000_000), "1.0M");
        assert_eq!(short_count(2_500_000), "2.5M");
    }

    #[test]
    fn percent_and_footer_have_clear_gap() {
        // Big percent baseline at y=21 (glyphs span ~12..=22), footer
        // baseline at y=31 (glyphs ~25..=31). Rows 23..=24 should be
        // entirely dark.
        let m = PiholeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&sample(34.2, 12_348, 4_221));
        for y in 23..=24u32 {
            let lit = (0..PANEL_W)
                .filter(|&x| img.get_pixel(x, y).0 != [0, 0, 0])
                .count();
            assert_eq!(lit, 0, "row {y} should be clear gap");
        }
    }

    #[test]
    fn high_percent_lights_bright_emerald_pixels() {
        let m = PiholeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&sample(45.0, 10_000, 4_500));
        // Bright emerald: low R, high G, mid B. Look for pixels matching.
        let bright = img
            .pixels()
            .filter(|p| p.0[0] < 100 && p.0[1] > 180 && p.0[2] > 80 && p.0[2] < 180)
            .count();
        assert!(bright > 5, "high-percent percent should paint bright emerald");
    }

    #[test]
    fn quiet_percent_uses_dim_color() {
        let m = PiholeMatrix::with_fonts(repo_fonts()).expect("fonts");
        let bright_img = m.frame(&sample(45.0, 10_000, 4_500));
        let dim_img = m.frame(&sample(2.0, 1_000, 20));
        // Both frames have label + footer pixels in common; the
        // difference shows up in the big percent row. Total
        // (sum-of-R, sum-of-G, sum-of-B) should be lower for the dim case.
        let sum = |img: &RgbImage| -> u64 {
            img.pixels()
                .map(|p| p.0[0] as u64 + p.0[1] as u64 + p.0[2] as u64)
                .sum()
        };
        assert!(
            sum(&bright_img) > sum(&dim_img),
            "bright tier should emit more total light than dim"
        );
    }
}
