//! ASCII-preview the two weather screens with synthetic data.
//!
//! Run: `PYO3_PYTHON=/usr/bin/python3.11 cargo run --example weather_render_check`

use chrono::{Duration, Local, TimeZone};
use oledlib::api::weather::model::{
    CurrentWeather, DayForecast, Weather, WeatherApiSource, WeatherIcon,
};
use oledlib::matrix::weather::{WeatherFonts, WeatherMatrix};
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
    let fonts = WeatherFonts {
        body: repo.join("04B_03B_.TTF"),
        icon: repo.join("weathericons.ttf"),
        retro: repo.join("retro_computer.ttf"),
    };
    let m = WeatherMatrix::with_fonts(fonts).expect("font load");

    let now = Local.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
    let data = Weather {
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
            icon: WeatherIcon { condition: "Sunny", glyph: '\u{f00d}', owm_code: 800 },
        },
        forecast: DayForecast {
            today_high: 74.0,
            today_low: 56.0,
            sunrise: now - Duration::hours(6),
            sunset: now + Duration::hours(8),
        },
    };

    let img1 = m.draw_screen_one(&data, 0);
    ascii_preview(&img1, "=== Screen 1 (temp + icon + location) ===");

    let img2 = m.draw_screen_two(&data, 0);
    ascii_preview(&img2, "=== Screen 2 (humidity + wind + sunrise/sunset) ===");
}
