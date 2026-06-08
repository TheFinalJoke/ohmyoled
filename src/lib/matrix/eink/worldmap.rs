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
/// semi_lat, semi_lon)`. Stylised, not survey-grade — enough to read as the
/// world. Antarctica is handled separately as a southern band.
const LAND: &[(f32, f32, f32, f32)] = &[
    // North America
    (50.0, -100.0, 18.0, 30.0),
    (62.0, -105.0, 12.0, 33.0),
    (64.0, -78.0, 10.0, 22.0),
    (24.0, -103.0, 10.0, 9.0),
    (12.0, -85.0, 7.0, 8.0),
    // Greenland
    (72.0, -42.0, 8.0, 14.0),
    // South America
    (-6.0, -58.0, 13.0, 14.0),
    (-30.0, -65.0, 18.0, 9.0),
    // Europe
    (50.0, 16.0, 11.0, 22.0),
    (60.0, 28.0, 10.0, 27.0),
    (54.0, -2.0, 5.0, 4.0),
    // Africa
    (17.0, 13.0, 16.0, 19.0),
    (2.0, 22.0, 11.0, 14.0),
    (-18.0, 25.0, 14.0, 12.0),
    // Asia
    (58.0, 95.0, 18.0, 58.0),
    (40.0, 80.0, 13.0, 38.0),
    (28.0, 80.0, 9.0, 12.0),
    (15.0, 78.0, 8.0, 8.0),
    (30.0, 112.0, 12.0, 20.0),
    (25.0, 45.0, 11.0, 13.0),
    (38.0, 140.0, 8.0, 4.0),
    // SE Asia / Indonesia
    (4.0, 110.0, 9.0, 20.0),
    // Australia
    (-25.0, 134.0, 12.0, 21.0),
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

/// Draw the map frame and stipple the land into the box.
pub fn draw(img: &mut RgbImage, mx: i32, my: i32, mw: i32, mh: i32, color: Color) {
    rect(img, mx, my, mw, mh, color);
    let step = (mh / 44).clamp(4, 8);
    let mut gy = my + step / 2;
    while gy < my + mh {
        let lat = 90.0 - (gy - my) as f32 / mh as f32 * 180.0;
        let mut gx = mx + step / 2;
        while gx < mx + mw {
            let lon = (gx - mx) as f32 / mw as f32 * 360.0 - 180.0;
            if is_land(lat, lon) {
                fill_rect(img, gx, gy, 2, 2, color);
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
        assert!(is_land(40.0, -100.0), "central US is land");
        assert!(is_land(0.0, 20.0), "central Africa is land");
        assert!(is_land(-80.0, 0.0), "Antarctica is land");
        assert!(!is_land(0.0, -140.0), "mid-Pacific is ocean");
        assert!(!is_land(40.0, -40.0), "mid-Atlantic is ocean");
    }

    #[test]
    fn project_corners() {
        // lon -180/lat 90 -> top-left; lon 180/lat -90 -> bottom-right.
        assert_eq!(project(90.0, -180.0, 0, 0, 360, 180), (0, 0));
        let (px, py) = project(0.0, 0.0, 0, 0, 360, 180);
        assert_eq!((px, py), (180, 90));
    }
}
