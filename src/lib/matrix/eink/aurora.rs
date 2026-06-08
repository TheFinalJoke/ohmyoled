//! E-paper aurora renderer — Kp index + the auroral oval on a world map.
//!
//! Static e-paper counterpart to [`crate::matrix::aurora::AuroraMatrix`]. The
//! left column is the Kp readout (big digit, a 9-step gauge, an alert badge,
//! and the lowest latitude the aurora is likely visible from); the right is a
//! world map with the **auroral oval** drawn for the current Kp — north and
//! south boundary curves that march toward the equator as the storm grows.
//! Composed white-on-black; the display inverts to black ink.
//!
//! The oval is a standard approximation: its equatorward edge sits near
//! geomagnetic latitude `67 − 2·Kp`, offset toward the geomagnetic poles (over
//! Arctic Canada / the Southern Ocean) so it dips lowest over North America.
//!
//! # Config
//!
//! Lives under the `eink.modules` block, reusing the `aurora` section shape:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     aurora:
//!       run: true
//!       alert_threshold: 5
//! ```
//!
//! Data source: NOAA SWPC (same collector as the LED tile).

use crate::api::aurora::model::AuroraReading;
use crate::matrix::eink::layout::{
    badge, badge_width, center_text, fill_rect, footer, header_band, hbar, margin, scaled_px,
};
use crate::matrix::eink::worldmap;
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::RgbImage;
use ohmyoled_matrix::graphics::{draw_line, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Equatorward edge amplitude of the oval (degrees of geomagnetic offset).
const OVAL_AMP: f32 = 8.0;

/// Font paths for the e-paper aurora renderer.
pub struct EinkAuroraFonts {
    /// Body/label font (04B_03B TTF).
    pub body: PathBuf,
    /// Heavy font for the big Kp digit (04b24 OTF).
    pub big: PathBuf,
}

impl Default for EinkAuroraFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
            big: Path::new(FONT_DIR).join("04b24.otf"),
        }
    }
}

/// Static e-paper aurora dashboard renderer.
pub struct EinkAuroraMatrix {
    title: Font,
    big: Font,
    label: Font,
    banner: Font,
    foot: Font,
}

