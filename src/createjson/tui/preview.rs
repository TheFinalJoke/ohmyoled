//! Config assembly + serialization for the live preview pane and the final
//! save. `build_value` walks the [`App`] state into a `serde_json::Value`,
//! branching on the display target; `render_string` serializes it with the
//! exact three encoders [`crate::config_io::write`] uses, so the preview is
//! byte-faithful to what lands on disk.

use super::app::{App, ConfigFormat, Target};
use super::form_module;
use serde_json::{Map, Value};

/// Section keys that live at the config top level (matrix) or under
/// `eink.modules` (eink), in stable output order.
const SECTION_ORDER: &[&str] = &[
    "time", "weather", "stock", "sport", "iss", "quake", "aurora", "flights", "launch", "hass",
    "pihole",
];

/// Assemble the full config. With `strict = true` any field/section validation
/// failure aborts with the collected `(field id, message)` errors (used on
/// save). With `strict = false` invalid fields are dropped so the preview never
/// goes blank.
pub fn build_value(app: &App, strict: bool) -> Result<Value, Vec<(String, String)>> {
    let mut errs: Vec<(String, String)> = Vec::new();
    let value = match app.target {
        Target::Matrix => build_matrix(app, strict, &mut errs),
        Target::Eink => build_eink(app, strict, &mut errs),
    };
    if strict && !errs.is_empty() {
        Err(errs)
    } else {
        Ok(value)
    }
}

/// Convenience wrapper for the preview pane — never fails.
pub fn preview_value(app: &App) -> Value {
    build_value(app, false).unwrap_or_else(|_| Value::Object(Map::new()))
}

fn build_matrix(app: &App, strict: bool, errs: &mut Vec<(String, String)>) -> Value {
    let mut map = Map::new();
    map.insert(
        "matrix_options".to_string(),
        target_form_value(app, strict, errs),
    );
    for &key in SECTION_ORDER {
        let folded = collect_section(app, key, strict, errs);
        if let Some(v) = folded {
            map.insert(key.to_string(), v);
        }
    }
    Value::Object(map)
}

