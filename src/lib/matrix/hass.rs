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

use crate::api::hass::{HassEntity, HassSample};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use chrono::Utc;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

const LABEL: Color = Color { r: 200, g: 200, b: 200 };
const NOMINAL_DEFAULT: Color = Color { r: 120, g: 220, b: 120 };
const ALARM_DEFAULT: Color = Color { r: 255, g: 60, b: 60 };
const FOOTER: Color = Color { r: 130, g: 130, b: 130 };
const PAST_VALUE: Color = Color { r: 170, g: 170, b: 170 };
const PAST_AGE: Color = Color { r: 120, g: 120, b: 120 };
const GRAPH_LINE: Color = Color { r: 120, g: 220, b: 120 };
/// Marker ring drawn at the newest sparkline point — the "you are
/// here" indicator, and the implicit anchor for a future scrolling
/// graph (new samples would push in from this side).
const GRAPH_MARKER: Color = Color { r: 255, g: 255, b: 255 };

// Same hardcoded y-coord pattern as launch — guarantees a clear gap
// between the centered state row and the footer.
const LABEL_Y: i32 = 6;
const STATE_Y: i32 = 19;
const FOOTER_Y: i32 = 31;
/// Sparkline vertical range for Graph mode — sits below the top
/// header (which ends around y=6) and fills to the panel bottom.
const GRAPH_TOP: i32 = 10;
const GRAPH_BOTTOM: i32 = PANEL_H as i32 - 1;

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

/// Which of the three layouts to render. Picked per-entity in config.
/// `Graph` and `Historical` fall back to `State` when the entity is
/// non-numeric or its `history` is empty, so a misconfigured tile is
/// never blank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HassDisplayMode {
    /// Big current value, footer "updated Ns ago" (legacy default).
    #[default]
    State,
    /// Current value on top + a stacked list of recent past samples
    /// with their relative ages. Good for sensors where the trend
    /// reads at a glance but a graph is overkill.
    Historical,
    /// Current value on top + a sparkline of `history`. Best for
    /// fast-changing numeric sensors.
    Graph,
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
    pub mode: HassDisplayMode,
}

impl Default for HassDisplay {
    fn default() -> Self {
        Self {
            nominal_color: NOMINAL_DEFAULT,
            alarm_color: ALARM_DEFAULT,
            alarm_state: None,
            mode: HassDisplayMode::State,
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
        // Graph/Historical require a numeric series; fall back to
        // single-value State when the entity isn't numeric or has no
        // history yet (e.g., REST collector hasn't fetched it).
        let mode = match self.display.mode {
            HassDisplayMode::State => HassDisplayMode::State,
            HassDisplayMode::Graph | HassDisplayMode::Historical
                if !data.is_numeric() || data.history.is_empty() =>
            {
                HassDisplayMode::State
            }
            other => other,
        };
        match mode {
            HassDisplayMode::State => self.frame_state(data),
            HassDisplayMode::Historical => self.frame_historical(data),
            HassDisplayMode::Graph => self.frame_graph(data),
        }
    }

    /// Single-value layout: label / big centered state / "updated Ns ago".
    fn frame_state(&self, data: &HassEntity) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let body = &self.body_font;

        // Label — truncate with `…` if it'd overflow.
        let label = truncate_to_width(&data.label, PANEL_W as i32 - 2, body);
        draw_text(&mut img, body, 1, LABEL_Y, LABEL, &label);

        // State — center, color depends on alarm_state match.
        let state_text = format_state(data);
        let color = self.state_color(&data.state);
        let state_w = text_width_with_degree(body, &state_text);
        let state_x = ((PANEL_W as i32 - state_w) / 2).max(0);
        draw_text_with_degree(&mut img, body, state_x, STATE_Y, color, &state_text);

        // Footer — "updated Ns ago" / "since Nm ago" depending on age.
        let footer_text = footer_for_age(data.age_seconds(Utc::now()), data.is_numeric());
        let footer_w = body.text_width(&footer_text);
        let footer_x = ((PANEL_W as i32 - footer_w) / 2).max(0);
        draw_text(&mut img, body, footer_x, FOOTER_Y, FOOTER, &footer_text);

        img
    }

