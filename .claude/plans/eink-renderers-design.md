# E-ink renderer designs — porting the remaining tiles

Design plan for giving every module an e-paper (`EinkRenderer`) screen, the
static full-resolution counterpart to its LED tile. Already done: **time**,
**weather**. This doc designs the remaining 12: stock, stock_chart, sport,
golf, f1, iss, quake, aurora, flights, launch, hass, pihole.

The data layer is reused verbatim — every collector already emits a normalized
struct (fields below come from each module's `model.rs` / the `preview.rs`
fakes). Only a new `EinkRenderer` + a `build_eink_*` arm is needed per tile.

---

## Shared principles

**Canvas.** The recommended/default panel is the 7.5" **800×480** (`7in5_v2`);
designs are authored against it. Renderers read live `display.width()/height()`
for positions (fractions of `w/h`) and size fonts via `layout::scaled_px`
(base sizes authored for 480px height, scaled to the actual panel), so a tile
reads the same on 800×480 or a 400×300 4.2". Compose white-on-black (the
`draw_*` convention); the display inverts to black-ink-on-white.

**Status:** the shared toolkit (`src/lib/matrix/eink/layout.rs`) and the
adaptive font sizing exist; time + weather are built on them. The remaining 12
tiles below follow the same pattern.

**Static, not scrolling.** E-paper holds its image and refreshes slowly, so
every tile shows its data *all at once* — the extra resolution (400×300 vs
64×32 = ~58× the area) means the LED scroll loops become a single laid-out
screen. No marquees, no animation.

**Monochrome encoding.** The LED tiles lean on color; B/W needs other signals:
- **Direction / sign** → arrow glyphs (▲ up, ▼ down) + sign, not red/green.
- **Magnitude / intensity** → filled bar length, ring fill, or big number, not
  a color ramp. Optional 50% **hatching** for a "mid" band.
- **Alert / emphasis** → a bordered/filled **badge** (inverted box: black fill,
  white text) instead of a red banner.
- **Categories** → labels + simple geometric icons drawn from `draw_line`/
  `draw_circle` (arrows, bars, rings, a plane/satellite from a few lines).

**Refresh cadence (`cycle_duration`).** Matched to data volatility AND
e-paper's slow refresh — never sub-minute. Suggested: pihole/hass 60s,
stock/stock_chart 60s, sport 60s, iss 60s, flights 60s, f1/golf 120s, quake
120s, aurora 300s, launch 300s. (The panel holds the last frame between
refreshes, so a slightly stale tile is fine.)

**Empty / off-season states.** Every sport tile, quake, flights, launch has a
"nothing right now" path — designed explicitly as a centered status card.

---

## Shared toolkit to build first — `src/lib/matrix/eink/widgets.rs`

The 12 tiles repeat the same primitives. Build these `pub(crate)` helpers once
(operating on `&mut RgbImage`, all resolution-agnostic) so the renderers stay
short and consistent. (Unlike the LED side, where scroll loops were
deliberately inlined, the e-ink side is pure static layout — a shared toolkit
is the right call.)

- `header_band(img, fonts, title, right_label) -> i32` — title left, optional
  right label, a rule under it; returns the y below the rule. Used by ~all.
- `footer(img, fonts, text)` — dim metadata line pinned to the bottom.
- `center_text` / `right_text` — promote the helpers already duplicated in
  eink/time + eink/weather here.
- `big_value(img, fonts, x, baseline, value, unit)` — large number + small
  unit, the "one big stat" element (pihole %, aurora Kp, iss distance).
- `stat_cell(img, fonts, rect, label, value)` — small label over a value,
  optionally boxed. The unit of every grid layout.
- `grid(rect, cols, rows) -> Vec<Rect>` — split a region into equal cells.
- `hbar(img, rect, frac, ticks)` — horizontal bar gauge 0..1 with optional tick
  marks (aurora Kp scale, pihole intensity, uv).
- `ring(img, cx, cy, r, frac)` — ring gauge (alternative big-stat treatment).
- `sparkline(img, rect, points)` / `line_chart(img, rect, series, baseline)` —
  for stock_chart + hass graph mode.
