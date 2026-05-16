# Vendored: rpi-led-panel

This directory is reserved for the vendored source of
[rpi-led-panel](https://github.com/EmbersArc/rpi_led_panel) v0.8.1.

## Provenance

| Field        | Value                                                               |
|--------------|---------------------------------------------------------------------|
| Upstream     | https://github.com/EmbersArc/rpi_led_panel                         |
| Version      | 0.8.1                                                               |
| Commit hash  | (fill in after vendoring)                                           |
| License      | GPL-2.0                                                             |
| Vendored by  | ohmyoled project                                                    |

## How to vendor

```bash
# Clone the upstream repo at the pinned tag
git clone --depth 1 --branch v0.8.1 https://github.com/EmbersArc/rpi_led_panel /tmp/rpi_led_panel

# Copy source into this directory
cp -r /tmp/rpi_led_panel/src/* crates/ohmyoled-matrix/src/hardware/

# Copy the upstream licence
cp /tmp/rpi_led_panel/LICENSE crates/ohmyoled-matrix/src/hardware/LICENSE-rpi-led-panel

# Record the exact commit hash above
git -C /tmp/rpi_led_panel rev-parse HEAD
```

Then replace the stub `HardwareBackend::init` in `mod.rs` with the real
`rpi_led_panel::RGBMatrix::new(options)` call.

## Our modifications

- Added a "Vendored from rpi-led-panel — see VENDORED.md" comment at the top of
  each copied source file.
- `mod.rs` (this crate) wraps the library behind the `Backend` trait; all other
  vendored files are unmodified.
