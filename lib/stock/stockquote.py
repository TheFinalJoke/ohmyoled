#!/usr/bin/env python3
from lib.run import Runner
from typing import Dict
class SQuote(Runner):
    """
    Module of Stock 
    Get Current Quote data from Finnhub.io
    """
    def __init__(self, token, config):
        # Only Pass in config pertains to stock
        self.config = config
        self.token = token

    def url_builder(self, symbol):
        """
        Builds Url to get quote
        """
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={symbol.upper()}&token={self.token}'
        return url
    
    async def run(self) -> Dict:
        """
        returns current stock Quotes and Price
        """
        symbol = self.config.get('symbol')
        api_data = await self.get_data(self.url_builder(symbol=symbol))
        api_data['symbol'] = symbol
        return api_data
        