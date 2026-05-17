//! Verify that the same config section can carry one *or* many entries,
//! and that golf/f1 nest under the unified `sport` array discriminated by
//! the inner `"sport"` field.

use oledlib::modules::registry::{RegistryConfig, SportSection};

const MIXED: &str = r#"
{
  "time": {"run": true, "color": [255, 255, 255]},
  "sport": [
    {"run": true, "sport": "basketball", "team_logo": {
      "name": "Boston Celtics", "sportsdb_leagueid": 4387,
      "url": "x", "sport": "basketball", "shorthand": "BOS",
      "apisportsid": 133, "sportsdbid": 134860, "sportsipyid": 0
    }},
    {"run": true, "sport": "hockey", "team_logo": {
      "name": "Boston Bruins", "sportsdb_leagueid": 4380,
      "url": "x", "sport": "hockey", "shorthand": "BOS",
      "apisportsid": 673, "sportsdbid": 134830, "sportsipyid": 0
    }},
    {"run": true, "sport": "golf", "tour": "pga"},
    {"run": true, "sport": "f1"}
  ],
  "stock": [
    {"run": true, "api": "finnhub", "api_key": "x", "symbol": "AAPL"},
    {"run": true, "api": "finnhub", "api_key": "x", "symbol": "MSFT"}
  ]
}
"#;

fn main() {
    let v: serde_json::Value = serde_json::from_str(MIXED).unwrap();
    let cfg: RegistryConfig = serde_json::from_value(v).unwrap();
    println!("time:    {} instance(s)", cfg.time.len());
    println!("sport:   {} instance(s)", cfg.sport.len());
    println!("stock:   {} instance(s)", cfg.stock.len());
    println!("weather: {} instance(s)", cfg.weather.len());
    println!();
    for (i, s) in cfg.sport.iter().enumerate() {
        let label = match s {
            SportSection::Basketball { team_logo, .. } => format!("basketball -> {}", team_logo.name),
            SportSection::Baseball { team_logo, .. } => format!("baseball -> {}", team_logo.name),
            SportSection::Football { team_logo, .. } => format!("football -> {}", team_logo.name),
            SportSection::Hockey { team_logo, .. } => format!("hockey -> {}", team_logo.name),
            SportSection::Golf { tour, .. } => format!("golf -> {tour:?}"),
            SportSection::F1 { .. } => "f1".to_string(),
        };
        println!("  sport[{i}]: {label}");
    }
    for (i, s) in cfg.stock.iter().enumerate() {
        println!("  stock[{i}]: {}", s.symbol);
    }
}
