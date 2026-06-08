use crate::createjson::tui::field::{FieldDef, FieldKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct IssOptions {
    pub run: bool,
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for IssOptions {
    fn default() -> Self {
        // NYC — matches the example configs so the dev-mode and
        // starter outputs both pick a plausible default location.
        Self {
            run: true,
            lat: 40.7128,
            lon: -74.0060,
            cache_ttl_secs: None,
        }
    }
}

/// TUI form schema.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "lat",
            "Latitude",
            "Degrees, -90..90.",
            FieldKind::Float {
                default: 40.7128,
                min: -90.0,
                max: 90.0,
            },
        ),
        FieldDef::new(
            "lon",
            "Longitude",
            "Degrees, -180..180.",
            FieldKind::Float {
                default: -74.0060,
                min: -180.0,
                max: 180.0,
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
