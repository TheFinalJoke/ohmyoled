use crate::createjson::tui::field::{FieldDef, FieldKind};
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PiholeOptions {
    pub run: bool,
    pub base_url: String,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub token: Option<String>,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for PiholeOptions {
    fn default() -> Self {
        Self {
            run: true,
            base_url: "http://pi.hole".to_owned(),
            token: None,
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
            "e.g. http://pi.hole",
            FieldKind::Text {
                default: "http://pi.hole",
            },
        ),
        FieldDef::new(
            "token",
            "Admin API token",
            "Blank = unauthenticated (many home installs allow this).",
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
