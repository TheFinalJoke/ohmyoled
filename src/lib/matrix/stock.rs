//! Stock renderer — pure-Rust port of `src/python/ohmyoled/matrix/stock/stockmatrix.py`.
//!
//! Three sections on the 64×32 panel:
//!
//! ```text
//!   ┌──────────────────────────────┬────────┐
//!   │ SYM   $current_price         │   %    │ (top:    50×8,  rows 0..8)
//!   ├──────────────────────────────┤  +/-   │
//!   │ CP    $previous_close        │  Δ%    │ (middle: 50×24, rows 9..32)
//!   │  ↑    $high                  │  +/-$  │
//!   │  ↓    $low                   │        │
//!   └──────────────────────────────┴────────┘
//! ```
//!
//! The top row and the long-form symbol may scroll if they overflow their
//! allotted width; that mirrors the Python `check_size`/`-xpos` pattern.

use crate::api::stock::model::{Direction, StockQuote};
use crate::matrix::error::RenderError;
use crate::matrix::renderer::Renderer;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, RGBMatrix};
use std::path::PathBuf;
use std::time::Duration;

const PANEL_W: u32 = 64;
const PANEL_H: u32 = 32;

const SCROLL_FRAMES: u32 = 50;
const SCROLL_TICK: Duration = Duration::from_millis(50);
const FIRST_FRAME_DWELL: Duration = Duration::from_secs(3);
const DWELL: Duration = Duration::from_secs(30);

const BLUE: Color = Color { r: 0, g: 89, b: 255 };
const UP: Color = Color { r: 0, g: 255, b: 0 };
const DOWN: Color = Color { r: 255, g: 0, b: 0 };

/// Paths to the two fonts the renderer needs. Same defaults as `WeatherMatrix`.
#[derive(Debug, Clone)]
pub struct StockFonts {
    pub body: PathBuf,
    pub icon: PathBuf,
}

impl Default for StockFonts {
    fn default() -> Self {
        Self {
            body: "/usr/share/fonts/04B_03B_.TTF".into(),
            icon: "/usr/share/fonts/weathericons.ttf".into(),
        }
    }
}

