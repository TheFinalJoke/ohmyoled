# CLAUDE.md — ohmyoled

Working guide for contributors and Claude. The README is the user-facing
doc; this file is the **developer-facing** companion: how the codebase is
shaped, the conventions we've settled on, and the mechanical recipe for
extending it.

---

## Project shape

```
src/
├── main.rs                   # CLI + config loading + scheduler bootstrap
├── config_io.rs              # JSON/YAML/TOML loader/writer
├── filelib.rs                # file-exists check
├── createjson/               # `-c` config builder (full-screen ratatui TUI in tui/)
└── lib/
    ├── lib.rs                # crate root (oledlib)
    ├── api/
    │   ├── collector.rs      # `Collector` trait
    │   ├── http.rs           # `shared_client()` + `get_json()`
    │   ├── error.rs          # `ApiError`
    │   ├── weather/          # one module per provider
    │   ├── stock/
    │   ├── sport/
    │   ├── golf/
    │   └── f1/
    ├── matrix/
    │   ├── renderer.rs       # `Renderer` trait
    │   ├── error.rs          # `RenderError`
    │   ├── time.rs           # canonical minimal example
    │   ├── weather.rs
    │   ├── stock.rs
    │   ├── sport.rs
    │   ├── golf.rs
    │   └── f1.rs
    ├── modules/
    │   ├── mod.rs            # `Module<C, R>` + `DynModule`
    │   ├── registry.rs       # ⭐ wiring site for every API/renderer pair
    │   ├── scheduler.rs      # round-robin async runner
    │   └── error.rs
    ├── serde_helpers.rs      # `one_or_many`, `null_string_as_none`, `zero_as_none`
    └── teams/                # hardcoded team rosters + Logo struct

crates/ohmyoled-matrix/       # panel-level abstractions (Color, Font, draw_*)
examples/
├── configs/                  # ohmyoled.{json,yaml,toml} — drop-in samples
├── config_formats.rs         # round-trip check across all three formats
└── multi_instance_check.rs   # single-or-array config shape verifier

# Visual smoke tests live in src/preview.rs — run via `ohmyoled --preview <name>`.
# Don't add new per-module `*_render_check.rs` examples; wire the renderer
# into preview.rs instead so it shows up under the same flag.
```

The split between `oledlib` (`src/lib/`) and the `ohmyoled` binary
(`src/main.rs`) is load-bearing: anything reusable lives in the lib;
`main.rs` only does argument parsing + scheduler bootstrap.

---

## The two contracts

Every module is a `Module<C: Collector, R: Renderer<Data = C::Output>>`.
The data type C produces flows directly into R — that's the entire
interface.

```rust
#[async_trait]
pub trait Collector: Send + Sync {
    type Output: Send + 'static;
    fn id(&self) -> &'static str;
    fn refresh_interval(&self) -> Duration;
    async fn poll(&self) -> Result<Self::Output, ApiError>;
}

#[async_trait]
pub trait Renderer: Send {
    type Data: Send + 'static;
    fn id(&self) -> &'static str;
    fn cycle_duration(&self) -> Duration;
    async fn render(
        &mut self,
        matrix: &mut RGBMatrix,
        data: &Self::Data,
    ) -> Result<(), RenderError>;
}
```

**`refresh_interval`** is the per-module poll cadence — e.g. weather
600 s, stock 30 s, time 1 s. A background `tokio::spawn`'d task owns
the collector, calls `poll().await`, publishes the result via
`tokio::sync::watch::channel::<Option<Arc<Output>>>`, then sleeps for
the interval. The scheduler's render loop reads the latest value via
`watch::Receiver::borrow()` — it never blocks on the network. Each
section can override the cadence with `cache_ttl_secs: <int>` in
config: `Some(n > 0)` ⇒ override the collector's default; `Some(0)`
⇒ no background task, the renderer polls inline on every render
(always fresh, panel briefly stalls during the round-trip); `None`
(omitted) ⇒ use `refresh_interval()`.

**`cycle_duration`** is the renderer's contract for how long one full
render cycle takes (e.g. weather 65 s for both screens + scroll +
dwell). The scheduler uses this to budget panel time fairly.

Read `src/lib/matrix/time.rs` end-to-end before adding anything new —
it's the canonical minimal example for both traits.

---

## Recipe: add a new API + matrix

Worked example: add a "subway" module that polls MTA arrivals and
displays the next train.

