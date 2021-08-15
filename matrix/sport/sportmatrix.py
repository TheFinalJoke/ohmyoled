#!/usr/bin/env python3

import asyncio
import time
from lib.sports.sports import Sport
from matrix.matrix import Matrix
from lib.sports.baseball.baseball import Baseball
from lib.sports.basketball.basketball import Basketball
from lib.sports.hockey.hockey import Hockey

class BaseballMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
    def render_standings(self):
        """
        Split the screen and then Roll the screen
        """
        self.draw_rectangle()
    def render_sport(self):
        self.render_standings()
        time.sleep(10)

class BasketballMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger

    def render_sport(self):
        return 

class HockeyMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger

    def render_sport(self):
        return 

class SportMatrix(Matrix):
    def __init__(self, matrix, api, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
    async def poll_api(self):
        return Sport(await self.api.run())
    
    def render(self, api):
        self.clear()
        self.reload_image()
        if 'baseball'in api.sport:
            self.draw_rectangle([(0, 0), (31,31)])
            self.draw_rectangle([(31, 0), (63, 31)])
        if 'basketball' in api.sport:
            sportmatrix = BasketballMatrix(self.matrix, api, self.logger)
        if 'hockey' in api.sport:
            sportmatrix = HockeyMatrix(self.matrix, api, self.logger)
        self.render_image()
        breakpoint()

