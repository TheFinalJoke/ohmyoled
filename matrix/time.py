#!/usr/bin/env python3

from matrix.matrix import Matrix, MatrixBase, FontException, Canvas
from datetime import datetime
from rgbmatrix import RGBMatrix, RGBMatrixOptions, graphics
import sys
import time 

class TimeMatrix(Canvas):
    def __init__(self, matrix) -> None:
        self.matrix = matrix
    def poll_api(self):
        return datetime.now().strftime('%Y-%m-%dT%H:%M:%SZ')
    def render(self):
        try:
            timestamp = self.poll_api()
            font = self.get_font("7x13.bdf")
            if not font:
                raise FontException
        except FontException:
            self.logger.critical("Font file is not found")
            sys.exit(1)
        canvas = self.matrix.CreateFrameCanvas()
        counter = 0
        while counter < 5:
            canvas.Clear()
            # Top Line
            font = graphics.Font()
            font.CharacterWidth(20)
            font.LoadFont("submodules/rgbmatrix/fonts/tom-thumb.bdf")
            color = graphics.Color(74,3,54)    
            graphics.DrawText(canvas, font, 14, 12, color, f"{datetime.now().strftime('%m/%d/%Y')}")
            # Bottom Line
            font = graphics.Font()
            font.CharacterWidth(10)
            font.LoadFont("submodules/rgbmatrix/fonts/5x8.bdf")
            graphics.DrawText(canvas, font, 13, 20, color, f"{datetime.now().strftime('%I:%M:%S')}")
            canvas = self.matrix.SwapOnVSync(canvas)
            counter = counter + 1
            time.sleep(1)


        

        
    