//! E-paper rocket-launch renderer — provider, vehicle, big countdown, mission.
//!
//! Static e-paper counterpart to [`crate::matrix::launch::LaunchMatrix`]. The
//! countdown is recomputed at render time from `launch_at`; the flashing-red
//! imminent state of the LED tile becomes a filled status badge (the panel
//! can't animate). Composed white-on-black; the display inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `launch` section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     launch:
//!       run: true
//! ```
//!
//! Data source: The Space Devs / Launch Library 2 (same collector as the LED tile).

use crate::api::launch::model::{LaunchStatus, UpcomingLaunch};
use crate::matrix::eink::layout::{
    badge, badge_width, big_value_centered, center_text, fit_text, footer, header_band, margin, scaled_px,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use image::RgbImage;
use ohmyoled_matrix::graphics::Font;
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Countdown phase derived from `launch_at` vs now (+ status).
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Far,
    Near,
    Imminent,
    Liftoff,
}

fn phase_of(data: &UpcomingLaunch, now: DateTime<Utc>) -> Phase {
    if matches!(data.status, LaunchStatus::InFlight) || data.launch_at <= now {
        return Phase::Liftoff;
    }
    let secs = (data.launch_at - now).num_seconds();
    if secs <= 60 {
        Phase::Imminent
    } else if secs <= 86_400 {
        Phase::Near
    } else {
        Phase::Far
    }
}

fn countdown_line(phase: Phase, data: &UpcomingLaunch, now: DateTime<Utc>) -> String {
    let secs = (data.launch_at - now).num_seconds().max(0);
    match phase {
        Phase::Far => format!("T-{}d {}h", secs / 86_400, (secs % 86_400) / 3_600),
        Phase::Near => format!("T-{:02}:{:02}:{:02}", secs / 3_600, (secs % 3_600) / 60, secs % 60),
        Phase::Imminent => format!("T-00:00:{:02}", secs % 60),
        Phase::Liftoff => "LIFT-OFF".to_string(),
    }
}

/// Status/phase badge text and whether it's filled (high-contrast = alert).
fn status_badge(phase: Phase, status: LaunchStatus) -> (&'static str, bool) {
    match status {
        LaunchStatus::Hold => ("HOLD", true),
        LaunchStatus::Success => ("SUCCESS", false),
        LaunchStatus::Failure => ("FAILURE", true),
        _ => match phase {
            Phase::Liftoff => ("LIFTOFF", true),
            Phase::Imminent => ("IMMINENT", true),
            Phase::Near => ("GO", false),
            Phase::Far => ("UPCOMING", false),
        },
    }
}

/// Font paths for the e-paper launch renderer.
pub struct EinkLaunchFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkLaunchFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper rocket-launch renderer.
pub struct EinkLaunchMatrix {
    title: Font,
    vehicle: Font,
    big: Font,
    badge: Font,
    foot: Font,
}

impl EinkLaunchMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkLaunchFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkLaunchFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkLaunchFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            vehicle: Font::load_ttf(&paths.body, scaled_px(26.0, h))?,
            big: Font::load_ttf(&paths.body, scaled_px(96.0, h))?,
            badge: Font::load_ttf(&paths.body, scaled_px(28.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(20.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkLaunchFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the launch screen at `w × h`, with the countdown sampled at `now`.
    pub fn frame(&self, data: &UpcomingLaunch, now: DateTime<Utc>, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let provider = fit_text(&self.title, &data.provider.to_uppercase(), wi - 2 * m - self.title.text_width(&data.country_code) - 4 * m);
        let content_top = header_band(&mut img, &self.title, &self.vehicle, m, &provider, Some(&data.country_code), fg);

        // Vehicle line.
        center_text(&mut img, &self.vehicle, cx, content_top + self.vehicle.ascent(), fg, &data.vehicle.to_uppercase());

        // Big countdown hero.
        let phase = phase_of(data, now);
        let line = countdown_line(phase, data, now);
        let hero_base = hi * 50 / 100 + self.big.ascent() / 2;
        big_value_centered(&mut img, &self.big, &self.big, cx, hero_base, fg, &line, "");

        // Status badge — filled for imminent/liftoff/hold/failure.
        let (text, filled) = status_badge(phase, data.status);
        let bx = cx - badge_width(&self.badge, text) / 2;
        badge(&mut img, &self.badge, bx, hero_base + m, text, fg, filled);

        // Mission name footer.
        let mission = fit_text(&self.foot, &data.mission, wi - 2 * m);
        footer(&mut img, &self.foot, fg, &mission);
        img
    }
}

#[async_trait]
impl EinkRenderer for EinkLaunchMatrix {
    type Data = UpcomingLaunch;

    fn id(&self) -> &'static str {
        "launch"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &UpcomingLaunch) -> Result<(), RenderError> {
        let now = Utc::now();
        let img = self.frame(data, now, display.width(), display.height());
        display.show(&img);
        // Tick faster through the final minute / liftoff.
        let dwell = match phase_of(data, now) {
            Phase::Imminent | Phase::Liftoff => 15,
            _ => 60,
        };
        tokio::time::sleep(Duration::from_secs(dwell)).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_fonts() -> EinkLaunchFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkLaunchFonts { body: base.join("04B_03B_.TTF") }
    }

    fn sample(offset_secs: i64, status: LaunchStatus) -> UpcomingLaunch {
        UpcomingLaunch {
            provider: "SpaceX".into(),
            vehicle: "Falcon 9".into(),
            mission: "Starlink Group 8-1".into(),
            launch_at: Utc::now() + chrono::Duration::seconds(offset_secs),
            status,
            country_code: "USA".into(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkLaunchMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&sample(3 * 86_400, LaunchStatus::Go), Utc::now(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated launch, got {lit} lit px");
    }

    #[test]
    fn far_and_imminent_differ() {
        let r = EinkLaunchMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let now = Utc::now();
        let far = r.frame(&sample(3 * 86_400, LaunchStatus::Go), now, 800, 480);
        let imm = r.frame(&sample(30, LaunchStatus::Go), now, 800, 480);
        assert_ne!(far.into_raw(), imm.into_raw(), "far vs imminent should differ");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkLaunchMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&sample(3600, LaunchStatus::Go), Utc::now(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
