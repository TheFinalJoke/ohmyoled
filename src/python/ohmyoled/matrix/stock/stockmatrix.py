#!/usr/bin/env python3 

import asyncio
from doctest import master
import time
from typing import Dict, Tuple
from PIL import ImageFont, Image, ImageDraw
from ohmyoled.matrix.matrix import Matrix
from ohmyoled.lib.stock.stocks import Stock

class StockMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
    def __str__(self) -> str:
        return "StockMatrix"

    async def poll_api(self) -> Stock:
        return Stock(await self.api.run())
    
    def nonasync_poll(self):
        return Stock(self.api.run_with_asyncio())

    def render_symbol(self, api) -> None:
        font = ImageFont.truetype("fonts/04B_03B_.TTF", 8)
        self.draw_text(
            (0, 1), 
            f"{api.get_symbol.upper()}", 
            font, 
            fill=(0, 89, 255)
        )
    
    def calculate_percent_diff(self, previous, current) -> Tuple[str, str]:
        diff = round(current - previous, 2)
        percent_base = round((diff / previous) * 100, 2)
        percent_change = lambda x: '{:<04}'.format(str(round(x, 2)))
        diff_format = lambda x: '{:<05}'.format(str(round(x, 2))) if x < 0 else '{:<04}'.format(str(round(x, 2)))
        return diff_format(diff), percent_change(percent_base)
    
    def format_price(self, price: float) -> str:
        return '{:.2f}'.format(price)
    
    def render_current_price(self, api, xpos) -> None:
        dollar_change, percent_change = self.calculate_percent_diff(
            api.get_quote_previous_close, 
            api.get_quote_current_price
        )
        if api.get_quote_previous_close == api.get_quote_current_price:
            self.draw_text(
                (-xpos + 12, 0),
                f"${self.format_price(api.get_quote_current_price)}", 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(255,255,255)
            )
            self.draw_text(
                (44, 6) if "-" in percent_change else (47, 6),
                percent_change, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(255,255,255)
            )
            self.draw_text(
                (44, 20) if "-" in dollar_change else (47, 20),
                f"{dollar_change}",
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8),
                fill=(255,255,255)
            )
        elif api.get_quote_previous_close > api.get_quote_current_price:
            self.draw_text(
                (-xpos + 12, 0),
                f"${self.format_price(api.get_quote_current_price)}", 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(255,0,0)
            )
            self.draw_text(
                (44, 6) if "-" in percent_change else (47, 6),
                percent_change, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(255,0,0)
            )
            self.draw_text(
                (44, 20) if "-" in dollar_change else (47, 20),
                f"{dollar_change}",
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8),
                fill=(255,0,0)
            )
        else:
            self.draw_text(
                (-xpos + 12, 0),
                f"${self.format_price(api.get_quote_current_price)}", 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(0,255,0)
            )
            self.draw_text(
                (44, 6) if "-" in percent_change else (47, 6),
                percent_change, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(0,255,0)
            )
            self.draw_text(
                (44, 20) if "-" in dollar_change else (47, 20),
                f"{dollar_change}",
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8),
                fill=(0,255,0)
            )
        self.draw_text(
            (52, 0),
            "%", 
            font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
            fill=(0,255,0)
        )
        self.draw_text(
            (48, 14),
            "+/-", 
            font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
            fill=(0,255,0)
        )
    def render_previous_close(self, api):
        font = ImageFont.truetype("fonts/04B_03B_.TTF", 8)
        self.draw_text((1, 8), "CP", font=font)
        self.draw_text(
            (13, 8),
            f"${self.format_price(api.get_quote_previous_close)}", 
            font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
            fill=(255,255,255)
        )
    def render_highest_price(self, api) -> None:
        font = ImageFont.truetype('fonts/04B_03B_.TTF', 8)
        self.draw_text(
            (1, 11), 
            "\uf058", 
            font=ImageFont.truetype("fonts/weathericons.ttf", 11), 
            fill=(0, 255, 0)
        )
        self.draw_text(
            (13, 16), 
            f"${self.format_price(api.get_quote_highest_price)}", 
            font=font, 
            fill=(0,255,0)
        )
    
    def render_lowest_price(self, api) -> None:
        self.draw_text(
            (1, 19), 
            "\uf044", 
            font=ImageFont.truetype("fonts/weathericons.ttf", 11), 
            fill=(255, 0, 0)
        )
        
        self.draw_text(
            (13, 24), 
            f"${self.format_price(api.get_quote_lowest_price)}", 
            font=ImageFont.truetype('fonts/04B_03B_.TTF', 8), 
            fill=(255,0,0)
        )
    def build_top_image(self, api):
        master_top_image = self.make_new_image((50,8))
        master_top_image_draw = ImageDraw.Draw(master_top_image)
        master_top_image_draw.rectangle((0, 0, 49, 7))
        symbol_image = self.make_new_image((12, 6))
        symboldraw = ImageDraw.Draw(symbol_image)
        symboldraw.rectangle((0,0,11,5))
        cp_image = self.make_new_image((32, 6))
        cpdraw = ImageDraw.Draw(cp_image)
        cpdraw.rectangle((0,0, 31,5)) # How big is th rectangle
        master_top_image.paste(cp_image, (15, 1))
        master_top_image.paste(symbol_image, (1,1))
        return master_top_image, (0,0)

    def build_middle_image(self, api):
        master_middle_image = self.make_new_image((50, 24))
        master_middle_image_draw = ImageDraw.Draw(master_middle_image)
        master_middle_image_draw.rectangle((0, 0, 49, 22))
        symbol_image = self.make_new_image((15, 15))
        symboldraw = ImageDraw.Draw(symbol_image)
        symboldraw.rectangle((0,0,7,5))
        # cp_image = self.make_new_image((32, 6))
        # cpdraw = ImageDraw.Draw(cp_image)
        # cpdraw.rectangle((0,0, 31,5)) # How big is th rectangle
        master_middle_image.paste(symbol_image, (0,0))
        return master_middle_image, (0, 9)
    def build_right_image(self, api):
        master_right_image = self.make_new_image((14, 32))
        return master_right_image, (51, 0)
    
    async def render(self, api, loop):
        self.clear()
        self.reload_image()
        images = [
            self.build_top_image(api),
            self.build_middle_image(api),
            #self.build_right_image(api)
        ]
        for image in images:
            self.paste_image(image[0], image[1])
        await self.render_image()
        time.sleep(5)
    # async def render(self, api, loop) -> None:
    #     self.logger.info("Started Render for Stock Matrix")
    #     xpos = 0
    #     self.clear()
    #     while xpos < 50:
    #         self.reload_image()
    #         self.render_symbol(api)
    #         self.render_current_price(api,xpos)
    #         self.render_previous_close(api)
    #         self.render_highest_price(api)
    #         self.render_lowest_price(api)
    #         await self.render_image()
    #         xpos += 1
    #         time.sleep(3) if xpos == 1 else time.sleep(.05)
    #     self.reload_image()
    #     self.render_symbol(api)
    #     self.render_current_price(api, xpos)
    #     self.render_previous_close(api)
    #     self.render_highest_price(api)
    #     self.render_lowest_price(api)
    #     await self.render_image()
    #     time.sleep(30)

    def non_async_render(self, api) -> None:
        self.logger.info("Started Render for Stock Matrix")
        self.clear()
        self.reload_image()
        self.render_symbol(api)
        self.render_current_price(api)
        self.render_previous_close(api)
        self.render_highest_price(api)
        self.render_lowest_price(api)
        self.nonasync_render_image()
        time.sleep(30)