    /// Stacked-list layout: header (label + current) on top, then up
    /// to three past samples each rendered as "value  age".
    fn frame_historical(&self, data: &HassEntity) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let body = &self.body_font;
        self.draw_top_header(&mut img, data);

        // Past samples — most recent first, drop the newest (it's the
        // header). Three rows starting under the header, 7 px pitch.
        // Last row ends at y=29, leaving a 2 px bottom margin.
        let now = Utc::now();
        let mut past: Vec<&HassSample> = data.history.iter().rev().skip(1).collect();
        past.truncate(3);
        for (i, sample) in past.iter().enumerate() {
            let y_top = 10 + i as i32 * 7;
            let baseline = y_top + body.ascent();
            let value_text = format_history_value(sample.value, data.unit.as_deref());
            draw_text_with_degree(&mut img, body, 2, baseline, PAST_VALUE, &value_text);

            let age_secs = (now - sample.at).num_seconds().max(0) as u32;
            let age_text = compact_age(age_secs);
            let age_w = body.text_width(&age_text);
            let age_x = PANEL_W as i32 - age_w - 2;
            draw_text(&mut img, body, age_x, baseline, PAST_AGE, &age_text);
        }
        img
    }

    /// Sparkline layout: header (label + current) on top, then the
    /// full `history` plotted across the bottom rows.
    fn frame_graph(&self, data: &HassEntity) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        self.draw_top_header(&mut img, data);
        draw_sparkline(&mut img, &data.history, GRAPH_TOP, GRAPH_BOTTOM);
        img
    }

    /// Top-row header used by both Graph and Historical: label on the
    /// left in dim grey, current value (state + unit) on the right in
    /// the alarm/nominal color.
    fn draw_top_header(&self, img: &mut RgbImage, data: &HassEntity) {
        let body = &self.body_font;
        let baseline = LABEL_Y;
        let color = self.state_color(&data.state);
        let value_text = format_state(data);
        let value_w = text_width_with_degree(body, &value_text);
        let value_x = (PANEL_W as i32 - value_w - 2).max(0);
        // Label gets whatever pixels are left after the value claims
        // its right-aligned slot. Truncates with `…` if needed.
        let label_max_w = (value_x - 4).max(0);
        let label = truncate_to_width(&data.label, label_max_w, body);
        draw_text(img, body, 1, baseline, LABEL, &label);
        draw_text_with_degree(img, body, value_x, baseline, color, &value_text);
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

/// Compact a numeric history sample to fit the historical-row's
/// value cell. Drops cents past 999 to leave room for the age cell.
fn format_history_value(value: f64, unit: Option<&str>) -> String {
    let n = if value.abs() >= 999.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    };
    match unit {
        Some(u) if !u.is_empty() => format!("{n}{u}"),
        _ => n,
    }
}

/// Compact age for the historical-row's age cell. Sparser than the
/// `footer_for_age` style so it doesn't crowd the value: `45s`, `2m`,
/// `1h`, `3d`.
fn compact_age(age_secs: u32) -> String {
    if age_secs < 60 {
        format!("{age_secs}s")
    } else if age_secs < 3_600 {
        format!("{}m", age_secs / 60)
    } else if age_secs < 86_400 {
        format!("{}h", age_secs / 3_600)
    } else {
        format!("{}d", age_secs / 86_400)
    }
}

