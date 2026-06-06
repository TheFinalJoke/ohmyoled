//! End-to-end smoke test for the e-paper pipeline.
//!
//! Exercises the **production** eink path — `EinkModule::render_cached`
//! (`DynEinkModule`) → `acquire_snapshot` (inline poll of a `Collector`) →
//! `EinkWeatherMatrix::render` → `EinkDisplay::show` → terminal backend flush
//! → PNG — with a local collector returning a real `Weather` value, so it
//! doesn't depend on a live upstream API. This is exactly what
//! `scheduler::run_eink` calls per cycle.
//!
//! Run: `cargo run --example eink_e2e`
//! Asserts a populated frame reached the panel (counts inked pixels in the
//! flushed PNG) and exits non-zero on failure.

use async_trait::async_trait;
use chrono::{Duration as ChDuration, Local, NaiveDate, TimeZone};
use oledlib::api::error::ApiError;
use oledlib::api::weather::icon_table;
use oledlib::api::weather::model::{
    CurrentWeather, DailyForecast, DayForecast, Weather, WeatherApiSource,
};
use oledlib::api::Collector;
use oledlib::matrix::eink::{EinkWeatherFonts, EinkWeatherMatrix};
use oledlib::modules::eink_module::{DynEinkModule, EinkModule};
use ohmyoled_matrix::{EinkDisplay, EinkOptions};
use std::time::Duration;

/// Local stand-in for a `WeatherCollector` — returns a fixed real `Weather`
/// snapshot so the e2e is deterministic and offline.
struct StubWeather;

#[async_trait]
impl Collector for StubWeather {
    type Output = Weather;
    fn id(&self) -> &'static str {
        "weather"
    }
    fn refresh_interval(&self) -> Duration {
        Duration::from_secs(600)
    }
    async fn poll(&self) -> Result<Weather, ApiError> {
        let now = Local.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
        Ok(Weather {
            api: WeatherApiSource::Nws,
            lat: 40.71,
            lon: -74.0,
            location_name: "New York".into(),
            current: CurrentWeather {
                conditions: "Partly Cloudy".into(),
                temp: 71.0,
                feels_like: 69.0,
                wind_speed: 11.0,
                humidity: 64,
                precipitation_chance: 20,
                uv: Some(6.0),
                wind_direction_deg: Some(225.0),
                icon: icon_table::PARTLY_CLOUDY,
            },
            forecast: DayForecast {
                today_high: 78.0,
                today_low: 61.0,
                sunrise: now - ChDuration::hours(6),
                sunset: now + ChDuration::hours(8),
            },
            hourly: vec![],
            daily: (1..=5)
                .map(|i| DailyForecast {
                    date: NaiveDate::from_ymd_opt(2026, 6, 6 + i).unwrap(),
                    high: 78.0 - i as f32,
                    low: 61.0 - i as f32,
                    icon: if i % 2 == 0 {
                        icon_table::RAIN
                    } else {
                        icon_table::SUNNY
                    },
                    precipitation_chance: 10 * i,
                })
                .collect(),
        })
    }
}

#[tokio::main]
async fn main() {
    // Force the hardware-free terminal backend + capture a PNG to inspect.
    std::env::set_var("OHMYOLED_EINK_MODE", "terminal");
    let png_path = "/tmp/eink_e2e.png";
    std::env::set_var("OHMYOLED_EINK_PNG", png_path);
    let _ = std::fs::remove_file(png_path);

    // Fonts: prefer the repo checkout so this runs without `install.sh`.
    let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
    std::env::set_var("OHMYOLED_FONTS_DIR", &fonts);
    let renderer = EinkWeatherMatrix::with_fonts_async(EinkWeatherFonts {
        body: fonts.join("04B_03B_.TTF"),
        icon: fonts.join("weathericons.ttf"),
    })
    .await
    .expect("load eink fonts");

    // Inline polling (ttl = 0) so render_cached polls the stub on every cycle.
    let mut module: Box<dyn DynEinkModule> =
        Box::new(EinkModule::new(StubWeather, renderer, Duration::ZERO));
    let mut display = EinkDisplay::test(EinkOptions::default());
    println!(
        "e2e: display {}x{} ({:?} backend)",
        display.width(),
        display.height(),
        display.mode
    );

    // One production render cycle. render() shows the frame, then dwells for
    // its 60 s cycle_duration — the timeout fires during that dwell, after the
    // frame (and PNG) is already on the panel.
    let _ = tokio::time::timeout(Duration::from_secs(5), module.render_cached(&mut display)).await;

    // Verify a populated frame actually reached the panel.
    let img = image::open(png_path)
        .unwrap_or_else(|e| panic!("e2e: expected a flushed frame at {png_path}: {e}"))
        .to_luma8();
    let (w, h) = img.dimensions();
    let ink = img.pixels().filter(|p| p.0[0] < 128).count();
    println!("e2e: flushed {w}x{h} frame, {ink} inked pixels -> {png_path}");
    assert_eq!((w, h), (400, 300), "frame should match the 4in2 panel");
    assert!(ink > 200, "expected a populated dashboard, got {ink} inked px");
    println!("e2e: PASS — real data rendered end-to-end through EinkModule + EinkDisplay");
}