- `table(img, fonts, rect, rows: &[Row], cols: &[Col])` — left/right aligned
  columns with a header row; the backbone of every leaderboard (f1, golf,
  quake list, flights, launch, sport standings).
- `badge(img, fonts, x, y, text, filled)` — inverted/bordered pill for alerts
  and statuses.
- `arrow(img, x, y, up)` + `signed(img, fonts, x, y, value)` — ▲/▼ + sign for
  stock/sport deltas.

Also a shared **`EinkFontset`** (one struct loaded once: `huge`, `big`, `body`,
`small`, `tiny` from `04B_03B`, plus `weathericons` for weather/forecast) so
each renderer stops re-declaring font sizes. time/weather get refactored onto
it as part of this step.

---

## Per-tile designs

Layout sketches are for 400×300. `[B]` = badge, `▲▼` = arrows, `▮▮▯` = bar.

### stock — `StockQuote { symbol, name, open, current, high, low, previous_close }`
One ticker, one screen.
```
┌ SYMBOL ───────────────────  Apple Inc. ┐   header band (symbol + name)
│                                         │
│   153.42        ▲ +3.22 (+2.1%)         │   huge price; arrow+signed delta
│                                         │   (vs previous_close)
│   O 150.00  H 154.10  L 149.85  PC 150.20  4-cell stat row (OHLC + prev close)
│   ▮▮▮▮▮▮▮▮▮▮▮▮▮▯▯▯▯▯  day range          │   hbar: where current sits in L..H
└─────────────────────────────────────────┘
```
Mono: up/down by **arrow + sign** (no red/green). Day-range bar shows current
within low..high. Refresh 60s.

### stock_chart — `StockHistory { symbol, current, previous_close, day, month, year: HistorySeries }`
The LED tile cycles 1D/1M/1Y; e-ink shows **all three stacked** (no cycling).
```
┌ SYMBOL  153.42  ▲ +2.1% ───────────────┐  header (price + overall delta)
│ 1D  ╱╲╱‾‾╲╱  range 149.8–154.1          │  line_chart row 1 (+ min/max label)
│ 1M  ╱‾‾╲__╱‾                            │  line_chart row 2
│ 1Y  ___╱‾‾‾                             │  line_chart row 3
└─────────────────────────────────────────┘
```
Mono: direction via the line shape + a baseline (previous_close dashed line);
arrow+sign for the delta. Each row autoscaled, labeled with its min/max.
Refresh 60s.

### sport (team) — `SportData { sport, team_name, record, next_game: Option<NextGame>, standings }`
`NextGame { start, status, home/away: TeamSide{name,abbr,score}, our_side }`
```
┌ NBA · Boston Celtics  56-26 ───────────┐  header (league + team + record)
│   BOS  108                              │  scoreboard: our side / opponent,
│   PHI  100      FINAL [B]               │  big scores, status badge
│   (or "Tue 7:30p vs PHI" when upcoming) │
│ ── Standings ──                         │
│  1 Team1    2 Team2    3 Team3 …        │  standings table (position+name)
└─────────────────────────────────────────┘
```
Mono: win/lead shown by an **underline/box on the leading score**, not color;
status (FINAL / LIVE / scheduled) as a badge. Empty → "NO GAME SCHEDULED"
card. Refresh 60s. (Team logos are URLs/PNGs — skip on B/W for v1; abbreviation
text stands in. Could add 1-bit logo support later.)

### golf — `GolfData { tour, event_name, status, leaderboard: [{position, player_short, score}] }`
```
┌ PGA · Masters Tournament  In Progress ─┐  header (tour + event + status)
│  1  SCHEFFLER         -12               │  leaderboard table (pos/name/score),
│  2  RAHM               -8               │  ~8–10 rows fit at 400×300
│  3  MORIKAWA           -6               │
│  …                                      │
└─────────────────────────────────────────┘
```
Mono: leader gets a bold/boxed row; scores are already strings ("-12"). Off-
season/empty → "NO EVENT THIS WEEK" card. Refresh 120s.

