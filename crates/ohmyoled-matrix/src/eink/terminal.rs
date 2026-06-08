//! Terminal backend for [`super::EinkDisplay`] — the always-available dev
//! backend that lets you iterate on e-paper layouts with no hardware.
//!
//! E-paper panels are large (a 7.5" is 800×480) and the renderers author text
//! at that native size. Faking those pixels with Unicode block/braille glyphs
//! never reads cleanly — a whole panel crammed into ~40 character rows starves
//! small text of resolution, and most terminals (VS Code's especially) leave
//! faint gaps between character rows, so block mosaics look striped. So the
//! default here draws a **real inline image** via the Sixel graphics protocol:
//! pixel-exact, font-independent, identical to the panel output.
//!
//! Three render modes, picked with `OHMYOLED_EINK_RENDER`:
//!
//! | mode     | what it does                                                   |
//! |----------|----------------------------------------------------------------|
//! | `sixel`  | inline Sixel image (**default**) + a refreshed PNG fallback     |
//! | `png`    | only refresh the PNG each frame (open it in an image viewer)    |
//! | `glyphs` | the legacy Unicode block/braille mosaic (`OHMYOLED_EINK_GLYPHS`)|
//!
//! In `sixel`/`png` mode each frame is also written as a full-resolution PNG to
//! `OHMYOLED_EINK_PNG` (or a temp file) — open it in VS Code and it live-reloads
//! on every refresh, the bulletproof fallback for terminals without Sixel. For
//! Sixel in VS Code, enable the `terminal.integrated.enableImages` setting.
//!
//! # Hardware emulation
//!
//! By default the preview flushes instantly — great for fast layout iteration,
//! but dishonest about the panel, which full-refreshes slowly with a black/white
//! flash and *cannot* animate. Set `OHMYOLED_EINK_EMULATE=1` to make every flush
//! play that full-refresh strobe and dwell (~2.5 s, tune with
//! `OHMYOLED_EINK_REFRESH_MS`) before settling on the frame — so what you see in
//! the terminal matches how the e-ink panel actually updates. It deliberately
//! blocks during the "refresh," exactly as the real SPI flush does.
//!
//! Ink pixels (the drawn foreground) render black on a white sheet, matching the
//! panel.

use super::EinkBackend;
use std::io::Write;
use std::path::PathBuf;

/// Fallback terminal width when no size can be detected.
const DEFAULT_COLS: u32 = 80;
/// Fallback terminal height when no size can be detected.
const DEFAULT_ROWS: u32 = 48;

/// A pixel of a downsampled cell counts as ink once it crosses this coverage
/// fraction of the source block (`num/den`). Below ½ so thin strokes render
/// bold and connected rather than eroded.
const INK_NUM: u32 = 2;
const INK_DEN: u32 = 5;

/// Number of black/white inversions in the emulated full-refresh flash. A real
/// Waveshare full refresh strobes the panel several times before settling.
const EMU_FLASH_FRAMES: usize = 4;
/// Default emulated full-refresh duration (ms). A panel full refresh is
/// dominated by a fixed waveform, so it barely varies with resolution — a flat
/// default reads truer than scaling by pixel count. Override with
/// `OHMYOLED_EINK_REFRESH_MS`.
const EMU_DEFAULT_REFRESH_MS: u64 = 2500;

/// `OHMYOLED_EINK_EMULATE` as a tri-state: `Some(true/false)` when set,
/// `None` when unset (so a config default can apply).
fn emulate_from_env() -> Option<bool> {
    std::env::var("OHMYOLED_EINK_EMULATE").ok().map(|s| {
        matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes" | "hardware" | "hw"
        )
    })
}

/// `OHMYOLED_EINK_REFRESH_MS` parsed to ms, or `None` when unset/invalid.
fn refresh_ms_from_env() -> Option<u64> {
    std::env::var("OHMYOLED_EINK_REFRESH_MS")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
}

/// Top-level rendering strategy.
#[derive(Clone, Copy, PartialEq)]
enum RenderMode {
    Sixel,
    Png,
    Glyphs,
}

impl RenderMode {
    fn from_env() -> Self {
        match std::env::var("OHMYOLED_EINK_RENDER")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("png") => RenderMode::Png,
            Some("glyph") | Some("glyphs") | Some("text") => RenderMode::Glyphs,
            _ => RenderMode::Sixel,
        }
    }
}

