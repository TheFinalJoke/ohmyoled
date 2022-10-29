#!/usr/bin/python3

# ABS_Matrix -> Matrix_Module
from abc import abstractmethod
import asyncio
import functools
import logging
import numpy as np
from PIL import Image, ImageDraw
import PIL.Image as img_types
import PIL.ImageDraw as draw_types
import typing
from collections import deque
from rgbmatrix import (
    RGBMatrix,
    graphics
)
stream_formatter = logging.Formatter(
    "%(levelname)s:%(asctime)s:%(module)s:%(message)s"
)
sh = logging.StreamHandler()
filehandler = logging.FileHandler("/var/log/ohmyoled.log","a")
sh.setFormatter(stream_formatter)
filehandler.setFormatter(stream_formatter)
logger = logging.getLogger(__name__)
logger.addHandler(sh)
logger.addHandler(filehandler)
logger.setLevel(logging.INFO)
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

class FailedApiException(Exception):
    pass

class TerminalMatrix(ABSMatrix):
    def __init__(self) -> None:
        super().__init__()
        self.logger = logger
        self.logger.debug(f"Loggerin is set to {self.logger.getEffectiveLevel()}")

    def get_font_graphics(self, font_file):
        font = graphics.Font()
        font.LoadFont(f"/etc/ohmyoled/fonts/{font_file}")
        return font

    def make_new_image(self, size: typing.Tuple[int]) -> img_types.Image:
        return Image.new("RGB", size)  # type: ignore

    def add_image_to_images(self, image: img_types.Image) -> None:
        self.images.append(image)
    
    def paste_image(self, image: img_types.Image, position: typing.Tuple[int, int]) -> None:
        self.image.paste(image, position)

    def reset_image_queue(self) -> None:
        self.images = deque([self.image])

    def get_logger(self):
        return self.logger

    def set_image(self, image) -> None:
        """
        When working with a single image
        """
        self.image = image

    def set_draw(self, draw):
        self.draw = draw

    @property
    def get_images(self) -> typing.Deque:
        return self.images
    
    @property
    def get_draw(self) -> draw_types.ImageDraw:
        """
        For Each image you have to 
        set draw. Each image has its own buffer
        """
        return self.draw

    @property
    def get_matrix(self) -> RGBMatrix:
        return self.matrix
    
    @property
    def get_image(self) -> img_types.Image:
        """
        When working with a single image
        """
        return self.image

    @property
    def get_image_size(self) -> typing.Tuple[int]:
        """
        When Working with a single Image
        """
        return self.image.size

    def set_matrix(self, matrix) -> None:
        self.get_matrix = matrix
    
    def create_matrix(self, options) -> None:
        return None

    def reload_image(self) -> None:
        """
        Clears out all images and starts with
        Clear slate
        """
        self.set_image(Image.new("RGB", (64, 32)))
        self.set_draw(ImageDraw.Draw(self.image))
        self.get_image
        self.get_draw
        self.reset_image_queue()


    def image_resize(self, width, height) -> None:
        """
        When Working with a single image
        """
        self.set_image(self.image.resize((width, height), Image.ANTIALIAS))
        self.set_draw(ImageDraw.Draw(self.image))

    async def render_image(self, loop=None, xoffset=0, yoffset=0):
        if not loop:
            loop = asyncio.get_event_loop()
        height = self.image.height
        width = self.image.width
        img_arr = np.asarray(self.image)
        # sys.
        
        await loop.run_in_executor(
            None,
            functools.partial(
                self.matrix.SetImage,
                self.get_image,
                offset_x=xoffset,
                offset_y=yoffset
            )
        )
    def nonasync_render_image(self, loop=None, xoffset=0, yoffset=0) -> None:
        self.matrix.SetImage(self.get_image, offset_x=xoffset, offset_y=yoffset)
 
    def draw_rectangle(self, position: typing.List[typing.Tuple]) -> None:
        self.draw.rectangle(position)
    def draw_line(self, pos):
        self.draw.line(pos)

    def draw_text(self, align, text, font, fill=None) -> None:
        self.draw.text(
            align,
            text,
            font=font,
            fill=fill
        )
    def draw_multiline_text(self, align, text, font, fill=None, spacing=4) -> None:
        self.draw.multiline_text(
            align,
            text,
            fill,
            font,
            spacing=spacing
        )
    def get_multiline_textsize(self, text: str) -> typing.Tuple[int, int]:
        return self.draw.multiline_textsize(text)

    def get_text_size(self, text: str) -> typing.Tuple[int]:
        return self.draw.textsize(text)

    def create_double_buffer(self):
        return self.matrix.CreateFrameCanvas()
    
    def clear(self) -> None:
        self.matrix.Clear()
    
