//! Module registration site.
//!
//! Each enabled section of the config is mapped to one or more
//! `Module<Collector, Renderer>` pairs here. **This is the one-line edit
//! when adding a new API.**
//!
//! Every section can be either a single object (legacy) or an array:
//!
//! ```json
//! "sport": { "run": true, "sport": "basketball", ... }
//! "sport": [
//!   { "run": true, "sport": "basketball", "team_logo": { ... } },
//!   { "run": true, "sport": "hockey",     "team_logo": { ... } },
//!   { "run": true, "sport": "golf",       "tour": "pga" },
//!   { "run": true, "sport": "f1" }
//! ]
//! ```
//!
//! Sport entries are discriminated by their inner `"sport"` field: the team
//! sports (`basketball`/`baseball`/`football`/`hockey`) take a `team_logo`;
//! `golf` takes an optional `tour`; `f1` takes nothing extra.

use crate::api::f1::F1Collector;
use crate::api::golf::{GolfCollector, GolfTour};
use crate::api::iss::wheretheiss::WhereTheIssConfig;
use crate::api::iss::IssCollector;
use crate::api::quake::model::QuakeFeed;
use crate::api::quake::usgs::UsgsConfig;
use crate::api::quake::QuakeCollector;
use crate::api::sport::espn::EspnConfig;
use crate::api::sport::model::SportKind;
use crate::api::sport::SportCollector;
use crate::api::stock::coingecko::CoingeckoConfig;
use crate::api::stock::finnhub::FinnhubConfig;
use crate::api::stock::StockCollector;
use crate::api::weather::accuweather::AccuWeatherConfig;
use crate::api::weather::nws::NwsConfig;
use crate::api::weather::openweather::OpenWeatherConfig;
use crate::api::weather::pirate::PirateWeatherConfig;
use crate::api::weather::WeatherCollector;
use crate::api::{StockApi, WeatherApi};
use crate::matrix::f1::F1Matrix;
use crate::matrix::golf::GolfMatrix;
use crate::matrix::iss::IssMatrix;
use crate::matrix::quake::QuakeMatrix;
use crate::matrix::sport::SportMatrix;
use crate::matrix::stock::StockMatrix;
use crate::matrix::time::{TimeCollector, TimeMatrix};
use crate::matrix::weather::WeatherMatrix;
use crate::modules::{DynModule, Module};
use crate::serde_helpers::one_or_many;
use serde::Deserialize;

/// Subset of the on-disk config the registry cares about.
///
/// Every section is a `Vec<_>` — `one_or_many` makes both `"sport": {...}` and
/// `"sport": [{...}, {...}]` parse cleanly.
#[derive(Debug, Deserialize, Default)]
pub struct RegistryConfig {
    #[serde(default, deserialize_with = "one_or_many")]
    pub time: Vec<TimeSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub weather: Vec<WeatherSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub stock: Vec<StockSection>,
    /// Team sports + golf + F1 all live here, discriminated by the `"sport"`
    /// field of each entry.
    #[serde(default, deserialize_with = "one_or_many")]
    pub sport: Vec<SportSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub iss: Vec<IssSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub quake: Vec<QuakeSection>,
}

#[derive(Debug, Deserialize)]
pub struct TimeSection {
    pub run: bool,
    pub color: (u8, u8, u8),
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub time_format: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WeatherSection {
    pub run: bool,
    pub api: WeatherApi,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub current_location: bool,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub city: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub weather_format: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub current_location_api_key: Option<String>,
    #[serde(default)]
    pub animation: crate::matrix::weather::WeatherAnimationMode,
}

#[derive(Debug, Deserialize)]
pub struct IssSection {
    pub run: bool,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Deserialize)]
pub struct QuakeSection {
    pub run: bool,
    #[serde(default)]
    pub feed: QuakeFeed,
}

#[derive(Debug, Deserialize)]
pub struct StockSection {
    pub run: bool,
    pub api: StockApi,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub api_key: Option<String>,
    pub symbol: String,
}

/// One entry in the `sport` array. The `sport` field selects which renderer
/// to instantiate; remaining fields are variant-specific.
#[derive(Debug, Deserialize)]
#[serde(tag = "sport", rename_all = "lowercase")]
pub enum SportSection {
    Basketball { run: bool, team_logo: crate::teams::Logo },
    Baseball { run: bool, team_logo: crate::teams::Logo },
    Football { run: bool, team_logo: crate::teams::Logo },
    Hockey { run: bool, team_logo: crate::teams::Logo },
    Golf {
        run: bool,
        #[serde(default = "default_golf_tour")]
        tour: GolfTour,
    },
    F1 { run: bool },
}

