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
use crate::api::aurora::swpc::SwpcConfig;
use crate::api::aurora::AuroraCollector;
use crate::api::flights::opensky::OpenSkyConfig;
use crate::api::flights::FlightsCollector;
use crate::api::hass::rest::RestConfig as HassRestConfig;
use crate::api::hass::HassCollector;
use crate::api::launch::lldev::LldevConfig;
use crate::api::launch::LaunchCollector;
use crate::api::pihole::v5::V5Config as PiholeV5Config;
use crate::api::pihole::PiholeCollector;
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
use crate::api::stock::yahoo::YahooConfig;
use crate::api::stock::{StockCollector, StockHistoryCollector};
use crate::api::weather::accuweather::AccuWeatherConfig;
use crate::api::weather::nws::NwsConfig;
use crate::api::weather::openweather::OpenWeatherConfig;
use crate::api::weather::pirate::PirateWeatherConfig;
use crate::api::weather::WeatherCollector;
use crate::api::{StockApi, WeatherApi};
use crate::matrix::f1::F1Matrix;
use crate::matrix::golf::GolfMatrix;
use crate::matrix::aurora::AuroraMatrix;
use crate::matrix::flights::FlightsMatrix;
use crate::matrix::hass::{HassDisplay, HassDisplayMode, HassMatrix};
use crate::matrix::iss::IssMatrix;
use crate::matrix::launch::LaunchMatrix;
use crate::matrix::pihole::PiholeMatrix;
use crate::matrix::quake::QuakeMatrix;
use crate::matrix::sport::SportMatrix;
use crate::matrix::stock::StockMatrix;
use crate::matrix::stock_chart::StockChartMatrix;
use crate::matrix::time::{TimeCollector, TimeMatrix};
use crate::matrix::weather::WeatherMatrix;
use crate::matrix::eink::{EinkTimeMatrix, EinkWeatherMatrix};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::modules::eink_module::{DynEinkModule, EinkModule};
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
    #[serde(default, deserialize_with = "one_or_many")]
    pub aurora: Vec<AuroraSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub flights: Vec<FlightsSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub launch: Vec<LaunchSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub hass: Vec<HassSection>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub pihole: Vec<PiholeSection>,
}

