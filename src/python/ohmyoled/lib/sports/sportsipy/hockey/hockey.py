import asyncio
from asyncio.tasks import Task
from ohmyoled.lib.sports.logo import Logo, logo_map
from ohmyoled.lib.sports.sportsipy.result import SportsipyApiResult
from sportsipy.nhl.schedule import Schedule, Game
from sportsipy.nhl.teams import (
    Team,
    Teams,
)
from typing import List, Tuple
from sportsipy.nhl.boxscore import Boxscore, Boxscores
from ohmyoled.lib.asynclib import make_async
from datetime import datetime
from ohmyoled.lib.run import Runner
import ohmyoled.lib.sports.sportbase as base

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
    
    async def run(self) -> SportsipyApiResult:
        try:
            self.logger.info('Running Sportsipy')
            sport = {}
            team = logo_map[self.config['sport']['team_logo']['name']]
            self.logger.info("Running Hockey Sportsipy Api")
            sport['team'] = asyncio.create_task(self.run_team(team.shorthand), name="team_task")
            sport['schedule'] = asyncio.create_task(self.run_schedule(team.shorthand), name="schedule_task")
            sport['standings'] = asyncio.create_task(self.run_standings(), name="standing_task")
            await asyncio.gather(*sport.values())
            sport['sport'] = base.SportStructure.Hockey
            hockey_result = base.ModuleResult(
                name=sport['team'].result().name,
                team=base.Team(
                    name=sport['team'].result().name,
                    logo=logo_map[sport['team'].result().name],
                    position=sport['team'].result().rank
                ),
                schedule=[
                    base.Game(
                        team = base.Team(
                            name=sport['team'].result().name,
                            logo=logo_map[sport['team'].result().name],
                            position=sport['team'].result().rank
                        ),
                        timestamp=game.datetime,
                        status=base.determine_game_status(sport['team'].result(), game)[0],
                        opposing_team= base.Team(
                            name=game.opponent_name,
                            logo=logo_map[game.opponent_name]
                        ),
                        result=base.determine_game_status(sport['team'].result(), game)[1],
                        homeoraway=game.location,
                        score=base.Score(
                            team=game.goals_scored,
                            opposing_team=game.goals_allowed
                        ) if base.GameStatus.Finished else None  
                    ) for game in sport['schedule'].result()
                ],
                standings=[
                    base.Team(
                        name=team.name,
                        logo=logo_map[team.name],
                        position=team.rank
                    ) for team in sport['standings'].result()
                ],
                sport=sport['sport'],
                games_played=sport['team'].result().games_played,
                wins=sport['team'].result().wins,
                losses=sport['team'].result().losses
            )
            return SportsipyApiResult(api_result=hockey_result)
        except Exception as error:
            self.logger.error(f"An Error Occured in hockey Module: {error}")
            return None

