//! Multi-format config loader/writer.
//!
//! Supports `.json`, `.yaml`/`.yml`, and `.toml` based on file extension.
//! All formats deserialize into the same `serde_json::Value` so the rest of
//! the app stays format-agnostic. Files without an extension (or with an
//! unknown extension) fall through JSON → YAML → TOML in that order.
//!
//! ```no_run
//! use serde_json::Value;
//! let v: Value = config_io::load("/etc/ohmyoled/ohmyoled.yaml").unwrap();
//! config_io::write("/tmp/copy.toml", &v).unwrap();
//! ```
//! (Module is binary-local — the example above is illustrative.)
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("write {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yml::Error),
    #[error("toml decode: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml encode: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("could not parse {path} as json, yaml, or toml")]
    UnknownFormat { path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Yaml,
    Toml,
}

impl Format {
    /// Guess the format from the file extension. Returns `None` for unknown
    /// or missing extensions (caller falls back to format-sniffing).
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = Path::new(path).extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }
}

/// Load `path` and return it as a generic `serde_json::Value`.
///
/// Format is chosen from the file extension; unknown extensions try JSON
/// → YAML → TOML in order.
pub fn load(path: &str) -> Result<serde_json::Value, ConfigError> {
    let contents = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_string(),
        source,
    })?;
    match Format::from_path(path) {
        Some(Format::Json) => Ok(serde_json::from_str(&contents)?),
        Some(Format::Yaml) => Ok(serde_yml::from_str(&contents)?),
        Some(Format::Toml) => Ok(toml::from_str(&contents)?),
        None => sniff(&contents).ok_or_else(|| ConfigError::UnknownFormat {
            path: path.to_string(),
        }),
    }
}

/// Write `value` to `path` in the format implied by the path's extension.
/// Unknown/missing extensions default to JSON.
pub fn write(path: &str, value: &serde_json::Value) -> Result<(), ConfigError> {
    let fmt = Format::from_path(path).unwrap_or(Format::Json);
    let contents = match fmt {
        Format::Json => serde_json::to_string_pretty(value)?,
        Format::Yaml => serde_yml::to_string(value)?,
        Format::Toml => toml::to_string_pretty(&strip_nulls(value))?,
    };
    std::fs::write(path, contents).map_err(|source| ConfigError::Write {
        path: path.to_string(),
        source,
    })
}

/// TOML has no null, so serializing a config with unset optionals (blank
/// `cache_ttl_secs`, optional strings) fails with "unsupported unit type".
/// Recursively drop null-valued object entries instead — every optional field
/// is `#[serde(default)]`, so an omitted key loads back as the same `None`.
pub fn strip_nulls(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(m) => serde_json::Value::Object(
            m.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), strip_nulls(v)))
                .collect(),
        ),
        serde_json::Value::Array(a) => {
            serde_json::Value::Array(a.iter().map(strip_nulls).collect())
        }
        other => other.clone(),
    }
}

fn sniff(contents: &str) -> Option<serde_json::Value> {
    if let Ok(v) = serde_json::from_str(contents) {
        return Some(v);
    }
    if let Ok(v) = serde_yml::from_str(contents) {
        return Some(v);
    }
    if let Ok(v) = toml::from_str(contents) {
        return Some(v);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"time":{"run":true,"color":[255,255,255]}}"#;
    const YAML: &str = "time:\n  run: true\n  color: [255, 255, 255]\n";
    const TOML: &str = "[time]\nrun = true\ncolor = [255, 255, 255]\n";

    #[test]
    fn format_from_extension() {
        assert_eq!(Format::from_path("a.json"), Some(Format::Json));
        assert_eq!(Format::from_path("a.yaml"), Some(Format::Yaml));
        assert_eq!(Format::from_path("a.yml"), Some(Format::Yaml));
        assert_eq!(Format::from_path("a.toml"), Some(Format::Toml));
        assert_eq!(Format::from_path("a.txt"), None);
        assert_eq!(Format::from_path("a"), None);
    }

    #[test]
    fn all_three_formats_produce_equivalent_value() {
        let j: serde_json::Value = serde_json::from_str(JSON).unwrap();
        let y: serde_json::Value = serde_yml::from_str(YAML).unwrap();
        let t: serde_json::Value = toml::from_str(TOML).unwrap();
        assert_eq!(j, y);
        assert_eq!(j, t);
    }

    #[test]
    fn sniff_falls_through_in_order() {
        assert_eq!(sniff(JSON).unwrap(), sniff(YAML).unwrap());
        assert_eq!(sniff(YAML).unwrap(), sniff(TOML).unwrap());
        // YAML accepts almost any single-line string as a scalar, so the
        // "nothing parses" case has to be something all three reject.
        // Mismatched braces fail every parser.
        assert!(sniff("{ unclosed: [").is_none());
    }

    #[test]
    fn round_trip_through_disk() {
        let tmp = std::env::temp_dir();
        let v: serde_json::Value = serde_json::from_str(JSON).unwrap();
        for (name, expected_fmt) in [
            ("ohmyoled_cfgio.json", Format::Json),
            ("ohmyoled_cfgio.yaml", Format::Yaml),
            ("ohmyoled_cfgio.toml", Format::Toml),
        ] {
            let path = tmp.join(name);
            let path_str = path.to_str().unwrap();
            write(path_str, &v).unwrap();
            assert_eq!(Format::from_path(path_str), Some(expected_fmt));
            let parsed = load(path_str).unwrap();
            assert_eq!(parsed, v);
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn toml_write_drops_null_optionals() {
        // TOML can't hold nulls; unset optionals are dropped on write and load
        // back as `None` through the sections' serde defaults.
        let tmp = std::env::temp_dir().join("ohmyoled_cfgio_nulls.toml");
        let path = tmp.to_str().unwrap();
        let v = serde_json::json!({
            "time": {"run": true, "cache_ttl_secs": null},
            "sleep": {"enabled": true, "start": null, "end": null}
        });
        write(path, &v).expect("toml write with nulls succeeds");
        let parsed = load(path).unwrap();
        assert_eq!(parsed["time"]["run"], serde_json::json!(true));
        assert!(parsed["time"].get("cache_ttl_secs").is_none());
        assert_eq!(parsed["sleep"]["enabled"], serde_json::json!(true));
        let _ = std::fs::remove_file(&tmp);
    }
}
