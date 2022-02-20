#!/usr/bin/env python3

from PIL.Image import Image
from ohmyoled.matrix.matrix import MatrixBase
from datetime import datetime
from PIL import Image, ImageDraw, ImageFont
from enum import Enum
import time 

class TimeFormat(Enum):
    TWELEVE = 1
    TWENTYFOUR = 2
class TimeMatrix(MatrixBase):
    def __init__(self, matrix, config) -> None:
        super().__init__(matrix)
        self.matrix = matrix
        self.config = config
        self.logger.debug("Time Matrix Initalized")

    def __str__(self) -> str:
        return "TimeMatrix"

    def return_time(self, fmt: str) -> datetime:
        return datetime.now().strftime(fmt) 

    async def poll_api(self) -> None:
        """
        Function that does not poll since this a time
        """
        self.logger.info("No Api call required for time module")
        return None
    
    def build_fmt(self) -> str:
        return "%I:%M:%S %p" if TimeFormat.TWELEVE else "%H:%M:%S"  

    async def render(self, poll: None, loop):
        # Build something that Loads in corner for all the modules loaded
        self.logger.info("Running Module TimeMatrix")
        counter = 0
        while counter < 30:
            self.logger.debug(f'Counter for module run {counter}')
            font = ImageFont.truetype("/usr/share/fonts/truetype/noto/NotoMono-Regular.ttf", 10)
            self.set_image(Image.new("RGB", (64, 32)))
            self.set_draw(ImageDraw.Draw(self.get_image))
            self.draw_text((3, 5), f"{self.return_time('%m/%d/%Y')}", font=font, fill=eval(self.config.get('color')))
            self.draw_text((8, 16), f"{self.return_time('%I:%M:%S')}", font=font, fill=eval(self.config.get('color')))
            await self.render_image()
            counter = counter + 1
            time.sleep(1)