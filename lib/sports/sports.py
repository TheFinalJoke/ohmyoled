from typing import List, Tuple, get_args
from lib.sports.apisports.apisports import ApiSports
from lib.run import Runner, Caller
import os
import json
import sys

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
        return

class SportFinal(Caller):
    """
    Final Normalized Object that goes to 
    The Sport Matrix.
    """
    def __init__(self, api_result) -> None:
        # Any Object from any api object
        self.api_result = api_result

    @property
    def get_sport(self):
        return self.api_result.get_sport
    @property
    def get_error(self):
        return self.api_result.get_error
    @property 
    def get_length_position_teams(self):
        return len(self.api_result.standings)
    
    @property
    def get_standings(self):
        return self.api_result.standings
    
    @property 
    def position_teams(self):
        return self.api_result.position_teams
    
    @property
    def get_leagues(self):
        return self.api_result.get_leagues
    
    @property
    def get_games_played(self):
        return self.api_result.games_played
    
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