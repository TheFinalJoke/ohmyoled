import asyncio
from ohmyoled.lib.sports.sportsipy.result import SportsipyApiResult
from sportsipy.mlb.schedule import Schedule, Game
from sportsipy.mlb.teams import (
    Team,
    Teams,
)
from typing import List
from ohmyoled.lib.sports.logo import logo_map, baseball_teams
from sportsipy.mlb.boxscore import Boxscore, Boxscores
from ohmyoled.lib.asynclib import make_async
from datetime import datetime
from ohmyoled.lib.run import Runner
import ohmyoled.lib.sports.sportbase as base


class BaseballSportsipy(Runner):
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
            self.logger.info("Running Baseball Sportsipy Api")
            sport['team'] = asyncio.create_task(self.run_team(team.shorthand), name="team_task")
            sport['schedule'] = asyncio.create_task(self.run_schedule(team.shorthand), name="schedule_task")
            sport['standings'] = asyncio.create_task(self.run_standings(), name="standing_task")
            await asyncio.gather(*sport.values())
            sport['sport'] = base.SportStructure.Baseball
            abbr = sport['team'].result().abbreviation
            baseball_result = base.ModuleResult(
                name=baseball_teams[abbr],
                team=base.Team(
                    name=baseball_teams[abbr],
                    logo=logo_map[baseball_teams[abbr]],
                    position=sport['team'].result().rank
                ),
                schedule=[
                    base.Game(
                        team=base.Team(
                            name=baseball_teams[abbr],
                            logo=logo_map[baseball_teams[abbr]],
                            position=sport['team'].result().rank
                        ),
                        timestamp=game.datetime,
                        status=base.determine_game_status(sport['team'].result(), game)[0],
                        opposing_team=base.Team(
                            name=baseball_teams[game.opponent_abbr],
                            logo=logo_map[baseball_teams[game.opponent_abbr]],
                        ),
                        result=base.determine_game_status(sport['team'].result(), game)[1],
                        homeoraway=game.location,
                        score=base.Score(
                            team=game.runs_scored,
                            opposing_team=game.runs_allowed
                        )
                    ) for game in sport['schedule'].result()
                ],
                standings=[
                    base.Team(
                        name=baseball_teams[team.abbreviation],
                        logo=logo_map[baseball_teams[team.abbreviation]],
                        position=team.rank
                    ) for team in sport['standings'].result()
                ],
                sport=base.SportStructure.Baseball,
                games_played=sport['team'].result().games_finished,
                wins=sport['team'].result().wins,
                losses=sport['team'].result().losses
            )
            return SportsipyApiResult(api_result=baseball_result)
        except Exception as error:
            self.logger.error(f"Error Occured inside of baseball module: {error}")
            return None

