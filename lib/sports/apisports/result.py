from lib.sports.sportbase import SportResultBase

import json

class SportApiResult(SportResultBase):
    def __init__(self, api) -> None:
        super().__init__()
        self.api_type = ""
        self.api = api
        self.main_sport = api
        self._sport = api['sport']
        if len(self.main_sport['standings']['errors']) != 0:
            self._error = self.set_error()
        else:
            self._error = self.set_error()
            self.standings = self.build_standings()
            self._length = len(self.standings)
            self._positions = [(team.get('name'), team.get('position')) for team in self.standings]
            self._leagues =  [(team.get('name'), team.get('league')) for team in self.standings]
            self._games_played = [(team.get('name'), team.get('games').get('played')) for team in self.standings]
            self._wins = [(team.get('name'), team['games']['win']['total']) for team in self.standings]
            self._wins_percentage = [(team.get('name'), team['games']['win']['percentage']) for team in self.standings]
            self._losses = [(team.get('name'), team['games']['lose']['total']) for team in self.standings]
            self._loss_percentage = [(team.get('name'), team['games']['lose']['percentage']) for team in self.standings]            
            self.next_game = self.build_nextgame()
            self._game_ids = [game.get('game_id') for game in self.next_game]
            self._timestamps = [(game.get('game_id'), game.get('timestamp')) for game in self.next_game]
            self._teams = [(game.get('game_id'), game.get('teams')) for game in self.next_game]
            self._vs = [(game.get('game_id'), (game['teams']['home']['name'], game['teams']['away']['name'])) for game in self.next_game]
            self._status = [(game.get('game_id'), game.get('status')) for game in self.next_game]
            self._game_result = {game.get('game_id'): game.get('score') for game in self.next_game}

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
        return f"Sport(\n{joined})"

    def build_standings(self):
        #counter = 0
        position = []
        regular_season_check = (
            "MLB - Regular Season", 
            "NBA - Regular Season",
            "NHL - Regular Season",
            "NFL - Regular Season"
        )
        # Can Be Empty Must try and except for that
        for pos in self.main_sport['standings'].get('response')[0]:
            if not pos.get('stage') in regular_season_check:
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
                'teams': game['teams'],
                'score': game['scores']
            })
        return main

    def set_error(self):
        if isinstance(self.main_sport['standings']['errors'], list):
            return True, ""
        else:
            return False, self.main_sport['standings']['errors']['requests']
    
    @property
    def get_sport(self):
        return self._sport

    @property
    def get_error(self):
        return self._error
    
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
    
    @property
    def get_scores(self):
        return self._game_result
    
    def get_specific_score(self, game_id):
        return self._game_result.get(game_id)
    
