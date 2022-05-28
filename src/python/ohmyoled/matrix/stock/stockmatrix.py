#!/usr/bin/env python3 

import time
import os
from typing import Dict, Tuple
from PIL import ImageFont, Image, ImageDraw
from ohmyoled.matrix.matrix import Matrix, FailedApiException
from ohmyoled.lib.stock.stocks import Stock, StockErrorResult
from ohmyoled.matrix.error import ErrorMatrix
MASTER_IMAGE = {
    "size": (64, 32),
    "master_top_image": {
        "size": (50,8),
        "location": (0,0),
        "rec": (0,0,49,7),
        "sub_images": {
            "symbol": {
                "size": (12, 6),
                "location": (1,1),
                "rec": (0,0,11,5),
                "text_location": (1,0),
            },
            "current_price": {
                "size": (32, 6),
                "location": (15, 1),
                "rec": (0,0, 31,5),
                "text_location": (0,0),
            }
        }
    },
    "master_middle_image": {
        "size": (50, 24),
        "location": (0, 9),
        "rec": (0, 0, 49, 22),
        "sub_images": {
            "cp_letter": {
                "size": (10, 7),
                "location": (2,2),
                "rec": (0,0,9,6),
            },
            "highest_letter": {
                "size": (10, 7),
                "location": (2,8),
                "rec": (0,0,9,6),
            },
            "lowest_letter": {
                "size": (10, 7),
                "location": (2,14),
                "rec": (0,0,9,6),
            },
            "closing_price": {
                "size": (30, 7),
                "location": (15, 2),
                "rec": (0,0,29,6),
            },
            "highest_price": {
                "size": (30, 7),
                "location": (15, 8),
                "rec": (0,0,29,6)
            },
            "lowest_price": {
                "size": (30, 7),
                "location": (15, 14),
                "rec": (0,0,29,6)
            }
        }
    },
    "right_middle_image": {
        "size": (14, 32),
        "location": (50, 0),
        "rec": (0, 0, 13, 31),
        "sub_images": {
            "percentage_letter": {
                "size": (10, 6),
                "location": (2, 2),
                "rec": (0,0,9, 5)
            },
            "percentage_image": {
                "size": (14, 6),
                "location": (1, 9),
                "rec": (0,0,13, 5)
            },
            "plus_minus_letter": {
                "size": (14, 6),
                "location": (2, 16),
                "rec": (0,0,13, 5)
            },
            "plus_minus_price": {
                "size": (14, 6),
                "location": (1, 22),
                "rec": (0,0,13, 5)
            }
        }
    }
}
class StockMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
    def __str__(self) -> str:
        return "StockMatrix"

    async def poll_api(self) -> Stock:
        try:
            polled_api = await self.api.run()
            if isinstance(polled_api, StockErrorResult):
                return None
            return Stock(polled_api)
        except Exception as e:
            return None
    
    def nonasync_poll(self):
        return Stock(self.api.run_with_asyncio())
    
    def check_size(self, draw, text: str, size: Tuple[int]):
        if draw.textsize(text)[0] > size[0]:
            return True
    
    def render_symbol(self, api, size, location, draw, xpos) -> None:
        font = ImageFont.truetype("fonts/04B_03B_.TTF", 8)
        if self.check_size(draw, api.get_symbol.upper(), size):
            draw.text(
                (-xpos + location[0], location[1]), 
                f"{api.get_symbol.upper()}", 
                font=font, 
                fill=(0, 89, 255)
            )   
        else:
            draw.text(
                location, 
                f"{api.get_symbol.upper()}", 
                font=font, 
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
    
    def render_percentage_change(self, api, size, location, draw, xpos):
        dollar_change, percent_change = self.calculate_percent_diff(
            api.get_quote_previous_close, 
            api.get_quote_current_price
        )
        if api.get_quote_previous_close == api.get_quote_current_price:
            fill = (255,255,255)
        elif api.get_quote_previous_close > api.get_quote_current_price:
            fill=(255,0,0)
        else:
            fill=(0,255,0)
        if self.check_size(draw, percent_change, size):
            draw.text(
                (-xpos,0),
                percent_change, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=fill
            )
        else:
            draw.text(
                (0,0),
                percent_change, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=fill
            )
    def render_dollar_change(self, api, size, location, draw, xpos):
        dollar_change, percent_change = self.calculate_percent_diff(
            api.get_quote_previous_close, 
            api.get_quote_current_price
        )
        if api.get_quote_previous_close == api.get_quote_current_price:
            fill = (255,255,255)
        elif api.get_quote_previous_close > api.get_quote_current_price:
            fill=(255,0,0)
        else:
            fill=(0,255,0)
        if self.check_size(draw, dollar_change, size):
            draw.text(
                (-xpos,0),
               dollar_change, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=fill
            )
        else:
            draw.text(
                (0,0),
               dollar_change, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=fill
            )
    def render_current_price(self, api, size, location, draw, xpos) -> None:
        if api.get_quote_previous_close == api.get_quote_current_price:
            fill = (255,255,255)
        elif api.get_quote_previous_close > api.get_quote_current_price:
            fill=(255,0,0)
        else:
            fill=(0,255,0)
        text = f"${self.format_price(api.get_quote_current_price)}"
        if self.check_size(draw, text, size):
            draw.text(
                (-xpos, 0),
                text, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=fill
            )
        else:
            draw.text(
                (0, 0),
                text, 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=fill
            )
    def render_previous_close(self, api, size, location, draw, xpos):
        if self.check_size(draw, f"${self.format_price(api.get_quote_previous_close)}", size):
            draw.text(
                (-xpos,0),
                f"${self.format_price(api.get_quote_previous_close)}", 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(255,255,255)
            )
        else:
            draw.text(
                (0,0),
                f"${self.format_price(api.get_quote_previous_close)}", 
                font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                fill=(255,255,255)
            )
    def render_highest_price(self, api, size, location, draw, xpos) -> None:
        text = f"${self.format_price(api.get_quote_highest_price)}"
        font = ImageFont.truetype('fonts/04B_03B_.TTF', 8)
        if self.check_size(draw, text, size):
            draw.text(
                (-xpos,0), 
                f"${self.format_price(api.get_quote_highest_price)}", 
                font=font, 
                fill=(0,255,0)
            )
        else:
            draw.text(
                (0,0), 
                f"${self.format_price(api.get_quote_highest_price)}", 
                font=font, 
                fill=(0,255,0)
            )
    def render_lowest_price(self, api, size, location, draw, xpos) -> None:
        text = f"${self.format_price(api.get_quote_lowest_price)}"
        font = ImageFont.truetype('fonts/04B_03B_.TTF', 8)
        if self.check_size(draw, text, size):
            draw.text(
                (-xpos,0), 
                text, 
                font=font, 
                fill=(255,0,0)
            )
        else:
            draw.text(
                (0,0), 
                text, 
                font=font, 
                fill=(255,0,0)
            )
    def build_top_image(self, api, xpos):
        top_config = MASTER_IMAGE["master_top_image"]
        master_top_image = self.make_new_image(top_config.get("size"))
        master_top_image_draw = ImageDraw.Draw(master_top_image)
        if os.getenv("DEV"):
            master_top_image_draw.rectangle(top_config["rec"])
        for sub_image, config in top_config["sub_images"].items():
            image = self.make_new_image(config["size"])
            sub_image_draw = ImageDraw.Draw(image)
            if os.getenv("DEV"):
                sub_image_draw.rectangle(config["rec"])
            if sub_image == "symbol":
                self.render_symbol(api, config["size"], config['text_location'], sub_image_draw, xpos)
            if sub_image == "current_price":
                self.render_current_price(api, config["size"], config['text_location'], sub_image_draw, xpos)
            master_top_image.paste(image, config['location'])
        return master_top_image, top_config["location"]

    def build_middle_image(self, api, xpos):
        middle_config = MASTER_IMAGE["master_middle_image"]
        master_middle_image = self.make_new_image(middle_config["size"])
        master_middle_image_draw = ImageDraw.Draw(master_middle_image)
        if os.getenv("DEV"):
            master_middle_image_draw.rectangle(middle_config["rec"])
        for sub_image, config in middle_config["sub_images"].items():
            image = self.make_new_image(config["size"])
            sub_image_draw = ImageDraw.Draw(image)
            if os.getenv("DEV"):
                sub_image_draw.rectangle(config["rec"])
            if sub_image == "cp_letter":
                font = ImageFont.truetype("fonts/04B_03B_.TTF", 8)
                sub_image_draw.text(xy=(0,0), text="CP", fill=(255,255,255), font=font)
            if sub_image == "highest_letter":
                sub_image_draw.text(
                    (0, -4), 
                    "\uf058", 
                    font=ImageFont.truetype("fonts/weathericons.ttf", 11), 
                    fill=(0, 255, 0)
                )
            if sub_image == "lowest_letter":
                sub_image_draw.text(
                    (0, -4), 
                    "\uf044", 
                    font=ImageFont.truetype("fonts/weathericons.ttf", 11), 
                    fill=(255, 0, 0)
                )
            if sub_image == "closing_price":
                self.render_previous_close(api, config["size"], config["location"], sub_image_draw, xpos)    
            if sub_image == "highest_price":
                self.render_highest_price(api, config["size"], config["location"], sub_image_draw, xpos)
            if sub_image == "lowest_price":
                self.render_lowest_price(api, config["size"], config["location"], sub_image_draw, xpos)
            master_middle_image.paste(image, config['location']) 
        return master_middle_image, middle_config["location"]
    
    def build_right_image(self, api, xpos):
        right_config = MASTER_IMAGE["right_middle_image"]
        master_right_image = self.make_new_image(right_config["size"])
        master_right_image_draw = ImageDraw.Draw(master_right_image)
        if os.getenv("DEV"):
            master_right_image_draw.rectangle(right_config["rec"])
        for sub_image, config in right_config["sub_images"].items():
            image = self.make_new_image(config["size"])
            sub_image_draw = ImageDraw.Draw(image)
            if os.getenv("DEV"):
                sub_image_draw.rectangle(config["rec"])
            if sub_image == "percentage_letter":
                sub_image_draw.text(
                    (2, 0),
                    "%", 
                    font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                    fill=(0,255,0)
                )
            if sub_image == "percentage_image":
                self.render_percentage_change(api, config["size"], config["location"], sub_image_draw, xpos)
            if sub_image == "plus_minus_letter":
                sub_image_draw.text(
                    (0,0),
                    "+/-", 
                    font=ImageFont.truetype("fonts/04B_03B_.TTF", 8), 
                    fill=(0,255,0)
                )
            if sub_image == "plus_minus_price":
                self.render_dollar_change(api, config["size"], config["location"], sub_image_draw, xpos)
            master_right_image.paste(image, config['location'])
        return master_right_image, right_config["location"]
    
    async def render(self, api, loop):
        try:
            self.clear()
            self.reload_image()
            if not api:
                raise FailedApiException("Error while getting API")
            xpos = 0
            while xpos < 50:
                images = [
                    self.build_top_image(api, xpos),
                    self.build_middle_image(api, xpos),
                    self.build_right_image(api, xpos)
                ]
                for image in images:
                    self.paste_image(image[0], image[1])
                await self.render_image()
                xpos += 1
                time.sleep(3) if xpos == 1 else time.sleep(.05)
            
            images = [
                self.build_top_image(api, 0),
                self.build_middle_image(api, 0),
                self.build_right_image(api, 0)
            ]
            for image in images:
                self.paste_image(image[0], image[1])
            await self.render_image()
            time.sleep(30)
        except Exception as e:
            self.logger.exception(e)
            error_matrix = ErrorMatrix(self.matrix, self.logger, "Stock Matrix")
            await error_matrix.render()

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