# Plan: Eight new dashboard modules (build over time)

## Context

`ohmyoled` currently ships six modules (time, weather, stock, sport, golf, f1). The user wants to expand the rotation with eight more, each adding a new "screen" to the 64×32 panel. This is a planning document for an incremental rollout — one module per PR, following the recipe in `CLAUDE.md`.

The plan is split into two halves:

- **Part 1 — API / Collector work**: endpoints, auth, refresh cadence, normalized output types. Everything under `src/lib/api/<name>/`.
- **Part 2 — Matrix / Renderer work**: 64×32 layout sketches, font choice, color palette, cycle behavior. Everything under `src/lib/matrix/<name>.rs`.

Build order (easiest → most complex): **crypto → iss → quake → aurora → flights → launch → hass → pihole**. This sequences single-number no-auth APIs first (exercise the recipe with minimal surface area), then countdown displays, then local-service auth.

## The 8 modules at a glance

| # | Module | One-line | Auth | Tier |
|---|--------|----------|------|------|
| 1 | crypto | Coin price + 24h % | none | easy |
| 2 | iss    | km to ISS, "OVERHEAD" flip | none | easy |
| 3 | quake  | Largest recent earthquake | none | easy |
| 4 | aurora | Kp index 0–9 + alert | none | easy |
| 5 | flights | Aircraft in bbox + closest callsign | none (anon tier) | medium |
| 6 | launch | Countdown to next orbital launch | none | medium |
| 7 | hass   | Any Home Assistant entity | bearer token | local-auth |
| 8 | pihole | Blocked-queries % | app password | local-auth |

---

# PART 1 — API / Collector work

For each module: **endpoint**, **auth**, **refresh**, **JSON parse strategy** (relevant fields to pluck out), **normalized output type** (`Collector::Output`), **files to create**.

Every collector follows the same skeleton from `src/lib/matrix/time.rs` + existing collector mods. Standard pieces to reuse:

- `crate::api::http::shared_client()` — process-wide `reqwest::Client` (`src/lib/api/http.rs:15`).
- `crate::api::http::get_json::<T>(url, &[(&str, &str)])` — single-shot GET → JSON decode (`src/lib/api/http.rs:26`).
- `crate::api::ApiError` — every collector error converges here via `#[from]`.
- `crate::serde_helpers::{one_or_many, null_string_as_none}` — for config + optional strings.

### 1. `crypto`

- **Endpoint:** `GET https://api.coingecko.com/api/v3/simple/price?ids=<id>&vs_currencies=usd&include_24hr_change=true&include_market_cap=true`
- **Auth:** none. Free tier ~5–15 req/min.
- **Refresh:** 60 s.
- **JSON shape:** `{"bitcoin": {"usd": 43521.0, "usd_24h_change": 1.24, "usd_market_cap": 850000000000.0}}` — top-level keys are coin IDs.
- **Parse note:** dynamic key (the coin id) — deserialize into `HashMap<String, PriceEntry>` then pluck the first/only entry.
- **Output:** `struct CryptoQuote { symbol: String, price_usd: f64, change_24h_pct: f64 }`.
- **Files:** `src/lib/api/crypto/{mod.rs, coingecko.rs, model.rs}`. `pub mod crypto;` in `src/lib/api/mod.rs`.
- **Provider enum:** `CryptoSource::Coingecko(_)` — leave room for Coinbase/Binance later.

### 2. `iss`

