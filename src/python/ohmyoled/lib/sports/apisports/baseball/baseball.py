from ohmyoled.lib.run import Runner
from datetime import datetime
from ohmyoled.lib.sports.apisports.result import SportApiResult
from ohmyoled.lib.sports.logo import logo_map, Logo
from datetime import datetime
import ohmyoled.lib.sports.sportbase as base


class Baseball(Runner):
    def __init__(self, token, config, headers):
        self.config = config
        self.token = token
        self.headers = headers
        self.year = datetime.now().year

    def parse_args(self):
        try:
            if self.config["type"] != "" or not self.config["type"]:
                args = self.config["type"]
            else:
                args = ("standings", "next_game")
            return args
        except KeyError:
            args = ("standings", "next_game")
        return args

    def url_builder(self, args, logo):
        self.headers.update({"x-apisports-host": "v1.baseball.api-sports.io"})
        urls = {}
        base = "https://v1.baseball.api-sports.io/"
        urls.update({"standings": base + f"standings?league=1&season=2022"})
        urls.update(
            {
                "next_game": base
                + f"games?team={logo.apisportsid}&league=1&season=2022&timezone=America/Chicago"
            }
        )
        return urls

    async def run(self) -> SportApiResult:
        self.logger.info("Running baseball API")
        parsed = self.parse_args()
        api_data = {}
        logo = logo_map[self.config["team_logo"].get("name")]
        for section, url in self.url_builder(parsed, logo).items():
            api_data.update({section: await self.get_data(url, self.headers)})
        # Check Data
        if api_data["standings"]["errors"] != []:
            raise base.SportApiExceedLimit("Api Exceeded the limit")
        api_data["sport"] = "baseball"
        try:
            schedule = [
                base.Game(
                    team=base.Team(
                        name=logo.name,
                        logo=logo,
                    ),
                    timestamp=datetime.fromtimestamp(game["timestamp"]),
                    status=base.determine_apisports_game_status(
                        game, base.Team(logo.name, logo=logo), api_data["sport"]
                    )[0],
                    opposing_team=base.Team(
                        name=game["teams"]["home"]["name"],
                        logo=logo_map[game["teams"]["home"]["name"]],
                    )
                    if logo.name != game["teams"]["home"]["name"]
                    else base.Team(
                        name=game["teams"]["away"]["name"],
                        logo=logo_map[game["teams"]["away"]["name"]],
                    ),
                    result=base.determine_apisports_game_status(
                        game, base.Team(logo.name, logo=logo), api_data["sport"]
                    )[1],
                    homeoraway="home"
                    if logo.name == game["teams"]["home"]["name"]
                    else "away",
                    score=base.Score(
                        team=game["scores"]["home"]
                        if logo.name == game["teams"]["home"]["name"]
                        else game["scores"]["away"],
                        opposing_team=game["scores"]["home"]
                        if logo.name != game["teams"]["home"]["name"]
                        else game["scores"]["away"],
                    ),
                )
                for game in api_data["next_game"]["response"]
            ]
            standings = [
                base.Team(
                    name=team["team"]["name"],
                    logo=logo_map[team["team"]["name"]],
                    position=team["position"],
                    league=team["group"]["name"],
                )
                for team in api_data["standings"]["response"][0]
                + api_data["standings"]["response"][1]
            ]
            result = base.ModuleResult(
                name=logo.name,
                team=base.Team(
                    name=logo.name,
                    logo=logo,
                ),
                schedule=schedule,
                standings=standings,
                sport=base.SportStructure.Baseball,
                games_played=len(
                    [
                        game
                        for game in schedule
                        if game.status == base.GameStatus.Finished
                        or game.status == base.GameStatus.Overtime
                    ]
                ),
                wins=len(
                    [game for game in schedule if game.result == base.GameResult.WIN]
                ),
                losses=len(
                    [game for game in schedule if game.result == base.GameResult.LOSS]
                ),
            )
            return SportApiResult(result)
        except Exception as e:
            raise base.SportException(e)
