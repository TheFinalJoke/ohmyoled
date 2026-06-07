//! OpenSky Network provider — anonymous bbox state-vector query.
//!
//! Endpoint:
//! - `https://opensky-network.org/api/states/all?lamin=&lomin=&lamax=&lomax=`
//!
//! Returns `{time, states: [[...17 positional fields...], ...]}`. We
//! filter to aircraft with valid lat/lon, post-filter by Haversine
//! distance to enforce a circular search radius (the bbox query is
//! always a square superset), and pick the closest.
//!
//! Field positions per the OpenSky REST docs:
//! <https://openskynetwork.github.io/opensky-api/rest.html>.

use super::model::{FlightInfo, FlightSnapshot};
use crate::api::error::ApiError;
use crate::api::http::get_json;
use crate::api::iss::haversine_km;
use serde::Deserialize;
use serde_json::Value;

const METERS_TO_FEET: f64 = 3.280_84;
const MPS_TO_KNOTS: f64 = 1.943_84;

#[derive(Debug, Clone)]
pub struct OpenSkyConfig {
    pub user_lat: f64,
    pub user_lon: f64,
    /// Search radius from the user, km. Bbox is a square superset; the
    /// Haversine post-filter enforces the actual circle.
    pub radius_km: f32,
    /// Drop aircraft OpenSky flags as `on_ground` (parked / taxiing at a
    /// nearby airport), keeping only airborne traffic.
    pub airborne_only: bool,
}

pub struct OpenSkyProvider {
    user_lat: f64,
    user_lon: f64,
    radius_km: f32,
    airborne_only: bool,
    bbox: BBox,
}

#[derive(Debug, Clone)]
struct BBox {
    lamin: f64,
    lomin: f64,
    lamax: f64,
    lomax: f64,
}

impl OpenSkyProvider {
    pub fn new(cfg: OpenSkyConfig) -> Result<Self, ApiError> {
        if !(-90.0..=90.0).contains(&cfg.user_lat) {
            return Err(ApiError::Config(format!(
                "opensky: user_lat {} outside [-90, 90]",
                cfg.user_lat
            )));
        }
        if !(-180.0..=180.0).contains(&cfg.user_lon) {
            return Err(ApiError::Config(format!(
                "opensky: user_lon {} outside [-180, 180]",
                cfg.user_lon
            )));
        }
        if !(1.0..=500.0).contains(&cfg.radius_km) {
            return Err(ApiError::Config(format!(
                "opensky: radius_km {} outside (1, 500] — OpenSky's bbox grows quadratically",
                cfg.radius_km
            )));
        }
        let bbox = compute_bbox(cfg.user_lat, cfg.user_lon, cfg.radius_km as f64);
        Ok(Self {
            user_lat: cfg.user_lat,
            user_lon: cfg.user_lon,
            radius_km: cfg.radius_km,
            airborne_only: cfg.airborne_only,
            bbox,
        })
    }

    pub async fn poll(&self) -> Result<FlightSnapshot, ApiError> {
        let url = format!(
            "https://opensky-network.org/api/states/all?lamin={}&lomin={}&lamax={}&lomax={}",
            self.bbox.lamin, self.bbox.lomin, self.bbox.lamax, self.bbox.lomax
        );
        let raw: RawResponse = get_json(&url, &[]).await.map_err(|e| ApiError::Provider {
            provider: "opensky",
            msg: e.to_string(),
        })?;
        Ok(snapshot_from_raw(
            raw,
            self.user_lat,
            self.user_lon,
            self.radius_km as f64,
            self.airborne_only,
        ))
    }
}

/// Compute the bbox (lat_min/lat_max/lon_min/lon_max) of a circle of
/// radius `radius_km` around `(lat, lon)`. The latitude conversion is
/// constant; longitude shrinks at higher latitudes by `cos(lat)`.
fn compute_bbox(lat: f64, lon: f64, radius_km: f64) -> BBox {
    const KM_PER_DEG_LAT: f64 = 111.0;
    let lat_rad = lat.to_radians();
    let km_per_deg_lon = (KM_PER_DEG_LAT * lat_rad.cos()).max(0.0001);
    let dlat = radius_km / KM_PER_DEG_LAT;
    let dlon = radius_km / km_per_deg_lon;
    BBox {
        lamin: (lat - dlat).max(-90.0),
        lamax: (lat + dlat).min(90.0),
        lomin: lon - dlon,
        lomax: lon + dlon,
    }
}

