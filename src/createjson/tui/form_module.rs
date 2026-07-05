//! Per-section dispatch between the generic [`Form`] engine and each module's
//! schema, plus the two display-target forms (matrix / eink block) and the
//! `fold_section` helper shared by save and preview.
//!
//! A *tile kind* (`time`, `weather`, …, and the three sport flavours `sport`/
//! `golf`/`f1`) is what the Modules screen lists. Several kinds can share one
//! *config key*: `sport`/`golf`/`f1` all fold into the `sport` key. The split
//! is the one wrinkle the otherwise-generic engine has to know about.

use super::field::{FieldDef, FieldKind, Form};
use crate::createjson::{
    aurora, f1, flights, golf, hass, iss, launch, pihole, quake, sleep, sport, stock, time,
    weather,
};
use serde_json::{json, Map, Value};

/// Selectable tile kinds shown on the Modules screen, in display order, as
/// `(kind id, menu label)`. Same set for both display targets. `sleep` is the
/// one non-module tile: single-instance, `enabled` instead of `run`, and it
/// always assembles at the config top level (never under `eink.modules`).
pub const TILE_KINDS: &[(&str, &str)] = &[
    ("time", "Time"),
    ("weather", "Weather"),
    ("stock", "Stock"),
    ("sport", "Sport — Team"),
    ("golf", "Sport — Golf"),
    ("f1", "Sport — Formula 1"),
    ("iss", "ISS"),
    ("quake", "Earthquake"),
    ("aurora", "Aurora"),
    ("flights", "Flights"),
    ("launch", "Launch"),
    ("hass", "Home Assistant"),
    ("pihole", "Pi-hole"),
    ("sleep", "Sleep Schedule"),
];

/// Whether a tile kind may have several instances (folded to a JSON array).
/// `sleep` is a single top-level object in the registry, so exactly one.
pub fn allow_multi(kind: &str) -> bool {
    kind != "sleep"
}

/// Menu label for a tile kind.
pub fn title(kind: &str) -> &'static str {
    TILE_KINDS
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, t)| *t)
        .unwrap_or("module")
}

/// The config object key a tile kind serializes under. The three sport flavours
/// share the `sport` key; everything else uses its own id.
pub fn config_key(kind: &str) -> &'static str {
    match kind {
        "sport" | "golf" | "f1" => "sport",
        "time" => "time",
        "weather" => "weather",
        "stock" => "stock",
        "iss" => "iss",
        "quake" => "quake",
        "aurora" => "aurora",
        "flights" => "flights",
        "launch" => "launch",
        "hass" => "hass",
        "pihole" => "pihole",
        "sleep" => "sleep",
        _ => "",
    }
}

/// The field schema for a tile kind.
pub fn fields(kind: &str) -> Vec<FieldDef> {
    match kind {
        "time" => time::fields(),
        "weather" => weather::fields(),
        "stock" => stock::fields(),
        "sport" => sport::fields(),
        "golf" => golf::fields(),
        "f1" => f1::fields(),
        "iss" => iss::fields(),
        "quake" => quake::fields(),
        "aurora" => aurora::fields(),
        "flights" => flights::fields(),
        "launch" => launch::fields(),
        "hass" => hass::fields(),
        "pihole" => pihole::fields(),
        "sleep" => sleep::fields(),
        _ => Vec::new(),
    }
}

/// A fresh default form for a tile kind, with any dynamic options populated
/// (sport's team list). `sleep` carries its own `enabled` switch, so it's the
/// one kind that doesn't get the implicit `run: true`.
pub fn default_form(kind: &str) -> Form {
    let mut form = Form::from_defs(fields(kind), kind != "sleep");
    init_dynamic(kind, &mut form);
    form
}

/// Build a form for a tile kind pre-filled from an existing JSON object.
pub fn value_to_form(kind: &str, value: &Value) -> Form {
    let mut form = default_form(kind);
    // First pass: set scalar fields + the sport enum from the value.
    form.apply_value(value);
    // Repopulate any dependent option lists now that the driving field (sport)
    // reflects the loaded value, then re-apply so the dependent selection
    // (team) lands correctly.
    on_field_changed(kind, &mut form, "sport");
    form.apply_value(value);
    form
}

/// Populate dynamic option lists on a freshly-built form.
fn init_dynamic(kind: &str, form: &mut Form) {
    if kind == "sport" {
        let slug = form.enum_slug("sport").unwrap_or("baseball");
        let opts = sport::team_options(slug);
        let sel = sport::default_team_index(&opts, "Chicago Cubs");
        form.set_value_enum("team_logo", opts, sel);
    }
}

