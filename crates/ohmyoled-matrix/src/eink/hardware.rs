//! Hardware backend for [`super::EinkDisplay`] — drives a real Waveshare B/W
//! e-paper panel over SPI using pure-Rust `rppal` (GPIO/SPI). No C dependency.
//!
//! The 4.2" panel is driven via the `epd-waveshare` crate. The 7.5" V2 panel is
//! driven by a **hand-rolled** controller sequence ([`SevenIn5V2`]) rather than
//! `epd-waveshare`, because that crate's `epd7in5_v2` init is modeled on the
//! 3-color B/C panel (it sends a `BoosterSoftStart`, a `0x17` `PowerSetting`
//! byte, and a `PllControl` the B/W panel doesn't take). On a B/W 7.5" V2 that
//! sequence never completes `PowerOn`, so BUSY never releases and init hangs
//! forever. We instead replicate the official `epd7in5_V2.py` B/W sequence,
//! which the panel acknowledges correctly. See caemor/epd-waveshare#70.
//!
//! Compiled only when the `eink` feature is enabled **and** we're building for
//! an ARM target (`rppal` is Pi-only). On any other target the dispatcher in
//! [`super::EinkDisplay::with_mode`] uses the terminal backend instead.
//!
//! # Pin mapping
//!
//! Uses the standard Waveshare e-Paper HAT wiring on SPI0:
//! `RST=GPIO17, DC=GPIO25, BUSY=GPIO24`, chip-select on `CE0` (driven by
//! `rppal` via `SlaveSelect::Ss0`). The HAT's Interface Config switch must be
//! **0 (4-line SPI)** and Display Config **A**.
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
struct SevenIn5V2 {
    spi: Spi,
    rst: Out,
    dc: Out,
    busy: InputPin,
}

impl SevenIn5V2 {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 480;
    /// Frame buffer size: 1bpp, stride = ceil(width/8).
    const FRAME_BYTES: usize = (Self::WIDTH as usize / 8) * Self::HEIGHT as usize;

    fn new(spi: Spi, rst: Out, dc: Out, busy: InputPin) -> Result<Self, String> {
        let mut p = Self { spi, rst, dc, busy };
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

    /// Power-on + panel configuration for the B/W 7.5" V2 — the exact sequence
    /// from `waveshareteam/e-Paper` `epd7in5_V2.py`. Deliberately omits the
    /// 3-color `BoosterSoftStart`/`PllControl` that hang this panel.
    fn init(&mut self) -> Result<(), String> {
        self.reset();
        self.cmd_data(0x01, &[0x07, 0x07, 0x3F, 0x3F])?; // POWER SETTING
        self.cmd(0x04)?; // POWER ON
        sleep(Duration::from_millis(100));
        self.wait_until_idle()?;
        self.cmd_data(0x00, &[0x1F])?; // PANEL SETTING (KW, no rotate)
        self.cmd_data(0x61, &[0x03, 0x20, 0x01, 0xE0])?; // TRES: 800×480
        self.cmd_data(0x15, &[0x00])?; // DUAL SPI off
        self.cmd_data(0x50, &[0x10, 0x07])?; // VCOM + data interval
        self.cmd_data(0x60, &[0x22])?; // TCON
        Ok(())
    }

    /// Push one packed frame (1bpp, MSB-first, 1 = white) and refresh.
    fn display(&mut self, packed: &[u8]) -> Result<(), String> {
        self.cmd_data(0x13, packed)?; // DATA_START_TRANSMISSION_2 (new frame)
        self.cmd(0x12)?; // DISPLAY_REFRESH
        sleep(Duration::from_millis(100));
        self.wait_until_idle()
    }

    /// Blank the panel to white (all 1 bits).
    fn clear(&mut self) -> Result<(), String> {
        let white = vec![0xFF; Self::FRAME_BYTES];
        self.display(&white)
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
}

impl EinkHardwareBackend {
    /// Try to initialise the panel. Returns `Err` (→ terminal fallback) when
    /// GPIO/SPI is unavailable, BUSY never releases, or the model isn't wired up.
    pub fn init(options: &EinkOptions) -> Result<Self, String> {
        let model = options.model.to_lowercase().replace(['-', '.', ' '], "_");

        let gpio = Gpio::new().map_err(|e| format!("gpio open failed: {e}"))?;
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

        Ok(Self { panel })
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
            Panel::SevenIn5V2(p) => p.display(packed),
        };
        if let Err(e) = r {
            log::error!("eink: frame update failed: {e}");
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
