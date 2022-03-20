pub mod stock;
pub mod time;
pub mod sport;
pub mod weather;
use oledlib;

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
impl MatrixOptions {
    pub fn convert_to_json(&self) -> json::JsonValue {
        json::object! {
                "chain_length": self.chain_length,
                "parallel": self.parallel,
                "brightness": self.brightness,
                "oled_slowdown": self.oled_slowdown,
                "fail_on_error": self.fail_on_error,
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

pub fn create_json(dev_mode: bool) -> json::JsonValue {
    let mut modules = vec![];
    let mut config_vec = vec![];
    if dev_mode {
        let dev_json = "{\"matrix_options\":{\"chain_length\":1,\"parallel\":1,\"brightness\":50,\"oled_slowdown\":3,\"fail_on_error\":false},\"time\":{\"run\":true,\"color\":[255,255,255],\"time_format\":\"null\",\"timezone\":\"null\"},\"weather\":{\"run\":true,\"api\":\"openweather\",\"api_key\":\"80ce462129470ef2f5d55e6f65d32eef\",\"current_location\":true,\"city\":\"null\",\"weather_format\":\"imperial\"},\"stock\":{\"run\":true,\"api\":\"finnhub\",\"api_key\":\"sandbox_c091niv48v6tm13rlepg\",\"symbol\":\"fb\"},\"sport\":{\"run\":true,\"api\":\"api-sports\",\"api_key\":\"ebb2c44c416b9a9a0b538e2d73c7dbe6\",\"sport\":\"basketball\",\"team_logo\":{\"name\":\"Dallas Mavericks\",\"sportsdb_leagueid\":4387,\"url\":\"https://www.thesportsdb.com/images/media/team/badge/yqrxrs1420568796.png\",\"sport\":\"basketball\",\"shorthand\":\"DAL\",\"apisportsid\":138,\"sportsdbid\":134875,\"sportsipyid\":0}}}";
        let unwrapedjson = json::parse(dev_json).unwrap();
        return unwrapedjson
    }
    loop {
        main_menu();
        match oledlib::get_input() {
            Some(input) => {
                match input.as_str() {
                    "1" => {
                        if modules.contains(&1) {
                            println!("Time is already enabled")
                        } else {
                            modules.push(1);
                        }
                    }
                    "2" => {
                        if modules.contains(&2) {
                            println!("Weather is already enabled")
                        } else {
                            modules.push(2);
                        }
                    }
                    "3" => {
                        if modules.contains(&3) {
                            println!("Stock is already enabled")
                        } else {
                            modules.push(3);
                        }
                    }
                    "4" => {
                        if modules.contains(&4) {
                            println!("Sports are already enabled")
                        } else {
                            modules.push(4);
                        }
                    }
                    "c" => break,
                    "q" => {
                        println!("Quitting the create_json, Did not write any new configuration");
                        return json::object!{
                        "failure": true,
                    }
                },
                    _ => break,
                };
            }
            None => return json::object!{
                "failure": true
            },
        };
    }

    for module in modules {
        match module {
            // will probably have to store or add to the json outright
            1 => {
               config_vec.push(("time", time::configure().convert()));
            }
            2 => {
                config_vec.push(("weather", match weather::configure() {
                    Ok(weather_config) => weather_config.convert_to_json(),
                    Err(e) => {
                        println!("Something happend with weather config, {}, Setting value to null", e);
                        json::object!{
                            failure: true
                        }
                    }
                }));
            }
            3 => {
                config_vec.push(("stock", match stock::configure() {
                    Ok(stock_config) => stock_config.convert_to_json(),
                    Err(e) => {
                        println!("Something happend with stock config, {}, setting value to null", e);
                        json::object!{
                            failure: true
                        }
                    }
                }))
            }
            4 => {
                config_vec.push(("sport", match sport::configure() {
                    Ok(sport_config) => sport_config.convert_to_json(),
                    Err(e) => {println!("Something happend with sport config, {}, setting value to null", e);
                   json::object!{
                       failure: true
                   }}
                }));
            }
            _ => break,
        }
    }

    let moptions = MatrixOptions::default().convert_to_json();
    let mut main_json = json::object!{
        "matrix_options": moptions
    };
    for module in config_vec {
        main_json.insert(module.0, module.1).unwrap();
    }
    return main_json;
}
