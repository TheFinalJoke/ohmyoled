//! Hardware backend for [`super::EinkDisplay`] — drives a real Waveshare B/W
//! e-paper panel over SPI using pure-Rust `rppal` (GPIO/SPI). No C dependency.
//!
//! The 4.2" panel is driven via the `epd-waveshare` crate. The 7.5" V2 panel is
//! driven by a **hand-rolled** controller sequence ([`SevenIn5V2`]) rather than
//! `epd-waveshare`, because that crate's `epd7in5_v2` power-on bytes are wrong
//! for the B/W panel (`PowerSetting 0x07,0x17,0x3F,0x3F` + a stray `PllControl`,
//! vs the panel's required `0x07,0x07,0x28,0x17` with no PLL). With the wrong
//! values `PowerOn` never completes, BUSY never releases, and init hangs forever
//! (caemor/epd-waveshare#70). We instead replicate the official `epd7in5_V2.py`
//! B/W sequence byte-for-byte, which the panel acknowledges correctly.
//!
//! Compiled only when the `eink` feature is enabled **and** we're building for
//! an ARM target (`rppal` is Pi-only). On any other target the dispatcher in
//! [`super::EinkDisplay::with_mode`] uses the terminal backend instead.
//!
//! # Pin mapping
//!
//! Uses the standard Waveshare e-Paper HAT wiring on SPI0:
//! `RST=GPIO17, DC=GPIO25, BUSY=GPIO24, PWR=GPIO18`, chip-select on `CE0`
//! (driven by `rppal` via `SlaveSelect::Ss0`). `PWR` is the rev2.2+ HAT's
//! power-enable and must be driven HIGH or the panel never powers on. The HAT's
//! Interface Config switch must be **0 (4-line SPI)** and Display Config **A**.
//!
//! # On-hardware verification
//!
//! The exact pin map, SPI clock, init sequence, and BUSY polarity are validated
//! on a physical panel. `init()` fails cleanly (returning `Err`, falling back to
//! terminal mode) if GPIO/SPI can't be opened, and the caller bounds the whole
//! init with a timeout so a mis-wired HAT degrades instead of hanging.

use std::thread::sleep;
use std::time::{Duration, Instant};

use embedded_hal::digital::{ErrorType, OutputPin};
use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::{epd4in2::Epd4in2, prelude::*};
use rppal::gpio::{Gpio, InputPin, OutputPin as Out};
use rppal::hal::Delay;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

use super::{EinkBackend, EinkOptions};

/// SPI clock for the e-paper bus. 4 MHz is comfortably within spec for the
/// Waveshare controllers and reliable over the HAT's short traces.
const SPI_CLOCK_HZ: u32 = 4_000_000;

// Standard Waveshare e-Paper HAT BCM pin numbers.
const PIN_RST: u8 = 17;
const PIN_DC: u8 = 25;
const PIN_BUSY: u8 = 24;
/// Power-enable pin on the rev2.2+ e-Paper Driver HAT. `epdconfig.py`'s
/// `module_init()` drives this HIGH to power the panel; without it the
/// controller never powers on and BUSY never releases. Held high for the
/// backend's lifetime (rppal resets pins to input on drop, cutting power).
const PIN_PWR: u8 = 18;

/// Upper bound on a single BUSY wait. A full 7.5" refresh takes a few seconds;
/// 30 s leaves generous headroom while still bailing out (rather than spinning
/// forever) if the panel never releases BUSY.
const BUSY_TIMEOUT: Duration = Duration::from_secs(30);

/// Max bytes per SPI transfer. The kernel spidev buffer defaults to 4096, so
/// frame data (48 000 bytes for the 7.5") is chunked to stay within it — the
/// controller latches data across chunks, matching Waveshare's `send_data2`.
const SPI_CHUNK: usize = 4096;

