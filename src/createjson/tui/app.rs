//! Wizard state model — pure data + assembly, no ratatui.
//!
//! The event layer ([`super::event`]) mutates these fields in response to key
//! presses; the render layer ([`super::ui`]) reads them; [`super::preview`]
//! turns them into the config `Value`. Keeping this module terminal-free makes
//! the whole projection unit-testable.

use super::field::Form;
use super::form_module;
use serde_json::Value;

/// Which display this config targets. Mutually exclusive — never both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Matrix,
    Eink,
}

impl Target {
    pub fn is_eink(self) -> bool {
        matches!(self, Target::Eink)
    }
}

/// Output file format chosen in the wizard. Drives serialization + the written
/// file's extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Yaml,
    Toml,
}

impl ConfigFormat {
    pub fn ext(self) -> &'static str {
        match self {
            ConfigFormat::Json => "json",
            ConfigFormat::Yaml => "yaml",
            ConfigFormat::Toml => "toml",
        }
    }
    pub fn label(self) -> &'static str {
        self.ext()
    }
    /// Cycle json → yaml → toml → json.
    pub fn next(self) -> Self {
        match self {
            ConfigFormat::Json => ConfigFormat::Yaml,
            ConfigFormat::Yaml => ConfigFormat::Toml,
            ConfigFormat::Toml => ConfigFormat::Json,
        }
    }
}

/// One configured tile instance.
pub struct Instance {
    /// Tile kind: `"time"`, `"weather"`, `"sport"`, `"golf"`, `"f1"`, …
    pub kind: &'static str,
    pub form: Form,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Setup,
    Modules,
    ConfirmQuit,
}

/// On the Modules screen, whether the module list or the field form has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Form,
}

/// Top-level config section keys scanned when loading an existing config.
const SECTION_KEYS: &[&str] = &[
    "time", "weather", "stock", "iss", "quake", "aurora", "flights", "launch", "hass", "pihole",
    "sport",
];

pub struct App {
    pub screen: Screen,
    pub target: Target,
    pub format: ConfigFormat,
    pub preview_fmt: ConfigFormat,
    /// Matrix geometry **or** eink-block options, depending on `target`.
    pub target_form: Form,
    pub instances: Vec<Instance>,

    // Setup-screen cursor: 0 = target radio, 1..=N = target_form fields,
    // N+1 = format radio.
    pub setup_idx: usize,

    // Modules-screen navigation.
    pub focus: Focus,
    /// Index into [`form_module::TILE_KINDS`].
    pub kind_idx: usize,
    /// Which instance of the selected kind is being edited (0-based).
    pub inst_idx: usize,
    /// Index into the focused form's *visible* fields.
    pub field_idx: usize,

    pub status: String,
    pub should_quit: bool,
    /// `Some` on save (config + chosen format), `None` while running / on quit.
    pub result: Option<(Value, ConfigFormat)>,
    /// Top-level `sleep` block carried over from a loaded config. The wizard
    /// doesn't edit sleep mode, but it must not drop it on re-save — so it's
    /// stashed here and re-emitted verbatim by [`super::preview::build_value`].
    pub preserved_sleep: Option<Value>,
}

impl App {
    /// Fresh wizard, or pre-loaded from an existing config to edit. When
    /// `existing` is `Some`, the target + tiles are loaded, `initial_fmt`
    /// (the existing file's format) seeds the format radio, and the wizard
    /// opens straight on the Modules screen so you can edit right away.
    pub fn new(existing: Option<Value>, initial_fmt: Option<ConfigFormat>) -> Self {
        let target = existing
            .as_ref()
            .map(detect_target)
            .unwrap_or(Target::Matrix);
        let target_form = build_target_form(target, existing.as_ref());
        let instances = existing
            .as_ref()
            .map(|v| load_instances(v, target))
            .unwrap_or_default();
        let fmt = initial_fmt.unwrap_or(ConfigFormat::Json);
        let preserved_sleep = existing.as_ref().and_then(|v| v.get("sleep").cloned());
        let (screen, status) = if existing.is_some() {
            (
                Screen::Modules,
                format!(
                    "Loaded your existing config ({} tile(s)) — edit and ^S to save, Esc for setup",
                    instances.len()
                ),
            )
        } else {
            (Screen::Setup, String::new())
        };
        App {
            screen,
            target,
            format: fmt,
            preview_fmt: fmt,
            target_form,
            instances,
            setup_idx: 0,
            focus: Focus::List,
            kind_idx: 0,
            inst_idx: 0,
            field_idx: 0,
            status,
            should_quit: false,
            result: None,
            preserved_sleep,
        }
    }

    /// Rebuild `target_form` after the target flips (instances are
    /// target-agnostic and carry over untouched).
    pub fn rebuild_target_form(&mut self) {
        self.target_form = build_target_form(self.target, None);
    }

    /// The currently-selected tile kind.
    pub fn selected_kind(&self) -> &'static str {
        form_module::TILE_KINDS
            .get(self.kind_idx)
            .map(|(k, _)| *k)
            .unwrap_or("time")
    }

    /// Global indices into `instances` for the selected kind, in order.
    pub fn instances_of(&self, kind: &str) -> Vec<usize> {
        self.instances
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == kind)
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Global index of the instance currently being edited (selected kind +
    /// `inst_idx`), if any.
    pub fn active_instance(&self) -> Option<usize> {
        let kind = self.selected_kind();
        let idxs = self.instances_of(kind);
        idxs.get(self.inst_idx).copied()
    }

    /// Whether the selected kind has at least one instance.
    pub fn kind_enabled(&self, kind: &str) -> bool {
        self.instances.iter().any(|i| i.kind == kind)
    }

    /// Append a fresh default instance of `kind`.
    pub fn add_instance(&mut self, kind: &'static str) {
        self.instances.push(Instance {
            kind,
            form: form_module::default_form(kind),
        });
    }

    /// Remove every instance of `kind` (the "disable" action).
    pub fn remove_kind(&mut self, kind: &str) {
        self.instances.retain(|i| i.kind != kind);
    }

    /// Remove the instance currently being edited, if any.
    pub fn remove_active_instance(&mut self) {
        if let Some(gi) = self.active_instance() {
            self.instances.remove(gi);
        }
    }

    /// Mutable access to the form of the instance currently being edited.
    pub fn active_form_mut(&mut self) -> Option<&mut Form> {
        let gi = self.active_instance()?;
        Some(&mut self.instances[gi].form)
    }
}

