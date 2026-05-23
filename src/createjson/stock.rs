use crate::createjson::ui;
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
            symbol: "AAPL".to_owned(),
        }
    }
}

fn pick_api() -> StockApi {
    ui::info("Finnhub is the only supported provider (free tier — register at finnhub.io).");
    StockApi::Finnhub
}

pub fn configure() -> Result<StockOptions, String> {
    ui::section("Stock");
    ui::hint("Polls a single ticker. Add this option again for multiple symbols.");

    let api = pick_api();
    let api_key = ui::read_required("Finnhub API key");
    let symbol = ui::read_line_default("Ticker symbol (uppercased)", "AAPL")
        .trim()
        .to_uppercase();

    ui::success(&format!("Stock — {symbol} via {}", api.get_api()));
    Ok(StockOptions {
        run: true,
        api,
        api_key: Some(api_key),
        symbol,
    })
}

pub fn summary_line(opts: &StockOptions) -> String {
    format!("stock: {} ({})", opts.symbol, opts.api.get_api())
}
