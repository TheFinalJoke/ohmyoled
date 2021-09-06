#!/usr/bin/env python3

from lib.run import Runner, Caller
import sys
import os
from requests import Response
import geocoder
from typing import Dict, Tuple
from datetime import datetime 
from enum import Enum
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
            self.logger.debug("Using Current location via ipstack")
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
        try: 
            if location:
                self.logger.debug("Getting Longitude and Latitude")
                url = f'http://api.openweathermap.org/data/2.5/weather?q={location}&appid={self.token}'
                response = await self.get_data(url)
                lon = response.get('coord').get('lon')
                lat = response.get('coord').get('lat')
                return lon, lat
        except Exception as e:
            sys.exit("No City Found")
    def get_current_location(self):
        g = geocoder.ip('me')
        return g.latlng[1], g.latlng[0]
    
    async def url_builder(self, location=None, zipcode=None, current_location=False):
        """
        Builds Url to poll the Api
        """
        if current_location:
            lon, lat = self.get_current_location()
            url = f"https://api.openweathermap.org/data/2.5/onecall?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('format')}"
        elif location:
            lon, lat = await self.get_long_and_lat(location)
            url = f"https://api.openweathermap.org/data/2.5/onecall?lat={lat}&lon={lon}&appid={self.token}&units={self.weather.get('format')}"
        else:
            lis_location = geocoder.location(str(zipcode)).latlng
            url = f"https://api.openweathermap.org/data/2.5/onecall?lat={lis_location[1]}&lon={lis_location[0]}&appid={self.token}&units={self.weather.get('format')}"
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
        geoloc = geocoder.arcgis(method='reverse', location=f"{api_data['lat']}, {api_data['lon']}")
        api_data['name'] = geoloc.city
        return {"weather": api_data}

class Weather(Caller):
    """
    Weather object to describe current Polled data
    """
    def __init__(self, api: Dict) -> None:
        super().__init__()
        self.api = api
        breakpoint()
        self.api_json = api
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
        self._icn_url = self._weather[0].get('url')
        self._icn_img = self._weather[0].get('file')

    def set_icn_url(self, url):
        self._icn_url = url
    @property
    def get_icn_url(self):
        return self._icn_url

    def set_icn_img(self, img):
        self._icn_img = img
    
    @property
    def get_icn_img(self):
        return self._icn_img
    
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
