//! Stylised F1 circuit outlines for the e-paper F1 tile.
//!
//! These are hand-traced, recognisable-in-spirit closed loops in a normalised
//! unit square (x right, y down) — one per circuit on the current calendar,
//! capturing each track's signature features (Monaco's hairpin + swimming
//! pool, Monza's chicanes + Parabolica, Baku's long straight + castle section,
//! Suzuka's figure-8, …) rather than survey-accurate geometry. A race name is
//! matched to its circuit by keyword; anything unrecognised falls back to a
//! generic winding-track template so it still looks like *a* track.

use ohmyoled_matrix::graphics::draw_line;
use ohmyoled_matrix::Color;
use image::RgbImage;

/// A circuit outline: a closed loop of normalised `(x, y)` points (x right,
/// y down, each in `0..=1`).
type Outline = &'static [(f32, f32)];

/// One entry in the name → outline table: the keywords that select it.
type CircuitMatch = (&'static [&'static str], Outline);

// --- Per-circuit outlines (clockwise unless the real track runs anti-) -------

/// Suzuka — the signature figure-8 with the crossover.
const SUZUKA: &[(f32, f32)] = &[
    (0.50, 0.06), (0.70, 0.10), (0.84, 0.26), (0.74, 0.42), (0.56, 0.47),
    (0.44, 0.53), (0.26, 0.58), (0.16, 0.74), (0.32, 0.90), (0.52, 0.92),
    (0.70, 0.84), (0.74, 0.64), (0.58, 0.53), (0.44, 0.47), (0.28, 0.42),
    (0.18, 0.28), (0.30, 0.12),
];

/// Monaco — harbour-front start, climb to Casino, Grand Hotel hairpin (the
/// sharp notch), tunnel, Nouvelle Chicane, swimming pool, Rascasse.
const MONACO: &[(f32, f32)] = &[
    (0.30, 0.72), (0.25, 0.56), (0.23, 0.40), (0.30, 0.28), (0.42, 0.24),
    (0.49, 0.31), (0.40, 0.37), (0.47, 0.43), (0.60, 0.47), (0.72, 0.55),
    (0.80, 0.63), (0.72, 0.67), (0.79, 0.74), (0.70, 0.80), (0.64, 0.73),
    (0.55, 0.71), (0.50, 0.79), (0.41, 0.77),
];

/// Monza — two long straights joined by the Variante chicanes, the Lesmos,
/// Ascari, and the long Parabolica sweeping onto the main straight.
const MONZA: &[(f32, f32)] = &[
    (0.30, 0.90), (0.30, 0.55), (0.36, 0.50), (0.30, 0.45), (0.33, 0.30),
    (0.42, 0.22), (0.46, 0.30), (0.42, 0.40), (0.52, 0.40), (0.70, 0.20),
    (0.78, 0.24), (0.66, 0.40), (0.56, 0.52), (0.64, 0.58), (0.78, 0.66),
    (0.80, 0.80), (0.70, 0.88), (0.52, 0.90), (0.40, 0.95),
];

/// Spa-Francorchamps — the long elongated triangle: La Source hairpin, Eau
/// Rouge/Raidillon, Kemmel straight, Les Combes, the long run down to
/// Stavelot and Blanchimont back to the Bus Stop.
const SPA: &[(f32, f32)] = &[
    (0.34, 0.86), (0.28, 0.78), (0.34, 0.72), (0.30, 0.58), (0.20, 0.40),
    (0.14, 0.22), (0.24, 0.12), (0.34, 0.20), (0.40, 0.36), (0.52, 0.44),
    (0.70, 0.40), (0.84, 0.46), (0.86, 0.60), (0.74, 0.70), (0.78, 0.82),
    (0.66, 0.90), (0.50, 0.86), (0.42, 0.92),
];

/// Silverstone — fast, flowing arena layout: Abbey, the Maggotts/Becketts
/// esses, Hangar straight, Stowe, and the Vale/Club complex.
const SILVERSTONE: &[(f32, f32)] = &[
    (0.22, 0.40), (0.34, 0.26), (0.30, 0.16), (0.42, 0.12), (0.52, 0.22),
    (0.50, 0.34), (0.64, 0.30), (0.78, 0.20), (0.86, 0.32), (0.74, 0.44),
    (0.80, 0.58), (0.74, 0.74), (0.60, 0.80), (0.50, 0.70), (0.38, 0.78),
    (0.26, 0.72), (0.30, 0.56), (0.20, 0.52),
];