/// No-op chip-select for `embedded-hal-bus`: `rppal` drives the real `CE0`
/// line per transfer, so the `SpiDevice` abstraction's CS toggling is inert.
struct NoCs;
impl ErrorType for NoCs {
    type Error = core::convert::Infallible;
}
impl OutputPin for NoCs {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

type EpdSpi = ExclusiveDevice<Spi, NoCs, Delay>;

/// Hand-rolled driver for the Waveshare 7.5" V2 **B/W** panel (800×480).
///
/// Talks to the controller directly over raw `rppal` SPI + GPIO, replicating
/// the official `epd7in5_V2.py` command sequence. `rppal` asserts `CE0` per
/// `write()`, so chip-select is handled implicitly; `DC` selects command (low)
/// vs data (high).
///
/// Updates use the panel's **fast/partial** refresh ([`init_part`] +
/// [`display_partial`]) so screen changes are quick and never do the slow,
/// flashy full-screen refresh. Each [`update`] clears to white then draws the
/// new frame — both partial refreshes — so the previous frame's ghost is
/// scrubbed without a full refresh.
struct SevenIn5V2 {
    spi: Spi,
    rst: Out,
    dc: Out,
    busy: InputPin,
    /// Whether the controller is currently in partial-update mode (`init_part`).
    /// [`clear`] re-enters full mode, so the next update re-arms partial mode.
    in_partial_mode: bool,
}

impl SevenIn5V2 {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 480;
    /// Frame buffer size: 1bpp, stride = ceil(width/8).
    const FRAME_BYTES: usize = (Self::WIDTH as usize / 8) * Self::HEIGHT as usize;

    fn new(spi: Spi, rst: Out, dc: Out, busy: InputPin) -> Result<Self, String> {
        let mut p = Self {
            spi,
            rst,
            dc,
            busy,
            in_partial_mode: false,
        };
        // Full power-on setup (resolution, VCOM, TCON). Updates then switch to
        // partial mode; the first update's clear scrubs any retained image.
        p.init()?;
        Ok(p)
    }

    /// Send a single command byte (DC low).
    fn cmd(&mut self, c: u8) -> Result<(), String> {
        self.dc.set_low();
        self.spi.write(&[c]).map_err(|e| format!("spi cmd {c:#04x}: {e}"))?;
        Ok(())
    }

    /// Send data bytes (DC high), chunked to the spidev buffer size.
    fn data(&mut self, d: &[u8]) -> Result<(), String> {
        self.dc.set_high();
        for chunk in d.chunks(SPI_CHUNK) {
            self.spi.write(chunk).map_err(|e| format!("spi data: {e}"))?;
        }
        Ok(())
    }

    fn cmd_data(&mut self, c: u8, d: &[u8]) -> Result<(), String> {
        self.cmd(c)?;
        self.data(d)
    }

    /// Hardware reset: high → low (2 ms) → high, matching the V2 timing.
    fn reset(&mut self) {
        self.rst.set_high();
        sleep(Duration::from_millis(20));
        self.rst.set_low();
        sleep(Duration::from_millis(2));
        self.rst.set_high();
        sleep(Duration::from_millis(20));
    }

    /// Block until the controller reports idle. The 7.5" V2 only updates BUSY
    /// after a `GetStatus` (0x71) command, and reads **HIGH = idle, LOW = busy**.
    /// Bounded by [`BUSY_TIMEOUT`] so a stuck line returns an error.
    fn wait_until_idle(&mut self) -> Result<(), String> {
        let start = Instant::now();
        loop {
            self.cmd(0x71)?; // GetStatus — required for BUSY to refresh
            if self.busy.is_high() {
                break;
            }
            if start.elapsed() >= BUSY_TIMEOUT {
                return Err(format!(
                    "7in5_v2 BUSY stuck low for {}s",
                    BUSY_TIMEOUT.as_secs()
                ));
            }
            sleep(Duration::from_millis(20));
        }
        sleep(Duration::from_millis(20));
        Ok(())
    }

