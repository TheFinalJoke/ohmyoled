use crate::createjson::iss::read_coord_pub as read_coord;
use crate::createjson::ui;
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

pub fn configure() -> Result<FlightsOptions, String> {
    ui::section("Flights");
    ui::hint("Aircraft within a circular bbox around your location, via OpenSky (anon).");

    let lat = read_coord("Latitude (degrees, -90..90)", 40.7128, -90.0, 90.0);
    let lon = read_coord("Longitude (degrees, -180..180)", -74.0060, -180.0, 180.0);
    let radius_km = loop {
        let raw = ui::read_line_default("Search radius (km, 1..=500)", "80");
        match raw.trim().parse::<f32>() {
            Ok(v) if (1.0..=500.0).contains(&v) => break v,
            _ => ui::warn("Expected a number in 1..=500"),
        }
    };

    let airborne_only = ui::read_yes_no("Only show airborne traffic (skip planes at the airport)?", true);
    let cache_ttl_secs = ui::read_cache_ttl_secs();
    ui::success(&format!("Flights — lat={lat:.4}, lon={lon:.4}, radius={radius_km}km"));
    Ok(FlightsOptions {
        run: true,
        lat,
        lon,
        radius_km,
        airborne_only,
        cache_ttl_secs,
    })
}

pub fn summary_line(opts: &FlightsOptions) -> String {
    format!(
        "flights (lat={:.4}, lon={:.4}, r={}km)",
        opts.lat, opts.lon, opts.radius_km
    )
}
