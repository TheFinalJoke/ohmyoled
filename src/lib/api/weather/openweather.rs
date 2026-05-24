//! OpenWeatherMap OneCall 3.0 provider.
//!
//! Direct port of `src/python/ohmyoled/lib/weather/openweather/weather.py`.
//! Maps the OneCall JSON into the normalized [`Weather`] type.

use super::geo::{lookup_ipinfo, GeoLocation};
use super::icon_table::icon_for_owm_code;
use super::model::{
    CurrentWeather, DailyForecast, DayForecast, HourlyForecast, Weather, WeatherApiSource,
};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct OpenWeatherConfig {
    pub api_key: String,
    pub units: String, // "imperial" or "metric"
    pub use_current_location: bool,
    pub ipinfo_token: Option<String>,
    /// Fallback city name when `use_current_location` is false.
    pub city: Option<String>,
}

pub struct OpenWeatherClient {
    cfg: OpenWeatherConfig,
    geo: Arc<OnceCell<GeoLocation>>,
}

impl OpenWeatherClient {
    pub fn new(cfg: OpenWeatherConfig) -> Result<Self, ApiError> {
        if cfg.api_key.is_empty() {
            return Err(ApiError::Config(
                "openweather: api_key missing".to_string(),
            ));
        }
        Ok(Self {
            cfg,
            geo: Arc::new(OnceCell::new()),
        })
    }

    /// Resolve lat/lon (cached after first successful resolution).
    async fn resolve_location(&self) -> Result<GeoLocation, ApiError> {
        if let Some(loc) = self.geo.get() {
            return Ok(loc.clone());
        }
        let loc = if self.cfg.use_current_location {
            let token = self
                .cfg
                .ipinfo_token
                .as_deref()
                .unwrap_or("");
            lookup_ipinfo(token).await?
        } else {
            let city = self
                .cfg
                .city
                .as_deref()
                .ok_or_else(|| ApiError::Config("openweather: city required when current_location=false".into()))?;
            geocode_city(city, &self.cfg.api_key).await?
        };
        let _ = self.geo.set(loc.clone());
        Ok(loc)
    }

    pub async fn poll(&self) -> Result<Weather, ApiError> {
        let loc = self.resolve_location().await?;
        let url = format!(
            "https://api.openweathermap.org/data/3.0/onecall?lat={}&lon={}&appid={}&units={}",
            loc.lat, loc.lon, self.cfg.api_key, self.cfg.units
        );
        let raw: OneCallResponse = get_json(&url, &[]).await?;
        normalize(raw, &loc)
    }
}

// ---------------------------------------------------------------------------
// One-shot geocode by city name (used when not using ipinfo).
// Mirrors `get_long_and_lat` in the Python provider.
// ---------------------------------------------------------------------------

async fn geocode_city(city: &str, api_key: &str) -> Result<GeoLocation, ApiError> {
    #[derive(Deserialize)]
    struct GeoCoord {
        lat: f64,
        lon: f64,
    }
    #[derive(Deserialize)]
    struct CityLookup {
        name: String,
        coord: GeoCoord,
    }
    let url = reqwest::Url::parse_with_params(
        "https://api.openweathermap.org/data/2.5/weather",
        &[("q", city), ("appid", api_key)],
    )
    .map_err(|e| ApiError::Config(format!("openweather geocode url: {e}")))?;
    let r: CityLookup = get_json(url.as_str(), &[]).await?;
    Ok(GeoLocation {
        lat: r.coord.lat,
        lon: r.coord.lon,
        city: r.name,
    })
}

// ---------------------------------------------------------------------------
// Raw OneCall 3.0 response shape — only the fields we consume.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OneCallResponse {
    #[serde(default)]
    lat: f64,
    #[serde(default)]
    lon: f64,
    current: CurrentRaw,
    daily: Vec<DailyRaw>,
    #[serde(default)]
    hourly: Vec<HourlyRaw>,
}

#[derive(Debug, Deserialize)]
struct CurrentRaw {
    dt: i64,
    sunrise: i64,
    sunset: i64,
    temp: f32,
    feels_like: f32,
    humidity: u32,
    wind_speed: f32,
    #[serde(default)]
    wind_deg: Option<f32>,
    weather: Vec<WeatherEntry>,
}

#[derive(Debug, Deserialize)]
struct WeatherEntry {
    id: u16,
    main: String,
}

#[derive(Debug, Deserialize)]
struct DailyRaw {
    dt: i64,
    temp: DailyTemp,
    #[serde(default)]
    pop: f32,
    #[serde(default)]
    uvi: f32,
    #[serde(default)]
    weather: Vec<WeatherEntry>,
}

#[derive(Debug, Deserialize)]
struct DailyTemp {
    min: f32,
    max: f32,
}

#[derive(Debug, Deserialize)]
struct HourlyRaw {
    dt: i64,
    temp: f32,
    #[serde(default)]
    pop: f32,
}

