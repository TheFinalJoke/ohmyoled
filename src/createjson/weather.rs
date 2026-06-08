use crate::createjson::tui::field::{FieldDef, FieldKind};
use oledlib::api;
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherFormat {
    Imperial,
    Metric,
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

/// TUI form schema. Conditional fields:
/// - `api_key` shown/required only when the provider isn't NWS.
/// - `city` shown/required only when not auto-locating.
/// - `current_location_api_key` (ipinfo token) shown only when auto-locating.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "api",
            "Provider",
            "Weather data source.",
            FieldKind::Enum {
                default: "nws",
                choices: &[
                    ("nws", "US National Weather Service — no key"),
                    ("openweather", "OpenWeather — key required"),
                    ("accuweather", "AccuWeather — key required"),
                    ("pirate", "Pirate Weather — key required"),
                ],
            },
        ),
        FieldDef::new(
            "api_key",
            "API key",
            "Required for every provider except NWS.",
            FieldKind::Text { default: "" },
        )
        .when(|f| f.enum_slug("api") != Some("nws")),
        FieldDef::new(
            "current_location",
            "Auto-locate (ipinfo)",
            "Geolocate via ipinfo instead of a fixed city.",
            FieldKind::Bool { default: true },
        ),
        FieldDef::new(
            "city",
            "City",
            "e.g. 'Dallas, TX'. Used when auto-locate is off.",
            FieldKind::Text { default: "" },
        )
        .when(|f| f.bool_val("current_location") == Some(false)),
        FieldDef::new(
            "current_location_api_key",
            "ipinfo token",
            "Optional ipinfo.io token for higher rate limits; blank to skip.",
            FieldKind::OptionalText { default: "" },
        )
        .when(|f| f.bool_val("current_location") == Some(true)),
        FieldDef::new(
            "weather_format",
            "Units",
            "Imperial (°F/mph) or metric (°C).",
            FieldKind::Enum {
                default: "imperial",
                choices: &[("imperial", "°F / mph"), ("metric", "°C / m·s⁻¹")],
            },
        ),
        FieldDef::new(
            "cache_ttl_secs",
            "Cache TTL (secs)",
            super::CACHE_TTL_HELP,
            FieldKind::CacheTtl,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use crate::createjson::tui::field::{FieldValue, Form};
    use crate::createjson::tui::form_module;

    fn set_enum(form: &mut Form, idx: usize, sel: usize) {
        if let FieldValue::Enum(s) = &mut form.values[idx] {
            *s = sel;
        }
    }
    fn set_bool(form: &mut Form, idx: usize, b: bool) {
        if let FieldValue::Bool(v) = &mut form.values[idx] {
            *v = b;
        }
    }

    #[test]
    fn nws_drops_api_key() {
        let form = form_module::default_form("weather");
        let v = form_module::section_to_value("weather", &form).unwrap();
        assert_eq!(v["api"], serde_json::json!("nws"));
        // Canonicalized through WeatherOptions: hidden optionals become null.
        assert!(v["api_key"].is_null());
        assert!(v["city"].is_null());
    }

    #[test]
    fn non_nws_requires_key() {
        let mut form = form_module::default_form("weather");
        set_enum(&mut form, 0, 1); // openweather
        assert!(form_module::section_to_value("weather", &form).is_err());
    }

    #[test]
    fn manual_location_requires_city_drops_ipinfo() {
        let mut form = form_module::default_form("weather");
        set_bool(&mut form, 2, false); // current_location off
        assert!(form_module::section_to_value("weather", &form).is_err());
        form.values[3] = FieldValue::Input(tui_input::Input::new("Dallas, TX".into()));
        let v = form_module::section_to_value("weather", &form).unwrap();
        assert_eq!(v["city"], serde_json::json!("Dallas, TX"));
        assert!(v["current_location_api_key"].is_null());
    }
}
