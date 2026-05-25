//! Stock-chart renderer — header + autoscaled line graph that cycles
//! through the 1D / 1M / 1Y windows within one render call.
//!
//! ```text
//!  ┌─────────────────────────────────────────────────┐
//!  │ AAPL              $182.34  +1.24%           1D  │  header  (8 px)
//!  │       ___          ___    ▴                     │
//!  │     ╱╱   ╲╲     ╱╱╱   ╲╲╲╱                      │
//!  │   ╱╱      ╲╲╲╱╱╱        ╲                       │  graph    (24 px,
//!  │╱╱╱                       ╲╲___                  │   autoscaled to
//!  │                              ╲╲╲___             │   the active
//!  │ ──────────────────────────────────╲╲╲           │   period's range)
//!  └─────────────────────────────────────────────────┘
//! ```
//!
//! - **Header (top 8 px)**: ticker (white), current price (white),
//!   percent change (green / red / grey by [`Direction`]), and the
//!   active period badge (`1D` / `1M` / `1Y`) in the top-right.
//! - **Graph (bottom 24 px)**: line drawn between consecutive closes.
//!   The series is bucketed across the 62-px-wide plotting area so a
//!   ~78-sample intraday window and a ~52-sample yearly window both
//!   compress cleanly. Y-axis autoscales to that window's `low`/`high`
//!   — a flat 1D doesn't get dwarfed by a volatile 1Y. Color follows
//!   the window's own direction (`closes[0]` vs `closes[last]`).
//! - **Current-price marker**: a 2 px triangle at the right edge of
//!   the active window. Always white so the "where we are now" reads
//!   independent of the line color.
//! - **Period cycle**: 12 s render cycle ⇒ 4 s on each of 1D, 1M, 1Y,
//!   then return. Empty windows render a "NO DATA" badge in their
//!   slot instead of an empty graph.
//!
//! # Config
//!
//! The chart tile reuses the existing `stock` section — set
//! `chart: true` on any entry to add it alongside the live tile:
//!
//! ```yaml
//! stock:
//!   - run: true
//!     symbol: AAPL
//!     api: finnhub
//!     api_key: "..."
//!     chart: true        # adds the chart tile
//! ```
//!
//! # Data source
//!
//! `StockHistoryCollector` — Yahoo Finance `/v8/finance/chart` for
//! Finnhub-style equity tickers, CoinGecko `/coins/{id}/market_chart`
//! for crypto. 5 min refresh; three windows fetched per refresh.

use crate::api::stock::{Direction, HistorySeries, StockHistory};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use image::{Rgb, RgbImage};
use ohmyoled_matrix::graphics::{draw_line, draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

const HEADER_H: i32 = 8;
const GRAPH_TOP: i32 = HEADER_H;
const GRAPH_BOTTOM: i32 = PANEL_H as i32 - 1;
const GRAPH_LEFT: i32 = 1;
const GRAPH_RIGHT: i32 = PANEL_W as i32 - 2;

// Palette.
const HEADER_LABEL: Color = Color { r: 255, g: 255, b: 255 };
const PERIOD_BADGE: Color = Color { r: 200, g: 200, b: 200 };
const UP: Color = Color { r: 30, g: 220, b: 90 };
const DOWN: Color = Color { r: 240, g: 70, b: 70 };
const FLAT: Color = Color { r: 180, g: 180, b: 180 };
const MARKER: Color = Color { r: 255, g: 255, b: 255 };
const NO_DATA: Color = Color { r: 140, g: 140, b: 140 };

const FRAME_TICK: Duration = Duration::from_millis(50);
/// 14 s per period × 3 periods = 42 s total. Each period gets a long
/// enough dwell that the user can actually read the chart, the marquee
/// can run its two passes, and the eye can settle on the trend.
const CYCLE: Duration = Duration::from_secs(42);
/// Frames each period holds before the cycle advances. 14 s × 20 fps
/// = 280 frames per period.
const FRAMES_PER_PERIOD: u32 = 280;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Period {
    Day,
    Month,
    Year,
}

impl Period {
    fn label(self) -> &'static str {
        match self {
            Self::Day => "1D",
            Self::Month => "1M",
            Self::Year => "1Y",
        }
    }

    fn from_frame_idx(frame: u32) -> Self {
        match (frame / FRAMES_PER_PERIOD) % 3 {
            0 => Self::Day,
            1 => Self::Month,
            _ => Self::Year,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StockChartFonts {
    pub body: PathBuf,
}

impl Default for StockChartFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
        }
    }
}

