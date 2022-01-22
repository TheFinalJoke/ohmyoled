#!/usr/bin/env python3 

import time
from typing import Dict, Tuple
from datetime import datetime
from PIL import ImageFont
from matrix.matrix import Matrix
import lib.weather.weatherbase as base
from lib.weather.normal import NormalizedWeather

class WeatherMatrix(Matrix):
    def __init__(self, matrix, api: Dict, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger

    def __str__(self) -> str:
        return "WeatherMatrix"

    async def poll_api(self):
        result: NormalizedWeather = await self.api.run_weather()
        return base.Weather(
            api=result.get_api,
            location=result.get_lat_long,
            location_name=result.get_place,
            current=base.CurrentWeather(
                conditions=result.get_conditions,
                temp=result.get_temp,
                feels_like=result.get_feels_like,
                wind_speed=result.get_wind_speed,
                humidity=result.get_humidity,
                perciptation_chance=result.get_precipitation,
                uv=result.get_uv,
                wind_direction=result.get_wind_deg,
                weather_icon=result.get_icon,
            ),
            dayforcast=base.DayForcast(
                todayhigh=result.get_max_temp,
                todaylow=result.get_min_temp,
                sunrise=result.get_sunrise,
                sunset=result.get_sunset
            )
        )
        
    
    def get_temp_color(self, temp: int) -> Tuple[int, int, int]:
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
    
    def render_temp(self, api) -> None:
        font: ImageFont = ImageFont.truetype("/usr/share/fonts/retro_computer.ttf", 7)
        self.draw_text((0, 8), "T:", font=font)
        self.draw_text((10, 8), f"{str(int(api.current.temp))}F", font=font, fill=self.get_temp_color(int(api.current.temp)))
        self.draw_text((30, 8), "R:", font=font)
        self.draw_text((40, 8), f"{str(int(api.current.feels_like))}F", font=font, fill=self.get_temp_color(int(api.current.feels_like)))
        self.draw_text((1, 18), f"H:", font=font)
        self.draw_text((10, 18), f"{str(int(api.dayforcast.todayhigh))}F", font=font, fill=self.get_temp_color(int(api.dayforcast.todayhigh)))
        self.draw_text((30, 18), f"L:", font=font)
        self.draw_text((40, 18), f"{str(int(api.dayforcast.todaylow))}F", font=font, fill=self.get_temp_color(int(api.dayforcast.todaylow)))
    
    def render_icon(self, api) -> None:
        font: ImageFont = ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 9)
        owm_wxcode: int = api.current.weather_icon.owmcode
        if owm_wxcode in range(200,299):
            # Thunderstorm Class
            color: Tuple[int] = (254, 204, 1)
        elif owm_wxcode in range(300,399):
            # Drizzle Class
            color: Tuple[int] = (220,220,220)
        elif owm_wxcode in range(500,599):
            # Rain Class
            color: Tuple[int] = (108, 204, 228)
        elif owm_wxcode in range(600,699):
            # Snow Class
            color: Tuple[int] = (255,255,255)
        elif owm_wxcode in range(700, 780):
            color: Tuple[int] = (192, 192, 192)
        elif owm_wxcode == 800:
            # Sunny
            if api.dayforcast.sunset.replace(tzinfo=None) > datetime.now():
                color: Tuple[int] = (220, 149, 3)
            else:
                color: Tuple[int] = (255,255,255)
        elif owm_wxcode in range(801,805):
            color: Tuple[int] = (220,220,220)
        else:
            owm_icon = owm_wxcode
            color: Tuple[int] = (255,255,255)
        self.draw_text((50, 0), api.current.weather_icon.font, font, fill=color)

    def render_location(self, api: base.Weather, xpos) -> None:
        font = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF",8)
        self.draw_text((-xpos, 1), api.location_name, font, (0, 254, 0))

    def render_humidity (self, api: base.Weather) -> None:
        font = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8)
        self.draw_text((2, 8), "H:", font)
        self.draw_text((10, 8), f"{api.current.humidity}%", font, fill=(7, 250, 246))
        self.draw_text((27, 8), f"P:", font)
        self.draw_text((34, 8), f"{int(api.current.perciptation_chance)}%", font, fill=(7, 250, 246))

    def render_wind(self, api: base.Weather) -> None:
        font = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8)
        speed = api.current.wind_speed
        deg = api.current.wind_direction
        self.draw_text((1, 12), "\uf050", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 9))
        self.draw_text((15, 15), f"{str(int(deg))}", font, fill=(201, 1, 253))
        if len(str(int(deg))) == 3:
            self.draw_text((30, 13), "\uf042", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 9), fill=(201, 1, 253))
        elif len(str(int(deg))) == 2:
            self.draw_text((29, 13), "\uf042", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 9), fill=(201, 1, 253))
        else:
            self.draw_text((28, 13), "\uf042", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 9), fill=(201, 1, 253))
        self.draw_text((36, 15), f"{str(int(speed))}mph", font, fill=(201, 1, 253))

    def render_time(self, api) -> None:
        font: ImageFont = ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8)
        sunrise: datetime = api.dayforcast.sunrise.strftime("%H:%M")
        sunset: datetime = api.dayforcast.sunset.strftime("%H:%M")
        self.draw_text((1, 18), "\uf058", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 11), fill=(255, 255, 0))
        self.draw_text((7, 23), sunrise, font=font)
        self.draw_text((35, 18), "\uf044", font=ImageFont.truetype("/usr/share/fonts/weathericons.ttf", 11), fill=(255, 145, 0))
        self.draw_text((40, 23), sunset, font=font)

    def render_conditions(self, api, xpos: int) -> None:
        self.draw_text((-xpos, 26), f"Conditions: {api.current.conditions}", font=ImageFont.truetype("/usr/share/fonts/04B_03B_.TTF", 8), fill=(255,255,255))
    
    async def render(self, api , loop) -> None:
        self.logger.info("Rendering Weather Matrix")
        self.logger.debug("Clearing Image")
        self.clear()
        self.logger.debug("Reloading Image in matrix")
        xpos = 0
        self.logger.info("Loading Screen 1 of Matrix")
        while xpos < 100:
            self.reload_image()
            self.render_temp(api)
            self.render_icon(api)
            self.render_location(api, xpos)
            self.render_conditions(api, xpos)
            xpos += 1
            await self.render_image()
            time.sleep(3) if xpos == 1 else time.sleep(.05)
        self.reload_image()
        self.render_temp(api)
        self.render_icon(api)
        self.render_location(api, 0)
        self.render_conditions(api, 0)
        await self.render_image()
        time.sleep(25)
        self.clear()
        self.logger.debug("Reloading Image in matrix")
        self.reload_image()
        xpos = 0
        self.logger.info("Loading Screen 2 of Matrix")
        while xpos < 100:
            self.reload_image()
            self.render_location(api, xpos)
            self.render_icon(api)
            self.render_humidity(api)
            self.render_wind(api)
            self.render_time(api)
            await self.render_image()
            xpos += 1
            time.sleep(3) if xpos == 1 else time.sleep(.05)
        self.reload_image()
        self.render_location(api, 0)
        self.render_icon(api)
        self.render_humidity(api)
        self.render_wind(api)
        self.render_time(api)
        await self.render_image()
        time.sleep(30)

