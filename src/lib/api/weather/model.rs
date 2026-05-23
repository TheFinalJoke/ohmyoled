//! Normalized weather data shared by all providers.
//!
//! Direct port of `src/python/ohmyoled/lib/weather/weatherbase.py`. The two
//! providers (OpenWeather and NWS) each map their raw responses into a
//! [`Weather`] value, so the renderer doesn't care which API served it.

use chrono::{DateTime, Local};
use ohmyoled_matrix::Color;
use serde::{Deserialize, Serialize};

/// Which API produced this data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherApiSource {
    OpenWeather,
    Nws,
    AccuWeather,
    PirateWeather,
}

/// Compass direction for wind.
///
/// Mirrors `WindDirection` in `weatherbase.py`. The `from_degrees` helper
/// reproduces the cascading-`if` lookup used there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindDirection {
    N, NNE, NE, ENE, E, ESE, SE, SSE,
    S, SSW, SW, WSW, W, WNW, NW, NNW,
}

impl WindDirection {
    /// Map a bearing in degrees (0–360) to the nearest 16-point compass direction.
    pub fn from_degrees(deg: f32) -> Self {
        // 22.5° wedges, starting at -11.25° centered on N.
        let normalized = ((deg.rem_euclid(360.0)) + 11.25) / 22.5;
        match normalized as i32 % 16 {
            0 => Self::N,
            1 => Self::NNE,
            2 => Self::NE,
            3 => Self::ENE,
            4 => Self::E,
            5 => Self::ESE,
            6 => Self::SE,
            7 => Self::SSE,
            8 => Self::S,
            9 => Self::SSW,
            10 => Self::SW,
            11 => Self::WSW,
            12 => Self::W,
            13 => Self::WNW,
            14 => Self::NW,
            15 => Self::NNW,
            _ => Self::N,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::N => "N", Self::NNE => "NNE", Self::NE => "NE", Self::ENE => "ENE",
            Self::E => "E", Self::ESE => "ESE", Self::SE => "SE", Self::SSE => "SSE",
            Self::S => "S", Self::SSW => "SSW", Self::SW => "SW", Self::WSW => "WSW",
            Self::W => "W", Self::WNW => "WNW", Self::NW => "NW", Self::NNW => "NNW",
        }
    }
}

/// A single weather-icon entry: a glyph plus the color to draw it in.
///
/// Color lives here (rather than being re-derived from owm_code at render
/// time) so that day/night variants of the same code — e.g. `SUNNY` and
/// `CLEAR_NIGHT` both have owm_code 800 — get the right color without the
/// renderer having to guess which variant was picked.
#[derive(Debug, Clone, Copy)]
pub struct WeatherIcon {
    pub condition: &'static str,
    /// The Unicode codepoint in the weather-icons font.
    pub glyph: char,
    /// The OpenWeatherMap weather code this entry maps from.
    pub owm_code: u16,
    /// Color to render the glyph in.
    pub color: Color,
}

/// Snapshot of "what's happening now."
#[derive(Debug, Clone)]
pub struct CurrentWeather {
    pub conditions: String,
    pub temp: f32,
    pub feels_like: f32,
    pub wind_speed: f32,
    pub humidity: u32,
    pub precipitation_chance: u32,
    /// UV index; `None` when the provider doesn't supply it (e.g. NWS).
    pub uv: Option<f32>,
    /// Wind bearing in degrees; `None` when missing.
    pub wind_direction_deg: Option<f32>,
    pub icon: WeatherIcon,
}

/// Today's high/low + sunrise/sunset.
#[derive(Debug, Clone)]
pub struct DayForecast {
    pub today_high: f32,
    pub today_low: f32,
    pub sunrise: DateTime<Local>,
    pub sunset: DateTime<Local>,
}

/// Normalized weather payload — what the collector hands to the renderer.
#[derive(Debug, Clone)]
pub struct Weather {
    pub api: WeatherApiSource,
    pub lat: f64,
    pub lon: f64,
    pub location_name: String,
    pub current: CurrentWeather,
    pub forecast: DayForecast,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wind_direction_cardinals() {
        assert_eq!(WindDirection::from_degrees(0.0), WindDirection::N);
        assert_eq!(WindDirection::from_degrees(90.0), WindDirection::E);
        assert_eq!(WindDirection::from_degrees(180.0), WindDirection::S);
        assert_eq!(WindDirection::from_degrees(270.0), WindDirection::W);
    }

    #[test]
    fn wind_direction_wraps() {
        assert_eq!(WindDirection::from_degrees(360.0), WindDirection::N);
        assert_eq!(WindDirection::from_degrees(-10.0), WindDirection::N);
    }

    #[test]
    fn wind_direction_intercardinals() {
        assert_eq!(WindDirection::from_degrees(45.0), WindDirection::NE);
        assert_eq!(WindDirection::from_degrees(135.0), WindDirection::SE);
    }
}
