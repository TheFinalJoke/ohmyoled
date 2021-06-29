#!/usr/bin/env python3
from lib.run import Runner
from typing import Dict
import datetime

class HistoricalStock(Runner):
    """
    Get Historical Stock in a certain timeline
    """
    def __init__(self, token, config):
        self.config = config
        self.token = token
    
    def url_builder(self):
        """
        Builds url with timeline
        """
        if not self.config.get('days_ago'):
            self.config['days_ago'] = "7"
        days_ago = self.config.getint('days_ago')
        days_ago_unix = (datetime.datetime.now() - datetime.timedelta(days_ago)).timestamp()
        now = datetime.datetime.now().timestamp()
        symbol = self.config.get('symbol')
        base = 'https://finnhub.io/api/v1/'
        return base + f"/stock/candle?symbol={symbol.upper()}&resolution=D&from={int(days_ago_unix)}&to={int(now)}&token={self.token}"
    
    async def run(self) -> Dict:
        url = self.url_builder()
        api_data = await self.get_data(url)
        api_data['symbol'] = self.config.get('symbol')
        return api_data