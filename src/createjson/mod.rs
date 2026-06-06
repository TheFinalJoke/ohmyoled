pub mod aurora;
pub mod eink;
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
    println!("    {} E-paper (e-ink) display — independent screen", ui::bold("[14]"));
    println!();
    println!("  {}", ui::bold(&ui::cyan("Actions")));
    println!("    {} Matrix panel options", ui::bold("[m]"));
    println!("    {} Undo last entry, or `u N` to remove entry #N", ui::bold("[u]"));
    println!(
        "    {} `e N` to edit entry #N in place (re-runs that module's prompts)",
        ui::bold("[e]")
    );
    println!(
        "    {} `s N` to show entry #N's full JSON (so you can copy api_keys etc.)",
        ui::bold("[s]")
    );
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

/// Pretty-print one entry's JSON so the user can read off api_keys /
/// tokens / urls without having to look them up elsewhere. Falls back
/// to compact form if pretty-printing somehow fails (shouldn't, but
/// belt-and-braces — we never want this helper to swallow content).
fn print_entry_json(entry: &Entry) {
    let pretty = serde_json::to_string_pretty(&entry.value)
        .unwrap_or_else(|_| entry.value.to_string());
    println!();
    for line in pretty.lines() {
        println!("    {line}");
    }
    println!();
}

/// Re-run the prompts for an existing entry, returning a fresh
/// `Entry` to insert in its place. Dispatches on `entry.section`
/// (plus the inner `sport` tag when the section is `sport`) so the
/// caller doesn't need to remember which module number maps to which
/// configure() function.
///
/// Returns `None` if the underlying configure() reported an error or
/// the section is unrecognised. The caller is responsible for
/// restoring the original entry in that case — `e N` should never
/// silently delete user data.
fn re_configure_entry(entry: &Entry) -> Option<Entry> {
    macro_rules! run_typed {
        ($section:expr, $module:ident, $expect:expr) => {
            match $module::configure() {
                Ok(opts) => Some(Entry {
                    section: $section,
                    label: $module::summary_line(&opts),
                    value: serde_json::to_value(opts).expect($expect),
                }),
                Err(e) => {
                    ui::error(&format!("{}: {e}", $section));
                    None
                }
            }
        };
    }
    macro_rules! run_value {
        ($section:expr, $module:ident) => {
            match $module::configure() {
                Ok(value) => Some(Entry {
                    section: $section,
                    label: $module::summary_line(&value),
                    value,
                }),
                Err(e) => {
                    ui::error(&format!("{}: {e}", $section));
                    None
                }
            }
        };
    }
    match entry.section {
        "time" => {
            let opts = time::configure();
            Some(Entry {
                section: "time",
                label: time::summary_line(&opts),
                value: serde_json::to_value(opts).expect("TimeOptions serializes"),
            })
        }
        "weather" => run_typed!("weather", weather, "WeatherOptions serializes"),
        "stock" => run_typed!("stock", stock, "StockOptions serializes"),
        "sport" => {
            // The "sport" section is the umbrella for three different
            // configure() flows; pick the one matching the original's
            // tag so editing a golf entry doesn't drop the user into
            // a basketball prompt.
            let kind = entry
                .value
                .get("sport")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match kind {
                "basketball" | "baseball" | "football" | "hockey" => {
                    run_value!("sport", sport)
                }
                "golf" => run_value!("sport", golf),
                "f1" => run_value!("sport", f1),
                other => {
                    ui::error(&format!("unknown sport variant `{other}` — can't edit"));
                    None
                }
            }
        }
        "iss" => run_typed!("iss", iss, "IssOptions serializes"),
        "quake" => run_typed!("quake", quake, "QuakeOptions serializes"),
        "aurora" => run_typed!("aurora", aurora, "AuroraOptions serializes"),
        "flights" => run_typed!("flights", flights, "FlightsOptions serializes"),
        "launch" => run_typed!("launch", launch, "LaunchOptions serializes"),
        "hass" => run_typed!("hass", hass, "HassOptions serializes"),
        "pihole" => run_typed!("pihole", pihole, "PiholeOptions serializes"),
        "eink" => run_value!("eink", eink),
        other => {
            ui::error(&format!("unknown section `{other}` — can't edit"));
            None
        }
    }
}

