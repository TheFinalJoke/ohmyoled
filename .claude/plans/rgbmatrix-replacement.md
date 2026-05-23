# Replace `rgbmatrix` with a local Rust crate + Python bindings

## Context

Today the project runtime-depends on hzeller's `rpi-rgb-led-matrix` library: `.devcontainer/install.sh` lines 29–39 clone it to `/tmp/rpi-rgb-led-matrix`, build it with `make build-python`, and `pip install` the resulting Python bindings. Every Python entrypoint (`main.py`, `matrix/matrix.py`, `matrix/test.py`) imports from `rgbmatrix`. This is brittle (depends on an external repo at install time, hardware-only, can't be tested off-Pi without the existing `TerminalMatrix` shim) and blocks the longer-term goal of moving modules from Python to Rust.

We're replacing it with a locally-owned Rust crate that:
- Exposes a native Rust API (so future Rust modules can use it directly).
- Exposes Python bindings via pyo3 under a **new** namespace `ohmyoled_matrix` (clean break, no shadowing).
- Ships **both** a terminal backend (ANSI rendering for dev) and an RPi hardware backend (vendored from `rpi-led-panel`).
- Provides full `graphics` parity including a BDF font parser + `DrawText`.

Outcome: `install.sh` no longer clones anything from /tmp, the codebase has zero external rgbmatrix dependency, and `DEV=1` runs work on any machine with `cargo`.

## Scope notes from clarifying questions

- **Rust-first, Python is a thin wrapper**: `crates/ohmyoled-matrix/` is the canonical implementation — all logic, types, and tests live there. `crates/ohmyoled-matrix-py/` exists only to expose the Rust API to Python; it contains **zero business logic**, only pyo3 boilerplate that:
  - Receives Python args → converts to Rust types
  - Calls into the Rust API
  - Converts return values back to Python
  Rule: every method body in `ohmyoled-matrix-py/src/lib.rs` should be one or two lines that delegate to `ohmyoled-matrix`. If a pyo3 method grows beyond that, the logic belongs in the Rust crate, not in the bindings.
- **Path to full Rust**: the existing `ohmyoled` root binary depends directly on `ohmyoled-matrix` (no Python in between). Future Rust modules (TimeMatrix, WeatherMatrix, etc. when ported) call the Rust API natively. The Python bindings are scaffolding — once all consumers move to Rust, `ohmyoled-matrix-py` can be deleted without touching the matrix crate itself.
- **Targets**: terminal + RPi hardware, hardware behind cargo feature `hardware` (default on).
- **Test mode is first-class**: the terminal backend is the official "test mode" — runs anywhere with no hardware. It's always compiled in (no feature flag), selectable explicitly, and used automatically when hardware is unavailable. Three ways to pick it:
  1. **Explicit constructor**: `RGBMatrix::test(options)` in Rust / `RGBMatrix(options, test_mode=True)` in Python.
  2. **Env var**: `OHMYOLED_MATRIX_MODE=test` forces test mode regardless of args (matches existing `DEV=1` pattern in `main.py`).
  3. **Auto-fallback**: if the `hardware` feature is disabled, or hardware init fails (no GPIO / not on Pi), `RGBMatrix::new` falls back to test mode and logs a warning rather than panicking.
