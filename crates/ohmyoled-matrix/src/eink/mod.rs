//! E-paper (e-ink) display abstraction — the static, high-resolution sibling
//! of [`crate::RGBMatrix`].
//!
//! Where the LED matrix renders a 64×32 animated frame many times a second,
//! an e-paper panel shows one large, static, monochrome image and refreshes
//! it slowly (seconds). [`EinkDisplay`] is the output target for that world:
//! renderers compose a full-resolution [`image::RgbImage`], hand it to
//! [`EinkDisplay::show`], and it gets thresholded to 1 bit-per-pixel and
//! pushed to whichever backend is active.
//!
//! # Backends
//!
//! - **Terminal** (always compiled): renders the 1-bpp buffer to stdout as a
//!   real inline image (Sixel, with a live-reloading PNG fallback) so you can
//!   iterate on layouts with no hardware — the e-ink analog of
//!   [`crate::terminal`]. This is the default off-Pi.
//! - **Hardware** (feature `eink`, ARM only): drives a real Waveshare panel
//!   over SPI via `rppal` + `epd-waveshare`. See [`hardware`].
//!
//! # Choosing a backend
//!
//! Mirrors the LED matrix: [`EinkDisplay::test`] forces terminal mode,
//! [`EinkDisplay::new`] auto-selects (hardware on a Pi, terminal otherwise),
//! and `OHMYOLED_EINK_MODE=terminal|hardware|auto` overrides at runtime.

use image::RgbImage;

pub mod terminal;

#[cfg(all(feature = "eink", any(target_arch = "arm", target_arch = "aarch64")))]
pub mod hardware;

/// Which backend an [`EinkDisplay`] uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EinkMode {
    /// Try the Waveshare hardware backend; fall back to terminal off-Pi.
    #[default]
    Auto,
    /// Always render to the terminal (works on any machine).
    Terminal,
    /// Always drive the Waveshare panel; falls back to terminal if the
    /// `eink` feature/target isn't available.
    Hardware,
}

/// Parse a backend name (e.g. from config or env) into an [`EinkMode`].
/// Returns `None` for unset/unrecognised so callers can fall back.
pub fn parse_mode(s: &str) -> Option<EinkMode> {
    match s.trim().to_lowercase().as_str() {
        "test" | "terminal" => Some(EinkMode::Terminal),
        "hardware" | "hw" => Some(EinkMode::Hardware),
        "auto" => Some(EinkMode::Auto),
        _ => None,
    }
}

/// Read `OHMYOLED_EINK_MODE` and parse it into an [`EinkMode`].
/// Returns `None` when unset/unrecognised so the caller keeps its default.
pub fn mode_from_env() -> Option<EinkMode> {
    parse_mode(&std::env::var("OHMYOLED_EINK_MODE").ok()?)
}

/// Configuration for an e-paper panel.
///
/// `model` selects the panel (and therefore its native resolution + the
/// concrete `epd-waveshare` driver on hardware). `threshold` is the luma
/// cutoff used when collapsing the composed RGB image to black/white.
#[derive(Debug, Clone)]
pub struct EinkOptions {
    /// Panel model id, e.g. `"4in2"`, `"7in5_v2"`, `"2in13"`. Drives the
    /// resolution via [`model_dimensions`] and the hardware driver.
    pub model: String,
    /// Display rotation in degrees (0/90/180/270). Currently informational
    /// for the terminal backend; honoured by the hardware backend.
    pub rotation: u16,
    /// Luma threshold 0–255: pixels at or above are white, below are black.
    pub threshold: u8,
    /// Explicit width override. When both `width` and `height` are set they
    /// take precedence over the model-derived resolution — for panels not in
    /// the model table, or custom dev sizes. Left `None` to use the model.
    pub width: Option<u32>,
    /// Explicit height override (see [`EinkOptions::width`]).
    pub height: Option<u32>,
    /// Configured backend (`Terminal`/`Hardware`/`Auto`). `None` defers to
    /// `OHMYOLED_EINK_MODE`, then to [`EinkMode::Auto`]. The env var always
    /// wins over this, so a config default can be overridden at runtime.
    pub mode: Option<EinkMode>,
    /// Configured default for the terminal backend's hardware emulation
    /// (full-refresh flash + dwell). `OHMYOLED_EINK_EMULATE` overrides it;
    /// `None` ⇒ off unless the env var enables it.
    pub emulate: Option<bool>,
    /// Configured emulated full-refresh duration in ms. `OHMYOLED_EINK_REFRESH_MS`
    /// overrides it; `None` ⇒ the backend's built-in default.
    pub refresh_ms: Option<u64>,
}

