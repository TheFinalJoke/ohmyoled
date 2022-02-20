#!/usr/bin/env python3

import asyncio
import time
import argparse
import os
import sys
from rgbmatrix import (
    RGBMatrixOptions, 
    RGBMatrix
)
from ohmyoled.lib.upgrade.upgrade import Upgrader
from ohmyoled.lib.weather.normal import WeatherApi
from ohmyoled.matrix.stock.stockmatrix import StockMatrix
from ohmyoled.lib.stock.stocks import StockApi
from ohmyoled.lib.sports.sports import SportApi
from ohmyoled.matrix.time import TimeMatrix
from ohmyoled.matrix.weathermatrix import WeatherMatrix
from ohmyoled.matrix.sport.sportmatrix import SportMatrix
import traceback
import logging
import json


stream_formatter = logging.Formatter(
    "%(asctime)s:%(module)s: %(message)s"
)
sh = logging.StreamHandler()
filehandler = logging.FileHandler("/var/log/ohmyoled/ohmyoled.log","a")
sh.setFormatter(stream_formatter)
filehandler.setFormatter(stream_formatter)
logger = logging.getLogger(__name__)
logger.addHandler(sh)
logger.addHandler(filehandler)

def __version__():
    return "2.0.0"
class OledExecption(Exception):
    pass
class Main():

    def version(cls):
        return cls.version

    def __init__(self, config) -> None:
        self.config = config
        self.logger = logger
        if int(os.getenv("DEV")) != 1:
            self.logger.setLevel(logging.INFO)
        else:
            self.logger.setLevel(logging.DEBUG)
        self.poll = None
        self.logger.debug(f"Logger is set to {self.logger.getEffectiveLevel()}")
    def parse_config_file(self):
        module = {}
        self.logger.info("Parsing config file")
        for section in self.config.keys():
            if section == 'matrix_options':
                continue
            module.update({section: self.config[section].get('run')})
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
        options = self.config.get("matrix_options", "There is no matrix options")
        rgboptions = RGBMatrixOptions()
        rgboptions.cols = 64
        rgboptions.rows = 32
        rgboptions.chain_length = options.get("chain_length")
        rgboptions.parallel = options.get("parallel")
        rgboptions.gpio_slowdown = options.get('oled_slowdown')
        rgboptions.brightness = options.get('brightness')
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
    
    async def run_matrix_worker(self, matrix, polled_data):
        self.logger.debug("Starting Worker")
        # Need to make sure these need to be coroutines
        matrix.render(polled_data)
    
    async def poll_api_worker(self, matrix):
        try:
            polled_data = await matrix.poll_api()
            return polled_data
        except:
            self.logger.error("Error in the Poll_API Worker")


    async def main_run(self, loop):
        try:
            self.logger.info("Starting OhMyOled")
            matrix = RGBMatrix(options=self.poll_rgbmatrix())
            self.logger.debug("Built Options for RGBMatrix")
            # Make the matrixes to a Queue
            matrixes = await self.init_matrix(matrix)
            self.logger.info("Starting Matrixes...")
            first_poll = True
            while True:
                for index, matrix in enumerate(matrixes):
                    matrix_start_time = time.perf_counter()
                    if first_poll:
                        self.poll = await matrix.poll_api()
                        first_poll = False
                    if index + 1 >= len(matrixes):
                        tasks = [asyncio.create_task(matrix.render(self.poll, loop)), asyncio.create_task(matrixes[0].poll_api())]
                    else:
                        tasks = [asyncio.create_task(matrix.render(self.poll, loop)), asyncio.create_task(matrixes[index+1].poll_api())]
                    _, self.poll = await asyncio.gather(*tasks)
                    matrix_finish_time = time.perf_counter()
                    logger.info(f"{matrix} rendered for {matrix_finish_time - matrix_start_time:0.4f}s")
        except Exception as E:
                logger.error(E)
                traceback.print_exc()
                loop.stop()
                
if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run an OhMyOled Matix")
    parser.add_argument(
        "-u",
        "--upgrade",
        action="store_true")
    parser.add_argument(
        '--version',
        action="store_true"
    )
    # This Should pass args to ohmyoled parser for determining json/configparser
    # Then Return back json and the program should ingest json
    # But for now we will parse it here and pass args it if upgrades
    args = parser.parse_args()
    if args.upgrade:
        upgrader = Upgrader(args, version=__version__(), logger=logger)
        asyncio.run(upgrader.run_upgrade())
        sys.exit(0)
    elif args.version:
        print(__version__())
        sys.exit(0)
    with open("/etc/ohmyoled/ohmyoled.json") as file:
        j_object = json.load(file)
        
    logger.info("Pulling configuration /etc/ohmyoled/ohmyoled.json")
    main = Main(j_object)

    loop = asyncio.get_event_loop()
    try:
        loop.create_task(main.main_run(loop))
        loop.run_forever()
    except KeyboardInterrupt:
        logger.critical("Key Interrupt")
    except Exception as e:
        logger.critical(e)
    finally:
        loop.stop()