class Matrix(ABSMatrix):
    def __init__(self, config) -> None:
        self.config = config
        self.logger = logger
        self.logger.debug(f"Logger is set to {self.logger.getEffectiveLevel()}")
    def get_font_graphics(self, font_file):
        font = graphics.Font()
        font.LoadFont(f"/etc/ohmyoled/fonts/{font_file}")
        return font

    def make_new_image(self, size: typing.Tuple[int]) -> img_types.Image:
        return Image.new("RGB", size)

    def add_image_to_images(self, image: img_types.Image) -> None:
        self.images.append(image)
    
    def paste_image(self, image: img_types.Image, position: typing.Tuple[int, int]) -> None:
        self.image.paste(image, position)

    def reset_image_queue(self) -> None:
        self.images = deque([self.image])

    def get_logger(self):
        return self.logger

    def set_image(self, image) -> None:
        """
        When working with a single image
        """
        self.image = image

    def set_draw(self, draw):
        self.draw = draw

    @property
    def get_images(self) -> typing.Deque:
        return self.images
    
    @property
    def get_draw(self) -> draw_types.ImageDraw:
        """
        For Each image you have to 
        set draw. Each image has its own buffer
        """
        return self.draw

    @property
    def get_matrix(self) -> RGBMatrix:
        return self.matrix  # type: ignore
    
    @property
    def get_image(self) -> img_types.Image:
        """
        When working with a single image
        """
        return self.image

    @property
    def get_image_size(self) -> typing.Tuple[int]:
        """
        When Working with a single Image
        """
        return self.image.size

    def set_matrix(self, matrix) -> None:
        self.get_matrix = matrix
    
    def create_matrix(self, options) -> RGBMatrix:
        self.set_matrix(RGBMatrix(options)) 

    def reload_image(self) -> None:
        """
        Clears out all images and starts with
        Clear slate
        """
        self.set_image(Image.new("RGB", (64, 32)))
        self.set_draw(ImageDraw.Draw(self.image))
        self.get_image
        self.get_draw
        self.reset_image_queue()


    def image_resize(self, width, height) -> None:
        """
        When Working with a single image
        """
        self.set_image(self.image.resize((width, height), Image.ANTIALIAS))
        self.set_draw(ImageDraw.Draw(self.image))

    async def render_image(self, loop=None, xoffset=0, yoffset=0) -> None:
        if not loop:
            loop = asyncio.get_event_loop()
        await loop.run_in_executor(
            None,
            functools.partial(
                self.matrix.SetImage,
                self.get_image,
                offset_x=xoffset,
                offset_y=yoffset
            )
        )
    def nonasync_render_image(self, loop=None, xoffset=0, yoffset=0):
        self.matrix.SetImage(self.get_image, offset_x=xoffset, offset_y=yoffset)
 
    def draw_rectangle(self, position: typing.List[typing.Tuple]):
        """
        [typing.List(typing.Tuple,)]
        """
        self.draw.rectangle(position)
    def draw_line(self, pos):
        self.draw.line(pos)

    def draw_text(self, align, text, font, fill=None) -> None:
        self.draw.text(
            align,
            text,
            font=font,
            fill=fill
        )
    def draw_multiline_text(self, align, text, font, fill=None, spacing=4) -> None:
        self.draw.multiline_text(
            align,
            text,
            fill,
            font,
            spacing=spacing
        )
    def get_multiline_textsize(self, text: str) -> typing.Tuple[int, int]:
        return self.draw.multiline_textsize(text)

    def get_text_size(self, text: str) -> typing.Tuple[int]:
        return self.draw.textsize(text)

    def create_double_buffer(self):
        return self.matrix.CreateFrameCanvas()
    
    def clear(self) -> None:
        self.matrix.Clear()

class MatrixBase(Matrix):
    def __init__(self, matrix) -> None:
        super().__init__(matrix)
        self.matrix = matrix

    def get_logger(self):
        return self.logger
    
    def clear(self):
        self.matrix.Clear()
