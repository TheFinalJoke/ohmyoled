//! Module-level errors — wraps collector + renderer failures.

use crate::api::error::ApiError;
use crate::matrix::error::RenderError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error(transparent)]
    Render(#[from] RenderError),

    #[error("config: {0}")]
    Config(String),
}
