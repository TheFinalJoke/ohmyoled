use log::info;
use oledlib::api;
use std::collections::HashMap;

#[derive(Debug)]
pub enum WeatherFormat {
    IMPERIAL,
    METRIC,
}
#[derive(Debug)]
pub struct WeatherOptions {
    pub run: bool,
    pub api: api::WeatherApi,
    pub api_key: Option<String>,
    pub current_location: bool,
    pub city: Option<String>,
    pub weather_format: Option<WeatherFormat>,
}
impl Default for WeatherOptions {
    fn default() -> Self {
        WeatherOptions {
            run: true,
            api: api::WeatherApi::Nws,
            api_key: None,
            current_location: true,
            city: None,
            weather_format: Some(WeatherFormat::IMPERIAL),
        }
    }
}
fn get_weather_api_key() -> String {
    println!("You Entered a api that requires an API Key");
    println!("Please enter Key now -> ");
    let api_key = match oledlib::get_input() {
        Some(input) => input,
        None => "No Key".to_string(),
    };
    api_key
}
fn get_weather_api() -> api::WeatherApiType {
    loop {
        println!("Please enter api, National Weather Service(nws),");
        println!("OpenWeather Api (openweather) , Requires an Api -> ");
        let api_map = match &*oledlib::get_input().unwrap().to_lowercase() {
            "nws" => api::WeatherApiType {
                api: api::WeatherApi::Nws,
                api_key: None,
            },
            "openweather" => api::WeatherApiType {
                api: api::WeatherApi::Openweather,
                api_key: Some(get_weather_api_key()),
            },
            _ => {
                println!("Not a Valid API, Try Again");
                continue;
            }
        };
        return api_map;
    }
}
pub fn configure_location() -> api::WeatherLocationData {
    println!("Do you want to use the current location??(Default) (y/n)");
    match oledlib::get_input().unwrap().to_lowercase().as_str() {
        "y" => api::WeatherLocationData {
            current_location: true,
            zipcode: None,
            city_and_state: None,
        },
        "n" => {
            println!("Enter zipcode ->");
            let input: Option<String> = oledlib::get_input();
            api::WeatherLocationData {
                current_location: false,
                zipcode: Some(input.unwrap().parse::<i32>().unwrap()),
                city_and_state: None,
            }
        }
        _ => {
            println!("Bad configuration Using default");
            api::WeatherLocationData {
                current_location: true,
                zipcode: None,
                city_and_state: None,
            }
        }
    }
}
pub fn config_format() -> WeatherFormat {
    loop {
        println!("What Weather Format, Imperial or Metric? (I, M)");
        let format = match oledlib::get_input().unwrap().to_lowercase().as_str() {
            "i" => WeatherFormat::IMPERIAL,
            "m" => WeatherFormat::METRIC,
            _ => {
                println!("Invalid format, Try again..");
                continue;
            }
        };
        return format;
    }
}
pub fn configure() {
    info!("In weather configuration");
    println!("[weather]: Do you want to use the default config?? (y/n)");
    match oledlib::get_input() {
        Some(input) => match &*input.to_lowercase() {
            "y" => Ok(WeatherOptions::default()),
            "n" => {
                let api_decision: api::WeatherApiType = get_weather_api();
                let location: api::WeatherLocationData = configure_location();
                Ok(WeatherOptions {
                    run: true,
                    api: api_decision.api,
                    api_key: api_decision.api_key,
                    current_location: location.current_location,
                    city: location.city_and_state,
                    weather_format: config_format(),
                })
            }
            _ => {
                info!("That is a wrong input");
                Err("That is a wrong input")
            }
        },
        None => Err("Problem while figuring"),
    }
}
