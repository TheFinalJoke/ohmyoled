//! Hardware backend — drives real RPi RGB-LED panels via the `rpi-led-matrix`
//! FFI bindings to Henner Zeller's `rpi-rgb-led-matrix` C++ library.
//!
//! Compiled when the `hardware` crate feature is enabled (the default).
//!
//! On non-Pi hardware `HardwareBackend::init` returns `Err`, which causes the
//! dispatcher in `RGBMatrix::new` to fall back to `TerminalBackend` automatically.

use image::RgbImage;
use rpi_led_matrix::{LedCanvas, LedColor, LedMatrix, LedMatrixOptions, LedRuntimeOptions};

use crate::backend::Backend;
use crate::options::MatrixOptions;

/// RPi hardware backend.
///
/// Call `HardwareBackend::init` to create one. Returns `Err` when not running on
/// RPi hardware (no GPIO found), which triggers automatic fall-back to test mode.
pub struct HardwareBackend {
    matrix: LedMatrix,
    canvas: Option<LedCanvas>,
    width: i32,
    height: i32,
}

// SAFETY: LedMatrix / LedCanvas wrap raw C pointers that the upstream crate
// does not mark as Send. The underlying rpi-rgb-led-matrix C library is safe
// to use from any thread as long as access is not concurrent. The scheduler
// owns the matrix and lends it as &mut to one renderer at a time — so cross-
// thread moves (e.g. tokio task migration) are sound, but parallel access
// is not, which matches `Send`-but-not-`Sync`.
unsafe impl Send for HardwareBackend {}

impl HardwareBackend {
    /// Try to initialise the hardware backend.
    ///
    /// Returns `Err` on any non-Pi machine so the caller can fall back gracefully.
    pub fn init(options: MatrixOptions) -> Result<Self, String> {
        if !is_rpi() {
            return Err("GPIO unavailable — not running on Raspberry Pi hardware".to_string());
        }

        let mut opts = LedMatrixOptions::new();
        opts.set_hardware_mapping(&options.hardware_mapping);
        opts.set_rows(options.rows);
        opts.set_cols(options.cols);
        opts.set_chain_length(options.chain_length);
        opts.set_parallel(options.parallel);
        opts.set_brightness(options.brightness.min(100) as u8)
            .map_err(|e| format!("brightness rejected: {e}"))?;

        let mut rt = LedRuntimeOptions::new();
        rt.set_gpio_slowdown(options.gpio_slowdown);

        let matrix = LedMatrix::new(Some(opts), Some(rt))
            .map_err(|e| format!("failed to initialise RGB matrix: {e}"))?;

        let canvas = matrix.offscreen_canvas();
        let (width, height) = canvas.canvas_size();

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
        canvas.clear();

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
                let color = LedColor {
                    red: pixel[0],
                    green: pixel[1],
                    blue: pixel[2],
                };
                canvas.set(px, py, &color);
            }
        }

        self.canvas = Some(self.matrix.swap(canvas));
    }

    fn clear(&mut self) {
        let Some(mut canvas) = self.canvas.take() else { return };
        canvas.clear();
        self.canvas = Some(self.matrix.swap(canvas));
    }
}

/// Heuristic check for Raspberry Pi: look for the BCM CPU in /proc/cpuinfo.
fn is_rpi() -> bool {
    std::fs::read_to_string("/proc/cpuinfo")
        .map(|s| s.contains("BCM") || s.contains("Raspberry"))
        .unwrap_or(false)
}
