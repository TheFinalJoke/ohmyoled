use crate::createjson::tui::field::{FieldDef, FieldKind};
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeOptions {
    pub run: bool,
    pub color: (i32, i32, i32),
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub time_format: Option<String>,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub timezone: Option<String>,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for TimeOptions {
    fn default() -> Self {
        Self {
            run: true,
            color: (255, 255, 255),
            time_format: None,
            timezone: None,
            cache_ttl_secs: None,
        }
    }
}

/// TUI form schema. `time_format` "system" is post-processed to JSON `null`
/// (legacy semantics) in `form_module::section_to_value`.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "color",
            "Color (R G B)",
            "Clock color, three values 0–255.",
            FieldKind::Rgb {
                default: (255, 255, 255),
            },
        ),
        FieldDef::new(
            "time_format",
            "Time format",
            "Clock style.",
            FieldKind::Enum {
                default: "system",
                choices: &[
                    ("system", "System locale"),
                    ("12h", "12-hour with AM/PM"),
                    ("24h", "24-hour"),
                ],
            },
        ),
        FieldDef::new(
            "timezone",
            "Timezone",
            "IANA name (e.g. America/Chicago); blank = system.",
            FieldKind::OptionalText { default: "" },
        ),
        FieldDef::new(
            "cache_ttl_secs",
            "Cache TTL (secs)",
            super::CACHE_TTL_HELP,
            FieldKind::CacheTtl,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_legacy() {
        let t = TimeOptions::default();
        assert!(t.run);
        assert_eq!(t.color, (255, 255, 255));
        assert!(t.time_format.is_none());
        assert!(t.timezone.is_none());
    }

    #[test]
    fn null_string_is_none() {
        let json = r#"{"run":true,"color":[255,255,255],"time_format":"null","timezone":"null"}"#;
        let parsed: TimeOptions = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.time_format, None);
        assert_eq!(parsed.timezone, None);
    }

    #[test]
    fn real_string_kept() {
        let json = r#"{"run":true,"color":[255,255,255],"time_format":"24h","timezone":"UTC"}"#;
        let parsed: TimeOptions = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.time_format.as_deref(), Some("24h"));
        assert_eq!(parsed.timezone.as_deref(), Some("UTC"));
    }

    /// The default form serializes to the same JSON as `TimeOptions::default`
    /// (after the `system` ⇒ `null` post-process), guarding against schema
    /// drift from the struct.
    #[test]
    fn default_form_matches_struct_default() {
        let form = crate::createjson::tui::form_module::default_form("time");
        let v = crate::createjson::tui::form_module::section_to_value("time", &form).unwrap();
        assert_eq!(v["color"], serde_json::json!([255, 255, 255]));
        assert!(v["time_format"].is_null());
        assert!(v["timezone"].is_null());
        assert_eq!(v["run"], serde_json::json!(true));
    }
}
