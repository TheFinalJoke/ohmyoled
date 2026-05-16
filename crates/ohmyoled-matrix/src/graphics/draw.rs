//! Drawing primitives for LED matrix images.
//!
//! All functions operate on an `image::RgbImage` and accept a `Color`.
//! `draw_text` renders a BDF font glyph-by-glyph starting at the given pen position.

use super::font::Font;
use crate::Color;
use image::RgbImage;

/// Draw a single line of text with a BDF font onto `img`.
///
/// `x` and `y` are the pen position (left edge, baseline). Returns the x
/// coordinate just past the last drawn character so callers can chain calls.
///
/// Characters not present in the font are skipped silently.
pub fn draw_text(img: &mut RgbImage, font: &Font, x: i32, y: i32, color: Color, text: &str) -> i32 {
    let bdf = match font.bdf() {
        Some(b) => b,
        None => return x,
    };

    let mut pen_x = x;
    for ch in text.chars() {
        let cp = ch as u32;
        let glyph = match bdf.glyphs.get(&cp) {
            Some(g) => g,
            None => continue,
        };

        // Glyph bitmap origin is (pen_x + offset_x, baseline - ascent + offset_y)
        // BDF offset_y is measured upward from baseline; we need to flip to image coords.
        let glyph_top = y - bdf.ascent + (bdf.ascent - glyph.height - glyph.offset_y);

        for (row_idx, &bits) in glyph.rows.iter().enumerate() {
            let py = glyph_top + row_idx as i32;
            if py < 0 || py >= img.height() as i32 {
                continue;
            }
            for col in 0..glyph.width {
                // Bits are MSB-first; bit 31 is the leftmost pixel.
                let bit_pos = 31 - col as u32;
                if (bits >> bit_pos) & 1 == 1 {
                    let px = pen_x + glyph.offset_x + col;
                    if px >= 0 && px < img.width() as i32 {
                        img.put_pixel(px as u32, py as u32, image::Rgb([color.r, color.g, color.b]));
                    }
                }
            }
        }

        pen_x += glyph.dwidth;
    }
    pen_x
}

/// Draw a straight line between two points (Bresenham).
pub fn draw_line(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    let w = img.width() as i32;
    let h = img.height() as i32;

    loop {
        if x >= 0 && x < w && y >= 0 && y < h {
            img.put_pixel(x as u32, y as u32, image::Rgb([color.r, color.g, color.b]));
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Draw the outline of a circle (midpoint algorithm).
pub fn draw_circle(img: &mut RgbImage, cx: i32, cy: i32, r: i32, color: Color) {
    let mut x = r;
    let mut y = 0i32;
    let mut p = 1 - r;
    let w = img.width() as i32;
    let h = img.height() as i32;
    let pixel = image::Rgb([color.r, color.g, color.b]);

    let mut plot = |px: i32, py: i32| {
        if px >= 0 && px < w && py >= 0 && py < h {
            img.put_pixel(px as u32, py as u32, pixel);
        }
    };

    while x >= y {
        plot(cx + x, cy + y);
        plot(cx - x, cy + y);
        plot(cx + x, cy - y);
        plot(cx - x, cy - y);
        plot(cx + y, cy + x);
        plot(cx - y, cy + x);
        plot(cx + y, cy - x);
        plot(cx - y, cy - x);
        y += 1;
        if p <= 0 {
            p += 2 * y + 1;
        } else {
            x -= 1;
            p += 2 * (y - x) + 1;
        }
    }
}
