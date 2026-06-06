//! Shared layout toolkit for e-paper renderers.
//!
//! Two things every e-ink tile needs:
//!
//! 1. **Resolution-adaptive font sizing** — [`scaled_px`] scales a base size
//!    (authored for an 800×480 / 7.5" panel) to the actual panel height, so a
//!    tile reads the same on a 400×300 4.2" or an 800×480 7.5" sheet.
//! 2. **Reusable widgets** — headers, stat rows, bars, badges, big values.
//!    The e-ink renderers are pure static layout (no scroll loops), so unlike
//!    the LED side a shared toolkit is the right call: it keeps each tile short
//!    and the visual language consistent.
//!
//! Everything composes white-foreground on black (the `draw_*` convention);
//! [`ohmyoled_matrix::EinkDisplay`] inverts to black-ink-on-white.

use image::{Rgb, RgbImage};
use ohmyoled_matrix::graphics::{draw_line, draw_text, Font};
use ohmyoled_matrix::Color;

/// Reference panel height the base font sizes are authored against (7.5").
pub const REF_HEIGHT: f32 = 480.0;

/// Scale a base font size (authored for an 800×480 panel) to `height`, with a
/// small floor so text never collapses on tiny panels.
pub fn scaled_px(base_px: f32, height: u32) -> f32 {
    (base_px * height as f32 / REF_HEIGHT).max(6.0)
}

/// A margin proportional to the panel, with a sensible floor.
pub fn margin(width: u32) -> i32 {
    ((width as f32 * 0.03) as i32).max(6)
}

/// Draw `text` centered horizontally on `cx`, at baseline `y`.
pub fn center_text(img: &mut RgbImage, font: &Font, cx: i32, y: i32, color: Color, text: &str) {
    let tw = font.text_width(text);
    draw_text(img, font, cx - tw / 2, y, color, text);
}

/// Draw `text` so it ends at `right_x`, at baseline `y`.
pub fn right_text(img: &mut RgbImage, font: &Font, right_x: i32, y: i32, color: Color, text: &str) {
    let tw = font.text_width(text);
    draw_text(img, font, right_x - tw, y, color, text);
}

/// Draw a header band: `title` on the left, optional `right` label on the
/// right, and a rule beneath. Returns the y at which content below can start.
pub fn header_band(
    img: &mut RgbImage,
    title_font: &Font,
    right_font: &Font,
    top: i32,
    title: &str,
    right: Option<&str>,
    color: Color,
) -> i32 {
    let m = margin(img.width());
    let wi = img.width() as i32;
    let base = top + title_font.ascent();
    draw_text(img, title_font, m, base, color, title);
    if let Some(r) = right {
        right_text(img, right_font, wi - m, top + right_font.ascent(), color, r);
    }
    let rule_y = base + title_font.height() / 4 + 2;
    draw_line(img, m, rule_y, wi - m, rule_y, color);
    rule_y + (title_font.height() / 3).max(4)
}

/// Draw a dim metadata line centered along the bottom of the panel.
pub fn footer(img: &mut RgbImage, font: &Font, color: Color, text: &str) {
    let hi = img.height() as i32;
    let y = hi - font.height() / 2;
    center_text(img, font, img.width() as i32 / 2, y, color, text);
}

/// Lay `cells` out as evenly spaced left-aligned columns between the side
/// margins, at baseline `y`.
pub fn stat_row(img: &mut RgbImage, font: &Font, y: i32, color: Color, cells: &[String]) {
    if cells.is_empty() {
        return;
    }
    let m = margin(img.width());
    let span = img.width() as i32 - 2 * m;
    let step = span / cells.len() as i32;
    for (i, cell) in cells.iter().enumerate() {
        draw_text(img, font, m + step * i as i32, y, color, cell);
    }
}

/// Draw a big value with a small trailing unit, left edge at `x`, baseline `y`.
/// Returns the x just past the unit.
#[allow(clippy::too_many_arguments)]
pub fn big_value(
    img: &mut RgbImage,
    big: &Font,
    unit_font: &Font,
    x: i32,
    y: i32,
    color: Color,
    value: &str,
    unit: &str,
) -> i32 {
    let end = draw_text(img, big, x, y, color, value);
    if unit.is_empty() {
        return end;
    }
    let ux = end + (big.height() / 12).max(2);
    draw_text(img, unit_font, ux, y, color, unit);
    ux + unit_font.text_width(unit)
}

