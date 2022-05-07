from dataclasses import dataclass
import typing
from PIL import Image, ImageDraw

@dataclass
class StockImage():
    size: typing.Tuple[int]
    location: typing.Tuple[int]
    sub_images: typing.Optional[typing.Dict]
    image: Image
    drawing_image: ImageDraw
    