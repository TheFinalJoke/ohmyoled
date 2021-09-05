#!/usr/bin/env python3

import asyncio
import time
from typing import List, Dict, Tuple
from collections import deque
from datetime import datetime, timedelta
from PIL import ImageFont, Image
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
    
    def baseball_divisions(self, standings: List[Dict]) -> List[str]:
        american_queue = deque(["American League"])
        national_queue = deque(["National League"])
        for team in standings:
            if team["league"] == "American League":
                american_queue.append(f"{team['position']}: {team['name']}")
            elif team["league"] == "National League":
                national_queue.append(f"{team['position']}: {team['name']}")
        return list(american_queue), list(national_queue)

    def determine_nextgame(self, nextgame_api):
        now = datetime.now()
        last_16 = now - timedelta(hours=16)
        status = ("FT", "ABD")
        for game in nextgame_api:
            if last_16 <= datetime.fromtimestamp(game['timestamp']) <= now:
                return game
            if game['status'] not in status:
                return game
    def away_team(self, nextgame_data):
        away = nextgame_data['teams']['away']
        away.update(nextgame_data['score']['away'])
        return away
    def home_team(self, nextgame_data):
        home = nextgame_data['teams']['home']
        home.update(nextgame_data['score']['home'])
        return home
    def render_baseball(self, api):
        self.clear()
        self.reload_image()
        scrolling_font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        if 'baseball'in api.sport:
            # Can't Have multiple images and or buffers
            american, national = self.baseball_divisions(api.standings)
            american.extend(national)
            text = "\n".join(american)
            nextgame = self.determine_nextgame(api.next_game)
            away_data = self.away_team(nextgame_data=nextgame)
            home_data = self.home_team(nextgame)
            img_width, img_height = self.get_text_size(text)

            ypos = 0
            while ypos < 100:
                ypos += 1
                if ypos == 1 or ypos % 8 == 0:
                    xpos = 0
                    pause = True
                    while xpos < img_width:
                        if xpos > 100:
                            break
                        self.reload_image()
                        self.draw_multiline_text((-xpos, -ypos-ypos), text, font=scrolling_font, fill=(0,255,0), spacing=1) if ypos > 1 else self.draw_multiline_text((-xpos, 0), text, font=scrolling_font, fill=(0,255,0), spacing=1)
                        self.render_image()
                        time.sleep(3) if pause else time.sleep(0)
                        xpos += 1 
                        pause = False
                self.reload_image()
                self.image_resize(64, img_height)
                self.draw_multiline_text((0, -ypos), text, font=scrolling_font, fill=(0,255,0), spacing=1)
                self.render_image(yoffset=-ypos)

    def render(self, api):
        self.clear()
        self.reload_image()
        scrolling_font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        if 'baseball'in api.sport:
            self.render_baseball(api)
        if 'basketball' in api.sport:
            sportmatrix = BasketballMatrix(self.matrix, api, self.logger)
        if 'hockey' in api.sport:
            sportmatrix = HockeyMatrix(self.matrix, api, self.logger)

