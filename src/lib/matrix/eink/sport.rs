//! E-paper team-sport renderer — a static scoreboard + standings table.
//!
//! Static e-paper counterpart to [`crate::matrix::sport::SportMatrix`]. The LED
//! tile scrolls names and standings and animates; the tall panel shows the
//! whole scoreboard at once: both teams (abbreviation badges + full names), a
//! big centered score/status, and a standings table. Off-season renders a card
//! rather than blanking. Win/lose color becomes the score position + a status
//! badge. Composed white-on-black; the display inverts to black ink.
//!
//! Logos: the LED tile fetches and caches raster team logos. The e-paper tile
//! keeps `frame()` pure and offline by drawing boxed team **abbreviations**
//! instead — legible at a glance and free of async logo I/O.
//!
//! # Config
//!
//! Lives under the `eink.modules.sport` array as a team-sport-tagged entry:
//!
//! ```yaml
//! eink:
//!   enabled: true
//!   modules:
//!     sport:
//!       - sport: basketball
//!         run: true
//!         team_logo: { name: "76ers", shorthand: "PHI" }
//! ```
//!
//! Data source: ESPN (same collector as the LED tile).

use crate::api::http::shared_client;
use crate::api::sport::model::{GameStatus, NextGame, SportData, TeamSide};
use crate::matrix::eink::layout::{
    badge, badge_width, center_text, fit_text, header_band, margin, rect, right_text, scaled_px,
};
use crate::matrix::eink_renderer::EinkRenderer;
use crate::matrix::error::RenderError;
use async_trait::async_trait;
use image::imageops::FilterType;
use image::RgbImage;
use chrono::Local;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use ohmyoled_matrix::graphics::{draw_text, Font};
use ohmyoled_matrix::{Color, EinkDisplay};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FONT_DIR: &str = "/usr/share/fonts";

/// Font paths for the e-paper sport renderer.
pub struct EinkSportFonts {
    /// The pixel font used at every size.
    pub body: PathBuf,
}

impl Default for EinkSportFonts {
    fn default() -> Self {
        Self {
            body: Path::new(FONT_DIR).join("04B_03B_.TTF"),
        }
    }
}

/// Static e-paper team-sport scoreboard renderer.
pub struct EinkSportMatrix {
    title: Font,
    abbr: Font,
    name: Font,
    score: Font,
    status: Font,
    row: Font,
    /// Fetched + resized team logos, keyed by team name. Populated by a
    /// background task spawned from `render` (never on the render path); `frame`
    /// draws from it (abbreviation box on a miss). Shared (`Arc<Mutex<…>>`) so
    /// the spawned fetch can insert without blocking the display. A `std::Mutex`
    /// is fine here — the guard is only ever held for a `get`/`insert`, never
    /// across an `.await` — and `frame` is sync so it can't await a tokio lock.
    logo_cache: Arc<Mutex<HashMap<String, RgbImage>>>,
    /// Team names with a logo fetch currently in flight, so at most one task is
    /// spawned per team even while the fetch is still pending.
    logo_inflight: Arc<Mutex<HashSet<String>>>,
    /// When `true`, off-season renders a crest + "OFF-SEASON" card; when `false`
    /// (the default) the tile yields its slot (no repaint) so the panel keeps
    /// the previous module's content.
    show_offseason: bool,
    /// Configured team badge URL, fetched for the off-season crest (off-season
    /// payloads carry no per-side `logo_url`).
    offseason_logo_url: Option<String>,
}

impl EinkSportMatrix {
    pub fn new(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts(EinkSportFonts::default(), dims)
    }

    pub async fn new_async(dims: (u32, u32)) -> Result<Self, String> {
        Self::with_fonts_async(EinkSportFonts::default(), dims).await
    }

    pub fn with_fonts(paths: EinkSportFonts, dims: (u32, u32)) -> Result<Self, String> {
        let h = dims.1;
        Ok(Self {
            title: Font::load_ttf(&paths.body, scaled_px(30.0, h))?,
            abbr: Font::load_ttf(&paths.body, scaled_px(34.0, h))?,
            name: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            score: Font::load_ttf(&paths.body, scaled_px(96.0, h))?,
            status: Font::load_ttf(&paths.body, scaled_px(26.0, h))?,
            row: Font::load_ttf(&paths.body, scaled_px(22.0, h))?,
            logo_cache: Arc::new(Mutex::new(HashMap::new())),
            logo_inflight: Arc::new(Mutex::new(HashSet::new())),
            show_offseason: false,
            offseason_logo_url: None,
        })
    }