    /// Power-on + panel configuration for the B/W 7.5" V2 — the exact byte
    /// sequence from `waveshareteam/e-Paper` `epd7in5_V2.py` `init()`. The
    /// `BoosterSoftStart` (0x06) and the `0x28,0x17` `PowerSetting` tail are
    /// load-bearing: without them PowerOn never completes and BUSY hangs.
    fn init(&mut self) -> Result<(), String> {
        self.reset();
        self.cmd_data(0x06, &[0x17, 0x17, 0x28, 0x17])?; // Booster soft start
        self.cmd_data(0x01, &[0x07, 0x07, 0x28, 0x17])?; // Power setting
        self.cmd(0x04)?; // Power on
        sleep(Duration::from_millis(100));
        self.wait_until_idle()?;
        self.cmd_data(0x00, &[0x1F])?; // Panel setting (KW, no rotate)
        self.cmd_data(0x61, &[0x03, 0x20, 0x01, 0xE0])?; // TRES: 800×480
        self.cmd_data(0x15, &[0x00])?; // Dual SPI off
        self.cmd_data(0x50, &[0x10, 0x07])?; // VCOM + data interval
        self.cmd_data(0x60, &[0x22])?; // TCON
        Ok(())
    }

    /// Lighter init for fast/partial updates — `epd7in5_V2.py` `init_part()`.
    /// Powers on and loads the fast-update registers (`0xE0`, `0xE5`) instead of
    /// the full booster/TRES/TCON setup. Required before [`display_partial`].
    fn init_part(&mut self) -> Result<(), String> {
        self.reset();
        self.cmd_data(0x00, &[0x1F])?; // Panel setting
        self.cmd(0x04)?; // Power on
        sleep(Duration::from_millis(100));
        self.wait_until_idle()?;
        self.cmd_data(0xE0, &[0x02])?; // Cascade setting (fast mode)
        self.cmd_data(0xE5, &[0x6E])?; // Force temperature (fast mode)
        Ok(())
    }

    /// Full, ghost-free refresh. Our buffer is 1 = white, but the panel's
    /// new-frame buffer (0x13) is 0 = white (see `clear`: writing 0x00 yields
    /// white), so 0x13 gets the bitwise inverse. The previous-frame buffer
    /// (0x10) gets `packed` — the inverse of 0x13 — so every pixel transitions.
    fn display_full(&mut self, packed: &[u8]) -> Result<(), String> {
        let inverse: Vec<u8> = packed.iter().map(|b| !b).collect();
        self.cmd_data(0x10, packed)?; // previous frame
        self.cmd_data(0x13, &inverse)?; // new frame (0 = white)
        self.cmd(0x12)?; // display refresh
        sleep(Duration::from_millis(100));
        self.wait_until_idle()
    }

    /// Fast/partial full-screen refresh — `epd7in5_V2.py` `display_Partial()`
    /// with the window set to the whole panel. Quick and flash-free; only `0x13`
    /// is written, no `0x10`.
    ///
    /// **Polarity:** partial mode interprets the `0x13` buffer with the *opposite*
    /// polarity to the full refresh, so — unlike [`display_full`], which inverts —
    /// `packed` is sent **as-is** here. (Mirrors the reference, where full
    /// `display()` writes `0x13 ← image` but `display_Partial()` writes
    /// `0x13 ← ~image`; our `packed` is already the inverse of that `image`, so
    /// the two flips cancel and a plain `packed` is correct.)
    fn display_partial(&mut self, packed: &[u8]) -> Result<(), String> {
        let (xs, xe, ys, ye) = (0u32, Self::WIDTH, 0u32, Self::HEIGHT);
        self.cmd_data(0x50, &[0xA9, 0x07])?; // VCOM/data interval (partial)
        self.cmd(0x91)?; // enter partial mode
        self.cmd_data(
            0x90, // resolution/window: x_start, x_end-1, y_start, y_end-1, 0x01
            &[
                (xs >> 8) as u8,
                (xs & 0xFF) as u8,
                ((xe - 1) >> 8) as u8,
                ((xe - 1) & 0xFF) as u8,
                (ys >> 8) as u8,
                (ys & 0xFF) as u8,
                ((ye - 1) >> 8) as u8,
                ((ye - 1) & 0xFF) as u8,
                0x01,
            ],
        )?;
        self.cmd_data(0x13, packed)?; // new frame (partial-mode polarity)
        self.cmd(0x12)?; // display refresh
        sleep(Duration::from_millis(100));
        self.wait_until_idle()
    }

