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
use ohmyoled_matrix::{Color, EinkDisplay, RGBMatrix};

use oledlib::api::f1::{DriverStanding, F1Data, NextRace};
use oledlib::api::golf::{GolfData, GolfTour, LeaderboardEntry};
use oledlib::api::aurora::AuroraReading;
use oledlib::api::flights::{FlightInfo, FlightSnapshot};
use oledlib::api::hass::HassEntity;
use oledlib::api::iss::IssState;
use oledlib::api::launch::{LaunchStatus, UpcomingLaunch};
use oledlib::api::pihole::PiholeSummary;
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
use oledlib::api::hass::HassSample;
use oledlib::matrix::hass::{HassDisplay, HassDisplayMode, HassFonts, HassMatrix};
use oledlib::matrix::iss::{IssFonts, IssMatrix};
use oledlib::matrix::launch::{LaunchFonts, LaunchMatrix};
use oledlib::matrix::pihole::{PiholeFonts, PiholeMatrix};
use oledlib::matrix::quake::{QuakeFonts, QuakeMatrix};
use oledlib::matrix::sport::{SportFonts, SportMatrix};
use oledlib::api::stock::{HistorySeries, StockHistory};
use oledlib::matrix::stock::{StockFonts, StockMatrix};
use oledlib::matrix::stock_chart::{StockChartFonts, StockChartMatrix};
use oledlib::matrix::time::TimeSnapshot;
use oledlib::matrix::weather::{WeatherAnimationMode, WeatherFonts, WeatherMatrix};
use oledlib::matrix::eink::{
    EinkAuroraFonts, EinkAuroraMatrix, EinkF1Fonts, EinkF1Matrix, EinkFlightsFonts,
    EinkFlightsMatrix, EinkGolfFonts, EinkGolfMatrix, EinkHassFonts, EinkHassMatrix, EinkIssFonts,
    EinkIssMatrix, EinkLaunchFonts, EinkLaunchMatrix, EinkPiholeFonts, EinkPiholeMatrix,
    EinkQuakeFonts, EinkQuakeMatrix, EinkSportFonts, EinkSportMatrix, EinkStockChartFonts,
    EinkStockChartMatrix, EinkStockFonts, EinkStockMatrix, EinkTimeFonts, EinkTimeMatrix,
    EinkWeatherFonts, EinkWeatherMatrix,
};
use oledlib::matrix::{Renderer, TimeMatrix};

pub const NAMES: &[&str] = &[
    "time", "weather", "stock", "stock_chart", "sport", "golf", "f1",
    "iss", "quake", "aurora", "flights", "launch", "hass", "pihole",
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
        "stock_chart" => preview_stock_chart(&mut matrix, &fonts).await,
        "sport" => preview_sport(&mut matrix, &fonts).await,
        "golf" => preview_golf(&mut matrix, &fonts).await,
        "f1" => preview_f1(&mut matrix, &fonts).await,
        "iss" => preview_iss(&mut matrix, &fonts).await,
        "quake" => preview_quake(&mut matrix, &fonts).await,
        "aurora" => preview_aurora(&mut matrix, &fonts).await,
        "flights" => preview_flights(&mut matrix, &fonts).await,
        "launch" => preview_launch(&mut matrix, &fonts).await,
        "hass" => preview_hass(&mut matrix, &fonts).await,
        "pihole" => preview_pihole(&mut matrix, &fonts).await,
        other => Err(format!(
            "unknown preview '{other}'. Available: {}",
            NAMES.join(", ")
        )),
    }
}

/// E-paper preview names (used after the `eink` prefix, e.g. `--preview eink:weather`).
pub const EINK_NAMES: &[&str] = &[
    "time", "weather", "pihole", "iss", "aurora", "quake", "stock", "launch", "hass", "flights",
    "stock_chart", "sport", "golf", "f1",
];

