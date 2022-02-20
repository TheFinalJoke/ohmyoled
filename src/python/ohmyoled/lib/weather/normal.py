#!/usr/bin/env python3
from typing import Dict, Tuple
from datetime import datetime, timedelta
from ohmyoled.lib.weather.openweather.weather import OpenWeatherApi
from ohmyoled.lib.weather.weathergov.nws import NWSApi
from ohmyoled.lib.run import Caller

class NormalizedWeather():

    def __init__(self, api_result) -> None:
        self.api_result = api_result
    
    @property
    def get_api(self):
        return self.api_result.get_api

    @property
    def get_icon(self):
        return self.api_result.get_icon
        
    @property
    def get_lat_long(self) -> Tuple[float, float]:
        return self.api_result.get_lat_long

    @property
    def get_wind_speed(self) -> int:
        return self.api_result.get_wind_speed
    
    @property
    def get_daily(self) -> Dict[str, str]:
        return self.api_result.get_daily
    
    @property
    def get_wind_deg(self) -> int:
        return self.api_result.get_wind_deg
    
    @property
    def get_precipitation(self) -> int:
        return self.api_result.get_precipitation * 100
    
    @property
    def get_uv(self) -> int:
        return self.api_result.get_uv
    
    @property
    def get_place(self) -> str:
        return self.api_result.get_place
        
    @property
    def get_weather(self) -> Dict[str, str]:
        return self.api_result.get_weather
    
    @property
    def get_conditions(self) -> str:
        return self.api_result.get_conditions
    
    @property
    def get_weather_icon(self) -> str:
        return self.api_result.get_weather_icon
    
    @property
    def get_temp(self) -> int:
        return self.api_result.get_temp
    
    @property
    def get_feels_like(self) -> int:
        return self.api_result.get_feels_like
    
    @property
    def get_min_temp(self) -> int:
        return self.api_result.get_min_temp
    
    @property
    def get_max_temp(self) -> int:
        return self.api_result.get_max_temp
    
    @property
    def get_humidity(self) -> None:
        return self.api_result.get_humidity
    
    @property
    def get_wind(self) -> Dict:
        return self.api_result.get_wind
    
    @property
    def get_time(self) -> datetime:
        return self.api_result.get_time
    
    @property
    def get_sunrise(self) -> datetime:
        return self.api_result.get_sunrise
    
    @property
    def get_sunset(self) -> datetime:
        return self.api_result.get_sunset
    
    def calculate_duration_of_daylight(self) -> timedelta:
        return self.api_result.get_sunset - self.api_result.get_time

class WeatherApi(Caller):
    
    def __init__(self, config) -> None:
        super().__init__()
        self.config = config 

    async def run_weather(self):
        if self.config['weather']['api'] == "openweather":
            open_weather = OpenWeatherApi(self.config)
            result = await open_weather.run()
        else:
            nws = NWSApi(self.config)
            result = await nws.run()
        return NormalizedWeather(result)