    /// Push one packed frame: clear to white, then draw it — both fast partial
    /// refreshes. The white clear scrubs the previous frame's ghost without the
    /// slow full-screen refresh.
    fn update(&mut self, packed: &[u8]) -> Result<(), String> {
        if !self.in_partial_mode {
            self.init_part()?;
            self.in_partial_mode = true;
        }
        let white = vec![0xFF; Self::FRAME_BYTES];
        self.display_partial(&white)?;
        self.display_partial(packed)?;
        Ok(())
    }

    /// Push one packed frame with a single partial refresh and **no** clear —
    /// for frequent updates (a ticking clock). The caller recomposes the whole
    /// frame each time, so the partial refresh drives toward the new image
    /// (the old frame's pixels are already gone) without a cumulative trail.
    fn update_fast(&mut self, packed: &[u8]) -> Result<(), String> {
        if !self.in_partial_mode {
            self.init_part()?;
            self.in_partial_mode = true;
        }
        self.display_partial(packed)
    }

    /// Blank the panel to white with a clean full refresh. Restores full mode
    /// first (partial mode leaves fast-update registers set). An all-white
    /// frame through `display_full` is `epd7in5_V2.py` `Clear()` (0x10←0xFF,
    /// 0x13←0x00). Used on shutdown for a ghost-free blank.
    fn clear(&mut self) -> Result<(), String> {
        self.init()?;
        self.in_partial_mode = false;
        let white = vec![0xFF; Self::FRAME_BYTES];
        self.display_full(&white)
    }
}

/// The active panel: 4.2" via `epd-waveshare`, 7.5" V2 via the hand-rolled
/// [`SevenIn5V2`] driver. The 4.2" variant carries the SPI device + delay it
/// needs for each call alongside the driver.
enum Panel {
    FourIn2 {
        spi: EpdSpi,
        epd: Epd4in2<EpdSpi, InputPin, Out, Out, Delay>,
        delay: Delay,
    },
    SevenIn5V2(SevenIn5V2),
}

/// Hardware e-paper backend for the Waveshare 4.2" (400×300) and 7.5" V2
/// (800×480) B/W panels. Other models fall back to terminal mode via the
/// `Err` path in [`Self::init`].
pub struct EinkHardwareBackend {
    panel: Panel,
    /// Power-enable pin (GPIO18), held HIGH for the backend's lifetime. Kept as
    /// a field purely to own the pin — dropping it would reset GPIO18 and cut
    /// panel power. Underscore-prefixed because it's never read after init.
    _pwr: Out,
}

impl EinkHardwareBackend {
    /// Try to initialise the panel. Returns `Err` (→ terminal fallback) when
    /// GPIO/SPI is unavailable, BUSY never releases, or the model isn't wired up.
    pub fn init(options: &EinkOptions) -> Result<Self, String> {
        let model = options.model.to_lowercase().replace(['-', '.', ' '], "_");

        let gpio = Gpio::new().map_err(|e| format!("gpio open failed: {e}"))?;
        // Power the panel first: the rev2.2+ HAT gates panel power through
        // GPIO18, and the controller won't respond (BUSY stays low) until it's
        // high. Mirrors epdconfig.py module_init(). Held for the lifetime below.
        let mut pwr = gpio
            .get(PIN_PWR)
            .map_err(|e| format!("gpio {PIN_PWR} (PWR): {e}"))?
            .into_output();
        pwr.set_high();
        sleep(Duration::from_millis(10));
        let rst = gpio
            .get(PIN_RST)
            .map_err(|e| format!("gpio {PIN_RST} (RST): {e}"))?
            .into_output();
        let dc = gpio
            .get(PIN_DC)
            .map_err(|e| format!("gpio {PIN_DC} (DC): {e}"))?
            .into_output();
        let busy = gpio
            .get(PIN_BUSY)
            .map_err(|e| format!("gpio {PIN_BUSY} (BUSY): {e}"))?
            .into_input();

        let bus = Spi::new(Bus::Spi0, SlaveSelect::Ss0, SPI_CLOCK_HZ, Mode::Mode0)
            .map_err(|e| format!("spi open failed: {e}"))?;

        // Both init paths reset the panel and block on the BUSY handshake. A
        // mis-wired BUSY pin, wrong panel revision, or unpowered HAT makes that
        // hang, so log *before* the call — if you see this line but not the
        // matching "initialised on SPI0" line, init is stuck on BUSY.
        log::info!("eink: opening {model} panel on SPI0 (reset + BUSY handshake)…");
        let panel = match model.as_str() {
            "4in2" | "4in2_v2" => {
                let mut spi = ExclusiveDevice::new(bus, NoCs, Delay)
                    .map_err(|e| format!("spi device init failed: {e:?}"))?;
                let mut delay = Delay;
                let epd = Epd4in2::new(&mut spi, busy, dc, rst, &mut delay, None)
                    .map_err(|e| format!("epd 4in2 init failed: {e:?}"))?;
                log::info!("eink: Waveshare 4in2 (400x300) initialised on SPI0");
                Panel::FourIn2 { spi, epd, delay }
            }
            "7in5" | "7in5_v2" => {
                let p = SevenIn5V2::new(bus, rst, dc, busy)?;
                log::info!("eink: Waveshare 7in5 V2 (800x480) initialised on SPI0");
                Panel::SevenIn5V2(p)
            }
            other => {
                return Err(format!(
                    "eink hardware: model '{other}' not wired yet (4in2, 7in5_v2 supported)"
                ))
            }
        };

        Ok(Self { panel, _pwr: pwr })
    }
}

impl EinkBackend for EinkHardwareBackend {
    fn flush(&mut self, packed: &[u8], _width: u32, _height: u32) {
        // Our packed buffer matches the panel's expected layout (MSB-first,
        // row-major, 1 = white), so it goes straight to the controller.
        let r = match &mut self.panel {
            Panel::FourIn2 { spi, epd, delay } => epd
                .update_and_display_frame(spi, packed, delay)
                .map_err(|e| format!("{e:?}")),
            Panel::SevenIn5V2(p) => p.update(packed),
        };
        if let Err(e) = r {
            log::error!("eink: frame update failed: {e}");
        }
    }

    fn flush_fast(&mut self, packed: &[u8], _width: u32, _height: u32) {
        let r = match &mut self.panel {
            // The 4.2" path has no distinct fast refresh — do a normal update.
            Panel::FourIn2 { spi, epd, delay } => epd
                .update_and_display_frame(spi, packed, delay)
                .map_err(|e| format!("{e:?}")),
            Panel::SevenIn5V2(p) => p.update_fast(packed),
        };
        if let Err(e) = r {
            log::error!("eink: fast frame update failed: {e}");
        }
    }

    fn clear(&mut self) {
        let r = match &mut self.panel {
            Panel::FourIn2 { spi, epd, delay } => {
                epd.clear_frame(spi, delay).map_err(|e| format!("{e:?}"))
            }
            Panel::SevenIn5V2(p) => p.clear(),
        };
        if let Err(e) = r {
            log::error!("eink: clear failed: {e}");
        }
    }
}
