//! National Weather Service provider.
//!
//! Direct port of `src/python/ohmyoled/lib/weather/weathergov/nws.py`. NWS
//! requires a two-step fetch: `points/{lat},{lon}` returns URLs for the
//! actual forecast/hourly/observation endpoints. We parallelize the second
//! step with `tokio::join!`.

use super::geo::{lookup_ipinfo, GeoLocation};
use super::icon_table::icon_for_nws_condition;
use super::model::{CurrentWeather, DayForecast, Weather, WeatherApiSource};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use chrono::{DateTime, Local, NaiveDate, TimeZone};
use serde::Deserialize;
use std::sync::Arc;
use sunrise::{Coordinates, SolarDay, SolarEvent};
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct NwsConfig {
    pub ipinfo_token: Option<String>,
}

pub struct NwsClient {
    cfg: NwsConfig,
    geo: Arc<OnceCell<GeoLocation>>,
}

impl NwsClient {
    pub fn new(cfg: NwsConfig) -> Self {
        Self {
            cfg,
            geo: Arc::new(OnceCell::new()),
        }
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

        let points_url = format!("https://api.weather.gov/points/{},{}", loc.lat, loc.lon);
        let points: PointsResponse = get_json(
            &points_url,
            &[("User-Agent", "ohmyoled/2.2.8"), ("Accept", "application/geo+json")],
        )
        .await?;

        let observation_url = format!(
            "https://api.weather.gov/stations/{}/observations/latest",
            points.properties.radar_station
        );

        // Parallel fan-out: hourly (for periods + conditions) + observation (for sensor data).
        let headers: &[(&str, &str)] = &[
            ("User-Agent", "ohmyoled/2.2.8"),
            ("Accept", "application/geo+json"),
        ];
        let (hourly_res, obs_res) = tokio::join!(
            get_json::<ForecastResponse>(&points.properties.forecast_hourly, headers),
            get_json::<ObservationResponse>(&observation_url, headers),
        );

        let hourly = hourly_res?;
        let obs = obs_res?;

        normalize(&loc, &points, &hourly, &obs)
    }
}

// ---------------------------------------------------------------------------
// Response shapes — only the fields we actually consume.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct PointsResponse {
    properties: PointsProperties,
    geometry: PointsGeometry,
}

#[derive(Debug, Deserialize)]
struct PointsProperties {
    #[serde(rename = "forecastHourly")]
    forecast_hourly: String,
    #[serde(rename = "radarStation")]
    radar_station: String,
    #[serde(rename = "relativeLocation")]
    relative_location: RelativeLocation,
}

#[derive(Debug, Deserialize)]
struct RelativeLocation {
    properties: RelativeLocationProps,
}

#[derive(Debug, Deserialize)]
struct RelativeLocationProps {
    city: String,
}

#[derive(Debug, Deserialize)]
struct PointsGeometry {
    /// [lon, lat] per GeoJSON convention.
    coordinates: (f64, f64),
}

#[derive(Debug, Deserialize)]
struct ForecastResponse {
    properties: ForecastProperties,
}

#[derive(Debug, Deserialize)]
struct ForecastProperties {
    periods: Vec<ForecastPeriod>,
}

#[derive(Debug, Deserialize)]
struct ForecastPeriod {
    #[serde(default)]
    temperature: Option<i32>,
    #[serde(rename = "startTime", default)]
    start_time: Option<String>,
    #[serde(rename = "shortForecast", default)]
    short_forecast: String,
}

#[derive(Debug, Deserialize)]
struct ObservationResponse {
    properties: ObservationProperties,
}

#[derive(Debug, Deserialize)]
struct ObservationProperties {
    /// Temperature in Celsius.
    #[serde(default)]
    temperature: Measurement,
    #[serde(rename = "relativeHumidity", default)]
    relative_humidity: Measurement,
    #[serde(rename = "windSpeed", default)]
    wind_speed: Measurement,
    #[serde(rename = "windDirection", default)]
    wind_direction: Measurement,
}

#[derive(Debug, Default, Deserialize)]
struct Measurement {
    #[serde(default)]
    value: Option<f64>,
}

// ---------------------------------------------------------------------------
// Normalization.
// ---------------------------------------------------------------------------

