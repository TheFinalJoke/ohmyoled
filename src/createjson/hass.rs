use crate::createjson::ui;
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HassOptions {
    pub run: bool,
    pub base_url: String,
    pub token: String,
    pub entity_id: String,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub label: Option<String>,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub alarm_state: Option<String>,
    pub nominal_color: (u8, u8, u8),
    pub alarm_color: (u8, u8, u8),
}

impl Default for HassOptions {
    fn default() -> Self {
        Self {
            run: true,
            base_url: "http://homeassistant.local:8123".to_owned(),
            token: String::new(),
            entity_id: "sensor.kitchen_temp".to_owned(),
            label: None,
            alarm_state: None,
            nominal_color: (120, 220, 120),
            alarm_color: (255, 60, 60),
        }
    }
}

pub fn configure() -> Result<HassOptions, String> {
    ui::section("Home Assistant");
    ui::hint("Renders any HASS entity. Needs base URL + long-lived access token + entity id.");

    let base_url = ui::read_line_default(
        "Base URL (e.g. http://homeassistant.local:8123)",
        "http://homeassistant.local:8123",
    );
    let token = ui::read_required("HASS long-lived access token");
    let entity_id = ui::read_required("Entity id (e.g. sensor.kitchen_temp)");

    let label_raw = ui::read_line_default("Display label override (blank = use friendly_name)", "");
    let label = if label_raw.trim().is_empty() {
        None
    } else {
        Some(label_raw.trim().to_string())
    };

    let alarm_raw = ui::read_line_default(
        "Alarm state (case-insensitive; flips to alarm color when matched; blank = no alarm)",
        "",
    );
    let alarm_state = if alarm_raw.trim().is_empty() {
        None
    } else {
        Some(alarm_raw.trim().to_string())
    };

    let nominal_color = read_rgb("Nominal-state RGB", (120, 220, 120));
    let alarm_color = read_rgb("Alarm-state RGB", (255, 60, 60));

    ui::success(&format!("HASS — {entity_id} @ {base_url}"));
    Ok(HassOptions {
        run: true,
        base_url: base_url.trim().to_string(),
        token: token.trim().to_string(),
        entity_id: entity_id.trim().to_string(),
        label,
        alarm_state,
        nominal_color,
        alarm_color,
    })
}

pub fn summary_line(opts: &HassOptions) -> String {
    format!("hass ({} @ {})", opts.entity_id, opts.base_url)
}

/// Re-prompt until three 0..=255 integers parse out of any common
/// separator (space/comma/slash/semicolon). Used for both the
/// nominal and alarm color prompts.
fn read_rgb(label: &str, default: (u8, u8, u8)) -> (u8, u8, u8) {
    let default_str = format!("{} {} {}", default.0, default.1, default.2);
    loop {
        let raw = ui::read_line_default(label, &default_str);
        let parts: Vec<&str> = raw
            .split(|c: char| c.is_whitespace() || c == ',' || c == '/' || c == ';')
            .filter(|p| !p.is_empty())
            .collect();
        if parts.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].parse::<u8>(),
                parts[1].parse::<u8>(),
                parts[2].parse::<u8>(),
            ) {
                return (r, g, b);
            }
        }
        ui::warn("Expected three integers in 0..=255, e.g. '120 220 120'");
    }
}
