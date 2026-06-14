//! A compact, asset-free world map for e-paper tiles.
//!
//! Land is modeled as a set of **coastline polygons** (lat/lon vertices) tested
//! with point-in-polygon, plus a few ellipses for small islands and an
//! Antarctic band — no map file or network needed. Sub-continents are split
//! into overlapping polygons whose union is contiguous, which keeps each loop
//! simple while still reproducing real continent silhouettes (peninsulas,
//! tapers, big bays). Tiles stipple the land into a box and plot points (the
//! ISS, an earthquake epicenter, …) with [`project`].

use crate::matrix::eink::layout::{fill_rect, rect};
use image::RgbImage;
use ohmyoled_matrix::Color;

// ── Coastline polygons ─────────────────────────────────────────────────────
// Vertices are (lat, lon), traced around each landmass. Not survey-grade, but
// topologically faithful. Enclosed/marginal seas (Hudson Bay, the Baltic, the
// Persian Gulf, the Black Sea) are filled rather than cut out — invisible at
// tile scale. Adjacent pieces overlap so Eurasia/the Americas read as one.

const NORTH_AMERICA: &[(f32, f32)] = &[
    (71.0, -156.0), // Barrow (Alaska N)
    (70.0, -141.0), // Beaufort, AK/Yukon
    (68.0, -118.0), // Canadian arctic coast
    (67.0, -95.0),
    (64.0, -78.0), // Baffin/Ungava (Hudson Bay filled)
    (58.0, -64.0), // Labrador
    (52.0, -56.0),
    (46.0, -60.0), // Maritimes
    (44.0, -66.0),
    (41.0, -70.0), // New England
    (38.0, -74.0), // Mid-Atlantic
    (34.0, -77.0), // Carolinas
    (30.0, -81.0), // NE Florida
    (25.0, -80.0), // S Florida
    (28.0, -83.0), // W Florida
    (30.0, -88.0), // Gulf coast
    (29.0, -94.0), // Texas
    (25.0, -97.0), // Rio Grande
    (31.0, -114.0), // chord across the SW (N Mexico desert)
    (33.0, -118.0), // S California
    (40.0, -124.0), // US west coast
    (48.0, -124.0), // Pacific NW
    (54.0, -131.0), // BC
    (59.0, -140.0), // SE Alaska
    (60.0, -149.0), // Anchorage
    (56.0, -160.0), // Alaska Peninsula
    (60.0, -165.0), // Bering
    (66.0, -163.0), // Seward Peninsula
];

const MEXICO_CENTRAL: &[(f32, f32)] = &[
    (26.0, -98.0), // N Mexico (overlap N. America)
    (22.0, -98.0), // Veracruz
    (18.0, -94.0),
    (21.0, -90.0), // Yucatán
    (18.0, -88.0), // Belize
    (15.0, -83.0), // Nicaragua Caribbean
    (9.0, -82.0), // Panama
    (8.0, -78.0), // Darién (overlap S. America)
    (13.0, -91.0), // Pacific coast
    (16.0, -96.0),
    (20.0, -106.0), // Puerto Vallarta
    (23.0, -110.0), // Cabo / Baja tip
    (28.0, -113.0), // Baja (Gulf of California filled)
    (26.0, -109.0),
    (24.0, -107.0),
];

const GREENLAND: &[(f32, f32)] = &[
    (60.0, -44.0),
    (66.0, -53.0),
    (72.0, -55.0),
    (78.0, -60.0),
    (81.0, -42.0),
    (82.0, -28.0),
    (76.0, -20.0),
    (70.0, -22.0),
    (64.0, -40.0),
];

