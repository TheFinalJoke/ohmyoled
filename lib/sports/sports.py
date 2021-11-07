from os import name
from lib.sports.apisports.apisports import ApiSports
from lib.sports.sportsipy.sportsipy import SportsipyAPI
from lib.run import Runner, Caller
from typing import Tuple, List, Dict
from lib.sports.sportbase import API
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

class SportFinal(Caller):
    """
    Final Normalized Object that goes to 
    The Sport Matrix. This is the final object
    * NO MATTER What the return types should not change

    """
    def __init__(self, api_result) -> None:
        # Any Object from any api object
        self.api_result = api_result
        breakpoint()
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
        return f"SportFinal(\n{joined})"

    @property
    def api(self) -> str:
        return self.api_result.get_api

    def _normalize_sport(self) -> str:
        """
        All apis Should return a string in 
        thier result object
        """
        return self.api_result.get_sport

    @property
    def get_sport(self):
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
    
    def _normalize_positions(self) -> base.SportStandings:
        if self.api == API.APISPORTS:
            standings = [
                base.Team(
                    name=team[0], 
                    position=team[1]
                ) for team in self.api_result.position_teams]
            return base.SportStandings(positions=standings)
        elif self.api == API.SPORTSIPY:
            standings = [
                base.Team(
                    name=team[0].name,
                    position=team[1],
                ) for team in self.api_result.position_teams]
            return base.SportStandings(positions=standings)
    @property 
    def position_teams(self):
        return self._normalize_positions()
    
    def _normalize_leagues(self):
        if not self.api_result.get_leagues:
            return self.api_result.get_leagues
        return {league[1] for league in self.api_result.get_leagues}
    
    @property
    def get_leagues(self):
        return self._normalize_leagues()
    
    def _normalize_games_played(self):
        if self.api == API.APISPORTS:
            return [game[0] for game in self.api_result.get_games_played]
        elif self.api == API.SPORTSIPY:
            return
    @property
    def get_games_played(self):
        return self._normalize_games_played()
    ###
    @property
    def get_wins(self):
        return self.api_result.get_wins
    
    @property
    def get_wins_percentage(self):
        return self.api_result.win_percentage
    
    @property
    def get_losses(self):
        return self.api_result.losses
    
    @property
    def get_loss_percentage(self):
        return self.api_result.loss_percentage

    @property 
    def get_game_ids(self):
        return self.api_result.game_ids
    
    @property
    def get_timestamps(self):
        return self.api_result.timestamps
    
    @property
    def get_teams(self):
        return self.api_result.teams
    
    @property
    def get_versus(self):
        return self.api_result.vs
    
    @property
    def get_status(self):
        return self.api_result.status
    
    @property
    def get_scores(self):
        return self.api_result.game_result
    
    def get_specific_score(self, game_id):
        return self.api_result.game_result.get(game_id)