import sys
from PIL import Image, ImageDraw, ImageFont
import numpy as np

def get_color(r, g, b):
    return f"\x1b[38;2;{r};{g};{b}m\u2022\x1b[0m"

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
font = ImageFont.truetype("/usr/share/fonts/truetype/noto/NotoMono-Regular.ttf", 12)
draw.rectangle((0,0,63,31))
draw.text((2,1), 'cool', fill=(0,255,255), font=font)
show_image(image)
