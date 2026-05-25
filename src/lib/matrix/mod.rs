//! Matrix renderers and the trait every new renderer implements.
//!
//! See [`renderer::Renderer`] for the contract and [`error::RenderError`] for
//! the error type. Each submodule (e.g. [`time`]) implements one renderer.

pub mod aurora;
pub mod cells;
pub mod error;
pub mod f1;
pub mod flights;
pub mod golf;
pub mod hass;
pub mod iss;
pub mod launch;
pub mod pihole;
pub mod quake;
pub mod renderer;
pub mod sport;
pub mod stock;
pub mod time;
pub mod weather;

pub use aurora::AuroraMatrix;
pub use error::RenderError;
pub use f1::F1Matrix;
pub use flights::FlightsMatrix;
pub use hass::HassMatrix;
pub use launch::LaunchMatrix;
pub use pihole::PiholeMatrix;
pub use golf::GolfMatrix;
pub use iss::IssMatrix;
pub use quake::QuakeMatrix;
pub use renderer::Renderer;
pub use sport::SportMatrix;
pub use stock::StockMatrix;
pub use time::{TimeFormat, TimeMatrix};
pub use weather::WeatherMatrix;