const SOUTH_AMERICA: &[(f32, f32)] = &[
    (12.0, -72.0), // Colombia (Caribbean)
    (11.0, -64.0), // Venezuela
    (8.0, -60.0),
    (4.0, -52.0), // Guianas
    (-1.0, -49.0), // Amazon mouth
    (-6.0, -35.0), // NE bulge (Recife)
    (-13.0, -39.0), // Bahia
    (-23.0, -43.0), // Rio
    (-33.0, -53.0), // Uruguay
    (-38.0, -58.0), // Buenos Aires
    (-42.0, -64.0),
    (-50.0, -69.0), // Patagonia
    (-55.0, -69.0), // Tierra del Fuego
    (-52.0, -75.0), // Chile S
    (-43.0, -74.0),
    (-30.0, -71.0), // Chile central
    (-18.0, -70.0), // Arica
    (-13.0, -76.0), // Lima
    (-5.0, -81.0), // Peru N
    (0.0, -80.0), // Ecuador
    (7.0, -77.0), // Panama (overlap)
];

const AFRICA: &[(f32, f32)] = &[
    (36.0, -6.0), // Morocco NW
    (37.0, 10.0), // Tunisia
    (31.0, 20.0), // Libya
    (31.0, 30.0), // Nile delta
    (24.0, 35.0), // Red Sea coast
    (15.0, 40.0), // Eritrea
    (12.0, 43.0), // Djibouti (Bab el Mandeb)
    (10.0, 51.0), // Horn (Somalia tip)
    (2.0, 46.0), // Somalia S
    (-5.0, 39.0), // Tanzania
    (-15.0, 41.0), // Mozambique
    (-26.0, 33.0), // Maputo
    (-34.0, 20.0), // Cape of Good Hope
    (-30.0, 16.0), // Namibia
    (-18.0, 12.0), // Angola
    (-6.0, 12.0), // Congo mouth
    (4.0, 9.0), // Bight of Biafra
    (5.0, -4.0), // Ghana/Côte d'Ivoire
    (10.0, -15.0), // Guinea
    (15.0, -17.0), // Dakar
    (21.0, -17.0), // Mauritania
    (28.0, -12.0), // W Sahara
];

const EUROPE: &[(f32, f32)] = &[
    (36.0, -9.0), // Portugal SW
    (43.0, -9.0), // Galicia
    (47.0, -2.0), // Biscay
    (49.0, -5.0), // Brittany
    (51.0, 2.0), // N France
    (54.0, 8.0), // Denmark
    (58.0, 11.0), // S Sweden (Baltic filled)
    (63.0, 12.0),
    (70.0, 25.0), // North Cape
    (67.0, 33.0), // White Sea
    (63.0, 40.0), // NW Russia (overlap)
    (57.0, 40.0),
    (50.0, 40.0),
    (46.0, 38.0), // Sea of Azov
    (45.0, 34.0), // Crimea
    (43.0, 28.0), // Bulgaria (Black Sea filled)
    (41.0, 29.0), // Istanbul
    (40.0, 26.0), // Aegean NE
    (37.0, 23.0), // S Greece
    (40.0, 20.0), // Albania
    (45.0, 14.0), // Venice
    (42.0, 16.0), // Croatia
    (40.0, 18.0), // Italy heel
    (38.0, 16.0), // toe / Sicily
    (44.0, 9.0), // Genoa
    (43.0, 5.0), // S France
    (41.0, 3.0), // Spain NE
    (39.0, 0.0), // Valencia
    (37.0, -1.0), // Spain S
];

const NORTH_ASIA: &[(f32, f32)] = &[
    (67.0, 33.0), // White Sea (overlap Europe)
    (70.0, 55.0), // Yamal
    (73.0, 70.0),
    (76.0, 90.0), // Taymyr
    (73.0, 113.0),
    (72.0, 130.0),
    (73.0, 142.0),
    (70.0, 160.0),
    (69.0, 170.0), // Chukotka
    (66.0, 180.0),
    (62.0, 179.0),
    (60.0, 165.0), // Kamchatka base
    (56.0, 162.0), // Kamchatka tip
    (60.0, 158.0),
    (59.0, 150.0), // Okhotsk coast
    (53.0, 141.0),
    (50.0, 127.0), // Amur (overlap East Asia)
    (50.0, 105.0), // Mongolia N
    (52.0, 80.0),
    (55.0, 60.0), // Urals
    (62.0, 55.0),
];

