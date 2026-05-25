# ohmyoled

A pure-Rust driver for a 64×32 RGB LED matrix panel that rotates through
modules — clock, weather, stock and crypto quotes, team sports, golf
leaderboards, Formula 1 standings, a live ISS tracker, recent
earthquakes, aurora forecasts, nearby flights, orbital launch
countdowns, and any Home Assistant entity — pulling from free or
freemium public APIs.

The legacy Python core has been fully migrated; the runtime is a single
Rust binary plus a config file. See the wiki at
<https://github.com/TheFinalJoke/ohmyoled/wiki> for hardware photos and
backstory.

---

## Table of contents

- [Quickstart](#quickstart)
- [CLI usage](#cli-usage)
- [Config file](#config-file)
  - [Supported formats](#supported-formats)
  - [`matrix_options`](#matrix_options)
  - [`time`](#time)
  - [`weather`](#weather)
  - [`stock`](#stock)
  - [`sport`](#sport)
  - [`iss`](#iss)
  - [`quake`](#quake)
  - [`aurora`](#aurora)
  - [`flights`](#flights)
  - [`launch`](#launch)
  - [`hass`](#hass)
- [Environment variables](#environment-variables)
- [Fonts](#fonts)
- [Gotchas](#gotchas)
- [Architecture](#architecture)
- [Development](#development)

---

## Quickstart

Get the image, generate a config, edit it, run it. Both
`ghcr.io/thefinaljoke/ohmyoled` and `thefinaljoke/ohmyoled` on Docker
Hub publish multi-arch (aarch64 + armv7) images on every release;
`:latest` aliases the most recent semver tag.

```bash
# 1. Pull (either registry — pick one)
docker pull ghcr.io/thefinaljoke/ohmyoled:latest
# docker pull thefinaljoke/ohmyoled:latest

# 2. Generate a starter config on the host. Format is picked from the
#    extension on the path you pass (.yaml / .json / .toml).
docker run --rm -v "$PWD:/work" \
  ghcr.io/thefinaljoke/ohmyoled:latest \
  --init-config /work/ohmyoled.yaml

# 3. Edit ./ohmyoled.yaml — replace the REPLACE_ME_* placeholders with
#    your API keys and flip `run: false` → `run: true` on the modules
#    you want active.

# 4. Run it. The mounts are the load-bearing bit:
#      /etc/localtime  — host clock so timestamps + sunrise math match
#      ohmyoled.yaml   — the config you just generated
#      --privileged    — broad GPIO access on the Pi (simplest)
docker run -d --name ohmyoled --restart unless-stopped \
  --privileged \
  -v /etc/localtime:/etc/localtime:ro \
  -v "$PWD/ohmyoled.yaml:/etc/ohmyoled/ohmyoled.yaml:ro" \
  ghcr.io/thefinaljoke/ohmyoled:latest \
  --config /etc/ohmyoled/ohmyoled.yaml
```

Logs go to stderr inside the container — `docker logs -f ohmyoled` to
follow. The binary also writes to `/var/ohmyoled/ohmyoled.log` inside
the container; add `-v /var/ohmyoled:/var/ohmyoled` to persist it to
the host.

### Don't want Docker?

The same release also ships standalone binaries. Same `--init-config`
flow, just running directly on the host:

```bash
# Pick aarch64 for 64-bit Pi OS, armv7 for 32-bit
sudo curl -L -o /usr/local/bin/ohmyoled \
  https://github.com/TheFinalJoke/ohmyoled/releases/latest/download/ohmyoled-aarch64
sudo chmod +x /usr/local/bin/ohmyoled

# Fetch the fonts the binary expects in /usr/share/fonts/
sudo bash -c '
  curl -fsSL https://github.com/TheFinalJoke/ohmyoled/releases/download/fonts-v1/fonts.tar.gz \
    -o /tmp/fonts.tar.gz
  tar -xzf /tmp/fonts.tar.gz -C /usr/share/fonts/
  rm /tmp/fonts.tar.gz
'

# Same --init-config flow as above, but writing straight to /etc/ohmyoled
sudo mkdir -p /etc/ohmyoled
sudo ohmyoled --init-config /etc/ohmyoled/ohmyoled.yaml
# edit /etc/ohmyoled/ohmyoled.yaml, then:
sudo ohmyoled --config /etc/ohmyoled/ohmyoled.yaml
```

To build from source instead, see [Development](#development).

To preview without hardware (renders to ANSI in the terminal):

```bash
OHMYOLED_MATRIX_MODE=test cargo run -- -f examples/configs/ohmyoled.yaml
```

---

## CLI usage

```
ohmyoled [--config <PATH>] [--create_config | --init-config <PATH> | --dev]
```

| Flag                       | Description                                                                              |
| -------------------------- | ---------------------------------------------------------------------------------------- |
| `-f`, `--config <PATH>`    | Path to config file. Format is chosen by extension (`.json`/`.yaml`/`.yml`/`.toml`).     |
| `--json_file <PATH>`       | Deprecated alias for `--config`. Still works.                                            |
| `-c`, `--create_config`    | Run the interactive config builder and write to `<PATH>` (or `/etc/ohmyoled/ohmyoled.json`). Format is chosen by the path's extension. Alias: `--create_json`. |
| `--init-config <PATH>`     | Write a non-interactive starter config to `<PATH>` (time enabled, weather/stock/sport wired but `run: false` with `REPLACE_ME_*` placeholders). Format from the extension. Refuses to overwrite an existing file. |
| `--dev`                    | Write a canned dev config instead of prompting.                                          |
| `-v` / `-vv`               | Bump log verbosity above the info default: debug / trace.                                |
| `--log-file <PATH>`        | Override the log file path (default `/var/ohmyoled/ohmyoled.log`).                       |

When `--config` is not passed, ohmyoled looks for the first existing file
among `/etc/ohmyoled/ohmyoled.{json,yaml,yml,toml}`.

---

## Config file

### Supported formats

`ohmyoled` accepts **JSON, YAML, or TOML** — same schema, different
syntax. The format is picked from the file extension. Files with no
known extension are sniffed in the order JSON → YAML → TOML.

Three drop-in example configs live in [`examples/configs/`](examples/configs/):

- [`ohmyoled.json`](examples/configs/ohmyoled.json)
- [`ohmyoled.yaml`](examples/configs/ohmyoled.yaml)
- [`ohmyoled.toml`](examples/configs/ohmyoled.toml)

Run `cargo run --example config_formats` to verify all three parse to
equivalent `RegistryConfig` values.

A complete config has up to five top-level keys: `matrix_options`,
`time`, `weather`, `stock`, `sport`. All sections except `matrix_options`
support **either a single object or a list** — useful when you want
multiple instances of the same module type (two stock symbols, two team
sport feeds, a basketball team plus a golf leaderboard, etc.).

---

### `matrix_options`

Panel hardware configuration. Required.

```yaml
matrix_options:
  chain_length: 1     # daisy-chained panels
  parallel: 1         # parallel chains
  brightness: 50      # 0-100
  oled_slowdown: 3    # GPIO timing slowdown; higher = slower signal
  fail_on_error: false
```

| Field           | Type | Default | Notes                                                            |
| --------------- | ---- | ------- | ---------------------------------------------------------------- |
| `chain_length`  | int  | `1`     | Number of panels chained together; clamped to ≥1.                |
| `parallel`      | int  | `1`     | Number of parallel chains; clamped to ≥1.                        |
| `brightness`    | int  | `50`    | 0 (off) – 100 (full). High brightness draws significant current. |
| `oled_slowdown` | int  | `3`     | Pi 4 typically needs 3–4. Bump if pixels flicker.                |
| `fail_on_error` | bool | `false` | Currently informational; modules log + skip on error regardless. |

Panel size (`cols`/`rows`) and `hardware_mapping` are hardcoded for
64×32 panels on the Adafruit RGB Matrix HAT. Edit `src/main.rs::build_matrix`
if you need other geometries.

---

### `time`

System clock. Single section (not a list).

```yaml
time:
  run: true
  color: [255, 255, 255]      # RGB 0-255
  time_format: "12h"          # "12h" | "24h"
  timezone: "America/New_York" # IANA tz name; optional
```

| Field         | Type        | Required | Notes                                            |
| ------------- | ----------- | -------- | ------------------------------------------------ |
| `run`         | bool        | yes      | Set `false` to skip without removing the section.|
| `color`       | `[r, g, b]` | yes      | RGB 0-255 tuple.                                 |
| `time_format` | string      | no       | Default `"12h"`. The literal string `"null"` is treated as unset. |
| `timezone`    | string      | no       | IANA timezone. Empty/null → system clock.        |

Cycle: ~30 s per rotation, refreshes every second.

---

### `weather`

A list of provider configs. Each entry renders its own pair of screens
(temp/feels/high/low, then humidity/wind/sunrise/sunset). Cycle ~65 s per
entry. Refresh: 10 min.

```yaml
weather:
  - run: true
    api: openweather                       # openweather | nws | accuweather | pirate
    api_key: YOUR_KEY                      # not needed for nws
    current_location: true
    current_location_api_key: YOUR_IPINFO_TOKEN
    city: "Boston, MA"
    weather_format: imperial               # imperial | metric
```

| Field                       | Type   | Required               | Notes                                                                |
| --------------------------- | ------ | ---------------------- | -------------------------------------------------------------------- |
| `run`                       | bool   | yes                    | Set `false` to skip.                                                 |
| `api`                       | enum   | yes                    | One of `openweather`, `nws`, `accuweather`, `pirate`.                |
| `api_key`                   | string | yes (all but `nws`)    | Provider API key.                                                    |
| `current_location`          | bool   | no (default `false`)   | If `true`, geolocate via ipinfo.                                     |
| `current_location_api_key`  | string | when `current_location`| ipinfo.io token. Free tier allows 50k requests/month.                |
| `city`                      | string | when `!current_location` | Plaintext city query (provider-dependent format). NWS ignores this — it always uses geolocation. |
| `weather_format`            | string | no (default `imperial`)| `imperial` or `metric`.                                              |

#### Provider notes

| Provider      | Endpoint                                | Key | Gotchas                                                                                                       |
| ------------- | --------------------------------------- | --- | ------------------------------------------------------------------------------------------------------------- |
| `openweather` | OneCall 3.0 (`api.openweathermap.org`)  | yes | OneCall 3.0 requires a paid-but-free-tier subscription. The legacy 2.5 endpoint is no longer supported.       |
| `nws`         | `api.weather.gov`                       | no  | **US only.** Two-step request (points → forecast). Still needs an `ipinfo` token if `current_location: true`. |
| `accuweather` | accuweather.com                         | yes | Aggressive free-tier quota (~50 calls/day); set a high refresh interval if running 24/7.                      |
| `pirate`      | Pirate Weather (Dark Sky drop-in)       | yes | Drop-in for the deprecated Dark Sky API. Free tier exists.                                                    |

All four providers normalize to a shared `Weather` shape — icons render
identically across providers because NWS strings are mapped to OWM codes.

---

### `stock`

A list of tickers. Each entry renders symbol + current price, change,
high/low, and prev-close. Two providers share this section: **Finnhub**
for equities (refresh 30 s) and **CoinGecko** for crypto (refresh 60 s).
Both normalize into the same on-panel layout.

```yaml
stock:
  - run: true
    api: finnhub
    api_key: YOUR_FINNHUB_KEY
    symbol: AAPL
  - run: true
    api: finnhub
    api_key: YOUR_FINNHUB_KEY
    symbol: MSFT
  - run: true
    api: coingecko
    api_key: null              # not needed for coingecko's free tier
    symbol: bitcoin            # coin id, not ticker
  - run: true
    api: coingecko
    api_key: null
    symbol: ethereum
```

| Field     | Type   | Required | Notes                                                                                                          |
| --------- | ------ | -------- | -------------------------------------------------------------------------------------------------------------- |
| `run`     | bool   | yes      |                                                                                                                |
| `api`     | enum   | yes      | `finnhub` (equities) or `coingecko` (crypto).                                                                  |
| `api_key` | string | finnhub only | Get a Finnhub key at <https://finnhub.io> (free tier OK). CoinGecko's public tier is keyless — set to `null`. |
| `symbol`  | string | yes      | For Finnhub: ticker (e.g. `AAPL`). For CoinGecko: coin id (e.g. `bitcoin`, `ethereum`) — display ticker comes from the API. |

Display colors: green for up, red for down.

---

### `sport`

A list of mixed sport entries. Each entry is **discriminated by the inner
`sport` field**: team sports (`basketball`, `baseball`, `football`,
`hockey`) require a `team_logo`; `golf` takes an optional `tour`; `f1`
takes nothing extra.

```yaml
sport:
  - run: true
    sport: basketball              # basketball | baseball | football | hockey | golf | f1
    team_logo:
      name: "Boston Celtics"
      shorthand: BOS
      sport: basketball
      url: "https://.../badge.png"
      sportsdb_leagueid: 4387
      apisportsid: 133
      sportsdbid: 134860
      sportsipyid: 0
  - run: true
    sport: golf
    tour: pga                      # pga | lpga | champions | korn | liv
  - run: true
    sport: f1
```

Multiple entries rotate through together — basketball + hockey + golf +
F1 in one display sequence is exactly the YAML above.

#### Team sports (basketball / baseball / football / hockey)

| Field                  | Type   | Notes                                                                |
| ---------------------- | ------ | -------------------------------------------------------------------- |
| `team_logo.name`       | string | Full team name. Used for the header scroll.                          |
| `team_logo.shorthand`  | string | Abbreviation (`BOS`, `NYY`, etc.). Used to look up the team on ESPN. |
| `team_logo.sport`      | enum   | Must match the entry's `sport` field.                                |
| `team_logo.url`        | string | Logo image URL — fetched on first render and cached.                 |
| `team_logo.sportsdb_leagueid`, `apisportsid`, `sportsdbid`, `sportsipyid` | int | Legacy IDs from the Python era. Kept for config compatibility; not all are still used by Rust. |

Layout: scrolling home/away names on top, logos + score in the middle,
scrolling division standings on the bottom. Middle-line color: green ⇐
home winning, red ⇐ home losing, white ⇐ tied or scheduled.

Data source: ESPN's public scoreboard + standings endpoints
(`site.api.espn.com`). No API key required.

#### Golf

```yaml
- run: true
  sport: golf
  tour: pga    # optional; default: pga
```

| `tour` value | What it shows               |
| ------------ | --------------------------- |
| `pga`        | PGA Tour leaderboard        |
| `lpga`       | LPGA Tour leaderboard       |
| `champions`  | PGA Tour Champions          |
| `korn`       | Korn Ferry Tour             |
| `liv`        | LIV Golf                    |

Top 5 only. Off-season shows a two-line placeholder. Score colors: red ⇐
under par, white ⇐ even, yellow ⇐ over par. Data source: ESPN.

#### Formula 1

```yaml
- run: true
  sport: f1
```

No extra fields. Header shows the next race + circuit + date; below it,
a gold/silver/bronze podium of the top 3 in the constructor standings
plus a scrolling tail with positions 4–10. Off-season shows a "season
ended" placeholder.

Data source: <https://api.jolpi.ca> (Ergast-compatible). Two parallel
requests per refresh: `current/next.json` + `current/driverStandings.json`.

---

### `iss`

Live great-circle distance from your location to the International Space
Station. Flips to a magenta `OVERHEAD` banner when the ISS is inside
your visibility footprint (i.e. above your local horizon right now).

```yaml
iss:
  run: true
  lat: 40.7128       # your latitude, decimal degrees
  lon: -74.0060      # your longitude, decimal degrees
```

| Field | Type  | Required | Notes                                                                 |
| ----- | ----- | -------- | --------------------------------------------------------------------- |
| `run` | bool  | yes      |                                                                       |
| `lat` | float | yes      | Decimal degrees, `-90..90`. No IP-based fallback for this module.     |
| `lon` | float | yes      | Decimal degrees, `-180..180`.                                         |

Refresh: 30 s. Data source: <https://api.wheretheiss.at/v1/satellites/25544>
(public, no auth). The `footprint` field from the API drives the
overhead check — no hand-rolled elevation math.

---

### `quake`

Top-magnitude earthquake from the last 24h of the USGS feed, or a
`QUIET / no events 24h` tile when the feed is empty. Magnitude is
color-coded — green under 4, amber 4–6, red 6 and up.

```yaml
quake:
  run: true
  feed: significant_day   # significant_day | m45_day | m25_day | all_day
```

| Field  | Type | Required | Notes                                                                                  |
| ------ | ---- | -------- | -------------------------------------------------------------------------------------- |
| `run`  | bool | yes      |                                                                                        |
| `feed` | enum | no       | Defaults to `significant_day` (quietest). The others raise event volume substantially. |

Refresh: 5 min. Data source:
<https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/> (public,
no auth). Place names that don't fit the panel word-wrap to two lines;
the second line marquees when the full text overflows.

---

### `aurora`

NOAA's planetary K-index as a single big digit (0–9) color-coded by
storm intensity, with a 9-block scale bar showing the gradient. An
`AURORA LIKELY` cyan banner appears whenever Kp meets the configured
alert threshold.

```yaml
aurora:
  run: true
  alert_threshold: 5    # 1–9; defaults to 5 (NOAA G1 minor storm)
```

| Field             | Type | Required | Notes                                                                                |
| ----------------- | ---- | -------- | ------------------------------------------------------------------------------------ |
| `run`             | bool | yes      |                                                                                      |
| `alert_threshold` | int  | no       | Defaults to 5. Lower (3, 4) catches high-latitude viewers; higher (6+) for far-south.|

Color bands match NOAA's G-scale: green Kp 0–3, amber 4, violet 5–6,
red 7–9. Refresh: 5 min. Data source:
<https://services.swpc.noaa.gov/json/planetary_k_index_1m.json> (public,
no auth).

---

### `flights`

Live count of aircraft within a configurable radius, plus a detail row
for the closest one: cyan callsign (or ICAO24 hex when no callsign is
broadcast), flight level (`FLnnn` for altitude in hundreds of feet, or
`GND` when the aircraft is on the ground), distance in km, and
8-octant bearing (`N`, `NE`, `E`, …). Long callsigns marquee.

```yaml
flights:
  run: true
  lat: 40.7128
  lon: -74.0060
  radius_km: 80
```

| Field       | Type  | Required | Notes                                                            |
| ----------- | ----- | -------- | ---------------------------------------------------------------- |
| `run`       | bool  | yes      |                                                                  |
| `lat`       | float | yes      | Decimal degrees, `-90..90`.                                      |
| `lon`       | float | yes      | Decimal degrees, `-180..180`.                                    |
| `radius_km` | float | no       | Defaults to 80; allowed range 1..500.                            |

Refresh: 60 s. Data source: <https://opensky-network.org> (anonymous
bbox endpoint, no auth). The bbox is computed locally and the result
is post-filtered by Haversine distance to enforce the circle; only
aircraft with a usable lat/lon are counted. OpenSky's anonymous tier
is credit-limited (~400 req/day) — a single flights tile at 60 s
will exceed that nominal budget, but small bbox queries tend to be
tolerated past the soft cap. Use a smaller `radius_km` if you start
seeing 429s.

---

### `launch`

Live countdown to the next upcoming orbital launch. Three visual modes
by remaining time: **T-far** (`T-2d 14h`, cyan) when more than 24h
away, **T-near** (`T-03:42:11`, amber) under 24h, and **T-imminent**
(`T-00:00:08`, flashing red) in the final 60 seconds. After liftoff
the panel switches to a green `LIFT-OFF` banner and lingers so the
moment doesn't rotate off the panel mid-launch.

```yaml
launch:
  run: true
  agency_filter: []         # case-insensitive substrings, e.g. ["SpaceX", "Rocket Lab"]
```

| Field           | Type      | Required | Notes                                                                                                  |
| --------------- | --------- | -------- | ------------------------------------------------------------------------------------------------------ |
| `run`           | bool      | yes      |                                                                                                        |
| `agency_filter` | string[]  | no       | Empty (default) = every upcoming launch worldwide. Otherwise, provider name must contain one of these. |

The countdown ticks at 1 fps and is computed at render time from the
cached `net` timestamp — the collector polls once per 30 minutes, well
inside Launch Library 2's anonymous ~15 req/hr limit. Provider, vehicle
and mission text truncates with `…` when it doesn't fit the 64-px row.

Data source: <https://ll.thespacedevs.com/2.2.0/launch/upcoming/>
(public, no auth).

---

### `hass`

Display any Home Assistant entity on the panel — sensors, binary
sensors, switches, anything that has a `state`. Multi-instance: one
entry per entity. Layout adapts: numeric states render as
`72.4 °F` (state + unit) in sage green; text states render uppercased
as `OPEN`/`ON`/`UNAVAILABLE`. When `alarm_state` is set and the
entity's state matches it, the state row flips to red — useful for
flagging tripped sensors at a glance.

```yaml
hass:
  - run: true
    base_url: http://homeassistant.local:8123
    token: YOUR_HASS_LONG_LIVED_TOKEN
    entity_id: sensor.kitchen_temp
    label: KITCHEN
  - run: true
    base_url: http://homeassistant.local:8123
    token: YOUR_HASS_LONG_LIVED_TOKEN
    entity_id: binary_sensor.garage_door
    label: GARAGE
    alarm_state: open
```

| Field           | Type     | Required | Notes                                                                                            |
| --------------- | -------- | -------- | ------------------------------------------------------------------------------------------------ |
| `run`           | bool     | yes      |                                                                                                  |
| `base_url`      | string   | yes      | HASS root, e.g. `http://homeassistant.local:8123` (no trailing slash needed; we strip it).      |
| `token`         | string   | yes      | Long-lived access token. Generate via Profile → Security → Long-Lived Access Tokens.            |
| `entity_id`     | string   | yes      | Any HASS entity id, e.g. `sensor.kitchen_temp`, `binary_sensor.front_door`, `switch.lamp_1`.    |
| `label`         | string   | no       | Display override. Defaults to `attributes.friendly_name`, then the bare entity id.              |
| `alarm_state`   | string   | no       | Case-insensitive. When the entity's state matches, the state row renders in `alarm_color`.      |
| `nominal_color` | [u8;3]   | no       | RGB for the steady-state row. Defaults to `[120, 220, 120]` (sage green).                       |
| `alarm_color`   | [u8;3]   | no       | RGB for the alarm-tripped row. Defaults to `[255, 60, 60]` (red).                               |

Refresh: 30 s. Data source: bearer-auth GET against your local HASS
REST API at `<base_url>/api/states/<entity_id>`. Footer shows
"updated Ns ago" for numeric entities, "since Nm ago" for text/binary
entities — both computed live from `last_changed`.

---

## Environment variables

| Variable               | Effect                                                                                                |
| ---------------------- | ----------------------------------------------------------------------------------------------------- |
| `OHMYOLED_MATRIX_MODE` | `auto` (default), `test` (force terminal renderer), `hardware` (force GPIO; fall back to terminal).   |
| `RUST_LOG`             | `error` (default), `warn`, `info`, `debug`, `trace`. Or `module=level` (e.g. `oledlib::api=debug`).   |
| `RUST_LOG_STYLE`       | `always` (default), `never`, `auto`. Controls ANSI color in logs.                                     |
| `DEV`                  | If set (any value), `RGBMatrix::test` is forced. Equivalent to `OHMYOLED_MATRIX_MODE=test` but coarser. |

`OHMYOLED_MATRIX_MODE` always wins if both are set.

---

## Fonts

`src/sh/install.sh` lays these down in `/usr/share/fonts/`:

| File                | Used by                                  |
| ------------------- | ---------------------------------------- |
| `4x6.bdf`           | `time` (BDF bitmap)                      |
| `04B_03B_.TTF`      | `weather`, `stock`, `sport`, `golf`, `f1`, `iss`, `quake`, `aurora`, `flights`, `launch`, `hass` (body text + iss big number) |
| `04b24.otf`         | `sport` (big score numerals), `aurora` (big Kp digit) |
| `weathericons.ttf`  | `weather`, `stock` (icon glyphs)         |
| `retro_computer.ttf`| `weather` (accent text)                  |

If any of these are missing the module will fail to construct and skip
itself at startup with a `font: ...` error in the log. Custom paths can
be wired via the `*Fonts` builders (`WeatherFonts`, `StockFonts`,
`IssFonts`, etc.) but there's no on-disk config knob for that yet.

---

## Gotchas

- **`/etc/ohmyoled/` requires root** to write to. Either `sudo` the
  `--create_config` invocation or pass `--config ~/ohmyoled.yaml` and
  point the binary somewhere writable.
- **OpenWeather OneCall 3.0** requires a credit card on file (still free
  up to 1000 calls/day). The legacy 2.5 endpoint is dead — don't use an
  old key from it.
- **NWS is US-only.** Outside the US, the points-lookup step returns
  404 and the module skips itself at startup with a log line.
- **AccuWeather free tier is small** (≈50 calls/day). The renderer refreshes
  every 10 minutes — that's ~144 calls/day. Use OpenWeather or Pirate for
  always-on displays unless you're on a paid AccuWeather tier.
- **Finnhub free tier rate-limits to 60 calls/min.** Per-symbol refresh
  is 30 s, so up to ~120 calls/hour per symbol. Fine unless you have a
  dozen symbols configured.
- **ESPN endpoints are unofficial.** They've been stable for years but
  there's no SLA. Schema changes will surface as deserialize errors in
  the log; the module renders the previous good frame in the meantime.
- **`sport_logo` URLs are fetched once per process.** A bad URL means a
  blank logo slot until you restart.
- **Brightness above ~80** can pull more current than a Pi USB supply
  delivers, causing the Pi to reboot mid-render. Use a dedicated 5V
  supply for the panel and stay ≤ 60 if you're powering both off the Pi.
- **`oled_slowdown`** is what to bump if you see flickering or noise. Pi
  4 typically wants 3 or 4; Pi 5 may need higher.
- **`run: false`** anywhere is the supported way to disable without
  deleting the section. Sections missing entirely also work — every
  field has a default empty list.
- **Single-or-list parsing** means `weather: {…}` and `weather: [{…}]`
  both work. The list form is required if you want more than one entry.
- **Sport entries with `sport: golf` or `sport: f1`** ignore any
  `team_logo` you accidentally include. Team-sport entries without
  `team_logo` fail to deserialize.
- **Time `time_format`** treats the literal string `"null"` as
  unset (legacy quirk preserved from the JSON-only era).
- **YAML config files** can use either lowercase enum values (`pga`,
  `openweather`) or quoted strings; TOML requires the quoted form. JSON
  requires double quotes everywhere — standard JSON rules.

---

## Architecture

Every module is a `Module<Collector, Renderer>` pair. The two traits live
in `src/lib/api/collector.rs` and `src/lib/matrix/renderer.rs`:

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
    async fn render(&mut self, matrix: &mut RGBMatrix, data: &Self::Data) -> Result<(), RenderError>;
}
```

The `Module<C, R>` bundle is `dyn`-erased to `Box<dyn DynModule>` and
fed to the scheduler at `src/lib/modules/scheduler.rs`, which round-robins
through them forever.

### Adding a new API + matrix

1. Add a `*Collector` under `src/lib/api/<name>/` that implements `Collector`.
2. Add a `*Matrix` under `src/lib/matrix/<name>.rs` that implements `Renderer`.
3. Add a config section to `src/lib/modules/registry.rs::RegistryConfig`
   plus a `build_<name>` async helper.
4. Add a `for` loop in `registry::build`.

That's the entire wiring change. See `src/lib/matrix/time.rs` for the
minimal end-to-end example.

---

## Development

```bash
# Library + bin
cargo build

# Run all unit tests (60+ in the library)
cargo test --lib

# Doc tests
cargo test --doc

# Lint
cargo clippy --lib --bin ohmyoled --examples

# Per-module visual smoke tests (render to ANSI in the terminal)
cargo run --example time_render_check
cargo run --example weather_render_check
cargo run --example stock_render_check
cargo run --example sport_render_check
cargo run --example golf_f1_smoke

# Verify the three config formats parse identically
cargo run --example config_formats

# Verify multi-instance config handling
cargo run --example multi_instance_check
```

### Devcontainer

A devcontainer is provided under `.devcontainer/` with Rust + Python 3.11
preinstalled. Note: linking the full test binary inside the devcontainer
can OOM the linker — run `cargo test --lib` rather than `cargo test` to
avoid building the bin-test artifact. This isn't an issue on the Pi.
