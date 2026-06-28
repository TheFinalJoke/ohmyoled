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
use ohmyoled_matrix::graphics::{draw_circle, draw_line, draw_text, Font};
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

/// Draw a donut gauge: an outlined ring (`inner`..`outer` radius) with a solid
/// arc swept clockwise from 12 o'clock proportional to `frac` (0..1). Good for
/// "X% of a whole" with a value in the hole.
pub fn donut(img: &mut RgbImage, cx: i32, cy: i32, outer: i32, inner: i32, frac: f32, color: Color) {
    let px = Rgb([color.r, color.g, color.b]);
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    let max_deg = frac.clamp(0.0, 1.0) * 360.0;
    let (o2, i2) = (outer * outer, inner * inner);
    for y in -outer..=outer {
        for x in -outer..=outer {
            let d2 = x * x + y * y;
            if d2 < i2 || d2 > o2 {
                continue;
            }
            let mut ang = (x as f32).atan2(-(y as f32)).to_degrees();
            if ang < 0.0 {
                ang += 360.0;
            }
            if ang <= max_deg {
                let (gx, gy) = (cx + x, cy + y);
                if gx >= 0 && gx < iw && gy >= 0 && gy < ih {
                    img.put_pixel(gx as u32, gy as u32, px);
                }
            }
        }
    }
    draw_circle(img, cx, cy, outer, color);
    draw_circle(img, cx, cy, inner, color);
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

/// Truncate `text` to the widest prefix that fits `max_px` in `font`, adding
/// an `…` when it was cut. Returns `text` unchanged when it already fits.
pub fn fit_text(font: &Font, text: &str, max_px: i32) -> String {
    if font.text_width(text) <= max_px {
        return text.to_string();
    }
    // ASCII "..." — the pixel fonts have no glyph for the single-char ellipsis
    // (U+2026), which renders as a .notdef circle.
    let ell = "...";
    let ell_w = font.text_width(ell);
    let mut out = String::new();
    let mut w = 0;
    for ch in text.chars() {
        let cw = font.text_width(&ch.to_string());
        if w + cw + ell_w > max_px {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push_str(ell);
    out
}

/// Total width a [`badge`] occupies for `text` (box + horizontal padding).
/// Use it to center a badge: `badge(img, font, cx - badge_width(font, text)/2, …)`.
pub fn badge_width(font: &Font, text: &str) -> i32 {
    font.text_width(text) + 2 * (font.height() / 3).max(3)
}

/// Draw a sparkline of `series` filling the box `[x, x+w] × [y, y+h]`.
///
/// Autoscales the value range to the box height (larger values draw higher),
/// connects consecutive points with line segments, and marks the newest
/// (rightmost) point with a small filled square — the "you are here" dot. Draws
/// nothing for fewer than two finite points (the caller shows a NO-DATA badge);
/// a flat series draws a centered horizontal line.
pub fn sparkline(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, series: &[f32], color: Color) {
    let pts: Vec<f32> = series.iter().copied().filter(|v| v.is_finite()).collect();
    if pts.len() < 2 || w < 2 || h < 1 {
        return;
    }
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &v in &pts {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    let span = hi - lo;
    // Map index i and value v to a screen point inside the box.
    let n = pts.len() as i32;
    let sx = |i: i32| x + (w - 1) * i / (n - 1);
    let sy = |v: f32| {
        if span <= f32::EPSILON {
            y + h / 2
        } else {
            // Invert: larger value → smaller y (higher on screen).
            y + h - 1 - ((v - lo) / span * (h - 1) as f32).round() as i32
        }
    };
    for i in 1..n {
        draw_line(img, sx(i - 1), sy(pts[(i - 1) as usize]), sx(i), sy(pts[i as usize]), color);
    }
    // Newest-point marker.
    let mx = sx(n - 1);
    let my = sy(pts[(n - 1) as usize]);
    fill_rect(img, mx - 1, my - 1, 3, 3, color);
}

/// Like [`sparkline`], but each point carries its own normalized x
/// position `fracs[i]` in `0.0..=1.0` instead of being evenly spaced.
/// Used by the intraday (1D) stock row so a half-finished trading day
/// fills only the elapsed portion of the plot width — the x-axis is
/// time of day, not sample index. `series` and `fracs` must be the same
/// length; values autoscale to the box height exactly as in `sparkline`.
#[allow(clippy::too_many_arguments)]
pub fn sparkline_timed(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, series: &[f32], fracs: &[f32], color: Color) {
    if series.len() != fracs.len() || series.len() < 2 || w < 2 || h < 1 {
        return;
    }
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &v in series {
        if v.is_finite() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return;
    }
    let span = hi - lo;
    let sx = |f: f32| x + ((w - 1) as f32 * f.clamp(0.0, 1.0)).round() as i32;
    let sy = |v: f32| {
        if span <= f32::EPSILON {
            y + h / 2
        } else {
            y + h - 1 - ((v - lo) / span * (h - 1) as f32).round() as i32
        }
    };
    for i in 1..series.len() {
        draw_line(img, sx(fracs[i - 1]), sy(series[i - 1]), sx(fracs[i]), sy(series[i]), color);
    }
    // Newest-point marker rides the last sample's time position.
    let last = series.len() - 1;
    fill_rect(img, sx(fracs[last]) - 1, sy(series[last]) - 1, 3, 3, color);
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

    /// `fit_text` truncates long strings with an ASCII `...`, never the
    /// single-char ellipsis (U+2026) — the pixel font has no glyph for it and
    /// would draw a .notdef circle. Regression test for the F1 circuit label.
    #[test]
    fn fit_text_uses_ascii_ellipsis() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/04B_03B_.TTF");
        let font = Font::load_ttf(&path, 22.0).expect("repo font loads");
        let long = "Circuit de Spa-Francorchamps";
        let fitted = fit_text(&font, long, 80);
        assert!(fitted.len() < long.len(), "long name should be truncated");
        assert!(fitted.ends_with("..."), "got: {fitted:?}");
        assert!(!fitted.contains('\u{2026}'), "must not use the U+2026 ellipsis");
        // Short text passes through untouched.
        assert_eq!(fit_text(&font, "F1", 200), "F1");
    }

    #[test]
    fn scaled_px_tracks_height() {
        assert_eq!(scaled_px(48.0, 480), 48.0);
        assert_eq!(scaled_px(48.0, 240), 24.0);
        // Floor prevents collapse on tiny panels.
        assert_eq!(scaled_px(4.0, 60), 6.0);
    }

    #[test]
    fn sparkline_rises_left_to_right() {
        // A monotonically increasing series should light more pixels in the
        // upper rows on the right half than the left half.
        let mut img = RgbImage::new(40, 20);
        let series: Vec<f32> = (0..20).map(|i| i as f32).collect();
        sparkline(&mut img, 0, 0, 40, 20, &series, Color::WHITE);
        let lit_in = |x0: u32, x1: u32, y0: u32, y1: u32| {
            let mut c = 0;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    if img.get_pixel(xx, yy).0 != [0, 0, 0] {
                        c += 1;
                    }
                }
            }
            c
        };
        // Top-right quadrant lit, bottom-right less so (line ends high on the right).
        assert!(lit_in(20, 40, 0, 10) > 0, "rising series should reach the top-right");
        // Empty / single-point series draws nothing.
        let mut blank = RgbImage::new(40, 20);
        sparkline(&mut blank, 0, 0, 40, 20, &[1.0], Color::WHITE);
        assert!(blank.pixels().all(|p| p.0 == [0, 0, 0]), "single point draws nothing");
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
