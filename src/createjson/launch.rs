use crate::createjson::tui::field::{FieldDef, FieldKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchOptions {
    pub run: bool,
    /// Case-insensitive substring whitelist matched against the
    /// Launch Library `launch_service_provider.name`. Empty = every
    /// upcoming launch.
    pub agency_filter: Vec<String>,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            run: true,
            agency_filter: Vec::new(),
            cache_ttl_secs: None,
        }
    }
}

/// TUI form schema.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "agency_filter",
            "Agency filter",
            "Comma-separated substrings (e.g. SpaceX, NASA); blank = all agencies.",
            FieldKind::StringList { default: "" },
        ),
        FieldDef::new(
            "cache_ttl_secs",
            "Cache TTL (secs)",
            super::CACHE_TTL_HELP,
            FieldKind::CacheTtl,
        ),
    ]
}
