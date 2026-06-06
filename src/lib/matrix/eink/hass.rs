//! E-paper Home Assistant renderer — one entity, in state / historical / graph
//! mode.
//!
//! Static e-paper counterpart to [`crate::matrix::hass::HassMatrix`]. Carries
//! the same [`HassDisplay`] config (mode + alarm state) alongside the entity;
//! the colors are ignored on the monochrome panel — an alarm match is shown as
//! a filled badge instead of a color flip. Composed white-on-black; the display
//! inverts to black ink.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `hass` section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     hass:
//!       run: true
//!       base_url: "http://homeassistant.local:8123"
//!       token: "REPLACE_ME_HASS_TOKEN"
//!       entity_id: sensor.kitchen_temp
//!       display_mode: graph
//! ```
//!
//! Data source: the Home Assistant REST API (same collector as the LED tile).

use crate::api::hass::model::HassEntity;
use crate::matrix::eink::layout::{
    badge, badge_width, big_value_centered, center_text, fit_text, footer, header_band, margin, scaled_px, sparkline,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use crate::matrix::hass::{HassDisplay, HassDisplayMode};
use async_trait::async_trait;
use chrono::Utc;
use image::RgbImage;
use ohmyoled_matrix::graphics::Font;
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper Home Assistant renderer.
pub struct EinkHassFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkHassFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper Home Assistant entity renderer.
pub struct EinkHassMatrix {
    title: Font,
    big: Font,
    mid: Font,
    unit: Font,
    label: Font,
    badge: Font,
    foot: Font,
    display: HassDisplay,
}

impl EinkHassMatrix {
    pub fn new(display: HassDisplay, dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkHassFonts::default(), display, dims)
    }

    pub async fn new_async(display: HassDisplay, dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkHassFonts::default(), display, dims).await
    }

    pub fn with_fonts(paths: EinkHassFonts, display: HassDisplay, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            big: Font::load_ttf(&paths.body, scaled_px(120.0, h))?,
            mid: Font::load_ttf(&paths.body, scaled_px(56.0, h))?,
            unit: Font::load_ttf(&paths.body, scaled_px(36.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            badge: Font::load_ttf(&paths.body, scaled_px(26.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
            display,
        })
    }

    pub async fn with_fonts_async(
        paths: EinkHassFonts,
        display: HassDisplay,
        dims: (u32, u32),
    ) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, display, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the entity screen at `w × h`.
    pub fn frame(&self, data: &HassEntity, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let unit = data.unit.clone().unwrap_or_default();
        let label = fit_text(&self.title, &data.label.to_uppercase(), wi - 4 * m);
        let right = if unit.is_empty() { None } else { Some(unit.as_str()) };
        let content_top = header_band(&mut img, &self.title, &self.label, m, &label, right, fg);

        // Alarm = a filled badge (replaces the LED color flip).
        let mut body_top = content_top;
        if let Some(alarm) = &self.display.alarm_state {
            if data.state.eq_ignore_ascii_case(alarm) {
                let bx = cx - badge_width(&self.badge, "ALARM") / 2;
                badge(&mut img, &self.badge, bx, content_top, "ALARM", fg, true);
                body_top = content_top + self.badge.height() + m;
            }
        }

        // Mode with the same fallback the LED tile uses: graph/historical drop
        // to state when the entity isn't numeric or has no history yet.
        let numeric = data.is_numeric();
        let mode = match self.display.mode {
            HassDisplayMode::Graph | HassDisplayMode::Historical
                if !numeric || data.history.is_empty() =>
            {
                HassDisplayMode::State
            }
            other => other,
        };

        match mode {
            HassDisplayMode::State => {
                let hero_base = hi * 50 / 100 + self.big.ascent() / 2;
                if numeric {
                    big_value_centered(&mut img, &self.big, &self.unit, cx, hero_base, fg, data.state.trim(), &unit);
                } else {
                    center_text(&mut img, &self.big, cx, hero_base, fg, &data.state.to_uppercase());
                }
            }
            HassDisplayMode::Historical => {
                // Current value, then recent samples newest-first.
                let cur_base = body_top + self.mid.ascent() + m;
                big_value_centered(&mut img, &self.mid, &self.unit, cx, cur_base, fg, data.state.trim(), &unit);
                let now = Utc::now();
                let mut y = cur_base + self.mid.height() + m;
                for s in data.history.iter().rev().take(5) {
                    let age = (now - s.at).num_seconds().max(0);
                    let ago = if age < 120 { format!("{age}s ago") } else { format!("{}m ago", age / 60) };
                    let line = format!("{:.1} {}   {}", s.value, unit, ago);
                    center_text(&mut img, &self.label, cx, y + self.label.ascent(), fg, &line);
                    y += self.label.height() + m / 2;
                }
            }
            HassDisplayMode::Graph => {
                let cur_base = body_top + self.mid.ascent() + m;
                big_value_centered(&mut img, &self.mid, &self.unit, cx, cur_base, fg, data.state.trim(), &unit);
                let gx = m;
                let gy = cur_base + m;
                let gw = wi - 2 * m;
                let gh = hi - gy - m - self.foot.height();
                let series: Vec<f32> = data.history.iter().map(|s| s.value as f32).collect();
                sparkline(&mut img, gx, gy, gw, gh, &series, fg);
            }
        }

        let age = data.age_seconds(Utc::now());
        let updated = if age < 120 { format!("updated {age}s ago") } else { format!("updated {}m ago", age / 60) };
        footer(&mut img, &self.foot, fg, &updated);
        img
    }
}

#[async_trait]
impl EinkRenderer for EinkHassMatrix {
    type Data = HassEntity;

    fn id(&self) -> &'static str {
        "hass"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &HassEntity) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::hass::model::HassSample;

    fn repo_fonts() -> EinkHassFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkHassFonts { body: base.join("04B_03B_.TTF") }
    }

    fn entity(mode: HassDisplayMode, with_history: bool) -> HassEntity {
        let now = Utc::now();
        let history = if with_history {
            (0..12)
                .map(|i| HassSample {
                    at: now - chrono::Duration::minutes(12 - i),
                    value: 70.0 + i as f64,
                })
                .collect()
        } else {
            vec![]
        };
        let _ = mode;
        HassEntity {
            state: "72.4".into(),
            unit: Some("F".into()),
            label: "Kitchen Temp".into(),
            last_changed: now - chrono::Duration::seconds(8),
            history,
        }
    }

    fn disp(mode: HassDisplayMode, alarm: Option<&str>) -> HassDisplay {
        HassDisplay {
            alarm_state: alarm.map(|s| s.to_string()),
            mode,
            ..HassDisplay::default()
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkHassMatrix::with_fonts(repo_fonts(), disp(HassDisplayMode::Graph, None), (800, 480)).unwrap();
        let img = r.frame(&entity(HassDisplayMode::Graph, true), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated entity, got {lit} lit px");
    }

    #[test]
    fn alarm_differs_from_nominal() {
        let nominal = EinkHassMatrix::with_fonts(repo_fonts(), disp(HassDisplayMode::State, None), (800, 480)).unwrap();
        let alarm = EinkHassMatrix::with_fonts(repo_fonts(), disp(HassDisplayMode::State, Some("72.4")), (800, 480)).unwrap();
        let a = nominal.frame(&entity(HassDisplayMode::State, false), 800, 480);
        let b = alarm.frame(&entity(HassDisplayMode::State, false), 800, 480);
        assert_ne!(a.into_raw(), b.into_raw(), "alarm badge should change the frame");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkHassMatrix::with_fonts(repo_fonts(), disp(HassDisplayMode::State, None), (400, 300)).unwrap();
        let img = r.frame(&entity(HassDisplayMode::State, false), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