    /// Enable (or disable) the off-season card and supply the team badge URL it
    /// draws. Builder-style so the registry can chain it onto `new_async`.
    pub fn with_offseason(mut self, show: bool, logo_url: Option<String>) -> Self {
        self.show_offseason = show;
        self.offseason_logo_url = logo_url;
        self
    }

    /// Background-fetch the configured badge for the off-season crest, keyed by
    /// team name (the same cache `draw_crest`/the card read). Returns
    /// immediately — never blocks the render path.
    fn prefetch_offseason_logo(&self, data: &SportData) {
        let Some(url) = self.offseason_logo_url.clone() else { return };
        let name = data.team_name.clone();
        if self.logo_cache.lock().unwrap().contains_key(&name) {
            return;
        }
        if !self.logo_inflight.lock().unwrap().insert(name.clone()) {
            return;
        }
        let cache = Arc::clone(&self.logo_cache);
        let inflight = Arc::clone(&self.logo_inflight);
        tokio::spawn(async move {
            match fetch_logo(&url).await {
                Ok(logo) => {
                    cache.lock().unwrap().insert(name.clone(), logo);
                }
                Err(e) => log::warn!("eink sport: offseason logo fetch failed for {name} ({url}): {e}"),
            }
            inflight.lock().unwrap().remove(&name);
        });
    }

    /// Kick off a background fetch of a team's logo into the shared cache.
    /// Returns immediately — never blocks the render path on the network. At
    /// most one task is spawned per team (in-flight guard); a miss or failure
    /// just leaves `frame` to draw the abbreviation box until the logo lands.
    fn prefetch_logo(&self, side: &TeamSide) {
        let Some(url) = side.logo_url.clone() else { return };
        let name = side.name.clone();
        // Already cached → nothing to do.
        if self.logo_cache.lock().unwrap().contains_key(&name) {
            return;
        }
        // Claim the in-flight slot; bail if another task is already fetching it.
        if !self.logo_inflight.lock().unwrap().insert(name.clone()) {
            return;
        }
        let cache = Arc::clone(&self.logo_cache);
        let inflight = Arc::clone(&self.logo_inflight);
        tokio::spawn(async move {
            match fetch_logo(&url).await {
                Ok(logo) => {
                    cache.lock().unwrap().insert(name.clone(), logo);
                }
                Err(e) => log::warn!("eink sport: logo fetch failed for {name} ({url}): {e}"),
            }
            inflight.lock().unwrap().remove(&name);
        });
    }

