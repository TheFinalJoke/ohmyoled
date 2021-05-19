#!/usr/bin/env python3

from abc import abstractmethod
from typing import Dict
import requests
import logging
import aiohttp

stream_formatter = logging.Formatter(
    "%(levelname)s:%(asctime)s:%(module)s:%(message)s"
)
sh = logging.StreamHandler()
filehandler = logging.FileHandler("/home/nickshorter/ohmyoled.log","a")
sh.setFormatter(stream_formatter)
filehandler.setFormatter(stream_formatter)
logger = logging.getLogger(__name__)
logger.addHandler(sh)
logger.addHandler(filehandler)
logger.setLevel(logging.DEBUG)

class RunnerABS():
    def __init__(self):
        pass
    @abstractmethod
    async def run(self): pass

    @abstractmethod
    def parse_args(self): pass

    @abstractmethod
    def url_builder(self): pass

class Caller(object):
    logger = logging.getLogger(__name__)
    logger.addHandler(sh)
    logger.addHandler(filehandler)
    logger.setLevel(logging.DEBUG)
    def __init__(self) -> None:
        super().__init__()
        self.caller_logger = logger
        
class Runner(RunnerABS):
    logger = logging.getLogger(__name__)
    logger.addHandler(sh)
    logger.addHandler(filehandler)
    logger.setLevel(logging.DEBUG)
    def __init__(self, config):
        super().__init__()
        self.config = config
        self.runner_logger = logger
    async def get_data(self, url, headers: Dict[str, str]={}):
        self.logger.debug(f'Getting data with URL {url}')
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=headers) as resp:
                data = await resp.json()
        return data