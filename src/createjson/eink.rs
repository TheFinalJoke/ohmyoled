//! Interactive builder for the independent e-paper (`eink`) display block.
//!
//! Unlike the per-tile module builders, this configures a whole separate
//! display: its on/off flag, panel model, B/W threshold, and its own tile
//! selection (currently the weather tile — the one with an e-ink renderer).
//! Produces the full `eink` config object.

use crate::createjson::{ui, weather};
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
            ("4in2", "4.2\" — 400x300 (default)"),
            ("2in13", "2.13\" — 250x122"),
            ("2in9", "2.9\" — 296x128"),
            ("7in5_v2", "7.5\" — 800x480"),
        ],
        "4in2",
    );
    let threshold = read_threshold();

    // Build the display's own module set. Only the weather tile has an e-ink
    // renderer today; reuse its existing prompt so the config is real.
    let mut modules = Map::new();
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

    ui::success(&format!(
        "e-ink — {model} ({}), {} tile(s)",
        if enabled { "enabled" } else { "disabled" },
        modules.len()
    ));

    Ok(json!({
        "enabled": enabled,
        "model": model,
        "rotation": 0,
        "threshold": threshold,
        "modules": Value::Object(modules),
    }))
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
