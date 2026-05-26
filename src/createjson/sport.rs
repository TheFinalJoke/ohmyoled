use crate::createjson::ui;
use oledlib::teams;
use serde_json::{json, Value};

/// Default team-sport entry used when the user accepts defaults.
fn default_entry() -> Value {
    json!({
        "run": true,
        "sport": "baseball",
        "team_logo": teams::Logo {
            name: "Chicago Cubs".to_string(),
            sportsdb_leagueid: 4424,
            url: "https://www.thesportsdb.com/images/media/team/badge/wxbe071521892391.png".to_string(),
            sport: teams::SportsTypes::BASEBALL,
            shorthand: "CHC".to_string(),
            apisportsid: 6,
            sportsdbid: 135269,
            sportsipyid: None,
        },
        "cache_ttl_secs": serde_json::Value::Null,
    })
}

fn pick_sport() -> teams::SportsTypes {
    let slug = ui::choose(
        "Which team sport?",
        &[
            ("baseball", "MLB — ESPN scoreboard"),
            ("basketball", "NBA — ESPN scoreboard"),
            ("football", "NFL — ESPN scoreboard"),
            ("hockey", "NHL — ESPN scoreboard"),
        ],
        "baseball",
    );
    teams::SportsTypes::str_to_sport(slug)
}

fn pick_team(sport: &teams::SportsTypes) -> teams::Logo {
    ui::info(&format!(
        "Available {} teams (type the full name as listed):",
        sport.get_sport_str()
    ));
    teams::print_teams(sport);
    loop {
        let input = ui::read_required("Team name");
        match teams::validate(input, sport) {
            Ok(logo) => return logo,
            Err(e) => ui::warn(&e),
        }
    }
}

pub fn configure() -> Result<Value, String> {
    ui::section("Sport — Team (MLB / NBA / NFL / NHL)");
    if ui::read_yes_no("Use the default team (Chicago Cubs)?", false) {
        let entry = default_entry();
        ui::success("Sport — Chicago Cubs (MLB) added");
        return Ok(entry);
    }
    let sport_kind = pick_sport();
    let logo = pick_team(&sport_kind);
    let cache_ttl_secs = ui::read_cache_ttl_secs();
    let entry = json!({
        "run": true,
        "sport": sport_kind.get_sport_str(),
        "team_logo": logo,
        "cache_ttl_secs": cache_ttl_secs,
    });
    ui::success(&format!(
        "Sport — {} ({}) added",
        logo_display(&entry),
        sport_kind.get_sport_str().to_uppercase()
    ));
    Ok(entry)
}

fn logo_display(entry: &Value) -> String {
    entry
        .get("team_logo")
        .and_then(|t| t.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string()
}

pub fn summary_line(entry: &Value) -> String {
    let sport = entry
        .get("sport")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_uppercase();
    let team = logo_display(entry);
    format!("sport: {} — {team}", sport.to_lowercase())
}
