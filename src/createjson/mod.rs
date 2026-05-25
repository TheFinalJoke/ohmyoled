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
pub mod ui;
pub mod weather;

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Single accumulated entry for the running summary.
struct Entry {
    /// Section key in the final JSON (`time`/`weather`/`stock`/`sport`).
    section: &'static str,
    /// One-line human description shown in the summary.
    label: String,
    /// JSON value for this entry.
    value: Value,
}

fn print_menu(entries: &[Entry]) {
    let summary: Vec<String> = entries.iter().map(|e| e.label.clone()).collect();
    ui::summary(&summary);

    println!("  {}", ui::bold(&ui::cyan("Modules")));
    println!("    {} Time", ui::bold("[1]"));
    println!("    {} Weather", ui::bold("[2]"));
    println!("    {} Stock (live + optional historical chart)", ui::bold("[3]"));
    println!("    {} Sport — team (MLB / NBA / NFL / NHL)", ui::bold("[4]"));
    println!("    {} Sport — Golf", ui::bold("[5]"));
    println!("    {} Sport — Formula 1", ui::bold("[6]"));
    println!("    {} ISS — overhead countdown", ui::bold("[7]"));
    println!("    {} Earthquake — largest recent USGS event", ui::bold("[8]"));
    println!("    {} Aurora — NOAA planetary Kp index", ui::bold("[9]"));
    println!("    {} Flights — nearby aircraft (OpenSky)", ui::bold("[10]"));
    println!("    {} Launch — next orbital launch countdown", ui::bold("[11]"));
    println!("    {} Home Assistant entity", ui::bold("[12]"));
    println!("    {} Pi-hole DNS-block summary", ui::bold("[13]"));
    println!();
    println!("  {}", ui::bold(&ui::cyan("Actions")));
    println!("    {} Matrix panel options", ui::bold("[m]"));
    println!("    {} Undo last entry", ui::bold("[u]"));
    println!("    {} Continue and write config", ui::bold("[c]"));
    println!("    {} Quit without saving", ui::bold("[q]"));
    println!();
}

fn configure_matrix(current: &mut MatrixOptions) {
    ui::section("Matrix panel options");
    ui::hint("Defaults work for a 64×32 Adafruit HAT panel. Press Enter to keep current.");

    current.chain_length = ui::read_line_default(
        "Chain length (panels wired horizontally)",
        &current.chain_length.to_string(),
    )
    .parse()
    .unwrap_or(current.chain_length);

    current.parallel = ui::read_line_default(
        "Parallel chains",
        &current.parallel.to_string(),
    )
    .parse()
    .unwrap_or(current.parallel);

    current.brightness = ui::read_line_default(
        "Brightness (0–100)",
        &current.brightness.to_string(),
    )
    .parse()
    .unwrap_or(current.brightness);

    current.oled_slowdown = ui::read_line_default(
        "GPIO slowdown (0–4; raise if flickering)",
        &current.oled_slowdown.to_string(),
    )
    .parse()
    .unwrap_or(current.oled_slowdown);

    current.hardware_mapping = ui::read_line_default(
        "Hardware mapping (adafruit-hat | adafruit-hat-pwm | regular | regular-pi1 | classic | classic-pi1)",
        &current.hardware_mapping,
    );

    ui::success("Matrix options updated");
}

/// Collapse repeated section entries into either a single object (count == 1)
/// or a JSON array (count > 1). Sections registry deserializes either shape via
/// `one_or_many`, so this just keeps the on-disk file shape natural.
fn fold_section(values: Vec<Value>) -> Option<Value> {
    match values.len() {
        0 => None,
        1 => values.into_iter().next(),
        _ => Some(Value::Array(values)),
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
        "time": {"run":true,"color":[255,255,255],"time_format":"null","timezone":"null"},
        "weather": {"run":false,"api":"nws","current_location":true,"current_location_api_key":"REPLACE_ME_IPINFO_TOKEN","weather_format":"imperial","animation":"subtle"},
        "stock": {"run":false,"api":"finnhub","api_key":"REPLACE_ME_FINNHUB_API_KEY","symbol":"AAPL","chart":false},
        "sport": [],
        "iss": {"run":false,"lat":40.7128,"lon":-74.0060},
        "quake": {"run":false,"feed":"significant_day"},
        "aurora": {"run":false,"alert_threshold":5},
        "flights": {"run":false,"lat":40.7128,"lon":-74.0060,"radius_km":80.0},
        "launch": {"run":false,"agency_filter":[]},
        "hass": {"run":false,"base_url":"http://homeassistant.local:8123","token":"REPLACE_ME_HASS_LONG_LIVED_TOKEN","entity_id":"sensor.kitchen_temp","label":"null","alarm_state":"null","nominal_color":[120,220,120],"alarm_color":[255,60,60]},
        "pihole": {"run":false,"base_url":"http://pi.hole","token":"null"}
    }"#;
    serde_json::from_str(json).expect("starter config must parse")
}