/// Interlagos — compact, anti-clockwise: the Senna S drop, the long Reta
/// Oposta back straight, Juncao, and the climb onto the main straight.
const INTERLAGOS: &[(f32, f32)] = &[
    (0.42, 0.16), (0.30, 0.22), (0.34, 0.36), (0.50, 0.42), (0.46, 0.54),
    (0.30, 0.58), (0.22, 0.72), (0.30, 0.86), (0.46, 0.90), (0.62, 0.86),
    (0.74, 0.74), (0.80, 0.58), (0.72, 0.46), (0.78, 0.34), (0.70, 0.22),
    (0.56, 0.18),
];

/// Baku — the long castle-section squeeze and the very long pit straight
/// down the seafront.
const BAKU: &[(f32, f32)] = &[
    (0.16, 0.86), (0.16, 0.40), (0.22, 0.26), (0.18, 0.16), (0.28, 0.12),
    (0.34, 0.22), (0.30, 0.32), (0.40, 0.30), (0.40, 0.16), (0.50, 0.12),
    (0.52, 0.28), (0.62, 0.30), (0.66, 0.40), (0.58, 0.48), (0.66, 0.56),
    (0.84, 0.58), (0.86, 0.72), (0.62, 0.80), (0.40, 0.84),
];

/// Singapore (Marina Bay) — a tight street circuit of near right-angle
/// corners around the bay.
const SINGAPORE: &[(f32, f32)] = &[
    (0.18, 0.82), (0.18, 0.60), (0.30, 0.58), (0.30, 0.40), (0.20, 0.38),
    (0.22, 0.22), (0.40, 0.20), (0.42, 0.34), (0.56, 0.34), (0.58, 0.18),
    (0.74, 0.20), (0.74, 0.40), (0.60, 0.46), (0.74, 0.56), (0.84, 0.70),
    (0.66, 0.80), (0.46, 0.76), (0.32, 0.84),
];

/// COTA (Austin) — the steep Turn 1 left, the Maggotts-style esses, the big
/// back loop and the stadium hairpins.
const COTA: &[(f32, f32)] = &[
    (0.20, 0.84), (0.20, 0.40), (0.30, 0.30), (0.26, 0.18), (0.36, 0.16),
    (0.46, 0.26), (0.40, 0.36), (0.52, 0.44), (0.66, 0.40), (0.78, 0.46),
    (0.74, 0.58), (0.60, 0.56), (0.68, 0.66), (0.82, 0.70), (0.74, 0.82),
    (0.58, 0.84), (0.50, 0.74), (0.36, 0.80), (0.28, 0.74), (0.30, 0.86),
];

/// Hungaroring — tight and twisty, "Monaco without the walls".
const HUNGARORING: &[(f32, f32)] = &[
    (0.24, 0.78), (0.22, 0.56), (0.32, 0.50), (0.24, 0.40), (0.30, 0.26),
    (0.44, 0.22), (0.50, 0.34), (0.44, 0.44), (0.56, 0.46), (0.68, 0.34),
    (0.80, 0.30), (0.82, 0.46), (0.70, 0.54), (0.78, 0.66), (0.70, 0.80),
    (0.54, 0.82), (0.46, 0.70), (0.36, 0.82),
];

/// Zandvoort — compact dune circuit with the banked Hugenholtz and the
/// banked final Arie Luyendyk corner onto the straight.
const ZANDVOORT: &[(f32, f32)] = &[
    (0.30, 0.84), (0.22, 0.66), (0.30, 0.56), (0.22, 0.44), (0.24, 0.26),
    (0.36, 0.16), (0.50, 0.18), (0.54, 0.32), (0.48, 0.44), (0.60, 0.46),
    (0.76, 0.40), (0.84, 0.52), (0.74, 0.62), (0.80, 0.74), (0.66, 0.84),
    (0.50, 0.80), (0.46, 0.66), (0.38, 0.78),
];