/// React to a field changing — repopulate dependent option lists. Currently
/// only sport's team picker depends on another field.
pub fn on_field_changed(kind: &str, form: &mut Form, changed_id: &str) {
    if kind == "sport" && changed_id == "sport" {
        let slug = form.enum_slug("sport").unwrap_or("baseball");
        let opts = sport::team_options(slug);
        form.set_value_enum("team_logo", opts, 0);
    }
}

/// Validate + assemble a tile kind's JSON value, applying per-kind
/// post-processing (sport-constant injection, stock symbol case-folding,
/// time's `system` ⇒ `null`).
pub fn section_to_value(kind: &str, form: &Form) -> Result<Value, Vec<(String, String)>> {
    let v = form.to_value()?;
    // Round-trip struct-backed kinds through their typed `Options` so the JSON
    // shape physically can't drift from the struct (unknown keys dropped,
    // renames applied). Value-native sport flavours pass through untouched.
    let v = canonicalize(kind, v).map_err(|e| vec![(kind.to_string(), e)])?;
    // Sleep gets the daemon's own schedule validation (cron syntax, HH:MM,
    // pairing rules) so a wizard-written config can't fail at startup.
    if kind == "sleep" {
        let opts: sleep::SleepOptions = serde_json::from_value(v.clone())
            .map_err(|e| vec![("sleep".to_string(), e.to_string())])?;
        let errs = sleep::validate(&opts);
        if !errs.is_empty() {
            return Err(errs);
        }
    }
    let mut map = match v {
        Value::Object(m) => m,
        other => return Ok(other),
    };
    match kind {
        "golf" => {
            map.insert("sport".to_string(), json!("golf"));
        }
        "f1" => {
            map.insert("sport".to_string(), json!("f1"));
        }
        "stock" => fold_stock_symbol(&mut map),
        // Legacy: time's "system" choice means "no override" ⇒ JSON null.
        "time" if map.get("time_format") == Some(&json!("system")) => {
            map.insert("time_format".to_string(), Value::Null);
        }
        _ => {}
    }
    Ok(Value::Object(map))
}

/// Round-trip a tile's JSON through its typed `Options` struct, so the on-disk
/// shape is whatever the struct serializes (the source of truth). Returns the
/// serde error message on a validation failure. Sport flavours are
/// `Value`-native and pass through.
fn canonicalize(kind: &str, v: Value) -> Result<Value, String> {
    macro_rules! roundtrip {
        ($t:ty) => {{
            let opts: $t = serde_json::from_value(v).map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(opts).expect("Options serializes"))
        }};
    }
    match kind {
        "time" => roundtrip!(time::TimeOptions),
        "weather" => roundtrip!(weather::WeatherOptions),
        "stock" => roundtrip!(stock::StockOptions),
        "iss" => roundtrip!(iss::IssOptions),
        "quake" => roundtrip!(quake::QuakeOptions),
        "aurora" => roundtrip!(aurora::AuroraOptions),
        "flights" => roundtrip!(flights::FlightsOptions),
        "launch" => roundtrip!(launch::LaunchOptions),
        "hass" => roundtrip!(hass::HassOptions),
        "pihole" => roundtrip!(pihole::PiholeOptions),
        "sleep" => roundtrip!(sleep::SleepOptions),
        _ => Ok(v),
    }
}

/// Upper-case the symbol for Finnhub, lower-case it for CoinGecko — matching
/// the legacy prompt's normalization.
fn fold_stock_symbol(map: &mut Map<String, Value>) {
    let api = map.get("api").and_then(|v| v.as_str()).unwrap_or("finnhub");
    if let Some(sym) = map.get("symbol").and_then(|v| v.as_str()) {
        let folded = match api {
            "coingecko" => sym.trim().to_lowercase(),
            _ => sym.trim().to_uppercase(),
        };
        map.insert("symbol".to_string(), json!(folded));
    }
}

// --- Display-target forms ---------------------------------------------------

/// Matrix (LED panel) geometry options → `matrix_options`.
pub fn matrix_fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "chain_length",
            "Chain length",
            "Panels wired horizontally.",
            FieldKind::Number {
                default: 1,
                min: 1,
                max: 8,
            },
        ),
        FieldDef::new(
            "parallel",
            "Parallel chains",
            "Chains wired vertically.",
            FieldKind::Number {
                default: 1,
                min: 1,
                max: 4,
            },
        ),
        FieldDef::new(
            "brightness",
            "Brightness",
            "0–100.",
            FieldKind::Number {
                default: 50,
                min: 0,
                max: 100,
            },
        ),
        FieldDef::new(
            "oled_slowdown",
            "GPIO slowdown",
            "0–4; raise if the panel flickers.",
            FieldKind::Number {
                default: 3,
                min: 0,
                max: 4,
            },
        ),
        FieldDef::new(
            "hardware_mapping",
            "Hardware mapping",
            "HAT/wiring layout.",
            FieldKind::Enum {
                default: "adafruit-hat",
                choices: &[
                    ("adafruit-hat", "Adafruit HAT/Bonnet"),
                    ("adafruit-hat-pwm", "Adafruit HAT with PWM mod"),
                    ("regular", "Regular GPIO mapping"),
                    ("regular-pi1", "Regular, Pi 1"),
                    ("classic", "Classic mapping"),
                    ("classic-pi1", "Classic, Pi 1"),
                ],
            },
        ),
        FieldDef::new(
            "fail_on_error",
            "Fail on error",
            "Abort instead of continuing past a render error.",
            FieldKind::Bool { default: false },
        ),
    ]
}

