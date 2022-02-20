#!/usr/bin/env python3

from ohmyoled.matrix.matrix import Matrix
from PIL import ImageFont
import time

class ErrorMatrix(Matrix):
    def __init__(self, matrix, logger, failed_matrix) -> None:
        self.matrix = matrix
        self.logger = logger
        self.failed_matrix = failed_matrix

    def __str__(self):
        return "ErrorMatrix"

    async def render(self):
        self.logger.info("Running Error Matrix")
        font: ImageFont = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8)
        self.clear()
        txt = f"There was \n An Exception \n {self.failed_matrix}"
        self.reload_image()
        self.draw_text((2, 4), txt, font, fill=(255,255,255))
        await self.render_image()
        time.sleep(45)