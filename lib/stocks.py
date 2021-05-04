#!/usr/bin/env python3

from typing import get_args
from lib.run import Runner
from lib.stockquote import SQuote
from datetime import date, datetime
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
    
    def symbol_lookup(self, company):
        """
        Current Config only Takes in symbols
        """
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={company.upper()}&token={self.token}'
        return self.get_data(url)

    async def run(self):
        stock_data = {}
        base = 'https://finnhub.io/api/v1/'
        symbol = self.stock["symbol"]
        if "historical" in self.stock and self.stock.getboolean("historical"):
            now = int(datetime.now().timestamp())
            week_ago = now - 604800
            url = base + f"/stock/candle?symbol={symbol}&resolution=D&from={week_ago}&to={now}&token={self.token}"
            stock_data['Historical'] = self.get_data(url)
        elif "quote" in self.stock and self.stock.getboolean("quote"):
            url = base + f'quote?symbol={symbol}&token={self.token}'
            stock_data['Quote'] = self.get_data(url)
        return stock_data