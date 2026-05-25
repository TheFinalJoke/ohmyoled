use crate::createjson::ui;
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

pub fn configure() -> Result<IssOptions, String> {
    ui::section("ISS");
    ui::hint("Distance + overhead flag for the International Space Station. Needs your lat/lon.");

    let lat = read_coord("Latitude (degrees, -90..90)", 40.7128, -90.0, 90.0);
    let lon = read_coord("Longitude (degrees, -180..180)", -74.0060, -180.0, 180.0);
    let cache_ttl_secs = ui::read_cache_ttl_secs();

    ui::success(&format!("ISS — lat={lat:.4}, lon={lon:.4}"));
    Ok(IssOptions { run: true, lat, lon, cache_ttl_secs })
}

pub fn summary_line(opts: &IssOptions) -> String {
    format!("iss (lat={:.4}, lon={:.4})", opts.lat, opts.lon)
}

/// Re-prompt until a parseable f64 in `min..=max` is supplied. Shared
/// with the other lat/lon-bearing modules via a tiny helper rather
/// than copy-pasting the loop.
fn read_coord(label: &str, default: f64, min: f64, max: f64) -> f64 {
    loop {
        let raw = ui::read_line_default(label, &format!("{default}"));
        match raw.trim().parse::<f64>() {
            Ok(v) if (min..=max).contains(&v) => return v,
            _ => ui::warn(&format!("Expected a number in [{min}, {max}]")),
        }
    }
}

pub(crate) fn read_coord_pub(label: &str, default: f64, min: f64, max: f64) -> f64 {
    read_coord(label, default, min, max)
}
