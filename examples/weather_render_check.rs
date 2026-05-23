//! ASCII-preview the two weather screens with synthetic data.
//!
//! Run: `cargo run --example weather_render_check`
//!
//! Covers the normal SF case plus an extreme-temperature scenario (100°F /
//! -12°F / 100% humidity) so the marquee and invisible-cell barriers can be
//! inspected. Also prints which columns are gutters and verifies they stay
//! dark across several scroll frames.

use chrono::{Duration, Local, TimeZone};
use oledlib::api::weather::model::{
    CurrentWeather, DayForecast, Weather, WeatherApiSource,
};
use oledlib::matrix::weather::{WeatherFonts, WeatherMatrix};
use std::path::PathBuf;

const ROW1_GUTTER_X: u32 = 24;
const ROW2_GUTTER_X: [u32; 2] = [30, 31];

fn ascii_preview(img: &image::RgbImage, label: &str) {
    println!("\n{label}");
    let lit: usize = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
    println!("lit pixels: {lit}");
    // Header showing the gutter columns.
    print!("        ");
    for x in 0..img.width() {
        if x == ROW1_GUTTER_X || ROW2_GUTTER_X.contains(&x) {
            print!("|");
        } else if x % 10 == 0 {
            print!("{}", (x / 10) % 10);
        } else {
            print!(" ");
        }
    }
    println!();
    for y in 0..img.height() {
        let band = match y {
            8..=15 => "row1   ",
            18..=25 => "row2   ",
            _ => "       ",
        };
        let mut row = String::new();
        for x in 0..img.width() {
            let p = img.get_pixel(x, y);
            row.push(if p.0 == [0, 0, 0] { '.' } else { '#' });
        }
        println!("{band:>7} {row}");
    }
}

fn check_gutters(m: &WeatherMatrix, data: &Weather, screen: u8) -> bool {
    let mut ok = true;
    for frame in [0i32, 5, 13, 27, 40, 80, 160] {
        let img = match screen {
            1 => m.draw_screen_one(data, frame),
            _ => m.draw_screen_two(data, frame),
        };
        // Row-1 gutter (single column) on temp-row band.
        for y in 8..16 {
            let p = img.get_pixel(ROW1_GUTTER_X, y).0;
            if p != [0, 0, 0] {
                eprintln!(
                    "  ✗ screen {screen} frame {frame}: row1 gutter (x={ROW1_GUTTER_X},y={y}) lit {p:?}"
                );
                ok = false;
            }
        }
        // Row-2 gutter (two columns) on H/L band — only screen 1.
        if screen == 1 {
            for &x in &ROW2_GUTTER_X {
                for y in 18..26 {
                    let p = img.get_pixel(x, y).0;
                    if p != [0, 0, 0] {
                        eprintln!(
                            "  ✗ screen {screen} frame {frame}: row2 gutter (x={x},y={y}) lit {p:?}"
                        );
                        ok = false;
                    }
                }
            }
        }
    }
    ok
}

fn sf_weather() -> Weather {
    let now = Local.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
    Weather {
        api: WeatherApiSource::OpenWeather,
        lat: 37.7749,
        lon: -122.4194,
        location_name: "San Francisco".into(),
        current: CurrentWeather {
            conditions: "Clear".into(),
            temp: 68.0,
            feels_like: 66.0,
            wind_speed: 9.0,
            humidity: 72,
            precipitation_chance: 10,
            uv: Some(5.2),
            wind_direction_deg: Some(270.0),
            icon: oledlib::api::weather::icon_table::SUNNY,
        },
        forecast: DayForecast {
            today_high: 74.0,
            today_low: 56.0,
            sunrise: now - Duration::hours(6),
            sunset: now + Duration::hours(8),
        },
    }
}

fn main() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let fonts = WeatherFonts {
        body: repo.join("04B_03B_.TTF"),
        icon: repo.join("weathericons.ttf"),
        temp: repo.join("BMmini.TTF"),
    };
    let m = WeatherMatrix::with_fonts(fonts).expect("font load");

    // 1. Normal SF day.
    let normal = sf_weather();
    println!("\nscreen1 scroll frames (normal): {}", m.screen_one_scroll_frames(&normal));
    println!("screen2 scroll frames (normal): {}", m.screen_two_scroll_frames(&normal));
    ascii_preview(&m.draw_screen_one(&normal, 0), "=== Screen 1 — normal SF (frame 0) ===");
    ascii_preview(&m.draw_screen_two(&normal, 0), "=== Screen 2 — normal SF (frame 0) ===");

    // 2. Extreme temps — 100°F current, -12°F low, 100% humidity & precip.
    let mut extreme = sf_weather();
    extreme.current.temp = 100.0;
    extreme.current.feels_like = 105.0;
    extreme.forecast.today_high = 110.0;
    extreme.forecast.today_low = -12.0;
    extreme.current.humidity = 100;
    extreme.current.precipitation_chance = 100;
    println!("\nscreen1 scroll frames (extreme): {}", m.screen_one_scroll_frames(&extreme));
    println!("screen2 scroll frames (extreme): {}", m.screen_two_scroll_frames(&extreme));
    ascii_preview(&m.draw_screen_one(&extreme, 0), "=== Screen 1 — extreme (frame 0, settled) ===");
    ascii_preview(&m.draw_screen_one(&extreme, 8), "=== Screen 1 — extreme (frame 8, mid-marquee) ===");
    ascii_preview(&m.draw_screen_two(&extreme, 0), "=== Screen 2 — extreme (frame 0, settled) ===");
    ascii_preview(&m.draw_screen_two(&extreme, 8), "=== Screen 2 — extreme (frame 8, mid-marquee) ===");

    // 3. Programmatic gutter audit.
    println!("\n--- gutter audit ---");
    let s1_ok = check_gutters(&m, &extreme, 1);
    let s2_ok = check_gutters(&m, &extreme, 2);
    if s1_ok && s2_ok {
        println!("  ✓ all gutter columns dark across frames 0/5/13/27/40/80/160 on both screens");
    } else {
        std::process::exit(1);
    }
}
