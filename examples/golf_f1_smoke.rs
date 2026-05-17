//! Live smoke: hit ESPN PGA + jolpica F1 + render one frame of each.
//!
//! Run: `cargo run --example golf_f1_smoke`

use oledlib::api::f1::{F1Collector, F1Source, JolpicaProvider};
use oledlib::api::golf::{EspnGolfProvider, GolfCollector, GolfSource, GolfTour};
use oledlib::api::Collector;
use oledlib::matrix::f1::{F1Fonts, F1Matrix};
use oledlib::matrix::golf::{GolfFonts, GolfMatrix};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");

    // ---- Golf ----
    let golf_collector =
        GolfCollector::new(GolfSource::Espn(EspnGolfProvider::new(GolfTour::Pga)));
    match golf_collector.poll().await {
        Ok(d) => {
            println!(
                "GOLF OK: tour={:?} event={:?} status={:?} leaders={}",
                d.tour,
                d.event_name,
                d.status,
                d.leaderboard.len()
            );
            for e in d.leaderboard.iter().take(3) {
                println!("  #{} {} {}", e.position, e.player_short, e.score);
            }
            let m = GolfMatrix::with_fonts(GolfFonts { body: repo.join("04B_03B_.TTF") }).unwrap();
            ascii(&m.draw_frame(&d, 0), "=== Golf frame ===");
        }
        Err(e) => println!("GOLF ERR: {e}"),
    }

    // ---- F1 ----
    let f1_collector = F1Collector::new(F1Source::Jolpica(JolpicaProvider::new()));
    match f1_collector.poll().await {
        Ok(d) => {
            println!(
                "F1 OK: season={} next={:?} drivers={}",
                d.season,
                d.next_race.as_ref().map(|r| &r.name),
                d.standings.len()
            );
            for s in d.standings.iter().take(3) {
                println!("  P{} {} {}pts", s.position, s.code, s.points);
            }
            let m = F1Matrix::with_fonts(F1Fonts { body: repo.join("04B_03B_.TTF") }).unwrap();
            ascii(&m.draw_frame(&d, 0), "=== F1 frame ===");
        }
        Err(e) => println!("F1 ERR: {e}"),
    }
}

fn ascii(img: &image::RgbImage, label: &str) {
    println!("\n{label}");
    for y in 0..img.height() {
        let mut row = String::new();
        for x in 0..img.width() {
            row.push(if img.get_pixel(x, y).0 == [0, 0, 0] { '.' } else { '#' });
        }
        println!("{row}");
    }
}