const CENTRAL_ASIA: &[(f32, f32)] = &[
    (41.0, 28.0), // Turkey W (overlap Europe)
    (42.0, 41.0), // Caucasus
    (40.0, 50.0), // Caspian S
    (45.0, 60.0), // Kazakhstan (overlap N Asia)
    (50.0, 75.0),
    (48.0, 90.0),
    (45.0, 95.0), // Mongolia W (overlap East Asia)
    (38.0, 75.0),
    (35.0, 70.0), // Afghanistan
    (30.0, 62.0), // overlap India/Arabia
    (30.0, 48.0), // Persian Gulf head (filled)
    (37.0, 38.0), // Syria
    (37.0, 32.0), // Anatolia S (Med coast)
];

const EAST_ASIA: &[(f32, f32)] = &[
    (53.0, 127.0), // Amur (overlap N Asia)
    (48.0, 135.0),
    (43.0, 132.0), // Vladivostok
    (39.0, 128.0), // Korea E
    (35.0, 129.0), // Busan
    (35.0, 126.0), // Korea SW
    (38.0, 124.0), // Yellow Sea (filled)
    (40.0, 121.0), // Bohai
    (37.0, 122.0), // Shandong
    (32.0, 121.0), // Shanghai
    (27.0, 121.0), // Fujian
    (22.0, 114.0), // Hong Kong
    (21.0, 109.0), // overlap SE Asia
    (25.0, 100.0), // Yunnan
    (35.0, 100.0),
    (42.0, 95.0), // overlap Central Asia
    (48.0, 110.0), // Mongolia (overlap)
    (52.0, 120.0),
];

const INDIA: &[(f32, f32)] = &[
    (35.0, 75.0), // Kashmir (overlap)
    (32.0, 71.0), // Pakistan
    (24.0, 67.0), // Indus / Karachi
    (22.0, 69.0), // Kutch
    (20.0, 73.0), // Mumbai
    (15.0, 74.0), // Goa
    (8.0, 77.0), // Kanyakumari (S tip)
    (13.0, 80.0), // Chennai
    (16.0, 82.0),
    (20.0, 87.0), // Odisha
    (22.0, 89.0), // Ganges delta
    (24.0, 92.0),
    (27.0, 88.0), // Himalaya (overlap)
    (30.0, 78.0),
];

const ARABIA: &[(f32, f32)] = &[
    (30.0, 33.0), // Sinai/Suez
    (29.0, 35.0), // Gulf of Aqaba
    (21.0, 39.0), // Jeddah (Red Sea)
    (13.0, 43.0), // Yemen SW
    (13.0, 48.0), // Yemen S
    (17.0, 53.0), // Dhofar
    (22.0, 59.0), // Oman E
    (26.0, 57.0), // Hormuz
    (29.0, 48.0), // Kuwait (Gulf filled)
    (31.0, 38.0), // Jordan
];

const SE_ASIA: &[(f32, f32)] = &[
    (21.0, 109.0), // Vietnam N (overlap China)
    (16.0, 108.0), // Vietnam central
    (9.0, 105.0), // Mekong delta
    (8.0, 100.0), // Gulf of Thailand
    (1.0, 104.0), // Singapore
    (8.0, 98.0), // Andaman coast
    (16.0, 94.0), // Myanmar (Arakan)
    (21.0, 93.0), // overlap India
    (25.0, 97.0),
    (22.0, 101.0), // Laos
];

