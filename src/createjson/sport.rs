use oledlib::{api, teams};
use log::{info};
use pyo3::{Python, PyObject, PyResult};
use pyo3::types::{PyDict};
use json;

#[derive(Debug)]
pub struct SportOptions {
    pub run: bool,
    pub api: api::SportApi,
    pub api_key: Option<String>,
    pub sport: teams::SportsTypes,
    pub team_logo: teams::Logo,
}
impl Default for SportOptions {
    fn default() -> Self {
        Self {
            run: true,
            api: api::SportApi::Sportsipy,
            api_key: None,
            sport: teams::SportsTypes::BASEBALL,
            team_logo: teams::Logo{
                name: "Chicago Cubs".to_string(), 
                sportsdb_leagueid: 4424, 
                url: "https://www.thesportsdb.com/images/media/team/badge/wxbe071521892391.png".to_string(), 
                sport: teams::SportsTypes::BASEBALL, 
                shorthand: "CHC".to_string(), 
                apisportsid: 6, 
                sportsdbid: 135269, 
                sportsipyid: None
            },
        }
    }
}
impl SportOptions {
    pub fn convert_to_json(&self) -> json::JsonValue {
        json::object!{
            "run": self.run,
            "api": self.api.get_api(),
            "api_key": match &self.api_key {
                Some(key) => key,
                None => "null"
            },
            "sport": self.sport.get_sport_str(),
            "team_logo": self.team_logo.to_json(),
        }
    }
    pub fn from_json(sport_json: &json::JsonValue) -> Self {
        let sj = &sport_json["sport"];
        Self {
            run: sj["run"].as_bool().unwrap(),
            api: api::SportApi::api_to_str(sj["api"].to_string()),
            api_key: {
                match sj["api_key"].as_str().unwrap() {
                    "null" => None,
                    key => Some(key.to_string())
                }
            },
            sport: teams::SportsTypes::str_to_sport(sj["sport"].to_string()),
            team_logo: teams::Logo::from_json(&sj["team_logo"])
        }
    }
    pub fn to_python_dict(&self, py: Python) -> PyResult<PyObject> {
        let result = PyDict::new(py);
        result.set_item("run", self.run)?;
        result.set_item("api", self.api.get_api())?;
        result.set_item("api_key", match self.api_key.clone().unwrap().as_str(){
            "null" => "null",
            key => key,
        })?;
        result.set_item("sport", self.sport.get_sport_str())?;
        result.set_item("team_logo", self.team_logo.to_python_dict(py)?)?;
        Ok(result.into())
    }
}

fn get_sport_api_key() -> String {
    println!("You Entered a api that requires an API Key");
    println!("Please enter Key now -> ");
    let api_key = match oledlib::get_input() {
        Some(input) => input,
        None => "No Key".to_string(),
    };
    api_key
}
fn get_sport_api() -> api::SportApiType {
    loop {
        println!("Please enter api, Sportsipy(Default) (sptipy),");
        println!("APISPORTS Api (apisport) , Requires an Api -> ");
        let api_map = match &*oledlib::get_input().unwrap().to_lowercase() {
            "sptipy" => api::SportApiType {
                api: api::SportApi::Sportsipy,
                api_key: None,
            },
            "apisport" => api::SportApiType {
                api: api::SportApi::ApiSports,
                api_key: Some(get_sport_api_key()),
            },
            _ => {
                println!("Not a Valid API, Try Again");
                continue;
            }
        };
        return api_map;
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
                continue
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
            Err(e) => {println!("{}", e); continue;}
        }
    }   
}

pub fn configure() -> Result<SportOptions, String> {
    info!("In Sports Configuration");
    println!("[sport]: Do you want to use the default config?? (y/n)");
    match oledlib::get_input() {
        Some(input) => match &*input.to_lowercase() {
            "y" => Ok(SportOptions::default()),
            "n" => {
                let api_decision: api::SportApiType = get_sport_api();
                let sport_choice: teams::SportsTypes = configure_sport();
                let team_choosen: teams::Logo = team_choice(&sport_choice);
                Ok(
                    SportOptions {
                        run: true,
                        api: api_decision.api,
                        api_key: api_decision.api_key,
                        sport: sport_choice,
                        team_logo: team_choosen,
                    }
                    )
            }
            _ => {
                info!("That is a wrong input");
                Err("That is a wrong input".to_owned())
            }
        },
        None => Err("Problem while figuring".to_owned()),
    }
}