/// Red Bull Ring (Austria) — short three-sector triangle of long straights
/// and tight uphill corners.
const RED_BULL_RING: &[(f32, f32)] = &[
    (0.26, 0.82), (0.18, 0.60), (0.22, 0.40), (0.34, 0.36), (0.30, 0.20),
    (0.44, 0.14), (0.52, 0.24), (0.46, 0.36), (0.62, 0.38), (0.80, 0.30),
    (0.86, 0.44), (0.72, 0.52), (0.82, 0.66), (0.70, 0.80), (0.50, 0.82),
    (0.40, 0.70),
];

/// Yas Marina (Abu Dhabi) — long straights, the hotel section by the marina,
/// and the reprofiled banked corners.
const YAS_MARINA: &[(f32, f32)] = &[
    (0.18, 0.30), (0.40, 0.20), (0.40, 0.36), (0.56, 0.34), (0.56, 0.18),
    (0.74, 0.22), (0.84, 0.40), (0.72, 0.50), (0.82, 0.62), (0.74, 0.76),
    (0.58, 0.80), (0.50, 0.66), (0.40, 0.78), (0.26, 0.74), (0.22, 0.58),
    (0.34, 0.50), (0.20, 0.44),
];

/// Bahrain (Sakhir) — long main straight, the heavy-braking Turn 1 complex
/// and a flowing middle sector.
const BAHRAIN: &[(f32, f32)] = &[
    (0.16, 0.80), (0.58, 0.80), (0.70, 0.68), (0.56, 0.60), (0.42, 0.64),
    (0.40, 0.50), (0.54, 0.42), (0.72, 0.42), (0.84, 0.30), (0.70, 0.18),
    (0.50, 0.20), (0.36, 0.14), (0.22, 0.22), (0.20, 0.40), (0.32, 0.50),
    (0.28, 0.66), (0.16, 0.66),
];

/// Jeddah (Saudi Arabia) — extremely fast, flowing seafront layout with a
/// long string of mild kinks.
const JEDDAH: &[(f32, f32)] = &[
    (0.16, 0.84), (0.16, 0.52), (0.26, 0.42), (0.20, 0.30), (0.30, 0.18),
    (0.44, 0.22), (0.48, 0.12), (0.62, 0.14), (0.60, 0.28), (0.72, 0.24),
    (0.84, 0.34), (0.78, 0.48), (0.86, 0.62), (0.74, 0.72), (0.80, 0.84),
    (0.62, 0.88), (0.44, 0.82), (0.30, 0.88),
];

/// Albert Park (Australia) — fast parkland circuit around the lake.
const ALBERT_PARK: &[(f32, f32)] = &[
    (0.24, 0.78), (0.18, 0.54), (0.26, 0.36), (0.40, 0.24), (0.58, 0.18),
    (0.74, 0.22), (0.84, 0.36), (0.82, 0.54), (0.72, 0.64), (0.78, 0.78),
    (0.64, 0.84), (0.52, 0.74), (0.42, 0.82), (0.30, 0.86),
];

/// Shanghai — the signature long Turn 1-2-3-4 "snail" and the long back
/// straight into the tight hairpin.
const SHANGHAI: &[(f32, f32)] = &[
    (0.30, 0.86), (0.24, 0.66), (0.34, 0.58), (0.26, 0.48), (0.34, 0.40),
    (0.30, 0.30), (0.40, 0.24), (0.48, 0.32), (0.44, 0.42), (0.54, 0.40),
    (0.78, 0.22), (0.86, 0.34), (0.66, 0.50), (0.74, 0.60), (0.82, 0.74),
    (0.66, 0.84), (0.50, 0.78), (0.40, 0.88),
];

/// Miami — fast straights wrapped around a twisty stadium infield.
const MIAMI: &[(f32, f32)] = &[
    (0.20, 0.80), (0.20, 0.40), (0.32, 0.26), (0.50, 0.20), (0.70, 0.22),
    (0.84, 0.34), (0.82, 0.50), (0.66, 0.52), (0.58, 0.44), (0.50, 0.52),
    (0.58, 0.60), (0.50, 0.68), (0.40, 0.60), (0.46, 0.50), (0.38, 0.46),
    (0.34, 0.58), (0.44, 0.78), (0.32, 0.84),
];