/// Terminal (dev) backend for e-paper.
pub struct EinkTerminalBackend {
    mode: RenderMode,
    /// Explicit `OHMYOLED_EINK_PNG` path, if set.
    png_path: Option<String>,
    /// Default PNG path used as the refresh target in sixel/png mode.
    fallback_png: PathBuf,
    /// Whether the one-time "where to look" hint has been printed.
    hinted: bool,
    /// Mimic the hardware: play the full-refresh flash + dwell on every flush
    /// so the preview behaves like a real panel. Off by default (instant) so
    /// layout iteration stays fast; enable with `OHMYOLED_EINK_EMULATE`.
    emulate: bool,
    /// Emulated full-refresh duration in ms (`OHMYOLED_EINK_REFRESH_MS`).
    refresh_ms: u64,
}

impl Default for EinkTerminalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EinkTerminalBackend {
    /// Build with no config defaults — emulation is env-driven only (the
    /// preview path, which has no config block).
    pub fn new() -> Self {
        Self::with_config(None, None)
    }

    /// Build with config-provided emulation defaults. The env vars
    /// (`OHMYOLED_EINK_EMULATE`, `OHMYOLED_EINK_REFRESH_MS`) take precedence so
    /// a config default can still be overridden at runtime.
    pub fn with_config(emulate_cfg: Option<bool>, refresh_ms_cfg: Option<u64>) -> Self {
        let emulate = emulate_from_env().or(emulate_cfg).unwrap_or(false);
        let refresh_ms = refresh_ms_from_env()
            .or(refresh_ms_cfg)
            .unwrap_or(EMU_DEFAULT_REFRESH_MS);
        Self {
            mode: RenderMode::from_env(),
            png_path: std::env::var("OHMYOLED_EINK_PNG").ok().filter(|s| !s.is_empty()),
            fallback_png: std::env::temp_dir().join("ohmyoled-eink-preview.png"),
            hinted: false,
            emulate,
            refresh_ms,
        }
    }

    /// Play the hardware full-refresh flash: strobe the panel black/white a few
    /// times, dwelling so the whole sequence takes ~`refresh_ms`. The caller
    /// paints the settled image immediately after, so the flash reads as the
    /// panel "developing" the new frame. Blocks the calling thread on purpose —
    /// the real SPI flush blocks for seconds too, so the preview feels honest.
    fn play_full_refresh(&self, stride: usize, width: u32, height: u32) {
        let frames = EMU_FLASH_FRAMES.max(2);
        let per = std::time::Duration::from_millis(self.refresh_ms / frames as u64);
        let black = vec![0x00u8; stride * height as usize];
        let white = vec![0xFFu8; stride * height as usize];
        for i in 0..frames {
            let buf = if i % 2 == 0 { &black } else { &white };
            self.emit_frame(buf, stride, width, height);
            std::thread::sleep(per);
        }
    }

    /// Paint one packed frame to the terminal with no PNG write / hint line —
    /// used for the transient flash frames of [`Self::play_full_refresh`].
    fn emit_frame(&self, packed: &[u8], stride: usize, width: u32, height: u32) {
        let mut out = std::io::stdout();
        match self.mode {
            RenderMode::Sixel => {
                let (tw, th) = sixel_target(width, height);
                let sixel = encode_sixel(packed, stride, width, height, tw, th);
                let mut buf = String::from("\x1b[2J\x1b[H");
                buf.push_str(&sixel);
                let _ = out.write_all(buf.as_bytes());
            }
            RenderMode::Glyphs => {
                let s = render_glyphs(packed, stride, width, height);
                let _ = out.write_all(b"\x1b[2J\x1b[H");
                let _ = out.write_all(s.as_bytes());
            }
            // In PNG mode there's no inline image to strobe; rewrite the file so
            // a live-reloading viewer flickers along with the refresh.
            RenderMode::Png => write_png(self.png_target(), packed, stride, width, height),
        }
        let _ = out.flush();
    }

