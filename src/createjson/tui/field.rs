//! The pure, terminal-free core of the TUI config builder.
//!
//! A [`FieldDef`] is the *static* description of one editable field (label,
//! help, kind, visibility predicate). A [`FieldValue`] is its *live* editable
//! state. A [`Form`] pairs an ordered list of defs with their values and knows
//! how to (a) build itself from defaults, (b) load itself from an existing JSON
//! object, and (c) validate + emit a `serde_json::Value`.
//!
//! Everything here is pure data + parsing — no ratatui, no stdin — so the whole
//! projection layer is unit-testable without a terminal. The ratatui rendering
//! and key handling live elsewhere and only ever *read* these types or call the
//! small mutation helpers at the bottom.

use serde_json::{json, Map, Value};
use tui_input::Input;

/// `(slug, human description)` pairs for an [`FieldKind::Enum`].
pub type Choices = &'static [(&'static str, &'static str)];

/// What kind of input a field is, plus its default and any bounds. The kind
/// drives both how the value is rendered and how it's parsed back into JSON.
#[derive(Clone)]
pub enum FieldKind {
    /// Required free text. Blank ⇒ validation error.
    Text { default: &'static str },
    /// Optional free text. Blank ⇒ JSON `null` (the structs use
    /// `null_string_as_none`, so `null` round-trips to `None`).
    OptionalText { default: &'static str },
    /// Checkbox.
    Bool { default: bool },
    /// Whole number with inclusive bounds. Emitted as a JSON integer; serde
    /// narrows it to the struct's `i8`/`i32`/`u8`/… on deserialize.
    Number { default: i64, min: i64, max: i64 },
    /// Optional whole number (e.g. eink `width`/`height`). Blank ⇒ `null`.
    OptionalNumber { min: i64, max: i64 },
    /// Floating point with inclusive bounds (lat/lon/radius). `f32` fields just
    /// narrow on deserialize.
    Float { default: f64, min: f64, max: f64 },
    /// Single-select over `choices`; the JSON value is the chosen slug.
    Enum { default: &'static str, choices: Choices },
    /// Three 0..=255 integers typed as `r g b` (any of space/comma/slash/semi
    /// separators), emitted as a `[r,g,b]` JSON array.
    Rgb { default: (i64, i64, i64) },
    /// Comma-separated list of strings, emitted as a JSON array.
    StringList { default: &'static str },
    /// The shared `cache_ttl_secs` widget: blank ⇒ `null`, `0` ⇒ `0`,
    /// `N` ⇒ `N`.
    CacheTtl,
    /// Single-select whose options each carry an arbitrary JSON value (the
    /// sport team picker). Options are populated at runtime, so they live on
    /// the [`FieldValue`] rather than here.
    ValueEnum,
}

impl FieldKind {
    /// True for every single-line text-edited kind (i.e. everything the cursor
    /// types into). `Bool`/`Enum`/`ValueEnum` are toggled/cycled instead.
    pub fn is_text(&self) -> bool {
        !matches!(
            self,
            FieldKind::Bool { .. } | FieldKind::Enum { .. } | FieldKind::ValueEnum
        )
    }
}

fn always(_: &Form) -> bool {
    true
}

/// Static description of one editable field.
pub struct FieldDef {
    /// Stable key — matches the serde field name in the target struct.
    pub id: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub kind: FieldKind,
    /// Predicate deciding whether this field is currently shown / emitted.
    /// Defaults to always-visible; conditional fields (weather `api_key`,
    /// stock `api_key`, weather `city`) override it.
    pub visible_when: fn(&Form) -> bool,
}

impl FieldDef {
    pub fn new(id: &'static str, label: &'static str, help: &'static str, kind: FieldKind) -> Self {
        Self {
            id,
            label,
            help,
            kind,
            visible_when: always,
        }
    }

    /// Attach a visibility predicate (builder style).
    pub fn when(mut self, f: fn(&Form) -> bool) -> Self {
        self.visible_when = f;
        self
    }
}

/// Live, editable value for one field. Text-ish kinds all share a single
/// [`Input`] (which owns the cursor + horizontal scroll); the rest carry an
/// index or a bool.
pub enum FieldValue {
    Input(Input),
    Bool(bool),
    /// Selected index into the def's `Enum` choices.
    Enum(usize),
    /// Runtime options + the selected index (sport team picker).
    ValueEnum {
        options: Vec<(String, Value)>,
        selected: usize,
    },
}

impl FieldValue {
    fn default_for(kind: &FieldKind) -> Self {
        match kind {
            FieldKind::Text { default }
            | FieldKind::OptionalText { default }
            | FieldKind::StringList { default } => FieldValue::Input(Input::new(default.to_string())),
            FieldKind::Number { default, .. } => {
                FieldValue::Input(Input::new(default.to_string()))
            }
            FieldKind::OptionalNumber { .. } | FieldKind::CacheTtl => {
                FieldValue::Input(Input::new(String::new()))
            }
            FieldKind::Float { default, .. } => {
                FieldValue::Input(Input::new(format_f64(*default)))
            }
            FieldKind::Rgb { default } => FieldValue::Input(Input::new(format!(
                "{} {} {}",
                default.0, default.1, default.2
            ))),
            FieldKind::Bool { default } => FieldValue::Bool(*default),
            FieldKind::Enum { default, choices } => {
                let idx = choices
                    .iter()
                    .position(|(slug, _)| slug == default)
                    .unwrap_or(0);
                FieldValue::Enum(idx)
            }
            FieldKind::ValueEnum => FieldValue::ValueEnum {
                options: Vec::new(),
                selected: 0,
            },
        }
    }

    /// The current text of an `Input` variant (empty for non-text variants).
    pub fn text(&self) -> &str {
        match self {
            FieldValue::Input(i) => i.value(),
            _ => "",
        }
    }

    pub fn as_bool(&self) -> bool {
        matches!(self, FieldValue::Bool(true))
    }

    /// Mutable access to the underlying `Input` for the event layer.
    pub fn input_mut(&mut self) -> Option<&mut Input> {
        match self {
            FieldValue::Input(i) => Some(i),
            _ => None,
        }
    }
}

/// Format an `f64` without scientific notation or a dangling `.0`-only when it's
/// integral, matching how a person would have typed it.
fn format_f64(v: f64) -> String {
    let s = format!("{v}");
    s
}

/// An ordered list of field defs + their live values.
pub struct Form {
    pub defs: Vec<FieldDef>,
    pub values: Vec<FieldValue>,
    /// Whether `to_value` injects `"run": true` (tile modules do; the matrix /
    /// eink-block forms don't — they have no `run` field).
    pub inserts_run: bool,
}

impl Form {
    /// Build a fresh form from a schema, each field at its default.
    pub fn from_defs(defs: Vec<FieldDef>, inserts_run: bool) -> Self {
        let values = defs.iter().map(|d| FieldValue::default_for(&d.kind)).collect();
        Self {
            defs,
            values,
            inserts_run,
        }
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.defs.iter().position(|d| d.id == id)
    }

    /// The chosen slug of an `Enum` field — used by `visible_when` predicates.
    pub fn enum_slug(&self, id: &str) -> Option<&'static str> {
        let i = self.index_of(id)?;
        match (&self.defs[i].kind, &self.values[i]) {
            (FieldKind::Enum { choices, .. }, FieldValue::Enum(sel)) => {
                choices.get(*sel).map(|(slug, _)| *slug)
            }
            _ => None,
        }
    }

    /// The value of a `Bool` field — used by `visible_when` predicates.
    pub fn bool_val(&self, id: &str) -> Option<bool> {
        let i = self.index_of(id)?;
        match &self.values[i] {
            FieldValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Indices of currently-visible fields, in display order.
    pub fn visible_indices(&self) -> Vec<usize> {
        (0..self.defs.len())
            .filter(|&i| (self.defs[i].visible_when)(self))
            .collect()
    }

    /// Replace a `ValueEnum` field's option list (sport team picker), keeping
    /// the selection in range. No-op if the field isn't a `ValueEnum`.
    pub fn set_value_enum(&mut self, id: &str, options: Vec<(String, Value)>, selected: usize) {
        if let Some(i) = self.index_of(id) {
            if let FieldValue::ValueEnum { .. } = self.values[i] {
                let sel = selected.min(options.len().saturating_sub(1));
                self.values[i] = FieldValue::ValueEnum { options, selected: sel };
            }
        }
    }

    /// Validate every visible field and assemble the JSON object. On any
    /// failure returns the list of `(field id, message)` so the UI can jump to
    /// and highlight the first offender.
    pub fn to_value(&self) -> Result<Value, Vec<(String, String)>> {
        self.assemble(false)
    }

    /// Best-effort assembly that never fails: fields that don't validate are
    /// simply omitted. Used for the live preview so it never goes blank while
    /// you're mid-edit.
    pub fn to_value_lossy(&self) -> Value {
        self.assemble(true)
            .unwrap_or_else(|_| Value::Object(Map::new()))
    }

    fn assemble(&self, lossy: bool) -> Result<Value, Vec<(String, String)>> {
        let mut map = Map::new();
        let mut errs: Vec<(String, String)> = Vec::new();
        // In lossy mode, validation failures are silently skipped (the field is
        // just omitted) so the live preview never errors.
        macro_rules! fail {
            ($e:expr) => {{
                if !lossy {
                    errs.push($e);
                }
            }};
        }
        if self.inserts_run {
            map.insert("run".to_string(), json!(true));
        }
        for i in self.visible_indices() {
            let def = &self.defs[i];
            let val = &self.values[i];
            let id = def.id.to_string();
            match &def.kind {
                FieldKind::Text { .. } => {
                    let s = val.text().trim();
                    if s.is_empty() {
                        fail!((id, format!("{} is required", def.label)));
                    } else {
                        map.insert(def.id.to_string(), json!(s));
                    }
                }
                FieldKind::OptionalText { .. } => {
                    let s = val.text().trim();
                    if s.is_empty() {
                        map.insert(def.id.to_string(), Value::Null);
                    } else {
                        map.insert(def.id.to_string(), json!(s));
                    }
                }
                FieldKind::Bool { .. } => {
                    map.insert(def.id.to_string(), json!(val.as_bool()));
                }
                FieldKind::Number { min, max, .. } => match val.text().trim().parse::<i64>() {
                    Ok(n) if (*min..=*max).contains(&n) => {
                        map.insert(def.id.to_string(), json!(n));
                    }
                    Ok(_) => fail!((id, format!("{} must be in [{min}, {max}]", def.label))),
                    Err(_) => fail!((id, format!("{} must be a whole number", def.label))),
                },
                FieldKind::OptionalNumber { min, max } => {
                    let s = val.text().trim();
                    if s.is_empty() {
                        map.insert(def.id.to_string(), Value::Null);
                    } else {
                        match s.parse::<i64>() {
                            Ok(n) if (*min..=*max).contains(&n) => {
                                map.insert(def.id.to_string(), json!(n));
                            }
                            Ok(_) => {
                                fail!((id, format!("{} must be in [{min}, {max}]", def.label)))
                            }
                            Err(_) => errs
                                .push((id, format!("{} must be blank or a whole number", def.label))),
                        }
                    }
                }
                FieldKind::Float { min, max, .. } => match val.text().trim().parse::<f64>() {
                    Ok(n) if (*min..=*max).contains(&n) => {
                        map.insert(def.id.to_string(), json!(n));
                    }
                    Ok(_) => fail!((id, format!("{} must be in [{min}, {max}]", def.label))),
                    Err(_) => fail!((id, format!("{} must be a number", def.label))),
                },
                FieldKind::Enum { choices, .. } => {
                    let idx = match val {
                        FieldValue::Enum(i) => *i,
                        _ => 0,
                    };
                    let slug = choices.get(idx).map(|(s, _)| *s).unwrap_or("");
                    map.insert(def.id.to_string(), json!(slug));
                }
                FieldKind::Rgb { .. } => match parse_rgb(val.text()) {
                    Some([r, g, b]) => {
                        map.insert(def.id.to_string(), json!([r, g, b]));
                    }
                    None => fail!((
                        id,
                        format!("{} needs three values in 0..=255", def.label),
                    )),
                },
                FieldKind::StringList { .. } => {
                    let list: Vec<Value> = val
                        .text()
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| json!(s))
                        .collect();
                    map.insert(def.id.to_string(), Value::Array(list));
                }
                FieldKind::CacheTtl => {
                    let s = val.text().trim();
                    if s.is_empty() {
                        map.insert(def.id.to_string(), Value::Null);
                    } else {
                        match s.parse::<u64>() {
                            Ok(n) => {
                                map.insert(def.id.to_string(), json!(n));
                            }
                            Err(_) => fail!((
                                id,
                                "cache_ttl_secs must be blank or a non-negative integer".to_string(),
                            )),
                        }
                    }
                }
                FieldKind::ValueEnum => match val {
                    FieldValue::ValueEnum { options, selected } => match options.get(*selected) {
                        Some((_, v)) => {
                            map.insert(def.id.to_string(), v.clone());
                        }
                        None => fail!((id, format!("{} — nothing to choose", def.label))),
                    },
                    _ => fail!((id, format!("{} — invalid state", def.label))),
                },
            }
        }
        if errs.is_empty() {
            Ok(Value::Object(map))
        } else {
            Err(errs)
        }
    }

    /// Overlay an existing JSON object onto this form's values (best effort;
    /// missing or mistyped keys keep the field at its default). Used for the
    /// merge / load-existing path. `ValueEnum` options must already be set
    /// (the sport hook does that before calling this).
    pub fn apply_value(&mut self, value: &Value) {
        let Some(obj) = value.as_object() else { return };
        for i in 0..self.defs.len() {
            let Some(v) = obj.get(self.defs[i].id) else {
                continue;
            };
            match &self.defs[i].kind {
                FieldKind::Text { .. }
                | FieldKind::OptionalText { .. }
                | FieldKind::Float { .. }
                | FieldKind::Number { .. }
                | FieldKind::OptionalNumber { .. } => {
                    let s = json_scalar_to_string(v);
                    self.values[i] = FieldValue::Input(Input::new(s));
                }
                FieldKind::CacheTtl => {
                    let s = match v {
                        Value::Null => String::new(),
                        _ => json_scalar_to_string(v),
                    };
                    self.values[i] = FieldValue::Input(Input::new(s));
                }
                FieldKind::Rgb { .. } => {
                    if let Some(arr) = v.as_array() {
                        let parts: Vec<String> =
                            arr.iter().map(json_scalar_to_string).collect();
                        self.values[i] = FieldValue::Input(Input::new(parts.join(" ")));
                    }
                }
                FieldKind::StringList { .. } => {
                    if let Some(arr) = v.as_array() {
                        let parts: Vec<String> =
                            arr.iter().map(json_scalar_to_string).collect();
                        self.values[i] = FieldValue::Input(Input::new(parts.join(", ")));
                    }
                }
                FieldKind::Bool { .. } => {
                    if let Some(b) = v.as_bool() {
                        self.values[i] = FieldValue::Bool(b);
                    }
                }
                FieldKind::Enum { choices, .. } => {
                    if let Some(slug) = v.as_str() {
                        if let Some(idx) = choices.iter().position(|(s, _)| *s == slug) {
                            self.values[i] = FieldValue::Enum(idx);
                        }
                    }
                }
                FieldKind::ValueEnum => {
                    if let FieldValue::ValueEnum { options, selected } = &mut self.values[i] {
                        // Match the incoming value against the option list (by
                        // the `name` field if present, else by full equality).
                        let incoming_name = v.get("name").and_then(|n| n.as_str());
                        if let Some(idx) = options.iter().position(|(label, opt)| {
                            incoming_name == Some(label.as_str()) || opt == v
                        }) {
                            *selected = idx;
                        }
                    }
                }
            }
        }
    }
}

/// Parse three 0..=255 integers from any of the common separators (space,
/// comma, slash, semicolon) — mirrors the legacy `parse_color`/`read_rgb`.
fn parse_rgb(s: &str) -> Option<[i64; 3]> {
    let parts: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || c == ',' || c == '/' || c == ';')
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let mut out = [0i64; 3];
    for (slot, part) in out.iter_mut().zip(parts) {
        let n = part.parse::<i64>().ok()?;
        if !(0..=255).contains(&n) {
            return None;
        }
        *slot = n;
    }
    Some(out)
}

/// Render a JSON scalar the way a user would type it into a text field.
fn json_scalar_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_def() -> Vec<FieldDef> {
        vec![
            FieldDef::new("name", "Name", "", FieldKind::Text { default: "" }),
            FieldDef::new(
                "count",
                "Count",
                "",
                FieldKind::Number {
                    default: 5,
                    min: 0,
                    max: 9,
                },
            ),
            FieldDef::new("cache_ttl_secs", "Cache", "", FieldKind::CacheTtl),
        ]
    }

    #[test]
    fn required_text_blank_errors() {
        let form = Form::from_defs(text_def(), true);
        let err = form.to_value().unwrap_err();
        assert!(err.iter().any(|(id, _)| id == "name"));
    }

    #[test]
    fn number_range_enforced() {
        let mut form = Form::from_defs(text_def(), true);
        form.values[0] = FieldValue::Input(Input::new("ok".into()));
        form.values[1] = FieldValue::Input(Input::new("20".into()));
        let err = form.to_value().unwrap_err();
        assert!(err.iter().any(|(id, _)| id == "count"));
    }

    #[test]
    fn cache_ttl_blank_zero_n() {
        let mut form = Form::from_defs(text_def(), true);
        form.values[0] = FieldValue::Input(Input::new("ok".into()));
        // blank -> null
        let v = form.to_value().unwrap();
        assert!(v["cache_ttl_secs"].is_null());
        // 0 -> 0
        form.values[2] = FieldValue::Input(Input::new("0".into()));
        assert_eq!(form.to_value().unwrap()["cache_ttl_secs"], json!(0));
        // garbage -> error
        form.values[2] = FieldValue::Input(Input::new("abc".into()));
        assert!(form.to_value().is_err());
    }

    #[test]
    fn run_injected_when_requested() {
        let mut form = Form::from_defs(text_def(), true);
        form.values[0] = FieldValue::Input(Input::new("ok".into()));
        assert_eq!(form.to_value().unwrap()["run"], json!(true));
        form.inserts_run = false;
        assert!(form.to_value().unwrap().get("run").is_none());
    }

    #[test]
    fn rgb_parses_separators_and_range() {
        assert_eq!(parse_rgb("255 0 128"), Some([255, 0, 128]));
        assert_eq!(parse_rgb("10,20,30"), Some([10, 20, 30]));
        assert_eq!(parse_rgb("10/20/30"), Some([10, 20, 30]));
        assert!(parse_rgb("256 0 0").is_none());
        assert!(parse_rgb("1 2").is_none());
    }

    #[test]
    fn enum_emits_slug_and_visible_when_reads_it() {
        let defs = vec![
            FieldDef::new(
                "api",
                "API",
                "",
                FieldKind::Enum {
                    default: "nws",
                    choices: &[("nws", "x"), ("openweather", "y")],
                },
            ),
            FieldDef::new("api_key", "Key", "", FieldKind::Text { default: "" })
                .when(|f| f.enum_slug("api") != Some("nws")),
        ];
        let mut form = Form::from_defs(defs, true);
        // nws selected -> api_key hidden -> not required, absent
        let v = form.to_value().unwrap();
        assert_eq!(v["api"], json!("nws"));
        assert!(v.get("api_key").is_none());
        // switch to openweather -> api_key now visible + required
        form.values[0] = FieldValue::Enum(1);
        assert!(form.to_value().is_err());
    }

    #[test]
    fn apply_value_round_trips_scalars() {
        let mut form = Form::from_defs(text_def(), true);
        form.apply_value(&json!({"name": "hi", "count": 7, "cache_ttl_secs": 30}));
        assert_eq!(form.values[0].text(), "hi");
        assert_eq!(form.values[1].text(), "7");
        assert_eq!(form.values[2].text(), "30");
        let v = form.to_value().unwrap();
        assert_eq!(v["count"], json!(7));
        assert_eq!(v["cache_ttl_secs"], json!(30));
    }
}
