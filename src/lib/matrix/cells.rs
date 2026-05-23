//! Invisible-box cell layout for multi-column rows.
//!
//! Renderers like stock and golf split a row into a left cell and a right
//! cell with a hard barrier between them. Each cell is defined by a
//! `(box_x, box_w)` window. Text rendered into a cell is **clipped strictly
//! to its window** — pixels that would land outside are dropped on the way to
//! the panel, so two cells can never bleed into each other no matter how long
//! their content gets.
//!
//! When a cell's text won't fit, [`draw_pieces_in_box`] marquee-scrolls it.
//! The renderer drives the animation by feeding an incrementing `frame`
//! counter. Use [`overflow_period`] to compute how many frames one full pass
//! takes — multiply by 2 for the "scroll twice" rule.

use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::Color;

/// Gap (px) between marquee repeats. Enough empty space that the eye
/// registers the wrap before the second copy enters the window.
pub const SCROLL_GAP: i32 = 8;

#[derive(Clone, Copy, Debug)]
pub enum Align {
    Left,
    Right,
}

/// A single colored chunk of text within a multi-piece cell unit.
#[derive(Clone, Copy)]
pub struct Piece<'a> {
    pub text: &'a str,
    pub color: Color,
}

/// One marquee period for a cell — `Some(period)` if the unit overflows the
/// box, `None` if it fits. A renderer takes the max `period` across all its
/// overflowing cells and scrolls for `2 * max_period` frames.
pub fn overflow_period(unit_w: i32, box_w: i32) -> Option<i32> {
    if unit_w > box_w {
        Some(unit_w + SCROLL_GAP)
    } else {
        None
    }
}

/// Draw a single-piece cell into a box. Convenience over [`draw_pieces_in_box`]
/// for the common case where one colored string fills the cell.
#[allow(clippy::too_many_arguments)]
pub fn draw_cell(
    img: &mut RgbImage,
    font: &Font,
    box_x: i32,
    box_w: i32,
    baseline: i32,
    align: Align,
    color: Color,
    text: &str,
    frame: u32,
) {
    draw_pieces_in_box(
        img,
        font,
        box_x,
        box_w,
        baseline,
        align,
        &[Piece { text, color }],
        0,
        frame,
    );
}

/// Draw a multi-piece unit (e.g. white "H" label + colored value) into a box.
///
/// Pieces are laid out left-to-right with `piece_gap` between them and treated
/// as one scrollable unit, so label and value never separate. If the unit fits
/// the box, it's rendered once at the box's edge per `align`. If it overflows,
/// the unit and a wrap copy are rendered with the frame-driven offset, clipped
/// to `[box_x, box_x + box_w)`.
#[allow(clippy::too_many_arguments)]
pub fn draw_pieces_in_box(
    img: &mut RgbImage,
    font: &Font,
    box_x: i32,
    box_w: i32,
    baseline: i32,
    align: Align,
    pieces: &[Piece<'_>],
    piece_gap: i32,
    frame: u32,
) {
    if pieces.is_empty() {
        return;
    }
    let widths: Vec<i32> = pieces.iter().map(|p| font.text_width(p.text)).collect();
    let total_w: i32 = widths.iter().sum::<i32>() + piece_gap * (pieces.len() as i32 - 1);
    let box_x_end = box_x + box_w;

    let anchors: Vec<i32> = match overflow_period(total_w, box_w) {
        None => {
            let start_x = match align {
                Align::Left => box_x,
                Align::Right => box_x_end - total_w,
            };
            vec![start_x]
        }
        Some(period) => {
            let offset = (frame as i32) % period;
            let start_x = box_x - offset;
            vec![start_x, start_x + period]
        }
    };

    for anchor in anchors {
        let mut x = anchor;
        for (i, piece) in pieces.iter().enumerate() {
            draw_text_in_window(img, font, x, baseline, piece.color, piece.text, box_x, box_x_end);
            x += widths[i] + piece_gap;
        }
    }
}

/// Draw `text` at pen position `(x, baseline)`, committing only pixels whose
/// column lands in `[win_x_min, win_x_max)`. Renders into a strip exactly the
/// window's width and lighten-composites back, so glyph clipping is handled by
/// the strip's own bounds.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_in_window(
    img: &mut RgbImage,
    font: &Font,
    x: i32,
    baseline: i32,
    color: Color,
    text: &str,
    win_x_min: i32,
    win_x_max: i32,
) {
    let win_w = win_x_max - win_x_min;
    if win_w <= 0 {
        return;
    }
    let mut strip = RgbImage::new(win_w as u32, img.height());
    draw_text(&mut strip, font, x - win_x_min, baseline, color, text);
    for sy in 0..strip.height() {
        for sx in 0..strip.width() {
            let p = strip.get_pixel(sx, sy).0;
            if p == [0, 0, 0] {
                continue;
            }
            let mx = win_x_min + sx as i32;
            if mx < 0 || mx >= img.width() as i32 {
                continue;
            }
            let existing = img.get_pixel(mx as u32, sy).0;
            img.put_pixel(
                mx as u32,
                sy,
                image::Rgb([
                    existing[0].max(p[0]),
                    existing[1].max(p[1]),
                    existing[2].max(p[2]),
                ]),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn font() -> Font {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fonts")
            .join("04B_03B_.TTF");
        Font::load_ttf(&p, 8.0).expect("fonts")
    }

    #[test]
    fn overflow_period_signals_overflow() {
        assert_eq!(overflow_period(20, 30), None);
        assert_eq!(overflow_period(30, 30), None);
        assert_eq!(overflow_period(31, 30), Some(31 + SCROLL_GAP));
    }

    #[test]
    fn fitting_text_stays_inside_box() {
        let f = font();
        let mut img = RgbImage::new(64, 32);
        draw_cell(&mut img, &f, 0, 32, 8, Align::Left, Color::WHITE, "AB", 0);
        // Nothing should be lit beyond the box.
        for y in 0..img.height() {
            for x in 32..img.width() {
                assert_eq!(img.get_pixel(x, y).0, [0, 0, 0], "({x},{y}) lit outside box");
            }
        }
    }

    #[test]
    fn overflowing_text_never_crosses_window() {
        let f = font();
        let long = "ABCDEFGHIJKLMNOP"; // ~64 px, much wider than a 24-px box.
        for frame in [0u32, 5, 13, 27, 40] {
            let mut img = RgbImage::new(64, 32);
            draw_cell(&mut img, &f, 0, 24, 8, Align::Left, Color::WHITE, long, frame);
            for y in 0..img.height() {
                for x in 24..img.width() {
                    assert_eq!(
                        img.get_pixel(x, y).0,
                        [0, 0, 0],
                        "overflowing text bled past x=24 at frame {frame}, pixel ({x},{y})"
                    );
                }
            }
        }
    }
}
