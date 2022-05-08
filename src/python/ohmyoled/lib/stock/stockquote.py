#!/usr/bin/env python3
from ohmyoled.lib.run import Runner
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
    def description_url(self, symbol):
        base = 'https://finnhub.io/api/v1/'
        url = base + f'stock/profile2?symbol={symbol.upper()}&token={self.token}'
        return url
    async def run(self) -> Dict:
        """
        returns current stock Quotes and Price
        """
        self.logger.info("Running Stock Api")
        symbol = self.config.get('symbol')
        api_data = await self.get_data(self.url_builder(symbol=symbol))
        api_data['symbol'] = symbol
        api_data['description'] = await self.get_data(self.description_url(symbol))
        return api_data
        