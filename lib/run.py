#!/usr/bin/env python3

from abc import abstractmethod
import requests

class RunnerABS():
    def __init__(self):
        pass
    @abstractmethod
    async def run(self): pass

    @abstractmethod
    def parse_args(self): pass

    @abstractmethod
    def url_builder(self): pass

class Runner(RunnerABS):

    def __init__(self, config):
        super().__init__()
        self.config = config

    def get_data(self, url):
        data = requests.get(url)
        return data