    /// Is the pixel at (x, y) inked (bit clear)? `packed` is MSB-first, stride
    /// bytes/row; bit `1` = white sheet, bit `0` = black ink.
    fn is_ink(packed: &[u8], stride: usize, x: u32, y: u32) -> bool {
        let byte = y as usize * stride + (x / 8) as usize;
        let mask = 0x80u8 >> (x % 8);
        // Out-of-range bytes read as white (no ink).
        packed.get(byte).map(|b| b & mask == 0).unwrap_or(false)
    }

    /// The PNG path frames are refreshed to (explicit override, else temp file).
    fn png_target(&self) -> &str {
        self.png_path
            .as_deref()
            .unwrap_or_else(|| self.fallback_png.to_str().unwrap_or("ohmyoled-eink-preview.png"))
    }
}

impl EinkBackend for EinkTerminalBackend {
    fn flush(&mut self, packed: &[u8], width: u32, height: u32) {
        let stride = width.div_ceil(8) as usize;
        if self.emulate {
            self.play_full_refresh(stride, width, height);
        }
        match self.mode {
            RenderMode::Sixel => {
                let (tw, th) = sixel_target(width, height);
                let sixel = encode_sixel(packed, stride, width, height, tw, th);
                let path = self.png_target().to_string();
                write_png(&path, packed, stride, width, height);
                let mut buf = String::from("\x1b[2J\x1b[H");
                buf.push_str(&sixel);
                buf.push('\n');
                buf.push_str(&format!(
                    "\x1b[2m[eink {width}x{height} sixel {tw}x{th} · no image? set VS Code terminal.integrated.enableImages or OHMYOLED_EINK_RENDER=png · PNG: {path}]\x1b[0m\n"
                ));
                let _ = std::io::stdout().write_all(buf.as_bytes());
                let _ = std::io::stdout().flush();
            }
            RenderMode::Png => {
                let path = self.png_target().to_string();
                write_png(&path, packed, stride, width, height);
                let mut buf = String::from("\x1b[2J\x1b[H");
                buf.push_str(&format!(
                    "eink preview → {path}\n\x1b[2m{width}x{height} · open it in VS Code (it live-reloads on each refresh)\x1b[0m\n"
                ));
                let _ = std::io::stdout().write_all(buf.as_bytes());
                let _ = std::io::stdout().flush();
            }
            RenderMode::Glyphs => {
                let s = render_glyphs(packed, stride, width, height);
                let _ = std::io::stdout().write_all(s.as_bytes());
                let _ = std::io::stdout().flush();
                if let Some(path) = &self.png_path {
                    write_png(path, packed, stride, width, height);
                }
            }
        }

        if !self.hinted {
            self.hinted = true;
            let emu = if self.emulate {
                format!("; hardware emulation ON (~{} ms full-refresh flash)", self.refresh_ms)
            } else {
                String::new()
            };
            log::info!(
                "eink preview: {} mode; PNG refreshed to {}{}",
                match self.mode {
                    RenderMode::Sixel => "sixel",
                    RenderMode::Png => "png",
                    RenderMode::Glyphs => "glyphs",
                },
                self.png_target(),
                emu
            );
        }
    }

    fn clear(&mut self) {
        let _ = std::io::stdout().write_all(b"\x1b[2J\x1b[H");
        let _ = std::io::stdout().flush();
    }
}

// ── Sixel ────────────────────────────────────────────────────────────────

/// Query the terminal's pixel dimensions via `TIOCGWINSZ` (`ws_xpixel` /
/// `ws_ypixel`). Returns `None` if unavailable or unreported (many terminals
/// leave these zero).
#[cfg(unix)]
fn tty_pixels() -> Option<(u32, u32)> {
    use std::os::unix::io::AsRawFd;
    let fd = std::io::stdout().as_raw_fd();
    // SAFETY: zeroed winsize is valid; ioctl only writes into it.
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
    if rc == 0 && ws.ws_xpixel > 0 && ws.ws_ypixel > 0 {
        Some((ws.ws_xpixel as u32, ws.ws_ypixel as u32))
    } else {
        None
    }
}

#[cfg(not(unix))]
fn tty_pixels() -> Option<(u32, u32)> {
    None
}

