//! Normalized flights data — what the collector hands to the renderer.

/// What the panel needs to know about the airspace right now.
#[derive(Debug, Clone)]
pub struct FlightSnapshot {
    /// Number of aircraft inside the configured radius (after filtering
    /// out states with no usable position).
    pub count: usize,
    /// Closest aircraft, or `None` when the airspace is empty.
    pub closest: Option<FlightInfo>,
}

/// One aircraft's display-ready state, relative to the user.
#[derive(Debug, Clone)]
pub struct FlightInfo {
    /// Callsign as supplied by OpenSky, trimmed. Empty when ATC hasn't
    /// announced one — the renderer falls back to the hex ICAO24.
    pub callsign: String,
    /// 24-bit ICAO transponder hex code (always present).
    pub icao24: String,
    /// Altitude above sea level, feet. Aircraft on the ground report 0.
    pub altitude_ft: u32,
    /// True when OpenSky's `on_ground` flag is set — useful so the
    /// renderer can hide a meaningless `FL000` label for taxiing planes.
    pub on_ground: bool,
    /// Great-circle distance from the user, km.
    pub distance_km: f32,
    /// Initial compass bearing from the user toward the aircraft, in
    /// degrees clockwise from true north (0..=360).
    pub bearing_deg: f32,
    /// Ground speed in knots. `None` when OpenSky didn't include
    /// `velocity_m_s` (sometimes the case for planes on the ground).
    pub ground_speed_kt: Option<u32>,
    /// Origin country (filed flight plan). Useful as a secondary label
    /// when no callsign is available.
    pub country: String,
}

/// Compass octant label for a bearing in degrees — eight-way (N, NE, E,
/// SE, S, SW, W, NW). Each slot is a 45° wide arc centered on the
/// cardinal direction, so 22°→N, 23°→NE, 67°→NE, 68°→E etc.
pub fn bearing_octant(bearing_deg: f32) -> &'static str {
    let b = bearing_deg.rem_euclid(360.0);
    // Shift so the N arc straddles 0° (337.5..=22.5).
    let idx = (((b + 22.5) % 360.0) / 45.0).floor() as usize;
    ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][idx.min(7)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearing_octant_cardinals() {
        assert_eq!(bearing_octant(0.0), "N");
        assert_eq!(bearing_octant(90.0), "E");
        assert_eq!(bearing_octant(180.0), "S");
        assert_eq!(bearing_octant(270.0), "W");
        assert_eq!(bearing_octant(360.0), "N");
    }

    #[test]
    fn bearing_octant_intercardinals() {
        assert_eq!(bearing_octant(45.0), "NE");
        assert_eq!(bearing_octant(135.0), "SE");
        assert_eq!(bearing_octant(225.0), "SW");
        assert_eq!(bearing_octant(315.0), "NW");
    }

    #[test]
    fn bearing_octant_arc_boundaries() {
        // N covers 337.5..=22.5; right at 22.5 it should flip to NE.
        assert_eq!(bearing_octant(22.0), "N");
        assert_eq!(bearing_octant(23.0), "NE");
        assert_eq!(bearing_octant(337.5), "N");
        assert_eq!(bearing_octant(336.0), "NW");
    }

    #[test]
    fn bearing_octant_handles_out_of_range_input() {
        // Negative / over-360 input wraps via rem_euclid.
        assert_eq!(bearing_octant(-1.0), "N");
        assert_eq!(bearing_octant(361.0), "N");
        assert_eq!(bearing_octant(720.0 + 90.0), "E");
    }
}
