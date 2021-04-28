#!/usr/bin/env python3

import asyncio
from lib.run import Runner
from lib.weather import WeatherApi
from lib.stocks import StockApi

"""
This file for now is for testing the library
and the calls to apis 
"""

class Main():
    def __init__(self) -> None:
        self.weather = WeatherApi()
        self.stock = StockApi()

    def main_run(self):
        return asyncio.run(self.stock.run("fb"))
if __name__ == "__main__":
    main = Main()

    print(main.main_run())