/// On-screen pixel size for the Sixel image: fit within the terminal's pixel
/// area (preserving aspect), or fall back to native size when the terminal
/// doesn't report pixels. `OHMYOLED_EINK_ZOOM` scales the result.
fn sixel_target(width: u32, height: u32) -> (u32, u32) {
    let (avail_w, avail_h) = match tty_pixels() {
        Some((w, h)) => ((w as f32 * 0.98), (h as f32 * 0.88)),
        None => (width.max(720) as f32, height.max(432) as f32),
    };
    let zoom = std::env::var("OHMYOLED_EINK_ZOOM")
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .filter(|z| *z > 0.0)
        .unwrap_or(1.0);
    let fit = (avail_w / width as f32).min(avail_h / height as f32);
    let s = (fit * zoom).clamp(0.2, 4.0);
    (
        ((width as f32 * s).round() as u32).max(1),
        ((height as f32 * s).round() as u32).max(1),
    )
}

/// Encode the packed 1-bpp frame as a Sixel image scaled to `tw × th`, black ink
/// on a white sheet.
fn encode_sixel(packed: &[u8], stride: usize, width: u32, height: u32, tw: u32, th: u32) -> String {
    // Area-sampled ink test for one target pixel — keeps thin strokes on
    // downscale, exact on upscale.
    let ink_at = |x: u32, y: u32| -> bool {
        let x0 = x * width / tw;
        let x1 = ((x + 1) * width).div_ceil(tw).min(width).max(x0 + 1);
        let y0 = y * height / th;
        let y1 = ((y + 1) * height).div_ceil(th).min(height).max(y0 + 1);
        dot_inked(packed, stride, x0, x1, y0, y1)
    };

    let mut out = String::with_capacity(8 * 1024);
    out.push_str("\x1bPq"); // DCS — enter Sixel
    out.push_str("#0;2;100;100;100"); // palette 0: white sheet
    out.push_str("#1;2;0;0;0"); // palette 1: black ink

    for band in 0..th.div_ceil(6) {
        // Two passes: white sheet (select=false), then black ink (select=true).
        for (color, select) in [(0u8, false), (1u8, true)] {
            if let Some(run) = band_runs(tw, th, band, select, &ink_at) {
                out.push('#');
                out.push_str(itoa(color).as_str());
                out.push_str(&run);
                out.push('$'); // graphics CR — overlay next color on this band
            }
        }
        out.push('-'); // graphics NL — next 6-row band
    }
    out.push_str("\x1b\\"); // ST — leave Sixel
    out
}

/// Build the RLE Sixel run for one (band, color), selecting pixels whose ink
/// value equals `select`. Returns `None` when the band has no such pixels.
fn band_runs(tw: u32, th: u32, band: u32, select: bool, ink_at: &impl Fn(u32, u32) -> bool) -> Option<String> {
    let mut s = String::new();
    let mut prev: Option<u8> = None;
    let mut count = 0u32;
    let mut any = false;
    for x in 0..tw {
        let mut bits = 0u8;
        for r in 0..6u32 {
            let y = band * 6 + r;
            if y < th && ink_at(x, y) == select {
                bits |= 1 << r;
            }
        }
        if bits != 0 {
            any = true;
        }
        if Some(bits) == prev {
            count += 1;
        } else {
            push_run(&mut s, prev, count);
            prev = Some(bits);
            count = 1;
        }
    }
    push_run(&mut s, prev, count);
    any.then_some(s)
}

/// Append one run of `count` copies of the Sixel char for `bits`, RLE-compressed.
fn push_run(s: &mut String, bits: Option<u8>, count: u32) {
    let Some(b) = bits else { return };
    if count == 0 {
        return;
    }
    let ch = char::from_u32(0x3F + b as u32).unwrap_or('?');
    if count >= 4 {
        s.push('!');
        s.push_str(itoa(count).as_str());
        s.push(ch);
    } else {
        for _ in 0..count {
            s.push(ch);
        }
    }
}

/// Tiny allocation-light unsigned-int formatter (avoids pulling a crate).
fn itoa(n: impl Into<u64>) -> String {
    n.into().to_string()
}

// ── Shared downsampling ────────────────────────────────────────────────────

