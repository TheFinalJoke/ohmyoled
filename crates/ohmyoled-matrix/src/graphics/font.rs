//! `Font` — loadable font for `draw_text`.
//!
//! Two variants:
//! - **BDF** (legacy): bitmap fonts, used by the time module.
//! - **TTF** (new): TrueType/OpenType via `ab_glyph`, for the weather/stock/sport renderers that
//!   were on PIL+freetype.
//!
//! Existing BDF call sites work unchanged via the `Font::new() + load_font(path)` shim.

use super::bdf::BdfFont;
use super::ttf::TtfFont;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct BdfHandle {
    pub(crate) inner: Option<BdfFont>,
    #[allow(dead_code)]
    pub(crate) path: String,
}

#[derive(Debug)]
pub enum Font {
    Bdf(BdfHandle),
    Ttf(TtfFont),
}

impl Clone for Font {
    fn clone(&self) -> Self {
        match self {
            Self::Bdf(b) => Self::Bdf(b.clone()),
            Self::Ttf(t) => Self::Ttf(t.clone()),
        }
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::Bdf(BdfHandle::default())
    }
}

impl Font {
    /// Create an empty BDF font handle. Call `load_font` to populate it.
    ///
    /// This is the legacy entry point preserved for `TimeMatrix`-style call sites:
    /// ```no_run
    /// use ohmyoled_matrix::graphics::Font;
    /// let mut font = Font::new();
    /// font.load_font("/etc/ohmyoled/fonts/4x6.bdf").unwrap();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a BDF font from disk into a `Font::new()` handle.
    pub fn load_font<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)
            .map_err(|e| format!("cannot open font '{path:?}': {e}"))?;
        let bdf = BdfFont::from_reader(file)?;
        *self = Self::Bdf(BdfHandle {
            inner: Some(bdf),
            path: path.to_string_lossy().into_owned(),
        });
        Ok(())
    }

    /// Load a BDF font as a fresh `Font` (preferred for new code).
    pub fn load_bdf<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut f = Self::new();
        f.load_font(path)?;
        Ok(f)
    }

    /// Load a TTF or OTF font at the given pixel size.
    pub fn load_ttf<P: AsRef<Path>>(path: P, px: f32) -> Result<Self, String> {
        Ok(Self::Ttf(TtfFont::load(path, px)?))
    }

    /// Total line height in pixels (ascent + descent). Returns 0 if BDF and unloaded.
    pub fn height(&self) -> i32 {
        match self {
            Self::Bdf(b) => b.inner.as_ref().map(|f| f.ascent + f.descent).unwrap_or(0),
            Self::Ttf(t) => t.height(),
        }
    }

    /// Baseline ascent — pixels above the baseline.
    pub fn ascent(&self) -> i32 {
        match self {
            Self::Bdf(b) => b.inner.as_ref().map(|f| f.ascent).unwrap_or(0),
            Self::Ttf(t) => t.ascent(),
        }
    }

    pub(crate) fn bdf(&self) -> Option<&BdfFont> {
        match self {
            Self::Bdf(b) => b.inner.as_ref(),
            Self::Ttf(_) => None,
        }
    }

    pub(crate) fn ttf(&self) -> Option<&TtfFont> {
        match self {
            Self::Ttf(t) => Some(t),
            Self::Bdf(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_ttf() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("..");
        p.push("..");
        p.push("fonts");
        p.push("04B_03B_.TTF");
        p
    }

    #[test]
    fn ttf_loads_at_size() {
        let font = Font::load_ttf(repo_ttf(), 8.0).expect("ttf load");
        assert!(font.ascent() > 0);
        assert!(font.height() > font.ascent());
    }

    #[test]
    fn ttf_render_lights_pixels() {
        use crate::graphics::draw_text;
        use crate::Color;
        let font = Font::load_ttf(repo_ttf(), 8.0).expect("ttf load");
        let mut img = image::RgbImage::new(64, 32);
        draw_text(&mut img, &font, 2, 8, Color::WHITE, "Hi");
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 0, "expected some non-black pixels");
    }

    #[test]
    fn missing_ttf_returns_err() {
        assert!(Font::load_ttf("/nonexistent/font.ttf", 10.0).is_err());
    }
}
