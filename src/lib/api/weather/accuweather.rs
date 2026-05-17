//! AccuWeather provider.
//!
//! Two-step fetch: lat/lon → location key (cities/geoposition/search) → then
//! `currentconditions/v1/{key}` + `forecasts/v1/daily/1day/{key}` in parallel.
//!
//! AccuWeather requires `metric=true|false` per request; we pass `imperial`
//! semantics by requesting metric and converting locally if the user asked for
//! imperial — keeping the on-disk schema (`weather_format`) consistent across
//! providers.
//!
//! The [`accuweather`](https://crates.io/crates/accuweather) crate exists but
//! pins to `reqwest 0.9` (sync, last released 2020) and lacks a geoposition
//! lookup. We deserialize the same response shapes with our own typed structs
//! and fetch via the workspace's `reqwest 0.12` async client instead.

use super::geo::{lookup_ipinfo, GeoLocation};
use super::icon_table::icon_for_accuweather_code;
use super::model::{CurrentWeather, DayForecast, Weather, WeatherApiSource};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct AccuWeatherConfig {
    pub api_key: String,
    /// `"imperial"` or `"metric"` — controls unit conversion of the response.
    pub units: String,
    pub use_current_location: bool,
    pub ipinfo_token: Option<String>,
}

pub struct AccuWeatherClient {
    cfg: AccuWeatherConfig,
    geo: Arc<OnceCell<GeoLocation>>,
    location_key: Arc<OnceCell<String>>,
}

impl AccuWeatherClient {
    pub fn new(cfg: AccuWeatherConfig) -> Result<Self, ApiError> {
        if cfg.api_key.is_empty() {
            return Err(ApiError::Config(
                "accuweather: api_key missing".to_string(),
            ));
        }
        Ok(Self {
            cfg,
            geo: Arc::new(OnceCell::new()),
            location_key: Arc::new(OnceCell::new()),
        })
    }

    async fn resolve_location(&self) -> Result<GeoLocation, ApiError> {
        if let Some(loc) = self.geo.get() {
            return Ok(loc.clone());
        }
        let token = self.cfg.ipinfo_token.as_deref().unwrap_or("");
        let loc = lookup_ipinfo(token).await?;
        let _ = self.geo.set(loc.clone());
        Ok(loc)
    }

    async fn resolve_location_key(&self, loc: &GeoLocation) -> Result<String, ApiError> {
        if let Some(k) = self.location_key.get() {
            return Ok(k.clone());
        }
        let url = reqwest::Url::parse_with_params(
            "https://dataservice.accuweather.com/locations/v1/cities/geoposition/search",
            &[
                ("apikey", self.cfg.api_key.as_str()),
                ("q", format!("{},{}", loc.lat, loc.lon).as_str()),
            ],
        )
        .map_err(|e| ApiError::Config(format!("accuweather url: {e}")))?;
        let lookup: GeoPositionResponse = get_json(url.as_str(), &[]).await?;
        let _ = self.location_key.set(lookup.key.clone());
        Ok(lookup.key)
    }

    pub async fn poll(&self) -> Result<Weather, ApiError> {
        let loc = self.resolve_location().await?;
        let key = self.resolve_location_key(&loc).await?;

        // Fan out: current conditions + 1-day forecast.
        let conditions_url = reqwest::Url::parse_with_params(
            &format!("https://dataservice.accuweather.com/currentconditions/v1/{key}"),
            &[("apikey", self.cfg.api_key.as_str()), ("details", "true")],
        )
        .map_err(|e| ApiError::Config(format!("accuweather conditions url: {e}")))?;
        let metric = matches!(self.cfg.units.as_str(), "metric");
        let daily_url = reqwest::Url::parse_with_params(
            &format!("https://dataservice.accuweather.com/forecasts/v1/daily/1day/{key}"),
            &[
                ("apikey", self.cfg.api_key.as_str()),
                ("details", "true"),
                ("metric", if metric { "true" } else { "false" }),
            ],
        )
        .map_err(|e| ApiError::Config(format!("accuweather daily url: {e}")))?;

        let (conditions_res, daily_res) = tokio::join!(
            get_json::<Vec<CurrentConditionRaw>>(conditions_url.as_str(), &[]),
            get_json::<DailyResponse>(daily_url.as_str(), &[]),
        );

        let conditions = conditions_res?
            .into_iter()
            .next()
            .ok_or(ApiError::Provider {
                provider: "accuweather",
                msg: "empty current conditions".into(),
            })?;
        let daily = daily_res?;
        let today = daily
            .daily_forecasts
            .into_iter()
            .next()
            .ok_or(ApiError::Provider {
                provider: "accuweather",
                msg: "empty daily forecast".into(),
            })?;

        Ok(normalize(&loc, conditions, today, metric))
    }
}

