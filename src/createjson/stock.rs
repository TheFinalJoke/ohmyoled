use oledlib::api::StockApi;
use log::{info};
use json;
use pyo3::{PyObject, PyResult, Python};
use pyo3::types::{PyDict};
use pyo3::prelude::*;

#[derive(Debug)]
pub struct StockOptions {
    pub run: bool,
    pub api: StockApi,
    pub api_key: Option<String>,
    pub symbol: String,
}
impl Default for StockOptions {
    fn default() -> Self {
        StockOptions {
            run: true,
            api: StockApi::Finnhub,
            api_key: None, // this will fail because finnhub requires a Api Key
            symbol: "fb".to_owned(),
        }
    }
}
impl StockOptions {
    pub fn convert_to_json(&self) -> json::JsonValue {
        json::object!{
            "run": self.run,
            "api": match &self.api {
                StockApi::Finnhub => "finnhub".to_string()
            },
            "api_key": match &self.api_key {
                Some(key) => key.to_string(),
                None => json::Null.to_string()
            },
            "symbol": self.symbol.as_str(),
        }
    }
    pub fn from_json(stock_json: &json::JsonValue) -> Self {
        let sj = &stock_json["stock"];
        Self {
            run: sj["run"].as_bool().unwrap(),
            api: StockApi::str_to_api(sj["api"].to_string()),
            api_key: match sj["api_key"].as_str().unwrap() {
                "null" => None,
                key => Some(key.to_string())
            },
            symbol: sj["symbol"].to_string(),
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
        result.set_item("symbol", self.symbol.to_string())?;
        Ok(result.into())
    }
}
fn get_stock_api_key() -> String {
    println!("You Entered a api that requires an API Key");
    println!("Please enter Key now -> ");
    let api_key = match oledlib::get_input() {
        Some(input) => input,
        None => "No Key".to_string(),
    };
    api_key
}
fn get_stock_api() -> Result<StockApi, &'static str> {
    info!("For now, the only api is finnhub");
    Ok(StockApi::Finnhub)
}
fn get_symbol() -> Result<String, &'static str> {
    println!("Please enter symbol for stock -> ");
    match oledlib::get_input() {
        Some(input) => Ok(input.to_owned()),
        _ =>  {
            Err("No input")
        },
    }
}
pub fn configure() -> Result<StockOptions, &'static str> {
    info!("In stock configuration");
    println!("[stock]: Do you want to use the default config?? (y/n)");
    match oledlib::get_input() {
        Some(input) => {
            match &*input.to_lowercase() {
                "y" => Ok(StockOptions::default()),
                "n" => Ok(StockOptions {
                    run: true,
                    api: get_stock_api()?,
                    api_key: Some(get_stock_api_key()),
                    symbol: get_symbol()?,
                    }),
                _ => {
                    info!("That is a wrong input");
                    Err("That is a wrong input")
                }
            }
        }
        None => Err("Problem while figuring")
    }
}