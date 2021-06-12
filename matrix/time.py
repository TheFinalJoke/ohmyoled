#!/usr/bin/env python3

from matrix.matrix import Matrix, MatrixBase, FontException, Canvas
from datetime import datetime
from rgbmatrix import RGBMatrix, RGBMatrixOptions, graphics
import sys
import time 

class TimeMatrix(Canvas):
    def __init__(self, matrix) -> None:
        self.matrix = matrix
        self.logger = self.get_logger()
    def poll_api(self, fmt: str):
        return datetime.now().strftime(fmt)
    def render(self):
        try:
            top_font = self.get_font("tom-thumb.bdf")
            bottom_font = self.get_font("5x8.bdf")
            if not top_font or not bottom_font:
                raise FontException
        except FontException:
            self.logger.critical("Font file is not found")
            sys.exit(1)
        canvas = self.matrix.CreateFrameCanvas()
        counter = 0
        while counter < 5:
            canvas.Clear()
            # Top Line
            color = graphics.Color(74,3,54)    
            graphics.DrawText(canvas, top_font, 14, 12, color, f"{self.poll_api('%m/%d/%Y')}")
            # Bottom Line
            graphics.DrawText(canvas, bottom_font, 13, 20, color, f"{self.poll_api('%I:%M:%S')}")
            canvas = self.matrix.SwapOnVSync(canvas)
            counter = counter + 1
            time.sleep(1)


        

        
    