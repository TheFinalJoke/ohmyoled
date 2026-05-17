use log::info;
use oledlib::api;
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherFormat {
    Imperial,
    Metric,
}
impl WeatherFormat {
    #[allow(dead_code)]
    pub fn get_format(&self) -> String {
        match self {
            WeatherFormat::Imperial => "imperial".to_string(),
            WeatherFormat::Metric => "metric".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WeatherOptions {
    pub run: bool,
    pub api: api::WeatherApi,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub api_key: Option<String>,
    pub current_location: bool,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub city: Option<String>,
    #[serde(default)]
    pub weather_format: Option<WeatherFormat>,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub current_location_api_key: Option<String>,
}
impl Default for WeatherOptions {
    fn default() -> Self {
        WeatherOptions {
            run: true,
            api: api::WeatherApi::Nws,
            api_key: None,
            current_location: true,
            city: None,
            weather_format: Some(WeatherFormat::Imperial),
            current_location_api_key: None,
        }
    }
}

fn get_weather_api_key() -> String {
    println!("You Entered a api that requires an API Key");
    println!("Please enter Key now -> ");
    oledlib::get_input().unwrap_or_else(|| "No Key".to_string())
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
        "y" => {
            println!("Enter Api key (ipinfo) ->");
            let input: Option<String> = oledlib::get_input();
            api::WeatherLocationData {
                current_location: true,
                zipcode: None,
                city_and_state: None,
                current_location_api_key: input,
            }
        }
        "n" => {
            println!("Enter zipcode ->");
            let input: Option<String> = oledlib::get_input();
            api::WeatherLocationData {
                current_location: false,
                zipcode: Some(input.unwrap().parse::<i32>().unwrap()),
                city_and_state: None,
                current_location_api_key: None,
            }
        }
        _ => {
            println!("Bad configuration Using default");
            api::WeatherLocationData {
                current_location: true,
                zipcode: None,
                city_and_state: None,
                current_location_api_key: None,
            }
        }
    }
}
pub fn config_format() -> Option<WeatherFormat> {
    loop {
        println!("What Weather Format, Imperial or Metric? (I, M)");
        let format = match oledlib::get_input().unwrap().to_lowercase().as_str() {
            "i" => Some(WeatherFormat::Imperial),
            "m" => Some(WeatherFormat::Metric),
            _ => {
                println!("Invalid format, Try again..");
                continue;
            }
        };
        return format;
    }
}
pub fn configure() -> Result<WeatherOptions, String> {
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
                    current_location_api_key: location.current_location_api_key,
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
