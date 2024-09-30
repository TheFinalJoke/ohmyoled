#!/usr/bin/env python3

import asyncio
import json
import os
import sys
import typing
from datetime import datetime, timedelta
import ipinfo
import ohmyoled.lib.weather.weatherbase as base
from ohmyoled.lib.asynclib import make_async
from ohmyoled.lib.run import Caller, Runner
from ohmyoled.lib.weather.weather_icon import weather_icon_mapping


class OpenWeatherException(Exception):
    pass


class OpenWeatherApi(Runner):
    """
    Weatherapi object
    To parse the config file and
    Run Data
    """

    def __init__(self, config) -> None:
        super().__init__(config)
        self.weather = self.config.get("weather")
        try:

            if "null" == self.weather.get("api_key"):
                raise KeyError
            elif "null" != self.weather.get("api_key"):
                self.token = self.weather.get("api_key")
            else:
                self.token = os.environ["WEATHERTOKEN"]
        except KeyError:
            self.logger.critical("No Weather Token")
            sys.exit("No Weather Token")

    async def parse_args(self) -> str:
        """
        Check if zipcode or city in config file
        """
        if self.weather.get("current_location") != "null":
            self.logger.debug("Using Current location via ipinfo")
            return await self.url_builder(current_location=True)
        else:
            self.logger.debug(f"Using City {self.weather['city']}")
            if self.weather.get("city"):
                raise OpenWeatherException
            return await self.url_builder(location=self.weather["city"])

    async def get_long_and_lat(
        self,
        location: typing.Optional[str] = None,
        zipcode: typing.Optional[int] = None,
    ) -> typing.Tuple:
        """
        Searches for Longitude and latitude for Given City
        """
        self.logger.debug("Getting Lat and Long")
        try:
            if location:
                self.logger.debug("Computing Longitude and Latitude")
                url = f"http://api.openweathermap.org/data/3.0/weather?q={location}&appid={self.token}"
                self.logger.debug(f"Runinng to get location {url}")
                response = await self.get_data(url)
                lon = response.get("coord").get("lon", 0)  # type: ignore
                lat = response.get("coord").get("lat", 0)  # type: ignore
                return lon, lat
            else:
                raise Exception("Zip Code not Supported")
        except Exception as e:
            self.logger.critical(e)
            sys.exit("No City Found")

    async def get_current_location(self) -> typing.Dict[str, str]:
        if (
            not self.weather.get("current_location_api_key")
            or self.weather.get("current_location_api_key") == "null"
        ):
            raise Exception("No Key for Ip Info")
        handler = ipinfo.getHandler(
            access_token=self.weather.get("current_location_api_key")
        )
        response = handler.getDetails()
        return response.details

    async def url_builder(self, location=None, zipcode=None, current_location=False):
        """
        Builds Url to poll the Api
        """
        self.logger.debug("Building Weather url...")
        if current_location:
            ip_json: typing.Dict[str, str] = await self.get_current_location()  # type: ignore

            lon, lat = ip_json["longitude"], ip_json["latitude"]
            url = f"https://api.openweathermap.org/data/3.0/onecall?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('weather_format')}"
        elif location:
            lon, lat = await self.get_long_and_lat(location)
            url = f"https://api.openweathermap.org/data/3.0/onecall?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('weather_format')}"
        else:
            ip_json = await self.get_current_location()  # type: ignore
            lon, lat = ip_json["longitude"], ip_json["latitude"]
            url = f"https://api.openweathermap.org/data/3.0/onecall?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('weather_format')}"
        return url

    async def run(self):
        try:
            self.logger.info("Running Api for Weather")

            args = await self.parse_args()

            api_data = await self.get_data(args)

            current_data = await self.get_current_location()  # type: ignore

            api_data["name"] = current_data["city"]

            return OpenWeather(api_data)
        except Exception as e:
            raise base.WeatherApiException(e)


