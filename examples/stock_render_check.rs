//! ASCII-preview the stock display with synthetic data.
//!
//! Run: `cargo run --example stock_render_check`
//!
//! Renders the normal AAPL up/down case plus two overflow cases — a long
//! ticker and a price above `$999.99` — so you can eyeball the invisible-box
//! barrier and the marquee-twice scroll behavior.

use oledlib::api::stock::model::{StockApiSource, StockQuote};
use oledlib::matrix::stock::{StockFonts, StockMatrix};
use std::path::PathBuf;

fn ascii_preview(img: &image::RgbImage, label: &str) {
    println!("\n{label}");
    let lit: usize = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
    println!("lit pixels: {lit}");
    for y in 0..img.height() {
        let mut row = String::new();
        for x in 0..img.width() {
            let p = img.get_pixel(x, y);
            row.push(if p.0 == [0, 0, 0] { '.' } else { '#' });
        }
        println!("{row}");
    }
}

fn main() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let fonts = StockFonts {
        body: repo.join("04B_03B_.TTF"),
    };
    let m = StockMatrix::with_fonts(fonts).expect("font load");

    let up = StockQuote {
        api: StockApiSource::Finnhub,
        symbol: "AAPL".into(),
        name: "Apple Inc.".into(),
        open: 150.00,
        current: 153.42,
        high: 154.10,
        low: 149.85,
        previous_close: 150.20,
    };
    let down = StockQuote {
        current: 147.10,
        ..up.clone()
    };

    let img = m.draw_frame(&up, 0);
    ascii_preview(&img, "=== AAPL (UP day) ===");
    let img = m.draw_frame(&down, 0);
    ascii_preview(&img, "=== AAPL (DOWN day) ===");

    // Long ticker → symbol marquees, price stays put.
    let long_symbol = StockQuote {
        symbol: "BERKSHIREB".into(),
        ..up.clone()
    };
    let img = m.draw_frame(&long_symbol, 0);
    ascii_preview(&img, "=== Long symbol — frame 0 (settled) ===");
    let img = m.draw_frame(&long_symbol, 12);
    ascii_preview(&img, "=== Long symbol — frame 12 (mid-scroll) ===");

    // High price → price + H/L all marquee through their boxes.
    let pricey = StockQuote {
        symbol: "BRK".into(),
        current: 12345.67,
        high: 99999.99,
        low: 88888.88,
        previous_close: 12000.00,
        open: 12000.00,
        ..up.clone()
    };
    let img = m.draw_frame(&pricey, 0);
    ascii_preview(&img, "=== High price — frame 0 (settled) ===");
    let img = m.draw_frame(&pricey, 15);
    ascii_preview(&img, "=== High price — frame 15 (mid-scroll) ===");
}
