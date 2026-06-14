use crate::createjson::tui::field::{FieldDef, FieldKind};
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

/// TUI form schema. `api_key` is shown/required only for Finnhub; `symbol` is
/// case-folded by provider in `form_module::section_to_value`.
pub fn fields() -> Vec<FieldDef> {
    vec![
        FieldDef::new(
            "api",
            "Provider",
            "Equity tickers (Finnhub) or crypto coin ids (CoinGecko).",
            FieldKind::Enum {
                default: "finnhub",
                choices: &[
                    ("finnhub", "Equities — free tier, requires API key"),
                    ("coingecko", "Crypto coin ids — free, no key"),
                ],
            },
        ),
        FieldDef::new(
            "api_key",
            "Finnhub API key",
            "Required for Finnhub; CoinGecko's public tier is unauthenticated.",
            FieldKind::Text { default: "" },
        )
        .when(|f| f.enum_slug("api") == Some("finnhub")),
        FieldDef::new(
            "symbol",
            "Symbol",
            "Ticker (AAPL) for Finnhub, or coin id (bitcoin) for CoinGecko.",
            FieldKind::Text { default: "AAPL" },
        ),
        FieldDef::new(
            "chart",
            "Historical chart",
            "Also show the 1D/1M/1Y line-chart tile.",
            FieldKind::Bool { default: false },
        ),
        FieldDef::new(
            "cache_ttl_secs",
            "Cache TTL (secs)",
            super::CACHE_TTL_HELP,
            FieldKind::CacheTtl,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use crate::createjson::tui::form_module;

    #[test]
    fn coingecko_drops_key_and_lowercases_symbol() {
        let mut form = form_module::default_form("stock");
        // switch api to coingecko (index 1), set symbol "BitCoin"
        if let crate::createjson::tui::field::FieldValue::Enum(sel) = &mut form.values[0] {
            *sel = 1;
        }
        form.values[2] =
            crate::createjson::tui::field::FieldValue::Input(tui_input::Input::new("BitCoin".into()));
        let v = form_module::section_to_value("stock", &form).unwrap();
        assert_eq!(v["symbol"], serde_json::json!("bitcoin"));
        // Canonicalized through StockOptions, so the absent key becomes null.
        assert!(v["api_key"].is_null(), "coingecko has no api key");
    }

    #[test]
    fn finnhub_requires_key_and_uppercases_symbol() {
        let mut form = form_module::default_form("stock");
        // finnhub default, but api_key blank -> strict error
        assert!(form_module::section_to_value("stock", &form).is_err());
        // fill the key
        form.values[1] =
            crate::createjson::tui::field::FieldValue::Input(tui_input::Input::new("KEY123".into()));
        form.values[2] =
            crate::createjson::tui::field::FieldValue::Input(tui_input::Input::new("aapl".into()));
        let v = form_module::section_to_value("stock", &form).unwrap();
        assert_eq!(v["symbol"], serde_json::json!("AAPL"));
        assert_eq!(v["api_key"], serde_json::json!("KEY123"));
    }
}
