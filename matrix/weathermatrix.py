#!/usr/bin/env python3 

import asyncio
from typing import Dict
from matrix.matrix import Matrix, MatrixBase, FontException, Canvas
from lib.weather import Weather

class WeatherMatrix(Canvas):
    def __init__(self, matrix: Canvas, api: Dict, logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger

    async def poll_api(self) -> Weather:
        return Weather(await self.api.run())
    
    def render(self, api: Weather):
        breakpoint()