/// Default matrix-options form.
pub fn matrix_default_form() -> Form {
    Form::from_defs(matrix_fields(), false)
}

/// Validate + assemble `matrix_options`, round-tripped through [`MatrixOptions`]
/// so the shape tracks the struct.
pub fn matrix_to_value(form: &Form) -> Result<Value, Vec<(String, String)>> {
    let v = form.to_value()?;
    let opts: crate::createjson::MatrixOptions =
        serde_json::from_value(v).map_err(|e| vec![("matrix_options".to_string(), e.to_string())])?;
    Ok(serde_json::to_value(opts).expect("MatrixOptions serializes"))
}

/// E-ink display block scalar options (the `modules` map is attached during
/// assembly, not here). `enabled` is forced true; `width`/`height` are dropped
/// when blank so the common case stays clean.
pub fn eink_block_fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "model",
            "Panel model",
            "Waveshare B/W panel.",
            FieldKind::Enum {
                default: "7in5_v2",
                choices: &[
                    ("7in5_v2", "7.5\" — 800×480 (recommended)"),
                    ("4in2", "4.2\" — 400×300"),
                    ("2in9", "2.9\" — 296×128"),
                    ("2in13", "2.13\" — 250×122"),
                ],
            },
        ),
        FieldDef::new(
            "threshold",
            "B/W threshold",
            "Luma cutoff 0–255 for black vs white.",
            FieldKind::Number {
                default: 128,
                min: 0,
                max: 255,
            },
        ),
        FieldDef::new(
            "rotation",
            "Rotation",
            "Degrees: 0, 90, 180, or 270.",
            FieldKind::Number {
                default: 0,
                min: 0,
                max: 270,
            },
        ),
        FieldDef::new(
            "refresh_ms",
            "Refresh (ms)",
            "Full-panel redraw interval.",
            FieldKind::Number {
                default: 2500,
                min: 100,
                max: 600_000,
            },
        ),
        FieldDef::new(
            "mode",
            "Backend mode",
            "'auto' picks hardware on a Pi, terminal off-Pi.",
            FieldKind::Text { default: "auto" },
        ),
        FieldDef::new(
            "emulate",
            "Emulate",
            "Force the terminal emulator backend.",
            FieldKind::Bool { default: false },
        ),
        FieldDef::new(
            "width",
            "Width override (px)",
            "Advanced: blank uses the model's native width.",
            FieldKind::OptionalNumber { min: 1, max: 4096 },
        ),
        FieldDef::new(
            "height",
            "Height override (px)",
            "Advanced: blank uses the model's native height.",
            FieldKind::OptionalNumber { min: 1, max: 4096 },
        ),
    ]
}

/// Default eink-block form.
pub fn eink_default_form() -> Form {
    Form::from_defs(eink_block_fields(), false)
}

/// Validate + assemble the eink block's scalar fields, forcing `enabled: true`
/// and dropping blank `width`/`height`.
pub fn eink_scalars_to_value(form: &Form) -> Result<Value, Vec<(String, String)>> {
    let v = form.to_value()?;
    let mut map = match v {
        Value::Object(m) => m,
        other => return Ok(other),
    };
    map.insert("enabled".to_string(), json!(true));
    // width/height are emitted as null when blank — drop them so the file only
    // carries the override when both are set.
    let w_set = map.get("width").map(|v| !v.is_null()).unwrap_or(false);
    let h_set = map.get("height").map(|v| !v.is_null()).unwrap_or(false);
    if !(w_set && h_set) {
        map.remove("width");
        map.remove("height");
    }
    Ok(Value::Object(map))
}

/// Collapse repeated section values into a single object (one) or array (many);
/// `None` for an empty section. The registry accepts either shape via
/// `one_or_many`, so this keeps the on-disk file natural.
pub fn fold_section(values: Vec<Value>) -> Option<Value> {
    match values.len() {
        0 => None,
        1 => values.into_iter().next(),
        _ => Some(Value::Array(values)),
    }
}