/// Build the on-disk config interactively (or with `dev_mode = true` for the
/// canned dev config).
pub fn create_json(dev_mode: bool) -> Value {
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
            ]
        }"#;
        return serde_json::from_str(dev_json).expect("dev json must parse");
    }

    ui::banner("ohmyoled config builder", &format!("v{VERSION}"));
    ui::info("Pick the modules you want on the panel. Most options have sane defaults.");
    ui::hint("Selections are accumulated below — sport entries can mix team sports, golf, and F1.");

    let mut entries: Vec<Entry> = Vec::new();
    let mut matrix_opts = MatrixOptions::default();

    loop {
        print_menu(&entries);
        let raw = match ui::read_line("Selection") {
            Some(s) => s,
            None => continue,
        };
        match raw.trim().to_lowercase().as_str() {
            "1" => {
                let opts = time::configure();
                let label = time::summary_line(&opts);
                let value = serde_json::to_value(opts).expect("TimeOptions serializes");
                entries.push(Entry { section: "time", label, value });
            }
            "2" => match weather::configure() {
                Ok(opts) => {
                    let label = weather::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("WeatherOptions serializes");
                    entries.push(Entry { section: "weather", label, value });
                }
                Err(e) => ui::error(&format!("weather config failed: {e}")),
            },
            "3" => match stock::configure() {
                Ok(opts) => {
                    let label = stock::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("StockOptions serializes");
                    entries.push(Entry { section: "stock", label, value });
                }
                Err(e) => ui::error(&format!("stock config failed: {e}")),
            },
            "4" => match sport::configure() {
                Ok(value) => {
                    let label = sport::summary_line(&value);
                    entries.push(Entry { section: "sport", label, value });
                }
                Err(e) => ui::error(&format!("sport config failed: {e}")),
            },
            "5" => match golf::configure() {
                Ok(value) => {
                    let label = golf::summary_line(&value);
                    entries.push(Entry { section: "sport", label, value });
                }
                Err(e) => ui::error(&format!("golf config failed: {e}")),
            },
            "6" => match f1::configure() {
                Ok(value) => {
                    let label = f1::summary_line(&value);
                    entries.push(Entry { section: "sport", label, value });
                }
                Err(e) => ui::error(&format!("f1 config failed: {e}")),
            },
            "7" => match iss::configure() {
                Ok(opts) => {
                    let label = iss::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("IssOptions serializes");
                    entries.push(Entry { section: "iss", label, value });
                }
                Err(e) => ui::error(&format!("iss config failed: {e}")),
            },
            "8" => match quake::configure() {
                Ok(opts) => {
                    let label = quake::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("QuakeOptions serializes");
                    entries.push(Entry { section: "quake", label, value });
                }
                Err(e) => ui::error(&format!("quake config failed: {e}")),
            },
            "9" => match aurora::configure() {
                Ok(opts) => {
                    let label = aurora::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("AuroraOptions serializes");
                    entries.push(Entry { section: "aurora", label, value });
                }
                Err(e) => ui::error(&format!("aurora config failed: {e}")),
            },
            "10" => match flights::configure() {
                Ok(opts) => {
                    let label = flights::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("FlightsOptions serializes");
                    entries.push(Entry { section: "flights", label, value });
                }
                Err(e) => ui::error(&format!("flights config failed: {e}")),
            },
            "11" => match launch::configure() {
                Ok(opts) => {
                    let label = launch::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("LaunchOptions serializes");
                    entries.push(Entry { section: "launch", label, value });
                }
                Err(e) => ui::error(&format!("launch config failed: {e}")),
            },
            "12" => match hass::configure() {
                Ok(opts) => {
                    let label = hass::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("HassOptions serializes");
                    entries.push(Entry { section: "hass", label, value });
                }
                Err(e) => ui::error(&format!("hass config failed: {e}")),
            },
            "13" => match pihole::configure() {
                Ok(opts) => {
                    let label = pihole::summary_line(&opts);
                    let value = serde_json::to_value(opts).expect("PiholeOptions serializes");
                    entries.push(Entry { section: "pihole", label, value });
                }
                Err(e) => ui::error(&format!("pihole config failed: {e}")),
            },
            "m" => configure_matrix(&mut matrix_opts),
            "u" => match entries.pop() {
                Some(e) => ui::success(&format!("Removed: {}", e.label)),
                None => ui::warn("Nothing to undo"),
            },
            "c" => {
                if entries.is_empty() {
                    ui::warn("No modules selected — add at least one before continuing");
                    continue;
                }
                break;
            }
            "q" => {
                ui::warn("Quit — no configuration written");
                return serde_json::json!({ "failure": true });
            }
            other => ui::warn(&format!("'{other}' is not a menu option")),
        }
    }

    let mut config = serde_json::Map::new();
    config.insert(
        "matrix_options".to_string(),
        serde_json::to_value(&matrix_opts).expect("MatrixOptions serializes"),
    );

    let sections = [
        "time", "weather", "stock", "sport", "iss", "quake", "aurora",
        "flights", "launch", "hass", "pihole",
    ];
    for name in sections {
        let bucket: Vec<Value> = entries
            .iter()
            .filter(|e| e.section == name)
            .map(|e| e.value.clone())
            .collect();
        if let Some(v) = fold_section(bucket) {
            config.insert(name.to_string(), v);
        }
    }

    ui::section("Summary");
    for e in &entries {
        ui::success(&e.label);
    }

    Value::Object(config)
}