impl SportSection {
    fn run(&self) -> bool {
        match self {
            Self::Basketball { run, .. }
            | Self::Baseball { run, .. }
            | Self::Football { run, .. }
            | Self::Hockey { run, .. }
            | Self::Golf { run, .. }
            | Self::F1 { run } => *run,
        }
    }
}

fn default_golf_tour() -> GolfTour {
    GolfTour::Pga
}

/// Build the active `Vec<Box<dyn DynModule>>` from a parsed config.
///
/// Modules whose `run` flag is false are skipped. Order in the returned vec
/// matches the order modules will rotate through the panel.
pub async fn build(cfg: &RegistryConfig) -> Vec<Box<dyn DynModule>> {
    let mut modules: Vec<Box<dyn DynModule>> = Vec::new();
    log::debug!(
        "registry: parsed sections — time={} weather={} stock={} sport={} iss={} quake={}",
        cfg.time.len(),
        cfg.weather.len(),
        cfg.stock.len(),
        cfg.sport.len(),
        cfg.iss.len(),
        cfg.quake.len()
    );

    for t in cfg.time.iter().filter(|t| t.run) {
        match build_time(t).await {
            Ok(m) => {
                log::info!("registry: time loaded (color={:?})", t.color);
                modules.push(m);
            }
            Err(e) => log::error!("time: skipping module: {e}"),
        }
    }
    for w in cfg.weather.iter().filter(|w| w.run) {
        match build_weather(w).await {
            Ok(m) => {
                log::info!(
                    "registry: weather loaded (provider={}, current_location={})",
                    w.api.get_api(),
                    w.current_location
                );
                modules.push(m);
            }
            Err(e) => log::error!("weather: skipping module: {e}"),
        }
    }
    for s in cfg.stock.iter().filter(|s| s.run) {
        match build_stock(s).await {
            Ok(m) => {
                log::info!("registry: stock loaded ({} via {})", s.symbol, s.api.get_api());
                modules.push(m);
            }
            Err(e) => log::error!("stock: skipping module: {e}"),
        }
    }
    for s in cfg.sport.iter().filter(|s| s.run()) {
        match build_sport(s).await {
            Ok(m) => {
                log::info!("registry: sport loaded ({})", describe_sport(s));
                modules.push(m);
            }
            Err(e) => log::error!("sport: skipping module: {e}"),
        }
    }
    for s in cfg.iss.iter().filter(|s| s.run) {
        match build_iss(s).await {
            Ok(m) => {
                log::info!("registry: iss loaded (lat={}, lon={})", s.lat, s.lon);
                modules.push(m);
            }
            Err(e) => log::error!("iss: skipping module: {e}"),
        }
    }
    for s in cfg.quake.iter().filter(|s| s.run) {
        match build_quake(s).await {
            Ok(m) => {
                log::info!("registry: quake loaded (feed={})", s.feed.slug());
                modules.push(m);
            }
            Err(e) => log::error!("quake: skipping module: {e}"),
        }
    }

    modules
}

fn describe_sport(s: &SportSection) -> String {
    match s {
        SportSection::Basketball { team_logo, .. } => format!("basketball: {}", team_logo.name),
        SportSection::Baseball { team_logo, .. } => format!("baseball: {}", team_logo.name),
        SportSection::Football { team_logo, .. } => format!("football: {}", team_logo.name),
        SportSection::Hockey { team_logo, .. } => format!("hockey: {}", team_logo.name),
        SportSection::Golf { tour, .. } => format!("golf: {tour:?}"),
        SportSection::F1 { .. } => "f1".to_string(),
    }
}

async fn build_time(t: &TimeSection) -> Result<Box<dyn DynModule>, String> {
    let color = ohmyoled_matrix::Color::new(t.color.0, t.color.1, t.color.2);
    let renderer = TimeMatrix::new_async(color, None)
        .await
        .map_err(|e| format!("font: {e}"))?;
    Ok(Box::new(Module::new(TimeCollector::new(), renderer)))
}

