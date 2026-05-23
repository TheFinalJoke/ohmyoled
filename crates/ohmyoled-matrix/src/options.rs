//! Matrix configuration options — mirrors the 7 fields used in main.py.

/// Configuration for the LED matrix panel.
///
/// All fields default to sensible values matching the OhMyOled hardware setup.
/// Override only the fields you need to change.
///
/// # Example
/// ```
/// use ohmyoled_matrix::MatrixOptions;
/// let opts = MatrixOptions { cols: 64, rows: 32, brightness: 60, ..Default::default() };
/// ```
#[derive(Debug, Clone)]
pub struct MatrixOptions {
    /// Number of pixel columns per panel (default: 64).
    pub cols: u32,
    /// Number of pixel rows per panel (default: 32).
    pub rows: u32,
    /// Number of panels daisy-chained together (default: 1).
    pub chain_length: u32,
    /// Number of parallel chains (default: 1).
    pub parallel: u32,
    /// GPIO slowdown factor for signal timing — higher = slower (default: 4).
    pub gpio_slowdown: u32,
    /// Panel brightness 0–100 (default: 60).
    pub brightness: u32,
    /// Hardware pin mapping name, e.g. `"adafruit-hat"` (default: `"adafruit-hat"`).
    pub hardware_mapping: String,
}

impl Default for MatrixOptions {
    fn default() -> Self {
        Self {
            cols: 64,
            rows: 32,
            chain_length: 1,
            parallel: 1,
            gpio_slowdown: 4,
            brightness: 60,
            hardware_mapping: "adafruit-hat".to_string(),
        }
    }
}