### f1 — `F1Data { season, next_race: Option<NextRace>, standings: [{position, code, family_name, points}] }`
`NextRace { round, name, circuit, start }`
```
┌ F1 2026 ───────────────────────────────┐
│ NEXT: R7 Monaco GP · Circuit de Monaco  │  next-race band
│       T- 2d 14h  (or date)              │  countdown to start
│ ── Drivers ──                           │
│  1 VER 161   2 NOR 145   3 LEC 132 …    │  standings table (code + points)
└─────────────────────────────────────────┘
```
Mono: P1/P2/P3 marked with a small medal ring or bold. Off-season (no
next_race) → champion card: "2025 CHAMPION · VERSTAPPEN". Refresh 120s.

### iss — `IssState { ground_distance_km, overhead, lat, lon, altitude_km, velocity_kms, visibility }`
```
┌ ISS ───────────────────────────────────┐
│            1 247 km                      │  huge distance (big_value)
│         to your location                 │
│  ALT 421 km   VEL 7.66 km/s   ☼ daylight │  stat row (altitude/velocity/vis)
│  lat 23.5  lon -50.1                     │  position line
└─────────────────────────────────────────┘
```
Overhead state → full-screen inverted **[ OVERHEAD ]** badge + "look up" + the
stats. Mono: the magenta banner becomes the inverted badge. Refresh 60s
(matches a slowly-changing distance; not per-second).

### quake — `QuakeStatus::Event(QuakeEvent { magnitude, title, origin, depth_km, felt }) | Quiet`
```
┌ EARTHQUAKE ─────────────────────────────┐
│   M 6.2                                  │  huge magnitude (big_value)
│   OFF EAST COAST OF HONSHU, JAPAN        │  wrapped location (title)
│   14 min ago · depth 24 km · felt 482    │  footer metadata
└─────────────────────────────────────────┘
```
Mono: LED magnitude-color band → magnitude **size is already the signal**;
optionally a severity bar (M0–8 hbar) under the number. `Quiet` → centered
"QUIET · no events 24h" card. Refresh 120s.

### aurora — `AuroraReading { kp, kp_index, kp_text, alert, sampled_at }`
```
┌ AURORA · Kp index ─────────────────────┐
│        6                                 │  huge Kp digit (big_value)
│  ▮▮▮▮▮▮▯▯▯  0 ─────────── 9              │  9-step Kp scale bar (hbar w/ ticks)
│  [ AURORA LIKELY ]                       │  badge shown when alert
│  sampled 12 min ago                      │  footer
└─────────────────────────────────────────┘
```
Mono: the green→red Kp ramp becomes the **filled length of the 9-step bar** +
the big digit; alert is the inverted badge. Refresh 300s.

### flights — `FlightSnapshot { count, closest: Option<FlightInfo>, nearby: [FlightInfo], radius_km }`
`FlightInfo { callsign, altitude_ft, on_ground, distance_km, bearing_deg, ground_speed_kt, country }`
```
┌ FLIGHTS · 80 km · 7 aircraft ──────────┐  header (radius + count)
│ CLOSEST  DAL2451  FL320  12 km  225°    │  closest highlighted
│ ── nearby ──                            │
│ UAL989  FL380  28km   JBU42  FL300 44km │  table of nearby (callsign/alt/dist)
│ AAL117  FL360  55km   …                 │
└─────────────────────────────────────────┘
```
Optional **mini radar**: a circle with dots placed by bearing/distance (drawn
with `draw_circle` + points) in a corner — a nice B/W use of the space. Empty →
"NO AIRCRAFT IN RANGE" card. On-ground flights tagged "GND". Refresh 60s.

