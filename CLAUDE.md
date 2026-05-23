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
├── createjson/               # interactive `-c` config builder
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
├── multi_instance_check.rs   # single-or-array config shape verifier
└── *_render_check.rs         # per-module ANSI render smoke tests
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

**`refresh_interval`** caps how often `poll()` is called per module —
e.g. weather 600 s, stock 30 s, time 1 s. The scheduler caches the last
good output and re-renders it between polls.

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
}

// 3b. field on RegistryConfig
pub struct RegistryConfig {
    // … existing fields …
    #[serde(default, deserialize_with = "one_or_many")]
    pub subway: Vec<SubwaySection>,
}

// 3c. builder helper
async fn build_subway(s: &SubwaySection) -> Result<Box<dyn DynModule>, String> {
    let collector = SubwayCollector::from_mta(MtaConfig {
        station_id: s.station_id.clone(),
    }).map_err(|e| e.to_string())?;
    let renderer = SubwayMatrix::new_async()
        .await
        .map_err(|e| format!("subway fonts: {e}"))?;
    Ok(Box::new(Module::new(collector, renderer)))
}

// 3d. loop in `build()`
for s in cfg.subway.iter().filter(|s| s.run) {
    match build_subway(s).await {
        Ok(m) => modules.push(m),
        Err(e) => log::error!("subway: skipping module: {e}"),
    }
}
```

### 4. Update docs

- Add a `# Config` block to the top of `src/lib/matrix/subway.rs` matching the
  format used by every other matrix doc-comment (layout diagram + YAML
  snippet + data source line).
- Add a `subway:` example to the three [`examples/configs/`](examples/configs/)
  files.
- Add a row to the `### subway` section of `README.md` if the module is
  user-facing.

That's the whole recipe. No changes to `main.rs`, no changes to the
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

---

## What lives where

| Need to…                              | Edit…                                                                  |
| ------------------------------------- | ---------------------------------------------------------------------- |
| Add a new API + matrix                | `src/lib/api/<name>/`, `src/lib/matrix/<name>.rs`, `registry.rs`       |
| Add a new provider to existing module | `src/lib/api/<name>/<provider>.rs` + enum variant in that module's `mod.rs` |
| Add a new config field                | The section struct in `registry.rs` + all three `examples/configs/*`   |
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

---

## Quick verification loop

```bash
bash scripts/fetch-fonts.sh                          # one-time after clone
cargo build                                          # bin + lib
cargo test --lib                                     # unit tests (60+)
cargo test --doc                                     # doc tests
cargo clippy --lib --bin ohmyoled --examples         # lint
cargo run --example config_formats                   # all three formats parse equivalently
cargo run --example multi_instance_check             # single-or-array shape
OHMYOLED_MATRIX_MODE=test cargo run -- -f examples/configs/ohmyoled.yaml  # end-to-end
```