/// Draw a sparkline of `history` clipped to the rectangle
/// `(GRAPH_LEFT_INSET..PANEL_W - GRAPH_RIGHT_INSET) × (y_top..=y_bot)`.
/// Empty / single-sample series do nothing — the caller already
/// guarantees a non-empty series via the mode dispatch fallback.
fn draw_sparkline(img: &mut RgbImage, history: &[HassSample], y_top: i32, y_bot: i32) {
    const X_LEFT: i32 = 1;
    const X_RIGHT: i32 = PANEL_W as i32 - 2;
    if history.len() < 2 {
        return;
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for s in history {
        if s.value < lo {
            lo = s.value;
        }
        if s.value > hi {
            hi = s.value;
        }
    }
    // Flat series → draw a horizontal line mid-graph rather than
    // dividing by zero in the normalization step.
    if (hi - lo).abs() < f64::EPSILON {
        let pad = lo.abs().max(1.0) * 0.005;
        lo -= pad;
        hi += pad;
    }
    let n = history.len();
    let width_px = (X_RIGHT - X_LEFT) as f64;
    let height_px = (y_bot - y_top) as f64;
    let to_xy = |i: usize, v: f64| -> (i32, i32) {
        // Stretch the series across the available pixel range so a
        // short series doesn't bunch up on the left.
        let t = if n > 1 { i as f64 / (n - 1) as f64 } else { 0.0 };
        let x = X_LEFT + (t * width_px).round() as i32;
        let frac = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
        // Invert: higher value = smaller y (closer to top of graph).
        let y = y_bot - (frac * height_px).round() as i32;
        (x, y)
    };
    for i in 1..n {
        let (x0, y0) = to_xy(i - 1, history[i - 1].value);
        let (x1, y1) = to_xy(i, history[i].value);
        draw_line(img, x0, y0, x1, y1, GRAPH_LINE);
    }
    // Mark the newest sample with a 4-pixel ring (diamond) — reads as
    // a small circle at panel scale and tells the user where the
    // current value sits on the time series, i.e. where new samples
    // will appear as the data scrolls forward.
    let (mx, my) = to_xy(n - 1, history[n - 1].value);
    draw_marker_ring(img, mx, my);
}

fn draw_marker_ring(img: &mut RgbImage, cx: i32, cy: i32) {
    let pixel = image::Rgb([GRAPH_MARKER.r, GRAPH_MARKER.g, GRAPH_MARKER.b]);
    let w = img.width() as i32;
    let h = img.height() as i32;
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let px = cx + dx;
        let py = cy + dy;
        if px >= 0 && px < w && py >= 0 && py < h {
            img.put_pixel(px as u32, py as u32, pixel);
        }
    }
}

/// Width of the manually-drawn degree mark, plus a 1 px gap after it.
const DEGREE_GLYPH_W: i32 = 3;

/// Pixel-space width of `text` assuming each `°` is rendered as a
/// `DEGREE_GLYPH_W`-wide manual mark rather than the font's own (chunky
/// 4×3 hollow-rect) glyph. Use this to size / center any string the
/// renderer will hand to `draw_text_with_degree`.
fn text_width_with_degree(font: &Font, text: &str) -> i32 {
    let degrees = text.chars().filter(|c| *c == '°').count() as i32;
    if degrees == 0 {
        return font.text_width(text);
    }
    let font_degree_w = font.text_width("°");
    font.text_width(text) - (font_degree_w - DEGREE_GLYPH_W) * degrees
}

/// `draw_text` with one tweak: every `°` is replaced by a small 2×2
/// pixel mark anchored at the top of the line. The pixel font's
/// native degree glyph reads as a chunky filled rectangle at 8 pt
/// — the 2×2 mark mirrors the weather renderer's `draw_degree` and
/// reads cleanly as a degree symbol next to a temperature.
fn draw_text_with_degree(
    img: &mut RgbImage,
    font: &Font,
    x: i32,
    baseline: i32,
    color: Color,
    text: &str,
) -> i32 {
    let mut pen_x = x;
    let mut last_end = 0usize;
    for (i, ch) in text.char_indices() {
        if ch == '°' {
            let before = &text[last_end..i];
            if !before.is_empty() {
                pen_x = draw_text(img, font, pen_x, baseline, color, before);
            }
            draw_degree_mark(img, pen_x, baseline - font.ascent(), color);
            pen_x += DEGREE_GLYPH_W;
            last_end = i + ch.len_utf8();
        }
    }
    if last_end < text.len() {
        pen_x = draw_text(img, font, pen_x, baseline, color, &text[last_end..]);
    }
    pen_x
}

