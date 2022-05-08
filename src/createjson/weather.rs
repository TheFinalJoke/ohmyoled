use log::info;
use oledlib::api;
use json;
use pyo3::{Python, PyObject, IntoPy};
use pyo3::types::{PyDict, IntoPyDict};

#[derive(Debug)]
pub enum WeatherFormat {
    IMPERIAL,
    METRIC,
}
impl WeatherFormat {
    fn get_format(&self) -> String {
        match self {
            WeatherFormat::IMPERIAL => "imperial".to_string(),
            WeatherFormat::METRIC => "metric".to_string(),
        }
    }
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
impl IntoPy<PyObject> for WeatherOptions {
    fn into_py(self, py: Python) -> PyObject {
        self.run.into_py(py)

    }
}
impl IntoPyDict for WeatherOptions {
    fn into_py_dict(self, py: Python) -> &PyDict {
        let result = PyDict::new(py);
        result.set_item("run", self.run).unwrap();
        result.set_item("api", self.api.get_api()).unwrap_or_default();
        result.set_item("api_key", self.match_api_key()).unwrap_or_default();
        result.set_item("current_location", self.current_location).unwrap();
        result.set_item("city", self.match_city()).unwrap();
        result.set_item("weather_format", self.unwrap_weather_format()).unwrap();
        result.into()
    }
}
impl WeatherOptions {
    pub fn match_api_key(&self) -> String {
        match &self.api_key {
            Some(key) => key.to_string(),
            None => "".to_string(),
        }
    }
    pub fn match_city(&self) -> String {
        match &self.city {
            Some(city) => city.to_string(),
            None => "".to_string(),
        }
    }
    pub fn unwrap_weather_format(&self) -> String {
        match &self.weather_format {
                Some(format) => format.get_format(),
                None => "null".to_string(),
            }
    }
    pub fn convert_to_json(&self) -> json::JsonValue {
        json::object!{
            "run": self.run,
            "api": match &self.api {
                api::WeatherApi::Nws => "nws".to_string(),
                api::WeatherApi::Openweather => "openweather".to_string(),
            },
            "api_key": match &self.api_key {
                Some(key) => key,
                None => "null"
            },
            "current_location": self.current_location,
            "city": match &self.city {
                Some(city) => city,
                None => "null",
            },
            "weather_format": match &self.weather_format {
                Some(format) => format.get_format(),
                None => "null".to_string(),
            }
        }
    }
    pub fn from_json(weather_json: &json::JsonValue) -> Self {
        Self {
            run: weather_json["run"].as_bool().unwrap(),
            api: {
                match weather_json["api"].as_str().unwrap() {
                    "nws" => api::WeatherApi::Nws,
                    "openweather" => api::WeatherApi::Openweather,
                    _ => api::WeatherApi::Nws
                }
            },
            api_key: {
                match weather_json["api_key"].as_str().unwrap() {
                    "null" => None,
                    key => Some(key.to_string())
                }
            },
            current_location: weather_json["current_location"].as_bool().unwrap(),
            city: {
                match weather_json["city"].as_str().unwrap() {
                "null" => None,
                city => Some(city.to_string())
                }
            },
            weather_format: {
                match weather_json["weather_format"].as_str().unwrap() {
                    "null" => None,
                    "imperal" => Some(WeatherFormat::IMPERIAL),
                    "metric" => Some(WeatherFormat::METRIC),
                    _ => Some(WeatherFormat::IMPERIAL),
                }
            }
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
pub fn config_format() -> Option<WeatherFormat> {
    loop {
        println!("What Weather Format, Imperial or Metric? (I, M)");
        let format = match oledlib::get_input().unwrap().to_lowercase().as_str() {
            "i" => Some(WeatherFormat::IMPERIAL),
            "m" => Some(WeatherFormat::METRIC),
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
