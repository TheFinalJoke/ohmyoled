#!/usr/bin/env python3

import asyncio
from asyncio.runners import run
import configparser
from lib.stocks import StockApi

from lib.run import Runner
from lib.weather import WeatherApi
from lib.stocks import StockApi
from lib.stockquote import SQuote
TESTING = True
"""
This file for now is for testing the library
and the calls to apis 
"""

class Main():
    def __init__(self, config) -> None:
        self.config = config
    def parse_config_file(self):
        module = {}
        for section in self.config.sections():
            if section == 'basic':
                continue
            module.update({section: self.config[section].getboolean('Run')})
        return module
    def get_modules_to_run(self):
        api_modules = []
        parsed = self.parse_config_file()
        for section, runtime in parsed.items():
            if runtime:
                if section == 'weather':
                    api_modules.append(WeatherApi(self.config))
                if section == 'stock':
                    api_modules.append(StockApi(self.config))
        return api_modules
    async def main_run(self):
        tasks = []
        for task in self.get_modules_to_run():
            tasks.append(asyncio.create_task(task.run()))
        completed = await asyncio.gather(*tasks, return_exceptions=True)
        for task in completed:
            print(task)
        

if __name__ == "__main__":
    config = configparser.ConfigParser()
    if TESTING:
        config.read('lib/config/ohmyoled.conf')
    else:
        config.read('/etc/ohmyoled/ohmyoled.conf')
    main = Main(config)
    asyncio.run(main.main_run())