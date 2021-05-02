#!/usr/bin/env python3

from typing import get_args
from lib.run import Runner

FINN_TOKEN = 'c091niv48v6tm13rlep0'

class StockApi(Runner):
    def __init__(self, config):
        super().__init__(config)
        self.stock = self.config['stock']
    def parse_args(self):
        return super().parse_args()
    
    def url_builder(self, symbol):
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={symbol.upper()}&token={FINN_TOKEN}'
        return url
    def symbol_lookup(self, company):
        """
        Current Config only Takes in symbols
        """
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={company.upper()}&token={FINN_TOKEN}'
        return self.runner.get_data(url)

    async def run(self):
        ####### Need to get symbol elf.config['stock']['symbol'] ##########
        symbol = self.stock.get('symbol')
        api_data = self.get_data(self.url_builder(symbol=symbol))
        return api_data.json()
