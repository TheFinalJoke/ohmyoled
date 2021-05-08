#!/usr/bin/env python3

import asyncio
from asyncio.runners import run
import configparser
from requests import api
import termplotlib as tpl
from lib.stocks import StockApi

from lib.run import Runner
from lib.weather import WeatherApi, Weather
from lib.stocks import StockApi, Stock
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
        api_modules = {}
        parsed = self.parse_config_file()
        for section, runtime in parsed.items():
            if runtime:
                if section == 'weather':
                    api_modules.update({'weather': WeatherApi(self.config)})
                if section == 'stock':
                    api_modules.update({'stock': StockApi(self.config)})
        return api_modules

    async def poll_apis(self):
        modules = self.get_modules_to_run()
        for task in modules:
            modules[task] = asyncio.create_task(modules[task].run())
        await asyncio.gather(*modules.values())
        return modules

    def build_obj(self, polled_apis):
        #TODO(thefinaljoke): When Convert to Python3.10 Use match Statment
        if 'weather' in polled_apis:
            polled_apis['weather'] = Weather(polled_apis['weather'].result())
        if 'stock' in polled_apis:
            polled_apis['stock'] = Stock(polled_apis['stock'].result())
        return polled_apis
    async def show_stock(self, api):
        x = [1,2,3,4,5]
        y = [1,2,3,4,5]
        fig = tpl.figure()
        fig.plot(x, y, label="line", width=50, height=15)
        fig.show()

    async def main_run(self):
        #while True:
        apis = await self.poll_apis()
        objs = self.build_obj(apis)
        breakpoint()
            #asyncio.ensure_future(self.show_stock(apis))
        #print(apis)
            #await asyncio.sleep(5)
if __name__ == "__main__":
    config = configparser.ConfigParser()
    if TESTING:
        config.read('lib/config/ohmyoled.conf')
    else:
        config.read('/etc/ohmyoled/ohmyoled.conf')
    main = Main(config)
    loop = asyncio.get_event_loop()
    try:
        loop.create_task(main.main_run())
        loop.run_forever()
    except KeyboardInterrupt:
        print("Key Interrupt")
    finally:
        loop.stop()