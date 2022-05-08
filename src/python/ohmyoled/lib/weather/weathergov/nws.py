#!/usr/bin/env python3
import asyncio
import sys
import json
import datetime as dt
import ohmyoled.lib.weather.weatherbase as base 
from ohmyoled.lib.weather.weather_icon import weather_icon_mapping
from ohmyoled.lib.asynclib import make_async
from ohmyoled.lib.run import Runner, Caller
from datetime import datetime, timedelta
from typing import Tuple, Dict, List
from suntime import Sun

class NWSApi(Runner):

    def __init__(self, config) -> None:
        super().__init__(config)
        self.weather = self.config['weather']

    async def parse_args(self) -> str:
        """
        Check if zipcode or city in config file
        """

        return await self.url_builder()


    async def get_long_and_lat(self, location: str=None, zipcode: int=None, url=None) -> Tuple:
        """
        Searches for Longitude and latitude for Given City
        """
        self.logger.debug("Getting Lat and Long")
        try: 
            if location:
                self.logger.debug("Computing Longitude and Latitude")
                response = await self.get_data(url)
                lon = response.get('coord').get('lon')
                lat = response.get('coord').get('lat')
                return lon, lat
            else:
                raise Exception("Zip Code not Supported")
        except Exception as e:
            self.logger.critical(e)
            sys.exit("No City Found")
    
    @make_async
    def get_current_location(self) -> Dict[str, str]:
        url = 'http://ipinfo.io/json'
        response = self.run_non_async_request(url)
        return response.json()
    
    async def url_builder(self):
        """
        Builds Url to poll the Api
        """
        self.logger.debug("Building Weather url...")
        ip_json: Dict[str, str] = await self.get_current_location()
        lon, lat = ip_json['loc'].split(',')[1], ip_json['loc'].split(',')[0]
        url = f"https://api.weather.gov/points/{lat},{lon}"
        return url
    
    async def run(self) -> Dict:
        self.logger.info("Running Api for Weather")
        args = await self.parse_args()
        main_json = await self.get_data(args)
        observation_url = f'https://api.weather.gov/stations/{main_json["properties"]["radarStation"]}/observations/latest'
        tasks = {
            'forcast': asyncio.create_task(self.get_data(main_json['properties']['forecast'])),
            'hourly': asyncio.create_task(self.get_data(main_json['properties']['forecastHourly'])),
            'observations': asyncio.create_task(self.get_data(observation_url))
        }
        await asyncio.gather(*tasks.values())
        count = 0
        while count <= 4:
            if not all([('status' in tasks['hourly'].result()), ('status' in tasks['forcast'].result())]):
                break
            count += 1
            tasks['hourly'] = asyncio.create_task(self.get_data(main_json['properties']['forecastHourly']))
            tasks['forcast'] = asyncio.create_task(self.get_data(main_json['properties']['forecast']))
            await asyncio.gather(tasks['hourly'], tasks['forcast'])
        api_data = {'main_json': main_json, 'forcast': tasks['forcast'].result(), 'hourly': tasks['hourly'].result(), 'observe': tasks['observations'].result()}
        return NWSTransform(api_data)

class NWSTransform(Caller):

    def __init__(self, api: Dict) -> None:
        super().__init__()
        self.api = api
        self.api_json = api
        self._api_caller = base.APIWeather.NWS
        self._observation = self.api_json['observe']
        self._late_observation = self._observation
        self._lat_long = (self.api['main_json']['geometry']['coordinates'][1], self.api['main_json']['geometry']['coordinates'][0])
        self._place = self.api_json['main_json']['properties']['relativeLocation']['properties']['city']
        self._current = self.api_json['hourly']['properties']['periods'][0]
        self._weather = self.api_json['hourly']['properties']['periods']
        self._conditions = self._current['shortForecast']
        self._temp = int((self._late_observation['properties']['temperature']['value'] * 1.8) + 32)
        # There is no Feels like
        self._feels_like = self._temp
        self._daily = self.api_json['forcast']['properties']['periods'][0]
        # Have to figure out how to get the temp mina nd max with 
        self._min_temp = min(self.determine_max_and_min_temps())
        self._max_temp = max(self.determine_max_and_min_temps())
        self._humidity = int(self._late_observation['properties']['relativeHumidity']['value'])
        self._wind_speed = int(self._late_observation['properties']['windSpeed']['value'] / 1.609344)
        self._wind_deg = self._late_observation['properties']['windDirection']['value']
        self._time = datetime.now()
        self._sunrise = self.gen_rise_and_set()[0]
        self._sunset = self.gen_rise_and_set()[1]
        self._pop = 0
        self._uv = None

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
    def get_icon(self):
        # Have to get the icon 
        condition: str = self._weather[0]['shortForecast'].lower()
        if any(s in condition.lower() for s in ("sunny", "clear", 'sun')):
             # Sunny
            if self._sunset.replace(tzinfo=None) > datetime.now():
                owm_icon = weather_icon_mapping[0]
            else:
                owm_icon = weather_icon_mapping[48]
        elif any(s in condition.lower() for s in ('rain', 'storm', 'thunderstorm ')):
            owm_icon = weather_icon_mapping[9]
        elif 'snow' in condition:
            owm_icon = weather_icon_mapping[13]
        elif any(s in condition.lower() for s in ('cloudy', 'cloud')):
            owm_icon = weather_icon_mapping[7]
        else:
            owm_icon = weather_icon_mapping[0]
        return owm_icon

    def determine_max_and_min_temps(self) -> List[int]:
        return [entry['temperature'] for entry in self._weather if datetime.now().date() == datetime.fromisoformat(entry['startTime']).date()]
    
    def gen_rise_and_set(self):
        lat, lng = self._lat_long
        tz = datetime.now().date()
        sun = Sun(lat, lng)
        sun_rise = sun.get_local_sunrise_time(tz)
        sun_set = sun.get_local_sunset_time(tz)
        return sun_rise, sun_set
    @property
    def get_api(self):
        return self._api_caller

    @property 
    def get_lat_long(self) -> Tuple[float, float]:
        return self._lat_long
    
    @property
    def get_wind_speed(self) -> int:
        return self._wind_speed
    
    @property
    def get_daily(self) -> Dict[str, str]:
        return self._daily
    
    @property
    def get_wind_deg(self) -> int:
        return self._wind_deg
    
    @property
    def get_precipitation(self) -> int:
        return self._pop
    
    @property
    def get_uv(self) -> int:
        return self._uv
    
    @property
    def get_place(self) -> str:
        return self._place
        
    @property
    def get_weather(self) -> Dict[str, str]:
        return self._weather
    
    @property
    def get_conditions(self) -> str:
        return self._conditions
    
    @property
    def get_weather_icon(self) -> str:
        return self._weather_icon
    
    @property
    def get_temp(self) -> int:
        return self._temp
    
    @property
    def get_feels_like(self) -> int:
        return self._feels_like
    
    @property
    def get_min_temp(self) -> int:
        return self._min_temp
    
    @property
    def get_max_temp(self) -> int:
        return self._max_temp

    @property
    def get_humidity(self) -> None:
        return self._humidity

    @property
    def get_wind(self) -> Dict:
        return self._wind

    @property
    def get_time(self) -> datetime:
        return self._time
    
    @property
    def get_sunrise(self) -> datetime:
        return self._sunrise
    
    @property
    def get_sunset(self) -> datetime:
        return self._sunset
    
    def calculate_duration_of_daylight(self) -> timedelta:
        return self._sunset - self._time