pub struct StockChartMatrix {
    body_font: Font,
}

impl StockChartMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(StockChartFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(StockChartFonts::default()).await
    }

    pub fn with_fonts(paths: StockChartFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
        })
    }

    pub async fn with_fonts_async(paths: StockChartFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Render one frame for a chosen period — useful for tests and
    /// the default `frame()` helper. The render loop drives this with
    /// successive frame indexes to cycle through periods.
    pub fn frame(&self, data: &StockHistory) -> RgbImage {
        self.draw_frame(data, Period::Day, 0)
    }

    fn draw_frame(&self, data: &StockHistory, period: Period, scroll_phase: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let series = pick_series(data, period);
        self.draw_header(&mut img, data, period, series, scroll_phase);
        self.draw_graph(&mut img, series);
        img
    }

    fn draw_header(
        &self,
        img: &mut RgbImage,
        data: &StockHistory,
        period: Period,
        series: &HistorySeries,
        scroll_phase: u32,
    ) {
        let body = &self.body_font;
        let baseline = body.ascent();

        // Period badge — right-anchored, always visible.
        let label = period.label();
        let label_w = body.text_width(label);
        let label_x = PANEL_W as i32 - label_w - 1;
        draw_text(img, body, label_x, baseline, PERIOD_BADGE, label);

        // Per-window direction colors the percent label.
        let pct_color = match series.direction() {
            Direction::Up => UP,
            Direction::Down => DOWN,
            Direction::Flat => FLAT,
        };

        // Percent change — right-anchored just left of the badge,
        // also always visible. The two right-side cells together
        // anchor the eye while the symbol/price strip scrolls on the
        // left if it needs to.
        let pct_txt = if series.is_empty() {
            String::new()
        } else {
            format!("{:+.2}%", series.percent_change())
        };
        let pct_w = body.text_width(&pct_txt);
        let right_gap = 2;
        let pct_x = label_x - pct_w - right_gap;
        if !pct_txt.is_empty() {
            draw_text(img, body, pct_x, baseline, pct_color, &pct_txt);
        }

        // Everything left of the percent is the "symbol + price"
        // strip. Compose them with a 2-px space and let
        // `draw_scrolling_header` decide whether the strip fits
        // statically or needs to marquee.
        let price = data.current;
        let price_txt = if price >= 1000.0 {
            format!("${price:.0}")
        } else {
            format!("${price:.2}")
        };
        let strip = if series.is_empty() {
            data.symbol.clone()
        } else {
            format!("{}  {}", data.symbol, price_txt)
        };
        // The strip occupies x=0..(pct_x - gap). Reserve one extra
        // gap pixel so the strip never visually kisses the percent.
        let strip_w = pct_x - right_gap;
        if strip_w > 0 {
            draw_scrolling_header(
                img,
                body,
                &strip,
                0,
                baseline,
                strip_w,
                HEADER_LABEL,
                scroll_phase,
            );
        }
    }

    fn draw_graph(&self, img: &mut RgbImage, series: &HistorySeries) {
        // Empty window → centered "NO DATA" badge in the graph area
        // instead of a misleading flat line.
        if series.closes.len() < 2 {
            let body = &self.body_font;
            let badge = "NO DATA";
            let bw = body.text_width(badge);
            let bx = (PANEL_W as i32 - bw) / 2;
            let by = GRAPH_TOP + (GRAPH_BOTTOM - GRAPH_TOP) / 2 + 3;
            draw_text(img, body, bx, by, NO_DATA, badge);
            return;
        }

        let color = match series.direction() {
            Direction::Up => UP,
            Direction::Down => DOWN,
            Direction::Flat => FLAT,
        };

        let width_px = (GRAPH_RIGHT - GRAPH_LEFT + 1) as usize;
        let buckets = bucket_to_width(&series.closes, width_px);

        // Vertical scaling: clamp degenerate ranges so a flat series
        // draws a horizontal stripe rather than dividing by zero.
        let mut lo = series.low;
        let mut hi = series.high;
        if (hi - lo).abs() < f64::EPSILON {
            // Pad ±0.5% so the line lands mid-graph for flat windows.
            let pad = lo.abs().max(1.0) * 0.005;
            lo -= pad;
            hi += pad;
        }
        let to_y = |price: f64| -> i32 {
            let t = ((price - lo) / (hi - lo)).clamp(0.0, 1.0);
            // Invert: higher price = smaller y (closer to top of graph).
            GRAPH_BOTTOM - (t * (GRAPH_BOTTOM - GRAPH_TOP) as f64).round() as i32
        };

        // Draw connected segments. With buckets.len() == width_px,
        // each segment is one pixel wide.
        for i in 1..buckets.len() {
            let x0 = GRAPH_LEFT + (i - 1) as i32;
            let x1 = GRAPH_LEFT + i as i32;
            let y0 = to_y(buckets[i - 1]);
            let y1 = to_y(buckets[i]);
            draw_line(img, x0, y0, x1, y1, color);
        }

        // Current-price marker — a small upward triangle at the right
        // edge of the line. White so it pops against the colored line.
        let last_y = to_y(*buckets.last().unwrap());
        let mx = GRAPH_RIGHT;
        put(img, mx, last_y, MARKER);
        put(img, mx - 1, last_y, MARKER);
        put(img, mx, last_y - 1, MARKER);
    }
}

