//! Graphics utilities: BDF font loading and 2D drawing primitives.
//!
//! Use `Font::new()` + `Font::load_font(path)` to load a BDF font, then call
//! `draw_text` to render text onto an `image::RgbImage`.

pub mod bdf;
pub mod draw;
pub mod font;

pub use draw::{draw_circle, draw_line, draw_text};
pub use font::Font;
