# ohmyoled-matrix Python bindings

Python interface to the `ohmyoled-matrix` Rust library, built with [maturin](https://github.com/PyO3/maturin).

## Install (devcontainer)

```bash
cd crates/ohmyoled-matrix-py
maturin develop
```

## Basic usage

```python
from ohmyoled_matrix import RGBMatrix, RGBMatrixOptions
from PIL import Image, ImageDraw

opts = RGBMatrixOptions()
opts.cols = 64
opts.rows = 32
opts.brightness = 60
opts.hardware_mapping = "adafruit-hat"

# test_mode=True → renders to terminal; omit on real Pi hardware
matrix = RGBMatrix(options=opts, test_mode=True)

img = Image.new("RGB", (64, 32))
draw = ImageDraw.Draw(img)
draw.text((2, 10), "Hi!", fill=(255, 255, 0))

matrix.SetImage(img)
```

## Using fonts (BDF)

```python
from ohmyoled_matrix import graphics

font = graphics.Font()
font.LoadFont("/etc/ohmyoled/fonts/4x6.bdf")
# font is now ready to pass to draw_text (Rust side) or used as a reference
```

## Checking the docs

```python
import ohmyoled_matrix
help(ohmyoled_matrix.RGBMatrix)
help(ohmyoled_matrix.RGBMatrixOptions)
help(ohmyoled_matrix.graphics.Font)
```
