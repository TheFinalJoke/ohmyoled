import PIL.Image as image_types
import numpy as np
import sys
import os
import time


class TerminalMatrix:
    def __init__(self, options) -> None:
        self.options = options

    def SetImage(self, image: image_types.Image, offset_x: int, offset_y: int) -> None:
        h = image.height
        w = image.width

        # Set to array
        img_arr = np.asarray(image)
        # Get the shape so we know x,y coords
        h, w, _ = img_arr.shape
        # Then draw our mona lisa
        img_data = ""
        for x in range(h):
            for y in range(w):
                pix = img_arr[x][y]
                color = " "
                # 90% of our image is black, and the pi sometimes has trouble writing to the terminal
                # quickly. So default the color to blank, and only fill in the color if it's not black
                if sum(pix) > 0:
                    color = self.__get_color(pix[0], pix[1], pix[2])
                if y == w - 1:
                    img_data += color + "\n"
                else:
                    img_data += color

        sys.stdout.write(img_data)
        self.Clear()
        sys.stdout.write(img_data)

    def __get_color(self, r, g, b):
        return f"\x1b[38;2;{r};{g};{b}m\u2022\x1b[0m"

    def Clear(self) -> None:
        clear = lambda: os.system("cls" if os.name in ("nt", "dos") else "clear")
        clear()
