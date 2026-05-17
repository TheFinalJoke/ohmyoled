pub mod sport;
pub mod stock;
pub mod time;
pub mod traits;
pub mod weather;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MatrixOptions {
    pub chain_length: i8,
    pub parallel: i8,
    pub brightness: i32,
    pub oled_slowdown: i32,
    pub fail_on_error: bool,
}
impl Default for MatrixOptions {
    fn default() -> Self {
        Self {
            chain_length: 1,
            parallel: 1,
            brightness: 50,
            oled_slowdown: 3,
            fail_on_error: false,
        }
    }
}

pub fn main_menu() {
    println!("Choose From Modules to customize");
    println!("Select a Number");
    println!("1. Time");
    println!("2. Weather");
    println!("3. Stock");
    println!("4. Sports");
    println!("C. Continue");
    println!("Q. Quit");
}

/// Build the on-disk config interactively (or with `dev_mode = true` for the
/// canned dev config).
pub fn create_json(dev_mode: bool) -> serde_json::Value {
    if dev_mode {
        let dev_json = r#"{"matrix_options":{"chain_length":1,"parallel":1,"brightness":50,"oled_slowdown":3,"fail_on_error":false},"time":{"run":true,"color":[255,255,255],"time_format":"null","timezone":"null"},"weather":{"run":true,"api":"openweather","api_key":"null","current_location":true,"city":"null","weather_format":"imperial"},"stock":{"run":true,"api":"finnhub","api_key":"null","symbol":"fb"},"sport":{"run":true,"sport":"basketball","team_logo":{"name":"Dallas Mavericks","sportsdb_leagueid":4387,"url":"https://www.thesportsdb.com/images/media/team/badge/yqrxrs1420568796.png","sport":"basketball","shorthand":"DAL","apisportsid":138,"sportsdbid":134875,"sportsipyid":0}}}"#;
        return serde_json::from_str(dev_json).expect("dev json must parse");
    }

    let mut modules: Vec<i32> = vec![];
    let mut config = serde_json::Map::new();
    config.insert(
        "matrix_options".to_string(),
        serde_json::to_value(MatrixOptions::default()).unwrap(),
    );

    loop {
        main_menu();
        match oledlib::get_input() {
            Some(input) => match input.as_str() {
                "1" => push_unique(&mut modules, 1, "Time"),
                "2" => push_unique(&mut modules, 2, "Weather"),
                "3" => push_unique(&mut modules, 3, "Stock"),
                "4" => push_unique(&mut modules, 4, "Sports"),
                "c" => break,
                "q" => {
                    println!("Quitting the create_json, Did not write any new configuration");
                    return serde_json::json!({ "failure": true });
                }
                _ => break,
            },
            None => return serde_json::json!({ "failure": true }),
        }
    }

    for module in modules {
        match module {
            1 => {
                config.insert("time".to_string(), serde_json::to_value(time::configure()).unwrap());
            }
            2 => match weather::configure() {
                Ok(w) => {
                    config.insert("weather".to_string(), serde_json::to_value(w).unwrap());
                }
                Err(e) => {
                    println!("weather config failed: {e}; skipping section");
                }
            },
            3 => match stock::configure() {
                Ok(s) => {
                    config.insert("stock".to_string(), serde_json::to_value(s).unwrap());
                }
                Err(e) => {
                    println!("stock config failed: {e}; skipping section");
                }
            },
            4 => match sport::configure() {
                Ok(sp) => {
                    config.insert("sport".to_string(), serde_json::to_value(sp).unwrap());
                }
                Err(e) => {
                    println!("sport config failed: {e}; skipping section");
                }
            },
            _ => break,
        }
    }

    serde_json::Value::Object(config)
}

fn push_unique(modules: &mut Vec<i32>, kind: i32, label: &str) {
    if modules.contains(&kind) {
        println!("{label} is already enabled");
    } else {
        modules.push(kind);
    }
}
