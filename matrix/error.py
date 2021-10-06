#!/usr/bin/env python3

from matrix.matrix import Matrix
from PIL import ImageFont
import time

class ErrorMatrix(Matrix):
    def __init__(self, matrix, config, failed_matrix) -> None:
        super().__init__(config)
        self.matrix = matrix
        self.config = config
        self.failed_matrix = failed_matrix
    
    def __str__(self):
        return "ErrorMatrix"

    async def render(self):
        self.logger.info("Running Error Matrix")
        font: ImageFont = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8)
        self.clear()
        txt = f"Error ON \n {self.failed_matrix}"
        self.draw_text((2, 16), txt, font, color=(255,255,255))
        await self.render_image()
        time.sleep(45)