/// Imola — old-school, flowing, anti-clockwise: Tamburello, Tosa, the climb
/// to Piratella and the Acque Minerali/Variante Alta sweep.
const IMOLA: &[(f32, f32)] = &[
    (0.40, 0.86), (0.30, 0.74), (0.34, 0.62), (0.24, 0.52), (0.30, 0.38),
    (0.22, 0.24), (0.34, 0.14), (0.46, 0.22), (0.44, 0.36), (0.56, 0.40),
    (0.66, 0.30), (0.80, 0.34), (0.78, 0.50), (0.64, 0.56), (0.74, 0.66),
    (0.66, 0.82), (0.52, 0.84), (0.48, 0.72),
];

/// Montreal (Gilles Villeneuve) — island circuit: long straights, chicanes,
/// the hairpin and the Wall of Champions onto the pit straight.
const MONTREAL: &[(f32, f32)] = &[
    (0.18, 0.80), (0.22, 0.50), (0.16, 0.38), (0.26, 0.30), (0.22, 0.18),
    (0.36, 0.16), (0.40, 0.30), (0.52, 0.24), (0.64, 0.30), (0.60, 0.42),
    (0.74, 0.40), (0.86, 0.52), (0.84, 0.68), (0.70, 0.66), (0.78, 0.78),
    (0.64, 0.86), (0.46, 0.80), (0.32, 0.84),
];

/// Barcelona-Catalunya — fast Turn 1-2-3 downhill, the long sweeps of sector
/// 2 and the tight final chicane onto the straight.
const BARCELONA: &[(f32, f32)] = &[
    (0.24, 0.82), (0.20, 0.54), (0.30, 0.40), (0.26, 0.26), (0.40, 0.16),
    (0.56, 0.18), (0.66, 0.28), (0.60, 0.40), (0.72, 0.44), (0.84, 0.40),
    (0.86, 0.56), (0.72, 0.62), (0.80, 0.74), (0.66, 0.84), (0.50, 0.78),
    (0.46, 0.66), (0.36, 0.80),
];

/// Mexico City (Hermanos Rodríguez) — the long straight, the Esses and the
/// famous slow stadium section through the Foro Sol.
const MEXICO: &[(f32, f32)] = &[
    (0.16, 0.82), (0.16, 0.34), (0.26, 0.20), (0.44, 0.16), (0.66, 0.18),
    (0.82, 0.28), (0.84, 0.44), (0.70, 0.50), (0.78, 0.58), (0.70, 0.66),
    (0.56, 0.60), (0.58, 0.72), (0.46, 0.78), (0.44, 0.66), (0.34, 0.74),
    (0.30, 0.60), (0.40, 0.52), (0.28, 0.46),
];

/// Las Vegas — the Strip street circuit: very long straights and a small
/// cluster of slow corners.
const LAS_VEGAS: &[(f32, f32)] = &[
    (0.16, 0.84), (0.16, 0.30), (0.26, 0.18), (0.42, 0.20), (0.44, 0.32),
    (0.58, 0.30), (0.58, 0.18), (0.84, 0.22), (0.84, 0.42), (0.50, 0.50),
    (0.84, 0.60), (0.84, 0.80), (0.50, 0.84), (0.32, 0.80), (0.30, 0.66),
    (0.20, 0.70),
];

/// Lusail (Qatar) — fast, flowing, oval-ish outer with a twisty infield run.
const LUSAIL: &[(f32, f32)] = &[
    (0.30, 0.84), (0.20, 0.66), (0.24, 0.48), (0.18, 0.34), (0.30, 0.20),
    (0.48, 0.14), (0.66, 0.16), (0.80, 0.26), (0.84, 0.44), (0.66, 0.48),
    (0.78, 0.56), (0.82, 0.72), (0.68, 0.84), (0.50, 0.86), (0.42, 0.72),
    (0.52, 0.60), (0.42, 0.52), (0.38, 0.68),
];

