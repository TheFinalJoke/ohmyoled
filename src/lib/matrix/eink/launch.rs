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
    badge, badge_width, center_text, fill_rect, fit_text, footer, header_band, margin, rect, scaled_px,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_circle, draw_line, draw_text, Font};
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
    seg: Font,
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
            vehicle: Font::load_ttf(&paths.body, scaled_px(24.0, h))?,
            big: Font::load_ttf(&paths.body, scaled_px(78.0, h))?,
            seg: Font::load_ttf(&paths.body, scaled_px(16.0, h))?,
            badge: Font::load_ttf(&paths.body, scaled_px(26.0, h))?,
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

        let phase = phase_of(data, now);
        let active = matches!(phase, Phase::Imminent | Phase::Liftoff);

        let provider = fit_text(&self.title, &data.provider.to_uppercase(), wi - 2 * m - self.title.text_width(&data.country_code) - 4 * m);
        let content_top = header_band(&mut img, &self.title, &self.vehicle, m, &provider, Some(&data.country_code), fg);
        let footer_top = hi - m - self.foot.height();

        // ── Rocket illustration down the left ───────────────────────────
        let rocket_w = wi * 20 / 100;
        self.draw_rocket(&mut img, m + rocket_w / 2, content_top + m, footer_top - m, active, fg);

        // ── Right column: vehicle, countdown, status, scheduled time ────
        let rx = rocket_w + m;
        let rcx = (rx + wi - m) / 2;

        center_text(&mut img, &self.vehicle, rcx, content_top + m + self.vehicle.ascent(), fg, &data.vehicle.to_uppercase());

        // Hero: segmented countdown (or a LIFT-OFF banner once it's flying).
        let hero_base = hi * 44 / 100 + self.big.ascent() / 2;
        if matches!(phase, Phase::Liftoff) {
            center_text(&mut img, &self.big, rcx, hero_base, fg, "LIFT-OFF");
        } else {
            let secs = (data.launch_at - now).num_seconds().max(0);
            self.draw_countdown(&mut img, rcx, hero_base, secs, fg);
        }

        // Status badge.
        let (text, filled) = status_badge(phase, data.status);
        let badge_y = hero_base + m + self.seg.height();
        badge(&mut img, &self.badge, rcx - badge_width(&self.badge, text) / 2, badge_y, text, fg, filled);

        // Scheduled launch time (absolute).
        let when = data.launch_at.format("%a %b %-d   %H:%M UTC").to_string();
        center_text(&mut img, &self.vehicle, rcx, badge_y + self.badge.height() + m + self.vehicle.ascent(), fg, &format!("T-0  {when}"));

        // Mission name footer.
        let mission = fit_text(&self.foot, &data.mission, wi - 2 * m);
        footer(&mut img, &self.foot, fg, &mission);
        img
    }

    /// Big DD:HH:MM:SS countdown with little DAYS/HRS/MIN/SEC labels, centered
    /// on `cx` at baseline `base`.
    fn draw_countdown(&self, img: &mut RgbImage, cx: i32, base: i32, secs: i64, fg: Color) {
        let parts = [
            (format!("{}", secs / 86_400), "DAYS"),
            (format!("{:02}", (secs % 86_400) / 3_600), "HRS"),
            (format!("{:02}", (secs % 3_600) / 60), "MIN"),
            (format!("{:02}", secs % 60), "SEC"),
        ];
        let colon = ":";
        let cw = self.big.text_width(colon);
        let total: i32 = parts.iter().map(|(p, _)| self.big.text_width(p)).sum::<i32>() + cw * 3;
        let label_y = base + self.seg.ascent();
        let mut x = cx - total / 2;
        for (i, (p, lab)) in parts.iter().enumerate() {
            let pw = self.big.text_width(p);
            draw_text(img, &self.big, x, base, fg, p);
            center_text(img, &self.seg, x + pw / 2, label_y, fg, lab);
            x += pw;
            if i < 3 {
                draw_text(img, &self.big, x, base, fg, colon);
                x += cw;
            }
        }
    }

    /// A line-art rocket standing in the given vertical band, centered on `cx`.
    /// `flames` adds exhaust (drawn for imminent/liftoff).
    fn draw_rocket(&self, img: &mut RgbImage, cx: i32, top: i32, bottom: i32, flames: bool, fg: Color) {
        let h = bottom - top;
        let hw = (h / 12).clamp(8, 26); // body half-width
        let nose_h = h / 5;
        let body_top = top + nose_h;
        let body_bot = top + h * 7 / 10;
        // Nosecone.
        draw_line(img, cx, top, cx - hw, body_top, fg);
        draw_line(img, cx, top, cx + hw, body_top, fg);
        // Body.
        rect(img, cx - hw, body_top, 2 * hw, body_bot - body_top, fg);
        // Window.
        draw_circle(img, cx, body_top + (body_bot - body_top) / 3, (hw / 3).max(3), fg);
        // Fins.
        let fin = (h / 12).clamp(8, 24);
        draw_line(img, cx - hw, body_bot - fin, cx - hw - fin, body_bot, fg);
        draw_line(img, cx - hw - fin, body_bot, cx - hw, body_bot, fg);
        draw_line(img, cx + hw, body_bot - fin, cx + hw + fin, body_bot, fg);
        draw_line(img, cx + hw + fin, body_bot, cx + hw, body_bot, fg);
        // Exhaust flames (a little flicker of lines) when active.
        if flames {
            for (i, dx) in [-hw / 2, 0, hw / 2].into_iter().enumerate() {
                let len = if i == 1 { h / 6 } else { h / 9 };
                draw_line(img, cx + dx, body_bot, cx + dx, (body_bot + len).min(bottom), fg);
            }
            fill_rect(img, cx - hw / 2, body_bot, hw + 1, 3, fg);
        }
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
