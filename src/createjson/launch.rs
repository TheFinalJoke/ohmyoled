use crate::createjson::ui;
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

pub fn configure() -> Result<LaunchOptions, String> {
    ui::section("Launch");
    ui::hint("Next orbital launch via Launch Library 2. No API key needed.");

    let raw = ui::read_line_default(
        "Agency filter (comma-separated substrings, blank = all)",
        "",
    );
    let agency_filter: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let cache_ttl_secs = ui::read_cache_ttl_secs();
    if agency_filter.is_empty() {
        ui::success("Launch — all providers");
    } else {
        ui::success(&format!("Launch — filtering {}", agency_filter.join(", ")));
    }
    Ok(LaunchOptions {
        run: true,
        agency_filter,
        cache_ttl_secs,
    })
}

pub fn summary_line(opts: &LaunchOptions) -> String {
    if opts.agency_filter.is_empty() {
        "launch (all agencies)".to_string()
    } else {
        format!("launch (filter: {})", opts.agency_filter.join(","))
    }
}