    pub async fn with_fonts_async(paths: EinkSportFonts, dims: (u32, u32)) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths, dims))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    /// Compose the scoreboard at `w × h`.
    pub fn frame(&self, data: &SportData, w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        let fg = Color::WHITE;
        let wi = w as i32;
        let hi = h as i32;
        let m = margin(w);
        let cx = wi / 2;

        let content_top = header_band(&mut img, &self.title, &self.row, m, data.sport.display_name(), Some(&data.record), fg);

        let standings_top = if data.standings.is_empty() { hi } else { hi - hi * 50 / 100 };

        match data.current_game(Local::now()) {
            Some(game) => self.draw_scoreboard(&mut img, game, content_top, standings_top, fg),
            None if data.standings.is_empty() => {
                self.draw_offseason_card(&mut img, data, content_top, standings_top, fg);
            }
            None => {
                // Standings present but no current game — a between-games lull,
                // not the off-season. Keep the simple label; the table renders
                // below.
                let label = "NO UPCOMING GAME".to_string();
                let bx = cx - badge_width(&self.abbr, &label) / 2;
                let by = (content_top + standings_top) / 2 - self.abbr.height() / 2;
                badge(&mut img, &self.abbr, bx, by, &label, fg, true);
            }
        }

        if !data.standings.is_empty() {
            ohmyoled_matrix::graphics::draw_line(&mut img, m, standings_top, wi - m, standings_top, fg);
            self.draw_standings(&mut img, data, standings_top + m / 2, fg);
        }
        img
    }

    fn draw_scoreboard(&self, img: &mut RgbImage, game: &NextGame, top: i32, bottom: i32, fg: Color) {
        let wi = img.width() as i32;
        let m = margin(img.width());
        let band_h = bottom - top;
        let home_cx = wi / 5;
        let away_cx = wi * 4 / 5;
        let cx = wi / 2;
        // Everything hangs off the band's vertical center so the scoreboard
        // fills the space rather than hugging the top.
        let score_cy = top + band_h / 2;

        // Playoff context (round/game headline + series score) pinned to the top
        // of the band. Absent in the regular season, so this is postseason-only.
        if let Some(note) = &game.playoff_note {
            let note = fit_text(&self.status, &note.to_uppercase(), wi - 2 * m);
            center_text(img, &self.status, cx, top + self.status.ascent(), fg, &note);
        }

        // Team crest (logo if fetched, else an abbreviation box) + name,
        // vertically centered as a unit on the score line.
        let box_w = (wi / 6).min(band_h * 42 / 100);
        let unit_h = box_w + m / 2 + self.name.height();
        let box_top = score_cy - unit_h / 2;
        for (tcx, side) in [(home_cx, &game.home), (away_cx, &game.away)] {
            self.draw_crest(img, tcx, box_top, box_w, side, fg);
            let name = fit_text(&self.name, &side.name.to_uppercase(), wi / 3 - m);
            center_text(img, &self.name, tcx, box_top + box_w + m / 2 + self.name.ascent(), fg, &name);
        }

        // Status badge above the centered score.
        let (status_text, filled) = match game.status {
            GameStatus::Final => ("FINAL", true),
            GameStatus::InProgress => ("LIVE", true),
            GameStatus::Postponed => ("PPD", true),
            GameStatus::Scheduled => ("SCHEDULED", false),
        };
        let sb_x = cx - badge_width(&self.status, status_text) / 2;
        let sb_y = score_cy - self.score.height() / 2 - self.status.height() - m / 2;
        badge(img, &self.status, sb_x, sb_y, status_text, fg, filled);

        match game.status {
            GameStatus::Final | GameStatus::InProgress => {
                let hs = game.home.score.unwrap_or(0);
                let as_ = game.away.score.unwrap_or(0);
                let s = format!("{hs}-{as_}");
                let base = score_cy - self.score.text_v_center_from_baseline(&s);
                center_text(img, &self.score, cx, base, fg, &s);
            }
            GameStatus::Scheduled | GameStatus::Postponed => {
                let date = game.start.format("%a %b %-d").to_string();
                let time = game.start.format("%-I:%M %p").to_string();
                center_text(img, &self.status, cx, score_cy, fg, &date);
                center_text(img, &self.status, cx, score_cy + self.status.height(), fg, &time);
            }
        }
    }

    /// Off-season card: the team crest (when its badge has been fetched)
    /// centered above the team name and an "OFF-SEASON" badge. With no crest yet
    /// (cold cache) it gracefully degrades to name + badge.
    fn draw_offseason_card(&self, img: &mut RgbImage, data: &SportData, top: i32, bottom: i32, fg: Color) {
        let wi = img.width() as i32;
        let m = margin(img.width());
        let cx = wi / 2;
        let band_h = bottom - top;
        let cy = top + band_h / 2;

        // Crest (if fetched) centered in the upper part of the band.
        let cached = self.logo_cache.lock().unwrap().get(&data.team_name).cloned();
        let crest_w = (wi / 5).min(band_h * 40 / 100);
        let name_y = if let Some(logo) = cached {
            let bx = cx - crest_w / 2;
            let by = cy - band_h / 4 - crest_w / 2;
            draw_logo_silhouette(img, &logo, bx, by, crest_w, fg);
            by + crest_w + m
        } else {
            top + m
        };

        // Team name under the crest.
        let name = fit_text(&self.name, &data.team_name.to_uppercase(), wi - 2 * m);
        center_text(img, &self.name, cx, name_y + self.name.ascent(), fg, &name);

        // "OFF-SEASON" badge anchored toward the bottom of the band.
        let label = "OFF-SEASON";
        let bx = cx - badge_width(&self.status, label) / 2;
        let by = (name_y + self.name.height() + bottom) / 2 - self.status.height() / 2;
        badge(img, &self.status, bx, by, label, fg, true);
    }

    /// Draw a team crest into the `box_w` square at `(cx - box_w/2, box_top)`:
    /// the fetched logo as a B/W silhouette when available, else an outlined
    /// box with the abbreviation.
    fn draw_crest(&self, img: &mut RgbImage, cx: i32, box_top: i32, box_w: i32, side: &TeamSide, fg: Color) {
        let bx = cx - box_w / 2;
        let cached = self.logo_cache.lock().unwrap().get(&side.name).cloned();
        if let Some(logo) = cached {
            draw_logo_silhouette(img, &logo, bx, box_top, box_w, fg);
        } else {
            rect(img, bx, box_top, box_w, box_w, fg);
            center_text(img, &self.abbr, cx, box_top + box_w / 2 + self.abbr.ascent() / 2, fg, &side.abbreviation);
        }
    }

    /// Two-column standings (1.. on the left, the next batch on the right) so
    /// the bottom band fills the panel width.
    fn draw_standings(&self, img: &mut RgbImage, data: &SportData, top: i32, fg: Color) {
        let wi = img.width() as i32;
        let hi = img.height() as i32;
        let m = margin(img.width());
        let pos_col = wi / 22;
        let row_h = self.row.height() + m / 3;
        let per_col = (((hi - m) - top) / row_h).clamp(1, 10) as usize;
        let col_w = (wi - 2 * m) / 2;
        for (i, e) in data.standings.iter().take(per_col * 2).enumerate() {
            let (col, r) = (i / per_col, i % per_col);
            let cx0 = m + (col as i32) * col_w;
            let base = top + row_h * r as i32 + self.row.ascent();
            let name_x = cx0 + pos_col + m;
            let ours = e.team_name.eq_ignore_ascii_case(&data.team_name);
            if ours {
                badge(img, &self.row, cx0, top + row_h * r as i32, &format!("{}", e.position), fg, true);
            } else {
                right_text(img, &self.row, cx0 + pos_col, base, fg, &format!("{}", e.position));
            }
            let name = fit_text(&self.row, &e.team_name, cx0 + col_w - name_x - m);
            draw_text(img, &self.row, name_x, base, fg, &name);
        }
    }
}

