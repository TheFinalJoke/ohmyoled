#!/usr/bin/env python3 

import asyncio
import time
from typing import Dict

import os
from PIL import Image
from PIL import ImageDraw
from PIL import ImageFont
from matrix.matrix import Matrix, MatrixBase, FontException, Canvas
from lib.stock.stocks import Stock

class StockMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
    
    async def poll_api(self):
        return Stock(await self.api.run())

    def render_symbol(self, api):
        font = ImageFont.truetype("fonts/retro_computer.ttf", 7)
        self.draw_text((0, 0), f"{api.get_symbol}", font)
    def render_current_price(self, api):
        if api.get_quote_previous_close == api.get_quote_current_price:
            self.draw_text((13, 7), f"${str(api.get_quote_current_price)}", font=ImageFont.truetype("fonts/retro_computer.ttf", 7))
        elif api.get_quote_previous_close > api.get_quote_current_price:
            self.draw_text((13, 7), f"${str(api.get_quote_current_price)}", font=ImageFont.truetype("fonts/retro_computer.ttf", 7), fill=(255,0,0))
        else:
            self.draw_text((13, 7), f"${str(api.get_quote_current_price)}", font=ImageFont.truetype("fonts/retro_computer.ttf", 7), fill=(0,255,0))

        font = ImageFont.truetype("fonts/retro_computer.ttf", 7)
        self.draw_text((0, 7), "P:", font=font)
    def render_highest_price(self, api):
        font = ImageFont.truetype('fonts/retro_computer.ttf', 7)
        self.draw_text((1, 11), "\uf058", font=ImageFont.truetype("fonts/weathericons.ttf", 11), fill=(0, 255, 0))
        self.draw_text((13, 14), f"${api.get_quote_highest_price}", font=font, fill=(0,255,0))
    def render_lowest_price(self, api):
        self.draw_text((1, 19), "\uf044", font=ImageFont.truetype("fonts/weathericons.ttf", 11), fill=(255, 0, 0))
        self.draw_text((13, 22), f"${api.get_quote_lowest_price}", font=ImageFont.truetype('fonts/retro_computer.ttf', 7), fill=(255,0,0))
    def render(self, api):
        self.logger.info("Started Render for Stock Matrix")
        self.clear()
        self.reload_image()
        self.render_symbol(api)
        self.render_current_price(api)
        self.render_highest_price(api)
        self.render_lowest_price(api)
        self.render_image()
        time.sleep(10)