import json
from asyncio import Task
from typing import Dict, List, Tuple
from datetime import datetime
from sportsipy.nhl.boxscore import Boxscores
from sportsipy.nhl.schedule import Game
from sportsipy.nhl.teams import Team
from lib.sports.sportbase import SportResultBase, API

class SportsipyApiResult(SportResultBase):

    def __init__(self, api_result: Dict[str, Task]) -> None:
        self.api_result = api_result
        self._get_sport: str = api_result['sport']
        self._team: Team = api_result['team'].result()
        self._schedule = api_result['schedule'].result()
        self._standings: List[Team] = [team for team in api_result['standings'].result()]
        self._position_teams: List[Tuple[Team, int]] = [(team, num) for num, team in enumerate(self._standings, start=1)]
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
        self._status = None
        self._get_error = (True, "")
        self._api = API.SPORTSIPY

    def __repr__(self):
        attrs = [
            f"length={self._length}",
            f"positions={json.dumps(self._positions, indent=2)}",
            f'leagues={json.dumps(self._leagues, indent=2)}',
            f"games_played={json.dumps(self._games_played, indent=2)}",
            f"wins={json.dumps(self._wins, indent=2)}",
            f"wins_percentage={json.dumps(self._wins_percentage, indent=2)}",
            f"losses={json.dumps(self._losses, indent=2)}",
            f"loss_percentage={json.dumps(self._loss_percentage, indent=2)}",
            f"game_ids={json.dumps(self._game_ids, indent=2)}",
            f"timestamps={json.dumps(self._timestamps, indent=2)}",
            f"teams={json.dumps(self._teams, indent=2)}",
            f"vs={json.dumps(self._vs, indent=2)}",
            f"status={json.dumps(self._status, indent=2)}",
            f"game_result={json.dumps(self._game_result, indent=2)}"
        ]
        joined = "\t\n".join(attrs)
        return f"SportsipyApiResult(\n{joined})"
    
    @property
    def get_api(self):
        return self._api

    @property
    def get_sport(self):
        return self._get_sport
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
    def get_teams(self):
        return self._teams
    
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