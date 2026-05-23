//! ASCII-preview the golf display with synthetic data.
//!
//! Run: `cargo run --example golf_render_check`
//!
//! Covers the normal PGA case plus a long-name marquee and a long-status
//! marquee, so you can eyeball the invisible-cell barriers and the
//! double-pass scroll behavior.

use oledlib::api::golf::{GolfData, GolfTour, LeaderboardEntry};
use oledlib::matrix::golf::{GolfFonts, GolfMatrix};
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

fn entry(pos: u32, name: &str, score: &str) -> LeaderboardEntry {
    LeaderboardEntry {
        position: pos,
        player_short: name.into(),
        score: score.into(),
    }
}

fn main() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let m = GolfMatrix::with_fonts(GolfFonts { body: repo.join("04B_03B_.TTF") }).expect("fonts");

    let normal = GolfData {
        tour: GolfTour::Pga,
        event_name: "Masters Tournament".into(),
        status: "In Progress".into(),
        leaderboard: vec![
            entry(1, "SCHEFFLER", "-12"),
            entry(2, "RAHM", "-8"),
            entry(3, "MORIKAWA", "-6"),
            entry(4, "SPIETH", "-4"),
            entry(5, "CANTLAY", "-3"),
        ],
    };
    ascii_preview(&m.draw_frame(&normal, 0), "=== PGA — typical (settled) ===");

    let long_names = GolfData {
        tour: GolfTour::Pga,
        event_name: "Masters Tournament".into(),
        status: "Final".into(),
        leaderboard: vec![
            entry(1, "DECHAMBEAU", "-15"),
            entry(2, "VANROOYEN", "-12"),
            entry(3, "STENSON", "-9"),
            entry(4, "OOSTHUIZEN", "-7"),
            entry(5, "MATSUYAMA", "-6"),
        ],
    };
    ascii_preview(
        &m.draw_frame(&long_names, 0),
        "=== Long names — frame 0 (settled) ===",
    );
    ascii_preview(
        &m.draw_frame(&long_names, 15),
        "=== Long names — frame 15 (mid-scroll) ===",
    );

    let long_status = GolfData {
        tour: GolfTour::Lpga,
        event_name: "ANA Inspiration".into(),
        status: "Round 4 In Progress".into(),
        leaderboard: vec![
            entry(1, "KO", "-8"),
            entry(2, "PARK", "-6"),
            entry(3, "THITIKUL", "-5"),
            entry(4, "SAGSTROM", "-4"),
            entry(5, "BOUTIER", "-3"),
        ],
    };
    ascii_preview(
        &m.draw_frame(&long_status, 0),
        "=== Long status + LPGA tour — frame 0 ===",
    );
    ascii_preview(
        &m.draw_frame(&long_status, 18),
        "=== Long status + LPGA tour — frame 18 (mid-scroll) ===",
    );
}
