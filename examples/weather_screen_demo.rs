//! Push a single weather screen straight to the `RGBMatrix` terminal
//! backend so its colors render the way the actual binary would.
//!
//! Run: `cargo run --example weather_screen_demo -- 3`
//! Args: 1..=6 — pick the screen to display (default 3 / comfort).
//!
//! Holds the screen on the panel for ~15 s with the bar-fill animation
//! cycling, then exits. Synthetic data so no API keys are needed.

use chrono::{Duration, Local, NaiveDate, TimeZone};
use ohmyoled_matrix::{MatrixOptions, RGBMatrix};
use oledlib::api::weather::model::{
    CurrentWeather, DailyForecast, DayForecast, HourlyForecast, Weather, WeatherApiSource,
};
use oledlib::matrix::weather::{Screen, WeatherAnimationMode, WeatherFonts, WeatherMatrix};
use std::path::PathBuf;
use std::time::Duration as StdDuration;

fn sample() -> Weather {
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
            wind_speed: 12.0,
            humidity: 72,
            precipitation_chance: 10,
            uv: Some(5.2),
            wind_direction_deg: Some(70.0), // ENE
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
                precipitation_chance: match i {
                    3 => 60, 4 => 80, 5 => 65, _ => 0,
                },
            })
            .collect(),
        daily: (1..=5)
            .map(|i| DailyForecast {
                date: NaiveDate::from_ymd_opt(2024, 6, 15 + i).unwrap(),
                high: 74.0 - i as f32,
                low: 56.0 - (i as f32 / 2.0),
                icon: oledlib::api::weather::icon_table::SUNNY,
                precipitation_chance: 10 * i,
            })
            .collect(),
    }
}

fn main() {
    let which: u32 = std::env::args()
        .nth(1)
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let screen = match which {
        1 => Screen::Hero,
        2 => Screen::Horizon,
        3 => Screen::Comfort,
        4 => Screen::Uv,
        5 => Screen::Forecast3,
        6 => Screen::Precip,
        _ => Screen::Comfort,
    };

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let fonts = WeatherFonts {
        body: repo.join("04B_03B_.TTF"),
        icon: repo.join("weathericons.ttf"),
        temp: repo.join("BMmini.TTF"),
        small: repo.join("4x6.bdf"),
    };
    let renderer =
        WeatherMatrix::with_fonts_and_animation(fonts, WeatherAnimationMode::Subtle)
            .expect("font load");

    let mut matrix = RGBMatrix::test(MatrixOptions::default());
    let data = sample();

    // Drive frames the same way render() does. ~15 s at 50 ms tick = 300 frames.
    let frames: u32 = 300;
    for entry_frame in 0..frames {
        let img = renderer.render_screen(screen, &data, entry_frame, entry_frame);
        matrix.set_image(&img, 0, 0);
        std::thread::sleep(StdDuration::from_millis(50));
    }
}
