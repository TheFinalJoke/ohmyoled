//! OpenWeatherMap (and NWS-via-OWM-codes) weather-icon glyph table.
//!
//! Direct port of `src/python/ohmyoled/lib/weather/weather_icon.py`. The
//! Python code used integer table indices (0–48) and looked them up from
//! provider-specific cascades; this module flattens that to a single
//! `icon_for_owm_code` function with `is_day` flag for day/night variants.

use super::model::WeatherIcon;

// ---------------------------------------------------------------------------
// Day-time variants (used when sunset > now in the Python code)
// ---------------------------------------------------------------------------

pub const SUNNY: WeatherIcon = WeatherIcon { condition: "Sunny", glyph: '\u{f00d}', owm_code: 800 };
pub const PARTLY_CLOUDY: WeatherIcon = WeatherIcon { condition: "Partly Cloudy", glyph: '\u{f002}', owm_code: 801 };
pub const MOSTLY_CLOUDY: WeatherIcon = WeatherIcon { condition: "Mostly Cloudy", glyph: '\u{f013}', owm_code: 801 };
pub const CLOUDY: WeatherIcon = WeatherIcon { condition: "Cloudy", glyph: '\u{f013}', owm_code: 801 };
pub const RAIN: WeatherIcon = WeatherIcon { condition: "Rain", glyph: '\u{f019}', owm_code: 500 };
pub const SNOW: WeatherIcon = WeatherIcon { condition: "Snow", glyph: '\u{f01b}', owm_code: 600 };
pub const THUNDERSTORM: WeatherIcon = WeatherIcon { condition: "Thunderstorm", glyph: '\u{f01e}', owm_code: 200 };
pub const DRIZZLE: WeatherIcon = WeatherIcon { condition: "Drizzle", glyph: '\u{f01c}', owm_code: 300 };
pub const SMOKE: WeatherIcon = WeatherIcon { condition: "Smoke", glyph: '\u{f062}', owm_code: 711 };
pub const HAZE: WeatherIcon = WeatherIcon { condition: "Haze", glyph: '\u{f0b6}', owm_code: 721 };
pub const FOG: WeatherIcon = WeatherIcon { condition: "Fog", glyph: '\u{f014}', owm_code: 741 };

// ---------------------------------------------------------------------------
// Night-time variants
// ---------------------------------------------------------------------------

pub const CLEAR_NIGHT: WeatherIcon = WeatherIcon { condition: "Clear", glyph: '\u{f02e}', owm_code: 800 };

/// Map an OpenWeatherMap weather code to a glyph.
///
/// Mirrors the cascade in `OpenWeather.get_icon` (openweather/weather.py:182-214):
/// 200–299 → thunderstorm, 300–399 → drizzle, 500–599 → rain, 600–699 → snow,
/// 700–780 → smoke/haze/fog (Python defaults to smoke), 800 → sunny/clear,
/// 801–805 → partly/mostly cloudy. Day variants when `is_day`; clear-night
/// fallback at night.
pub fn icon_for_owm_code(code: u16, is_day: bool) -> WeatherIcon {
    match code {
        200..=299 => THUNDERSTORM,
        300..=399 => DRIZZLE,
        500..=599 => RAIN,
        600..=699 => SNOW,
        700..=780 => SMOKE,
        800 if is_day => SUNNY,
        800 => CLEAR_NIGHT,
        801..=805 if is_day => PARTLY_CLOUDY,
        801..=805 => CLEAR_NIGHT,
        _ => SUNNY,
    }
}

/// Map an AccuWeather icon code (1–44) to a glyph.
///
/// AccuWeather's icon enum is documented at
/// <https://developer.accuweather.com/weather-icons>. Codes group naturally
/// into the same buckets as OWM, so we collapse to that and reuse the
/// day/night logic.
pub fn icon_for_accuweather_code(code: u8, is_day: bool) -> WeatherIcon {
    match code {
        1..=2 => if is_day { SUNNY } else { CLEAR_NIGHT },        // sunny, mostly sunny
        3..=5 => if is_day { PARTLY_CLOUDY } else { CLEAR_NIGHT }, // partly sunny
        6..=8 => if is_day { MOSTLY_CLOUDY } else { CLOUDY },     // mostly cloudy / cloudy
        11 => FOG,                                                  // fog
        12..=14 => RAIN,                                            // showers / rain
        15..=17 | 41..=42 => THUNDERSTORM,                          // t-storms
        18..=21 => RAIN,                                            // rain / mostly cloudy w/ showers
        22..=24 | 31 => SNOW,                                       // snow / cold
        25..=26 | 29 => SNOW,                                       // sleet / mixed -> nearest is snow
        32 => SUNNY,                                                // windy (no wind glyph here)
        33..=34 => CLEAR_NIGHT,                                     // clear night / mostly clear night
        35..=38 => CLEAR_NIGHT,                                     // partly/mostly cloudy night
        39..=40 => RAIN,                                            // partly cloudy w/ showers night
        43..=44 => SNOW,                                            // mostly/partly cloudy w/ flurries night
        _ => if is_day { SUNNY } else { CLEAR_NIGHT },
    }
}

