//! Launch renderer — next-orbital-launch countdown for the 64×32 panel.
//!
//! ```text
//!  ┌──── T-far (> 24h) ─────────┐    ┌──── T-near (< 24h) ────────┐
//!  │ SpaceX                     │    │ SpaceX                     │
//!  │ Falcon 9                   │    │ Falcon 9                   │
//!  │                            │    │                            │
//!  │ T-2d 14h                   │    │ T-03:42:11                 │
//!  │                            │    │                            │
//!  │ Starlink Group 8-5         │    │ Starlink Group 8-5         │
//!  └────────────────────────────┘    └────────────────────────────┘
//!
//!  ┌──── T-imminent (< 60s) ────┐    ┌──── post-launch ───────────┐
//!  │                            │    │ SpaceX                     │
//!  │     T-00:00:08             │    │ Falcon 9                   │
//!  │     ↑ flashing red ↑       │    │                            │
//!  │                            │    │ LIFT-OFF                   │
//!  │     SpaceX Falcon 9        │    │                            │
//!  └────────────────────────────┘    │ Starlink Group 8-5         │
//!                                    └────────────────────────────┘
//! ```
//!
//! The countdown is **computed at render time** from `data.launch_at`
//! so the seconds tick every frame even though the collector polls
//! once per 30 minutes.
//!
//! # Config
//!
//! ```yaml
//! launch:
//!   run: true
//!   agency_filter: []         # ["SpaceX", "ULA"] to narrow; [] = all providers
//! ```
//!
//! # Data source
//!
//! `LaunchCollector::from_lldev` — Launch Library 2 v2.2.0 upcoming
//! endpoint, no auth, 30-minute refresh.

use crate::api::launch::{LaunchStatus, UpcomingLaunch};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

const PROVIDER: Color = Color { r: 200, g: 200, b: 200 };
const VEHICLE: Color = Color { r: 255, g: 255, b: 255 };
const MISSION: Color = Color { r: 150, g: 150, b: 150 };
const T_FAR: Color = Color { r: 0, g: 200, b: 255 };
const T_NEAR: Color = Color { r: 255, g: 200, b: 0 };
const T_IMMINENT_A: Color = Color { r: 255, g: 30, b: 30 };
const T_IMMINENT_B: Color = Color { r: 110, g: 0, b: 0 };
const LIFTOFF: Color = Color { r: 0, g: 255, b: 80 };

/// Frame interval for the live countdown — 1 fps is enough resolution
/// for the seconds digit to tick visibly without burning frames.
const TICK: Duration = Duration::from_secs(1);
/// One full render cycle, in frames. Extended in T-imminent mode so the
/// scheduler doesn't rotate the panel off mid-countdown.
const STATIC_FRAMES: u32 = 15;
const IMMINENT_FRAMES: u32 = 70;

#[derive(Debug, Clone)]
pub struct LaunchFonts {
    pub body: PathBuf,
}

impl Default for LaunchFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
        }
    }
}

pub struct LaunchMatrix {
    body_font: Font,
}

impl LaunchMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(LaunchFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(LaunchFonts::default()).await
    }

    pub fn with_fonts(paths: LaunchFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
        })
    }

    pub async fn with_fonts_async(paths: LaunchFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    pub fn frame(&self, data: &UpcomingLaunch, now: DateTime<Utc>, blink: bool) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let body = &self.body_font;
        let line_h = body.height().max(body.ascent() + 1);

        let mode = countdown_mode(data, now);

        // Top: provider (small grey). Truncate if it doesn't fit the panel.
        let provider = truncate_to_width(&data.provider, PANEL_W as i32 - 2, body);
        draw_text(&mut img, body, 1, body.ascent(), PROVIDER, &provider);

        // Row 2: vehicle (bright white).
        let vehicle = truncate_to_width(&data.vehicle, PANEL_W as i32 - 2, body);
        let vehicle_y = body.ascent() + line_h + 1;
        draw_text(&mut img, body, 1, vehicle_y, VEHICLE, &vehicle);

        // Row 3 (centered, the visual focal point): countdown text or LIFT-OFF banner.
        let (line, color) = countdown_line(mode, data, now, blink);
        let line_w = body.text_width(&line);
        let line_x = ((PANEL_W as i32 - line_w) / 2).max(0);
        let line_y = body.ascent() + 2 * line_h + 4;
        draw_text(&mut img, body, line_x, line_y, color, &line);

        // Row 4: mission (dim grey), truncated to fit.
        if !data.mission.is_empty() {
            let mission = truncate_to_width(&data.mission, PANEL_W as i32 - 2, body);
            let mission_y = (PANEL_H as i32) - 1;
            draw_text(&mut img, body, 1, mission_y, MISSION, &mission);
        }

        img
    }
}