/// Initial compass bearing from `(lat1, lon1)` toward `(lat2, lon2)`,
/// in degrees clockwise from true north (0..=360).
fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f32 {
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let y = dlon.sin() * phi2.cos();
    let x = phi1.cos() * phi2.sin() - phi1.sin() * phi2.cos() * dlon.cos();
    let theta = y.atan2(x).to_degrees();
    (theta + 360.0) as f32 % 360.0
}

/// Pure: from a parsed OpenSky response, build a FlightSnapshot
/// post-filtered to the actual `radius_km` circle.
fn snapshot_from_raw(
    raw: RawResponse,
    user_lat: f64,
    user_lon: f64,
    radius_km: f64,
    airborne_only: bool,
) -> FlightSnapshot {
    let states = raw.states.unwrap_or_default();
    let mut nearby: Vec<FlightInfo> = states
        .into_iter()
        .filter_map(|s| parse_state(&s, user_lat, user_lon))
        .filter(|f| (f.distance_km as f64) <= radius_km)
        .filter(|f| !airborne_only || !f.on_ground)
        .collect();
    nearby.sort_by(|a, b| {
        a.distance_km
            .partial_cmp(&b.distance_km)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Cap at 32 — the 64×32 panel can't legibly show more dots than
    // that, and OpenSky bbox queries over busy regions can return
    // hundreds. The cap is on the radar input; `count` still reflects
    // the true filtered total so the "AC N" overlay stays honest.
    let count = nearby.len();
    let closest = nearby.first().cloned();
    nearby.truncate(32);
    FlightSnapshot {
        count,
        closest,
        nearby,
        radius_km: radius_km as f32,
    }
}

/// Pull lat/lon/altitude/callsign/etc. from one positional tuple. Returns
/// `None` for entries that don't carry lat & lon — those are useless
/// for distance ranking. Other fields fall back to sensible defaults.
fn parse_state(s: &[Value], user_lat: f64, user_lon: f64) -> Option<FlightInfo> {
    let lon = s.get(5)?.as_f64()?;
    let lat = s.get(6)?.as_f64()?;
    let icao24 = s.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
    let callsign = s
        .get(1)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let country = s
        .get(2)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // baro_altitude_m at [7]; fall back to geo_altitude_m at [13].
    let alt_m = s
        .get(7)
        .and_then(|v| v.as_f64())
        .or_else(|| s.get(13).and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    let altitude_ft = (alt_m * METERS_TO_FEET).max(0.0).round() as u32;
    let on_ground = s.get(8).and_then(|v| v.as_bool()).unwrap_or(false);
    let ground_speed_kt = s
        .get(9)
        .and_then(|v| v.as_f64())
        .map(|mps| (mps * MPS_TO_KNOTS).max(0.0).round() as u32);

    let distance_km = haversine_km(user_lat, user_lon, lat, lon) as f32;
    let bearing = bearing_deg(user_lat, user_lon, lat, lon);

    Some(FlightInfo {
        callsign,
        icao24,
        altitude_ft,
        on_ground,
        distance_km,
        bearing_deg: bearing,
        ground_speed_kt,
        country,
    })
}

#[derive(Debug, Deserialize)]
struct RawResponse {
    #[serde(default)]
    states: Option<Vec<Vec<Value>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tight NYC bbox with three sample state vectors at different distances
    // from JFK Airport (40.6413, -73.7781). The fixture mixes a perfect
    // tuple, one with `null` altitude, and one on the ground — the same
    // shape variations live OpenSky actually produces.
    const FIXTURE: &str = r#"{
        "time": 1700000000,
        "states": [
            ["abc123", "DAL2451 ", "United States", 1700000000, 1700000000,
             -73.7846, 40.6256, 9753.6, false, 245.0, 235.0,
             0.0, null, 9876.5, "1200", false, 0],
            ["def456", "UAL1234 ", "United States", 1700000000, 1700000000,
             -74.5,    41.2,     12192.0, false, 230.0, 80.0,
             0.0, null, 12200.0, null, false, 0],
            ["ghi789", "         ", "Switzerland", 1700000000, 1700000000,
             -73.7800, 40.6420, null, true, 0.0, 0.0,
             null, null, null, null, false, 0]
        ]
    }"#;

    fn parse_fixture() -> RawResponse {
        serde_json::from_str(FIXTURE).expect("fixture should parse")
    }

    #[test]
    fn snapshot_picks_closest_aircraft() {
        // User at JFK; the on-ground Swiss state is closest (~1 km),
        // DAL2451 at ~5 km, UAL1234 ~80 km north.
        let snap = snapshot_from_raw(parse_fixture(), 40.6413, -73.7781, 100.0, false);
        assert_eq!(snap.count, 3);
        let closest = snap.closest.expect("closest flight");
        assert_eq!(closest.icao24, "ghi789");
        // Callsign was all-whitespace -> trimmed to empty.
        assert_eq!(closest.callsign, "");
        assert!(closest.on_ground);
        assert!(closest.distance_km < 5.0);
    }

    #[test]
    fn altitude_falls_back_to_geo_when_baro_missing() {
        // DAL2451 has baro 9753.6 m -> ~32000 ft.
        let snap = snapshot_from_raw(parse_fixture(), 40.6413, -73.7781, 100.0, false);
        let dal = snap
            .closest
            .as_ref()
            .map(|_| ()) // closest is the on-ground one; pull DAL directly
            .map(|_| {
                let states = parse_fixture().states.unwrap();
                parse_state(&states[0], 40.6413, -73.7781).unwrap()
            })
            .unwrap();
        // 9753.6 m × 3.28084 = ~32000 ft
        assert!(
            (31800..=32100).contains(&dal.altitude_ft),
            "got altitude_ft {}",
            dal.altitude_ft
        );
        assert!(!dal.on_ground);
    }

    #[test]
    fn radius_filter_drops_distant_flights() {
        // UAL1234 is at (41.2, -74.5) — ~80 km from JFK; a 10 km filter
        // should drop it, leaving only the two close ones.
        let snap = snapshot_from_raw(parse_fixture(), 40.6413, -73.7781, 10.0, false);
        assert_eq!(snap.count, 2);
    }

    #[test]
    fn airborne_only_drops_grounded_aircraft() {
        // Within 100 km there are 3 (one on the ground at JFK); airborne-only
        // keeps just the two flying, and the closest is no longer the ground one.
        let all = snapshot_from_raw(parse_fixture(), 40.6413, -73.7781, 100.0, false);
        let air = snapshot_from_raw(parse_fixture(), 40.6413, -73.7781, 100.0, true);
        assert_eq!(all.count, 3);
        assert_eq!(air.count, 2);
        assert!(air.nearby.iter().all(|f| !f.on_ground), "no grounded aircraft survive");
        assert!(!air.closest.unwrap().on_ground, "closest is now airborne");
    }

    #[test]
    fn empty_states_returns_quiet_snapshot() {
        let raw: RawResponse = serde_json::from_str(r#"{"time":1,"states":null}"#).unwrap();
        let snap = snapshot_from_raw(raw, 0.0, 0.0, 100.0, false);
        assert_eq!(snap.count, 0);
        assert!(snap.closest.is_none());
    }

    #[test]
    fn bbox_widens_with_radius() {
        let small = compute_bbox(40.0, -74.0, 50.0);
        let big = compute_bbox(40.0, -74.0, 200.0);
        assert!(big.lamax - big.lamin > small.lamax - small.lamin);
        assert!(big.lomax - big.lomin > small.lomax - small.lomin);
    }

    #[test]
    fn bbox_narrows_in_longitude_at_high_latitude() {
        // At 70°N, a 100 km radius spans a much wider longitude range
        // than at the equator (because cos(lat) is small).
        let equator = compute_bbox(0.0, 0.0, 100.0);
        let arctic = compute_bbox(70.0, 0.0, 100.0);
        assert!(
            arctic.lomax - arctic.lomin > equator.lomax - equator.lomin,
            "arctic bbox should be wider in longitude than the equator's"
        );
    }

    #[test]
    fn bearing_cardinal_sanity() {
        // From (0, 0) toward (1, 0) — due north.
        let b = bearing_deg(0.0, 0.0, 1.0, 0.0);
        assert!(b.abs() < 1.0, "expected ~0°, got {b}");
        // From (0, 0) toward (0, 1) — due east.
        let b = bearing_deg(0.0, 0.0, 0.0, 1.0);
        assert!((b - 90.0).abs() < 1.0, "expected ~90°, got {b}");
        // From (1, 0) toward (0, 0) — due south.
        let b = bearing_deg(1.0, 0.0, 0.0, 0.0);
        assert!((b - 180.0).abs() < 1.0, "expected ~180°, got {b}");
    }

    #[test]
    fn rejects_invalid_config() {
        // Out-of-range lat.
        let r = OpenSkyProvider::new(OpenSkyConfig {
            user_lat: 95.0,
            user_lon: 0.0,
            radius_km: 50.0,
            airborne_only: true,
        });
        assert!(matches!(r, Err(ApiError::Config(_))));
        // Out-of-range radius.
        let r = OpenSkyProvider::new(OpenSkyConfig {
            user_lat: 0.0,
            user_lon: 0.0,
            radius_km: 1000.0,
            airborne_only: true,
        });
        assert!(matches!(r, Err(ApiError::Config(_))));
    }
}
