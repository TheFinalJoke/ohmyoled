import ohmyoled.lib.tester as tester
import ohmyoled.lib.weather.normal as normal_types
import ohmyoled.lib.weather.openweather.weather as weather_types
import unittest.mock as mock_types
import asyncio

class TestWeatherApi(tester.ApiTester):
    def setUp(self) -> None:
        self.config = super().config()["weather"]
        self.weather_return = weather_types.OpenWeather({"name": "foo"})
        self.weather = normal_types.WeatherApi(self.config)
    @mock_types.patch.object(normal_types.open_weather_types.OpenWeatherApi, "run")
    def test_run_weather_open_weather(self, weather_data) -> None:
        weather_data.return_value = normal_types.NormalizedWeather(self.weather_return)
        result = self.weather.run_weather()
        self.assertEqual(result.get_place, "foo")
        