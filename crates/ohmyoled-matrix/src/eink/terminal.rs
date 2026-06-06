//! Terminal backend for [`super::EinkDisplay`] — renders the packed 1-bpp
//! frame to stdout using Unicode half-block characters.
//!
//! This is the always-available dev backend: it lets you iterate on e-paper
//! layouts with no hardware, the e-ink analog of [`crate::terminal`]. Each
//! printed character row encodes two vertically-stacked panel pixels via the
//! upper/lower half-block glyphs, so the aspect ratio reads roughly true.
//!
//! Ink pixels (the drawn foreground) print as lit half-blocks; the white sheet
//! prints blank — so the terminal shows exactly what would be inked onto the
//! panel. A blank (all-white) panel prints empty.
//!
//! Set `OHMYOLED_EINK_PNG=/path/to/out.png` to additionally dump each frame
//! as a pixel-exact PNG (handy for large panels that overflow the terminal).

use super::EinkBackend;
use std::io::Write;

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
}

impl EinkBackend for EinkTerminalBackend {
    fn flush(&mut self, packed: &[u8], width: u32, height: u32) {
        let stride = width.div_ceil(8) as usize;

        // Home the cursor + clear screen with ANSI (no subprocess) so the
        // frame redraws in place — the "live" iteration view.
        let mut buf = String::from("\x1b[2J\x1b[H");
        // Two panel rows per text row via half-blocks.
        let mut y = 0u32;
        while y < height {
            for x in 0..width {
                let top = Self::is_ink(packed, stride, x, y);
                let bottom = if y + 1 < height {
                    Self::is_ink(packed, stride, x, y + 1)
                } else {
                    false
                };
                buf.push(match (top, bottom) {
                    (true, true) => '\u{2588}',   // █ full block
                    (true, false) => '\u{2580}',  // ▀ upper half
                    (false, true) => '\u{2584}',  // ▄ lower half
                    (false, false) => ' ',
                });
            }
            buf.push('\n');
            y += 2;
        }

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

/// Dump the packed frame as a 1-bit-looking grayscale PNG for exact inspection.
fn write_png(path: &str, packed: &[u8], stride: usize, width: u32, height: u32) {
    let mut img = image::GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            // Panel-accurate: ink is black, sheet is white.
            let ink = EinkTerminalBackend::is_ink(packed, stride, x, y);
            img.put_pixel(x, y, image::Luma([if ink { 0 } else { 255 }]));
        }
    }
    if let Err(e) = img.save(path) {
        log::warn!("eink: failed to write OHMYOLED_EINK_PNG to '{path}': {e}");
    }
}
