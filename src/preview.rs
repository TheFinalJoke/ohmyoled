//! `--preview <name>` — drive any renderer live against the matrix with
//! built-in fake data so you can eyeball changes without a real config or
//! network. Loops forever until SIGINT.
//!
//! Backend is whatever `RGBMatrix` resolves to (terminal mode by default in
//! the devcontainer; real panel on a Pi). Pick a screen by name:
//!
//! ```text
//! ohmyoled --preview time
//! ohmyoled --preview weather
//! ohmyoled --preview stock
//! ohmyoled --preview sport
//! ohmyoled --preview golf
//! ohmyoled --preview f1
//! ohmyoled --preview iss
//! ```
//!
//! Fonts resolve in this order: `OHMYOLED_FONTS_DIR`, the repo `fonts/`
//! directory (when running from a source checkout), then
//! `/usr/share/fonts/`.

use std::path::{Path, PathBuf};

use chrono::{Duration as ChDuration, Local, TimeZone};
use ohmyoled_matrix::{Color, RGBMatrix};

use oledlib::api::f1::{DriverStanding, F1Data, NextRace};
use oledlib::api::golf::{GolfData, GolfTour, LeaderboardEntry};
use oledlib::api::aurora::AuroraReading;
use oledlib::api::flights::{FlightInfo, FlightSnapshot};
use oledlib::api::hass::HassEntity;
use oledlib::api::iss::IssState;
use oledlib::api::launch::{LaunchStatus, UpcomingLaunch};
use oledlib::api::quake::{QuakeEvent, QuakeStatus};
use oledlib::api::sport::model::{
    GameStatus, HomeOrAway, NextGame, SportApiSource, SportData, SportKind, StandingsEntry,
    TeamSide,
};
use oledlib::api::stock::model::{StockApiSource, StockQuote};
use chrono::NaiveDate;
use oledlib::api::weather::model::{
    CurrentWeather, DailyForecast, DayForecast, HourlyForecast, Weather, WeatherApiSource,
};
use oledlib::matrix::f1::{F1Fonts, F1Matrix};
use oledlib::matrix::golf::{GolfFonts, GolfMatrix};
use oledlib::matrix::aurora::{AuroraFonts, AuroraMatrix};
use oledlib::matrix::flights::{FlightsFonts, FlightsMatrix};
use oledlib::matrix::hass::{HassDisplay, HassFonts, HassMatrix};
use oledlib::matrix::iss::{IssFonts, IssMatrix};
use oledlib::matrix::launch::{LaunchFonts, LaunchMatrix};
use oledlib::matrix::quake::{QuakeFonts, QuakeMatrix};
use oledlib::matrix::sport::{SportFonts, SportMatrix};
use oledlib::matrix::stock::{StockFonts, StockMatrix};
use oledlib::matrix::time::TimeSnapshot;
use oledlib::matrix::weather::{WeatherAnimationMode, WeatherFonts, WeatherMatrix};
use oledlib::matrix::{Renderer, TimeMatrix};

pub const NAMES: &[&str] = &[
    "time", "weather", "stock", "sport", "golf", "f1",
    "iss", "quake", "aurora", "flights", "launch", "hass",
];

/// Resolve a directory that contains the project font files.
fn font_dir() -> PathBuf {
    if let Ok(env) = std::env::var("OHMYOLED_FONTS_DIR") {
        let p = PathBuf::from(env);
        if p.exists() {
            return p;
        }
    }
    // CARGO_MANIFEST_DIR is fixed at compile time; check it exists at runtime
    // before trusting it (release binaries are compiled on a different host).
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fonts");
    if repo.exists() {
        return repo;
    }
    PathBuf::from("/usr/share/fonts")
}

