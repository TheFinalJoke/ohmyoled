//! Drawing primitives for LED matrix images.
//!
//! All functions operate on an `image::RgbImage` and accept a `Color`.
//! `draw_text` dispatches on font kind (BDF vs TTF).

use super::font::Font;
use crate::Color;
use ab_glyph::{Font as _, ScaleFont};
use image::RgbImage;

/// Draw a single line of text onto `img`.
///
/// `x` is the pen position (left edge); `y` is the baseline. Returns the pen
/// position after the last drawn character so callers can chain.
///
/// For BDF fonts, characters not present in the font are skipped silently.
/// For TTF fonts, coverage alpha is multiplied into `color` (anti-aliased).
pub fn draw_text(img: &mut RgbImage, font: &Font, x: i32, y: i32, color: Color, text: &str) -> i32 {
    if let Some(bdf) = font.bdf() {
        return draw_text_bdf(img, bdf, x, y, color, text);
    }
    if let Some(ttf) = font.ttf() {
        return draw_text_ttf(img, ttf, x, y, color, text);
    }
    x
}

fn draw_text_bdf(
    img: &mut RgbImage,
    bdf: &super::bdf::BdfFont,
    x: i32,
    y: i32,
    color: Color,
    text: &str,
) -> i32 {
    let mut pen_x = x;
    for ch in text.chars() {
        let cp = ch as u32;
        let glyph = match bdf.glyphs.get(&cp) {
            Some(g) => g,
            None => continue,
        };

        let glyph_top = y - bdf.ascent + (bdf.ascent - glyph.height - glyph.offset_y);

        for (row_idx, &bits) in glyph.rows.iter().enumerate() {
            let py = glyph_top + row_idx as i32;
            if py < 0 || py >= img.height() as i32 {
                continue;
            }
            for col in 0..glyph.width {
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

fn draw_text_ttf(
    img: &mut RgbImage,
    ttf: &super::ttf::TtfFont,
    x: i32,
    y: i32,
    color: Color,
    text: &str,
) -> i32 {
    let scaled = ttf.font.as_scaled(ttf.scale);
    let mut pen_x = x as f32;
    let img_w = img.width() as i32;
    let img_h = img.height() as i32;

    for ch in text.chars() {
        let glyph_id = scaled.glyph_id(ch);
        let glyph = glyph_id.with_scale_and_position(ttf.scale, ab_glyph::point(pen_x, y as f32));
        let h_advance = scaled.h_advance(glyph_id);

        if let Some(outlined) = scaled.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                if coverage <= 0.0 {
                    return;
                }
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px < 0 || px >= img_w || py < 0 || py >= img_h {
                    return;
                }
                let r = (color.r as f32 * coverage).round() as u8;
                let g = (color.g as f32 * coverage).round() as u8;
                let b = (color.b as f32 * coverage).round() as u8;
                // Blend over existing pixel: simple "lighten" — keep the brightest channel.
                // Matches the way PIL+freetype renders onto an initially-black RGB image.
                let existing = img.get_pixel(px as u32, py as u32).0;
                let blended = image::Rgb([
                    existing[0].max(r),
                    existing[1].max(g),
                    existing[2].max(b),
                ]);
                img.put_pixel(px as u32, py as u32, blended);
            });
        }

        pen_x += h_advance;
    }
    pen_x.round() as i32
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
