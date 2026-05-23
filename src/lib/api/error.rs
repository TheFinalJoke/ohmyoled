//! Errors raised by API collectors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("decode: {0}")]
    Decode(#[from] serde_json::Error),

    #[error("config: {0}")]
    Config(String),

    #[error("provider {provider}: {msg}")]
    Provider {
        provider: &'static str,
        msg: String,
    },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