impl Default for EinkOptions {
    fn default() -> Self {
        Self {
            // 7.5" B/W V2 (800×480) — the recommended panel: lots of room for a
            // data dashboard.
            model: "7in5_v2".to_string(),
            rotation: 0,
            threshold: 128,
            width: None,
            height: None,
            mode: None,
            emulate: None,
            refresh_ms: None,
        }
    }
}

impl EinkOptions {
    /// Resolve the logical canvas size: an explicit `width` + `height` pair
    /// wins; otherwise it's derived from `model` via [`model_dimensions`].
    ///
    /// Note: on the hardware backend the panel *driver* is still selected by
    /// `model`, so an override should match the physical panel — it's meant
    /// for dev/preview and for unlisted panels.
    pub fn dimensions(&self) -> (u32, u32) {
        match (self.width, self.height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => (w, h),
            _ => model_dimensions(&self.model),
        }
    }
}

/// Native (width, height) in pixels for a known panel `model`.
///
/// Unknown models fall back to 4.2" dimensions with a warning — the renderer
/// still gets a sane canvas rather than panicking.
pub fn model_dimensions(model: &str) -> (u32, u32) {
    match model.to_lowercase().replace(['-', '.', ' '], "_").as_str() {
        "1in54" | "1in54_v2" => (200, 200),
        "2in13" | "2in13_v2" | "2in13_v3" => (250, 122),
        "2in9" | "2in9_v2" => (296, 128),
        "2in7" => (264, 176),
        "4in2" | "4in2_v2" => (400, 300),
        "5in83" | "5in83_v2" => (648, 480),
        "7in5" | "7in5_v2" => (800, 480),
        other => {
            log::warn!("eink: unknown panel model '{other}', defaulting to 400x300 (4in2)");
            (400, 300)
        }
    }
}

/// A backend that can flush a packed 1-bpp frame to some surface.
///
/// `packed` is MSB-first, row-major, with a stride of `ceil(width/8)` bytes.
/// Bit `1` = white (no ink), bit `0` = black (ink) — an all-`0xFF` buffer is
/// blank/white, matching `epd-waveshare`'s convention.
pub trait EinkBackend: Send {
    /// Push one full frame. `width`/`height` describe the logical image; the
    /// stride is derived as `ceil(width/8)`.
    fn flush(&mut self, packed: &[u8], width: u32, height: u32);
    /// Push a frame using the panel's fast/partial refresh **without** the
    /// preceding clear — for frequent updates (e.g. a ticking clock) where the
    /// white-clear flash is undesirable. The default delegates to [`flush`] for
    /// backends with no distinct fast path (terminal, 4.2").
    fn flush_fast(&mut self, packed: &[u8], width: u32, height: u32) {
        self.flush(packed, width, height);
    }
    /// Blank the panel to white.
    fn clear(&mut self);
}

/// The e-paper display handle. Owns the active backend.
///
/// See the module docs for backend selection rules.
pub struct EinkDisplay {
    backend: Box<dyn EinkBackend>,
    width: u32,
    height: u32,
    threshold: u8,
    pub mode: EinkMode,
    /// The last packed buffer flushed to the panel. [`EinkDisplay::show`] skips
    /// the flush when the new frame is byte-identical — e-paper refreshes are
    /// slow and flashy, so re-pushing an unchanged image is pure waste (and a
    /// visible flicker on hardware / under emulation).
    last_packed: Option<Vec<u8>>,
}

impl EinkDisplay {
    /// Create a display using automatic backend selection
    /// (`OHMYOLED_EINK_MODE` wins, otherwise [`EinkMode::Auto`]).
    pub fn new(options: EinkOptions) -> Self {
        // Precedence: env var wins, then the configured mode, then Auto.
        let mode = mode_from_env().or(options.mode).unwrap_or(EinkMode::Auto);
        Self::with_mode(options, mode)
    }

    /// Create a display forced into terminal (dev) mode.
    pub fn test(options: EinkOptions) -> Self {
        Self::with_mode(options, EinkMode::Terminal)
    }

