from dataclasses import dataclass
from ohmyoled.lib.run import Runner, Caller
from typing import List, Tuple, Optional, Dict
from enum import Enum
from ohmyoled.lib.sports.logo import Logo
from datetime import datetime

class ModuleException(Exception):
    pass

class RequestException(ModuleException):
    pass

class API(Enum):
    APISPORTS = 0
    SPORTSIPY = 1
    SPORTSDB = 2

class SportStructure(Enum):
    Hockey = 0
    Baseball = 1
    Football = 2
    Basketball = 3

class GameStatus(Enum):
    NotStarted = 0
    InGame = 1
    Finished = 2
    Overtime = 3

class GameResult(Enum):
    WIN = 0
    LOSS = 1
    TIE = 2
    NOT_PLAYED = 3

class SportBase(Runner):
    pass 

class SportResultBase(Caller):
    pass

@dataclass(repr=True)
class Team():
    name: str
    logo: Logo
    position: int = None
    league: str = None

@dataclass(repr=True)
class SportStandings():
    positions: List[Team]

@dataclass(repr=True)
class Score:
    team: int
    opposing_team: int
@dataclass(repr=True)
class Game:
    team: Team
    timestamp: datetime
    status: GameStatus
    opposing_team: Team
    result: GameResult
    homeoraway: str
    score: Score = None

@dataclass(repr=True)
class BaseModuleResult():
    name: str
    team: Dict[str, str]
    schedule: Dict[str, str]
    standings: Dict[str, str]
    sport: SportStructure

@dataclass(repr=True)
class ModuleResult(BaseModuleResult):
    name: str
    team: Team
    schedule: List[Game]
    standings: List[Team]
    sport: SportStructure
    games_played: int
    wins: int
    losses: int

@dataclass(repr=True)
class HockeyResult(BaseModuleResult):
    name: str
    team: Team
    schedule: List[Game]
    standings: List[Team]
    sport: SportStructure
    games_played: int
    wins: int
    losses: int

@dataclass(repr=True)
class FootballResult(BaseModuleResult):
    name: str
    team: Team
    schedule: List[Game]
    standings: List[Team]
    sport: SportStructure
    games_played: int
    wins: int
    losses: int

@dataclass(repr=True)
class BaseballResult(BaseModuleResult):
    name: str
    team: Team
    schedule: List[Game]
    standings: List[Team]
    sport: SportStructure
    games_played: int
    wins: int
    losses: int

def determine_game_status(team, game) -> Tuple[GameStatus, GameResult]:
    if hasattr(game, "game"):
        game_num = game.game
    elif hasattr(game, "week"):
        game_num = game.week
    if hasattr(team, 'games_played'):
        len_games = team.games_played
    if hasattr(team, "games_finished"):
        len_games = team.games_finished
    if game_num == len_games:
        return GameStatus.NotStarted, GameResult.NOT_PLAYED
    elif game_num > len_games:
        return GameStatus.NotStarted, GameResult.NOT_PLAYED
    else:
        if game.result == "Loss" or game.result == "OTL":
            return GameStatus.Finished, GameResult.LOSS
        else:
            return GameStatus.Finished, GameResult.WIN
def determine_team(game, team):
    if game['teams']['home'] == team.name:
        return True
    return False

