#!/usr/bin/env python3 

import asyncio
import time
from typing import Dict
from PIL import ImageFont
from matrix.matrix import Matrix
from lib.stock.stocks import Stock

class StockMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
    
    async def poll_api(self) -> Stock:
        return Stock(await self.api.run())

    def render_symbol(self, api) -> None:
        font = ImageFont.truetype("fonts/04B_03B_.TTF", 8)
        self.draw_text(
            (0, 0), 
            f"{api.get_symbol.upper()}", 
            font, 
            fill=(0, 89, 255)
        )
    
    def format_price(self, price: int) -> str:
        return '{:<06}'.format(str(round(price, 2)))

    def render_current_price(self, api) -> None:
        if api.get_quote_previous_close == api.get_quote_current_price:
            self.draw_text(
                (13, 8), 
                f"${self.format_price(api.get_quote_current_price)}", 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8)
            )
        elif api.get_quote_previous_close > api.get_quote_current_price:
            self.draw_text(
                (13, 8), 
                f"${self.format_price(api.get_quote_current_price)}", 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(255,0,0)
            )
        else:
            self.draw_text(
                (13, 8), 
                f"${self.format_price(api.get_quote_current_price)}", 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(0,255,0)
            )

        font = ImageFont.truetype("fonts/04B_03B_.TTF", 8)
        self.draw_text((1, 8), "P:", font=font)

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
            f"${self.format_price(api.get_quote_current_price)}", 
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
            f"${self.format_price(api.get_quote_current_price)}", 
            font=ImageFont.truetype('fonts/04B_03B_.TTF', 8), 
            fill=(255,0,0)
        )
    
    def render(self, api) -> None:
        self.logger.info("Started Render for Stock Matrix")
        self.clear()
        self.reload_image()
        self.render_symbol(api)
        self.render_current_price(api)
        self.render_highest_price(api)
        self.render_lowest_price(api)
        self.render_image()
        time.sleep(30)