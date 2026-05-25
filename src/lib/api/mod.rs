//! API collectors and the trait every new module implements.
//!
//! See [`collector::Collector`] for the contract, [`http::get_json`] for the
//! shared HTTP helper, and [`error::ApiError`] for the error type.

pub mod aurora;
pub mod collector;
pub mod error;
pub mod f1;
pub mod flights;
pub mod golf;
pub mod http;
pub mod iss;
pub mod launch;
pub mod quake;
pub mod sport;
pub mod stock;
pub mod weather;

pub use collector::Collector;
pub use error::ApiError;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherApi {
    Nws,
    Openweather,
    Accuweather,
    #[serde(rename = "pirate", alias = "pirateweather", alias = "pirate_weather")]
    Pirate,
}
impl WeatherApi {
    pub fn get_api(&self) -> &'static str {
        match self {
            WeatherApi::Nws => "nws",
            WeatherApi::Openweather => "openweather",
            WeatherApi::Accuweather => "accuweather",
            WeatherApi::Pirate => "pirate",
        }
    }
}
pub struct WeatherLocationData {
    pub current_location: bool,
    pub zipcode: Option<i32>,
    pub city_and_state: Option<String>,
    pub current_location_api_key: Option<String>,
}
pub struct WeatherApiType {
    pub api: WeatherApi,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StockApi {
    Finnhub,
    Coingecko,
}
impl StockApi {
    pub fn get_api(&self) -> String {
        match self {
            StockApi::Finnhub => "finnhub".to_string(),
            StockApi::Coingecko => "coingecko".to_string(),
        }
    }
    pub fn str_to_api(api_str: String) -> Self {
        match api_str.as_str() {
            "finnhub" => Self::Finnhub,
            "coingecko" => Self::Coingecko,
            _ => Self::Finnhub,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SportApi {
    Sportsipy,
    #[serde(rename = "api-sports")]
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
