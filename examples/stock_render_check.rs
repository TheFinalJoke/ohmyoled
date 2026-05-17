//! ASCII-preview the stock display with synthetic data.
//!
//! Run: `PYO3_PYTHON=/usr/bin/python3.11 cargo run --example stock_render_check`

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
        icon: repo.join("weathericons.ttf"),
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
}
