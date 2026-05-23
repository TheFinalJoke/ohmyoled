# ohmyoled-matrix usage examples

## Show a solid-colour image

```rust
use ohmyoled_matrix::{RGBMatrix, MatrixOptions};
use image::{RgbImage, Rgb};

let mut matrix = RGBMatrix::test(MatrixOptions::default());
let mut img = RgbImage::new(64, 32);
for px in img.pixels_mut() { *px = Rgb([0, 128, 255]); }  // blue
matrix.set_image(&img, 0, 0);
```

## Draw text with a BDF font

```rust
use ohmyoled_matrix::{RGBMatrix, MatrixOptions, Color};
use ohmyoled_matrix::graphics::{Font, draw_text};
use image::RgbImage;

let mut font = Font::new();
font.load_font("/etc/ohmyoled/fonts/4x6.bdf").unwrap();

let mut img = RgbImage::new(64, 32);
draw_text(&mut img, &font, 2, 10, Color::WHITE, "Hello");

let mut matrix = RGBMatrix::test(MatrixOptions::default());
matrix.set_image(&img, 0, 0);
```

## Run in test mode on your laptop

```bash
# Via env var (no code change)
OHMYOLED_MATRIX_MODE=test python3 -m ohmyoled.main

# Via DEV flag (existing convention)
DEV=1 python3 -m ohmyoled.main
```

## Run on a Raspberry Pi

No special flags needed. When running on Pi hardware, `RGBMatrix::new` detects
GPIO access and uses the hardware backend automatically.

```bash
python3 -m ohmyoled.main
```

To force terminal mode even on Pi (e.g. for debugging):

```bash
OHMYOLED_MATRIX_MODE=test python3 -m ohmyoled.main
```

## Build Python bindings after a code change

```bash
cd crates/ohmyoled-matrix-py
maturin develop          # debug build, fastest
maturin develop --release  # optimised build for install.sh
```