    /// Create a display with an explicit backend, falling back to terminal
    /// when the hardware backend isn't compiled in for this target.
    pub fn with_mode(options: EinkOptions, mode: EinkMode) -> Self {
        let (width, height) = options.dimensions();
        let threshold = options.threshold;

        let (backend, actual_mode): (Box<dyn EinkBackend>, EinkMode) = match mode {
            EinkMode::Terminal => (
                Box::new(terminal::EinkTerminalBackend::with_config(options.emulate, options.refresh_ms)),
                EinkMode::Terminal,
            ),
            EinkMode::Hardware | EinkMode::Auto => {
                #[cfg(all(feature = "eink", any(target_arch = "arm", target_arch = "aarch64")))]
                {
                    // `init()` blocks polling the panel's BUSY line with no
                    // internal timeout, so a mis-wired or mis-configured panel
                    // (BUSY stuck low, wrong interface mode) would hang the whole
                    // app forever. Run it on a worker thread and cap the wait —
                    // on timeout we leak that thread (still blocked in the driver,
                    // harmless: terminal mode never touches SPI) and degrade
                    // gracefully instead of freezing startup.
                    const INIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
                    let init_opts = options.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        let _ = tx.send(hardware::EinkHardwareBackend::init(&init_opts));
                    });
                    match rx.recv_timeout(INIT_TIMEOUT) {
                        Ok(Ok(hw)) => (Box::new(hw), EinkMode::Hardware),
                        Ok(Err(e)) => {
                            if mode == EinkMode::Hardware {
                                log::warn!("eink hardware init failed ({e}); using terminal mode");
                            } else {
                                log::info!("eink hardware unavailable ({e}); using terminal mode");
                            }
                            (Box::new(terminal::EinkTerminalBackend::with_config(options.emulate, options.refresh_ms)), EinkMode::Terminal)
                        }
                        Err(_) => {
                            log::error!(
                                "eink hardware init timed out after {}s — BUSY never released; \
                                 check the ribbon seating and HAT config switches (4-line SPI = 0, Display Config = A); \
                                 using terminal mode",
                                INIT_TIMEOUT.as_secs()
                            );
                            (Box::new(terminal::EinkTerminalBackend::with_config(options.emulate, options.refresh_ms)), EinkMode::Terminal)
                        }
                    }
                }
                #[cfg(not(all(feature = "eink", any(target_arch = "arm", target_arch = "aarch64"))))]
                {
                    log::info!(
                        "eink hardware backend unavailable on this target/feature set; using terminal mode"
                    );
                    (Box::new(terminal::EinkTerminalBackend::with_config(options.emulate, options.refresh_ms)), EinkMode::Terminal)
                }
            }
        };

        Self {
            backend,
            width,
            height,
            threshold,
            mode: actual_mode,
            last_packed: None,
        }
    }

    /// Native panel width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Native panel height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Threshold (compose, pack to 1-bpp, flush) one full frame to the panel.
    ///
    /// `img` should be `width() × height()`; larger/smaller images are packed
    /// against the panel dimensions (extra pixels ignored, missing ones blank).
    ///
    /// No-ops when the packed frame is byte-identical to the last one shown, so
    /// callers can re-render on a timer without forcing a needless refresh — the
    /// panel only updates when its pixels actually change.
    pub fn show(&mut self, img: &RgbImage) {
        let packed = pack_1bpp(img, self.width, self.height, self.threshold);
        // Skip the (slow, flashy) panel refresh when nothing changed.
        if self.last_packed.as_deref() == Some(packed.as_slice()) {
            return;
        }
        self.backend.flush(&packed, self.width, self.height);
        self.last_packed = Some(packed);
    }

    /// Push a frame using the fast/partial refresh (no clear) — for frequent
    /// updates like a ticking clock. Skips the refresh when nothing changed.
    /// Falls back to a full flush on backends without a fast path.
    pub fn show_fast(&mut self, img: &RgbImage) {
        let packed = pack_1bpp(img, self.width, self.height, self.threshold);
        if self.last_packed.as_deref() == Some(packed.as_slice()) {
            return;
        }
        self.backend.flush_fast(&packed, self.width, self.height);
        self.last_packed = Some(packed);
    }

    /// Blank the panel to white.
    pub fn clear(&mut self) {
        self.backend.clear();
        // The panel no longer matches any prior frame; force the next show().
        self.last_packed = None;
    }
}

