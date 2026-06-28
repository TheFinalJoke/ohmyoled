pub mod aurora;
pub mod f1;
pub mod flights;
pub mod golf;
pub mod hass;
pub mod iss;
pub mod launch;
pub mod pihole;
pub mod quake;
pub mod sport;
pub mod stock;
pub mod time;
pub mod traits;
pub mod tui;
pub mod weather;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Shared help text for the `cache_ttl_secs` field on every section.
pub const CACHE_TTL_HELP: &str =
    "Background poll cadence. Blank = API default, 0 = always fresh, N = every N seconds.";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatrixOptions {
    pub chain_length: i8,
    pub parallel: i8,
    pub brightness: i32,
    pub oled_slowdown: i32,
    pub fail_on_error: bool,
    /// One of: `adafruit-hat`, `adafruit-hat-pwm`, `regular`, `regular-pi1`,
    /// `classic`, `classic-pi1`. Defaults to `adafruit-hat` so existing configs
    /// that predate this field keep parsing.
    #[serde(default = "default_hardware_mapping")]
    pub hardware_mapping: String,
}

fn default_hardware_mapping() -> String {
    "adafruit-hat".to_string()
}

impl Default for MatrixOptions {
    fn default() -> Self {
        Self {
            chain_length: 1,
            parallel: 1,
            brightness: 50,
            oled_slowdown: 3,
            fail_on_error: false,
            hardware_mapping: default_hardware_mapping(),
        }
    }
}

/// Non-interactive starter config. Used by `--init-config` so users who
/// download a release binary get a config they can edit without having to
/// run the interactive `-c` flow. Time is enabled out of the box (no
/// external API needed); every other module is wired in with `run: false`
/// and `REPLACE_ME_*` placeholders for any required values, so the user
/// just has to flip `run` and fill the placeholders for whatever tiles
/// they want on the panel.
pub fn default_config() -> Value {
    let json = r#"{
        "matrix_options": {"chain_length":1,"parallel":1,"brightness":50,"oled_slowdown":3,"fail_on_error":false,"hardware_mapping":"adafruit-hat"},
        "time": {"run":true,"color":[255,255,255],"time_format":"null","timezone":"null","cache_ttl_secs":null},
        "weather": {"run":false,"api":"nws","current_location":true,"current_location_api_key":"REPLACE_ME_IPINFO_TOKEN","weather_format":"imperial","animation":"subtle","cache_ttl_secs":null},
        "stock": {"run":false,"api":"finnhub","api_key":"REPLACE_ME_FINNHUB_API_KEY","symbol":"AAPL","chart":false,"cache_ttl_secs":null},
        "sport": [],
        "iss": {"run":false,"lat":40.7128,"lon":-74.0060,"cache_ttl_secs":null},
        "quake": {"run":false,"feed":"significant_day","cache_ttl_secs":null},
        "aurora": {"run":false,"alert_threshold":5,"cache_ttl_secs":null},
        "flights": {"run":false,"lat":40.7128,"lon":-74.0060,"radius_km":80.0,"airborne_only":true,"cache_ttl_secs":null},
        "launch": {"run":false,"agency_filter":[],"cache_ttl_secs":null},
        "hass": {"run":false,"base_url":"http://homeassistant.local:8123","token":"REPLACE_ME_HASS_LONG_LIVED_TOKEN","entity_id":"sensor.kitchen_temp","label":"null","alarm_state":"null","nominal_color":[120,220,120],"alarm_color":[255,60,60],"display_mode":"state","cache_ttl_secs":null},
        "pihole": {"run":false,"base_url":"http://pi.hole","token":"null","cache_ttl_secs":null},
        "sleep": {"enabled":false,"sleep":"null","wake":"null","start":"null","end":"null","windows":[]},
        "eink": {"enabled":false,"model":"7in5_v2","rotation":0,"threshold":128,"mode":"auto","emulate":false,"refresh_ms":2500,"modules":{"time":{"run":true,"color":[255,255,255],"time_format":"12h","timezone":"null","cache_ttl_secs":null},"weather":{"run":false,"api":"nws","current_location":true,"current_location_api_key":"REPLACE_ME_IPINFO_TOKEN","weather_format":"imperial","animation":"subtle","cache_ttl_secs":null}}}
    }"#;
    serde_json::from_str(json).expect("starter config must parse")
}

/// Build the on-disk config.
///
/// `dev_mode = true` returns the canned dev config (no UI). Otherwise this
/// drives the full-screen [`tui`] wizard, returning the assembled config plus
/// the output format the user picked on the Setup screen. A user who quits
/// without saving yields the `{"failure": true}` sentinel (and `None` format),
/// which `main.rs` checks before writing. `dev_mode` also returns `None` for
/// the format, so the caller keeps the `-f` path's extension unchanged.
pub fn create_json(
    dev_mode: bool,
    existing: Option<Value>,
    initial_fmt: Option<tui::app::ConfigFormat>,
) -> (Value, Option<tui::app::ConfigFormat>) {
    if dev_mode {
        let dev_json = r#"{
            "matrix_options": {"chain_length":1,"parallel":1,"brightness":50,"oled_slowdown":3,"fail_on_error":false,"hardware_mapping":"adafruit-hat"},
            "time": {"run":true,"color":[255,255,255],"time_format":"null","timezone":"null"},
            "weather": {"run":true,"api":"nws","current_location":true,"current_location_api_key":"null","weather_format":"imperial","animation":"subtle"},
            "stock": {"run":true,"api":"finnhub","api_key":"null","symbol":"AAPL"},
            "sport": [
                {"run":true,"sport":"basketball","team_logo":{"name":"Dallas Mavericks","sportsdb_leagueid":4387,"url":"https://www.thesportsdb.com/images/media/team/badge/yqrxrs1420568796.png","sport":"basketball","shorthand":"DAL","apisportsid":138,"sportsdbid":134875,"sportsipyid":0}},
                {"run":true,"sport":"golf","tour":"pga"},
                {"run":true,"sport":"f1"}
            ],
            "sleep": {"enabled":false,"sleep":"null","wake":"null","start":"null","end":"null","windows":[]},
            "eink": {"enabled":false,"model":"7in5_v2","rotation":0,"threshold":128,"mode":"auto","emulate":false,"refresh_ms":2500,"modules":{"time":{"run":true,"color":[255,255,255],"time_format":"12h","timezone":"null"},"weather":{"run":true,"api":"nws","current_location":true,"current_location_api_key":"null","weather_format":"imperial","animation":"subtle"}}}
        }"#;
        return (
            serde_json::from_str(dev_json).expect("dev json must parse"),
            None,
        );
    }

    match tui::run(existing, initial_fmt) {
        Ok(Some((value, fmt))) => (value, Some(fmt)),
        Ok(None) => (serde_json::json!({ "failure": true }), None),
        Err(e) => {
            eprintln!("config builder: {e}");
            (serde_json::json!({ "failure": true }), None)
        }
    }
}
