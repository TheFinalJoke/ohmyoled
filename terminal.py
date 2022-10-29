# This is a basic program to display the a 64x32 pixels to the terminal
# 1. You will have to be in a Dev Container (ping @nickshorter if you can't pull down the images)
#   - You will have to maybe look at the documentation for Dev Containers On Microsoft website
#   - You can use any computer, x86_64, or ARM
# 2. Need to checkout the Git Branch "term_problem"
# 3. Problem:
#       1. Print out the current time of the system and display 
#       2. Print out CPU usage and display 
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
    data = ''
    # Range of 0 - 32 pixels
    for x in range(h):
        # Range of 0 - 64 pixels 
        for y in range(w):
            pix = img_arr[x][y]
            color = ' '
            # 90% of our image is black, and the pi sometimes has trouble writing to the terminal
            # quickly. So default the color to blank, and only fill in the color if it's not black
            if sum(pix) > 0:
                color = get_color(pix[0], pix[1], pix[2])
            # Check if at the end of the line
            # Make a new line
            if y == 63:
               data += color + "\n"
            else:
               data += color
    # Print directly to Stdout
    sys.stdout.write(data)


