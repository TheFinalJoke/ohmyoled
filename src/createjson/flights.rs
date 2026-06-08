use crate::createjson::tui::field::{FieldDef, FieldKind};
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlightsOptions {
    pub run: bool,
    pub lat: f64,
    pub lon: f64,
    pub radius_km: f32,
    /// Only show airborne traffic (drop planes on the ground at nearby airports).
    #[serde(default = "default_true")]
    pub airborne_only: bool,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for FlightsOptions {
    fn default() -> Self {
        Self {
            run: true,
            lat: 40.7128,
            lon: -74.0060,
            radius_km: 80.0,
            airborne_only: true,
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
            "radius_km",
            "Search radius (km)",
            "1..500.",
            FieldKind::Float {
                default: 80.0,
                min: 1.0,
                max: 500.0,
            },
        ),
        FieldDef::new(
            "airborne_only",
            "Airborne only",
            "Skip planes parked at nearby airports.",
            FieldKind::Bool { default: true },
        ),
        FieldDef::new(
            "cache_ttl_secs",
            "Cache TTL (secs)",
            super::CACHE_TTL_HELP,
            FieldKind::CacheTtl,
        ),
    ]
}
