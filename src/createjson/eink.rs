//! Interactive builder for the independent e-paper (`eink`) display block.
//!
//! Unlike the per-tile module builders, this configures a whole separate
//! display: its on/off flag, panel model, B/W threshold, and its own tile
//! selection (currently the weather tile — the one with an e-ink renderer).
//! Produces the full `eink` config object.

use crate::createjson::{aurora, hass, iss, launch, pihole, quake, stock, time, ui, weather};
use serde_json::{json, Map, Value};

pub fn configure() -> Result<Value, String> {
    ui::section("E-paper (e-ink) display");
    ui::hint(
        "Independent Waveshare B/W panel. Same data APIs as the LED matrix, but its own tiles \
         rendered as one large static screen.",
    );

    let enabled = ui::read_yes_no(
        "Enable the e-paper display (switches the panel output from LED to e-ink)?",
        false,
    );
    let model = ui::choose(
        "Panel model",
        &[
            ("7in5_v2", "7.5\" — 800x480 (recommended)"),
            ("4in2", "4.2\" — 400x300"),
            ("2in9", "2.9\" — 296x128"),
            ("2in13", "2.13\" — 250x122"),
        ],
        "7in5_v2",
    );
    let threshold = read_threshold();

    // Optional explicit resolution override for unlisted/custom panels.
    let (width, height) = if ui::read_yes_no(
        "Override resolution? (advanced — blank uses the model's native size)",
        false,
    ) {
        let w = read_dim("Width (px)");
        let h = read_dim("Height (px)");
        (Some(w), Some(h))
    } else {
        (None, None)
    };

    // Build the display's own module set. Time + weather have e-ink renderers
    // today; reuse their existing prompts so the config is real. The display
    // rotates through whichever tiles are added.
    let mut modules = Map::new();
    if ui::read_yes_no("Add the time tile to the e-paper display?", true) {
        let opts = time::configure();
        modules.insert(
            "time".to_string(),
            serde_json::to_value(opts).expect("TimeOptions serializes"),
        );
    }
    if ui::read_yes_no("Add the weather tile to the e-paper display?", true) {
        match weather::configure() {
            Ok(opts) => {
                modules.insert(
                    "weather".to_string(),
                    serde_json::to_value(opts).expect("WeatherOptions serializes"),
                );
            }
            Err(e) => ui::warn(&format!("weather tile skipped: {e}")),
        }
    }
    if ui::read_yes_no("Add the ISS tracker tile to the e-paper display?", false) {
        match iss::configure() {
            Ok(opts) => {
                modules.insert("iss".to_string(), serde_json::to_value(opts).expect("IssOptions serializes"));
            }
            Err(e) => ui::warn(&format!("iss tile skipped: {e}")),
        }
    }
    if ui::read_yes_no("Add the earthquake tile to the e-paper display?", false) {
        match quake::configure() {
            Ok(opts) => {
                modules.insert("quake".to_string(), serde_json::to_value(opts).expect("QuakeOptions serializes"));
            }
            Err(e) => ui::warn(&format!("quake tile skipped: {e}")),
        }
    }
    if ui::read_yes_no("Add the aurora tile to the e-paper display?", false) {
        match aurora::configure() {
            Ok(opts) => {
                modules.insert("aurora".to_string(), serde_json::to_value(opts).expect("AuroraOptions serializes"));
            }
            Err(e) => ui::warn(&format!("aurora tile skipped: {e}")),
        }
    }
    if ui::read_yes_no("Add the Pi-hole tile to the e-paper display?", false) {
        match pihole::configure() {
            Ok(opts) => {
                modules.insert("pihole".to_string(), serde_json::to_value(opts).expect("PiholeOptions serializes"));
            }
            Err(e) => ui::warn(&format!("pihole tile skipped: {e}")),
        }
    }
    if ui::read_yes_no("Add the stock tile to the e-paper display?", false) {
        match stock::configure() {
            Ok(opts) => {
                modules.insert("stock".to_string(), serde_json::to_value(opts).expect("StockOptions serializes"));
            }
            Err(e) => ui::warn(&format!("stock tile skipped: {e}")),
        }
    }
    if ui::read_yes_no("Add the rocket-launch tile to the e-paper display?", false) {
        match launch::configure() {
            Ok(opts) => {
                modules.insert("launch".to_string(), serde_json::to_value(opts).expect("LaunchOptions serializes"));
            }
            Err(e) => ui::warn(&format!("launch tile skipped: {e}")),
        }
    }
    if ui::read_yes_no("Add the Home Assistant tile to the e-paper display?", false) {
        match hass::configure() {
            Ok(opts) => {
                modules.insert("hass".to_string(), serde_json::to_value(opts).expect("HassOptions serializes"));
            }
            Err(e) => ui::warn(&format!("hass tile skipped: {e}")),
        }
    }

    ui::success(&format!(
        "e-ink — {model} ({}), {} tile(s)",
        if enabled { "enabled" } else { "disabled" },
        modules.len()
    ));

    let mut block = json!({
        "enabled": enabled,
        "model": model,
        "rotation": 0,
        "threshold": threshold,
        // Backend + emulation defaults; all overridable by OHMYOLED_EINK_* env
        // vars. "auto" picks hardware on a Pi and the terminal preview off-Pi.
        "mode": "auto",
        "emulate": false,
        "refresh_ms": 2500,
        "modules": Value::Object(modules),
    });
    // Only emit width/height when overridden, so the common case stays clean.
    if let (Some(w), Some(h)) = (width, height) {
        block["width"] = json!(w);
        block["height"] = json!(h);
    }
    Ok(block)
}

pub fn summary_line(v: &Value) -> String {
    let enabled = v.get("enabled").and_then(|b| b.as_bool()).unwrap_or(false);
    let model = v.get("model").and_then(|m| m.as_str()).unwrap_or("?");
    let tiles = v
        .get("modules")
        .and_then(|m| m.as_object())
        .map(|m| m.len())
        .unwrap_or(0);
    format!(
        "eink ({model}, {}, {tiles} tile(s))",
        if enabled { "enabled" } else { "disabled" }
    )
}

fn read_threshold() -> u8 {
    loop {
        let raw = ui::read_line_default("B/W luma threshold (0–255)", "128");
        match raw.trim().parse::<u8>() {
            Ok(v) => return v,
            _ => ui::warn("Expected an integer in [0, 255]"),
        }
    }
}

fn read_dim(label: &str) -> u32 {
    loop {
        match ui::read_required(label).trim().parse::<u32>() {
            Ok(v) if v > 0 => return v,
            _ => ui::warn("Expected a positive integer"),
        }
    }
}
