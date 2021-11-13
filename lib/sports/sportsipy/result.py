import json
from asyncio import Task
from typing import Dict, List, Tuple
from datetime import datetime
from enum import Enum
from sportsipy.nhl.schedule import Game
from sportsipy.nhl.teams import Team as nhl_team
from lib.sports.sportbase import SportResultBase, API
from lib.sports.logo import Logo, logo_map
import lib.sports.sportbase as base

class SportsipyApiResult(SportResultBase):

    def __init__(self, api_result: Dict[str, Task]) -> None:
        self.api_result = api_result
        self._get_sport: Enum = api_result['sport']
        self._team: base.Team = base.Team(
            name=api_result['team'].result().name,
            logo=logo_map[api_result['team'].result().name],
            position=api_result['team'].result().rank
        )
        self._schedule: base.SportStandings = base.SportStandings(
            positions=[
                base.Game(
                    team = self.get_team,
                    timestamp=game.datetime,
                    status=self.determine_game_status(game)[0],
                    opposing_team= base.Team(
                        name=game.opponent_name,
                        logo=logo_map[game.opponent_name]
                    ),
                    result=self.determine_game_status(game)[1],
                    homeoraway=game.location,
                    score=base.Score(
                        team=game.goals_scored,
                        opposing_team=game.goals_allowed
                    ) if base.GameStatus.Finished else None
                ) for game in api_result['schedule'].result()
            ]
        )
       
        self._api: Enum = API.SPORTSIPY
        self._standings: List[base.Team] = [
            base.Team(
                name=team.name,
                logo=logo_map[team.name],
                position=team.rank
                ) for team in api_result['standings'].result()
        ]
        self._position = self._team.position
        self._get_leagues = None
        self._games_played: List[Game] = self._schedule.positions[:api_result['team'].result().games_played]
        self._get_wins: List[Game] = [game for game in self._games_played if base.GameResult.WIN == game.result]
        self._win_percentage: float = api_result['team'].result().wins/len(self._games_played)
        self._losses: List[Game] = [game for game in self._games_played if base.GameResult.LOSS == game.result]
        self._loss_percentage: float = api_result['team'].result().losses/len(self._games_played)
        self._next_game = self._schedule.positions[len(self._games_played)]
        self._game_ids = None
        self._get_error = (True, "")
    
    def determine_game_status(self, game) -> Tuple[base.GameStatus, base.GameResult]:
        if game.game == self.api_result['team'].result().games_played:
            return base.GameStatus.NotStarted, base.GameResult.NOT_PLAYED
        elif game.game > self.api_result['team'].result().games_played:
            return base.GameStatus.NotStarted, base.GameResult.NOT_PLAYED
        else:
            if game.result == "Loss" or game.result == "OTL":
                return base.GameStatus.Finished, base.GameResult.LOSS
            else:
                return base.GameStatus.Finished, base.GameResult.WIN

    @property
    def get_api(self) -> Enum:
        return self._api

    @property
    def get_sport(self) -> Enum:
        return self._get_sport
    
    @property
    def team_name(self):
        return self._team.name
    
    @property
    def get_logo(self) -> Logo:
        return logo_map[self._team.name]

    @property
    def get_team(self):
        return self._team

    @property
    def get_error(self):
        return self._get_error

    @property 
    def get_length_position_teams(self):
        return len(self._standings)
    
    @property
    def get_standings(self):
        return self._standings
    
    @property
    def get_schedule(self):
        return self._schedule
    @property 
    def position_teams(self):
        return self._position_teams
    
    @property
    def get_leagues(self):
        return self._get_leagues
    
    @property
    def get_games_played(self):
        return self._games_played
    
    @property
    def get_wins(self):
        return self._get_wins
    
    @property
    def get_wins_percentage(self):
        return self._win_percentage
    
    @property
    def get_losses(self):
        return self._losses
    
    @property
    def get_loss_percentage(self):
        return self._loss_percentage

    @property 
    def get_game_ids(self):
        return self._game_ids
    
    def get_specific_score(self, game_id):
        return self._game_result.get(game_id)
    
    @property
    def get_next_game(self):
        return self._next_game