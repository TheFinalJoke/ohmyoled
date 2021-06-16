from lib.run import Runner
from datetime import datetime

class Baseball(Runner):
    def __init__(self, token, config, headers):
        self.config = config
        self.token = token
        self.headers = headers
        self.year = datetime.now().year
    
    def parse_args(self):
        try:
            if self.config['type'] != '' or not self.config['type']:
                args = (self.config['type'])
            else:
                args = ('standings', 'next_game')
            return args
        except KeyError:
            args = ('standings', 'next_game')
        return args

    def url_builder(self, args):
        self.headers.update({'x-apisports-host': 'v1.baseball.api-sports.io'})
        urls = {}
        base = "https://v1.baseball.api-sports.io/"
        if 'standings' in args:
            urls.update({'standings': base + f'standings?league=1&season=2021'})
        if 'next_game' in args:
            urls.update({'next_game': base + f"games?team={self.config.getint('team_id')}&league=1&season=2021&timezone=America/Chicago"})
        return urls
    
    async def run(self):
        self.logger.info('Running baseball API')
        parsed = self.parse_args()
        api_data = {}
        for section, url in self.url_builder(parsed).items():
            api_data.update({section: await self.get_data(url, self.headers)})
        return api_data