/// Detect the display target from an existing config: an `eink` block (with no
/// top-level tiles) ⇒ E-ink, otherwise Matrix.
fn detect_target(value: &Value) -> Target {
    if value.get("eink").and_then(|e| e.as_object()).is_some()
        && value.get("matrix_options").is_none()
    {
        Target::Eink
    } else {
        Target::Matrix
    }
}

/// Build the target form, optionally seeding it from an existing config's
/// `matrix_options` / `eink` scalar fields.
fn build_target_form(target: Target, existing: Option<&Value>) -> Form {
    match target {
        Target::Matrix => {
            let mut form = form_module::matrix_default_form();
            if let Some(mo) = existing.and_then(|v| v.get("matrix_options")) {
                form.apply_value(mo);
            }
            form
        }
        Target::Eink => {
            let mut form = form_module::eink_default_form();
            if let Some(blk) = existing.and_then(|v| v.get("eink")) {
                form.apply_value(blk);
            }
            form
        }
    }
}

/// Expand an existing config into tile instances.
fn load_instances(value: &Value, target: Target) -> Vec<Instance> {
    let mut out = Vec::new();
    let source: Option<&Value> = if target.is_eink() {
        value.get("eink").and_then(|e| e.get("modules"))
    } else {
        Some(value)
    };
    let Some(source) = source.and_then(|s| s.as_object()) else {
        return out;
    };
    for &key in SECTION_KEYS {
        let Some(v) = source.get(key) else { continue };
        let items: Vec<Value> = match v {
            Value::Array(arr) => arr.clone(),
            Value::Object(_) => vec![v.clone()],
            _ => continue,
        };
        for item in items {
            let kind = if key == "sport" {
                sport_kind_of(&item)
            } else {
                kind_static(key)
            };
            out.push(Instance {
                kind,
                form: form_module::value_to_form(kind, &item),
            });
        }
    }
    out
}

/// Map a sport-array entry to its tile kind by the inner `sport` tag.
fn sport_kind_of(v: &Value) -> &'static str {
    match v.get("sport").and_then(|s| s.as_str()) {
        Some("golf") => "golf",
        Some("f1") => "f1",
        _ => "sport",
    }
}

/// Resolve a scanned section key to its `&'static` kind id (the entries in
/// [`form_module::TILE_KINDS`] are `'static`).
fn kind_static(key: &str) -> &'static str {
    form_module::TILE_KINDS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(k, _)| *k)
        .unwrap_or("time")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_cycles_and_ext() {
        assert_eq!(ConfigFormat::Json.next(), ConfigFormat::Yaml);
        assert_eq!(ConfigFormat::Yaml.next(), ConfigFormat::Toml);
        assert_eq!(ConfigFormat::Toml.next(), ConfigFormat::Json);
        assert_eq!(ConfigFormat::Yaml.ext(), "yaml");
    }

    #[test]
    fn detects_eink_target() {
        let eink = json!({"eink": {"enabled": true, "model": "7in5_v2", "modules": {}}});
        assert_eq!(detect_target(&eink), Target::Eink);
        let matrix = json!({"matrix_options": {}, "time": {"run": true}});
        assert_eq!(detect_target(&matrix), Target::Matrix);
    }

    #[test]
    fn loads_matrix_instances_with_sport_dispatch() {
        let cfg = json!({
            "matrix_options": {"chain_length": 1, "parallel": 1, "brightness": 50, "oled_slowdown": 3, "fail_on_error": false, "hardware_mapping": "adafruit-hat"},
            "time": {"run": true, "color": [255,255,255]},
            "stock": [
                {"run": true, "api": "finnhub", "symbol": "AAPL"},
                {"run": true, "api": "finnhub", "symbol": "MSFT"}
            ],
            "sport": [
                {"run": true, "sport": "basketball", "team_logo": {"name": "Boston Celtics"}},
                {"run": true, "sport": "golf", "tour": "pga"},
                {"run": true, "sport": "f1"}
            ]
        });
        let app = App::new(Some(cfg), None);
        assert_eq!(app.target, Target::Matrix);
        assert_eq!(app.instances_of("stock").len(), 2);
        assert_eq!(app.instances_of("sport").len(), 1);
        assert_eq!(app.instances_of("golf").len(), 1);
        assert_eq!(app.instances_of("f1").len(), 1);
        assert_eq!(app.instances_of("time").len(), 1);
    }

    #[test]
    fn loads_eink_instances_from_modules() {
        let cfg = json!({
            "eink": {
                "enabled": true, "model": "7in5_v2", "threshold": 128,
                "modules": {
                    "time": {"run": true, "color": [255,255,255]},
                    "weather": {"run": true, "api": "nws", "current_location": true}
                }
            }
        });
        let app = App::new(Some(cfg), None);
        assert_eq!(app.target, Target::Eink);
        assert_eq!(app.instances_of("time").len(), 1);
        assert_eq!(app.instances_of("weather").len(), 1);
    }
}
