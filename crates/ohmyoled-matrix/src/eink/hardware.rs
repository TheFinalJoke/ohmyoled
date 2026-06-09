//! Hardware backend for [`super::EinkDisplay`] — drives a real Waveshare B/W
//! e-paper panel over SPI using pure-Rust `rppal` (GPIO/SPI) + `epd-waveshare`
//! (the panel protocol). No C dependency.
//!
//! Compiled only when the `eink` feature is enabled **and** we're building for
//! an ARM target (`rppal` is Pi-only). On any other target the dispatcher in
//! [`super::EinkDisplay::with_mode`] uses the terminal backend instead.
//!
//! # Pin mapping
//!
//! Uses the standard Waveshare e-Paper HAT wiring on SPI0:
//! `RST=GPIO17, DC=GPIO25, BUSY=GPIO24`, chip-select on `CE0` (driven by
//! `rppal` via `SlaveSelect::Ss0`). The `epd-waveshare` `SpiDevice` chip-select
//! is a no-op ([`NoCs`]) because `rppal` already asserts `CE0` per transfer.
//!
//! # On-hardware verification
//!
//! The exact pin map, SPI clock, and bit polarity are validated on a physical
//! panel — this is the "Pi (on hardware)" step in the plan. `init()` fails
//! cleanly (returning `Err`, falling back to terminal mode) if GPIO/SPI can't
//! be opened, so a misconfigured or absent HAT never hard-crashes the app.

use embedded_hal::digital::{ErrorType, OutputPin};
use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::{epd4in2::Epd4in2, epd7in5_v2::Epd7in5, prelude::*};
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

/// The concrete `epd-waveshare` driver for the selected panel. Each model is a
/// distinct type, so they're held in an enum and dispatched per call.
enum Panel {
    FourIn2(Epd4in2<EpdSpi, InputPin, Out, Out, Delay>),
    SevenIn5(Epd7in5<EpdSpi, InputPin, Out, Out, Delay>),
}

/// Hardware e-paper backend for the Waveshare 4.2" (400×300) and 7.5" V2
/// (800×480) B/W panels. Other models fall back to terminal mode via the
/// `Err` path in [`Self::init`].
pub struct EinkHardwareBackend {
    spi: EpdSpi,
    panel: Panel,
    delay: Delay,
}

impl EinkHardwareBackend {
    /// Try to initialise the panel. Returns `Err` (→ terminal fallback) when
    /// GPIO/SPI is unavailable or the model isn't wired up here yet.
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
        let mut spi = ExclusiveDevice::new(bus, NoCs, Delay)
            .map_err(|e| format!("spi device init failed: {e:?}"))?;

        let mut delay = Delay;
        // The panel constructors below perform a hardware reset and then block
        // polling the BUSY GPIO until the controller reports idle. A mis-wired
        // BUSY pin, wrong panel revision, or unpowered HAT makes that wait hang
        // indefinitely, so log *before* the call — if you see this line but not
        // the matching "initialised on SPI0" line, init is stuck on BUSY.
        log::info!("eink: opening {model} panel on SPI0 (reset + BUSY handshake)…");
        let panel = match model.as_str() {
            "4in2" | "4in2_v2" => {
                let epd = Epd4in2::new(&mut spi, busy, dc, rst, &mut delay, None)
                    .map_err(|e| format!("epd 4in2 init failed: {e:?}"))?;
                log::info!("eink: Waveshare 4in2 (400x300) initialised on SPI0");
                Panel::FourIn2(epd)
            }
            "7in5" | "7in5_v2" => {
                let epd = Epd7in5::new(&mut spi, busy, dc, rst, &mut delay, None)
                    .map_err(|e| format!("epd 7in5_v2 init failed: {e:?}"))?;
                log::info!("eink: Waveshare 7in5 V2 (800x480) initialised on SPI0");
                Panel::SevenIn5(epd)
            }
            other => {
                return Err(format!(
                    "eink hardware: model '{other}' not wired yet (4in2, 7in5_v2 supported)"
                ))
            }
        };

        Ok(Self { spi, panel, delay })
    }
}

impl EinkBackend for EinkHardwareBackend {
    fn flush(&mut self, packed: &[u8], _width: u32, _height: u32) {
        // Our packed buffer matches epd-waveshare's expected layout (MSB-first,
        // row-major, 1 = white), so it goes straight to the panel.
        let r = match &mut self.panel {
            Panel::FourIn2(epd) => epd.update_and_display_frame(&mut self.spi, packed, &mut self.delay),
            Panel::SevenIn5(epd) => epd.update_and_display_frame(&mut self.spi, packed, &mut self.delay),
        };
        if let Err(e) = r {
            log::error!("eink: frame update failed: {e:?}");
        }
    }

    fn clear(&mut self) {
        let r = match &mut self.panel {
            Panel::FourIn2(epd) => epd.clear_frame(&mut self.spi, &mut self.delay),
            Panel::SevenIn5(epd) => epd.clear_frame(&mut self.spi, &mut self.delay),
        };
        if let Err(e) = r {
            log::error!("eink: clear failed: {e:?}");
        }
    }
}