### 1. Add the collector — `src/lib/api/subway/`

```
src/lib/api/subway/
├── mod.rs       # re-exports + `SubwaySource` enum + `SubwayCollector`
├── model.rs     # normalized `Arrival` struct (Deserialize)
└── mta.rs       # `MtaClient { cfg }` + provider-specific JSON shapes
```

Pattern from `src/lib/api/weather/mod.rs`:

```rust
// model.rs — what the renderer consumes
#[derive(Debug, Clone)]
pub struct Arrival {
    pub line: String,
    pub destination: String,
    pub minutes_away: u32,
}

// mta.rs — provider-specific
pub struct MtaConfig { pub station_id: String }
pub struct MtaClient { cfg: MtaConfig }
impl MtaClient {
    pub fn new(cfg: MtaConfig) -> Result<Self, ApiError> { /* … */ }
    pub async fn poll(&self) -> Result<Vec<Arrival>, ApiError> { /* … */ }
}

// mod.rs — provider dispatch + the Collector impl
pub enum SubwaySource { Mta(MtaClient) }
pub struct SubwayCollector { source: SubwaySource }
impl SubwayCollector {
    pub fn from_mta(cfg: MtaConfig) -> Result<Self, ApiError> { /* … */ }
}

#[async_trait]
impl Collector for SubwayCollector {
    type Output = Vec<Arrival>;
    fn id(&self) -> &'static str { "subway" }
    fn refresh_interval(&self) -> Duration { Duration::from_secs(30) }
    async fn poll(&self) -> Result<Vec<Arrival>, ApiError> {
        match &self.source { SubwaySource::Mta(c) => c.poll().await }
    }
}
```

Then add `pub mod subway;` to `src/lib/api/mod.rs`.

### 2. Add the renderer — `src/lib/matrix/subway.rs`

Pattern from any of `golf.rs`/`f1.rs`/`stock.rs`. Key points:

```rust
pub struct SubwayFonts { pub body: PathBuf }
impl Default for SubwayFonts {
    fn default() -> Self { Self { body: "/usr/share/fonts/04B_03B_.TTF".into() } }
}

pub struct SubwayMatrix { body_font: Font }

impl SubwayMatrix {
    // sync constructor — useful for tests
    pub fn new() -> Result<Self, String> { Self::with_fonts(SubwayFonts::default()) }

    // async constructor — what the registry actually calls
    pub async fn new_async() -> Result<Self, String> {
        Self::with_fonts_async(SubwayFonts::default()).await
    }

    pub fn with_fonts(paths: SubwayFonts) -> Result<Self, String> {
        Ok(Self { body_font: Font::load_ttf(&paths.body, 8.0)? })
    }
    pub async fn with_fonts_async(paths: SubwayFonts) -> Result<Self, String> {
        tokio::task::spawn_blocking(move || Self::with_fonts(paths))
            .await
            .map_err(|e| format!("font load task panicked: {e}"))?
    }

    fn frame(&self, data: &[Arrival]) -> RgbImage {
        let mut img = RgbImage::new(64, 32);
        // … draw_text + Color usage; keep helpers private to this file
        img
    }
}

#[async_trait]
impl Renderer for SubwayMatrix {
    type Data = Vec<Arrival>;
    fn id(&self) -> &'static str { "subway" }
    fn cycle_duration(&self) -> Duration { Duration::from_secs(30) }
    async fn render(&mut self, matrix: &mut RGBMatrix, data: &Vec<Arrival>)
        -> Result<(), RenderError>
    {
        matrix.clear();
        let img = self.frame(data);
        matrix.set_image(&img, 0, 0);
        tokio::time::sleep(Duration::from_secs(25)).await;
        matrix.clear();
        Ok(())
    }
}
```

Then add `pub mod subway;` to `src/lib/matrix/mod.rs`.

### 3. Wire it into the registry — `src/lib/modules/registry.rs`

This is **the one-line registration site**. Add the section struct,
deserialize it via `one_or_many`, and emit modules in `build()`.