/// Drive an e-paper renderer live against an [`EinkDisplay`] with fake data.
/// Backend is whatever `EinkDisplay` resolves to — the terminal inline-image
/// (Sixel/PNG) view in the devcontainer (no hardware), the panel on a Pi.
/// Unlike the module path this refreshes every few seconds so layout iteration
/// is quick.
pub async fn run_eink(name: &str, mut display: EinkDisplay) -> Result<(), String> {
    let fonts = font_dir();
    log::info!(
        "preview: rendering eink '{name}' ({}x{}) with fonts from {}",
        display.width(),
        display.height(),
        fonts.display()
    );
    match name {
        "time" => preview_eink_time(&mut display, &fonts).await,
        "weather" => preview_eink_weather(&mut display, &fonts).await,
        "pihole" => preview_eink_pihole(&mut display, &fonts).await,
        "iss" => preview_eink_iss(&mut display, &fonts).await,
        "aurora" => preview_eink_aurora(&mut display, &fonts).await,
        "quake" => preview_eink_quake(&mut display, &fonts).await,
        "stock" => preview_eink_stock(&mut display, &fonts).await,
        "launch" => preview_eink_launch(&mut display, &fonts).await,
        "hass" => preview_eink_hass(&mut display, &fonts).await,
        "flights" => preview_eink_flights(&mut display, &fonts).await,
        "stock_chart" => preview_eink_stock_chart(&mut display, &fonts).await,
        "sport" => preview_eink_sport(&mut display, &fonts).await,
        "golf" => preview_eink_golf(&mut display, &fonts).await,
        "f1" => preview_eink_f1(&mut display, &fonts).await,
        other => Err(format!(
            "unknown eink preview '{other}'. Available: {}",
            EINK_NAMES.join(", ")
        )),
    }
}

