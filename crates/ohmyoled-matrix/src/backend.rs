//! Backend trait and mode-selection dispatcher.
//!
//! The `Backend` trait is the single interface that both the terminal and hardware
//! implementations satisfy. `select_backend` picks which one to instantiate based on
//! the `MatrixMode` plus the `OHMYOLED_MATRIX_MODE` env var.

use image::RgbImage;

/// Selects which backend the matrix uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatrixMode {
    /// Try hardware first; fall back to terminal if GPIO is unavailable.
    #[default]
    Auto,
    /// Always use the ANSI terminal backend (works on any machine).
    Test,
    /// Always use the RPi hardware backend; fails if not on Pi.
    Hardware,
}

/// Core matrix operations — implemented by both backends.
pub trait Backend: Send + Sync {
    /// Push an RGB image to the display.
    fn set_image(&mut self, img: &RgbImage, offset_x: i32, offset_y: i32);
    /// Clear the display to black.
    fn clear(&mut self);
}

/// Read the `OHMYOLED_MATRIX_MODE` env var and parse it into a `MatrixMode`.
/// Returns `None` if the var is unset or unrecognised (caller uses its default).
pub fn mode_from_env() -> Option<MatrixMode> {
    match std::env::var("OHMYOLED_MATRIX_MODE").ok()?.to_lowercase().as_str() {
        "test" | "terminal" => Some(MatrixMode::Test),
        "hardware" | "hw" => Some(MatrixMode::Hardware),
        "auto" => Some(MatrixMode::Auto),
        _ => None,
    }
}
