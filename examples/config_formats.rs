//! Verify the three on-disk config formats parse to equivalent values.
//!
//! Run with `cargo run --example config_formats`. Reads
//! `examples/configs/ohmyoled.{json,yaml,toml}`, deserializes each into
//! `RegistryConfig`, and prints a short summary.

use oledlib::modules::registry::{RegistryConfig, SportSection};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("examples");
    p.push("configs");
    p.push(name);
    p
}

fn load_as_json_value(path: &PathBuf) -> serde_json::Value {
    let contents = std::fs::read_to_string(path).expect("read fixture");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "json" => serde_json::from_str(&contents).expect("parse json"),
        "yaml" | "yml" => serde_yml::from_str(&contents).expect("parse yaml"),
        "toml" => toml::from_str(&contents).expect("parse toml"),
        other => panic!("unknown extension: {other}"),
    }
}

fn summarize(cfg: &RegistryConfig) -> String {
    let sport_kinds: Vec<&str> = cfg
        .sport
        .iter()
        .map(|s| match s {
            SportSection::Basketball { .. } => "basketball",
            SportSection::Baseball { .. } => "baseball",
            SportSection::Football { .. } => "football",
            SportSection::Hockey { .. } => "hockey",
            SportSection::Golf { .. } => "golf",
            SportSection::F1 { .. } => "f1",
        })
        .collect();
    format!(
        "time={} weather={} stock={} sport={:?} iss={} quake={} aurora={} flights={}",
        cfg.time.len(),
        cfg.weather.len(),
        cfg.stock.len(),
        sport_kinds,
        cfg.iss.len(),
        cfg.quake.len(),
        cfg.aurora.len(),
        cfg.flights.len()
    )
}

fn main() {
    let json_v = load_as_json_value(&fixture("ohmyoled.json"));
    let yaml_v = load_as_json_value(&fixture("ohmyoled.yaml"));
    let toml_v = load_as_json_value(&fixture("ohmyoled.toml"));

    let j: RegistryConfig = serde_json::from_value(json_v).expect("json -> RegistryConfig");
    let y: RegistryConfig = serde_json::from_value(yaml_v).expect("yaml -> RegistryConfig");
    let t: RegistryConfig = serde_json::from_value(toml_v).expect("toml -> RegistryConfig");

    let sj = summarize(&j);
    let sy = summarize(&y);
    let st = summarize(&t);
    println!("json: {sj}");
    println!("yaml: {sy}");
    println!("toml: {st}");

    assert_eq!(sj, sy, "json/yaml diverge");
    assert_eq!(sj, st, "json/toml diverge");
    println!("all three formats produce equivalent RegistryConfig");
}
