#!/usr/bin/env python
import time
import sys
from datetime import datetime

from rgbmatrix import RGBMatrix, RGBMatrixOptions, graphics, FrameCanvas
from PIL import ImageDraw, Image, ImageFont
"""
if len(sys.argv) < 2:
    sys.exit("Require an image argument")
else:
    image_file = sys.argv[1]
"""
image = Image.new("RGB", (64, 32))
#image = Image.open(image_file)
draw = ImageDraw.Draw(image)
font = ImageFont.truetype("/usr/share/fonts/truetype/noto/NotoMono-Regular.ttf", 10)
draw.text((1, 5), "hello", font=font)
#draw.rectangle((0, 0, 63, 31), fill=(0, 0, 0), outline=(0, 0, 255))
# Configuration for the matrix
options = RGBMatrixOptions()
options.rows = 32
options.cols = 64
options.chain_length = 1
options.parallel = 1
options.gpio_slowdown = 5
options.brightness = 60
options.hardware_mapping = 'adafruit-hat'  # If you have an Adafruit HAT: 'adafruit-hat'

matrix = RGBMatrix(options=options)
offscreen_canvas = matrix.CreateFrameCanvas()

#RGBMatrix(options = options)
pos = offscreen_canvas.width
"""
while True:
    offscreen_canvas.Clear()
    font = graphics.Font()
    font.CharacterWidth(20)
    font.LoadFont("submodules/rgbmatrix/fonts/tom-thumb.bdf")
    red = graphics.Color(74,3,54)
    #print(datetime.now().strftime('%Y-%m-%dT%H:%M:%SZ'))
    #stuff = graphics.DrawText(offscreen_canvas, font, 2, 10, red, datetime.now().strftime('%H:%M:%S'))
    #pos -= 1
    #if (pos + stuff < 0):
    #    pos = offscreen_canvas.width
    #graphics.DrawText(matrix, font, num, 16, red, datetime.now().strftime('%Y-%m-%dT%H:%M:%SZ'))
    #time.sleep(.01)
    #while True:
    graphics.DrawText(offscreen_canvas, font, 14, 12, red, f"{datetime.now().strftime('%m/%d/%Y')}")
    font = graphics.Font()
    font.CharacterWidth(10)
    font.LoadFont("submodules/rgbmatrix/fonts/5x8.bdf")
    graphics.DrawText(offscreen_canvas, font, 13, 20, red, f"{datetime.now().strftime('%I:%M:%S')}")
    offscreen_canvas = matrix.SwapOnVSync(offscreen_canvas)
    # matrix.SetImage(image.convert('RGB'))
#graphics.DrawLine(matrix, 5, 7, 22, 13, red)
#graphics.DrawCircle(matrix, 15, 15, 10, red)
"""
# Make image fit our screen.
image.thumbnail((matrix.width, matrix.height), Image.ANTIALIAS)

matrix.SetImage(image.convert('RGB'))
try:
    print("Press CTRL-C to stop.")
    while True:
        time.sleep(100)
except KeyboardInterrupt:
    sys.exit(0)