use oledlib::api;
use log::{info};
#[derive(Debug)]
pub enum SportsTypes {
    BASKETBALL,
    BASEBALL,
    FOOTBALL,
    HOCKEY,
}
impl SportsTypes {
    pub fn print_apis() {
        println!("Choose From Apis");
        println!("Basketball");
        println!("Baseball");
        println!("Football");
        println!("Hockey");
    }
    pub fn print_hockey_teams() {
        println!(
        "Arizona Coyotes 
        Boston Bruins 
        Buffalo Sabres
        Calgary Flames
        Carolina Hurricanes
        Chicago Blackhawks
        Colorado Avalanche
        Columbus Blue Jackets
        Dallas Stars ,
        Detroit Red Wings
        Edmonton Oilers
        Florida Panthers
        Los Angeles Kings
        Minnesota Wild
        Montreal Canadiens
        Nashville Predators
        New Jersey Devils
        New York Islanders
        New York Rangers
        Ottawa Senators
        Philadelphia Flyers
        Pittsburgh Penguins
        San Jose Sharks
        Seattle Kraken
        St. Louis Blues
        Tampa Bay Lightning
        Toronto Maple Leafs
        Vancouver Canucks
        Vegas Golden Knights
        Washington Capitals
        Winnipeg Jets"
    }
    pub fn print_football_teams() {
        println!(
         "Arizona Cardinals
          Atlanta Falcons
          Baltimore Ravens
          Buffalo BillsCarolina Panthers
          Chicago BearsCincinnati Bengals
          Cleveland Browns
          Dallas Cowboys
          Denver Broncos 
          Detroit Lions
          Green Bay Packers
          Houston Texans 
          Indianapolis Colts
          Jacksonville Jaguars
          Kansas City Chiefs
          Las Vegas Raiders
          Los Angeles Chargers
          Los Angeles Rams
          Miami Dolphins 
          Minnesota Vikings
          New England Patriots
          New Orleans Saints
          New York Giants
          New York Jets
          Philadelphia Eagles
          Pittsburgh Steelers
          San Francisco 49ers
          Seattle Seahawks
          Tampa Bay Buccaneers
          Tennessee Titans
          Washington Football Team"
        );
    }
    pub fn print_baseball_teams() {
        println!(
 "San Francisco Giants,
 Los Angeles Dodgers,
 Tampa Bay Rays,
 Houston Astros,
 Milwaukee Brewers,
 Chicago White Sox,
 Boston Red Sox,
 New York Yankees,
 Toronto Blue Jays,
 St. Louis Cardinals,
 Seattle Mariners,
 Atlanta Braves,
 Oakland Athletics,
 Cincinnati Reds,
 Philadelphia Phillies,
 Cleveland Indians,
 San Diego Padres,
 Detroit Tigers,
 New York Mets,
 Los Angeles Angels,
 Colorado Rockies,
 Kansas City Royals,
 Minnesota Twins,
 Chicago Cubs,
 Miami Marlins,
 Washington Nationals,
 Pittsburgh Pirates,
 Texas Rangers,
 Arizona Diamondbacks,
 Baltimore Orioles");
    }
pub fn print_basketball_teams() {
    println!(
        "Atlanta Hawks
        Boston Celtics
        Brooklyn Nets
        Charlotte Hornets
        Chicago Bulls
        Cleveland Cavaliers
        Dallas Mavericks
        Denver Nuggets
        Detroit Pistons
        Golden State Warriors
        Houston Rockets
        Indiana Pacers
        Los Angeles Clippers
        Los Angeles Lakers
        Memphis Grizzlies
        Miami Heat
        Milwaukee Bucks
        Minnesota Timberwolves
        New Orleans Pelicans
        New York Knicks
        Oklahoma City Thunder
        Orlando Magic
        Philadelphia 76ers
        Phoenix Suns
        Portland Trail Blazers
        Sacramento Kings
        San Antonio Spurs
        Toronto Raptors
        Utah Jazz 
        Washington Wizards"
    );
}
}

#[derive(Debug)]
struct SportOptions {
    run: bool,
    api: api::SportApi,
    api_key: Option<String> 
    sport: SportsTypes,
    team_id: String,
}
impl Default for SportOptions {
    fn default() -> Self {
        SportOptions {
            run: true,
            api: api::SportApi::Sportsipy,
            api_key: None
            sport: SportsTypes::BASEBALL,
            team_id: "Chicago Cubs".to_string(),
        }
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
fn get_sport_api() -> api::WeatherApiType {
    loop {
        println!("Please enter api, Sportsipy(Default) (sptipy),");
        println!("OpenWeather Api (apisport) , Requires an Api -> ");
        let api_map = match &*oledlib::get_input().unwrap().to_lowercase() {
            "sptipy" => api::SportApiType {
                api: api::SportApiType::Sportsipy,
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
pub fn configure_sport() -> SportsTypes {
    loop {
        SportsTypes::print_apis();
        let sport_choice = match oledlib::get_input().unwrap().to_lowercase().as_str() {
            "baseball" => SportsTypes::BASEBALL,
            "basketball" => SportsTypes::BASKETBALL,
            "hockey" => SportsTypes::HOCKEY,
            "football" => SportsTypes::FOOTBALL,
            _ => {
                println!("Incorrect sport");
                continue
            } 
        }
        return sport_choice;
    }
}
pub get_team(team: String) -> Result<String, String>{

}
pub fn configure() -> Result<SportOptions, String> {
    info!("In Sports Configuration");
    println!("[sport]: Do you want to use the default config?? (y/n)");
    match oledlib::get_input() {
        Some(input) => match &*input.to_lowercase() {
            "y" => Ok(SportOptions::default()),
            "n" => {
                let api_decision: api::SportApiType = get_sport_api();
                let sport_choice: SportsTypes = configure_sport();
                Ok(
                    SportOptions {
                        run: true,
                        api: api_decision.api,
                        api_key: api_decision.api_key,
                        sport: sport_choice,

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