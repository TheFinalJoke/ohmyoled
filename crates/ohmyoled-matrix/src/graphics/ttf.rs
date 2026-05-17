//! TrueType / OpenType font support via `ab_glyph`.
//!
//! Loaded once at a chosen pixel size; reused across many `draw_text` calls.
//! Glyphs are rasterized on demand (no cache yet — the 64×32 panel doesn't justify it).

use ab_glyph::{Font as _, FontVec, PxScale, ScaleFont};
use std::path::Path;

/// A loaded TrueType / OpenType font at a fixed pixel size.
pub struct TtfFont {
    pub(crate) font: FontVec,
    pub(crate) scale: PxScale,
    pub(crate) path: String,
}

impl std::fmt::Debug for TtfFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TtfFont")
            .field("path", &self.path)
            .field("scale_px", &self.scale.y)
            .finish()
    }
}

impl Clone for TtfFont {
    fn clone(&self) -> Self {
        // FontVec doesn't implement Clone; reload from path.
        Self::load(&self.path, self.scale.y).expect("font path no longer readable")
    }
}

impl TtfFont {
    /// Load a TTF/OTF font from disk at the given pixel size (font height).
    pub fn load<P: AsRef<Path>>(path: P, px: f32) -> Result<Self, String> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| format!("cannot read font '{path:?}': {e}"))?;
        let font = FontVec::try_from_vec(bytes)
            .map_err(|e| format!("invalid font '{path:?}': {e}"))?;
        Ok(Self {
            font,
            scale: PxScale::from(px),
            path: path.to_string_lossy().into_owned(),
        })
    }

    /// Pixel-space ascent (distance from baseline to top).
    pub fn ascent(&self) -> i32 {
        self.font.as_scaled(self.scale).ascent().round() as i32
    }

    /// Pixel-space descent (positive number — distance from baseline down).
    pub fn descent(&self) -> i32 {
        self.font.as_scaled(self.scale).descent().abs().round() as i32
    }

    /// Total line height in pixels.
    pub fn height(&self) -> i32 {
        self.ascent() + self.descent()
    }
}
