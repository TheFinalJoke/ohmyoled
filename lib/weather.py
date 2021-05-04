#!/usr/bin/env python3

from lib.run import Runner
import sys
import os

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
            sys.exit("No Weather Token")

    def parse_args(self):
        if 'zipcode' in self.weather:
            return self.url_builder(zipcode=self.weather.getint('zipcode'))
        else:
            return self.url_builder(location=self.weather['city'])


    def get_long_and_lat(self, location):
        try: 
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
        args = self.parse_args()
        api_data = self.get_data(args)
        return {"weather": {"city": api_data}}