#!/usr/bin/env python3

from logging import Logger
import os
import time
import urllib.request
import ohmyoled.lib.sports.sportbase as sport_types
from datetime import datetime
from typing import Dict, Tuple

from ohmyoled.matrix.error import ErrorMatrix
from PIL import ImageFont, Image, ImageDraw
from ohmyoled.lib.sports.sports import SportTransform, Sport
from ohmyoled.matrix.matrix import Matrix

class SportMatrix(Matrix):
    def __init__(self, matrix, api, logger: Logger) -> None:
        self.matrix = matrix
        self.api = api
        self.logger = logger

    def __str__(self) -> str:
        return "SportMatrix"

    async def poll_api(self) -> Sport:
        sport = SportTransform(await self.api.run())
        if not sport.api_result:
            return None
        return Sport(
            team_name=sport.team_name,
            sport=sport.get_sport,
            logo=sport.get_logo,
            api=sport.get_api,
            standings=sport.get_standings,
            schedule=sport.get_schedule,
            next_game=sport.get_next_game,
            wins=sport.get_wins,
            losses=sport.get_losses,
        )
    
    def away_team(self, nextgame_data):
        if nextgame_data.homeoraway.lower() == "home":
            return nextgame_data.team
        else:
            return nextgame_data.opposing_team

    def home_team(self, nextgame_data):
        if nextgame_data.homeoraway.lower() == "away":
            return nextgame_data.team
        else:
            return nextgame_data.opposing_team

    def get_logo(self, logo_url: str, name: str) -> str:
        name = name.replace(" ", "_")
        file_name: str = f"/tmp/{name}.png"
        if not os.path.isfile(file_name):
            urllib.request.urlretrieve(logo_url, file_name)
        return file_name
        
    def build_in_game_image(self, nextgame):
        middle_image = self.make_new_image((34,16))
        middle_draw = ImageDraw.Draw(middle_image)
        font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        status = nextgame.status
        self.logger.debug(f"status: {status}")
        score = (nextgame.score.team, nextgame.score.opposing_team)
        self.logger.debug(f"Score: {score}")
        middle_draw.multiline_text((12,0), f"{status}\n{score[0]}-{score[1]}", font=font)
        return middle_image, (15, 0)

    def build_finished_game_image(self, nextgame):
        middle_image = self.make_new_image((34,16))
        middle_draw = ImageDraw.Draw(middle_image)
        font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        status = nextgame.status
        score = (nextgame.score.team, nextgame.score.opposing_team)
        middle_draw.multiline_text((12,0), f"{status}\n{score[0]}-{score[1]}", font=font)
        return middle_image, (15, 0)
        
    def check_offseason(self, api) -> bool:
        try:
            start_time, end_time = api.schedule.positions[0].timestamp, api.schedule.positions[-1].timestamp
            # Have to check for play offs
            if start_time <= datetime.now() <= end_time:
                return True
        except Exception:
            return False

    def build_next_game_image(self, nextgame):
        middle_image = self.make_new_image((34,16))
        middle_draw = ImageDraw.Draw(middle_image)
        font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        time = nextgame.timestamp.strftime("%m-%d %a")
        formatted_time = time.split()
        formatted_time[0] = f"   {formatted_time[0]}"
        time = "\n   ".join(formatted_time)
        middle_draw.multiline_text((0,0), f'{time}', font=font)
        return middle_image, (15, 0)

    def build_home_away_image(self, nextgame):
        home_data = self.home_team(nextgame)
        home_logo = Image.open(self.get_logo(home_data.logo.url, home_data.name))
        home_logo.thumbnail((16,16))
        away_data = self.away_team(nextgame)
        away_logo = Image.open(self.get_logo(away_data.logo.url, away_data.name))
        away_logo.thumbnail((16,16))
        return (home_logo, (-2,0)), (away_logo, (50, 0))

    def build_middle_nextgame(self, api) -> Image:
        if api.next_game.status == sport_types.GameStatus.InGame:
            return self.build_in_game_image(api.next_game)
        elif sport_types.GameStatus.NotStarted == api.next_game.status:
            return self.build_next_game_image(api.next_game)
        elif sport_types.GameStatus.Finished == api.next_game.status:
            return self.build_finished_game_image(api.next_game)

    def build_middle_image(self, api) -> Image:
        home_image, away_image = self.build_home_away_image(api.next_game)
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
        hometeam = nextgame.team.name
        awayteam = nextgame.opposing_team.name
        top_home_draw.text((-xpos,0), hometeam, font=font)
        top_away_draw.text((-xpos,0), awayteam, font=font)
        return top_home_image, top_away_image

    def build_top_image(self, api: Dict, xpos: int) -> Tuple:
        master_top_image = self.make_new_image((64, 8))
        home_image, away_image = self.build_top_home_away_images(api.next_game, xpos)
        top_middle_image = self.make_new_image((22,8))
        top_middle_draw = ImageDraw.Draw(top_middle_image)
        top_middle_draw.text((5,0), "VS", font=ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8))
        master_top_image.paste(home_image, (0,0))
        master_top_image.paste(away_image, (44,0))
        master_top_image.paste(top_middle_image, (22,0))
        return master_top_image, (0,0)
    
    def build_standings_image(self, api, xpos) -> Tuple[int, int]:
        """
        This is most bottom Image
        """
        standings_image = Image.new("RGB", (64,8))
        standings_draw = ImageDraw.Draw(standings_image)
        scrolling_font = ImageFont.truetype("/usr/share/fonts/fonts/04B_03B_.TTF", 8)
        color = (156,163,173)

        text = " ".join(api)
        standings_draw.text(
            (-xpos, 0), 
            text, 
            font=scrolling_font, 
            fill=color
            )
    
        return standings_image, (0, 25)

    async def render(self, api, loop):
        try:
            if not api:
                raise Exception("Error Ocurred inside of the sport matrix")
            self.clear()
            self.reload_image()
            if self.check_offseason(api):
                xpos = 0
                xpos_for_top = 0 
                positions = [f"{team.position}. {team.name}" for team in api.standings]
                while xpos < 2700:
                    self.reload_image()
                    images = (
                        self.build_standings_image(positions, xpos),
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
                self.draw_multiline_text((0, 0), f"{api.sport.name}\nOffseason", font=font)
                await self.render_image()
                time.sleep(30)

        except Exception as e:
            self.logger.error(e)
            error_matrix = ErrorMatrix(self.matrix, self.logger, "Sports Matrix")
            await error_matrix.render()
