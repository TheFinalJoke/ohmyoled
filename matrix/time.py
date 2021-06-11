#!/usr/bin/env python3

from matrix.matrix import Matrix, MatrixBase, FontException, Canvas
from datetime import datetime
from rgbmatrix import RGBMatrix, RGBMatrixOptions, graphics
import sys
import time 

class TimeMatrix(MatrixBase):
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
        graphics.DrawText(canvas, font, 2, 16, graphics.Color(0,0,255), timestamp)
        counter = 0
        while counter < 5:
            self.matrix.SwapOnVSync(canvas)
            time.sleep(1)
            counter = counter + 1
        print('done')


        

        
    