//! Stock renderer — single-screen 3-row layout on the 64×32 panel.
//!
//! ```text
//!   ┌────────────────┊────────────────┐
//!   │ AAPL           ┊        $153.42 │ row 0–9     ← left box | gutter | right box
//!   ├────────────────┊────────────────┤
//!   │ +3.22          ┊         +2.14% │ row 10–19
//!   ├────────────────┊────────────────┤
//!   │ H 154.10       ┊      L 149.85  │ row 20–31
//!   └────────────────┴────────────────┘
//! ```
//!
//! Each row carries one piece of information so nothing overlaps. The panel is
//! split into two **invisible** 30-px boxes separated by a 2-px gutter — no
//! line is drawn, but every cell is clipped to its box so the two halves
//! cannot bleed into each other no matter how long the text gets.
//!
//! When a cell's text would overflow its box (e.g. a 10-char ticker, a price
//! above `$999.99`, or a wide H/L value), that cell marquee-scrolls **twice**
//! within the cycle before settling back to its start position.
//!
//! Colors: symbol blue; current price + change line green (up) / red (down) /
//! white (flat); high green; low red.
//!
//! # Config
//!
//! Stock is a list — one entry per symbol you want on rotation.
//!
//! ```yaml
//! stock:
//!   - run: true
//!     api: finnhub               # only finnhub supported today
//!     api_key: YOUR_FINNHUB_KEY
//!     symbol: AAPL
//!   - run: true
//!     api: finnhub
//!     api_key: YOUR_FINNHUB_KEY
//!     symbol: MSFT
//! ```
//!
//! # Data source
//!
//! `StockCollector::from_finnhub(cfg)` — uses the `finnhub` crate for the
//! `/quote` endpoint. Refresh interval: 30s. Get a free key at
//! <https://finnhub.io>.

use crate::api::stock::model::{Direction, StockQuote};
use crate::matrix::cells::{draw_cell, draw_pieces_in_box, overflow_period, Align, Piece};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::Font;
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

// --- Invisible row layout ---------------------------------------------------
// Two 30-px boxes with a 2-px gutter (the "barrier") and 1-px outer padding.
// 1 + 30 + 2 + 30 + 1 = 64.
const LEFT_BOX_X: i32 = 1;
const BOX_W: i32 = 30;
const RIGHT_BOX_X: i32 = 33;

// Gap (px) between the value and its "H"/"L" label. The font's right-side
// bearing on the letter already gives visible separation; an extra gap pushes
// the unit past the 30-px box.
const LABEL_GAP: i32 = 0;

const SCROLL_TICK: Duration = Duration::from_millis(50);
const FIRST_FRAME_DWELL: Duration = Duration::from_secs(2);
const DWELL: Duration = Duration::from_secs(28);

const BLUE: Color = Color { r: 0, g: 89, b: 255 };
const UP: Color = Color { r: 0, g: 255, b: 0 };
const DOWN: Color = Color { r: 255, g: 0, b: 0 };

/// Path to the body font (the renderer no longer uses icon glyphs — the +/-
/// sign and the color encode direction).
#[derive(Debug, Clone)]
pub struct StockFonts {
    pub body: PathBuf,
}

impl Default for StockFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
        }
    }
}

pub struct StockMatrix {
    body_font: Font,
}

impl StockMatrix {
    pub fn new() -> Result<Self, String> {
        Self::with_fonts(StockFonts::default())
    }

    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(StockFonts::default()).await
    }

    pub fn with_fonts(paths: StockFonts) -> Result<Self, String> {
        Ok(Self {
            body_font: Font::load_ttf(&paths.body, 8.0)?,
        })
    }

    pub async fn with_fonts_async(paths: StockFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }
}

impl Default for StockMatrix {
    fn default() -> Self {
        Self::new().expect("default StockMatrix font load failed")
    }
}

#[async_trait]
impl Renderer for StockMatrix {
    type Data = StockQuote;

