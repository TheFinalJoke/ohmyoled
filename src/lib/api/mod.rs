#[derive(Debug)]
pub enum WeatherApi {
    Nws,
    Openweather,
}
impl WeatherApi {
    pub fn get_api(&self) -> &'static str {
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
    pub current_location_api_key: Option<String>
}
pub struct WeatherApiType {
    pub api: WeatherApi,
    pub api_key: Option<String>,
}

#[derive(Debug)]
pub enum StockApi {
    Finnhub,
}
impl StockApi {
    pub fn get_api(&self) -> String {
        match self {
            StockApi::Finnhub => "finnhub".to_string(),
        }
    }
    pub fn str_to_api(api_str: String) -> Self {
        match api_str.as_str() {
            "finnhub" => Self::Finnhub,
            _ => Self::Finnhub,
        }
    }
}
#[derive(Debug)]
pub enum SportApi {
    Sportsipy,
    ApiSports,
}
impl SportApi {
    pub fn get_api(&self) -> String {
        match self {
            SportApi::ApiSports => "api-sports".to_string(),
            SportApi::Sportsipy => "sportsipy".to_string(),
        }
    }
    pub fn api_to_str(api_str: String) -> Self {
        match api_str.as_str() {
            "api-sports" => SportApi::ApiSports,
            "sportsipy" => SportApi::Sportsipy,
            _ => SportApi::Sportsipy,
        }
    }
}
pub struct SportApiType {
    pub api: SportApi,
    pub api_key: Option<String>,
}
