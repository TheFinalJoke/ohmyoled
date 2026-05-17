//! Graphics utilities: font loading (BDF + TTF) and 2D drawing primitives.
//!
//! Use `Font::new()` + `Font::load_font(path)` to load a BDF font, or
//! `Font::load_ttf(path, px)` for a TrueType/OpenType font, then call
//! `draw_text` to render text onto an `image::RgbImage`.

pub mod bdf;
pub mod draw;
pub mod font;
pub mod ttf;

pub use draw::{draw_circle, draw_line, draw_text};
pub use font::Font;