```rust
// 3a. section struct
#[derive(Debug, Deserialize)]
pub struct SubwaySection {
    pub run: bool,
    pub station_id: String,
    /// Optional override for the background poll cadence — see the
    /// "polling model" notes above. Same shape on every section.
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

// 3b. field on RegistryConfig
pub struct RegistryConfig {
    // … existing fields …
    #[serde(default, deserialize_with = "one_or_many")]
    pub subway: Vec<SubwaySection>,
}

// 3c. builder helper — `module_with_ttl` resolves cache_ttl_secs to a
// Duration (or falls back to collector.refresh_interval()) and either
// spawns a background poll task or stages an inline-poll renderer.
async fn build_subway(s: &SubwaySection) -> Result<Box<dyn DynModule>, String> {
    let collector = SubwayCollector::from_mta(MtaConfig {
        station_id: s.station_id.clone(),
    }).map_err(|e| e.to_string())?;
    let renderer = SubwayMatrix::new_async()
        .await
        .map_err(|e| format!("subway fonts: {e}"))?;
    Ok(module_with_ttl(collector, renderer, s.cache_ttl_secs))
}

// 3d. loop in `build()`
for s in cfg.subway.iter().filter(|s| s.run) {
    match build_subway(s).await {
        Ok(m) => modules.push(m),
        Err(e) => log::error!("subway: skipping module: {e}"),
    }
}
```

### 4. Wire into `--preview` — `src/preview.rs`

The visual smoke test path for every renderer. Add a `preview_subway`
async fn that builds the renderer via `with_fonts_async`, constructs a
hand-tuned fake `Arrival` (or whatever the module's data type is), and
loops `r.render(matrix, &data).await` forever. Then:

- Add `"subway"` to `pub const NAMES: &[&str]` at the top.
- Add `"subway" => preview_subway(&mut matrix, &fonts).await,` to the
  `match name` block in `run()`.

If the renderer has multiple visual modes (off-season vs in-season,
overhead vs distance, etc.), alternate between fake data values inside
the loop so each mode gets airtime. See `preview_iss` for the pattern.

After this, `ohmyoled --preview subway` drives the renderer live against
whichever backend `RGBMatrix` resolves to (terminal in devcontainer,
panel on a Pi). **Don't add a new `examples/subway_render_check.rs`** —
`--preview` is the one place we eyeball renderers.

### 5. Wire into the config builder + starter config — `src/createjson/`

The `-c` builder is a **full-screen ratatui TUI** (`src/createjson/tui/`),
not a line-prompt flow. It's a two-screen wizard: a **Setup** screen (pick
Matrix _or_ E-ink — mutually exclusive — fill that target's options, pick
json/yaml/toml) and a **Modules** screen (toggle the applicable tiles, edit
their fields, watch a live preview). The `--init-config <path>` one-shot
still writes `default_config()`. **Both** must learn about a new module.

The TUI is a generic form engine driven by a per-module **field schema** —
you don't write any ratatui per module. Each `src/createjson/<name>.rs`
keeps its serde `Options` struct + `Default`, and adds a `fields()` schema:

```
src/createjson/
├── mod.rs          # MatrixOptions + default_config() + create_json() → tui::run()
├── subway.rs       # new: `Options` struct + `Default` + `fields()`
├── tui/
│   ├── field.rs       # FieldDef/FieldKind/Form — the pure projection engine
│   ├── form_module.rs # per-section dispatch: fields()/value_to_form()/section_to_value()
│   ├── app.rs         # wizard state (Target, ConfigFormat, Instance)
│   ├── event.rs       # key handling
│   ├── ui.rs          # ratatui rendering (the thin shell)
│   └── preview.rs     # config assembly + json/yaml/toml serialization
└── …                  # one-file-per-module pattern (struct + Default + fields)
```

Pattern from any of `iss.rs` / `flights.rs` / `pihole.rs`:

```rust
// src/createjson/subway.rs
use crate::createjson::tui::field::{FieldDef, FieldKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SubwayOptions {
    pub run: bool,
    pub station_id: String,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for SubwayOptions {
    fn default() -> Self {
        Self { run: true, station_id: "127N".into(), cache_ttl_secs: None }
    }
}

/// Field schema. `id` MUST match the serde field name — the value is
/// round-tripped through `SubwayOptions` on save, so a typo'd id is dropped.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new("station_id", "MTA station id", "e.g. 127N",
            FieldKind::Text { default: "127N" }),
        FieldDef::new("cache_ttl_secs", "Cache TTL (secs)", super::CACHE_TTL_HELP,
            FieldKind::CacheTtl),
    ]
}
```

