enum SportsApiOptions {
    SPORTSIPY,
    APISPORTS,
}
enum SportsTypes {
    BASKETBALL,
    BASEBALL,
    FOOTBALL,
    HOCKEY,
}
struct SportOptions {
    run: bool,
    api: SportsApiOptions,
    sport: SportsTypes,
    team_id: String
}