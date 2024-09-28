import unittest
import json
import typing
class ApiTester(unittest.TestCase):
    """
    A base Class for testing API modules
    """

    def config(self) -> typing.Any:
        return json.loads("{\"matrix_options\":{\"chain_length\":1,\"parallel\":1,\"brightness\":50,\"oled_slowdown\":3,\"fail_on_error\":false},\"time\":{\"run\":true,\"color\":[255,255,255],\"time_format\":\"null\",\"timezone\":\"null\"},\"weather\":{\"run\":true,\"api\":\"openweather\",\"api_key\":\"null\",\"current_location\":true,\"city\":\"null\",\"weather_format\":\"imperial\"},\"stock\":{\"run\":true,\"api\":\"finnhub\",\"api_key\":\"null\",\"symbol\":\"fb\"},\"sport\":{\"run\":true,\"api\":\"api-sports\",\"api_key\":\"null\",\"sport\":\"basketball\",\"team_logo\":{\"name\":\"Dallas Mavericks\",\"sportsdb_leagueid\":4387,\"url\":\"https://www.thesportsdb.com/images/media/team/badge/yqrxrs1420568796.png\",\"sport\":\"basketball\",\"shorthand\":\"DAL\",\"apisportsid\":138,\"sportsdbid\":134875,\"sportsipyid\":0}}}")