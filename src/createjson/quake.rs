use crate::createjson::tui::field::{FieldDef, FieldKind};
use oledlib::api::quake::QuakeFeed;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct QuakeOptions {
    pub run: bool,
    pub feed: QuakeFeed,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for QuakeOptions {
    fn default() -> Self {
        Self {
            run: true,
            feed: QuakeFeed::default(),
            cache_ttl_secs: None,
        }
    }
}

/// TUI form schema. Slugs are the serde `snake_case` reprs of [`QuakeFeed`]
/// (what lands in the file), not the display-only `slug()` strings.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "feed",
            "Feed",
            "How chatty the USGS earthquake tile is.",
            FieldKind::Enum {
                default: "significant_day",
                choices: &[
                    ("significant_day", "Significant events, 24h (quietest)"),
                    ("m45_day", "M ≥ 4.5, 24h"),
                    ("m25_day", "M ≥ 2.5, 24h"),
                    ("all_day", "Everything, 24h (busiest)"),
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
