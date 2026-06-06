//! Terminal backend for [`super::EinkDisplay`] — renders the packed 1-bpp
//! frame to stdout using Unicode half-block characters.
//!
//! This is the always-available dev backend: it lets you iterate on e-paper
//! layouts with no hardware, the e-ink analog of [`crate::terminal`]. Each
//! printed character row encodes two vertically-stacked panel pixels via the
//! upper/lower half-block glyphs, so the aspect ratio reads roughly true.
//!
//! E-paper panels are large (a 4.2" is 400×300), so the frame is **downscaled
//! to fit the terminal** before rendering — otherwise a 400-wide frame
//! overflows and wraps into noise. The fit width defaults to 80 columns and is
//! overridable with `OHMYOLED_EINK_COLS` (or `COLUMNS`). Downsampling is
//! ink-preserving: an output cell is inked if *any* source pixel under it is,
//! so thin strokes survive the shrink.
//!
//! Ink pixels (the drawn foreground) print as lit half-blocks; the white sheet
//! prints blank — so the terminal shows exactly what would be inked onto the
//! panel. A blank (all-white) panel prints empty.
//!
//! Set `OHMYOLED_EINK_PNG=/path/to/out.png` to additionally dump each frame
//! as a pixel-exact, full-resolution PNG.

use super::EinkBackend;
use std::io::Write;

/// Fallback terminal width when neither `OHMYOLED_EINK_COLS` nor `COLUMNS` is
/// set — fits a standard 80-column terminal.
const DEFAULT_COLS: u32 = 80;

/// Terminal (dev) backend for e-paper.
#[derive(Default)]
pub struct EinkTerminalBackend {
    png_path: Option<String>,
}

impl EinkTerminalBackend {
    pub fn new() -> Self {
        Self {
            png_path: std::env::var("OHMYOLED_EINK_PNG").ok().filter(|s| !s.is_empty()),
        }
    }

    /// Is the pixel at (x, y) inked (bit clear)? `packed` is MSB-first, stride
    /// bytes/row; bit `1` = white sheet, bit `0` = black ink.
    fn is_ink(packed: &[u8], stride: usize, x: u32, y: u32) -> bool {
        let byte = y as usize * stride + (x / 8) as usize;
        let mask = 0x80u8 >> (x % 8);
        // Out-of-range bytes read as white (no ink).
        packed.get(byte).map(|b| b & mask == 0).unwrap_or(false)
    }

    /// Any inked pixel in the source rectangle `[x0,x1) × [y0,y1)`? Used by the
    /// downscaler so thin strokes don't vanish.
    #[allow(clippy::too_many_arguments)]
    fn block_has_ink(
        packed: &[u8],
        stride: usize,
        x0: u32,
        x1: u32,
        y0: u32,
        y1: u32,
    ) -> bool {
        for y in y0..y1 {
            for x in x0..x1 {
                if Self::is_ink(packed, stride, x, y) {
                    return true;
                }
            }
        }
        false
    }
}

/// Target terminal width: `OHMYOLED_EINK_COLS`, else `COLUMNS`, else 80;
/// never wider than the panel itself.
fn target_cols(panel_width: u32) -> u32 {
    let cap = std::env::var("OHMYOLED_EINK_COLS")
        .ok()
        .or_else(|| std::env::var("COLUMNS").ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&c| c >= 16)
        .unwrap_or(DEFAULT_COLS);
    panel_width.min(cap)
}

/// Downscale the packed frame to an `out_w × out_h` boolean ink grid,
/// preserving aspect ratio with square source blocks (`scale` px per cell).
fn downsample(packed: &[u8], stride: usize, width: u32, height: u32) -> (Vec<bool>, u32, u32) {
    let out_w = target_cols(width).max(1);
    let scale = width as f32 / out_w as f32; // source px per output cell
    let out_h = ((height as f32 / scale).round() as u32).max(1);

    let mut grid = vec![false; (out_w * out_h) as usize];
    for oy in 0..out_h {
        let y0 = (oy as f32 * scale) as u32;
        let y1 = (((oy + 1) as f32 * scale).ceil() as u32).min(height).max(y0 + 1);
        for ox in 0..out_w {
            let x0 = (ox as f32 * scale) as u32;
            let x1 = (((ox + 1) as f32 * scale).ceil() as u32).min(width).max(x0 + 1);
            grid[(oy * out_w + ox) as usize] =
                EinkTerminalBackend::block_has_ink(packed, stride, x0, x1, y0, y1);
        }
    }
    (grid, out_w, out_h)
}

impl EinkBackend for EinkTerminalBackend {
    fn flush(&mut self, packed: &[u8], width: u32, height: u32) {
        let stride = width.div_ceil(8) as usize;
        let (grid, out_w, out_h) = downsample(packed, stride, width, height);
        let at = |x: u32, y: u32| -> bool {
            y < out_h && x < out_w && grid[(y * out_w + x) as usize]
        };

        // Home the cursor + clear screen with ANSI (no subprocess) so the
        // frame redraws in place — the "live" iteration view.
        let mut buf = String::from("\x1b[2J\x1b[H");
        // Two downsampled rows per text row via half-blocks.
        let mut y = 0u32;
        while y < out_h {
            for x in 0..out_w {
                let top = at(x, y);
                let bottom = at(x, y + 1);
                buf.push(match (top, bottom) {
                    (true, true) => '\u{2588}',  // █ full block
                    (true, false) => '\u{2580}', // ▀ upper half
                    (false, true) => '\u{2584}', // ▄ lower half
                    (false, false) => ' ',
                });
            }
            buf.push('\n');
            y += 2;
        }
        // Footer so the scaling is discoverable.
        buf.push_str(&format!(
            "\x1b[2m[eink {width}x{height} → {out_w}x{out_h} cols; OHMYOLED_EINK_COLS to resize]\x1b[0m\n"
        ));

        let _ = std::io::stdout().write_all(buf.as_bytes());
        let _ = std::io::stdout().flush();

        if let Some(path) = &self.png_path {
            write_png(path, packed, stride, width, height);
        }
    }

    fn clear(&mut self) {
        let _ = std::io::stdout().write_all(b"\x1b[2J\x1b[H");
        let _ = std::io::stdout().flush();
    }
}

/// Dump the packed frame as a full-resolution grayscale PNG for exact
/// inspection (ink = black, sheet = white).
fn write_png(path: &str, packed: &[u8], stride: usize, width: u32, height: u32) {
    let mut img = image::GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let ink = EinkTerminalBackend::is_ink(packed, stride, x, y);
            img.put_pixel(x, y, image::Luma([if ink { 0 } else { 255 }]));
        }
    }
    if let Err(e) = img.save(path) {
        log::warn!("eink: failed to write OHMYOLED_EINK_PNG to '{path}': {e}");
    }
}
