//! Stylised F1 circuit outlines for the e-paper F1 tile.
//!
//! We don't have survey-accurate track geometry, so these are simplified,
//! recognisable-in-spirit closed loops in a normalised unit square (x right, y
//! down). Suzuka's signature figure-8 crossover is special-cased; every other
//! circuit gets one of a few generic winding-track templates picked
//! deterministically from its name, so different races look different without
//! claiming exact layouts. (Track *shapes* are geographic facts, not artwork.)

use ohmyoled_matrix::graphics::draw_line;
use ohmyoled_matrix::Color;
use image::RgbImage;

/// Suzuka — the famous figure-8 with the crossover bridge.
const SUZUKA: &[(f32, f32)] = &[
    (0.50, 0.06), (0.70, 0.10), (0.84, 0.26), (0.74, 0.42), (0.56, 0.47),
    (0.44, 0.53), (0.26, 0.58), (0.16, 0.74), (0.32, 0.90), (0.52, 0.92),
    (0.70, 0.84), (0.74, 0.64), (0.58, 0.53), (0.44, 0.47), (0.28, 0.42),
    (0.18, 0.28), (0.30, 0.12),
];

/// Generic winding-track templates (chosen by name hash for everything else).
const TEMPLATES: &[&[(f32, f32)]] = &[
    &[
        (0.12, 0.42), (0.10, 0.20), (0.28, 0.10), (0.50, 0.16), (0.58, 0.34),
        (0.76, 0.16), (0.90, 0.34), (0.84, 0.56), (0.64, 0.58), (0.72, 0.80),
        (0.50, 0.90), (0.30, 0.86), (0.24, 0.62), (0.12, 0.58),
    ],
    &[
        (0.20, 0.30), (0.45, 0.12), (0.70, 0.14), (0.88, 0.34), (0.80, 0.50),
        (0.60, 0.50), (0.72, 0.66), (0.82, 0.84), (0.54, 0.90), (0.28, 0.86),
        (0.12, 0.62), (0.18, 0.42),
    ],
    &[
        (0.44, 0.07), (0.58, 0.10), (0.62, 0.44), (0.80, 0.60), (0.70, 0.70),
        (0.54, 0.60), (0.50, 0.80), (0.62, 0.92), (0.42, 0.95), (0.28, 0.82),
        (0.34, 0.48), (0.22, 0.32), (0.36, 0.20),
    ],
    &[
        (0.16, 0.50), (0.20, 0.24), (0.42, 0.16), (0.50, 0.34), (0.66, 0.18),
        (0.86, 0.28), (0.82, 0.50), (0.66, 0.56), (0.78, 0.74), (0.58, 0.86),
        (0.40, 0.78), (0.36, 0.58), (0.20, 0.66),
    ],
];

/// Pick a circuit outline for a circuit/race name.
pub fn path_for(name: &str) -> &'static [(f32, f32)] {
    let key = name.to_lowercase();
    if key.contains("suzuka") || key.contains("japan") {
        return SUZUKA;
    }
    let hash = key.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    TEMPLATES[hash as usize % TEMPLATES.len()]
}

/// Draw a circuit `path` (closed loop) aspect-fit and centered in the box
/// `[x, x+w] × [y, y+h]`, with a start/finish tick at the first point.
pub fn draw(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, path: &[(f32, f32)], color: Color) {
    if path.len() < 2 {
        return;
    }
    let s = w.min(h);
    let ox = x + (w - s) / 2;
    let oy = y + (h - s) / 2;
    let pt = |p: (f32, f32)| (ox + (p.0 * s as f32) as i32, oy + (p.1 * s as f32) as i32);
    for i in 0..path.len() {
        let a = pt(path[i]);
        let b = pt(path[(i + 1) % path.len()]);
        draw_line(img, a.0, a.1, b.0, b.1, color);
        // Second pass offset by 1px so the track reads as a thicker line.
        draw_line(img, a.0, a.1 + 1, b.0, b.1 + 1, color);
    }
    // Start/finish tick: a short bar across the track at the first point.
    let (sx, sy) = pt(path[0]);
    let tick = (s / 14).max(3);
    draw_line(img, sx - tick, sy - tick, sx + tick, sy + tick, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suzuka_special_cased_others_templated() {
        assert!(std::ptr::eq(path_for("Suzuka Circuit"), SUZUKA));
        assert!(std::ptr::eq(path_for("Japanese Grand Prix"), SUZUKA));
        assert!(!std::ptr::eq(path_for("Circuit de Monaco"), SUZUKA));
        // Deterministic: same name -> same template.
        assert!(std::ptr::eq(path_for("Monza"), path_for("Monza")));
    }

    #[test]
    fn draw_lights_pixels() {
        let mut img = RgbImage::new(200, 200);
        draw(&mut img, 10, 10, 180, 180, path_for("Bahrain"), Color::WHITE);
        assert!(img.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 50);
    }
}
