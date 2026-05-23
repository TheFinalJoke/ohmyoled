# ohmyoled-matrix

A Rust library for driving the OhMyOled LED matrix display. It has two backends:

- **Terminal mode** — renders each frame as coloured dots in your terminal using ANSI colour codes. Works on any machine; no hardware required. This is the official way to develop and test without a Raspberry Pi.
- **Hardware mode** — drives a real 64×32 RGB LED panel connected to a Raspberry Pi via GPIO.

The library picks the right backend automatically. If you're not on a Pi, it falls back to terminal mode and prints a warning.

---

## Quick Rust example

```rust
use ohmyoled_matrix::{RGBMatrix, MatrixOptions};
use image::RgbImage;

fn main() {
    let opts = MatrixOptions::default();          // 64×32, brightness 60, adafruit-hat
    let mut matrix = RGBMatrix::test(opts);       // always terminal mode

    let img = RgbImage::new(64, 32);              // black 64×32 image
    matrix.set_image(&img, 0, 0);                 // push to display
}
```

## Quick Python example

```python
from ohmyoled_matrix import RGBMatrix, RGBMatrixOptions
from PIL import Image

opts = RGBMatrixOptions()
opts.cols = 64
opts.rows = 32

matrix = RGBMatrix(options=opts, test_mode=True)  # terminal mode

img = Image.new("RGB", (64, 32), color=(255, 0, 0))  # red frame
matrix.SetImage(img)
```

---

## Choosing a mode

| How | What to do |
|-----|-----------|
| Always terminal | `RGBMatrix::test(opts)` in Rust / `RGBMatrix(opts, test_mode=True)` in Python |
| Env var | `OHMYOLED_MATRIX_MODE=test` — overrides any code-level setting |
| Auto (default) | `RGBMatrix::new(opts)` — tries hardware, falls back to terminal |
| Always hardware | `RGBMatrix::with_mode(opts, MatrixMode::Hardware)` |

The `DEV=1` environment variable used by `main.py` maps to `test_mode=True` automatically — no change needed to the existing invocation.

---

## Building

```bash
# Compile the Rust library (test mode only, no hardware deps)
cargo build -p ohmyoled-matrix --no-default-features

# Compile with the hardware backend stub enabled (default)
cargo build -p ohmyoled-matrix

# Install Python bindings into the active Python env
cd crates/ohmyoled-matrix-py && maturin develop
```

## Tests

```bash
cargo test -p ohmyoled-matrix
```