async fn build_weather(w: &WeatherSection) -> Result<Box<dyn DynModule>, String> {
    let units = w
        .weather_format
        .clone()
        .unwrap_or_else(|| "imperial".to_string());
    let require_key = |provider: &'static str| -> Result<String, String> {
        w.api_key
            .clone()
            .ok_or_else(|| format!("{provider}: api_key missing"))
    };
    let collector = match w.api {
        WeatherApi::Openweather => WeatherCollector::from_openweather(OpenWeatherConfig {
            api_key: require_key("openweather")?,
            units: units.clone(),
            use_current_location: w.current_location,
            ipinfo_token: w.current_location_api_key.clone(),
            city: w.city.clone(),
        })
        .map_err(|e| e.to_string())?,
        WeatherApi::Nws => WeatherCollector::from_nws(NwsConfig {
            ipinfo_token: w.current_location_api_key.clone(),
        }),
        WeatherApi::Accuweather => WeatherCollector::from_accuweather(AccuWeatherConfig {
            api_key: require_key("accuweather")?,
            units: units.clone(),
            use_current_location: w.current_location,
            ipinfo_token: w.current_location_api_key.clone(),
        })
        .map_err(|e| e.to_string())?,
        WeatherApi::Pirate => WeatherCollector::from_pirate(PirateWeatherConfig {
            api_key: require_key("pirate")?,
            units,
            use_current_location: w.current_location,
            ipinfo_token: w.current_location_api_key.clone(),
        })
        .map_err(|e| e.to_string())?,
    };
    let renderer = WeatherMatrix::new_with_animation_async(w.animation)
        .await
        .map_err(|e| format!("weather fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

async fn build_stock(s: &StockSection) -> Result<Box<dyn DynModule>, String> {
    let collector = match s.api {
        StockApi::Finnhub => {
            let api_key = s
                .api_key
                .clone()
                .ok_or_else(|| "stock: api_key missing".to_string())?;
            StockCollector::from_finnhub(FinnhubConfig {
                api_key,
                symbol: s.symbol.clone(),
            })
            .map_err(|e| e.to_string())?
        }
        // CoinGecko's public tier is unauthenticated — `api_key` in the
        // config is ignored. `symbol` carries the coin id (e.g. "bitcoin").
        StockApi::Coingecko => StockCollector::from_coingecko(CoingeckoConfig {
            coin_id: s.symbol.clone(),
        })
        .map_err(|e| e.to_string())?,
    };
    let renderer = StockMatrix::new_async()
        .await
        .map_err(|e| format!("stock fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

async fn build_sport(s: &SportSection) -> Result<Box<dyn DynModule>, String> {
    match s {
        SportSection::Basketball { team_logo, .. } => {
            build_team_sport(SportKind::Basketball, team_logo).await
        }
        SportSection::Baseball { team_logo, .. } => {
            build_team_sport(SportKind::Baseball, team_logo).await
        }
        SportSection::Football { team_logo, .. } => {
            build_team_sport(SportKind::Football, team_logo).await
        }
        SportSection::Hockey { team_logo, .. } => {
            build_team_sport(SportKind::Hockey, team_logo).await
        }
        SportSection::Golf { tour, .. } => {
            let collector = GolfCollector::from_espn(*tour);
            let renderer = GolfMatrix::new_async()
                .await
                .map_err(|e| format!("golf fonts: {e}"))?;
            Ok(Box::new(Module::new(collector, renderer)))
        }
        SportSection::F1 { .. } => {
            let collector = F1Collector::from_jolpica();
            let renderer = F1Matrix::new_async()
                .await
                .map_err(|e| format!("f1 fonts: {e}"))?;
            Ok(Box::new(Module::new(collector, renderer)))
        }
    }
}

async fn build_iss(s: &IssSection) -> Result<Box<dyn DynModule>, String> {
    let collector = IssCollector::from_wheretheiss(WhereTheIssConfig {
        user_lat: s.lat,
        user_lon: s.lon,
    })
    .map_err(|e| e.to_string())?;
    let renderer = IssMatrix::new_async()
        .await
        .map_err(|e| format!("iss fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

async fn build_quake(s: &QuakeSection) -> Result<Box<dyn DynModule>, String> {
    let collector = QuakeCollector::from_usgs(UsgsConfig { feed: s.feed })
        .map_err(|e| e.to_string())?;
    let renderer = QuakeMatrix::new_async()
        .await
        .map_err(|e| format!("quake fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

async fn build_team_sport(
    kind: SportKind,
    team_logo: &crate::teams::Logo,
) -> Result<Box<dyn DynModule>, String> {
    let collector = SportCollector::from_espn(EspnConfig {
        sport: kind,
        team_name: team_logo.name.clone(),
        team_abbreviation: team_logo.shorthand.clone(),
    });
    let renderer = SportMatrix::new_async()
        .await
        .map_err(|e| format!("sport fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}
