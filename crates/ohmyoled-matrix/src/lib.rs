//! ohmyoled-matrix: LED matrix abstraction for the OhMyOled project.
//!
//! Provides a `RGBMatrix` type with two backends:
//! - **Terminal** (always available): renders ANSI true-colour to stdout — the official test mode.
//! - **Hardware** (feature `hardware`): drives real RPi RGB-LED panels via vendored rpi-led-panel.
//!
//! # Choosing a mode
//!
//! 1. **Explicit**: `RGBMatrix::test(options)` forces terminal mode.
//! 2. **Env var**: set `OHMYOLED_MATRIX_MODE=test` to override at runtime.
//! 3. **Auto** (default): `RGBMatrix::new(options)` tries hardware; falls back to terminal with a warning.
//!
//! # Quick example
//! ```no_run
//! use ohmyoled_matrix::{RGBMatrix, MatrixOptions};
//! use image::RgbImage;
//!
//! let opts = MatrixOptions::default();
//! let mut matrix = RGBMatrix::test(opts);
//! let img = RgbImage::new(64, 32);
//! matrix.set_image(&img, 0, 0);
//! ```

pub mod backend;
pub mod eink;
pub mod graphics;
pub mod options;
pub mod terminal;

// The Pi backend only exists when the `hardware` feature is on AND we're
// compiling for an ARM target — `rpi-led-matrix` is a target-conditional
// dep in Cargo.toml, so referencing it on x86_64 would not compile.
#[cfg(all(feature = "hardware", any(target_arch = "arm", target_arch = "aarch64")))]
pub mod hardware;

pub use backend::{Backend, MatrixMode};
pub use eink::{EinkDisplay, EinkMode, EinkOptions};
pub use options::MatrixOptions;

/// An RGB colour — red, green, blue channels each 0–255.
///
/// Used by the graphics drawing primitives (`draw_text`, `draw_line`, `draw_circle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255 };
    pub const RED: Color   = Color { r: 255, g: 0,   b: 0   };
    pub const GREEN: Color = Color { r: 0,   g: 255, b: 0   };
    pub const BLUE: Color  = Color { r: 0,   g: 0,   b: 255 };
}

/// The main matrix handle. Owns the active backend (terminal or hardware).
///
/// See the crate-level docs for mode selection rules.
pub struct RGBMatrix {
    backend: Box<dyn Backend>,
    pub options: MatrixOptions,
    pub mode: MatrixMode,
}

impl RGBMatrix {
    /// Create a matrix using automatic mode selection.
    ///
    /// 1. If `OHMYOLED_MATRIX_MODE` env var is set, it takes precedence.
    /// 2. If the `hardware` feature is enabled and GPIO is detected, uses hardware.
    /// 3. Otherwise falls back to terminal mode and logs a warning.
    pub fn new(options: MatrixOptions) -> Self {
        let env_mode = backend::mode_from_env();
        let mode = env_mode.unwrap_or(MatrixMode::Auto);
        Self::with_mode(options, mode)
    }

    /// Create a matrix forced into terminal (test) mode.
    pub fn test(options: MatrixOptions) -> Self {
        Self::with_mode(options, MatrixMode::Test)
    }

