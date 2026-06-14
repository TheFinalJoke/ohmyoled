//! A tiny, asset-free world map for e-paper tiles.
//!
//! Continents are modeled as a union of lat/lon ellipses (plus an Antarctic
//! band) — no map file or network needed; it renders as a recognisable,
//! stylised equirectangular world. Tiles stipple the land into a box and plot
//! points (the ISS, an earthquake epicenter, …) with [`project`].

use crate::matrix::eink::layout::{fill_rect, rect};
use image::RgbImage;
use ohmyoled_matrix::Color;

/// Continents as a union of lat/lon ellipses `(center_lat, center_lon,
/// semi_lat, semi_lon)`. Stylised, not survey-grade — but a denser set than a
/// few blobs, sculpted to read as recognisable continents (tapers, peninsulas,
/// big islands). Antarctica is handled separately as a southern band.
const LAND: &[(f32, f32, f32, f32)] = &[
    // ── North America ──────────────────────────────────────────────
    (64.0, -150.0, 6.0, 12.0),  // Alaska
    (60.0, -115.0, 12.0, 26.0), // Canada west/central
    (55.0, -75.0, 11.0, 17.0),  // Canada east / Quebec
    (73.0, -95.0, 7.0, 32.0),   // Canadian Arctic islands
    (42.0, -115.0, 8.0, 12.0),  // US west
    (40.0, -90.0, 9.0, 17.0),   // US central/east
    (30.0, -83.0, 6.0, 6.0),    // US southeast / Florida
    (24.0, -103.0, 9.0, 8.0),   // Mexico
    (18.0, -95.0, 5.0, 6.0),    // Mexico south taper
    (12.0, -84.0, 5.0, 6.0),    // Central America
    // Greenland
    (72.0, -42.0, 8.0, 12.0),
    (78.0, -35.0, 5.0, 12.0),
    // ── South America ──────────────────────────────────────────────
    (6.0, -68.0, 8.0, 10.0),    // Colombia / Venezuela
    (-7.0, -50.0, 10.0, 13.0),  // Brazil (eastern bulge)
    (-16.0, -60.0, 9.0, 11.0),  // central
    (-30.0, -63.0, 9.0, 7.0),   // south taper
    (-44.0, -69.0, 8.0, 4.0),   // Patagonia
    // ── Europe ─────────────────────────────────────────────────────
    (47.0, 6.0, 8.0, 12.0),     // western Europe
    (40.0, -4.0, 4.0, 6.0),     // Iberia
    (63.0, 15.0, 9.0, 9.0),     // Scandinavia
    (55.0, 38.0, 11.0, 22.0),   // European Russia
    (54.0, -3.0, 4.0, 3.0),     // British Isles
    (43.0, 13.0, 5.0, 3.0),     // Italy
    // ── Africa ─────────────────────────────────────────────────────
    (24.0, 14.0, 9.0, 22.0),    // Sahara / north
    (10.0, -3.0, 8.0, 12.0),    // west bulge
    (2.0, 22.0, 9.0, 14.0),     // central / Congo
    (8.0, 42.0, 7.0, 7.0),      // Horn of Africa
    (-20.0, 27.0, 13.0, 12.0),  // southern
    (-32.0, 22.0, 4.0, 7.0),    // Cape
    // ── Asia ───────────────────────────────────────────────────────
    (62.0, 75.0, 12.0, 35.0),   // West Siberia
    (66.0, 135.0, 11.0, 40.0),  // East Siberia
    (45.0, 70.0, 10.0, 25.0),   // Central Asia
    (33.0, 108.0, 11.0, 18.0),  // China
    (46.0, 125.0, 6.0, 10.0),   // Manchuria
    (22.0, 78.0, 9.0, 9.0),     // India
    (11.0, 78.0, 5.0, 5.0),     // India south taper
    (24.0, 45.0, 9.0, 11.0),    // Arabia
    (33.0, 43.0, 6.0, 9.0),     // Middle East
    (16.0, 102.0, 7.0, 8.0),    // SE Asia mainland
    // ── Islands ────────────────────────────────────────────────────
    (0.0, 112.0, 6.0, 20.0),    // Indonesia / Malay archipelago
    (-5.0, 142.0, 4.0, 8.0),    // New Guinea
    (37.0, 138.0, 7.0, 3.0),    // Japan
    (12.0, 122.0, 6.0, 3.0),    // Philippines
    (-20.0, 47.0, 7.0, 3.0),    // Madagascar
    (20.0, -77.0, 2.0, 7.0),    // Caribbean (Cuba/Hispaniola)
    (65.0, -19.0, 2.0, 4.0),    // Iceland
    // ── Australia & NZ ─────────────────────────────────────────────
    (-25.0, 134.0, 11.0, 19.0), // mainland
    (-16.0, 140.0, 4.0, 6.0),   // Cape York / north
    (-42.0, 172.0, 6.0, 3.0),   // New Zealand
];

/// Is `(lat, lon)` over land in the stylised model?
pub fn is_land(lat: f32, lon: f32) -> bool {
    if lat < -62.0 {
        return true; // Antarctica
    }
    LAND.iter().any(|&(clat, clon, alat, alon)| {
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
        assert!(is_land(0.0, 20.0), "central Africa is land");
        assert!(is_land(-80.0, 0.0), "Antarctica is land");
        assert!(is_land(22.0, 78.0), "India is land");
        assert!(is_land(-25.0, 134.0), "central Australia is land");
        assert!(is_land(-10.0, -52.0), "Amazon basin is land");
        assert!(is_land(62.0, 90.0), "Siberia is land");
        // Ocean
        assert!(!is_land(0.0, -140.0), "mid-Pacific is ocean");
        assert!(!is_land(40.0, -40.0), "mid-Atlantic is ocean");
        assert!(!is_land(-30.0, 80.0), "central Indian Ocean is ocean");
        assert!(!is_land(-40.0, -120.0), "South Pacific is ocean");
    }

    #[test]
    fn project_corners() {
        // lon -180/lat 90 -> top-left; lon 180/lat -90 -> bottom-right.
        assert_eq!(project(90.0, -180.0, 0, 0, 360, 180), (0, 0));
        let (px, py) = project(0.0, 0.0, 0, 0, 360, 180);
        assert_eq!((px, py), (180, 90));
    }
}