/// Decide whether a source block reads as ink: a low coverage threshold (so
/// strokes stay bold) with a thin-line rescue (any fully-inked row or column),
/// so 1px rules survive the shrink.
fn dot_inked(packed: &[u8], stride: usize, x0: u32, x1: u32, y0: u32, y1: u32) -> bool {
    let total = ((x1 - x0) * (y1 - y0)).max(1);
    let mut ink = 0u32;
    for y in y0..y1 {
        let mut row_all = true;
        for x in x0..x1 {
            if EinkTerminalBackend::is_ink(packed, stride, x, y) {
                ink += 1;
            } else {
                row_all = false;
            }
        }
        if row_all {
            return true;
        }
    }
    for x in x0..x1 {
        if (y0..y1).all(|y| EinkTerminalBackend::is_ink(packed, stride, x, y)) {
            return true;
        }
    }
    ink * INK_DEN >= total * INK_NUM
}

// ── PNG ────────────────────────────────────────────────────────────────────

/// Dump the packed frame as a full-resolution grayscale PNG (ink = black,
/// sheet = white).
fn write_png(path: &str, packed: &[u8], stride: usize, width: u32, height: u32) {
    let mut img = image::GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let ink = EinkTerminalBackend::is_ink(packed, stride, x, y);
            img.put_pixel(x, y, image::Luma([if ink { 0 } else { 255 }]));
        }
    }
    if let Err(e) = img.save(path) {
        log::warn!("eink: failed to write preview PNG to '{path}': {e}");
    }
}

// ── Legacy glyph mosaic (OHMYOLED_EINK_RENDER=glyphs) ───────────────────────

/// Sub-cell encoding for the legacy text mosaic.
#[derive(Clone, Copy)]
enum GlyphMode {
    Braille,
    Sextant,
    Quadrant,
    Half,
}

impl GlyphMode {
    fn from_env() -> Self {
        match std::env::var("OHMYOLED_EINK_GLYPHS")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("sextant") | Some("sextants") => GlyphMode::Sextant,
            Some("quadrant") | Some("quadrants") | Some("quad") => GlyphMode::Quadrant,
            Some("half") | Some("halfblock") | Some("half-block") => GlyphMode::Half,
            _ => GlyphMode::Braille,
        }
    }

    fn dots(self) -> (u32, u32) {
        match self {
            GlyphMode::Braille => (2, 4),
            GlyphMode::Sextant => (2, 3),
            GlyphMode::Quadrant => (2, 2),
            GlyphMode::Half => (1, 2),
        }
    }

    fn glyph(self, bits: u8) -> char {
        match self {
            GlyphMode::Half => [' ', '\u{2580}', '\u{2584}', '\u{2588}'][(bits & 0b11) as usize],
            GlyphMode::Quadrant => QUADRANTS[(bits & 0b1111) as usize],
            GlyphMode::Sextant => sextant(bits & 0b111111),
            GlyphMode::Braille => {
                let mut b = 0u8;
                for i in 0..8 {
                    if bits & (1 << i) != 0 {
                        b |= BRAILLE_BITS[(i / 2) as usize][(i % 2) as usize];
                    }
                }
                char::from_u32(0x2800 + b as u32).unwrap_or(' ')
            }
        }
    }
}

const QUADRANTS: [char; 16] = [
    ' ', '\u{2598}', '\u{259D}', '\u{2580}', '\u{2596}', '\u{258C}', '\u{259E}', '\u{259B}',
    '\u{2597}', '\u{259A}', '\u{2590}', '\u{259C}', '\u{2584}', '\u{2599}', '\u{259F}', '\u{2588}',
];

const BRAILLE_BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

fn sextant(p: u8) -> char {
    match p {
        0 => return ' ',
        0b010101 => return '\u{258C}',
        0b101010 => return '\u{2590}',
        0b111111 => return '\u{2588}',
        _ => {}
    }
    let skipped = (p > 0b010101) as u32 + (p > 0b101010) as u32;
    char::from_u32(0x1FB00 + p as u32 - 1 - skipped).unwrap_or(' ')
}

fn env_dim(key: &str, floor: u32) -> Option<u32> {
    std::env::var(key)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&c| c >= floor)
}

#[cfg(unix)]
fn tty_cells() -> Option<(u32, u32)> {
    use std::os::unix::io::AsRawFd;
    let fd = std::io::stdout().as_raw_fd();
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
    if rc == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        Some((ws.ws_col as u32, ws.ws_row as u32))
    } else {
        None
    }
}

