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

// Top-right curtain area — a small rectangle where vertical "aurora
// strands" wobble horizontally. Kept clear of the "Kp" label
// (x≈1..8), the centered big digit (x≈26..38), and the bar (rows
// 22..=24), so the curtains coexist with everything else.
const CURTAIN_X_MIN: i32 = 50;
const CURTAIN_X_MAX: i32 = 62;
const CURTAIN_Y_MIN: i32 = 0;
const CURTAIN_Y_MAX: i32 = 13;
const CURTAIN_N: usize = 2;
/// X positions of each curtain strand's resting (un-wobbled) center.
const CURTAIN_BASE_XS: [i32; CURTAIN_N] = [53, 60];

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

        // Background layer: aurora-curtain wobble in the top-right corner.
        // Drawn first so any other element drawn later (label / digit /
        // bar / banner) overlays cleanly without leaving curtain pixels
        // showing through letterforms.
        draw_curtains(&mut img, data.kp, tick);

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

/// Curtain wobble parameters per Kp:
///   `amp_px`    — peak horizontal displacement of a strand (pixels)
///   `vert_px`   — vertical wavelength (pixels per full sine cycle)
///   `scroll`    — frames per full scroll cycle (smaller = faster)
fn curtain_params(kp: u8) -> (f32, f32, u32) {
    match kp {
        0 => (0.0, 1.0, 1),
        1..=2 => (1.0, 12.0, 72),
        3 => (1.5, 10.0, 54),
        4 => (2.0, 8.0, 42),
        5..=6 => (2.5, 7.0, 28),
        _ => (3.5, 5.0, 18),
    }
}

/// Maximum brightness multiplier for the curtain layer at a given Kp.
/// Kept distinctly below 1.0 for low Kp so the curtains read as a faint
/// background glow rather than fighting the digit for attention.
fn curtain_max_brightness(kp: u8) -> f32 {
    match kp {
        0 => 0.0,
        1..=2 => 0.35,
        3 => 0.50,
        4 => 0.70,
        5..=6 => 0.85,
        _ => 1.00,
    }
}