def determine_apisports_game_status(game_dict: Dict, team, sport):
    if sport == 'hockey':
        in_game = ['P', 'OT', 'BT', "Q", "HT", "IN"]
        if game_dict['status']['short'] in in_game:
            return GameStatus.InGame, GameResult.NOT_PLAYED 
        elif game_dict['status']['short'] in ('FT', 'AP') and datetime.now().date() == datetime.fromtimestamp(game_dict['timestamp']).date():
            is_team_home_team = determine_team(game_dict, team)
            if is_team_home_team:
                if game_dict['scores']['home'] == game_dict['scores']['away']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home'] < game_dict['scores']['away']:
                    return GameStatus.Finished, GameResult.LOSS
                else:
                    return GameStatus.Finished, GameResult.WIN
            else:
                if game_dict['scores']['home'] == game_dict['scores']['away']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home'] > game_dict['scores']['away']:
                    return GameStatus.Finished, GameResult.WIN
                else:
                    return GameStatus.Finished, GameResult.LOSS
        elif game_dict['status']['short'] in ('FT', 'AP') or datetime.fromtimestamp(game_dict['timestamp']) < datetime.now():
            is_team_home_team = determine_team(game_dict, team)
            if is_team_home_team:
                if game_dict['scores']['home'] == game_dict['scores']['away']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home'] < game_dict['scores']['away']:
                    return GameStatus.Finished, GameResult.LOSS
                else:
                    return GameStatus.Finished, GameResult.WIN
            else:
                if game_dict['scores']['home'] == game_dict['scores']['away']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home'] > game_dict['scores']['away']:
                    return GameStatus.Finished, GameResult.WIN
                else:
                    return GameStatus.Finished, GameResult.LOSS
        else:
            return GameStatus.NotStarted, GameResult.NOT_PLAYED
    elif sport == 'basketball':
        in_game = ['P', 'OT', 'BT', "Q", "HT", "IN"]
        if game_dict['status']['short'] in in_game:
            return GameStatus.InGame, GameResult.NOT_PLAYED 
        elif game_dict['status']['short'] in ('FT', 'AP') and datetime.now().date() == datetime.fromtimestamp(game_dict['timestamp']).date():
            is_team_home_team = determine_team(game_dict, team)
            if is_team_home_team:
                if game_dict['scores']['home']['total'] == game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home']['total'] < game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.LOSS
                else:
                    return GameStatus.Finished, GameResult.WIN
            else:
                if game_dict['scores']['home']['total'] == game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home']['total'] > game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.WIN
                else:
                    return GameStatus.Finished, GameResult.LOSS
        elif game_dict['status']['short'] in ('FT', 'AP') or datetime.fromtimestamp(game_dict['timestamp']) < datetime.now():
            is_team_home_team = determine_team(game_dict, team)
            if is_team_home_team:
                if game_dict['scores']['home']['total'] == game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home']['total'] < game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.LOSS
                else:
                    return GameStatus.Finished, GameResult.WIN
            else:
                if game_dict['scores']['home']['total'] == game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home']['total'] > game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.WIN
                else:
                    return GameStatus.Finished, GameResult.LOSS
        else:
            return GameStatus.NotStarted, GameResult.NOT_PLAYED
    elif sport == "baseball":
        in_game = ['P', 'OT', 'BT', "Q", "HT", "IN"]
        if game_dict['status']['short'] in in_game:
            return GameStatus.InGame, GameResult.NOT_PLAYED 
        elif game_dict['status']['short'] in ('FT', 'AP') and datetime.now().date() == datetime.fromtimestamp(game_dict['timestamp']).date():
            is_team_home_team = determine_team(game_dict, team)
            if is_team_home_team:
                if game_dict['scores']['home']['total'] == game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home']['total'] < game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.LOSS
                else:
                    return GameStatus.Finished, GameResult.WIN
            else:
                if game_dict['scores']['home']['total'] == game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home']['total'] > game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.WIN
                else:
                    return GameStatus.Finished, GameResult.LOSS
        elif game_dict['status']['short'] in ('FT', 'AP') or datetime.fromtimestamp(game_dict['timestamp']) < datetime.now():
            is_team_home_team = determine_team(game_dict, team)
            if is_team_home_team:
                if game_dict['scores']['home']['total'] == game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home']['total'] < game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.LOSS
                else:
                    return GameStatus.Finished, GameResult.WIN
            else:
                if game_dict['scores']['home']['total'] == game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.TIE
                elif game_dict['scores']['home']['total'] > game_dict['scores']['away']['total']:
                    return GameStatus.Finished, GameResult.WIN
                else:
                    return GameStatus.Finished, GameResult.LOSS
        else:
            return GameStatus.NotStarted, GameResult.NOT_PLAYED