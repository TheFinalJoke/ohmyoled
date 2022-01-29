use oledlib::api::StockApi;
use oledlib;
use log::{info};

#[derive(Debug)]
pub struct StockOptions {
    pub run: bool,
    pub api: StockApi,
    pub symbol: String,
}
impl Default for StockOptions {
    fn default() -> Self {
        StockOptions {
            run: true,
            api: StockApi::Finnhub,
            symbol: "fb".to_owned(),
        }
    }
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