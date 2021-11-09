from statistics import mean
from lib.sports.apisports.apisports import ApiSports
from lib.sports.sportsipy.sportsipy import SportsipyAPI
from sportsipy.nhl.teams import Team as nhl_team
from lib.run import Runner, Caller
from dataclasses import dataclass
from enum import Enum
from typing import Tuple, List, Dict, Set, Optional
from datetime import datetime
from lib.sports.sportbase import API, GameStatus
import lib.sports.sportbase as base
import json

# TODO @thefinaljoke More abstract to use different apis 
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
    # An Error if Applicable
    error: Optional[str] = None

class SportTransform(Caller):
    """
    Final Normalized Object that goes to 
    The Sport Matrix. This is the final object
    * NO MATTER What the return types should not change

    """
    def __init__(self, api_result) -> None:
        # Any Object from any api object
        self.api_result = api_result
        breakpoint()
    
    @property
    def team_name(self):
        return self.api_result.team_name
    @property
    def api(self) -> Enum:
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
    
    def _normalize_error(self) -> Tuple[bool, str]:
        return self.api_result.get_error

    @property
    def get_error(self) -> Tuple[bool, str]:
        return self._normalize_error()
    
    def _normalize_length_position_teams(self) -> int:
        if self.api == API.APISPORTS:
            return len(self.api_result.standings)
        elif self.api == API.SPORTSIPY:
            return self.api_result.get_length_position_teams
    @property 
    def get_length_position_teams(self):
        return self._normalize_length_position_teams()
    
    def _normalize_standings(self) -> base.SportStandings:
        if self.api == API.APISPORTS:
            standings = [
                base.Team(
                    name=team['name'], 
                    position=team['position'],
                    league=team['league']
                ) for team in self.api_result.get_standings]
            return base.SportStandings(positions=standings)
        elif self.api == API.SPORTSIPY:
            standings = [
                base.Team(
                    name=team.name,
                    position=num,
                ) for num, team in enumerate(self.api_result.get_standings)]
            return base.SportStandings(positions=standings)
    
    @property
    def get_standings(self):
        return self._normalize_standings()
    
    def _normalize_leagues(self) -> Set[str]:
        if not self.api_result.get_leagues:
            return self.api_result.get_leagues
        return {league[1] for league in self.api_result.get_leagues}
    
    @property
    def get_leagues(self) -> Set[str]:
        return self._normalize_leagues()

    def determine_game_status(self, game) -> base.GameStatus:
        if game.game == len(self.get_games_played):
            return base.GameStatus.NotStarted, base.GameResult.NOT_PLAYED
        elif game.game < len(self.get_games_played):
            return base.GameStatus.NotStarted, base.GameResult.NOT_PLAYED
        else:
            if game.result == "Loss" or game.result == "OTL":
                return base.GameStatus.Finished, base.GameResult.LOSS
            else:
                return base.GameStatus.Finished, base.GameResult.WIN
            

    def _normalize_schedule(self):
        schedule = []
        if self.api == API.SPORTSIPY:
            for game in self.api_result.get_schedule:
                game_status, result = self.determine_game_status(game)
                schedule.append(
                    base.Game(
                        team=self.api_result.get_team,
                        timestamp=game.datetime,
                        status=game_status,
                        opposing_team=game.opponent_name,
                        result=result,
                        score=base.Score(
                            team=game.goals_scored,
                            opposing_team=game.goals_allowed
                        ) if base.GameStatus.Finished else None
                    )
                )
        return schedule

    @property
    def get_schedule(self):
        return self._normalize_schedule()

    def _normalize_games_played(self) -> List[str]:
        if self.api == API.APISPORTS:
            return [game[0] for game in self.api_result.get_games_played]
        elif self.api == API.SPORTSIPY:
            return [game.opponent_name for game in self.api_result.get_games_played]
    
    @property
    def get_games_played(self) -> List[str]:
        return self._normalize_games_played()

    def _normalize_wins(self) -> List[str]:
        if self.api == API.APISPORTS:
            return [game[0] for game in self.api_result.get_wins]
        elif self.api == API.SPORTSIPY:
            return [game[0] for game in self.api_result.get_wins]
    @property
    def get_wins(self) -> List[str]:
        return self._normalize_wins()
    
    def _normalize_win_percentage(self) -> float:
        if self.api == API.APISPORTS:
            nums = [float(percent[1]) for percent in self.get_wins_percentage]
            return mean(nums)
        elif self.api == API.SPORTSIPY:
            return self.api_result.get_wins_percentage
    @property
    def get_wins_percentage(self) -> float:
        return self._normalize_win_percentage()
    
    def _normalize_losses(self) -> List[str]:
        if self.api == API.APISPORTS:
            return [game[0] for game in self.api_result.get_losses]
        elif self.api == API.SPORTSIPY:
            return [game[0] for game in self.api_result.get_losses]
    
    @property
    def get_losses(self) -> List[str]:
        return self._normalize_losses()
    
    def _normalize_loss_percentage(self) -> float:
        if self.api == API.APISPORTS:
            nums = [float(percent[1]) for percent in self.get_loss_percentage]
            return mean(nums)
        elif self.api == API.SPORTSIPY:
            return self.api_result.get_loss_percentage

    @property
    def get_loss_percentage(self) -> float:
        return self._normalize_loss_percentage()

    def _normalize_game_ids(self) -> Optional[List[int]]:
        if not hasattr(self.api_result, "game_ids"):
            return
        return self.api_result.game_ids

    @property 
    def get_game_ids(self) -> Optional[List[int]]:
        return self._normalize_game_ids()
    
    def _normalize_timestamps(self) -> List[Tuple[str, datetime]]:
        if self.api == API.SPORTSIPY:
            return self.api_result.get_timestamps

    @property
    def get_timestamps(self):
        return self._normalize_timestamps()
    
    def _normalize_versus(self):
        if self.api == API.SPORTSIPY:
            return [game for game in self.api_result.get_versus]
    @property
    def get_versus(self):
        return self._normalize_versus()
    
    def _normalize_status(self) -> List[Tuple[str, str]]:
        if self.api == API.SPORTSIPY:
            return self.api_result.get_status
        # Check if FT, VS, IN, etc
        elif self.api == API.APISPORTS:
            return self.api_result.get_status
    @property
    def get_status(self):
        return self._normalize_status()
    
    @property
    def get_scores(self):
        return self.api_result.game_result
    
    def get_specific_score(self, game_id):
        return self.api_result.game_result.get(game_id)