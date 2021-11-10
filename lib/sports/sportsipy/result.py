import json
from asyncio import Task
from typing import Dict, List, Tuple
from datetime import datetime
from enum import Enum
from sportsipy.nhl.boxscore import Boxscores
from sportsipy.nhl.schedule import Game
from sportsipy.nhl.teams import Team as nhl_team
from lib.sports.sportbase import SportResultBase, API
from lib.sports.logo import Logo, logo_dict
import lib.sports.sportbase as base

class SportsipyApiResult(SportResultBase):

    def __init__(self, api_result: Dict[str, Task]) -> None:
        self.api_result = api_result
        self._get_sport: Enum = api_result['sport']
        self._team: nhl_team = api_result['team'].result()
        self._schedule = api_result['schedule'].result()
        self._api: Enum = API.SPORTSIPY
        self._abbr = [team.abbreviation for team in api_result['standings'].result()]
        self._standings: List[nhl_team] = [team for team in api_result['standings'].result()]
        self._position_teams: List[Tuple[nhl_team, int]] = [(team, num) for num, team in enumerate(self._standings, start=1)]
        self._position = [team[1] for team in self.position_teams if team[0].name == self.team_name][0]
        self._get_leagues = None
        self._games_played: List[Game] = api_result['games']
        self._get_wins: List[Tuple[str, int]] = [(self._team.name, game.goals_scored) for game in self._games_played if game.result == "Win" or game.result == "OTW"]
        self._win_percentage: float = self._team.wins/len(self._games_played)
        self._losses: List[Tuple[str, int]] = [(self._team.name, game.goals_scored) for game in self._games_played if game.result == "Loss" or game.result == "OTL"]
        self._loss_percentage: float = self._team.losses/len(self._games_played)
        self._next_game = self._schedule[len(self._games_played) + 1]
        self._game_ids = None
        self._timestamps: List[Tuple[str, datetime]] = [(game.opponent_name, game.datetime) for game in self._schedule]
        self._vs = [(None, game.opponent_name) for game in self._schedule]
        self._status = [(game.opponent_name, game.result) for game in self._schedule]
        self._get_error = (True, "")
    
    @property
    def get_api(self) -> Enum:
        return self._api

    @property
    def get_sport(self) -> Enum:
        return self._get_sport
    
    @property
    def team_name(self):
        return self._team.name
    
    def get_logo(self):
        return logo_dict[self._team.abbreviation]

    @property
    def get_team(self):
        return base.Team(
            name=self._team.name,
            position=self._position,
            logo=self.get_logo,
            league=None
        )

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
    
    @property
    def get_timestamps(self):
        return self._timestamps
    
    @property
    def get_versus(self):
        return self._vs
    
    @property
    def get_status(self):
        return self._status
    
    @property
    def get_scores(self):
        return self._game_result
    
    def get_specific_score(self, game_id):
        return self._game_result.get(game_id)