- **`MatrixMode` enum**: `Hardware` | `Test` | `Auto` (default). The dispatcher in `backend.rs` selects which backend to instantiate.
- **Hardware impl**: vendor `rpi-led-panel` 0.8.1 (https://github.com/EmbersArc/rpi_led_panel, pure Rust, GPL-2.0) into `crates/ohmyoled-matrix/src/hardware/`. Preserve upstream LICENSE + add VENDORED.md noting provenance and commit hash. Our crate inherits GPL-2.0 — fine because:
- **Distribution**: the matrix crate is **repo-local only**, never published to crates.io. Both new crates get `publish = false` in their `Cargo.toml`. No PyPI wheel either; Python bindings installed locally via `maturin develop` in the devcontainer. No version pinning or release process needed.
- **Python API**: new namespace `ohmyoled_matrix` (not drop-in `rgbmatrix`). All Python import sites updated.
- **Fonts**: full BDF parser + `graphics.DrawText`. (Current code only loads BDF but doesn't draw with it; we ship the real implementation so future modules can use it.)

## Documentation & code style

User requirements: **plain-language documentation** anyone can read, and code that is **heavily commented but not overbearing**.

### Docs (created)

- `crates/ohmyoled-matrix/README.md` — short, plain-English: what the crate is, what "test mode" vs "hardware mode" mean, a 10-line Rust example, a 10-line Python example, and how to choose a mode. No jargon, no Cargo-feature deep dives.
- `crates/ohmyoled-matrix/USAGE.md` — walk-through with one example per task: "show an image", "draw text with a BDF font", "run on Pi", "run in test mode on my laptop". Each example is a runnable snippet.
- `crates/ohmyoled-matrix-py/README.md` — Python-side quickstart: `pip install`-equivalent step, importing, an example that uses Pillow to build an image and pushes it via `RGBMatrix`.
- Rust doc-comments (`///`) on every public type, function, and the crate root so `cargo doc` produces decent reference docs. Each `///` block starts with one short sentence explaining purpose, then a usage hint or example if non-obvious.
- pyo3 `#[pyo3(text_signature = "...")]` + `///` doc-comments on every exported Python class/method so `help(RGBMatrix)` in Python prints something useful.

### Commenting policy for new Rust code

- Module-level `//!` doc-comment at the top of every new file in `crates/ohmyoled-matrix/src/` explaining the file's purpose in 1–3 sentences.
- Public items: `///` doc-comments are required.
- Private items: comment **why** something is done when it isn't obvious from the name (e.g. timing-sensitive GPIO writes, BDF format quirks, ANSI escape choices). Don't comment what the code clearly already says.
- Avoid block-paragraph comments inside function bodies; prefer one short line above the relevant chunk.
- Inherited vendored code under `src/hardware/` keeps its original comments untouched; we only add a note at the top of each vendored file saying "Vendored from rpi-led-panel — see VENDORED.md".

## Critical files

### Created
- `crates/ohmyoled-matrix/Cargo.toml` — new lib crate, `publish = false`, GPL-2.0 (repo-local)
- `crates/ohmyoled-matrix/src/lib.rs` — public surface: `Matrix`, `MatrixOptions`, `MatrixMode`, `Color`, `HardwareMapping`
- `crates/ohmyoled-matrix/src/options.rs` — `MatrixOptions` (mirrors the 7 fields used in `main.py:91-98`: cols, rows, chain_length, parallel, gpio_slowdown, brightness, hardware_mapping)
- `crates/ohmyoled-matrix/src/backend.rs` — `Backend` trait + dyn dispatch + mode selection (env var, explicit, auto-fallback)
- `crates/ohmyoled-matrix/src/terminal.rs` — ANSI true-color backend = **test mode** (always compiled, no feature flag; ports logic from `src/python/ohmyoled/matrix/terminal.py`)
- `crates/ohmyoled-matrix/src/hardware/` — vendored `rpi-led-panel` source tree
- `crates/ohmyoled-matrix/src/hardware/VENDORED.md` — provenance, upstream commit, our modifications
- `crates/ohmyoled-matrix/src/graphics/{mod.rs,bdf.rs,font.rs,draw.rs}` — BDF parsing + DrawText/DrawLine/DrawCircle
- `crates/ohmyoled-matrix/tests/fixtures/*.bdf` — small test font from upstream's `fonts/` dir
- `crates/ohmyoled-matrix-py/Cargo.toml` — pyo3 cdylib (depends on `ohmyoled-matrix`), `publish = false`
- `crates/ohmyoled-matrix-py/src/lib.rs` — pyo3 module `ohmyoled_matrix` exposing `RGBMatrix`, `RGBMatrixOptions`, `TerminalMatrix`, `graphics` submodule
- `crates/ohmyoled-matrix-py/pyproject.toml` — maturin build config
- `crates/ohmyoled-matrix-py/python/ohmyoled_matrix/__init__.py` — re-export from the compiled module

### Modified
- `Cargo.toml` (root) — add `[workspace]` section listing root + the two new crates. Existing `[package]`/`[dependencies]` stay; the `ohmyoled` binary depends on `ohmyoled-matrix` for future Rust integration.
- `.devcontainer/install.sh` — delete the entire rgbmatrix clone/build block (lines 29–39). Add: `pip install maturin` then `cd /workspaces/ohmyoled/crates/ohmyoled-matrix-py && maturin develop`.
- `src/python/ohmyoled/main.py:8-10` — `from rgbmatrix import RGBMatrixOptions, RGBMatrix` → `from ohmyoled_matrix import RGBMatrixOptions, RGBMatrix`
- `src/python/ohmyoled/matrix/matrix.py:15` — `from rgbmatrix import RGBMatrix, graphics` → `from ohmyoled_matrix import RGBMatrix, graphics`
- `src/python/ohmyoled/matrix/test.py:7` — same swap
- `src/python/ohmyoled/matrix/terminal.py` — delete (replaced by `ohmyoled_matrix.TerminalMatrix`); update `main.py:20` import accordingly
- `src/python/setup.py` — add `ohmyoled_matrix` to `install_requires` (or rely on dev install via maturin)

### Existing functions to reuse
- `src/python/ohmyoled/matrix/terminal.py` — port `SetImage` ANSI rendering logic verbatim into `terminal.rs::set_image`. Same `\x1b[38;2;{r};{g};{b}m•\x1b[0m` escape format that `Matrix.get_color` (`matrix/matrix.py:148`) already uses.
- `src/python/ohmyoled/matrix/matrix.py:63-66` — `get_font_graphics` calls `graphics.Font()` + `.LoadFont(...)` from `/etc/ohmyoled/fonts/`. New pyo3 `graphics.Font` matches this shape.
- The two `cargo test` units in `src/createjson/time.rs` (currently passing) — keep them passing through the workspace migration.

## Implementation phases

Each phase ends with a green `cargo check --workspace` and a meaningful commit.

1. **Workspace bootstrap** — convert root `Cargo.toml` to a workspace, create empty `crates/ohmyoled-matrix/` and `crates/ohmyoled-matrix-py/`. No behavior change.
2. **Test mode + public API** — `MatrixOptions`, `MatrixMode`, `Color`, `Backend` trait, `TerminalBackend` rendering ANSI to stdout. Explicit constructors `RGBMatrix::new` / `::test` / `::with_mode`. Env-var override `OHMYOLED_MATRIX_MODE`. Snapshot test against a 4×4 image fixture; unit test that env var forces test mode.
3. **Vendor `rpi-led-panel`** — copy source under `crates/ohmyoled-matrix/src/hardware/`, write `VENDORED.md` with upstream commit hash + GPL-2.0 LICENSE copy, wire under cargo feature `hardware` (default-on). Stub a `HardwareBackend` implementing the same `Backend` trait. Auto-fallback: if `HardwareBackend::init` returns `Err`, the dispatcher falls back to test mode with a `log::warn!` rather than panicking.
4. **BDF graphics** — use `bdf-parser` crate (or vendor one of upstream's BDF files + write a tiny parser). Implement `graphics::Font::load_font`, `Color`, `DrawText`. Tests against bundled BDF font.
5. **pyo3 bindings (thin wrapper)** — `ohmyoled-matrix-py` exposes `RGBMatrix`, `RGBMatrixOptions`, `graphics`. Every wrapper method is 1–2 lines that delegate to the Rust crate; only translation logic (PIL Image → `image::RgbImage`, Python error mapping) lives here. Build with maturin. Test that the same image renders identically when called from Rust and from Python.
6. **Migrate Python imports + wire test mode** — update 3 import sites + delete `terminal.py`. In `main.py`, replace the `if os.getenv("DEV")` branch that picks `TerminalMatrix` vs `RGBMatrix` with a single `RGBMatrix(options=..., test_mode=bool(os.getenv("DEV")))` call so test mode flows through the same constructor. Verify `DEV=1 python3 -m ohmyoled.main` renders to terminal end-to-end.
7. **Clean up `install.sh`** — drop the rgbmatrix clone/build, add maturin develop step. Rebuild the devcontainer to confirm.
8. **Documentation pass** — write the READMEs and USAGE.md described above, then proofread by running `cargo doc --open` and reading the generated Rust docs, plus `python -c "help(ohmyoled_matrix.RGBMatrix)"` for Python docs. Confirm a non-Rust reader can follow the quickstart end-to-end.

## Verification

1. `cargo check --workspace` — workspace compiles.
2. `cargo test --workspace` — Rust unit tests pass (BDF parser, terminal snapshot, existing `createjson::time` tests).
3. `cargo build --release --workspace` — release build clean.
4. `cd crates/ohmyoled-matrix-py && maturin develop` — Python bindings install into the current Python env.
5. `python3 -c "from ohmyoled_matrix import RGBMatrix, RGBMatrixOptions, graphics; print(RGBMatrixOptions().cols)"` — imports resolve, attributes work.
6. `DEV=1 python3 -m ohmyoled.main` — runs end-to-end with ANSI output instead of GPIO; verify time/weather modules render. Also verify `OHMYOLED_MATRIX_MODE=test python3 -m ohmyoled.main` (without `DEV`) forces test mode.
7. `cargo test -p ohmyoled-matrix --no-default-features` — confirm `hardware` feature off compiles and test mode still works.
7. `pytest src/python/ohmyoled/lib/stock/__tests__ src/python/ohmyoled/lib/weather/__tests__` — the 3 passing tests (TestStockQuote) continue to pass.
8. `grep -r "from rgbmatrix" src/python` — zero hits.
9. `grep -rn "rpi-rgb-led-matrix" .devcontainer/` — zero hits.
10. Rebuild devcontainer from scratch — install.sh completes without cloning anything from /tmp.

## Out of scope (deferred)

- Migrating the actual Python matrix modules (`TimeMatrix`, `WeatherMatrix`, `StockMatrix`, `SportMatrix`) to Rust. That's the broader Python-to-Rust migration the user mentioned; this plan only delivers the matrix library underneath it.
- Removing the `rpi-rgb-led-matrix` reference from the legacy `src/python/ohmyoled/lib/binary_build.sh` and `deprecated_install.sh` if they exist — those scripts are already labeled deprecated.
- Real hardware testing — the agent can't run on Pi hardware; the hardware backend will be verified via `cargo check --features hardware` and code review, not live testing.
