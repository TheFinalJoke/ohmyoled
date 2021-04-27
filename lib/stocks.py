#!/usr/bin/env python3

from typing import get_args
from lib.run import Runner, RunnerABS

FINN_TOKEN = 'c091niv48v6tm13rlep0'

class StockApi(RunnerABS):
    def __init__(self, symbol=None):
        super().__init__()
        self.symbol = symbol
        self.runner = Runner()

    def symbol_lookup(self, company):
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={company.upper()}&token={FINN_TOKEN}'
        return self.runner.get_data(url)

    async def run(self, symbol=None, company=None):
        base = 'https://finnhub.io/api/v1/'
        if company:
            symbol = self.symbol_lookup(company)
        if self.symbol:
            symbol = self.symbol
        url = base + f'quote?symbol={symbol.upper()}&token={FINN_TOKEN}'
        return self.runner.get_data(url).json()
