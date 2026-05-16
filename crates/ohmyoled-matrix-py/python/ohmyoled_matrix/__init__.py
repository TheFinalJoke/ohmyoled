"""
ohmyoled_matrix — Python bindings for the OhMyOled LED matrix library.

Import RGBMatrix and RGBMatrixOptions to drive the display:

    from ohmyoled_matrix import RGBMatrix, RGBMatrixOptions

For test/dev mode (renders to terminal, no GPIO needed):

    matrix = RGBMatrix(options=opts, test_mode=True)

Or set the env var:

    OHMYOLED_MATRIX_MODE=test python3 -m ohmyoled.main
"""
from ._ohmyoled_matrix import (
    RGBMatrix,
    RGBMatrixOptions,
    TerminalMatrix,
)

__all__ = ["RGBMatrix", "RGBMatrixOptions", "TerminalMatrix"]
