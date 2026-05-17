//! Terminal backend — renders frames as ANSI true-colour to stdout.
//!
//! This is the official test mode: it works on any machine with a colour terminal,
//! no GPIO or RPi hardware required. Always compiled in; never behind a feature flag.
//!
//! Pixel rendering matches the Python `TerminalMatrix` implementation exactly:
//! black pixels are drawn as spaces; non-black pixels use `\x1b[38;2;R;G;Bm•\x1b[0m`.

use crate::backend::Backend;
use image::RgbImage;
use std::io::Write;

/// The terminal (test-mode) backend.
#[derive(Default)]
pub struct TerminalBackend;

impl TerminalBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Backend for TerminalBackend {
    fn set_image(&mut self, img: &RgbImage, _offset_x: i32, _offset_y: i32) {
        let w = img.width() as usize;
        let h = img.height() as usize;
        // Pre-allocate: worst case each pixel is ~25 bytes of ANSI escape
        let mut buf = String::with_capacity(h * (w * 25 + 1));

        for y in 0..h {
            for x in 0..w {
                let px = img.get_pixel(x as u32, y as u32);
                let [r, g, b] = px.0;
                if r == 0 && g == 0 && b == 0 {
                    buf.push(' ');
                } else {
                    buf.push_str(&format!("\x1b[38;2;{r};{g};{b}m\u{2022}\x1b[0m"));
                }
            }
            buf.push('\n');
        }

        self.clear();
        let _ = std::io::stdout().write_all(buf.as_bytes());
    }

    fn clear(&mut self) {
        // Use the same clear strategy as the Python TerminalMatrix
        if cfg!(windows) {
            let _ = std::process::Command::new("cls").status();
        } else {
            let _ = std::process::Command::new("clear").status();
        }
    }
}
