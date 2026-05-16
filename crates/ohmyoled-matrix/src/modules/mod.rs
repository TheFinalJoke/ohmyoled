//! Built-in display modules.
//!
//! Each submodule provides a self-contained renderer that pushes images to an
//! `RGBMatrix`. These were originally Python `MatrixBase` subclasses; they now
//! live in Rust so the rendering path stays out of PIL.

pub mod time;

pub use time::{TimeFormat, TimeMatrix};