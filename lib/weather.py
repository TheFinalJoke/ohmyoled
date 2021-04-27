#!/usr/bin/env python3

from lib.run import Runner, RunnerABS


WEATHER_TOKEN  = '80ce462129470ef2f5d55e6f65d32eef'

class WeatherApi(RunnerABS):
    def __init__(self, location=None, zipcode=None):
        super().__init__()
        self.location = location
        self.zipcode = zipcode
        self.runner = Runner()

    def parse_args(self, location):
        pass 

    def get_long_and_lat(self, location):
        try: 
            url = f'http://api.openweathermap.org/data/2.5/weather?q={location}&appid={WEATHER_TOKEN}'
            response = self.runner.get_data(url)
            lon = response.json().get('coord').get('lon')
            lat = response.json().get('coord').get('lat')
            return lon, lat
        except Exception as e:
            return "No", "City Found"

    def url_builder(self, location=None, zipcode=None):
        if location:
            lon, lat = self.get_long_and_lat(location)
            url = f"http://api.openweathermap.org/data/2.5/weather?lat={lat}&lon={lon}&appid={WEATHER_TOKEN}"
        else:
            url = f"http://api.openweathermap.org/data/2.5/weather?zip={str(zipcode)},US&appid={WEATHER_TOKEN}&units=imperial"
        return url
    
    async def run(self, location=None, zipcode=None):
        """
        Get Args
        parse args
        Build URL
        make request
        return Json
        """
        if not location:
            location = self.location 
        elif not zipcode:
            zipcode = self.zipcode
        # location = self.parse_args(location)
        url = self.url_builder(location, zipcode)
        api_data = self.runner.get_data(url)
        return api_data.json()