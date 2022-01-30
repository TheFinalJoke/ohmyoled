pub trait API {
    fn get_api(&self) -> &'static str;
}
#[derive(Debug)]
pub enum WeatherApi {
    Nws,
    Openweather,
}
impl API for WeatherApi {
    fn get_api(&self) -> &'static str {
        match self {
            WeatherApi::Nws => "nws",
            WeatherApi::Openweather => "openweather",
        }
    }
}
pub struct WeatherLocationData {
    pub current_location: bool,
    pub zipcode: Option<i32>,
    pub city_and_state: Option<String>,
}
pub struct WeatherApiType {
    pub api: WeatherApi,
    pub api_key: Option<String>,
}

#[derive(Debug)]
pub enum StockApi {
    Finnhub,
}
impl API for StockApi {
    fn get_api(&self) -> &'static str {
        match self {
            StockApi::Finnhub => "finnhub",
        }
    }
}

pub enum SportApi {
    Sportsipy,
    ApiSports,
}
impl API for SportApi {
    fn get_api(&self) -> &'static str {
        match self {
            SportApi::ApiSports => "api-sports",
            SportApi::Sportsipy => "sportsipy",
        }
    }
}
pub struct SportApiType {
    pub api: SportApi,
    pub api_key: Option<String>,
}
