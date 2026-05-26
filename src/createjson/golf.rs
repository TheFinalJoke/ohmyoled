use crate::createjson::ui;
use serde_json::{json, Value};

pub fn configure() -> Result<Value, String> {
    ui::section("Sport — Golf");
    ui::hint("ESPN's free leaderboard feed; no API key required.");

    let tour = ui::choose(
        "Which tour should the panel follow?",
        &[
            ("pga", "PGA Tour"),
            ("lpga", "LPGA Tour"),
            ("champions", "PGA Champions Tour"),
            ("korn", "Korn Ferry Tour"),
            ("liv", "LIV Golf"),
        ],
        "pga",
    );

    let cache_ttl_secs = ui::read_cache_ttl_secs();
    ui::success(&format!("Golf — {} tour configured", tour.to_uppercase()));
    Ok(json!({
        "run": true,
        "sport": "golf",
        "tour": tour,
        "cache_ttl_secs": cache_ttl_secs,
    }))
}

pub fn summary_line(entry: &Value) -> String {
    let tour = entry
        .get("tour")
        .and_then(Value::as_str)
        .unwrap_or("pga")
        .to_uppercase();
    format!("sport: golf ({tour})")
}
