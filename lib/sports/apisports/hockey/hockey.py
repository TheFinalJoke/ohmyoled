from lib.run import Runner
from datetime import datetime
from lib.sports.apisports.result import SportApiResult

class Hockey(Runner):
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
        self.headers.update({'x-apisports-host': 'v1.hockey.api-sports.io'})
        urls = {}
        base = "https://v1.hockey.api-sports.io/"
        if 'standings' in args:
            urls.update({'standings': base + f'standings?league=57&season=2021'})
        if 'next_game' in args:
            urls.update({'next_game': base + f"games?team={self.config.getint('team_id')}&league=57&season=2020&timezone=America/Chicago"})
        return urls
    
    async def run(self) -> SportApiResult:
        self.logger.info('Running Hockey API')
        parsed = self.parse_args()
        api_data = {}
        for section, url in self.url_builder(parsed).items():
            api_data.update({section: await self.get_data(url, self.headers)})
        api_data['sport'] = 'hockey'
        return SportApiResult(api_data)