/// Collapse an RGB image to a packed 1-bpp buffer for the panel.
///
/// Output layout: MSB-first, row-major, stride `ceil(width/8)` bytes. Bit `1`
/// = white (no ink), bit `0` = black (ink), matching `epd-waveshare`'s
/// `Display` packing so the bytes can go straight to the panel driver.
///
/// **Polarity:** the rest of the codebase composes *white-foreground on a
/// black background* (the LED convention the `draw_*` primitives are built
/// for). E-paper is the inverse — black ink on a white sheet — so a *bright*
/// source pixel (luma ≥ `threshold`, i.e. drawn foreground) becomes **ink**
/// (bit cleared) and a dark/background pixel stays white. The buffer therefore
/// starts all-white (`0xFF`) and bright pixels clear their bit.
pub fn pack_1bpp(img: &RgbImage, width: u32, height: u32, threshold: u8) -> Vec<u8> {
    let stride = width.div_ceil(8) as usize;
    let mut buf = vec![0xFFu8; stride * height as usize];
    let (iw, ih) = (img.width(), img.height());
    for y in 0..height.min(ih) {
        for x in 0..width.min(iw) {
            let px = img.get_pixel(x, y).0;
            // Rec. 601 luma — matches how the eye weights the channels.
            let luma = (px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000;
            if (luma as u8) >= threshold {
                // Bright foreground pixel → black ink → clear the bit.
                let byte = y as usize * stride + (x / 8) as usize;
                let bit = 0x80u8 >> (x % 8);
                buf[byte] &= !bit;
            }
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Backend that just counts how many frames it was asked to flush.
    struct CountingBackend {
        flushes: Arc<AtomicUsize>,
    }
    impl EinkBackend for CountingBackend {
        fn flush(&mut self, _packed: &[u8], _width: u32, _height: u32) {
            self.flushes.fetch_add(1, Ordering::SeqCst);
        }
        fn clear(&mut self) {}
    }

    fn white(w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        for p in img.pixels_mut() {
            *p = image::Rgb([255, 255, 255]);
        }
        img
    }

    #[test]
    fn show_skips_identical_frames_and_clear_resets() {
        let flushes = Arc::new(AtomicUsize::new(0));
        let mut d = EinkDisplay {
            backend: Box::new(CountingBackend { flushes: flushes.clone() }),
            width: 8,
            height: 8,
            threshold: 128,
            mode: EinkMode::Terminal,
            last_packed: None,
        };
        let black = RgbImage::new(8, 8);
        let lit = white(8, 8);

        d.show(&black); // first frame always flushes
        d.show(&black); // identical → skipped
        assert_eq!(flushes.load(Ordering::SeqCst), 1, "duplicate frame must not refresh");

        d.show(&lit); // changed → flush
        d.show(&lit); // identical → skipped
        assert_eq!(flushes.load(Ordering::SeqCst), 2, "only the change refreshes");

        d.clear(); // invalidates the cache
        d.show(&lit); // same pixels as before, but post-clear → must flush
        assert_eq!(flushes.load(Ordering::SeqCst), 3, "clear forces the next show to refresh");
    }

    #[test]
    fn dimensions_known_and_unknown() {
        assert_eq!(model_dimensions("4in2"), (400, 300));
        assert_eq!(model_dimensions("7in5_v2"), (800, 480));
        assert_eq!(model_dimensions("2in13"), (250, 122));
        // Unknown falls back rather than panicking.
        assert_eq!(model_dimensions("banana"), (400, 300));
    }

    #[test]
    fn mode_env_parsing() {
        // No env set in this test by default.
        std::env::set_var("OHMYOLED_EINK_MODE", "terminal");
        assert_eq!(mode_from_env(), Some(EinkMode::Terminal));
        std::env::set_var("OHMYOLED_EINK_MODE", "hw");
        assert_eq!(mode_from_env(), Some(EinkMode::Hardware));
        std::env::remove_var("OHMYOLED_EINK_MODE");
        assert_eq!(mode_from_env(), None);
    }

    #[test]
    fn test_constructor_is_terminal() {
        let d = EinkDisplay::test(EinkOptions::default());
        assert_eq!(d.mode, EinkMode::Terminal);
        // Default is the 7.5" panel.
        assert_eq!((d.width(), d.height()), (800, 480));
    }

    #[test]
    fn explicit_dimensions_override_model() {
        // Both set → override wins over the model's native size.
        let opts = EinkOptions {
            model: "4in2".into(),
            width: Some(640),
            height: Some(384),
            ..Default::default()
        };
        assert_eq!(opts.dimensions(), (640, 384));
        let d = EinkDisplay::test(opts);
        assert_eq!((d.width(), d.height()), (640, 384));
    }

    #[test]
    fn partial_dimensions_fall_back_to_model() {
        // Only one of width/height set → ignore, use the model.
        let opts = EinkOptions {
            model: "7in5_v2".into(),
            width: Some(640),
            height: None,
            ..Default::default()
        };
        assert_eq!(opts.dimensions(), (800, 480));
    }

    #[test]
    fn pack_black_image_is_blank_white() {
        // A black (background-only) source image inks nothing → all-white sheet.
        let img = RgbImage::new(16, 2);
        let packed = pack_1bpp(&img, 16, 2, 128);
        // 16px wide => 2 bytes/row, 2 rows => 4 bytes, all white => 0xFF.
        assert_eq!(packed, vec![0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn pack_bright_foreground_pixel_inks_msb_first() {
        // Black background, one bright pixel at x=0 → that pixel becomes ink,
        // clearing the MSB (0x80) of the row's first byte.
        let mut img = RgbImage::new(8, 1);
        img.put_pixel(0, 0, image::Rgb([255, 255, 255]));
        let packed = pack_1bpp(&img, 8, 1, 128);
        assert_eq!(packed, vec![0x7F]);
    }

    #[test]
    fn pack_stride_rounds_up_for_non_byte_multiple_width() {
        // 250px wide => ceil(250/8) = 32 bytes/row.
        let img = RgbImage::new(250, 122);
        let packed = pack_1bpp(&img, 250, 122, 128);
        assert_eq!(packed.len(), 32 * 122);
    }
}
