#!/usr/bin/env python3

import asyncio
import configparser
from rgbmatrix import (
    RGBMatrixOptions, 
    RGBMatrix
)
from lib.weather.weather import WeatherApi, Weather
from matrix.stock.stockmatrix import StockMatrix
from lib.stock.stocks import StockApi, Stock
from lib.sports.sports import SportApi, Sport
from matrix.time import TimeMatrix
from matrix.weathermatrix import WeatherMatrix

TESTING = True
"""
This file for now is for testing the library
and the calls to apis 
"""
from abc import abstractmethod
import requests
import logging

stream_formatter = logging.Formatter(
    "%(levelname)s:%(asctime)s:%(module)s:%(message)s"
)
sh = logging.StreamHandler()
filehandler = logging.FileHandler("/var/log/ohmyoled.log","a")
sh.setFormatter(stream_formatter)
filehandler.setFormatter(stream_formatter)
logger = logging.getLogger(__name__)
logger.addHandler(sh)
logger.addHandler(filehandler)
logger.setLevel(logging.DEBUG)

class Main():
    def __init__(self, config) -> None:
        self.config = config
        self.logger = logger
    def parse_config_file(self):
        module = {}
        self.logger.info("Parsing config file")
        for section in self.config.sections():
            if section == 'basic':
                continue
            module.update({section: self.config[section].getboolean('Run')})
        return module
    def get_modules_to_run(self):
        api_modules = {}
        parsed = self.parse_config_file()
        self.logger.info("Getting Modules")
        for section, runtime in parsed.items():
            if runtime:
                if section == 'weather':
                    self.logger.debug("Weather Module Selected From Config")
                    api_modules.update({'weather': WeatherApi(self.config)})
                if section == 'stock':
                    self.logger.debug("Stock module was Selected from config")
                    api_modules.update({'stock': StockApi(self.config)})
                if section == 'sport':
                    self.logger.debug("Sport Module was passed into config")
                    api_modules.update({"sport": SportApi(self.config)})
        return api_modules

    async def poll_apis(self):
        modules = self.get_modules_to_run()
        self.logger.info('Creating asyncio tasks for polling')
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
        if 'sport' in polled_apis:
            polled_apis['sport'] = Sport(polled_apis['sport'].result())
        return polled_apis
    def poll_rgbmatrix(self):
        options = self.config['matrix']
        rgboptions = RGBMatrixOptions()
        rgboptions.cols = 64
        rgboptions.rows = 32
        rgboptions.chain_length = options.getint('parallel')
        rgboptions.parallel = options.getint('chain_length')
        rgboptions.gpio_slowdown = options.getint('oled_slowdown')
        rgboptions.brightness = options.getint('brightness')
        rgboptions.hardware_mapping = 'adafruit-hat'
        return rgboptions

    async def init_matrix(self, matrix):
        verified_modules = [TimeMatrix(matrix)]
        modules = self.get_modules_to_run()
        if 'weather' in modules:
            self.logger.debug("Initialized Weather")
            verified_modules.append(WeatherMatrix(matrix, modules['weather'], logger))
        if 'stock' in modules:
            self.logger.debug("Initialized Stock")
            verified_modules.append(StockMatrix(matrix, modules['stock'], logger))
        if 'sport' in modules:
            pass
        self.logger.info("Initalized matrixes")
        return verified_modules

    async def main_run(self):
        self.logger.info("Starting OhMyOled")
        matrix = RGBMatrix(options=self.poll_rgbmatrix())
        self.logger.debug("Built Options for RGBMatrix")
        matrixes = await self.init_matrix(matrix)
        self.logger.info("Starting Matrixes...")
        while True:
            for matrix in matrixes:
                poll = await matrix.poll_api()
                matrix.render(poll)
                
if __name__ == "__main__":
    config = configparser.ConfigParser()
    if TESTING:
        logger.info("Testing is ENABLED")
        logger.info("Using local config lib/config/ohmyoled.conf")
        config.read('lib/config/ohmyoled.conf')
    else:
        logger.info("Pulling configuration /etc/ohmyoled/ohmyoled.conf")
        config.read('/etc/ohmyoled/ohmyoled.conf')
    main = Main(config)
    loop = asyncio.get_event_loop()
    try:
        loop.create_task(main.main_run())
        loop.run_forever()
    except KeyboardInterrupt:
        logger.critical("Key Interrupt")
    finally:
        loop.stop()
