#!/usr/bin/env python3
"""
import time
import sys
import datetime
import urllib.request as req
from rgbmatrix import RGBMatrix, RGBMatrixOptions, graphics, FrameCanvas
from PIL import ImageDraw, Image, ImageFont
bottom_options = RGBMatrixOptions()
bottom_options.rows = 32
bottom_options.cols = 64
bottom_options.chain_length = 1
bottom_options.parallel = 1
bottom_options.gpio_slowdown = 5
bottom_options.brightness = 60
bottom_options.hardware_mapping = 'adafruit-hat'  # If you have an Adafruit HAT: 'adafruit-hat'
scrolling_matrix = RGBMatrix(options=bottom_options)
master_image = Image.new(
    "RGB",
    (64,32)
)
img1 = Image.new(
    "RGB",
    (64, 8)
)
img2 = Image.new(
    "RGB",
    (16, 16)
)
img3 = Image.new(
    "RGB",
    (16, 16)
)
img4 = Image.new(
    "RGB",
    (22, 9)
)
img5 = Image.new(
    "RGB",
    (22, 9)
)
img6 = Image.new(
    "RGB",
    (22, 9)
)
img7 = Image.new(
    "RGB",
    (34,16)
)
im1_draw = ImageDraw.Draw(img1)
im1_draw.rectangle([(0,0), (63,6)])
im2_draw = ImageDraw.Draw(img2)
im2_draw.rectangle([(0,0), (15,15)])
im3_draw = ImageDraw.Draw(img3)
im3_draw.rectangle([(0,0), (15,15)])
im4_draw = ImageDraw.Draw(img4)
im4_draw.rectangle([(0,0), (21,8)])
im5_draw = ImageDraw.Draw(img5)
im5_draw.rectangle([(0,0), (21,8)])
im6_draw = ImageDraw.Draw(img6)
im6_draw.rectangle([(0,0), (21,8)])
im7_draw = ImageDraw.Draw(img7)
im7_draw.rectangle([(0,0), (33,15)])
master_image.paste(img1, (0, 25))
master_image.paste(img2, (-2, 9))
master_image.paste(img3, (50, 9))
master_image.paste(img4, (0,0))
master_image.paste(img5, (44,0))
master_image.paste(img6, (22, 0))
master_image.paste(img7, (15, 9))
scrolling_matrix.SetImage(master_image)
# Image within an image 
"""
"""
scrolling_font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
scrolling_matrix = RGBMatrix(options=bottom_options)
img1_text = Image.new("RGB", (22, 5))
img2_text = Image.new("RGB", (22, 5))
logo1 = req.urlretrieve("https://media.api-sports.io/baseball/teams/33.png", "/tmp/33.png")
logo2 = req.urlretrieve("https://media.api-sports.io/baseball/teams/32.png", "/tmp/32.png")
image1 = Image.open('/tmp/33.png')
image2 = Image.open('/tmp/32.png')
main_image = Image.new('RGB', (64, 24))
image1.thumbnail((17,17))
img1_draw = ImageDraw.Draw(img1_text)

img1_draw.text((0,0), "CAR", font=scrolling_font)
image2.thumbnail((17,17))
main_image.paste(img1_text, (0,0))
main_image.paste(image1, (-2, 6), 255)
main_image.paste(image2, (50, 6), 255)
#image1.thumbnail((7, 10), Image.ANTIALIAS)
scrolling_matrix.SetImage(main_image.convert('RGB'))
time.sleep(30)
"""
"""
time_stamps = [datetime.datetime(2021, 8, 30, 19, 0), datetime.datetime(2021, 8, 31, 19, 0), datetime.datetime(2021, 9, 1, 19, 0), datetime.datetime(2021, 9, 2, 19, 0), datetime.datetime(2021, 9, 3, 19, 0), datetime.datetime(2021, 9, 4, 19, 0), datetime.datetime(2021, 9, 5, 19, 0), datetime.datetime(2021, 9, 6, 19, 0)]
close_prices = [122.8155, 123.0363342, 123.1214334105, 123.34281770849219, 123.42812890112556, 123.18127264332331, 122.93491009803665, 122.68904027784059]
fake = [4,2,1,4,5,6,5]
for t, c in zip(range(len(close_prices)), close_prices):
    print(t,round(c, 2))
stuff = [(t,round(c, 2)) for t,c in zip(range(len(fake)), fake)]
image = Image.new("RGB", (64, 32))
draw = ImageDraw.Draw(image)
draw.point(stuff)
scrolling_matrix.SetImage(image)
time.sleep(30)
"""
"""
if len(sys.argv) < 2:
    sys.exit("Require an image argument")
else:
    image_file = sys.argv[1]

#image = Image.new("RGB", (64, 32))
image = Image.open(image_file)
draw = ImageDraw.Draw(image)
font = ImageFont.truetype("/usr/share/fonts/truetype/noto/NotoMono-Regular.ttf", 10)
draw.text((1, 5), "hello", font=font)
#draw.rectangle((0, 0, 63, 31), fill=(0, 0, 0), outline=(0, 0, 255))
# Configuration for the matrix

# Can not Make 2 matrixes and/or 2 images
bottom_options = RGBMatrixOptions()
bottom_options.rows = 32
bottom_options.cols = 64
bottom_options.chain_length = 1
bottom_options.parallel = 1
bottom_options.gpio_slowdown = 5
bottom_options.brightness = 60
bottom_options.hardware_mapping = 'adafruit-hat'  # If you have an Adafruit HAT: 'adafruit-hat'

scrolling_matrix = RGBMatrix(options=bottom_options)
ypos = 0
text = 'American League\n1: Tampa Bay Rays\n2: Houston Astros\n3: New York Yankees\n4: Chicago White Sox\n5: Boston Red Sox\n6: Oakland Athletics\n7: Seattle Mariners\n8: Toronto Blue Jays\n9: Cleveland Indians\n10: Los Angeles Angels\n11: Detroit Tigers\n12: Kansas City Royals\n13: Minnesota Twins\n14: Texas Rangers\n15: Baltimore Orioles\nNational League\n1: San Francisco Giants\n2: Los Angeles Dodgers\n3: Milwaukee Brewers\n4: Cincinnati Reds\n5: Atlanta Braves\n6: San Diego Padres\n7: St.Louis Cardinals\n8: Philadelphia Phillies\n9: New York Mets\n10: Colorado Rockies\n11: Chicago Cubs\n12: Washington Nationals\n13: Miami Marlins\n14: Pittsburgh Pirates\n15: Arizona Diamondbacks'
scrolling_font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
image = Image.new("RGB", (64, 32))
draw = ImageDraw.Draw(image)

width, height = draw.multiline_textsize(text)
print(width, height)
test_image = Image.new("RGB", (64,32))
test_draw = ImageDraw.Draw(test_image)
# test_draw.text((32,0), "hello", font=scrolling_font)

ycounter = 0
while True:
    ypos += 1 
    ycounter += 1
    if ycounter == 1 or ycounter % 8 == 0:
        xpos = 0
        pause = True
        while True:
            if xpos > width:
                break
            else:
                if xpos > 100:
                    break
                image = Image.new("RGB", (64, 32))
                draw = ImageDraw.Draw(image)
                # FOUND IT YAASS
                # - Y position - yposition to offset the image
                image.paste(test_image)
                draw.multiline_text((-xpos-xpos, -ypos-ypos), text, font=scrolling_font, fill=(255,255,255), spacing=1) if ycounter > 1 else draw.multiline_text((-xpos-xpos, 0), text, font=scrolling_font, fill=(255,255,255), spacing=1)
                scrolling_matrix.SetImage(image)
                time.sleep(3) if pause else time.sleep(0)
                xpos += 1
                pause = False
    print(ypos)
    if ypos > 100:
        ypos = 0
    image = Image.new("RGB", (64, 32))
    image = image.resize((64, height), Image.ANTIALIAS)
    draw = ImageDraw.Draw(image)

    draw.multiline_text((0, -ypos), text, font=scrolling_font, fill=(255,255,255), spacing=1)
    scrolling_matrix.SetImage(image, offset_y=-ypos)

    time.sleep(.05)
"""
#time.sleep(30)

import asyncio
from setuptools import find_packages, setup
breakpoint()
#@make_async
def test1(arg1, arg2):
    print(arg1, arg2)

def test2(arg1, arg2):
    print(arg1, arg2)


asyncio.run(test1("hello", "world"))