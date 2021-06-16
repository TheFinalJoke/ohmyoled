from typing import List, Tuple
from lib.run import Runner, Caller
from lib.sports.football import Football
from lib.sports.baseball.baseball import Baseball
from lib.sports.basketball.basketball import Basketball
from lib.sports.hockey.hockey import Hockey
import os 
import sys

class SportApi(Runner):
    def __init__(self, config):
        super().__init__(config)
        self.sport = self.config['sport']
        try:
            if "sport_token" in self.config['basic']:
                self.token = self.config['basic'].get('sport_token')
            else:
                self.token = os.environ['SPORTTOKEN']
        except KeyError:
            self.logger.critical("No Sport Token")
            sys.exit("No Sport Token")
        self.headers = {'x-apisports-key': 'ebb2c44c416b9a9a0b538e2d73c7dbe6'}
    
    def parse_args(self):
        return super().parse_args()
    
    async def run(self):
        self.logger.info("Running Sports")
        sport_data = {"Sport": {}}
        if 'football' == self.sport.get('sport').lower():
            self.logger.debug("Running football data")
            football = Football(self.token, self.config['sport'], self.headers)
            football_return = await football.run()
            sport_data['Sport'].update({'football': football_return})
        elif 'baseball' == self.sport.get('sport').lower():
            self.logger.debug('Running baseball data')
            baseball = Baseball(self.token, self.config['sport'], self.headers)
            baseball_return = await baseball.run()
            sport_data['Sport'].update({'baseball': baseball_return})
        elif 'basketball' == self.sport.get('sport').lower():
            self.logger.debug('Got basketball in config')
            basketball = Basketball(self.token, self.config['sport'], self.headers)
            basketball_return = await basketball.run()
            sport_data['Sport'].update({'baseketball': basketball_return})
        elif 'hockey' == self.sport.get('sport').lower():
            self.logger.debug('Got Hockey from Config')
            hockey = Hockey(self.token, self.config['sport'], self.headers)
            hockey_return = await hockey.run()
            sport_data['Sport'].update({'hockey': hockey_return})
        return sport_data

class Sport(Caller):
    def __init__(self, api) -> None:
        super().__init__()
        self.api = api
        self.full_sport = self.api['Sport']
        self.sport = [*self.full_sport]
        self.main_sport = self.full_sport[self.sport[0]]
        if 'standings' in self.main_sport:
            self.standings = self.build_standings()
            self._length = len(self.standings)
            self._positions = [(team.get('name'), team.get('position')) for team in self.standings]
            self._leagues =  [(team.get('name'), team.get('league')) for team in self.standings]
            self._games_played = [(team.get('name'), team.get('games').get('played')) for team in self.standings]
            self._wins = [(team.get('name'), team['games']['win']['total']) for team in self.standings]
            self._wins_percentage = [(team.get('name'), team['games']['win']['percentage']) for team in self.standings]
            self._losses = [(team.get('name'), team['games']['lose']['total']) for team in self.standings]
            self._loss_percentage = [(team.get('name'), team['games']['lose']['percentage']) for team in self.standings]
        if 'next_game' in self.main_sport:
            self.next_game = self.build_nextgame()
            self._game_ids = [game.get('game_id') for game in self.next_game]
            self._timestamps = [(game.get('game_id'), game.get('timestamp')) for game in self.next_game]
            self._teams = [(game.get('game_id'), game.get('teams')) for game in self.next_game]
            self._vs = [(game.get('game_id'), (game['teams']['home']['name'], game['teams']['away']['name'])) for game in self.next_game]
            self._status = [(game.get('game_id'), game.get('status')) for game in self.next_game]
            
            
    def build_standings(self):
        #counter = 0
        position = []
        # Can Be Empty Must try and except for that
        for pos in self.main_sport['standings'].get('response')[0]:
            if pos.get('stage') != "MLB - Regular Season":
                continue
            position.append({'name': pos.get('team').get('name'),
                    'position': pos.get('position'),
                    'league': pos.get('group').get('name'),
                    'games': pos.get('games')
                    })
        return position

    def build_nextgame(self):
        main = []
        for game in self.main_sport['next_game'].get('response'):
            main.append({
                'game_id': game.get('id'),
                'timestamp': game.get('timestamp'),
                'status': game['status']['short'],
                'teams': game['teams']
            })
        return main

    @property 
    def get_length_position_teams(self):
        return len(self.standings)
    
    @property
    def get_standings(self):
        return self.standings
    
    @property 
    def get_position_teams(self):
        return self._positions
    
    @property
    def get_leagues(self):
        return self._leagues
    
    @property
    def get_games_played(self):
        return self._games_played
    
    @property
    def get_wins(self):
        return self._wins
    
    @property
    def get_wins_percentage(self):
        return self._wins_percentage
    
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