/// [`big_value`] horizontally centered on `cx` (value + unit treated as one
/// group). The workhorse for the single-value "hero" tiles.
#[allow(clippy::too_many_arguments)]
pub fn big_value_centered(
    img: &mut RgbImage,
    big: &Font,
    unit_font: &Font,
    cx: i32,
    y: i32,
    color: Color,
    value: &str,
    unit: &str,
) {
    let vw = big.text_width(value);
    let total = if unit.is_empty() {
        vw
    } else {
        vw + (big.height() / 12).max(2) + unit_font.text_width(unit)
    };
    big_value(img, big, unit_font, cx - total / 2, y, color, value, unit);
}

/// Fill a solid rectangle.
pub fn fill_rect(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, color: Color) {
    let px = Rgb([color.r, color.g, color.b]);
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    for yy in y.max(0)..(y + h).min(ih) {
        for xx in x.max(0)..(x + w).min(iw) {
            img.put_pixel(xx as u32, yy as u32, px);
        }
    }
}

/// Draw a rectangle outline.
pub fn rect(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, color: Color) {
    draw_line(img, x, y, x + w, y, color);
    draw_line(img, x, y + h, x + w, y + h, color);
    draw_line(img, x, y, x, y + h, color);
    draw_line(img, x + w, y, x + w, y + h, color);
}

/// Horizontal bar gauge: an outlined track filled to `frac` (0..1), with
/// `ticks` evenly spaced division marks (0 for none).
#[allow(clippy::too_many_arguments)]
pub fn hbar(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, frac: f32, ticks: u32, color: Color) {
    rect(img, x, y, w, h, color);
    let fill = (w as f32 * frac.clamp(0.0, 1.0)).round() as i32;
    if fill > 1 {
        fill_rect(img, x + 1, y + 1, (fill - 1).min(w - 1), h - 1, color);
    }
    for i in 1..ticks {
        let tx = x + (w * i as i32) / ticks as i32;
        draw_line(img, tx, y, tx, y + h, color);
    }
}

/// Total width a [`badge`] occupies for `text` (box + horizontal padding).
/// Use it to center a badge: `badge(img, font, cx - badge_width(font, text)/2, …)`.
pub fn badge_width(font: &Font, text: &str) -> i32 {
    font.text_width(text) + 2 * (font.height() / 3).max(3)
}

/// Draw a badge — a small boxed label. `filled` inverts it (solid box with the
/// text knocked out), which reads as a high-contrast alert on the panel.
/// Returns the x just past the badge.
pub fn badge(img: &mut RgbImage, font: &Font, x: i32, y: i32, text: &str, color: Color, filled: bool) -> i32 {
    let pad_x = (font.height() / 3).max(3);
    let pad_y = (font.height() / 6).max(2);
    let bw = font.text_width(text) + 2 * pad_x;
    let bh = font.height() + 2 * pad_y;
    let baseline = y + pad_y + font.ascent();
    if filled {
        fill_rect(img, x, y, bw, bh, color);
        knockout_text(img, font, x + pad_x, baseline, text);
    } else {
        rect(img, x, y, bw, bh, color);
        draw_text(img, font, x + pad_x, baseline, color, text);
    }
    x + bw
}

/// Render `text` and *clear* (set to black) those pixels in `img` — used to
/// knock text out of a filled badge so it shows as the sheet colour.
fn knockout_text(img: &mut RgbImage, font: &Font, x: i32, baseline: i32, text: &str) {
    let w = font.text_width(text).max(1) as u32;
    let h = (font.height().max(1)) as u32 + 2;
    let mut mask = RgbImage::new(w + 2, h);
    // Draw the glyphs white onto the mask at a local baseline.
    draw_text(&mut mask, font, 0, font.ascent(), Color::WHITE, text);
    let top = baseline - font.ascent();
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    for my in 0..mask.height() as i32 {
        for mx in 0..mask.width() as i32 {
            if mask.get_pixel(mx as u32, my as u32).0 != [0, 0, 0] {
                let (px, py) = (x + mx, top + my);
                if px >= 0 && px < iw && py >= 0 && py < ih {
                    img.put_pixel(px as u32, py as u32, Rgb([0, 0, 0]));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_px_tracks_height() {
        assert_eq!(scaled_px(48.0, 480), 48.0);
        assert_eq!(scaled_px(48.0, 240), 24.0);
        // Floor prevents collapse on tiny panels.
        assert_eq!(scaled_px(4.0, 60), 6.0);
    }

    #[test]
    fn hbar_fills_proportionally() {
        let mut img = RgbImage::new(100, 20);
        hbar(&mut img, 0, 0, 100, 10, 0.5, 0, Color::WHITE);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        let mut full = RgbImage::new(100, 20);
        hbar(&mut full, 0, 0, 100, 10, 1.0, 0, Color::WHITE);
        let lit_full = full.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit < lit_full, "half bar should light fewer px than full");
    }
}