const AUSTRALIA: &[(f32, f32)] = &[
    (-11.0, 142.0), // Cape York
    (-17.0, 146.0), // Queensland
    (-25.0, 153.0), // Brisbane
    (-32.0, 153.0), // Sydney
    (-38.0, 148.0), // Victoria E
    (-38.0, 141.0),
    (-35.0, 135.0), // South Australia
    (-32.0, 127.0), // Nullarbor
    (-34.0, 120.0),
    (-35.0, 116.0), // SW corner
    (-22.0, 114.0), // Pilbara
    (-16.0, 123.0), // Kimberley
    (-14.0, 127.0),
    (-12.0, 131.0), // Darwin
    (-15.0, 136.0), // Gulf of Carpentaria (filled)
    (-17.0, 140.0),
];

const POLYGONS: &[&[(f32, f32)]] = &[
    NORTH_AMERICA,
    MEXICO_CENTRAL,
    GREENLAND,
    SOUTH_AMERICA,
    AFRICA,
    EUROPE,
    NORTH_ASIA,
    CENTRAL_ASIA,
    EAST_ASIA,
    INDIA,
    ARABIA,
    SE_ASIA,
    AUSTRALIA,
];

/// Smaller islands as lat/lon ellipses `(center_lat, center_lon, semi_lat,
/// semi_lon)` — cheaper than polygons and fine at tile scale.
const ISLANDS: &[(f32, f32, f32, f32)] = &[
    (54.0, -2.0, 4.0, 3.0),   // Great Britain
    (53.0, -8.0, 2.0, 2.0),   // Ireland
    (65.0, -19.0, 2.0, 4.0),  // Iceland
    (37.0, 138.0, 7.0, 3.0),  // Japan (Honshu/Kyushu)
    (43.0, 142.0, 2.0, 2.0),  // Hokkaido
    (24.0, 121.0, 2.0, 1.0),  // Taiwan
    (12.0, 122.0, 6.0, 3.0),  // Philippines
    (0.0, 101.0, 5.0, 4.0),   // Sumatra
    (0.0, 114.0, 4.0, 4.0),   // Borneo
    (-8.0, 111.0, 1.5, 5.0),  // Java
    (-2.0, 121.0, 3.0, 2.0),  // Sulawesi
    (-5.0, 141.0, 4.0, 8.0),  // New Guinea
    (-42.0, 147.0, 2.0, 2.0), // Tasmania
    (-38.0, 176.0, 2.5, 2.0), // New Zealand (North)
    (-44.0, 170.0, 3.0, 2.0), // New Zealand (South)
    (-20.0, 47.0, 7.0, 3.0),  // Madagascar
    (8.0, 81.0, 2.0, 1.0),    // Sri Lanka
    (22.0, -79.0, 1.5, 4.0),  // Cuba
    (19.0, -71.0, 1.0, 2.0),  // Hispaniola
    (50.0, 143.0, 3.0, 1.0),  // Sakhalin
];

