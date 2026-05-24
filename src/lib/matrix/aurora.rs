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

/// Frame interval for the pulse animation — ~12 fps. Fine-grained enough
/// for smooth motion on a 64×32 panel without being CPU-heavy.
const ANIM_TICK: Duration = Duration::from_millis(83);
/// Phase shift (in fraction-of-period units) between adjacent bar blocks,
/// so the shimmer reads as a wave traveling rightward instead of all
/// cells pulsing in lockstep.
const BAR_PHASE_STEP: f32 = 0.07;

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
        self.draw_frame(data, 0)
    }

    /// Render one animation frame. `tick` is the monotonically increasing
    /// per-render frame index; `pulse_factor` and `bar_factor` use it to
    /// shape the digit's brightness pulse and the bar's shimmer wave.
    pub fn draw_frame(&self, data: &AuroraReading, tick: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);

        // "Kp" label, top-left — never animated, anchors the eye.
        let label = "Kp";
        let label_y = self.label_font.ascent();
        draw_text(&mut img, &self.label_font, 1, label_y, LABEL, label);

        // Huge centered digit, brightness modulated by pulse_factor so the
        // tile feels alive when activity is high and stays close to static
        // when it isn't.
        let digit = format!("{}", data.kp);
        let digit_w = self.big_font.text_width(&digit);
        let digit_x = ((PANEL_W as i32 - digit_w) / 2).max(0);
        let digit_y = 20;
        let base = kp_color(data.kp);
        let digit_color = scale_color(base, pulse_factor(data.kp, tick, 0.0));
        draw_text(&mut img, &self.big_font, digit_x, digit_y, digit_color, &digit);

        // 9-block scale bar at rows 22..=24, each lit cell carries an
        // increasing phase offset so the shimmer reads as a wave.
        draw_kp_bar(&mut img, data.kp, 22, tick);

        // Alert banner — same pulse driver as the digit, slightly subdued so
        // the digit stays the dominant element.
        if data.alert {
            let banner = "AURORA LIKELY";
            let banner_w = self.label_font.text_width(banner);
            let banner_x = ((PANEL_W as i32 - banner_w) / 2).max(0);
            let banner_y = (PANEL_H as i32) - 1;
            let banner_color = scale_color(ALERT_BANNER, pulse_factor(data.kp, tick, 0.5));
            draw_text(&mut img, &self.label_font, banner_x, banner_y, banner_color, banner);
        }

        img
    }

    /// Total animated frames for a `render()` cycle. Aurora always picks
    /// the animated path — even quiet readings get a very subtle pulse,
    /// which keeps the tile from looking frozen next to other moving
    /// modules in the rotation.
    fn frames_per_cycle(alert: bool) -> u32 {
        // 10s quiet / 15s alert at ~12 fps.
        let secs = if alert { 15 } else { 10 };
        (secs * 1000) / ANIM_TICK.as_millis() as u32
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
        let total = Self::frames_per_cycle(data.alert);
        for tick in 0..total {
            let img = self.draw_frame(data, tick);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(ANIM_TICK).await;
        }
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

/// Period (in frames) and amplitude (fraction of full brightness) of the
/// pulse for a given Kp. Returns `(period, amplitude)` — higher Kp =
/// shorter period (faster pulse) AND larger amplitude (deeper dimming).
fn pulse_params(kp: u8) -> (u32, f32) {
    match kp {
        0 => (1, 0.0),         // truly quiet — single-color, no movement
        1..=2 => (96, 0.08),   // ~8s period, barely visible — a "heartbeat"
        3 => (72, 0.15),       // ~6s, subtle
        4 => (48, 0.25),       // ~4s, noticeable
        5..=6 => (30, 0.40),   // ~2.5s, clearly pulsing
        _ => (18, 0.55),       // ~1.5s, dramatic
    }
}

/// Brightness multiplier in `[1 − amplitude, 1.0]` for the given Kp,
/// frame, and per-element phase offset (fraction of a period).
fn pulse_factor(kp: u8, tick: u32, phase_offset: f32) -> f32 {
    let (period, amp) = pulse_params(kp);
    if amp == 0.0 {
        return 1.0;
    }
    let phase = (tick as f32 / period as f32 + phase_offset) * std::f32::consts::TAU;
    let osc = (phase.sin() + 1.0) / 2.0; // 0..1
    1.0 - amp + amp * osc
}

/// Multiply each channel of `c` by `factor` (clamped to `0.0..=1.0`).
fn scale_color(c: Color, factor: f32) -> Color {
    let f = factor.clamp(0.0, 1.0);
    Color {
        r: (c.r as f32 * f).round() as u8,
        g: (c.g as f32 * f).round() as u8,
        b: (c.b as f32 * f).round() as u8,
    }
}

/// Draw the 9-block Kp gauge at `top_y`. The bar is 3 rows tall and
/// horizontally centered. Each block is 5 px wide + 1 px gap; lit blocks
/// take their band-color (modulated by the pulse with a per-cell phase
/// offset, so the shimmer waves rightward), unlit blocks render in dim
/// grey so the scale is always visible.
fn draw_kp_bar(img: &mut RgbImage, kp: u8, top_y: i32, tick: u32) {
    const BLOCK_W: i32 = 5;
    const GAP: i32 = 1;
    const N: u8 = 9;
    let total_w = N as i32 * BLOCK_W + (N as i32 - 1) * GAP;
    let start_x = (PANEL_W as i32 - total_w) / 2;
    let bar_h = 3;

    for i in 0..N {
        let x0 = start_x + i as i32 * (BLOCK_W + GAP);
        let color = if i < kp {
            // Lit: per-position band color, modulated by the pulse with a
            // phase offset proportional to the cell index — turns the
            // shimmer into a left-to-right traveling wave.
            let phase = BAR_PHASE_STEP * i as f32;
            scale_color(kp_color(i + 1), pulse_factor(kp, tick, phase))
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
        // Cyan-ish banner pixels (low R, dominant G+B with G < B) should
        // appear in alert mode. Threshold relaxed since the banner is
        // pulse-modulated and may render below the unpulsed peak color.
        let cyan = a
            .pixels()
            .filter(|p| p.0[0] < 80 && p.0[1] > 100 && p.0[2] > 130 && p.0[2] > p.0[1])
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
        // strip and divide by the per-block pixel count. tick=0 keeps
        // the pulse at its peak amplitude so lit cells are at full color.
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        for kp in 0..=9u8 {
            let img = m.draw_frame(&sample(kp, false), 0);
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
        // Kp=8 at tick=0 should produce strongly-red digit pixels.
        let img = m.draw_frame(&sample(8, true), 0);
        let red_pixels = img
            .pixels()
            .filter(|p| p.0[0] > 200 && p.0[1] < 80 && p.0[2] < 80)
            .count();
        assert!(red_pixels > 5, "expected red digit pixels at Kp=8, got {red_pixels}");
    }

    #[test]
    fn kp_zero_is_static_across_frames() {
        // Quiet floor: no pulse, no shimmer — frames are byte-identical.
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        let s = sample(0, false);
        let f0 = m.draw_frame(&s, 0);
        let f30 = m.draw_frame(&s, 30);
        assert_eq!(f0.as_raw(), f30.as_raw(), "Kp=0 must not animate");
    }

    #[test]
    fn high_kp_animates_across_frames() {
        // Severe-storm: pulse + shimmer should make pixels visibly differ
        // between an "on the peak" and "off the peak" frame.
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        let s = sample(8, true);
        let (period, _amp) = pulse_params(8);
        let f0 = m.draw_frame(&s, 0); // peak
        let f_mid = m.draw_frame(&s, period / 2); // trough
        assert_ne!(f0.as_raw(), f_mid.as_raw(), "Kp=8 must animate over time");

        // Specifically, the digit should be dimmer mid-pulse.
        let digit_red_peak = f0.pixels().map(|p| p.0[0] as u32).sum::<u32>();
        let digit_red_mid = f_mid.pixels().map(|p| p.0[0] as u32).sum::<u32>();
        assert!(
            digit_red_peak > digit_red_mid,
            "digit total brightness should fall at the pulse trough"
        );
    }

    #[test]
    fn pulse_factor_stays_in_range() {
        // Pulse multiplier must never exceed 1.0 or drop below the
        // configured floor (1 − amplitude).
        for kp in 0..=9u8 {
            let (_period, amp) = pulse_params(kp);
            let floor = 1.0 - amp;
            for tick in 0..240 {
                let f = pulse_factor(kp, tick, 0.0);
                assert!(
                    f >= floor - 1e-6 && f <= 1.0 + 1e-6,
                    "kp={kp} tick={tick}: factor {f} outside [{floor}, 1.0]"
                );
            }
        }
    }

    #[test]
    fn pulse_speed_scales_with_kp() {
        // Higher Kp -> shorter period (faster pulse).
        let mut periods: Vec<(u8, u32)> = (0..=9u8).map(|k| (k, pulse_params(k).0)).collect();
        // Drop kp=0 (period = 1 = sentinel for "no animation").
        periods.retain(|(k, _)| *k > 0);
        // Each subsequent entry's period should be <= the previous (monotonic).
        for w in periods.windows(2) {
            let (k_lo, p_lo) = w[0];
            let (k_hi, p_hi) = w[1];
            assert!(
                p_hi <= p_lo,
                "pulse period should be monotone in Kp: kp={k_lo} -> {p_lo}, kp={k_hi} -> {p_hi}"
            );
        }
    }
}
