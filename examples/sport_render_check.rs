//! ASCII-preview the sport display with synthetic data.
//!
//! Run: `cargo run --example sport_render_check`

use chrono::{Local, TimeZone};
use oledlib::api::sport::model::{
    GameStatus, HomeOrAway, NextGame, SportApiSource, SportData, SportKind, StandingsEntry,
    TeamSide,
};
use oledlib::matrix::sport::{SportFonts, SportMatrix};
use std::path::PathBuf;

fn ascii(img: &image::RgbImage, label: &str) {
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
    let fonts = SportFonts {
        body: repo.join("04B_03B_.TTF"),
        big: repo.join("04b24.otf"),
    };
    let m = SportMatrix::with_fonts(fonts).expect("font load");

    let data = SportData {
        api: SportApiSource::Espn,
        sport: SportKind::Basketball,
        team_name: "Boston Celtics".into(),
        record: "56-26".into(),
        next_game: Some(NextGame {
            start: Local.with_ymd_and_hms(2026, 5, 2, 19, 30, 0).unwrap(),
            status: GameStatus::Final,
            home: TeamSide {
                name: "Boston Celtics".into(),
                abbreviation: "BOS".into(),
                logo_url: None,
                score: Some(108),
            },
            away: TeamSide {
                name: "Philadelphia 76ers".into(),
                abbreviation: "PHI".into(),
                logo_url: None,
                score: Some(100),
            },
            our_side: HomeOrAway::Home,
            playoff_note: None,
        }),
        standings: (1..=5)
            .map(|i| StandingsEntry {
                position: i,
                team_name: format!("Team{i}"),
            })
            .collect(),
    };
    ascii(&m.draw_frame(&data, 0), "=== Final, scroll xpos=0 ===");
    ascii(&m.draw_frame(&data, 40), "=== Final, scroll xpos=40 ===");

    let mut sched = data.clone();
    if let Some(ng) = sched.next_game.as_mut() {
        ng.status = GameStatus::Scheduled;
        ng.home.score = None;
        ng.away.score = None;
    }
    ascii(&m.draw_frame(&sched, 0), "=== Scheduled, scroll xpos=0 ===");

    let offseason = SportData {
        next_game: None,
        standings: vec![],
        ..data
    };
    ascii(
        &m.draw_offseason(&offseason),
        "=== Offseason placeholder ===",
    );
}
