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

/// Frame interval for the render loop. 50 ms (~20 fps) buys smooth
/// pixel-by-pixel mission marquee without the live countdown stuttering.
const TICK: Duration = Duration::from_millis(50);
const FPS: u32 = 20;
/// One full render cycle, in *frames* (not seconds). Extended in
/// T-imminent and Liftoff modes so the panel doesn't rotate off mid-
/// countdown.
const STATIC_FRAMES: u32 = 15 * FPS; // 15 s
const IMMINENT_FRAMES: u32 = 70 * FPS; // 70 s
/// Mission marquee pacing: pause at start, scroll left, pause at end,
/// then loop. All in 20 fps frames.
const MISSION_SETTLE: u32 = 2 * FPS;
const MISSION_TRAIL_PAUSE: u32 = FPS;
const MISSION_OVERSCROLL: i32 = 12;

// Fixed y-coordinates for the four rows. Hardcoded (rather than derived
// from font metrics) so we can guarantee a visible gap between the
// countdown row and the mission row — the original ascent-derived layout
// put them 2 rows apart and they visually blurred together.
//   rows  0–5  : provider     (baseline y = 5)
//   rows  6–12 : vehicle      (baseline y = 12)
//   rows 14–20 : countdown    (baseline y = 20)
//   rows 24–31 : mission      (baseline y = 31)
// Gap between countdown bottom (row 21 with descender) and mission top
// (row 24) is 3 dark rows — enough breathing room that the eye reads
// the countdown and the mission caption as distinct elements.
const PROVIDER_Y: i32 = 5;
const VEHICLE_Y: i32 = 12;
const COUNTDOWN_Y: i32 = 20;
const MISSION_Y: i32 = 31;

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
        self.draw_frame(data, now, blink, 0)
    }

    /// Render one frame at the given mission-marquee offset. `scroll_px`
    /// shifts the mission row leftward by N pixels — 0 means settled at
    /// the left margin. The provider/vehicle/countdown rows are unaffected
    /// so the at-a-glance read stays stable.
    pub fn draw_frame(
        &self,
        data: &UpcomingLaunch,
        now: DateTime<Utc>,
        blink: bool,
        scroll_px: i32,
    ) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let body = &self.body_font;
        let mode = countdown_mode(data, now);

        // Top: provider (small grey). Truncates with `…` if too long.
        let provider = truncate_to_width(&data.provider, PANEL_W as i32 - 2, body);
        draw_text(&mut img, body, 1, PROVIDER_Y, PROVIDER, &provider);

        // Vehicle (bright white).
        let vehicle = truncate_to_width(&data.vehicle, PANEL_W as i32 - 2, body);
        draw_text(&mut img, body, 1, VEHICLE_Y, VEHICLE, &vehicle);

        // Countdown (centered, color-coded by mode).
        let (line, color) = countdown_line(mode, data, now, blink);
        let line_w = body.text_width(&line);
        let line_x = ((PANEL_W as i32 - line_w) / 2).max(0);
        draw_text(&mut img, body, line_x, COUNTDOWN_Y, color, &line);

        // Mission row at the bottom — marquees when the rendered text
        // would overflow the panel so the full name reads over time
        // instead of truncating mid-word.
        if !data.mission.is_empty() {
            draw_text(&mut img, body, 1 - scroll_px, MISSION_Y, MISSION, &data.mission);
        }

        img
    }

    /// True when the mission text wouldn't fit on the panel statically.
    fn mission_overflows(&self, data: &UpcomingLaunch) -> bool {
        self.body_font.text_width(&data.mission) > PANEL_W as i32 - 2
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
        Duration::from_millis((STATIC_FRAMES as u64) * (TICK.as_millis() as u64))
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &UpcomingLaunch) -> Result<(), RenderError> {
        matrix.clear();
        let frames = match countdown_mode(data, Utc::now()) {
            CountdownMode::Imminent | CountdownMode::Liftoff => IMMINENT_FRAMES,
            _ => STATIC_FRAMES,
        };
        let mission_w = self.body_font.text_width(&data.mission);
        let mission_overflows = self.mission_overflows(data);
        for tick in 0..frames {
            // ~2 Hz blink in imminent/liftoff modes — slow enough to read.
            let blink = (tick / (FPS / 2)).is_multiple_of(2);
            let scroll_px = mission_scroll_px(tick, mission_w, mission_overflows);
            let img = self.draw_frame(data, Utc::now(), blink, scroll_px);
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

/// X-offset for the mission row at frame `tick`. When the text fits
/// statically we return 0 (no scroll). Otherwise we cycle: settle for
/// MISSION_SETTLE frames, scroll left at 1 px/frame until the text is
/// fully off the left edge, pause briefly, then reset and repeat.
fn mission_scroll_px(tick: u32, text_w: i32, overflows: bool) -> i32 {
    if !overflows {
        return 0;
    }
    let scroll_frames = (text_w + MISSION_OVERSCROLL).max(1) as u32;
    let cycle = MISSION_SETTLE + scroll_frames + MISSION_TRAIL_PAUSE;
    let local = tick % cycle;
    if local < MISSION_SETTLE {
        0
    } else if local < MISSION_SETTLE + scroll_frames {
        (local - MISSION_SETTLE) as i32
    } else {
        // Trail pause holds the line just off the right edge, then loops.
        scroll_frames as i32
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

    /// Regression for the "Starlink Ground" complaint: the countdown row
    /// (baseline y=20, glyphs span ~14..=21) and the mission row
    /// (baseline y=31, glyphs span ~25..=31) must leave at least one
    /// fully-dark row between them, so they don't visually merge.
    #[test]
    fn countdown_and_mission_have_clear_gap() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        let n = now();
        let img = m.frame(&sample(n + chrono::Duration::days(2), LaunchStatus::Go), n, false);
        // Rows 22, 23, 24 should be entirely dark — the dividing gap
        // between countdown bottom and mission top.
        for y in 22..=24u32 {
            let lit = (0..PANEL_W)
                .filter(|&x| img.get_pixel(x, y).0 != [0, 0, 0])
                .count();
            assert_eq!(
                lit, 0,
                "row {y} should be a clear gap between countdown and mission"
            );
        }
    }

    #[test]
    fn long_mission_marquees() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        let n = now();
        let mut s = sample(n + chrono::Duration::days(2), LaunchStatus::Go);
        s.mission = "Starlink Group 8-5 with extra long suffix".into();
        assert!(m.mission_overflows(&s), "this mission should overflow");

        // Mission row (rows 25..=31) should look different at scroll_px=0
        // vs scroll_px=12 — the marquee has shifted the text leftward.
        let f0 = m.draw_frame(&s, n, false, 0);
        let fmid = m.draw_frame(&s, n, false, 12);
        let row_diff = (25..32u32)
            .flat_map(|y| (0..PANEL_W).map(move |x| (x, y)))
            .filter(|&(x, y)| f0.get_pixel(x, y) != fmid.get_pixel(x, y))
            .count();
        assert!(row_diff > 5, "mission row should visibly shift with scroll_px");

        // The other rows (provider/vehicle/countdown) must stay identical
        // between scroll positions — only the mission row animates.
        let upper_diff = (0..24u32)
            .flat_map(|y| (0..PANEL_W).map(move |x| (x, y)))
            .filter(|&(x, y)| f0.get_pixel(x, y) != fmid.get_pixel(x, y))
            .count();
        assert_eq!(upper_diff, 0, "non-mission rows must stay static");
    }

    #[test]
    fn short_mission_does_not_marquee() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut s = sample(now() + chrono::Duration::days(2), LaunchStatus::Go);
        s.mission = "Crew-9".into();
        assert!(!m.mission_overflows(&s), "'Crew-9' should fit statically");
    }

    /// Regression for the original user complaint: the default sample
    /// mission "Starlink Group 8-5" is wide enough that it has to
    /// marquee on the 64-px panel. If this ever stops being true (font
    /// metrics drift, mission name shortens, panel widens) the
    /// renderer needs a different overflow strategy.
    #[test]
    fn starlink_group_8_5_actually_overflows() {
        let m = LaunchMatrix::with_fonts(repo_fonts()).expect("fonts");
        let s = sample(now() + chrono::Duration::days(2), LaunchStatus::Go);
        assert_eq!(s.mission, "Starlink Group 8-5");
        assert!(
            m.mission_overflows(&s),
            "default mission must overflow — that's why we wired up the marquee"
        );
    }

    #[test]
    fn mission_scroll_px_holds_at_zero_when_no_overflow() {
        for tick in 0..1000 {
            assert_eq!(mission_scroll_px(tick, 100, false), 0);
        }
    }

    #[test]
    fn mission_scroll_px_advances_during_scroll_phase() {
        // 80 px text -> scroll_frames = 92. Settle 40 frames, then scroll.
        // At tick=40 we're at the start of the scroll phase, scroll=0.
        // At tick=50 we should be 10 px in.
        let s0 = mission_scroll_px(40, 80, true);
        let s10 = mission_scroll_px(50, 80, true);
        assert_eq!(s0, 0);
        assert_eq!(s10, 10);
    }
}
