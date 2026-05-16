//! BDF (Bitmap Distribution Format) font file parser.
//!
//! Parses the subset of BDF 2.1 that the OhMyOled fonts use:
//! STARTFONT → FONT_ASCENT/DESCENT → STARTCHAR → BBX/BITMAP/ENDCHAR.
//! Ignores properties we don't need (SWIDTH, DWIDTH metadata, comments).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

/// A single glyph extracted from a BDF file.
#[derive(Debug, Clone)]
pub struct BdfGlyph {
    /// Codepoint this glyph represents.
    pub codepoint: u32,
    /// Width and height of the glyph bitmap in pixels.
    pub width: i32,
    pub height: i32,
    /// Horizontal and vertical offset from the pen position.
    pub offset_x: i32,
    pub offset_y: i32,
    /// Device-width advance (how far to move the pen after drawing).
    pub dwidth: i32,
    /// Row bitmaps: each `u32` holds one row, MSB-first, left-aligned.
    pub rows: Vec<u32>,
}

/// Parsed BDF font.
#[derive(Debug, Clone)]
pub struct BdfFont {
    /// Global ascent in pixels (above baseline).
    pub ascent: i32,
    /// Global descent in pixels (below baseline, positive value).
    pub descent: i32,
    /// Glyph map keyed by codepoint.
    pub glyphs: HashMap<u32, BdfGlyph>,
}

impl BdfFont {
    /// Parse a BDF font from any readable source.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, String> {
        let reader = BufReader::new(reader);
        let mut ascent = 0i32;
        let mut descent = 0i32;
        let mut glyphs: HashMap<u32, BdfGlyph> = HashMap::new();

        // Parser state
        let mut in_char = false;
        let mut in_bitmap = false;
        let mut codepoint: u32 = 0;
        let mut width = 0i32;
        let mut height = 0i32;
        let mut offset_x = 0i32;
        let mut offset_y = 0i32;
        let mut dwidth = 0i32;
        let mut rows: Vec<u32> = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            let line = line.trim();

            if line.starts_with("FONT_ASCENT") {
                ascent = parse_int_after(line, "FONT_ASCENT")?;
            } else if line.starts_with("FONT_DESCENT") {
                descent = parse_int_after(line, "FONT_DESCENT")?;
            } else if line.starts_with("STARTCHAR") {
                in_char = true;
                in_bitmap = false;
                codepoint = 0;
                width = 0;
                height = 0;
                offset_x = 0;
                offset_y = 0;
                dwidth = 0;
                rows = Vec::new();
            } else if line.starts_with("ENCODING") && in_char {
                codepoint = parse_int_after(line, "ENCODING")? as u32;
            } else if line.starts_with("BBX") && in_char {
                // BBX <width> <height> <offset_x> <offset_y>
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    width = parts[1].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                    height = parts[2].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                    offset_x = parts[3].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                    offset_y = parts[4].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                }
            } else if line.starts_with("DWIDTH") && in_char {
                // DWIDTH <x_advance> <y_advance> — we only care about x
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    dwidth = parts[1].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
                }
            } else if line == "BITMAP" {
                in_bitmap = true;
            } else if line == "ENDCHAR" {
                glyphs.insert(codepoint, BdfGlyph { codepoint, width, height, offset_x, offset_y, dwidth, rows: rows.clone() });
                in_char = false;
                in_bitmap = false;
            } else if in_bitmap && !line.is_empty() {
                // Each bitmap row is a hex string. We store it as a u32 MSB-first.
                let val = u32::from_str_radix(line, 16)
                    .map_err(|e| format!("bad bitmap hex '{line}': {e}"))?;
                rows.push(val);
            }
        }

        Ok(BdfFont { ascent, descent, glyphs })
    }
}

fn parse_int_after(line: &str, keyword: &str) -> Result<i32, String> {
    line[keyword.len()..]
        .trim()
        .parse()
        .map_err(|e: std::num::ParseIntError| format!("parsing {keyword}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY_BDF: &str = r#"STARTFONT 2.1
FONT_ASCENT 5
FONT_DESCENT 1
STARTCHAR A
ENCODING 65
DWIDTH 4 0
BBX 4 6 0 -1
BITMAP
60
A0
A0
E0
A0
A0
ENDCHAR
ENDFONT
"#;

    #[test]
    fn parses_ascent_descent() {
        let font = BdfFont::from_reader(TINY_BDF.as_bytes()).unwrap();
        assert_eq!(font.ascent, 5);
        assert_eq!(font.descent, 1);
    }

    #[test]
    fn parses_glyph_a() {
        let font = BdfFont::from_reader(TINY_BDF.as_bytes()).unwrap();
        let g = font.glyphs.get(&65).expect("glyph 'A'");
        assert_eq!(g.width, 4);
        assert_eq!(g.height, 6);
        assert_eq!(g.dwidth, 4);
        assert_eq!(g.rows.len(), 6);
    }
}
