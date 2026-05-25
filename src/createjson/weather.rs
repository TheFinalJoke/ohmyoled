use crate::createjson::ui;
use oledlib::api;
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherFormat {
    Imperial,
    Metric,
}

impl WeatherFormat {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Imperial => "imperial",
            Self::Metric => "metric",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WeatherOptions {
    pub run: bool,
    pub api: api::WeatherApi,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub api_key: Option<String>,
    pub current_location: bool,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub city: Option<String>,
    #[serde(default)]
    pub weather_format: Option<WeatherFormat>,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub current_location_api_key: Option<String>,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for WeatherOptions {
    fn default() -> Self {
        WeatherOptions {
            run: true,
            api: api::WeatherApi::Nws,
            api_key: None,
            current_location: true,
            city: None,
            weather_format: Some(WeatherFormat::Imperial),
            current_location_api_key: None,
            cache_ttl_secs: None,
        }
    }
}

fn pick_api() -> api::WeatherApi {
    let slug = ui::choose(
        "Which weather provider?",
        &[
            ("nws", "US National Weather Service — no key"),
            ("openweather", "OpenWeather — key required"),
            ("accuweather", "AccuWeather — key required"),
            ("pirate", "Pirate Weather — key required"),
        ],
        "nws",
    );
    match slug.as_str() {
        "nws" => api::WeatherApi::Nws,
        "openweather" => api::WeatherApi::Openweather,
        "accuweather" => api::WeatherApi::Accuweather,
        "pirate" => api::WeatherApi::Pirate,
        _ => api::WeatherApi::Nws,
    }
}

fn pick_format() -> Option<WeatherFormat> {
    let slug = ui::choose(
        "Units",
        &[
            ("imperial", "°F / mph"),
            ("metric", "°C / m·s⁻¹"),
        ],
        "imperial",
    );
    Some(match slug.as_str() {
        "metric" => WeatherFormat::Metric,
        _ => WeatherFormat::Imperial,
    })
}

fn configure_location() -> (bool, Option<String>, Option<String>) {
    let use_current = ui::read_yes_no(
        "Geolocate via ipinfo? (otherwise prompt for city)",
        true,
    );
    if use_current {
        ui::hint("Optional: paste an ipinfo.io token for higher rate limits (blank to skip).");
        let token = ui::read_line_default("ipinfo token", "");
        let token = if token.trim().is_empty() { None } else { Some(token) };
        (true, None, token)
    } else {
        let city = ui::read_required("City (e.g. 'Dallas, TX')");
        (false, Some(city), None)
    }
}

pub fn configure() -> Result<WeatherOptions, String> {
    ui::section("Weather");
    ui::hint("Pick a provider, set your location, and choose units.");

    if ui::read_yes_no("Use defaults (NWS, auto-locate, imperial)?", true) {
        ui::success("Weather — NWS defaults");
        return Ok(WeatherOptions::default());
    }

    let api_choice = pick_api();
    let needs_key = !matches!(api_choice, api::WeatherApi::Nws);
    let api_key = if needs_key {
        ui::hint(&format!(
            "{} requires an API key. Get one from the provider, then paste it here.",
            api_choice.get_api()
        ));
        Some(ui::read_required("API key"))
    } else {
        None
    };

    let (current_location, city, ipinfo_token) = configure_location();
    let weather_format = pick_format();
    let cache_ttl_secs = ui::read_cache_ttl_secs();

    ui::success(&format!(
        "Weather — {} ({}, {})",
        api_choice.get_api(),
        if current_location { "auto-locate" } else { city.as_deref().unwrap_or("city") },
        weather_format.map(WeatherFormat::slug).unwrap_or("imperial"),
    ));

    Ok(WeatherOptions {
        run: true,
        api: api_choice,
        api_key,
        current_location,
        city,
        weather_format,
        current_location_api_key: ipinfo_token,
        cache_ttl_secs,
    })
}

pub fn summary_line(opts: &WeatherOptions) -> String {
    let where_at = if opts.current_location {
        "auto-locate".to_string()
    } else {
        opts.city.clone().unwrap_or_else(|| "?".to_string())
    };
    format!(
        "weather: {} — {} ({})",
        opts.api.get_api(),
        where_at,
        opts.weather_format.map(WeatherFormat::slug).unwrap_or("imperial"),
    )
}