fn build_eink(app: &App, strict: bool, errs: &mut Vec<(String, String)>) -> Value {
    let mut block = match target_form_value(app, strict, errs) {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    // `eink.modules` is itself a full RegistryConfig, so each section folds the
    // same way as the matrix top level — one instance → object, several → array.
    let mut modules = Map::new();
    for &key in SECTION_ORDER {
        if let Some(v) = collect_section(app, key, strict, errs) {
            modules.insert(key.to_string(), v);
        }
    }
    block.insert("modules".to_string(), Value::Object(modules));
    let mut root = Map::new();
    root.insert("eink".to_string(), Value::Object(block));
    Value::Object(root)
}

/// Serialize the active target form (matrix options or eink-block scalars).
fn target_form_value(app: &App, strict: bool, errs: &mut Vec<(String, String)>) -> Value {
    let result = match app.target {
        Target::Matrix => form_module::matrix_to_value(&app.target_form),
        Target::Eink => form_module::eink_scalars_to_value(&app.target_form),
    };
    match result {
        Ok(v) => v,
        Err(mut e) => {
            if strict {
                errs.append(&mut e);
            }
            // Lossy fallback: best-effort scalars.
            let mut v = app.target_form.to_value_lossy();
            if app.target.is_eink() {
                if let Value::Object(m) = &mut v {
                    m.insert("enabled".to_string(), Value::Bool(true));
                }
            }
            v
        }
    }
}

/// All serialized values for the tiles whose config key is `key`, in instance
/// order. Honors strict/lossy per-instance.
fn section_values(
    app: &App,
    key: &str,
    strict: bool,
    errs: &mut Vec<(String, String)>,
) -> Vec<Value> {
    let mut out = Vec::new();
    for inst in &app.instances {
        if form_module::config_key(inst.kind) != key {
            continue;
        }
        match form_module::section_to_value(inst.kind, &inst.form) {
            Ok(v) => out.push(v),
            Err(mut e) => {
                if strict {
                    errs.append(&mut e);
                } else {
                    out.push(inst.form.to_value_lossy());
                }
            }
        }
    }
    out
}

/// `section_values` folded by the single-or-array rule.
fn collect_section(
    app: &App,
    key: &str,
    strict: bool,
    errs: &mut Vec<(String, String)>,
) -> Option<Value> {
    form_module::fold_section(section_values(app, key, strict, errs))
}

/// Serialize `value` in the given format, matching `config_io::write` exactly.
pub fn render_string(value: &Value, fmt: ConfigFormat) -> String {
    match fmt {
        ConfigFormat::Json => {
            serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("// json: {e}"))
        }
        ConfigFormat::Yaml => {
            serde_yml::to_string(value).unwrap_or_else(|e| format!("# yaml: {e}"))
        }
        ConfigFormat::Toml => {
            // TOML can't represent some shapes (e.g. a top-level null); show an
            // inline comment rather than panicking.
            toml::to_string_pretty(value).unwrap_or_else(|e| format!("# toml: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::createjson::tui::app::App;
    use serde_json::json;

    /// Build an app, enable a couple of tiles, and check the assembly shape per
    /// target.
    fn app_with_tiles(target: Target) -> App {
        let mut app = App::new(None, None);
        app.target = target;
        app.rebuild_target_form();
        // time + two stock + sport-team + golf
        app.instances.push(super::super::app::Instance {
            kind: "time",
            form: form_module::default_form("time"),
        });
        app.instances.push(super::super::app::Instance {
            kind: "stock",
            form: form_module::default_form("stock"),
        });
        app.instances.push(super::super::app::Instance {
            kind: "sport",
            form: form_module::default_form("sport"),
        });
        app.instances.push(super::super::app::Instance {
            kind: "golf",
            form: form_module::default_form("golf"),
        });
        app
    }

    #[test]
    fn matrix_assembly_has_top_level_sections() {
        // stock needs an api key for finnhub to validate strictly; use lossy.
        let app = app_with_tiles(Target::Matrix);
        let v = preview_value(&app);
        assert!(v.get("matrix_options").is_some());
        assert!(v.get("time").is_some());
        assert!(v.get("stock").is_some());
        // sport-team + golf fold under the single `sport` key as an array.
        assert!(v["sport"].is_array());
        assert_eq!(v["sport"].as_array().unwrap().len(), 2);
        assert!(v.get("eink").is_none());
    }

    #[test]
    fn eink_assembly_nests_under_modules() {
        let app = app_with_tiles(Target::Eink);
        let v = preview_value(&app);
        assert!(v.get("matrix_options").is_none());
        let modules = &v["eink"]["modules"];
        assert!(modules.get("time").is_some());
        assert!(modules["time"].is_object());
        assert!(modules["sport"].is_array());
        assert_eq!(v["eink"]["enabled"], json!(true));
    }

    #[test]
    fn multiple_stocks_fold_to_array_on_both_targets() {
        for target in [Target::Matrix, Target::Eink] {
            let mut app = App::new(None, None);
            app.target = target;
            app.rebuild_target_form();
            app.instances.push(super::super::app::Instance {
                kind: "stock",
                form: form_module::default_form("stock"),
            });
            app.instances.push(super::super::app::Instance {
                kind: "stock",
                form: form_module::default_form("stock"),
            });
            let v = preview_value(&app);
            let stock = if target.is_eink() {
                &v["eink"]["modules"]["stock"]
            } else {
                &v["stock"]
            };
            assert!(stock.is_array(), "two stocks should be an array on {target:?}");
            assert_eq!(stock.as_array().unwrap().len(), 2);
        }
    }

    #[test]
    fn render_round_trips_all_formats() {
        let v = json!({"matrix_options": {"chain_length": 1}, "time": {"run": true}});
        for fmt in [ConfigFormat::Json, ConfigFormat::Yaml, ConfigFormat::Toml] {
            let s = render_string(&v, fmt);
            assert!(!s.is_empty());
        }
    }
}
