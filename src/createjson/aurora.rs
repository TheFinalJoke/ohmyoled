use crate::createjson::tui::field::{FieldDef, FieldKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuroraOptions {
    pub run: bool,
    /// Kp value at which the "AURORA LIKELY" banner appears. 5 matches
    /// NOAA's G1 minor-storm threshold and the registry default.
    pub alert_threshold: u8,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for AuroraOptions {
    fn default() -> Self {
        Self {
            run: true,
            alert_threshold: 5,
            cache_ttl_secs: None,
        }
    }
}

/// TUI form schema.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "alert_threshold",
            "Alert threshold (Kp)",
            "NOAA planetary K-index 0–9; the alert banner trips at or above this.",
            FieldKind::Number {
                default: 5,
                min: 0,
                max: 9,
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
