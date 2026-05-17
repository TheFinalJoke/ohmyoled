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
//!   { "run": true, "sport": "basketball", ... },
//!   { "run": true, "sport": "hockey",     ... }
//! ]
//! ```

use crate::api::f1::F1Collector;
use crate::api::golf::{GolfCollector, GolfTour};
use crate::api::sport::espn::EspnConfig;
use crate::api::sport::model::SportKind;
use crate::api::sport::SportCollector;
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
use crate::matrix::sport::SportMatrix;
use crate::matrix::stock::StockMatrix;
use crate::matrix::time::{TimeCollector, TimeMatrix};
use crate::matrix::weather::WeatherMatrix;
use crate::modules::{DynModule, Module};
use crate::serde_helpers::one_or_many;
use crate::teams::SportsTypes;
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
    #[serde(default, deserialize_with = "one_or_many")]
    pub sport: Vec<SportSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub golf: Vec<GolfSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub f1: Vec<F1Section>,
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
}

#[derive(Debug, Deserialize)]
pub struct StockSection {
    pub run: bool,
    pub api: StockApi,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub api_key: Option<String>,
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct SportSection {
    pub run: bool,
    pub sport: SportsTypes,
    pub team_logo: crate::teams::Logo,
}

#[derive(Debug, Deserialize)]
pub struct GolfSection {
    pub run: bool,
    #[serde(default = "default_golf_tour")]
    pub tour: GolfTour,
}

fn default_golf_tour() -> GolfTour {
    GolfTour::Pga
}

#[derive(Debug, Deserialize)]
pub struct F1Section {
    pub run: bool,
}

/// Build the active `Vec<Box<dyn DynModule>>` from a parsed config.
///
/// Modules whose `run` flag is false are skipped. Order in the returned vec
/// matches the order modules will rotate through the panel.
pub async fn build(cfg: &RegistryConfig) -> Vec<Box<dyn DynModule>> {
    let mut modules: Vec<Box<dyn DynModule>> = Vec::new();

    for t in cfg.time.iter().filter(|t| t.run) {
        match build_time(t).await {
            Ok(m) => modules.push(m),
            Err(e) => log::error!("time: skipping module: {e}"),
        }
    }
    for w in cfg.weather.iter().filter(|w| w.run) {
        match build_weather(w).await {
            Ok(m) => modules.push(m),
            Err(e) => log::error!("weather: skipping module: {e}"),
        }
    }
    for s in cfg.stock.iter().filter(|s| s.run) {
        match build_stock(s).await {
            Ok(m) => modules.push(m),
            Err(e) => log::error!("stock: skipping module: {e}"),
        }
    }
    for s in cfg.sport.iter().filter(|s| s.run) {
        match build_sport(s).await {
            Ok(m) => modules.push(m),
            Err(e) => log::error!("sport: skipping module: {e}"),
        }
    }
    for g in cfg.golf.iter().filter(|g| g.run) {
        match build_golf(g).await {
            Ok(m) => modules.push(m),
            Err(e) => log::error!("golf: skipping module: {e}"),
        }
    }
    for f in cfg.f1.iter().filter(|f| f.run) {
        match build_f1(f).await {
            Ok(m) => modules.push(m),
            Err(e) => log::error!("f1: skipping module: {e}"),
        }
    }

    modules
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
    let renderer = WeatherMatrix::new_async()
        .await
        .map_err(|e| format!("weather fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

async fn build_stock(s: &StockSection) -> Result<Box<dyn DynModule>, String> {
    let api_key = s
        .api_key
        .clone()
        .ok_or_else(|| "stock: api_key missing".to_string())?;
    let collector = match s.api {
        StockApi::Finnhub => StockCollector::from_finnhub(FinnhubConfig {
            api_key,
            symbol: s.symbol.clone(),
        })
        .map_err(|e| e.to_string())?,
    };
    let renderer = StockMatrix::new_async()
        .await
        .map_err(|e| format!("stock fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

async fn build_sport(s: &SportSection) -> Result<Box<dyn DynModule>, String> {
    let sport_kind = match s.sport {
        SportsTypes::BASEBALL => SportKind::Baseball,
        SportsTypes::BASKETBALL => SportKind::Basketball,
        SportsTypes::FOOTBALL => SportKind::Football,
        SportsTypes::HOCKEY => SportKind::Hockey,
    };
    let collector = SportCollector::from_espn(EspnConfig {
        sport: sport_kind,
        team_name: s.team_logo.name.clone(),
        team_abbreviation: s.team_logo.shorthand.clone(),
    });
    let renderer = SportMatrix::new_async()
        .await
        .map_err(|e| format!("sport fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

async fn build_golf(g: &GolfSection) -> Result<Box<dyn DynModule>, String> {
    let collector = GolfCollector::from_espn(g.tour);
    let renderer = GolfMatrix::new_async()
        .await
        .map_err(|e| format!("golf fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

async fn build_f1(_f: &F1Section) -> Result<Box<dyn DynModule>, String> {
    let collector = F1Collector::from_jolpica();
    let renderer = F1Matrix::new_async()
        .await
        .map_err(|e| format!("f1 fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}