impl Default for StockChartMatrix {
    fn default() -> Self {
        Self::new().expect("default StockChartMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for StockChartMatrix {
    type Data = StockHistory;

    fn id(&self) -> &'static str {
        "stock_chart"
    }

    fn cycle_duration(&self) -> Duration {
        CYCLE
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &StockHistory) -> Result<(), RenderError> {
        matrix.clear();
        let frames = (CYCLE.as_millis() / FRAME_TICK.as_millis()) as u32;
        for frame_idx in 0..frames {
            let period = Period::from_frame_idx(frame_idx);
            // Scroll phase resets at every period change so the user
            // gets a fresh marquee in each period — keeps long
            // symbols from quietly settling forever after period 1.
            let scroll_phase = frame_idx % FRAMES_PER_PERIOD;
            matrix.set_image(&self.draw_frame(data, period, scroll_phase), 0, 0);
            tokio::time::sleep(FRAME_TICK).await;
        }
        matrix.clear();
        Ok(())
    }
}

fn pick_series(data: &StockHistory, period: Period) -> &HistorySeries {
    match period {
        Period::Day => &data.day,
        Period::Month => &data.month,
        Period::Year => &data.year,
    }
}

/// Down-sample / pass-through `closes` to exactly `width_px` buckets
/// by averaging each input slice. Returns an empty `Vec` when
/// `closes` is empty; otherwise the output length always equals
/// `width_px` so the renderer can plot one pixel per bucket without
/// branching on series length.
fn bucket_to_width(closes: &[f64], width_px: usize) -> Vec<f64> {
    if closes.is_empty() || width_px == 0 {
        return Vec::new();
    }
    if closes.len() >= width_px {
        // Compress: each bucket holds the mean of its input slice.
        // Using mean (vs. last) smooths very volatile intraday data
        // when a 78-sample 1D compresses into 62 pixels.
        (0..width_px)
            .map(|i| {
                let start = i * closes.len() / width_px;
                let end = ((i + 1) * closes.len() / width_px).max(start + 1);
                let slice = &closes[start..end.min(closes.len())];
                slice.iter().sum::<f64>() / slice.len() as f64
            })
            .collect()
    } else {
        // Stretch: linear interpolate so the line spans the full
        // graph width even for short windows. A 5-sample week looks
        // weird shoved into 5 leftmost pixels.
        (0..width_px)
            .map(|i| {
                let t = i as f64 / (width_px - 1) as f64;
                let pos = t * (closes.len() - 1) as f64;
                let a = pos.floor() as usize;
                let b = (a + 1).min(closes.len() - 1);
                let frac = pos - a as f64;
                closes[a] * (1.0 - frac) + closes[b] * frac
            })
            .collect()
    }
}

/// Pixel gap between repeats of the marquee — keeps the wrap-around
/// from reading as a joined string.
const MARQUEE_GAP_PX: i32 = 6;
/// How many full passes the strip scrolls before settling at the head
/// for the rest of the period. Two passes lets the eye verify the
/// content; settling stops the panel from moving forever.
const MARQUEE_PASSES: i32 = 2;

/// Draw `text` at `(dest_x, baseline)` with `color`, clipped to
/// `max_w` pixels. When the rendered text fits the slot it draws
/// once; otherwise it marquees left and settles after
/// `MARQUEE_PASSES`. Renders into a temp `RgbImage` and blits the
/// visible window so nothing bleeds into the percent / badge cells
/// to the right.
///
/// Inlined per the codebase convention (CLAUDE.md: "scroll loops are
/// inlined per renderer; don't extract a shared helper").
#[allow(clippy::too_many_arguments)]
fn draw_scrolling_header(
    img: &mut RgbImage,
    font: &Font,
    text: &str,
    dest_x: i32,
    baseline: i32,
    max_w: i32,
    color: Color,
    scroll_phase: u32,
) {
    let text_w = font.text_width(text);
    if text_w <= max_w {
        draw_text(img, font, dest_x, baseline, color, text);
        return;
    }
    let cycle = text_w + MARQUEE_GAP_PX;
    let total_scroll = cycle * MARQUEE_PASSES;
    let phase = scroll_phase as i32;
    let offset = if phase >= total_scroll {
        0
    } else {
        phase.rem_euclid(cycle)
    };

    // Render two consecutive copies so a window straddling the wrap
    // reads continuously.
    let temp_w = (cycle * 2) as u32;
    let row_h = HEADER_H as u32;
    let mut temp = RgbImage::new(temp_w, row_h);
    let temp_baseline = font.ascent();
    draw_text(&mut temp, font, 0, temp_baseline, color, text);
    draw_text(&mut temp, font, cycle, temp_baseline, color, text);

    // The header sits in rows 0..HEADER_H — convert the temp's
    // baseline-relative y back to row-top relative for the blit.
    let row_top = baseline - font.ascent();
    for x in 0..max_w {
        let src_x = offset + x;
        if src_x < 0 || src_x as u32 >= temp_w {
            continue;
        }
        for y in 0..row_h as i32 {
            let src = temp.get_pixel(src_x as u32, y as u32);
            if src.0 == [0, 0, 0] {
                continue;
            }
            let dx = dest_x + x;
            let dy = row_top + y;
            if dx >= 0 && dx < img.width() as i32 && dy >= 0 && dy < img.height() as i32 {
                img.put_pixel(dx as u32, dy as u32, *src);
            }
        }
    }
}

/// Bounds-checked single-pixel plot — drops out-of-range coords.
fn put(img: &mut RgbImage, x: i32, y: i32, c: Color) {
    if x >= 0 && x < img.width() as i32 && y >= 0 && y < img.height() as i32 {
        img.put_pixel(x as u32, y as u32, Rgb([c.r, c.g, c.b]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::stock::model::StockApiSource;
    use std::path::PathBuf;

    fn repo_fonts() -> StockChartFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        StockChartFonts {
            body: repo.join("04B_03B_.TTF"),
        }
    }

    fn ramp(start: f64, step: f64, n: usize) -> HistorySeries {
        HistorySeries::from_closes((0..n).map(|i| start + step * i as f64).collect())
    }

    fn sample(day: HistorySeries, month: HistorySeries, year: HistorySeries) -> StockHistory {
        StockHistory {
            api: StockApiSource::Yahoo,
            symbol: "AAPL".into(),
            current: day.closes.last().copied().unwrap_or(0.0),
            previous_close: day.closes.first().copied().unwrap_or(0.0),
            day,
            month,
            year,
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let m = StockChartMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.frame(&sample(ramp(180.0, 0.05, 78), ramp(170.0, 0.5, 22), ramp(140.0, 0.8, 52)));
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        // Header text + ~62 px of line + marker → comfortably > 80.
        assert!(lit > 80, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn rising_window_uses_up_color() {
        let m = StockChartMatrix::with_fonts(repo_fonts()).expect("fonts");
        let img = m.draw_frame(&sample(ramp(180.0, 0.05, 78), ramp(170.0, 0.5, 22), ramp(140.0, 0.8, 52)), Period::Day, 0);
        let up = [UP.r, UP.g, UP.b];
        let down = [DOWN.r, DOWN.g, DOWN.b];
        let up_count = img.pixels().filter(|p| p.0 == up).count();
        let down_count = img.pixels().filter(|p| p.0 == down).count();
        assert!(up_count > 10, "expected the rising line to be green");
        assert_eq!(down_count, 0, "no red pixels for a rising window");
    }

    #[test]
    fn falling_window_uses_down_color() {
        let m = StockChartMatrix::with_fonts(repo_fonts()).expect("fonts");
        let day = HistorySeries::from_closes((0..78).map(|i| 200.0 - i as f64 * 0.1).collect());
        let img = m.draw_frame(&sample(day, ramp(170.0, 0.5, 22), ramp(140.0, 0.8, 52)), Period::Day, 0);
        let down = [DOWN.r, DOWN.g, DOWN.b];
        let down_count = img.pixels().filter(|p| p.0 == down).count();
        assert!(down_count > 10, "expected the falling line to be red, got {down_count}");
    }

    #[test]
    fn period_badge_differs_per_period() {
        // 1D / 1M / 1Y badges should each draw distinct pixels in the
        // top-right header region — confirms the cycle actually swaps
        // the label, not just the data.
        let m = StockChartMatrix::with_fonts(repo_fonts()).expect("fonts");
        let h = sample(ramp(180.0, 0.05, 78), ramp(170.0, 0.5, 22), ramp(140.0, 0.8, 52));
        let d = m.draw_frame(&h, Period::Day, 0);
        let mo = m.draw_frame(&h, Period::Month, 0);
        let yr = m.draw_frame(&h, Period::Year, 0);
        // Top-right ~10 px wide, header height — the badge area.
        let badge_pixels = |img: &RgbImage| -> Vec<[u8; 3]> {
            ((PANEL_W - 11)..PANEL_W)
                .flat_map(|x| (0..HEADER_H as u32).map(move |y| (x, y)))
                .map(|(x, y)| img.get_pixel(x, y).0)
                .collect()
        };
        assert_ne!(badge_pixels(&d), badge_pixels(&mo));
        assert_ne!(badge_pixels(&mo), badge_pixels(&yr));
        assert_ne!(badge_pixels(&d), badge_pixels(&yr));
    }

    #[test]
    fn empty_window_shows_no_data_badge() {
        // An empty period draws the NO_DATA badge instead of a line.
        let m = StockChartMatrix::with_fonts(repo_fonts()).expect("fonts");
        let h = sample(
            HistorySeries::from_closes(vec![]),
            ramp(170.0, 0.5, 22),
            ramp(140.0, 0.8, 52),
        );
        let img = m.draw_frame(&h, Period::Day, 0);
        let needle = [NO_DATA.r, NO_DATA.g, NO_DATA.b];
        let n = img.pixels().filter(|p| p.0 == needle).count();
        assert!(n > 5, "NO DATA badge should light grey pixels, got {n}");
    }

    #[test]
    fn bucket_compresses_long_series_to_width() {
        let closes: Vec<f64> = (0..400).map(|i| i as f64).collect();
        let out = bucket_to_width(&closes, 62);
        assert_eq!(out.len(), 62);
        // Monotonic ramp in → monotonic ramp out.
        assert!(out.windows(2).all(|w| w[1] >= w[0]));
        // First bucket ≈ mean of first 400/62 samples ≈ 3, last ≈ mean of last ≈ 396.
        assert!(out[0] < 10.0);
        assert!(*out.last().unwrap() > 390.0);
    }

    #[test]
    fn bucket_stretches_short_series_to_width() {
        // 5 samples → 62 px, linearly interpolated.
        let closes = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let out = bucket_to_width(&closes, 62);
        assert_eq!(out.len(), 62);
        assert!((out[0] - 10.0).abs() < 1e-9);
        assert!((out.last().unwrap() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn flat_series_renders_horizontal_stripe() {
        // All-equal series shouldn't divide by zero in autoscaling —
        // it should draw flat at mid-graph.
        let m = StockChartMatrix::with_fonts(repo_fonts()).expect("fonts");
        let flat = HistorySeries::from_closes(vec![100.0; 22]);
        let img = m.draw_frame(
            &sample(flat.clone(), flat.clone(), flat),
            Period::Day,
            0,
        );
        // Flat direction → FLAT color (grey-ish).
        let needle = [FLAT.r, FLAT.g, FLAT.b];
        let n = img.pixels().filter(|p| p.0 == needle).count();
        assert!(n > 0, "flat series should still render a line");
    }

    #[test]
    fn long_symbol_marquees_in_header() {
        // A symbol wide enough to overflow the header strip should
        // make the strip's pixels shift between scroll phases.
        let m = StockChartMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut h = sample(
            ramp(180.0, 0.05, 78),
            ramp(170.0, 0.5, 22),
            ramp(140.0, 0.8, 52),
        );
        h.symbol = "VERYLONGTICKERNAME".into();
        let f0 = m.draw_frame(&h, Period::Day, 0);
        let f20 = m.draw_frame(&h, Period::Day, 20);
        // Just check the header band — graph + right-anchored percent
        // should remain identical between phases.
        let header_diff: usize = (0..PANEL_W)
            .flat_map(|x| (0..HEADER_H as u32).map(move |y| (x, y)))
            .filter(|&(x, y)| f0.get_pixel(x, y) != f20.get_pixel(x, y))
            .count();
        assert!(
            header_diff > 5,
            "long symbol should shift header pixels across scroll phases, got {header_diff}"
        );
    }

    #[test]
    fn marquee_settles_after_two_passes() {
        // Past total_scroll, the strip pixels stop changing — same
        // contract as the flights marquee.
        let m = StockChartMatrix::with_fonts(repo_fonts()).expect("fonts");
        let mut h = sample(
            ramp(180.0, 0.05, 78),
            ramp(170.0, 0.5, 22),
            ramp(140.0, 0.8, 52),
        );
        h.symbol = "VERYLONGTICKERNAME".into();
        let late_a = m.draw_frame(&h, Period::Day, 5000);
        let late_b = m.draw_frame(&h, Period::Day, 6000);
        let header_diff: usize = (0..PANEL_W)
            .flat_map(|x| (0..HEADER_H as u32).map(move |y| (x, y)))
            .filter(|&(x, y)| late_a.get_pixel(x, y) != late_b.get_pixel(x, y))
            .count();
        assert_eq!(header_diff, 0, "marquee should settle after passes complete");
    }

    #[test]
    fn period_rotates_with_frame_index() {
        // Confirm the rotation tracks FRAMES_PER_PERIOD without hard-
        // coding the value, so future dwell bumps don't require test
        // edits.
        let n = FRAMES_PER_PERIOD;
        assert_eq!(Period::from_frame_idx(0), Period::Day);
        assert_eq!(Period::from_frame_idx(n - 1), Period::Day);
        assert_eq!(Period::from_frame_idx(n), Period::Month);
        assert_eq!(Period::from_frame_idx(2 * n - 1), Period::Month);
        assert_eq!(Period::from_frame_idx(2 * n), Period::Year);
        assert_eq!(
            Period::from_frame_idx(3 * n),
            Period::Day,
            "wraps after one full cycle"
        );
    }
}
