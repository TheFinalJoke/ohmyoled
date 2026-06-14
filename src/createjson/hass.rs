use crate::createjson::tui::field::{FieldDef, FieldKind};
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
    /// `"state"` (current), `"historical"` (recent list), or `"graph"`
    /// (sparkline). Falls back to `"state"` if history isn't available.
    #[serde(default = "default_display_mode")]
    pub display_mode: String,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

fn default_display_mode() -> String {
    "state".to_owned()
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
            display_mode: default_display_mode(),
            cache_ttl_secs: None,
        }
    }
}

/// TUI form schema.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "base_url",
            "Base URL",
            "e.g. http://homeassistant.local:8123",
            FieldKind::Text {
                default: "http://homeassistant.local:8123",
            },
        ),
        FieldDef::new(
            "token",
            "Access token",
            "Long-lived access token (required).",
            FieldKind::Text { default: "" },
        ),
        FieldDef::new(
            "entity_id",
            "Entity id",
            "e.g. sensor.kitchen_temp (required).",
            FieldKind::Text {
                default: "sensor.kitchen_temp",
            },
        ),
        FieldDef::new(
            "label",
            "Label override",
            "Blank = use the entity's friendly_name.",
            FieldKind::OptionalText { default: "" },
        ),
        FieldDef::new(
            "alarm_state",
            "Alarm state",
            "State that flips to the alarm color (case-insensitive); blank = none.",
            FieldKind::OptionalText { default: "" },
        ),
        FieldDef::new(
            "nominal_color",
            "Nominal color (R G B)",
            "Normal-state color, 0–255 each.",
            FieldKind::Rgb {
                default: (120, 220, 120),
            },
        ),
        FieldDef::new(
            "alarm_color",
            "Alarm color (R G B)",
            "Alarm-state color, 0–255 each.",
            FieldKind::Rgb {
                default: (255, 60, 60),
            },
        ),
        FieldDef::new(
            "display_mode",
            "Display mode",
            "How the value is drawn.",
            FieldKind::Enum {
                default: "state",
                choices: &[
                    ("state", "Current value"),
                    ("historical", "Recent list"),
                    ("graph", "Sparkline"),
                ],
            },
        ),
        FieldDef::new(
            "cache_ttl_secs",
            "Cache TTL (secs)",
            super::CACHE_TTL_HELP,
            FieldKind::CacheTtl,
        ),
    ]
}