/// Composite a fetched logo into the `box_w` square as a high-contrast B/W
/// silhouette: source pixels that are dark or saturated (the logo's marks)
/// become foreground ink; near-white background is left clear.
fn draw_logo_silhouette(img: &mut RgbImage, logo: &RgbImage, bx: i32, by: i32, box_w: i32, fg: Color) {
    let n = logo.width().max(1);
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    let px = image::Rgb([fg.r, fg.g, fg.b]);
    for ty in 0..box_w {
        for tx in 0..box_w {
            let sx = (tx as u32 * n / box_w as u32).min(n - 1);
            let sy = (ty as u32 * n / box_w as u32).min(logo.height() - 1);
            let p = logo.get_pixel(sx, sy).0;
            let luma = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
            let sat = *p.iter().max().unwrap() as i32 - *p.iter().min().unwrap() as i32;
            if luma < 200 || sat > 40 {
                let (gx, gy) = (bx + tx, by + ty);
                if gx >= 0 && gx < iw && gy >= 0 && gy < ih {
                    img.put_pixel(gx as u32, gy as u32, px);
                }
            }
        }
    }
}

#[async_trait]
impl EinkRenderer for EinkSportMatrix {
    type Data = SportData;

    fn id(&self) -> &'static str {
        "sport"
    }

    fn cycle_duration(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn render(&mut self, display: &mut EinkDisplay, data: &SportData) -> Result<(), RenderError> {
        // Off-season + opted out: don't repaint. Leave the previous module's
        // content on the panel and let the scheduler advance.
        if data.is_offseason() && !self.show_offseason {
            return Ok(());
        }
        // Kick off logo fetches in the background — never block the panel on
        // the network. `frame` draws the abbreviation box until a logo lands.
        // A current game prefetches both sides' crests; the off-season card
        // prefetches the configured team badge instead.
        if let Some(game) = data.current_game(Local::now()) {
            self.prefetch_logo(&game.home);
            self.prefetch_logo(&game.away);
        } else if data.is_offseason() {
            self.prefetch_offseason_logo(data);
        }
        let img = self.frame(data, display.width(), display.height());
        display.show(&img);
        tokio::time::sleep(self.cycle_duration()).await;
        Ok(())
    }
}

/// Square size every cached logo is normalized to.
const LOGO_PX: u32 = 120;

/// On-disk cache directory for processed logos. Honors `XDG_CACHE_HOME`, then
/// `$HOME/.cache`, then `/tmp`. Team logos never change, so once a processed
/// crest is written here it's reused across restarts with no network at all.
fn logo_cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("ohmyoled").join("logos")
}

/// Stable cache path for a logo URL at the current size. `DefaultHasher::new()`
/// is fixed-seed (not `RandomState`), so the name is deterministic across runs.
fn logo_cache_path(url: &str) -> PathBuf {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    LOGO_PX.hash(&mut h);
    logo_cache_dir().join(format!("{:016x}.png", h.finish()))
}