/// 2×2 filled mark anchored at `(x, line_top)` — the same shape
/// `weather.rs::draw_degree` uses.
fn draw_degree_mark(img: &mut RgbImage, x: i32, line_top: i32, color: Color) {
    let pixel = image::Rgb([color.r, color.g, color.b]);
    let w = img.width() as i32;
    let h = img.height() as i32;
    for dy in 0..2 {
        for dx in 0..2 {
            let px = x + dx;
            let py = line_top + dy;
            if px >= 0 && px < w && py >= 0 && py < h {
                img.put_pixel(px as u32, py as u32, pixel);
            }
        }
    }
}

/// Truncate `s` to a prefix that fits `max_px` plus an ellipsis. If it
/// already fits, return it unchanged.
fn truncate_to_width(s: &str, max_px: i32, font: &Font) -> String {
    if font.text_width(s) <= max_px {
        return s.to_string();
    }
    // ASCII "..." — the pixel font has no glyph for U+2026, which would render
    // as a .notdef circle.
    let ellipsis = "...";
    let ellipsis_w = font.text_width(ellipsis);
    let mut out = String::new();
    for ch in s.chars() {
        let mut probe = out.clone();
        probe.push(ch);
        if font.text_width(&probe) + ellipsis_w > max_px {
            break;
        }
        out = probe;
    }
    out.push_str(ellipsis);
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
            history: vec![],
        }
    }

    fn binary_sample() -> HassEntity {
        HassEntity {
            state: "open".into(),
            unit: None,
            label: "GARAGE".into(),
            last_changed: Utc::now() - ChDuration::minutes(14),
            history: vec![],
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
            history: vec![],
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

    fn numeric_with_history(samples: &[(i64, f64)]) -> HassEntity {
        let now = Utc::now();
        let history = samples
            .iter()
            .map(|(secs_ago, v)| HassSample {
                at: now - ChDuration::seconds(*secs_ago),
                value: *v,
            })
            .collect();
        HassEntity {
            state: "72.4".into(),
            unit: Some("°F".into()),
            label: "KITCHEN".into(),
            last_changed: now - ChDuration::seconds(12),
            history,
        }
    }

    fn matrix_with_mode(mode: HassDisplayMode) -> HassMatrix {
        HassMatrix::with_fonts(
            repo_fonts(),
            HassDisplay {
                mode,
                ..HassDisplay::default()
            },
        )
        .expect("fonts")
    }

    #[test]
    fn historical_renders_past_value_rows() {
        // With a numeric series + Historical mode, the panel should
        // light pixels in y=10..30 (past-sample rows below the header).
        let m = matrix_with_mode(HassDisplayMode::Historical);
        let e = numeric_with_history(&[
            (240, 70.0),
            (180, 70.5),
            (120, 71.2),
            (60, 71.8),
            (0, 72.4),
        ]);
        let img = m.frame(&e);
        let past_lit = (0..PANEL_W)
            .flat_map(|x| (10..30u32).map(move |y| (x, y)))
            .filter(|&(x, y)| img.get_pixel(x, y).0 != [0, 0, 0])
            .count();
        assert!(
            past_lit > 60,
            "historical mode should fill the past-sample rows; got {past_lit}"
        );
    }

    #[test]
    fn graph_renders_sparkline_in_graph_band() {
        // Graph mode lights GRAPH_LINE pixels in y=10..32, with the
        // ascending series climbing toward the top of the band.
        let m = matrix_with_mode(HassDisplayMode::Graph);
        let e = numeric_with_history(&(0..12).map(|i| (60 * (12 - i) as i64, 68.0 + i as f64 * 0.3)).collect::<Vec<_>>());
        let img = m.frame(&e);
        let needle = [GRAPH_LINE.r, GRAPH_LINE.g, GRAPH_LINE.b];
        let line_pixels = img.pixels().filter(|p| p.0 == needle).count();
        assert!(
            line_pixels > 30,
            "graph mode should draw a sparkline; got {line_pixels} line pixels"
        );
    }

    #[test]
    fn graph_draws_marker_ring_at_newest_sample() {
        // The newest sample on the right edge of the sparkline gets a
        // small ring marker in GRAPH_MARKER (white) so the user can
        // see "you are here" — the implicit anchor for new samples
        // scrolling in.
        let m = matrix_with_mode(HassDisplayMode::Graph);
        let e = numeric_with_history(
            &(0..12)
                .map(|i| (60 * (12 - i) as i64, 68.0 + i as f64 * 0.3))
                .collect::<Vec<_>>(),
        );
        let img = m.frame(&e);
        let marker = [GRAPH_MARKER.r, GRAPH_MARKER.g, GRAPH_MARKER.b];
        let mut found_marker_right_of_centre = false;
        // Marker pixels live somewhere in x > PANEL_W/2 (the newest
        // sample is on the right), inside the graph band.
        for x in (PANEL_W / 2)..PANEL_W {
            for y in (GRAPH_TOP as u32 - 1)..PANEL_H {
                if img.get_pixel(x, y).0 == marker {
                    found_marker_right_of_centre = true;
                    break;
                }
            }
        }
        assert!(
            found_marker_right_of_centre,
            "graph mode should paint a white marker ring at the newest sample"
        );
    }

    #[test]
    fn graph_falls_back_to_state_for_binary_entity() {
        // A binary entity (state="open") configured for Graph mode
        // can't produce a line — fall back to the State layout so the
        // tile isn't blank. Identify "state-mode" by the bottom-row
        // "since Ns ago" footer being present.
        let m = matrix_with_mode(HassDisplayMode::Graph);
        let img = m.frame(&binary_sample());
        // Footer lives in the y=25..32 strip.
        let footer_lit = (0..PANEL_W)
            .flat_map(|x| (25..32u32).map(move |y| (x, y)))
            .filter(|&(x, y)| img.get_pixel(x, y).0 != [0, 0, 0])
            .count();
        assert!(
            footer_lit > 5,
            "Graph mode on a binary entity should fall back to State (footer present)"
        );
    }

    #[test]
    fn graph_falls_back_to_state_when_history_empty() {
        let m = matrix_with_mode(HassDisplayMode::Graph);
        let img = m.frame(&numeric_sample()); // history is empty
        // Same footer-present heuristic.
        let footer_lit = (0..PANEL_W)
            .flat_map(|x| (25..32u32).map(move |y| (x, y)))
            .filter(|&(x, y)| img.get_pixel(x, y).0 != [0, 0, 0])
            .count();
        assert!(footer_lit > 5);
    }

    #[test]
    fn format_history_value_with_and_without_unit() {
        assert_eq!(format_history_value(72.4, Some("°F")), "72.4°F");
        assert_eq!(format_history_value(72.4, None), "72.4");
        assert_eq!(format_history_value(72.4, Some("")), "72.4");
        assert_eq!(format_history_value(1234.5, Some("kWh")), "1234kWh");
    }

    #[test]
    fn degree_renders_as_compact_2x2_mark() {
        // Regression: the font's native `°` glyph at 8 pt is a chunky
        // 4×3 hollow rectangle that reads as a small box, not a
        // degree symbol. The renderer must intercept it and draw a
        // 2×2 mark instead. Verify by counting state-mode lit pixels
        // with and without the unit: the difference should equal the
        // degree mark's pixel count + the "F" glyph, both small.
        let m = HassMatrix::with_fonts(repo_fonts(), HassDisplay::default()).expect("fonts");
        let mut with_unit = numeric_sample();
        with_unit.unit = Some("°F".into());
        let mut without_unit = numeric_sample();
        without_unit.unit = None;

        let lit = |e: &HassEntity| {
            m.frame(e).pixels().filter(|p| p.0 != [0, 0, 0]).count()
        };
        let degree_added = lit(&with_unit) as i32 - lit(&without_unit) as i32;
        // 2×2 degree mark = 4 px, "F" glyph ≈ 9-11 px in 04B_03B 8pt,
        // total ≈ 13-15. The legacy native-glyph path would have added
        // 8 (4×3 hollow ring) + ~10 (F) ≈ 18 — assert we're notably
        // under that.
        assert!(
            (10..17).contains(&degree_added),
            "expected ~13-15 added pixels for '°F'; got {degree_added}"
        );
    }

    #[test]
    fn compact_age_units() {
        assert_eq!(compact_age(12), "12s");
        assert_eq!(compact_age(120), "2m");
        assert_eq!(compact_age(3_900), "1h");
        assert_eq!(compact_age(2 * 86_400), "2d");
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
