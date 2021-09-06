#!/usr/bin/env python3

from typing import Dict
from lib.run import Runner, Caller
from lib.stock.stockquote import SQuote
from lib.stock.historical_stock import HistoricalStock
from datetime import date, datetime
import os 
import sys

class StockApi(Runner):
    """
    Mirrored of WeatherApi Object
    Used to Build and Poll Stock Data from
    Finnhub.IO
    """
    def __init__(self, config):
        super().__init__(config)
        self.stock = self.config['stock']
        try:
            if "stock_api_token" in self.config['basic']:
                self.token = self.config['basic'].get('stock_api_token')
            else:
                self.token = os.environ['STOCKTOKEN']
        except KeyError:
            self.logger.critical("No Stock Token")
            sys.exit("No Stock Token")
    
    def parse_args(self):
        """
        This Method does nothing
        """
        return super().parse_args()
    
    async def symbol_lookup(self, company):
        """
        Current Config only Takes in symbols
        """
        self.logger.debug(f"Looking up Symbol for company {company}")
        base = 'https://finnhub.io/api/v1/'
        url = base + f'quote?symbol={company.upper()}&token={self.token}'
        return await self.get_data(url)

    async def run(self) -> Dict:
        """
        Runs Stock to bring back the dictionary
        """
        self.logger.info("Getting Stock")
        stock_data = {"Stock": {}}
        if "historical" in self.stock and self.stock.getboolean("historical"):
            self.logger.debug("Config has historical data for stocks")
            hist_stock = HistoricalStock(self.token, self.config['stock'])
            stock_data["Stock"].update({"Historical": await hist_stock.run()})
        if "quote" in self.stock and self.stock.getboolean("quote"):
            self.logger.debug("Getting Quote data")
            quote = SQuote(self.token, self.config['stock'])
            stock_api_return = await quote.run()
            stock_data["Stock"].update({"Quote": stock_api_return})
        return stock_data
    
class Stock(Caller):
    """
    Reflecting of current stock polling
    Any Updates will need to update this object
    or create new object and pulled here
    """
    def __init__(self, stock_data):
        self.stock_data = stock_data
        self.stock_data = self.stock_data.get('Stock')
        if 'Quote' in self.stock_data:
            self.quote = self.stock_data.get('Quote')
            self._open_price = self.quote['o']
            self._current_price = self.quote['c']
            self._highest_price = self.quote['h']
            self._lowest_price = self.quote['l']
            self._previous_close = self.quote['pc']
            self._symbol = self.quote['symbol']
        if 'Historical' in self.stock_data:
            self.historical = self.stock_data.get('Historical')
            self._hist_close_prices = self.historical['c']
            self._hist_high_prices = self.historical['h']
            self._hist_low_prices = self.historical['l']
            self._hist_opening_prices = self.historical['o']
            self._hist_volume = self.historical['v']
            self._hist_timestamps = [datetime.fromtimestamp(time) for time in self.historical['t']]
            self._symbol = self.historical['symbol']
    
    @property
    def get_symbol(self):
        return self._symbol
    @property
    def get_quote_open_price(self):
        return self._open_price
    @property 
    def get_quote_current_price(self):
        return self._current_price
    @property
    def get_quote_highest_price(self):
        return self._highest_price
    @property
    def get_quote_lowest_price(self):
        return self._lowest_price
    @property
    def get_quote_previous_close(self):
        return self._previous_close
    @property
    def get_hist_close_prices(self):
        return self._hist_close_prices
    @property
    def get_hist_low_prices(self):
        return self._hist_low_prices
    @property
    def get_hist_opening_prices(self):
        return self._hist_opening_prices
    @property
    def get_hist_volume(self):
        return self._hist_volume
    @property
    def get_hist_timestamps(self):
        return self._hist_timestamps
