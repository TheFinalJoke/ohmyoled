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
    attach_sleep(app, &mut map, strict, errs);
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
    attach_sleep(app, &mut root, strict, errs);
    Value::Object(root)
}

/// Serialize the sleep tile (if enabled) into the config root. `sleep` is a
/// top-level, target-independent block, so it never joins the SECTION_ORDER
/// walk (which nests under `eink.modules` on the eink target). The
/// cron-anchored `windows` list carried from a loaded config is re-attached
/// here — the wizard doesn't edit it, but it must survive a round-trip.
fn attach_sleep(app: &App, map: &mut Map<String, Value>, strict: bool, errs: &mut Vec<(String, String)>) {
    let Some(inst) = app.instances.iter().find(|i| i.kind == "sleep") else {
        return;
    };
    let mut v = match form_module::section_to_value("sleep", &inst.form) {
        Ok(v) => v,
        Err(mut e) => {
            if strict {
                errs.append(&mut e);
            }
            inst.form.to_value_lossy()
        }
    };
    if let (Value::Object(m), Some(w)) = (&mut v, &app.preserved_sleep_windows) {
        m.insert("windows".to_string(), w.clone());
    }
    map.insert("sleep".to_string(), v);
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
            // TOML has no null — unset optionals are dropped (they load back as
            // `None` via serde defaults), matching `config_io::write`. Any
            // other unrepresentable shape shows an inline comment rather than
            // panicking.
            toml::to_string_pretty(&crate::config_io::strip_nulls(value))
                .unwrap_or_else(|e| format!("# toml: {e}"))
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
    fn round_trips_loaded_sleep_block_on_both_targets() {
        let cfg = json!({
            "matrix_options": {"chain_length": 1},
            "time": {"run": true, "color": [255, 255, 255]},
            "sleep": {"enabled": true, "sleep": "0 22 * * *", "wake": "0 7 * * *"}
        });
        // Matrix target: the block loads into the sleep tile and re-emits at
        // the top level (plus the struct's remaining keys at their defaults).
        let app = App::new(Some(cfg), None);
        let v = preview_value(&app);
        assert_eq!(v["sleep"]["enabled"], json!(true));
        assert_eq!(v["sleep"]["sleep"], json!("0 22 * * *"));
        assert_eq!(v["sleep"]["wake"], json!("0 7 * * *"));
        assert!(v["sleep"].get("run").is_none(), "sleep uses enabled, not run");

        // And an eink config keeps it top-level (never under eink.modules).
        let eink_cfg = json!({
            "eink": {"enabled": true, "model": "7in5_v2", "modules": {"time": {"run": true}}},
            "sleep": {"enabled": true, "start": "22:00", "end": "07:00"}
        });
        let app = App::new(Some(eink_cfg), None);
        let v = preview_value(&app);
        assert_eq!(v["sleep"]["start"], json!("22:00"));
        assert_eq!(v["sleep"]["end"], json!("07:00"));
        assert!(v["eink"]["modules"].get("sleep").is_none());
    }

    #[test]
    fn wizard_added_sleep_tile_emits_top_level_on_both_targets() {
        for target in [Target::Matrix, Target::Eink] {
            let mut app = App::new(None, None);
            app.target = target;
            app.rebuild_target_form();
            app.instances.push(super::super::app::Instance {
                kind: "sleep",
                form: form_module::default_form("sleep"),
            });
            let v = preview_value(&app);
            // Default form = enabled cron pair.
            assert_eq!(v["sleep"]["enabled"], json!(true), "on {target:?}");
            assert_eq!(v["sleep"]["sleep"], json!("0 22 * * *"));
            assert_eq!(v["sleep"]["wake"], json!("0 7 * * *"));
            if target.is_eink() {
                assert!(v["eink"]["modules"].get("sleep").is_none());
            }
        }
    }

    #[test]
    fn preserves_unedited_sleep_windows_across_round_trip() {
        let cfg = json!({
            "matrix_options": {"chain_length": 1},
            "sleep": {
                "enabled": true,
                "windows": [{"at": "0 13 * * 6,0", "for_mins": 120}]
            }
        });
        let app = App::new(Some(cfg.clone()), None);
        let v = preview_value(&app);
        assert_eq!(v["sleep"]["windows"], cfg["sleep"]["windows"]);
    }

    #[test]
    fn legacy_null_literals_load_as_blank_and_reemit_null() {
        // The --init-config starter writes "null" strings for unset optionals;
        // they must not surface as the literal word in the form or the output.
        let cfg = json!({
            "matrix_options": {"chain_length": 1},
            "sleep": {"enabled": false, "sleep": "null", "wake": "null",
                       "start": "null", "end": "null", "windows": []}
        });
        let app = App::new(Some(cfg), None);
        let v = preview_value(&app);
        assert_eq!(v["sleep"]["enabled"], json!(false));
        assert!(v["sleep"]["sleep"].is_null());
        assert!(v["sleep"]["start"].is_null());
    }

    #[test]
    fn invalid_sleep_cron_fails_strict_save() {
        let mut app = App::new(None, None);
        app.instances.push(super::super::app::Instance {
            kind: "sleep",
            form: form_module::default_form("sleep"),
        });
        // Corrupt the sleep cron field.
        let form = &mut app.instances.last_mut().unwrap().form;
        let idx = form.defs.iter().position(|d| d.id == "sleep").unwrap();
        form.values[idx] =
            crate::createjson::tui::field::FieldValue::Input(tui_input::Input::new(
                "not a cron".into(),
            ));
        let err = build_value(&app, true).unwrap_err();
        assert!(err.iter().any(|(id, _)| id == "sleep"), "got {err:?}");
    }

    #[test]
    fn render_round_trips_all_formats() {
        let v = json!({"matrix_options": {"chain_length": 1}, "time": {"run": true}});
        for fmt in [ConfigFormat::Json, ConfigFormat::Yaml, ConfigFormat::Toml] {
            let s = render_string(&v, fmt);
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn toml_render_survives_null_optionals() {
        // Blank optionals (cache_ttl_secs, sleep start/end) emit JSON nulls,
        // which TOML can't represent — they must be dropped, not error out.
        let v = json!({
            "time": {"run": true, "cache_ttl_secs": null},
            "sleep": {"enabled": true, "sleep": "0 22 * * *", "wake": "0 7 * * *",
                       "start": null, "end": null, "windows": []}
        });
        let s = render_string(&v, ConfigFormat::Toml);
        assert!(!s.starts_with("# toml:"), "toml render errored: {s}");
        assert!(s.contains("enabled = true"));
        assert!(!s.contains("cache_ttl_secs"), "null keys should be dropped");
    }
}
