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
    /// Flips on the historical 1D/1M/1Y line-chart tile in addition
    /// to the live price tile. Defaults to false so existing configs
    /// keep parsing unchanged.
    #[serde(default)]
    pub chart: bool,
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,
}

impl Default for StockOptions {
    fn default() -> Self {
        StockOptions {
            run: true,
            api: StockApi::Finnhub,
            api_key: None,
            symbol: "AAPL".to_owned(),
            chart: false,
            cache_ttl_secs: None,
        }
    }
}

fn pick_api() -> StockApi {
    let slug = ui::choose(
        "Provider",
        &[
            ("finnhub", "Equity tickers — free tier, requires API key"),
            ("coingecko", "Crypto coin ids — free, no key"),
        ],
        "finnhub",
    );
    StockApi::str_to_api(slug)
}

pub fn configure() -> Result<StockOptions, String> {
    ui::section("Stock");
    ui::hint("Polls a single ticker. Add this option again for multiple symbols.");

    let api = pick_api();
    // Finnhub needs an API key; CoinGecko's public tier is unauth.
    let api_key = match api {
        StockApi::Finnhub => Some(ui::read_required("Finnhub API key")),
        StockApi::Coingecko => None,
    };
    let symbol_prompt = match api {
        StockApi::Finnhub => "Ticker symbol (uppercased)",
        StockApi::Coingecko => "CoinGecko coin id (e.g. bitcoin, ethereum)",
    };
    let raw_symbol = ui::read_line_default(symbol_prompt, "AAPL");
    let symbol = match api {
        StockApi::Finnhub => raw_symbol.trim().to_uppercase(),
        StockApi::Coingecko => raw_symbol.trim().to_lowercase(),
    };

    let chart = ui::read_yes_no(
        "Also enable the 1D/1M/1Y historical chart tile?",
        false,
    );
    let cache_ttl_secs = ui::read_cache_ttl_secs();

    let chart_tag = if chart { " + chart" } else { "" };
    ui::success(&format!("Stock — {symbol} via {}{chart_tag}", api.get_api()));
    Ok(StockOptions {
        run: true,
        api,
        api_key,
        symbol,
        chart,
        cache_ttl_secs,
    })
}

pub fn summary_line(opts: &StockOptions) -> String {
    let chart = if opts.chart { " + chart" } else { "" };
    format!("stock: {} ({}){}", opts.symbol, opts.api.get_api(), chart)
}
