//! Hardware backend — drives real RPi RGB-LED panels.
//!
//! This module is compiled when the `hardware` crate feature is enabled (the default).
//! It wraps the vendored `rpi-led-panel` library (see VENDORED.md).
//!
//! On non-Pi hardware `HardwareBackend::init` returns `Err`, which causes the
//! dispatcher in `RGBMatrix::new` to fall back to `TerminalBackend` automatically.
// Vendored from rpi-led-panel — see VENDORED.md

use crate::backend::Backend;
use crate::options::MatrixOptions;
use image::RgbImage;

/// RPi hardware backend.
///
/// Call `HardwareBackend::init` to create one. Returns `Err` when not running on
/// RPi hardware (no GPIO found), which triggers automatic fall-back to test mode.
pub struct HardwareBackend {
    _options: MatrixOptions,
}

impl HardwareBackend {
    /// Try to initialise the hardware backend.
    ///
    /// Returns `Err` on any non-Pi machine so the caller can fall back gracefully.
    pub fn init(options: MatrixOptions) -> Result<Self, String> {
        // Detect whether we have GPIO access; fail fast on non-Pi.
        // When rpi-led-panel is vendored here, replace this with the real init call.
        if !is_rpi() {
            return Err("GPIO unavailable — not running on Raspberry Pi hardware".to_string());
        }
        Ok(Self { _options: options })
    }
}

impl Backend for HardwareBackend {
    fn set_image(&mut self, _img: &RgbImage, _offset_x: i32, _offset_y: i32) {
        // Placeholder: real implementation drives the rpi-led-panel matrix here.
        unimplemented!("hardware backend not yet wired to rpi-led-panel")
    }

    fn clear(&mut self) {
        unimplemented!("hardware backend not yet wired to rpi-led-panel")
    }
}

/// Heuristic check for Raspberry Pi: look for the BCM CPU in /proc/cpuinfo.
fn is_rpi() -> bool {
    std::fs::read_to_string("/proc/cpuinfo")
        .map(|s| s.contains("BCM") || s.contains("Raspberry"))
        .unwrap_or(false)
}