/// Parse a 1-based entry index from a user-supplied string. Returns
/// the 0-based vec position on success, a user-facing error message
/// otherwise. Used by the `u N` / `e N` action arms.
fn parse_entry_index(s: &str, len: usize) -> Result<usize, String> {
    let n: usize = s
        .parse()
        .map_err(|_| format!("`{s}` is not a positive integer"))?;
    if n == 0 || n > len {
        if len == 0 {
            return Err("Nothing to remove — entry list is empty".into());
        }
        return Err(format!(
            "No entry #{n} — valid range is 1..={}",
            len
        ));
    }
    Ok(n - 1)
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
        "time": {"run":true,"color":[255,255,255],"time_format":"null","timezone":"null","cache_ttl_secs":null},
        "weather": {"run":false,"api":"nws","current_location":true,"current_location_api_key":"REPLACE_ME_IPINFO_TOKEN","weather_format":"imperial","animation":"subtle","cache_ttl_secs":null},
        "stock": {"run":false,"api":"finnhub","api_key":"REPLACE_ME_FINNHUB_API_KEY","symbol":"AAPL","chart":false,"cache_ttl_secs":null},
        "sport": [],
        "iss": {"run":false,"lat":40.7128,"lon":-74.0060,"cache_ttl_secs":null},
        "quake": {"run":false,"feed":"significant_day","cache_ttl_secs":null},
        "aurora": {"run":false,"alert_threshold":5,"cache_ttl_secs":null},
        "flights": {"run":false,"lat":40.7128,"lon":-74.0060,"radius_km":80.0,"cache_ttl_secs":null},
        "launch": {"run":false,"agency_filter":[],"cache_ttl_secs":null},
        "hass": {"run":false,"base_url":"http://homeassistant.local:8123","token":"REPLACE_ME_HASS_LONG_LIVED_TOKEN","entity_id":"sensor.kitchen_temp","label":"null","alarm_state":"null","nominal_color":[120,220,120],"alarm_color":[255,60,60],"display_mode":"state","cache_ttl_secs":null},
        "pihole": {"run":false,"base_url":"http://pi.hole","token":"null","cache_ttl_secs":null},
        "eink": {"enabled":false,"model":"4in2","rotation":0,"threshold":128,"modules":{"weather":{"run":false,"api":"nws","current_location":true,"current_location_api_key":"REPLACE_ME_IPINFO_TOKEN","weather_format":"imperial","animation":"subtle","cache_ttl_secs":null}}}
    }"#;
    serde_json::from_str(json).expect("starter config must parse")
}

/// Section names recognised when merging an existing config back in.
const KNOWN_SECTIONS: &[&str] = &[
    "time", "weather", "stock", "sport", "iss", "quake", "aurora",
    "flights", "launch", "hass", "pihole", "eink",
];

/// Pull existing entries out of a previously-written config so the
/// builder can pre-populate them and let the user append / undo rather
/// than starting blank. A section that's an array becomes one entry
/// per array element; a single object becomes one entry. Unknown or
/// malformed sections are skipped (the builder rewrites them faithfully
/// because the JSON `Value` survives round-trip).
fn load_existing_entries(existing: &Value) -> (Vec<Entry>, MatrixOptions) {
    let mut entries = Vec::new();
    let matrix_opts = existing
        .get("matrix_options")
        .and_then(|mo| serde_json::from_value::<MatrixOptions>(mo.clone()).ok())
        .unwrap_or_default();

    for &section in KNOWN_SECTIONS {
        let Some(v) = existing.get(section) else { continue };
        let items: Vec<Value> = match v {
            Value::Array(arr) => arr.clone(),
            Value::Object(_) => vec![v.clone()],
            _ => continue,
        };
        for item in items {
            let label = describe_existing_entry(section, &item);
            entries.push(Entry { section, label, value: item });
        }
    }
    (entries, matrix_opts)
}

