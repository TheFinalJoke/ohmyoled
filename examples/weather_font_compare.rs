//! Render the weather temperature row using several candidate fonts so we can
//! pick one with more breathing room than `retro_computer.ttf`.
//!
//! Each candidate is drawn into a 64×16 strip (panel width, two text rows) with
//! the *exact* layout `WeatherMatrix::render_temp` uses today: `T:65F R:63F`
//! on row 0 and `H:74F L:56F` on row 1. The width of the rendered string and
//! the per-glyph advance are reported so we can tell which fonts pack the
//! digits too tightly.
//!
//! Run: `cargo run --example weather_font_compare`
//!
//! Outputs:
//! - ASCII preview to stdout (one block per font)
//! - PNGs at /tmp/weather_font_<name>.png for visual inspection.

use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::Color;
use std::path::PathBuf;

const STRIP_W: u32 = 64;
const STRIP_H: u32 = 20;

struct Candidate {
    label: &'static str,
    file: &'static str,
    px: f32,
}

const CANDIDATES: &[Candidate] = &[
    Candidate { label: "retro_computer 7pt (current)", file: "retro_computer.ttf", px: 7.0 },
    Candidate { label: "Press Start 2P 6pt",           file: "press2p.ttf",        px: 6.0 },
    Candidate { label: "Press Start 2P 7pt",           file: "press2p.ttf",        px: 7.0 },
    Candidate { label: "VCR Mono 8pt",                 file: "vcr_mono.ttf",       px: 8.0 },
    Candidate { label: "Pixeboy 8pt",                  file: "Pixeboy-z8XGD.ttf",  px: 8.0 },
    Candidate { label: "BMmini 8pt",                   file: "BMmini.TTF",         px: 8.0 },
    Candidate { label: "Minecart LCD 8pt",             file: "Minecart LCD.ttf",   px: 8.0 },
    Candidate { label: "Merchant Copy 9pt",            file: "Merchant Copy.ttf",  px: 9.0 },
    Candidate { label: "3270Condensed 9pt",            file: "3270Condensed-Regular.ttf", px: 9.0 },
    Candidate { label: "Pixeboy 9pt",                  file: "Pixeboy-z8XGD.ttf",  px: 9.0 },
    Candidate { label: "VGA 8pt",                      file: "VGA.ttf",            px: 8.0 },
    Candidate { label: "mini_pixel-7 8pt",             file: "mini_pixel-7.ttf",   px: 8.0 },
];

fn top_to_baseline(top_y: i32, ascent: i32) -> i32 {
    top_y + ascent
}

fn render_strip(font: &Font) -> (RgbImage, i32, i32) {
    let mut img = RgbImage::new(STRIP_W, STRIP_H);
    let baseline1 = top_to_baseline(0, font.ascent());
    let baseline2 = top_to_baseline(10, font.ascent());

    // Row 1: T:65F R:63F (matches current render_temp layout x=0/10/30/40).
    draw_text(&mut img, font, 0, baseline1, Color::WHITE, "T:");
    let end1 = draw_text(&mut img, font, 10, baseline1, Color::new(5, 223, 3), "65F");
    draw_text(&mut img, font, 30, baseline1, Color::WHITE, "R:");
    let end2 = draw_text(&mut img, font, 40, baseline1, Color::new(5, 223, 3), "63F");

    // Row 2: H:74F L:56F.
    draw_text(&mut img, font, 0, baseline2, Color::WHITE, "H:");
    draw_text(&mut img, font, 10, baseline2, Color::new(247, 157, 3), "74F");
    draw_text(&mut img, font, 30, baseline2, Color::WHITE, "L:");
    draw_text(&mut img, font, 40, baseline2, Color::new(0, 255, 255), "56F");

    (img, end1, end2)
}

fn measure(font: &Font, s: &str) -> i32 {
    let mut probe = RgbImage::new(128, 32);
    draw_text(&mut probe, font, 0, 16, Color::WHITE, s)
}

fn ascii(img: &RgbImage) {
    for y in 0..img.height() {
        let mut row = String::new();
        for x in 0..img.width() {
            let p = img.get_pixel(x, y).0;
            row.push(if p == [0, 0, 0] { '.' } else { '#' });
        }
        println!("  {row}");
    }
}

fn main() {
    let fonts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let out_dir = std::path::Path::new("/tmp");

    println!("Weather temperature font comparison");
    println!("Strip: {}x{} (top row T:65F R:63F  /  bottom row H:74F L:56F)\n", STRIP_W, STRIP_H);
    println!("{:<34} {:>6} {:>6} {:>7}", "font", "width", "100F", "advance");
    println!("{}", "-".repeat(60));

    for c in CANDIDATES {
        let path = fonts_dir.join(c.file);
        let font = match Font::load_ttf(&path, c.px) {
            Ok(f) => f,
            Err(e) => {
                println!("{:<34}  FAILED: {e}", c.label);
                continue;
            }
        };
        let triple_digit = measure(&font, "100F");
        let two_digit = measure(&font, "65F");
        let advance = two_digit; // px to render "65F"
        println!(
            "{:<34} {:>6} {:>6} {:>7}",
            c.label, two_digit, triple_digit, advance
        );
    }

    println!();
    for c in CANDIDATES {
        let path = fonts_dir.join(c.file);
        let font = match Font::load_ttf(&path, c.px) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let (img, _, _) = render_strip(&font);

        // ASCII preview.
        println!("\n=== {} ===", c.label);
        ascii(&img);

        // PNG snapshot.
        let safe_name: String = c
            .label
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let png = out_dir.join(format!("weather_font_{safe_name}.png"));
        if let Err(e) = img.save(&png) {
            println!("  (save failed: {e})");
        } else {
            println!("  -> {}", png.display());
        }
    }

    println!("\nLayout columns today: 'T:' x=0, value x=10, 'R:' x=30, value x=40.");
    println!("That gives 20 px for the 3-glyph value 'XXF' before it bumps into the next label.");
    println!("Any 'width' above ≥ 19 will visibly touch — pick a font with width ≤ 18 for 65F (or widen columns).");
}