impl Default for LaunchMatrix {
    fn default() -> Self {
        Self::new().expect("default LaunchMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for LaunchMatrix {
    type Data = UpcomingLaunch;

    fn id(&self) -> &'static str {
        "launch"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(STATIC_FRAMES as u64)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &UpcomingLaunch) -> Result<(), RenderError> {
        matrix.clear();
        // Pick frame count based on mode — extend in T-imminent so the
        // launch doesn't rotate off the panel before liftoff.
        let now = Utc::now();
        let frames = match countdown_mode(data, now) {
            CountdownMode::Imminent | CountdownMode::Liftoff => IMMINENT_FRAMES,
            _ => STATIC_FRAMES,
        };
        for tick in 0..frames {
            // Blink in T-imminent and liftoff modes by alternating per frame.
            let blink = tick.is_multiple_of(2);
            let img = self.frame(data, Utc::now(), blink);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(TICK).await;
        }
        matrix.clear();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CountdownMode {
    /// > 24h to launch — "T-Nd Hh" coarse format.
    Far,
    /// 60s..=24h to launch — "T-HH:MM:SS" live ticker.
    Near,
    /// 0s..60s — flashing red big countdown.
    Imminent,
    /// Launch has occurred (status InFlight or net is past) — show
    /// "LIFT-OFF" banner.
    Liftoff,
}

fn countdown_mode(data: &UpcomingLaunch, now: DateTime<Utc>) -> CountdownMode {
    // Once the API has flipped status to InFlight or the launch time
    // has passed, we're in liftoff/recently-launched territory.
    if matches!(data.status, LaunchStatus::InFlight) || data.launch_at <= now {
        return CountdownMode::Liftoff;
    }
    let secs = (data.launch_at - now).num_seconds();
    if secs <= 60 {
        CountdownMode::Imminent
    } else if secs <= 86_400 {
        CountdownMode::Near
    } else {
        CountdownMode::Far
    }
}

/// Produce the headline countdown text + color for the current mode.
/// `blink` toggles a dimmer-red secondary color in T-imminent so the
/// number visibly throbs during the final seconds.
fn countdown_line(
    mode: CountdownMode,
    data: &UpcomingLaunch,
    now: DateTime<Utc>,
    blink: bool,
) -> (String, Color) {
    match mode {
        CountdownMode::Far => {
            let secs = (data.launch_at - now).num_seconds().max(0);
            let days = secs / 86_400;
            let hours = (secs % 86_400) / 3_600;
            (format!("T-{days}d {hours}h"), T_FAR)
        }
        CountdownMode::Near => {
            let secs = (data.launch_at - now).num_seconds().max(0);
            let h = secs / 3_600;
            let m = (secs % 3_600) / 60;
            let s = secs % 60;
            (format!("T-{h:02}:{m:02}:{s:02}"), T_NEAR)
        }
        CountdownMode::Imminent => {
            let secs = (data.launch_at - now).num_seconds().max(0);
            (
                format!("T-00:00:{secs:02}"),
                if blink { T_IMMINENT_A } else { T_IMMINENT_B },
            )
        }
        CountdownMode::Liftoff => ("LIFT-OFF".to_string(), LIFTOFF),
    }
}

/// Truncate `s` to the widest prefix that fits `max_px` in `font`. The
/// last fitting character gets an `…` appended when text was cut.
fn truncate_to_width(s: &str, max_px: i32, font: &Font) -> String {
    if font.text_width(s) <= max_px {
        return s.to_string();
    }
    let ellipsis_w = font.text_width("…");
    let mut out = String::new();
    for ch in s.chars() {
        let mut probe = out.clone();
        probe.push(ch);
        if font.text_width(&probe) + ellipsis_w > max_px {
            break;
        }
        out = probe;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::path::PathBuf;

    fn repo_fonts() -> LaunchFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        LaunchFonts {
            body: repo.join("04B_03B_.TTF"),
        }
    }

    fn sample(at: DateTime<Utc>, status: LaunchStatus) -> UpcomingLaunch {
        UpcomingLaunch {
            provider: "SpaceX".into(),
            vehicle: "Falcon 9".into(),
            mission: "Starlink Group 8-5".into(),
            launch_at: at,
            status,
            country_code: "USA".into(),
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 25, 0, 0, 0).unwrap()
    }

    #[test]
    fn mode_thresholds() {
        let n = now();
        // 2 days out = Far
        assert_eq!(
            countdown_mode(&sample(n + chrono::Duration::days(2), LaunchStatus::Go), n),
            CountdownMode::Far
        );
        // 2 hours out = Near
        assert_eq!(
            countdown_mode(&sample(n + chrono::Duration::hours(2), LaunchStatus::Go), n),
            CountdownMode::Near
        );
        // 30 seconds out = Imminent
        assert_eq!(
            countdown_mode(&sample(n + chrono::Duration::seconds(30), LaunchStatus::Go), n),
            CountdownMode::Imminent
        );
        // Past net = Liftoff
        assert_eq!(
            countdown_mode(&sample(n - chrono::Duration::seconds(5), LaunchStatus::Go), n),
            CountdownMode::Liftoff
        );
        // Status InFlight overrides countdown
        assert_eq!(
            countdown_mode(
                &sample(n + chrono::Duration::days(10), LaunchStatus::InFlight),
                n
            ),
            CountdownMode::Liftoff
        );
    }

    #[test]
    fn far_mode_renders_d_h_text() {
        let n = now();
        let (line, color) = countdown_line(
            CountdownMode::Far,
            &sample(n + chrono::Duration::hours(2 * 24 + 14), LaunchStatus::Go),
            n,
            false,
        );
        assert_eq!(line, "T-2d 14h");
        assert_eq!(color, T_FAR);
    }

    #[test]
    fn near_mode_renders_hms_text() {
        let n = now();
        let (line, _) = countdown_line(
            CountdownMode::Near,
            &sample(n + chrono::Duration::seconds(3 * 3600 + 42 * 60 + 11), LaunchStatus::Go),
            n,
            false,
        );
        assert_eq!(line, "T-03:42:11");
    }

    #[test]
    fn imminent_mode_blinks_between_two_reds() {
        let n = now();
        let l = sample(n + chrono::Duration::seconds(8), LaunchStatus::Go);
        let (line_on, col_on) = countdown_line(CountdownMode::Imminent, &l, n, true);
        let (line_off, col_off) = countdown_line(CountdownMode::Imminent, &l, n, false);
        assert_eq!(line_on, "T-00:00:08");
        assert_eq!(line_off, "T-00:00:08");
        assert_ne!(col_on, col_off, "blink should alternate colors");
    }

    #[test]
    fn liftoff_text_is_banner() {
        let n = now();
        let (line, color) = countdown_line(
            CountdownMode::Liftoff,
            &sample(n - chrono::Duration::seconds(2), LaunchStatus::InFlight),
            n,
            true,
        );
        assert_eq!(line, "LIFT-OFF");
        assert_eq!(color, LIFTOFF);
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        let n = now();
        let img = m.frame(&sample(n + chrono::Duration::hours(5), LaunchStatus::Go), n, false);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn far_and_imminent_frames_differ() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        let n = now();
        let far = m.frame(&sample(n + chrono::Duration::days(2), LaunchStatus::Go), n, false);
        let imm = m.frame(&sample(n + chrono::Duration::seconds(8), LaunchStatus::Go), n, true);
        assert_ne!(far.as_raw(), imm.as_raw());
    }

    #[test]
    fn long_mission_name_truncates() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        let n = now();
        let mut s = sample(n + chrono::Duration::days(2), LaunchStatus::Go);
        s.mission = "Some Extremely Long Mission Name That Overflows The Panel".into();
        // Shouldn't panic and should still render lit pixels.
        let img = m.frame(&s, n, false);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80);
    }

    #[test]
    fn truncate_to_width_appends_ellipsis() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        // Force "AAAAAAAA..." into a tiny pixel budget.
        let short = truncate_to_width("AAAAAAAAAAAAAAAAAAAA", 16, &m.body_font);
        assert!(short.ends_with('…'));
        assert!(m.body_font.text_width(&short) <= 16);
    }

    #[test]
    fn truncate_to_width_passes_through_short_text() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        assert_eq!(truncate_to_width("Hi", 64, &m.body_font), "Hi");
    }
}
