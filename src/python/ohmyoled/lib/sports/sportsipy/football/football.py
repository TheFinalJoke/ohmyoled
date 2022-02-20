import asyncio
import ohmyoled.lib.sports.sportbase as base
from ohmyoled.lib.sports.sportsipy.result import SportsipyApiResult
from sportsipy.nfl.schedule import Schedule, Game
from sportsipy.nfl.teams import (
    Team,
    Teams,
)
from typing import List
from ohmyoled.lib.sports.logo import logo_map
from ohmyoled.lib.asynclib import make_async
from datetime import datetime
from ohmyoled.lib.run import Runner

class FootballSportsipy(Runner):
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
            self.logger.info("Inside of the Football Sportsipy")
            sport = {}
            team = logo_map[self.config['sport']['team_logo']['name']]
            self.logger.info("Running Football Sportsipy Api")
            sport['team'] = asyncio.create_task(self.run_team(team.shorthand), name="team_task")
            sport['schedule'] = asyncio.create_task(self.run_schedule(team.shorthand), name="schedule_task")
            sport['standings'] = asyncio.create_task(self.run_standings(), name="standing_task")
            await asyncio.gather(*sport.values())
            sport['sport'] = base.SportStructure.Football
            football_result = base.ModuleResult(
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
                            team=game.points_scored,
                            opposing_team=game.points_allowed
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
            return SportsipyApiResult(api_result=football_result)
        except Exception as error:
            self.logger.error(f"Error Occured inside of football module: {error}")
            return None
