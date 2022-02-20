from statistics import mean
from ohmyoled.lib.sports.apisports.apisports import ApiSports
from ohmyoled.lib.sports.sportsipy.sportsipy import SportsipyAPI
from sportsipy.nhl.teams import Team as nhl_team
from ohmyoled.lib.run import Runner, Caller
from dataclasses import dataclass
from enum import Enum
from typing import Tuple, List, Dict, Set, Optional
from datetime import datetime
from ohmyoled.lib.sports.sportbase import API
from ohmyoled.lib.sports.logo import Logo, logo_map
import ohmyoled.lib.sports.sportbase as base
import json

class SportApi(Runner):
    def __init__(self, config):
        super().__init__(config)
    
    def parse_args(self):
        return super().parse_args()
    
    async def run(self):
        self.logger.info("Running Sports")
        # Instead of using a json and dictionary -> Build individual objects 
        # For Each API and then Bubble up back to sport to be normalized
        # Build like a binary Tree
        # Can Do checks here to bubble up problems
        if self.config['sport']['api'] == "api-sports":
            api_sports = ApiSports(self.config)
            api_result = await api_sports.run_api_sports()
            return api_result
        elif self.config['sport']['api'] or self.config['sport']['api'] == 'sportsipy':
            sports = SportsipyAPI(self.config)
            api_result = await sports.run_api_sportsipy()
            return api_result
        return

@dataclass(repr=True)
class Sport:
    team_name: str
    # Enum Mapping to sport 
    sport: base.SportStructure
    # Method of API
    api: base.API
    # Logo for the Team
    logo: Logo
    # List of standing in order
    standings: base.SportStandings
    # List of games 
    schedule: List[base.Game]
    # A Single game representing the next game
    next_game: base.Game
    # A List of Wins, with a float of win percentage
    wins: Tuple[List[base.Game], float]
    # A List of Losses, with a flost of loss percentage
    losses: Tuple[List[base.Game], float]
    # Set of Leagues if Applicable
    leagues: Set[str] = None

class SportTransform(Caller):
    """
    Final Normalized Object that goes to 
    The Sport Matrix. This is the final object
    * NO MATTER What the return types should not change

    """
    def __init__(self, api_result) -> None:
        # Any Object from any api object
        self.api_result = api_result
    
    @property
    def team_name(self):
        return self.api_result.team_name

    @property
    def get_api(self) -> Enum:
        return self.api_result.get_api

    def _normalize_sport(self) -> Enum:
        """
        All apis Should return a string in 
        thier result object
        """
        return self.api_result.get_sport

    @property
    def get_sport(self) -> Enum:
        return self._normalize_sport()

    def _normalize_logo(self) -> Logo:
        return self.api_result.get_logo

    @property
    def get_logo(self) -> Logo:
        return self._normalize_logo()
    
    def _normalize_length_position_teams(self) -> int:
        return self.api_result.get_length_position_teams

    @property 
    def get_length_position_teams(self):
        return self._normalize_length_position_teams()
    
    def _normalize_standings(self) -> base.SportStandings:
        return self.api_result.get_standings
    
    @property
    def get_standings(self):
        return self._normalize_standings()
    
    def _normalize_leagues(self) -> Set[str]:
        return self.api_result.get_leagues
    
    @property
    def get_leagues(self) -> Set[str]:
        return self._normalize_leagues()

    def _normalize_schedule(self):
        return self.api_result.get_schedule

    @property
    def get_schedule(self):
        return self._normalize_schedule()

    def _normalize_games_played(self) -> List[str]:
        return self.api_result.get_games_played
    
    @property
    def get_games_played(self) -> List[str]:
        return self._normalize_games_played()
    
    def _normalize_next_game(self) -> base.Game:
        return self.api_result.get_next_game
    
    @property
    def get_next_game(self) -> base.Game:
        return self._normalize_next_game()

    def _normalize_wins(self) -> List[str]:
        return self.api_result.get_wins
   
    @property
    def get_wins(self) -> List[str]:
        return self._normalize_wins()
    
    def _normalize_win_percentage(self) -> float:
        return self.api_result.get_wins_percentage

    @property
    def get_wins_percentage(self) -> float:
        return self._normalize_win_percentage()
    
    def _normalize_losses(self) -> List[str]:
        return self.api_result.get_losses
    
    @property
    def get_losses(self) -> List[str]:
        return self._normalize_losses()
    
    def _normalize_loss_percentage(self) -> float:
        return self.api_result.get_loss_percentage

    @property
    def get_loss_percentage(self) -> float:
        return self._normalize_loss_percentage()

    def _normalize_game_ids(self) -> Optional[List[int]]:
        return self.api_result.get_game_ids

    @property 
    def get_game_ids(self) -> Optional[List[int]]:
        return self._normalize_game_ids()