    /// Create a matrix with an explicit mode.
    ///
    /// `Hardware` mode returns an error-fallback to `Test` if GPIO is unavailable.
    pub fn with_mode(options: MatrixOptions, mode: MatrixMode) -> Self {
        let (backend, actual_mode): (Box<dyn Backend>, MatrixMode) = match mode {
            MatrixMode::Test => {
                (Box::new(terminal::TerminalBackend::new()), MatrixMode::Test)
            }
            MatrixMode::Hardware => {
                #[cfg(all(feature = "hardware", any(target_arch = "arm", target_arch = "aarch64")))]
                {
                    match hardware::HardwareBackend::init(options.clone()) {
                        Ok(hw) => (Box::new(hw), MatrixMode::Hardware),
                        Err(e) => {
                            log::warn!("Hardware init failed ({e}), falling back to terminal mode");
                            (Box::new(terminal::TerminalBackend::new()), MatrixMode::Test)
                        }
                    }
                }
                #[cfg(not(all(feature = "hardware", any(target_arch = "arm", target_arch = "aarch64"))))]
                {
                    log::warn!("hardware backend unavailable on this target, using terminal mode");
                    (Box::new(terminal::TerminalBackend::new()), MatrixMode::Test)
                }
            }
            MatrixMode::Auto => {
                #[cfg(all(feature = "hardware", any(target_arch = "arm", target_arch = "aarch64")))]
                {
                    match hardware::HardwareBackend::init(options.clone()) {
                        Ok(hw) => (Box::new(hw), MatrixMode::Hardware),
                        Err(e) => {
                            log::warn!("Hardware unavailable ({e}), using terminal mode");
                            (Box::new(terminal::TerminalBackend::new()), MatrixMode::Test)
                        }
                    }
                }
                #[cfg(not(all(feature = "hardware", any(target_arch = "arm", target_arch = "aarch64"))))]
                {
                    (Box::new(terminal::TerminalBackend::new()), MatrixMode::Test)
                }
            }
        };

        Self { backend, options, mode: actual_mode }
    }

    /// Push an image to the display.
    ///
    /// `offset_x` / `offset_y` shift where the image is placed on the panel.
    pub fn set_image(&mut self, img: &image::RgbImage, offset_x: i32, offset_y: i32) {
        self.backend.set_image(img, offset_x, offset_y);
    }

    /// Push raw RGB bytes to the display (3 bytes/pixel, no padding).
    ///
    /// Returns `Err` if the byte length doesn't match `width * height * 3`.
    /// This is the fast path for Rust modules that build images natively — no
    /// PIL or other intermediate format needed.
    pub fn set_image_bytes(&mut self, width: u32, height: u32, bytes: Vec<u8>, offset_x: i32, offset_y: i32) -> Result<(), String> {
        let img = image::RgbImage::from_raw(width, height, bytes)
            .ok_or_else(|| format!("byte buffer wrong size for {width}x{height} RGB image"))?;
        self.backend.set_image(&img, offset_x, offset_y);
        Ok(())
    }

    /// Clear the display to black.
    pub fn clear(&mut self) {
        self.backend.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_forces_test_mode() {
        std::env::set_var("OHMYOLED_MATRIX_MODE", "test");
        let m = RGBMatrix::new(MatrixOptions::default());
        assert_eq!(m.mode, MatrixMode::Test);
        std::env::remove_var("OHMYOLED_MATRIX_MODE");
    }

    #[test]
    fn explicit_test_constructor_gives_test_mode() {
        let m = RGBMatrix::test(MatrixOptions::default());
        assert_eq!(m.mode, MatrixMode::Test);
    }

    #[test]
    fn auto_falls_back_to_test_on_non_pi() {
        // This devcontainer is never on Pi hardware.
        std::env::remove_var("OHMYOLED_MATRIX_MODE");
        let m = RGBMatrix::new(MatrixOptions::default());
        // Either test or hardware — but since we're not on Pi, expect test.
        assert_eq!(m.mode, MatrixMode::Test);
    }

    #[test]
    fn matrix_options_defaults() {
        let opts = MatrixOptions::default();
        assert_eq!(opts.cols, 64);
        assert_eq!(opts.rows, 32);
        assert_eq!(opts.brightness, 60);
    }

    #[test]
    fn set_image_does_not_panic_in_test_mode() {
        let mut m = RGBMatrix::test(MatrixOptions::default());
        let img = image::RgbImage::new(4, 4);
        // Should not panic; clears + writes to stdout (output not checked in unit test)
        m.set_image(&img, 0, 0);
    }
}
