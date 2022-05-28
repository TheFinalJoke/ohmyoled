from dataclasses import dataclass
from datetime import datetime
from typing import Tuple, Union
from enum import Enum
from ohmyoled.lib.run import Caller

class WeatherApiException(Exception):
    pass

@dataclass(repr=True, init=True)
class WeatherErrorResult(Caller):
    error: bool = False
    msg: str = None

class APIWeather(Enum):
    OPENWEATHER = 1
    # National Weather Service
    NWS = 2

class WindDirection(Enum):
    S = 180
    N = 0
    W = 270
    E = 90
    NE = 45
    NW = 315
    NNE = 30
    ENE = 75
    ESE = 105
    SE = 135
    SSE = 165
    SSW = 210
    SW = 225
    WSW = 255
    WNW = 285
    NNW = 345

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

def get_wind_dir_direction(deg) -> str:
    if deg <= WindDirection.N.value and deg < WindDirection.NNE.value:
        return WindDirection.N.name
    elif deg <= WindDirection.NNE.value and deg < WindDirection.NE.value:
        return WindDirection.NNE.name
    elif deg <= WindDirection.NE.value and deg < WindDirection.ENE.value:
        return WindDirection.NE.name
    elif deg <= WindDirection.ENE.value and deg < WindDirection.E.value:
        return WindDirection.ENE.name
    elif deg <= WindDirection.E.value and deg < WindDirection.ESE.value:
            return WindDirection.E.name
    elif deg <= WindDirection.ESE.value and deg < WindDirection.SE.value:
        return WindDirection.ESE.name
    elif deg <= WindDirection.SE.value and deg < WindDirection.SSE.value:
        return WindDirection.SE.name
    elif deg <= WindDirection.SSE.value and deg < WindDirection.S.value:
        return WindDirection.SSE.name
    elif deg <= WindDirection.S.value and deg < WindDirection.SSW.value:
        return WindDirection.S.name
    elif deg <= WindDirection.SSW.value and deg < WindDirection.SW.value:
        return WindDirection.SSW.name
    elif deg <= WindDirection.SW.value and deg < WindDirection.WSW.value:
        return WindDirection.SW.name
    elif deg <= WindDirection.WSW.value and deg < WindDirection.W.value:
        return WindDirection.WSW.name
    elif deg <= WindDirection.W.value and deg < WindDirection.WNW.value:
        return WindDirection.W.name
    elif deg <= WindDirection.WNW.value and deg < WindDirection.NW.value:
        return WindDirection.WNW.name
    elif deg <= WindDirection.NW.value and deg < WindDirection.NNW.value:
        return WindDirection.NW.name
    else:
        return WindDirection.NNW.value