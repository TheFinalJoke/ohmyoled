//! Hardware backend — drives real RPi RGB-LED panels via the `rpi-led-panel` crate.
//!
//! Compiled when the `hardware` crate feature is enabled (the default).
//!
//! On non-Pi hardware `HardwareBackend::init` returns `Err`, which causes the
//! dispatcher in `RGBMatrix::new` to fall back to `TerminalBackend` automatically.

use std::str::FromStr;

use image::RgbImage;
use rpi_led_panel::{Canvas, HardwareMapping, RGBMatrix as PanelMatrix, RGBMatrixConfig};

use crate::backend::Backend;
use crate::options::MatrixOptions;

/// RPi hardware backend.
///
/// Call `HardwareBackend::init` to create one. Returns `Err` when not running on
/// RPi hardware (no GPIO found), which triggers automatic fall-back to test mode.
pub struct HardwareBackend {
    matrix: PanelMatrix,
    canvas: Option<Box<Canvas>>,
    width: i32,
    height: i32,
}

impl HardwareBackend {
    /// Try to initialise the hardware backend.
    ///
    /// Returns `Err` on any non-Pi machine so the caller can fall back gracefully.
    pub fn init(options: MatrixOptions) -> Result<Self, String> {
        if !is_rpi() {
            return Err("GPIO unavailable — not running on Raspberry Pi hardware".to_string());
        }

        let hardware_mapping = HardwareMapping::from_str(&options.hardware_mapping)
            .map_err(|e| format!("unknown hardware_mapping `{}`: {e}", options.hardware_mapping))?;

        let config = RGBMatrixConfig {
            hardware_mapping,
            rows: options.rows as usize,
            cols: options.cols as usize,
            chain_length: options.chain_length as usize,
            parallel: options.parallel as usize,
            slowdown: Some(options.gpio_slowdown),
            led_brightness: options.brightness.min(100) as u8,
            ..RGBMatrixConfig::default()
        };

        let (matrix, canvas) = PanelMatrix::new(config, 0)
            .map_err(|e| format!("failed to initialise RGB matrix: {e}"))?;

        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        Ok(Self {
            matrix,
            canvas: Some(canvas),
            width,
            height,
        })
    }
}

impl Backend for HardwareBackend {
    fn set_image(&mut self, img: &RgbImage, offset_x: i32, offset_y: i32) {
        let Some(mut canvas) = self.canvas.take() else { return };
        canvas.fill(0, 0, 0);

        let img_w = img.width() as i32;
        let img_h = img.height() as i32;
        for y in 0..img_h {
            let py = y + offset_y;
            if py < 0 || py >= self.height {
                continue;
            }
            for x in 0..img_w {
                let px = x + offset_x;
                if px < 0 || px >= self.width {
                    continue;
                }
                let pixel = img.get_pixel(x as u32, y as u32);
                canvas.set_pixel(px as usize, py as usize, pixel[0], pixel[1], pixel[2]);
            }
        }

        self.canvas = Some(self.matrix.update_on_vsync(canvas));
    }

    fn clear(&mut self) {
        let Some(mut canvas) = self.canvas.take() else { return };
        canvas.fill(0, 0, 0);
        self.canvas = Some(self.matrix.update_on_vsync(canvas));
    }
}

/// Heuristic check for Raspberry Pi: look for the BCM CPU in /proc/cpuinfo.
fn is_rpi() -> bool {
    std::fs::read_to_string("/proc/cpuinfo")
        .map(|s| s.contains("BCM") || s.contains("Raspberry"))
        .unwrap_or(false)
}
