//! Verify the three on-disk config formats parse to equivalent values.
//!
//! Run with `cargo run --example config_formats`. Reads
//! `examples/configs/ohmyoled.{json,yaml,toml}`, deserializes each into
//! `RegistryConfig`, and prints a short summary.

use oledlib::modules::registry::{EinkRegistryConfig, RegistryConfig, SleepConfig, SportSection};
use std::path::PathBuf;

/// Pulls the top-level `eink` block, mirroring how `main.rs` parses it.
#[derive(serde::Deserialize, Default)]
struct EinkTop {
    #[serde(default)]
    eink: EinkRegistryConfig,
}

/// Pulls the top-level `sleep` block, mirroring how `main.rs` parses it.
#[derive(serde::Deserialize, Default)]
struct SleepTop {
    #[serde(default)]
    sleep: SleepConfig,
}

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
        "time={} weather={} stock={} sport={:?} iss={} quake={} aurora={} flights={} launch={} hass={} pihole={}",
        cfg.time.len(),
        cfg.weather.len(),
        cfg.stock.len(),
        sport_kinds,
        cfg.iss.len(),
        cfg.quake.len(),
        cfg.aurora.len(),
        cfg.flights.len(),
        cfg.launch.len(),
        cfg.hass.len(),
        cfg.pihole.len()
    )
}

fn main() {
    let json_v = load_as_json_value(&fixture("ohmyoled.json"));
    let yaml_v = load_as_json_value(&fixture("ohmyoled.yaml"));
    let toml_v = load_as_json_value(&fixture("ohmyoled.toml"));

    let j: RegistryConfig = serde_json::from_value(json_v.clone()).expect("json -> RegistryConfig");
    let y: RegistryConfig = serde_json::from_value(yaml_v.clone()).expect("yaml -> RegistryConfig");
    let t: RegistryConfig = serde_json::from_value(toml_v.clone()).expect("toml -> RegistryConfig");

    let sj = summarize(&j);
    let sy = summarize(&y);
    let st = summarize(&t);
    println!("json: {sj}");
    println!("yaml: {sy}");
    println!("toml: {st}");

    assert_eq!(sj, sy, "json/yaml diverge");
    assert_eq!(sj, st, "json/toml diverge");
    println!("all three formats produce equivalent RegistryConfig");

    // The independent `eink` block must also parse equivalently in all three.
    let ej: EinkTop = serde_json::from_value(json_v).expect("json -> EinkTop");
    let ey: EinkTop = serde_json::from_value(yaml_v).expect("yaml -> EinkTop");
    let et: EinkTop = serde_json::from_value(toml_v).expect("toml -> EinkTop");
    let eink_summary = |e: &EinkTop| {
        format!(
            "enabled={} model={} weather={}",
            e.eink.enabled,
            e.eink.model,
            e.eink.modules.weather.len()
        )
    };
    let (sej, sey, set) = (eink_summary(&ej), eink_summary(&ey), eink_summary(&et));
    println!("eink json: {sej}");
    println!("eink yaml: {sey}");
    println!("eink toml: {set}");
    assert_eq!(sej, sey, "eink json/yaml diverge");
    assert_eq!(sej, set, "eink json/toml diverge");
    println!("all three formats produce equivalent eink config");

    // The independent `sleep` block must also parse equivalently in all three.
    let json_v2 = load_as_json_value(&fixture("ohmyoled.json"));
    let yaml_v2 = load_as_json_value(&fixture("ohmyoled.yaml"));
    let toml_v2 = load_as_json_value(&fixture("ohmyoled.toml"));
    let zj: SleepTop = serde_json::from_value(json_v2).expect("json -> SleepTop");
    let zy: SleepTop = serde_json::from_value(yaml_v2).expect("yaml -> SleepTop");
    let zt: SleepTop = serde_json::from_value(toml_v2).expect("toml -> SleepTop");
    let sleep_summary = |s: &SleepTop| {
        format!(
            "enabled={} sleep={:?} wake={:?} start={:?} end={:?} windows={}",
            s.sleep.enabled,
            s.sleep.sleep,
            s.sleep.wake,
            s.sleep.start,
            s.sleep.end,
            s.sleep.windows.len()
        )
    };
    let (szj, szy, szt) = (sleep_summary(&zj), sleep_summary(&zy), sleep_summary(&zt));
    println!("sleep json: {szj}");
    println!("sleep yaml: {szy}");
    println!("sleep toml: {szt}");
    assert_eq!(szj, szy, "sleep json/yaml diverge");
    assert_eq!(szj, szt, "sleep json/toml diverge");
    println!("all three formats produce equivalent sleep config");
}
