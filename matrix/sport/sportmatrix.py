#!/usr/bin/env python3

from logging import Logger
import os
import time
import urllib.request
from datetime import datetime
from typing import List, Dict, Tuple
from collections import deque
from PIL import ImageFont, Image, ImageDraw
from lib.sports.sports import Sport
from matrix.matrix import Matrix

class SportMatrix(Matrix):
    def __init__(self, matrix, api: Sport, logger: Logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger
    def __str__(self) -> str:
        return "SportMatrix"
    async def poll_api(self) -> Sport:
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
        status: Tuple = ("FT", "ABD")
        for game in nextgame_api:
            if "IN" in game['status']:
                self.logger.debug(f"In Game")
                # During the game
                return game
            if game['status'] == "FT" and datetime.fromtimestamp(game['timestamp']).date() == datetime.today().date():
                # Same Day will display for the rest of the day
                self.logger.debug("Game is finished but still same day")
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

    def get_logo(self, logo_url: str, name: str) -> str:
        file_name: str = f"/tmp/{name}.png"
        if not os.path.isfile(file_name):
            urllib.request.urlretrieve(logo_url, file_name)
        return file_name
        
    def build_in_game_image(self, nextgame: Dict):
        middle_image = self.make_new_image((34,16))
        middle_draw = ImageDraw.Draw(middle_image)
        font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        status = nextgame['status']
        self.logger.debug(f"status: {status}")
        score = (nextgame['teams']['home']['total'], nextgame['teams']['away']['total'])
        self.logger.debug(f"Score: {score}")
        middle_draw.multiline_text((12,0), f"{status}\n{score[0]}-{score[1]}", font=font)
        return middle_image, (15, 0)

    def build_finished_game_image(self, nextgame):
        middle_image = self.make_new_image((34,16))
        middle_draw = ImageDraw.Draw(middle_image)
        font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        status = nextgame['status']
        score = (nextgame['teams']['home']['total'], nextgame['teams']['away']['total'])
        middle_draw.multiline_text((12,0), f"{status}\n{score[0]}-{score[1]}", font=font)
        return middle_image, (15, 0)
        
    def check_offseason(self, api) -> bool:
        try:
            start_time, end_time = api.get_timestamps[0][1], api.get_timestamps[-1][1]
            if datetime.fromtimestamp(start_time) <= datetime.now() <= datetime.fromtimestamp(end_time):
                return True
        except Exception:
            return False

    def build_next_game_image(self, nextgame: Dict):
        middle_image = self.make_new_image((34,16))
        middle_draw = ImageDraw.Draw(middle_image)
        font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        time = datetime.fromtimestamp(nextgame['timestamp']).strftime("%I:%M%p %a")
        formatted_time = time.split()
        formatted_time[0] = f" {formatted_time[0]}"
        time = "\n   ".join(formatted_time)
        middle_draw.multiline_text((0,0), f'{time}', font=font)
        return middle_image, (15, 0)

    def build_home_away_image(self, nextgame):
        self.logger.debug(f"Building Home away image")
        home_data = self.home_team(nextgame)
        self.logger.debug(f"Home Data {home_data}")
        home_logo = Image.open(self.get_logo(home_data['logo'], home_data['id']))
        self.logger.debug(f"Got Logo {home_logo}")
        home_logo.thumbnail((16,16))
        away_data = self.away_team(nextgame)
        self.logger.debug(f"Away Data: {away_data}")
        away_logo = Image.open(self.get_logo(away_data['logo'], away_data['id']))
        self.logger.debug(f"Away Logo {away_logo}")
        away_logo.thumbnail((16,16))
        return (home_logo, (-2,0)), (away_logo, (50, 0))

    def build_middle_nextgame(self, api) -> Image:
        nextgame = self.determine_nextgame(api.next_game)
        if "IN" in nextgame['status']: 
            return self.build_in_game_image(nextgame)
        elif "NS" == nextgame['status']:
            return self.build_next_game_image(nextgame)
        elif "FT" == nextgame['status']:
            return self.build_finished_game_image(nextgame)

    def build_middle_image(self, api) -> Image:
        nextgame = self.determine_nextgame(api.next_game)
        home_image, away_image = self.build_home_away_image(nextgame)
        middle_image = self.build_middle_nextgame(api)
        master_middle_image = self.make_new_image((64, 16))
        master_middle_image.paste(home_image[0], home_image[1])
        master_middle_image.paste(away_image[0], away_image[1])
        master_middle_image.paste(middle_image[0], middle_image[1])
        return master_middle_image, (0,9)
    
    def build_top_home_away_images(self, nextgame: Dict, xpos: int) -> Tuple:
        font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        top_home_image = self.make_new_image((22,8))
        top_home_draw = ImageDraw.Draw(top_home_image)
        top_away_image = self.make_new_image((22,8))
        top_away_draw = ImageDraw.Draw(top_away_image)
        hometeam = nextgame['teams']['home']['name']
        awayteam = nextgame['teams']['away']['name']
        top_home_draw.text((-xpos,0), hometeam, font=font)
        top_away_draw.text((-xpos,0), awayteam, font=font)
        return top_home_image, top_away_image

    def build_top_image(self, api: Dict, xpos: int) -> Tuple:
        nextgame = self.determine_nextgame(api.next_game)
        master_top_image = self.make_new_image((64, 8))
        home_image, away_image = self.build_top_home_away_images(nextgame, xpos)
        top_middle_image = self.make_new_image((22,8))
        top_middle_draw = ImageDraw.Draw(top_middle_image)
        top_middle_draw.text((5,0), "VS", font=ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8))
        master_top_image.paste(home_image, (0,0))
        master_top_image.paste(away_image, (44,0))
        master_top_image.paste(top_middle_image, (22,0))
        return master_top_image, (0,0)
    
    def build_standings_image(self, api, xpos) -> Tuple[int, int]:
        """,
        This is most bottom Image
        """
        standings_image = Image.new("RGB", (64,8))
        standings_draw = ImageDraw.Draw(standings_image)
        scrolling_font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        color = (156,163,173)
        # Can't Have multiple images and or buffers
        american, national = self.baseball_divisions(api.standings)
        american.extend(national)
        text = " ".join(american)
        standings_draw.text(
            (-xpos, 0), 
            text, 
            font=scrolling_font, 
            fill=color
        )
        return standings_image, (0, 25)

    async def render(self, api, loop):
        self.clear()
        self.reload_image()
        if 'baseball'in api.sport:
            # Check Data if Offseason if yes Diplay Offseason, Otherwise Display Data
            # Check data if Game is active, if yes Display game -> Score Inning AT bat Maybe?
            # Else Display next game
            # Only do standings right now
            self.logger.info("Found Baseball, Displaying Baseball Matrix")
            if self.check_offseason(api):
                xpos = 0
                xpos_for_top = 0 
                while xpos < 2700:
                    self.reload_image()
                    images = (
                        self.build_standings_image(api, xpos),
                        self.build_middle_image(api),
                        self.build_top_image(api, xpos_for_top),
                    )
                    for image, position in images:
                        self.paste_image(image, position)
                    await self.render_image()
                    xpos +=1
                    xpos_for_top += 1
                    if xpos_for_top == 100:
                        xpos_for_top = 0
                    time.sleep(3) if xpos == 1 else time.sleep(.001)
            else:
                font = ImageFont.truetype("/usr/share/fonts/fonts/04b24.otf", 14)
                self.draw_multiline_text((0, 0), "Basketball\nOffseason", font=font)
                await self.render_image()
                time.sleep(30)
        if 'basketball' in api.sport:
            # Check Data if Offseason if yes Diplay Offseason, Otherwise Display Data
            # Check data if Game is active, if yes Display game -> Score Inning AT bat Maybe?
            # Else Display next game
            # Only do standings right now
            self.logger.info("Found Basketball, Displaying Basketball Matrix")
            if self.check_offseason(api):
                xpos = 0
                xpos_for_top = 0 
                while xpos < 2700:
                    self.reload_image()
                    images = (
                        self.build_standings_image(api, xpos),
                        self.build_middle_image(api),
                        self.build_top_image(api, xpos_for_top),
                    )
                    for image, position in images:
                        self.paste_image(image, position)
                    self.render_image()
                    xpos +=1
                    xpos_for_top += 1
                    if xpos_for_top == 100:
                        xpos_for_top = 0
                    time.sleep(3) if xpos == 1 else time.sleep(.01)
            else:
                font = ImageFont.truetype("/usr/share/fonts/fonts/04b24.otf", 14)
                self.draw_multiline_text((0, 0), "Basketball\nOffseason", font=font)
                self.render_image()
                time.sleep(30)
        if 'hockey' in api.sport:
            self.logger.info("Found Hockey, Displaying Hockey Matrix")
            if self.check_offseason(api):
                pass
                #sportmatrix = HockeyMatrix(self.matrix, api, self.logger)
            else:
                font = ImageFont.truetype("/usr/share/fonts/fonts/04b24.otf", 14)
                self.draw_multiline_text((0, 0), "Hockey\nOffseason", font=font)
                self.render_image()
                time.sleep(30)