fn normalize(
    loc: &GeoLocation,
    points: &PointsResponse,
    hourly: &ForecastResponse,
    obs: &ObservationResponse,
) -> Result<Weather, ApiError> {
    // Coords come back as [lon, lat] per GeoJSON.
    let (lon, lat) = points.geometry.coordinates;
    let city = points.properties.relative_location.properties.city.clone();
    let _ = loc; // place-name from points endpoint preferred; geo loc kept for fallback

    let current_period = hourly
        .properties
        .periods
        .first()
        .ok_or(ApiError::Provider {
            provider: "nws",
            msg: "empty hourly periods".into(),
        })?;

    // Observations are in metric — convert.
    let temp_f = obs
        .properties
        .temperature
        .value
        .map(|c| (c * 1.8) + 32.0)
        .unwrap_or(current_period.temperature.unwrap_or(0) as f64) as f32;
    let humidity = obs
        .properties
        .relative_humidity
        .value
        .map(|h| h.round() as u32)
        .unwrap_or(0);
    let wind_mph = obs
        .properties
        .wind_speed
        .value
        .map(|kmh| kmh / 1.609_344)
        .unwrap_or(0.0) as f32;
    let wind_deg = obs.properties.wind_direction.value.map(|d| d as f32);

    // Today's high/low: scan all hourly periods whose `startTime` date matches today.
    let now_local = Local::now();
    let today = now_local.date_naive();
    let mut today_temps = hourly
        .properties
        .periods
        .iter()
        .filter_map(|p| {
            let t = p.temperature?;
            let st = p.start_time.as_deref()?;
            let parsed = chrono::DateTime::parse_from_rfc3339(st).ok()?;
            if parsed.with_timezone(&Local).date_naive() == today {
                Some(t)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    today_temps.sort_unstable();
    let today_low = today_temps.first().copied().unwrap_or(0) as f32;
    let today_high = today_temps.last().copied().unwrap_or(0) as f32;

    // Sunrise/sunset from lat/lon (NWS doesn't provide these).
    let (sunrise, sunset) = sunrise_sunset_local(lat, lon, today)?;

    let is_day = now_local < sunset;
    let icon = icon_for_nws_condition(&current_period.short_forecast, is_day);

    Ok(Weather {
        api: WeatherApiSource::Nws,
        lat,
        lon,
        location_name: city,
        current: CurrentWeather {
            conditions: current_period.short_forecast.clone(),
            temp: temp_f,
            feels_like: temp_f, // NWS doesn't supply feels-like; mirror Python's fallback.
            wind_speed: wind_mph,
            humidity,
            precipitation_chance: 0,
            uv: None,
            wind_direction_deg: wind_deg,
            icon,
        },
        forecast: DayForecast {
            today_high,
            today_low,
            sunrise,
            sunset,
        },
    })
}

fn sunrise_sunset_local(
    lat: f64,
    lon: f64,
    day: NaiveDate,
) -> Result<(DateTime<Local>, DateTime<Local>), ApiError> {
    let coords = Coordinates::new(lat, lon).ok_or(ApiError::Provider {
        provider: "nws",
        msg: format!("invalid coordinates {lat},{lon}"),
    })?;
    let solar = SolarDay::new(coords, day);
    let sunrise = solar
        .event_time(SolarEvent::Sunrise)
        .ok_or(ApiError::Provider {
            provider: "nws",
            msg: "no sunrise on this date".into(),
        })?;
    let sunset = solar
        .event_time(SolarEvent::Sunset)
        .ok_or(ApiError::Provider {
            provider: "nws",
            msg: "no sunset on this date".into(),
        })?;
    let sunrise = Local
        .timestamp_opt(sunrise.timestamp(), 0)
        .single()
        .unwrap_or_else(Local::now);
    let sunset = Local
        .timestamp_opt(sunset.timestamp(), 0)
        .single()
        .unwrap_or_else(Local::now);
    Ok((sunrise, sunset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sunrise_sets_for_known_location() {
        // San Francisco, June 21 — sun rises ~5:48 PDT, sets ~20:34 PDT.
        let day = NaiveDate::from_ymd_opt(2024, 6, 21).unwrap();
        let (sr, ss) = sunrise_sunset_local(37.7749, -122.4194, day).unwrap();
        assert!(sr < ss);
        // At least 12 hours of daylight on summer solstice in SF.
        assert!((ss - sr).num_seconds() > 12 * 3600);
    }
}