#[derive(Debug, Deserialize)]
pub struct TimeSection {
    pub run: bool,
    pub color: (u8, u8, u8),
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub time_format: Option<String>,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub timezone: Option<String>,
    /// Seconds between background polls. `0` ⇒ inline poll every
    /// render cycle (slow but always fresh). Omitted ⇒ use the
    /// collector's default `refresh_interval()`.
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
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
    /// Seconds between background polls. `0` ⇒ inline poll. Omitted ⇒
    /// use the collector default.
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct IssSection {
    pub run: bool,
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct QuakeSection {
    pub run: bool,
    #[serde(default)]
    pub feed: QuakeFeed,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct AuroraSection {
    pub run: bool,
    /// Kp value at which the "AURORA LIKELY" banner appears. Defaults to
    /// 5 — NOAA's G1 minor-storm threshold and the value above which
    /// mid-latitude observers can typically see aurora.
    #[serde(default = "default_aurora_threshold")]
    pub alert_threshold: u8,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

fn default_aurora_threshold() -> u8 {
    5
}

#[derive(Debug, Deserialize)]
pub struct FlightsSection {
    pub run: bool,
    pub lat: f64,
    pub lon: f64,
    /// Search radius from `(lat, lon)`, in km. Larger radius = larger
    /// bbox query = more credits/req against OpenSky's anonymous tier.
    #[serde(default = "default_flights_radius_km")]
    pub radius_km: f32,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

fn default_flights_radius_km() -> f32 {
    80.0
}

#[derive(Debug, Deserialize)]
pub struct LaunchSection {
    pub run: bool,
    /// Case-insensitive substring whitelist applied to LL2's
    /// `launch_service_provider.name`. Empty (the default) = show every
    /// upcoming launch regardless of provider.
    #[serde(default)]
    pub agency_filter: Vec<String>,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct HassSection {
    pub run: bool,
    /// Base URL of the HASS instance, e.g. `http://homeassistant.local:8123`.
    pub base_url: String,
    /// HASS long-lived access token (Profile → Security → Long-Lived
    /// Access Tokens). Sent as a `Bearer` header.
    pub token: String,
    /// Entity id, e.g. `sensor.kitchen_temp`.
    pub entity_id: String,
    /// Optional display label override. Falls back to HASS's
    /// `attributes.friendly_name`, then the bare entity id.
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub label: Option<String>,
    /// State string (case-insensitive) that flips the color from
    /// `nominal_color` to `alarm_color`. Useful for binary entities
    /// where one state is interesting (e.g. `"open"`, `"on"`).
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub alarm_state: Option<String>,
    /// RGB color for the default state row. Defaults to sage green.
    #[serde(default = "default_hass_nominal_color")]
    pub nominal_color: (u8, u8, u8),
    /// RGB color for the alarm state row. Defaults to red.
    #[serde(default = "default_hass_alarm_color")]
    pub alarm_color: (u8, u8, u8),
    /// Which layout to use: `"state"` (default), `"historical"`, or
    /// `"graph"`. Historical and graph fall back to state for
    /// non-numeric entities or until history has been fetched.
    #[serde(default)]
    pub display_mode: HassDisplayMode,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

fn default_hass_nominal_color() -> (u8, u8, u8) {
    (120, 220, 120)
}

fn default_hass_alarm_color() -> (u8, u8, u8) {
    (255, 60, 60)
}

#[derive(Debug, Deserialize)]
pub struct PiholeSection {
    pub run: bool,
    /// Pi-hole base URL (no trailing slash needed), e.g. `http://pi.hole`.
    pub base_url: String,
    /// Optional v5 admin token (SHA-256 from Settings → API/Web
    /// interface → Show API token). `null`/omitted = unauth.
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub token: Option<String>,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct StockSection {
    pub run: bool,
    pub api: StockApi,
    #[serde(default, deserialize_with = "crate::serde_helpers::null_string_as_none")]
    pub api_key: Option<String>,
    pub symbol: String,
    /// When true, also build a `stock_chart` tile (historical 1D/1M/1Y
    /// line graph) alongside this entry's live tile. Defaults to false
    /// to preserve existing configs unchanged. For Finnhub-style
    /// equity tickers the chart is sourced from Yahoo Finance (the
    /// only free historical option); CoinGecko entries pull the chart
    /// from CoinGecko's own `/market_chart` endpoint.
    #[serde(default)]
    pub chart: bool,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

/// One entry in the `sport` array. The `sport` field selects which renderer
/// to instantiate; remaining fields are variant-specific.
#[derive(Debug, Deserialize)]
#[serde(tag = "sport", rename_all = "lowercase")]
pub enum SportSection {
    Basketball {
        run: bool,
        team_logo: crate::teams::Logo,
        #[serde(default)]
        cache_ttl_secs: Option<u64>,
    },
    Baseball {
        run: bool,
        team_logo: crate::teams::Logo,
        #[serde(default)]
        cache_ttl_secs: Option<u64>,
    },
    Football {
        run: bool,
        team_logo: crate::teams::Logo,
        #[serde(default)]
        cache_ttl_secs: Option<u64>,
    },
    Hockey {
        run: bool,
        team_logo: crate::teams::Logo,
        #[serde(default)]
        cache_ttl_secs: Option<u64>,
    },
    Golf {
        run: bool,
        #[serde(default = "default_golf_tour")]
        tour: GolfTour,
        #[serde(default)]
        cache_ttl_secs: Option<u64>,
    },
    F1 {
        run: bool,
        #[serde(default)]
        cache_ttl_secs: Option<u64>,
    },
}

impl SportSection {
    fn run(&self) -> bool {
        match self {
            Self::Basketball { run, .. }
            | Self::Baseball { run, .. }
            | Self::Football { run, .. }
            | Self::Hockey { run, .. }
            | Self::Golf { run, .. }
            | Self::F1 { run, .. } => *run,
        }
    }

    fn cache_ttl_secs(&self) -> Option<u64> {
        match self {
            Self::Basketball { cache_ttl_secs, .. }
            | Self::Baseball { cache_ttl_secs, .. }
            | Self::Football { cache_ttl_secs, .. }
            | Self::Hockey { cache_ttl_secs, .. }
            | Self::Golf { cache_ttl_secs, .. }
            | Self::F1 { cache_ttl_secs, .. } => *cache_ttl_secs,
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
        "registry: parsed sections — time={} weather={} stock={} sport={} iss={} quake={} aurora={} flights={} launch={} hass={} pihole={}",
        cfg.time.len(),
        cfg.weather.len(),
        cfg.stock.len(),
        cfg.sport.len(),
        cfg.iss.len(),
        cfg.quake.len(),
        cfg.aurora.len(),
        cfg.flights.len(),
        cfg.launch.len(),
        cfg.hass.len(),
        cfg.pihole.len()
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
        if s.chart {
            match build_stock_chart(s).await {
                Ok(m) => {
                    log::info!("registry: stock_chart loaded ({})", s.symbol);
                    modules.push(m);
                }
                Err(e) => log::error!("stock_chart: skipping module ({}): {e}", s.symbol),
            }
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
    for s in cfg.aurora.iter().filter(|s| s.run) {
        match build_aurora(s).await {
            Ok(m) => {
                log::info!("registry: aurora loaded (threshold={})", s.alert_threshold);
                modules.push(m);
            }
            Err(e) => log::error!("aurora: skipping module: {e}"),
        }
    }
    for s in cfg.flights.iter().filter(|s| s.run) {
        match build_flights(s).await {
            Ok(m) => {
                log::info!(
                    "registry: flights loaded (lat={}, lon={}, radius_km={})",
                    s.lat, s.lon, s.radius_km
                );
                modules.push(m);
            }
            Err(e) => log::error!("flights: skipping module: {e}"),
        }
    }
    for s in cfg.launch.iter().filter(|s| s.run) {
        match build_launch(s).await {
            Ok(m) => {
                log::info!(
                    "registry: launch loaded (agency_filter={:?})",
                    s.agency_filter
                );
                modules.push(m);
            }
            Err(e) => log::error!("launch: skipping module: {e}"),
        }
    }
    for s in cfg.hass.iter().filter(|s| s.run) {
        match build_hass(s).await {
            Ok(m) => {
                log::info!(
                    "registry: hass loaded (entity_id={}, label={:?}, alarm_state={:?})",
                    s.entity_id, s.label, s.alarm_state
                );
                modules.push(m);
            }
            Err(e) => log::error!("hass: skipping module: {e}"),
        }
    }
    for s in cfg.pihole.iter().filter(|s| s.run) {
        match build_pihole(s).await {
            Ok(m) => {
                log::info!(
                    "registry: pihole loaded (base_url={}, token={})",
                    s.base_url,
                    if s.token.is_some() { "present" } else { "none" }
                );
                modules.push(m);
            }
            Err(e) => log::error!("pihole: skipping module: {e}"),
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
    Ok(module_with_ttl(TimeCollector::new(), renderer, t.cache_ttl_secs))
}

/// Build a `WeatherCollector` from a section's provider + credentials.
/// Shared by the LED ([`build_weather`]) and e-paper ([`build_eink_weather`])
/// tiles so both gather data the same way.
fn weather_collector(w: &WeatherSection) -> Result<WeatherCollector, String> {
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
    Ok(collector)
}

async fn build_weather(w: &WeatherSection) -> Result<Box<dyn DynModule>, String> {
    let collector = weather_collector(w)?;
    let renderer = WeatherMatrix::new_with_animation_async(w.animation)
        .await
        .map_err(|e| format!("weather fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, w.cache_ttl_secs))
}

/// Build the chart-mode (historical) module for one stock entry. For
/// Finnhub-style configs the chart source is always Yahoo (Finnhub's
/// `/candle` is paid-tier only); for CoinGecko configs the chart is
/// sourced from CoinGecko's own `/market_chart` endpoint so the same
/// provider stays in use across both tiles.
async fn build_stock_chart(s: &StockSection) -> Result<Box<dyn DynModule>, String> {
    let collector = match s.api {
        StockApi::Finnhub => StockHistoryCollector::from_yahoo(YahooConfig {
            symbol: s.symbol.clone(),
        })
        .map_err(|e| e.to_string())?,
        StockApi::Coingecko => StockHistoryCollector::from_coingecko(CoingeckoConfig {
            coin_id: s.symbol.clone(),
        })
        .map_err(|e| e.to_string())?,
    };
    let renderer = StockChartMatrix::new_async()
        .await
        .map_err(|e| format!("stock_chart fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
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
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

async fn build_sport(s: &SportSection) -> Result<Box<dyn DynModule>, String> {
    let ttl = s.cache_ttl_secs();
    match s {
        SportSection::Basketball { team_logo, .. } => {
            build_team_sport(SportKind::Basketball, team_logo, ttl).await
        }
        SportSection::Baseball { team_logo, .. } => {
            build_team_sport(SportKind::Baseball, team_logo, ttl).await
        }
        SportSection::Football { team_logo, .. } => {
            build_team_sport(SportKind::Football, team_logo, ttl).await
        }
        SportSection::Hockey { team_logo, .. } => {
            build_team_sport(SportKind::Hockey, team_logo, ttl).await
        }
        SportSection::Golf { tour, .. } => {
            let collector = GolfCollector::from_espn(*tour);
            let renderer = GolfMatrix::new_async()
                .await
                .map_err(|e| format!("golf fonts: {e}"))?;
            Ok(module_with_ttl(collector, renderer, ttl))
        }
        SportSection::F1 { .. } => {
            let collector = F1Collector::from_jolpica();
            let renderer = F1Matrix::new_async()
                .await
                .map_err(|e| format!("f1 fonts: {e}"))?;
            Ok(module_with_ttl(collector, renderer, ttl))
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
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

async fn build_quake(s: &QuakeSection) -> Result<Box<dyn DynModule>, String> {
    let collector = QuakeCollector::from_usgs(UsgsConfig { feed: s.feed })
        .map_err(|e| e.to_string())?;
    let renderer = QuakeMatrix::new_async()
        .await
        .map_err(|e| format!("quake fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

async fn build_aurora(s: &AuroraSection) -> Result<Box<dyn DynModule>, String> {
    let collector = AuroraCollector::from_swpc(SwpcConfig {
        alert_threshold: s.alert_threshold,
    })
    .map_err(|e| e.to_string())?;
    let renderer = AuroraMatrix::new_async()
        .await
        .map_err(|e| format!("aurora fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

async fn build_flights(s: &FlightsSection) -> Result<Box<dyn DynModule>, String> {
    let collector = FlightsCollector::from_opensky(OpenSkyConfig {
        user_lat: s.lat,
        user_lon: s.lon,
        radius_km: s.radius_km,
    })
    .map_err(|e| e.to_string())?;
    let renderer = FlightsMatrix::new_async()
        .await
        .map_err(|e| format!("flights fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

async fn build_launch(s: &LaunchSection) -> Result<Box<dyn DynModule>, String> {
    let collector = LaunchCollector::from_lldev(LldevConfig {
        agency_filter: s.agency_filter.clone(),
    })
    .map_err(|e| e.to_string())?;
    let renderer = LaunchMatrix::new_async()
        .await
        .map_err(|e| format!("launch fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

async fn build_hass(s: &HassSection) -> Result<Box<dyn DynModule>, String> {
    let collector = HassCollector::from_rest(HassRestConfig {
        base_url: s.base_url.clone(),
        token: s.token.clone(),
        entity_id: s.entity_id.clone(),
        label_override: s.label.clone(),
    })
    .map_err(|e| e.to_string())?;
    let display = HassDisplay {
        nominal_color: ohmyoled_matrix::Color::new(s.nominal_color.0, s.nominal_color.1, s.nominal_color.2),
        alarm_color: ohmyoled_matrix::Color::new(s.alarm_color.0, s.alarm_color.1, s.alarm_color.2),
        alarm_state: s.alarm_state.clone(),
        mode: s.display_mode,
    };
    let renderer = HassMatrix::with_fonts_async(
        crate::matrix::hass::HassFonts::default(),
        display,
    )
    .await
    .map_err(|e| format!("hass fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

async fn build_pihole(s: &PiholeSection) -> Result<Box<dyn DynModule>, String> {
    let collector = PiholeCollector::from_v5(PiholeV5Config {
        base_url: s.base_url.clone(),
        token: s.token.clone(),
    })
    .map_err(|e| e.to_string())?;
    let renderer = PiholeMatrix::new_async()
        .await
        .map_err(|e| format!("pihole fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

async fn build_team_sport(
    kind: SportKind,
    team_logo: &crate::teams::Logo,
    cache_ttl_secs: Option<u64>,
) -> Result<Box<dyn DynModule>, String> {
    let collector = SportCollector::from_espn(EspnConfig {
        sport: kind,
        team_name: team_logo.name.clone(),
        team_abbreviation: team_logo.shorthand.clone(),
    });
    let renderer = SportMatrix::new_async()
        .await
        .map_err(|e| format!("sport fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, cache_ttl_secs))
}

// ──────────────────────────────────────────────────────────────────────────
// E-paper (e-ink) display
// ──────────────────────────────────────────────────────────────────────────

/// The independent e-paper display config, parsed from the top-level `eink`
/// block. The e-ink screen shows its **own** content — its `modules` are a
/// full, separate [`RegistryConfig`] (same section shapes and the same
/// collectors as the LED display, chosen and tuned independently).
///
/// ```yaml
/// eink:
///   enabled: true
///   model: "4in2"
///   rotation: 0
///   threshold: 128
///   modules:
///     weather: { run: true, api: nws, current_location: true }
/// ```
#[derive(Debug, Deserialize, Default)]
pub struct EinkRegistryConfig {
    /// Drive the e-paper display instead of the LED matrix. Off by default so
    /// existing configs are unchanged.
    #[serde(default)]
    pub enabled: bool,
    /// Panel model id (e.g. `"4in2"`, `"7in5_v2"`) — sets resolution + driver.
    #[serde(default = "default_eink_model")]
    pub model: String,
    /// Rotation in degrees (0/90/180/270).
    #[serde(default)]
    pub rotation: u16,
    /// Luma threshold 0–255 for the black/white cut.
    #[serde(default = "default_eink_threshold")]
    pub threshold: u8,
    /// Optional explicit width override. Set both `width` and `height` to drive
    /// a panel not in the model table (or a custom dev size); otherwise the
    /// resolution comes from `model`.
    #[serde(default)]
    pub width: Option<u32>,
    /// Optional explicit height override (see `width`).
    #[serde(default)]
    pub height: Option<u32>,
    /// This display's own tile selection — a full registry config.
    #[serde(default)]
    pub modules: RegistryConfig,
}

fn default_eink_model() -> String {
    "4in2".to_string()
}

fn default_eink_threshold() -> u8 {
    128
}

impl EinkRegistryConfig {
    /// Convert to the matrix crate's panel options.
    pub fn options(&self) -> ohmyoled_matrix::EinkOptions {
        ohmyoled_matrix::EinkOptions {
            model: self.model.clone(),
            rotation: self.rotation,
            threshold: self.threshold,
            width: self.width,
            height: self.height,
        }
    }
}

/// Build the active e-paper modules from the `eink.modules` config.
///
/// Mirrors [`build`] but emits `DynEinkModule`s. For the first cut only the
/// weather tile has an e-ink renderer; any other enabled section is counted
/// and reported (a follow-up ports the rest), never silently dropped.
pub async fn build_eink(cfg: &RegistryConfig) -> Vec<Box<dyn DynEinkModule>> {
    let mut modules: Vec<Box<dyn DynEinkModule>> = Vec::new();

    for t in cfg.time.iter().filter(|t| t.run) {
        match build_eink_time(t).await {
            Ok(m) => {
                log::info!("eink registry: time loaded");
                modules.push(m);
            }
            Err(e) => log::error!("eink time: skipping module: {e}"),
        }
    }
    for w in cfg.weather.iter().filter(|w| w.run) {
        match build_eink_weather(w).await {
            Ok(m) => {
                log::info!("eink registry: weather loaded (provider={})", w.api.get_api());
                modules.push(m);
            }
            Err(e) => log::error!("eink weather: skipping module: {e}"),
        }
    }

    // Anything else enabled in the eink block is configured but not yet
    // renderable on e-paper — report it so the tile isn't silently ignored.
    let pending = cfg.stock.iter().filter(|s| s.run).count()
        + cfg.sport.iter().filter(|s| s.run()).count()
        + cfg.iss.iter().filter(|s| s.run).count()
        + cfg.quake.iter().filter(|s| s.run).count()
        + cfg.aurora.iter().filter(|s| s.run).count()
        + cfg.flights.iter().filter(|s| s.run).count()
        + cfg.launch.iter().filter(|s| s.run).count()
        + cfg.hass.iter().filter(|s| s.run).count()
        + cfg.pihole.iter().filter(|s| s.run).count();
    if pending > 0 {
        log::info!(
            "eink registry: {pending} enabled tile(s) have no e-ink renderer yet; \
             time and weather are supported so far"
        );
    }

    modules
}

async fn build_eink_time(t: &TimeSection) -> Result<Box<dyn DynEinkModule>, String> {
    let format = parse_time_format(t.time_format.as_deref());
    let renderer = EinkTimeMatrix::new_async()
        .await
        .map_err(|e| format!("eink time fonts: {e}"))?
        .with_format(format);
    Ok(eink_module_with_ttl(TimeCollector::new(), renderer, t.cache_ttl_secs))
}

/// Parse the config `time_format` string into a [`TimeFormat`]. Anything
/// 24-hour-ish selects 24h; everything else (incl. omitted) defaults to 12h.
fn parse_time_format(s: Option<&str>) -> crate::matrix::time::TimeFormat {
    use crate::matrix::time::TimeFormat;
    match s.map(|s| s.trim().to_lowercase()).as_deref() {
        Some("24h") | Some("24") | Some("twentyfour") | Some("military") => TimeFormat::TwentyFour,
        _ => TimeFormat::Twelve,
    }
}

async fn build_eink_weather(w: &WeatherSection) -> Result<Box<dyn DynEinkModule>, String> {
    let collector = weather_collector(w)?;
    let renderer = EinkWeatherMatrix::new_async()
        .await
        .map_err(|e| format!("eink weather fonts: {e}"))?;
    Ok(eink_module_with_ttl(collector, renderer, w.cache_ttl_secs))
}

/// E-paper analog of [`module_with_ttl`] — wraps a `(collector, eink renderer)`
/// pair into a boxed `DynEinkModule`, resolving the cache TTL identically.
fn eink_module_with_ttl<C, R>(
    collector: C,
    renderer: R,
    cache_ttl_secs: Option<u64>,
) -> Box<dyn DynEinkModule>
where
    C: crate::api::Collector + 'static,
    R: EinkRenderer<Data = C::Output> + 'static,
    C::Output: Send + Sync + 'static,
{
    let ttl = cache_ttl_secs
        .map(std::time::Duration::from_secs)
        .unwrap_or_else(|| collector.refresh_interval());
    Box::new(EinkModule::new(collector, renderer, ttl))
}

/// Wrap a `(collector, renderer)` pair into a boxed `DynModule`,
/// resolving the cache TTL from config (`cache_ttl_secs`) or the
/// collector's own `refresh_interval()` if unset.
///
/// - `Some(0)` ⇒ inline polling — the renderer calls `collector.poll()`
///   on every render. Always fresh; panel stalls during slow API calls.
/// - `Some(n > 0)` ⇒ background task polls every `n` seconds.
/// - `None` ⇒ background task polls at `collector.refresh_interval()`.
fn module_with_ttl<C, R>(
    collector: C,
    renderer: R,
    cache_ttl_secs: Option<u64>,
) -> Box<dyn DynModule>
where
    C: crate::api::Collector + 'static,
    R: crate::matrix::Renderer<Data = C::Output> + 'static,
    C::Output: Send + Sync + 'static,
{
    let ttl = cache_ttl_secs
        .map(std::time::Duration::from_secs)
        .unwrap_or_else(|| collector.refresh_interval());
    Box::new(Module::new(collector, renderer, ttl))
}
