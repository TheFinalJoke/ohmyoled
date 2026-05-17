//! Errors raised by matrix renderers.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("font: {0}")]
    Font(String),

    #[error("image: {0}")]
    Image(String),

    #[error("backend: {0}")]
    Backend(String),
}
