#!/usr/bin/env python3

import asyncio
import configparser
import os
from rgbmatrix import (
    RGBMatrixOptions, 
    RGBMatrix
)
from lib.weather.weather import WeatherApi
from matrix.stock.stockmatrix import StockMatrix
from matrix.stock.historicalstockmatrix import HistoricalStockMatrix
from lib.stock.stocks import StockApi, Stock
from lib.sports.sports import SportApi, Sport
from matrix.matrix import Matrix
from matrix.time import TimeMatrix
from matrix.weathermatrix import WeatherMatrix
from matrix.sport.sportmatrix import SportMatrix
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

class Main():
    def __init__(self, config) -> None:
        self.config = config
        self.logger = logger
        if self.config['basic'].getboolean('testing'):
            self.logger.setLevel(self.config['basic'].getint('loglevel'))
        else:
            self.logger.setLevel(10)
        self.logger.debug(f"Logger is set to {self.logger.getEffectiveLevel()}")
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
                if section == 'time':
                    self.logger.debug("Time midule selected from config")
                    api_modules.update({'time': " "})
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
        verified_modules = []
        modules = self.get_modules_to_run()
        if 'time' in modules:
            self.logger.debug("Initialized Time")
            verified_modules.append(TimeMatrix(matrix, self.config['time']))
        if 'weather' in modules:
            self.logger.debug("Initialized Weather")
            verified_modules.append(WeatherMatrix(matrix, modules['weather'], logger))
        if 'stock' in modules:
            self.logger.debug("Initialized Stock")
            verified_modules.append(StockMatrix(matrix, modules['stock'], logger))
            # This Might not be able to be supported
            # verified_modules.append(HistoricalStockMatrix(matrix, modules['stock'], logger))
        if 'sport' in modules:
            self.logger.debug("Initialized Sports")
            verified_modules.append(SportMatrix(matrix, modules['sport'], logger))
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
    logger.info("Pulling configuration /etc/ohmyoled/ohmyoled.conf")
    config.read('/etc/ohmyoled/ohmyoled.conf')
    main = Main(config)
    loop = asyncio.get_event_loop()
    try:
        loop.create_task(main.main_run())
        loop.run_forever()
    except KeyboardInterrupt:
        logger.critical("Key Interrupt")
    except Exception as e:
        logger.critical(e)
    finally:
        loop.stop()
