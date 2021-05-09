#!/usr/bin/env python3
from lib.run import Runner
class SQuote(Runner):
    def __init__(self, token, config):
        # Only Pass in config pertains to stock
        self.config = config
        self.token = token

    def url_builder(self, symbol):
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={symbol.upper()}&token={self.token}'
        return url
    
    async def run(self):
        symbol = self.config.get('symbol')
        api_data = self.get_data(self.url_builder(symbol=symbol))
        return api_data
        