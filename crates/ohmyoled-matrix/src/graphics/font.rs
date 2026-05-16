//! `Font` — loadable BDF font with a `DrawText`-compatible interface.
//!
//! Matches the shape of `rgbmatrix.graphics.Font` used by `Matrix.get_font_graphics`
//! in `matrix.py`: create a `Font`, call `load_font(path)`, then pass it to `draw_text`.

use super::bdf::BdfFont;
use std::path::Path;

/// A loaded BDF font ready for drawing.
///
/// # Example
/// ```no_run
/// use ohmyoled_matrix::graphics::Font;
/// let mut font = Font::new();
/// font.load_font("/etc/ohmyoled/fonts/4x6.bdf").unwrap();
/// ```
#[derive(Debug, Clone, Default)]
pub struct Font {
    pub(crate) inner: Option<BdfFont>,
    pub(crate) path: String,
}

impl Font {
    /// Create an empty (not yet loaded) font handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a BDF font from disk.
    ///
    /// Replaces any previously loaded font in this handle.
    pub fn load_font<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)
            .map_err(|e| format!("cannot open font '{path:?}': {e}"))?;
        self.inner = Some(BdfFont::from_reader(file)?);
        self.path = path.to_string_lossy().into_owned();
        Ok(())
    }

    /// Height of the font in pixels (ascent + descent).
    ///
    /// Returns 0 if no font has been loaded.
    pub fn height(&self) -> i32 {
        self.inner.as_ref().map(|f| f.ascent + f.descent).unwrap_or(0)
    }

    /// Baseline ascent — pixels above the baseline.
    pub fn ascent(&self) -> i32 {
        self.inner.as_ref().map(|f| f.ascent).unwrap_or(0)
    }

    pub(crate) fn bdf(&self) -> Option<&BdfFont> {
        self.inner.as_ref()
    }
}