    fn id(&self) -> &'static str {
        "stock"
    }

    fn cycle_duration(&self) -> Duration {
        // Steady state (no overflow): ~30s. With overflow, scroll phase adds
        // 2 × longest period × 50ms (typically a few seconds).
        Duration::from_secs(33)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &StockQuote) -> Result<(), RenderError> {
        matrix.clear();

        // Settle on frame 0 first so non-overflowing cells appear immediately.
        let img = self.draw_frame(data, 0);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(FIRST_FRAME_DWELL).await;

        // If anything overflows, marquee-scroll twice; otherwise skip straight to dwell.
        let scroll_total = self.scroll_total_frames(data);
        if scroll_total > 0 {
            for f in 1..=scroll_total {
                let img = self.draw_frame(data, f);
                matrix.set_image(&img, 0, 0);
                tokio::time::sleep(SCROLL_TICK).await;
            }
            let img = self.draw_frame(data, 0);
            matrix.set_image(&img, 0, 0);
        }

        tokio::time::sleep(DWELL).await;
        matrix.clear();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Drawing — pure functions on a fresh RgbImage so frames are unit-testable.
// ---------------------------------------------------------------------------

impl StockMatrix {
    /// Render one frame of the stock display at the given scroll frame index.
    /// Cells that fit their box stay static; cells that overflow marquee in
    /// step with `frame`.
    pub fn draw_frame(&self, data: &StockQuote, frame: u32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        let dir_color = direction_color(data.direction());
        self.draw_row_price(&mut img, data, frame, dir_color);
        self.draw_row_change(&mut img, data, frame, dir_color);
        self.draw_row_range(&mut img, data, frame);
        img
    }

    /// Row 1 (y=0..9): ticker symbol on the left, current price right-anchored.
    fn draw_row_price(&self, img: &mut RgbImage, data: &StockQuote, frame: u32, dir_color: Color) {
        let font = &self.body_font;
        let baseline = top_to_baseline(1, font.ascent());
        let symbol = data.symbol.to_uppercase();
        let price_text = format!("${}", format_price(data.current));

        draw_cell(img, font, LEFT_BOX_X, BOX_W, baseline, Align::Left, BLUE, &symbol, frame);
        draw_cell(img, font, RIGHT_BOX_X, BOX_W, baseline, Align::Right, dir_color, &price_text, frame);
    }

    /// Row 2 (y=10..19): signed dollar change (left) and bare percent change
    /// (right). Both colored by direction.
    fn draw_row_change(&self, img: &mut RgbImage, data: &StockQuote, frame: u32, dir_color: Color) {
        let font = &self.body_font;
        let baseline = top_to_baseline(12, font.ascent());
        let dollar = format_signed_plus(data.dollar_change());
        let percent = format!("{}%", format_signed_plus(data.percent_change()));

        draw_cell(img, font, LEFT_BOX_X, BOX_W, baseline, Align::Left, dir_color, &dollar, frame);
        draw_cell(img, font, RIGHT_BOX_X, BOX_W, baseline, Align::Right, dir_color, &percent, frame);
    }

    /// Row 3 (y=20..31): day range — "H <high>" left (white label + green
    /// value), "L <low>" right (white label + red value). Each side scrolls as
    /// a single unit so the colored value never separates from its label.
    fn draw_row_range(&self, img: &mut RgbImage, data: &StockQuote, frame: u32) {
        let font = &self.body_font;
        let baseline = top_to_baseline(23, font.ascent());
        let high = format_price(data.high);
        let low = format_price(data.low);

        draw_pieces_in_box(
            img,
            font,
            LEFT_BOX_X,
            BOX_W,
            baseline,
            Align::Left,
            &[
                Piece { text: "H", color: Color::WHITE },
                Piece { text: &high, color: UP },
            ],
            LABEL_GAP,
            frame,
        );
        draw_pieces_in_box(
            img,
            font,
            RIGHT_BOX_X,
            BOX_W,
            baseline,
            Align::Right,
            &[
                Piece { text: "L", color: Color::WHITE },
                Piece { text: &low, color: DOWN },
            ],
            LABEL_GAP,
            frame,
        );
    }

    /// Total scroll frames needed for two marquee passes of the longest
    /// overflowing cell. Returns 0 when nothing overflows so we can skip the
    /// scroll phase entirely.
    fn scroll_total_frames(&self, data: &StockQuote) -> u32 {
        let font = &self.body_font;
        let symbol = data.symbol.to_uppercase();
        let price_text = format!("${}", format_price(data.current));
        let dollar = format_signed_plus(data.dollar_change());
        let percent = format!("{}%", format_signed_plus(data.percent_change()));
        let high = format_price(data.high);
        let low = format_price(data.low);

        let high_unit = font.text_width("H") + LABEL_GAP + font.text_width(&high);
        let low_unit = font.text_width("L") + LABEL_GAP + font.text_width(&low);

        let cell_widths = [
            font.text_width(&symbol),
            font.text_width(&price_text),
            font.text_width(&dollar),
            font.text_width(&percent),
            high_unit,
            low_unit,
        ];

        let max_period = cell_widths
            .iter()
            .filter_map(|w| overflow_period(*w, BOX_W))
            .max()
            .unwrap_or(0);

        (max_period as u32).saturating_mul(2)
    }
}

// ---------------------------------------------------------------------------
// Formatting + low-level draw helpers.
// ---------------------------------------------------------------------------

fn top_to_baseline(top_y: i32, ascent: i32) -> i32 {
    top_y + ascent
}

fn format_price(p: f64) -> String {
    format!("{p:.2}")
}

/// Two-decimal signed format with an explicit `+` for non-negatives. `0.00`
/// renders as `+0.00`, which keeps the change row visually stable across ticks.
fn format_signed_plus(v: f64) -> String {
    format!("{v:+.2}")
}

fn direction_color(d: Direction) -> Color {
    match d {
        Direction::Up => UP,
        Direction::Down => DOWN,
        Direction::Flat => Color::WHITE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::stock::model::StockApiSource;
    use std::path::PathBuf;

    fn repo_fonts() -> StockFonts {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
        StockFonts {
            body: repo.join("04B_03B_.TTF"),
        }
    }

    fn sample(symbol: &str, current: f64, prev: f64) -> StockQuote {
        StockQuote {
            api: StockApiSource::Finnhub,
            symbol: symbol.into(),
            name: "Test Co".into(),
            open: prev,
            current,
            high: current + 5.0,
            low: current - 5.0,
            previous_close: prev,
        }
    }

    #[test]
    fn frame_dimensions_and_lit_pixels() {
        let m = StockMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let img = m.draw_frame(&sample("AAPL", 150.0, 145.0), 0);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80, "expected substantial lit pixels, got {lit}");
    }

    #[test]
    fn short_symbol_does_not_trigger_scrolling() {
        let m = StockMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let q = sample("AAPL", 150.0, 145.0);
        assert_eq!(m.scroll_total_frames(&q), 0);
    }

    #[test]
    fn long_symbol_triggers_double_pass_scroll() {
        let m = StockMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let q = sample("BERKSHIREB", 150.0, 145.0);
        let frames = m.scroll_total_frames(&q);
        // 10 chars * 4 = 40 px symbol > 30 px box → period 48, twice = 96 frames.
        assert!(frames >= 60, "expected ≥60 scroll frames, got {frames}");
        let period = frames / 2;
        // After exactly one period the marquee returns to its start position.
        let img0 = m.draw_frame(&q, 0);
        let img_p = m.draw_frame(&q, period);
        assert_eq!(
            pixels_in_row(&img0, 0..9),
            pixels_in_row(&img_p, 0..9),
            "symbol row should match its start after one full marquee pass"
        );
        // Mid-pass the symbol row has shifted.
        let img_mid = m.draw_frame(&q, period / 2);
        assert_ne!(
            pixels_in_row(&img0, 0..9),
            pixels_in_row(&img_mid, 0..9),
            "symbol row should differ at mid-pass"
        );
    }

    #[test]
    fn gutter_stays_dark_even_when_cells_overflow() {
        // Wide price ($12345.67) and long symbol both overflow simultaneously.
        // The 2-px gutter at x=31..32 must remain unlit on every row, on every frame.
        let m = StockMatrix::with_fonts(repo_fonts()).expect("fonts load");
        let q = StockQuote {
            api: StockApiSource::Finnhub,
            symbol: "BERKSHIREB".into(),
            name: "Test Co".into(),
            open: 12000.0,
            current: 12345.67,
            high: 99999.99,
            low: 88888.88,
            previous_close: 12000.0,
        };
        for frame in [0u32, 3, 7, 15, 23, 40] {
            let img = m.draw_frame(&q, frame);
            for x in 31..33 {
                for y in 0..PANEL_H {
                    assert_eq!(
                        img.get_pixel(x, y).0,
                        [0, 0, 0],
                        "gutter pixel ({x},{y}) lit at frame {frame}"
                    );
                }
            }
        }
    }

    #[test]
    fn high_price_triggers_marquee() {
        let m = StockMatrix::with_fonts(repo_fonts()).expect("fonts load");
        // $12345.67 is 9 chars * 4 = 36 px > 30 px box.
        let q = sample("BRK", 12345.67, 12000.0);
        assert!(m.scroll_total_frames(&q) > 0);
    }

    #[test]
    fn format_price_two_decimals() {
        assert_eq!(format_price(123.456), "123.46");
        assert_eq!(format_price(100.0), "100.00");
    }

    #[test]
    fn direction_picks_green_for_up_red_for_down() {
        assert_eq!(direction_color(Direction::Up), UP);
        assert_eq!(direction_color(Direction::Down), DOWN);
        assert_eq!(direction_color(Direction::Flat), Color::WHITE);
    }

    fn pixels_in_row(img: &RgbImage, y_range: std::ops::Range<u32>) -> Vec<[u8; 3]> {
        y_range
            .flat_map(|y| (0..img.width()).map(move |x| img.get_pixel(x, y).0))
            .collect()
    }
}