// ---------------------------------------------------------------------------
// Response shapes — minimal fields we consume. Matches AccuWeather's PascalCase
// JSON via `rename_all`.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GeoPositionResponse {
    key: String,
    #[serde(default)]
    #[allow(dead_code)]
    localized_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CurrentConditionRaw {
    epoch_time: i64,
    weather_text: String,
    weather_icon: u8,
    is_day_time: bool,
    temperature: UnitGroup,
    real_feel_temperature: UnitGroup,
    relative_humidity: u32,
    wind: WindRaw,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UnitGroup {
    metric: Measurement,
    imperial: Measurement,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Measurement {
    value: f32,
    #[allow(dead_code)]
    unit: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WindRaw {
    direction: WindDirectionRaw,
    speed: UnitGroup,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WindDirectionRaw {
    degrees: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DailyResponse {
    daily_forecasts: Vec<DailyForecastRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DailyForecastRaw {
    sun: SunRaw,
    temperature: TempRange,
    day: DayPart,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SunRaw {
    epoch_rise: i64,
    epoch_set: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TempRange {
    minimum: TempValue,
    maximum: TempValue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TempValue {
    value: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DayPart {
    #[serde(rename = "PrecipitationProbability", default)]
    precipitation_probability: u32,
}

// ---------------------------------------------------------------------------
// Normalization.
// ---------------------------------------------------------------------------

fn normalize(
    loc: &GeoLocation,
    conditions: CurrentConditionRaw,
    today: DailyForecastRaw,
    metric: bool,
) -> Weather {
    let (temp, feels_like, wind_speed) = if metric {
        (
            conditions.temperature.metric.value,
            conditions.real_feel_temperature.metric.value,
            conditions.wind.speed.metric.value,
        )
    } else {
        (
            conditions.temperature.imperial.value,
            conditions.real_feel_temperature.imperial.value,
            conditions.wind.speed.imperial.value,
        )
    };

    let icon = icon_for_accuweather_code(conditions.weather_icon, conditions.is_day_time);
    let _now = epoch_to_local(conditions.epoch_time);

    Weather {
        api: WeatherApiSource::AccuWeather,
        lat: loc.lat,
        lon: loc.lon,
        location_name: loc.city.clone(),
        current: CurrentWeather {
            conditions: conditions.weather_text,
            temp,
            feels_like,
            wind_speed,
            humidity: conditions.relative_humidity,
            precipitation_chance: today.day.precipitation_probability,
            uv: None,
            wind_direction_deg: Some(conditions.wind.direction.degrees),
            icon,
        },
        forecast: DayForecast {
            today_high: today.temperature.maximum.value,
            today_low: today.temperature.minimum.value,
            sunrise: epoch_to_local(today.sun.epoch_rise),
            sunset: epoch_to_local(today.sun.epoch_set),
        },
    }
}

fn epoch_to_local(ts: i64) -> DateTime<Local> {
    Local.timestamp_opt(ts, 0).single().unwrap_or_else(Local::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed-but-real-shape AccuWeather response combining the two endpoints
    /// for a single normalization round-trip.
    const CONDITIONS_FIXTURE: &str = r#"
    [{
      "EpochTime": 1700000000,
      "WeatherText": "Mostly sunny",
      "WeatherIcon": 2,
      "IsDayTime": true,
      "Temperature": {
        "Metric": {"Value": 18.3, "Unit": "C", "UnitType": 17},
        "Imperial": {"Value": 65.0, "Unit": "F", "UnitType": 18}
      },
      "RealFeelTemperature": {
        "Metric": {"Value": 17.2, "Unit": "C", "UnitType": 17},
        "Imperial": {"Value": 63.0, "Unit": "F", "UnitType": 18}
      },
      "RelativeHumidity": 72,
      "Wind": {
        "Direction": {"Degrees": 270, "Localized": "W", "English": "W"},
        "Speed": {
          "Metric": {"Value": 13.7, "Unit": "km/h", "UnitType": 7},
          "Imperial": {"Value": 8.5, "Unit": "mi/h", "UnitType": 9}
        }
      }
    }]
    "#;

    const DAILY_FIXTURE: &str = r#"
    {
      "DailyForecasts": [{
        "Sun": {"EpochRise": 1699970000, "EpochSet": 1700010000, "Rise": "x", "Set": "y"},
        "Temperature": {
          "Minimum": {"Value": 55.0, "Unit": "F", "UnitType": 18},
          "Maximum": {"Value": 70.0, "Unit": "F", "UnitType": 18}
        },
        "Day": {"PrecipitationProbability": 10}
      }]
    }
    "#;

    #[test]
    fn normalizes_combined_response() {
        let conditions: Vec<CurrentConditionRaw> = serde_json::from_str(CONDITIONS_FIXTURE).unwrap();
        let daily: DailyResponse = serde_json::from_str(DAILY_FIXTURE).unwrap();
        let loc = GeoLocation { lat: 37.7749, lon: -122.4194, city: "San Francisco".into() };
        let w = normalize(
            &loc,
            conditions.into_iter().next().unwrap(),
            daily.daily_forecasts.into_iter().next().unwrap(),
            false, // imperial
        );
        assert_eq!(w.api, WeatherApiSource::AccuWeather);
        assert_eq!(w.location_name, "San Francisco");
        assert_eq!(w.current.temp, 65.0);
        assert_eq!(w.current.feels_like, 63.0);
        assert_eq!(w.current.humidity, 72);
        assert_eq!(w.current.precipitation_chance, 10);
        assert_eq!(w.forecast.today_high, 70.0);
        assert_eq!(w.forecast.today_low, 55.0);
        assert_eq!(w.current.icon.condition, "Sunny");
    }

    #[test]
    fn metric_path() {
        let conditions: Vec<CurrentConditionRaw> = serde_json::from_str(CONDITIONS_FIXTURE).unwrap();
        let daily: DailyResponse = serde_json::from_str(DAILY_FIXTURE).unwrap();
        let loc = GeoLocation { lat: 0.0, lon: 0.0, city: "X".into() };
        let w = normalize(
            &loc,
            conditions.into_iter().next().unwrap(),
            daily.daily_forecasts.into_iter().next().unwrap(),
            true, // metric
        );
        assert_eq!(w.current.temp, 18.3);
        assert_eq!(w.current.wind_speed, 13.7);
    }
}
