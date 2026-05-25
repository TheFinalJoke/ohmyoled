use crate::createjson::ui;
use serde_json::{json, Value};

pub fn configure() -> Result<Value, String> {
    ui::section("Sport — Formula 1");
    ui::info("F1 uses the free jolpica/Ergast API — no configuration required.");
    ui::hint("Renderer rotates between the next race and the driver standings.");
    let cache_ttl_secs = ui::read_cache_ttl_secs();
    ui::success("F1 added to the rotation");
    Ok(json!({
        "run": true,
        "sport": "f1",
        "cache_ttl_secs": cache_ttl_secs,
    }))
}

pub fn summary_line(_entry: &Value) -> String {
    "sport: f1".to_string()
}
