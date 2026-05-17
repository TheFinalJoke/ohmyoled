//! Matrix renderers and the trait every new renderer implements.
//!
//! See [`renderer::Renderer`] for the contract and [`error::RenderError`] for
//! the error type. Each submodule (e.g. [`time`]) implements one renderer.

pub mod error;
pub mod f1;
pub mod golf;
pub mod renderer;
pub mod sport;
pub mod stock;
pub mod time;
pub mod weather;

pub use error::RenderError;
pub use f1::F1Matrix;
pub use golf::GolfMatrix;
pub use renderer::Renderer;
pub use sport::SportMatrix;
pub use stock::StockMatrix;
pub use time::{TimeFormat, TimeMatrix};
pub use weather::WeatherMatrix;
