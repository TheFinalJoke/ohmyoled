from dataclasses import dataclass
from datetime import datetime
from typing import Tuple, Union
from enum import Enum

class APIWeather(Enum):
    OPENWEATHER = 1
    # National Weather Service
    NWS = 2
    
@dataclass(repr=True)
class WeatherIcon:
    condition: str
    icon: str
    font: str
    time_of_day: str
    url: str = None
    owmcode: int = None
    
    def request_url_icon(self):
        pass

@dataclass(repr=True)
class CurrentWeather():
    conditions: str
    temp: Union[int, float]
    feels_like: Union[int, float]
    wind_speed: int
    humidity: int
    perciptation_chance: int
    uv: int
    wind_direction: int = None
    weather_icon: WeatherIcon = None

@dataclass(repr=True)
class DayForcast:
    todayhigh: int
    todaylow: int
    sunrise: datetime
    sunset: datetime

@dataclass(repr=True)
class WeatherBase():
    api: APIWeather
    # (lat, lng)
    location: Tuple[float, float]
    location_name: str
    current: CurrentWeather
    dayforcast: DayForcast

@dataclass(repr=True)
class Weather():
    api: APIWeather
    # (lat, lng)
    location: Tuple[float, float]
    location_name: str
    current: CurrentWeather
    dayforcast: DayForcast

@dataclass(repr=True)
class OpenWeatherResult(WeatherBase):
    api: APIWeather
    # (lat, lng)
    location: Tuple[float, float]
    location_name: str
    current: CurrentWeather
    dayforcast: DayForcast