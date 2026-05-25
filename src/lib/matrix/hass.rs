//! Home Assistant renderer — generic 3-row tile for any HASS entity.
//!
//! ```text
//!  ┌── numeric (sensor) ────────┐    ┌── binary (alarm tripped) ───┐
//!  │ KITCHEN                    │    │ GARAGE                      │
//!  │                            │    │                             │
//!  │  72.4 °F                   │    │      OPEN                   │
//!  │                            │    │                             │
//!  │  updated 12s ago           │    │  since 14m ago              │
//!  └────────────────────────────┘    └─────────────────────────────┘
//! ```
//!
//! - **Label** (top, grey): the caller's override or HASS's
//!   `friendly_name`, marqueed if long.
//! - **State** (middle, centered): the raw HASS state string + optional
//!   unit. Numeric states render right-padded ("72.4 °F"); text states
//!   render plain centered ("OPEN", "unavailable").
//! - **Footer** (bottom, dim): "updated Ns ago" or "since Nm ago"
//!   computed live from `last_changed` at render time.
//!
//! Color rules:
//!   * If `alarm_state` is configured and matches the current state
//!     (case-insensitive), the state row renders in `alarm_color`
//!     (default red).
//!   * Otherwise the state row uses `nominal_color` (default sage green).
//!
//! # Config
//!
//! ```yaml
//! hass:
//!   - run: true
//!     base_url: http://homeassistant.local:8123
//!     token: YOUR_LONG_LIVED_TOKEN
//!     entity_id: sensor.kitchen_temp
//!     label: KITCHEN              # optional override
//!     alarm_state: null           # e.g. "open" or "on" to enable the flip
//! ```
//!
//! # Data source
//!
//! `HassCollector::from_rest` — bearer-auth GET against the local
//! HASS REST API. 30 s refresh.

use crate::api::hass::HassEntity;
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::Utc;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

const LABEL: Color = Color { r: 200, g: 200, b: 200 };
const NOMINAL_DEFAULT: Color = Color { r: 120, g: 220, b: 120 };
const ALARM_DEFAULT: Color = Color { r: 255, g: 60, b: 60 };
const FOOTER: Color = Color { r: 130, g: 130, b: 130 };

// Same hardcoded y-coord pattern as launch — guarantees a clear gap
// between the centered state row and the footer.
const LABEL_Y: i32 = 6;
const STATE_Y: i32 = 19;
const FOOTER_Y: i32 = 31;

/// Render loop tick — 1 fps is plenty for "Nms ago" to tick visibly
/// without burning CPU.
const TICK: Duration = Duration::from_secs(1);
const STATIC_FRAMES: u32 = 8;

#[derive(Debug, Clone)]
pub struct HassFonts {
    pub body: PathBuf,
}

impl Default for HassFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
        }
    }
}

/// Color configuration carried alongside the entity for renderer use.
/// Held separately from `HassEntity` because it doesn't come from the
/// HASS API — it's a per-config knob.
#[derive(Debug, Clone)]
pub struct HassDisplay {
    pub nominal_color: Color,
    pub alarm_color: Color,
    /// State string (case-insensitive) that flips the color from
    /// `nominal_color` to `alarm_color`. `None` disables the flip.
    pub alarm_state: Option<String>,
}

impl Default for HassDisplay {
    fn default() -> Self {
        Self {
            nominal_color: NOMINAL_DEFAULT,
            alarm_color: ALARM_DEFAULT,
            alarm_state: None,
        }
    }
}

pub struct HassMatrix {
    body_font: Font,
    display: HassDisplay,
}