class OpenWeather(Caller):
    """
    Weather object to describe current Polled data
    """

    def __init__(self, api: typing.Dict) -> None:
        super().__init__()
        self.api = api
        self.api_json = api
        self._api_caller = base.APIWeather.OPENWEATHER
        self._lat_long = (self.api["lat"], self.api["lon"])
        self._place = self.api_json.get("name")
        self._current = self.api_json.get("current")
        self._weather = self._current.get("weather")  # type: ignore
        self._conditions = self._weather[0].get("main")
        self._weather_icon = self._weather[0].get("icon")
        self._temp = self._current.get("temp")  # type: ignore
        self._feels_like = self._current.get("feels_like")  # type: ignore
        self._daily = self.api_json.get("daily")[0]  # type: ignore
        self._min_temp = self._daily["temp"]["min"]
        self._max_temp = self._daily["temp"]["max"]
        self._humidity = self._current.get("humidity")  # type: ignore
        self._wind_speed = self._current.get("wind_speed")  # type: ignore
        self._wind_deg = self._current.get("wind_deg")  # type: ignore
        self._time = datetime.fromtimestamp(self._current.get("dt"))  # type: ignore
        self._sunrise = datetime.fromtimestamp(self._current.get("sunrise"))  # type: ignore
        self._sunset = datetime.fromtimestamp(self._current.get("sunset"))  # type: ignore
        self._pop = self._daily["pop"]
        self._uv = self._daily["uvi"]

    def __repr__(self) -> str:
        attrs = [
            f"name={self._place}",
            f"current={json.dumps(self._current, indent=2)}",
            f"weather={json.dumps(self._weather, indent=2)}",
            f"conditions={self._conditions}",
            f"weather_icon={self._weather_icon}",
            f"temp={self._temp}",
            f"feels_like={self._feels_like}",
            f"daily={json.dumps(self._daily, indent=2)}",
            f"min_temp={self._min_temp}",
            f"max_temp={self._max_temp}",
            f"humidity={self._humidity}",
            f"wind_speed={self._wind_speed}",
            f"wind_deg={self._wind_deg}",
            f"time={self._time}",
            f"sunrise={self._sunrise}",
            f"sunset={self._sunset}",
            f"precipitation={self._pop}",
            f"uv={self._uv}",
        ]
        joined_attrs = ",\n".join(attrs)
        return f"Weather(\n{joined_attrs})"

    @property
    def get_icon(self):
        owm_wxcode: int = int(self._weather[0]["id"])  # type: ignore
        if owm_wxcode in range(200, 299):
            # Thunderstorm Class
            owm_icon = weather_icon_mapping[17]
        elif owm_wxcode in range(300, 399):
            # Drizzle Class
            owm_icon = weather_icon_mapping[23]
        elif owm_wxcode in range(500, 599):
            # Rain Class
            owm_icon = weather_icon_mapping[9]
        elif owm_wxcode in range(600, 699):
            # Snow Class
            owm_icon = weather_icon_mapping[13]
        elif owm_wxcode in range(700, 780):
            owm_icon = weather_icon_mapping[36]
        elif owm_wxcode == 800:
            # Sunny
            if self._sunset > datetime.now():
                owm_icon = weather_icon_mapping[0]
            else:
                owm_icon = weather_icon_mapping[48]

        elif owm_wxcode in range(801, 805):
            # Rain Class
            if self._sunset > datetime.now():
                owm_icon = weather_icon_mapping[3]
            else:
                owm_icon = weather_icon_mapping[48]
        else:
            owm_icon = weather_icon_mapping[0]

        return owm_icon

    @property
    def get_api(self):
        return self._api_caller

    @property
    def get_lat_long(self) -> typing.Tuple[float, float]:
        return self._lat_long

    @property
    def get_wind_speed(self) -> int:
        return self._wind_speed

    def set_wind_speed(self, speed: int) -> None:
        self._wind_speed = speed

    @property
    def get_daily(self) -> typing.Dict[str, str]:
        return self._daily

    @property
    def get_wind_deg(self) -> int:
        return self._wind_deg

    @property
    def get_precipitation(self) -> int:
        return self._pop * 100

    @property
    def get_uv(self) -> int:
        return self._uv

    def set_place(self, place: str) -> None:
        self._place = place

    @property
    def get_place(self) -> str:
        return self._place  # type: ignore

    def set_weather(self, weather: typing.Dict[str, str]) -> None:
        self._weather = weather

    @property
    def get_weather(self) -> typing.Dict[str, str]:
        return self._weather

    def set_conditions(self, conditions: str) -> None:
        self._conditions = conditions

    @property
    def get_conditions(self) -> str:
        return self._conditions

    def set_weather_icon(self, icon: str) -> None:
        self._weather_icon = icon

    @property
    def get_weather_icon(self) -> str:
        return self._weather_icon

    def set_temp(self, temp: int) -> None:
        self._temp = temp

    @property
    def get_temp(self) -> int:
        return self._temp

    def set_feels_like(self, feels: int):
        self._feels_like = feels

    @property
    def get_feels_like(self) -> int:
        return self._feels_like

    def set_min_temp(self, temp: int) -> None:
        self._min_temp = temp

    @property
    def get_min_temp(self) -> int:
        return self._min_temp

    def set_max_temp(self, temp: int) -> None:
        self._max_temp = temp

    @property
    def get_max_temp(self) -> int:
        return self._max_temp

    def set_humidity(self, humidity: int) -> None:
        self._humidity = humidity

    @property
    def get_humidity(self) -> None:
        return self._humidity  # type: ignore

    def set_wind(self, wind: typing.Dict) -> None:
        self._wind = wind

    @property
    def get_wind(self) -> typing.Dict:
        return self._wind

    def set_time(self, time: int) -> None:
        self._time = datetime.fromtimestamp(time)

    @property
    def get_time(self) -> datetime:
        return self._time

    def set_sunrise(self, time: int) -> None:
        self._sunrise = datetime.fromtimestamp(time)

    @property
    def get_sunrise(self) -> datetime:
        return self._sunrise

    def set_sunset(self, time) -> None:
        self._sunset = datetime.fromtimestamp(time)

    @property
    def get_sunset(self) -> datetime:
        return self._sunset

    def calculate_duration_of_daylight(self) -> timedelta:
        return self._sunset - self._time
