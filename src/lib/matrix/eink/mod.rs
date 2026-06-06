//! E-paper renderers — the static, full-resolution counterparts to the LED
//! renderers in the parent module.
//!
//! Each submodule implements [`crate::matrix::eink_renderer::EinkRenderer`] for
//! one tile, composing a native-resolution `RgbImage` (white-foreground on
//! black, the usual convention) that [`ohmyoled_matrix::EinkDisplay`]
//! thresholds to black-ink-on-white and pushes to the panel.

pub mod aurora;
pub mod f1;
pub mod flights;
pub mod golf;
pub mod hass;
pub mod iss;
pub mod launch;
pub mod layout;
pub mod pihole;
pub mod quake;
pub mod sport;
pub mod stock;
pub mod stock_chart;
pub mod time;
pub mod weather;

pub use aurora::{EinkAuroraFonts, EinkAuroraMatrix};
pub use f1::{EinkF1Fonts, EinkF1Matrix};
pub use flights::{EinkFlightsFonts, EinkFlightsMatrix};
pub use golf::{EinkGolfFonts, EinkGolfMatrix};
pub use hass::{EinkHassFonts, EinkHassMatrix};
pub use iss::{EinkIssFonts, EinkIssMatrix};
pub use launch::{EinkLaunchFonts, EinkLaunchMatrix};
pub use pihole::{EinkPiholeFonts, EinkPiholeMatrix};
pub use quake::{EinkQuakeFonts, EinkQuakeMatrix};
pub use sport::{EinkSportFonts, EinkSportMatrix};
pub use stock::{EinkStockFonts, EinkStockMatrix};
pub use stock_chart::{EinkStockChartFonts, EinkStockChartMatrix};
pub use time::{EinkTimeFonts, EinkTimeMatrix};
pub use weather::{EinkWeatherFonts, EinkWeatherMatrix};