/// Load a processed logo: disk cache first (static, so a hit means zero
/// network), otherwise HTTP fetch → decode → resize → composite, then write it
/// back to disk so every subsequent run (this team, forever) is a disk hit.
async fn fetch_logo(url: &str) -> Result<RgbImage, String> {
    let path = logo_cache_path(url);

    // Disk hit — reuse the already-processed crest, no network.
    let read_path = path.clone();
    if let Ok(Ok(img)) = tokio::task::spawn_blocking(move || {
        image::open(&read_path).map(|i| i.to_rgb8())
    })
    .await
    {
        log::debug!("eink sport: logo disk-cache hit for {url}");
        return Ok(img);
    }

    let bytes = shared_client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("http: {e}"))?
        .error_for_status()
        .map_err(|e| format!("http status: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("body: {e}"))?;
    let owned = bytes.to_vec();
    tokio::task::spawn_blocking(move || {
        let dyn_img = image::load_from_memory(&owned).map_err(|e| format!("decode: {e}"))?;
        let rgba = dyn_img.resize_exact(LOGO_PX, LOGO_PX, FilterType::Lanczos3).to_rgba8();
        let mut out = RgbImage::from_pixel(LOGO_PX, LOGO_PX, image::Rgb([255, 255, 255]));
        for (x, y, p) in rgba.enumerate_pixels() {
            let a = p[3] as f32 / 255.0;
            let blend = |c: u8| (c as f32 * a + 255.0 * (1.0 - a)).round() as u8;
            out.put_pixel(x, y, image::Rgb([blend(p[0]), blend(p[1]), blend(p[2])]));
        }
        // Persist the processed crest (best-effort — a cache write failure must
        // never fail the render; we just re-fetch next time).
        if let Some(dir) = path.parent() {
            let saved = std::fs::create_dir_all(dir)
                .map_err(|e| e.to_string())
                .and_then(|_| out.save(&path).map_err(|e| e.to_string()));
            if let Err(e) = saved {
                log::debug!("eink sport: logo cache write failed ({}): {e}", path.display());
            }
        }
        Ok(out)
    })
    .await
    .map_err(|e| format!("decode task panicked: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sport::model::{HomeOrAway, SportApiSource, SportKind, StandingsEntry, TeamSide};
    use chrono::Local;

    fn repo_fonts() -> EinkSportFonts {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
        EinkSportFonts { body: base.join("04B_03B_.TTF") }
    }

    fn side(name: &str, abbr: &str, score: Option<i32>) -> TeamSide {
        TeamSide { name: name.into(), abbreviation: abbr.into(), logo_url: None, score }
    }

    fn standings() -> Vec<StandingsEntry> {
        vec![
            StandingsEntry { position: 1, team_name: "Celtics".into() },
            StandingsEntry { position: 2, team_name: "76ers".into() },
            StandingsEntry { position: 3, team_name: "Knicks".into() },
        ]
    }

    fn live() -> SportData {
        SportData {
            api: SportApiSource::Espn,
            sport: SportKind::Basketball,
            team_name: "76ers".into(),
            record: "41-21".into(),
            next_game: Some(NextGame {
                start: Local::now(),
                status: GameStatus::InProgress,
                home: side("76ers", "PHI", Some(88)),
                away: side("Celtics", "BOS", Some(81)),
                our_side: HomeOrAway::Home,
                playoff_note: Some("EAST 1ST ROUND - Game 5 · Series tied 2-2".into()),
            }),
            standings: standings(),
        }
    }

    #[test]
    fn frame_has_dimensions_and_lit_pixels() {
        let r = EinkSportMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let img = r.frame(&live(), 800, 480);
        assert_eq!(img.dimensions(), (800, 480));
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "expected a populated scoreboard, got {lit} lit px");
    }

    #[test]
    fn offseason_renders() {
        let r = EinkSportMatrix::with_fonts(repo_fonts(), (800, 480)).unwrap();
        let off = SportData {
            api: SportApiSource::Espn,
            sport: SportKind::Basketball,
            team_name: "76ers".into(),
            record: "—".into(),
            next_game: None,
            standings: vec![],
        };
        assert!(off.is_offseason());
        let img = r.frame(&off, 800, 480);
        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 200, "off-season must still render a card");
    }

    #[test]
    fn adapts_to_smaller_panel() {
        let r = EinkSportMatrix::with_fonts(repo_fonts(), (400, 300)).unwrap();
        let img = r.frame(&live(), 400, 300);
        assert_eq!(img.dimensions(), (400, 300));
        assert!(img.pixels().any(|p| p.0 != [0, 0, 0]));
    }
}
