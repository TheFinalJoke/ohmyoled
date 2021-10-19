from lib.sports.sportbase import SportBase
from lib.sports.apisports.baseball.baseball import Baseball
from lib.sports.apisports.basketball.basketball import Basketball
from lib.sports.apisports.hockey.hockey import Hockey
from lib.sports.apisports.football.football import Football
import json
import sys
import os

class ApiSports(SportBase):
    def __init__(self, config):
        self.config = config
        self.sport = self.config['sport']
        try:
            if "sport_token" in self.config['basic']:
                self.token = self.config['basic'].get('sport_token')
            else:
                self.token = os.environ['SPORTTOKEN']
        except KeyError:
            self.logger.critical("No Sport Token")
            sys.exit("No Sport Token")
        self.headers = {'x-apisports-key': 'ebb2c44c416b9a9a0b538e2d73c7dbe6'}

    async def run_api_sports(self):
        sport_data = {"Sport": {}}
        if 'football' == self.sport.get('sport').lower():
            self.logger.debug("Running football data")
            football = Football(self.token, self.config['sport'], self.headers)
            football_return = await football.run()
            sport_data['Sport'].update({'football': football_return})
        elif 'baseball' == self.sport.get('sport').lower():
            self.logger.debug('Running baseball data')
            baseball = Baseball(self.token, self.config['sport'], self.headers)
            sport_data = await baseball.run()
        elif 'basketball' == self.sport.get('sport').lower():
            self.logger.debug('Got basketball in config')
            basketball = Basketball(self.token, self.config['sport'], self.headers)
            basketball_return = await basketball.run()
            sport_data['Sport'].update({'basketball': basketball_return})
        elif 'hockey' == self.sport.get('sport').lower():
            self.logger.debug('Got Hockey from Config')
            hockey = Hockey(self.token, self.config['sport'], self.headers)
            sport_data = await hockey.run()
        return sport_data