impl HassMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(HassFonts::default(), HassDisplay::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(HassFonts::default(), HassDisplay::default()).await
    }

    pub fn with_fonts(paths: HassFonts, display: HassDisplay) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
            display,
        })
    }

    pub async fn with_fonts_async(paths: HassFonts, display: HassDisplay) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, display))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    pub fn frame(&self, data: &HassEntity) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let body = &self.body_font;

        // Label — truncate with `…` if it'd overflow.
        let label = truncate_to_width(&data.label, PANEL_W as i32 - 2, body);
        draw_text(&mut img, body, 1, LABEL_Y, LABEL, &label);

        // State — center, color depends on alarm_state match.
        let state_text = format_state(data);
        let color = self.state_color(&data.state);
        let state_w = body.text_width(&state_text);
        let state_x = ((PANEL_W as i32 - state_w) / 2).max(0);
        draw_text(&mut img, body, state_x, STATE_Y, color, &state_text);

        // Footer — "updated Ns ago" / "since Nm ago" depending on age.
        let footer_text = footer_for_age(data.age_seconds(Utc::now()), data.is_numeric());
        let footer_w = body.text_width(&footer_text);
        let footer_x = ((PANEL_W as i32 - footer_w) / 2).max(0);
        draw_text(&mut img, body, footer_x, FOOTER_Y, FOOTER, &footer_text);

        img
    }

    fn state_color(&self, state: &str) -> Color {
        match &self.display.alarm_state {
            Some(alarm) if state.eq_ignore_ascii_case(alarm) => self.display.alarm_color,
            _ => self.display.nominal_color,
        }
    }
}

impl Default for HassMatrix {
    fn default() -> Self {
        Self::new().expect("default HassMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for HassMatrix {
    type Data = HassEntity;

    fn id(&self) -> &'static str {
        "hass"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(STATIC_FRAMES as u64)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &HassEntity) -> Result<(), RenderError> {
        matrix.clear();
        // 1 fps for STATIC_FRAMES seconds — the footer ticks live.
        for _ in 0..STATIC_FRAMES {
            let img = self.frame(data);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(TICK).await;
        }
        matrix.clear();
        Ok(())
    }
}

/// Build the centered state row text. Numeric states get the unit
/// appended with a space ("72.4 °F"); text states ignore the unit
/// (a "door open °F" tile would be nonsense).
fn format_state(data: &HassEntity) -> String {
    if data.is_numeric() {
        match &data.unit {
            Some(u) if !u.is_empty() => format!("{} {}", data.state.trim(), u),
            _ => data.state.trim().to_string(),
        }
    } else {
        data.state.to_uppercase()
    }
}

/// Footer text for a given age. Numeric sensors use "updated Ns ago"
/// because the underlying value is implicitly always-current; binary
/// state changes use "since Nm ago" because the *event* is what's
/// interesting. Time units roll over at sensible thresholds.
fn footer_for_age(age_secs: u32, is_numeric: bool) -> String {
    let pretty_age = if age_secs < 60 {
        format!("{age_secs}s")
    } else if age_secs < 3_600 {
        format!("{}m", age_secs / 60)
    } else if age_secs < 86_400 {
        format!("{}h", age_secs / 3_600)
    } else {
        format!("{}d", age_secs / 86_400)
    };
    if is_numeric {
        format!("updated {pretty_age} ago")
    } else {
        format!("since {pretty_age} ago")
    }
}

/// Truncate `s` to a prefix that fits `max_px` plus an ellipsis. If it
/// already fits, return it unchanged.
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
    use chrono::{Duration as ChDuration, Utc};
    use std::path::PathBuf;