### launch — `UpcomingLaunch { provider, vehicle, mission, launch_at, status, country_code }`
```
┌ NEXT LAUNCH ───────────────────────────┐
│   T- 03:42:11                            │  big countdown (or T-2d 14h far out)
│   SpaceX · Falcon 9                      │  provider · vehicle
│   Starlink Group 8-5                     │  mission
│   GO [B]            USA                   │  status badge + country
└─────────────────────────────────────────┘
```
Mono: imminent/red-flash → inverted **[ T-MINUS ]** badge + bold countdown;
in-flight → "LIFTOFF" badge. Refresh 300s (re-render recomputes countdown from
`launch_at` each cycle). Could keep a list of next N below the hero.

### hass — `HassEntity { state, unit, label, last_changed, history }` + `HassDisplay { alarm_state, mode, … }`
Three modes (state / historical / graph) → on e-ink, **one richer screen**:
```
┌ KITCHEN ───────────────────────────────┐  label
│   72.4 °F                                │  big_value (state + unit)
│   ╱‾╲__╱‾  (history sparkline)           │  sparkline when numeric history
│   updated 12s ago                        │  last_changed
└─────────────────────────────────────────┘
```
Mono: alarm_state match → inverted badge / boxed value instead of red. Non-
numeric entities (e.g. "open") → big text state, no sparkline. Refresh 60s.
(`mode` can still select state-only vs graph for users who want it.)

### pihole — `PiholeSummary { percent_blocked, queries_today, blocked_today, unique_clients }`
```
┌ PI-HOLE ───────────────────────────────┐
│        34.2%                             │  huge percent (big_value)
│   ▮▮▮▮▮▮▯▯▯▯▯▯▯▯▯▯▯▯  blocked            │  hbar of percent_blocked
│  12 348 queries   4 221 blocked          │  stat row
│  12 clients                              │
└─────────────────────────────────────────┘
```
Mono: intensity color tiers → the **bar fill + big number**. Refresh 60s.

---

## Rollout / phasing

**Phase 0 — toolkit.** Build `eink/widgets.rs` + `EinkFontset`; refactor the
existing time/weather renderers onto them. Nothing user-visible, but every
later tile gets shorter and consistent. Land + test first.

**Phase 1 — single big-stat tiles (simplest, exercise big_value/hbar/badge):**
pihole, iss, aurora, quake. (quake/iss also establish the "status card" empty
state.)

**Phase 2 — tables/leaderboards (exercise `table`):** f1, golf, flights,
launch.

**Phase 3 — composite/rich (charts, scoreboards):** stock, stock_chart, sport,
hass.

Each tile is one `EinkRenderer` + a `build_eink_*` arm + a `frame_*` test +
`--preview eink:<name>` wiring + a doc-comment Config block, exactly like the
time/weather recipe. The `build_eink` "pending" counter shrinks to zero as they
land.

## Recipe per tile (same every time)

1. `src/lib/matrix/eink/<name>.rs` — `Eink<Name>Matrix` with
   `frame(data, w, h) -> RgbImage` built from `widgets::*`, `new()/new_async()`,
   and `EinkRenderer` (compose → `display.show` → dwell).
2. Export in `eink/mod.rs` + `matrix/mod.rs`.
3. `registry::build_eink_<name>` + a loop in `build_eink`; drop it from the
   `pending` counter. Reuse the existing collector builder (extract a
   `*_collector` helper from the LED `build_<name>` where one isn't already
   factored, as done for weather).
4. `preview.rs`: add to `EINK_NAMES` + a `preview_eink_<name>` with fake data.
5. Tests: `frame_has_dimensions_and_lit_pixels` (+ an empty/off-season test for
   sport/quake/flights/launch/golf/f1).
6. Docs: Config doc-comment; README note if useful.

## Open questions

- **1-bit team/league logos** (sport, f1, launch agency) — fetch + threshold a
  PNG to a small bitmap, or stay text-only? Proposed: text-only for v1, add a
  shared 1-bit logo cache later.
- **Composite "dashboard" pane** (multiple APIs in one screen) — tracked
  separately; the `widgets::grid`/`stat_cell`/`header_band` toolkit here is the
  foundation it will reuse.
- **Per-tile `model` sizing** — fonts are tuned for 400×300; very small panels
  (2.13") may need a compact variant. Defer until a non-4in2 panel is in play.
