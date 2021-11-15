from dataclasses import dataclass
from lib.run import Runner, Caller
from typing import List, Tuple, Optional, Dict
from enum import Enum
from lib.sports.logo import Logo
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
