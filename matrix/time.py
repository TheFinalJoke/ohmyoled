#!/usr/bin/env python3

from PIL.Image import Image
from matrix.matrix import Matrix, MatrixBase, FontException
from datetime import date, datetime
from rgbmatrix import RGBMatrix, RGBMatrixOptions, graphics
from PIL import Image, ImageDraw, ImageFont
import sys
import time 

class TimeMatrix(MatrixBase):
    def __init__(self, matrix) -> None:
        super().__init__(matrix)
        self.matrix = matrix
        self.logger.debug("Time Matrix Initalized")
    def return_time(self, fmt: str):
        return datetime.now().strftime(fmt) 

    async def poll_api(self):
        """
        Function that does not poll since this a time
        """
        self.logger.debug("No Api call reqiured for time module")
        return None
    def render(self, poll):
        # Build something that Loads in corner for all the modules loaded
        self.logger.info("Running Module TimeMatrix")
        counter = 0
<<<<<<< HEAD
        while counter < 10:
=======
        while counter < 15:
>>>>>>> dev
            self.logger.debug(f'Counter for module run {counter}')
            font = ImageFont.truetype("/usr/share/fonts/truetype/noto/NotoMono-Regular.ttf", 10)
            self.set_image(Image.new("RGB", (64, 32)))
            self.set_draw(ImageDraw.Draw(self.get_image))
<<<<<<< HEAD
            self.draw_text((3, 5), f"{self.return_time('%m/%d/%Y')}", font=font, fill=(255,255,255))
            self.draw_text((8, 16), f"{self.return_time('%I:%M:%S')}", font=font, fill=(255,255,255))
=======
            self.draw_text((3, 5), f"{self.return_time('%m/%d/%Y')}", font=font, fill=(71, 181, 214,255))
            self.draw_text((8, 16), f"{self.return_time('%I:%M:%S')}", font=font, fill=(71, 181, 214,255))
>>>>>>> dev
            self.render_image()
            counter = counter + 1
            time.sleep(1)