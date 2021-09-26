#!/usr/bin/env python3

from lib.run import Runner, Caller
import sys
import os
import json
from typing import Dict, Tuple
from datetime import datetime 
import csv

def get_weather_csv():
    csv_path = '/etc/ohmyoled/ecIcons_utf8.csv'
    return list(csv.DictReader(open(csv_path)))
def build_weather_icons():
    csv = get_weather_csv()
    icon = {}
    for icn in csv:
        icon.update({icn["OWMCode"]: WeatherIcon(icn['OWMCode'], icn['Description'], icn['fontcode'], icn['font'])})
    return icon

class WeatherIcon():
    def __init__(self, owm_id, description, fontcode, font) -> None:
        self.owm_id = owm_id
        self.description = description
        self.fontcode = fontcode
        self.font = font

    @property
    def get_owm_id(self):
        return self.owm_id
    @property
    def get_description(self):
        return self.description
    @property
    def get_fontcode(self):
        return self.fontcode
    @property
    def get_font(self):
        return self.font
    


class WeatherApi(Runner):
    """
    Weatherapi object 
    To parse the config file and 
    Run Data  
    """
    def __init__(self, config):
        super().__init__(config)
        self.weather = self.config['weather']
        try:
            if "open_weather_token" in self.config['basic']:
                self.token = self.config['basic'].get('open_weather_token')
            else:
                self.token = os.environ['WEATHERTOKEN']
        except KeyError:
            self.logger.critical("No Weather Token")
            sys.exit("No Weather Token")

    async def parse_args(self) -> str:
        """
        Check if zipcode or city in config file
        """
        if 'current_location' in self.weather:
            self.logger.debug("Using Current location via ipinfo")
            return await self.url_builder(current_location=True)
        elif 'zipcode' in self.weather:
            self.logger.debug(f"Using Zipcode {self.weather.getint('zipcode')}")
            return await self.url_builder(zipcode=self.weather.getint('zipcode'))
        else:
            self.logger.debug(f"Using City {self.weather['city']}")
            return await self.url_builder(location=self.weather['city'])


    async def get_long_and_lat(self, location: str=None, zipcode: int=None) -> Tuple:
        """
        Searches for Longitude and latitude for Given City
        """
        self.logger.debug("Getting Lat and Long")
        try: 
            if location:
                self.logger.debug("Getting Longitude and Latitude")
                url = f'http://api.openweathermap.org/data/2.5/weather?q={location}&appid={self.token}'
                response = await self.get_data(url)
                lon = response.get('coord').get('lon')
                lat = response.get('coord').get('lat')
                return lon, lat
            else:
                raise Exception("Zip Code not Supported")
        except Exception as e:
            self.logger.critical(e)
            sys.exit("No City Found")
    def get_current_location(self):
        url = 'http://ipinfo.io/json'
        response = self.run_non_async_request(url)
        return response.json()
    
    async def url_builder(self, location=None, zipcode=None, current_location=False):
        """
        Builds Url to poll the Api
        """
        self.logger.debug("Building url...")
        if current_location:
            ip_json = self.get_current_location()
            lon, lat = ip_json['loc'].split(',')[1], ip_json['loc'].split(',')[0]
            url = f"https://api.openweathermap.org/data/2.5/onecall?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('format')}"
        elif location:
            lon, lat = await self.get_long_and_lat(location)
            url = f"https://api.openweathermap.org/data/2.5/onecall?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('format')}"
        else:
            ip_json = self.get_current_location()
            lon, lat = ip_json['loc'].split(',')[1], ip_json['loc'].split(',')[0]
            url = f"https://api.openweathermap.org/data/2.5/onecall?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('format')}"
        return url
    
    async def run(self):
        """
        Get Args
        parse args
        Build URL
        make request
        return Json
        """
        self.logger.info("Using to get Weather")
        args = await self.parse_args()
        api_data = await self.get_data(args)
        api_data['name'] = self.get_current_location()['city']
        return api_data

class Weather(Caller):
    """
    Weather object to describe current Polled data
    """
    def __init__(self, api: Dict) -> None:
        super().__init__()
        self.api = api
        self.api_json = api
        self._place = self.api_json.get('name')
        self._current = self.api_json.get('current')
        self._weather = self._current.get('weather')
        self._conditions = self._weather[0].get('main')
        self._weather_icon = self._weather[0].get('icon')
        self._temp = self._current.get('temp')
        self._feels_like = self._current.get('feels_like')
        self._daily = self.api_json.get('daily')[0]
        self._min_temp = self._daily['temp']['min']
        self._max_temp = self._daily['temp']['max']
        self._humidity = self._current.get('humidity')
        self._wind_speed = self._current.get('wind_speed')
        self._wind_deg = self._current.get('wind_deg')
        self._time = datetime.fromtimestamp(self._current.get('dt'))
        self._sunrise = datetime.fromtimestamp(self._current.get('sunrise'))
        self._sunset = datetime.fromtimestamp(self._current.get('sunset'))
        self._pop = self._daily['pop']
        self._uv = self._daily['uvi']

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
            f"uv={self._uv}"
        ]
        joined_attrs = ',\n'.join(attrs)
        return f"Weather(\n{joined_attrs})"
    
    @property
    def get_wind_speed(self):
        return self._wind_speed
    
    def set_wind_speed(self, speed):
        self._wind_speed = speed
    
    @property
    def get_daily(self):
        return self._daily
    
    @property
    def get_wind_deg(self):
        return self._wind_deg
    
    @property
    def get_precipitation(self):
        return self._pop
    
    @property
    def get_uv(self):
        return self._uv
    
    def set_place(self, place):
        self._place = place
    
    @property
    def get_place(self):
        return self._place

    def set_weather(self, weather):
        self._weather = weather
        
    @property
    def get_weather(self):
        return self._weather
    
    def set_conditions(self, conditions):
        self._conditions = conditions
    
    @property
    def get_conditions(self):
        return self._conditions
    
    def set_weather_icon(self, icon):
        self._weather_icon = icon
    
    @property
    def get_weather_icon(self):
        return self._weather_icon

    def set_temp(self, temp):
        self._temp = temp
    
    @property
    def get_temp(self):
        return self._temp
    
    def set_feels_like(self, feels):
        self._feels_like = feels
    
    @property
    def get_feels_like(self):
        return self._feels_like
    
    def set_min_temp(self, temp):
        self._min_temp = temp
    
    @property
    def get_min_temp(self):
        return self._min_temp
    
    def set_max_temp(self, temp):
        self._max_temp = temp
    
    @property
    def get_max_temp(self):
        return self._max_temp
    
    def set_humidity(self, humidity):
        self._humidity = humidity
    
    @property
    def get_humidity(self):
        return self._humidity

    def set_wind(self, wind: Dict) -> None:
        self._wind = wind
    
    @property
    def get_wind(self) -> Dict:
        return self._wind
    
    def set_time(self, time) -> None:
        self._time = datetime.fromtimestamp(time)
    
    @property
    def get_time(self) -> datetime:
        return self._time

    def set_sunrise(self, time) -> None:
        self._sunrise = datetime.fromtimestamp(time)
    
    @property
    def get_sunrise(self) -> datetime:
        return self._sunrise
    
    def set_sunset(self, time) -> None:
        self._sunset = datetime.fromtimestamp(time)
    
    @property
    def get_sunset(self) -> datetime:
        return self._sunset
    
    def calculate_duration_of_daylight(self):
        return self._sunset - self._time
