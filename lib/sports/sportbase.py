from dataclasses import dataclass
from lib.run import Runner, Caller
from typing import List
from enum import Enum

class API(Enum):
    APISPORTS = 0
    SPORTSIPY = 1
class SportBase(Runner):
    pass 
class SportResultBase(Caller):
    pass

@dataclass(repr=True)
class Team():
    name: str
    position: int
    league: str = None
@dataclass(repr=True)
class SportStandings():
    positions: List[Team]

