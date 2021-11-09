from dataclasses import dataclass
from lib.run import Runner, Caller
from typing import List, Tuple, Optional
from enum import Enum
from datetime import datetime

class API(Enum):
    APISPORTS = 0
    SPORTSIPY = 1
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
    position: int
    logo: str
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
    opposing_team: str
    result: GameResult
    homeoraway: str
    score: Score = None


