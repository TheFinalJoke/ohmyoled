#!/usr/bin/env python3

from lib.run import Runner, Caller
import sys
import os
from requests import Response
from typing import Dict
from datetime import datetime 

class WeatherApi(Runner):
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

    def parse_args(self):
        if 'zipcode' in self.weather:
            self.logger.debug(f"Using Zipcode {self.weather.getint('zipcode')}")
            return self.url_builder(zipcode=self.weather.getint('zipcode'))
        else:
            self.logger.debug(f"Using City {self.weather['city']}")
            return self.url_builder(location=self.weather['city'])


    def get_long_and_lat(self, location):
        try: 
            self.logger.debug("Getting Longitude and Latitude")
            url = f'http://api.openweathermap.org/data/2.5/weather?q={location}&appid={self.token}'
            response = self.get_data(url)
            lon = response.json().get('coord').get('lon')
            lat = response.json().get('coord').get('lat')
            return lon, lat
        except Exception as e:
            sys.exit("No City Found")

    def url_builder(self, location=None, zipcode=None):
        if location:
            lon, lat = self.get_long_and_lat(location)
            url = f"http://api.openweathermap.org/data/2.5/weather?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('format')}"
        else:
            url = f"http://api.openweathermap.org/data/2.5/weather?zip={str(zipcode)},US&appid={self.token}&units={self.weather.get('format')}"
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
        args = self.parse_args()
        api_data = self.get_data(args)
        return {"weather": api_data}

class Weather(Caller):
    def __init__(self, api: Dict) -> None:
        super().__init__()
        self.api = api
        self.api_json = api['weather'].json()
        self._place = self.api_json.get('name')
        self._weather = self.api_json.get('weather')
        self._conditions = self._weather[0].get('main')
        self._weather_icon = self._weather[0].get('icon')
        self._temp = self.api_json.get('main').get('temp')
        self._feels_like = self.api_json['main'].get('feels_like')
        self._min_temp = self.api_json['main'].get('temp_min')
        self._max_temp = self.api_json['main'].get('temp_max')
        self._humidity = self.api_json['main'].get('humidity')
        self._wind = self.api_json.get('wind')
        self._time = datetime.fromtimestamp(self.api_json.get('dt'))
        self._sunrise = datetime.fromtimestamp(self.api_json['sys'].get('sunrise'))
        self._sunset = datetime.fromtimestamp(self.api_json['sys'].get('sunset'))

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
        return self._sunset - self._sunrise
