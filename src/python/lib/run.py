#!/usr/bin/env python3

from abc import abstractmethod
from typing import Dict
import requests
import os
import logging
import aiohttp
from requests.models import Response

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

class RunnerABS():
    """ 
    Abstract Class to define polling modules
    """
    def __init__(self):
        pass
    @abstractmethod
    async def run(self): pass

    @abstractmethod
    def parse_args(self): pass

    @abstractmethod
    def url_builder(self): pass

class Caller(object):
    """
    Abstract Class to create base Objects
    of current datasets 
    """
    logger = logging.getLogger(__name__)
    logger.addHandler(sh)
    logger.addHandler(filehandler)
    def __init__(self) -> None:
        super().__init__()
        self.caller_logger = logger
        self.caller_logger.debug(f"Caller logger is set to {self.logger.getEffectiveLevel()}")
        
class Runner(RunnerABS):
    """
    Base Class for all poll Api and Modules 
    """
    logger = logging.getLogger(__name__)
    logger.addHandler(sh)
    logger.addHandler(filehandler)
    def __init__(self, config):
        super().__init__()
        self.config = config
        self.runner_logger = logger
        if int(os.getenv("DEV")) != 1:
            self.logger.setLevel(logging.INFO)
        else:
            self.logger.setLevel(logging.DEBUG)
        self.logger.debug(f"Runner logger is set to {self.logger.getEffectiveLevel()}")

    def run_non_async_request(self, url) -> Response:
        response = requests.get(url)
        return response

    async def get_data(self, url, headers: Dict[str, str]={}) -> Dict:
        self.logger.debug(f'Getting data with URL {url}')
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=headers) as resp:
                data = await resp.json()
        return data

    async def get_non_json_data(self, url, headers: Dict[str, str]={}) -> Dict:
        self.logger.debug(f'Getting data with URL {url}')
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=headers) as resp:
                data = await resp.text()
        return data

    async def get_img(self, url, headers: Dict[str, str]={}) -> Dict:
        self.logger.debug(f'Getting data with URL {url}')
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=headers) as resp:
                data = await resp
        return data