    fn repo_fonts() -> HassFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        HassFonts {
            body: repo.join("04B_03B_.TTF"),
        }
    }

    fn numeric_sample() -> HassEntity {
        HassEntity {
            state: "72.4".into(),
            unit: Some("°F".into()),
            label: "KITCHEN".into(),
            last_changed: Utc::now() - ChDuration::seconds(12),
        }
    }

    fn binary_sample() -> HassEntity {
        HassEntity {
            state: "open".into(),
            unit: None,
            label: "GARAGE".into(),
            last_changed: Utc::now() - ChDuration::minutes(14),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = HassMatrix::with_fonts(repo_fonts(), HassDisplay::default()).expect("fonts");
        let img = m.frame(&numeric_sample());
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn binary_state_uppercases_in_renderer() {
        // Even when the API gives "open", the rendered state should be
        // "OPEN" for visual prominence.
        let e = HassEntity {
            state: "open".into(),
            unit: Some("ignored-on-binary".into()),
            label: "x".into(),
            last_changed: Utc::now(),
        };
        let text = format_state(&e);
        assert_eq!(text, "OPEN");
    }

    #[test]
    fn numeric_state_appends_unit_with_space() {
        assert_eq!(format_state(&numeric_sample()), "72.4 °F");
    }

    #[test]
    fn numeric_state_omits_empty_unit_cleanly() {
        let mut e = numeric_sample();
        e.unit = None;
        assert_eq!(format_state(&e), "72.4");
    }

    #[test]
    fn alarm_state_flips_color() {
        // alarm_state="open" → state="open" → alarm color
        let m = HassMatrix::with_fonts(
            repo_fonts(),
            HassDisplay {
                alarm_state: Some("open".into()),
                ..HassDisplay::default()
            },
        )
        .unwrap();
        assert_eq!(m.state_color("open"), ALARM_DEFAULT);
        // Case-insensitive
        assert_eq!(m.state_color("OPEN"), ALARM_DEFAULT);
        // Anything else uses nominal
        assert_eq!(m.state_color("closed"), NOMINAL_DEFAULT);
    }

    #[test]
    fn no_alarm_state_always_uses_nominal() {
        let m = HassMatrix::with_fonts(repo_fonts(), HassDisplay::default()).unwrap();
        assert_eq!(m.state_color("on"), NOMINAL_DEFAULT);
        assert_eq!(m.state_color("off"), NOMINAL_DEFAULT);
        assert_eq!(m.state_color("unavailable"), NOMINAL_DEFAULT);
    }

    #[test]
    fn alarm_mode_renders_visibly_red_pixels() {
        let m = HassMatrix::with_fonts(
            repo_fonts(),
            HassDisplay {
                alarm_state: Some("open".into()),
                ..HassDisplay::default()
            },
        )
        .unwrap();
        let img = m.frame(&binary_sample());
        let red_pixels = img
            .pixels()
            .filter(|p| p.0[0] > 200 && p.0[1] < 100 && p.0[2] < 100)
            .count();
        assert!(
            red_pixels > 5,
            "alarm state should paint red pixels, got {red_pixels}"
        );
    }

    #[test]
    fn footer_for_numeric_uses_updated_phrasing() {
        assert_eq!(footer_for_age(12, true), "updated 12s ago");
        assert_eq!(footer_for_age(90, true), "updated 1m ago");
        assert_eq!(footer_for_age(3_900, true), "updated 1h ago");
    }

    #[test]
    fn footer_for_binary_uses_since_phrasing() {
        assert_eq!(footer_for_age(45, false), "since 45s ago");
        assert_eq!(footer_for_age(14 * 60, false), "since 14m ago");
        assert_eq!(footer_for_age(2 * 86_400, false), "since 2d ago");
    }

    #[test]
    fn long_label_truncates_with_ellipsis() {
        let m = HassMatrix::with_fonts(repo_fonts(), HassDisplay::default()).unwrap();
        let mut e = numeric_sample();
        e.label = "Living Room Couch Floor Lamp Brightness Sensor".into();
        // Shouldn't panic and should still light pixels.
        let img = m.frame(&e);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80);
        // The label row (around y=LABEL_Y) shouldn't paint past column ~62.
        let label_far_right_lit = (0..PANEL_W)
            .filter(|&x| x > 62 && img.get_pixel(x, LABEL_Y as u32).0 != [0, 0, 0])
            .count();
        assert_eq!(label_far_right_lit, 0, "truncated label must not reach the right edge");
    }

    /// Same hardcoded-spacing test as launch — guarantee a visible
    /// gap between the centered state row and the bottom footer.
    #[test]
    fn state_and_footer_have_clear_gap() {
        let m = HassMatrix::with_fonts(repo_fonts(), HassDisplay::default()).unwrap();
        let img = m.frame(&numeric_sample());
        // STATE_Y=19, so state glyphs land around rows 13-20.
        // FOOTER_Y=31, footer glyphs around rows 25-31.
        // Rows 21..=24 should be dark.
        for y in 21..=24u32 {
            let lit = (0..PANEL_W)
                .filter(|&x| img.get_pixel(x, y).0 != [0, 0, 0])
                .count();
            assert_eq!(lit, 0, "row {y} should be empty between state and footer");
        }
    }
}
