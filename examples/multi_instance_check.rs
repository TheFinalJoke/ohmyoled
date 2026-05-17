//! Verify that the same config section can carry one *or* many entries.

use oledlib::modules::registry::RegistryConfig;

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
    }}
  ],
  "golf": {"run": true, "tour": "pga"},
  "f1": [{"run": true}],
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
    println!("golf:    {} instance(s)", cfg.golf.len());
    println!("f1:      {} instance(s)", cfg.f1.len());
    println!("stock:   {} instance(s)", cfg.stock.len());
    println!("weather: {} instance(s)", cfg.weather.len());
    println!();
    for (i, s) in cfg.sport.iter().enumerate() {
        println!("  sport[{i}]: {:?} -> {}", s.sport, s.team_logo.name);
    }
    for (i, s) in cfg.stock.iter().enumerate() {
        println!("  stock[{i}]: {}", s.symbol);
    }
}
