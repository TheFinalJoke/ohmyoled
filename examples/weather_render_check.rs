//! ASCII-preview every weather screen with synthetic data.
//!
//! Run: `cargo run --example weather_render_check`
//!
//! Previews each of the six screens (hero stat-strip, horizon, comfort, UV,
//! 5-day strip, precip bars) with normal SF data, exercises the entry-frame
//! animations on a few screens, and shows the conditional-skip behaviour
//! when provider data is missing.

use chrono::{Duration, Local, NaiveDate, TimeZone};
use oledlib::api::weather::model::{
    CurrentWeather, DailyForecast, DayForecast, HourlyForecast, Weather, WeatherApiSource,
};
use oledlib::matrix::weather::{
    build_screens, Screen, WeatherAnimationMode, WeatherFonts, WeatherMatrix,
};
use std::path::PathBuf;

fn ascii_preview(img: &image::RgbImage, label: &str) {
    println!("\n{label}");
    let lit: usize = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
    println!("lit pixels: {lit}");
    // ANSI 24-bit truecolor background per pixel, two characters wide so
    // each "pixel" looks roughly square in a normal terminal font. Reset
    // at the end of every row so the right margin doesn't bleed.
    for y in 0..img.height() {
        let mut row = String::new();
        for x in 0..img.width() {
            let p = img.get_pixel(x, y).0;
            row.push_str(&format!("\x1b[48;2;{};{};{}m  ", p[0], p[1], p[2]));
        }
        row.push_str("\x1b[0m");
        println!("  {row}");
    }
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
        hourly: (0..12)
            .map(|i| HourlyForecast {
                time: now + Duration::hours(i),
                temp: 68.0 + (i as f32 / 2.0),
                // Build a wave: 0, 15, 35, 60, 80, 65, 40, 20, 10, 0, 0, 0
                precipitation_chance: match i {
                    0 => 0,
                    1 => 15,
                    2 => 35,
                    3 => 60,
                    4 => 80,
                    5 => 65,
                    6 => 40,
                    7 => 20,
                    8 => 10,
                    _ => 0,
                },
            })
            .collect(),
        daily: (1..=5)
            .map(|i| DailyForecast {
                date: NaiveDate::from_ymd_opt(2024, 6, 15 + i).unwrap(),
                high: 74.0 - i as f32,
                low: 56.0 - (i as f32 / 2.0),
                icon: if i == 3 {
                    oledlib::api::weather::icon_table::RAIN
                } else if i % 2 == 0 {
                    oledlib::api::weather::icon_table::PARTLY_CLOUDY
                } else {
                    oledlib::api::weather::icon_table::SUNNY
                },
                precipitation_chance: match i {
                    3 => 70,
                    4 => 50,
                    _ => 10,
                },
            })
            .collect(),
    }
}

fn build(mode: WeatherAnimationMode) -> WeatherMatrix {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let fonts = WeatherFonts {
        body: repo.join("04B_03B_.TTF"),
        icon: repo.join("weathericons.ttf"),
        temp: repo.join("BMmini.TTF"),
        small: repo.join("4x6.bdf"),
    };
    WeatherMatrix::with_fonts_and_animation(fonts, mode).expect("font load")
}

fn main() {
    let m = build(WeatherAnimationMode::Subtle);
    let normal = sf_weather();

    // ---- All six screens, sample frames ----
    ascii_preview(&m.frame_hero_stats(&normal, 0), "=== 1. Hero stat-strip — clear day, frame 0 ===");
    ascii_preview(&m.frame_horizon(&normal, 0), "=== 2. Horizon day-length bar ===");
    ascii_preview(&m.frame_comfort(&normal, 30), "=== 3. Comfort — bars filled ===");
    ascii_preview(&m.frame_uv(&normal, 0), "=== 4. UV gauge ===");
    ascii_preview(&m.frame_forecast_strip(&normal, 30), "=== 5. 3-day forecast strip ===");
    ascii_preview(&m.frame_precip_bars(&normal, 30), "=== 6. Precip-chance bars ===");

    // ---- Entry-frame animations ----
    ascii_preview(&m.frame_comfort(&normal, 0), "=== Comfort — entry frame 0 (bars empty) ===");
    ascii_preview(&m.frame_forecast_strip(&normal, 0), "=== Forecast — frame 0 (first cell only) ===");
    ascii_preview(&m.frame_precip_bars(&normal, 0), "=== Precip — frame 0 (bars hidden) ===");

    // ---- Hero across conditions to exercise animated icons ----
    let mut rainy = sf_weather();
    rainy.current.icon = oledlib::api::weather::icon_table::RAIN;
    rainy.current.conditions = "Light rain".into();
    ascii_preview(&m.frame_hero_stats(&rainy, 5), "=== Hero — rain SUBTLE frame 5 ===");

    let prominent = build(WeatherAnimationMode::Prominent);
    let mut storm = sf_weather();
    storm.current.icon = oledlib::api::weather::icon_table::THUNDERSTORM;
    storm.current.conditions = "Thunderstorm".into();
    ascii_preview(&prominent.frame_hero_stats(&storm, 0), "=== Hero — storm PROMINENT (lightning) ===");

    let mut snowy = sf_weather();
    snowy.current.icon = oledlib::api::weather::icon_table::SNOW;
    snowy.current.conditions = "Snow".into();
    ascii_preview(&m.frame_hero_stats(&snowy, 8), "=== Hero — snow SUBTLE frame 8 ===");

    // ---- Conditional skip ----
    let mut nws_like = sf_weather();
    nws_like.current.uv = None;
    nws_like.daily.clear();
    nws_like.hourly.clear();
    println!("\n=== NWS-like skip (no UV / no daily / no hourly) ===");
    for s in build_screens(&nws_like) {
        println!("  - {s:?}");
    }
    assert_eq!(
        build_screens(&nws_like),
        vec![Screen::Hero, Screen::Horizon, Screen::Comfort]
    );
    println!("  (expected: [Hero, Horizon, Comfort])");
}
