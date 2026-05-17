use log::info;
use oledlib::teams;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SportOptions {
    pub run: bool,
    pub sport: teams::SportsTypes,
    pub team_logo: teams::Logo,
}
impl Default for SportOptions {
    fn default() -> Self {
        Self {
            run: true,
            sport: teams::SportsTypes::BASEBALL,
            team_logo: teams::Logo {
                name: "Chicago Cubs".to_string(),
                sportsdb_leagueid: 4424,
                url: "https://www.thesportsdb.com/images/media/team/badge/wxbe071521892391.png"
                    .to_string(),
                sport: teams::SportsTypes::BASEBALL,
                shorthand: "CHC".to_string(),
                apisportsid: 6,
                sportsdbid: 135269,
                sportsipyid: None,
            },
        }
    }
}

pub fn configure_sport() -> teams::SportsTypes {
    loop {
        teams::SportsTypes::print_apis();
        let sport_choice = match oledlib::get_input().unwrap().to_lowercase().as_str() {
            "baseball" => teams::SportsTypes::BASEBALL,
            "basketball" => teams::SportsTypes::BASKETBALL,
            "hockey" => teams::SportsTypes::HOCKEY,
            "football" => teams::SportsTypes::FOOTBALL,
            _ => {
                println!("Incorrect sport");
                continue;
            }
        };
        return sport_choice;
    }
}
pub fn team_choice(sport: &teams::SportsTypes) -> teams::Logo {
    loop {
        println!("Choose your Team, -> name of team)");
        teams::print_teams(sport);
        let str_input = oledlib::get_input().unwrap();
        let team: Result<teams::Logo, String> = teams::validate(str_input, sport);
        return match team {
            Ok(t) => t,
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };
    }
}

pub fn configure() -> Result<SportOptions, String> {
    info!("In Sports Configuration");
    println!("[sport]: Do you want to use the default config?? (y/n)");
    match oledlib::get_input() {
        Some(input) => match &*input.to_lowercase() {
            "y" => Ok(SportOptions::default()),
            "n" => {
                let sport_choice: teams::SportsTypes = configure_sport();
                let team_choosen: teams::Logo = team_choice(&sport_choice);
                Ok(SportOptions {
                    run: true,
                    sport: sport_choice,
                    team_logo: team_choosen,
                })
            }
            _ => {
                info!("That is a wrong input");
                Err("That is a wrong input".to_owned())
            }
        },
        None => Err("Problem while figuring".to_owned()),
    }
}