fn normalize(raw: OneCallResponse, loc: &GeoLocation) -> Result<Weather, ApiError> {
    let weather_entry = raw.current.weather.first().ok_or(ApiError::Provider {
        provider: "openweather",
        msg: "empty weather array".into(),
    })?;
    let today = raw.daily.first().ok_or(ApiError::Provider {
        provider: "openweather",
        msg: "empty daily forecast".into(),
    })?;

    let sunrise = ts_to_local(raw.current.sunrise);
    let sunset = ts_to_local(raw.current.sunset);
    let now = ts_to_local(raw.current.dt);
    let is_day = now < sunset;

    let icon = icon_for_owm_code(weather_entry.id, is_day);

    // Skip index 0 (today is already captured in `forecast.today_*`), take next 5.
    let daily = raw
        .daily
        .iter()
        .skip(1)
        .take(5)
        .map(|d| {
            let day_icon = d
                .weather
                .first()
                .map(|w| icon_for_owm_code(w.id, true))
                .unwrap_or(icon);
            DailyForecast {
                date: ts_to_local(d.dt).date_naive(),
                high: d.temp.max,
                low: d.temp.min,
                icon: day_icon,
                precipitation_chance: (d.pop * 100.0).round() as u32,
            }
        })
        .collect();

    let hourly = raw
        .hourly
        .iter()
        .take(12)
        .map(|h| HourlyForecast {
            time: ts_to_local(h.dt),
            temp: h.temp,
            precipitation_chance: (h.pop * 100.0).round() as u32,
        })
        .collect();

    Ok(Weather {
        api: WeatherApiSource::OpenWeather,
        lat: if raw.lat == 0.0 { loc.lat } else { raw.lat },
        lon: if raw.lon == 0.0 { loc.lon } else { raw.lon },
        location_name: loc.city.clone(),
        current: CurrentWeather {
            conditions: weather_entry.main.clone(),
            temp: raw.current.temp,
            feels_like: raw.current.feels_like,
            wind_speed: raw.current.wind_speed,
            humidity: raw.current.humidity,
            precipitation_chance: (today.pop * 100.0).round() as u32,
            uv: Some(today.uvi),
            wind_direction_deg: raw.current.wind_deg,
            icon,
        },
        forecast: DayForecast {
            today_high: today.temp.max,
            today_low: today.temp.min,
            sunrise,
            sunset,
        },
        hourly,
        daily,
    })
}

fn ts_to_local(ts: i64) -> DateTime<Local> {
    Local.timestamp_opt(ts, 0).single().unwrap_or_else(Local::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-shape OneCall 3.0 response (trimmed to what the parser actually reads).
    const FIXTURE: &str = r#"
    {
      "lat": 37.7749,
      "lon": -122.4194,
      "current": {
        "dt": 1700000000,
        "sunrise": 1699970000,
        "sunset": 1700010000,
        "temp": 65.0,
        "feels_like": 63.0,
        "humidity": 72,
        "wind_speed": 8.5,
        "wind_deg": 270,
        "weather": [{"id": 800, "main": "Clear"}]
      },
      "daily": [
        {"dt": 1700000000, "temp": {"min": 55.0, "max": 70.0}, "pop": 0.1, "uvi": 5.2,
         "weather": [{"id": 800, "main": "Clear"}]},
        {"dt": 1700086400, "temp": {"min": 54.0, "max": 71.0}, "pop": 0.2, "uvi": 5.5,
         "weather": [{"id": 801, "main": "Clouds"}]},
        {"dt": 1700172800, "temp": {"min": 53.0, "max": 69.0}, "pop": 0.4, "uvi": 4.8,
         "weather": [{"id": 500, "main": "Rain"}]},
        {"dt": 1700259200, "temp": {"min": 52.0, "max": 67.0}, "pop": 0.6, "uvi": 4.5,
         "weather": [{"id": 500, "main": "Rain"}]},
        {"dt": 1700345600, "temp": {"min": 55.0, "max": 70.0}, "pop": 0.1, "uvi": 5.0,
         "weather": [{"id": 800, "main": "Clear"}]},
        {"dt": 1700432000, "temp": {"min": 56.0, "max": 72.0}, "pop": 0.0, "uvi": 5.2,
         "weather": [{"id": 800, "main": "Clear"}]}
      ],
      "hourly": [
        {"dt": 1700000000, "temp": 65.0, "pop": 0.0},
        {"dt": 1700003600, "temp": 66.0, "pop": 0.0},
        {"dt": 1700007200, "temp": 67.0, "pop": 0.1},
        {"dt": 1700010800, "temp": 66.0, "pop": 0.2},
        {"dt": 1700014400, "temp": 65.0, "pop": 0.4},
        {"dt": 1700018000, "temp": 64.0, "pop": 0.5}
      ]
    }
    "#;

    #[test]
    fn parses_onecall_fixture() {
        let raw: OneCallResponse = serde_json::from_str(FIXTURE).unwrap();
        let loc = GeoLocation { lat: 37.7749, lon: -122.4194, city: "San Francisco".into() };
        let w = normalize(raw, &loc).unwrap();
        assert_eq!(w.api, WeatherApiSource::OpenWeather);
        assert_eq!(w.location_name, "San Francisco");
        assert_eq!(w.current.temp, 65.0);
        assert_eq!(w.current.humidity, 72);
        assert_eq!(w.current.precipitation_chance, 10);
        assert_eq!(w.forecast.today_high, 70.0);
        assert_eq!(w.forecast.today_low, 55.0);
        // Conditions string preserves OWM's `main` value verbatim.
        assert_eq!(w.current.conditions, "Clear");
        // Day-time code 800 picks the SUNNY icon entry.
        assert_eq!(w.current.icon.condition, "Sunny");
        // Daily skips index 0 (today is in `forecast.today_*`), takes next 5.
        assert_eq!(w.daily.len(), 5);
        assert_eq!(w.daily[0].high, 71.0);
        assert_eq!(w.daily[0].precipitation_chance, 20);
        // Hourly takes the first 12 entries (or all if fewer).
        assert_eq!(w.hourly.len(), 6);
        assert_eq!(w.hourly[2].temp, 67.0);
        assert_eq!(w.hourly[2].precipitation_chance, 10);
    }
}