/// Generic winding-track templates — fallback for any unmatched name.
const TEMPLATES: &[&[(f32, f32)]] = &[
    &[
        (0.12, 0.42), (0.10, 0.20), (0.28, 0.10), (0.50, 0.16), (0.58, 0.34),
        (0.76, 0.16), (0.90, 0.34), (0.84, 0.56), (0.64, 0.58), (0.72, 0.80),
        (0.50, 0.90), (0.30, 0.86), (0.24, 0.62), (0.12, 0.58),
    ],
    &[
        (0.20, 0.30), (0.45, 0.12), (0.70, 0.14), (0.88, 0.34), (0.80, 0.50),
        (0.60, 0.50), (0.72, 0.66), (0.82, 0.84), (0.54, 0.90), (0.28, 0.86),
        (0.12, 0.62), (0.18, 0.42),
    ],
];

/// Race-name keyword → circuit outline. Checked in order, so put the most
/// specific keywords first; country names act as a backstop. The F1 API gives
/// us either the circuit name ("Circuit de Monaco") or the race name ("Monaco
/// Grand Prix"), so we match substrings that appear in either.
const CIRCUITS: &[CircuitMatch] = &[
    (&["suzuka", "japan"], SUZUKA),
    (&["monaco", "monte"], MONACO),
    (&["monza"], MONZA),
    (&["spa", "francorchamps", "belgian", "belgium"], SPA),
    (&["silverstone", "british", "britain"], SILVERSTONE),
    (&["interlagos", "carlos pace", "paulo", "brazil", "brasil"], INTERLAGOS),
    (&["baku", "azerbaijan"], BAKU),
    (&["marina bay", "singapore"], SINGAPORE),
    (&["americas", "cota", "austin"], COTA),
    (&["hungaroring", "hungar", "budapest"], HUNGARORING),
    (&["zandvoort", "dutch", "netherlands"], ZANDVOORT),
    (&["red bull ring", "spielberg", "austria", "austrian"], RED_BULL_RING),
    (&["yas marina", "abu dhabi", "yas island"], YAS_MARINA),
    (&["bahrain", "sakhir"], BAHRAIN),
    (&["jeddah", "saudi", "corniche"], JEDDAH),
    (&["albert park", "melbourne", "australia", "australian"], ALBERT_PARK),
    (&["shanghai", "chinese", "china"], SHANGHAI),
    (&["miami"], MIAMI),
    (&["imola", "emilia", "enzo", "dino"], IMOLA),
    (&["villeneuve", "montreal", "canad"], MONTREAL),
    (&["catalunya", "barcelona", "spanish", "spain"], BARCELONA),
    (&["hermanos", "rodriguez", "rodríguez", "mexic"], MEXICO),
    (&["las vegas", "vegas", "strip"], LAS_VEGAS),
    (&["lusail", "losail", "qatar"], LUSAIL),
    // "italian"/"italy" last so Imola (Emilia-Romagna) is matched first.
    (&["italian", "italy"], MONZA),
    (&["united states", "u.s."], COTA),
];

/// Pick a circuit outline for a circuit/race name. Matches a keyword if we
/// have that circuit; otherwise returns a deterministic generic template so
/// unknown races still look like a track (and the same name always renders the
/// same outline).
pub fn path_for(name: &str) -> &'static [(f32, f32)] {
    let key = name.to_lowercase();
    for (keywords, path) in CIRCUITS {
        if keywords.iter().any(|kw| key.contains(kw)) {
            return path;
        }
    }
    let hash = key.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    TEMPLATES[hash as usize % TEMPLATES.len()]
}