- **Endpoint:** `GET https://api.wheretheiss.at/v1/satellites/25544`
- **Auth:** none. ~1 req/sec.
- **Refresh:** 30 s.
- **JSON shape:** `{"latitude": 12.3, "longitude": 45.6, "altitude": 408.5, "velocity": 27500.0, …}` — `latitude`/`longitude` in degrees.
- **Parse note:** straightforward `serde_json::from_str`. No nested structures.
- **Output:** `struct IssState { km_from_user: u32, overhead: bool, lat: f64, lon: f64 }`. Distance computed in `poll()` via Haversine against user lat/lon (config). `overhead` = elevation angle > 10°.
- **Files:** `src/lib/api/iss/{mod.rs, wheretheiss.rs}`. Small Haversine helper in `iss/mod.rs` (don't pull a geo crate — 10 lines of math).

### 3. `quake`

- **Endpoint:** `GET https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/<feed>.geojson` where `<feed>` ∈ `significant_day | 4.5_day | 2.5_day | all_day`.
- **Auth:** none.
- **Refresh:** 5 min.
- **JSON shape:** GeoJSON FeatureCollection — `features[].properties.{mag, place, time, url}`, `features[].geometry.coordinates = [lon, lat, depth_km]`.
- **Parse note:** ignore `geometry` if not needed; just pull `properties`. Sort by `mag` desc; take top entry.
- **Output:** `struct QuakeEvent { magnitude: f32, place: String, age_minutes: u32, depth_km: f32 }`. Empty feed → return a sentinel "QUIET" variant (`Option<QuakeEvent>` or `enum QuakeStatus { Quiet, Event(_) }`).
- **Files:** `src/lib/api/quake/{mod.rs, usgs.rs, model.rs}`.

### 4. `aurora`

- **Endpoint:** `GET https://services.swpc.noaa.gov/json/planetary_k_index_1m.json`
- **Auth:** none.
- **Refresh:** 5 min (data updates every minute, but no hurry).
- **JSON shape:** array of objects `[{"time_tag": "2026-05-24T18:00:00Z", "kp_index": 4.67, "estimated_kp": 5.0, "kp": "4-"}, ...]`. Most recent at end.
- **Parse note:** take the last element. Round `kp_index` to 0–9 integer for display.
- **Output:** `struct AuroraReading { kp: u8, alert: bool, sampled_at: DateTime<Utc> }`. `alert = kp >= threshold` (default 5).
- **Files:** `src/lib/api/aurora/{mod.rs, swpc.rs}`.

### 5. `flights`

- **Endpoint:** `GET https://opensky-network.org/api/states/all?lamin=&lomin=&lamax=&lomax=` (bbox).
- **Auth:** none (anonymous tier — ~400 credits/day, small bbox queries only).
- **Refresh:** 30 s.
- **JSON shape:** `{"time": 1700000000, "states": [[icao24, callsign, country, ..., lon, lat, alt_m, ..., velocity_m_s, ...], ...]}` — `states[]` is a fixed-position tuple array (state vectors).
- **Parse note:** Use a custom struct with `#[serde(rename = "states")]` of `Vec<Vec<serde_json::Value>>` and index into it (positions documented at https://openskynetwork.github.io/opensky-api/rest.html). OR deserialize as a typed tuple via `#[serde(from = "RawStateVector")]`.
- **Output:** `struct FlightSnapshot { count: usize, closest: Option<FlightInfo> }` where `FlightInfo { callsign: String, altitude_ft: u32, distance_km: f32, bearing: f32 }`.
- **Files:** `src/lib/api/flights/{mod.rs, opensky.rs, model.rs}`. Bbox computed client-side from `(lat, lon, radius_km)` — `1° lat ≈ 111 km; 1° lon ≈ 111 km × cos(lat)`.

### 6. `launch`

- **Endpoint:** `GET https://ll.thespacedevs.com/2.3.0/launch/upcoming/?limit=1`
- **Auth:** none. ~15 req/hr unauthenticated → cache aggressively.
- **Refresh:** 30 min.
- **JSON shape:** `{"results": [{"id": "...", "name": "Falcon 9 | Starlink ...", "net": "2026-06-01T14:23:00Z", "launch_service_provider": {"name": "SpaceX", "abbrev": "SPX"}, "rocket": {"configuration": {"name": "Falcon 9"}}, ...}]}`
- **Parse note:** Lots of nested fields — define small `#[derive(Deserialize)]` structs for only the bits we render. `net` is the launch time (UTC).
- **Output:** `struct UpcomingLaunch { provider_abbrev: String, vehicle: String, mission: String, launch_at: DateTime<Utc> }`. T-minus computed at render time (so the countdown ticks on each render frame without re-polling).
- **Files:** `src/lib/api/launch/{mod.rs, lldev.rs, model.rs}`.
- **Config:** optional `agency_filter: Vec<String>` (filter `launch_service_provider.abbrev` against this list when set).

### 7. `hass`

- **Endpoint:** `GET <base_url>/api/states/<entity_id>` with header `Authorization: Bearer <long_lived_token>`.
- **Auth:** bearer token, supplied in config.
- **Refresh:** 30 s.
- **JSON shape:** `{"entity_id": "sensor.kitchen_temp", "state": "72.4", "attributes": {"unit_of_measurement": "°F", "friendly_name": "Kitchen Temp"}, "last_changed": "2026-05-24T..."}`
- **Parse note:** `state` is always a string (HASS quirk). Render-side decides whether to interpret as number, boolean, or raw string.
- **Output:** `struct HassEntity { state: String, unit: Option<String>, label: String, last_changed: DateTime<Utc> }`. `label` from config, falling back to `attributes.friendly_name`.
- **Files:** `src/lib/api/hass/{mod.rs, rest.rs, model.rs}`.
- **HTTP note:** first module needing a custom header. Drop into `shared_client().get(url).bearer_auth(&token).send().await?` rather than `get_json()`. Consider adding `get_json_with_headers` to `http.rs` if hass + pihole both need it — see cross-cutting.

### 8. `pihole`

- **Endpoint:** v6 — `GET <base_url>/api/stats/summary` with `Authorization: <app_password>` header (Pi-hole v6 changed auth from v5's `?auth=` query string). v5 fallback at `GET <base_url>/admin/api.php?summary` still works on old installs.
- **Auth:** app password, supplied in config. Optional — if omitted, queries without auth (works for `summary` on some configs).
- **Refresh:** 30 s.
- **JSON shape (v6):** `{"queries": {"total": 12348, "blocked": 4221, "percent_blocked": 34.2, "unique_domains": 521, ...}, "clients": {...}, ...}`.
- **Output:** `struct PiholeSummary { percent_blocked: f32, queries_today: u32, blocked_today: u32 }`.
- **Files:** `src/lib/api/pihole/{mod.rs, v6.rs, v5.rs, model.rs}`. Variant enum `PiholeApi::V5 | PiholeApi::V6` with config-selectable default `v6`.

### Cross-cutting API additions (consider as ≥2 modules need them)

- **Bearer-auth GET helper** (`get_json_with_headers<T>(url, &[(&str, &str)]) -> Result<T, ApiError>` already accepts headers — check current signature; if the current one ignores headers, fix it). Used by `hass`, possibly `pihole`. **Status: probably already supported** — `src/lib/api/http.rs:26` takes a header slice.
- **Haversine distance helper** in a shared location vs. inline. Used by `iss` and `flights`. **Recommendation: inline** — 10 lines each, no need to share for two callers (per CLAUDE.md "three similar lines is better than a premature abstraction").
- **`zero_as_none` serde helper** — CLAUDE.md references this, but it doesn't exist in `src/lib/serde_helpers.rs`. None of the eight modules above strictly need it. **Defer until a real use case appears, then either add it or strike from CLAUDE.md.**

---

# PART 2 — Matrix / Renderer work

For each module: **64×32 layout sketch**, **font choice**, **color palette**, **cycle duration**, **animation/transition notes**, **files to create**.

The 64×32 panel fits roughly 16 chars wide × 5 rows tall using the 4×6 BDF font, or 12 chars × 4 rows with BMmini at 5–6 px. The "big number" cluster (`iss`, `quake`, `aurora`, `pihole`, `crypto`) leans on `04B_03B_.TTF` at 8–12 px for the headline number and `4x6.bdf` for labels.

Existing fonts (paths under `/usr/share/fonts/` once installed):

- `4x6.bdf` — labels, sub-text. Crisp at native size.
- `BMmini.TTF` — body text, 5–6 px. Used by sport/weather.
- `04B_03B_.TTF` — standard ~8 px. Used by time/stock.
- `04b24.otf` — chunkier OTF variant.
- `weathericons.ttf` — icon font, weather-themed (probably unusable for these modules; emoji literals OR drawn primitives instead).

Every renderer follows the template in `src/lib/matrix/time.rs:111–219`: sync `new()` + async `new_async()` constructors, `frame(&self, data) -> RgbImage` for layout math, `impl Renderer` with `id()`/`cycle_duration()`/`render()`.

### 1. `crypto` — 64×32 layout

```
┌──────────────────────────┐  (64×32, not to scale)
│ BTC               ▲1.24% │  ← symbol left, % change right
│                          │
│   $43,521                │  ← big price, headline
│                          │
│ vol 28.4B    cap 850B    │  ← optional small metadata row
└──────────────────────────┘
```

- **Fonts:** `04B_03B_.TTF` @ 10–12 px for price; `4x6.bdf` for symbol/labels/footer.
- **Colors:** white symbol/price; green `▲` and price if up, red `▼` and price if down; dim grey for footer metadata.
- **Cycle:** 8 s static (no scroll needed — info fits).
- **Animation:** brief flash on price change (1 frame).
- **Files:** `src/lib/matrix/crypto.rs`. Borrows layout math from `src/lib/matrix/stock.rs` but copy-paste — don't share (per CLAUDE.md).

### 2. `iss` — 64×32 layout (two modes)

**Normal mode:**
```
┌──────────────────────────┐
│ 🛰  ISS                  │
│                          │
│      1,247               │  ← big km
│       km                 │
└──────────────────────────┘
```

**Overhead mode** (elevation > 10°):
```
┌──────────────────────────┐
│ 🛰  OVERHEAD             │  ← magenta, flashing
│                          │
│      ●●●●●               │  ← orbital arc indicator
│                          │
│   PASS ENDS  4:23        │  ← time-to-disappear
└──────────────────────────┘
```

- **Fonts:** `04B_03B_.TTF` for distance / banner; `4x6.bdf` for "km" / "PASS ENDS".
- **Colors:** cyan default; magenta on OVERHEAD (with subtle blink).
- **Cycle:** 12 s in normal mode; extend to 20 s if overhead.
- **Animation:** on transition to OVERHEAD, brief "scan" sweep across the panel (one pass top→bottom).
- **Files:** `src/lib/matrix/iss.rs`. Emoji rendering: probably draw a tiny satellite glyph with `put_pixel()` rather than relying on a font's satellite codepoint.

### 3. `quake` — 64×32 layout

```
┌──────────────────────────┐
│ ⚡  M 5.8                │  ← magnitude, color-coded
│                          │
│ OFF EAST COAST OF        │  ← region (scrolls if longer)
│ HONSHU JAPAN             │
│                          │
│ 14m ago         24km dp  │  ← age + depth
└──────────────────────────┘
```

- **Fonts:** `04B_03B_.TTF` @ 12 px for `M 5.8`; `BMmini.TTF` for region; `4x6.bdf` for footer.
- **Colors:** green if mag < 4, amber 4–6, red ≥ 6. Region in white. Footer dim.
- **Cycle:** 15 s (allows one full region scroll for long names).
- **Animation:** region scrolls right→left if it exceeds 16 chars. No scroll otherwise.
- **Empty-feed mode:**
  ```
  ┌──────────────────────────┐
  │     ⚡ QUIET              │
  │   no events 24h          │
  └──────────────────────────┘
  ```
- **Files:** `src/lib/matrix/quake.rs`. Off-season pattern: see how `sport.rs:offseason_renders_two_line_placeholder` test handles its equivalent.

### 4. `aurora` — 64×32 layout

```
┌──────────────────────────┐
│  Kp                      │
│        5                 │  ← huge centered digit
│                          │
│ ▰▰▰▰▰▱▱▱▱                │  ← 9-block bar
│                          │
│   AURORA LIKELY          │  ← only when kp ≥ threshold
└──────────────────────────┘
```

- **Fonts:** `04b24.otf` or even larger custom-drawn digit for the headline; `4x6.bdf` for `Kp` label + banner.
- **Colors:** green Kp 0–3, amber 4, violet 5–6, red 7–9 (matches NOAA convention). Banner cyan on alert.
- **Cycle:** 10 s (15 s when alerting).
- **Animation:** subtle pulse on alert (banner brightness shifts).
- **Files:** `src/lib/matrix/aurora.rs`. The 9-block bar can lift the drawing logic from `.claude/statusline-command.sh`'s context-bar idea (10 blocks; here it's 9 for the Kp scale).

### 5. `flights` — 64×32 layout

```
┌──────────────────────────┐
│ ✈ 7 NEAR                 │  ← count of aircraft in bbox
│                          │
│ DAL2451                  │  ← closest callsign (scrolls if long)
│ FL320  •  12 km SW       │  ← altitude in flight level + range/bearing
└──────────────────────────┘
```

- **Fonts:** `04B_03B_.TTF` for callsign + count; `4x6.bdf` for altitude/bearing.
- **Colors:** cyan callsign, white count, dim grey footer.
- **Cycle:** 12 s.
- **Animation:** a tiny moving dot across the panel representing the closest aircraft's bearing (optional, second iteration).
- **Files:** `src/lib/matrix/flights.rs`. Empty-bbox mode: `┃ ✈ — — — — ┃` line, no count.

### 6. `launch` — 64×32 layout (three modes by countdown)

**T-far (> 24 h):**
```
┌──────────────────────────┐
│ 🚀  SPX                  │
│   T-2d 14h               │
│                          │
│ Falcon 9 / Starlink      │  ← vehicle/mission (scrolls)
└──────────────────────────┘
```

**T-near (< 24 h):**
```
┌──────────────────────────┐
│ 🚀  SPX                  │
│   T-3:42:11              │  ← live HH:MM:SS countdown
│                          │
│ STARLINK 8-5             │
└──────────────────────────┘
```

**T-imminent (< 60 s):**
```
┌──────────────────────────┐
│                          │
│     T-00:00:08           │  ← big, flashing red
│                          │
│      🚀 SPX              │
└──────────────────────────┘
```

- **Fonts:** `04B_03B_.TTF` for countdown digits (live-ticking); `4x6.bdf` for vehicle/mission.
- **Colors:** amber for normal countdown, red flashing under 60 s.
- **Cycle:** 15 s in T-far; 20 s in T-near; extend to 60 s in T-imminent (don't rotate off during launch).
- **Animation:** countdown digits re-render every 1 s in T-near/T-imminent (the collector polls every 30 min — the renderer computes T-minus from `launch_at` locally).
- **Files:** `src/lib/matrix/launch.rs`. The "extend cycle during imminent" needs registry-level awareness — easiest is `cycle_duration()` returns a longer value when the data's `T-minus` is under 60 s. Or expose a hook.

### 7. `hass` — 64×32 layout (generic)

```
┌──────────────────────────┐
│ KITCHEN                  │  ← config label
│                          │
│   72.4 °F                │  ← state + optional unit
│                          │
│  updated 12s ago         │  ← derived from last_changed
└──────────────────────────┘
```

For binary entities (door open/closed, light on/off):
```
┌──────────────────────────┐
│ GARAGE                   │
│                          │
│      OPEN                │  ← state in red/green by config
│                          │
│  since 14m ago           │
└──────────────────────────┘
```

- **Fonts:** `04B_03B_.TTF` for state; `4x6.bdf` for label + footer.
- **Colors:** configurable. Default sage green for numeric, red/green flip for binary based on config-supplied `alarm_state: "OPEN"`.
- **Cycle:** 8 s.
- **Animation:** none.
- **Files:** `src/lib/matrix/hass.rs`. Layout must auto-adapt to state length (right-align numerics, center text).

### 8. `pihole` — 64×32 layout

```
┌──────────────────────────┐
│ 🛡  PIHOLE               │
│                          │
│    34.2%                 │  ← big blocked %
│  blocked today           │
│                          │
│ 12,348 q  /  4,221 blk   │  ← total / blocked
└──────────────────────────┘
```

- **Fonts:** `04B_03B_.TTF` @ 12 px for the %; `4x6.bdf` for labels.
- **Colors:** emerald green (Pi-hole brand). Brighter when % > 30; dim when < 10.
- **Cycle:** 10 s.
- **Animation:** none.
- **Files:** `src/lib/matrix/pihole.rs`. Draw a tiny shield icon (5×7 pixels) for the 🛡 — easier than finding a glyph font.

### Cross-cutting matrix decisions

- **No shared "big-number-with-label" base renderer.** Even though `iss`, `quake`, `aurora`, `pihole`, `crypto` all share that shape, per CLAUDE.md scroll loops and layout are inlined. Revisit only if a third copy-paste happens AND the variations stay minimal.
- **Tiny icons (🛰, ⚡, 🚀, 🛡, ✈) are drawn pixel-by-pixel** in a small `fn draw_<icon>(img, x, y, color)` private to each renderer. The repo has no emoji font (`weathericons.ttf` is weather-specific). 5×7 or 7×7 sprites are quick to hand-design.
- **Color-coded thresholds repeat 3× (quake mag, aurora kp, hass alarm).** A tiny `Color::ramp(value, &[(threshold, color), ...])` helper in `crates/ohmyoled-matrix` is justified once all three exist. Don't pre-extract.
- **Live-ticking displays (launch T-minus, iss pass timer, hass "updated Xs ago")** re-compute display values from immutable `Data` on each render frame, not on each poll. The pattern is already in `src/lib/matrix/time.rs:render()` — copy it.
- **Layout sketches above are starting points, not contracts.** Refine in-renderer during implementation when you can see actual pixel widths of fonts.

---

## Per-module recipe checklist (the recipe, applied 8×)

Use this as a per-PR checklist. Identical for every module.

1. **Collector** — `src/lib/api/<name>/`
   - `mod.rs`: provider enum + collector struct + `impl Collector`.
   - `<provider>.rs`: HTTP + JSON shapes.
   - `model.rs` if normalization is non-trivial.
   - `pub mod <name>;` in `src/lib/api/mod.rs`.

2. **Renderer** — `src/lib/matrix/<name>.rs`
   - Pattern from `src/lib/matrix/time.rs`.
   - Sync `new()` + async `new_async()`, `frame()` for layout, `impl Renderer`.
   - `pub mod <name>;` in `src/lib/matrix/mod.rs`.
   - Doc-comment header with `# Config` block + layout diagram (lift from sketches above).

3. **Registry** — `src/lib/modules/registry.rs`
   - `<Name>Section { run, ... }` struct.
   - `pub <name>: Vec<<Name>Section>` field with `#[serde(default, deserialize_with = "one_or_many")]`.
   - `async fn build_<name>(s) -> Result<Box<dyn DynModule>, String>`.
   - `for s in cfg.<name>.iter().filter(|s| s.run)` loop in `build()`.

4. **Configs** — `examples/configs/ohmyoled.{json,yaml,toml}` — all three in lockstep.

5. **Examples** — `examples/<name>_render_check.rs` for ASCII smoke test.

6. **Tests**
   - Collector fixture parse test (pattern: `src/lib/api/weather/pirate.rs:237`).
   - Renderer `frame_has_dimensions_and_lit_pixels` (pattern: `src/lib/matrix/sport.rs:506`).

7. **README** — module list row if user-facing.

## Cross-cutting decisions

- **Each module = 1 PR.** No bundling.
- **Multi-instance from day one** for `crypto`, `flights`, `hass` (the most likely "I want several of these" cases). The `one_or_many` deserializer already gives this for free — just commit by shipping multi-entry examples.
- **Refresh intervals — match source cadence**, don't out-poll: crypto 60s, iss 30s, quake 5min, aurora 5min, flights 30s, launch 30min, hass 30s, pihole 30s.
- **Crypto kept separate from `stock`.** Different provider universe, different data fields, different refresh tiers. Visual layout cribs from stock but the code is independent. (Re-evaluate if a third "ticker-ish" module ever appears.)

## Future-research bucket (the "search for more" note)

Next candidates after the eight ship. Re-screen each against: (1) real public JSON API no scraping, (2) visually meaningful at 64×32, (3) auth complexity matches the current tier.

- **AQI** — Open-Meteo Air Quality (free, no key). Likely module #9.
- **Moon phase** — sunrisesunset.io or USNO. Icon + illumination %.
- **UV index** — currentuvindex.com, free no-key. Single number color-coded.
- **NASA APOD** — title scroller (image not feasible on 64×32). Free key.
- **XKCD** — daily comic title scroller. No auth.
- **Hacker News top story** — two-hop API. No auth.
- **GitHub repo stars / Actions status** — unauth 60/hr; one repo per cycle.
- **Subway/transit arrivals** — MTA / BART / TfL all expose free feeds per-region.
- **Calendar next meeting** — CalDAV or Google Calendar (OAuth — `hass` tier).
- **Strava / fitness** — OAuth + webhooks; defers indefinitely unless a polling endpoint appears.

Also keep an eye on:

- API status changes (OpenAQ recently moved behind a key — re-check for free alternatives).
- New free-tier providers (the space/astronomy area gets new APIs every year).
- Reader-suggested modules — log them here when they arrive.

## Doc / scaffolding fixups discovered

- `CLAUDE.md` references a `zero_as_none` serde helper that doesn't exist in `src/lib/serde_helpers.rs`. Strike the reference or add the helper. **Defer** — none of the eight modules need it.

## Verification (per module, end-to-end)

After each module:

```bash
cargo build                                           # bin + lib compile
cargo test --lib                                      # unit tests
cargo clippy --lib --bin ohmyoled --examples         # lint
cargo run --example config_formats                   # 3 formats equivalent
cargo run --example multi_instance_check             # one-or-many works
cargo run --example <name>_render_check              # visual ASCII smoke
OHMYOLED_MATRIX_MODE=test cargo run -- -f examples/configs/ohmyoled.yaml   # end-to-end rotation
```

For modules talking to live APIs (1–6), confirm real fetches with `RUST_LOG=debug`. For `hass`/`pihole`, document URL + token in the doc-comment so the LAN test is repeatable.

## Suggested first PR

**`crypto`** — closest to existing patterns (`stock`), exercises the recipe end-to-end with the most reused code, surfaces any conventions worth tightening (provider enum shape, config key naming, layout-copy vs share) on the easiest possible canvas. The remaining seven chip away one PR at a time afterward.
