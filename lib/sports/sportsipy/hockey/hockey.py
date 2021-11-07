import asyncio
from lib.sports.sportsipy.result import SportsipyApiResult
from sportsipy.nhl.schedule import Schedule, Game
from sportsipy.nhl.teams import (
    Team,
    Teams,
)
from typing import List
from sportsipy.nhl.boxscore import Boxscores
from lib.asynclib import make_async
from datetime import datetime
from lib.run import Runner

class HockeySportsipy(Runner):
    def __init__(self, config):
        super().__init__(config)
    
    @make_async
    def run_team(self, team: str) -> Team:
        self.logger.debug("Running Team")
        return Team(team)
    
    @make_async 
    def run_schedule(self, team: str) -> Schedule:
        self.logger.debug("Running Schedule")
        return Schedule(team)

    @make_async
    def run_standings(self):
        self.logger.debug("Running Standings")
        return Teams()

    @make_async
    def run_games(self, team, games) -> List[Game]:
        return games[:team.games_played]
            
    async def run(self) -> SportsipyApiResult:
        self.logger.info('Running Sportsipy')
        sport = {}
        team = self.config['sport']['team_id']
        self.logger.info("Running Hockey Sportsipy Api")
        sport['team'] = asyncio.create_task(self.run_team(team), name="team_task")
        sport['schedule'] = asyncio.create_task(self.run_schedule(team), name="schedule_task")
        sport['standings'] = asyncio.create_task(self.run_standings(), name="standing_task")
        await asyncio.gather(*sport.values())
        sport['games'] = await self.run_games(sport['team'].result(), sport['schedule'].result())
        sport['sport'] = 'hockey'
        return SportsipyApiResult(api_result=sport)

