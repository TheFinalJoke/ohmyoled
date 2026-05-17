//! Weather collector — pluggable via [`WeatherSource`] between OpenWeatherMap
//! and the National Weather Service.

use crate::api::collector::Collector;
use crate::api::error::ApiError;
use async_trait::async_trait;
use std::time::Duration;

pub mod accuweather;
pub mod geo;
pub mod icon_table;
pub mod model;
pub mod nws;
pub mod openweather;
pub mod pirate;

pub use model::{CurrentWeather, DayForecast, Weather, WeatherApiSource, WeatherIcon, WindDirection};

use accuweather::{AccuWeatherClient, AccuWeatherConfig};
use nws::{NwsClient, NwsConfig};
use openweather::{OpenWeatherClient, OpenWeatherConfig};
use pirate::{PirateWeatherClient, PirateWeatherConfig};

/// Which provider this collector is using.
pub enum WeatherSource {
    OpenWeather(OpenWeatherClient),
    Nws(NwsClient),
    AccuWeather(AccuWeatherClient),
    Pirate(PirateWeatherClient),
}

impl WeatherSource {
    pub async fn poll(&self) -> Result<Weather, ApiError> {
        match self {
            Self::OpenWeather(c) => c.poll().await,
            Self::Nws(c) => c.poll().await,
            Self::AccuWeather(c) => c.poll().await,
            Self::Pirate(c) => c.poll().await,
        }
    }
}

/// Top-level weather collector. Bridges the trait + provider enum.
pub struct WeatherCollector {
    source: WeatherSource,
    refresh: Duration,
}

impl WeatherCollector {
    pub fn new(source: WeatherSource) -> Self {
        Self {
            source,
            refresh: Duration::from_secs(600),
        }
    }

    /// Build directly from a [`crate::api::WeatherApi`] choice + the
    /// per-provider config.
    pub fn from_openweather(cfg: OpenWeatherConfig) -> Result<Self, ApiError> {
        Ok(Self::new(WeatherSource::OpenWeather(OpenWeatherClient::new(cfg)?)))
    }

    pub fn from_nws(cfg: NwsConfig) -> Self {
        Self::new(WeatherSource::Nws(NwsClient::new(cfg)))
    }

    pub fn from_accuweather(cfg: AccuWeatherConfig) -> Result<Self, ApiError> {
        Ok(Self::new(WeatherSource::AccuWeather(AccuWeatherClient::new(cfg)?)))
    }

    pub fn from_pirate(cfg: PirateWeatherConfig) -> Result<Self, ApiError> {
        Ok(Self::new(WeatherSource::Pirate(PirateWeatherClient::new(cfg)?)))
    }
}

#[async_trait]
impl Collector for WeatherCollector {
    type Output = Weather;

    fn id(&self) -> &'static str {
        "weather"
    }

    fn refresh_interval(&self) -> Duration {
        self.refresh
    }

    async fn poll(&self) -> Result<Weather, ApiError> {
        self.source.poll().await
    }
}