/// Draw a circuit `path` (closed loop) aspect-fit and centered in the box
/// `[x, x+w] × [y, y+h]`, with a start/finish tick at the first point.
pub fn draw(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, path: &[(f32, f32)], color: Color) {
    if path.len() < 2 {
        return;
    }
    let s = w.min(h);
    let ox = x + (w - s) / 2;
    let oy = y + (h - s) / 2;
    let pt = |p: (f32, f32)| (ox + (p.0 * s as f32) as i32, oy + (p.1 * s as f32) as i32);
    for i in 0..path.len() {
        let a = pt(path[i]);
        let b = pt(path[(i + 1) % path.len()]);
        draw_line(img, a.0, a.1, b.0, b.1, color);
        // Second pass offset by 1px so the track reads as a thicker line.
        draw_line(img, a.0, a.1 + 1, b.0, b.1 + 1, color);
    }
    // Start/finish tick: a short bar drawn *perpendicular* to the track at the
    // first point, so it reads as a start/finish line crossing the track rather
    // than a fixed diagonal that pokes out past the outline.
    let a = pt(path[0]);
    let b = pt(path[1 % path.len()]);
    let (dx, dy) = ((b.0 - a.0) as f32, (b.1 - a.1) as f32);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let (nx, ny) = (-dy / len, dx / len); // unit normal to the first segment
    let half = (s / 28).max(2) as f32;
    let ex = (nx * half) as i32;
    let ey = (ny * half) as i32;
    draw_line(img, a.0 - ex, a.1 - ey, a.0 + ex, a.1 + ey, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_circuits_matched_by_keyword() {
        assert_eq!(path_for("Suzuka Circuit"), SUZUKA);
        assert_eq!(path_for("Japanese Grand Prix"), SUZUKA);
        assert_eq!(path_for("Circuit de Monaco"), MONACO);
        assert_eq!(path_for("Autodromo Nazionale di Monza"), MONZA);
        assert_eq!(path_for("Italian Grand Prix"), MONZA);
        assert_eq!(path_for("Circuit de Spa-Francorchamps"), SPA);
        assert_eq!(path_for("Circuit of the Americas"), COTA);
        assert_eq!(path_for("United States Grand Prix"), COTA);
        assert_eq!(path_for("Baku City Circuit"), BAKU);
        // Imola must win over the generic "italian" rule.
        assert_eq!(path_for("Autodromo Enzo e Dino Ferrari"), IMOLA);
    }

    #[test]
    fn unknown_falls_back_to_template_deterministically() {
        let p = path_for("Some Made Up Circuit");
        assert!(TEMPLATES.contains(&p));
        assert_eq!(path_for("Some Made Up Circuit"), p);
    }

    #[test]
    fn draw_lights_pixels() {
        let mut img = RgbImage::new(200, 200);
        draw(&mut img, 10, 10, 180, 180, path_for("Bahrain"), Color::WHITE);
        assert!(img.pixels().filter(|p| p.0 != [0, 0, 0]).count() > 50);
    }

    /// Dev-only: `cargo test --lib eink::circuits::tests::ascii -- --nocapture
    /// --ignored` prints every outline as ASCII so the shapes can be eyeballed.
    #[test]
    #[ignore]
    fn ascii_preview() {
        let (w, h) = (60i32, 30i32);
        let mut named: Vec<(String, &[(f32, f32)])> = CIRCUITS
            .iter()
            .map(|(kw, p)| (kw[0].to_string(), *p))
            .collect();
        named.push(("(template 0)".into(), TEMPLATES[0]));
        named.push(("(template 1)".into(), TEMPLATES[1]));
        for (name, path) in named {
            let mut img = RgbImage::new(w as u32, h as u32);
            draw(&mut img, 0, 0, w, h, path, Color::WHITE);
            println!("\n=== {name} ===");
            for yy in 0..h {
                let mut row = String::new();
                for xx in 0..w {
                    let lit = img.get_pixel(xx as u32, yy as u32).0 != [0, 0, 0];
                    row.push(if lit { '#' } else { ' ' });
                }
                println!("{row}");
            }
        }
    }

    /// Every outline is a sane closed loop: enough points, all in the unit
    /// square, and no two consecutive points identical (which would render a
    /// zero-length segment).
    #[test]
    fn all_outlines_well_formed() {
        let mut all: Vec<&[(f32, f32)]> = CIRCUITS.iter().map(|(_, p)| *p).collect();
        all.extend_from_slice(TEMPLATES);
        for path in all {
            assert!(path.len() >= 8, "outline too short: {}", path.len());
            for &(px, py) in path {
                assert!((0.0..=1.0).contains(&px) && (0.0..=1.0).contains(&py));
            }
            for w in path.windows(2) {
                assert!(w[0] != w[1], "duplicate consecutive point");
            }
        }
    }
}