pub struct StockMatrix {
    body_font: Font,
    icon_font: Font,
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
            icon_font: Font::load_ttf(&paths.icon, 11.0)?,
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
        // 50 frames * 50ms = 2.5s scroll + 30s dwell ≈ 32.5s.
        Duration::from_secs(33)
    }

    async fn render(&mut self, matrix: &mut RGBMatrix, data: &StockQuote) -> Result<(), RenderError> {
        matrix.clear();
        for xpos in 0..SCROLL_FRAMES {
            let img = self.draw_frame(data, xpos as i32);
            matrix.set_image(&img, 0, 0);
            tokio::time::sleep(if xpos == 0 { FIRST_FRAME_DWELL } else { SCROLL_TICK }).await;
        }
        let img = self.draw_frame(data, 0);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(DWELL).await;
        matrix.clear();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Drawing — pure functions on a fresh RgbImage so frames are unit-testable.
// ---------------------------------------------------------------------------

impl StockMatrix {
    /// Render one frame of the stock display at the given horizontal scroll offset.
    pub fn draw_frame(&self, data: &StockQuote, xpos: i32) -> RgbImage {
        let mut img = RgbImage::new(PANEL_W, PANEL_H);
        self.draw_top(&mut img, data, xpos);
        self.draw_middle(&mut img, data, xpos);
        self.draw_right(&mut img, data);
        img
    }

    /// Top strip: ticker symbol + current price.
    fn draw_top(&self, img: &mut RgbImage, data: &StockQuote, xpos: i32) {
        let font = &self.body_font;
        let baseline = top_to_baseline(1, font.ascent());

        // Symbol (left). Scrolls if it can't fit in the 12px slot.
        let symbol = data.symbol.to_uppercase();
        let sym_x = if estimated_width(&symbol, font) > 12 { 1 - xpos } else { 1 };
        draw_text(img, font, sym_x, baseline, BLUE, &symbol);

        // Current price (right). Scrolls when wider than 32px.
        let price_text = format!("${}", format_price(data.current));
        let color = direction_color(data.direction());
        let price_x = if estimated_width(&price_text, font) > 32 { 15 - xpos } else { 15 };
        draw_text(img, font, price_x, baseline, color, &price_text);
    }

    /// Middle column (cols 0..50, rows 9..32): three rows of label + value.
    fn draw_middle(&self, img: &mut RgbImage, data: &StockQuote, xpos: i32) {
        // Row 1: "CP" + previous close.
        let body = &self.body_font;
        let row1 = top_to_baseline(11, body.ascent());
        draw_text(img, body, 2, row1, Color::WHITE, "CP");
        draw_text(
            img,
            body,
            15,
            row1,
            Color::WHITE,
            &format!("${}", format_price(data.previous_close)),
        );

        // Row 2: green sunrise glyph (`high` indicator) + highest price.
        let icon = &self.icon_font;
        let row2_glyph = top_to_baseline(13, icon.ascent());
        draw_text(img, icon, 2, row2_glyph, UP, "\u{f058}");
        let row2_text = top_to_baseline(17, body.ascent());
        draw_text(
            img,
            body,
            15,
            row2_text,
            UP,
            &format!("${}", format_price(data.high)),
        );

        // Row 3: red sunset glyph (`low` indicator) + lowest price.
        let row3_glyph = top_to_baseline(19, icon.ascent());
        draw_text(img, icon, 2, row3_glyph, DOWN, "\u{f044}");
        let row3_text = top_to_baseline(23, body.ascent());
        draw_text(
            img,
            body,
            15,
            row3_text,
            DOWN,
            &format!("${}", format_price(data.low)),
        );

        // Touch xpos so the unused-variable lint stays quiet when no scroll happens.
        let _ = xpos;
    }

    /// Right strip (cols 50..64): "%", percent change, "+/-", dollar change.
    fn draw_right(&self, img: &mut RgbImage, data: &StockQuote) {
        let font = &self.body_font;
        let color = direction_color(data.direction());

        draw_text(img, font, 52, top_to_baseline(2, font.ascent()), UP, "%");
        draw_text(
            img,
            font,
            51,
            top_to_baseline(9, font.ascent()),
            color,
            &format_signed(data.percent_change()),
        );
        draw_text(img, font, 52, top_to_baseline(16, font.ascent()), UP, "+/-");
        draw_text(
            img,
            font,
            51,
            top_to_baseline(22, font.ascent()),
            color,
            &format_signed(data.dollar_change()),
        );
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers.
// ---------------------------------------------------------------------------

fn top_to_baseline(top_y: i32, ascent: i32) -> i32 {
    top_y + ascent
}

fn format_price(p: f64) -> String {
    format!("{p:.2}")
}

/// Two-decimal signed format, no leading "+" for negatives.
fn format_signed(v: f64) -> String {
    format!("{v:.2}")
}

fn direction_color(d: Direction) -> Color {
    match d {
        Direction::Up => UP,
        Direction::Down => DOWN,
        Direction::Flat => Color::WHITE,
    }
}

/// Rough text-width estimate (chars × per-glyph advance). The 04B_03B body
/// font is roughly 4px per character at 8pt, so this is good enough for the
/// "does this overflow?" check that drives scrolling.
fn estimated_width(text: &str, _font: &Font) -> i32 {
    (text.chars().count() as i32) * 4
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
            icon: repo.join("weathericons.ttf"),
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
    fn long_symbol_scrolls_off_left() {
        let m = StockMatrix::with_fonts(repo_fonts()).expect("fonts load");
        // Long symbol overflows the 12px slot so the renderer scrolls.
        let q = sample("BERKSHIREB", 150.0, 145.0);
        let img0 = m.draw_frame(&q, 0);
        let img_far = m.draw_frame(&q, 200);
        let count = |img: &RgbImage, y_range: std::ops::Range<u32>| {
            y_range
                .flat_map(|y| (0..15).map(move |x| img.get_pixel(x, y).0))
                .filter(|p| *p != [0, 0, 0])
                .count()
        };
        let row_full = count(&img0, 1..9);
        let row_scrolled = count(&img_far, 1..9);
        assert!(
            row_full > row_scrolled,
            "expected fewer lit pixels after scrolling: full={row_full} scrolled={row_scrolled}"
        );
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
}