impl EinkAuroraMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkAuroraFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkAuroraFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkAuroraFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            big: Font::load_ttf(&paths.big, scaled_px(120.0, h))?,
            label: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            banner: Font::load_ttf(&paths.body, scaled_px(24.0, h))?,
            foot: Font::load_ttf(&paths.body, scaled_px(18.0, h))?,
        })
    }

    pub async fn with_fonts_async(paths: EinkAuroraFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the dashboard at `w × h`.
    pub fn frame(&self, data: &AuroraReading, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);

        let content_top = header_band(&mut img, &self.title, &self.label, m, "AURORA", Some(&data.kp_text), fg);
        let footer_top = hi - m - self.foot.height();
        footer(&mut img, &self.foot, fg, &format!("updated {}", data.sampled_at.format("%H:%M UTC")));

        // ── Left column: Kp, gauge, alert, visibility ──────────────────
        let col_w = wi * 36 / 100;
        let col_cx = (m + col_w) / 2;
        center_text(&mut img, &self.label, col_cx, content_top + m + self.label.ascent(), fg, "Kp INDEX");
        let big_base = content_top + m + self.label.height() + m + self.big.ascent();
        center_text(&mut img, &self.big, col_cx, big_base, fg, &format!("{}", data.kp));

        let (alert, filled) = if data.alert { ("AURORA LIKELY", true) } else { ("QUIET", false) };
        let alert_y = big_base + m;
        badge(&mut img, &self.banner, col_cx - badge_width(&self.banner, alert) / 2, alert_y, alert, fg, filled);

        let gauge_y = alert_y + self.banner.height() + m;
        let gauge_h = (hi / 24).max(8);
        hbar(&mut img, m, gauge_y, col_w - 2 * m, gauge_h, data.kp as f32 / 9.0, 9, fg);

        // Lowest geomagnetic-ish latitude the oval reaches (most equatorward
        // point, over North America).
        let visible_to = (67.0 - 2.0 * data.kp as f32 - OVAL_AMP).max(40.0);
        center_text(
            &mut img,
            &self.label,
            col_cx,
            gauge_y + gauge_h + m + self.label.ascent(),
            fg,
            &format!("VISIBLE TO ~{visible_to:.0} N"),
        );

        // ── Right: world map with the auroral oval ─────────────────────
        let map_x = col_w + m;
        let map_y = content_top + m / 2;
        let map_w = wi - m - map_x;
        let map_h = (footer_top - m / 2 - map_y).max(20);
        worldmap::draw(&mut img, map_x, map_y, map_w, map_h, fg);
        // Northern oval (pole over Arctic Canada) + southern oval.
        self.draw_oval(&mut img, map_x, map_y, map_w, map_h, data.kp, -100.0, 1.0, fg);
        self.draw_oval(&mut img, map_x, map_y, map_w, map_h, data.kp, 110.0, -1.0, fg);

        img
    }

    /// Draw one hemisphere's auroral oval equatorward boundary as a curve, with
    /// a sparse poleward stipple to suggest the glow band. `sign` is +1 for the
    /// north, −1 for the south; `pole_lon` offsets the oval toward that pole.
    #[allow(clippy::too_many_arguments)]
    fn draw_oval(&self, img: &mut RgbImage, mx: i32, my: i32, mw: i32, mh: i32, kp: u8, pole_lon: f32, sign: f32, fg: Color) {
        let l0 = 67.0 - 2.0 * kp as f32;
        let edge_lat = |lon: f32| -> f32 {
            let e = l0 - OVAL_AMP * (lon - pole_lon).to_radians().cos();
            (sign * e).clamp(-88.0, 88.0)
        };
        let mut prev: Option<(i32, i32)> = None;
        let mut x = mx;
        while x <= mx + mw {
            let lon = (x - mx) as f32 / mw as f32 * 360.0 - 180.0;
            let (_, py) = worldmap::project(edge_lat(lon), lon, mx, my, mw, mh);
            if let Some((ppx, ppy)) = prev {
                draw_line(img, ppx, ppy, x, py, fg);
            }
            prev = Some((x, py));
            // Sparse poleward glow dots (toward the pole = +sign latitude).
            if (x - mx) % 12 == 0 {
                for d in [6.0_f32, 12.0] {
                    let (_, gy) = worldmap::project(edge_lat(lon) + sign * d, lon, mx, my, mw, mh);
                    fill_rect(img, x, gy, 2, 2, fg);
                }
            }
            x += 4;
        }
    }
}

#[async_trait]
impl EinkRenderer for EinkAuroraMatrix {
    type Data = AuroraReading;

    fn id(&self) -> &'static str {
        "aurora"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &AuroraReading) -> Result<(), RenderError> {
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn repo_fonts() -> EinkAuroraFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkAuroraFonts {
            body: base.join("04B_03B_.TTF"),
            big: base.join("04b24.otf"),
        }
    }

    fn reading(kp: u8, alert: bool) -> AuroraReading {
        AuroraReading {
            kp,
            kp_index: kp as f32,
            kp_text: format!("{kp}"),
            alert,
            sampled_at: Utc::now(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkAuroraMatrix::with_fonts(repo_fonts(), (800, 480)).expect("fonts load");
        let img = r.frame(&reading(6, true), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated dashboard, got {lit} lit px");
    }

    #[test]
    fn higher_kp_pushes_the_oval_equatorward() {
        // A bigger storm should move the boundary, so the frames differ.
        let r = EinkAuroraMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let calm = r.frame(&reading(1, false), 800, 480);
        let storm = r.frame(&reading(8, true), 800, 480);
        assert_ne!(calm.into_raw(), storm.into_raw(), "Kp should reshape the oval + gauge");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkAuroraMatrix::with_fonts(repo_fonts(), (400, 300)).expect("fonts load");
        let img = r.frame(&reading(4, false), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
