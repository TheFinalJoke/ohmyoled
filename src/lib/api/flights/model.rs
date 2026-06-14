//! Normalized flights data — what the collector hands to the renderer.

/// What the panel needs to know about the airspace right now.
#[derive(Debug, Clone)]
pub struct FlightSnapshot {
    /// Number of aircraft inside the configured radius (after filtering
    /// out states with no usable position). Equal to `nearby.len()`.
    pub count: usize,
    /// Closest aircraft, or `None` when the airspace is empty.
    /// Equivalent to `nearby.first().cloned()` — kept as a separate
    /// field so the renderer's corner overlays read it without an
    /// `Option`-chained `.first()`.
    pub closest: Option<FlightInfo>,
    /// All aircraft inside the configured radius, sorted ascending by
    /// distance. The radar renderer plots one dot per entry; the
    /// text-mode renderer historically only used `closest` and ignores
    /// this. Capped at 32 to keep the plot legible on the 64×32 panel.
    pub nearby: Vec<FlightInfo>,
    /// The search radius the collector was configured with, km. The
    /// renderer scales dot positions against this so a flight at the
    /// edge of the radius lands at the edge of the radar circle.
    pub radius_km: f32,
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
    /// True track / heading — the compass direction the aircraft is *flying*,
    /// degrees clockwise from true north. `None` when OpenSky omitted it.
    pub heading_deg: Option<f32>,
    /// Origin country (filed flight plan). Useful as a secondary label
    /// when no callsign is available.
    pub country: String,
}

impl FlightInfo {
    /// A human-friendly label: `"United 123"` when the callsign's 3-letter
    /// airline prefix is recognised, otherwise the raw callsign, otherwise the
    /// ICAO24 hex code.
    pub fn label(&self) -> String {
        let cs = self.callsign.trim();
        if cs.is_empty() {
            return self.icao24.to_uppercase();
        }
        if cs.len() > 3 {
            let (prefix, rest) = cs.split_at(3);
            if prefix.chars().all(|c| c.is_ascii_alphabetic()) {
                if let Some(name) = airline_name(prefix) {
                    return format!("{name} {}", rest.trim());
                }
            }
        }
        cs.to_string()
    }
}

/// Map a 3-letter ICAO airline code (the callsign prefix) to a short airline
/// name. Covers the common carriers; unknown codes return `None` so callers
/// fall back to the raw callsign.
pub fn airline_name(icao3: &str) -> Option<&'static str> {
    Some(match icao3.to_ascii_uppercase().as_str() {
        "UAL" => "United",
        "DAL" => "Delta",
        "AAL" => "American",
        "SWA" => "Southwest",
        "JBU" => "JetBlue",
        "ASA" => "Alaska",
        "FFT" => "Frontier",
        "NKS" => "Spirit",
        "SKW" => "SkyWest",
        "ACA" => "Air Canada",
        "WJA" => "WestJet",
        "AMX" => "Aeromexico",
        "BAW" => "British Airways",
        "VIR" => "Virgin Atlantic",
        "DLH" => "Lufthansa",
        "AFR" => "Air France",
        "KLM" => "KLM",
        "EZY" => "easyJet",
        "RYR" => "Ryanair",
        "EIN" => "Aer Lingus",
        "ICE" => "Icelandair",
        "UAE" => "Emirates",
        "QTR" => "Qatar",
        "THY" => "Turkish",
        "SIA" => "Singapore",
        "CPA" => "Cathay",
        "QFA" => "Qantas",
        "ANA" => "ANA",
        "JAL" => "Japan Airlines",
        "FDX" => "FedEx",
        "UPS" => "UPS",
        "GTI" => "Atlas Air",
        _ => return None,
    })
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

    fn info(callsign: &str, icao24: &str) -> FlightInfo {
        FlightInfo {
            callsign: callsign.into(),
            icao24: icao24.into(),
            altitude_ft: 34_000,
            on_ground: false,
            distance_km: 10.0,
            bearing_deg: 0.0,
            ground_speed_kt: Some(440),
            heading_deg: Some(90.0),
            country: "United States".into(),
        }
    }

    #[test]
    fn label_decodes_airline_then_falls_back() {
        assert_eq!(info("UAL123", "a1").label(), "United 123");
        assert_eq!(info("BAW276", "a2").label(), "British Airways 276");
        // Unknown airline prefix → raw callsign.
        assert_eq!(info("ZZZ999", "a3").label(), "ZZZ999");
        // No callsign → ICAO24 hex (uppercased).
        assert_eq!(info("  ", "abc123").label(), "ABC123");
    }

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
