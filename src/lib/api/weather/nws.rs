//! National Weather Service provider.
//!
//! Direct port of `src/python/ohmyoled/lib/weather/weathergov/nws.py`. NWS
//! requires a two-step fetch: `points/{lat},{lon}` returns URLs for the
//! actual forecast/hourly/observation endpoints. We parallelize the second
//! step with `tokio::join!`.

use super::geo::{lookup_ipinfo, GeoLocation};
use super::icon_table::icon_for_nws_condition;
use super::model::{
    CurrentWeather, DailyForecast, DayForecast, HourlyForecast, Weather, WeatherApiSource,
};
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

        // Parallel fan-out: hourly (current + precip bars) + daily forecast
        // (5-day strip) + observation (sensor data).
        let headers: &[(&str, &str)] = &[
            ("User-Agent", "ohmyoled/2.2.8"),
            ("Accept", "application/geo+json"),
        ];
        let (hourly_res, daily_res, obs_res) = tokio::join!(
            get_json::<ForecastResponse>(&points.properties.forecast_hourly, headers),
            get_json::<ForecastResponse>(&points.properties.forecast, headers),
            get_json::<ObservationResponse>(&observation_url, headers),
        );

        let hourly = hourly_res?;
        // Daily is best-effort — if it 500s, the 5-day screen just skips.
        let daily = daily_res.ok();
        let obs = obs_res?;

        normalize(&loc, &points, &hourly, daily.as_ref(), &obs)
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
    #[serde(rename = "forecast")]
    forecast: String,
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
    /// Day vs night marker on the daily endpoint. Hourly periods set this too
    /// but it isn't meaningful there.
    #[serde(rename = "isDaytime", default)]
    is_daytime: bool,
    /// `probabilityOfPrecipitation` is `{value: int|null, unitCode: "wmoUnit:percent"}`.
    /// `default` lets the field be absent (older endpoints) without erroring.
    #[serde(rename = "probabilityOfPrecipitation", default)]
    pop: ProbabilityOfPrecip,
}

#[derive(Debug, Default, Deserialize)]
struct ProbabilityOfPrecip {
    #[serde(default)]
    value: Option<u32>,
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
    daily: Option<&ForecastResponse>,
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

    // Hourly precip bars: parse next 12 entries.
    let hourly_forecast = hourly
        .properties
        .periods
        .iter()
        .take(12)
        .filter_map(|p| {
            let start = p.start_time.as_deref()?;
            let parsed = chrono::DateTime::parse_from_rfc3339(start).ok()?;
            Some(HourlyForecast {
                time: parsed.with_timezone(&Local),
                temp: p.temperature.unwrap_or(0) as f32,
                precipitation_chance: p.pop.value.unwrap_or(0),
            })
        })
        .collect();

    // Daily 5-day strip: pair each "day" period with its matching "night"
    // period on the same date so we get a high (day) + low (night).
    let daily_forecast = daily.map(|d| collect_daily(&d.properties.periods)).unwrap_or_default();

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
        hourly: hourly_forecast,
        daily: daily_forecast,
    })
}

/// NWS daily endpoint returns alternating day/night periods. We pair them
/// by date, take the day's temperature as the high and the night's as the
/// low, and skip "today" so the strip shows upcoming days only.
fn collect_daily(periods: &[ForecastPeriod]) -> Vec<DailyForecast> {
    use std::collections::BTreeMap;
    let today = Local::now().date_naive();
    let mut by_date: BTreeMap<NaiveDate, (Option<&ForecastPeriod>, Option<&ForecastPeriod>)> =
        BTreeMap::new();

    for p in periods {
        let Some(start) = p.start_time.as_deref() else { continue };
        let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(start) else { continue };
        let date = parsed.with_timezone(&Local).date_naive();
        let entry = by_date.entry(date).or_default();
        if p.is_daytime {
            entry.0 = Some(p);
        } else {
            entry.1 = Some(p);
        }
    }

    by_date
        .into_iter()
        .filter(|(date, _)| *date > today)
        .take(5)
        .map(|(date, (day, night))| {
            let high = day.and_then(|p| p.temperature).unwrap_or(0) as f32;
            let low = night.and_then(|p| p.temperature).unwrap_or(0) as f32;
            let conditions = day.map(|p| p.short_forecast.as_str()).unwrap_or("");
            let pop = day
                .and_then(|p| p.pop.value)
                .or_else(|| night.and_then(|p| p.pop.value))
                .unwrap_or(0);
            DailyForecast {
                date,
                high,
                low,
                icon: icon_for_nws_condition(conditions, true),
                precipitation_chance: pop,
            }
        })
        .collect()
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