#[cfg(not(unix))]
fn tty_cells() -> Option<(u32, u32)> {
    None
}

fn target_cells(panel_width: u32) -> (u32, u32) {
    let tty = tty_cells();
    let cols = env_dim("OHMYOLED_EINK_COLS", 16)
        .or_else(|| tty.map(|(c, _)| c).filter(|&c| c >= 16))
        .or_else(|| env_dim("COLUMNS", 16))
        .unwrap_or(DEFAULT_COLS)
        .min(panel_width);
    let rows = env_dim("OHMYOLED_EINK_ROWS", 8)
        .or_else(|| tty.map(|(_, r)| r.saturating_sub(1)).filter(|&r| r >= 8))
        .unwrap_or(DEFAULT_ROWS);
    (cols.max(16), rows.max(8))
}

fn render_glyphs(packed: &[u8], stride: usize, width: u32, height: u32) -> String {
    let mode = GlyphMode::from_env();
    let (dots_x, dots_y) = mode.dots();
    let (max_cols, max_rows) = target_cells(width);
    let max_dw = (max_cols * dots_x).max(dots_x);
    let max_dh = (max_rows * dots_y).max(dots_y);
    let scale = (width as f32 / max_dw as f32)
        .max(height as f32 / max_dh as f32)
        .max(1.0);
    let dot_w = ((width as f32 / scale).round() as u32).max(1);
    let dot_h = ((height as f32 / scale).round() as u32).max(1);

    let mut grid = vec![false; (dot_w * dot_h) as usize];
    for dy in 0..dot_h {
        let y0 = (dy as f32 * scale) as u32;
        let y1 = (((dy + 1) as f32 * scale).ceil() as u32).min(height).max(y0 + 1);
        for dx in 0..dot_w {
            let x0 = (dx as f32 * scale) as u32;
            let x1 = (((dx + 1) as f32 * scale).ceil() as u32).min(width).max(x0 + 1);
            grid[(dy * dot_w + dx) as usize] = dot_inked(packed, stride, x0, x1, y0, y1);
        }
    }
    let at = |x: u32, y: u32| -> bool { x < dot_w && y < dot_h && grid[(y * dot_w + x) as usize] };

    let cols = dot_w.div_ceil(dots_x);
    let rows = dot_h.div_ceil(dots_y);
    let mut buf = String::from("\x1b[2J\x1b[H");
    for cr in 0..rows {
        for cc in 0..cols {
            let mut bits = 0u8;
            for sy in 0..dots_y {
                for sx in 0..dots_x {
                    if at(cc * dots_x + sx, cr * dots_y + sy) {
                        bits |= 1 << (sy * dots_x + sx);
                    }
                }
            }
            buf.push(mode.glyph(bits));
        }
        buf.push('\n');
    }
    buf.push_str("\x1b[2m[eink glyph mosaic · OHMYOLED_EINK_RENDER=sixel for a real image]\x1b[0m\n");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sextant_endpoints_map_to_block_elements() {
        assert_eq!(sextant(0), ' ');
        assert_eq!(sextant(0b111111), '\u{2588}');
        assert_eq!(sextant(0b010101), '\u{258C}');
        assert_eq!(sextant(0b101010), '\u{2590}');
        assert_eq!(sextant(0b000001), '\u{1FB00}');
    }

    #[test]
    fn sixel_wraps_with_dcs_and_st() {
        // All-ink 8x8 frame (every bit 0).
        let packed = vec![0u8; 8];
        let s = encode_sixel(&packed, 1, 8, 8, 8, 8);
        assert!(s.starts_with("\x1bPq"), "starts with DCS + q");
        assert!(s.ends_with("\x1b\\"), "ends with ST");
        assert!(s.contains("#1"), "uses the ink palette entry");
    }

    #[test]
    fn sixel_target_preserves_aspect() {
        // With no tty pixels reported, falls back to native-ish size.
        let (w, h) = sixel_target(800, 480);
        let ar = w as f32 / h as f32;
        assert!((ar - 800.0 / 480.0).abs() < 0.05, "aspect preserved, got {ar}");
    }
}
