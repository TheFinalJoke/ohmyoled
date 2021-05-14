#!/usr/bin/env python3

from lib.run import Runner

class Crypto(Runner):
    def __init__(self, token, config):
        self.config = config
        self.token = token
    
    def url_builder(self):
        return super().url_builder()
    
    async def run(self):
        return 