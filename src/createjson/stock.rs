use log::info;
use oledlib::api::StockApi;
use oledlib::serde_helpers::null_string_as_none;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct StockOptions {
    pub run: bool,
    pub api: StockApi,
    #[serde(default, deserialize_with = "null_string_as_none")]
    pub api_key: Option<String>,
    pub symbol: String,
}
impl Default for StockOptions {
    fn default() -> Self {
        StockOptions {
            run: true,
            api: StockApi::Finnhub,
            api_key: None,
            symbol: "fb".to_owned(),
        }
    }
}

fn get_stock_api_key() -> String {
    println!("You Entered a api that requires an API Key");
    println!("Please enter Key now -> ");
    oledlib::get_input().unwrap_or_else(|| "No Key".to_string())
}
fn get_stock_api() -> Result<StockApi, &'static str> {
    info!("For now, the only api is finnhub");
    Ok(StockApi::Finnhub)
}
fn get_symbol() -> Result<String, &'static str> {
    println!("Please enter symbol for stock -> ");
    match oledlib::get_input() {
        Some(input) => Ok(input),
        _ => Err("No input"),
    }
}
pub fn configure() -> Result<StockOptions, &'static str> {
    info!("In stock configuration");
    println!("[stock]: Do you want to use the default config?? (y/n)");
    match oledlib::get_input() {
        Some(input) => match &*input.to_lowercase() {
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
        },
        None => Err("Problem while figuring"),
    }
}
