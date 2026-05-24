//! Normalized ISS state — what the collector hands to the renderer.

/// Where the ISS is *right now*, relative to the user.
#[derive(Debug, Clone)]
pub struct IssState {
    /// Great-circle ground distance from the user to the ISS sub-point (km).
    pub ground_distance_km: u32,
    /// True when the ISS is currently inside the user's visibility footprint
    /// (i.e. above the horizon for the user's location).
    pub overhead: bool,
    /// ISS sub-point latitude (degrees, -90..90).
    pub lat: f64,
    /// ISS sub-point longitude (degrees, -180..180).
    pub lon: f64,
    /// ISS altitude above the Earth's surface (km).
    pub altitude_km: f64,
    /// Orbital ground speed (km/s).
    pub velocity_kms: f64,
    /// `"daylight"` or `"eclipsed"` (sunlit vs in Earth's shadow). Affects
    /// whether the ISS is visually observable when overhead.
    pub visibility: String,
}
