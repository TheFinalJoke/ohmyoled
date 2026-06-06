//! E-paper renderers — the static, full-resolution counterparts to the LED
//! renderers in the parent module.
//!
//! Each submodule implements [`crate::matrix::eink_renderer::EinkRenderer`] for
//! one tile, composing a native-resolution `RgbImage` (white-foreground on
//! black, the usual convention) that [`ohmyoled_matrix::EinkDisplay`]
//! thresholds to black-ink-on-white and pushes to the panel.

pub mod aurora;
pub mod iss;
pub mod layout;
pub mod pihole;
pub mod quake;
pub mod time;
pub mod weather;

pub use aurora::{EinkAuroraFonts, EinkAuroraMatrix};
pub use iss::{EinkIssFonts, EinkIssMatrix};
pub use pihole::{EinkPiholeFonts, EinkPiholeMatrix};
pub use quake::{EinkQuakeFonts, EinkQuakeMatrix};
pub use time::{EinkTimeFonts, EinkTimeMatrix};
pub use weather::{EinkWeatherFonts, EinkWeatherMatrix};