/// One-line summary for a pre-loaded entry. Pulls the obvious
/// identifier per section (symbol / city / entity / team) so the user
/// can recognise what's already there without re-running each module's
/// `configure()` to regenerate the summary.
fn describe_existing_entry(section: &'static str, value: &Value) -> String {
    let str_field = |key: &str| -> Option<String> {
        value.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    };
    let suffix = match section {
        "weather" => str_field("api").map(|s| format!(" ({s})")),
        "stock" => str_field("symbol").map(|s| format!(" ({s})")),
        "sport" => {
            let kind = str_field("sport").unwrap_or_else(|| "?".into());
            let team = value
                .get("team_logo")
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str());
            match team {
                Some(name) => Some(format!(": {kind} ({name})")),
                None => Some(format!(": {kind}")),
            }
        }
        "hass" => str_field("entity_id").map(|s| format!(" ({s})")),
        "iss" | "flights" => {
            match (value.get("lat").and_then(|v| v.as_f64()), value.get("lon").and_then(|v| v.as_f64())) {
                (Some(lat), Some(lon)) => Some(format!(" ({lat:.2}, {lon:.2})")),
                _ => None,
            }
        }
        _ => None,
    };
    format!(
        "{section}{} — existing",
        suffix.unwrap_or_default()
    )
}

