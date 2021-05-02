#!/usr/bin/env python3

from typing import get_args
from lib.run import Runner
import os 
import sys

class StockApi(Runner):
    def __init__(self, config):
        super().__init__(config)
        self.stock = self.config['stock']
        try:
            if "stock_api_token" in self.config['basic']:
                self.token = self.config['basic'].get('stock_api_token')
            else:
                self.token = os.environ['STOCKTOKEN']
        except KeyError:
            sys.exit("No Stock Token")
    def parse_args(self):
        return super().parse_args()
    
    def url_builder(self, symbol):
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={symbol.upper()}&token={self.token}'
        return url
    def symbol_lookup(self, company):
        """
        Current Config only Takes in symbols
        """
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={company.upper()}&token={self.token}'
        return self.runner.get_data(url)

    async def run(self):
        symbol = self.stock.get('symbol')
        api_data = self.get_data(self.url_builder(symbol=symbol))
        return api_data.json()
