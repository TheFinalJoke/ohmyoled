from lib.sports.sportbase import SportBase, SportResultBase
from lib.sports.sportsipy.hockey.hockey import HockeySportsipy

class SportsipyAPI(SportBase):
    def __init__(self, config):
        self.config = config
        self.sport = self.config['sport']

    async def run_api_sportsipy(self):
        if 'hockey' == self.sport.get('sport').lower():
            self.logger.debug("Running hockey sportsipy")
            hockey = HockeySportsipy(self.config)
            return await hockey.run()