/// Map a Pirate Weather / DarkSky icon string to a glyph.
///
/// Pirate Weather uses DarkSky's enum: `clear-day`, `clear-night`, `rain`,
/// `snow`, `sleet`, `wind`, `fog`, `cloudy`, `partly-cloudy-day`,
/// `partly-cloudy-night`. Newer Pirate codes (`hail`, `thunderstorm`, `tornado`)
/// are recognized but fall back to nearest neighbours.
pub fn icon_for_pirate_icon(icon: &str, _is_day: bool) -> WeatherIcon {
    match icon {
        "clear-day" => SUNNY,
        "clear-night" => CLEAR_NIGHT,
        "rain" => RAIN,
        "snow" | "sleet" | "hail" => SNOW,
        "fog" => FOG,
        "cloudy" => CLOUDY,
        "partly-cloudy-day" => PARTLY_CLOUDY,
        "partly-cloudy-night" => CLEAR_NIGHT,
        "wind" => CLOUDY,
        "thunderstorm" => THUNDERSTORM,
        _ => SUNNY,
    }
}

/// Map an NWS short-forecast string to a glyph.
///
/// Mirrors `NWSTransform.get_icon` in `weathergov/nws.py:175-193` — fuzzy
/// substring match on the forecast text.
pub fn icon_for_nws_condition(condition: &str, is_day: bool) -> WeatherIcon {
    let c = condition.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| c.contains(n));
    if has(&["sunny", "clear", "sun"]) {
        if is_day { SUNNY } else { CLEAR_NIGHT }
    } else if has(&["thunderstorm"]) {
        THUNDERSTORM
    } else if has(&["rain", "storm"]) {
        RAIN
    } else if c.contains("snow") {
        SNOW
    } else if has(&["cloudy", "cloud"]) {
        CLOUDY
    } else {
        if is_day { SUNNY } else { CLEAR_NIGHT }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owm_thunderstorm_range() {
        let icon = icon_for_owm_code(212, true);
        assert_eq!(icon.glyph, '\u{f01e}');
    }

    #[test]
    fn owm_clear_day_vs_night() {
        assert_eq!(icon_for_owm_code(800, true).glyph, '\u{f00d}');
        assert_eq!(icon_for_owm_code(800, false).glyph, '\u{f02e}');
    }

    #[test]
    fn owm_rain() {
        assert_eq!(icon_for_owm_code(500, true).owm_code, 500);
        assert_eq!(icon_for_owm_code(521, false).glyph, '\u{f019}');
    }

    #[test]
    fn nws_fuzzy_match() {
        assert_eq!(icon_for_nws_condition("Mostly sunny", true).condition, "Sunny");
        assert_eq!(icon_for_nws_condition("Light snow", true).condition, "Snow");
        assert_eq!(icon_for_nws_condition("Rain likely", true).condition, "Rain");
        assert_eq!(icon_for_nws_condition("Thunderstorms", true).condition, "Thunderstorm");
        assert_eq!(icon_for_nws_condition("Partly cloudy", true).condition, "Cloudy");
    }

    #[test]
    fn accuweather_codes() {
        assert_eq!(icon_for_accuweather_code(1, true).condition, "Sunny");
        assert_eq!(icon_for_accuweather_code(1, false).condition, "Clear");
        assert_eq!(icon_for_accuweather_code(12, true).condition, "Rain");
        assert_eq!(icon_for_accuweather_code(15, true).condition, "Thunderstorm");
        assert_eq!(icon_for_accuweather_code(22, true).condition, "Snow");
        assert_eq!(icon_for_accuweather_code(33, true).condition, "Clear");
        assert_eq!(icon_for_accuweather_code(7, true).condition, "Mostly Cloudy");
    }

    #[test]
    fn pirate_icons() {
        assert_eq!(icon_for_pirate_icon("clear-day", true).condition, "Sunny");
        assert_eq!(icon_for_pirate_icon("clear-night", false).condition, "Clear");
        assert_eq!(icon_for_pirate_icon("rain", true).condition, "Rain");
        assert_eq!(icon_for_pirate_icon("snow", true).condition, "Snow");
        assert_eq!(icon_for_pirate_icon("cloudy", true).condition, "Cloudy");
        assert_eq!(icon_for_pirate_icon("partly-cloudy-day", true).condition, "Partly Cloudy");
        assert_eq!(icon_for_pirate_icon("thunderstorm", true).condition, "Thunderstorm");
        // Unknown icon falls back to sunny.
        assert_eq!(icon_for_pirate_icon("unknown", true).condition, "Sunny");
    }
}
