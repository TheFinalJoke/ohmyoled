#!/usr/bin/env python3

from matrix.matrix import Matrix, MatrixBase, FontException, Canvas
from datetime import date, datetime
from rgbmatrix import RGBMatrix, RGBMatrixOptions, graphics
import sys
import time 

class TimeMatrix(Canvas):
    def __init__(self, matrix, logger) -> None:
        self.matrix = matrix
        self.logger = logger
        self.logger.debug("Time Matrix Initalized")
    def return_time(self, fmt: str):
        return datetime.now().strftime(fmt) 

    def poll_api(self):
        """
        Function that does not poll since this a time
        """
        self.logger.debug("No Api call reqiured for time module")
        return None
    def render(self, poll):
        try:
            self.logger.info("Running Module TimeMatrix")
            self.logger.debug('Getting Top Font')
            top_font = self.get_font("tom-thumb.bdf")
            self.logger.debug('Getting bottom Font')
            bottom_font = self.get_font("5x8.bdf")
            if not top_font or not bottom_font:
                raise FontException
        except FontException:
            self.logger.critical("Font file is not found")
            sys.exit(1)
        canvas = self.matrix.CreateFrameCanvas()
        counter = 0
        while counter < 5:
            self.logger.debug(f'Counter for module run {counter}')
            canvas.Clear()
            # Top Line
            color = graphics.Color(74,3,54)    
            graphics.DrawText(canvas, top_font, 14, 12, color, f"{self.return_time('%m/%d/%Y')}")
            # Bottom Line
            graphics.DrawText(canvas, bottom_font, 13, 20, color, f"{self.return_time('%I:%M:%S')}")
            canvas = self.matrix.SwapOnVSync(canvas)
            counter = counter + 1
            time.sleep(1)


        

        
    