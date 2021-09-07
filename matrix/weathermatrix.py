#!/usr/bin/env python3 

import asyncio
import time
from typing import Dict

import os
from PIL import Image
from PIL import ImageDraw
from PIL import ImageFont
from matrix.matrix import Matrix, MatrixBase, FontException
from lib.weather.weather import (
    Weather,
    build_weather_icons
)

class WeatherMatrix(Matrix):
    def __init__(self, matrix, api: Dict, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
        self.icons = build_weather_icons()

    async def poll_api(self) -> Weather:
        return Weather(await self.api.run())
    def get_temp_color(self, temp):
        if temp >= 100:
            return (255, 12, 3)
        elif temp in range(70, 99):
            return (247, 157, 3)
        elif temp in range(40, 69):
            return (5, 223, 3)
        elif temp in range(20,39):
            return (0, 255, 255)
        else:
            return (0, 76, 255)
    def render_temp(self, api):
        font = ImageFont.truetype("/usr/share/fonts/retro_computer.ttf", 7)
        metric = "\uf045"
        self.draw_text((0, 10), "T:", font=font)
        self.draw_text((10, 10), f"{str(int(api.get_temp))}F", font=font, fill=self.get_temp_color(int(api.get_temp)))
        self.draw_text((30, 10), "R:", font=font)
        self.draw_text((40, 10), f"{str(int(api.get_feels_like))}F", font=font, fill=self.get_temp_color(int(api.get_temp)))
        self.draw_text((1,20), f"H:", font=font)
        self.draw_text((10, 20), f"{str(int(api.get_max_temp))}F", font=font, fill=self.get_temp_color(int(api.get_temp)))
        self.draw_text((30, 20), f"L:", font=font)
        self.draw_text((40, 20), f"{str(int(api.get_min_temp))}F", font=font, fill=self.get_temp_color(int(api.get_temp)))
    
    def render_icon(self, api):
        font = ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 9)
        owm_wxcode = int(api.get_weather[0]['id'])
        if owm_wxcode in range(200,299):
            # Thunderstorm Class
            owm_icon = 200
            color = (254, 204, 1)
        elif owm_wxcode in range(300,399):
            # Drizzle Class
            owm_icon = 300
            color = (220,220,220)
        elif owm_wxcode in range(500,599):
            # Rain Class
            owm_icon = 500
            color = (108, 204, 228)
        elif owm_wxcode in range(600,699):
            # Snow Class
            owm_icon = 600
            color = (255,255,255)
        elif owm_wxcode == 800:
            # Sunny
            owm_icon = 800
            color = (220, 149, 3)
        elif owm_wxcode in range(801,805):
            # Rain Class
            owm_icon = 801
            color = (220,220,220)
        else:
            owm_icon = owm_wxcode
        weather_icon = self.icons[str(owm_icon)]
        self.draw_text((50, 0), weather_icon.get_font, font, fill=color)
    def render_location(self, api: Weather):
        font = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF",8)
        self.draw_text((2, 1), api.get_place, font, (0, 254, 0))
    def render_humidity (self, api: Weather):
        font = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8)
        self.draw_text((2, 8), "H:", font)
        self.draw_text((10, 8), f"{api.get_humidity}%", font, fill=(7, 250, 246))
        self.draw_text((27, 8), f"P:", font)
        self.draw_text((34, 8), f"{int(api.get_precipitation * 100)}%", font, fill=(7, 250, 246))
    def render_wind(self, api: Weather):
        font = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8)
        speed = api.get_wind_speed
        deg = api.get_wind_deg
        self.draw_text((1, 12), "\uf050", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 9))
        self.draw_text((15, 15), f"{str(int(deg))}", font, fill=(201, 1, 253))
        self.draw_text((30, 13), "\uf042", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 9), fill=(201, 1, 253))
        self.draw_text((36, 15), f"{str(int(speed))}mph", font, fill=(201, 1, 253))
    def render_time(self, api: Weather):
        font = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8)
        sunrise = api.get_sunrise.strftime("%H:%M")
        sunset = api.get_sunset.strftime("%H:%M")
        self.draw_text((1, 18), "\uf058", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 11), fill=(255, 255, 0))
        self.draw_text((7, 23), sunrise, font=font)
        self.draw_text((35, 18), "\uf044", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 11), fill=(255, 145, 0))
        self.draw_text((40, 23), sunset, font=font)
    def render(self, api: Weather):
        self.logger.info("Rendering Weather Matrix")
        self.logger.debug("Clearing Image")
        self.clear()
        self.logger.debug("Reloading Image in matrix")
        self.reload_image()
        self.render_temp(api)
        self.render_icon(api)
        self.render_location(api)
        self.logger.info("Loading Screen 1 of Matrix")
        self.render_image()
        time.sleep(30)
        self.clear()
        self.logger.debug("Reloading Image in matrix")
        self.reload_image()
        self.render_location(api)
        self.render_icon(api)
        self.render_humidity(api)
        self.render_wind(api)
        self.render_time(api)
        self.logger.info("Loading Screen 2 of Matrix")
        self.render_image()
        time.sleep(30)

