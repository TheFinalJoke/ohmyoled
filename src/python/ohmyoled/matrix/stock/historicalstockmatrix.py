#!/usr/bin/env python3

import asyncio
import time
from typing import Dict

import os
from PIL import Image
from PIL import ImageDraw
from PIL import ImageFont
from ohmyoled.matrix.matrix import Matrix, MatrixBase, FontException
from ohmyoled.lib.stock.stocks import Stock

class HistoricalStockMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
    async def poll_api(self):
        return Stock(await self.api.run())
    
    def render_symbol(self, api):
        font = ImageFont.truetype("fonts/retro_computer.ttf", 7)
        self.draw_text((0, 0), f"{api.get_symbol}", font)
    
    def render(self, api):
        self.logger.info("Started Render for Historical Stock Matrix")
        self.clear()
        self.reload_image()
        self.render_symbol(api)
        self.render_image()
        time.sleep(10)