#!/usr/bin/python3

# ABS_Matrix -> Matrix_Module
#
#
#
from abc import abstractmethod
import configparser
import logging
from sys import exec_prefix
from PIL import Image, ImageDraw, ImageFont
from typing import Tuple
from rgbmatrix import (
    RGBMatrixOptions, 
    RGBMatrix,
    graphics
)
stream_formatter = logging.Formatter(
    "%(levelname)s:%(asctime)s:%(module)s:%(message)s"
)
sh = logging.StreamHandler()
filehandler = logging.FileHandler("/home/nickshorter/ohmyoled.log","a")
sh.setFormatter(stream_formatter)
filehandler.setFormatter(stream_formatter)
logger = logging.getLogger(__name__)
logger.addHandler(sh)
logger.addHandler(filehandler)
logger.setLevel(logging.DEBUG)

class ABSMatrix():
    def __init__(self) -> None:
        pass 
    @abstractmethod
    def poll_api(self): pass 
    
    @abstractmethod
    def positioning(self): pass

    @abstractmethod
    def gather(self): pass 

    @abstractmethod
    def render(self): pass 

class FontException(Exception):
    pass

class Matrix(ABSMatrix):
    def __init__(self, config) -> None:
        self.config = config
        self.logger = logger

    
    def get_font_graphics(self, font_file):
        font = graphics.Font()
        font.LoadFont(f"fonts/{font_file}")
        return font

    def get_logger(self):
        return self.logger

    def set_image(self, image):
        self.image = image
    
    def set_draw(self, draw):
        self.draw = draw

    @property
    def get_draw(self):
        return self.draw

    @property
    def get_matrix(self):
        return self.matrix
    
    @property
    def get_image(self):
        return self.image

    def reload_image(self):
        self.set_image(Image.new("RGB", (64, 32)))
        self.set_draw(ImageDraw.Draw(self.image))
        self.get_image
        self.get_draw

    def render_image(self):
        self.matrix.SetImage(self.get_image)
    def draw_rectangle(self):
        self.draw.rectangle([(0,0), (63,31)])
    def draw_text(self, align, text, font, fill=None):
        self.draw.text(
            align,
            text,
            font=font,
            fill=fill
        )
    def draw_textBox(self):
        pass
    def clear(self):
        self.matrix.Clear()

class Canvas(Matrix):
    def __init__(self, matrix) -> None:
        super().__init__(matrix)
        self.matrix = matrix
        self.canvas = self.matrix.CreateFrameCanvas()
    
    def get_logger(self):
        return self.logger
    
    def clear(self):
        self.matrix.Clear()
    

class MatrixBase(Matrix):
    def __init__(self, matrix) -> None:
        super().__init__(matrix)
        self.matrix = matrix

    def get_logger(self):
        return self.logger
    
    def clear(self):
        self.matrix.Clear()