#!/usr/bin/python3

# ABS_Matrix -> Matrix_Module
#
#
#
from abc import abstractmethod
import configparser
import logging
from sys import exec_prefix
from rgbmatrix import (
    RGBMatrixOptions, 
    RGBMatrix,
    graphics
)
import rgbmatrix

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
        super().__init__()
        self.config = config
        self.options = self.poll_rgbmatrix()
        self.matrix = RGBMatrix(options=self.poll_rgbmatrix())

    def poll_rgbmatrix(self):
        options = self.config['matrix']
        rgboptions = RGBMatrixOptions()
        rgboptions.cols = 64
        rgboptions.rows = 32
        rgboptions.chain_length = options.getint('parallel')
        rgboptions.parallel = options.getint('chain_length')
        rgboptions.gpio_slowdown = options.getint('oled_slowdown')
        rgboptions.brightness = options.getint('brightness')
        rgboptions.hardware_mapping = 'adafruit-hat'
        return rgboptions
    
    def get_font(self, font_file):
        font = graphics.Font()
        font.LoadFont(f"submodules/rgbmatrix/fonts/{font_file}")
        return font

class Canvas(Matrix):
    def __init__(self, matrix) -> None:
        super().__init__()
        self.matrix = matrix
        self.canvas = self.matrix.CreateFrameCanvas()

class MatrixBase(Matrix):
    def __init__(self, matrix) -> None:
        super().__init__()
        self.matrix = matrix
        self.logger = logger
        self.logger.setLevel(logging.DEBUG)

    def get_logger(self):
        return self.logger