async fn preview_eink_time(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkTimeMatrix::with_fonts_async(
        EinkTimeFonts {
            body: fonts.join("04B_03B_.TTF"),
        },
        dims,
    )
    .await?;
    loop {
        let img = r.frame(Local::now(), display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn preview_eink_weather(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkWeatherMatrix::with_fonts_async(
        EinkWeatherFonts {
            body: fonts.join("04B_03B_.TTF"),
            icon: fonts.join("weathericons.ttf"),
        },
        dims,
    )
    .await?;
    let data = fake_weather();
    loop {
        // Compose + push directly (rather than `render()`, which dwells for the
        // full 60 s cycle) so the preview redraws every few seconds.
        let img = r.frame(&data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_pihole(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkPiholeMatrix::with_fonts_async(
        EinkPiholeFonts {
            body: fonts.join("04B_03B_.TTF"),
            big: fonts.join("04b24.otf"),
        },
        dims,
    )
    .await?;
    let s = |pct: f32, q: u32, blk: u32| PiholeSummary {
        percent_blocked: pct,
        queries_today: q,
        blocked_today: blk,
        unique_clients: 12,
    };
    let cycle = [s(34.2, 12_348, 4_221), s(58.7, 88_500, 51_949), s(7.4, 2_100, 156)];
    let mut i = 0usize;
    loop {
        let img = r.frame(&cycle[i % cycle.len()], display.width(), display.height());
        display.show(&img);
        i = i.wrapping_add(1);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_iss(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkIssMatrix::with_fonts_async(EinkIssFonts { body: fonts.join("04B_03B_.TTF") }, dims).await?;
    let distant = IssState {
        ground_distance_km: 1247,
        overhead: false,
        lat: 23.5,
        lon: -50.1,
        altitude_km: 421.0,
        velocity_kms: 7.66,
        visibility: "daylight".into(),
    };
    let overhead = IssState { ground_distance_km: 120, overhead: true, ..distant.clone() };
    let mut toggle = false;
    loop {
        let data = if toggle { &overhead } else { &distant };
        toggle = !toggle;
        let img = r.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_aurora(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkAuroraMatrix::with_fonts_async(
        EinkAuroraFonts {
            body: fonts.join("04B_03B_.TTF"),
            big: fonts.join("04b24.otf"),
        },
        dims,
    )
    .await?;
    let now = chrono::Utc::now();
    let make = |kp: u8, alert: bool| AuroraReading {
        kp,
        kp_index: kp as f32,
        kp_text: format!("{kp}Z"),
        alert,
        sampled_at: now,
    };
    let cycle = [make(2, false), make(4, false), make(6, true), make(8, true)];
    let mut i = 0usize;
    loop {
        let img = r.frame(&cycle[i % cycle.len()], display.width(), display.height());
        display.show(&img);
        i = i.wrapping_add(1);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_quake(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkQuakeMatrix::with_fonts_async(EinkQuakeFonts { body: fonts.join("04B_03B_.TTF") }, dims).await?;
    let now = chrono::Utc::now() - ChDuration::minutes(14);
    let ev = |mag: f32, title: &str, lat: f32, lon: f32, felt: Option<u32>| {
        QuakeStatus::Event(QuakeEvent {
            magnitude: mag,
            title: title.into(),
            origin: now,
            lat,
            lon,
            depth_km: 24.0,
            felt,
        })
    };
    let cycle = [
        ev(6.2, "M 6.2 - OFF EAST COAST OF HONSHU, JAPAN", 38.3, 142.1, Some(482)),
        ev(4.7, "M 4.7 - 120km SW of San Francisco, CA", 37.0, -123.5, Some(37)),
        ev(3.1, "M 3.1 - 14 km NE of Reykjavik, Iceland", 64.2, -21.6, None),
        QuakeStatus::Quiet,
    ];
    let mut i = 0usize;
    loop {
        let img = r.frame(&cycle[i % cycle.len()], display.width(), display.height());
        display.show(&img);
        i = i.wrapping_add(1);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_stock(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkStockMatrix::with_fonts_async(EinkStockFonts { body: fonts.join("04B_03B_.TTF") }, dims).await?;
    let day = |slope: f64| HistorySeries::from_closes((0..26).map(|i| 188.0 + i as f64 * slope + ((i % 4) as f64 - 1.5)).collect());
    let q = |current: f64, slope: f64| StockHistory {
        api: StockApiSource::Yahoo,
        symbol: "AAPL".into(),
        current,
        previous_close: 190.0,
        day: day(slope),
        month: HistorySeries::from_closes((0..30).map(|i| 180.0 + i as f64 * 0.4).collect()),
        year: HistorySeries::from_closes((0..52).map(|i| 150.0 + i as f64 * 0.8).collect()),
    };
    let cycle = [q(195.2, 0.3), q(185.4, -0.25)];
    let mut i = 0usize;
    loop {
        let img = r.frame(&cycle[i % cycle.len()], display.width(), display.height());
        display.show(&img);
        i = i.wrapping_add(1);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_launch(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkLaunchMatrix::with_fonts_async(EinkLaunchFonts { body: fonts.join("04B_03B_.TTF") }, dims).await?;
    let l = |offset_secs: i64, status: LaunchStatus| UpcomingLaunch {
        provider: "SpaceX".into(),
        vehicle: "Falcon 9".into(),
        mission: "Starlink Group 8-1".into(),
        launch_at: chrono::Utc::now() + ChDuration::seconds(offset_secs),
        status,
        country_code: "USA".into(),
    };
    // Far → near → imminent → liftoff, so each countdown phase/badge shows.
    let cycle = [
        l(3 * 86_400, LaunchStatus::Go),
        l(2 * 3600, LaunchStatus::Go),
        l(42, LaunchStatus::Go),
        l(-30, LaunchStatus::InFlight),
    ];
    let mut i = 0usize;
    loop {
        let img = r.frame(&cycle[i % cycle.len()], chrono::Utc::now(), display.width(), display.height());
        display.show(&img);
        i = i.wrapping_add(1);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_hass(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let f = || EinkHassFonts { body: fonts.join("04B_03B_.TTF") };
    let disp = |mode: HassDisplayMode, alarm: Option<&str>| HassDisplay {
        alarm_state: alarm.map(|s| s.to_string()),
        mode,
        ..HassDisplay::default()
    };
    // One renderer per mode so all three layouts (state/historical/graph) and
    // the alarm badge get airtime.
    let renderers = [
        EinkHassMatrix::with_fonts_async(f(), disp(HassDisplayMode::State, None), dims).await?,
        EinkHassMatrix::with_fonts_async(f(), disp(HassDisplayMode::Graph, None), dims).await?,
        EinkHassMatrix::with_fonts_async(f(), disp(HassDisplayMode::Historical, None), dims).await?,
        EinkHassMatrix::with_fonts_async(f(), disp(HassDisplayMode::State, Some("72.4")), dims).await?,
    ];
    let now = chrono::Utc::now();
    let entity = HassEntity {
        state: "72.4".into(),
        unit: Some("F".into()),
        label: "Kitchen Temp".into(),
        last_changed: now - ChDuration::seconds(8),
        history: (0..16)
            .map(|i| HassSample {
                at: now - ChDuration::minutes(16 - i),
                value: 68.0 + (i as f64 * 0.4),
            })
            .collect(),
    };
    let mut i = 0usize;
    loop {
        let img = renderers[i % renderers.len()].frame(&entity, display.width(), display.height());
        display.show(&img);
        i = i.wrapping_add(1);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_flights(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkFlightsMatrix::with_fonts_async(EinkFlightsFonts { body: fonts.join("04B_03B_.TTF") }, dims).await?;
    let f = |callsign: &str, dist: f32, bearing: f32, heading: f32| FlightInfo {
        callsign: callsign.into(),
        icao24: "a1b2c3".into(),
        altitude_ft: 34_000,
        on_ground: false,
        distance_km: dist,
        bearing_deg: bearing,
        ground_speed_kt: Some(440),
        heading_deg: Some(heading),
        country: "United States".into(),
    };
    let nearby = vec![
        f("UAL123", 8.0, 45.0, 270.0),
        f("DAL456", 22.0, 200.0, 90.0),
        f("SWA789", 41.0, 300.0, 180.0),
        f("JBU221", 63.0, 120.0, 0.0),
    ];
    let busy = FlightSnapshot { count: nearby.len(), closest: Some(nearby[0].clone()), nearby, radius_km: 80.0 };
    let empty = FlightSnapshot { count: 0, closest: None, nearby: vec![], radius_km: 80.0 };
    let mut toggle = 0usize;
    loop {
        let data = if toggle % 3 == 2 { &empty } else { &busy };
        toggle = toggle.wrapping_add(1);
        let img = r.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_stock_chart(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkStockChartMatrix::with_fonts_async(EinkStockChartFonts { body: fonts.join("04B_03B_.TTF") }, dims).await?;
    let series = |n: usize, base: f64, slope: f64| {
        HistorySeries::from_closes((0..n).map(|i| base + (i as f64 * slope) + ((i % 5) as f64 - 2.0)).collect())
    };
    let data = StockHistory {
        api: StockApiSource::Yahoo,
        symbol: "AAPL".into(),
        current: 191.2,
        previous_close: 190.0,
        day: series(24, 188.0, 0.2),
        month: series(30, 180.0, 0.5),
        year: series(52, 150.0, 0.9),
    };
    loop {
        let img = r.frame(&data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_sport(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkSportMatrix::with_fonts_async(EinkSportFonts { body: fonts.join("04B_03B_.TTF") }, dims).await?;
    let standings: Vec<StandingsEntry> = ["Celtics", "76ers", "Knicks", "Bucks", "Heat", "Magic"]
        .iter()
        .enumerate()
        .map(|(i, t)| StandingsEntry { position: i as u32 + 1, team_name: (*t).into() })
        .collect();
    let live = SportData {
        api: SportApiSource::Espn,
        sport: SportKind::Basketball,
        team_name: "76ers".into(),
        record: "41-21".into(),
        next_game: Some(NextGame {
            start: Local::now(),
            status: GameStatus::InProgress,
            home: TeamSide { name: "76ers".into(), abbreviation: "PHI".into(), logo_url: None, score: Some(88) },
            away: TeamSide { name: "Celtics".into(), abbreviation: "BOS".into(), logo_url: None, score: Some(81) },
            our_side: HomeOrAway::Home,
        }),
        standings,
    };
    let offseason = SportData {
        api: SportApiSource::Espn,
        sport: SportKind::Basketball,
        team_name: "76ers".into(),
        record: "—".into(),
        next_game: None,
        standings: vec![],
    };
    let mut toggle = false;
    loop {
        let data = if toggle { &offseason } else { &live };
        toggle = !toggle;
        let img = r.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_golf(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkGolfMatrix::with_fonts_async(EinkGolfFonts { body: fonts.join("04B_03B_.TTF") }, dims)
        .await?
        .with_tour(GolfTour::Pga);
    let board: Vec<LeaderboardEntry> = [
        ("S. SCHEFFLER", "-12"), ("J. RAHM", "-9"), ("C. MORIKAWA", "-7"), ("J. SPIETH", "-5"),
        ("P. CANTLAY", "-3"), ("X. SCHAUFFELE", "E"), ("R. MCILROY", "+1"), ("T. FINAU", "+4"),
    ]
    .iter()
    .enumerate()
    .map(|(i, (n, s))| LeaderboardEntry { position: i as u32 + 1, player_short: (*n).into(), score: (*s).into() })
    .collect();
    let live = GolfData { tour: GolfTour::Pga, event_name: "The Masters".into(), status: "Round 3".into(), leaderboard: board };
    let offseason = GolfData { tour: GolfTour::Pga, event_name: "Next: Players Championship".into(), status: String::new(), leaderboard: vec![] };
    let mut toggle = false;
    loop {
        let data = if toggle { &offseason } else { &live };
        toggle = !toggle;
        let img = r.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn preview_eink_f1(display: &mut EinkDisplay, fonts: &Path) -> Result<(), String> {
    let dims = (display.width(), display.height());
    let r = EinkF1Matrix::with_fonts_async(EinkF1Fonts { body: fonts.join("04B_03B_.TTF") }, dims).await?;
    let standings: Vec<DriverStanding> = [
        ("VER", "Verstappen", 314.0), ("NOR", "Norris", 270.0), ("LEC", "Leclerc", 226.0),
        ("PIA", "Piastri", 197.0), ("SAI", "Sainz", 184.0), ("HAM", "Hamilton", 150.0),
    ]
    .iter()
    .enumerate()
    .map(|(i, (c, n, p))| DriverStanding { position: i as u32 + 1, code: (*c).into(), family_name: (*n).into(), points: *p })
    .collect();
    let in_season = F1Data {
        season: "2025".into(),
        next_race: Some(NextRace {
            round: 7,
            name: "Monaco Grand Prix".into(),
            circuit: "Circuit de Monaco".into(),
            start: Local::now() + ChDuration::days(3) + ChDuration::hours(4),
        }),
        standings: standings.clone(),
    };
    let offseason = F1Data { season: "2025".into(), next_race: None, standings };
    let mut toggle = false;
    loop {
        let data = if toggle { &offseason } else { &in_season };
        toggle = !toggle;
        let img = r.frame(data, Local::now(), display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
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

async fn preview_stock_chart(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = StockChartMatrix::with_fonts_async(StockChartFonts {
        body: fonts.join("04B_03B_.TTF"),
    })
    .await?;
    // Hand-tuned trajectories so each window has a distinct shape:
    // 1D = noisy intraday wave that closes up; 1M = clear uptrend
    // with a midmonth dip; 1Y = steady climb. Lets the preview verify
    // the autoscale and direction-color logic across all three.
    let day: Vec<f64> = (0..78)
        .map(|i| 180.0 + (i as f64 * 0.12).sin() * 1.2 + i as f64 * 0.03)
        .collect();
    let month: Vec<f64> = (0..22)
        .map(|i| 170.0 + i as f64 * 0.7 - ((i as f64 - 12.0) * 0.4).abs())
        .collect();
    let year: Vec<f64> = (0..52)
        .map(|i| 140.0 + i as f64 * 0.85 + (i as f64 * 0.3).sin() * 4.0)
        .collect();
    let data = StockHistory {
        api: oledlib::api::stock::StockApiSource::Yahoo,
        symbol: "AAPL".into(),
        current: *day.last().unwrap(),
        previous_close: day[0],
        day: HistorySeries::from_closes(day),
        month: HistorySeries::from_closes(month),
        year: HistorySeries::from_closes(year),
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
        lat: 38.3,
        lon: 142.1,
        depth_km: 24.0,
        felt: Some(482),
    });
    let medium = QuakeStatus::Event(QuakeEvent {
        magnitude: 4.7,
        title: "M 4.7 - 120km SW of San Francisco, CA".into(),
        origin: now,
        lat: 37.0,
        lon: -123.5,
        depth_km: 8.0,
        felt: Some(37),
    });
    let small = QuakeStatus::Event(QuakeEvent {
        magnitude: 3.1,
        title: "M 3.1 - 14 km NE of Reykjavík, Iceland".into(),
        origin: now,
        lat: 64.2,
        lon: -21.6,
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
        heading_deg: Some(bearing),
        country: "United States".into(),
    };
    let snap = |radius_km: f32, flights: Vec<FlightInfo>| FlightSnapshot {
        count: flights.len(),
        closest: flights.first().cloned(),
        nearby: flights,
        radius_km,
    };
    // Cycle through: a busy airspace (full radar with assorted bearings
    // and ranges), a single taxiing aircraft (GND footer), a single
    // long-callsign airliner (corner truncation), and an empty
    // airspace tile so the NO AC badge gets airtime too.
    let cycle = [
        snap(
            80.0,
            vec![
                flight("DAL2451", 32_000, 12.4, 225.0, false),
                flight("UAL989", 38_000, 28.0, 5.0, false),
                flight("JBU42", 30_000, 44.0, 88.0, false),
                flight("AAL117", 36_000, 55.0, 145.0, false),
                flight("FDX1338", 34_000, 62.0, 200.0, false),
                flight("SWA2210", 31_000, 71.0, 305.0, false),
                flight("N123AB", 4_500, 18.0, 340.0, false),
            ],
        ),
        snap(80.0, vec![flight("UAL899", 0, 1.2, 90.0, true)]),
        snap(80.0, vec![flight("LUFTHANSA404", 38_000, 47.0, 315.0, false)]),
        snap(80.0, vec![]),
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
    // Cycle through the three display modes: state (current), historical
    // (recent past samples), graph (sparkline). Each mode gets its own
    // matrix instance because `display.mode` is set at construction.
    let make = |mode: HassDisplayMode| -> Result<HassMatrix, String> {
        HassMatrix::with_fonts(
            HassFonts {
                body: fonts.join("04B_03B_.TTF"),
            },
            HassDisplay {
                alarm_state: Some("open".into()),
                mode,
                ..HassDisplay::default()
            },
        )
    };
    let mut state_r = make(HassDisplayMode::State)?;
    let mut hist_r = make(HassDisplayMode::Historical)?;
    let mut graph_r = make(HassDisplayMode::Graph)?;

    let now = chrono::Utc::now();
    let entity = |state: &str, unit: Option<&str>, label: &str, age_secs: i64| HassEntity {
        state: state.into(),
        unit: unit.map(str::to_string),
        label: label.into(),
        last_changed: now - chrono::Duration::seconds(age_secs),
        history: vec![],
    };
    // Fake history for the numeric kitchen sensor — gentle warm-up
    // through the morning, plotted across the sparkline.
    let trend: Vec<f64> = (0..16)
        .map(|i| 68.0 + (i as f64 * 0.3) + ((i as f64 * 0.4).sin() * 0.4))
        .collect();
    let history: Vec<HassSample> = trend
        .iter()
        .enumerate()
        .map(|(i, v)| HassSample {
            at: now - chrono::Duration::seconds((trend.len() - i) as i64 * 60),
            value: *v,
        })
        .collect();

    let kitchen_state = entity("72.4", Some("°F"), "KITCHEN", 12);
    let kitchen_hist = HassEntity { history: history.clone(), ..kitchen_state.clone() };
    let garage = entity("open", None, "GARAGE", 14 * 60);
    let motion = entity("off", None, "MOTION", 35);
    let unavail = entity("unavailable", None, "OFFICE LIGHT", 5);

    // Cycle: state mode on a numeric sensor → historical view of the same
    // sensor → graph view of the same sensor → state mode on assorted
    // non-numeric entities so the user can see all three modes plus the
    // existing alarm-flip and unavailable paths.
    let cycle: [(HassDisplayMode, &HassEntity); 6] = [
        (HassDisplayMode::State,      &kitchen_state),
        (HassDisplayMode::Historical, &kitchen_hist),
        (HassDisplayMode::Graph,      &kitchen_hist),
        (HassDisplayMode::State,      &garage),
        (HassDisplayMode::State,      &motion),
        (HassDisplayMode::State,      &unavail),
    ];
    let mut i = 0usize;
    loop {
        let (mode, data) = cycle[i % cycle.len()];
        i = i.wrapping_add(1);
        let r = match mode {
            HassDisplayMode::State => &mut state_r,
            HassDisplayMode::Historical => &mut hist_r,
            HassDisplayMode::Graph => &mut graph_r,
        };
        r.render(matrix, data).await.map_err(|e| e.to_string())?;
    }
}

async fn preview_pihole(matrix: &mut RGBMatrix, fonts: &Path) -> Result<(), String> {
    let mut r = PiholeMatrix::with_fonts_async(PiholeFonts {
        body: fonts.join("04B_03B_.TTF"),
        big: fonts.join("04b24.otf"),
    })
    .await?;
    let s = |pct: f32, q: u32, blk: u32| PiholeSummary {
        percent_blocked: pct,
        queries_today: q,
        blocked_today: blk,
        unique_clients: 12,
    };
    // Cycle: typical home install, busy/aggressive, near-zero (small
    // network), and brand-new install (no traffic yet).
    let cycle = [
        s(34.2, 12_348, 4_221),  // bright tier
        s(58.7, 88_500, 51_949), // bright (lots of ads)
        s(7.4, 2_100, 156),      // dim tier (quiet weekend)
        s(0.0, 0, 0),             // brand new
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
            // Today's sunrise/sunset (relative to the real clock) so the sun/moon
            // arc lands at the actual current time of day.
            sunrise: Local::now().date_naive().and_hms_opt(6, 12, 0).unwrap().and_local_timezone(Local).single().unwrap(),
            sunset: Local::now().date_naive().and_hms_opt(20, 45, 0).unwrap().and_local_timezone(Local).single().unwrap(),
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