`FieldKind` covers `Text`/`OptionalText`/`Bool`/`Number{min,max}`/`Float`/
`Enum{choices}`/`Rgb`/`StringList`/`CacheTtl`/`OptionalNumber`/`ValueEnum`.
**Conditional fields** use `.when(|f| …)` (see `weather.rs` `api_key` /
`stock.rs` `api_key`); **dependent option lists** (sport's team picker) use
`form_module::on_field_changed`. Any non-trivial normalization (stock's
symbol case-folding, time's `system` ⇒ `null`) lives in
`form_module::section_to_value`. No `configure()`, no `summary_line` — those
are gone.

Then in `src/createjson/tui/form_module.rs`, three small edits:

1. `pub mod subway;` already exists in `mod.rs`; nothing to add there for
   the schema itself.
2. Add `("subway", "Subway")` to `TILE_KINDS` and arms to `fields()`,
   `config_key()` (returns `"subway"`), and `canonicalize()` (round-trips
   `subway::SubwayOptions`). If `subway` can have multiple instances, extend
   `allow_multi()`.
3. Add `"subway"` to `SECTION_KEYS` in `app.rs` (existing-config loading) and
   `SECTION_ORDER` in `preview.rs` (assembly order).

**Also update `default_config()`** in `mod.rs` — its JSON literal is what
`--init-config` writes. Add a `"subway": {"run":false,
"station_id":"REPLACE_ME_STATION_ID","cache_ttl_secs":null}` entry. Required
keys take `REPLACE_ME_*` placeholders; optional keys take their defaults.

### 6. Update docs + `--preview` help

- Add a `# Config` block to the top of `src/lib/matrix/subway.rs` matching the
  format used by every other matrix doc-comment (layout diagram + YAML
  snippet + data source line).
- Add a `subway:` example to the three [`examples/configs/`](examples/configs/)
  files.
- Add a row to the `### subway` section of `README.md` if the module is
  user-facing.
- Append `subway` to the comma-separated list in the `--preview` help
  text in `src/main.rs` so `--help` reports the full set.

That's the whole recipe. Beyond the `--preview` help-text update,
no other changes to `main.rs` are needed, and no changes to the
scheduler.

---

## Conventions we've settled on

### Config sections

- **Always use `#[serde(default, deserialize_with = "one_or_many")]`** so
  the section can be either a single object or an array. This is
  load-bearing for multi-instance support (two stock symbols, two team
  feeds, etc.).
- **`run: bool` is required on every section.** Filtering happens in
  `registry::build()`, not in the renderer.
- **Optional strings use `null_string_as_none`** (from `serde_helpers`)
  to preserve the legacy `"null"` literal quirk from the JSON-only era.
- **Variant-bearing sections use `#[serde(tag = "...", rename_all = "lowercase")]`**
  enums — see `SportSection` for the pattern. Don't nest separate
  top-level keys for variants of the same concept.
- **Every section gets `cache_ttl_secs: Option<u64>`** with
  `#[serde(default)]`. The `module_with_ttl(collector, renderer, s.cache_ttl_secs)`
  helper in `registry.rs` turns it into a `Duration`, picking
  `collector.refresh_interval()` when omitted and treating `Some(0)`
  as "skip the background task, poll inline on every render." For
  enum-shaped sections (`SportSection`), add the field to every
  variant and expose a `cache_ttl_secs()` accessor that pulls it from
  whichever variant matched — same pattern as the existing `run()`.

### Collectors

- **One shared `reqwest` client per process** — call `crate::api::http::shared_client()`,
  don't `Client::new()` yourself. Connection pooling matters on the Pi.
- **Use `crate::api::http::get_json::<T>(url, headers).await`** when you
  just need to deserialize a single GET response. Drop down to the raw
  client only when you need POST/multipart/etc.
- **Provider dispatch is an enum, not a `dyn` trait.** All providers are
  known at compile time; two function-call hops beat trait-object
  virtual dispatch.
- **Errors → `ApiError`.** Everything is `#[from]`-convertible. Don't
  add new error variants unless a current one genuinely doesn't fit.
- **Normalize on the way out.** Each provider's `poll()` returns the
  module's normalized data type directly — don't expose provider-specific
  shapes to the renderer.
- **Cache cross-poll state inside the collector** when it's stable for
  the lifetime of the process (e.g. resolved team IDs, ipinfo lookups).
  Use `tokio::sync::OnceCell` — `std::sync::Mutex` isn't `Send` across
  await points.

### Renderers

- **Two constructors per renderer.** `new()` for tests; `new_async()`
  for the registry. The async one moves font loading onto a worker
  thread via `tokio::task::spawn_blocking`. Don't block the executor on
  font I/O.
- **Fonts are paths.** Defaults point at `/usr/share/fonts/...`; the
  install script lays them down. Renderers expose a `*Fonts` struct so
  custom paths can be plumbed in tests.
- **Keep the layout calculation in a `frame(&self, data) -> RgbImage`
  method.** The `render()` method should just be a scroll loop calling
  `frame()` + `matrix.set_image()` + `sleep()`. Tests then exercise
  `frame()` directly without an `RGBMatrix`.
- **Coordinate math uses signed integers.** Cast to `i32` before
  computing offsets — wrap-arounds with `u32` will silently produce
  enormous values.
- **Scroll loops are inlined per renderer.** Don't try to extract a
  shared `scroll_text` helper — the per-module variations (dwell time,
  reset behavior, color rules) make a shared abstraction more painful
  than the duplication.

### Tests

- **Each renderer has a `frame_has_dimensions_and_lit_pixels` test.**
  Load the repo fonts via `env!("CARGO_MANIFEST_DIR")` so it works in
  CI without the install script having run.
- **Collectors get fixture-based JSON parse tests.** Drop a captured
  response in a `FIXTURE: &str` constant and `serde_json::from_str` it.
  Don't hit the network from tests.
- **Sport renderers also need an `offseason_renders` test** — every team
  sport, golf, and F1 has months without data, and the off-season path
  is a separate code path.
- **Visual smoke testing goes through `--preview`, not example bins.**
  `ohmyoled --preview <name>` drives the renderer live with hand-tuned
  fake data; that's the one path for eyeballing layout, scroll, color,
  and mode transitions. Don't add new `examples/*_render_check.rs`
  files — wire the renderer into `src/preview.rs` instead (see recipe
  step 4). Unit tests still own correctness; preview owns visual judgement.

### Async + signals

- **`tokio::sync::Mutex`, not `std::sync::Mutex`.** The latter isn't
  `Send` across `.await` points and will not compile inside async fns
  that need it.
- **SIGINT goes through the libc handler in `main.rs`** which calls
  `libc::_exit`. Don't try to add a graceful shutdown path via
  `tokio::signal` — the matrix is already cleared by the panel power
  cut, and async drop ordering across reqwest + tokio is unreliable
  under signal.

### Configs

- **Add new fields to all three example configs.** `ohmyoled.{json,yaml,toml}`
  must stay in sync; `cargo run --example config_formats` enforces this.
- **TOML requires quoted strings** for enum values; JSON does too. YAML
  alone accepts unquoted lowercase. Match this in the examples.

### Dependencies

- **`reqwest` with `rustls-tls`**, not `native-tls`. OpenSSL on the Pi is
  a maintenance burden.
- **No new crates without a clear win.** We've already paid the
  compile-time cost for the current set; prefer adding to an existing
  helper module over pulling in a single-purpose crate.

---

## Common pitfalls

- **Pi devcontainer linker OOM.** `cargo test` builds the bin-test
  binary, which fails to link in the devcontainer. Use `cargo test --lib`
  or `cargo test --doc` instead. The full build succeeds on the Pi.
- **Cross-target build.** `rpi-led-matrix` is declared under
  `[target.'cfg(any(target_arch = "arm", target_arch = "aarch64"))'.dependencies]`
  in `crates/ohmyoled-matrix/Cargo.toml`, so `cargo build` on x86_64
  with the default `hardware` feature still works — the dep just isn't
  pulled and the runtime falls back to terminal mode. Code that uses
  `rpi_led_matrix` is gated by
  `#[cfg(all(feature = "hardware", any(target_arch = "arm", target_arch = "aarch64")))]`.
  ARM cross-builds (via `cross`) keep the hardware backend unchanged.
- **Forgetting `pub mod subway;` in `src/lib/api/mod.rs` or
  `src/lib/matrix/mod.rs`** is the #1 mistake when adding a module.
  Cargo will complain about an unresolved import in `registry.rs` — that
  usually means a missing `mod` declaration.
- **`async fn` in a trait** requires `#[async_trait]`. We use the crate
  consistently — don't try to mix in native async-in-trait, the dyn-
  dispatch shape isn't compatible.
- **`#[serde(tag = "...")]` enums** require each variant to be a struct
  variant, even if it has no fields. Use `F1 { run: bool }`, not just
  `F1`.
- **Don't add a `Default` impl for renderers that load fonts** — the
  default fonts only exist at `/usr/share/fonts/` on installed systems.
  `Default` calling `.expect()` is a footgun in tests.
- **`one_or_many` consumes `#[serde(default)]`.** Both attributes are
  required. The default `Vec::new()` is what makes missing sections
  parse cleanly.
- **`-c` now needs a TTY.** The config builder is a full-screen ratatui
  TUI, so `-c` errors out under a pipe / in CI. Non-interactive setups use
  `--init-config <path>` (writes `default_config()`), then hand-edit. The
  builder's `Options` structs are the source of truth: `section_to_value`
  round-trips each tile through its struct, so a `fields()` `id` that
  doesn't match a serde field name is silently dropped — keep them in sync.

## What lives where

| Need to…                              | Edit…                                                                  |
| ------------------------------------- | ---------------------------------------------------------------------- |
| Add a new API + matrix                | `src/lib/api/<name>/`, `src/lib/matrix/<name>.rs`, `registry.rs`, **`src/createjson/<name>.rs` + `createjson/mod.rs`** |
| Add a new provider to existing module | `src/lib/api/<name>/<provider>.rs` + enum variant in that module's `mod.rs` |
| Add a new config field                | The section struct in `registry.rs` + all three `examples/configs/*` + the matching `*Options` struct **+ a `FieldDef` in `fields()`** in `src/createjson/<name>.rs` + the `default_config()` JSON literal in `createjson/mod.rs` |
| Add the `-c` / `--init-config` tile    | `src/createjson/<name>.rs` (`Options` + `Default` + `fields()`) + register in `createjson/tui/form_module.rs` (`TILE_KINDS`, `fields()`, `config_key()`, `canonicalize()`) + `SECTION_KEYS` (`app.rs`) + `SECTION_ORDER` (`preview.rs`) + `default_config()` placeholder |
| Change panel geometry                 | `src/main.rs::build_matrix`                                            |
| Add a new font                        | `src/sh/install.sh` + the renderer's `*Fonts` struct                   |
| Change a shared HTTP behavior         | `src/lib/api/http.rs`                                                  |
| Tweak the scheduler                   | `src/lib/modules/scheduler.rs`                                         |

---

## Don't do these

- **Don't add Python bindings or pyo3 anywhere.** The migration is
  complete; coexistence is not a goal.
- **Don't write helper crates outside the workspace.** `ohmyoled-matrix`
  is the only sub-crate and it owns the panel abstraction. Everything
  else stays in `oledlib`.
- **Don't add CLI flags for things that should be config.** The
  config file is the single source of truth for what to render. CLI
  flags are reserved for ops concerns (which file to load, dev mode).
- **Don't introduce `Arc<Mutex<…>>` shared between collector and
  renderer.** The scheduler hands the renderer the data; that's the
  channel. If you find yourself wanting shared mutable state, the
  module probably wants splitting.
- **Don't bypass `shared_client()` to hold a per-collector `reqwest::Client`.**
  Connection pooling is real on the Pi.

---

## Fonts

Fonts are **not tracked in git**. The five files the renderers and tests
need (`04B_03B_.TTF`, `04b24.otf`, `BMmini.TTF`, `weathericons.ttf`,
`4x6.bdf`) are bundled in a GitHub Release tarball and fetched on demand.

Run once after a fresh clone, before tests or `cargo run`:

```bash
bash scripts/fetch-fonts.sh
```

The script is idempotent — it skips the download if every expected font
is already on disk. `src/sh/install.sh` calls it automatically before
copying into `/usr/share/fonts/`.

The release URL and expected sha256 live in `scripts/fetch-fonts.sh`;
override with `OHMYOLED_FONTS_URL` / `OHMYOLED_FONTS_SHA256` for testing
a new tarball before tagging.

### Publishing a new fonts tarball

When you need to add a font or change one, build a fresh tarball and
publish a new release tag:

```bash
# 1. Stage the five required fonts in a clean directory
mkdir -p /tmp/ohmyoled-fonts-staging
cp fonts/04B_03B_.TTF \
   fonts/04b24.otf \
   fonts/BMmini.TTF \
   fonts/weathericons.ttf \
   fonts/4x6.bdf \
   /tmp/ohmyoled-fonts-staging/

# 2. Tar+gzip and record the hash
tar -czf fonts.tar.gz -C /tmp/ohmyoled-fonts-staging .
sha256sum fonts.tar.gz

# 3. Publish under a new release tag (e.g. fonts-v2)
gh release create fonts-v2 fonts.tar.gz \
   --title "Runtime fonts v2" \
   --notes "Bumped <font> from <reason>."

# 4. Update scripts/fetch-fonts.sh:
#    - point TARBALL_URL at the new tag (.../download/fonts-v2/fonts.tar.gz)
#    - replace TARBALL_SHA256 with the hash from step 2
```

`fonts.tar.gz` and other tarballs are gitignored via `*.tar.gz`, so the
staging artifact won't accidentally land in git. The initial release
that the current `scripts/fetch-fonts.sh` points at is `fonts-v1`
(sha256 `5f123ded1322ff26d506f524c54e8df82eca9e4eef7eed9acf4a8123ab71b4e1`).

---

## Releasing the binary

`.github/workflows/release.yml` fires on `v*.*.*` tags. It does three
things in parallel and then publishes one GitHub Release:

- Cross-compiles `ohmyoled` for `aarch64-unknown-linux-gnu` and
  `armv7-unknown-linux-gnueabihf` via [cross-rs](https://github.com/cross-rs/cross).
- Builds the same commit natively and runs `ohmyoled --init-config
  ohmyoled-starter.json` to emit a fresh starter config — keeps the
  shipped JSON in lockstep with `createjson::default_config()` so a
  hand-edited sample can never drift from the code.
- Uploads `ohmyoled-aarch64`, `ohmyoled-armv7`, and
  `ohmyoled-starter.json` to the release, with auto-generated release
  notes.

To cut a release:

```bash
# Bump version in Cargo.toml first, commit, push, then:
git tag v3.0.0
git push origin v3.0.0
```

The `--init-config PATH` flag is what powers the starter generation —
end users can run it on their own machine to regenerate a template
config without going through the interactive `-c` flow. Format is
chosen by the extension on PATH (json / yaml / toml).

The same workflow also builds and pushes per-arch Docker images. Each
architecture gets a single-platform image built from
`prodcontainer_build/Dockerfile.release` (which `COPY`s in the
already-cross-compiled binary instead of recompiling under QEMU), and
a final `manifest` job glues them together with
`docker buildx imagetools create`. Two registries:

- **GHCR** — always pushed. Auth via the workflow's `GITHUB_TOKEN`
  (no setup needed); `packages: write` per-job.
- **Docker Hub** — pushed when the `DOCKERHUB_USERNAME` and
  `DOCKERHUB_TOKEN` repo secrets are set. Skipped otherwise so the
  workflow stays green on forks that don't have the secrets.

The result on each registry:

```
…/ohmyoled:v3.0.1                          # multi-arch manifest
…/ohmyoled:latest                          # alias to the same
…/ohmyoled:v3.0.1-aarch64-unknown-linux-gnu
…/ohmyoled:v3.0.1-armv7-unknown-linux-gnueabihf
```

For end users building from source locally, the sibling
`prodcontainer_build/Dockerfile` still does the git clone + cargo
build approach.

---

## Quick verification loop

```bash
bash scripts/fetch-fonts.sh                          # one-time after clone
cargo build                                          # bin + lib
cargo test --lib                                     # unit tests
cargo test --doc                                     # doc tests
cargo clippy --lib --bin ohmyoled --examples         # lint
cargo run --example config_formats                   # all three formats parse equivalently
cargo run --example multi_instance_check             # single-or-array shape
OHMYOLED_MATRIX_MODE=test cargo run -- --preview <name>                    # visual smoke for one renderer
OHMYOLED_MATRIX_MODE=test cargo run -- -f examples/configs/ohmyoled.yaml   # end-to-end rotation
```

`<name>` is any of `time`, `weather`, `stock`, `sport`, `golf`, `f1`,
`iss`, … (see `pub const NAMES` in `src/preview.rs` for the live list).
The preview loops until SIGINT, so wrap in `timeout 6 …` when scripting.

The legacy `examples/*_render_check.rs` bins still compile but are
frozen — new renderers ship via `--preview` only.
