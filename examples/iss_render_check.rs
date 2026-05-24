//! ASCII-preview the ISS display in both modes with synthetic data.
//!
//! Run: `cargo run --example iss_render_check`

use oledlib::api::iss::IssState;
use oledlib::matrix::iss::{IssFonts, IssMatrix};
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
    let fonts = IssFonts {
        body: repo.join("04B_03B_.TTF"),
    };
    let m = IssMatrix::with_fonts(fonts).expect("font load");

    let distant = IssState {
        ground_distance_km: 1247,
        overhead: false,
        lat: 23.5,
        lon: -50.1,
        altitude_km: 421.0,
        velocity_kms: 7.66,
        visibility: "daylight".into(),
    };
    let img = m.frame(&distant);
    ascii_preview(&img, "=== ISS — distance mode (1,247 km away) ===");

    let far = IssState {
        ground_distance_km: 12_500,
        ..distant.clone()
    };
    let img = m.frame(&far);
    ascii_preview(&img, "=== ISS — distance mode (12,500 km away) ===");

    let overhead = IssState {
        ground_distance_km: 250,
        overhead: true,
        ..distant.clone()
    };
    let img = m.frame(&overhead);
    ascii_preview(&img, "=== ISS — OVERHEAD mode ===");
}
