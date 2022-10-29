import sys
from PIL import Image, ImageDraw, ImageFont
import numpy as np
def get_ansi_color_code(r, g, b):
    if r == g and g == b:
        if r < 8:
            return 16
        if r > 248:
            return 231
        return round(((r - 8) / 247) * 24) + 232
    return 16 + (36 * round(r / 255 * 5)) + (6 * round(g / 255 * 5)) + round(b / 255 * 5)

def get_color(r, g, b):
    return "\x1b[48;{}m\u2022\x1b[13m".format(int(get_ansi_color_code(r,g,b)))

def show_image(img):
    h = img.height
    w = img.width

    # Get image
    img = img.resize((w,h), Image.LANCZOS)
    # Set to array
    img_arr = np.asarray(img)
    # Get the shape so we know x,y coords
    h,w,c = img_arr.shape
    # Then draw our mona lisa
    mona_lisa = ''
    for x in range(h):
        for y in range(w):
            pix = img_arr[x][y]
            color = ' '
            # 90% of our image is black, and the pi sometimes has trouble writing to the terminal
            # quickly. So default the color to blank, and only fill in the color if it's not black
            if sum(pix) > 0:
                color = get_color(pix[0], pix[1], pix[2])
            if y == 63:
                mona_lisa += color + "\n"
            else:
                mona_lisa += color
    sys.stdout.write(mona_lisa)

image = Image.new("RGB", (64, 32))

draw = ImageDraw.Draw(image)
font = ImageFont.truetype("/usr/share/fonts/truetype/noto/NotoMono-Regular.ttf", 14)
draw.text((2,1), 'cool', fill=(255,0,0))
show_image(image)