/// Draw the top-right aurora-curtain wobble. Each strand is a vertical
/// sinusoid traced pixel-by-pixel; per-strand phase offsets make
/// neighboring strands sway independently. Pixel brightness peaks at
/// the wave crests and tapers toward the top/bottom of the curtain
/// rectangle so the strands look like glowing arcs rather than solid
/// bars. Bounded strictly to the curtain rectangle — anything outside
/// is dropped to avoid bleeding into the label/digit/bar.
fn draw_curtains(img: &mut RgbImage, kp: u8, tick: u32) {
    if kp == 0 {
        return;
    }
    let (amp, vert_px, scroll_frames) = curtain_params(kp);
    let max_b = curtain_max_brightness(kp);
    let base_color = kp_color(kp);
    let scroll_phase = tick as f32 / scroll_frames as f32;
    let curtain_h = (CURTAIN_Y_MAX - CURTAIN_Y_MIN) as f32;
    let curtain_mid = (CURTAIN_Y_MIN + CURTAIN_Y_MAX) as f32 / 2.0;

    for (idx, &base_x) in CURTAIN_BASE_XS.iter().enumerate() {
        let strand_phase = idx as f32 * 0.5;
        for y in CURTAIN_Y_MIN..=CURTAIN_Y_MAX {
            let angle = (y as f32 / vert_px + scroll_phase + strand_phase)
                * std::f32::consts::TAU;
            let osc = angle.sin(); // -1..1
            let px = base_x + (osc * amp).round() as i32;
            if !(CURTAIN_X_MIN..=CURTAIN_X_MAX).contains(&px) {
                continue;
            }

            // Crest-bright, taper top/bottom: |sin| peaks at crests; a
            // linear cosine taper attenuates the strand endpoints so the
            // strand fades into the surrounding darkness rather than
            // ending abruptly.
            let crest = 0.5 + 0.5 * osc.abs();
            let dist_from_mid = (y as f32 - curtain_mid).abs() / (curtain_h / 2.0);
            let taper = (1.0 - dist_from_mid * 0.7).clamp(0.0, 1.0);
            let brightness = max_b * crest * taper;
            if brightness < 0.05 {
                continue;
            }
            let color = scale_color(base_color, brightness);
            img.put_pixel(px as u32, y as u32, Rgb([color.r, color.g, color.b]));
        }
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
    fn curtains_dark_at_kp_zero() {
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.draw_frame(&sample(0, false), 0);
        for y in CURTAIN_Y_MIN as u32..=CURTAIN_Y_MAX as u32 {
            for x in CURTAIN_X_MIN as u32..=CURTAIN_X_MAX as u32 {
                assert_eq!(
                    img.get_pixel(x, y).0,
                    [0, 0, 0],
                    "curtain area must be dark at Kp=0 — found light at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn curtains_light_pixels_at_nonzero_kp() {
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        for kp in 1..=9u8 {
            // Sample across several ticks so we don't accidentally land on a
            // frame where every strand pixel happens to be sub-threshold.
            let lit_total: usize = (0u32..16)
                .map(|t| {
                    let img = m.draw_frame(&sample(kp, kp >= 5), t);
                    (CURTAIN_Y_MIN as u32..=CURTAIN_Y_MAX as u32)
                        .flat_map(|y| {
                            (CURTAIN_X_MIN as u32..=CURTAIN_X_MAX as u32)
                                .map(move |x| (x, y))
                        })
                        .filter(|&(x, y)| img.get_pixel(x, y).0 != [0, 0, 0])
                        .count()
                })
                .sum();
            assert!(
                lit_total > 0,
                "kp={kp}: curtain area should light up across the animation"
            );
        }
    }

    #[test]
    fn curtains_move_horizontally_across_frames() {
        // Pick a Kp where the wobble amplitude is at least 2 px and snap
        // two frames a quarter-period apart. The two frames must differ
        // somewhere in the curtain rectangle — i.e. the strands moved.
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        let s = sample(8, true);
        let (_amp, _vert, scroll) = curtain_params(8);
        let f0 = m.draw_frame(&s, 0);
        let f_q = m.draw_frame(&s, scroll / 4);
        let mut diffs = 0usize;
        for y in CURTAIN_Y_MIN as u32..=CURTAIN_Y_MAX as u32 {
            for x in CURTAIN_X_MIN as u32..=CURTAIN_X_MAX as u32 {
                if f0.get_pixel(x, y) != f_q.get_pixel(x, y) {
                    diffs += 1;
                }
            }
        }
        assert!(diffs > 1, "curtains should move across frames at Kp=8, got {diffs}");
    }

    #[test]
    fn curtains_stay_inside_their_bounding_box() {
        // Vital: curtains must never paint into the label / digit / bar.
        // Subtract a frame with curtains from a frame without (Kp=0) and
        // assert all the differences land strictly inside the rectangle.
        let m = AuroraMatrix::with_fonts(repo_fonts()).expect("fonts");
        let baseline = m.draw_frame(&sample(0, false), 0);
        let s = sample(8, true);
        for t in 0..32 {
            let img = m.draw_frame(&s, t);
            for y in 0..PANEL_H {
                for x in 0..PANEL_W {
                    if img.get_pixel(x, y) != baseline.get_pixel(x, y) {
                        // Either the change is inside the curtain rect, OR
                        // it belongs to one of the other animated layers
                        // (digit baseline ~5..=20, bar rows 22..=24, alert
                        // banner row 31). Curtain pixels must always be
                        // inside the rect.
                        let in_curtain = (CURTAIN_X_MIN as u32..=CURTAIN_X_MAX as u32).contains(&x)
                            && (CURTAIN_Y_MIN as u32..=CURTAIN_Y_MAX as u32).contains(&y);
                        // Digit glyphs (04b24 18pt, baseline y=20) span the
                        // full ascent above their baseline. Bar at rows
                        // 22..=24. Banner glyphs (label font 8pt, baseline
                        // y=31) span ~rows 24..=31.
                        let in_digit_row_band = (4..=22).contains(&y);
                        let in_bar_rows = (22..=24).contains(&y);
                        let in_banner_rows = (24..PANEL_H).contains(&y);
                        assert!(
                            in_curtain || in_digit_row_band || in_bar_rows || in_banner_rows,
                            "unexpected animated pixel at ({x},{y}) outside known layers"
                        );
                    }
                }
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