/// Ray-casting point-in-polygon for a `(lat, lon)` point. Polygon vertices are
/// `(lat, lon)`; the ray is cast along increasing longitude.
fn point_in_poly(lat: f32, lon: f32, poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (lat_i, lon_i) = poly[i];
        let (lat_j, lon_j) = poly[j];
        if (lat_i > lat) != (lat_j > lat)
            && lon < (lon_j - lon_i) * (lat - lat_i) / (lat_j - lat_i) + lon_i
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Is `(lat, lon)` over land in the stylised model?
pub fn is_land(lat: f32, lon: f32) -> bool {
    if lat < -62.0 {
        return true; // Antarctica
    }
    if POLYGONS.iter().any(|p| point_in_poly(lat, lon, p)) {
        return true;
    }
    ISLANDS.iter().any(|&(clat, clon, alat, alon)| {
        let dl = (lat - clat) / alat;
        let dn = (lon - clon) / alon;
        dl * dl + dn * dn <= 1.0
    })
}

/// Equirectangular projection: map `(lat, lon)` to a pixel inside the box
/// `[mx, mx+mw] × [my, my+mh]`. Lon −180..180 spans the width; lat 90..−90 the
/// height (north at the top).
pub fn project(lat: f32, lon: f32, mx: i32, my: i32, mw: i32, mh: i32) -> (i32, i32) {
    let px = mx + ((lon.clamp(-180.0, 180.0) + 180.0) / 360.0 * mw as f32) as i32;
    let py = my + ((90.0 - lat.clamp(-90.0, 90.0)) / 180.0 * mh as f32) as i32;
    (px, py)
}

/// Draw the map frame, then stipple the land into the box. Coast cells — land
/// that borders ocean — are drawn solid (2×2) to outline the continents, while
/// interior land is a lighter single-pixel stipple, so the landmasses read as
/// shapes with defined coastlines rather than uniform dot fields.
pub fn draw(img: &mut RgbImage, mx: i32, my: i32, mw: i32, mh: i32, color: Color) {
    rect(img, mx, my, mw, mh, color);
    let step = (mh / 48).clamp(3, 7);
    // One grid step expressed in degrees, for the neighbour (coastline) probes.
    let dlat = step as f32 / mh as f32 * 180.0;
    let dlon = step as f32 / mw as f32 * 360.0;
    let mut gy = my + step / 2;
    while gy < my + mh {
        let lat = 90.0 - (gy - my) as f32 / mh as f32 * 180.0;
        let mut gx = mx + step / 2;
        while gx < mx + mw {
            let lon = (gx - mx) as f32 / mw as f32 * 360.0 - 180.0;
            if is_land(lat, lon) {
                // A coast cell has at least one ocean neighbour a step away.
                let coast = !is_land(lat + dlat, lon)
                    || !is_land(lat - dlat, lon)
                    || !is_land(lat, lon + dlon)
                    || !is_land(lat, lon - dlon);
                if coast {
                    fill_rect(img, gx, gy, 2, 2, color);
                } else {
                    fill_rect(img, gx, gy, 1, 1, color);
                }
            }
            gx += step;
        }
        gy += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn land_model_hits_known_points() {
        // Land
        assert!(is_land(40.0, -100.0), "central US is land");
        assert!(is_land(45.0, -75.0), "SE Canada is land");
        assert!(is_land(0.0, 20.0), "central Africa is land");
        assert!(is_land(9.0, 48.0), "Horn of Africa is land");
        assert!(is_land(-80.0, 0.0), "Antarctica is land");
        assert!(is_land(22.0, 78.0), "India is land");
        assert!(is_land(-25.0, 134.0), "central Australia is land");
        assert!(is_land(-10.0, -52.0), "Amazon basin is land");
        assert!(is_land(62.0, 90.0), "Siberia is land");
        assert!(is_land(45.0, 90.0), "Mongolia is land");
        assert!(is_land(24.0, 45.0), "Arabia is land");
        assert!(is_land(36.0, 138.0), "Japan is land");
        assert!(is_land(-19.0, 46.0), "Madagascar is land");
    }

    #[test]
    fn land_model_misses_oceans() {
        assert!(!is_land(0.0, -140.0), "mid-Pacific is ocean");
        assert!(!is_land(40.0, -40.0), "mid-Atlantic is ocean");
        assert!(!is_land(-30.0, 80.0), "central Indian Ocean is ocean");
        assert!(!is_land(-40.0, -120.0), "South Pacific is ocean");
        assert!(!is_land(0.0, -25.0), "equatorial Atlantic is ocean");
        assert!(!is_land(60.0, -30.0), "North Atlantic is ocean");
    }

    #[test]
    fn project_corners() {
        // lon -180/lat 90 -> top-left; lon 180/lat -90 -> bottom-right.
        assert_eq!(project(90.0, -180.0, 0, 0, 360, 180), (0, 0));
        let (px, py) = project(0.0, 0.0, 0, 0, 360, 180);
        assert_eq!((px, py), (180, 90));
    }
}
