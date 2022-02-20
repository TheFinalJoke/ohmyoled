from ohmyoled.lib.sports.sportbase import SportBase, SportResultBase
from ohmyoled.lib.sports.sportsipy.hockey.hockey import HockeySportsipy
from ohmyoled.lib.sports.sportsipy.baseball.baseball import BaseballSportsipy
from ohmyoled.lib.sports.sportsipy.basketball.basketball import BasketballSportsipy
from ohmyoled.lib.sports.sportsipy.football.football import FootballSportsipy

class SportsipyAPI(SportBase):
    def __init__(self, config):
        self.config = config
        self.sport = self.config['sport']

    async def run_api_sportsipy(self):
        if 'hockey' == self.sport.get('sport').lower():
            self.logger.debug("Running hockey sportsipy")
            hockey = HockeySportsipy(self.config)
            return await hockey.run()
        elif 'baseball' == self.sport.get('sport').lower():
            self.logger.debug("Running Baseball sportsipy")
            baseball = BaseballSportsipy(self.config)
            return await baseball.run()
        elif 'basketball' == self.sport.get('sport').lower():
            self.logger.debug("Running basketball sportsipy")
            basketball = BasketballSportsipy(self.config)
            return await basketball.run()
        elif 'football' == self.sport.get('sport').lower():
            self.logger.debug("Running football sportsipy")
            football = FootballSportsipy(self.config)
            return await football.run()