pub async fn run(name: &str, mut matrix: RGBMatrix) -> Result<(), String> {
    let fonts = font_dir();
    log::info!("preview: rendering '{name}' with fonts from {}", fonts.display());
    match name {
        "time" => preview_time(&mut matrix, &fonts).await,
        "weather" => preview_weather(&mut matrix, &fonts).await,
        "stock" => preview_stock(&mut matrix, &fonts).await,
        "sport" => preview_sport(&mut matrix, &fonts).await,
        "golf" => preview_golf(&mut matrix, &fonts).await,
        "f1" => preview_f1(&mut matrix, &fonts).await,
        "iss" => preview_iss(&mut matrix, &fonts).await,
        "quake" => preview_quake(&mut matrix, &fonts).await,
        "aurora" => preview_aurora(&mut matrix, &fonts).await,
        "flights" => preview_flights(&mut matrix, &fonts).await,
        "launch" => preview_launch(&mut matrix, &fonts).await,
        "hass" => preview_hass(&mut matrix, &fonts).await,
        other => Err(format!(
            "unknown preview '{other}'. Available: {}",
            NAMES.join(", ")
        )),
    }
}

async fn preview_time(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let path = fonts.join("04B_03B_.TTF");
    let mut r = TimeMatrix::new_async(Color::WHITE, Some(&path)).await?;
    loop {
        let snap = TimeSnapshot { now: Local::now() };
        r.render(matrix, &snap).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_weather(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = WeatherMatrix::with_fonts_and_animation_async(
        WeatherFonts {
            body: fonts.join("04B_03B_.TTF"),
            icon: fonts.join("weathericons.ttf"),
            temp: fonts.join("BMmini.TTF"),
            small: fonts.join("4x6.bdf"),
        },
        WeatherAnimationMode::default(),
    )
    .await?;
    let data = fake_weather();
    loop {
        r.render(matrix, &data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_stock(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = StockMatrix::with_fonts_async(StockFonts {
        body: fonts.join("04B_03B_.TTF"),
    })
    .await?;
    let data = StockQuote {
        api: StockApiSource::Finnhub,
        symbol: "AAPL".into(),
        name: "Apple Inc.".into(),
        open: 150.00,
        current: 153.42,
        high: 154.10,
        low: 149.85,
        previous_close: 150.20,
    };
    loop {
        r.render(matrix, &data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_sport(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = SportMatrix::with_fonts_async(SportFonts {
        body: fonts.join("04B_03B_.TTF"),
        big: fonts.join("04b24.otf"),
    })
    .await?;
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
        }),
        standings: (1..=5)
            .map(|i| StandingsEntry {
                position: i,
                team_name: format!("Team{i}"),
            })
            .collect(),
    };
    loop {
        r.render(matrix, &data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_golf(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = GolfMatrix::with_fonts_async(GolfFonts {
        body: fonts.join("04B_03B_.TTF"),
    })
    .await?;
    let data = GolfData {
        tour: GolfTour::Pga,
        event_name: "Masters Tournament".into(),
        status: "In Progress".into(),
        leaderboard: vec![
            LeaderboardEntry { position: 1, player_short: "SCHEFFLER".into(), score: "-12".into() },
            LeaderboardEntry { position: 2, player_short: "RAHM".into(),      score: "-8".into()  },
            LeaderboardEntry { position: 3, player_short: "MORIKAWA".into(),  score: "-6".into()  },
            LeaderboardEntry { position: 4, player_short: "SPIETH".into(),    score: "-4".into()  },
            LeaderboardEntry { position: 5, player_short: "CANTLAY".into(),   score: "-3".into()  },
        ],
    };
    loop {
        r.render(matrix, &data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_f1(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = F1Matrix::with_fonts_async(F1Fonts {
        body: fonts.join("04B_03B_.TTF"),
    })
    .await?;
    let data = F1Data {
        season: "2026".into(),
        next_race: Some(NextRace {
            round: 7,
            name: "Monaco Grand Prix".into(),
            circuit: "Circuit de Monaco".into(),
            start: Local::now() + ChDuration::days(3),
        }),
        standings: vec![
            DriverStanding { position: 1, code: "VER".into(), family_name: "Verstappen".into(), points: 161.0 },
            DriverStanding { position: 2, code: "NOR".into(), family_name: "Norris".into(),     points: 145.0 },
            DriverStanding { position: 3, code: "LEC".into(), family_name: "Leclerc".into(),    points: 132.0 },
            DriverStanding { position: 4, code: "RUS".into(), family_name: "Russell".into(),    points: 118.0 },
            DriverStanding { position: 5, code: "HAM".into(), family_name: "Hamilton".into(),   points: 99.0  },
        ],
    };
    loop {
        r.render(matrix, &data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_iss(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = IssMatrix::with_fonts_async(IssFonts {
        body: fonts.join("04B_03B_.TTF"),
    })
    .await?;
    let distant = IssState {
        ground_distance_km: 1247,
        overhead: false,
        lat: 23.5,
        lon: -50.1,
        altitude_km: 421.0,
        velocity_kms: 7.66,
        visibility: "daylight".into(),
    };
    let overhead = IssState { ground_distance_km: 250, overhead: true, ..distant.clone() };
    // Alternate the two modes so the magenta OVERHEAD banner gets airtime too.
    let mut toggle = false;
    loop {
        let data = if toggle { &overhead } else { &distant };
        toggle = !toggle;
        r.render(matrix, data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_quake(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = QuakeMatrix::with_fonts_async(QuakeFonts {
        body: fonts.join("04B_03B_.TTF"),
    })
    .await?;
    // Cycle through magnitude bands + a long region + quiet, so each visual
    // mode and color band gets airtime.
    let now = chrono::Utc::now() - ChDuration::minutes(14);
    let big = QuakeStatus::Event(QuakeEvent {
        magnitude: 6.2,
        title: "M 6.2 - OFF EAST COAST OF HONSHU, JAPAN".into(),
        origin: now,
        depth_km: 24.0,
        felt: Some(482),
    });
    let medium = QuakeStatus::Event(QuakeEvent {
        magnitude: 4.7,
        title: "M 4.7 - 120km SW of San Francisco, CA".into(),
        origin: now,
        depth_km: 8.0,
        felt: Some(37),
    });
    let small = QuakeStatus::Event(QuakeEvent {
        magnitude: 3.1,
        title: "M 3.1 - 14 km NE of Reykjavík, Iceland".into(),
        origin: now,
        depth_km: 5.0,
        felt: None,
    });
    let quiet = QuakeStatus::Quiet;
    let cycle = [big, medium, small, quiet];
    let mut i = 0usize;
    loop {
        let data = &cycle[i % cycle.len()];
        i = i.wrapping_add(1);
        r.render(matrix, data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_aurora(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = AuroraMatrix::with_fonts_async(AuroraFonts {
        label: fonts.join("04B_03B_.TTF"),
        big: fonts.join("04b24.otf"),
    })
    .await?;
    // Cycle through quiet → unsettled → minor storm → severe so each
    // color band and the alert banner all get airtime.
    let now = chrono::Utc::now();
    let make = |kp: u8, alert: bool| AuroraReading {
        kp,
        kp_index: kp as f32,
        kp_text: format!("{kp}Z"),
        alert,
        sampled_at: now,
    };
    let cycle = [
        make(2, false), // quiet
        make(4, false), // unsettled
        make(6, true),  // minor/moderate storm — alert
        make(8, true),  // severe — alert
    ];
    let mut i = 0usize;
    loop {
        let data = &cycle[i % cycle.len()];
        i = i.wrapping_add(1);
        r.render(matrix, data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_flights(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = FlightsMatrix::with_fonts_async(FlightsFonts {
        body: fonts.join("04B_03B_.TTF"),
    })
    .await?;
    let flight = |callsign: &str, alt: u32, dist: f32, bearing: f32, on_ground: bool| FlightInfo {
        callsign: callsign.into(),
        icao24: "abc123".into(),
        altitude_ft: alt,
        on_ground,
        distance_km: dist,
        bearing_deg: bearing,
        ground_speed_kt: Some(440),
        country: "United States".into(),
    };
    let snap = |count: usize, closest: Option<FlightInfo>| FlightSnapshot { count, closest };
    // Cycle through: typical airliner, on-ground taxiing flight, long
    // callsign (triggers the marquee), and an empty-airspace tile so the
    // QUIET SKIES path gets airtime too.
    let cycle = [
        snap(7, Some(flight("DAL2451", 32_000, 12.4, 225.0, false))),
        snap(3, Some(flight("UAL899", 0, 1.2, 90.0, true))),
        snap(12, Some(flight("LUFTHANSA404", 38_000, 47.0, 315.0, false))),
        snap(0, None),
    ];
    let mut i = 0usize;
    loop {
        let data = &cycle[i % cycle.len()];
        i = i.wrapping_add(1);
        r.render(matrix, data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_launch(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = LaunchMatrix::with_fonts_async(LaunchFonts {
        body: fonts.join("04B_03B_.TTF"),
    })
    .await?;
    let now = chrono::Utc::now();
    let make = |offset: chrono::Duration, status: LaunchStatus| UpcomingLaunch {
        provider: "SpaceX".into(),
        vehicle: "Falcon 9".into(),
        mission: "Starlink Group 8-5".into(),
        launch_at: now + offset,
        status,
        country_code: "USA".into(),
    };
    // Cycle through all four countdown modes so each color band and the
    // imminent-blink path get airtime.
    let cycle = [
        make(chrono::Duration::days(2) + chrono::Duration::hours(14), LaunchStatus::Go), // T-far
        make(chrono::Duration::hours(3) + chrono::Duration::minutes(42), LaunchStatus::Go), // T-near
        make(chrono::Duration::seconds(8), LaunchStatus::Go), // T-imminent
        make(chrono::Duration::seconds(-2), LaunchStatus::InFlight), // liftoff
    ];
    let mut i = 0usize;
    loop {
        let data = &cycle[i % cycle.len()];
        i = i.wrapping_add(1);
        r.render(matrix, data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_hass(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    // Alarm-state defaults to "open" so the binary-sensor cycle entry
    // demonstrates the color flip. Other cycle entries don't match the
    // alarm, so they render in the nominal green.
    let mut r = HassMatrix::with_fonts_async(
        HassFonts {
            body: fonts.join("04B_03B_.TTF"),
        },
        HassDisplay {
            alarm_state: Some("open".into()),
            ..HassDisplay::default()
        },
    )
    .await?;
    let now = chrono::Utc::now();
    let entity = |state: &str, unit: Option<&str>, label: &str, age_secs: i64| HassEntity {
        state: state.into(),
        unit: unit.map(str::to_string),
        label: label.into(),
        last_changed: now - chrono::Duration::seconds(age_secs),
    };
    // Cycle: numeric sensor, binary door (alarm tripped), binary motion (idle),
    // unavailable (edge case).
    let cycle = [
        entity("72.4", Some("°F"), "KITCHEN", 12),
        entity("open", None, "GARAGE", 14 * 60),
        entity("off", None, "MOTION", 35),
        entity("unavailable", None, "OFFICE LIGHT", 5),
    ];
    let mut i = 0usize;
    loop {
        let data = &cycle[i % cycle.len()];
        i = i.wrapping_add(1);
        r.render(matrix, data).await.map_err(|e| e.to_string())?;
    }
}

fn fake_weather() -> Weather {
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
            sunrise: now - ChDuration::hours(6),
            sunset: now + ChDuration::hours(8),
        },
        hourly: (0..12)
            .map(|i| HourlyForecast {
                time: now + ChDuration::hours(i),
                temp: 68.0 + (i as f32 / 2.0),
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
