#!/usr/bin/env python3

import typing
from ohmyoled.lib.run import Runner, Caller
from ohmyoled.lib.stock.stockquote import SQuote
from dataclasses import dataclass
import os
import asyncio
import json
import sys


class StockException(Exception):
    """
    Main Stock Exception
    """

    pass


@dataclass(repr=True, init=True)
class StockErrorResult(Caller):
    error: bool = False
    msg: str = ""


class StockApi(Runner):
    """
    Mirrored of WeatherApi Object
    Used to Build and Poll Stock Data from
    Finnhub.IO
    """

    def __init__(self, config):
        super().__init__(config)
        self.stock = self.config["stock"]
        try:
            if "null" == self.stock.get("api_key"):
                raise KeyError
            elif "null" != self.stock.get("api_key"):
                self.token = self.stock.get("api_key")
            else:
                self.token = os.environ["STOCKTOKEN"]
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
        base = "https://finnhub.io/api/v1/"
        url = base + f"quote?symbol={company.upper()}&token={self.token}"
        return await self.get_data(url)

    async def run(self) -> typing.Dict:
        """
        Runs Stock to bring back the typing.dictionary
        """
        try:
            self.logger.info("Getting Stock")
            stock_data = {"Stock": {}}
            self.logger.debug("Getting Quote data")
            quote = SQuote(self.token, self.config["stock"])
            stock_api_return = await quote.run()
            stock_data["Stock"].update({"Quote": stock_api_return})
            return stock_data
        except Exception as e:
            raise StockException(e)

    def run_with_asyncio(self):
        """
        Runs Stock to bring back the typing.dictionary
        """
        self.logger.info("Getting Stock")
        stock_data = {"Stock": {}}
        self.logger.debug("Config has historical data for stocks")
        self.logger.debug("Getting Quote data")
        quote = SQuote(self.token, self.config["stock"])
        stock_api_return = asyncio.run(quote.run())
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
        self.stock_data = self.stock_data.get("Stock")
        self.quote = self.stock_data.get("Quote")
        self._open_price = self.quote["o"]
        self._current_price = self.quote["c"]
        self._highest_price = self.quote["h"]
        self._lowest_price = self.quote["l"]
        self._previous_close = self.quote["pc"]
        self._symbol = self.quote["symbol"]
        self._description = self.quote["description"]
        self._name = self._description["name"]

    def __repr__(self) -> str:
        # datetimes = [dates.strftime("%Y-%m-%d %H:%M:%S") for dates in self._hist_timestamps]
        attrs = [
            f"symbol={self._symbol}",
            f"open_price={self._open_price}",
            f"current_price={self._current_price}",
            f"highest_price={self._highest_price}",
            f"lowest_price={self._lowest_price}",
            f"previous_close_price={self._previous_close}",
            f"description={json.dumps(self._description, indent=2)}",
            f"name={self._name}",
        ]
        joined_attrs = ",\n".join(attrs)
        return f"Stock(\n{joined_attrs})"

    @property
    def get_name(self):
        return self._name

    @property
    def get_description(self):
        return self._description

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
