#!/usr/bin/env python3

from abc import abstractmethod
import requests

class RunnerABS():
    def __init__(self):
        pass 
    @abstractmethod
    def run(self): pass

class Runner(RunnerABS):

    def __init__(self, *args, **kwargs):
        super().__init__()

    def run(self, url):
        return requests.get(url)