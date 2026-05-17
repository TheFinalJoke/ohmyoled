//! Pirate Weather provider.
//!
//! Pirate Weather is a drop-in replacement for the retired DarkSky API; the
//! response shape is identical. We hit `https://api.pirateweather.net/forecast/{key}/{lat},{lon}`
//! and pull the fields we need from `currently` and `daily.data[0]`.
//!
//! No crates.io client crate exists for Pirate Weather (or for the historical
//! DarkSky shape with current maintenance), so this is a thin async client
//! built on the workspace's `reqwest 0.12` + `serde_json`.

use super::geo::{lookup_ipinfo, GeoLocation};
use super::icon_table::icon_for_pirate_icon;
use super::model::{CurrentWeather, DayForecast, Weather, WeatherApiSource};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct PirateWeatherConfig {
    pub api_key: String,
    /// `"imperial"`, `"metric"`, `"si"`, `"ca"`, or `"uk2"` — passed straight
    /// to Pirate as the `units` query param.
    pub units: String,
    pub use_current_location: bool,
    pub ipinfo_token: Option<String>,
}

pub struct PirateWeatherClient {
    cfg: PirateWeatherConfig,
    geo: Arc<OnceCell<GeoLocation>>,
}

impl PirateWeatherClient {
    pub fn new(cfg: PirateWeatherConfig) -> Result<Self, ApiError> {
        if cfg.api_key.is_empty() {
            return Err(ApiError::Config("pirate: api_key missing".to_string()));
        }
        Ok(Self {
            cfg,
            geo: Arc::new(OnceCell::new()),
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

    pub async fn poll(&self) -> Result<Weather, ApiError> {
        let loc = self.resolve_location().await?;
        let url = format!(
            "https://api.pirateweather.net/forecast/{}/{},{}?units={}",
            self.cfg.api_key, loc.lat, loc.lon, self.cfg.units
        );
        let raw: ForecastResponse = get_json(&url, &[]).await?;
        Ok(normalize(raw, &loc))
    }
}

// ---------------------------------------------------------------------------
// DarkSky-shape response. Only the fields we actually read are listed.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ForecastResponse {
    #[serde(default)]
    latitude: f64,
    #[serde(default)]
    longitude: f64,
    currently: Currently,
    daily: Daily,
}

#[derive(Debug, Deserialize)]
struct Currently {
    time: i64,
    summary: Option<String>,
    icon: Option<String>,
    temperature: f32,
    #[serde(rename = "apparentTemperature", default)]
    apparent_temperature: Option<f32>,
    /// Pirate Weather returns humidity as a 0–1 fraction.
    humidity: f32,
    #[serde(rename = "windSpeed", default)]
    wind_speed: f32,
    #[serde(rename = "windBearing", default)]
    wind_bearing: Option<f32>,
    #[serde(rename = "uvIndex", default)]
    uv_index: Option<f32>,
    #[serde(rename = "precipProbability", default)]
    precip_probability: f32,
}

#[derive(Debug, Deserialize)]
struct Daily {
    data: Vec<DailyDay>,
}

#[derive(Debug, Deserialize)]
struct DailyDay {
    #[serde(rename = "temperatureMax", default)]
    temperature_max: f32,
    #[serde(rename = "temperatureMin", default)]
    temperature_min: f32,
    #[serde(rename = "sunriseTime", default)]
    sunrise_time: i64,
    #[serde(rename = "sunsetTime", default)]
    sunset_time: i64,
}

fn normalize(raw: ForecastResponse, loc: &GeoLocation) -> Weather {
    let today = raw.daily.data.first();
    let now = epoch_to_local(raw.currently.time);
    let sunrise = today
        .map(|d| epoch_to_local(d.sunrise_time))
        .unwrap_or(now);
    let sunset = today
        .map(|d| epoch_to_local(d.sunset_time))
        .unwrap_or(now);
    let is_day = now < sunset;

    let icon_str = raw.currently.icon.as_deref().unwrap_or("clear-day");
    let icon = icon_for_pirate_icon(icon_str, is_day);

    Weather {
        api: WeatherApiSource::PirateWeather,
        lat: if raw.latitude == 0.0 { loc.lat } else { raw.latitude },
        lon: if raw.longitude == 0.0 { loc.lon } else { raw.longitude },
        location_name: loc.city.clone(),
        current: CurrentWeather {
            conditions: raw
                .currently
                .summary
                .unwrap_or_else(|| icon_str.to_string()),
            temp: raw.currently.temperature,
            feels_like: raw
                .currently
                .apparent_temperature
                .unwrap_or(raw.currently.temperature),
            wind_speed: raw.currently.wind_speed,
            humidity: (raw.currently.humidity * 100.0).round() as u32,
            precipitation_chance: (raw.currently.precip_probability * 100.0).round() as u32,
            uv: raw.currently.uv_index,
            wind_direction_deg: raw.currently.wind_bearing,
            icon,
        },
        forecast: DayForecast {
            today_high: today.map(|d| d.temperature_max).unwrap_or(0.0),
            today_low: today.map(|d| d.temperature_min).unwrap_or(0.0),
            sunrise,
            sunset,
        },
    }
}

fn epoch_to_local(ts: i64) -> DateTime<Local> {
    Local.timestamp_opt(ts, 0).single().unwrap_or_else(Local::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-shape Pirate Weather payload, trimmed to only the fields we consume.
    const FIXTURE: &str = r#"
    {
      "latitude": 37.7749,
      "longitude": -122.4194,
      "currently": {
        "time": 1700000000,
        "summary": "Clear",
        "icon": "clear-day",
        "temperature": 65.0,
        "apparentTemperature": 63.0,
        "humidity": 0.72,
        "windSpeed": 8.5,
        "windBearing": 270,
        "uvIndex": 5,
        "precipProbability": 0.1
      },
      "daily": {
        "data": [{
          "temperatureMax": 70.0,
          "temperatureMin": 55.0,
          "sunriseTime": 1699970000,
          "sunsetTime": 1700010000
        }]
      }
    }
    "#;

    #[test]
    fn parses_pirate_fixture() {
        let raw: ForecastResponse = serde_json::from_str(FIXTURE).unwrap();
        let loc = GeoLocation { lat: 37.7749, lon: -122.4194, city: "San Francisco".into() };
        let w = normalize(raw, &loc);
        assert_eq!(w.api, WeatherApiSource::PirateWeather);
        assert_eq!(w.current.temp, 65.0);
        assert_eq!(w.current.feels_like, 63.0);
        assert_eq!(w.current.humidity, 72);
        assert_eq!(w.current.precipitation_chance, 10);
        assert_eq!(w.current.icon.condition, "Sunny");
        assert_eq!(w.forecast.today_high, 70.0);
        assert_eq!(w.forecast.today_low, 55.0);
    }

    #[test]
    fn missing_summary_falls_back_to_icon() {
        let no_summary = FIXTURE.replace(r#""summary": "Clear","#, "");
        let raw: ForecastResponse = serde_json::from_str(&no_summary).unwrap();
        let w = normalize(raw, &GeoLocation { lat: 0.0, lon: 0.0, city: "X".into() });
        assert_eq!(w.current.conditions, "clear-day");
    }
}
