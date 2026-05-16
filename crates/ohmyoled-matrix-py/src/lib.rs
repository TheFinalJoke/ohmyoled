//! Python bindings for ohmyoled-matrix — thin pyo3 wrappers only.
// PascalCase method names (SetImage, Clear) are intentional: they
// match the rgbmatrix Python API so existing call sites don't need changes.
#![allow(non_snake_case)]
//!
//! Rule: every method body here is 1–2 lines that delegate to the Rust crate.
//! If a method grows beyond that, the logic belongs in `ohmyoled-matrix`, not here.

use ohmyoled_matrix::{MatrixMode, MatrixOptions, RGBMatrix};
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

// ---------------------------------------------------------------------------
// RGBMatrixOptions
// ---------------------------------------------------------------------------

/// Configuration for the LED matrix panel.
///
/// All fields mirror the original `rgbmatrix.RGBMatrixOptions` attributes used
/// by OhMyOled so existing `main.py` code requires only an import change.
#[pyclass(name = "RGBMatrixOptions")]
#[derive(Clone)]
struct PyRGBMatrixOptions {
    inner: MatrixOptions,
}

#[pymethods]
impl PyRGBMatrixOptions {
    #[new]
    fn new() -> Self {
        Self { inner: MatrixOptions::default() }
    }

    #[getter] fn cols(&self) -> u32 { self.inner.cols }
    #[setter] fn set_cols(&mut self, v: u32) { self.inner.cols = v; }

    #[getter] fn rows(&self) -> u32 { self.inner.rows }
    #[setter] fn set_rows(&mut self, v: u32) { self.inner.rows = v; }

    #[getter] fn chain_length(&self) -> u32 { self.inner.chain_length }
    #[setter] fn set_chain_length(&mut self, v: u32) { self.inner.chain_length = v; }

    #[getter] fn parallel(&self) -> u32 { self.inner.parallel }
    #[setter] fn set_parallel(&mut self, v: u32) { self.inner.parallel = v; }

    #[getter] fn gpio_slowdown(&self) -> u32 { self.inner.gpio_slowdown }
    #[setter] fn set_gpio_slowdown(&mut self, v: u32) { self.inner.gpio_slowdown = v; }

    #[getter] fn brightness(&self) -> u32 { self.inner.brightness }
    #[setter] fn set_brightness(&mut self, v: u32) { self.inner.brightness = v; }

    #[getter] fn hardware_mapping(&self) -> &str { &self.inner.hardware_mapping }
    #[setter] fn set_hardware_mapping(&mut self, v: String) { self.inner.hardware_mapping = v; }
}

// ---------------------------------------------------------------------------
// RGBMatrix
// ---------------------------------------------------------------------------

/// LED matrix handle. Wraps `ohmyoled_matrix::RGBMatrix`.
///
/// # Mode selection
/// - `test_mode=True` → always uses the ANSI terminal backend (safe on any machine).
/// - `OHMYOLED_MATRIX_MODE=test` env var → same effect, no code change needed.
/// - Default (neither): tries hardware, falls back to terminal with a log warning.
#[pyclass(name = "RGBMatrix")]
struct PyRGBMatrix {
    inner: RGBMatrix,
}

#[pymethods]
impl PyRGBMatrix {
    /// Create a matrix.
    ///
    /// Args:
    ///     options: RGBMatrixOptions — panel configuration.
    ///     test_mode: bool — if True, forces terminal (ANSI) rendering. Default False.
    #[new]
    #[pyo3(signature = (options=None, test_mode=false))]
    fn new(options: Option<PyRGBMatrixOptions>, test_mode: bool) -> Self {
        let opts = options.map(|o| o.inner).unwrap_or_default();
        let inner = if test_mode {
            RGBMatrix::test(opts)
        } else {
            RGBMatrix::new(opts)
        };
        Self { inner }
    }

    /// Push a Pillow image to the display.
    ///
    /// `image` must be a `PIL.Image.Image` in RGB mode.
    /// `offset_x` / `offset_y` shift placement on the panel (default 0).
    #[pyo3(signature = (image, offset_x=0, offset_y=0))]
    fn SetImage(&mut self, py: Python<'_>, image: PyObject, offset_x: i32, offset_y: i32) -> PyResult<()> {
        let rgb_img = pil_to_rgb_image(py, &image)?;
        self.inner.set_image(&rgb_img, offset_x, offset_y);
        Ok(())
    }

    /// Push raw RGB bytes to the display (3 bytes/pixel).
    ///
    /// Used by Rust-implemented modules that build images natively — no PIL
    /// round-trip needed. `bytes` length must equal `width * height * 3`.
    #[pyo3(signature = (width, height, bytes, offset_x=0, offset_y=0))]
    fn set_image_bytes(&mut self, width: u32, height: u32, bytes: Vec<u8>, offset_x: i32, offset_y: i32) -> PyResult<()> {
        self.inner.set_image_bytes(width, height, bytes, offset_x, offset_y)
            .map_err(PyRuntimeError::new_err)
    }

    /// Clear the display to black.
    fn Clear(&mut self) {
        self.inner.clear();
    }
}

// ---------------------------------------------------------------------------
// TerminalMatrix — convenience alias that always uses terminal mode
// ---------------------------------------------------------------------------

/// Convenience class that always uses the ANSI terminal backend.
///
/// Equivalent to `RGBMatrix(options=opts, test_mode=True)`.
#[pyclass(name = "TerminalMatrix")]
struct PyTerminalMatrix {
    inner: RGBMatrix,
}

#[pymethods]
impl PyTerminalMatrix {
    #[new]
    #[pyo3(signature = (options=None))]
    fn new(options: Option<PyRGBMatrixOptions>) -> Self {
        let opts = options.map(|o| o.inner).unwrap_or_default();
        Self { inner: RGBMatrix::with_mode(opts, MatrixMode::Test) }
    }

    #[pyo3(signature = (image, offset_x=0, offset_y=0))]
    fn SetImage(&mut self, py: Python<'_>, image: PyObject, offset_x: i32, offset_y: i32) -> PyResult<()> {
        let rgb_img = pil_to_rgb_image(py, &image)?;
        self.inner.set_image(&rgb_img, offset_x, offset_y);
        Ok(())
    }

    fn Clear(&mut self) {
        self.inner.clear();
    }
}

// ---------------------------------------------------------------------------
// Helper: convert a PIL Image to image::RgbImage
// ---------------------------------------------------------------------------

/// Extract raw RGB bytes from a PIL Image object.
///
/// Converts to RGB mode first so we always get 3 bytes per pixel.
fn pil_to_rgb_image(py: Python<'_>, pil_image: &PyObject) -> PyResult<image::RgbImage> {
    let img = pil_image.bind(py);

    // Ensure the image is in RGB mode by calling .convert("RGB")
    let rgb_img = img.call_method1("convert", ("RGB",))?;

    let width: u32 = rgb_img.getattr("width")?.extract()?;
    let height: u32 = rgb_img.getattr("height")?.extract()?;

    // tobytes() returns the raw pixel data as bytes
    let raw_bytes: Vec<u8> = rgb_img.call_method0("tobytes")?.extract()?;

    image::RgbImage::from_raw(width, height, raw_bytes)
        .ok_or_else(|| PyRuntimeError::new_err("PIL image byte buffer size mismatch"))
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Root pyo3 module: `ohmyoled_matrix._ohmyoled_matrix`
#[pymodule]
fn _ohmyoled_matrix(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRGBMatrixOptions>()?;
    m.add_class::<PyRGBMatrix>()?;
    m.add_class::<PyTerminalMatrix>()?;
    Ok(())
}