/// Build the on-disk config interactively (or with `dev_mode = true` for the
/// canned dev config).
///
/// When `existing` is `Some`, pre-load its entries + matrix options so
/// the user can append to / undo from the current config instead of
/// starting blank. Passing `None` (overwrite mode) yields the legacy
/// fresh-start behavior.
pub fn create_json(dev_mode: bool, existing: Option<Value>) -> Value {
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

    let (mut entries, mut matrix_opts): (Vec<Entry>, MatrixOptions) = match existing {
        Some(v) => {
            let (entries, opts) = load_existing_entries(&v);
            if !entries.is_empty() {
                ui::success(&format!(
                    "Loaded {} entr{} from existing config — append or `u` to remove",
                    entries.len(),
                    if entries.len() == 1 { "y" } else { "ies" }
                ));
            }
            (entries, opts)
        }
        None => (Vec::new(), MatrixOptions::default()),
    };

    loop {
        print_menu(&entries);
        let raw = match ui::read_line("Selection") {
            Some(s) => s,
            None => continue,
        };
        // Split into head + optional index arg — supports `u`, `u 3`, `e 2`.
        let lowered = raw.trim().to_lowercase();
        let (head, arg) = match lowered.split_once(char::is_whitespace) {
            Some((h, rest)) => (h, rest.trim()),
            None => (lowered.as_str(), ""),
        };
        match head {
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
            "14" => match eink::configure() {
                Ok(value) => {
                    // The eink block is a single top-level object; replace any
                    // prior one rather than accumulating duplicates.
                    entries.retain(|e| e.section != "eink");
                    let label = eink::summary_line(&value);
                    entries.push(Entry { section: "eink", label, value });
                }
                Err(e) => ui::error(&format!("eink config failed: {e}")),
            },
            "m" => configure_matrix(&mut matrix_opts),
            "u" => {
                if arg.is_empty() {
                    match entries.pop() {
                        Some(e) => ui::success(&format!("Removed: {}", e.label)),
                        None => ui::warn("Nothing to undo"),
                    }
                } else {
                    match parse_entry_index(arg, entries.len()) {
                        Ok(idx) => {
                            let removed = entries.remove(idx);
                            ui::success(&format!("Removed #{}: {}", idx + 1, removed.label));
                        }
                        Err(e) => ui::warn(&e),
                    }
                }
            }
            "e" => {
                if arg.is_empty() {
                    ui::warn("Usage: `e N` — pick entry # from the summary above to edit");
                } else {
                    match parse_entry_index(arg, entries.len()) {
                        Ok(idx) => {
                            // Pull the entry out, hand it to the matching
                            // module's `configure()` for a fresh prompt run,
                            // then insert the new value at the original
                            // index. On error or unrecognised section the
                            // original entry is restored — `e N` never
                            // silently destroys data.
                            let original = entries.remove(idx);
                            ui::info(&format!("Editing #{}: {}", idx + 1, original.label));
                            // Print the entry's current JSON so the user
                            // can read off api_keys / tokens / urls
                            // without leaving the prompt. Prompts still
                            // default to the module's hard-coded values,
                            // so this is the "look it up here" affordance.
                            print_entry_json(&original);
                            match re_configure_entry(&original) {
                                Some(new) => {
                                    let new_label = new.label.clone();
                                    entries.insert(idx, new);
                                    ui::success(&format!(
                                        "Replaced #{}: {new_label}", idx + 1
                                    ));
                                }
                                None => {
                                    ui::warn(
                                        "Edit cancelled or failed — restoring original entry"
                                    );
                                    entries.insert(idx, original);
                                }
                            }
                        }
                        Err(e) => ui::warn(&e),
                    }
                }
            }
            "s" => {
                if arg.is_empty() {
                    ui::warn("Usage: `s N` — pick entry # from the summary above to show");
                } else {
                    match parse_entry_index(arg, entries.len()) {
                        Ok(idx) => {
                            ui::info(&format!(
                                "Entry #{}: {}", idx + 1, entries[idx].label
                            ));
                            print_entry_json(&entries[idx]);
                        }
                        Err(e) => ui::warn(&e),
                    }
                }
            }
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
        "flights", "launch", "hass", "pihole", "eink",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_existing_expands_arrays_and_objects() {
        // Mixed shape: stock is an array, weather is a single object,
        // sport carries a team_logo with a name, hass is an array.
        // Each item should land as its own Entry.
        let existing = serde_json::json!({
            "matrix_options": {
                "chain_length": 2, "parallel": 1, "brightness": 70,
                "oled_slowdown": 3, "fail_on_error": false,
                "hardware_mapping": "adafruit-hat"
            },
            "time": {"run": true, "color": [255, 255, 255]},
            "weather": {"run": true, "api": "nws"},
            "stock": [
                {"run": true, "api": "finnhub", "symbol": "AAPL"},
                {"run": true, "api": "finnhub", "symbol": "MSFT"}
            ],
            "sport": [
                {"run": true, "sport": "basketball", "team_logo": {"name": "Boston Celtics"}},
                {"run": true, "sport": "f1"}
            ],
            "iss": {"run": true, "lat": 40.7128, "lon": -74.0060},
            "hass": [{"run": true, "entity_id": "sensor.kitchen_temp"}]
        });
        let (entries, opts) = load_existing_entries(&existing);
        let sections: Vec<&str> = entries.iter().map(|e| e.section).collect();
        // 1 time + 1 weather + 2 stock + 2 sport + 1 iss + 1 hass = 8
        assert_eq!(entries.len(), 8);
        assert!(sections.contains(&"time"));
        assert!(sections.contains(&"weather"));
        assert_eq!(sections.iter().filter(|s| **s == "stock").count(), 2);
        assert_eq!(sections.iter().filter(|s| **s == "sport").count(), 2);
        assert!(sections.contains(&"hass"));
        // Matrix options carried through.
        assert_eq!(opts.chain_length, 2);
        assert_eq!(opts.brightness, 70);
    }

    #[test]
    fn describe_picks_up_identifying_fields() {
        let stock = serde_json::json!({"symbol": "AAPL", "api": "finnhub"});
        assert!(describe_existing_entry("stock", &stock).contains("AAPL"));

        let weather = serde_json::json!({"api": "nws"});
        assert!(describe_existing_entry("weather", &weather).contains("nws"));

        let hass = serde_json::json!({"entity_id": "sensor.kitchen_temp"});
        assert!(describe_existing_entry("hass", &hass).contains("sensor.kitchen_temp"));

        let sport = serde_json::json!({
            "sport": "basketball",
            "team_logo": {"name": "Boston Celtics"}
        });
        let s = describe_existing_entry("sport", &sport);
        assert!(s.contains("basketball"));
        assert!(s.contains("Boston Celtics"));

        let iss = serde_json::json!({"lat": 40.7128, "lon": -74.0060});
        let s = describe_existing_entry("iss", &iss);
        assert!(s.contains("40.71"));
        assert!(s.contains("-74.01"));
    }

    #[test]
    fn parse_entry_index_accepts_valid_one_based() {
        assert_eq!(parse_entry_index("1", 3).unwrap(), 0);
        assert_eq!(parse_entry_index("3", 3).unwrap(), 2);
    }

    #[test]
    fn parse_entry_index_rejects_out_of_range_and_zero() {
        assert!(parse_entry_index("0", 3).is_err());
        assert!(parse_entry_index("4", 3).is_err());
        assert!(parse_entry_index("abc", 3).is_err());
        assert!(parse_entry_index("1", 0).is_err());
    }

    #[test]
    fn load_existing_skips_unknown_sections_cleanly() {
        let existing = serde_json::json!({
            "time": {"run": true, "color": [255, 255, 255]},
            "made_up_section": {"foo": "bar"}
        });
        let (entries, _) = load_existing_entries(&existing);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].section, "time");
    }
}
